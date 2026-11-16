use bevy::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Building {
    Granary,
    Barracks,
    Library,
    Marketplace,
    Walls,
    Aqueduct,
    University,
    Bank,
    Factory,
    Harbor,
    // Wonders
    Pyramids,
    GreatLibrary,
    HangingGardens,
    Colossus,
    Oracle,
    GreatLighthouse,
}

impl Building {
    pub const ALL: [Building; 16] = [
        Building::Granary, Building::Barracks, Building::Library, Building::Marketplace, Building::Walls,
        Building::Aqueduct, Building::University, Building::Bank, Building::Factory, Building::Harbor, Building::Pyramids,
        Building::GreatLibrary, Building::HangingGardens, Building::Colossus, Building::Oracle,
        Building::GreatLighthouse,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Building::Granary => "Granary",
            Building::Barracks => "Barracks",
            Building::Library => "Library",
            Building::Marketplace => "Marketplace",
            Building::Walls => "Walls",
            Building::Aqueduct => "Aqueduct",
            Building::University => "University",
            Building::Bank => "Bank",
            Building::Factory => "Factory",
            Building::Harbor => "Harbor",
            Building::Pyramids => "Pyramids",
            Building::GreatLibrary => "Great Library",
            Building::HangingGardens => "Hanging Gardens",
            Building::Colossus => "Colossus",
            Building::Oracle => "Oracle",
            Building::GreatLighthouse => "Great Lighthouse",
        }
    }

    pub fn cost(&self) -> u32 {
        match self {
            Building::Granary => 40,
            Building::Barracks => 30,
            Building::Library => 50,
            Building::Marketplace => 60,
            Building::Walls => 40,
            Building::Aqueduct => 80,
            Building::University => 100,
            Building::Bank => 120,
            Building::Factory => 150,
            Building::Harbor => 60,
            Building::Pyramids => 200,
            Building::GreatLibrary => 250,
            Building::HangingGardens => 180,
            Building::Colossus => 200,
            Building::Oracle => 150,
            Building::GreatLighthouse => 180,
        }
    }

    pub fn required_tech(&self) -> Option<crate::tech::Tech> {
        match self {
            Building::Granary => Some(crate::tech::Tech::Pottery),
            Building::Barracks => None,
            Building::Library => Some(crate::tech::Tech::Writing),
            Building::Marketplace => Some(crate::tech::Tech::Currency),
            Building::Walls => Some(crate::tech::Tech::Masonry),
            Building::Aqueduct => Some(crate::tech::Tech::Construction),
            Building::University => Some(crate::tech::Tech::Philosophy),
            Building::Bank => Some(crate::tech::Tech::Currency),
            Building::Factory => Some(crate::tech::Tech::Industrialization),
            Building::Harbor => Some(crate::tech::Tech::Seafaring),
            Building::Pyramids => Some(crate::tech::Tech::Masonry),
            Building::GreatLibrary => Some(crate::tech::Tech::Writing),
            Building::HangingGardens => Some(crate::tech::Tech::Monarchy),
            Building::Colossus => Some(crate::tech::Tech::BronzeWorking),
            Building::Oracle => Some(crate::tech::Tech::Mysticism),
            Building::GreatLighthouse => Some(crate::tech::Tech::Masonry),
        }
    }

    /// Strategic resources a city must have access to before it can build this.
    pub fn required_resources(&self) -> &'static [crate::terrain::Resource] {
        use crate::terrain::Resource;
        match self {
            Building::Factory => &[Resource::Coal, Resource::Iron],
            Building::Colossus => &[Resource::Iron],
            _ => &[],
        }
    }

    pub fn is_wonder(&self) -> bool {
        matches!(self, Building::Pyramids | Building::GreatLibrary | Building::HangingGardens | Building::Colossus | Building::Oracle | Building::GreatLighthouse)
    }

    pub fn food_bonus(&self) -> u32 {
        match self {
            Building::Granary => 2,
            Building::Harbor => 1,
            Building::HangingGardens => 3,
            Building::Pyramids => 1, // half granary effect approximated
            _ => 0,
        }
    }

    pub fn science_bonus(&self) -> u32 {
        match self {
            Building::Library => 2,
            Building::University => 4,
            Building::GreatLibrary => 5,
            _ => 0,
        }
    }

    pub fn production_bonus(&self) -> u32 {
        match self {
            Building::Barracks => 1,
            Building::Factory => 3,
            _ => 0,
        }
    }

    pub fn gold_bonus(&self) -> u32 {
        match self {
            Building::Marketplace => 2,
            Building::Bank => 4,
            Building::Colossus => 3,
            _ => 0,
        }
    }
}

#[derive(Component)]
pub struct CityBuildings {
    pub buildings: Vec<Building>,
}

impl CityBuildings {
    pub fn has(&self, building: Building) -> bool {
        self.buildings.contains(&building)
    }

    pub fn total_food_bonus(&self) -> u32 {
        self.buildings.iter().map(|b| b.food_bonus()).sum()
    }

    pub fn total_science_bonus(&self) -> u32 {
        self.buildings.iter().map(|b| b.science_bonus()).sum()
    }

    pub fn total_production_bonus(&self) -> u32 {
        self.buildings.iter().map(|b| b.production_bonus()).sum()
    }

    pub fn total_gold_bonus(&self) -> u32 {
        self.buildings.iter().map(|b| b.gold_bonus()).sum()
    }

    pub fn has_wonder(&self, wonder: Building) -> bool {
        self.buildings.iter().any(|b| *b == wonder && b.is_wonder())
    }
}
