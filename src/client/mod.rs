//! Presentation layer: sprite atlases, map and actor views, camera input and the interface.

use bevy::prelude::*;

pub mod actors;
pub mod input;
pub mod map;
pub mod sfx;
pub mod ui;

use crate::GameLogic;

/// Handles to every texture atlas and font, loaded once at startup.
#[derive(Resource)]
pub struct Art {
    pub terrain: Handle<Image>,
    pub terrain_layout: Handle<TextureAtlasLayout>,
    pub overlays: Handle<Image>,
    pub overlays_layout: Handle<TextureAtlasLayout>,
    pub resources: Handle<Image>,
    pub resources_layout: Handle<TextureAtlasLayout>,
    pub units: Handle<Image>,
    pub units_layout: Handle<TextureAtlasLayout>,
    pub cities: Handle<Image>,
    pub cities_layout: Handle<TextureAtlasLayout>,
    pub flag: Handle<Image>,
    pub flag_layout: Handle<TextureAtlasLayout>,
    pub burst: Handle<Image>,
    pub burst_layout: Handle<TextureAtlasLayout>,
    pub icons: Handle<Image>,
    pub icons_layout: Handle<TextureAtlasLayout>,
    pub panel: Handle<Image>,
    pub parchment: Handle<Image>,
    pub button: Handle<Image>,
    pub leaders: Handle<Image>,
    pub leaders_layout: Handle<TextureAtlasLayout>,
    pub font_title: Handle<Font>,
    pub font_body: Handle<Font>,
    pub font_bold: Handle<Font>,
}

impl Art {
    pub fn atlas(&self, layout: &Handle<TextureAtlasLayout>, index: usize) -> TextureAtlas {
        TextureAtlas { layout: layout.clone(), index }
    }
}

/// Frame indices in `overlays.png`.
pub mod overlay {
    pub const SHORE: usize = 0;
    pub const BORDER: usize = 1;
    pub const FARM: usize = 2;
    pub const ROAD: usize = 3;
    pub const ROAD_HUB: usize = 4;
    pub const HOVER: usize = 5;
    pub const SELECT: usize = 6;
    pub const UNKNOWN: usize = 7;
    pub const MINE: usize = 8;
    pub const DOT: usize = 9;
    pub const BASE: usize = 10;
}

/// Frame indices in `icons.png`.
pub mod icon {
    pub const GOLD: usize = 0;
    pub const SCIENCE: usize = 1;
    pub const FOOD: usize = 2;
    pub const PRODUCTION: usize = 3;
    pub const HAPPY: usize = 4;
    pub const POPULATION: usize = 5;
    pub const TURN: usize = 6;
    pub const ATTACK: usize = 7;
    pub const DEFENSE: usize = 8;
    pub const MOVES: usize = 9;
    pub const CULTURE: usize = 10;
}

fn load_art(mut commands: Commands, assets: Res<AssetServer>, mut layouts: ResMut<Assets<TextureAtlasLayout>>) {
    let mut grid = |w: u32, h: u32, cols: u32, rows: u32| layouts.add(TextureAtlasLayout::from_grid(UVec2::new(w, h), cols, rows, None, None));
    commands.insert_resource(Art {
        terrain: assets.load("sprites/terrain.png"),
        terrain_layout: grid(192, 224, 8, 7),
        overlays: assets.load("sprites/overlays.png"),
        overlays_layout: grid(192, 168, 12, 1),
        resources: assets.load("sprites/resources.png"),
        resources_layout: grid(96, 96, 18, 1),
        units: assets.load("sprites/units.png"),
        units_layout: grid(128, 128, 4, 27),
        cities: assets.load("sprites/cities.png"),
        cities_layout: grid(192, 192, 4, 1),
        flag: assets.load("sprites/flag.png"),
        flag_layout: grid(64, 64, 4, 1),
        burst: assets.load("sprites/burst.png"),
        burst_layout: grid(128, 128, 6, 1),
        icons: assets.load("sprites/icons.png"),
        icons_layout: grid(48, 48, 12, 1),
        panel: assets.load("sprites/panel.png"),
        parchment: assets.load("sprites/parchment.png"),
        button: assets.load("sprites/button.png"),
        leaders: assets.load("sprites/leaders.png"),
        leaders_layout: grid(128, 128, 17, 1),
        font_title: assets.load("fonts/Cinzel.ttf"),
        font_body: assets.load("fonts/AlegreyaSans-Medium.ttf"),
        font_bold: assets.load("fonts/AlegreyaSans-Bold.ttf"),
    });
}

/// Banner colour of a civilization.
pub fn owner_color(owner: u32) -> Color {
    match owner {
        0 => Color::srgb(0.22, 0.52, 0.95),
        1 => Color::srgb(0.86, 0.2, 0.18),
        2 => Color::srgb(0.95, 0.76, 0.16),
        3 => Color::srgb(0.25, 0.75, 0.4),
        4 => Color::srgb(0.62, 0.36, 0.86),
        5 => Color::srgb(0.95, 0.5, 0.15),
        6 => Color::srgb(0.2, 0.78, 0.8),
        7 => Color::srgb(0.93, 0.45, 0.72),
        8 => Color::srgb(0.55, 0.75, 0.2),
        9 => Color::srgb(0.6, 0.42, 0.25),
        10 => Color::srgb(0.85, 0.85, 0.9),
        11 => Color::srgb(0.1, 0.4, 0.45),
        _ => Color::srgb(0.32, 0.3, 0.3),
    }
}

/// "Your", "Roman", "Barbarian": how an owner's units and cities are described.
pub fn owner_name(owner: u32, civs: &bevy_civ::ai::Civilizations) -> String {
    match owner {
        crate::PLAYER => "Your".into(),
        crate::BARBARIANS => "Barbarian".into(),
        id => civs.players.iter().find(|c| c.id == id).map(|c| c.adjective.clone()).unwrap_or_else(|| "Unknown".into()),
    }
}

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        let playing = in_state(crate::AppState::Playing);
        app.add_plugins(sfx::SfxPlugin)
            .init_resource::<map::Hovered>()
            .init_resource::<input::Pointer>()
            .init_resource::<ui::UiState>()
            .init_resource::<ui::MenuState>()
            .add_systems(PreStartup, load_art)
            .add_systems(Startup, (map::setup_map_view, ui::setup_ui, ui::setup_menu))
            .add_systems(Update, (ui::button_actions, ui::rebuild_menu, ui::button_visuals, ui::fit_ui_scale, ui::update_sound_button).chain().before(GameLogic))
            .add_systems(
                Update,
                (input::pointer_input, input::keyboard_input, ui::minimap_click)
                    .chain()
                    .after(ui::button_actions)
                    .before(GameLogic)
                    .run_if(playing.clone()),
            )
            .add_systems(
                Update,
                (
                    map::attach_tile_views,
                    map::refresh_map,
                    map::animate_water,
                    map::update_markers,
                    map::update_path_dots,
                    actors::attach_unit_views,
                    actors::attach_city_views,
                    actors::animate_units,
                    actors::update_city_views,
                    actors::animate_flags,
                    actors::spawn_bursts,
                    actors::animate_bursts,
                    ui::rebuild_panels,
                    ui::update_top_bar,
                    ui::update_tile_info,
                    ui::update_log,
                    ui::update_minimap,
                    ui::update_turn_buttons,
                )
                    .chain()
                    .after(GameLogic)
                    .run_if(playing),
            );
    }
}
