//! Espionage, carried out by diplomats.
//!
//! A diplomat standing in or next to a foreign city can open an embassy (always succeeds, and
//! reveals that civilization's cities and state of research) or attempt a covert mission. Covert
//! missions may fail; a diplomat is spent either way. A failure means the diplomat is caught: the
//! victim's opinion of the spy's civilization falls sharply.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use hexx::Hex;

use crate::tech::{Tech, TechState};

/// What a diplomat can do at a foreign city.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum Mission {
    Embassy,
    StealTech,
    Sabotage,
    StealGold,
    InciteUnrest,
}

/// Turns a city stands idle after unrest is incited.
pub const UNREST_TURNS: u32 = 3;
/// How much the victim's opinion falls when a spy is caught.
pub const CAUGHT_ATTITUDE: i32 = -25;

impl Mission {
    pub const ALL: [Mission; 5] = [Mission::Embassy, Mission::StealTech, Mission::Sabotage, Mission::StealGold, Mission::InciteUnrest];

    pub fn name(&self) -> &'static str {
        match self {
            Mission::Embassy => "Embassy",
            Mission::StealTech => "Steal Tech",
            Mission::Sabotage => "Sabotage",
            Mission::StealGold => "Steal Gold",
            Mission::InciteUnrest => "Incite Unrest",
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            Mission::Embassy => "see their cities and research (always succeeds)",
            Mission::StealTech => "learn one technology they know and you do not",
            Mission::Sabotage => "destroy the city's production so far",
            Mission::StealGold => "take a quarter of their treasury (up to 300)",
            Mission::InciteUnrest => "the city stands idle for 3 turns: no growth, no production",
        }
    }

    pub fn is_covert(&self) -> bool {
        *self != Mission::Embassy
    }

    /// Chance of success. City walls make covert work harder.
    pub fn odds(&self, walls: bool) -> f32 {
        let base = match self {
            Mission::Embassy => return 1.0,
            Mission::StealTech => 0.55,
            Mission::Sabotage | Mission::StealGold => 0.7,
            Mission::InciteUnrest => 0.5,
        };
        if walls { base - 0.15 } else { base }
    }
}

/// Technologies `victim` knows that `thief` does not.
pub fn stealable(thief: &TechState, victim: &TechState) -> Vec<Tech> {
    let mut techs: Vec<Tech> = victim.researched.difference(&thief.researched).copied().collect();
    techs.sort_by_key(|t| Tech::ALL.iter().position(|a| a == t));
    techs
}

/// Gold a successful theft takes from a treasury.
pub fn stolen_gold(treasury: i32) -> i32 {
    (treasury / 4).clamp(0, 300)
}

/// Embassies and cities in unrest.
#[derive(Resource, Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Espionage {
    /// (from, to): `from` has an embassy with `to`.
    pub embassies: HashSet<(u32, u32)>,
    /// Turns of unrest left, by city tile.
    #[serde(with = "crate::save::pairs")]
    pub unrest: HashMap<Hex, u32>,
}

impl Espionage {
    pub fn has_embassy(&self, from: u32, to: u32) -> bool {
        self.embassies.contains(&(from, to))
    }

    pub fn in_unrest(&self, city: Hex) -> bool {
        self.unrest.get(&city).is_some_and(|t| *t > 0)
    }

    pub fn incite(&mut self, city: Hex) {
        self.unrest.insert(city, UNREST_TURNS);
    }

    /// Counts a turn of unrest down everywhere.
    pub fn tick(&mut self) {
        self.unrest.retain(|_, turns| {
            *turns = turns.saturating_sub(1);
            *turns > 0
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embassies_always_succeed_and_walls_hinder_spies() {
        assert_eq!(Mission::Embassy.odds(true), 1.0);
        assert!(Mission::StealTech.odds(true) < Mission::StealTech.odds(false));
        assert!(Mission::ALL.iter().all(|m| m.odds(true) > 0.0));
    }

    #[test]
    fn only_unknown_techs_can_be_stolen() {
        let thief = TechState::knowing(&["Pottery".to_string()]);
        let victim = TechState::knowing(&["Pottery".to_string(), "Mining".to_string()]);
        assert_eq!(stealable(&thief, &victim), vec![Tech::Mining]);
        assert!(stealable(&victim, &thief).is_empty());
    }

    #[test]
    fn unrest_wears_off() {
        let mut spies = Espionage::default();
        let city = Hex::new(1, 1);
        spies.incite(city);
        for _ in 0..UNREST_TURNS - 1 {
            spies.tick();
            assert!(spies.in_unrest(city));
        }
        spies.tick();
        assert!(!spies.in_unrest(city));
        assert_eq!(stolen_gold(2000), 300);
        assert_eq!(stolen_gold(-5), 0);
    }
}
