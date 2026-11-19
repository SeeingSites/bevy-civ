use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

/// The fewest and the most turns a discovery takes: beakers beyond the cost are lost while the
/// scholars finish their four turns, and after forty turns of study the discovery is made however
/// little science went into it.
pub const MIN_RESEARCH_TURNS: u32 = 4;
pub const MAX_RESEARCH_TURNS: u32 = 40;
/// Most a technology's cost falls (in percent) when every other people knows it; in between, the
/// cost falls with the share of the other peoples that know it and that we have met.
pub const MAX_DIFFUSION: u32 = 50;

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct TechState {
    pub researched: HashSet<Tech>,
    pub current: Option<Tech>,
    pub progress: u32,
    pub science_per_turn: u32,
    /// A technology the scholars are working towards: research follows the path to it.
    #[serde(default)]
    pub goal: Option<Tech>,
    /// Percent taken off each technology's cost because peoples we know already know it (see
    /// [`diffusion_discount`]); refreshed every turn.
    #[serde(default)]
    pub discount: HashMap<Tech, u32>,
    /// Turns spent on the current research so far.
    #[serde(default)]
    pub turns_on: u32,
    /// The latest era this people has been told it entered (None until first looked at).
    #[serde(default)]
    pub era_seen: Option<Era>,
}

/// The four ages of history. A people enters an age with its first discovery there, and may study
/// an age's technologies only once it knows every required (not [`Tech::optional`]) technology of
/// the ages before.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum Era {
    Ancient,
    Middle,
    Industrial,
    Modern,
}

impl Era {
    pub const ALL: [Era; 4] = [Era::Ancient, Era::Middle, Era::Industrial, Era::Modern];

    pub fn name(&self) -> &'static str {
        match self {
            Era::Ancient => "Ancient Times",
            Era::Middle => "Middle Ages",
            Era::Industrial => "Industrial Age",
            Era::Modern => "Modern Age",
        }
    }
}

/// Percent off a technology's cost when `knowers` of the `peoples` other than us know it.
pub fn diffusion_discount(knowers: u32, peoples: u32) -> u32 {
    if peoples == 0 {
        0
    } else {
        MAX_DIFFUSION * knowers.min(peoples) / peoples
    }
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

    /// The age a technology belongs to.
    pub fn era(&self) -> Era {
        use Tech::*;
        match self {
            Pottery | AnimalHusbandry | Mining | BronzeWorking | Masonry | Alphabet | TheWheel | Writing | CodeOfLaws | Mysticism | Seafaring | IronWorking
            | Mathematics | HorsebackRiding | Construction | Philosophy | Monarchy | Currency | TheRepublic => Era::Ancient,
            Engineering | Astronomy | Chivalry | Theology | Banking | Gunpowder | PrintingPress | Navigation | TheoryOfGravity | Democracy | Medicine
            | Economics => Era::Middle,
            SteamPower | Industrialization | Electricity | Refining | ReplaceableParts | Steel | Electronics | Sanitation | Communism | Fundamentalism
            | Flight | MotorizedTransport => Era::Industrial,
            Computers | Rocketry | SpaceFlight | Plastics | Superconductors | AdvancedFlight | MassProduction | Ecology | NuclearFission
            | GeneticEngineering => Era::Modern,
        }
    }

    /// Not needed to open the next age: the byways of horsemen and seafarers, knights and
    /// navigators, and the forms of government a people may pass by.
    pub fn optional(&self) -> bool {
        matches!(
            self,
            Tech::HorsebackRiding | Tech::Seafaring | Tech::TheRepublic | Tech::Chivalry | Tech::Democracy | Tech::Navigation | Tech::Communism | Tech::Fundamentalism
        )
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
        crate::theme::rename(crate::theme::Kind::Tech, self, self.base_name())
    }

    /// The name in the default (Earth) theme.
    pub fn base_name(&self) -> &'static str {
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
            goal: None,
            discount: HashMap::new(),
            turns_on: 0,
            era_seen: None,
        }
    }
}

impl TechState {
    /// A fresh state that already knows the named technologies.
    pub fn knowing(names: &[String]) -> Self {
        let mut state = Self { current: None, ..Self::default() };
        state.researched.extend(Tech::ALL.into_iter().filter(|t| names.iter().any(|n| n == t.base_name())));
        state.current = Tech::ALL.into_iter().find(|t| state.can_research(*t));
        state
    }

    /// Adds a technology gained by trade and moves research on if it was the current project.
    pub fn learn(&mut self, tech: Tech) {
        self.researched.insert(tech);
        if self.current == Some(tech) {
            self.progress = 0;
            self.turns_on = 0;
            self.current = self.next_pick();
        }
    }

    /// Research cost of a technology right now: each discovery makes the next ones dearer.
    /// Beakers to research `tech`: dearer the more is already known, and twice as dear from the
    /// Industrial era on, so the game's pace holds as cities grow - and cheaper the more of the
    /// peoples we know have it already (see [`diffusion_discount`]).
    pub fn cost_of(&self, tech: Tech) -> u32 {
        let era = match tech.cost() {
            0..=45 => 1,
            _ => 2,
        };
        let full = tech.cost() * era * (2 + self.researched.len() as u32) / 2;
        full * (100 - self.discount.get(&tech).copied().unwrap_or(0).min(MAX_DIFFUSION)) / 100
    }

    /// Sets each technology's discount from how many of the other `peoples` that we know have it.
    pub fn set_known_abroad(&mut self, knowers: impl Fn(Tech) -> u32, peoples: u32) {
        self.discount = Tech::ALL
            .into_iter()
            .filter(|t| !self.researched.contains(t))
            .map(|t| (t, diffusion_discount(knowers(t), peoples)))
            .filter(|(_, d)| *d > 0)
            .collect();
    }

    pub fn can_research(&self, tech: Tech) -> bool {
        !self.researched.contains(&tech)
            && tech.prerequisites().iter().all(|p| self.researched.contains(p))
            && self.era_open(tech.era())
    }

    /// The latest age this people has entered: the age of its most advanced discovery.
    pub fn era(&self) -> Era {
        self.researched.iter().map(|t| t.era()).max().unwrap_or(Era::Ancient)
    }

    /// Whether this people may study the technologies of `era`: it knows every required technology
    /// of the ages before it.
    pub fn era_open(&self, era: Era) -> bool {
        Tech::ALL.iter().all(|t| t.era() >= era || t.optional() || self.researched.contains(t))
    }

    /// Required technologies still missing before the next age opens (empty in the last age, or
    /// once it is open).
    pub fn missing_for_next_era(&self) -> Vec<Tech> {
        let Some(next) = Era::ALL.into_iter().find(|e| *e > self.era()) else { return Vec::new() };
        Tech::ALL.into_iter().filter(|t| t.era() < next && !t.optional() && !self.researched.contains(t)).collect()
    }

    /// The age this people has entered since it was last asked, if it entered one; the first look
    /// only takes note.
    pub fn enter_new_era(&mut self) -> Option<Era> {
        let now = self.era();
        match self.era_seen.replace(now) {
            Some(seen) if now > seen => Some(now),
            _ => None,
        }
    }

    /// Everything still to learn on the way to `goal`, prerequisites before what needs them (the
    /// goal itself last), including the required technologies of earlier ages that open the
    /// goal's age. Empty once the goal is known.
    pub fn path_to(&self, goal: Tech) -> Vec<Tech> {
        fn visit(state: &TechState, tech: Tech, path: &mut Vec<Tech>) {
            if state.researched.contains(&tech) || path.contains(&tech) {
                return;
            }
            for p in tech.prerequisites() {
                visit(state, p, path);
            }
            // The ages before must be finished first.
            for gate in Tech::ALL.into_iter().filter(|g| g.era() < tech.era() && !g.optional()) {
                visit(state, gate, path);
            }
            path.push(tech);
        }
        let mut path = Vec::new();
        visit(self, goal, &mut path);
        path
    }

    /// What to study next: the next open step towards the goal (the goal is dropped once reached),
    /// or else the first technology open to study.
    pub fn next_pick(&mut self) -> Option<Tech> {
        if self.goal.is_some_and(|g| self.researched.contains(&g)) {
            self.goal = None;
        }
        let toward = self.goal.and_then(|g| self.path_to(g).into_iter().find(|t| self.can_research(*t)));
        toward.or_else(|| Tech::ALL.into_iter().find(|t| self.can_research(*t)))
    }

    /// Works towards a goal: research turns to its first open step at once.
    pub fn set_goal(&mut self, goal: Tech) {
        if self.researched.contains(&goal) {
            return;
        }
        self.goal = Some(goal);
        if !self.current.is_some_and(|c| self.path_to(goal).contains(&c)) {
            self.current = self.next_pick();
        }
    }

    /// Beakers still needed for everything on the way to the goal (at today's costs).
    pub fn beakers_to(&self, goal: Tech) -> u32 {
        self.path_to(goal).iter().map(|t| self.cost_of(*t)).sum::<u32>().saturating_sub(self.progress)
    }

    /// A great scientist's discovery: the current research is completed at once. Returns what was learned.
    pub fn discover(&mut self) -> Option<Tech> {
        let current = self.current?;
        self.researched.insert(current);
        self.progress = 0;
        self.turns_on = 0;
        self.current = self.next_pick();
        Some(current)
    }

    /// Turns the current research still takes at today's science: never fewer than are left of
    /// [`MIN_RESEARCH_TURNS`], never more than are left of [`MAX_RESEARCH_TURNS`].
    pub fn turns_left(&self) -> Option<u32> {
        let current = self.current?;
        let left = self.cost_of(current).saturating_sub(self.progress);
        let by_beakers = if self.science_per_turn == 0 { u32::MAX } else { left.div_ceil(self.science_per_turn) };
        let least = MIN_RESEARCH_TURNS.saturating_sub(self.turns_on).max(1);
        let most = MAX_RESEARCH_TURNS.saturating_sub(self.turns_on).max(1);
        Some(by_beakers.max(least).min(most))
    }

    /// A turn of study. A discovery takes at least [`MIN_RESEARCH_TURNS`] turns and at most
    /// [`MAX_RESEARCH_TURNS`]; beakers beyond its cost are lost.
    pub fn process_turn(&mut self) -> Option<Tech> {
        let Some(current) = self.current else { return None };
        self.progress += self.science_per_turn;
        self.turns_on += 1;
        let paid = self.progress >= self.cost_of(current) && self.turns_on >= MIN_RESEARCH_TURNS;
        if paid || self.turns_on >= MAX_RESEARCH_TURNS {
            self.researched.insert(current);
            self.progress = 0;
            self.turns_on = 0;
            // On along the path to the goal, or else the next available technology.
            self.current = self.next_pick();
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
    fn research_follows_the_path_to_a_goal() {
        let mut state = TechState::knowing(&[]);
        let goal = Tech::Monarchy;
        let path = state.path_to(goal);
        assert_eq!(path.last(), Some(&goal));
        // Every step comes after what it needs.
        for (i, t) in path.iter().enumerate() {
            assert!(t.prerequisites().iter().all(|p| path[..i].contains(p)), "{t:?}");
        }
        state.set_goal(goal);
        assert!(path.contains(&state.current.unwrap()) && state.can_research(state.current.unwrap()));
        // Research walks the path and the goal is dropped once reached.
        state.science_per_turn = 10_000;
        while !state.researched.contains(&goal) {
            if let Some(learned) = state.process_turn() {
                assert!(path.contains(&learned), "{learned:?} is off the path");
            }
        }
        assert_eq!(state.next_pick().is_some(), true);
        assert_eq!(state.goal, None);
        assert_eq!(state.beakers_to(goal), 0);
    }

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
    fn an_age_opens_once_the_required_techs_of_the_ages_before_are_known() {
        let mut state = TechState::knowing(&[]);
        // Gunpowder's prerequisites alone are not enough: the Ancient age must be finished.
        state.researched.extend([Tech::BronzeWorking, Tech::IronWorking, Tech::Masonry, Tech::Construction, Tech::Alphabet, Tech::Writing, Tech::Mathematics, Tech::Engineering]);
        assert!(!state.can_research(Tech::Gunpowder));
        assert!(state.missing_for_next_era().contains(&Tech::Pottery));
        for t in Tech::ALL.into_iter().filter(|t| t.era() == Era::Ancient && !t.optional()) {
            state.researched.insert(t);
        }
        assert!(state.can_research(Tech::Gunpowder), "optional ancient techs are not needed");
        assert!(!state.can_research(Tech::SteamPower) && state.missing_for_next_era().is_empty() == false);
        // Every step of a path is open by the time it is reached, the gates included.
        let fresh = TechState::knowing(&[]);
        let path = fresh.path_to(Tech::SteamPower);
        let mut walk = TechState::knowing(&[]);
        for t in &path {
            assert!(walk.can_research(*t), "{t:?} is not open when the path reaches it");
            walk.researched.insert(*t);
        }
        assert!(path.contains(&Tech::Medicine) && !path.contains(&Tech::Chivalry), "required, not optional, middle-age techs open the Industrial Age");
    }

    #[test]
    fn studying_what_others_know_is_cheaper_and_takes_four_to_forty_turns() {
        let mut state = TechState::knowing(&[]);
        let full = state.cost_of(Tech::Pottery);
        assert_eq!(diffusion_discount(4, 4), MAX_DIFFUSION);
        assert_eq!(diffusion_discount(1, 4), MAX_DIFFUSION / 4);
        assert_eq!(diffusion_discount(3, 0), 0);
        state.set_known_abroad(|t| if t == Tech::Pottery { 2 } else { 0 }, 2);
        assert_eq!(state.cost_of(Tech::Pottery), full / 2);
        assert_eq!(state.cost_of(Tech::Mining), Tech::Mining.cost());
        // However much science, a discovery takes four turns; the extra beakers are lost.
        state.current = Some(Tech::Pottery);
        state.science_per_turn = 1_000;
        assert_eq!(state.turns_left(), Some(MIN_RESEARCH_TURNS));
        let turns: Vec<Option<Tech>> = (0..4).map(|_| state.process_turn()).collect();
        assert_eq!(turns, vec![None, None, None, Some(Tech::Pottery)]);
        assert_eq!(state.progress, 0);
        // And with no science at all, forty.
        state.science_per_turn = 0;
        let next = state.current.unwrap();
        assert_eq!(state.turns_left(), Some(MAX_RESEARCH_TURNS));
        let learned = (0..MAX_RESEARCH_TURNS).filter_map(|_| state.process_turn()).collect::<Vec<_>>();
        assert_eq!(learned, vec![next]);
    }

    #[test]
    fn every_tech_is_reachable_through_the_ages_and_entering_one_is_noticed_once() {
        let mut state = TechState::knowing(&[]);
        assert_eq!(state.enter_new_era(), None, "the first look only takes note");
        let mut entered = Vec::new();
        while let Some(next) = Tech::ALL.into_iter().find(|t| state.can_research(*t)) {
            state.researched.insert(next);
            entered.extend(state.enter_new_era());
        }
        assert_eq!(state.researched.len(), Tech::ALL.len());
        assert_eq!(entered, vec![Era::Middle, Era::Industrial, Era::Modern]);
        for t in Tech::ALL {
            assert!(t.prerequisites().iter().all(|p| p.era() <= t.era()), "{t:?} needs a later age");
        }
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
