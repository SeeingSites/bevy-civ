//! Painting into an RGBA image: lines for diagrams (the relations web, the tree of knowledge).
//! Game-agnostic: it knows only pixels.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// A transparent image to paint into, `width` by `height` pixels.
pub fn blank(width: u32, height: u32) -> Image {
    Image::new_fill(
        Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// A thick line from `from` to `to` (pixels), stamped as discs along its length, into the RGBA
/// pixels of an image `width` pixels wide.
pub fn line(data: &mut [u8], width: u32, from: Vec2, to: Vec2, thickness: f32, colour: [u8; 4]) {
    let height = (data.len() / 4) as u32 / width.max(1);
    let length = from.distance(to).max(1.0);
    let steps = (length * 2.0) as usize;
    let r = thickness / 2.0;
    for s in 0..=steps {
        let p = from.lerp(to, s as f32 / steps as f32);
        let (x0, x1) = ((p.x - r).floor() as i32, (p.x + r).ceil() as i32);
        let (y0, y1) = ((p.y - r).floor() as i32, (p.y + r).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                    continue;
                }
                if Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(p) > r + 0.5 {
                    continue;
                }
                let i = ((y as u32 * width + x as u32) * 4) as usize;
                data[i..i + 4].copy_from_slice(&colour);
            }
        }
    }
}

/// A connector from the right side of one box to the left side of another: across, down (or up)
/// and across again, so lines between columns stay readable.
pub fn elbow(data: &mut [u8], width: u32, from: Vec2, to: Vec2, thickness: f32, colour: [u8; 4]) {
    let mid = (from.x + to.x) / 2.0;
    line(data, width, from, Vec2::new(mid, from.y), thickness, colour);
    line(
        data,
        width,
        Vec2::new(mid, from.y),
        Vec2::new(mid, to.y),
        thickness,
        colour,
    );
    line(data, width, Vec2::new(mid, to.y), to, thickness, colour);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_elbows_are_painted_where_they_should_be() {
        let (w, h) = (100u32, 50u32);
        let mut data = vec![0u8; (w * h * 4) as usize];
        line(
            &mut data,
            w,
            Vec2::new(5.0, 5.0),
            Vec2::new(90.0, 5.0),
            3.0,
            [255, 0, 0, 255],
        );
        let red = |data: &[u8], x: u32, y: u32| data[((y * w + x) * 4) as usize];
        assert_eq!(red(&data, 50, 5), 255);
        assert_eq!(red(&data, 50, 30), 0);
        let mut data = vec![0u8; (w * h * 4) as usize];
        elbow(
            &mut data,
            w,
            Vec2::new(10.0, 10.0),
            Vec2::new(90.0, 40.0),
            2.0,
            [255, 0, 0, 255],
        );
        assert_eq!(red(&data, 50, 25), 255, "the vertical leg at the midpoint");
        assert_eq!(red(&data, 80, 40), 255);
        assert_eq!(red(&data, 80, 10), 0);
    }
}
