use bevy::prelude::*;
use hexx::Hex;
use crate::unit::{Unit, UnitType};
use crate::terrain::{HexTile, Terrain};
use std::collections::HashMap;
use crate::tech::TechState;
use crate::pathfinding::find_path;
use rand::Rng;

#[derive(Resource)]
pub struct Civilizations {
    pub players: Vec<Civ>,
}

#[derive(Clone)]
pub struct Civ {
    pub id: u32,
    pub name: String,
    pub is_human: bool,
    pub gold: i32,
    pub tech: TechState,
    pub aggression: f32,
}

impl Default for Civilizations {
    fn default() -> Self {
        Self {
            players: vec![
                Civ { id: 0, name: "Player".into(), is_human: true, gold: 0, tech: TechState::default(), aggression: 0.5 },
                Civ { id: 1, name: "AI Rome".into(), is_human: false, gold: 50, tech: TechState::default(), aggression: 0.7 },
                Civ { id: 2, name: "AI Egypt".into(), is_human: false, gold: 50, tech: TechState::default(), aggression: 0.4 },
            ],
        }
    }
}

fn unit_strength(unit: &Unit) -> f32 {
    let (atk, def, _) = unit.unit_type.stats();
    (atk as f32) * unit.unit_type.attack_bonus(crate::terrain::Terrain::Plains)
        + (def as f32) * unit.unit_type.defense_bonus()
}

/// An attack an AI unit wants to make this turn; the caller resolves the combat.
pub struct AiAttack {
    pub attacker: Entity,
    pub target: Hex,
}

/// Moves every computer-controlled unit and returns the attacks they launch.
///
/// `hostile(a, b)` says whether owner `a` is willing to attack owner `b`.
pub fn process_ai_turn(
    civs: &mut Civilizations,
    units: &mut Query<(Entity, &mut Unit)>,
    city_positions: &[(Hex, u32)],
    tiles: &Query<&mut HexTile>,
    hostile: &dyn Fn(u32, u32) -> bool,
    commands: &mut Commands,
) -> Vec<AiAttack> {
    let mut rng = rand::thread_rng();
    let mut attacks = Vec::new();

    for civ in &mut civs.players {
        if civ.is_human { continue; }
        civ.tech.science_per_turn = 4;
        if let Some(completed) = civ.tech.process_turn() {
            info!("{} researched {}", civ.name, completed.name());
        }
        if civ.gold > 100 && rng.gen_bool(0.3) {
            civ.gold -= 40;
        }
    }

    let unit_snapshots: Vec<(Hex, u32, f32)> = units
        .iter()
        .map(|(_, u)| (u.position, u.owner, unit_strength(u)))
        .collect();
    // Who stands where at the start of the AI phase (strongest unit per tile).
    let mut occupied: HashMap<Hex, (u32, f32)> = HashMap::new();
    for (pos, owner, strength) in &unit_snapshots {
        let entry = occupied.entry(*pos).or_insert((*owner, *strength));
        if *strength > entry.1 {
            *entry = (*owner, *strength);
        }
    }
    let land: Vec<(Hex, Terrain)> = tiles.iter().map(|t| (t.coord, t.terrain)).collect();
    let is_land = |hex: Hex| land.iter().any(|(c, t)| *c == hex && *t != Terrain::Water);
    let mut founded: Vec<Hex> = Vec::new();

    for (entity, mut unit) in units.iter_mut() {
        if unit.owner == 0 || unit.movement_left == 0 || unit.build_turns_left.is_some() {
            continue;
        }
        let owner = unit.owner;
        let aggression = civs.players.iter().find(|c| c.id == owner).map(|c| c.aggression).unwrap_or(0.8);

        // Settler: found a city on free land nearby
        if unit.unit_type == UnitType::Settler {
            let spot = unit.position.all_neighbors().into_iter().find(|n| {
                is_land(*n) && !founded.contains(n) && !city_positions.iter().any(|(p, _)| p.distance_to(*n) < 2)
            });
            if let Some(n) = spot {
                crate::city::spawn_city(commands, n, format!("Outpost {}", city_positions.len() + founded.len() + 1), 1, owner, false);
                founded.push(n);
                commands.entity(entity).despawn_recursive();
            }
            continue;
        }

        // Worker: improve near home
        if unit.unit_type.is_worker() {
            if let Some(home) = city_positions.iter().find(|(_, o)| *o == owner).map(|(p, _)| *p) {
                if unit.position.distance_to(home) <= 2 && rng.gen_bool(0.6) {
                    unit.start_build(crate::terrain::Improvement::Farm);
                }
            }
            continue;
        }
        if !unit.unit_type.is_military() {
            continue;
        }

        // Pick a target: a beatable enemy unit close by, otherwise the nearest enemy city.
        let my_strength = unit_strength(&unit);
        let threshold = if aggression > 0.6 { 0.9 } else { 1.2 };
        let here = unit.position;
        let enemy_unit = unit_snapshots
            .iter()
            .filter(|(pos, other, strength)| {
                hostile(owner, *other) && here.distance_to(*pos) <= 4 && my_strength > *strength * threshold
            })
            .min_by_key(|(pos, _, _)| here.distance_to(*pos))
            .map(|(pos, _, _)| *pos);
        let city_range = if aggression > 0.6 { 14 } else { 7 };
        let enemy_city = city_positions
            .iter()
            .filter(|(pos, other)| hostile(owner, *other) && here.distance_to(*pos) <= city_range)
            .min_by_key(|(pos, _)| here.distance_to(*pos))
            .map(|(pos, _)| *pos);

        if let Some(target) = enemy_unit.or(enemy_city) {
            if let Some(path) = find_path(here, target, land.iter().copied()) {
                for next in path.into_iter().skip(1) {
                    if unit.movement_left == 0 {
                        break;
                    }
                    match occupied.get(&next) {
                        Some((other, strength)) if *other != owner => {
                            // Attack if the odds look acceptable; otherwise hold position.
                            if hostile(owner, *other) && (aggression > 0.6 || my_strength >= *strength) {
                                attacks.push(AiAttack { attacker: entity, target: next });
                                unit.movement_left = 0;
                            }
                            break;
                        }
                        _ => {
                            unit.position = next;
                            unit.movement_left -= 1;
                        }
                    }
                }
            }
            continue;
        }

        // Nothing to fight: garrison a city, or wander a little.
        let in_own_city = city_positions.iter().any(|(p, o)| *p == here && *o == owner);
        if in_own_city || rng.gen_bool(0.5) {
            continue;
        }
        let neighbors = here.all_neighbors();
        let step = neighbors[rng.gen_range(0..neighbors.len())];
        if is_land(step) && !occupied.get(&step).is_some_and(|(other, _)| *other != owner) {
            unit.position = step;
            unit.movement_left -= 1;
        }
    }
    attacks
}
