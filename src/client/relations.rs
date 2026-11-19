//! The web of relations drawn for the foreign advisor: every people we know (and us) on a ring,
//! joined by a line for each treaty - red for war, green for alliance, a thin line for peace, a
//! faint thread where there is none. The
//! lines are painted into a picture whenever they change; the portraits are laid over it by the
//! advisor window (see `ui::relations_page`), at the places `positions` gives.

use bevy::prelude::*;

use bevy_civ::ai::Civilizations;
use bevy_civ::diplomacy::{Diplomacy, Treaty};

use super::ui::{DiploPage, Modal, UiState};
use crate::foreign::Foreign;
use crate::PLAYER;

/// Width and height of the picture, in pixels.
pub const WEB: u32 = 360;

/// The picture the lines are painted into.
#[derive(Resource)]
pub struct RelationsWeb(pub Handle<Image>);

pub fn setup_web(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(RelationsWeb(images.add(super::paint::blank(WEB, WEB))));
}

/// The peoples on the web, in order: us, then everyone we know.
pub fn web_ids(civs: &Civilizations, foreign: &Foreign) -> Vec<u32> {
    std::iter::once(PLAYER)
        .chain(
            civs.players
                .iter()
                .filter(|c| {
                    !c.is_human && !foreign.gone.contains(&c.id) && foreign.met.contains(&c.id)
                })
                .map(|c| c.id),
        )
        .collect()
}

/// Where each of `n` peoples sits: round a ring, us at the top.
pub fn positions(n: usize) -> Vec<Vec2> {
    let centre = Vec2::splat(WEB as f32 / 2.0);
    let radius = WEB as f32 / 2.0 - 40.0;
    (0..n)
        .map(|i| {
            let angle =
                -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / n.max(1) as f32;
            centre + Vec2::new(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

/// Colour and width of the line for a treaty.
fn stroke(treaty: Treaty) -> Option<([u8; 4], f32)> {
    match treaty {
        Treaty::War => Some(([200, 45, 35, 255], 3.0)),
        Treaty::Alliance => Some(([55, 150, 55, 255], 3.0)),
        Treaty::Peace => Some(([125, 105, 75, 220], 1.6)),
        // No treaty: a faint thread, so the ring reads as a web.
        Treaty::None => Some(([125, 105, 75, 60], 0.8)),
    }
}

/// Repaints the lines while the relations page is open and something changed.
pub fn draw_web(
    ui: Res<UiState>,
    diplomacy: Res<Diplomacy>,
    foreign: Res<Foreign>,
    civs: Res<Civilizations>,
    web: Option<Res<RelationsWeb>>,
    mut images: ResMut<Assets<Image>>,
    mut drawn: Local<Vec<(usize, usize, Treaty)>>,
) {
    let Some(web) = web else { return };
    if ui.modal != Some(Modal::Diplomacy) || ui.diplo_page != DiploPage::Relations {
        drawn.clear();
        return;
    }
    let ids = web_ids(&civs, &foreign);
    let mut links = Vec::new();
    for i in 0..ids.len() {
        for j in i + 1..ids.len() {
            links.push((i, j, diplomacy.get_treaty(ids[i], ids[j])));
        }
    }
    // Lines are keyed by place on the ring, so a change of who is shown also repaints.
    links.push((ids.len(), 0, Treaty::None));
    if *drawn == links {
        return;
    }
    let Some(image) = images.get_mut(&web.0) else {
        return;
    };
    image.data.fill(0);
    let at = positions(ids.len());
    // Peace first, so war and alliance lines lie on top.
    let mut order: Vec<&(usize, usize, Treaty)> =
        links.iter().filter(|(i, _, _)| *i < ids.len()).collect();
    order.sort_by_key(|(_, _, t)| matches!(t, Treaty::War | Treaty::Alliance));
    for (i, j, treaty) in order {
        if let Some((colour, width)) = stroke(*treaty) {
            super::paint::line(&mut image.data, WEB, at[*i], at[*j], width, colour);
        }
    }
    *drawn = links;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_puts_us_at_the_top_and_lines_are_painted() {
        let at = positions(4);
        assert!(at[0].y < at[2].y && (at[0].x - WEB as f32 / 2.0).abs() < 0.01);
        let mut data = vec![0u8; (WEB * WEB * 4) as usize];
        super::super::paint::line(
            &mut data,
            WEB,
            Vec2::new(10.0, 10.0),
            Vec2::new(100.0, 10.0),
            3.0,
            [255, 0, 0, 255],
        );
        let pixel = |x: u32, y: u32| data[((y * WEB + x) * 4) as usize];
        assert_eq!(pixel(50, 10), 255);
        assert_eq!(pixel(50, 40), 0);
    }
}
