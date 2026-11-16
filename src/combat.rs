use crate::unit::UnitType;
use crate::terrain::Terrain;

/// How much harder troops fight with a great general on their tile.
pub const GENERAL_BONUS: f32 = 1.5;

/// One side of a battle.
#[derive(Clone, Copy, Debug)]
pub struct Side {
    pub kind: UnitType,
    pub hp: u32,
    /// 0 regular, 1 veteran, 2 elite: each level fights a quarter harder.
    pub level: u8,
    /// A great general stands with it.
    pub led: bool,
}

/// Where the defender stands.
#[derive(Clone, Copy, Debug)]
pub struct Ground {
    pub terrain: Terrain,
    pub fortified: bool,
    /// City walls (double defence against land attacks).
    pub walls: bool,
    /// A river on the defender's tile (+25% defence).
    pub river: bool,
    /// A Coastal Fortress (double defence against ships).
    pub coast_fort: bool,
}

/// How a battle ended, with what each side has left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The defender is destroyed; the attacker has this many hit points left.
    Won { attacker_hp: u32 },
    /// The attacker is destroyed; the defender has this many hit points left.
    Lost { defender_hp: u32 },
    /// The attacker pulled back with one hit point; the defender has this many left.
    Retreated { defender_hp: u32 },
}

/// Attack and defence strength of each side.
pub fn strengths(attacker: &Side, defender: &Side, ground: &Ground) -> (f32, f32) {
    let level = |l: u8| 1.0 + 0.25 * l as f32;
    let led = |l: bool| if l { GENERAL_BONUS } else { 1.0 };
    let mut attack = attacker.kind.stats().0 as f32 * attacker.kind.attack_bonus(ground.terrain) * level(attacker.level) * led(attacker.led);
    let mut defence = (defender.kind.stats().1 as f32).max(0.1) * defender.kind.defense_bonus() * level(defender.level) * led(defender.led);
    defence *= match ground.terrain {
        Terrain::Hills | Terrain::Forest => 1.5,
        Terrain::Mountain => 2.0,
        _ => 1.0,
    };
    if ground.fortified {
        defence *= 1.25;
    }
    if ground.river {
        defence *= 1.25;
    }
    if ground.walls && attacker.kind.domain() == crate::unit::Domain::Land {
        defence *= 2.0;
    }
    if ground.coast_fort && attacker.kind.domain() == crate::unit::Domain::Sea {
        defence *= 2.0;
    }
    attack = attack.max(0.0);
    (attack, defence)
}

/// A battle fought in rounds: each round the stronger side is likelier to land a blow, and the
/// first to run out of hit points loses. A faster attacker (more moves) about to die retreats.
pub fn fight(attacker: Side, defender: Side, ground: Ground, rng: &mut impl rand::Rng) -> Outcome {
    let (attack, defence) = strengths(&attacker, &defender, &ground);
    if attack <= 0.0 {
        return Outcome::Lost { defender_hp: defender.hp };
    }
    let p = attack / (attack + defence);
    let can_retreat = attacker.kind.stats().2 > defender.kind.stats().2 && attacker.kind.domain() == crate::unit::Domain::Land;
    let (mut a, mut d) = (attacker.hp.max(1), defender.hp.max(1));
    loop {
        if rng.gen::<f32>() < p {
            d -= 1;
            if d == 0 {
                return Outcome::Won { attacker_hp: a };
            }
        } else {
            a -= 1;
            if a == 1 && can_retreat {
                return Outcome::Retreated { defender_hp: d };
            }
            if a == 0 {
                return Outcome::Lost { defender_hp: d };
            }
        }
    }
}

/// Artillery fire: a few rounds against the defender that can wound but never kill, at no risk.
/// Returns the defender's hit points afterwards.
pub fn bombard(attacker: Side, defender: Side, ground: Ground, rng: &mut impl rand::Rng) -> u32 {
    let (attack, defence) = strengths(&attacker, &defender, &ground);
    let p = attack / (attack + defence).max(0.01);
    let mut hp = defender.hp;
    for _ in 0..3 {
        if hp > 1 && rng.gen::<f32>() < p {
            hp -= 1;
        }
    }
    hp
}

#[cfg(test)]
mod fight_tests {
    use super::*;
    use rand::SeedableRng;

    fn side(kind: UnitType) -> Side {
        Side { kind, hp: 3, level: 0, led: false }
    }

    fn open() -> Ground {
        Ground { terrain: Terrain::Plains, fortified: false, walls: false, river: false, coast_fort: false }
    }

    fn wins(attacker: Side, defender: Side, ground: Ground) -> u32 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        (0..1000).filter(|_| matches!(fight(attacker, defender, ground, &mut rng), Outcome::Won { .. })).count() as u32
    }

    #[test]
    fn stronger_units_win_more_and_walls_and_veterans_matter() {
        let even = wins(side(UnitType::Warrior), side(UnitType::Warrior), open());
        assert!((350..650).contains(&even), "{even}");
        assert!(wins(side(UnitType::Tank), side(UnitType::Warrior), open()) > 950, "a tank rarely fails (and then retreats)");
        let walled = Ground { walls: true, ..open() };
        assert!(wins(side(UnitType::Legion), side(UnitType::Spearman), walled) < wins(side(UnitType::Legion), side(UnitType::Spearman), open()));
        let veteran = Side { level: 2, hp: 5, ..side(UnitType::Spearman) };
        assert!(wins(side(UnitType::Legion), veteran, open()) < wins(side(UnitType::Legion), side(UnitType::Spearman), open()));
    }

    #[test]
    fn fast_attackers_retreat_and_bombardment_never_kills() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(9);
        let outcomes: Vec<Outcome> = (0..200).map(|_| fight(side(UnitType::Horseman), side(UnitType::Phalanx), open(), &mut rng)).collect();
        assert!(outcomes.iter().any(|o| matches!(o, Outcome::Retreated { .. })));
        assert!(!outcomes.iter().any(|o| matches!(o, Outcome::Lost { .. })), "a horseman always gets away");
        for _ in 0..50 {
            assert!(bombard(side(UnitType::Artillery), side(UnitType::Warrior), open(), &mut rng) >= 1);
        }
    }
}
