use bevy::prelude::*;
use hexx::{shapes, *};
use rand::Rng;

mod city;
mod unit;
mod terrain;
mod tech;
mod government;
mod combat;
mod building;
mod ai;
mod fog;
mod diplomacy;
mod pathfinding;

use city::*;
use unit::*;
use terrain::*;
use tech::*;
use government::*;
use combat::*;
use building::*;
use ai::*;
use fog::*;
use diplomacy::*;
use pathfinding::*;

const HEX_SIZE: f32 = 28.0;
const MAP_RADIUS: u32 = 10;

#[derive(Resource)]
struct MapData {
    layout: HexLayout,
}

#[derive(Resource)]
struct GameState {
    turn: u32,
    gold: i32,
    science: i32,
    government: Government,
    victory: Option<String>,
}

#[derive(Resource, Default)]
struct Selection {
    selected_unit: Option<Entity>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameState {
            turn: 1,
            gold: 0,
            science: 0,
            government: Government::Despotism,
            victory: None,
        })
        .insert_resource(TechState::default())
        .insert_resource(Selection::default())
        .insert_resource(Civilizations::default())
        .insert_resource(FogOfWar::default())
        .insert_resource(Diplomacy::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (camera_pan, handle_click, handle_worker_build, process_turn, update_ui, update_unit_positions))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut fog: ResMut<FogOfWar>,
) {
    commands.spawn(Camera2d);

    let layout = HexLayout::flat().with_hex_size(HEX_SIZE);
    let mesh = meshes.add(hexagonal_plane(&layout));

    let mut rng = rand::thread_rng();

    for hex in shapes::hexagon(Hex::ZERO, MAP_RADIUS) {
        let terrain = random_terrain(&mut rng);
        let resource = random_resource(&mut rng, terrain);
        let pos = layout.hex_to_world_pos(hex);

        commands.spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(materials.add(terrain.color())),
            Transform::from_xyz(pos.x, pos.y, 0.0),
            HexTile { coord: hex, terrain, improvements: vec![], resource },
        ));
    }

    let start = Hex::ZERO;
    spawn_city(&mut commands, &mut meshes, &mut materials, &layout, start, "Capital".into(), 1);
    spawn_unit(&mut commands, &mut meshes, &mut materials, &layout, start, UnitType::Settler);
    spawn_unit(&mut commands, &mut meshes, &mut materials, &layout, Hex::new(1, 0), UnitType::Worker);

    // AI unique units
    let ai1 = Hex::new(6, -3);
    spawn_unit_for_owner(&mut commands, &mut meshes, &mut materials, &layout, ai1, UnitType::Legion, 1);

    let ai2 = Hex::new(-5, 4);
    spawn_unit_for_owner(&mut commands, &mut meshes, &mut materials, &layout, ai2, UnitType::WarChariot, 2);

    fog.reveal(start, 3);

    commands.insert_resource(MapData { layout });

    commands.spawn((
        Text::new("Loading..."),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        Name::new("hud"),
    ));
}

fn hexagonal_plane(hex_layout: &HexLayout) -> Mesh {
    let mesh_info = PlaneMeshBuilder::new(hex_layout)
        .facing(Vec3::Z)
        .with_scale(Vec3::splat(0.92))
        .center_aligned()
        .build();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh_info.vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, mesh_info.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, mesh_info.uvs)
        .with_inserted_indices(Indices::U16(mesh_info.indices))
}

fn camera_pan(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
) {
    let Ok(mut t) = q.get_single_mut() else { return };
    let speed = 400.0 * time.delta_secs();
    if keyboard.pressed(KeyCode::KeyW) { t.translation.y += speed; }
    if keyboard.pressed(KeyCode::KeyS) { t.translation.y -= speed; }
    if keyboard.pressed(KeyCode::KeyA) { t.translation.x -= speed; }
    if keyboard.pressed(KeyCode::KeyD) { t.translation.x += speed; }
}

fn handle_worker_build(
    keyboard: Res<ButtonInput<KeyCode>>,
    selection: Res<Selection>,
    mut units: Query<&mut Unit>,
) {
    let Some(entity) = selection.selected_unit else { return };
    let Ok(mut unit) = units.get_mut(entity) else { return };
    if !unit.unit_type.is_worker() || unit.movement_left == 0 {
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyF) {
        unit.start_build(Improvement::Farm);
        info!("Worker started building Farm");
    } else if keyboard.just_pressed(KeyCode::KeyM) {
        unit.start_build(Improvement::Mine);
        info!("Worker started building Mine");
    } else if keyboard.just_pressed(KeyCode::KeyR) {
        unit.start_build(Improvement::Road);
        info!("Worker started building Road");
    }
}

fn handle_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    map: Res<MapData>,
    mut selection: ResMut<Selection>,
    mut units: Query<(Entity, &mut Unit, &mut Transform)>,
    cities: Query<&City>,
    tiles: Query<&HexTile>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut fog: ResMut<FogOfWar>,
    path_visuals: Query<Entity, With<PathVisual>>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }

    clear_path_visuals(&mut commands, &path_visuals);

    let Ok(window) = windows.get_single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok((cam, gt)) = cameras.get_single() else { return };
    let Ok(world) = cam.viewport_to_world_2d(gt, cursor) else { return };
    let clicked = map.layout.world_pos_to_hex(world);

    let mut clicked_unit = None;
    for (entity, unit, _) in &units {
        if unit.position == clicked {
            clicked_unit = Some(entity);
            break;
        }
    }

    if let Some(entity) = clicked_unit {
        selection.selected_unit = Some(entity);
        return;
    }

    if let Some(selected) = selection.selected_unit {
        if let Ok((_, mut unit, mut transform)) = units.get_mut(selected) {
            if unit.movement_left == 0 || unit.build_turns_left.is_some() {
                return;
            }

            let Some(path) = find_path(unit.position, clicked, &tiles) else {
                info!("No path to {:?}", clicked);
                return;
            };

            visualize_path(
                &mut commands,
                &mut meshes,
                &mut materials,
                &map.layout,
                &path,
            );

            let final_pos = path.last().copied().unwrap_or(unit.position);
            let mut enemy_entity = None;
            for (e, other, _) in &units {
                if other.position == final_pos && other.owner != unit.owner {
                    enemy_entity = Some((e, other.unit_type));
                    break;
                }
            }

            let spent = move_along_path(&mut unit.position, &mut unit.movement_left, &path);
            if spent > 0 {
                let pos = map.layout.hex_to_world_pos(unit.position);
                transform.translation = Vec3::new(pos.x, pos.y, 1.0);
                fog.reveal(unit.position, 2);
                info!("Moved {} hexes along path of length {}", spent, path.len());
            }

            if let Some((enemy_e, enemy_type)) = enemy_entity {
                if unit.position == final_pos {
                    let terrain = tiles.iter().find(|t| t.coord == final_pos).map(|t| t.terrain).unwrap_or(Terrain::Plains);
                    let wins = resolve_combat(unit.unit_type, enemy_type, terrain);
                    log_combat(unit.unit_type, enemy_type, terrain, wins);
                    if wins {
                        commands.entity(enemy_e).despawn();
                    } else {
                        commands.entity(selected).despawn();
                        selection.selected_unit = None;
                    }
                }
            }

            if unit.unit_type == UnitType::Settler && enemy_entity.is_none() {
                let city_exists = cities.iter().any(|c| c.position == unit.position);
                let land = tiles.iter().any(|t| t.coord == unit.position && t.terrain != Terrain::Water);
                if !city_exists && land && unit.position != path[0] {
                    spawn_city(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &map.layout,
                        unit.position,
                        format!("City {}", cities.iter().count() + 1),
                        1,
                    );
                    commands.entity(selected).despawn();
                    selection.selected_unit = None;
                    fog.reveal(unit.position, 2);
                }
            }
        }
    }
}

fn update_unit_positions(
    map: Res<MapData>,
    mut units: Query<(&Unit, &mut Transform)>,
) {
    for (unit, mut transform) in &mut units {
        let pos = map.layout.hex_to_world_pos(unit.position);
        transform.translation = Vec3::new(pos.x, pos.y, 1.0);
    }
}

fn has_strategic_resource(tiles: &Query<&HexTile>, required: Resource) -> bool {
    tiles.iter().any(|t| t.resource == Some(required))
}

fn process_turn(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut tech: ResMut<TechState>,
    mut cities: Query<(&mut City, &mut CityBuildings, &mut Happiness)>,
    mut units: Query<&mut Unit>,
    mut tiles: Query<&mut HexTile>,
    mut civs: ResMut<Civilizations>,
    mut fog: ResMut<FogOfWar>,
    mut diplomacy: ResMut<Diplomacy>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    map: Res<MapData>,
) {
    if !keyboard.just_pressed(KeyCode::Space) { return; }
    if state.victory.is_some() { return; }

    state.turn += 1;

    let prod_mod = state.government.production_modifier();
    let sci_mod = state.government.science_modifier();

    let mut total_science = 0;
    let mut disorder_count = 0;
    let mut player_cities = 0;

    for (mut city, mut buildings, mut happiness) in &mut cities {
        if city.owner == 0 {
            player_cities += 1;
        }

        if city.process_food(&buildings) {
            info!("{} grew to population {}", city.name, city.population);
        }

        let mut luxury_bonus = 0u32;
        for tile in tiles.iter() {
            if tile.coord.distance_to(city.position) <= 2 {
                if let Some(res) = tile.resource {
                    if res.is_luxury() {
                        luxury_bonus += res.happiness_bonus();
                    }
                }
            }
        }

        happiness.update(city.population, luxury_bonus);
        if happiness.in_disorder() {
            disorder_count += 1;
            info!("{} is in civil disorder!", city.name);
        }

        let prod = ((city.production_per_turn(&buildings) as f32) * prod_mod) as u32;
        city.production_progress += prod;

        if let Some(completed) = city.process_production(&buildings) {
            match completed {
                ProductionItem::Unit(ut) => {
                    let can_build = match ut.required_resource() {
                        Some(req) => has_strategic_resource(&tiles, req),
                        None => true,
                    };
                    if can_build {
                        spawn_unit_for_owner(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &map.layout,
                            city.position,
                            ut,
                            city.owner,
                        );
                        info!("City {} completed {:?}", city.name, ut);
                    } else {
                        info!("Cannot build {:?}: missing strategic resource", ut);
                        city.production_progress += ut.cost() / 2;
                    }
                }
                ProductionItem::Building(b) => {
                    buildings.buildings.push(b.clone());
                    info!("City {} completed building {}", city.name, b.name());
                }
            }
        }

        total_science += city.science_per_turn(&buildings);
    }

    tech.science_per_turn = ((total_science as f32) * sci_mod) as u32;

    for mut unit in &mut units {
        if let Some(completed_imp) = unit.process_build() {
            for mut tile in &mut tiles {
                if tile.coord == unit.position {
                    tile.add_improvement(completed_imp);
                    info!("Built {:?} at {:?}", completed_imp, unit.position);
                    break;
                }
            }
        }
        let (_, _, mv) = unit.unit_type.stats();
        if unit.build_turns_left.is_none() {
            unit.movement_left = mv;
        }
    }

    process_ai_turn(
        &mut civs,
        units.reborrow(),
        cities.reborrow().map(|(c, _, _)| c),
        tiles.reborrow(),
        commands.reborrow(),
        meshes.reborrow(),
        materials.reborrow(),
        &map.layout,
    );

    let unit_positions: Vec<_> = units.iter().map(|u| u.position).collect();
    let city_positions: Vec<_> = cities.iter().map(|(c, _, _)| c.position).collect();
    fog.update_visibility(&unit_positions, &city_positions);

    diplomacy.change_attitude(0, 1, -1);

    if let Some(completed) = tech.process_turn() {
        info!("Researched: {}", completed.name());
        if completed == Tech::Monarchy {
            state.government = Government::Monarchy;
        }
        if completed == Tech::Democracy {
            state.government = Government::Democracy;
        }
    }

    if player_cities >= 5 {
        state.victory = Some("Domination Victory!".into());
        info!("Victory: Domination");
    } else if tech.researched.contains(&Tech::Democracy) {
        state.victory = Some("Science / Culture Victory!".into());
        info!("Victory: Democracy researched");
    }

    if disorder_count > 0 {
        info!("{} cities in disorder", disorder_count);
    }
}

fn update_ui(
    state: Res<GameState>,
    tech: Res<TechState>,
    selection: Res<Selection>,
    units: Query<&Unit>,
    cities: Query<(&City, &Happiness)>,
    civs: Res<Civilizations>,
    diplomacy: Res<Diplomacy>,
    mut q: Query<&mut Text, With<Name>>,
) {
    let Ok(mut text) = q.get_single_mut() else { return };
    let current = tech.current.map(|t| t.name()).unwrap_or("None");
    let progress = if let Some(t) = tech.current {
        format!("{}/{}", tech.progress, t.cost())
    } else {
        "-".to_string()
    };

    let sel = if let Some(e) = selection.selected_unit {
        if let Ok(u) = units.get(e) {
            let build = if let Some(imp) = u.current_build {
                format!(" building {:?} ({})", imp, u.build_turns_left.unwrap_or(0))
            } else {
                "".to_string()
            };
            format!("{:?} (moves: {}){}", u.unit_type, u.movement_left, build)
        } else {
            "None".to_string()
        }
    } else {
        "None".to_string()
    };

    let total_pop: u32 = cities.iter().map(|(c, _)| c.population).sum();
    let disorder = cities.iter().filter(|(_, h)| h.in_disorder()).count();
    let attitude = diplomacy.get_attitude(0, 1);

    let victory_text = state.victory.as_deref().unwrap_or("");

    text.0 = format!(
        "Turn: {} | Gov: {} | Pop: {} | Disorder: {}\nTech: {} ({})\nSelected: {}\nCivs: {} | Attitude vs AI: {}\n{}\nSpace = End Turn | F/M/R = Farm/Mine/Road",
        state.turn,
        state.government.name(),
        total_pop,
        disorder,
        current,
        progress,
        sel,
        civs.players.len(),
        attitude,
        victory_text
    );
}
