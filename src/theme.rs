//! Themes: how the game is dressed - what its units, buildings, technologies, terrain, resources,
//! governments and faiths are called, how buildings are described, how years are counted, a few
//! proper nouns, and where its art lives. The rules stay the same; a theme only renames and redraws
//! them, so the same game can be set on Earth or on another world.
//!
//! A theme is a JSON file in `assets/themes/` (see `earth.json` for the format), built into the game.
//! One theme is active for a session, chosen at start (`--theme <id>` natively, `?theme=<id>` in
//! the browser). Anything a theme leaves out keeps its Earth name. Content names are looked up by the
//! Rust name of the variant: `"units": { "Musketman": "Plasma Trooper" }`.

use std::collections::HashMap;
use std::sync::RwLock;

use serde::Deserialize;

/// How a theme counts years: a suffix for years before and after its zero, e.g. "BC" and "AD".
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Calendar {
    /// "{year} BC" for a year before zero, and "{year} AD" from year one: `{year}` is replaced.
    pub before: String,
    pub after: String,
}

impl Default for Calendar {
    fn default() -> Self {
        Self { before: "{year} BC".into(), after: "{year} AD".into() }
    }
}

/// A theme as read from its file.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub id: String,
    pub name: String,
    /// A theme whose scenarios this one also plays (e.g. a reskin of Earth's maps).
    pub base: Option<String>,
    /// Whether peoples have unique units (a theme may switch them off).
    #[serde(default = "yes")]
    pub unique_units: bool,
    /// The folder the sprite sheets are loaded from, under `assets/` (default `sprites`).
    pub sprites: Option<String>,
    pub calendar: Calendar,
    pub units: HashMap<String, String>,
    pub buildings: HashMap<String, String>,
    /// What buildings and wonders do, as shown in the city panel.
    pub effects: HashMap<String, String>,
    pub techs: HashMap<String, String>,
    pub terrain: HashMap<String, String>,
    pub resources: HashMap<String, String>,
    pub governments: HashMap<String, String>,
    pub faiths: HashMap<String, String>,
    /// Proper nouns and set words: `game_title`, `destination` (the space race's star) and
    /// `barbarian` (the raiders' adjective). Wonders such as the United Nations are buildings.
    pub terms: HashMap<String, String>,
}

fn yes() -> bool {
    true
}

/// Whether the theme in force gives peoples unique units (yes, unless it says otherwise).
pub fn unique_units() -> bool {
    active().is_none_or(|t| t.unique_units)
}

/// Themes built into the game, by id.
pub const BUILT_IN: &[(&str, &str)] = &[
    ("earth", include_str!("../assets/themes/earth.json")),
    ("example", include_str!("../assets/themes/example.json")),
];

/// The theme in force: leaked once per change so names can be handed out as `&'static str`.
static ACTIVE: RwLock<Option<&'static Theme>> = RwLock::new(None);

impl Theme {
    /// Parses a theme file.
    pub fn parse(json: &str) -> Result<Theme, String> {
        serde_json::from_str(json).map_err(|e| format!("theme: {e}"))
    }

    fn table(&self, kind: Kind) -> &HashMap<String, String> {
        match kind {
            Kind::Unit => &self.units,
            Kind::Building => &self.buildings,
            Kind::Effect => &self.effects,
            Kind::Tech => &self.techs,
            Kind::Terrain => &self.terrain,
            Kind::Resource => &self.resources,
            Kind::Government => &self.governments,
            Kind::Faith => &self.faiths,
        }
    }

    /// The name this theme gives a variant (by its Rust name), if it renames it.
    pub fn rename_in(&self, kind: Kind, key: &str) -> Option<&str> {
        self.table(kind).get(key).map(String::as_str)
    }

    /// A year as this theme writes it.
    pub fn year(&self, year: i32) -> String {
        if year < 0 {
            self.calendar.before.replace("{year}", &(-year).to_string())
        } else {
            self.calendar.after.replace("{year}", &year.max(1).to_string())
        }
    }
}

/// What kind of thing a name belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Unit,
    Building,
    Effect,
    Tech,
    Terrain,
    Resource,
    Government,
    Faith,
}

/// Makes `theme` the theme in force.
pub fn activate(theme: Theme) {
    let leaked: &'static Theme = Box::leak(Box::new(theme));
    if let Ok(mut active) = ACTIVE.write() {
        *active = Some(leaked);
    }
}

/// Activates a built-in theme by id; returns false (leaving the current one) if there is none.
pub fn activate_built_in(id: &str) -> bool {
    match BUILT_IN.iter().find(|(name, _)| *name == id).map(|(_, json)| Theme::parse(json)) {
        Some(Ok(theme)) => {
            activate(theme);
            true
        }
        _ => false,
    }
}

/// The theme in force, if one was activated (none means plain Earth).
pub fn active() -> Option<&'static Theme> {
    ACTIVE.read().ok().and_then(|a| *a)
}

/// Whether a scenario of `theme` is played under the theme in force (its own, or its base's).
pub fn plays(theme: &str) -> bool {
    theme == id() || active().and_then(|t| t.base.as_deref()) == Some(theme)
}

/// The id of the theme in force ("earth" by default).
pub fn id() -> &'static str {
    active().map(|t| t.id.as_str()).filter(|id| !id.is_empty()).unwrap_or("earth")
}

/// The name of `value` under the theme in force, or `default` (its Earth name).
pub fn rename<T: std::fmt::Debug>(kind: Kind, value: &T, default: &'static str) -> &'static str {
    let Some(theme) = active() else { return default };
    if theme.table(kind).is_empty() {
        return default;
    }
    theme.rename_in(kind, &format!("{value:?}")).unwrap_or(default)
}

/// A proper noun or set word under the theme in force, or `default`.
pub fn term(key: &str, default: &'static str) -> &'static str {
    active().and_then(|t| t.terms.get(key)).map(String::as_str).unwrap_or(default)
}

/// Civilization III's calendar: turns of 50 years at first, shortening as history quickens, to 1
/// year a turn at the end - 540 turns from 4000 BC to 2050 AD. (turns, years a turn)
pub const AGES_CALENDAR: [(u32, i32); 8] = [(25, 50), (25, 40), (40, 25), (50, 20), (100, 10), (100, 5), (100, 2), (100, 1)];
/// The year the ages calendar ends on the last turn.
pub const AGES_END: i32 = 2050;

/// The year of `turn` (1 is the first). A world that counts `years_per_turn` years a turn does so
/// from `start`; one with 0 follows [`AGES_CALENDAR`] from 4000 BC, squeezed or stretched so that
/// the game's `last_turn` falls in 2050 AD.
pub fn game_year(start: i32, years_per_turn: i32, turn: u32, last_turn: u32) -> i32 {
    if years_per_turn != 0 {
        return start + (turn as i32 - 1) * years_per_turn;
    }
    let total: u32 = AGES_CALENDAR.iter().map(|(n, _)| n).sum();
    let mut left = (turn.saturating_sub(1)) as f64 * total as f64 / last_turn.saturating_sub(1).max(1) as f64;
    let mut year = start as f64;
    for (turns, step) in AGES_CALENDAR {
        let here = left.min(turns as f64);
        year += here * step as f64;
        left -= here;
        if left <= 0.0 {
            break;
        }
    }
    let year = year.round() as i32;
    // Whole decades while turns are long; there is no year 0.
    let year = if year < 1000 { (year as f64 / 5.0).round() as i32 * 5 } else { year };
    if year == 0 { 1 } else { year }
}

/// A year as the theme in force writes it.
pub fn year(year: i32) -> String {
    match active() {
        Some(theme) => theme.year(year),
        None => Theme::default().year(year),
    }
}

/// The folder under `assets/` that sprite sheets come from.
pub fn sprites() -> &'static str {
    active().and_then(|t| t.sprites.as_deref()).unwrap_or("sprites")
}

#[cfg(test)]
mod calendar_tests {
    use super::*;

    #[test]
    fn the_ages_calendar_runs_from_4000_bc_to_2050_ad_whatever_the_length() {
        for last in [200, 300, 400, 540] {
            assert_eq!(game_year(-4000, 0, 1, last), -4000);
            assert_eq!(game_year(-4000, 0, last, last), AGES_END, "{last}");
            let years: Vec<i32> = (1..=last).map(|t| game_year(-4000, 0, t, last)).collect();
            assert!(years.windows(2).all(|w| w[1] >= w[0]), "{last}: never goes back");
        }
        // Civ III's own milestones on its own length.
        for (turn, year) in [(26, -2750), (241, 1250), (441, 1950)] {
            assert!((game_year(-4000, 0, turn, 540) - year).abs() <= 5, "turn {turn}");
        }
        // Turns quicken: early turns span far more years than late ones.
        assert!(game_year(-4000, 0, 2, 400) - game_year(-4000, 0, 1, 400) > 50);
        assert!(game_year(-4000, 0, 400, 400) - game_year(-4000, 0, 399, 400) <= 2);
        // A scenario's fixed years per turn are kept.
        assert_eq!(game_year(1939, 1, 7, 400), 1945);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_renames_what_it_lists_and_counts_years_its_own_way() {
        let theme = Theme::parse(
            r#"{ "id": "xeno", "units": { "Musketman": "Plasma Trooper" }, "calendar": { "before": "{year} BL", "after": "Year {year} AL" },
                 "terms": { "destination": "Home" } }"#,
        )
        .unwrap();
        assert_eq!(theme.rename_in(Kind::Unit, "Musketman"), Some("Plasma Trooper"));
        assert_eq!(theme.rename_in(Kind::Unit, "Warrior"), None);
        assert_eq!(theme.rename_in(Kind::Tech, "Musketman"), None);
        assert_eq!(theme.year(-300), "300 BL");
        assert_eq!(theme.year(12), "Year 12 AL");
        assert_eq!(Theme::default().year(-4000), "4000 BC");
        assert_eq!(Theme::default().year(0), "1 AD");
        // Every built-in theme parses.
        for (id, json) in BUILT_IN {
            assert!(Theme::parse(json).is_ok(), "{id}");
        }
    }
}
