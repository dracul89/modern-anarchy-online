//! Statels: the static objects of a playfield, from its `<n>.pf` file in the
//! game's `cd_image/data/statels` folder.
//!
//! Derived from the original's `n3StatelController_t` and `n3Zone_t`
//! readers in `N3.dll` (read from the binary; `ReadStatelIndices`,
//! `ReadGlobalData`, the zone loader and the statel reader). All integers
//! are little-endian.
//!
//! # The file
//! `[1 u32]`, then the zones' offsets (u32, ascending: the table ends where
//! a value is smaller than the position it stands at), then the playfield's
//! global data (`[int]` — 4 skips the rest — else `[count u16]` global
//! statels), then the zones, each running to the next offset (the last to
//! the end of the file).
//!
//! # A zone
//! Counts are `u16`. Its first list is a count of `u16`s, then four lists of
//! statels (a count and that many each) and a list of objects of another
//! kind (rooms). A zone of the other layout (it starts with a packed block
//! and lights instead) isn't read here.
//!
//! # A statel
//! Position `x y z` (f32; the world is Y-up), flags `A` (i32), the mesh's
//! instance `B` (i32; 0 for none), bytes `C` (the scale) and `D` (how many
//! extras), then the extras (for each mask `i32`, each set bit's `i32`, in
//! bit order, `D` in all) and, when `A & 4`, a colour `i32`. The scale is
//! `C / 100 + 0.1`; the rotation is packed in `A >> 7` ([`Statel::rotation`]).

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Truncated(&'static str),
    Version(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated(what) => write!(f, "statel file truncated in {what}"),
            Error::Version(v) => write!(f, "statel file version {v} (1 is the only one known)"),
        }
    }
}

impl std::error::Error for Error {}

/// A placed static object.
#[derive(Clone, Debug)]
pub struct Statel {
    pub position: [f32; 3],
    pub flags: i32,
    /// The instance of its mesh (type 0xf6951).
    pub mesh: u32,
    pub scale: f32,
    /// The scale along each local axis (the same three for a plain statel).
    pub scale_xyz: [f32; 3],
    /// A quaternion (x, y, z, w).
    pub rotation: [f32; 4],
    pub colour: u32,
    /// Whether the flags use the second layout (bit 0): a rotation about Y, a stretch along X and a shear ([`shear`](Self::shear)).
    pub unread_rotation: bool,
    /// The second layout's shear (0 for the first): N3.dll's statel reader (FUN_1002777a) gives such a statel a matrix
    /// of its own (`VisualMesh_t::SetAnimMatrix`, through FUN_1002435c), which scales by (1, 1/s, 1/s) (FUN_10026cdc)
    /// and adds this times x to y (FUN_10026d5c), applied before the scale, turn and place; with the uniform scale
    /// `s × C` that makes the stretch along X ([`scale_xyz`](Self::scale_xyz)) and a y that rises `shear × s × C` per unit
    /// of x. It is `(v / 630 / 211) / 100 − 1.25` of the packed `v = A >> 7`. See [`Statel::matrix`].
    pub shear: f32,
    /// The extras: (mask number, bit, value) for each set bit of each mask.
    pub extras: Vec<(u32, u32, u32)>,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&[u8], Error> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len()).ok_or(Error::Truncated(what))?;
        let b = &self.data[self.at..end];
        self.at = end;
        Ok(b)
    }

    fn u32(&mut self, what: &'static str) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }

    fn u16(&mut self, what: &'static str) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take(2, what)?.try_into().unwrap()))
    }

    fn f32(&mut self, what: &'static str) -> Result<f32, Error> {
        Ok(f32::from_bits(self.u32(what)?))
    }
}

type Quat = [f32; 4];

/// A rotation of `angle` radians about the axis (x, y, z).
fn axis_angle(axis: [f32; 3], angle: f32) -> Quat {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    let (s, c) = (angle * 0.5).sin_cos();
    [axis[0] / len * s, axis[1] / len * s, axis[2] / len * s, c]
}

/// `a * b` (Hamilton).
fn quat_mul(a: Quat, b: Quat) -> Quat {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

impl Statel {
    /// The statel's placing matrix (row vectors, the world Y up), as the original's frame builds it
    /// (`update_world_matrix`: the anim matrix, then the uniform scale and the turn, then the place): for the second
    /// layout the anim matrix `[[1, shear, 0], [0, 1/s, 0], [0, 0, 1/s]]` before the scale `s × C`, which is the stretch
    /// `scale_xyz` with y rising `shear × s × C` per unit of x; for the first layout the scale and the turn alone.
    pub fn matrix(&self) -> crate::model::Matrix {
        let trs = crate::model::trs_matrix(self.position, self.rotation, self.scale_xyz);
        if !self.unread_rotation {
            return trs;
        }
        // (scale_xyz is (s C, C, C): the anim matrix's 1/s cancels s on y and z; its shear, met before the scale, is
        // `shear` × s × C of y per unit of x)
        let mut anim = crate::model::IDENTITY;
        anim[1] = self.shear * self.scale_xyz[0] / self.scale_xyz[1];
        crate::model::multiply(&anim, &trs)
    }

    /// The rotation packed in the flags (`A >> 7`, with `A & 1` clear): a
    /// pitch of `v % 180 - 90` degrees (`v` first reduced by 0x163f500 when
    /// it is at least that, the pitch then 180 - 90), a yaw of the next
    /// `% 360` and a roll of what's left, in degrees: the turn about Y first,
    /// then the pitch about the world's X, then the roll about the world's Z.
    /// N3.dll (FUN_10026e66 and FUN_1002777a, where 0x163f500 is tested) builds the
    /// yaw's quaternion and "multiplies" the pitch's and the roll's onto it with
    /// FUN_10003dc3, whose `a *= b` is Hamilton's `b * a`: the product is
    /// roll * pitch * yaw. (Until 2026-11-15 this was yaw * pitch * roll: the
    /// same for a statel turned about one axis, wrong for one turned and
    /// tilted, like the bridge pieces between ICC's landing platforms.)
    fn unpack_rotation(flags: i32) -> Quat {
        let mut v = (flags as u32) >> 7;
        let pitch = if v >= 0x163f500 {
            v -= 0x163f500;
            180
        } else {
            let p = v % 180;
            v /= 180;
            p
        };
        let yaw = v % 360;
        let roll = v / 360;
        let radians = |degrees: f32| degrees * std::f32::consts::TAU / 360.0;
        let a = axis_angle([0.0, 1.0, 0.0], radians(yaw as f32));
        let b = axis_angle([1.0, 0.0, 0.0], radians(pitch as f32 - 90.0));
        let c = axis_angle([0.0, 0.0, 1.0], radians(roll as f32));
        quat_mul(c, quat_mul(b, a))
    }
}

fn statel(r: &mut Reader) -> Result<Option<Statel>, Error> {
    let position = [r.f32("a position")?, r.f32("a position")?, r.f32("a position")?];
    let flags = r.u32("a statel's flags")? as i32;
    let mesh = r.u32("a statel's mesh")?;
    let c = r.take(2, "a statel's bytes")?;
    let (scale_byte, extras) = (c[0], c[1]);
    let mut left = extras as i32;
    let mut kept = Vec::new();
    let mut mask_number = 0;
    while left != 0 {
        let mask = r.u32("an extra mask")?;
        for bit in (0..32).filter(|b| mask & (1 << b) != 0) {
            kept.push((mask_number, bit, r.u32("an extra")?));
            left -= 1;
        }
        mask_number += 1;
        if mask == 0 && left != 0 && r.at >= r.data.len() {
            return Err(Error::Truncated("the extras"));
        }
    }
    let colour = if flags & 4 != 0 { r.u32("a colour")? } else { 0xff_ffff };
    if mesh == 0 {
        return Ok(None);
    }
    let unread = flags & 1 != 0;
    let scale = scale_byte as f32 / 100.0 + 0.1;
    let (rotation, scale_xyz, shear) = if unread {
        // `A >> 7` is a turn about Y (v % 630 of 630 to the circle), a
        // stretch s = (v / 630 % 211) / 100 + 0.5 along X and a shear
        // (v / 630 / 211) / 100 - 1.25 (FUN_1002777a).
        let v = (flags as u32) >> 7;
        let turn = (v % 630) as f32 * std::f32::consts::TAU / 630.0;
        let s = ((v / 630) % 211) as f32 / 100.0 + 0.5;
        let shear = ((v / 630) / 211) as f32 / 100.0 - 1.25;
        (axis_angle([0.0, 1.0, 0.0], turn), [scale * s, scale, scale], shear)
    } else {
        (Statel::unpack_rotation(flags), [scale; 3], 0.0)
    };
    Ok(Some(Statel { position, flags, mesh, scale, scale_xyz, rotation, colour, unread_rotation: unread, shear, extras: kept }))
}

fn statels(r: &mut Reader, out: &mut Vec<Statel>) -> Result<(), Error> {
    let count = r.u16("a statel count")?;
    for _ in 0..count {
        if let Some(s) = statel(r)? {
            out.push(s);
        }
    }
    Ok(())
}

/// All the statels of a playfield's file.
pub fn parse(file: &[u8]) -> Result<Vec<Statel>, Error> {
    Ok(parse_report(file)?.0)
}

/// The statels, how many zones the file has and how many of them couldn't
/// be read as the layout this module knows (their statels are left out).
pub fn parse_report(file: &[u8]) -> Result<(Vec<Statel>, usize, usize), Error> {
    let (mut out, zones) = parse_zoned(file)?;
    let failed = zones.iter().filter(|z| z.is_none()).count();
    let count = zones.len();
    out.extend(zones.into_iter().flatten().flatten());
    Ok((out, count, failed))
}

/// The playfield's global statels and each zone's own (`None` for a zone that couldn't be read). In an instanced interior
/// the zones are its rooms, in the rooms' order, and a zone's statels sit relative to its room's centre.
pub fn parse_zoned(file: &[u8]) -> Result<(Vec<Statel>, Vec<Option<Vec<Statel>>>), Error> {
    let mut r = Reader { data: file, at: 0 };
    let version = r.u32("the version")?;
    if version != 1 {
        return Err(Error::Version(version));
    }
    let mut offsets: Vec<usize> = Vec::new();
    loop {
        // The table ends where the first zone begins (or earlier, where a
        // value can't be an offset).
        if offsets.first().is_some_and(|&first| r.at >= first) {
            break;
        }
        let v = u32::from_le_bytes(file.get(r.at..r.at + 4).ok_or(Error::Truncated("the zone table"))?.try_into().unwrap()) as usize;
        if v < r.at + 4 || offsets.last().is_some_and(|&l| v < l) {
            break;
        }
        offsets.push(v);
        r.at += 4;
    }
    let mut out = Vec::new();
    // The playfield's global statels (a count) follow the table.
    let global = if r.at < offsets.first().copied().unwrap_or(0) { r.u32("the global data")? } else { 4 };
    if global != 4 {
        let count = r.u16("the global statel count")?;
        for _ in 0..count {
            // (A global statel that can't be read ends the global data.)
            match statel(&mut r) {
                Ok(Some(s)) => out.push(s),
                Ok(None) => {}
                Err(_) => break,
            }
        }
    }
    let mut zones_out: Vec<Option<Vec<Statel>>> = Vec::new();
    for (k, &start) in offsets.iter().enumerate() {
        let end = offsets.get(k + 1).copied().unwrap_or(file.len());
        let Some(zone) = file.get(start..end.min(file.len())) else {
            zones_out.push(None);
            continue;
        };
        // A zone has one of two layouts, told apart here by which one reads
        // exactly to the zone's end (the room objects at the end of the zone
        // can't be skipped, so a zone that has some is taken as it stands).
        let full = |z: &mut Reader| -> Result<Vec<Statel>, Error> {
            let mut list = Vec::new();
            let ints = z.u16("a zone's int count")?;
            z.take(ints as usize * 2, "a zone's shorts")?;
            for _ in 0..2 {
                statels(z, &mut list)?;
            }
            Ok(list)
        };
        let lite = |z: &mut Reader| -> Result<Vec<Statel>, Error> {
            // A packed block (a size, a count and the bytes), then two lists of lights.
            let size = z.u32("a packed block's size")? as usize;
            if size != 0 {
                z.u32("a packed block's length")?;
                z.take(size.checked_sub(4).ok_or(Error::Truncated("a packed block"))?, "a packed block")?;
            }
            for _ in 0..2 {
                let count = z.u16("a light count")?;
                z.take(count as usize * 18, "lights")?;
            }
            Ok(Vec::new())
        };
        let tail = |z: &mut Reader, mut list: Vec<Statel>| -> Result<(Vec<Statel>, bool), Error> {
            for _ in 0..2 {
                statels(z, &mut list)?;
            }
            let rooms = z.u16("a room count")?;
            Ok((list, rooms == 0 && z.at == z.data.len() || rooms != 0))
        };
        let mut result = None;
        for layout in [0, 1] {
            let mut z = Reader { data: zone, at: 0 };
            let first = if layout == 0 { full(&mut z) } else { lite(&mut z) };
            if let Ok((list, exact)) = first.and_then(|l| tail(&mut z, l)) {
                if exact {
                    result = Some(list);
                    break;
                }
            }
        }
        zones_out.push(result);
    }
    Ok((out, zones_out))
}

#[cfg(test)]
mod rotation_tests {
    use super::*;

    fn rotated(q: Quat, v: [f32; 3]) -> [f32; 3] {
        // (q v q*: Hamilton's rotation)
        let p = quat_mul(quat_mul(q, [v[0], v[1], v[2], 0.0]), [-q[0], -q[1], -q[2], q[3]]);
        [p[0], p[1], p[2]]
    }

    /// A statel of the second flag layout (bit 0): turn 10 of 630, stretch index 60, shear index 150
    /// (150 / 100 - 1.25 = 0.25), as FUN_1002777a unpacks it; its matrix raises y by shear × s × C per unit of x.
    #[test]
    fn second_layout_shears() {
        let v: u32 = 10 + 630 * (60 + 211 * 150);
        let mut bytes = Vec::new();
        for f in [1.0f32, 2.0, 3.0] {
            bytes.extend(f.to_le_bytes());
        }
        bytes.extend((((v << 7) | 1) as i32).to_le_bytes());
        bytes.extend(77i32.to_le_bytes());
        bytes.extend([90u8, 0]);
        let s = statel(&mut Reader { data: &bytes, at: 0 }).unwrap().unwrap();
        assert!(s.unread_rotation && (s.shear - 0.25).abs() < 1e-6);
        // C = 90 / 100 + 0.1 = 1.0, s = 60 / 100 + 0.5 = 1.1
        assert!((s.scale_xyz[0] - 1.1).abs() < 1e-6 && (s.scale_xyz[1] - 1.0).abs() < 1e-6);
        // without the turn: a unit along x goes 1.1 along x and 0.25 × 1.1 up
        let flat = Statel { rotation: [0.0, 0.0, 0.0, 1.0], position: [0.0; 3], ..s.clone() };
        let m = flat.matrix();
        assert!((m[0] - 1.1).abs() < 1e-6 && (m[1] - 0.275).abs() < 1e-6 && m[2].abs() < 1e-6, "{m:?}");
        assert!(m[4].abs() < 1e-6 && (m[5] - 1.0).abs() < 1e-6 && (m[10] - 1.0).abs() < 1e-6);
        // the first layout: no shear
        let first = Statel { unread_rotation: false, shear: 0.0, ..flat };
        assert_eq!(first.matrix(), crate::model::trs_matrix([0.0; 3], [0.0, 0.0, 0.0, 1.0], first.scale_xyz));
    }

    fn packed(yaw: u32, pitch: i32, roll: u32) -> i32 {
        ((((roll * 360 + yaw) * 180 + (pitch + 90) as u32) << 7) as i32) & !1
    }

    /// The two bridge pieces between ICC's landing platforms (playfield 655, mesh 295990, a piece that rises along
    /// its length): one stands as it is with a pitch of 26 degrees, its twin is turned half round with a pitch of -26.
    /// The pitch is about the world's X after the turn, so both lie flat the same way; in the other order one of them
    /// stood up at twice the slope.
    #[test]
    fn a_turned_statel_pitches_about_the_worlds_axis() {
        let a = Statel::unpack_rotation(packed(0, 26, 0));
        let b = Statel::unpack_rotation(packed(180, -26, 0));
        let slope = 26.0f32.to_radians();
        // the piece's own length: along -z and rising
        let length = [0.0, slope.sin(), -slope.cos()];
        let (ya, yb) = (rotated(a, length)[1], rotated(b, length)[1]);
        assert!((ya - yb).abs() < 1e-4, "{ya} {yb}");
        // a turn alone is as before
        let turn = Statel::unpack_rotation(packed(90, 0, 0));
        let x = rotated(turn, [0.0, 0.0, 1.0]);
        assert!((x[0] - 1.0).abs() < 1e-5 && x[2].abs() < 1e-5, "{x:?}");
    }
}
