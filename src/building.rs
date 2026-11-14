use bevy::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Building {
    Granary,
    Barracks,
    Library,
    Marketplace,
    Walls,
    Aqueduct,
    // Wonders
    Pyramids,
    GreatLibrary,
    HangingGardens,
}

impl Building {
    pub fn name(&self) -> &'static str {
        match self {
            Building::Granary => "Granary",
            Building::Barracks => "Barracks",
            Building::Library => "Library",
            Building::Marketplace => "Marketplace",
            Building::Walls => "Walls",
            Building::Aqueduct => "Aqueduct",
            Building::Pyramids => "Pyramids",
            Building::GreatLibrary => "Great Library",
            Building::HangingGardens => "Hanging Gardens",
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
            Building::Pyramids => 200,
            Building::GreatLibrary => 250,
            Building::HangingGardens => 180,
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
            Building::Pyramids => Some(crate::tech::Tech::Masonry),
            Building::GreatLibrary => Some(crate::tech::Tech::Writing),
            Building::HangingGardens => Some(crate::tech::Tech::Monarchy),
        }
    }

    /// Food bonus from this building
    pub fn food_bonus(&self) -> u32 {
        match self {
            Building::Granary => 2,
            Building::HangingGardens => 3,
            _ => 0,
        }
    }

    /// Science bonus
    pub fn science_bonus(&self) -> u32 {
        match self {
            Building::Library => 2,
            Building::GreatLibrary => 5,
            _ => 0,
        }
    }

    /// Production bonus
    pub fn production_bonus(&self) -> u32 {
        match self {
            Building::Barracks => 1,
            _ => 0,
        }
    }

    /// Gold / trade bonus
    pub fn gold_bonus(&self) -> u32 {
        match self {
            Building::Marketplace => 2,
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
}
