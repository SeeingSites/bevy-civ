//! Barbarians: raiders who appear in the wild lands outside every border, in bands that grow with
//! the age of the world and carry its weapons (warriors at first, rifles and artillery much later).

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

/// The band that rises when the world's peoples know `techs` technologies on average.
pub fn band(techs: usize) -> Vec<UnitType> {
    let pair = match techs {
        0..=7 => [UnitType::Warrior, UnitType::Warrior],
        8..=15 => [UnitType::Archer, UnitType::Horseman],
        16..=23 => [UnitType::Legion, UnitType::Horseman],
        24..=31 => [UnitType::Longbowman, UnitType::Knight],
        32..=39 => [UnitType::Musketman, UnitType::Cavalry],
        40..=45 => [UnitType::Rifleman, UnitType::Cavalry],
        _ => [UnitType::Infantry, UnitType::Artillery],
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
}
