//! Play milestones for the page's opt-in statistics: a game starting (or resuming), every 25th turn
//! and how it ends. In the browser each one goes to `window.bevyCivEvent` in `web/index.html`, which
//! sends it on only if the player ticked "share anonymous play statistics"; elsewhere nothing happens.
//! Nothing here identifies the player.

use bevy::prelude::*;

use crate::scenario::NewGame;
use crate::{GameState, PLAYER};

/// Turns between progress reports.
pub const EVERY: u32 = 25;

/// What has been reported for the game in play: the last turn seen, and whether its end was told.
#[derive(Default)]
pub struct Reported {
    turn: Option<u32>,
    ended: bool,
}

pub fn report_milestones(
    state: Option<Res<GameState>>,
    game: Option<Res<NewGame>>,
    cities: Query<&bevy_civ::city::City>,
    mut reported: Local<Reported>,
) {
    let Some(state) = state else {
        *reported = Reported::default();
        return;
    };
    let scenario = game.as_ref().map(|g| g.scenario.id.clone()).unwrap_or_else(|| "saved".into());
    let difficulty = game.as_ref().map(|g| g.difficulty.name()).unwrap_or("unknown");
    let ours = cities.iter().filter(|c| c.owner == PLAYER).count();
    let detail = |extra: String| {
        format!(
            r#"{{"civ":{:?},"difficulty":{difficulty:?},"cities":{ours},"score":{},"government":{:?}{extra}}}"#,
            state.title,
            state.score.total,
            state.government.name()
        )
    };
    match reported.turn {
        // A new game, or one continued from a save.
        None => {
            let resumed = if state.turn > 1 { r#","resumed":true"# } else { "" };
            emit("game_start", &scenario, state.turn, &detail(resumed.into()));
        }
        Some(last) if last != state.turn => {
            if state.turn < last {
                // Another game began (from the menu) without leaving this screen.
                *reported = Reported::default();
                emit("game_start", &scenario, state.turn, &detail(String::new()));
            } else if state.turn % EVERY == 0 {
                emit("turn", &scenario, state.turn, &detail(String::new()));
            }
        }
        _ => {}
    }
    reported.turn = Some(state.turn);
    if let (Some(result), false) = (&state.victory, reported.ended) {
        reported.ended = true;
        let won = !result.starts_with("Defeat") && !result.contains("fallen");
        emit("game_over", &scenario, state.turn, &detail(format!(r#","won":{won},"result":{result:?}"#)));
    }
}

#[cfg(target_arch = "wasm32")]
fn emit(kind: &str, scenario: &str, turn: u32, detail: &str) {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        // Defined by the page; a no-op until the player opts in.
        #[wasm_bindgen(js_name = bevyCivEvent, catch)]
        fn js_event(kind: &str, scenario: &str, turn: u32, detail: &str) -> Result<(), JsValue>;
    }
    let _ = js_event(kind, scenario, turn, detail);
}

#[cfg(not(target_arch = "wasm32"))]
fn emit(_kind: &str, _scenario: &str, _turn: u32, _detail: &str) {}
