//! What sets each civilization apart: two traits and a unique unit.
//!
//! Traits give small, steady advantages in every city (more gold, science, shields, food or
//! contentment) or in war (troops promoted twice as often). A civilization's unique unit takes the
//! place of one ordinary unit in its cities: stronger, and with an ability of its own (see
//! `unit::Ability`).
//! Both follow from the civilization's name, so they need no saving.

use crate::building::Building;
use crate::unit::UnitType;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Trait {
    /// Troops are promoted twice as often.
    Militaristic,
    /// +1 gold in every city.
    Commercial,
    /// +1 science in every city, and a free discovery on entering each new age.
    Scientific,
    /// +1 content citizen in every city, and revolutions take a turn less.
    Religious,
    /// +1 shield in every city.
    Industrious,
    /// +1 food in every city.
    Agricultural,
}

impl Trait {
    pub const ALL: [Trait; 6] = [
        Trait::Militaristic,
        Trait::Commercial,
        Trait::Scientific,
        Trait::Religious,
        Trait::Industrious,
        Trait::Agricultural,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Trait::Militaristic => "Militaristic",
            Trait::Commercial => "Commercial",
            Trait::Scientific => "Scientific",
            Trait::Religious => "Religious",
            Trait::Industrious => "Industrious",
            Trait::Agricultural => "Agricultural",
        }
    }

    pub fn effect(&self) -> &'static str {
        match self {
            Trait::Militaristic => "troops are promoted twice as often",
            Trait::Commercial => "+1 gold in every city",
            Trait::Scientific => "+1 science in every city, a free discovery in each new age",
            Trait::Religious => "+1 content in every city, shorter revolutions",
            Trait::Industrious => "+1 shield in every city",
            Trait::Agricultural => "+1 food in every city",
        }
    }
}

/// The traits a wonder speaks to. A people's golden age begins with its first wonder that matches
/// one of its own traits (or with the first victory of its unique unit over another people).
pub fn of_wonder(wonder: Building) -> &'static [Trait] {
    use Building as B;
    use Trait::*;
    match wonder {
        B::Pyramids => &[Agricultural, Industrious],
        B::GreatLibrary => &[Scientific],
        B::HangingGardens => &[Agricultural],
        B::Colossus => &[Commercial],
        B::Oracle => &[Religious],
        B::GreatLighthouse => &[Commercial],
        B::GreatWall => &[Militaristic, Industrious],
        B::StatueOfZeus => &[Militaristic, Religious],
        B::Mausoleum => &[Religious],
        B::Copernicus => &[Scientific],
        B::SunTzu => &[Militaristic],
        B::MarcoPolo => &[Commercial],
        B::KingRichards => &[Industrious, Militaristic],
        B::Michelangelo => &[Religious],
        B::Leonardo => &[Scientific, Industrious],
        B::Bach => &[Religious],
        B::Magellan => &[Commercial],
        B::Newton => &[Scientific],
        B::AdamSmith => &[Commercial],
        B::Shakespeare => &[Agricultural],
        B::Darwin => &[Scientific],
        B::Liberty => &[Agricultural, Commercial],
        B::HooverDam => &[Industrious],
        B::Eiffel => &[Industrious, Commercial],
        B::Internet => &[Scientific, Commercial],
        B::Apollo => &[Scientific],
        B::CureForCancer => &[Agricultural, Scientific],
        B::Manhattan => &[Militaristic, Scientific],
        B::UnitedNations => &[Commercial, Religious],
        _ => &[],
    }
}

/// "Scientific and Commercial": the traits a wonder speaks to, for its description.
pub fn wonder_traits(wonder: Building) -> String {
    of_wonder(wonder)
        .iter()
        .map(|t| t.name())
        .collect::<Vec<_>>()
        .join(" and ")
}

/// A civilization's two traits and its unique unit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Character {
    pub traits: [Trait; 2],
    pub unique: UnitType,
}

impl Character {
    pub fn has(&self, t: Trait) -> bool {
        self.traits.contains(&t)
    }

    /// Extra (food, shields, gold, science) in each of its cities.
    pub fn city_bonus(&self) -> (u32, u32, u32, u32) {
        let n = |t: Trait| u32::from(self.has(t));
        (
            n(Trait::Agricultural),
            n(Trait::Industrious),
            n(Trait::Commercial),
            n(Trait::Scientific),
        )
    }

    /// Extra content citizens in each of its cities.
    pub fn content(&self) -> u32 {
        u32::from(self.has(Trait::Religious))
    }

    /// Chance a victorious unit of this level is promoted (twice the usual for a militaristic
    /// people; see [`crate::combat::promotion_chance`]).
    pub fn promotion_chance(&self, level: u8, against_barbarians: bool) -> f64 {
        crate::combat::promotion_chance(level, self.has(Trait::Militaristic), against_barbarians)
    }

    /// Whether a wonder speaks to one of its traits (and so can begin its golden age).
    pub fn admires(&self, wonder: Building) -> bool {
        of_wonder(wonder).iter().any(|t| self.has(*t))
    }

    /// "Militaristic and Commercial".
    pub fn describe(&self) -> String {
        format!("{} and {}", self.traits[0].name(), self.traits[1].name())
    }
}

/// The character of the civilization with this name (any civilization not listed gets one drawn
/// from its name, so it is always the same).
pub fn of(name: &str) -> Character {
    use Trait::*;
    use UnitType as U;
    let key = name.trim_start_matches("the ");
    let (traits, unique) = match key {
        "Rome" => ([Militaristic, Commercial], U::Praetorian),
        "Carthage" => ([Commercial, Industrious], U::Numidian),
        "China" => ([Militaristic, Scientific], U::ChuKoNu),
        "Egypt" => ([Religious, Industrious], U::RoyalChariot),
        "England" => ([Commercial, Scientific], U::ManOWar),
        "France" => ([Industrious, Commercial], U::Musketeer),
        "Greece" => ([Scientific, Commercial], U::Hoplite),
        "India" => ([Religious, Commercial], U::WarElephant),
        "Japan" => ([Militaristic, Religious], U::Samurai),
        "Macedon" => ([Militaristic, Scientific], U::Companion),
        "Mali" => ([Commercial, Agricultural], U::Mandekalu),
        "Muscovy" => ([Industrious, Agricultural], U::Cossack),
        "Mongols" => ([Militaristic, Agricultural], U::Keshik),
        "Persia" => ([Scientific, Industrious], U::ImmortalGuard),
        "Songhai" => ([Commercial, Agricultural], U::AskiaRider),
        "Spain" => ([Religious, Militaristic], U::Conquistador),
        "Aztecs" => ([Militaristic, Religious], U::JaguarWarrior),
        "Gauls" => ([Militaristic, Agricultural], U::GallicSwordsman),
        "Inca" => ([Agricultural, Industrious], U::Chasqui),
        "Ottomans" => ([Militaristic, Scientific], U::Sipahi),
        "Seleucids" => ([Scientific, Militaristic], U::Cataphract),
        // The Modern Nations expansion's peoples.
        "United States" => ([Commercial, Industrious], U::Marine),
        "United Kingdom" => ([Commercial, Scientific], U::Commando),
        "Germany" => ([Industrious, Scientific], U::Panzer),
        "Russia" => ([Militaristic, Industrious], U::GuardsRifleman),
        "Brazil" => ([Agricultural, Commercial], U::Bandeirante),
        "Australia" => ([Agricultural, Industrious], U::LightHorse),
        "Canada" => ([Agricultural, Scientific], U::Mountie),
        "Mexico" => ([Religious, Agricultural], U::Charro),
        _ => {
            let seed = key
                .bytes()
                .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
            let first = Trait::ALL[(seed % 6) as usize];
            let second = Trait::ALL[((seed / 6) % 5 + 1 + seed % 6) as usize % 6];
            let pool: Vec<UnitType> = UnitType::ALL
                .into_iter()
                .filter(|u| u.unique_base().is_some())
                .collect();
            ([first, second], pool[(seed / 30) as usize % pool.len()])
        }
    };
    // A theme without unique units: every people trains the ordinary units.
    let unique = if crate::theme::unique_units() {
        unique
    } else {
        U::Infantry
    };
    Character { traits, unique }
}

impl Character {
    /// Its unique unit, if the game has unique units and it has one.
    pub fn unique_unit(&self) -> Option<UnitType> {
        Some(self.unique).filter(|u| u.unique_base().is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_civilization_has_two_different_traits() {
        for name in [
            "Rome",
            "the Aztecs",
            "Atlantis",
            "Nowhere",
            "the Observers",
            "",
        ] {
            let c = of(name);
            assert_ne!(c.traits[0], c.traits[1], "{name}");
        }
        assert_eq!(of("the Inca"), of("Inca"));
        assert_eq!(of("Rome").unique, UnitType::Praetorian);
        assert_eq!(UnitType::Praetorian.unique_base(), Some(UnitType::Legion));
        assert_eq!(of("Rome").city_bonus(), (0, 0, 1, 0));
        assert_eq!(of("Egypt").content(), 1);
        assert!(of("Rome").promotion_chance(0, false) > of("Egypt").promotion_chance(0, false));
    }

    #[test]
    fn every_wonder_speaks_to_a_trait_and_every_trait_has_ancient_wonders() {
        let wonders: Vec<Building> = Building::ALL
            .into_iter()
            .filter(|b| b.is_wonder())
            .collect();
        for w in &wonders {
            assert!(!of_wonder(*w).is_empty(), "{w:?} speaks to no trait");
        }
        for t in Trait::ALL {
            let early = wonders
                .iter()
                .filter(|w| {
                    of_wonder(**w).contains(&t)
                        && w.required_tech()
                            .is_some_and(|tech| tech.era() == crate::tech::Era::Ancient)
                })
                .count();
            assert!(early >= 1, "no ancient wonder for {t:?}");
        }
        assert!(of("Rome").admires(Building::Colossus) && !of("Rome").admires(Building::Oracle));
        assert_eq!(
            wonder_traits(Building::Leonardo),
            "Scientific and Industrious"
        );
    }
}
