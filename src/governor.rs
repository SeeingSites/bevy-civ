//! City management for computer players: what each of their cities should build next.

use hexx::Hex;
use rand::Rng;

use bevy_civ::building::Building;
use bevy_civ::city::ProductionItem;
use bevy_civ::tech::TechState;
use bevy_civ::unit::{Domain, UnitType};

use crate::supply::Supplies;

/// What the governor knows about the city it is deciding for.
pub struct CityView {
    pub position: Hex,
    pub population: u32,
    pub built: Vec<Building>,
    /// Next to water, so it can have a harbour.
    pub coastal: bool,
    /// Joined to the capital's trade network.
    pub linked: bool,
    /// A military unit of the owner stands in the city.
    pub defended: bool,
    /// An enemy army is close.
    pub threatened: bool,
    /// Within 6 tiles of its people's capital (no place for a second seat of government).
    pub near_capital: bool,
    /// Level of its drug trade.
    pub narcotics: u32,
}

/// The state of the civilization as a whole, including what its other cities have already queued.
pub struct CivView<'a> {
    pub tech: &'a TechState,
    pub aggression: f32,
    pub cities: usize,
    pub workers: usize,
    pub settlers: usize,
    /// Land forces.
    pub military: usize,
    pub navy: usize,
    pub air: usize,
    /// Sources and cities still waiting for a road.
    pub road_work: usize,
    /// Cities not joined to the capital's network.
    pub cut_off: usize,
    pub at_war: bool,
    /// Wonders already standing somewhere in the world.
    pub wonders_taken: &'a std::collections::HashSet<Building>,
    pub caravans: usize,
    pub diplomats: usize,
    pub missionaries: usize,
    /// Has a state religion and cities of its own that do not follow it yet.
    pub faith_to_spread: bool,
    /// Another city of this civilization is building a wonder (caravans can help it).
    pub wonder_elsewhere: bool,
    /// Spaceship parts its ship still needs.
    pub space_parts: &'a [Building],
    /// Free land for new cities lies within reach of its cities.
    pub room_to_expand: bool,
    /// Already has a Forbidden Palace somewhere.
    pub forbidden_palace: bool,
    /// Ships it has (or is building) for carrying troops overseas.
    pub ferries: usize,
    /// Its units waiting in port for a ship.
    pub ferry_wanted: usize,
}

/// Largest army a computer player keeps.
pub fn army_cap(cities: usize) -> usize {
    // Two units a city while small; past eight cities, two more for every three (a great empire
    // fields a greater army, which is what lets it overwhelm smaller neighbours).
    (4 + 2 * cities).min(6 + cities * 3 / 2)
}

fn can_train(unit: UnitType, city: &CityView, civ: &CivView, supplies: &Supplies) -> bool {
    unit.is_trainable()
        && unit.required_tech().is_none_or(|t| civ.tech.researched.contains(&t))
        && unit.required_wonder().is_none_or(|w| civ.wonders_taken.contains(&w))
        && supplies.has(city.position, unit.required_resources())
}

pub fn can_build(building: &Building, city: &CityView, civ: &CivView, supplies: &Supplies) -> bool {
    !city.built.contains(building)
        && !(building.is_wonder() && civ.wonders_taken.contains(building))
        && (!building.is_spaceship_part() || (city.built.contains(&Building::Factory) && civ.space_parts.contains(building)))
        && building.required_tech().is_none_or(|t| civ.tech.researched.contains(&t))
        && supplies.has(city.position, building.required_resources())
        && (!building.needs_coast() || city.coastal)
        && !(building.one_per_people() && civ.forbidden_palace)
        && building.required_building().is_none_or(|b| city.built.contains(&b))
}

/// The unit that holds a city best.
pub fn best_defender(city: &CityView, civ: &CivView, supplies: &Supplies) -> UnitType {
    UnitType::ALL
        .into_iter()
        .filter(|u| u.is_army() && can_train(*u, city, civ, supplies))
        .max_by(|a, b| {
            let score = |u: &UnitType| u.stats().1 as f32 * u.defense_bonus() + u.stats().0 as f32 * 0.1 - u.cost() as f32 * 0.001;
            score(a).total_cmp(&score(b))
        })
        .unwrap_or(UnitType::Warrior)
}

/// The unit that takes the field best.
pub fn best_attacker(city: &CityView, civ: &CivView, supplies: &Supplies) -> UnitType {
    UnitType::ALL
        .into_iter()
        .filter(|u| u.is_army() && can_train(*u, city, civ, supplies))
        .max_by(|a, b| {
            let score = |u: &UnitType| u.stats().0 as f32 + u.stats().1 as f32 * 0.4 + u.stats().2 as f32 * 0.5;
            score(a).total_cmp(&score(b))
        })
        .unwrap_or(UnitType::Warrior)
}

/// Picks the next project: safety first, then the workers and harbours that connect the empire,
/// then an army sized to the situation, then growth and buildings.
pub fn choose(city: &CityView, civ: &CivView, supplies: &Supplies, rng: &mut impl Rng) -> ProductionItem {
    let unit = ProductionItem::Unit;
    if !city.defended {
        return unit(best_defender(city, civ, supplies));
    }
    // A young empire with land to fill expands first: two settlers at a time, from any city that
    // has grown past its first citizen (unless an enemy is close).
    if civ.room_to_expand && civ.cities < 6 && civ.settlers < 2 && city.population >= 2 && !city.threatened {
        return unit(UnitType::Settler);
    }
    let wanted_workers = (civ.cities.div_ceil(2) + usize::from(civ.road_work > 3 * civ.workers.max(1))).min(civ.cities.max(1));
    if civ.workers < wanted_workers {
        return unit(worker_kind(civ));
    }
    // Harbours join coasts: a cut-off city needs one, and so does the mainland it must reach.
    if (!city.linked || civ.cut_off > 0) && can_build(&Building::Harbor, city, civ, supplies) {
        return ProductionItem::Building(Building::Harbor);
    }
    // Troops or settlers waiting in port and no ship to carry them: a coastal city builds one.
    if city.coastal && civ.ferry_wanted > 0 && civ.ferries < civ.ferry_wanted.div_ceil(6).min(3) {
        let ferry = [UnitType::Transport, UnitType::Galleon, UnitType::Galley].into_iter().find(|u| can_train(*u, city, civ, supplies));
        if let Some(ferry) = ferry {
            return unit(ferry);
        }
    }
    let cap = army_cap(civ.cities);
    let wanted_army = if civ.at_war { 2 * civ.cities + 2 } else { civ.cities + 1 }.min(cap);
    if civ.military < wanted_army {
        let offensive = civ.at_war || civ.aggression > 0.6;
        return unit(if offensive { best_attacker(city, civ, supplies) } else { best_defender(city, civ, supplies) });
    }
    // At war, a coastal city adds warships and an industrial power adds aircraft.
    if civ.at_war {
        let strongest = |domain: Domain| {
            UnitType::ALL
                .into_iter()
                .filter(|u| u.domain() == domain && !u.is_ferry() && *u != UnitType::Carrier && can_train(*u, city, civ, supplies))
                .max_by_key(|u| u.stats().0 + u.stats().1)
        };
        if city.coastal && civ.navy < 2 {
            if let Some(ship) = strongest(Domain::Sea) {
                return unit(ship);
            }
        }
        if civ.air < 2 {
            if let Some(plane) = strongest(Domain::Air) {
                return unit(plane);
            }
        }
    }
    // Expand while there is room: a settler at a time for a small empire, more for a big one.
    let settlers_wanted = 1 + civ.cities / 6;
    if civ.room_to_expand && civ.settlers < settlers_wanted && civ.cities < 24 && city.population >= 3 && !city.threatened && rng.gen_bool(0.5) {
        return unit(UnitType::Settler);
    }
    // Caravans carry shields to a wonder going up elsewhere, or open trade routes.
    let caravan_wanted = civ.wonder_elsewhere || (civ.cities >= 2 && rng.gen_bool(0.2));
    if civ.caravans < 2 && city.population >= 4 && caravan_wanted && can_train(UnitType::Caravan, city, civ, supplies) {
        return unit(UnitType::Caravan);
    }
    // One diplomat to open embassies and spy, once the civilization has a few cities.
    if civ.faith_to_spread && civ.missionaries == 0 && rng.gen_bool(0.25) && can_train(UnitType::Missionary, city, civ, supplies) {
        return unit(UnitType::Missionary);
    }
    if civ.diplomats == 0 && civ.cities >= 3 && rng.gen_bool(0.3) && can_train(UnitType::Diplomat, city, civ, supplies) {
        return unit(UnitType::Diplomat);
    }

    // The space race comes first once a civilization can build its ship.
    if let Some(part) = civ.space_parts.iter().find(|p| can_build(p, city, civ, supplies)) {
        return ProductionItem::Building(*part);
    }
    // A drug trade is dealt with first.
    if city.narcotics > 0 && can_build(&Building::PoliceStation, city, civ, supplies) {
        return ProductionItem::Building(Building::PoliceStation);
    }
    let wish = [
        (Building::Walls, city.threatened || civ.at_war),
        (Building::Temple, city.population >= bevy_civ::diplomacy::BASE_CONTENT),
        (Building::Aqueduct, city.population >= 7),
        (Building::Granary, city.population < 8),
        (Building::Barracks, civ.at_war || civ.aggression > 0.6),
        (Building::Library, true),
        (Building::Marketplace, true),
        (Building::Harbor, true),
        (Building::Courthouse, civ.cities >= 4),
        (Building::ForbiddenPalace, civ.cities >= 8 && !city.near_capital),
        (Building::Colosseum, city.population >= 8),
        (Building::Cathedral, city.population >= 10),
        (Building::SewerSystem, city.population >= 15),
        (Building::University, true),
        (Building::Bank, true),
        (Building::StockExchange, city.population >= 8),
        (Building::Factory, true),
        (Building::Observatory, true),
        (Building::Supermarket, city.population >= 12),
        (Building::PoliceStation, city.population >= 12 || city.narcotics > 0),
        (Building::CoastalFortress, civ.at_war),
        (Building::PowerPlant, true),
        (Building::ResearchLab, true),
        (Building::FoodSafety, city.population >= 9),
        (Building::GmFarms, city.population >= 10 && city.built.contains(&Building::FoodSafety)),
        (Building::MassTransit, city.population >= 10),
        (Building::RecyclingCenter, city.population >= 10),
        (Building::Airport, civ.at_war),
    ];
    if let Some((building, _)) = wish.into_iter().find(|(b, wanted)| *wanted && can_build(b, city, civ, supplies)) {
        return ProductionItem::Building(building);
    }
    if city.population >= 6 {
        if let Some(wonder) = Building::ALL.into_iter().find(|b| b.is_wonder() && can_build(b, city, civ, supplies)) {
            return ProductionItem::Building(wonder);
        }
    }
    // At war, a people with the bomb now and then builds a missile.
    if civ.at_war && can_train(UnitType::NuclearMissile, city, civ, supplies) && rng.gen_bool(0.04) {
        return unit(UnitType::NuclearMissile);
    }
    if civ.military < cap {
        return unit(best_attacker(city, civ, supplies));
    }
    // Nothing left to build: a few spare hands at most. More would only pile up (a city that has
    // everything keeps producing); the army is capped, so its units are simply not raised.
    if civ.workers < civ.cities + 2 {
        return unit(worker_kind(civ));
    }
    unit(best_defender(city, civ, supplies))
}

/// Engineers once the civilization can train them, workers before.
fn worker_kind(civ: &CivView) -> UnitType {
    if UnitType::Engineers.required_tech().is_some_and(|t| civ.tech.researched.contains(&t)) { UnitType::Engineers } else { UnitType::Worker }
}

#[cfg(test)]
mod tests {
    use super::*;

    static NO_WONDERS: std::sync::LazyLock<std::collections::HashSet<Building>> = std::sync::LazyLock::new(Default::default);
    use bevy_civ::tech::Tech;
    use bevy_civ::terrain::Resource as Good;
    use std::collections::HashSet;

    fn city() -> CityView {
        CityView { position: Hex::ZERO, population: 4, built: vec![], coastal: false, linked: true, defended: true, threatened: false, near_capital: true, narcotics: 0 }
    }

    fn civ(tech: &TechState) -> CivView<'_> {
        CivView { tech, aggression: 0.4, cities: 2, workers: 1, settlers: 2, military: 3, navy: 0, air: 0, road_work: 0, cut_off: 0, at_war: false, wonders_taken: &NO_WONDERS, caravans: 2, diplomats: 1, missionaries: 0, faith_to_spread: false, wonder_elsewhere: false, space_parts: &[], room_to_expand: true, forbidden_palace: false, ferries: 0, ferry_wanted: 0 }
    }

    fn knowing(techs: &[Tech]) -> TechState {
        let mut state = TechState::default();
        state.researched.extend(techs.iter().copied());
        state
    }

    fn supplies_with(goods: &[Good]) -> Supplies {
        let mut supplies = Supplies::default();
        supplies.city.insert(Hex::ZERO, goods.iter().copied().collect::<HashSet<_>>());
        supplies
    }

    #[test]
    fn an_undefended_city_trains_its_best_defender_first() {
        let tech = knowing(&[Tech::BronzeWorking]);
        let view = CityView { defended: false, ..city() };
        let pick = choose(&view, &civ(&tech), &Supplies::default(), &mut rand::thread_rng());
        assert_eq!(pick, ProductionItem::Unit(UnitType::Phalanx));
    }

    #[test]
    fn units_need_their_resources() {
        let tech = knowing(&[Tech::IronWorking, Tech::BronzeWorking]);
        let view = city();
        let at_war = CivView { at_war: true, military: 0, ..civ(&tech) };
        assert_ne!(best_attacker(&view, &at_war, &Supplies::default()), UnitType::Legion);
        assert_eq!(best_attacker(&view, &at_war, &supplies_with(&[Good::Iron])), UnitType::Legion);
    }

    #[test]
    fn a_city_with_a_drug_trade_builds_a_police_station() {
        let tech = knowing(&[Tech::Pottery, Tech::Sanitation]);
        let troubled = CityView { narcotics: 1, population: 9, ..city() };
        assert_eq!(choose(&troubled, &civ(&tech), &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Building(Building::PoliceStation));
    }

    #[test]
    fn a_young_empire_with_room_expands_first() {
        let tech = knowing(&[Tech::Pottery]);
        let young = CivView { settlers: 0, ..civ(&tech) };
        assert_eq!(choose(&city(), &young, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(UnitType::Settler));
        // Not with an enemy close, nor once the land is full.
        let threatened = CityView { threatened: true, ..city() };
        assert_ne!(choose(&threatened, &young, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(UnitType::Settler));
        let full = CivView { room_to_expand: false, ..young };
        assert_ne!(choose(&city(), &full, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(UnitType::Settler));
    }

    #[test]
    fn a_cut_off_coastal_city_builds_a_harbour() {
        let tech = knowing(&[Tech::Seafaring, Tech::Pottery]);
        let view = CityView { coastal: true, linked: false, ..city() };
        assert_eq!(choose(&view, &civ(&tech), &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Building(Building::Harbor));
        // The mainland builds one too while part of the empire is cut off.
        let mainland = CityView { coastal: true, linked: true, ..city() };
        let split = CivView { cut_off: 1, ..civ(&tech) };
        assert_eq!(choose(&mainland, &split, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Building(Building::Harbor));
        let inland = CityView { coastal: false, linked: false, ..city() };
        assert_ne!(choose(&inland, &civ(&tech), &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Building(Building::Harbor));
    }

    #[test]
    fn a_settled_peaceful_city_develops() {
        let tech = knowing(&[Tech::Pottery, Tech::Writing]);
        let view = city();
        assert_eq!(choose(&view, &civ(&tech), &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Building(Building::Granary));
        let grown = CityView { built: vec![Building::Granary], ..city() };
        assert_eq!(choose(&grown, &civ(&tech), &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Building(Building::Library));
    }

    #[test]
    fn caravans_go_to_help_a_wonder_elsewhere() {
        let tech = knowing(&[Tech::Pottery, Tech::Writing]);
        let view = CityView { population: 5, ..city() };
        let helping = CivView { caravans: 0, wonder_elsewhere: true, ..civ(&tech) };
        assert_eq!(choose(&view, &helping, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(UnitType::Caravan));
        // A small city does not spare one.
        let small = CityView { population: 2, ..city() };
        assert_ne!(choose(&small, &helping, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(UnitType::Caravan));
    }

    #[test]
    fn war_calls_for_a_bigger_army() {
        let tech = knowing(&[Tech::Pottery]);
        let view = city();
        let at_war = CivView { at_war: true, ..civ(&tech) };
        assert!(matches!(choose(&view, &at_war, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(u) if u.is_military()));
    }

    #[test]
    fn a_coastal_city_builds_a_ferry_for_troops_waiting_in_port() {
        let tech = knowing(&[Tech::Pottery, Tech::Seafaring]);
        let at_war = |coastal: bool| (CityView { coastal, ..city() }, CivView { at_war: true, military: 6, ferry_wanted: 3, ..civ(&tech) });
        let (port, state) = at_war(true);
        assert_eq!(choose(&port, &state, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(UnitType::Galley));
        // A Galley is a ferry, not a warship: none is built while nobody waits for one.
        let idle = CivView { ferry_wanted: 0, ..civ(&tech) };
        let idle = CivView { at_war: true, military: 6, ..idle };
        assert!(!matches!(choose(&port, &idle, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(UnitType::Galley)));
        let (inland, state) = at_war(false);
        assert!(!matches!(choose(&inland, &state, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(u) if u.domain() == Domain::Sea));
    }

    #[test]
    fn a_city_with_nothing_to_build_does_not_pile_up_workers() {
        let tech = knowing(&[Tech::Pottery]);
        let done = CivView { military: army_cap(2), workers: 4, room_to_expand: false, ..civ(&tech) };
        let pick = choose(&CityView { built: Building::ALL.to_vec(), ..city() }, &done, &Supplies::default(), &mut rand::thread_rng());
        assert!(matches!(pick, ProductionItem::Unit(u) if u.is_army()), "{pick:?}");
    }

    #[test]
    fn great_empires_field_armies_beyond_their_garrisons() {
        assert!(army_cap(25) >= 25 + 10);
        assert_eq!(army_cap(2), 8);
    }

    #[test]
    fn workers_come_before_buildings_when_roads_are_needed() {
        let tech = knowing(&[Tech::Pottery]);
        let view = city();
        let short = CivView { workers: 0, ..civ(&tech) };
        assert_eq!(choose(&view, &short, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(UnitType::Worker));
    }
}
