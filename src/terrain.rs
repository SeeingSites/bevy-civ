use bevy::prelude::*;
use hexx::Hex;
use rand::Rng;

#[derive(Component)]
pub struct HexTile {
    pub coord: Hex,
    pub terrain: Terrain,
    pub improvements: Vec<Improvement>,
    pub resource: Option<Resource>,
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Improvement {
    Farm,
    Mine,
    Road,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Resource {
    // Strategic
    Horses,
    Iron,
    // Luxury
    Wine,
    Spices,
    Silk,
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

impl Improvement {
    pub fn food_bonus(&self) -> u32 {
        match self {
            Improvement::Farm => 1,
            _ => 0,
        }
    }

    pub fn production_bonus(&self) -> u32 {
        match self {
            Improvement::Mine => 1,
            _ => 0,
        }
    }

    pub fn movement_cost_mod(&self) -> u32 {
        match self {
            Improvement::Road => 0,
            _ => 1,
        }
    }
}

impl Resource {
    pub fn name(&self) -> &'static str {
        match self {
            Resource::Horses => "Horses",
            Resource::Iron => "Iron",
            Resource::Wine => "Wine",
            Resource::Spices => "Spices",
            Resource::Silk => "Silk",
        }
    }

    pub fn is_strategic(&self) -> bool {
        matches!(self, Resource::Horses | Resource::Iron)
    }

    pub fn is_luxury(&self) -> bool {
        matches!(self, Resource::Wine | Resource::Spices | Resource::Silk)
    }

    pub fn happiness_bonus(&self) -> u32 {
        if self.is_luxury() { 1 } else { 0 }
    }
}

impl HexTile {
    pub fn total_food(&self) -> u32 {
        self.terrain.food() + self.improvements.iter().map(|i| i.food_bonus()).sum::<u32>()
    }

    pub fn total_production(&self) -> u32 {
        self.terrain.production() + self.improvements.iter().map(|i| i.production_bonus()).sum::<u32>()
    }

    pub fn has_road(&self) -> bool {
        self.improvements.contains(&Improvement::Road)
    }

    pub fn add_improvement(&mut self, imp: Improvement) {
        if !self.improvements.contains(&imp) {
            self.improvements.push(imp);
        }
    }
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

pub fn random_resource(rng: &mut impl Rng, terrain: Terrain) -> Option<Resource> {
    if matches!(terrain, Terrain::Water) {
        return None;
    }
    if rng.gen_range(0..100) < 12 {
        Some(match rng.gen_range(0..5) {
            0 => Resource::Horses,
            1 => Resource::Iron,
            2 => Resource::Wine,
            3 => Resource::Spices,
            _ => Resource::Silk,
        })
    } else {
        None
    }
}
