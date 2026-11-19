//! The stats the original shows but the server does not send: armour classes, attack rating and the bonuses to the
//! stored values are built by the client from the effects of what the character wears (see `docs/STAT_ENGINE.md`:
//! shown value = stored value + the modifier map). This module sums the worn items' effects; perks, research and the
//! active nanos are not known to this client yet (the server's lists for them are not decoded).

use std::collections::BTreeMap;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};
use mao_net::inventory::Entry;

/// The item stat that holds an item's quality level.
const QUALITY_LEVEL: u32 = 54;

/// Whether a placement is worn: the weapon tab (including the belt in 7, its NCU units in 9-14 and the second HUD place, 15 (a Dreadloch booster sat there), which add health and
/// nano like any worn item: a level-25 character's belt gives +100 health and each unit +50), clothes and implants. The
/// social tab (49 and up) only changes how the character looks.
pub(crate) fn worn(slot: u32) -> bool {
    matches!(slot, 1..=15 | 17..=31 | 33..=45)
}

/// An item's effects at its quality: the lowest and highest template of the item are both read and the amounts are
/// interpolated between them by the item's quality level, as the original does.
pub fn effects(db: &ResourceDatabase, entry: &Entry, level: i64) -> Vec<(u32, i64)> {
    let template = |id: u32| db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten();
    let Some(low) = template(entry.low_id) else { return Vec::new() };
    let low_mods = item::modifiers_at(&low, level);
    if entry.high_id == entry.low_id {
        return low_mods.into_iter().map(|(s, a)| (s, a as i64)).collect();
    }
    let Some(high) = template(entry.high_id) else { return low_mods.into_iter().map(|(s, a)| (s, a as i64)).collect() };
    let high_mods = item::modifiers_at(&high, level);
    let quality_of = |data: &[u8]| Item::parse(data).and_then(|i| i.stats.iter().find(|s| s.0 == QUALITY_LEVEL).map(|s| s.1 as i64));
    let (Some(low_ql), Some(high_ql)) = (quality_of(&low), quality_of(&high)) else { return low_mods.into_iter().map(|(s, a)| (s, a as i64)).collect() };
    let fraction = if high_ql > low_ql { (entry.quality as i64 - low_ql).clamp(0, high_ql - low_ql) } else { 0 };
    let span = (high_ql - low_ql).max(1);
    low_mods
        .iter()
        .enumerate()
        .map(|(i, &(stat, a))| {
            let a = a as i64;
            match high_mods.get(i) {
                // (to the nearest whole, a half upward, whichever way the amounts run: `between`)
                Some(&(s, b)) if s == stat => (stat, between(a, b as i64, fraction, span)),
                _ => (stat, a),
            }
        })
        .collect()
}

/// A value between a low and a high template's at `fraction` of the `span` between their quality levels: Gamecode
/// FUN_100cb909, the exact line's value to the nearest whole number, a half upward (it counts from the smaller of the
/// two and cuts `x + 0.5`). Until 2026-11-18 a value that falls with the quality (a penalty: -10 to -50) was cut toward
/// the low template's by the division of a negative number: -21 where the original has -22.
pub fn between(low: i64, high: i64, fraction: i64, span: i64) -> i64 {
    low + ((high - low) * fraction * 2 + span).div_euclid(2 * span)
}

/// The stats of an item that lie between its two templates' at a quality between their levels (Gamecode
/// FUN_100cbbe0's list); every other stat is the low template's. The price, stat 74, goes by a curve of its own
/// ([`price_between`]).
const BETWEEN_STATS: [u32; 26] = [1, 2, 26, 27, 36, 45, 73, 75, 101, 134, 210, 211, 212, 236, 250, 284, 285, 286, 287, 289, 294, 297, 299, 341, 422, 538];

/// The price between two templates' (Gamecode FUN_100cbb68): the line's value `v` first, then its share of the way
/// from the smaller to the larger price, squared (floats, cut to a whole number).
fn price_between(a: i64, b: i64, v: i64) -> i64 {
    let (lo, hi) = (a.min(b) as f32, a.max(b) as f32);
    if hi <= lo {
        return a;
    }
    let t = (v as f32 - lo) / (hi - lo);
    (t * t * (hi - lo) + lo) as i64
}

/// An item of two templates as it is at a quality (Gamecode FUN_100cc5a1, FUN_100cbfc8): its template's bytes and
/// what the pages and lists show of it. At the low template's level, or with one template, the low template; **at the
/// high template's level the high template itself**, name, icon and all; between the two the low template with the
/// stats of [`BETWEEN_STATS`] and the price worked out between both ([`between`]). The stats are left the low
/// template's when the two templates do not have the same number of them (the original then works out nothing), or
/// when the high one's level is not above the low one's.
pub fn item_at(db: &ResourceDatabase, low: u32, high: u32, quality: u32) -> Option<(Vec<u8>, Item)> {
    let template = |id: u32| db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten();
    let Some(low_data) = template(low) else {
        let data = template(high)?;
        let it = Item::parse(&data)?;
        return Some((data, it));
    };
    let mut it = Item::parse(&low_data)?;
    let Some((high_data, top)) = (if high == low { None } else { template(high) }).and_then(|d| Item::parse(&d).map(|i| (d, i))) else { return Some((low_data, it)) };
    let level = |i: &Item| i.stat(QUALITY_LEVEL).map(|v| v as i32 as i64);
    let (Some(l), Some(h), q) = (level(&it), level(&top), quality as i64) else { return Some((low_data, it)) };
    if q == h && h != l {
        return Some((high_data, top));
    }
    if q == l || h <= l || !(1..=511).contains(&l) || !(1..=511).contains(&h) || it.stats.len() != top.stats.len() {
        return Some((low_data, it));
    }
    for s in it.stats.iter_mut().filter(|s| BETWEEN_STATS.contains(&s.0) || s.0 == 74) {
        let Some(b) = top.stat(s.0).map(|v| v as i32 as i64) else { continue };
        let a = s.1 as i32 as i64;
        let v = between(a, b, q - l, h - l);
        s.1 = (if s.0 == 74 { price_between(a, b, v) } else { v }) as i32 as u32;
    }
    Some((low_data, it))
}

/// An item's actions (use, wear, wield) with their criteria at the item's quality: like the effects, the values of the lowest and
/// highest template are interpolated by the quality level.
pub fn actions(db: &ResourceDatabase, entry: &Entry) -> Vec<item::Action> {
    let template = |id: u32| db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten();
    let Some(low) = template(entry.low_id) else { return Vec::new() };
    let low_actions = item::actions(&low);
    if entry.high_id == entry.low_id {
        return low_actions;
    }
    let Some(high) = template(entry.high_id) else { return low_actions };
    let high_actions = item::actions(&high);
    let quality_of = |data: &[u8]| Item::parse(data).and_then(|i| i.stats.iter().find(|s| s.0 == QUALITY_LEVEL).map(|s| s.1 as i64));
    let (Some(low_ql), Some(high_ql)) = (quality_of(&low), quality_of(&high)) else { return low_actions };
    let fraction = if high_ql > low_ql { (entry.quality as i64 - low_ql).clamp(0, high_ql - low_ql) } else { 0 };
    let span = (high_ql - low_ql).max(1);
    low_actions
        .into_iter()
        .zip(high_actions.iter().map(Some).chain(std::iter::repeat(None)))
        .map(|(mut a, h)| {
            if let Some(h) = h.filter(|h| h.kind == a.kind && h.criteria.len() == a.criteria.len()) {
                for (c, hc) in a.criteria.iter_mut().zip(&h.criteria) {
                    // only the comparisons of a stat are interpolated (the operators and ids are the same on both sides)
                    if c.0 == hc.0 && c.2 == hc.2 && matches!(c.2, 0..=2) {
                        let (lo, hi) = (c.1 as i64, hc.1 as i64);
                        c.1 = between(lo, hi, fraction, span) as i32;
                    }
                }
            }
            a
        })
        .collect()
}

/// The perk-record type; its instance is the perk id and the template of the perk's item is the word at byte 12.
const PERK_RECORD: u32 = 0xf4264;

/// The summed standing effects of the perks (and research ranks) the server lists for the character, by stat. Each perk
/// rank is an item-type template reached through its perk record.
pub fn perk_modifiers(db: &ResourceDatabase, perks: &[u32], level: i64) -> BTreeMap<u32, i64> {
    let mut sum = BTreeMap::new();
    for &perk in perks {
        let Some(record) = db.get(Id { kind: PERK_RECORD, instance: perk }).ok().flatten() else { continue };
        let Some(template) = record.get(12..16).map(|b| u32::from_le_bytes(b.try_into().unwrap())) else { continue };
        let Some(data) = db.get(Id { kind: item::TEMPLATE, instance: template }).ok().flatten() else { continue };
        for (stat, amount) in item::modifiers_at(&data, level) {
            *sum.entry(stat).or_insert(0) += amount as i64;
        }
    }
    sum
}

/// The nano type of the database; its instance is the nano's id.
const NANO_TEMPLATE: u32 = 0xfde85;

/// The summed stat effects of the active nano programs, by stat (the same effect records as items carry).
pub fn nano_modifiers(db: &ResourceDatabase, nanos: &[u32], level: i64) -> BTreeMap<u32, i64> {
    let mut sum = BTreeMap::new();
    for &nano in nanos {
        let Some(data) = db.get(Id { kind: NANO_TEMPLATE, instance: nano }).ok().flatten() else { continue };
        for (stat, amount) in item::nano_modifiers_at(&data, level) {
            *sum.entry(stat).or_insert(0) += amount as i64;
        }
    }
    sum
}

/// The summed effects of the organization benefits the server lists for the character (`OrgServer` messages of type 6: a
/// quality and the low and high template of an item such as "Large Org HQ Management Benefit" or "Universal Advantage - Medical
/// Expertise"), worked out like those of an item at that quality. Their records are the level-scaled `0xcfb7` kind.
pub fn org_modifiers(db: &ResourceDatabase, benefits: &[Entry], level: i64) -> BTreeMap<u32, i64> {
    let mut sum = BTreeMap::new();
    for benefit in benefits {
        for (stat, amount) in effects(db, benefit, level) {
            *sum.entry(stat).or_insert(0) += amount;
        }
    }
    sum
}

/// Everything that adds to the stored stats: what is worn, the perks, the active nanos and the organization benefits.
pub fn modifiers(db: &ResourceDatabase, entries: &[Entry], perks: &[u32], nanos: &[u32], org: &[Entry], level: i64) -> BTreeMap<u32, i64> {
    let mut sum = worn_modifiers(db, entries, level);
    for part in [perk_modifiers(db, perks, level), nano_modifiers(db, nanos, level), org_modifiers(db, org, level)] {
        for (stat, amount) in part {
            *sum.entry(stat).or_insert(0) += amount;
        }
    }
    sum
}

/// The summed effects of everything worn, by stat.
pub fn worn_modifiers(db: &ResourceDatabase, entries: &[Entry], level: i64) -> BTreeMap<u32, i64> {
    let mut sum = BTreeMap::new();
    for entry in entries.iter().filter(|e| !e.is_bag() && worn(e.slot)) {
        for (stat, amount) in effects(db, entry, level) {
            *sum.entry(stat).or_insert(0) += amount;
        }
    }
    sum
}

/// The attribute stats, in the order of the trickle-down table's columns.
const ATTRIBUTES: [u32; 6] = [0x10, 0x11, 0x12, 0x13, 0x14, 0x15];

/// What the attributes add to a skill: the skill's share of each attribute (`trickle_data::WEIGHTS`, percent), a quarter of
/// the weighted sum, cut to a whole number. Worked out as the original does it (doubles for five attributes, floats for the
/// last), which matched all 69 skills of a live character.
pub fn trickle(weights: &[u8; 6], attributes: &[i64; 6]) -> i64 {
    let first_five: f64 = (0..5).map(|i| weights[i] as f64 * attributes[i] as f64 / 100.0).sum();
    let last = weights[5] as f32 * attributes[5] as f32 / 100.0f32;
    ((last + first_five as f32) * 0.25f32) as i64
}

/// A skill nothing has trained has this stored value (the server does not send it).
const DEFAULT_SKILL: i64 = 5;

/// What a shown value is made of: the stored (trained) value, the bonuses of what is worn, of the perks and of the active
/// nanos, and for a skill what the attributes trickle down to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Breakdown {
    pub stored: i64,
    pub worn: i64,
    pub perks: i64,
    pub nanos: i64,
    pub trickle: i64,
    /// The bonuses of the organization's benefits (see [`org_modifiers`]).
    pub org: i64,
}

impl Breakdown {
    pub fn total(&self) -> i64 {
        self.stored + self.worn + self.perks + self.nanos + self.trickle + self.org
    }
}

/// The title level of a character level (the server does not send it; the original works it out): 1 below level 15, 2 below 50,
/// 3 below 100, 4 below 150, 5 below 190, 6 below 205, else 7 (checked against characters of level 5 to 220).
pub fn title_level(level: i64) -> i64 {
    match level {
        ..=14 => 1,
        15..=49 => 2,
        50..=99 => 3,
        100..=149 => 4,
        150..=189 => 5,
        190..=204 => 6,
        _ => 7,
    }
}

/// Every stat's parts (stats without a stored value count from zero, an untrained skill from 5).
pub fn breakdown(stored: &BTreeMap<u32, i64>, worn: &BTreeMap<u32, i64>, perks: &BTreeMap<u32, i64>, nanos: &BTreeMap<u32, i64>, org: &BTreeMap<u32, i64>) -> BTreeMap<u32, Breakdown> {
    let mut out: BTreeMap<u32, Breakdown> = stored.iter().map(|(&s, &v)| (s, Breakdown { stored: v, ..Default::default() })).collect();
    for (map, part) in [(worn, 0), (perks, 1), (nanos, 2), (org, 3)] {
        for (&stat, &amount) in map {
            let b = out.entry(stat).or_default();
            match part {
                0 => b.worn += amount,
                1 => b.perks += amount,
                2 => b.nanos += amount,
                _ => b.org += amount,
            }
        }
    }
    if let (Some(level), false) = (out.get(&0x36).map(Breakdown::total), out.contains_key(&37)) {
        out.insert(37, Breakdown { stored: title_level(level), ..Default::default() });
    }
    let attributes = ATTRIBUTES.map(|a| out.get(&a).map_or(0, Breakdown::total));
    if attributes.iter().any(|&a| a > 0) {
        for &(stat, ref weights) in crate::trickle_data::WEIGHTS {
            let b = out.entry(stat).or_insert_with(|| Breakdown { stored: DEFAULT_SKILL, ..Default::default() });
            b.trickle = trickle(weights, &attributes);
        }
    }
    out
}

/// The shown values of a set of parts.
pub fn totals(parts: &BTreeMap<u32, Breakdown>) -> BTreeMap<u32, i64> {
    parts.iter().map(|(&s, b)| (s, b.total())).collect()
}

/// The shown stats: the stored ones plus the modifiers (stats without a stored value count from zero), and for the skills
/// the trickle-down of the shown attributes.
pub fn shown(stored: &BTreeMap<u32, i64>, modifiers: &BTreeMap<u32, i64>) -> BTreeMap<u32, i64> {
    let mut shown = totals(&breakdown(stored, modifiers, &BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new()));
    // (under the resurrection shock the skills and the abilities are less: `shock_penalty`)
    if stored.get(&SHOCK).is_some_and(|s| *s > 0) {
        for (stat, value) in shown.iter_mut() {
            *value -= shock_penalty(stored, *stat, *value - modifiers.get(stat).copied().unwrap_or(0));
        }
    }
    shown
}

/// The resurrection shock's stat: 5000 when a character of level 15 or more is brought back where it lies, counted
/// down by the client itself.
pub const SHOCK: u32 = 0xf7;

/// What the resurrection shock takes off a skill or an ability (Gamecode FUN_10064b51, which every shown read of a
/// stat goes through: FUN_10064d46, FUN_10065a92): while the shock is above 0, a skill (stats 100 to 168) or an
/// ability (16 to 21) is less `shock * base / 10000`, cut to a whole number, `base` being its value before the
/// modifiers. Half of it at the shock's start.
pub fn shock_penalty(stored: &BTreeMap<u32, i64>, stat: u32, base: i64) -> i64 {
    let shock = stored.get(&SHOCK).copied().unwrap_or(0);
    if shock > 0 && ((100..=168).contains(&stat) || (16..=21).contains(&stat)) { shock * base / 10000 } else { 0 }
}

/// The shock a tick later (the character's frame function FUN_1005b55d, on the game clock's ping every two
/// seconds): 27 less, not under 0.
pub fn shock_step(shock: i64) -> i64 {
    (shock - 27).max(0)
}

/// One stat's shown value, as [`shown`] has it, without working out all the others: the original's read of a stat in
/// mode 3 (the stat container's FUN_10059399 → FUN_10064d46: the stored value and the modifier map's part for it, with
/// the attributes' trickle for a skill). The movement reads `RunSpeed` so (FUN_1006f2fc) and the body its `Scale`.
pub fn shown_one(stored: &BTreeMap<u32, i64>, modifiers: &BTreeMap<u32, i64>, stat: u32) -> i64 {
    let bonus = modifiers.get(&stat).copied().unwrap_or(0);
    let attributes = ATTRIBUTES.map(|a| stored.get(&a).copied().unwrap_or(0) + modifiers.get(&a).copied().unwrap_or(0));
    match crate::trickle_data::WEIGHTS.iter().find(|w| w.0 == stat).filter(|_| attributes.iter().any(|&a| a > 0)) {
        Some((_, weights)) => {
            let unknown = if modifiers.contains_key(&stat) { 0 } else { DEFAULT_SKILL };
            let base = stored.get(&stat).copied().unwrap_or(unknown) + trickle(weights, &attributes);
            base + bonus - shock_penalty(stored, stat, base)
        }
        None => {
            let base = stored.get(&stat).copied().unwrap_or(0);
            base + bonus - shock_penalty(stored, stat, base)
        }
    }
}

/// A stat as the criteria evaluator reads it (Gamecode FUN_100065d8: the holder's `GetStat` in mode 0, which for a
/// character is FUN_10059399): the value the windows show ([`shown_one`]: FUN_10065a92), but the stored value alone
/// for the stats that function reads so whatever the mode: 2, 0x1b, 0x36, 0xcc, 0x167, 0x184, 0x1c0, 0x1c6, 0x1c7,
/// 0x1c9, 0x1e4, 0x206 and those from 0x44c on. (Its stats made of two others, 0x20d to 0x274, and a morphed
/// character's second container are not here.)
pub fn judged_one(stored: &BTreeMap<u32, i64>, modifiers: &BTreeMap<u32, i64>, stat: u32) -> i64 {
    if matches!(stat, 2 | 0x1b | 0x36 | 0xcc | 0x167 | 0x184 | 0x1c0 | 0x1c6 | 0x1c7 | 0x1c9 | 0x1e4 | 0x206) || stat >= 0x44c {
        return stored.get(&stat).copied().unwrap_or(0);
    }
    shown_one(stored, modifiers, stat)
}

/// The modifier map as it will be with one worn item taken off: its effects less. An item in a place that adds
/// nothing (the backpack, the social tab) changes nothing.
pub fn modifiers_without(db: &ResourceDatabase, modifiers: &BTreeMap<u32, i64>, off: &Entry, level: i64) -> BTreeMap<u32, i64> {
    let mut left = modifiers.clone();
    if !off.is_bag() && worn(off.slot) {
        for (stat, amount) in effects(db, off, level) {
            *left.entry(stat).or_insert(0) -= amount;
        }
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The resurrection shock lowers skills and abilities by its share of their value before the modifiers, and
    /// counts down by 27.
    #[test]
    fn the_shock_takes_its_share_of_skills_and_abilities() {
        let stored: BTreeMap<u32, i64> = [(SHOCK, 5000), (17, 200), (27, 900)].into_iter().collect();
        let modifiers: BTreeMap<u32, i64> = [(17, 30)].into_iter().collect();
        // (an ability: half of its 200 at the shock's start, the modifier untouched; health is no skill)
        assert_eq!(shown_one(&stored, &modifiers, 17), 200 + 30 - 100);
        assert_eq!(shown_one(&stored, &modifiers, 27), 900);
        assert_eq!(shown(&stored, &modifiers).get(&17), Some(&130));
        assert_eq!((shock_penalty(&stored, 100, 301), shock_penalty(&stored, 168, 10), shock_penalty(&stored, 169, 10), shock_penalty(&stored, 15, 10)), (150, 5, 0, 0));
        let calm: BTreeMap<u32, i64> = [(17, 200)].into_iter().collect();
        assert_eq!(shown_one(&calm, &modifiers, 17), 230);
        assert_eq!((shock_step(5000), shock_step(27), shock_step(5)), (4973, 0, 0));
    }

    /// `MAO_GAME`: the Belt Component Platform between the Ti-200X (level 20) and the 300X (level 40): at 30 it has
    /// the low template's name, deck places and a price between the two; at 40 it is the 300X itself.
    #[test]
    fn an_item_between_two_templates() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let at = |q: u32| item_at(&db, 36782, 36777, q).unwrap().1;
        let (low, mid, top) = (at(20), at(30), at(40));
        assert!(low.name.contains("Ti-200X") && mid.name.contains("Ti-200X") && top.name.contains("300X"));
        // (deck places 2 and 3: 2.5 goes up; the value stat 2 falls from 2000 to 1000)
        assert_eq!((low.stat(45), mid.stat(45), top.stat(45)), (Some(2), Some(3), Some(3)));
        assert_eq!((low.stat(2), mid.stat(2), top.stat(2)), (Some(2000), Some(1500), Some(1000)));
        // (the price 2500 to 10000: half the way is a quarter of the difference)
        assert_eq!((low.stat(74), mid.stat(74), top.stat(74)), (Some(2500), Some(4375), Some(10000)));
        assert_eq!(price_between(10000, 2500, 6250), 4375);
        // (one template: itself)
        assert_eq!(item_at(&db, 252158, 252158, 1).unwrap().1.stat(45), Some(2));
    }

    /// FUN_100cb909's rounding: rising and falling amounts alike to the nearest whole, a half upward.
    #[test]
    fn a_value_between_two_templates_is_the_nearest_whole() {
        // (levels 1 and 100, quality 30: 29 of 99)
        assert_eq!(between(-10, -50, 29, 99), -22);
        assert_eq!(between(10, 50, 29, 99), 22);
        assert_eq!(between(30, 120, 10, 20), 75);
        // (a half goes up: -10.5 is -10, 10.5 is 11)
        assert_eq!((between(-10, -11, 1, 2), between(10, 11, 1, 2)), (-10, 11));
        assert_eq!((between(5, 9, 0, 7), between(5, 9, 7, 7)), (5, 9));
    }

    /// One stat's shown value is what the whole table has for it: a trained skill, an untrained one with and without a
    /// bonus, an attribute and a stat that is neither.
    #[test]
    fn one_shown_value_is_the_tables() {
        let stored: BTreeMap<u32, i64> = [(0x10, 100), (0x11, 300), (0x12, 250), (0x13, 100), (0x14, 80), (0x15, 100), (156, 121), (1, 3805), (360, 100)].into();
        let modifiers: BTreeMap<u32, i64> = [(156, 300), (0x11, 20), (1, 40), (103, 7)].into();
        let all = shown(&stored, &modifiers);
        for stat in [156u32, 0x11, 1, 360, 103, 104] {
            assert_eq!(shown_one(&stored, &modifiers, stat), all[&stat], "stat {stat}");
        }
        // (RunSpeed: the trained 121, the worn 300 and what strength, agility, stamina and sense give)
        assert_eq!(shown_one(&stored, &modifiers, 156), 121 + 300 + trickle(&[20, 40, 40, 0, 10, 0], &[100, 320, 250, 100, 80, 100]));
        assert_eq!(shown_one(&BTreeMap::new(), &BTreeMap::new(), 156), 0);
    }

    #[test]
    fn the_shown_value_adds_the_modifier_to_the_stored_one() {
        let stored: BTreeMap<u32, i64> = [(1, 100), (0x36, 8)].into();
        let modifiers: BTreeMap<u32, i64> = [(1, 25), (0x5b, 230)].into();
        let shown = shown(&stored, &modifiers);
        assert_eq!(shown[&1], 125);
        assert_eq!(shown[&0x5b], 230);
        assert_eq!(shown[&0x36], 8);
    }

    #[test]
    fn the_parts_add_up_and_name_their_sources() {
        let stored: BTreeMap<u32, i64> = [(0x10, 100), (0x11, 100), (0x12, 100), (0x13, 100), (0x14, 100), (0x15, 100), (103, 40)].into();
        let worn: BTreeMap<u32, i64> = [(103, 7), (0x10, 20)].into();
        let perks: BTreeMap<u32, i64> = [(103, 3)].into();
        let nanos: BTreeMap<u32, i64> = [(103, 10)].into();
        let parts = breakdown(&stored, &worn, &perks, &nanos, &BTreeMap::new());
        let b = parts[&103];
        assert_eq!((b.stored, b.worn, b.perks, b.nanos), (40, 7, 3, 10));
        assert_eq!(b.trickle, trickle(&[30, 40, 30, 0, 0, 0], &[120, 100, 100, 100, 100, 100]));
        assert_eq!(b.total(), 60 + b.trickle);
        // an untrained skill starts at 5
        assert_eq!(parts[&100].stored, 5);
        assert_eq!(totals(&parts)[&0x10], 120);
    }

    #[test]
    fn the_title_level_follows_the_level() {
        // the original's own values: level 86 -> 3, level 220 -> 7
        assert_eq!((title_level(5), title_level(25), title_level(86), title_level(125), title_level(220)), (1, 2, 3, 4, 7));
        let stored: BTreeMap<u32, i64> = [(0x36, 86)].into();
        assert_eq!(shown(&stored, &BTreeMap::new())[&37], 3);
    }

    #[test]
    fn the_trickle_matches_a_live_skill() {
        // Conjecture (attributes as shown by the original): skill 100 takes 20% strength, 50% agility, 30% psychic.
        assert_eq!(trickle(&[20, 50, 0, 0, 0, 30], &[1175, 1026, 1128, 904, 1001, 873]), 252);
        assert_eq!(trickle(&[50, 10, 40, 0, 0, 0], &[1175, 1026, 1128, 904, 1001, 873]), 285);
    }

    #[test]
    fn an_untrained_skill_starts_at_five() {
        let stored: BTreeMap<u32, i64> = [(0x10, 100), (0x11, 100), (0x12, 100), (0x13, 100), (0x14, 100), (0x15, 100)].into();
        let shown = shown(&stored, &BTreeMap::new());
        assert_eq!(shown[&100], 5 + 25); // every attribute 100: a quarter of 100
    }

    #[test]
    fn only_the_worn_places_count() {
        assert!(worn(6) && worn(7) && worn(9) && worn(21) && worn(40));
        assert!(worn(15) && !worn(16) && !worn(49) && !worn(64));
    }
}
