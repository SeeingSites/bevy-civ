use bevy::prelude::*;
use hexx::Hex;
use crate::terrain::{Improvement, Resource};

#[derive(Component)]
pub struct Unit {
    pub unit_type: UnitType,
    pub movement_left: u32,
    pub owner: u32,
    pub position: Hex,
    pub build_turns_left: Option<u32>,
    pub current_build: Option<Improvement>,
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
    // Unique units
    Legion,       // Player / Rome — stronger melee
    WarChariot,   // Egypt — faster, bonus on open terrain
    Immortal,     // Placeholder unique — high defense
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
            UnitType::Legion => (2, 2, 1),       // +1 atk / +1 def
            UnitType::WarChariot => (2, 1, 3),   // high movement
            UnitType::Immortal => (1, 3, 1),     // very high defense
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
            UnitType::Legion => 25,
            UnitType::WarChariot => 30,
            UnitType::Immortal => 25,
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
            UnitType::Legion => Color::srgb(0.9, 0.3, 0.1),
            UnitType::WarChariot => Color::srgb(0.8, 0.6, 0.2),
            UnitType::Immortal => Color::srgb(0.5, 0.2, 0.6),
        }
    }

    pub fn is_worker(&self) -> bool {
        matches!(self, UnitType::Worker)
    }

    pub fn required_resource(&self) -> Option<Resource> {
        match self {
            UnitType::Horseman | UnitType::WarChariot => Some(Resource::Horses),
            UnitType::Spearman | UnitType::Archer | UnitType::Legion => Some(Resource::Iron),
            _ => None,
        }
    }

    /// Unique ability modifiers
    pub fn attack_bonus(&self, terrain: crate::terrain::Terrain) -> f32 {
        match self {
            UnitType::Legion => 1.25, // always stronger
            UnitType::WarChariot if matches!(terrain, crate::terrain::Terrain::Plains | crate::terrain::Terrain::Grassland) => 1.5,
            _ => 1.0,
        }
    }

    pub fn defense_bonus(&self) -> f32 {
        match self {
            UnitType::Immortal => 1.5,
            UnitType::Legion => 1.2,
            _ => 1.0,
        }
    }

    /// Preferred unique unit for a civilization
    pub fn unique_for_civ(civ_id: u32) -> Option<Self> {
        match civ_id {
            0 => Some(UnitType::Legion),      // Player
            1 => Some(UnitType::Legion),      // Rome
            2 => Some(UnitType::WarChariot),  // Egypt
            _ => None,
        }
    }
}

impl Unit {
    pub fn start_build(&mut self, improvement: Improvement) {
        if self.unit_type.is_worker() {
            self.current_build = Some(improvement);
            self.build_turns_left = Some(3);
            self.movement_left = 0;
        }
    }

    pub fn process_build(&mut self) -> Option<Improvement> {
        if let Some(turns) = self.build_turns_left.as_mut() {
            if *turns > 0 {
                *turns -= 1;
            }
            if *turns == 0 {
                let completed = self.current_build.take();
                self.build_turns_left = None;
                return completed;
            }
        }
        None
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
            build_turns_left: None,
            current_build: None,
        },
        Mesh2d(mesh),
        MeshMaterial2d(materials.add(unit_type.color())),
        Transform::from_xyz(pos.x, pos.y, 1.0),
    ));
}
