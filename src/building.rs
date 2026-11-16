use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
            Building::HangingGardens => 2,
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
            Building::Pyramids => 3,
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

impl Building {
    /// What a wonder does, in a few words (its yields in its own city, plus anything wider).
    pub fn wonder_effect(&self) -> &'static str {
        match self {
            Building::Pyramids => "+3 shields here",
            Building::GreatLibrary => "+5 science here",
            Building::HangingGardens => "+2 food here, +1 content in every city",
            Building::Colossus => "+3 gold here",
            Building::Oracle => "+2 content in every city",
            Building::GreatLighthouse => "+1 move for ships and troops at sea",
            _ => "",
        }
    }

    /// Content citizens this wonder adds to every city of its owner.
    pub fn empire_content(&self) -> u32 {
        match self {
            Building::Oracle => 2,
            Building::HangingGardens => 1,
            _ => 0,
        }
    }
}

/// Which wonders stand in the world, and where: each can be built only once.
#[derive(Resource, Default, Clone, Debug)]
pub struct WorldWonders {
    pub built: std::collections::HashMap<Building, (u32, String)>,
}

impl WorldWonders {
    pub fn is_taken(&self, wonder: Building) -> bool {
        self.built.contains_key(&wonder)
    }

    /// Whether `owner` has the wonder.
    pub fn owns(&self, owner: u32, wonder: Building) -> bool {
        self.built.get(&wonder).is_some_and(|(o, _)| *o == owner)
    }

    /// Content citizens every city of `owner` gets from its wonders.
    pub fn empire_content(&self, owner: u32) -> u32 {
        self.built.iter().filter(|(_, (o, _))| *o == owner).map(|(w, _)| w.empire_content()).sum()
    }

    /// Extra moves a ship (or troops at sea) of `owner` gets each turn.
    pub fn sea_moves(&self, owner: u32) -> u32 {
        u32::from(self.owns(owner, Building::GreatLighthouse))
    }

    /// Records a finished wonder. Returns false if someone else got there first.
    pub fn complete(&mut self, wonder: Building, owner: u32, city: &str) -> bool {
        if self.is_taken(wonder) {
            return false;
        }
        self.built.insert(wonder, (owner, city.to_string()));
        true
    }

    /// A captured city's wonders change hands with it.
    pub fn transfer_city(&mut self, city: &str, new_owner: u32) {
        for (owner, name) in self.built.values_mut() {
            if name == city {
                *owner = new_owner;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_wonder_is_built_once_and_its_effects_follow_the_owner() {
        let mut world = WorldWonders::default();
        assert!(world.complete(Building::Oracle, 1, "Delphi"));
        assert!(!world.complete(Building::Oracle, 2, "Carthage"));
        assert_eq!(world.empire_content(1), 2);
        assert_eq!(world.empire_content(2), 0);
        world.complete(Building::GreatLighthouse, 2, "Carthage");
        assert_eq!(world.sea_moves(2), 1);
        world.transfer_city("Carthage", 1);
        assert_eq!(world.sea_moves(1), 1);
        assert_eq!(world.sea_moves(2), 0);
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
