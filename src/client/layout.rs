//! Screen layout that adapts to the device: the part of the interface that any game built on this
//! client can reuse. It knows nothing of cities or units - only screens, panels and where they go.
//!
//! - **The browser canvas sized by hand.** On a high-density screen (most phones, Retina laptops)
//!   the engine's own "fit the canvas to its parent" mode and the browser disagree about the device
//!   pixel ratio: the canvas keeps CSS-pixel size while the engine divides it by the ratio again, so
//!   the interface is laid out for a screen half as wide as it is and touches land in the wrong
//!   place. Instead, [`fit_canvas`] reads the page's size and pixel ratio itself and gives the window
//!   exactly that - logical pixels are CSS pixels, physical pixels are the screen's - whenever either
//!   changes (a phone turned, a browser resized or zoomed).
//! - **A screen class** ([`Screen`]), recomputed whenever the window changes size (rotating a phone
//!   included): its size in interface pixels, whether it is portrait, and whether it is compact
//!   (phone-sized). Layout systems read it instead of the window.
//! - **Docked panels** ([`Dock`]): a panel says where it belongs (a corner, an edge, the centre) and
//!   the layout system positions and sizes it for the screen class, so panels never collide.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};

/// Width (in interface pixels) below which the interface uses its compact (phone) arrangement.
pub const COMPACT_WIDTH: f32 = 700.0;
/// The narrowest the interface lays itself out for; narrower screens scale it down to fit.
pub const MIN_LAYOUT_WIDTH: f32 = 380.0;

/// The screen as the interface sees it, in interface pixels (after `UiScale`).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub width: f32,
    pub height: f32,
    pub portrait: bool,
    /// Phone-sized: panels stack and slim down.
    pub compact: bool,
    /// How much the interface is scaled (1 on a big screen).
    pub scale: f32,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            width: 1280.0,
            height: 720.0,
            portrait: false,
            compact: false,
            scale: 1.0,
        }
    }
}

impl Screen {
    /// The screen class for a window of this size (in CSS pixels).
    pub fn for_window(width: f32, height: f32) -> Self {
        // Scale down only what is too narrow to lay out at all; otherwise keep text readable.
        let short = width.min(height);
        let scale =
            (width / MIN_LAYOUT_WIDTH).min(1.0).max(0.7) * if short < 420.0 { 0.9 } else { 1.0 };
        let (w, h) = (width / scale, height / scale);
        Self {
            width: w,
            height: h,
            portrait: h > w,
            compact: w < COMPACT_WIDTH || h < 480.0,
            scale,
        }
    }

    /// The most a panel may take of the width, leaving a margin either side.
    pub fn usable_width(&self, margin: f32) -> f32 {
        (self.width - 2.0 * margin).max(0.0)
    }
}

/// Where a panel belongs on the screen. The layout system keeps docked panels clear of each other.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dock {
    /// Across the top.
    Top,
    /// The bottom-left corner (and, on a compact screen, the left part of the bottom strip).
    BottomLeft,
    /// The bottom-right corner (on a compact screen, the right part of the bottom strip).
    BottomRight,
    /// A tall panel on the right, between the top bar and the bottom-right corner.
    Right,
    /// Centred windows.
    Centre,
}

/// The size a docked panel wants on a big screen; the layout system shrinks it to fit.
#[derive(Component, Clone, Copy, Debug)]
pub struct DockSize {
    pub width: f32,
}

/// Measured heights of the panels the others must keep clear of (set by `measure_docks`).
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct DockExtents {
    pub top: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}

/// Margin around docked panels.
pub const MARGIN: f32 = 8.0;

/// A panel or window whose contents scroll when they do not fit: with the mouse wheel, or by
/// dragging a finger over it. Needs `RelativeCursorPosition` and `ScrollPosition` too.
#[derive(Component)]
pub struct Scrollable;

/// A finger dragging a panel: where it started and was last frame (None while no drag is under way).
#[derive(Resource, Default)]
pub struct TouchScroll {
    start: Option<Vec2>,
    last: Option<Vec2>,
    /// The drag moved far enough to be a scroll rather than a tap.
    pub scrolling: bool,
}

/// How far a finger moves before a touch is a scroll, not a tap.
pub const TAP_SLOP: f32 = 10.0;

/// A button was activated: clicked with the mouse (on press), or tapped with a finger (on release,
/// and only if the finger did not scroll - so dragging a list of buttons scrolls it).
#[derive(Event, Clone, Copy, Debug)]
pub struct Activated(pub Entity);

/// The layout's input systems; the game's button handling runs after them.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutInput;

/// Turns presses into activations (see [`Activated`]).
pub fn activate_buttons(
    buttons: Query<(Entity, &Interaction), (Changed<Interaction>, With<Button>)>,
    touches: Res<Touches>,
    drag: Res<TouchScroll>,
    mut pending: Local<Option<Entity>>,
    mut activated: EventWriter<Activated>,
) {
    let touch = touches.iter().next().is_some()
        || touches.iter_just_released().next().is_some()
        || touches.iter_just_pressed().next().is_some();
    for (entity, interaction) in &buttons {
        if *interaction == Interaction::Pressed {
            if touch {
                *pending = Some(entity);
            } else {
                activated.send(Activated(entity));
            }
        }
    }
    if touches.iter_just_released().next().is_some() {
        if let Some(entity) = pending.take() {
            if !drag.scrolling {
                activated.send(Activated(entity));
            }
        }
    }
    if touches.iter_just_canceled().next().is_some() {
        *pending = None;
    }
}

/// Scrolls the innermost scrollable panel the pointer (or finger) is over that can move that way:
/// up and down for panels that scroll vertically, sideways for those that scroll horizontally
/// (Shift + wheel, a sideways trackpad swipe, or a finger).
pub fn scroll_panels(
    mut wheel: EventReader<bevy::input::mouse::MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    mut drag: ResMut<TouchScroll>,
    mut panels: Query<
        (
            Entity,
            &bevy::ui::RelativeCursorPosition,
            &mut ScrollPosition,
            &Node,
            &ComputedNode,
        ),
        With<Scrollable>,
    >,
) {
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let (mut delta, mut delta_x) = (0.0f32, 0.0f32);
    for event in wheel.read() {
        let unit = match event.unit {
            bevy::input::mouse::MouseScrollUnit::Line => 40.0,
            bevy::input::mouse::MouseScrollUnit::Pixel => 1.0,
        };
        if shift {
            delta_x += event.y * unit;
        } else {
            delta += event.y * unit;
            delta_x += event.x * unit;
        }
    }
    // One finger dragging: the content follows it.
    let fingers: Vec<Vec2> = touches.iter().map(|t| t.position()).collect();
    if fingers.len() == 1 {
        let at = fingers[0];
        let start = *drag.start.get_or_insert(at);
        if (at - start).length() > TAP_SLOP {
            drag.scrolling = true;
        }
        if let (Some(last), true) = (drag.last, drag.scrolling) {
            delta += at.y - last.y;
            delta_x += at.x - last.x;
        }
        drag.last = Some(at);
    } else {
        drag.last = None;
        drag.start = None;
        if fingers.is_empty() {
            drag.scrolling = false;
        }
    }
    if delta == 0.0 && delta_x == 0.0 {
        return;
    }
    // The innermost (smallest) panel under the pointer that scrolls each way.
    let mut vertical: Option<(f32, Entity)> = None;
    let mut horizontal: Option<(f32, Entity)> = None;
    for (entity, cursor, _, node, computed) in &panels {
        if !cursor.mouse_over() {
            continue;
        }
        let area = computed.size().x * computed.size().y;
        if node.overflow.y == OverflowAxis::Scroll && vertical.is_none_or(|(a, _)| area < a) {
            vertical = Some((area, entity));
        }
        if node.overflow.x == OverflowAxis::Scroll && horizontal.is_none_or(|(a, _)| area < a) {
            horizontal = Some((area, entity));
        }
    }
    for (me, cursor, mut scroll, _, _) in &mut panels {
        if !cursor.mouse_over() {
            continue;
        }
        if delta != 0.0 && vertical.is_some_and(|(_, e)| e == me) {
            scroll.offset_y = (scroll.offset_y - delta).max(0.0);
        }
        if delta_x != 0.0 && horizontal.is_some_and(|(_, e)| e == me) {
            scroll.offset_x = (scroll.offset_x - delta_x).max(0.0);
        }
    }
}

/// Whether the pointer (or finger) is over a scrollable panel, so the map should not pan or zoom.
pub fn over_scrollable(
    panels: &Query<&bevy::ui::RelativeCursorPosition, With<Scrollable>>,
) -> bool {
    panels.iter().any(|c| c.mouse_over())
}

/// In the browser, gives the window the page's size (in CSS pixels) and the screen's pixel ratio
/// (see the module notes). Does nothing elsewhere.
pub fn fit_canvas(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(page) = web_sys::window() else {
            return;
        };
        let width = page
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let height = page
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let ratio = page.device_pixel_ratio() as f32;
        if width < 1.0 || height < 1.0 {
            return;
        }
        for mut window in &mut windows {
            let res = &window.resolution;
            let same = (res.width() - width).abs() < 0.5
                && (res.height() - height).abs() < 0.5
                && res.scale_factor_override() == Some(ratio);
            if !same {
                window.resolution.set_scale_factor_override(Some(ratio));
                window.resolution.set(width, height);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &mut windows;
}

/// Recomputes the screen class when the window changes size (a phone turned, a browser resized).
pub fn classify_screen(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut resized: EventReader<WindowResized>,
    mut screen: ResMut<Screen>,
    mut ui_scale: ResMut<UiScale>,
    mut first: Local<bool>,
) {
    let changed = resized.read().count() > 0 || !*first;
    let Ok(window) = windows.get_single() else {
        return;
    };
    // The window may be resized without an event reaching us first (see `fit_canvas`), so also
    // compare sizes.
    let size_moved = (window.width() / screen.scale - screen.width).abs() > 1.0
        || (window.height() / screen.scale - screen.height).abs() > 1.0;
    if !changed && !size_moved {
        return;
    }
    *first = true;
    let next = Screen::for_window(window.width(), window.height());
    if *screen != next {
        *screen = next;
    }
    if (ui_scale.0 - next.scale).abs() > 0.001 {
        ui_scale.0 = next.scale;
    }
}

/// Measures the docked panels others must keep clear of.
pub fn measure_docks(
    docks: Query<(&Dock, &ComputedNode, &Node)>,
    mut extents: ResMut<DockExtents>,
) {
    let mut next = DockExtents::default();
    for (dock, computed, node) in &docks {
        if node.display == Display::None {
            continue;
        }
        let h = computed.size().y * computed.inverse_scale_factor();
        match dock {
            Dock::Top => next.top = next.top.max(h),
            Dock::BottomLeft => next.bottom_left = next.bottom_left.max(h),
            Dock::BottomRight => next.bottom_right = next.bottom_right.max(h),
            _ => {}
        }
    }
    if (extents.top - next.top).abs() > 0.5
        || (extents.bottom_left - next.bottom_left).abs() > 0.5
        || (extents.bottom_right - next.bottom_right).abs() > 0.5
    {
        *extents = next;
    }
}

/// Places and sizes the docked panels for the screen class:
/// - big screens: the bottom corners keep their natural widths; the right panel runs from under the
///   top bar to above the bottom-right corner;
/// - compact screens: the bottom corners share the bottom strip (the left one takes what the right
///   one leaves), and the right panel becomes a full-width sheet over the bottom strip, below the
///   top bar.
pub fn place_docks(
    screen: Res<Screen>,
    extents: Res<DockExtents>,
    mut docks: Query<(&Dock, Option<&DockSize>, &mut Node)>,
) {
    let s = *screen;
    let right_width = docks
        .iter()
        .find(|(d, _, _)| **d == Dock::BottomRight)
        .and_then(|(_, size, _)| size.map(|z| z.width))
        .unwrap_or(200.0);
    for (dock, size, mut node) in &mut docks {
        let want = size.map(|z| z.width);
        let set = |node: &mut Node, field: fn(&mut Node) -> &mut Val, value: Val| {
            let slot = field(node);
            if *slot != value {
                *slot = value;
            }
        };
        match dock {
            Dock::Top => {
                set(&mut node, |n| &mut n.left, Val::Px(0.0));
                set(&mut node, |n| &mut n.right, Val::Px(0.0));
                set(&mut node, |n| &mut n.top, Val::Px(0.0));
            }
            Dock::BottomRight => {
                set(&mut node, |n| &mut n.right, Val::Px(MARGIN));
                set(&mut node, |n| &mut n.bottom, Val::Px(MARGIN));
                let width = if s.compact {
                    want.unwrap_or(200.0).min(s.usable_width(MARGIN) * 0.38)
                } else {
                    want.unwrap_or(200.0)
                };
                set(&mut node, |n| &mut n.width, Val::Px(width));
            }
            Dock::BottomLeft => {
                set(&mut node, |n| &mut n.left, Val::Px(MARGIN));
                set(&mut node, |n| &mut n.bottom, Val::Px(MARGIN));
                // Whatever the bottom-right panel leaves of the width.
                let room = s.usable_width(MARGIN)
                    - if s.compact {
                        right_width.min(s.usable_width(MARGIN) * 0.38)
                    } else {
                        right_width
                    }
                    - MARGIN;
                let width = want.unwrap_or(360.0).min(room.max(160.0));
                set(&mut node, |n| &mut n.width, Val::Px(width));
                set(
                    &mut node,
                    |n| &mut n.max_height,
                    Val::Px((s.height - extents.top - 2.0 * MARGIN).max(120.0)),
                );
            }
            Dock::Right => {
                let top = extents.top + MARGIN;
                if s.compact {
                    // A sheet across the screen, from under the top bar to the bottom.
                    set(&mut node, |n| &mut n.left, Val::Px(MARGIN));
                    set(&mut node, |n| &mut n.right, Val::Px(MARGIN));
                    set(&mut node, |n| &mut n.width, Val::Auto);
                    set(&mut node, |n| &mut n.max_width, Val::Percent(100.0));
                    set(&mut node, |n| &mut n.top, Val::Px(top));
                    set(&mut node, |n| &mut n.bottom, Val::Px(MARGIN));
                } else {
                    set(&mut node, |n| &mut n.left, Val::Auto);
                    set(&mut node, |n| &mut n.right, Val::Px(MARGIN));
                    set(
                        &mut node,
                        |n| &mut n.width,
                        Val::Px(want.unwrap_or(430.0).min(s.usable_width(MARGIN))),
                    );
                    set(&mut node, |n| &mut n.top, Val::Px(top));
                    set(
                        &mut node,
                        |n| &mut n.bottom,
                        Val::Px(extents.bottom_right + 2.0 * MARGIN),
                    );
                }
            }
            Dock::Centre => {
                set(&mut node, |n| &mut n.top, Val::Px(extents.top + MARGIN));
                set(&mut node, |n| &mut n.bottom, Val::Px(MARGIN));
            }
        }
    }
}

/// The layout systems, in order (the game's own interface code runs before them).
pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Screen>()
            .init_resource::<DockExtents>()
            .init_resource::<TouchScroll>()
            .add_event::<Activated>()
            .add_systems(
                Update,
                (activate_buttons, scroll_panels)
                    .chain()
                    .in_set(LayoutInput),
            )
            .add_systems(PreUpdate, fit_canvas)
            .add_systems(
                PostUpdate,
                (classify_screen, measure_docks, place_docks)
                    .chain()
                    .before(bevy::ui::UiSystem::Layout),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phones_are_compact_both_ways_and_desktops_are_not() {
        let portrait = Screen::for_window(390.0, 844.0);
        assert!(portrait.compact && portrait.portrait);
        assert!(
            portrait.width >= MIN_LAYOUT_WIDTH - 1.0,
            "laid out at least {MIN_LAYOUT_WIDTH} wide, got {}",
            portrait.width
        );
        let landscape = Screen::for_window(844.0, 390.0);
        assert!(
            landscape.compact && !landscape.portrait,
            "a short landscape phone is compact too"
        );
        let desktop = Screen::for_window(1280.0, 800.0);
        assert!(!desktop.compact && desktop.scale == 1.0);
        let tablet = Screen::for_window(820.0, 1180.0);
        assert!(!tablet.compact && tablet.portrait);
    }
}
