use bevy::prelude::*;
use hexx::Hex;
use rand::Rng;

#[derive(Component)]
pub struct HexTile {
    pub coord: Hex,
    pub terrain: Terrain,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Terrain {
    Water,
    Grassland,
    Plains,
    Desert,
    Hills,
    Mountain,
    Forest,
}

impl Terrain {
    pub fn color(&self) -> Color {
        match self {
            Terrain::Water => Color::srgb(0.15, 0.35, 0.75),
            Terrain::Grassland => Color::srgb(0.25, 0.65, 0.25),
            Terrain::Plains => Color::srgb(0.7, 0.65, 0.35),
            Terrain::Desert => Color::srgb(0.85, 0.75, 0.4),
            Terrain::Hills => Color::srgb(0.55, 0.45, 0.25),
            Terrain::Mountain => Color::srgb(0.45, 0.45, 0.45),
            Terrain::Forest => Color::srgb(0.15, 0.45, 0.2),
        }
    }

    pub fn food(&self) -> u32 { match self { Terrain::Grassland => 2, Terrain::Plains => 1, Terrain::Forest => 1, _ => 0 } }
    pub fn production(&self) -> u32 { match self { Terrain::Hills => 2, Terrain::Mountain => 1, Terrain::Forest => 1, Terrain::Plains => 1, _ => 0 } }
}

pub fn random_terrain(rng: &mut impl Rng) -> Terrain {
    match rng.gen_range(0..100) {
        0..=15 => Terrain::Water,
        16..=40 => Terrain::Grassland,
        41..=60 => Terrain::Plains,
        61..=75 => Terrain::Forest,
        76..=88 => Terrain::Hills,
        89..=95 => Terrain::Desert,
        _ => Terrain::Mountain,
    }
}
