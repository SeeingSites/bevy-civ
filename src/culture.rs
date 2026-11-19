//! Culture: what a city's temples, libraries, cathedrals, universities and wonders add up to over
//! time (as in Civ III, each building's share doubles once it has stood long enough). Culture
//! widens a city's borders, can make a restless neighbour defect, and a civilization far ahead of
//! everyone else - or one holding a city of unrivalled culture - wins a cultural victory.

use std::collections::HashMap;

use bevy::prelude::*;
use hexx::Hex;
use serde::{Deserialize, Serialize};

use crate::building::Building;

/// Culture a civilization needs for a cultural victory, with [`VICTORY_LEAD_TENTHS`] / 10 times
/// anyone else's - or that one of its cities needs alone ([`CITY_VICTORY_CULTURE`]), as in Civ III.
pub const VICTORY_CULTURE: u32 = 100_000;
/// How far ahead of the next civilization a cultural victory takes (in tenths: 20 = twice).
pub const VICTORY_LEAD_TENTHS: u32 = 20;
pub const CITY_VICTORY_CULTURE: u32 = 20_000;

/// Culture a cultural victory takes in a game of `last_turn` turns (100,000 in a 400-turn game).
pub fn victory_culture(last_turn: u32) -> u32 {
    VICTORY_CULTURE * last_turn.max(100) / 400
}

/// Culture a single city needs for a cultural victory in a game of `last_turn` turns (20,000 in a
/// 400-turn game).
pub fn city_victory_culture(last_turn: u32) -> u32 {
    CITY_VICTORY_CULTURE * last_turn.max(100) / 400
}
/// Chance a city in disorder next to a far greater culture defects each turn (before the
/// adjustments of [`defection_chance`]).
pub const DEFECTION_CHANCE: f64 = 0.2;
/// How near a greater culture must be to tempt a city.
pub const DEFECTION_REACH: i32 = 5;

/// Chance a city in disorder defects this turn to a far greater culture nearby. As in Civ III, a
/// city far from its own capital, and one whose land the tempter's borders already hold, goes more
/// readily, and troops in the city hold it back: each one divides the chance.
pub fn defection_chance(distance_to_capital: u32, tiles_held_by_tempter: u32, garrison: u32) -> f64 {
    let lure = (1.0 + distance_to_capital as f64 / 10.0) * (1.0 + tiles_held_by_tempter as f64 / 6.0);
    (DEFECTION_CHANCE * lure / (1 + garrison) as f64).min(0.6)
}

/// Culture the palace earns in a capital, and every wonder, each turn.
pub const PALACE_CULTURE: u32 = 1;
pub const WONDER_CULTURE: u32 = 4;

/// Culture a building earns each turn: a Temple, Colosseum, Research Lab or Forbidden Palace 2, a
/// Library or Cathedral 3, a University 4, a wonder [`WONDER_CULTURE`].
pub fn building_culture(building: Building) -> u32 {
    match building {
        Building::Temple | Building::Colosseum | Building::ResearchLab | Building::ForbiddenPalace => 2,
        Building::Library | Building::Cathedral => 3,
        Building::University => 4,
        b if b.is_wonder() => WONDER_CULTURE,
        _ => 0,
    }
}

/// Turns after which a building or wonder has stood long enough to earn double culture: a quarter
/// of the game (100 turns in a game of 400).
pub fn historic_after(last_turn: u32) -> u32 {
    (last_turn / 4).max(25)
}

/// Culture a city earns each turn from its buildings and wonders - double for each that has stood
/// `historic` turns (`age` says how long each has stood) - and from the palace in a capital.
pub fn per_turn(buildings: &[Building], capital: bool, age: impl Fn(Building) -> u32, historic: u32) -> u32 {
    let from_buildings: u32 = buildings
        .iter()
        .map(|b| {
            let points = building_culture(*b);
            if points > 0 && age(*b) >= historic { 2 * points } else { points }
        })
        .sum();
    from_buildings + if capital { PALACE_CULTURE } else { 0 }
}

/// Culture at which a city's borders widen: to 3 tiles at 100, 4 at 1,000 and 5 at 10,000 (from 2).
pub const BORDER_STEPS: [u32; 3] = [100, 1_000, 10_000];

/// How far a city's borders reach: 2 tiles, 3 from 100 culture, 4 from 1,000, 5 from 10,000.
pub fn border_radius(points: u32) -> u32 {
    2 + BORDER_STEPS.iter().filter(|step| points >= **step).count() as u32
}

/// Culture a city needs for its borders to widen next, if they still can.
pub fn next_border_step(points: u32) -> Option<u32> {
    BORDER_STEPS.into_iter().find(|step| points < *step)
}

/// Culture gathered by every city, by its tile (a captured city keeps its culture).
#[derive(Resource, Default, Clone, Debug, Serialize, Deserialize)]
pub struct CultureLedger {
    #[serde(with = "crate::save::pairs")]
    pub city: HashMap<Hex, u32>,
    /// The turn each city's buildings and wonders were completed, by the city's tile (anything not
    /// listed has stood since the game began).
    #[serde(default, with = "crate::save::pairs")]
    pub built: HashMap<Hex, Vec<(Building, u32)>>,
}

impl CultureLedger {
    pub fn of(&self, city: Hex) -> u32 {
        self.city.get(&city).copied().unwrap_or(0)
    }

    pub fn add(&mut self, city: Hex, points: u32) {
        *self.city.entry(city).or_default() += points;
    }

    /// Notes the turn a building or wonder was completed in a city.
    pub fn record(&mut self, city: Hex, building: Building, turn: u32) {
        let list = self.built.entry(city).or_default();
        list.retain(|(b, _)| *b != building);
        list.push((building, turn));
    }

    /// Turns a city's building has stood by `turn`.
    pub fn age(&self, city: Hex, building: Building, turn: u32) -> u32 {
        let since = self.built.get(&city).and_then(|list| list.iter().find(|(b, _)| *b == building)).map_or(0, |(_, t)| *t);
        turn.saturating_sub(since)
    }

    /// Culture a city earns each turn now (see [`per_turn`]), in a game of `last_turn` turns.
    pub fn earned(&self, city: Hex, buildings: &[Building], capital: bool, turn: u32, last_turn: u32) -> u32 {
        per_turn(buildings, capital, |b| self.age(city, b, turn), historic_after(last_turn))
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
pub fn cultural_winner(totals: &HashMap<u32, u32>, threshold: u32) -> Option<u32> {
    let (leader, best) = totals.iter().max_by_key(|(_, c)| **c).map(|(o, c)| (*o, *c))?;
    let second = totals.iter().filter(|(o, _)| **o != leader).map(|(_, c)| *c).max().unwrap_or(0);
    (best >= threshold && best * 10 >= second * VICTORY_LEAD_TENTHS).then_some(leader)
}

impl CultureLedger {
    /// The civilization (if any) that has won by culture in a game of `last_turn` turns, given who
    /// owns which city: by its total (see [`cultural_winner`]), or by a single city's culture.
    pub fn winner(&self, owners: &[(Hex, u32)], last_turn: u32) -> Option<u32> {
        cultural_winner(&self.totals(owners.iter().copied()), victory_culture(last_turn)).or_else(|| {
            owners.iter().filter(|(at, _)| self.of(*at) >= city_victory_culture(last_turn)).max_by_key(|(at, _)| self.of(*at)).map(|(_, owner)| *owner)
        })
    }

    /// The city with the most culture, and its owner.
    pub fn best_city(&self, owners: &[(Hex, u32)]) -> Option<(Hex, u32, u32)> {
        owners.iter().map(|(at, owner)| (*at, *owner, self.of(*at))).max_by_key(|(_, _, points)| *points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_and_foreign_borders_tempt_a_city_and_troops_hold_it() {
        let near = defection_chance(2, 0, 0);
        assert!((near - DEFECTION_CHANCE * 1.2).abs() < 1e-9);
        assert!(defection_chance(12, 0, 0) > near && defection_chance(2, 6, 0) > near);
        assert!((defection_chance(2, 0, 1) - near / 2.0).abs() < 1e-9);
        assert_eq!(defection_chance(40, 18, 0), 0.6, "never more than 60%");
    }

    #[test]
    fn old_buildings_earn_double_culture() {
        let new = |_: Building| 0;
        assert_eq!(per_turn(&[Building::Temple, Building::Cathedral, Building::Oracle, Building::Granary], true, new, 100), 2 + 3 + WONDER_CULTURE + PALACE_CULTURE);
        assert_eq!(per_turn(&[Building::Library, Building::University], false, new, 100), 3 + 4);
        let mut ledger = CultureLedger::default();
        let city = Hex::new(3, 1);
        ledger.record(city, Building::Temple, 10);
        ledger.record(city, Building::Library, 80);
        assert_eq!((ledger.age(city, Building::Temple, 110), ledger.age(city, Building::Library, 110)), (100, 30));
        assert_eq!(ledger.age(city, Building::Colosseum, 110), 110, "standing since the start");
        assert_eq!(historic_after(400), 100);
        // The Temple has stood 100 turns: double; the Library not yet.
        assert_eq!(ledger.earned(city, &[Building::Temple, Building::Library], false, 110, 400), 2 * 2 + 3);
        // Rebuilt, a building starts again.
        ledger.record(city, Building::Temple, 105);
        assert_eq!(ledger.earned(city, &[Building::Temple, Building::Library], false, 110, 400), 2 + 3);
    }

    #[test]
    fn culture_widens_borders_and_decides_the_cultural_victory() {
        assert_eq!((border_radius(0), border_radius(150), border_radius(9_999), border_radius(10_000)), (2, 3, 4, 5));
        assert_eq!((next_border_step(0), next_border_step(999), next_border_step(12_000)), (Some(100), Some(1_000), None));
        let mut ledger = CultureLedger::default();
        let threshold = victory_culture(400);
        assert_eq!((threshold, victory_culture(200)), (100_000, 50_000));
        assert_eq!((city_victory_culture(400), city_victory_culture(300)), (20_000, 15_000));
        // Many cities of modest culture: the total counts.
        let ours: Vec<(Hex, u32)> = (0..10).map(|i| (Hex::new(i * 4, 0), 0)).collect();
        let theirs = (Hex::new(0, 9), 1);
        for (at, _) in &ours {
            ledger.add(*at, 11_000);
        }
        ledger.add(theirs.0, 50_000);
        let owners: Vec<(Hex, u32)> = ours.iter().copied().chain([theirs]).collect();
        let totals = ledger.totals(owners.iter().copied());
        assert_eq!(cultural_winner(&totals, threshold), Some(0));
        // A rival not far enough behind: no win by the total - but a city of 20,000 wins alone.
        ledger.add(theirs.0, 10_000);
        let totals = ledger.totals(owners.iter().copied());
        assert_eq!(cultural_winner(&totals, threshold), None, "not twice the next");
        assert_eq!(ledger.winner(&owners, 400), Some(1), "their great city wins");
        assert_eq!(ledger.best_city(&owners), Some((theirs.0, 1, 60_000)));
        assert_eq!(CultureLedger::default().winner(&owners, 400), None, "no culture, no winner");
    }
}
