use hexx::{algorithms::a_star, Hex};
use std::collections::HashMap;

use crate::terrain::{HexTile, Terrain};

/// How many tiles of road one move covers: a road step costs a third of a move.
pub const ROAD_STEPS_PER_MOVE: u32 = 3;

/// The best way along a tile. Cities count as railroad: lines run on through them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Road {
    Road,
    Rail,
}

/// Every road, railroad and city on the map.
pub fn road_network<'a>(
    tiles: impl Iterator<Item = &'a HexTile>,
    cities: impl Iterator<Item = Hex>,
) -> HashMap<Hex, Road> {
    let mut roads: HashMap<Hex, Road> = tiles
        .filter_map(|t| {
            if t.has_railroad() {
                Some((t.coord, Road::Rail))
            } else {
                t.has_road().then_some((t.coord, Road::Road))
            }
        })
        .collect();
    roads.extend(cities.map(|c| (c, Road::Rail)));
    roads
}

/// What one step of a land unit costs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StepCost {
    /// Off the roads: a whole move.
    Full,
    /// Rough ground off the roads (hills and forest 2, mountains 3). A unit with fewer moves left can
    /// still enter, but what it cannot pay is owed from its next turns (it is worn out).
    Rough(u32),
    /// Road to road: a third of a move.
    Road,
    /// Railroad to railroad: nothing.
    Free,
    /// Boarding or leaving a boat: the rest of the turn.
    EndsTurn,
}

/// Zone of control: an army may not slip from one tile next to an enemy army to another tile next to
/// the same enemy. (Moving onto the enemy is an attack, handled elsewhere.)
pub fn zoc_blocks(from: Hex, to: Hex, enemies: &std::collections::HashSet<Hex>) -> bool {
    !enemies.contains(&to)
        && enemies
            .iter()
            .any(|e| e.distance_to(from) == 1 && e.distance_to(to) == 1)
}

/// The cost of a step onto a tile of `terrain` for a unit on foot (or anything else), counting roads.
pub fn land_step(
    from: Hex,
    to: Hex,
    terrain: Option<Terrain>,
    roads: &HashMap<Hex, Road>,
    on_foot: bool,
) -> StepCost {
    match step_cost(from, to, roads, on_foot) {
        StepCost::Full if on_foot => match terrain.and_then(terrain_cost) {
            Some(cost) if cost > 1 => StepCost::Rough(cost),
            _ => StepCost::Full,
        },
        other => other,
    }
}

/// The cost of a step between neighbouring tiles for a unit of the given travel, ignoring boarding.
/// Only troops on foot use roads; ships and aircraft always pay a whole move.
pub fn step_cost(from: Hex, to: Hex, roads: &HashMap<Hex, Road>, on_foot: bool) -> StepCost {
    if !on_foot {
        return StepCost::Full;
    }
    match (roads.get(&from), roads.get(&to)) {
        (Some(a), Some(b)) if (*a).min(*b) == Road::Rail => StepCost::Free,
        (Some(_), Some(_)) => StepCost::Road,
        _ => StepCost::Full,
    }
}

/// The movement a unit has left this turn: whole moves plus thirds of a move left over from
/// travelling on roads.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Budget {
    pub moves: u32,
    pub road_credit: u32,
    /// Moves owed after entering rough ground without enough left: taken from the next turns.
    pub debt: u32,
}

impl Budget {
    pub fn can_step(&self) -> bool {
        self.moves > 0 || self.road_credit > 0
    }

    /// Pays for one step. Off the roads, a part-move left from road travel still buys one last step.
    pub fn spend(&mut self, cost: StepCost) {
        match cost {
            StepCost::Free => {}
            StepCost::Road if self.road_credit > 0 => self.road_credit -= 1,
            StepCost::Road => {
                self.moves = self.moves.saturating_sub(1);
                self.road_credit = ROAD_STEPS_PER_MOVE - 1;
            }
            StepCost::Full if self.moves > 0 => self.moves -= 1,
            StepCost::Full => self.road_credit = 0,
            StepCost::Rough(cost) if self.moves > 0 => {
                self.debt += cost.saturating_sub(self.moves);
                self.moves = self.moves.saturating_sub(cost);
                self.road_credit = 0;
            }
            // Only a part-move left from the road: it counts as one move towards the climb.
            StepCost::Rough(cost) => {
                self.debt += cost.saturating_sub(1);
                self.road_credit = 0;
            }
            StepCost::EndsTurn => *self = Budget::default(),
        }
    }
}

/// Movement cost for a terrain type. None = impassable.
pub fn terrain_cost(terrain: Terrain) -> Option<u32> {
    match terrain {
        Terrain::Water => None,
        Terrain::Grassland | Terrain::Plains => Some(1),
        Terrain::Desert => Some(1),
        Terrain::Forest => Some(2),
        Terrain::Hills => Some(2),
        Terrain::Mountain => Some(3),
    }
}

/// How a unit travels.
#[derive(Clone, Copy)]
pub enum Travel<'a> {
    /// On foot; water is impassable.
    Land,
    /// On foot, boarding a boat to cross water (slowly).
    Amphibious,
    /// A ship: water, plus the given ports (the owner's coastal cities).
    Sea(&'a std::collections::HashSet<Hex>),
    /// An aircraft: anywhere on the map.
    Air,
    /// On wheels (chariots, siege guns): like on foot, but forest and mountains only along a road;
    /// `boats` lets it cross water too.
    Wheels { boats: bool },
}

/// How a land unit travels: on foot or on wheels, by boat once its people can sail.
pub fn land_travel(wheeled: bool, boats: bool) -> Travel<'static> {
    match (wheeled, boats) {
        (true, _) => Travel::Wheels { boats },
        (false, true) => Travel::Amphibious,
        (false, false) => Travel::Land,
    }
}

/// Find a path from start to end using A*, respecting how the unit travels.
/// The destination itself is always allowed, so a ship can be routed up to a target on the shore.
/// Returns the path (including start) or None if unreachable.
pub fn find_path(
    start: Hex,
    end: Hex,
    tiles: impl Iterator<Item = (Hex, Terrain)>,
    travel: Travel,
) -> Option<Vec<Hex>> {
    find_route(start, end, tiles, &HashMap::new(), travel)
}

/// Like [`find_path`], but troops on foot prefer roads and railroads, which they travel faster.
pub fn find_route(
    start: Hex,
    end: Hex,
    tiles: impl Iterator<Item = (Hex, Terrain)>,
    roads: &HashMap<Hex, Road>,
    travel: Travel,
) -> Option<Vec<Hex>> {
    let tile_map: HashMap<Hex, Terrain> = tiles.collect();
    // Costs are in sixths of a move so that road (2) and rail (1) steps stay at least as dear as the
    // straight-line estimate the search uses, which keeps the routes it finds the shortest.
    const SCALE: u32 = 6;
    a_star(start, end, |from, to| {
        // The search asks whether the start itself can be stood on: it can (troops aboard a ship
        // start at sea).
        if to == start {
            return Some(0);
        }
        let terrain = *tile_map.get(&to)?;
        let on_foot = matches!(
            travel,
            Travel::Land | Travel::Amphibious | Travel::Wheels { .. }
        );
        if on_foot && terrain != Terrain::Water {
            match step_cost(from, to, roads, true) {
                StepCost::Free => return Some(1),
                StepCost::Road => return Some(2),
                _ => {}
            }
        }
        match travel {
            // Troops may end a route on the sea only to board a ship there (the caller checks).
            Travel::Land if terrain == Terrain::Water => (to == end).then_some(SCALE),
            Travel::Land => terrain_cost(terrain).map(|c| c * SCALE),
            Travel::Amphibious => {
                if terrain == Terrain::Water {
                    Some(3 * SCALE)
                } else {
                    terrain_cost(terrain).map(|c| c * SCALE)
                }
            }
            Travel::Sea(ports) => {
                (terrain == Terrain::Water || ports.contains(&to) || to == end).then_some(SCALE)
            }
            Travel::Air => Some(SCALE),
            Travel::Wheels { .. } if matches!(terrain, Terrain::Forest | Terrain::Mountain) => None,
            Travel::Wheels { boats } => {
                if terrain == Terrain::Water {
                    if boats {
                        Some(3 * SCALE)
                    } else {
                        (to == end).then_some(SCALE)
                    }
                } else {
                    terrain_cost(terrain).map(|c| c * SCALE)
                }
            }
        }
    })
}

/// Move a unit along a path up to its remaining movement points.
/// Returns how many movement points were spent.
pub fn move_along_path(unit_pos: &mut Hex, movement_left: &mut u32, path: &[Hex]) -> u32 {
    if path.len() <= 1 || *movement_left == 0 {
        return 0;
    }

    let mut spent = 0;
    for &next in path.iter().skip(1) {
        if *movement_left == 0 {
            break;
        }
        *unit_pos = next;
        *movement_left -= 1;
        spent += 1;
    }
    spent
}

/// The turn on which a unit following `route` (which starts at its position) enters each later tile:
/// 0 is this turn. `budget` is what it has now, `moves_at(tile)` the allowance it gets when a turn
/// begins on that tile, and `cost(from, to)` the price of each step. The result has one entry per
/// step, `route.len() - 1` in all.
pub fn turns_along(
    route: &[Hex],
    budget: Budget,
    moves_at: impl Fn(Hex) -> u32,
    cost: impl Fn(Hex, Hex) -> StepCost,
) -> Vec<u32> {
    let mut budget = budget;
    let mut turn = 0;
    route
        .windows(2)
        .map(|step| {
            // A new turn, paying off any moves owed for rough ground first.
            while !budget.can_step() {
                turn += 1;
                let fresh = moves_at(step[0]).max(1);
                let paid = fresh.min(budget.debt);
                budget = Budget {
                    moves: fresh - paid,
                    road_credit: 0,
                    debt: budget.debt - paid,
                };
            }
            budget.spend(cost(step[0], step[1]));
            turn
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moves(n: u32) -> Budget {
        Budget {
            moves: n,
            road_credit: 0,
            debt: 0,
        }
    }

    #[test]
    fn long_routes_are_split_into_turns() {
        let route: Vec<Hex> = (0..=7).map(|q| Hex::new(q, 0)).collect();
        let full = |_: Hex, _: Hex| StepCost::Full;
        // One move left now, then two a turn.
        assert_eq!(
            turns_along(&route, moves(1), |_| 2, full),
            vec![0, 1, 1, 2, 2, 3, 3]
        );
        // A unit that has already moved starts next turn.
        assert_eq!(turns_along(&route[..3], moves(0), |_| 2, full), vec![1, 1]);
        // Boarding at step 2 ends that turn; at sea it gets three moves.
        let boards = |from: Hex, _: Hex| {
            if from.x == 1 {
                StepCost::EndsTurn
            } else {
                StepCost::Full
            }
        };
        let at_sea = |tile: Hex| if tile.x >= 2 { 3 } else { 2 };
        assert_eq!(
            turns_along(&route, moves(2), at_sea, boards),
            vec![0, 0, 1, 1, 1, 2, 2]
        );
        assert!(turns_along(&route[..1], moves(2), |_| 2, full).is_empty());
    }

    #[test]
    fn roads_triple_movement_and_railroads_are_free() {
        let route: Vec<Hex> = (0..=7).map(|q| Hex::new(q, 0)).collect();
        let mut roads: HashMap<Hex, Road> = (0..=3).map(|q| (Hex::new(q, 0), Road::Road)).collect();
        fn cost(roads: &HashMap<Hex, Road>) -> impl Fn(Hex, Hex) -> StepCost + '_ {
            move |a, b| step_cost(a, b, roads, true)
        }
        // A one-move unit covers three road tiles in a turn, then one tile a turn off the road.
        assert_eq!(
            turns_along(&route, moves(1), |_| 1, cost(&roads)),
            vec![0, 0, 0, 1, 2, 3, 4]
        );
        // A railroad the whole way costs nothing.
        roads.extend((0..=7).map(|q| (Hex::new(q, 0), Road::Rail)));
        assert_eq!(
            turns_along(&route, moves(1), |_| 1, cost(&roads)),
            vec![0; 7]
        );
        // Ships and aircraft ignore roads.
        assert_eq!(step_cost(route[0], route[1], &roads, false), StepCost::Full);
        // Rough ground: a two-move unit spends both on a hill, but one move still lets it climb.
        let mut climber = moves(2);
        climber.spend(land_step(
            route[0],
            route[1],
            Some(Terrain::Hills),
            &HashMap::new(),
            true,
        ));
        assert!(!climber.can_step());
        let mut tired = moves(1);
        tired.spend(land_step(
            route[0],
            route[1],
            Some(Terrain::Mountain),
            &HashMap::new(),
            true,
        ));
        assert!(!tired.can_step());
        assert_eq!(
            tired.debt, 2,
            "a mountain wears a one-move unit out for two more turns"
        );
        // So a one-move unit takes two turns a hill and three a mountain, and one a tile on the flat.
        let terrain = |h: Hex| {
            if h.x == 2 {
                Terrain::Hills
            } else if h.x == 3 {
                Terrain::Mountain
            } else {
                Terrain::Plains
            }
        };
        let rough = |a: Hex, b: Hex| land_step(a, b, Some(terrain(b)), &HashMap::new(), true);
        let walk: Vec<Hex> = (0..=4).map(|q| Hex::new(q, 0)).collect();
        assert_eq!(turns_along(&walk, moves(1), |_| 1, rough), vec![0, 1, 3, 6]);
        assert_eq!(
            land_step(route[0], route[1], Some(Terrain::Mountain), &roads, true),
            StepCost::Free,
            "railroad over the pass"
        );
        // Leaving the road with a part-move left still buys one step.
        let mut budget = moves(1);
        budget.spend(StepCost::Road);
        budget.spend(StepCost::Full);
        assert!(!budget.can_step());
    }

    #[test]
    fn armies_cannot_slip_past_an_enemy() {
        let enemy: std::collections::HashSet<Hex> = [Hex::new(1, 0)].into();
        // Both tiles touch the enemy: blocked.
        assert!(zoc_blocks(Hex::new(0, 0), Hex::new(1, -1), &enemy));
        // Stepping away is fine, and so is attacking it.
        assert!(!zoc_blocks(Hex::new(0, 0), Hex::new(-1, 0), &enemy));
        assert!(!zoc_blocks(Hex::new(0, 0), Hex::new(1, 0), &enemy));
    }

    #[test]
    fn routes_follow_the_roads() {
        // Two ways from (0,0) to (4,0): straight over hills, or a detour along a road.
        let tiles: Vec<(Hex, Terrain)> = hexx::shapes::hexagon(Hex::ZERO, 6)
            .map(|h| {
                (
                    h,
                    if h.y == 0 && h.x > 0 && h.x < 4 {
                        Terrain::Hills
                    } else {
                        Terrain::Plains
                    },
                )
            })
            .collect();
        let detour = [
            Hex::new(0, 0),
            Hex::new(0, 1),
            Hex::new(1, 1),
            Hex::new(2, 1),
            Hex::new(3, 1),
            Hex::new(4, 0),
        ];
        let roads: HashMap<Hex, Road> = detour.iter().map(|h| (*h, Road::Road)).collect();
        let route = find_route(
            detour[0],
            Hex::new(4, 0),
            tiles.iter().copied(),
            &roads,
            Travel::Land,
        )
        .unwrap();
        assert_eq!(route, detour);
        assert_ne!(
            find_path(detour[0], Hex::new(4, 0), tiles.into_iter(), Travel::Land).unwrap(),
            detour
        );
    }
    use std::collections::HashSet;

    /// A strip: land at x = 0..=1, water at 2..=4, land at 5..=6.
    fn strait() -> Vec<(Hex, Terrain)> {
        (0..=6)
            .map(|q| {
                (
                    Hex::new(q, 0),
                    if (2..=4).contains(&q) {
                        Terrain::Water
                    } else {
                        Terrain::Plains
                    },
                )
            })
            .collect()
    }

    #[test]
    fn only_boats_and_aircraft_cross_water() {
        let (from, to) = (Hex::new(0, 0), Hex::new(6, 0));
        assert!(find_path(from, to, strait().into_iter(), Travel::Land).is_none());
        assert_eq!(
            find_path(from, to, strait().into_iter(), Travel::Amphibious).map(|p| p.len()),
            Some(7)
        );
        assert_eq!(
            find_path(from, to, strait().into_iter(), Travel::Air).map(|p| p.len()),
            Some(7)
        );
    }

    #[test]
    fn troops_go_ashore_from_a_ship_and_board_one_at_the_end_of_a_route() {
        // From a ship at sea onto the shore next to it.
        let tiles = strait();
        assert_eq!(
            find_route(
                Hex::new(2, 0),
                Hex::new(1, 0),
                tiles.iter().copied(),
                &HashMap::new(),
                Travel::Land
            ),
            Some(vec![Hex::new(2, 0), Hex::new(1, 0)])
        );
        // Onto a ship's tile at the end of a march, but never across the sea.
        assert_eq!(
            find_route(
                Hex::new(0, 0),
                Hex::new(2, 0),
                tiles.iter().copied(),
                &HashMap::new(),
                Travel::Land
            )
            .map(|p| p.len()),
            Some(3)
        );
        assert!(find_route(
            Hex::new(0, 0),
            Hex::new(3, 0),
            tiles.iter().copied(),
            &HashMap::new(),
            Travel::Land
        )
        .is_none());
    }

    #[test]
    fn ships_keep_to_water_and_their_own_ports() {
        let port = Hex::new(1, 0);
        let ports: HashSet<Hex> = [port].into();
        // From port out to sea.
        assert!(find_path(
            port,
            Hex::new(4, 0),
            strait().into_iter(),
            Travel::Sea(&ports)
        )
        .is_some());
        // The far shore can be targeted (to bombard it) but not sailed across.
        assert!(find_path(
            port,
            Hex::new(5, 0),
            strait().into_iter(),
            Travel::Sea(&ports)
        )
        .is_some());
        assert!(find_path(
            port,
            Hex::new(6, 0),
            strait().into_iter(),
            Travel::Sea(&ports)
        )
        .is_none());
        // Inland tiles behind the port are out of reach.
        assert!(find_path(
            Hex::new(3, 0),
            Hex::new(0, 0),
            strait().into_iter(),
            Travel::Sea(&HashSet::new())
        )
        .is_none());
    }

    #[test]
    fn wheels_keep_to_open_ground_or_the_road() {
        // A forest wall at x = 2 except for a road at (2, 0).
        let tiles: Vec<(Hex, Terrain)> = (-1..=4)
            .flat_map(|q| {
                (-3..=3).map(move |r| {
                    (
                        Hex::new(q, r),
                        if q == 2 {
                            Terrain::Forest
                        } else {
                            Terrain::Plains
                        },
                    )
                })
            })
            .collect();
        let none = HashMap::new();
        assert!(find_route(
            Hex::new(0, 1),
            Hex::new(4, 1),
            tiles.iter().copied(),
            &none,
            land_travel(true, false)
        )
        .is_none());
        assert!(find_route(
            Hex::new(0, 1),
            Hex::new(4, 1),
            tiles.iter().copied(),
            &none,
            land_travel(false, false)
        )
        .is_some());
        let road: HashMap<Hex, Road> = [Hex::new(1, 0), Hex::new(2, 0), Hex::new(3, 0)]
            .into_iter()
            .map(|h| (h, Road::Road))
            .collect();
        let route = find_route(
            Hex::new(0, 1),
            Hex::new(4, 1),
            tiles.iter().copied(),
            &road,
            land_travel(true, false),
        )
        .expect("along the road");
        assert!(route.contains(&Hex::new(2, 0)));
    }
}
