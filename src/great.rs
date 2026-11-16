//! Great people: generals, merchants, scientists, artists and engineers.
//!
//! Cities earn great-person points every turn from their buildings, wonders and specialists. When a
//! city's points reach its civilization's threshold, a great person is born there: the kind is drawn
//! at random, weighted by the points of each kind. Each birth raises the civilization's threshold.
//! Generals are different: they come from victories in battle, counted for the whole civilization.

use std::collections::HashMap;

use bevy::prelude::*;
use hexx::Hex;
use rand::Rng;

use crate::building::Building;
use crate::city::Specialist;
use crate::unit::UnitType;

/// Points of each kind, in the order of [`CITY_KINDS`].
pub type Points = [u32; 4];

/// The great people a city can give birth to (generals come from battle).
pub const CITY_KINDS: [UnitType; 4] = [UnitType::GreatMerchant, UnitType::GreatScientist, UnitType::GreatArtist, UnitType::GreatEngineer];

/// Points needed for a civilization's first great person; each one born adds [`THRESHOLD_STEP`].
pub const FIRST_THRESHOLD: u32 = 40;
pub const THRESHOLD_STEP: u32 = 30;
/// General points per battle won, and the points needed for each general.
pub const GENERAL_PER_VICTORY: u32 = 10;
pub const GENERAL_THRESHOLD: u32 = 40;

/// What a great person does when used (shown on its button and in the unit panel).
pub fn ability(kind: UnitType) -> &'static str {
    match kind {
        UnitType::GreatGeneral => "Leads the troops it stands with: they fight half again as hard",
        UnitType::GreatMerchant => "Trade mission: in a foreign city not at war with you, earns gold (more the farther from home)",
        UnitType::GreatScientist => "Discovery: completes the current research at once",
        UnitType::GreatArtist => "Masterpiece: in one of your cities, +3 content citizens for good",
        UnitType::GreatEngineer => "Hurry: in one of your cities, finishes what it is building (wonders too)",
        _ => "",
    }
}

/// Great-person points a city earns each turn.
pub fn city_points(buildings: &[Building], specialists: &[Specialist], capital: bool) -> Points {
    let mut points: Points = [0; 4];
    let [merchant, scientist, artist, engineer] = &mut points;
    for building in buildings {
        match building {
            Building::Marketplace | Building::Harbor => *merchant += 1,
            Building::Bank => *merchant += 2,
            Building::Colossus | Building::GreatLighthouse => *merchant += 3,
            Building::Library => *scientist += 1,
            Building::University => *scientist += 2,
            Building::GreatLibrary => *scientist += 3,
            Building::Oracle | Building::HangingGardens => *artist += 3,
            Building::Aqueduct => *engineer += 1,
            Building::Factory => *engineer += 2,
            Building::Pyramids => *engineer += 3,
            _ => {}
        }
    }
    for specialist in specialists {
        match specialist {
            Specialist::TaxCollector => *merchant += 1,
            Specialist::Scientist => *scientist += 1,
            Specialist::Entertainer => *artist += 1,
        }
    }
    // The court of a capital patronises the arts.
    if capital {
        *artist += 1;
    }
    points
}

/// Draws the kind of great person a city's points produce, weighted by each kind's share.
pub fn pick_kind(points: &Points, rng: &mut impl Rng) -> UnitType {
    let total: u32 = points.iter().sum();
    if total == 0 {
        return UnitType::GreatArtist;
    }
    let mut roll = rng.gen_range(0..total);
    for (kind, share) in CITY_KINDS.iter().zip(points) {
        if roll < *share {
            return *kind;
        }
        roll -= share;
    }
    CITY_KINDS[CITY_KINDS.len() - 1]
}

/// Every civilization's progress towards its next great people.
#[derive(Resource, Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GreatPeople {
    /// Points gathered by each city, by its tile.
    #[serde(with = "crate::save::pairs")]
    pub city: HashMap<Hex, Points>,
    /// Great people born so far, by civilization (raises the next threshold).
    pub born: HashMap<u32, u32>,
    /// General points from battles won, by civilization.
    pub general: HashMap<u32, u32>,
    /// Masterpieces made by great artists, by city tile.
    #[serde(with = "crate::save::pairs")]
    pub masterpieces: HashMap<Hex, u32>,
}

impl GreatPeople {
    /// Points a city of `owner` needs for the next great person.
    pub fn threshold(&self, owner: u32) -> u32 {
        FIRST_THRESHOLD + THRESHOLD_STEP * self.born.get(&owner).copied().unwrap_or(0)
    }

    /// Adds a turn's points to a city. Returns the great person born there, if any.
    pub fn gather(&mut self, owner: u32, at: Hex, earned: Points, rng: &mut impl Rng) -> Option<UnitType> {
        let threshold = self.threshold(owner);
        let pool = self.city.entry(at).or_default();
        for (have, add) in pool.iter_mut().zip(earned) {
            *have += add;
        }
        if pool.iter().sum::<u32>() < threshold {
            return None;
        }
        let kind = pick_kind(pool, rng);
        *pool = [0; 4];
        *self.born.entry(owner).or_default() += 1;
        Some(kind)
    }

    /// Counts a battle won. Returns true when it brings forth a great general.
    pub fn victory(&mut self, owner: u32) -> bool {
        let points = self.general.entry(owner).or_default();
        *points += GENERAL_PER_VICTORY;
        if *points >= GENERAL_THRESHOLD {
            *points -= GENERAL_THRESHOLD;
            true
        } else {
            false
        }
    }

    /// Points a city has gathered so far.
    pub fn progress(&self, at: Hex) -> u32 {
        self.city.get(&at).map(|p| p.iter().sum()).unwrap_or(0)
    }

    /// Content citizens a city has from masterpieces.
    pub fn masterpiece_content(&self, at: Hex) -> u32 {
        self.masterpieces.get(&at).copied().unwrap_or(0) * 3
    }
}

/// Gold a trade mission brings: more the farther the city is from the merchant's capital.
pub fn trade_mission_gold(distance_from_capital: i32) -> i32 {
    (100 + 12 * distance_from_capital).min(400)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn buildings_and_wonders_earn_points_of_their_kind() {
        let points = city_points(&[Building::Library, Building::GreatLibrary, Building::Marketplace, Building::Pyramids], &[Specialist::Entertainer], true);
        assert_eq!(points, [1, 4, 2, 3]);
        assert_eq!(city_points(&[], &[], false), [0; 4]);
    }

    #[test]
    fn a_great_person_is_born_at_the_threshold_and_the_next_needs_more() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let mut people = GreatPeople::default();
        let city = Hex::new(2, 3);
        // Only scientist points: the birth must be a scientist.
        let mut born = None;
        for _ in 0..FIRST_THRESHOLD / 4 {
            born = born.or(people.gather(1, city, [0, 4, 0, 0], &mut rng));
        }
        assert_eq!(born, Some(UnitType::GreatScientist));
        assert_eq!(people.progress(city), 0);
        assert_eq!(people.threshold(1), FIRST_THRESHOLD + THRESHOLD_STEP);
        // Another civilization's threshold is untouched.
        assert_eq!(people.threshold(2), FIRST_THRESHOLD);
    }

    #[test]
    fn mixed_points_give_every_kind_in_proportion() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let mut merchants = 0;
        for _ in 0..1000 {
            if pick_kind(&[30, 10, 0, 0], &mut rng) == UnitType::GreatMerchant {
                merchants += 1;
            }
        }
        assert!((680..820).contains(&merchants), "{merchants}");
    }

    #[test]
    fn generals_come_from_victories() {
        let mut people = GreatPeople::default();
        let wins: Vec<bool> = (0..8).map(|_| people.victory(0)).collect();
        assert_eq!(wins.iter().filter(|w| **w).count(), 2);
        assert!(wins[3] && wins[7]);
    }
}
