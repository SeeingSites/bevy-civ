use bevy::prelude::*;
use hexx::Hex;
use crate::building::CityBuildings;
use crate::diplomacy::Happiness;

#[derive(Component)]
pub struct City {
    pub name: String,
    pub population: u32,
    pub food_storage: u32,
    pub production_progress: u32,
    pub production_queue: Option<ProductionItem>,
    pub position: Hex,
    pub owner: u32,
}

#[derive(Clone, Debug)]
pub enum ProductionItem {
    Unit(crate::unit::UnitType),
    Building(crate::building::Building),
}

impl City {
    pub fn production_per_turn(&self, buildings: &CityBuildings) -> u32 {
        2 + self.population + buildings.total_production_bonus()
    }

    pub fn food_per_turn(&self, buildings: &CityBuildings) -> u32 {
        3 + self.population + buildings.total_food_bonus()
    }

    pub fn science_per_turn(&self, buildings: &CityBuildings) -> u32 {
        1 + self.population / 2 + buildings.total_science_bonus()
    }

    pub fn food_to_grow(&self) -> u32 {
        10 + self.population * 5
    }

    pub fn process_food(&mut self, buildings: &CityBuildings) -> bool {
        let food = self.food_per_turn(buildings);
        self.food_storage += food;
        if self.food_storage >= self.food_to_grow() {
            self.food_storage -= self.food_to_grow();
            self.population += 1;
            true
        } else {
            false
        }
    }

    pub fn process_production(&mut self, buildings: &CityBuildings) -> Option<ProductionItem> {
        let Some(item) = &self.production_queue else { return None };
        let cost = match item {
            ProductionItem::Unit(u) => u.cost(),
            ProductionItem::Building(b) => b.cost(),
        };
        let prod = self.production_per_turn(buildings);
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
        },
        Mesh2d(mesh),
        MeshMaterial2d(materials.add(Color::srgb(0.9, 0.8, 0.1))),
        Transform::from_xyz(pos.x, pos.y, 0.5),
        Name::new(name),
        CityBuildings { buildings: vec![] },
        Happiness::new(population),
    ));
}
