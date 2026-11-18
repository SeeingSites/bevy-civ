//! Tips for a new player: short notes that appear the first time something happens (the first
//! turn, a first city, first contact, first disorder...). One shows at a time, under the status
//! bar; "Got it" moves on, "No more tips" turns them off (remembered between games, and back on
//! from the Game menu).

use std::collections::HashSet;

use bevy::prelude::*;

use bevy_civ::city::City;
use bevy_civ::diplomacy::{Diplomacy, Happiness, Treaty};
use bevy_civ::save::{read_setting, write_setting};
use bevy_civ::unit::{Unit, UnitType};

use super::ui::{label_bundle, tip_buttons, UiState};
use super::Art;
use crate::foreign::Foreign;
use crate::{GameState, Selection, PLAYER};

const SETTING: &str = "tips.json";

/// (id, title, text)
const TIPS: [(&str, &str, &str); 21] = [
    ("welcome", "Welcome", "Your people await your orders. Click one of your units to select it, then click a tile to send it there. Enter ends the turn. New to this kind of game? F1 opens How to play, a short guide to your first turns."),
    ("end_turn", "Ending the turn", "Every unit has its orders. Press Enter (or End Turn, bottom right) and the other peoples take their turns; then a new turn begins and your units can move again."),
    ("unit", "Moving units", "Hover over a tile to see the route and how many turns it takes. Far destinations become standing orders. H fortifies a unit so it is not asked again."),
    ("settler", "Settlers", "Settlers found new cities: move one to a good spot at least two tiles from other cities and press B. Rivers and coasts make fine sites."),
    ("explore", "Exploring", "Scouts move far and see far. Send them towards the dark edges of the map: they find good land, tribal villages (walk in for a gift) and other peoples."),
    ("ship", "Ships", "This ship can carry units. Move troops onto its tile (or press I in port) to board, then sail it; to land them, select a unit aboard and click a land tile next to the ship."),
    ("aboard", "At sea", "This unit is aboard a ship and travels with it. Click a land tile next to the ship to go ashore - that takes the rest of its turn. Troops cannot attack from a ship."),
    ("worker", "Workers", "Workers improve the land: F farm, M mine, R road (roads triple movement), X clear land. K lets the worker look after your land by itself."),
    ("city", "Your cities", "The city panel shows food, shields, science, gold and happiness. Choose what to build in the Units, Buildings and Wonders tabs. A city keeps training the same unit until you change it."),
    ("research", "Research", "Press Q for the science advisor: choose what your scholars study next, and see the whole tree of knowledge."),
    ("idle", "A city needs a project", "A city that finished a building waits for your next order. Space brings it up before the turn ends."),
    ("contact", "Other peoples", "You have met another people. Press E for the foreign advisor: treaties, trade in resources and technology, and their mood towards you."),
    ("disorder", "Unhappy cities", "A city in disorder produces nothing. Build a temple, connect luxuries, make some citizens entertainers (Overview tab) or station troops under Despotism or Monarchy."),
    ("great", "A great person", "A great person has been born. Select it and press U to use it: a scientist completes research, an engineer finishes a building, a merchant trades abroad, an artist makes a masterpiece."),
    ("caravan", "Caravans", "Move a caravan into another of your cities (6+ tiles away) or next to a foreign one and press T for a trade route, or U in a city building a wonder to add its shields."),
    ("diplomat", "Diplomats", "Move a diplomat next to a foreign city: open an embassy to see their research, or try a covert mission. If caught, they will not forget it."),
    ("war", "At war", "Attack by moving onto an enemy. Hills, forests, rivers, walls and fortifying help defenders; veterans fight harder. Armies cannot slip past enemy armies."),
    ("grow", "Growing cities", "A city grows when its food store fills: work food-rich tiles and build farms. Bigger cities work more tiles, but past a few citizens they need temples or luxuries to stay content."),
    ("score", "How to win", "There are five ways to win: conquer everyone, reach Alpha Centauri first, lead the world in culture, win the United Nations vote, or have the best score on the last turn. Press Y (Empire) to see how everyone stands."),
    ("defend", "Guard your cities", "A city with no troops inside falls to the first enemy that walks in. Keep at least one defender in each city - Spearmen and Phalanxes early, then Pikemen and Musketmen - and fortify it (H)."),
    ("save", "Saving", "The game saves itself every turn (Continue on the start menu). F2 downloads a file you can keep or send to a friend to load."),
];

#[derive(Resource)]
pub struct Tips {
    pub enabled: bool,
    pub shown: HashSet<&'static str>,
    pub current: Option<&'static str>,
}

impl Default for Tips {
    fn default() -> Self {
        Self { enabled: read_setting(SETTING).as_deref() != Some("off"), shown: HashSet::new(), current: None }
    }
}

impl Tips {
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.current = None;
        }
        write_setting(SETTING, if on { "on" } else { "off" });
    }

    /// Moves on from the tip on screen.
    pub fn done(&mut self) {
        self.current = None;
    }
}

/// The tip panel under the status bar.
#[derive(Component)]
pub struct TipPanel;

/// Chooses the next tip whose moment has come.
#[allow(clippy::too_many_arguments)]
pub fn queue_tips(
    mut tips: ResMut<Tips>,
    state: Res<GameState>,
    selection: Res<Selection>,
    units: Query<&Unit>,
    cities: Query<(&City, &Happiness)>,
    foreign: Res<Foreign>,
    diplomacy: Res<Diplomacy>,
    ui: Res<UiState>,
) {
    // No new tip while a window is open: it would only cover what the player is reading.
    if !tips.enabled || tips.current.is_some() || state.victory.is_some() || ui.modal.is_some() {
        return;
    }
    let selected = selection.unit.and_then(|e| units.get(e).ok()).filter(|u| u.owner == PLAYER).map(|u| u.unit_type);
    let ours = || units.iter().filter(|u| u.owner == PLAYER);
    let due = |id: &str| match id {
        "welcome" => true,
        "unit" => selected.is_some(),
        "settler" => selected == Some(UnitType::Settler),
        "end_turn" => state.turn <= 5 && ours().next().is_some() && !ours().any(crate::needs_orders),
        "explore" => selected == Some(UnitType::Scout),
        "ship" => selected.is_some_and(|u| u.capacity() > 0 && u != UnitType::Carrier),
        "aboard" => selection.unit.and_then(|e| units.get(e).ok()).is_some_and(|u| u.owner == PLAYER && u.carrier.is_some() && u.unit_type.domain() == bevy_civ::unit::Domain::Land),
        "grow" => state.turn >= 6,
        "score" => state.turn >= 20,
        "defend" => cities.iter().any(|(c, _)| c.owner == PLAYER && !ours().any(|u| u.position == c.position && u.unit_type.is_army())) && state.turn >= 3,
        "worker" => selected == Some(UnitType::Worker),
        "city" => selection.city.is_some(),
        "research" => state.turn >= 2,
        "idle" => cities.iter().any(|(c, _)| c.owner == PLAYER && c.production_queue.is_none()),
        "contact" => !foreign.met.is_empty() && state.turn > 1,
        "disorder" => cities.iter().any(|(c, h)| c.owner == PLAYER && h.in_disorder()),
        "great" => ours().any(|u| u.unit_type.is_great_person()),
        "caravan" => selected == Some(UnitType::Caravan),
        "diplomat" => selected == Some(UnitType::Diplomat),
        "war" => diplomacy.treaties.iter().any(|((a, b), t)| (*a == PLAYER || *b == PLAYER) && *t == Treaty::War),
        "save" => state.turn >= 10,
        _ => false,
    };
    if let Some((id, _, _)) = TIPS.iter().find(|(id, _, _)| !tips.shown.contains(id) && due(id)) {
        tips.shown.insert(id);
        tips.current = Some(id);
    }
}

/// Shows (or hides) the tip panel. It sits in the top-left corner, clear of the city panel and the
/// windows in the middle of the screen, and steps aside while a window is open.
pub fn show_tip(
    tips: Res<Tips>,
    ui: Res<UiState>,
    art: Res<Art>,
    screen: Res<super::layout::Screen>,
    extents: Res<super::layout::DockExtents>,
    selection: Res<Selection>,
    panel: Query<Entity, With<TipPanel>>,
    mut commands: Commands,
) {
    if !tips.is_changed() && !ui.is_changed() && !screen.is_changed() && !extents.is_changed() && !selection.is_changed() {
        return;
    }
    for entity in &panel {
        commands.entity(entity).despawn_recursive();
    }
    // A window, or on a phone an open city (a sheet over the screen), keeps the tip waiting.
    if ui.modal.is_some() || (screen.compact && selection.city.is_some()) {
        return;
    }
    let Some((_, title, text)) = tips.current.and_then(|id| TIPS.iter().find(|(t, _, _)| *t == id)) else { return };
    commands
        .spawn((
            TipPanel,
            Node {
                position_type: PositionType::Absolute,
                // Under the top bar: on the left on a big screen; on a phone held sideways, on the
                // right, clear of the unit panel; on a phone upright, across the top.
                top: Val::Px(extents.top + 8.0),
                left: if screen.compact && !screen.portrait { Val::Auto } else { Val::Px(8.0) },
                right: if screen.compact && !screen.portrait { Val::Px(8.0) } else { Val::Auto },
                width: Val::Px(if screen.compact && !screen.portrait { (screen.width * 0.42).min(340.0) } else { 380.0 }),
                max_width: Val::Percent(96.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.09, 0.07, 0.92)),
            BorderColor(Color::srgb(0.86, 0.68, 0.36)),
            BorderRadius::all(Val::Px(8.0)),
            Interaction::None,
            bevy::ui::FocusPolicy::Block,
            GlobalZIndex(20),
        ))
        .with_children(|p| {
            p.spawn(label_bundle(&art.font_title, format!("Tip: {title}"), 17.0, Color::srgb(0.95, 0.83, 0.52)));
            p.spawn(label_bundle(&art.font_body, *text, 14.0, Color::srgb(0.94, 0.9, 0.82)));
            tip_buttons(p, &art);
        });
}
