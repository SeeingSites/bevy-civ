//! City management for computer players: what each of their cities should build next.

use hexx::Hex;
use rand::Rng;

use bevy_civ::building::Building;
use bevy_civ::city::ProductionItem;
use bevy_civ::tech::TechState;
use bevy_civ::unit::UnitType;

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
}

/// The state of the civilization as a whole, including what its other cities have already queued.
pub struct CivView<'a> {
    pub tech: &'a TechState,
    pub aggression: f32,
    pub cities: usize,
    pub workers: usize,
    pub settlers: usize,
    pub military: usize,
    /// Sources and cities still waiting for a road.
    pub road_work: usize,
    pub at_war: bool,
}

/// Largest army a computer player keeps.
pub fn army_cap(cities: usize) -> usize {
    (4 + 2 * cities).min(20)
}

fn can_train(unit: UnitType, city: &CityView, civ: &CivView, supplies: &Supplies) -> bool {
    unit.required_tech().is_none_or(|t| civ.tech.researched.contains(&t)) && supplies.has(city.position, unit.required_resources())
}

pub fn can_build(building: &Building, city: &CityView, civ: &CivView, supplies: &Supplies) -> bool {
    !city.built.contains(building)
        && building.required_tech().is_none_or(|t| civ.tech.researched.contains(&t))
        && supplies.has(city.position, building.required_resources())
        && (*building != Building::Harbor || city.coastal)
}

/// The unit that holds a city best.
pub fn best_defender(city: &CityView, civ: &CivView, supplies: &Supplies) -> UnitType {
    UnitType::ALL
        .into_iter()
        .filter(|u| u.is_military() && can_train(*u, city, civ, supplies))
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
        .filter(|u| u.is_military() && can_train(*u, city, civ, supplies))
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
    let wanted_workers = (civ.cities.div_ceil(2) + usize::from(civ.road_work > 3 * civ.workers.max(1))).min(civ.cities.max(1));
    if civ.workers < wanted_workers {
        return unit(UnitType::Worker);
    }
    if !city.linked && can_build(&Building::Harbor, city, civ, supplies) {
        return ProductionItem::Building(Building::Harbor);
    }
    let cap = army_cap(civ.cities);
    let wanted_army = if civ.at_war { 2 * civ.cities + 2 } else { civ.cities + 1 }.min(cap);
    if civ.military < wanted_army {
        let offensive = civ.at_war || civ.aggression > 0.6;
        return unit(if offensive { best_attacker(city, civ, supplies) } else { best_defender(city, civ, supplies) });
    }
    if civ.settlers == 0 && civ.cities < 6 && city.population >= 3 && rng.gen_bool(0.35) {
        return unit(UnitType::Settler);
    }

    let wish = [
        (Building::Walls, city.threatened || civ.at_war),
        (Building::Granary, city.population < 8),
        (Building::Barracks, civ.at_war || civ.aggression > 0.6),
        (Building::Library, true),
        (Building::Marketplace, true),
        (Building::Harbor, true),
        (Building::Aqueduct, city.population >= 6),
        (Building::University, true),
        (Building::Bank, true),
        (Building::Factory, true),
    ];
    if let Some((building, _)) = wish.into_iter().find(|(b, wanted)| *wanted && can_build(b, city, civ, supplies)) {
        return ProductionItem::Building(building);
    }
    if city.population >= 6 {
        if let Some(wonder) = Building::ALL.into_iter().find(|b| b.is_wonder() && can_build(b, city, civ, supplies)) {
            return ProductionItem::Building(wonder);
        }
    }
    if civ.military < cap {
        return unit(best_attacker(city, civ, supplies));
    }
    unit(UnitType::Worker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_civ::tech::Tech;
    use bevy_civ::terrain::Resource as Good;
    use std::collections::HashSet;

    fn city() -> CityView {
        CityView { position: Hex::ZERO, population: 4, built: vec![], coastal: false, linked: true, defended: true, threatened: false }
    }

    fn civ(tech: &TechState) -> CivView<'_> {
        CivView { tech, aggression: 0.4, cities: 2, workers: 1, settlers: 1, military: 3, road_work: 0, at_war: false }
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
    fn a_cut_off_coastal_city_builds_a_harbour() {
        let tech = knowing(&[Tech::Seafaring, Tech::Pottery]);
        let view = CityView { coastal: true, linked: false, ..city() };
        assert_eq!(choose(&view, &civ(&tech), &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Building(Building::Harbor));
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
    fn war_calls_for_a_bigger_army() {
        let tech = knowing(&[Tech::Pottery]);
        let view = city();
        let at_war = CivView { at_war: true, ..civ(&tech) };
        assert!(matches!(choose(&view, &at_war, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(u) if u.is_military()));
    }

    #[test]
    fn workers_come_before_buildings_when_roads_are_needed() {
        let tech = knowing(&[Tech::Pottery]);
        let view = city();
        let short = CivView { workers: 0, ..civ(&tech) };
        assert_eq!(choose(&view, &short, &Supplies::default(), &mut rand::thread_rng()), ProductionItem::Unit(UnitType::Worker));
    }
}
