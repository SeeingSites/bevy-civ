use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    Temple,
    Courthouse,
    Colosseum,
    Cathedral,
    SewerSystem,
    StockExchange,
    Observatory,
    ResearchLab,
    PowerPlant,
    PoliceStation,
    Supermarket,
    /// Inspects the food trade: no adulterated-food scandals, and GM food is trusted.
    FoodSafety,
    /// Engineered crops: +1 food on every farmed tile worked.
    GmFarms,
    MassTransit,
    RecyclingCenter,
    CoastalFortress,
    Airport,
    SdiDefense,
    ForbiddenPalace,
    // Spaceship parts: each goes to its civilization's ship, not into the city
    SpaceStructure,
    SpaceComponent,
    SpaceModule,
    // Wonders
    Pyramids,
    GreatLibrary,
    HangingGardens,
    Colossus,
    Oracle,
    GreatLighthouse,
    GreatWall,
    StatueOfZeus,
    Mausoleum,
    Copernicus,
    SunTzu,
    MarcoPolo,
    KingRichards,
    Michelangelo,
    Leonardo,
    Bach,
    Magellan,
    Newton,
    AdamSmith,
    Shakespeare,
    Darwin,
    Liberty,
    HooverDam,
    Eiffel,
    Internet,
    Apollo,
    CureForCancer,
    Manhattan,
    UnitedNations,
    // Civilization III's later additions.
    /// +1 shield on every ocean tile the city works.
    OffshorePlatform,
    /// +25% shields (needs a Factory).
    ManufacturingPlant,
    /// A nuclear strike costs the city a quarter of its people, not half.
    CivilDefense,
    // National wonders: one in each people's empire.
    /// +50% shields in its city (it must reach Coal and Iron).
    IronWorks,
    /// +10 gold to the cap on the treasury's interest.
    WallStreet,
    /// Troops trained in its city one level higher.
    MilitaryAcademy,
    /// Its people's troops heal two hit points a turn in the field.
    BattlefieldMedicine,
    /// A world wonder: a Police Station's calm in every city - war weariness halved everywhere.
    WomensSuffrage,
    /// A national wonder: its people's armies hold one more unit.
    Pentagon,
    /// A national wonder: its people's diplomats succeed more often.
    IntelligenceAgency,
}

impl Building {
    pub const ALL: [Building; 71] = [
        Building::Granary, Building::Barracks, Building::Library, Building::Marketplace, Building::Walls,
        Building::Aqueduct, Building::University, Building::Bank, Building::Factory, Building::Harbor,
        Building::Temple, Building::Courthouse, Building::Colosseum, Building::Cathedral, Building::SewerSystem, Building::StockExchange,
        Building::Observatory, Building::ResearchLab, Building::PowerPlant, Building::PoliceStation, Building::Supermarket, Building::FoodSafety, Building::GmFarms, Building::MassTransit, Building::RecyclingCenter, Building::CoastalFortress, Building::Airport, Building::SdiDefense, Building::ForbiddenPalace,
        Building::SpaceStructure, Building::SpaceComponent, Building::SpaceModule, Building::Pyramids,
        Building::GreatLibrary, Building::HangingGardens, Building::Colossus, Building::Oracle,
        Building::GreatLighthouse, Building::GreatWall, Building::StatueOfZeus, Building::Mausoleum, Building::Copernicus, Building::SunTzu, Building::MarcoPolo, Building::KingRichards, Building::Michelangelo, Building::Leonardo, Building::Bach, Building::Magellan, Building::Newton, Building::AdamSmith, Building::Shakespeare, Building::Darwin, Building::Liberty, Building::HooverDam, Building::Eiffel, Building::Internet, Building::Apollo, Building::CureForCancer, Building::Manhattan, Building::UnitedNations,
        Building::OffshorePlatform, Building::ManufacturingPlant, Building::CivilDefense, Building::IronWorks, Building::WallStreet,
        Building::MilitaryAcademy, Building::BattlefieldMedicine, Building::WomensSuffrage, Building::Pentagon, Building::IntelligenceAgency,
    ];

    pub fn name(&self) -> &'static str {
        crate::theme::rename(crate::theme::Kind::Building, self, self.base_name())
    }

    /// The name in the default (Earth) theme.
    pub fn base_name(&self) -> &'static str {
        match self {
            Building::Granary => "Granary",
            Building::Barracks => "Barracks",
            Building::Library => "Library",
            Building::Marketplace => "Marketplace",
            Building::Walls => "Walls",
            Building::Aqueduct => "Aqueduct",
            Building::University => "University",
            Building::Bank => "Bank",
            Building::StockExchange => "Stock Exchange",
            Building::Observatory => "Observatory",
            Building::ResearchLab => "Research Lab",
            Building::PowerPlant => "Power Plant",
            Building::PoliceStation => "Police Station",
            Building::Supermarket => "Supermarket",
            Building::FoodSafety => "Food Safety Agency",
            Building::GmFarms => "GM Farms",
            Building::MassTransit => "Mass Transit",
            Building::RecyclingCenter => "Recycling Center",
            Building::CoastalFortress => "Coastal Fortress",
            Building::Airport => "Airport",
            Building::SdiDefense => "SDI Defense",
            Building::ForbiddenPalace => "Forbidden Palace",
            Building::Manhattan => "Manhattan Project",
            Building::UnitedNations => "United Nations",
            Building::Factory => "Factory",
            Building::Harbor => "Harbor",
            Building::Temple => "Temple",
            Building::Courthouse => "Courthouse",
            Building::Colosseum => "Colosseum",
            Building::Cathedral => "Cathedral",
            Building::SewerSystem => "Sewer System",
            Building::SpaceStructure => "Space Structure",
            Building::SpaceComponent => "Space Component",
            Building::SpaceModule => "Space Module",
            Building::Pyramids => "Pyramids",
            Building::GreatLibrary => "Great Library",
            Building::HangingGardens => "Hanging Gardens",
            Building::Colossus => "Colossus",
            Building::Oracle => "Oracle",
            Building::GreatLighthouse => "Great Lighthouse",
            Building::GreatWall => "Great Wall",
            Building::StatueOfZeus => "Statue of Zeus",
            Building::Mausoleum => "Mausoleum",
            Building::Copernicus => "Copernicus' Observatory",
            Building::SunTzu => "Sun Tzu's War Academy",
            Building::MarcoPolo => "Marco Polo's Embassy",
            Building::KingRichards => "King Richard's Crusade",
            Building::Michelangelo => "Michelangelo's Chapel",
            Building::Leonardo => "Leonardo's Workshop",
            Building::Bach => "J.S. Bach's Cathedral",
            Building::Magellan => "Magellan's Expedition",
            Building::Newton => "Isaac Newton's College",
            Building::AdamSmith => "Adam Smith's Trading Company",
            Building::Shakespeare => "Shakespeare's Theatre",
            Building::Darwin => "Darwin's Voyage",
            Building::Liberty => "Statue of Liberty",
            Building::HooverDam => "Hoover Dam",
            Building::Eiffel => "Eiffel Tower",
            Building::Internet => "The Internet",
            Building::Apollo => "Apollo Program",
            Building::CureForCancer => "Cure for Cancer",
            Building::OffshorePlatform => "Offshore Platform",
            Building::ManufacturingPlant => "Manufacturing Plant",
            Building::CivilDefense => "Civil Defense",
            Building::IronWorks => "Iron Works",
            Building::WallStreet => "Wall Street",
            Building::MilitaryAcademy => "Military Academy",
            Building::BattlefieldMedicine => "Battlefield Medicine",
            Building::WomensSuffrage => "Women's Suffrage",
            Building::Pentagon => "The Pentagon",
            Building::IntelligenceAgency => "Intelligence Agency",
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
            Building::StockExchange => 160,
            Building::Observatory => 80,
            Building::ResearchLab => 160,
            Building::PowerPlant => 160,
            Building::PoliceStation => 60,
            Building::Supermarket => 80,
            Building::FoodSafety => 70,
            Building::GmFarms => 110,
            Building::MassTransit => 120,
            Building::RecyclingCenter => 140,
            Building::CoastalFortress => 80,
            Building::Airport => 120,
            Building::SdiDefense => 160,
            Building::ForbiddenPalace => 150,
            Building::Manhattan => 500,
            Building::UnitedNations => 600,
            Building::Factory => 150,
            Building::Harbor => 60,
            Building::Temple => 30,
            Building::Courthouse => 50,
            Building::Colosseum => 70,
            Building::Cathedral => 100,
            Building::SewerSystem => 100,
            Building::SpaceStructure => 160,
            Building::SpaceComponent => 200,
            Building::SpaceModule => 240,
            Building::Pyramids => 200,
            Building::GreatLibrary => 250,
            Building::HangingGardens => 180,
            Building::Colossus => 200,
            Building::Oracle => 150,
            Building::GreatLighthouse => 180,
            Building::GreatWall => 160,
            Building::StatueOfZeus => 120,
            Building::Mausoleum => 140,
            Building::Copernicus => 200,
            Building::SunTzu => 180,
            Building::MarcoPolo => 180,
            Building::KingRichards => 200,
            Building::Michelangelo => 300,
            Building::Leonardo => 300,
            Building::Bach => 280,
            Building::Magellan => 260,
            Building::Newton => 320,
            Building::AdamSmith => 350,
            Building::Shakespeare => 300,
            Building::Darwin => 400,
            Building::Liberty => 300,
            Building::HooverDam => 450,
            Building::Eiffel => 400,
            Building::Internet => 600,
            Building::Apollo => 600,
            Building::CureForCancer => 700,
            Building::OffshorePlatform => 200,
            Building::ManufacturingPlant => 280,
            Building::CivilDefense => 120,
            Building::IronWorks => 300,
            Building::WallStreet => 400,
            Building::MilitaryAcademy => 300,
            Building::BattlefieldMedicine => 300,
            Building::WomensSuffrage => 600,
            Building::Pentagon => 400,
            Building::IntelligenceAgency => 300,
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
            Building::Bank => Some(crate::tech::Tech::Banking),
            Building::StockExchange => Some(crate::tech::Tech::Economics),
            Building::Observatory => Some(crate::tech::Tech::Physics),
            Building::ResearchLab => Some(crate::tech::Tech::Computers),
            Building::PowerPlant => Some(crate::tech::Tech::Electronics),
            Building::PoliceStation => Some(crate::tech::Tech::Sanitation),
            Building::Supermarket => Some(crate::tech::Tech::Refrigeration),
            Building::FoodSafety => Some(crate::tech::Tech::Industrialization),
            Building::GmFarms => Some(crate::tech::Tech::GeneticEngineering),
            Building::MassTransit => Some(crate::tech::Tech::MassProduction),
            Building::RecyclingCenter => Some(crate::tech::Tech::Ecology),
            Building::CoastalFortress => Some(crate::tech::Tech::Steel),
            Building::Airport => Some(crate::tech::Tech::Radio),
            Building::SdiDefense => Some(crate::tech::Tech::Superconductors),
            Building::ForbiddenPalace => Some(crate::tech::Tech::Monarchy),
            Building::Manhattan => Some(crate::tech::Tech::NuclearFission),
            Building::UnitedNations => Some(crate::tech::Tech::Computers),
            Building::Factory => Some(crate::tech::Tech::Industrialization),
            Building::Harbor => Some(crate::tech::Tech::Seafaring),
            Building::Temple => Some(crate::tech::Tech::CeremonialBurial),
            Building::Courthouse => Some(crate::tech::Tech::CodeOfLaws),
            Building::Colosseum => Some(crate::tech::Tech::Construction),
            Building::Cathedral => Some(crate::tech::Tech::Theology),
            Building::SewerSystem => Some(crate::tech::Tech::Sanitation),
            Building::SpaceStructure => Some(crate::tech::Tech::SpaceFlight),
            Building::SpaceComponent => Some(crate::tech::Tech::Plastics),
            Building::SpaceModule => Some(crate::tech::Tech::Superconductors),
            Building::Pyramids => Some(crate::tech::Tech::Mining),
            Building::GreatLibrary => Some(crate::tech::Tech::Literature),
            Building::HangingGardens => Some(crate::tech::Tech::Monarchy),
            Building::Colossus => Some(crate::tech::Tech::BronzeWorking),
            Building::Oracle => Some(crate::tech::Tech::Mysticism),
            Building::GreatLighthouse => Some(crate::tech::Tech::MapMaking),
            Building::GreatWall => Some(crate::tech::Tech::Construction),
            Building::StatueOfZeus => Some(crate::tech::Tech::Polytheism),
            Building::Mausoleum => Some(crate::tech::Tech::Masonry),
            Building::Copernicus => Some(crate::tech::Tech::Astronomy),
            Building::SunTzu => Some(crate::tech::Tech::Feudalism),
            Building::MarcoPolo => Some(crate::tech::Tech::Currency),
            Building::KingRichards => Some(crate::tech::Tech::Monarchy),
            Building::Michelangelo => Some(crate::tech::Tech::Monotheism),
            Building::Leonardo => Some(crate::tech::Tech::Invention),
            Building::Bach => Some(crate::tech::Tech::PrintingPress),
            Building::Magellan => Some(crate::tech::Tech::Navigation),
            Building::Newton => Some(crate::tech::Tech::TheoryOfGravity),
            Building::AdamSmith => Some(crate::tech::Tech::Economics),
            Building::Shakespeare => Some(crate::tech::Tech::Medicine),
            Building::Darwin => Some(crate::tech::Tech::Railroad),
            Building::Liberty => Some(crate::tech::Tech::Democracy),
            Building::HooverDam => Some(crate::tech::Tech::Electronics),
            Building::Eiffel => Some(crate::tech::Tech::Refining),
            Building::Internet => Some(crate::tech::Tech::Computers),
            Building::Apollo => Some(crate::tech::Tech::SpaceFlight),
            Building::CureForCancer => Some(crate::tech::Tech::Superconductors),
            Building::OffshorePlatform => Some(crate::tech::Tech::Miniaturization),
            Building::ManufacturingPlant => Some(crate::tech::Tech::Robotics),
            Building::CivilDefense => Some(crate::tech::Tech::Electronics),
            Building::IronWorks => Some(crate::tech::Tech::Industrialization),
            Building::WallStreet => Some(crate::tech::Tech::Economics),
            Building::MilitaryAcademy => Some(crate::tech::Tech::Gunpowder),
            Building::BattlefieldMedicine => Some(crate::tech::Tech::Sanitation),
            Building::WomensSuffrage => Some(crate::tech::Tech::Industrialization),
            Building::Pentagon => Some(crate::tech::Tech::Radio),
            Building::IntelligenceAgency => Some(crate::tech::Tech::Electronics),
        }
    }

    /// Strategic resources a city must have access to before it can build this.
    pub fn required_resources(&self) -> &'static [crate::terrain::Resource] {
        use crate::terrain::Resource;
        match self {
            Building::Factory => &[Resource::Coal, Resource::Iron],
            Building::Colossus => &[Resource::Iron],
            Building::IronWorks => &[Resource::Coal, Resource::Iron],
            // A starship is built of aluminium.
            Building::SpaceStructure | Building::SpaceComponent => &[Resource::Aluminum],
            _ => &[],
        }
    }

    /// A part for its civilization's spaceship: buildable again and again, and only in a city with a Factory.
    pub fn is_spaceship_part(&self) -> bool {
        matches!(self, Building::SpaceStructure | Building::SpaceComponent | Building::SpaceModule)
    }

    /// A world wonder that must stand somewhere in the world before anyone can build this: the
    /// Apollo Program opens the space race to every people.
    pub fn required_wonder(&self) -> Option<Building> {
        self.is_spaceship_part().then_some(Building::Apollo)
    }

    /// Citizens this building makes content in its city.
    pub fn content(&self) -> u32 {
        match self {
            Building::Temple => 2,
            Building::Colosseum | Building::Cathedral => 3,
            Building::PoliceStation => 2,
            _ => 0,
        }
    }

    /// The largest a city can grow without (and with) the buildings that supply water: 6 without an
    /// Aqueduct (a river beside the city serves as one), 12 without a Sewer System.
    pub fn size_limit(buildings: &[Building], fresh_water: bool) -> u32 {
        if buildings.contains(&Building::SewerSystem) {
            u32::MAX
        } else if buildings.contains(&Building::Aqueduct) || fresh_water {
            AQUEDUCT_LIMIT * 2
        } else {
            AQUEDUCT_LIMIT
        }
    }

    /// What a regular building does, in a few words (wonders use `wonder_effect`).
    pub fn effect(&self) -> &'static str {
        crate::theme::rename(crate::theme::Kind::Effect, self, self.base_effect())
    }

    /// What it does in the default (Earth) theme.
    pub fn base_effect(&self) -> &'static str {
        match self {
            Building::Granary => "the city keeps half its food when it grows",
            Building::Barracks => "+1 shields",
            Building::Library => "+50% science",
            Building::Marketplace => "+50% taxes and luxury; luxuries count double",
            Building::Walls => "doubles defence against land attacks",
            Building::Aqueduct => "lets the city grow past size 6 (to 12)",
            Building::University => "+50% science (needs a Library)",
            Building::Bank => "+30% taxes and luxury; interest on the treasury (needs a Marketplace)",
            Building::StockExchange => "+30% taxes and luxury; more interest (needs a Bank)",
            Building::Observatory => "+25% science",
            Building::ResearchLab => "+50% science (needs a University)",
            Building::PowerPlant => "+25% shields (needs a Factory)",
            Building::PoliceStation => "+2 content; halves war weariness; cuts corruption; stops narcotics and roots them out",
            Building::FoodSafety => "no adulterated-food scandals; GM food is trusted",
            Building::GmFarms => "+1 food on every farmed tile worked (distrusted without a Food Safety Agency)",
            Building::Supermarket => "+3 food",
            Building::MassTransit => "the city no longer pollutes",
            Building::RecyclingCenter => "halves the city's pollution",
            Building::CoastalFortress => "doubles defence against ships (coastal cities)",
            Building::Airport => "aircraft trained here are veterans",
            Building::SdiDefense => "shoots down nuclear missiles aimed within 3 tiles",
            Building::ForbiddenPalace => "a second seat of government: corruption counts from here too (one per people)",
            Building::Factory => "+25% shields; needed for spaceship parts",
            Building::Harbor => "+1 food on worked ocean tiles; links coasts",
            Building::Temple => "+2 content",
            Building::Courthouse => "cuts corruption and waste (halves the part due to distance)",
            Building::Colosseum => "+3 content",
            Building::Cathedral => "+3 content",
            Building::SewerSystem => "lets the city grow past size 12",
            Building::SpaceStructure => "spaceship part (4 needed)",
            Building::SpaceComponent => "spaceship part (2 needed)",
            Building::SpaceModule => "spaceship part (2 needed)",
            Building::OffshorePlatform => "+1 shield on worked ocean tiles (coastal cities)",
            Building::ManufacturingPlant => "+25% shields (needs a Factory)",
            Building::CivilDefense => "a nuclear strike costs the city a quarter of its people, not half",
            Building::IronWorks => "+50% shields here (needs Coal and Iron; one per people)",
            Building::WallStreet => "+10 to the cap on the treasury's interest (needs a Bank; one per people)",
            Building::MilitaryAcademy => "troops trained here one level higher (one per people)",
            Building::BattlefieldMedicine => "your troops heal two hit points a turn in the field (one per people)",
            Building::Pentagon => "your armies hold one more unit (one per people)",
            Building::IntelligenceAgency => "your diplomats' missions succeed more often (one per people)",
            _ => self.wonder_effect(),
        }
    }

    /// The wonders added after the first six (each with its own effect, mostly handled in the game's turn).
    fn is_later_wonder(&self) -> bool {
        matches!(self, Building::GreatWall | Building::StatueOfZeus | Building::Mausoleum | Building::Copernicus | Building::SunTzu | Building::MarcoPolo | Building::KingRichards | Building::Michelangelo | Building::Leonardo | Building::Bach | Building::Magellan | Building::Newton | Building::AdamSmith | Building::Shakespeare | Building::Darwin | Building::Liberty | Building::HooverDam | Building::Eiffel | Building::Internet | Building::Apollo | Building::CureForCancer | Building::Manhattan | Building::UnitedNations | Building::WomensSuffrage)
    }

    pub fn is_wonder(&self) -> bool {
        matches!(self, Building::Pyramids | Building::GreatLibrary | Building::HangingGardens | Building::Colossus | Building::Oracle | Building::GreatLighthouse) || self.is_later_wonder()
    }

    pub fn food_bonus(&self) -> u32 {
        match self {
            Building::HangingGardens => 2,
            Building::Supermarket => 3,
            _ => 0,
        }
    }

    /// Science a wonder adds outright in its city (the Library and its like multiply science instead:
    /// see `economy::science_bonus`).
    pub fn science_bonus(&self) -> u32 {
        match self {
            _ => 0,
        }
    }

    pub fn production_bonus(&self) -> u32 {
        match self {
            Building::Barracks => 1,
            Building::KingRichards => 4,
            _ => 0,
        }
    }

    /// Gold a wonder adds outright in its city (markets and banks multiply taxes instead: see
    /// `economy::tax_bonus`).
    pub fn gold_bonus(&self) -> u32 {
        match self {
            _ => 0,
        }
    }
}

/// The largest a city grows without an Aqueduct or fresh water; twice that without a Sewer System.
pub const AQUEDUCT_LIMIT: u32 = 6;

impl Building {
    /// Only one in each people's empire.
    pub fn one_per_people(&self) -> bool {
        matches!(
            self,
            Building::ForbiddenPalace | Building::IronWorks | Building::WallStreet | Building::MilitaryAcademy | Building::BattlefieldMedicine | Building::Pentagon | Building::IntelligenceAgency
        )
    }

    /// Only a city on the coast can build it (the seafaring wonders too, as in Civ III).
    pub fn needs_coast(&self) -> bool {
        matches!(self, Building::Harbor | Building::CoastalFortress | Building::Colossus | Building::GreatLighthouse | Building::Magellan | Building::OffshorePlatform)
    }

    /// A building the city must already have before it can build this one.
    pub fn required_building(&self) -> Option<Building> {
        match self {
            Building::Bank => Some(Building::Marketplace),
            Building::University => Some(Building::Library),
            Building::StockExchange => Some(Building::Bank),
            Building::ResearchLab => Some(Building::University),
            Building::PowerPlant => Some(Building::Factory),
            Building::GmFarms => Some(Building::Granary),
            Building::ManufacturingPlant => Some(Building::Factory),
            Building::WallStreet => Some(Building::Bank),
            _ => None,
        }
    }
}

/// Interest a civilization earns on its treasury each turn: 5% of its gold, up to 2 gold for each
/// Bank and 3 more for each Stock Exchange it owns.
pub fn interest(gold: i32, banks: u32, exchanges: u32) -> i32 {
    let cap = (2 * banks + 3 * exchanges) as i32;
    (gold.max(0) / 20).min(cap)
}

impl Building {
    /// What a wonder does, in a few words (its yields in its own city, plus anything wider).
    pub fn wonder_effect(&self) -> &'static str {
        match self {
            Building::Pyramids => "a Granary in every city of its owner (+2 food where there is none)",
            Building::GreatLibrary => "teaches its owner every technology known to two or more of the peoples it has met (until it learns Theory of Gravity)",
            Building::HangingGardens => "+2 food here, +1 content in every city",
            Building::Colossus => "+1 gold on every ocean tile its city works",
            Building::Oracle => "+2 content in every city",
            Building::GreatLighthouse => "+1 move for ships and troops at sea",
            Building::GreatWall => "every city of its owner counts as walled",
            Building::StatueOfZeus => "one more unit per city supported free",
            Building::Mausoleum => "temples and walls each calm one more citizen",
            Building::Copernicus => "+100% science here",
            Building::SunTzu => "new troops trained as veterans everywhere; armies hold a fifth unit",
            Building::MarcoPolo => "an embassy with every civilization",
            Building::KingRichards => "+4 shields here",
            Building::Michelangelo => "+3 content in every city",
            Building::Leonardo => "upgrades one obsolete unit free every turn",
            Building::Bach => "+2 content in every city",
            Building::Magellan => "+2 moves for ships",
            Building::Newton => "+100% science here",
            Building::AdamSmith => "+1 gold for every Marketplace, Bank and Harbor",
            Building::Shakespeare => "no unhappy citizens here",
            Building::Darwin => "two technologies at once",
            Building::Liberty => "revolutions without anarchy",
            Building::HooverDam => "+25% shields in every city",
            Building::Eiffel => "+2 culture a turn in every city",
            Building::Internet => "+3 science in every city with a Library",
            Building::Apollo => "the whole world revealed to its owner; opens the space race: no people can build spaceship parts until it stands",
            Building::CureForCancer => "+2 content in every city",
            Building::Manhattan => "lets every people who knows Rocketry build Nuclear Missiles",
            Building::UnitedNations => "every 10 turns the world votes for Secretary-General: a majority of all the members wins a diplomatic victory",
            Building::WomensSuffrage => "war weariness halved in every city, as by a Police Station",
            _ => "",
        }
    }

    /// Content citizens this wonder adds to every city of its owner.
    pub fn empire_content(&self) -> u32 {
        match self {
            Building::Oracle => 2,
            Building::HangingGardens => 1,
            Building::Michelangelo => 3,
            Building::Bach | Building::CureForCancer => 2,
            _ => 0,
        }
    }
}

impl Building {
    /// The technology that makes a wonder obsolete once its owner knows it, as in Civ III: its
    /// effect ends, though its culture goes on.
    pub fn obsoleted_by(&self) -> Option<crate::tech::Tech> {
        use crate::tech::Tech;
        match self {
            Building::Oracle => Some(Tech::Theology),
            Building::GreatWall => Some(Tech::Gunpowder),
            Building::GreatLighthouse => Some(Tech::Navigation),
            Building::GreatLibrary => Some(Tech::TheoryOfGravity),
            Building::Colossus => Some(Tech::Flight),
            Building::Leonardo => Some(Tech::MotorizedTransport),
            _ => None,
        }
    }
}

#[cfg(test)]
mod civ3_building_tests {
    use super::*;

    #[test]
    fn civ_threes_later_works_are_here() {
        for b in [Building::IronWorks, Building::WallStreet, Building::MilitaryAcademy, Building::BattlefieldMedicine, Building::ForbiddenPalace] {
            assert!(b.one_per_people() && !b.is_wonder(), "{b:?} is a national wonder");
        }
        assert!(Building::WomensSuffrage.is_wonder());
        assert!(Building::OffshorePlatform.needs_coast());
        assert_eq!(Building::ManufacturingPlant.required_building(), Some(Building::Factory));
        assert_eq!(Building::IronWorks.required_resources(), &[crate::terrain::Resource::Coal, crate::terrain::Resource::Iron]);
        assert_eq!(crate::economy::production_bonus(&[Building::Factory, Building::IronWorks, Building::ManufacturingPlant]), 100);
        // Every building has a name, a cost and a technology.
        for b in Building::ALL {
            assert!(!b.base_name().is_empty() && b.cost() > 0, "{b:?}");
            assert!(b.required_tech().is_some() || b == Building::Barracks, "{b:?}");
        }
    }
}

/// Which wonders stand in the world, and where: each can be built only once.
#[derive(Resource, Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorldWonders {
    pub built: std::collections::HashMap<Building, (u32, String)>,
    /// Wonders made obsolete: their owner knows what supersedes them (see
    /// [`Building::obsoleted_by`]), so their effects have ended.
    #[serde(default)]
    pub expired: std::collections::HashSet<Building>,
}

impl WorldWonders {
    pub fn is_taken(&self, wonder: Building) -> bool {
        self.built.contains_key(&wonder)
    }

    /// Whether `owner` has the wonder and it is still in effect (not obsolete).
    pub fn owns(&self, owner: u32, wonder: Building) -> bool {
        self.built.get(&wonder).is_some_and(|(o, _)| *o == owner) && !self.expired.contains(&wonder)
    }

    /// Content citizens every city of `owner` gets from its wonders still in effect.
    pub fn empire_content(&self, owner: u32) -> u32 {
        self.built.iter().filter(|(w, (o, _))| *o == owner && !self.expired.contains(*w)).map(|(w, _)| w.empire_content()).sum()
    }

    /// Notes which wonders their owners have made obsolete (`knows(owner, tech)`); a wonder taken by
    /// a people that does not know the technology works again. Returns those newly obsolete, with
    /// their owners.
    pub fn refresh_expired(&mut self, knows: impl Fn(u32, crate::tech::Tech) -> bool) -> Vec<(Building, u32)> {
        let mut fresh = Vec::new();
        for (wonder, (owner, _)) in &self.built {
            let gone = wonder.obsoleted_by().is_some_and(|t| knows(*owner, t));
            if gone && self.expired.insert(*wonder) {
                fresh.push((*wonder, *owner));
            } else if !gone {
                self.expired.remove(wonder);
            }
        }
        fresh
    }

    /// Extra moves a ship (or troops at sea) of `owner` gets each turn.
    pub fn sea_moves(&self, owner: u32) -> u32 {
        u32::from(self.owns(owner, Building::GreatLighthouse)) + 2 * u32::from(self.owns(owner, Building::Magellan))
    }

    /// Whether `owner`'s cities all count as walled (the Great Wall).
    pub fn walls_everywhere(&self, owner: u32) -> bool {
        self.owns(owner, Building::GreatWall)
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
        // Navigation makes the Lighthouse obsolete for its owner; a new owner without it uses it again.
        let fresh = world.refresh_expired(|owner, tech| owner == 1 && tech == crate::tech::Tech::Navigation);
        assert_eq!(fresh, vec![(Building::GreatLighthouse, 1)]);
        assert_eq!(world.sea_moves(1), 0);
        assert!(world.is_taken(Building::GreatLighthouse), "it still stands");
        world.transfer_city("Carthage", 2);
        assert!(world.refresh_expired(|owner, tech| owner == 1 && tech == crate::tech::Tech::Navigation).is_empty());
        assert_eq!(world.sea_moves(2), 1);
        // Theology ends the Oracle's calm.
        world.refresh_expired(|_, tech| tech == crate::tech::Tech::Theology);
        assert_eq!(world.empire_content(1), 0);
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

    /// Gold the city's wonders add outright.
    pub fn total_gold_bonus(&self) -> u32 {
        self.buildings.iter().map(|b| b.gold_bonus()).sum()
    }

    pub fn has_wonder(&self, wonder: Building) -> bool {
        self.buildings.iter().any(|b| *b == wonder && b.is_wonder())
    }
}

#[cfg(test)]
mod finance_tests {
    use super::*;

    #[test]
    fn interest_is_five_percent_capped_by_banks_and_exchanges() {
        assert_eq!(interest(1_000, 0, 0), 0, "no banks, no interest");
        assert_eq!(interest(1_000, 3, 0), 6);
        assert_eq!(interest(100, 3, 0), 5);
        assert_eq!(interest(1_000, 3, 2), 12);
        assert_eq!(interest(-50, 3, 2), 0);
    }

    #[test]
    fn finance_buildings_come_in_order() {
        assert_eq!(Building::StockExchange.required_building(), Some(Building::Bank));
        assert_eq!(Building::University.required_building(), Some(Building::Library));
        assert_eq!(Building::Bank.required_building(), Some(Building::Marketplace));
        assert_eq!(Building::Bank.required_tech(), Some(crate::tech::Tech::Banking));
    }
}
