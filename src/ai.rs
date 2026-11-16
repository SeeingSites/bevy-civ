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
    pub leader: String,
    pub adjective: String,
    /// Frame in the leader portrait atlas.
    pub portrait: usize,
    /// Names for cities this civilization founds later.
    pub city_names: Vec<String>,
}

impl Default for Civilizations {
    fn default() -> Self {
        Self { players: Vec::new() }
    }
}

pub fn unit_strength(unit: &Unit) -> f32 {
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
    let land_set: std::collections::HashSet<Hex> = land.iter().filter(|(_, t)| *t != Terrain::Water).map(|(c, _)| *c).collect();
    let is_land = |hex: Hex| land_set.contains(&hex);
    let mut founded: Vec<Hex> = Vec::new();

    for (entity, mut unit) in units.iter_mut() {
        if unit.owner == 0 || unit.movement_left == 0 || unit.build_turns_left.is_some() {
            continue;
        }
        let owner = unit.owner;
        let civ = civs.players.iter().find(|c| c.id == owner);
        let aggression = civ.map(|c| c.aggression).unwrap_or(0.8);
        let can_sail = civ.is_some_and(|c| c.tech.researched.contains(&crate::tech::Tech::Seafaring));

        // Settler: walk away from existing cities, then found a new one
        if unit.unit_type == UnitType::Settler {
            let here = unit.position;
            let crowded = |hex: Hex| {
                city_positions.iter().any(|(p, _)| p.distance_to(hex) < 3) || founded.iter().any(|p| p.distance_to(hex) < 3)
            };
            if is_land(here) && !crowded(here) {
                let civ = civs.players.iter().find(|c| c.id == owner);
                let count = city_positions.iter().filter(|(_, o)| *o == owner).count() + founded.len();
                let name = civ
                    .and_then(|c| c.city_names.get(count.saturating_sub(1)).cloned())
                    .unwrap_or_else(|| format!("Outpost {}", city_positions.len() + founded.len() + 1));
                crate::city::spawn_city(commands, here, name, 1, owner, false);
                founded.push(here);
                commands.entity(entity).despawn_recursive();
            } else {
                let options: Vec<Hex> = here.all_neighbors().into_iter().filter(|n| is_land(*n) && !occupied.get(n).is_some_and(|(o, _)| *o != owner)).collect();
                // Prefer steps that lead away from the nearest city.
                let away = |hex: Hex| city_positions.iter().map(|(p, _)| p.distance_to(hex)).min().unwrap_or(0);
                if let Some(step) = options.iter().copied().max_by_key(|n| (away(*n), rng.gen_range(0..4))) {
                    unit.position = step;
                    unit.movement_left = 0;
                }
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
            if let Some(path) = find_path(here, target, land.iter().copied(), can_sail) {
                for next in path.into_iter().skip(1) {
                    if unit.movement_left == 0 {
                        break;
                    }
                    let boarding = is_land(unit.position) != is_land(next);
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
                            // Boarding or leaving a boat takes the rest of the turn.
                            unit.movement_left = if boarding { 0 } else { unit.movement_left - 1 };
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
        if is_land(step) && is_land(here) && !occupied.get(&step).is_some_and(|(other, _)| *other != owner) {
            unit.position = step;
            unit.movement_left -= 1;
        }
    }
    attacks
}
