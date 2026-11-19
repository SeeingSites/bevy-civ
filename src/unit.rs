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
    /// Cartel gunmen: modern raiders bred by a city's drug trade (see `barbarians`).
    Cartel,
    /// A terror cell: modern raiders bred by grievance (see `barbarians`).
    Insurgent,
    // Unique units: each people's own version of an ordinary unit (see `unique_base` and
    // `traits::of`), with its own strength and an ability.
    Praetorian,
    Hoplite,
    Companion,
    Numidian,
    RoyalChariot,
    ImmortalGuard,
    Cataphract,
    GallicSwordsman,
    WarElephant,
    ChuKoNu,
    Samurai,
    Keshik,
    JaguarWarrior,
    Chasqui,
    Mandekalu,
    AskiaRider,
    Conquistador,
    ManOWar,
    Musketeer,
    Sipahi,
    Cossack,
    // The Modern Nations expansion's peoples.
    Marine,
    Panzer,
    GuardsRifleman,
    Bandeirante,
    LightHorse,
    Mountie,
    Charro,
    Commando,
    // The Age of Revolution and World Wars expansions' peoples.
    BlackHussar,
    Grenzer,
    Bersaglieri,
}

/// What sets a unique unit apart (see [`UnitType::ability`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ability {
    /// +50% attack against units on open ground (grassland, plains, desert) outside cities.
    Charge,
    /// +50% attack against a city.
    Storm,
    /// Defends at double strength against mounted attackers.
    ShieldWall,
    /// +50% defence in a city.
    Bastion,
    /// Slips past enemy zones of control.
    Outrider,
    /// Each victory brings back plunder: gold for its treasury.
    Plunder,
    /// Needs no strategic resource to be trained.
    SelfSufficient,
}

impl Ability {
    pub fn name(&self) -> &'static str {
        match self {
            Ability::Charge => "Charge",
            Ability::Storm => "Storm",
            Ability::ShieldWall => "Shield wall",
            Ability::Bastion => "Bastion",
            Ability::Outrider => "Outrider",
            Ability::Plunder => "Plunder",
            Ability::SelfSufficient => "Self-sufficient",
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            Ability::Charge => "+50% attack against units on open ground",
            Ability::Storm => "+50% attack against cities",
            Ability::ShieldWall => "double defence against mounted attackers",
            Ability::Bastion => "+50% defence in a city",
            Ability::Outrider => "ignores enemy zones of control",
            Ability::Plunder => "each victory brings back gold",
            Ability::SelfSufficient => "needs no Horses or Iron",
        }
    }
}

/// Gold a unit with [`Ability::Plunder`] brings back from each victory.
pub const PLUNDER_GOLD: i32 = 15;

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
    pub const ALL: [UnitType; 80] = [
        UnitType::Settler, UnitType::Worker, UnitType::Warrior, UnitType::Spearman, UnitType::Archer,
        UnitType::Horseman, UnitType::Legion, UnitType::WarChariot, UnitType::Immortal, UnitType::Phalanx,
        UnitType::Catapult, UnitType::Knight, UnitType::Cannon, UnitType::Rifleman, UnitType::Cavalry,
        UnitType::Tank, UnitType::Artillery, UnitType::Diplomat, UnitType::Caravan, UnitType::Galley,
        UnitType::Frigate, UnitType::Ironclad, UnitType::Destroyer, UnitType::Battleship, UnitType::Fighter,
        UnitType::Bomber, UnitType::GreatGeneral, UnitType::GreatMerchant, UnitType::GreatScientist,
        UnitType::GreatArtist, UnitType::GreatEngineer, UnitType::Pikeman, UnitType::Musketman, UnitType::Infantry,
        UnitType::MechInfantry, UnitType::ModernArmor, UnitType::Scout, UnitType::Longbowman, UnitType::Galleon,
        UnitType::Submarine, UnitType::Engineers, UnitType::RadarArtillery, UnitType::Missionary, UnitType::NuclearMissile,
        UnitType::Transport, UnitType::Carrier, UnitType::Cartel, UnitType::Insurgent,
        UnitType::Praetorian, UnitType::Hoplite, UnitType::Companion, UnitType::Numidian, UnitType::RoyalChariot, UnitType::ImmortalGuard, UnitType::Cataphract, UnitType::GallicSwordsman, UnitType::WarElephant, UnitType::ChuKoNu, UnitType::Samurai, UnitType::Keshik, UnitType::JaguarWarrior, UnitType::Chasqui, UnitType::Mandekalu, UnitType::AskiaRider, UnitType::Conquistador, UnitType::ManOWar, UnitType::Musketeer, UnitType::Sipahi, UnitType::Cossack,
        UnitType::Marine, UnitType::Panzer, UnitType::GuardsRifleman, UnitType::Bandeirante, UnitType::LightHorse, UnitType::Mountie, UnitType::Charro, UnitType::Commando,
        UnitType::BlackHussar, UnitType::Grenzer, UnitType::Bersaglieri,
    ];

    /// The five kinds of great person.
    pub const GREAT_PEOPLE: [UnitType; 5] =
        [UnitType::GreatGeneral, UnitType::GreatMerchant, UnitType::GreatScientist, UnitType::GreatArtist, UnitType::GreatEngineer];

    /// Born in a city from great-person points rather than trained.
    /// The ordinary unit this people's unique unit stands in for (`None` for ordinary units). A
    /// unique unit costs, needs and upgrades like its base unit; it differs in strength and ability.
    pub fn unique_base(&self) -> Option<UnitType> {
        use UnitType::*;
        Some(match self {
            Praetorian => Legion,
            Hoplite => Phalanx,
            Companion => Horseman,
            Numidian => Horseman,
            RoyalChariot => WarChariot,
            ImmortalGuard => Immortal,
            Cataphract => Knight,
            GallicSwordsman => Legion,
            WarElephant => Knight,
            ChuKoNu => Longbowman,
            Samurai => Knight,
            Keshik => Knight,
            JaguarWarrior => Warrior,
            Chasqui => Spearman,
            Mandekalu => Knight,
            AskiaRider => Horseman,
            Conquistador => Knight,
            ManOWar => Frigate,
            Musketeer => Musketman,
            Sipahi => Cavalry,
            Cossack => Cavalry,
            Marine => Infantry,
            Panzer => Tank,
            GuardsRifleman => Rifleman,
            Bandeirante => Musketman,
            LightHorse => Cavalry,
            Mountie => Cavalry,
            Charro => Cavalry,
            Commando => Infantry,
            BlackHussar => Cavalry,
            Grenzer => Musketman,
            Bersaglieri => Infantry,
            _ => return None,
        })
    }

    /// The special ability of a unique unit.
    pub fn ability(&self) -> Option<Ability> {
        use UnitType::*;
        Some(match self {
            Praetorian => Ability::Storm,
            Hoplite => Ability::ShieldWall,
            Companion => Ability::Charge,
            Numidian => Ability::Plunder,
            RoyalChariot => Ability::Charge,
            ImmortalGuard => Ability::Storm,
            Cataphract => Ability::Charge,
            GallicSwordsman => Ability::Plunder,
            WarElephant => Ability::SelfSufficient,
            ChuKoNu => Ability::Bastion,
            Samurai => Ability::Storm,
            Keshik => Ability::Outrider,
            JaguarWarrior => Ability::Plunder,
            Chasqui => Ability::Outrider,
            Mandekalu => Ability::Charge,
            AskiaRider => Ability::Charge,
            Conquistador => Ability::Plunder,
            ManOWar => Ability::Plunder,
            Musketeer => Ability::Bastion,
            Sipahi => Ability::Charge,
            Cossack => Ability::Outrider,
            Marine => Ability::Storm,
            Panzer => Ability::Charge,
            GuardsRifleman => Ability::Bastion,
            Bandeirante => Ability::Outrider,
            LightHorse => Ability::Charge,
            Mountie => Ability::Outrider,
            Charro => Ability::Plunder,
            Commando => Ability::Outrider,
            BlackHussar => Ability::Charge,
            Grenzer => Ability::Outrider,
            Bersaglieri => Ability::Outrider,
            _ => return None,
        })
    }

    pub fn is_great_person(&self) -> bool {
        Self::GREAT_PEOPLE.contains(self)
    }

    /// Whether a city can put this unit into production.
    pub fn is_trainable(&self) -> bool {
        // A unique unit is never ordered by name: a city of its people that trains the ordinary unit
        // turns out the unique one instead.
        !self.is_great_person() && !self.is_raider() && self.unique_base().is_none()
    }

    /// Modern raiders: cartel gunmen and insurgents, who rise inside a people's own lands and never
    /// hold a city (they extort it, or strike at it, and are spent).
    pub fn is_raider(&self) -> bool {
        matches!(self, UnitType::Cartel | UnitType::Insurgent)
    }

    /// A world wonder that must stand somewhere in the world before anyone can build this unit.
    pub fn required_wonder(&self) -> Option<crate::building::Building> {
        if let Some(base) = self.unique_base() {
            return base.required_wonder();
        }
        (*self == UnitType::NuclearMissile).then_some(crate::building::Building::Manhattan)
    }

    pub fn name(&self) -> &'static str {
        crate::theme::rename(crate::theme::Kind::Unit, self, self.base_name())
    }

    /// The name in the default (Earth) theme.
    pub fn base_name(&self) -> &'static str {
        match self {
            UnitType::Praetorian => "Praetorian",
            UnitType::Hoplite => "Hoplite",
            UnitType::Companion => "Companion Cavalry",
            UnitType::Numidian => "Numidian Rider",
            UnitType::RoyalChariot => "Royal Chariot",
            UnitType::ImmortalGuard => "Immortal Guard",
            UnitType::Cataphract => "Cataphract",
            UnitType::GallicSwordsman => "Gallic Swordsman",
            UnitType::WarElephant => "War Elephant",
            UnitType::ChuKoNu => "Chu-ko-nu",
            UnitType::Samurai => "Samurai",
            UnitType::Keshik => "Keshik",
            UnitType::JaguarWarrior => "Jaguar Warrior",
            UnitType::Chasqui => "Chasqui Runner",
            UnitType::Mandekalu => "Mandekalu Cavalry",
            UnitType::AskiaRider => "Askia Rider",
            UnitType::Conquistador => "Conquistador",
            UnitType::ManOWar => "Man-O-War",
            UnitType::Musketeer => "Musketeer",
            UnitType::Sipahi => "Sipahi",
            UnitType::Cossack => "Cossack",
            UnitType::Marine => "Marine",
            UnitType::Panzer => "Panzer",
            UnitType::GuardsRifleman => "Guards Rifleman",
            UnitType::Bandeirante => "Bandeirante",
            UnitType::LightHorse => "Light Horse",
            UnitType::Mountie => "Mountie",
            UnitType::Charro => "Charro",
            UnitType::Commando => "Commando",
            UnitType::BlackHussar => "Black Hussar",
            UnitType::Grenzer => "Grenzer",
            UnitType::Bersaglieri => "Bersaglieri",
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
            UnitType::Cartel => "Cartel Gunmen",
            UnitType::Insurgent => "Insurgents",
        }
    }

    /// Technology needed before a city can train this unit.
    pub fn required_tech(&self) -> Option<crate::tech::Tech> {
        if let Some(base) = self.unique_base() {
            return base.required_tech();
        }
        use crate::tech::Tech;
        match self {
            UnitType::JaguarWarrior => Some(Tech::BronzeWorking),
            UnitType::Praetorian | UnitType::Hoplite | UnitType::Companion | UnitType::Numidian | UnitType::RoyalChariot | UnitType::ImmortalGuard | UnitType::Cataphract | UnitType::GallicSwordsman | UnitType::WarElephant | UnitType::ChuKoNu | UnitType::Samurai | UnitType::Keshik | UnitType::Chasqui | UnitType::Mandekalu | UnitType::AskiaRider | UnitType::Conquistador | UnitType::ManOWar | UnitType::Musketeer | UnitType::Sipahi | UnitType::Cossack | UnitType::Marine | UnitType::Panzer | UnitType::GuardsRifleman | UnitType::Bandeirante | UnitType::LightHorse | UnitType::Mountie | UnitType::Charro | UnitType::Commando | UnitType::BlackHussar | UnitType::Grenzer | UnitType::Bersaglieri => self.unique_base().and_then(|b| b.required_tech()),
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
            UnitType::Cartel | UnitType::Insurgent => None,
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

    /// Attack, defence and moves, as Civilization III rates them (the units this game has no
    /// counterpart for keep values in proportion).
    pub fn stats(&self) -> (u32, u32, u32) {
        match self {
            UnitType::Praetorian => (4, 2, 1),
            UnitType::Hoplite => (1, 4, 1),
            UnitType::Companion => (3, 1, 2),
            UnitType::Numidian => (2, 2, 3),
            UnitType::RoyalChariot => (3, 1, 2),
            UnitType::ImmortalGuard => (4, 3, 1),
            UnitType::Cataphract => (5, 3, 2),
            UnitType::GallicSwordsman => (3, 2, 2),
            UnitType::WarElephant => (4, 4, 1),
            UnitType::ChuKoNu => (4, 2, 1),
            UnitType::Samurai => (4, 4, 2),
            UnitType::Keshik => (4, 3, 3),
            UnitType::JaguarWarrior => (2, 1, 2),
            UnitType::Chasqui => (1, 2, 2),
            UnitType::Mandekalu => (4, 4, 2),
            UnitType::AskiaRider => (3, 1, 2),
            UnitType::Conquistador => (4, 3, 2),
            UnitType::ManOWar => (4, 3, 4),
            UnitType::Musketeer => (2, 5, 1),
            UnitType::Sipahi => (8, 3, 3),
            UnitType::Cossack => (6, 3, 4),
            UnitType::Marine => (7, 10, 1),
            UnitType::Panzer => (17, 8, 2),
            UnitType::GuardsRifleman => (4, 7, 1),
            UnitType::Bandeirante => (3, 4, 2),
            UnitType::LightHorse => (7, 3, 3),
            UnitType::Mountie => (7, 4, 3),
            UnitType::Charro => (7, 3, 3),
            UnitType::Commando => (8, 9, 2),
            UnitType::BlackHussar => (8, 2, 3),
            UnitType::Grenzer => (3, 5, 1),
            UnitType::Bersaglieri => (6, 10, 2),
            UnitType::Settler => (0, 0, 1),
            UnitType::Worker => (0, 0, 1),
            UnitType::Warrior => (1, 1, 1),
            UnitType::Spearman => (1, 2, 1),
            UnitType::Archer => (2, 1, 1),
            UnitType::Horseman => (2, 1, 2),
            // The swordsmen of the iron age.
            UnitType::Legion => (3, 2, 1),
            UnitType::WarChariot => (2, 1, 2),
            UnitType::Immortal => (4, 2, 1),
            UnitType::Phalanx => (1, 3, 1),
            // Guns only bombard (see `bombard`).
            UnitType::Catapult => (0, 1, 1),
            UnitType::Knight => (4, 3, 2),
            UnitType::Cannon => (0, 1, 1),
            UnitType::Rifleman => (4, 6, 1),
            // Twice as strong against mounted attackers (see `anti_mounted`).
            UnitType::Pikeman => (1, 2, 1),
            UnitType::Musketman => (2, 4, 1),
            UnitType::Infantry => (6, 10, 1),
            UnitType::MechInfantry => (12, 18, 2),
            UnitType::ModernArmor => (24, 16, 3),
            UnitType::Scout => (0, 0, 2),
            UnitType::Longbowman => (4, 1, 1),
            UnitType::Galleon => (2, 2, 5),
            UnitType::Submarine => (10, 2, 5),
            UnitType::Engineers => (0, 0, 2),
            UnitType::RadarArtillery => (0, 2, 2),
            UnitType::Missionary => (0, 0, 2),
            UnitType::NuclearMissile => (0, 0, 0),
            UnitType::Transport => (0, 3, 5),
            UnitType::Carrier => (1, 9, 5),
            UnitType::Cartel => (5, 4, 2),
            UnitType::Insurgent => (6, 3, 1),
            UnitType::Cavalry => (6, 3, 3),
            UnitType::Tank => (16, 8, 2),
            UnitType::Artillery => (0, 2, 1),
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
        if let Some(base) = self.unique_base() {
            return base.cost();
        }
        match self {
            UnitType::JaguarWarrior => 15,
            UnitType::Praetorian | UnitType::Hoplite | UnitType::Companion | UnitType::Numidian | UnitType::RoyalChariot | UnitType::ImmortalGuard | UnitType::Cataphract | UnitType::GallicSwordsman | UnitType::WarElephant | UnitType::ChuKoNu | UnitType::Samurai | UnitType::Keshik | UnitType::Chasqui | UnitType::Mandekalu | UnitType::AskiaRider | UnitType::Conquistador | UnitType::ManOWar | UnitType::Musketeer | UnitType::Sipahi | UnitType::Cossack | UnitType::Marine | UnitType::Panzer | UnitType::GuardsRifleman | UnitType::Bandeirante | UnitType::LightHorse | UnitType::Mountie | UnitType::Charro | UnitType::Commando | UnitType::BlackHussar | UnitType::Grenzer | UnitType::Bersaglieri => self.unique_base().map_or(0, |b| b.cost()),
            UnitType::Settler => 30,
            UnitType::Worker => 10,
            UnitType::Warrior => 10,
            UnitType::Spearman => 20,
            UnitType::Archer => 20,
            UnitType::Horseman => 20,
            UnitType::Legion => 30,
            UnitType::WarChariot => 20,
            UnitType::Immortal => 40,
            UnitType::Phalanx => 30,
            UnitType::Catapult => 20,
            UnitType::Knight => 50,
            UnitType::Cannon => 40,
            UnitType::Rifleman => 50,
            UnitType::Pikeman => 30,
            UnitType::Musketman => 30,
            UnitType::Infantry => 70,
            UnitType::MechInfantry => 110,
            UnitType::ModernArmor => 150,
            UnitType::Scout => 10,
            UnitType::Longbowman => 30,
            UnitType::Galleon => 40,
            UnitType::Submarine => 100,
            UnitType::Engineers => 30,
            UnitType::RadarArtillery => 100,
            UnitType::Missionary => 30,
            UnitType::NuclearMissile => 160,
            UnitType::Transport => 60,
            UnitType::Carrier => 150,
            UnitType::Cartel | UnitType::Insurgent => 0,
            UnitType::Cavalry => 70,
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
        if let Some(base) = self.unique_base() {
            return base.color();
        }
        match self {
            UnitType::Praetorian | UnitType::Hoplite | UnitType::Companion | UnitType::Numidian | UnitType::RoyalChariot | UnitType::ImmortalGuard | UnitType::Cataphract | UnitType::GallicSwordsman | UnitType::WarElephant | UnitType::ChuKoNu | UnitType::Samurai | UnitType::Keshik | UnitType::JaguarWarrior | UnitType::Chasqui | UnitType::Mandekalu | UnitType::AskiaRider | UnitType::Conquistador | UnitType::ManOWar | UnitType::Musketeer | UnitType::Sipahi | UnitType::Cossack | UnitType::Marine | UnitType::Panzer | UnitType::GuardsRifleman | UnitType::Bandeirante | UnitType::LightHorse | UnitType::Mountie | UnitType::Charro | UnitType::Commando | UnitType::BlackHussar | UnitType::Grenzer | UnitType::Bersaglieri => self.unique_base().map_or(Color::WHITE, |b| b.color()),
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
            UnitType::Cartel => Color::srgb(0.3, 0.28, 0.3),
            UnitType::Insurgent => Color::srgb(0.45, 0.4, 0.32),
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
        if let Some(base) = self.unique_base() {
            return base.domain();
        }
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
        if let Some(base) = self.unique_base() {
            return base.is_wheeled();
        }
        matches!(self, UnitType::WarChariot | UnitType::Catapult | UnitType::Cannon | UnitType::Artillery | UnitType::RadarArtillery)
    }

    /// A military unit that fights on the ground: the only kind that holds or takes cities.
    pub fn is_army(&self) -> bool {
        if let Some(base) = self.unique_base() {
            return base.is_army();
        }
        self.is_military() && self.domain() == Domain::Land
    }

    pub fn is_worker(&self) -> bool {
        if let Some(base) = self.unique_base() {
            return base.is_worker();
        }
        matches!(self, UnitType::Worker | UnitType::Engineers)
    }

    /// Turns a worker of this kind takes over a job (Engineers work faster).
    pub fn build_turns(&self) -> u32 {
        if let Some(base) = self.unique_base() {
            return base.build_turns();
        }
        if *self == UnitType::Engineers { 2 } else { 3 }
    }

    pub fn is_military(&self) -> bool {
        if let Some(base) = self.unique_base() {
            return base.is_military();
        }
        !matches!(self, UnitType::Settler | UnitType::Worker | UnitType::Engineers | UnitType::Scout | UnitType::Diplomat | UnitType::Caravan | UnitType::Missionary | UnitType::NuclearMissile | UnitType::Transport) && !self.is_great_person()
    }

    /// How many units this ship carries: troops (any land unit) for the transports, aircraft for
    /// the Carrier. Warships of the iron age and after carry none.
    pub fn capacity(&self) -> u32 {
        if let Some(base) = self.unique_base() {
            return base.capacity();
        }
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
        if let Some(base) = self.unique_base() {
            return base.carries(cargo);
        }
        self.capacity() > 0
            && match self {
                UnitType::Carrier => cargo.domain() == Domain::Air,
                _ => cargo.domain() == Domain::Land,
            }
    }

    /// A ship the computer players use to ferry troops and settlers overseas.
    pub fn is_ferry(&self) -> bool {
        if let Some(base) = self.unique_base() {
            return base.is_ferry();
        }
        matches!(self, UnitType::Galley | UnitType::Galleon | UnitType::Transport)
    }

    /// Strategic resources a city must have access to before it can train this unit.
    pub fn required_resources(&self) -> &'static [Resource] {
        if self.ability() == Some(Ability::SelfSufficient) {
            return &[];
        }
        if let Some(base) = self.unique_base() {
            return base.required_resources();
        }
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

    /// Horse-borne: chariots, horsemen, knights and cavalry (pikemen stand twice as firm against them).
    pub fn is_mounted(&self) -> bool {
        if let Some(base) = self.unique_base() {
            return base.is_mounted();
        }
        matches!(self, UnitType::Horseman | UnitType::WarChariot | UnitType::Knight | UnitType::Cavalry)
    }

    /// Defends at double strength against mounted attackers.
    pub fn anti_mounted(&self) -> bool {
        if self.ability() == Some(Ability::ShieldWall) {
            return true;
        }
        *self == UnitType::Pikeman
    }

    /// A fast unit (two moves or more): on land it may pull back from a losing fight against a
    /// slow one.
    pub fn is_fast(&self) -> bool {
        self.stats().2 >= 2
    }

    /// What this unit can be upgraded to once that is buildable (in a city, for gold).
    pub fn upgrades_to(&self) -> Option<UnitType> {
        if let Some(base) = self.unique_base() {
            return base.upgrades_to();
        }
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

    /// The most advanced unit along the upgrade line that `can_equip` allows: a unit whose next
    /// step needs a resource the city lacks can still skip ahead to one that does not.
    pub fn best_upgrade(&self, can_equip: impl Fn(UnitType) -> bool) -> Option<UnitType> {
        let mut best = None;
        let mut step = *self;
        for _ in 0..12 {
            let Some(next) = step.upgrades_to() else { break };
            if can_equip(next) {
                best = Some(next);
            }
            step = next;
        }
        best
    }

    /// Gold to upgrade to `to`.
    pub fn upgrade_cost(&self, to: UnitType) -> i32 {
        (to.cost() as i32 - self.cost() as i32).max(10) * 2 + 20
    }

    /// Bombardment, as Civilization III rates it: strength, range in tiles, and rate of fire (the
    /// most hit points one bombardment can take). Guns - catapults to radar artillery - only
    /// bombard; some warships and the bomber may bombard instead of attacking.
    pub fn bombard(&self) -> Option<(u32, i32, u32)> {
        if let Some(base) = self.unique_base() {
            return base.bombard();
        }
        match self {
            UnitType::Catapult => Some((4, 1, 1)),
            UnitType::Cannon => Some((8, 1, 1)),
            UnitType::Artillery => Some((12, 2, 2)),
            UnitType::RadarArtillery => Some((16, 2, 3)),
            UnitType::Frigate => Some((3, 1, 1)),
            UnitType::Ironclad => Some((5, 1, 1)),
            UnitType::Battleship => Some((12, 2, 2)),
            UnitType::Bomber => Some((12, 1, 2)),
            _ => None,
        }
    }

    /// Can bombard a tile within its range, without risk.
    pub fn can_bombard(&self) -> bool {
        self.bombard().is_some()
    }

    /// A gun: it bombards, but cannot attack (or take a city).
    pub fn bombard_only(&self) -> bool {
        self.can_bombard() && self.stats().0 == 0
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

    /// Gold to re-equip this unit - every member of an army - with the best that `can_equip`
    /// allows, and what the leader becomes; `None` when nobody can be upgraded.
    pub fn refit_cost(&self, can_equip: impl Fn(UnitType) -> bool) -> Option<(i32, UnitType)> {
        let mut cost = 0;
        let mut any = false;
        let mut leader = self.unit_type;
        for (i, kind) in std::iter::once(self.unit_type).chain(self.troops.iter().copied()).enumerate() {
            if let Some(to) = kind.best_upgrade(&can_equip) {
                cost += kind.upgrade_cost(to);
                any = true;
                if i == 0 {
                    leader = to;
                }
            }
        }
        any.then_some((cost, leader))
    }

    /// Re-equips this unit (and every member of an army) as [`Unit::refit_cost`] priced it,
    /// at full strength.
    pub fn refit(&mut self, can_equip: impl Fn(UnitType) -> bool) {
        if let Some(to) = self.unit_type.best_upgrade(&can_equip) {
            self.unit_type = to;
        }
        for troop in &mut self.troops {
            if let Some(to) = troop.best_upgrade(&can_equip) {
                *troop = to;
            }
        }
        self.hp = self.max_hp();
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

/// Most units in one army (the leader and its troops), three as in Civilization III; Sun Tzu's War
/// Academy adds one more.
pub const ARMY_SIZE: usize = 3;

/// How many units an army of this owner can hold.
pub fn army_capacity(sun_tzu: bool) -> usize {
    ARMY_SIZE + usize::from(sun_tzu)
}

/// Each member beyond the first makes an army fight this much harder (on top of pooling their
/// hit points).
pub const ARMY_MEMBER_BONUS: f32 = 0.1;

/// Units that can join an army: ground troops (not guns) that are not already in one.
pub fn can_join_army(unit: &Unit) -> bool {
    unit.unit_type.is_army() && !unit.unit_type.bombard_only() && unit.troops.is_empty()
}

/// Forms an army from units standing together: the strongest leads, and up to `capacity` in all
/// march with it. Returns the leader's index in `units` and the indices of the units absorbed (to
/// remove), or None if fewer than two units can join.
pub fn plan_army(units: &[&Unit], capacity: usize) -> Option<(usize, Vec<usize>)> {
    let mut eligible: Vec<usize> = (0..units.len()).filter(|i| can_join_army(units[*i])).collect();
    if eligible.len() < 2 {
        return None;
    }
    let power = |u: &Unit| {
        let (a, d, _) = u.unit_type.stats();
        (a + d) * 10 + u.level as u32
    };
    eligible.sort_by_key(|i| std::cmp::Reverse(power(units[*i])));
    eligible.truncate(capacity);
    Some((eligible[0], eligible[1..].to_vec()))
}

impl Unit {
    /// Units in this army, the leader included (1 for a lone unit).
    pub fn members(&self) -> u32 {
        1 + self.troops.len() as u32
    }

    /// Takes a unit into this army, if there is room: its hit points come with it.
    pub fn take_into_army(&mut self, recruit: &Unit, capacity: usize) -> bool {
        if !self.is_army_group() || self.members() as usize >= capacity || !can_join_army(recruit) {
            return false;
        }
        self.troops.push(recruit.unit_type);
        self.hp = (self.hp + recruit.hp).min(self.max_hp());
        true
    }
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

    #[test]
    fn upgrades_skip_a_step_the_city_cannot_equip() {
        // No saltpeter: a longbowman cannot become a musketman or rifleman, but can become infantry.
        let no_saltpeter = |to: UnitType| !to.required_resources().contains(&Resource::Saltpeter) && to != UnitType::MechInfantry;
        assert_eq!(UnitType::Longbowman.best_upgrade(no_saltpeter), Some(UnitType::Infantry));
        assert_eq!(UnitType::Longbowman.best_upgrade(|_| false), None);
        assert_eq!(UnitType::Warrior.best_upgrade(|to| to == UnitType::Spearman), Some(UnitType::Spearman));
    }

    fn unit(kind: UnitType, hp: u32, level: u8) -> Unit {
        Unit { unit_type: kind, movement_left: 1, owner: 0, position: Hex::ZERO, build_turns_left: None, current_build: None, fortified: false, road_credit: 0, hp, level, troops: Vec::new(), fatigue: 0, carrier: None }
    }

    #[test]
    fn the_strongest_leads_and_the_army_pools_hit_points() {
        let units = [unit(UnitType::Spearman, 3, 0), unit(UnitType::Musketman, 2, 1), unit(UnitType::Worker, 3, 0), unit(UnitType::Knight, 3, 0), unit(UnitType::Warrior, 3, 0)];
        let refs: Vec<&Unit> = units.iter().collect();
        let (leader, absorbed) = plan_army(&refs, 3).unwrap();
        // The knight (attack 4, defence 3) outranks the veteran musketman (2, 4).
        assert_eq!(units[leader].unit_type, UnitType::Knight);
        assert_eq!(absorbed, vec![1, 0]);
        assert!(!absorbed.contains(&2), "workers do not join");
        let mut army = units[leader].clone();
        let members: Vec<Unit> = absorbed.iter().map(|i| units[*i].clone()).collect();
        join_army(&mut army, &members);
        assert_eq!(army.max_hp(), 9);
        assert_eq!(army.hp, 8);
        assert_eq!(army.troop_moves(), 1);
        assert!(army.display_name().starts_with("Army (Knight"));
        // An army does not join another.
        assert!(plan_army(&[&army, &units[0]], 3).is_none());
        // A unit joins the army while there is room: 3 units, or 4 with Sun Tzu.
        assert!(!army.take_into_army(&units[0], army_capacity(false)), "full");
        assert!(!army.take_into_army(&units[2], army_capacity(true)), "workers never join");
        assert!(army.take_into_army(&units[0], army_capacity(true)));
        assert_eq!((army.members(), army.max_hp(), army.hp), (4, 12, 11));
        assert!(!army.take_into_army(&units[4], army_capacity(true)), "full");
    }
}
