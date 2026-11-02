//! A character's progress: its level, experience, health, credits, where it stands and what it carries, and the `Stat` message that
//! tells the client of the stats. The curve of experience and the health per level are the server's own for now: the original's
//! tables are not in the captures (see `docs/AO_SERVER.md`).

use mao_net::n3::Message;

use crate::templates::{self, int, list, record};

/// The stat numbers the client reads (`GameData.Stat`: the same numbers as `mao-client`'s `stats.rs`).
pub mod stat {
    pub const MAX_HEALTH: u32 = 0x01;
    pub const HEALTH: u32 = 0x1b;
    pub const XP: u32 = 0x34;
    pub const LAST_XP: u32 = 0x39;
    pub const LEVEL: u32 = 0x36;
    pub const CREDITS: u32 = 0x3d;
    pub const NEXT_XP: u32 = 0x15e;
}

/// The highest level a character can reach.
pub const MAX_LEVEL: u32 = 220;

/// The first backpack slot (the original's numbering: the backpack is 0x40 and up).
pub const BACKPACK_FIRST: u32 = 0x40;
/// How many backpack slots a character has.
pub const BACKPACK_SLOTS: u32 = 30;

/// The total experience a character needs to have to be at `level` (the server's own curve).
pub fn xp_for_level(level: u32) -> u64 {
    if level <= 1 {
        return 0;
    }
    500 * (level as u64 - 1) * level as u64
}

/// The level a total of experience gives (1 to [`MAX_LEVEL`]).
pub fn level_for_xp(xp: u64) -> u32 {
    (1..=MAX_LEVEL).take_while(|&l| xp_for_level(l) <= xp).last().unwrap_or(1)
}

/// The health a character has at `level` (the server's own curve).
pub fn max_health_for(level: u32) -> u32 {
    100 + 10 * level.min(MAX_LEVEL)
}

/// An item a character carries: where it is, its template (the item's low id; its high id is the same), its stack size and its
/// quality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub placement: u32,
    pub template: u32,
    pub count: u16,
    pub quality: u32,
}

/// A character's progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Hero {
    pub id: u32,
    pub level: u32,
    pub xp: u64,
    pub health: u32,
    pub credits: u32,
    pub playfield: u32,
    pub position: [f32; 3],
    pub items: Vec<Item>,
    /// The missions taken, in the order they were taken.
    pub missions: Vec<crate::missions::Taken>,
}

impl Hero {
    /// A new character: first level, full health, nothing carried.
    pub fn new(id: u32) -> Hero {
        Hero { id, level: 1, xp: 0, health: max_health_for(1), credits: 0, playfield: 0, position: [0.0; 3], items: Vec::new(), missions: Vec::new() }
    }

    pub fn max_health(&self) -> u32 {
        max_health_for(self.level)
    }

    /// Adds experience, and the levels and health it brings. Returns whether the character went up a level.
    pub fn grant_xp(&mut self, amount: u64) -> bool {
        let before = self.level;
        self.xp = self.xp.saturating_add(amount);
        self.level = level_for_xp(self.xp).max(self.level);
        if self.level > before {
            self.health = self.max_health();
        }
        self.level > before
    }

    /// The first free backpack slot, if there is one.
    pub fn free_slot(&self) -> Option<u32> {
        (BACKPACK_FIRST..BACKPACK_FIRST + BACKPACK_SLOTS).find(|slot| !self.items.iter().any(|i| i.placement == *slot))
    }

    /// Puts an item in the backpack (a stack of an item it already carries grows instead). Returns its slot, or none when the
    /// backpack is full.
    pub fn add_item(&mut self, template: u32, count: u16, quality: u32) -> Option<u32> {
        if let Some(item) = self.items.iter_mut().find(|i| i.template == template && i.quality == quality && i.placement >= BACKPACK_FIRST) {
            item.count = item.count.saturating_add(count);
            return Some(item.placement);
        }
        let slot = self.free_slot()?;
        self.items.push(Item { placement: slot, template, count, quality });
        Some(slot)
    }

    /// Takes `price` credits off the character. Fails (and takes nothing) when it has fewer.
    pub fn spend(&mut self, price: u32) -> Result<(), u32> {
        self.credits = self.credits.checked_sub(price).ok_or(self.credits)?;
        Ok(())
    }
}

/// The stats the client keeps: (stat, value).
pub fn stats(hero: &Hero) -> Vec<(u32, u32)> {
    vec![
        (stat::LEVEL, hero.level),
        (stat::HEALTH, hero.health),
        (stat::MAX_HEALTH, hero.max_health()),
        (stat::XP, hero.xp.min(u32::MAX as u64) as u32),
        (stat::NEXT_XP, xp_for_level(hero.level + 1).min(u32::MAX as u64) as u32),
        (stat::LAST_XP, xp_for_level(hero.level).min(u32::MAX as u64) as u32),
        (stat::CREDITS, hero.credits),
    ]
}

/// The `Stat` message (`2b333d6e`) with the given stats, from the captured one.
pub fn stat_message(character: u32, values: &[(u32, u32)]) -> Result<Message, String> {
    let mut t = templates::of(0x2b333d6e).ok_or("no Stat template")?;
    let items = values.iter().map(|(s, v)| record(vec![("Item1", int(*s as i64)), ("Item2", int(*v as i64))])).collect();
    t.set("Stats", list(items));
    t.message(0x2b333d6e, (mao_net::n3::CHARACTER, character))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_levels_follow_the_curve() {
        assert_eq!(level_for_xp(0), 1);
        assert_eq!(level_for_xp(xp_for_level(2)), 2);
        assert_eq!(level_for_xp(xp_for_level(2) - 1), 1);
        assert_eq!(level_for_xp(xp_for_level(10)), 10);
        assert_eq!(level_for_xp(u64::MAX), MAX_LEVEL);
    }

    #[test]
    fn a_level_up_fills_the_health() {
        let mut h = Hero::new(7);
        h.health = 3;
        assert!(!h.grant_xp(10));
        assert_eq!(h.health, 3);
        assert!(h.grant_xp(xp_for_level(2)));
        assert_eq!((h.level, h.health), (2, h.max_health()));
    }

    #[test]
    fn items_stack_and_fill_the_backpack_in_order() {
        let mut h = Hero::new(7);
        assert_eq!(h.add_item(0x100, 1, 5), Some(BACKPACK_FIRST));
        assert_eq!(h.add_item(0x100, 2, 5), Some(BACKPACK_FIRST));
        assert_eq!(h.items[0].count, 3);
        assert_eq!(h.add_item(0x200, 1, 5), Some(BACKPACK_FIRST + 1));
        for i in 0..BACKPACK_SLOTS {
            h.add_item(0x300 + i, 1, 1);
        }
        assert_eq!(h.add_item(0x999, 1, 1), None);
    }

    #[test]
    fn credits_are_spent_or_refused() {
        let mut h = Hero::new(7);
        h.credits = 50;
        assert!(h.spend(60).is_err());
        assert_eq!(h.credits, 50);
        assert!(h.spend(20).is_ok());
        assert_eq!(h.credits, 30);
    }

    #[test]
    fn the_stat_message_carries_the_stats() {
        let hero = Hero::new(7);
        let m = stat_message(7, &stats(&hero)).unwrap();
        let parsed = mao_net::schema::Schema::global().parse_n3(0x2b333d6e, &m.data).unwrap();
        let first = match parsed.value.get("Stats") {
            Some(mao_net::schema::Value::List(items)) => items[0].clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(first.get("Item1").and_then(|v| v.as_int()), Some(stat::LEVEL as i64));
        assert_eq!(first.get("Item2").and_then(|v| v.as_int()), Some(1));
    }
}
