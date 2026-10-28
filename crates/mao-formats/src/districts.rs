//! A playfield's districts: the `0xf424e` object (one per playfield), GameData's `PlayfieldDistrictInfo_t` (the `gamedata`
//! drop-in's `dbobject.rs` `zone_info_read`, `district.rs` `district_read`). After the object head: a version (7, 5 or 6), the
//! zone count, the district count (a byte), the districts, then a byte per zone naming its district.
//!
//! A district (`DistrictData_t`, version 6 and up) is its centre, its name (a 16-bit length and the bytes), nine music IDs (16
//! bits each: the first the district's sound, Gamecode's `ActivateEnvZone` district; then the music layers by the time of
//! day and the weather, FUN_100b72bf), the NPC and land control level ranges, respawn and fight settings and its spawn points.
//! The spawn points (hash spawn points carry spell lists) are not read: each district's head is found after the last by its
//! shape (a centre of three finite non-negative floats, a name of 1 to 64 printable characters not used before,
//! music IDs of layers or none, level ranges to 1024), and the count checked.

use crate::texture::OBJECT_HEAD;

/// The type of a playfield's districts.
pub const KIND: u32 = 0xf424e;

#[derive(Clone, Debug, PartialEq)]
pub struct District {
    pub name: String,
    pub centre: [f32; 3],
    pub music: [u16; 9],
    pub control: (u16, u16),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Districts {
    pub districts: Vec<District>,
    /// Each zone's district.
    pub zones: Vec<u8>,
}

/// A district's head at `at`, and where it ends: version 6 and up a 16-bit length and the name, 16-bit level ranges and
/// the land control range; before that a fixed 32-byte name and four bytes then byte ranges (no land control).
fn head(object: &[u8], at: usize, old: bool) -> Option<(District, usize)> {
    let f = |o: usize| Some(f32::from_le_bytes(object.get(o..o + 4)?.try_into().ok()?));
    let h = |o: usize| Some(u16::from_le_bytes(object.get(o..o + 2)?.try_into().ok()?));
    let centre = [f(at)?, f(at + 4)?, f(at + 8)?];
    if !centre.iter().all(|c| c.is_finite() && *c >= 0.0 && *c < 1.0e6) {
        return None;
    }
    let (name, mut at) = if old {
        let field = object.get(at + 12..at + 44)?;
        let end = field.iter().position(|&b| b == 0).unwrap_or(32);
        (&field[..end], at + 44)
    } else {
        let len = h(at + 12)? as usize;
        (object.get(at + 14..at + 14 + len)?, at + 14 + len)
    };
    if !(1..=0x40).contains(&name.len()) || !name.iter().all(|&b| (0x20..0x7f).contains(&b)) {
        return None;
    }
    let mut music = [0u16; 9];
    for m in &mut music {
        *m = h(at)?;
        at += 2;
    }
    // (the ranges `district_read` checks: up to 1024; the music IDs are layers or none, 0xffff)
    if old {
        let (min, max) = (*object.get(at + 4)? as u16, *object.get(at + 5)? as u16);
        if music.iter().any(|&m| m != 0xffff && m >= 1000) || min > max {
            return None;
        }
        return Some((District { name: name.iter().map(|&b| b as char).collect(), centre, music, control: (0, 0) }, at + 6));
    }
    let (npc, control) = ((h(at)?, h(at + 2)?), (h(at + 4)?, h(at + 6)?));
    if npc.0 > 1024 || npc.1 > 1024 || control.0 > 1024 || control.1 > 1024 || music.iter().any(|&m| m != 0xffff && m >= 1000) {
        return None;
    }
    Some((District { name: name.iter().map(|&b| b as char).collect(), centre, music, control }, at + 8))
}

pub fn parse(object: &[u8]) -> Option<Districts> {
    let version = u16::from_le_bytes(object.get(OBJECT_HEAD..OBJECT_HEAD + 2)?.try_into().ok()?);
    if !matches!(version, 5..=7) {
        return None;
    }
    let old = version < 6;
    let zones = u32::from_le_bytes(object.get(OBJECT_HEAD + 2..OBJECT_HEAD + 6)?.try_into().ok()?) as usize;
    let count = *object.get(OBJECT_HEAD + 6)? as usize;
    if zones == 0 || count == 0 || zones > object.len() {
        return None;
    }
    let table = &object[object.len() - zones..];
    if table.iter().any(|&d| d as usize >= count) {
        return None;
    }
    let end = object.len() - zones;
    let mut districts = Vec::new();
    let mut at = OBJECT_HEAD + 7;
    while districts.len() < count {
        // (names are unique: `VerifyDistrictNames` refuses a playfield with two of a name)
        let (d, next) = (at..end).find_map(|p| head(object, p, old).filter(|(d, _)| !districts.iter().any(|x: &District| x.name == d.name)))?;
        districts.push(d);
        at = next;
    }
    Some(Districts { districts, zones: table.to_vec() })
}

impl Districts {
    /// The district of a zone (`PlayfieldDistrictInfo_t::GetDistrictData`; none past the table).
    pub fn of_zone(&self, zone: u32) -> Option<&District> {
        self.districts.get(*self.zones.get(zone as usize)? as usize)
    }
}

/// The zone at a position of an outdoor playfield (N3's `n3Playfield_t::GetZoneInstance`: none outside the map; else the
/// zone column plus the row times the zones across, a zone `zone_size` tiles a side).
pub fn zone_at(position: [f32; 3], tile_size: f32, width_tiles: u32, height_tiles: u32, zone_size: u32) -> u32 {
    let (x, z) = (position[0], position[2]);
    if x < 0.0 || x >= width_tiles as f32 * tile_size || z < 0.0 || z >= height_tiles as f32 * tile_size || zone_size == 0 || tile_size <= 0.0 {
        return 0;
    }
    let across = width_tiles / zone_size;
    let (zx, zz) = ((x / tile_size) as u32 / zone_size, (z / tile_size) as u32 / zone_size);
    zz * across + zx
}

/// The zone size (in tiles) of an outdoor playfield's definition object (0xf4241: `RDBPlayfield_t` +0x18, the word after
/// the tile map's number).
pub fn zone_size(playfield: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(playfield.get(OBJECT_HEAD + 44..OBJECT_HEAD + 48)?.try_into().ok()?))
}
