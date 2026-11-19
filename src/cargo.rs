//! Ships that carry other units: troops ride in Galleys, Galleons, Frigates and Transports, and
//! aircraft on Carriers. Land units cannot cross the sea by themselves: they step aboard a ship
//! with room (on the sea next to them, or in port), ride with it, and go ashore onto a land tile
//! next to the ship, which takes the rest of their turn. Aircraft that land on a carrier move with
//! it, and the carrier serves them as an airfield. A ship that sinks takes its cargo with it.
//!
//! Each carried unit remembers its ship and the tile the ship stood on ([`Unit::carrier`]). Once a
//! frame, [`stow`] works out what has changed: ships that sailed carry their cargo along, units
//! that left the ship's tile by themselves have gone ashore, and units that stepped onto a ship
//! (or were loaded in a saved game) are taken aboard.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use hexx::Hex;

use crate::unit::{Domain, Unit, UnitType};

/// What [`stow`] needs to know of each unit.
#[derive(Clone, Copy, Debug)]
pub struct Berth {
    pub entity: Entity,
    pub unit_type: UnitType,
    pub owner: u32,
    pub position: Hex,
    pub carrier: Option<(Entity, Hex)>,
}

impl Berth {
    pub fn of(entity: Entity, unit: &Unit) -> Self {
        Berth {
            entity,
            unit_type: unit.unit_type,
            owner: unit.owner,
            position: unit.position,
            carrier: unit.carrier,
        }
    }
}

/// A change to one unit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Change {
    /// Its ship sailed: the unit goes with it.
    Carried { entity: Entity, to: Hex },
    /// It left its ship (went ashore, took off, or stayed in port when the ship sailed).
    Ashore { entity: Entity },
    /// It is now aboard this ship.
    Aboard {
        entity: Entity,
        ship: Entity,
        at: Hex,
    },
    /// Troops at sea with no ship under them: lost with it.
    Lost { entity: Entity },
}

/// Units aboard each ship.
pub fn loads(units: &[Berth]) -> HashMap<Entity, u32> {
    let mut load: HashMap<Entity, u32> = HashMap::new();
    for berth in units {
        if let Some((ship, _)) = berth.carrier {
            *load.entry(ship).or_default() += 1;
        }
    }
    load
}

/// A ship of `owner` at `at` with room for a unit of this kind, the emptiest first.
pub fn ship_with_room(units: &[Berth], owner: u32, at: Hex, cargo: UnitType) -> Option<Entity> {
    let load = loads(units);
    units
        .iter()
        .filter(|s| s.owner == owner && s.position == at && s.unit_type.carries(cargo))
        .map(|s| {
            (
                s.entity,
                s.unit_type
                    .capacity()
                    .saturating_sub(load.get(&s.entity).copied().unwrap_or(0)),
            )
        })
        .filter(|(_, room)| *room > 0)
        .max_by_key(|(e, room)| (*room, std::cmp::Reverse(e.index())))
        .map(|(e, _)| e)
}

/// Works out what follows from this frame's movements (see the module notes).
pub fn stow(units: &[Berth], water: &HashSet<Hex>) -> Vec<Change> {
    let ships: HashMap<Entity, (Hex, u32)> = units
        .iter()
        .filter(|u| u.unit_type.capacity() > 0)
        .map(|u| (u.entity, (u.position, u.owner)))
        .collect();
    let mut changes = Vec::new();
    let mut load: HashMap<Entity, u32> = HashMap::new();
    let mut loose: Vec<&Berth> = Vec::new();
    for berth in units {
        let Some((ship, at)) = berth.carrier else {
            loose.push(berth);
            continue;
        };
        match ships.get(&ship) {
            // Moved off the ship's tile by itself.
            _ if berth.position != at => {
                changes.push(Change::Ashore {
                    entity: berth.entity,
                });
                loose.push(berth);
            }
            Some((ship_at, owner)) if *owner == berth.owner => {
                *load.entry(ship).or_default() += 1;
                if *ship_at != at {
                    changes.push(Change::Carried {
                        entity: berth.entity,
                        to: *ship_at,
                    });
                }
            }
            // The ship is gone.
            _ => {
                changes.push(Change::Ashore {
                    entity: berth.entity,
                });
                loose.push(berth);
            }
        }
    }
    for berth in loose {
        let domain = berth.unit_type.domain();
        let at_sea = water.contains(&berth.position);
        if !at_sea || !matches!(domain, Domain::Land | Domain::Air) {
            continue;
        }
        let candidates = units.iter().filter(|s| {
            s.owner == berth.owner
                && s.position == berth.position
                && s.unit_type.carries(berth.unit_type)
        });
        let mut best: Option<(Entity, u32)> = None;
        for ship in candidates {
            let room = ship
                .unit_type
                .capacity()
                .saturating_sub(load.get(&ship.entity).copied().unwrap_or(0));
            if best.is_none_or(|(_, r)| room > r) {
                best = Some((ship.entity, room));
            }
        }
        match best {
            // Troops always go aboard a ship that is there (orders already checked for room);
            // aircraft only land on a carrier with room, and otherwise stay in the air.
            Some((ship, room)) if room > 0 || domain == Domain::Land => {
                *load.entry(ship).or_default() += 1;
                changes.push(Change::Aboard {
                    entity: berth.entity,
                    ship,
                    at: berth.position,
                });
            }
            None if domain == Domain::Land => changes.push(Change::Lost {
                entity: berth.entity,
            }),
            _ => {}
        }
    }
    changes
}

/// Applies one change to a unit (troops are kept out of the round of orders while at sea).
pub fn apply(unit: &mut Unit, change: Change, water: &HashSet<Hex>) {
    match change {
        Change::Carried { to, .. } => {
            unit.position = to;
            if let Some((_, at)) = unit.carrier.as_mut() {
                *at = to;
            }
        }
        Change::Ashore { .. } => unit.carrier = None,
        Change::Aboard { ship, at, .. } => {
            unit.carrier = Some((ship, at));
            if unit.unit_type.domain() == Domain::Land && water.contains(&at) {
                unit.fortified = true;
            }
        }
        Change::Lost { .. } => {}
    }
}

/// Where a player's aircraft may fly from: their cities and their carriers.
pub fn airfields(cities: impl Iterator<Item = Hex>, units: &[Berth], owner: u32) -> Vec<Hex> {
    cities
        .chain(
            units
                .iter()
                .filter(|u| u.owner == owner && u.unit_type == UnitType::Carrier)
                .map(|u| u.position),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn berth(index: u32, unit_type: UnitType, position: Hex, carrier: Option<(u32, Hex)>) -> Berth {
        Berth {
            entity: Entity::from_raw(index),
            unit_type,
            owner: 1,
            position,
            carrier: carrier.map(|(e, at)| (Entity::from_raw(e), at)),
        }
    }

    #[test]
    fn troops_go_aboard_ride_along_and_go_ashore() {
        let sea = Hex::new(1, 0);
        let water: HashSet<Hex> = [sea, Hex::new(2, 0)].into();
        // A rifleman steps onto a galleon's tile: it goes aboard (and sleeps).
        let units = [
            berth(0, UnitType::Galleon, sea, None),
            berth(1, UnitType::Rifleman, sea, None),
        ];
        let changes = stow(&units, &water);
        assert_eq!(
            changes,
            vec![Change::Aboard {
                entity: Entity::from_raw(1),
                ship: Entity::from_raw(0),
                at: sea
            }]
        );
        let mut rifleman = crate::unit::Unit {
            unit_type: UnitType::Rifleman,
            movement_left: 1,
            owner: 1,
            position: sea,
            build_turns_left: None,
            current_build: None,
            fortified: false,
            road_credit: 0,
            hp: 3,
            level: 0,
            troops: Vec::new(),
            fatigue: 0,
            carrier: None,
        };
        apply(&mut rifleman, changes[0], &water);
        assert!(rifleman.fortified && rifleman.carrier.is_some());
        // The galleon sails on: the rifleman goes with it.
        let units = [
            berth(0, UnitType::Galleon, Hex::new(2, 0), None),
            berth(1, UnitType::Rifleman, sea, Some((0, sea))),
        ];
        assert_eq!(
            stow(&units, &water),
            vec![Change::Carried {
                entity: Entity::from_raw(1),
                to: Hex::new(2, 0)
            }]
        );
        // The rifleman steps ashore by itself.
        let units = [
            berth(0, UnitType::Galleon, sea, None),
            berth(1, UnitType::Rifleman, Hex::new(1, 1), Some((0, sea))),
        ];
        assert_eq!(
            stow(&units, &water),
            vec![Change::Ashore {
                entity: Entity::from_raw(1)
            }]
        );
        // The galleon is sunk: the troops at sea are lost.
        let units = [berth(1, UnitType::Rifleman, sea, Some((0, sea)))];
        assert_eq!(
            stow(&units, &water),
            vec![
                Change::Ashore {
                    entity: Entity::from_raw(1)
                },
                Change::Lost {
                    entity: Entity::from_raw(1)
                }
            ]
        );
    }

    #[test]
    fn carriers_take_aircraft_up_to_their_room() {
        let sea = Hex::new(1, 0);
        let water: HashSet<Hex> = [sea].into();
        let mut units = vec![berth(0, UnitType::Carrier, sea, None)];
        for i in 1..=5 {
            units.push(berth(i, UnitType::Fighter, sea, None));
        }
        let aboard = stow(&units, &water)
            .iter()
            .filter(|c| matches!(c, Change::Aboard { .. }))
            .count();
        assert_eq!(aboard, 4, "the fifth fighter stays in the air");
        assert!(!UnitType::Carrier.carries(UnitType::Rifleman));
        assert!(!UnitType::Galleon.carries(UnitType::Fighter));
        assert_eq!(
            ship_with_room(&units, 1, sea, UnitType::Fighter),
            Some(Entity::from_raw(0))
        );
        assert_eq!(
            airfields([Hex::ZERO].into_iter(), &units, 1),
            vec![Hex::ZERO, sea]
        );
    }
}
