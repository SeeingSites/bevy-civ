//! The whole game as one JSON document. It is written to the browser's storage (or `autosave.json`)
//! at the end of every turn, downloaded as a file when the player saves (F2), and can be loaded back
//! from the start menu: the automatic save with Continue, or any saved file, including one someone
//! else sent, with Load.

use std::collections::HashSet;

use bevy::prelude::*;
use hexx::{Hex, HexLayout};
use serde::{Deserialize, Serialize};

use bevy_civ::ai::Civilizations;
use bevy_civ::barbarians::BarbarianState;
use bevy_civ::building::{Building, CityBuildings, WorldWonders};
use bevy_civ::city::{spawn_city, City};
use bevy_civ::culture::CultureLedger;
use bevy_civ::diplomacy::{Diplomacy, Happiness};
use bevy_civ::espionage::Espionage;
use bevy_civ::fog::FogOfWar;
use bevy_civ::great::GreatPeople;
use bevy_civ::save::{read_setting, write_setting};
use bevy_civ::space::SpaceRace;
use bevy_civ::tech::TechState;
use bevy_civ::terrain::{HexTile, Terrain};
use bevy_civ::trade::TradeRoutes;
use bevy_civ::unit::{spawn_unit, Unit};

use crate::foreign::Foreign;
use crate::{Automated, Cycle, Dirty, GameLog, GameState, GoTo, GoldenAges, History, HomeCity, MapData, Rivers, PLAYER};

/// Bumped whenever the format changes in a way older files cannot be read.
pub const VERSION: u32 = 1;
const AUTOSAVE: &str = "autosave.json";
/// Where a file is written on the desktop (and read by Load there).
const SAVE_FILE: &str = "savegame.json";

#[derive(Serialize, Deserialize)]
pub struct SavedUnit {
    pub unit: Unit,
    pub home: Option<Hex>,
    pub goto: Option<Hex>,
    pub automated: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SavedCity {
    pub city: City,
    pub buildings: Vec<Building>,
}

#[derive(Serialize, Deserialize)]
pub struct SaveGame {
    pub version: u32,
    pub state: GameState,
    pub tech: TechState,
    pub civs: Civilizations,
    pub fog: FogOfWar,
    pub diplomacy: Diplomacy,
    pub barbarians: BarbarianState,
    pub trade: TradeRoutes,
    pub space: SpaceRace,
    pub foreign: Foreign,
    pub wonders: WorldWonders,
    pub great: GreatPeople,
    pub spies: Espionage,
    pub history: History,
    pub culture: CultureLedger,
    pub golden: GoldenAges,
    pub rivers: Rivers,
    #[serde(default)]
    pub religions: bevy_civ::religion::Religions,
    #[serde(default)]
    pub villages: bevy_civ::villages::Villages,
    pub log: Vec<String>,
    pub tiles: Vec<HexTile>,
    pub cities: Vec<SavedCity>,
    pub units: Vec<SavedUnit>,
}

impl SaveGame {
    /// Reads a saved game, explaining what is wrong with a file that is not one.
    pub fn parse(json: &str) -> Result<SaveGame, String> {
        let game: SaveGame = serde_json::from_str(json).map_err(|e| format!("That file is not a Bevy Civ save ({e})."))?;
        if game.version != VERSION {
            return Err(format!("That save comes from another version of the game (format {}, this game reads {VERSION}).", game.version));
        }
        Ok(game)
    }
}

/// Asks for the game to be saved: always to the automatic save, and as a file when `download`.
#[derive(Event)]
pub struct SaveRequest {
    pub download: bool,
}

/// A saved game waiting to be restored when the game starts.
#[derive(Resource)]
pub struct PendingLoad(pub SaveGame);

/// The automatic save, if there is one.
pub fn autosave() -> Option<String> {
    read_setting(AUTOSAVE)
}

/// Writes the game whenever it is asked to.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn write_save(
    mut requests: EventReader<SaveRequest>,
    (state, tech, civs, fog, diplomacy, barbarians, trade, space): (
        Res<GameState>,
        Res<TechState>,
        Res<Civilizations>,
        Res<FogOfWar>,
        Res<Diplomacy>,
        Res<BarbarianState>,
        Res<TradeRoutes>,
        Res<SpaceRace>,
    ),
    (foreign, wonders, great, spies, history, culture, golden, rivers, religions, villages): (
        Res<Foreign>,
        Res<WorldWonders>,
        Res<GreatPeople>,
        Res<Espionage>,
        Res<History>,
        Res<CultureLedger>,
        Res<GoldenAges>,
        Res<Rivers>,
        Res<bevy_civ::religion::Religions>,
        Res<bevy_civ::villages::Villages>,
    ),
    mut log: ResMut<GameLog>,
    tiles: Query<&HexTile>,
    cities: Query<(&City, &CityBuildings)>,
    units: Query<(&Unit, Option<&HomeCity>, Option<&GoTo>, Has<Automated>)>,
) {
    let download = requests.read().fold(None, |any: Option<bool>, r| Some(any.unwrap_or(false) || r.download));
    let Some(download) = download else { return };
    let game = SaveGame {
        version: VERSION,
        state: state.clone(),
        tech: tech.clone(),
        civs: civs.clone(),
        fog: fog.clone(),
        diplomacy: diplomacy.clone(),
        barbarians: barbarians.clone(),
        trade: trade.clone(),
        space: space.clone(),
        foreign: foreign.clone(),
        wonders: wonders.clone(),
        great: great.clone(),
        spies: spies.clone(),
        history: history.clone(),
        culture: culture.clone(),
        golden: golden.clone(),
        rivers: rivers.clone(),
        religions: religions.clone(),
        villages: villages.clone(),
        log: log.lines.clone(),
        tiles: tiles.iter().cloned().collect(),
        cities: cities.iter().map(|(c, b)| SavedCity { city: c.clone(), buildings: b.buildings.clone() }).collect(),
        units: units
            .iter()
            .map(|(u, home, goto, automated)| SavedUnit { unit: u.clone(), home: home.map(|h| h.0), goto: goto.map(|g| g.0), automated })
            .collect(),
    };
    let json = match serde_json::to_string(&game) {
        Ok(json) => json,
        Err(e) => {
            log.push(format!("The game could not be saved: {e}"));
            return;
        }
    };
    let stored = write_setting(AUTOSAVE, &json);
    if download {
        let name = format!("bevy-civ-{}-turn-{}.json", state.title.to_lowercase().replace(' ', "-"), state.turn);
        log.push(match platform::download(&name, &json) {
            Some(place) => format!("Game saved: {place}. Anyone can load it from the start menu (Load a saved game)."),
            None => "The game could not be saved as a file.".into(),
        });
    } else if !stored {
        log.push("The automatic save failed (the browser's storage may be full).");
    }
}

/// Starts the file picker (browser) or reads `savegame.json` (desktop). The text arrives through
/// [`take_import`].
pub fn pick_file() {
    platform::pick_file();
}

/// A file the player picked to load, once it has been read.
pub fn take_import() -> Option<String> {
    platform::take_import()
}

/// Builds the saved world when the game starts from a loaded file.
pub fn restore_world(
    mut commands: Commands,
    pending: Res<PendingLoad>,
    mut camera: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    let game = &pending.0;
    let layout = HexLayout::flat().with_hex_size(crate::client::map::HEX_SIZE);
    let mut map = MapData {
        layout,
        cells: HashSet::new(),
        water: HashSet::new(),
        terrain: Default::default(),
        min: Vec2::splat(f32::MAX),
        max: Vec2::splat(f32::MIN),
    };
    for tile in &game.tiles {
        let pos = map.layout.hex_to_world_pos(tile.coord);
        map.min = map.min.min(pos);
        map.max = map.max.max(pos);
        map.cells.insert(tile.coord);
        map.terrain.insert(tile.coord, tile.terrain);
        if tile.terrain == Terrain::Water {
            map.water.insert(tile.coord);
        }
        commands.spawn(tile.clone());
    }
    for saved in &game.cities {
        let c = &saved.city;
        let entity = spawn_city(&mut commands, c.position, c.name.clone(), c.population, c.owner, c.is_capital);
        commands.entity(entity).insert((c.clone(), CityBuildings { buildings: saved.buildings.clone() }, Happiness::new(c.population)));
    }
    for saved in &game.units {
        let u = &saved.unit;
        let entity = spawn_unit(&mut commands, u.position, u.unit_type, u.owner);
        let mut e = commands.entity(entity);
        e.insert(u.clone());
        if let Some(home) = saved.home {
            e.insert(HomeCity(home));
        }
        if let Some(goal) = saved.goto {
            e.insert(GoTo(goal));
        }
        if saved.automated {
            e.insert(Automated);
        }
    }
    // Saves from before troops needed ships can have troops crossing the sea by themselves: give
    // them a Galley (or a Transport for a large party), so they are not lost.
    for (at, owner, troops) in stranded(&game.units, &map.water) {
        let kind = if troops > bevy_civ::unit::UnitType::Galley.capacity() as usize { bevy_civ::unit::UnitType::Transport } else { bevy_civ::unit::UnitType::Galley };
        for _ in 0..troops.div_ceil(kind.capacity() as usize) {
            spawn_unit(&mut commands, at, kind, owner);
        }
    }
    if let Ok((mut transform, mut projection)) = camera.get_single_mut() {
        let capital = game.cities.iter().find(|c| c.city.owner == PLAYER && c.city.is_capital).or(game.cities.iter().find(|c| c.city.owner == PLAYER));
        if let Some(home) = capital {
            transform.translation = map.layout.hex_to_world_pos(home.city.position).extend(transform.translation.z);
        }
        projection.scale = 1.0;
    }
    commands.insert_resource(map);
    commands.insert_resource(game.state.clone());
    commands.insert_resource(game.tech.clone());
    commands.insert_resource(game.civs.clone());
    commands.insert_resource(game.fog.clone());
    commands.insert_resource(game.diplomacy.clone());
    commands.insert_resource(game.barbarians.clone());
    commands.insert_resource(game.trade.clone());
    commands.insert_resource(game.space.clone());
    commands.insert_resource(game.foreign.clone());
    commands.insert_resource(game.wonders.clone());
    commands.insert_resource(game.great.clone());
    commands.insert_resource(game.spies.clone());
    commands.insert_resource(game.history.clone());
    commands.insert_resource(game.culture.clone());
    commands.insert_resource(game.golden.clone());
    commands.insert_resource(game.rivers.clone());
    commands.insert_resource(game.religions.clone());
    commands.insert_resource(game.villages.clone());
    let mut log = GameLog {
        lines: game.log.clone(),
        written: game.log.len(),
        history: game.log.iter().map(|l| (game.state.turn, l.clone())).collect(),
        turn: game.state.turn,
    };
    log.push(format!("Game loaded: turn {}.", game.state.turn));
    commands.insert_resource(log);
    commands.insert_resource(Dirty { map: true, ui: true });
    commands.insert_resource(Cycle { advance: true, force: false });
    commands.remove_resource::<PendingLoad>();
}

/// Desktop: files in the working directory.
#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use super::SAVE_FILE;
    use std::sync::Mutex;

    static IMPORT: Mutex<Option<String>> = Mutex::new(None);

    pub fn download(_name: &str, json: &str) -> Option<String> {
        std::fs::write(SAVE_FILE, json).ok().map(|_| format!("written to {SAVE_FILE}"))
    }

    pub fn pick_file() {
        if let Ok(text) = std::fs::read_to_string(SAVE_FILE) {
            *IMPORT.lock().unwrap() = Some(text);
        }
    }

    pub fn take_import() -> Option<String> {
        IMPORT.lock().unwrap().take()
    }
}

/// Browser: a download, and a file picker (both small helpers in `web/index.html`).
#[cfg(target_arch = "wasm32")]
mod platform {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = bevyCivDownload)]
        fn js_download(name: &str, text: &str);
        #[wasm_bindgen(js_name = bevyCivPickFile)]
        fn js_pick_file();
        #[wasm_bindgen(js_name = bevyCivTakeImport)]
        fn js_take_import() -> Option<String>;
    }

    pub fn download(name: &str, json: &str) -> Option<String> {
        js_download(name, json);
        Some(format!("downloaded as {name}"))
    }

    pub fn pick_file() {
        js_pick_file();
    }

    pub fn take_import() -> Option<String> {
        js_take_import()
    }
}

/// Tiles at sea where a people's troops have no ship to carry them, and how many troops are there.
fn stranded(units: &[SavedUnit], water: &std::collections::HashSet<Hex>) -> Vec<(Hex, u32, usize)> {
    let mut need: std::collections::BTreeMap<(i32, i32, u32), usize> = std::collections::BTreeMap::new();
    for saved in units.iter().filter(|s| s.unit.unit_type.domain() == bevy_civ::unit::Domain::Land && water.contains(&s.unit.position)) {
        let u = &saved.unit;
        let room: u32 = units.iter().filter(|s| s.unit.owner == u.owner && s.unit.position == u.position && s.unit.unit_type.carries(u.unit_type)).map(|s| s.unit.unit_type.capacity()).sum();
        if room == 0 {
            *need.entry((u.position.x, u.position.y, u.owner)).or_default() += 1;
        }
    }
    need.into_iter().map(|((x, y, owner), n)| (Hex::new(x, y), owner, n)).collect()
}

#[cfg(test)]
mod tests {

    #[test]
    fn troops_left_at_sea_by_an_old_save_are_given_a_ship() {
        use bevy_civ::unit::UnitType;
        let at = |kind: UnitType, x: i32, owner: u32| {
            let unit = Unit { unit_type: kind, movement_left: 1, owner, position: Hex::new(x, 0), build_turns_left: None, current_build: None, fortified: false, road_credit: 0, hp: 3, level: 0, troops: Vec::new(), fatigue: 0, carrier: None };
            SavedUnit { unit, home: None, goto: None, automated: false }
        };
        let water: HashSet<Hex> = [Hex::new(1, 0), Hex::new(2, 0)].into();
        let units = vec![at(UnitType::Rifleman, 1, 2), at(UnitType::Rifleman, 1, 2), at(UnitType::Rifleman, 1, 2), at(UnitType::Rifleman, 2, 2), at(UnitType::Galleon, 2, 2), at(UnitType::Rifleman, 0, 2)];
        assert_eq!(stranded(&units, &water), vec![(Hex::new(1, 0), 2, 3)]);
    }
    use super::*;
    use bevy_civ::city::ProductionItem;
    use bevy_civ::diplomacy::Treaty;
    use bevy_civ::unit::UnitType;

    fn sample() -> SaveGame {
        let mut diplomacy = Diplomacy::default();
        diplomacy.set_treaty(0, 2, Treaty::War);
        let mut great = GreatPeople::default();
        great.city.insert(Hex::new(1, 2), [1, 2, 3, 4]);
        let mut culture = CultureLedger::default();
        culture.add(Hex::new(1, 2), 77);
        let city = City {
            name: "Roma".into(),
            population: 6,
            food_storage: 9,
            production_progress: 12,
            production_queue: Some(ProductionItem::Building(Building::Temple)),
            position: Hex::new(1, 2),
            owner: PLAYER,
            specialists: vec![],
            is_capital: true,
            resistance: 0,
            queue: Vec::new(),
            worked: Vec::new(),
            land: (0, 0, 0),
        };
        let unit = Unit { unit_type: UnitType::Legion, movement_left: 1, owner: PLAYER, position: Hex::new(2, 2), build_turns_left: None, current_build: None, fortified: true, road_credit: 0, hp: 2, level: 1, troops: Vec::new(), fatigue: 0, carrier: None };
        SaveGame {
            version: VERSION,
            state: GameState { turn: 42, ..Default::default() },
            tech: TechState::default(),
            civs: Civilizations::default(),
            fog: FogOfWar::default(),
            diplomacy,
            barbarians: BarbarianState::default(),
            trade: TradeRoutes::default(),
            space: SpaceRace::default(),
            foreign: Foreign::default(),
            wonders: WorldWonders::default(),
            great,
            spies: Espionage::default(),
            history: History::default(),
            culture,
            golden: GoldenAges::default(),
            rivers: Rivers::default(),
            religions: Default::default(),
            villages: Default::default(),
            log: vec!["hello".into()],
            tiles: vec![HexTile { coord: Hex::new(1, 2), terrain: Terrain::Hills, improvements: vec![], resource: None }],
            cities: vec![SavedCity { city, buildings: vec![Building::Temple, Building::Oracle] }],
            units: vec![SavedUnit { unit, home: Some(Hex::new(1, 2)), goto: Some(Hex::new(5, 5)), automated: false }],
        }
    }

    #[test]
    fn a_saved_game_reads_back_whole() {
        let json = serde_json::to_string(&sample()).unwrap();
        let game = SaveGame::parse(&json).unwrap();
        assert_eq!(game.state.turn, 42);
        assert_eq!(game.diplomacy.get_treaty(0, 2), Treaty::War);
        assert_eq!(game.great.city[&Hex::new(1, 2)], [1, 2, 3, 4]);
        assert_eq!(game.culture.of(Hex::new(1, 2)), 77);
        assert_eq!(game.cities[0].buildings, vec![Building::Temple, Building::Oracle]);
        assert_eq!(game.cities[0].city.production_queue, Some(ProductionItem::Building(Building::Temple)));
        let unit = &game.units[0];
        assert_eq!((unit.unit.hp, unit.unit.level, unit.goto), (2, 1, Some(Hex::new(5, 5))));
    }

    #[test]
    fn other_files_are_refused_with_a_reason() {
        assert!(SaveGame::parse("{\"hello\": 1}").err().unwrap().contains("not a Bevy Civ save"));
        let mut old = sample();
        old.version = VERSION + 1;
        assert!(SaveGame::parse(&serde_json::to_string(&old).unwrap()).err().unwrap().contains("another version"));
    }
}
