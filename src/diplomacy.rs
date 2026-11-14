use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct Diplomacy {
    pub relations: HashMap<(u32, u32), i32>,
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
    pub content: u32,
    pub unhappy: u32,
}

impl Happiness {
    pub fn new(population: u32) -> Self {
        // Simple model: base content, some become unhappy as city grows
        let happy = 1;
        let unhappy = if population > 4 { population - 4 } else { 0 };
        let content = population.saturating_sub(happy + unhappy);
        Self { happy, content, unhappy }
    }

    pub fn is_content(&self) -> bool {
        self.unhappy == 0
    }

    pub fn in_disorder(&self) -> bool {
        self.unhappy > self.happy + self.content
    }

    /// Recalculate based on population and luxury/buildings (simplified)
    pub fn update(&mut self, population: u32, luxury_bonus: u32) {
        self.happy = 1 + luxury_bonus;
        self.unhappy = if population > 6 { (population - 6) / 2 } else { 0 };
        self.content = population.saturating_sub(self.happy + self.unhappy);
    }
}
