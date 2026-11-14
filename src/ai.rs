use bevy::prelude::*;
use hexx::Hex;
use crate::unit::{Unit, UnitType};
use crate::city::City;
use crate::terrain::HexTile;
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
}

impl Default for Civilizations {
    fn default() -> Self {
        Self {
            players: vec![
                Civ { id: 0, name: "Player".into(), is_human: true, gold: 0 },
                Civ { id: 1, name: "AI Rome".into(), is_human: false, gold: 50 },
                Civ { id: 2, name: "AI Egypt".into(), is_human: false, gold: 50 },
            ],
        }
    }
}

pub fn process_ai_turn(
    civs: &Civilizations,
    mut units: Query<&mut Unit>,
    cities: Query<&City>,
    tiles: Query<&HexTile>,
) {
    let mut rng = rand::thread_rng();

    for mut unit in &mut units {
        if unit.owner == 0 || unit.movement_left == 0 { continue; }

        // Settler AI: try to found city on nearby empty land
        if unit.unit_type == UnitType::Settler {
            let neighbors = unit.position.all_neighbors();
            for &n in &neighbors {
                let occupied = cities.iter().any(|c| c.position == n);
                let land = tiles.iter().any(|t| t.coord == n && t.terrain != crate::terrain::Terrain::Water);
                if !occupied && land {
                    // Would found here — for now just mark as moving toward it
                    unit.position = n;
                    unit.movement_left = 0;
                    info!("AI settler moved toward potential city site {:?}", n);
                    break;
                }
            }
            continue;
        }

        // Other units: random adjacent move
        let neighbors = unit.position.all_neighbors();
        if let Some(&target) = neighbors.get(rng.gen_range(0..neighbors.len())) {
            let land = tiles.iter().any(|t| t.coord == target && t.terrain != crate::terrain::Terrain::Water);
            if land {
                unit.position = target;
                unit.movement_left -= 1;
            }
        }
    }
}
