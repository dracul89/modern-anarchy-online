//! Item templates: the objects of type 0xf4254, one per item id (the low or
//! high id of an inventory entry). Little-endian words: the type, the
//! instance, 1000000, the identity type (0xc74e), then (stat, value) pairs,
//! then a word holding the name's length (low half) and the description's
//! (high half), then the two strings. Among the stats: 79 is the icon (a
//! 48x48 texture of type 0xf6958, by instance) and 12 the item's mesh (type
//! 0xf6952).

/// The object type of an item template.
pub const TEMPLATE: u32 = 0xf4254;
/// The database type of nano programs: the same record shape as an item's.
pub const NANO_TEMPLATE: u32 = 0xfde85;
/// The texture type of an item's icon.
pub const ICON_TYPE: u32 = 0xf6958;

/// The stat keys this client reads.
pub mod stat {
    pub const MESH: u32 = 12;
    pub const ICON: u32 = 79;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub description: String,
    /// (stat, value) pairs as stored.
    pub stats: Vec<(u32, u32)>,
}

/// The "modify a stat" effects in an item template's trailing tables: `(stat, amount)`. In the tables each such
/// spell (type 0xcf35) is ten little-endian words, `0xcf35, 0, 4, 0, 1, 0, 2, 9, stat, amount`, found in the
/// events that apply when the item is worn. Read from the items' templates and checked against the original's
/// `N3Msg_GetSkill` (see `docs/STAT_ENGINE.md`): the sleeves "Strong Armor Sleeves" give 230 melee AC, 250
/// projectile AC and so on. Effects of other spell types (and their requirements) are not read.
pub fn modifiers(template: &[u8]) -> Vec<(u32, i32)> {
    // The tables follow the text, so they are not aligned to the record's words: look at every byte offset.
    const HEAD: [u32; 8] = [0xcf35, 0, 4, 0, 1, 0, 2, 9];
    let mut head = Vec::with_capacity(32);
    for w in HEAD {
        head.extend_from_slice(&w.to_le_bytes());
    }
    let mut out = Vec::new();
    let mut at = 0;
    while at + 40 <= template.len() {
        if template[at..at + 32] == head[..] {
            let word = |o: usize| u32::from_le_bytes(template[at + o..at + o + 4].try_into().unwrap());
            out.push((word(32), word(36) as i32));
            at += 40;
        } else {
            at += 1;
        }
    }
    out
}

/// Like [`modifiers`], but also the records with requirements, `0xcf35, 0, 4, N, (stat, value, operator) x N, 1, 0, 2, 9, stat,
/// amount`, kept when every requirement holds for `level` (the only requirement stat seen is the level, stat 0x36; operator 1 is
/// "below" and 2 is "above" (strictly: the operators are the original's `LessThan` and `GreaterThan`), from the two records of
/// item template 281199: +200 below level 201, +500 above 200).
/// Requirements on other stats, or with other operators, are not known to hold, so those records are left out.
pub fn modifiers_at(template: &[u8], level: i64) -> Vec<(u32, i32)> {
    modifiers_with(template, level, &[2, 3])
}

/// The same for a nano program: its buffs carry the third word of the record's tail (`1, 0, x, 9`) as 1, 2 or 3 (checked
/// against the live client: 2 and 3 for 51 effects of 8 buffs, 1 for Mongo's Kraken's +20000 health).
pub fn nano_modifiers_at(template: &[u8], level: i64) -> Vec<(u32, i32)> {
    modifiers_with(template, level, &[1, 2, 3])
}

fn modifiers_with(template: &[u8], level: i64, tails: &[u32]) -> Vec<(u32, i32)> {
    const FLAT: u32 = 0xcf35;
    /// Spell type 0xcf14 (53012): a second flat modify-stat form, seen in the weapon-enhancement nanos (stats 278-282, 311, 317 +5), whose
    /// effects appear in the character's modifier map like 0xcf35's.
    const FLAT_B: u32 = 0xcf14;
    /// Spell type 0xcfb7: the amount is the level times the spell's value over 200, cut toward zero (`FUN_100a8d0b`; the
    /// original's own spell records show it: 16 at level 220 gives 17, 36 gives 39, -6 gives -6).
    const SCALED: u32 = 0xcfb7;
    let word = |at: usize| u32::from_le_bytes(template[at..at + 4].try_into().unwrap());
    let mut out = Vec::new();
    let mut at = 0;
    while at + 40 <= template.len() {
        if (word(at) == FLAT || word(at) == FLAT_B || word(at) == SCALED) && word(at + 4) == 0 && word(at + 8) == 4 && word(at + 12) < 8 {
            let n = word(at + 12) as usize;
            let tail = at + 16 + 12 * n;
            if tail + 24 <= template.len() && [word(tail), word(tail + 4), word(tail + 12)] == [1, 0, 9] && tails.contains(&word(tail + 8)) {
                let holds = (0..n).all(|i| {
                    let (stat, value, op) = (word(at + 16 + 12 * i), word(at + 20 + 12 * i) as i64, word(at + 24 + 12 * i));
                    stat == 0x36 && match op {
                        1 => level < value,
                        2 => level > value,
                        _ => false,
                    }
                });
                if holds {
                    let value = word(tail + 20) as i32;
                    let amount = if word(at) == SCALED { (level as f64 * value as f64 / 200.0).trunc() as i32 } else { value };
                    out.push((word(tail + 16), amount));
                }
                at = tail + 24;
                continue;
            }
        }
        at += 1;
    }
    out
}

/// The nano line (stat `NanoStrain`) of the polymorph programs.
pub const POLYMORPH_STRAIN: u32 = 223;

/// The creature a polymorph program turns its target into: the argument of its `ChangeMesh` spell (type `0xcf44`), an NPC template
/// (type `0xfde97`). Spell records read `type, 0, 4, n, n criteria of three words, 1, 0, x, 9`, then the arguments.
pub fn polymorph_template(template: &[u8]) -> Option<u32> {
    let word = |at: usize| u32::from_le_bytes(template[at..at + 4].try_into().unwrap());
    (0..template.len().saturating_sub(40)).find_map(|at| {
        if word(at) != 0xcf44 || word(at + 4) != 0 || word(at + 8) != 4 || word(at + 12) >= 4 {
            return None;
        }
        let tail = at + 16 + 12 * word(at + 12) as usize;
        (tail + 20 <= template.len() && word(tail) == 1 && word(tail + 12) == 9).then(|| word(tail + 16))
    })
}

/// The body a vehicle program's spells are tested against: the stats its `ChangeMesh` criteria read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    /// Stat 4 `Breed`, stat 59 `Sex` and stat 12 `Mesh` (the body's CAT mesh instance).
    pub breed: u32,
    pub sex: u32,
    pub mesh: u32,
}

/// The `ChangeMesh` spells (type `0xcf44`) of a program, as (criteria, NPC template): spell records read `type, 0, 4, n, n
/// criteria (stat, value, operator), 1, 0, x, 9`, then the arguments, the template first.
fn change_mesh_spells(template: &[u8]) -> Vec<(Vec<(u32, u32, u32)>, u32)> {
    let word = |at: usize| u32::from_le_bytes(template[at..at + 4].try_into().unwrap());
    (0..template.len().saturating_sub(40))
        .filter_map(|at| {
            if word(at) != 0xcf44 || word(at + 4) != 0 || word(at + 8) != 4 || word(at + 12) >= 16 {
                return None;
            }
            let n = word(at + 12) as usize;
            let tail = at + 16 + 12 * n;
            if tail + 20 > template.len() || word(tail) != 1 || word(tail + 12) != 9 {
                return None;
            }
            Some(((0..n).map(|i| (word(at + 16 + 12 * i), word(at + 20 + 12 * i), word(at + 24 + 12 * i))).collect(), word(tail + 16)))
        })
        .collect()
}

/// Whether a body meets a spell's criteria, read in reverse Polish order as the item actions' are (see [`Action`]): a comparison
/// (operator 0, equal) pushes its result, operator 3 ORs and 4 ANDs the two before it. Stats other than the body's are not known
/// and compare false (stat 359 `MonsterData` among them: the body is not on the vehicle yet).
fn body_meets(criteria: &[(u32, u32, u32)], body: &Body) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    for &(stat, value, operator) in criteria {
        match operator {
            3 | 4 if stat == 0 => {
                let (Some(b), Some(a)) = (stack.pop(), stack.pop()) else { return false };
                stack.push(if operator == 3 { a || b } else { a && b });
            }
            0 => stack.push(match stat {
                4 => body.breed == value,
                59 => body.sex == value,
                12 => body.mesh == value,
                _ => false,
            }),
            _ => return false,
        }
    }
    stack.len() == 1 && stack[0]
}

/// The vehicle a program puts a body on: the NPC template (type `0xfde97`) of its `ChangeMesh` spell whose criteria the body
/// meets. The Phasefront programs carry one such spell per body. The hoverboards (270431, 270327, 284003) key them on stat 12
/// `Mesh`: the thin, the plain or the fat mesh of a breed and sex (robed or not), ORed, or stat 359 `MonsterData` already the
/// template; the hoverbikes (270836) on stat 4 `Breed` AND stat 59 `Sex`. The template's mesh is the rider and vehicle as one
/// CAT body (`Hoverboard_b_male.cir`, `Hoverbike_a_atrox_t9.cir`). The server applies the spell by setting `MonsterData`, and the
/// client then takes the template's mesh and animations (FUN_1005a3b1's case 0x167 → FUN_10052686 → FUN_10052271).
pub fn vehicle_template(template: &[u8], body: &Body) -> Option<u32> {
    change_mesh_spells(template).into_iter().find(|(criteria, _)| body_meets(criteria, body)).map(|(_, t)| t)
}

/// Whether a program changes its target's mesh by its body (see [`vehicle_template`]): one of its `ChangeMesh` spells has
/// criteria on the body (`Mesh`, `Breed` or `Sex`). A polymorph's has none.
pub fn is_vehicle_program(template: &[u8]) -> bool {
    change_mesh_spells(template).iter().any(|(criteria, _)| criteria.iter().any(|c| matches!(c.0, 4 | 12 | 59) && c.2 == 0))
}

/// What an item's action needs: the item's trailing tables hold, after each event's spells, the marker `22, 36, 2018`, the action
/// (3 = use, 6 = wear, 8 = wield) and a criteria list: `(n + 1) * 0x3f1`, then `n` criteria `(stat, value, operator)` in reverse
/// Polish order (a comparison pushes its result, operator 4 is AND and 3 is OR of the two before it). Operators: 0 equal, 1 less than,
/// 2 greater than (strict: a requirement of level 215 is stored as "level > 214"), 24 unequal, 22 bit-and; 93 is "has the perk".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub kind: u32,
    pub criteria: Vec<(u32, i32, u32)>,
}

/// A class-to-values set of an item template (Gamecode FUN_1002b458 reads the chunks: `key, sub`, then for keys 0x0e and 0x12
/// the animation set into the item's +0x78 map, FUN_1007dae8, and for key 0x14 the sound set into +0x88, FUN_1007dc1e; both
/// `(n + 1) × 1009`, then n sets of `class, (k + 1) × 1009` and k words). The first chunk with `key`: `(class, words)`.
pub fn class_sets(template: &[u8], key: u32) -> Vec<(u32, Vec<u32>)> {
    let word = |at: usize| template.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let mut at = 0;
    while at + 12 <= template.len() {
        if word(at) == Some(key) && word(at + 4).is_some_and(|s| s <= 0x36) && word(at + 8).is_some_and(|c| c % 1009 == 0 && (2..=41).contains(&(c / 1009))) {
            let n = (word(at + 8).unwrap() / 1009 - 1) as usize;
            let mut sets = Vec::new();
            let mut p = at + 12;
            for _ in 0..n {
                let (Some(class), Some(count)) = (word(p), word(p + 4)) else { return sets };
                if count % 1009 != 0 || !(1..=41).contains(&(count / 1009)) {
                    return sets;
                }
                let k = (count / 1009 - 1) as usize;
                let values: Option<Vec<u32>> = (0..k).map(|j| word(p + 8 + 4 * j)).collect();
                let Some(values) = values else { return sets };
                sets.push((class, values));
                p += 8 + 4 * k;
            }
            return sets;
        }
        at += 1;
    }
    Vec::new()
}

/// An item template's animation set (chunk 0x0e or 0x12): `(class, actions)`; class 0xb is the attack's.
pub fn anim_sets(template: &[u8]) -> Vec<(u32, Vec<u32>)> {
    let sets = class_sets(template, 0x12);
    if sets.is_empty() { class_sets(template, 0x0e) } else { sets }
}

/// What a weapon in place `slot` swings (class 0xb of the item's animation map, which `FUN_10045976` picks from at random): when
/// it is wielded `WeaponItem_t` (FUN_1009e84b) fills the map by its AnimSet stat (353) as FUN_1009d964 does: 0 small arms
/// (`smallarms-shot`, `-shotl` in the left hand, place 8), 1 a one-handed blade (slash, stab, slash; the left hand's), 2 a
/// two-handed blade (chop, downcut, slash, stab), 3 a rifle, 6 a bow, 7 `spell-sys`, 8 a bazooka; other sets keep what the
/// template's own animation set gives (the martial arts items' and the monsters' innate weapons' swings), as does an item
/// without the stat (taken as unset: FUN_1009d964's stat read was not traced to its default). The crawling variants (the
/// owner's stat 430 at 14) are not used.
pub fn attack_actions(template: &[u8], slot: u32) -> Vec<u32> {
    let left = slot == 8;
    let set = Item::parse(template).and_then(|i| i.stat(353));
    match set {
        Some(0) => vec![if left { 0x3f8 } else { 0x3f5 }],
        Some(1) => if left { vec![0x3ee, 0x3ef, 0x3ee] } else { vec![0x3eb, 0x3ec, 0x3eb] },
        Some(2) => vec![0x41a, 0x41b, 0x41c, 0x41d],
        Some(3) => vec![0x3ff],
        Some(6) => vec![0xb7],
        Some(7) => vec![0xcb],
        Some(8) => vec![0x426],
        _ => anim_sets(template).into_iter().filter(|(class, _)| *class == 0xb).flat_map(|(_, actions)| actions).collect(),
    }
}

pub fn actions(template: &[u8]) -> Vec<Action> {
    let word = |at: usize| u32::from_le_bytes(template[at..at + 4].try_into().unwrap());
    let mut out = Vec::new();
    let mut at = 0;
    while at + 20 <= template.len() {
        if [word(at), word(at + 4), word(at + 8)] == [22, 36, 2018] {
            let (kind, count) = (word(at + 12), word(at + 16));
            if count % 0x3f1 == 0 && (2..=41).contains(&(count / 0x3f1)) {
                let n = (count / 0x3f1 - 1) as usize;
                if at + 20 + 12 * n <= template.len() {
                    let criteria = (0..n).map(|k| (word(at + 20 + 12 * k), word(at + 24 + 12 * k) as i32, word(at + 28 + 12 * k))).collect();
                    out.push(Action { kind, criteria });
                    at += 20 + 12 * n;
                    continue;
                }
            }
        }
        at += 1;
    }
    out
}

impl Item {
    pub fn parse(data: &[u8]) -> Option<Item> {
        let words: Vec<u32> = data.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
        if words.len() < 6 || (words[0] != TEMPLATE && words[0] != NANO_TEMPLATE) {
            return None;
        }
        let printable = |bytes: &[u8]| bytes.iter().all(|&b| b >= 0x20 && b != 0x7f || matches!(b, 9 | 10 | 13));
        // The length word is the first one, after the stats, whose two halves
        // measure printable text that fills the rest of the record (the
        // description and the name; then the trailing tables).
        // The stats are pairs from word 4, so the length word sits at an even index.
        for at in (6..words.len()).step_by(2) {
            let (name_len, description_len) = ((words[at] & 0xffff) as usize, (words[at] >> 16) as usize);
            let start = (at + 1) * 4;
            if name_len < 2 || name_len > 80 || description_len > 4000 || start + name_len + description_len > data.len() {
                continue;
            }
            let name = &data[start..start + name_len];
            let description = &data[start + name_len..start + name_len + description_len];
            if printable(name) && printable(description) && name.iter().any(|b| b.is_ascii_alphanumeric()) {
                let stats = words[4..at].chunks_exact(2).map(|p| (p[0], p[1])).collect();
                return Some(Item { id: words[1], name: String::from_utf8_lossy(name).into_owned(), description: String::from_utf8_lossy(description).into_owned(), stats });
            }
        }
        None
    }

    /// The icon texture's instance.
    pub fn icon(&self) -> Option<u32> {
        self.stat(stat::ICON)
    }

    pub fn stat(&self, key: u32) -> Option<u32> {
        self.stats.iter().find(|s| s.0 == key).map(|s| s.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_mesh_spell_names_the_creature() {
        // "Calia's Form: Wolf": cf44, 0, 4, 0 criteria, 1, 0, 2, 9, 0x768c
        let words = [0xcf35u32, 0, 4, 0, 1, 0, 2, 9, 5, 5, 0xcf44, 0, 4, 0, 1, 0, 2, 9, 0x768c, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(polymorph_template(&bytes), Some(0x768c));
        assert_eq!(polymorph_template(&bytes[..40]), None);
    }

    #[test]
    fn a_vehicle_spell_is_keyed_on_the_body() {
        // Phasefront Phantom - Beige (270431), its first two body spells: Mesh 17534 or 5907 or 17532 (or MonsterData already the
        // board) gives template 270044; the robed meshes 272944.
        let spell = |meshes: [u32; 3], board: u32| {
            let mut w = vec![0xcf44u32, 0, 4, 7, 12, meshes[0], 0, 12, meshes[1], 0, 0, 0, 3, 12, meshes[2], 0, 0, 0, 3, 359, board, 0, 0, 0, 3];
            w.extend([1, 0, 1, 9, board, 0]);
            w
        };
        let mut words = vec![0xcf35u32, 0, 4, 0, 1, 0, 1, 9, 156, 150];
        words.extend(spell([17534, 5907, 17532], 270044));
        words.extend(spell([17872, 17870, 17867], 272944));
        words.extend([0; 10]);
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let body = |mesh: u32| Body { breed: 1, sex: 2, mesh };
        assert_eq!(vehicle_template(&bytes, &body(5907)), Some(270044));
        assert_eq!(vehicle_template(&bytes, &body(17867)), Some(272944));
        assert_eq!(vehicle_template(&bytes, &body(5927)), None);
        assert!(is_vehicle_program(&bytes));
        // Phasefront Spectre (270836), a hoverbike: Breed and Sex ANDed
        let bike = |breed: u32, sex: u32, t: u32| vec![0xcf44u32, 0, 4, 3, 4, breed, 0, 59, sex, 0, 0, 0, 4, 1, 0, 2, 9, t, 0];
        let mut words = bike(1, 2, 270812);
        words.extend(bike(4, 1, 270823));
        words.extend([0; 10]);
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(vehicle_template(&bytes, &Body { breed: 4, sex: 1, mesh: 5900 }), Some(270823));
        assert_eq!(vehicle_template(&bytes, &Body { breed: 1, sex: 3, mesh: 5927 }), None);
        // (a polymorph keys nothing on the body)
        let calia: Vec<u8> = [0xcf44u32, 0, 4, 0, 1, 0, 2, 9, 0x768c, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].iter().flat_map(|w| w.to_le_bytes()).collect();
        assert!(!is_vehicle_program(&calia));
    }

    /// Item 0x4321c, "Dream Mesh Circuit", as the game's database holds it.
    fn circuit() -> Vec<u8> {
        let mut words: Vec<u32> = vec![TEMPLATE, 0x4321c, 1_000_000, 0xc74e, 4, 0xf, 0x17, 0x333d, 0x4a, 1, 0x4f, 0x249b2, 0xd3, 0x1b58, 0x15, 0x21];
        let (name, description) = ("Dream Mesh Circuit", "This circuit board is woven with the pulsing ether.");
        words.push(name.len() as u32 | (description.len() as u32) << 16);
        let mut bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(description.as_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }

    #[test]
    fn name_description_and_icon() {
        let item = Item::parse(&circuit()).unwrap();
        assert_eq!(item.id, 0x4321c);
        assert_eq!(item.name, "Dream Mesh Circuit");
        assert!(item.description.starts_with("This circuit board"));
        assert_eq!(item.icon(), Some(0x249b2));
    }

    #[test]
    fn other_objects_are_refused() {
        assert_eq!(Item::parse(&[0; 64]), None);
    }

    #[test]
    fn the_wear_effects_are_found() {
        // Two of the sleeves' effects: +230 melee AC (stat 0x5b) and +250 projectile AC (0x5a).
        let words: Vec<u32> = vec![2, 0xe, 0x8211, 0xcf35, 0, 4, 0, 1, 0, 2, 9, 0x5b, 0xe6, 0xcf35, 0, 4, 0, 1, 0, 2, 9, 0x5a, 0xfa, 0xcf2f];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(modifiers(&bytes), vec![(0x5b, 230), (0x5a, 250)]);
    }

    #[test]
    fn effects_with_a_level_requirement() {
        // Item 281199: +500 nano resist (stat 168) from level 200 (operator 2), +200 below level 201 (operator 1).
        let words: Vec<u32> = vec![7, 0xcf35, 0, 4, 1, 0x36, 200, 2, 1, 0, 2, 9, 168, 500, 0xcf35, 0, 4, 1, 0x36, 201, 1, 1, 0, 2, 9, 168, 200, 0xcf35, 0, 4, 0, 1, 0, 2, 9, 0x5b, 7];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(modifiers_at(&bytes, 220), vec![(168, 500), (0x5b, 7)]);
        assert_eq!(modifiers_at(&bytes, 150), vec![(168, 200), (0x5b, 7)]);
        // the comparisons are strict: "above 200" is level 201 and up, "below 201" is level 200 and down
        assert_eq!(modifiers_at(&bytes, 200), vec![(168, 200), (0x5b, 7)]);
        assert_eq!(modifiers_at(&bytes, 201), vec![(168, 500), (0x5b, 7)]);
    }

    #[test]
    fn level_scaled_effects() {
        // "Coalescing Energy Cascade": 0xcfb7 +1000 percent-ish on max health: 1100 at level 220, plus a flat +10 strength.
        let words: Vec<u32> = vec![0xcfb7, 0, 4, 0, 1, 0, 2, 9, 1, 1000, 0xcf35, 0, 4, 0, 1, 0, 2, 9, 16, 10];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(modifiers_at(&bytes, 220), vec![(1, 1100), (16, 10)]);
        assert_eq!(modifiers_at(&bytes, 25), vec![(1, 125), (16, 10)]);
        // cut toward zero, as the original's records show: values 16, 36, 54, -6 at level 220 give 17, 39, 59, -6
        let words: Vec<u32> = vec![0xcfb7, 0, 4, 0, 1, 0, 2, 9, 123, 16, 0xcfb7, 0, 4, 0, 1, 0, 2, 9, 122, 36, 0xcfb7, 0, 4, 0, 1, 0, 2, 9, 139, 54, 0xcfb7, 0, 4, 0, 1, 0, 2, 9, 318, (-6i32) as u32];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(modifiers_at(&bytes, 220), vec![(123, 17), (122, 39), (139, 59), (318, -6)]);
    }

    #[test]
    fn the_second_flat_form_counts() {
        // "Weapon Enhancement": 0xcf14 records (stat 278, +5) with the tail's third word 3
        let words: Vec<u32> = vec![0xcf14, 0, 4, 0, 1, 0, 3, 9, 278, 5, 0, 0xcf14, 0, 4, 0, 1, 0, 3, 9, 279, 5, 0];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(nano_modifiers_at(&bytes, 220), vec![(278, 5), (279, 5)]);
    }

    #[test]
    fn the_wear_requirements_are_read() {
        // Carbonum Breastplate at its lowest quality: wear needs stamina above 10 and agility above 7 (AND).
        let words: Vec<u32> = vec![53045, 0, 4, 0, 1, 0, 2, 9, 97, 8, 22, 36, 2018, 6, 4 * 0x3f1, 18, 10, 2, 17, 7, 2, 0, 0, 4];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(actions(&bytes), vec![Action { kind: 6, criteria: vec![(18, 10, 2), (17, 7, 2), (0, 0, 4)] }]);
    }

    #[test]
    fn nano_buffs_use_either_tail() {
        // "Composite Attribute Boost": +12 strength (stat 16) with the tail's third word 3; an item's wear effect uses 2.
        let words: Vec<u32> = vec![0xcf35, 0, 4, 0, 1, 0, 3, 9, 16, 12, 0xcf35, 0, 4, 0, 1, 0, 2, 9, 18, 5];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(nano_modifiers_at(&bytes, 25), vec![(16, 12), (18, 5)]);
        assert_eq!(modifiers_at(&bytes, 25), vec![(16, 12), (18, 5)]);
    }
}
