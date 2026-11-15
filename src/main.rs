//! Bevy Civ — game setup, turn processing and player orders.
//! Everything visual (sprites, camera, interface) lives in `client`.

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use hexx::{shapes, Hex, HexLayout};
use rand::Rng;
use std::collections::HashMap;

use bevy_civ::ai::{process_ai_turn, Civilizations};
use bevy_civ::barbarians::{process_barbarians, BarbarianState};
use bevy_civ::building::CityBuildings;
use bevy_civ::city::{spawn_city, City, ProductionItem};
use bevy_civ::combat::resolve_combat;
use bevy_civ::diplomacy::{Diplomacy, Happiness, Treaty};
use bevy_civ::fog::FogOfWar;
use bevy_civ::government::Government;
use bevy_civ::pathfinding::{find_path, move_along_path};
use bevy_civ::save::save_game;
use bevy_civ::score::Score;
use bevy_civ::space::{SpaceComponent, SpaceRace};
use bevy_civ::tech::{Tech, TechState};
use bevy_civ::terrain::{random_resource, HexTile, Improvement, Terrain};
use bevy_civ::trade::TradeRoutes;
use bevy_civ::unit::{spawn_unit, Unit, UnitType};

mod client;

pub const MAP_RADIUS: u32 = 10;
pub const PLAYER: u32 = 0;
pub const BARBARIANS: u32 = 99;
/// Tiles around a city that belong to its owner.
pub const CITY_RADIUS: i32 = 2;

const CITY_NAMES: [&str; 10] = [
    "Ashford", "Brightwater", "Stonehaven", "Eastmarch", "Highgate", "Riverbend", "Oakhollow",
    "Silverkeep", "Windmere", "Thornwall",
];

#[derive(Resource)]
pub struct MapData {
    pub layout: HexLayout,
}

#[derive(Resource)]
pub struct GameState {
    pub turn: u32,
    pub gold: i32,
    pub gold_per_turn: i32,
    pub government: Government,
    pub victory: Option<String>,
    pub score: Score,
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
    Build(Improvement),
    Transform,
    FoundCity,
    Skip,
    Deselect,
    SetTreaty(u32, Treaty),
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
        .insert_resource(GameState {
            turn: 1,
            gold: 0,
            gold_per_turn: 0,
            government: Government::Despotism,
            victory: None,
            score: Score::default(),
        })
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
        .add_event::<Action>()
        .add_event::<MapClick>()
        .add_event::<Burst>()
        .add_plugins(client::ClientPlugin)
        .add_systems(Startup, setup_world)
        .add_systems(
            Update,
            (handle_map_click, apply_actions, process_turn)
                .chain()
                .in_set(GameLogic),
        )
        .run();
}

/// Game rules run after input is gathered and before the views are refreshed.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameLogic;

/// Smooth pseudo-random field in [0, 1] built from a few sine waves.
struct Field {
    waves: Vec<(f32, f32, f32, f32)>,
}

impl Field {
    fn new(rng: &mut impl Rng, base_freq: f32) -> Self {
        let waves = (0..5)
            .map(|i| {
                let freq = base_freq * (1.0 + i as f32 * 0.55);
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                (angle.cos() * freq, angle.sin() * freq, rng.gen_range(0.0..std::f32::consts::TAU), 1.0 / (1.0 + i as f32 * 0.5))
            })
            .collect();
        Self { waves }
    }

    fn at(&self, p: Vec2) -> f32 {
        let total: f32 = self.waves.iter().map(|w| w.3).sum();
        let v: f32 = self.waves.iter().map(|(fx, fy, phase, amp)| (p.x * fx + p.y * fy + phase).sin() * amp).sum();
        0.5 + 0.5 * v / total
    }
}

fn setup_world(mut commands: Commands, mut fog: ResMut<FogOfWar>, mut log: ResMut<GameLog>) {
    let layout = HexLayout::flat().with_hex_size(client::map::HEX_SIZE);
    let mut rng = rand::thread_rng();

    let starts = [(Hex::ZERO, PLAYER), (Hex::new(7, -4), 1), (Hex::new(-6, 6), 2)];
    let height = Field::new(&mut rng, 0.006);
    let moisture = Field::new(&mut rng, 0.008);

    for hex in shapes::hexagon(Hex::ZERO, MAP_RADIUS) {
        let pos = layout.hex_to_world_pos(hex);
        let edge = hex.distance_to(Hex::ZERO) as f32 / MAP_RADIUS as f32;
        // Continents in the middle, ocean towards the rim.
        let h = height.at(pos) + rng.gen_range(-0.04..0.04) - edge.powi(3) * 0.38;
        let m = moisture.at(pos) + rng.gen_range(-0.05..0.05);
        let near_start = starts.iter().any(|(s, _)| s.distance_to(hex) <= 1);
        let terrain = if near_start {
            if hex.x.rem_euclid(2) == 0 { Terrain::Grassland } else { Terrain::Plains }
        } else if h < 0.34 {
            Terrain::Water
        } else if h > 0.78 {
            Terrain::Mountain
        } else if h > 0.66 {
            Terrain::Hills
        } else if m < 0.3 {
            Terrain::Desert
        } else if m > 0.62 {
            Terrain::Forest
        } else if m > 0.46 {
            Terrain::Grassland
        } else {
            Terrain::Plains
        };
        let resource = random_resource(&mut rng, terrain);
        commands.spawn(HexTile { coord: hex, terrain, improvements: vec![], resource });
    }

    spawn_city(&mut commands, starts[0].0, "Aurelia".into(), 1, PLAYER, true);
    spawn_unit(&mut commands, Hex::new(0, 1), UnitType::Settler, PLAYER);
    spawn_unit(&mut commands, Hex::new(1, 0), UnitType::Worker, PLAYER);
    spawn_unit(&mut commands, Hex::new(-1, 0), UnitType::Warrior, PLAYER);

    spawn_city(&mut commands, starts[1].0, "Rome".into(), 1, 1, true);
    spawn_unit(&mut commands, starts[1].0 + Hex::new(0, 1), UnitType::Legion, 1);
    spawn_city(&mut commands, starts[2].0, "Thebes".into(), 1, 2, true);
    spawn_unit(&mut commands, starts[2].0 + Hex::new(1, 0), UnitType::WarChariot, 2);

    fog.reveal(Hex::ZERO, 3);
    commands.insert_resource(MapData { layout });
    log.push("Your people settle at Aurelia. Lead them well.");
}

fn handle_map_click(
    mut clicks: EventReader<MapClick>,
    mut selection: ResMut<Selection>,
    mut units: Query<(Entity, &mut Unit)>,
    mut cities: Query<(Entity, &mut City)>,
    tiles: Query<&HexTile>,
    state: Res<GameState>,
    mut fog: ResMut<FogOfWar>,
    mut diplomacy: ResMut<Diplomacy>,
    mut log: ResMut<GameLog>,
    mut dirty: ResMut<Dirty>,
    mut path: ResMut<PathPreview>,
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
        let Some(route) = find_path(unit.position, hex, tiles.iter().map(|t| (t.coord, t.terrain))) else {
            log.push("There is no route to that tile.");
            continue;
        };
        dirty.map = true;

        let enemy = snapshot.iter().find(|u| u.1 == hex && u.2 != unit.owner).copied();
        // Stop next to a defended tile; the last step is the attack itself.
        let walk = if enemy.is_some() { &route[..route.len() - 1] } else { &route[..] };
        move_along_path(&mut unit.position, &mut unit.movement_left, walk);
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
                if resolve_combat(unit.unit_type, enemy_type, terrain) {
                    log.push(format!("Victory! Your {} defeated the enemy {}.", unit.unit_type.name(), enemy_type.name()));
                    commands.entity(enemy_entity).despawn_recursive();
                    let more_defenders = snapshot.iter().any(|u| u.1 == hex && u.2 != PLAYER && u.0 != enemy_entity);
                    if !more_defenders {
                        unit.position = hex;
                    }
                } else {
                    log.push(format!("Defeat. Your {} was lost attacking a {}.", unit.unit_type.name(), enemy_type.name()));
                    commands.entity(selected).despawn_recursive();
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

fn apply_actions(
    mut actions: EventReader<Action>,
    mut selection: ResMut<Selection>,
    mut units: Query<&mut Unit>,
    mut cities: Query<&mut City>,
    mut tiles: Query<&mut HexTile>,
    state: Res<GameState>,
    mut tech: ResMut<TechState>,
    mut diplomacy: ResMut<Diplomacy>,
    mut trade: ResMut<TradeRoutes>,
    mut fog: ResMut<FogOfWar>,
    civs: Res<Civilizations>,
    mut log: ResMut<GameLog>,
    mut dirty: ResMut<Dirty>,
    mut path: ResMut<PathPreview>,
    mut commands: Commands,
) {
    for action in actions.read() {
        dirty.ui = true;
        dirty.map = true;
        let selected_unit = selection.unit.filter(|e| units.get(*e).is_ok_and(|u| u.owner == PLAYER));
        match action {
            Action::EndTurn => {}
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
                let name = CITY_NAMES[(count.max(1) - 1) % CITY_NAMES.len()];
                let city = spawn_city(&mut commands, here, name.into(), 1, PLAYER, false);
                commands.entity(entity).despawn_recursive();
                selection.unit = None;
                selection.city = Some(city);
                fog.reveal(here, 3);
                log.push(format!("The city of {name} is founded."));
            }
            Action::Skip => {
                if let Some(mut unit) = selected_unit.and_then(|e| units.get_mut(e).ok()) {
                    unit.movement_left = 0;
                }
            }
            Action::SetTreaty(civ, treaty) => {
                let name = civs.players.iter().find(|c| c.id == *civ).map(|c| c.name.clone()).unwrap_or_default();
                diplomacy.set_treaty(PLAYER, *civ, *treaty);
                log.push(match treaty {
                    Treaty::Peace => format!("Peace is agreed with {name}."),
                    Treaty::Alliance => format!("An alliance is formed with {name}."),
                    Treaty::War => format!("War is declared on {name}!"),
                    Treaty::None => format!("Treaties with {name} are dissolved."),
                });
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
                save_game(state.turn, state.gold, state.government.name());
                log.push("Game saved.");
            }
            Action::SetProduction(item) => {
                let Some(mut city) = selection.city.and_then(|e| cities.get_mut(e).ok()) else { continue };
                if city.owner == PLAYER {
                    city.production_queue = Some(item.clone());
                    log.push(format!("{} begins work on {}.", city.name, item.name()));
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
    mut actions: EventReader<Action>,
    mut state: ResMut<GameState>,
    mut tech: ResMut<TechState>,
    mut cities: Query<(&mut City, &mut CityBuildings, &mut Happiness)>,
    mut units: Query<&mut Unit>,
    mut tiles: Query<&mut HexTile>,
    mut civs: ResMut<Civilizations>,
    mut fog: ResMut<FogOfWar>,
    mut barbarians: ResMut<BarbarianState>,
    trade: Res<TradeRoutes>,
    mut space: ResMut<SpaceRace>,
    mut log: ResMut<GameLog>,
    mut dirty: ResMut<Dirty>,
    mut path: ResMut<PathPreview>,
    mut commands: Commands,
) {
    if !actions.read().any(|a| *a == Action::EndTurn) || state.victory.is_some() {
        return;
    }
    dirty.map = true;
    dirty.ui = true;
    path.0.clear();
    state.turn += 1;

    let gov = state.government;
    let mut total_science = 0;
    let mut total_gold = 0i32;
    let mut player_cities = 0;

    let capitals: HashMap<u32, Hex> =
        cities.iter().filter(|(c, _, _)| c.is_capital).map(|(c, _, _)| (c.owner, c.position)).collect();
    let mut army_sizes: HashMap<u32, usize> = HashMap::new();
    for unit in units.iter() {
        *army_sizes.entry(unit.owner).or_default() += 1;
    }

    for (mut city, mut buildings, mut happiness) in &mut cities {
        let mine = city.owner == PLAYER;
        if mine {
            player_cities += 1;
        }
        let dist = capitals.get(&city.owner).map(|cp| city.position.distance_to(*cp) as u32).unwrap_or(0);

        if city.process_food(&buildings, gov, dist) && mine {
            log.push(format!("{} grows to size {}.", city.name, city.population));
        }

        let mut luxury_bonus = city.luxury_from_specialists();
        for tile in tiles.iter() {
            if tile.coord.distance_to(city.position) <= CITY_RADIUS {
                if let Some(res) = tile.resource {
                    luxury_bonus += res.happiness_bonus();
                }
            }
        }
        happiness.update(city.population, luxury_bonus);
        if happiness.in_disorder() && mine {
            log.push(format!("Civil disorder in {}!", city.name));
        }

        if let Some(completed) = city.process_production(&buildings, gov, dist) {
            match completed {
                ProductionItem::Unit(ut) => {
                    let has_resource = ut.required_resource().is_none_or(|req| tiles.iter().any(|t| t.resource == Some(req)));
                    // Keep computer players from flooding the map.
                    let at_cap = !mine && army_sizes.get(&city.owner).copied().unwrap_or(0) >= 6;
                    if has_resource && !at_cap {
                        spawn_unit(&mut commands, city.position, ut, city.owner);
                        *army_sizes.entry(city.owner).or_default() += 1;
                        if mine {
                            log.push(format!("{} trains a {}.", city.name, ut.name()));
                        }
                    } else if mine {
                        log.push(format!("{} cannot train a {} without its strategic resource.", city.name, ut.name()));
                        city.production_progress += ut.cost() / 2;
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

    total_gold += trade.total_gold() as i32;
    total_science += trade.total_science();

    let free_support = gov.free_unit_support() * player_cities.max(1);
    let military = units.iter().filter(|u| u.owner == PLAYER && u.unit_type.is_military()).count() as u32;
    if military > free_support {
        total_gold -= (military - free_support) as i32 * gov.unit_upkeep_food() as i32;
    }
    state.gold_per_turn = total_gold;
    state.gold = (state.gold + total_gold).max(0);
    tech.science_per_turn = total_science;

    for mut unit in &mut units {
        if let Some(done) = unit.process_build() {
            if let Some(mut tile) = tiles.iter_mut().find(|t| t.coord == unit.position) {
                tile.add_improvement(done);
                if unit.owner == PLAYER {
                    log.push(format!("A {:?} is finished.", done));
                }
            }
        }
        if unit.build_turns_left.is_none() {
            unit.movement_left = unit.unit_type.stats().2;
        }
    }

    let city_owners: Vec<(Hex, u32)> = cities.iter().map(|(c, _, _)| (c.position, c.owner)).collect();
    process_ai_turn(&mut civs, &mut units, &city_owners, &tiles, &mut commands);
    process_barbarians(&mut barbarians, &mut commands, &tiles, &units);

    let unit_positions: Vec<Hex> = units.iter().filter(|u| u.owner == PLAYER).map(|u| u.position).collect();
    let city_positions: Vec<Hex> = cities.iter().filter(|(c, _, _)| c.owner == PLAYER).map(|(c, _, _)| c.position).collect();
    fog.update_visibility(&unit_positions, &city_positions);

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

    if space.can_build(&tech) && space.progress < 4 {
        let next = [SpaceComponent::Thrusters, SpaceComponent::LifeSupport, SpaceComponent::Habitation, SpaceComponent::SolarPanels][space.progress as usize].clone();
        space.add_component(next);
    }

    state.score = Score::calculate(cities.iter().map(|(c, b, _)| (c, b)), &tech);

    if player_cities >= 5 {
        state.victory = Some("Domination Victory!".into());
    } else if tech.researched.contains(&Tech::Democracy) {
        state.victory = Some("Cultural Victory!".into());
    } else if space.is_complete() {
        state.victory = Some("Space Race Victory!".into());
    } else if player_cities == 0 {
        state.victory = Some("Your civilization has fallen.".into());
    }
}
