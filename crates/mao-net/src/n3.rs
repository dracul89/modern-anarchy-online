//! N3 messages (frame kind 10): the game world's traffic. The body is a
//! `u32` message type, the identity of the object it concerns (a `u32`
//! type, 0xc350 for characters, and a `u32` instance) and the rest.
//!
//! Only what the captures show is decoded so far.

use crate::frame::{Frame, N3};

pub const CHARACTER: u32 = 0xc350;

/// A character or creature's description (its position among other
/// things; its name is at the end).
pub const ENTITY_INFO: u32 = 0x271b3a6b;
/// The client's own position update.
pub const POSITION_UPDATE: u32 = 0x54111123;
/// Movement types of a [`POSITION_UPDATE`] (guessed from when the original
/// client sends them in the capture).
pub mod movement {
    pub const FORWARD_START: u8 = 1;
    pub const FORWARD_STOP: u8 = 2;
    pub const BACKWARD_START: u8 = 3;
    pub const BACKWARD_STOP: u8 = 4;
    pub const TURN_LEFT_START: u8 = 9;
    pub const TURN_RIGHT_START: u8 = 11;
    pub const TURN_STOP: u8 = 12;
    /// Jump (sent as the jump starts; the capture's heights rise about 2.5
    /// units and fall back in about a second).
    pub const JUMP: u8 = 22;
    /// A new heading while standing.
    pub const HEADING: u8 = 14;
    /// The periodic update while moving.
    pub const UPDATE: u8 = 15;
}

/// The server's announcement of the playfield the character is in (at login
/// and after zoning): the identity's instance is the playfield number and
/// the data holds the arrival position.
pub const PLAYFIELD: u32 = 0x5f4b1a39;

/// The first thing the client sends after entering.
pub const READY: u32 = 0x570c2039;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub kind: u32,
    pub identity: (u32, u32),
    pub data: Vec<u8>,
}

impl Message {
    pub fn parse(frame: &Frame) -> Option<Message> {
        if frame.kind != N3 || frame.body.len() < 12 {
            return None;
        }
        let long = |at: usize| u32::from_be_bytes(frame.body[at..at + 4].try_into().unwrap());
        Some(Message { kind: long(0), identity: (long(4), long(8)), data: frame.body[12..].to_vec() })
    }

    /// A frame from a character (`source`) to the server.
    pub fn frame(&self, source: u32) -> Frame {
        let mut body = self.kind.to_be_bytes().to_vec();
        body.extend_from_slice(&self.identity.0.to_be_bytes());
        body.extend_from_slice(&self.identity.1.to_be_bytes());
        body.extend_from_slice(&self.data);
        Frame { kind: N3, source, destination: 2, body }
    }

    /// The position of a [`POSITION_UPDATE`]: the movement type, the
    /// rotation (a quaternion, x y z w) and the position. The same message
    /// goes both ways (the server sends other characters' to the client).
    pub fn movement(&self) -> Option<(u8, [f32; 4], [f32; 3])> {
        if self.kind != POSITION_UPDATE || self.data.len() < 30 {
            return None;
        }
        let f = |at: usize| f32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        Some((self.data[1], [f(2), f(6), f(10), f(14)], [f(18), f(22), f(26)]))
    }

    /// The rotation of an [`ENTITY_INFO`] (a quaternion, x y z w), found
    /// where the captures put it, after the position.
    pub fn rotation(&self) -> Option<[f32; 4]> {
        if self.kind != ENTITY_INFO || self.data.len() < 38 {
            return None;
        }
        let f = |at: usize| f32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        Some([f(22), f(26), f(30), f(34)])
    }

    /// The position of an [`ENTITY_INFO`]: `x y z` (the world is Y-up).
    /// Found by where the captures put it; the fields before it
    /// (a byte, a flag byte, a word, the playfield instance) aren't
    /// understood yet.
    pub fn position(&self) -> Option<[f32; 3]> {
        if self.kind != ENTITY_INFO || self.data.len() < 22 {
            return None;
        }
        let f = |at: usize| f32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        Some([f(10), f(14), f(18)])
    }
}

/// The client's first message after entering the world.
pub fn ready(character: u32) -> Frame {
    Message { kind: READY, identity: (CHARACTER, character), data: vec![1] }.frame(character)
}

/// The client's position update: `movement` type, the heading (radians
/// about Y; 0 faces +z), the position, and the milliseconds since the last
/// update.
pub fn position_update(character: u32, movement: u8, heading: f32, position: [f32; 3], elapsed_ms: u32) -> Frame {
    let (s, c) = (heading * 0.5).sin_cos();
    let mut d = vec![1, movement];
    for v in [0.0, s, 0.0, c, position[0], position[1], position[2]] {
        d.extend_from_slice(&f32::to_be_bytes(v));
    }
    d.extend_from_slice(&elapsed_ms.to_be_bytes());
    d.extend_from_slice(&[0; 8]);
    Message { kind: POSITION_UPDATE, identity: (CHARACTER, character), data: d }.frame(character)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The client's first update in the capture.
    #[test]
    fn update_matches_the_capture() {
        let hex = "541111230000c3500009210c010c000000003f741240000000003e9a782c43f6c8ec410028f643dfae6e00001ded0000000000000000";
        let bytes: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        let m = Message { kind: POSITION_UPDATE, identity: (CHARACTER, 0x9210c), data: bytes[12..].to_vec() };
        let (kind, q, p) = m.movement().unwrap();
        assert_eq!(kind, 12);
        let heading = 2.0 * q[1].atan2(q[3]);
        let ours = position_update(0x9210c, 12, heading, p, 7661);
        assert_eq!(ours.body.len(), bytes.len());
        let mine = Message::parse(&ours).unwrap();
        let (_, q2, p2) = mine.movement().unwrap();
        assert!((q2[1] - q[1]).abs() < 1e-5 && (q2[3] - q[3]).abs() < 1e-5);
        assert_eq!(p2, p);
        assert_eq!(mine.data[30..34], 7661u32.to_be_bytes());
    }
}

/// What an [`ENTITY_INFO`] says about a character or creature, as far as
/// it is decoded. Derived from the reader of `SimpleCharFullUpdateIIR_t`
/// in `Gamecode.dll` (big-endian `BinaryStream`) and checked against a
/// capture.
///
/// After the identity: a byte 0 (header), a version byte (0x39 or 0x3a), a
/// flags `i32`, then, by the flags:
/// * bit 6: the playfield (`i32`);
/// * bit 5: an identity (not decoded here: a capture without it);
/// * always the position (three floats);
/// * bit 9: the rotation (four floats, x y z w);
/// * always a packed `i32`: breed (3 bits), two bits, three bits, sex (2
///   bits at 8), two bits, five bits from bit 12;
/// * the name: a length byte (counting the NUL) and the bytes;
/// * `i32` flags 2, an `i32` of two 16-bit values, and (without flag bit 0)
///   an `i32` (not a model id), an `i32`, then seven `i16`s; the rest (stats,
///   textures, clothing and more) is not decoded.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityInfo {
    pub version: u8,
    pub flags: u32,
    pub playfield: Option<u32>,
    pub position: [f32; 3],
    pub rotation: Option<[f32; 4]>,
    pub breed: u8,
    pub sex: u8,
    pub name: String,
    pub flags2: u32,
    /// A number after the flags that varies widely per character (365 to
    /// 20838 in the capture; probably health). NOT a model id. Absent for
    /// creatures whose flags have bit 0 set.
    pub unknown_a0: Option<u32>,
    /// The texture overrides (flag bit 4): material name and the texture's
    /// instance (type 0xf6954). Found by scanning for the vector's marker,
    /// not by decoding the fields before it.
    pub textures: Vec<(String, u32)>,
    /// The worn clothes: body part name and the texture's instance (types
    /// 0xf6954, 0xf6960, 0xf6961 share instances). Found like `textures`.
    pub clothes: Vec<(String, u32)>,
}

impl EntityInfo {
    /// Whether it is dressed from the breed and sex tables: the characters
    /// (flag bit 0 clear) of a breed 1 to 4. Sex is the game's `Sex` stat:
    /// 1 neuter, 2 male, 3 female.
    pub fn is_player(&self) -> bool {
        self.unknown_a0.is_some() && (1..=4).contains(&self.breed)
    }

    /// Whether the breed and sex tables dress it (players and the human
    /// characters among the NPCs alike).
    pub fn is_humanoid(&self) -> bool {
        matches!((self.breed, self.sex), (1..=3, 2 | 3) | (4, 1))
    }
}

/// The texture list in the rest of an entity message: an `i32` of
/// `(count + 1) * 0x3f1`, then per texture a 32-byte NUL-padded name and
/// three `i32`s (the instance, a second number, a flag). Searched for byte by
/// byte: a marker for 1 to 12 entries followed by a plausible name.
fn texture_overrides(rest: &[u8]) -> Vec<(String, u32)> {
    for start in 0..rest.len().saturating_sub(48) {
        let marker = u32::from_be_bytes(rest[start..start + 4].try_into().unwrap());
        if marker % 0x3f1 != 0 || !(2..=13).contains(&(marker / 0x3f1)) {
            continue;
        }
        let count = (marker / 0x3f1 - 1) as usize;
        let mut at = start + 4;
        let mut out = Vec::new();
        for _ in 0..count {
            let Some(entry) = rest.get(at..at + 44) else { break };
            let name = &entry[..32];
            let end = name.iter().position(|&c| c == 0).unwrap_or(32);
            let plausible = end > 0 && name[..end].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_') && name[end..].iter().all(|&c| c == 0);
            if !plausible {
                out.clear();
                break;
            }
            out.push((String::from_utf8_lossy(&name[..end]).into_owned(), u32::from_be_bytes(entry[32..36].try_into().unwrap())));
            at += 44;
        }
        if out.len() == count && count > 0 {
            return out;
        }
    }
    Vec::new()
}

/// The body parts a cloth record's slot indexes (`ClothData_t::GetName`).
const CLOTH_PARTS: [&str; 6] = ["hands", "body", "feet", "arms", "legs", "head"];

/// The cloth list in the rest of an entity message: the same counted-vector
/// marker as the textures, then a record per slot 0, 1, 2...: an `i32` id (the
/// slot; its high 16 bits above 0 mean two more `i32`s follow the record), the
/// texture's instance and a second number. Slot 0 and above with a nonzero
/// instance are kept.
fn cloth_list(rest: &[u8]) -> Vec<(String, u32)> {
    let int = |at: usize| rest.get(at..at + 4).map(|b| i32::from_be_bytes(b.try_into().unwrap()));
    for start in 0..rest.len().saturating_sub(16) {
        let Some(marker) = int(start) else { break };
        if marker <= 0 || marker % 0x3f1 != 0 || !(2..=7).contains(&(marker / 0x3f1)) {
            continue;
        }
        let count = (marker / 0x3f1 - 1) as usize;
        let mut at = start + 4;
        let mut out = Vec::new();
        let mut valid = true;
        for slot in 0..count {
            let (Some(packed), Some(instance), Some(_)) = (int(at), int(at + 4), int(at + 8)) else {
                valid = false;
                break;
            };
            if packed & 0xffff != slot as i32 {
                valid = false;
                break;
            }
            at += 12;
            if (packed >> 16) > 0 {
                at += 8;
            }
            if instance > 0 {
                if let Some(part) = CLOTH_PARTS.get(slot) {
                    out.push((part.to_string(), instance as u32));
                }
            }
        }
        if valid && !out.is_empty() {
            return out;
        }
    }
    Vec::new()
}

impl Message {
    /// The playfield announced by a [`PLAYFIELD`] message and where the
    /// character arrives in it: after the identity, a zero word, a flags
    /// byte and the position (three floats).
    pub fn arrival(&self) -> Option<(u32, [f32; 3])> {
        if self.kind != PLAYFIELD || self.data.len() < 17 {
            return None;
        }
        let f = |at: usize| f32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        Some((self.identity.1, [f(5), f(9), f(13)]))
    }

    pub fn entity_info(&self) -> Option<EntityInfo> {
        if self.kind != ENTITY_INFO {
            return None;
        }
        let d = &self.data;
        let mut at = 1usize; // the header byte
        let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
            let b = d.get(*at..*at + n)?;
            *at += n;
            Some(b)
        };
        let version = take(&mut at, 1)?[0];
        if !matches!(version, 0x39 | 0x3a) {
            return None;
        }
        let u32_at = |at: &mut usize| take(at, 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
        let f32_at = |at: &mut usize| take(at, 4).map(|b| f32::from_be_bytes(b.try_into().unwrap()));
        let flags = u32_at(&mut at)?;
        let playfield = if flags & 0x40 != 0 { Some(u32_at(&mut at)?) } else { None };
        if flags & 0x20 != 0 {
            return None;
        }
        let position = [f32_at(&mut at)?, f32_at(&mut at)?, f32_at(&mut at)?];
        let rotation = if flags & 0x200 != 0 { Some([f32_at(&mut at)?, f32_at(&mut at)?, f32_at(&mut at)?, f32_at(&mut at)?]) } else { None };
        let packed = u32_at(&mut at)?;
        let (breed, sex) = ((packed & 7) as u8, ((packed >> 8) & 3) as u8);
        let length = take(&mut at, 1)?[0] as usize;
        let text = take(&mut at, length)?;
        let name = String::from_utf8_lossy(text).trim_end_matches('\0').to_string();
        let flags2 = u32_at(&mut at)?;
        u32_at(&mut at)?;
        let model = if flags & 1 == 0 { u32_at(&mut at) } else { None };
        let tail = &d[at.min(d.len())..];
        let textures = if flags & 0x10 != 0 { texture_overrides(tail) } else { Vec::new() };
        let clothes = cloth_list(tail);
        Some(EntityInfo { version, flags, playfield, position, rotation, breed, sex, name, flags2, unknown_a0: model, textures, clothes })
    }
}

#[cfg(test)]
mod entity_tests {
    use super::*;

    fn bytes(hex: &str) -> Vec<u8> {
        let hex: String = hex.split_whitespace().collect();
        (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    /// The start of a monster's entity message from a capture.
    #[test]
    fn monster_from_capture() {
        let data = bytes(
            "00 3a 04 20 6a ca 00 00 02 1c 43 f5 f0 c7 41 00 28 f6 43 dc 31 c6 00 00 00 00 3f 38 be c4 00 00 00 00 3f 31 37 16 \
             00 00 07 41 09 4d 6f 70 68 69 6e 67 34 00 00 18 12 41 00 01 00 01 00 00 0f 7e 00 00 00 00",
        );
        let info = Message { kind: ENTITY_INFO, identity: (CHARACTER, 1933326420), data }.entity_info().unwrap();
        assert_eq!(info.name, "Mophing4");
        assert_eq!(info.playfield, Some(0x21c));
        assert!((info.position[0] - 491.9).abs() < 0.1 && (info.position[1] - 8.0).abs() < 0.1);
        assert_eq!((info.breed, info.sex), (1, 3));
        assert!(info.is_player());
        assert_eq!(info.unknown_a0, Some(0xf7e));
    }

    /// The character from the capture (a player: sex 2).
    #[test]
    fn player_from_capture() {
        let data = bytes(
            "00 3a 00 00 4a c8 00 00 02 1c 43 f6 c8 ec 41 00 28 f6 43 df ae 6e 00 00 00 00 3f 74 12 40 00 00 00 00 3e 9a 78 2f \
             00 00 06 29 0d 42 72 61 73 73 63 61 73 69 6e 67 73 00 00 18 12 41 00 01 00 01 00 00 01 6d 00 00 00 00",
        );
        let info = Message { kind: ENTITY_INFO, identity: (CHARACTER, 598284), data }.entity_info().unwrap();
        assert_eq!(info.name, "Brasscasings");
        assert_eq!((info.breed, info.sex), (1, 2));
        assert!(info.is_player());
        assert_eq!(info.unknown_a0, Some(365));
        assert!(info.textures.is_empty() || info.textures.iter().all(|t| !t.0.is_empty()));
    }
}

#[cfg(test)]
mod zoning_tests {
    use super::*;

    /// The announcement of West Athens from a capture.
    #[test]
    fn arrival_from_capture() {
        let data: Vec<u8> = [0u8, 0, 0, 0, 4, 0x44, 0x2f, 0xdf, 0xf2, 0x41, 0x20, 0x1d, 0x07, 0x43, 0xd4, 0x61, 0x42, 0x61, 0, 0, 0xc7, 0x9c, 0, 0, 2, 0x21].to_vec();
        let m = Message { kind: PLAYFIELD, identity: (0x9c50, 0x221), data };
        let (playfield, at) = m.arrival().unwrap();
        assert_eq!(playfield, 545);
        assert!((at[0] - 704.0).abs() < 0.1 && (at[1] - 10.0).abs() < 0.1 && (at[2] - 424.76).abs() < 0.1);
    }
}
