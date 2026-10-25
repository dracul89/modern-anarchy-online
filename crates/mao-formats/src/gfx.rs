//! The effect templates (`Setupf/gfxtweak.bin`): what the original draws when a nano program is cast, lands, runs on a character or
//! flies as a tracer. The ids a nano template names (`CastEffectType` 428, `HitEffectType` 361, `EffectType` 413, `TracerEffectType`
//! 419, `GatherEffectType` 366, `ImpactEffectType` 414) and the `GfxEffect` spell's argument are keys here.
//!
//! The file is little-endian 32-bit words: the number of records, then each record as `id, kind, n` and `n` payload words. Every record
//! of the real file is read this way (2687 of 2687, to the last word). The kind is the class of the template: `_EffectHandler_t::
//! CreateGfxControl` (Gamecode.dll) makes a control of a class for the kinds 1005, 1013, 1018, 1019, 1021, 1022, 1024-1027, 2002, 2004,
//! 2005, 2007, 2010, 2013, 3011, 3015, 3017, 3019, 3020, 3026, 3028 and 4000; the other kinds (1001-1004, 1006-1012...) are parts that
//! those refer to. A payload word is an integer, a float (as its bit pattern) or the id of another record: a template is a tree.
//!
//! What is known of the payloads (from the records themselves; the field meanings are partly guessed from their values):
//! * the first payload word is a flag set (5, 7, 513, 771, 1799...), words 1 to 6 are zero or an offset and a rotation (a position of
//!   `0, -0.25, 0` and an angle of 1.57 are seen), word 7 often a reference (a child template or a sprite);
//! * a particle emitter (kinds 1003, 1005-1008, 1010) carries a count, start and end sizes, a start and an end colour as alpha, red,
//!   green, blue (`1 0.5 0.4 1  0 0.5 0.4 1`: fading out), and angles `0 6.28 -1.57 1.57` (a full turn around, a hemisphere up);
//! * kind 1001, the persistent effect of a running buff (`EffectType` 413 of 486 nanos), is a colour (`0.2 0.2 0.99` blue,
//!   `0.99 0.2 0.2` red...) and two references to kind 1002 helix emitters (a radius path of kind 0 with 21 pairs, a spin of 3.14 and
//!   -12.6 radians per second, a sparkle emitter of kind 1005 and a trail of kind 1003);
//! * kind 1010 is the cast effect (`CastEffectType` of 1320 nanos): a few colours and the references of its six parts.

use std::collections::HashMap;

/// One template: its id, its class and its payload.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub id: i32,
    pub kind: i32,
    pub payload: Vec<i32>,
}

impl Record {
    /// A payload word read as a float.
    pub fn float(&self, at: usize) -> f32 {
        self.payload.get(at).map_or(0.0, |&w| f32::from_bits(w as u32))
    }

    pub fn int(&self, at: usize) -> i32 {
        self.payload.get(at).copied().unwrap_or(0)
    }
}

/// All the templates, by id.
#[derive(Debug, Default)]
pub struct Gfx {
    pub records: Vec<Record>,
    by_id: HashMap<i32, usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Malformed {
    pub at_word: usize,
}

impl std::fmt::Display for Malformed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the effect templates end badly at word {}", self.at_word)
    }
}

impl Gfx {
    pub fn parse(data: &[u8]) -> Result<Gfx, Malformed> {
        let words: Vec<i32> = data.chunks_exact(4).map(|c| i32::from_le_bytes(c.try_into().unwrap())).collect();
        let count = *words.first().ok_or(Malformed { at_word: 0 })?;
        let mut gfx = Gfx::default();
        let mut at = 1;
        for _ in 0..count.max(0) {
            if at + 3 > words.len() {
                return Err(Malformed { at_word: at });
            }
            let (id, kind, n) = (words[at], words[at + 1], words[at + 2]);
            if n < 0 || at + 3 + n as usize > words.len() {
                return Err(Malformed { at_word: at });
            }
            let payload = words[at + 3..at + 3 + n as usize].to_vec();
            // (the first record of an id wins: a few ids repeat)
            gfx.by_id.entry(id).or_insert(gfx.records.len());
            gfx.records.push(Record { id, kind, payload });
            at += 3 + n as usize;
        }
        Ok(gfx)
    }

    pub fn get(&self, id: i32) -> Option<&Record> {
        self.by_id.get(&id).map(|&i| &self.records[i])
    }

    /// The records a template refers to: the payload words that are ids of other templates (not the template itself). Small numbers
    /// that happen to be ids are skipped (a flag of 5 or a count of 8 is not a reference), so only ids of 100 and up count.
    pub fn references(&self, id: i32) -> Vec<i32> {
        let Some(r) = self.get(id) else { return Vec::new() };
        r.payload.iter().copied().filter(|&w| w >= 100 && w != id && self.by_id.contains_key(&w)).collect()
    }

    /// The template and every template below it, each once, in the order they are first reached.
    pub fn tree(&self, id: i32) -> Vec<&Record> {
        let mut seen = Vec::new();
        let mut stack = vec![id];
        while let Some(next) = stack.pop() {
            if seen.contains(&next) {
                continue;
            }
            seen.push(next);
            stack.extend(self.references(next).into_iter().rev());
        }
        seen.into_iter().filter_map(|i| self.get(i)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(words: &[i32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn records_are_read_to_the_last_word_and_trees_follow_references() {
        // two records: a parent (kind 1001) naming its child (2004) and a float
        let data = bytes(&[2, 1000, 1001, 3, 5, 2004_0, 0x3f80_0000, 20040, 2004, 2, 5, -1]);
        let gfx = Gfx::parse(&data).unwrap();
        assert_eq!(gfx.records.len(), 2);
        assert_eq!(gfx.get(1000).unwrap().kind, 1001);
        assert_eq!(gfx.get(1000).unwrap().float(2), 1.0);
        assert_eq!(gfx.references(1000), vec![20040]);
        assert_eq!(gfx.tree(1000).iter().map(|r| r.id).collect::<Vec<_>>(), vec![1000, 20040]);
        // a count that outruns the file is an error, not a panic
        assert!(Gfx::parse(&bytes(&[5, 1, 2, 9, 0])).is_err());
    }
}
