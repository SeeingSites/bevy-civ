//! Standalone pure-Rust game simulator.
//!
//! Run from the simulate/ directory:
//!   cargo run -- --games 50 --max-turns 100

use rand::Rng;
use std::env;

#[derive(Clone, Copy)]
enum Government {
    Despotism,
    Monarchy,
    Democracy,
}

impl Government {
    fn science_modifier(self) -> f32 {
        match self {
            Government::Despotism => 0.8,
            Government::Monarchy => 1.0,
            Government::Democracy => 1.5,
        }
    }
}

#[derive(Clone)]
struct City {
    population: u32,
    food: u32,
    production: u32,
}

#[derive(Clone)]
struct Game {
    turn: u32,
    gold: i32,
    government: Government,
    cities: Vec<City>,
    techs: u32,
    science_progress: u32,
    victory: Option<String>,
}

impl Game {
    fn new(rng: &mut impl Rng) -> Self {
        Self {
            turn: 1,
            gold: 0,
            government: Government::Despotism,
            cities: vec![City {
                population: 1 + rng.gen_range(0..2),
                food: 0,
                production: 0,
            }],
            techs: 0,
            science_progress: 0,
            victory: None,
        }
    }

    fn process_turn(&mut self, rng: &mut impl Rng) {
        if self.victory.is_some() {
            return;
        }
        self.turn += 1;

        let mut total_science = 0u32;
        let mut new_cities = 0u32;

        for city in &mut self.cities {
            let food = 3 + city.population;
            city.food += food;
            let grow_cost = 10 + city.population * 5;
            if city.food >= grow_cost {
                city.food -= grow_cost;
                city.population += 1;
            }

            let prod = 2 + city.population;
            city.production += prod;
            if city.production >= 30 && rng.gen_bool(0.35) {
                city.production -= 30;
                if self.cities.len() + new_cities as usize < 10 && rng.gen_bool(0.5) {
                    new_cities += 1;
                }
            }

            total_science += 1 + city.population / 2;
        }

        for _ in 0..new_cities {
            self.cities.push(City {
                population: 1,
                food: 0,
                production: 0,
            });
        }

        self.science_progress += ((total_science as f32) * self.government.science_modifier()) as u32;
        if self.science_progress >= 35 {
            self.science_progress = 0;
            self.techs += 1;
            if self.techs == 4 {
                self.government = Government::Monarchy;
            }
            if self.techs == 9 {
                self.government = Government::Democracy;
            }
        }

        self.gold += (total_science / 2) as i32;

        let player_cities = self.cities.len() as u32;
        if player_cities >= 6 {
            self.victory = Some("Domination".into());
        } else if self.techs >= 12 {
            self.victory = Some("Science".into());
        } else if self.turn > 90 {
            self.victory = Some("Timeout".into());
        }
    }
}

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
        .unwrap_or(100);

    println!("Running {} games (max {} turns each)...", num_games, max_turns);

    let mut results = Vec::new();
    let mut rng = rand::thread_rng();

    for game_id in 0..num_games {
        if game_id % 10 == 0 {
            println!("Game {}/{}", game_id + 1, num_games);
        }
        let mut game = Game::new(&mut rng);
        for _ in 0..max_turns {
            if game.victory.is_some() {
                break;
            }
            game.process_turn(&mut rng);
        }

        let pop: u32 = game.cities.iter().map(|c| c.population).sum();
        let cities = game.cities.len() as u32;
        let score = pop * 2 + game.techs * 10 + cities * 5;

        results.push(SimResult {
            turns: game.turn,
            victory: game.victory.unwrap_or_else(|| "Timeout".into()),
            score,
            cities,
            population: pop,
            techs: game.techs,
            gold: game.gold,
        });
    }

    print_statistics(&results);
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
