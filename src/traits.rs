//! What sets each civilization apart: two traits and a unique unit.
//!
//! Traits give small, steady advantages in every city (more gold, science, shields, food or
//! contentment) or in war (troops promoted more often). A civilization's unique unit is one of the
//! ordinary units, trained by that civilization one rank higher (a veteran, or elite with Barracks).
//! Both follow from the civilization's name, so they need no saving.

use crate::unit::UnitType;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Trait {
    /// Troops are promoted after half their victories instead of a third.
    Militaristic,
    /// +1 gold in every city.
    Commercial,
    /// +1 science in every city.
    Scientific,
    /// +1 content citizen in every city, and revolutions take a turn less.
    Religious,
    /// +1 shield in every city.
    Industrious,
    /// +1 food in every city.
    Agricultural,
}

impl Trait {
    pub const ALL: [Trait; 6] = [Trait::Militaristic, Trait::Commercial, Trait::Scientific, Trait::Religious, Trait::Industrious, Trait::Agricultural];

    pub fn name(&self) -> &'static str {
        match self {
            Trait::Militaristic => "Militaristic",
            Trait::Commercial => "Commercial",
            Trait::Scientific => "Scientific",
            Trait::Religious => "Religious",
            Trait::Industrious => "Industrious",
            Trait::Agricultural => "Agricultural",
        }
    }

    pub fn effect(&self) -> &'static str {
        match self {
            Trait::Militaristic => "troops are promoted after half their victories",
            Trait::Commercial => "+1 gold in every city",
            Trait::Scientific => "+1 science in every city",
            Trait::Religious => "+1 content in every city, shorter revolutions",
            Trait::Industrious => "+1 shield in every city",
            Trait::Agricultural => "+1 food in every city",
        }
    }
}

/// A civilization's two traits and its unique unit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Character {
    pub traits: [Trait; 2],
    pub unique: UnitType,
}

impl Character {
    pub fn has(&self, t: Trait) -> bool {
        self.traits.contains(&t)
    }

    /// Extra (food, shields, gold, science) in each of its cities.
    pub fn city_bonus(&self) -> (u32, u32, u32, u32) {
        let n = |t: Trait| u32::from(self.has(t));
        (n(Trait::Agricultural), n(Trait::Industrious), n(Trait::Commercial), n(Trait::Scientific))
    }

    /// Extra content citizens in each of its cities.
    pub fn content(&self) -> u32 {
        u32::from(self.has(Trait::Religious))
    }

    /// Chance a victorious unit is promoted.
    pub fn promotion_chance(&self) -> f64 {
        if self.has(Trait::Militaristic) { 0.5 } else { 1.0 / 3.0 }
    }

    /// "Militaristic and Commercial".
    pub fn describe(&self) -> String {
        format!("{} and {}", self.traits[0].name(), self.traits[1].name())
    }
}

/// The character of the civilization with this name (any civilization not listed gets one drawn
/// from its name, so it is always the same).
pub fn of(name: &str) -> Character {
    use Trait::*;
    use UnitType as U;
    let key = name.trim_start_matches("the ");
    let (traits, unique) = match key {
        "Rome" => ([Militaristic, Commercial], U::Legion),
        "Carthage" => ([Commercial, Industrious], U::Horseman),
        "China" => ([Militaristic, Scientific], U::Longbowman),
        "Egypt" => ([Religious, Industrious], U::WarChariot),
        "England" => ([Commercial, Scientific], U::Frigate),
        "France" => ([Industrious, Commercial], U::Musketman),
        "Greece" => ([Scientific, Commercial], U::Phalanx),
        "India" => ([Religious, Commercial], U::Knight),
        "Japan" => ([Militaristic, Religious], U::Knight),
        "Macedon" => ([Militaristic, Scientific], U::Phalanx),
        "Mali" => ([Commercial, Agricultural], U::Horseman),
        "Muscovy" => ([Industrious, Agricultural], U::Cavalry),
        "Mongols" => ([Militaristic, Agricultural], U::Knight),
        "Persia" => ([Scientific, Industrious], U::Immortal),
        "Songhai" => ([Commercial, Agricultural], U::Horseman),
        "Spain" => ([Religious, Militaristic], U::Knight),
        "Aztecs" => ([Militaristic, Religious], U::Warrior),
        "Gauls" => ([Militaristic, Agricultural], U::Warrior),
        "Inca" => ([Agricultural, Industrious], U::Spearman),
        "Ottomans" => ([Militaristic, Scientific], U::Cavalry),
        "Seleucids" => ([Scientific, Militaristic], U::Knight),
        _ => {
            let seed = key.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
            let first = Trait::ALL[(seed % 6) as usize];
            let second = Trait::ALL[((seed / 6) % 5 + 1 + seed % 6) as usize % 6];
            let pool = [U::Legion, U::Horseman, U::Longbowman, U::Knight, U::Musketman, U::Cavalry, U::Phalanx];
            ([first, second], pool[(seed / 30) as usize % pool.len()])
        }
    };
    Character { traits, unique }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_civilization_has_two_different_traits() {
        for name in ["Rome", "the Aztecs", "Atlantis", "Nowhere", "the Observers", ""] {
            let c = of(name);
            assert_ne!(c.traits[0], c.traits[1], "{name}");
        }
        assert_eq!(of("the Inca"), of("Inca"));
        assert_eq!(of("Rome").unique, UnitType::Legion);
        assert_eq!(of("Rome").city_bonus(), (0, 0, 1, 0));
        assert_eq!(of("Egypt").content(), 1);
        assert!(of("Rome").promotion_chance() > of("Egypt").promotion_chance());
    }
}
