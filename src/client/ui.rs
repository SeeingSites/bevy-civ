//! The interface: status bar, unit and city panels, advisors, minimap and message log.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::image::ImageSampler;
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::ui::widget::NodeImageMode;
use bevy::ui::{FocusPolicy, RelativeCursorPosition};

use bevy_civ::ai::Civilizations;
use bevy_civ::building::{Building, CityBuildings, WorldWonders};
use bevy_civ::great::{self, GreatPeople};
use bevy_civ::city::{City, ProductionItem};
use bevy_civ::diplomacy::{Diplomacy, Happiness, Treaty};
use bevy_civ::fog::FogOfWar;
use bevy_civ::tech::{Tech, TechState};
use bevy_civ::terrain::{HexTile, Improvement, Terrain};
use bevy_civ::unit::{Unit, UnitType};

use super::actors::unit_row;
use super::map::Hovered;
use super::sfx::{Sfx, SoundSettings};
use super::{icon, owner_color, owner_name, Art};
use crate::foreign::{self, DiploAct, Foreign, OfferKind, TradeAsk};
use crate::supply::Supplies;
use bevy_civ::terrain::{Resource as Good, ResourceKind};
use crate::scenario::{self, NewGame, Scenario};
use crate::{Action, AppState, Dirty, GameLog, GameState, MapData, Selection, BARBARIANS, PLAYER};

const CREAM: Color = Color::srgb(0.94, 0.9, 0.81);
const GOLD: Color = Color::srgb(0.95, 0.83, 0.52);
const MUTED: Color = Color::srgb(0.7, 0.68, 0.62);
const INK: Color = Color::srgb(0.16, 0.11, 0.07);
const INK_SOFT: Color = Color::srgb(0.36, 0.27, 0.18);
const BRONZE: Color = Color::srgb(0.86, 0.68, 0.36);
const GREEN: Color = Color::srgb(0.56, 0.8, 0.42);
const MM: u32 = 176;
const RED: Color = Color::srgb(0.9, 0.45, 0.38);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modal {
    Science,
    Diplomacy,
    Trade,
}

#[derive(Resource, Default)]
pub struct UiState {
    pub modal: Option<Modal>,
    /// The civilization shown in the foreign advisor window.
    pub diplo_target: Option<u32>,
    /// The page of the city panel being shown (kept from city to city).
    pub city_tab: CityTab,
}

/// Pages of the city panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CityTab {
    #[default]
    Units,
    Buildings,
    Wonders,
    Overview,
}

impl CityTab {
    const ALL: [CityTab; 4] = [CityTab::Units, CityTab::Buildings, CityTab::Wonders, CityTab::Overview];

    fn name(self) -> &'static str {
        match self {
            CityTab::Units => "Units",
            CityTab::Buildings => "Buildings",
            CityTab::Wonders => "Wonders",
            CityTab::Overview => "Overview",
        }
    }
}

/// What a building adds to its city, e.g. "+2 food, +1 gold".
fn building_effect(building: &Building) -> String {
    [
        (building.food_bonus(), "food"),
        (building.production_bonus(), "shields"),
        (building.science_bonus(), "science"),
        (building.gold_bonus(), "gold"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, what)| format!("+{n} {what}"))
    .collect::<Vec<_>>()
    .join(", ")
}

/// What the start menu is showing.
#[derive(Resource, Default)]
pub struct MenuState {
    pub catalog: Vec<Scenario>,
    pub scenario: Option<usize>,
}

#[derive(Clone)]
pub enum Command {
    Game(Action),
    Toggle(Modal),
    CloseModal,
    SelectCiv(u32),
    PickScenario(usize),
    PickCiv(usize),
    BackToScenarios,
    QuitToMenu,
    ToggleSound,
    CityTab(CityTab),
}

#[derive(Component)]
pub struct ButtonCommand(Command);
#[derive(Component)]
pub struct ButtonTint(Color);
#[derive(Component, Clone, Copy)]
pub enum StatText {
    Title,
    Turn,
    Gold,
    Science,
    Population,
    Government,
    Score,
}
#[derive(Component)]
pub struct UnitPanel;
#[derive(Component)]
pub struct CityPanel;
#[derive(Component)]
pub struct ModalRoot;
#[derive(Component)]
pub struct MenuRoot;
#[derive(Component)]
pub struct NextUnitLabel;
#[derive(Component)]
pub struct EndTurnButton;
#[derive(Component)]
pub struct TileInfoText;
#[derive(Component)]
pub struct LogText;
#[derive(Component)]
pub struct VictoryText;
#[derive(Component)]
pub struct MinimapNode;
#[derive(Resource)]
pub struct MinimapImage(Handle<Image>);

// ---- building blocks ---------------------------------------------------------------------------

fn sliced(border: f32) -> NodeImageMode {
    NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::square(border),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    })
}

/// A framed panel that also stops clicks from reaching the map.
fn panel(image: &Handle<Image>, node: Node) -> impl Bundle {
    (node, ImageNode { image: image.clone(), image_mode: sliced(22.0), ..default() }, Interaction::None, FocusPolicy::Block)
}

fn label(font: &Handle<Font>, text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (Text::new(text), TextFont { font: font.clone(), font_size: size, ..default() }, TextColor(color))
}

fn icon_node(art: &Art, index: usize, size: f32) -> impl Bundle {
    (
        Node { width: Val::Px(size), height: Val::Px(size), ..default() },
        ImageNode { image: art.icons.clone(), texture_atlas: Some(art.atlas(&art.icons_layout, index)), ..default() },
    )
}

fn row(gap: f32) -> Node {
    Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(gap), row_gap: Val::Px(gap), flex_wrap: FlexWrap::Wrap, ..default() }
}

fn column(gap: f32) -> Node {
    Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(gap), ..default() }
}

fn button(parent: &mut ChildBuilder, art: &Art, text: impl Into<String>, command: Command, tint: Color, size: f32) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(size * 0.8), Val::Px(size * 0.42)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode { image: art.button.clone(), image_mode: sliced(14.0), color: tint, ..default() },
            ButtonTint(tint),
            ButtonCommand(command),
        ))
        .with_children(|b| {
            b.spawn(label(&art.font_bold, text, size, INK));
        });
}

/// Moves left, with the thirds of a move left over from road travel ("1⅔").
fn moves_left(unit: &Unit) -> String {
    let fraction = match unit.road_credit {
        1 => "⅓",
        2 => "⅔",
        _ => "",
    };
    match (unit.movement_left, fraction) {
        (0, f) if !f.is_empty() => f.to_string(),
        (n, f) => format!("{n}{f}"),
    }
}

/// Label of the in-game sound switch.
#[derive(Component)]
pub struct SoundLabel;

fn sound_label(on: bool) -> &'static str {
    if on { "Sound: On (V)" } else { "Sound: Off (V)" }
}

fn sound_button(parent: &mut ChildBuilder, art: &Art) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(14.0 * 0.8), Val::Px(14.0 * 0.42)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode { image: art.button.clone(), image_mode: sliced(14.0), color: BRONZE, ..default() },
            ButtonTint(BRONZE),
            ButtonCommand(Command::ToggleSound),
        ))
        .with_children(|b| {
            b.spawn((label(&art.font_bold, sound_label(true), 14.0, INK), SoundLabel));
        });
}

/// Keeps the sound switch's label in step with the setting (it can also change from the keyboard).
pub fn update_sound_button(settings: Res<SoundSettings>, mut labels: Query<&mut Text, With<SoundLabel>>, added: Query<(), Added<SoundLabel>>) {
    if !settings.is_changed() && added.is_empty() {
        return;
    }
    for mut text in &mut labels {
        text.0 = sound_label(settings.enabled).into();
    }
}

/// The wonders that already stand, and who holds them.
fn wonders_of_the_world(p: &mut ChildBuilder, art: &Art, wonders: &WorldWonders, civs: &Civilizations) {
    if wonders.built.is_empty() {
        return;
    }
    let mut built: Vec<_> = wonders.built.iter().collect();
    built.sort_by_key(|(w, _)| w.name());
    p.spawn(label(&art.font_title, "Wonders of the world", 14.0, GOLD));
    for (wonder, (owner, city)) in built {
        p.spawn(label(&art.font_body, format!("{}: {} ({}) - {}", wonder.name(), city, owner_name(*owner, civs), wonder.wonder_effect()), 13.0, owner_color(*owner).mix(&Color::WHITE, 0.55)));
    }
}

/// A third-width button in the city panel: the name, and under it the details and build time.
fn production_button(parent: &mut ChildBuilder, art: &Art, name: &str, detail: String, command: Command, tint: Color) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(32.0),
                padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // A flat face (no bevel) keeps the small second line readable.
            BackgroundColor(tint),
            BorderColor(Color::srgba(0.1, 0.06, 0.02, 0.8)),
            BorderRadius::all(Val::Px(5.0)),
            ButtonTint(tint),
            ButtonCommand(command),
        ))
        .with_children(|b| {
            b.spawn(label(&art.font_bold, name, 13.0, INK));
            b.spawn(label(&art.font_body, detail, 11.0, INK));
        });
}

fn bar(parent: &mut ChildBuilder, fraction: f32, color: Color) {
    parent
        .spawn((
            Node { width: Val::Percent(100.0), height: Val::Px(12.0), border: UiRect::all(Val::Px(1.0)), ..default() },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            BorderColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|b| {
            b.spawn((
                Node { width: Val::Percent(fraction.clamp(0.0, 1.0) * 100.0), height: Val::Percent(100.0), ..default() },
                BackgroundColor(color),
                BorderRadius::all(Val::Px(3.0)),
            ));
        });
}

fn stat(parent: &mut ChildBuilder, art: &Art, index: usize, text: impl Into<String>, color: Color) {
    parent.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(4.0), ..default() }).with_children(|s| {
        s.spawn(icon_node(art, index, 20.0));
        s.spawn(label(&art.font_bold, text, 16.0, color));
    });
}

pub fn year_label(state: &GameState) -> String {
    let year = state.start_year + (state.turn as i32 - 1) * state.years_per_turn;
    if year < 0 { format!("{} BC", -year) } else { format!("{} AD", year.max(1)) }
}

fn good_icon(art: &Art, good: Good, size: f32) -> impl Bundle {
    (
        Node { width: Val::Px(size), height: Val::Px(size), flex_shrink: 0.0, ..default() },
        ImageNode { image: art.resources.clone(), texture_atlas: Some(art.atlas(&art.resources_layout, good.index())), ..default() },
    )
}

const GRAY: Color = Color::srgb(0.62, 0.6, 0.57);

fn portrait(art: &Art, index: usize, size: f32) -> impl Bundle {
    (
        Node { width: Val::Px(size), height: Val::Px(size), flex_shrink: 0.0, ..default() },
        ImageNode { image: art.leaders.clone(), texture_atlas: Some(art.atlas(&art.leaders_layout, index.min(16))), ..default() },
    )
}

// ---- static layout -----------------------------------------------------------------------------

pub fn setup_ui(mut commands: Commands, art: Res<Art>, mut images: ResMut<Assets<Image>>) {
    // Status bar
    commands
        .spawn(panel(
            &art.panel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                min_height: Val::Px(52.0),
                padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(13.0),
                row_gap: Val::Px(4.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
        ))
        .with_children(|bar| {
            bar.spawn((label(&art.font_title, "", 20.0, GOLD), StatText::Title));
            for (index, kind) in [
                (icon::TURN, StatText::Turn),
                (icon::GOLD, StatText::Gold),
                (icon::SCIENCE, StatText::Science),
                (icon::POPULATION, StatText::Population),
                (icon::CULTURE, StatText::Score),
            ] {
                bar.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|item| {
                    item.spawn(icon_node(&art, index, 24.0));
                    item.spawn((label(&art.font_bold, "", 17.0, CREAM), kind));
                });
            }
            bar.spawn((label(&art.font_body, "", 16.0, MUTED), StatText::Government));
            bar.spawn(Node { flex_grow: 1.0, ..default() });
            button(bar, &art, "Science (Q)", Command::Toggle(Modal::Science), BRONZE, 14.0);
            button(bar, &art, "Diplomacy (E)", Command::Toggle(Modal::Diplomacy), BRONZE, 14.0);
            button(bar, &art, "Trade (G)", Command::Toggle(Modal::Trade), BRONZE, 14.0);
            if SoundSettings::AVAILABLE {
                sound_button(bar, &art);
            }
            button(bar, &art, "Save", Command::Game(Action::Save), BRONZE, 14.0);
            button(bar, &art, "Menu", Command::QuitToMenu, BRONZE, 14.0);
        });

    // Message log stacked on the unit panel (bottom left)
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(10.0),
            width: Val::Px(360.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|left| {
            left.spawn((
                Node { padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)), ..default() },
                BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.72)),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|b| {
                b.spawn((label(&art.font_body, "", 15.0, CREAM), LogText));
            });
            left.spawn((
                UnitPanel,
                panel(&art.panel, Node { padding: UiRect::all(Val::Px(18.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() }),
            ));
        });

    // Minimap + end turn (bottom right)
    let mut image = Image::new_fill(
        Extent3d { width: MM, height: MM, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[10, 9, 8, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    let minimap = images.add(image);
    commands.insert_resource(MinimapImage(minimap.clone()));
    commands
        .spawn(panel(
            &art.panel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                bottom: Val::Px(10.0),
                width: Val::Px(212.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((Node { padding: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(Color::srgb(0.42, 0.29, 0.11)))).with_children(|frame| {
                frame.spawn((
                    MinimapNode,
                    Node { width: Val::Px(176.0), height: Val::Px(176.0), ..default() },
                    ImageNode { image: minimap, image_mode: NodeImageMode::Stretch, ..default() },
                    Interaction::None,
                    RelativeCursorPosition::default(),
                ));
            });
            p.spawn((label(&art.font_body, "", 14.0, CREAM), TileInfoText, Node { height: Val::Px(38.0), ..default() }));
            // Cycle through the units that still need orders, then end the turn.
            p.spawn((
                Button,
                Node { padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)), justify_content: JustifyContent::Center, ..default() },
                ImageNode { image: art.button.clone(), image_mode: sliced(14.0), color: BRONZE, ..default() },
                ButtonTint(BRONZE),
                ButtonCommand(Command::Game(Action::NextUnit)),
            ))
            .with_children(|b| {
                b.spawn((label(&art.font_bold, "Next Unit", 16.0, INK), NextUnitLabel));
            });
            p.spawn((
                Button,
                EndTurnButton,
                Node { padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)), justify_content: JustifyContent::Center, ..default() },
                ImageNode { image: art.button.clone(), image_mode: sliced(14.0), color: BRONZE, ..default() },
                ButtonTint(BRONZE),
                ButtonCommand(Command::Game(Action::EndTurn)),
            ))
            .with_children(|b| {
                b.spawn(label(&art.font_bold, "End Turn", 19.0, INK));
            });
        });

    // City panel (right)
    commands.spawn((
        CityPanel,
        panel(
            &art.panel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(62.0),
                width: Val::Px(430.0),
                max_width: Val::Percent(90.0),
                padding: UiRect::all(Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(7.0),
                display: Display::None,
                ..default()
            },
        ),
    ));

    // Advisor windows (centre)
    commands.spawn((
        ModalRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(74.0),
            justify_content: JustifyContent::Center,
            display: Display::None,
            ..default()
        },
    ));

    // Victory banner
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Percent(34.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|p| {
            p.spawn((label(&art.font_title, "", 52.0, GOLD), VictoryText));
        });
}

// ---- dynamic panels ----------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn rebuild_panels(
    mut dirty: ResMut<Dirty>,
    selection: Res<Selection>,
    mut ui: ResMut<UiState>,
    (foreign, supplies, map): (Res<Foreign>, Res<Supplies>, Res<MapData>),
    art: Res<Art>,
    state: Res<GameState>,
    tech: Res<TechState>,
    civs: Res<Civilizations>,
    diplomacy: Res<Diplomacy>,
    (units, tiles, unit_entities, wonders, great): (Query<&Unit>, Query<&HexTile>, Query<Entity, With<Unit>>, Res<WorldWonders>, Res<GreatPeople>),
    cities: Query<(&City, &CityBuildings, &Happiness)>,
    unit_panel: Query<Entity, With<UnitPanel>>,
    mut city_panel: Query<(Entity, &mut Node), (With<CityPanel>, Without<ModalRoot>)>,
    mut modal_root: Query<(Entity, &mut Node), (With<ModalRoot>, Without<CityPanel>)>,
    mut commands: Commands,
) {
    if !(dirty.ui || selection.is_changed() || ui.is_changed() || foreign.is_changed() || supplies.is_changed()) {
        return;
    }
    dirty.ui = false;
    let art = &*art;

    // --- unit panel ---
    if let Ok(root) = unit_panel.get_single() {
        commands.entity(root).despawn_descendants().with_children(|p| {
            let Some(unit) = selection.unit.and_then(|e| units.get(e).ok()) else {
                p.spawn(label(&art.font_title, "No unit selected", 17.0, GOLD));
                p.spawn(label(
                    &art.font_body,
                    "Click a unit or city to select it; click again (or Tab) for the next unit on that tile.\nClick a tile (or right-click) to move. Space skips a unit; Enter ends the turn.",
                    14.0,
                    MUTED,
                ));
                return;
            };
            let (attack, defense, moves) = unit.unit_type.stats();
            let mine = unit.owner == PLAYER;
            p.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(14.0), ..default() }).with_children(|r| {
                r.spawn((
                    Node { width: Val::Px(80.0), height: Val::Px(80.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    ImageNode { image: art.parchment.clone(), image_mode: sliced(20.0), ..default() },
                ))
                .with_children(|frame| {
                    frame.spawn((
                        Node { width: Val::Px(68.0), height: Val::Px(68.0), ..default() },
                        ImageNode { image: art.units.clone(), texture_atlas: Some(art.atlas(&art.units_layout, unit_row(unit.unit_type) * 4)), ..default() },
                    ));
                });
                r.spawn(column(4.0)).with_children(|c| {
                    c.spawn(label(&art.font_title, unit.unit_type.name(), 20.0, GOLD));
                    let status = match (unit.current_build, unit.build_turns_left) {
                        (Some(job), Some(turns)) => format!("Building {:?} ({} turns)", job, turns),
                        _ if !mine => String::new(),
                        _ if unit.fortified => "Fortified".into(),
                        _ if !unit.can_move() => "No moves left".into(),
                        _ => "Awaiting orders".into(),
                    };
                    c.spawn(label(&art.font_body, format!("{} unit  {}", owner_name(unit.owner, &civs), status), 14.0, owner_color(unit.owner).mix(&Color::WHITE, 0.55)));
                    c.spawn(row(12.0)).with_children(|s| {
                        stat(s, art, icon::ATTACK, attack.to_string(), CREAM);
                        stat(s, art, icon::DEFENSE, defense.to_string(), CREAM);
                        stat(s, art, icon::MOVES, format!("{}/{}", moves_left(unit), moves), CREAM);
                    });
                });
            });
            if unit.unit_type.is_great_person() {
                p.spawn(label(&art.font_bold, great::ability(unit.unit_type), 14.0, GOLD));
            }
            // Several of the player's units on one tile: say so, and list them to pick from.
            if mine {
                let stack = crate::own_stack(unit.position, unit_entities.iter(), |e| units.get(e).ok().map(|u| (u.position, u.owner)));
                if stack.len() > 1 {
                    let index = selection.unit.and_then(|s| stack.iter().position(|e| *e == s)).unwrap_or(0);
                    p.spawn(label(
                        &art.font_bold,
                        format!("Unit {} of {} on this tile - Tab or click the tile again for the next:", index + 1, stack.len()),
                        14.0,
                        GOLD,
                    ));
                    p.spawn(row(6.0)).with_children(|r| {
                        for entity in &stack {
                            let Ok(other) = units.get(*entity) else { continue };
                            let state = if other.build_turns_left.is_some() {
                                " (working)"
                            } else if other.fortified {
                                " (fortified)"
                            } else if !other.can_move() {
                                " (done)"
                            } else {
                                ""
                            };
                            let tint = if selection.unit == Some(*entity) { GREEN } else { BRONZE };
                            button(r, art, format!("{}{}", other.unit_type.name(), state), Command::Game(Action::Select(*entity)), tint, 13.0);
                        }
                    });
                }
            }
            if mine {
                p.spawn(row(6.0)).with_children(|r| {
                    if unit.unit_type == UnitType::Settler {
                        button(r, art, "Found City (B)", Command::Game(Action::FoundCity), GREEN, 15.0);
                    }
                    if unit.unit_type.is_worker() && unit.build_turns_left.is_none() {
                        button(r, art, "Farm (F)", Command::Game(Action::Build(Improvement::Farm)), BRONZE, 15.0);
                        button(r, art, "Mine (M)", Command::Game(Action::Build(Improvement::Mine)), BRONZE, 15.0);
                        let rail = tiles.iter().any(|t| t.coord == unit.position && t.has_road() && !t.has_railroad()) && tech.researched.contains(&Tech::SteamPower);
                        button(r, art, if rail { "Railroad (R)" } else { "Road (R)" }, Command::Game(Action::Build(Improvement::Road)), BRONZE, 15.0);
                        button(r, art, "Clear (X)", Command::Game(Action::Transform), BRONZE, 15.0);
                    }
                    if unit.unit_type.is_great_person() && unit.unit_type != UnitType::GreatGeneral {
                        button(r, art, "Use (U)", Command::Game(Action::UseGreatPerson), GREEN, 15.0);
                    }
                    if !unit.fortified {
                        let text = if unit.unit_type.is_military() { "Fortify (H)" } else { "Sleep (H)" };
                        button(r, art, text, Command::Game(Action::Fortify), BRONZE, 15.0);
                    }
                    button(r, art, "Skip (Space)", Command::Game(Action::Skip), BRONZE, 15.0);
                    button(r, art, "Done", Command::Game(Action::Deselect), BRONZE, 15.0);
                });
            }
        });
    }

    // --- city panel ---
    if let Ok((root, mut node)) = city_panel.get_single_mut() {
        let city = selection.city.and_then(|e| cities.get(e).ok());
        node.display = if city.is_some() { Display::Flex } else { Display::None };
        let capital = cities.iter().find(|(c, _, _)| c.is_capital && c.owner == PLAYER).map(|(c, _, _)| c.position);
        commands.entity(root).despawn_descendants().with_children(|p| {
            let Some((city, buildings, happiness)) = city else { return };
            let mine = city.owner == PLAYER;
            p.spawn(Node { justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, ..default() }).with_children(|r| {
                r.spawn(label(&art.font_title, city.name.clone(), 23.0, GOLD));
                button(r, art, "Close", Command::Game(Action::Deselect), BRONZE, 13.0);
            });
            let kind = if city.is_capital { "capital" } else { "city" };
            p.spawn(label(&art.font_body, format!("{} {}  -  size {}", owner_name(city.owner, &civs), kind, city.population), 15.0, owner_color(city.owner).mix(&Color::WHITE, 0.55)));
            if !mine {
                return;
            }
            let garrison = crate::own_stack(city.position, unit_entities.iter(), |e| units.get(e).ok().map(|u| (u.position, u.owner)));
            if !garrison.is_empty() {
                let units_here = if garrison.len() == 1 { "1 of your units is".to_string() } else { format!("{} of your units are", garrison.len()) };
                p.spawn(label(&art.font_bold, format!("{units_here} here - press Tab or click the city again to pick one."), 14.0, GOLD));
            }
            let dist = capital.map(|c| city.position.distance_to(c) as u32).unwrap_or(0);
            let gov = state.government;
            let (extra_food, extra_shields, extra_gold) = supplies.yields.get(&city.position).copied().unwrap_or_default();
            let food = city.food_per_turn(buildings, gov, dist) + extra_food;
            let production = city.production_per_turn(buildings, gov, dist) + extra_shields;
            p.spawn(row(14.0)).with_children(|r| {
                stat(r, art, icon::FOOD, format!("+{food}"), CREAM);
                stat(r, art, icon::PRODUCTION, format!("+{production}"), CREAM);
                stat(r, art, icon::SCIENCE, format!("+{}", city.science_per_turn(buildings, gov, dist)), CREAM);
                stat(r, art, icon::GOLD, format!("+{}", city.gold_per_turn(buildings) + 1 + extra_gold), CREAM);
                stat(r, art, icon::HAPPY, format!("{} / {}", happiness.happy + happiness.content, happiness.unhappy), if happiness.in_disorder() { Color::srgb(1.0, 0.45, 0.4) } else { CREAM });
            });
            let grow_in = (city.food_to_grow().saturating_sub(city.food_storage) as f32 / food.max(1) as f32).ceil();
            p.spawn(label(&art.font_body, format!("Growth: {}/{} food  (about {} turns)", city.food_storage, city.food_to_grow(), grow_in), 14.0, MUTED));
            bar(p, city.food_storage as f32 / city.food_to_grow() as f32, Color::srgb(0.45, 0.78, 0.35));
            match &city.production_queue {
                Some(item) => {
                    let left = (item.cost().saturating_sub(city.production_progress) as f32 / production.max(1) as f32).ceil();
                    let when = if left <= 1.0 { "done at the end of the turn".to_string() } else { format!("about {left} turns") };
                    p.spawn(label(&art.font_bold, format!("Building: {}  {}/{}  ({when})", item.name(), city.production_progress.min(item.cost()), item.cost()), 14.0, CREAM));
                    bar(p, city.production_progress as f32 / item.cost() as f32, Color::srgb(0.92, 0.62, 0.25));
                }
                None => {
                    p.spawn(label(&art.font_bold, "Building: nothing - choose something below", 14.0, RED));
                }
            }

            // Tabs: what to build is split into units, buildings and wonders; the rest is on Overview.
            p.spawn(Node { column_gap: Val::Px(4.0), margin: UiRect::top(Val::Px(4.0)), ..default() }).with_children(|r| {
                for tab in CityTab::ALL {
                    let tint = if ui.city_tab == tab { GOLD } else { Color::srgb(0.55, 0.5, 0.42) };
                    button(r, art, tab.name(), Command::CityTab(tab), tint, 14.0);
                }
            });

            let known = |t: Option<Tech>| t.is_none_or(|t| tech.researched.contains(&t));
            let coastal = city.position.all_neighbors().iter().any(|n| map.water.contains(n));
            let turns = |item: &ProductionItem| {
                let done = if city.production_queue.as_ref() == Some(item) { city.production_progress } else { 0 };
                (item.cost().saturating_sub(done) as f32 / production.max(1) as f32).ceil() as u32
            };
            // One entry per thing the city could build: (item, detail line).
            let options: Vec<(ProductionItem, String)> = match ui.city_tab {
                CityTab::Units => UnitType::ALL
                    .into_iter()
                    .filter(|u| u.is_trainable() && known(u.required_tech()) && (u.domain() != bevy_civ::unit::Domain::Sea || coastal))
                    .map(|u| {
                        let (attack, defense, moves) = u.stats();
                        (ProductionItem::Unit(u), format!("A{attack} D{defense} M{moves}"))
                    })
                    .collect(),
                CityTab::Buildings | CityTab::Wonders => {
                    let wonder_tab = ui.city_tab == CityTab::Wonders;
                    Building::ALL
                        .into_iter()
                        .filter(|b| b.is_wonder() == wonder_tab && known(b.required_tech()) && !buildings.buildings.contains(b) && (*b != Building::Harbor || coastal))
                        .filter(|b| !wonders.is_taken(*b))
                        .map(|b| {
                            let effect = if b.is_wonder() { b.wonder_effect().to_string() } else { building_effect(&b) };
                            (ProductionItem::Building(b), effect)
                        })
                        .collect()
                }
                CityTab::Overview => Vec::new(),
            };

            if ui.city_tab == CityTab::Overview {
                let (mut wonders, mut plain): (Vec<&Building>, Vec<&Building>) = buildings.buildings.iter().partition(|b| b.is_wonder());
                plain.sort_by_key(|b| b.name());
                wonders.sort_by_key(|b| b.name());
                let list = |items: &[&Building]| if items.is_empty() { "none yet".to_string() } else { items.iter().map(|b| b.name()).collect::<Vec<_>>().join(", ") };
                p.spawn(label(&art.font_body, format!("Buildings: {}", list(&plain)), 14.0, CREAM));
                p.spawn(label(&art.font_body, format!("Wonders: {}", list(&wonders)), 14.0, CREAM));
                // Progress towards the next great person, and what this city's points lean towards.
                let earned = great::city_points(&buildings.buildings, &city.specialists, city.is_capital);
                let per_turn: u32 = earned.iter().sum();
                let kinds: Vec<String> = great::CITY_KINDS
                    .iter()
                    .zip(earned)
                    .filter(|(_, n)| *n > 0)
                    .map(|(k, n)| format!("{} +{n}", k.name().trim_start_matches("Great ")))
                    .collect();
                let pace = if per_turn == 0 { "none yet: libraries, markets, wonders and specialists earn them".to_string() } else { format!("+{per_turn} a turn ({})", kinds.join(", ")) };
                p.spawn(label(&art.font_body, format!("Great people: {}/{} points, {pace}", great.progress(city.position), great.threshold(PLAYER)), 14.0, CREAM));
                let masterpieces = great.masterpieces.get(&city.position).copied().unwrap_or(0);
                if masterpieces > 0 {
                    p.spawn(label(&art.font_body, format!("Masterpieces: {masterpieces} (+{} content)", masterpieces * 3), 14.0, CREAM));
                }
                // Resources this city can draw on through roads, harbours and trade.
                let mut goods: Vec<Good> = supplies.city.get(&city.position).map(|set| set.iter().copied().collect()).unwrap_or_default();
                goods.sort_by_key(|g| g.index());
                p.spawn(row(4.0)).with_children(|r| {
                    r.spawn(label(&art.font_body, if goods.is_empty() { "Resources: none connected" } else { "Resources:" }, 14.0, CREAM));
                    for good in &goods {
                        r.spawn(good_icon(art, *good, 24.0));
                    }
                });
                return;
            }
            if options.is_empty() {
                let none = match ui.city_tab {
                    CityTab::Wonders => "No wonder can be started here now: research unlocks more, and each is built only once in the world.",
                    _ => "Nothing new to build here yet: research unlocks more.",
                };
                p.spawn(label(&art.font_body, none, 14.0, MUTED));
                if ui.city_tab == CityTab::Wonders {
                    wonders_of_the_world(p, art, &wonders, &civs);
                }
                return;
            }
            // What can be built first, then what still lacks a resource.
            let (ready, blocked): (Vec<_>, Vec<_>) = options.into_iter().partition(|(item, _)| supplies.missing(city.position, item.required_resources()).is_empty());
            p.spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(5.0), row_gap: Val::Px(5.0), ..default() }).with_children(|grid| {
                for (item, detail) in ready.iter().chain(blocked.iter()) {
                    let missing = supplies.missing(city.position, item.required_resources());
                    let current = city.production_queue.as_ref() == Some(item);
                    let tint = if !missing.is_empty() {
                        Color::srgb(0.66, 0.64, 0.6)
                    } else if current {
                        Color::srgb(0.66, 0.9, 0.52)
                    } else {
                        Color::srgb(0.96, 0.84, 0.6)
                    };
                    let second = if missing.is_empty() {
                        let t = turns(item);
                        let when = if t == 1 { "1 turn".to_string() } else { format!("{t} turns") };
                        if detail.is_empty() { format!("{} - {when}", item.cost()) } else { format!("{detail} - {when}") }
                    } else {
                        format!("needs {}", missing.iter().map(|g| g.name()).collect::<Vec<_>>().join(", "))
                    };
                    production_button(grid, art, item.name(), second, Command::Game(Action::SetProduction(item.clone())), tint);
                }
            });
            if ui.city_tab == CityTab::Wonders {
                wonders_of_the_world(p, art, &wonders, &civs);
            }
        });
    }

    // --- advisors and messages from other leaders ---
    let rivals: Vec<&bevy_civ::ai::Civ> = civs.players.iter().filter(|c| !c.is_human && !foreign.gone.contains(&c.id)).collect();
    let known: Vec<&bevy_civ::ai::Civ> = rivals.iter().copied().filter(|c| foreign.met.contains(&c.id)).collect();
    if ui.modal == Some(Modal::Diplomacy) && !ui.diplo_target.is_some_and(|t| known.iter().any(|c| c.id == t)) {
        let first = known.first().map(|c| c.id);
        if ui.diplo_target != first {
            ui.diplo_target = first;
        }
    }
    if let Ok((root, mut node)) = modal_root.get_single_mut() {
        node.display = if ui.modal.is_some() || foreign.offer.is_some() { Display::Flex } else { Display::None };
        commands.entity(root).despawn_descendants().with_children(|p| {
            let window = |width: f32| {
                panel(
                    &art.parchment,
                    Node { width: Val::Px(width), max_width: Val::Percent(96.0), padding: UiRect::all(Val::Px(26.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() },
                )
            };
            // A waiting proposal takes priority over the advisors.
            if let Some(offer) = &foreign.offer {
                let Some(civ) = civs.players.iter().find(|c| c.id == offer.from) else { return };
                p.spawn(window(560.0)).with_children(|w| {
                    w.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(16.0), ..default() }).with_children(|r| {
                        r.spawn(portrait(art, civ.portrait, 104.0));
                        r.spawn(column(6.0)).with_children(|c| {
                            c.spawn(label(&art.font_title, format!("{} of {}", civ.leader, civ.name), 22.0, INK));
                            c.spawn(label(&art.font_bold, format!("\"{}\"", offer.text), 17.0, INK_SOFT));
                        });
                    });
                    let detail = match &offer.kind {
                        OfferKind::Tribute(amount) => format!("They demand {amount} gold. We have {}.", state.gold),
                        OfferKind::TechSwap { give, get } => format!("We would learn {} and teach them {}.", get.name(), give.name()),
                        OfferKind::JoinWar(target) => format!("Accepting means declaring war on {}.", civs.players.iter().find(|c| c.id == *target).map(|c| c.name.as_str()).unwrap_or("them")),
                        OfferKind::Peace => "Accepting ends the war.".to_string(),
                        OfferKind::BuyFromUs(good, gold) => format!("We would send them our spare {} and receive {gold} gold a turn for {} turns.", good.name(), foreign::DEAL_TURNS),
                        OfferKind::SellToUs(good, gold) => format!("We would receive {} and pay {gold} gold a turn for {} turns. We have {} gold.", good.name(), foreign::DEAL_TURNS, state.gold),
                        OfferKind::Notice => String::new(),
                    };
                    if !detail.is_empty() {
                        w.spawn(label(&art.font_body, detail, 15.0, INK));
                    }
                    w.spawn(row(8.0)).with_children(|r| {
                        if offer.kind == OfferKind::Notice {
                            button(r, art, "Very well", Command::Game(Action::AnswerOffer(true)), BRONZE, 15.0);
                        } else {
                            button(r, art, "Accept", Command::Game(Action::AnswerOffer(true)), GREEN, 15.0);
                            button(r, art, "Refuse", Command::Game(Action::AnswerOffer(false)), RED, 15.0);
                        }
                    });
                });
                return;
            }
            let Some(modal) = ui.modal else { return };
            match modal {
                Modal::Science => {
                    p.spawn(window(580.0)).with_children(|w| {
                        w.spawn(label(&art.font_title, "Science Advisor", 26.0, INK));
                        match tech.current {
                            Some(current) => {
                                let cost = tech.cost_of(current);
                                let left = (cost.saturating_sub(tech.progress) as f32 / tech.science_per_turn.max(1) as f32).ceil();
                                w.spawn(label(&art.font_bold, format!("Researching {}  -  {}/{} beakers, about {} turns", current.name(), tech.progress, cost, left), 16.0, INK));
                                bar(w, tech.progress as f32 / cost as f32, Color::srgb(0.25, 0.6, 0.9));
                            }
                            None => {
                                w.spawn(label(&art.font_bold, "Our scholars are idle. Choose a field of study.", 16.0, INK));
                            }
                        }
                        w.spawn(label(&art.font_title, "Choose research", 16.0, INK_SOFT));
                        w.spawn(row(6.0)).with_children(|r| {
                            for t in Tech::ALL.into_iter().filter(|t| tech.can_research(*t)) {
                                let tint = if tech.current == Some(t) { GREEN } else { BRONZE };
                                button(r, art, format!("{} ({})", t.name(), tech.cost_of(t)), Command::Game(Action::Research(t)), tint, 14.0);
                            }
                        });
                        let learned: Vec<&str> = Tech::ALL.iter().filter(|t| tech.researched.contains(t)).map(|t| t.name()).collect();
                        w.spawn(label(&art.font_body, format!("Known: {}", if learned.is_empty() { "nothing yet".into() } else { learned.join(", ") }), 15.0, INK_SOFT));
                        w.spawn(row(6.0)).with_children(|r| button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0));
                    });
                }
                Modal::Trade => {
                    p.spawn(window(720.0)).with_children(|w| {
                        w.spawn(label(&art.font_title, "Trade Advisor", 26.0, INK));
                        w.spawn(label(
                            &art.font_body,
                            "A resource must lie inside our borders and be joined to a city by road (or by harbours) before that city can use it. Strategic resources are needed for units and buildings; each different luxury keeps one more citizen content in every connected city.",
                            14.0,
                            INK_SOFT,
                        ));
                        let ours = supplies.civ(PLAYER);
                        let mut any = false;
                        for (kind, title) in [(ResourceKind::Strategic, "Strategic resources"), (ResourceKind::Luxury, "Luxuries")] {
                            let rows: Vec<Good> = Good::ALL
                                .into_iter()
                                .filter(|g| g.kind() == kind)
                                .filter(|g| g.reveal_tech().is_none_or(|t| tech.researched.contains(&t)))
                                .collect();
                            w.spawn(label(&art.font_title, title, 16.0, INK_SOFT));
                            w.spawn(row(10.0)).with_children(|r| {
                                for good in rows {
                                    let sources = ours.sources.get(&good).copied().unwrap_or(0);
                                    let connected = ours.connected.get(&good).copied().unwrap_or(0);
                                    let exported = ours.exports.get(&good).copied().unwrap_or(0);
                                    let status = if ours.imports.contains(&good) {
                                        "imported".to_string()
                                    } else if connected > exported {
                                        if ours.spare(good) > 0 { format!("{} connected, {} spare", connected, ours.spare(good)) } else { "connected".to_string() }
                                    } else if sources > 0 {
                                        "needs a road".to_string()
                                    } else {
                                        "none".to_string()
                                    };
                                    let color = if ours.has(good) { INK } else if sources > 0 { Color::srgb(0.7, 0.35, 0.1) } else { GRAY };
                                    any |= sources > 0;
                                    r.spawn(Node { width: Val::Px(200.0), align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|c| {
                                        c.spawn(good_icon(art, good, 30.0));
                                        c.spawn(label(&art.font_bold, format!("{}: {status}", good.name()), 14.0, color));
                                    });
                                }
                            });
                        }
                        if !any {
                            w.spawn(label(&art.font_body, "No resources lie inside our borders yet. Found cities near them.", 14.0, INK));
                        }
                        // What the lack of resources is holding back.
                        let capital = cities.iter().find(|(c, _, _)| c.owner == PLAYER && c.is_capital).or(cities.iter().find(|(c, _, _)| c.owner == PLAYER)).map(|(c, _, _)| c.position);
                        if let Some(capital) = capital {
                            let blocked: Vec<String> = UnitType::ALL
                                .into_iter()
                                .filter(|u| u.is_trainable() && u.required_tech().is_none_or(|t| tech.researched.contains(&t)))
                                .filter_map(|u| {
                                    let missing = supplies.missing(capital, u.required_resources());
                                    (!missing.is_empty()).then(|| format!("{} ({})", u.name(), missing.iter().map(|g| g.name()).collect::<Vec<_>>().join(", ")))
                                })
                                .collect();
                            if !blocked.is_empty() {
                                w.spawn(label(&art.font_body, format!("Our capital cannot yet train: {}.", blocked.join("; ")), 14.0, INK));
                            }
                        }
                        if !foreign.deals.is_empty() {
                            w.spawn(label(&art.font_title, "Deals", 16.0, INK_SOFT));
                            for deal in &foreign.deals {
                                let partner = civs.players.iter().find(|c| c.id == deal.civ).map(|c| c.name.as_str()).unwrap_or("?");
                                let text = match (deal.import, deal.export) {
                                    (Some(good), _) => format!("{} from {partner} for {} gold a turn", good.name(), -deal.gold),
                                    (_, Some(good)) => format!("{} to {partner} for {} gold a turn", good.name(), deal.gold),
                                    _ => String::new(),
                                };
                                w.spawn(label(&art.font_body, format!("{text} ({} turns left)", deal.ends.saturating_sub(state.turn)), 14.0, INK));
                            }
                        }
                        w.spawn(label(&art.font_body, "Buy and sell resources with other leaders in the foreign advisor (E).", 14.0, INK_SOFT));
                        w.spawn(row(6.0)).with_children(|r| button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0));
                    });
                }
                Modal::Diplomacy => {
                    p.spawn(window(760.0)).with_children(|w| {
                        w.spawn(label(&art.font_title, "Foreign Advisor", 26.0, INK));
                        let unknown = rivals.len() - known.len();
                        if known.is_empty() {
                            w.spawn(label(&art.font_bold, "We have not met any other people yet. Explore to find them.", 16.0, INK));
                        }
                        // Everyone we know, as a strip of faces.
                        w.spawn(row(6.0)).with_children(|r| {
                            for civ in &known {
                                let at_war = diplomacy.get_treaty(PLAYER, civ.id) == Treaty::War;
                                let tint = if ui.diplo_target == Some(civ.id) { GREEN } else if at_war { RED } else { BRONZE };
                                r.spawn((
                                    Button,
                                    Node { padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)), align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() },
                                    ImageNode { image: art.button.clone(), image_mode: sliced(14.0), color: tint, ..default() },
                                    ButtonTint(tint),
                                    ButtonCommand(Command::SelectCiv(civ.id)),
                                ))
                                .with_children(|b| {
                                    b.spawn(portrait(art, civ.portrait, 34.0));
                                    b.spawn(label(&art.font_bold, civ.name.trim_start_matches("the ").to_string(), 14.0, INK));
                                });
                            }
                        });
                        if unknown > 0 {
                            w.spawn(label(&art.font_body, format!("{unknown} other peoples are still unknown to us."), 14.0, INK_SOFT));
                        }
                        if let Some(civ) = ui.diplo_target.and_then(|t| known.iter().find(|c| c.id == t)) {
                            let treaty = diplomacy.get_treaty(PLAYER, civ.id);
                            let attitude = diplomacy.get_attitude(PLAYER, civ.id);
                            let age = foreign.treaty_age(PLAYER, civ.id, state.turn);
                            let ours = foreign.strength.get(&PLAYER).copied().unwrap_or(1.0).max(1.0);
                            let theirs = foreign.strength.get(&civ.id).copied().unwrap_or(1.0).max(1.0);
                            let power = match theirs / ours {
                                r if r > 1.8 => "far stronger than us",
                                r if r > 1.2 => "stronger than us",
                                r if r > 0.8 => "our equal in arms",
                                r if r > 0.5 => "weaker than us",
                                _ => "far weaker than us",
                            };
                            let standing = match treaty {
                                Treaty::War => format!("At war for {age} turns"),
                                Treaty::Peace => format!("Peace treaty ({age} turns)"),
                                Treaty::Alliance => format!("Allied ({age} turns)"),
                                Treaty::None => "No treaty".to_string(),
                            };
                            let pact = if foreign.pacts.contains(&civ.id) { "  -  trade agreement" } else { "" };
                            w.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(16.0), ..default() }).with_children(|r| {
                                r.spawn(portrait(art, civ.portrait, 104.0));
                                r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(5.0), flex_grow: 1.0, ..default() }).with_children(|c| {
                                    c.spawn(label(&art.font_title, format!("{} of {}", civ.leader, civ.name), 22.0, INK));
                                    c.spawn(label(&art.font_bold, format!("{} ({:+})  -  {standing}{pact}", foreign::mood(attitude), attitude), 16.0, INK));
                                    bar(c, (attitude + 100) as f32 / 200.0, if attitude < -14 { Color::srgb(0.8, 0.25, 0.2) } else if attitude < 15 { Color::srgb(0.85, 0.7, 0.25) } else { Color::srgb(0.35, 0.7, 0.3) });
                                    c.spawn(label(&art.font_body, format!("{} gold in their treasury; {power}.", civ.gold), 15.0, INK_SOFT));
                                    if let Some(words) = foreign.reply.get(&civ.id) {
                                        c.spawn(label(&art.font_bold, format!("\"{words}\""), 16.0, INK_SOFT));
                                    }
                                });
                            });
                            let feuds: Vec<&str> = known.iter().filter(|o| o.id != civ.id && diplomacy.get_treaty(civ.id, o.id) == Treaty::War).map(|o| o.name.as_str()).collect();
                            if !feuds.is_empty() {
                                w.spawn(label(&art.font_body, format!("They are at war with {}.", feuds.join(", ")), 15.0, INK_SOFT));
                            }
                            w.spawn(row(6.0)).with_children(|r| {
                                let ask = |act: DiploAct| Command::Game(Action::Diplo(civ.id, act));
                                if treaty == Treaty::War {
                                    button(r, art, "Propose peace", ask(DiploAct::Peace), GREEN, 14.0);
                                } else {
                                    if treaty != Treaty::Alliance {
                                        button(r, art, "Propose alliance", ask(DiploAct::Alliance), BRONZE, 14.0);
                                    }
                                    if treaty == Treaty::None {
                                        button(r, art, "Sign peace treaty", ask(DiploAct::Peace), BRONZE, 14.0);
                                    }
                                    if !foreign.pacts.contains(&civ.id) {
                                        button(r, art, "Trade agreement", ask(DiploAct::TradePact), BRONZE, 14.0);
                                    }
                                    button(r, art, "Exchange technology", ask(DiploAct::TechTrade), BRONZE, 14.0);
                                    button(r, art, "Declare war", ask(DiploAct::War), RED, 14.0);
                                }
                                button(r, art, format!("Gift {} gold", foreign::GIFT), ask(DiploAct::Gift), BRONZE, 14.0);
                                button(r, art, "Demand tribute", ask(DiploAct::Demand), BRONZE, 14.0);
                            });
                        }
                        if let Some(civ) = ui.diplo_target.and_then(|t| known.iter().find(|c| c.id == t)) {
                            let (ours, theirs) = (supplies.civ(PLAYER), supplies.civ(civ.id));
                            let tradeable = |g: &Good| g.kind() != ResourceKind::Bonus;
                            let for_sale: Vec<Good> = Good::ALL.into_iter().filter(tradeable).filter(|g| theirs.spare(*g) > 0 && !ours.imports.contains(g)).collect();
                            let wanted: Vec<Good> = Good::ALL.into_iter().filter(tradeable).filter(|g| ours.spare(*g) > 0 && !theirs.has(*g)).collect();
                            w.spawn(label(&art.font_title, "Resource trade", 16.0, INK_SOFT));
                            if for_sale.is_empty() && wanted.is_empty() && !foreign.deals.iter().any(|d| d.civ == civ.id) {
                                w.spawn(label(&art.font_body, "Neither side has spare resources the other lacks. A second connected source of a resource can be sold.", 14.0, INK_SOFT));
                            }
                            w.spawn(row(6.0)).with_children(|r| {
                                for good in for_sale {
                                    r.spawn(good_icon(art, good, 26.0));
                                    button(r, art, format!("Buy {} ({} gold/turn)", good.name(), foreign::price(good)), Command::Game(Action::Trade(civ.id, TradeAsk::Buy(good))), BRONZE, 13.0);
                                }
                                for good in wanted {
                                    r.spawn(good_icon(art, good, 26.0));
                                    button(r, art, format!("Sell {} (+{} gold/turn)", good.name(), foreign::price(good) - 1), Command::Game(Action::Trade(civ.id, TradeAsk::Sell(good))), GREEN, 13.0);
                                }
                            });
                            for (index, deal) in foreign.deals.iter().enumerate().filter(|(_, d)| d.civ == civ.id) {
                                w.spawn(row(8.0)).with_children(|r| {
                                    let text = match (deal.import, deal.export) {
                                        (Some(good), _) => format!("We buy {} for {} gold/turn", good.name(), -deal.gold),
                                        (_, Some(good)) => format!("We sell {} for {} gold/turn", good.name(), deal.gold),
                                        _ => String::new(),
                                    };
                                    r.spawn(label(&art.font_bold, format!("{text}  ({} turns left)", deal.ends.saturating_sub(state.turn)), 14.0, INK));
                                    button(r, art, "Cancel", Command::Game(Action::Trade(civ.id, TradeAsk::Cancel(index))), RED, 12.0);
                                });
                            }
                        }
                        w.spawn(row(6.0)).with_children(|r| {
                            button(r, art, "Link our cities by trade (T)", Command::Game(Action::TradeRoute), BRONZE, 14.0);
                            button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0);
                        });
                    });
                }
            }
        });
    }
}

pub fn update_top_bar(
    state: Res<GameState>,
    tech: Res<TechState>,
    cities: Query<&City>,
    mut stats: Query<(&StatText, &mut Text), Without<VictoryText>>,
    mut victory: Query<&mut Text, With<VictoryText>>,
) {
    let population: u32 = cities.iter().filter(|c| c.owner == PLAYER).map(|c| c.population).sum();
    for (kind, mut text) in &mut stats {
        let value = match kind {
            StatText::Title => state.title.to_uppercase(),
            StatText::Turn => format!("Turn {}  -  {}", state.turn, year_label(&state)),
            StatText::Gold => format!("{} ({:+})", state.gold, state.gold_per_turn),
            StatText::Science => match tech.current {
                Some(t) => format!("{} {}/{}", t.name(), tech.progress, tech.cost_of(t)),
                None => "No research".into(),
            },
            StatText::Population => population.to_string(),
            StatText::Government => state.government.name().to_string(),
            StatText::Score => state.score.total.to_string(),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    if let Ok(mut text) = victory.get_single_mut() {
        let value = state.victory.clone().unwrap_or_default();
        if text.0 != value {
            text.0 = value;
        }
    }
}

pub fn update_tile_info(
    hovered: Res<Hovered>,
    fog: Res<FogOfWar>,
    tiles: Query<&HexTile>,
    (tech, supplies): (Res<TechState>, Res<Supplies>),
    mut info: Query<&mut Text, With<TileInfoText>>,
) {
    let Ok(mut text) = info.get_single_mut() else { return };
    let value = match hovered.0.and_then(|hex| tiles.iter().find(|t| t.coord == hex)) {
        Some(tile) if fog.explored.contains(&tile.coord) => {
            let mut extras: Vec<String> = tile
                .resource
                .iter()
                .filter(|r| r.reveal_tech().is_none_or(|t| tech.researched.contains(&t)))
                .map(|r| if supplies.unconnected.get(&tile.coord) == Some(&PLAYER) { format!("{} (needs a road)", r.name()) } else { r.name().to_string() })
                .collect();
            extras.extend(tile.improvements.iter().map(|i| format!("{:?}", i)));
            format!("{}  -  {} food, {} shields\n{}", tile.terrain.name(), tile.total_food(), tile.total_production(), extras.join(", "))
        }
        Some(_) => "Unexplored".to_string(),
        None => String::new(),
    };
    if text.0 != value {
        text.0 = value;
    }
}

pub fn update_log(log: Res<GameLog>, mut text: Query<&mut Text, With<LogText>>) {
    if !log.is_changed() {
        return;
    }
    if let Ok(mut text) = text.get_single_mut() {
        let start = log.lines.len().saturating_sub(4);
        text.0 = log.lines[start..].join("\n");
    }
}

/// Shrinks the whole interface on narrow (phone) screens so the panels never collide.
pub fn fit_ui_scale(windows: Query<&Window>, mut scale: ResMut<UiScale>) {
    let Ok(window) = windows.get_single() else { return };
    let target = (window.width() / 740.0).clamp(0.55, 1.0);
    if (scale.0 - target).abs() > 0.01 {
        scale.0 = target;
    }
}

// ---- buttons -----------------------------------------------------------------------------------

pub fn button_actions(
    buttons: Query<(&Interaction, &ButtonCommand), Changed<Interaction>>,
    mut actions: EventWriter<Action>,
    mut ui: ResMut<UiState>,
    mut menu: ResMut<MenuState>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    (mut sounds, mut sound): (EventWriter<Sfx>, ResMut<SoundSettings>),
) {
    for (interaction, command) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        sounds.send(Sfx::Click);
        match &command.0 {
            Command::ToggleSound => sound.toggle(),
            Command::CityTab(tab) => ui.city_tab = *tab,
            Command::Game(action) => {
                actions.send(action.clone());
            }
            Command::Toggle(modal) => ui.modal = if ui.modal == Some(*modal) { None } else { Some(*modal) },
            Command::CloseModal => ui.modal = None,
            Command::SelectCiv(civ) => ui.diplo_target = Some(*civ),
            Command::PickScenario(index) => menu.scenario = Some(*index),
            Command::BackToScenarios => menu.scenario = None,
            Command::PickCiv(civ) => {
                if let Some(scenario) = menu.scenario.and_then(|i| menu.catalog.get(i)) {
                    commands.insert_resource(NewGame { scenario: scenario.clone(), civ: *civ });
                    next.set(AppState::Playing);
                }
            }
            Command::QuitToMenu => {
                ui.modal = None;
                ui.diplo_target = None;
                menu.scenario = None;
                next.set(AppState::Menu);
            }
        }
    }
}

// ---- start menu --------------------------------------------------------------------------------

pub fn setup_menu(mut commands: Commands, mut menu: ResMut<MenuState>) {
    menu.catalog = scenario::catalog();
    commands.spawn((
        MenuRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.07, 0.065, 0.055)),
        GlobalZIndex(50),
        Interaction::None,
        FocusPolicy::Block,
    ));
}

pub fn rebuild_menu(
    menu: Res<MenuState>,
    state: Res<State<AppState>>,
    art: Res<Art>,
    mut root: Query<(Entity, &mut Node), With<MenuRoot>>,
    mut commands: Commands,
) {
    if !(menu.is_changed() || state.is_changed()) {
        return;
    }
    let Ok((root, mut node)) = root.get_single_mut() else { return };
    node.display = if *state.get() == AppState::Menu { Display::Flex } else { Display::None };
    let art = &*art;
    commands.entity(root).despawn_descendants().with_children(|p| {
        if *state.get() != AppState::Menu {
            return;
        }
        p.spawn(panel(
            &art.parchment,
            Node { width: Val::Px(760.0), max_width: Val::Percent(96.0), padding: UiRect::all(Val::Px(30.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), ..default() },
        ))
        .with_children(|w| {
            w.spawn(label(&art.font_title, "BEVY CIV", 46.0, INK));
            match menu.scenario.and_then(|i| menu.catalog.get(i)) {
                None => {
                    w.spawn(label(&art.font_body, "Choose a world to rule.", 18.0, INK_SOFT));
                    for (index, scenario) in menu.catalog.iter().enumerate() {
                        w.spawn((
                            Button,
                            Node { padding: UiRect::axes(Val::Px(18.0), Val::Px(12.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() },
                            ImageNode { image: art.button.clone(), image_mode: sliced(14.0), color: BRONZE, ..default() },
                            ButtonTint(BRONZE),
                            ButtonCommand(Command::PickScenario(index)),
                        ))
                        .with_children(|b| {
                            b.spawn(label(&art.font_title, scenario.name.clone(), 21.0, INK));
                            b.spawn(label(&art.font_body, scenario.blurb.clone(), 15.0, INK));
                        });
                    }
                }
                Some(scenario) => {
                    w.spawn(label(&art.font_title, scenario.name.clone(), 24.0, INK));
                    w.spawn(label(&art.font_body, scenario.blurb.clone(), 16.0, INK_SOFT));
                    w.spawn(label(&art.font_title, "Choose your people", 17.0, INK_SOFT));
                    w.spawn(row(8.0)).with_children(|r| {
                        for (index, civ) in scenario.civs.iter().enumerate() {
                            r.spawn((
                                Button,
                                Node { width: Val::Px(220.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)), align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() },
                                ImageNode { image: art.button.clone(), image_mode: sliced(14.0), color: BRONZE, ..default() },
                                ButtonTint(BRONZE),
                                ButtonCommand(Command::PickCiv(index)),
                            ))
                            .with_children(|b| {
                                b.spawn(portrait(art, civ.portrait, 46.0));
                                b.spawn(column(1.0)).with_children(|c| {
                                    c.spawn(label(&art.font_bold, civ.name.trim_start_matches("the ").to_string(), 16.0, INK));
                                    c.spawn(label(&art.font_body, civ.leader.clone(), 14.0, INK));
                                });
                            });
                        }
                    });
                    w.spawn(row(6.0)).with_children(|r| button(r, art, "Back", Command::BackToScenarios, BRONZE, 15.0));
                }
            }
        });
    });
}

pub fn button_visuals(mut buttons: Query<(&Interaction, &ButtonTint, Option<&mut ImageNode>, Option<&mut BackgroundColor>), Changed<Interaction>>) {
    for (interaction, tint, image, background) in &mut buttons {
        let color = match interaction {
            Interaction::Pressed => tint.0.mix(&Color::BLACK, 0.25),
            Interaction::Hovered => tint.0.mix(&Color::WHITE, 0.35),
            Interaction::None => tint.0,
        };
        // Textured buttons are tinted; flat ones (the city panel's) are filled.
        if let Some(mut image) = image {
            image.color = color;
        } else if let Some(mut background) = background {
            background.0 = color;
        }
    }
}

// ---- minimap -----------------------------------------------------------------------------------

/// World-to-minimap scale and the world point shown at the minimap's centre.
fn minimap_view(map: &MapData) -> (f32, Vec2) {
    let span = (map.max - map.min + Vec2::splat(120.0)).max(Vec2::splat(1.0));
    ((MM as f32 / span.x).min(MM as f32 / span.y), (map.min + map.max) / 2.0)
}

fn minimap_color(terrain: Terrain) -> [u8; 3] {
    match terrain {
        Terrain::Water => [44, 104, 176],
        Terrain::Grassland => [92, 154, 62],
        Terrain::Plains => [176, 164, 92],
        Terrain::Desert => [224, 200, 134],
        Terrain::Forest => [44, 104, 52],
        Terrain::Hills => [128, 140, 74],
        Terrain::Mountain => [150, 146, 140],
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_minimap(
    time: Res<Time>,
    mut cooldown: Local<f32>,
    minimap: Res<MinimapImage>,
    mut images: ResMut<Assets<Image>>,
    map: Res<MapData>,
    fog: Res<FogOfWar>,
    tiles: Query<&HexTile>,
    cities: Query<&City>,
    units: Query<&Unit>,
    camera: Query<(&Transform, &OrthographicProjection), With<Camera2d>>,
    windows: Query<&Window>,
) {
    *cooldown -= time.delta_secs();
    if *cooldown > 0.0 {
        return;
    }
    *cooldown = 0.25;
    let Some(image) = images.get_mut(&minimap.0) else { return };
    let size = MM as i32;
    let mut data = vec![0u8; (MM * MM * 4) as usize];
    for px in data.chunks_exact_mut(4) {
        px.copy_from_slice(&[14, 12, 10, 255]);
    }
    let (scale, centre) = minimap_view(&map);
    let to_px = |world: Vec2| ((size as f32 / 2.0 + (world.x - centre.x) * scale) as i32, (size as f32 / 2.0 - (world.y - centre.y) * scale) as i32);
    let (tile_w, tile_h) = (((72.0 * scale) / 2.0).ceil() as i32, ((83.0 * scale) / 2.0).ceil() as i32);
    let mut fill = |cx: i32, cy: i32, half_w: i32, half_h: i32, color: [u8; 3]| {
        for y in (cy - half_h).max(0)..=(cy + half_h).min(size - 1) {
            for x in (cx - half_w).max(0)..=(cx + half_w).min(size - 1) {
                let i = ((y * size + x) * 4) as usize;
                data[i..i + 3].copy_from_slice(&color);
            }
        }
    };
    for tile in &tiles {
        let (x, y) = to_px(map.layout.hex_to_world_pos(tile.coord));
        let mut color = if fog.explored.contains(&tile.coord) { minimap_color(tile.terrain) } else { [40, 35, 27] };
        if fog.explored.contains(&tile.coord) && !fog.visible.contains(&tile.coord) {
            color = color.map(|c| (c as f32 * 0.6) as u8);
        }
        fill(x, y, tile_w, tile_h, color);
    }
    let rgb = |owner: u32| {
        let c = owner_color(owner).to_srgba();
        [(c.red * 255.0) as u8, (c.green * 255.0) as u8, (c.blue * 255.0) as u8]
    };
    for unit in units.iter().filter(|u| u.owner == PLAYER || fog.visible.contains(&u.position)) {
        let (x, y) = to_px(map.layout.hex_to_world_pos(unit.position));
        fill(x, y, 1, 1, if unit.owner == BARBARIANS { [20, 20, 20] } else { rgb(unit.owner) });
    }
    for city in cities.iter().filter(|c| fog.explored.contains(&c.position)) {
        let (x, y) = to_px(map.layout.hex_to_world_pos(city.position));
        fill(x, y, tile_w + 2, tile_w + 2, [255, 255, 255]);
        fill(x, y, tile_w + 1, tile_w + 1, rgb(city.owner));
    }
    // Outline of what the camera currently shows.
    if let (Ok((transform, projection)), Ok(window)) = (camera.get_single(), windows.get_single()) {
        let half = Vec2::new(window.width(), window.height()) * projection.scale / 2.0;
        let (x0, y0) = to_px(transform.translation.truncate() + Vec2::new(-half.x, half.y));
        let (x1, y1) = to_px(transform.translation.truncate() + Vec2::new(half.x, -half.y));
        let (x0, y0, x1, y1) = (x0.clamp(0, size - 1), y0.clamp(0, size - 1), x1.clamp(0, size - 1), y1.clamp(0, size - 1));
        fill((x0 + x1) / 2, y0, (x1 - x0) / 2, 0, [255, 255, 255]);
        fill((x0 + x1) / 2, y1, (x1 - x0) / 2, 0, [255, 255, 255]);
        fill(x0, (y0 + y1) / 2, 0, (y1 - y0) / 2, [255, 255, 255]);
        fill(x1, (y0 + y1) / 2, 0, (y1 - y0) / 2, [255, 255, 255]);
    }
    image.data = data;
}

/// Clicking (or dragging on) the minimap moves the camera there.
pub fn minimap_click(
    minimap: Query<(&Interaction, &RelativeCursorPosition), With<MinimapNode>>,
    map: Res<MapData>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
) {
    let (Ok((interaction, cursor)), Ok(mut transform)) = (minimap.get_single(), camera.get_single_mut()) else { return };
    if *interaction != Interaction::Pressed {
        return;
    }
    if let Some(pos) = cursor.normalized {
        let (scale, centre) = minimap_view(&map);
        let world = centre + Vec2::new(pos.x - 0.5, 0.5 - pos.y) * MM as f32 / scale;
        transform.translation.x = world.x;
        transform.translation.y = world.y;
    }
}

/// Keeps the turn buttons in step with how many units still wait for orders.
pub fn update_turn_buttons(
    units: Query<&Unit>,
    mut label: Query<&mut Text, With<NextUnitLabel>>,
    mut end: Query<(&Interaction, &mut ButtonTint, &mut ImageNode), With<EndTurnButton>>,
) {
    let waiting = units.iter().filter(|u| crate::needs_orders(u)).count();
    if let Ok(mut text) = label.get_single_mut() {
        let value = if waiting == 0 { "All units have orders".to_string() } else { format!("Next Unit ({waiting} waiting)") };
        if text.0 != value {
            text.0 = value;
        }
    }
    if let Ok((interaction, mut tint, mut image)) = end.get_single_mut() {
        let color = if waiting == 0 { Color::srgb(0.62, 0.86, 0.45) } else { BRONZE };
        if tint.0 != color {
            tint.0 = color;
            if *interaction == Interaction::None {
                image.color = color;
            }
        }
    }
}
