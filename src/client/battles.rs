//! Showing battles: each one is played out in turn instead of flashing past. The camera goes to
//! battles the player fought (including attacks on the player's units during the other peoples'
//! turns), the attacker lunges at the defender, blows land, and the result floats over the tile.
//! A unit that died in the battle is shown by a stand-in figure that falls and fades. The next unit
//! is not selected until the battles are over.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy::sprite::Anchor;

use bevy_civ::fog::FogOfWar;

use super::actors::unit_row;
use super::{owner_color, Art};
use crate::{BattleShown, Burst, Focus, MapData};

/// How long one battle is shown (shorter when several are waiting).
const BATTLE_SECONDS: f32 = 1.4;
const QUICK_BATTLE_SECONDS: f32 = 0.8;

/// Battles waiting to be shown, and the one on screen.
#[derive(Resource, Default)]
pub struct BattleTheatre {
    queue: VecDeque<BattleShown>,
    current: Option<(BattleShown, Timer, u8)>,
}

impl BattleTheatre {
    /// A battle is being shown or waits to be.
    pub fn busy(&self) -> bool {
        self.current.is_some() || !self.queue.is_empty()
    }
}

/// An attacker's lunge: the figure darts at the enemy and back.
#[derive(Component)]
pub struct Lunge {
    toward: Vec2,
    elapsed: f32,
    duration: f32,
    /// The offset added last frame (taken off before the figure's own movement runs).
    pub applied: Vec2,
}

impl Lunge {
    /// This frame's offset from the figure's own place.
    pub fn offset(&mut self, dt: f32) -> Option<Vec2> {
        self.elapsed += dt;
        if self.elapsed >= self.duration {
            return None;
        }
        let t = self.elapsed / self.duration;
        // Two jabs: out and back, twice.
        let reach = (t * std::f32::consts::TAU).sin().abs() * 22.0;
        Some(self.toward * reach)
    }
}

/// A stand-in for a unit that died in the battle: it shakes, falls and fades.
#[derive(Component)]
pub struct Fallen {
    timer: Timer,
}

/// The result of a battle, rising and fading over the tile.
#[derive(Component)]
pub struct BattleText {
    timer: Timer,
}

/// Collects the battles of this frame (only those worth showing: the player's and those in view).
pub fn queue_battles(
    mut events: EventReader<BattleShown>,
    fog: Res<FogOfWar>,
    mut theatre: ResMut<BattleTheatre>,
) {
    for battle in events.read() {
        if battle.ours || fog.visible.contains(&battle.at) || fog.visible.contains(&battle.from) {
            theatre.queue.push_back(battle.clone());
        }
    }
}

/// Plays the battles one after another.
#[allow(clippy::too_many_arguments)]
pub fn stage_battles(
    time: Res<Time>,
    mut theatre: ResMut<BattleTheatre>,
    map: Res<MapData>,
    art: Res<Art>,
    mut focus: EventWriter<Focus>,
    mut bursts: EventWriter<Burst>,
    mut commands: Commands,
) {
    if theatre.current.is_none() {
        let Some(battle) = theatre.queue.pop_front() else {
            return;
        };
        let seconds = if theatre.queue.len() >= 3 {
            QUICK_BATTLE_SECONDS
        } else {
            BATTLE_SECONDS
        };
        if battle.ours {
            focus.send(Focus(battle.at));
        }
        let from = map.layout.hex_to_world_pos(battle.from);
        let at = map.layout.hex_to_world_pos(battle.at);
        let toward = (at - from).normalize_or_zero();
        // The attacker lunges (if it still stands), or its stand-in does.
        let lunge = Lunge {
            toward,
            elapsed: 0.0,
            duration: seconds * 0.6,
            applied: Vec2::ZERO,
        };
        match battle.attacker {
            // The attacker may be gone by the time its battle is shown (the queue plays later):
            // then its stand-in lunges instead.
            Some(entity) if !battle.attacker_died && commands.get_entity(entity).is_some() => {
                commands.entity(entity).try_insert(lunge);
            }
            _ => {
                spawn_fallen(
                    &mut commands,
                    &art,
                    battle.attacker_side,
                    from,
                    seconds,
                    Some(lunge),
                );
            }
        }
        if battle.defender_died {
            spawn_fallen(&mut commands, &art, battle.defender_side, at, seconds, None);
        }
        // The result floats up from the tile.
        let colour = match battle.good {
            Some(true) => Color::srgb(0.55, 1.0, 0.5),
            Some(false) => Color::srgb(1.0, 0.45, 0.4),
            None => Color::srgb(0.98, 0.92, 0.75),
        };
        commands.spawn((
            BattleText {
                timer: Timer::from_seconds(seconds + 0.6, TimerMode::Once),
            },
            Text2d::new(battle.result.clone()),
            TextFont {
                font: art.font_bold.clone(),
                font_size: 20.0,
                ..default()
            },
            TextColor(colour),
            Transform::from_xyz(at.x, at.y + 46.0, 30.0),
        ));
        bursts.send(Burst(battle.at));
        theatre.current = Some((battle, Timer::from_seconds(seconds, TimerMode::Once), 1));
        return;
    }
    let Some((battle, timer, blows)) = theatre.current.as_mut() else {
        return;
    };
    timer.tick(time.delta());
    // Three exchanges of blows over the battle.
    let due = (timer.fraction() * 3.0) as u8 + 1;
    if due > *blows && *blows < 3 {
        *blows += 1;
        bursts.send(Burst(battle.at));
    }
    if timer.finished() {
        theatre.current = None;
    }
}

fn spawn_fallen(
    commands: &mut Commands,
    art: &Art,
    (kind, owner): (bevy_civ::unit::UnitType, u32),
    at: Vec2,
    seconds: f32,
    lunge: Option<Lunge>,
) {
    let mut entity = commands.spawn((
        Fallen {
            timer: Timer::from_seconds(seconds + 0.4, TimerMode::Once),
        },
        Sprite {
            image: art.units.clone(),
            texture_atlas: Some(art.atlas(&art.units_layout, unit_row(kind) * 4)),
            custom_size: Some(Vec2::splat(76.0)),
            anchor: Anchor::Custom(Vec2::new(0.0, -0.39)),
            flip_x: owner != crate::PLAYER,
            ..default()
        },
        Transform::from_xyz(at.x, at.y - 10.0, 8.0),
    ));
    entity.with_children(|p| {
        p.spawn((
            Sprite {
                color: owner_color(owner).with_alpha(0.8),
                custom_size: Some(Vec2::new(30.0, 6.0)),
                ..default()
            },
            Transform::from_xyz(0.0, -2.0, -0.01),
        ));
    });
    if let Some(lunge) = lunge {
        entity.insert(lunge);
    }
}

/// Stand-ins of the fallen tip over and fade once the blows are struck.
pub fn animate_fallen(
    time: Res<Time>,
    mut fallen: Query<(
        Entity,
        &mut Fallen,
        &mut Sprite,
        &mut Transform,
        Option<&mut Lunge>,
    )>,
    mut commands: Commands,
) {
    for (entity, mut f, mut sprite, mut transform, lunge) in &mut fallen {
        f.timer.tick(time.delta());
        if let Some(mut lunge) = lunge {
            transform.translation -= lunge.applied.extend(0.0);
            lunge.applied = lunge.offset(time.delta_secs()).unwrap_or(Vec2::ZERO);
            transform.translation += lunge.applied.extend(0.0);
        }
        let t = f.timer.fraction();
        // Standing through the fight, then falling and fading in the last third.
        let fall = ((t - 0.6) / 0.4).clamp(0.0, 1.0);
        transform.rotation =
            Quat::from_rotation_z(-fall * 1.4 * if sprite.flip_x { -1.0 } else { 1.0 });
        sprite.color = Color::srgba(1.0, 1.0, 1.0, 1.0 - fall);
        if f.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Battle results rise and fade.
pub fn animate_battle_text(
    time: Res<Time>,
    mut texts: Query<(Entity, &mut BattleText, &mut Transform, &mut TextColor)>,
    mut commands: Commands,
) {
    for (entity, mut text, mut transform, mut colour) in &mut texts {
        text.timer.tick(time.delta());
        transform.translation.y += 18.0 * time.delta_secs();
        let fade = ((text.timer.fraction() - 0.7) / 0.3).clamp(0.0, 1.0);
        colour.0.set_alpha(1.0 - fade);
        if text.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}
