//! The space race to the nearest star. Cities with a Factory build spaceship parts
//! (`Building::SpaceStructure`, `SpaceComponent`, `SpaceModule`, unlocked by Space Flight, Plastics
//! and Superconductors). A ship with every part it needs can be launched for Alpha Centauri, 4.37
//! light-years away; extra components (engines) make the voyage shorter. The first ship to arrive
//! wins. Losing the capital while the ship is in flight loses the ship.

use std::collections::HashMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::building::Building;

/// Parts a ship needs: structures, components, modules.
pub const NEEDED: [u32; 3] = [4, 2, 2];
/// The most of each part a ship can carry (more components, faster).
pub const MOST: [u32; 3] = [4, 4, 2];
/// Turns from launch to arrival with the fewest engines; each extra component saves two.
pub const FLIGHT_TURNS: u32 = 12;
/// Where the ships are bound, and how far.
pub const DESTINATION: &str = "Alpha Centauri";
pub const LIGHT_YEARS: f32 = 4.37;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ship {
    /// Structures, components and modules built so far.
    pub parts: [u32; 3],
    /// The turn the ship arrives, once launched.
    pub arrives: Option<u32>,
    /// The turn it was launched.
    #[serde(default)]
    pub launched: Option<u32>,
}

impl Ship {
    pub fn is_ready(&self) -> bool {
        self.parts.iter().zip(NEEDED).all(|(have, need)| *have >= need)
    }

    /// Turns the voyage takes with the engines aboard.
    pub fn flight_turns(&self) -> u32 {
        FLIGHT_TURNS - 2 * self.parts[1].saturating_sub(NEEDED[1]).min(MOST[1] - NEEDED[1])
    }

    /// How far along the voyage the ship is on `turn` (0 at launch, 1 on arrival).
    pub fn progress(&self, turn: u32) -> f32 {
        match (self.launched, self.arrives) {
            (Some(start), Some(end)) if end > start => ((turn.saturating_sub(start)) as f32 / (end - start) as f32).min(1.0),
            _ => 0.0,
        }
    }

    /// Sends the ship on its way. Returns the arrival turn.
    pub fn launch(&mut self, turn: u32) -> Option<u32> {
        if self.arrives.is_some() || !self.is_ready() {
            return None;
        }
        self.launched = Some(turn);
        self.arrives = Some(turn + self.flight_turns());
        self.arrives
    }

    /// Parts still missing, e.g. "2 structures, 1 module".
    pub fn missing(&self) -> String {
        let names = ["structure", "component", "module"];
        let parts: Vec<String> = self
            .parts
            .iter()
            .zip(NEEDED)
            .zip(names)
            .filter(|((have, need), _)| *have < need)
            .map(|((have, need), name)| {
                let left = need - have;
                format!("{left} {name}{}", if left == 1 { "" } else { "s" })
            })
            .collect();
        parts.join(", ")
    }
}

#[derive(Resource, Default, Clone, Debug, Serialize, Deserialize)]
pub struct SpaceRace {
    pub ships: HashMap<u32, Ship>,
}

impl SpaceRace {
    /// Adds a finished part. A computer player's ship (`auto_launch`) leaves as soon as it is
    /// ready; returns the arrival turn if it did.
    pub fn add_part(&mut self, owner: u32, part: Building, turn: u32, auto_launch: bool) -> Option<u32> {
        let index = match part {
            Building::SpaceStructure => 0,
            Building::SpaceComponent => 1,
            Building::SpaceModule => 2,
            _ => return None,
        };
        let ship = self.ships.entry(owner).or_default();
        if ship.arrives.is_some() {
            return None;
        }
        ship.parts[index] = (ship.parts[index] + 1).min(MOST[index]);
        if auto_launch {
            return ship.launch(turn);
        }
        None
    }

    /// The player sends a ready ship on its way.
    pub fn launch(&mut self, owner: u32, turn: u32) -> Option<u32> {
        self.ships.get_mut(&owner)?.launch(turn)
    }

    /// Whether `owner` can use more of this part: up to what the ship needs, or (`extra`, for the
    /// player who launches by hand) up to the most it can carry.
    pub fn wants(&self, owner: u32, part: Building, extra: bool) -> bool {
        let index = match part {
            Building::SpaceStructure => 0,
            Building::SpaceComponent => 1,
            Building::SpaceModule => 2,
            _ => return false,
        };
        let ship = self.ships.get(&owner);
        let limit = if extra { MOST[index] } else { NEEDED[index] };
        ship.is_none_or(|s| s.arrives.is_none() && s.parts[index] < limit)
    }

    /// The civilization whose ship has arrived by `turn`, if any.
    pub fn arrived(&self, turn: u32) -> Option<u32> {
        self.ships.iter().filter(|(_, s)| s.arrives.is_some_and(|t| t <= turn)).min_by_key(|(_, s)| s.arrives).map(|(o, _)| *o)
    }

    /// A ship in flight is lost when its capital falls.
    pub fn lose(&mut self, owner: u32) -> bool {
        self.ships.remove(&owner).is_some_and(|s| s.arrives.is_some())
    }

    /// Whether any part has been built yet (for the old progress line).
    pub fn is_complete(&self) -> bool {
        self.ships.values().any(|s| s.arrives.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ship_launches_with_every_part_and_arrives_later() {
        let mut race = SpaceRace::default();
        for _ in 0..4 {
            assert_eq!(race.add_part(0, Building::SpaceStructure, 100, true), None);
        }
        assert!(!race.wants(0, Building::SpaceStructure, false));
        race.add_part(0, Building::SpaceComponent, 101, true);
        race.add_part(0, Building::SpaceComponent, 102, true);
        race.add_part(0, Building::SpaceModule, 103, true);
        assert_eq!(race.ships[&0].missing(), "1 module");
        assert_eq!(race.add_part(0, Building::SpaceModule, 104, true), Some(104 + FLIGHT_TURNS));
        assert_eq!(race.arrived(104 + FLIGHT_TURNS - 1), None);
        assert_eq!(race.arrived(104 + FLIGHT_TURNS), Some(0));
        assert!(race.lose(0));
    }

    #[test]
    fn extra_engines_shorten_a_voyage_launched_by_hand() {
        let mut race = SpaceRace::default();
        for part in [Building::SpaceStructure; 4].into_iter().chain([Building::SpaceComponent; 3]).chain([Building::SpaceModule; 2]) {
            assert_eq!(race.add_part(0, part, 50, false), None, "the player launches by hand");
        }
        assert!(race.wants(0, Building::SpaceComponent, true));
        assert_eq!(race.ships[&0].flight_turns(), FLIGHT_TURNS - 2);
        assert_eq!(race.launch(0, 60), Some(60 + FLIGHT_TURNS - 2));
        assert!((race.ships[&0].progress(65) - 0.5).abs() < 0.01);
        assert_eq!(race.launch(0, 61), None);
    }
}
