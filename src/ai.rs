use bevy::prelude::*;
use hexx::Hex;
use crate::unit::{Unit, UnitType};
use crate::terrain::{HexTile, Improvement, Terrain};
use std::collections::HashMap;
use crate::tech::TechState;
use crate::pathfinding::find_path;
use rand::Rng;

#[derive(Resource)]
pub struct Civilizations {
    pub players: Vec<Civ>,
}

#[derive(Clone)]
pub struct Civ {
    pub id: u32,
    pub name: String,
    pub is_human: bool,
    pub gold: i32,
    pub tech: TechState,
    pub aggression: f32,
    pub leader: String,
    pub adjective: String,
    /// Frame in the leader portrait atlas.
    pub portrait: usize,
    /// Names for cities this civilization founds later.
    pub city_names: Vec<String>,
}

impl Default for Civilizations {
    fn default() -> Self {
        Self { players: Vec::new() }
    }
}

pub fn unit_strength(unit: &Unit) -> f32 {
    let (atk, def, _) = unit.unit_type.stats();
    (atk as f32) * unit.unit_type.attack_bonus(crate::terrain::Terrain::Plains)
        + (def as f32) * unit.unit_type.defense_bonus()
}

/// An attack an AI unit wants to make this turn; the caller resolves the combat.
pub struct AiAttack {
    pub attacker: Entity,
    pub target: Hex,
}

/// Moves every computer-controlled unit and returns the attacks they launch.
///
/// `hostile(a, b)` says whether owner `a` is willing to attack owner `b`.
/// `road_jobs` lists, per owner and in order of urgency, the tiles that owner wants a road on;
/// `owns(tile, owner)` says whether a tile lies inside that owner's borders.
pub fn process_ai_turn(
    civs: &mut Civilizations,
    units: &mut Query<(Entity, &mut Unit)>,
    city_positions: &[(Hex, u32)],
    tiles: &Query<&mut HexTile>,
    hostile: &dyn Fn(u32, u32) -> bool,
    road_jobs: &mut HashMap<u32, Vec<Hex>>,
    owns: &dyn Fn(Hex, u32) -> bool,
    commands: &mut Commands,
) -> Vec<AiAttack> {
    let mut rng = rand::thread_rng();
    let mut attacks = Vec::new();

    for civ in &mut civs.players {
        if civ.is_human { continue; }
        civ.tech.science_per_turn = 4;
        if let Some(completed) = civ.tech.process_turn() {
            info!("{} researched {}", civ.name, completed.name());
        }
        if civ.gold > 100 && rng.gen_bool(0.3) {
            civ.gold -= 40;
        }
    }

    let unit_snapshots: Vec<(Hex, u32, f32)> = units
        .iter()
        .map(|(_, u)| (u.position, u.owner, unit_strength(u)))
        .collect();
    // Who stands where at the start of the AI phase (strongest unit per tile).
    let mut occupied: HashMap<Hex, (u32, f32)> = HashMap::new();
    for (pos, owner, strength) in &unit_snapshots {
        let entry = occupied.entry(*pos).or_insert((*owner, *strength));
        if *strength > entry.1 {
            *entry = (*owner, *strength);
        }
    }
    let land: Vec<(Hex, Terrain)> = tiles.iter().map(|t| (t.coord, t.terrain)).collect();
    let land_set: std::collections::HashSet<Hex> = land.iter().filter(|(_, t)| *t != Terrain::Water).map(|(c, _)| *c).collect();
    let is_land = |hex: Hex| land_set.contains(&hex);
    let features: HashMap<Hex, (Terrain, bool, bool)> = tiles
        .iter()
        .map(|t| (t.coord, (t.terrain, t.improvements.contains(&Improvement::Farm), t.improvements.contains(&Improvement::Mine))))
        .collect();
    let mut founded: Vec<Hex> = Vec::new();

    for (entity, mut unit) in units.iter_mut() {
        if unit.owner == 0 || unit.movement_left == 0 || unit.build_turns_left.is_some() {
            continue;
        }
        let owner = unit.owner;
        let civ = civs.players.iter().find(|c| c.id == owner);
        let aggression = civ.map(|c| c.aggression).unwrap_or(0.8);
        let can_sail = civ.is_some_and(|c| c.tech.researched.contains(&crate::tech::Tech::Seafaring));

        // Settler: walk away from existing cities, then found a new one
        if unit.unit_type == UnitType::Settler {
            let here = unit.position;
            let crowded = |hex: Hex| {
                city_positions.iter().any(|(p, _)| p.distance_to(hex) < 3) || founded.iter().any(|p| p.distance_to(hex) < 3)
            };
            if is_land(here) && !crowded(here) {
                let civ = civs.players.iter().find(|c| c.id == owner);
                let count = city_positions.iter().filter(|(_, o)| *o == owner).count() + founded.len();
                let name = civ
                    .and_then(|c| c.city_names.get(count.saturating_sub(1)).cloned())
                    .unwrap_or_else(|| format!("Outpost {}", city_positions.len() + founded.len() + 1));
                crate::city::spawn_city(commands, here, name, 1, owner, false);
                founded.push(here);
                commands.entity(entity).try_despawn_recursive();
            } else {
                let options: Vec<Hex> = here.all_neighbors().into_iter().filter(|n| is_land(*n) && !occupied.get(n).is_some_and(|(o, _)| *o != owner)).collect();
                // Prefer steps that lead away from the nearest city.
                let away = |hex: Hex| city_positions.iter().map(|(p, _)| p.distance_to(hex)).min().unwrap_or(0);
                if let Some(step) = options.iter().copied().max_by_key(|n| (away(*n), rng.gen_range(0..4))) {
                    unit.position = step;
                    unit.movement_left = 0;
                }
            }
            continue;
        }

        // Worker: build the roads the civilization needs, then improve its land
        if unit.unit_type.is_worker() {
            let here = unit.position;
            let jobs = road_jobs.entry(owner).or_default();
            if let Some(index) = jobs.iter().position(|h| *h == here) {
                jobs.remove(index);
                unit.start_build(Improvement::Road);
                continue;
            }
            // Head for the nearest tile that needs a road; other workers take the next ones.
            let nearest = jobs.iter().copied().enumerate().min_by_key(|(index, h)| (here.distance_to(*h), *index));
            if let Some((index, target)) = nearest {
                jobs.remove(index);
                let step = find_path(here, target, land.iter().copied(), false).and_then(|path| path.get(1).copied());
                if let Some(next) = step.filter(|n| !occupied.get(n).is_some_and(|(other, _)| *other != owner)) {
                    unit.position = next;
                    unit.movement_left = 0;
                    continue;
                }
            }
            // No road work: farm the flat land and mine the hills inside the borders.
            let on_city = city_positions.iter().any(|(p, _)| *p == here);
            if let Some((terrain, farm, mine)) = features.get(&here).copied().filter(|_| owns(here, owner) && !on_city) {
                let job = match terrain {
                    Terrain::Hills | Terrain::Mountain if !mine => Some(Improvement::Mine),
                    Terrain::Grassland | Terrain::Plains if !farm => Some(Improvement::Farm),
                    _ => None,
                };
                if let Some(job) = job {
                    unit.start_build(job);
                    continue;
                }
            }
            let options: Vec<Hex> = here
                .all_neighbors()
                .into_iter()
                .filter(|n| is_land(*n) && owns(*n, owner) && !occupied.get(n).is_some_and(|(other, _)| *other != owner))
                .collect();
            if !options.is_empty() {
                unit.position = options[rng.gen_range(0..options.len())];
                unit.movement_left = 0;
            }
            continue;
        }
        if !unit.unit_type.is_military() {
            continue;
        }

        // Pick a target: a beatable enemy unit close by, otherwise the nearest enemy city.
        let my_strength = unit_strength(&unit);
        let threshold = if aggression > 0.6 { 0.9 } else { 1.2 };
        let here = unit.position;
        let enemy_unit = unit_snapshots
            .iter()
            .filter(|(pos, other, strength)| {
                hostile(owner, *other) && here.distance_to(*pos) <= 4 && my_strength > *strength * threshold
            })
            .min_by_key(|(pos, _, _)| here.distance_to(*pos))
            .map(|(pos, _, _)| *pos);
        let city_range = if aggression > 0.6 { 14 } else { 7 };
        let enemy_city = city_positions
            .iter()
            .filter(|(pos, other)| hostile(owner, *other) && here.distance_to(*pos) <= city_range)
            .min_by_key(|(pos, _)| here.distance_to(*pos))
            .map(|(pos, _)| *pos);

        if let Some(target) = enemy_unit.or(enemy_city) {
            if let Some(path) = find_path(here, target, land.iter().copied(), can_sail) {
                for next in path.into_iter().skip(1) {
                    if unit.movement_left == 0 {
                        break;
                    }
                    let boarding = is_land(unit.position) != is_land(next);
                    match occupied.get(&next) {
                        Some((other, strength)) if *other != owner => {
                            // Attack if the odds look acceptable; otherwise hold position.
                            if hostile(owner, *other) && (aggression > 0.6 || my_strength >= *strength) {
                                attacks.push(AiAttack { attacker: entity, target: next });
                                unit.movement_left = 0;
                            }
                            break;
                        }
                        _ => {
                            unit.position = next;
                            // Boarding or leaving a boat takes the rest of the turn.
                            unit.movement_left = if boarding { 0 } else { unit.movement_left - 1 };
                        }
                    }
                }
            }
            continue;
        }

        // Nothing to fight: garrison a city, or wander a little.
        let in_own_city = city_positions.iter().any(|(p, o)| *p == here && *o == owner);
        if in_own_city || rng.gen_bool(0.5) {
            continue;
        }
        let neighbors = here.all_neighbors();
        let step = neighbors[rng.gen_range(0..neighbors.len())];
        if is_land(step) && is_land(here) && !occupied.get(&step).is_some_and(|(other, _)| *other != owner) {
            unit.position = step;
            unit.movement_left -= 1;
        }
    }
    attacks
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[derive(Resource)]
    struct Jobs(HashMap<u32, Vec<Hex>>);

    fn ai_phase(
        mut civs: ResMut<Civilizations>,
        mut units: Query<(Entity, &mut Unit)>,
        tiles: Query<&mut HexTile>,
        mut jobs: ResMut<Jobs>,
        mut commands: Commands,
    ) {
        let cities = [(Hex::ZERO, 1)];
        process_ai_turn(&mut civs, &mut units, &cities, &tiles, &|_, _| false, &mut jobs.0, &|hex, _| hex.distance_to(Hex::ZERO) <= 2, &mut commands);
    }

    fn rival() -> Civ {
        Civ {
            id: 1,
            name: "Rival".into(),
            is_human: false,
            gold: 0,
            tech: TechState::default(),
            aggression: 0.5,
            leader: String::new(),
            adjective: String::new(),
            portrait: 0,
            city_names: vec![],
        }
    }

    fn world_with_worker(terrain: Terrain) -> (World, Entity) {
        let mut world = World::new();
        for q in -3..=3 {
            world.spawn(HexTile { coord: Hex::new(q, 0), terrain, improvements: vec![], resource: None });
        }
        world.insert_resource(Civilizations { players: vec![rival()] });
        let worker = world
            .spawn(Unit { unit_type: UnitType::Worker, movement_left: 1, owner: 1, position: Hex::ZERO, build_turns_left: None, current_build: None, fortified: false })
            .id();
        (world, worker)
    }

    #[test]
    fn worker_walks_to_a_road_job_and_builds_it() {
        let (mut world, worker) = world_with_worker(Terrain::Plains);
        let jobs = || Jobs(HashMap::from([(1, vec![Hex::new(2, 0), Hex::new(1, 0)])]));

        // The nearer job is taken first, even though it is listed second.
        world.insert_resource(jobs());
        world.run_system_once(ai_phase).unwrap();
        assert_eq!(world.get::<Unit>(worker).unwrap().position, Hex::new(1, 0));
        assert!(world.get::<Unit>(worker).unwrap().current_build.is_none());

        world.get_mut::<Unit>(worker).unwrap().movement_left = 1;
        world.insert_resource(jobs());
        world.run_system_once(ai_phase).unwrap();
        let unit = world.get::<Unit>(worker).unwrap();
        assert_eq!(unit.position, Hex::new(1, 0));
        assert_eq!(unit.current_build, Some(Improvement::Road));
    }

    #[test]
    fn idle_worker_improves_land_inside_the_borders() {
        let (mut world, worker) = world_with_worker(Terrain::Hills);
        world.get_mut::<Unit>(worker).unwrap().position = Hex::new(1, 0);
        world.insert_resource(Jobs(HashMap::new()));
        world.run_system_once(ai_phase).unwrap();
        assert_eq!(world.get::<Unit>(worker).unwrap().current_build, Some(Improvement::Mine));
    }
}
