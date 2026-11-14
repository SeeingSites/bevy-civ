use bevy::prelude::*;
use hexx::Hex;

#[derive(Component)]
pub struct Unit {
    pub unit_type: UnitType,
    pub movement_left: u32,
    pub owner: u32, // player id
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UnitType {
    Settler,
    Worker,
    Warrior,
    Spearman,
    Archer,
    Horseman,
}

impl UnitType {
    pub fn stats(&self) -> (u32, u32, u32) { // attack, defense, movement
        match self {
            UnitType::Settler => (0, 0, 1),
            UnitType::Worker => (0, 0, 1),
            UnitType::Warrior => (1, 1, 1),
            UnitType::Spearman => (1, 2, 1),
            UnitType::Archer => (2, 1, 1),
            UnitType::Horseman => (2, 1, 2),
        }
    }

    pub fn cost(&self) -> u32 {
        match self {
            UnitType::Settler => 30,
            UnitType::Worker => 10,
            UnitType::Warrior => 10,
            UnitType::Spearman => 20,
            UnitType::Archer => 20,
            UnitType::Horseman => 20,
        }
    }
}

pub fn spawn_unit(commands: &mut Commands, coord: Hex, unit_type: UnitType) {
    let (atk, def, mv) = unit_type.stats();
    commands.spawn((
        Unit {
            unit_type,
            movement_left: mv,
            owner: 0,
        },
        // TODO: visual representation (colored circle or sprite)
    ));
    info!("Spawned {:?} (A{} D{} M{}) at {:?}", unit_type, atk, def, mv, coord);
}
