use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct TechState {
    pub researched: HashSet<Tech>,
    pub current: Option<Tech>,
    pub progress: u32,
    pub science_per_turn: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum Tech {
    // Ancient
    Pottery,
    AnimalHusbandry,
    Mining,
    BronzeWorking,
    Masonry,
    Alphabet,
    TheWheel,
    Writing,
    CodeOfLaws,
    Mysticism,
    Seafaring,
    // Classical / Medieval
    IronWorking,
    Mathematics,
    HorsebackRiding,
    Construction,
    Philosophy,
    Monarchy,
    Currency,
    Engineering,
    // Later placeholders
    Gunpowder,
    PrintingPress,
    Democracy,
    // Industrial
    SteamPower,
    Industrialization,
    Refining,
    ReplaceableParts,
    MotorizedTransport,
    // Governments and faith
    TheRepublic,
    Theology,
    Fundamentalism,
    Communism,
    // Modern
    Sanitation,
    Electricity,
    Computers,
    Rocketry,
    SpaceFlight,
    Plastics,
    Superconductors,
    // Finance
    Banking,
    Economics,
    Astronomy,
    Chivalry,
    Navigation,
    TheoryOfGravity,
    Medicine,
    Steel,
    Electronics,
    Flight,
    AdvancedFlight,
    MassProduction,
    Ecology,
    NuclearFission,
    /// Engineered crops: GM Farms.
    GeneticEngineering,
}

impl Tech {
    pub const ALL: [Tech; 53] = [
        Tech::Pottery, Tech::AnimalHusbandry, Tech::Mining, Tech::BronzeWorking, Tech::Masonry,
        Tech::Alphabet, Tech::TheWheel, Tech::Writing, Tech::CodeOfLaws, Tech::Mysticism, Tech::Seafaring,
        Tech::IronWorking, Tech::Mathematics, Tech::HorsebackRiding, Tech::Construction,
        Tech::Philosophy, Tech::Monarchy, Tech::Currency, Tech::Engineering, Tech::Gunpowder,
        Tech::PrintingPress, Tech::Democracy, Tech::SteamPower, Tech::Industrialization, Tech::Refining,
        Tech::ReplaceableParts, Tech::MotorizedTransport,
        Tech::TheRepublic, Tech::Theology, Tech::Fundamentalism, Tech::Communism, Tech::Sanitation,
        Tech::Electricity, Tech::Computers, Tech::Rocketry, Tech::SpaceFlight, Tech::Plastics, Tech::Superconductors,
        Tech::Banking, Tech::Economics,
        Tech::Astronomy, Tech::Chivalry, Tech::Navigation, Tech::TheoryOfGravity, Tech::Medicine, Tech::Steel, Tech::Electronics, Tech::Flight, Tech::AdvancedFlight, Tech::MassProduction, Tech::Ecology, Tech::NuclearFission,
        Tech::GeneticEngineering,
    ];

    pub fn cost(&self) -> u32 {
        match self {
            Tech::Pottery | Tech::AnimalHusbandry | Tech::Mining => 10,
            Tech::BronzeWorking | Tech::Masonry | Tech::Alphabet | Tech::TheWheel => 15,
            Tech::Writing | Tech::CodeOfLaws | Tech::Mysticism | Tech::Seafaring => 20,
            Tech::IronWorking | Tech::Mathematics | Tech::HorsebackRiding => 25,
            Tech::Construction | Tech::Philosophy | Tech::Monarchy | Tech::Currency => 30,
            Tech::Engineering => 40,
            Tech::Gunpowder | Tech::PrintingPress => 50,
            Tech::Democracy => 60,
            Tech::SteamPower => 60,
            Tech::Industrialization => 70,
            Tech::Refining | Tech::ReplaceableParts => 80,
            Tech::MotorizedTransport => 90,
            Tech::TheRepublic => 35,
            Tech::Theology => 45,
            Tech::Fundamentalism => 70,
            Tech::Communism => 85,
            Tech::Sanitation => 95,
            Tech::Electricity => 85,
            Tech::Computers => 110,
            Tech::Rocketry => 120,
            Tech::SpaceFlight => 140,
            Tech::Plastics => 150,
            Tech::Superconductors => 160,
            Tech::Banking => 45,
            Tech::Economics => 75,
            Tech::Astronomy => 35,
            Tech::Chivalry => 35,
            Tech::Navigation => 45,
            Tech::TheoryOfGravity => 55,
            Tech::Medicine => 50,
            Tech::Steel => 90,
            Tech::Electronics => 95,
            Tech::Flight => 95,
            Tech::AdvancedFlight => 115,
            Tech::MassProduction => 110,
            Tech::Ecology => 130,
            Tech::NuclearFission => 115,
            Tech::GeneticEngineering => 120,
        }
    }

    pub fn prerequisites(&self) -> Vec<Tech> {
        match self {
            Tech::Writing => vec![Tech::Alphabet],
            Tech::CodeOfLaws => vec![Tech::Writing],
            Tech::Mysticism => vec![Tech::Pottery],
            Tech::Seafaring => vec![Tech::Pottery],
            Tech::IronWorking => vec![Tech::BronzeWorking],
            Tech::Mathematics => vec![Tech::Writing, Tech::Masonry],
            Tech::HorsebackRiding => vec![Tech::AnimalHusbandry],
            Tech::Construction => vec![Tech::Masonry],
            Tech::Philosophy => vec![Tech::Mysticism, Tech::Writing],
            Tech::Monarchy => vec![Tech::CodeOfLaws],
            Tech::Currency => vec![Tech::CodeOfLaws],
            Tech::Engineering => vec![Tech::Mathematics, Tech::Construction],
            Tech::Gunpowder => vec![Tech::IronWorking, Tech::Engineering],
            Tech::PrintingPress => vec![Tech::Writing],
            Tech::Democracy => vec![Tech::Philosophy, Tech::PrintingPress],
            Tech::SteamPower => vec![Tech::Gunpowder],
            Tech::Industrialization => vec![Tech::SteamPower, Tech::Currency],
            Tech::Refining => vec![Tech::Industrialization],
            Tech::ReplaceableParts => vec![Tech::Industrialization],
            Tech::MotorizedTransport => vec![Tech::Refining, Tech::ReplaceableParts],
            Tech::TheRepublic => vec![Tech::CodeOfLaws, Tech::Writing],
            Tech::Theology => vec![Tech::Philosophy, Tech::Monarchy],
            Tech::Fundamentalism => vec![Tech::Theology, Tech::Gunpowder],
            Tech::Communism => vec![Tech::Philosophy, Tech::Industrialization],
            Tech::Sanitation => vec![Tech::Engineering, Tech::Industrialization],
            Tech::Electricity => vec![Tech::SteamPower, Tech::Mathematics],
            Tech::Computers => vec![Tech::Electronics, Tech::ReplaceableParts],
            Tech::Rocketry => vec![Tech::Computers, Tech::AdvancedFlight],
            Tech::SpaceFlight => vec![Tech::Rocketry],
            Tech::Plastics => vec![Tech::SpaceFlight, Tech::Refining],
            Tech::Superconductors => vec![Tech::Plastics, Tech::Computers],
            Tech::Banking => vec![Tech::Currency, Tech::TheRepublic],
            Tech::Economics => vec![Tech::Banking, Tech::PrintingPress],
            Tech::Astronomy => vec![Tech::Mysticism, Tech::Mathematics],
            Tech::Chivalry => vec![Tech::Monarchy, Tech::HorsebackRiding],
            Tech::Navigation => vec![Tech::Seafaring, Tech::Astronomy],
            Tech::TheoryOfGravity => vec![Tech::Astronomy, Tech::PrintingPress],
            Tech::Medicine => vec![Tech::Philosophy, Tech::Currency],
            Tech::Steel => vec![Tech::Electricity, Tech::Industrialization],
            Tech::Electronics => vec![Tech::Electricity, Tech::Engineering],
            Tech::Flight => vec![Tech::Refining, Tech::TheoryOfGravity],
            Tech::AdvancedFlight => vec![Tech::Flight, Tech::Electronics],
            Tech::MassProduction => vec![Tech::MotorizedTransport, Tech::Economics],
            Tech::Ecology => vec![Tech::Computers, Tech::Sanitation],
            Tech::NuclearFission => vec![Tech::Electronics, Tech::MassProduction],
            Tech::GeneticEngineering => vec![Tech::Medicine, Tech::Computers],
            _ => vec![],
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Tech::Pottery => "Pottery",
            Tech::AnimalHusbandry => "Animal Husbandry",
            Tech::Mining => "Mining",
            Tech::BronzeWorking => "Bronze Working",
            Tech::Masonry => "Masonry",
            Tech::Alphabet => "Alphabet",
            Tech::TheWheel => "The Wheel",
            Tech::Writing => "Writing",
            Tech::CodeOfLaws => "Code of Laws",
            Tech::Mysticism => "Mysticism",
            Tech::Seafaring => "Seafaring",
            Tech::IronWorking => "Iron Working",
            Tech::Mathematics => "Mathematics",
            Tech::HorsebackRiding => "Horseback Riding",
            Tech::Construction => "Construction",
            Tech::Philosophy => "Philosophy",
            Tech::Monarchy => "Monarchy",
            Tech::Currency => "Currency",
            Tech::Engineering => "Engineering",
            Tech::Gunpowder => "Gunpowder",
            Tech::PrintingPress => "Printing Press",
            Tech::Democracy => "Democracy",
            Tech::SteamPower => "Steam Power",
            Tech::Industrialization => "Industrialization",
            Tech::Refining => "Refining",
            Tech::ReplaceableParts => "Replaceable Parts",
            Tech::MotorizedTransport => "Motorized Transport",
            Tech::TheRepublic => "The Republic",
            Tech::Theology => "Theology",
            Tech::Fundamentalism => "Fundamentalism",
            Tech::Communism => "Communism",
            Tech::Sanitation => "Sanitation",
            Tech::Electricity => "Electricity",
            Tech::Computers => "Computers",
            Tech::Rocketry => "Rocketry",
            Tech::SpaceFlight => "Space Flight",
            Tech::Plastics => "Plastics",
            Tech::Superconductors => "Superconductors",
            Tech::Banking => "Banking",
            Tech::Economics => "Economics",
            Tech::Astronomy => "Astronomy",
            Tech::Chivalry => "Chivalry",
            Tech::Navigation => "Navigation",
            Tech::TheoryOfGravity => "Theory of Gravity",
            Tech::Medicine => "Medicine",
            Tech::Steel => "Steel",
            Tech::Electronics => "Electronics",
            Tech::Flight => "Flight",
            Tech::AdvancedFlight => "Advanced Flight",
            Tech::MassProduction => "Mass Production",
            Tech::Ecology => "Ecology",
            Tech::NuclearFission => "Nuclear Fission",
            Tech::GeneticEngineering => "Genetic Engineering",
        }
    }
}

impl Default for TechState {
    fn default() -> Self {
        Self {
            researched: HashSet::new(),
            current: Some(Tech::Pottery),
            progress: 0,
            science_per_turn: 2,
        }
    }
}

impl TechState {
    /// A fresh state that already knows the named technologies.
    pub fn knowing(names: &[String]) -> Self {
        let mut state = Self { researched: HashSet::new(), current: None, progress: 0, science_per_turn: 2 };
        state.researched.extend(Tech::ALL.into_iter().filter(|t| names.iter().any(|n| n == t.name())));
        state.current = Tech::ALL.into_iter().find(|t| state.can_research(*t));
        state
    }

    /// Adds a technology gained by trade and moves research on if it was the current project.
    pub fn learn(&mut self, tech: Tech) {
        self.researched.insert(tech);
        if self.current == Some(tech) {
            self.progress = 0;
            self.current = Tech::ALL.into_iter().find(|t| self.can_research(*t));
        }
    }

    /// Research cost of a technology right now: each discovery makes the next ones dearer.
    /// Beakers to research `tech`: dearer the more is already known, and twice as dear from the
    /// Industrial era on, so the game's pace holds as cities grow.
    pub fn cost_of(&self, tech: Tech) -> u32 {
        let era = match tech.cost() {
            0..=45 => 1,
            _ => 2,
        };
        tech.cost() * era * (2 + self.researched.len() as u32) / 2
    }

    pub fn can_research(&self, tech: Tech) -> bool {
        !self.researched.contains(&tech)
            && tech.prerequisites().iter().all(|p| self.researched.contains(p))
    }

    /// A great scientist's discovery: the current research is completed at once. Returns what was learned.
    pub fn discover(&mut self) -> Option<Tech> {
        let current = self.current?;
        self.researched.insert(current);
        self.progress = 0;
        self.current = Tech::ALL.into_iter().find(|t| self.can_research(*t));
        Some(current)
    }

    pub fn process_turn(&mut self) -> Option<Tech> {
        let Some(current) = self.current else { return None };
        self.progress += self.science_per_turn;
        if self.progress >= self.cost_of(current) {
            self.researched.insert(current);
            self.progress = 0;
            // Auto-pick the next available technology; the player can change it.
            let next = Tech::ALL.into_iter().find(|t| self.can_research(*t));
            self.current = next;
            Some(current)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tree_tests {
    use super::*;

    #[test]
    fn every_prerequisite_comes_earlier_and_every_tech_is_reachable() {
        for tech in Tech::ALL {
            for pre in tech.prerequisites() {
                assert!(pre.cost() <= tech.cost(), "{} needs {} which costs more", tech.name(), pre.name());
            }
        }
        let mut known: HashSet<Tech> = HashSet::new();
        loop {
            let next: Vec<Tech> = Tech::ALL.into_iter().filter(|t| !known.contains(t) && t.prerequisites().iter().all(|p| known.contains(p))).collect();
            if next.is_empty() {
                break;
            }
            known.extend(next);
        }
        assert_eq!(known.len(), Tech::ALL.len());
        let names: HashSet<&str> = Tech::ALL.iter().map(|t| t.name()).collect();
        assert_eq!(names.len(), Tech::ALL.len(), "names are unique");
    }

    #[test]
    fn every_tech_after_the_alphabet_unlocks_something() {
        use crate::building::Building;
        use crate::government::Government;
        use crate::unit::UnitType;
        let idle: Vec<&str> = Tech::ALL
            .into_iter()
            .filter(|t| *t != Tech::Alphabet)
            .filter(|tech| {
                !(UnitType::ALL.iter().any(|u| u.required_tech() == Some(*tech))
                    || Building::ALL.iter().any(|b| b.required_tech() == Some(*tech))
                    || Government::CHOICES.iter().any(|g| g.required_tech() == Some(*tech))
                    || crate::terrain::Resource::ALL.iter().any(|r| r.reveal_tech() == Some(*tech)))
            })
            .map(|t| t.name())
            .collect();
        assert!(idle.is_empty(), "these unlock nothing: {idle:?}");
    }
}
