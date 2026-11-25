use bevy::prelude::*;
use hexx::Hex;
use rand::Rng;

#[derive(Component, Clone, serde::Serialize, serde::Deserialize)]
pub struct HexTile {
    pub coord: Hex,
    pub terrain: Terrain,
    pub improvements: Vec<Improvement>,
    pub resource: Option<Resource>,
}

#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Terrain {
    Water,
    Grassland,
    Plains,
    Desert,
    Hills,
    Mountain,
    Forest,
    /// Cold land near the poles (Civilization III): poor food, no farms.
    Tundra,
    /// Wet tropical forest: slow going and good cover; cleared, it becomes grassland.
    Jungle,
    /// Wetland: slow going; drained, it becomes grassland.
    Marsh,
}

#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Improvement {
    Farm,
    Mine,
    Road,
    /// Built over a road once Steam Power is known: movement along it is free.
    Railroad,
    /// Left by industry: the tile yields nothing until a worker cleans it up (a "build" of this
    /// removes it).
    Pollution,
    /// A fortress: troops in it defend as if behind city walls.
    Fortress,
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
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
    /// Civilization III's light metal of the modern age (Rocketry).
    Aluminum,
    /// The fuel of the atom (Nuclear Fission).
    Uranium,
    /// A bonus of the sea.
    Whales,
    /// A bonus of the wild: forest, jungle and tundra.
    Game,
}

impl Terrain {
    pub fn name(&self) -> &'static str {
        crate::theme::rename(crate::theme::Kind::Terrain, self, self.base_name())
    }

    /// The name in the default (Earth) theme.
    pub fn base_name(&self) -> &'static str {
        match self {
            Terrain::Water => "Ocean",
            Terrain::Grassland => "Grassland",
            Terrain::Plains => "Plains",
            Terrain::Desert => "Desert",
            Terrain::Hills => "Hills",
            Terrain::Mountain => "Mountains",
            Terrain::Forest => "Forest",
            Terrain::Tundra => "Tundra",
            Terrain::Jungle => "Jungle",
            Terrain::Marsh => "Marsh",
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
            Terrain::Tundra => Color::srgb(0.62, 0.66, 0.6),
            Terrain::Jungle => Color::srgb(0.1, 0.38, 0.14),
            Terrain::Marsh => Color::srgb(0.34, 0.46, 0.36),
        }
    }

    pub fn food(&self) -> u32 { match self { Terrain::Grassland => 2, Terrain::Plains | Terrain::Forest | Terrain::Tundra | Terrain::Jungle | Terrain::Marsh => 1, _ => 0 } }
    pub fn production(&self) -> u32 { match self { Terrain::Hills => 2, Terrain::Mountain => 1, Terrain::Forest => 1, Terrain::Plains => 1, _ => 0 } }

    /// Land that can be farmed (given fresh water: see `tiles::irrigable`).
    pub fn farmland(&self) -> bool {
        matches!(self, Terrain::Grassland | Terrain::Plains | Terrain::Desert | Terrain::Hills)
    }

    pub fn can_transform_to(&self) -> Option<Terrain> {
        match self {
            Terrain::Forest => Some(Terrain::Plains),
            Terrain::Desert => Some(Terrain::Plains),
            Terrain::Hills => Some(Terrain::Plains),
            Terrain::Jungle | Terrain::Marsh => Some(Terrain::Grassland),
            _ => None,
        }
    }
}

/// What a pillager wrecks first: the railroad, then a fortress, mine or farm, and the road last.
pub fn pillage_target(improvements: &[Improvement]) -> Option<Improvement> {
    [Improvement::Railroad, Improvement::Fortress, Improvement::Mine, Improvement::Farm, Improvement::Road].into_iter().find(|i| improvements.contains(i))
}

pub fn improvement_name(improvement: Improvement) -> &'static str {
    match improvement {
        Improvement::Farm => "farm",
        Improvement::Mine => "mine",
        Improvement::Road => "road",
        Improvement::Railroad => "railroad",
        Improvement::Pollution => "pollution",
        Improvement::Fortress => "fortress",
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
    pub const ALL: [Resource; 22] = [
        Resource::Horses, Resource::Iron, Resource::Wine, Resource::Spices, Resource::Silk, Resource::Saltpeter,
        Resource::Coal, Resource::Oil, Resource::Rubber, Resource::Furs, Resource::Dyes, Resource::Incense,
        Resource::Ivory, Resource::Gems, Resource::Wheat, Resource::Cattle, Resource::Fish, Resource::Gold,
        Resource::Aluminum, Resource::Uranium, Resource::Whales, Resource::Game,
    ];

    pub fn name(&self) -> &'static str {
        crate::theme::rename(crate::theme::Kind::Resource, self, self.base_name())
    }

    /// The name in the default (Earth) theme.
    pub fn base_name(&self) -> &'static str {
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
            Resource::Aluminum => "Aluminum",
            Resource::Uranium => "Uranium",
            Resource::Whales => "Whales",
            Resource::Game => "Game",
        }
    }

    pub fn index(&self) -> usize {
        Resource::ALL.iter().position(|r| r == self).unwrap_or(0)
    }

    pub fn kind(&self) -> ResourceKind {
        match self {
            Resource::Horses | Resource::Iron | Resource::Saltpeter | Resource::Coal | Resource::Oil | Resource::Rubber | Resource::Aluminum | Resource::Uranium => ResourceKind::Strategic,
            Resource::Wheat | Resource::Cattle | Resource::Fish | Resource::Gold | Resource::Whales | Resource::Game => ResourceKind::Bonus,
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
            Resource::Aluminum => Some(Tech::Rocketry),
            Resource::Uranium => Some(Tech::NuclearFission),
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
            Resource::Whales => (1, 1, 1),
            Resource::Game => (2, 0, 0),
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
        Terrain::Water => (6, &[Fish, Fish, Whales]),
        Terrain::Grassland => (13, &[Horses, Cattle, Wheat, Wine, Dyes, Cattle, Wheat]),
        Terrain::Plains => (13, &[Horses, Wheat, Ivory, Oil, Cattle, Wheat, Wine]),
        Terrain::Desert => (14, &[Saltpeter, Oil, Incense, Saltpeter, Oil, Gold, Uranium, Aluminum]),
        Terrain::Forest => (15, &[Rubber, Furs, Silk, Spices, Dyes, Coal, Furs, Spices, Game, Game]),
        Terrain::Hills => (18, &[Iron, Coal, Saltpeter, Wine, Incense, Gems, Gold, Iron, Coal, Aluminum]),
        Terrain::Mountain => (18, &[Iron, Gold, Gems, Coal, Iron, Uranium, Aluminum]),
        Terrain::Tundra => (16, &[Furs, Oil, Furs, Game, Uranium]),
        Terrain::Jungle => (17, &[Rubber, Gems, Spices, Ivory, Dyes, Silk, Game]),
        Terrain::Marsh => (12, &[Oil, Dyes, Spices]),
    };
    (rng.gen_range(0..100) < chance).then(|| options[rng.gen_range(0..options.len())])
}

#[cfg(test)]
mod pillage_tests {
    use super::*;

    #[test]
    fn civ_threes_resources_are_all_here() {
        assert_eq!(Resource::ALL.len(), 22);
        assert!(Resource::Aluminum.is_strategic() && Resource::Uranium.is_strategic());
        assert_eq!(Resource::Whales.kind(), ResourceKind::Bonus);
        assert_eq!(Resource::Game.bonus_yield(), (2, 0, 0));
        assert_eq!(Resource::Uranium.reveal_tech(), Some(crate::tech::Tech::NuclearFission));
        assert!(crate::unit::UnitType::NuclearMissile.required_resources().contains(&Resource::Uranium));
        assert!(crate::building::Building::SpaceStructure.required_resources().contains(&Resource::Aluminum));
        // Every resource has an icon frame in order.
        assert!(Resource::ALL.iter().enumerate().all(|(i, r)| r.index() == i));
    }

    #[test]
    fn pillagers_wreck_rails_and_works_before_roads() {
        assert_eq!(pillage_target(&[Improvement::Road, Improvement::Farm]), Some(Improvement::Farm));
        assert_eq!(pillage_target(&[Improvement::Road, Improvement::Railroad, Improvement::Mine]), Some(Improvement::Railroad));
        assert_eq!(pillage_target(&[Improvement::Road]), Some(Improvement::Road));
        assert_eq!(pillage_target(&[Improvement::Pollution]), None);
    }
}
