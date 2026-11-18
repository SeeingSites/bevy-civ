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
/// A button that cannot be used just now.
const SPENT: Color = Color::srgb(0.6, 0.57, 0.52);
/// The tree of knowledge: technologies known, on the way to the goal, and the goal itself.
const KNOWN: Color = Color::srgb(0.62, 0.78, 0.55);
const PATH: Color = Color::srgb(0.98, 0.86, 0.5);
const GOAL: Color = Color::srgb(0.95, 0.6, 0.3);
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
    /// Every message of the game so far (or only the great events).
    Messages,
}

/// Pages of the empire advisor.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EmpireTab {
    #[default]
    Government,
    Cities,
    Military,
    Score,
    Space,
}

impl EmpireTab {
    const ALL: [EmpireTab; 5] = [EmpireTab::Government, EmpireTab::Cities, EmpireTab::Military, EmpireTab::Score, EmpireTab::Space];

    fn name(self) -> &'static str {
        match self {
            EmpireTab::Government => "Government",
            EmpireTab::Cities => "Cities",
            EmpireTab::Military => "Military",
            EmpireTab::Score => "Score",
            EmpireTab::Space => "Space",
        }
    }
}

/// Topics of the encyclopedia.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum HelpTopic {
    #[default]
    Start,
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
    const ALL: [HelpTopic; 9] = [
        HelpTopic::Start,
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
            HelpTopic::Start => "How to play",
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
        HelpTopic::Start => [
            "You lead a people from its first villages to the space age. Everything happens in turns: you give your orders - move units, choose what cities build and what to research - then press End Turn (Enter), and every other people takes its turn too.",
            "The aim: be the last people standing, win the race to Alpha Centauri, outshine everyone in culture, be elected by the United Nations - or simply have the highest score when the last turn comes (the Empire window, Y, keeps the score).",
            "1. Found cities. Cities are everything: they grow, build units and buildings, and produce the gold and science that drive the rest. Move a Settler at least two tiles from other cities - rivers, coasts and grassland make good sites - and press B.",
            "2. Choose what each city builds. Click a city to open it: Units, Buildings and Wonders tabs list what it can make, with the turns each takes. Early on, a Warrior or Spearman to guard each city, a Worker, and Settlers to spread out are the usual start.",
            "3. Improve the land. Workers build farms (F, more food, so cities grow), mines (M, more shields, so cities build faster) and roads (R, faster travel and trade). Press K and a worker does it all by itself.",
            "4. Research. Press Q to choose what your people learn next. Each discovery opens new units, buildings and wonders. Click any technology in the tree to make it a goal: your scholars research the way to it, in order, and the tree lights up the path.",
            "5. Explore. The map starts dark. Send a Warrior or Scout out to find good land, tribal villages (with gifts inside) and other peoples. Ships, once you know Seafaring, carry troops across the sea.",
            "6. Keep your people happy. Past a few citizens, a city needs temples, luxuries or entertainers, or it falls into disorder and stops working. The city panel shows content and unhappy faces.",
            "7. Meet your neighbours. Press E to make peace, trade resources and technology, or declare war. Defend your cities - a city with no troops in it is easily taken.",
            "Each turn, the game brings up every unit waiting for orders (a ring marks the one selected) and any city that needs a project. When nothing is left, it is time to end the turn. Short tips appear the first time something new happens; Basics & keys lists every shortcut.",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        HelpTopic::Basics => [
            "Click a unit or city to select it; click again or press Tab (Shift+Tab back) for the next unit on that tile - or pick one from the cards in the unit panel. Click a tile (or right-click) to move; far destinations become standing orders. Hover over a tile to see the route and how many turns it takes.",
            "Space: skip a unit (or go to a city that needs a project, then end the turn). Enter: end the turn. N: next unit. H: fortify or sleep. Z: sentry (sleep until an enemy comes near). J: a worker or settler joins the city it stands in. I: board a ship in port. [ and ]: previous and next city. Ctrl+Z or Backspace: take back the last move. L: messages. Esc: close or deselect.",
            "Workers and Engineers: F farm, M mine, R road (railroad on a road, with Steam Power), O fortress (Construction), X clear land or clean pollution, K automate. Settlers: B found a city.",
            "U uses the selected unit: a great person's gift, a General forming an army, a Missionary preaching, a caravan helping a wonder. T opens a caravan's trade route. P pillages enemy land. Diplomats act from the unit panel next to a foreign city.",
            "Q science, E foreign affairs, G trade, Y empire (government, rates, cities, military, score), F1 this encyclopedia, V sound, F2 save a file (the game also saves itself every turn).",
            "Terrain: hills and forest cost 2 moves, mountains 3. A unit without the moves still enters, but is worn out for as many turns as it owes (a one-move unit takes 2 turns a hill, 3 a mountain). Roads triple movement and railroads make it free. Wheeled units - chariots, catapults, cannon, artillery - cannot enter forest or mountains except along a road.",
            "Ships carry troops: a Galley or Frigate holds 2, a Galleon 4, a Transport 8. Move troops onto a ship of yours with room, or press I (Board ship) in a port where one lies; sail the ship and its troops go with it. To land, select a unit aboard (click the ship's tile again or press Tab) and click a land tile next to the ship - going ashore, like boarding from the shore, takes the rest of the unit's turn. Troops cannot attack from a ship, and go down with it if it sinks.",
            "A Carrier holds 4 aircraft and serves as an airfield: aircraft may fly within 8 tiles of your cities or carriers, and land on a carrier by flying to its tile. Aircraft on a carrier sail with it.",
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
                } else if u == UnitType::Cartel {
                    format!("{}: attack {a}, defence {d}, moves {m}. Modern raiders, not trained: they rise near cities with a drug trade (likelier the worse it is) and no Police Station, keep the police from rooting it out while near, and extort gold from an undefended city. Killing them seizes {} gold.", u.name(), bevy_civ::barbarians::SEIZED_GOLD)
                } else if u == UnitType::Insurgent {
                    format!("{}: attack {a}, defence {d}, moves {m}. A terror cell, not trained: once a people knows Electronics, it forms near a city with grievances (disorder, occupation, a faith not its ruler's, a long war) unless it has a Police Station. In an undefended city it wrecks a building and kills a citizen; police foil half such plots.", u.name())
                } else if u.capacity() > 0 {
                    let kind = if u == UnitType::Carrier { "aircraft" } else { "troops" };
                    format!("{}: attack {a}, defence {d}, moves {m}; carries {} {kind}; {} shields{needs}{res}", u.name(), u.capacity(), u.cost())
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
            "Cities grow when their granary fills and cannot pass size 8 without an Aqueduct, or 16 without a Sewer System. Every citizen eats 2 food; specialists eat without working the land.",
            "Each working citizen works one tile of its city's area, and the city's own tile is worked for free. Open a city to see each tile's food, shields and gold (worked tiles ringed in green); click a tile to move a citizen there or off it, or let the governor choose. Farms, mines, roads, railroads, resources and rivers make tiles richer.",
            "Modern troubles: once a people knows Sanitation, big unhappy cities may fall into a drug trade (up to 3 levels, each +1 unhappy and -1 gold), spread by smugglers from foreign cities nearby - a Police Station roots it out, and you can demand that a neighbour cracks down. With Industrialization, adulterated food may spoil half a granary and anger a city for 6 turns (likelier where corruption is high) unless it has a Food Safety Agency. GM Farms (Genetic Engineering) add a food on every farmed tile, but people distrust the food (+1 unhappy) without a Food Safety Agency.",
            "Migration: every 5 turns, citizens leave unhappy, hungry or troubled cities for clearly better ones within 10 tiles - at home, or abroad in peoples that know Industrialization and welcome them. Set your policy in the Empire window (Government): welcome migrants and grow (their rulers resent it), or turn them away. Communist and fundamentalist peoples admit none.",
            "Each city's governor works the land for a focus, always feeding the city first: Growth (the most food), Production (shields), Trade (gold-rich tiles, for science and taxes) or Balanced. On Auto, the default, it picks one from the city's situation - production for a wonder or for troops in wartime, growth for a small city, trade for a big one with a library or market - and the city panel says why.",
            "Culture from temples, cathedrals, colosseums, libraries, universities and wonders widens a city's borders over time (2 tiles, 3 at 100, 4 at 1,000, 5 at 5,000). Where borders meet, the land goes to the city with the stronger culture for its distance, so a great culture pushes its neighbours' borders back. A city in disorder next to a much greater culture may defect to it.",
            "Corruption grows with distance from the capital and is halved by a Courthouse. A city keeps training the same unit until told otherwise; a finished building or wonder leaves it waiting for its next project - unless its build queue holds more (switch the city panel to Add to queue and pick up to six projects).",
            "Pollution and nuclear fallout foul the land; past 15 fouled tiles in the world, global warming now and then turns land drier. Clean it up with workers; Mass Transit and Recycling Centers prevent it.",
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
            "Armies: a Great General in one of your cities forms up to four land units (five with Sun Tzu's War Academy) into one army. It fights as its best unit with the hit points of all, 10% harder for every member beyond the first, and moves at its slowest member's pace. Troops standing with an army that has room can join it (U).".to_string(),
            "Artillery, some ships and bombers bombard without risk. Troops cannot slip past enemy troops (zone of control). Pillage (P) wrecks improvements on enemy land; computer raiders do the same to yours.".to_string(),
            "A captured city resists for a while (longer for a big one; a garrison of two halves it) and produces nothing. Under a Republic or Democracy a long war makes citizens unhappy (a Police Station halves it).".to_string(),
            "Modern raiders rise inside your own lands, on the barbarians' side. Cartel Gunmen form near a city with a drug trade and no Police Station (likelier the worse it is); while near, its drug trade cannot be rooted out, and an undefended city pays them off - killing them seizes their assets. Insurgents form, once you know Electronics, near a city with grievances (disorder, occupation, a faith not its ruler's, a long war); in an undefended city they wreck a building and kill a citizen. A Police Station stops both forming in its city and foils half of all plots; a garrison keeps them out.".to_string(),
            "Older units are upgraded in your cities for gold once the better unit can be built there. Under Monarchy, Republic, Communism or Fundamentalism, once you know Gunpowder, a city can draft a citizen into its best defender once a turn.".to_string(),
            "Air: enemy fighters that have not flown this turn intercept bombers attacking within 3 tiles of them, shooting most down; a bomber that survives is damaged.".to_string(),
            "Nuclear weapons: once anyone has built the Manhattan Project (Nuclear Fission), every people that knows Rocketry can build Nuclear Missiles. Select a missile, click a target within 12 tiles, then click it again to launch: everything within a tile is destroyed, a city there loses half its people and two buildings, fallout spoils the land, the victim goes to war with you and every other people trusts you less. An SDI Defense (Superconductors) shoots down missiles aimed within 3 tiles of its city.".to_string(),
        ],
        HelpTopic::Victory => vec![
            "Conquest: take every rival city. Domination: hold two thirds of the cities in the world (at least five).".to_string(),
            "Diplomacy: once the United Nations (Computers) stands, every 10 turns the world votes for Secretary-General between its owner and the highest-scoring other people. Each candidate votes for itself; every other people backs the candidate it likes better, but only one it is friendly with (peace treaties, alliances, gifts, trade and a shared faith build goodwill) - otherwise it abstains. Two thirds of all the votes win.".to_string(),
            "Space: the race to Alpha Centauri, the nearest star. Research Space Flight, Plastics and Superconductors; cities with a Factory build 4 Space Structures, 2 to 4 Components (engines) and 2 Modules. Launch from the Empire advisor's Space page: the voyage takes 12 turns, 2 fewer for each extra engine. The first ship to arrive wins - unless its capital falls first.".to_string(),
            format!("Culture: reach {} culture points (in a game of {} turns; scaled to the game's length) and twice as much as any rival.", bevy_civ::culture::VICTORY_CULTURE, 400),
            "Score: if nobody has won by the last turn (chosen when the game begins), the highest score wins. Score counts 2 points a citizen, 10 a technology and 50 a wonder, for everyone alike. A rival who holds two thirds of the cities wins by domination.".to_string(),
        ],
    }
}

#[derive(Resource, Default)]
pub struct UiState {
    pub modal: Option<Modal>,
    /// The civilization shown in the foreign advisor window.
    pub diplo_target: Option<u32>,
    /// The foreign advisor's page.
    pub diplo_page: DiploPage,
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
    /// The Messages window: page (0 = the latest), and whether only great events are shown.
    pub messages_page: usize,
    pub highlights_only: bool,
}

/// Pages of the foreign advisor.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DiploPage {
    /// Talk to one leader: treaties, trade, demands.
    #[default]
    Talk,
    /// Who stands where with whom.
    Relations,
    /// What we know of each people.
    Intelligence,
}

impl DiploPage {
    const ALL: [DiploPage; 3] = [DiploPage::Talk, DiploPage::Relations, DiploPage::Intelligence];

    fn name(self) -> &'static str {
        match self {
            DiploPage::Talk => "Talk",
            DiploPage::Relations => "Relations",
            DiploPage::Intelligence => "Intelligence",
        }
    }
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
    /// New World options (None until chosen: normal size, five peoples).
    pub map_size: Option<usize>,
    pub peoples: Option<usize>,
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
    DiploPage(DiploPage),
    ToggleGroupMove,
    ToggleQueueMode,
    ToggleMusic,
    /// Messages window: older (+1) or newer (-1) page.
    MessagesPage(i32),
    ToggleHighlights,
    /// From the end of the game: the chronicle of its great events.
    Chronicle,
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
    PickMapSize(usize),
    PickPeoples(usize),
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

/// The minimap's frame: put away while a city is open, to leave the city panel room.
#[derive(Component)]
pub struct MinimapFrame;

/// The bottom-right panel (minimap, Next Unit, End Turn): the city panel stops above it.
#[derive(Component)]
pub struct TurnPanel;
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

/// A small card for a unit in a stack: its figure, name and a line of status.
fn unit_card(parent: &mut ChildBuilder, art: &Art, unit: &Unit, name: String, status: String, command: Command, tint: Color) {
    parent
        .spawn((
            Button,
            Node { padding: UiRect::new(Val::Px(4.0), Val::Px(9.0), Val::Px(3.0), Val::Px(3.0)), align_items: AlignItems::Center, column_gap: Val::Px(5.0), width: Val::Px(156.0), ..default() },
            ImageNode { image: art.button.clone(), image_mode: sliced(14.0), color: tint, ..default() },
            ButtonTint(tint),
            ButtonCommand(command),
        ))
        .with_children(|card| {
            card.spawn((
                Node { width: Val::Px(34.0), height: Val::Px(34.0), ..default() },
                ImageNode { image: art.units.clone(), texture_atlas: Some(art.atlas(&art.units_layout, super::actors::unit_row(unit.unit_type) * 4)), ..default() },
            ));
            card.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|text| {
                text.spawn(label(&art.font_bold, name, 12.0, INK));
                text.spawn(label(&art.font_body, status, 11.0, INK));
            });
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

/// The status bar across the top of the screen.
#[derive(Component)]
pub struct StatusBar;

/// The status bar's figures (left), which give way before the buttons (right) on a wide screen.
#[derive(Component)]
pub struct StatusFigures;

pub fn setup_ui(mut commands: Commands, art: Res<Art>, mut images: ResMut<Assets<Image>>) {
    // Status bar: the figures on the left, the advisors' buttons on the right. On a wide screen
    // they share one row and the figures are clipped if they run long; on a narrow one the buttons
    // wrap onto a second row (see `fit_ui_scale`).
    commands
        .spawn((
            StatusBar,
            super::layout::Dock::Top,
            panel(
                &art.panel,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    min_height: Val::Px(52.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    row_gap: Val::Px(4.0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
            ),
        ))
        .with_children(|top| {
            top.spawn((
                StatusFigures,
                Node { align_items: AlignItems::Center, column_gap: Val::Px(11.0), row_gap: Val::Px(4.0), flex_wrap: FlexWrap::Wrap, flex_shrink: 1.0, flex_grow: 1.0, min_width: Val::Px(0.0), overflow: Overflow::clip_x(), ..default() },
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
            });
            top.spawn(Node { column_gap: Val::Px(6.0), row_gap: Val::Px(4.0), flex_wrap: FlexWrap::Wrap, flex_shrink: 0.0, max_width: Val::Percent(100.0), ..default() }).with_children(|bar| {
                button(bar, &art, "Science (Q)", Command::Toggle(Modal::Science), BRONZE, 14.0);
                button(bar, &art, "Diplomacy (E)", Command::Toggle(Modal::Diplomacy), BRONZE, 14.0);
                button(bar, &art, "Trade (G)", Command::Toggle(Modal::Trade), BRONZE, 14.0);
                button(bar, &art, "Empire (Y)", Command::Toggle(Modal::Empire), BRONZE, 14.0);
                button(bar, &art, "Help (F1)", Command::Toggle(Modal::Help), BRONZE, 14.0);
                button(bar, &art, "Game", Command::Toggle(Modal::Game), BRONZE, 14.0);
            });
        });

    // Message log stacked on the unit panel (bottom left)
    commands
        .spawn((
            LeftColumn,
            super::layout::Dock::BottomLeft,
            super::layout::DockSize { width: 360.0 },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                width: Val::Px(360.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                row_gap: Val::Px(8.0),
                overflow: Overflow::clip(),
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
                // Shrinks to the room left under the top bar and scrolls (wheel or finger).
                super::layout::Scrollable,
                RelativeCursorPosition::default(),
                ScrollPosition::default(),
                panel(
                    &art.panel,
                    Node {
                        padding: UiRect::all(Val::Px(16.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(9.0),
                        flex_shrink: 1.0,
                        min_height: Val::Px(0.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ),
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
        .spawn((TurnPanel, super::layout::Dock::BottomRight, super::layout::DockSize { width: 212.0 }, panel(
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
        )))
        .with_children(|p| {
            p.spawn((MinimapFrame, Node { padding: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(Color::srgb(0.42, 0.29, 0.11)))).with_children(|frame| {
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
        super::layout::Dock::Right,
        super::layout::DockSize { width: 430.0 },
        // Scrolls (mouse wheel) when it is taller than the room between the status bar and the
        // End Turn panel (the wheel over it does not zoom the map).
        super::layout::Scrollable,
        RelativeCursorPosition::default(),
        ScrollPosition::default(),
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
                overflow: Overflow::scroll_y(),
                display: Display::None,
                ..default()
            },
        ),
    ));

    // Advisor windows (centre)
    commands.spawn((
        ModalRoot,
        super::layout::Dock::Centre,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(74.0),
            // A bottom edge gives windows a height to keep within (they scroll beyond it).
            bottom: Val::Px(12.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexStart,
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
    (foreign, supplies, map, religions, last_move, log, sound_on): (Res<Foreign>, Res<Supplies>, Res<MapData>, Res<bevy_civ::religion::Religions>, Res<crate::LastMove>, Res<crate::GameLog>, Res<SoundSettings>),
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
    (city_list, space, history, culture, taken, tips_on, rivers, web): (
        Query<(Entity, &City)>,
        Res<bevy_civ::space::SpaceRace>,
        Res<crate::History>,
        Res<bevy_civ::culture::CultureLedger>,
        Res<crate::JustCaptured>,
        Res<super::tips::Tips>,
        Res<crate::Rivers>,
        Res<super::relations::RelationsWeb>,
    ),
    unit_panel: Query<Entity, With<UnitPanel>>,
    mut city_panel: Query<(Entity, &mut Node), (With<CityPanel>, Without<ModalRoot>)>,
    mut modal_root: Query<(Entity, &mut Node), (With<ModalRoot>, Without<CityPanel>)>,
    mut commands: Commands,
) {
    if !(dirty.ui || selection.is_changed() || ui.is_changed() || foreign.is_changed() || supplies.is_changed() || tips_on.is_changed() || sound_on.is_changed()) {
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
                let capacity = bevy_civ::unit::army_capacity(wonders.owns(unit.owner, Building::SunTzu));
                let bonus = (bevy_civ::unit::ARMY_MEMBER_BONUS * (unit.members() - 1) as f32 * 100.0).round();
                p.spawn(label(
                    &art.font_body,
                    format!(
                        "{} - {} of {capacity} units: fights as its best unit, {bonus}% harder, with the hit points of all.{}",
                        unit.display_name(),
                        unit.members(),
                        if (unit.members() as usize) < capacity && unit.owner == PLAYER { " Troops on its tile can join it (U)." } else { "" }
                    ),
                    14.0,
                    CREAM,
                ));
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
                        format!("Unit {} of {} on this tile - Tab / Shift+Tab or click a card:", index + 1, stack.len()),
                        14.0,
                        GOLD,
                    ));
                    let together = if ui.group_move { "Move together: On" } else { "Move together: Off (or Shift-click)" };
                    p.spawn(row(6.0)).with_children(|r| {
                        button(r, art, together, Command::ToggleGroupMove, if ui.group_move { GREEN } else { BRONZE }, 13.0);
                        button(r, art, "Fortify all", Command::Game(Action::StackOrder(true)), BRONZE, 13.0);
                        button(r, art, "Wake all", Command::Game(Action::StackOrder(false)), BRONZE, 13.0);
                    });
                    // One card per unit: its figure, health, and whether it still has orders to give.
                    p.spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(5.0), row_gap: Val::Px(5.0), max_height: Val::Px(190.0), overflow: Overflow::clip_y(), ..default() }).with_children(|grid| {
                        for entity in &stack {
                            let Ok(other) = units.get(*entity) else { continue };
                            let state = if other.build_turns_left.is_some() {
                                "working"
                            } else if other.carrier.is_some() && map.water.contains(&other.position) {
                                "aboard"
                            } else if other.fortified {
                                "fortified"
                            } else if !other.can_move() {
                                "done"
                            } else {
                                "ready"
                            };
                            let tint = if selection.unit == Some(*entity) { GREEN } else if state == "ready" { BRONZE } else { SPENT };
                            let name = if other.is_army_group() { format!("Army of {}", other.members()) } else { other.unit_type.name().to_string() };
                            unit_card(grid, art, other, name, format!("{}/{} HP - {state}", other.hp, other.max_hp()), Command::Game(Action::Select(*entity)), tint);
                        }
                    });
                }
            }
            // Ships and their cargo.
            let berths: Vec<bevy_civ::cargo::Berth> = unit_entities.iter().filter_map(|e| units.get(e).ok().map(|u| bevy_civ::cargo::Berth::of(e, u))).collect();
            let at_sea = map.water.contains(&unit.position);
            let in_port = cities.iter().any(|(c, _, _)| c.position == unit.position && c.owner == PLAYER);
            let mut can_board = false;
            let mut can_leave = false;
            if mine && unit.unit_type.capacity() > 0 {
                let cargo: Vec<&str> = berths.iter().filter(|b| b.carrier.map(|c| c.0) == selection.unit).map(|b| b.unit_type.name()).collect();
                let kind = if unit.unit_type == UnitType::Carrier { "aircraft" } else { "troops" };
                let text = if cargo.is_empty() {
                    format!("Carries up to {} {kind}; none aboard. Move {kind} onto it to board{}.", unit.unit_type.capacity(), if kind == "troops" { " (or Board in port)" } else { "" })
                } else {
                    format!("Aboard ({} of {}): {}.", cargo.len(), unit.unit_type.capacity(), cargo.join(", "))
                };
                p.spawn(label(&art.font_body, text, 13.0, MUTED));
                can_leave = in_port && !cargo.is_empty();
            }
            if mine {
                if let Some((ship, _)) = unit.carrier {
                    let ship_name = units.get(ship).map(|s| s.unit_type.name()).unwrap_or("ship");
                    let text = if unit.unit_type.domain() == bevy_civ::unit::Domain::Air {
                        format!("Based on the {ship_name}: it sails with the ship, and can strike anything within {} tiles of it.", bevy_civ::unit::AIR_RANGE)
                    } else if at_sea {
                        format!("Aboard the {ship_name}. Select it on this tile and click a land tile next to the ship to go ashore (that takes the rest of the turn).")
                    } else {
                        format!("Aboard the {ship_name}, in port: it sails with the ship.")
                    };
                    p.spawn(label(&art.font_body, text, 13.0, MUTED));
                    can_leave = !at_sea;
                } else if in_port && unit.unit_type.domain() != bevy_civ::unit::Domain::Sea {
                    can_board = bevy_civ::cargo::ship_with_room(&berths, PLAYER, unit.position, unit.unit_type).is_some();
                }
            }
            if mine {
                p.spawn(row(6.0)).with_children(|r| {
                    if can_board {
                        button(r, art, "Board ship (I)", Command::Game(Action::Board), GREEN, 15.0);
                    }
                    if can_leave {
                        button(r, art, if unit.carrier.is_some() { "Leave ship" } else { "Unload all" }, Command::Game(Action::Unload), BRONZE, 15.0);
                    }
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
                    // Ground troops can join an army of ours on their tile that has room.
                    let capacity = bevy_civ::unit::army_capacity(wonders.owns(PLAYER, Building::SunTzu));
                    let army_here = unit_entities.iter().filter(|e| Some(*e) != selection.unit).filter_map(|e| units.get(e).ok()).any(|a| {
                        a.owner == PLAYER && a.position == unit.position && a.is_army_group() && (a.members() as usize) < capacity
                    });
                    if bevy_civ::unit::can_join_army(unit) && army_here {
                        button(r, art, "Join Army (U)", Command::Game(Action::UseUnit), GREEN, 15.0);
                    }
                    if unit.unit_type == UnitType::GreatGeneral {
                        button(r, art, "Form Army (U)", Command::Game(Action::UseUnit), GREEN, 15.0);
                    } else if unit.unit_type.is_great_person() {
                        button(r, art, "Use (U)", Command::Game(Action::UseUnit), GREEN, 15.0);
                    }
                    if !unit.fortified {
                        let text = if unit.unit_type.is_military() { "Fortify (H)" } else { "Sleep (H)" };
                        button(r, art, text, Command::Game(Action::Fortify), BRONZE, 15.0);
                        if unit.unit_type.is_military() {
                            button(r, art, "Sentry (Z)", Command::Game(Action::Sentry), BRONZE, 15.0);
                        }
                    }
                    let in_own_city = cities.iter().any(|(c, _, _)| c.position == unit.position && c.owner == PLAYER);
                    if in_own_city && matches!(unit.unit_type, UnitType::Worker | UnitType::Engineers | UnitType::Settler) {
                        button(r, art, "Join City (J)", Command::Game(Action::JoinCity), BRONZE, 15.0);
                    }
                    if last_move.0.as_ref().is_some_and(|(e, _)| Some(*e) == selection.unit) {
                        button(r, art, "Undo move (Ctrl+Z)", Command::Game(Action::Undo), BRONZE, 15.0);
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
        // Seats of government (capital and Forbidden Palace): corruption counts from the nearer.
        let seats: Vec<hexx::Hex> = cities.iter().filter(|(c, b, _)| c.owner == PLAYER && (c.is_capital || b.has(Building::ForbiddenPalace))).map(|(c, _, _)| c.position).collect();
        let has_forbidden_palace = cities.iter().any(|(c, b, _)| c.owner == PLAYER && b.has(Building::ForbiddenPalace));
        commands.entity(root).despawn_descendants().with_children(|p| {
            let Some((city, buildings, happiness)) = city else { return };
            let mine = city.owner == PLAYER;
            p.spawn(Node { justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|r| {
                r.spawn(label(&art.font_title, city.name.clone(), 23.0, GOLD));
                r.spawn(Node { column_gap: Val::Px(5.0), align_items: AlignItems::Center, ..default() }).with_children(|nav| {
                    // Step through our cities without going back to the map.
                    if mine {
                        let mut ours: Vec<(Entity, hexx::Hex)> = city_list.iter().filter(|(_, c)| c.owner == PLAYER).map(|(e, c)| (e, c.position)).collect();
                        ours.sort_by_key(|(e, _)| e.index());
                        if ours.len() > 1 {
                            let index = ours.iter().position(|(_, at)| *at == city.position).unwrap_or(0);
                            button(nav, art, "< [", Command::Game(Action::CycleCity(false)), BRONZE, 13.0);
                            nav.spawn(label(&art.font_body, format!("{} of {}", index + 1, ours.len()), 13.0, CREAM));
                            button(nav, art, "] >", Command::Game(Action::CycleCity(true)), BRONZE, 13.0);
                        }
                    }
                    button(nav, art, "Close", Command::Game(Action::Deselect), BRONZE, 13.0);
                });
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
            let dist = seats.iter().map(|s| city.position.distance_to(*s) as u32).min().or(capital.map(|c| city.position.distance_to(c) as u32)).unwrap_or(0);
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
            // The land: what the city's tile and its worked tiles give, and who works them.
            if mine {
                p.spawn(row(6.0)).with_children(|r| {
                    let (f, s, g) = city.land;
                    r.spawn(label(
                        &art.font_body,
                        format!(
                            "Land: {} of {} citizens work tiles ({f} food, {s} shields, {g} gold; {} eaten). Click a tile on the map to move a citizen.",
                            city.worked.len(),
                            city.population,
                            city.population * bevy_civ::tiles::FOOD_PER_CITIZEN
                        ),
                        13.0,
                        MUTED,
                    ));
                    button(r, art, "Governor picks tiles", Command::Game(Action::AutoTiles), BRONZE, 12.0);
                });
                // The governor's focus: what it works the land for, and on Auto what it chose and why.
                let at_war = diplomacy.treaties.iter().any(|((a, b), t)| *t == Treaty::War && (*a == PLAYER || *b == PLAYER));
                let situation = crate::city_situation(city, buildings, Some(happiness), at_war, &rivers);
                let (suggested, why) = bevy_civ::tiles::suggest(&situation);
                p.spawn(row(5.0)).with_children(|r| {
                    r.spawn(label(&art.font_bold, "Focus:", 13.0, CREAM));
                    for focus in bevy_civ::tiles::Focus::ALL {
                        let tint = if city.focus == focus { GREEN } else { BRONZE };
                        button(r, art, focus.name(), Command::Game(Action::SetFocus(focus)), tint, 12.0);
                    }
                });
                let note = if city.focus == bevy_civ::tiles::Focus::Auto {
                    format!("Auto: working for {} - {why}.", suggested.name())
                } else if city.focus == suggested {
                    format!("The governor agrees: {why}.")
                } else {
                    format!("The governor would suggest {}: {why}.", suggested.name())
                };
                p.spawn(label(&art.font_body, note, 13.0, MUTED));
            }
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
            // The draft: a citizen into the army at once.
            if mine && state.government.allows_draft() && tech.researched.contains(&Tech::Gunpowder) && city.population >= 2 && !state.drafted.contains(&city.position) {
                p.spawn(row(6.0)).with_children(|r| {
                    button(r, art, "Draft a defender (-1 citizen)", Command::Game(Action::Draft), Color::srgb(0.9, 0.6, 0.5), 13.0);
                });
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
                    .filter(|u| u.required_wonder().is_none_or(|w| wonders.built.contains_key(&w)))
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
                        .filter(|b| !b.is_spaceship_part() || (buildings.has(Building::Factory) && space.wants(PLAYER, *b, true)))
                        .filter(|b| b.required_building().is_none_or(|r| buildings.has(r)))
                        .filter(|b| !(b.one_per_people() && has_forbidden_palace))
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
                // The troubles of the modern city.
                let mut troubles = Vec::new();
                if city.narcotics > 0 {
                    let cure = if buildings.has(Building::PoliceStation) { "the police are rooting it out" } else { "a Police Station would root it out" };
                    troubles.push(format!("a drug trade (level {} of {}: +{} unhappy, -{} gold; {cure})", city.narcotics, bevy_civ::troubles::NARCOTICS_MAX, city.narcotics, city.narcotics));
                }
                if city.scandal > 0 {
                    troubles.push(format!("anger over adulterated food (+{} unhappy, {} more turns)", bevy_civ::troubles::SCANDAL_UNHAPPY, city.scandal));
                }
                if buildings.has(Building::GmFarms) && !buildings.has(Building::FoodSafety) {
                    troubles.push("distrust of its engineered food (+1 unhappy; a Food Safety Agency reassures them)".to_string());
                }
                if !troubles.is_empty() {
                    p.spawn(label(&art.font_bold, format!("Troubles: {}.", troubles.join("; ")), 14.0, Color::srgb(1.0, 0.6, 0.45)));
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
            // A window no taller than the screen: the mouse wheel scrolls what does not fit.
            let window = |width: f32| {
                (
                    super::layout::Scrollable,
                    RelativeCursorPosition::default(),
                    ScrollPosition::default(),
                    panel(
                        &art.parchment,
                        Node {
                            width: Val::Px(width),
                            max_width: Val::Percent(96.0),
                            max_height: Val::Percent(100.0),
                            overflow: Overflow::scroll_y(),
                            padding: UiRect::all(Val::Px(26.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(10.0),
                            ..default()
                        },
                    ),
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
                        button(r, art, "Chronicle", Command::Chronicle, BRONZE, 14.0);
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
                        OfferKind::Passage => "Our units and theirs could then share tiles and pass through each other.".to_string(),
                        OfferKind::Crackdown => format!("Accepting keeps smugglers from our cities out of theirs for {} turns. Refusing angers them.", bevy_civ::troubles::CRACKDOWN_TURNS),
                        OfferKind::Withdraw => "Accepting sends our troops in their lands back to our nearest city (and ends any right of passage). Refusing angers them, and they may fight.".to_string(),
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
                                let ours = city_list.iter().filter(|(_, c)| c.owner == PLAYER).count() as u32;
                                let sprawl = bevy_civ::diplomacy::sprawl_unhappiness(ours);
                                w.spawn(label(
                                    &art.font_body,
                                    format!("Empire size: {ours} cities; {sprawl} more unhappy in every city (none up to 12 cities, then one more for every 6). A Forbidden Palace (Monarchy) is a second seat of government against corruption."),
                                    14.0,
                                    if sprawl > 0 { RED } else { INK_SOFT },
                                ));
                                let fouled = tiles.iter().filter(|t| t.is_polluted()).count();
                                w.spawn(label(
                                    &art.font_body,
                                    format!(
                                        "World pollution: {fouled} fouled tiles (global warming threatens past 15){}.",
                                        if state.warming > 0 { format!("; global warming has struck {} times", state.warming) } else { String::new() }
                                    ),
                                    14.0,
                                    if fouled > 15 { RED } else { INK_SOFT },
                                ));
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
                                                // Taxes take the rest, so they have no buttons; elsewhere a step that is
                                                // not allowed shows greyed, keeping the buttons in place.
                                                if name == "Taxes" {
                                                    return;
                                                }
                                                match change {
                                                    Some((s, l)) => button(slot, art, text, Command::Game(Action::SetRates(s, l)), BRONZE, 13.0),
                                                    None => button(slot, art, text, Command::Game(Action::SetRates(sci, lux)), SPENT, 13.0),
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
                                // Migration policy.
                                w.spawn(row(8.0)).with_children(|r| {
                                    r.spawn(label(&art.font_bold, "Migrants from abroad:", 15.0, INK));
                                    let welcome = state.welcome_migrants;
                                    button(r, art, "Welcome", Command::Game(Action::SetMigration(true)), if welcome { GREEN } else { BRONZE }, 13.0);
                                    button(r, art, "Turn away", Command::Game(Action::SetMigration(false)), if welcome { BRONZE } else { GREEN }, 13.0);
                                    r.spawn(label(
                                        &art.font_body,
                                        "Once we know Industrialization, people leave unhappy, hungry or troubled cities for better ones nearby.",
                                        13.0,
                                        INK_SOFT,
                                    ));
                                });
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
                                    let food = city.food_balance(buildings);
                                    let food = if food < 0 { format!("starving ({food} food)") } else { format!("{food:+} food") };
                                    let text = format!(
                                        "{} (size {}): {food}, building {} - {} happy, {} unhappy, {} - focus {}",
                                        city.name,
                                        city.population,
                                        making,
                                        happiness.happy,
                                        happiness.unhappy,
                                        mood,
                                        city.focus.name()
                                    );
                                    let tint = if city.production_queue.is_none() || happiness.in_disorder() || city.food_balance(buildings) < 0 { Color::srgb(0.95, 0.6, 0.5) } else { BRONZE };
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
                            EmpireTab::Space => {
                                use bevy_civ::space::{DESTINATION, LIGHT_YEARS, MOST, NEEDED};
                                w.spawn(label(
                                    &art.font_body,
                                    format!(
                                        "The race to {DESTINATION}, the nearest star ({LIGHT_YEARS} light-years). Cities with a Factory build {} structures (Space Flight), {} to {} components - engines; each extra one saves two turns (Plastics) - and {} modules (Superconductors). The first ship to arrive wins; a ship in flight is lost if its capital falls.",
                                        NEEDED[0], NEEDED[1], MOST[1], NEEDED[2]
                                    ),
                                    14.0,
                                    INK_SOFT,
                                ));
                                let mut fleet: Vec<(u32, String)> = vec![(PLAYER, "Our ship".to_string())];
                                fleet.extend(civs.players.iter().filter(|c| !c.is_human && foreign.met.contains(&c.id)).map(|c| (c.id, format!("{} ship", c.adjective))));
                                let mut any = false;
                                for (id, name) in fleet {
                                    let Some(ship) = space.ships.get(&id) else { continue };
                                    any = true;
                                    let status = match ship.arrives {
                                        Some(turn) => format!("in flight, arrives turn {turn}"),
                                        None if ship.is_ready() => format!("ready to launch ({} turns to {DESTINATION})", ship.flight_turns()),
                                        None => format!("being built: {} still needed", ship.missing()),
                                    };
                                    w.spawn(label(
                                        &art.font_bold,
                                        format!("{name}: {} of {} structures, {} components, {} of {} modules - {status}", ship.parts[0], NEEDED[0], ship.parts[1], ship.parts[2], NEEDED[2]),
                                        15.0,
                                        INK,
                                    ));
                                    // The voyage: Sol to the star, with the ship's progress.
                                    w.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(8.0), width: Val::Percent(100.0), ..default() }).with_children(|r| {
                                        r.spawn(label(&art.font_body, "Sol", 13.0, INK_SOFT));
                                        r.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, ..default() }).with_children(|b| {
                                            bar(b, ship.progress(state.turn), if id == PLAYER { Color::srgb(0.35, 0.7, 0.95) } else { Color::srgb(0.85, 0.45, 0.35) });
                                        });
                                        r.spawn(label(&art.font_body, DESTINATION, 13.0, INK_SOFT));
                                    });
                                    if id == PLAYER && ship.arrives.is_none() && ship.is_ready() {
                                        w.spawn(row(6.0)).with_children(|r| {
                                            button(r, art, format!("Launch for {DESTINATION} ({} turns)", ship.flight_turns()), Command::Game(Action::LaunchShip), GREEN, 14.0);
                                            if ship.parts[1] < MOST[1] {
                                                r.spawn(label(&art.font_body, "or build more components first to fly faster", 13.0, INK_SOFT));
                                            }
                                        });
                                    }
                                }
                                if !any {
                                    w.spawn(label(&art.font_body, "No spaceship has been started yet.", 14.0, INK));
                                }
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
                                let music = if sound_on.music { "Music: On" } else { "Music: Off" };
                                c.spawn(Node::default()).with_children(|r| button(r, art, music, Command::ToggleMusic, BRONZE, 15.0));
                            }
                            c.spawn(Node::default()).with_children(|r| button(r, art, "Encyclopedia (F1)", Command::Toggle(Modal::Help), BRONZE, 15.0));
                            c.spawn(Node::default()).with_children(|r| button(r, art, "Messages (L)", Command::Toggle(Modal::Messages), BRONZE, 15.0));
                            let tips_label = if tips_on.enabled { "Tips for new players: On" } else { "Tips for new players: Off" };
                            c.spawn(Node::default()).with_children(|r| button(r, art, tips_label, Command::ToggleTips, BRONZE, 15.0));
                            c.spawn(Node::default()).with_children(|r| button(r, art, "Quit to the start menu", Command::QuitToMenu, Color::srgb(0.9, 0.6, 0.5), 15.0));
                        });
                        w.spawn(row(6.0)).with_children(|r| button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0));
                    });
                }
                Modal::Messages => {
                    const PER_PAGE: usize = 24;
                    p.spawn(window(800.0)).with_children(|w| {
                        w.spawn(label(&art.font_title, if ui.highlights_only { "Chronicle" } else { "Messages" }, 26.0, INK));
                        let shown: Vec<&(u32, String)> = log
                            .history
                            .iter()
                            .filter(|(_, line)| !ui.highlights_only || crate::HIGHLIGHTS.iter().any(|h| line.contains(h)))
                            .collect();
                        let pages = shown.len().div_ceil(PER_PAGE).max(1);
                        let page = ui.messages_page.min(pages - 1);
                        let end = shown.len().saturating_sub(page * PER_PAGE);
                        let start = end.saturating_sub(PER_PAGE);
                        w.spawn(row(6.0)).with_children(|r| {
                            button(r, art, if ui.highlights_only { "Show all messages" } else { "Great events only" }, Command::ToggleHighlights, BRONZE, 13.0);
                            if page + 1 < pages {
                                button(r, art, "Older", Command::MessagesPage(1), BRONZE, 13.0);
                            }
                            if page > 0 {
                                button(r, art, "Newer", Command::MessagesPage(-1), BRONZE, 13.0);
                            }
                            r.spawn(label(&art.font_body, format!("page {} of {pages}", page + 1), 13.0, INK_SOFT));
                        });
                        if shown.is_empty() {
                            w.spawn(label(&art.font_body, "Nothing yet.", 14.0, INK_SOFT));
                        }
                        for (turn, line) in &shown[start..end] {
                            w.spawn(label(&art.font_body, format!("Turn {turn}: {line}"), 13.0, INK));
                        }
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
                    p.spawn(window(760.0)).with_children(|w| {
                        w.spawn(label(&art.font_title, "Science Advisor", 26.0, INK));
                        match tech.current {
                            Some(current) => {
                                let cost = tech.cost_of(current);
                                let left = (cost.saturating_sub(tech.progress) as f32 / tech.science_per_turn.max(1) as f32).ceil();
                                // The pace is only known once the first turn's science is counted.
                                let pace = if state.turn <= 1 { "the pace shows after the first turn".to_string() } else { format!("about {left} turns") };
                                w.spawn(label(&art.font_bold, format!("Researching {}  -  {}/{} beakers, {pace}", current.name(), tech.progress, cost), 16.0, INK));
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
                        // The whole tree, era by era. Click any technology not yet known to make it the
                        // goal: our scholars research the way to it, step by step.
                        w.spawn(label(&art.font_title, "The tree of knowledge", 16.0, INK_SOFT));
                        let path = tech.goal.map(|g| tech.path_to(g)).unwrap_or_default();
                        match tech.goal {
                            Some(goal) => {
                                let turns = (tech.beakers_to(goal) as f32 / tech.science_per_turn.max(1) as f32).ceil();
                                let pace = if state.turn <= 1 { "the pace shows after the first turn".to_string() } else { format!("about {turns} turns") };
                                let steps: Vec<&str> = path.iter().map(|t| t.name()).collect();
                                w.spawn(row(8.0)).with_children(|r| {
                                    r.spawn(label(
                                        &art.font_bold,
                                        format!("Goal: {} - {} discoveries, {pace}: {}", goal.name(), path.len(), steps.join(" > ")),
                                        14.0,
                                        INK,
                                    ));
                                    button(r, art, "Clear goal", Command::Game(Action::ClearResearchGoal), BRONZE, 12.0);
                                });
                            }
                            None => {
                                w.spawn(label(&art.font_body, "Click any technology below to make it our goal: the scholars will research the way to it, in order.", 13.0, INK_SOFT));
                            }
                        }
                        // Legend: known, being studied, on the path, the goal, open, locked.
                        w.spawn(row(8.0)).with_children(|r| {
                            for (text, colour) in [("known", KNOWN), ("studying", GREEN), ("on the path", PATH), ("goal", GOAL), ("open", BRONZE), ("needs more first", SPENT)] {
                                r.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(4.0), ..default() }).with_children(|item| {
                                    item.spawn((Node { width: Val::Px(12.0), height: Val::Px(12.0), ..default() }, BackgroundColor(colour), BorderRadius::all(Val::Px(3.0))));
                                    item.spawn(label(&art.font_body, text, 12.0, INK_SOFT));
                                });
                            }
                        });
                        let eras: [(&str, u32, u32); 4] = [("Ancient", 0, 25), ("Classical", 26, 45), ("Industrial", 46, 95), ("Modern", 96, u32::MAX)];
                        for (era, low, high) in eras {
                            w.spawn(Node { column_gap: Val::Px(5.0), flex_wrap: FlexWrap::Wrap, row_gap: Val::Px(4.0), align_items: AlignItems::Center, ..default() }).with_children(|r| {
                                r.spawn(Node { width: Val::Px(78.0), ..default() }).with_children(|c| {
                                    c.spawn(label(&art.font_bold, era, 13.0, INK));
                                });
                                for t in Tech::ALL.into_iter().filter(|t| (low..=high).contains(&t.cost())) {
                                    if tech.researched.contains(&t) {
                                        r.spawn((Node { padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)), ..default() }, BackgroundColor(KNOWN), BorderRadius::all(Val::Px(5.0)))).with_children(|k| {
                                            k.spawn(label(&art.font_body, t.name(), 12.0, INK));
                                        });
                                        continue;
                                    }
                                    let tint = if tech.current == Some(t) {
                                        GREEN
                                    } else if tech.goal == Some(t) {
                                        GOAL
                                    } else if path.contains(&t) {
                                        PATH
                                    } else if tech.can_research(t) {
                                        BRONZE
                                    } else {
                                        SPENT
                                    };
                                    button(r, art, t.name(), Command::Game(Action::ResearchGoal(t)), tint, 12.0);
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
                    p.spawn(window(820.0)).with_children(|w| {
                        w.spawn(label(&art.font_title, "Foreign Advisor", 26.0, INK));
                        w.spawn(row(6.0)).with_children(|r| {
                            for page in DiploPage::ALL {
                                let tint = if ui.diplo_page == page { GREEN } else { BRONZE };
                                button(r, art, page.name(), Command::DiploPage(page), tint, 14.0);
                            }
                        });
                        match ui.diplo_page {
                            DiploPage::Relations => {
                                relations_page(w, art, &civs.players, &known, &web.0, &diplomacy, &foreign, state.turn);
                                w.spawn(row(6.0)).with_children(|r| button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0));
                                return;
                            }
                            DiploPage::Intelligence => {
                                let latest = |id: u32| history.scores.iter().rev().find(|(_, c, _)| *c == id).map(|(_, _, s)| s).copied().unwrap_or(0);
                                let mut report = Vec::new();
                                for civ in &known {
                                    let owned: Vec<&City> = city_list.iter().map(|(_, c)| c).filter(|c| c.owner == civ.id).collect();
                                    let capital = owned.iter().find(|c| c.is_capital).map(|c| c.position);
                                    report.push(Intel {
                                        id: civ.id,
                                        cities: owned.len(),
                                        citizens: owned.iter().map(|c| c.population).sum(),
                                        score: latest(civ.id),
                                        faith: capital.and_then(|cp| religions.of(cp)).map(|f| f.name()),
                                        wonders: wonders.built.iter().filter(|(_, (o, _))| *o == civ.id).map(|(b, _)| b.name()).collect(),
                                    });
                                }
                                let our_cities: Vec<&City> = city_list.iter().map(|(_, c)| c).filter(|c| c.owner == PLAYER).collect();
                                let ours = (latest(PLAYER), our_cities.len(), our_cities.iter().map(|c| c.population).sum(), state.gold);
                                intelligence_page(w, art, &civs.players, &known, &report, ours, &diplomacy, &spies, &foreign, &tech);
                                w.spawn(row(6.0)).with_children(|r| button(r, art, "Close (Esc)", Command::CloseModal, BRONZE, 14.0));
                                return;
                            }
                            DiploPage::Talk => {}
                        }
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
                                    if let Some(troops) = foreign.trespass.get(&civ.id).filter(|n| **n > 0) {
                                        button(r, art, format!("Demand they withdraw ({troops} troops in our lands)"), ask(DiploAct::Withdraw), BRONZE, 14.0);
                                    }
                                    if foreign.smugglers.contains(&civ.id) {
                                        button(r, art, "Demand a crackdown on smugglers", ask(DiploAct::Crackdown), BRONZE, 14.0);
                                    }
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

/// What the intelligence page reports of one people.
struct Intel {
    id: u32,
    cities: usize,
    citizens: u32,
    score: u32,
    faith: Option<&'static str>,
    wonders: Vec<&'static str>,
}

fn short_name(name: &str) -> String {
    let name = name.trim_start_matches("the ");
    if name.chars().count() > 9 { format!("{}.", name.chars().take(8).collect::<String>()) } else { name.to_string() }
}

/// The relations page: a ring of every people we know (and us), joined by a line for each treaty
/// - painted by `relations::draw_web` - beside how each of them feels about us; then the wars and
/// alliances in words.
#[allow(clippy::too_many_arguments)]
fn relations_page(w: &mut ChildBuilder, art: &Art, all: &[bevy_civ::ai::Civ], known: &[&bevy_civ::ai::Civ], web: &Handle<Image>, diplomacy: &Diplomacy, foreign: &Foreign, turn: u32) {
    let ids: Vec<u32> = std::iter::once(PLAYER).chain(known.iter().map(|c| c.id)).collect();
    let civ_of = |id: u32| all.iter().find(|c| c.id == id);
    let name = |id: u32| if id == PLAYER { "Us".to_string() } else { civ_of(id).map(|c| short_name(&c.name)).unwrap_or_default() };
    let at = super::relations::positions(ids.len());
    let size = super::relations::WEB as f32;
    w.spawn(Node { column_gap: Val::Px(18.0), align_items: AlignItems::Start, flex_wrap: FlexWrap::Wrap, ..default() }).with_children(|r| {
        // The web: lines underneath, portraits on top.
        r.spawn((Node { width: Val::Px(size), height: Val::Px(size), flex_shrink: 0.0, ..default() }, ImageNode::new(web.clone()))).with_children(|ring| {
            for (i, id) in ids.iter().enumerate() {
                let p = at[i];
                let tint = owner_color(*id);
                ring.spawn((
                    Node { position_type: PositionType::Absolute, left: Val::Px(p.x - 23.0), top: Val::Px(p.y - 23.0), padding: UiRect::all(Val::Px(2.0)), ..default() },
                    BackgroundColor(tint),
                    BorderRadius::all(Val::Px(6.0)),
                ))
                .with_children(|frame| {
                    frame.spawn(portrait(art, civ_of(*id).map_or(0, |c| c.portrait), 42.0));
                });
                ring.spawn((
                    Node { position_type: PositionType::Absolute, left: Val::Px(p.x - 45.0), top: Val::Px(p.y + 23.0), width: Val::Px(90.0), justify_content: JustifyContent::Center, ..default() },
                ))
                .with_children(|tag| {
                    tag.spawn(label(&art.font_bold, name(*id), 12.0, INK));
                });
            }
        });
        // Beside it: the legend, and how each people feels about us.
        r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), flex_grow: 1.0, min_width: Val::Px(300.0), ..default() }).with_children(|side| {
            side.spawn(row(10.0)).with_children(|legend| {
                for (text, colour, thick) in [("War", Color::srgb(0.78, 0.18, 0.14), 5.0), ("Alliance", Color::srgb(0.22, 0.59, 0.22), 5.0), ("Peace", Color::srgb(0.49, 0.41, 0.29), 2.0)] {
                    legend.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(4.0), ..default() }).with_children(|item| {
                        item.spawn((Node { width: Val::Px(22.0), height: Val::Px(thick), ..default() }, BackgroundColor(colour)));
                        item.spawn(label(&art.font_body, text, 13.0, INK));
                    });
                }
            });
            side.spawn(label(&art.font_bold, "How they feel about us", 15.0, INK));
            for civ in known {
                let attitude = diplomacy.get_attitude(PLAYER, civ.id);
                side.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|line| {
                    line.spawn(portrait(art, civ.portrait, 24.0));
                    line.spawn(Node { width: Val::Px(78.0), ..default() }).with_children(|n| {
                        n.spawn(label(&art.font_bold, short_name(&civ.name), 12.0, INK));
                    });
                    // A bar from -100 to +100, the middle marked.
                    let share = ((attitude + 100) as f32 / 200.0).clamp(0.0, 1.0);
                    let colour = if attitude < -14 { Color::srgb(0.8, 0.25, 0.2) } else if attitude < 15 { Color::srgb(0.85, 0.7, 0.25) } else { Color::srgb(0.35, 0.7, 0.3) };
                    line.spawn((Node { width: Val::Px(120.0), height: Val::Px(10.0), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.18)))).with_children(|track| {
                        track.spawn((Node { width: Val::Percent(share * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(colour)));
                        track.spawn((Node { position_type: PositionType::Absolute, left: Val::Percent(50.0), width: Val::Px(1.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(INK)));
                    });
                    line.spawn(label(&art.font_body, format!("{} {attitude:+}", foreign::mood(attitude)), 12.0, INK_SOFT));
                });
            }
        });
    });
    // The same in words: who fights whom, who stands together.
    let mut wars = Vec::new();
    let mut alliances = Vec::new();
    for (i, a) in ids.iter().enumerate() {
        for b in ids.iter().skip(i + 1) {
            let age = foreign.treaty_age(*a, *b, turn);
            let pair = |x: u32, y: u32| format!("{} and {}", if x == PLAYER { "we".to_string() } else { name(x) }, name(y));
            match diplomacy.get_treaty(*a, *b) {
                Treaty::War => wars.push(format!("{} ({age} turns)", pair(*a, *b))),
                Treaty::Alliance => alliances.push(pair(*a, *b)),
                _ => {}
            }
        }
    }
    let line = |items: &[String], none: &str| if items.is_empty() { none.to_string() } else { items.join("; ") };
    w.spawn(label(&art.font_bold, format!("At war: {}.", line(&wars, "nobody")), 14.0, INK));
    w.spawn(label(&art.font_bold, format!("Allied: {}.", line(&alliances, "nobody")), 14.0, INK));
}

/// One measure on an intelligence card: an icon, a bar against the largest among the peoples we
/// know, and the figure (a grey "?" bar when we cannot know it).
fn measure(parent: &mut ChildBuilder, art: &Art, icon_index: usize, share: Option<f32>, text: String, colour: Color) {
    parent.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(5.0), width: Val::Px(205.0), ..default() }).with_children(|m| {
        m.spawn(icon_node(art, icon_index, 18.0));
        m.spawn((Node { width: Val::Px(90.0), height: Val::Px(9.0), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.18)))).with_children(|track| {
            if let Some(share) = share {
                track.spawn((Node { width: Val::Percent(share.clamp(0.0, 1.0) * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(colour)));
            }
        });
        m.spawn(label(&art.font_body, text, 12.0, if share.is_some() { INK } else { INK_SOFT }));
    });
}

/// What the intelligence page shows of one people (us included, for comparison).
struct Dossier {
    id: u32,
    portrait: usize,
    title: String,
    line: String,
    score: u32,
    cities: usize,
    citizens: u32,
    army: f32,
    techs: Option<usize>,
    gold: Option<i32>,
    note: String,
}

/// The intelligence page: a card for each people - us first - with bars for score, people, army,
/// knowledge and treasury against the largest among them. Research and treasury need an embassy.
#[allow(clippy::too_many_arguments)]
fn intelligence_page(w: &mut ChildBuilder, art: &Art, all: &[bevy_civ::ai::Civ], known: &[&bevy_civ::ai::Civ], report: &[Intel], ours: (u32, usize, u32, i32), diplomacy: &Diplomacy, spies: &Espionage, foreign: &Foreign, tech: &TechState) {
    if known.is_empty() {
        w.spawn(label(&art.font_bold, "We have not met any other people yet.", 15.0, INK));
        return;
    }
    let strength = |id: u32| foreign.strength.get(&id).copied().unwrap_or(1.0).max(1.0);
    let me = all.iter().find(|c| c.id == PLAYER);
    let mut dossiers = vec![Dossier {
        id: PLAYER,
        portrait: me.map_or(0, |c| c.portrait),
        title: "Us".to_string(),
        line: String::new(),
        score: ours.0,
        cities: ours.1,
        citizens: ours.2,
        army: strength(PLAYER),
        techs: Some(tech.researched.len()),
        gold: Some(ours.3),
        note: String::new(),
    }];
    for civ in known {
        let Some(intel) = report.iter().find(|r| r.id == civ.id) else { continue };
        let embassy = spies.has_embassy(PLAYER, civ.id);
        let at_war: Vec<String> = known.iter().filter(|o| o.id != civ.id && diplomacy.get_treaty(civ.id, o.id) == Treaty::War).map(|o| short_name(&o.name)).collect();
        let allies: Vec<String> = known.iter().filter(|o| o.id != civ.id && diplomacy.get_treaty(civ.id, o.id) == Treaty::Alliance).map(|o| short_name(&o.name)).collect();
        let mut ties = Vec::new();
        if !at_war.is_empty() {
            ties.push(format!("at war with {}", at_war.join(", ")));
        }
        if !allies.is_empty() {
            ties.push(format!("allied with {}", allies.join(", ")));
        }
        let wonders = if intel.wonders.is_empty() { "no wonders".to_string() } else { intel.wonders.join(", ") };
        dossiers.push(Dossier {
            id: civ.id,
            portrait: civ.portrait,
            title: format!("{} of {}", civ.leader, civ.name),
            line: format!("{} - faith: {} - {wonders}{}", civ.government.name(), intel.faith.unwrap_or("none"), if ties.is_empty() { String::new() } else { format!(" - {}", ties.join("; ")) }),
            score: intel.score,
            cities: intel.cities,
            citizens: intel.citizens,
            army: strength(civ.id),
            techs: embassy.then(|| civ.tech.researched.len()),
            gold: embassy.then_some(civ.gold),
            note: if embassy {
                let lacking = espionage::stealable(tech, &civ.tech).len();
                format!("Embassy: researching {}; {lacking} technologies we lack.", civ.tech.current.map(|t| t.name()).unwrap_or("nothing"))
            } else {
                "No embassy: research and treasury hidden (a Diplomat at one of their cities can open one).".to_string()
            },
        });
    }
    let most = |f: &dyn Fn(&Dossier) -> f32| dossiers.iter().map(f).fold(1.0f32, f32::max);
    let (top_score, top_people, top_army) = (most(&|d| d.score as f32), most(&|d| d.citizens as f32), most(&|d| d.army));
    let top_techs = most(&|d| d.techs.unwrap_or(0) as f32);
    let top_gold = most(&|d| d.gold.unwrap_or(0).max(0) as f32);
    for d in &dossiers {
        let colour = owner_color(d.id);
        w.spawn(Node { column_gap: Val::Px(10.0), align_items: AlignItems::Center, ..default() }).with_children(|card| {
            card.spawn((Node { padding: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(colour), BorderRadius::all(Val::Px(6.0)))).with_children(|frame| {
                frame.spawn(portrait(art, d.portrait, 46.0));
            });
            card.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(3.0), flex_shrink: 1.0, ..default() }).with_children(|c| {
                c.spawn(label(&art.font_bold, d.title.clone(), 14.0, INK));
                if !d.line.is_empty() {
                    c.spawn(label(&art.font_body, d.line.clone(), 12.0, INK_SOFT));
                }
                c.spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(6.0), row_gap: Val::Px(2.0), ..default() }).with_children(|bars| {
                    measure(bars, art, icon::CULTURE, Some(d.score as f32 / top_score), format!("score {}", d.score), colour);
                    measure(bars, art, icon::POPULATION, Some(d.citizens as f32 / top_people), format!("{} in {} cities", d.citizens, d.cities), colour);
                    measure(bars, art, icon::ATTACK, Some(d.army / top_army), format!("army {:.0}", d.army), colour);
                    measure(bars, art, icon::SCIENCE, d.techs.map(|t| t as f32 / top_techs), d.techs.map_or("techs ?".to_string(), |t| format!("{t} techs")), colour);
                    measure(bars, art, icon::GOLD, d.gold.map(|g| g.max(0) as f32 / top_gold), d.gold.map_or("gold ?".to_string(), |g| format!("{g} gold")), colour);
                });
                if !d.note.is_empty() {
                    c.spawn(label(&art.font_body, d.note.clone(), 12.0, INK_SOFT));
                }
            });
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
    (tech, supplies, rivers): (Res<TechState>, Res<Supplies>, Res<crate::Rivers>),
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
            let seen = tile.resource.filter(|r| r.reveal_tech().is_none_or(|t| tech.researched.contains(&t)));
            let (f, s, g) = bevy_civ::tiles::tile_yield(tile.terrain, &tile.improvements, seen, rivers.tiles.contains(&tile.coord));
            let moves = match bevy_civ::pathfinding::terrain_cost(tile.terrain) {
                _ if tile.has_road() => "road".to_string(),
                Some(1) => "1 move".to_string(),
                Some(n) => format!("{n} moves"),
                None => "by sea".to_string(),
            };
            format!("{}  -  {f} food, {s} shields, {g} gold  ({moves})\n{}", tile.terrain.name(), extras.join(", "))
        }
        Some(_) => "Unexplored".to_string(),
        None => String::new(),
    };
    if text.0 != value {
        text.0 = value;
    }
}

pub fn update_log(log: Res<GameLog>, screen: Res<super::layout::Screen>, mut text: Query<&mut Text, With<LogText>>) {
    if !log.is_changed() && !screen.is_changed() {
        return;
    }
    if let Ok(mut text) = text.get_single_mut() {
        // A phone shows the latest two messages (L opens the rest).
        let shown = if screen.compact { 2 } else { 4 };
        let start = log.lines.len().saturating_sub(shown);
        text.0 = log.lines[start..].join("\n");
    }
}

/// A newly opened city's panel starts at the top (scrolling itself is `layout::scroll_panels`).
pub fn scroll_panels(selection: Res<Selection>, mut panels: Query<&mut ScrollPosition, With<CityPanel>>, mut shown: Local<Option<Entity>>) {
    if *shown == selection.city {
        return;
    }
    *shown = selection.city;
    for mut scroll in &mut panels {
        scroll.offset_y = 0.0;
    }
}

/// The game's own adjustments to the docked layout (see `layout`): the status bar wraps on a
/// compact screen, and the minimap steps aside on a phone or while a city is open (the city panel
/// then has the right side to itself).
pub fn fit_ui_scale(
    screen: Res<super::layout::Screen>,
    mut minimap: Query<&mut Node, With<MinimapFrame>>,
    mut bar: Query<&mut Node, (With<StatusBar>, Without<MinimapFrame>)>,
    mut tile_info: Query<&mut Node, (With<TileInfoText>, Without<StatusBar>, Without<MinimapFrame>)>,
    selection: Res<Selection>,
) {
    // A touch screen has no hovering, so the tile readout under the minimap only takes room there.
    let info_display = if screen.compact { Display::None } else { Display::Flex };
    for mut node in &mut tile_info {
        if node.display != info_display {
            node.display = info_display;
        }
    }
    let wrap = if screen.width < 1000.0 { FlexWrap::Wrap } else { FlexWrap::NoWrap };
    for mut node in &mut bar {
        if node.flex_wrap != wrap {
            node.flex_wrap = wrap;
        }
    }
    let minimap_display = if screen.compact || selection.city.is_some() { Display::None } else { Display::Flex };
    for mut node in &mut minimap {
        if node.display != minimap_display {
            node.display = minimap_display;
        }
    }
}

// ---- buttons -----------------------------------------------------------------------------------

pub fn button_actions(
    mut activated: EventReader<super::layout::Activated>,
    buttons: Query<&ButtonCommand>,
    mut actions: EventWriter<Action>,
    mut ui: ResMut<UiState>,
    mut menu: ResMut<MenuState>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    (mut sounds, mut sound, mut tips): (EventWriter<Sfx>, ResMut<SoundSettings>, ResMut<super::tips::Tips>),
) {
    for super::layout::Activated(entity) in activated.read() {
        let Ok(command) = buttons.get(*entity) else { continue };
        sounds.send(Sfx::Click);
        match &command.0 {
            Command::ToggleSound => sound.toggle(),
            Command::CityTab(tab) => ui.city_tab = *tab,
            Command::EmpireTab(tab) => ui.empire_tab = *tab,
            Command::HelpTopic(topic) => ui.help_topic = *topic,
            Command::DiploPage(page) => ui.diplo_page = *page,
            Command::ToggleGroupMove => ui.group_move = !ui.group_move,
            Command::ToggleQueueMode => ui.queue_mode = !ui.queue_mode,
            Command::ToggleMusic => sound.toggle_music(),
            Command::MessagesPage(delta) => ui.messages_page = (ui.messages_page as i32 + delta).max(0) as usize,
            Command::ToggleHighlights => {
                ui.highlights_only = !ui.highlights_only;
                ui.messages_page = 0;
            }
            Command::Chronicle => {
                ui.end_dismissed = true;
                ui.highlights_only = true;
                ui.messages_page = 0;
                ui.modal = Some(Modal::Messages);
            }
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
            Command::PickMapSize(size) => menu.map_size = Some(*size),
            Command::PickPeoples(n) => menu.peoples = Some(*n),
            Command::StartGame => {
                if let (Some(scenario), Some(civ)) = (menu.scenario.and_then(|i| menu.catalog.get(i)), menu.civ) {
                    let last_turn = if menu.last_turn == 0 { crate::LAST_TURN } else { menu.last_turn };
                    commands.insert_resource(NewGame {
                        scenario: scenario.clone(),
                        civ,
                        difficulty: menu.difficulty,
                        last_turn,
                        map_size: menu.map_size.unwrap_or(1),
                        peoples: menu.peoples.unwrap_or(default_peoples()),
                    });
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
        // No taller than the screen: what does not fit scrolls (wheel or finger).
        p.spawn((
            super::layout::Scrollable,
            RelativeCursorPosition::default(),
            ScrollPosition::default(),
            panel(
                &art.parchment,
                Node {
                    width: Val::Px(760.0),
                    max_width: Val::Percent(96.0),
                    max_height: Val::Percent(96.0),
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::all(Val::Px(30.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
            ),
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
                    w.spawn(label(
                        &art.font_body,
                        "New to games like this? The Rise of Rome is the quickest to learn: a small map, with your first cities already founded. Once in, press F1 for How to play; short tips explain each new thing as it comes up.",
                        14.0,
                        INK_SOFT,
                    ));
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
                    // A New World can be made smaller or larger, with fewer or more peoples.
                    if scenario.rows.is_empty() {
                        let size = menu.map_size.unwrap_or(1);
                        let peoples = menu.peoples.unwrap_or(default_peoples());
                        w.spawn(label(&art.font_title, "World", 17.0, INK_SOFT));
                        w.spawn(row(6.0)).with_children(|r| {
                            for (i, (name, cols, rows)) in crate::scenario::MAP_SIZES.iter().enumerate() {
                                let tint = if size == i { GREEN } else { BRONZE };
                                button(r, art, format!("{name} ({cols}x{rows})"), Command::PickMapSize(i), tint, 14.0);
                            }
                        });
                        w.spawn(row(6.0)).with_children(|r| {
                            for n in crate::scenario::PEOPLE_CHOICES {
                                let tint = if peoples == n { GREEN } else { BRONZE };
                                button(r, art, format!("{n} peoples"), Command::PickPeoples(n), tint, 14.0);
                            }
                        });
                    }
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

/// Peoples in a New World unless the player chooses otherwise.
fn default_peoples() -> usize {
    crate::scenario::DEFAULT_PEOPLES
}
