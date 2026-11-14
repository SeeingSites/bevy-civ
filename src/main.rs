use bevy::prelude::*;
use hexx::{shapes, *};
use rand::Rng;

mod city;
mod unit;
mod terrain;
mod tech;
mod government;
mod combat;

use city::*;
use unit::*;
use terrain::*;
use tech::*;
use government::*;
use combat::*;

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
        .add_systems(Startup, setup)
        .add_systems(Update, (camera_pan, handle_click, process_turn, update_ui, update_unit_positions))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
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
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }

    let Ok(window) = windows.get_single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok((cam, gt)) = cameras.get_single() else { return };
    let Ok(world) = cam.viewport_to_world_2d(gt, cursor) else { return };
    let clicked = map.layout.world_pos_to_hex(world);

    // Check if clicked on a unit
    let mut clicked_unit = None;
    for (entity, unit, _) in &units {
        if unit.position == clicked {
            clicked_unit = Some(entity);
            break;
        }
    }

    if let Some(entity) = clicked_unit {
        selection.selected_unit = Some(entity);
        info!("Selected unit");
        return;
    }

    // Move selected unit
    if let Some(selected) = selection.selected_unit {
        if let Ok((_, mut unit, mut transform)) = units.get_mut(selected) {
            if unit.movement_left > 0 && unit.position.distance_to(clicked) <= 1 {
                // Check for city founding with settler
                if unit.unit_type == UnitType::Settler {
                    let city_exists = cities.iter().any(|c| c.position == clicked);
                    if !city_exists {
                        spawn_city(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &map.layout,
                            clicked,
                            format!("City {}", cities.iter().count() + 1),
                            1,
                        );
                        commands.entity(selected).despawn();
                        selection.selected_unit = None;
                        info!("Founded city");
                        return;
                    }
                }

                unit.position = clicked;
                unit.movement_left -= 1;
                let pos = map.layout.hex_to_world_pos(clicked);
                transform.translation = Vec3::new(pos.x, pos.y, 1.0);
                info!("Moved unit to {:?}", clicked);
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
    mut cities: Query<&mut City>,
    mut units: Query<&mut Unit>,
) {
    if !keyboard.just_pressed(KeyCode::Space) { return; }

    state.turn += 1;

    let prod_mod = state.government.production_modifier();
    let sci_mod = state.government.science_modifier();

    for mut city in &mut cities {
        let prod = (city.production_per_turn() as f32 * prod_mod) as u32;
        city.production_progress += prod;
    }

    // Reset unit movement
    for mut unit in &mut units {
        let (_, _, mv) = unit.unit_type.stats();
        unit.movement_left = mv;
    }

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

    text.0 = format!(
        "Turn: {} | Gov: {}\nTech: {} ({})\nSelected: {}\nSpace = End Turn | Click unit then destination",
        state.turn,
        state.government.name(),
        current,
        progress,
        sel
    );
}
