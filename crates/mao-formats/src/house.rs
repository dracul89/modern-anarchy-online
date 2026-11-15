//! The buildings of a player city: the house templates (database type 0xf4268, `RDBHouseTemplate_t`; city.dll's
//! `CityHouseTemplate_c`, read by its `operator>>` at 0x1000f1a7). A city's playfield gets the list of its houses from the
//! server (`PlayfieldAllCities`: a place, a rotation in degrees and a template each); everything else about a house is
//! its template's: the tiles it stands on, the meshes it is drawn with (each with a place inside the footprint and a
//! rotation), where its doors and guards go (the server's business), and a collision surface.
//!
//! The record is little-endian. After the object's 12-byte head: the format's version (4 in every record but two old
//! ones, which are not read here), an identity (zero), the interior's number, a "fixtures allowed" word, a counted name
//! (empty), a word, the stealth factor, the upkeep, the house's type, the layout (a bit count, the tiles along X and Z
//! and the bits, a byte for every eight), then four lists counted `(n + 1) * 1009`: the visuals (mesh, x, y, z, rotation),
//! the doors (item template, x, y, z, rotation), the guards (a four-letter name, level, x, y, z, rotation) and the
//! advantages (a counted name and a level), and what is left is the surface (the same range-coded tree as a zone's,
//! [`crate::surface::parse_bare`]).
//!
//! A tile is 4 units. Places inside a template are measured from the least corner of its footprint as it stands
//! unturned; a house's own place is the least corner of the footprint as it stands turned ([`Template::rotate_local`]).

use crate::collision::Triangle;

pub const KIND: u32 = 0xf4268;
/// A tile's side (city.dll's double at 0x1002a258).
pub const TILE: f32 = 4.0;
const HEAD: usize = 12;

/// One mesh of a house.
#[derive(Clone, Debug, PartialEq)]
pub struct Visual {
    pub mesh: u32,
    pub position: [f32; 3],
    /// Degrees.
    pub rotation: i32,
}

/// Where the server puts one of the house's doors (the client makes none itself).
#[derive(Clone, Debug, PartialEq)]
pub struct Door {
    /// The door's item template.
    pub template: u32,
    pub position: [f32; 3],
    /// Degrees, as the original keeps it: the stored number negated.
    pub rotation: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Template {
    /// 0 a house, 1 headquarters, 2 market, 3 grid, 4 guard house, 5 radar station, 6 cloaking device, 7 a small one,
    /// 8 tax office, 9 a unique house (`fHouseTypeToString`, 0x10014f62). It changes nothing in how the house is drawn.
    pub kind: i32,
    /// `GetInsideTemplateInstance` (a guess: the interior's playfield).
    pub inside: u32,
    pub tiles_x: i32,
    pub tiles_z: i32,
    pub visuals: Vec<Visual>,
    pub doors: Vec<Door>,
    /// The collision surface's bytes (none for a house without one).
    pub surface: Vec<u8>,
}

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.data.get(self.at..self.at.checked_add(n)?)?;
        self.at += n;
        Some(s)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }
    fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }
    fn place(&mut self) -> Option<[f32; 3]> {
        Some([self.f32()?, self.f32()?, self.f32()?])
    }
    fn text(&mut self) -> Option<()> {
        let n = u16::from_le_bytes(self.take(2)?.try_into().ok()?) as usize;
        self.take(n).map(|_| ())
    }
    /// A list's count, `(n + 1) * 1009`: a number that is not a multiple, or a count over 30000, breaks the stream.
    fn count(&mut self) -> Option<usize> {
        let w = self.u32()?;
        (w % 1009 == 0 && w >= 1009 && w / 1009 - 1 < 30001).then(|| (w / 1009 - 1) as usize)
    }
}

/// A turn about the up axis by minus `degrees`: the quaternion (x, y, z, w) the original gives a house's mesh
/// (Gamecode FUN_100520d9 with the axis (0, 1, 0)) and turns a template's places by.
pub fn heading(degrees: i32) -> [f32; 4] {
    let half = -(degrees as f32).to_radians() * 0.5;
    [0.0, half.sin(), 0.0, half.cos()]
}

/// `p` turned about the up axis by minus `degrees`.
fn turned(p: [f32; 3], degrees: i32) -> [f32; 3] {
    // (exact for the quarter turns the houses stand at)
    let (s, c) = match degrees.rem_euclid(360) {
        0 => (0.0, 1.0),
        90 => (-1.0, 0.0),
        180 => (0.0, -1.0),
        270 => (1.0, 0.0),
        d => (-(d as f32).to_radians()).sin_cos(),
    };
    [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c]
}

impl Template {
    /// The record (the whole database object, its head included); none when it is not a version 4 one or it ends early.
    pub fn parse(record: &[u8]) -> Option<Template> {
        let mut r = Cursor { data: record, at: HEAD };
        if r.i32()? != 4 {
            return None;
        }
        r.take(8)?;
        let inside = r.u32()?;
        r.u32()?;
        r.text()?;
        r.take(12)?;
        let kind = r.i32()?;
        let (bits, tiles_x, tiles_z) = (r.i32()?, r.i32()?, r.i32()?);
        if bits < 0 || tiles_x <= 0 || tiles_z <= 0 {
            return None;
        }
        r.take((bits as usize).div_ceil(8))?;
        let visuals = (0..r.count()?).map(|_| Some(Visual { mesh: r.u32()?, position: r.place()?, rotation: r.i32()? })).collect::<Option<Vec<_>>>()?;
        let doors = (0..r.count()?).map(|_| Some(Door { template: r.u32()?, position: r.place()?, rotation: r.i32()?.wrapping_neg() })).collect::<Option<Vec<_>>>()?;
        for _ in 0..r.count()? {
            r.take(24)?;
        }
        for _ in 0..r.count()? {
            r.text()?;
            r.u32()?;
        }
        Some(Template { kind, inside, tiles_x, tiles_z, visuals, doors, surface: record[r.at..].to_vec() })
    }

    /// A place inside the template for a house turned by `degrees` (`CityHouseTemplate_c::RotateLocalPos`,
    /// 0x1000e0db): turned about the middle of the unturned footprint, and the middle put back where the turned
    /// footprint has it, so that the result is again measured from the least corner. For a quarter turn (x, z)
    /// becomes (Lz - z, x), for a half (Lx - x, Lz - z), for three quarters (z, Lx - x).
    pub fn rotate_local(&self, p: [f32; 3], degrees: i32) -> [f32; 3] {
        let c = [self.tiles_x as f32 * TILE * 0.5, 0.0, self.tiles_z as f32 * TILE * 0.5];
        let moved = turned([p[0] - c[0], p[1] - c[1], p[2] - c[2]], degrees);
        let middle = turned(c, degrees);
        [moved[0] + middle[0].abs(), moved[1] + middle[1].abs(), moved[2] + middle[2].abs()]
    }

    /// The collision surface's triangles for a house turned by `degrees`, measured from the house's place
    /// (`GetKDTree`, 0x1000eafc: the rotation snapped to a quarter turn, the tree read through the same turn as the
    /// places). None when the template has no surface or it cannot be read.
    pub fn triangles(&self, degrees: i32) -> Option<Vec<Triangle>> {
        if self.surface.len() < 4 {
            return None;
        }
        let snapped = degrees.rem_euclid(360) / 90 * 90;
        let surface = crate::surface::parse_bare(&self.surface)?;
        Some(surface.triangles.into_iter().map(|t| t.map(|v| self.rotate_local(v, snapped))).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_turns_about_the_footprints_middle() {
        // (template 0x6a0006: 20 by 15 tiles, its mesh at (38.2311, 0.0751, 29.6345); the original's functions, run
        // as a copy, give (30.366, 0.075, 38.231) at 90 degrees)
        let t = Template { tiles_x: 20, tiles_z: 15, ..Default::default() };
        let p = [38.2311, 0.0751, 29.6345];
        let close = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3);
        assert!(close(t.rotate_local(p, 0), p));
        assert!(close(t.rotate_local(p, 90), [60.0 - 29.6345, 0.0751, 38.2311]), "{:?}", t.rotate_local(p, 90));
        assert!(close(t.rotate_local(p, 180), [80.0 - 38.2311, 0.0751, 60.0 - 29.6345]));
        assert!(close(t.rotate_local(p, 270), [29.6345, 0.0751, 80.0 - 38.2311]));
        // (the footprint's corners stay in the turned footprint, least corner at nothing)
        assert!(close(t.rotate_local([0.0, 0.0, 0.0], 90), [60.0, 0.0, 0.0]));
        assert!(close(t.rotate_local([80.0, 0.0, 60.0], 90), [0.0, 0.0, 80.0]));
    }

    #[test]
    fn a_mesh_turned_a_quarter_looks_along_z() {
        // (the house's mesh is turned as its places are: its +X towards +Z at 90 degrees)
        let q = heading(90);
        let (y, w) = (q[1], q[3]);
        // the X axis under the quaternion (0, y, 0, w): (1 - 2yy, 0, -2yw)
        let x = [1.0 - 2.0 * y * y, 0.0, -2.0 * y * w];
        assert!(x[0].abs() < 1e-6 && (x[2] - 1.0).abs() < 1e-6, "{x:?}");
        assert_eq!(turned([1.0, 0.0, 0.0], 90), [0.0, 0.0, 1.0]);
    }
}
