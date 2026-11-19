//! Bevy Civ — game setup, turn processing and player orders.
//! Everything visual (sprites, camera, interface) lives in `client`.

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use hexx::{Hex, HexLayout};
use std::collections::HashMap;

use bevy_civ::ai::{process_ai_turn, Civilizations};
use std::collections::HashSet;
use bevy_civ::barbarians::{process_barbarians, BarbarianState};
use bevy_civ::building::{Building, CityBuildings, WorldWonders};
use bevy_civ::city::{spawn_city, City, Growth, ProductionItem};
use bevy_civ::combat::{self, Ground, Outcome, Side};
use bevy_civ::villages::{Find, Villages};
use bevy_civ::religion::{Faith, Religions};
use bevy_civ::great::{self, GreatPeople};
use bevy_civ::culture::{self as culture_rules, CultureLedger};
use bevy_civ::espionage::{self, Espionage, Mission};
use rand::Rng;
use bevy_civ::diplomacy::{Diplomacy, Happiness, Treaty};
use bevy_civ::fog::FogOfWar;
use bevy_civ::government::Government;
use bevy_civ::pathfinding::{find_path, find_route, land_step, road_network, turns_along, Road, StepCost, Travel};
use bevy_civ::unit::{Domain, AIR_RANGE};
use bevy_civ::cargo::{self, Berth};
use bevy_civ::score::Score;
use bevy_civ::space::SpaceRace;
use bevy_civ::tech::{Tech, TechState};
use bevy_civ::terrain::{random_resource, HexTile, Improvement, Terrain};
use bevy_civ::trade::TradeRoutes;
use bevy_civ::unit::{spawn_unit, Unit, UnitType};

mod autoplay;
mod client;
mod foreign;
mod governor;
mod savegame;
mod scenario;
mod supply;
mod telemetry;

use client::sfx::Sfx;
use foreign::{DiploAct, Foreign, TradeAsk};
use supply::Supplies;
use scenario::NewGame;

pub const PLAYER: u32 = 0;
pub const BARBARIANS: u32 = 99;
/// The longest game; if nobody has won by its last turn, the highest score does.
pub const LAST_TURN: u32 = 400;

/// How hard the computer players are: they gather shields and science faster (or slower), and on
/// the easiest levels the player's people are easier to please.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Difficulty {
    Chieftain,
    Warlord,
    #[default]
    Regent,
    Monarch,
    Emperor,
}

impl Difficulty {
    pub const ALL: [Difficulty; 5] = [Difficulty::Chieftain, Difficulty::Warlord, Difficulty::Regent, Difficulty::Monarch, Difficulty::Emperor];

    pub fn name(&self) -> &'static str {
        match self {
            Difficulty::Chieftain => "Chieftain",
            Difficulty::Warlord => "Warlord",
            Difficulty::Regent => "Regent",
            Difficulty::Monarch => "Monarch",
            Difficulty::Emperor => "Emperor",
        }
    }

    /// What the computer players' shields and science are multiplied by.
    pub fn ai_output(&self) -> f32 {
        match self {
            Difficulty::Chieftain => 0.6,
            Difficulty::Warlord => 0.8,
            Difficulty::Regent => 1.0,
            Difficulty::Monarch => 1.2,
            Difficulty::Emperor => 1.4,
        }
    }

    /// Extra content citizens in each of the player's cities.
    pub fn player_content(&self) -> u32 {
        match self {
            Difficulty::Chieftain => 2,
            Difficulty::Warlord => 1,
            _ => 0,
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            Difficulty::Chieftain => "Learning the game: slow rivals, easy-going citizens",
            Difficulty::Warlord => "Gentle rivals",
            Difficulty::Regent => "Even terms",
            Difficulty::Monarch => "Rivals build and learn a fifth faster",
            Difficulty::Emperor => "Rivals build and learn two fifths faster",
        }
    }
}

/// What every people still in the game (holding a city) knows: the computer players' and ours.
fn peoples_knowledge(civs: &Civilizations, tech: &TechState, holding: &HashSet<u32>) -> Vec<(u32, HashSet<Tech>)> {
    civs.players
        .iter()
        .filter(|c| !c.is_human && !c.eliminated && holding.contains(&c.id))
        .map(|c| (c.id, c.tech.researched.clone()))
        .chain(holding.contains(&PLAYER).then(|| (PLAYER, tech.researched.clone())))
        .collect()
}

/// The first victory of a people's unique unit over another people (not barbarians) begins its
/// golden age, if it has not had one.
fn unique_victory(golden: &mut GoldenAges, civs: &Civilizations, met: &HashSet<u32>, log: &mut GameLog, winner: u32, kind: UnitType, loser: u32) {
    if winner == BARBARIANS || loser == BARBARIANS {
        return;
    }
    let Some(civ) = civs.players.iter().find(|c| c.id == winner) else { return };
    if bevy_civ::traits::of(&civ.name).unique_unit() != Some(kind) || !golden.begin(winner) {
        return;
    }
    if winner == PLAYER {
        log.push(format!("Our {} win their first battle against another people: a golden age begins! For {GOLDEN_AGE_TURNS} turns every worked tile that yields a shield or gold yields one more.", kind.name()));
    } else if met.contains(&winner) {
        log.push(format!("The victories of the {} {} begin a golden age for their people.", civ.adjective, kind.name()));
    }
}

/// The Great Library: its owner learns every technology that two or more of the other peoples it
/// has met already know (and that it may study), until the wonder is obsolete (see
/// `Building::obsoleted_by`).
fn great_library(civs: &mut Civilizations, tech: &mut TechState, wonders: &WorldWonders, met: &HashSet<u32>, holding: &HashSet<u32>, log: &mut GameLog) {
    let Some(&(owner, _)) = wonders.built.get(&Building::GreatLibrary) else { return };
    let knowledge = peoples_knowledge(civs, tech, holding);
    let known_to = |t: Tech| knowledge.iter().filter(|(id, k)| *id != owner && (owner != PLAYER || met.contains(id)) && k.contains(&t)).count();
    let learner = if owner == PLAYER { Some(tech) } else { civs.players.iter_mut().find(|c| c.id == owner && !c.eliminated).map(|c| &mut c.tech) };
    let Some(learner) = learner else { return };
    if Building::GreatLibrary.obsoleted_by().is_some_and(|t| learner.researched.contains(&t)) {
        return;
    }
    while let Some(t) = Tech::ALL.into_iter().find(|t| learner.can_research(*t) && known_to(*t) >= 2) {
        learner.learn(t);
        if owner == PLAYER {
            log.push(format!("The {} teaches us {}, already known to {} peoples.", Building::GreatLibrary.name(), t.name(), known_to(t)));
        }
    }
}

/// A people entering a new age: the news, and a free discovery for a scientific people.
fn new_ages(civs: &mut Civilizations, tech: &mut TechState, met: &HashSet<u32>, holding: &HashSet<u32>, log: &mut GameLog) {
    for civ in civs.players.iter_mut().filter(|c| !c.is_human && !c.eliminated) {
        let Some(era) = civ.tech.enter_new_era() else { continue };
        let scientific = bevy_civ::traits::of(&civ.name).has(bevy_civ::traits::Trait::Scientific);
        let gift = if scientific { civ.tech.discover() } else { None };
        if met.contains(&civ.id) {
            let gift = gift.map(|t| format!(" Their scholars mark it with a discovery: {}.", t.name())).unwrap_or_default();
            log.push(format!("{} enter the {}.{gift}", capitalised(&civ.name), era.name()));
        }
    }
    if !holding.contains(&PLAYER) {
        return;
    }
    if let Some(era) = tech.enter_new_era() {
        let scientific = civs.players.iter().find(|c| c.id == PLAYER).is_some_and(|c| bevy_civ::traits::of(&c.name).has(bevy_civ::traits::Trait::Scientific));
        match scientific.then(|| tech.discover()).flatten() {
            Some(gift) => log.push(format!("We enter the {}! Our scientific people marks it with a free discovery: {}.", era.name(), gift.name())),
            None => log.push(format!("We enter the {}.", era.name())),
        }
    }
}

/// A computer player's score, counted the same way as the player's (population, cities' buildings).
/// A computer player's score, counted exactly as the player's is (see `Score::calculate`): two points
/// a citizen, ten a technology, fifty a wonder.
fn rival_score(civ: &bevy_civ::ai::Civ, cities: &Query<(&mut City, &mut CityBuildings, &mut Happiness)>) -> u32 {
    let held: u32 = cities
        .iter()
        .filter(|(c, _, _)| c.owner == civ.id)
        .map(|(c, b, _)| c.population * 2 + b.buildings.iter().map(|w| if w.is_wonder() { 50 } else { 0 }).sum::<u32>())
        .sum();
    held + civ.tech.researched.len() as u32 * 10
}

/// "the Gauls" -> "The Gauls" at the start of a sentence.
pub fn capitalised(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

/// Aggressive computer players leave neighbours they are not at war with alone until this turn.
pub const AI_GRACE_TURNS: u32 = 25;
/// Tiles around a city that belong to its owner.
pub const CITY_RADIUS: i32 = 2;

const CITY_NAMES: [&str; 10] = [
    "Ashford", "Brightwater", "Stonehaven", "Eastmarch", "Highgate", "Riverbend", "Oakhollow",
    "Silverkeep", "Windmere", "Thornwall",
];

/// Start menu or a game in progress.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    Playing,
}

#[derive(Resource)]
pub struct MapData {
    pub layout: HexLayout,
    /// Every tile of the map.
    pub cells: HashSet<Hex>,
    pub water: HashSet<Hex>,
    /// The terrain of every tile (kept in step with the tiles when land is cleared).
    pub terrain: HashMap<Hex, Terrain>,
    /// World-space bounding box of the tile centres.
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    pub turn: u32,
    pub gold: i32,
    pub gold_per_turn: i32,
    pub government: Government,
    pub victory: Option<String>,
    pub score: Score,
    pub start_year: i32,
    pub years_per_turn: i32,
    /// Name of the player's civilization, shown in the status bar.
    pub title: String,
    /// Turns of anarchy left in a revolution, and the government it leads to.
    pub anarchy_turns: u32,
    pub pending_government: Option<Government>,
    #[serde(default)]
    pub difficulty: Difficulty,
    /// The game ends on this turn if nobody has won.
    #[serde(default = "default_last_turn")]
    pub last_turn: u32,
    /// Turns each civilization has been at war (falls back two a turn at peace): the people of a
    /// republic or democracy tire of it.
    #[serde(default)]
    pub war_turns: HashMap<u32, u32>,
    /// Times global warming has struck.
    #[serde(default)]
    pub warming: u32,
    /// Cities that drafted a citizen this turn.
    #[serde(default)]
    pub drafted: HashSet<Hex>,
    /// Cities that sold a building this turn (one a turn).
    #[serde(default)]
    pub sold: HashSet<Hex>,
    /// Foreign migrants may settle in our cities.
    #[serde(default = "welcome_by_default")]
    pub welcome_migrants: bool,
    /// Citizens who have moved between cities so far, and how many of them crossed a border.
    #[serde(default)]
    pub migrations: (u32, u32),
    /// Modern raiders so far: cartel bands and terror cells formed, and raids they made on cities.
    #[serde(default)]
    pub raiders: (u32, u32, u32),
    /// How the player's commerce is split, in percent: science and luxury; taxes take the rest.
    #[serde(default = "default_science_rate")]
    pub science_rate: u32,
    #[serde(default)]
    pub luxury_rate: u32,
    /// Interest earned on the treasury last turn (included in `gold_per_turn`).
    #[serde(default)]
    pub interest: i32,
    /// Every people's accounts last turn (trade, corruption, science, luxury, taxes, upkeep), the
    /// player's under `PLAYER`.
    #[serde(default)]
    pub accounts: HashMap<u32, bevy_civ::economy::Ledger>,
    /// Gold the player paid last turn for units beyond those supported free.
    #[serde(default)]
    pub unit_support: i32,
}

fn default_science_rate() -> u32 {
    60
}

impl GameState {
    pub fn tax_rate(&self) -> u32 {
        self.rates().tax()
    }

    /// The player's rates, as the government allows them.
    pub fn rates(&self) -> bevy_civ::economy::Rates {
        bevy_civ::economy::Rates { science: self.science_rate, luxury: self.luxury_rate }.fit(self.government)
    }

    /// Sets the rates, brought within what the government allows.
    pub fn set_rates(&mut self, rates: bevy_civ::economy::Rates) {
        let rates = rates.fit(self.government);
        self.science_rate = rates.science;
        self.luxury_rate = rates.luxury;
    }
}

fn default_last_turn() -> u32 {
    LAST_TURN
}

fn welcome_by_default() -> bool {
    true
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            turn: 1,
            gold: 0,
            gold_per_turn: 0,
            government: Government::Despotism,
            victory: None,
            score: Score::default(),
            start_year: -4000,
            years_per_turn: 40,
            title: String::new(),
            anarchy_turns: 0,
            pending_government: None,
            difficulty: Difficulty::Regent,
            war_turns: HashMap::new(),
            warming: 0,
            drafted: HashSet::new(),
            sold: HashSet::new(),
            welcome_migrants: true,
            migrations: (0, 0),
            raiders: (0, 0, 0),
            last_turn: LAST_TURN,
            science_rate: 60,
            luxury_rate: 0,
            interest: 0,
            accounts: HashMap::new(),
            unit_support: 0,
        }
    }
}

#[derive(Resource, Default)]
pub struct Selection {
    pub unit: Option<Entity>,
    pub city: Option<Entity>,
}

/// Messages shown to the player, newest last.
#[derive(Resource, Default)]
pub struct GameLog {
    pub lines: Vec<String>,
    /// Every line ever written (the log keeps only the latest).
    pub written: usize,
    /// The game's messages with the turn of each, for the Messages window (the latest 2,000).
    pub history: std::collections::VecDeque<(u32, String)>,
    /// The current turn (stamped on new messages).
    pub turn: u32,
}

/// Words that mark a message as one of the game's great events (the Messages window's highlights).
pub const HIGHLIGHTS: [&str; 19] = [
    "Wonder of the world", "completes the", "means war", "captured", "has fallen", "capture", "priests found", "We have founded",
    "launched", "lifts off", "nuclear", "Secretary-General", "golden age", "is born", "are no more", "Global warming", "Revolution", "Discovery", "spaceship",
];

impl GameLog {
    pub fn push(&mut self, line: impl Into<String>) {
        self.written += 1;
        let line = line.into();
        self.history.push_back((self.turn, line.clone()));
        if self.history.len() > 2000 {
            self.history.pop_front();
        }
        self.lines.push(line);
        if self.lines.len() > 60 {
            self.lines.remove(0);
        }
    }
}

/// What needs redrawing after the game state changed.
#[derive(Resource)]
pub struct Dirty {
    pub map: bool,
    pub ui: bool,
}

/// The route drawn on the map: to the hovered tile while one of the player's units is selected,
/// otherwise the rest of the selected unit's standing order. Each step after the start carries the
/// turn it is reached on (0 = this turn).
#[derive(Resource, Default, PartialEq)]
pub struct PathPreview {
    pub start: Option<Hex>,
    pub steps: Vec<(Hex, u32)>,
}

/// A destination the player sent a unit to: it keeps marching there on the following turns.
#[derive(Component, Clone, Copy, PartialEq)]
pub struct GoTo(pub Hex);

/// The tiles a unit just walked through, so its figure can follow the road instead of cutting across.
#[derive(Component, Default)]
pub struct Trail(pub Vec<Hex>);

/// A player order, from the keyboard or an interface button.
#[derive(Event, Clone, Debug, PartialEq)]
pub enum Action {
    EndTurn,
    /// Space bar: skip the selected unit, move on to the next one, or end the turn when none is left.
    SkipOrEnd,
    /// Jump to the next unit that still needs orders without skipping the current one.
    NextUnit,
    Fortify,
    /// Sleep until an enemy comes near (the unit then wakes and asks for orders).
    Sentry,
    /// A worker or settler settles in the city it stands in, adding to its people.
    JoinCity,
    /// The selected city's governor works the land for this focus.
    SetFocus(bevy_civ::tiles::Focus),
    /// Troops (or aircraft) in port go aboard a ship there with room.
    Board,
    /// Units aboard a ship in port leave it (a ship lets all of its cargo off).
    Unload,
    /// A resource deal proposed to another leader.
    Trade(u32, TradeAsk),
    Build(Improvement),
    Transform,
    FoundCity,
    Skip,
    Deselect,
    /// A request to another leader, made in the foreign advisor window.
    Diplo(u32, DiploAct),
    /// The player's answer to a pending proposal.
    AnswerOffer(bool),
    /// T: the selected caravan opens a trade route from its home city to the city it stands in.
    TradeRoute,
    Save,
    /// Pick one of the player's units (from the list of units sharing a tile).
    Select(Entity),
    /// U: the selected great person does what it is for (discovery, trade mission, hurry, masterpiece),
    /// or a caravan helps build the wonder in the city it stands in.
    UseUnit,
    /// A mission for the selected diplomat at the foreign city it stands in or next to.
    Spy(Mission),
    /// Overthrow the government for another: a few turns of anarchy first.
    Revolution(Government),
    /// Select one of the player's cities (from the cities advisor) and look at it.
    SelectCity(Entity),
    /// The selected unit, in one of the player's cities, is re-equipped as its modern successor.
    Upgrade,
    /// Every unit of ours in the selected unit's city is re-equipped, while the treasury lasts.
    UpgradeAll,
    /// Burn the selected city, taken this turn, to the ground.
    Raze,
    /// Let the governor choose all the selected city's worked tiles again.
    AutoTiles,
    /// Launch our completed spaceship.
    LaunchShip,
    /// Give up: the game ends in defeat, and the final scores and the replay are shown.
    Surrender,
    /// Draft a citizen of the selected city into its best defender.
    Draft,
    /// Take back the last plain move.
    Undo,
    /// Add a project to the end of the selected city's build queue.
    QueueProduction(ProductionItem),
    /// Take the project at this place out of the selected city's build queue.
    Unqueue(usize),
    /// The selected unit wrecks the best improvement on its tile (enemy land only).
    Pillage,
    /// Set the share of commerce going to science and to luxury (percent; taxes take the rest).
    SetRates(u32, u32),
    /// Whether foreign migrants may settle in our cities.
    SetMigration(bool),
    /// The selected city's governor chooses its builds (true) or the player does.
    AutoBuild(bool),
    /// Every one of our cities' governors chooses its builds (true), or none does.
    AutoBuildAll(bool),
    /// Buy the rest of the selected city's project with gold (not wonders).
    Hurry,
    /// Sell one of the selected city's buildings for its cost in gold (one a turn).
    SellBuilding(Building),
    /// Rush the selected city's project with citizens' labour (Despotism and Communism).
    Rush,
    /// K: the selected worker looks after the land by itself (again to take it back).
    Automate,
    /// C: the selected scout, soldier or ship explores by itself (again to take it back).
    Explore,
    /// Make one of the selected city's workers a specialist of this kind (true), or send one back (false).
    Specialist(bevy_civ::city::Specialist, bool),
    /// The selected gun, warship or bomber bombards the stoutest enemy within its range (a city's
    /// garrison first).
    Bombard,
    /// The selected gun, warship or bomber bombards the defender of this tile (if it is in range).
    BombardAt(Hex),
    /// Tab: the next of the player's units on the selected unit's or city's tile.
    CycleStack,
    /// Shift+Tab: the previous unit on the tile.
    CycleStackBack,
    /// The next (true) or previous of our cities, in the order they were founded.
    CycleCity(bool),
    /// Fortify (true) or wake (false) every unit of ours on the selected unit's tile.
    StackOrder(bool),
    SetProduction(ProductionItem),
    Research(Tech),
    /// Work towards a technology: research follows the path to it.
    ResearchGoal(Tech),
    ClearResearchGoal,
}

/// A click (or tap) on a map tile. `secondary` is the right mouse button: always an order.
#[derive(Event)]
pub struct MapClick {
    pub hex: Hex,
    pub secondary: bool,
    /// Move every unit of the same kind (land, sea or air) on the selected unit's tile.
    pub group: bool,
}

/// Rivers: each a chain of tiles running from high ground down to the sea.
#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Rivers {
    pub paths: Vec<Vec<Hex>>,
    pub tiles: HashSet<Hex>,
}

impl Rivers {
    /// Traces rivers from some hills and mountains, always downhill (and towards the sea), until they
    /// reach water. Roughly one river per sixty land tiles.
    pub fn generate(terrain: &HashMap<Hex, Terrain>, rng: &mut impl Rng) -> Self {
        let height = |t: Terrain| match t {
            Terrain::Water => 0,
            Terrain::Grassland | Terrain::Plains | Terrain::Desert => 1,
            Terrain::Forest => 2,
            Terrain::Hills => 3,
            Terrain::Mountain => 4,
        };
        // Distance from the sea, by breadth-first search from every water tile.
        let mut to_sea: HashMap<Hex, i32> = terrain.iter().filter(|(_, t)| **t == Terrain::Water).map(|(h, _)| (*h, 0)).collect();
        let mut frontier: Vec<Hex> = to_sea.keys().copied().collect();
        while !frontier.is_empty() {
            let mut next = Vec::new();
            for hex in frontier {
                let d = to_sea[&hex];
                for n in hex.all_neighbors() {
                    if terrain.contains_key(&n) && !to_sea.contains_key(&n) {
                        to_sea.insert(n, d + 1);
                        next.push(n);
                    }
                }
            }
            frontier = next;
        }
        let mut sources: Vec<Hex> = terrain.iter().filter(|(_, t)| matches!(t, Terrain::Hills | Terrain::Mountain)).map(|(h, _)| *h).collect();
        sources.sort_by_key(|h| (h.x, h.y));
        let land = terrain.values().filter(|t| **t != Terrain::Water).count();
        let wanted = land / 60;
        let mut rivers = Rivers::default();
        for _ in 0..wanted * 4 {
            if rivers.paths.len() >= wanted || sources.is_empty() {
                break;
            }
            let start = sources[rng.gen_range(0..sources.len())];
            if rivers.tiles.contains(&start) || start.all_neighbors().iter().any(|n| rivers.tiles.contains(n)) {
                continue;
            }
            let mut path = vec![start];
            let mut here = start;
            while path.len() < 30 {
                let step = here
                    .all_neighbors()
                    .into_iter()
                    .filter(|n| terrain.contains_key(n) && !path.contains(n))
                    .min_by_key(|n| (height(terrain[n]), to_sea.get(n).copied().unwrap_or(99)));
                let Some(step) = step else { break };
                if height(terrain[&step]) > height(terrain[&here]) {
                    break;
                }
                path.push(step);
                here = step;
                if terrain[&step] == Terrain::Water || rivers.tiles.contains(&step) {
                    break;
                }
            }
            let reached_water = path.last().is_some_and(|h| terrain[h] == Terrain::Water || rivers.tiles.contains(h));
            if path.len() >= 4 && reached_water {
                rivers.tiles.extend(path.iter().filter(|h| terrain[*h] != Terrain::Water));
                rivers.paths.push(path);
            }
        }
        rivers
    }
}

/// A worker the player has handed over to automation: it finds its own jobs every turn.
#[derive(Component)]
pub struct Automated;

/// A scout, soldier or ship the player has sent exploring: each turn it heads for the nearest edge of
/// the known world, until nothing is left to find (or the player gives it another order).
#[derive(Component)]
pub struct Exploring;

/// Cities the player took this turn: they can still be razed.
#[derive(Resource, Default)]
pub struct JustCaptured(pub HashSet<Hex>, pub Vec<u32>);

/// Ends the player's turn (sent once the order to end it is confirmed).
#[derive(Event)]
pub struct EndTurnNow;

/// The player asked to end the turn (Enter, the button); `end_turn_guard` may first warn about a
/// city left open to enemies, then sends `EndTurnNow`.
#[derive(Event)]
pub struct EndTurnRequest;

/// Asks the camera to centre on a tile.
#[derive(Event)]
pub struct Focus(pub Hex);

/// Set when the selection should move on to the next unit that needs orders.
#[derive(Resource, Default)]
pub struct Cycle {
    pub advance: bool,
    /// Move on even if the selected unit could still act.
    pub force: bool,
}

/// A player unit that still expects orders this turn.
pub fn needs_orders(unit: &Unit) -> bool {
    unit.owner == PLAYER && unit.can_move() && unit.build_turns_left.is_none() && !unit.fortified
}

/// Golden ages, once per game for each people: begun by its first wonder that speaks to one of its
/// traits (see `bevy_civ::traits::of_wonder`), or by the first victory of its unique unit over
/// another people. For [`GOLDEN_AGE_TURNS`] turns every worked tile that gives a shield gives one
/// more, and so does every tile that gives a trade (see `bevy_civ::economy::tile_rule`).
#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoldenAges {
    #[serde(with = "bevy_civ::save::pairs")]
    pub turns_left: HashMap<u32, u32>,
    pub had: HashSet<u32>,
}

pub const GOLDEN_AGE_TURNS: u32 = 20;

impl GoldenAges {
    pub fn active(&self, owner: u32) -> bool {
        self.turns_left.get(&owner).is_some_and(|t| *t > 0)
    }

    /// What a worked tile yields to a people in its golden age.
    pub fn tile(food_shields_gold: bevy_civ::tiles::Yield) -> bevy_civ::tiles::Yield {
        let (food, shields, gold) = food_shields_gold;
        (food, shields + u32::from(shields > 0), gold + u32::from(gold > 0))
    }

    /// Starts one if the civilization never had one. Returns whether it started.
    pub fn begin(&mut self, owner: u32) -> bool {
        if !self.had.insert(owner) {
            return false;
        }
        self.turns_left.insert(owner, GOLDEN_AGE_TURNS);
        true
    }

    pub fn tick(&mut self) {
        self.turns_left.retain(|_, t| {
            *t = t.saturating_sub(1);
            *t > 0
        });
    }
}

/// Every civilization's score, turn by turn, for the score advisor - and, for the replay at the end
/// of the game, how borders and cities changed hands.
#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct History {
    /// (turn, civilization, score)
    pub scores: Vec<(u32, u32, u32)>,
    /// Tiles whose owner changed: (turn, q, r, new owner or [`NOBODY`]).
    #[serde(default)]
    pub borders: Vec<(u32, i32, i32, u32)>,
    /// Cities founded, taken or destroyed: (turn, q, r, owner or [`NOBODY`]).
    #[serde(default)]
    pub cities: Vec<(u32, i32, i32, u32)>,
}

/// No owner (an unclaimed tile, or a city that is gone).
pub const NOBODY: u32 = u32::MAX;

impl History {
    /// Who owned each tile, and each city, at the end of `turn`, replayed from the changes.
    pub fn at(&self, turn: u32) -> (HashMap<Hex, u32>, HashMap<Hex, u32>) {
        let play = |log: &[(u32, i32, i32, u32)]| {
            let mut owners = HashMap::new();
            for (t, q, r, owner) in log.iter().take_while(|(t, ..)| *t <= turn) {
                let _ = t;
                if *owner == NOBODY {
                    owners.remove(&Hex::new(*q, *r));
                } else {
                    owners.insert(Hex::new(*q, *r), *owner);
                }
            }
            owners
        };
        (play(&self.borders), play(&self.cities))
    }

    /// The first and last turns recorded.
    pub fn span(&self) -> (u32, u32) {
        let turns = self.borders.iter().chain(&self.cities).map(|(t, ..)| *t).chain(self.scores.iter().map(|(t, ..)| *t));
        turns.fold((u32::MAX, 0), |(lo, hi), t| (lo.min(t), hi.max(t)))
    }
}

/// Records, once a turn, the tiles and cities that changed hands (for the replay).
pub fn record_history(state: Res<GameState>, supplies: Res<Supplies>, cities: Query<&City>, mut history: ResMut<History>, mut last: Local<Option<u32>>) {
    if *last == Some(state.turn) || state.turn == 0 {
        return;
    }
    *last = Some(state.turn);
    let (borders, towns) = history.at(state.turn);
    let turn = state.turn;
    let now: HashMap<Hex, u32> = supplies.territory.iter().map(|(hex, (owner, _))| (*hex, *owner)).collect();
    let mut changes: Vec<(u32, i32, i32, u32)> = now.iter().filter(|(hex, owner)| borders.get(hex) != Some(owner)).map(|(hex, owner)| (turn, hex.x, hex.y, *owner)).collect();
    changes.extend(borders.keys().filter(|hex| !now.contains_key(hex)).map(|hex| (turn, hex.x, hex.y, NOBODY)));
    changes.sort();
    history.borders.extend(changes);
    let now: HashMap<Hex, u32> = cities.iter().map(|c| (c.position, c.owner)).collect();
    let mut changes: Vec<(u32, i32, i32, u32)> = now.iter().filter(|(hex, owner)| towns.get(hex) != Some(owner)).map(|(hex, owner)| (turn, hex.x, hex.y, *owner)).collect();
    changes.extend(towns.keys().filter(|hex| !now.contains_key(hex)).map(|hex| (turn, hex.x, hex.y, NOBODY)));
    changes.sort();
    history.cities.extend(changes);
}

/// The city a unit was trained in (a caravan's trade routes start there).
#[derive(Component, Clone, Copy)]
pub struct HomeCity(pub Hex);

/// Plays a combat effect on a tile.
#[derive(Event)]
pub struct Burst(pub Hex);

/// A battle to show the player: the attacker lunges from `from` at `at`, blows are exchanged and
/// the result floats over the tile. `ours` marks battles the player fought (the camera goes to
/// them); `good` colours the result for the player (None when neither side is theirs).
/// A nuclear missile launched by `by` at `at` (the missile itself is already gone).
#[derive(Event, Clone, Copy, Debug)]
pub struct Detonation {
    pub at: Hex,
    pub by: u32,
}

/// How far a nuclear missile flies, and how near an SDI Defense must be to shoot one down.
pub const MISSILE_RANGE: i32 = 12;
pub const SDI_RANGE: i32 = 3;

/// The player's last plain move, which can be taken back: the unit and how it stood before.
#[derive(Resource, Default)]
pub struct LastMove(pub Option<(Entity, Unit)>);

/// A unit on sentry: it sleeps until an enemy comes within two tiles.
#[derive(Component)]
pub struct Sentry;

/// The tile the player's selected missile is aimed at (a second click there launches it).
#[derive(Resource, Default)]
pub struct NukeAim(pub Option<Hex>);

#[derive(Event, Clone, Debug)]
pub struct BattleShown {
    pub from: Hex,
    pub at: Hex,
    pub attacker: Option<Entity>,
    pub result: String,
    pub good: Option<bool>,
    pub ours: bool,
    /// The two sides (kind and owner), and which of them died.
    pub attacker_side: (UnitType, u32),
    pub defender_side: (UnitType, u32),
    pub attacker_died: bool,
    pub defender_died: bool,
}

/// The theme to dress the game in: `--theme <id>` on the command line, `?theme=<id>` in the page.
fn chosen_theme(args: &[String]) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_name = bevyCivThemeId, catch)]
            fn js_theme() -> Result<Option<String>, JsValue>;
        }
        let _ = args;
        js_theme().ok().flatten()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        args.iter().position(|a| a == "--theme").and_then(|i| args.get(i + 1)).cloned()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let theme = chosen_theme(&args).unwrap_or_else(|| "earth".into());
    if !bevy_civ::theme::activate_built_in(&theme) {
        bevy_civ::theme::activate_built_in("earth");
    }
    // `--autoplay [scenario] [turns]`: a whole game between computer players, without a window.
    if let Some(i) = args.iter().position(|a| a == "--autoplay") {
        let which = args.get(i + 1).cloned().unwrap_or_else(|| "rome".into());
        let turns = args.get(i + 2).and_then(|t| t.parse().ok()).unwrap_or(LAST_TURN);
        std::process::exit(if autoplay::run(&which, turns) { 0 } else { 1 });
    }
    let mut app = App::new();
    app
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: bevy_civ::theme::term("game_title", "Bevy Civ").into(),
                        // Web build: render into the page's canvas. Its size follows the page through
                        // `client::layout::fit_canvas`, not the engine's fit-to-parent mode, which
                        // gets the pixel ratio wrong on high-density screens.
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: false,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    // Static hosting serves no `.meta` files; don't request them.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.07, 0.065, 0.055)))
        .init_state::<AppState>()
        .add_plugins(client::ClientPlugin);
    init_game(&mut app);
    app
        .add_systems(
            OnEnter(AppState::Playing),
            (
                setup_world.run_if(not(resource_exists::<savegame::PendingLoad>)),
                savegame::restore_world.run_if(resource_exists::<savegame::PendingLoad>),
            ),
        )
        .add_systems(Update, savegame::write_save.after(GameLogic).run_if(in_state(AppState::Playing)))
        .add_systems(OnExit(AppState::Playing), cleanup_world)
        .add_systems(Update, (record_history, telemetry::report_milestones).chain().after(GameLogic).run_if(in_state(AppState::Playing)))
        .add_systems(
            Update,
            (handle_map_click, apply_actions, end_turn_guard, cycle_cities, withdraw_troops, stow_cargo, process_turn, religion_turn, auto_explore, continue_orders, stow_cargo, visit_villages, automate_workers, advance_selection, preview_route, refresh_supplies, work_tiles, nuclear_strikes)
                .chain()
                .in_set(GameLogic)
                .run_if(in_state(AppState::Playing)),
        )
        .run();
}

/// The game's state and events, shared by the real game and the headless autoplay.
pub fn init_game(app: &mut App) {
    app
        .init_resource::<GameState>()
        .init_resource::<Religions>()
        .init_resource::<Villages>()
        .insert_resource(Dirty { map: true, ui: true })
        .init_resource::<TechState>()
        .init_resource::<Selection>()
        .init_resource::<GameLog>()
        .init_resource::<PathPreview>()
        .init_resource::<Civilizations>()
        .init_resource::<FogOfWar>()
        .init_resource::<Diplomacy>()
        .init_resource::<BarbarianState>()
        .init_resource::<TradeRoutes>()
        .init_resource::<SpaceRace>()
        .init_resource::<Foreign>()
        .init_resource::<Supplies>()
        .insert_resource(Cycle { advance: true, force: false })
        .init_resource::<WorldWonders>()
        .init_resource::<GreatPeople>()
        .init_resource::<combat::BattleTally>()
        .init_resource::<Espionage>()
        .init_resource::<History>()
        .init_resource::<CultureLedger>()
        .init_resource::<GoldenAges>()
        .init_resource::<JustCaptured>()
        .init_resource::<Rivers>()
        .add_event::<EndTurnNow>()
        .add_event::<EndTurnRequest>()
        .add_event::<Focus>()
        .add_event::<Action>()
        .add_event::<MapClick>()
        .add_event::<Burst>()
        .add_event::<BattleShown>()
        .add_event::<Detonation>()
        .init_resource::<NukeAim>()
        .init_resource::<LastMove>()
        .init_resource::<bevy_civ::ai::Ferries>()
        .add_event::<savegame::SaveRequest>();
}

/// Present when nobody plays: every real civilization is the computer's (see `autoplay`).
#[derive(Resource)]
pub struct Observer;

/// Game rules run after input is gathered and before the views are refreshed.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameLogic;

fn terrain_from(code: char) -> Terrain {
    match code {
        'g' => Terrain::Grassland,
        'p' => Terrain::Plains,
        'd' => Terrain::Desert,
        'f' => Terrain::Forest,
        'h' => Terrain::Hills,
        'm' => Terrain::Mountain,
        _ => Terrain::Water,
    }
}

/// Builds the world for the game chosen in the start menu.
#[allow(clippy::too_many_arguments)]
fn setup_world(
    mut commands: Commands,
    new_game: Res<NewGame>,
    mut state: ResMut<GameState>,
    mut tech: ResMut<TechState>,
    mut civs: ResMut<Civilizations>,
    mut fog: ResMut<FogOfWar>,
    mut diplomacy: ResMut<Diplomacy>,
    mut foreign: ResMut<Foreign>,
    mut log: ResMut<GameLog>,
    mut dirty: ResMut<Dirty>,
    mut camera: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    let (scenario, player) = if new_game.scenario.rows.is_empty() {
        scenario::generate_random(&new_game.scenario, new_game.civ, new_game.map_size, new_game.peoples)
    } else {
        (new_game.scenario.clone(), new_game.civ)
    };
    // The player's civilization always has id 0.
    let mut defs = scenario.civs.clone();
    let player = player.min(defs.len().saturating_sub(1));
    defs.swap(0, player);

    let layout = HexLayout::flat().with_hex_size(client::map::HEX_SIZE);
    let mut rng = rand::thread_rng();
    let mut map = MapData { layout, cells: HashSet::new(), water: HashSet::new(), terrain: HashMap::new(), min: Vec2::splat(f32::MAX), max: Vec2::splat(f32::MIN) };
    for (row, line) in scenario.rows.iter().enumerate() {
        for (col, code) in line.chars().enumerate() {
            let hex = scenario::offset_to_hex(col as i32, row as i32);
            let terrain = terrain_from(code);
            let pos = map.layout.hex_to_world_pos(hex);
            map.min = map.min.min(pos);
            map.max = map.max.max(pos);
            map.cells.insert(hex);
            map.terrain.insert(hex, terrain);
            if terrain == Terrain::Water {
                map.water.insert(hex);
            }
            commands.spawn(HexTile { coord: hex, terrain, improvements: vec![], resource: random_resource(&mut rng, terrain) });
        }
    }

    civs.players = defs
        .iter()
        .enumerate()
        .map(|(id, def)| bevy_civ::ai::Civ {
            id: id as u32,
            name: def.name.clone(),
            is_human: id == 0,
            gold: def.gold,
            tech: TechState::knowing(&def.techs),
            aggression: def.aggression,
            leader: def.leader.clone(),
            adjective: def.adjective.clone(),
            portrait: def.portrait,
            city_names: def.city_names.clone(),
            government: bevy_civ::government::Government::Despotism,
            anarchy: 0,
            pending_government: None,
            eliminated: false,
            rates: Default::default(),
        })
        .collect();
    *tech = TechState::knowing(&defs[0].techs);

    // Every starting city's land: the tiles within two of it that are nearer to it than to any other.
    let sites: Vec<Hex> = defs.iter().flat_map(|d| d.cities.iter().map(|c| scenario::offset_to_hex(c.col, c.row))).collect();
    for (id, def) in defs.iter().enumerate() {
        let owner = id as u32;
        for city in &def.cities {
            let hex = scenario::offset_to_hex(city.col, city.row);
            let land: Vec<Terrain> = hexx::shapes::hexagon(hex, 2)
                .filter(|h| *h != hex && sites.iter().min_by_key(|s| (s.distance_to(*h), s.x, s.y)) == Some(&hex))
                .filter_map(|h| map.terrain.get(&h).copied())
                .collect();
            let centre = bevy_civ::tiles::centre_yield(bevy_civ::tiles::tile_yield(map.terrain.get(&hex).copied().unwrap_or(Terrain::Plains), &[], None, false)).0;
            let foods = |harbour: bool| -> Vec<u32> {
                land.iter().map(|t| bevy_civ::tiles::tile_yield(*t, &[], None, false).0 + u32::from(harbour && *t == Terrain::Water)).collect()
            };
            // A port that could not feed its people has a harbour; a city that still could not starts smaller.
            let wanted = city.pop.max(1);
            let coastal = land.contains(&Terrain::Water);
            let harbour = coastal && bevy_civ::tiles::sustainable(wanted, centre, foods(false)) < wanted;
            let pop = bevy_civ::tiles::sustainable(wanted, centre, foods(harbour));
            let entity = spawn_city(&mut commands, hex, city.name.clone(), pop, owner, city.capital);
            if harbour {
                commands.entity(entity).insert(CityBuildings { buildings: vec![Building::Harbor] });
            }
            if owner == PLAYER {
                fog.reveal(hex, 3);
            }
        }
        for unit in &def.units {
            let Some(kind) = UnitType::ALL.into_iter().find(|u| u.base_name() == unit.kind) else { continue };
            let hex = scenario::offset_to_hex(unit.col, unit.row);
            spawn_unit(&mut commands, hex, kind, owner);
            if owner == PLAYER {
                fog.reveal(hex, 2);
            }
        }
    }

    let id_of = |name: &str| defs.iter().position(|d| d.name == name).map(|i| i as u32);
    for (a, b) in &scenario.wars {
        if let (Some(a), Some(b)) = (id_of(a), id_of(b)) {
            diplomacy.set_treaty(a, b, Treaty::War);
            foreign.since.insert((a.min(b), a.max(b)), 1);
        }
    }

    if scenario.known {
        foreign.met = (1..defs.len() as u32).collect();
    }

    *state = GameState {
        gold: defs[0].gold,
        start_year: scenario.start_year,
        years_per_turn: scenario.years_per_turn,
        title: defs[0].name.trim_start_matches("the ").to_string(),
        difficulty: new_game.difficulty,
        last_turn: new_game.last_turn,
        ..default()
    };

    if let Ok((mut transform, mut projection)) = camera.get_single_mut() {
        let home = defs[0].cities.first().map(|c| map.layout.hex_to_world_pos(scenario::offset_to_hex(c.col, c.row))).unwrap_or(Vec2::ZERO);
        transform.translation = home.extend(transform.translation.z);
        projection.scale = 1.0;
    }
    commands.insert_resource(Rivers::generate(&map.terrain, &mut rng));
    // Tribal villages, clear of every people's starting cities and units.
    let starts: Vec<Hex> = defs
        .iter()
        .flat_map(|d| d.cities.iter().map(|c| (c.col, c.row)).chain(d.units.iter().map(|u| (u.col, u.row))))
        .map(|(col, row)| scenario::offset_to_hex(col, row))
        .collect();
    let mut land: Vec<Hex> = map.cells.iter().copied().filter(|h| !map.water.contains(h)).collect();
    land.sort_by_key(|h| (h.x, h.y));
    commands.insert_resource(Villages { sites: bevy_civ::villages::place(&land, &starts, &mut rng) });
    commands.insert_resource(map);
    dirty.map = true;
    dirty.ui = true;
    log.turn = state.turn.max(1);
    log.push(format!("{}: you lead {} as {}.", scenario.name, defs[0].name, defs[0].leader));
}

/// The share of its trade every city loses to corruption this turn (see
/// `bevy_civ::economy::corruption`): `government` gives each people's government, `linked` the cities
/// joined to their capital by road or harbour, `land_tiles` the size of the world's land.
pub fn corruption_rates<'a>(
    cities: impl Iterator<Item = (&'a City, &'a CityBuildings)>,
    government: impl Fn(u32) -> Government,
    linked: &HashSet<Hex>,
    land_tiles: usize,
) -> HashMap<Hex, f32> {
    let list: Vec<(&City, &CityBuildings)> = cities.collect();
    let palace = |c: &City, b: &CityBuildings| c.is_capital || b.has(Building::ForbiddenPalace);
    let standings = bevy_civ::economy::standings(&list.iter().map(|(c, b)| (c.position, c.owner, palace(c, b))).collect::<Vec<_>>());
    let peoples: HashSet<u32> = list.iter().map(|(c, _)| c.owner).filter(|o| *o != BARBARIANS).collect();
    let optimal = bevy_civ::economy::optimal_cities(land_tiles, peoples.len());
    list.iter()
        .map(|(c, b)| {
            let standing = standings.get(&c.position).copied().unwrap_or_default();
            (c.position, bevy_civ::economy::corruption(government(c.owner), standing, optimal, &b.buildings, palace(c, b), linked.contains(&c.position)))
        })
        .collect()
}

/// A city's accounts this turn (see `bevy_civ::economy::ledger`): its trade, less a trade for each
/// level of a drug trade (the black market's cut) and less corruption, split by `rates` and
/// multiplied by its buildings, plus what its wonders, its people's traits and settled merchants
/// add outright (Adam Smith's Trading Company rewards markets, banks and harbours; the Internet,
/// libraries). Also returns that outright gold.
#[allow(clippy::too_many_arguments)]
pub fn city_books(
    city: &City,
    buildings: &CityBuildings,
    gov: Government,
    rates: bevy_civ::economy::Rates,
    corruption: f32,
    wonders: &WorldWonders,
    great: &GreatPeople,
    character: Option<bevy_civ::traits::Character>,
    supplies: &Supplies,
) -> (bevy_civ::economy::Ledger, u32) {
    let owner = city.owner;
    let (_, _, trait_gold, trait_science) = character.map(|c| c.city_bonus()).unwrap_or_default();
    let land_gold = supplies.yields.get(&city.position).map_or(0, |y| y.2);
    let wonder_gold = if wonders.owns(owner, Building::AdamSmith) {
        buildings.buildings.iter().filter(|b| matches!(b, Building::Marketplace | Building::Bank | Building::StockExchange | Building::Harbor)).count() as u32
    } else {
        0
    };
    let wonder_science = if wonders.owns(owner, Building::Internet) && buildings.has(Building::Library) { 3 } else { 0 };
    let outright = land_gold + trait_gold + wonder_gold + great.merchant_gold(city.position).max(0) as u32 + buildings.total_gold_bonus();
    let books = bevy_civ::economy::ledger(&bevy_civ::economy::CityTrade {
        trade: city.land.2.saturating_sub(city.narcotics),
        corruption,
        rates,
        government: gov,
        buildings: &buildings.buildings,
        scientists: city.scientists(),
        tax_collectors: city.tax_collectors(),
        extra_science: trait_science + wonder_science + buildings.total_science_bonus(),
        extra_gold: outright,
    });
    (books, outright)
}

/// Sells the building of `owner`'s that costs the most to keep up (in its smallest city, among
/// equals) to pay a debt. Returns the city's name and the building, or None if there is nothing to sell.
fn sell_for_debt(cities: &mut Query<(&mut City, &mut CityBuildings, &mut Happiness)>, owner: u32) -> Option<(String, Building)> {
    let mut best: Option<((u32, std::cmp::Reverse<u32>), Hex, Building)> = None;
    for (city, buildings, _) in cities.iter().filter(|(c, _, _)| c.owner == owner) {
        for building in &buildings.buildings {
            let upkeep = bevy_civ::economy::upkeep(*building);
            let key = (upkeep, std::cmp::Reverse(city.population));
            if upkeep > 0 && best.as_ref().is_none_or(|(k, _, _)| key > *k) {
                best = Some((key, city.position, *building));
            }
        }
    }
    let (_, at, building) = best?;
    let (city, mut buildings, _) = cities.iter_mut().find(|(c, _, _)| c.position == at)?;
    buildings.buildings.retain(|b| *b != building);
    Some((city.name.clone(), building))
}

/// Removes the finished game so a new one can start from the menu.
fn cleanup_world(
    mut commands: Commands,
    entities: Query<Entity, Or<(With<HexTile>, With<Unit>, With<City>, With<client::map::Decor>, With<client::map::PathDot>, With<client::actors::BurstFx>, With<client::actors::StackBadge>)>>,
    parents: Query<&Parent>,
) {
    for entity in &entities {
        // Children (flags, labels, bases) go with their parents.
        if parents.get(entity).is_err() {
            commands.entity(entity).try_despawn_recursive();
        }
    }
    commands.insert_resource(GameState::default());
    commands.insert_resource(TechState::default());
    commands.insert_resource(Selection::default());
    commands.insert_resource(GameLog::default());
    commands.insert_resource(PathPreview::default());
    commands.insert_resource(Civilizations::default());
    commands.insert_resource(FogOfWar::default());
    commands.insert_resource(Diplomacy::default());
    commands.insert_resource(BarbarianState::default());
    commands.insert_resource(TradeRoutes::default());
    commands.insert_resource(SpaceRace::default());
    commands.insert_resource(Foreign::default());
    commands.insert_resource(Supplies::default());
    commands.insert_resource(WorldWonders::default());
    commands.insert_resource(GreatPeople::default());
    commands.insert_resource(Religions::default());
    commands.insert_resource(Espionage::default());
    commands.insert_resource(History::default());
    commands.insert_resource(CultureLedger::default());
    commands.insert_resource(GoldenAges::default());
    commands.insert_resource(JustCaptured::default());
    commands.insert_resource(Rivers::default());
    commands.insert_resource(Cycle { advance: true, force: false });
    commands.insert_resource(Dirty { map: true, ui: true });
}

/// A unit with [`bevy_civ::unit::Ability::Plunder`] that wins brings gold home to its people.
fn plunder(state: &mut GameState, civs: &mut Civilizations, log: &mut GameLog, owner: u32, kind: UnitType) {
    if kind.ability() != Some(bevy_civ::unit::Ability::Plunder) {
        return;
    }
    if owner == PLAYER {
        state.gold += bevy_civ::unit::PLUNDER_GOLD;
        log.push(format!("Our {} brings back {} gold in plunder.", kind.name(), bevy_civ::unit::PLUNDER_GOLD));
    } else if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
        civ.gold += bevy_civ::unit::PLUNDER_GOLD;
    }
}

fn handle_map_click(
    mut clicks: EventReader<MapClick>,
    mut selection: ResMut<Selection>,
    mut units: Query<(Entity, &mut Unit)>,
    mut cities: Query<(Entity, &mut City)>,
    (tiles, walls_of, rivers, civs, supplies): (Query<&HexTile>, Query<&CityBuildings>, Res<Rivers>, Res<Civilizations>, Res<Supplies>),
    state: Res<GameState>,
    map: Res<MapData>,
    mut fog: ResMut<FogOfWar>,
    mut diplomacy: ResMut<Diplomacy>,
    mut log: ResMut<GameLog>,
    mut dirty: ResMut<Dirty>,
    (mut cycle, mut wonders, mut great, mut taken): (ResMut<Cycle>, ResMut<WorldWonders>, ResMut<GreatPeople>, ResMut<JustCaptured>),
    (mut sounds, mut battles, mut strikes, mut aim, mut last_move): (EventWriter<Sfx>, EventWriter<BattleShown>, EventWriter<Detonation>, ResMut<NukeAim>, ResMut<LastMove>),
    mut orders: EventWriter<Action>,
    mut commands: Commands,
) {
    for click in clicks.read() {
        if state.victory.is_some() {
            continue;
        }
        dirty.ui = true;
        let hex = click.hex;
        let Some(terrain) = tiles.iter().find(|t| t.coord == hex).map(|t| t.terrain) else {
            if !click.secondary {
                selection.unit = None;
                selection.city = None;
            }
            continue;
        };
        // A selected nuclear missile: the first click aims, a second click on the same tile launches.
        if let Some(missile) = selection.unit.filter(|e| units.get(*e).is_ok_and(|(_, u)| u.owner == PLAYER && u.unit_type == UnitType::NuclearMissile)) {
            let from = units.get(missile).map(|(_, u)| u.position).unwrap_or(hex);
            if from.distance_to(hex) > MISSILE_RANGE {
                log.push(format!("That is beyond the missile's range ({MISSILE_RANGE} tiles)."));
                continue;
            }
            if aim.0 == Some(hex) {
                aim.0 = None;
                commands.entity(missile).try_despawn_recursive();
                selection.unit = None;
                strikes.send(Detonation { at: hex, by: PLAYER });
            } else {
                aim.0 = Some(hex);
                log.push("Target set. Click the same tile again to launch the missile - everything within a tile of it will be destroyed. (Esc to stand down.)");
            }
            continue;
        }
        aim.0 = None;
        // With one of our cities open, a click on a tile of its area moves a citizen there (or off it).
        if let Some(city_entity) = selection.city.filter(|_| selection.unit.is_none() && !click.secondary) {
            let own_units_here = units.iter().any(|(_, u)| u.position == hex && u.owner == PLAYER);
            if let Ok((_, mut city)) = cities.get_mut(city_entity) {
                let in_area = supplies.territory.get(&hex).is_some_and(|(o, home)| *o == PLAYER && *home == city.position);
                if city.owner == PLAYER && in_area && hex != city.position && !own_units_here {
                    let value = |h: Hex| tiles.iter().find(|t| t.coord == h).map(|t| {
                        let y = bevy_civ::tiles::tile_yield(t.terrain, &t.improvements, None, rivers.tiles.contains(&h));
                        y.0 * 2 + y.1 * 2 + y.2
                    }).unwrap_or(0);
                    if let Some(message) = toggle_worked(&mut city, hex, value) {
                        log.push(message);
                    }
                    sounds.send(Sfx::Select);
                    continue;
                }
            }
        }

        let snapshot: Vec<(Entity, Hex, u32, UnitType)> =
            units.iter().map(|(e, u)| (e, u.position, u.owner, u.unit_type)).collect();
        let berths: Vec<Berth> = units.iter().map(|(e, u)| Berth::of(e, &u)).collect();
        // Hit points, level and whether dug in, for whoever has to defend.
        let vitals: HashMap<Entity, (u32, u8, bool)> = units.iter().map(|(e, u)| (e, (u.hp, u.level, u.fortified))).collect();
        let members_of: HashMap<Entity, u32> = units.iter().map(|(e, u)| (e, u.members())).collect();
        // Enemy fighters that have not flown this turn, ready to intercept our bombers.
        let fighters_ready: Vec<(Entity, Hex, u32)> = units
            .iter()
            .filter(|(_, u)| u.unit_type == UnitType::Fighter && u.owner != PLAYER && u.movement_left > 0)
            .map(|(e, u)| (e, u.position, u.owner))
            .collect();
        let city_here = cities.iter().find(|(_, c)| c.position == hex).map(|(e, c)| (e, c.owner));
        let own_here: Vec<Entity> =
            snapshot.iter().filter(|u| u.1 == hex && u.2 == PLAYER).map(|u| u.0).collect();
        let commanding = selection
            .unit
            .and_then(|e| snapshot.iter().find(|u| u.0 == e && u.2 == PLAYER))
            .copied();

        let own_city = matches!(city_here, Some((_, PLAYER)));
        let wants_move = commanding.is_some_and(|u| u.1 != hex)
            && (click.secondary || (own_here.is_empty() && !own_city));

        if !wants_move {
            if click.secondary {
                continue;
            }
            // Select: repeated clicks cycle through the city and the player's units on the tile.
            #[derive(PartialEq, Clone, Copy)]
            enum Pick {
                City(Entity),
                Unit(Entity),
            }
            let mut picks: Vec<Pick> = Vec::new();
            if let Some((entity, _)) = city_here.filter(|_| fog.explored.contains(&hex)) {
                picks.push(Pick::City(entity));
            }
            picks.extend(own_here.iter().map(|e| Pick::Unit(*e)));
            if picks.is_empty() && fog.visible.contains(&hex) {
                picks.extend(snapshot.iter().filter(|u| u.1 == hex).take(1).map(|u| Pick::Unit(u.0)));
            }
            let current = selection.city.map(Pick::City).or(selection.unit.map(Pick::Unit));
            // Coming to a stack afresh, a unit that still needs orders is picked first (a city first
            // on its own tile).
            let ready = |p: &Pick| matches!(p, Pick::Unit(e) if units.get(*e).is_ok_and(|(_, u)| needs_orders(u)));
            let fresh = if matches!(picks.first(), Some(Pick::City(_))) { 0 } else { picks.iter().position(ready).unwrap_or(0) };
            let index = current.and_then(|c| picks.iter().position(|p| *p == c)).map(|i| i + 1).unwrap_or(fresh);
            match picks.get(index % picks.len().max(1)) {
                Some(Pick::City(entity)) => {
                    selection.city = Some(*entity);
                    selection.unit = None;
                    sounds.send(Sfx::Select);
                }
                Some(Pick::Unit(entity)) => {
                    selection.unit = Some(*entity);
                    selection.city = None;
                    sounds.send(Sfx::Select);
                }
                None => {
                    selection.unit = None;
                    selection.city = None;
                }
            }
            continue;
        }

        // --- Move / attack order -------------------------------------------------------------
        let (selected, ..) = commanding.unwrap();
        let Ok((_, mut unit)) = units.get_mut(selected) else { continue };
        let unit = &mut *unit;
        // Remember how the unit stood, so a plain move can be taken back (not an attack, a group
        // move or a capture: see below).
        last_move.0 = Some((selected, unit.clone()));
        if unit.build_turns_left.is_some() {
            log.push(format!("{} is busy building.", unit.unit_type.name()));
            continue;
        }
        if !unit.can_move() {
            log.push(format!("{} has no moves left this turn.", unit.unit_type.name()));
            continue;
        }
        let domain = unit.unit_type.domain();
        // The strongest defender on the tile meets an attack.
        // Units of a people who grant us passage are not in the way: we share the tile.
        let enemy = snapshot
            .iter()
            .filter(|u| u.1 == hex && u.2 != unit.owner && (u.2 == BARBARIANS || !diplomacy.has_passage(PLAYER, u.2)))
            .max_by_key(|u| defending_as(u.3, u.1, &map).stats().1 * 10 + vitals.get(&u.0).map_or(0, |v| v.0))
            .copied();
        // Walking into the units of a people at peace with us would be war: that takes a declaration.
        if let Some((_, _, owner, _)) = enemy {
            if owner != BARBARIANS && matches!(diplomacy.get_treaty(PLAYER, owner), Treaty::Peace | Treaty::Alliance) {
                log.push("We are at peace with them: attacking means war, so declare it first (Diplomacy), or ask for a right of passage to share their tiles.");
                continue;
            }
        }
        // Nor may we walk into the city of a people we are not at war with.
        if enemy.is_none() && city_here.is_some_and(|(_, o)| o != PLAYER && o != BARBARIANS && diplomacy.get_treaty(PLAYER, o) != Treaty::War) {
            log.push("That is the city of a people we are not at war with: a right of passage lets us through their lands, but not into their cities.");
            continue;
        }
        let ports: HashSet<Hex> = cities.iter().filter(|(_, c)| c.owner == PLAYER).map(|(_, c)| c.position).collect();
        let airfields = cargo::airfields(ports.iter().copied(), &berths, PLAYER);
        let boarding = domain == Domain::Land && enemy.is_none() && map.water.contains(&hex);
        // Where each kind of unit may go.
        let refusal = match domain {
            Domain::Sea if enemy.is_none() && !map.water.contains(&hex) && !ports.contains(&hex) => Some("Ships can only enter water and your own coastal cities."),
            Domain::Air if !airfields.iter().any(|p| p.distance_to(hex) <= AIR_RANGE) => Some("That is beyond the range of our airfields (our cities and carriers)."),
            Domain::Land if boarding && cargo::ship_with_room(&berths, PLAYER, hex, unit.unit_type).is_none() => {
                Some("Troops cross the sea aboard ships: move them onto a ship of ours with room (a Galley, Galleon, Frigate or Transport), or board one in port.")
            }
            Domain::Land if enemy.is_some() && map.water.contains(&unit.position) => Some("Troops cannot attack from a ship; land them first."),
            Domain::Land if enemy.is_some() && map.water.contains(&hex) => Some("Land units cannot attack ships at sea."),
            Domain::Land if enemy.is_none() && unit.unit_type.bombard_only() && city_here.is_some_and(|(_, o)| o != PLAYER) => Some("Guns cannot take a city: send in troops (guns bombard its defenders from range)."),
            _ => None,
        };
        if let Some(reason) = refusal {
            log.push(reason);
            continue;
        }
        let travel = player_travel(unit.unit_type, &ports);
        let roads = road_network(tiles.iter(), cities.iter().map(|(_, c)| c.position));
        let Some(route) = find_route(unit.position, hex, tiles.iter().map(|t| (t.coord, t.terrain)), &roads, travel) else {
            log.push(if domain != Domain::Land { "There is no route to that tile." } else { "There is no route over land to that tile: troops cross the sea aboard ships." });
            continue;
        };
        dirty.map = true;
        unit.fortified = false;
        cycle.advance = true;

        // Stop next to a defended tile; the last step is the attack itself. A gun stops as soon as
        // its target is within range.
        let gun_range = unit.unit_type.bombard().filter(|_| unit.unit_type.bombard_only()).map(|(_, range, _)| range);
        let approach = match (enemy.is_some(), gun_range) {
            (true, Some(range)) => &route[..=route.iter().position(|h| h.distance_to(hex) <= range).unwrap_or(route.len().saturating_sub(2))],
            (true, None) => &route[..route.len() - 1],
            (false, _) => &route[..],
        };
        let mut trail = Vec::new();
        let enemies: HashSet<Hex> = snapshot
            .iter()
            .filter(|u| u.2 != PLAYER && u.3.is_army() && (u.2 == BARBARIANS || diplomacy.get_treaty(PLAYER, u.2) == Treaty::War))
            .map(|u| u.1)
            .collect();
        walk(unit, approach, &map, &roads, &enemies, &mut fog, &mut trail);
        if !trail.is_empty() {
            sounds.send(Sfx::moving(domain));
        }
        // Too far for one turn: the unit carries on next turn. A new order replaces the old one.
        let short = trail.last().copied().unwrap_or(unit.position) != *approach.last().unwrap_or(&hex);
        if short {
            commands.entity(selected).insert((GoTo(hex), Trail(trail))).remove::<Exploring>();
        } else {
            commands.entity(selected).insert(Trail(trail)).remove::<GoTo>().remove::<Exploring>();
        }

        if enemy.is_some() || click.group {
            last_move.0 = None;
        }
        if let Some((enemy_entity, _, enemy_owner, enemy_type)) = enemy {
            let adjacent = unit.position.distance_to(hex) == 1;
            if let Some(range) = gun_range {
                // A gun does not attack: it bombards, once the target is within range.
                if unit.position.distance_to(hex) <= range {
                    if unit.can_move() {
                        orders.send(Action::BombardAt(hex));
                    } else {
                        log.push(format!("Our {} is in range but has no moves left: it can bombard next turn.", unit.unit_type.name()));
                    }
                }
            } else if adjacent && unit.can_move() && unit.unit_type.is_military() {
                unit.spend(StepCost::Full);
                // A bomber may be met by an enemy fighter within three tiles of the target.
                if unit.unit_type == UnitType::Bomber {
                    let hostile = |o: u32| o == BARBARIANS || o == enemy_owner || diplomacy.get_treaty(PLAYER, o) == Treaty::War;
                    if let Some((fighter, _, _)) = fighters_ready.iter().find(|(_, at, o)| hostile(*o) && at.distance_to(hex) <= 3) {
                        let fighter = *fighter;
                        commands.queue(move |world: &mut World| {
                            if let Some(mut f) = world.get_mut::<Unit>(fighter) {
                                f.end_move();
                            }
                        });
                        if rand::thread_rng().gen_bool(UnitType::interception_odds(UnitType::Fighter, UnitType::Bomber)) {
                            log.push("Enemy fighters intercept our bomber and shoot it down!");
                            commands.entity(selected).try_despawn_recursive();
                            selection.unit = None;
                            last_move.0 = None;
                            continue;
                        }
                        unit.hp = unit.hp.saturating_sub(1).max(1);
                        log.push("Enemy fighters intercept our bomber; damaged, it presses on.");
                    }
                }
                if enemy_owner != BARBARIANS && diplomacy.get_treaty(PLAYER, enemy_owner) != Treaty::War {
                    diplomacy.set_treaty(PLAYER, enemy_owner, Treaty::War);
                    log.push("Your attack means war!");
                }
                let enemy_type = defending_as(enemy_type, hex, &map);
                let led = |at: Hex, owner: u32| snapshot.iter().any(|u| u.1 == at && u.2 == owner && u.3 == UnitType::GreatGeneral);
                let (d_hp, d_level, dug_in) = vitals.get(&enemy_entity).copied().unwrap_or((3, 0, false));
                let walls = city_here.is_some_and(|(e, owner)| walls_of.get(e).is_ok_and(|b| b.has(Building::Walls)) || wonders.walls_everywhere(owner));
                let coast_fort = city_here.is_some_and(|(e, _)| walls_of.get(e).is_ok_and(|b| b.has(Building::CoastalFortress)));
                let population = city_here.and_then(|(e, _)| cities.get(e).ok().map(|(_, c)| c.population));
                let fortress = tiles.iter().any(|t| t.coord == hex && t.improvements.contains(&Improvement::Fortress));
                let outcome = combat::fight(
                    Side { kind: unit.unit_type, hp: unit.hp, level: unit.level, led: led(unit.position, PLAYER), members: unit.members() },
                    Side { kind: enemy_type, hp: d_hp, level: d_level, led: led(hex, enemy_owner), members: members_of.get(&enemy_entity).copied().unwrap_or(1) },
                    Ground { terrain, fortified: dug_in, walls, river: rivers.tiles.contains(&hex), coast_fort, city: population, fortress },
                    &mut rand::thread_rng(),
                );
                // The defender's wounds (it is another entity, so the change is queued).
                let wound = |commands: &mut Commands, hp: u32, promote: bool| {
                    commands.queue(move |world: &mut World| {
                        if let Some(mut defender) = world.get_mut::<Unit>(enemy_entity) {
                            defender.hp = hp.max(1);
                            if promote {
                                defender.promote();
                            }
                        }
                    });
                };
                // The winner may be promoted (more often in a militaristic civilization, less for
                // beating barbarians).
                let character = |owner: u32| civs.players.iter().find(|c| c.id == owner).map(|c| bevy_civ::traits::of(&c.name));
                let chance = |owner: u32, level: u8, beat: u32| {
                    let barbarian = beat == BARBARIANS;
                    character(owner).map_or(combat::promotion_chance(level, false, barbarian), |c| c.promotion_chance(level, barbarian))
                };
                let won_chance = if matches!(outcome, Outcome::Won { .. }) { chance(PLAYER, unit.level, enemy_owner) } else { chance(enemy_owner, d_level, PLAYER) };
                let promoted = rand::thread_rng().gen_bool(won_chance);
                let (result, good) = match outcome {
                    Outcome::Won { attacker_hp } => (format!("Victory! {attacker_hp}/{} HP", unit.max_hp()), true),
                    Outcome::Retreated { .. } => ("Pulled back".to_string(), false),
                    Outcome::Lost { .. } => (format!("Our {} is lost", unit.unit_type.name()), false),
                };
                battles.send(BattleShown {
                    from: unit.position,
                    at: hex,
                    attacker: Some(selected),
                    result,
                    good: Some(good),
                    ours: true,
                    attacker_side: (unit.unit_type, PLAYER),
                    defender_side: (enemy_type, enemy_owner),
                    attacker_died: matches!(outcome, Outcome::Lost { .. }),
                    defender_died: matches!(outcome, Outcome::Won { .. }),
                });
                let won = match outcome {
                    Outcome::Won { attacker_hp } => {
                        unit.hp = attacker_hp;
                        true
                    }
                    Outcome::Retreated { defender_hp } => {
                        unit.hp = 1;
                        wound(&mut commands, defender_hp, false);
                        log.push(format!("Your {} pulls back, badly hurt, from the enemy {}.", unit.unit_type.name(), enemy_type.name()));
                        continue;
                    }
                    Outcome::Lost { defender_hp } => {
                        wound(&mut commands, defender_hp, promoted);
                        false
                    }
                };
                if won {
                    let fought_as = unit.level;
                    let rank = if promoted && unit.promote() { format!(" It is promoted to {}.", unit.level_name()) } else { String::new() };
                    log.push(format!("Victory! Your {} defeated the enemy {} ({} of {} hit points left).{rank}", unit.unit_type.name(), enemy_type.name(), unit.hp, unit.max_hp()));
                    // A cartel's assets are seized.
                    if enemy_type == UnitType::Cartel {
                        commands.queue(|world: &mut World| world.resource_mut::<GameState>().gold += bevy_civ::barbarians::SEIZED_GOLD);
                        log.push(format!("We seize the cartel's assets: {} gold.", bevy_civ::barbarians::SEIZED_GOLD));
                    }
                    // A raider's victory brings back plunder.
                    if unit.unit_type.ability() == Some(bevy_civ::unit::Ability::Plunder) {
                        commands.queue(|world: &mut World| world.resource_mut::<GameState>().gold += bevy_civ::unit::PLUNDER_GOLD);
                        log.push(format!("Our {} brings back {} gold in plunder.", unit.unit_type.name(), bevy_civ::unit::PLUNDER_GOLD));
                    }
                    if great.victory(PLAYER, fought_as, &mut rand::thread_rng()) {
                        spawn_unit(&mut commands, unit.position, UnitType::GreatGeneral, PLAYER);
                        log.push(format!("The victory of our elite {} brings forth a Great General! Keep it with your troops (they fight half again as hard), or form an army in one of our cities.", unit.unit_type.name()));
                    }
                    // Our unique unit's first victory over another people begins our golden age.
                    let kind = unit.unit_type;
                    commands.queue(move |world: &mut World| world.resource_scope(|world, mut golden: Mut<GoldenAges>| {
                        world.resource_scope(|world, mut log: Mut<GameLog>| unique_victory(&mut golden, world.resource::<Civilizations>(), &world.resource::<foreign::Foreign>().met, &mut log, PLAYER, kind, enemy_owner));
                    }));
                    commands.entity(enemy_entity).try_despawn_recursive();
                    let more_defenders = snapshot.iter().any(|u| u.1 == hex && u.2 != PLAYER && u.0 != enemy_entity);
                    if !more_defenders && may_enter(unit.unit_type, hex, &map, &ports) {
                        unit.position = hex;
                    }
                } else {
                    log.push(format!("Defeat. Your {} was lost attacking a {}.", unit.unit_type.name(), enemy_type.name()));
                    commands.queue(move |world: &mut World| world.resource_scope(|world, mut golden: Mut<GoldenAges>| {
                        world.resource_scope(|world, mut log: Mut<GameLog>| unique_victory(&mut golden, world.resource::<Civilizations>(), &world.resource::<foreign::Foreign>().met, &mut log, enemy_owner, enemy_type, PLAYER));
                    }));
                    commands.entity(selected).try_despawn_recursive();
                    selection.unit = None;
                    continue;
                }
            } else if !unit.unit_type.is_military() {
                log.push(format!("A {} cannot attack.", unit.unit_type.name()));
            }
        }

        fog.reveal(unit.position, 2);
        let captured_before = taken.0.len();
        capture(unit, cities.iter_mut().map(|(_, c)| c), &mut wonders, &mut taken, &mut log);
        if taken.0.len() != captured_before {
            last_move.0 = None;
        }

        // The rest of the stack follows when ordered together.
        if click.group {
            let from = snapshot.iter().find(|u| u.0 == selected).map(|u| u.1);
            let followers: Vec<Entity> = snapshot.iter().filter(|u| Some(u.1) == from && u.0 != selected && u.2 == PLAYER && u.3.domain() == domain).map(|u| u.0).collect();
            let enemies: HashSet<Hex> = snapshot
                .iter()
                .filter(|u| u.2 != PLAYER && u.3.is_army() && (u.2 == BARBARIANS || diplomacy.get_treaty(PLAYER, u.2) == Treaty::War))
                .map(|u| u.1)
                .collect();
            // Boarding together: only as many as the ships there have room for.
            let load = cargo::loads(&berths);
            let mut seats: usize = if boarding {
                berths
                    .iter()
                    .filter(|s| s.owner == PLAYER && s.position == hex && s.unit_type.carries(UnitType::Warrior))
                    .map(|s| s.unit_type.capacity().saturating_sub(load.get(&s.entity).copied().unwrap_or(0)) as usize)
                    .sum::<usize>()
                    .saturating_sub(1)
            } else {
                usize::MAX
            };
            for follower in followers {
                let Ok((_, mut unit)) = units.get_mut(follower) else { continue };
                if unit.build_turns_left.is_some() || !unit.can_move() || seats == 0 {
                    continue;
                }
                seats = seats.saturating_sub(usize::from(boarding));
                let travel = player_travel(unit.unit_type, &ports);
                let Some(route) = find_route(unit.position, hex, tiles.iter().map(|t| (t.coord, t.terrain)), &roads, travel) else { continue };
                let approach = if enemy.is_some() { &route[..route.len() - 1] } else { &route[..] };
                let mut trail = Vec::new();
                unit.fortified = false;
                walk(&mut unit, approach, &map, &roads, &enemies, &mut fog, &mut trail);
                let short = unit.position != *approach.last().unwrap_or(&hex);
                if short {
                    commands.entity(follower).insert((GoTo(hex), Trail(trail)));
                } else {
                    commands.entity(follower).insert(Trail(trail)).remove::<GoTo>();
                }
            }
        }
    }
}

/// An army standing in a foreign city takes it.
fn capture<'a>(unit: &Unit, cities: impl Iterator<Item = Mut<'a, City>>, wonders: &mut WorldWonders, taken: &mut JustCaptured, log: &mut GameLog) {
    if !unit.unit_type.is_army() || unit.unit_type.bombard_only() {
        return;
    }
    for mut city in cities {
        if city.position == unit.position && city.owner != PLAYER {
            // Whoever loses it has a grievance against us (settled in the turn, by `process_turn`).
            taken.1.push(city.owner);
            city.owner = PLAYER;
            city.is_capital = false;
            city.resistance = bevy_civ::city::resistance_turns(city.population);
            wonders.transfer_city(&city.name, PLAYER);
            taken.0.insert(city.position);
            city.production_queue = None;
            log.push(format!("Your {} captured {}! Keep it, or raze it from the city panel this turn.", unit.unit_type.name(), city.name));
        }
    }
}

/// The player's units on a tile, in the fixed order that Tab and the unit panel use.
pub fn own_stack(at: Hex, entities: impl Iterator<Item = Entity>, place: impl Fn(Entity) -> Option<(Hex, u32)>) -> Vec<Entity> {
    let mut stack: Vec<Entity> = entities.filter(|e| place(*e) == Some((at, PLAYER))).collect();
    stack.sort_by_key(|e| e.index());
    stack
}

/// The foreign (not barbarian) city a diplomat at `here` can work on: the one it stands in, or the
/// nearest one beside it. Returns its tile and owner.
pub fn spy_target(here: Hex, cities: impl Iterator<Item = (Hex, u32)>) -> Option<(Hex, u32)> {
    cities.filter(|(at, owner)| *owner != PLAYER && *owner != BARBARIANS && at.distance_to(here) <= 1).min_by_key(|(at, _)| at.distance_to(here))
}

/// How the player's units of a kind travel, given where the ports are. Troops cross the sea only
/// aboard ships, so their routes keep to the land (a route may end on a ship's tile, to board it).
fn player_travel(unit: UnitType, ports: &HashSet<Hex>) -> Travel<'_> {
    match unit.domain() {
        Domain::Sea => Travel::Sea(ports),
        Domain::Air => Travel::Air,
        Domain::Land => bevy_civ::pathfinding::land_travel(unit.is_wheeled(), false),
    }
}

/// Boarding a ship or going ashore takes the rest of the turn (ships and aircraft are unaffected).
fn ends_move(domain: Domain, from: Hex, to: Hex, map: &MapData) -> bool {
    domain == Domain::Land && map.water.contains(&from) != map.water.contains(&to)
}

/// What a step costs: rough ground slows troops on foot, roads and railroads speed them; boarding a
/// ship or going ashore ends the turn.
fn move_cost(domain: Domain, from: Hex, to: Hex, map: &MapData, roads: &HashMap<Hex, Road>) -> StepCost {
    if ends_move(domain, from, to, map) {
        StepCost::EndsTurn
    } else {
        land_step(from, to, map.terrain.get(&to).copied(), roads, domain == Domain::Land)
    }
}

/// Steps through our cities ([ and ], or the arrows in the city panel), opening each and looking at it.
fn cycle_cities(mut actions: EventReader<Action>, mut selection: ResMut<Selection>, cities: Query<(Entity, &City)>, mut focus: EventWriter<Focus>, mut dirty: ResMut<Dirty>) {
    for action in actions.read() {
        let Action::CycleCity(forward) = action else { continue };
        let mut ours: Vec<(Entity, Hex)> = cities.iter().filter(|(_, c)| c.owner == PLAYER).map(|(e, c)| (e, c.position)).collect();
        if ours.is_empty() {
            continue;
        }
        ours.sort_by_key(|(e, _)| e.index());
        let len = ours.len();
        let next = match selection.city.and_then(|s| ours.iter().position(|(e, _)| *e == s)) {
            Some(i) if *forward => (i + 1) % len,
            Some(i) => (i + len - 1) % len,
            None if *forward => 0,
            None => len - 1,
        };
        let (entity, at) = ours[next];
        selection.city = Some(entity);
        selection.unit = None;
        focus.send(Focus(at));
        dirty.ui = true;
    }
}

/// Carries out agreed withdrawals: the troops of one people standing in another's lands march back
/// to their nearest city (ships to their nearest port).
fn withdraw_troops(mut foreign: ResMut<Foreign>, mut units: Query<&mut Unit>, cities: Query<&City>, supplies: Res<Supplies>, map: Res<MapData>, mut log: ResMut<GameLog>) {
    if foreign.withdrawals.is_empty() {
        return;
    }
    for (whose, lands) in std::mem::take(&mut foreign.withdrawals) {
        let homes: Vec<(Hex, bool)> = cities
            .iter()
            .filter(|c| c.owner == whose)
            .map(|c| (c.position, c.position.all_neighbors().iter().any(|n| map.water.contains(n))))
            .collect();
        let mut moved = 0;
        for mut unit in units.iter_mut() {
            if unit.owner != whose || !unit.unit_type.is_military() || unit.carrier.is_some() {
                continue;
            }
            if supplies.territory.get(&unit.position).map(|(o, _)| *o) != Some(lands) {
                continue;
            }
            let sea = unit.unit_type.domain() == Domain::Sea;
            let here = unit.position;
            let Some((home, _)) = homes.iter().filter(|(_, port)| !sea || *port).min_by_key(|(p, _)| (p.distance_to(here), p.x, p.y)) else { continue };
            unit.position = *home;
            unit.fortified = false;
            unit.end_move();
            moved += 1;
        }
        if whose == PLAYER && moved > 0 {
            log.push(format!("{moved} of our units have returned to our cities."));
        }
        if lands == PLAYER {
            foreign.trespass.remove(&whose);
        } else if whose == PLAYER {
            foreign.our_trespass.remove(&lands);
        }
    }
}

/// Keeps carried units with their ships: ships that sailed take their cargo along, units that left
/// the ship's tile by themselves are ashore, units that stepped onto a ship go aboard, and troops
/// whose ship went down are lost (see `bevy_civ::cargo`).
fn stow_cargo(mut units: Query<(Entity, &mut Unit)>, map: Res<MapData>, mut log: ResMut<GameLog>, mut commands: Commands) {
    let berths: Vec<Berth> = units.iter().map(|(e, u)| Berth::of(e, &u)).collect();
    for change in cargo::stow(&berths, &map.water) {
        match change {
            cargo::Change::Lost { entity } => {
                if let Ok((_, unit)) = units.get(entity) {
                    if unit.owner == PLAYER {
                        log.push(format!("Our {} went down with its ship.", unit.unit_type.name()));
                    }
                }
                commands.entity(entity).try_despawn_recursive();
            }
            cargo::Change::Carried { entity, .. } | cargo::Change::Ashore { entity } | cargo::Change::Aboard { entity, .. } => {
                if let Ok((_, mut unit)) = units.get_mut(entity) {
                    cargo::apply(&mut unit, change, &map.water);
                }
            }
        }
    }
}

/// A unit's moves for a turn: its own speed, plus what the Lighthouse and Magellan's add to ships.
/// (Troops aboard a ship keep their own moves, for going ashore.)
fn moves_per_turn(unit_type: UnitType, _at: Hex, _map: &MapData, sea_bonus: u32) -> u32 {
    unit_type.stats().2 + if unit_type.domain() == Domain::Sea { sea_bonus } else { 0 }
}

/// Moves a unit along `route` (which starts where it stands) while it has moves left, revealing the
/// land around each tile entered and adding it to `trail`.
fn walk(unit: &mut Unit, route: &[Hex], map: &MapData, roads: &HashMap<Hex, Road>, enemies: &HashSet<Hex>, fog: &mut FogOfWar, trail: &mut Vec<Hex>) {
    for next in route.iter().skip(1) {
        if !unit.can_move() || (unit.unit_type.is_army() && unit.unit_type.ability() != Some(bevy_civ::unit::Ability::Outrider) && bevy_civ::pathfinding::zoc_blocks(unit.position, *next, enemies)) {
            break;
        }
        let cost = move_cost(unit.unit_type.domain(), unit.position, *next, map, roads);
        unit.step(*next, cost);
        if unit.owner == PLAYER {
            fog.reveal(*next, 2);
        }
        trail.push(*next);
    }
}

/// At the start of the player's turn, units with a standing order march on towards it. They stop
/// next to anything foreign in their way (attacking stays the player's decision) and drop the order
/// once they arrive or can no longer get there.
#[allow(clippy::too_many_arguments)]
fn continue_orders(
    state: Res<GameState>,
    mut last_turn: Local<u32>,
    mut units: Query<(Entity, &mut Unit, Option<&GoTo>)>,
    mut cities: Query<&mut City>,
    tiles: Query<&HexTile>,
    (map, diplomacy): (Res<MapData>, Res<Diplomacy>),
    mut fog: ResMut<FogOfWar>,
    (mut log, mut wonders, mut taken): (ResMut<GameLog>, ResMut<WorldWonders>, ResMut<JustCaptured>),
    mut dirty: ResMut<Dirty>,
    mut commands: Commands,
) {
    if std::mem::replace(&mut *last_turn, state.turn) == state.turn || state.victory.is_some() {
        return;
    }
    let foreign: HashSet<Hex> = units.iter().filter(|(_, u, _)| u.owner != PLAYER).map(|(_, u, _)| u.position).collect();
    let enemy_armies: HashSet<Hex> = units
        .iter()
        .filter(|(_, u, _)| u.owner != PLAYER && u.unit_type.is_army() && (u.owner == BARBARIANS || diplomacy.get_treaty(PLAYER, u.owner) == Treaty::War))
        .map(|(_, u, _)| u.position)
        .collect();
    let ports: HashSet<Hex> = cities.iter().filter(|c| c.owner == PLAYER).map(|c| c.position).collect();
    let land: Vec<(Hex, Terrain)> = tiles.iter().map(|t| (t.coord, t.terrain)).collect();
    let roads = road_network(tiles.iter(), cities.iter().map(|c| c.position));
    let berths: Vec<Berth> = units.iter().map(|(e, u, _)| Berth::of(e, &u)).collect();
    let mut arrived = Vec::new();
    for (entity, mut unit, order) in &mut units {
        let Some(&GoTo(target)) = order else { continue };
        if unit.owner != PLAYER || unit.build_turns_left.is_some() || unit.fortified || !unit.can_move() {
            continue;
        }
        let name = unit.unit_type.name();
        // Troops bound for a ship: it must still be there, with room.
        if unit.unit_type.domain() == Domain::Land && map.water.contains(&target) && cargo::ship_with_room(&berths, PLAYER, target, unit.unit_type).is_none() {
            log.push(format!("Your {name} stops: the ship it was to board has gone or is full."));
            commands.entity(entity).remove::<GoTo>();
            continue;
        }
        let travel = player_travel(unit.unit_type, &ports);
        let Some(route) = find_route(unit.position, target, land.iter().copied(), &roads, travel) else {
            log.push(format!("Your {name} can no longer reach its destination."));
            commands.entity(entity).remove::<GoTo>();
            continue;
        };
        // Walk up to (not into) the first tile held by someone else.
        let clear = route.iter().skip(1).position(|h| foreign.contains(h)).map_or(route.len(), |i| i + 1);
        let mut trail = Vec::new();
        walk(&mut unit, &route[..clear], &map, &roads, &enemy_armies, &mut fog, &mut trail);
        capture(&unit, cities.iter_mut(), &mut wonders, &mut taken, &mut log);
        let blocked = clear < route.len() && unit.position == route[clear - 1];
        if unit.position == target || blocked {
            if blocked {
                log.push(if route[clear] == target {
                    format!("Your {name} has reached the enemy and awaits orders.")
                } else {
                    format!("Your {name} halts: the way ahead is held.")
                });
            }
            arrived.push(entity);
            commands.entity(entity).remove::<GoTo>();
        }
        commands.entity(entity).try_insert(Trail(trail));
        dirty.map = true;
    }
    if !arrived.is_empty() {
        dirty.ui = true;
    }
}

/// Who can be sent exploring.
pub fn can_explore(unit: &Unit) -> bool {
    let kind = unit.unit_type;
    (kind == UnitType::Scout || (kind.is_military() && kind.domain() != Domain::Air)) && !unit.is_army_group() && unit.carrier.is_none()
}

/// Whether exploring units take another look at the map this turn: when a turn begins, or when one
/// has just been sent.
///
/// Each explorer heads for the nearest known tile that borders the unknown and it can reach,
/// walking as far as it can; with moves to spare it picks another. It stops exploring when there is
/// nothing left it can reach.
#[allow(clippy::too_many_arguments)]
fn auto_explore(
    state: Res<GameState>,
    mut last_turn: Local<u32>,
    fresh: Query<(), Added<Exploring>>,
    mut explorers: Query<(Entity, &mut Unit), With<Exploring>>,
    others: Query<&Unit, Without<Exploring>>,
    cities: Query<&City>,
    tiles: Query<&HexTile>,
    (map, diplomacy): (Res<MapData>, Res<Diplomacy>),
    mut fog: ResMut<FogOfWar>,
    (mut log, mut dirty): (ResMut<GameLog>, ResMut<Dirty>),
    mut commands: Commands,
) {
    let new_turn = std::mem::replace(&mut *last_turn, state.turn) != state.turn;
    if (!new_turn && fresh.is_empty()) || explorers.is_empty() || state.victory.is_some() {
        return;
    }
    let ports: HashSet<Hex> = cities.iter().filter(|c| c.owner == PLAYER).map(|c| c.position).collect();
    let land: Vec<(Hex, Terrain)> = tiles.iter().map(|t| (t.coord, t.terrain)).collect();
    let roads = road_network(tiles.iter(), cities.iter().map(|c| c.position));
    let foreign: HashSet<Hex> = others.iter().filter(|u| u.owner != PLAYER).map(|u| u.position).collect();
    let enemy_armies: HashSet<Hex> = others
        .iter()
        .filter(|u| u.owner != PLAYER && u.unit_type.is_army() && (u.owner == BARBARIANS || diplomacy.get_treaty(PLAYER, u.owner) == Treaty::War))
        .map(|u| u.position)
        .collect();
    let mut claimed: Vec<Hex> = Vec::new();
    for (entity, mut unit) in &mut explorers {
        if unit.build_turns_left.is_some() || !unit.can_move() {
            continue;
        }
        let sea = unit.unit_type.domain() == Domain::Sea;
        let travel = player_travel(unit.unit_type, &ports);
        let mut found_any = false;
        for _ in 0..4 {
            if !unit.can_move() {
                break;
            }
            let from = unit.position;
            // Known tiles of the right kind that border the unknown, nearest first.
            let mut frontier: Vec<Hex> = fog
                .explored
                .iter()
                .copied()
                .filter(|h| *h != from && map.water.contains(h) == sea && !foreign.contains(h) && !claimed.iter().any(|c| c.distance_to(*h) <= 2))
                .filter(|h| h.all_neighbors().iter().any(|n| map.cells.contains(n) && !fog.explored.contains(n)))
                .collect();
            frontier.sort_by_key(|h| (from.distance_to(*h), h.x, h.y));
            let Some((target, route)) = frontier
                .iter()
                .take(6)
                .find_map(|t| find_route(from, *t, land.iter().copied(), &roads, travel).filter(|r| r.len() > 1).map(|r| (*t, r)))
            else {
                break;
            };
            found_any = true;
            claimed.push(target);
            let clear = route.iter().skip(1).position(|h| foreign.contains(h)).map_or(route.len(), |i| i + 1);
            let mut trail = Vec::new();
            walk(&mut unit, &route[..clear], &map, &roads, &enemy_armies, &mut fog, &mut trail);
            if trail.is_empty() {
                break;
            }
            commands.entity(entity).try_insert(Trail(trail));
            dirty.map = true;
        }
        if found_any {
            // Done for the turn: it does not ask for orders.
            unit.end_move();
        } else {
            commands.entity(entity).remove::<Exploring>();
            log.push(format!("Our {} has seen all it can reach and awaits orders.", unit.unit_type.name()));
            dirty.ui = true;
        }
    }
}

/// At the start of the player's turn, automated workers pick the most useful job inside our borders:
/// clean pollution, road a resource, mine hills, farm flat land, then lay railroads. They walk to the
/// nearest such tile (two workers never take the same one) and start work when they arrive.
#[allow(clippy::too_many_arguments)]
fn automate_workers(
    state: Res<GameState>,
    mut last_turn: Local<u32>,
    mut workers: Query<(Entity, &mut Unit), With<Automated>>,
    tiles: Query<&HexTile>,
    cities: Query<&City>,
    (map, tech, supplies): (Res<MapData>, Res<TechState>, Res<Supplies>),
    mut fog: ResMut<FogOfWar>,
    mut commands: Commands,
) {
    if std::mem::replace(&mut *last_turn, state.turn) == state.turn || state.victory.is_some() {
        return;
    }
    let city_tiles: HashSet<Hex> = cities.iter().map(|c| c.position).collect();
    let roads = road_network(tiles.iter(), city_tiles.iter().copied());
    let land: Vec<(Hex, Terrain)> = tiles.iter().map(|t| (t.coord, t.terrain)).collect();
    let steam = tech.researched.contains(&Tech::SteamPower);
    // The job a tile needs, if any, with its urgency (lower is sooner).
    let job = |tile: &HexTile| -> Option<(u8, Improvement)> {
        let ours = supplies.territory.get(&tile.coord).is_some_and(|(o, _)| *o == PLAYER);
        if !ours || tile.terrain == Terrain::Water || city_tiles.contains(&tile.coord) {
            return None;
        }
        if tile.is_polluted() {
            return Some((0, Improvement::Pollution));
        }
        if tile.resource.is_some() && !tile.has_road() {
            return Some((1, Improvement::Road));
        }
        match tile.terrain {
            Terrain::Hills | Terrain::Mountain if !tile.improvements.contains(&Improvement::Mine) => return Some((2, Improvement::Mine)),
            Terrain::Grassland | Terrain::Plains if !tile.improvements.contains(&Improvement::Farm) => return Some((3, Improvement::Farm)),
            // A road on flat land brings trade.
            Terrain::Grassland | Terrain::Plains | Terrain::Desert if !tile.has_road() => return Some((4, Improvement::Road)),
            _ => {}
        }
        (steam && tile.has_road() && !tile.has_railroad()).then_some((5, Improvement::Railroad))
    };
    let jobs: Vec<(Hex, u8, Improvement)> = tiles.iter().filter_map(|t| job(t).map(|(p, j)| (t.coord, p, j))).collect();
    let mut claimed: HashSet<Hex> = HashSet::new();
    for (entity, mut unit) in &mut workers {
        if unit.owner != PLAYER || unit.build_turns_left.is_some() || !unit.can_move() {
            continue;
        }
        let here = unit.position;
        let target = jobs
            .iter()
            .filter(|(at, _, _)| !claimed.contains(at))
            .min_by_key(|(at, urgency, _)| (*urgency as i32) * 3 + at.distance_to(here))
            .copied();
        let Some((at, _, improvement)) = target else {
            unit.end_move();
            continue;
        };
        claimed.insert(at);
        if at != here {
            let Some(route) = find_route(here, at, land.iter().copied(), &roads, Travel::Land) else { continue };
            let mut trail = Vec::new();
            walk(&mut unit, &route, &map, &roads, &HashSet::new(), &mut fog, &mut trail);
            commands.entity(entity).try_insert(Trail(trail));
        }
        if unit.position == at && unit.can_move() {
            unit.start_build(improvement);
        } else {
            unit.end_move();
        }
    }
}

/// Works out the route to show for the selected unit, re-planning only when something it depends on changed.
#[allow(clippy::too_many_arguments)]
fn preview_route(
    selection: Res<Selection>,
    hovered: Res<client::map::Hovered>,
    units: Query<(&Unit, Option<&GoTo>)>,
    cities: Query<&City>,
    tiles: Query<&HexTile>,
    (map, fog, wonders): (Res<MapData>, Res<FogOfWar>, Res<WorldWonders>),
    mut path: ResMut<PathPreview>,
    mut planned: Local<Option<(Hex, bevy_civ::pathfinding::Budget, Hex)>>,
) {
    let unit = selection.unit.and_then(|e| units.get(e).ok()).filter(|(u, _)| u.owner == PLAYER && u.build_turns_left.is_none());
    let Some((unit, order)) = unit else {
        *planned = None;
        path.set_if_neq(PathPreview::default());
        return;
    };
    // A click on the hovered tile would move there (not select something of the player's).
    let own_there = |hex: Hex| units.iter().any(|(u, _)| u.position == hex && u.owner == PLAYER) || cities.iter().any(|c| c.position == hex && c.owner == PLAYER);
    let hover = hovered.0.filter(|h| *h != unit.position && fog.explored.contains(h) && !own_there(*h));
    let Some(target) = hover.or(order.map(|o| o.0)).filter(|t| *t != unit.position) else {
        *planned = None;
        path.set_if_neq(PathPreview::default());
        return;
    };
    let key = (unit.position, unit.budget(), target);
    if *planned == Some(key) {
        return;
    }
    *planned = Some(key);
    let ports: HashSet<Hex> = cities.iter().filter(|c| c.owner == PLAYER).map(|c| c.position).collect();
    let travel = player_travel(unit.unit_type, &ports);
    let roads = road_network(tiles.iter(), cities.iter().map(|c| c.position));
    let Some(route) = find_route(unit.position, target, tiles.iter().map(|t| (t.coord, t.terrain)), &roads, travel) else {
        path.set_if_neq(PathPreview::default());
        return;
    };
    let domain = unit.unit_type.domain();
    let turns = turns_along(&route, unit.budget(), |at| moves_per_turn(unit.unit_type, at, &map, wonders.sea_moves(PLAYER)), |a, b| move_cost(domain, a, b, &map, &roads));
    path.set_if_neq(PathPreview { start: Some(unit.position), steps: route[1..].iter().copied().zip(turns).collect() });
}

/// Troops in a ship and aircraft on a carrier cannot fight back at sea: the ship defends them.
fn defending_as(unit: UnitType, at: Hex, map: &MapData) -> UnitType {
    if unit.domain() != Domain::Sea && map.water.contains(&at) { UnitType::Worker } else { unit }
}

/// Whether a victorious attacker can move onto the tile it cleared.
fn may_enter(unit: UnitType, tile: Hex, map: &MapData, own_cities: &HashSet<Hex>) -> bool {
    match unit.domain() {
        Domain::Land => true,
        Domain::Sea => map.water.contains(&tile) || own_cities.contains(&tile),
        Domain::Air => false,
    }
}

/// How near an enemy must be for ending the turn to be worth a second thought.
const THREAT_RANGE: i32 = 3;

/// Ending the turn passes through here: a city with no soldier in it, with an enemy in sight close by,
/// is shown to the player first (a second End Turn goes ahead anyway).
fn end_turn_guard(
    mut requests: EventReader<EndTurnRequest>,
    mut go: EventWriter<EndTurnNow>,
    mut focus: EventWriter<Focus>,
    mut warned: Local<Option<u32>>,
    state: Res<GameState>,
    fog: Res<FogOfWar>,
    diplomacy: Res<Diplomacy>,
    cities: Query<&City>,
    units: Query<&Unit>,
    mut log: ResMut<GameLog>,
) {
    for _ in requests.read() {
        let at_war = |owner: u32| owner == BARBARIANS || (owner != PLAYER && diplomacy.get_treaty(PLAYER, owner) == Treaty::War);
        let danger = cities
            .iter()
            .filter(|c| c.owner == PLAYER && !units.iter().any(|u| u.owner == PLAYER && u.position == c.position && u.unit_type.is_army()))
            .find_map(|c| {
                units
                    .iter()
                    .filter(|u| at_war(u.owner) && u.unit_type.is_military() && fog.visible.contains(&u.position))
                    .filter(|u| u.position.distance_to(c.position) <= THREAT_RANGE)
                    .min_by_key(|u| u.position.distance_to(c.position))
                    .map(|u| (c.name.clone(), c.position, u.unit_type.name(), u.position.distance_to(c.position)))
            });
        match danger {
            Some((city, at, enemy, distance)) if *warned != Some(state.turn) => {
                *warned = Some(state.turn);
                focus.send(Focus(at));
                log.push(format!("{city} has no soldier inside and an enemy {enemy} is {distance} tiles away. Press End Turn again to end the turn anyway."));
            }
            _ => {
                go.send(EndTurnNow);
            }
        }
    }
}

fn apply_actions(
    mut actions: EventReader<Action>,
    mut selection: ResMut<Selection>,
    mut units: Query<&mut Unit>,
    mut cities: Query<&mut City>,
    mut tiles: Query<&mut HexTile>,
    mut state: ResMut<GameState>,
    mut tech: ResMut<TechState>,
    (mut diplomacy, mut foreign): (ResMut<Diplomacy>, ResMut<Foreign>),
    mut trade: ResMut<TradeRoutes>,
    mut fog: ResMut<FogOfWar>,
    mut civs: ResMut<Civilizations>,
    mut log: ResMut<GameLog>,
    mut dirty: ResMut<Dirty>,
    (mut cycle, supplies, mut map, mut end_turn, mut focus, mut battles): (ResMut<Cycle>, Res<Supplies>, ResMut<MapData>, EventWriter<EndTurnRequest>, EventWriter<Focus>, EventWriter<BattleShown>),
    (unit_entities, mut sounds, mut wonders, mut great, mut spies, homes, mut walled, mut taken, automated, rivers, mut saves, mut religions, mut space, mut last_move, exploring): (
        Query<Entity, With<Unit>>,
        EventWriter<Sfx>,
        ResMut<WorldWonders>,
        ResMut<GreatPeople>,
        ResMut<Espionage>,
        Query<&HomeCity>,
        Query<(Entity, &mut CityBuildings)>,
        ResMut<JustCaptured>,
        Query<Entity, With<Automated>>,
        Res<Rivers>,
        EventWriter<savegame::SaveRequest>,
        ResMut<Religions>,
        ResMut<SpaceRace>,
        ResMut<LastMove>,
        Query<Entity, With<Exploring>>,
    ),
    mut commands: Commands,
) {
    for action in actions.read() {
        dirty.ui = true;
        dirty.map = true;
        let selected_unit = selection.unit.filter(|e| units.get(*e).is_ok_and(|u| u.owner == PLAYER));
        // B founds a city with a settler and bombards with a gun, warship or bomber, as in Civ III.
        let bombards = selected_unit.and_then(|e| units.get(e).ok()).is_some_and(|u| u.unit_type.can_bombard());
        let action = if matches!(action, Action::FoundCity) && bombards { &Action::Bombard } else { action };
        // Anything a unit does besides moving cannot be taken back with Undo.
        if matches!(
            action,
            Action::Bombard | Action::BombardAt(_) | Action::Pillage | Action::UseUnit | Action::Spy(_) | Action::TradeRoute | Action::FoundCity | Action::Build(_) | Action::Transform | Action::JoinCity | Action::Upgrade | Action::UpgradeAll | Action::EndTurn
        ) {
            last_move.0 = None;
        }
        match action {
            Action::EndTurn => {
                end_turn.send(EndTurnRequest);
            }
            Action::NextUnit => {
                selection.city = None;
                cycle.advance = true;
                cycle.force = true;
            }
            Action::SkipOrEnd => {
                let waiting = selected_unit.is_some_and(|e| units.get(e).is_ok_and(needs_orders));
                if waiting {
                    if let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) {
                        unit.end_move();
                    }
                    cycle.advance = true;
                } else if units.iter().any(needs_orders) {
                    selection.city = None;
                    cycle.advance = true;
                    cycle.force = true;
                } else if let Some((entity, _)) = idle_city(walled.iter().filter_map(|(e, _)| cities.get(e).ok().map(|c| (e, c)))).filter(|(e, _)| selection.city != Some(*e)) {
                    // A city waiting for a project is shown before the turn ends (Enter ends it anyway).
                    selection.unit = None;
                    selection.city = Some(entity);
                    if let Ok(city) = cities.get(entity) {
                        log.push(format!("{} needs something to build (Enter ends the turn anyway).", city.name));
                    }
                } else {
                    end_turn.send(EndTurnRequest);
                }
            }
            Action::Sentry => {
                if let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) {
                    unit.fortified = true;
                    cycle.advance = true;
                }
                if let Some(entity) = selected_unit {
                    commands.entity(entity).remove::<GoTo>().remove::<Exploring>().insert(Sentry);
                }
            }
            Action::Board => {
                let Some(entity) = selected_unit else { continue };
                let berths: Vec<Berth> = unit_entities.iter().filter_map(|e| units.get(e).ok().map(|u| Berth::of(e, &u))).collect();
                let Ok(mut unit) = units.get_mut(entity) else { continue };
                let ship = cargo::ship_with_room(&berths, PLAYER, unit.position, unit.unit_type).filter(|s| *s != entity);
                let Some(ship) = ship else {
                    log.push("There is no ship of ours here with room for it.");
                    continue;
                };
                unit.carrier = Some((ship, unit.position));
                unit.fortified = unit.unit_type.domain() == Domain::Land;
                let name = unit.unit_type.name();
                let ship_name = units.get(ship).map(|s| s.unit_type.name()).unwrap_or("ship");
                log.push(format!("Our {name} goes aboard the {ship_name}. Sail the ship, then select the {name} and click a shore tile next to it to land."));
                commands.entity(entity).remove::<GoTo>();
                cycle.advance = true;
                dirty.ui = true;
            }
            Action::Unload => {
                let Some(entity) = selected_unit else { continue };
                let aboard: Vec<Entity> = unit_entities
                    .iter()
                    .filter(|e| *e == entity || units.get(*e).is_ok_and(|u| u.carrier.map(|c| c.0) == Some(entity)))
                    .collect();
                let mut count = 0;
                for e in aboard {
                    let Ok(mut unit) = units.get_mut(e) else { continue };
                    if unit.carrier.is_some() && !map.water.contains(&unit.position) {
                        unit.carrier = None;
                        unit.fortified = false;
                        count += 1;
                    }
                }
                if count == 0 {
                    log.push("Only units aboard a ship in port can leave it here; at sea, select them and click a shore tile next to the ship.");
                }
                dirty.ui = true;
            }
            Action::JoinCity => {
                let Some(entity) = selected_unit else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                let adds = match unit.unit_type {
                    UnitType::Settler => 2,
                    UnitType::Worker | UnitType::Engineers => 1,
                    _ => continue,
                };
                let here = unit.position;
                let Some(city_entity) = walled.iter().map(|(e, _)| e).find(|e| cities.get(*e).is_ok_and(|c| c.position == here && c.owner == PLAYER)) else {
                    log.push("Workers and settlers join one of our cities from inside it.");
                    continue;
                };
                let limit = walled.get(city_entity).map(|(_, b)| {
                    let fresh = rivers.tiles.contains(&here) || here.all_neighbors().iter().any(|n| rivers.tiles.contains(n));
                    Building::size_limit(&b.buildings, fresh)
                }).unwrap_or(8);
                let Ok(mut city) = cities.get_mut(city_entity) else { continue };
                if city.population + adds > limit {
                    log.push(format!("{} has no room for more people (size {limit} until it has more water).", city.name));
                    continue;
                }
                city.population += adds;
                log.push(format!("The {} settles in {}, which grows to size {}.", unit_name(adds), city.name, city.population));
                commands.entity(entity).try_despawn_recursive();
                selection.unit = None;
                cycle.advance = true;
                sounds.send(Sfx::Select);
            }
            Action::Fortify => {
                if let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) {
                    unit.fortified = true;
                    cycle.advance = true;
                }
                if let Some(entity) = selected_unit {
                    commands.entity(entity).remove::<GoTo>().remove::<Exploring>();
                }
            }
            Action::Trade(civ, ask) => {
                let mut ctx = foreign::Ctx { diplomacy: &mut diplomacy, foreign: &mut foreign, civs: &mut civs, state: &mut state, tech: &mut tech, log: &mut log };
                foreign::player_trade(&mut ctx, &supplies, *civ, *ask);
            }
            Action::Deselect => {
                selection.unit = None;
                selection.city = None;
            }
            Action::Select(entity) => {
                if units.get(*entity).is_ok_and(|u| u.owner == PLAYER) {
                    selection.unit = Some(*entity);
                    selection.city = None;
                    sounds.send(Sfx::Select);
                }
            }
            Action::UseUnit => {
                let Some(entity) = selected_unit else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                let (kind, here) = (unit.unit_type, unit.position);
                // Ground troops join an army of ours on their tile that has room.
                if bevy_civ::unit::can_join_army(unit) {
                    let recruit = unit.clone();
                    let capacity = bevy_civ::unit::army_capacity(wonders.owns(PLAYER, Building::SunTzu));
                    let army = unit_entities.iter().find(|e| {
                        *e != entity && units.get(*e).is_ok_and(|a| a.owner == PLAYER && a.position == here && a.is_army_group() && (a.members() as usize) < capacity)
                    });
                    let Some(army) = army else {
                        log.push(format!("No army of ours with room stands here. A Great General forms one in a city; it then takes up to {capacity} units."));
                        continue;
                    };
                    if let Ok(mut lead) = units.get_mut(army) {
                        if lead.take_into_army(&recruit, capacity) {
                            log.push(format!("The {} joins the army: {} ({} of {capacity}, {} hit points).", recruit.unit_type.name(), lead.display_name(), lead.members(), lead.max_hp()));
                            commands.entity(entity).try_despawn_recursive();
                            selection.unit = Some(army);
                            sounds.send(Sfx::Select);
                            dirty.ui = true;
                        }
                    }
                    continue;
                }
                if kind == UnitType::Caravan {
                    // A caravan in one of the player's cities adds its shields to the wonder being built there.
                    let Some(mut city) = cities.iter_mut().find(|c| c.position == here && c.owner == PLAYER) else {
                        log.push("A caravan helps build a wonder: move it into one of your cities that is building one, then press U. (T opens a trade route.)");
                        continue;
                    };
                    match city.production_queue.clone() {
                        Some(ProductionItem::Building(wonder)) if wonder.is_wonder() => {
                            city.production_progress += UnitType::Caravan.cost();
                            log.push(format!("The caravan adds {} shields to the {} in {}.", UnitType::Caravan.cost(), wonder.name(), city.name));
                            commands.entity(entity).try_despawn_recursive();
                            selection.unit = None;
                            cycle.advance = true;
                            sounds.send(Sfx::Select);
                        }
                        _ => log.push(format!("{} is not building a wonder; a caravan can only help with wonders.", city.name)),
                    }
                    continue;
                }
                if kind == UnitType::Missionary {
                    // A missionary brings our state religion (our capital's faith) to the city it stands in.
                    let capital = cities.iter().find(|c| c.owner == PLAYER && c.is_capital).map(|c| c.position);
                    let Some(faith) = capital.and_then(|cp| religions.of(cp)) else {
                        log.push("We have no state religion to preach: our capital must follow a faith first.");
                        continue;
                    };
                    // The city it stands in, or else a neighbouring city that does not follow our faith yet.
                    let Some((owner, name, target)) = cities
                        .iter()
                        .find(|c| c.position == here)
                        .or_else(|| cities.iter().filter(|c| c.position.distance_to(here) == 1).find(|c| religions.of(c.position) != Some(faith)))
                        .map(|c| (c.owner, c.name.clone(), c.position))
                    else {
                        log.push("A Missionary preaches in or next to a city: move it into one of ours, or beside a foreign city we are not at war with, then press U.");
                        continue;
                    };
                    if owner != PLAYER && (owner == BARBARIANS || diplomacy.get_treaty(PLAYER, owner) == Treaty::War) {
                        log.push("A Missionary cannot preach in the city of an enemy.");
                        continue;
                    }
                    if religions.of(target) == Some(faith) {
                        log.push(format!("{name} already follows {}.", faith.name()));
                        continue;
                    }
                    religions.city.insert(target, faith);
                    log.push(format!("Our Missionary brings {} to {name}.", faith.name()));
                    commands.entity(entity).try_despawn_recursive();
                    selection.unit = None;
                    cycle.advance = true;
                    sounds.send(Sfx::Select);
                    continue;
                }
                if !kind.is_great_person() {
                    continue;
                }
                let city_here = cities.iter().find(|c| c.position == here).map(|c| (c.owner, c.name.clone()));
                let own_city = city_here.as_ref().filter(|(o, _)| *o == PLAYER).map(|(_, n)| n.clone());
                let used = match kind {
                    UnitType::GreatScientist => match tech.discover() {
                        Some(learned) => {
                            log.push(format!("Your Great Scientist's discovery: {}!", learned.name()));
                            true
                        }
                        None => {
                            log.push("Choose something to research first (Q); the Great Scientist will complete it.");
                            false
                        }
                    },
                    UnitType::GreatMerchant => match &city_here {
                        Some((owner, name)) if *owner == PLAYER => {
                            *great.merchants.entry(here).or_default() += 1;
                            log.push(format!("Your Great Merchant settles in {name} and founds a trading house: +{} gold there every turn.", great::SETTLED_MERCHANT_GOLD));
                            true
                        }
                        Some((owner, name)) if *owner != BARBARIANS && diplomacy.get_treaty(PLAYER, *owner) != Treaty::War => {
                            let capital = cities.iter().find(|c| c.owner == PLAYER && c.is_capital).map(|c| c.position).unwrap_or(here);
                            let gold = great::trade_mission_gold(capital.distance_to(here));
                            state.gold += gold;
                            log.push(format!("Your Great Merchant's trade mission to {name} brings in {gold} gold."));
                            true
                        }
                        _ => {
                            log.push("A Great Merchant trades in a foreign city you are not at war with (a trade mission for gold now), or settles in one of your cities (gold every turn): move it into one, then press U.");
                            false
                        }
                    },
                    UnitType::GreatEngineer => match own_city {
                        Some(name) => {
                            let Some(mut city) = cities.iter_mut().find(|c| c.position == here) else { continue };
                            match city.production_queue.clone() {
                                Some(item) => {
                                    city.production_progress = city.production_progress.max(item.cost());
                                    log.push(format!("Your Great Engineer finishes the work: {name} completes its {} at the end of the turn.", item.name()));
                                    true
                                }
                                None => {
                                    log.push(format!("{name} is building nothing; choose a project for the Great Engineer to finish."));
                                    false
                                }
                            }
                        }
                        None => {
                            log.push("A Great Engineer hurries production in one of your cities: move it into one, then press U.");
                            false
                        }
                    },
                    UnitType::GreatArtist => match own_city {
                        Some(name) => {
                            *great.masterpieces.entry(here).or_default() += 1;
                            log.push(format!("Your Great Artist creates a masterpiece in {name}: +3 content citizens there for good."));
                            true
                        }
                        None => {
                            log.push("A Great Artist makes a masterpiece in one of your cities: move it into one, then press U.");
                            false
                        }
                    },
                    UnitType::GreatGeneral => {
                        // In one of our cities, the general forms the troops there into an army.
                        let stack: Vec<(Entity, Unit)> = if own_city.is_some() {
                            unit_entities.iter().filter_map(|e| units.get(e).ok().filter(|u| u.position == here && u.owner == PLAYER).map(|u| (e, u.clone()))).collect()
                        } else {
                            Vec::new()
                        };
                        let refs: Vec<&Unit> = stack.iter().map(|(_, u)| u).collect();
                        match bevy_civ::unit::plan_army(&refs, bevy_civ::unit::army_capacity(wonders.owns(PLAYER, Building::SunTzu))) {
                            Some((leader, absorbed)) => {
                                let members: Vec<Unit> = absorbed.iter().map(|i| stack[*i].1.clone()).collect();
                                for i in &absorbed {
                                    commands.entity(stack[*i].0).try_despawn_recursive();
                                }
                                if let Ok(mut lead) = units.get_mut(stack[leader].0) {
                                    bevy_civ::unit::join_army(&mut lead, &members);
                                    log.push(format!("Your Great General forms an army: {} ({} hit points), fighting as one.", lead.display_name(), lead.max_hp()));
                                }
                                true
                            }
                            None => {
                                log.push("A Great General forms an army: bring it and two or three land units into one of your cities, then press U. Until then, troops on its tile fight half again as hard.");
                                false
                            }
                        }
                    }
                    _ => false,
                };
                if used {
                    commands.entity(entity).try_despawn_recursive();
                    selection.unit = None;
                    cycle.advance = true;
                    sounds.send(Sfx::Select);
                }
            }
            Action::Revolution(government) => {
                if !government.available(&tech) || *government == state.government || state.anarchy_turns > 0 {
                    continue;
                }
                if wonders.owns(PLAYER, Building::Liberty) {
                    state.government = *government;
                    let rates = state.rates();
                    state.set_rates(rates);
                    log.push(format!("The Statue of Liberty spares us anarchy: we are now a {}.", government.name()));
                    continue;
                }
                let religious = civs.players.iter().find(|c| c.id == PLAYER).is_some_and(|c| bevy_civ::traits::of(&c.name).has(bevy_civ::traits::Trait::Religious));
                let turns = if religious { Government::REVOLUTION_TURNS - 1 } else { Government::REVOLUTION_TURNS };
                state.government = Government::Anarchy;
                state.anarchy_turns = turns;
                state.pending_government = Some(*government);
                log.push(format!("Revolution! After {turns} turns of anarchy we will be a {}.", government.name()));
            }
            Action::SelectCity(entity) => {
                if let Ok(city) = cities.get(*entity) {
                    selection.city = Some(*entity);
                    selection.unit = None;
                    focus.send(Focus(city.position));
                    sounds.send(Sfx::Select);
                }
            }
            Action::Automate => {
                let Some(entity) = selected_unit else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                if !unit.unit_type.is_worker() {
                    continue;
                }
                if automated.contains(entity) {
                    commands.entity(entity).remove::<Automated>();
                    log.push("The worker awaits your orders again.");
                } else {
                    commands.entity(entity).insert(Automated);
                    commands.entity(entity).remove::<GoTo>();
                    if let Ok(mut unit) = units.get_mut(entity) {
                        unit.end_move();
                    }
                    log.push("The worker will look after our land by itself from next turn (K again to take it back).");
                    cycle.advance = true;
                }
            }
            Action::Explore => {
                let Some(entity) = selected_unit else { continue };
                let Ok(mut unit) = units.get_mut(entity) else { continue };
                if !can_explore(&unit) {
                    log.push("Only scouts, soldiers and warships can explore by themselves (not armies, or troops aboard a ship).");
                    continue;
                }
                if exploring.contains(entity) {
                    commands.entity(entity).remove::<Exploring>();
                    log.push(format!("The {} awaits your orders again.", unit.unit_type.name()));
                } else {
                    unit.fortified = false;
                    commands.entity(entity).remove::<GoTo>().insert(Exploring);
                    log.push(format!("The {} sets out to explore by itself (C again takes it back; any order of yours does too).", unit.unit_type.name()));
                    cycle.advance = true;
                }
            }
            Action::AutoBuild(on) => {
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                if city.owner == PLAYER {
                    city.auto_build = *on;
                    dirty.ui = true;
                }
            }
            Action::AutoBuildAll(on) => {
                for mut city in cities.iter_mut().filter(|c| c.owner == PLAYER) {
                    city.auto_build = *on;
                }
                log.push(if *on { "Every city's governor now chooses its builds when a project is done." } else { "Our cities wait for our orders again when a project is done." });
                dirty.ui = true;
            }
            Action::SetMigration(welcome) => {
                state.welcome_migrants = *welcome;
                log.push(if *welcome {
                    "Our borders are open to migrants: they settle in our cities and grow them (their rulers resent it)."
                } else {
                    "Our borders are closed to migrants from abroad."
                });
                dirty.ui = true;
            }
            Action::SetRates(science, luxury) => {
                let rates = bevy_civ::economy::Rates { science: *science, luxury: *luxury };
                if rates.allowed(state.government) {
                    state.set_rates(rates);
                }
            }
            Action::SellBuilding(building) => {
                let Some(entity) = selection.city else { continue };
                let Ok(city) = cities.get(entity) else { continue };
                if city.owner != PLAYER || building.is_wonder() || state.sold.contains(&city.position) {
                    continue;
                }
                let Ok((_, mut built)) = walled.get_mut(entity) else { continue };
                if built.has(*building) {
                    built.buildings.retain(|b| b != building);
                    state.sold.insert(city.position);
                    state.gold += building.cost() as i32;
                    log.push(format!("{} sells its {} for {} gold (it cost {} a turn to keep).", city.name, building.name(), building.cost(), bevy_civ::economy::upkeep(*building)));
                }
            }
            Action::Hurry | Action::Rush => {
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                if city.owner != PLAYER {
                    continue;
                }
                let Some(item) = city.production_queue.clone() else { continue };
                if !city.can_hurry() {
                    log.push(if matches!(item, ProductionItem::Building(b) if b.is_wonder()) {
                        "Wonders cannot be bought; caravans and great engineers can help build them."
                    } else {
                        "There is nothing left to hurry."
                    });
                    continue;
                }
                if *action == Action::Hurry {
                    let Some(cost) = city.hurry_cost() else { continue };
                    if state.gold < cost {
                        log.push(format!("Finishing the {} costs {cost} gold; we have {}.", item.name(), state.gold));
                        continue;
                    }
                    state.gold -= cost;
                    log.push(format!("We pay {cost} gold: {} completes its {} at the end of the turn.", city.name, item.name()));
                } else {
                    let Some(citizens) = city.rush_citizens(state.government) else {
                        log.push("Only Despotism and Communism can rush work with citizens, and the city must keep one.");
                        continue;
                    };
                    city.population -= citizens;
                    city.food_storage = city.food_storage.min(city.food_to_grow());
                    while city.specialists.len() as u32 > city.population {
                        city.specialists.pop();
                    }
                    log.push(format!("{citizens} citizens of {} are worked to the bone: its {} is done at the end of the turn.", city.name, item.name()));
                }
                city.production_progress = item.cost();
            }
            Action::Raze => {
                let Some(entity) = selection.city else { continue };
                let Ok(city) = cities.get(entity) else { continue };
                if city.owner != PLAYER || !taken.0.contains(&city.position) {
                    continue;
                }
                log.push(format!("{} is razed to the ground.", city.name));
                let (at, name) = (city.position, city.name.clone());
                wonders.built.retain(|_, (_, place)| *place != name);
                taken.0.remove(&at);
                commands.entity(entity).despawn_recursive();
                selection.city = None;
            }
            Action::Specialist(kind, add) => {
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                if city.owner != PLAYER {
                    continue;
                }
                if *add {
                    if city.workers() == 0 {
                        log.push("Everyone in this city is already a specialist.");
                    } else {
                        city.specialists.push(kind.clone());
                    }
                } else if let Some(i) = city.specialists.iter().position(|s| s == kind) {
                    city.specialists.remove(i);
                }
            }
            Action::Upgrade => {
                let Some(entity) = selected_unit else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                let here = unit.position;
                if !cities.iter().any(|c| c.position == here && c.owner == PLAYER) {
                    log.push("Units are upgraded in one of your cities.");
                    continue;
                }
                let equip = |to: UnitType| to.required_tech().is_none_or(|t| tech.researched.contains(&t)) && supplies.missing(here, to.required_resources()).is_empty();
                let Some((cost, to)) = unit.refit_cost(equip) else {
                    log.push(format!("This city cannot re-equip our {} yet.", unit.unit_type.name()));
                    continue;
                };
                if state.gold < cost {
                    log.push(format!("Re-equipping our {} costs {cost} gold; we have {}.", unit.unit_type.name(), state.gold));
                    continue;
                }
                state.gold -= cost;
                if let Ok(mut unit) = units.get_mut(entity) {
                    if unit.is_army_group() {
                        log.push(format!("Our army is re-equipped for {cost} gold."));
                    } else {
                        log.push(format!("Our {} is re-equipped as a {} for {cost} gold.", unit.unit_type.name(), to.name()));
                    }
                    unit.refit(equip);
                    unit.end_move();
                }
            }
            Action::UpgradeAll => {
                let Some(entity) = selected_unit else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                let here = unit.position;
                if !cities.iter().any(|c| c.position == here && c.owner == PLAYER) {
                    log.push("Units are upgraded in one of your cities.");
                    continue;
                }
                let equip = |to: UnitType| to.required_tech().is_none_or(|t| tech.researched.contains(&t)) && supplies.missing(here, to.required_resources()).is_empty();
                let mut garrison: Vec<(Entity, i32)> = unit_entities
                    .iter()
                    .filter_map(|e| units.get(e).ok().map(|u| (e, u)))
                    .filter(|(_, u)| u.owner == PLAYER && u.position == here)
                    .filter_map(|(e, u)| u.refit_cost(equip).map(|(cost, _)| (e, cost)))
                    .collect();
                // Cheapest first, so the gold goes as far as it can.
                garrison.sort_by_key(|(e, cost)| (*cost, e.index()));
                let (mut done, mut spent, mut short) = (0, 0, 0);
                for (e, cost) in garrison {
                    if state.gold < cost {
                        short += 1;
                        continue;
                    }
                    if let Ok(mut unit) = units.get_mut(e) {
                        state.gold -= cost;
                        spent += cost;
                        done += 1;
                        unit.refit(equip);
                        unit.end_move();
                    }
                }
                match (done, short) {
                    (0, 0) => log.push("Nothing here can be re-equipped yet."),
                    (0, _) => log.push("We cannot afford to re-equip anyone here."),
                    (_, 0) => log.push(format!("{done} units re-equipped for {spent} gold.")),
                    _ => log.push(format!("{done} units re-equipped for {spent} gold; {short} more wait for gold.")),
                }
            }
            Action::Bombard | Action::BombardAt(_) => {
                let Some(entity) = selected_unit else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                let Some((_, range, _)) = unit.unit_type.bombard() else { continue };
                if !unit.can_move() {
                    log.push(format!("Our {} has no moves left: it can bombard next turn.", unit.unit_type.name()));
                    continue;
                }
                let (kind, here) = (unit.unit_type, unit.position);
                let aimed = if let Action::BombardAt(at) = action { Some(*at) } else { None };
                // The stoutest enemy we are at war with (or barbarians) within range: a city's
                // garrison first.
                let at_war = |owner: u32| owner == BARBARIANS || diplomacy.get_treaty(PLAYER, owner) == Treaty::War;
                let in_city = |at: Hex| cities.iter().any(|c| c.position == at);
                let target = unit_entities
                    .iter()
                    .filter_map(|e| units.get(e).ok().map(|u| (e, u)))
                    .filter(|(_, u)| (1..=range).contains(&u.position.distance_to(here)) && u.owner != PLAYER && at_war(u.owner) && aimed.is_none_or(|a| a == u.position))
                    .max_by_key(|(_, u)| (in_city(u.position), defending_as(u.unit_type, u.position, &map).stats().1 * 10 + u.hp))
                    .map(|(e, u)| (e, u.unit_type, u.hp, u.max_hp(), u.level, u.fortified, u.position, u.owner, u.members()));
                let Some((target, their_kind, their_hp, their_max, their_level, dug_in, at, their_owner, members)) = target else {
                    let reach = if range > 1 { format!("within {range} tiles") } else { "next to it".to_string() };
                    log.push(format!("There is no enemy we are at war with {reach} for our {} to bombard.", kind.name()));
                    continue;
                };
                let terrain = map.terrain.get(&at).copied().unwrap_or(Terrain::Plains);
                let city = cities.iter().find(|c| c.position == at).map(|c| (c.population, c.owner));
                let has = |building: Building| walled.iter().any(|(e, b)| b.has(building) && cities.get(e).is_ok_and(|c| c.position == at));
                let ground = Ground {
                    fortified: dug_in,
                    walls: city.is_some_and(|(_, owner)| has(Building::Walls) || wonders.walls_everywhere(owner)),
                    river: rivers.tiles.contains(&at),
                    coast_fort: has(Building::CoastalFortress),
                    city: city.map(|(population, _)| population),
                    fortress: tiles.iter().any(|t| t.coord == at && t.improvements.contains(&Improvement::Fortress)),
                    ..Ground::open(terrain)
                };
                let defender = Side { kind: defending_as(their_kind, at, &map), hp: their_hp, level: their_level, led: false, members };
                let hp = combat::bombard(kind, defender, ground, &mut rand::thread_rng());
                if let Ok(mut victim) = units.get_mut(target) {
                    victim.hp = hp;
                }
                if let Ok(mut unit) = units.get_mut(entity) {
                    unit.end_move();
                }
                let hits = their_hp - hp;
                log.push(match hits {
                    0 => format!("Our {} bombards the enemy {}, without effect ({their_hp} of {their_max} hit points).", kind.name(), their_kind.name()),
                    _ => format!("Our {} bombards the enemy {}: {hp} of {their_max} hit points left.", kind.name(), their_kind.name()),
                });
                battles.send(BattleShown {
                    from: here,
                    at,
                    attacker: Some(entity),
                    result: if hits == 0 { "No effect".to_string() } else { format!("-{hits} HP: {hp}/{their_max}") },
                    good: Some(hits > 0),
                    ours: true,
                    attacker_side: (kind, PLAYER),
                    defender_side: (their_kind, their_owner),
                    attacker_died: false,
                    defender_died: false,
                });
                sounds.send(Sfx::Battle);
                cycle.advance = true;
            }
            Action::CycleStack | Action::CycleStackBack => {
                let here = selected_unit
                    .and_then(|e| units.get(e).ok().map(|u| u.position))
                    .or_else(|| selection.city.and_then(|e| cities.get(e).ok().map(|c| c.position)));
                let Some(here) = here else { continue };
                let stack = own_stack(here, unit_entities.iter(), |e| units.get(e).ok().map(|u| (u.position, u.owner)));
                let len = stack.len().max(1);
                let step = if *action == Action::CycleStackBack { len - 1 } else { 1 };
                let next = selected_unit.and_then(|e| stack.iter().position(|s| *s == e)).map_or(0, |i| (i + step) % len);
                if let Some(entity) = stack.get(next) {
                    selection.unit = Some(*entity);
                    selection.city = None;
                    sounds.send(Sfx::Select);
                }
            }
            // Handled by `cycle_cities`.
            Action::CycleCity(_) => {}
            Action::StackOrder(fortify) => {
                let Some(here) = selected_unit.and_then(|e| units.get(e).ok().map(|u| u.position)) else { continue };
                let stack = own_stack(here, unit_entities.iter(), |e| units.get(e).ok().map(|u| (u.position, u.owner)));
                let mut changed = 0;
                for entity in stack {
                    let Ok(mut unit) = units.get_mut(entity) else { continue };
                    // Units at work and troops aboard a ship are left as they are.
                    if unit.build_turns_left.is_some() || (unit.carrier.is_some() && map.water.contains(&unit.position)) || unit.fortified == *fortify {
                        continue;
                    }
                    unit.fortified = *fortify;
                    changed += 1;
                    if !*fortify {
                        commands.entity(entity).remove::<Sentry>();
                    }
                }
                log.push(format!("{changed} unit{} {}.", if changed == 1 { "" } else { "s" }, if *fortify { "fortified" } else { "woken: they wait for orders" }));
                cycle.advance = true;
                cycle.force = *fortify;
                dirty.ui = true;
            }
            Action::Build(improvement) => {
                let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) else { continue };
                if !unit.unit_type.is_worker() || !unit.can_move() {
                    continue;
                }
                let Some(tile) = tiles.iter().find(|t| t.coord == unit.position) else { continue };
                // Road work on a road lays a railroad, once Steam Power is known.
                let improvement = &if *improvement == Improvement::Road && tile.has_road() {
                    if tile.has_railroad() {
                        log.push("A railroad already runs here.");
                        continue;
                    }
                    if !tech.researched.contains(&Tech::SteamPower) {
                        log.push("A road already runs here. Railroads need Steam Power.");
                        continue;
                    }
                    Improvement::Railroad
                } else {
                    *improvement
                };
                if tile.terrain == Terrain::Water || tile.improvements.contains(improvement) {
                    log.push("That cannot be built here.");
                    continue;
                }
                if *improvement == Improvement::Fortress {
                    if !tech.researched.contains(&Tech::Construction) {
                        log.push("Fortresses need Construction.");
                        continue;
                    }
                    if cities.iter().any(|c| c.position == tile.coord) {
                        log.push("A city has walls for that; fortresses are built in the field.");
                        continue;
                    }
                }
                unit.start_build(*improvement);
                let (who, turns) = (unit.unit_type.name(), unit.unit_type.build_turns());
                cycle.advance = true;
                if let Some(entity) = selected_unit {
                    commands.entity(entity).remove::<GoTo>();
                }
                log.push(format!("{who} begin a {:?} ({turns} turns).", improvement));
            }
            Action::Transform => {
                let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) else { continue };
                if !unit.unit_type.is_worker() || !unit.can_move() {
                    continue;
                }
                if tiles.iter().any(|t| t.coord == unit.position && t.is_polluted()) {
                    unit.start_build(Improvement::Pollution);
                    cycle.advance = true;
                    log.push(format!("{} begin cleaning up the pollution ({} turns).", unit.unit_type.name(), unit.unit_type.build_turns()));
                    continue;
                }
                if let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == unit.position) {
                    if tile.transform_terrain() {
                        map.terrain.insert(tile.coord, tile.terrain);
                        unit.end_move();
                        cycle.advance = true;
                        log.push("The land is cleared into plains.");
                    } else {
                        log.push("This terrain cannot be transformed.");
                    }
                }
            }
            Action::FoundCity => {
                let Some(entity) = selected_unit else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                if unit.unit_type != UnitType::Settler {
                    continue;
                }
                let here = unit.position;
                let land = tiles.iter().any(|t| t.coord == here && t.terrain != Terrain::Water);
                let nearest = cities.iter().filter(|c| c.position.distance_to(here) < 2).min_by_key(|c| c.position.distance_to(here));
                if !land {
                    log.push("A city cannot be founded at sea.");
                    continue;
                }
                if let Some(near) = nearest {
                    let how = if near.position == here { "stands on this very tile" } else { "is right next door" };
                    log.push(format!("{} {how}: cities need at least one empty tile between them. Move the settler further away.", near.name));
                    continue;
                }
                let count = cities.iter().filter(|c| c.owner == PLAYER).count();
                let taken: Vec<&str> = cities.iter().map(|c| c.name.as_str()).collect();
                let own_names = civs.players.first().map(|c| c.city_names.clone()).unwrap_or_default();
                let name = own_names
                    .iter()
                    .map(String::as_str)
                    .chain(CITY_NAMES)
                    .find(|n| !taken.contains(n))
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Colony {}", count + 1));
                let city = spawn_city(&mut commands, here, name.clone(), 1, PLAYER, false);
                commands.entity(entity).try_despawn_recursive();
                selection.unit = None;
                selection.city = Some(city);
                fog.reveal(here, 3);
                log.push(format!("The city of {name} is founded."));
            }
            Action::Skip => {
                if let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) {
                    unit.end_move();
                    cycle.advance = true;
                }
            }
            Action::Diplo(civ, act) => {
                let mut ctx = foreign::Ctx { diplomacy: &mut diplomacy, foreign: &mut foreign, civs: &mut civs, state: &mut state, tech: &mut tech, log: &mut log };
                foreign::player_act(&mut ctx, *civ, *act);
            }
            Action::AnswerOffer(accept) => {
                let mut ctx = foreign::Ctx { diplomacy: &mut diplomacy, foreign: &mut foreign, civs: &mut civs, state: &mut state, tech: &mut tech, log: &mut log };
                foreign::answer_offer(&mut ctx, *accept);
            }
            Action::TradeRoute => {
                let caravan = selected_unit.filter(|e| units.get(*e).is_ok_and(|u| u.unit_type == UnitType::Caravan));
                let Some(entity) = caravan else {
                    log.push("Trade routes are opened by caravans: train one, move it into another city (yours at least 6 tiles away, or a foreign one at peace) and press T.");
                    continue;
                };
                let Ok(unit) = units.get(entity) else { continue };
                let here = unit.position;
                // The city it stands in, or a foreign city beside it (defended cities cannot be entered).
                let far = cities
                    .iter()
                    .find(|c| c.position == here)
                    .or_else(|| cities.iter().filter(|c| c.owner != PLAYER && c.position.distance_to(here) == 1).min_by_key(|c| c.population))
                    .map(|c| (c.position, c.owner, c.population, c.name.clone()));
                let Some(far) = far else {
                    log.push("A caravan opens a trade route at another city: move it into one of yours, or next to a foreign one.");
                    continue;
                };
                if far.1 == BARBARIANS || (far.1 != PLAYER && diplomacy.get_treaty(PLAYER, far.1) == Treaty::War) {
                    log.push("No trade with those we are at war with.");
                    continue;
                }
                // From the caravan's home city, or the nearest city of ours if that is gone.
                let home = homes.get(entity).ok().map(|h| h.0).filter(|h| *h != here && cities.iter().any(|c| c.position == *h && c.owner == PLAYER));
                let home = home.or_else(|| cities.iter().filter(|c| c.owner == PLAYER && c.position != here).min_by_key(|c| c.position.distance_to(here)).map(|c| c.position));
                let Some(home) = home.and_then(|h| cities.iter().find(|c| c.position == h).map(|c| (c.position, c.population, c.name.clone()))) else {
                    log.push("A trade route needs a second city of ours for the caravan to come from.");
                    continue;
                };
                match trade.establish((home.0, PLAYER, home.1), (far.0, far.1, far.2)) {
                    Ok(route) => {
                        log.push(format!(
                            "A trade route opens between {} and {}: +{} gold and +{} science a turn{}.",
                            home.2,
                            far.3,
                            route.gold_per_turn,
                            route.science_per_turn,
                            if route.is_foreign() { " (they earn from it too)" } else { "" }
                        ));
                        commands.entity(entity).try_despawn_recursive();
                        selection.unit = None;
                        cycle.advance = true;
                        sounds.send(Sfx::Select);
                    }
                    Err(reason) => log.push(reason),
                }
            }
            Action::Spy(mission) => {
                let spy = selected_unit.filter(|e| units.get(*e).is_ok_and(|u| u.unit_type == UnitType::Diplomat));
                let Some(entity) = spy else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                let here = unit.position;
                let target = spy_target(here, cities.iter().map(|c| (c.position, c.owner)));
                let Some((at, victim)) = target else {
                    log.push("A diplomat works at a foreign city: move it next to one first.");
                    continue;
                };
                let city_name = cities.iter().find(|c| c.position == at).map(|c| c.name.clone()).unwrap_or_default();
                let city_walls: HashSet<Hex> = walled
                    .iter()
                    .filter_map(|(e, b)| cities.get(e).ok().filter(|c| b.has(Building::Walls) || wonders.walls_everywhere(c.owner)).map(|c| c.position))
                    .collect();
                let Some(civ_index) = civs.players.iter().position(|c| c.id == victim) else { continue };
                let civ_name = civs.players[civ_index].name.trim_start_matches("the ").to_string();
                if *mission == Mission::Embassy {
                    if spies.has_embassy(PLAYER, victim) {
                        log.push(format!("We already have an embassy with {civ_name}."));
                        continue;
                    }
                    spies.embassies.insert((PLAYER, victim));
                    foreign.met.insert(victim);
                    for city in cities.iter().filter(|c| c.owner == victim) {
                        fog.reveal(city.position, 2);
                    }
                    let theirs = &civs.players[civ_index];
                    log.push(format!(
                        "Our embassy in {city_name} reports: {civ_name} know {} technologies, are researching {} and hold {} gold. Their cities are now on our maps.",
                        theirs.tech.researched.len(),
                        theirs.tech.current.map(|t| t.name()).unwrap_or("nothing"),
                        theirs.gold
                    ));
                } else {
                    let loot = espionage::stealable(&tech, &civs.players[civ_index].tech);
                    if *mission == Mission::StealTech && loot.is_empty() {
                        log.push(format!("{civ_name} know nothing we do not."));
                        continue;
                    }
                    let mut rng = rand::thread_rng();
                    if rng.gen::<f32>() < mission.odds(city_walls.contains(&at)) {
                        match mission {
                            Mission::StealTech => {
                                let stolen = loot[rng.gen_range(0..loot.len())];
                                tech.learn(stolen);
                                log.push(format!("Our diplomat steals the secret of {} from {city_name}!", stolen.name()));
                            }
                            Mission::Sabotage => {
                                if let Some(mut city) = cities.iter_mut().find(|c| c.position == at) {
                                    log.push(format!("Our diplomat sabotages {city_name}: {} shields of work are lost.", city.production_progress));
                                    city.production_progress = 0;
                                }
                            }
                            Mission::StealGold => {
                                let amount = espionage::stolen_gold(civs.players[civ_index].gold);
                                civs.players[civ_index].gold -= amount;
                                state.gold += amount;
                                log.push(format!("Our diplomat makes off with {amount} gold from {city_name}."));
                            }
                            Mission::InciteUnrest => {
                                spies.incite(at);
                                log.push(format!("Our diplomat stirs up {city_name}: it stands idle for {} turns.", espionage::UNREST_TURNS));
                            }
                            Mission::StageIncident => {
                                foreign.add_grievance(PLAYER, victim, foreign::Wrong::BorderIncident, state.turn);
                                log.push(format!(
                                    "Our agents stage a clash near {city_name} that the world blames on {civ_name}. We have a just cause for war against them for {} turns.",
                                    foreign::GRIEVANCE_TURNS
                                ));
                            }
                            Mission::Frame(framed) => {
                                let framed_name = civs.players.iter().find(|c| c.id == *framed).map(|c| c.name.clone()).unwrap_or_default();
                                diplomacy.change_attitude(victim, *framed, espionage::FRAMED_ATTITUDE);
                                foreign.add_grievance(victim, *framed, foreign::Wrong::BorderIncident, state.turn);
                                log.push(format!("Our agents strike in {city_name} disguised as {framed_name}: {civ_name} blame {framed_name} and want revenge."));
                            }
                            Mission::Embassy => {}
                        }
                    } else {
                        diplomacy.change_attitude(PLAYER, victim, espionage::CAUGHT_ATTITUDE);
                        foreign.add_grievance(victim, PLAYER, foreign::Wrong::SpyCaught, state.turn);
                        if mission.is_false_flag() {
                            // Exposed: worse than an honest war. The world hears of it, and a framed
                            // people takes it as an attack too.
                            foreign.reputation -= espionage::EXPOSED_REPUTATION;
                            for other in civs.players.iter().filter(|c| !c.is_human && !c.eliminated && c.id != victim).map(|c| c.id).collect::<Vec<_>>() {
                                diplomacy.change_attitude(PLAYER, other, espionage::EXPOSED_ATTITUDE);
                            }
                            if let Mission::Frame(framed) = mission {
                                diplomacy.change_attitude(PLAYER, *framed, espionage::CAUGHT_ATTITUDE);
                                foreign.add_grievance(*framed, PLAYER, foreign::Wrong::SpyCaught, state.turn);
                            }
                            log.push(format!("Our false flag in {city_name} is exposed! {civ_name} are outraged, our reputation suffers, and the world will not soon trust us."));
                        } else {
                            log.push(format!("Our diplomat is caught in {city_name}! {civ_name} are outraged - and now have a just cause for war against us."));
                        }
                    }
                }
                commands.entity(entity).try_despawn_recursive();
                selection.unit = None;
                cycle.advance = true;
                sounds.send(Sfx::Select);
            }

            Action::Save => {
                saves.send(savegame::SaveRequest { download: true });
            }
            Action::QueueProduction(item) => {
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                if city.owner != PLAYER {
                    continue;
                }
                if city.production_queue.is_none() {
                    city.production_queue = Some(item.clone());
                } else if city.queue.len() >= bevy_civ::city::QUEUE_LIMIT {
                    log.push(format!("{}'s build queue is full ({} projects).", city.name, bevy_civ::city::QUEUE_LIMIT));
                } else if matches!(item, ProductionItem::Building(_)) && (city.queue.contains(item) || city.production_queue.as_ref() == Some(item)) {
                    log.push(format!("The {} is already on order in {}.", item.name(), city.name));
                } else {
                    city.queue.push(item.clone());
                }
            }
            Action::Undo => {
                let Some((entity, before)) = last_move.0.take() else {
                    log.push("There is no move to take back.");
                    continue;
                };
                if let Ok(mut unit) = units.get_mut(entity) {
                    if unit.owner == PLAYER {
                        *unit = before;
                        commands.entity(entity).remove::<GoTo>();
                        selection.unit = Some(entity);
                        selection.city = None;
                        log.push("Move taken back.");
                    }
                }
            }
            Action::Draft => {
                let Some(city_entity) = selection.city else { continue };
                if !state.government.allows_draft() || !tech.researched.contains(&Tech::Gunpowder) {
                    log.push("Drafting needs Gunpowder and a Monarchy, Republic, Communism or Fundamentalism.");
                    continue;
                }
                let Ok(mut city) = cities.get_mut(city_entity) else { continue };
                if city.owner != PLAYER || city.population < 2 || state.drafted.contains(&city.position) {
                    log.push(format!("{} cannot draft now (size 2 or more, once a turn).", city.name));
                    continue;
                }
                let at = city.position;
                let score = |u: &UnitType| u.stats().1 as f32 + u.stats().0 as f32 * 0.1;
                let Some(recruit) = UnitType::ALL
                    .into_iter()
                    .filter(|u| u.is_army() && u.is_trainable() && u.required_tech().is_none_or(|t| tech.researched.contains(&t)))
                    .filter(|u| u.required_wonder().is_none() && supplies.has(at, u.required_resources()))
                    .max_by(|a, b| score(a).total_cmp(&score(b)))
                else {
                    continue;
                };
                city.population -= 1;
                state.drafted.insert(at);
                let drafted = spawn_unit(&mut commands, at, recruit, PLAYER);
                commands.entity(drafted).insert(HomeCity(at));
                log.push(format!("{} drafts a citizen into a {} (size {} now).", city.name, recruit.name(), city.population));
            }
            Action::Surrender => {
                if state.victory.is_none() {
                    state.victory = Some(format!("Defeat: we surrendered on turn {}.", state.turn));
                    log.push("We lay down our arms. The game is over.");
                }
            }
            Action::LaunchShip => {
                if let Some(arrives) = space.launch(PLAYER, state.turn) {
                    log.push(format!("Our spaceship lifts off for {}, {} light-years away! It arrives on turn {arrives} - unless our capital falls.", bevy_civ::space::destination(), bevy_civ::space::LIGHT_YEARS));
                }
            }
            Action::SetFocus(focus) => {
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                if city.owner == PLAYER {
                    city.focus = *focus;
                    // The governor works the land afresh for the new focus.
                    city.worked.clear();
                    city.pinned.clear();
                    dirty.ui = true;
                }
            }
            Action::AutoTiles => {
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                if city.owner == PLAYER {
                    city.worked.clear();
                    city.pinned.clear();
                }
            }
            Action::Unqueue(index) => {
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                if city.owner == PLAYER && *index < city.queue.len() {
                    city.queue.remove(*index);
                }
            }
            Action::Pillage => {
                let Some(entity) = selected_unit else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                if !unit.unit_type.is_army() || !unit.can_move() {
                    log.push("Only troops with moves left can pillage.");
                    continue;
                }
                let here = unit.position;
                if cities.iter().any(|c| c.position == here) {
                    log.push("Cities cannot be pillaged; capture them instead.");
                    continue;
                }
                let owner = supplies.territory.get(&here).map(|(o, _)| *o);
                match owner {
                    Some(PLAYER) => {
                        log.push("We will not pillage our own land.");
                        continue;
                    }
                    Some(o) if o != BARBARIANS && diplomacy.get_treaty(PLAYER, o) != Treaty::War => {
                        log.push("Pillaging their land would break the peace: declare war first.");
                        continue;
                    }
                    _ => {}
                }
                let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == here) else { continue };
                let Some(target) = bevy_civ::terrain::pillage_target(&tile.improvements) else {
                    log.push("There is nothing here to pillage.");
                    continue;
                };
                tile.improvements.retain(|i| *i != target);
                state.gold += 5;
                if let Ok(mut unit) = units.get_mut(entity) {
                    unit.end_move();
                }
                log.push(format!("Our troops wreck the {} here and carry off 5 gold.", bevy_civ::terrain::improvement_name(target)));
                cycle.advance = true;
                sounds.send(Sfx::Battle);
            }
            Action::SetProduction(item) => {
                if let ProductionItem::Building(b) = item {
                    if b.one_per_people() && walled.iter().any(|(e, built)| built.has(*b) && cities.get(e).is_ok_and(|c| c.owner == PLAYER)) {
                        log.push(format!("We already have a {}.", b.name()));
                        continue;
                    }
                }
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                let missing = supplies.missing(city.position, item.required_resources());
                if city.owner != PLAYER {
                    continue;
                }
                if let ProductionItem::Building(part) = item {
                    if part.is_spaceship_part() && !selection.city.and_then(|e| walled.get(e).ok()).is_some_and(|(_, b)| b.has(Building::Factory)) {
                        log.push("Spaceship parts are built in cities with a Factory.");
                        continue;
                    }
                    if let Some(wonder) = part.required_wonder().filter(|w| !wonders.built.contains_key(w)) {
                        log.push(format!("No people can build a {} until the {} stands somewhere in the world.", part.name(), wonder.name()));
                        continue;
                    }
                    if let Some(needed) = part.required_building() {
                        if !selection.city.and_then(|e| walled.get(e).ok()).is_some_and(|(_, b)| b.has(needed)) {
                            log.push(format!("A {} needs a {} in the city first.", part.name(), needed.name()));
                            continue;
                        }
                    }
                }
                if let ProductionItem::Building(wonder) = item {
                    if let Some((_, place)) = wonder.is_wonder().then(|| wonders.built.get(wonder)).flatten() {
                        log.push(format!("The {} already stands in {}; each wonder can be built only once.", wonder.name(), place));
                        continue;
                    }
                }
                let needs_coast = matches!(item, ProductionItem::Building(b) if b.needs_coast()) || matches!(item, ProductionItem::Unit(u) if u.domain() == Domain::Sea);
                if needs_coast && !city.position.all_neighbors().iter().any(|n| map.water.contains(n)) {
                    log.push(format!("{} is not on the coast and cannot build a {}.", city.name, item.name()));
                    continue;
                }
                if missing.is_empty() {
                    city.production_queue = Some(item.clone());
                    log.push(format!("{} begins work on {}.", city.name, item.name()));
                } else {
                    let names: Vec<&str> = missing.iter().map(|g| g.name()).collect();
                    log.push(format!("{} cannot build {}: it has no access to {}.", city.name, item.name(), names.join(" or ")));
                }
            }
            Action::Research(target) => {
                if tech.can_research(*target) && tech.current != Some(*target) {
                    tech.current = Some(*target);
                    // Studying something off the way to the goal sets the goal aside.
                    if tech.goal.is_some_and(|g| !tech.path_to(g).contains(target)) {
                        tech.goal = None;
                    }
                    log.push(format!("Our scholars turn to {}.", target.name()));
                }
            }
            Action::ResearchGoal(goal) => {
                if !tech.researched.contains(goal) {
                    tech.set_goal(*goal);
                    let steps = tech.path_to(*goal).len();
                    log.push(format!(
                        "Our scholars work towards {} ({steps} discover{} away), starting with {}.",
                        goal.name(),
                        if steps == 1 { "y" } else { "ies" },
                        tech.current.map(|t| t.name()).unwrap_or("nothing")
                    ));
                }
            }
            Action::ClearResearchGoal => {
                tech.goal = None;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_turn(
    mut actions: EventReader<EndTurnNow>,
    mut state: ResMut<GameState>,
    mut tech: ResMut<TechState>,
    mut cities: Query<(&mut City, &mut CityBuildings, &mut Happiness)>,
    mut units: Query<(Entity, &mut Unit)>,
    mut tiles: Query<&mut HexTile>,
    mut civs: ResMut<Civilizations>,
    mut fog: ResMut<FogOfWar>,
    mut barbarians: ResMut<BarbarianState>,
    (mut diplomacy, mut foreign): (ResMut<Diplomacy>, ResMut<Foreign>),
    (mut trade, mut map, supplies, mut religions): (ResMut<TradeRoutes>, ResMut<MapData>, Res<Supplies>, ResMut<Religions>),
    (mut space, mut wonders, mut great, mut spies): (ResMut<SpaceRace>, ResMut<WorldWonders>, ResMut<GreatPeople>, ResMut<Espionage>),
    (mut log, mut dirty, mut cycle, mut tally): (ResMut<GameLog>, ResMut<Dirty>, ResMut<Cycle>, ResMut<combat::BattleTally>),
    (mut battles, mut strikes, sentries): (EventWriter<BattleShown>, EventWriter<Detonation>, Query<Entity, With<Sentry>>),
    (homes, mut history, mut culture, mut golden, mut taken, rivers, mut saves, observer, mut ferries): (
        Query<&HomeCity>,
        ResMut<History>,
        ResMut<CultureLedger>,
        ResMut<GoldenAges>,
        ResMut<JustCaptured>,
        Res<Rivers>,
        EventWriter<savegame::SaveRequest>,
        Option<Res<Observer>>,
        ResMut<bevy_civ::ai::Ferries>,
    ),
    mut commands: Commands,
) {
    if actions.read().count() == 0 || state.victory.is_some() {
        return;
    }
    dirty.map = true;
    dirty.ui = true;
    state.turn += 1;
    state.drafted.clear();
    state.sold.clear();
    log.turn = state.turn;
    spies.tick();
    golden.tick();
    // Knowledge spreads: a technology is cheaper to study the more of the other peoples know it -
    // of those we have met, for us - down to half price once all of them do.
    {
        let holding: HashSet<u32> = cities.iter().map(|(c, _, _)| c.owner).collect();
        let knowledge = peoples_knowledge(&civs, &tech, &holding);
        let others = |me: u32| knowledge.iter().filter(move |(id, _)| *id != me);
        for civ in civs.players.iter_mut().filter(|c| !c.is_human) {
            let id = civ.id;
            civ.tech.set_known_abroad(|t| others(id).filter(|(_, k)| k.contains(&t)).count() as u32, others(id).count() as u32);
        }
        let met = &foreign.met;
        tech.set_known_abroad(|t| others(PLAYER).filter(|(id, k)| met.contains(id) && k.contains(&t)).count() as u32, others(PLAYER).count() as u32);
    }
    // Wonders their owners have outgrown are obsolete: their effects end (their culture goes on).
    for (wonder, owner) in wonders.refresh_expired(|owner, t| if owner == PLAYER { tech.researched.contains(&t) } else { civs.players.iter().any(|c| c.id == owner && c.tech.researched.contains(&t)) }) {
        if owner == PLAYER {
            let by = wonder.obsoleted_by().map(|t| t.name()).unwrap_or_default();
            log.push(format!("Now that we know {by}, our {} is obsolete: its effect ends, though its culture goes on.", wonder.name()));
        }
    }
    // The chance to raze last turn's conquests has passed; their former owners hold it against us.
    taken.0.clear();
    for loser in std::mem::take(&mut taken.1) {
        if loser != BARBARIANS {
            foreign.add_grievance(loser, PLAYER, foreign::Wrong::CityTaken, state.turn);
        }
    }
    // Routes close when a city changes hands or their two owners are at war.
    {
        let owners: HashMap<Hex, u32> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();
        let names: HashMap<Hex, String> = cities.iter().map(|(c, _, _)| (c.position, c.name.clone())).collect();
        for route in trade.prune(|h| owners.get(&h).copied(), |a, b| diplomacy.get_treaty(a, b) == Treaty::War) {
            if route.owner == PLAYER || route.partner == PLAYER {
                let name = |h: Hex| names.get(&h).cloned().unwrap_or_else(|| "a lost city".into());
                log.push(format!("The trade route between {} and {} has closed.", name(route.from), name(route.to)));
            }
        }
    }

    // (A game saved before peoples kept the culture they earned credits each city's to its owner.)
    culture.seed(&cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect::<Vec<_>>());
    let wonders_taken: HashSet<Building> = wonders.built.keys().copied().collect();
    // Caravans and diplomats each civilization has, and the cities building wonders.
    let mut envoy_counts: HashMap<(u32, UnitType), usize> = HashMap::new();
    for (_, unit) in units.iter().filter(|(_, u)| matches!(u.unit_type, UnitType::Caravan | UnitType::Diplomat | UnitType::Missionary)) {
        *envoy_counts.entry((unit.owner, unit.unit_type)).or_default() += 1;
    }
    // Civilizations with free land for new cities near their own (at least 3 tiles from any city,
    // within 10 of one of theirs - or 20, for a seafaring people that can ship settlers there).
    let room: HashSet<u32> = {
        let city_spots: Vec<(Hex, u32)> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();
        let free: Vec<Hex> = tiles
            .iter()
            .filter(|t| t.terrain != Terrain::Water && city_spots.iter().all(|(p, _)| p.distance_to(t.coord) >= 3))
            .map(|t| t.coord)
            .collect();
        civs.players
            .iter()
            .filter(|c| {
                let reach = if c.tech.researched.contains(&Tech::Seafaring) { 20 } else { 10 };
                free.iter().any(|f| city_spots.iter().any(|(p, o)| *o == c.id && p.distance_to(*f) <= reach))
            })
            .map(|c| c.id)
            .collect()
    };
    // A computer civilization hemmed in on a small island studies seafaring first, to reach new land.
    for civ in civs.players.iter_mut().filter(|c| c.id != PLAYER) {
        let owned = cities.iter().filter(|(c, _, _)| c.owner == civ.id).count();
        if owned > 0 && owned < 4 && state.turn > 20 && civ.tech.can_research(Tech::Seafaring) {
            civ.tech.current = Some(Tech::Seafaring);
        }
    }
    // Spaceship parts each civilization still needs (one city per part kind at a time is enough).
    let space_wanted: HashMap<u32, Vec<Building>> = civs
        .players
        .iter()
        .map(|c| (c.id, [Building::SpaceStructure, Building::SpaceComponent, Building::SpaceModule].into_iter().filter(|p| p.required_tech().is_some_and(|t| c.tech.researched.contains(&t)) && space.wants(c.id, *p, false)).collect()))
        .collect();
    let wonder_sites: Vec<(Hex, u32)> = cities
        .iter()
        .filter(|(c, _, _)| matches!(&c.production_queue, Some(ProductionItem::Building(b)) if b.is_wonder()))
        .map(|(c, _, _)| (c.position, c.owner))
        .collect();
    let mut rng = rand::thread_rng();
    let gov = state.government;
    // Each civilization lives under its own government. Computer players adopt the best they know.
    let mut governments: HashMap<u32, Government> = HashMap::from([(PLAYER, state.government)]);
    for civ in civs.players.iter_mut().filter(|c| !c.is_human) {
        // Computer players pay for a revolution with anarchy too.
        if civ.anarchy > 0 {
            civ.anarchy -= 1;
            if civ.anarchy == 0 {
                if let Some(next) = civ.pending_government.take() {
                    civ.government = next;
                    if foreign.met.contains(&civ.id) {
                        log.push(format!("{} now live under {}.", capitalised(&civ.name), next.name()));
                    }
                }
            }
        } else {
            let preferred = Government::preferred(&civ.tech, civ.aggression);
            if preferred != civ.government {
                if wonders.owns(civ.id, Building::Liberty) {
                    civ.government = preferred;
                } else {
                    civ.government = Government::Anarchy;
                    civ.anarchy = if bevy_civ::traits::of(&civ.name).has(bevy_civ::traits::Trait::Religious) { Government::REVOLUTION_TURNS - 1 } else { Government::REVOLUTION_TURNS };
                    civ.pending_government = Some(preferred);
                }
            }
        }
        governments.insert(civ.id, civ.government);
    }
    let mut total_science = 0;
    let mut total_gold = 0i32;
    let mut player_cities = 0;

    // A people that has lost its capital moves the palace to its largest remaining city.
    {
        let with_capital: HashSet<u32> = cities.iter().filter(|(c, _, _)| c.is_capital).map(|(c, _, _)| c.owner).collect();
        let mut largest: HashMap<u32, (Hex, u32)> = HashMap::new();
        for (city, _, _) in cities.iter().filter(|(c, _, _)| c.owner != BARBARIANS && !with_capital.contains(&c.owner)) {
            let best = largest.entry(city.owner).or_insert((city.position, city.population));
            if city.population > best.1 || (city.population == best.1 && (city.position.x, city.position.y) < (best.0.x, best.0.y)) {
                *best = (city.position, city.population);
            }
        }
        for (owner, (at, _)) in largest {
            if let Some((mut city, _, _)) = cities.iter_mut().find(|(c, _, _)| c.position == at) {
                city.is_capital = true;
                if owner == PLAYER {
                    log.push(format!("{} is our new capital.", city.name));
                }
            }
        }
    }
    let capitals: HashMap<u32, Hex> =
        cities.iter().filter(|(c, _, _)| c.is_capital).map(|(c, _, _)| (c.owner, c.position)).collect();
    // Seats of government: the capital and a Forbidden Palace. Corruption counts from the nearer.
    let mut seats: HashMap<u32, Vec<Hex>> = HashMap::new();
    for (city, _, _) in cities.iter().filter(|(c, b, _)| c.is_capital || b.has(Building::ForbiddenPalace)) {
        seats.entry(city.owner).or_default().push(city.position);
    }
    // The share of every city's trade lost to corruption (by distance to its palace and the number
    // of cities nearer, under its people's government).
    let corruption_of = corruption_rates(
        cities.iter().map(|(c, b, _)| (c, b)),
        |owner| governments.get(&owner).copied().unwrap_or_default(),
        &supplies.linked,
        map.cells.len().saturating_sub(map.water.len()),
    );
    // Each people's rates this turn: the player's as set, computer players' as they chose last turn.
    let rates_of: HashMap<u32, bevy_civ::economy::Rates> = civs
        .players
        .iter()
        .map(|c| (c.id, if c.id == PLAYER { state.rates() } else { c.rates.fit(governments.get(&c.id).copied().unwrap_or_default()) }))
        .collect();
    // Each people's accounts this turn; what its cities would raise at a 100% tax rate, its gold
    // that does not depend on the rates, and its cities in disorder (for computer players' rates).
    let mut accounts: HashMap<u32, bevy_civ::economy::Ledger> = HashMap::new();
    let mut full_taxes: HashMap<u32, u32> = HashMap::new();
    let mut fixed_income: HashMap<u32, i32> = HashMap::new();
    let mut rioting: HashMap<u32, usize> = HashMap::new();
    // Peoples with a state religion (their capital's faith) and own cities that do not follow it yet.
    let missionary_work: HashSet<u32> = {
        let faith_of_capital: HashMap<u32, Faith> = capitals.iter().filter_map(|(o, cp)| religions.of(*cp).map(|f| (*o, f))).collect();
        cities.iter().filter(|(c, _, _)| faith_of_capital.get(&c.owner).is_some_and(|f| religions.of(c.position) != Some(*f))).map(|(c, _, _)| c.owner).collect()
    };
    // A sprawling empire is harder to keep content: past 12 cities, every 6 more make one more
    // citizen unhappy in each city.
    let mut empire_sizes: HashMap<u32, u32> = HashMap::new();
    for (city, _, _) in cities.iter() {
        *empire_sizes.entry(city.owner).or_default() += 1;
    }
    let mut army_sizes: HashMap<u32, usize> = HashMap::new();
    let mut worker_counts: HashMap<u32, usize> = HashMap::new();
    let mut navy_sizes: HashMap<u32, usize> = HashMap::new();
    let mut ferry_counts: HashMap<u32, usize> = HashMap::new();
    let mut air_sizes: HashMap<u32, usize> = HashMap::new();
    for (_, unit) in units.iter() {
        if unit.unit_type.is_army() {
            *army_sizes.entry(unit.owner).or_default() += 1;
        } else if unit.unit_type.is_ferry() {
            *ferry_counts.entry(unit.owner).or_default() += 1;
        } else if unit.unit_type.domain() == Domain::Sea {
            *navy_sizes.entry(unit.owner).or_default() += 1;
        } else if unit.unit_type.domain() == Domain::Air {
            *air_sizes.entry(unit.owner).or_default() += 1;
        } else if unit.unit_type.is_worker() {
            *worker_counts.entry(unit.owner).or_default() += 1;
        }
    }
    let mut settler_counts: HashMap<u32, usize> = HashMap::new();
    for (_, unit) in units.iter().filter(|(_, u)| u.unit_type == UnitType::Settler) {
        *settler_counts.entry(unit.owner).or_default() += 1;
    }
    // Where every army stands, who is at war, and how much road work each computer player has left.
    let armies: Vec<(Hex, u32)> = units.iter().filter(|(_, u)| u.unit_type.is_army()).map(|(_, u)| (u.position, u.owner)).collect();
    let wars: HashSet<(u32, u32)> = diplomacy.treaties.iter().filter(|(_, t)| **t == Treaty::War).map(|(pair, _)| *pair).collect();
    let enemies = |a: u32, b: u32| a != b && (a == BARBARIANS || b == BARBARIANS || wars.contains(&(a.min(b), a.max(b))));
    let mut road_work: HashMap<u32, usize> = HashMap::new();
    for owner in supplies.unconnected.values() {
        *road_work.entry(*owner).or_default() += 1;
    }
    // Science and gold the computer players' cities bring in this turn.
    let mut cut_off: HashMap<u32, usize> = HashMap::new();
    for (city, _, _) in cities.iter().filter(|(c, _, _)| !supplies.linked.contains(&c.position)) {
        *cut_off.entry(city.owner).or_default() += 1;
    }
    let mut ai_income: HashMap<u32, (u32, i32)> = HashMap::new();
    // Banks and Stock Exchanges each civilization owns (they decide its interest).
    let mut finance: HashMap<u32, (u32, u32)> = HashMap::new();
    // Each civilization's traits and unique unit.
    let characters: HashMap<u32, bevy_civ::traits::Character> = civs.players.iter().map(|c| (c.id, bevy_civ::traits::of(&c.name))).collect();
    let mut hurried: HashMap<u32, u32> = HashMap::new();
    let mut city_counts: HashMap<u32, usize> = HashMap::new();
    for (city, _, _) in cities.iter() {
        *city_counts.entry(city.owner).or_default() += 1;
        // Workers and settlers already on order count, so cities do not all train the same thing.
        match city.production_queue {
            Some(ProductionItem::Unit(UnitType::Worker | UnitType::Engineers)) => *worker_counts.entry(city.owner).or_default() += 1,
            Some(ProductionItem::Unit(UnitType::Settler)) => *settler_counts.entry(city.owner).or_default() += 1,
            _ => {}
        }
    }

    // War weariness: each civilization's count of turns at war goes up while it fights anyone.
    {
        let ids: Vec<u32> = civs.players.iter().map(|c| c.id).collect();
        for id in &ids {
            let fighting = ids.iter().any(|other| other != id && diplomacy.get_treaty(*id, *other) == Treaty::War);
            let turns = state.war_turns.entry(*id).or_default();
            *turns = if fighting { *turns + 1 } else { turns.saturating_sub(2) };
        }
    }
    let war_turns = state.war_turns.clone();

    // The troubles of the modern city (see `bevy_civ::troubles`): who knows enough to have them,
    // and where drugs are rife (smugglers carry them across borders, unless a crackdown was agreed).
    let knows_tech = |owner: u32, t: Tech| if owner == PLAYER { tech.researched.contains(&t) } else { civs.players.iter().any(|c| c.id == owner && c.tech.researched.contains(&t)) };
    let urban: HashSet<u32> = civs.players.iter().map(|c| c.id).filter(|o| knows_tech(*o, Tech::Sanitation)).collect();
    let industrial: HashSet<u32> = civs.players.iter().map(|c| c.id).filter(|o| knows_tech(*o, Tech::Industrialization)).collect();
    let drug_dens: Vec<(Hex, u32)> = cities.iter().filter(|(c, _, _)| c.narcotics >= 2).map(|(c, _, _)| (c.position, c.owner)).collect();
    let crackdown = foreign.crackdown.clone();
    let our_crackdown = foreign.our_crackdown.clone();
    let turn_now = state.turn;
    // Smugglers from a drug-ridden foreign city nearby - unless their rulers promised to stop them.
    let smuggled_into = |at: Hex, owner: u32| {
        drug_dens
            .iter()
            .find(|(p, o)| {
                let stopped = (owner == PLAYER && crackdown.get(o).is_some_and(|until| *until > turn_now))
                    || (*o == PLAYER && our_crackdown.get(&owner).is_some_and(|until| *until > turn_now));
                *o != owner && p.distance_to(at) <= bevy_civ::troubles::SMUGGLING_RANGE && !stopped
            })
            .map(|(_, o)| *o)
    };
    let mut smugglers: HashSet<u32> = HashSet::new();
    let mut smuggled_by_us: HashSet<u32> = HashSet::new();
    // Modern raiders already about, the tiles others hold, and who has the media age's grievances.
    let raiders: Vec<(Hex, UnitType)> = units.iter().filter(|(_, u)| u.owner == BARBARIANS && u.unit_type.is_raider()).map(|(_, u)| (u.position, u.unit_type)).collect();
    let held_tiles: HashSet<Hex> = units.iter().filter(|(_, u)| u.owner != BARBARIANS).map(|(_, u)| u.position).collect();
    let media_age: HashSet<u32> = civs.players.iter().map(|c| c.id).filter(|o| knows_tech(*o, Tech::Electronics)).collect();
    let mut uprisings: Vec<(Hex, u32, UnitType, String)> = Vec::new();

    for (mut city, mut buildings, mut happiness) in &mut cities {
        let mine = city.owner == PLAYER;
        if mine {
            player_cities += 1;
        }
        // Corruption takes a share of the city's trade, waste a share of its shields.
        let corruption = corruption_of.get(&city.position).copied().unwrap_or(0.0);
        let waste = bevy_civ::economy::waste(corruption);

        // Drugs, food scandals and distrust of engineered food.
        {
            let owner = city.owner;
            if urban.contains(&owner) || city.narcotics > 0 {
                let smuggler = smuggled_into(city.position, owner);
                if mine {
                    if let Some(s) = smuggler {
                        smugglers.insert(s);
                    }
                } else if smuggler == Some(PLAYER) {
                    smuggled_by_us.insert(owner);
                }
                let restless = happiness.unhappy > happiness.happy;
                let before = city.narcotics;
                // Cartel gunmen nearby keep the police from rooting the trade out.
                let cartel_near = raiders.iter().any(|(p, k)| *k == UnitType::Cartel && p.distance_to(city.position) <= 2);
                let policed = buildings.has(Building::PoliceStation) && !cartel_near;
                city.narcotics = bevy_civ::troubles::narcotics_turn(before, city.population, restless, smuggler.is_some(), policed, &mut rng);
                if mine && city.narcotics > before {
                    log.push(format!(
                        "A drug trade takes hold in {} (level {} of {}): its people grow restless and the black market takes its cut. A Police Station roots it out.",
                        city.name,
                        city.narcotics,
                        bevy_civ::troubles::NARCOTICS_MAX
                    ));
                } else if mine && city.narcotics < before && city.narcotics == 0 {
                    log.push(format!("The police have cleared the drug trade out of {}.", city.name));
                }
            }
            // Modern raiders: cartel gunmen from a rife drug trade, a terror cell from grievance.
            if owner != BARBARIANS && raiders.iter().filter(|(p, _)| p.distance_to(city.position) <= 4).count() < bevy_civ::barbarians::RAIDERS_NEAR {
                let police = buildings.has(Building::PoliceStation);
                if rng.gen_bool(bevy_civ::barbarians::cartel_chance(city.narcotics, police)) {
                    uprisings.push((city.position, owner, UnitType::Cartel, "its drug trade".to_string()));
                } else if media_age.contains(&owner) {
                    let state_faith = capitals.get(&owner).and_then(|cp| religions.of(*cp));
                    let grievance = bevy_civ::barbarians::Grievance {
                        disorder: happiness.in_disorder(),
                        occupied: city.resistance > 0,
                        foreign_faith: religions.of(city.position).is_some_and(|f| state_faith.is_some_and(|s| s != f)),
                        war_weary: war_turns.get(&owner).copied().unwrap_or(0) >= 15,
                    };
                    if rng.gen_bool(bevy_civ::barbarians::terror_chance(&grievance, police)) {
                        uprisings.push((city.position, owner, UnitType::Insurgent, grievance.reason().to_string()));
                    }
                }
            }
            city.scandal = city.scandal.saturating_sub(1);
            if industrial.contains(&owner) && !buildings.has(Building::FoodSafety) && city.scandal == 0 {
                let chance = bevy_civ::troubles::scandal_chance(city.population, corruption);
                if rng.gen_bool(chance) {
                    city.scandal = bevy_civ::troubles::SCANDAL_TURNS;
                    city.food_storage /= 2;
                    if mine {
                        log.push(format!(
                            "Adulterated food in {}! Dealers have cut the bread with chalk and the milk with water: half the granary is spoiled and the people are angry. A Food Safety Agency prevents it.",
                            city.name
                        ));
                    }
                }
            }
        }

        // What a computer player's governor sees of this city (the Pyramids count as its Granary).
        let owner = city.owner;
        let gov = governments.get(&owner).copied().unwrap_or(Government::Despotism);
        let has_pyramids = wonders.owns(owner, Building::Pyramids);
        let governed_buildings = |b: &CityBuildings| {
            let mut built = b.buildings.clone();
            if has_pyramids && !built.contains(&Building::Granary) {
                built.push(Building::Granary);
            }
            built
        };
        let view = governor::CityView {
            position: city.position,
            population: city.population,
            built: governed_buildings(&buildings),
            coastal: city.position.all_neighbors().iter().any(|n| map.water.contains(n)),
            linked: supplies.linked.contains(&city.position),
            defended: armies.iter().any(|(p, o)| *p == city.position && *o == owner),
            threatened: armies.iter().any(|(p, o)| enemies(owner, *o) && p.distance_to(city.position) <= 3),
            near_capital: capitals.get(&owner).is_none_or(|cp| cp.distance_to(city.position) <= 6),
            narcotics: city.narcotics,
            trade: city.land.2,
            corruption: (corruption * 100.0).round() as u32,
            water_limit: Building::size_limit(&buildings.buildings, rivers.tiles.contains(&city.position) || city.position.all_neighbors().iter().any(|n| rivers.tiles.contains(n))),
        };
        let mut needs_orders = false;
        let mut just_trained = false;
        if !mine && owner != BARBARIANS && !view.defended {
            if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
                // An empty city drops everything for a defender, and pays to hurry it when in danger.
                let already = matches!(&city.production_queue, Some(ProductionItem::Unit(u)) if u.is_army());
                if !already {
                    let civ_view = governor::CivView { tech: &civ.tech, aggression: civ.aggression, cities: 1, workers: 1, settlers: 1, military: 0, navy: 0, air: 0, road_work: 0, cut_off: 0, at_war: true, wonders_taken: &wonders_taken, caravans: 1, diplomats: 1, missionaries: 1, faith_to_spread: false, wonder_elsewhere: false, space_parts: &[], room_to_expand: false, forbidden_palace: true, ferries: 1, ferry_wanted: 0, thrifty: true };
                    city.production_queue = Some(ProductionItem::Unit(governor::best_defender(&view, &civ_view, &supplies)));
                }
                let remaining = city.shields_left();
                if let Some(cost) = city.hurry_cost().filter(|cost| view.threatened && remaining > 0 && civ.gold >= *cost) {
                    civ.gold -= cost;
                    city.production_progress += remaining;
                }
            }
        }

        // A rich treasury hurries along what cities are working on: one purchase a turn, and one more
        // for every 500 gold in the treasury, always keeping a reserve.
        if !mine && owner != BARBARIANS {
            if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
                let allowed = 1 + (civ.gold / 500).max(0) as u32;
                let done = hurried.get(&owner).copied().unwrap_or(0);
                let remaining = city.shields_left();
                if let Some(cost) = city.hurry_cost().filter(|cost| done < allowed && remaining > 0 && civ.gold >= 150 + *cost) {
                    civ.gold -= cost;
                    city.production_progress += remaining;
                    *hurried.entry(owner).or_default() += 1;
                }
            }
        }

        // Farms, mines and bonus resources on the city's land add to what it gathers.
        let (extra_food, extra_shields, _) = supplies.yields.get(&city.position).copied().unwrap_or_default();
        // So do the civilization's traits.
        let character = characters.get(&owner).copied();
        let (trait_food, trait_shields, _, _) = character.map(|c| c.city_bonus()).unwrap_or_default();
        // The Pyramids are a Granary in every city of their owner.
        let extra_food = extra_food + if has_pyramids && !buildings.has(Building::Granary) { Building::Granary.food_bonus() } else { 0 };
        city.food_storage += extra_food + trait_food;
        city.production_progress += extra_shields + trait_shields;
        // The city's accounts: its trade less corruption, split by its people's rates, multiplied
        // by its buildings.
        let (books, outright_gold) = city_books(&city, &buildings, gov, rates_of.get(&owner).copied().unwrap_or_default(), corruption, &wonders, &great, character, &supplies);

        // Happiness. Temples and the like, wonders, masterpieces, entertainers and martial law calm the
        // unhappy; every different luxury the city can reach makes someone happy.
        let has_market = buildings.buildings.contains(&Building::Marketplace);
        let garrison = armies.iter().filter(|(p, o)| *p == city.position && *o == owner).count() as u32;
        let calm = buildings.buildings.iter().map(|b| b.content()).sum::<u32>()
            + wonders.empire_content(owner)
            + great.masterpiece_content(city.position)
            + city.luxury_from_specialists()
            + garrison.min(gov.martial_law())
            + gov.content_bonus()
            + if wonders.owns(owner, Building::Mausoleum) { buildings.buildings.iter().filter(|b| matches!(b, Building::Temple | Building::Walls)).count() as u32 } else { 0 }
            + if buildings.has(Building::Shakespeare) { city.population } else { 0 }
            + character.map_or(0, |c| c.content())
            // Citizens who share their ruler's faith (that of the capital) are calmer, more so with a Temple.
            + if religions.of(city.position).is_some_and(|f| capitals.get(&owner).and_then(|cp| religions.of(*cp)) == Some(f)) {
                bevy_civ::religion::STATE_FAITH_CONTENT + u32::from(buildings.has(Building::Temple))
            } else {
                0
            }
            + if mine { state.difficulty.player_content() } else { 0 };
        // Every two points of the city's luxury make one citizen happy.
        let joy = supply::luxury_happiness(supplies.luxuries(city.position), has_market) + books.luxury / 2;
        // A long war wears on a republic or democracy (a Police Station halves it in its city).
        let weary = gov.war_weariness(war_turns.get(&owner).copied().unwrap_or(0));
        let weary = if buildings.has(Building::PoliceStation) { weary / 2 } else { weary };
        let sprawl = bevy_civ::diplomacy::sprawl_unhappiness(empire_sizes.get(&owner).copied().unwrap_or(1));
        let gm_distrusted = buildings.has(Building::GmFarms) && !buildings.has(Building::FoodSafety);
        let troubled = bevy_civ::troubles::unhappiness(city.narcotics, city.scandal, gm_distrusted);
        let calm = calm.saturating_sub(weary + sprawl + troubled);
        happiness.update(city.population, calm, joy);
        if happiness.in_disorder() {
            *rioting.entry(owner).or_default() += 1;
        }
        // A computer player's governor turns workers into entertainers to end disorder, and back again.
        if !mine && owner != BARBARIANS {
            if happiness.in_disorder() && city.workers() > 1 {
                city.add_specialist(bevy_civ::city::Specialist::Entertainer);
                happiness.update(city.population, calm + 2, joy);
            } else if happiness.unhappy == 0 && happiness.happy > 0 && city.specialists.contains(&bevy_civ::city::Specialist::Entertainer) {
                if let Some(i) = city.specialists.iter().position(|s| *s == bevy_civ::city::Specialist::Entertainer) {
                    city.specialists.remove(i);
                }
            }
        }
        // The player's governor (the "Governor builds" switch) does the same for its city, and lets an
        // entertainer go back to work once the city would stay orderly without it.
        if mine && city.auto_build {
            let mut calm = calm;
            while happiness.in_disorder() && city.workers() > 1 {
                city.add_specialist(bevy_civ::city::Specialist::Entertainer);
                calm += 2;
                happiness.update(city.population, calm, joy);
                log.push(format!("The governor of {} puts a citizen to entertaining to calm the city.", city.name));
            }
            if let Some(i) = city.specialists.iter().position(|s| *s == bevy_civ::city::Specialist::Entertainer) {
                let mut probe = Happiness::new(city.population);
                probe.update(city.population, calm.saturating_sub(2), joy);
                if !probe.in_disorder() {
                    city.specialists.remove(i);
                    happiness.update(city.population, calm.saturating_sub(2), joy);
                    log.push(format!("The governor of {} sends an entertainer back to the fields.", city.name));
                }
            }
        }
        let disorder = happiness.in_disorder();
        if disorder && mine {
            log.push(format!(
                "Civil disorder in {}: no growth, production, science or taxes until its {} unhappy citizens are calmed (temple, luxuries, entertainers, or troops under martial law).",
                city.name, happiness.unhappy
            ));
        }

        // A city in disorder, or stirred up by a foreign diplomat, stands idle.
        let unrest = spies.in_unrest(city.position);
        if unrest && mine {
            log.push(format!("{} is in unrest and stands idle.", city.name));
        }
        // A captured city resists its conqueror for a while; a garrison of two settles it faster.
        let resisting = city.resistance > 0;
        if resisting {
            let step = if garrison >= 2 { 2 } else { 1 };
            city.resistance = city.resistance.saturating_sub(step);
            if mine {
                if city.resistance == 0 {
                    log.push(format!("{} accepts our rule and returns to work.", city.name));
                } else {
                    log.push(format!("{}'s people resist our rule: it produces nothing for {} more turns (a garrison of two speeds this up).", city.name, city.resistance));
                }
            }
        }
        let idle = unrest || disorder || resisting;
        if !idle {
            let fresh_water = rivers.tiles.contains(&city.position) || city.position.all_neighbors().iter().any(|n| rivers.tiles.contains(n));
            match city.process_food(&buildings, fresh_water) {
                Growth::Grew if mine => log.push(format!("{} grows to size {}.", city.name, city.population)),
                Growth::Starved if mine => log.push(format!("{} is starving and shrinks to size {}.", city.name, city.population)),
                Growth::Capped if mine && state.turn % 5 == 0 => {
                    let need = if buildings.has(Building::Aqueduct) { "a Sewer System" } else { "an Aqueduct" };
                    log.push(format!("{} cannot grow past size {} without {need}.", city.name, city.population));
                }
                _ => {}
            }
        }

        // A city that has lost access to a resource its project needs falls back to something it can build.
        if let Some(item) = city.production_queue.clone() {
            let missing = supplies.missing(city.position, item.required_resources());
            if !missing.is_empty() {
                if mine {
                    let names: Vec<&str> = missing.iter().map(|g| g.name()).collect();
                    log.push(format!("{} has no access to {} and stops work on {}.", city.name, names.join(" or "), item.name()));
                }
                city.production_queue = Some(ProductionItem::Unit(UnitType::Warrior));
                needs_orders = true;
            }
        }

        // A wonder finished elsewhere can no longer be built; the work done is kept for the next project.
        if let Some(ProductionItem::Building(wonder)) = city.production_queue.clone() {
            if wonder.is_wonder() && wonders.is_taken(wonder) {
                if mine {
                    log.push(format!("{} must stop work on the {}: it has been built elsewhere. Its {} shields go to the next project.", city.name, wonder.name(), city.production_progress));
                }
                city.production_queue = None;
                needs_orders = true;
            }
        }

        // Difficulty: the computer players' cities build faster or slower.
        if !idle && !mine && owner != BARBARIANS {
            let factor = state.difficulty.ai_output();
            let base = city.production_per_turn(&buildings, waste) as f32;
            let extra = (base * (factor - 1.0)).round() as i32;
            city.production_progress = (city.production_progress as i32 + extra).max(0) as u32;
        }
        // The Hoover Dam powers every city of its owner: a quarter more shields.
        if !idle && wonders.owns(owner, Building::HooverDam) {
            city.production_progress += city.production_per_turn(&buildings, waste) / 4;
        }
        // (A golden age adds a shield and a trade to the tiles that have them: see `work_tiles`.)
        let finished = if idle { None } else { city.process_production(&buildings, waste) };
        let finished_unit = matches!(finished, Some(ProductionItem::Unit(_)));
        let finished_any = finished.is_some();
        if let Some(completed) = finished {
            needs_orders = true;
            match completed {
                ProductionItem::Unit(ordered) => {
                    // A people's unique unit takes the place of the ordinary one it stands in for.
                    let ut = character.map(|c| c.unique).filter(|u| u.unique_base() == Some(ordered)).unwrap_or(ordered);
                    // Keep computer players from flooding the map.
                    let at_cap = !mine && ut.is_army() && army_sizes.get(&city.owner).copied().unwrap_or(0) >= governor::army_cap(city_counts.get(&city.owner).copied().unwrap_or(1));
                    if !at_cap {
                        let trained = spawn_unit(&mut commands, city.position, ut, city.owner);
                        commands.entity(trained).try_insert(HomeCity(city.position));
                        // A city with its governor on sends its new workers to look after the land.
                        if mine && city.auto_build && ut.is_worker() {
                            commands.entity(trained).try_insert(Automated);
                        }
                        // Barracks train veterans.
                        let veteran = if ut.domain() == Domain::Air { buildings.has(Building::Airport) } else { buildings.has(Building::Barracks) || wonders.owns(city.owner, Building::SunTzu) };
                        if ut.is_military() && veteran {
                            commands.queue(move |world: &mut World| {
                                if let Some(mut unit) = world.get_mut::<Unit>(trained) {
                                    unit.promote();
                                }
                            });
                        }
                        if ut.is_army() {
                            *army_sizes.entry(city.owner).or_default() += 1;
                            just_trained = true;
                        }
                    }
                    // Units keep coming until the player changes the order.
                    if mine {
                        if city.auto_build && ut.is_worker() {
                            log.push(format!("{} trains a {}, which sets about improving the land by itself (K takes it in hand).", city.name, ut.name()));
                        } else {
                            log.push(format!("{} trains a {}.", city.name, ut.name()));
                        }
                    }
                }
                ProductionItem::Building(b) if b.is_spaceship_part() => {
                    match space.add_part(city.owner, b, state.turn, !mine) {
                        Some(arrives) => {
                            let who = civs.players.iter().find(|c| c.id == city.owner).map(|c| c.name.clone()).unwrap_or_else(|| "They".into());
                            log.push(format!(
                                "{} have launched a spaceship for {}! It arrives on turn {arrives} unless their capital falls.",
                                capitalised(&who),
                                bevy_civ::space::destination()
                            ));
                        }
                        None if mine => {
                            let ship = space.ships.get(&PLAYER).cloned().unwrap_or_default();
                            log.push(if ship.is_ready() {
                                format!(
                                    "{} completes a {}. Our ship is ready: launch it from the Empire advisor (Space), {} turns to {} - or add components (up to {}) to fly faster.",
                                    city.name,
                                    b.name(),
                                    ship.flight_turns(),
                                    bevy_civ::space::destination(),
                                    bevy_civ::space::MOST[1]
                                )
                            } else {
                                format!("{} completes a {}. Still needed: {}.", city.name, b.name(), ship.missing())
                            });
                        }
                        None => {}
                    }
                    city.production_queue = None;
                }
                ProductionItem::Building(b) if b.is_wonder() => {
                    if wonders.complete(b, city.owner, &city.name) {
                        // Wonders that act once, when completed.
                        match b {
                            Building::Darwin => {
                                for _ in 0..2 {
                                    let learned = if mine { tech.discover() } else { civs.players.iter_mut().find(|c| c.id == city.owner).and_then(|c| c.tech.discover()) };
                                    if let (true, Some(learned)) = (mine, learned) {
                                        log.push(format!("Darwin's Voyage reveals {}!", learned.name()));
                                    }
                                }
                            }
                            Building::Apollo if mine => {
                                let all: Vec<Hex> = map.cells.iter().copied().collect();
                                fog.explored.extend(all);
                                log.push("The Apollo Program shows us the whole world.");
                            }
                            _ => {}
                        }
                        // A wonder in the spirit of its builders begins their golden age.
                        if character.is_some_and(|c| c.admires(b)) && golden.begin(city.owner) && (mine || foreign.met.contains(&city.owner)) {
                            let spirit = bevy_civ::traits::wonder_traits(b);
                            log.push(if mine {
                                format!("Our {spirit} people take pride in the {}: a golden age begins! For {GOLDEN_AGE_TURNS} turns every worked tile that gives a shield or a trade gives one more.", b.name())
                            } else {
                                let who = civs.players.iter().find(|c| c.id == city.owner).map(|c| c.adjective.clone()).unwrap_or_default();
                                format!("A golden age begins for the {who} people, inspired by the {}.", b.name())
                            });
                        }
                        let builder = civs.players.iter().find(|c| c.id == city.owner).map(|c| c.name.clone()).unwrap_or_else(|| "A people".into());
                        log.push(if mine {
                            format!("{} completes the {}! ({}) {}", city.name, b.name(), b.wonder_effect(), next_hint(&city))
                        } else {
                            format!("Wonder of the world: {} completed the {} in {}.", builder, b.name(), city.name)
                        });
                        buildings.buildings.push(b);
                        culture.record(city.position, b, state.turn);
                    } else {
                        // Beaten to it this very turn: the shields are kept.
                        city.production_progress += b.cost();
                        city.production_queue = None;
                        if mine {
                            log.push(format!("{} was beaten to the {}. Its shields go to the next project.", city.name, b.name()));
                        }
                    }
                }
                ProductionItem::Building(b) => {
                    if mine {
                        log.push(format!("{} completes the {}. {}", city.name, b.name(), next_hint(&city)));
                    }
                    buildings.buildings.push(b);
                    culture.record(city.position, b, state.turn);
                }
            }
        }

        // The build queue: a finished building (or a unit, when more is queued) makes way for the next
        // project in line that can still be built here.
        if finished_any && !city.queue.is_empty() && (city.production_queue.is_none() || finished_unit) {
            city.production_queue = None;
            while !city.queue.is_empty() {
                let next = city.queue.remove(0);
                let possible = match &next {
                    ProductionItem::Building(b) => !buildings.has(*b) && !(b.is_wonder() && wonders.is_taken(*b)),
                    ProductionItem::Unit(_) => true,
                };
                if possible {
                    if mine {
                        log.push(format!("{} starts on the {} from its build queue.", city.name, next.name()));
                    }
                    city.production_queue = Some(next);
                    break;
                }
            }
        }

        // Big industrial cities foul the land around them.
        // Mass Transit stops it; a Recycling Center halves it.
        let pollution_odds = (0.04 + 0.01 * city.population.saturating_sub(10) as f64).min(0.3) * if buildings.has(Building::RecyclingCenter) { 0.5 } else { 1.0 };
        if buildings.has(Building::Factory) && !buildings.has(Building::MassTransit) && city.population >= 10 && rng.gen_bool(pollution_odds) {
            let spoiled: Vec<Hex> = supplies
                .territory
                .iter()
                .filter(|(at, (_, home))| *home == city.position && **at != city.position && !map.water.contains(at))
                .map(|(at, _)| *at)
                .collect();
            if !spoiled.is_empty() {
                let at = spoiled[rng.gen_range(0..spoiled.len())];
                if let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == at && !t.is_polluted()) {
                    tile.add_improvement(Improvement::Pollution);
                    if mine {
                        log.push(format!("Pollution spoils land near {}: a worker can clean it up (X).", city.name));
                    }
                }
            }
        }

        // Culture accrues whatever the city's mood.
        let earned = culture.earned(city.position, &buildings.buildings, city.is_capital, state.turn, state.last_turn);
        culture.add(city.position, owner, earned + if wonders.owns(owner, Building::Eiffel) { 2 } else { 0 });

        // Great-person points; a birth appears in the city. Computer players put theirs to work at once.
        if owner != BARBARIANS {
            let earned = great::city_points(&buildings.buildings, &city.specialists, city.is_capital);
            if let Some(kind) = great.gather(owner, city.position, earned, &mut rng) {
                if mine {
                    spawn_unit(&mut commands, city.position, kind, PLAYER);
                    log.push(format!("A {} is born in {}! {}. (Select it and press U.)", kind.name(), city.name, great::ability(kind)));
                } else if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
                    match kind {
                        UnitType::GreatScientist => {
                            civ.tech.discover();
                        }
                        // A rich civilization settles its merchant for lasting income; a poor one takes the cash.
                        UnitType::GreatMerchant if civ.gold >= 200 => *great.merchants.entry(city.position).or_default() += 1,
                        UnitType::GreatMerchant => civ.gold += great::trade_mission_gold(6),
                        UnitType::GreatEngineer => {
                            if let Some(item) = &city.production_queue {
                                city.production_progress = city.production_progress.max(item.cost());
                            }
                        }
                        _ => *great.masterpieces.entry(city.position).or_default() += 1,
                    }
                    if foreign.met.contains(&owner) {
                        let article = if civ.adjective.starts_with(['A', 'E', 'I', 'O', 'U']) { "An" } else { "A" };
                        log.push(format!("{article} {} {} has been born.", civ.adjective, kind.name()));
                    }
                }
            }
        }

        // A computer player's city gets its next project from the governor - and so does one of ours
        // whose governor we have put in charge of building, once its queue runs out.
        let governed = !mine || (city.auto_build && city.queue.is_empty());
        if needs_orders && governed && owner != BARBARIANS {
            if let Some(civ) = civs.players.iter().find(|c| c.id == owner) {
                let civ_view = governor::CivView {
                    tech: if mine { &tech } else { &civ.tech },
                    aggression: if mine { 0.4 } else { civ.aggression },
                    cities: city_counts.get(&owner).copied().unwrap_or(1),
                    workers: worker_counts.get(&owner).copied().unwrap_or(0),
                    settlers: settler_counts.get(&owner).copied().unwrap_or(0),
                    military: army_sizes.get(&owner).copied().unwrap_or(0),
                    navy: navy_sizes.get(&owner).copied().unwrap_or(0),
                    air: air_sizes.get(&owner).copied().unwrap_or(0),
                    road_work: road_work.get(&owner).copied().unwrap_or(0),
                    cut_off: cut_off.get(&owner).copied().unwrap_or(0),
                    at_war: wars.iter().any(|(a, b)| *a == owner || *b == owner),
                    wonders_taken: &wonders_taken,
                    caravans: envoy_counts.get(&(owner, UnitType::Caravan)).copied().unwrap_or(0),
                    diplomats: envoy_counts.get(&(owner, UnitType::Diplomat)).copied().unwrap_or(0),
                    missionaries: envoy_counts.get(&(owner, UnitType::Missionary)).copied().unwrap_or(0),
                    faith_to_spread: missionary_work.contains(&owner),
                    wonder_elsewhere: wonder_sites.iter().any(|(at, o)| *o == owner && *at != city.position),
                    space_parts: &space_wanted.get(&owner).cloned().unwrap_or_default(),
                    room_to_expand: room.contains(&owner),
                    forbidden_palace: seats.get(&owner).is_some_and(|s| s.len() > 1),
                    ferries: ferry_counts.get(&owner).copied().unwrap_or(0),
                    ferry_wanted: ferries.wanted.get(&owner).copied().unwrap_or(0),
                    // Upkeep already leaves little for science (a computer player), or the treasury
                    // runs at a loss (ours).
                    thrifty: if mine { state.gold_per_turn < 0 } else { civ.rates.science <= 30 && civ.government != Government::Anarchy },
                };
                // The unit just trained (if any) now holds the city.
                let view = governor::CityView { built: governed_buildings(&buildings), defended: view.defended || just_trained, ..view };
                let next = governor::choose(&view, &civ_view, &supplies, &mut rng);
                // Count what is now on order so the civilization's other cities do not duplicate it.
                match &next {
                    ProductionItem::Unit(UnitType::Worker | UnitType::Engineers) => *worker_counts.entry(owner).or_default() += 1,
                    ProductionItem::Unit(UnitType::Settler) => *settler_counts.entry(owner).or_default() += 1,
                    ProductionItem::Unit(u) if u.is_ferry() => *ferry_counts.entry(owner).or_default() += 1,
                    ProductionItem::Unit(u) if u.domain() == Domain::Sea => *navy_sizes.entry(owner).or_default() += 1,
                    ProductionItem::Unit(u) if u.domain() == Domain::Air => *air_sizes.entry(owner).or_default() += 1,
                    _ => {}
                }
                if mine && city.production_queue.as_ref() != Some(&next) {
                    log.push(format!("The governor of {} starts on {}.", city.name, next.name()));
                }
                city.production_queue = Some(next);
            }
        }
        {
            let held = finance.entry(owner).or_default();
            held.0 += u32::from(buildings.has(Building::Bank));
            held.1 += u32::from(buildings.has(Building::StockExchange));
        }
        // Science and taxes come in unless the city stands idle; its upkeep is owed all the same.
        if owner != BARBARIANS {
            let collected = if idle { bevy_civ::economy::Ledger { science: 0, gold: 0, ..books } } else { books };
            accounts.entry(owner).or_default().add(&collected);
            if mine {
                total_science += collected.science;
                total_gold += collected.gold as i32;
            } else {
                let income = ai_income.entry(owner).or_default();
                income.0 += collected.science;
                income.1 += collected.gold as i32;
            }
            if !idle {
                let lift = 100 + bevy_civ::economy::tax_bonus(&buildings.buildings);
                *full_taxes.entry(owner).or_default() += (books.trade - books.corruption) * lift / 100;
                *fixed_income.entry(owner).or_default() += (outright_gold + bevy_civ::economy::TAX_COLLECTOR * city.tax_collectors() * lift / 100) as i32;
            }
        }
    }

    foreign.smugglers = smugglers;
    foreign.smuggled_by_us = smuggled_by_us;

    // The raiders gather on free land within two tiles of the city that bred them (insurgents close in).
    {
        let city_tiles: HashSet<Hex> = cities.iter().map(|(c, _, _)| c.position).collect();
        let mut used: HashSet<Hex> = HashSet::new();
        for (at, owner, kind, reason) in uprisings {
            let reach = if kind == UnitType::Insurgent { 1 } else { 2 };
            let spot = hexx::shapes::hexagon(at, 2)
                .filter(|h| map.cells.contains(h) && !map.water.contains(h) && !city_tiles.contains(h) && !held_tiles.contains(h) && !used.contains(h))
                .min_by_key(|h| ((h.distance_to(at) - reach).abs(), h.x, h.y));
            let Some(spot) = spot else { continue };
            used.insert(spot);
            spawn_unit(&mut commands, spot, kind, BARBARIANS);
            if kind == UnitType::Cartel {
                state.raiders.0 += 1;
            } else {
                state.raiders.1 += 1;
            }
            if owner == PLAYER {
                let name = cities.iter().find(|(c, _, _)| c.position == at).map(|(c, _, _)| c.name.clone()).unwrap_or_default();
                log.push(match kind {
                    UnitType::Cartel => format!(
                        "Cartel gunmen have set up near {name}, fed by {reason}. While they are near, the drug trade cannot be rooted out; an undefended city pays them off. Kill them to seize their assets."
                    ),
                    _ => format!(
                        "A terror cell has formed near {name}, fed by {reason}. Insurgents strike any city they can reach undefended. A Police Station stops cells forming and can foil their plots."
                    ),
                });
            }
        }
    }

    // Migration: every few turns, citizens leave troubled cities for better ones nearby that welcome
    // them - at home, or abroad in peoples that know Industrialization.
    if state.turn % bevy_civ::troubles::MIGRATION_EVERY == 0 {
        use bevy_civ::troubles::Prospect;
        let prospects: Vec<Prospect> = cities
            .iter()
            .map(|(c, b, h)| {
                let fresh_water = rivers.tiles.contains(&c.position) || c.position.all_neighbors().iter().any(|n| rivers.tiles.contains(n));
                Prospect {
                    at: c.position,
                    owner: c.owner,
                    population: c.population,
                    mood: h.happy as i32 - h.unhappy as i32,
                    food_balance: c.food_balance(&b),
                    narcotics: c.narcotics,
                    scandal: c.scandal > 0,
                    room: c.population < Building::size_limit(&b.buildings, fresh_water),
                }
            })
            .collect();
        let welcome_ours = state.welcome_migrants;
        let welcomes = |to: u32, from: u32| {
            if to == from {
                return true;
            }
            if to == BARBARIANS || from == BARBARIANS || !industrial.contains(&to) || diplomacy.get_treaty(to, from) == Treaty::War {
                return false;
            }
            if to == PLAYER { welcome_ours } else { !matches!(governments.get(&to), Some(Government::Communism | Government::Fundamentalism)) }
        };
        let mut moves: Vec<(Hex, Hex)> = Vec::new();
        for from in prospects.iter().filter(|p| p.pushes() && p.owner != BARBARIANS) {
            if rng.gen_bool(0.4) {
                if let Some(to) = bevy_civ::troubles::destination(from, &prospects, welcomes) {
                    moves.push((from.at, to));
                }
            }
        }
        let owner_at = |at: Hex| prospects.iter().find(|p| p.at == at).map(|p| p.owner).unwrap_or(BARBARIANS);
        let name_at = |at: Hex| cities.iter().find(|(c, _, _)| c.position == at).map(|(c, _, _)| c.name.clone()).unwrap_or_default();
        let people = |id: u32| civs.players.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_default();
        let mut news = Vec::new();
        for (from, to) in &moves {
            let (a, b) = (owner_at(*from), owner_at(*to));
            if a == PLAYER && b != PLAYER {
                news.push(format!("Citizens of {} emigrate to {} ({}), seeking a better life.", name_at(*from), name_at(*to), people(b)));
            } else if b == PLAYER && a != PLAYER {
                news.push(format!("Migrants from {} ({}) settle in {} (+1 citizen).", name_at(*from), people(a), name_at(*to)));
                // Their rulers resent losing their people to us.
                diplomacy.change_attitude(PLAYER, a, -1);
            } else if a == PLAYER && b == PLAYER {
                news.push(format!("Citizens move from troubled {} to {}.", name_at(*from), name_at(*to)));
            }
        }
        for (from, to) in &moves {
            state.migrations.0 += 1;
            if owner_at(*from) != owner_at(*to) {
                state.migrations.1 += 1;
            }
        }
        for (from, to) in moves {
            let left = cities.iter_mut().find(|(c, _, _)| c.position == from).map(|(mut c, _, _)| {
                c.population -= 1;
                let size = c.population as usize;
                c.specialists.truncate(size);
            });
            if left.is_some() {
                if let Some((mut c, _, _)) = cities.iter_mut().find(|(c, _, _)| c.position == to) {
                    c.population += 1;
                }
            }
        }
        for line in news {
            log.push(line);
        }
    }
    let difficulty = state.difficulty;
    for civ in civs.players.iter_mut().filter(|c| !c.is_human) {
        let (science, gold) = ai_income.get(&civ.id).copied().unwrap_or_default();
        let (trade_gold, trade_science) = trade.income(civ.id);
        let (science, gold) = (((science + trade_science) as f32 * difficulty.ai_output()) as u32, gold + trade_gold as i32);
        let (banks, exchanges) = finance.get(&civ.id).copied().unwrap_or_default();
        let interest = bevy_civ::building::interest(civ.gold, banks, exchanges);
        // Buildings cost their upkeep (not in anarchy, when no taxes come in).
        let upkeep = if civ.government == Government::Anarchy { 0 } else { accounts.get(&civ.id).map_or(0, |a| a.upkeep) as i32 };
        civ.gold += gold + interest - upkeep;
        // Savings beyond a reserve are invested in research.
        let invested = ((civ.gold - 300) / 3).max(0);
        civ.gold -= invested;
        civ.tech.science_per_turn = (science + invested as u32 / 2).max(1);
        // Next turn's rates: luxury while its cities riot, then as much science as it can afford.
        let held = city_counts.get(&civ.id).copied().unwrap_or(0);
        let luxury = bevy_civ::economy::luxury_for(civ.rates.luxury, rioting.get(&civ.id).copied().unwrap_or(0), held);
        let income = fixed_income.get(&civ.id).copied().unwrap_or(0) + trade_gold as i32 + interest;
        civ.rates = bevy_civ::economy::computer_rates(civ.government, luxury, full_taxes.get(&civ.id).copied().unwrap_or(0), income, upkeep, civ.gold);
    }

    let (trade_gold, trade_science) = trade.income(PLAYER);
    total_gold += trade_gold as i32 + foreign.pacts.len() as i32 * foreign::PACT_INCOME;
    {
        let mut ctx = foreign::Ctx { diplomacy: &mut diplomacy, foreign: &mut foreign, civs: &mut civs, state: &mut state, tech: &mut tech, log: &mut log };
        total_gold += foreign::settle_deals(&mut ctx, &supplies);
    }
    total_science += trade_science;

    let free_support = gov.free_unit_support() * player_cities.max(1) + if wonders.owns(PLAYER, Building::StatueOfZeus) { player_cities } else { 0 };
    let military = units.iter().filter(|(_, u)| u.owner == PLAYER && u.unit_type.is_military()).count() as u32;
    state.unit_support = if military > free_support { (military - free_support) as i32 * gov.unit_upkeep_food() as i32 } else { 0 };
    total_gold -= state.unit_support;
    // Buildings cost their upkeep (not in anarchy, when no taxes come in).
    if gov != Government::Anarchy {
        total_gold -= accounts.get(&PLAYER).map_or(0, |a| a.upkeep) as i32;
    }
    let (banks, exchanges) = finance.get(&PLAYER).copied().unwrap_or_default();
    state.interest = bevy_civ::building::interest(state.gold, banks, exchanges);
    total_gold += state.interest;
    state.gold_per_turn = total_gold;
    state.gold += total_gold;
    tech.science_per_turn = total_science;
    state.accounts = accounts;
    // A treasury that cannot pay its bills sells buildings, the dearest to keep first, until it can.
    {
        let mut debtors: Vec<(u32, i32)> = civs.players.iter().filter(|c| !c.is_human && c.gold < 0).map(|c| (c.id, c.gold)).collect();
        if state.gold < 0 {
            debtors.push((PLAYER, state.gold));
        }
        for (owner, mut gold) in debtors {
            while gold < 0 {
                let Some((name, sold)) = sell_for_debt(&mut cities, owner) else {
                    gold = 0;
                    break;
                };
                gold += sold.cost() as i32;
                if owner == PLAYER {
                    log.push(format!("Our treasury is empty: {name} sells its {} for {} gold to pay the bills.", sold.name(), sold.cost()));
                }
            }
            if owner == PLAYER {
                state.gold = gold;
            } else if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
                civ.gold = gold;
            }
        }
    }

    // Marco Polo's Embassy: an embassy with every civilization (and the player meets them all).
    if let Some(&(owner, _)) = wonders.built.get(&Building::MarcoPolo) {
        for civ in civs.players.iter().filter(|c| c.id != owner) {
            spies.embassies.insert((owner, civ.id));
            if owner == PLAYER && !civ.is_human {
                foreign.met.insert(civ.id);
            }
        }
    }
    // Computer generals in one of their cities form the troops there into an army.
    {
        let city_spots: HashSet<(Hex, u32)> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();
        let generals: Vec<(Entity, Hex, u32)> = units
            .iter()
            .filter(|(_, u)| u.unit_type == UnitType::GreatGeneral && u.owner != PLAYER && city_spots.contains(&(u.position, u.owner)))
            .map(|(e, u)| (e, u.position, u.owner))
            .collect();
        // Units merged into an army this turn (despawned, but still in the query until next frame).
        let mut taken: HashSet<Entity> = HashSet::new();
        // A computer player keeps no more than a few generals: the rest retire (they would only pile up).
        let mut retained: HashMap<u32, usize> = HashMap::new();
        for (e, u) in units.iter().filter(|(_, u)| u.unit_type == UnitType::GreatGeneral && u.owner != PLAYER && u.owner != BARBARIANS) {
            let kept = retained.entry(u.owner).or_default();
            *kept += 1;
            if *kept > GENERALS_KEPT {
                taken.insert(e);
                commands.entity(e).try_despawn_recursive();
            }
        }
        let generals: Vec<(Entity, Hex, u32)> = generals.into_iter().filter(|(g, _, _)| !taken.contains(g)).collect();
        for (general, at, owner) in generals {
            let stack: Vec<(Entity, Unit)> = units.iter().filter(|(_, u)| u.position == at && u.owner == owner).map(|(e, u)| (e, u.clone())).collect();
            // Keep one unit back as the city's own garrison.
            if stack.iter().filter(|(_, u)| bevy_civ::unit::can_join_army(u)).count() < 3 {
                continue;
            }
            let refs: Vec<&Unit> = stack.iter().map(|(_, u)| u).collect();
            let capacity = bevy_civ::unit::army_capacity(wonders.owns(owner, Building::SunTzu));
            let Some((leader, mut absorbed)) = bevy_civ::unit::plan_army(&refs, capacity) else { continue };
            // One unit always stays behind to hold the city.
            let joinable = stack.iter().filter(|(_, u)| bevy_civ::unit::can_join_army(u)).count();
            absorbed.truncate(joinable.saturating_sub(2).max(1));
            let members: Vec<Unit> = absorbed.iter().map(|i| stack[*i].1.clone()).collect();
            for i in &absorbed {
                taken.insert(stack[*i].0);
                commands.entity(stack[*i].0).try_despawn_recursive();
            }
            commands.entity(general).try_despawn_recursive();
            if let Ok((_, mut lead)) = units.get_mut(stack[leader].0) {
                bevy_civ::unit::join_army(&mut lead, &members);
            }
        }
        // Their armies resting in a city take in the troops there that are not needed to hold it.
        let armies: Vec<(Entity, Hex, u32)> = units
            .iter()
            .filter(|(e, u)| !taken.contains(e) && u.owner != PLAYER && u.is_army_group() && city_spots.contains(&(u.position, u.owner)))
            .map(|(e, u)| (e, u.position, u.owner))
            .collect();
        for (army, at, owner) in armies {
            let capacity = bevy_civ::unit::army_capacity(wonders.owns(owner, Building::SunTzu));
            let recruits: Vec<(Entity, Unit)> = units
                .iter()
                .filter(|(e, u)| *e != army && !taken.contains(e) && u.position == at && u.owner == owner && bevy_civ::unit::can_join_army(u))
                .map(|(e, u)| (e, u.clone()))
                .collect();
            let spare = recruits.len().saturating_sub(1);
            for (entity, recruit) in recruits.into_iter().take(spare) {
                let Ok((_, mut lead)) = units.get_mut(army) else { break };
                if !lead.take_into_army(&recruit, capacity) {
                    break;
                }
                taken.insert(entity);
                commands.entity(entity).try_despawn_recursive();
            }
        }
    }
    // Leonardo's Workshop: one obsolete unit of its owner is upgraded free each turn.
    if let Some(&(owner, _)) = wonders.built.get(&Building::Leonardo).filter(|_| !wonders.expired.contains(&Building::Leonardo)) {
        let known = |t: Option<Tech>| {
            t.is_none_or(|t| if owner == PLAYER { tech.researched.contains(&t) } else { civs.players.iter().any(|c| c.id == owner && c.tech.researched.contains(&t)) })
        };
        let candidate = units
            .iter()
            .filter(|(_, u)| u.owner == owner && u.troops.is_empty() && u.unit_type.upgrades_to().is_some_and(|to| known(to.required_tech())))
            .map(|(e, _)| e)
            .min_by_key(|e| e.index());
        if let Some(entity) = candidate {
            if let Ok((_, mut unit)) = units.get_mut(entity) {
                if let Some(to) = unit.unit_type.upgrades_to() {
                    if owner == PLAYER {
                        log.push(format!("Leonardo's Workshop re-equips a {} as a {}.", unit.unit_type.name(), to.name()));
                    }
                    unit.unit_type = to;
                    unit.hp = unit.max_hp();
                }
            }
        }
    }
    let home_cities: HashSet<(Hex, u32)> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();
    for (_, mut unit) in &mut units {
        if let Some(done) = unit.process_build() {
            if let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == unit.position) {
                if done == Improvement::Pollution {
                    tile.improvements.retain(|i| *i != Improvement::Pollution);
                    if unit.owner == PLAYER {
                        log.push("The pollution is cleaned up.");
                    }
                } else {
                    tile.add_improvement(done);
                    if unit.owner == PLAYER {
                        log.push(format!("A {:?} is finished.", done));
                    }
                }
            }
        }
        // Wounds heal while a unit rests: fully in one of its own cities, a hit point a turn elsewhere.
        if unit.hp < unit.max_hp() && (unit.can_move() || unit.fortified) {
            let home = home_cities.contains(&(unit.position, unit.owner));
            unit.hp = if home { unit.max_hp() } else { unit.hp + 1 };
        }
        if unit.build_turns_left.is_none() {
            let moves = moves_per_turn(unit.unit_type, unit.position, &map, wonders.sea_moves(unit.owner)).min(unit.troop_moves());
            unit.refresh(moves);
        }
    }

    // A city in disorder near a far greater culture may defect to it, its garrison sent home - more
    // readily far from its own capital and where the tempter's borders hold its land, less so with
    // troops inside.
    {
        let owners: Vec<(Hex, u32)> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();
        let totals = culture.totals(owners.iter().copied());
        let total = |owner: u32| totals.get(&owner).copied().unwrap_or(0);
        let mut defections: Vec<(Hex, u32, u32)> = Vec::new();
        for (city, _, happiness) in cities.iter() {
            if city.is_capital || city.owner == BARBARIANS || !happiness.in_disorder() {
                continue;
            }
            let tempter = owners
                .iter()
                .filter(|(at, owner)| {
                    *owner != city.owner
                        && *owner != BARBARIANS
                        && at.distance_to(city.position) <= culture_rules::DEFECTION_REACH
                        && total(*owner) >= 2 * total(city.owner).max(1)
                        && culture.of(*at) > culture.of(city.position)
                })
                .min_by_key(|(at, _)| at.distance_to(city.position))
                .map(|(_, owner)| *owner);
            let Some(new_owner) = tempter else { continue };
            let far = capitals.get(&city.owner).map_or(0, |cp| cp.distance_to(city.position).max(0) as u32);
            let held = supplies.territory.iter().filter(|(at, (o, _))| *o == new_owner && at.distance_to(city.position) <= CITY_RADIUS).count() as u32;
            let garrison = armies.iter().filter(|(p, o)| *p == city.position && *o == city.owner).count() as u32;
            if rng.gen_bool(culture_rules::defection_chance(far, held, garrison)) {
                defections.push((city.position, city.owner, new_owner));
            }
        }
        for (at, old_owner, new_owner) in defections {
            let Some((mut city, _, _)) = cities.iter_mut().find(|(c, _, _)| c.position == at) else { continue };
            let theirs = civs.players.iter().find(|c| c.id == new_owner).map(|c| c.adjective.clone()).unwrap_or_else(|| "our".into());
            if old_owner == PLAYER {
                log.push(format!("{} is drawn to {} culture and defects!", city.name, theirs));
            } else if new_owner == PLAYER {
                log.push(format!("{} admires our culture and joins us!", city.name));
            }
            city.owner = new_owner;
            city.production_queue = None;
            city.production_progress = 0;
            wonders.transfer_city(&city.name, new_owner);
            for (entity, unit) in units.iter() {
                if unit.position == at && unit.owner == old_owner {
                    commands.entity(entity).try_despawn_recursive();
                }
            }
        }
    }

    let city_owners: Vec<(Hex, u32)> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();

    // --- Computer players and barbarians move, attack and capture -----------------------------
    // Barbarians fight everyone; civilizations fight those they are at war with.
    // A flat table of who is at war (ids are small), as the AI asks this for every pair of units.
    const SIDES: usize = 128;
    let mut war_table = vec![false; SIDES * SIDES];
    for ((a, b), treaty) in &diplomacy.treaties {
        if *treaty == Treaty::War && (*a as usize) < SIDES && (*b as usize) < SIDES {
            war_table[*a as usize * SIDES + *b as usize] = true;
            war_table[*b as usize * SIDES + *a as usize] = true;
        }
    }
    let hostile = move |a: u32, b: u32| {
        if a == b {
            return false;
        }
        a == BARBARIANS || b == BARBARIANS || ((a as usize) < SIDES && (b as usize) < SIDES && war_table[a as usize * SIDES + b as usize])
    };
    // Roads the computer players want: from the governing city to each source that is not yet
    // connected, and from each capital to cities that are cut off from it.
    let mut road_jobs: HashMap<u32, Vec<Hex>> = HashMap::new();
    {
        let land: Vec<(Hex, Terrain)> = tiles.iter().map(|t| (t.coord, t.terrain)).collect();
        let done: HashSet<Hex> = tiles.iter().filter(|t| t.has_road()).map(|t| t.coord).chain(city_owners.iter().map(|(p, _)| *p)).collect();
        let mut routes: Vec<(u32, Hex, Hex)> = supplies
            .unconnected
            .iter()
            .filter(|(_, owner)| **owner != PLAYER)
            .filter_map(|(hex, owner)| supplies.territory.get(hex).map(|(_, home)| (*owner, *home, *hex)))
            .collect();
        // A cut-off city is joined to the nearest city of its owner that is on the network; if none can
        // be reached by land, to the nearest coastal one, whose harbour will carry the link overseas.
        let coastal = |hex: &Hex| hex.all_neighbors().iter().any(|n| map.water.contains(n));
        for (city, owner) in city_owners.iter().filter(|(p, o)| *o != PLAYER && *o != BARBARIANS && !supplies.linked.contains(p)) {
            let mut hubs: Vec<Hex> = city_owners.iter().filter(|(p, o)| o == owner && p != city).map(|(p, _)| *p).collect();
            hubs.sort_by_key(|p| (!supplies.linked.contains(p), !coastal(p), p.distance_to(*city)));
            let reachable = hubs
                .into_iter()
                .filter(|p| p.distance_to(*city) <= 14 && (supplies.linked.contains(p) || coastal(p)))
                .take(4)
                .find(|p| find_path(*p, *city, land.iter().copied(), Travel::Land).is_some());
            if let Some(hub) = reachable {
                routes.push((*owner, hub, *city));
            }
        }
        // Nearest goals first, so short useful roads are finished before long ones.
        routes.sort_by_key(|(_, from, to)| from.distance_to(*to));
        for (owner, from, to) in routes {
            if let Some(route) = find_path(from, to, land.iter().copied(), Travel::Land) {
                let jobs = road_jobs.entry(owner).or_default();
                for step in route {
                    if !done.contains(&step) && !jobs.contains(&step) {
                        jobs.push(step);
                    }
                }
            }
        }
    }
    let owns = |hex: Hex, owner: u32| supplies.territory.get(&hex).is_some_and(|(o, _)| *o == owner);
    // Cities worth a long march: the capital of a people whose spaceship is in flight (taking it
    // stops the ship), and the last cities of a people on its last legs (taking them ends it).
    let dying: HashSet<u32> = city_counts.iter().filter(|(o, n)| **o != PLAYER && **o != BARBARIANS && **n <= 3).map(|(o, _)| *o).collect();
    let must_fall: Vec<Hex> = cities
        .iter()
        .filter(|(c, _, _)| (c.is_capital && space.ships.get(&c.owner).is_some_and(|s| s.arrives.is_some())) || dying.contains(&c.owner))
        .map(|(c, _, _)| c.position)
        .collect();
    // What a defender stands on, tile by tile: the land, a river, a fortress, and a city's size and walls.
    let walled_cities: HashSet<Hex> = cities.iter().filter(|(c, b, _)| b.has(Building::Walls) || wonders.walls_everywhere(c.owner)).map(|(c, _, _)| c.position).collect();
    let forts: HashSet<Hex> = cities.iter().filter(|(_, b, _)| b.has(Building::CoastalFortress)).map(|(c, _, _)| c.position).collect();
    let fortresses: HashSet<Hex> = tiles.iter().filter(|t| t.improvements.contains(&Improvement::Fortress)).map(|t| t.coord).collect();
    let populations: HashMap<Hex, u32> = cities.iter().map(|(c, _, _)| (c.position, c.population)).collect();
    let terrain_of: HashMap<Hex, Terrain> = tiles.iter().map(|t| (t.coord, t.terrain)).collect();
    let ground_at = |at: Hex| Ground {
        terrain: terrain_of.get(&at).copied().unwrap_or(Terrain::Plains),
        fortified: false,
        walls: walled_cities.contains(&at),
        river: rivers.tiles.contains(&at),
        coast_fort: forts.contains(&at),
        city: populations.get(&at).copied(),
        fortress: fortresses.contains(&at),
    };
    let attacks = process_ai_turn(&mut civs, &mut units, &city_owners, &tiles, &hostile, &mut road_jobs, &owns, &mut ferries, &mut commands, &must_fall, &ground_at);
    let names: HashMap<u32, String> = civs.players.iter().map(|c| (c.id, c.adjective.clone())).collect();
    let civ_name = |owner: u32| names.get(&owner).cloned().unwrap_or_else(|| bevy_civ::theme::term("barbarian", "Barbarian").to_string());

    let mut dead: HashSet<Entity> = HashSet::new();
    // Guns fire first, softening up what the troops then storm.
    let mut attacks = attacks;
    attacks.sort_by_key(|a| !a.bombard);
    for attack in attacks {
        if dead.contains(&attack.attacker) {
            continue;
        }
        let Ok((_, attacker)) = units.get(attack.attacker) else { continue };
        let (a_type, a_owner, a_pos) = (attacker.unit_type, attacker.owner, attacker.position);
        let defender = units
            .iter()
            .filter(|(e, u)| u.position == attack.target && u.owner != a_owner && !dead.contains(e))
            .max_by_key(|(_, u)| defending_as(u.unit_type, u.position, &map).stats().1 * 10 + u.hp)
            .map(|(e, u)| (e, u.unit_type, u.owner, u.fortified));
        let a_vitals = (attacker.hp, attacker.level);
        let own_cities: HashSet<Hex> = city_owners.iter().filter(|(_, o)| *o == a_owner).map(|(p, _)| *p).collect();
        let can_enter = may_enter(a_type, attack.target, &map, &own_cities);
        let Some((d_entity, d_type, d_owner, dug_in)) = defender else {
            if can_enter && !attack.bombard {
                if let Ok((_, mut attacker)) = units.get_mut(attack.attacker) {
                    attacker.position = attack.target;
                }
            }
            continue;
        };
        if d_owner == PLAYER && a_owner != BARBARIANS && diplomacy.get_treaty(PLAYER, a_owner) != Treaty::War {
            diplomacy.set_treaty(PLAYER, a_owner, Treaty::War);
            log.push(format!("The {} attack means war!", civ_name(a_owner)));
            foreign.since.insert((PLAYER, a_owner), state.turn);
        }
        // A gun's bombardment: it wears the defender down but never kills, at no risk to itself.
        if attack.bombard {
            let Ok((_, victim)) = units.get(d_entity) else { continue };
            let (before, most, level, members) = (victim.hp, victim.max_hp(), victim.level, victim.members());
            let side = Side { kind: defending_as(d_type, attack.target, &map), hp: before, level, led: false, members };
            let hp = combat::bombard(a_type, side, Ground { fortified: dug_in, ..ground_at(attack.target) }, &mut rng);
            if let Ok((_, mut victim)) = units.get_mut(d_entity) {
                victim.hp = hp;
            }
            tally.bombardments += 1;
            tally.shelled += before - hp;
            if d_owner == PLAYER {
                log.push(format!("A {} {} bombards our {}: {hp} of {most} hit points left.", civ_name(a_owner), a_type.name(), d_type.name()));
                battles.send(BattleShown {
                    from: a_pos,
                    at: attack.target,
                    attacker: Some(attack.attacker),
                    result: if hp == before { "No effect".to_string() } else { format!("-{} HP: {hp}/{most}", before - hp) },
                    good: Some(hp == before),
                    ours: true,
                    attacker_side: (a_type, a_owner),
                    defender_side: (d_type, d_owner),
                    attacker_died: false,
                    defender_died: false,
                });
            }
            continue;
        }
        // A bomber may be met by a fighter of the defender (or of anyone at war with the attacker)
        // within three tiles of the target that has not flown this turn.
        if a_type == UnitType::Bomber {
            let interceptor = units
                .iter()
                .filter(|(e, u)| !dead.contains(e) && u.unit_type == UnitType::Fighter && u.movement_left > 0 && u.position.distance_to(attack.target) <= 3)
                .find(|(_, u)| u.owner == d_owner || (u.owner != a_owner && (a_owner == BARBARIANS || diplomacy.get_treaty(a_owner, u.owner) == Treaty::War)))
                .map(|(e, u)| (e, u.owner));
            if let Some((fighter, fighter_owner)) = interceptor {
                if let Ok((_, mut f)) = units.get_mut(fighter) {
                    f.end_move();
                }
                if rng.gen_bool(UnitType::interception_odds(UnitType::Fighter, UnitType::Bomber)) {
                    dead.insert(attack.attacker);
                    commands.entity(attack.attacker).try_despawn_recursive();
                    if fighter_owner == PLAYER {
                        log.push(format!("Our fighters intercept and shoot down a {} bomber.", civ_name(a_owner)));
                    }
                    continue;
                }
                if let Ok((_, mut bomber)) = units.get_mut(attack.attacker) {
                    bomber.hp = bomber.hp.saturating_sub(1).max(1);
                }
            }
        }
        let led = |at: Hex, owner: u32| units.iter().any(|(e, u)| !dead.contains(&e) && u.position == at && u.owner == owner && u.unit_type == UnitType::GreatGeneral);
        let (a_led, d_led) = (led(a_pos, a_owner), led(attack.target, d_owner));
        let d_vitals = units.get(d_entity).map(|(_, u)| (u.hp, u.level)).unwrap_or((3, 0));
        let a_members = units.get(attack.attacker).map(|(_, u)| u.members()).unwrap_or(1);
        let d_members = units.get(d_entity).map(|(_, u)| u.members()).unwrap_or(1);
        let outcome = combat::fight(
            Side { kind: a_type, hp: a_vitals.0, level: a_vitals.1, led: a_led, members: a_members },
            Side { kind: defending_as(d_type, attack.target, &map), hp: d_vitals.0, level: d_vitals.1, led: d_led, members: d_members },
            Ground { fortified: dug_in, ..ground_at(attack.target) },
            &mut rng,
        );
        // Show it: every battle against the player, and others the player can see.
        let ours = d_owner == PLAYER || a_owner == PLAYER;
        if ours || fog.visible.contains(&attack.target) {
            let (result, good) = match (outcome, d_owner == PLAYER) {
                (Outcome::Won { .. }, true) => (format!("Our {} is lost", d_type.name()), Some(false)),
                (Outcome::Lost { .. }, true) => (format!("Our {} holds!", d_type.name()), Some(true)),
                (Outcome::Retreated { .. }, true) => ("Attack driven off".to_string(), Some(true)),
                (Outcome::Won { .. }, false) => (format!("{} wins", a_type.name()), None),
                (_, false) => (format!("{} holds", d_type.name()), None),
            };
            battles.send(BattleShown {
                from: a_pos,
                at: attack.target,
                attacker: Some(attack.attacker),
                result,
                good,
                ours,
                attacker_side: (a_type, a_owner),
                defender_side: (d_type, d_owner),
                attacker_died: matches!(outcome, Outcome::Lost { .. }),
                defender_died: matches!(outcome, Outcome::Won { .. }),
            });
        }
        tally.battles += 1;
        tally.won += u32::from(matches!(outcome, Outcome::Won { .. }));
        tally.elite_wins += u32::from(match outcome {
            Outcome::Won { .. } => a_vitals.1 >= 2,
            Outcome::Lost { .. } => d_vitals.1 >= 2,
            Outcome::Retreated { .. } => false,
        });
        tally.retreated += u32::from(matches!(outcome, Outcome::Retreated { .. }));
        // Hit points left after the fight, and a chance of promotion for the victor (`beat`: whose
        // unit it defeated).
        let mut settle = |entity: Entity, hp: u32, beat: Option<u32>, rng: &mut rand::rngs::ThreadRng| {
            if let Ok((_, mut unit)) = units.get_mut(entity) {
                unit.hp = hp.max(1);
                if let Some(loser) = beat {
                    let barbarian = loser == BARBARIANS;
                    let chance = characters.get(&unit.owner).map_or(combat::promotion_chance(unit.level, false, barbarian), |c| c.promotion_chance(unit.level, barbarian));
                    if rng.gen_bool(chance) && unit.promote() {
                        tally.promotions += 1;
                    }
                }
            }
        };
        let attacker_won = match outcome {
            Outcome::Won { attacker_hp } => {
                settle(attack.attacker, attacker_hp, Some(d_owner), &mut rng);
                true
            }
            Outcome::Retreated { defender_hp } => {
                settle(attack.attacker, 1, None, &mut rng);
                settle(d_entity, defender_hp, None, &mut rng);
                if d_owner == PLAYER {
                    log.push(format!("Our {} drove off a {} {}.", d_type.name(), civ_name(a_owner), a_type.name()));
                }
                continue;
            }
            Outcome::Lost { defender_hp } => {
                settle(d_entity, defender_hp, Some(a_owner), &mut rng);
                false
            }
        };
        if attacker_won {
            unique_victory(&mut golden, &civs, &foreign.met, &mut log, a_owner, a_type, d_owner);
            plunder(&mut state, &mut civs, &mut log, a_owner, a_type);
            if a_owner != BARBARIANS && great.victory(a_owner, a_vitals.1, &mut rng) {
                spawn_unit(&mut commands, a_pos, UnitType::GreatGeneral, a_owner);
                tally.generals += 1;
            }
            dead.insert(d_entity);
            commands.entity(d_entity).try_despawn_recursive();
            if d_owner == PLAYER {
                log.push(format!("Our {} was destroyed by a {} {}.", d_type.name(), civ_name(a_owner), a_type.name()));
            }
            let still_defended = units.iter().any(|(e, u)| u.position == attack.target && u.owner != a_owner && !dead.contains(&e));
            if !still_defended && can_enter {
                if let Ok((_, mut attacker)) = units.get_mut(attack.attacker) {
                    attacker.position = attack.target;
                }
            }
        } else {
            dead.insert(attack.attacker);
            commands.entity(attack.attacker).try_despawn_recursive();
            if d_owner == PLAYER {
                log.push(format!("Our {} repelled a {} {}.", d_type.name(), civ_name(a_owner), a_type.name()));
            }
            unique_victory(&mut golden, &civs, &foreign.met, &mut log, d_owner, d_type, a_owner);
            plunder(&mut state, &mut civs, &mut log, d_owner, d_type);
            if d_owner != BARBARIANS && great.victory(d_owner, d_vitals.1, &mut rng) {
                spawn_unit(&mut commands, attack.target, UnitType::GreatGeneral, d_owner);
                tally.generals += 1;
                if d_owner == PLAYER {
                    log.push(format!("The stand of our elite {} brings forth a Great General! Keep it with your troops (they fight half again as hard), or form an army in one of our cities.", d_type.name()));
                }
            }
        }
    }

    // An enemy military unit standing in an undefended city takes it - except modern raiders, who
    // extort it (cartels) or strike at it (insurgents), and are spent.
    for (mut city, mut buildings, _) in &mut cities {
        let defended = units.iter().any(|(e, u)| !dead.contains(&e) && u.position == city.position && u.owner == city.owner && u.unit_type.is_army());
        let occupying = units
            .iter()
            .find(|(e, u)| {
                !dead.contains(e) && u.position == city.position && u.owner != PLAYER && u.owner != city.owner && u.unit_type.is_army() && !u.unit_type.bombard_only() && hostile(u.owner, city.owner)
            })
            .map(|(e, u)| (e, u.owner, u.unit_type));
        if let Some((raider, _, kind)) = occupying.filter(|(_, _, k)| k.is_raider() && !defended) {
            state.raiders.2 += 1;
            dead.insert(raider);
            commands.entity(raider).try_despawn_recursive();
            let mine = city.owner == PLAYER;
            if kind == UnitType::Cartel {
                let treasury = if mine { state.gold } else { civs.players.iter().find(|c| c.id == city.owner).map_or(0, |c| c.gold) };
                let paid = bevy_civ::barbarians::extortion(city.population, treasury);
                if mine {
                    state.gold -= paid;
                    log.push(format!("Cartel gunmen walk into undefended {} and extort {paid} gold, then melt into the streets.", city.name));
                } else if let Some(civ) = civs.players.iter_mut().find(|c| c.id == city.owner) {
                    civ.gold -= paid;
                }
            } else if buildings.has(Building::PoliceStation) && rng.gen_bool(bevy_civ::barbarians::POLICE_FOIL) {
                if mine {
                    log.push(format!("The police of {} foil an insurgents' plot and round up the cell.", city.name));
                }
            } else {
                // A building wrecked (never a wonder or the palace) and a citizen killed.
                let targets: Vec<Building> = buildings.buildings.iter().copied().filter(|b| !b.is_wonder() && *b != Building::ForbiddenPalace && !b.is_spaceship_part()).collect();
                let wrecked = (!targets.is_empty()).then(|| targets[rng.gen_range(0..targets.len())]);
                if let Some(b) = wrecked {
                    buildings.buildings.retain(|x| *x != b);
                }
                if city.population > 1 {
                    city.population -= 1;
                }
                if mine {
                    let what = wrecked.map(|b| format!("the {} is wrecked", b.name())).unwrap_or_else(|| "the streets are left in ruins".to_string());
                    log.push(format!("Insurgents strike undefended {}: {what} and a citizen is killed. Keep troops in your cities; a Police Station foils such plots.", city.name));
                }
            }
            continue;
        }
        let occupier = occupying.map(|(_, owner, _)| owner);
        if let Some(new_owner) = occupier.filter(|_| !defended) {
            if city.owner != BARBARIANS && new_owner != BARBARIANS {
                foreign.add_grievance(city.owner, new_owner, foreign::Wrong::CityTaken, state.turn);
            }
            if city.owner == PLAYER {
                log.push(format!("{} has fallen to {} forces!", city.name, civ_name(new_owner)));
            } else if fog.explored.contains(&city.position) {
                log.push(format!("{} forces capture {}.", civ_name(new_owner), city.name));
            }
            city.owner = new_owner;
            city.is_capital = false;
            city.resistance = bevy_civ::city::resistance_turns(city.population);
            wonders.transfer_city(&city.name, new_owner);
            city.production_progress = 0;
            city.production_queue = Some(ProductionItem::Unit(UnitType::Warrior));
        }
    }
    // Computer players keep their armies modern: troops in their cities are re-equipped while the
    // treasury allows (keeping a reserve), and when the army is at its size limit the weakest unit
    // that can no longer be upgraded is stood down to make room for something better.
    {
        let city_spots: HashMap<Hex, u32> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();
        let city_count = |owner: u32| city_spots.values().filter(|o| **o == owner).count();
        for civ in civs.players.iter_mut().filter(|c| c.id != PLAYER && !c.is_human) {
            let knows = |t: Option<Tech>| t.is_none_or(|t| civ.tech.researched.contains(&t));
            // A unit's worth: its attack (a gun's bombardment) and defence.
            let power = |u: UnitType| u.stats().0.max(u.bombard().map_or(0, |(strength, _, _)| strength)) + u.stats().1;
            let mut upgraded = 0;
            let mut candidates: Vec<(Entity, bool, i32)> = units
                .iter()
                .filter(|(_, u)| u.owner == civ.id && city_spots.get(&u.position) == Some(&civ.id))
                .filter_map(|(e, u)| {
                    let (_, to) = u.refit_cost(|to| knows(to.required_tech()) && supplies.has(u.position, to.required_resources()))?;
                    // Soldiers before workers, and the most outdated first.
                    Some((e, u.unit_type.is_military(), power(to) as i32 - power(u.unit_type) as i32))
                })
                .collect();
            candidates.sort_by_key(|(e, military, gain)| (!*military, -*gain, e.index()));
            let candidates: Vec<Entity> = candidates.into_iter().map(|(e, _, _)| e).collect();
            for entity in candidates {
                if upgraded >= 4 {
                    break;
                }
                let Ok((_, mut unit)) = units.get_mut(entity) else { continue };
                let here = unit.position;
                let equip = |to: UnitType| knows(to.required_tech()) && supplies.has(here, to.required_resources());
                let Some((cost, _)) = unit.refit_cost(equip) else { continue };
                if civ.gold < cost + 50 {
                    continue;
                }
                civ.gold -= cost;
                unit.refit(equip);
                upgraded += 1;
            }
            // At the size limit: stand down the weakest land unit if something twice as strong can be built.
            let military: Vec<(Entity, UnitType)> = units
                .iter()
                .filter(|(e, u)| !dead.contains(e) && u.owner == civ.id && u.unit_type.is_army() && u.troops.is_empty())
                .map(|(e, u)| (e, u.unit_type))
                .collect();
            let all_military = units.iter().filter(|(_, u)| u.owner == civ.id && u.unit_type.is_military()).count();
            if all_military >= governor::army_cap(city_count(civ.id)) {
                let best = UnitType::ALL
                    .into_iter()
                    .filter(|u| u.is_trainable() && u.is_army() && knows(u.required_tech()))
                    .map(power)
                    .max()
                    .unwrap_or(0);
                if let Some((entity, kind)) = military.iter().min_by_key(|(_, k)| power(*k)) {
                    let stuck = kind.best_upgrade(|to| knows(to.required_tech())).is_none();
                    if power(*kind) * 2 <= best && (stuck || civ.gold < 200) {
                        dead.insert(*entity);
                        commands.entity(*entity).try_despawn_recursive();
                    }
                }
            }
        }
    }
    // Global warming: a world fouled by pollution and fallout (more than 15 spoiled tiles) now and
    // then turns land drier - grassland to plains, plains and forest to desert and plains.
    {
        let fouled = tiles.iter().filter(|t| t.is_polluted()).count();
        let chance = (fouled.saturating_sub(15) as f64 * 0.004).min(0.3);
        if chance > 0.0 && rng.gen_bool(chance) {
            let city_tiles: HashSet<Hex> = cities.iter().map(|(c, _, _)| c.position).collect();
            let land: Vec<Hex> = tiles
                .iter()
                .filter(|t| matches!(t.terrain, Terrain::Grassland | Terrain::Plains | Terrain::Forest) && !city_tiles.contains(&t.coord))
                .map(|t| t.coord)
                .collect();
            let mut changed = 0;
            for _ in 0..3 {
                if land.is_empty() {
                    break;
                }
                let at = land[rng.gen_range(0..land.len())];
                if let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == at) {
                    tile.terrain = match tile.terrain {
                        Terrain::Grassland => Terrain::Plains,
                        Terrain::Plains => Terrain::Desert,
                        _ => Terrain::Plains,
                    };
                    map.terrain.insert(at, tile.terrain);
                    changed += 1;
                }
            }
            if changed > 0 {
                state.warming += 1;
                dirty.map = true;
                log.push(format!("Global warming: with {fouled} tiles of the world fouled by pollution and fallout, the climate shifts and land turns drier. Clean up pollution; Mass Transit and Recycling Centers prevent it."));
            }
        }
    }
    // Our sentries wake when an enemy comes within two tiles (and stop watching once given orders).
    {
        let threats: Vec<Hex> = units
            .iter()
            .filter(|(e, u)| !dead.contains(e) && u.owner != PLAYER && u.unit_type.is_army() && (u.owner == BARBARIANS || diplomacy.get_treaty(PLAYER, u.owner) == Treaty::War))
            .map(|(_, u)| u.position)
            .collect();
        for entity in &sentries {
            let Ok((_, mut unit)) = units.get_mut(entity) else { continue };
            if !unit.fortified {
                commands.entity(entity).remove::<Sentry>();
                continue;
            }
            if threats.iter().any(|t| t.distance_to(unit.position) <= 2) {
                unit.fortified = false;
                commands.entity(entity).remove::<Sentry>();
                log.push(format!("Our {} on sentry wakes: enemies are near.", unit.unit_type.name()));
            }
        }
    }
    // Computer players at war fire their nuclear missiles at the largest enemy city in range that no
    // SDI Defense protects.
    {
        let missiles: Vec<(Entity, u32, Hex)> = units
            .iter()
            .filter(|(e, u)| !dead.contains(e) && u.owner != PLAYER && u.unit_type == UnitType::NuclearMissile)
            .map(|(e, u)| (e, u.owner, u.position))
            .collect();
        for (entity, owner, at) in missiles {
            if !rng.gen_bool(0.3) {
                continue;
            }
            let shielded = |hex: Hex| cities.iter().any(|(c, b, _)| c.owner != owner && b.has(Building::SdiDefense) && c.position.distance_to(hex) <= SDI_RANGE);
            let target = cities
                .iter()
                .filter(|(c, _, _)| c.owner != owner && c.position.distance_to(at) <= MISSILE_RANGE)
                .filter(|(c, _, _)| c.owner == BARBARIANS || diplomacy.get_treaty(owner, c.owner) == Treaty::War)
                .filter(|(c, _, _)| !shielded(c.position))
                .max_by_key(|(c, _, _)| c.population)
                .map(|(c, _, _)| c.position);
            if let Some(target) = target {
                dead.insert(entity);
                commands.entity(entity).try_despawn_recursive();
                strikes.send(Detonation { at: target, by: owner });
            }
        }
    }
    // Computer troops standing on improved enemy land they are at war with sometimes pillage it.
    {
        let city_tiles: HashSet<Hex> = cities.iter().map(|(c, _, _)| c.position).collect();
        let raiders: Vec<(u32, Hex)> = units
            .iter()
            .filter(|(e, u)| !dead.contains(e) && u.owner != PLAYER && u.unit_type.is_army() && !city_tiles.contains(&u.position))
            .map(|(_, u)| (u.owner, u.position))
            .collect();
        for (owner, at) in raiders {
            let Some(&(land_owner, home)) = supplies.territory.get(&at) else { continue };
            let enemy = land_owner != owner && (owner == BARBARIANS || diplomacy.get_treaty(owner, land_owner) == Treaty::War);
            if !enemy || !rng.gen_bool(0.25) {
                continue;
            }
            let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == at) else { continue };
            let Some(target) = bevy_civ::terrain::pillage_target(&tile.improvements) else { continue };
            tile.improvements.retain(|i| *i != target);
            if land_owner == PLAYER {
                let near = cities.iter().find(|(c, _, _)| c.position == home).map(|(c, _, _)| c.name.clone()).unwrap_or_default();
                log.push(format!("{} raiders pillage the {} near {near}.", capitalised(&civ_name(owner)), bevy_civ::terrain::improvement_name(target)));
            }
        }
    }
    // --- Computer players' caravans and diplomats --------------------------------------------
    {
        let land: Vec<(Hex, Terrain)> = tiles.iter().map(|t| (t.coord, t.terrain)).collect();
        let city_info: Vec<(Hex, u32, u32, String, bool)> = cities
            .iter()
            .map(|(c, b, _)| (c.position, c.owner, c.population, c.name.clone(), b.has(Building::Walls) || wonders.walls_everywhere(c.owner)))
            .collect();
        let roads = road_network(tiles.iter(), city_info.iter().map(|c| c.0));
        let sites: Vec<(Hex, u32)> = cities
            .iter()
            .filter(|(c, _, _)| matches!(&c.production_queue, Some(ProductionItem::Building(b)) if b.is_wonder()))
            .map(|(c, _, _)| (c.position, c.owner))
            .collect();
        let envoys: Vec<(Entity, u32, UnitType, Hex)> = units
            .iter()
            .filter(|(e, u)| !dead.contains(e) && u.owner != PLAYER && u.owner != BARBARIANS && matches!(u.unit_type, UnitType::Caravan | UnitType::Diplomat | UnitType::Missionary))
            .map(|(e, u)| (e, u.owner, u.unit_type, u.position))
            .collect();
        let state_faiths: HashMap<u32, Faith> = city_info.iter().filter_map(|c| capitals.get(&c.1).filter(|cp| **cp == c.0).and_then(|cp| religions.of(*cp)).map(|f| (c.1, f))).collect();
        for (entity, owner, kind, here) in envoys {
            let Some(civ_index) = civs.players.iter().position(|c| c.id == owner) else { continue };
            let faith = state_faiths.get(&owner).copied();
            let at_peace = |other: u32| other != BARBARIANS && diplomacy.get_treaty(owner, other) != Treaty::War;
            let home = homes.get(entity).ok().map(|h| h.0).filter(|h| city_info.iter().any(|c| c.0 == *h && c.1 == owner));
            let home = home.or_else(|| city_info.iter().filter(|c| c.1 == owner).min_by_key(|c| c.0.distance_to(here)).map(|c| c.0));
            // Where to go, and whether that city is entered (own) or approached (foreign).
            let goal: Option<Hex> = match kind {
                UnitType::Caravan => sites
                    .iter()
                    .filter(|(_, o)| *o == owner)
                    .map(|(at, _)| *at)
                    .min_by_key(|at| at.distance_to(here))
                    .or_else(|| {
                        let home = home?;
                        let pop = city_info.iter().find(|c| c.0 == home).map(|c| c.2).unwrap_or(1);
                        city_info
                            .iter()
                            .filter(|c| (c.1 == owner || at_peace(c.1)) && trade.refusal(home, c.0, owner, c.1).is_none())
                            .max_by_key(|c| (bevy_civ::trade::route_yield(home.distance_to(c.0), pop, c.2, c.1 != owner).0, -c.0.distance_to(here)))
                            .map(|c| c.0)
                    }),
                // A missionary: our own cities without our faith first, then faithless foreign cities at peace.
                UnitType::Missionary => faith.and_then(|f| {
                    city_info
                        .iter()
                        .filter(|c| c.1 == owner && religions.of(c.0) != Some(f))
                        .min_by_key(|c| c.0.distance_to(here))
                        .or_else(|| city_info.iter().filter(|c| c.1 != owner && at_peace(c.1) && religions.of(c.0).is_none()).min_by_key(|c| c.0.distance_to(here)))
                        .map(|c| c.0)
                }),
                _ => city_info
                    .iter()
                    .filter(|c| c.1 != owner && c.1 != BARBARIANS && diplomacy.get_treaty(owner, c.1) != Treaty::Alliance)
                    .min_by_key(|c| (diplomacy.get_treaty(owner, c.1) != Treaty::War, c.0.distance_to(here)))
                    .map(|c| c.0),
            };
            let Some(goal) = goal else { continue };
            let foreign_goal = city_info.iter().any(|c| c.0 == goal && c.1 != owner);
            let arrived = if foreign_goal { here.distance_to(goal) <= 1 } else { here == goal };
            if !arrived {
                if let Some(route) = find_route(here, goal, land.iter().copied(), &roads, Travel::Land) {
                    let stop = if foreign_goal { route.len().saturating_sub(1) } else { route.len() };
                    // Never into a tile someone else holds.
                    let blocked = route.iter().skip(1).position(|h| units.iter().any(|(e, u)| !dead.contains(&e) && u.position == *h && u.owner != owner)).map(|i| i + 1);
                    let stop = blocked.map_or(stop, |b| b.min(stop));
                    if let Ok((_, mut unit)) = units.get_mut(entity) {
                        let mut trail = Vec::new();
                        walk(&mut unit, &route[..stop.max(1)], &map, &roads, &HashSet::new(), &mut fog, &mut trail);
                        commands.entity(entity).try_insert(Trail(trail));
                    }
                }
                continue;
            }
            let Some((_, partner, partner_pop, city_name, walls)) = city_info.iter().find(|c| c.0 == goal).cloned() else { continue };
            if kind == UnitType::Missionary {
                if let Some(f) = faith {
                    religions.city.insert(goal, f);
                    if partner == PLAYER {
                        log.push(format!("{} missionaries bring {} to {city_name}.", capitalised(&civs.players[civ_index].adjective), f.name()));
                    }
                }
                dead.insert(entity);
                commands.entity(entity).try_despawn_recursive();
                continue;
            }
            match kind {
                UnitType::Caravan if partner == owner && sites.contains(&(goal, owner)) => {
                    if let Some((mut city, _, _)) = cities.iter_mut().find(|(c, _, _)| c.position == goal) {
                        city.production_progress += UnitType::Caravan.cost();
                    }
                    commands.entity(entity).try_despawn_recursive();
                }
                UnitType::Caravan => {
                    let Some(home) = home else { continue };
                    let pop = city_info.iter().find(|c| c.0 == home).map(|c| c.2).unwrap_or(1);
                    if trade.establish((home, owner, pop), (goal, partner, partner_pop)).is_ok() && partner == PLAYER {
                        log.push(format!("A {} caravan opens a trade route to {}: we earn from it too.", civ_name(owner), city_name));
                    }
                    commands.entity(entity).try_despawn_recursive();
                }
                _ => {
                    // A diplomat: an embassy first, then technology, then sabotage (at war) or gold.
                    let thief = &civs.players[civ_index].tech;
                    let loot = if partner == PLAYER {
                        espionage::stealable(thief, &tech)
                    } else {
                        civs.players.iter().find(|c| c.id == partner).map(|c| espionage::stealable(thief, &c.tech)).unwrap_or_default()
                    };
                    // False flags now and then: an aggressive people stages an incident to justify a
                    // war, and one at war frames its enemy in a neutral city to drag that city's owner in.
                    let aggression = civs.players[civ_index].aggression;
                    let at_peace = diplomacy.get_treaty(owner, partner) != Treaty::War;
                    let frame_target = civs
                        .players
                        .iter()
                        .map(|c| c.id)
                        .chain(std::iter::once(PLAYER))
                        .filter(|x| *x != owner && *x != partner && !civs.players.iter().any(|c| c.id == *x && c.eliminated))
                        .filter(|x| diplomacy.get_treaty(owner, *x) == Treaty::War && diplomacy.get_treaty(partner, *x) != Treaty::War)
                        .min();
                    let mission = if !spies.has_embassy(owner, partner) {
                        Mission::Embassy
                    } else if aggression > 0.6 && at_peace && foreign.grievance(owner, partner, state.turn).is_none() && rng.gen_bool(0.25) {
                        Mission::StageIncident
                    } else if let Some(x) = frame_target.filter(|_| at_peace && rng.gen_bool(0.3)) {
                        Mission::Frame(x)
                    } else if !loot.is_empty() {
                        Mission::StealTech
                    } else if diplomacy.get_treaty(owner, partner) == Treaty::War {
                        Mission::Sabotage
                    } else {
                        Mission::StealGold
                    };
                    commands.entity(entity).try_despawn_recursive();
                    if mission == Mission::Embassy {
                        spies.embassies.insert((owner, partner));
                        continue;
                    }
                    if rng.gen::<f32>() >= mission.odds(walls) {
                        diplomacy.change_attitude(owner, partner, espionage::CAUGHT_ATTITUDE);
                        foreign.add_grievance(partner, owner, foreign::Wrong::SpyCaught, state.turn);
                        if let Mission::Frame(framed) = mission {
                            diplomacy.change_attitude(owner, framed, espionage::CAUGHT_ATTITUDE);
                            foreign.add_grievance(framed, owner, foreign::Wrong::SpyCaught, state.turn);
                        }
                        if partner == PLAYER {
                            let what = if mission.is_false_flag() { "staging a false flag" } else { "spying" };
                            log.push(format!("We caught a {} diplomat {what} in {}! We have a just cause for war against them.", civ_name(owner), city_name));
                        } else if mission == Mission::Frame(PLAYER) {
                            log.push(format!("Rumours say the {} tried to blame us for an attack in {}. We have a just cause for war against them.", civ_name(owner), city_name));
                        }
                        continue;
                    }
                    match mission {
                        Mission::StageIncident => {
                            foreign.add_grievance(owner, partner, foreign::Wrong::BorderIncident, state.turn);
                            if partner == PLAYER {
                                log.push(format!("The {} accuse us of attacking their troops near {} - we did no such thing. They now claim a just cause for war.", civ_name(owner), city_name));
                            }
                        }
                        Mission::Frame(framed) => {
                            diplomacy.change_attitude(partner, framed, espionage::FRAMED_ATTITUDE);
                            foreign.add_grievance(partner, framed, foreign::Wrong::BorderIncident, state.turn);
                            if partner == PLAYER {
                                log.push(format!("An attack in {} - by the {}, it seems. We have a just cause for war against them (if they really did it).", city_name, civ_name(framed)));
                            } else if framed == PLAYER {
                                log.push(format!("Rumours reach us that we are blamed for an attack in {}. We did not do it.", city_name));
                            }
                        }
                        Mission::StealTech => {
                            let stolen = loot[rng.gen_range(0..loot.len())];
                            civs.players[civ_index].tech.learn(stolen);
                            if partner == PLAYER {
                                log.push(format!("A {} diplomat stole the secret of {} from {}!", civ_name(owner), stolen.name(), city_name));
                            }
                        }
                        Mission::Sabotage => {
                            if let Some((mut city, _, _)) = cities.iter_mut().find(|(c, _, _)| c.position == goal) {
                                city.production_progress = 0;
                            }
                            if partner == PLAYER {
                                log.push(format!("Saboteurs from the {} wrecked the work in {}!", civ_name(owner), city_name));
                            }
                        }
                        _ => {
                            let amount = if partner == PLAYER {
                                let amount = espionage::stolen_gold(state.gold);
                                state.gold -= amount;
                                log.push(format!("A {} diplomat made off with {amount} gold from {}!", civ_name(owner), city_name));
                                amount
                            } else if let Some(victim) = civs.players.iter_mut().find(|c| c.id == partner) {
                                let amount = espionage::stolen_gold(victim.gold);
                                victim.gold -= amount;
                                amount
                            } else {
                                0
                            };
                            civs.players[civ_index].gold += amount;
                        }
                    }
                }
            }
        }
    }

    let player_cities = cities.iter().filter(|(c, _, _)| c.owner == PLAYER).count();

    // --- Diplomacy -----------------------------------------------------------------------------
    let city_owners: Vec<(Hex, u32)> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();
    let unit_owners: Vec<(Hex, u32)> = units.iter().filter(|(e, _)| !dead.contains(e)).map(|(_, u)| (u.position, u.owner)).collect();
    // Troops standing in someone else's lands: theirs in ours, and ours in theirs.
    foreign.trespass.clear();
    foreign.our_trespass.clear();
    for (_, unit) in units.iter().filter(|(e, u)| !dead.contains(e) && u.unit_type.is_military() && u.carrier.is_none()) {
        let Some(&(land_owner, _)) = supplies.territory.get(&unit.position) else { continue };
        if land_owner == PLAYER && unit.owner != PLAYER && unit.owner != BARBARIANS {
            *foreign.trespass.entry(unit.owner).or_default() += 1;
        } else if unit.owner == PLAYER && land_owner != PLAYER && land_owner != BARBARIANS {
            *foreign.our_trespass.entry(land_owner).or_default() += 1;
        }
    }
    foreign.strength.clear();
    for (entity, unit) in units.iter() {
        if !dead.contains(&entity) {
            *foreign.strength.entry(unit.owner).or_default() += bevy_civ::ai::unit_strength(unit);
        }
    }
    for (_, owner) in &city_owners {
        *foreign.strength.entry(*owner).or_default() += 2.0;
    }
    foreign.in_flight = space.ships.iter().filter(|(_, s)| s.arrives.is_some()).map(|(o, _)| *o).collect();
    {
        let mut ctx = foreign::Ctx { diplomacy: &mut diplomacy, foreign: &mut foreign, civs: &mut civs, state: &mut state, tech: &mut tech, log: &mut log };
        foreign::turn(&mut ctx, &fog, &supplies, &unit_owners, &city_owners);
    }

    let all_positions: Vec<Hex> = units.iter().filter(|(e, _)| !dead.contains(e)).map(|(_, u)| u.position).collect();
    let living: Vec<usize> = civs.players.iter().filter(|c| !c.eliminated && c.id != PLAYER).map(|c| c.tech.researched.len()).chain(std::iter::once(tech.researched.len())).collect();
    let world_techs = living.iter().sum::<usize>() / living.len().max(1);
    process_barbarians(&mut barbarians, &mut commands, &tiles, &all_positions, &supplies.territory, state.turn, world_techs);

    let unit_positions: Vec<Hex> = units.iter().filter(|(e, u)| u.owner == PLAYER && !dead.contains(e)).map(|(_, u)| u.position).collect();
    let city_positions: Vec<Hex> = cities.iter().filter(|(c, _, _)| c.owner == PLAYER).map(|(c, _, _)| c.position).collect();
    fog.update_visibility(&unit_positions, &city_positions);
    cycle.advance = true;

    if let Some(completed) = tech.process_turn() {
        log.push(format!("Discovery! We have learned {}.", completed.name()));
        if let Some(government) = Government::CHOICES.into_iter().find(|g| g.required_tech() == Some(completed)) {
            log.push(format!("We may now adopt {}: open the Empire advisor (Y) to start a revolution.", government.name()));
        }
    }
    {
        let holding: HashSet<u32> = cities.iter().map(|(c, _, _)| c.owner).collect();
        great_library(&mut civs, &mut tech, &wonders, &foreign.met, &holding, &mut log);
        new_ages(&mut civs, &mut tech, &foreign.met, &holding, &mut log);
    }

    // A revolution ends after its turns of anarchy.
    if state.anarchy_turns > 0 {
        state.anarchy_turns -= 1;
        if state.anarchy_turns == 0 {
            if let Some(next) = state.pending_government.take() {
                state.government = next;
                // The rates are brought within what the new government allows.
                let rates = state.rates();
                state.set_rates(rates);
                log.push(format!("The anarchy ends: we are now a {}.", next.name()));
            }
        }
    }

    // A capital that fell takes its civilization's spaceship with it.
    for civ_id in std::iter::once(PLAYER).chain(civs.players.iter().map(|c| c.id)) {
        let has_capital = cities.iter().any(|(c, _, _)| c.owner == civ_id && c.is_capital);
        if !has_capital && space.lose(civ_id) {
            log.push(if civ_id == PLAYER { "With our capital lost, our spaceship is lost too.".to_string() } else { "A spaceship in flight has been lost with its capital.".to_string() });
        }
    }

    // A people with no city and no settler left is out of the game: its last troops lay down
    // their arms and it leaves the councils of the world.
    {
        let alive: HashSet<u32> = cities
            .iter()
            .map(|(c, _, _)| c.owner)
            .chain(units.iter().filter(|(e, u)| !dead.contains(e) && u.unit_type == UnitType::Settler).map(|(_, u)| u.owner))
            .collect();
        for civ in civs.players.iter_mut().filter(|c| !c.is_human && !c.eliminated && !alive.contains(&c.id)) {
            civ.eliminated = true;
            for (entity, unit) in units.iter() {
                if unit.owner == civ.id {
                    commands.entity(entity).try_despawn_recursive();
                }
            }
            if foreign.met.remove(&civ.id) {
                log.push(format!("{} are no more: their last city has fallen.", capitalised(&civ.name)));
            }
            diplomacy.treaties.retain(|(a, b), _| *a != civ.id && *b != civ.id);
            diplomacy.passage.retain(|(a, b)| *a != civ.id && *b != civ.id);
        }
    }

    // The automatic save, every turn.
    saves.send(savegame::SaveRequest { download: false });
    state.score = Score::calculate(cities.iter().map(|(c, b, _)| (c, b)), &tech);
    history.scores.push((state.turn, PLAYER, state.score.total));
    for civ in civs.players.iter().filter(|c| !c.is_human) {
        history.scores.push((state.turn, civ.id, rival_score(civ, &cities)));
    }

    let total_cities = cities.iter().count();
    let rivals_remain = cities.iter().any(|(c, _, _)| c.owner != PLAYER && c.owner != BARBARIANS);
    let civ_named = |id: u32| civs.players.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_default();
    if observer.is_some() {
        // Nobody plays: any civilization can win by conquest or domination too.
        let mut held: HashMap<u32, usize> = HashMap::new();
        for (city, _, _) in cities.iter().filter(|(c, _, _)| c.owner != BARBARIANS) {
            *held.entry(city.owner).or_default() += 1;
        }
        let leader = held.iter().max_by_key(|(_, n)| **n).map(|(o, n)| (*o, *n));
        let culture_winner = culture.winner(&cities.iter().filter(|(c, _, _)| c.owner != BARBARIANS).map(|(c, _, _)| (c.position, c.owner)).collect::<Vec<_>>(), state.last_turn);
        state.victory = if held.len() == 1 {
            leader.map(|(o, _)| format!("Conquest victory for {}.", civ_named(o)))
        } else if let Some((o, n)) = leader.filter(|(_, n)| dominates(*n, total_cities)) {
            Some(format!("Domination victory for {} ({n} of {total_cities} cities).", civ_named(o)))
        } else if let Some(o) = culture_winner {
            Some(format!("Cultural victory for {}.", civ_named(o)))
        } else if let Some(o) = space.arrived(state.turn) {
            Some(format!("Space race victory for {}.", civ_named(o)))
        } else if state.turn >= state.last_turn {
            let best = civs.players.iter().filter(|c| !c.is_human).map(|c| (rival_score(c, &cities), c.name.clone())).max_by_key(|(s, _)| *s);
            best.map(|(s, name)| format!("Score victory for {name} ({s}) at the turn limit."))
        } else {
            None
        };
    } else if player_cities == 0 {
        state.victory = Some("Your civilization has fallen.".into());
    } else if let Some((name, held)) = {
        // A rival holding two in five of the world's cities dominates it.
        let mut held: HashMap<u32, usize> = HashMap::new();
        for (city, _, _) in cities.iter().filter(|(c, _, _)| c.owner != PLAYER && c.owner != BARBARIANS) {
            *held.entry(city.owner).or_default() += 1;
        }
        held.into_iter().filter(|(_, n)| dominates(*n, total_cities)).max_by_key(|(_, n)| *n).map(|(o, n)| (civ_named(o), n))
    } {
        state.victory = Some(format!("Defeat: {name} dominate the world, holding {held} of its {total_cities} cities."));
    } else if !rivals_remain {
        state.victory = Some("Conquest Victory!".into());
    } else if dominates(player_cities, total_cities) {
        state.victory = Some("Domination Victory!".into());
    } else if let Some(winner) = culture.winner(&cities.iter().filter(|(c, _, _)| c.owner != BARBARIANS).map(|(c, _, _)| (c.position, c.owner)).collect::<Vec<_>>(), state.last_turn) {
        state.victory = Some(if winner == PLAYER {
            "Cultural Victory! Our civilization is the envy of the world.".to_string()
        } else {
            let name = civs.players.iter().find(|c| c.id == winner).map(|c| c.name.clone()).unwrap_or_default();
            format!("Defeat: {name} have won a cultural victory.")
        });
    } else if let Some(winner) = space.arrived(state.turn) {
        state.victory = Some(if winner == PLAYER {
            "Space Race Victory! Our colonists have reached Alpha Centauri.".to_string()
        } else {
            let name = civs.players.iter().find(|c| c.id == winner).map(|c| c.name.clone()).unwrap_or_default();
            format!("Defeat: {name} reached Alpha Centauri first.")
        });
    } else if state.turn >= state.last_turn {
        // Time runs out: the highest score wins.
        let ours = state.score.total;
        let best_rival = civs.players.iter().filter(|c| !c.is_human).map(|c| (rival_score(c, &cities), c.name.clone())).max_by_key(|(s, _)| *s);
        state.victory = Some(match best_rival {
            Some((theirs, name)) if theirs > ours => format!("Time is up: {name} lead the world on score ({theirs} to our {ours})."),
            _ => format!("Score Victory! Time is up and we lead the world with {ours} points."),
        });
    }

    // The United Nations: every 10 turns once it stands, a vote for Secretary-General between its
    // owner and the highest-scoring other people. Two thirds of all the votes win the game.
    if state.victory.is_none() && state.turn % 10 == 0 {
        if let Some(&(un_owner, _)) = wonders.built.get(&Building::UnitedNations) {
            let holding: HashSet<u32> = cities.iter().map(|(c, _, _)| c.owner).filter(|o| *o != BARBARIANS).collect();
            let voters: Vec<u32> = holding.iter().copied().filter(|o| observer.is_none() || *o != PLAYER).collect();
            let score_of = |id: u32| if id == PLAYER { state.score.total } else { civs.players.iter().find(|c| c.id == id).map(|c| rival_score(c, &cities)).unwrap_or(0) };
            let rival = voters.iter().copied().filter(|o| *o != un_owner).max_by_key(|o| score_of(*o));
            if let (Some(rival), true) = (rival, holding.contains(&un_owner)) {
                let (tally, winner) = bevy_civ::diplomacy::un_vote(&voters, [un_owner, rival], |a, b| diplomacy.get_attitude(a, b));
                let name = |id: u32| if id == PLAYER { "we".to_string() } else { civ_named(id) };
                let needed = bevy_civ::diplomacy::un_votes_needed(voters.len());
                log.push(format!(
                    "The {} votes for Secretary-General: {} {}, {} {} ({needed} of {} needed).",
                    Building::UnitedNations.name(),
                    capitalised(&name(un_owner)),
                    tally[0],
                    name(rival),
                    tally[1],
                    voters.len()
                ));
                if let Some(winner) = winner {
                    state.victory = Some(if observer.is_some() {
                        format!("Diplomatic victory for {}.", civ_named(winner))
                    } else if winner == PLAYER {
                        "Diplomatic Victory! The nations of the world elect us to lead them.".to_string()
                    } else {
                        format!("Defeat: the {} elects {} to lead the world.", Building::UnitedNations.name(), civ_named(winner))
                    });
                }
            }
        }
    }
}

/// Moves the selection on to the next unit that needs orders and points the camera at it.
fn advance_selection(
    mut cycle: ResMut<Cycle>,
    mut selection: ResMut<Selection>,
    units: Query<(Entity, &Unit)>,
    cities: Query<(Entity, &City)>,
    state: Res<GameState>,
    mut focus: EventWriter<Focus>,
    mut dirty: ResMut<Dirty>,
    theatre: Option<Res<client::battles::BattleTheatre>>,
    mut fresh_battles: EventReader<BattleShown>,
) {
    // Wait while a battle is being shown (or has just been fought), so the camera does not leave it.
    let fought = fresh_battles.read().count() > 0;
    if !cycle.advance || fought || theatre.is_some_and(|t| t.busy()) {
        return;
    }
    // A new game's units are not in the world yet: wait for them, then select the first.
    if !units.iter().any(|(_, u)| u.owner == PLAYER) {
        return;
    }
    let force = std::mem::take(&mut cycle.force);
    cycle.advance = false;
    if selection.city.is_some() || state.victory.is_some() {
        return;
    }
    let current = selection.unit.and_then(|e| units.get(e).ok());
    if current.is_some_and(|(_, u)| needs_orders(u)) && !force {
        return;
    }
    let mut waiting: Vec<(Entity, Hex)> = units.iter().filter(|(_, u)| needs_orders(u)).map(|(e, u)| (e, u.position)).collect();
    waiting.sort_by_key(|(e, _)| e.index());
    let after = selection.unit.map(|e| e.index()).unwrap_or(0);
    let next = waiting.iter().find(|(e, _)| e.index() > after).or(waiting.first()).copied();
    match next {
        Some((entity, position)) => {
            if selection.unit != Some(entity) {
                selection.unit = Some(entity);
                focus.send(Focus(position));
            }
        }
        // No unit needs orders: a city waiting for a project comes next.
        None => match idle_city(cities.iter()) {
            Some((entity, position)) => {
                selection.unit = None;
                selection.city = Some(entity);
                focus.send(Focus(position));
            }
            // Nothing left to order: release a unit that has finished so the turn can end.
            None if current.is_some_and(|(_, u)| u.owner == PLAYER) => selection.unit = None,
            None => {}
        },
    }
    dirty.ui = true;
}

/// One of the player's cities that has nothing to build.
fn idle_city<'a>(cities: impl Iterator<Item = (Entity, &'a City)>) -> Option<(Entity, Hex)> {
    cities.filter(|(_, c)| c.owner == PLAYER && c.production_queue.is_none()).min_by_key(|(e, _)| e.index()).map(|(e, c)| (e, c.position))
}

/// Religion's turn, after the rest of the turn: faiths are founded by the first to learn their
/// technology, spread quietly to nearby cities, pay their holy city's owner, and colour diplomacy.
#[allow(clippy::too_many_arguments)]
pub fn religion_turn(
    mut state: ResMut<GameState>,
    mut last: Local<u32>,
    mut tech: ResMut<TechState>,
    mut civs: ResMut<Civilizations>,
    cities: Query<&City>,
    mut religions: ResMut<Religions>,
    mut diplomacy: ResMut<Diplomacy>,
    mut log: ResMut<GameLog>,
) {
    if *last == state.turn || state.victory.is_some() {
        return;
    }
    let fresh = *last == 0;
    *last = state.turn;
    let mut rng = rand::thread_rng();
    let city_list: Vec<(Hex, u32, String, bool, u32)> = cities.iter().map(|c| (c.position, c.owner, c.name.clone(), c.is_capital, c.population)).collect();
    let positions: Vec<Hex> = city_list.iter().map(|c| c.0).collect();
    religions.retain(&positions);
    let adjectives: HashMap<u32, String> = civs.players.iter().map(|c| (c.id, c.adjective.clone())).collect();
    let name_of = |id: u32| adjectives.get(&id).cloned().unwrap_or_default();

    // Founding: the first civilization to know a faith's technology founds it in one of its cities
    // that has none yet (the capital if it can).
    let unfounded: Vec<Faith> = Faith::ALL.into_iter().filter(|f| !religions.is_founded(*f)).collect();
    for faith in unfounded {
        let knows = |id: u32| if id == PLAYER { tech.researched.contains(&faith.founding_tech()) } else { civs.players.iter().any(|c| c.id == id && c.tech.researched.contains(&faith.founding_tech())) };
        let knowers: Vec<u32> = civs.players.iter().map(|c| c.id).filter(|id| knows(*id) && city_list.iter().any(|c| c.1 == *id)).collect();
        if knowers.is_empty() {
            continue;
        }
        let founder = knowers[rng.gen_range(0..knowers.len())];
        let mut own: Vec<&(Hex, u32, String, bool, u32)> = city_list.iter().filter(|c| c.1 == founder).collect();
        own.sort_by_key(|c| (religions.of(c.0).is_some(), !c.3, std::cmp::Reverse(c.4)));
        let Some(site) = own.first() else { continue };
        religions.found(faith, site.0);
        if founder == PLAYER {
            log.push(format!(
                "We have founded {} in {}! It will spread to nearby cities; Missionaries carry it further. Cities that share our capital's faith are calmer.",
                faith.name(),
                site.2
            ));
        } else {
            log.push(format!("{} priests found {} in {}.", capitalised(&name_of(founder)), faith.name(), site.2));
        }
        // The first people to master Philosophy (the founders of its faith) also make a free discovery.
        if faith.founding_tech() == Tech::Philosophy && !fresh {
            let gift = if founder == PLAYER { tech.discover() } else { civs.players.iter_mut().find(|c| c.id == founder).and_then(|c| c.tech.discover()) };
            if let Some(gift) = gift {
                log.push(if founder == PLAYER {
                    format!("As the first to master {}, our thinkers make a free discovery: {}!", Tech::Philosophy.name(), gift.name())
                } else {
                    format!("As the first to master {}, the {} thinkers make a free discovery.", Tech::Philosophy.name(), name_of(founder))
                });
            }
        }
    }
    if fresh {
        return;
    }

    // Quiet spreading.
    for (at, faith) in religions.spread(&positions, &mut rng) {
        if let Some(city) = city_list.iter().find(|c| c.0 == at && c.1 == PLAYER) {
            log.push(format!("{} takes up {}.", city.2, faith.name()));
        }
    }

    // The owner of each holy city earns gold from the faith's followers.
    for faith in Faith::ALL {
        let Some(holy) = religions.holy.get(&faith).copied() else { continue };
        let Some(owner) = city_list.iter().find(|c| c.0 == holy).map(|c| c.1) else { continue };
        let gold = religions.holy_city_gold(faith);
        if owner == PLAYER {
            state.gold += gold;
        } else if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
            civ.gold += gold;
        }
    }

    // Every ten turns, peoples of one faith grow closer and peoples of different faiths drift apart.
    if state.turn % 10 == 0 {
        let state_faith: HashMap<u32, Faith> = city_list.iter().filter(|c| c.3).filter_map(|c| religions.of(c.0).map(|f| (c.1, f))).collect();
        let ids: Vec<u32> = civs.players.iter().map(|c| c.id).collect();
        for (i, a) in ids.iter().enumerate() {
            for b in ids.iter().skip(i + 1) {
                match (state_faith.get(a), state_faith.get(b)) {
                    (Some(x), Some(y)) if x == y => diplomacy.change_attitude(*a, *b, 3),
                    (Some(_), Some(_)) => diplomacy.change_attitude(*a, *b, -2),
                    _ => {}
                }
            }
        }
    }
}

/// A unit walking into a tribal village: gold, knowledge, warriors who join, maps - or an ambush.
#[allow(clippy::too_many_arguments)]
pub fn visit_villages(
    mut villages: ResMut<Villages>,
    units: Query<&Unit>,
    mut state: ResMut<GameState>,
    mut tech: ResMut<TechState>,
    mut civs: ResMut<Civilizations>,
    mut fog: ResMut<FogOfWar>,
    map: Res<MapData>,
    (mut log, mut dirty, last_move): (ResMut<GameLog>, ResMut<Dirty>, Option<ResMut<LastMove>>),
    mut commands: Commands,
) {
    if villages.sites.is_empty() {
        return;
    }
    let mut last_move = last_move;
    let visits: Vec<(u32, Hex, UnitType)> = units
        .iter()
        .filter(|u| u.owner != BARBARIANS && u.unit_type.domain() == Domain::Land && villages.sites.contains(&u.position))
        .map(|u| (u.owner, u.position, u.unit_type))
        .collect();
    let mut rng = rand::thread_rng();
    for (owner, at, kind) in visits {
        if !villages.sites.remove(&at) {
            continue;
        }
        // What a village gives cannot be taken back.
        if owner == PLAYER {
            if let Some(last) = last_move.as_mut() {
                last.0 = None;
            }
        }
        dirty.map = true;
        let mine = owner == PLAYER;
        let knows = |t: Tech| if mine { tech.researched.contains(&t) } else { civs.players.iter().any(|c| c.id == owner && c.tech.researched.contains(&t)) };
        let soldier = if knows(Tech::Gunpowder) { UnitType::Musketman } else if knows(Tech::BronzeWorking) { UnitType::Spearman } else { UnitType::Warrior };
        let raider = if knows(Tech::Gunpowder) { UnitType::Musketman } else if knows(Tech::IronWorking) { UnitType::Legion } else { UnitType::Archer };
        let mut find = bevy_civ::villages::roll(kind == UnitType::Scout, &mut rng);
        let open_land: Vec<Hex> = at.all_neighbors().into_iter().filter(|h| map.cells.contains(h) && !map.water.contains(h)).collect();
        if find == Find::Hostile && open_land.is_empty() {
            find = Find::Gold(25);
        }
        match find {
            Find::Gold(gold) => {
                if mine {
                    state.gold += gold;
                    log.push(format!("The villagers share their treasure: {gold} gold."));
                } else if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
                    civ.gold += gold;
                }
            }
            Find::Technology => {
                let learned = if mine {
                    let options: Vec<Tech> = Tech::ALL.into_iter().filter(|t| tech.can_research(*t)).collect();
                    let pick = options.get(rng.gen_range(0..options.len().max(1))).copied();
                    if let Some(t) = pick {
                        tech.learn(t);
                    }
                    pick
                } else {
                    civs.players.iter_mut().find(|c| c.id == owner).and_then(|civ| {
                        let options: Vec<Tech> = Tech::ALL.into_iter().filter(|t| civ.tech.can_research(*t)).collect();
                        let pick = options.get(rng.gen_range(0..options.len().max(1))).copied();
                        if let Some(t) = pick {
                            civ.tech.learn(t);
                        }
                        pick
                    })
                };
                match learned {
                    Some(t) if mine => log.push(format!("The village elders teach us {}!", t.name())),
                    None if mine => {
                        state.gold += 50;
                        log.push("The elders have nothing left to teach us, and give us 50 gold instead.");
                    }
                    _ => {}
                }
            }
            Find::Warriors => {
                spawn_unit(&mut commands, at, soldier, owner);
                if mine {
                    log.push(format!("Young warriors of the village join us: a {}.", soldier.name()));
                }
            }
            Find::Maps => {
                if mine {
                    fog.explored.extend(hexx::shapes::hexagon(at, 7).filter(|h| map.cells.contains(h)));
                    log.push("The villagers draw us maps of the lands around.");
                } else if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
                    civ.gold += 30;
                }
            }
            Find::Hostile => {
                for hex in open_land.iter().take(2) {
                    spawn_unit(&mut commands, *hex, raider, BARBARIANS);
                }
                if mine {
                    log.push(format!("The villagers were hostile! {} tribesmen take up arms around us.", open_land.len().min(2)));
                }
            }
        }
    }
}

/// Share of the world's cities (in fifths, at least five cities) that wins by domination.
pub const DOMINATION_FIFTHS: usize = 2;

/// Whether holding `held` of the world's `total` cities is a domination victory.
/// Great generals a computer player keeps at a time.
pub const GENERALS_KEPT: usize = 3;

pub fn dominates(held: usize, total: usize) -> bool {
    held >= 5 && held * 5 >= total * DOMINATION_FIFTHS
}

/// Puts every city's working citizens on tiles of its area: the player's own choices are kept (and
/// gaps filled by the governor); computer players' cities are re-planned each time. Runs whenever
/// borders, cities, the land or units change, and stores each city's worked tiles and their yield.
#[allow(clippy::too_many_arguments)]
/// What a city's governor weighs in choosing its focus (see `bevy_civ::tiles::suggest`).
pub fn city_situation(city: &City, buildings: &CityBuildings, happiness: Option<&Happiness>, at_war: bool, rivers: &Rivers) -> bevy_civ::tiles::Situation {
    let fresh_water = rivers.tiles.contains(&city.position) || city.position.all_neighbors().iter().any(|n| rivers.tiles.contains(n));
    let capped = city.population >= Building::size_limit(&buildings.buildings, fresh_water);
    // One more citizen would tip the city into disorder.
    let restless = happiness.is_some_and(|h| h.unhappy + 1 > h.happy && h.unhappy > 0);
    let project_turns = city
        .production_queue
        .as_ref()
        .map(|item| item.cost().saturating_sub(city.production_progress).div_ceil(city.land.1.max(1)))
        .unwrap_or(0);
    bevy_civ::tiles::Situation {
        population: city.population,
        growth_blocked: capped || restless,
        project_turns,
        building_wonder: matches!(&city.production_queue, Some(ProductionItem::Building(b)) if b.is_wonder()),
        arming_for_war: at_war && matches!(&city.production_queue, Some(ProductionItem::Unit(u)) if u.is_military()),
        trade_buildings: [Building::Library, Building::Marketplace, Building::University].iter().any(|b| buildings.has(*b)),
    }
}

/// The focus a city's land is worked for: its own, or the governor's suggestion on Auto.
pub fn effective_focus(city: &City, situation: &bevy_civ::tiles::Situation) -> bevy_civ::tiles::Focus {
    match city.focus {
        bevy_civ::tiles::Focus::Auto => bevy_civ::tiles::suggest(situation).0,
        own => own,
    }
}

pub fn work_tiles(
    (supplies, rivers, tech, civs, diplomacy): (Res<Supplies>, Res<Rivers>, Res<TechState>, Res<Civilizations>, Res<Diplomacy>),
    (state, golden, mut last_rules): (Res<GameState>, Res<GoldenAges>, Local<Vec<(u32, Government, bool)>>),
    tiles: Query<&HexTile>,
    units: Query<&Unit>,
    mut cities: Query<(&mut City, &CityBuildings, Option<&Happiness>)>,
    (changed_tiles, changed_units): (Query<(), Changed<HexTile>>, Query<(), Changed<Unit>>),
) {
    // Each people's government (and golden age) changes what its land gives.
    let land_rule = |owner: u32| -> (Government, bool) {
        let gov = if owner == PLAYER { state.government } else { civs.players.iter().find(|c| c.id == owner).map_or(Government::Despotism, |c| c.government) };
        (gov, golden.active(owner))
    };
    let rules: Vec<(u32, Government, bool)> = civs.players.iter().map(|c| (c.id, land_rule(c.id).0, land_rule(c.id).1)).collect();
    let rules_changed = *last_rules != rules;
    let city_changed = cities.iter_mut().any(|(c, _, _)| c.is_changed());
    if !(supplies.is_changed() || city_changed || golden.is_changed() || !changed_tiles.is_empty() || !changed_units.is_empty() || rules_changed) {
        return;
    }
    *last_rules = rules;
    let tile_at: HashMap<Hex, &HexTile> = tiles.iter().map(|t| (t.coord, t)).collect();
    let knows = |owner: u32, t: Tech| if owner == PLAYER { tech.researched.contains(&t) } else { civs.players.iter().any(|c| c.id == owner && c.tech.researched.contains(&t)) };
    let yield_for = |owner: u32, hex: Hex| -> Option<bevy_civ::tiles::Yield> {
        let t = tile_at.get(&hex)?;
        let resource = t.resource.filter(|r| r.reveal_tech().is_none_or(|need| knows(owner, need)));
        Some(bevy_civ::tiles::tile_yield(t.terrain, &t.improvements, resource, rivers.tiles.contains(&hex)))
    };
    let mut areas: HashMap<Hex, Vec<Hex>> = HashMap::new();
    for (hex, (_, home)) in &supplies.territory {
        areas.entry(*home).or_default().push(*hex);
    }
    let occupants: Vec<(Hex, u32)> = units.iter().filter(|u| u.unit_type.is_army()).map(|u| (u.position, u.owner)).collect();
    let at_war: HashSet<u32> = diplomacy.treaties.iter().filter(|(_, t)| **t == Treaty::War).flat_map(|((a, b), _)| [*a, *b]).collect();
    for (mut city, buildings, happiness) in &mut cities {
        let owner = city.owner;
        let focus = effective_focus(&city, &city_situation(&city, buildings, happiness, at_war.contains(&owner), &rivers));
        // A Harbor makes every worked ocean tile one food richer, and the Colossus one gold.
        let harbor = buildings.has(Building::Harbor);
        let colossus = buildings.has(Building::Colossus) && !Building::Colossus.obsoleted_by().is_some_and(|t| knows(owner, t));
        // GM Farms make every farmed tile one food richer.
        let engineered = buildings.has(Building::GmFarms);
        let raw_yield = |owner: u32, hex: Hex| {
            yield_for(owner, hex).map(|y| {
                let tile = tile_at.get(&hex);
                let water = tile.is_some_and(|t| t.terrain == Terrain::Water);
                let farmed = engineered && tile.is_some_and(|t| t.improvements.contains(&Improvement::Farm));
                (y.0 + u32::from(harbor && water) + u32::from(farmed), y.1, y.2 + u32::from(colossus && water))
            })
        };
        let (gov, golden_age) = land_rule(owner);
        // A city that celebrates its ruler escapes the despot's penalty, and under Monarchy or
        // Communism earns a republic's trade.
        let festive = happiness.is_some_and(|h| bevy_civ::economy::celebrating(city.population, h.happy, h.unhappy));
        let ruled = |y| bevy_civ::economy::tile_rule(y, gov, golden_age, festive);
        let enemy_on = |h: &Hex| occupants.iter().any(|(p, o)| p == h && *o != owner && (*o == BARBARIANS || owner == BARBARIANS || diplomacy.get_treaty(owner, *o) == Treaty::War));
        let centre = ruled(bevy_civ::tiles::centre_yield(raw_yield(owner, city.position).unwrap_or((2, 0, 0))));
        let mut candidates: Vec<(Hex, bevy_civ::tiles::Yield)> = areas
            .get(&city.position)
            .map(|area| area.iter().filter(|h| **h != city.position && !enemy_on(h)).filter_map(|h| raw_yield(owner, *h).map(|y| (*h, ruled(y)))).collect())
            .unwrap_or_default();
        candidates.sort_by_key(|(h, _)| (h.x, h.y));
        let keep = if owner == PLAYER { city.pinned.clone() } else { Vec::new() };
        let worked = bevy_civ::tiles::assign(&candidates, &keep, centre, city.workers(), city.population, focus);
        let land = bevy_civ::tiles::total(&candidates, &worked, centre);
        if city.worked != worked {
            city.worked = worked;
        }
        if city.land != land {
            city.land = land;
        }
    }
}

/// The player moves a citizen: off a worked tile (to become an entertainer), or onto a free tile of
/// the city's area (taking a specialist back to work, or else the least useful worked tile's
/// citizen). Returns a note for the log when something worth saying happened.
fn toggle_worked(city: &mut City, hex: Hex, value: impl Fn(Hex) -> u32) -> Option<String> {
    use bevy_civ::city::Specialist;
    if let Some(i) = city.worked.iter().position(|h| *h == hex) {
        city.worked.remove(i);
        city.pinned.retain(|h| *h != hex);
        city.add_specialist(Specialist::Entertainer);
        return Some(format!("A citizen of {} leaves that tile to entertain (+2 luxury).", city.name));
    }
    if let Some(i) = city.specialists.iter().position(|s| *s == Specialist::Entertainer).or((!city.specialists.is_empty()).then_some(0)) {
        city.specialists.remove(i);
    } else if let Some(i) = city.worked.iter().enumerate().filter(|(_, h)| !city.pinned.contains(h)).min_by_key(|(_, h)| value(**h)).map(|(i, _)| i) {
        city.worked.remove(i);
    } else if let Some(i) = city.worked.iter().enumerate().min_by_key(|(_, h)| value(**h)).map(|(i, _)| i) {
        let freed = city.worked.remove(i);
        city.pinned.retain(|h| *h != freed);
    } else {
        return None;
    }
    city.worked.push(hex);
    city.pinned.push(hex);
    None
}

/// Nuclear strikes: unless an SDI Defense within reach shoots the missile down, everything within a
/// tile of the target is destroyed, a city there loses half its people and two buildings, fallout
/// spoils the land around, the victim goes to war with the attacker and the world turns against it.
#[allow(clippy::too_many_arguments)]
pub fn nuclear_strikes(
    mut strikes: EventReader<Detonation>,
    units: Query<(Entity, &Unit)>,
    mut cities: Query<(&mut City, &mut CityBuildings)>,
    mut tiles: Query<&mut HexTile>,
    (mut diplomacy, civs, supplies): (ResMut<Diplomacy>, Res<Civilizations>, Res<Supplies>),
    (mut log, mut dirty, mut bursts): (ResMut<GameLog>, ResMut<Dirty>, EventWriter<Burst>),
    mut commands: Commands,
) {
    let name_of = |id: u32| if id == PLAYER { "We".to_string() } else { civs.players.iter().find(|c| c.id == id).map(|c| capitalised(&c.name)).unwrap_or_else(|| "Someone".into()) };
    let mut rng = rand::thread_rng();
    for strike in strikes.read() {
        dirty.map = true;
        let target_city = cities.iter().find(|(c, _)| c.position == strike.at).map(|(c, _)| (c.owner, c.name.clone()));
        let victim = target_city.as_ref().map(|(o, _)| *o).or_else(|| supplies.territory.get(&strike.at).map(|(o, _)| *o));
        let place = target_city.as_ref().map(|(_, n)| n.clone()).unwrap_or_else(|| "open country".into());
        // An SDI Defense of anyone but the attacker shoots the missile down.
        let shield = cities.iter().find(|(c, b)| c.owner != strike.by && b.has(Building::SdiDefense) && c.position.distance_to(strike.at) <= SDI_RANGE).map(|(c, _)| c.name.clone());
        if let Some(defender) = shield {
            log.push(format!("A nuclear missile aimed at {place} is shot down by the SDI Defense of {defender}."));
            continue;
        }
        for hex in hexx::shapes::hexagon(strike.at, 1) {
            bursts.send(Burst(hex));
            for (entity, unit) in units.iter().filter(|(_, u)| u.position == hex) {
                let _ = unit;
                commands.entity(entity).try_despawn_recursive();
            }
            if let Some((mut city, mut buildings)) = cities.iter_mut().find(|(c, _)| c.position == hex) {
                city.population = (city.population / 2).max(1);
                city.food_storage = 0;
                for _ in 0..2 {
                    let ordinary: Vec<usize> = buildings.buildings.iter().enumerate().filter(|(_, b)| !b.is_wonder()).map(|(i, _)| i).collect();
                    if ordinary.is_empty() {
                        break;
                    }
                    let index = ordinary[rng.gen_range(0..ordinary.len())];
                    buildings.buildings.remove(index);
                }
            } else if let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == hex && t.terrain != Terrain::Water) {
                if rng.gen_bool(0.7) {
                    tile.add_improvement(Improvement::Pollution);
                }
            }
        }
        log.push(format!("{} launched a nuclear missile at {place}! Everything within a tile is destroyed, and fallout spoils the land.", name_of(strike.by)));
        // The victim fights back; every other people recoils.
        if let Some(victim) = victim.filter(|v| *v != strike.by && *v != BARBARIANS) {
            diplomacy.set_treaty(strike.by, victim, Treaty::War);
        }
        for civ in civs.players.iter().filter(|c| c.id != strike.by && !c.eliminated) {
            diplomacy.change_attitude(civ.id, strike.by, -25);
        }
    }
}

fn unit_name(adds: u32) -> &'static str {
    if adds >= 2 { "settlers" } else { "workers" }
}

/// What a city does after finishing a building: the next project in its queue, or wait for orders.
fn next_hint(city: &City) -> String {
    match city.queue.first() {
        Some(next) => format!("Next from its queue: {}.", next.name()),
        None => "Choose its next project.".to_string(),
    }
}

/// Rebuilds the resource network whenever the map, borders or deals may have changed.
fn refresh_supplies(
    dirty: Res<Dirty>,
    foreign: Res<Foreign>,
    tech: Res<TechState>,
    civs: Res<Civilizations>,
    tiles: Query<&HexTile>,
    cities: Query<(&City, &CityBuildings)>,
    culture: Res<CultureLedger>,
    mut supplies: ResMut<Supplies>,
) {
    if !(dirty.map || foreign.is_changed() || tech.is_changed() || culture.is_changed()) {
        return;
    }
    let nodes: Vec<supply::CityNode> = cities
        .iter()
        .map(|(c, b)| supply::CityNode {
            position: c.position,
            owner: c.owner,
            capital: c.is_capital,
            harbor: b.buildings.contains(&Building::Harbor),
            radius: bevy_civ::culture::border_radius(culture.of(c.position)),
            culture: culture.of(c.position),
        })
        .collect();
    let mut techs: HashMap<u32, HashSet<Tech>> = civs.players.iter().map(|c| (c.id, c.tech.researched.clone())).collect();
    techs.insert(PLAYER, tech.researched.clone());
    supply::recompute(&mut supplies, tiles.iter(), &nodes, &techs, &foreign.transfers());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ships_and_aircraft_keep_their_own_speed_at_sea() {
        let mut map = plain_map();
        let sea = Hex::new(3, 0);
        map.water.insert(sea);
        let land = Hex::new(0, 0);
        assert_eq!(moves_per_turn(UnitType::Destroyer, sea, &map, 0), 6);
        assert_eq!(moves_per_turn(UnitType::Galleon, sea, &map, 1), 6, "Lighthouse bonus at sea");
        assert_eq!(moves_per_turn(UnitType::Galley, land, &map, 1), 4, "a ship in port is afloat too");
        assert_eq!(moves_per_turn(UnitType::Fighter, sea, &map, 1), 8, "aircraft ignore the sea");
        assert_eq!(moves_per_turn(UnitType::Rifleman, sea, &map, 1), 1, "troops aboard keep their own moves");
        assert_eq!(moves_per_turn(UnitType::Cavalry, land, &map, 1), 3);
    }

    #[test]
    fn rates_stay_within_the_government_limit() {
        use bevy_civ::economy::Rates;
        let mut state = GameState::default();
        assert_eq!(state.rates(), Rates { science: 60, luxury: 0 });
        assert_eq!(state.rates().split(10, state.government), (6, 0, 4));
        state.set_rates(Rates { science: 50, luxury: 10 });
        assert_eq!(state.tax_rate(), 40);
        // Despotism allows at most 60% in any share, taxes included.
        state.set_rates(Rates { science: 90, luxury: 0 });
        assert_eq!(state.rates(), Rates { science: 60, luxury: 0 });
        state.set_rates(Rates { science: 20, luxury: 0 });
        assert_eq!(state.tax_rate(), 60);
        state.government = bevy_civ::government::Government::Democracy;
        state.set_rates(Rates { science: 90, luxury: 0 });
        assert_eq!(state.rates().science, 90);
        state.government = bevy_civ::government::Government::Anarchy;
        assert_eq!(state.rates().split(10, state.government), (0, 10, 0));
    }

    fn plain_map() -> MapData {
        MapData {
            layout: HexLayout::flat(),
            cells: HashSet::new(),
            water: HashSet::new(),
            terrain: HashMap::new(),
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        }
    }

    fn legion(moves: u32) -> Unit {
        Unit { unit_type: UnitType::Legion, movement_left: moves, owner: PLAYER, position: Hex::ZERO, build_turns_left: None, current_build: None, fortified: false, road_credit: 0, hp: 3, level: 0, troops: Vec::new(), fatigue: 0, carrier: None }
    }

    /// A minimal app that runs `apply_actions`, with the player's capital at the origin and a
    /// friendly foreign city three tiles east.
    fn order_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_event::<Action>()
            .add_event::<EndTurnRequest>()
            .add_event::<Sfx>()
            .add_event::<Focus>()
            .add_event::<BattleShown>()
            .add_event::<savegame::SaveRequest>()
            .insert_resource(plain_map())
            .insert_resource(Dirty { map: false, ui: false })
            .init_resource::<Selection>()
            .init_resource::<GameState>()
            .init_resource::<TechState>()
            .init_resource::<Diplomacy>()
            .init_resource::<Foreign>()
            .init_resource::<TradeRoutes>()
            .init_resource::<FogOfWar>()
            .init_resource::<Civilizations>()
            .init_resource::<GameLog>()
            .insert_resource(Cycle { advance: true, force: false })
            .init_resource::<Supplies>()
            .init_resource::<WorldWonders>()
            .init_resource::<GreatPeople>()
            .init_resource::<Espionage>()
            .init_resource::<Religions>()
            .init_resource::<Villages>()
            .init_resource::<SpaceRace>()
            .init_resource::<LastMove>()
            .init_resource::<bevy_civ::ai::Ferries>()
            .init_resource::<JustCaptured>()
            .init_resource::<Rivers>()
            .add_systems(Update, apply_actions);
        let world = app.world_mut();
        let mut commands = world.commands();
        spawn_city(&mut commands, Hex::ZERO, "Roma".into(), 3, PLAYER, true);
        spawn_city(&mut commands, Hex::new(3, 0), "Memphis".into(), 3, 1, true);
        world.flush();
        world.resource_mut::<Civilizations>().players.push(bevy_civ::ai::Civ {
            id: 1,
            name: "Egypt".into(),
            is_human: false,
            gold: 400,
            tech: TechState::knowing(&["Pottery".to_string(), "Mining".to_string()]),
            aggression: 0.5,
            leader: "Ptolemy".into(),
            adjective: "Egyptian".into(),
            portrait: 0,
            city_names: vec![],
            government: bevy_civ::government::Government::Despotism,
            anarchy: 0,
            pending_government: None,
            eliminated: false,
            rates: Default::default(),
        });
        app
    }

    /// Selects a fresh unit of `kind` at `at` (trained in `home`, if given) and gives it an order.
    fn order(app: &mut App, kind: UnitType, at: Hex, home: Option<Hex>, action: Action) -> Entity {
        let world = app.world_mut();
        let unit = spawn_unit(&mut world.commands(), at, kind, PLAYER);
        world.flush();
        if let Some(home) = home {
            world.entity_mut(unit).insert(HomeCity(home));
        }
        world.resource_mut::<Selection>().unit = Some(unit);
        world.send_event(action);
        app.update();
        unit
    }

    #[test]
    fn projects_can_be_bought_but_wonders_cannot() {
        let mut app = order_app();
        let mut cities = app.world_mut().query::<(Entity, &mut City)>();
        let roma = cities.iter_mut(app.world_mut()).find(|(_, c)| c.name == "Roma").map(|(e, _)| e).unwrap();
        let set = |app: &mut App, item: ProductionItem| {
            let mut city = app.world_mut().get_mut::<City>(roma).unwrap();
            city.production_queue = Some(item);
            city.production_progress = 10;
        };
        app.world_mut().resource_mut::<Selection>().city = Some(roma);
        app.world_mut().resource_mut::<GameState>().gold = 1000;

        set(&mut app, ProductionItem::Building(Building::Temple));
        let cost = app.world().get::<City>(roma).unwrap().hurry_cost().unwrap();
        app.world_mut().send_event(Action::Hurry);
        app.update();
        assert_eq!(app.world().resource::<GameState>().gold, 1000 - cost);
        assert_eq!(app.world().get::<City>(roma).unwrap().production_progress, Building::Temple.cost());

        set(&mut app, ProductionItem::Building(Building::Pyramids));
        app.world_mut().send_event(Action::Hurry);
        app.update();
        assert_eq!(app.world().get::<City>(roma).unwrap().production_progress, 10, "wonders cannot be bought");

        // Despotism (the default) can rush with citizens.
        set(&mut app, ProductionItem::Building(Building::Barracks));
        app.world_mut().send_event(Action::Rush);
        app.update();
        let city = app.world().get::<City>(roma).unwrap();
        assert_eq!((city.population, city.production_progress), (2, Building::Barracks.cost()));
    }

    #[test]
    fn a_building_can_be_sold_once_a_turn() {
        let mut app = order_app();
        let mut cities = app.world_mut().query::<(Entity, &City)>();
        let roma = cities.iter(app.world()).find(|(_, c)| c.name == "Roma").map(|(e, _)| e).unwrap();
        app.world_mut().get_mut::<CityBuildings>(roma).unwrap().buildings = vec![Building::Temple, Building::Library];
        app.world_mut().resource_mut::<Selection>().city = Some(roma);
        app.world_mut().resource_mut::<GameState>().gold = 0;
        app.world_mut().send_event(Action::SellBuilding(Building::Library));
        app.update();
        assert_eq!(app.world().resource::<GameState>().gold, Building::Library.cost() as i32);
        assert_eq!(app.world().get::<CityBuildings>(roma).unwrap().buildings, vec![Building::Temple]);
        // Only one a turn.
        app.world_mut().send_event(Action::SellBuilding(Building::Temple));
        app.update();
        assert_eq!(app.world().get::<CityBuildings>(roma).unwrap().buildings, vec![Building::Temple]);
    }

    #[test]
    fn founding_too_close_names_the_city_in_the_way() {
        let mut app = order_app();
        for at in [Hex::new(1, 0), Hex::new(-2, 0)] {
            app.world_mut().spawn(HexTile { coord: at, terrain: Terrain::Grassland, improvements: vec![], resource: None });
        }
        let settler = order(&mut app, UnitType::Settler, Hex::new(1, 0), None, Action::FoundCity);
        assert!(app.world().get_entity(settler).is_ok(), "next to Roma: not founded");
        let log = app.world().resource::<GameLog>();
        assert!(log.lines.last().is_some_and(|l| l.contains("Roma") && l.contains("empty tile")), "{:?}", log.lines.last());
        let far = order(&mut app, UnitType::Settler, Hex::new(-2, 0), None, Action::FoundCity);
        assert!(app.world().get_entity(far).is_err(), "two tiles away: founded");
    }

    #[test]
    fn ending_the_turn_warns_once_of_an_open_city_with_an_enemy_near() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_event::<EndTurnRequest>()
            .add_event::<EndTurnNow>()
            .add_event::<Focus>()
            .init_resource::<GameState>()
            .init_resource::<FogOfWar>()
            .init_resource::<Diplomacy>()
            .init_resource::<GameLog>()
            .add_systems(Update, end_turn_guard);
        let world = app.world_mut();
        spawn_city(&mut world.commands(), Hex::ZERO, "Roma".into(), 3, PLAYER, true);
        let raider = spawn_unit(&mut world.commands(), Hex::new(2, 0), UnitType::Warrior, BARBARIANS);
        world.flush();
        world.resource_mut::<FogOfWar>().visible.insert(Hex::new(2, 0));
        let ended = |app: &mut App| app.world_mut().resource_mut::<Events<EndTurnNow>>().drain().count();

        app.world_mut().send_event(EndTurnRequest);
        app.update();
        assert_eq!(ended(&mut app), 0, "warned first");
        assert!(app.world().resource::<GameLog>().lines.last().is_some_and(|l| l.contains("Roma has no soldier")));
        app.world_mut().send_event(EndTurnRequest);
        app.update();
        assert_eq!(ended(&mut app), 1, "a second request goes ahead");

        // A soldier inside, or an enemy out of sight, is no reason to wait.
        app.world_mut().resource_mut::<GameState>().turn += 1;
        spawn_unit(&mut app.world_mut().commands(), Hex::ZERO, UnitType::Spearman, PLAYER);
        app.world_mut().send_event(EndTurnRequest);
        app.update();
        assert_eq!(ended(&mut app), 1);
        let _ = raider;
    }

    #[test]
    fn a_scout_explores_towards_the_edge_of_the_known_and_stops_when_done() {
        // A strip of land along the x axis, known only around the scout's start.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameState>()
            .init_resource::<FogOfWar>()
            .init_resource::<Diplomacy>()
            .init_resource::<GameLog>()
            .insert_resource(Dirty { map: false, ui: false })
            .add_systems(Update, auto_explore);
        let mut map = plain_map();
        for q in -10..=10 {
            let hex = Hex::new(q, 0);
            map.cells.insert(hex);
            map.terrain.insert(hex, Terrain::Grassland);
            app.world_mut().spawn(HexTile { coord: hex, terrain: Terrain::Grassland, improvements: vec![], resource: None });
        }
        app.insert_resource(map);
        app.world_mut().resource_mut::<FogOfWar>().reveal(Hex::ZERO, 2);
        let scout = spawn_unit(&mut app.world_mut().commands(), Hex::ZERO, UnitType::Scout, PLAYER);
        app.world_mut().flush();
        app.world_mut().entity_mut(scout).insert(Exploring);
        app.update();
        let at = app.world().get::<Unit>(scout).unwrap().position;
        assert!(at.distance_to(Hex::ZERO) >= 2, "the scout set out: {at:?}");
        assert!(!app.world().get::<Unit>(scout).unwrap().can_move(), "and is done for the turn");
        // Turn after turn it reaches the far ends, then gives up.
        for turn in 1..40 {
            app.world_mut().resource_mut::<GameState>().turn = turn;
            app.world_mut().get_mut::<Unit>(scout).unwrap().movement_left = 3;
            app.update();
            if app.world().get::<Exploring>(scout).is_none() {
                break;
            }
        }
        assert!(app.world().get::<Exploring>(scout).is_none(), "nothing left to find");
        let fog = app.world().resource::<FogOfWar>();
        assert!(fog.explored.contains(&Hex::new(10, 0)) && fog.explored.contains(&Hex::new(-10, 0)));
    }

    #[test]
    fn caravans_open_trade_routes_and_help_wonders() {
        let mut app = order_app();
        // Next to a foreign city at peace: a route from home.
        let caravan = order(&mut app, UnitType::Caravan, Hex::new(2, 0), Some(Hex::ZERO), Action::TradeRoute);
        assert!(app.world().get_entity(caravan).is_err(), "spent");
        let trade = app.world().resource::<TradeRoutes>();
        assert_eq!(trade.routes.len(), 1);
        assert_eq!((trade.routes[0].from, trade.routes[0].to, trade.routes[0].partner), (Hex::ZERO, Hex::new(3, 0), 1));
        assert!(trade.income(PLAYER).0 > 0 && trade.income(1).0 > 0);
        // In its own city with nowhere to trade with: kept.
        let idle = order(&mut app, UnitType::Caravan, Hex::ZERO, Some(Hex::ZERO), Action::TradeRoute);
        assert!(app.world().get_entity(idle).is_ok());

        // Helping a wonder adds the caravan's shields.
        let mut cities = app.world_mut().query::<&mut City>();
        for mut city in cities.iter_mut(app.world_mut()) {
            if city.name == "Roma" {
                city.production_queue = Some(ProductionItem::Building(Building::Pyramids));
                city.production_progress = 10;
            }
        }
        order(&mut app, UnitType::Caravan, Hex::ZERO, None, Action::UseUnit);
        let mut cities = app.world_mut().query::<&City>();
        let roma = cities.iter(app.world()).find(|c| c.name == "Roma").unwrap();
        assert_eq!(roma.production_progress, 10 + UnitType::Caravan.cost());
    }

    #[test]
    fn diplomats_open_embassies_and_spy() {
        let mut app = order_app();
        let far = order(&mut app, UnitType::Diplomat, Hex::new(-3, 0), None, Action::Spy(Mission::Embassy));
        assert!(app.world().get_entity(far).is_ok(), "must be next to a foreign city");

        order(&mut app, UnitType::Diplomat, Hex::new(2, 0), None, Action::Spy(Mission::Embassy));
        assert!(app.world().resource::<Espionage>().has_embassy(PLAYER, 1));

        // Covert missions succeed or get caught; either way the diplomat is spent and the outcome shows.
        for _ in 0..8 {
            let gold = app.world().resource::<GameState>().gold;
            let attitude = app.world().resource::<Diplomacy>().get_attitude(PLAYER, 1);
            let spy = order(&mut app, UnitType::Diplomat, Hex::new(2, 0), None, Action::Spy(Mission::StealGold));
            assert!(app.world().get_entity(spy).is_err());
            let stole = app.world().resource::<GameState>().gold > gold;
            let caught = app.world().resource::<Diplomacy>().get_attitude(PLAYER, 1) < attitude;
            assert!(stole != caught, "exactly one outcome");
        }
        // Stealing technology teaches us something Egypt knows.
        app.world_mut().resource_mut::<TechState>().researched.clear();
        for _ in 0..12 {
            order(&mut app, UnitType::Diplomat, Hex::new(2, 0), None, Action::Spy(Mission::StealTech));
        }
        assert!(!app.world().resource::<TechState>().researched.is_empty());
    }

    #[test]
    fn a_staged_incident_gives_a_just_cause_or_costs_reputation() {
        let mut app = order_app();
        order(&mut app, UnitType::Diplomat, Hex::new(2, 0), None, Action::Spy(Mission::Embassy));
        // Each attempt either succeeds (a just cause against them) or is exposed (reputation lost).
        for _ in 0..10 {
            let reputation = app.world().resource::<Foreign>().reputation;
            app.world_mut().resource_mut::<Foreign>().grievances.clear();
            order(&mut app, UnitType::Diplomat, Hex::new(2, 0), None, Action::Spy(Mission::StageIncident));
            let turn = app.world().resource::<GameState>().turn;
            let foreign = app.world().resource::<Foreign>();
            let cause = foreign.grievance(PLAYER, 1, turn).is_some_and(|g| g.wrong == foreign::Wrong::BorderIncident);
            let exposed = foreign.reputation == reputation - espionage::EXPOSED_REPUTATION && foreign.grievance(1, PLAYER, turn).is_some();
            assert!(cause != exposed, "exactly one outcome");
        }
    }

    #[test]
    fn history_replays_who_held_what_turn_by_turn() {
        let mut history = History::default();
        // Turn 1: a tile and a city are ours; turn 5: a rival takes both; turn 9: the city is razed.
        history.borders.extend([(1, 0, 0, PLAYER), (5, 0, 0, 2)]);
        history.cities.extend([(1, 0, 0, PLAYER), (5, 0, 0, 2), (9, 0, 0, NOBODY)]);
        history.scores.extend([(1, PLAYER, 3), (9, PLAYER, 10)]);
        let (borders, cities) = history.at(3);
        assert_eq!((borders.get(&Hex::ZERO), cities.get(&Hex::ZERO)), (Some(&PLAYER), Some(&PLAYER)));
        let (borders, cities) = history.at(7);
        assert_eq!((borders.get(&Hex::ZERO), cities.get(&Hex::ZERO)), (Some(&2), Some(&2)));
        assert!(history.at(9).1.is_empty(), "the razed city is gone");
        assert_eq!(history.span(), (1, 9));
    }

    #[test]
    fn surrender_ends_the_game_in_defeat() {
        let mut app = order_app();
        app.world_mut().send_event(Action::Surrender);
        app.update();
        let outcome = app.world().resource::<GameState>().victory.clone().unwrap_or_default();
        assert!(outcome.starts_with("Defeat"), "{outcome}");
    }

    /// Selects a fresh great person of `kind` at `at` and orders it to act.
    fn use_great(app: &mut App, kind: UnitType, at: Hex) -> Entity {
        let world = app.world_mut();
        let person = spawn_unit(&mut world.commands(), at, kind, PLAYER);
        world.flush();
        world.resource_mut::<Selection>().unit = Some(person);
        world.send_event(Action::UseUnit);
        app.update();
        person
    }

    #[test]
    fn great_people_do_their_work_where_they_stand() {
        let mut app = order_app();

        // Scientist: completes the current research.
        app.world_mut().resource_mut::<TechState>().current = Some(Tech::Pottery);
        let scientist = use_great(&mut app, UnitType::GreatScientist, Hex::new(1, 0));
        assert!(app.world().resource::<TechState>().researched.contains(&Tech::Pottery));
        assert!(app.world().get_entity(scientist).is_err(), "used up");

        // Engineer: only in one of the player's cities, where it finishes the project.
        let away = use_great(&mut app, UnitType::GreatEngineer, Hex::new(1, 0));
        assert!(app.world().get_entity(away).is_ok(), "kept when not in a city");
        use_great(&mut app, UnitType::GreatEngineer, Hex::ZERO);
        let mut cities = app.world_mut().query::<&City>();
        let roma = cities.iter(app.world()).find(|c| c.name == "Roma").unwrap();
        assert!(roma.production_progress >= roma.production_queue.as_ref().unwrap().cost());

        // Artist: a masterpiece in the player's city.
        use_great(&mut app, UnitType::GreatArtist, Hex::ZERO);
        assert_eq!(app.world().resource::<GreatPeople>().masterpiece_content(Hex::ZERO), 3);

        // Merchant: gold from a trade mission to a foreign city at peace.
        let before = app.world().resource::<GameState>().gold;
        use_great(&mut app, UnitType::GreatMerchant, Hex::new(3, 0));
        assert_eq!(app.world().resource::<GameState>().gold - before, great::trade_mission_gold(3));
    }

    #[test]
    fn units_march_three_times_as_far_on_roads_and_freely_on_rails() {
        let route: Vec<Hex> = (0..=9).map(|q| Hex::new(q, 0)).collect();
        let map = plain_map();
        let mut fog = FogOfWar::default();
        let walked = |roads: &HashMap<Hex, Road>, fog: &mut FogOfWar| {
            let mut unit = legion(1);
            let mut trail = Vec::new();
            walk(&mut unit, &route, &map, roads, &HashSet::new(), fog, &mut trail);
            trail.len()
        };
        assert_eq!(walked(&HashMap::new(), &mut fog), 1);
        let roads: HashMap<Hex, Road> = route.iter().map(|h| (*h, Road::Road)).collect();
        assert_eq!(walked(&roads, &mut fog), 3);
        let rails: HashMap<Hex, Road> = route.iter().map(|h| (*h, Road::Rail)).collect();
        assert_eq!(walked(&rails, &mut fog), 9);
    }

    fn people(id: u32, name: &str, techs: &[&str]) -> bevy_civ::ai::Civ {
        bevy_civ::ai::Civ {
            id,
            name: name.into(),
            is_human: id == PLAYER,
            gold: 100,
            tech: TechState::knowing(&techs.iter().map(|t| t.to_string()).collect::<Vec<_>>()),
            aggression: 0.5,
            leader: "Leader".into(),
            adjective: name.into(),
            portrait: 0,
            city_names: vec![],
            government: bevy_civ::government::Government::Despotism,
            anarchy: 0,
            pending_government: None,
            eliminated: false,
            rates: Default::default(),
        }
    }

    #[test]
    fn a_golden_age_comes_once_from_a_unique_unit_and_enriches_productive_tiles() {
        let civs = Civilizations { players: vec![people(PLAYER, "Rome", &[]), people(1, "Egypt", &[])] };
        let (mut golden, mut log) = (GoldenAges::default(), GameLog::default());
        let met: HashSet<u32> = [1].into();
        // Beating barbarians or winning with another unit does nothing.
        unique_victory(&mut golden, &civs, &met, &mut log, PLAYER, UnitType::Praetorian, BARBARIANS);
        unique_victory(&mut golden, &civs, &met, &mut log, PLAYER, UnitType::Spearman, 1);
        assert!(!golden.active(PLAYER));
        // Rome's Legion beats Egypt: twenty turns of plenty, once a game.
        unique_victory(&mut golden, &civs, &met, &mut log, PLAYER, UnitType::Praetorian, 1);
        assert!(golden.active(PLAYER) && !golden.active(1));
        assert!(!golden.begin(PLAYER), "only one golden age a game");
        for _ in 0..GOLDEN_AGE_TURNS - 1 {
            golden.tick();
        }
        assert!(golden.active(PLAYER));
        golden.tick();
        assert!(!golden.active(PLAYER));
        // A shield more where the land gives shields, a gold more where it gives gold.
        assert_eq!(GoldenAges::tile((2, 0, 1)), (2, 0, 2));
        assert_eq!(GoldenAges::tile((1, 2, 0)), (1, 3, 0));
    }

    #[test]
    fn the_great_library_teaches_what_two_known_peoples_know_and_scientists_mark_new_ages() {
        let ancient: Vec<&str> = Tech::ALL.into_iter().filter(|t| t.era() == bevy_civ::tech::Era::Ancient && !t.optional()).map(|t| t.base_name()).collect();
        let mut civs = Civilizations {
            players: vec![
                people(PLAYER, "Rome", &[]),
                people(1, "Egypt", &["Pottery", "Mining", "Alphabet"]),
                people(2, "Persia", &["Pottery", "Mining"]),
                people(3, "Greece", &["Pottery", "Alphabet"]),
            ],
        };
        let mut tech = TechState::knowing(&["Bronze Working".to_string()]);
        let mut wonders = WorldWonders::default();
        wonders.complete(Building::GreatLibrary, PLAYER, "Roma");
        let holding: HashSet<u32> = [PLAYER, 1, 2, 3].into();
        let mut log = GameLog::default();
        // We have met only Egypt and Persia: Pottery and Mining are known to both; the Alphabet to Egypt alone.
        great_library(&mut civs, &mut tech, &wonders, &[1, 2].into(), &holding, &mut log);
        assert!(tech.researched.contains(&Tech::Pottery) && tech.researched.contains(&Tech::Mining));
        assert!(!tech.researched.contains(&Tech::Alphabet));
        // Once we know Theory of Gravity it teaches no more.
        tech.researched.insert(Building::GreatLibrary.obsoleted_by().unwrap());
        great_library(&mut civs, &mut tech, &wonders, &[1, 2, 3].into(), &holding, &mut log);
        assert!(!tech.researched.contains(&Tech::Alphabet));

        // Greece is Scientific: entering the Middle Ages brings a free discovery; Egypt's does not.
        let mut civs = Civilizations { players: vec![people(PLAYER, "Rome", &[]), people(1, "Egypt", &ancient), people(2, "Greece", &ancient)] };
        let mut tech = TechState::knowing(&[]);
        new_ages(&mut civs, &mut tech, &HashSet::new(), &holding, &mut log);
        for civ in civs.players.iter_mut() {
            civ.tech.learn(Tech::Engineering);
            civ.tech.current = Some(Tech::Astronomy);
        }
        new_ages(&mut civs, &mut tech, &HashSet::new(), &holding, &mut log);
        assert!(civs.players[2].tech.researched.contains(&Tech::Astronomy), "a scientific people's free discovery");
        assert!(!civs.players[1].tech.researched.contains(&Tech::Astronomy));
    }
}
