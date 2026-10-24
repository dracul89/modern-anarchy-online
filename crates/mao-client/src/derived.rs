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

/// Whether a placement is worn: the weapon tab (including the belt in 7, its NCU units in 9-14 and the two places after them, 15 and 16, where a Dreadloch booster sat, which add health and
/// nano like any worn item: a level-25 character's belt gives +100 health and each unit +50), clothes and implants. The
/// social tab (49 and up) only changes how the character looks.
fn worn(slot: u32) -> bool {
    matches!(slot, 1..=16 | 17..=31 | 33..=45)
}

/// An item's effects at its quality: the lowest and highest template of the item are both read and the amounts are
/// interpolated between them by the item's quality level, as the original does.
fn effects(db: &ResourceDatabase, entry: &Entry, level: i64) -> Vec<(u32, i64)> {
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
                Some(&(s, b)) if s == stat => (stat, a + ((b as i64 - a) * fraction * 2 + span) / (2 * span)),
                _ => (stat, a),
            }
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

/// Everything that adds to the stored stats: what is worn, the perks and the active nanos.
pub fn modifiers(db: &ResourceDatabase, entries: &[Entry], perks: &[u32], nanos: &[u32], level: i64) -> BTreeMap<u32, i64> {
    let mut sum = worn_modifiers(db, entries, level);
    for part in [perk_modifiers(db, perks, level), nano_modifiers(db, nanos, level)] {
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

/// The shown stats: the stored ones plus the modifiers (stats without a stored value count from zero), and for the skills
/// the trickle-down of the shown attributes.
pub fn shown(stored: &BTreeMap<u32, i64>, modifiers: &BTreeMap<u32, i64>) -> BTreeMap<u32, i64> {
    let mut out = stored.clone();
    for (&stat, &amount) in modifiers {
        *out.entry(stat).or_insert(0) += amount;
    }
    let attributes = ATTRIBUTES.map(|a| out.get(&a).copied().unwrap_or(0));
    if attributes.iter().any(|&a| a > 0) {
        for &(stat, ref weights) in crate::trickle_data::WEIGHTS {
            let base = if stored.contains_key(&stat) { 0 } else { DEFAULT_SKILL };
            *out.entry(stat).or_insert(0) += base + trickle(weights, &attributes);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(worn(15) && worn(16) && !worn(49) && !worn(64));
    }
}
