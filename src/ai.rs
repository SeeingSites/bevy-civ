use bevy::prelude::*;
use crate::unit::Unit;
use crate::city::City;

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
                Civ { id: 1, name: "AI Rome".into(), is_human: false, gold: 0 },
                Civ { id: 2, name: "AI Egypt".into(), is_human: false, gold: 0 },
            ],
        }
    }
}

pub fn process_ai_turn(
    civs: &Civilizations,
    mut units: Query<&mut Unit>,
    cities: Query<&City>,
) {
    // Very simple AI: units with movement try to move randomly or expand
    for mut unit in &mut units {
        if unit.owner == 0 || unit.movement_left == 0 { continue; }
        // Placeholder AI logic
        info!("AI unit of owner {} has {} moves left", unit.owner, unit.movement_left);
    }
}
