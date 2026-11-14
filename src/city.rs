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

#[derive(Clone, Debug)]
pub enum ProductionItem {
    Unit(crate::unit::UnitType),
    Building(crate::building::Building),
}

impl City {
    pub fn production_per_turn(&self, buildings: &CityBuildings, gov: Government, dist_to_capital: u32) -> u32 {
        let base = 2 + self.population + buildings.total_production_bonus();
        let corruption = self.corruption_penalty(gov, dist_to_capital);
        ((base as f32) * gov.production_modifier() * (1.0 - corruption)).max(0.0) as u32
    }

    pub fn food_per_turn(&self, buildings: &CityBuildings, gov: Government, dist_to_capital: u32) -> u32 {
        let base = 3 + self.population + buildings.total_food_bonus();
        let corruption = self.corruption_penalty(gov, dist_to_capital);
        ((base as f32) * (1.0 - corruption)).max(0.0) as u32
    }

    pub fn science_per_turn(&self, buildings: &CityBuildings, gov: Government, dist_to_capital: u32) -> u32 {
        let base = 1 + self.population / 2 + buildings.total_science_bonus();
        let specialist_sci = self.specialists.iter().filter(|s| matches!(s, Specialist::Scientist)).count() as u32 * 3;
        let corruption = self.corruption_penalty(gov, dist_to_capital);
        (((base + specialist_sci) as f32) * gov.science_modifier() * (1.0 - corruption)).max(0.0) as u32
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

    pub fn process_food(&mut self, buildings: &CityBuildings, gov: Government, dist: u32) -> bool {
        let food = self.food_per_turn(buildings, gov, dist);
        self.food_storage += food;
        if self.food_storage >= self.food_to_grow() {
            self.food_storage -= self.food_to_grow();
            self.population += 1;
            true
        } else {
            false
        }
    }

    pub fn process_production(&mut self, buildings: &CityBuildings, gov: Government, dist: u32) -> Option<ProductionItem> {
        let Some(item) = &self.production_queue else { return None };
        let cost = match item {
            ProductionItem::Unit(u) => u.cost(),
            ProductionItem::Building(b) => b.cost(),
        };
        let prod = self.production_per_turn(buildings, gov, dist);
        self.production_progress += prod;
        if self.production_progress >= cost {
            self.production_progress -= cost;
            let completed = self.production_queue.take();
            self.production_queue = Some(ProductionItem::Unit(crate::unit::UnitType::Warrior));
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

pub fn spawn_city(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layout: &hexx::HexLayout,
    coord: Hex,
    name: String,
    population: u32,
) {
    let pos = layout.hex_to_world_pos(coord);
    let mesh = meshes.add(Circle::new(12.0));

    commands.spawn((
        City {
            name: name.clone(),
            population,
            food_storage: 0,
            production_progress: 0,
            production_queue: Some(ProductionItem::Unit(crate::unit::UnitType::Warrior)),
            position: coord,
            owner: 0,
            specialists: vec![],
            is_capital: name == "Capital",
        },
        Mesh2d(mesh),
        MeshMaterial2d(materials.add(Color::srgb(0.9, 0.8, 0.1))),
        Transform::from_xyz(pos.x, pos.y, 0.5),
        Name::new(name),
        CityBuildings { buildings: vec![] },
        Happiness::new(population),
    ));
}
