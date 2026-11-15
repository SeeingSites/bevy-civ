use bevy::prelude::*;
use hexx::Hex;
use crate::terrain::Terrain;

#[derive(Component)]
pub struct NavalUnit {
    pub unit_type: NavalType,
    pub movement_left: u32,
    pub owner: u32,
    pub position: Hex,
    pub cargo: Vec<Entity>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NavalType {
    Galley,
    Trireme,
    Caravel,
    Frigate,
    Battleship,
    Transport,
}

impl NavalType {
    pub fn stats(&self) -> (u32, u32, u32, u32) {
        // attack, defense, movement, cargo capacity
        match self {
            NavalType::Galley => (1, 1, 2, 2),
            NavalType::Trireme => (2, 1, 3, 0),
            NavalType::Caravel => (3, 2, 4, 3),
            NavalType::Frigate => (5, 3, 4, 0),
            NavalType::Battleship => (10, 6, 4, 0),
            NavalType::Transport => (0, 1, 3, 6),
        }
    }

    pub fn cost(&self) -> u32 {
        match self {
            NavalType::Galley => 40,
            NavalType::Trireme => 50,
            NavalType::Caravel => 60,
            NavalType::Frigate => 80,
            NavalType::Battleship => 120,
            NavalType::Transport => 50,
        }
    }

    pub fn can_enter(&self, terrain: Terrain) -> bool {
        matches!(terrain, Terrain::Water)
    }
}

impl NavalUnit {
    pub fn load_cargo(&mut self, unit: Entity) -> bool {
        let capacity = self.unit_type.stats().3;
        if (self.cargo.len() as u32) < capacity {
            self.cargo.push(unit);
            true
        } else {
            false
        }
    }

    pub fn unload_cargo(&mut self) -> Option<Entity> {
        self.cargo.pop()
    }

    pub fn resolve_naval_combat(self_type: NavalType, enemy_type: NavalType) -> bool {
        let (atk, _, _, _) = self_type.stats();
        let (_, def, _, _) = enemy_type.stats();
        let atk_roll = atk as f32 * (0.8 + rand::random::<f32>() * 0.4);
        let def_roll = def as f32 * (0.8 + rand::random::<f32>() * 0.4);
        atk_roll > def_roll
    }
}
