use bevy::prelude::*;
use hexx::Hex;
use crate::unit::{Unit, UnitType};
use crate::terrain::HexTile;
use crate::tech::TechState;
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

pub fn process_ai_turn(
    civs: &mut Civilizations,
    units: &mut Query<&mut Unit>,
    city_positions: &[(Hex, u32)],
    tiles: &Query<&mut HexTile>,
    commands: &mut Commands,
) {
    let mut rng = rand::thread_rng();

    // AI research + gold management
    for civ in &mut civs.players {
        if civ.is_human { continue; }
        civ.tech.science_per_turn = 4;
        if let Some(completed) = civ.tech.process_turn() {
            info!("{} researched {}", civ.name, completed.name());
        }
        // AI spends gold on unit production occasionally
        if civ.gold > 100 && rng.gen_bool(0.3) {
            civ.gold -= 40;
        }
    }

    let unit_snapshots: Vec<(Hex, u32, UnitType, f32)> = units
        .iter()
        .map(|u| (u.position, u.owner, u.unit_type, unit_strength(u)))
        .collect();

    for mut unit in units.iter_mut() {
        if unit.owner == 0 || unit.movement_left == 0 || unit.build_turns_left.is_some() {
            continue;
        }

        let civ_aggression = civs.players.iter()
            .find(|c| c.id == unit.owner)
            .map(|c| c.aggression)
            .unwrap_or(0.5);

        // Settler: found city on good land
        if unit.unit_type == UnitType::Settler {
            let neighbors = unit.position.all_neighbors();
            for &n in &neighbors {
                let occupied = city_positions.iter().any(|(p, _)| *p == n);
                let land = tiles.iter().any(|t| t.coord == n && t.terrain != crate::terrain::Terrain::Water);
                if !occupied && land {
                    crate::city::spawn_city(
                        commands,
                        n,
                        format!("Outpost {}", city_positions.len() + 1),
                        1,
                        unit.owner,
                        false,
                    );
                    info!("AI founded city at {:?}", n);
                    unit.position = n;
                    unit.movement_left = 0;
                    return;
                }
            }
            continue;
        }

        // Worker: improve tiles near own cities
        if unit.unit_type.is_worker() {
            if let Some(home) = city_positions.iter().find(|(_, o)| *o == unit.owner).map(|(p, _)| *p) {
                if unit.position.distance_to(home) <= 2 && rng.gen_bool(0.6) {
                    // Prefer farm on food tiles
                    unit.start_build(crate::terrain::Improvement::Farm);
                    continue;
                }
            }
        }

        let my_strength = unit_strength(&unit);
        let mut best_target: Option<(Hex, f32)> = None;
        let mut threat_nearby = false;

        for (pos, owner, _, enemy_strength) in &unit_snapshots {
            if *owner == unit.owner { continue; }
            let dist = unit.position.distance_to(*pos);
            if dist <= 3 {
                threat_nearby = true;
                let attack_threshold = if civ_aggression > 0.6 { 0.9 } else { 1.2 };
                if my_strength > *enemy_strength * attack_threshold {
                    if best_target.map(|(_, s)| *enemy_strength < s).unwrap_or(true) {
                        best_target = Some((*pos, *enemy_strength));
                    }
                }
            }
        }

        // Also target enemy cities if aggressive
        if best_target.is_none() && civ_aggression > 0.6 {
            for (pos, owner) in city_positions {
                if *owner != unit.owner && unit.position.distance_to(*pos) <= 4 {
                    best_target = Some((*pos, 0.0));
                    break;
                }
            }
        }

        if let Some((target_pos, _)) = best_target {
            let neighbors = unit.position.all_neighbors();
            let mut best_move = None;
            let mut best_dist = unit.position.distance_to(target_pos);
            for &n in &neighbors {
                let land = tiles.iter().any(|t| t.coord == n && t.terrain != crate::terrain::Terrain::Water);
                if land {
                    let d = n.distance_to(target_pos);
                    if d < best_dist {
                        best_dist = d;
                        best_move = Some(n);
                    }
                }
            }
            if let Some(mv) = best_move {
                unit.position = mv;
                unit.movement_left = unit.movement_left.saturating_sub(1);
                info!("AI unit advancing on target at {:?}", target_pos);
                continue;
            }
        } else if threat_nearby && my_strength < 4.0 {
            if let Some(home) = city_positions.iter().find(|(_, o)| *o == unit.owner).map(|(p, _)| *p) {
                let neighbors = unit.position.all_neighbors();
                let mut best_move = None;
                let mut best_dist = unit.position.distance_to(home);
                for &n in &neighbors {
                    let land = tiles.iter().any(|t| t.coord == n && t.terrain != crate::terrain::Terrain::Water);
                    if land {
                        let d = n.distance_to(home);
                        if d < best_dist {
                            best_dist = d;
                            best_move = Some(n);
                        }
                    }
                }
                if let Some(mv) = best_move {
                    unit.position = mv;
                    unit.movement_left = unit.movement_left.saturating_sub(1);
                    info!("AI unit retreating toward home");
                    continue;
                }
            }
        }

        // Default: move toward own cities or random
        let neighbors = unit.position.all_neighbors();
        if let Some(&target) = neighbors.get(rng.gen_range(0..neighbors.len())) {
            let land = tiles.iter().any(|t| t.coord == target && t.terrain != crate::terrain::Terrain::Water);
            if land {
                unit.position = target;
                unit.movement_left = unit.movement_left.saturating_sub(1);
            }
        }
    }
}
