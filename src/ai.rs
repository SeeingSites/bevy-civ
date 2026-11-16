use bevy::prelude::*;
use hexx::Hex;
use crate::unit::{Unit, UnitType};
use crate::terrain::{HexTile, Improvement, Terrain};
use std::collections::HashMap;
use crate::tech::{Tech, TechState};
use crate::pathfinding::{find_path, find_route, land_step, road_network, zoc_blocks, StepCost, Travel};
use crate::unit::{Domain, AIR_RANGE};
use rand::Rng;

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct Civilizations {
    pub players: Vec<Civ>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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
    pub government: crate::government::Government,
    /// Turns of anarchy left in a revolution, and the government it leads to.
    #[serde(default)]
    pub anarchy: u32,
    #[serde(default)]
    pub pending_government: Option<crate::government::Government>,
}

impl Default for Civilizations {
    fn default() -> Self {
        Self { players: Vec::new() }
    }
}

/// How much a technology appeals to a warlike (first) and to a builder (second) civilization.
fn appeal(tech: Tech) -> (f32, f32) {
    match tech {
        Tech::BronzeWorking => (3.0, 1.0),
        Tech::AnimalHusbandry => (3.0, 1.5),
        Tech::IronWorking => (4.0, 1.0),
        Tech::HorsebackRiding => (3.5, 0.5),
        Tech::TheWheel => (2.5, 1.0),
        Tech::Mathematics => (3.0, 1.5),
        Tech::Gunpowder => (5.0, 1.0),
        Tech::ReplaceableParts => (4.0, 2.0),
        Tech::MotorizedTransport => (5.0, 1.0),
        Tech::Masonry => (2.0, 2.0),
        Tech::Mining => (1.5, 1.5),
        Tech::Pottery => (0.5, 3.0),
        Tech::Alphabet => (0.5, 2.5),
        Tech::Writing => (0.5, 3.5),
        Tech::Seafaring => (1.5, 3.0),
        Tech::CodeOfLaws => (0.5, 2.5),
        Tech::Currency => (0.5, 3.5),
        Tech::Mysticism => (0.5, 2.0),
        Tech::Philosophy => (0.5, 3.0),
        Tech::Monarchy => (1.5, 2.5),
        Tech::Construction => (1.0, 2.5),
        Tech::Engineering => (2.0, 2.5),
        Tech::PrintingPress => (0.5, 3.0),
        Tech::Democracy => (0.0, 3.5),
        Tech::SteamPower => (2.5, 3.0),
        Tech::Industrialization => (2.0, 4.0),
        Tech::Refining => (3.0, 2.5),
        Tech::TheRepublic => (0.5, 3.0),
        Tech::Theology => (0.5, 2.5),
        Tech::Fundamentalism => (3.0, 0.5),
        Tech::Communism => (2.5, 1.5),
        Tech::Sanitation => (0.5, 3.0),
        Tech::Electricity => (1.5, 3.0),
        Tech::Computers => (2.0, 4.0),
        Tech::Rocketry => (3.5, 3.0),
        Tech::SpaceFlight => (1.0, 5.0),
        Tech::Plastics => (1.0, 5.0),
        Tech::Superconductors => (1.0, 5.0),
    }
}

/// The technology a computer player studies next: warlike leaders favour arms, builders the economy,
/// and cheap discoveries are preferred over distant ones.
pub fn pick_research(tech: &TechState, aggression: f32) -> Option<Tech> {
    let score = |t: Tech| {
        let (war, peace) = appeal(t);
        war * aggression + peace * (1.0 - aggression) - t.cost() as f32 / 40.0
    };
    Tech::ALL.into_iter().filter(|t| tech.can_research(*t)).max_by(|a, b| score(*a).total_cmp(&score(*b)))
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
        // Science comes from the civilization's cities (set by the caller before this phase).
        if civ.tech.process_turn().is_some() || civ.tech.current.is_none() {
            civ.tech.current = pick_research(&civ.tech, civ.aggression);
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
    let roads = road_network(tiles.iter(), city_positions.iter().map(|(p, _)| *p));
    let terrain_at: HashMap<Hex, Terrain> = land.iter().copied().collect();
    let polluted: std::collections::HashSet<Hex> = tiles.iter().filter(|t| t.is_polluted()).map(|t| t.coord).collect();
    let resources: std::collections::HashSet<Hex> = tiles.iter().filter(|t| t.resource.is_some()).map(|t| t.coord).collect();
    // How good a place is for a city: the land around it (grass and plains best), the sea for a
    // harbour, and any resources within reach.
    let site_value = |site: Hex| -> i32 {
        hexx::shapes::hexagon(site, 2)
            .map(|h| {
                let land = match terrain_at.get(&h) {
                    Some(Terrain::Grassland | Terrain::Plains) => 3,
                    Some(Terrain::Hills) => 2,
                    Some(Terrain::Forest) => 1,
                    Some(Terrain::Water) => 1,
                    _ => 0,
                };
                land + if resources.contains(&h) { 3 } else { 0 }
            })
            .sum()
    };
    let features: HashMap<Hex, (Terrain, bool, bool)> = tiles
        .iter()
        .map(|t| (t.coord, (t.terrain, t.improvements.contains(&Improvement::Farm), t.improvements.contains(&Improvement::Mine))))
        .collect();
    let mut founded: Vec<Hex> = Vec::new();
    // Roads that could carry a railroad (cities need none), and which of them a worker has already taken.
    let city_tiles: std::collections::HashSet<Hex> = city_positions.iter().map(|(p, _)| *p).collect();
    let unrailed: Vec<Hex> = tiles.iter().filter(|t| t.has_road() && !t.has_railroad() && !city_tiles.contains(&t.coord)).map(|t| t.coord).collect();
    let mut rail_claimed: std::collections::HashSet<Hex> = units
        .iter()
        .filter(|(_, u)| u.current_build == Some(Improvement::Railroad))
        .map(|(_, u)| u.position)
        .collect();

    // The best defender standing in each city is its garrison and stays there.
    let mut holders: HashMap<Hex, (Entity, f32)> = HashMap::new();
    let mut field_armies: HashMap<u32, usize> = HashMap::new();
    for (entity, unit) in units.iter() {
        if unit.owner == 0 || !unit.unit_type.is_army() {
            continue;
        }
        *field_armies.entry(unit.owner).or_default() += 1;
        if city_positions.iter().any(|(p, o)| *p == unit.position && *o == unit.owner) {
            let defence = unit.unit_type.stats().1 as f32 * unit.unit_type.defense_bonus();
            if holders.get(&unit.position).is_none_or(|(_, best)| defence > *best) {
                holders.insert(unit.position, (entity, defence));
            }
        }
    }
    let garrison: std::collections::HashSet<Entity> = holders.values().map(|(e, _)| *e).collect();
    for (pos, owner) in city_positions {
        if holders.contains_key(pos) {
            if let Some(count) = field_armies.get_mut(owner) {
                *count = count.saturating_sub(1);
            }
        }
    }
    // Cities of computer players that nobody is holding.
    let mut undefended: Vec<(Hex, u32)> = city_positions.iter().copied().filter(|(p, o)| *o != 0 && !holders.contains_key(p)).collect();

    for (entity, mut unit) in units.iter_mut() {
        if unit.owner == 0 || unit.movement_left == 0 || unit.build_turns_left.is_some() {
            continue;
        }
        let owner = unit.owner;
        let civ = civs.players.iter().find(|c| c.id == owner);
        let aggression = civ.map(|c| c.aggression).unwrap_or(0.8);
        let can_sail = civ.is_some_and(|c| c.tech.researched.contains(&Tech::Seafaring));

        // Caravans and diplomats are sent on their errands by the caller (see `main.rs`).
        if matches!(unit.unit_type, UnitType::Caravan | UnitType::Diplomat) {
            continue;
        }

        // Great general: march with the nearest army of its own (it never attacks).
        if unit.unit_type.is_great_person() {
            let here = unit.position;
            let army = unit_snapshots
                .iter()
                .filter(|(pos, other, strength)| *other == owner && *strength > 0.0 && is_land(*pos))
                .map(|(pos, _, _)| *pos)
                .min_by_key(|pos| here.distance_to(*pos));
            if let Some(path) = army.filter(|a| *a != here).and_then(|a| find_route(here, a, land.iter().copied(), &roads, Travel::Land)) {
                for next in path.into_iter().skip(1) {
                    if !unit.can_move() || occupied.get(&next).is_some_and(|(other, _)| *other != owner) {
                        break;
                    }
                    let cost = land_step(unit.position, next, terrain_at.get(&next).copied(), &roads, true);
                    unit.step(next, cost);
                }
            }
            unit.end_move();
            continue;
        }

        // Settler: walk away from existing cities, then found a new one
        if unit.unit_type == UnitType::Settler {
            let here = unit.position;
            let crowded = |hex: Hex| {
                city_positions.iter().any(|(p, _)| p.distance_to(hex) < 3) || founded.iter().any(|p| p.distance_to(hex) < 3)
            };
            // The best free site within reach (by land, or by sea once Seafaring is known).
            let reach = if can_sail { 14 } else { 10 };
            let target = land_set
                .iter()
                .copied()
                .filter(|h| h.distance_to(here) <= reach && !crowded(*h) && !occupied.get(h).is_some_and(|(o, _)| *o != owner))
                .max_by_key(|h| (site_value(*h) * 4 - h.distance_to(here) * 3, h.x, h.y));
            let settle_here = is_land(here) && !crowded(here) && target.is_none_or(|t| t == here || site_value(here) * 4 >= site_value(t) * 4 - 6);
            if !settle_here {
                if let Some(target) = target {
                    let travel = if can_sail { Travel::Amphibious } else { Travel::Land };
                    if let Some(path) = find_route(here, target, land.iter().copied(), &roads, travel) {
                        for next in path.into_iter().skip(1) {
                            if !unit.can_move() || occupied.get(&next).is_some_and(|(o, _)| *o != owner) {
                                break;
                            }
                            let boarding = is_land(unit.position) != is_land(next);
                            let cost = if boarding { StepCost::EndsTurn } else { land_step(unit.position, next, terrain_at.get(&next).copied(), &roads, true) };
                            unit.step(next, cost);
                        }
                        unit.end_move();
                        continue;
                    }
                }
            }
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
                let step = find_path(here, target, land.iter().copied(), Travel::Land).and_then(|path| path.get(1).copied());
                if let Some(next) = step.filter(|n| !occupied.get(n).is_some_and(|(other, _)| *other != owner)) {
                    unit.position = next;
                    unit.movement_left = 0;
                    continue;
                }
            }
            // Once Steam Power is known, lay railroads over the roads inside the borders, nearest first.
            let steam = civs.players.iter().any(|c| c.id == owner && c.tech.researched.contains(&Tech::SteamPower));
            if steam {
                if unrailed.contains(&here) && owns(here, owner) && rail_claimed.insert(here) {
                    unit.start_build(Improvement::Railroad);
                    continue;
                }
                let target = unrailed
                    .iter()
                    .copied()
                    .filter(|h| owns(*h, owner) && !rail_claimed.contains(h) && here.distance_to(*h) <= 10)
                    .min_by_key(|h| (here.distance_to(*h), h.x, h.y));
                if let Some(target) = target {
                    rail_claimed.insert(target);
                    let step = find_route(here, target, land.iter().copied(), &roads, Travel::Land).and_then(|path| path.get(1).copied());
                    if let Some(next) = step.filter(|n| !occupied.get(n).is_some_and(|(other, _)| *other != owner)) {
                        unit.position = next;
                        unit.end_move();
                        continue;
                    }
                }
            }
            // Pollution is cleaned up where the worker stands.
            if polluted.contains(&here) && owns(here, owner) {
                unit.start_build(Improvement::Pollution);
                continue;
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

        let here = unit.position;
        // Ships and aircraft hunt enemies that come within reach and otherwise stay at their base.
        if unit.unit_type.domain() != Domain::Land {
            let flying = unit.unit_type.domain() == Domain::Air;
            let ports: std::collections::HashSet<Hex> = city_positions.iter().filter(|(_, o)| *o == owner).map(|(p, _)| *p).collect();
            let in_reach = |hex: Hex| {
                if flying {
                    ports.iter().any(|p| p.distance_to(hex) <= AIR_RANGE)
                } else {
                    // A ship can strike anything at sea or on the shore.
                    !is_land(hex) || hex.all_neighbors().iter().any(|n| !is_land(*n))
                }
            };
            let prey = unit_snapshots
                .iter()
                .filter(|(pos, other, _)| hostile(owner, *other) && here.distance_to(*pos) <= 7 && in_reach(*pos))
                .min_by_key(|(pos, _, _)| here.distance_to(*pos))
                .map(|(pos, _, _)| *pos);
            let base = ports.iter().copied().min_by_key(|p| here.distance_to(*p));
            let travel = if flying { Travel::Air } else { Travel::Sea(&ports) };
            let Some(target) = prey.or(base.filter(|b| flying && *b != here)) else { continue };
            if let Some(path) = find_path(here, target, land.iter().copied(), travel) {
                for next in path.into_iter().skip(1) {
                    if !unit.can_move() {
                        break;
                    }
                    if occupied.get(&next).is_some_and(|(other, _)| *other != owner) {
                        if Some(next) == prey {
                            attacks.push(AiAttack { attacker: entity, target: next });
                            unit.end_move();
                        }
                        break;
                    }
                    if !flying && is_land(next) && !ports.contains(&next) {
                        break;
                    }
                    unit.step(next, StepCost::Full);
                }
            }
            continue;
        }
        if garrison.contains(&entity) {
            unit.fortified = true;
            unit.movement_left = 0;
            continue;
        }
        unit.fortified = false;

        // Goal, in order: hold an undefended city of ours, strike a beatable enemy nearby,
        // march on the nearest enemy city.
        let my_strength = unit_strength(&unit);
        let threshold = if aggression > 0.6 { 0.9 } else { 1.2 };
        let relief = undefended
            .iter()
            .enumerate()
            .filter(|(_, (p, o))| *o == owner && here.distance_to(*p) <= 8)
            .min_by_key(|(_, (p, _))| here.distance_to(*p))
            .map(|(index, (p, _))| (index, *p));
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

        let goal = if let Some((index, city)) = relief {
            undefended.remove(index);
            Some(city)
        } else if enemy_unit.is_some() {
            enemy_unit
        } else if let Some(city) = enemy_city {
            // A defended city is attacked in strength: wait nearby until enough of the army has gathered.
            let defended = occupied.get(&city).is_some_and(|(other, _)| *other != owner);
            let needed = if !defended { 1 } else if aggression > 0.6 { 2 } else { 3 };
            let gathered = unit_snapshots.iter().filter(|(pos, other, strength)| *other == owner && *strength > 0.0 && here.distance_to(*pos) <= 3).count();
            let available = field_armies.get(&owner).copied().unwrap_or(0);
            if available < needed {
                None
            } else if gathered < needed && here.distance_to(city) <= 3 {
                continue;
            } else {
                Some(city)
            }
        } else {
            None
        };

        if let Some(target) = goal {
            let travel = if can_sail { Travel::Amphibious } else { Travel::Land };
            let enemy_armies: std::collections::HashSet<Hex> =
                unit_snapshots.iter().filter(|(_, other, strength)| hostile(owner, *other) && *strength > 0.0).map(|(pos, _, _)| *pos).collect();
            if let Some(path) = find_route(here, target, land.iter().copied(), &roads, travel) {
                for next in path.into_iter().skip(1) {
                    if !unit.can_move() || zoc_blocks(unit.position, next, &enemy_armies) {
                        break;
                    }
                    let boarding = is_land(unit.position) != is_land(next);
                    match occupied.get(&next) {
                        Some((other, strength)) if *other != owner => {
                            // Attack if the odds look acceptable; otherwise hold position.
                            if hostile(owner, *other) && (aggression > 0.6 || my_strength >= *strength) {
                                attacks.push(AiAttack { attacker: entity, target: next });
                                unit.end_move();
                            }
                            break;
                        }
                        _ => {
                            // Roads speed the march; boarding or leaving a boat takes the rest of the turn.
                            let cost = if boarding { StepCost::EndsTurn } else { land_step(unit.position, next, terrain_at.get(&next).copied(), &roads, true) };
                            unit.step(next, cost);
                        }
                    }
                }
            }
            continue;
        }

        // Nothing to do: stay near home, drifting back towards the nearest city of ours.
        let home = city_positions.iter().filter(|(_, o)| *o == owner).min_by_key(|(p, _)| here.distance_to(*p)).map(|(p, _)| *p);
        let options: Vec<Hex> = here
            .all_neighbors()
            .into_iter()
            .filter(|n| is_land(*n) && is_land(here) && !occupied.get(n).is_some_and(|(other, _)| *other != owner))
            .collect();
        if options.is_empty() || rng.gen_bool(0.5) {
            continue;
        }
        let step = match home {
            Some(home) if here.distance_to(home) > 3 => options.iter().copied().min_by_key(|n| n.distance_to(home)).unwrap(),
            _ => options[rng.gen_range(0..options.len())],
        };
        unit.position = step;
        unit.movement_left -= 1;
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
            government: crate::government::Government::Despotism,
            anarchy: 0,
            pending_government: None,
        }
    }

    fn world_with_worker(terrain: Terrain) -> (World, Entity) {
        let mut world = World::new();
        for q in -3..=3 {
            world.spawn(HexTile { coord: Hex::new(q, 0), terrain, improvements: vec![], resource: None });
        }
        world.insert_resource(Civilizations { players: vec![rival()] });
        let worker = world
            .spawn(Unit { unit_type: UnitType::Worker, movement_left: 1, owner: 1, position: Hex::ZERO, build_turns_left: None, current_build: None, fortified: false, road_credit: 0, hp: 3, level: 0 })
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
    fn workers_lay_railroads_over_roads_once_steam_power_is_known() {
        let (mut world, worker) = world_with_worker(Terrain::Plains);
        // A road two tiles east, inside the borders; the worker stands off the road.
        for mut tile in world.query::<&mut HexTile>().iter_mut(&mut world) {
            if tile.coord == Hex::new(2, 0) {
                tile.add_improvement(Improvement::Road);
            }
        }
        world.get_mut::<Unit>(worker).unwrap().position = Hex::new(-1, 0);
        world.insert_resource(Jobs(HashMap::new()));

        // Without Steam Power it farms instead.
        world.run_system_once(ai_phase).unwrap();
        assert_eq!(world.get::<Unit>(worker).unwrap().current_build, Some(Improvement::Farm));

        let reset = |world: &mut World| {
            let mut unit = world.get_mut::<Unit>(worker).unwrap();
            unit.current_build = None;
            unit.build_turns_left = None;
            unit.movement_left = 1;
        };
        reset(&mut world);
        world.resource_mut::<Civilizations>().players[0].tech.researched.insert(Tech::SteamPower);
        // Three turns to walk there, one to start work.
        for _ in 0..4 {
            world.run_system_once(ai_phase).unwrap();
            if world.get::<Unit>(worker).unwrap().current_build.is_some() {
                break;
            }
            world.get_mut::<Unit>(worker).unwrap().movement_left = 1;
        }
        let unit = world.get::<Unit>(worker).unwrap();
        assert_eq!(unit.position, Hex::new(2, 0));
        assert_eq!(unit.current_build, Some(Improvement::Railroad));
    }

    #[test]
    fn a_great_general_joins_the_nearest_army() {
        let mut world = World::new();
        for q in -4..=4 {
            world.spawn(HexTile { coord: Hex::new(q, 0), terrain: Terrain::Plains, improvements: vec![], resource: None });
        }
        world.insert_resource(Civilizations { players: vec![rival()] });
        world.insert_resource(Jobs(HashMap::new()));
        let general = soldier(&mut world, UnitType::GreatGeneral, 1, -2);
        world.get_mut::<Unit>(general).unwrap().movement_left = 2;
        let legion = soldier(&mut world, UnitType::Legion, 1, 0);
        world.get_mut::<Unit>(legion).unwrap().fortified = true;
        world.run_system_once(ai_phase).unwrap();
        assert_eq!(world.get::<Unit>(general).unwrap().position, world.get::<Unit>(legion).unwrap().position);
    }

    fn soldier(world: &mut World, kind: UnitType, owner: u32, q: i32) -> Entity {
        world
            .spawn(Unit { unit_type: kind, movement_left: 1, owner, position: Hex::new(q, 0), build_turns_left: None, current_build: None, fortified: false, road_credit: 0, hp: 3, level: 0 })
            .id()
    }

    fn war_phase(
        mut civs: ResMut<Civilizations>,
        mut units: Query<(Entity, &mut Unit)>,
        tiles: Query<&mut HexTile>,
        mut commands: Commands,
    ) {
        // Rival city at the origin, the player's city three tiles east; they are at war.
        let cities = [(Hex::ZERO, 1), (Hex::new(3, 0), 0)];
        process_ai_turn(&mut civs, &mut units, &cities, &tiles, &|a, b| a != b, &mut HashMap::new(), &|_, _| false, &mut commands);
    }

    #[test]
    fn best_defender_garrisons_and_the_rest_take_the_field() {
        let (mut world, worker) = world_with_worker(Terrain::Plains);
        world.despawn(worker);
        let phalanx = soldier(&mut world, UnitType::Phalanx, 1, 0);
        let warrior = soldier(&mut world, UnitType::Warrior, 1, 0);
        world.run_system_once(war_phase).unwrap();
        let held = world.get::<Unit>(phalanx).unwrap();
        assert_eq!(held.position, Hex::ZERO);
        assert!(held.fortified, "the stronger defender holds the city");
        // The enemy city is undefended, so a single unit is enough to march on it.
        assert_eq!(world.get::<Unit>(warrior).unwrap().position, Hex::new(1, 0));
    }

    #[test]
    fn a_lone_unit_waits_for_support_before_a_defended_city() {
        let (mut world, worker) = world_with_worker(Terrain::Plains);
        world.despawn(worker);
        soldier(&mut world, UnitType::Phalanx, 1, 0); // garrison
        let scout = soldier(&mut world, UnitType::Warrior, 1, 1);
        let support = soldier(&mut world, UnitType::Warrior, 1, -3);
        soldier(&mut world, UnitType::Warrior, 1, -3);
        soldier(&mut world, UnitType::Warrior, 0, 3); // the defender
        world.run_system_once(war_phase).unwrap();
        assert_eq!(world.get::<Unit>(scout).unwrap().position, Hex::new(1, 0), "too few have gathered to attack");
        assert_eq!(world.get::<Unit>(support).unwrap().position, Hex::new(-2, 0), "the rest of the army closes up");
    }

    #[test]
    fn an_empty_city_is_reoccupied() {
        let (mut world, worker) = world_with_worker(Terrain::Plains);
        world.despawn(worker);
        let warrior = soldier(&mut world, UnitType::Warrior, 1, -2);
        world.run_system_once(war_phase).unwrap();
        assert_eq!(world.get::<Unit>(warrior).unwrap().position, Hex::new(-1, 0));
    }

    #[test]
    fn a_warship_attacks_troops_at_sea_but_stays_off_the_land() {
        let mut world = World::new();
        for q in -1..=6 {
            let terrain = if (1..=4).contains(&q) { Terrain::Water } else { Terrain::Plains };
            world.spawn(HexTile { coord: Hex::new(q, 0), terrain, improvements: vec![], resource: None });
        }
        world.insert_resource(Civilizations { players: vec![rival()] });
        // The rival's port is at the origin; an enemy transport is two tiles out to sea.
        let galley = soldier(&mut world, UnitType::Galley, 1, 0);
        world.get_mut::<Unit>(galley).unwrap().movement_left = 3;
        soldier(&mut world, UnitType::Warrior, 0, 2);
        world.insert_resource(Jobs(HashMap::new()));
        let attacks = world.run_system_once(naval_phase).unwrap();
        assert_eq!(world.get::<Unit>(galley).unwrap().position, Hex::new(1, 0), "sails next to its prey");
        assert_eq!(attacks, vec![Hex::new(2, 0)]);

        // With nothing to hunt it stays in port rather than wandering inland.
        let mut world = World::new();
        for q in -1..=3 {
            world.spawn(HexTile { coord: Hex::new(q, 0), terrain: if q >= 1 { Terrain::Water } else { Terrain::Plains }, improvements: vec![], resource: None });
        }
        world.insert_resource(Civilizations { players: vec![rival()] });
        let galley = soldier(&mut world, UnitType::Galley, 1, 0);
        world.run_system_once(naval_phase).unwrap();
        assert_eq!(world.get::<Unit>(galley).unwrap().position, Hex::ZERO);
    }

    fn naval_phase(mut civs: ResMut<Civilizations>, mut units: Query<(Entity, &mut Unit)>, tiles: Query<&mut HexTile>, mut commands: Commands) -> Vec<Hex> {
        let cities = [(Hex::ZERO, 1)];
        process_ai_turn(&mut civs, &mut units, &cities, &tiles, &|a, b| a != b, &mut HashMap::new(), &|_, _| false, &mut commands)
            .into_iter()
            .map(|a| a.target)
            .collect()
    }

    #[test]
    fn research_follows_temperament() {
        let tech = TechState::default();
        assert_eq!(pick_research(&tech, 0.9), Some(Tech::AnimalHusbandry));
        assert_eq!(pick_research(&tech, 0.1), Some(Tech::Pottery));
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
