//! The troubles of the modern city.
//!
//! - **Narcotics** (once its people know Sanitation): a big, unhappy city can fall into a drug
//!   trade, in up to three levels. Smugglers carry it in from neighbouring foreign cities that have
//!   it, unless their rulers have agreed to crack down. Each level makes a citizen unhappy and costs
//!   a gold of trade to the black market. A Police Station stops it spreading and roots it out.
//! - **Adulterated food** (once its people know Industrialization): in a city of size 5 or more
//!   without a Food Safety Agency, dealers may cut bread with chalk and milk with water. A scandal
//!   spoils half the granary and angers the people for a few turns. Corruption makes it likelier.
//! - **Genetically modified food** (Genetic Engineering): GM Farms add a food to every farmed tile
//!   the city works, but the people distrust the engineered food - one more unhappy citizen - unless
//!   a Food Safety Agency vouches for it.
//! - **Migration** (into peoples that know Industrialization): citizens leave a city that is unhappy,
//!   starving, troubled by drugs or by a food scandal, for a better-off city nearby - at home or
//!   abroad, wherever they are welcome.

use hexx::Hex;
use rand::Rng;

/// Highest level of a city's drug trade.
pub const NARCOTICS_MAX: u32 = 3;
/// Turns a food scandal keeps a city angry.
pub const SCANDAL_TURNS: u32 = 6;
/// Unhappy citizens while a food scandal lasts.
pub const SCANDAL_UNHAPPY: u32 = 2;
/// Turns a crackdown agreement keeps a people's smugglers at home.
pub const CRACKDOWN_TURNS: u32 = 30;
/// How far smugglers carry drugs, and how far migrants travel.
pub const SMUGGLING_RANGE: i32 = 6;
pub const MIGRATION_RANGE: i32 = 10;
/// Turns between waves of migration.
pub const MIGRATION_EVERY: u32 = 5;

/// A turn of the drug trade in one city: whether it gets worse (or, with a Police Station, better).
/// `restless`: its unhappy outnumber its happy. `smuggled`: drugs are carried in from a foreign city
/// nearby. Returns the new level.
pub fn narcotics_turn(
    level: u32,
    population: u32,
    restless: bool,
    smuggled: bool,
    police: bool,
    rng: &mut impl Rng,
) -> u32 {
    if police {
        // The police root it out, a level every few turns.
        return if level > 0 && rng.gen_bool(0.25) {
            level - 1
        } else {
            level
        };
    }
    if population < 7 || level >= NARCOTICS_MAX {
        return level;
    }
    let chance = 0.01 + if restless { 0.03 } else { 0.0 } + if smuggled { 0.03 } else { 0.0 };
    if rng.gen_bool(chance) {
        level + 1
    } else {
        level
    }
}

/// The chance this turn of an adulterated-food scandal in a city, by its corruption (0 to 0.5).
pub fn scandal_chance(population: u32, corruption: f32) -> f64 {
    if population < 5 {
        return 0.0;
    }
    0.004 + corruption.clamp(0.0, 0.5) as f64 * 0.04
}

/// Unhappy citizens from a city's troubles: its drug trade, a food scandal, and distrust of GM food.
pub fn unhappiness(narcotics: u32, scandal_turns: u32, gm_distrusted: bool) -> u32 {
    narcotics
        + if scandal_turns > 0 {
            SCANDAL_UNHAPPY
        } else {
            0
        }
        + u32::from(gm_distrusted)
}

/// A city as migrants weigh it.
#[derive(Clone, Copy, Debug)]
pub struct Prospect {
    pub at: Hex,
    pub owner: u32,
    pub population: u32,
    /// Its people are content enough: happy minus unhappy.
    pub mood: i32,
    pub food_balance: i32,
    pub narcotics: u32,
    pub scandal: bool,
    /// It has room to grow.
    pub room: bool,
}

impl Prospect {
    /// How good a place it is to live.
    pub fn appeal(&self) -> i32 {
        self.mood * 2 + self.food_balance.clamp(-4, 4)
            - self.narcotics as i32 * 2
            - if self.scandal { 3 } else { 0 }
    }

    /// Whether its people would leave: unhappy, hungry or troubled (and big enough to spare one).
    pub fn pushes(&self) -> bool {
        self.population >= 3
            && (self.mood < 0 || self.food_balance < 0 || self.narcotics >= 2 || self.scandal)
    }
}

/// Where a citizen leaving `from` goes: the most appealing city within reach that has room, is
/// clearly better, and whose people welcome them (`welcomes(owner, from_owner)`).
pub fn destination(
    from: &Prospect,
    cities: &[Prospect],
    welcomes: impl Fn(u32, u32) -> bool,
) -> Option<Hex> {
    cities
        .iter()
        .filter(|c| c.at != from.at && c.room && c.at.distance_to(from.at) <= MIGRATION_RANGE)
        .filter(|c| c.appeal() >= from.appeal() + 3 && welcomes(c.owner, from.owner))
        .max_by_key(|c| (c.appeal() * 10 - c.at.distance_to(from.at), c.at.x, c.at.y))
        .map(|c| c.at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn drugs_take_hold_in_big_unhappy_cities_and_the_police_root_them_out() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let mut level = 0;
        for _ in 0..300 {
            level = narcotics_turn(level, 12, true, true, false, &mut rng);
        }
        assert_eq!(level, NARCOTICS_MAX);
        // A small city never gets them.
        assert_eq!(
            (0..300).fold(0, |l, _| narcotics_turn(l, 4, true, true, false, &mut rng)),
            0
        );
        // The police clear them.
        assert_eq!(
            (0..100).fold(NARCOTICS_MAX, |l, _| narcotics_turn(
                l, 12, true, true, true, &mut rng
            )),
            0
        );
        assert_eq!(unhappiness(2, 3, true), 2 + SCANDAL_UNHAPPY + 1);
        assert!(scandal_chance(8, 0.5) > scandal_chance(8, 0.0));
        assert_eq!(scandal_chance(3, 0.5), 0.0);
    }

    #[test]
    fn migrants_leave_troubled_cities_for_better_ones_that_welcome_them() {
        let city = |x: i32, owner: u32, mood: i32| Prospect {
            at: Hex::new(x, 0),
            owner,
            population: 6,
            mood,
            food_balance: 2,
            narcotics: 0,
            scandal: false,
            room: true,
        };
        let miserable = Prospect {
            narcotics: 2,
            ..city(0, 1, -2)
        };
        assert!(miserable.pushes());
        assert!(!city(0, 1, 2).pushes());
        let near_and_happy = city(3, 2, 3);
        let far_and_happier = city(20, 3, 5);
        let cities = [miserable, near_and_happy, far_and_happier];
        assert_eq!(
            destination(&miserable, &cities, |_, _| true),
            Some(Hex::new(3, 0))
        );
        // Turned away: nowhere else within reach.
        assert_eq!(destination(&miserable, &cities, |to, _| to != 2), None);
    }
}
