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
    /// Dug in: skipped when cycling through units, and harder to dislodge.
    pub fortified: bool,
    /// Thirds of a move left over from travelling on roads this turn.
    pub road_credit: u32,
    /// Hit points left (see `max_hp`).
    pub hp: u32,
    /// 0 regular, 1 veteran, 2 elite.
    pub level: u8,
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
    // Naval
    Galley,
    Frigate,
    Ironclad,
    Destroyer,
    Battleship,
    // Air
    Fighter,
    Bomber,
    // Great people: born in cities from great-person points, never trained
    GreatGeneral,
    GreatMerchant,
    GreatScientist,
    GreatArtist,
    GreatEngineer,
}

/// Where a unit can go: over land (boarding a boat to cross water), on the sea, or through the air.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Domain {
    Land,
    Sea,
    Air,
}

/// How far an aircraft may operate from the nearest city of its owner.
pub const AIR_RANGE: i32 = 8;

impl UnitType {
    /// Every unit type, in declaration order (also the row order of the sprite atlas).
    pub const ALL: [UnitType; 31] = [
        UnitType::Settler, UnitType::Worker, UnitType::Warrior, UnitType::Spearman, UnitType::Archer,
        UnitType::Horseman, UnitType::Legion, UnitType::WarChariot, UnitType::Immortal, UnitType::Phalanx,
        UnitType::Catapult, UnitType::Knight, UnitType::Cannon, UnitType::Rifleman, UnitType::Cavalry,
        UnitType::Tank, UnitType::Artillery, UnitType::Diplomat, UnitType::Caravan, UnitType::Galley,
        UnitType::Frigate, UnitType::Ironclad, UnitType::Destroyer, UnitType::Battleship, UnitType::Fighter,
        UnitType::Bomber, UnitType::GreatGeneral, UnitType::GreatMerchant, UnitType::GreatScientist,
        UnitType::GreatArtist, UnitType::GreatEngineer,
    ];

    /// The five kinds of great person.
    pub const GREAT_PEOPLE: [UnitType; 5] =
        [UnitType::GreatGeneral, UnitType::GreatMerchant, UnitType::GreatScientist, UnitType::GreatArtist, UnitType::GreatEngineer];

    /// Born in a city from great-person points rather than trained.
    pub fn is_great_person(&self) -> bool {
        Self::GREAT_PEOPLE.contains(self)
    }

    /// Whether a city can put this unit into production.
    pub fn is_trainable(&self) -> bool {
        !self.is_great_person()
    }

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
            UnitType::Galley => "Galley",
            UnitType::Frigate => "Frigate",
            UnitType::Ironclad => "Ironclad",
            UnitType::Destroyer => "Destroyer",
            UnitType::Battleship => "Battleship",
            UnitType::Fighter => "Fighter",
            UnitType::Bomber => "Bomber",
            UnitType::GreatGeneral => "Great General",
            UnitType::GreatMerchant => "Great Merchant",
            UnitType::GreatScientist => "Great Scientist",
            UnitType::GreatArtist => "Great Artist",
            UnitType::GreatEngineer => "Great Engineer",
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
            UnitType::Artillery => Some(Tech::ReplaceableParts),
            UnitType::Tank => Some(Tech::MotorizedTransport),
            UnitType::Diplomat => Some(Tech::Writing),
            UnitType::Caravan => Some(Tech::Pottery),
            UnitType::Galley => Some(Tech::Seafaring),
            UnitType::Frigate => Some(Tech::Gunpowder),
            UnitType::Ironclad => Some(Tech::SteamPower),
            UnitType::Destroyer => Some(Tech::Refining),
            UnitType::Battleship | UnitType::Fighter | UnitType::Bomber => Some(Tech::MotorizedTransport),
            UnitType::GreatGeneral | UnitType::GreatMerchant | UnitType::GreatScientist | UnitType::GreatArtist | UnitType::GreatEngineer => None,
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
            UnitType::Galley => (1, 1, 3),
            UnitType::Frigate => (3, 2, 4),
            UnitType::Ironclad => (5, 5, 4),
            UnitType::Destroyer => (8, 6, 6),
            UnitType::Battleship => (12, 10, 5),
            UnitType::Fighter => (6, 5, 8),
            UnitType::Bomber => (12, 2, 8),
            UnitType::GreatGeneral | UnitType::GreatMerchant | UnitType::GreatScientist | UnitType::GreatArtist | UnitType::GreatEngineer => (0, 0, 2),
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
            UnitType::Galley => 30,
            UnitType::Frigate => 50,
            UnitType::Ironclad => 80,
            UnitType::Destroyer => 110,
            UnitType::Battleship => 160,
            UnitType::Fighter => 90,
            UnitType::Bomber => 110,
            // Never trained; this is only their value as a prize.
            UnitType::GreatGeneral | UnitType::GreatMerchant | UnitType::GreatScientist | UnitType::GreatArtist | UnitType::GreatEngineer => 200,
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
            UnitType::Galley | UnitType::Frigate => Color::srgb(0.6, 0.45, 0.3),
            UnitType::Ironclad | UnitType::Destroyer | UnitType::Battleship => Color::srgb(0.5, 0.52, 0.56),
            UnitType::Fighter | UnitType::Bomber => Color::srgb(0.55, 0.6, 0.5),
            UnitType::GreatGeneral | UnitType::GreatMerchant | UnitType::GreatScientist | UnitType::GreatArtist | UnitType::GreatEngineer => Color::srgb(0.95, 0.82, 0.35),
        }
    }

    pub fn domain(&self) -> Domain {
        match self {
            UnitType::Galley | UnitType::Frigate | UnitType::Ironclad | UnitType::Destroyer | UnitType::Battleship => Domain::Sea,
            UnitType::Fighter | UnitType::Bomber => Domain::Air,
            _ => Domain::Land,
        }
    }

    /// A military unit that fights on the ground: the only kind that holds or takes cities.
    pub fn is_army(&self) -> bool {
        self.is_military() && self.domain() == Domain::Land
    }

    pub fn is_worker(&self) -> bool {
        matches!(self, UnitType::Worker)
    }

    pub fn is_military(&self) -> bool {
        !matches!(self, UnitType::Settler | UnitType::Worker | UnitType::Diplomat | UnitType::Caravan) && !self.is_great_person()
    }

    /// Strategic resources a city must have access to before it can train this unit.
    pub fn required_resources(&self) -> &'static [Resource] {
        match self {
            UnitType::Horseman | UnitType::WarChariot => &[Resource::Horses],
            UnitType::Legion | UnitType::Immortal | UnitType::Artillery => &[Resource::Iron],
            UnitType::Knight => &[Resource::Horses, Resource::Iron],
            UnitType::Cannon => &[Resource::Iron, Resource::Saltpeter],
            UnitType::Rifleman => &[Resource::Saltpeter],
            UnitType::Cavalry => &[Resource::Horses, Resource::Saltpeter],
            UnitType::Tank | UnitType::Bomber => &[Resource::Oil, Resource::Rubber],
            UnitType::Frigate => &[Resource::Iron, Resource::Saltpeter],
            UnitType::Ironclad => &[Resource::Coal, Resource::Iron],
            UnitType::Destroyer | UnitType::Fighter => &[Resource::Oil],
            UnitType::Battleship => &[Resource::Oil, Resource::Iron],
            _ => &[],
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

    /// What this unit can be upgraded to once that is buildable (in a city, for gold).
    pub fn upgrades_to(&self) -> Option<UnitType> {
        use UnitType::*;
        Some(match self {
            Warrior => Spearman,
            Spearman => Phalanx,
            Phalanx | Archer | Legion | Immortal => Rifleman,
            Horseman | WarChariot => Knight,
            Knight => Cavalry,
            Cavalry => Tank,
            Catapult => Cannon,
            Cannon => Artillery,
            Galley => Frigate,
            Frigate => Ironclad,
            Ironclad => Destroyer,
            Destroyer => Battleship,
            _ => return None,
        })
    }

    /// Gold to upgrade to `to`.
    pub fn upgrade_cost(&self, to: UnitType) -> i32 {
        (to.cost() as i32 - self.cost() as i32).max(10) * 2 + 20
    }

    /// Artillery that can bombard a neighbouring tile without risk.
    pub fn can_bombard(&self) -> bool {
        matches!(self, UnitType::Catapult | UnitType::Cannon | UnitType::Artillery | UnitType::Frigate | UnitType::Ironclad | UnitType::Battleship | UnitType::Bomber)
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
            self.end_move();
        }
    }

    /// Hit points when unhurt: 3, 4 for a veteran, 5 for elite.
    pub fn max_hp(&self) -> u32 {
        3 + self.level as u32
    }

    pub fn level_name(&self) -> &'static str {
        match self.level {
            0 => "Regular",
            1 => "Veteran",
            _ => "Elite",
        }
    }

    /// Promotion after a victory (one level, up to elite), which also adds a hit point.
    pub fn promote(&mut self) -> bool {
        if self.level >= 2 {
            return false;
        }
        self.level += 1;
        self.hp += 1;
        true
    }

    /// What it can still move this turn.
    pub fn budget(&self) -> crate::pathfinding::Budget {
        crate::pathfinding::Budget { moves: self.movement_left, road_credit: self.road_credit }
    }

    /// Whether it can take another step this turn (part-moves left on a road count).
    pub fn can_move(&self) -> bool {
        self.budget().can_step()
    }

    /// Steps onto a neighbouring tile, paying for it.
    pub fn step(&mut self, to: Hex, cost: crate::pathfinding::StepCost) {
        self.spend(cost);
        self.position = to;
    }

    /// Pays for an action that takes movement without moving (an attack).
    pub fn spend(&mut self, cost: crate::pathfinding::StepCost) {
        let mut budget = self.budget();
        budget.spend(cost);
        self.movement_left = budget.moves;
        self.road_credit = budget.road_credit;
    }

    /// Gives up the rest of this turn's movement.
    pub fn end_move(&mut self) {
        self.movement_left = 0;
        self.road_credit = 0;
    }

    /// Starts a turn with a full allowance.
    pub fn refresh(&mut self, moves: u32) {
        self.movement_left = moves;
        self.road_credit = 0;
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
            fortified: false,
            road_credit: 0,
            hp: 3,
            level: 0,
        })
        .id()
}
