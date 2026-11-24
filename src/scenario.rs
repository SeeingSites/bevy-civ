//! Maps and starting positions: the built-in historical scenarios and the random-map generator.

use bevy::prelude::*;
use hexx::Hex;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::Deserialize;

/// Scenario files produced by `tools/maps/build.cjs`.
const BUILT_IN: [&str; 8] = [
    include_str!("../assets/scenarios/earth.json"),
    include_str!("../assets/scenarios/rome.json"),
    include_str!("../assets/scenarios/discovery.json"),
    // Expansion packs' scenarios (tools/maps/expansions.cjs), oldest first.
    include_str!("../assets/scenarios/revolution.json"),
    include_str!("../assets/scenarios/napoleon.json"),
    include_str!("../assets/scenarios/greatwar.json"),
    include_str!("../assets/scenarios/ww2.json"),
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
    /// The expansion pack the scenario belongs to (e.g. "worldwars"), if any: listed only while
    /// that pack is on.
    #[serde(default)]
    pub expansion: Option<String>,
    /// Pairs of civilization names that start allied.
    #[serde(default)]
    pub alliances: Vec<(String, String)>,
    /// Cities worth points to whoever holds them (see `bevy_civ::score::OBJECTIVE_POINTS`): what a
    /// period's war is fought over, so a score at the end rewards winning it, not sitting it out.
    #[serde(default)]
    pub objectives: Vec<String>,
    /// A world already built: its land worked (roads, farms, mines, railroads) and its cities
    /// furnished with the buildings of their size and time.
    #[serde(default)]
    pub developed: bool,
    /// Wonders already standing, by the city they stand in (or none: lost to history).
    #[serde(default)]
    pub wonders: Vec<(String, Option<String>)>,
    /// Draws the world from this seed: a New World's land and homes, and every map's resources and
    /// villages. The same seed builds the same world (how the game then goes still varies). None:
    /// a fresh world each time.
    #[serde(default)]
    pub seed: Option<u64>,
    /// Percent added to the cost of research: a world that starts deep in the tech tree would
    /// otherwise race through the little that is left of it.
    #[serde(default)]
    pub research_cost: u32,
}

/// An expansion pack (`assets/expansions/`): nations that can be led from 4000 BC, on the Whole
/// Earth (from the home given as their first city) or in a New World, and scenarios of its period
/// (those whose `expansion` is its id).
#[derive(Deserialize, Clone, Default)]
pub struct Expansion {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub peoples: Vec<CivDef>,
}

/// The expansion packs built into the game, in the order the menu lists them.
const EXPANSIONS: [&str; 3] = [
    include_str!("../assets/expansions/revolution.json"),
    include_str!("../assets/expansions/worldwars.json"),
    include_str!("../assets/expansions/modern.json"),
];

pub fn expansions() -> Vec<Expansion> {
    EXPANSIONS.iter().filter_map(|json| serde_json::from_str(json).ok()).collect()
}

/// Whether an expansion pack is switched on (off unless the player turned it on).
pub fn expansion_on(id: &str) -> bool {
    bevy_civ::save::read_setting(&format!("expansion.{id}")).as_deref() == Some("on")
}

/// Switches an expansion pack on or off, remembered between games.
pub fn set_expansion(id: &str, on: bool) {
    bevy_civ::save::write_setting(&format!("expansion.{id}"), if on { "on" } else { "off" });
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
    /// The government it starts under (by name, e.g. "Monarchy"; Despotism if none).
    #[serde(default)]
    pub government: Option<String>,
    /// An expansion's people: in a game only when the player leads it.
    #[serde(default)]
    pub optional: bool,
    /// The people it takes the place of when it joins the Whole Earth (sharing its homeland).
    #[serde(default)]
    pub replaces: Option<String>,
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
impl CivDef {
    /// The government it starts under.
    pub fn starting_government(&self) -> bevy_civ::government::Government {
        use bevy_civ::government::Government::*;
        [Monarchy, Republic, Democracy, Communism, Fundamentalism]
            .into_iter()
            .find(|g| Some(g.base_name()) == self.government.as_deref())
            .unwrap_or(Despotism)
    }
}

pub fn catalog() -> Vec<Scenario> {
    catalog_with(expansion_on)
}

/// The start menu's games, with the expansion packs `on` says are switched on.
pub fn catalog_with(on: impl Fn(&str) -> bool) -> Vec<Scenario> {
    let theme = bevy_civ::theme::id();
    let all: Vec<Scenario> = BUILT_IN.iter().filter_map(|json| serde_json::from_str::<Scenario>(json).ok()).collect();
    let fallback = all.first().cloned();
    let mut built_in: Vec<Scenario> =
        all.into_iter().filter(|s| bevy_civ::theme::plays(&s.theme) && s.expansion.as_deref().is_none_or(&on)).collect();
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
    if let Some(first) = built_in.iter().find(|s| s.expansion.is_none()).or(fallback.as_ref()) {
        random.civs = first.civs.iter().take(8).cloned().collect();
    }
    // The packs' peoples can be led from the start too: on a New World, and on the Whole Earth
    // from their homes. They join only a game whose player leads them.
    if theme == "earth" {
        let peoples: Vec<CivDef> = expansions()
            .into_iter()
            .filter(|pack| on(&pack.id))
            .flat_map(|pack| pack.peoples)
            .map(|mut civ| {
                civ.optional = true;
                civ
            })
            .collect();
        random.civs.extend(peoples.iter().cloned().map(|mut civ| {
            civ.units.clear();
            civ
        }));
        if let Some(earth) = built_in.iter_mut().find(|s| s.id == "earth") {
            earth.civs.extend(peoples);
        }
    }
    std::iter::once(random).chain(built_in).collect()
}

/// The scenario as played when the player leads `civ`: the packs' other peoples stay out, and a
/// people that takes another's homeland takes its place. Returns the player's new index.
pub fn prepare(scenario: &Scenario, civ: usize) -> (Scenario, usize) {
    let mut scenario = scenario.clone();
    let chosen = scenario.civs.get(civ).cloned();
    let replaced = chosen.as_ref().and_then(|c| c.replaces.clone()).filter(|_| !scenario.rows.is_empty());
    let name = chosen.map(|c| c.name).unwrap_or_default();
    scenario.civs.retain(|c| c.name == name || (!c.optional && Some(&c.name) != replaced.as_ref()));
    let index = scenario.civs.iter().position(|c| c.name == name).unwrap_or(0);
    // Its home is workable land.
    if let Some(home) = scenario.civs[index].cities.first().filter(|_| scenario.civs[index].optional) {
        if let Some(row) = scenario.rows.get_mut(home.row as usize) {
            let mut cells: Vec<char> = row.chars().collect();
            if let Some(cell) = cells.get_mut(home.col as usize).filter(|c| "mhd".contains(**c)) {
                *cell = 'p';
            }
            *row = cells.into_iter().collect();
        }
    }
    (scenario, index)
}

fn neighbours(col: i32, row: i32) -> [(i32, i32); 6] {
    let odd = col & 1;
    [(col, row - 1), (col, row + 1), (col - 1, row - 1 + odd), (col - 1, row + odd), (col + 1, row - 1 + odd), (col + 1, row + odd)]
}

/// The random numbers a world is drawn from: seeded when the scenario has a seed (`stream` keeps the
/// map's and the resources' draws apart), fresh otherwise.
pub fn world_rng(seed: Option<u64>, stream: u64) -> rand::rngs::StdRng {
    use rand::SeedableRng;
    match seed {
        Some(seed) => rand::rngs::StdRng::seed_from_u64(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ stream),
        None => rand::rngs::StdRng::from_entropy(),
    }
}

/// Fills in a random map and start positions for the "New World" scenario.
/// `player` is kept among the civilizations that take part; its new index is returned.
pub fn generate_random(template: &Scenario, player: usize, map_size: usize, peoples: usize) -> (Scenario, usize) {
    let (_, cols, rows) = MAP_SIZES[map_size.min(MAP_SIZES.len() - 1)];
    #[allow(non_snake_case)]
    let (COLS, ROWS) = (cols, rows);
    let civs_wanted = peoples.clamp(2, template.civs.len().max(2));
    let mut rng = world_rng(template.seed, 0);
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

    /// Everything a scenario puts on the map belongs to its age: each people knows the techs behind
    /// its units, wonders and government, and the techs those rest on.
    #[test]
    fn scenarios_hold_nothing_beyond_their_age() {
        use bevy_civ::building::Building;
        use bevy_civ::tech::Tech;
        let mut faults = Vec::new();
        let catalog = catalog_with(|_| true);
        let peoples = catalog.iter().map(|s| (s.id.clone(), s.civs.clone())).chain(expansions().into_iter().map(|e| (format!("pack {}", e.id), e.peoples)));
        for (id, civs) in peoples {
            for civ in &civs {
                let known: Vec<Tech> = Tech::ALL.into_iter().filter(|t| civ.techs.iter().any(|n| n == t.base_name())).collect();
                for name in &civ.techs {
                    if !known.iter().any(|t| t.base_name() == name) {
                        faults.push(format!("{id}: {}: unknown tech {name}", civ.name));
                    }
                }
                for t in &known {
                    for pre in t.prerequisites() {
                        if !known.contains(&pre) {
                            faults.push(format!("{id}: {} knows {t:?} but not {pre:?}", civ.name));
                        }
                    }
                }
                for unit in &civ.units {
                    match UnitType::ALL.into_iter().find(|u| u.base_name() == unit.kind) {
                        None => faults.push(format!("{id}: {}: unknown unit {}", civ.name, unit.kind)),
                        Some(u) => {
                            if let Some(t) = u.required_tech().filter(|t| !known.contains(t)) {
                                faults.push(format!("{id}: {} has {u:?} without {t:?}", civ.name));
                            }
                            if !u.is_trainable() && !u.is_great_person() {
                                faults.push(format!("{id}: {} starts with untrainable {u:?}", civ.name));
                            }
                        }
                    }
                }
                let government = civ.government.as_deref().and_then(|g| bevy_civ::government::Government::CHOICES.into_iter().find(|x| x.base_name() == g));
                if let Some(t) = government.and_then(|g| g.required_tech()).filter(|t| !known.contains(t)) {
                    faults.push(format!("{id}: {} rules by {government:?} without {t:?}", civ.name));
                }
            }
        }
        for scenario in &catalog {
            for (wonder, at) in &scenario.wonders {
                let Some(w) = Building::ALL.into_iter().find(|b| b.is_wonder() && b.base_name() == wonder) else {
                    faults.push(format!("{}: unknown wonder {wonder}", scenario.id));
                    continue;
                };
                let Some(at) = at else { continue };
                let Some(owner) = scenario.civs.iter().find(|c| c.cities.iter().any(|c| &c.name == at)) else {
                    faults.push(format!("{}: {wonder} stands in {at}, no city", scenario.id));
                    continue;
                };
                if let Some(t) = w.required_tech().filter(|t| !owner.techs.iter().any(|n| n == t.base_name())) {
                    faults.push(format!("{}: {wonder} in {at} without {t:?}", scenario.id));
                }
            }
        }
        assert!(faults.is_empty(), "{} anachronisms:\n{}", faults.len(), faults.join("\n"));
    }

    #[test]
    fn expansion_packs_add_their_scenarios_and_peoples_only_when_on() {
        let off = catalog_with(|_| false);
        assert!(off.iter().all(|s| s.expansion.is_none() && s.civs.iter().all(|c| !c.optional)));
        assert_eq!(off[0].civs.len(), 8);
        let on = catalog_with(|_| true);
        for id in ["revolution", "napoleon", "greatwar", "ww2", "modern"] {
            let scenario = on.iter().find(|s| s.id == id).unwrap_or_else(|| panic!("{id} is listed"));
            assert!(scenario.expansion.is_some(), "{id}");
            // Every city stands on land, and every war and alliance names peoples in the game.
            for civ in &scenario.civs {
                for city in &civ.cities {
                    assert_ne!(scenario.rows[city.row as usize].as_bytes()[city.col as usize], b'~', "{id}: {} {}", civ.name, city.name);
                }
            }
            for (a, b) in scenario.wars.iter().chain(&scenario.alliances) {
                assert!(scenario.civs.iter().any(|c| &c.name == a) && scenario.civs.iter().any(|c| &c.name == b), "{id}: {a} / {b}");
            }
        }
        // The packs' peoples join the New World and the Whole Earth, with their own units.
        let earth = on.iter().find(|s| s.id == "earth").unwrap();
        for (name, unique) in [
            ("the United States", UnitType::Marine),
            ("Prussia", UnitType::BlackHussar),
            ("Austria", UnitType::Grenzer),
            ("Italy", UnitType::Bersaglieri),
            ("Germany", UnitType::Panzer),
            ("Canada", UnitType::Mountie),
            ("Mexico", UnitType::Charro),
        ] {
            assert!(on[0].civs.iter().any(|c| c.name == name && c.optional && c.units.is_empty()), "{name} in the New World");
            let civ = earth.civs.iter().position(|c| c.name == name).unwrap_or_else(|| panic!("{name} on the Whole Earth"));
            assert!(earth.civs[civ].city_names.len() >= 6, "{name} has names for its cities");
            assert_eq!(bevy_civ::traits::of(name).unique_unit(), Some(unique));
            // Leading it brings in no other pack's people; a homeland it shares is its own.
            let (game, index) = prepare(earth, civ);
            assert_eq!(game.civs[index].name, name);
            assert_eq!(game.civs.iter().filter(|c| c.optional).count(), 1);
            let home = &game.civs[index].cities[0];
            assert!(game.civs.iter().enumerate().all(|(i, c)| i == index || c.cities.iter().all(|x| (x.col, x.row) != (home.col, home.row))));
        }
        let italy = earth.civs.iter().position(|c| c.name == "Italy").unwrap();
        assert!(prepare(earth, italy).0.civs.iter().all(|c| c.name != "Rome"));
        let rome = earth.civs.iter().position(|c| c.name == "Rome").unwrap();
        let (game, index) = prepare(earth, rome);
        assert_eq!((game.civs.len(), index), (11, 0));
    }
}
