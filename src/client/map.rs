//! Terrain tiles, ground decorations (shores, roads, farms, borders), fog of war and map markers.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use hexx::Hex;
use std::collections::{HashMap, HashSet};

use bevy_civ::city::City;
use bevy_civ::fog::FogOfWar;
use bevy_civ::tech::TechState;
use bevy_civ::terrain::{HexTile, Improvement, Terrain};
use bevy_civ::unit::Unit;

use super::{overlay, owner_color, Art};
use crate::{Dirty, MapData, PathPreview, Selection, PLAYER};

pub const HEX_SIZE: f32 = 48.0;
/// On-screen size of one overlay frame (a full hexagon).
pub const HEX_SPRITE: Vec2 = Vec2::new(96.0, 84.0);
const TILE_SPRITE: Vec2 = Vec2::new(96.0, 112.0);
/// The hexagon's centre sits below the middle of the taller terrain frame.
const TILE_ANCHOR: Anchor = Anchor::Custom(Vec2::new(0.0, -0.125));
const DIM: Color = Color::srgb(0.55, 0.57, 0.62);

/// The tile under the mouse cursor.
#[derive(Resource, Default)]
pub struct Hovered(pub Option<Hex>);

#[derive(Component)]
pub struct Decor;
#[derive(Component)]
pub struct HoverMarker;
#[derive(Component)]
pub struct SelectMarker;
#[derive(Component)]
pub struct PathDot;
/// The bouncing arrow over the selected unit or city.
#[derive(Component)]
pub struct SelectArrow;
#[derive(Resource)]
pub struct WaterClock(Timer, usize);

/// Tiles further south draw over tiles further north, so peaks and trees overlap correctly.
pub fn tile_z(y: f32) -> f32 {
    1.0 - y * 0.0005
}

fn variant(hex: Hex) -> usize {
    (hex.x.wrapping_mul(73_856_093) ^ hex.y.wrapping_mul(19_349_663)).unsigned_abs() as usize
}

fn terrain_row(terrain: Terrain) -> usize {
    match terrain {
        Terrain::Grassland => 0,
        Terrain::Plains => 1,
        Terrain::Desert => 2,
        Terrain::Forest => 3,
        Terrain::Hills => 4,
        Terrain::Mountain => 5,
        Terrain::Water => 6,
        Terrain::Tundra => 7,
        Terrain::Jungle => 8,
        Terrain::Marsh => 9,
    }
}

pub fn setup_map_view(mut commands: Commands, art: Res<Art>) {
    commands.spawn(Camera2d);
    commands.insert_resource(WaterClock(Timer::from_seconds(0.16, TimerMode::Repeating), 0));
    let overlay_sprite = |index: usize| Sprite {
        image: art.overlays.clone(),
        texture_atlas: Some(art.atlas(&art.overlays_layout, index)),
        custom_size: Some(HEX_SPRITE),
        ..default()
    };
    commands.spawn((HoverMarker, overlay_sprite(overlay::HOVER), Transform::from_xyz(0.0, 0.0, 4.0), Visibility::Hidden));
    commands.spawn((SelectMarker, overlay_sprite(overlay::SELECT), Transform::from_xyz(0.0, 0.0, 5.5), Visibility::Hidden));
    let arrow = |color: Color| (Text2d::new("▼"), TextFont { font: art.font_bold.clone(), font_size: 80.0, ..default() }, TextColor(color));
    commands
        .spawn((SelectArrow, arrow(Color::srgb(1.0, 0.86, 0.3)), Transform::from_xyz(0.0, 0.0, 9.5), Visibility::Hidden))
        .with_children(|parent| {
            // A dark copy just behind it keeps the arrow readable on any terrain.
            parent.spawn((arrow(Color::srgba(0.1, 0.06, 0.02, 0.85)), Transform::from_xyz(2.0, -3.0, -0.05).with_scale(Vec3::splat(1.15))));
        });
}

pub fn attach_tile_views(mut commands: Commands, map: Res<MapData>, tiles: Query<(Entity, &HexTile), Added<HexTile>>) {
    for (entity, tile) in &tiles {
        let pos = map.layout.hex_to_world_pos(tile.coord);
        commands.entity(entity).insert((Sprite::default(), Transform::from_xyz(pos.x, pos.y, tile_z(pos.y))));
    }
}

/// Rebuilds tile sprites and ground decorations whenever the map changed.
pub fn refresh_map(
    mut dirty: ResMut<Dirty>,
    art: Res<Art>,
    map: Res<MapData>,
    fog: Res<FogOfWar>,
    clock: Res<WaterClock>,
    (tech, supplies): (Res<TechState>, Res<crate::supply::Supplies>),
    rivers: Res<crate::Rivers>,
    mut tiles: Query<(&HexTile, &mut Sprite)>,
    cities: Query<&City>,
    decor: Query<Entity, With<Decor>>,
    mut commands: Commands,
) {
    if !dirty.map {
        return;
    }
    dirty.map = false;
    for entity in &decor {
        commands.entity(entity).despawn();
    }

    let info: HashMap<Hex, (Terrain, bool)> = tiles.iter().map(|(t, _)| (t.coord, (t.terrain, t.has_road()))).collect();
    let rails: HashSet<Hex> = tiles.iter().filter(|(t, _)| t.has_railroad()).map(|(t, _)| t.coord).collect();
    let city_at: HashMap<Hex, u32> = cities.iter().map(|c| (c.position, c.owner)).collect();
    // Borders as the supply network worked them out (each city's reach grows with its culture).
    let territory: HashMap<Hex, (u32, Hex)> = supplies.territory.clone();

    for (tile, mut sprite) in &mut tiles {
        let hex = tile.coord;
        let pos = map.layout.hex_to_world_pos(hex);
        if !fog.explored.contains(&hex) {
            *sprite = Sprite {
                image: art.overlays.clone(),
                texture_atlas: Some(art.atlas(&art.overlays_layout, overlay::UNKNOWN)),
                custom_size: Some(HEX_SPRITE),
                ..default()
            };
            continue;
        }
        let tint = if fog.visible.contains(&hex) { Color::WHITE } else { DIM };
        let column = if tile.terrain == Terrain::Water { (clock.1 + variant(hex)) % 8 } else { variant(hex) % 4 };
        *sprite = Sprite {
            image: art.terrain.clone(),
            texture_atlas: Some(art.atlas(&art.terrain_layout, terrain_row(tile.terrain) * 8 + column)),
            custom_size: Some(TILE_SPRITE),
            anchor: TILE_ANCHOR,
            color: tint,
            ..default()
        };

        let z = tile_z(pos.y);
        let mut overlay_at = |index: usize, toward: Option<Hex>, color: Color, dz: f32| {
            let rotation = toward.map_or(Quat::IDENTITY, |n| {
                let dir = map.layout.hex_to_world_pos(n) - pos;
                Quat::from_rotation_z(dir.y.atan2(dir.x) - std::f32::consts::FRAC_PI_2)
            });
            commands.spawn((
                Decor,
                Sprite {
                    image: art.overlays.clone(),
                    texture_atlas: Some(art.atlas(&art.overlays_layout, index)),
                    custom_size: Some(HEX_SPRITE),
                    color,
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, z + dz).with_rotation(rotation),
            ));
        };

        let neighbors = hex.all_neighbors();
        if tile.terrain == Terrain::Water {
            for n in neighbors {
                if info.get(&n).is_some_and(|(t, _)| *t != Terrain::Water) {
                    overlay_at(overlay::SHORE, Some(n), tint, 0.004);
                }
            }
        }
        if tile.improvements.contains(&Improvement::Farm) {
            overlay_at(overlay::FARM, None, tint, 0.006);
        }
        if tile.has_road() {
            // Railroads are drawn as dark iron lines wherever they link up with another line or a city.
            let rail = tile.has_railroad();
            let shade = tint.to_srgba();
            let iron = Color::srgb(shade.red * 0.42, shade.green * 0.36, shade.blue * 0.34);
            for n in neighbors {
                if info.get(&n).is_some_and(|(_, road)| *road) || city_at.contains_key(&n) {
                    let linked = rail && (rails.contains(&n) || city_at.contains_key(&n));
                    overlay_at(overlay::ROAD, Some(n), if linked { iron } else { tint }, 0.008);
                }
            }
            overlay_at(overlay::ROAD_HUB, None, if rail { iron } else { tint }, 0.009);
        }
        // Rivers: a blue channel towards the next and previous tiles of each river.
        if rivers.tiles.contains(&hex) {
            for path in rivers.paths.iter().filter(|p| p.contains(&hex)) {
                let i = path.iter().position(|h| *h == hex).unwrap_or(0);
                for n in [i.checked_sub(1).and_then(|j| path.get(j)), path.get(i + 1)].into_iter().flatten() {
                    overlay_at(overlay::ROAD, Some(*n), Color::srgb(0.32, 0.6, 0.95).mix(&tint, 0.15), 0.007);
                }
            }
        }
        if tile.is_polluted() {
            overlay_at(overlay::BASE, None, Color::srgba(0.25, 0.2, 0.12, 0.7), 0.012);
        }
        if tile.improvements.contains(&Improvement::Mine) {
            overlay_at(overlay::MINE, None, tint, 0.01);
        }
        if tile.improvements.contains(&Improvement::Fortress) {
            overlay_at(overlay::FORT, None, tint, 0.011);
        }
        if let Some((owner, _)) = territory.get(&hex) {
            for n in neighbors {
                if territory.get(&n).map(|(o, _)| o) != Some(owner) {
                    overlay_at(overlay::BORDER, Some(n), owner_color(*owner).with_alpha(0.9), 0.012);
                }
            }
        }
        // Strategic resources stay hidden until the technology to use them is known.
        let revealed = tile.resource.filter(|r| r.reveal_tech().is_none_or(|t| tech.researched.contains(&t)));
        if let Some(resource) = revealed {
            // A source inside our borders that no road reaches yet is shown washed out.
            let tint = if supplies.unconnected.get(&hex) == Some(&PLAYER) { tint.with_alpha(0.55) } else { tint };
            commands.spawn((
                Decor,
                Sprite {
                    image: art.resources.clone(),
                    texture_atlas: Some(art.atlas(&art.resources_layout, resource.index())),
                    custom_size: Some(Vec2::splat(34.0)),
                    color: tint,
                    ..default()
                },
                Transform::from_xyz(pos.x + 24.0, pos.y - 20.0, z + 0.014),
            ));
        }
    }
}

pub fn animate_water(
    time: Res<Time>,
    mut clock: ResMut<WaterClock>,
    fog: Res<FogOfWar>,
    mut tiles: Query<(&HexTile, &mut Sprite)>,
) {
    if !clock.0.tick(time.delta()).just_finished() {
        return;
    }
    clock.1 = (clock.1 + 1) % 8;
    for (tile, mut sprite) in &mut tiles {
        if tile.terrain != Terrain::Water || !fog.explored.contains(&tile.coord) {
            continue;
        }
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = terrain_row(Terrain::Water) * 8 + (clock.1 + variant(tile.coord)) % 8;
        }
    }
}

/// Positions the hover outline and the pulsing ring under the selection.
/// Marks drawn over the area of the selected city: each tile's yield, and a ring on worked tiles.
#[derive(Component)]
pub struct AreaMark;

/// While one of the player's cities is selected, shows what every tile of its area yields (food,
/// shield and gold icons) and which tiles its citizens work; clicking a tile moves a citizen.
#[allow(clippy::too_many_arguments)]
pub fn show_city_area(
    selection: Res<Selection>,
    cities: Query<(&City, &bevy_civ::building::CityBuildings, Option<&bevy_civ::diplomacy::Happiness>)>,
    changed: Query<(), Changed<City>>,
    supplies: Res<crate::supply::Supplies>,
    tiles: Query<&HexTile>,
    rivers: Res<crate::Rivers>,
    tech: Res<TechState>,
    art: Res<Art>,
    map: Res<MapData>,
    marks: Query<Entity, With<AreaMark>>,
    (state, golden): (Res<crate::GameState>, Res<crate::GoldenAges>),
    mut commands: Commands,
) {
    if !(selection.is_changed() || supplies.is_changed() || !changed.is_empty()) {
        return;
    }
    for entity in &marks {
        commands.entity(entity).despawn_recursive();
    }
    let Some((city, buildings, happiness)) = selection.city.and_then(|e| cities.get(e).ok()).filter(|(c, _, _)| c.owner == PLAYER) else { return };
    let festive = happiness.is_some_and(|h| bevy_civ::economy::celebrating(city.population, h.happy, h.unhappy));
    let harbor = buildings.has(bevy_civ::building::Building::Harbor);
    for (hex, _) in supplies.territory.iter().filter(|(_, (o, home))| *o == PLAYER && *home == city.position) {
        let Some(tile) = tiles.iter().find(|t| t.coord == *hex) else { continue };
        let resource = tile.resource.filter(|r| r.reveal_tech().is_none_or(|t| tech.researched.contains(&t)));
        let mut y = bevy_civ::tiles::tile_yield(tile.terrain, &tile.improvements, resource, rivers.tiles.contains(hex));
        if harbor && tile.terrain == Terrain::Water {
            y.0 += 1;
        }
        let centre = *hex == city.position;
        if centre {
            y = bevy_civ::tiles::centre_yield(y);
        }
        // What the government (and a golden age) makes of it.
        y = bevy_civ::economy::tile_rule(y, state.government, golden.active(crate::PLAYER), festive);
        let worked = centre || city.worked.contains(hex);
        let pos = map.layout.hex_to_world_pos(*hex);
        let z = 6.5;
        // A ring: bright on worked tiles, faint on the others.
        commands.spawn((
            AreaMark,
            Sprite {
                image: art.overlays.clone(),
                texture_atlas: Some(art.atlas(&art.overlays_layout, overlay::SELECT)),
                custom_size: Some(HEX_SPRITE),
                color: if worked { Color::srgba(0.55, 1.0, 0.5, 0.95) } else { Color::srgba(1.0, 1.0, 1.0, 0.25) },
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, z),
        ));
        // The yield: a row of small icons (food, then shields, then gold).
        let icons: Vec<usize> = std::iter::repeat_n(super::icon::FOOD, y.0 as usize)
            .chain(std::iter::repeat_n(super::icon::PRODUCTION, y.1 as usize))
            .chain(std::iter::repeat_n(super::icon::GOLD, y.2 as usize))
            .collect();
        let size = if icons.len() > 5 { 13.0 } else { 17.0 };
        let width = size * icons.len() as f32;
        if !icons.is_empty() {
            commands.spawn((
                AreaMark,
                Sprite { color: Color::srgba(0.08, 0.06, 0.04, if worked { 0.75 } else { 0.5 }), custom_size: Some(Vec2::new(width + 6.0, size + 4.0)), ..default() },
                Transform::from_xyz(pos.x, pos.y - 22.0, z + 0.05),
            ));
        }
        for (i, icon) in icons.iter().enumerate() {
            commands.spawn((
                AreaMark,
                Sprite {
                    image: art.icons.clone(),
                    texture_atlas: Some(art.atlas(&art.icons_layout, *icon)),
                    custom_size: Some(Vec2::splat(size)),
                    color: if worked { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.6) },
                    ..default()
                },
                Transform::from_xyz(pos.x - width / 2.0 + size * (i as f32 + 0.5), pos.y - 22.0, z + 0.1),
            ));
        }
    }
}

/// A tribal village on the map (shown once the land around it has been explored).
#[derive(Component)]
pub struct VillageMark(Hex);

pub fn update_villages(
    villages: Res<bevy_civ::villages::Villages>,
    fog: Res<FogOfWar>,
    art: Res<Art>,
    map: Res<MapData>,
    marks: Query<(Entity, &VillageMark)>,
    mut commands: Commands,
) {
    if !villages.is_changed() && !fog.is_changed() {
        return;
    }
    let shown: HashSet<Hex> = marks.iter().map(|(_, m)| m.0).collect();
    for (entity, mark) in &marks {
        if !villages.sites.contains(&mark.0) || !fog.explored.contains(&mark.0) {
            commands.entity(entity).despawn_recursive();
        }
    }
    for at in villages.sites.iter().filter(|h| fog.explored.contains(*h) && !shown.contains(*h)) {
        let pos = map.layout.hex_to_world_pos(*at);
        commands.spawn((
            VillageMark(*at),
            Decor,
            Sprite { image: art.village.clone(), custom_size: Some(Vec2::splat(84.0)), ..default() },
            Transform::from_xyz(pos.x, pos.y + 6.0, 4.0),
        ));
    }
}

pub fn update_markers(
    time: Res<Time>,
    map: Res<MapData>,
    hovered: Res<Hovered>,
    selection: Res<Selection>,
    units: Query<(&Unit, &Transform), (Without<HoverMarker>, Without<SelectMarker>)>,
    cities: Query<&City>,
    mut hover: Query<(&mut Transform, &mut Visibility), (With<HoverMarker>, Without<SelectMarker>)>,
    mut select: Query<(&mut Transform, &mut Visibility, &mut Sprite), (With<SelectMarker>, Without<HoverMarker>)>,
    mut arrow: Query<(&mut Transform, &mut Visibility, &mut TextColor), (With<SelectArrow>, Without<SelectMarker>, Without<HoverMarker>, Without<Unit>)>,
) {
    // A bouncing arrow over the selected unit (or city), on top of everything, so it is always found.
    if let Ok((mut transform, mut visibility, mut color)) = arrow.get_single_mut() {
        let bounce = 7.0 * (time.elapsed_secs() * 4.5).sin().abs();
        let over = if let Some((unit, unit_transform)) = selection.unit.and_then(|e| units.get(e).ok()) {
            color.0 = if unit.owner == PLAYER { Color::srgb(1.0, 0.86, 0.3) } else { Color::srgb(1.0, 0.45, 0.35) };
            Some(unit_transform.translation.truncate() + Vec2::new(0.0, 74.0))
        } else {
            color.0 = Color::srgb(1.0, 0.86, 0.3);
            selection.city.and_then(|e| cities.get(e).ok()).map(|c| map.layout.hex_to_world_pos(c.position) + Vec2::new(0.0, 88.0))
        };
        match over {
            Some(pos) => {
                transform.translation = (pos + Vec2::new(0.0, bounce)).extend(9.5);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    if let Ok((mut transform, mut visibility)) = hover.get_single_mut() {
        match hovered.0 {
            Some(hex) => {
                let pos = map.layout.hex_to_world_pos(hex);
                transform.translation = pos.extend(4.0);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    if let Ok((mut transform, mut visibility, mut sprite)) = select.get_single_mut() {
        let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 5.0).sin();
        let target = if let Some((unit, unit_transform)) = selection.unit.and_then(|e| units.get(e).ok()) {
            let color = if unit.owner == PLAYER { Color::srgb(1.0, 0.9, 0.35) } else { Color::srgb(1.0, 0.4, 0.3) };
            Some((unit_transform.translation.truncate() + Vec2::new(0.0, -2.0), 0.8, color))
        } else if let Some(city) = selection.city.and_then(|e| cities.get(e).ok()) {
            Some((map.layout.hex_to_world_pos(city.position) + Vec2::new(0.0, -6.0), 1.25, Color::srgb(1.0, 0.9, 0.35)))
        } else {
            None
        };
        match target {
            Some((pos, scale, color)) => {
                transform.translation = pos.extend(5.5);
                transform.scale = Vec3::splat(scale * (1.0 + 0.05 * pulse));
                sprite.color = color.with_alpha(0.65 + 0.35 * pulse);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

pub fn update_path_dots(
    path: Res<PathPreview>,
    art: Res<Art>,
    map: Res<MapData>,
    dots: Query<Entity, With<PathDot>>,
    mut commands: Commands,
) {
    if !path.is_changed() {
        return;
    }
    for entity in &dots {
        commands.entity(entity).despawn_recursive();
    }
    let Some(start) = path.start else { return };
    let dot = |color: Color, scale: f32| Sprite {
        image: art.overlays.clone(),
        texture_atlas: Some(art.atlas(&art.overlays_layout, overlay::DOT)),
        custom_size: Some(HEX_SPRITE * scale),
        color,
        ..default()
    };
    let multi_turn = path.steps.last().is_some_and(|(_, turn)| *turn > 0);
    let mut from = map.layout.hex_to_world_pos(start);
    for (i, (hex, turn)) in path.steps.iter().enumerate() {
        let pos = map.layout.hex_to_world_pos(*hex);
        let last = i + 1 == path.steps.len();
        // This turn's steps are bright; later turns fade so the reach of each turn reads at a glance.
        let color = if last {
            Color::srgb(1.0, 0.35, 0.3)
        } else if *turn == 0 {
            Color::srgb(1.0, 0.92, 0.4)
        } else {
            Color::srgba(0.95, 0.88, 0.62, 0.75)
        };
        // Small dots in between tiles trace the road itself.
        for t in [0.33, 0.66] {
            let between = from.lerp(pos, t);
            commands.spawn((PathDot, dot(color.with_alpha(color.alpha() * 0.7), 0.45), Transform::from_xyz(between.x, between.y, 4.5)));
        }
        from = pos;
        let turn_ends = path.steps.get(i + 1).is_none_or(|(_, next)| next != turn);
        let marker = multi_turn && turn_ends;
        commands.spawn((PathDot, dot(color, if last { 1.3 } else { 0.9 }), Transform::from_xyz(pos.x, pos.y, 4.5)));
        // Where each turn's march ends, a badge with the number of turns it takes to get there, drawn
        // above units and cities (and beside them) so it is never hidden.
        if marker {
            commands
                .spawn((PathDot, dot(color.with_alpha(1.0), 1.7), Transform::from_xyz(pos.x + 24.0, pos.y - 20.0, 8.6)))
                .with_children(|parent| {
                    parent.spawn((
                        Text2d::new((turn + 1).to_string()),
                        TextFont { font: art.font_bold.clone(), font_size: 19.0, ..default() },
                        TextColor(Color::srgb(0.12, 0.08, 0.05)),
                        Transform::from_xyz(0.0, 1.0, 0.1),
                    ));
                });
        }
    }
}
