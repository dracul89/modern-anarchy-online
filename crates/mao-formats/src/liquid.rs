//! The liquid surfaces as the original shapes and colours them (`VisualWater_t::Create`, DisplaySystem FUN_1003a4a4, ported
//! in `AnarchyReimagined`'s `dropin/displaysystem/src/liquid.rs`), from a playfield's or a room's water triangles.
//!
//! - **A still liquid** (the type's low bit clear) is cut into four by its edges' middles while twice its area is over
//!   10000 (FUN_1003a3b4), so the vertices the passes wave and light are never far apart.
//! - **A falling liquid** (the low bit set) under a tenth of a unit high is treated as still; otherwise its heights are
//!   made 0..1 and snapped to 1 or −0.01, and with two corners at the same height it is laid as ten strips from the top
//!   to the bottom, each height bent back over the fall as `1 − (1 − y)³` (FUN_1003a72c: the sheet of a waterfall curls
//!   over its lip). Three different heights fall back to the still cut.
//! - **Every triangle kept** has its middle on the playfield's 60 by 60 tiles of 100, three different corners and a face;
//!   its corners are ordered so the normal points up (FUN_10039f14). Types 0x10 and 0x11 are not drawn.
//! - **Its colour** is the type's (FUN_1003a0d6's table), the alpha being the surface's translucency, and **its layer**
//!   picks the passes that draw it: water (0), lava (1), toxic (2), the star water (3) and the wide water (4).
//!
//! Points are in the server's frame (x, height, z); [`surfaces`] turns the result into the viewer's Z-up world.

/// How many units across a playfield's tile is, and how many tiles a side (`triangle`'s check).
const TILE: f32 = 100.0;
const TILES: i32 = 60;

/// A liquid vertex colour (RGBA) and layer, by the type (`flags & 0x1f`).
pub fn colour(kind: u32) -> [u8; 4] {
    let argb: u32 = match kind {
        0 | 1 => 0xcc00_186a,
        2 | 3 => 0xffff_0000,
        4 | 5 => 0xf21f_3a21,
        6 | 7 => 0xf727_fc05,
        8 | 9 => 0xed66_3b1b,
        10 | 11 => 0x9933_997f,
        12 | 13 => 0xccc8_e6ff,
        14 | 15 => 0xffff_ffff,
        18 | 19 => 0x3334_9dfa,
        // (other types are coloured by their type word in the original, a dark near-black)
        _ => kind | 0xff00_0000,
    };
    [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8, (argb >> 24) as u8]
}

/// The passes that draw a type: 0 water, 1 lava, 2 toxic, 3 the star water, 4 the wide water; none for 0x10 and 0x11.
pub fn layer(kind: u32) -> Option<u8> {
    Some(match kind {
        2 | 3 => 1,
        6 | 7 => 2,
        0xe | 0xf => 3,
        0x10 | 0x11 => return None,
        0x12 | 0x13 => 4,
        _ => 0,
    })
}

type V3 = [f32; 3];

fn minus(a: &V3, b: &V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn plus(a: &V3, b: &V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn over(a: &V3, s: f32) -> V3 {
    [a[0] / s, a[1] / s, a[2] / s]
}

fn cross(a: &V3, b: &V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// The middle of an edge as the original takes it: `(p − q) / 2 + q`.
fn middle(p: &V3, q: &V3) -> V3 {
    plus(&over(&minus(p, q), 2.0), q)
}

/// The face's unit normal turned up (its second and third corners swapped when it pointed down), or none for no face.
pub fn face_up(a: V3, mut b: V3, mut c: V3) -> Option<([V3; 3], V3)> {
    let mut n = cross(&minus(&b, &a), &minus(&c, &a));
    if n[1] < 0.0 {
        std::mem::swap(&mut b, &mut c);
        n = cross(&minus(&b, &a), &minus(&c, &a));
    }
    let eps = 1e-5;
    if n.iter().all(|c| c.abs() < eps) {
        return None;
    }
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    Some(([a, b, c], over(&n, len)))
}

/// FUN_1003a0d6: one liquid triangle kept (its corners ordered upward), or dropped: when its middle is off the tiles (the
/// playfield's x and z both 0..6000: the row is counted down the negative of z, the tile `column − 60 × row`), two corners
/// are the same, or it has no face.
fn triangle(kind: u32, a: V3, b: V3, c: V3, out: &mut Vec<Surface>) {
    let mz = (b[2] + a[2] + c[2]) / 3.0;
    let row = (mz / -TILE) as i32;
    let mx = (b[0] + a[0] + c[0]) / 3.0;
    let column = (mx / TILE) as i32;
    if column.wrapping_sub(row.wrapping_mul(TILES)) as u32 > (TILES * TILES - 1) as u32 {
        return;
    }
    if a == b || b == c || c == a {
        return;
    }
    let Some((points, normal)) = face_up(a, b, c) else { return };
    let Some(layer) = layer(kind) else { return };
    out.push(Surface { points, normal, kind, layer, colour: colour(kind) });
}

/// FUN_1003a3b4: a still triangle cut into four by its edges' middles while twice its area is over 10000; then kept.
fn split(kind: u32, a: V3, b: V3, c: V3, out: &mut Vec<Surface>) {
    if kind & 1 == 0 {
        let n = cross(&minus(&b, &a), &minus(&c, &a));
        let size = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if size > 10000.0 {
            let (ab, bc, ca) = (middle(&b, &a), middle(&c, &b), middle(&a, &c));
            split(kind, a, ab, middle(&c, &a), out);
            split(kind, b, bc, middle(&a, &b), out);
            split(kind, c, ca, middle(&b, &c), out);
            split(kind, ab, bc, ca, out);
            return;
        }
    }
    triangle(kind, a, b, c, out);
}

/// FUN_1003a72c: a falling triangle (its heights 0..1, `a` and `b` at the same one) laid as ten strips from the odd corner
/// `c` down to the other two, or from them up to it when it is the low one, each height bent back over `base..base + h`.
fn strips(kind: u32, a: V3, b: V3, c: V3, base: f32, h: f32, out: &mut Vec<Surface>) {
    let bent = |p: &V3| -> V3 {
        let t = 1.0 - p[1];
        [p[0], (1.0 - t * t * t) * h + base, p[2]]
    };
    let high = 0.5 < c[1];
    let (da, db, mut p, mut q) = if high { (over(&minus(&a, &c), 10.0), over(&minus(&b, &c), 10.0), c, c) } else { (over(&minus(&c, &a), 10.0), over(&minus(&c, &b), 10.0), a, b) };
    let (mut last_p, mut last_q) = (p, q);
    for k in 0..10 {
        p = plus(&p, &da);
        q = plus(&q, &db);
        if high || k != 9 {
            split(kind, bent(&last_p), bent(&p), bent(&q), out);
        }
        if !high || k != 0 {
            split(kind, bent(&last_p), bent(&last_q), bent(&q), out);
        }
        last_p = p;
        last_q = q;
    }
}

/// One shaped liquid triangle: its corners (ordered so the normal points up), its normal, type, layer and colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    pub points: [V3; 3],
    pub normal: V3,
    pub kind: u32,
    pub layer: u8,
    pub colour: [u8; 4],
}

/// `VisualWater_t::Create`: a water triangle (the server's frame) as the triangles the original draws it as.
pub fn shape(kind: u32, a: V3, b: V3, c: V3, out: &mut Vec<Surface>) {
    if a == b || b == c || c == a {
        return;
    }
    if kind & 1 == 0 {
        split(kind, a, b, c, out);
        return;
    }
    let most = a[1].max(b[1]).max(c[1]);
    let least = a[1].min(b[1]).min(c[1]);
    let h = most - least;
    if h < 0.1 {
        split(kind, a, b, c, out);
        return;
    }
    let snap = |y: f32| if 0.5 < (y - least) / h { 1.0 } else { -0.01 };
    let (na, nb, nc) = ([a[0], snap(a[1]), a[2]], [b[0], snap(b[1]), b[2]], [c[0], snap(c[1]), c[2]]);
    // (a falling triangle with one high corner starts the fall's effect in the original; the effect is not here)
    if na[1] == nb[1] {
        strips(kind, na, nb, nc, least, h, out);
    } else if na[1] == nc[1] {
        strips(kind, na, nc, nb, least, h, out);
    } else if nb[1] == nc[1] {
        strips(kind, nb, nc, na, least, h, out);
    } else {
        split(kind, a, b, c, out);
    }
}

/// The surfaces of a set of water triangles (points in the server's frame, x, height, z), in the viewer's Z-up world
/// (x, the server's z, height) with their normals turned the same way.
pub fn surfaces<'a>(triangles: impl IntoIterator<Item = (u32, [V3; 3])>) -> Vec<Surface> {
    let mut out = Vec::new();
    for (kind, [a, b, c]) in triangles {
        shape(kind & 0x1f, a, b, c, &mut out);
    }
    for s in &mut out {
        for p in &mut s.points {
            *p = [p[0], p[2], p[1]];
        }
        s.normal = [s.normal[0], s.normal[2], s.normal[1]];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_and_layers_follow_the_type() {
        assert_eq!(colour(0), [0x00, 0x18, 0x6a, 0xcc]);
        assert_eq!(colour(7), [0x27, 0xfc, 0x05, 0xf7]);
        assert_eq!(colour(0x12), [0x34, 0x9d, 0xfa, 0x33]);
        assert_eq!([layer(0), layer(3), layer(6), layer(0xf), layer(0x10), layer(0x13)], [Some(0), Some(1), Some(2), Some(3), None, Some(4)]);
    }

    #[test]
    fn a_wide_still_sheet_is_cut_small_and_faces_up() {
        let mut out = Vec::new();
        // a 400 by 400 right triangle: twice its area 160000, cut until each piece is not over 10000 (two levels: 16 pieces)
        shape(0, [0.0, 10.0, 0.0], [0.0, 10.0, 400.0], [400.0, 10.0, 0.0], &mut out);
        assert_eq!(out.len(), 16);
        for s in &out {
            assert_eq!(s.normal, [0.0, 1.0, 0.0]);
            assert_eq!((s.layer, s.colour), (0, colour(0)));
        }
        // the corners were given clockwise seen from above (normal down): they come back turned
        let first = &out[0].points;
        let n = cross(&minus(&first[1], &first[0]), &minus(&first[2], &first[0]));
        assert!(n[1] > 0.0);
    }

    #[test]
    fn a_triangle_off_the_tiles_or_without_a_face_is_dropped() {
        let mut out = Vec::new();
        shape(0, [-500.0, 0.0, 0.0], [-500.0, 0.0, 10.0], [-490.0, 0.0, 0.0], &mut out);
        shape(0, [0.0, 0.0, -220.0], [0.0, 0.0, -210.0], [10.0, 0.0, -220.0], &mut out);
        shape(0, [0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0], &mut out);
        shape(0, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [20.0, 0.0, 5.0], &mut out);
        assert!(out.is_empty());
        shape(0x10, [0.0, 0.0, 0.0], [0.0, 0.0, 10.0], [10.0, 0.0, 0.0], &mut out);
        assert!(out.is_empty(), "types 0x10 and 0x11 are not drawn");
    }

    #[test]
    fn a_fall_is_laid_in_bent_strips() {
        let mut out = Vec::new();
        // a sheet 8 high with its top edge at the back: two high corners (a, b), one low (c)
        shape(1, [0.0, 8.0, 10.0], [10.0, 8.0, 10.0], [5.0, 0.0, 0.0], &mut out);
        // ten strips: the top one a single triangle, the others two (the odd corner is low: the first strip is one piece)
        assert_eq!(out.len(), 19);
        let heights: Vec<f32> = out.iter().flat_map(|s| s.points.iter().map(|p| p[1])).collect();
        let top = heights.iter().cloned().fold(f32::MIN, f32::max);
        let bottom = heights.iter().cloned().fold(f32::MAX, f32::min);
        assert!((top - 8.0).abs() < 1e-4 && (bottom - (8.0 * (1.0 - 1.01f32.powi(3)))).abs() < 1e-3, "{top} {bottom}");
        // the bend: halfway along the fall (the snapped height 0.495, so t = 0.505) the sheet is still at 87% of its height
        let mid = out.iter().flat_map(|s| s.points.iter()).find(|p| (p[2] - 5.0).abs() < 1e-3).expect("a point halfway");
        assert!((mid[1] - 8.0 * (1.0 - 0.505f32.powi(3))).abs() < 1e-3, "{mid:?}");
        for s in &out {
            assert!(s.normal[1] > 0.0);
        }
        // a low sheet is treated as still
        let mut flat = Vec::new();
        shape(1, [0.0, 0.05, 0.0], [0.0, 0.0, 10.0], [10.0, 0.0, 0.0], &mut flat);
        assert_eq!(flat.len(), 1);
    }

    #[test]
    fn surfaces_come_out_in_the_viewer_s_frame() {
        let out = surfaces([(0u32, [[0.0, 3.0, 0.0], [0.0, 3.0, 10.0], [10.0, 3.0, 0.0]])]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].normal, [0.0, 0.0, 1.0]);
        assert!(out[0].points.iter().all(|p| p[2] == 3.0));
    }
}
