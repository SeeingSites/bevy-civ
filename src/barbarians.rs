use bevy::prelude::*;
use hexx::Hex;
use crate::unit::{Unit, UnitType, spawn_unit_for_owner};
use crate::terrain::HexTile;
use rand::Rng;

#[derive(Resource, Default)]
pub struct BarbarianState {
    pub spawn_timer: u32,
}

pub fn process_barbarians(
    mut state: ResMut<BarbarianState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    layout: &hexx::HexLayout,
    tiles: &Query<&HexTile>,
    units: &Query<&Unit>,
) {
    state.spawn_timer += 1;
    if state.spawn_timer < 8 { return; }
    state.spawn_timer = 0;

    let mut rng = rand::thread_rng();
    // Spawn a barbarian warrior on random land tile far from units
    let land_tiles: Vec<_> = tiles.iter()
        .filter(|t| t.terrain != crate::terrain::Terrain::Water)
        .collect();
    if land_tiles.is_empty() { return; }

    let tile = land_tiles[rng.gen_range(0..land_tiles.len())];
    let nearby_units = units.iter().any(|u| u.position.distance_to(tile.coord) < 4);
    if !nearby_units {
        spawn_unit_for_owner(
            &mut commands,
            &mut meshes,
            &mut materials,
            layout,
            tile.coord,
            UnitType::Warrior,
            99, // barbarian owner
        );
        info!("Barbarians spawned at {:?}", tile.coord);
    }
}
