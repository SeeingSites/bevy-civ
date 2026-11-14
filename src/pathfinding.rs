use hexx::{algorithms::a_star, Hex};
use crate::terrain::{HexTile, Terrain};
use bevy::prelude::*;

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
    tiles: &Query<&HexTile>,
) -> Option<Vec<Hex>> {
    let tile_map: std::collections::HashMap<Hex, Terrain> = tiles
        .iter()
        .map(|t| (t.coord, t.terrain))
        .collect();

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

/// Marker for temporary path visualization entities.
#[derive(Component)]
pub struct PathVisual;

/// Spawn visual markers along a path (small yellow circles).
pub fn visualize_path(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    layout: &hexx::HexLayout,
    path: &[Hex],
) {
    let mesh = meshes.add(Circle::new(6.0));
    let material = materials.add(Color::srgba(1.0, 0.9, 0.1, 0.7));

    for (i, &hex) in path.iter().enumerate() {
        // Skip the starting tile slightly or mark start differently
        let color = if i == 0 {
            Color::srgba(0.2, 0.8, 1.0, 0.6) // cyan for start
        } else if i == path.len() - 1 {
            Color::srgba(1.0, 0.3, 0.3, 0.8) // red for destination
        } else {
            Color::srgba(1.0, 0.9, 0.1, 0.7) // yellow for path
        };

        let pos = layout.hex_to_world_pos(hex);
        commands.spawn((
            PathVisual,
            Mesh2d(mesh.clone()),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(pos.x, pos.y, 2.0),
        ));
    }
}

/// Clear all previous path visuals.
pub fn clear_path_visuals(commands: &mut Commands, visuals: &Query<Entity, With<PathVisual>>) {
    for entity in visuals.iter() {
        commands.entity(entity).despawn();
    }
}
