//! Unit and city sprites, their animation, and combat effects.

use bevy::prelude::*;
use bevy::sprite::Anchor;

use bevy_civ::city::City;
use bevy_civ::fog::FogOfWar;
use bevy_civ::unit::{Unit, UnitType};

use super::map::HEX_SPRITE;
use super::sfx::Sfx;
use super::{overlay, owner_color, Art};
use std::collections::{HashMap, VecDeque};

use hexx::Hex;

use crate::{Burst, MapData, Selection, Trail, PLAYER};

const UNIT_SIZE: f32 = 76.0;
/// Feet of the figure inside its 128px frame, relative to the frame centre.
const UNIT_ANCHOR: Anchor = Anchor::Custom(Vec2::new(0.0, -0.39));
const UNIT_OFFSET: Vec2 = Vec2::new(0.0, -10.0);
/// Row of the transport boat in the unit atlas.
const BOAT_ROW: usize = 19;
const CITY_OFFSET: Vec2 = Vec2::new(0.0, -8.0);

#[derive(Component)]
pub struct UnitView {
    facing_left: bool,
    phase: f32,
    /// The tile the unit is on in the game, which the figure is standing on or walking to.
    at: Hex,
    /// The last tile the figure itself reached (decides whether it is drawn in a boat).
    shown: Hex,
    /// Tiles the figure still has to pass through, with their positions.
    waypoints: VecDeque<(Hex, Vec2)>,
}
#[derive(Component)]
pub struct UnitBase;
#[derive(Component)]
pub struct CityFlag;
#[derive(Component)]
pub struct CityLabel;
#[derive(Component)]
pub struct CityLabelBack;
#[derive(Component)]
pub struct BurstFx(Timer);
/// The "×3" marker on a tile where several units stand.
#[derive(Component)]
pub struct StackBadge;

/// Units sharing a tile, front first: the selected unit, then the player's, then the rest.
fn stacks<'a>(units: impl Iterator<Item = (Entity, &'a Unit)>, selected: Option<Entity>) -> HashMap<Hex, Vec<Entity>> {
    let mut by_tile: HashMap<Hex, Vec<(Entity, u32)>> = HashMap::new();
    for (entity, unit) in units {
        by_tile.entry(unit.position).or_default().push((entity, unit.owner));
    }
    by_tile
        .into_iter()
        .map(|(hex, mut list)| {
            list.sort_by_key(|(e, owner)| (Some(*e) != selected, *owner != PLAYER, e.index()));
            (hex, list.into_iter().map(|(e, _)| e).collect())
        })
        .collect()
}

/// Where a unit stands within its tile: the front unit in the middle, the others fanned out behind it.
fn stack_offset(slot: usize) -> Vec2 {
    if slot == 0 {
        return Vec2::ZERO;
    }
    let side = if slot % 2 == 1 { -1.0 } else { 1.0 };
    let reach = ((slot + 1) / 2).min(3) as f32;
    Vec2::new(side * 17.0 * reach, 9.0 + 3.0 * reach)
}

/// Row of a unit type in the atlas; the transport boat sits between the land units and the rest.
pub fn unit_row(unit_type: UnitType) -> usize {
    let index = UnitType::ALL.iter().position(|u| *u == unit_type).unwrap_or(0);
    if index >= BOAT_ROW { index + 1 } else { index }
}

fn unit_z(y: f32) -> f32 {
    7.0 - y * 0.0005
}

pub fn attach_unit_views(mut commands: Commands, art: Res<Art>, map: Res<MapData>, units: Query<(Entity, &Unit), Added<Unit>>) {
    for (entity, unit) in &units {
        let pos = map.layout.hex_to_world_pos(unit.position) + UNIT_OFFSET;
        commands
            .entity(entity)
            .insert((
                Sprite {
                    image: art.units.clone(),
                    texture_atlas: Some(art.atlas(&art.units_layout, unit_row(unit.unit_type) * 4)),
                    custom_size: Some(Vec2::splat(UNIT_SIZE)),
                    anchor: UNIT_ANCHOR,
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, unit_z(pos.y)),
                UnitView { facing_left: unit.owner != PLAYER, phase: (entity.index() % 7) as f32 * 0.6, at: unit.position, shown: unit.position, waypoints: VecDeque::new() },
            ))
            .with_children(|parent| {
                // Team-coloured disc under the figure.
                parent.spawn((
                    UnitBase,
                    Sprite {
                        image: art.overlays.clone(),
                        texture_atlas: Some(art.atlas(&art.overlays_layout, overlay::BASE)),
                        custom_size: Some(HEX_SPRITE * 0.72),
                        color: owner_color(unit.owner),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 5.0, -0.02),
                ));
            });
    }
}

pub fn animate_units(
    time: Res<Time>,
    map: Res<MapData>,
    fog: Res<FogOfWar>,
    selection: Res<Selection>,
    mut units: Query<(Entity, &Unit, &mut Transform, &mut Sprite, &mut UnitView, &mut Visibility, &Children, Option<&mut Trail>)>,
    mut bases: Query<&mut Sprite, (With<UnitBase>, Without<UnitView>)>,
) {
    let place = |hex: Hex| map.layout.hex_to_world_pos(hex) + UNIT_OFFSET;
    let stacked = stacks(units.iter().map(|(e, u, ..)| (e, u)), selection.unit);
    for (entity, unit, mut transform, mut sprite, mut view, mut visibility, children, trail) in &mut units {
        let slot = stacked.get(&unit.position).and_then(|s| s.iter().position(|e| *e == entity)).unwrap_or(0);
        let target = place(unit.position) + stack_offset(slot);
        // The unit moved: walk the figure through every tile on the way, following the route it took
        // when that is known (the player's orders) and a straight line of tiles otherwise.
        if unit.position != view.at {
            let steps: Vec<Hex> = match trail {
                Some(mut trail) if trail.0.last() == Some(&unit.position) => std::mem::take(&mut trail.0),
                _ => view.at.line_to(unit.position).skip(1).collect(),
            };
            let watched = unit.owner == PLAYER || fog.visible.contains(&unit.position) || fog.visible.contains(&view.at);
            if watched && steps.len() <= 16 {
                view.waypoints.extend(steps.into_iter().map(|hex| (hex, place(hex))));
            } else {
                view.waypoints.clear();
                view.shown = unit.position;
                transform.translation = target.extend(unit_z(target.y));
            }
            view.at = unit.position;
        }
        let (goal_hex, goal) = view.waypoints.front().copied().unwrap_or((unit.position, target));
        let here = transform.translation.truncate();
        let delta = goal - here;
        let distance = delta.length();
        let moving = distance > 0.5 || !view.waypoints.is_empty();
        if distance > 0.5 {
            // Long marches play a little faster so a turn of moves never drags.
            let speed = 280.0 + 70.0 * view.waypoints.len() as f32;
            let step = (speed * time.delta_secs()).min(distance);
            let next = here + delta / distance * step;
            transform.translation = next.extend(unit_z(next.y));
            if delta.x.abs() > 1.0 {
                view.facing_left = delta.x < 0.0;
            }
        } else {
            transform.translation = goal.extend(unit_z(goal.y));
            view.shown = goal_hex;
            view.waypoints.pop_front();
        }
        sprite.flip_x = view.facing_left;
        let speed = if moving { 11.0 } else if unit.build_turns_left.is_some() { 7.0 } else { 3.5 };
        let frame = (time.elapsed_secs() * speed + view.phase) as usize % 4;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            // Land units are shown as a transport boat while at sea.
            let embarked = unit.unit_type.domain() == bevy_civ::unit::Domain::Land && map.water.contains(&view.shown);
            let row = if embarked { BOAT_ROW } else { unit_row(unit.unit_type) };
            atlas.index = row * 4 + frame;
        }
        let shown = unit.owner == PLAYER || fog.visible.contains(&unit.position);
        *visibility = if shown { Visibility::Inherited } else { Visibility::Hidden };
        // Units that have spent their moves get a faded disc.
        let spent = unit.owner == PLAYER && !unit.can_move();
        for child in children.iter() {
            if let Ok(mut base) = bases.get_mut(*child) {
                base.color = owner_color(unit.owner).with_alpha(if spent { 0.45 } else { 1.0 });
            }
        }
    }
}

fn city_frame(population: u32) -> usize {
    match population {
        0..=2 => 0,
        3..=5 => 1,
        6..=9 => 2,
        _ => 3,
    }
}

pub fn attach_city_views(mut commands: Commands, art: Res<Art>, map: Res<MapData>, cities: Query<(Entity, &City), Added<City>>) {
    for (entity, city) in &cities {
        let pos = map.layout.hex_to_world_pos(city.position) + CITY_OFFSET;
        commands
            .entity(entity)
            .insert((
                Sprite {
                    image: art.cities.clone(),
                    texture_atlas: Some(art.atlas(&art.cities_layout, city_frame(city.population))),
                    custom_size: Some(Vec2::splat(104.0)),
                    anchor: Anchor::Custom(Vec2::new(0.0, -0.32)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 5.0 - pos.y * 0.0005),
            ))
            .with_children(|parent| {
                parent.spawn((
                    CityFlag,
                    Sprite {
                        image: art.flag.clone(),
                        texture_atlas: Some(art.atlas(&art.flag_layout, 0)),
                        custom_size: Some(Vec2::splat(34.0)),
                        ..default()
                    },
                    Transform::from_xyz(36.0, 46.0, 0.01),
                ));
                parent.spawn((
                    CityLabelBack,
                    Sprite { color: Color::srgba(0.05, 0.06, 0.08, 0.82), custom_size: Some(Vec2::new(80.0, 19.0)), ..default() },
                    Transform::from_xyz(0.0, -24.0, 3.0),
                ));
                parent.spawn((
                    CityLabel,
                    Text2d::new(""),
                    TextFont { font: art.font_bold.clone(), font_size: 15.0, ..default() },
                    TextColor(Color::WHITE),
                    Transform::from_xyz(0.0, -24.0, 3.1),
                ));
            });
    }
}

pub fn update_city_views(
    fog: Res<FogOfWar>,
    mut cities: Query<(&City, &mut Sprite, &mut Visibility, &Children)>,
    mut flags: Query<&mut Sprite, (With<CityFlag>, Without<City>, Without<CityLabelBack>)>,
    mut backs: Query<&mut Sprite, (With<CityLabelBack>, Without<City>, Without<CityFlag>)>,
    mut labels: Query<(&mut Text2d, &mut TextColor), With<CityLabel>>,
) {
    for (city, mut sprite, mut visibility, children) in &mut cities {
        *visibility = if fog.explored.contains(&city.position) { Visibility::Inherited } else { Visibility::Hidden };
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = city_frame(city.population);
        }
        let text = format!("{} {}", city.population, city.name);
        for child in children.iter() {
            if let Ok(mut flag) = flags.get_mut(*child) {
                flag.color = owner_color(city.owner);
            }
            if let Ok(mut back) = backs.get_mut(*child) {
                back.custom_size = Some(Vec2::new(text.len() as f32 * 7.6 + 14.0, 19.0));
            }
            if let Ok((mut label, mut color)) = labels.get_mut(*child) {
                if label.0 != text {
                    label.0 = text.clone();
                }
                color.0 = owner_color(city.owner).mix(&Color::WHITE, 0.6);
            }
        }
    }
}

pub fn animate_flags(time: Res<Time>, mut flags: Query<&mut Sprite, With<CityFlag>>) {
    let frame = (time.elapsed_secs() * 7.0) as usize % 4;
    for mut sprite in &mut flags {
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = frame;
        }
    }
}

/// Marks tiles where several units stand with their number, so a stack is never mistaken for one unit.
pub fn update_stack_badges(
    art: Res<Art>,
    map: Res<MapData>,
    fog: Res<FogOfWar>,
    units: Query<&Unit>,
    changed: Query<(), Changed<Unit>>,
    mut removed: RemovedComponents<Unit>,
    badges: Query<Entity, With<StackBadge>>,
    mut commands: Commands,
) {
    let units_gone = !removed.is_empty();
    removed.clear();
    if changed.is_empty() && !units_gone && !fog.is_changed() {
        return;
    }
    for badge in &badges {
        commands.entity(badge).despawn_recursive();
    }
    let mut counts: HashMap<Hex, (usize, u32)> = HashMap::new();
    for unit in units.iter().filter(|u| u.owner == PLAYER || fog.visible.contains(&u.position)) {
        let entry = counts.entry(unit.position).or_insert((0, unit.owner));
        entry.0 += 1;
    }
    for (hex, (count, owner)) in counts.into_iter().filter(|(_, (n, _))| *n > 1) {
        let pos = map.layout.hex_to_world_pos(hex) + Vec2::new(-44.0, 30.0);
        commands
            .spawn((
                StackBadge,
                Sprite {
                    image: art.overlays.clone(),
                    texture_atlas: Some(art.atlas(&art.overlays_layout, overlay::DOT)),
                    custom_size: Some(HEX_SPRITE * 1.9),
                    color: owner_color(owner).mix(&Color::BLACK, 0.55),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 8.7),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text2d::new(format!("×{count}")),
                    TextFont { font: art.font_bold.clone(), font_size: 16.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.95, 0.8)),
                    Transform::from_xyz(0.0, 1.0, 0.1),
                ));
            });
    }
}

pub fn spawn_bursts(
    mut events: EventReader<Burst>,
    mut commands: Commands,
    art: Res<Art>,
    map: Res<MapData>,
    fog: Res<FogOfWar>,
    mut sounds: EventWriter<Sfx>,
) {
    for Burst(hex) in events.read() {
        if fog.visible.contains(hex) {
            sounds.send(Sfx::Battle);
        }
        let pos = map.layout.hex_to_world_pos(*hex);
        commands.spawn((
            BurstFx(Timer::from_seconds(0.07, TimerMode::Repeating)),
            Sprite {
                image: art.burst.clone(),
                texture_atlas: Some(art.atlas(&art.burst_layout, 0)),
                custom_size: Some(Vec2::splat(110.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y + 8.0, 9.0),
        ));
    }
}

pub fn animate_bursts(time: Res<Time>, mut commands: Commands, mut bursts: Query<(Entity, &mut BurstFx, &mut Sprite)>) {
    for (entity, mut fx, mut sprite) in &mut bursts {
        if !fx.0.tick(time.delta()).just_finished() {
            continue;
        }
        let Some(atlas) = sprite.texture_atlas.as_mut() else { continue };
        if atlas.index >= 5 {
            commands.entity(entity).despawn();
        } else {
            atlas.index += 1;
        }
    }
}
