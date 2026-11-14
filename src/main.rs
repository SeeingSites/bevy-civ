use bevy::prelude::*;
use hexx::{shapes, *};
use rand::Rng;

const HEX_SIZE: f32 = 30.0;
const MAP_RADIUS: u32 = 8;

#[derive(Component)]
struct HexTile {
    coord: Hex,
    terrain: Terrain,
}

#[derive(Clone, Copy, PartialEq)]
enum Terrain {
    Water,
    Grass,
    Desert,
    Mountain,
}

impl Terrain {
    fn color(&self) -> Color {
        match self {
            Terrain::Water => Color::srgb(0.2, 0.4, 0.8),
            Terrain::Grass => Color::srgb(0.2, 0.7, 0.3),
            Terrain::Desert => Color::srgb(0.9, 0.8, 0.4),
            Terrain::Mountain => Color::srgb(0.5, 0.5, 0.5),
        }
    }
}

#[derive(Resource)]
struct MapData {
    layout: HexLayout,
    selected: Option<Hex>,
}

#[derive(Resource)]
struct GameState {
    turn: u32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameState { turn: 1 })
        .add_systems(Startup, setup)
        .add_systems(Update, (camera_pan, tile_selection, update_ui))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let layout = HexLayout::flat().with_hex_size(HEX_SIZE);
    let mesh = meshes.add(hexagonal_plane(&layout));

    let mut rng = rand::thread_rng();

    for hex in shapes::hexagon(Hex::ZERO, MAP_RADIUS) {
        let terrain = match rng.gen_range(0..10) {
            0..=2 => Terrain::Water,
            3..=6 => Terrain::Grass,
            7..=8 => Terrain::Desert,
            _ => Terrain::Mountain,
        };

        let pos = layout.hex_to_world_pos(hex);
        commands.spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(materials.add(terrain.color())),
            Transform::from_xyz(pos.x, pos.y, 0.0),
            HexTile { coord: hex, terrain },
        ));
    }

    commands.insert_resource(MapData {
        layout,
        selected: None,
    });

    // UI text
    commands.spawn((
        TextBundle::from_section(
            "Turn: 1\nClick a tile to select\nWASD to pan",
            TextStyle {
                font_size: 20.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        Name::new("ui_text"),
    ));
}

fn hexagonal_plane(hex_layout: &HexLayout) -> Mesh {
    let mesh_info = PlaneMeshBuilder::new(hex_layout)
        .facing(Vec3::Z)
        .with_scale(Vec3::splat(0.95))
        .center_aligned()
        .build();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh_info.vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, mesh_info.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, mesh_info.uvs)
    .with_inserted_indices(Indices::U16(mesh_info.indices))
}

fn camera_pan(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
) {
    let mut camera = query.single_mut();
    let speed = 300.0 * time.delta_seconds();
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        camera.translation.y += speed;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        camera.translation.y -= speed;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        camera.translation.x -= speed;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        camera.translation.x += speed;
    }
}

fn tile_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut map: ResMut<MapData>,
    tiles: Query<(&HexTile, &Transform)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    tile_materials: Query<&MeshMaterial2d<ColorMaterial>, With<HexTile>>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let window = windows.single();
    let Some(cursor) = window.cursor_position() else { return; };
    let (camera, cam_transform) = cameras.single();
    let Some(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor) else { return; };

    let hex = map.layout.world_pos_to_hex(world_pos);

    // Reset previous selection color (simplified)
    // For MVP, just log and update selected
    map.selected = Some(hex);

    info!("Selected hex: {:?}", hex);
}

fn update_ui(
    state: Res<GameState>,
    mut query: Query<&mut Text, With<Name>>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        text.sections[0].value = format!("Turn: {}\nClick a tile to select\nWASD to pan", state.turn);
    }
}
