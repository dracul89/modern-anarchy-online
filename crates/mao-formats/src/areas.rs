//! A playfield's areas (districts) and their environments: the `0xf425d` object, one per playfield, read by
//! `GameData::PlayfieldDistrictInfo_t::ReadBlob`'s sibling for areas (a format word, the zone count, a byte count of
//! `AreaData_t` records, then a byte per zone naming its area) with each record in `AreaData_t`'s stream form, version 4
//! (`GameData.dll`, ported in `dropin/gamedata/src/area.rs area_read`): the four texts in one sized block, the 26-byte
//! `EnvironmentData_t` (bytes 10 and 11 swapped in the stream, `values.rs environment_read`), the control percentages, the
//! level range, five sounds, two style words, the centre, then two groups of auto-content points, each followed by a zone word.
//!
//! What the client takes from an area is what `Environment_t` (Gamecode, FUN_100be0bb and its per-frame FUN_100becbe) takes:
//! the environment bytes 19–21 are the ambient light, 22–24 the fog's colour and 25 its density over 100 (FUN_100bd6d9 copies
//! them, FUN_100bcec6 divides by 255 and 100); bytes 0–7 are the weights of the eight weathers (an area with none of them is
//! left out of the blend, FUN_100bd555 giving it no weight); the centre is the area's, and its radius is the distance to the
//! nearest other area's centre (the constructor's loop, at most 100000, 1000 when that comes to nothing, FUN_100bcde5).
//!
//! Per frame (FUN_100beacd, FUN_100be9bb, FUN_100bf96e) the areas within their radius of the character are weighted
//! `1 - (d / r)²`, the weights scaled down when they sum past 1, and blended one after another into the playfield's own table
//! (zero here: the resource database holds no playfield-level `EnvironmentData_t`, and the constructor makes an empty one when
//! none is given): the light by the weight; the fog's colour by the two densities' shares, its density by the weight.

use crate::resource::{Id, ResourceDatabase};
use crate::texture::OBJECT_HEAD;

/// The type of a playfield's area list.
pub const KIND: u32 = 0xf425d;

/// An area's environment, as the original's state table holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub name: String,
    /// The centre (server coordinates: x, height, z).
    pub centre: [f32; 3],
    /// The raw `EnvironmentData_t`, in memory order.
    pub environment: [u8; 26],
    /// How far the area's environment reaches (see the module).
    pub radius: f32,
}

impl Area {
    /// The ambient light (bytes 19–21 over 255).
    pub fn ambient(&self) -> [f32; 3] {
        [self.environment[19] as f32 / 255.0, self.environment[20] as f32 / 255.0, self.environment[21] as f32 / 255.0]
    }

    /// The fog's colour (bytes 22–24 over 255).
    pub fn fog(&self) -> [f32; 3] {
        [self.environment[22] as f32 / 255.0, self.environment[23] as f32 / 255.0, self.environment[24] as f32 / 255.0]
    }

    /// The fog's density (byte 25 over 100).
    pub fn density(&self) -> f32 {
        self.environment[25] as f32 / 100.0
    }

    /// Whether the area takes part in the blend: it has some weather weight (bytes 0–7; FUN_100bd3b7 sums them, doubled and
    /// raised by 100 when above 1, and FUN_100bd555 gives a table with no sum no weight).
    pub fn takes_part(&self) -> bool {
        self.environment[..8].iter().any(|&b| b != 0)
    }
}

/// The environment at a point: what `Environment_t` hands `VisualAmbientLight_t::AddAmbientLight` and `VisualFog_t::AddFog`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Blend {
    pub ambient: [f32; 3],
    pub fog: [f32; 3],
    pub density: f32,
}

#[derive(Debug)]
pub enum Error {
    Truncated(&'static str),
    Layout(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Truncated(what) => write!(f, "area list truncated in {what}"),
            Error::Layout(what) => write!(f, "{what}"),
        }
    }
}

impl std::error::Error for Error {}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&[u8], Error> {
        let b = self.data.get(self.at..self.at + n).ok_or(Error::Truncated(what))?;
        self.at += n;
        Ok(b)
    }
    fn u8(&mut self, what: &'static str) -> Result<u8, Error> {
        Ok(self.take(1, what)?[0])
    }
    fn u16(&mut self, what: &'static str) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take(2, what)?.try_into().unwrap()))
    }
    fn u32(&mut self, what: &'static str) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }
    fn f32(&mut self, what: &'static str) -> Result<f32, Error> {
        Ok(f32::from_bits(self.u32(what)?))
    }

    /// One auto-content point, as `PointFactory_t` reads it (`points.rs factory_create`): the position and a flags word
    /// naming its type, then what the type adds — an entrance's sized name and style word, a fence's angle, a placement's
    /// two angles — and the zone word `area_read` takes after each.
    fn point(&mut self) -> Result<(), Error> {
        self.take(12, "a point's position")?;
        let flags = self.u32("a point's flags")?;
        let kind = match flags >> 24 {
            0 => flags & 3,
            1 => flags & 0xff_ffff,
            _ => return Err(Error::Layout("unknown point version")),
        };
        match kind + 1 {
            2..=4 => {
                let length = self.u16("an entrance's name length")? as usize;
                self.take(length, "an entrance's name")?;
                self.u32("an entrance's styles")?;
            }
            8 => {
                self.u16("a fence's angle")?;
            }
            9 => {
                self.take(4, "a placement's angles")?;
            }
            _ => {}
        }
        self.u16("a point's zone")?;
        Ok(())
    }
}

/// The areas of a playfield's `0xf425d` object.
pub fn parse(object: &[u8]) -> Result<Vec<Area>, Error> {
    let mut r = Reader { data: object, at: OBJECT_HEAD };
    let format = r.u16("the format")?;
    if !(5..=7).contains(&format) {
        return Err(Error::Layout("unknown area list format"));
    }
    let _zones = r.u32("the zone count")?;
    let count = r.u8("the area count")?;
    let mut areas = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let version = r.u16("an area's version")?;
        if version != 4 {
            return Err(Error::Layout("unknown area version"));
        }
        let size = r.u16("an area's text size")? as usize;
        let text = r.take(size, "an area's texts")?;
        let name = String::from_utf8_lossy(&text[..text.iter().position(|&b| b == 0).unwrap_or(text.len())]).into_owned();
        // The environment: 22 bytes with 10 and 11 swapped, then the last four (a fresh `EnvironmentData_t` has no size, so all are read).
        let stream = r.take(26, "an area's environment")?;
        let mut environment = [0u8; 26];
        for (i, &b) in stream.iter().enumerate() {
            let at = match i {
                10 => 11,
                11 => 10,
                _ => i,
            };
            environment[at] = b;
        }
        r.take(2, "an area's control")?;
        r.take(4, "an area's level range")?;
        r.take(20, "an area's sounds")?;
        r.take(8, "an area's styles")?;
        let centre = [r.f32("an area's centre")?, r.f32("an area's centre")?, r.f32("an area's centre")?];
        for _ in 0..2 {
            let points = r.u8("a point count")?;
            for _ in 0..points {
                r.point()?;
            }
        }
        areas.push(Area { name, centre, environment, radius: 0.0 });
    }
    // Each area reaches to the nearest other area's centre.
    for i in 0..areas.len() {
        let mut nearest = 100_000.0f32;
        for j in 0..areas.len() {
            if i != j {
                let d = distance(areas[i].centre, areas[j].centre);
                if d < nearest {
                    nearest = d;
                }
            }
        }
        areas[i].radius = if nearest > 0.0 { nearest } else { 1000.0 };
    }
    Ok(areas)
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// A playfield's areas from the database (none when the playfield has no list).
pub fn load(db: &ResourceDatabase, playfield: u32) -> Vec<Area> {
    match db.get(Id { kind: KIND, instance: playfield }) {
        Ok(Some(object)) => parse(&object).unwrap_or_else(|e| {
            eprintln!("playfield {playfield}: areas not read: {e}");
            Vec::new()
        }),
        _ => Vec::new(),
    }
}

/// The environment at `position` (server coordinates), blended as the original does each frame (see the module): the areas
/// within reach weighted `1 - (d / r)²`, scaled down when the weights sum past 1, and each lerped in turn into a zero table.
pub fn blend(areas: &[Area], position: [f32; 3]) -> Blend {
    blend_with_weather(areas, position, None).0
}

/// [`blend`] with each area's weather (`crate::weather::state` at `day` and `seconds`) blended the same way, the
/// first fourteen table values (FUN_100bf96e lerps the whole table).
pub fn blend_with_weather(areas: &[Area], position: [f32; 3], time: Option<(i32, f32)>) -> (Blend, crate::weather::Weather) {
    let mut weather = crate::weather::Weather::default();
    let mut weighted: Vec<(&Area, f32)> = Vec::new();
    let mut sum = 0.0f32;
    for a in areas.iter().filter(|a| a.takes_part()) {
        let d = distance(position, a.centre);
        if d < a.radius {
            let q = d / a.radius;
            let w = 1.0 - q * q;
            weighted.push((a, w));
            sum += w;
        }
    }
    if sum > 1.0 {
        for (_, w) in &mut weighted {
            *w /= sum;
        }
    }
    let mut out = Blend::default();
    for (a, w) in weighted {
        // FUN_100bf96e: the light by the weight; the fog's colour by the shares of the two densities, its density by the weight.
        let ambient = a.ambient();
        for k in 0..3 {
            out.ambient[k] = (1.0 - w) * out.ambient[k] + w * ambient[k];
        }
        if let Some((day, seconds)) = time {
            let own = crate::weather::state(&a.environment, day, seconds).unwrap_or_default();
            for k in 0..14 {
                weather.v[k] = (1.0 - w) * weather.v[k] + w * own.v[k];
            }
        }
        let kept = (1.0 - w) * out.density;
        let total = kept + w * a.density();
        let share = if total > 0.0 { kept / total } else { 0.0 };
        let fog = a.fog();
        for k in 0..3 {
            out.fog[k] = share * out.fog[k] + (1.0 - share) * fog[k];
        }
        out.density = total;
    }
    (out, weather)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, centre: [f32; 3], environment: [u8; 26], points: &[Vec<u8>]) -> Vec<u8> {
        let mut d = 4u16.to_le_bytes().to_vec();
        let texts: Vec<u8> = [name, "owner", "description", "knowledge"].iter().flat_map(|t| t.bytes().chain(std::iter::once(0))).collect();
        d.extend_from_slice(&(texts.len() as u16).to_le_bytes());
        d.extend_from_slice(&texts);
        // the stream order swaps bytes 10 and 11
        let mut stream = environment;
        stream.swap(10, 11);
        d.extend_from_slice(&stream);
        d.extend_from_slice(&[50, 50]);
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&200u16.to_le_bytes());
        d.extend_from_slice(&[0xff; 20]);
        d.extend_from_slice(&[0; 8]);
        for v in centre {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.push(points.len() as u8);
        for p in points {
            d.extend_from_slice(p);
        }
        d.push(0);
        d
    }

    fn object(records: &[Vec<u8>]) -> Vec<u8> {
        let mut d = vec![0u8; OBJECT_HEAD];
        d.extend_from_slice(&7u16.to_le_bytes());
        d.extend_from_slice(&3u32.to_le_bytes());
        d.push(records.len() as u8);
        for r in records {
            d.extend_from_slice(r);
        }
        d.extend_from_slice(&[0, 0, 0]);
        d
    }

    fn environment(ambient: [u8; 3], fog: [u8; 3], density: u8, weather: bool) -> [u8; 26] {
        let mut e = [0u8; 26];
        e[0] = weather as u8;
        e[10] = 7;
        e[11] = 9;
        e[19..22].copy_from_slice(&ambient);
        e[22..25].copy_from_slice(&fog);
        e[25] = density;
        e
    }

    #[test]
    fn areas_are_read_past_their_points() {
        // a plain point, an entrance and a placement point (the kinds with extra bytes), each with its zone word
        let mut plain = vec![0u8; 12];
        plain.extend_from_slice(&0x0100_0000u32.to_le_bytes());
        plain.extend_from_slice(&[1, 0]);
        let mut entrance = vec![0u8; 12];
        entrance.extend_from_slice(&0x0100_0001u32.to_le_bytes());
        entrance.extend_from_slice(&3u16.to_le_bytes());
        entrance.extend_from_slice(b"abc");
        entrance.extend_from_slice(&[0; 4]);
        entrance.extend_from_slice(&[2, 0]);
        let mut placement = vec![0u8; 12];
        placement.extend_from_slice(&0x0100_0008u32.to_le_bytes());
        placement.extend_from_slice(&[0; 4]);
        placement.extend_from_slice(&[3, 0]);
        let a = record("Nostep", [1220.0, 47.0, 1260.0], environment([10, 20, 30], [40, 50, 60], 30, true), &[plain, entrance, placement]);
        let b = record("Clan", [1220.0, 47.0, 1360.0], environment([100, 100, 100], [0, 0, 0], 0, true), &[]);
        let areas = parse(&object(&[a, b])).unwrap();
        assert_eq!(areas.len(), 2);
        assert_eq!(areas[0].name, "Nostep");
        assert_eq!(areas[0].centre, [1220.0, 47.0, 1260.0]);
        assert_eq!((areas[0].environment[10], areas[0].environment[11]), (7, 9));
        assert_eq!(areas[0].ambient(), [10.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0]);
        assert_eq!(areas[0].density(), 0.3);
        // each reaches the other's centre
        assert_eq!((areas[0].radius, areas[1].radius), (100.0, 100.0));
    }

    #[test]
    fn the_blend_weights_by_distance_and_leaves_out_areas_without_weather() {
        let mut areas = parse(&object(&[
            record("A", [0.0, 0.0, 0.0], environment([255, 0, 0], [255, 255, 255], 50, true), &[]),
            record("B", [100.0, 0.0, 0.0], environment([0, 255, 0], [0, 0, 0], 0, false), &[]),
        ]))
        .unwrap();
        // at A's centre: all of A
        let at_a = blend(&areas, [0.0, 0.0, 0.0]);
        assert_eq!(at_a.ambient, [1.0, 0.0, 0.0]);
        assert_eq!(at_a.density, 0.5);
        assert_eq!(at_a.fog, [1.0, 1.0, 1.0]);
        // halfway: a quarter of the way gone, so three quarters of A; B has no weather and adds nothing
        let mid = blend(&areas, [50.0, 0.0, 0.0]);
        assert!((mid.ambient[0] - 0.75).abs() < 1e-6 && mid.ambient[1] == 0.0);
        assert!((mid.density - 0.375).abs() < 1e-6);
        // beyond A's reach: nothing
        assert_eq!(blend(&areas, [150.0, 0.0, 0.0]), Blend::default());
        // with B taking part, the two overlap and the weights are scaled down to sum to 1 (a half each), then lerped in turn:
        // A leaves (0.5, 0, 0), B halves it and adds half of its own
        areas[1].environment[0] = 1;
        let mid = blend(&areas, [50.0, 0.0, 0.0]);
        assert!((mid.ambient[0] - 0.25).abs() < 1e-6, "{mid:?}");
        assert!((mid.ambient[1] - 0.5).abs() < 1e-6, "{mid:?}");
        assert!((mid.density - 0.125).abs() < 1e-6, "{mid:?}");
    }
}
