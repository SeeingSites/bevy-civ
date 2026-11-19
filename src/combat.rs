//! Battles, fought the way Civilization III fights them.
//!
//! A battle is a run of rounds, and every round one side loses a hit point: the attacker lands the
//! blow with chance attack / (attack + defence), both strengths with all their modifiers. Units
//! have 3 hit points, 4 as veterans and 5 as elites - experience adds hit points, never strength.
//!
//! The attacker fights at its plain attack. The defender's bonuses are added up before they
//! multiply its defence: terrain (+10% on open ground and at sea, +25% in forest, +50% on hills,
//! +100% on mountains), a river +25%, fortified +25%, a fortress +50%; in a city its size (a town
//! nothing, a city +50%, a metropolis +100%) or, against land attacks, its walls (+100%) if those
//! are better; against ships a Coastal Fortress (+100%). Pikemen stand twice as firm against
//! mounted troops. A great general (+50%) and an army's extra members (+10% each) are this game's
//! own and multiply both sides alike.
//!
//! A fast attacker (two moves or more) that is about to fall to a slow defender may pull back with
//! its last hit point: half the time, a little more often as a veteran or elite. Winners may be
//! promoted (see [`promotion_chance`]).
//!
//! Guns (catapults to radar artillery) cannot attack: they bombard (see [`bombard`]), wearing a
//! defender down from up to their range away but never killing it.

use crate::building::Building;
use crate::terrain::Terrain;
use crate::unit::{Ability, Domain, UnitType};

/// How much harder troops fight with a great general on their tile.
pub const GENERAL_BONUS: f32 = 1.5;

/// Defence bonuses, added together (0.25 = +25%).
pub const FORTIFIED_BONUS: f32 = 0.25;
pub const RIVER_BONUS: f32 = 0.25;
pub const FORTRESS_BONUS: f32 = 0.5;
/// City walls, against land attacks.
pub const WALLS_BONUS: f32 = 1.0;
/// A city that has outgrown a town (see [`city_bonus`]), and a metropolis.
pub const CITY_BONUS: f32 = 0.5;
pub const METROPOLIS_BONUS: f32 = 1.0;
/// A Coastal Fortress, against ships.
pub const COASTAL_FORT_BONUS: f32 = 1.0;

/// The chance that a fast attacker about to fall pulls back, by experience (regular, veteran,
/// elite).
pub const RETREAT_CHANCE: [f64; 3] = [0.5, 0.58, 0.66];

/// One side of a battle.
#[derive(Clone, Copy, Debug)]
pub struct Side {
    pub kind: UnitType,
    pub hp: u32,
    /// 0 regular, 1 veteran, 2 elite: more hit points (already in `hp`) and readier to pull back.
    pub level: u8,
    /// A great general stands with it.
    pub led: bool,
    /// Units fighting as one (an army has several; a lone unit 1).
    pub members: u32,
}

/// Where the defender stands.
#[derive(Clone, Copy, Debug)]
pub struct Ground {
    pub terrain: Terrain,
    pub fortified: bool,
    /// City walls (or the Great Wall): +100% against land attacks.
    pub walls: bool,
    /// A river on the defender's tile (+25% defence).
    pub river: bool,
    /// A Coastal Fortress (+100% against ships).
    pub coast_fort: bool,
    /// The number of citizens of the city the defender stands in (`None` in the field).
    pub city: Option<u32>,
    /// A fortress on the tile (+50%).
    pub fortress: bool,
}

impl Ground {
    /// Open ground of this terrain: no city, river or works.
    pub fn open(terrain: Terrain) -> Ground {
        Ground { terrain, fortified: false, walls: false, river: false, coast_fort: false, city: None, fortress: false }
    }
}

/// A running count of the battles fought between the peoples' turns (for the autoplay report).
#[derive(bevy::prelude::Resource, Default, Clone, Debug)]
pub struct BattleTally {
    pub battles: u32,
    /// Won by the attacker.
    pub won: u32,
    /// The attacker pulled back.
    pub retreated: u32,
    pub promotions: u32,
    /// Battles won by an elite unit, and the great generals they brought forth.
    pub elite_wins: u32,
    pub generals: u32,
    pub bombardments: u32,
    /// Hit points knocked off by bombardment.
    pub shelled: u32,
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

/// The defensive bonus of the land itself.
pub fn terrain_bonus(terrain: Terrain) -> f32 {
    match terrain {
        Terrain::Mountain => 1.0,
        Terrain::Hills => 0.5,
        Terrain::Forest => 0.25,
        _ => 0.1,
    }
}

/// The bonus a city of this many citizens gives its defenders. A town - a place no bigger than a
/// city grows without water works - has no defences of its own; a city (up to the size an Aqueduct
/// allows) has +50%, a metropolis beyond that +100%. Against land attacks, walls give +100% where
/// that is better.
pub fn city_bonus(population: u32, walls: bool, attacker: Domain) -> f32 {
    let natural = if population > Building::size_limit(&[Building::Aqueduct], false) {
        METROPOLIS_BONUS
    } else if population > Building::size_limit(&[], false) {
        CITY_BONUS
    } else {
        0.0
    };
    if walls && attacker == Domain::Land { natural.max(WALLS_BONUS) } else { natural }
}

/// Every bonus the defender has on this ground against an attacker of that domain, added up
/// (0.5 means +50%).
pub fn defence_bonus(ground: &Ground, attacker: Domain) -> f32 {
    let mut bonus = terrain_bonus(ground.terrain);
    if ground.river {
        bonus += RIVER_BONUS;
    }
    if ground.fortified {
        bonus += FORTIFIED_BONUS;
    }
    if ground.fortress {
        bonus += FORTRESS_BONUS;
    }
    if let Some(population) = ground.city {
        bonus += city_bonus(population, ground.walls, attacker);
    }
    if ground.coast_fort && attacker == Domain::Sea {
        bonus += COASTAL_FORT_BONUS;
    }
    bonus
}

fn led(l: bool) -> f32 {
    if l { GENERAL_BONUS } else { 1.0 }
}

fn army(members: u32) -> f32 {
    1.0 + crate::unit::ARMY_MEMBER_BONUS * members.saturating_sub(1) as f32
}

/// How much a unique unit's ability adds to an attack and to a defence (+50%).
pub const ABILITY_BONUS: f32 = 1.5;

/// A unique unit's ability in attack: a charge over open ground, or storming a city.
fn ability_attack(kind: UnitType, ground: &Ground) -> f32 {
    let open = matches!(ground.terrain, Terrain::Grassland | Terrain::Plains | Terrain::Desert);
    match kind.ability() {
        Some(Ability::Charge) if open && ground.city.is_none() => ABILITY_BONUS,
        Some(Ability::Storm) if ground.city.is_some() => ABILITY_BONUS,
        _ => 1.0,
    }
}

/// The defence `defender` puts up on this ground against an attacker (or gun) of that kind.
pub fn defence_against(attacker: UnitType, defender: &Side, ground: &Ground) -> f32 {
    let mut base = (defender.kind.stats().1 as f32).max(0.1);
    if defender.kind.anti_mounted() && attacker.is_mounted() {
        base *= 2.0;
    }
    // A unique unit holding a city.
    if defender.kind.ability() == Some(Ability::Bastion) && ground.city.is_some() {
        base *= ABILITY_BONUS;
    }
    base * (1.0 + defence_bonus(ground, attacker.domain())) * led(defender.led) * army(defender.members)
}

/// Attack and defence strength of each side.
pub fn strengths(attacker: &Side, defender: &Side, ground: &Ground) -> (f32, f32) {
    let attack = attacker.kind.stats().0 as f32 * led(attacker.led) * army(attacker.members) * ability_attack(attacker.kind, ground);
    (attack.max(0.0), defence_against(attacker.kind, defender, ground))
}

/// Whether this attacker may pull back from a losing fight against that defender: a fast unit on
/// land, against one that is not fast.
pub fn may_retreat(attacker: UnitType, defender: UnitType) -> bool {
    attacker.domain() == Domain::Land && attacker.is_fast() && !defender.is_fast()
}

/// The chance that `attacker` pulls back when it is about to fall (0 when it cannot).
fn retreat_chance(attacker: &Side, defender: &Side) -> f64 {
    if may_retreat(attacker.kind, defender.kind) { RETREAT_CHANCE[(attacker.level as usize).min(2)] } else { 0.0 }
}

/// A battle fought in rounds: each round the attacker lands a blow with chance
/// attack / (attack + defence), else takes one, and the first to run out of hit points loses. A fast
/// attacker brought down to its last hit point by a slow defender (that has more than one left) may
/// pull back.
pub fn fight(attacker: Side, defender: Side, ground: Ground, rng: &mut impl rand::Rng) -> Outcome {
    let (attack, defence) = strengths(&attacker, &defender, &ground);
    if attack <= 0.0 {
        return Outcome::Lost { defender_hp: defender.hp };
    }
    let p = attack / (attack + defence);
    let retreat = retreat_chance(&attacker, &defender);
    let (mut a, mut d) = (attacker.hp.max(1), defender.hp.max(1));
    loop {
        if rng.gen::<f32>() < p {
            d -= 1;
            if d == 0 {
                return Outcome::Won { attacker_hp: a };
            }
        } else {
            a -= 1;
            if a == 0 {
                return Outcome::Lost { defender_hp: d };
            }
            if a == 1 && d > 1 && retreat > 0.0 && rng.gen_bool(retreat) {
                return Outcome::Retreated { defender_hp: d };
            }
        }
    }
}

/// The chances of each way a battle can end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Odds {
    pub win: f64,
    pub retreat: f64,
    pub loss: f64,
}

/// The exact odds of [`fight`], worked out over every way the hit points can fall.
pub fn odds(attacker: &Side, defender: &Side, ground: &Ground) -> Odds {
    let (attack, defence) = strengths(attacker, defender, ground);
    if attack <= 0.0 {
        return Odds { win: 0.0, retreat: 0.0, loss: 1.0 };
    }
    let p = (attack / (attack + defence)) as f64;
    let retreat = retreat_chance(attacker, defender);
    let (ha, hd) = (attacker.hp.max(1) as usize, defender.hp.max(1) as usize);
    // reach[a][d]: the chance the fight reaches a round with these hit points left.
    let mut reach = vec![vec![0.0f64; hd + 1]; ha + 1];
    reach[ha][hd] = 1.0;
    let mut result = Odds { win: 0.0, retreat: 0.0, loss: 0.0 };
    // Every round takes a hit point from one side, so states are visited in falling order of a + d.
    for total in (2..=ha + hd).rev() {
        for a in 1..=ha {
            let Some(d) = total.checked_sub(a).filter(|d| (1..=hd).contains(d)) else { continue };
            let q = reach[a][d];
            if q == 0.0 {
                continue;
            }
            if d == 1 {
                result.win += q * p;
            } else {
                reach[a][d - 1] += q * p;
            }
            let hit = q * (1.0 - p);
            if a == 1 {
                result.loss += hit;
            } else if a == 2 && d > 1 && retreat > 0.0 {
                result.retreat += hit * retreat;
                reach[1][d] += hit * (1.0 - retreat);
            } else {
                reach[a - 1][d] += hit;
            }
        }
    }
    result
}

/// The chance that a victor of this level is promoted, as in Civilization III: a regular one time
/// in four, a veteran one time in eight (an elite is at the top); twice as often for a militaristic
/// people, half as often for beating barbarians.
pub fn promotion_chance(level: u8, militaristic: bool, against_barbarians: bool) -> f64 {
    let base = match level {
        0 => 0.25,
        1 => 0.125,
        _ => return 0.0,
    };
    base * if militaristic { 2.0 } else { 1.0 } * if against_barbarians { 0.5 } else { 1.0 }
}

/// The chance that one round of `gun`'s bombardment lands on this defender.
pub fn bombard_odds(gun: UnitType, defender: &Side, ground: &Ground) -> f32 {
    let Some((strength, _, _)) = gun.bombard() else { return 0.0 };
    let strength = strength as f32;
    strength / (strength + defence_against(gun, defender, ground)).max(0.01)
}

/// Bombardment, as in Civilization III: the gun fires as many rounds as its rate of fire, each
/// landing with chance bombard / (bombard + defence) - the defence with all its bonuses - and
/// taking a hit point; but it never takes the last one, and the gun comes to no harm. Returns the
/// defender's hit points afterwards.
pub fn bombard(gun: UnitType, defender: Side, ground: Ground, rng: &mut impl rand::Rng) -> u32 {
    let Some((_, _, rate)) = gun.bombard() else { return defender.hp };
    let p = bombard_odds(gun, &defender, &ground);
    let mut hp = defender.hp;
    for _ in 0..rate {
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
        Side { kind, hp: 3, level: 0, led: false, members: 1 }
    }

    fn open() -> Ground {
        Ground::open(Terrain::Plains)
    }

    fn wins(attacker: Side, defender: Side, ground: Ground) -> u32 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        (0..1000).filter(|_| matches!(fight(attacker, defender, ground, &mut rng), Outcome::Won { .. })).count() as u32
    }

    /// The textbook chance of winning a fight to the death with p a round: the defender's last
    /// hit point falls after k of the attacker's, for any k below the attacker's hit points.
    fn closed_form(p: f64, ha: u32, hd: u32) -> f64 {
        let choose = |n: u32, k: u32| (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64);
        (0..ha).map(|k| choose(hd - 1 + k, k) * p.powi(hd as i32) * (1.0 - p).powi(k as i32)).sum()
    }

    #[test]
    fn odds_follow_the_civ3_round_formula() {
        // Swordsmen (Legion, attack 3) against a fortified Spearman (defence 2) on open ground:
        // 2 x (1 + 0.1 + 0.25) = 2.7, so each round goes to the attacker with p = 3 / 5.7.
        let ground = Ground { fortified: true, ..open() };
        let (a, d) = strengths(&side(UnitType::Legion), &side(UnitType::Spearman), &ground);
        assert!((a - 3.0).abs() < 1e-6 && (d - 2.7).abs() < 1e-5, "{a} {d}");
        let p = 3.0 / 5.7;
        for (ha, hd) in [(3, 3), (4, 3), (3, 5), (5, 4), (1, 1)] {
            let o = odds(&Side { hp: ha, ..side(UnitType::Legion) }, &Side { hp: hd, ..side(UnitType::Spearman) }, &ground);
            assert!((o.win - closed_form(p, ha, hd)).abs() < 1e-5, "{ha} {hd}: {o:?}");
            assert_eq!(o.retreat, 0.0, "slow units never pull back");
            assert!((o.win + o.loss - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn exact_odds_match_fought_battles() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(11);
        let cases = [
            (Side { level: 1, hp: 4, ..side(UnitType::Knight) }, side(UnitType::Musketman), Ground { fortified: true, ..Ground::open(Terrain::Hills) }),
            (side(UnitType::Cavalry), Side { level: 2, hp: 5, ..side(UnitType::Rifleman) }, open()),
            (side(UnitType::Tank), side(UnitType::Infantry), Ground { city: Some(12), walls: true, ..open() }),
        ];
        for (attacker, defender, ground) in cases {
            let o = odds(&attacker, &defender, &ground);
            assert!((o.win + o.retreat + o.loss - 1.0).abs() < 1e-9);
            let n = 20_000;
            let (mut won, mut fled) = (0, 0);
            for _ in 0..n {
                match fight(attacker, defender, ground, &mut rng) {
                    Outcome::Won { .. } => won += 1,
                    Outcome::Retreated { .. } => fled += 1,
                    Outcome::Lost { .. } => {}
                }
            }
            assert!((won as f64 / n as f64 - o.win).abs() < 0.015, "{attacker:?}: {o:?} vs {won}");
            assert!((fled as f64 / n as f64 - o.retreat).abs() < 0.015, "{attacker:?}: {o:?} vs {fled}");
        }
    }

    #[test]
    fn defence_bonuses_add_up() {
        // A fortified Spearman in a walled town on a hill by a river: 2 x (1 + 0.5 + 0.25 + 0.25 + 1).
        let ground = Ground { terrain: Terrain::Hills, fortified: true, walls: true, river: true, city: Some(4), ..open() };
        let (_, d) = strengths(&side(UnitType::Warrior), &side(UnitType::Spearman), &ground);
        assert!((d - 6.0).abs() < 1e-5, "{d}");
        // Walls do not stop ships; a Coastal Fortress does.
        let (_, d) = strengths(&side(UnitType::Frigate), &side(UnitType::Spearman), &Ground { coast_fort: true, ..ground });
        assert!((d - 2.0 * (1.0 + 0.5 + 0.25 + 0.25 + 1.0)).abs() < 1e-5, "{d}");
        let (_, d) = strengths(&side(UnitType::Frigate), &side(UnitType::Spearman), &ground);
        assert!((d - 2.0 * 2.0).abs() < 1e-5, "{d}");
        // A fortress: +50%, on top of fortifying.
        let (_, d) = strengths(&side(UnitType::Warrior), &side(UnitType::Spearman), &Ground { fortress: true, fortified: true, ..Ground::open(Terrain::Grassland) });
        assert!((d - 2.0 * 1.85).abs() < 1e-5, "{d}");
        // Mountains double, forest a quarter more.
        assert_eq!(terrain_bonus(Terrain::Mountain), 1.0);
        assert_eq!(terrain_bonus(Terrain::Forest), 0.25);
    }

    #[test]
    fn a_city_defends_by_its_size_and_walls_lift_it() {
        let town = Building::size_limit(&[], false);
        let city = Building::size_limit(&[Building::Aqueduct], false);
        assert_eq!(city_bonus(town, false, Domain::Land), 0.0);
        assert_eq!(city_bonus(town + 1, false, Domain::Land), CITY_BONUS);
        assert_eq!(city_bonus(city + 1, false, Domain::Land), METROPOLIS_BONUS);
        assert_eq!(city_bonus(town, true, Domain::Land), WALLS_BONUS);
        assert_eq!(city_bonus(town + 1, true, Domain::Land), WALLS_BONUS);
        assert_eq!(city_bonus(town, true, Domain::Sea), 0.0, "walls only stop land attacks");
        assert_eq!(city_bonus(city + 1, false, Domain::Air), METROPOLIS_BONUS);
    }

    #[test]
    fn pikemen_stand_firm_against_horses_and_experience_is_hit_points() {
        let (_, against_knight) = strengths(&side(UnitType::Knight), &side(UnitType::Pikeman), &open());
        let (_, against_legion) = strengths(&side(UnitType::Legion), &side(UnitType::Pikeman), &open());
        assert!((against_knight - 2.0 * against_legion).abs() < 1e-5);
        // A veteran fights no harder, but lasts longer.
        let veteran = Side { level: 1, hp: 4, ..side(UnitType::Spearman) };
        assert_eq!(strengths(&side(UnitType::Legion), &veteran, &open()), strengths(&side(UnitType::Legion), &side(UnitType::Spearman), &open()));
        assert!(wins(side(UnitType::Legion), veteran, open()) < wins(side(UnitType::Legion), side(UnitType::Spearman), open()));
    }

    #[test]
    fn stronger_units_win_more_and_walls_matter() {
        let even = wins(side(UnitType::Warrior), side(UnitType::Warrior), Ground::open(Terrain::Water));
        assert!((350..650).contains(&even), "{even}");
        assert!(wins(side(UnitType::Tank), side(UnitType::Warrior), open()) > 990);
        let walled = Ground { walls: true, city: Some(3), ..open() };
        assert!(wins(side(UnitType::Legion), side(UnitType::Spearman), walled) < wins(side(UnitType::Legion), side(UnitType::Spearman), open()));
        // An army of four pools its hit points and fights 30% harder than one unit.
        let army = Side { hp: 12, members: 4, ..side(UnitType::Legion) };
        assert!(wins(army, side(UnitType::Legion), open()) > 950);
        let (lone, _) = strengths(&side(UnitType::Legion), &side(UnitType::Legion), &open());
        let (strong, _) = strengths(&army, &side(UnitType::Legion), &open());
        assert!((strong / lone - 1.3).abs() < 0.001);
    }

    #[test]
    fn fast_attackers_sometimes_pull_back_from_slow_defenders_only() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(9);
        let outcomes: Vec<Outcome> = (0..400).map(|_| fight(side(UnitType::Horseman), side(UnitType::Phalanx), open(), &mut rng)).collect();
        let fled = outcomes.iter().filter(|o| matches!(o, Outcome::Retreated { .. })).count();
        let lost = outcomes.iter().filter(|o| matches!(o, Outcome::Lost { .. })).count();
        assert!(fled > 50 && lost > 50, "about half of the doomed get away: {fled} fled, {lost} lost");
        // Fast against fast: no way out.
        let o = odds(&side(UnitType::Cavalry), &side(UnitType::Knight), &open());
        assert_eq!(o.retreat, 0.0);
        // Elites get away more often.
        let regular = odds(&side(UnitType::Knight), &side(UnitType::Pikeman), &open());
        let elite = odds(&Side { level: 2, ..side(UnitType::Knight) }, &side(UnitType::Pikeman), &open());
        assert!(elite.retreat > regular.retreat);
        // Never from a defender on its last hit point.
        let o = odds(&side(UnitType::Horseman), &Side { hp: 1, ..side(UnitType::Phalanx) }, &open());
        assert_eq!(o.retreat, 0.0);
    }

    #[test]
    fn guns_bombard_within_their_rate_of_fire_and_never_kill() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        assert_eq!(UnitType::Artillery.stats().0, 0, "guns cannot attack");
        let o = odds(&side(UnitType::Cannon), &side(UnitType::Warrior), &open());
        assert_eq!(o.loss, 1.0, "a gun that attacks is lost");
        // A cannon takes at most one hit point a turn, radar artillery up to three.
        let wounded: Vec<u32> = (0..200).map(|_| bombard(UnitType::Cannon, side(UnitType::Warrior), open(), &mut rng)).collect();
        assert!(wounded.iter().all(|hp| *hp >= 2) && wounded.contains(&2));
        let shelled: Vec<u32> = (0..200).map(|_| bombard(UnitType::RadarArtillery, Side { hp: 5, ..side(UnitType::Rifleman) }, open(), &mut rng)).collect();
        assert!(shelled.iter().all(|hp| *hp >= 2) && shelled.contains(&2));
        for _ in 0..100 {
            assert!(bombard(UnitType::RadarArtillery, Side { hp: 2, ..side(UnitType::Warrior) }, open(), &mut rng) >= 1);
        }
        // Walls blunt land guns, not warships; Artillery (12) against a fortified Infantryman (10)
        // in a walled city on a hill: 10 x (1 + 0.5 + 0.25 + 1) = 27.5.
        let walled = Ground { terrain: Terrain::Hills, fortified: true, walls: true, city: Some(5), ..open() };
        assert!((bombard_odds(UnitType::Artillery, &side(UnitType::Infantry), &walled) - 12.0 / 39.5).abs() < 1e-5);
        assert!(bombard_odds(UnitType::Battleship, &side(UnitType::Infantry), &walled) > bombard_odds(UnitType::Artillery, &side(UnitType::Infantry), &walled));
        assert_eq!(bombard(UnitType::Warrior, side(UnitType::Warrior), open(), &mut rng), 3, "only guns bombard");
    }

    #[test]
    fn promotions_are_rare_and_rarer_at_the_top() {
        assert_eq!(promotion_chance(0, false, false), 0.25);
        assert_eq!(promotion_chance(1, false, false), 0.125);
        assert_eq!(promotion_chance(2, true, false), 0.0);
        assert_eq!(promotion_chance(0, true, false), 0.5);
        assert_eq!(promotion_chance(1, false, true), 0.0625);
    }

    #[test]
    fn unique_abilities_change_the_odds_where_they_should() {
        let mut in_city = open();
        in_city.city = Some(5);
        // Storm: the Praetorian strikes cities harder than the Legion, but not the open field.
        let (praetorian, _) = strengths(&side(UnitType::Praetorian), &side(UnitType::Spearman), &in_city);
        let (legion, _) = strengths(&side(UnitType::Legion), &side(UnitType::Spearman), &in_city);
        assert!(praetorian > legion * 1.5 - 0.01);
        let (praetorian_open, _) = strengths(&side(UnitType::Praetorian), &side(UnitType::Spearman), &open());
        assert_eq!(praetorian_open, UnitType::Praetorian.stats().0 as f32);
        // Bastion: a Musketeer holds a city half again as well; Shield wall: a Hoplite doubles against riders.
        let (_, musketeer) = strengths(&side(UnitType::Knight), &side(UnitType::Musketeer), &in_city);
        let (_, musketeer_open) = strengths(&side(UnitType::Knight), &side(UnitType::Musketeer), &open());
        assert!(musketeer > musketeer_open);
        let (_, hoplite) = strengths(&side(UnitType::Knight), &side(UnitType::Hoplite), &open());
        assert_eq!(hoplite, 2.0 * UnitType::Hoplite.stats().1 as f32 * (1.0 + defence_bonus(&open(), Domain::Land)));
        // Self-sufficient: the War Elephant needs no Horses.
        assert!(UnitType::WarElephant.required_resources().is_empty() && !UnitType::Knight.required_resources().is_empty());
        // Unique units are built in place of their base unit, never by name.
        assert!(!UnitType::Hoplite.is_trainable() && UnitType::Hoplite.cost() == UnitType::Phalanx.cost());
    }
}
