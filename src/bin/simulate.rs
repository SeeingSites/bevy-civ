//! Headless simulator that runs many games and evaluates gameplay balance.
//!
//! Usage: cargo run --bin simulate -- --games 100 --max-turns 200

use bevy::prelude::*;
use bevy_civ::*;
use hexx::{shapes, Hex, HexLayout};
use rand::Rng;
use std::env;

const HEX_SIZE: f32 = 28.0;
const MAP_RADIUS: u32 = 8;

#[derive(Clone)]
struct SimResult {
    turns: u32,
    victory: String,
    score: u32,
    cities: u32,
    population: u32,
    techs: u32,
    gold: i32,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let num_games = args.iter().position(|a| a == "--games")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let max_turns = args.iter().position(|a| a == "--max-turns")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(150);

    println!("Running {} games (max {} turns each)...", num_games, max_turns);

    let mut results = Vec::new();
    let mut rng = rand::thread_rng();

    for game_id in 0..num_games {
        if game_id % 10 == 0 {
            println!("Game {}/{}", game_id + 1, num_games);
        }
        let result = run_single_game(max_turns, &mut rng);
        results.push(result);
    }

    print_statistics(&results);
}

fn run_single_game(max_turns: u32, rng: &mut impl Rng) -> SimResult {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    app.insert_resource(GameStateSim {
        turn: 1,
        gold: 0,
        government: Government::Despotism,
        victory: None,
    });
    app.insert_resource(TechState::default());
    app.insert_resource(Civilizations::default());
    app.insert_resource(Diplomacy::default());
    app.insert_resource(BarbarianState::default());
    app.insert_resource(TradeRoutes::default());
    app.insert_resource(SpaceRace::default());
    app.insert_resource(FogOfWar::default());

    let layout = HexLayout::flat().with_hex_size(HEX_SIZE);
    app.insert_resource(MapDataSim { layout });

    let mut world = app.world_mut();

    for hex in shapes::hexagon(Hex::ZERO, MAP_RADIUS) {
        let terrain = random_terrain(rng);
        let resource = random_resource(rng, terrain);
        world.spawn(HexTile {
            coord: hex,
            terrain,
            improvements: vec![],
            resource,
        });
    }

    world.spawn((
        City {
            name: "Capital".into(),
            population: 1,
            food_storage: 0,
            production_progress: 0,
            production_queue: Some(ProductionItem::Unit(UnitType::Warrior)),
            position: Hex::ZERO,
            owner: 0,
            specialists: vec![],
            is_capital: true,
        },
        CityBuildings { buildings: vec![] },
        Happiness::new(1),
    ));

    world.spawn(Unit {
        unit_type: UnitType::Settler,
        movement_left: 1,
        owner: 0,
        position: Hex::ZERO,
        build_turns_left: None,
        current_build: None,
    });

    for _ in 0..max_turns {
        let victory = world.resource::<GameStateSim>().victory.clone();
        if victory.is_some() {
            break;
        }
        sim_process_turn(&mut world);
    }

    let state = world.resource::<GameStateSim>();
    let tech = world.resource::<TechState>();
    let cities: Vec<_> = world.query::<&City>().iter(&world).collect();
    let player_cities = cities.iter().filter(|c| c.owner == 0).count() as u32;
    let total_pop: u32 = cities.iter().filter(|c| c.owner == 0).map(|c| c.population).sum();
    let score = Score::calculate(
        &world.query::<(&City, &CityBuildings)>(),
        tech,
    );

    SimResult {
        turns: state.turn,
        victory: state.victory.clone().unwrap_or_else(|| "Timeout".into()),
        score: score.total,
        cities: player_cities,
        population: total_pop,
        techs: tech.researched.len() as u32,
        gold: state.gold,
    }
}

fn sim_process_turn(world: &mut World) {
    let mut state = world.resource_mut::<GameStateSim>();
    state.turn += 1;
    let gov = state.government;
    drop(state);

    let mut total_science = 0u32;
    let mut player_cities = 0u32;

    let mut cities = world.query::<(&mut City, &mut CityBuildings, &mut Happiness)>();
    for (mut city, mut buildings, mut happiness) in cities.iter_mut(world) {
        if city.owner == 0 {
            player_cities += 1;
        }

        let food = city.food_per_turn(&buildings, gov, 0);
        city.food_storage += food;
        if city.food_storage >= city.food_to_grow() {
            city.food_storage -= city.food_to_grow();
            city.population += 1;
        }

        let prod = city.production_per_turn(&buildings, gov, 0);
        city.production_progress += prod;

        if let Some(item) = &city.production_queue {
            let cost = match item {
                ProductionItem::Unit(u) => u.cost(),
                ProductionItem::Building(b) => b.cost(),
            };
            if city.production_progress >= cost {
                city.production_progress -= cost;
                match item {
                    ProductionItem::Unit(ut) => {
                        world.spawn(Unit {
                            unit_type: *ut,
                            movement_left: ut.stats().2,
                            owner: city.owner,
                            position: city.position,
                            build_turns_left: None,
                            current_build: None,
                        });
                    }
                    ProductionItem::Building(b) => {
                        buildings.buildings.push(b.clone());
                    }
                }
                city.production_queue = Some(ProductionItem::Unit(UnitType::Warrior));
            }
        }

        total_science += city.science_per_turn(&buildings, gov, 0);
        happiness.update(city.population, city.luxury_from_specialists());
    }

    let mut tech = world.resource_mut::<TechState>();
    tech.science_per_turn = total_science;
    if let Some(completed) = tech.process_turn() {
        if completed == Tech::Monarchy {
            world.resource_mut::<GameStateSim>().government = Government::Monarchy;
        }
        if completed == Tech::Democracy {
            world.resource_mut::<GameStateSim>().government = Government::Democracy;
        }
    }

    let mut state = world.resource_mut::<GameStateSim>();
    if player_cities >= 5 {
        state.victory = Some("Domination".into());
    } else if world.resource::<TechState>().researched.contains(&Tech::Democracy) {
        state.victory = Some("Democracy".into());
    }
}

fn print_statistics(results: &[SimResult]) {
    let n = results.len() as f64;
    let avg_turns = results.iter().map(|r| r.turns as f64).sum::<f64>() / n;
    let avg_score = results.iter().map(|r| r.score as f64).sum::<f64>() / n;
    let avg_cities = results.iter().map(|r| r.cities as f64).sum::<f64>() / n;
    let avg_pop = results.iter().map(|r| r.population as f64).sum::<f64>() / n;
    let avg_techs = results.iter().map(|r| r.techs as f64).sum::<f64>() / n;
    let avg_gold = results.iter().map(|r| r.gold as f64).sum::<f64>() / n;

    let mut victory_counts = std::collections::HashMap::new();
    for r in results {
        *victory_counts.entry(r.victory.clone()).or_insert(0) += 1;
    }

    println!("\n========== SIMULATION RESULTS ==========");
    println!("Games: {}", results.len());
    println!("Avg turns: {:.1}", avg_turns);
    println!("Avg score: {:.1}", avg_score);
    println!("Avg cities: {:.1}", avg_cities);
    println!("Avg population: {:.1}", avg_pop);
    println!("Avg techs: {:.1}", avg_techs);
    println!("Avg gold: {:.1}", avg_gold);
    println!("\nVictory distribution:");
    for (victory, count) in &victory_counts {
        let pct = (*count as f64 / n) * 100.0;
        println!("  {}: {} ({:.1}%)", victory, count, pct);
    }
    println!("=======================================");
}

#[derive(Resource)]
struct GameStateSim {
    turn: u32,
    gold: i32,
    government: Government,
    victory: Option<String>,
}

#[derive(Resource)]
struct MapDataSim {
    layout: HexLayout,
}
