//! Great people: generals, merchants, scientists, artists and engineers.
//!
//! Cities earn great-person points every turn from their buildings, wonders and specialists. When a
//! city's points reach its civilization's threshold, a great person is born there: the kind is drawn
//! at random, weighted by the points of each kind. Each birth raises the civilization's threshold.
//! Generals are different: like Civilization III's great leaders, they come from battle - now and
//! then an elite unit's victory brings one forth.

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

/// Points needed for a civilization's first great person; each later one needs [`THRESHOLD_STEP`]
/// times one more than the one before (60, 180, 360, 600...), so great people grow rarer.
pub const FIRST_THRESHOLD: u32 = 60;
pub const THRESHOLD_STEP: u32 = 60;
/// The chance that a victory won by an elite unit brings forth a great general (Civilization III's
/// great leaders come this way).
pub const LEADER_CHANCE: f64 = 1.0 / 16.0;

/// Gold a great merchant settled in one of its own cities brings in every turn.
pub const SETTLED_MERCHANT_GOLD: u32 = 3;

/// What a great person does when used (shown on its button and in the unit panel).
pub fn ability(kind: UnitType) -> &'static str {
    match kind {
        UnitType::GreatGeneral => "Leads the troops it stands with (they fight half again as hard); in one of your cities, forms up to three units there into an army (four with Sun Tzu)",
        UnitType::GreatMerchant => "Trade mission in a foreign city not at war with you (gold now, more the farther from home), or settle in one of your cities (+3 gold there every turn)",
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
            Building::StockExchange => *merchant += 3,
            Building::Colossus | Building::GreatLighthouse | Building::MarcoPolo | Building::AdamSmith | Building::Magellan => *merchant += 3,
            Building::Copernicus | Building::Newton | Building::Darwin | Building::Internet | Building::Apollo => *scientist += 3,
            Building::Michelangelo | Building::Bach | Building::Shakespeare | Building::Mausoleum | Building::Eiffel | Building::Liberty | Building::CureForCancer => *artist += 3,
            Building::GreatWall | Building::Leonardo | Building::KingRichards | Building::HooverDam | Building::StatueOfZeus | Building::SunTzu => *engineer += 3,
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
    /// Great generals born in battle, by civilization (older saves kept victory points here).
    pub general: HashMap<u32, u32>,
    /// Masterpieces made by great artists, by city tile.
    #[serde(with = "crate::save::pairs")]
    pub masterpieces: HashMap<Hex, u32>,
    /// Great merchants settled in a city of their own people, by city tile.
    #[serde(default, with = "crate::save::pairs")]
    pub merchants: HashMap<Hex, u32>,
}

impl GreatPeople {
    /// Points a city of `owner` needs for the next great person.
    pub fn threshold(&self, owner: u32) -> u32 {
        let born = self.born.get(&owner).copied().unwrap_or(0);
        FIRST_THRESHOLD + THRESHOLD_STEP * born * (born + 3) / 2
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

    /// A battle won by a unit of `owner` that fought at this level (0 regular, 1 veteran, 2 elite).
    /// Returns true when the victory brings forth a great general: only an elite's can, one time in
    /// sixteen.
    pub fn victory(&mut self, owner: u32, level: u8, rng: &mut impl Rng) -> bool {
        let born = level >= 2 && rng.gen_bool(LEADER_CHANCE);
        if born {
            *self.general.entry(owner).or_default() += 1;
        }
        born
    }

    /// Points a city has gathered so far.
    pub fn progress(&self, at: Hex) -> u32 {
        self.city.get(&at).map(|p| p.iter().sum()).unwrap_or(0)
    }

    /// Gold a city earns each turn from great merchants settled there.
    pub fn merchant_gold(&self, at: Hex) -> i32 {
        (self.merchants.get(&at).copied().unwrap_or(0) * SETTLED_MERCHANT_GOLD) as i32
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
        assert_eq!(people.threshold(1), 180);
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
    fn generals_come_from_the_victories_of_elites() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        let mut people = GreatPeople::default();
        assert!((0..400).all(|_| !people.victory(0, 0, &mut rng) && !people.victory(0, 1, &mut rng)), "regulars and veterans never");
        let born = (0..1600).filter(|_| people.victory(0, 2, &mut rng)).count();
        assert!((70..130).contains(&born), "about one elite victory in sixteen: {born}");
        assert_eq!(people.general.get(&0).copied(), Some(born as u32));
    }
}
