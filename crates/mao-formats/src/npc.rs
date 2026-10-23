//! NPC templates: the objects of type 0xfde97 that a creature's entity
//! message points at (by instance). A template is little-endian words: the
//! type, the instance, 1000000, a count, then (key, value) pairs among which
//! key 12 is the CAT mesh (type 0xf6952) and key 64, where present, a head
//! mesh; later (key, 2018, instance) triples name the animations (type
//! 0xf6953) by action key.

use crate::appearance::Action;

/// The object type of an NPC template.
pub const TEMPLATE: u32 = 0xfde97;
/// The type number that marks an animation in the triples.
const ANIMATION: u32 = 2018;
/// How many pairs after the header are read as stats (the triples follow).
const STATS: usize = 12;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Template {
    /// The CAT mesh's instance.
    pub mesh: Option<u32>,
    pub head: Option<u32>,
    /// (action key, animation instance): 100 walk, 101 run, 120 idle.
    pub animations: Vec<(u32, u32)>,
}

impl Template {
    pub fn parse(data: &[u8]) -> Option<Template> {
        let words: Vec<u32> = data.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
        if words.len() < 6 || words[0] != TEMPLATE {
            return None;
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

    /// The animation for an action (the walk stands in for a missing run).
    pub fn animation(&self, action: Action) -> Option<u32> {
        let find = |key: u32| self.animations.iter().find(|a| a.0 == key).map(|a| a.1);
        match action {
            Action::Walk => find(100),
            Action::Run => find(101).or_else(|| find(100)),
            _ => find(120),
        }
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
        assert_eq!(t.animation(Action::Run), Some(17327));
    }
}
