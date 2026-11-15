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
