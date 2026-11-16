//! Bevy Civ — game setup, turn processing and player orders.
//! Everything visual (sprites, camera, interface) lives in `client`.

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use hexx::{Hex, HexLayout};
use rand::Rng;
use std::collections::HashMap;

use bevy_civ::ai::{process_ai_turn, Civilizations};
use std::collections::HashSet;
use bevy_civ::barbarians::{process_barbarians, BarbarianState};
use bevy_civ::building::{Building, CityBuildings};
use bevy_civ::city::{spawn_city, City, ProductionItem};
use bevy_civ::combat::resolve_combat;
use bevy_civ::diplomacy::{Diplomacy, Happiness, Treaty};
use bevy_civ::fog::FogOfWar;
use bevy_civ::government::Government;
use bevy_civ::pathfinding::find_path;
use bevy_civ::save::{save_game, SaveData};
use bevy_civ::score::Score;
use bevy_civ::space::{SpaceComponent, SpaceRace};
use bevy_civ::tech::{Tech, TechState};
use bevy_civ::terrain::{random_resource, HexTile, Improvement, Terrain};
use bevy_civ::trade::TradeRoutes;
use bevy_civ::unit::{spawn_unit, Unit, UnitType};

mod client;
mod foreign;
mod scenario;
mod supply;

use foreign::{DiploAct, Foreign, TradeAsk};
use supply::Supplies;
use scenario::NewGame;

pub const PLAYER: u32 = 0;
pub const BARBARIANS: u32 = 99;
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
    /// World-space bounding box of the tile centres.
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Resource)]
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

/// The route of the last move order, drawn as dots on the map.
#[derive(Resource, Default)]
pub struct PathPreview(pub Vec<Hex>);

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
    TradeRoute,
    Save,
    SetProduction(ProductionItem),
    Research(Tech),
}

/// A click (or tap) on a map tile. `secondary` is the right mouse button: always an order.
#[derive(Event)]
pub struct MapClick {
    pub hex: Hex,
    pub secondary: bool,
}

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
    unit.owner == PLAYER && unit.movement_left > 0 && unit.build_turns_left.is_none() && !unit.fortified
}

/// Plays a combat effect on a tile.
#[derive(Event)]
pub struct Burst(pub Hex);

fn main() {
    App::new()
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
        .add_event::<EndTurnNow>()
        .add_event::<Focus>()
        .add_event::<Action>()
        .add_event::<MapClick>()
        .add_event::<Burst>()
        .add_plugins(client::ClientPlugin)
        .add_systems(OnEnter(AppState::Playing), setup_world)
        .add_systems(OnExit(AppState::Playing), cleanup_world)
        .add_systems(
            Update,
            (handle_map_click, apply_actions, process_turn, advance_selection, refresh_supplies)
                .chain()
                .in_set(GameLogic)
                .run_if(in_state(AppState::Playing)),
        )
        .run();
}

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
    let mut map = MapData { layout, cells: HashSet::new(), water: HashSet::new(), min: Vec2::splat(f32::MAX), max: Vec2::splat(f32::MIN) };
    for (row, line) in scenario.rows.iter().enumerate() {
        for (col, code) in line.chars().enumerate() {
            let hex = scenario::offset_to_hex(col as i32, row as i32);
            let terrain = terrain_from(code);
            let pos = map.layout.hex_to_world_pos(hex);
            map.min = map.min.min(pos);
            map.max = map.max.max(pos);
            map.cells.insert(hex);
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
        ..default()
    };

    if let Ok((mut transform, mut projection)) = camera.get_single_mut() {
        let home = defs[0].cities.first().map(|c| map.layout.hex_to_world_pos(scenario::offset_to_hex(c.col, c.row))).unwrap_or(Vec2::ZERO);
        transform.translation = home.extend(transform.translation.z);
        projection.scale = 1.0;
    }
    commands.insert_resource(map);
    dirty.map = true;
    dirty.ui = true;
    log.push(format!("{}: you lead {} as {}.", scenario.name, defs[0].name, defs[0].leader));
}

/// Removes the finished game so a new one can start from the menu.
fn cleanup_world(
    mut commands: Commands,
    entities: Query<Entity, Or<(With<HexTile>, With<Unit>, With<City>, With<client::map::Decor>, With<client::map::PathDot>, With<client::actors::BurstFx>)>>,
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
    commands.insert_resource(Cycle { advance: true, force: false });
    commands.insert_resource(Dirty { map: true, ui: true });
}

fn handle_map_click(
    mut clicks: EventReader<MapClick>,
    mut selection: ResMut<Selection>,
    mut units: Query<(Entity, &mut Unit)>,
    mut cities: Query<(Entity, &mut City)>,
    tiles: Query<&HexTile>,
    state: Res<GameState>,
    map: Res<MapData>,
    tech: Res<TechState>,
    mut fog: ResMut<FogOfWar>,
    mut diplomacy: ResMut<Diplomacy>,
    mut log: ResMut<GameLog>,
    mut dirty: ResMut<Dirty>,
    (mut path, mut cycle): (ResMut<PathPreview>, ResMut<Cycle>),
    mut bursts: EventWriter<Burst>,
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
                path.0.clear();
            }
            continue;
        };

        let snapshot: Vec<(Entity, Hex, u32, UnitType)> =
            units.iter().map(|(e, u)| (e, u.position, u.owner, u.unit_type)).collect();
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
            path.0.clear();
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
                }
                Some(Pick::Unit(entity)) => {
                    selection.unit = Some(*entity);
                    selection.city = None;
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
        if unit.movement_left == 0 {
            log.push(format!("{} has no moves left this turn.", unit.unit_type.name()));
            continue;
        }
        let sail = tech.researched.contains(&Tech::Seafaring);
        let Some(route) = find_path(unit.position, hex, tiles.iter().map(|t| (t.coord, t.terrain)), sail) else {
            log.push(if sail { "There is no route to that tile." } else { "There is no route to that tile. Crossing water needs Seafaring." });
            continue;
        };
        dirty.map = true;
        unit.fortified = false;
        cycle.advance = true;

        let enemy = snapshot.iter().find(|u| u.1 == hex && u.2 != unit.owner).copied();
        // Stop next to a defended tile; the last step is the attack itself.
        let walk = if enemy.is_some() { &route[..route.len() - 1] } else { &route[..] };
        for next in walk.iter().skip(1) {
            if unit.movement_left == 0 {
                break;
            }
            // Boarding or leaving a boat takes the rest of the turn.
            let boarding = map.water.contains(&unit.position) != map.water.contains(next);
            unit.position = *next;
            unit.movement_left = if boarding { 0 } else { unit.movement_left - 1 };
        }
        path.0 = route.clone();

        if let Some((enemy_entity, _, enemy_owner, enemy_type)) = enemy {
            let adjacent = unit.position.distance_to(hex) == 1;
            if adjacent && unit.movement_left > 0 && unit.unit_type.is_military() {
                unit.movement_left -= 1;
                bursts.send(Burst(hex));
                if enemy_owner != BARBARIANS && diplomacy.get_treaty(PLAYER, enemy_owner) != Treaty::War {
                    diplomacy.set_treaty(PLAYER, enemy_owner, Treaty::War);
                    log.push("Your attack means war!");
                }
                let terrain = defence_terrain(Some(terrain), city_here.is_some());
                if resolve_combat(unit.unit_type, enemy_type, terrain, None, None) {
                    log.push(format!("Victory! Your {} defeated the enemy {}.", unit.unit_type.name(), enemy_type.name()));
                    commands.entity(enemy_entity).try_despawn_recursive();
                    let more_defenders = snapshot.iter().any(|u| u.1 == hex && u.2 != PLAYER && u.0 != enemy_entity);
                    if !more_defenders {
                        unit.position = hex;
                    }
                } else {
                    log.push(format!("Defeat. Your {} was lost attacking a {}.", unit.unit_type.name(), enemy_type.name()));
                    commands.entity(selected).try_despawn_recursive();
                    selection.unit = None;
                    path.0.clear();
                    continue;
                }
            } else if !unit.unit_type.is_military() {
                log.push(format!("A {} cannot attack.", unit.unit_type.name()));
            }
        }

        fog.reveal(unit.position, 2);

        if unit.unit_type.is_military() {
            for (_, mut city) in &mut cities {
                if city.position == unit.position && city.owner != PLAYER {
                    city.owner = PLAYER;
                    city.is_capital = false;
                    log.push(format!("Your {} captured {}!", unit.unit_type.name(), city.name));
                }
            }
        }
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
    (mut path, mut cycle, supplies, mut end_turn): (ResMut<PathPreview>, ResMut<Cycle>, Res<Supplies>, EventWriter<EndTurnNow>),
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
                        unit.movement_left = 0;
                    }
                    cycle.advance = true;
                } else if units.iter().any(needs_orders) {
                    selection.city = None;
                    cycle.advance = true;
                    cycle.force = true;
                } else {
                    end_turn.send(EndTurnNow);
                }
            }
            Action::Fortify => {
                if let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) {
                    unit.fortified = true;
                    cycle.advance = true;
                }
            }
            Action::Trade(civ, ask) => {
                let mut ctx = foreign::Ctx { diplomacy: &mut diplomacy, foreign: &mut foreign, civs: &mut civs, state: &mut state, tech: &mut tech, log: &mut log };
                foreign::player_trade(&mut ctx, &supplies, *civ, *ask);
            }
            Action::Deselect => {
                selection.unit = None;
                selection.city = None;
                path.0.clear();
            }
            Action::Build(improvement) => {
                let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) else { continue };
                if !unit.unit_type.is_worker() || unit.movement_left == 0 {
                    continue;
                }
                let Some(tile) = tiles.iter().find(|t| t.coord == unit.position) else { continue };
                if tile.terrain == Terrain::Water || tile.improvements.contains(improvement) {
                    log.push("That cannot be built here.");
                    continue;
                }
                unit.start_build(*improvement);
                cycle.advance = true;
                log.push(format!("Worker begins a {:?} (3 turns).", improvement));
            }
            Action::Transform => {
                let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) else { continue };
                if !unit.unit_type.is_worker() || unit.movement_left == 0 {
                    continue;
                }
                if let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == unit.position) {
                    if tile.transform_terrain() {
                        unit.movement_left = 0;
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
                    unit.movement_left = 0;
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
                let own: Vec<Hex> = cities.iter().filter(|c| c.owner == PLAYER).map(|c| c.position).collect();
                if own.len() >= 2 {
                    let before = trade.routes.len();
                    for pair in own.windows(2) {
                        trade.add_route(pair[0], pair[1]);
                    }
                    log.push(if trade.routes.len() > before { "New trade routes link your cities." } else { "Your cities already trade with each other." });
                } else {
                    log.push("Trade needs at least two cities.");
                }
            }
            Action::Save => {
                let saved = save_game(&SaveData {
                    turn: state.turn,
                    gold: state.gold,
                    science: tech.science_per_turn as i32,
                    government: state.government.name().to_string(),
                    score: state.score.total,
                    player_cities: cities.iter().filter(|c| c.owner == PLAYER).count() as u32,
                    researched_techs: tech.researched.len() as u32,
                    mode: "single".into(),
                });
                log.push(if saved { "Game saved." } else { "The game could not be saved." });
            }
            Action::SetProduction(item) => {
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                let missing = supplies.missing(city.position, item.required_resources());
                if city.owner != PLAYER {
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
    (trade, map, supplies): (Res<TradeRoutes>, Res<MapData>, Res<Supplies>),
    mut space: ResMut<SpaceRace>,
    (mut log, mut dirty, mut path, mut cycle): (ResMut<GameLog>, ResMut<Dirty>, ResMut<PathPreview>, ResMut<Cycle>),
    mut bursts: EventWriter<Burst>,
    mut commands: Commands,
) {
    if actions.read().count() == 0 || state.victory.is_some() {
        return;
    }
    dirty.map = true;
    dirty.ui = true;
    path.0.clear();
    state.turn += 1;

    let mut rng = rand::thread_rng();
    let gov = state.government;
    let mut total_science = 0;
    let mut total_gold = 0i32;
    let mut player_cities = 0;

    let capitals: HashMap<u32, Hex> =
        cities.iter().filter(|(c, _, _)| c.is_capital).map(|(c, _, _)| (c.owner, c.position)).collect();
    let mut army_sizes: HashMap<u32, usize> = HashMap::new();
    let mut worker_counts: HashMap<u32, usize> = HashMap::new();
    for (_, unit) in units.iter() {
        if unit.unit_type.is_military() {
            *army_sizes.entry(unit.owner).or_default() += 1;
        } else if unit.unit_type.is_worker() {
            *worker_counts.entry(unit.owner).or_default() += 1;
        }
    }
    let mut city_counts: HashMap<u32, usize> = HashMap::new();
    for (city, _, _) in cities.iter() {
        *city_counts.entry(city.owner).or_default() += 1;
    }

    for (mut city, mut buildings, mut happiness) in &mut cities {
        let mine = city.owner == PLAYER;
        if mine {
            player_cities += 1;
        }
        let dist = capitals.get(&city.owner).map(|cp| city.position.distance_to(*cp) as u32).unwrap_or(0);

        // Farms, mines and bonus resources on the city's land add to what it gathers.
        let (extra_food, extra_shields, extra_gold) = supplies.yields.get(&city.position).copied().unwrap_or_default();
        city.food_storage += extra_food;
        city.production_progress += extra_shields;
        if mine {
            total_gold += extra_gold as i32;
        }

        if city.process_food(&buildings, gov, dist) && mine {
            log.push(format!("{} grows to size {}.", city.name, city.population));
        }

        // Every different luxury the city has access to keeps people content.
        let has_market = buildings.buildings.contains(&Building::Marketplace);
        let luxury_bonus = city.luxury_from_specialists() + supply::luxury_happiness(supplies.luxuries(city.position), has_market);
        happiness.update(city.population, luxury_bonus);
        if happiness.in_disorder() && mine {
            log.push(format!("Civil disorder in {}!", city.name));
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
            }
        }

        if let Some(completed) = city.process_production(&buildings, gov, dist) {
            match completed {
                ProductionItem::Unit(ut) => {
                    // Keep computer players from flooding the map.
                    let at_cap = !mine && ut.is_military() && army_sizes.get(&city.owner).copied().unwrap_or(0) >= 8;
                    if !at_cap {
                        spawn_unit(&mut commands, city.position, ut, city.owner);
                        if ut.is_military() {
                            *army_sizes.entry(city.owner).or_default() += 1;
                        }
                    }
                    {
                        if mine {
                            log.push(format!("{} trains a {}.", city.name, ut.name()));
                            // Once the army outgrows what the government supports for free, a city left on
                            // its default order turns to the cheapest building it can construct.
                            let supported = gov.free_unit_support() as usize * capitals.len().max(1);
                            if ut == UnitType::Warrior && army_sizes.get(&PLAYER).copied().unwrap_or(0) > supported + 2 {
                                let next = Building::ALL
                                    .into_iter()
                                    .filter(|b| !b.is_wonder() && !buildings.buildings.contains(b))
                                    .filter(|b| b.required_tech().is_none_or(|t| tech.researched.contains(&t)))
                                    .min_by_key(|b| b.cost());
                                if let Some(building) = next {
                                    log.push(format!("{} starts building a {}.", city.name, building.name()));
                                    city.production_queue = Some(ProductionItem::Building(building));
                                }
                            }
                        } else if let Some(civ) = civs.players.iter().find(|c| c.id == city.owner) {
                            // Computer players field the strongest unit their technology and resources allow.
                            let best = UnitType::ALL
                                .into_iter()
                                .filter(|u| u.is_military())
                                .filter(|u| u.required_tech().is_none_or(|t| civ.tech.researched.contains(&t)))
                                .filter(|u| supplies.has(city.position, u.required_resources()))
                                .max_by_key(|u| (u.stats().0 + u.stats().1, u.stats().2));
                            // Enough workers to road the resources and link the cities: one per two cities.
                            let workers = worker_counts.entry(city.owner).or_default();
                            let wants_worker = *workers < city_counts.get(&city.owner).copied().unwrap_or(1).div_ceil(2);
                            // Small empires send out settlers now and then.
                            let wants_settler = city.population >= 2 && rng.gen_bool(0.2);
                            if wants_worker {
                                *workers += 1;
                                city.production_queue = Some(ProductionItem::Unit(UnitType::Worker));
                            } else if wants_settler {
                                city.production_queue = Some(ProductionItem::Unit(UnitType::Settler));
                            } else if let Some(best) = best {
                                city.production_queue = Some(ProductionItem::Unit(best));
                            }
                        }
                    }
                }
                ProductionItem::Building(b) => {
                    if mine {
                        log.push(format!("{} completes the {}.", city.name, b.name()));
                    }
                    buildings.buildings.push(b);
                }
            }
        }

        if mine {
            total_science += city.science_per_turn(&buildings, gov, dist);
            total_gold += city.gold_per_turn(&buildings) as i32 + 1;
        }
    }

    total_gold += trade.total_gold() as i32 + foreign.pacts.len() as i32 * foreign::PACT_INCOME;
    {
        let mut ctx = foreign::Ctx { diplomacy: &mut diplomacy, foreign: &mut foreign, civs: &mut civs, state: &mut state, tech: &mut tech, log: &mut log };
        total_gold += foreign::settle_deals(&mut ctx, &supplies);
    }
    total_science += trade.total_science();

    let free_support = gov.free_unit_support() * player_cities.max(1);
    let military = units.iter().filter(|(_, u)| u.owner == PLAYER && u.unit_type.is_military()).count() as u32;
    if military > free_support {
        total_gold -= (military - free_support) as i32 * gov.unit_upkeep_food() as i32;
    }
    state.gold_per_turn = total_gold;
    state.gold = (state.gold + total_gold).max(0);
    tech.science_per_turn = total_science;

    for (_, mut unit) in &mut units {
        if let Some(done) = unit.process_build() {
            if let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == unit.position) {
                tile.add_improvement(done);
                if unit.owner == PLAYER {
                    log.push(format!("A {:?} is finished.", done));
                }
            }
        }
        if unit.build_turns_left.is_none() {
            // Boats make steady progress whatever they carry.
            unit.movement_left = if map.water.contains(&unit.position) { 3 } else { unit.unit_type.stats().2 };
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
        for (owner, capital) in capitals.iter().filter(|(owner, _)| **owner != PLAYER) {
            for (city, _) in city_owners.iter().filter(|(p, o)| o == owner && !supplies.linked.contains(p) && p.distance_to(*capital) <= 12) {
                routes.push((*owner, *capital, *city));
            }
        }
        // Nearest goals first, so short useful roads are finished before long ones.
        routes.sort_by_key(|(_, from, to)| from.distance_to(*to));
        for (owner, from, to) in routes {
            if let Some(route) = find_path(from, to, land.iter().copied(), false) {
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

    let mut dead: HashSet<Entity> = HashSet::new();
    for attack in attacks {
        if dead.contains(&attack.attacker) {
            continue;
        }
        let Ok((_, attacker)) = units.get(attack.attacker) else { continue };
        let (a_type, a_owner) = (attacker.unit_type, attacker.owner);
        let defender = units
            .iter()
            .filter(|(e, u)| u.position == attack.target && u.owner != a_owner && !dead.contains(e))
            .max_by_key(|(_, u)| u.unit_type.stats().1)
            .map(|(e, u)| (e, u.unit_type, u.owner, u.fortified));
        let Some((d_entity, d_type, d_owner, dug_in)) = defender else {
            if let Ok((_, mut attacker)) = units.get_mut(attack.attacker) {
                attacker.position = attack.target;
            }
            continue;
        };
        if d_owner == PLAYER && a_owner != BARBARIANS && diplomacy.get_treaty(PLAYER, a_owner) != Treaty::War {
            diplomacy.set_treaty(PLAYER, a_owner, Treaty::War);
            log.push(format!("The {} attack means war!", civ_name(a_owner)));
            foreign.since.insert((PLAYER, a_owner), state.turn);
        }
        let terrain = defence_terrain(tiles.iter().find(|t| t.coord == attack.target).map(|t| t.terrain), city_owners.iter().any(|(p, _)| *p == attack.target) || dug_in);
        if fog.visible.contains(&attack.target) {
            bursts.send(Burst(attack.target));
        }
        if resolve_combat(a_type, d_type, terrain, None, None) {
            dead.insert(d_entity);
            commands.entity(d_entity).try_despawn_recursive();
            if d_owner == PLAYER {
                log.push(format!("Our {} was destroyed by a {} {}.", d_type.name(), civ_name(a_owner), a_type.name()));
            }
            let still_defended = units.iter().any(|(e, u)| u.position == attack.target && u.owner != a_owner && !dead.contains(&e));
            if !still_defended {
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
        }
    }

    // An enemy military unit standing in an undefended city takes it.
    for (mut city, _, _) in &mut cities {
        let defended = units.iter().any(|(e, u)| !dead.contains(&e) && u.position == city.position && u.owner == city.owner);
        let occupier = units
            .iter()
            .find(|(e, u)| {
                !dead.contains(e) && u.position == city.position && u.owner != PLAYER && u.owner != city.owner && u.unit_type.is_military() && hostile(u.owner, city.owner)
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
            city.production_progress = 0;
            city.production_queue = Some(ProductionItem::Unit(UnitType::Warrior));
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
        foreign::turn(&mut ctx, &fog, &unit_owners, &city_owners);
    }

    let all_positions: Vec<Hex> = units.iter().filter(|(e, _)| !dead.contains(e)).map(|(_, u)| u.position).collect();
    process_barbarians(&mut barbarians, &mut commands, &tiles, &all_positions);

    let unit_positions: Vec<Hex> = units.iter().filter(|(e, u)| u.owner == PLAYER && !dead.contains(e)).map(|(_, u)| u.position).collect();
    let city_positions: Vec<Hex> = cities.iter().filter(|(c, _, _)| c.owner == PLAYER).map(|(c, _, _)| c.position).collect();
    fog.update_visibility(&unit_positions, &city_positions);
    cycle.advance = true;

    if let Some(completed) = tech.process_turn() {
        log.push(format!("Discovery! We have learned {}.", completed.name()));
        if completed == Tech::Monarchy {
            state.government = Government::Monarchy;
            log.push("Our government becomes a Monarchy.");
        }
        if completed == Tech::Democracy {
            state.government = Government::Democracy;
        }
    }

    // The space race opens with Democracy; a new stage is completed every ten turns after that.
    if tech.researched.contains(&Tech::Democracy) && space.progress < 4 && state.turn % 10 == 0 {
        let next = [SpaceComponent::Thrusters, SpaceComponent::LifeSupport, SpaceComponent::Habitation, SpaceComponent::SolarPanels][space.progress as usize].clone();
        log.push(format!("Our spaceship gains its {} ({}/4).", next.name(), space.progress + 1));
        space.add_component(next);
    }

    state.score = Score::calculate(cities.iter().map(|(c, b, _)| (c, b)), &tech);

    let total_cities = cities.iter().count();
    let rivals_remain = cities.iter().any(|(c, _, _)| c.owner != PLAYER && c.owner != BARBARIANS);
    if player_cities == 0 {
        state.victory = Some("Your civilization has fallen.".into());
    } else if !rivals_remain {
        state.victory = Some("Conquest Victory!".into());
    } else if player_cities >= 5 && player_cities * 3 >= total_cities * 2 {
        state.victory = Some("Domination Victory!".into());
    } else if space.is_complete() {
        state.victory = Some("Space Race Victory!".into());
    }
}

/// Moves the selection on to the next unit that needs orders and points the camera at it.
fn advance_selection(
    mut cycle: ResMut<Cycle>,
    mut selection: ResMut<Selection>,
    units: Query<(Entity, &Unit)>,
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
        // Nothing left to order: release a unit that has finished so the turn can end.
        None if current.is_some_and(|(_, u)| u.owner == PLAYER) => selection.unit = None,
        None => {}
    }
    dirty.ui = true;
}

/// Rebuilds the resource network whenever the map, borders or deals may have changed.
fn refresh_supplies(
    dirty: Res<Dirty>,
    foreign: Res<Foreign>,
    tech: Res<TechState>,
    civs: Res<Civilizations>,
    tiles: Query<&HexTile>,
    cities: Query<(&City, &CityBuildings)>,
    mut supplies: ResMut<Supplies>,
) {
    if !(dirty.map || foreign.is_changed() || tech.is_changed()) {
        return;
    }
    let nodes: Vec<supply::CityNode> = cities
        .iter()
        .map(|(c, b)| supply::CityNode { position: c.position, owner: c.owner, capital: c.is_capital, harbor: b.buildings.contains(&Building::Harbor) })
        .collect();
    let mut techs: HashMap<u32, HashSet<Tech>> = civs.players.iter().map(|c| (c.id, c.tech.researched.clone())).collect();
    techs.insert(PLAYER, tech.researched.clone());
    supply::recompute(&mut supplies, tiles.iter(), &nodes, &techs, &foreign.deals);
}
