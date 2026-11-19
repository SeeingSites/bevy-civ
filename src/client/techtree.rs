//! The tree of knowledge drawn as a diagram: each technology a box, in columns by how many steps of
//! study lie before it, with a line from every prerequisite to what it opens. Lines are painted into
//! an image when research changes; the boxes (buttons that set a research goal) are laid over it by
//! the science advisor at the places [`TreeLayout`] gives.

use std::collections::HashMap;

use bevy::prelude::*;

use bevy_civ::tech::{Tech, TechState};

use super::ui::{Modal, UiState};

/// Width of a column (a box and the gap its lines cross), and of a box.
pub const COLUMN: f32 = 150.0;
pub const BOX_WIDTH: f32 = 122.0;
/// Height of a row, and of a box.
pub const ROW: f32 = 46.0;
pub const BOX_HEIGHT: f32 = 34.0;
const PAD: f32 = 8.0;

/// Where every technology's box sits (top-left corner, pixels) and the size of the whole diagram.
#[derive(Resource, Clone)]
pub struct TreeLayout {
    pub at: HashMap<Tech, Vec2>,
    pub size: UVec2,
}

impl TreeLayout {
    /// Columns by depth (the longest chain of prerequisites before a technology); within a column,
    /// each technology near the middle of its prerequisites, so lines run as straight as they can.
    pub fn new() -> Self {
        fn depth(t: Tech, memo: &mut HashMap<Tech, usize>) -> usize {
            if let Some(d) = memo.get(&t) {
                return *d;
            }
            let d = t
                .prerequisites()
                .iter()
                .map(|p| depth(*p, memo) + 1)
                .max()
                .unwrap_or(0);
            memo.insert(t, d);
            d
        }
        let mut memo = HashMap::new();
        let mut columns: Vec<Vec<Tech>> = Vec::new();
        for t in Tech::ALL {
            let d = depth(t, &mut memo);
            if columns.len() <= d {
                columns.resize(d + 1, Vec::new());
            }
            columns[d].push(t);
        }
        let tallest = columns.iter().map(|c| c.len()).max().unwrap_or(1);
        let mut row_of: HashMap<Tech, f32> = HashMap::new();
        let mut at = HashMap::new();
        for (c, column) in columns.iter_mut().enumerate() {
            // Order by the average row of the prerequisites (first column: as listed).
            let key = |t: &Tech, rows: &HashMap<Tech, f32>| {
                let pre = t.prerequisites();
                if pre.is_empty() {
                    0.0
                } else {
                    pre.iter()
                        .map(|p| rows.get(p).copied().unwrap_or(0.0))
                        .sum::<f32>()
                        / pre.len() as f32
                }
            };
            if c > 0 {
                column.sort_by(|a, b| key(a, &row_of).total_cmp(&key(b, &row_of)));
            }
            let offset = (tallest - column.len()) as f32 / 2.0;
            for (r, t) in column.iter().enumerate() {
                let row = offset + r as f32;
                row_of.insert(*t, row);
                at.insert(*t, Vec2::new(PAD + c as f32 * COLUMN, PAD + row * ROW));
            }
        }
        let size = UVec2::new(
            (2.0 * PAD + (columns.len() as f32 - 1.0) * COLUMN + BOX_WIDTH) as u32,
            (2.0 * PAD + (tallest as f32 - 1.0) * ROW + BOX_HEIGHT) as u32,
        );
        Self { at, size }
    }
}

/// The image the lines are painted into.
#[derive(Resource)]
pub struct TreeImage(pub Handle<Image>);

pub fn setup_tree(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let layout = TreeLayout::new();
    commands.insert_resource(TreeImage(
        images.add(super::paint::blank(layout.size.x, layout.size.y)),
    ));
    commands.insert_resource(layout);
}

/// Line colours: a link already walked, a link on the way to the goal, and the rest.
const KNOWN: [u8; 4] = [70, 140, 70, 230];
const ON_PATH: [u8; 4] = [225, 120, 40, 255];
const AHEAD: [u8; 4] = [120, 100, 75, 140];

/// Repaints the lines while the science advisor is open and research has changed.
pub fn draw_tree(
    ui: Res<UiState>,
    tech: Res<TechState>,
    layout: Option<Res<TreeLayout>>,
    image: Option<Res<TreeImage>>,
    mut images: ResMut<Assets<Image>>,
    mut drawn: Local<Option<(usize, Option<Tech>)>>,
) {
    let (Some(layout), Some(image)) = (layout, image) else {
        return;
    };
    if ui.modal != Some(Modal::Science) {
        *drawn = None;
        return;
    }
    let key = (tech.researched.len(), tech.goal);
    if *drawn == Some(key) {
        return;
    }
    let Some(picture) = images.get_mut(&image.0) else {
        return;
    };
    picture.data.fill(0);
    let path = tech.goal.map(|g| tech.path_to(g)).unwrap_or_default();
    let on_path = |t: &Tech| path.contains(t);
    let width = layout.size.x;
    // The ordinary lines first, so the path is drawn on top.
    for pass in 0..2 {
        for t in Tech::ALL {
            let Some(to) = layout.at.get(&t) else {
                continue;
            };
            for p in t.prerequisites() {
                let Some(from) = layout.at.get(&p) else {
                    continue;
                };
                let goal_link = on_path(&t) && (on_path(&p) || tech.researched.contains(&p));
                if goal_link != (pass == 1) {
                    continue;
                }
                let (colour, thick) = if goal_link {
                    (ON_PATH, 3.0)
                } else if tech.researched.contains(&t) {
                    (KNOWN, 2.0)
                } else {
                    (AHEAD, 1.5)
                };
                let start = *from + Vec2::new(BOX_WIDTH, BOX_HEIGHT / 2.0);
                let end = *to + Vec2::new(0.0, BOX_HEIGHT / 2.0);
                super::paint::elbow(&mut picture.data, width, start, end, thick, colour);
            }
        }
    }
    *drawn = Some(key);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_technology_stands_to_the_right_of_what_it_needs() {
        let layout = TreeLayout::new();
        assert_eq!(layout.at.len(), Tech::ALL.len());
        for t in Tech::ALL {
            for p in t.prerequisites() {
                assert!(layout.at[&p].x < layout.at[&t].x, "{p:?} before {t:?}");
            }
            let at = layout.at[&t];
            assert!(
                at.x + BOX_WIDTH <= layout.size.x as f32
                    && at.y + BOX_HEIGHT <= layout.size.y as f32
            );
        }
        // No two boxes share a place.
        let mut places: Vec<(i32, i32)> = layout
            .at
            .values()
            .map(|v| (v.x as i32, v.y as i32))
            .collect();
        places.sort();
        places.dedup();
        assert_eq!(places.len(), Tech::ALL.len());
    }
}
