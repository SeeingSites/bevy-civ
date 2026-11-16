use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct Diplomacy {
    #[serde(with = "crate::save::pairs")]
    pub relations: HashMap<(u32, u32), i32>,
    #[serde(with = "crate::save::pairs")]
    pub treaties: HashMap<(u32, u32), Treaty>,
    /// Pairs of peoples who let each other's units pass through and share tiles.
    #[serde(default)]
    pub passage: std::collections::HashSet<(u32, u32)>,
}

#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
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
            passage: Default::default(),
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

    /// Whether `a` and `b` let each other's units pass (a right of passage, or an alliance).
    pub fn has_passage(&self, a: u32, b: u32) -> bool {
        self.passage.contains(&(a.min(b), a.max(b))) || self.get_treaty(a, b) == Treaty::Alliance
    }

    pub fn grant_passage(&mut self, a: u32, b: u32) {
        self.passage.insert((a.min(b), a.max(b)));
    }

    pub fn set_treaty(&mut self, a: u32, b: u32, treaty: Treaty) {
        let key = (a.min(b), a.max(b));
        self.treaties.insert(key, treaty);
        if treaty == Treaty::War {
            self.passage.remove(&key);
        }
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

/// Citizens content without help; the rest of a bigger city are unhappy.
pub const BASE_CONTENT: u32 = 5;

impl Happiness {
    pub fn new(population: u32) -> Self {
        let mut happiness = Self { happy: 0, content: 0, unhappy: 0 };
        happiness.update(population, 0, 0);
        happiness
    }

    pub fn is_content(&self) -> bool {
        self.unhappy == 0
    }

    /// A city falls into disorder when its unhappy citizens outnumber its happy ones.
    pub fn in_disorder(&self) -> bool {
        self.unhappy > self.happy
    }

    /// `calm` makes unhappy citizens content (temples, wonders, martial law, entertainers);
    /// `joy` makes content citizens happy (luxuries).
    pub fn update(&mut self, population: u32, calm: u32, joy: u32) {
        let unhappy = population.saturating_sub(BASE_CONTENT).saturating_sub(calm);
        let happy = joy.min(population - unhappy);
        self.unhappy = unhappy;
        self.happy = happy;
        self.content = population - unhappy - happy;
    }
}

#[cfg(test)]
mod happiness_tests {
    use super::*;

    #[test]
    fn big_cities_need_temples_or_luxuries() {
        let mut h = Happiness::new(5);
        assert!(!h.in_disorder());
        h.update(8, 0, 0);
        assert!(h.in_disorder());
        h.update(8, 2, 1); // a temple and a luxury
        assert_eq!((h.happy, h.content, h.unhappy), (1, 6, 1));
        assert!(!h.in_disorder());
    }
}

#[cfg(test)]
mod passage_tests {
    use super::*;

    #[test]
    fn passage_comes_with_alliance_or_a_deal_and_ends_with_war() {
        let mut d = Diplomacy::default();
        assert!(!d.has_passage(0, 2));
        d.set_treaty(0, 2, Treaty::Peace);
        d.grant_passage(2, 0);
        assert!(d.has_passage(0, 2));
        d.set_treaty(0, 2, Treaty::War);
        assert!(!d.has_passage(0, 2));
        d.set_treaty(0, 3, Treaty::Alliance);
        assert!(d.has_passage(3, 0));
    }
}
