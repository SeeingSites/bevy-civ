//! Camera control (drag, wheel, pinch, keys) and translation of clicks, taps and keys into game events.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use bevy_civ::terrain::Improvement;

use super::map::{Hovered, HEX_SIZE};
use super::sfx::SoundSettings;
use super::ui::{Modal, UiState};
use crate::{Action, Focus, MapClick, MapData};

const DRAG_THRESHOLD: f32 = 7.0;

/// Tracks a press so a drag pans the map and a clean release counts as a click.
#[derive(Resource, Default)]
pub struct Pointer {
    press: Option<Vec2>,
    last: Vec2,
    dragging: bool,
    pinch: Option<f32>,
    /// Where the camera is gliding to after a unit was auto-selected.
    glide: Option<Vec2>,
}

#[allow(clippy::too_many_arguments)]
pub fn pointer_input(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    (mut wheel, mut focus): (EventReader<MouseWheel>, EventReader<Focus>),
    windows: Query<&Window>,
    interactions: Query<&Interaction>,
    map: Res<MapData>,
    mut pointer: ResMut<Pointer>,
    mut hovered: ResMut<Hovered>,
    mut clicks: EventWriter<MapClick>,
    mut camera: Query<(&Camera, &GlobalTransform, &mut Transform, &mut OrthographicProjection), With<Camera2d>>,
    ui: Res<UiState>,
    scrollables: Query<&bevy::ui::RelativeCursorPosition, With<super::layout::Scrollable>>,
) {
    let Ok(window) = windows.get_single() else { return };
    // The wheel over a panel or window scrolls it (see `ui::scroll_panels`), not the map.
    let over_scrollable = scrollables.iter().any(|c| c.mouse_over());
    // Shift (or the "move together" switch) orders the whole stack.
    let group = ui.group_move || keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let Ok((cam, cam_global, mut cam_transform, mut projection)) = camera.get_single_mut() else { return };
    let over_ui = interactions.iter().any(|i| *i != Interaction::None);
    let to_hex = |screen: Vec2| cam.viewport_to_world_2d(cam_global, screen).ok().map(|w| map.layout.world_pos_to_hex(w));
    let scale = projection.scale;
    let mut pan = Vec2::ZERO;
    let mut zoom = 1.0f32;

    // --- touch -----------------------------------------------------------------------------
    let active: Vec<Vec2> = touches.iter().map(|t| t.position()).collect();
    let touching = !active.is_empty() || touches.iter_just_released().next().is_some();
    if active.len() >= 2 {
        let spread = active[0].distance(active[1]);
        if let Some(previous) = pointer.pinch {
            if spread > 1.0 {
                zoom *= previous / spread;
            }
        }
        pointer.pinch = Some(spread);
        pointer.dragging = true;
    } else {
        pointer.pinch = None;
        if let Some(touch) = touches.iter_just_pressed().next() {
            if !over_ui {
                pointer.press = Some(touch.position());
                pointer.last = touch.position();
                pointer.dragging = false;
            }
        } else if let (Some(start), Some(pos)) = (pointer.press, active.first().copied()) {
            if pos.distance(start) > DRAG_THRESHOLD {
                pointer.dragging = true;
            }
            if pointer.dragging {
                pan += pos - pointer.last;
            }
            pointer.last = pos;
        }
        if let Some(touch) = touches.iter_just_released().next() {
            if pointer.press.take().is_some() && !pointer.dragging {
                if let Some(hex) = to_hex(touch.position()) {
                    clicks.send(MapClick { hex, secondary: false, group });
                }
            }
            pointer.dragging = false;
        }
    }

    // --- mouse -----------------------------------------------------------------------------
    if !touching {
        if let Some(cursor) = window.cursor_position() {
            if mouse.just_pressed(MouseButton::Left) && !over_ui {
                pointer.press = Some(cursor);
                pointer.dragging = false;
            }
            if mouse.just_pressed(MouseButton::Right) && !over_ui {
                if let Some(hex) = to_hex(cursor) {
                    clicks.send(MapClick { hex, secondary: true, group });
                }
            }
            let dragging_left = mouse.pressed(MouseButton::Left) && pointer.press.is_some();
            if dragging_left && pointer.press.is_some_and(|start| cursor.distance(start) > DRAG_THRESHOLD) {
                pointer.dragging = true;
            }
            if (dragging_left && pointer.dragging) || mouse.pressed(MouseButton::Middle) {
                pan += cursor - pointer.last;
            }
            if mouse.just_released(MouseButton::Left) {
                if pointer.press.take().is_some() && !pointer.dragging {
                    if let Some(hex) = to_hex(cursor) {
                        clicks.send(MapClick { hex, secondary: false, group });
                    }
                }
                pointer.dragging = false;
            }
            pointer.last = cursor;
            hovered.0 = if over_ui { None } else { to_hex(cursor).filter(|h| map.cells.contains(h)) };
        } else {
            hovered.0 = None;
        }
        for event in wheel.read() {
            let lines = match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y / 60.0,
            };
            if !over_ui && !over_scrollable {
                zoom *= 1.0 - lines.clamp(-3.0, 3.0) * 0.1;
            }
        }
    } else {
        wheel.clear();
        hovered.0 = None;
    }

    // --- keyboard pan ----------------------------------------------------------------------
    let mut key_pan = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) { key_pan.y += 1.0; }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) { key_pan.y -= 1.0; }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) { key_pan.x -= 1.0; }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) { key_pan.x += 1.0; }

    // Glide to a newly selected unit unless the player is steering the camera.
    if let Some(Focus(hex)) = focus.read().last() {
        pointer.glide = Some(map.layout.hex_to_world_pos(*hex));
    }
    if pan != Vec2::ZERO || key_pan != Vec2::ZERO {
        pointer.glide = None;
    }
    if let Some(target) = pointer.glide {
        let here = cam_transform.translation.truncate();
        let step = (target - here) * (1.0 - (-9.0 * time.delta_secs()).exp());
        cam_transform.translation += step.extend(0.0);
        if here.distance(target) < 2.0 {
            pointer.glide = None;
        }
    }

    projection.scale = (scale * zoom).clamp(0.45, 2.4);
    cam_transform.translation.x += -pan.x * scale + key_pan.x * 620.0 * scale * time.delta_secs();
    cam_transform.translation.y += pan.y * scale + key_pan.y * 620.0 * scale * time.delta_secs();
    let margin = HEX_SIZE * 3.0;
    cam_transform.translation.x = cam_transform.translation.x.clamp(map.min.x - margin, map.max.x + margin);
    cam_transform.translation.y = cam_transform.translation.y.clamp(map.min.y - margin, map.max.y + margin);
}

pub fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut actions: EventWriter<Action>,
    mut ui: ResMut<UiState>,
    mut dirty: ResMut<crate::Dirty>,
    mut sound: ResMut<SoundSettings>,
) {
    let bindings = [
        (KeyCode::Space, Action::SkipOrEnd),
        (KeyCode::KeyN, Action::NextUnit),
        (KeyCode::Tab, Action::CycleStack),
        (KeyCode::KeyU, Action::UseUnit),
        (KeyCode::KeyK, Action::Automate),
        (KeyCode::KeyH, Action::Fortify),
        (KeyCode::Enter, Action::EndTurn),
        (KeyCode::KeyF, Action::Build(Improvement::Farm)),
        (KeyCode::KeyM, Action::Build(Improvement::Mine)),
        (KeyCode::KeyR, Action::Build(Improvement::Road)),
        (KeyCode::KeyX, Action::Transform),
        (KeyCode::KeyO, Action::Build(Improvement::Fortress)),
        (KeyCode::KeyP, Action::Pillage),
        (KeyCode::KeyZ, Action::Sentry),
        (KeyCode::KeyJ, Action::JoinCity),
        (KeyCode::KeyI, Action::Board),
        (KeyCode::BracketRight, Action::CycleCity(true)),
        (KeyCode::BracketLeft, Action::CycleCity(false)),
        (KeyCode::Backspace, Action::Undo),
        (KeyCode::KeyB, Action::FoundCity),
        (KeyCode::KeyT, Action::TradeRoute),
        (KeyCode::F2, Action::Save),
    ];
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight, KeyCode::SuperLeft, KeyCode::SuperRight]);
    // Held, or let go in this same frame (a quick Shift+Tab can arrive all at once).
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) || keys.any_just_released([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    for (key, action) in bindings {
        if keys.just_pressed(key) {
            // Ctrl+Z (Cmd+Z) undoes the last move; Z alone is sentry. Shift+Tab goes back through a stack.
            let action = if key == KeyCode::KeyZ && ctrl {
                Action::Undo
            } else if key == KeyCode::Tab && shift {
                Action::CycleStackBack
            } else {
                action
            };
            actions.send(action);
        }
    }
    let mut toggle = |modal: Modal, ui: &mut UiState| {
        ui.modal = if ui.modal == Some(modal) { None } else { Some(modal) };
        dirty.ui = true;
    };
    if keys.just_pressed(KeyCode::KeyQ) {
        toggle(Modal::Science, &mut ui);
    }
    if keys.just_pressed(KeyCode::KeyE) {
        toggle(Modal::Diplomacy, &mut ui);
    }
    if keys.just_pressed(KeyCode::KeyG) {
        toggle(Modal::Trade, &mut ui);
    }
    if keys.just_pressed(KeyCode::KeyL) {
        ui.messages_page = 0;
        toggle(Modal::Messages, &mut ui);
    }
    if keys.just_pressed(KeyCode::KeyY) {
        toggle(Modal::Empire, &mut ui);
    }
    if keys.just_pressed(KeyCode::F1) {
        toggle(Modal::Help, &mut ui);
    }
    if keys.just_pressed(KeyCode::KeyV) && SoundSettings::AVAILABLE {
        sound.toggle();
    }
    if keys.just_pressed(KeyCode::Escape) {
        if ui.modal.take().is_none() {
            actions.send(Action::Deselect);
        }
        dirty.ui = true;
    }
}
