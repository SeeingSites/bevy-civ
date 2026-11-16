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
    /// Built over a road once Steam Power is known: movement along it is free.
    Railroad,
    /// Left by industry: the tile yields nothing until a worker cleans it up (a "build" of this
    /// removes it).
    Pollution,
}

/// What a resource is good for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResourceKind {
    /// Needed to build certain units and buildings.
    Strategic,
    /// Makes people content in every connected city.
    Luxury,
    /// Simply improves the yield of the city that owns the tile.
    Bonus,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Resource {
    Horses,
    Iron,
    Wine,
    Spices,
    Silk,
    Saltpeter,
    Coal,
    Oil,
    Rubber,
    Furs,
    Dyes,
    Incense,
    Ivory,
    Gems,
    Wheat,
    Cattle,
    Fish,
    Gold,
}

impl Terrain {
    pub fn name(&self) -> &'static str {
        match self {
            Terrain::Water => "Ocean",
            Terrain::Grassland => "Grassland",
            Terrain::Plains => "Plains",
            Terrain::Desert => "Desert",
            Terrain::Hills => "Hills",
            Terrain::Mountain => "Mountains",
            Terrain::Forest => "Forest",
        }
    }

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

    pub fn can_transform_to(&self) -> Option<Terrain> {
        match self {
            Terrain::Forest => Some(Terrain::Plains),
            Terrain::Desert => Some(Terrain::Plains),
            Terrain::Hills => Some(Terrain::Plains),
            _ => None,
        }
    }
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
            Improvement::Road | Improvement::Railroad => 0,
            _ => 1,
        }
    }
}

impl Resource {
    /// Every resource, in declaration order (also the frame order of the icon atlas).
    pub const ALL: [Resource; 18] = [
        Resource::Horses, Resource::Iron, Resource::Wine, Resource::Spices, Resource::Silk, Resource::Saltpeter,
        Resource::Coal, Resource::Oil, Resource::Rubber, Resource::Furs, Resource::Dyes, Resource::Incense,
        Resource::Ivory, Resource::Gems, Resource::Wheat, Resource::Cattle, Resource::Fish, Resource::Gold,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Resource::Horses => "Horses",
            Resource::Iron => "Iron",
            Resource::Wine => "Wine",
            Resource::Spices => "Spices",
            Resource::Silk => "Silk",
            Resource::Saltpeter => "Saltpeter",
            Resource::Coal => "Coal",
            Resource::Oil => "Oil",
            Resource::Rubber => "Rubber",
            Resource::Furs => "Furs",
            Resource::Dyes => "Dyes",
            Resource::Incense => "Incense",
            Resource::Ivory => "Ivory",
            Resource::Gems => "Gems",
            Resource::Wheat => "Wheat",
            Resource::Cattle => "Cattle",
            Resource::Fish => "Fish",
            Resource::Gold => "Gold",
        }
    }

    pub fn index(&self) -> usize {
        Resource::ALL.iter().position(|r| r == self).unwrap_or(0)
    }

    pub fn kind(&self) -> ResourceKind {
        match self {
            Resource::Horses | Resource::Iron | Resource::Saltpeter | Resource::Coal | Resource::Oil | Resource::Rubber => ResourceKind::Strategic,
            Resource::Wheat | Resource::Cattle | Resource::Fish | Resource::Gold => ResourceKind::Bonus,
            _ => ResourceKind::Luxury,
        }
    }

    pub fn is_strategic(&self) -> bool {
        self.kind() == ResourceKind::Strategic
    }

    pub fn is_luxury(&self) -> bool {
        self.kind() == ResourceKind::Luxury
    }

    pub fn happiness_bonus(&self) -> u32 {
        if self.is_luxury() { 1 } else { 0 }
    }

    /// Technology needed before the resource can be seen on the map and used.
    pub fn reveal_tech(&self) -> Option<crate::tech::Tech> {
        use crate::tech::Tech;
        match self {
            Resource::Horses => Some(Tech::AnimalHusbandry),
            Resource::Iron => Some(Tech::IronWorking),
            Resource::Saltpeter => Some(Tech::Gunpowder),
            Resource::Coal => Some(Tech::SteamPower),
            Resource::Oil => Some(Tech::Refining),
            Resource::Rubber => Some(Tech::ReplaceableParts),
            _ => None,
        }
    }

    /// Extra (food, shields, gold) a bonus resource gives the city whose land it lies in.
    pub fn bonus_yield(&self) -> (u32, u32, u32) {
        match self {
            Resource::Wheat => (2, 0, 0),
            Resource::Cattle => (1, 1, 0),
            Resource::Fish => (2, 0, 0),
            Resource::Gold => (0, 0, 3),
            _ => (0, 0, 0),
        }
    }
}

impl HexTile {
    pub fn total_food(&self) -> u32 {
        self.terrain.food() + self.improvements.iter().map(|i| i.food_bonus()).sum::<u32>()
    }

    pub fn total_production(&self) -> u32 {
        self.terrain.production() + self.improvements.iter().map(|i| i.production_bonus()).sum::<u32>()
    }

    /// A road or a railroad (a railroad is built over a road and works as one too).
    pub fn has_road(&self) -> bool {
        self.improvements.contains(&Improvement::Road) || self.has_railroad()
    }

    pub fn is_polluted(&self) -> bool {
        self.improvements.contains(&Improvement::Pollution)
    }

    pub fn has_railroad(&self) -> bool {
        self.improvements.contains(&Improvement::Railroad)
    }

    pub fn add_improvement(&mut self, imp: Improvement) {
        if !self.improvements.contains(&imp) {
            self.improvements.push(imp);
        }
    }

    pub fn transform_terrain(&mut self) -> bool {
        if let Some(new_terrain) = self.terrain.can_transform_to() {
            self.terrain = new_terrain;
            true
        } else {
            false
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

/// Scatters resources according to the terrain they are found in.
pub fn random_resource(rng: &mut impl Rng, terrain: Terrain) -> Option<Resource> {
    use crate::terrain::Resource::*;
    let (chance, options): (u32, &[Resource]) = match terrain {
        Terrain::Water => (5, &[Fish]),
        Terrain::Grassland => (13, &[Horses, Cattle, Wheat, Wine, Dyes, Cattle, Wheat]),
        Terrain::Plains => (13, &[Horses, Wheat, Ivory, Oil, Cattle, Wheat, Wine]),
        Terrain::Desert => (13, &[Saltpeter, Oil, Incense, Saltpeter, Oil, Gold]),
        Terrain::Forest => (14, &[Rubber, Furs, Silk, Spices, Dyes, Coal, Furs, Spices]),
        Terrain::Hills => (17, &[Iron, Coal, Saltpeter, Wine, Incense, Gems, Gold, Iron, Coal]),
        Terrain::Mountain => (17, &[Iron, Gold, Gems, Coal, Iron]),
    };
    (rng.gen_range(0..100) < chance).then(|| options[rng.gen_range(0..options.len())])
}
