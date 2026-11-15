//! The interface: status bar, unit and city panels, advisors, minimap and message log.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::image::ImageSampler;
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::ui::widget::NodeImageMode;
use bevy::ui::{FocusPolicy, RelativeCursorPosition};

use bevy_civ::ai::Civilizations;
use bevy_civ::building::{Building, CityBuildings};
use bevy_civ::city::{City, ProductionItem};
use bevy_civ::diplomacy::{Diplomacy, Happiness, Treaty};
use bevy_civ::fog::FogOfWar;
use bevy_civ::tech::{Tech, TechState};
use bevy_civ::terrain::{HexTile, Improvement, Terrain};
use bevy_civ::unit::{Unit, UnitType};

use super::actors::unit_row;
use super::map::Hovered;
use super::{icon, owner_color, owner_name, Art};
use crate::{Action, Dirty, GameLog, GameState, MapData, Selection, BARBARIANS, PLAYER};

const CREAM: Color = Color::srgb(0.94, 0.9, 0.81);
const GOLD: Color = Color::srgb(0.95, 0.83, 0.52);
const MUTED: Color = Color::srgb(0.7, 0.68, 0.62);
const INK: Color = Color::srgb(0.16, 0.11, 0.07);
const INK_SOFT: Color = Color::srgb(0.36, 0.27, 0.18);
const BRONZE: Color = Color::srgb(0.86, 0.68, 0.36);
const GREEN: Color = Color::srgb(0.56, 0.8, 0.42);
const MM: u32 = 176;
const MM_SCALE: f32 = 0.094;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modal {
    Science,
    Diplomacy,
}

#[derive(Resource, Default)]
pub struct UiState {
    pub modal: Option<Modal>,
}

#[derive(Clone)]
pub enum Command {
    Game(Action),
    Toggle(Modal),
    CloseModal,
}

#[derive(Component)]
pub struct ButtonCommand(Command);
#[derive(Component)]
pub struct ButtonTint(Color);
#[derive(Component, Clone, Copy)]
pub enum StatText {
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

pub fn year_label(turn: u32) -> String {
    let year = -4000 + (turn as i32 - 1) * 40;
    if year < 0 { format!("{} BC", -year) } else { format!("{} AD", year.max(1)) }
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
                column_gap: Val::Px(20.0),
                row_gap: Val::Px(4.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
        ))
        .with_children(|bar| {
            bar.spawn(label(&art.font_title, "BEVY CIV", 20.0, GOLD));
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
            button(bar, &art, "Science (Q)", Command::Toggle(Modal::Science), BRONZE, 15.0);
            button(bar, &art, "Diplomacy (E)", Command::Toggle(Modal::Diplomacy), BRONZE, 15.0);
            button(bar, &art, "Save", Command::Game(Action::Save), BRONZE, 15.0);
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
            p.spawn((label(&art.font_body, "", 14.0, CREAM), TileInfoText, Node { min_height: Val::Px(36.0), ..default() }));
            button(p, &art, "End Turn", Command::Game(Action::EndTurn), Color::srgb(0.62, 0.86, 0.45), 19.0);
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
                width: Val::Px(330.0),
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
    ui: Res<UiState>,
    art: Res<Art>,
    state: Res<GameState>,
    tech: Res<TechState>,
    civs: Res<Civilizations>,
    diplomacy: Res<Diplomacy>,
    units: Query<&Unit>,
    cities: Query<(&City, &CityBuildings, &Happiness)>,
    unit_panel: Query<Entity, With<UnitPanel>>,
    mut city_panel: Query<(Entity, &mut Node), (With<CityPanel>, Without<ModalRoot>)>,
    mut modal_root: Query<(Entity, &mut Node), (With<ModalRoot>, Without<CityPanel>)>,
    mut commands: Commands,
) {
    if !(dirty.ui || selection.is_changed() || ui.is_changed()) {
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
                    "Click a unit or city to select it. Click a tile (or right-click) to move.\nDrag to pan, scroll to zoom, Space ends the turn.",
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
                        _ if unit.movement_left == 0 => "No moves left".into(),
                        _ => "Awaiting orders".into(),
                    };
                    c.spawn(label(&art.font_body, format!("{} unit  {}", owner_name(unit.owner, &civs), status), 14.0, owner_color(unit.owner).mix(&Color::WHITE, 0.55)));
                    c.spawn(row(12.0)).with_children(|s| {
                        stat(s, art, icon::ATTACK, attack.to_string(), CREAM);
                        stat(s, art, icon::DEFENSE, defense.to_string(), CREAM);
                        stat(s, art, icon::MOVES, format!("{}/{}", unit.movement_left, moves), CREAM);
                    });
                });
            });
            if mine {
                p.spawn(row(6.0)).with_children(|r| {
                    if unit.unit_type == UnitType::Settler {
                        button(r, art, "Found City (B)", Command::Game(Action::FoundCity), GREEN, 15.0);
                    }
                    if unit.unit_type.is_worker() && unit.build_turns_left.is_none() {
                        button(r, art, "Farm (F)", Command::Game(Action::Build(Improvement::Farm)), BRONZE, 15.0);
                        button(r, art, "Mine (M)", Command::Game(Action::Build(Improvement::Mine)), BRONZE, 15.0);
                        button(r, art, "Road (R)", Command::Game(Action::Build(Improvement::Road)), BRONZE, 15.0);
                        button(r, art, "Clear (X)", Command::Game(Action::Transform), BRONZE, 15.0);
                    }
                    button(r, art, "Skip", Command::Game(Action::Skip), BRONZE, 15.0);
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
            let dist = capital.map(|c| city.position.distance_to(c) as u32).unwrap_or(0);
            let gov = state.government;
            let food = city.food_per_turn(buildings, gov, dist);
            let production = city.production_per_turn(buildings, gov, dist);
            p.spawn(row(14.0)).with_children(|r| {
                stat(r, art, icon::FOOD, format!("+{food}"), CREAM);
                stat(r, art, icon::PRODUCTION, format!("+{production}"), CREAM);
                stat(r, art, icon::SCIENCE, format!("+{}", city.science_per_turn(buildings, gov, dist)), CREAM);
                stat(r, art, icon::GOLD, format!("+{}", city.gold_per_turn(buildings) + 1), CREAM);
                stat(r, art, icon::HAPPY, format!("{} / {}", happiness.happy + happiness.content, happiness.unhappy), if happiness.in_disorder() { Color::srgb(1.0, 0.45, 0.4) } else { CREAM });
            });
            let grow_in = (city.food_to_grow().saturating_sub(city.food_storage) as f32 / food.max(1) as f32).ceil();
            p.spawn(label(&art.font_body, format!("Growth: {}/{} food  (about {} turns)", city.food_storage, city.food_to_grow(), grow_in), 14.0, MUTED));
            bar(p, city.food_storage as f32 / city.food_to_grow() as f32, Color::srgb(0.45, 0.78, 0.35));
            if let Some(item) = &city.production_queue {
                let left = (item.cost().saturating_sub(city.production_progress) as f32 / production.max(1) as f32).ceil();
                p.spawn(label(&art.font_body, format!("Building: {}  {}/{}  (about {} turns)", item.name(), city.production_progress, item.cost(), left), 14.0, MUTED));
                bar(p, city.production_progress as f32 / item.cost() as f32, Color::srgb(0.92, 0.62, 0.25));
            }
            let built = if buildings.buildings.is_empty() { "none yet".to_string() } else { buildings.buildings.iter().map(|b| b.name()).collect::<Vec<_>>().join(", ") };
            p.spawn(label(&art.font_body, format!("Buildings: {built}"), 14.0, CREAM));
            p.spawn(label(&art.font_title, "Change production", 15.0, GOLD));
            p.spawn(row(5.0)).with_children(|r| {
                let known = |t: Option<Tech>| t.is_none_or(|t| tech.researched.contains(&t));
                let mut options: Vec<ProductionItem> = UnitType::ALL.into_iter().filter(|u| known(u.required_tech())).map(ProductionItem::Unit).collect();
                options.extend(Building::ALL.into_iter().filter(|b| known(b.required_tech()) && !buildings.buildings.contains(b)).map(ProductionItem::Building));
                for item in options {
                    let current = city.production_queue.as_ref() == Some(&item);
                    let tint = if current { GREEN } else if matches!(item, ProductionItem::Building(_)) { Color::srgb(0.72, 0.78, 0.88) } else { BRONZE };
                    button(r, art, format!("{} {}", item.name(), item.cost()), Command::Game(Action::SetProduction(item)), tint, 13.0);
                }
            });
        });
    }

    // --- advisors ---
    if let Ok((root, mut node)) = modal_root.get_single_mut() {
        node.display = if ui.modal.is_some() { Display::Flex } else { Display::None };
        commands.entity(root).despawn_descendants().with_children(|p| {
            let Some(modal) = ui.modal else { return };
            p.spawn(panel(
                &art.parchment,
                Node { width: Val::Px(580.0), max_width: Val::Percent(94.0), padding: UiRect::all(Val::Px(26.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() },
            ))
            .with_children(|w| match modal {
                Modal::Science => {
                    w.spawn(label(&art.font_title, "Science Advisor", 26.0, INK));
                    match tech.current {
                        Some(current) => {
                            let left = (current.cost().saturating_sub(tech.progress) as f32 / tech.science_per_turn.max(1) as f32).ceil();
                            w.spawn(label(&art.font_bold, format!("Researching {}  -  {}/{} beakers, about {} turns", current.name(), tech.progress, current.cost(), left), 16.0, INK));
                            bar(w, tech.progress as f32 / current.cost() as f32, Color::srgb(0.25, 0.6, 0.9));
                        }
                        None => {
                            w.spawn(label(&art.font_bold, "Our scholars are idle. Choose a field of study.", 16.0, INK));
                        }
                    }
                    w.spawn(label(&art.font_title, "Choose research", 16.0, INK_SOFT));
                    w.spawn(row(6.0)).with_children(|r| {
                        for t in Tech::ALL.into_iter().filter(|t| tech.can_research(*t)) {
                            let tint = if tech.current == Some(t) { GREEN } else { BRONZE };
                            button(r, art, format!("{} ({})", t.name(), t.cost()), Command::Game(Action::Research(t)), tint, 14.0);
                        }
                    });
                    let known: Vec<&str> = Tech::ALL.iter().filter(|t| tech.researched.contains(t)).map(|t| t.name()).collect();
                    w.spawn(label(&art.font_body, format!("Known: {}", if known.is_empty() { "nothing yet".into() } else { known.join(", ") }), 15.0, INK_SOFT));
                    w.spawn(row(6.0)).with_children(|r| button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0));
                }
                Modal::Diplomacy => {
                    w.spawn(label(&art.font_title, "Foreign Advisor", 26.0, INK));
                    w.spawn(label(&art.font_body, format!("We are governed as a {}.", state.government.name()), 15.0, INK_SOFT));
                    for civ in civs.players.iter().filter(|c| !c.is_human) {
                        let treaty = diplomacy.get_treaty(PLAYER, civ.id);
                        let attitude = diplomacy.get_attitude(PLAYER, civ.id);
                        let mood = match attitude {
                            i32::MIN..=-20 => "hostile",
                            -19..=-1 => "wary",
                            0..=14 => "neutral",
                            _ => "friendly",
                        };
                        w.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(8.0), ..default() }).with_children(|r| {
                            r.spawn((Node { width: Val::Px(16.0), height: Val::Px(16.0), ..default() }, BackgroundColor(owner_color(civ.id)), BorderRadius::all(Val::Px(8.0))));
                            r.spawn(label(&art.font_bold, format!("{}  -  {mood} ({attitude}), treaty: {:?}", civ.name.replace("AI ", ""), treaty), 16.0, INK));
                        });
                        w.spawn(row(6.0)).with_children(|r| {
                            for (text, t) in [("Offer peace", Treaty::Peace), ("Propose alliance", Treaty::Alliance), ("Declare war", Treaty::War)] {
                                let tint = if treaty == t { GREEN } else { BRONZE };
                                button(r, art, text, Command::Game(Action::SetTreaty(civ.id, t)), tint, 14.0);
                            }
                        });
                    }
                    w.spawn(row(6.0)).with_children(|r| {
                        button(r, art, "Link cities by trade (T)", Command::Game(Action::TradeRoute), BRONZE, 14.0);
                        button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0);
                    });
                }
            });
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
            StatText::Turn => format!("Turn {}  -  {}", state.turn, year_label(state.turn)),
            StatText::Gold => format!("{} ({:+})", state.gold, state.gold_per_turn),
            StatText::Science => match tech.current {
                Some(t) => format!("{} {}/{}", t.name(), tech.progress, t.cost()),
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
    mut info: Query<&mut Text, With<TileInfoText>>,
) {
    let Ok(mut text) = info.get_single_mut() else { return };
    let value = match hovered.0.and_then(|hex| tiles.iter().find(|t| t.coord == hex)) {
        Some(tile) if fog.explored.contains(&tile.coord) => {
            let mut extras: Vec<String> = tile.resource.iter().map(|r| r.name().to_string()).collect();
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
) {
    for (interaction, command) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match &command.0 {
            Command::Game(action) => {
                actions.send(action.clone());
            }
            Command::Toggle(modal) => ui.modal = if ui.modal == Some(*modal) { None } else { Some(*modal) },
            Command::CloseModal => ui.modal = None,
        }
    }
}

pub fn button_visuals(mut buttons: Query<(&Interaction, &ButtonTint, &mut ImageNode), Changed<Interaction>>) {
    for (interaction, tint, mut image) in &mut buttons {
        image.color = match interaction {
            Interaction::Pressed => tint.0.mix(&Color::BLACK, 0.25),
            Interaction::Hovered => tint.0.mix(&Color::WHITE, 0.35),
            Interaction::None => tint.0,
        };
    }
}

// ---- minimap -----------------------------------------------------------------------------------

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
    let to_px = |world: Vec2| ((size as f32 / 2.0 + world.x * MM_SCALE) as i32, (size as f32 / 2.0 - world.y * MM_SCALE) as i32);
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
        fill(x, y, 3, 4, color);
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
        fill(x, y, 4, 4, [255, 255, 255]);
        fill(x, y, 3, 3, rgb(city.owner));
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
    mut camera: Query<&mut Transform, With<Camera2d>>,
) {
    let (Ok((interaction, cursor)), Ok(mut transform)) = (minimap.get_single(), camera.get_single_mut()) else { return };
    if *interaction != Interaction::Pressed {
        return;
    }
    if let Some(pos) = cursor.normalized {
        let world = Vec2::new(pos.x - 0.5, 0.5 - pos.y) * MM as f32 / MM_SCALE;
        transform.translation.x = world.x;
        transform.translation.y = world.y;
    }
}
