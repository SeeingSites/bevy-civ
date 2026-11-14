use bevy::prelude::*;
use hexx::Hex;

#[derive(Component)]
pub struct City {
    pub name: String,
    pub population: u32,
    pub food_storage: u32,
    pub production_progress: u32,
    pub production_queue: Option<ProductionItem>,
}

#[derive(Clone, Debug)]
pub enum ProductionItem {
    Unit(crate::unit::UnitType),
    Building(String),
}

impl City {
    pub fn production_per_turn(&self) -> u32 {
        // Simplified: base + population
        2 + self.population
    }

    pub fn food_per_turn(&self) -> u32 {
        // Will later pull from worked tiles
        3 + self.population
    }
}

pub fn spawn_city(commands: &mut Commands, coord: Hex, name: String, population: u32) {
    commands.spawn((
        City {
            name,
            population,
            food_storage: 0,
            production_progress: 0,
            production_queue: Some(ProductionItem::Unit(crate::unit::UnitType::Warrior)),
        },
        // Visual marker for city (simple circle for now)
        // TODO: better city sprite / icon
    ));
    info!("Founded city at {:?}", coord);
}
