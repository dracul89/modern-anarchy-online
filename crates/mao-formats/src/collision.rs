//! Collision from triangles: what blocks a walking character and what it
//! can stand on. Z is up (the viewer's convention).
//!
//! The triangles are the original's collision surfaces (`crate::surface`)
//! or, where a playfield has none, the props' visual meshes: steep
//! triangles are walls, flat ones upward floors.

use std::collections::HashMap;

pub type Triangle = [[f32; 3]; 3];

const CELL: f32 = 4.0;
/// A triangle whose normal's z (up) is at least this, either way (the
/// viewer's axis swap mirrors the meshes, so winding says nothing), is a
/// floor; the others are walls. The cut keeps slopes up to ~53 degrees
/// walkable.
const FLOOR_NZ: f32 = 0.6;

struct Tri {
    v: Triangle,
    /// Unit normal's z.
    nz: f32,
    min_z: f32,
    max_z: f32,
}

#[derive(Default)]
pub struct Collision {
    tris: Vec<Tri>,
    grid: HashMap<(i32, i32), Vec<u32>>,
}

fn cell(v: f32) -> i32 {
    (v / CELL).floor() as i32
}

/// The squared distance from (`px`, `py`) to the triangle's shadow on the
/// ground plane (0 inside it).
fn distance_sq_2d(t: &Triangle, px: f32, py: f32) -> f32 {
    closest_2d(t, px, py).map_or(0.0, |(cx, cy)| (cx - px) * (cx - px) + (cy - py) * (cy - py))
}

/// The nearest point of the triangle's shadow on the ground plane to
/// (`px`, `py`); none when the point is inside it.
fn closest_2d(t: &Triangle, px: f32, py: f32) -> Option<(f32, f32)> {
    let inside = {
        let s = |a: [f32; 3], b: [f32; 3]| (b[0] - a[0]) * (py - a[1]) - (b[1] - a[1]) * (px - a[0]);
        let (d0, d1, d2) = (s(t[0], t[1]), s(t[1], t[2]), s(t[2], t[0]));
        (d0 >= 0.0 && d1 >= 0.0 && d2 >= 0.0) || (d0 <= 0.0 && d1 <= 0.0 && d2 <= 0.0)
    };
    if inside {
        return None;
    }
    let mut best = (f32::MAX, (px, py));
    for i in 0..3 {
        let (a, b) = (t[i], t[(i + 1) % 3]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len2 = dx * dx + dy * dy;
        let u = if len2 > 0.0 { (((px - a[0]) * dx + (py - a[1]) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
        let (cx, cy) = (a[0] + u * dx, a[1] + u * dy);
        let d = (cx - px) * (cx - px) + (cy - py) * (cy - py);
        if d < best.0 {
            best = (d, (cx, cy));
        }
    }
    Some(best.1)
}

impl Collision {
    pub fn add(&mut self, v: Triangle) {
        let e1 = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
        let e2 = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
        let n = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len < 1e-6 {
            return;
        }
        let id = self.tris.len() as u32;
        let (lo_x, hi_x) = (v[0][0].min(v[1][0]).min(v[2][0]), v[0][0].max(v[1][0]).max(v[2][0]));
        let (lo_y, hi_y) = (v[0][1].min(v[1][1]).min(v[2][1]), v[0][1].max(v[1][1]).max(v[2][1]));
        self.tris.push(Tri { v, nz: n[2] / len, min_z: v[0][2].min(v[1][2]).min(v[2][2]), max_z: v[0][2].max(v[1][2]).max(v[2][2]) });
        for cx in cell(lo_x)..=cell(hi_x) {
            for cy in cell(lo_y)..=cell(hi_y) {
                self.grid.entry((cx, cy)).or_default().push(id);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.tris.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }

    /// The box around every triangle (min, max); none when empty.
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut it = self.tris.iter().flat_map(|t| t.v.iter());
        let first = *it.next()?;
        Some(it.fold((first, first), |(lo, hi), v| ([0, 1, 2].map(|c| lo[c].min(v[c])), [0, 1, 2].map(|c| hi[c].max(v[c])))))
    }

    fn near(&self, x: f32, y: f32, reach: f32) -> impl Iterator<Item = &Tri> {
        let (x0, x1, y0, y1) = (cell(x - reach), cell(x + reach), cell(y - reach), cell(y + reach));
        let mut ids: Vec<u32> = Vec::new();
        for cx in x0..=x1 {
            for cy in y0..=y1 {
                if let Some(list) = self.grid.get(&(cx, cy)) {
                    ids.extend_from_slice(list);
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter().map(|i| &self.tris[i as usize])
    }

    /// Whether a character of `radius` and `height`, with its feet at
    /// (`x`, `y`, `feet`), is inside a wall. Walls lower than `step` above
    /// the feet are stepped over.
    pub fn blocked(&self, x: f32, y: f32, feet: f32, radius: f32, height: f32, step: f32) -> bool {
        self.near(x, y, radius).any(|t| t.nz.abs() < FLOOR_NZ && t.max_z > feet + step && t.min_z < feet + height && distance_sq_2d(&t.v, x, y) < radius * radius)
    }

    /// The way out of the walls blocking a character at (`x`, `y`) (as
    /// [`Collision::blocked`] has them): the unit direction on the ground
    /// plane from the walls' nearest points towards the character, summed;
    /// none when nothing blocks or the walls push every way at once.
    pub fn wall_normal(&self, x: f32, y: f32, feet: f32, radius: f32, height: f32, step: f32) -> Option<[f32; 2]> {
        let mut sum = [0.0f32; 2];
        for t in self.near(x, y, radius) {
            if !(t.nz.abs() < FLOOR_NZ && t.max_z > feet + step && t.min_z < feet + height) {
                continue;
            }
            let away = match closest_2d(&t.v, x, y) {
                Some((cx, cy)) if (cx - x) * (cx - x) + (cy - y) * (cy - y) < radius * radius => [x - cx, y - cy],
                Some(_) => continue,
                // (standing over the wall's shadow: its own facing, either side, towards where the character came from is
                // unknown, so the side the centre is on)
                None => {
                    let (a, b, c) = (t.v[0], t.v[1], t.v[2]);
                    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                    let n = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2]];
                    let side = (x - a[0]) * n[0] + (y - a[1]) * n[1];
                    if side < 0.0 { [-n[0], -n[1]] } else { n }
                }
            };
            let len = (away[0] * away[0] + away[1] * away[1]).sqrt();
            if len > 1e-6 {
                sum = [sum[0] + away[0] / len, sum[1] + away[1] / len];
            }
        }
        let len = (sum[0] * sum[0] + sum[1] * sum[1]).sqrt();
        (len > 1e-3).then(|| [sum[0] / len, sum[1] / len])
    }

    /// The highest surface to stand on at (`x`, `y`): a floor triangle whose
    /// height there is at most `feet + step` (and not lower than
    /// `feet - drop`).
    pub fn floor(&self, x: f32, y: f32, feet: f32, step: f32, drop: f32) -> Option<f32> {
        let mut best: Option<f32> = None;
        for t in self.near(x, y, 0.0) {
            if t.nz.abs() < FLOOR_NZ || distance_sq_2d(&t.v, x, y) > 0.0 {
                continue;
            }
            // The plane through the triangle at (x, y).
            let (a, b, c) = (t.v[0], t.v[1], t.v[2]);
            let det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
            if det.abs() < 1e-9 {
                continue;
            }
            let u = ((x - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (y - a[1])) / det;
            let v = ((b[0] - a[0]) * (y - a[1]) - (x - a[0]) * (b[1] - a[1])) / det;
            let z = a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]);
            if z <= feet + step && z >= feet - drop && best.is_none_or(|b| z > b) {
                best = Some(z);
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wall along x = 10 from y 0..10, z 0..3 (two triangles).
    fn wall() -> Collision {
        let mut c = Collision::default();
        c.add([[10.0, 0.0, 0.0], [10.0, 10.0, 0.0], [10.0, 10.0, 3.0]]);
        c.add([[10.0, 0.0, 0.0], [10.0, 10.0, 3.0], [10.0, 0.0, 3.0]]);
        c
    }

    #[test]
    fn wall_blocks_near_and_not_far() {
        let c = wall();
        assert!(c.blocked(9.8, 5.0, 0.0, 0.4, 1.8, 0.5));
        assert!(!c.blocked(9.0, 5.0, 0.0, 0.4, 1.8, 0.5));
        assert!(!c.blocked(9.8, 15.0, 0.0, 0.4, 1.8, 0.5), "past its end");
        assert!(!c.blocked(9.8, 5.0, 3.5, 0.4, 1.8, 0.5), "standing above it");
    }

    #[test]
    fn a_wall_pushes_away_from_itself() {
        let c = wall();
        let n = c.wall_normal(9.8, 5.0, 0.0, 0.4, 1.8, 0.5).unwrap();
        assert!(n[0] < -0.99 && n[1].abs() < 0.01, "{n:?}");
        let n = c.wall_normal(10.2, 5.0, 0.0, 0.4, 1.8, 0.5).unwrap();
        assert!(n[0] > 0.99, "{n:?}");
        assert!(c.wall_normal(5.0, 5.0, 0.0, 0.4, 1.8, 0.5).is_none());
    }

    #[test]
    fn low_walls_are_stepped_over() {
        let mut c = Collision::default();
        c.add([[10.0, 0.0, 0.0], [10.0, 10.0, 0.0], [10.0, 10.0, 0.3]]);
        c.add([[10.0, 0.0, 0.0], [10.0, 10.0, 0.3], [10.0, 0.0, 0.3]]);
        assert!(!c.blocked(9.9, 5.0, 0.0, 0.4, 1.8, 0.5));
    }

    #[test]
    fn floor_height_follows_a_slope() {
        let mut c = Collision::default();
        // A ramp rising from z 0 at x 0 to z 2 at x 10.
        c.add([[0.0, 0.0, 0.0], [10.0, 0.0, 2.0], [10.0, 10.0, 2.0]]);
        c.add([[0.0, 0.0, 0.0], [10.0, 10.0, 2.0], [0.0, 10.0, 0.0]]);
        let z = c.floor(5.0, 5.0, 0.8, 0.6, 3.0).unwrap();
        assert!((z - 1.0).abs() < 1e-4, "{z}");
        assert_eq!(c.floor(20.0, 5.0, 0.0, 0.6, 3.0), None);
        assert_eq!(c.floor(9.0, 5.0, 0.0, 0.6, 3.0), None, "too high to step onto");
    }
}
