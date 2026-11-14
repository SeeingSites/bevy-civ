use bevy::prelude::*;
use crate::unit::UnitType;
use crate::terrain::Terrain;

/// Simple Civ-style combat resolution with unique unit bonuses.
/// Returns true if attacker wins.
pub fn resolve_combat(
    attacker: UnitType,
    defender: UnitType,
    defender_terrain: Terrain,
) -> bool {
    let (atk, _, _) = attacker.stats();
    let (_, def, _) = defender.stats();

    let mut atk_mod = (atk as f32) * attacker.attack_bonus(defender_terrain);
    let mut def_mod = (def as f32) * defender.defense_bonus();

    // Terrain defense bonuses
    match defender_terrain {
        Terrain::Hills => def_mod *= 1.5,
        Terrain::Mountain => def_mod *= 2.0,
        Terrain::Forest => def_mod *= 1.5,
        Terrain::Grassland | Terrain::Plains => def_mod *= 1.0,
        _ => {}
    }

    let attack_roll = atk_mod * rand_factor();
    let defense_roll = def_mod * rand_factor();

    attack_roll > defense_roll
}

fn rand_factor() -> f32 {
    0.8 + (rand::random::<f32>() * 0.4)
}

pub fn log_combat(attacker: UnitType, defender: UnitType, terrain: Terrain, attacker_wins: bool) {
    let result = if attacker_wins { "ATTACKER WINS" } else { "DEFENDER HOLDS" };
    info!(
        "Combat: {:?} vs {:?} on {:?} → {}",
        attacker, defender, terrain, result
    );
}
