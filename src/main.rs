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
        })
        .insert_resource(TechState::default())
        .insert_resource(Selection::default())
        .insert_resource(Civilizations::default())
        .insert_resource(FogOfWar::default())
        .insert_resource(Diplomacy::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (camera_pan, handle_click, process_turn, update_ui, update_unit_positions))
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
        let pos = layout.hex_to_world_pos(hex);

        commands.spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(materials.add(terrain.color())),
            Transform::from_xyz(pos.x, pos.y, 0.0),
            HexTile { coord: hex, terrain },
        ));
    }

    let start = Hex::ZERO;
    spawn_city(&mut commands, &mut meshes, &mut materials, &layout, start, "Capital".into(), 1);
    spawn_unit(&mut commands, &mut meshes, &mut materials, &layout, start, UnitType::Settler);

    let ai1 = Hex::new(6, -3);
    spawn_unit(&mut commands, &mut meshes, &mut materials, &layout, ai1, UnitType::Warrior);

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

    // Clear previous path visualization
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
            if unit.movement_left == 0 {
                return;
            }

            let Some(path) = find_path(unit.position, clicked, &tiles) else {
                info!("No path to {:?}", clicked);
                return;
            };

            // Visualize the full computed path
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

fn process_turn(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut tech: ResMut<TechState>,
    mut cities: Query<(Entity, &mut City)>,
    mut units: Query<&mut Unit>,
    tiles: Query<&HexTile>,
    civs: Res<Civilizations>,
    mut fog: ResMut<FogOfWar>,
    mut diplomacy: ResMut<Diplomacy>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    map: Res<MapData>,
) {
    if !keyboard.just_pressed(KeyCode::Space) { return; }

    state.turn += 1;

    let prod_mod = state.government.production_modifier();
    let sci_mod = state.government.science_modifier();

    for (entity, mut city) in &mut cities {
        let prod = (city.production_per_turn() as f32 * prod_mod) as u32;
        city.production_progress += prod;

        if let Some(completed) = city.process_production() {
            match completed {
                ProductionItem::Unit(ut) => {
                    spawn_unit(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &map.layout,
                        city.position,
                        ut,
                    );
                    info!("City {} completed {:?}", city.name, ut);
                }
                ProductionItem::Building(b) => {
                    info!("City {} completed building {}", city.name, b.name());
                }
            }
        }
    }

    for mut unit in &mut units {
        let (_, _, mv) = unit.unit_type.stats();
        unit.movement_left = mv;
    }

    process_ai_turn(&civs, units.reborrow(), cities.reborrow().map(|(_, c)| c), tiles);

    let unit_positions: Vec<_> = units.iter().map(|u| u.position).collect();
    let city_positions: Vec<_> = cities.iter().map(|(_, c)| c.position).collect();
    fog.update_visibility(&unit_positions, &city_positions);

    diplomacy.change_attitude(0, 1, -1);

    tech.science_per_turn = ((2 + state.turn / 10) as f32 * sci_mod) as u32;
    if let Some(completed) = tech.process_turn() {
        info!("Researched: {}", completed.name());
        if completed == Tech::Monarchy {
            state.government = Government::Monarchy;
        }
        if completed == Tech::Democracy {
            state.government = Government::Democracy;
        }
    }
}

fn update_ui(
    state: Res<GameState>,
    tech: Res<TechState>,
    selection: Res<Selection>,
    units: Query<&Unit>,
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
            format!("{:?} (moves: {})", u.unit_type, u.movement_left)
        } else {
            "None".to_string()
        }
    } else {
        "None".to_string()
    };

    let attitude = diplomacy.get_attitude(0, 1);

    text.0 = format!(
        "Turn: {} | Gov: {}\nTech: {} ({})\nSelected: {}\nCivs: {} | Attitude vs AI: {}\nSpace = End Turn | Click unit then destination (path shown in yellow)",
        state.turn,
        state.government.name(),
        current,
        progress,
        sel,
        civs.players.len(),
        attitude
    );
}
