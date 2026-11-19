//! Religion: four faiths, each founded by the first civilization to learn its technology. A faith
//! starts in its founder's capital (its holy city), spreads by itself to nearby cities that have
//! none, and is carried further by missionaries. A civilization's state religion is the faith of its
//! capital: cities that share it are calmer, the holy city's owner earns gold from every city of the
//! faith, and peoples of one faith get on better than peoples of different faiths.

use std::collections::HashMap;

use bevy::prelude::*;
use hexx::Hex;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::tech::Tech;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Faith {
    OldGods,
    WayOfReason,
    OneFaith,
    Reformed,
}

impl Faith {
    pub const ALL: [Faith; 4] = [
        Faith::OldGods,
        Faith::WayOfReason,
        Faith::OneFaith,
        Faith::Reformed,
    ];

    pub fn name(&self) -> &'static str {
        crate::theme::rename(crate::theme::Kind::Faith, self, self.base_name())
    }

    /// The name in the default (Earth) theme.
    pub fn base_name(&self) -> &'static str {
        match self {
            Faith::OldGods => "the Old Gods",
            Faith::WayOfReason => "the Way of Reason",
            Faith::OneFaith => "the One Faith",
            Faith::Reformed => "the Reformed Faith",
        }
    }

    /// The technology whose first discoverer founds this faith.
    pub fn founding_tech(&self) -> Tech {
        match self {
            Faith::OldGods => Tech::Mysticism,
            Faith::WayOfReason => Tech::Philosophy,
            Faith::OneFaith => Tech::Theology,
            Faith::Reformed => Tech::PrintingPress,
        }
    }
}

/// How far a faith spreads by itself from a city that holds it.
pub const SPREAD_REACH: i32 = 8;
/// Content citizens in a city that follows its owner's state religion (one more with a Temple).
pub const STATE_FAITH_CONTENT: u32 = 1;

/// Every city's faith and where each faith was founded.
#[derive(Resource, Default, Clone, Debug, Serialize, Deserialize)]
pub struct Religions {
    #[serde(with = "crate::save::pairs")]
    pub city: HashMap<Hex, Faith>,
    /// Holy city of each faith founded so far.
    #[serde(with = "crate::save::pairs")]
    pub holy: HashMap<Faith, Hex>,
}

impl Religions {
    pub fn of(&self, city: Hex) -> Option<Faith> {
        self.city.get(&city).copied()
    }

    pub fn is_founded(&self, faith: Faith) -> bool {
        self.holy.contains_key(&faith)
    }

    /// Founds a faith in a city (its holy city).
    pub fn found(&mut self, faith: Faith, at: Hex) {
        self.holy.insert(faith, at);
        self.city.insert(at, faith);
    }

    /// The faith a city is a holy city of, if any.
    pub fn holy_faith(&self, city: Hex) -> Option<Faith> {
        self.holy
            .iter()
            .find(|(_, at)| **at == city)
            .map(|(f, _)| *f)
    }

    /// Cities following each faith.
    pub fn followers(&self, faith: Faith) -> usize {
        self.city.values().filter(|f| **f == faith).count()
    }

    /// Gold the owner of a faith's holy city earns each turn: one for every two cities of the faith.
    pub fn holy_city_gold(&self, faith: Faith) -> i32 {
        (self.followers(faith) / 2) as i32
    }

    /// A turn of quiet spreading: each city without a faith may take up the faith of the nearest
    /// city that has one (likelier the nearer it is). Returns the cities converted.
    pub fn spread(&mut self, cities: &[Hex], rng: &mut impl Rng) -> Vec<(Hex, Faith)> {
        let mut converted = Vec::new();
        for &city in cities.iter().filter(|c| !self.city.contains_key(c)) {
            let nearest = self
                .city
                .iter()
                .filter(|(at, _)| at.distance_to(city) <= SPREAD_REACH)
                .min_by_key(|(at, _)| (at.distance_to(city), at.x, at.y));
            let Some((at, faith)) = nearest else { continue };
            let chance = (0.09 - 0.008 * at.distance_to(city) as f64).max(0.02);
            if rng.gen_bool(chance) {
                converted.push((city, *faith));
            }
        }
        for (city, faith) in &converted {
            self.city.insert(*city, *faith);
        }
        converted
    }

    /// Forgets cities that no longer exist (razed).
    pub fn retain(&mut self, cities: &[Hex]) {
        self.city.retain(|at, _| cities.contains(at));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn a_faith_spreads_to_nearby_cities_but_not_far_ones() {
        let mut religions = Religions::default();
        religions.found(Faith::OldGods, Hex::ZERO);
        let near = Hex::new(3, 0);
        let far = Hex::new(20, 0);
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        for _ in 0..200 {
            religions.spread(&[Hex::ZERO, near, far], &mut rng);
        }
        assert_eq!(religions.of(near), Some(Faith::OldGods));
        assert_eq!(religions.of(far), None);
        assert_eq!(religions.holy_faith(Hex::ZERO), Some(Faith::OldGods));
        assert_eq!(religions.followers(Faith::OldGods), 2);
        assert_eq!(religions.holy_city_gold(Faith::OldGods), 1);
    }
}
