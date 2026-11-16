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
const TIPS: [(&str, &str, &str); 14] = [
    ("welcome", "Welcome", "Your people await your orders. Click one of your units to select it, then click a tile to send it there. Enter ends the turn; F1 opens the encyclopedia."),
    ("unit", "Moving units", "Hover over a tile to see the route and how many turns it takes. Far destinations become standing orders. H fortifies a unit so it is not asked again."),
    ("settler", "Settlers", "Settlers found new cities: move one to a good spot at least two tiles from other cities and press B. Rivers and coasts make fine sites."),
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
pub fn show_tip(tips: Res<Tips>, ui: Res<UiState>, art: Res<Art>, panel: Query<Entity, With<TipPanel>>, mut commands: Commands) {
    if !tips.is_changed() && !ui.is_changed() {
        return;
    }
    for entity in &panel {
        commands.entity(entity).despawn_recursive();
    }
    if ui.modal.is_some() {
        return;
    }
    let Some((_, title, text)) = tips.current.and_then(|id| TIPS.iter().find(|(t, _, _)| *t == id)) else { return };
    commands
        .spawn((
            TipPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(62.0),
                left: Val::Px(10.0),
                width: Val::Px(380.0),
                max_width: Val::Percent(94.0),
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
