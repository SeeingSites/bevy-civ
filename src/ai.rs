use bevy::prelude::*;
use hexx::Hex;
use crate::unit::{Unit, UnitType};
use crate::terrain::{HexTile, Improvement, Terrain};
use std::collections::HashMap;
use crate::tech::{Tech, TechState};
use crate::pathfinding::{find_path, find_route, land_step, road_network, zoc_blocks, StepCost, Travel};
use crate::unit::{Domain, AIR_RANGE};
use rand::Rng;
use crate::cargo::Berth;
use std::collections::HashSet;

/// What the computer players' ferries are doing: where each loaded ship is taking its troops, and
/// how many units each civilization has waiting in port for a ship (its cities build ferries when
/// none is coming).
#[derive(Resource, Default)]
pub struct Ferries {
    pub plans: HashMap<Entity, Hex>,
    pub wanted: HashMap<u32, usize>,
    /// Units each civilization has landed overseas so far (for the autoplay report).
    pub landed: HashMap<u32, u32>,
    /// Turns a ship has lain in port (by port and owner) waiting for a worthwhile load.
    pub waited: HashMap<(Hex, u32), u32>,
}

/// Troops a ferry wants aboard before it sails (a lone unit landed overseas only dies), and the
/// turns it waits for them before taking what there is.
pub const FERRY_LOAD: usize = 4;
pub const FERRY_PATIENCE: u32 = 8;

/// Numbers each land tile by the landmass it belongs to.
pub fn continents(land: &HashSet<Hex>) -> HashMap<Hex, u32> {
    let mut id: HashMap<Hex, u32> = HashMap::new();
    let mut next = 0;
    let mut tiles: Vec<Hex> = land.iter().copied().collect();
    tiles.sort_by_key(|h| (h.x, h.y));
    for start in tiles {
        if id.contains_key(&start) {
            continue;
        }
        let mut stack = vec![start];
        id.insert(start, next);
        while let Some(at) = stack.pop() {
            for n in at.all_neighbors() {
                if land.contains(&n) && !id.contains_key(&n) {
                    id.insert(n, next);
                    stack.push(n);
                }
            }
        }
        next += 1;
    }
    id
}

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
    /// Lost every city (and has no settler left): out of the game.
    #[serde(default)]
    pub eliminated: bool,
    /// How it splits its cities' trade between science, luxury and taxes (set each turn).
    #[serde(default)]
    pub rates: crate::economy::Rates,
}

impl Default for Civilizations {
    fn default() -> Self {
        Self { players: Vec::new() }
    }
}

/// Extra appeal of a technology that gives a better government than the one a people has.
pub const GOVERNMENT_APPEAL: f32 = 3.0;

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
        Tech::Banking => (0.5, 3.5),
        Tech::Economics => (0.5, 3.5),
        Tech::Astronomy => (0.5, 3.0),
        Tech::Chivalry => (3.5, 0.5),
        Tech::Navigation => (1.5, 3.0),
        Tech::TheoryOfGravity => (0.5, 3.5),
        Tech::Medicine => (0.5, 3.0),
        Tech::Steel => (3.5, 2.0),
        Tech::Electronics => (1.5, 3.5),
        Tech::Flight => (3.5, 2.0),
        Tech::AdvancedFlight => (4.0, 1.5),
        Tech::MassProduction => (1.5, 3.5),
        Tech::Ecology => (0.5, 3.0),
        Tech::NuclearFission => (4.0, 1.5),
        Tech::GeneticEngineering => (0.5, 4.0),
    }
}

/// The technology a computer player studies next: warlike leaders favour arms, builders the economy,
/// and cheap discoveries are preferred over distant ones.
pub fn pick_research(tech: &TechState, aggression: f32) -> Option<Tech> {
    let now = crate::government::Government::preferred(tech, aggression);
    let score = |t: Tech| {
        let (war, peace) = appeal(t);
        // A technology that opens a better form of government is worth a detour: science, corruption
        // and production all depend on it.
        let better = crate::government::Government::CHOICES.iter().any(|g| {
            g.required_tech() == Some(t) && {
                let mut with = tech.clone();
                with.researched.insert(t);
                crate::government::Government::preferred(&with, aggression) == *g && *g != now
            }
        });
        war * aggression + peace * (1.0 - aggression) - t.cost() as f32 / 40.0 + if better { GOVERNMENT_APPEAL } else { 0.0 }
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

/// Walls and digging in, as one factor on a garrison's defence, as the computer players reckon it.
pub const SIEGE_FORTIFICATION: f32 = 1.5;
/// Safety margin on the number of attackers a siege needs.
pub const SIEGE_MARGIN: f32 = 1.0;
/// Most attackers a siege ever waits for.
pub const SIEGE_MAX: f32 = 8.0;

/// One land unit seen by the siege planner: where, whose, attack and defence of each man in it,
/// and how many men (an army counts every member).
pub struct Fighter {
    pub at: Hex,
    pub owner: u32,
    pub attack: f32,
    pub defence: f32,
    pub men: u32,
    /// Hit points left, as a share of full (wounded defenders are cheaper to kill).
    pub vigour: f32,
}

impl Fighter {
    pub fn of(unit: &Unit) -> Option<Fighter> {
        if !unit.unit_type.is_army() || unit.unit_type.domain() != Domain::Land {
            return None;
        }
        let (atk, def, _) = unit.unit_type.stats();
        let shape = (1.0 + 0.25 * unit.level as f32) * (1.0 + crate::unit::ARMY_MEMBER_BONUS * unit.troops.len() as f32);
        Some(Fighter {
            at: unit.position,
            owner: unit.owner,
            attack: atk as f32 * unit.unit_type.attack_bonus(crate::terrain::Terrain::Plains) * shape,
            defence: def as f32 * unit.unit_type.defense_bonus() * shape,
            men: unit.members(),
            vigour: (unit.hp as f32 / unit.max_hp().max(1) as f32).clamp(0.34, 1.0),
        })
    }
}

/// Whether `attackers` gathered before a city can take its `garrison`: a blow by one attacker lands
/// with chance p = attack / (attack + defence), and a unit that fights to the end deals about
/// (1 - p) / p blows for each it takes, so each defender costs about (1 - p) / p attackers.
pub fn siege_ready(attackers: &[&Fighter], garrison: &[&Fighter], aggression: f32) -> bool {
    let men: f32 = attackers.iter().map(|f| f.men as f32).sum();
    if men < 1.0 {
        return false;
    }
    let attack = attackers.iter().map(|f| f.attack * f.men as f32).sum::<f32>() / men;
    let defence = garrison.iter().map(|f| f.defence).fold(0.0, f32::max) * SIEGE_FORTIFICATION;
    let defenders: f32 = garrison.iter().map(|f| f.men as f32 * f.vigour).sum();
    if defenders == 0.0 {
        return true;
    }
    let p = attack / (attack + defence).max(0.01);
    // Warlike leaders commit with fewer men.
    let margin = if aggression > 0.6 { SIEGE_MARGIN * 0.5 } else { SIEGE_MARGIN };
    let needed = (defenders * (1.0 - p) / p.max(0.02) * margin + 1.0).min(SIEGE_MAX);
    men >= needed
}

/// Moves every computer-controlled unit and returns the attacks they launch.
///
/// `hostile(a, b)` says whether owner `a` is willing to attack owner `b`. `must_fall`: cities
/// worth marching much further for (the capital of a people whose spaceship is in flight).
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
    ferries: &mut Ferries,
    commands: &mut Commands,
    must_fall: &[Hex],
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
    let fighters: Vec<Fighter> = units.iter().filter_map(|(_, u)| Fighter::of(&u)).collect();
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
    let continent = continents(&land_set);
    let water_set: HashSet<Hex> = land.iter().filter(|(_, t)| *t == Terrain::Water).map(|(c, _)| *c).collect();
    let same_land = |a: Hex, b: Hex| continent.get(&a).is_some_and(|c| continent.get(&b) == Some(c));
    // Coastal cities: where troops wait for a ship and ferries put in.
    let harbours: Vec<(Hex, u32)> =
        city_positions.iter().copied().filter(|(p, _)| p.all_neighbors().iter().any(|n| water_set.contains(n))).collect();
    // The port on this landmass from which a ship best carries a unit at `from` towards `goal`.
    let port_for = |owner: u32, from: Hex, goal: Hex| {
        harbours.iter().filter(|(p, o)| *o == owner && same_land(*p, from)).min_by_key(|(p, _)| (from.distance_to(*p) + p.distance_to(goal), p.x, p.y)).map(|(p, _)| *p)
    };
    // Units that want a ship: (unit, where it is going, the port it waits in).
    let mut requests: Vec<(Entity, u32, Hex, Hex)> = Vec::new();
    // Where each owner's enemies' armies stand (for zones of control), worked out once per owner.
    let mut enemy_armies_of: HashMap<u32, std::collections::HashSet<Hex>> = HashMap::new();
    // Each civilization's airfields: its cities and its carriers.
    let carriers: Vec<(Hex, u32)> = units.iter().filter(|(_, u)| u.unit_type == UnitType::Carrier).map(|(_, u)| (u.position, u.owner)).collect();
    let roads = road_network(tiles.iter(), city_positions.iter().map(|(p, _)| *p));
    let terrain_at: HashMap<Hex, Terrain> = land.iter().copied().collect();
    let polluted: std::collections::HashSet<Hex> = tiles.iter().filter(|t| t.is_polluted()).map(|t| t.coord).collect();
    let forts: std::collections::HashSet<Hex> = tiles.iter().filter(|t| t.improvements.contains(&Improvement::Fortress)).map(|t| t.coord).collect();
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
    let features: HashMap<Hex, (Terrain, bool, bool, bool)> = tiles
        .iter()
        .map(|t| (t.coord, (t.terrain, t.improvements.contains(&Improvement::Farm), t.improvements.contains(&Improvement::Mine), t.has_road())))
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
        // Units at sea in a ship, and the ferries and carriers themselves, are moved below.
        if unit.carrier.is_some() || unit.unit_type.is_ferry() || unit.unit_type == UnitType::Carrier {
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
            if let Some(path) = army.filter(|a| *a != here).and_then(|a| find_route(here, a, land.iter().copied(), &roads, crate::pathfinding::land_travel(unit.unit_type.is_wheeled(), false))) {
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
            // The best free site on this landmass; failing that, one overseas once Seafaring is known
            // (a ship carries the settler there from the nearest port).
            let free = |h: &Hex| !crowded(*h) && !occupied.get(h).is_some_and(|(o, _)| *o != owner);
            let best = |near: &dyn Fn(Hex) -> bool| {
                land_set.iter().copied().filter(|h| near(*h) && free(h)).max_by_key(|h| (site_value(*h) * 4 - h.distance_to(here) * 3, h.x, h.y))
            };
            let target = best(&|h: Hex| h.distance_to(here) <= 10 && same_land(h, here));
            if target.is_none() && can_sail && is_land(here) && !free(&here) {
                let overseas = best(&|h: Hex| h.distance_to(here) <= 22 && !same_land(h, here));
                if let Some((goal, port)) = overseas.and_then(|g| port_for(owner, here, g).map(|p| (g, p))) {
                    requests.push((entity, owner, goal, port));
                    if here != port {
                        if let Some(path) = find_route(here, port, land.iter().copied(), &roads, Travel::Land) {
                            for next in path.into_iter().skip(1) {
                                if !unit.can_move() || occupied.get(&next).is_some_and(|(o, _)| *o != owner) {
                                    break;
                                }
                                let cost = land_step(unit.position, next, terrain_at.get(&next).copied(), &roads, true);
                                unit.step(next, cost);
                            }
                        }
                    }
                    unit.end_move();
                    continue;
                }
            }
            let settle_here = is_land(here) && !crowded(here) && target.is_none_or(|t| t == here || site_value(here) * 4 >= site_value(t) * 4 - 6);
            if !settle_here {
                if let Some(target) = target {
                    if let Some(path) = find_route(here, target, land.iter().copied(), &roads, Travel::Land) {
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
            // A hill or mountain of ours with hostile armies close by: fortify it.
            let high = matches!(terrain_at.get(&here), Some(Terrain::Hills | Terrain::Mountain));
            let threatened = unit_snapshots.iter().any(|(pos, other, strength)| pos.distance_to(here) <= 4 && *strength > 0.0 && hostile(owner, *other));
            let knows_construction = civs.players.iter().any(|c| c.id == owner && c.tech.researched.contains(&Tech::Construction));
            if high && threatened && knows_construction && owns(here, owner) && !forts.contains(&here) && !city_positions.iter().any(|(p, _)| *p == here) {
                unit.start_build(Improvement::Fortress);
                continue;
            }
            // No road work: farm the flat land and mine the hills inside the borders, and road the
            // flat land once it is farmed (a road brings trade on grassland, plains and desert).
            let on_city = city_positions.iter().any(|(p, _)| *p == here);
            if let Some((terrain, farm, mine, road)) = features.get(&here).copied().filter(|_| owns(here, owner) && !on_city) {
                let job = match terrain {
                    Terrain::Hills | Terrain::Mountain if !mine => Some(Improvement::Mine),
                    Terrain::Grassland | Terrain::Plains if !farm => Some(Improvement::Farm),
                    Terrain::Grassland | Terrain::Plains | Terrain::Desert if !road => Some(Improvement::Road),
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
            let airfields: Vec<Hex> = ports.iter().copied().chain(carriers.iter().filter(|(_, o)| *o == owner).map(|(p, _)| *p)).collect();
            let in_reach = |hex: Hex| {
                if flying {
                    airfields.iter().any(|p| p.distance_to(hex) <= AIR_RANGE)
                } else {
                    // A ship can strike anything at sea or on the shore.
                    !is_land(hex) || hex.all_neighbors().iter().any(|n| !is_land(*n))
                }
            };
            let prey = unit_snapshots
                .iter()
                .filter(|(pos, other, _)| here.distance_to(*pos) <= 7 && hostile(owner, *other) && in_reach(*pos))
                .min_by_key(|(pos, _, _)| here.distance_to(*pos))
                .map(|(pos, _, _)| *pos);
            let base = if flying { airfields.iter().copied().min_by_key(|p| here.distance_to(*p)) } else { ports.iter().copied().min_by_key(|p| here.distance_to(*p)) };
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
            .filter(|(_, (p, o))| *o == owner && here.distance_to(*p) <= 8 && same_land(*p, here))
            .min_by_key(|(_, (p, _))| here.distance_to(*p))
            .map(|(index, (p, _))| (index, *p));
        let enemy_unit = unit_snapshots
            .iter()
            .filter(|(pos, other, strength)| {
                here.distance_to(*pos) <= 4 && hostile(owner, *other) && my_strength > *strength * threshold && same_land(*pos, here)
            })
            .min_by_key(|(pos, _, _)| here.distance_to(*pos))
            .map(|(pos, _, _)| *pos);
        let city_range = if aggression > 0.6 { 14 } else { 7 };
        let urgent = |pos: &Hex| must_fall.contains(pos);
        let enemy_city = city_positions
            .iter()
            .filter(|(pos, other)| here.distance_to(*pos) <= if urgent(pos) { 3 * city_range } else { city_range } && hostile(owner, *other) && same_land(*pos, here))
            .min_by_key(|(pos, _)| (!urgent(pos), here.distance_to(*pos)))
            .map(|(pos, _)| *pos);
        // Nothing to fight on this landmass: an enemy city overseas is reached by ship.
        if relief.is_none() && enemy_unit.is_none() && enemy_city.is_none() && can_sail && is_land(here) {
            let overseas = city_positions
                .iter()
                .filter(|(pos, other)| hostile(owner, *other) && *other != 99 && here.distance_to(*pos) <= if urgent(pos) { 4 * city_range } else { 2 * city_range + 6 } && !same_land(*pos, here))
                .min_by_key(|(pos, _)| (!urgent(pos), here.distance_to(*pos), pos.x, pos.y))
                .map(|(pos, _)| *pos);
            if let Some((goal, port)) = overseas.and_then(|g| port_for(owner, here, g).map(|p| (g, p))) {
                requests.push((entity, owner, goal, port));
                if here != port {
                    if let Some(path) = find_route(here, port, land.iter().copied(), &roads, crate::pathfinding::land_travel(unit.unit_type.is_wheeled(), false)) {
                        for next in path.into_iter().skip(1) {
                            if !unit.can_move() || occupied.get(&next).is_some_and(|(o, _)| *o != owner) {
                                break;
                            }
                            let cost = land_step(unit.position, next, terrain_at.get(&next).copied(), &roads, true);
                            unit.step(next, cost);
                        }
                    }
                }
                unit.end_move();
                continue;
            }
        }

        // A siege: enough of the army gathered before a defended city to outweigh its garrison.
        let mut siege = None;
        let goal = if let Some((index, city)) = relief {
            undefended.remove(index);
            Some(city)
        } else if enemy_unit.is_some() {
            enemy_unit
        } else if let Some(city) = enemy_city {
            // A defended city is attacked in strength: wait nearby until enough of the army has
            // gathered - at least two or three units, and half again the garrison's strength.
            let defended = occupied.get(&city).is_some_and(|(other, _)| *other != owner);
            let needed = if !defended { 1 } else if aggression > 0.6 { 2 } else { 3 };
            let near: Vec<&Fighter> = fighters.iter().filter(|f| f.owner == owner && f.attack > 0.0 && here.distance_to(f.at) <= 3).collect();
            let garrison_of: Vec<&Fighter> = fighters.iter().filter(|f| f.at == city && f.owner != owner).collect();
            let ready = !defended || (near.len() >= needed && siege_ready(&near, &garrison_of, aggression));
            let available = field_armies.get(&owner).copied().unwrap_or(0);
            if available < needed {
                None
            } else if !ready && here.distance_to(city) <= 3 {
                continue;
            } else {
                siege = ready.then_some(city);
                Some(city)
            }
        } else {
            None
        };

        if let Some(target) = goal {
            let travel = crate::pathfinding::land_travel(unit.unit_type.is_wheeled(), false);
            let enemy_armies = enemy_armies_of.entry(owner).or_insert_with(|| {
                unit_snapshots.iter().filter(|(_, other, strength)| *strength > 0.0 && hostile(owner, *other)).map(|(pos, _, _)| *pos).collect()
            });
            if let Some(path) = find_route(here, target, land.iter().copied(), &roads, travel) {
                for next in path.into_iter().skip(1) {
                    if !unit.can_move() || zoc_blocks(unit.position, next, enemy_armies) {
                        break;
                    }
                    let boarding = is_land(unit.position) != is_land(next);
                    match occupied.get(&next) {
                        Some((other, strength)) if *other != owner => {
                            // Attack if the odds look acceptable (or the siege is ready: every unit
                            // gathered before the city joins in, wearing the garrison down); otherwise
                            // hold position.
                            if hostile(owner, *other) && (aggression > 0.6 || my_strength >= *strength || siege == Some(next)) {
                                attacks.push(AiAttack { attacker: entity, target: next });
                                unit.end_move();
                            }
                            break;
                        }
                        _ => {
                            // Roads speed the march; boarding or leaving a ship takes the rest of the turn.
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

    sail_ferries(units, &requests, &land, (&land_set, &water_set), &continent, city_positions, &occupied, ferries);
    attacks
}

/// The computer players' ferries: an empty ship sails to the port where most troops wait and takes
/// them aboard; a loaded one sails for the coast nearest its troops' goal and lands them there.
#[allow(clippy::too_many_arguments)]
fn sail_ferries(
    units: &mut Query<(Entity, &mut Unit)>,
    requests: &[(Entity, u32, Hex, Hex)],
    land: &[(Hex, Terrain)],
    (land_set, water): (&HashSet<Hex>, &HashSet<Hex>),
    continent: &HashMap<Hex, u32>,
    city_positions: &[(Hex, u32)],
    occupied: &HashMap<Hex, (u32, f32)>,
    ferries: &mut Ferries,
) {
    let berths: Vec<Berth> = units.iter().map(|(e, u)| Berth::of(e, &u)).collect();
    let mut waiting: HashMap<(Hex, u32), Vec<(Entity, Hex)>> = HashMap::new();
    ferries.wanted.clear();
    for (entity, owner, goal, port) in requests {
        *ferries.wanted.entry(*owner).or_default() += 1;
        if berths.iter().any(|b| b.entity == *entity && b.position == *port) {
            waiting.entry((*port, *owner)).or_default().push((*entity, *goal));
        }
    }
    let alive: HashSet<Entity> = berths.iter().map(|b| b.entity).collect();
    ferries.plans.retain(|ship, _| alive.contains(ship));
    let ships: Vec<Entity> = berths.iter().filter(|b| b.owner != 0 && b.unit_type.is_ferry()).map(|b| b.entity).collect();
    for ship in ships {
        let Ok((_, unit)) = units.get(ship) else { continue };
        let (owner, capacity) = (unit.owner, unit.unit_type.capacity());
        let ports: HashSet<Hex> = city_positions.iter().filter(|(_, o)| *o == owner).map(|(p, _)| *p).collect();
        let mut cargo: Vec<Entity> = berths.iter().filter(|b| b.carrier.map(|c| c.0) == Some(ship)).map(|b| b.entity).collect();
        let here = unit.position;
        // Empty in a port where troops wait: take them aboard.
        if cargo.is_empty() {
            let patient = ferries.waited.get(&(here, owner)).copied().unwrap_or(0) >= FERRY_PATIENCE;
            // Settlers sail at once; troops wait for company.
            let settling = |list: &Vec<(Entity, Hex)>| list.iter().any(|(e, _)| berths.iter().any(|b| b.entity == *e && b.unit_type == UnitType::Settler));
            if let Some(list) = waiting.get_mut(&(here, owner)).filter(|l| !l.is_empty() && (l.len() >= FERRY_LOAD.min(capacity as usize) || patient || settling(l))) {
                ferries.waited.remove(&(here, owner));
                let take = list.len().min(capacity as usize);
                let loaded: Vec<(Entity, Hex)> = list.drain(..take).collect();
                if let Some((_, goal)) = loaded.first() {
                    ferries.plans.insert(ship, *goal);
                }
                for (entity, _) in loaded {
                    if let Ok((_, mut troop)) = units.get_mut(entity) {
                        troop.carrier = Some((ship, here));
                        cargo.push(entity);
                    }
                }
            }
        }
        if cargo.is_empty() && waiting.get(&(here, owner)).is_some_and(|l| !l.is_empty()) {
            // Troops are waiting here but too few to sail: the ship waits for more.
            *ferries.waited.entry((here, owner)).or_default() += 1;
            continue;
        }
        if cargo.is_empty() {
            // Make for the port with the most troops waiting (or home).
            let pickup = waiting
                .iter()
                .filter(|((_, o), list)| *o == owner && !list.is_empty())
                .max_by_key(|((p, _), list)| (list.len(), -(here.distance_to(*p)), p.x, p.y))
                .map(|((p, _), _)| *p);
            let home = ports.iter().copied().filter(|p| p.all_neighbors().iter().any(|n| water.contains(n))).min_by_key(|p| (here.distance_to(*p), p.x, p.y));
            if let Some(dest) = pickup.or(home).filter(|d| *d != here) {
                sail(units, ship, dest, (land, water), &ports, occupied, &[]);
            }
            continue;
        }
        // Loaded: land the troops on the coast nearest their goal.
        let goal = ferries.plans.get(&ship).copied().or_else(|| {
            city_positions.iter().filter(|(_, o)| *o != owner).min_by_key(|(p, _)| here.distance_to(*p)).map(|(p, _)| *p)
        });
        let Some(goal) = goal else { continue };
        let goal_land = continent.get(&goal).copied();
        let foreign_city = |h: Hex| city_positions.iter().any(|(p, o)| *p == h && *o != owner);
        let shore = |h: Hex| {
            land_set.contains(&h) && continent.get(&h).copied() == goal_land && !foreign_city(h) && !occupied.get(&h).is_some_and(|(o, _)| *o != owner)
        };
        let beach = hexx::shapes::hexagon(goal, 6)
            .filter(|h| shore(*h) && h.all_neighbors().iter().any(|n| water.contains(n)))
            .min_by_key(|h| (h.distance_to(goal), here.distance_to(*h), h.x, h.y));
        let Some(beach) = beach else {
            ferries.plans.remove(&ship);
            continue;
        };
        let near = |at: Hex| at.all_neighbors().into_iter().filter(|n| shore(*n)).min_by_key(|n| (n.distance_to(goal), n.x, n.y));
        let landing_ok = |at: Hex| near(at).is_some_and(|n| n.distance_to(goal) <= beach.distance_to(goal) + 2);
        if !landing_ok(here) {
            let water_by_beach = beach.all_neighbors().into_iter().filter(|n| water.contains(n)).min_by_key(|n| (here.distance_to(*n), n.x, n.y));
            if let Some(dest) = water_by_beach {
                sail(units, ship, dest, (land, water), &ports, occupied, &cargo);
            }
        }
        let Ok((_, unit)) = units.get(ship) else { continue };
        let at = unit.position;
        if landing_ok(at) && !ports.contains(&at) {
            let Some(landing) = near(at) else { continue };
            for entity in &cargo {
                if let Ok((_, mut troop)) = units.get_mut(*entity) {
                    troop.position = landing;
                    troop.carrier = None;
                    troop.fortified = false;
                    troop.end_move();
                    *ferries.landed.entry(owner).or_default() += 1;
                }
            }
            ferries.plans.remove(&ship);
        }
    }
}

/// Moves a ship towards `dest` as far as its moves take it, its cargo with it.
fn sail(
    units: &mut Query<(Entity, &mut Unit)>,
    ship: Entity,
    dest: Hex,
    (land, water): (&[(Hex, Terrain)], &HashSet<Hex>),
    ports: &HashSet<Hex>,
    occupied: &HashMap<Hex, (u32, f32)>,
    cargo: &[Entity],
) {
    let Ok((_, mut unit)) = units.get_mut(ship) else { return };
    let owner = unit.owner;
    let Some(path) = find_path(unit.position, dest, land.iter().copied(), Travel::Sea(ports)) else { return };
    for next in path.into_iter().skip(1) {
        if !unit.can_move() || occupied.get(&next).is_some_and(|(o, _)| *o != owner) || !(water.contains(&next) || ports.contains(&next)) {
            break;
        }
        unit.step(next, StepCost::Full);
    }
    let at = unit.position;
    for entity in cargo {
        if let Ok((_, mut troop)) = units.get_mut(*entity) {
            troop.position = at;
            troop.carrier = Some((ship, at));
        }
    }
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
        process_ai_turn(&mut civs, &mut units, &cities, &tiles, &|_, _| false, &mut jobs.0, &|hex, _| hex.distance_to(Hex::ZERO) <= 2, &mut Ferries::default(), &mut commands, &[]);
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
            eliminated: false,
            rates: Default::default(),
        }
    }

    fn world_with_worker(terrain: Terrain) -> (World, Entity) {
        let mut world = World::new();
        for q in -3..=3 {
            world.spawn(HexTile { coord: Hex::new(q, 0), terrain, improvements: vec![], resource: None });
        }
        world.insert_resource(Civilizations { players: vec![rival()] });
        let worker = world
            .spawn(Unit { unit_type: UnitType::Worker, movement_left: 1, owner: 1, position: Hex::ZERO, build_turns_left: None, current_build: None, fortified: false, road_credit: 0, hp: 3, level: 0, troops: Vec::new(), fatigue: 0, carrier: None })
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
            .spawn(Unit { unit_type: kind, movement_left: 1, owner, position: Hex::new(q, 0), build_turns_left: None, current_build: None, fortified: false, road_credit: 0, hp: 3, level: 0, troops: Vec::new(), fatigue: 0, carrier: None })
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
        process_ai_turn(&mut civs, &mut units, &cities, &tiles, &|a, b| a != b, &mut HashMap::new(), &|_, _| false, &mut Ferries::default(), &mut commands, &[]);
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
        let galley = soldier(&mut world, UnitType::Frigate, 1, 0);
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
        let galley = soldier(&mut world, UnitType::Frigate, 1, 0);
        world.run_system_once(naval_phase).unwrap();
        assert_eq!(world.get::<Unit>(galley).unwrap().position, Hex::ZERO);
    }

    fn naval_phase(mut civs: ResMut<Civilizations>, mut units: Query<(Entity, &mut Unit)>, tiles: Query<&mut HexTile>, mut commands: Commands) -> Vec<Hex> {
        let cities = [(Hex::ZERO, 1)];
        process_ai_turn(&mut civs, &mut units, &cities, &tiles, &|a, b| a != b, &mut HashMap::new(), &|_, _| false, &mut Ferries::default(), &mut commands, &[])
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
    fn a_siege_waits_for_enough_men_to_wear_the_garrison_down() {
        let man = |kind: UnitType, at: Hex| {
            let (atk, def, _) = kind.stats();
            Fighter { at, owner: 1, attack: atk as f32 * kind.attack_bonus(Terrain::Plains), defence: def as f32 * kind.defense_bonus(), men: 1, vigour: 1.0 }
        };
        let city = Hex::new(5, 0);
        let garrison = [man(UnitType::MechInfantry, city), man(UnitType::MechInfantry, city)];
        let few: Vec<Fighter> = (0..3).map(|_| man(UnitType::RadarArtillery, Hex::ZERO)).collect();
        let many: Vec<Fighter> = (0..12).map(|_| man(UnitType::RadarArtillery, Hex::ZERO)).collect();
        let g: Vec<&Fighter> = garrison.iter().collect();
        assert!(!siege_ready(&few.iter().collect::<Vec<_>>(), &g, 0.4), "three guns cannot take two walled mechanized infantry");
        assert!(siege_ready(&many.iter().collect::<Vec<_>>(), &g, 0.4));
        // An empty city (or one held by workers only) falls to anyone.
        assert!(siege_ready(&few.iter().collect::<Vec<_>>(), &[], 0.4));
        // Warlike leaders commit sooner.
        let eight: Vec<Fighter> = (0..8).map(|_| man(UnitType::RadarArtillery, Hex::ZERO)).collect();
        let eight: Vec<&Fighter> = eight.iter().collect();
        assert!(!siege_ready(&eight, &g, 0.4) || siege_ready(&eight, &g, 0.9));
    }

    #[test]
    fn a_better_government_is_worth_a_detour() {
        // Monarchy only needs Code of Laws: a builder with Alphabet and Code of Laws learned goes for it
        // before the other techs on offer.
        let mut tech = TechState::default();
        tech.researched.extend([Tech::Alphabet, Tech::CodeOfLaws]);
        assert_eq!(pick_research(&tech, 0.4), Some(Tech::Monarchy));
    }

    #[test]
    fn idle_worker_improves_land_inside_the_borders() {
        let (mut world, worker) = world_with_worker(Terrain::Hills);
        world.get_mut::<Unit>(worker).unwrap().position = Hex::new(1, 0);
        world.insert_resource(Jobs(HashMap::new()));
        world.run_system_once(ai_phase).unwrap();
        assert_eq!(world.get::<Unit>(worker).unwrap().current_build, Some(Improvement::Mine));
    }

    #[test]
    fn farmed_flat_land_gets_a_road_for_its_trade() {
        let (mut world, worker) = world_with_worker(Terrain::Grassland);
        for mut tile in world.query::<&mut HexTile>().iter_mut(&mut world) {
            if tile.coord == Hex::new(1, 0) {
                tile.add_improvement(Improvement::Farm);
            }
        }
        world.get_mut::<Unit>(worker).unwrap().position = Hex::new(1, 0);
        world.insert_resource(Jobs(HashMap::new()));
        world.run_system_once(ai_phase).unwrap();
        assert_eq!(world.get::<Unit>(worker).unwrap().current_build, Some(Improvement::Road));
    }
}
