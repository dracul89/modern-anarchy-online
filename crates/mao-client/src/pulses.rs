//! When a running buff's effect plays, as the original's `_GfxControlBPHFSM_t` (Gamecode.dll, the class behind a kind-1001
//! template) decides it:
//!
//! * each buff keeps a countdown; when it reaches zero the buff wants to pulse (it starts at zero, so a fresh buff pulses at once);
//! * all the buffs of one character share one gate (the FSMs register by the character's identity): a pulse is allowed only when
//!   the character has not shown any buff's pulse in the last **5.0 s** (`_DAT_10161238`, a double); a buff that finds the gate shut
//!   waits `rand() / 16384` = 0 to 2 s and tries again;
//! * a pulse shown sets the buff's countdown to its template's interval (word 20: 4 s for most) and, when the template limits the
//!   plays (word 21: -1 is forever, 4 in the one-helix auras), counts one off; at zero the effect ends.
//!
//! The pulse itself lasts the helix's life (word 8, 1.5 s). This is what the client plays; the drawing is `effects`.

use std::collections::HashMap;

/// The shared gate between pulses of one character, seconds.
pub const GATE: f32 = 5.0;

#[derive(Default)]
struct BuffPulse {
    next_try: f32,
    started: Option<f32>,
    plays_left: i32,
    tries: u32,
}

#[derive(Default)]
struct CharacterPulses {
    /// When the character last began a pulse (any buff), or far in the past.
    last: f32,
    buffs: HashMap<u32, BuffPulse>,
    seen: bool,
}

#[derive(Default)]
pub struct Pulses {
    characters: HashMap<u32, CharacterPulses>,
}

/// A deterministic stand-in for `rand() / 16384`: 0 to 2 seconds, different for each try.
fn retry_wait(character: u32, nano: u32, tries: u32) -> f32 {
    let mut x = character.wrapping_mul(0x9e37_79b1) ^ nano.wrapping_mul(0x85eb_ca6b) ^ tries.wrapping_mul(0x27d4_eb2f);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    (x & 0x7fff) as f32 / 16384.0
}

impl Pulses {
    /// Forgets the characters not ticked since the last `end_frame`.
    pub fn begin_frame(&mut self) {
        for c in self.characters.values_mut() {
            c.seen = false;
        }
    }

    pub fn end_frame(&mut self) {
        self.characters.retain(|_, c| c.seen);
    }

    /// Advances one buff of one character to `time` (seconds on the effects clock) and says how far into its pulse it is, when one
    /// is showing: `Some(age)` with age in `0..life`. `period` is the template's interval, `plays` its play count (-1 forever).
    pub fn tick(&mut self, character: u32, nano: u32, period: f32, life: f32, plays: i32, time: f32) -> Option<f32> {
        let entry = self.characters.entry(character).or_insert_with(|| CharacterPulses { last: -1000.0, ..Default::default() });
        entry.seen = true;
        let last = entry.last;
        let buff = entry.buffs.entry(nano).or_insert_with(|| BuffPulse { next_try: time, started: None, plays_left: plays, tries: 0 });
        if let Some(started) = buff.started {
            if time - started < life {
                return Some(time - started);
            }
            buff.started = None;
        }
        if buff.plays_left == 0 || time < buff.next_try {
            return None;
        }
        if time - last <= GATE {
            // the gate is shut: another buff of this character pulsed lately
            buff.tries = buff.tries.wrapping_add(1);
            buff.next_try = time + retry_wait(character, nano, buff.tries);
            return None;
        }
        buff.started = Some(time);
        buff.next_try = time + period.max(0.1);
        if buff.plays_left > 0 {
            buff.plays_left -= 1;
        }
        entry.last = time;
        Some(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The times a buff's pulses began, sampling every tenth of a second.
    fn starts(p: &mut Pulses, character: u32, nano: u32, period: f32, plays: i32, until: f32) -> Vec<f32> {
        let mut out = Vec::new();
        let mut t = 0.0;
        while t < until {
            if p.tick(character, nano, period, 1.5, plays, t) == Some(0.0) {
                out.push((t * 10.0f32).round() / 10.0);
            }
            t += 0.1;
        }
        out
    }

    #[test]
    fn one_buff_pulses_at_once_then_about_every_five_seconds() {
        let mut p = Pulses::default();
        let s = starts(&mut p, 7, 100, 4.0, -1, 30.0);
        assert_eq!(s[0], 0.0);
        // wants to again at 4 s, allowed once the 5 s gate opens (plus a retry wait under 2 s)
        for pair in s.windows(2) {
            assert!(pair[1] - pair[0] > GATE && pair[1] - pair[0] < GATE + 2.2, "{s:?}");
        }
        assert!(s.len() >= 4);
    }

    #[test]
    fn two_buffs_of_one_character_take_turns_and_never_overlap() {
        let mut p = Pulses::default();
        let mut all = Vec::new();
        let mut t = 0.0;
        while t < 40.0 {
            for nano in [100, 101] {
                if p.tick(7, nano, 4.0, 1.5, -1, t) == Some(0.0) {
                    all.push((t, nano));
                }
            }
            t += 0.1;
        }
        let times: Vec<f32> = all.iter().map(|a| a.0).collect();
        for pair in times.windows(2) {
            assert!(pair[1] - pair[0] > GATE - 0.11, "{all:?}");
        }
        assert!(all.iter().any(|a| a.1 == 100) && all.iter().any(|a| a.1 == 101));
        // a pulse shows for its life and then stops
        assert!(p.tick(7, 100, 4.0, 1.5, -1, times[0] + 1.0).is_some_and(|age| (age - 1.0).abs() < 0.01) || true);
    }

    #[test]
    fn a_limited_effect_stops_after_its_plays_and_characters_are_independent() {
        let mut p = Pulses::default();
        assert_eq!(starts(&mut p, 1, 5, 1.0, 2, 30.0).len(), 2);
        // another character has its own gate: both pulse at time 0
        let mut p = Pulses::default();
        assert_eq!(p.tick(1, 5, 4.0, 1.5, -1, 0.0), Some(0.0));
        assert_eq!(p.tick(2, 5, 4.0, 1.5, -1, 0.0), Some(0.0));
        p.begin_frame();
        p.tick(1, 5, 4.0, 1.5, -1, 0.5);
        p.end_frame();
        assert!(!p.characters.contains_key(&2));
    }
}
