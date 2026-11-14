use bevy::prelude::*;
use hexx::Hex;
use crate::unit::{Unit, UnitType};
use crate::city::City;
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
}

impl Default for Civilizations {
    fn default() -> Self {
        Self {
            players: vec![
                Civ { id: 0, name: "Player".into(), is_human: true, gold: 0, tech: TechState::default() },
                Civ { id: 1, name: "AI Rome".into(), is_human: false, gold: 50, tech: TechState::default() },
                Civ { id: 2, name: "AI Egypt".into(), is_human: false, gold: 50, tech: TechState::default() },
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
    mut units: Query<&mut Unit>,
    mut cities: Query<&mut City>,
    tiles: Query<&HexTile>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    layout: &hexx::HexLayout,
) {
    let mut rng = rand::thread_rng();

    // AI research
    for civ in &mut civs.players {
        if civ.is_human { continue; }
        civ.tech.science_per_turn = 3;
        if let Some(completed) = civ.tech.process_turn() {
            info!("{} researched {}", civ.name, completed.name());
        }
    }

    // Collect unit data for war decisions (avoid borrow issues)
    let unit_snapshots: Vec<(Hex, u32, UnitType, f32)> = units
        .iter()
        .map(|u| (u.position, u.owner, u.unit_type, unit_strength(u)))
        .collect();

    for mut unit in &mut units {
        if unit.owner == 0 || unit.movement_left == 0 || unit.build_turns_left.is_some() {
            continue;
        }

        // Settler: found city
        if unit.unit_type == UnitType::Settler {
            let neighbors = unit.position.all_neighbors();
            for &n in &neighbors {
                let occupied = cities.iter().any(|c| c.position == n);
                let land = tiles.iter().any(|t| t.coord == n && t.terrain != crate::terrain::Terrain::Water);
                if !occupied && land {
                    crate::city::spawn_city(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        layout,
                        n,
                        format!("AI City {}", cities.iter().count() + 1),
                        1,
                    );
                    info!("AI founded city at {:?}", n);
                    unit.position = n;
                    unit.movement_left = 0;
                    return;
                }
            }
            continue;
        }

        // War AI: look for nearby enemies
        let my_strength = unit_strength(&unit);
        let mut best_target: Option<(Hex, f32)> = None;
        let mut threat_nearby = false;

        for (pos, owner, _, enemy_strength) in &unit_snapshots {
            if *owner == unit.owner { continue; }
            let dist = unit.position.distance_to(*pos);
            if dist <= 2 {
                threat_nearby = true;
                if my_strength > *enemy_strength * 1.1 {
                    // Stronger: attack toward it
                    if best_target.map(|(_, s)| *enemy_strength < s).unwrap_or(true) {
                        best_target = Some((*pos, *enemy_strength));
                    }
                }
            }
        }

        if let Some((target_pos, _)) = best_target {
            // Move toward the weak enemy
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
                info!("AI unit advancing on enemy at {:?}", target_pos);
                continue;
            }
        } else if threat_nearby && my_strength < 3.0 {
            // Weaker and threatened: try to retreat toward own city
            if let Some(home) = cities.iter().find(|c| c.owner == unit.owner).map(|c| c.position) {
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

        // Default: random land move
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
