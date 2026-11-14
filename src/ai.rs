use bevy::prelude::*;
use hexx::Hex;
use crate::unit::{Unit, UnitType};
use crate::city::City;
use crate::terrain::HexTile;
use crate::tech::{Tech, TechState};
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

    // AI units
    for mut unit in &mut units {
        if unit.owner == 0 || unit.movement_left == 0 { continue; }

        // Settler: found city on good land
        if unit.unit_type == UnitType::Settler {
            let neighbors = unit.position.all_neighbors();
            for &n in &neighbors {
                let occupied = cities.iter().any(|c| c.position == n);
                let land = tiles.iter().any(|t| t.coord == n && t.terrain != crate::terrain::Terrain::Water);
                if !occupied && land {
                    // Found city
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
                    // Consume settler by moving it off and zeroing (simplified)
                    unit.position = n;
                    unit.movement_left = 0;
                    return; // one founding per turn for simplicity
                }
            }
            continue;
        }

        // Prefer moving toward nearest city or random land
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
