use bevy::prelude::*;
use hexx::Hex;
use crate::building::CityBuildings;
use crate::diplomacy::Happiness;
use crate::government::Government;

#[derive(Component, Clone, serde::Serialize, serde::Deserialize)]
pub struct City {
    pub name: String,
    pub population: u32,
    pub food_storage: u32,
    pub production_progress: u32,
    pub production_queue: Option<ProductionItem>,
    pub position: Hex,
    pub owner: u32,
    pub specialists: Vec<Specialist>,
    pub is_capital: bool,
    /// Turns its people still resist a conqueror: the city produces nothing until they settle.
    #[serde(default)]
    pub resistance: u32,
    /// Projects to take up, in order, after the current one.
    #[serde(default)]
    pub queue: Vec<ProductionItem>,
    /// The tiles its working citizens work (its own tile is always worked and not listed).
    #[serde(default)]
    pub worked: Vec<hexx::Hex>,
    /// What its own tile and the worked tiles give: food, shields, gold.
    #[serde(default)]
    pub land: (u32, u32, u32),
    /// What its governor works the land for (Auto: the governor decides).
    #[serde(default)]
    pub focus: crate::tiles::Focus,
    /// Tiles the player put a citizen on by hand: the governor leaves them worked.
    #[serde(default)]
    pub pinned: Vec<hexx::Hex>,
    /// Level of its drug trade (see `troubles`).
    #[serde(default)]
    pub narcotics: u32,
    /// Turns its people stay angry after an adulterated-food scandal.
    #[serde(default)]
    pub scandal: u32,
    /// The governor chooses its next project whenever one is finished (and its queue is empty).
    #[serde(default)]
    pub auto_build: bool,
}

/// Most projects a city can have waiting.
pub const QUEUE_LIMIT: usize = 6;

/// Turns a captured city resists its new owner: longer for a big city (a garrison of two or more
/// land units shortens it).
pub fn resistance_turns(population: u32) -> u32 {
    (1 + population / 3).min(6)
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Specialist {
    /// Calms two citizens.
    Entertainer,
    /// +2 gold, multiplied by the city's market and bank.
    TaxCollector,
    /// +3 science, multiplied by the city's library and university.
    Scientist,
}

/// What a turn of eating did to a city.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Growth {
    Steady,
    Grew,
    /// The granary is full but the city needs an Aqueduct (or Sewer System) to grow.
    Capped,
    Starved,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ProductionItem {
    Unit(crate::unit::UnitType),
    Building(crate::building::Building),
}

impl ProductionItem {
    pub fn name(&self) -> &'static str {
        match self {
            ProductionItem::Unit(u) => u.name(),
            ProductionItem::Building(b) => b.name(),
        }
    }

    pub fn cost(&self) -> u32 {
        match self {
            ProductionItem::Unit(u) => u.cost(),
            ProductionItem::Building(b) => b.cost(),
        }
    }

    pub fn required_resources(&self) -> &'static [crate::terrain::Resource] {
        match self {
            ProductionItem::Unit(u) => u.required_resources(),
            ProductionItem::Building(b) => b.required_resources(),
        }
    }
}

impl City {
    /// Shields still needed to finish the current project.
    pub fn shields_left(&self) -> u32 {
        self.production_queue.as_ref().map(|i| i.cost().saturating_sub(self.production_progress)).unwrap_or(0)
    }

    /// Whether the current project can be hurried at all (wonders cannot: caravans and great engineers help those).
    pub fn can_hurry(&self) -> bool {
        match &self.production_queue {
            Some(ProductionItem::Building(b)) => !b.is_wonder() && self.shields_left() > 0,
            Some(ProductionItem::Unit(_)) => self.shields_left() > 0,
            None => false,
        }
    }

    /// Gold to buy the rest of the current project: two gold a missing shield for a building; for a
    /// unit, a premium on top that grows with what is missing. Double if nothing has been built of
    /// it yet.
    pub fn hurry_cost(&self) -> Option<i32> {
        if !self.can_hurry() {
            return None;
        }
        let left = self.shields_left() as i32;
        let gold = match self.production_queue {
            Some(ProductionItem::Unit(_)) => 2 * left + left * left / 20,
            _ => 2 * left,
        };
        Some(if self.production_progress == 0 { gold * 2 } else { gold })
    }

    /// Citizens it takes to rush the current project under a government that allows it (one for
    /// every 20 shields missing). The city must keep at least one.
    pub fn rush_citizens(&self, gov: Government) -> Option<u32> {
        if !self.can_hurry() || !gov.allows_forced_labour() {
            return None;
        }
        let citizens = self.shields_left().div_ceil(20);
        (citizens < self.population).then_some(citizens)
    }

    /// Citizens working the land (specialists do not).
    pub fn workers(&self) -> u32 {
        self.population.saturating_sub(self.specialists.len() as u32)
    }

    /// Shields the city's land and buildings give, less the share `waste` lost (see
    /// `economy::waste`). Waste always leaves a city at least one shield.
    pub fn production_per_turn(&self, buildings: &CityBuildings, waste: f32) -> u32 {
        let base = self.land.1 + buildings.total_production_bonus();
        let lost = ((base as f32) * waste.clamp(0.0, 1.0)).round() as u32;
        base.saturating_sub(lost).max(base.min(1))
    }

    /// Food left over each turn once everyone has eaten (two each): the worked land and the
    /// buildings feed the city, and specialists eat without farming. Can be negative.
    pub fn food_balance(&self, buildings: &CityBuildings) -> i32 {
        (self.land.0 + buildings.total_food_bonus()) as i32 - (self.population * crate::tiles::FOOD_PER_CITIZEN) as i32
    }

    /// The surplus shown in the city panel (never below zero; see `food_balance`).
    pub fn food_per_turn(&self, buildings: &CityBuildings) -> u32 {
        self.food_balance(buildings).max(0) as u32
    }

    /// Citizens its entertainers calm (two each).
    pub fn luxury_from_specialists(&self) -> u32 {
        self.specialists.iter().filter(|s| matches!(s, Specialist::Entertainer)).count() as u32 * 2
    }

    /// Its scientists and tax collectors.
    pub fn scientists(&self) -> u32 {
        self.specialists.iter().filter(|s| matches!(s, Specialist::Scientist)).count() as u32
    }

    pub fn tax_collectors(&self) -> u32 {
        self.specialists.iter().filter(|s| matches!(s, Specialist::TaxCollector)).count() as u32
    }

    pub fn food_to_grow(&self) -> u32 {
        10 + self.population * 5
    }

    /// A turn of eating: the city grows when its granary fills (up to the size its water supply
    /// allows) and starves when it runs dry.
    pub fn process_food(&mut self, buildings: &CityBuildings, fresh_water: bool) -> Growth {
        let balance = self.food_balance(buildings);
        if balance < 0 {
            let need = balance.unsigned_abs();
            if self.food_storage >= need {
                self.food_storage -= need;
                return Growth::Steady;
            }
            self.food_storage = 0;
            if self.population > 1 {
                self.population -= 1;
                // The hungry stop specialising first.
                if self.specialists.len() as u32 > self.population {
                    self.specialists.pop();
                }
                return Growth::Starved;
            }
            return Growth::Steady;
        }
        self.food_storage += balance as u32;
        if self.food_storage < self.food_to_grow() {
            return Growth::Steady;
        }
        if self.population >= crate::building::Building::size_limit(&buildings.buildings, fresh_water) {
            self.food_storage = self.food_to_grow();
            return Growth::Capped;
        }
        self.food_storage -= self.food_to_grow();
        // A Granary keeps half of the food the city had gathered.
        if buildings.has(crate::building::Building::Granary) {
            self.food_storage += self.food_to_grow() / 2;
        }
        self.population += 1;
        self.food_storage = self.food_storage.min(self.food_to_grow().saturating_sub(1));
        Growth::Grew
    }

    pub fn process_production(&mut self, buildings: &CityBuildings, waste: f32) -> Option<ProductionItem> {
        let Some(item) = &self.production_queue else {
            // No project: the shields are kept for the next one.
            self.production_progress += self.production_per_turn(buildings, waste);
            return None;
        };
        let cost = match item {
            ProductionItem::Unit(u) => u.cost(),
            ProductionItem::Building(b) => b.cost(),
        };
        let prod = self.production_per_turn(buildings, waste);
        self.production_progress += prod;
        if self.production_progress >= cost {
            self.production_progress -= cost;
            // Units keep being trained until the order changes; after a building the city waits for orders.
            let completed = self.production_queue.clone();
            if matches!(completed, Some(ProductionItem::Building(_))) {
                self.production_queue = None;
            }
            completed
        } else {
            None
        }
    }

    pub fn add_specialist(&mut self, specialist: Specialist) {
        if self.specialists.len() < self.population as usize {
            self.specialists.push(specialist);
        }
    }
}

/// Spawns the logical city; the client attaches sprites to anything with a `City`.
pub fn spawn_city(
    commands: &mut Commands,
    coord: Hex,
    name: String,
    population: u32,
    owner: u32,
    is_capital: bool,
) -> Entity {
    commands
        .spawn((
            City {
                name: name.clone(),
                population,
                food_storage: 0,
                production_progress: 0,
                production_queue: Some(ProductionItem::Unit(crate::unit::UnitType::Warrior)),
                position: coord,
                owner,
                specialists: vec![],
                is_capital,
                resistance: 0,
                queue: Vec::new(),
                worked: Vec::new(),
                land: (0, 0, 0),
                focus: Default::default(),
                pinned: Vec::new(),
                narcotics: 0,
                scandal: 0,
                auto_build: true,
            },
            Name::new(name),
            CityBuildings { buildings: vec![] },
            Happiness::new(population),
        ))
        .id()
}

#[cfg(test)]
mod hurry_tests {
    use super::*;
    use crate::building::Building;
    use crate::unit::UnitType;

    fn city(item: ProductionItem, progress: u32, population: u32) -> City {
        City {
            name: "Roma".into(),
            population,
            food_storage: 0,
            production_progress: progress,
            production_queue: Some(item),
            position: Hex::ZERO,
            owner: 0,
            specialists: vec![],
            is_capital: true,
            resistance: 0,
            queue: Vec::new(),
            worked: Vec::new(),
            land: (0, 0, 0),
            focus: Default::default(),
            pinned: Vec::new(),
                narcotics: 0,
                scandal: 0,
                auto_build: true,
        }
    }

    #[test]
    fn buying_costs_more_for_more_and_double_from_scratch() {
        // Buildings: two gold a missing shield.
        let temple = city(ProductionItem::Building(Building::Temple), 10, 5);
        assert_eq!(temple.hurry_cost(), Some(2 * 20));
        let fresh = city(ProductionItem::Building(Building::Temple), 0, 5);
        assert_eq!(fresh.hurry_cost(), Some(2 * (2 * 30)));
        assert_eq!(city(ProductionItem::Building(Building::Pyramids), 10, 5).hurry_cost(), None, "no buying wonders");
        // Units: a premium that grows with what is missing.
        let legion = city(ProductionItem::Unit(UnitType::Legion), 5, 5);
        let left = (UnitType::Legion.cost() - 5) as i32;
        assert_eq!(legion.hurry_cost(), Some(2 * left + left * left / 20));
    }

    #[test]
    fn waste_leaves_at_least_a_shield() {
        let mut town = city(ProductionItem::Building(Building::Temple), 0, 3);
        town.land = (6, 10, 2);
        let none = CityBuildings { buildings: vec![] };
        assert_eq!(town.production_per_turn(&none, 0.0), 10);
        assert_eq!(town.production_per_turn(&none, 0.3), 7);
        assert_eq!(town.production_per_turn(&none, 1.0), 1);
        town.land = (6, 0, 2);
        assert_eq!(town.production_per_turn(&none, 0.5), 0);
    }

    #[test]
    fn a_granary_keeps_half_the_food_and_water_limits_growth() {
        let full = |granary: bool, population: u32| {
            let mut town = city(ProductionItem::Building(Building::Temple), 0, population);
            town.food_storage = town.food_to_grow() - 1;
            town.land = (population * 2 + 3, 1, 0);
            let built = CityBuildings { buildings: if granary { vec![Building::Granary] } else { vec![] } };
            let growth = town.process_food(&built, false);
            (growth, town)
        };
        let (growth, plain) = full(false, 3);
        assert_eq!(growth, Growth::Grew);
        assert_eq!(plain.food_storage, 2);
        let (_, stored) = full(true, 3);
        assert_eq!(stored.food_storage, 2 + (10 + 3 * 5) / 2);
        // Size 6 is the limit without an aqueduct or fresh water.
        assert_eq!(full(false, 6).0, Growth::Capped);
        let mut riverside = city(ProductionItem::Building(Building::Temple), 0, 6);
        riverside.food_storage = riverside.food_to_grow();
        riverside.land = (20, 1, 0);
        assert_eq!(riverside.process_food(&CityBuildings { buildings: vec![] }, true), Growth::Grew);
        assert_eq!(Building::size_limit(&[Building::Aqueduct], false), 12);
        assert_eq!(Building::size_limit(&[], true), 12);
    }

    #[test]
    fn rushing_takes_citizens_under_hard_governments_only() {
        let barracks = city(ProductionItem::Building(Building::Barracks), 0, 4);
        assert_eq!(barracks.rush_citizens(Government::Despotism), Some(2));
        assert_eq!(barracks.rush_citizens(Government::Democracy), None);
        let small = city(ProductionItem::Building(Building::Barracks), 0, 2);
        assert_eq!(small.rush_citizens(Government::Despotism), None, "a city keeps one citizen");
    }
}

#[cfg(test)]
mod resistance_tests {
    #[test]
    fn bigger_cities_resist_longer_up_to_a_limit() {
        assert_eq!(super::resistance_turns(1), 1);
        assert_eq!(super::resistance_turns(9), 4);
        assert_eq!(super::resistance_turns(40), 6);
    }
}
