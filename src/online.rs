//! Online play, as correspondence chess is played: each human takes a turn when they like, the
//! game passes to the next, and the computer players move once a round, after the last human.
//!
//! The engine treats seat 0 as "you". The server keeps every game in its canonical numbering
//! (seat 0 is the host). A player whose seat is `k` loads it with [`swap_seat`]: every nation id 0
//! becomes `k` and every `k` becomes 0, and the active seat's own records (its treasury, research,
//! fog of war, foreign relations and messages) change places with the stored ones of seat `k`. The
//! swap is its own inverse, so the same call turns the game back before it is uploaded.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use bevy::prelude::*;
use bevy_civ::fog::FogOfWar;

use crate::foreign::Foreign;
use crate::savegame::SaveGame;

/// A human seat that is not playing right now: its own view of the world.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Seat {
    /// The seat's nation id, in the numbering of the game it is stored in.
    pub id: u32,
    pub fog: FogOfWar,
    pub foreign: Foreign,
    pub log: Vec<String>,
    pub welcome_migrants: bool,
}

/// An online game's place on the server and its order of play (canonical ids).
#[derive(Resource, Clone, Default, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Online {
    pub game: String,
    /// The secret that lets this player upload their turns.
    #[serde(default)]
    pub token: String,
    /// This player's seat (canonical id).
    pub seat: u32,
    /// The humans' seats, in the order they move each round.
    pub order: Vec<u32>,
    /// Index into `order` of the seat whose move it is.
    pub next: usize,
    /// The game is in this player's numbering (their seat is 0), as in their own saves; the server
    /// keeps it canonical.
    #[serde(default)]
    pub active: bool,
}

impl Online {
    /// Whether this player is the last human to move in the round (and so runs the world's turn).
    pub fn last_of_round(&self) -> bool {
        self.order.last() == Some(&self.seat)
    }

    /// The index in `order` of the seat after this one, for the upload.
    pub fn after_me(&self) -> usize {
        let me = self.order.iter().position(|s| *s == self.seat).unwrap_or(0);
        (me + 1) % self.order.len().max(1)
    }
}

/// The other humans' seats while one plays (kept with the game, in its current numbering).
#[derive(Resource, Clone, Default)]
pub struct OnlineSeats(pub Vec<Seat>);

fn swap_id(id: u32, k: u32) -> u32 {
    if id == 0 {
        k
    } else if id == k {
        0
    } else {
        id
    }
}

fn swap_keys<V>(map: &mut HashMap<u32, V>, k: u32) {
    let old = std::mem::take(map);
    map.extend(old.into_iter().map(|(id, v)| (swap_id(id, k), v)));
}

fn swap_set(set: &mut HashSet<u32>, k: u32) {
    let old = std::mem::take(set);
    set.extend(old.into_iter().map(|id| swap_id(id, k)));
}

/// A map keyed by an unordered pair of nations, kept as (smaller, larger).
fn swap_pairs<V>(map: &mut HashMap<(u32, u32), V>, k: u32) {
    let old = std::mem::take(map);
    map.extend(old.into_iter().map(|((a, b), v)| {
        let (a, b) = (swap_id(a, k), swap_id(b, k));
        ((a.min(b), a.max(b)), v)
    }));
}

fn swap_pair_set(set: &mut HashSet<(u32, u32)>, k: u32, ordered: bool) {
    let old = std::mem::take(set);
    set.extend(old.into_iter().map(|(a, b)| {
        let (a, b) = (swap_id(a, k), swap_id(b, k));
        if ordered { (a.min(b), a.max(b)) } else { (a, b) }
    }));
}

fn swap_value_keys<K: Eq + Hash, V>(map: &mut HashMap<K, V>, k: u32, id: impl Fn(&mut V) -> &mut u32) {
    for v in map.values_mut() {
        let slot = id(v);
        *slot = swap_id(*slot, k);
    }
}

/// Re-numbers a people's foreign relations between nations 0 and `k`.
fn swap_foreign(f: &mut Foreign, k: u32) {
    swap_set(&mut f.met, k);
    swap_set(&mut f.gone, k);
    swap_keys(&mut f.reply, k);
    swap_pairs(&mut f.since, k);
    swap_set(&mut f.pacts, k);
    if let Some(offer) = f.offer.as_mut() {
        offer.from = swap_id(offer.from, k);
        if let crate::foreign::OfferKind::JoinWar(target) = &mut offer.kind {
            *target = swap_id(*target, k);
        }
    }
    swap_keys(&mut f.cooldown, k);
    swap_keys(&mut f.strength, k);
    for d in &mut f.deals {
        d.civ = swap_id(d.civ, k);
    }
    for d in &mut f.ai_deals {
        d.seller = swap_id(d.seller, k);
        d.buyer = swap_id(d.buyer, k);
    }
    swap_keys(&mut f.crackdown, k);
    swap_keys(&mut f.our_crackdown, k);
    swap_set(&mut f.embargoes, k);
    swap_keys(&mut f.ceasefires, k);
    for g in &mut f.grievances {
        g.holder = swap_id(g.holder, k);
        g.against = swap_id(g.against, k);
    }
}

/// Makes seat `k` the one at seat 0 (and seat 0 seat `k`): every nation id and the active seat's
/// own records change places. Applying it twice gives back the game it started from.
pub fn swap_seat(save: &mut SaveGame, k: u32) {
    if k == 0 {
        return;
    }
    // The active seat's own records change places with those stored for seat k.
    let stored = save.seats.iter().position(|s| s.id == k);
    let mut theirs = stored.map(|i| save.seats.remove(i)).unwrap_or_default();
    let mut mine = Seat {
        id: k,
        fog: std::mem::take(&mut save.fog),
        foreign: std::mem::take(&mut save.foreign),
        log: std::mem::take(&mut save.log),
        welcome_migrants: save.state.welcome_migrants,
    };
    save.fog = std::mem::take(&mut theirs.fog);
    save.foreign = std::mem::take(&mut theirs.foreign);
    save.log = std::mem::take(&mut theirs.log);
    save.state.welcome_migrants = theirs.welcome_migrants || stored.is_none();
    // Their foreign relations are written in the old numbering: re-number them, and the stored ones.
    swap_foreign(&mut save.foreign, k);
    swap_foreign(&mut mine.foreign, k);
    for seat in &mut save.seats {
        seat.id = swap_id(seat.id, k);
        swap_foreign(&mut seat.foreign, k);
    }
    save.seats.push(mine);

    // The treasury, research and government of seat 0 live in the game state; of seat k, in its
    // entry among the peoples. Seat k's go to the game state, and seat 0's to the entry that becomes
    // seat k's (the one numbered 0 now).
    let players = &mut save.civs.players;
    if let (Some(ki), Some(zi)) = (players.iter().position(|c| c.id == k), players.iter().position(|c| c.id == 0)) {
        let s = &mut save.state;
        let civ = &mut players[ki];
        std::mem::swap(&mut s.gold, &mut civ.gold);
        std::mem::swap(&mut save.tech, &mut civ.tech);
        std::mem::swap(&mut s.government, &mut civ.government);
        std::mem::swap(&mut s.anarchy_turns, &mut civ.anarchy);
        std::mem::swap(&mut s.pending_government, &mut civ.pending_government);
        let rates = bevy_civ::economy::Rates { science: s.science_rate, luxury: s.luxury_rate };
        s.science_rate = civ.rates.science;
        s.luxury_rate = civ.rates.luxury;
        civ.rates = rates;
        s.title = civ.name.trim_start_matches("the ").to_string();
        let (a, b) = if ki < zi { let (l, r) = players.split_at_mut(zi); (&mut l[ki], &mut r[0]) } else { let (l, r) = players.split_at_mut(ki); (&mut r[0], &mut l[zi]) };
        std::mem::swap(&mut a.gold, &mut b.gold);
        std::mem::swap(&mut a.tech, &mut b.tech);
        std::mem::swap(&mut a.government, &mut b.government);
        std::mem::swap(&mut a.anarchy, &mut b.anarchy);
        std::mem::swap(&mut a.pending_government, &mut b.pending_government);
        std::mem::swap(&mut a.rates, &mut b.rates);
    }
    for civ in &mut save.civs.players {
        civ.id = swap_id(civ.id, k);
    }
    save.civs.players.sort_by_key(|c| c.id);

    // Everything else that names a nation.
    let s = &mut save.state;
    s.dominance.0 = swap_id(s.dominance.0, k);
    for (_, owner) in &mut s.objective_start {
        *owner = swap_id(*owner, k);
    }
    for e in &mut s.events {
        for (a, b) in e.wars.iter_mut().chain(e.alliances.iter_mut()).chain(e.peace.iter_mut()) {
            *a = swap_id(*a, k);
            *b = swap_id(*b, k);
        }
        for (owner, _, _) in &mut e.units {
            *owner = swap_id(*owner, k);
        }
    }
    for (a, b) in &mut s.nuclear_strikes {
        *a = swap_id(*a, k);
        *b = swap_id(*b, k);
    }
    swap_keys(&mut s.war_turns, k);
    swap_keys(&mut s.accounts, k);

    swap_pairs(&mut save.diplomacy.relations, k);
    swap_pairs(&mut save.diplomacy.treaties, k);
    swap_pair_set(&mut save.diplomacy.passage, k, true);
    for route in &mut save.trade.routes {
        route.owner = swap_id(route.owner, k);
        route.partner = swap_id(route.partner, k);
    }
    swap_keys(&mut save.space.ships, k);
    swap_value_keys(&mut save.wonders.built, k, |v| &mut v.0);
    swap_keys(&mut save.great.born, k);
    swap_keys(&mut save.great.general, k);
    swap_pair_set(&mut save.spies.embassies, k, false);
    for (_, owner, _) in &mut save.history.scores {
        *owner = swap_id(*owner, k);
    }
    for (_, _, _, owner) in save.history.borders.iter_mut().chain(save.history.cities.iter_mut()) {
        *owner = swap_id(*owner, k);
    }
    swap_keys(&mut save.culture.by_people, k);
    swap_value_keys(&mut save.culture.held, k, |v| &mut v.0);
    swap_keys(&mut save.golden.turns_left, k);
    swap_set(&mut save.golden.had, k);
    for city in &mut save.cities {
        city.city.owner = swap_id(city.city.owner, k);
    }
    for unit in &mut save.units {
        unit.unit.owner = swap_id(unit.unit.owner, k);
    }
}
