use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct Diplomacy {
    pub relations: HashMap<(u32, u32), i32>, // (civ_a, civ_b) -> attitude -100 to 100
}

impl Default for Diplomacy {
    fn default() -> Self {
        Self {
            relations: HashMap::new(),
        }
    }
}

impl Diplomacy {
    pub fn get_attitude(&self, a: u32, b: u32) -> i32 {
        *self.relations.get(&(a.min(b), a.max(b))).unwrap_or(&0)
    }

    pub fn change_attitude(&mut self, a: u32, b: u32, delta: i32) {
        let key = (a.min(b), a.max(b));
        *self.relations.entry(key).or_insert(0) += delta;
    }
}

#[derive(Component)]
pub struct Happiness {
    pub happy: u32,
    pub unhappy: u32,
}

impl Happiness {
    pub fn is_content(&self) -> bool {
        self.unhappy == 0
    }
}
