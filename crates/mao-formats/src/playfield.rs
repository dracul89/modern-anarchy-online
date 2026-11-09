//! Playfields: the zones of the world. The resource database has one
//! `0xf4241` object per playfield (the instance is the playfield's number)
//! whose data is a version (10), the number again and the name as a
//! NUL-terminated string.

use crate::resource::{Id, ResourceDatabase};
use crate::texture::OBJECT_HEAD;

/// A playfield's number and name.
#[derive(Clone, Debug)]
pub struct Playfield {
    pub number: u32,
    pub name: String,
}

/// The type of a playfield's definition object.
pub const KIND: u32 = 0xf4241;

/// A playfield's name from its definition object's bytes.
pub fn name(object: &[u8]) -> Option<String> {
    let text = object.get(OBJECT_HEAD + 8..)?;
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    Some(String::from_utf8_lossy(&text[..end]).into_owned())
}

/// A triangle of liquid from the playfield's water polygons (server coordinates: x, height, z).
#[derive(Clone, Debug, PartialEq)]
pub struct WaterTriangle {
    /// The liquid type (`flags & 0x1f`; the low bit set means a falling liquid, the type over two the layer: water,
    /// toxic, lava, …).
    pub kind: u32,
    /// `(flags >> 5) / 10`, 100000 when that is 0.
    pub value: f32,
    pub points: [[f32; 3]; 3],
}

/// What Gamecode's `RDBPlayfieldAnarchy_t::ReadBlob` (FUN_10124692) reads after N3's `RDBPlayfield_t::ReadBlob`: the
/// water polygons (a count, then per polygon a flags word, its points and its index triangles; a polygon with the low
/// flag bit clear and more than three points is fanned round its centre, the others take their triangles) and, after
/// two words and a size, the playfield's own `EnvironmentData_t` (`GameData::operator>>`: 22 bytes with 10 and 11
/// swapped, three more unless the size is 0x1a, the last unless it is 0x1d).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Anarchy {
    pub water: Vec<WaterTriangle>,
    pub environment: Option<[u8; 26]>,
}

/// The Anarchy part of a playfield definition object. N3's reader comes first: the header (`rooms.rs`), then for an
/// outdoor playfield its zones — each a camera attractor count and the attractors (position, rotation, a direction
/// from version 6, a float) — or for an interior its rooms.
pub fn anarchy(object: &[u8]) -> Option<Anarchy> {
    let (rooms, mut at) = crate::rooms::parse_with_end(object).ok()?;
    let u32_at = |o: usize| object.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let f32_at = |o: usize| u32_at(o).map(f32::from_bits);
    let version = u32_at(OBJECT_HEAD)?;
    if rooms.is_none() {
        let number = u32_at(OBJECT_HEAD + 4)?;
        let tilemap = u32_at(OBJECT_HEAD + 40)?;
        let zones = u32_at(OBJECT_HEAD + 48)?;
        if number != tilemap || zones == 0 || zones >= 50_000 {
            return None;
        }
        let attractor = 12 + 16 + if version >= 6 { 12 } else { 0 } + 4;
        for _ in 0..zones {
            if version > 4 {
                let n = u32_at(at)? as usize;
                if n > 999 {
                    return None;
                }
                at += 4 + n * attractor;
            }
        }
    }
    let count = u32_at(at)? as usize;
    at += 4;
    let mut water = Vec::new();
    for _ in 0..count {
        let flags = u32_at(at)?;
        let n = u32_at(at + 4)? as usize;
        at += 8;
        if n > 100_000 {
            return None;
        }
        let mut points = Vec::with_capacity(n);
        for k in 0..n {
            points.push([f32_at(at + k * 12)?, f32_at(at + k * 12 + 4)?, f32_at(at + k * 12 + 8)?]);
        }
        at += n * 12;
        let triangles = u32_at(at)? as usize;
        at += 4;
        let index = |k: usize| object.get(at + k * 2..at + k * 2 + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
        let kind = flags & 0x1f;
        let value = {
            let v = (flags >> 5) as f32 / 10.0;
            if v == 0.0 { 100_000.0 } else { v }
        };
        if flags & 1 == 0 && n != 3 {
            if n > 0 {
                let mut centre = [0.0f32; 3];
                for p in &points {
                    for c in 0..3 {
                        centre[c] += p[c];
                    }
                }
                let centre = centre.map(|v| v / n as f32);
                for k in 0..n {
                    let before = if k == 0 { n - 1 } else { k - 1 };
                    water.push(WaterTriangle { kind, value, points: [centre, points[before], points[k]] });
                }
            }
        } else {
            for t in 0..triangles {
                let (a, b, c) = (index(t * 3)?, index(t * 3 + 1)?, index(t * 3 + 2)?);
                if a < n && b < n && c < n {
                    water.push(WaterTriangle { kind, value, points: [points[a], points[b], points[c]] });
                }
            }
        }
        at += triangles * 6;
    }
    at += 8;
    let size = u32_at(at)?;
    at += 4;
    let environment = if size == 0 {
        None
    } else {
        let mut env = [0u8; 26];
        let mut read = |slot: usize| -> Option<()> {
            env[slot] = *object.get(at)?;
            at += 1;
            Some(())
        };
        for slot in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21] {
            read(slot)?;
        }
        if size != 0x1a {
            for slot in 22..25 {
                read(slot)?;
            }
            if size != 0x1d {
                read(25)?;
            }
        }
        Some(env)
    };
    Some(Anarchy { water, environment })
}

impl Anarchy {
    /// The sky kind of the playfield's own environment data (its byte 0x12, which `n3EngineClientAnarchy_t::PlayfieldInit`
    /// reads from `PlayfieldAnarchy_t`'s copy of it); 0 without one.
    pub fn environment_kind(&self) -> u8 {
        self.environment.map_or(0, |e| e[18])
    }
}

/// The effect scripts a playfield's sky and environment come from (see [`sky_scripts`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkyScripts {
    /// Another playfield whose `Tweak_Playfield_<n>` is used in place of the playfield's own, when the kind says so.
    pub playfield: Option<u32>,
    /// `LoadPFTweakData`'s environment: 0 outdoors, 1 indoors, 2 the orbit, 3 the sand skyline, 4 the rock skyline.
    pub environment: u32,
}

/// How the original picks a playfield's sky scripts from its environment kind ([`Anarchy::environment_kind`]) and whether
/// its tile-map is a ground: `n3EngineClientAnarchy_t::PlayfieldInit` (Gamecode 0x10016e94) turns the kind into the
/// environment type (`n3Playfield_t` +0xd8: on a ground 6..11 become 7..12 and the rest 0; elsewhere 1 is 3, 2 is 2, 3 is
/// 4 and the rest 1), and FUN_100b1e95 turns that into `LoadPFTweakData`'s arguments: types 1..4 are the environments 1, 2,
/// 4 and 3; types 10, 11 and 12 load the Omni, Neutral and Clan training grounds' scripts (950, 954, 952) outdoors; every
/// other type the playfield's own scripts outdoors.
pub fn sky_scripts(ground: bool, kind: u8) -> SkyScripts {
    let environment_type = if ground {
        match kind {
            6..=11 => kind as u32 + 1,
            _ => 0,
        }
    } else {
        match kind {
            1 => 3,
            2 => 2,
            3 => 4,
            _ => 1,
        }
    };
    match environment_type {
        1 => SkyScripts { playfield: None, environment: 1 },
        2 => SkyScripts { playfield: None, environment: 2 },
        3 => SkyScripts { playfield: None, environment: 4 },
        4 => SkyScripts { playfield: None, environment: 3 },
        10 => SkyScripts { playfield: Some(950), environment: 0 },
        11 => SkyScripts { playfield: Some(954), environment: 0 },
        12 => SkyScripts { playfield: Some(952), environment: 0 },
        _ => SkyScripts { playfield: None, environment: 0 },
    }
}

/// [`anarchy`] of a playfield in the database.
pub fn load_anarchy(db: &ResourceDatabase, playfield: u32) -> Option<Anarchy> {
    let object = db.get(Id { kind: KIND, instance: playfield }).ok()??;
    anarchy(&object)
}

/// Every playfield, by number.
pub fn all(db: &ResourceDatabase) -> Vec<Playfield> {
    let Ok(ids) = db.ids() else { return Vec::new() };
    ids.filter_map(Result::ok)
        .filter(|id| id.kind == KIND)
        .filter_map(|id| {
            let object = db.get(Id { kind: KIND, instance: id.instance }).ok()??;
            Some(Playfield { number: id.instance, name: name(&object)? })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sky_scripts_follow_the_kind_and_the_ground() {
        // the cities (kind 1 or 2 on a ground) and the Pandemoniums (14) run their own outdoor scripts
        assert_eq!(sky_scripts(true, 1), SkyScripts { playfield: None, environment: 0 });
        assert_eq!(sky_scripts(true, 14), SkyScripts { playfield: None, environment: 0 });
        // a ground of kinds 9, 10 and 11 borrows a training ground's scripts
        assert_eq!(sky_scripts(true, 9), SkyScripts { playfield: Some(950), environment: 0 });
        assert_eq!(sky_scripts(true, 11), SkyScripts { playfield: Some(952), environment: 0 });
        assert_eq!(sky_scripts(true, 6), SkyScripts { playfield: None, environment: 0 });
        // interiors: the temples' kind 1 is the rock skyline, 2 the orbit, 3 the sand skyline, anything else indoors
        assert_eq!(sky_scripts(false, 1), SkyScripts { playfield: None, environment: 4 });
        assert_eq!(sky_scripts(false, 2), SkyScripts { playfield: None, environment: 2 });
        assert_eq!(sky_scripts(false, 3), SkyScripts { playfield: None, environment: 3 });
        assert_eq!(sky_scripts(false, 0), SkyScripts { playfield: None, environment: 1 });
        let mut env = [0u8; 26];
        env[18] = 1;
        assert_eq!(Anarchy { water: Vec::new(), environment: Some(env) }.environment_kind(), 1);
        assert_eq!(Anarchy { water: Vec::new(), environment: None }.environment_kind(), 0);
    }
}
