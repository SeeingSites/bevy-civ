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
use bevy_civ::great::{self, GreatPeople};
use bevy_civ::culture::{self as culture_rules, CultureLedger};
use bevy_civ::espionage::{self, Espionage, Mission};
use rand::Rng;
use bevy_civ::diplomacy::{Diplomacy, Happiness, Treaty};
use bevy_civ::fog::FogOfWar;
use bevy_civ::government::Government;
use bevy_civ::pathfinding::{find_path, find_route, land_step, road_network, turns_along, Road, StepCost, Travel};
use bevy_civ::unit::{Domain, AIR_RANGE};
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

/// A computer player's score, counted the same way as the player's (population, cities' buildings).
fn rival_score(owner: u32, cities: &Query<(&mut City, &mut CityBuildings, &mut Happiness)>) -> u32 {
    cities
        .iter()
        .filter(|(c, _, _)| c.owner == owner)
        .map(|(c, b, _)| c.population * 2 + b.buildings.iter().map(|w| if w.is_wonder() { 50 } else { 0 }).sum::<u32>())
        .sum()
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
    /// How the player's commerce is split, in percent: science and luxury; taxes take the rest.
    #[serde(default = "default_science_rate")]
    pub science_rate: u32,
    #[serde(default)]
    pub luxury_rate: u32,
}

fn default_science_rate() -> u32 {
    60
}

impl GameState {
    pub fn tax_rate(&self) -> u32 {
        100u32.saturating_sub(self.science_rate + self.luxury_rate)
    }

    /// Splits a city's commerce into (science, luxury, gold) by the rates.
    pub fn split_commerce(&self, commerce: u32) -> (u32, u32, u32) {
        // A government that allows less than the chosen rate caps it (the rest goes to taxes).
        let cap = self.government.science_cap();
        let science = commerce * self.science_rate.min(cap) / 100;
        let luxury = commerce * self.luxury_rate.min(cap) / 100;
        (science, luxury, commerce - science - luxury)
    }
}

fn default_last_turn() -> u32 {
    LAST_TURN
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
            last_turn: LAST_TURN,
            science_rate: 60,
            luxury_rate: 0,
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
}

impl GameLog {
    pub fn push(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
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
    /// Burn the selected city, taken this turn, to the ground.
    Raze,
    /// Set the share of commerce going to science and to luxury (percent; taxes take the rest).
    SetRates(u32, u32),
    /// Buy the rest of the selected city's project with gold (not wonders).
    Hurry,
    /// Rush the selected city's project with citizens' labour (Despotism and Communism).
    Rush,
    /// K: the selected worker looks after the land by itself (again to take it back).
    Automate,
    /// Make one of the selected city's workers a specialist of this kind (true), or send one back (false).
    Specialist(bevy_civ::city::Specialist, bool),
    /// The selected artillery, ship or bomber fires on the strongest enemy next to it.
    Bombard,
    /// Tab: the next of the player's units on the selected unit's or city's tile.
    CycleStack,
    SetProduction(ProductionItem),
    Research(Tech),
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

/// Cities the player took this turn: they can still be razed.
#[derive(Resource, Default)]
pub struct JustCaptured(pub HashSet<Hex>);

/// Ends the player's turn (sent once the order to end it is confirmed).
#[derive(Event)]
pub struct EndTurnNow;

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

/// Golden ages: a civilization's first wonder starts one (once per game): +50% shields and +1 gold
/// in every city for [`GOLDEN_AGE_TURNS`] turns.
#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoldenAges {
    #[serde(with = "bevy_civ::save::pairs")]
    pub turns_left: HashMap<u32, u32>,
    pub had: HashSet<u32>,
}

pub const GOLDEN_AGE_TURNS: u32 = 10;

impl GoldenAges {
    pub fn active(&self, owner: u32) -> bool {
        self.turns_left.get(&owner).is_some_and(|t| *t > 0)
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

/// Every civilization's score, turn by turn, for the score advisor.
#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct History {
    /// (turn, civilization, score)
    pub scores: Vec<(u32, u32, u32)>,
}

/// The city a unit was trained in (a caravan's trade routes start there).
#[derive(Component, Clone, Copy)]
pub struct HomeCity(pub Hex);

/// Plays a combat effect on a tile.
#[derive(Event)]
pub struct Burst(pub Hex);

fn main() {
    // `--autoplay [scenario] [turns]`: a whole game between computer players, without a window.
    let args: Vec<String> = std::env::args().collect();
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
                        title: "Bevy Civ".into(),
                        // Web build: render into the page's canvas and track its size.
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
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
        .add_systems(
            Update,
            (handle_map_click, apply_actions, process_turn, continue_orders, automate_workers, advance_selection, preview_route, refresh_supplies)
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
        .init_resource::<Cycle>()
        .init_resource::<WorldWonders>()
        .init_resource::<GreatPeople>()
        .init_resource::<Espionage>()
        .init_resource::<History>()
        .init_resource::<CultureLedger>()
        .init_resource::<GoldenAges>()
        .init_resource::<JustCaptured>()
        .init_resource::<Rivers>()
        .add_event::<EndTurnNow>()
        .add_event::<Focus>()
        .add_event::<Action>()
        .add_event::<MapClick>()
        .add_event::<Burst>()
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
        scenario::generate_random(&new_game.scenario, new_game.civ)
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
        })
        .collect();
    *tech = TechState::knowing(&defs[0].techs);

    for (id, def) in defs.iter().enumerate() {
        let owner = id as u32;
        for city in &def.cities {
            let hex = scenario::offset_to_hex(city.col, city.row);
            spawn_city(&mut commands, hex, city.name.clone(), city.pop.max(1), owner, city.capital);
            if owner == PLAYER {
                fog.reveal(hex, 3);
            }
        }
        for unit in &def.units {
            let Some(kind) = UnitType::ALL.into_iter().find(|u| u.name() == unit.kind) else { continue };
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
    commands.insert_resource(map);
    dirty.map = true;
    dirty.ui = true;
    log.push(format!("{}: you lead {} as {}.", scenario.name, defs[0].name, defs[0].leader));
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
    commands.insert_resource(Espionage::default());
    commands.insert_resource(History::default());
    commands.insert_resource(CultureLedger::default());
    commands.insert_resource(GoldenAges::default());
    commands.insert_resource(JustCaptured::default());
    commands.insert_resource(Rivers::default());
    commands.insert_resource(Cycle { advance: true, force: false });
    commands.insert_resource(Dirty { map: true, ui: true });
}

fn handle_map_click(
    mut clicks: EventReader<MapClick>,
    mut selection: ResMut<Selection>,
    mut units: Query<(Entity, &mut Unit)>,
    mut cities: Query<(Entity, &mut City)>,
    (tiles, walls_of, rivers): (Query<&HexTile>, Query<&CityBuildings>, Res<Rivers>),
    state: Res<GameState>,
    map: Res<MapData>,
    tech: Res<TechState>,
    mut fog: ResMut<FogOfWar>,
    mut diplomacy: ResMut<Diplomacy>,
    mut log: ResMut<GameLog>,
    mut dirty: ResMut<Dirty>,
    (mut cycle, mut wonders, mut great, mut taken): (ResMut<Cycle>, ResMut<WorldWonders>, ResMut<GreatPeople>, ResMut<JustCaptured>),
    (mut bursts, mut sounds): (EventWriter<Burst>, EventWriter<Sfx>),
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

        let snapshot: Vec<(Entity, Hex, u32, UnitType)> =
            units.iter().map(|(e, u)| (e, u.position, u.owner, u.unit_type)).collect();
        // Hit points, level and whether dug in, for whoever has to defend.
        let vitals: HashMap<Entity, (u32, u8, bool)> = units.iter().map(|(e, u)| (e, (u.hp, u.level, u.fortified))).collect();
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
            let index = current.and_then(|c| picks.iter().position(|p| *p == c)).map(|i| i + 1).unwrap_or(0);
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
        if unit.build_turns_left.is_some() {
            log.push(format!("{} is busy building.", unit.unit_type.name()));
            continue;
        }
        if !unit.can_move() {
            log.push(format!("{} has no moves left this turn.", unit.unit_type.name()));
            continue;
        }
        let sail = tech.researched.contains(&Tech::Seafaring);
        let domain = unit.unit_type.domain();
        // The strongest defender on the tile meets an attack.
        let enemy = snapshot.iter().filter(|u| u.1 == hex && u.2 != unit.owner).max_by_key(|u| u.3.stats().1 * 10 + vitals.get(&u.0).map_or(0, |v| v.0)).copied();
        let ports: HashSet<Hex> = cities.iter().filter(|(_, c)| c.owner == PLAYER).map(|(_, c)| c.position).collect();
        // Where each kind of unit may go.
        let refusal = match domain {
            Domain::Sea if enemy.is_none() && !map.water.contains(&hex) && !ports.contains(&hex) => Some("Ships can only enter water and your own coastal cities."),
            Domain::Air if !ports.iter().any(|p| p.distance_to(hex) <= AIR_RANGE) => Some("That is beyond the range of our airfields."),
            Domain::Land if enemy.is_some() && map.water.contains(&unit.position) => Some("Troops cannot attack from their boats; land them first."),
            Domain::Land if enemy.is_some() && map.water.contains(&hex) => Some("Land units cannot attack ships at sea."),
            _ => None,
        };
        if let Some(reason) = refusal {
            log.push(reason);
            continue;
        }
        let travel = player_travel(domain, sail, &ports);
        let roads = road_network(tiles.iter(), cities.iter().map(|(_, c)| c.position));
        let Some(route) = find_route(unit.position, hex, tiles.iter().map(|t| (t.coord, t.terrain)), &roads, travel) else {
            log.push(if sail || domain != Domain::Land { "There is no route to that tile." } else { "There is no route to that tile. Crossing water needs Seafaring." });
            continue;
        };
        dirty.map = true;
        unit.fortified = false;
        cycle.advance = true;

        // Stop next to a defended tile; the last step is the attack itself.
        let approach = if enemy.is_some() { &route[..route.len() - 1] } else { &route[..] };
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
            commands.entity(selected).insert((GoTo(hex), Trail(trail)));
        } else {
            commands.entity(selected).insert(Trail(trail)).remove::<GoTo>();
        }

        if let Some((enemy_entity, _, enemy_owner, enemy_type)) = enemy {
            let adjacent = unit.position.distance_to(hex) == 1;
            if adjacent && unit.can_move() && unit.unit_type.is_military() {
                unit.spend(StepCost::Full);
                bursts.send(Burst(hex));
                if enemy_owner != BARBARIANS && diplomacy.get_treaty(PLAYER, enemy_owner) != Treaty::War {
                    diplomacy.set_treaty(PLAYER, enemy_owner, Treaty::War);
                    log.push("Your attack means war!");
                }
                let terrain = defence_terrain(Some(terrain), city_here.is_some());
                let enemy_type = defending_as(enemy_type, hex, &map);
                let led = |at: Hex, owner: u32| snapshot.iter().any(|u| u.1 == at && u.2 == owner && u.3 == UnitType::GreatGeneral);
                let (d_hp, d_level, dug_in) = vitals.get(&enemy_entity).copied().unwrap_or((3, 0, false));
                let walls = city_here.is_some_and(|(e, owner)| walls_of.get(e).is_ok_and(|b| b.has(Building::Walls)) || wonders.walls_everywhere(owner));
                let outcome = combat::fight(
                    Side { kind: unit.unit_type, hp: unit.hp, level: unit.level, led: led(unit.position, PLAYER) },
                    Side { kind: enemy_type, hp: d_hp, level: d_level, led: led(hex, enemy_owner) },
                    Ground { terrain, fortified: dug_in, walls, river: rivers.tiles.contains(&hex) },
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
                let promoted = rand::thread_rng().gen_bool(1.0 / 3.0);
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
                    let rank = if promoted && unit.promote() { format!(" It is promoted to {}.", unit.level_name()) } else { String::new() };
                    log.push(format!("Victory! Your {} defeated the enemy {} ({} of {} hit points left).{rank}", unit.unit_type.name(), enemy_type.name(), unit.hp, unit.max_hp()));
                    if great.victory(PLAYER) {
                        spawn_unit(&mut commands, unit.position, UnitType::GreatGeneral, PLAYER);
                        log.push("Your victories bring forth a Great General! Keep it with your troops: they fight half again as hard.");
                    }
                    commands.entity(enemy_entity).try_despawn_recursive();
                    let more_defenders = snapshot.iter().any(|u| u.1 == hex && u.2 != PLAYER && u.0 != enemy_entity);
                    if !more_defenders && may_enter(unit.unit_type, hex, &map, &ports) {
                        unit.position = hex;
                    }
                } else {
                    log.push(format!("Defeat. Your {} was lost attacking a {}.", unit.unit_type.name(), enemy_type.name()));
                    commands.entity(selected).try_despawn_recursive();
                    selection.unit = None;
                    continue;
                }
            } else if !unit.unit_type.is_military() {
                log.push(format!("A {} cannot attack.", unit.unit_type.name()));
            }
        }

        fog.reveal(unit.position, 2);
        capture(unit, cities.iter_mut().map(|(_, c)| c), &mut wonders, &mut taken, &mut log);

        // The rest of the stack follows when ordered together.
        if click.group {
            let from = snapshot.iter().find(|u| u.0 == selected).map(|u| u.1);
            let followers: Vec<Entity> = snapshot.iter().filter(|u| Some(u.1) == from && u.0 != selected && u.2 == PLAYER && u.3.domain() == domain).map(|u| u.0).collect();
            let enemies: HashSet<Hex> = snapshot
                .iter()
                .filter(|u| u.2 != PLAYER && u.3.is_army() && (u.2 == BARBARIANS || diplomacy.get_treaty(PLAYER, u.2) == Treaty::War))
                .map(|u| u.1)
                .collect();
            for follower in followers {
                let Ok((_, mut unit)) = units.get_mut(follower) else { continue };
                if unit.build_turns_left.is_some() || !unit.can_move() {
                    continue;
                }
                let travel = player_travel(unit.unit_type.domain(), sail, &ports);
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
    if !unit.unit_type.is_army() {
        return;
    }
    for mut city in cities {
        if city.position == unit.position && city.owner != PLAYER {
            city.owner = PLAYER;
            city.is_capital = false;
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

/// How the player's units of a kind travel, given whether Seafaring is known and where the ports are.
fn player_travel(domain: Domain, sail: bool, ports: &HashSet<Hex>) -> Travel<'_> {
    match domain {
        Domain::Sea => Travel::Sea(ports),
        Domain::Air => Travel::Air,
        Domain::Land if sail => Travel::Amphibious,
        Domain::Land => Travel::Land,
    }
}

/// Boarding or leaving a boat takes the rest of the turn (ships and aircraft are unaffected).
fn ends_move(domain: Domain, from: Hex, to: Hex, map: &MapData) -> bool {
    domain == Domain::Land && map.water.contains(&from) != map.water.contains(&to)
}

/// What a step costs: rough ground slows troops on foot, roads and railroads speed them; boarding a
/// boat ends the turn.
fn move_cost(domain: Domain, from: Hex, to: Hex, map: &MapData, roads: &HashMap<Hex, Road>) -> StepCost {
    if ends_move(domain, from, to, map) {
        StepCost::EndsTurn
    } else {
        land_step(from, to, map.terrain.get(&to).copied(), roads, domain == Domain::Land)
    }
}

/// The moves a unit gets when a turn begins on `at`: boats make steady progress whatever they carry.
/// `sea_bonus` is what the owner's wonders add at sea (the Great Lighthouse).
fn moves_per_turn(unit_type: UnitType, at: Hex, map: &MapData, sea_bonus: u32) -> u32 {
    let at_sea = map.water.contains(&at) || unit_type.domain() == Domain::Sea;
    let base = if map.water.contains(&at) { 3 } else { unit_type.stats().2 };
    base + if at_sea { sea_bonus } else { 0 }
}

/// Moves a unit along `route` (which starts where it stands) while it has moves left, revealing the
/// land around each tile entered and adding it to `trail`.
fn walk(unit: &mut Unit, route: &[Hex], map: &MapData, roads: &HashMap<Hex, Road>, enemies: &HashSet<Hex>, fog: &mut FogOfWar, trail: &mut Vec<Hex>) {
    for next in route.iter().skip(1) {
        if !unit.can_move() || (unit.unit_type.is_army() && bevy_civ::pathfinding::zoc_blocks(unit.position, *next, enemies)) {
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
    (map, tech, diplomacy): (Res<MapData>, Res<TechState>, Res<Diplomacy>),
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
    let sail = tech.researched.contains(&Tech::Seafaring);
    let mut arrived = Vec::new();
    for (entity, mut unit, order) in &mut units {
        let Some(&GoTo(target)) = order else { continue };
        if unit.owner != PLAYER || unit.build_turns_left.is_some() || unit.fortified || !unit.can_move() {
            continue;
        }
        let name = unit.unit_type.name();
        let travel = player_travel(unit.unit_type.domain(), sail, &ports);
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
        commands.entity(entity).insert(Trail(trail));
        dirty.map = true;
    }
    if !arrived.is_empty() {
        dirty.ui = true;
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
            _ => {}
        }
        (steam && tile.has_road() && !tile.has_railroad()).then_some((4, Improvement::Railroad))
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
            commands.entity(entity).insert(Trail(trail));
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
    (map, tech, fog, wonders): (Res<MapData>, Res<TechState>, Res<FogOfWar>, Res<WorldWonders>),
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
    let travel = player_travel(unit.unit_type.domain(), tech.researched.contains(&Tech::Seafaring), &ports);
    let roads = road_network(tiles.iter(), cities.iter().map(|c| c.position));
    let Some(route) = find_route(unit.position, target, tiles.iter().map(|t| (t.coord, t.terrain)), &roads, travel) else {
        path.set_if_neq(PathPreview::default());
        return;
    };
    let domain = unit.unit_type.domain();
    let turns = turns_along(&route, unit.budget(), |at| moves_per_turn(unit.unit_type, at, &map, wonders.sea_moves(PLAYER)), |a, b| move_cost(domain, a, b, &map, &roads));
    path.set_if_neq(PathPreview { start: Some(unit.position), steps: route[1..].iter().copied().zip(turns).collect() });
}

/// Troops caught in their transport boats cannot fight back.
fn defending_as(unit: UnitType, at: Hex, map: &MapData) -> UnitType {
    if unit.domain() == Domain::Land && map.water.contains(&at) { UnitType::Worker } else { unit }
}

/// Whether a victorious attacker can move onto the tile it cleared.
fn may_enter(unit: UnitType, tile: Hex, map: &MapData, own_cities: &HashSet<Hex>) -> bool {
    match unit.domain() {
        Domain::Land => true,
        Domain::Sea => map.water.contains(&tile) || own_cities.contains(&tile),
        Domain::Air => false,
    }
}

/// Terrain used for a defender's bonus: city streets and walls defend at least as well as hills.
fn defence_terrain(terrain: Option<Terrain>, in_city: bool) -> Terrain {
    // `in_city` also covers fortified units.
    match terrain.unwrap_or(Terrain::Plains) {
        Terrain::Mountain => Terrain::Mountain,
        _ if in_city => Terrain::Hills,
        other => other,
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
    (mut cycle, supplies, mut map, mut end_turn, mut focus): (ResMut<Cycle>, Res<Supplies>, ResMut<MapData>, EventWriter<EndTurnNow>, EventWriter<Focus>),
    (unit_entities, mut sounds, mut wonders, mut great, mut spies, homes, walled, mut taken, automated, rivers, mut saves): (
        Query<Entity, With<Unit>>,
        EventWriter<Sfx>,
        ResMut<WorldWonders>,
        ResMut<GreatPeople>,
        ResMut<Espionage>,
        Query<&HomeCity>,
        Query<(Entity, &CityBuildings)>,
        ResMut<JustCaptured>,
        Query<Entity, With<Automated>>,
        Res<Rivers>,
        EventWriter<savegame::SaveRequest>,
    ),
    mut commands: Commands,
) {
    for action in actions.read() {
        dirty.ui = true;
        dirty.map = true;
        let selected_unit = selection.unit.filter(|e| units.get(*e).is_ok_and(|u| u.owner == PLAYER));
        match action {
            Action::EndTurn => {
                end_turn.send(EndTurnNow);
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
                    end_turn.send(EndTurnNow);
                }
            }
            Action::Fortify => {
                if let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) {
                    unit.fortified = true;
                    cycle.advance = true;
                }
                if let Some(entity) = selected_unit {
                    commands.entity(entity).remove::<GoTo>();
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
                        Some((owner, name)) if *owner != PLAYER && *owner != BARBARIANS && diplomacy.get_treaty(PLAYER, *owner) != Treaty::War => {
                            let capital = cities.iter().find(|c| c.owner == PLAYER && c.is_capital).map(|c| c.position).unwrap_or(here);
                            let gold = great::trade_mission_gold(capital.distance_to(here));
                            state.gold += gold;
                            log.push(format!("Your Great Merchant's trade mission to {name} brings in {gold} gold."));
                            true
                        }
                        _ => {
                            log.push("A Great Merchant trades in a foreign city you are not at war with: move it into one, then press U.");
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
                    _ => {
                        log.push("A Great General needs no orders: troops on its tile fight half again as hard. Keep it with your army.");
                        false
                    }
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
                    log.push(format!("The Statue of Liberty spares us anarchy: we are now a {}.", government.name()));
                    continue;
                }
                state.government = Government::Anarchy;
                state.anarchy_turns = Government::REVOLUTION_TURNS;
                state.pending_government = Some(*government);
                log.push(format!("Revolution! After {} turns of anarchy we will be a {}.", Government::REVOLUTION_TURNS, government.name()));
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
            Action::SetRates(science, luxury) => {
                let cap = state.government.science_cap();
                let (science, luxury) = ((*science).min(cap), (*luxury).min(cap));
                if science + luxury <= 100 {
                    state.science_rate = science;
                    state.luxury_rate = luxury;
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
                let Some(to) = unit.unit_type.upgrades_to() else {
                    log.push(format!("A {} has nothing to upgrade to.", unit.unit_type.name()));
                    continue;
                };
                if !cities.iter().any(|c| c.position == here && c.owner == PLAYER) {
                    log.push("Units are upgraded in one of your cities.");
                    continue;
                }
                if to.required_tech().is_some_and(|t| !tech.researched.contains(&t)) || !supplies.missing(here, to.required_resources()).is_empty() {
                    log.push(format!("This city cannot equip a {} yet.", to.name()));
                    continue;
                }
                let cost = unit.unit_type.upgrade_cost(to);
                if state.gold < cost {
                    log.push(format!("Upgrading to a {} costs {cost} gold; we have {}.", to.name(), state.gold));
                    continue;
                }
                state.gold -= cost;
                if let Ok(mut unit) = units.get_mut(entity) {
                    log.push(format!("Our {} is re-equipped as a {} for {cost} gold.", unit.unit_type.name(), to.name()));
                    unit.unit_type = to;
                    unit.hp = unit.max_hp();
                    unit.end_move();
                }
            }
            Action::Bombard => {
                let Some(entity) = selected_unit else { continue };
                let Ok(unit) = units.get(entity) else { continue };
                if !unit.unit_type.can_bombard() || !unit.can_move() {
                    continue;
                }
                let (kind, level, here) = (unit.unit_type, unit.level, unit.position);
                // The strongest enemy we are at war with (or barbarians) next to us.
                let target = unit_entities
                    .iter()
                    .filter_map(|e| units.get(e).ok().map(|u| (e, u)))
                    .filter(|(_, u)| u.position.distance_to(here) == 1 && u.owner != PLAYER && (u.owner == BARBARIANS || diplomacy.get_treaty(PLAYER, u.owner) == Treaty::War))
                    .max_by_key(|(_, u)| u.unit_type.stats().1 * 10 + u.hp)
                    .map(|(e, u)| (e, u.unit_type, u.hp, u.level, u.fortified, u.position));
                let Some((target, their_kind, their_hp, their_level, dug_in, at)) = target else {
                    log.push("There is no enemy we are at war with next to us to bombard.");
                    continue;
                };
                let terrain = defence_terrain(map.terrain.get(&at).copied(), cities.iter().any(|c| c.position == at));
                let hp = combat::bombard(
                    Side { kind, hp: 3, level, led: false },
                    Side { kind: their_kind, hp: their_hp, level: their_level, led: false },
                    Ground { terrain, fortified: dug_in, walls: false, river: rivers.tiles.contains(&at) },
                    &mut rand::thread_rng(),
                );
                if let Ok(mut victim) = units.get_mut(target) {
                    victim.hp = hp;
                }
                if let Ok(mut unit) = units.get_mut(entity) {
                    unit.end_move();
                }
                log.push(format!("Our {} bombards the enemy {}: {} of {} hit points left.", kind.name(), their_kind.name(), hp, their_hp));
                sounds.send(Sfx::Battle);
                cycle.advance = true;
            }
            Action::CycleStack => {
                let here = selected_unit
                    .and_then(|e| units.get(e).ok().map(|u| u.position))
                    .or_else(|| selection.city.and_then(|e| cities.get(e).ok().map(|c| c.position)));
                let Some(here) = here else { continue };
                let stack = own_stack(here, unit_entities.iter(), |e| units.get(e).ok().map(|u| (u.position, u.owner)));
                let next = selected_unit.and_then(|e| stack.iter().position(|s| *s == e)).map_or(0, |i| (i + 1) % stack.len().max(1));
                if let Some(entity) = stack.get(next) {
                    selection.unit = Some(*entity);
                    selection.city = None;
                    sounds.send(Sfx::Select);
                }
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
                unit.start_build(*improvement);
                cycle.advance = true;
                if let Some(entity) = selected_unit {
                    commands.entity(entity).remove::<GoTo>();
                }
                log.push(format!("Worker begins a {:?} (3 turns).", improvement));
            }
            Action::Transform => {
                let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) else { continue };
                if !unit.unit_type.is_worker() || !unit.can_move() {
                    continue;
                }
                if tiles.iter().any(|t| t.coord == unit.position && t.is_polluted()) {
                    unit.start_build(Improvement::Pollution);
                    cycle.advance = true;
                    log.push("Worker begins cleaning up the pollution (3 turns).");
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
                let crowded = cities.iter().any(|c| c.position.distance_to(here) < 2);
                if !land || crowded {
                    log.push("A city cannot be founded here; move further from other cities.");
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
                            Mission::Embassy => {}
                        }
                    } else {
                        diplomacy.change_attitude(PLAYER, victim, espionage::CAUGHT_ATTITUDE);
                        log.push(format!("Our diplomat is caught in {city_name}! {civ_name} are outraged."));
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
            Action::SetProduction(item) => {
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
                }
                if let ProductionItem::Building(wonder) = item {
                    if let Some((_, place)) = wonder.is_wonder().then(|| wonders.built.get(wonder)).flatten() {
                        log.push(format!("The {} already stands in {}; each wonder can be built only once.", wonder.name(), place));
                        continue;
                    }
                }
                let needs_coast = matches!(item, ProductionItem::Building(Building::Harbor)) || matches!(item, ProductionItem::Unit(u) if u.domain() == Domain::Sea);
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
                    log.push(format!("Our scholars turn to {}.", target.name()));
                }
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
    (mut trade, map, supplies): (ResMut<TradeRoutes>, Res<MapData>, Res<Supplies>),
    (mut space, mut wonders, mut great, mut spies): (ResMut<SpaceRace>, ResMut<WorldWonders>, ResMut<GreatPeople>, ResMut<Espionage>),
    (mut log, mut dirty, mut cycle): (ResMut<GameLog>, ResMut<Dirty>, ResMut<Cycle>),
    mut bursts: EventWriter<Burst>,
    (homes, mut history, mut culture, mut golden, mut taken, rivers, mut saves, observer): (
        Query<&HomeCity>,
        ResMut<History>,
        ResMut<CultureLedger>,
        ResMut<GoldenAges>,
        ResMut<JustCaptured>,
        Res<Rivers>,
        EventWriter<savegame::SaveRequest>,
        Option<Res<Observer>>,
    ),
    mut commands: Commands,
) {
    if actions.read().count() == 0 || state.victory.is_some() {
        return;
    }
    dirty.map = true;
    dirty.ui = true;
    state.turn += 1;
    spies.tick();
    golden.tick();
    // The chance to raze last turn's conquests has passed.
    taken.0.clear();
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

    let wonders_taken: HashSet<Building> = wonders.built.keys().copied().collect();
    // Caravans and diplomats each civilization has, and the cities building wonders.
    let mut envoy_counts: HashMap<(u32, UnitType), usize> = HashMap::new();
    for (_, unit) in units.iter().filter(|(_, u)| matches!(u.unit_type, UnitType::Caravan | UnitType::Diplomat)) {
        *envoy_counts.entry((unit.owner, unit.unit_type)).or_default() += 1;
    }
    // Civilizations with free land for new cities near their own (at least 3 tiles from any city,
    // within 10 of one of theirs).
    let room: HashSet<u32> = {
        let city_spots: Vec<(Hex, u32)> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();
        let free: Vec<Hex> = tiles
            .iter()
            .filter(|t| t.terrain != Terrain::Water && city_spots.iter().all(|(p, _)| p.distance_to(t.coord) >= 3))
            .map(|t| t.coord)
            .collect();
        civs.players
            .iter()
            .filter(|c| free.iter().any(|f| city_spots.iter().any(|(p, o)| *o == c.id && p.distance_to(*f) <= 10)))
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
        .map(|c| (c.id, [Building::SpaceStructure, Building::SpaceComponent, Building::SpaceModule].into_iter().filter(|p| p.required_tech().is_some_and(|t| c.tech.researched.contains(&t)) && space.wants(c.id, *p)).collect()))
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
                    civ.anarchy = Government::REVOLUTION_TURNS;
                    civ.pending_government = Some(preferred);
                }
            }
        }
        governments.insert(civ.id, civ.government);
    }
    let mut total_science = 0;
    let mut total_gold = 0i32;
    let mut player_cities = 0;

    let capitals: HashMap<u32, Hex> =
        cities.iter().filter(|(c, _, _)| c.is_capital).map(|(c, _, _)| (c.owner, c.position)).collect();
    let mut army_sizes: HashMap<u32, usize> = HashMap::new();
    let mut worker_counts: HashMap<u32, usize> = HashMap::new();
    let mut navy_sizes: HashMap<u32, usize> = HashMap::new();
    let mut air_sizes: HashMap<u32, usize> = HashMap::new();
    for (_, unit) in units.iter() {
        if unit.unit_type.is_army() {
            *army_sizes.entry(unit.owner).or_default() += 1;
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
    let mut hurried: HashSet<u32> = HashSet::new();
    let mut city_counts: HashMap<u32, usize> = HashMap::new();
    for (city, _, _) in cities.iter() {
        *city_counts.entry(city.owner).or_default() += 1;
        // Workers and settlers already on order count, so cities do not all train the same thing.
        match city.production_queue {
            Some(ProductionItem::Unit(UnitType::Worker)) => *worker_counts.entry(city.owner).or_default() += 1,
            Some(ProductionItem::Unit(UnitType::Settler)) => *settler_counts.entry(city.owner).or_default() += 1,
            _ => {}
        }
    }

    for (mut city, mut buildings, mut happiness) in &mut cities {
        let mine = city.owner == PLAYER;
        if mine {
            player_cities += 1;
        }
        let dist = capitals.get(&city.owner).map(|cp| city.position.distance_to(*cp) as u32).unwrap_or(0);

        // What a computer player's governor sees of this city.
        let owner = city.owner;
        let gov = governments.get(&owner).copied().unwrap_or(Government::Despotism);
        let view = governor::CityView {
            position: city.position,
            population: city.population,
            built: buildings.buildings.clone(),
            coastal: city.position.all_neighbors().iter().any(|n| map.water.contains(n)),
            linked: supplies.linked.contains(&city.position),
            defended: armies.iter().any(|(p, o)| *p == city.position && *o == owner),
            threatened: armies.iter().any(|(p, o)| enemies(owner, *o) && p.distance_to(city.position) <= 3),
        };
        let mut needs_orders = false;
        let mut just_trained = false;
        if !mine && owner != BARBARIANS && !view.defended {
            if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
                // An empty city drops everything for a defender, and pays to hurry it when in danger.
                let already = matches!(&city.production_queue, Some(ProductionItem::Unit(u)) if u.is_army());
                if !already {
                    let civ_view = governor::CivView { tech: &civ.tech, aggression: civ.aggression, cities: 1, workers: 1, settlers: 1, military: 0, navy: 0, air: 0, road_work: 0, cut_off: 0, at_war: true, wonders_taken: &wonders_taken, caravans: 1, diplomats: 1, wonder_elsewhere: false, space_parts: &[], room_to_expand: false };
                    city.production_queue = Some(ProductionItem::Unit(governor::best_defender(&view, &civ_view, &supplies)));
                }
                let remaining = city.production_queue.as_ref().map(|i| i.cost()).unwrap_or(0).saturating_sub(city.production_progress) as i32;
                if view.threatened && remaining > 0 && civ.gold >= remaining * 2 {
                    civ.gold -= remaining * 2;
                    city.production_progress += remaining as u32;
                }
            }
        }

        // A rich treasury hurries along whatever a city is working on (one purchase per civilization per turn).
        if !mine && owner != BARBARIANS && !hurried.contains(&owner) {
            if let Some(civ) = civs.players.iter_mut().find(|c| c.id == owner) {
                let remaining = city.production_queue.as_ref().map(|i| i.cost()).unwrap_or(0).saturating_sub(city.production_progress) as i32;
                if remaining > 0 && civ.gold >= 150 + remaining * 2 {
                    civ.gold -= remaining * 2;
                    city.production_progress += remaining as u32;
                    hurried.insert(owner);
                }
            }
        }

        // Farms, mines and bonus resources on the city's land add to what it gathers.
        let (extra_food, extra_shields, extra_gold) = supplies.yields.get(&city.position).copied().unwrap_or_default();
        city.food_storage += extra_food;
        city.production_progress += extra_shields;
        // Trade along the rivers in the city's land: a gold for every river tile.
        let river_gold = supplies.territory.iter().filter(|(at, (_, home))| *home == city.position && rivers.tiles.contains(at)).count() as i32;
        if mine {
            total_gold += extra_gold as i32 + river_gold;
        } else if owner != BARBARIANS {
            ai_income.entry(owner).or_default().1 += river_gold;
        }

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
            + if mine { state.difficulty.player_content() } else { 0 };
        // The player's commerce (what the city's science and taxes would be) is split by the rates;
        // every two points of luxury make one citizen happy.
        let commerce = city.science_per_turn(&buildings, gov, dist) + city.gold_per_turn(&buildings) + 1;
        let (rate_science, rate_luxury, rate_gold) = if mine { state.split_commerce(commerce) } else { (0, 0, 0) };
        let joy = supply::luxury_happiness(supplies.luxuries(city.position), has_market) + rate_luxury / 2;
        happiness.update(city.population, calm, joy);
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
        let idle = unrest || disorder;
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
            let base = city.production_per_turn(&buildings, gov, dist) as f32;
            let extra = (base * (factor - 1.0)).round() as i32;
            city.production_progress = (city.production_progress as i32 + extra).max(0) as u32;
        }
        // The Hoover Dam powers every city of its owner: a quarter more shields.
        if !idle && wonders.owns(owner, Building::HooverDam) {
            city.production_progress += city.production_per_turn(&buildings, gov, dist) / 4;
        }
        // A golden age adds half again to every city's shields.
        if !idle && golden.active(owner) {
            city.production_progress += city.production_per_turn(&buildings, gov, dist) / 2;
            if mine {
                total_gold += 1;
            }
        }
        let finished = if idle { None } else { city.process_production(&buildings, gov, dist) };
        if let Some(completed) = finished {
            needs_orders = true;
            match completed {
                ProductionItem::Unit(ut) => {
                    // Keep computer players from flooding the map.
                    let at_cap = !mine && ut.is_army() && army_sizes.get(&city.owner).copied().unwrap_or(0) >= governor::army_cap(city_counts.get(&city.owner).copied().unwrap_or(1));
                    if !at_cap {
                        let trained = spawn_unit(&mut commands, city.position, ut, city.owner);
                        commands.entity(trained).insert(HomeCity(city.position));
                        // Barracks train veterans.
                        if ut.is_military() && (buildings.has(Building::Barracks) || wonders.owns(city.owner, Building::SunTzu)) {
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
                        log.push(format!("{} trains a {}.", city.name, ut.name()));
                    }
                }
                ProductionItem::Building(b) if b.is_spaceship_part() => {
                    match space.add_part(city.owner, b, state.turn) {
                        Some(arrives) => {
                            let who = civs.players.iter().find(|c| c.id == city.owner).map(|c| c.name.clone()).unwrap_or_else(|| "We".into());
                            log.push(if mine {
                                format!("Our spaceship is complete and launches! It reaches Alpha Centauri on turn {arrives} unless our capital falls.")
                            } else {
                                format!("{who} have launched a spaceship! It arrives on turn {arrives} unless their capital falls.")
                            });
                        }
                        None if mine => {
                            let missing = space.ships.get(&PLAYER).map(|s| s.missing()).unwrap_or_default();
                            log.push(format!("{} completes a {}. Still needed: {missing}.", city.name, b.name()));
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
                        if golden.begin(city.owner) && (mine || foreign.met.contains(&city.owner)) {
                            let who = if mine { "Our".to_string() } else { civs.players.iter().find(|c| c.id == city.owner).map(|c| format!("The {} people's", c.adjective)).unwrap_or_default() };
                            log.push(format!("{who} first wonder begins a golden age: half again as many shields, and more gold, for {GOLDEN_AGE_TURNS} turns."));
                        }
                        let builder = civs.players.iter().find(|c| c.id == city.owner).map(|c| c.name.clone()).unwrap_or_else(|| "A people".into());
                        log.push(if mine {
                            format!("{} completes the {}! ({}) Choose its next project.", city.name, b.name(), b.wonder_effect())
                        } else {
                            format!("Wonder of the world: {} completed the {} in {}.", builder, b.name(), city.name)
                        });
                        buildings.buildings.push(b);
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
                        log.push(format!("{} completes the {}. Choose its next project.", city.name, b.name()));
                    }
                    buildings.buildings.push(b);
                }
            }
        }

        // Big industrial cities foul the land around them.
        if buildings.has(Building::Factory) && city.population >= 10 && rng.gen_bool((0.04 + 0.01 * (city.population - 10) as f64).min(0.3)) {
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
        culture.add(city.position, culture_rules::per_turn(&buildings.buildings, city.is_capital) + if wonders.owns(owner, Building::Eiffel) { 2 } else { 0 });

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

        // A computer player's city gets its next project from the governor.
        if needs_orders && !mine && owner != BARBARIANS {
            if let Some(civ) = civs.players.iter().find(|c| c.id == owner) {
                let civ_view = governor::CivView {
                    tech: &civ.tech,
                    aggression: civ.aggression,
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
                    wonder_elsewhere: wonder_sites.iter().any(|(at, o)| *o == owner && *at != city.position),
                    space_parts: &space_wanted.get(&owner).cloned().unwrap_or_default(),
                    room_to_expand: room.contains(&owner),
                };
                // The unit just trained (if any) now holds the city.
                let view = governor::CityView { built: buildings.buildings.clone(), defended: view.defended || just_trained, ..view };
                let next = governor::choose(&view, &civ_view, &supplies, &mut rng);
                // Count what is now on order so the civilization's other cities do not duplicate it.
                match &next {
                    ProductionItem::Unit(UnitType::Worker) => *worker_counts.entry(owner).or_default() += 1,
                    ProductionItem::Unit(UnitType::Settler) => *settler_counts.entry(owner).or_default() += 1,
                    ProductionItem::Unit(u) if u.domain() == Domain::Sea => *navy_sizes.entry(owner).or_default() += 1,
                    ProductionItem::Unit(u) if u.domain() == Domain::Air => *air_sizes.entry(owner).or_default() += 1,
                    _ => {}
                }
                city.production_queue = Some(next);
            }
        }
        // Adam Smith's Trading Company and the Internet reward the owner's markets and libraries.
        let wonder_gold = if wonders.owns(owner, Building::AdamSmith) {
            buildings.buildings.iter().filter(|b| matches!(b, Building::Marketplace | Building::Bank | Building::Harbor)).count() as i32
        } else {
            0
        };
        let wonder_science = if wonders.owns(owner, Building::Internet) && buildings.has(Building::Library) { 3 } else { 0 };
        if !idle {
            if mine {
                total_gold += wonder_gold;
                total_science += wonder_science;
            } else if owner != BARBARIANS {
                let income = ai_income.entry(owner).or_default();
                income.0 += wonder_science;
                income.1 += wonder_gold;
            }
        }
        if !mine && !idle {
            let income = ai_income.entry(owner).or_default();
            income.0 += city.science_per_turn(&buildings, gov, dist);
            income.1 += city.gold_per_turn(&buildings) as i32 + 2 + supplies.yields.get(&city.position).map(|y| y.2 as i32).unwrap_or(0);
        }

        if mine && !idle {
            total_science += rate_science;
            total_gold += rate_gold as i32;
        }
    }

    let difficulty = state.difficulty;
    for civ in civs.players.iter_mut().filter(|c| !c.is_human) {
        let (science, gold) = ai_income.get(&civ.id).copied().unwrap_or_default();
        let (trade_gold, trade_science) = trade.income(civ.id);
        let (science, gold) = (((science + trade_science) as f32 * difficulty.ai_output()) as u32, gold + trade_gold as i32);
        civ.gold += gold;
        // Savings beyond a reserve are invested in research.
        let invested = ((civ.gold - 250) / 10).max(0);
        civ.gold -= invested;
        civ.tech.science_per_turn = (science + invested as u32 / 2).max(1);
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
    if military > free_support {
        total_gold -= (military - free_support) as i32 * gov.unit_upkeep_food() as i32;
    }
    state.gold_per_turn = total_gold;
    state.gold = (state.gold + total_gold).max(0);
    tech.science_per_turn = total_science;

    // Marco Polo's Embassy: an embassy with every civilization (and the player meets them all).
    if let Some(&(owner, _)) = wonders.built.get(&Building::MarcoPolo) {
        for civ in civs.players.iter().filter(|c| c.id != owner) {
            spies.embassies.insert((owner, civ.id));
            if owner == PLAYER && !civ.is_human {
                foreign.met.insert(civ.id);
            }
        }
    }
    // Leonardo's Workshop: one obsolete unit of its owner is upgraded free each turn.
    if let Some(&(owner, _)) = wonders.built.get(&Building::Leonardo) {
        let known = |t: Option<Tech>| {
            t.is_none_or(|t| if owner == PLAYER { tech.researched.contains(&t) } else { civs.players.iter().any(|c| c.id == owner && c.tech.researched.contains(&t)) })
        };
        let candidate = units
            .iter()
            .filter(|(_, u)| u.owner == owner && u.unit_type.upgrades_to().is_some_and(|to| known(to.required_tech())))
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
            let moves = moves_per_turn(unit.unit_type, unit.position, &map, wonders.sea_moves(unit.owner));
            unit.refresh(moves);
        }
    }

    // A city in disorder near a far greater culture may defect to it, its garrison sent home.
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
            if let Some(new_owner) = tempter.filter(|_| rng.gen_bool(culture_rules::DEFECTION_CHANCE)) {
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
    let treaties = diplomacy.treaties.clone();
    let hostile = move |a: u32, b: u32| {
        if a == b {
            return false;
        }
        a == BARBARIANS || b == BARBARIANS || treaties.get(&(a.min(b), a.max(b))).copied() == Some(Treaty::War)
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
    let attacks = process_ai_turn(&mut civs, &mut units, &city_owners, &tiles, &hostile, &mut road_jobs, &owns, &mut commands);
    let names: HashMap<u32, String> = civs.players.iter().map(|c| (c.id, c.adjective.clone())).collect();
    let civ_name = |owner: u32| names.get(&owner).cloned().unwrap_or_else(|| "Barbarian".to_string());

    let walled_cities: HashSet<Hex> = cities.iter().filter(|(c, b, _)| b.has(Building::Walls) || wonders.walls_everywhere(c.owner)).map(|(c, _, _)| c.position).collect();
    let mut dead: HashSet<Entity> = HashSet::new();
    for attack in attacks {
        if dead.contains(&attack.attacker) {
            continue;
        }
        let Ok((_, attacker)) = units.get(attack.attacker) else { continue };
        let (a_type, a_owner, a_pos) = (attacker.unit_type, attacker.owner, attacker.position);
        let defender = units
            .iter()
            .filter(|(e, u)| u.position == attack.target && u.owner != a_owner && !dead.contains(e))
            .max_by_key(|(_, u)| u.unit_type.stats().1 * 10 + u.hp)
            .map(|(e, u)| (e, u.unit_type, u.owner, u.fortified));
        let a_vitals = (attacker.hp, attacker.level);
        let own_cities: HashSet<Hex> = city_owners.iter().filter(|(_, o)| *o == a_owner).map(|(p, _)| *p).collect();
        let can_enter = may_enter(a_type, attack.target, &map, &own_cities);
        let Some((d_entity, d_type, d_owner, dug_in)) = defender else {
            if can_enter {
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
        let in_city = city_owners.iter().any(|(p, _)| *p == attack.target);
        let terrain = defence_terrain(tiles.iter().find(|t| t.coord == attack.target).map(|t| t.terrain), in_city);
        if fog.visible.contains(&attack.target) {
            bursts.send(Burst(attack.target));
        }
        let led = |at: Hex, owner: u32| units.iter().any(|(e, u)| !dead.contains(&e) && u.position == at && u.owner == owner && u.unit_type == UnitType::GreatGeneral);
        let (a_led, d_led) = (led(a_pos, a_owner), led(attack.target, d_owner));
        let d_vitals = units.get(d_entity).map(|(_, u)| (u.hp, u.level)).unwrap_or((3, 0));
        let outcome = combat::fight(
            Side { kind: a_type, hp: a_vitals.0, level: a_vitals.1, led: a_led },
            Side { kind: defending_as(d_type, attack.target, &map), hp: d_vitals.0, level: d_vitals.1, led: d_led },
            Ground { terrain, fortified: dug_in, walls: walled_cities.contains(&attack.target), river: rivers.tiles.contains(&attack.target) },
            &mut rng,
        );
        // Hit points left after the fight, and a chance of promotion for the victor.
        let mut settle = |entity: Entity, hp: u32, won: bool, rng: &mut rand::rngs::ThreadRng| {
            if let Ok((_, mut unit)) = units.get_mut(entity) {
                unit.hp = hp.max(1);
                if won && rng.gen_bool(1.0 / 3.0) {
                    unit.promote();
                }
            }
        };
        let attacker_won = match outcome {
            Outcome::Won { attacker_hp } => {
                settle(attack.attacker, attacker_hp, true, &mut rng);
                true
            }
            Outcome::Retreated { defender_hp } => {
                settle(attack.attacker, 1, false, &mut rng);
                settle(d_entity, defender_hp, false, &mut rng);
                if d_owner == PLAYER {
                    log.push(format!("Our {} drove off a {} {}.", d_type.name(), civ_name(a_owner), a_type.name()));
                }
                continue;
            }
            Outcome::Lost { defender_hp } => {
                settle(d_entity, defender_hp, true, &mut rng);
                false
            }
        };
        if attacker_won {
            if a_owner != BARBARIANS && great.victory(a_owner) {
                spawn_unit(&mut commands, a_pos, UnitType::GreatGeneral, a_owner);
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
            if d_owner != BARBARIANS && great.victory(d_owner) {
                spawn_unit(&mut commands, attack.target, UnitType::GreatGeneral, d_owner);
                if d_owner == PLAYER {
                    log.push("Your stand brings forth a Great General! Keep it with your troops: they fight half again as hard.");
                }
            }
        }
    }

    // An enemy military unit standing in an undefended city takes it.
    for (mut city, _, _) in &mut cities {
        let defended = units.iter().any(|(e, u)| !dead.contains(&e) && u.position == city.position && u.owner == city.owner && u.unit_type.is_army());
        let occupier = units
            .iter()
            .find(|(e, u)| {
                !dead.contains(e) && u.position == city.position && u.owner != PLAYER && u.owner != city.owner && u.unit_type.is_army() && hostile(u.owner, city.owner)
            })
            .map(|(_, u)| u.owner);
        if let Some(new_owner) = occupier.filter(|_| !defended) {
            if city.owner == PLAYER {
                log.push(format!("{} has fallen to {} forces!", city.name, civ_name(new_owner)));
            } else if fog.explored.contains(&city.position) {
                log.push(format!("{} forces capture {}.", civ_name(new_owner), city.name));
            }
            city.owner = new_owner;
            city.is_capital = false;
            wonders.transfer_city(&city.name, new_owner);
            city.production_progress = 0;
            city.production_queue = Some(ProductionItem::Unit(UnitType::Warrior));
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
            .filter(|(e, u)| !dead.contains(e) && u.owner != PLAYER && u.owner != BARBARIANS && matches!(u.unit_type, UnitType::Caravan | UnitType::Diplomat))
            .map(|(e, u)| (e, u.owner, u.unit_type, u.position))
            .collect();
        for (entity, owner, kind, here) in envoys {
            let Some(civ_index) = civs.players.iter().position(|c| c.id == owner) else { continue };
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
                let sail = civs.players[civ_index].tech.researched.contains(&Tech::Seafaring);
                let travel = if sail { Travel::Amphibious } else { Travel::Land };
                if let Some(route) = find_route(here, goal, land.iter().copied(), &roads, travel) {
                    let stop = if foreign_goal { route.len().saturating_sub(1) } else { route.len() };
                    // Never into a tile someone else holds.
                    let blocked = route.iter().skip(1).position(|h| units.iter().any(|(e, u)| !dead.contains(&e) && u.position == *h && u.owner != owner)).map(|i| i + 1);
                    let stop = blocked.map_or(stop, |b| b.min(stop));
                    if let Ok((_, mut unit)) = units.get_mut(entity) {
                        let mut trail = Vec::new();
                        walk(&mut unit, &route[..stop.max(1)], &map, &roads, &HashSet::new(), &mut fog, &mut trail);
                        commands.entity(entity).insert(Trail(trail));
                    }
                }
                continue;
            }
            let Some((_, partner, partner_pop, city_name, walls)) = city_info.iter().find(|c| c.0 == goal).cloned() else { continue };
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
                    let mission = if !spies.has_embassy(owner, partner) {
                        Mission::Embassy
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
                        if partner == PLAYER {
                            log.push(format!("We caught a {} diplomat spying in {}!", civ_name(owner), city_name));
                        }
                        continue;
                    }
                    match mission {
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
    foreign.strength.clear();
    for (entity, unit) in units.iter() {
        if !dead.contains(&entity) {
            *foreign.strength.entry(unit.owner).or_default() += bevy_civ::ai::unit_strength(unit);
        }
    }
    for (_, owner) in &city_owners {
        *foreign.strength.entry(*owner).or_default() += 2.0;
    }
    {
        let mut ctx = foreign::Ctx { diplomacy: &mut diplomacy, foreign: &mut foreign, civs: &mut civs, state: &mut state, tech: &mut tech, log: &mut log };
        foreign::turn(&mut ctx, &fog, &supplies, &unit_owners, &city_owners);
    }

    let all_positions: Vec<Hex> = units.iter().filter(|(e, _)| !dead.contains(e)).map(|(_, u)| u.position).collect();
    process_barbarians(&mut barbarians, &mut commands, &tiles, &all_positions);

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

    // A revolution ends after its turns of anarchy.
    if state.anarchy_turns > 0 {
        state.anarchy_turns -= 1;
        if state.anarchy_turns == 0 {
            if let Some(next) = state.pending_government.take() {
                state.government = next;
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

    // The automatic save, every turn.
    saves.send(savegame::SaveRequest { download: false });
    state.score = Score::calculate(cities.iter().map(|(c, b, _)| (c, b)), &tech);
    history.scores.push((state.turn, PLAYER, state.score.total));
    for civ in civs.players.iter().filter(|c| !c.is_human) {
        history.scores.push((state.turn, civ.id, rival_score(civ.id, &cities)));
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
        let culture_winner = culture_rules::cultural_winner(&culture.totals(cities.iter().map(|(c, _, _)| (c.position, c.owner))));
        state.victory = if held.len() == 1 {
            leader.map(|(o, _)| format!("Conquest victory for {}.", civ_named(o)))
        } else if let Some((o, n)) = leader.filter(|(_, n)| *n >= 5 && n * 3 >= total_cities * 2) {
            Some(format!("Domination victory for {} ({n} of {total_cities} cities).", civ_named(o)))
        } else if let Some(o) = culture_winner {
            Some(format!("Cultural victory for {}.", civ_named(o)))
        } else if let Some(o) = space.arrived(state.turn) {
            Some(format!("Space race victory for {}.", civ_named(o)))
        } else if state.turn >= state.last_turn {
            let best = civs.players.iter().filter(|c| !c.is_human).map(|c| (rival_score(c.id, &cities), c.name.clone())).max_by_key(|(s, _)| *s);
            best.map(|(s, name)| format!("Score victory for {name} ({s}) at the turn limit."))
        } else {
            None
        };
    } else if player_cities == 0 {
        state.victory = Some("Your civilization has fallen.".into());
    } else if !rivals_remain {
        state.victory = Some("Conquest Victory!".into());
    } else if player_cities >= 5 && player_cities * 3 >= total_cities * 2 {
        state.victory = Some("Domination Victory!".into());
    } else if let Some(winner) = culture_rules::cultural_winner(&culture.totals(cities.iter().map(|(c, _, _)| (c.position, c.owner)))) {
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
        let best_rival = civs.players.iter().filter(|c| !c.is_human).map(|c| (rival_score(c.id, &cities), c.name.clone())).max_by_key(|(s, _)| *s);
        state.victory = Some(match best_rival {
            Some((theirs, name)) if theirs > ours => format!("Time is up: {name} lead the world on score ({theirs} to our {ours})."),
            _ => format!("Score Victory! Time is up and we lead the world with {ours} points."),
        });
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
) {
    if !cycle.advance {
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
    fn rates_split_commerce_within_the_government_cap() {
        let mut state = GameState::default();
        assert_eq!(state.split_commerce(10), (6, 0, 4));
        state.science_rate = 50;
        state.luxury_rate = 30;
        assert_eq!(state.split_commerce(10), (5, 3, 2));
        assert_eq!(state.tax_rate(), 20);
        // Despotism allows at most 60% in one: a 90% science rate pays only 60%.
        state.government = bevy_civ::government::Government::Despotism;
        state.science_rate = 90;
        state.luxury_rate = 0;
        assert_eq!(state.split_commerce(10), (6, 0, 4));
        state.government = bevy_civ::government::Government::Anarchy;
        assert_eq!(state.split_commerce(10).0, 0);
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
        Unit { unit_type: UnitType::Legion, movement_left: moves, owner: PLAYER, position: Hex::ZERO, build_turns_left: None, current_build: None, fortified: false, road_credit: 0, hp: 3, level: 0 }
    }

    /// A minimal app that runs `apply_actions`, with the player's capital at the origin and a
    /// friendly foreign city three tiles east.
    fn order_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_event::<Action>()
            .add_event::<EndTurnNow>()
            .add_event::<Sfx>()
            .add_event::<Focus>()
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
            .init_resource::<Cycle>()
            .init_resource::<Supplies>()
            .init_resource::<WorldWonders>()
            .init_resource::<GreatPeople>()
            .init_resource::<Espionage>()
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
}
