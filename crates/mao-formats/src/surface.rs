//! Collision surfaces (database type 0xf424d, one a zone: its instance the
//! zone's id, the playfield in the high half): N3.dll's
//! `n3SurfaceResource_t` (`ReadBlob` 0x10016179). The objects in the game
//! are version 5 (`ReadVersion5` 0x10015fc1): after the object's head a
//! range-coded KD tree (FUN_1002f1b0, the same static library city.dll and
//! Collision.dll link): the precision 0.01, the volume and node counts, the
//! quantized box, each volume's mesh (FUN_10012af0's coder), a marker, the
//! nodes; then whether a teleportal follows.
//!
//! Only the triangles are kept: the tree's nodes only sort the volumes for
//! the original's queries.

use crate::collision::Triangle;

pub const KIND: u32 = 0xf424d;
/// The object's head: type, instance, version.
const HEAD: usize = 12;
/// `_DAT_1003d618`.
const PRECISION: f32 = 0.01;

/// `a*b/c` as the original's `MUL`/`DIV` (a quotient over 32 bits, which
/// faults there, is a broken stream here).
fn scale(a: u32, b: u32, c: u32) -> Option<u32> {
    if c == 0 {
        return None;
    }
    u32::try_from(a as u64 * b as u64 / c as u64).ok()
}

/// The range decoder (FUN_1002f952 a symbol among `count`, FUN_1002f7b6 a
/// bit of two frequencies).
struct Reader<'a> {
    range: u32,
    code: u32,
    bytes: &'a [u8],
    at: usize,
    broken: bool,
}

impl Reader<'_> {
    fn normalize(&mut self) {
        while self.range < 0x0100_0000 {
            self.range <<= 8;
            self.code <<= 8;
            if self.at != self.bytes.len() {
                self.code |= self.bytes[self.at] as u32;
                self.at += 1;
            }
        }
    }

    fn checked(&mut self, v: Option<u32>) -> u32 {
        v.unwrap_or_else(|| {
            self.broken = true;
            0
        })
    }

    fn uniform(&mut self, count: u32) -> u32 {
        if self.broken {
            return 0;
        }
        self.normalize();
        let v = scale(self.code, count, self.range);
        let mut v = self.checked(v);
        let high = scale(v.wrapping_add(1), self.range, count);
        let mut high = self.checked(high);
        if high <= self.code {
            let h = scale(v.wrapping_add(2), self.range, count);
            high = self.checked(h);
            v = v.wrapping_add(1);
        }
        let low = scale(v, self.range, count);
        let low = self.checked(low);
        self.code = self.code.wrapping_sub(low);
        self.range = high.wrapping_sub(low);
        if self.range == 0 {
            self.broken = true;
        }
        v
    }

    fn bit(&mut self, f0: u32, f1: u32) -> bool {
        if self.broken {
            return false;
        }
        self.normalize();
        let split = scale(f0, self.range, f0.wrapping_add(f1));
        let split = self.checked(split);
        if self.code < split {
            self.range = split;
            false
        } else {
            self.range = self.range.wrapping_sub(split);
            self.code = self.code.wrapping_sub(split);
            true
        }
    }

    /// A count: 100 or more (among a million) or under (among 100).
    fn count(&mut self) -> u32 {
        if self.bit(1, 1) { self.uniform(1_000_000).wrapping_add(100) } else { self.uniform(100) }
    }
}

/// An open edge: (a, b), the triangle beyond it running b then a.
type Gate = (u32, u32);

/// FUN_10013f55: 167, 97 or 49 as the angle at `p.1` between `p.0` and
/// `q.1` is acute, right or obtuse; 0 without both.
fn angle(verts: &[[i32; 3]], p: Option<Gate>, q: Option<Gate>) -> u32 {
    let (Some(p), Some(q)) = (p, q) else { return 0 };
    let get = |i: u32| verts.get(i as usize).copied().unwrap_or_default();
    let (a, b, c) = (get(p.0), get(p.1), get(q.1));
    let d = (0..3).fold(0i32, |s, k| s.wrapping_add(c[k].wrapping_sub(b[k]).wrapping_mul(a[k].wrapping_sub(b[k]))));
    match d {
        0 => 97,
        d if d >= 0 => 167,
        _ => 49,
    }
}

/// The open edges, in order.
#[derive(Default)]
struct Gates(Vec<Gate>);

impl Gates {
    fn pop(&mut self) -> Option<Gate> {
        (!self.0.is_empty()).then(|| self.0.remove(0))
    }

    /// FUN_1001279b: the first ending at `v`.
    fn ending(&self, v: u32) -> Option<usize> {
        self.0.iter().position(|g| g.1 == v)
    }

    /// FUN_1001277c: the first starting at `v`.
    fn starting(&self, v: u32) -> Option<usize> {
        self.0.iter().position(|g| g.0 == v)
    }
}

/// A mesh being decoded.
struct Mesh {
    idx: Vec<u32>,
    verts: Vec<[i32; 3]>,
    min: [i32; 3],
    ext: [i32; 3],
}

impl Mesh {
    /// FUN_10013ff1.
    fn vertex(&mut self, s: &mut Reader) {
        let mut v = [0i32; 3];
        for c in 0..3 {
            let d = if !s.bit(0x44, 0x11) {
                s.uniform(self.ext[c].wrapping_sub(2) as u32).wrapping_add(1) as i32
            } else if !s.bit(3, 3) {
                self.ext[c] - 1
            } else {
                0
            };
            v[c] = self.min[c].wrapping_add(d);
        }
        self.verts.push(v);
    }

    fn triangle(&mut self, a: u32, b: u32, c: u32) -> usize {
        self.idx.extend([a, b, c]);
        self.idx.len() - 3
    }
}

/// The most triangles a volume is let have here (the original's reader
/// takes up to 999 a volume; a broken stream would otherwise run long).
const MESH_LIMIT: usize = 3 * 4000;

/// FUN_10012af0: one volume's mesh (indices and vertices on the grid), if
/// its end mark reads right.
fn decode_mesh(s: &mut Reader, q: &[i32; 6]) -> Option<(Vec<u32>, Vec<[i32; 3]>)> {
    let mut d = Mesh { idx: Vec::new(), verts: Vec::new(), min: [0; 3], ext: [0; 3] };
    for c in 0..3 {
        d.min[c] = s.uniform(q[3 + c].wrapping_sub(q[c]).wrapping_add(1) as u32).wrapping_add(q[c] as u32) as i32;
    }
    for c in 0..3 {
        d.ext[c] = s.uniform(q[3 + c].wrapping_sub(d.min[c]).wrapping_add(1) as u32).wrapping_add(1) as i32;
    }
    let flag = s.bit(3, 2);
    let mut gates = Gates::default();
    loop {
        let t = d.idx.len();
        for _ in 0..3 {
            let n = d.verts.len() as u32;
            d.idx.push(n);
            d.vertex(s);
        }
        for k in 0..3 {
            gates.0.push((d.idx[t + k], d.idx[t + (k + 1) % 3]));
        }
        while let Some(cur) = gates.pop() {
            if s.broken || d.idx.len() > MESH_LIMIT {
                return None;
            }
            if flag && !s.bit(2, 6) {
                continue;
            }
            let (g1, g2) = (gates.ending(cur.0), gates.starting(cur.1));
            if g1.is_some() || g2.is_some() {
                let (v1, v2) = (g1.map(|i| gates.0[i]), g2.map(|i| gates.0[i]));
                let c1 = angle(&d.verts, v1, Some(cur));
                let c2 = angle(&d.verts, Some(cur), v2);
                if s.bit(200, c1.wrapping_add(c2)) {
                    if let (Some(i1), Some(i2)) = (g1, g2)
                        && gates.0[i1].0 == gates.0[i2].1
                    {
                        let g = gates.0[i1];
                        d.triangle(g.1, g.0, cur.1);
                        gates.0.remove(i1.max(i2));
                        gates.0.remove(i1.min(i2));
                    } else if s.bit(c2, c1) {
                        let i = g1?;
                        let g = gates.0[i];
                        let p = d.triangle(g.1, g.0, cur.1);
                        gates.0[i] = (d.idx[p + 1], d.idx[p + 2]);
                    } else {
                        let i = g2?;
                        let g = gates.0[i];
                        let p = d.triangle(cur.1, cur.0, g.1);
                        gates.0[i] = (d.idx[p + 1], d.idx[p + 2]);
                    }
                    continue;
                }
            }
            // FUN_100144d1.
            let n = d.verts.len() as u32;
            let v = if !s.bit(1, 15) {
                s.uniform(n)
            } else {
                d.vertex(s);
                n
            };
            let p = d.triangle(v, cur.1, cur.0);
            gates.0.push((d.idx[p + 2], d.idx[p]));
            gates.0.push((d.idx[p], d.idx[p + 1]));
        }
        if s.broken || !s.bit(0x18, 1) {
            break;
        }
    }
    if s.broken || !s.bit(1, 999) {
        return None;
    }
    Some((d.idx, d.verts))
}

/// A zone's collision surface: its triangles in the game's coordinates
/// (y up).
#[derive(Debug, Default)]
pub struct Surface {
    pub triangles: Vec<Triangle>,
}

/// Reads a surface object (the whole database object, its head included);
/// none if it is not a version 5 surface or its stream breaks.
pub fn parse(object: &[u8]) -> Option<Surface> {
    let version = u32::from_le_bytes(object.get(8..HEAD)?.try_into().ok()?);
    if version != 5 {
        return None;
    }
    let mut r = Reader { range: 1, code: 0, bytes: &object[HEAD..], at: 0, broken: false };
    r.uniform(15);
    let volumes = r.count();
    let mut surface = Surface::default();
    if volumes == 0 {
        return Some(surface);
    }
    let _nodes = r.count();
    let mut q = [0i32; 6];
    for c in &mut q {
        *c = r.uniform(0x0100_0000).wrapping_sub(0x80_0000) as i32;
    }
    // (the grid's cells by their centres, FUN_10014307)
    let cell = |c: i32| ((c as f64 + 0.5) * PRECISION as f64) as f32;
    for _ in 0..volumes {
        let (idx, verts) = decode_mesh(&mut r, &q)?;
        let at = |i: u32| verts.get(i as usize).map(|v| v.map(cell));
        for t in idx.chunks_exact(3) {
            surface.triangles.push([at(t[0])?, at(t[1])?, at(t[2])?]);
        }
    }
    if r.broken || r.uniform(0xff) != 0x5a {
        return None;
    }
    Some(surface)
}
