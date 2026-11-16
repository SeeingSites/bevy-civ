//! Diplomacy: how other leaders feel about the player, what they agree to, and what they propose.

use bevy::prelude::*;
use hexx::Hex;
use rand::Rng;
use std::collections::{HashMap, HashSet};

use bevy_civ::ai::Civilizations;
use bevy_civ::diplomacy::{Diplomacy, Treaty};
use bevy_civ::fog::FogOfWar;
use bevy_civ::tech::{Tech, TechState};
use bevy_civ::terrain::Resource as Good;

use crate::supply::Supplies;

use crate::{GameLog, GameState, AI_GRACE_TURNS, BARBARIANS, PLAYER};

pub const GIFT: i32 = 50;
pub const PACT_INCOME: i32 = 2;
/// How many turns a resource deal runs.
pub const DEAL_TURNS: u32 = 20;

/// A running exchange of resources and gold with another civilization.
#[derive(Clone, Debug)]
pub struct Deal {
    pub civ: u32,
    /// What the player sends.
    pub export: Option<Good>,
    /// What the player receives.
    pub import: Option<Good>,
    /// Gold the player receives each turn (negative: pays).
    pub gold: i32,
    pub ends: u32,
}

/// A resource one computer player sells to another.
#[derive(Clone, Debug)]
pub struct AiDeal {
    pub seller: u32,
    pub buyer: u32,
    pub good: Good,
    pub gold: i32,
    pub ends: u32,
}

/// A resource trade the player proposes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TradeAsk {
    Buy(Good),
    Sell(Good),
    Cancel(usize),
}

/// Gold per turn a resource is worth.
pub fn price(good: Good) -> i32 {
    if good.is_strategic() { 5 } else { 3 }
}

/// Something the player can ask of another leader.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DiploAct {
    Peace,
    Alliance,
    War,
    Gift,
    Demand,
    TradePact,
    TechTrade,
}

#[derive(Clone, PartialEq, Debug)]
pub enum OfferKind {
    /// Just news; nothing to decide.
    Notice,
    Peace,
    Tribute(i32),
    TechSwap { give: Tech, get: Tech },
    JoinWar(u32),
    /// They want to buy a resource from us for this much gold a turn.
    BuyFromUs(Good, i32),
    /// They offer to sell us a resource for this much gold a turn.
    SellToUs(Good, i32),
}

/// A message from another leader waiting for the player's answer.
#[derive(Clone, Debug)]
pub struct Offer {
    pub from: u32,
    pub kind: OfferKind,
    pub text: String,
}

#[derive(Resource, Default)]
pub struct Foreign {
    /// Civilizations the player has made contact with.
    pub met: HashSet<u32>,
    /// Civilizations that have lost everything.
    pub gone: HashSet<u32>,
    /// Each leader's latest words to the player.
    pub reply: HashMap<u32, String>,
    /// Turn on which the current treaty between two civilizations began.
    pub since: HashMap<(u32, u32), u32>,
    /// Civilizations with a trade agreement with the player.
    pub pacts: HashSet<u32>,
    /// Falls when the player breaks treaties; every leader remembers.
    pub reputation: i32,
    pub offer: Option<Offer>,
    /// Earliest turn each civilization will approach the player again.
    pub cooldown: HashMap<u32, u32>,
    /// Military strength per civilization, refreshed every turn.
    pub strength: HashMap<u32, f32>,
    pub deals: Vec<Deal>,
    /// Resource deals between computer players.
    pub ai_deals: Vec<AiDeal>,
}

impl Foreign {
    /// Every running resource deal as (seller, buyer, good).
    pub fn transfers(&self) -> Vec<crate::supply::Transfer> {
        let mut list = Vec::new();
        for deal in &self.deals {
            if let Some(good) = deal.import {
                list.push((deal.civ, PLAYER, good));
            }
            if let Some(good) = deal.export {
                list.push((PLAYER, deal.civ, good));
            }
        }
        list.extend(self.ai_deals.iter().map(|d| (d.seller, d.buyer, d.good)));
        list
    }

    pub fn treaty_age(&self, a: u32, b: u32, turn: u32) -> u32 {
        turn.saturating_sub(self.since.get(&pair(a, b)).copied().unwrap_or(0))
    }
}

/// Mutable access to everything diplomacy touches.
pub struct Ctx<'a> {
    pub diplomacy: &'a mut Diplomacy,
    pub foreign: &'a mut Foreign,
    pub civs: &'a mut Civilizations,
    pub state: &'a mut GameState,
    pub tech: &'a mut TechState,
    pub log: &'a mut GameLog,
}

fn pair(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

pub fn mood(attitude: i32) -> &'static str {
    match attitude {
        i32::MIN..=-40 => "Furious",
        -39..=-15 => "Hostile",
        -14..=-1 => "Wary",
        0..=14 => "Neutral",
        15..=34 => "Friendly",
        _ => "Devoted",
    }
}

fn set_treaty(ctx: &mut Ctx, a: u32, b: u32, treaty: Treaty) {
    ctx.diplomacy.set_treaty(a, b, treaty);
    ctx.foreign.since.insert(pair(a, b), ctx.state.turn);
    if treaty == Treaty::War {
        for (x, y) in [(a, b), (b, a)] {
            if x == PLAYER {
                ctx.foreign.pacts.remove(&y);
                let before = ctx.foreign.deals.len();
                ctx.foreign.deals.retain(|d| d.civ != y);
                if ctx.foreign.deals.len() < before {
                    ctx.log.push("War ends our resource deals with them.");
                }
            }
        }
    }
}

fn shift(ctx: &mut Ctx, civ: u32, delta: i32) {
    ctx.diplomacy.change_attitude(PLAYER, civ, delta);
    let key = pair(PLAYER, civ);
    if let Some(value) = ctx.diplomacy.relations.get_mut(&key) {
        *value = (*value).clamp(-100, 100);
    }
}

fn strength(ctx: &Ctx, civ: u32) -> f32 {
    ctx.foreign.strength.get(&civ).copied().unwrap_or(0.0).max(1.0)
}

/// What each side could teach the other: (they teach us, we teach them).
fn tech_swap(ctx: &Ctx, civ: u32) -> (Option<Tech>, Option<Tech>) {
    let Some(other) = ctx.civs.players.iter().find(|c| c.id == civ) else { return (None, None) };
    let get = Tech::ALL.into_iter().find(|t| other.tech.researched.contains(t) && ctx.tech.can_research(*t));
    let give = Tech::ALL.into_iter().find(|t| ctx.tech.researched.contains(t) && other.tech.can_research(*t));
    (get, give)
}

fn teach(ctx: &mut Ctx, civ: u32, tech: Tech) {
    if let Some(other) = ctx.civs.players.iter_mut().find(|c| c.id == civ) {
        other.tech.learn(tech);
    }
}

/// The player declares war; treaty-breaking costs reputation and the victim's allies join in.
fn declare_war(ctx: &mut Ctx, civ: u32, name: &str) {
    let treaty = ctx.diplomacy.get_treaty(PLAYER, civ);
    if matches!(treaty, Treaty::Peace | Treaty::Alliance) {
        ctx.foreign.reputation -= 15;
        let others: Vec<u32> = ctx.civs.players.iter().filter(|c| !c.is_human && c.id != civ).map(|c| c.id).collect();
        for other in others {
            shift(ctx, other, -8);
        }
        ctx.log.push("Breaking our treaty has damaged our reputation abroad.");
    }
    set_treaty(ctx, PLAYER, civ, Treaty::War);
    ctx.log.push(format!("We declare war on {name}."));
    let allies: Vec<(u32, String)> = ctx
        .civs
        .players
        .iter()
        .filter(|c| !c.is_human && c.id != civ && !ctx.foreign.gone.contains(&c.id))
        .filter(|c| ctx.diplomacy.get_treaty(civ, c.id) == Treaty::Alliance && ctx.diplomacy.get_treaty(PLAYER, c.id) != Treaty::War)
        .map(|c| (c.id, c.name.clone()))
        .collect();
    for (ally, ally_name) in allies {
        set_treaty(ctx, PLAYER, ally, Treaty::War);
        ctx.log.push(format!("{ally_name}, allied with {name}, declares war on us!"));
    }
}

/// Handles a request the player makes in the foreign advisor window.
pub fn player_act(ctx: &mut Ctx, civ: u32, act: DiploAct) {
    let Some(other) = ctx.civs.players.iter().find(|c| c.id == civ) else { return };
    let (name, aggression, their_gold) = (other.name.clone(), other.aggression, other.gold);
    let treaty = ctx.diplomacy.get_treaty(PLAYER, civ);
    let attitude = ctx.diplomacy.get_attitude(PLAYER, civ);
    let (ours, theirs) = (strength(ctx, PLAYER), strength(ctx, civ));
    let age = ctx.foreign.treaty_age(PLAYER, civ, ctx.state.turn);
    let at_war = treaty == Treaty::War;

    let reply: String = match act {
        DiploAct::War => {
            if at_war {
                "We are already at war.".into()
            } else {
                declare_war(ctx, civ, &name);
                if aggression > 0.6 { "You will regret this." } else { "So be it. We will defend what is ours." }.into()
            }
        }
        DiploAct::Peace => {
            if !at_war {
                "There is no war between us to end.".into()
            } else if age < 5 {
                "The blood is not yet dry. Ask again later.".into()
            } else if theirs > ours * 1.5 && attitude < -20 {
                "Why would we stop now? We are winning.".into()
            } else {
                set_treaty(ctx, PLAYER, civ, Treaty::Peace);
                ctx.log.push(format!("Peace is made with {name}."));
                "Let there be peace between our peoples.".into()
            }
        }
        DiploAct::Alliance => {
            if at_war {
                "End this war first.".into()
            } else if treaty == Treaty::Alliance {
                "We already stand together.".into()
            } else if attitude >= 30 {
                set_treaty(ctx, PLAYER, civ, Treaty::Alliance);
                ctx.log.push(format!("We are now allied with {name}."));
                "Together we shall prosper. Your enemies are ours.".into()
            } else {
                "An alliance is for trusted friends. We are not there yet.".into()
            }
        }
        DiploAct::Gift => {
            if ctx.state.gold < GIFT {
                format!("(A gift costs {GIFT} gold.)")
            } else {
                ctx.state.gold -= GIFT;
                if let Some(other) = ctx.civs.players.iter_mut().find(|c| c.id == civ) {
                    other.gold += GIFT;
                }
                shift(ctx, civ, 12);
                "A generous gift. It will be remembered.".into()
            }
        }
        DiploAct::Demand => {
            if treaty == Treaty::Alliance {
                shift(ctx, civ, -5);
                "Allies do not make demands of each other.".into()
            } else if ours > theirs * 1.8 && their_gold >= 10 {
                let amount = their_gold.min(40);
                if let Some(other) = ctx.civs.players.iter_mut().find(|c| c.id == civ) {
                    other.gold -= amount;
                }
                ctx.state.gold += amount;
                shift(ctx, civ, -15);
                ctx.log.push(format!("{name} pays us {amount} gold in tribute."));
                "Take it, and leave us in peace.".into()
            } else if aggression > 0.6 && theirs >= ours && !at_war && ctx.state.turn >= AI_GRACE_TURNS {
                shift(ctx, civ, -10);
                set_treaty(ctx, PLAYER, civ, Treaty::War);
                ctx.log.push(format!("{name} answers our demand with war!"));
                "Insolence! To arms!".into()
            } else {
                shift(ctx, civ, -10);
                "You are in no position to make demands.".into()
            }
        }
        DiploAct::TradePact => {
            if at_war {
                "We do not trade with enemies.".into()
            } else if ctx.foreign.pacts.contains(&civ) {
                "Our merchants already trade freely.".into()
            } else if attitude >= 5 {
                ctx.foreign.pacts.insert(civ);
                shift(ctx, civ, 5);
                ctx.log.push(format!("A trade agreement is signed with {name} (+{PACT_INCOME} gold per turn)."));
                "Our markets are open to you.".into()
            } else {
                "We see no profit in it yet.".into()
            }
        }
        DiploAct::TechTrade => {
            if at_war {
                "We share nothing with enemies.".into()
            } else if attitude < 10 {
                "Our knowledge is not for strangers.".into()
            } else {
                match tech_swap(ctx, civ) {
                    (Some(get), Some(give)) => {
                        ctx.tech.learn(get);
                        teach(ctx, civ, give);
                        shift(ctx, civ, 4);
                        ctx.log.push(format!("{name} teaches us {} in exchange for {}.", get.name(), give.name()));
                        format!("A fair exchange: {} for {}.", get.name(), give.name())
                    }
                    (Some(get), None) => {
                        let price = get.cost() as i32 * 3;
                        if ctx.state.gold >= price {
                            ctx.state.gold -= price;
                            ctx.tech.learn(get);
                            ctx.log.push(format!("We buy the secret of {} from {name} for {price} gold.", get.name()));
                            format!("For {price} gold, the secret of {} is yours.", get.name())
                        } else {
                            format!("You have nothing to teach us. {} would cost you {price} gold.", get.name())
                        }
                    }
                    (None, _) => "You have nothing to learn from us.".into(),
                }
            }
        }
    };
    ctx.foreign.reply.insert(civ, reply);
}

/// Handles a resource deal the player proposes in the foreign advisor window.
pub fn player_trade(ctx: &mut Ctx, supplies: &Supplies, civ: u32, ask: TradeAsk) {
    let Some(other) = ctx.civs.players.iter().find(|c| c.id == civ) else { return };
    let (name, their_gold) = (other.name.clone(), other.gold);
    let attitude = ctx.diplomacy.get_attitude(PLAYER, civ);
    let at_war = ctx.diplomacy.get_treaty(PLAYER, civ) == Treaty::War;
    let (ours, theirs) = (supplies.civ(PLAYER), supplies.civ(civ));
    let ends = ctx.state.turn + DEAL_TURNS;

    let reply: String = match ask {
        TradeAsk::Cancel(index) => {
            if index < ctx.foreign.deals.len() && ctx.foreign.deals[index].civ == civ {
                ctx.foreign.deals.remove(index);
                shift(ctx, civ, -3);
                ctx.log.push(format!("We end a resource deal with {name}."));
                "A pity. Our merchants will look elsewhere.".into()
            } else {
                return;
            }
        }
        _ if at_war => "We do not trade with enemies.".into(),
        TradeAsk::Buy(good) => {
            let needed = if good.is_strategic() { 10 } else { -5 };
            if ours.imports.contains(&good) {
                format!("You already receive {} from abroad.", good.name())
            } else if theirs.spare(good) == 0 {
                format!("We have no {} to spare.", good.name())
            } else if attitude < needed {
                if good.is_strategic() { "We do not arm those we do not trust.".into() } else { "Not while relations are this poor.".into() }
            } else if ctx.state.gold < price(good) {
                format!("{} costs {} gold a turn. Your treasury is empty.", good.name(), price(good))
            } else {
                ctx.foreign.deals.push(Deal { civ, export: None, import: Some(good), gold: -price(good), ends });
                ctx.log.push(format!("{name} will supply us with {} for {} gold a turn ({DEAL_TURNS} turns).", good.name(), price(good)));
                format!("{} for {} gold a turn. Agreed.", good.name(), price(good))
            }
        }
        TradeAsk::Sell(good) => {
            let pays = price(good) - 1;
            if ours.spare(good) == 0 {
                format!("You have no {} to spare.", good.name())
            } else if theirs.has(good) {
                format!("We have all the {} we need.", good.name())
            } else if attitude < -10 {
                "We want nothing from you.".into()
            } else if their_gold < pays * 4 {
                "Our treasury cannot bear it just now.".into()
            } else {
                ctx.foreign.deals.push(Deal { civ, export: Some(good), import: None, gold: pays, ends });
                shift(ctx, civ, 3);
                ctx.log.push(format!("We will supply {name} with {} for {pays} gold a turn ({DEAL_TURNS} turns).", good.name()));
                format!("We will pay {pays} gold a turn for your {}.", good.name())
            }
        }
    };
    ctx.foreign.reply.insert(civ, reply);
}

/// Settles running resource deals for the turn; returns the gold the player nets from them.
pub fn settle_deals(ctx: &mut Ctx, supplies: &Supplies) -> i32 {
    let turn = ctx.state.turn;
    let ours = supplies.civ(PLAYER);
    let mut income = 0;
    let mut kept = Vec::new();
    for deal in std::mem::take(&mut ctx.foreign.deals) {
        let name = ctx.civs.players.iter().find(|c| c.id == deal.civ).map(|c| c.name.clone()).unwrap_or_default();
        let what = deal.export.or(deal.import).map(|g| g.name()).unwrap_or("goods");
        let their_gold = ctx.civs.players.iter().find(|c| c.id == deal.civ).map(|c| c.gold).unwrap_or(0);
        let lost_source = deal.export.is_some_and(|g| ours.connected.get(&g).copied().unwrap_or(0) < ours.exports.get(&g).copied().unwrap_or(0));
        let they_lost = deal.import.is_some_and(|g| supplies.civ(deal.civ).connected.get(&g).copied().unwrap_or(0) == 0);
        if turn >= deal.ends {
            ctx.log.push(format!("Our deal with {name} for {what} has run its course."));
        } else if ctx.foreign.gone.contains(&deal.civ) || they_lost {
            ctx.log.push(format!("{name} can no longer supply {what}."));
        } else if lost_source {
            ctx.log.push(format!("We can no longer supply {what} to {name}; the deal ends."));
        } else if deal.gold < 0 && ctx.state.gold + income + deal.gold < 0 {
            ctx.log.push(format!("We cannot pay {name} for {what}; the deal ends."));
        } else if deal.gold > 0 && their_gold < deal.gold {
            ctx.log.push(format!("{name} can no longer pay for {what}; the deal ends."));
        } else {
            income += deal.gold;
            if let Some(other) = ctx.civs.players.iter_mut().find(|c| c.id == deal.civ) {
                other.gold -= deal.gold;
            }
            kept.push(deal);
        }
    }
    ctx.foreign.deals = kept;
    income
}

/// Applies the player's answer to the pending offer.
pub fn answer_offer(ctx: &mut Ctx, accept: bool) {
    let Some(offer) = ctx.foreign.offer.take() else { return };
    let civ = offer.from;
    let name = ctx.civs.players.iter().find(|c| c.id == civ).map(|c| c.name.clone()).unwrap_or_default();
    let reply: &str = match (offer.kind, accept) {
        (OfferKind::Notice, _) => return,
        (OfferKind::Peace, true) => {
            set_treaty(ctx, PLAYER, civ, Treaty::Peace);
            ctx.log.push(format!("Peace is made with {name}."));
            "A wise choice."
        }
        (OfferKind::Peace, false) => "Then the war goes on.",
        (OfferKind::Tribute(amount), true) => {
            let paid = amount.min(ctx.state.gold);
            ctx.state.gold -= paid;
            if let Some(other) = ctx.civs.players.iter_mut().find(|c| c.id == civ) {
                other.gold += paid;
            }
            shift(ctx, civ, 8);
            ctx.log.push(format!("We pay {paid} gold in tribute to {name}."));
            "You have bought yourself some time."
        }
        (OfferKind::Tribute(_), false) => {
            shift(ctx, civ, -15);
            if rand::thread_rng().gen_bool(0.5) && ctx.diplomacy.get_treaty(PLAYER, civ) != Treaty::War {
                set_treaty(ctx, PLAYER, civ, Treaty::War);
                ctx.log.push(format!("{name} declares war on us!"));
                "Then we will take it by force."
            } else {
                "You will come to regret your pride."
            }
        }
        (OfferKind::TechSwap { give, get }, true) => {
            ctx.tech.learn(get);
            teach(ctx, civ, give);
            shift(ctx, civ, 5);
            ctx.log.push(format!("{name} teaches us {} in exchange for {}.", get.name(), give.name()));
            "Knowledge shared is knowledge doubled."
        }
        (OfferKind::TechSwap { .. }, false) => "As you wish.",
        (OfferKind::JoinWar(target), true) => {
            let target_name = ctx.civs.players.iter().find(|c| c.id == target).map(|c| c.name.clone()).unwrap_or_default();
            declare_war(ctx, target, &target_name);
            shift(ctx, civ, 20);
            "We will not forget your friendship."
        }
        (OfferKind::JoinWar(_), false) => {
            shift(ctx, civ, -6);
            "We had hoped for more from you."
        }
        (OfferKind::BuyFromUs(good, gold), true) => {
            ctx.foreign.deals.push(Deal { civ, export: Some(good), import: None, gold, ends: ctx.state.turn + DEAL_TURNS });
            shift(ctx, civ, 4);
            ctx.log.push(format!("We will supply {name} with {} for {gold} gold a turn ({DEAL_TURNS} turns).", good.name()));
            "A pleasure doing business."
        }
        (OfferKind::SellToUs(good, gold), true) => {
            ctx.foreign.deals.push(Deal { civ, export: None, import: Some(good), gold: -gold, ends: ctx.state.turn + DEAL_TURNS });
            shift(ctx, civ, 2);
            ctx.log.push(format!("{name} will supply us with {} for {gold} gold a turn ({DEAL_TURNS} turns).", good.name()));
            "Our caravans leave at dawn."
        }
        (OfferKind::BuyFromUs(..) | OfferKind::SellToUs(..), false) => "Perhaps another time.",
    };
    ctx.foreign.reply.insert(civ, reply.to_string());
}

/// Runs once per turn: contact, drifting attitudes, and the decisions other leaders make.
pub fn turn(ctx: &mut Ctx, fog: &FogOfWar, supplies: &Supplies, units: &[(Hex, u32)], cities: &[(Hex, u32)]) {
    let mut rng = rand::thread_rng();
    let turn = ctx.state.turn;
    let roster: Vec<(u32, String, String, f32)> =
        ctx.civs.players.iter().filter(|c| !c.is_human).map(|c| (c.id, c.name.clone(), c.leader.clone(), c.aggression)).collect();
    let city_count = |owner: u32| cities.iter().filter(|(_, o)| *o == owner).count();
    let nearest = |a: u32, b: u32| {
        cities
            .iter()
            .filter(|(_, o)| *o == a)
            .flat_map(|(p, _)| cities.iter().filter(|(_, o)| *o == b).map(move |(q, _)| p.distance_to(*q)))
            .min()
            .unwrap_or(i32::MAX)
    };

    // The fallen leave the stage.
    for (id, name, _, _) in &roster {
        let alive = city_count(*id) > 0 || units.iter().any(|(_, o)| o == id);
        if !alive && ctx.foreign.gone.insert(*id) && ctx.foreign.met.contains(id) {
            ctx.log.push(format!("{name} has been destroyed."));
        }
    }
    let alive: Vec<&(u32, String, String, f32)> = roster.iter().filter(|c| !ctx.foreign.gone.contains(&c.0)).collect();

    // First contact.
    for (id, name, leader, aggression) in alive.iter().copied() {
        if ctx.foreign.met.contains(id) {
            continue;
        }
        let seen = units.iter().any(|(p, o)| o == id && fog.visible.contains(p)) || cities.iter().any(|(p, o)| o == id && fog.explored.contains(p));
        if seen {
            ctx.foreign.met.insert(*id);
            ctx.log.push(format!("We have made contact with {name}, led by {leader}."));
            let greeting = if *aggression > 0.6 { "Stay out of our way and you may live to see us prosper." } else { "Greetings, stranger. May our peoples know peace." };
            ctx.foreign.reply.insert(*id, greeting.to_string());
        }
    }

    let ours = strength(ctx, PLAYER);
    for (id, name, leader, aggression) in alive.iter().copied() {
        let (id, aggression) = (*id, *aggression);
        if !ctx.foreign.met.contains(&id) {
            continue;
        }
        let treaty = ctx.diplomacy.get_treaty(PLAYER, id);
        let age = ctx.foreign.treaty_age(PLAYER, id, turn);
        let theirs = strength(ctx, id);

        // Attitude drifts towards what the situation warrants.
        let shared_enemy = alive.iter().any(|other| {
            other.0 != id && ctx.diplomacy.get_treaty(PLAYER, other.0) == Treaty::War && ctx.diplomacy.get_treaty(id, other.0) == Treaty::War
        });
        let crowding = cities
            .iter()
            .filter(|(_, o)| *o == PLAYER)
            .filter(|(p, _)| cities.iter().any(|(q, o)| *o == id && p.distance_to(*q) <= 5))
            .count() as i32;
        let mut target = 15 - (aggression * 50.0) as i32 + ctx.foreign.reputation - crowding * 4;
        target += match treaty {
            Treaty::War => -40,
            Treaty::Peace => 5,
            Treaty::Alliance => 25,
            Treaty::None => 0,
        };
        if ctx.foreign.pacts.contains(&id) {
            target += 15;
        }
        if shared_enemy {
            target += 15;
        }
        if aggression > 0.55 && ours < theirs * 0.6 {
            target -= 10; // contempt for the weak
        }
        let attitude = ctx.diplomacy.get_attitude(PLAYER, id);
        shift(ctx, id, (target - attitude).clamp(-2, 2));
        let attitude = ctx.diplomacy.get_attitude(PLAYER, id);

        // Allies join the player's wars.
        if treaty == Treaty::Alliance {
            for other in alive.iter().copied() {
                if other.0 != id && ctx.diplomacy.get_treaty(PLAYER, other.0) == Treaty::War && ctx.diplomacy.get_treaty(id, other.0) != Treaty::War {
                    set_treaty(ctx, id, other.0, Treaty::War);
                    ctx.log.push(format!("Our ally {name} declares war on {}.", other.1));
                }
            }
        }

        // An unhappy, confident neighbour declares war.
        let may_attack = match treaty {
            Treaty::None => true,
            Treaty::Peace => age >= 15 && aggression > 0.6,
            _ => false,
        };
        let threshold = -12 - ((1.0 - aggression) * 40.0) as i32;
        if may_attack && turn >= AI_GRACE_TURNS && attitude <= threshold && theirs >= ours * 0.8 && rng.gen_bool(0.25) {
            set_treaty(ctx, PLAYER, id, Treaty::War);
            ctx.log.push(format!("{name} declares war on us!"));
            ctx.foreign.reply.insert(id, "Your lands will be ours.".into());
            if ctx.foreign.offer.is_none() {
                ctx.foreign.offer = Some(Offer { from: id, kind: OfferKind::Notice, text: format!("{leader} has had enough of words. {name} declares war on us!") });
            }
            continue;
        }

        // Proposals, at most one waiting at a time.
        if ctx.foreign.offer.is_some() || ctx.foreign.cooldown.get(&id).copied().unwrap_or(0) > turn {
            continue;
        }
        let offer = if treaty == Treaty::War {
            (age >= 8 && (theirs < ours * 0.8 || age >= 25))
                .then(|| (OfferKind::Peace, "This war bleeds us both. Shall we put down our swords?".to_string(), 10))
        } else if treaty != Treaty::Alliance && aggression > 0.55 && theirs > ours * 1.5 && turn >= AI_GRACE_TURNS && rng.gen_bool(0.15) {
            let amount = (ctx.state.gold / 2).clamp(20, 80);
            Some((OfferKind::Tribute(amount), format!("Our armies are restless. {amount} gold would calm them."), 20))
        } else if attitude >= 20 && rng.gen_bool(0.2) {
            match tech_swap(ctx, id) {
                (Some(get), Some(give)) => Some((OfferKind::TechSwap { give, get }, format!("Our scholars would trade {} for your knowledge of {}.", get.name(), give.name()), 12)),
                _ => None,
            }
        } else if attitude >= 15 && rng.gen_bool(0.15) {
            alive
                .iter()
                .find(|other| {
                    other.0 != id
                        && ctx.foreign.met.contains(&other.0)
                        && ctx.diplomacy.get_treaty(id, other.0) == Treaty::War
                        && matches!(ctx.diplomacy.get_treaty(PLAYER, other.0), Treaty::None | Treaty::Peace)
                })
                .map(|other| (OfferKind::JoinWar(other.0), format!("{} threatens us all. Join our war against them.", other.1), 20))
        } else {
            None
        };
        // Failing all that, their merchants look for business: buy what they lack, sell what they can spare.
        let offer = offer.or_else(|| {
            if treaty == Treaty::War || attitude < -5 || !rng.gen_bool(0.12) {
                return None;
            }
            let (ours, theirs) = (supplies.civ(PLAYER), supplies.civ(id));
            let their_gold = ctx.civs.players.iter().find(|c| c.id == id).map(|c| c.gold).unwrap_or(0);
            let their_tech = ctx.civs.players.iter().find(|c| c.id == id).map(|c| c.tech.researched.clone()).unwrap_or_default();
            let tradeable = |g: &Good| g.is_strategic() || g.is_luxury();
            let wanted = Good::ALL.into_iter().filter(tradeable).find(|g| {
                ours.spare(*g) > 0 && !theirs.has(*g) && g.reveal_tech().is_none_or(|t| their_tech.contains(&t)) && their_gold >= price(*g) * 4
            });
            if let Some(good) = wanted {
                let gold = price(good) - 1;
                return Some((OfferKind::BuyFromUs(good, gold), format!("We hear you have {} to spare. We would pay {gold} gold a turn for it.", good.name()), 15));
            }
            let needed = |g: Good| if g.is_strategic() { attitude >= 10 } else { true };
            let on_offer = Good::ALL.into_iter().filter(tradeable).find(|g| {
                theirs.spare(*g) > 0 && !ours.has(*g) && needed(*g) && g.reveal_tech().is_none_or(|t| ctx.tech.researched.contains(&t)) && ctx.state.gold >= price(*g) * 3
            });
            on_offer.map(|good| (OfferKind::SellToUs(good, price(good)), format!("Our merchants can bring you {} for {} gold a turn.", good.name(), price(good)), 15))
        });
        if let Some((kind, text, wait)) = offer {
            ctx.foreign.cooldown.insert(id, turn + wait);
            ctx.foreign.offer = Some(Offer { from: id, kind, text });
        }
    }

    // The other powers fight and make up among themselves.
    for (i, a) in alive.iter().enumerate() {
        for b in alive.iter().skip(i + 1) {
            if nearest(a.0, b.0) > 12 {
                continue;
            }
            let treaty = ctx.diplomacy.get_treaty(a.0, b.0);
            let age = ctx.foreign.treaty_age(a.0, b.0, turn);
            let known = ctx.foreign.met.contains(&a.0) || ctx.foreign.met.contains(&b.0);
            match treaty {
                Treaty::War if age >= 15 && rng.gen_bool(0.07) => {
                    set_treaty(ctx, a.0, b.0, Treaty::Peace);
                    if known {
                        ctx.log.push(format!("{} and {} make peace.", a.1, b.1));
                    }
                }
                Treaty::None | Treaty::Peace if turn >= AI_GRACE_TURNS && (treaty == Treaty::None || age >= 20) && rng.gen_bool((0.008 * (a.3 + b.3)) as f64) => {
                    set_treaty(ctx, a.0, b.0, Treaty::War);
                    if known {
                        ctx.log.push(format!("{} and {} are now at war.", a.1, b.1));
                    }
                }
                _ => {}
            }
        }
    }

    // Computer players also trade resources with each other: settle the running deals...
    let mut kept = Vec::new();
    for deal in std::mem::take(&mut ctx.foreign.ai_deals) {
        let seller_stock = supplies.civ(deal.seller);
        let short = seller_stock.connected.get(&deal.good).copied().unwrap_or(0) < seller_stock.exports.get(&deal.good).copied().unwrap_or(0);
        let buyer_gold = ctx.civs.players.iter().find(|c| c.id == deal.buyer).map(|c| c.gold).unwrap_or(0);
        let ended = turn >= deal.ends
            || short
            || buyer_gold < deal.gold
            || ctx.foreign.gone.contains(&deal.seller)
            || ctx.foreign.gone.contains(&deal.buyer)
            || ctx.diplomacy.get_treaty(deal.seller, deal.buyer) == Treaty::War;
        if ended {
            continue;
        }
        for civ in ctx.civs.players.iter_mut() {
            if civ.id == deal.buyer {
                civ.gold -= deal.gold;
            } else if civ.id == deal.seller {
                civ.gold += deal.gold;
            }
        }
        kept.push(deal);
    }
    ctx.foreign.ai_deals = kept;
    // ...and strike new ones: a neighbour with a spare source sells to one that has none.
    for buyer in alive.iter().copied() {
        if !rng.gen_bool(0.25) {
            continue;
        }
        let Some(buyer_civ) = ctx.civs.players.iter().find(|c| c.id == buyer.0) else { continue };
        let (buyer_gold, buyer_tech) = (buyer_civ.gold, buyer_civ.tech.researched.clone());
        let stock = supplies.civ(buyer.0);
        let wanted = Good::ALL.into_iter().filter(|g| g.is_strategic() || g.is_luxury()).find_map(|good| {
            let useful = !stock.has(good) && good.reveal_tech().is_none_or(|t| buyer_tech.contains(&t)) && buyer_gold >= price(good) * 4;
            let already = ctx.foreign.ai_deals.iter().any(|d| d.buyer == buyer.0 && d.good == good);
            if !useful || already {
                return None;
            }
            alive
                .iter()
                .find(|seller| {
                    seller.0 != buyer.0
                        && supplies.civ(seller.0).spare(good) > 0
                        && ctx.diplomacy.get_treaty(seller.0, buyer.0) != Treaty::War
                        && nearest(seller.0, buyer.0) <= 16
                })
                .map(|seller| (good, (seller.0, seller.1.clone())))
        });
        if let Some((good, seller)) = wanted {
            ctx.foreign.ai_deals.push(AiDeal { seller: seller.0, buyer: buyer.0, good, gold: price(good) - 1, ends: turn + DEAL_TURNS });
            if ctx.foreign.met.contains(&seller.0) && ctx.foreign.met.contains(&buyer.0) {
                ctx.log.push(format!("{} now buys {} from {}.", buyer.1, good.name(), seller.1));
            }
        }
    }
    let _ = BARBARIANS;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_civ::ai::Civ;
    use crate::supply::CivSupply;

    fn civ(id: u32, human: bool) -> Civ {
        Civ {
            id,
            name: if human { "Us".into() } else { format!("Rival {id}") },
            is_human: human,
            gold: 100,
            tech: TechState::knowing(&["Iron Working".to_string(), "Bronze Working".to_string()]),
            aggression: 0.3,
            leader: "Leader".into(),
            adjective: "Rival".into(),
            portrait: 0,
            city_names: vec![],
        }
    }

    struct Table {
        diplomacy: Diplomacy,
        foreign: Foreign,
        civs: Civilizations,
        state: GameState,
        tech: TechState,
        log: GameLog,
    }

    impl Table {
        fn new(rivals: u32) -> Self {
            let mut players = vec![civ(0, true)];
            players.extend((1..=rivals).map(|id| civ(id, false)));
            let mut foreign = Foreign::default();
            foreign.met.extend(1..=rivals);
            Self {
                diplomacy: Diplomacy::default(),
                foreign,
                civs: Civilizations { players },
                state: GameState { gold: 100, ..Default::default() },
                tech: TechState::knowing(&["Iron Working".to_string(), "Bronze Working".to_string()]),
                log: GameLog::default(),
            }
        }

        fn ctx(&mut self) -> Ctx<'_> {
            Ctx { diplomacy: &mut self.diplomacy, foreign: &mut self.foreign, civs: &mut self.civs, state: &mut self.state, tech: &mut self.tech, log: &mut self.log }
        }
    }

    /// Supplies in which `owner` has two connected iron sources and nobody else has any.
    fn iron_with(owner: u32) -> Supplies {
        let mut supplies = Supplies::default();
        let mut stock = CivSupply::default();
        stock.sources.insert(Good::Iron, 2);
        stock.connected.insert(Good::Iron, 2);
        supplies.civs.insert(owner, stock);
        supplies
    }

    #[test]
    fn a_rival_offers_to_buy_our_spare_resource() {
        let mut table = Table::new(1);
        let supplies = iron_with(PLAYER);
        let cities = [(Hex::ZERO, PLAYER), (Hex::new(6, 0), 1)];
        let fog = FogOfWar::default();
        // Their merchants call only now and then; give them time.
        for turn in 1..400 {
            table.state.turn = turn;
            foreign_turn(&mut table, &fog, &supplies, &cities);
            if table.foreign.offer.is_some() {
                break;
            }
        }
        let offer = table.foreign.offer.clone().expect("the rival proposes a deal");
        assert_eq!(offer.kind, OfferKind::BuyFromUs(Good::Iron, price(Good::Iron) - 1));

        answer_offer(&mut table.ctx(), true);
        assert_eq!(table.foreign.deals.len(), 1);
        assert_eq!(table.foreign.deals[0].export, Some(Good::Iron));
        assert_eq!(table.foreign.transfers(), vec![(PLAYER, 1, Good::Iron)]);
        let income = settle_deals(&mut table.ctx(), &supplies);
        assert_eq!(income, price(Good::Iron) - 1);
    }

    fn foreign_turn(table: &mut Table, fog: &FogOfWar, supplies: &Supplies, cities: &[(Hex, u32)]) {
        turn(&mut table.ctx(), fog, supplies, &[], cities);
    }

    #[test]
    fn a_rival_offers_to_sell_what_we_lack() {
        let mut table = Table::new(1);
        // Friendly enough to sell a strategic resource.
        table.diplomacy.change_attitude(PLAYER, 1, 60);
        table.foreign.pacts.insert(1);
        let supplies = iron_with(1);
        let cities = [(Hex::ZERO, PLAYER), (Hex::new(6, 0), 1)];
        let fog = FogOfWar::default();
        let mut seen = None;
        for turn in 1..600 {
            table.state.turn = turn;
            table.foreign.offer = None;
            foreign_turn(&mut table, &fog, &supplies, &cities);
            if let Some(Offer { kind: OfferKind::SellToUs(good, gold), .. }) = table.foreign.offer.clone() {
                seen = Some((good, gold));
                break;
            }
        }
        assert_eq!(seen, Some((Good::Iron, price(Good::Iron))));
        answer_offer(&mut table.ctx(), true);
        assert_eq!(table.foreign.transfers(), vec![(1, PLAYER, Good::Iron)]);
    }

    #[test]
    fn rivals_trade_resources_among_themselves() {
        let mut table = Table::new(2);
        let supplies = iron_with(1);
        let cities = [(Hex::ZERO, PLAYER), (Hex::new(6, 0), 1), (Hex::new(12, 0), 2)];
        let fog = FogOfWar::default();
        for turn in 1..200 {
            table.state.turn = turn;
            table.foreign.offer = None;
            foreign_turn(&mut table, &fog, &supplies, &cities);
            if !table.foreign.ai_deals.is_empty() {
                break;
            }
        }
        let deal = table.foreign.ai_deals.first().expect("the rival without iron buys it");
        assert_eq!((deal.seller, deal.buyer, deal.good), (1, 2, Good::Iron));

        // War between them ends the deal.
        table.diplomacy.set_treaty(1, 2, Treaty::War);
        table.state.turn += 1;
        foreign_turn(&mut table, &fog, &supplies, &cities);
        assert!(table.foreign.ai_deals.iter().all(|d| !(d.seller == 1 && d.buyer == 2)));
    }
}
