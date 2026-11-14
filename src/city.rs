use bevy::prelude::*;
use hexx::Hex;

#[derive(Component)]
pub struct City {
    pub name: String,
    pub population: u32,
    pub food_storage: u32,
    pub production_progress: u32,
    pub production_queue: Option<ProductionItem>,
    pub position: Hex,
}

#[derive(Clone, Debug)]
pub enum ProductionItem {
    Unit(crate::unit::UnitType),
    Building(String),
}

impl City {
    pub fn production_per_turn(&self) -> u32 {
        2 + self.population
    }

    pub fn food_per_turn(&self) -> u32 {
        3 + self.population
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
        },
        Mesh2d(mesh),
        MeshMaterial2d(materials.add(Color::srgb(0.9, 0.8, 0.1))),
        Transform::from_xyz(pos.x, pos.y, 0.5),
        Name::new(name),
    ));
}
