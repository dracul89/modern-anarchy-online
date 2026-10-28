//! Character animations: the resource database's `RDBCATAnim_t` objects
//! (type 0xf6953).
//!
//! Derived from the original's `RDBCATAnim_t::ReadBlob`
//! (`AnarchyReimagined`: `dropin/displaysystem/src/rdbanim.rs`) and randy31's
//! `CATKeyframeAnimData_t` reader and sampling (`dropin/randy31/src/anim.rs`).
//! Everything is little-endian.
//!
//! # The object
//! The 12-byte head, a name (32 bytes; its tail from byte 16 is the second,
//! shorter name), a note count and the notes (each a time `i32` and a
//! 32-byte name: events such as footsteps), then the keyframe data:
//!
//! * `i32` 3, an `i32` version above 0x104 (bit 0x1000000: compressed), the
//!   first time (an `i32` before 0x106, else a float), a word, the duration
//!   (float) and the track count;
//! * plain: per track its index, an unused word, its layers and a key count;
//!   per key a time (an `i32` before 0x106) and a rotation (a quaternion,
//!   x y z w) and a translation (three floats);
//! * compressed: two bytes (the quaternion's and the translation's bit
//!   widths), then five value streams (see [`Values`]) holding the indices
//!   and counts, the times, the quaternions, the translations' ranges and
//!   the translations, read in track order.
//!
//! There is a track for each bone of the mesh, by index.

use std::fmt;
use std::io::Read;

use crate::texture::OBJECT_HEAD;

#[derive(Debug)]
pub enum Error {
    Truncated(&'static str),
    NotAnimation,
    Version(i32),
    Zlib(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated(what) => write!(f, "animation truncated in {what}"),
            Error::NotAnimation => write!(f, "not a CAT animation"),
            Error::Version(v) => write!(f, "CAT animation version {v:#x} not supported"),
            Error::Zlib(e) => write!(f, "a packed animation stream won't unpack: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub struct Note {
    pub time: f32,
    pub name: String,
}

impl Note {
    /// The note's event (`RDBCATAnim_t::ReadBlob`'s note id, displaysystem `rdbanim.rs`): the number of its name's prefix
    /// ([`note_number`]), or for none a name of more than nine characters starting `SM_Sandy_` its sound ID.
    pub fn event(&self) -> i32 {
        let id = note_number(self.name.as_bytes());
        if id == 0 && self.name.len() > 9 && self.name.starts_with("SM_Sandy_") {
            return crate::sound::sound_id(&self.name) as i32;
        }
        id
    }
}

/// FUN_10075c84: the number of an animation note's name, by its prefix: the first of these it starts with (`step` is checked
/// before `stepfast`, which starts with it, so a fast step is a step), 0 for none or a loop marker.
pub fn note_number(name: &[u8]) -> i32 {
    if name.starts_with(b"loopstart") || name.starts_with(b"loopend") {
        return 0;
    }
    const CHAIN: &[(&[u8], i32)] = &[
        (b"right", 0x26),
        (b"left", 0x26),
        (b"land", 0x85),
        (b"step", 0x26),
        (b"wingflap", 0x25),
        (b"wingflapfast", 0x37),
        (b"swim", 0x38),
        (b"swimfast", 0x39),
        (b"othermove", 0x3a),
        (b"othermovefast", 0x3b),
        (b"idle", 0xf),
        (b"attack_start_1", 0x77),
        (b"attack_start_2", 0x78),
        (b"attack_start_3", 0x79),
        (b"attack_start_4", 0x7a),
        (b"attack_start_5", 0x7b),
        (b"attack_start_6", 0x7c),
        (b"attack_start_7", 0x7d),
        (b"attack_start_8", 0x7e),
        (b"attack_start_9", 0x7f),
        (b"attack_effect_1", 0x8e),
        (b"attack_effect_2", 0x8f),
        (b"attack_effect_3", 0x90),
        (b"attack_effect_4", 0x91),
        (b"attack", 0xb),
        (b"get", 1),
        (b"drop", 2),
        (b"use", 3),
        (b"repair", 4),
        (b"useitemonitem", 5),
        (b"wear", 6),
        (b"unwear", 7),
        (b"wield", 8),
        (b"unwield", 9),
        (b"die", 0x1e),
        (b"impact", 0x1f),
        (b"doubleattack", 0xe),
        (b"aimedshot", 0x15),
        (b"burst", 0x16),
        (b"fullauto", 0x17),
        (b"leftattack", 0x18),
        (b"quickattack", 0x19),
        (b"flingshot", 0x1c),
        (b"sneakattack", 0x1d),
        (b"dimach", 0x24),
        (b"brawl", 0x23),
        (b"blurstart", 0x2d),
        (b"blurend", 0x35),
        (b"itemhide_l", 0x3c),
        (b"itemhide_r", 0x3d),
        (b"itemhide_b", 0x3e),
        (b"itemshow_l", 0x3f),
        (b"itemshow_r", 0x40),
        (b"itemshow_b", 0x41),
        (b"effect1start", 0x42),
        (b"effect1stop", 0x43),
        (b"effect2start", 0x44),
        (b"effect2stop", 0x45),
        (b"swish_punch", 0x73),
        (b"swish_kick", 0x74),
        (b"swish_whip", 0x75),
        (b"swish_huge", 0x76),
        (b"enter_combat", 0x80),
        (b"exit_combat", 0x81),
        (b"idle_combat", 0x82),
    ];
    CHAIN.iter().find(|(prefix, _)| name.starts_with(prefix)).map_or(0, |&(_, n)| n)
}

#[derive(Clone, Debug, Default)]
pub struct Track {
    /// (time, quaternion x y z w)
    pub rotations: Vec<(f32, [f32; 4])>,
    /// (time, translation)
    pub translations: Vec<(f32, [f32; 3])>,
}

#[derive(Clone, Debug, Default)]
pub struct CatAnim {
    pub name: String,
    pub notes: Vec<Note>,
    pub first: f32,
    pub duration: f32,
    pub tracks: Vec<Track>,
    /// The layers each track belongs to (a mask).
    pub layers: Vec<u32>,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len()).ok_or(Error::Truncated(what))?;
        let b = &self.data[self.at..end];
        self.at = end;
        Ok(b)
    }

    fn i32(&mut self, what: &'static str) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }

    fn f32(&mut self, what: &'static str) -> Result<f32, Error> {
        Ok(f32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }

    fn u8(&mut self, what: &'static str) -> Result<u8, Error> {
        Ok(self.take(1, what)?[0])
    }

    fn text(&mut self, n: usize, what: &'static str) -> Result<String, Error> {
        let b = self.take(n, what)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(n);
        Ok(String::from_utf8_lossy(&b[..end]).into_owned())
    }
}

/// A compressed value stream: `channels` interleaved channels (each stored
/// as its zlib size, its full size and the zlib data), each value the
/// running sum of big-endian deltas of `bits` bits (rounded up to bytes).
struct Values {
    mask: u32,
    width: usize,
    channel: usize,
    position: usize,
    channels: Vec<(Vec<u8>, u32)>,
}

impl Values {
    fn new(bits: u32, r: &mut Reader, count: usize) -> Result<Values, Error> {
        let mut channels = Vec::new();
        for _ in 0..count {
            let packed = r.i32("a stream's size")? as u32 as usize;
            let size = r.i32("a stream's size")? as u32 as usize;
            let bytes = r.take(packed, "a stream")?;
            let mut out = Vec::with_capacity(size);
            if packed != 0 {
                flate2::read::ZlibDecoder::new(bytes).read_to_end(&mut out).map_err(Error::Zlib)?;
            }
            channels.push((out, 0u32));
        }
        Ok(Values { mask: if bits >= 32 { u32::MAX } else { (1u32 << (bits & 31)).wrapping_sub(1) }, width: (bits as usize).div_ceil(8), channel: 0, position: 0, channels })
    }

    fn next(&mut self) -> Result<u32, Error> {
        let (bytes, value) = &mut self.channels[self.channel];
        let b = bytes.get(self.position..self.position + self.width).ok_or(Error::Truncated("a value stream"))?;
        let delta = b.iter().fold(0u32, |a, &c| a << 8 | c as u32);
        *value = value.wrapping_add(delta);
        let v = *value;
        self.channel += 1;
        if self.channel == self.channels.len() {
            self.channel = 0;
            self.position += self.width;
        }
        Ok(v & self.mask)
    }
}

/// Reads an object (with its 12-byte head).
pub fn parse(object: &[u8]) -> Result<CatAnim, Error> {
    let mut r = Reader { data: object, at: OBJECT_HEAD };
    let name = r.text(0x20, "the name")?;
    let mut anim = CatAnim { name, ..Default::default() };
    let notes = r.i32("the note count")?;
    for _ in 0..notes.max(0) {
        let time = r.i32("a note")? as f32;
        let mut name = r.take(0x20, "a note")?.to_vec();
        // Characters below a space end the name (compared signed, so bytes from 0x80 too).
        if let Some(i) = name.iter().position(|&c| (c as i8) < b' ' as i8) {
            name.truncate(i);
        }
        anim.notes.push(Note { time, name: String::from_utf8_lossy(&name).into_owned() });
    }
    if r.i32("the marker")? != 3 {
        return Err(Error::NotAnimation);
    }
    let mut version = r.i32("the version")?;
    let compressed = version & 0x0100_0000 != 0;
    version &= !0x0100_0000;
    if version <= 0x104 {
        return Err(Error::Version(version));
    }
    anim.first = if version < 0x106 { r.i32("the first time")? as f32 } else { r.f32("the first time")? };
    r.i32("a word")?;
    anim.duration = r.f32("the duration")?;
    let tracks = r.i32("the track count")?;
    if !(0..=0x10000).contains(&tracks) {
        return Err(Error::Truncated("the track count"));
    }
    anim.tracks = vec![Track::default(); tracks as usize];
    anim.layers = vec![0; tracks as usize];
    let slot = |index: i32| -> Result<usize, Error> { usize::try_from(index).ok().filter(|&i| i < tracks as usize).ok_or(Error::Truncated("a track index")) };
    if !compressed {
        for _ in 0..tracks {
            let index = slot(r.i32("a track index")?)?;
            r.i32("a word")?;
            anim.layers[index] = r.i32("the layers")? as u32;
            let keys = r.i32("the key count")?;
            for _ in 0..keys.max(0) {
                let time = if version < 0x106 { r.i32("a key time")? as f32 } else { r.f32("a key time")? };
                let q = [r.f32("a rotation")?, r.f32("a rotation")?, r.f32("a rotation")?, r.f32("a rotation")?];
                let t = [r.f32("a translation")?, r.f32("a translation")?, r.f32("a translation")?];
                anim.tracks[index].rotations.push((time, q));
                anim.tracks[index].translations.push((time, t));
            }
        }
    } else {
        let quat_bits = r.u8("the quaternion width")? as u32;
        let move_bits = r.u8("the translation width")? as u32;
        let mut indices = Values::new(32, &mut r, 1)?;
        let mut times = Values::new(if version >= 0x106 { 32 } else { 20 }, &mut r, 1)?;
        let mut quats = Values::new(quat_bits, &mut r, 4)?;
        let mut ranges = Values::new(32, &mut r, 1)?;
        let mut moves = Values::new(move_bits, &mut r, 3)?;
        let time = |times: &mut Values| -> Result<f32, Error> {
            let v = times.next()?;
            Ok(if version < 0x106 { v as f32 } else { f32::from_bits(v) })
        };
        for _ in 0..tracks {
            let index = slot(indices.next()? as i32)?;
            indices.next()?;
            anim.layers[index] = indices.next()?;
            let rotations = indices.next()?;
            let half = (1i64 << (quat_bits.wrapping_sub(1) & 31)) as f64;
            for _ in 0..rotations {
                let t = time(&mut times)?;
                let mut c = [0f32; 4];
                for v in &mut c {
                    *v = (quats.next()? as f64 - half) as f32;
                }
                let length = (c.iter().map(|&v| v as f64 * v as f64).sum::<f64>()).sqrt();
                let q = c.map(|v| (v as f64 / length) as f32);
                anim.tracks[index].rotations.push((t, q));
            }
            let range: Vec<f32> = (0..6).map(|_| ranges.next().map(f32::from_bits)).collect::<Result<_, _>>()?;
            let translations = indices.next()?;
            let scale = ((1u64 << (move_bits & 31)) - 1) as f64;
            for _ in 0..translations {
                let t = time(&mut times)?;
                let mut v = [0f32; 3];
                for (c, out) in v.iter_mut().enumerate() {
                    let f = moves.next()? as f64 / scale;
                    let (lo, hi) = (range[c * 2], range[c * 2 + 1]);
                    *out = ((hi - lo) as f64 * f + lo as f64) as f32;
                }
                anim.tracks[index].translations.push((t, v));
            }
        }
    }
    Ok(anim)
}

/// The keys around `time` and how far it is between them: past the last
/// key the first is used.
fn around(times: impl Fn(usize) -> f32, count: usize, time: f32) -> (usize, usize, f32) {
    let (mut lo, mut hi) = (0usize, count.saturating_sub(1));
    let mut f = 0.0;
    if hi != 0 && times(hi) >= time {
        while hi - lo > 1 {
            let mid = (hi + lo) / 2;
            if time >= times(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        f = (time - times(lo)) / (times(hi) - times(lo));
    }
    (lo, hi, f)
}

fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut d = a.iter().zip(&b).map(|(x, y)| x * y).sum::<f32>();
    let flip = d < 0.0;
    if flip {
        d = -d;
    }
    let (s0, mut s1);
    if 1.0 - d < 1e-6 {
        (s0, s1) = (1.0 - t, t);
    } else {
        let theta = d.acos();
        let inverse = 1.0 / theta.sin();
        s0 = (theta - t * theta).sin() * inverse;
        s1 = (t * theta).sin() * inverse;
    }
    if flip {
        s1 = -s1;
    }
    [0, 1, 2, 3].map(|i| a[i] * s0 + b[i] * s1)
}

impl CatAnim {
    /// The track's rotation at `time` (the identity for a track without keys).
    pub fn rotation(&self, track: usize, time: f32) -> [f32; 4] {
        let Some(t) = self.tracks.get(track).filter(|t| !t.rotations.is_empty()) else { return [0.0, 0.0, 0.0, 1.0] };
        let (lo, hi, f) = around(|k| t.rotations[k].0, t.rotations.len(), time);
        if f < 0.01 { t.rotations[lo].1 } else { slerp(t.rotations[lo].1, t.rotations[hi].1, f) }
    }

    /// The track's translation at `time`.
    pub fn translation(&self, track: usize, time: f32) -> [f32; 3] {
        let Some(t) = self.tracks.get(track).filter(|t| !t.translations.is_empty()) else { return [0.0; 3] };
        let (lo, hi, f) = around(|k| t.translations[k].0, t.translations.len(), time);
        let (a, b) = (t.translations[lo].1, t.translations[hi].1);
        if f < 0.01 { a } else { [0, 1, 2].map(|i| a[i] * (1.0 - f) + b[i] * f) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_events() {
        let note = |name: &str| Note { time: 0.0, name: name.to_string() }.event();
        assert_eq!([note("left_foot"), note("stepfast"), note("land"), note("swimfast"), note("attack_effect_2"), note("attack")], [0x26, 0x26, 0x85, 0x38, 0x8f, 0xb]);
        assert_eq!([note("loopstart"), note("SM_Sandy_"), note("nothing")], [0, 0, 0]);
        assert_eq!(note("SM_Sandy_Swish_punch"), crate::sound::sound_id("SM_Sandy_Swish_punch") as i32);
    }

    fn anim_with_keys() -> CatAnim {
        CatAnim {
            tracks: vec![Track { rotations: vec![(0.0, [0.0, 0.0, 0.0, 1.0]), (10.0, [0.0, 0.0, 0.0, 1.0])], translations: vec![(0.0, [0.0, 0.0, 0.0]), (10.0, [10.0, 0.0, 0.0])] }],
            ..Default::default()
        }
    }

    #[test]
    fn translation_interpolates() {
        let a = anim_with_keys();
        assert_eq!(a.translation(0, 0.0), [0.0; 3]);
        assert!((a.translation(0, 5.0)[0] - 5.0).abs() < 1e-5);
        assert_eq!(a.translation(3, 5.0), [0.0; 3]);
    }

    #[test]
    fn rotation_slerps_the_short_way() {
        let (s, c) = (std::f32::consts::FRAC_PI_4.sin(), std::f32::consts::FRAC_PI_4.cos());
        let mid = slerp([0.0, 0.0, 0.0, 1.0], [0.0, s, 0.0, c], 0.5);
        let half = std::f32::consts::FRAC_PI_8;
        assert!((mid[1] - half.sin()).abs() < 1e-5 && (mid[3] - half.cos()).abs() < 1e-5);
    }
}
