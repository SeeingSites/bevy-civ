use bevy::prelude::*;
use hexx::Hex;
use crate::building::CityBuildings;
use crate::diplomacy::Happiness;
use crate::government::Government;

#[derive(Component)]
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
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
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
    /// Citizens working the land (specialists do not).
    pub fn workers(&self) -> u32 {
        self.population.saturating_sub(self.specialists.len() as u32)
    }

    pub fn production_per_turn(&self, buildings: &CityBuildings, gov: Government, dist_to_capital: u32) -> u32 {
        let base = 2 + self.workers() + buildings.total_production_bonus();
        let corruption = self.corruption(buildings, gov, dist_to_capital);
        ((base as f32) * gov.production_modifier() * (1.0 - corruption)).max(0.0) as u32
    }

    /// Food left over each turn once everyone has eaten: the land feeds its workers with a little to
    /// spare, buildings add more, and every specialist is a mouth that does not farm. Can be negative.
    pub fn food_balance(&self, buildings: &CityBuildings) -> i32 {
        2 + buildings.total_food_bonus() as i32 + self.workers() as i32 / 4 - self.specialists.len() as i32
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
        let base = buildings.total_gold_bonus();
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
    pub fn process_food(&mut self, buildings: &CityBuildings, _gov: Government, _dist: u32) -> Growth {
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
        if self.population >= crate::building::Building::size_limit(&buildings.buildings) {
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
            },
            Name::new(name),
            CityBuildings { buildings: vec![] },
            Happiness::new(population),
        ))
        .id()
}
