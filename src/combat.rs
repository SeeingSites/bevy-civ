use bevy::prelude::*;
use crate::unit::UnitType;
use crate::terrain::Terrain;
use crate::promotions::UnitPromotions;

/// Civ-style combat with terrain, unique unit, and promotion bonuses.
/// Returns true if attacker wins.
pub fn resolve_combat(
    attacker: UnitType,
    defender: UnitType,
    defender_terrain: Terrain,
    attacker_promos: Option<&UnitPromotions>,
    defender_promos: Option<&UnitPromotions>,
) -> bool {
    let (atk, _, _) = attacker.stats();
    let (_, def, _) = defender.stats();

    let mut atk_mod = (atk as f32) * attacker.attack_bonus(defender_terrain);
    let mut def_mod = (def as f32) * defender.defense_bonus();

    if let Some(p) = attacker_promos {
        atk_mod *= 1.0 + p.total_attack_bonus();
    }
    if let Some(p) = defender_promos {
        def_mod *= 1.0 + p.total_defense_bonus();
    }

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

/// Award XP to a unit that participated in combat.
pub fn award_combat_xp(promos: &mut UnitPromotions, won: bool) {
    let amount = if won { 2 } else { 1 };
    promos.add_xp(amount);
}
