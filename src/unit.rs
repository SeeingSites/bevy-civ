use bevy::prelude::*;
use hexx::Hex;
use crate::terrain::{Improvement, Resource};

#[derive(Component, Clone, serde::Serialize, serde::Deserialize)]
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
    /// An army led by a great general: the other units marching with this one. The army fights
    /// as its best unit with every member's hit points, and moves at its slowest member's pace.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub troops: Vec<UnitType>,
    /// Moves still owed for climbing rough ground: taken from the next turns' allowance.
    #[serde(default)]
    pub fatigue: u32,
    /// The ship this unit rides in (troops in a transport, aircraft on a carrier), and the tile the
    /// ship stood on when the unit was last carried. A unit that moves off that tile by itself has
    /// gone ashore (or taken off). Rebuilt from positions after loading a game.
    #[serde(skip)]
    pub carrier: Option<(Entity, Hex)>,
}

#[derive(Component)]
pub struct Selected;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
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
    // The later defenders (after the great people, to keep the sprite rows in order)
    Pikeman,
    Musketman,
    Infantry,
    MechInfantry,
    ModernArmor,
    // Filling the gaps
    Scout,
    Longbowman,
    Galleon,
    Submarine,
    Engineers,
    RadarArtillery,
    Missionary,
    NuclearMissile,
    /// Carries eight units of troops over the sea.
    Transport,
    /// Carries four aircraft and serves them as a moving airfield.
    Carrier,
}

/// Where a unit can go: over land (crossing the sea aboard ships), on the sea, or through the air.
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
    pub const ALL: [UnitType; 46] = [
        UnitType::Settler, UnitType::Worker, UnitType::Warrior, UnitType::Spearman, UnitType::Archer,
        UnitType::Horseman, UnitType::Legion, UnitType::WarChariot, UnitType::Immortal, UnitType::Phalanx,
        UnitType::Catapult, UnitType::Knight, UnitType::Cannon, UnitType::Rifleman, UnitType::Cavalry,
        UnitType::Tank, UnitType::Artillery, UnitType::Diplomat, UnitType::Caravan, UnitType::Galley,
        UnitType::Frigate, UnitType::Ironclad, UnitType::Destroyer, UnitType::Battleship, UnitType::Fighter,
        UnitType::Bomber, UnitType::GreatGeneral, UnitType::GreatMerchant, UnitType::GreatScientist,
        UnitType::GreatArtist, UnitType::GreatEngineer, UnitType::Pikeman, UnitType::Musketman, UnitType::Infantry,
        UnitType::MechInfantry, UnitType::ModernArmor, UnitType::Scout, UnitType::Longbowman, UnitType::Galleon,
        UnitType::Submarine, UnitType::Engineers, UnitType::RadarArtillery, UnitType::Missionary, UnitType::NuclearMissile,
        UnitType::Transport, UnitType::Carrier,
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

    /// A world wonder that must stand somewhere in the world before anyone can build this unit.
    pub fn required_wonder(&self) -> Option<crate::building::Building> {
        (*self == UnitType::NuclearMissile).then_some(crate::building::Building::Manhattan)
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
            UnitType::Pikeman => "Pikeman",
            UnitType::Musketman => "Musketman",
            UnitType::Infantry => "Infantry",
            UnitType::MechInfantry => "Mech. Infantry",
            UnitType::ModernArmor => "Modern Armor",
            UnitType::Scout => "Scout",
            UnitType::Longbowman => "Longbowman",
            UnitType::Galleon => "Galleon",
            UnitType::Submarine => "Submarine",
            UnitType::Engineers => "Engineers",
            UnitType::RadarArtillery => "Radar Artillery",
            UnitType::Missionary => "Missionary",
            UnitType::NuclearMissile => "Nuclear Missile",
            UnitType::Transport => "Transport",
            UnitType::Carrier => "Carrier",
        }
    }

    /// Technology needed before a city can train this unit.
    pub fn required_tech(&self) -> Option<crate::tech::Tech> {
        use crate::tech::Tech;
        match self {
            UnitType::Settler | UnitType::Worker | UnitType::Warrior | UnitType::Scout => None,
            UnitType::Longbowman => Some(Tech::Monarchy),
            UnitType::Galleon => Some(Tech::Navigation),
            UnitType::Submarine => Some(Tech::Electronics),
            UnitType::Engineers => Some(Tech::SteamPower),
            UnitType::RadarArtillery => Some(Tech::Rocketry),
            UnitType::Missionary => Some(Tech::Mysticism),
            UnitType::NuclearMissile => Some(Tech::Rocketry),
            UnitType::Transport => Some(Tech::Industrialization),
            UnitType::Carrier => Some(Tech::AdvancedFlight),
            UnitType::Spearman | UnitType::Phalanx => Some(Tech::BronzeWorking),
            UnitType::Archer => Some(Tech::Mining),
            UnitType::Horseman => Some(Tech::HorsebackRiding),
            UnitType::Legion | UnitType::Immortal => Some(Tech::IronWorking),
            UnitType::WarChariot => Some(Tech::TheWheel),
            UnitType::Catapult => Some(Tech::Mathematics),
            UnitType::Knight => Some(Tech::Chivalry),
            UnitType::Pikeman => Some(Tech::Engineering),
            UnitType::Cannon | UnitType::Musketman | UnitType::Cavalry => Some(Tech::Gunpowder),
            UnitType::Rifleman => Some(Tech::Industrialization),
            UnitType::Infantry => Some(Tech::Electricity),
            UnitType::MechInfantry => Some(Tech::Computers),
            UnitType::ModernArmor => Some(Tech::Plastics),
            UnitType::Artillery => Some(Tech::ReplaceableParts),
            UnitType::Tank => Some(Tech::MotorizedTransport),
            UnitType::Diplomat => Some(Tech::Writing),
            UnitType::Caravan => Some(Tech::Pottery),
            UnitType::Galley => Some(Tech::Seafaring),
            UnitType::Frigate => Some(Tech::Gunpowder),
            UnitType::Ironclad => Some(Tech::SteamPower),
            UnitType::Destroyer => Some(Tech::Refining),
            UnitType::Battleship => Some(Tech::Steel),
            UnitType::Fighter => Some(Tech::Flight),
            UnitType::Bomber => Some(Tech::AdvancedFlight),
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
            UnitType::Rifleman => (5, 7, 1),
            UnitType::Pikeman => (1, 4, 1),
            UnitType::Musketman => (2, 6, 1),
            UnitType::Infantry => (6, 10, 1),
            UnitType::MechInfantry => (6, 14, 2),
            UnitType::ModernArmor => (20, 12, 3),
            UnitType::Scout => (0, 0, 2),
            UnitType::Longbowman => (4, 1, 1),
            UnitType::Galleon => (2, 2, 5),
            UnitType::Submarine => (10, 2, 5),
            UnitType::Engineers => (0, 0, 2),
            UnitType::RadarArtillery => (16, 2, 2),
            UnitType::Missionary => (0, 0, 2),
            UnitType::NuclearMissile => (0, 0, 0),
            UnitType::Transport => (0, 3, 5),
            UnitType::Carrier => (1, 9, 5),
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
            UnitType::Pikeman => 30,
            UnitType::Musketman => 40,
            UnitType::Infantry => 70,
            UnitType::MechInfantry => 110,
            UnitType::ModernArmor => 150,
            UnitType::Scout => 10,
            UnitType::Longbowman => 30,
            UnitType::Galleon => 40,
            UnitType::Submarine => 100,
            UnitType::Engineers => 30,
            UnitType::RadarArtillery => 120,
            UnitType::Missionary => 30,
            UnitType::NuclearMissile => 160,
            UnitType::Transport => 60,
            UnitType::Carrier => 150,
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
            UnitType::Pikeman => Color::srgb(0.42, 0.48, 0.56),
            UnitType::Musketman => Color::srgb(0.7, 0.15, 0.12),
            UnitType::Infantry | UnitType::MechInfantry => Color::srgb(0.37, 0.42, 0.23),
            UnitType::ModernArmor => Color::srgb(0.72, 0.65, 0.48),
            UnitType::Scout => Color::srgb(0.54, 0.42, 0.23),
            UnitType::Longbowman => Color::srgb(0.18, 0.42, 0.23),
            UnitType::Galleon => Color::srgb(0.42, 0.25, 0.12),
            UnitType::Submarine => Color::srgb(0.37, 0.42, 0.46),
            UnitType::Engineers => Color::srgb(0.85, 0.54, 0.17),
            UnitType::RadarArtillery => Color::srgb(0.42, 0.46, 0.33),
            UnitType::Missionary => Color::srgb(0.94, 0.9, 0.82),
            UnitType::NuclearMissile => Color::srgb(0.9, 0.9, 0.85),
            UnitType::Transport => Color::srgb(0.45, 0.48, 0.5),
            UnitType::Carrier => Color::srgb(0.4, 0.44, 0.48),
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
            UnitType::Galley | UnitType::Galleon | UnitType::Frigate | UnitType::Ironclad | UnitType::Destroyer | UnitType::Battleship | UnitType::Submarine | UnitType::Transport | UnitType::Carrier => Domain::Sea,
            UnitType::Fighter | UnitType::Bomber => Domain::Air,
            _ => Domain::Land,
        }
    }

    /// The odds that a fighter of this kind shoots down a bomber of that kind it intercepts.
    pub fn interception_odds(fighter: UnitType, bomber: UnitType) -> f64 {
        let (attack, defence) = (fighter.stats().0 as f64, bomber.stats().1 as f64);
        attack / (attack + defence).max(1.0)
    }

    /// Wheeled: keeps to open ground unless a road leads through forest or mountains.
    pub fn is_wheeled(&self) -> bool {
        matches!(self, UnitType::WarChariot | UnitType::Catapult | UnitType::Cannon | UnitType::Artillery | UnitType::RadarArtillery)
    }

    /// A military unit that fights on the ground: the only kind that holds or takes cities.
    pub fn is_army(&self) -> bool {
        self.is_military() && self.domain() == Domain::Land
    }

    pub fn is_worker(&self) -> bool {
        matches!(self, UnitType::Worker | UnitType::Engineers)
    }

    /// Turns a worker of this kind takes over a job (Engineers work faster).
    pub fn build_turns(&self) -> u32 {
        if *self == UnitType::Engineers { 2 } else { 3 }
    }

    pub fn is_military(&self) -> bool {
        !matches!(self, UnitType::Settler | UnitType::Worker | UnitType::Engineers | UnitType::Scout | UnitType::Diplomat | UnitType::Caravan | UnitType::Missionary | UnitType::NuclearMissile | UnitType::Transport) && !self.is_great_person()
    }

    /// How many units this ship carries: troops (any land unit) for the transports, aircraft for
    /// the Carrier. Warships of the iron age and after carry none.
    pub fn capacity(&self) -> u32 {
        match self {
            UnitType::Galley | UnitType::Frigate => 2,
            UnitType::Galleon => 4,
            UnitType::Transport => 8,
            UnitType::Carrier => 4,
            _ => 0,
        }
    }

    /// Whether this ship can carry a unit of that kind.
    pub fn carries(&self, cargo: UnitType) -> bool {
        self.capacity() > 0
            && match self {
                UnitType::Carrier => cargo.domain() == Domain::Air,
                _ => cargo.domain() == Domain::Land,
            }
    }

    /// A ship the computer players use to ferry troops and settlers overseas.
    pub fn is_ferry(&self) -> bool {
        matches!(self, UnitType::Galley | UnitType::Galleon | UnitType::Transport)
    }

    /// Strategic resources a city must have access to before it can train this unit.
    pub fn required_resources(&self) -> &'static [Resource] {
        match self {
            UnitType::Horseman | UnitType::WarChariot => &[Resource::Horses],
            UnitType::Legion | UnitType::Immortal | UnitType::Artillery => &[Resource::Iron],
            UnitType::Knight => &[Resource::Horses, Resource::Iron],
            UnitType::Cannon => &[Resource::Iron, Resource::Saltpeter],
            UnitType::Pikeman => &[Resource::Iron],
            UnitType::Musketman | UnitType::Rifleman => &[Resource::Saltpeter],
            UnitType::MechInfantry | UnitType::ModernArmor => &[Resource::Oil],
            UnitType::Cavalry => &[Resource::Horses, Resource::Saltpeter],
            UnitType::Tank | UnitType::Bomber => &[Resource::Oil, Resource::Rubber],
            UnitType::Frigate => &[Resource::Iron, Resource::Saltpeter],
            UnitType::Ironclad => &[Resource::Coal, Resource::Iron],
            UnitType::Destroyer | UnitType::Fighter | UnitType::Submarine | UnitType::Carrier => &[Resource::Oil],
            UnitType::Battleship => &[Resource::Oil, Resource::Iron],
            _ => &[],
        }
    }

    pub fn attack_bonus(&self, terrain: crate::terrain::Terrain) -> f32 {
        match self {
            UnitType::Legion => 1.25,
            UnitType::WarChariot if matches!(terrain, crate::terrain::Terrain::Plains | crate::terrain::Terrain::Grassland) => 1.5,
            UnitType::Catapult | UnitType::Cannon | UnitType::Artillery | UnitType::RadarArtillery => 1.3,
            _ => 1.0,
        }
    }

    pub fn defense_bonus(&self) -> f32 {
        match self {
            UnitType::Immortal | UnitType::Phalanx => 1.5,
            UnitType::Legion => 1.2,
            UnitType::Tank => 1.3,
            UnitType::Pikeman => 1.25,
            _ => 1.0,
        }
    }

    /// What this unit can be upgraded to once that is buildable (in a city, for gold).
    pub fn upgrades_to(&self) -> Option<UnitType> {
        use UnitType::*;
        Some(match self {
            Warrior => Spearman,
            Worker => Engineers,
            Archer => Longbowman,
            Longbowman => Musketman,
            Artillery => RadarArtillery,
            Galley => Galleon,
            Galleon => Transport,
            Spearman => Phalanx,
            Phalanx => Pikeman,
            Pikeman | Legion | Immortal => Musketman,
            Musketman => Rifleman,
            Rifleman => Infantry,
            Infantry => MechInfantry,
            Tank => ModernArmor,
            Horseman | WarChariot => Knight,
            Knight => Cavalry,
            Cavalry => Tank,
            Catapult => Cannon,
            Cannon => Artillery,
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
        matches!(self, UnitType::Catapult | UnitType::Cannon | UnitType::Artillery | UnitType::RadarArtillery | UnitType::Frigate | UnitType::Ironclad | UnitType::Battleship | UnitType::Bomber)
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
            self.build_turns_left = Some(self.unit_type.build_turns());
            self.end_move();
        }
    }

    /// Hit points when unhurt: 3, 4 for a veteran, 5 for elite; an army has that for each member.
    pub fn max_hp(&self) -> u32 {
        (3 + self.level as u32) * (1 + self.troops.len() as u32)
    }

    pub fn is_army_group(&self) -> bool {
        !self.troops.is_empty()
    }

    /// The most an army's slowest member can move in a turn (no limit for a single unit).
    pub fn troop_moves(&self) -> u32 {
        self.troops.iter().map(|t| t.stats().2).min().unwrap_or(u32::MAX)
    }

    /// "Rifleman" or "Army (Rifleman, Musketman, Pikeman)".
    pub fn display_name(&self) -> String {
        if self.troops.is_empty() {
            self.unit_type.name().to_string()
        } else {
            let names: Vec<&str> = std::iter::once(self.unit_type).chain(self.troops.iter().copied()).map(|t| t.name()).collect();
            format!("Army ({})", names.join(", "))
        }
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
        crate::pathfinding::Budget { moves: self.movement_left, road_credit: self.road_credit, debt: 0 }
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
        self.fatigue += budget.debt;
    }


    /// Gives up the rest of this turn's movement.
    pub fn end_move(&mut self) {
        self.movement_left = 0;
        self.road_credit = 0;
    }

    /// Starts a turn with a full allowance, less any moves still owed for rough ground.
    pub fn refresh(&mut self, moves: u32) {
        let paid = moves.min(self.fatigue);
        self.fatigue -= paid;
        self.movement_left = moves - paid;
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
            troops: Vec::new(),
            fatigue: 0,
            carrier: None,
        })
        .id()
}

/// Most units in one army (the leader and its troops).
pub const ARMY_SIZE: usize = 3;

/// Units that can join an army: ground troops that are not already in one.
pub fn can_join_army(unit: &Unit) -> bool {
    unit.unit_type.is_army() && unit.troops.is_empty()
}

/// Forms an army from units standing together: the strongest leads, up to two more march with it.
/// Returns the leader's index in `units` and the indices of the units absorbed (to remove), or
/// None if fewer than two units can join.
pub fn plan_army(units: &[&Unit]) -> Option<(usize, Vec<usize>)> {
    let mut eligible: Vec<usize> = (0..units.len()).filter(|i| can_join_army(units[*i])).collect();
    if eligible.len() < 2 {
        return None;
    }
    let power = |u: &Unit| {
        let (a, d, _) = u.unit_type.stats();
        (a + d) * 10 + u.level as u32
    };
    eligible.sort_by_key(|i| std::cmp::Reverse(power(units[*i])));
    eligible.truncate(ARMY_SIZE);
    Some((eligible[0], eligible[1..].to_vec()))
}

/// Makes `leader` an army with `members` marching in it (their wounds come along).
pub fn join_army(leader: &mut Unit, members: &[Unit]) {
    let hp = leader.hp + members.iter().map(|m| m.hp).sum::<u32>();
    leader.troops.extend(members.iter().map(|m| m.unit_type));
    leader.hp = hp.min(leader.max_hp());
    leader.fortified = false;
    leader.end_move();
}

#[cfg(test)]
mod army_tests {
    use super::*;

    fn unit(kind: UnitType, hp: u32, level: u8) -> Unit {
        Unit { unit_type: kind, movement_left: 1, owner: 0, position: Hex::ZERO, build_turns_left: None, current_build: None, fortified: false, road_credit: 0, hp, level, troops: Vec::new(), fatigue: 0, carrier: None }
    }

    #[test]
    fn the_strongest_leads_and_the_army_pools_hit_points() {
        let units = [unit(UnitType::Spearman, 3, 0), unit(UnitType::Musketman, 2, 1), unit(UnitType::Worker, 3, 0), unit(UnitType::Knight, 3, 0), unit(UnitType::Warrior, 3, 0)];
        let refs: Vec<&Unit> = units.iter().collect();
        let (leader, absorbed) = plan_army(&refs).unwrap();
        assert_eq!(units[leader].unit_type, UnitType::Musketman);
        assert_eq!(absorbed.len(), 2);
        assert!(!absorbed.contains(&2), "workers do not join");
        let mut army = units[leader].clone();
        let members: Vec<Unit> = absorbed.iter().map(|i| units[*i].clone()).collect();
        join_army(&mut army, &members);
        assert_eq!(army.max_hp(), 12);
        assert_eq!(army.hp, 8);
        assert_eq!(army.troop_moves(), 1);
        assert!(army.display_name().starts_with("Army (Musketman"));
        // An army does not join another.
        assert!(plan_army(&[&army, &units[0]]).is_none());
    }
}
