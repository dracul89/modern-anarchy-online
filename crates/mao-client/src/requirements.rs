//! The criteria evaluator: Gamecode's FUN_100065d8 (called through FUN_1000960d by the items' `CanUse`/`CanWear`... checks,
//! vtable entries 0x58..0x68 of `DummyItemBase_t` at 0x10163a64, and by the spell runner FUN_100a5f4b), the one function every
//! requirement goes through: an item's actions (wear, use, wield ...), a perk's rank (`N3Msg_MeetsPerkCriteria` FUN_10052e48 →
//! vtable +0x5c, FUN_10081d5f, `CanWear`) and a spell's criteria.
//!
//! The criteria are `(stat, value, operator)` triples in postfix order. FUN_100065d8 keeps a stack of results (bytes at
//! ebp-0xa8, its depth at ebp-0xac): a comparison pushes true or false, and (4) and (3) or pop two and push one, (0x2a) not pops
//! one; one with too few operands (FUN_100052ae logs it) makes the whole list fail. At the end what is left is and-ed together;
//! an empty stack, or an empty list, passes. Operators outside its jump table (at 0x100093b0; 0x17, 0x19, 0x29, ..., and any
//! above 0x91) make the whole list fail at once.
//!
//! The comparisons read a stat of the "stat holder" (ebp-0x14, a `Beholder_t`: at first the one checked, the caller's), through
//! its vtable +0x3c `GetStat`. The holder is changed by the "apply on" operators: 0x13 (19) "On Self", 0x12 (18) "On Target",
//! 0x1a (26) "On User", 0x15 (21) "On Secondary Item", 0x6e (110) "On Fight Target" (text category 1005, key 1000 + operator)
//! through FUN_100021ff = FUN_10001fba with 1, 3, 2, 4 and 0xe; 0x64 (100) "On Caster" (the spell run's +0x10 identity); 0x25
//! (37) the same picker with the criterion's value, the holder itself when that finds nothing. When the holder cannot be found
//! the whole list fails (except for 37). FUN_10001fba has no case for 0xe, so "On Fight Target" always fails on the client.
//!
//! Whenever the holder's kind changes (ebp-0xb4 against ebp-0x118), ebp-0x10 is set to the holder's character when the holder
//! is one (vtable +0xc `GetType` 50000, the character being the holder less 0xe8) and ebp-0x15 to that character's +0x140, the
//! flag `SetMainDynel` (0x10019fdf) sets: the client's own character. The below / above comparisons (1, 2, and 0x82..0x85)
//! check only the client's own character; for anyone else (whose stats the client does not have) they pass.
//!
//! Ported here, each with the address of its case: see [`evaluate`]. The operators that read state the client does not keep
//! (timers, items in slots, teams, playfields, vehicles, organizations, positions and the rest) give
//! [`Unknown`] through the [`Context`]; the caller decides what to show for those.
//!
//! The evaluator's two flag arguments are not ported: ebp+0x18 (with it, equality on stats 4, 0x21, 0x3c, 0x170 compares the
//! level) and ebp+0x28 (with it, every criterion but the basic comparisons on stats 4, 0x21, 0x2f, 0x3b, 0x3c, 0x185 passes).
//! Both callers ported (the spell runner and the perk check) pass 0 for them. The feedback lines it writes when its feedback
//! flag (ebp+0x14) is set are not ported either (both callers pass 0).

/// A criterion: (stat, value, operator) as the templates keep them.
pub type Criterion = (u32, i32, u32);

/// Who the stats are read from: a dynel's identity (type, instance).
pub type Holder = (u32, u32);

/// The type of a character (`SimpleChar_t`, vtable +0xc `GetType`).
pub const CHARACTER: u32 = 50000;

/// What the evaluator could not tell, with the operator that needed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unknown {
    pub operator: u32,
}

/// What the evaluator reads from the world. Every answer may be `None`: the client does not know it.
pub trait Context {
    /// `GetStat` (vtable +0x3c) of the holder's stat.
    fn stat(&self, holder: Holder, stat: u32) -> Option<i64>;
    /// The holder is the client's own character (`SimpleChar_t` +0x140, set by `SetMainDynel`).
    fn is_own(&self, holder: Holder) -> bool;
    /// FUN_10001fba: the dynel the holder's picker `which` finds (1 itself, 2 its user, 3 its target, 4 its secondary item);
    /// `Some(None)` when it finds none.
    fn pick(&self, holder: Holder, which: i32) -> Option<Option<Holder>> {
        let _ = (holder, which);
        None
    }
    /// The spell run's +0x10 identity ("On Caster", operator 0x64): `None` when the evaluation is not a spell's.
    fn caster(&self) -> Option<Holder> {
        None
    }
    /// FUN_100051d8: the character has the perk (operators 0x5d and 0x67).
    fn has_perk(&self, character: Holder, perk: u32) -> Option<bool> {
        let _ = (character, perk);
        None
    }
}

/// FUN_100065d8 on `criteria`, the stats read from `holder`. `Err` when an operator needs what the context does not know.
pub fn evaluate(criteria: &[Criterion], holder: Holder, ctx: &dyn Context) -> Result<bool, Unknown> {
    if criteria.is_empty() {
        return Ok(true);
    }
    let mut stack: Vec<bool> = Vec::new();
    let mut holder = holder;
    // (ebp-0xb4 the holder's kind, ebp-0x118 the kind ebp-0x10 / ebp-0x15 were worked out for)
    let (mut kind, mut kind_seen) = (0u32, u32::MAX);
    let mut character: Option<Holder> = None;
    let mut own = false;
    for &(stat, value, op) in criteria {
        if kind_seen != kind {
            kind_seen = kind;
            character = (holder.0 == CHARACTER).then_some(holder);
            own = character.is_some() && ctx.is_own(holder);
        }
        let unknown = Unknown { operator: op };
        let get = |h: Holder| ctx.stat(h, stat).ok_or(unknown);
        let v = value as i64;
        let result: bool = match op {
            // (0x100068fa) equal
            0 => get(holder)? == v,
            // (0x10007065, 0x100071f7) below, above: the client's own character only
            1 => !own || get(holder)? < v,
            2 => !own || get(holder)? > v,
            // (0x10006f20) unequal
            0x18 => get(holder)? != v,
            // (0x1000717a) all the value's bits set; (0x10007129) none of them
            0x16 => get(holder)? & v == v,
            0x6b => get(holder)? & v == 0,
            // (0x100072f1, 0x10007332, 0x10007367) or, and, not
            3 | 4 => {
                let (Some(b), Some(a)) = (stack.pop(), stack.pop()) else { return Ok(false) };
                if op == 4 { a && b } else { a || b }
            }
            0x2a => {
                let Some(a) = stack.pop() else { return Ok(false) };
                !a
            }
            // (0x100067db, 0x100067ff, 0x10006847, 0x1000686b, 0x10006823) the apply-on operators: a new holder, nothing pushed
            0x12 | 0x1a | 0x15 | 0x13 | 0x6e => {
                let which = match op {
                    0x12 => 3,
                    0x1a => 2,
                    0x15 => 4,
                    0x13 => 1,
                    _ => 0xe,
                };
                match ctx.pick(holder, which).ok_or(unknown)? {
                    Some(h) => holder = h,
                    None => return Ok(false),
                }
                kind = op;
                continue;
            }
            // (0x1000688f) the picker the value names, else the holder stays (its kind does not change)
            0x25 => {
                if let Some(h) = ctx.pick(holder, value).ok_or(unknown)? {
                    holder = h;
                }
                continue;
            }
            // (0x100068ae) "On Caster": the run's caster
            0x64 => {
                match ctx.caster() {
                    Some(h) => holder = h,
                    None => return Ok(false),
                }
                kind = 0x64;
                continue;
            }
            // (0x10007ca7) stat 251's nibble (|value| / 1000 & 7) below |value| % 1000, or (a negative value) not below it
            0x42 => {
                let a = v.abs();
                let nibble = (ctx.stat(holder, 0xfb).ok_or(unknown)? >> ((a / 1000 & 7) * 4)) & 0xf;
                if v >= 0 { nibble < a % 1000 } else { nibble >= a % 1000 }
            }
            // (0x10008750) has the perk: needs a character; (0x100087c6) has not: passes without one
            0x5d => match character {
                Some(c) => ctx.has_perk(c, value as u32).ok_or(unknown)?,
                None => false,
            },
            0x67 => match character {
                Some(c) => !ctx.has_perk(c, value as u32).ok_or(unknown)?,
                None => true,
            },
            // (0x100066f3) always true on the client: 0x2d, 0x49, 0x4a, 0x62, 0x68, 0x6c (108), 0x6d, 0x73..0x76, 0x7f, 0x80,
            // 0x86..0x88
            0x2d | 0x49 | 0x4a | 0x62 | 0x68 | 0x6c | 0x6d | 0x73..=0x76 | 0x7f | 0x80 | 0x86..=0x88 => true,
            // (0x100072de) always false: 0x41, 0x63, 0x71, 0x7b
            0x41 | 0x63 | 0x71 | 0x7b => false,
            // (0x10009267) not in the table: the list fails
            0x17 | 0x19 | 0x29 | 0x2b | 0x2e..=0x31 | 0x33 | 0x34 | 0x3a | 0x3b | 0x3d..=0x40 | 0x48 | 0x4b..=0x4f | 0x54 | 0x56 | 0x57 | 0x5a | 0x5f | 0x60 | 0x89 => return Ok(false),
            _ if op > 0x91 => return Ok(false),
            // (the rest read what the context does not give: times, slots, items, teams, organizations, places ...)
            _ => return Err(unknown),
        };
        stack.push(result);
    }
    Ok(stack.into_iter().all(|b| b))
}

/// The client's own character's view: its stats and perks; every other dynel's stats are unknown.
pub struct Own<'a> {
    pub id: Holder,
    pub stats: &'a std::collections::BTreeMap<u32, i64>,
    pub perks: &'a std::collections::HashSet<u32>,
}

impl Context for Own<'_> {
    fn stat(&self, holder: Holder, stat: u32) -> Option<i64> {
        // (a stat the character does not have reads as 0 here, as the client's stat map holds only what the server sent)
        (holder == self.id).then(|| self.stats.get(&stat).copied().unwrap_or(0))
    }

    fn is_own(&self, holder: Holder) -> bool {
        holder == self.id
    }

    fn pick(&self, holder: Holder, which: i32) -> Option<Option<Holder>> {
        // (FUN_10001fba on a character: 1 the holder itself; 2 its user (`Beholder_t` +0x3c), 3 its target and 4 its secondary
        // item, not known here; anything else nothing)
        match which {
            1 => Some(Some(holder)),
            2..=4 => None,
            _ => Some(None),
        }
    }

    fn has_perk(&self, character: Holder, perk: u32) -> Option<bool> {
        (character == self.id).then(|| self.perks.contains(&perk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashSet};

    const ME: Holder = (CHARACTER, 7);

    fn own<'a>(stats: &'a BTreeMap<u32, i64>, perks: &'a HashSet<u32>) -> Own<'a> {
        Own { id: ME, stats, perks }
    }

    #[test]
    fn comparisons_and_logic() {
        let stats: BTreeMap<u32, i64> = [(54, 20), (60, 7), (389, 0x2c)].into_iter().collect();
        let perks: HashSet<u32> = [100].into_iter().collect();
        let c = own(&stats, &perks);
        // (Accumulator rank 2: profession 7, level above 19, has perk 100)
        assert_eq!(evaluate(&[(60, 7, 0), (54, 19, 2), (0, 0, 4), (0, 100, 93), (0, 0, 4)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(60, 7, 0), (54, 20, 2), (0, 0, 4)], ME, &c), Ok(false));
        assert_eq!(evaluate(&[(0, 101, 93)], ME, &c), Ok(false));
        assert_eq!(evaluate(&[(0, 101, 103)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(54, 21, 1), (60, 6, 0), (0, 0, 3)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(54, 20, 24), (0, 0, 42)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(389, 0x24, 22)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(389, 0x02, 107)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(389, 0x04, 107)], ME, &c), Ok(false));
        // (108 always passes on the client; 0x63 always fails; 0x56 is outside the table)
        assert_eq!(evaluate(&[(0, 256081, 108)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(0, 0, 99)], ME, &c), Ok(false));
        assert_eq!(evaluate(&[(0, 0, 86)], ME, &c), Ok(false));
        // (left on the stack: and-ed; an operator short of operands fails the list)
        assert_eq!(evaluate(&[(60, 7, 0), (54, 30, 2)], ME, &c), Ok(false));
        assert_eq!(evaluate(&[(60, 7, 0), (0, 0, 4)], ME, &c), Ok(false));
        assert_eq!(evaluate(&[], ME, &c), Ok(true));
        // (a timer operator: unknown here)
        assert_eq!(evaluate(&[(0, 5, 5)], ME, &c), Err(Unknown { operator: 5 }));
    }

    #[test]
    fn below_and_above_pass_for_others() {
        struct Npc;
        impl Context for Npc {
            fn stat(&self, _: Holder, _: u32) -> Option<i64> {
                None
            }
            fn is_own(&self, _: Holder) -> bool {
                false
            }
            fn caster(&self) -> Option<Holder> {
                Some((CHARACTER, 9))
            }
        }
        // (the Shadowland Anvian's sparks: On Caster, stat 410 above 0: the caster is not the client's character)
        assert_eq!(evaluate(&[(0, 0, 100), (410, 0, 2)], (CHARACTER, 9), &Npc), Ok(true));
        // (equality reads the stat whoever it is)
        assert_eq!(evaluate(&[(589, 6553, 0)], (CHARACTER, 9), &Npc), Err(Unknown { operator: 0 }));
        // (no caster: the list fails)
        struct NoCaster;
        impl Context for NoCaster {
            fn stat(&self, _: Holder, _: u32) -> Option<i64> {
                None
            }
            fn is_own(&self, _: Holder) -> bool {
                false
            }
        }
        assert_eq!(evaluate(&[(0, 0, 100), (410, 0, 2)], (CHARACTER, 9), &NoCaster), Ok(false));
    }

    #[test]
    fn apply_on_changes_the_holder() {
        let stats: BTreeMap<u32, i64> = [(251, 0x0310)].into_iter().collect();
        let perks = HashSet::new();
        let c = own(&stats, &perks);
        // (On Self keeps the character; On Fight Target finds nothing on the client, so the list fails)
        assert_eq!(evaluate(&[(0, 0, 19), (251, 0x0310, 0)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(0, 0, 110), (525, 16, 1)], ME, &c), Ok(false));
        // (On Target: who the character targets is not known here)
        assert_eq!(evaluate(&[(0, 0, 18), (204, 30, 2)], ME, &c), Err(Unknown { operator: 18 }));
        // (operator 66 on stat 251: nibble 0 is 0 (below 1), nibble 1 is 1 (not below 1), nibble 2 is 3 (below 4))
        assert_eq!(evaluate(&[(251, 1, 66)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(251, 1001, 66)], ME, &c), Ok(false));
        assert_eq!(evaluate(&[(251, -1001, 66)], ME, &c), Ok(true));
        assert_eq!(evaluate(&[(251, 2004, 66)], ME, &c), Ok(true));
    }

    /// `MAO_GAME`: real items' criteria. The Assault-Issue Omni-Pol Armor Sleeves (21955): to wear, Psychic (18) above 540,
    /// Strength (16) above 490 and side (33) 2. The Engineer Robot Shell (43328): to use, profession 3, two skills above 7 and
    /// stat 251's lowest nibble below 1 (operator 66). The Simple First-Aid Kit (23314): to use, stat 123 above 7, none of
    /// stat 397's flag 16 (operator 107), stat 531 at 0.
    #[test]
    fn real_item_criteria() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let actions = |t: u32| mao_formats::item::actions(&db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: t }).unwrap().unwrap());
        let criteria = |t: u32, kind: u32| actions(t).into_iter().find(|a| a.kind == kind).unwrap().criteria;
        let perks = HashSet::new();
        let sleeves = criteria(21955, 6);
        assert_eq!(sleeves, vec![(18, 540, 2), (16, 490, 2), (0, 0, 4), (33, 2, 0), (0, 0, 4)]);
        let mut stats: BTreeMap<u32, i64> = [(18, 541), (16, 491), (33, 2)].into_iter().collect();
        assert_eq!(evaluate(&sleeves, ME, &own(&stats, &perks)), Ok(true));
        stats.insert(16, 490);
        assert_eq!(evaluate(&sleeves, ME, &own(&stats, &perks)), Ok(false));
        let shell = criteria(43328, 3);
        let mut stats: BTreeMap<u32, i64> = [(60, 3), (130, 8), (131, 8), (251, 0x10)].into_iter().collect();
        assert_eq!(evaluate(&shell, ME, &own(&stats, &perks)), Ok(true));
        stats.insert(251, 0x11);
        assert_eq!(evaluate(&shell, ME, &own(&stats, &perks)), Ok(false));
        let kit = criteria(23314, 3);
        assert_eq!(kit, vec![(123, 7, 2), (397, 16, 107), (0, 0, 4), (531, 0, 0), (0, 0, 4)]);
        let mut stats: BTreeMap<u32, i64> = [(123, 8), (397, 1)].into_iter().collect();
        assert_eq!(evaluate(&kit, ME, &own(&stats, &perks)), Ok(true));
        stats.insert(397, 17);
        assert_eq!(evaluate(&kit, ME, &own(&stats, &perks)), Ok(false));
    }
}
