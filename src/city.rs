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
    Entertainer,   // +2 luxury
    TaxCollector,  // +3 gold
    Scientist,     // +3 science
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

    /// Gold to buy the rest of the current project: two gold a shield plus a premium that grows with
    /// what is missing, and double if nothing has been built of it yet.
    pub fn hurry_cost(&self) -> Option<i32> {
        if !self.can_hurry() {
            return None;
        }
        let left = self.shields_left() as i32;
        let gold = 2 * left + left * left / 20;
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

    pub fn production_per_turn(&self, buildings: &CityBuildings, gov: Government, dist_to_capital: u32) -> u32 {
        let base = self.land.1 + buildings.total_production_bonus();
        let corruption = self.corruption(buildings, gov, dist_to_capital);
        ((base as f32) * gov.production_modifier() * (1.0 - corruption)).max(0.0) as u32
    }

    /// Food left over each turn once everyone has eaten (two each): the worked land and the
    /// buildings feed the city, and specialists eat without farming. Can be negative.
    pub fn food_balance(&self, buildings: &CityBuildings) -> i32 {
        (self.land.0 + buildings.total_food_bonus()) as i32 - (self.population * crate::tiles::FOOD_PER_CITIZEN) as i32
    }

    /// The surplus shown in the city panel (never below zero; see `food_balance`).
    pub fn food_per_turn(&self, buildings: &CityBuildings, _gov: Government, _dist_to_capital: u32) -> u32 {
        self.food_balance(buildings).max(0) as u32
    }

    /// Corruption, halved by a courthouse.
    pub fn corruption(&self, buildings: &CityBuildings, gov: Government, dist_to_capital: u32) -> f32 {
        let corruption = self.corruption_penalty(gov, dist_to_capital);
        if buildings.has(crate::building::Building::Courthouse) { corruption / 2.0 } else { corruption }
    }

    pub fn science_per_turn(&self, buildings: &CityBuildings, gov: Government, dist_to_capital: u32) -> u32 {
        let base = 1 + self.population / 2 + buildings.total_science_bonus();
        let specialist_sci = self.specialists.iter().filter(|s| matches!(s, Specialist::Scientist)).count() as u32 * 3;
        let corruption = self.corruption(buildings, gov, dist_to_capital);
        // Round up so a small city under a weak government still contributes something.
        (((base + specialist_sci) as f32) * gov.science_modifier() * (1.0 - corruption)).max(0.0).ceil() as u32
    }

    pub fn gold_per_turn(&self, buildings: &CityBuildings) -> u32 {
        let base = buildings.total_gold_bonus() + self.land.2;
        let specialist_gold = self.specialists.iter().filter(|s| matches!(s, Specialist::TaxCollector)).count() as u32 * 3;
        base + specialist_gold
    }

    pub fn luxury_from_specialists(&self) -> u32 {
        self.specialists.iter().filter(|s| matches!(s, Specialist::Entertainer)).count() as u32 * 2
    }

    pub fn corruption_penalty(&self, gov: Government, dist_to_capital: u32) -> f32 {
        if self.is_capital { return 0.0; }
        (gov.corruption_factor() * dist_to_capital as f32).min(0.5)
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
        self.population += 1;
        Growth::Grew
    }

    pub fn process_production(&mut self, buildings: &CityBuildings, gov: Government, dist: u32) -> Option<ProductionItem> {
        let Some(item) = &self.production_queue else {
            // No project: the shields are kept for the next one.
            self.production_progress += self.production_per_turn(buildings, gov, dist);
            return None;
        };
        let cost = match item {
            ProductionItem::Unit(u) => u.cost(),
            ProductionItem::Building(b) => b.cost(),
        };
        let prod = self.production_per_turn(buildings, gov, dist);
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
        }
    }

    #[test]
    fn buying_costs_more_for_more_and_double_from_scratch() {
        let temple = city(ProductionItem::Building(Building::Temple), 10, 5);
        assert_eq!(temple.hurry_cost(), Some(2 * 20 + 20 * 20 / 20));
        let fresh = city(ProductionItem::Building(Building::Temple), 0, 5);
        assert_eq!(fresh.hurry_cost(), Some(2 * (2 * 30 + 30 * 30 / 20)));
        assert_eq!(city(ProductionItem::Building(Building::Pyramids), 10, 5).hurry_cost(), None, "no buying wonders");
        let units = city(ProductionItem::Unit(UnitType::Legion), 5, 5);
        assert!(units.hurry_cost().is_some());
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
