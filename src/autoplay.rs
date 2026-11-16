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

/// Plays `turns` turns of the scenario whose id (or name) contains `which`, and prints a report.
/// Returns false if the scenario is unknown.
pub fn run(which: &str, turns: u32) -> bool {
    let catalog = scenario::catalog();
    let Some(mut scenario) = catalog
        .into_iter()
        .filter(|s| !s.rows.is_empty())
        .find(|s| s.id.contains(which) || s.name.to_lowercase().contains(&which.to_lowercase()))
    else {
        eprintln!("No scenario matches '{which}'.");
        return false;
    };
    // An empty seat for the player, so that every real civilization is played by the computer.
    scenario.civs.insert(0, CivDef { name: "the Observers".into(), adjective: "Observer".into(), leader: "Nobody".into(), ..Default::default() });
    println!("Autoplay: {} for up to {turns} turns, {} civilizations.", scenario.name, scenario.civs.len() - 1);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    crate::init_game(&mut app);
    app.insert_resource(Observer);
    app.add_systems(Update, (crate::process_turn, crate::religion_turn, crate::visit_villages, crate::refresh_supplies).chain());
    app.insert_resource(NewGame { scenario, civ: 0, difficulty: crate::Difficulty::Regent, last_turn: turns.max(1) });
    if let Err(e) = app.world_mut().run_system_once(crate::setup_world) {
        eprintln!("The world could not be set up: {e}");
        return false;
    }
    app.update();

    let started = Instant::now();
    let mut seen_techs: HashSet<Tech> = HashSet::new();
    let mut eras_reached: BTreeMap<&str, u32> = BTreeMap::new();
    let mut log_seen = 0;
    // Cities changing hands (conquest, defection), and peoples wiped out.
    let mut owners: std::collections::HashMap<hexx::Hex, u32> = std::collections::HashMap::new();
    let mut changes = 0;
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
                        println!("  turn {turn:>3}: {} enter the {era} era ({}).", civ.name, tech.name());
                        turn
                    });
                }
            }
        }
        // Wonders, spaceships, governments and other news from the log.
        let log = world.resource::<GameLog>();
        let news: Vec<String> = log.lines.iter().skip(log_seen.min(log.lines.len())).cloned().collect();
        log_seen = log.lines.len();
        for line in news {
            let notable = ["priests found", "Wonder of the world", "launched a spaceship", "adopt", "golden age", "has been born", "Alpha Centauri", "cultural victory"];
            if notable.iter().any(|n| line.contains(n)) {
                println!("  turn {turn:>3}: {line}");
            }
        }
        {
            let mut q = world.query::<&City>();
            let now: Vec<(hexx::Hex, u32)> = q.iter(world).map(|c| (c.position, c.owner)).collect();
            for (at, owner) in &now {
                if owners.get(at).is_some_and(|o| o != owner) {
                    changes += 1;
                }
                owners.insert(*at, *owner);
            }
            let holding: HashSet<u32> = now.iter().map(|(_, o)| *o).collect();
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
        if turn % 25 == 0 {
            snapshot(world, turn);
        }
        if let Some(outcome) = world.resource::<GameState>().victory.clone() {
            println!("Game over on turn {turn}: {outcome}");
            println!("Cities that changed hands: {changes}.");
            snapshot(world, turn);
            armies(world);
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

/// Era of a technology, by its cost (as the science advisor groups them).
fn era_of(tech: Tech) -> &'static str {
    match tech.cost() {
        0..=25 => "Ancient",
        26..=45 => "Classical",
        46..=95 => "Industrial",
        _ => "Modern",
    }
}

/// The make-up of each civilization's military at the end: its most common unit types.
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
}

/// One line per civilization: cities, people, technologies, army, gold, government, culture, wonders.
fn snapshot(world: &mut World, turn: u32) {
    let civs = world.resource::<Civilizations>().clone();
    let space = world.resource::<SpaceRace>().clone();
    let culture = world.resource::<CultureLedger>().clone();
    let wonders = world.resource::<WorldWonders>().clone();
    let mut cities = world.query::<(&City, &CityBuildings)>();
    let city_list: Vec<(u32, u32, bool, bevy_civ::city::City)> = cities.iter(world).map(|(c, b)| (c.owner, c.population, b.buildings.len() > 3, c.clone())).collect();
    let mut units = world.query::<&Unit>();
    let armies: Vec<u32> = units.iter(world).filter(|u| u.unit_type.is_military()).map(|u| u.owner).collect();
    let groups: Vec<u32> = units.iter(world).filter(|u| u.is_army_group()).map(|u| u.owner).collect();
    println!("--- turn {turn} ---");
    for civ in civs.players.iter().filter(|c| c.id != PLAYER) {
        let owned: Vec<_> = city_list.iter().filter(|c| c.0 == civ.id).collect();
        if owned.is_empty() {
            continue;
        }
        let people: u32 = owned.iter().map(|c| c.1).sum();
        let army = armies.iter().filter(|o| **o == civ.id).count();
        let grouped = groups.iter().filter(|o| **o == civ.id).count();
        let grouped = if grouped > 0 { format!(" ({grouped} armies)") } else { String::new() };
        let points: u32 = owned.iter().map(|c| culture.of(c.3.position)).sum();
        let held = wonders.built.values().filter(|(o, _)| *o == civ.id).count();
        let ship = space.ships.get(&civ.id).map(|s| format!(", ship needs {}", if s.arrives.is_some() { "nothing (launched)".into() } else { s.missing() })).unwrap_or_default();
        println!(
            "  {:<14} {:>2} cities {:>3} people {:>2} techs {:>3} army{grouped} {:>5} gold  {:<14} culture {:>5}  wonders {}{ship}",
            civ.name.trim_start_matches("the "),
            owned.len(),
            people,
            civ.tech.researched.len(),
            army,
            civ.gold,
            civ.government.name(),
            points,
            held
        );
    }
}
