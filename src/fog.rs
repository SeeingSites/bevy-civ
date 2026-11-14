use bevy::prelude::*;
use hexx::Hex;
use std::collections::HashSet;

#[derive(Resource)]
pub struct FogOfWar {
    pub explored: HashSet<Hex>,
    pub visible: HashSet<Hex>,
}

impl Default for FogOfWar {
    fn default() -> Self {
        Self {
            explored: HashSet::new(),
            visible: HashSet::new(),
        }
    }
}

impl FogOfWar {
    pub fn reveal(&mut self, center: Hex, radius: u32) {
        for hex in hexx::shapes::hexagon(center, radius) {
            self.explored.insert(hex);
            self.visible.insert(hex);
        }
    }

    pub fn update_visibility(&mut self, unit_positions: &[Hex], city_positions: &[Hex]) {
        self.visible.clear();
        for &pos in unit_positions.iter().chain(city_positions.iter()) {
            for hex in hexx::shapes::hexagon(pos, 2) {
                self.visible.insert(hex);
                self.explored.insert(hex);
            }
        }
    }
}
