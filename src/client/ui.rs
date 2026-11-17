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
use bevy_civ::espionage::{self, Espionage, Mission};
use bevy_civ::trade::TradeRoutes;
use bevy_civ::government::Government;
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
    Empire,
    Help,
    /// Save, sound, and leaving the game.
    Game,
}

/// Pages of the empire advisor.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EmpireTab {
    #[default]
    Government,
    Cities,
    Military,
    Score,
}

impl EmpireTab {
    const ALL: [EmpireTab; 4] = [EmpireTab::Government, EmpireTab::Cities, EmpireTab::Military, EmpireTab::Score];

    fn name(self) -> &'static str {
        match self {
            EmpireTab::Government => "Government",
            EmpireTab::Cities => "Cities",
            EmpireTab::Military => "Military",
            EmpireTab::Score => "Score",
        }
    }
}

/// Topics of the encyclopedia.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum HelpTopic {
    #[default]
    Basics,
    Units,
    Buildings,
    Wonders,
    Cities,
    Peoples,
    War,
    Victory,
}

impl HelpTopic {
    const ALL: [HelpTopic; 8] = [
        HelpTopic::Basics,
        HelpTopic::Units,
        HelpTopic::Buildings,
        HelpTopic::Wonders,
        HelpTopic::Cities,
        HelpTopic::Peoples,
        HelpTopic::War,
        HelpTopic::Victory,
    ];

    fn name(self) -> &'static str {
        match self {
            HelpTopic::Basics => "Basics & keys",
            HelpTopic::Units => "Units",
            HelpTopic::Buildings => "Buildings",
            HelpTopic::Wonders => "Wonders",
            HelpTopic::Cities => "Cities & happiness",
            HelpTopic::Peoples => "Peoples & faith",
            HelpTopic::War => "War",
            HelpTopic::Victory => "Victory",
        }
    }
}

/// The text of an encyclopedia page, one paragraph or table row per line.
fn help_lines(topic: HelpTopic) -> Vec<String> {
    match topic {
        HelpTopic::Basics => [
            "Click a unit or city to select it; click again or press Tab for the next unit on that tile. Click a tile (or right-click) to move; far destinations become standing orders. Hover over a tile to see the route and how many turns it takes.",
            "Space: skip a unit (or go to a city that needs a project, then end the turn). Enter: end the turn. N: next unit. H: fortify or sleep. Esc: close or deselect.",
            "Workers and Engineers: F farm, M mine, R road (railroad on a road, with Steam Power), O fortress (Construction), X clear land or clean pollution, K automate. Settlers: B found a city.",
            "U uses the selected unit: a great person's gift, a General forming an army, a Missionary preaching, a caravan helping a wonder. T opens a caravan's trade route. P pillages enemy land. Diplomats act from the unit panel next to a foreign city.",
            "Q science, E foreign affairs, G trade, Y empire (government, rates, cities, military, score), F1 this encyclopedia, V sound, F2 save a file (the game also saves itself every turn).",
            "Roads triple movement for troops on foot; railroads make it free. Hills and forest cost 2 moves, mountains 3. Troops cross water in boats once you know Seafaring.",
            "Tribal villages dot the land: the first unit in finds gold, a technology, warriors who join you, maps of the area - or hostile tribesmen (Scouts rarely meet them).",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        HelpTopic::Units => UnitType::ALL
            .into_iter()
            .map(|u| {
                let (a, d, m) = u.stats();
                let needs = u.required_tech().map(|t| format!(", needs {}", t.name())).unwrap_or_default();
                let res: Vec<&str> = u.required_resources().iter().map(|r| r.name()).collect();
                let res = if res.is_empty() { String::new() } else { format!(" and {}", res.join(", ")) };
                if u.is_great_person() {
                    format!("{}: {}", u.name(), great::ability(u))
                } else {
                    format!("{}: attack {a}, defence {d}, moves {m}; {} shields{needs}{res}", u.name(), u.cost())
                }
            })
            .collect(),
        HelpTopic::Buildings => Building::ALL
            .into_iter()
            .filter(|b| !b.is_wonder())
            .map(|b| format!("{}: {} ({} shields{})", b.name(), b.effect(), b.cost(), b.required_tech().map(|t| format!(", needs {}", t.name())).unwrap_or_default()))
            .collect(),
        HelpTopic::Wonders => Building::ALL
            .into_iter()
            .filter(|b| b.is_wonder())
            .map(|b| format!("{}: {} ({} shields{}). One per world.", b.name(), b.wonder_effect(), b.cost(), b.required_tech().map(|t| format!(", needs {}", t.name())).unwrap_or_default()))
            .collect(),
        HelpTopic::Cities => [
            "A city's first five citizens are content; every one beyond is unhappy. Temples (+2), Colosseums and Cathedrals (+3), wonders, masterpieces, entertainers and, under Despotism, Monarchy and Communism, troops in the city (martial law) calm them. Each different luxury the city can reach makes one citizen happy.",
            "When the unhappy outnumber the happy the city falls into disorder: no growth, production, science or taxes until it is calmed.",
            "Cities grow when their granary fills and cannot pass size 8 without an Aqueduct, or 16 without a Sewer System. Specialists eat without farming: too many and the city starves.",
            "Culture from temples, cathedrals, colosseums, libraries, universities and wonders widens a city's borders over time (2 tiles, 3 at 100, 4 at 1,000, 5 at 5,000). Where borders meet, the land goes to the city with the stronger culture for its distance, so a great culture pushes its neighbours' borders back. A city in disorder next to a much greater culture may defect to it.",
            "Corruption grows with distance from the capital and is halved by a Courthouse. A city keeps training the same unit until told otherwise; a finished building or wonder leaves it waiting for its next project - unless its build queue holds more (switch the city panel to Add to queue and pick up to six projects).",
            "Projects other than wonders can be bought with gold, or rushed under Despotism and Communism by putting citizens to forced labour. Luxury (Empire advisor) makes one citizen happy for every 2 points.",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        HelpTopic::Peoples => vec![
            "Every people has two traits - Militaristic (troops promoted after half their victories), Commercial (+1 gold a city), Scientific (+1 science a city), Religious (+1 content a city, shorter revolutions), Industrious (+1 shield a city) or Agricultural (+1 food a city) - and a unique unit, which it trains a rank higher.".to_string(),
            "Governments: a revolution brings a few turns of anarchy. Each government limits how much of your commerce can go to science or luxury (Empire advisor, Government tab); taxes take the rest, and savings earn 5% interest, up to 2 gold per Bank and 3 per Stock Exchange.".to_string(),
            "Faith: the first to learn Mysticism, Philosophy, Theology and Printing Press founds the Old Gods, the Way of Reason, the One Faith and the Reformed Faith. Faiths spread to nearby cities; Missionaries carry your state religion (your capital's faith) further. Cities of your state religion get +1 content (+2 with a Temple); a holy city's owner earns gold from the faith's followers; shared faith warms relations, different faiths cool them.".to_string(),
            "Diplomacy (E): peace, alliances, gifts, tribute, trade agreements, technology and resource deals. A right of passage lets your units share tiles with theirs; an alliance includes it, and war ends it. Units cannot walk into the units or cities of a people at peace with you: declare war first.".to_string(),
            "Culture from temples, libraries, colosseums, cathedrals, universities and wonders widens borders (2 tiles, 3 at 100 culture, 4 at 1,000, 5 at 5,000) and pushes neighbours' borders back. A city in disorder next to a much greater culture may defect.".to_string(),
            "Great people come from great-person points: Merchants trade abroad or settle as trading houses, Scientists complete research, Artists make masterpieces, Engineers finish projects; Generals come from victories.".to_string(),
        ],
        HelpTopic::War => vec![
            "Attack by moving onto an enemy. Battles are fought in rounds over hit points (3, 4 for a veteran, 5 for an elite); winners may be promoted, faster attackers may retreat, wounds heal while resting.".to_string(),
            "Defence: hills and forest +50%, mountains double, rivers +25%, fortifying +25%, city walls or a fortress double against land attacks, a Coastal Fortress double against ships. A Great General on the tile makes troops fight half again as hard.".to_string(),
            "Armies: a Great General in one of your cities forms up to three land units into one army that fights as its best unit with the hit points of all, and moves at its slowest member's pace.".to_string(),
            "Artillery, some ships and bombers bombard without risk. Troops cannot slip past enemy troops (zone of control). Pillage (P) wrecks improvements on enemy land; computer raiders do the same to yours.".to_string(),
            "A captured city resists for a while (longer for a big one; a garrison of two halves it) and produces nothing. Under a Republic or Democracy a long war makes citizens unhappy (a Police Station halves it).".to_string(),
            "Older units are upgraded in your cities for gold once the better unit can be built there.".to_string(),
        ],
        HelpTopic::Victory => vec![
            "Conquest: take every rival city. Domination: hold two thirds of the cities in the world (at least five).".to_string(),
            "Space: research Space Flight, Plastics and Superconductors; cities with a Factory build 4 Space Structures, 2 Components and 2 Modules. The ship launches when complete and arrives 12 turns later - unless your capital falls first.".to_string(),
            format!("Culture: reach {} culture points and half as much again as any rival.", bevy_civ::culture::VICTORY_CULTURE),
            "Score: if nobody has won by the last turn (chosen when the game begins), the highest score wins. Score counts 2 points a citizen, 10 a technology and 50 a wonder, for everyone alike. A rival who holds two thirds of the cities wins by domination.".to_string(),
        ],
    }
}

#[derive(Resource, Default)]
pub struct UiState {
    pub modal: Option<Modal>,
    /// The civilization shown in the foreign advisor window.
    pub diplo_target: Option<u32>,
    /// The page of the city panel being shown (kept from city to city).
    pub city_tab: CityTab,
    pub empire_tab: EmpireTab,
    pub help_topic: HelpTopic,
    /// Move orders take the whole stack along (as Shift-click does).
    pub group_move: bool,
    /// The end-of-game window has been put away to look around the map.
    pub end_dismissed: bool,
    /// Clicking a project adds it to the city's build queue instead of starting it now.
    pub queue_mode: bool,
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
    /// A message for the player (e.g. why a file could not be loaded).
    pub notice: Option<String>,
    /// The people chosen, while the game's options are set.
    pub civ: Option<usize>,
    pub difficulty: crate::Difficulty,
    /// Game length in turns (0 until chosen: the longest).
    pub last_turn: u32,
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
    EmpireTab(EmpireTab),
    HelpTopic(HelpTopic),
    ToggleGroupMove,
    ToggleQueueMode,
    /// Put the end-of-game window away (the outcome stays on screen).
    DismissEnd,
    /// Start menu: carry on from the automatic save.
    Continue,
    /// Start menu: load a saved game file.
    LoadFile,
    PickDifficulty(crate::Difficulty),
    TipDone,
    TipsOff,
    ToggleTips,
    PickLength(u32),
    StartGame,
    BackToPeoples,
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
/// The message log and unit panel (bottom left), narrowed on phones.
#[derive(Component)]
pub struct LeftColumn;
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

/// A line of text (for other client modules).
pub fn label_bundle(font: &Handle<Font>, text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    label(font, text, size, color)
}

/// The tip panel's two buttons.
pub fn tip_buttons(parent: &mut ChildBuilder, art: &Art) {
    parent.spawn(row(6.0)).with_children(|r| {
        button(r, art, "Got it", Command::TipDone, GREEN, 14.0);
        button(r, art, "No more tips", Command::TipsOff, BRONZE, 14.0);
    });
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
/// "Religious (+1 content in every city, shorter revolutions) and Militaristic (...). Unique unit: Spanish Knight, trained a rank higher."
pub fn character_line(name: &str, adjective: &str) -> String {
    let c = bevy_civ::traits::of(name);
    format!(
        "{} ({}) and {} ({}). Unique unit: {adjective} {}, trained a rank higher.",
        c.traits[0].name(),
        c.traits[0].effect(),
        c.traits[1].name(),
        c.traits[1].effect(),
        c.unique.name()
    )
}

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
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // A flat face (no bevel) keeps the small second line readable.
            BackgroundColor(tint),
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
            button(bar, &art, "Empire (Y)", Command::Toggle(Modal::Empire), BRONZE, 14.0);
            button(bar, &art, "Help (F1)", Command::Toggle(Modal::Help), BRONZE, 14.0);
            button(bar, &art, "Game", Command::Toggle(Modal::Game), BRONZE, 14.0);
        });

    // Message log stacked on the unit panel (bottom left)
    commands
        .spawn((
            LeftColumn,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                width: Val::Px(360.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
        ))
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
    (foreign, supplies, map, religions): (Res<Foreign>, Res<Supplies>, Res<MapData>, Res<bevy_civ::religion::Religions>),
    art: Res<Art>,
    state: Res<GameState>,
    tech: Res<TechState>,
    civs: Res<Civilizations>,
    diplomacy: Res<Diplomacy>,
    (units, tiles, unit_entities, wonders, great, spies, trade): (
        Query<&Unit>,
        Query<&HexTile>,
        Query<Entity, With<Unit>>,
        Res<WorldWonders>,
        Res<GreatPeople>,
        Res<Espionage>,
        Res<TradeRoutes>,
    ),
    cities: Query<(&City, &CityBuildings, &Happiness)>,
    (city_list, space, history, culture, taken, tips_on): (
        Query<(Entity, &City)>,
        Res<bevy_civ::space::SpaceRace>,
        Res<crate::History>,
        Res<bevy_civ::culture::CultureLedger>,
        Res<crate::JustCaptured>,
        Res<super::tips::Tips>,
    ),
    unit_panel: Query<Entity, With<UnitPanel>>,
    mut city_panel: Query<(Entity, &mut Node), (With<CityPanel>, Without<ModalRoot>)>,
    mut modal_root: Query<(Entity, &mut Node), (With<ModalRoot>, Without<CityPanel>)>,
    mut commands: Commands,
) {
    if !(dirty.ui || selection.is_changed() || ui.is_changed() || foreign.is_changed() || supplies.is_changed() || tips_on.is_changed()) {
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
            let moves = moves.min(unit.troop_moves());
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
                    let unique = civs.players.iter().find(|c| c.id == unit.owner).filter(|c| bevy_civ::traits::of(&c.name).unique == unit.unit_type);
                    let title = if unit.is_army_group() {
                        format!("Army of {} units", 1 + unit.troops.len())
                    } else if let Some(civ) = unique {
                        format!("{} {}", civ.adjective, unit.unit_type.name())
                    } else {
                        unit.unit_type.name().to_string()
                    };
                    c.spawn(label(&art.font_title, title, 20.0, GOLD));
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
                        if unit.unit_type.is_military() {
                            let hurt = unit.hp < unit.max_hp();
                            s.spawn(label(&art.font_bold, format!("HP {}/{} {}", unit.hp, unit.max_hp(), unit.level_name()), 14.0, if hurt { RED } else { CREAM }));
                        }
                    });
                });
            });
            if unit.unit_type.is_great_person() {
                p.spawn(label(&art.font_bold, great::ability(unit.unit_type), 14.0, GOLD));
            }
            if unit.is_army_group() {
                p.spawn(label(&art.font_body, format!("{}: fights as its best unit, with the hit points of all.", unit.display_name()), 14.0, CREAM));
            }
            if mine && unit.unit_type == UnitType::Caravan {
                p.spawn(label(&art.font_body, "In another of your cities, or next to a foreign one: T opens a trade route from its home city. In one of your cities building a wonder: U adds its shields.", 13.0, MUTED));
            }
            // A diplomat next to (or in) a foreign city: its missions, with their odds.
            if mine && unit.unit_type == UnitType::Diplomat {
                let target = crate::spy_target(unit.position, cities.iter().map(|(c, _, _)| (c.position, c.owner)));
                match target.and_then(|(at, owner)| cities.iter().find(|(c, _, _)| c.position == at && c.owner == owner)) {
                    Some((city, buildings, _)) => {
                        p.spawn(label(&art.font_bold, format!("At {} ({}):", city.name, owner_name(city.owner, &civs)), 14.0, GOLD));
                        let walls = buildings.has(Building::Walls);
                        p.spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(5.0), row_gap: Val::Px(5.0), ..default() }).with_children(|grid| {
                            for mission in Mission::ALL {
                                if mission == Mission::Embassy && spies.has_embassy(PLAYER, city.owner) {
                                    continue;
                                }
                                let odds = (mission.odds(walls) * 100.0).round() as u32;
                                let tint = if mission.is_covert() { Color::srgb(0.95, 0.72, 0.6) } else { Color::srgb(0.66, 0.9, 0.52) };
                                production_button(grid, art, mission.name(), format!("{odds}%: {}", mission.describe()), Command::Game(Action::Spy(mission)), tint);
                            }
                        });
                        p.spawn(label(&art.font_body, "The diplomat is spent either way. If caught, they will not forget it.", 12.0, MUTED));
                    }
                    None => {
                        p.spawn(label(&art.font_body, "Move next to a foreign city to open an embassy, steal technology, sabotage, steal gold or incite unrest.", 13.0, MUTED));
                    }
                }
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
                    let together = if ui.group_move { "Move together: On" } else { "Move together: Off (or Shift-click)" };
                    p.spawn(row(6.0)).with_children(|r| button(r, art, together, Command::ToggleGroupMove, if ui.group_move { GREEN } else { BRONZE }, 13.0));
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
                        if tech.researched.contains(&Tech::Construction) {
                            button(r, art, "Fortress (O)", Command::Game(Action::Build(Improvement::Fortress)), BRONZE, 15.0);
                        }
                        button(r, art, "Automate (K)", Command::Game(Action::Automate), GREEN, 15.0);
                    }
                    // Upgrade in one of our cities, once the successor can be equipped there.
                    if let Some(to) = unit.unit_type.upgrades_to() {
                        let in_city = cities.iter().any(|(c, _, _)| c.position == unit.position && c.owner == PLAYER);
                        let ready = to.required_tech().is_none_or(|t| tech.researched.contains(&t)) && supplies.missing(unit.position, to.required_resources()).is_empty();
                        if in_city && ready {
                            button(r, art, format!("Upgrade to {} ({} gold)", to.name(), unit.unit_type.upgrade_cost(to)), Command::Game(Action::Upgrade), GREEN, 15.0);
                        }
                    }
                    if unit.unit_type.can_bombard() && unit.can_move() {
                        button(r, art, "Bombard", Command::Game(Action::Bombard), Color::srgb(0.95, 0.65, 0.5), 15.0);
                    }
                    if unit.unit_type == UnitType::Caravan {
                        button(r, art, "Trade Route (T)", Command::Game(Action::TradeRoute), GREEN, 15.0);
                        button(r, art, "Help Wonder (U)", Command::Game(Action::UseUnit), GREEN, 15.0);
                    }
                    // Pillage: troops on improved land that is not ours.
                    let foreign_land = supplies.territory.get(&unit.position).is_none_or(|(o, _)| *o != PLAYER);
                    let improved = tiles.iter().any(|t| t.coord == unit.position && bevy_civ::terrain::pillage_target(&t.improvements).is_some());
                    if unit.unit_type.is_army() && foreign_land && improved && !cities.iter().any(|(c, _, _)| c.position == unit.position) {
                        button(r, art, "Pillage (P)", Command::Game(Action::Pillage), Color::srgb(0.95, 0.65, 0.5), 15.0);
                    }
                    if unit.unit_type == UnitType::Missionary {
                        button(r, art, "Spread Faith (U)", Command::Game(Action::UseUnit), GREEN, 15.0);
                    }
                    if unit.unit_type == UnitType::GreatGeneral {
                        button(r, art, "Form Army (U)", Command::Game(Action::UseUnit), GREEN, 15.0);
                    } else if unit.unit_type.is_great_person() {
                        button(r, art, "Use (U)", Command::Game(Action::UseUnit), GREEN, 15.0);
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
            if taken.0.contains(&city.position) {
                p.spawn(row(6.0)).with_children(|r| {
                    r.spawn(label(&art.font_bold, "Just captured:", 14.0, GOLD));
                    button(r, art, "Raze it to the ground", Command::Game(Action::Raze), RED, 13.0);
                });
            }
            if let Some(turns) = spies.unrest.get(&city.position) {
                p.spawn(label(&art.font_bold, format!("In unrest: no growth or production for {turns} more turns."), 14.0, RED));
            }
            let routes = trade.at(city.position);
            if routes > 0 {
                p.spawn(label(&art.font_body, format!("Trade routes: {routes} of {}", bevy_civ::trade::MAX_ROUTES_PER_CITY), 14.0, CREAM));
            }
            let dist = capital.map(|c| city.position.distance_to(c) as u32).unwrap_or(0);
            let gov = state.government;
            let (extra_food, extra_shields, extra_gold) = supplies.yields.get(&city.position).copied().unwrap_or_default();
            let food = city.food_per_turn(buildings, gov, dist) + extra_food;
            let production = city.production_per_turn(buildings, gov, dist) + extra_shields;
            p.spawn(row(14.0)).with_children(|r| {
                stat(r, art, icon::FOOD, format!("+{food}"), CREAM);
                stat(r, art, icon::PRODUCTION, format!("+{production}"), CREAM);
                let commerce = city.science_per_turn(buildings, gov, dist) + city.gold_per_turn(buildings) + 1;
                let (science, luxury, gold) = state.split_commerce(commerce);
                stat(r, art, icon::SCIENCE, format!("+{science}"), CREAM);
                stat(r, art, icon::GOLD, format!("+{}", gold as i32 + extra_gold as i32 + great.merchant_gold(city.position)), CREAM);
                if luxury > 0 {
                    r.spawn(label(&art.font_body, format!("lux {luxury}"), 14.0, CREAM));
                }
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
                    // Hurrying: gold for anything but wonders, or citizens under Despotism and Communism.
                    if mine && city.shields_left() > 0 && left > 1.0 {
                        p.spawn(row(6.0)).with_children(|r| {
                            match city.hurry_cost() {
                                Some(cost) => {
                                    let tint = if state.gold >= cost { GREEN } else { GRAY };
                                    button(r, art, format!("Buy now: {cost} gold"), Command::Game(Action::Hurry), tint, 13.0);
                                    if let Some(citizens) = city.rush_citizens(state.government) {
                                        let plural = if citizens == 1 { "citizen" } else { "citizens" };
                                        button(r, art, format!("Rush: {citizens} {plural}"), Command::Game(Action::Rush), Color::srgb(0.9, 0.6, 0.5), 13.0);
                                    }
                                }
                                None => {
                                    r.spawn(label(&art.font_body, "Wonders cannot be bought: caravans (U) and great engineers help.", 12.0, MUTED));
                                }
                            }
                        });
                    }
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
                        .filter(|b| b.is_wonder() == wonder_tab && known(b.required_tech()) && !buildings.buildings.contains(b) && (!b.needs_coast() || coastal))
                        .filter(|b| !wonders.is_taken(*b))
                        .filter(|b| !b.is_spaceship_part() || (buildings.has(Building::Factory) && space.wants(PLAYER, *b)))
                        .filter(|b| b.required_building().is_none_or(|r| buildings.has(r)))
                        .map(|b| {
                            let effect = b.effect().to_string();
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
                // Culture and the reach of the city's borders.
                let points = culture.of(city.position);
                let next = bevy_civ::culture::next_border_step(points).map(|step| format!(" (borders widen at {step})")).unwrap_or_default();
                p.spawn(label(&art.font_body, format!("Culture: {points}, +{} a turn{next}", bevy_civ::culture::per_turn(&buildings.buildings, city.is_capital)), 14.0, CREAM));
                // Citizens: workers on the land, and specialists who eat without farming.
                use bevy_civ::city::Specialist;
                let count = |kind: &Specialist| city.specialists.iter().filter(|s| *s == kind).count();
                p.spawn(label(&art.font_body, format!("Citizens: {} working the land, {} specialists. Food {:+} a turn.", city.workers(), city.specialists.len(), city.food_balance(buildings)), 14.0, CREAM));
                p.spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(5.0), row_gap: Val::Px(5.0), ..default() }).with_children(|grid| {
                    for (kind, what) in [(Specialist::Entertainer, "+2 content"), (Specialist::TaxCollector, "+3 gold"), (Specialist::Scientist, "+3 science")] {
                        let name = match kind {
                            Specialist::Entertainer => "Entertainers",
                            Specialist::TaxCollector => "Tax collectors",
                            Specialist::Scientist => "Scientists",
                        };
                        production_button(grid, art, &format!("{name}: {} (+)", count(&kind)), format!("each {what}"), Command::Game(Action::Specialist(kind.clone(), true)), Color::srgb(0.96, 0.84, 0.6));
                    }
                    for kind in [Specialist::Entertainer, Specialist::TaxCollector, Specialist::Scientist] {
                        if count(&kind) > 0 {
                            production_button(grid, art, "Back to work (-)", format!("one fewer {}", match kind { Specialist::Entertainer => "entertainer", Specialist::TaxCollector => "tax collector", Specialist::Scientist => "scientist" }), Command::Game(Action::Specialist(kind.clone(), false)), Color::srgb(0.75, 0.85, 0.95));
                        }
                    }
                });
                let masterpieces = great.masterpieces.get(&city.position).copied().unwrap_or(0);
                if masterpieces > 0 {
                    p.spawn(label(&art.font_body, format!("Masterpieces: {masterpieces} (+{} content)", masterpieces * 3), 14.0, CREAM));
                }
                // Faith: the city's, and whether it shares its ruler's (the capital's).
                let state_faith = city_list.iter().find(|(_, c)| c.owner == city.owner && c.is_capital).and_then(|(_, c)| religions.of(c.position));
                let faith_text = match religions.of(city.position) {
                    Some(f) => {
                        let holy = if religions.holy_faith(city.position) == Some(f) { format!(", its holy city (+{} gold a turn)", religions.holy_city_gold(f)) } else { String::new() };
                        let shared = if state_faith == Some(f) { ", our state religion: +1 content (+2 with a Temple)" } else { "" };
                        format!("Faith: {}{holy}{shared}", f.name())
                    }
                    None => "Faith: none yet (faiths spread from nearby cities, or a Missionary brings one)".to_string(),
                };
                p.spawn(label(&art.font_body, faith_text, 14.0, CREAM));
                if city.resistance > 0 {
                    p.spawn(label(&art.font_bold, format!("Resisting our rule: produces nothing for {} more turns", city.resistance), 14.0, Color::srgb(1.0, 0.45, 0.4)));
                }
                let merchants = great.merchants.get(&city.position).copied().unwrap_or(0);
                if merchants > 0 {
                    p.spawn(label(&art.font_body, format!("Trading houses: {merchants} (+{} gold)", great.merchant_gold(city.position)), 14.0, CREAM));
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
            // How a click on a project acts (start it now, or add it to the queue), and the queue itself.
            p.spawn(row(6.0)).with_children(|r| {
                r.spawn(label(&art.font_body, "Click to:", 14.0, CREAM));
                button(r, art, "Build now", Command::ToggleQueueMode, if ui.queue_mode { BRONZE } else { GREEN }, 13.0);
                button(r, art, "Add to queue", Command::ToggleQueueMode, if ui.queue_mode { GREEN } else { BRONZE }, 13.0);
            });
            if !city.queue.is_empty() {
                p.spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(5.0), row_gap: Val::Px(4.0), align_items: AlignItems::Center, ..default() }).with_children(|r| {
                    r.spawn(label(&art.font_body, "Then:", 14.0, CREAM));
                    for (i, item) in city.queue.iter().enumerate() {
                        button(r, art, format!("{}. {} x", i + 1, item.name()), Command::Game(Action::Unqueue(i)), BRONZE, 12.0);
                    }
                });
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
                    let order = if ui.queue_mode { Action::QueueProduction(item.clone()) } else { Action::SetProduction(item.clone()) };
                    production_button(grid, art, item.name(), second, Command::Game(order), tint);
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
        let ended = state.victory.is_some() && !ui.end_dismissed;
        node.display = if ended || ui.modal.is_some() || (foreign.offer.is_some() && state.victory.is_none()) { Display::Flex } else { Display::None };
        commands.entity(root).despawn_descendants().with_children(|p| {
            let window = |width: f32| {
                panel(
                    &art.parchment,
                    Node { width: Val::Px(width), max_width: Val::Percent(96.0), padding: UiRect::all(Val::Px(26.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() },
                )
            };
            // A waiting proposal takes priority over the advisors.
            // The game is over: the outcome, the final standing, and where to go next.
            if let Some(outcome) = state.victory.as_ref().filter(|_| !ui.end_dismissed) {
                p.spawn(window(560.0)).with_children(|w| {
                    w.spawn(label(&art.font_title, "The game is over", 28.0, INK));
                    w.spawn(label(&art.font_bold, outcome.clone(), 20.0, INK));
                    w.spawn(label(&art.font_body, format!("Turn {} ({}). Our score: {}.", state.turn, year_label(&state), state.score.total), 15.0, INK_SOFT));
                    let mut standing: Vec<(String, u32)> = std::iter::once(PLAYER)
                        .chain(civs.players.iter().filter(|c| !c.is_human && foreign.met.contains(&c.id)).map(|c| c.id))
                        .map(|id| {
                            let name = if id == PLAYER { "Us".to_string() } else { civs.players.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_default() };
                            let score = history.scores.iter().rev().find(|(_, c, _)| *c == id).map(|(_, _, s)| *s).unwrap_or(0);
                            (name, score)
                        })
                        .collect();
                    standing.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
                    for (name, score) in standing {
                        w.spawn(label(&art.font_body, format!("{name}: {score}"), 14.0, INK));
                    }
                    w.spawn(row(6.0)).with_children(|r| {
                        button(r, art, "Look around the map", Command::DismissEnd, BRONZE, 14.0);
                        button(r, art, "Save to a file", Command::Game(Action::Save), BRONZE, 14.0);
                        button(r, art, "Back to the start menu", Command::QuitToMenu, GREEN, 14.0);
                    });
                });
                return;
            }
            if let Some(offer) = foreign.offer.as_ref().filter(|_| state.victory.is_none()) {
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
                Modal::Empire => {
                    p.spawn(window(760.0)).with_children(|w| {
                        w.spawn(label(&art.font_title, "Empire Advisor", 26.0, INK));
                        w.spawn(Node { column_gap: Val::Px(4.0), ..default() }).with_children(|r| {
                            for tab in EmpireTab::ALL {
                                let tint = if ui.empire_tab == tab { GOLD } else { BRONZE };
                                button(r, art, tab.name(), Command::EmpireTab(tab), tint, 14.0);
                            }
                        });
                        match ui.empire_tab {
                            EmpireTab::Government => {
                                let status = if state.anarchy_turns > 0 {
                                    format!("Anarchy: {} more turns until we become a {}.", state.anarchy_turns, state.pending_government.map(|g| g.name()).unwrap_or("new state"))
                                } else {
                                    format!("We are a {}.", state.government.name())
                                };
                                w.spawn(label(&art.font_bold, status, 16.0, INK));
                                if let Some(me) = civs.players.iter().find(|c| c.id == PLAYER) {
                                    w.spawn(label(&art.font_body, format!("Our people: {}", character_line(&me.name, &me.adjective)), 14.0, INK_SOFT));
                                }
                                // The faiths of the world.
                                let founded: Vec<String> = bevy_civ::religion::Faith::ALL
                                    .iter()
                                    .filter_map(|f| {
                                        let holy = religions.holy.get(f)?;
                                        let place = city_list.iter().find(|(_, c)| c.position == *holy).map(|(_, c)| c.name.clone()).unwrap_or_else(|| "a lost city".into());
                                        Some(format!("{} ({place}, {} {})", f.name(), religions.followers(*f), if religions.followers(*f) == 1 { "city" } else { "cities" }))
                                    })
                                    .collect();
                                let ours = city_list.iter().find(|(_, c)| c.owner == PLAYER && c.is_capital).and_then(|(_, c)| religions.of(c.position));
                                let faith_text = match ours {
                                    Some(f) => format!("State religion: {} (our capital's faith). Faiths of the world: {}.", f.name(), founded.join("; ")),
                                    None if founded.is_empty() => "No faith has been founded yet: the first to learn Mysticism, Philosophy, Theology or Printing Press founds one.".to_string(),
                                    None => format!("Our capital follows no faith yet. Faiths of the world: {}.", founded.join("; ")),
                                };
                                w.spawn(label(&art.font_body, faith_text, 14.0, INK_SOFT));
                                let war = state.war_turns.get(&PLAYER).copied().unwrap_or(0);
                                if war > 0 {
                                    let weary = state.government.war_weariness(war);
                                    w.spawn(label(
                                        &art.font_body,
                                        format!("War weariness: {war} {} of war; {weary} more unhappy in every city (Republic and Democracy tire of war; a Police Station halves it).", if war == 1 { "turn" } else { "turns" }),
                                        14.0,
                                        if weary > 0 { RED } else { INK_SOFT },
                                    ));
                                }
                                w.spawn(label(&art.font_body, format!("A revolution costs {} turns of anarchy: no science, poor production.", Government::REVOLUTION_TURNS), 14.0, INK_SOFT));
                                // Rates: how the empire's commerce is split between science, luxury and taxes.
                                let cap = state.government.science_cap();
                                w.spawn(label(&art.font_bold, format!("Rates (a {} allows at most {cap}% in one)", state.government.name()), 15.0, INK));
                                let (sci, lux) = (state.science_rate, state.luxury_rate);
                                let rows: [(&str, u32, Option<(u32, u32)>, Option<(u32, u32)>); 3] = [
                                    ("Science", sci, (sci >= 10).then(|| (sci - 10, lux)), (sci + 10 <= cap && sci + lux + 10 <= 100).then(|| (sci + 10, lux))),
                                    ("Luxury", lux, (lux >= 10).then(|| (sci, lux - 10)), (lux + 10 <= cap && sci + lux + 10 <= 100).then(|| (sci, lux + 10))),
                                    ("Taxes", state.tax_rate(), None, None),
                                ];
                                for (name, value, less, more) in rows {
                                    w.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                                        r.spawn(Node { width: Val::Px(150.0), ..default() }).with_children(|c| {
                                            c.spawn(label(&art.font_bold, format!("{name} {value}%"), 15.0, INK));
                                        });
                                        for (text, change) in [("-10%", less), ("+10%", more)] {
                                            r.spawn(Node { width: Val::Px(64.0), ..default() }).with_children(|slot| {
                                                if let Some((s, l)) = change {
                                                    button(slot, art, text, Command::Game(Action::SetRates(s, l)), BRONZE, 13.0);
                                                }
                                            });
                                        }
                                        if name == "Taxes" {
                                            r.spawn(label(&art.font_body, "the rest; every 2 luxury make a citizen happy", 13.0, INK_SOFT));
                                        }
                                    });
                                }
                                w.spawn(label(
                                    &art.font_body,
                                    format!("Interest on the treasury: +{} gold a turn (5% of savings, up to 2 for each Bank and 3 for each Stock Exchange).", state.interest),
                                    14.0,
                                    INK_SOFT,
                                ));
                                for g in Government::CHOICES {
                                    let known = g.available(&tech);
                                    w.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                                        r.spawn(Node { width: Val::Px(150.0), ..default() }).with_children(|c| {
                                            c.spawn(label(&art.font_bold, g.name(), 15.0, if known { INK } else { INK_SOFT }));
                                        });
                                        r.spawn(Node { width: Val::Px(380.0), ..default() }).with_children(|c| {
                                            c.spawn(label(&art.font_body, g.summary(), 13.0, INK_SOFT));
                                        });
                                        if g == state.government {
                                            r.spawn(label(&art.font_bold, "current", 13.0, INK));
                                        } else if !known {
                                            let needs = g.required_tech().map(|t| t.name()).unwrap_or("");
                                            r.spawn(label(&art.font_body, format!("needs {needs}"), 13.0, INK_SOFT));
                                        } else if state.anarchy_turns == 0 {
                                            button(r, art, "Revolution", Command::Game(Action::Revolution(g)), BRONZE, 13.0);
                                        }
                                    });
                                }
                            }
                            EmpireTab::Cities => {
                                let mut ours: Vec<_> = city_list.iter().filter(|(_, c)| c.owner == PLAYER).collect();
                                ours.sort_by_key(|(_, c)| std::cmp::Reverse(c.population));
                                if ours.is_empty() {
                                    w.spawn(label(&art.font_body, "We have no cities.", 14.0, INK));
                                }
                                for (entity, city) in ours {
                                    let Some((_, buildings, happiness)) = cities.iter().find(|(c, _, _)| c.position == city.position) else { continue };
                                    let making = city.production_queue.as_ref().map(|i| i.name().to_string()).unwrap_or_else(|| "nothing!".into());
                                    let mood = if happiness.in_disorder() { "DISORDER" } else if spies.in_unrest(city.position) { "unrest" } else { "calm" };
                                    let text = format!(
                                        "{} (size {}) - {:+} food, building {} - {} happy, {} unhappy, {}",
                                        city.name,
                                        city.population,
                                        city.food_balance(buildings),
                                        making,
                                        happiness.happy,
                                        happiness.unhappy,
                                        mood
                                    );
                                    let tint = if city.production_queue.is_none() || happiness.in_disorder() { Color::srgb(0.95, 0.6, 0.5) } else { BRONZE };
                                    w.spawn(Node { ..default() }).with_children(|r| button(r, art, text, Command::Game(Action::SelectCity(entity)), tint, 13.0));
                                }
                            }
                            EmpireTab::Military => {
                                let mut counts: Vec<(UnitType, usize)> = UnitType::ALL
                                    .into_iter()
                                    .map(|k| (k, units.iter().filter(|u| u.owner == PLAYER && u.unit_type == k).count()))
                                    .filter(|(_, n)| *n > 0)
                                    .collect();
                                counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                                let military = units.iter().filter(|u| u.owner == PLAYER && u.unit_type.is_military()).count() as u32;
                                let own_cities = city_list.iter().filter(|(_, c)| c.owner == PLAYER).count() as u32;
                                let free = state.government.free_unit_support() * own_cities.max(1);
                                let upkeep = military.saturating_sub(free) * state.government.unit_upkeep_food();
                                w.spawn(label(&art.font_bold, format!("{military} military units; {free} supported free; upkeep {upkeep} gold a turn."), 15.0, INK));
                                w.spawn(row(6.0)).with_children(|r| {
                                    for (kind, n) in counts {
                                        r.spawn(label(&art.font_body, format!("{} x{n}", kind.name()), 14.0, INK));
                                    }
                                });
                                let wounded = units.iter().filter(|u| u.owner == PLAYER && u.hp < u.max_hp()).count();
                                let veterans = units.iter().filter(|u| u.owner == PLAYER && u.level > 0).count();
                                w.spawn(label(&art.font_body, format!("{veterans} veterans or elite; {wounded} wounded (they heal while resting, fully in a city)."), 14.0, INK_SOFT));
                            }
                            EmpireTab::Score => {
                                let mut civ_ids: Vec<u32> = vec![PLAYER];
                                civ_ids.extend(civs.players.iter().filter(|c| !c.is_human && foreign.met.contains(&c.id)).map(|c| c.id));
                                let latest = |id: u32| history.scores.iter().rev().find(|(_, c, _)| *c == id).map(|(_, _, s)| *s).unwrap_or(0);
                                let best = civ_ids.iter().map(|id| latest(*id)).max().unwrap_or(1).max(1);
                                w.spawn(label(&art.font_body, format!("Turn {} of {}: if nobody wins sooner, the highest score wins then.", state.turn, state.last_turn), 14.0, INK_SOFT));
                                for id in civ_ids {
                                    let name = if id == PLAYER { "Us".to_string() } else { civs.players.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_default() };
                                    let score = latest(id);
                                    let then = history.scores.iter().filter(|(t, c, _)| *c == id && *t + 20 <= state.turn).last().map(|(_, _, s)| *s);
                                    let trend = then.map(|t| format!(" ({:+} in 20 turns)", score as i64 - t as i64)).unwrap_or_default();
                                    w.spawn(label(&art.font_bold, format!("{name}: {score}{trend}"), 14.0, owner_color(id).mix(&Color::BLACK, 0.35)));
                                    bar(w, score as f32 / best as f32, owner_color(id));
                                }
                                for (owner, ship) in &space.ships {
                                    let who = if *owner == PLAYER { "Our".to_string() } else { format!("{}'s", owner_name(*owner, &civs)) };
                                    let state_text = match ship.arrives {
                                        Some(turn) => format!("in flight, arriving on turn {turn}"),
                                        None => format!("being built: still needs {}", ship.missing()),
                                    };
                                    w.spawn(label(&art.font_body, format!("{who} spaceship: {state_text}."), 14.0, INK));
                                }
                            }
                        }
                        w.spawn(row(6.0)).with_children(|r| button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0));
                    });
                }
                Modal::Game => {
                    p.spawn(window(420.0)).with_children(|w| {
                        w.spawn(label(&art.font_title, "Game", 26.0, INK));
                        w.spawn(label(&art.font_body, format!("Turn {}. The game also saves itself at the end of every turn; Continue on the start menu picks it up.", state.turn), 14.0, INK_SOFT));
                        w.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), ..default() }).with_children(|c| {
                            c.spawn(Node::default()).with_children(|r| button(r, art, "Save to a file (F2)", Command::Game(Action::Save), GREEN, 15.0));
                            if SoundSettings::AVAILABLE {
                                c.spawn(Node::default()).with_children(|r| sound_button(r, art));
                            }
                            c.spawn(Node::default()).with_children(|r| button(r, art, "Encyclopedia (F1)", Command::Toggle(Modal::Help), BRONZE, 15.0));
                            let tips_label = if tips_on.enabled { "Tips for new players: On" } else { "Tips for new players: Off" };
                            c.spawn(Node::default()).with_children(|r| button(r, art, tips_label, Command::ToggleTips, BRONZE, 15.0));
                            c.spawn(Node::default()).with_children(|r| button(r, art, "Quit to the start menu", Command::QuitToMenu, Color::srgb(0.9, 0.6, 0.5), 15.0));
                        });
                        w.spawn(row(6.0)).with_children(|r| button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0));
                    });
                }
                Modal::Help => {
                    p.spawn(window(800.0)).with_children(|w| {
                        w.spawn(label(&art.font_title, "Encyclopedia", 26.0, INK));
                        w.spawn(Node { column_gap: Val::Px(4.0), flex_wrap: FlexWrap::Wrap, row_gap: Val::Px(4.0), ..default() }).with_children(|r| {
                            for topic in HelpTopic::ALL {
                                let tint = if ui.help_topic == topic { GOLD } else { BRONZE };
                                button(r, art, topic.name(), Command::HelpTopic(topic), tint, 14.0);
                            }
                        });
                        for line in help_lines(ui.help_topic) {
                            w.spawn(label(&art.font_body, line, 13.0, INK));
                        }
                        w.spawn(row(6.0)).with_children(|r| button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0));
                    });
                }
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
                        // The whole tree, era by era: known in green, open to study in ink, the rest with what they need.
                        w.spawn(label(&art.font_title, "The tree of knowledge", 16.0, INK_SOFT));
                        let eras: [(&str, u32, u32); 4] = [("Ancient", 0, 25), ("Classical", 26, 45), ("Industrial", 46, 95), ("Modern", 96, u32::MAX)];
                        for (era, low, high) in eras {
                            w.spawn(Node { column_gap: Val::Px(8.0), flex_wrap: FlexWrap::Wrap, row_gap: Val::Px(2.0), ..default() }).with_children(|r| {
                                r.spawn(Node { width: Val::Px(90.0), ..default() }).with_children(|c| {
                                    c.spawn(label(&art.font_bold, era, 13.0, INK));
                                });
                                for t in Tech::ALL.into_iter().filter(|t| (low..=high).contains(&t.cost())) {
                                    let (text, color) = if tech.researched.contains(&t) {
                                        (t.name().to_string(), Color::srgb(0.2, 0.5, 0.2))
                                    } else if tech.can_research(t) {
                                        (format!("{} (open)", t.name()), INK)
                                    } else {
                                        let needs: Vec<&str> = t.prerequisites().iter().filter(|p| !tech.researched.contains(p)).map(|p| p.name()).collect();
                                        (format!("{} (needs {})", t.name(), needs.join(", ")), Color::srgb(0.55, 0.5, 0.45))
                                    };
                                    r.spawn(label(&art.font_body, text, 12.0, color));
                                }
                            });
                        }
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
                        // Trade routes opened by our caravans (and foreign ones that end in our cities).
                        let city_name = |h: hexx::Hex| cities.iter().find(|(c, _, _)| c.position == h).map(|(c, _, _)| c.name.clone()).unwrap_or_default();
                        let ours: Vec<_> = trade.routes.iter().filter(|r| r.owner == PLAYER || r.partner == PLAYER).collect();
                        let (gold, science) = trade.income(PLAYER);
                        w.spawn(label(&art.font_title, format!("Trade routes: +{gold} gold, +{science} science a turn"), 17.0, INK));
                        if ours.is_empty() {
                            w.spawn(label(&art.font_body, "None yet. Train a Caravan, move it into another city (yours at least 6 tiles away, or a foreign city at peace) and press T. Longer routes between bigger cities pay more; foreign ones half again as much.", 14.0, INK_SOFT));
                        }
                        for route in ours {
                            let text = if route.owner == PLAYER {
                                format!("{} - {}: +{} gold, +{} science", city_name(route.from), city_name(route.to), route.gold_per_turn, route.science_per_turn)
                            } else {
                                format!("{} - {} (their caravan): +{} gold", city_name(route.from), city_name(route.to), route.gold_per_turn / 2)
                            };
                            w.spawn(label(&art.font_body, text, 14.0, INK));
                        }
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
                                    let embassy = if spies.has_embassy(PLAYER, civ.id) {
                                        let lacking = espionage::stealable(&tech, &civ.tech).len();
                                        format!(
                                            "Embassy: they know {} technologies ({lacking} we lack) and are researching {}.",
                                            civ.tech.researched.len(),
                                            civ.tech.current.map(|t| t.name()).unwrap_or("nothing")
                                        )
                                    } else {
                                        "No embassy: a Diplomat at one of their cities can open one.".to_string()
                                    };
                                    c.spawn(label(&art.font_body, embassy, 14.0, INK_SOFT));
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
                                    if treaty == Treaty::Peace && !diplomacy.has_passage(PLAYER, civ.id) {
                                        button(r, art, "Right of passage", ask(DiploAct::Passage), BRONZE, 14.0);
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
    ui: Res<UiState>,
    golden: Res<crate::GoldenAges>,
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
            StatText::Government => {
                let golden = golden.turns_left.get(&PLAYER).map(|t| format!(" - golden age ({t})")).unwrap_or_default();
                format!("{}{golden}", state.government.name())
            }
            StatText::Score => state.score.total.to_string(),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    if let Ok(mut text) = victory.get_single_mut() {
        // The outcome stays across the map once the end-of-game window is put away.
        let value = state.victory.clone().filter(|_| ui.end_dismissed).unwrap_or_default();
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
pub fn fit_ui_scale(
    windows: Query<&Window>,
    mut scale: ResMut<UiScale>,
    mut minimap: Query<&mut Node, With<MinimapNode>>,
    mut left: Query<&mut Node, (With<LeftColumn>, Without<MinimapNode>)>,
) {
    let Ok(window) = windows.get_single() else { return };
    // Narrow (phone) screens get a bigger interface than a straight fit would give: panels wrap and
    // stack instead, and text stays readable.
    let target = (window.width() / 520.0).clamp(0.6, 1.0);
    if (scale.0 - target).abs() > 0.01 {
        scale.0 = target;
    }
    // On a phone the minimap gives way, and the unit panel narrows, so the bottom panels sit side by side.
    let narrow = window.width() < 640.0;
    let minimap_display = if narrow { Display::None } else { Display::Flex };
    for mut node in &mut minimap {
        if node.display != minimap_display {
            node.display = minimap_display;
        }
    }
    let width = Val::Px(if narrow { 290.0 } else { 360.0 });
    for mut node in &mut left {
        if node.width != width {
            node.width = width;
        }
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
    (mut sounds, mut sound, mut tips): (EventWriter<Sfx>, ResMut<SoundSettings>, ResMut<super::tips::Tips>),
) {
    for (interaction, command) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        sounds.send(Sfx::Click);
        match &command.0 {
            Command::ToggleSound => sound.toggle(),
            Command::CityTab(tab) => ui.city_tab = *tab,
            Command::EmpireTab(tab) => ui.empire_tab = *tab,
            Command::HelpTopic(topic) => ui.help_topic = *topic,
            Command::ToggleGroupMove => ui.group_move = !ui.group_move,
            Command::ToggleQueueMode => ui.queue_mode = !ui.queue_mode,
            Command::TipDone => tips.done(),
            Command::TipsOff => tips.set_enabled(false),
            Command::ToggleTips => {
                let on = !tips.enabled;
                tips.set_enabled(on);
            }
            Command::DismissEnd => ui.end_dismissed = true,
            Command::Game(action) => {
                actions.send(action.clone());
            }
            Command::Toggle(modal) => ui.modal = if ui.modal == Some(*modal) { None } else { Some(*modal) },
            Command::CloseModal => ui.modal = None,
            Command::SelectCiv(civ) => ui.diplo_target = Some(*civ),
            Command::PickScenario(index) => menu.scenario = Some(*index),
            Command::BackToScenarios => {
                menu.scenario = None;
                menu.civ = None;
            }
            Command::BackToPeoples => menu.civ = None,
            Command::PickDifficulty(level) => menu.difficulty = *level,
            Command::PickLength(turns) => menu.last_turn = *turns,
            Command::StartGame => {
                if let (Some(scenario), Some(civ)) = (menu.scenario.and_then(|i| menu.catalog.get(i)), menu.civ) {
                    let last_turn = if menu.last_turn == 0 { crate::LAST_TURN } else { menu.last_turn };
                    commands.insert_resource(NewGame { scenario: scenario.clone(), civ, difficulty: menu.difficulty, last_turn });
                    menu.civ = None;
                    next.set(AppState::Playing);
                }
            }
            Command::Continue => match crate::savegame::autosave().map(|json| crate::savegame::SaveGame::parse(&json)) {
                Some(Ok(game)) => {
                    commands.insert_resource(crate::savegame::PendingLoad(game));
                    next.set(AppState::Playing);
                }
                Some(Err(e)) => menu.notice = Some(e),
                None => menu.notice = Some("There is no saved game to continue.".into()),
            },
            Command::LoadFile => {
                crate::savegame::pick_file();
                menu.notice = Some("Choose a saved game file (a .json file saved from Bevy Civ)...".into());
            }
            // The people chosen; the options page comes next.
            Command::PickCiv(civ) => menu.civ = Some(*civ),
            Command::QuitToMenu => {
                ui.end_dismissed = false;
                tips.shown.clear();
                tips.done();
                ui.modal = None;
                ui.diplo_target = None;
                menu.scenario = None;
                next.set(AppState::Menu);
            }
        }
    }
}

// ---- start menu --------------------------------------------------------------------------------

/// Picks up a saved game file the player chose and starts it.
pub fn poll_import(mut menu: ResMut<MenuState>, mut next: ResMut<NextState<AppState>>, mut commands: Commands) {
    let Some(json) = crate::savegame::take_import() else { return };
    match crate::savegame::SaveGame::parse(&json) {
        Ok(game) => {
            menu.notice = None;
            commands.insert_resource(crate::savegame::PendingLoad(game));
            next.set(AppState::Playing);
        }
        Err(e) => menu.notice = Some(e),
    }
}

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
                    // Carry on, or load a game saved as a file (yours, or one someone sent you).
                    w.spawn(row(8.0)).with_children(|r| {
                        if crate::savegame::autosave().is_some() {
                            button(r, art, "Continue your last game", Command::Continue, GREEN, 16.0);
                        }
                        button(r, art, "Load a saved game file...", Command::LoadFile, BRONZE, 16.0);
                    });
                    if let Some(notice) = &menu.notice {
                        w.spawn(label(&art.font_bold, notice.clone(), 15.0, INK));
                    }
                    w.spawn(label(&art.font_body, "Or choose a world to rule.", 18.0, INK_SOFT));
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
                Some(scenario) if menu.civ.is_some() => {
                    // The game's options: difficulty and length.
                    let civ = menu.civ.and_then(|i| scenario.civs.get(i));
                    let who = civ.map(|c| format!("{} of {}", c.leader, c.name)).unwrap_or_default();
                    w.spawn(label(&art.font_title, scenario.name.clone(), 24.0, INK));
                    w.spawn(label(&art.font_bold, format!("You lead as {who}."), 16.0, INK_SOFT));
                    if let Some(c) = civ {
                        w.spawn(label(&art.font_body, character_line(&c.name, &c.adjective), 14.0, INK_SOFT));
                    }
                    w.spawn(label(&art.font_title, "Difficulty", 17.0, INK_SOFT));
                    w.spawn(row(6.0)).with_children(|r| {
                        for level in crate::Difficulty::ALL {
                            let tint = if menu.difficulty == level { GREEN } else { BRONZE };
                            button(r, art, level.name(), Command::PickDifficulty(level), tint, 15.0);
                        }
                    });
                    w.spawn(label(&art.font_body, menu.difficulty.describe(), 14.0, INK_SOFT));
                    w.spawn(label(&art.font_title, "Length", 17.0, INK_SOFT));
                    let chosen = if menu.last_turn == 0 { crate::LAST_TURN } else { menu.last_turn };
                    w.spawn(row(6.0)).with_children(|r| {
                        for (turns, name) in [(200, "Short (200 turns)"), (300, "Standard (300 turns)"), (400, "Long (400 turns)")] {
                            let tint = if chosen == turns { GREEN } else { BRONZE };
                            button(r, art, name, Command::PickLength(turns), tint, 15.0);
                        }
                    });
                    w.spawn(label(&art.font_body, "If nobody has won by the last turn, the highest score wins.", 14.0, INK_SOFT));
                    w.spawn(row(8.0)).with_children(|r| {
                        button(r, art, "Back", Command::BackToPeoples, BRONZE, 15.0);
                        button(r, art, "Begin", Command::StartGame, GREEN, 18.0);
                    });
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
