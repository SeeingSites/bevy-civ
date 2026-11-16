//! Bevy Civ — game setup, turn processing and player orders.
//! Everything visual (sprites, camera, interface) lives in `client`.

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use hexx::{Hex, HexLayout};
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
use bevy_civ::pathfinding::{find_path, find_route, road_network, step_cost, turns_along, Road, StepCost, Travel};
use bevy_civ::unit::{Domain, AIR_RANGE};
use bevy_civ::save::{save_game, SaveData};
use bevy_civ::score::Score;
use bevy_civ::space::{SpaceComponent, SpaceRace};
use bevy_civ::tech::{Tech, TechState};
use bevy_civ::terrain::{random_resource, HexTile, Improvement, Terrain};
use bevy_civ::trade::TradeRoutes;
use bevy_civ::unit::{spawn_unit, Unit, UnitType};

mod client;
mod foreign;
mod governor;
mod scenario;
mod supply;

use client::sfx::Sfx;
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
    TradeRoute,
    Save,
    /// Pick one of the player's units (from the list of units sharing a tile).
    Select(Entity),
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
    unit.owner == PLAYER && unit.can_move() && unit.build_turns_left.is_none() && !unit.fortified
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
            (handle_map_click, apply_actions, process_turn, continue_orders, advance_selection, preview_route, refresh_supplies)
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
    mut cycle: ResMut<Cycle>,
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
        let enemy = snapshot.iter().find(|u| u.1 == hex && u.2 != unit.owner).copied();
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
        walk(unit, approach, &map, &roads, &mut fog, &mut trail);
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
                if resolve_combat(unit.unit_type, enemy_type, terrain, None, None) {
                    log.push(format!("Victory! Your {} defeated the enemy {}.", unit.unit_type.name(), enemy_type.name()));
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
        capture(unit, cities.iter_mut().map(|(_, c)| c), &mut log);
    }
}

/// An army standing in a foreign city takes it.
fn capture<'a>(unit: &Unit, cities: impl Iterator<Item = Mut<'a, City>>, log: &mut GameLog) {
    if !unit.unit_type.is_army() {
        return;
    }
    for mut city in cities {
        if city.position == unit.position && city.owner != PLAYER {
            city.owner = PLAYER;
            city.is_capital = false;
            log.push(format!("Your {} captured {}!", unit.unit_type.name(), city.name));
        }
    }
}

/// The player's units on a tile, in the fixed order that Tab and the unit panel use.
pub fn own_stack(at: Hex, entities: impl Iterator<Item = Entity>, place: impl Fn(Entity) -> Option<(Hex, u32)>) -> Vec<Entity> {
    let mut stack: Vec<Entity> = entities.filter(|e| place(*e) == Some((at, PLAYER))).collect();
    stack.sort_by_key(|e| e.index());
    stack
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

/// What a step costs: roads and railroads speed troops on foot; boarding a boat ends the turn.
fn move_cost(domain: Domain, from: Hex, to: Hex, map: &MapData, roads: &HashMap<Hex, Road>) -> StepCost {
    if ends_move(domain, from, to, map) {
        StepCost::EndsTurn
    } else {
        step_cost(from, to, roads, domain == Domain::Land)
    }
}

/// The moves a unit gets when a turn begins on `at`: boats make steady progress whatever they carry.
fn moves_per_turn(unit_type: UnitType, at: Hex, map: &MapData) -> u32 {
    if map.water.contains(&at) { 3 } else { unit_type.stats().2 }
}

/// Moves a unit along `route` (which starts where it stands) while it has moves left, revealing the
/// land around each tile entered and adding it to `trail`.
fn walk(unit: &mut Unit, route: &[Hex], map: &MapData, roads: &HashMap<Hex, Road>, fog: &mut FogOfWar, trail: &mut Vec<Hex>) {
    for next in route.iter().skip(1) {
        if !unit.can_move() {
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
    (map, tech): (Res<MapData>, Res<TechState>),
    mut fog: ResMut<FogOfWar>,
    mut log: ResMut<GameLog>,
    mut dirty: ResMut<Dirty>,
    mut commands: Commands,
) {
    if std::mem::replace(&mut *last_turn, state.turn) == state.turn || state.victory.is_some() {
        return;
    }
    let foreign: HashSet<Hex> = units.iter().filter(|(_, u, _)| u.owner != PLAYER).map(|(_, u, _)| u.position).collect();
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
        walk(&mut unit, &route[..clear], &map, &roads, &mut fog, &mut trail);
        capture(&unit, cities.iter_mut(), &mut log);
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

/// Works out the route to show for the selected unit, re-planning only when something it depends on changed.
#[allow(clippy::too_many_arguments)]
fn preview_route(
    selection: Res<Selection>,
    hovered: Res<client::map::Hovered>,
    units: Query<(&Unit, Option<&GoTo>)>,
    cities: Query<&City>,
    tiles: Query<&HexTile>,
    (map, tech, fog): (Res<MapData>, Res<TechState>, Res<FogOfWar>),
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
    let turns = turns_along(&route, unit.budget(), |at| moves_per_turn(unit.unit_type, at, &map), |a, b| move_cost(domain, a, b, &map, &roads));
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
    (mut cycle, supplies, map, mut end_turn): (ResMut<Cycle>, Res<Supplies>, Res<MapData>, EventWriter<EndTurnNow>),
    (unit_entities, mut sounds): (Query<Entity, With<Unit>>, EventWriter<Sfx>),
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
                if let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == unit.position) {
                    if tile.transform_terrain() {
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
    (trade, map, supplies): (Res<TradeRoutes>, Res<MapData>, Res<Supplies>),
    mut space: ResMut<SpaceRace>,
    (mut log, mut dirty, mut cycle): (ResMut<GameLog>, ResMut<Dirty>, ResMut<Cycle>),
    mut bursts: EventWriter<Burst>,
    mut commands: Commands,
) {
    if actions.read().count() == 0 || state.victory.is_some() {
        return;
    }
    dirty.map = true;
    dirty.ui = true;
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
                    let civ_view = governor::CivView { tech: &civ.tech, aggression: civ.aggression, cities: 1, workers: 1, settlers: 1, military: 0, navy: 0, air: 0, road_work: 0, cut_off: 0, at_war: true };
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
                needs_orders = true;
            }
        }

        if let Some(completed) = city.process_production(&buildings, gov, dist) {
            needs_orders = true;
            match completed {
                ProductionItem::Unit(ut) => {
                    // Keep computer players from flooding the map.
                    let at_cap = !mine && ut.is_army() && army_sizes.get(&city.owner).copied().unwrap_or(0) >= governor::army_cap(city_counts.get(&city.owner).copied().unwrap_or(1));
                    if !at_cap {
                        spawn_unit(&mut commands, city.position, ut, city.owner);
                        if ut.is_army() {
                            *army_sizes.entry(city.owner).or_default() += 1;
                            just_trained = true;
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
        if !mine {
            let income = ai_income.entry(owner).or_default();
            income.0 += city.science_per_turn(&buildings, gov, dist);
            income.1 += city.gold_per_turn(&buildings) as i32 + 2 + supplies.yields.get(&city.position).map(|y| y.2 as i32).unwrap_or(0);
        }

        if mine {
            total_science += city.science_per_turn(&buildings, gov, dist);
            total_gold += city.gold_per_turn(&buildings) as i32 + 1;
        }
    }

    for civ in civs.players.iter_mut().filter(|c| !c.is_human) {
        let (science, gold) = ai_income.get(&civ.id).copied().unwrap_or_default();
        civ.gold += gold;
        // Savings beyond a reserve are invested in research.
        let invested = ((civ.gold - 250) / 10).max(0);
        civ.gold -= invested;
        civ.tech.science_per_turn = (science + invested as u32 / 2).max(1);
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
            let moves = moves_per_turn(unit.unit_type, unit.position, &map);
            unit.refresh(moves);
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
        let terrain = defence_terrain(tiles.iter().find(|t| t.coord == attack.target).map(|t| t.terrain), city_owners.iter().any(|(p, _)| *p == attack.target) || dug_in);
        if fog.visible.contains(&attack.target) {
            bursts.send(Burst(attack.target));
        }
        if resolve_combat(a_type, defending_as(d_type, attack.target, &map), terrain, None, None) {
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
    supply::recompute(&mut supplies, tiles.iter(), &nodes, &techs, &foreign.transfers());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_map() -> MapData {
        MapData {
            layout: HexLayout::flat(),
            cells: HashSet::new(),
            water: HashSet::new(),
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        }
    }

    fn legion(moves: u32) -> Unit {
        Unit { unit_type: UnitType::Legion, movement_left: moves, owner: PLAYER, position: Hex::ZERO, build_turns_left: None, current_build: None, fortified: false, road_credit: 0 }
    }

    #[test]
    fn units_march_three_times_as_far_on_roads_and_freely_on_rails() {
        let route: Vec<Hex> = (0..=9).map(|q| Hex::new(q, 0)).collect();
        let map = plain_map();
        let mut fog = FogOfWar::default();
        let walked = |roads: &HashMap<Hex, Road>, fog: &mut FogOfWar| {
            let mut unit = legion(1);
            let mut trail = Vec::new();
            walk(&mut unit, &route, &map, roads, fog, &mut trail);
            trail.len()
        };
        assert_eq!(walked(&HashMap::new(), &mut fog), 1);
        let roads: HashMap<Hex, Road> = route.iter().map(|h| (*h, Road::Road)).collect();
        assert_eq!(walked(&roads, &mut fog), 3);
        let rails: HashMap<Hex, Road> = route.iter().map(|h| (*h, Road::Rail)).collect();
        assert_eq!(walked(&rails, &mut fog), 9);
    }
}
