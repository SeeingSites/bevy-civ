//! The story of a finished game: the map replayed turn by turn - each people's lands and cities
//! growing, shrinking and changing hands - and every people's score over the whole game. Both are
//! painted into images that the replay window (`ui.rs`, [`Modal::Replay`]) shows; this module paints
//! them and moves the replay along while it plays.

use bevy::prelude::*;

use super::ui::{Modal, UiState};
use crate::{History, MapData};

/// Size of the replayed map and of the score graph, in pixels.
pub const MAP_SIZE: UVec2 = UVec2::new(560, 340);
pub const GRAPH_SIZE: UVec2 = UVec2::new(560, 200);

/// Seconds each turn stays on screen while the replay plays.
const TURN_SECONDS: f32 = 0.12;

/// Where the replay stands.
#[derive(Resource, Default)]
pub struct ReplayView {
    pub turn: u32,
    pub playing: bool,
    elapsed: f32,
}

impl ReplayView {
    /// Moves by `turns` (backwards when negative), within the recorded game.
    pub fn step(&mut self, turns: i32, history: &History) {
        let (first, last) = history.span();
        self.turn = (self.turn as i32 + turns).clamp(first.min(last) as i32, last as i32) as u32;
    }
}

/// The images the replay window shows.
#[derive(Resource)]
pub struct ReplayImages {
    pub map: Handle<Image>,
    pub graph: Handle<Image>,
}

/// The label that names the turn being shown.
#[derive(Component)]
pub struct ReplayTurnText;

pub fn setup_replay(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let map = images.add(super::paint::blank(MAP_SIZE.x, MAP_SIZE.y));
    let graph = images.add(super::paint::blank(GRAPH_SIZE.x, GRAPH_SIZE.y));
    commands.insert_resource(ReplayImages { map, graph });
    commands.init_resource::<ReplayView>();
}

fn rgba(colour: Color) -> [u8; 4] {
    let c = colour.to_srgba();
    [
        (c.red * 255.0) as u8,
        (c.green * 255.0) as u8,
        (c.blue * 255.0) as u8,
        255,
    ]
}

/// Plays the replay, and repaints the map (and, on opening, the score graph) when needed.
#[allow(clippy::too_many_arguments)]
pub fn run_replay(
    time: Res<Time>,
    ui: Res<UiState>,
    history: Option<Res<History>>,
    map: Option<Res<MapData>>,
    state: Option<Res<crate::GameState>>,
    images_handles: Option<Res<ReplayImages>>,
    mut view: ResMut<ReplayView>,
    mut images: ResMut<Assets<Image>>,
    mut label: Query<&mut Text, With<ReplayTurnText>>,
    mut painted: Local<Option<u32>>,
) {
    let (Some(history), Some(map), Some(handles)) = (history, map, images_handles) else {
        return;
    };
    if ui.modal != Some(Modal::Replay) {
        *painted = None;
        return;
    }
    let (first, last) = history.span();
    if last == 0 {
        return;
    }
    // Opening: start from the beginning, and draw the score graph once.
    if painted.is_none() {
        if view.turn < first || view.turn > last {
            view.turn = first;
        }
        if let Some(image) = images.get_mut(&handles.graph) {
            paint_graph(&mut image.data, &history);
        }
    }
    if view.playing {
        view.elapsed += time.delta_secs();
        while view.elapsed >= TURN_SECONDS {
            view.elapsed -= TURN_SECONDS;
            if view.turn >= last {
                view.playing = false;
                break;
            }
            view.turn += 1;
        }
    }
    if *painted == Some(view.turn) {
        return;
    }
    *painted = Some(view.turn);
    if let Some(image) = images.get_mut(&handles.map) {
        paint_map(&mut image.data, &map, &history, view.turn);
    }
    if let Ok(mut text) = label.get_single_mut() {
        let year = state
            .as_ref()
            .map(|s| {
                bevy_civ::theme::year(s.start_year + (view.turn as i32 - 1) * s.years_per_turn)
            })
            .unwrap_or_default();
        text.0 = format!("Turn {} of {last} ({year})", view.turn);
    }
}

/// The map at the end of `turn`: sea and unclaimed land in muted tones, each people's lands in its
/// colour, its cities as bright dots.
fn paint_map(data: &mut [u8], map: &MapData, history: &History, turn: u32) {
    data.fill(0);
    let (borders, cities) = history.at(turn);
    let span = (map.max - map.min).max(Vec2::splat(1.0));
    let margin = 8.0;
    let scale = ((MAP_SIZE.x as f32 - 2.0 * margin) / span.x)
        .min((MAP_SIZE.y as f32 - 2.0 * margin) / span.y);
    let offset = Vec2::new(
        (MAP_SIZE.x as f32 - span.x * scale) / 2.0,
        (MAP_SIZE.y as f32 - span.y * scale) / 2.0,
    );
    // The map's y grows upwards; the image's downwards.
    let place = |hex: hexx::Hex| {
        let world = map.layout.hex_to_world_pos(hex);
        Vec2::new(
            offset.x + (world.x - map.min.x) * scale,
            offset.y + (map.max.y - world.y) * scale,
        )
    };
    let tile = (map.layout.scale.x * 1.8 * scale).max(2.0);
    for hex in &map.cells {
        let colour = match borders.get(hex) {
            Some(owner) => {
                let c = rgba(super::owner_color(*owner));
                // Land in the owner's colour, darkened a little so cities stand out.
                [
                    (c[0] as f32 * 0.75) as u8,
                    (c[1] as f32 * 0.75) as u8,
                    (c[2] as f32 * 0.75) as u8,
                    255,
                ]
            }
            None if map.water.contains(hex) => [28, 52, 84, 255],
            None => [92, 84, 64, 255],
        };
        let at = place(*hex);
        super::paint::line(data, MAP_SIZE.x, at, at, tile, colour);
    }
    for (hex, owner) in &cities {
        let at = place(*hex);
        super::paint::line(
            data,
            MAP_SIZE.x,
            at,
            at,
            (tile * 1.6).max(4.0),
            [20, 16, 12, 255],
        );
        super::paint::line(
            data,
            MAP_SIZE.x,
            at,
            at,
            (tile * 1.1).max(3.0),
            rgba(super::owner_color(*owner)),
        );
    }
}

/// Every people's score over the game, one line each in its colour.
fn paint_graph(data: &mut [u8], history: &History) {
    data.fill(0);
    let (w, h) = (GRAPH_SIZE.x as f32, GRAPH_SIZE.y as f32);
    let (first, last) = history.span();
    let top = history
        .scores
        .iter()
        .map(|(_, _, s)| *s)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let x = |turn: u32| {
        6.0 + (turn.saturating_sub(first)) as f32 / (last.saturating_sub(first)).max(1) as f32
            * (w - 12.0)
    };
    let y = |score: u32| h - 6.0 - score as f32 / top * (h - 12.0);
    // Faint guides at a quarter, a half and three quarters of the best score.
    for k in 1..4 {
        let gy = h - 6.0 - k as f32 / 4.0 * (h - 12.0);
        super::paint::line(
            data,
            GRAPH_SIZE.x,
            Vec2::new(6.0, gy),
            Vec2::new(w - 6.0, gy),
            1.0,
            [120, 100, 75, 90],
        );
    }
    let mut owners: Vec<u32> = history.scores.iter().map(|(_, c, _)| *c).collect();
    owners.sort_unstable();
    owners.dedup();
    for owner in owners {
        let colour = rgba(super::owner_color(owner));
        let points: Vec<Vec2> = history
            .scores
            .iter()
            .filter(|(_, c, _)| *c == owner)
            .map(|(t, _, s)| Vec2::new(x(*t), y(*s)))
            .collect();
        let thickness = if owner == crate::PLAYER { 3.0 } else { 2.0 };
        for pair in points.windows(2) {
            super::paint::line(data, GRAPH_SIZE.x, pair[0], pair[1], thickness, colour);
        }
    }
}
