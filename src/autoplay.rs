//! A whole game played by the computer, headless: `bevy-civ --autoplay [scenario] [turns]`.
//!
//! The player's seat is taken by an empty observer civilization, so every real civilization is
//! played by the AI, under exactly the rules of the game. The run prints a snapshot of the world
//! every 25 turns and the milestones as they happen - eras, wonders, governments, spaceships - and
//! how the game ends. It is the tool for finding crashes, stalls and pacing problems.

use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use bevy_civ::ai::Civilizations;
use bevy_civ::building::{CityBuildings, WorldWonders};
use bevy_civ::city::City;
use bevy_civ::culture::CultureLedger;
use bevy_civ::space::SpaceRace;
use bevy_civ::tech::Tech;
use bevy_civ::unit::Unit;

use crate::scenario::{self, CivDef, NewGame};
use crate::{EndTurnNow, GameLog, GameState, Observer, PLAYER};

/// The systems that play a turn without a window: what `--autoplay` and `--balance` both run, so a
/// headless game is the same game as one on screen.
pub fn add_turn_systems(app: &mut App) {
    app.add_systems(Update, (crate::stow_cargo, crate::process_turn, crate::religion_turn, crate::visit_villages, crate::refresh_supplies, crate::work_tiles, crate::nuclear_strikes).chain());
}

/// Plays `turns` turns of the scenario whose id (or name) contains `which`, and prints a report.
/// Returns false if the scenario is unknown.
pub fn run(which: &str, turns: u32) -> bool {
    let catalog = scenario::catalog_with(|_| true);
    let Some(mut scenario) = catalog
        .into_iter()
        .filter(|s| !s.rows.is_empty())
        .find(|s| s.id.contains(which) || s.name.to_lowercase().contains(&which.to_lowercase()))
    else {
        eprintln!("No scenario matches '{which}'.");
        return false;
    };
    // The packs' optional peoples stay out, as when a player leads one of the scenario's own.
    scenario.civs.retain(|c| !c.optional);
    // An empty seat for the player, so that every real civilization is played by the computer.
    scenario.civs.insert(0, CivDef { name: "the Observers".into(), adjective: "Observer".into(), leader: "Nobody".into(), ..Default::default() });
    println!("Autoplay: {} for up to {turns} turns, {} civilizations.", scenario.name, scenario.civs.len() - 1);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    crate::init_game(&mut app);
    app.insert_resource(Observer);
    add_turn_systems(&mut app);
    app.insert_resource(NewGame { scenario, civ: 0, difficulty: crate::Difficulty::Regent, last_turn: turns.max(1), map_size: 1, peoples: scenario::DEFAULT_PEOPLES });
    if let Err(e) = app.world_mut().run_system_once(crate::setup_world) {
        eprintln!("The world could not be set up: {e}");
        return false;
    }
    app.update();

    let started = Instant::now();
    let mut seen_techs: HashSet<Tech> = HashSet::new();
    let mut eras_reached: BTreeMap<&str, u32> = BTreeMap::new();
    // What the scenario starts with is no milestone.
    for civ in &app.world().resource::<Civilizations>().players {
        for tech in &civ.tech.researched {
            seen_techs.insert(*tech);
            eras_reached.insert(era_of(*tech), 0);
        }
    }
    let mut log_seen = 0;
    // Cities changing hands (conquest, defection), and peoples wiped out.
    let mut owners: std::collections::HashMap<hexx::Hex, u32> = std::collections::HashMap::new();
    let mut changes = 0;
    let mut between = 0;
    let mut alive: HashSet<u32> = HashSet::new();
    for _ in 0..turns {
        app.world_mut().send_event(EndTurnNow);
        app.update();
        let world = app.world_mut();
        let turn = world.resource::<GameState>().turn;

        // Milestones: the first civilization to reach each era.
        let civs = world.resource::<Civilizations>();
        for civ in &civs.players {
            for tech in &civ.tech.researched {
                if seen_techs.insert(*tech) {
                    let era = era_of(*tech);
                    eras_reached.entry(era).or_insert_with(|| {
                        println!("  turn {turn:>3}: {} enter the {era} ({}).", civ.name, tech.name());
                        turn
                    });
                }
            }
        }
        // Wonders, spaceships, governments and other news from the log.
        let log = world.resource::<GameLog>();
        // Lines written since the last look (the log itself keeps only the latest ones).
        let fresh = log.written.saturating_sub(log_seen).min(log.lines.len());
        let news: Vec<String> = log.lines.iter().skip(log.lines.len() - fresh).cloned().collect();
        log_seen = log.written;
        for line in news {
            let notable = ["Secretary-General", "first to master", "missionaries bring", "nuclear missile", "lifts off", "have launched a spaceship", "priests found", "Wonder of the world", "launched a spaceship", "adopt", "golden age", "has been born", "Alpha Centauri", "cultural victory"];
            if notable.iter().any(|n| line.contains(n)) {
                println!("  turn {turn:>3}: {line}");
            }
        }
        {
            let mut q = world.query::<&City>();
            let now: Vec<(hexx::Hex, u32)> = q.iter(world).map(|c| (c.position, c.owner)).collect();
            for (at, owner) in &now {
                if let Some(old) = owners.get(at).filter(|o| *o != owner) {
                    changes += 1;
                    if *old != bevy_civ::barbarians::BARBARIANS && *owner != bevy_civ::barbarians::BARBARIANS {
                        between += 1;
                    }
                }
                owners.insert(*at, *owner);
            }
            let holding: HashSet<u32> = now.iter().map(|(_, o)| *o).filter(|o| *o != bevy_civ::barbarians::BARBARIANS).collect();
            if alive.is_empty() {
                alive = holding.clone();
            }
            let names = world.resource::<Civilizations>().players.iter().map(|c| (c.id, c.name.clone())).collect::<Vec<_>>();
            for gone in alive.difference(&holding) {
                let name = names.iter().find(|(id, _)| id == gone).map(|(_, n)| n.clone()).unwrap_or_else(|| "The barbarians".into());
                println!("  turn {turn:>3}: {name} have lost their last city.");
            }
            alive = holding;
        }
        // For tuning the cultural victory: the leading peoples' culture every five turns.
        if turn % 5 == 0 && std::env::var_os("BEVY_CIV_CULTURE_TRACE").is_some() {
            let ledger = world.resource::<CultureLedger>().clone();
            let mut q = world.query::<&City>();
            let totals = ledger.totals(q.iter(world).filter(|c| c.owner != bevy_civ::barbarians::BARBARIANS).map(|c| (c.position, c.owner)));
            let mut top: Vec<u32> = totals.values().copied().collect();
            top.sort_unstable_by(|a, b| b.cmp(a));
            top.resize(3.max(top.len()), 0);
            let owners: Vec<(hexx::Hex, u32)> = q.iter(world).map(|c| (c.position, c.owner)).collect();
            let best_city = ledger.best_city(&owners).map_or(0, |(_, _, points)| points);
            println!("culture-trace {turn} {} city {best_city}", top.iter().take(3).map(|c| c.to_string()).collect::<Vec<_>>().join(" "));
        }
        if turn % 25 == 0 {
            snapshot(world, turn);
        }
        if let Some(outcome) = world.resource::<GameState>().victory.clone() {
            println!("Game over on turn {turn}: {outcome}");
            println!("Cities that changed hands: {changes} ({between} between civilizations).");
            println!("Technologies traded between computer players: {}.", world.resource::<crate::foreign::Foreign>().tech_trades);
            dominance(world);
            snapshot(world, turn);
            armies(world);
            troubles(world);
            break;
        }
    }
    let state = app.world().resource::<GameState>();
    if state.victory.is_none() {
        println!("No winner after {} turns.", state.turn);
    }
    println!("Took {:.1}s ({:.0} ms a turn).", started.elapsed().as_secs_f32(), started.elapsed().as_millis() as f32 / state.turn.max(1) as f32);
    true
}

/// How much of the world the strongest peoples hold at the end: cities, land within their borders
/// and citizens, each as a share of the world's (for weighing domination rules).
fn dominance(world: &mut World) {
    let map = world.resource::<crate::MapData>();
    let land = map.cells.len().saturating_sub(map.water.len()).max(1);
    let water = map.water.clone();
    let territory = world.resource::<crate::supply::Supplies>().territory.clone();
    let mut q = world.query::<&City>();
    let cities: Vec<(u32, u32)> = q.iter(world).filter(|c| c.owner != bevy_civ::barbarians::BARBARIANS).map(|c| (c.owner, c.population)).collect();
    let (all_cities, all_people) = (cities.len().max(1), cities.iter().map(|c| c.1).sum::<u32>().max(1));
    let mut owners: Vec<u32> = cities.iter().map(|c| c.0).collect();
    owners.sort_unstable();
    owners.dedup();
    let mut rows: Vec<(f32, f32, f32, u32)> = owners
        .into_iter()
        .map(|o| {
            let held = cities.iter().filter(|c| c.0 == o).count() as f32 / all_cities as f32;
            let tiles = territory.iter().filter(|(at, (owner, _))| *owner == o && !water.contains(at)).count() as f32 / land as f32;
            let people = cities.iter().filter(|c| c.0 == o).map(|c| c.1).sum::<u32>() as f32 / all_people as f32;
            (held, tiles, people, o)
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let text: Vec<String> = rows.iter().take(3).map(|(c, l, p, o)| format!("#{o}: cities {:.0}%, land {:.0}%, people {:.0}%", c * 100.0, l * 100.0, p * 100.0)).collect();
    println!("--- dominance: {}", text.join("; "));
}

/// Era of a technology, by its cost (as the science advisor groups them).
fn era_of(tech: Tech) -> &'static str {
    tech.era().name()
}

/// The make-up of each civilization's military at the end: its most common unit types.
/// The modern troubles at the end: drugs, food scandals, the buildings against them, migration.
fn troubles(world: &mut World) {
    use bevy_civ::building::{Building, CityBuildings};
    let mut cities = world.query::<(&bevy_civ::city::City, &CityBuildings)>();
    let (mut drugs, mut scandals, mut police, mut safety, mut gm) = (0, 0, 0, 0, 0);
    for (city, buildings) in cities.iter(world) {
        drugs += usize::from(city.narcotics > 0);
        scandals += usize::from(city.scandal > 0);
        police += usize::from(buildings.has(Building::PoliceStation));
        safety += usize::from(buildings.has(Building::FoodSafety));
        gm += usize::from(buildings.has(Building::GmFarms));
    }
    {
        let mut all: BTreeMap<&str, usize> = BTreeMap::new();
        let mut q = world.query::<&CityBuildings>();
        for b in q.iter(world) {
            for building in b.buildings.iter().filter(|b| !b.is_wonder()) {
                *all.entry(building.name()).or_default() += 1;
            }
        }
        let n = world.query::<&bevy_civ::city::City>().iter(world).count();
        let list: Vec<String> = all.iter().map(|(k, v)| format!("{k} {v}")).collect();
        println!("--- buildings in {n} cities: {}", list.join(", "));
    }
    {
        // Growth: how many cities are held at the size their water supply allows.
        let rivers = world.resource::<crate::Rivers>().tiles.clone();
        let mut q = world.query::<(&bevy_civ::city::City, &CityBuildings)>();
        let (mut capped, mut total, mut people) = (0, 0, 0);
        for (city, built) in q.iter(world) {
            let fresh = rivers.contains(&city.position) || city.position.all_neighbors().iter().any(|n| rivers.contains(n));
            total += 1;
            people += city.population;
            capped += usize::from(city.population >= Building::size_limit(&built.buildings, fresh));
        }
        println!("--- growth: {capped} of {total} cities at their water limit; {:.1} citizens a city", people as f32 / total.max(1) as f32);
    }
    let (moved, abroad) = world.resource::<GameState>().migrations;
    let (cartels, cells, raids) = world.resource::<GameState>().raiders;
    println!("--- troubles: {drugs} cities with a drug trade, {scandals} in a food scandal; {police} Police Stations, {safety} Food Safety Agencies, {gm} GM Farms; {moved} migrants ({abroad} abroad)");
    println!("--- raiders: {cartels} cartel bands and {cells} terror cells formed; {raids} raids on undefended cities");
}

fn armies(world: &mut World) {
    let civs = world.resource::<Civilizations>().clone();
    let mut units = world.query::<&Unit>();
    let all: Vec<(u32, bevy_civ::unit::UnitType)> = units.iter(world).filter(|u| u.unit_type.is_military()).map(|u| (u.owner, u.unit_type)).collect();
    println!("--- armies ---");
    for civ in civs.players.iter().filter(|c| c.id != PLAYER) {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, kind) in all.iter().filter(|(o, _)| *o == civ.id) {
            *counts.entry(kind.name()).or_default() += 1;
        }
        let mut list: Vec<(&str, usize)> = counts.into_iter().collect();
        list.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        let text: Vec<String> = list.iter().take(6).map(|(k, n)| format!("{n} {k}")).collect();
        println!("  {:<14} {}", civ.name.trim_start_matches("the "), text.join(", "));
    }
    {
        let mut civ_units: BTreeMap<&str, usize> = BTreeMap::new();
        for u in units.iter(world).filter(|u| !u.unit_type.is_military() && u.owner != PLAYER && u.owner != bevy_civ::barbarians::BARBARIANS) {
            *civ_units.entry(u.unit_type.name()).or_default() += 1;
        }
        println!("--- civilian units: {}", civ_units.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", "));
    }
    let t = world.resource::<bevy_civ::combat::BattleTally>().clone();
    let share = |n: u32| 100 * n / t.battles.max(1);
    println!(
        "--- battles: {} ({}% won by the attacker, {}% pulled back), {} promotions, {} won by elites, {} generals born; {} bombardments took {} hit points",
        t.battles, share(t.won), share(t.retreated), t.promotions, t.elite_wins, t.generals, t.bombardments, t.shelled
    );
    let ferries = world.resource::<bevy_civ::ai::Ferries>();
    let landed: Vec<String> = civs.players.iter().filter_map(|c| ferries.landed.get(&c.id).map(|n| format!("{} {n}", c.name.trim_start_matches("the ")))).collect();
    println!("--- landed overseas: {}", if landed.is_empty() { "none".to_string() } else { landed.join(", ") });
}

/// One line per civilization: cities, people, technologies, army, gold, government, culture, wonders.
fn snapshot(world: &mut World, turn: u32) {
    let civs = world.resource::<Civilizations>().clone();
    let space = world.resource::<SpaceRace>().clone();
    let culture = world.resource::<CultureLedger>().clone();
    let wonders = world.resource::<WorldWonders>().clone();
    let accounts = world.resource::<GameState>().accounts.clone();
    let mut cities = world.query::<(&City, &CityBuildings)>();
    let city_list: Vec<(u32, u32, bool, bevy_civ::city::City)> = cities.iter(world).map(|(c, b)| (c.owner, c.population, b.buildings.len() > 3, c.clone())).collect();
    let mut units = world.query::<&Unit>();
    let armies: Vec<u32> = units.iter(world).filter(|u| u.unit_type.is_military()).map(|u| u.owner).collect();
    let groups: Vec<(u32, u32)> = units.iter(world).filter(|u| u.is_army_group()).map(|u| (u.owner, u.members())).collect();
    println!("--- turn {turn} ---");
    for civ in civs.players.iter().filter(|c| c.id != PLAYER) {
        let owned: Vec<_> = city_list.iter().filter(|c| c.0 == civ.id).collect();
        if owned.is_empty() {
            continue;
        }
        let people: u32 = owned.iter().map(|c| c.1).sum();
        let army = armies.iter().filter(|o| **o == civ.id).count();
        let sizes: Vec<String> = groups.iter().filter(|(o, _)| *o == civ.id).map(|(_, m)| m.to_string()).collect();
        let grouped = if sizes.is_empty() { String::new() } else { format!(" ({} armies: {})", sizes.len(), sizes.join("+")) };
        let points = culture.of_people(civ.id);
        let held = wonders.built.values().filter(|(o, _)| *o == civ.id).count();
        let ship = space.ships.get(&civ.id).map(|s| format!(", ship needs {}", if s.arrives.is_some() { "nothing (launched)".into() } else { s.missing() })).unwrap_or_default();
        // The economy: trade from the land worked, the share lost to corruption, the upkeep of
        // buildings, the rates (science/luxury) and the science it all comes to.
        let trade: u32 = owned.iter().map(|c| c.3.land.2).sum();
        let books = accounts.get(&civ.id).copied().unwrap_or_default();
        let corrupt = books.corruption * 100 / books.trade.max(1);
        let economy = format!("corr {corrupt:>2}% upk {:>3} rates {}/{}", books.upkeep, civ.rates.science, civ.rates.luxury);
        println!(
            "  {:<14} {:>2} cities {:>3} people {:>2} techs {:>3} army{grouped} {:>5} gold  {:<14} culture {:>5}  wonders {}  trade {:>4} sci {:>4} {economy}{ship}",
            civ.name.trim_start_matches("the "),
            owned.len(),
            people,
            civ.tech.researched.len(),
            army,
            civ.gold,
            civ.government.name(),
            points,
            held,
            trade,
            civ.tech.science_per_turn
        );
    }
}
