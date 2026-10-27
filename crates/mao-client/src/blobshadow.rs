//! The characters' shadows as the original draws them: `GfxVisualSimpleShadow_c` (DisplaySystem, `dropin/displaysystem/src/
//! gfxshadow.rs`: FUN_1001f0e9 the constructor, FUN_1001e556 `Update`, FUN_1001e75d the vertex gather, FUN_1001e90e the draw)
//! driven by Gamecode's shadow control (FUN_100ed21e makes one per CAT mesh, FUN_100ed439 feeds it every frame, FUN_100ed0c0 lays
//! its points on the ground). The original has no other cast shadows: a character throws a black, translucent blob — a solid disc
//! with a fading rim — stretched from its mesh's extent along the light, laid on the terrain.
//!
//! The rule, in the original's order:
//! - The colour is black with alpha 0x5f outdoors, 0x2f in a dungeon (the constructor's argument).
//! - `Update` gets the light's direction (`VisualEnvFX_t` +0x7e8's +0x104: the ground light, or straight down when it has none;
//!   in a dungeon straight down), the camera's position, the ground point under the character and the ground's normal. It builds
//!   a basis `d`, `e` across the light (seeded with the up axis, or the x axis when the light is within 0.95 of vertical).
//! - The vertex callback takes every fourth vertex of the mesh above the ground point's height and keeps the extent of their
//!   offsets from the mesh's position in that basis.
//! - The draw: half extents `hx`, `hy`; the segment count from the extent's size and the distance to the camera
//!   (`v = (√(√(hx² + hy²) × π) + 0.2) × (1 − dist / 50)`, `2^round(log2(20v + 1) − 0.49999)`, at most 300; fewer than four draws
//!   nothing — so the blob is gone some 50 units out). Nothing unless `light · normal < −0.1`; the alpha is scaled by
//!   `(−dot − 0.1) / 0.3` (at most 1). Points are projected along the light onto the ground plane. The solid rim sits at
//!   `0.7 + 0.2 × h × dot` of the extent (at least 0.1; `h` the projection distance), the transparent rim at `1 − 0.4 × dot`.
//!   The whole draw is faded to 0.9.
//! - The ground callback sets each point's height to the tile-map's at it plus 0.02, outdoors, unless the character stands on
//!   something other than the ground (its ground point differs from the heightmap by more than 0.2).
//!
//! The viewer works in its Z-up frame; the maths is the same with the up axis swapped.

use mao_formats::model::{multiply, Matrix, Part};

/// A character's mesh, as the shadow sees it: where it stands (the mesh frame's translation) and every fourth vertex, in the
/// viewer's world (Z up).
#[derive(Clone, Debug, Default)]
pub struct Caster {
    pub translation: [f32; 3],
    pub points: Vec<[f32; 3]>,
    /// The player's own character (moved on between updates like its parts).
    pub own: bool,
}

fn transform(m: &Matrix, p: [f32; 3]) -> [f32; 3] {
    [
        p[0] * m[0] + p[1] * m[4] + p[2] * m[8] + m[12],
        p[0] * m[1] + p[1] * m[5] + p[2] * m[9] + m[13],
        p[0] * m[2] + p[1] * m[6] + p[2] * m[10] + m[14],
    ]
}

impl Caster {
    /// A character drawn as `parts` placed by `place` (each part's own matrix first).
    pub fn new(parts: &[Part], place: &Matrix, own: bool) -> Caster {
        let mut points = Vec::new();
        for part in parts {
            let m = multiply(&part.world, place);
            points.extend(part.vertices.iter().step_by(4).map(|v| transform(&m, v.position)));
        }
        Caster { translation: transform(place, [0.0; 3]), points, own }
    }

    pub fn shifted(&self, by: [f32; 3]) -> Caster {
        let add = |p: &[f32; 3]| [p[0] + by[0], p[1] + by[1], p[2] + by[2]];
        Caster { translation: add(&self.translation), points: self.points.iter().map(add).collect(), own: self.own }
    }
}

/// The alpha of the blob's colour outdoors and in a dungeon (FUN_100ed21e: `0x5f000000`, `0x2f000000`).
pub const ALPHA_OUTDOORS: u8 = 0x5f;
pub const ALPHA_DUNGEON: u8 = 0x2f;

/// Where a blob is laid: the ground's point under the character and its normal, and how to find the ground's height at a point
/// (none when the character does not stand on the heightmap, or indoors: the points keep the plane's height).
pub struct Ground<'a> {
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub height: Option<&'a dyn Fn(f32, f32) -> f32>,
}

/// A vertex of the blob: position (Z up) and colour with alpha.
pub type BlobVertex = ([f32; 3], [f32; 4]);

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// The blob of one character as a triangle list (empty when the original would draw none). `light` is the direction the light
/// travels (unit, Z up), `eye` the camera's position, `alpha` the colour's alpha byte.
pub fn blob(caster: &Caster, light: [f32; 3], eye: [f32; 3], ground: &Ground, alpha: u8) -> Vec<BlobVertex> {
    let t = caster.translation;
    let (c2, c3) = (ground.point, ground.normal);
    // The basis across the light (`Update`): the pole case within 0.95 of vertical.
    let (d, e) = if light[2].abs() >= 0.95 {
        let e = cross(light, [1.0, 0.0, 0.0]);
        (cross(e, light), e)
    } else {
        let d = cross([0.0, 0.0, 1.0], light);
        (d, cross(light, d))
    };
    // The extent of the mesh above the ground point, in that basis (the vertex callback).
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (1e30f32, 1e30f32, -1e30f32, -1e30f32);
    for &p in &caster.points {
        if p[2] < c2[2] {
            continue;
        }
        let r = sub(p, t);
        let (cx, cy) = (dot(d, r), dot(e, r));
        min_x = min_x.min(cx);
        min_y = min_y.min(cy);
        max_x = max_x.max(cx);
        max_y = max_y.max(cy);
    }
    let (cx, cy) = ((max_x + min_x) * 0.5, (max_y + min_y) * 0.5);
    let (hx, hy) = ((max_x - min_x) * 0.5, (max_y - min_y) * 0.5);
    if !(hx > 0.0) || !(hy > 0.0) {
        return Vec::new();
    }
    // The segment count from the size and the distance.
    let dist = sub(t, eye);
    let dist = dot(dist, dist).sqrt();
    let fade = 1.0 - dist / 50.0;
    let v = (((hx * hx + hy * hy).sqrt() * std::f32::consts::PI).sqrt() + 0.2) * fade;
    let v = if v > 0.0 { v } else { 0.0 };
    let n = ((v * 20.0 + 1.0).ln() / 2f32.ln() - 0.49999).round().max(0.0) as u32;
    let segments = (1usize << n.min(31)).min(300);
    if segments < 4 {
        return Vec::new();
    }
    let dot_ln = dot(light, c3);
    if !(dot_ln < -0.1) {
        return Vec::new();
    }
    let (plane_add, plane_mul) = (-dot(c2, c3), -1.0 / dot_ln);
    let alpha_scale = ((-dot_ln - 0.1) / 0.3).min(1.0);
    let full = (alpha as f32 * alpha_scale).round().clamp(0.0, 255.0) / 255.0 * 0.9;
    let centre = add(add(t, scale(d, cx)), scale(e, cy));
    let height_of = |p: [f32; 3]| (dot(p, c3) + plane_add) * plane_mul;
    let project = |p: [f32; 3]| add(p, scale(light, height_of(p)));
    let inner_scale = dot_ln * -0.4 + 1.0;
    let lay = |mut p: [f32; 3]| {
        if let Some(height) = ground.height {
            p[2] = height(p[0], p[1]) + 0.02;
        }
        p
    };
    let centre_point = lay(project(centre));
    let step = 2.0 * std::f32::consts::PI / segments as f32;
    let mut solid = Vec::with_capacity(segments);
    let mut clear = Vec::with_capacity(segments);
    let mut angle = 0.0f32;
    for _ in 0..segments {
        let (sa, ca) = angle.sin_cos();
        let ring = add(scale(d, ca * hx), scale(e, sa * hy));
        let p_in = add(centre, scale(ring, inner_scale));
        let h_in = height_of(p_in);
        clear.push(lay(add(p_in, scale(light, h_in))));
        let oc = (0.7 + 0.2 * h_in * dot_ln).max(0.1);
        solid.push(lay(project(add(centre, scale(ring, oc)))));
        angle += step;
    }
    let dark = [0.0, 0.0, 0.0, full];
    let none = [0.0, 0.0, 0.0, 0.0];
    let mut out = Vec::with_capacity(segments * 9);
    for k in 0..segments {
        let next = (k + 1) % segments;
        out.extend_from_slice(&[(centre_point, dark), (solid[k], dark), (solid[next], dark)]);
        out.extend_from_slice(&[(solid[k], dark), (clear[k], none), (solid[next], dark)]);
        out.extend_from_slice(&[(solid[next], dark), (clear[k], none), (clear[next], none)]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standing() -> Caster {
        // a box 0.6 wide, 1.8 tall, standing at (10, 20, 5)
        let mut points = Vec::new();
        for z in [0.0, 0.9, 1.8] {
            for (x, y) in [(-0.3, -0.3), (0.3, -0.3), (0.3, 0.3), (-0.3, 0.3)] {
                points.push([10.0 + x, 20.0 + y, 5.0 + z]);
            }
        }
        Caster { translation: [10.0, 20.0, 5.0], points, own: false }
    }

    #[test]
    fn a_character_in_the_sun_throws_a_blob_on_the_ground() {
        let light = [-0.4f32, 0.5, -0.8];
        let len = (light[0] * light[0] + light[1] * light[1] + light[2] * light[2]).sqrt();
        let light = light.map(|c| c / len);
        let flat = |_: f32, _: f32| 5.0f32;
        let ground = Ground { point: [10.0, 20.0, 5.0], normal: [0.0, 0.0, 1.0], height: Some(&flat) };
        let blob = blob(&standing(), light, [10.0, 10.0, 8.0], &ground, ALPHA_OUTDOORS);
        assert!(!blob.is_empty());
        // on the ground, a little above it; the solid part dark, the rim clear
        assert!(blob.iter().all(|(p, _)| (p[2] - 5.02).abs() < 1e-4));
        let alphas: std::collections::BTreeSet<u32> = blob.iter().map(|(_, c)| (c[3] * 1000.0) as u32).collect();
        assert_eq!(alphas.len(), 2);
        assert!(alphas.contains(&0));
        // the full alpha: 0x5f scaled by the sun's height, faded to 0.9
        let full = *alphas.iter().max().unwrap() as f32 / 1000.0;
        assert!((full - 95.0 / 255.0 * 0.9).abs() < 0.01, "{full}");
        // the blob lies down-light of the feet
        let mean_x = blob.iter().map(|(p, _)| p[0]).sum::<f32>() / blob.len() as f32;
        let mean_y = blob.iter().map(|(p, _)| p[1]).sum::<f32>() / blob.len() as f32;
        assert!(mean_x < 10.0 && mean_y > 20.0, "{mean_x} {mean_y}");
    }

    #[test]
    fn far_away_or_under_a_low_sun_there_is_none() {
        let flat = |_: f32, _: f32| 5.0f32;
        let ground = Ground { point: [10.0, 20.0, 5.0], normal: [0.0, 0.0, 1.0], height: Some(&flat) };
        assert!(blob(&standing(), [0.0, 0.0, -1.0], [10.0, 90.0, 8.0], &ground, ALPHA_OUTDOORS).is_empty());
        // a grazing light: the dot with the normal is above -0.1
        assert!(blob(&standing(), [0.995, 0.0, -0.05], [10.0, 22.0, 8.0], &ground, ALPHA_OUTDOORS).is_empty());
        // straight down works too (the pole basis)
        assert!(!blob(&standing(), [0.0, 0.0, -1.0], [10.0, 22.0, 8.0], &ground, ALPHA_DUNGEON).is_empty());
    }

    #[test]
    fn the_segment_count_grows_with_the_size_and_shrinks_with_distance() {
        let flat = |_: f32, _: f32| 5.0f32;
        let ground = Ground { point: [10.0, 20.0, 5.0], normal: [0.0, 0.0, 1.0], height: Some(&flat) };
        let near = blob(&standing(), [0.0, 0.0, -1.0], [10.0, 22.0, 8.0], &ground, ALPHA_OUTDOORS).len();
        let far = blob(&standing(), [0.0, 0.0, -1.0], [10.0, 55.0, 8.0], &ground, ALPHA_OUTDOORS).len();
        assert!(near > far, "{near} {far}");
    }
}
