use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct Diplomacy {
    pub relations: HashMap<(u32, u32), i32>,
    pub treaties: HashMap<(u32, u32), Treaty>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Treaty {
    None,
    Peace,
    Alliance,
    War,
}

impl Default for Diplomacy {
    fn default() -> Self {
        Self {
            relations: HashMap::new(),
            treaties: HashMap::new(),
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

    pub fn get_treaty(&self, a: u32, b: u32) -> Treaty {
        *self.treaties.get(&(a.min(b), a.max(b))).unwrap_or(&Treaty::None)
    }

    pub fn set_treaty(&mut self, a: u32, b: u32, treaty: Treaty) {
        let key = (a.min(b), a.max(b));
        self.treaties.insert(key, treaty);
        match treaty {
            Treaty::Peace => self.change_attitude(a, b, 10),
            Treaty::Alliance => self.change_attitude(a, b, 20),
            Treaty::War => self.change_attitude(a, b, -30),
            Treaty::None => {}
        }
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

    pub fn update(&mut self, population: u32, luxury_bonus: u32) {
        self.happy = 1 + luxury_bonus;
        self.unhappy = if population > 5 { (population - 5) / 2 } else { 0 };
        self.content = population.saturating_sub(self.happy + self.unhappy);
    }
}
