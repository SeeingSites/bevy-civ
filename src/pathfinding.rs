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

/// Find a path from start to end using A*, respecting terrain costs.
/// Returns the path (including start) or None if unreachable.
pub fn find_path(
    start: Hex,
    end: Hex,
    tiles: impl Iterator<Item = (Hex, Terrain)>,
) -> Option<Vec<Hex>> {
    let tile_map: std::collections::HashMap<Hex, Terrain> = tiles.collect();

    a_star(start, end, |_, to| {
        tile_map.get(&to).and_then(|t| terrain_cost(*t))
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
