use bevy::prelude::*;
use hexx::Hex;

#[derive(Component)]
pub struct Unit {
    pub unit_type: UnitType,
    pub movement_left: u32,
    pub owner: u32,
    pub position: Hex,
}

#[derive(Component)]
pub struct Selected;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UnitType {
    Settler,
    Worker,
    Warrior,
    Spearman,
    Archer,
    Horseman,
}

impl UnitType {
    pub fn stats(&self) -> (u32, u32, u32) {
        match self {
            UnitType::Settler => (0, 0, 1),
            UnitType::Worker => (0, 0, 1),
            UnitType::Warrior => (1, 1, 1),
            UnitType::Spearman => (1, 2, 1),
            UnitType::Archer => (2, 1, 1),
            UnitType::Horseman => (2, 1, 2),
        }
    }

    pub fn cost(&self) -> u32 {
        match self {
            UnitType::Settler => 30,
            UnitType::Worker => 10,
            UnitType::Warrior => 10,
            UnitType::Spearman => 20,
            UnitType::Archer => 20,
            UnitType::Horseman => 20,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            UnitType::Settler => Color::srgb(0.9, 0.9, 0.2),
            UnitType::Worker => Color::srgb(0.6, 0.4, 0.2),
            UnitType::Warrior => Color::srgb(0.8, 0.2, 0.2),
            UnitType::Spearman => Color::srgb(0.3, 0.3, 0.8),
            UnitType::Archer => Color::srgb(0.2, 0.7, 0.2),
            UnitType::Horseman => Color::srgb(0.7, 0.4, 0.1),
        }
    }
}

pub fn spawn_unit(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layout: &hexx::HexLayout,
    coord: Hex,
    unit_type: UnitType,
) {
    let (_, _, mv) = unit_type.stats();
    let pos = layout.hex_to_world_pos(coord);
    let mesh = meshes.add(Circle::new(8.0));

    commands.spawn((
        Unit {
            unit_type,
            movement_left: mv,
            owner: 0,
            position: coord,
        },
        Mesh2d(mesh),
        MeshMaterial2d(materials.add(unit_type.color())),
        Transform::from_xyz(pos.x, pos.y, 1.0),
    ));
}
