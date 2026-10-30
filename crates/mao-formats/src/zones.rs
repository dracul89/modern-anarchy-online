//! The zones' collision surfaces of a playfield as the vehicle queries
//! them (`KDTreeSurface_c`'s `GetLineIntersection` and
//! `CalculateClosestPoint` over every zone, `ao_kdtree`'s port of the KD
//! library): each triangle's record, found by a grid on the ground plane
//! in place of the trees (the same answers: the first front face along a
//! segment, the first upward face below a point). Game coordinates, y up.

use std::collections::HashMap;

use crate::collision::Triangle;

/// A triangle as the KD library records it for its queries (FUN_10011990):
/// its unit normal (the third edge crossed with the first, so the winding
/// picks the front), the outward normal of the plane through each edge
/// (perpendicular to the face), and the planes' offsets; its box.
#[derive(Clone, Debug)]
struct Record {
    normal: [f32; 3],
    d: f32,
    edge_normal: [[f32; 3]; 3],
    edge_d: [f32; 3],
    min: [f32; 3],
    max: [f32; 3],
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The library's dot product (y, x, z summed in doubles).
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[1] as f64 * b[1] as f64 + a[0] as f64 * b[0] as f64 + a[2] as f64 * b[2] as f64) as f32
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// FUN_10011990's normalization: by 1/length unless the length is 0, when
/// y becomes 1.
fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length == 0.0 {
        return [v[0], 1.0, v[2]];
    }
    let inverse = (1.0 / length as f64) as f32;
    [v[0] * inverse, v[1] * inverse, v[2] * inverse]
}

/// The slack of a triangle's parameter range (the double at 0x10016490).
const SLACK: f64 = 1.1920928955078125e-06;

impl Record {
    fn new(p: &Triangle) -> Record {
        let mut dir = [sub(p[1], p[0]), sub(p[2], p[1]), sub(p[0], p[2])];
        for d in &mut dir {
            let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if length != 0.0 {
                let inverse = (1.0 / length as f64) as f32;
                *d = [d[0] * inverse, d[1] * inverse, d[2] * inverse];
            }
        }
        let normal = unit(cross(dir[2], dir[0]));
        let d = -dot(p[0], normal);
        let edge_normal = [0, 1, 2].map(|i| unit(cross(dir[i], normal)));
        let edge_d = [0, 1, 2].map(|i| -dot(edge_normal[i], p[i]));
        let min = [0, 1, 2].map(|c| p[0][c].min(p[1][c]).min(p[2][c]));
        let max = [0, 1, 2].map(|c| p[0][c].max(p[1][c]).max(p[2][c]));
        Record { normal, d, edge_normal, edge_d, min, max }
    }

    /// FUN_10012014: where `start + s*d`, `s` from 0 to `max`, enters the
    /// triangle's front: the point and `s`.
    fn segment(&self, start: [f32; 3], d: [f32; 3], max: f32) -> Option<([f32; 3], f32)> {
        let dn = dot(d, self.normal);
        if dn >= 0.0 {
            return None;
        }
        let s = ((-(self.d as f64) - dot(start, self.normal) as f64) / dn as f64) as f32;
        if !(s as f64 + SLACK >= 0.0) || !(max as f64 + SLACK >= s as f64) {
            return None;
        }
        let hit = [start[0] + d[0] * s, start[1] + d[1] * s, start[2] + d[2] * s];
        for i in 0..3 {
            if dot(self.edge_normal[i], hit) as f64 + self.edge_d[i] as f64 > 0.0 {
                return None;
            }
        }
        Some((hit, s))
    }
}

/// The zones' surfaces of a playfield.
#[derive(Debug, Default)]
pub struct Zones {
    records: Vec<Record>,
    grid: HashMap<(i32, i32), Vec<u32>>,
    /// An interior's room cells on the ground plane (min x, min z, max x, max z), by the grid's cells: none outdoors.
    rooms: Option<HashMap<(i32, i32), Vec<[f32; 4]>>>,
}

const CELL: f32 = 4.0;

fn cell(v: f32) -> i32 {
    (v / CELL).floor() as i32
}

impl Zones {
    /// Adds a zone's triangles.
    pub fn add(&mut self, triangles: &[Triangle]) {
        for t in triangles {
            let r = Record::new(t);
            let id = self.records.len() as u32;
            for cx in cell(r.min[0])..=cell(r.max[0]) {
                for cz in cell(r.min[2])..=cell(r.max[2]) {
                    self.grid.entry((cx, cz)).or_default().push(id);
                }
            }
            self.records.push(r);
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Adds an interior's room cells ([`crate::rooms::floor_cells`]).
    pub fn add_rooms(&mut self, cells: &[[f32; 4]]) {
        let rooms = self.rooms.get_or_insert_with(HashMap::new);
        for c in cells {
            for cx in cell(c[0])..=cell(c[2]) {
                for cz in cell(c[1])..=cell(c[3]) {
                    rooms.entry((cx, cz)).or_default().push(*c);
                }
            }
        }
    }

    /// Whether (`x`, `z`) is over one of the interior's room cells (none outdoors).
    pub fn in_room(&self, x: f32, z: f32) -> Option<bool> {
        let rooms = self.rooms.as_ref()?;
        Some(rooms.get(&(cell(x), cell(z))).is_some_and(|cells| cells.iter().any(|c| x >= c[0] && x <= c[2] && z >= c[1] && z <= c[3])))
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The triangles in the cells the box on the ground plane touches.
    fn near(&self, lo: [f32; 2], hi: [f32; 2]) -> Vec<u32> {
        let (x0, x1, z0, z1) = (cell(lo[0]), cell(hi[0]), cell(lo[1]), cell(hi[1]));
        // (a segment across much of the playfield: every triangle)
        if (x1 as i64 - x0 as i64 + 1) * (z1 as i64 - z0 as i64 + 1) > 4096 {
            return (0..self.records.len() as u32).collect();
        }
        let mut ids = Vec::new();
        for cx in x0..=x1 {
            for cz in z0..=z1 {
                if let Some(list) = self.grid.get(&(cx, cz)) {
                    ids.extend_from_slice(list);
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// `GetLineIntersection` (FUN_1000b8ae over FUN_1000b0cd): the first
    /// front face from `start` to `end`: the point and the face's normal.
    pub fn line(&self, start: [f32; 3], end: [f32; 3]) -> Option<([f32; 3], [f32; 3])> {
        let d = sub(end, start);
        let lo = [start[0].min(end[0]), start[2].min(end[2])];
        let hi = [start[0].max(end[0]), start[2].max(end[2])];
        let mut best: Option<([f32; 3], [f32; 3], f32)> = None;
        for id in self.near(lo, hi) {
            let r = &self.records[id as usize];
            if (0..3).any(|c| r.min[c] > start[c].max(end[c]) || r.max[c] < start[c].min(end[c])) {
                continue;
            }
            let max = best.map_or(1.0, |b| b.2);
            if let Some((hit, s)) = r.segment(start, d, max)
                && best.is_none_or(|b| b.2 > s)
            {
                best = Some((hit, r.normal, s));
            }
        }
        best.map(|(hit, normal, _)| (hit, normal))
    }

    /// `CalculateClosestPoint` (FUN_1000b842 over FUN_1000b4fc): the first
    /// upward face dropping from `p` by at most `depth`: the point and its
    /// normal.
    pub fn floor_below(&self, p: [f32; 3], depth: f32) -> Option<([f32; 3], [f32; 3])> {
        let mut best: Option<([f32; 3], [f32; 3], f32)> = None;
        for id in self.near([p[0], p[2]], [p[0], p[2]]) {
            let r = &self.records[id as usize];
            if r.min[0] > p[0] || r.max[0] < p[0] || r.min[2] > p[2] || r.max[2] < p[2] || r.min[1] > p[1] {
                continue;
            }
            let max = best.map_or(depth, |b| b.2);
            if let Some((hit, s)) = r.segment(p, [0.0, -1.0, 0.0], max)
                && best.is_none_or(|b| b.2 > s)
            {
                best = Some(([p[0], hit[1], p[2]], r.normal, s));
            }
        }
        best.map(|(hit, normal, _)| (hit, normal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A floor at y 2 over x, z 0..10 (wound to face up) and a wall at x 5 facing -x.
    fn zones() -> Zones {
        let mut z = Zones::default();
        z.add(&[[[0.0, 2.0, 0.0], [0.0, 2.0, 10.0], [10.0, 2.0, 0.0]], [[10.0, 2.0, 0.0], [0.0, 2.0, 10.0], [10.0, 2.0, 10.0]]]);
        z.add(&[[[5.0, 0.0, 0.0], [5.0, 10.0, 0.0], [5.0, 0.0, 10.0]], [[5.0, 10.0, 0.0], [5.0, 10.0, 10.0], [5.0, 0.0, 10.0]]]);
        z
    }

    #[test]
    fn faces_are_hit_from_the_front_only() {
        let z = zones();
        let (hit, normal) = z.floor_below([3.0, 5.0, 3.0], 10.0).unwrap();
        assert!((hit[1] - 2.0).abs() < 1e-5 && normal[1] > 0.99, "{hit:?} {normal:?}");
        // (from below the floor: nothing)
        assert!(z.floor_below([3.0, 1.0, 3.0], 10.0).is_none());
        // the wall from one side only
        let a = z.line([2.0, 4.0, 3.0], [8.0, 4.0, 3.0]);
        let b = z.line([8.0, 4.0, 3.0], [2.0, 4.0, 3.0]);
        assert!(a.is_some() != b.is_some(), "{a:?} {b:?}");
        let (hit, normal) = a.or(b).unwrap();
        assert!((hit[0] - 5.0).abs() < 1e-5 && normal[0].abs() > 0.99, "{hit:?} {normal:?}");
    }
}
