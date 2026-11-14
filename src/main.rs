use bevy::prelude::*;
use hexx::{shapes, *};
use rand::Rng;

mod city;
mod unit;
mod terrain;

use city::*;
use unit::*;
use terrain::*;

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
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameState { turn: 1, gold: 0, science: 0 })
        .add_systems(Startup, setup)
        .add_systems(Update, (camera_pan, tile_selection, process_turn))
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

    // Starting city + settler
    let start = Hex::ZERO;
    spawn_city(&mut commands, start, "Capital".into(), 1);
    spawn_unit(&mut commands, start, UnitType::Settler);

    commands.insert_resource(MapData { layout });

    // UI
    commands.spawn((
        Text::new("Turn: 1 | Gold: 0 | Science: 0\nSpace: End Turn | WASD: Pan"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
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

fn tile_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    map: Res<MapData>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }
    let Ok(window) = windows.get_single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok((cam, gt)) = cameras.get_single() else { return };
    let Ok(world) = cam.viewport_to_world_2d(gt, cursor) else { return };
    let hex = map.layout.world_pos_to_hex(world);
    info!("Clicked hex {:?}", hex);
}

fn process_turn(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut cities: Query<&mut City>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        state.turn += 1;
        for mut city in &mut cities {
            city.production_progress += city.production_per_turn();
            // TODO: finish production, growth, etc.
        }
        info!("Turn {}", state.turn);
    }
}
