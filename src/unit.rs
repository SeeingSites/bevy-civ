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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum UnitType {
    Settler,
    Worker,
    Warrior,
    Spearman,
    Archer,
    Horseman,
    Legion,
    WarChariot,
    Immortal,
    // Classical / Medieval
    Phalanx,
    Catapult,
    Knight,
    // Renaissance / Industrial
    Cannon,
    Rifleman,
    Cavalry,
    // Modern
    Tank,
    Artillery,
    // Special
    Diplomat,
    Caravan,
}

impl UnitType {
    /// Every unit type, in declaration order (also the row order of the sprite atlas).
    pub const ALL: [UnitType; 19] = [
        UnitType::Settler, UnitType::Worker, UnitType::Warrior, UnitType::Spearman, UnitType::Archer,
        UnitType::Horseman, UnitType::Legion, UnitType::WarChariot, UnitType::Immortal, UnitType::Phalanx,
        UnitType::Catapult, UnitType::Knight, UnitType::Cannon, UnitType::Rifleman, UnitType::Cavalry,
        UnitType::Tank, UnitType::Artillery, UnitType::Diplomat, UnitType::Caravan,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            UnitType::WarChariot => "War Chariot",
            UnitType::Settler => "Settler",
            UnitType::Worker => "Worker",
            UnitType::Warrior => "Warrior",
            UnitType::Spearman => "Spearman",
            UnitType::Archer => "Archer",
            UnitType::Horseman => "Horseman",
            UnitType::Legion => "Legion",
            UnitType::Immortal => "Immortal",
            UnitType::Phalanx => "Phalanx",
            UnitType::Catapult => "Catapult",
            UnitType::Knight => "Knight",
            UnitType::Cannon => "Cannon",
            UnitType::Rifleman => "Rifleman",
            UnitType::Cavalry => "Cavalry",
            UnitType::Tank => "Tank",
            UnitType::Artillery => "Artillery",
            UnitType::Diplomat => "Diplomat",
            UnitType::Caravan => "Caravan",
        }
    }

    /// Technology needed before a city can train this unit.
    pub fn required_tech(&self) -> Option<crate::tech::Tech> {
        use crate::tech::Tech;
        match self {
            UnitType::Settler | UnitType::Worker | UnitType::Warrior => None,
            UnitType::Spearman | UnitType::Phalanx => Some(Tech::BronzeWorking),
            UnitType::Archer => Some(Tech::Mining),
            UnitType::Horseman => Some(Tech::AnimalHusbandry),
            UnitType::Legion | UnitType::Immortal => Some(Tech::IronWorking),
            UnitType::WarChariot => Some(Tech::TheWheel),
            UnitType::Catapult => Some(Tech::Mathematics),
            UnitType::Knight => Some(Tech::HorsebackRiding),
            UnitType::Cannon | UnitType::Rifleman | UnitType::Cavalry => Some(Tech::Gunpowder),
            UnitType::Tank | UnitType::Artillery => Some(Tech::Democracy),
            UnitType::Diplomat => Some(Tech::Writing),
            UnitType::Caravan => Some(Tech::Pottery),
        }
    }

    pub fn stats(&self) -> (u32, u32, u32) {
        match self {
            UnitType::Settler => (0, 0, 1),
            UnitType::Worker => (0, 0, 1),
            UnitType::Warrior => (1, 1, 1),
            UnitType::Spearman => (1, 2, 1),
            UnitType::Archer => (2, 1, 1),
            UnitType::Horseman => (2, 1, 2),
            UnitType::Legion => (2, 2, 1),
            UnitType::WarChariot => (2, 1, 3),
            UnitType::Immortal => (1, 3, 1),
            UnitType::Phalanx => (1, 3, 1),
            UnitType::Catapult => (4, 1, 1),
            UnitType::Knight => (4, 2, 2),
            UnitType::Cannon => (6, 1, 1),
            UnitType::Rifleman => (5, 4, 1),
            UnitType::Cavalry => (6, 2, 2),
            UnitType::Tank => (10, 5, 2),
            UnitType::Artillery => (8, 2, 1),
            UnitType::Diplomat => (0, 0, 2),
            UnitType::Caravan => (0, 0, 1),
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
            UnitType::Phalanx => 30,
            UnitType::Catapult => 40,
            UnitType::Knight => 50,
            UnitType::Cannon => 60,
            UnitType::Rifleman => 50,
            UnitType::Cavalry => 60,
            UnitType::Tank => 100,
            UnitType::Artillery => 80,
            UnitType::Diplomat => 30,
            UnitType::Caravan => 40,
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
            UnitType::Phalanx => Color::srgb(0.4, 0.4, 0.6),
            UnitType::Catapult => Color::srgb(0.5, 0.3, 0.1),
            UnitType::Knight => Color::srgb(0.6, 0.2, 0.5),
            UnitType::Cannon => Color::srgb(0.3, 0.3, 0.3),
            UnitType::Rifleman => Color::srgb(0.2, 0.5, 0.2),
            UnitType::Cavalry => Color::srgb(0.7, 0.5, 0.2),
            UnitType::Tank => Color::srgb(0.4, 0.4, 0.2),
            UnitType::Artillery => Color::srgb(0.5, 0.4, 0.3),
            UnitType::Diplomat => Color::srgb(0.8, 0.8, 0.9),
            UnitType::Caravan => Color::srgb(0.9, 0.7, 0.4),
        }
    }

    pub fn is_worker(&self) -> bool {
        matches!(self, UnitType::Worker)
    }

    pub fn is_military(&self) -> bool {
        !matches!(self, UnitType::Settler | UnitType::Worker | UnitType::Diplomat | UnitType::Caravan)
    }

    pub fn required_resource(&self) -> Option<Resource> {
        match self {
            UnitType::Horseman | UnitType::WarChariot | UnitType::Knight | UnitType::Cavalry => Some(Resource::Horses),
            UnitType::Spearman | UnitType::Archer | UnitType::Legion | UnitType::Phalanx | UnitType::Rifleman => Some(Resource::Iron),
            _ => None,
        }
    }

    pub fn attack_bonus(&self, terrain: crate::terrain::Terrain) -> f32 {
        match self {
            UnitType::Legion => 1.25,
            UnitType::WarChariot if matches!(terrain, crate::terrain::Terrain::Plains | crate::terrain::Terrain::Grassland) => 1.5,
            UnitType::Catapult | UnitType::Cannon | UnitType::Artillery => 1.3,
            _ => 1.0,
        }
    }

    pub fn defense_bonus(&self) -> f32 {
        match self {
            UnitType::Immortal | UnitType::Phalanx => 1.5,
            UnitType::Legion => 1.2,
            UnitType::Rifleman | UnitType::Tank => 1.3,
            _ => 1.0,
        }
    }

    pub fn unique_for_civ(civ_id: u32) -> Option<Self> {
        match civ_id {
            0 | 1 => Some(UnitType::Legion),
            2 => Some(UnitType::WarChariot),
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

/// Spawns the logical unit; the client attaches sprites to anything with a `Unit`.
pub fn spawn_unit(commands: &mut Commands, coord: Hex, unit_type: UnitType, owner: u32) -> Entity {
    let (_, _, mv) = unit_type.stats();
    commands
        .spawn(Unit {
            unit_type,
            movement_left: mv,
            owner,
            position: coord,
            build_turns_left: None,
            current_build: None,
        })
        .id()
}
