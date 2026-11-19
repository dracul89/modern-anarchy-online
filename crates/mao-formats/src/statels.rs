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
//! Counts are `u16`. A zone has one of two layouts, and the file doesn't say
//! which: the original reads every zone of a room playfield (a dungeon or an
//! interior: its zones are its rooms, `n3RoomMonitor_t::EnableRoom` →
//! `n3StatelController_t::EnableRoomStatels` 0x10025ae8 → the loader built by
//! FUN_100274b1) in the room layout and every zone of an outdoor one (the tile
//! map's squares, `MapZonesInViewcone` → FUN_10028fe7 → FUN_1002739d) in the
//! tile layout ([`Layout`]); the loader's run (FUN_10028029) then reads the
//! rest, the same for both.
//! * The tile layout (FUN_10027cb2): a count of `u16`s (the playfield's
//!   global statels the zone holds too) and two lists of statels.
//! * The room layout (FUN_10027f16): a size word and, when it isn't 0, a
//!   count and `size - 4` bytes: the zone's statels' colours, `count` 565
//!   colours zlib-packed; then two lists of 18-byte lights (FUN_10027d90).
//! * Both: two lists of statels and a list of light objects (FUN_10026e66).
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
    /// Its colours, a 565 colour per vertex of its mesh (a statel of a room playfield's zone: FUN_100271b6 hands them out
    /// from the zone's packed colours, and the statel's visual gives them to `VisualMesh_t::SetVertexColours`, FUN_100240a5);
    /// empty for the others.
    pub light: Vec<u16>,
}

/// How a playfield's zones are laid out (see the module's notes): the original chooses by the playfield, not by the bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// An outdoor playfield's zones (FUN_10027cb2, the loader's flag +8 set by FUN_1002739d).
    Tiles,
    /// A room playfield's zones (FUN_10027f16, the flag clear from FUN_100274b1), with the playfield's data version (the
    /// definition's first word, `RDBPlayfield_t` +0x64, handed to the loader as its +0x2c): from version 10 on, each statel's
    /// colour count in the packed colours is two words.
    Rooms { version: u32 },
}

/// A light of a room's zone (FUN_10027d90: 18 bytes), in the room's own frame like its statels; the original turns and
/// places it by the room as it reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub position: [f32; 3],
    /// The colour word (ARGB); in the first list a top byte over 100 is made 100.
    pub colour: u32,
    pub value: i16,
}

/// A light object of a zone (FUN_10026e66), which the original makes a `VisualMesh_t` with `SetLight`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightMesh {
    pub position: [f32; 3],
    /// Turned as a statel of the first flag layout ([`Statel::rotation`]'s packing, the bits above 7 of its word).
    pub rotation: [f32; 4],
    /// The light's kind (the kind byte's low five bits; 2 and 4 carry the numbers below).
    pub kind: u8,
    /// The kind byte's bit 5 (handed on with the light).
    pub flag: bool,
    /// The colour bytes over 255.
    pub colour: [f32; 3],
    /// `SetLight`'s numbers: a word as it stands, then the next three over 100, 10000 and (10000 with the kind byte's
    /// bit 6, else 1000000), and for kind 4 two angles (degrees read, radians kept as 6.2832 / 360 × each).
    pub range: u16,
    pub numbers: [f32; 5],
}

/// A zone as the original reads it.
#[derive(Clone, Debug, Default)]
pub struct Zone {
    /// Its statels (those with a mesh), in the file's order: the tile layout's two lists, then the two lists both layouts have.
    pub statels: Vec<Statel>,
    /// The tile layout's first list: the playfield's global statels (by their place in the global list) the zone holds too
    /// (FUN_100275f6 adds them to the zone's lists).
    pub global: Vec<u16>,
    /// The room layout's two lists of lights.
    pub lights: [Vec<Light>; 2],
    /// The light objects (those the original makes: a kind byte with bit 7 clear).
    pub light_meshes: Vec<LightMesh>,
    /// The bytes after what was read (none in the game's files).
    pub unread: usize,
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
    Ok(Some(Statel { position, flags, mesh, scale, scale_xyz, rotation, colour, unread_rotation: unread, shear, extras: kept, light: Vec::new() }))
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

/// A light object (FUN_10026e66): `None` for one the original doesn't make (the kind byte's bit 7 set).
fn light_mesh(r: &mut Reader) -> Result<Option<LightMesh>, Error> {
    let position = [r.f32("a light's position")?, r.f32("a light's position")?, r.f32("a light's position")?];
    let flags = r.u32("a light's word")? as i32;
    let b = r.take(4, "a light's bytes")?;
    let (kind, colour) = (b[0], [b[1] as f32 / 255.0, b[2] as f32 / 255.0, b[3] as f32 / 255.0]);
    let (mut range, mut numbers) = (0u16, [0.0f32; 5]);
    if kind & 0x1f == 2 || kind & 0x1f == 4 {
        range = r.u16("a light's numbers")?;
        numbers[0] = r.u16("a light's numbers")? as f32 / 100.0;
        numbers[1] = r.u16("a light's numbers")? as f32 / 10000.0;
        numbers[2] = r.u16("a light's numbers")? as f32 / if kind & 0x40 != 0 { 10000.0 } else { 1000000.0 };
        if kind & 0x1f == 4 {
            numbers[3] = r.u16("a light's angles")? as f32 * 6.2832 / 360.0;
            numbers[4] = r.u16("a light's angles")? as f32 * 6.2832 / 360.0;
        }
    }
    if kind & 0x80 != 0 {
        return Ok(None);
    }
    Ok(Some(LightMesh { position, rotation: Statel::unpack_rotation(flags), kind: kind & 0x1f, flag: kind & 0x20 != 0, colour, range, numbers }))
}

/// FUN_10027d90's two lists of lights (in the first, a colour's top byte over 100 is made 100).
fn lights(r: &mut Reader) -> Result<[Vec<Light>; 2], Error> {
    let mut out = [Vec::new(), Vec::new()];
    for (k, list) in out.iter_mut().enumerate() {
        let count = r.u16("a light count")?;
        for _ in 0..count {
            let position = [r.f32("a light")?, r.f32("a light")?, r.f32("a light")?];
            let mut colour = r.u32("a light")?;
            let value = r.u16("a light")? as i16;
            if k == 0 && colour & 0xff00_0000 > 0x6400_0000 {
                colour = colour & 0xff_ffff | 0x6400_0000;
            }
            list.push(Light { position, colour, value });
        }
    }
    Ok(out)
}

/// The packed colours inflated to at most `size` bytes as zlib's `uncompress` does (FUN_1002a180, zlib 1.2.5): nothing
/// when the stream is bad, ends early or holds more than that.
fn inflate(packed: &[u8], size: usize) -> Vec<u16> {
    use std::io::Read;
    let mut out = Vec::new();
    match flate2::read::ZlibDecoder::new(packed).take(size as u64 + 1).read_to_end(&mut out) {
        Ok(n) if n <= size => out.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect(),
        _ => Vec::new(),
    }
}

/// FUN_100271b6's hand-out of a room zone's colours to its statels, in their order: each takes a count (a word, two from
/// version 10 on) and that many colours, until the colours run out. (The original bounds the place by the colours' size in
/// bytes, not in colours, and so reads past them where they run short: here they stop at their end.)
fn hand_out(colours: &[u16], wide: bool, statels: &mut [Statel]) {
    let mut at = 0usize;
    for s in statels {
        let Some(&low) = colours.get(at) else { return };
        let mut count = low as usize;
        at += 1;
        if wide {
            let Some(&high) = colours.get(at) else { return };
            count |= (high as usize) << 16;
            at += 1;
        }
        let Some(list) = colours.get(at..at + count) else { return };
        s.light = list.to_vec();
        at += count;
    }
}

/// One zone in the given layout (the loader's run, FUN_10028029).
pub fn read_zone(zone: &[u8], layout: Layout) -> Result<Zone, Error> {
    let mut z = Reader { data: zone, at: 0 };
    let mut out = Zone::default();
    let mut colours = Vec::new();
    match layout {
        Layout::Tiles => {
            let count = z.u16("a zone's global count")?;
            for _ in 0..count {
                out.global.push(z.u16("a zone's global statels")?);
            }
            for _ in 0..2 {
                statels(&mut z, &mut out.statels)?;
            }
        }
        Layout::Rooms { .. } => {
            let size = z.u32("a zone's colour size")? as usize;
            if size != 0 {
                let count = z.u32("a zone's colour count")? as usize;
                let packed = z.take(size.checked_sub(4).ok_or(Error::Truncated("a zone's colours"))?, "a zone's colours")?;
                if count != 0 {
                    colours = inflate(packed, count.saturating_mul(2));
                }
            }
            out.lights = lights(&mut z)?;
        }
    }
    let first = out.statels.len();
    for _ in 0..2 {
        statels(&mut z, &mut out.statels)?;
    }
    let count = z.u16("a zone's light count")?;
    for _ in 0..count {
        if let Some(l) = light_mesh(&mut z)? {
            out.light_meshes.push(l);
        }
    }
    if let Layout::Rooms { version } = layout {
        hand_out(&colours, version > 9, &mut out.statels[first..]);
    }
    out.unread = zone.len() - z.at;
    Ok(out)
}

/// All the statels of a playfield's file, its zones read in the layout the file seems to have ([`guess_layout`]).
pub fn parse(file: &[u8]) -> Result<Vec<Statel>, Error> {
    Ok(parse_report(file)?.0)
}

/// All the statels of a playfield's file, its zones read in `layout`.
pub fn parse_as(file: &[u8], layout: Layout) -> Result<Vec<Statel>, Error> {
    let (mut out, zones) = parse_zones(file, layout)?;
    out.extend(zones.into_iter().flatten().flat_map(|z| z.statels));
    Ok(out)
}

/// The statels, how many zones the file has and how many of them couldn't be read ([`parse`]; their statels are left out).
pub fn parse_report(file: &[u8]) -> Result<(Vec<Statel>, usize, usize), Error> {
    let (mut out, zones) = parse_zones(file, guess_layout(file))?;
    let failed = zones.iter().filter(|z| z.is_none()).count();
    let count = zones.len();
    out.extend(zones.into_iter().flatten().flat_map(|z| z.statels));
    Ok((out, count, failed))
}

/// The layout of a file's zones without its playfield's definition (which says it: see [`Layout`]): the room layout when
/// every zone reads to its end in it (true of every room playfield's file of the game and of no outdoor one's), with the
/// version unknown taken as 9. For the tools; the client reads the definition.
pub fn guess_layout(file: &[u8]) -> Layout {
    let rooms = Layout::Rooms { version: 9 };
    match zone_spans(file) {
        Ok((spans, _)) if !spans.is_empty() && spans.iter().all(|&(a, b)| file.get(a..b).and_then(|z| read_zone(z, rooms).ok()).is_some_and(|z| z.unread == 0)) => rooms,
        _ => Layout::Tiles,
    }
}

/// The playfield's global statels and each zone's own (`None` for a zone that couldn't be read). In an instanced interior
/// the zones are its rooms, in the rooms' order, and a zone's statels sit relative to its room's centre.
pub fn parse_zoned(file: &[u8], layout: Layout) -> Result<(Vec<Statel>, Vec<Option<Vec<Statel>>>), Error> {
    let (out, zones) = parse_zones(file, layout)?;
    Ok((out, zones.into_iter().map(|z| z.map(|z| z.statels)).collect()))
}

/// The zones' places in the file (start, end) and where the table ends.
fn zone_spans(file: &[u8]) -> Result<(Vec<(usize, usize)>, usize), Error> {
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
    let spans = offsets.iter().enumerate().map(|(k, &start)| (start, offsets.get(k + 1).copied().unwrap_or(file.len()))).collect();
    Ok((spans, r.at))
}

/// The playfield's global statels and each zone as read in `layout` (`None` for a zone that couldn't be read).
pub fn parse_zones(file: &[u8], layout: Layout) -> Result<(Vec<Statel>, Vec<Option<Zone>>), Error> {
    let (spans, table_end) = zone_spans(file)?;
    let mut r = Reader { data: file, at: table_end };
    let mut out = Vec::new();
    // The playfield's global statels (a count) follow the table.
    let global = if r.at < spans.first().map_or(0, |s| s.0) { r.u32("the global data")? } else { 4 };
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
    let zones = spans.iter().map(|&(start, end)| file.get(start..end.min(file.len())).and_then(|zone| read_zone(zone, layout).ok())).collect();
    Ok((out, zones))
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

#[cfg(test)]
mod zone_tests {
    use super::*;
    use std::io::Write;

    fn statel_bytes(mesh: u32, x: f32) -> Vec<u8> {
        let mut b = Vec::new();
        for f in [x, 0.0f32, 0.0] {
            b.extend(f.to_le_bytes());
        }
        b.extend(0i32.to_le_bytes());
        b.extend(mesh.to_le_bytes());
        b.extend([90u8, 0]);
        b
    }

    fn list(statels: &[Vec<u8>]) -> Vec<u8> {
        let mut b = (statels.len() as u16).to_le_bytes().to_vec();
        for s in statels {
            b.extend(s);
        }
        b
    }

    /// A room zone (FUN_10027f16, then FUN_10028029's rest): the packed colours handed out to the statels with a mesh in
    /// order (FUN_100271b6: a word of count each, two from version 10), the lights with the first list's alpha made at most
    /// 100, and a light object of kind 4 with its numbers, one of bit 7 left out.
    #[test]
    fn a_room_zone() {
        for version in [9u32, 10] {
            let wide = version > 9;
            let mut colours: Vec<u16> = Vec::new();
            for (n, first) in [(2u16, 0x1111u16), (3, 0x2222)] {
                colours.push(n);
                if wide {
                    colours.push(0);
                }
                colours.extend((0..n).map(|k| first + k));
            }
            let raw: Vec<u8> = colours.iter().flat_map(|c| c.to_le_bytes()).collect();
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            z.write_all(&raw).unwrap();
            let packed = z.finish().unwrap();
            let mut b = Vec::new();
            b.extend((packed.len() as u32 + 4).to_le_bytes());
            b.extend((colours.len() as u32).to_le_bytes());
            b.extend(&packed);
            // the lights: one in each list, both with alpha 0xff
            for _ in 0..2 {
                b.extend(1u16.to_le_bytes());
                for f in [1.0f32, 2.0, 3.0] {
                    b.extend(f.to_le_bytes());
                }
                b.extend(0xff12_3456u32.to_le_bytes());
                b.extend((-5i16).to_le_bytes());
            }
            // the statels: a mesh, none (no colours for it), a mesh; then an empty list
            b.extend(list(&[statel_bytes(7, 1.0), statel_bytes(0, 2.0), statel_bytes(8, 3.0)]));
            b.extend(list(&[]));
            // the light objects: kind 4 with bit 6, then one with bit 7 (read past, not made)
            b.extend(2u16.to_le_bytes());
            for (kind, more) in [(0x44u8, true), (0x82u8, true)] {
                for f in [4.0f32, 5.0, 6.0] {
                    b.extend(f.to_le_bytes());
                }
                b.extend(0i32.to_le_bytes());
                b.extend([kind, 255, 0, 51]);
                if more {
                    for v in [300u16, 150, 5000, 20000] {
                        b.extend(v.to_le_bytes());
                    }
                    if kind & 0x1f == 4 {
                        b.extend(90u16.to_le_bytes());
                        b.extend(180u16.to_le_bytes());
                    }
                }
            }
            let zone = read_zone(&b, Layout::Rooms { version }).unwrap();
            assert_eq!(zone.unread, 0);
            assert_eq!(zone.statels.iter().map(|s| s.mesh).collect::<Vec<_>>(), [7, 8]);
            assert_eq!(zone.statels[0].light, [0x1111, 0x1112]);
            assert_eq!(zone.statels[1].light, [0x2222, 0x2223, 0x2224]);
            assert_eq!(zone.lights[0][0].colour, 0x6412_3456);
            assert_eq!(zone.lights[1][0].colour, 0xff12_3456);
            assert_eq!((zone.lights[0][0].position, zone.lights[0][0].value), ([1.0, 2.0, 3.0], -5));
            assert_eq!(zone.light_meshes.len(), 1);
            let l = zone.light_meshes[0];
            assert_eq!((l.kind, l.flag, l.range), (4, false, 300));
            assert_eq!(l.colour, [1.0, 0.0, 0.2]);
            assert!((l.numbers[0] - 1.5).abs() < 1e-6 && (l.numbers[1] - 0.5).abs() < 1e-6 && (l.numbers[2] - 2.0).abs() < 1e-6);
            assert!((l.numbers[3] - 6.2832 / 4.0).abs() < 1e-5 && (l.numbers[4] - 6.2832 / 2.0).abs() < 1e-5);
            // the same bytes in the tile layout don't read to the end
            assert!(read_zone(&b, Layout::Tiles).map_or(true, |z| z.unread != 0));
        }
    }

    /// A tile zone (FUN_10027cb2): the global statels it holds, its two lists, then the two lists and the light objects.
    #[test]
    fn a_tile_zone() {
        let mut b = Vec::new();
        b.extend(2u16.to_le_bytes());
        b.extend(3u16.to_le_bytes());
        b.extend(9u16.to_le_bytes());
        b.extend(list(&[statel_bytes(1, 1.0)]));
        b.extend(list(&[]));
        b.extend(list(&[statel_bytes(2, 2.0)]));
        b.extend(list(&[statel_bytes(3, 3.0)]));
        b.extend(0u16.to_le_bytes());
        let zone = read_zone(&b, Layout::Tiles).unwrap();
        assert_eq!((zone.global.as_slice(), zone.unread), (&[3u16, 9][..], 0));
        assert_eq!(zone.statels.iter().map(|s| s.mesh).collect::<Vec<_>>(), [1, 2, 3]);
        assert!(zone.statels.iter().all(|s| s.light.is_empty()));
    }
}
