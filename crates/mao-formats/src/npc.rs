//! NPC templates (animation sets): the objects of type 0xfde97 a character's `MonsterData` (stat 359) names, which its
//! entity message points at (by instance). Gamecode reads one with FUN_1004e174 → FUN_1004de5d: little-endian words, the
//! type and the instance, two words, then sections each led by two words (a key and a size) whose lists are counted the
//! game's way, as 1009 × (count + 1):
//! - key 23: the stats (FUN_1002e7d9), (stat, value) pairs; 12 is the CAT mesh (type 0xf6952), 64 a head mesh;
//! - key 14: the animations (FUN_1007db83), per action its key, a count and that many animations (type 0xf6953): the
//!   action's variants, one picked at random each time it is asked for (FUN_10045976);
//! - key 20: the sounds (FUN_1007dc1e), then the name.

use crate::appearance::Action;

/// The object type of an NPC template.
pub const TEMPLATE: u32 = 0xfde97;
/// A count of one, the way the lists count (1009 × 2): the heuristic reader of an odd record looks for it.
const ANIMATION: u32 = 2018;
/// How many pairs after the header the heuristic reader takes as stats.
const STATS: usize = 12;

/// A list's count, counted as 1009 × (count + 1).
fn count(word: u32) -> Option<usize> {
    (word >= 1009 && word % 1009 == 0).then(|| (word / 1009 - 1) as usize)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Template {
    /// The CAT mesh's instance.
    pub mesh: Option<u32>,
    pub head: Option<u32>,
    /// (action key, animation instance): 100 walk, 101 run, 120 idle. An action with variants is listed once per variant, in
    /// the record's order.
    pub animations: Vec<(u32, u32)>,
    /// (event key, sound ID): the sounds section (FUN_1007dc1e into the set's sound map, FUN_1004e647), an event with
    /// variants listed once per variant; Gamecode plays them by the animation notes' events (FUN_100452d3: 0x26 a step, 0x85 a
    /// landing, 0xf an idle, 0x77.. an attack's start) and the attacks' (0xb, 0x1f an impact).
    pub sounds: Vec<(u32, u32)>,
    /// The stats section's (stat, value) pairs (FUN_1002e7d9; FUN_1002e740 looks one up, 0 by default): 12 the mesh, 64 the
    /// head, 0x29 the sound material its impacts play on (FUN_1009b9f3).
    pub stats: Vec<(u32, u32)>,
}

impl Template {
    pub fn parse(data: &[u8]) -> Option<Template> {
        let words: Vec<u32> = data.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
        if words.len() < 6 || words[0] != TEMPLATE {
            return None;
        }
        if let Some(t) = Self::parse_sections(&words) {
            return Some(t);
        }
        let mut template = Template::default();
        for pair in words[4..].chunks_exact(2).take(STATS) {
            match pair[0] {
                12 => template.mesh = Some(pair[1]),
                64 => template.head = Some(pair[1]),
                _ => {}
            }
        }
        for at in 4..words.len().saturating_sub(2) {
            if words[at + 1] == ANIMATION && !template.animations.iter().any(|a| a.0 == words[at]) {
                template.animations.push((words[at], words[at + 2]));
            }
        }
        Some(template)
    }

    /// The record read as FUN_1004de5d reads it (None when its layout is not that).
    fn parse_sections(words: &[u32]) -> Option<Template> {
        let mut template = Template::default();
        // (the stats: after the type, the instance and two words, a key and the count)
        let mut at = 4;
        let n = count(*words.get(at + 1)?)?;
        for pair in words.get(at + 2..at + 2 + 2 * n)?.chunks_exact(2) {
            template.stats.push((pair[0], pair[1]));
            match pair[0] {
                12 => template.mesh = Some(pair[1]),
                64 => template.head = Some(pair[1]),
                _ => {}
            }
        }
        at += 2 + 2 * n;
        // (the animations: two words, then the count of actions and each action's key, count and animations)
        let n = count(*words.get(at + 2)?)?;
        at += 3;
        for _ in 0..n {
            let key = *words.get(at)?;
            let k = count(*words.get(at + 1)?)?;
            for &anim in words.get(at + 2..at + 2 + k)? {
                template.animations.push((key, anim));
            }
            at += 2 + k;
        }
        // (the sounds: two words (20 and a size), then the count of events and each event's key, count and sounds; a count
        // that isn't one stops the reading, as FUN_1007dc1e's `clear(4)` does)
        if words.get(at) == Some(&20) {
            if let Some(n) = words.get(at + 2).and_then(|&w| count(w)) {
                at += 3;
                for _ in 0..n {
                    let (Some(&key), Some(k)) = (words.get(at), words.get(at + 1).and_then(|&w| count(w)).filter(|&k| k <= 30000)) else { break };
                    let Some(ids) = words.get(at + 2..at + 2 + k) else { break };
                    template.sounds.extend(ids.iter().map(|&id| (key, id)));
                    at += 2 + k;
                }
            }
        }
        Some(template)
    }

    /// A stat of the set (FUN_1002e740 with its default 0).
    pub fn stat(&self, stat: u32) -> u32 {
        self.stats.iter().find(|s| s.0 == stat).map_or(0, |s| s.1)
    }

    /// The sound of an event `seed` picks among its variants (FUN_10045976 picks with `rand()`), if the set has one.
    pub fn sound(&self, key: u32, seed: u32) -> Option<u32> {
        let all: Vec<u32> = self.sounds.iter().filter(|s| s.0 == key).map(|s| s.1).collect();
        (!all.is_empty()).then(|| all[seed as usize % all.len()])
    }

    /// The variant of an action's animation `seed` picks (FUN_10045976 picks with `rand()`), if the record has the action.
    pub fn variant(&self, key: u32, seed: u32) -> Option<u32> {
        let all: Vec<u32> = self.animations.iter().filter(|a| a.0 == key).map(|a| a.1).collect();
        (!all.is_empty()).then(|| all[seed as usize % all.len()])
    }

    /// The animation for an action (the walk stands in for a missing run).
    pub fn animation(&self, action: Action) -> Option<u32> {
        let find = |key: u32| self.animations.iter().find(|a| a.0 == key).map(|a| a.1);
        match action {
            Action::Walk => find(100),
            Action::Run => find(101).or_else(|| find(100)),
            Action::StrafeLeft => find(134).or_else(|| find(100)),
            Action::StrafeRight => find(135).or_else(|| find(100)),
            Action::WalkBack => find(136).or_else(|| find(100)),
            Action::RunBack => find(222).or_else(|| find(101)).or_else(|| find(100)),
            // (sneaking is 0x66 `sneakcool`, falling to the walk, FUN_10010b3c)
            Action::Sneak => find(102).or_else(|| find(100)),
            Action::SneakStand => find(102).or_else(|| find(120)).or_else(|| find(127)),
            // (flying is 0xb3 `hover-norm`, falling to the walk, FUN_10010b3c)
            Action::Hover => find(179).or_else(|| find(100)),
            // (the standing animation is 120; a creature without it, such as a bird in flight, has 127, else the walk's first frame stands)
            _ => find(120).or_else(|| find(127)).or_else(|| find(100)),
        }
    }

    /// The animations to try for an action, best first, as (key, instance): a creature's record can name one the database does not hold
    /// (a flier's standing animation was seen pointing nowhere), so the caller takes the first that loads.
    /// Of an action with variants, the one `seed` picks.
    pub fn animation_candidates(&self, action: Action, seed: u32) -> Vec<(u32, u32)> {
        let find = |key: u32| self.variant(key, seed).map(|a| (key, a));
        let order: &[u32] = match action {
            Action::Walk => &[100, 101, 120, 127],
            Action::Run => &[101, 100, 120, 127],
            Action::StrafeLeft => &[134, 100, 101, 120],
            Action::StrafeRight => &[135, 100, 101, 120],
            Action::WalkBack => &[136, 100, 101, 120],
            Action::RunBack => &[222, 101, 100, 120],
            Action::Sneak => &[102, 100, 101, 120],
            Action::SneakStand => &[102, 120, 127, 100],
            Action::Hover => &[179, 100, 101, 120],
            _ => &[120, 127, 100, 101],
        };
        order.iter().filter_map(|&k| find(k)).collect()
    }

    /// Whether the animation for a standing creature is really a movement cycle, to be held at its first frame.
    pub fn stands_on_walk(&self) -> bool {
        let has = |key: u32| self.animations.iter().any(|a| a.0 == key);
        !has(120) && !has(127) && has(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(w: &[u32]) -> Vec<u8> {
        w.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    /// The Waste Collector's template, abridged from the game data.
    #[test]
    fn waste_collector() {
        let data = words(&[TEMPLATE, 0x4532, 1_000_000, 15, 23, 8072, 12, 0x43a4, 421, 3, 2, 100_000, 0, 0, 224, 32775, 197, 0, 41, 0, 14, 19, 18162, 100, 2018, 17327, 120, 2018, 17321]);
        let t = Template::parse(&data).unwrap();
        assert_eq!(t.mesh, Some(0x43a4));
        assert_eq!(t.animation(Action::Walk), Some(17327));
        assert_eq!(t.animation(Action::Idle), Some(17321));
        assert!(!t.stands_on_walk());
        // a creature with no standing animation (a flier): the second standing one, else the walk held still
        let flier = Template { animations: vec![(100, 1), (101, 2), (127, 3)], ..Default::default() };
        assert_eq!(flier.animation(Action::Idle), Some(3));
        let walker = Template { animations: vec![(100, 1)], ..Default::default() };
        assert_eq!(walker.animation(Action::Idle), Some(1));
        assert!(walker.stands_on_walk());
        assert_eq!(t.animation(Action::Run), Some(17327));
    }

    /// An abridged "solitus male" (26092): an action with three variants, and the sounds after the animations.
    #[test]
    fn sections_and_variants() {
        let data = words(&[TEMPLATE, 0x65ec, 1_000_000, 15, 23, 1009 * 3, 12, 0x1713, 64, 0x9ef6, 14, 19, 1009 * 3, 100, 2018, 0x27cf, 0xb7, 1009 * 4, 0x186dd, 0x186de, 0x186df, 20, 5, 1009 * 2, 0x1e, 2018, 0x8ee1_3fa5]);
        let t = Template::parse(&data).unwrap();
        assert_eq!((t.mesh, t.head), (Some(0x1713), Some(0x9ef6)));
        assert_eq!(t.animations, vec![(100, 0x27cf), (0xb7, 0x186dd), (0xb7, 0x186de), (0xb7, 0x186df)]);
        assert_eq!(t.variant(0xb7, 4), Some(0x186de));
        assert_eq!(t.variant(0x1e, 0), None);
        assert_eq!(t.animation_candidates(Action::Walk, 9), vec![(100, 0x27cf)]);
        assert_eq!(t.sounds, vec![(0x1e, 0x8ee1_3fa5)]);
        assert_eq!(t.sound(0x1e, 3), Some(0x8ee1_3fa5));
    }
}
