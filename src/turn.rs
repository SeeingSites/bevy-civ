use bevy::prelude::*;
use crate::city::{City, CityBuildings};
use crate::culture::{Culture, process_culture_turn};
use crate::economy::Economy;
use crate::government::Government;
use crate::trade::TradeRoutes;
use crate::promotions::UnitPromotions;
use crate::combat::{resolve_combat, award_combat_xp};
use crate::unit::UnitType;
use crate::terrain::Terrain;

/// Run end-of-turn systems that were previously defined but not always called.
pub fn process_extra_systems(
    mut culture_query: Query<(&City, &CityBuildings, &mut Culture)>,
    cities_for_economy: Query<(&City, &CityBuildings)>,
    mut economy: ResMut<Economy>,
    trade: Res<TradeRoutes>,
    mut gold: &mut i32,
    gov: Government,
) {
    process_culture_turn(culture_query);
    economy.process_turn(&cities_for_economy, &trade, gold, gov);
}

/// Resolve combat using promotions when available, and award XP.
pub fn resolve_combat_with_promotions(
    attacker_type: UnitType,
    defender_type: UnitType,
    terrain: Terrain,
    attacker_promos: Option<&mut UnitPromotions>,
    defender_promos: Option<&mut UnitPromotions>,
) -> bool {
    let atk_ref = attacker_promos.as_deref();
    let def_ref = defender_promos.as_deref();
    let wins = resolve_combat(attacker_type, defender_type, terrain, atk_ref, def_ref);

    if let Some(p) = attacker_promos {
        award_combat_xp(p, wins);
    }
    if let Some(p) = defender_promos {
        award_combat_xp(p, !wins);
    }
    wins
}
