use bevy::prelude::*;
use hexx::Hex;
use crate::city::City;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Religion {
    None,
    Polytheism,
    Monotheism,
    Philosophy,
}

impl Religion {
    pub fn name(&self) -> &'static str {
        match self {
            Religion::None => "None",
            Religion::Polytheism => "Polytheism",
            Religion::Monotheism => "Monotheism",
            Religion::Philosophy => "Philosophy",
        }
    }

    pub fn happiness_bonus(&self) -> u32 {
        match self {
            Religion::None => 0,
            Religion::Polytheism => 1,
            Religion::Monotheism => 2,
            Religion::Philosophy => 1,
        }
    }

    pub fn science_penalty(&self) -> f32 {
        match self {
            Religion::None => 0.0,
            Religion::Polytheism => 0.05,
            Religion::Monotheism => 0.1,
            Religion::Philosophy => -0.1, // bonus
        }
    }
}

#[derive(Component)]
pub struct CityReligion {
    pub religion: Religion,
    pub influence: u32,
}

impl CityReligion {
    pub fn new() -> Self {
        Self {
            religion: Religion::None,
            influence: 0,
        }
    }

    pub fn spread(&mut self, from: Religion, amount: u32) {
        if self.religion == Religion::None || self.religion == from {
            self.religion = from;
            self.influence += amount;
        } else if self.influence > 0 {
            self.influence = self.influence.saturating_sub(amount);
            if self.influence == 0 {
                self.religion = from;
                self.influence = amount;
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct ReligionState {
    pub founded: Vec<Religion>,
}

impl ReligionState {
    pub fn found(&mut self, religion: Religion) -> bool {
        if !self.founded.contains(&religion) && religion != Religion::None {
            self.founded.push(religion);
            true
        } else {
            false
        }
    }

    pub fn spread_from_city(&self, city_pos: Hex, religion: Religion, cities: &mut Query<&mut CityReligion>) {
        // Simple spread to nearby cities handled externally
    }
}
