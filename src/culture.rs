//! Culture: what a city's temples, libraries, cathedrals, universities and wonders add up to over
//! time. Culture widens a city's borders, can make a restless neighbour defect, and a civilization
//! far ahead of everyone else wins a cultural victory.

use std::collections::HashMap;

use bevy::prelude::*;
use hexx::Hex;
use serde::{Deserialize, Serialize};

use crate::building::Building;

/// Culture a civilization needs, with half as much again as anyone else, for a cultural victory.
pub const VICTORY_CULTURE: u32 = 10_000;
/// Chance a city in disorder next to a far greater culture defects each turn.
pub const DEFECTION_CHANCE: f64 = 0.2;
/// How near a greater culture must be to tempt a city.
pub const DEFECTION_REACH: i32 = 5;

/// Culture a city earns each turn from its buildings (and the palace in a capital).
pub fn per_turn(buildings: &[Building], capital: bool) -> u32 {
    let from_buildings: u32 = buildings
        .iter()
        .map(|b| match b {
            Building::Temple | Building::Library | Building::Colosseum => 1,
            Building::Cathedral | Building::University => 2,
            b if b.is_wonder() => 3,
            _ => 0,
        })
        .sum();
    from_buildings + u32::from(capital)
}

/// How far a city's borders reach: 2 tiles, 3 from 100 culture, 4 from 1,000.
pub fn border_radius(points: u32) -> u32 {
    match points {
        0..=99 => 2,
        100..=999 => 3,
        _ => 4,
    }
}

/// Culture gathered by every city, by its tile (a captured city keeps its culture).
#[derive(Resource, Default, Clone, Debug, Serialize, Deserialize)]
pub struct CultureLedger {
    #[serde(with = "crate::save::pairs")]
    pub city: HashMap<Hex, u32>,
}

impl CultureLedger {
    pub fn of(&self, city: Hex) -> u32 {
        self.city.get(&city).copied().unwrap_or(0)
    }

    pub fn add(&mut self, city: Hex, points: u32) {
        *self.city.entry(city).or_default() += points;
    }

    /// Each civilization's total, given who owns which city.
    pub fn totals(&self, owners: impl Iterator<Item = (Hex, u32)>) -> HashMap<u32, u32> {
        let mut totals = HashMap::new();
        for (city, owner) in owners {
            *totals.entry(owner).or_default() += self.of(city);
        }
        totals
    }
}

/// The civilization (if any) that has won by culture.
pub fn cultural_winner(totals: &HashMap<u32, u32>) -> Option<u32> {
    let (leader, best) = totals.iter().max_by_key(|(_, c)| **c).map(|(o, c)| (*o, *c))?;
    let second = totals.iter().filter(|(o, _)| **o != leader).map(|(_, c)| *c).max().unwrap_or(0);
    (best >= VICTORY_CULTURE && best * 2 >= second * 3).then_some(leader)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn culture_widens_borders_and_decides_the_cultural_victory() {
        assert_eq!(per_turn(&[Building::Temple, Building::Cathedral, Building::Oracle], true), 1 + 2 + 3 + 1);
        assert_eq!((border_radius(0), border_radius(150), border_radius(5000)), (2, 3, 4));
        let mut ledger = CultureLedger::default();
        ledger.add(Hex::ZERO, 12_000);
        ledger.add(Hex::new(5, 0), 7_000);
        let totals = ledger.totals([(Hex::ZERO, 0), (Hex::new(5, 0), 1)].into_iter());
        assert_eq!(cultural_winner(&totals), Some(0));
        ledger.add(Hex::new(5, 0), 2_000);
        let totals = ledger.totals([(Hex::ZERO, 0), (Hex::new(5, 0), 1)].into_iter());
        assert_eq!(cultural_winner(&totals), None, "not half as much again as the next");
    }
}
