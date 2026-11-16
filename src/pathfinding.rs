use hexx::{algorithms::a_star, Hex};
use crate::terrain::Terrain;

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
    let tile_map: std::collections::HashMap<Hex, Terrain> = tiles.collect();

    a_star(start, end, |_, to| {
        let terrain = *tile_map.get(&to)?;
        match travel {
            Travel::Land => terrain_cost(terrain),
            Travel::Amphibious => if terrain == Terrain::Water { Some(3) } else { terrain_cost(terrain) },
            Travel::Sea(ports) => (terrain == Terrain::Water || ports.contains(&to) || to == end).then_some(1),
            Travel::Air => Some(1),
        }
    })
}

/// Move a unit along a path up to its remaining movement points.
/// Returns how many movement points were spent.
pub fn move_along_path(
    unit_pos: &mut Hex,
    movement_left: &mut u32,
    path: &[Hex],
) -> u32 {
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
/// 0 is this turn. `moves_left` is what it has now, `moves_at(tile)` the allowance it gets when a
/// turn begins on that tile, and `ends_move(from, to)` marks a step that uses up the rest of the turn
/// (boarding or leaving a boat). The result has one entry per step, `route.len() - 1` in all.
pub fn turns_along(
    route: &[Hex],
    moves_left: u32,
    moves_at: impl Fn(Hex) -> u32,
    ends_move: impl Fn(Hex, Hex) -> bool,
) -> Vec<u32> {
    let mut moves = moves_left;
    let mut turn = 0;
    route
        .windows(2)
        .map(|step| {
            if moves == 0 {
                turn += 1;
                moves = moves_at(step[0]).max(1);
            }
            moves = if ends_move(step[0], step[1]) { 0 } else { moves - 1 };
            turn
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_routes_are_split_into_turns() {
        let route: Vec<Hex> = (0..=7).map(|q| Hex::new(q, 0)).collect();
        // One move left now, then two a turn.
        assert_eq!(turns_along(&route, 1, |_| 2, |_, _| false), vec![0, 1, 1, 2, 2, 3, 3]);
        // A unit that has already moved starts next turn.
        assert_eq!(turns_along(&route[..3], 0, |_| 2, |_, _| false), vec![1, 1]);
        // Boarding at step 2 ends that turn; at sea it gets three moves.
        let boards = |from: Hex, _: Hex| from.x == 1;
        let at_sea = |tile: Hex| if tile.x >= 2 { 3 } else { 2 };
        assert_eq!(turns_along(&route, 2, at_sea, boards), vec![0, 0, 1, 1, 1, 2, 2]);
        assert!(turns_along(&route[..1], 2, |_| 2, |_, _| false).is_empty());
    }
    use std::collections::HashSet;

    /// A strip: land at x = 0..=1, water at 2..=4, land at 5..=6.
    fn strait() -> Vec<(Hex, Terrain)> {
        (0..=6).map(|q| (Hex::new(q, 0), if (2..=4).contains(&q) { Terrain::Water } else { Terrain::Plains })).collect()
    }

    #[test]
    fn only_boats_and_aircraft_cross_water() {
        let (from, to) = (Hex::new(0, 0), Hex::new(6, 0));
        assert!(find_path(from, to, strait().into_iter(), Travel::Land).is_none());
        assert_eq!(find_path(from, to, strait().into_iter(), Travel::Amphibious).map(|p| p.len()), Some(7));
        assert_eq!(find_path(from, to, strait().into_iter(), Travel::Air).map(|p| p.len()), Some(7));
    }

    #[test]
    fn ships_keep_to_water_and_their_own_ports() {
        let port = Hex::new(1, 0);
        let ports: HashSet<Hex> = [port].into();
        // From port out to sea.
        assert!(find_path(port, Hex::new(4, 0), strait().into_iter(), Travel::Sea(&ports)).is_some());
        // The far shore can be targeted (to bombard it) but not sailed across.
        assert!(find_path(port, Hex::new(5, 0), strait().into_iter(), Travel::Sea(&ports)).is_some());
        assert!(find_path(port, Hex::new(6, 0), strait().into_iter(), Travel::Sea(&ports)).is_none());
        // Inland tiles behind the port are out of reach.
        assert!(find_path(Hex::new(3, 0), Hex::new(0, 0), strait().into_iter(), Travel::Sea(&HashSet::new())).is_none());
    }
}
