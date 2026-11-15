//! Unit and city sprites, their animation, and combat effects.

use bevy::prelude::*;
use bevy::sprite::Anchor;

use bevy_civ::city::City;
use bevy_civ::fog::FogOfWar;
use bevy_civ::unit::{Unit, UnitType};

use super::map::HEX_SPRITE;
use super::{overlay, owner_color, Art};
use crate::{Burst, MapData, PLAYER};

const UNIT_SIZE: f32 = 76.0;
/// Feet of the figure inside its 128px frame, relative to the frame centre.
const UNIT_ANCHOR: Anchor = Anchor::Custom(Vec2::new(0.0, -0.39));
const UNIT_OFFSET: Vec2 = Vec2::new(0.0, -10.0);
const CITY_OFFSET: Vec2 = Vec2::new(0.0, -8.0);

#[derive(Component)]
pub struct UnitView {
    facing_left: bool,
    phase: f32,
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

pub fn unit_row(unit_type: UnitType) -> usize {
    UnitType::ALL.iter().position(|u| *u == unit_type).unwrap_or(0)
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
                UnitView { facing_left: unit.owner != PLAYER, phase: (entity.index() % 7) as f32 * 0.6 },
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
    mut units: Query<(&Unit, &mut Transform, &mut Sprite, &mut UnitView, &mut Visibility, &Children)>,
    mut bases: Query<&mut Sprite, (With<UnitBase>, Without<UnitView>)>,
) {
    for (unit, mut transform, mut sprite, mut view, mut visibility, children) in &mut units {
        let target = map.layout.hex_to_world_pos(unit.position) + UNIT_OFFSET;
        let delta = target - transform.translation.truncate();
        let distance = delta.length();
        let moving = distance > 0.5;
        if moving {
            let step = (300.0 * time.delta_secs()).min(distance);
            let next = transform.translation.truncate() + delta / distance * step;
            transform.translation = next.extend(unit_z(next.y));
            if delta.x.abs() > 1.0 {
                view.facing_left = delta.x < 0.0;
            }
        } else {
            transform.translation = target.extend(unit_z(target.y));
        }
        sprite.flip_x = view.facing_left;
        let speed = if moving { 11.0 } else if unit.build_turns_left.is_some() { 7.0 } else { 3.5 };
        let frame = (time.elapsed_secs() * speed + view.phase) as usize % 4;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = unit_row(unit.unit_type) * 4 + frame;
        }
        let shown = unit.owner == PLAYER || fog.visible.contains(&unit.position);
        *visibility = if shown { Visibility::Inherited } else { Visibility::Hidden };
        // Units that have spent their moves get a faded disc.
        let spent = unit.owner == PLAYER && unit.movement_left == 0;
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

pub fn spawn_bursts(mut events: EventReader<Burst>, mut commands: Commands, art: Res<Art>, map: Res<MapData>) {
    for Burst(hex) in events.read() {
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
