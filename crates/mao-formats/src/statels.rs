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
    /// Whether the flags use the second layout (bit 0), of which only the
    /// rotation about Y and the stretch along X are read (its further matrix
    /// is left out).
    pub unread_rotation: bool,
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
    /// The rotation packed in the flags (`A >> 7`, with `A & 1` clear): a
    /// pitch of `v % 180 - 90` degrees (`v` first reduced by 0x163f500 when
    /// it is at least that, the pitch then 180 - 90), a yaw of the next
    /// `% 360` and a roll of what's left, in degrees, about Y, then X, then Z.
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
        quat_mul(quat_mul(a, b), c)
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
    let (rotation, scale_xyz) = if unread {
        // `A >> 7` is a turn about Y (v % 630 of 630 to the circle) and a
        // stretch s = (v / 630 % 211) / 100 + 0.5 along X.
        let v = (flags as u32) >> 7;
        let turn = (v % 630) as f32 * std::f32::consts::TAU / 630.0;
        let s = ((v / 630) % 211) as f32 / 100.0 + 0.5;
        (axis_angle([0.0, 1.0, 0.0], turn), [scale * s, scale, scale])
    } else {
        (Statel::unpack_rotation(flags), [scale; 3])
    };
    Ok(Some(Statel { position, flags, mesh, scale, scale_xyz, rotation, colour, unread_rotation: unread, extras: kept }))
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
    let mut failed = 0;
    for (k, &start) in offsets.iter().enumerate() {
        let end = offsets.get(k + 1).copied().unwrap_or(file.len());
        let Some(zone) = file.get(start..end.min(file.len())) else { continue };
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
        match result {
            Some(list) => out.extend(list),
            None => failed += 1,
        }
    }
    Ok((out, offsets.len(), failed))
}
