//! Maps and starting positions: the built-in historical scenarios and the random-map generator.

use bevy::prelude::*;
use hexx::Hex;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::Deserialize;

/// Scenario files produced by `tools/maps/build.cjs`.
const BUILT_IN: [&str; 4] = [
    include_str!("../assets/scenarios/earth.json"),
    include_str!("../assets/scenarios/rome.json"),
    include_str!("../assets/scenarios/discovery.json"),
    include_str!("../assets/scenarios/modern.json"),
];

#[derive(Deserialize, Clone, Default)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub start_year: i32,
    pub years_per_turn: i32,
    pub width: i32,
    pub height: i32,
    /// One string per map row; `~` water, `g` grass, `p` plains, `d` desert, `f` forest, `h` hills, `m` mountain.
    pub rows: Vec<String>,
    pub civs: Vec<CivDef>,
    /// Pairs of civilization names that start at war.
    #[serde(default)]
    pub wars: Vec<(String, String)>,
    /// Every civilization already knows the others when the game starts.
    #[serde(default)]
    pub known: bool,
    /// The theme the scenario belongs to (see `bevy_civ::theme`); the menu lists only the scenarios
    /// of the theme in force.
    #[serde(default = "earth")]
    pub theme: String,
    /// The expansion the scenario belongs to (e.g. "modern"), if any: listed only while it is on,
    /// and its peoples then also join the New World's.
    #[serde(default)]
    pub expansion: Option<String>,
}

/// The setting that holds whether the Modern Nations expansion is on.
const EXPANSION_SETTING: &str = "expansion.modern";

/// Whether the Modern Nations expansion is switched on (off unless the player turned it on).
pub fn modern_expansion() -> bool {
    bevy_civ::save::read_setting(EXPANSION_SETTING).as_deref() == Some("on")
}

/// Switches the Modern Nations expansion on or off, remembered between games.
pub fn set_modern_expansion(on: bool) {
    bevy_civ::save::write_setting(EXPANSION_SETTING, if on { "on" } else { "off" });
}

fn earth() -> String {
    "earth".into()
}

#[derive(Deserialize, Clone, Default)]
pub struct CivDef {
    pub name: String,
    pub adjective: String,
    pub leader: String,
    pub portrait: usize,
    pub aggression: f32,
    pub gold: i32,
    pub techs: Vec<String>,
    pub city_names: Vec<String>,
    pub cities: Vec<CityDef>,
    pub units: Vec<UnitDef>,
}

#[derive(Deserialize, Clone, Default)]
pub struct CityDef {
    pub name: String,
    pub col: i32,
    pub row: i32,
    pub pop: u32,
    pub capital: bool,
}

#[derive(Deserialize, Clone, Default)]
pub struct UnitDef {
    #[serde(rename = "type")]
    pub kind: String,
    pub col: i32,
    pub row: i32,
}

/// The game chosen in the start menu.
#[derive(Resource, Clone)]
pub struct NewGame {
    pub scenario: Scenario,
    /// Index into `scenario.civs` of the civilization the player leads.
    pub civ: usize,
    pub difficulty: crate::Difficulty,
    /// The game ends on this turn if nobody has won.
    pub last_turn: u32,
    /// For the New World: the map's size (0 small, 1 normal, 2 large) and how many peoples share it.
    pub map_size: usize,
    pub peoples: usize,
}

/// The New World's map sizes: (name, columns, rows).
pub const MAP_SIZES: [(&str, i32, i32); 3] = [("Small", 34, 22), ("Normal", 44, 28), ("Large", 56, 36)];
/// How many peoples a New World may hold.
pub const PEOPLE_CHOICES: [usize; 3] = [3, 5, 8];
pub const DEFAULT_PEOPLES: usize = 5;

/// Map cell (column, row; row 0 is the northern edge) to its hex. Odd columns sit half a row lower.
pub fn offset_to_hex(col: i32, row: i32) -> Hex {
    let r = row - (col - (col & 1)) / 2;
    Hex::new(col, -r - col)
}

/// Everything the start menu offers: a random map first, then the scenarios of the theme in force.
pub fn catalog() -> Vec<Scenario> {
    catalog_with(modern_expansion())
}

/// The start menu's games, with or without the expansions' scenarios and peoples.
pub fn catalog_with(expansions: bool) -> Vec<Scenario> {
    let theme = bevy_civ::theme::id();
    let all: Vec<Scenario> = BUILT_IN.iter().filter_map(|json| serde_json::from_str::<Scenario>(json).ok()).collect();
    let fallback = all.first().cloned();
    let built_in: Vec<Scenario> =
        all.into_iter().filter(|s| bevy_civ::theme::plays(&s.theme) && (expansions || s.expansion.is_none())).collect();
    let mut random = Scenario {
        id: "random".into(),
        name: "New World".into(),
        blurb: "Peoples share an unknown, randomly generated land: choose how many and how big. Every game is different.".into(),
        start_year: -4000,
        years_per_turn: 40,
        theme: theme.into(),
        ..default()
    };
    // The random map borrows its peoples from the theme's first scenario (Earth's, by default);
    // their homes are chosen when the game starts.
    let first = built_in.iter().find(|s| s.expansion.is_none()).or(fallback.as_ref());
    if let Some(first) = first {
        random.civs = first.civs.iter().take(8).cloned().collect();
    }
    // An expansion's own peoples (those its scenario adds) join them, starting from nothing like
    // everyone else.
    let known: Vec<String> = first.map(|f| f.civs.iter().map(|c| c.name.clone()).collect()).unwrap_or_default();
    for expansion in built_in.iter().filter(|s| s.expansion.is_some()) {
        for civ in expansion.civs.iter().filter(|c| !known.contains(&c.name)) {
            let mut civ = civ.clone();
            civ.techs.clear();
            civ.units.clear();
            civ.cities.truncate(1);
            civ.gold = random.civs.first().map_or(50, |c| c.gold);
            random.civs.push(civ);
        }
    }
    std::iter::once(random).chain(built_in).collect()
}

fn neighbours(col: i32, row: i32) -> [(i32, i32); 6] {
    let odd = col & 1;
    [(col, row - 1), (col, row + 1), (col - 1, row - 1 + odd), (col - 1, row + odd), (col + 1, row - 1 + odd), (col + 1, row + odd)]
}

/// Fills in a random map and start positions for the "New World" scenario.
/// `player` is kept among the civilizations that take part; its new index is returned.
pub fn generate_random(template: &Scenario, player: usize, map_size: usize, peoples: usize) -> (Scenario, usize) {
    let (_, cols, rows) = MAP_SIZES[map_size.min(MAP_SIZES.len() - 1)];
    #[allow(non_snake_case)]
    let (COLS, ROWS) = (cols, rows);
    let civs_wanted = peoples.clamp(2, template.civs.len().max(2));
    let mut rng = rand::thread_rng();
    let mut waves = |freq: f32| -> Vec<(f32, f32, f32, f32)> {
        (0..5)
            .map(|i| {
                let f = freq * (1.0 + i as f32 * 0.6);
                let a = rng.gen_range(0.0..std::f32::consts::TAU);
                (a.cos() * f, a.sin() * f, rng.gen_range(0.0..std::f32::consts::TAU), 1.0 / (1.0 + i as f32 * 0.5))
            })
            .collect()
    };
    let height = waves(0.19);
    let moisture = waves(0.26);
    let sample = |field: &[(f32, f32, f32, f32)], x: f32, y: f32| {
        let total: f32 = field.iter().map(|w| w.3).sum();
        0.5 + 0.5 * field.iter().map(|(fx, fy, phase, amp)| (x * fx + y * fy + phase).sin() * amp).sum::<f32>() / total
    };

    let mut grid = vec![vec!['~'; COLS as usize]; ROWS as usize];
    for row in 0..ROWS {
        for col in 0..COLS {
            let x = col as f32 * 1.5;
            let y = row as f32 * 1.732 + if col & 1 == 1 { 0.866 } else { 0.0 };
            let edge = ((col.min(COLS - 1 - col) as f32 / 5.0).min(row.min(ROWS - 1 - row) as f32 / 4.0)).min(1.0);
            let h = sample(&height, x, y) + rng.gen_range(-0.04..0.04) - (1.0 - edge) * 0.45;
            let m = sample(&moisture, x, y) + rng.gen_range(-0.05..0.05);
            grid[row as usize][col as usize] = if h < 0.42 {
                '~'
            } else if h > 0.8 {
                'm'
            } else if h > 0.69 {
                'h'
            } else if m < 0.3 {
                'd'
            } else if m > 0.62 {
                'f'
            } else if m > 0.46 {
                'g'
            } else {
                'p'
            };
        }
    }

    // Homes: land tiles with room around them, spread as far apart as possible.
    let land = |grid: &Vec<Vec<char>>, col: i32, row: i32| col >= 0 && row >= 0 && col < COLS && row < ROWS && grid[row as usize][col as usize] != '~';
    let mut sites: Vec<(i32, i32)> = (0..ROWS)
        .flat_map(|row| (0..COLS).map(move |col| (col, row)))
        .filter(|(col, row)| land(&grid, *col, *row) && neighbours(*col, *row).iter().filter(|(c, r)| land(&grid, *c, *r)).count() >= 5)
        .collect();
    sites.shuffle(&mut rng);
    let mut homes: Vec<(i32, i32)> = sites.first().copied().into_iter().collect();
    while homes.len() < civs_wanted && homes.len() < sites.len() {
        let best = sites
            .iter()
            .copied()
            .max_by_key(|s| homes.iter().map(|h| offset_to_hex(s.0, s.1).distance_to(offset_to_hex(h.0, h.1))).min().unwrap_or(0));
        match best {
            Some(site) => homes.push(site),
            None => break,
        }
    }

    // The player's people plus a random pick of the others.
    let mut order: Vec<usize> = (0..template.civs.len()).filter(|i| *i != player).collect();
    order.shuffle(&mut rng);
    order.truncate(homes.len().saturating_sub(1));
    order.insert(0, player);

    let civs = order
        .iter()
        .zip(&homes)
        .map(|(index, (col, row))| {
            let mut civ = template.civs[*index].clone();
            let capital = civ.cities.first().map(|c| c.name.clone()).unwrap_or_else(|| civ.name.clone());
            for (c, r) in neighbours(*col, *row).into_iter().chain([(*col, *row)]) {
                if land(&grid, c, r) {
                    grid[r as usize][c as usize] = if (c + r) % 2 == 0 { 'g' } else { 'p' };
                }
            }
            let spots: Vec<(i32, i32)> = neighbours(*col, *row).into_iter().filter(|(c, r)| land(&grid, *c, *r)).collect();
            civ.cities = vec![CityDef { name: capital, col: *col, row: *row, pop: 1, capital: true }];
            civ.units = ["Settler", "Worker", "Warrior"]
                .iter()
                .enumerate()
                .map(|(i, kind)| {
                    let (c, r) = spots.get(i % spots.len().max(1)).copied().unwrap_or((*col, *row));
                    UnitDef { kind: kind.to_string(), col: c, row: r }
                })
                .collect();
            civ
        })
        .collect();

    let scenario = Scenario {
        width: COLS,
        height: ROWS,
        rows: grid.into_iter().map(|r| r.into_iter().collect()).collect(),
        civs,
        ..template.clone()
    };
    (scenario, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_civ::unit::UnitType;

    #[test]
    fn the_modern_expansion_adds_its_scenario_and_peoples_only_when_on() {
        let off = catalog_with(false);
        assert!(off.iter().all(|s| s.expansion.is_none()));
        assert_eq!(off[0].civs.len(), 8);
        let on = catalog_with(true);
        let modern = on
            .iter()
            .find(|s| s.id == "modern")
            .expect("The Modern World is listed");
        assert_eq!(modern.expansion.as_deref(), Some("modern"));
        // Every modern city stands on land.
        for civ in &modern.civs {
            for city in &civ.cities {
                assert_ne!(
                    modern.rows[city.row as usize].as_bytes()[city.col as usize],
                    b'~',
                    "{} {}",
                    civ.name,
                    city.name
                );
            }
        }
        // The New World offers the expansion's peoples too, starting from nothing, with their own units.
        let random = &on[0];
        for (name, unique) in [
            ("Canada", UnitType::Mountie),
            ("Mexico", UnitType::Charro),
            ("the United States", UnitType::Marine),
            ("Germany", UnitType::Panzer),
        ] {
            let civ = random
                .civs
                .iter()
                .find(|c| c.name == name)
                .unwrap_or_else(|| panic!("{name} in the New World"));
            assert!(
                civ.techs.is_empty() && civ.units.is_empty() && civ.cities.len() == 1,
                "{name}"
            );
            assert!(civ.city_names.len() >= 6, "{name} has names for its cities");
            assert_eq!(bevy_civ::traits::of(name).unique_unit(), Some(unique));
        }
    }
}
