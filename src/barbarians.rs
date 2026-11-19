//! Barbarians: raiders who appear in the wild lands outside every border, in bands that grow with
//! the age of the world and carry its weapons (warriors at first, rifles much later).
//!
//! Modern raiders rise inside a people's own lands, out of its cities' troubles, and fight on the
//! barbarians' side:
//! - **Cartel gunmen** form near a city with a drug trade (likelier the worse it is), unless it has a
//!   Police Station. While they are near, its drug trade cannot be rooted out; in an undefended city they
//!   extort gold and melt away. Killing them seizes their assets.
//! - **Insurgents** (a terror cell) form, once a people knows Electronics, in a city with grievances:
//!   disorder, occupation (still resisting a conqueror), a faith not its ruler's, or long war. A
//!   Police Station stops cells forming in its city. In an undefended city they wreck a building
//!   and kill a citizen, and are spent - unless the police foil the plot.

use std::collections::HashMap;

use bevy::prelude::*;
use hexx::Hex;
use rand::Rng;

use crate::terrain::HexTile;
use crate::unit::{spawn_unit, UnitType};

/// Barbarians' owner id.
pub const BARBARIANS: u32 = 99;
/// Turns between bands.
pub const SPAWN_EVERY: u32 = 8;
/// No barbarians in the first turns of a game.
pub const QUIET_TURNS: u32 = 10;

#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct BarbarianState {
    pub spawn_timer: u32,
}

/// Most modern raiders allowed near one city at a time.
pub const RAIDERS_NEAR: usize = 2;
/// Gold seized from a cartel when its gunmen are killed.
pub const SEIZED_GOLD: i32 = 25;
/// Chance a city's Police Station foils an insurgents' plot.
pub const POLICE_FOIL: f64 = 0.5;

/// What breeds a terror cell in a city.
#[derive(Clone, Copy, Debug, Default)]
pub struct Grievance {
    pub disorder: bool,
    /// Still resisting a conqueror.
    pub occupied: bool,
    /// Follows a faith other than its ruler's.
    pub foreign_faith: bool,
    /// Its people have been at war a long time.
    pub war_weary: bool,
}

impl Grievance {
    pub fn count(&self) -> u32 {
        [self.disorder, self.occupied, self.foreign_faith, self.war_weary].iter().filter(|g| **g).count() as u32
    }

    /// The worst of them, for the news.
    pub fn reason(&self) -> &'static str {
        if self.occupied {
            "resentment of its occupiers"
        } else if self.disorder {
            "unrest in its streets"
        } else if self.foreign_faith {
            "anger at its rulers' faith"
        } else {
            "weariness of a long war"
        }
    }
}

/// The chance this turn that cartel gunmen form near a city with this level of drug trade.
pub fn cartel_chance(narcotics: u32, police: bool) -> f64 {
    if police || narcotics == 0 { 0.0 } else { 0.02 * narcotics as f64 - 0.01 }
}

/// The chance this turn that a terror cell forms in a city with these grievances.
pub fn terror_chance(grievance: &Grievance, police: bool) -> f64 {
    if police { 0.0 } else { 0.01 * grievance.count() as f64 }
}

/// Gold cartel gunmen extort from an undefended city's people.
pub fn extortion(population: u32, treasury: i32) -> i32 {
    (10 + population as i32 * 4).min(treasury.max(0))
}

/// The band that rises when the world's peoples know `techs` technologies on average.
pub fn band(techs: usize) -> Vec<UnitType> {
    let pair = match techs {
        0..=7 => [UnitType::Warrior, UnitType::Warrior],
        8..=15 => [UnitType::Archer, UnitType::Horseman],
        16..=23 => [UnitType::Legion, UnitType::Horseman],
        24..=31 => [UnitType::Longbowman, UnitType::Knight],
        32..=39 => [UnitType::Musketman, UnitType::Cavalry],
        40..=45 => [UnitType::Rifleman, UnitType::Cavalry],
        _ => [UnitType::Infantry, UnitType::Cavalry],
    };
    let size = match techs {
        0..=11 => 1,
        12..=29 => 2,
        _ => 3,
    };
    (0..size).map(|i| pair[i % 2]).collect()
}

/// Every few turns a band appears on wild land: outside every border and at least 4 tiles from any
/// unit.
pub fn process_barbarians(
    state: &mut BarbarianState,
    commands: &mut Commands,
    tiles: &Query<&mut HexTile>,
    unit_positions: &[Hex],
    territory: &HashMap<Hex, (u32, Hex)>,
    turn: u32,
    world_techs: usize,
) {
    state.spawn_timer += 1;
    if state.spawn_timer < SPAWN_EVERY || turn < QUIET_TURNS {
        return;
    }
    state.spawn_timer = 0;
    let mut rng = rand::thread_rng();
    let wild: Vec<Hex> = tiles
        .iter()
        .filter(|t| t.terrain != crate::terrain::Terrain::Water && !territory.contains_key(&t.coord))
        .map(|t| t.coord)
        .filter(|at| unit_positions.iter().all(|p| p.distance_to(*at) >= 4))
        .collect();
    if wild.is_empty() {
        return;
    }
    let camp = wild[rng.gen_range(0..wild.len())];
    // The band gathers on the camp and the wild land beside it.
    let spots: Vec<Hex> = std::iter::once(camp).chain(camp.all_neighbors().into_iter().filter(|h| wild.contains(h))).collect();
    for (i, kind) in band(world_techs).into_iter().enumerate() {
        spawn_unit(commands, spots[i % spots.len()], kind, BARBARIANS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_grow_and_rearm_with_the_world() {
        assert_eq!(band(0), vec![UnitType::Warrior]);
        assert_eq!(band(20).len(), 2);
        assert!(band(50).contains(&UnitType::Infantry) && band(50).len() == 3);
    }

    #[test]
    fn modern_raiders_rise_from_drugs_and_grievance_unless_policed() {
        assert_eq!(cartel_chance(0, false), 0.0);
        assert!(cartel_chance(1, false) > 0.0);
        assert!(cartel_chance(3, false) > cartel_chance(2, false));
        assert_eq!(cartel_chance(3, true), 0.0);
        let angry = Grievance { disorder: true, occupied: true, ..Default::default() };
        assert_eq!(angry.count(), 2);
        assert!(terror_chance(&angry, false) > 0.0);
        assert_eq!(terror_chance(&angry, true), 0.0);
        assert_eq!(terror_chance(&Grievance::default(), false), 0.0);
        assert_eq!(angry.reason(), "resentment of its occupiers");
        assert_eq!(extortion(5, 1000), 30);
        assert_eq!(extortion(5, 12), 12);
        assert!(UnitType::Cartel.is_raider() && !UnitType::Cartel.is_trainable() && UnitType::Insurgent.is_army());
    }
}
