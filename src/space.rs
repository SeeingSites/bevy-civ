//! The space race. Cities with a Factory build spaceship parts (`Building::SpaceStructure`,
//! `SpaceComponent`, `SpaceModule`, unlocked by Space Flight, Plastics and Superconductors). Once a
//! civilization's ship has every part it launches, and arrives after [`FLIGHT_TURNS`] turns: the
//! first to arrive wins. Losing the capital while the ship is in flight loses the ship.

use std::collections::HashMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::building::Building;

/// Parts a ship needs: structures, components, modules.
pub const NEEDED: [u32; 3] = [4, 2, 2];
/// Turns from launch to arrival.
pub const FLIGHT_TURNS: u32 = 12;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ship {
    /// Structures, components and modules built so far.
    pub parts: [u32; 3],
    /// The turn the ship arrives, once launched.
    pub arrives: Option<u32>,
}

impl Ship {
    pub fn is_ready(&self) -> bool {
        self.parts.iter().zip(NEEDED).all(|(have, need)| *have >= need)
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
    /// Adds a finished part. Returns the arrival turn if this launches the ship.
    pub fn add_part(&mut self, owner: u32, part: Building, turn: u32) -> Option<u32> {
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
        ship.parts[index] += 1;
        if ship.is_ready() {
            ship.arrives = Some(turn + FLIGHT_TURNS);
            return ship.arrives;
        }
        None
    }

    /// Whether `owner` still needs more of this part (a city should not build surplus parts).
    pub fn wants(&self, owner: u32, part: Building) -> bool {
        let index = match part {
            Building::SpaceStructure => 0,
            Building::SpaceComponent => 1,
            Building::SpaceModule => 2,
            _ => return false,
        };
        let ship = self.ships.get(&owner);
        ship.is_none_or(|s| s.arrives.is_none() && s.parts[index] < NEEDED[index])
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
            assert_eq!(race.add_part(0, Building::SpaceStructure, 100), None);
        }
        assert!(!race.wants(0, Building::SpaceStructure));
        race.add_part(0, Building::SpaceComponent, 101);
        race.add_part(0, Building::SpaceComponent, 102);
        race.add_part(0, Building::SpaceModule, 103);
        assert_eq!(race.ships[&0].missing(), "1 module");
        assert_eq!(race.add_part(0, Building::SpaceModule, 104), Some(104 + FLIGHT_TURNS));
        assert_eq!(race.arrived(104 + FLIGHT_TURNS - 1), None);
        assert_eq!(race.arrived(104 + FLIGHT_TURNS), Some(0));
        assert!(race.lose(0));
    }
}
