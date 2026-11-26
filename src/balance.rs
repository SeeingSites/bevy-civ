//! Many headless games between computer players, for measuring balance.
//!
//! `bevy-civ --balance [games] [turns] [directory]`
//!
//! Each game is a New World played under the real rules: the same systems as `--autoplay`, not the
//! dice-model `simulate` binaries. Games rotate through the map sizes and people counts the start
//! menu offers, unless `--size` or `--peoples` fixes one. `--shuffle` gives every people the same
//! chance of a seat and then shuffles that order, so the people that moves first is not always the
//! theme's first people.
//!
//! The directory receives `games.csv` (one row per game: victory, winner, end turn), `civs.csv`
//! (one row per people per game) and `summary.json` (win share per people, victory counts, end
//! turns, and whether each share sits in the fair band). The same summary is printed.
//!
//! A people is inside the fair band when its win rate is within five percentage points of an equal
//! share of the decided games it played. For two peoples that share is a half, so the band is
//! 45-55%. A short sample usually only says the rate is unclear. The run still succeeds: imbalance
//! is a finding, not a failed build. Nothing here runs unless `--balance` is set.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;

use bevy_civ::ai::Civilizations;
use bevy_civ::city::City;

use crate::scenario::{self, CivDef, NewGame};
use crate::{EndTurnNow, GameState, Observer};

/// How far a win rate may sit from an equal share and still count as fair.
/// Five points around a half is the 45-55% band.
pub const FAIR_MARGIN: f64 = 0.05;

const FAIR_BAND: &str =
    "within 5 percentage points of an equal share (45-55% when two peoples play)";

/// 95% Wilson score interval.
const WILSON_Z: f64 = 1.96;

pub struct Options {
    games: u32,
    turns: u32,
    out: String,
    shuffle: bool,
    from: u32,
    peoples: Option<usize>,
    map_size: Option<usize>,
    /// Game `i` draws its world from `seed + i`: the same command builds the same worlds.
    seed: Option<u64>,
    /// The expansion packs' peoples join the New World's.
    expansions: bool,
    /// Play this scenario (by id) instead of New Worlds.
    scenario: Option<String>,
}

#[derive(Clone, serde::Serialize)]
struct CivResult {
    seat: u32,
    name: String,
    traits: String,
    unique: String,
    aggression: f32,
    cities: u32,
    population: u32,
    technologies: u32,
    eliminated: bool,
    winner: bool,
}

#[derive(Clone, serde::Serialize)]
struct GameResult {
    game: u32,
    map: String,
    peoples: usize,
    shuffled: bool,
    end_turn: u32,
    victory: String,
    outcome: String,
    winner: Option<String>,
    seconds: f32,
    civs: Vec<CivResult>,
}

#[derive(serde::Serialize)]
struct CivShare {
    name: String,
    traits: String,
    unique: String,
    /// Games in which this people had a seat.
    games: u32,
    /// Those games that produced a winner.
    decided: u32,
    wins: u32,
    /// Wins divided by decided games this people played.
    win_rate: f64,
    /// The mean of an equal share (1 / peoples) over those decided games.
    fair_share: f64,
    /// Lower and upper bounds of a 95% Wilson interval on `win_rate`.
    low: f64,
    high: f64,
    /// `inside`, `unclear`, `outside` or `none` (no decided game).
    band: &'static str,
}

pub fn options_from(args: &[String]) -> Result<Options, String> {
    if !args.iter().any(|a| a == "--balance") {
        return Err(usage().into());
    }
    // Seats are shuffled unless asked otherwise: a fixed seat makes one people move first in every game.
    let mut shuffle = true;
    let mut seed = None;
    let mut expansions = false;
    let mut scenario_id = None;
    let mut peoples = None;
    let mut map_size = None;
    let mut from = 0u32;
    let mut i = 0;
    while i < args.len() {
        let next = args.get(i + 1).map(String::as_str);
        match args[i].as_str() {
            "--shuffle" => shuffle = true,
            "--fixed-seats" => shuffle = false,
            "--expansions" => expansions = true,
            "--seed" => {
                let value = next.ok_or_else(|| flag_needs("--seed", "a number"))?;
                seed = Some(value.parse::<u64>().map_err(|_| format!("--seed needs a whole number, not {value}"))?);
                i += 1;
            }
            "--scenario" => {
                let value = next.ok_or_else(|| flag_needs("--scenario", "a scenario id (earth, rome, discovery, napoleon, ww2...)"))?;
                scenario_id = Some(value.to_string());
                i += 1;
            }
            "--peoples" => {
                let value = next.ok_or_else(|| flag_needs("--peoples", "a number of peoples"))?;
                let n: usize = value
                    .parse()
                    .map_err(|_| format!("--peoples needs a whole number, not {value}"))?;
                if n < 2 {
                    return Err("--peoples must be at least 2".into());
                }
                peoples = Some(n);
                i += 1;
            }
            "--size" => {
                let value = next.ok_or_else(|| flag_needs("--size", "small, normal or large"))?;
                map_size = Some(map_size_of(value)?);
                i += 1;
            }
            "--from" => {
                let value = next.ok_or_else(|| flag_needs("--from", "a game number"))?;
                from = value
                    .parse()
                    .map_err(|_| format!("--from needs a whole number, not {value}"))?;
                i += 1;
            }
            "--theme" => {
                if next.is_none() {
                    return Err("--theme needs a theme id".into());
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    let balance = args
        .iter()
        .position(|a| a == "--balance")
        .expect("--balance");
    let mut positional = Vec::new();
    let mut j = balance + 1;
    while j < args.len() {
        if args[j].starts_with("--") {
            if !matches!(
                args[j].as_str(),
                "--shuffle" | "--fixed-seats" | "--expansions" | "--peoples" | "--size" | "--from" | "--theme" | "--seed" | "--scenario"
            ) {
                return Err(format!("unknown option {}\n{}", args[j], usage()));
            }
            if matches!(
                args[j].as_str(),
                "--peoples" | "--size" | "--from" | "--theme" | "--seed" | "--scenario"
            ) {
                j += 1;
            }
            j += 1;
            continue;
        }
        positional.push(args[j].as_str());
        j += 1;
    }

    let games = match positional.first() {
        None => 1,
        Some(v) => v
            .parse::<u32>()
            .map_err(|_| format!("the number of games must be a whole number, not {v}"))?,
    };
    if games == 0 {
        return Err("play at least one game".into());
    }
    let turns = match positional.get(1) {
        None => crate::LAST_TURN,
        Some(v) => v
            .parse::<u32>()
            .map_err(|_| format!("the turn limit must be a whole number, not {v}"))?,
    };
    if turns == 0 {
        return Err("the turn limit must be at least 1".into());
    }
    if positional.len() > 3 {
        return Err(format!("unexpected {}\n{}", positional[3], usage()));
    }
    let out = positional
        .get(2)
        .map(|s| (*s).to_string())
        .unwrap_or_else(|| "balance-out".into());
    Ok(Options {
        games,
        turns,
        out,
        shuffle,
        from,
        peoples,
        map_size,
        seed,
        expansions,
        scenario: scenario_id,
    })
}

fn usage() -> &'static str {
    "bevy-civ --balance [games] [turns] [directory] [--fixed-seats] [--peoples N] [--size small|normal|large] [--from N] [--seed N] [--expansions] [--scenario ID]"
}

fn flag_needs(flag: &str, what: &str) -> String {
    format!("{flag} needs {what}")
}

fn map_size_of(name: &str) -> Result<usize, String> {
    scenario::MAP_SIZES
        .iter()
        .position(|(n, _, _)| n.eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("--size must be tiny, small, normal, large or huge, not {name}"))
}

/// Map size and people count for game `index`.
///
/// With neither fixed, the sizes cycle every game and the people count steps once each size has
/// come round (small/3, normal/3, large/3, small/5, ...). A fixed size lets the people count step
/// every game, and the other way about.
fn schedule(index: u32, opts: &Options) -> (usize, usize) {
    let n = index as usize;
    let map_size = opts.map_size.unwrap_or(n % scenario::STANDARD_MAP_SIZES);
    let people_slot = if opts.map_size.is_none() {
        n / scenario::STANDARD_MAP_SIZES
    } else {
        n
    };
    let peoples = opts
        .peoples
        .unwrap_or(scenario::STANDARD_PEOPLE_CHOICES[people_slot % scenario::STANDARD_PEOPLE_CHOICES.len()]);
    (map_size, peoples)
}

/// The people kept in the game for certain. Unshuffled, that is the theme's first people, and it
/// is seated first, which is who moves first. Shuffled, any of them.
fn guaranteed_player(shuffle: bool, civs: usize, rng: &mut impl Rng) -> usize {
    if shuffle && civs > 0 {
        rng.gen_range(0..civs)
    } else {
        0
    }
}

/// Move order. Setup spawns peoples in list order; the AI researches and moves in that order
/// (the observer, inserted at the front afterwards, holds the human seat and does not play).
fn seat_order(civs: &mut [CivDef], shuffle: bool, rng: &mut impl Rng) {
    if shuffle {
        civs.shuffle(rng);
    }
}

fn victory_kind(outcome: &str) -> &'static str {
    const KINDS: &[(&str, &str)] = &[
        ("Conquest", "Conquest"),
        ("Domination", "Domination"),
        ("Cultural", "Cultural"),
        ("Space race", "Space"),
        ("Diplomatic", "Diplomatic"),
        ("Score", "Score"),
        ("Objectives", "Objectives"),
    ];
    if outcome.is_empty() {
        return "None";
    }
    KINDS
        .iter()
        .find(|(needle, _)| outcome.contains(needle))
        .map(|(_, kind)| *kind)
        .unwrap_or("Other")
}

/// The seated people named in a victory sentence. The longest name wins, so one people is not
/// matched inside another's name.
fn winner_name<'a>(outcome: &str, names: &'a [String]) -> Option<&'a str> {
    names
        .iter()
        .filter(|n| !n.is_empty() && outcome.contains(n.as_str()))
        .max_by_key(|n| n.len())
        .map(String::as_str)
}

fn wilson(wins: u32, decided: u32) -> (f64, f64) {
    if decided == 0 {
        return (0.0, 0.0);
    }
    let n = decided as f64;
    let p = wins as f64 / n;
    let z2 = WILSON_Z * WILSON_Z;
    let denom = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let margin = WILSON_Z * ((p * (1.0 - p) / n) + z2 / (4.0 * n * n)).sqrt();
    ((centre - margin) / denom, (centre + margin) / denom)
}

/// `inside` when the rate sits in the band, `outside` when the 95% interval misses it entirely,
/// `unclear` when the rate is out but the interval still overlaps, `none` with no decided game.
fn judge(wins: u32, decided: u32, fair: f64) -> (&'static str, f64, f64, f64) {
    if decided == 0 {
        return ("none", 0.0, 0.0, 0.0);
    }
    let rate = wins as f64 / decided as f64;
    let (low, high) = wilson(wins, decided);
    let lo = (fair - FAIR_MARGIN).max(0.0);
    let hi = (fair + FAIR_MARGIN).min(1.0);
    let point_inside = rate + 1e-9 >= lo && rate - 1e-9 <= hi;
    let interval_misses = high < lo || low > hi;
    let band = if point_inside {
        "inside"
    } else if interval_misses {
        "outside"
    } else {
        "unclear"
    };
    (band, rate, low, high)
}

fn shares(games: &[GameResult]) -> Vec<CivShare> {
    struct Acc {
        traits: String,
        unique: String,
        games: u32,
        decided: u32,
        wins: u32,
        fair_weight: f64,
    }
    let mut by_name: BTreeMap<String, Acc> = BTreeMap::new();
    for game in games {
        let equal = 1.0 / game.peoples.max(1) as f64;
        let decided = game.winner.is_some();
        for civ in &game.civs {
            let acc = by_name.entry(civ.name.clone()).or_insert_with(|| Acc {
                traits: civ.traits.clone(),
                unique: civ.unique.clone(),
                games: 0,
                decided: 0,
                wins: 0,
                fair_weight: 0.0,
            });
            acc.games += 1;
            if decided {
                acc.decided += 1;
                acc.fair_weight += equal;
                acc.wins += u32::from(civ.winner);
            }
        }
    }
    by_name
        .into_iter()
        .map(|(name, acc)| {
            let fair = if acc.decided == 0 {
                0.0
            } else {
                acc.fair_weight / acc.decided as f64
            };
            let (band, win_rate, low, high) = judge(acc.wins, acc.decided, fair);
            CivShare {
                name,
                traits: acc.traits,
                unique: acc.unique,
                games: acc.games,
                decided: acc.decided,
                wins: acc.wins,
                win_rate,
                fair_share: fair,
                low,
                high,
                band,
            }
        })
        .collect()
}

fn victory_counts(games: &[GameResult]) -> BTreeMap<String, u32> {
    let mut counts = BTreeMap::new();
    for game in games {
        *counts.entry(game.victory.clone()).or_default() += 1;
    }
    counts
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn csv_line(fields: &[String]) -> String {
    fields
        .iter()
        .map(|f| csv_field(f))
        .collect::<Vec<_>>()
        .join(",")
}

fn yn(value: bool) -> String {
    if value {
        "true".into()
    } else {
        "false".into()
    }
}

fn games_csv(games: &[GameResult]) -> String {
    let mut out = String::from("game,map,peoples,shuffled,end_turn,victory,winner\n");
    for game in games {
        let row = csv_line(&[
            game.game.to_string(),
            game.map.clone(),
            game.peoples.to_string(),
            yn(game.shuffled),
            game.end_turn.to_string(),
            game.victory.clone(),
            game.winner.clone().unwrap_or_default(),
        ]);
        out.push_str(&row);
        out.push('\n');
    }
    out
}

fn civs_csv(games: &[GameResult]) -> String {
    let mut out = String::from("game,seat,name,traits,unique,aggression,cities,population,technologies,eliminated,winner\n");
    for game in games {
        for civ in &game.civs {
            let row = csv_line(&[
                game.game.to_string(),
                civ.seat.to_string(),
                civ.name.clone(),
                civ.traits.clone(),
                civ.unique.clone(),
                civ.aggression.to_string(),
                civ.cities.to_string(),
                civ.population.to_string(),
                civ.technologies.to_string(),
                yn(civ.eliminated),
                yn(civ.winner),
            ]);
            out.push_str(&row);
            out.push('\n');
        }
    }
    out
}

fn pct(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn layout_phrase(opts: &Options) -> String {
    if let Some(id) = &opts.scenario {
        return format!("scenario {id}");
    }
    let extra = if opts.expansions { ", with the expansion peoples" } else { "" };
    let seed = opts.seed.map(|s| format!(", worlds from seed {s}")).unwrap_or_default();
    format!("{}{extra}{seed}", layout_only(opts))
}

fn layout_only(opts: &Options) -> String {
    match (opts.map_size, opts.peoples) {
        (Some(i), Some(n)) => format!("{}, {n} peoples", scenario::MAP_SIZES[i].0),
        (Some(i), None) => format!("{}, peoples rotate", scenario::MAP_SIZES[i].0),
        (None, Some(n)) => format!("map size rotates, {n} peoples"),
        (None, None) => "map size and peoples rotate".into(),
    }
}

fn text_report(opts: &Options, games: &[GameResult]) -> String {
    let seats = if opts.shuffle {
        "seats shuffled"
    } else {
        "the first people moves first"
    };
    let mut lines = vec![format!(
        "Balance: {} games, up to {} turns, {}, {seats}.",
        opts.games,
        opts.turns,
        layout_phrase(opts)
    )];
    if games.is_empty() {
        lines.push("No games finished.".into());
        return lines.join("\n");
    }
    let mean = games.iter().map(|g| g.end_turn as f64).sum::<f64>() / games.len() as f64;
    let decided = games.iter().filter(|g| g.winner.is_some()).count();
    let mut counts: Vec<(String, u32)> = victory_counts(games).into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let victories = if counts.is_empty() {
        "none".into()
    } else {
        counts
            .iter()
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut second = format!(
        "Mean end turn {mean:.1}. Decided {decided} of {}. Victories: {victories}.",
        games.len()
    );
    let undecided = games.len() - decided;
    if undecided > 0 {
        second.push_str(&format!(" {undecided} had no winner."));
    }
    lines.push(second);
    lines.push(format!("Win share ({FAIR_BAND}):"));
    lines.push("  people                 wins      share      fair    band".into());
    for civ in shares(games) {
        let band = if civ.band == "none" {
            "none".into()
        } else {
            format!(
                "{} ({:.0}-{:.0}%)",
                civ.band,
                civ.low * 100.0,
                civ.high * 100.0
            )
        };
        lines.push(format!(
            "  {:<20} {:>3} of {:<3} {:>7}   {:>7}   {band}",
            civ.name,
            civ.wins,
            civ.decided,
            pct(civ.win_rate),
            pct(civ.fair_share)
        ));
    }
    lines
        .push("A rate outside the band is a finding. This run does not fail because of it.".into());
    let dir = &opts.out;
    lines.push(format!(
        "Wrote {dir}/games.csv, {dir}/civs.csv, {dir}/summary.json."
    ));
    lines.join("\n")
}

fn summary_json(opts: &Options, games: &[GameResult]) -> Result<String, String> {
    let civs = shares(games);
    let decided = games.iter().filter(|g| g.winner.is_some()).count();
    let mean = if games.is_empty() {
        0.0
    } else {
        games.iter().map(|g| g.end_turn as f64).sum::<f64>() / games.len() as f64
    };
    let body = serde_json::json!({
        "games": games.len(),
        "decided": decided,
        "turn_limit": opts.turns,
        "shuffled": opts.shuffle,
        "mean_end_turn": mean,
        "fair_margin": FAIR_MARGIN,
        "fair_band": FAIR_BAND,
        "victory_types": victory_counts(games),
        "civs": civs,
        "results": games,
    });
    serde_json::to_string_pretty(&body)
        .map(|mut s| {
            s.push('\n');
            s
        })
        .map_err(|e| format!("could not write the summary: {e}"))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("could not write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(())
}

fn write_reports(dir: &str, opts: &Options, games: &[GameResult]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {dir}: {e}"))?;
    let base = PathBuf::from(dir);
    write_atomic(&base.join("games.csv"), games_csv(games).as_bytes())?;
    write_atomic(&base.join("civs.csv"), civs_csv(games).as_bytes())?;
    write_atomic(
        &base.join("summary.json"),
        summary_json(opts, games)?.as_bytes(),
    )?;
    Ok(())
}

/// Plays the batch. False when a game or the report could not be written.
pub fn run(opts: &Options) -> bool {
    let mut done = Vec::with_capacity(opts.games as usize);
    for n in 0..opts.games {
        let index = opts.from.saturating_add(n);
        match play(index, opts) {
            Ok(game) => {
                let who = game.winner.as_deref().unwrap_or("no winner");
                eprintln!(
                    "game {}: {} on turn {} ({who}) in {:.2}s",
                    game.game, game.victory, game.end_turn, game.seconds
                );
                done.push(game);
                if let Err(e) = write_reports(&opts.out, opts, &done) {
                    eprintln!("{e}");
                    return false;
                }
            }
            Err(e) => {
                eprintln!("game {index}: {e}");
                return false;
            }
        }
    }
    println!("{}", text_report(opts, &done));
    true
}

fn observer() -> CivDef {
    CivDef {
        name: "the Observers".into(),
        adjective: "Observer".into(),
        leader: "Nobody".into(),
        ..Default::default()
    }
}

fn play(index: u32, opts: &Options) -> Result<GameResult, String> {
    let started = Instant::now();
    let (map_size, wanted) = schedule(index, opts);
    let map_size = map_size.min(scenario::MAP_SIZES.len() - 1);
    let mut rng = rand::thread_rng();
    let seed = opts.seed.map(|s| s.wrapping_add(index as u64));
    let (mut scenario, seated) = if let Some(id) = &opts.scenario {
        // A fixed scenario: its own peoples and map; only the move order is shuffled.
        let mut scenario = scenario::catalog_with(|_| true)
            .into_iter()
            .find(|s| &s.id == id && !s.rows.is_empty())
            .ok_or_else(|| format!("No scenario has the id {id}."))?;
        scenario.civs.retain(|c| !c.optional);
        scenario.seed = seed;
        seat_order(&mut scenario.civs, opts.shuffle, &mut rng);
        let seated = scenario.civs.len();
        (scenario, seated)
    } else {
        let mut template = scenario::catalog_with(|_| opts.expansions)
            .into_iter()
            .find(|s| s.id == "random")
            .ok_or("The New World scenario is missing.")?;
        // The packs' peoples (with --expansions) are ordinary candidates here.
        for civ in &mut template.civs {
            civ.optional = false;
        }
        if template.civs.is_empty() {
            return Err("The New World has no peoples.".into());
        }
        template.seed = seed;
        let player = match seed {
            // The same seed seats the same peoples.
            Some(s) if opts.shuffle => (s % template.civs.len() as u64) as usize,
            _ => guaranteed_player(opts.shuffle, template.civs.len(), &mut rng),
        };
        let (mut scenario, _) = scenario::generate_random(&template, player, map_size, wanted);
        // Shuffling after the homes are chosen separates who moves first from who was placed first.
        seat_order(&mut scenario.civs, opts.shuffle, &mut rng);
        let seated = scenario.civs.len();
        let cap = template.civs.len();
        let expected = wanted.min(cap).max(2.min(cap));
        if seated != expected {
            eprintln!("game {index}: seated {seated} peoples (asked for {expected})");
        }
        (scenario, seated)
    };
    scenario.civs.insert(0, observer());

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    crate::init_game(&mut app);
    app.insert_resource(Observer);
    // The same turn systems as `--autoplay`, so the balance run is a real game.
    crate::autoplay::add_turn_systems(&mut app);
    app.insert_resource(NewGame {
        scenario,
        civ: 0,
        difficulty: crate::Difficulty::Regent,
        last_turn: opts.turns.max(1),
        map_size,
        peoples: seated,
        barbarians: bevy_civ::barbarians::DEFAULT_LEVEL,
        online: 0,
    });
    if let Err(e) = app.world_mut().run_system_once(crate::setup_world) {
        return Err(format!("The world could not be set up: {e}"));
    }
    // One update so the world exists before the first end-of-turn, as autoplay does.
    app.update();

    let mut end_turn = app.world().resource::<GameState>().turn;
    for _ in 0..opts.turns.max(1) {
        app.world_mut().send_event(EndTurnNow);
        app.update();
        end_turn = app.world().resource::<GameState>().turn;
        if app.world().resource::<GameState>().victory.is_some() {
            break;
        }
    }

    let world = app.world_mut();
    let outcome = world
        .resource::<GameState>()
        .victory
        .clone()
        .unwrap_or_default();
    let civs = world.resource::<Civilizations>().clone();
    let pops: Vec<(u32, u32)> = {
        let mut cities = world.query::<&City>();
        cities
            .iter(world)
            .map(|c| (c.owner, c.population))
            .collect()
    };
    let playing: Vec<_> = civs.players.iter().filter(|c| !c.is_human).collect();
    let names: Vec<String> = playing.iter().map(|c| c.name.clone()).collect();
    let winner = winner_name(&outcome, &names).map(str::to_string);
    let civ_rows = playing
        .iter()
        .map(|c| {
            let character = bevy_civ::traits::of(&c.name);
            let owned: Vec<u32> = pops
                .iter()
                .filter(|(owner, _)| *owner == c.id)
                .map(|(_, pop)| *pop)
                .collect();
            CivResult {
                seat: c.id,
                name: c.name.clone(),
                traits: character.describe(),
                unique: character
                    .unique_unit()
                    .map(|u| u.name().to_string())
                    .unwrap_or_else(|| "none".into()),
                aggression: c.aggression,
                cities: owned.len() as u32,
                population: owned.iter().sum(),
                technologies: c.tech.researched.len() as u32,
                eliminated: c.eliminated,
                winner: winner.as_deref() == Some(c.name.as_str()),
            }
        })
        .collect();

    Ok(GameResult {
        game: index,
        map: opts.scenario.clone().unwrap_or_else(|| scenario::MAP_SIZES[map_size].0.to_string()),
        peoples: seated,
        shuffled: opts.shuffle,
        end_turn,
        victory: victory_kind(&outcome).to_string(),
        outcome,
        winner,
        seconds: started.elapsed().as_secs_f32(),
        civs: civ_rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn defaults_and_flags() {
        let only = options_from(&args(&["bevy-civ", "--balance"])).unwrap();
        assert_eq!(only.games, 1);
        assert_eq!(only.turns, crate::LAST_TURN);
        assert_eq!(only.out, "balance-out");
        assert!(only.shuffle, "seats are shuffled unless asked otherwise");
        assert_eq!((only.seed, only.expansions, only.scenario.as_deref()), (None, false, None));
        let fixed = options_from(&args(&["bevy-civ", "--balance", "3", "--fixed-seats", "--seed", "7", "--expansions", "--scenario", "ww2", "100"])).unwrap();
        assert!(!fixed.shuffle);
        assert_eq!((fixed.games, fixed.turns, fixed.seed, fixed.expansions, fixed.scenario.as_deref()), (3, 100, Some(7), true, Some("ww2")));
        assert!(options_from(&args(&["bevy-civ", "--balance", "--seed", "x"])).is_err());
        assert_eq!(only.from, 0);
        assert_eq!(only.peoples, None);
        assert_eq!(only.map_size, None);

        let mixed = options_from(&args(&[
            "bevy-civ",
            "--theme",
            "earth",
            "--shuffle",
            "--balance",
            "4",
            "--peoples",
            "3",
            "--size",
            "small",
            "--from",
            "10",
            "80",
            "out/here",
        ]))
        .unwrap();
        assert_eq!(mixed.games, 4);
        assert_eq!(mixed.turns, 80);
        assert_eq!(mixed.out, "out/here");
        assert!(mixed.shuffle);
        assert_eq!(mixed.from, 10);
        assert_eq!(mixed.peoples, Some(3));
        assert_eq!(mixed.map_size, Some(0));
        assert!(options_from(&args(&["bevy-civ", "--balance", "--nope"])).is_err());
        assert!(options_from(&args(&["bevy-civ", "--balance", "0"])).is_err());
    }

    #[test]
    fn a_seed_draws_the_same_world_again() {
        let mut template = scenario::catalog_with(|_| false).into_iter().find(|s| s.id == "random").unwrap();
        template.seed = Some(42);
        let (a, _) = scenario::generate_random(&template, 2, 0, 5);
        let (b, _) = scenario::generate_random(&template, 2, 0, 5);
        assert_eq!(a.rows, b.rows, "same land");
        let homes = |s: &scenario::Scenario| s.civs.iter().map(|c| (c.name.clone(), c.cities[0].col, c.cities[0].row)).collect::<Vec<_>>();
        assert_eq!(homes(&a), homes(&b), "same peoples in the same homes");
        template.seed = Some(43);
        let (c, _) = scenario::generate_random(&template, 2, 0, 5);
        assert_ne!(a.rows, c.rows, "another seed, another world");
    }

    #[test]
    fn schedule_rotates_size_then_peoples() {
        let opts = options_from(&args(&["bevy-civ", "--balance"])).unwrap();
        assert_eq!(schedule(0, &opts), (0, scenario::STANDARD_PEOPLE_CHOICES[0]));
        assert_eq!(schedule(1, &opts), (1, scenario::STANDARD_PEOPLE_CHOICES[0]));
        let after_sizes = scenario::STANDARD_MAP_SIZES as u32;
        assert_eq!(
            schedule(after_sizes, &opts),
            (0, scenario::STANDARD_PEOPLE_CHOICES[1])
        );

        let fixed = options_from(&args(&[
            "bevy-civ",
            "--balance",
            "--size",
            "large",
            "--peoples",
            "8",
        ]))
        .unwrap();
        assert_eq!(schedule(0, &fixed), (2, 8));
        assert_eq!(schedule(5, &fixed), (2, 8));
    }

    #[test]
    fn shuffle_changes_who_is_seated_first_and_who_moves_first() {
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(guaranteed_player(false, 8, &mut rng), 0);
        let picked: Vec<usize> = (0..24)
            .map(|_| guaranteed_player(true, 8, &mut rng))
            .collect();
        assert!(picked.iter().any(|p| *p != 0));
        assert!(picked.iter().all(|p| *p < 8));

        let original: Vec<CivDef> = ["Rome", "Egypt", "Greece"]
            .into_iter()
            .map(|name| CivDef {
                name: name.into(),
                ..Default::default()
            })
            .collect();
        let mut stayed = original.clone();
        seat_order(&mut stayed, false, &mut rng);
        assert_eq!(stayed[0].name, "Rome");
        let changed = (0..30u64).any(|seed| {
            let mut civs = original.clone();
            seat_order(&mut civs, true, &mut StdRng::seed_from_u64(seed));
            civs[0].name != "Rome"
        });
        assert!(
            changed,
            "a shuffle should be able to move another people first"
        );
    }

    #[test]
    fn victory_sentences_name_the_winner_and_the_kind() {
        let names = vec![
            "Rome".into(),
            "Egypt".into(),
            "Austria".into(),
            "Austria-Hungary".into(),
        ];
        let cases = [
            ("Conquest victory for Rome.", "Conquest", Some("Rome")),
            (
                "Domination victory for Egypt (8 of 12 cities).",
                "Domination",
                Some("Egypt"),
            ),
            ("Cultural victory for Rome.", "Cultural", Some("Rome")),
            ("Space race victory for Egypt.", "Space", Some("Egypt")),
            ("Diplomatic victory for Rome.", "Diplomatic", Some("Rome")),
            (
                "Score victory for Egypt (400) at the turn limit.",
                "Score",
                Some("Egypt"),
            ),
            (
                "Score victory for Austria-Hungary (10) at the turn limit.",
                "Score",
                Some("Austria-Hungary"),
            ),
            ("", "None", None),
        ];
        for (sentence, kind, winner) in cases {
            assert_eq!(victory_kind(sentence), kind, "{sentence}");
            assert_eq!(winner_name(sentence, &names), winner, "{sentence}");
        }
    }

    #[test]
    fn fair_band_splits_a_sweep_from_a_short_sample() {
        assert_eq!(judge(0, 0, 0.5).0, "none");
        let (band, rate, _, _) = judge(5, 10, 0.5);
        assert_eq!(band, "inside");
        assert!((rate - 0.5).abs() < 1e-9);
        // One win in one game is 100%, far from 45-55%, but the interval still covers a half.
        assert_eq!(judge(1, 1, 0.5).0, "unclear");
        let (band, _, low, high) = judge(10, 10, 0.5);
        assert_eq!(band, "outside");
        assert!(low > 0.55 && high > 0.99, "{low}..{high}");
        let (band, _, _, high) = judge(0, 40, 0.5);
        assert_eq!(band, "outside");
        assert!(high < 0.45, "{high}");
    }

    #[test]
    fn win_share_uses_an_equal_split_of_decided_games() {
        let rome = |winner: bool| CivResult {
            seat: 1,
            name: "Rome".into(),
            traits: "Militaristic and Commercial".into(),
            unique: "Praetorian".into(),
            aggression: 0.7,
            cities: 3,
            population: 8,
            technologies: 4,
            eliminated: false,
            winner,
        };
        let egypt = |seat: u32, winner: bool| CivResult {
            seat,
            name: "Egypt".into(),
            winner,
            ..rome(false)
        };
        let game = |n: u32, winner: &str, civs: Vec<CivResult>| GameResult {
            game: n,
            map: "Small".into(),
            peoples: civs.len(),
            shuffled: true,
            end_turn: 40,
            victory: "Score".into(),
            outcome: format!("Score victory for {winner} (10) at the turn limit."),
            winner: Some(winner.into()),
            seconds: 0.1,
            civs,
        };
        let games = vec![
            game(0, "Rome", vec![rome(true), egypt(2, false)]),
            game(1, "Egypt", vec![egypt(1, true), rome(false)]),
        ];
        let rows = shares(&games);
        let rome = rows.iter().find(|c| c.name == "Rome").unwrap();
        let egypt = rows.iter().find(|c| c.name == "Egypt").unwrap();
        assert_eq!((rome.wins, rome.decided, rome.band), (1, 2, "inside"));
        assert!((rome.fair_share - 0.5).abs() < 1e-9);
        assert_eq!(egypt.wins, 1);
        assert_eq!(egypt.band, "inside");
    }
}
