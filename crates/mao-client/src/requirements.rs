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
//! Both callers ported (the spell runner and the perk check) pass 0 for them.
//!
//! With its feedback flag (ebp+0x14) set, as the items' wear and wield checks set it for a player's own move, a
//! comparison that fails leaves a line in a list (ebp-0x24), and when the whole list has failed and the one judged is
//! the client's own character the lines are told ([`evaluate_told`], [`Told`]). Ported: below and above (0x10007065,
//! 0x100071f7) and the equalities of breed, side, sex and profession (0x100068fa on). The other operators' lines
//! (targets, teams, running nanos, perks and the rest) are not: such a failure says nothing here.

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
    /// The character is in fly mode (its movement mode 7: operator 0x46).
    fn flying(&self, character: Holder) -> Option<bool> {
        let _ = character;
        None
    }
    /// The character's playfield does not allow flying (bit 0x80000 of the playfield record's third flag word:
    /// operator 0x70).
    fn flying_forbidden(&self, character: Holder) -> Option<bool> {
        let _ = character;
        None
    }
    /// FUN_100051d8: the character has the perk (operators 0x5d and 0x67).
    fn has_perk(&self, character: Holder, perk: u32) -> Option<bool> {
        let _ = (character, perk);
        None
    }
    /// The server's time (`GameTime_t` +0xb4: operators 5 and 6).
    fn server_time(&self) -> Option<u32> {
        None
    }
    /// FUN_10047390 on the character's inventory (+0x1b8): one of its wear places 0 to 63 on the page `page` (FUN_10046fb7:
    /// 7 the weapons' places 0-15, 3 the clothes' 16-31, 8 the implants' 32-47, 14 the social 48-63) holds an item of the
    /// template `template` (the first word of the place's item key, +0xc: operators 0x1f to 0x22).
    fn wears(&self, character: Holder, template: u32, page: u32) -> Option<bool> {
        let _ = (character, template, page);
        None
    }
    /// FUN_1004f045 on the character's nano object (+0x1c0, its list at +0x14): the character has the nano program
    /// (operators 0x23 and 0x24).
    fn knows_nano(&self, character: Holder, nano: u32) -> Option<bool> {
        let _ = (character, nano);
        None
    }
    /// FUN_1004f0d7 on the character's nano object (its list at +0x20): the nano program runs on the character
    /// (operators 0x5b and 0x65).
    fn runs_nano(&self, character: Holder, nano: u32) -> Option<bool> {
        let _ = (character, nano);
        None
    }
    /// FUN_1004f7ed: a nano program running on the character is of the strain (its stat 75 or one of 546 to 550,
    /// FUN_10051e9e: operators 0x5c and 0x66).
    fn runs_strain(&self, character: Holder, strain: u32) -> Option<bool> {
        let _ = (character, strain);
        None
    }
    /// FUN_10052eb2 on the character's perk object (+0x1f4, FUN_10058ff8; its list at +8, which `PerkUnavailable` fills,
    /// FUN_10053b5d): the perk lock runs (operator 0x61).
    fn perk_locked(&self, character: Holder, lock: u32) -> Option<bool> {
        let _ = (character, lock);
        None
    }
}

/// What a failed comparison says when the evaluator is asked to tell (its feedback flag), each a text of the feedback
/// category fed as FUN_100065d8 feeds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Told {
    /// `Feedback_IsRequiredToBeAtLeast` (above, 0x100071f7): whose ("Target" after On Target, else "Your": text
    /// category 2007), the stat (its name, category 2002) and the least value that passes, the criterion's plus one.
    AtLeast { target: bool, stat: u32, value: i64 },
    /// `Feedback_IsNotLessThan` (below, 0x10007065): whose, the stat and the criterion's value.
    NotBelow { target: bool, stat: u32, value: i64 },
    /// `Feedback_TargetBreedMustBe` (stat 4: text category 1005, the value and 600).
    Breed(i64),
    /// `Feedback_TargetSideMustBe` (stat 0x21: category 2005).
    Side(i64),
    /// `Feedback_TargetSexMustBe` (stat 0x3b: category 1005, the value and 100).
    Sex(i64),
    /// `Feedback_ThisEffectCanOnlyBeUtilitizedBy` (stats 0x3c and 0x170: the profession's name, category 2004).
    Profession(i64),
    /// Text category 100's `FlyingMustMeAllowedOnThePlayfield` (operator 0x70 in a playfield without flying).
    NoFlying,
}

/// FUN_100065d8 on `criteria`, the stats read from `holder`. `Err` when an operator needs what the context does not know.
pub fn evaluate(criteria: &[Criterion], holder: Holder, ctx: &dyn Context) -> Result<bool, Unknown> {
    judge(criteria, holder, ctx, &mut Vec::new(), &mut false)
}

/// [`evaluate`] with the feedback flag: the answer, and what is to be told when it is no. Lines are told only of a
/// list judged to its end (one cut short by a missing holder or an operator outside the table says nothing: the
/// original frees its list there, 0x100091de) and only when `holder` is the client's own character.
pub fn evaluate_told(criteria: &[Criterion], holder: Holder, ctx: &dyn Context) -> (Result<bool, Unknown>, Vec<Told>) {
    let (mut told, mut ended) = (Vec::new(), false);
    let result = judge(criteria, holder, ctx, &mut told, &mut ended);
    if result != Ok(false) || !ended || !ctx.is_own(holder) {
        told.clear();
    }
    (result, told)
}

fn judge(criteria: &[Criterion], holder: Holder, ctx: &dyn Context, told: &mut Vec<Told>, ended: &mut bool) -> Result<bool, Unknown> {
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
            // (0x100068fa) equal; a character's failed one has a line for a few stats
            0 => {
                let met = get(holder)? == v;
                if !met && character.is_some() {
                    match stat {
                        4 => told.push(Told::Breed(v)),
                        0x21 => told.push(Told::Side(v)),
                        0x3b => told.push(Told::Sex(v)),
                        0x3c | 0x170 => told.push(Told::Profession(v)),
                        _ => {}
                    }
                }
                met
            }
            // (0x10007065, 0x100071f7) below, above: the client's own character only
            1 => {
                let met = !own || get(holder)? < v;
                if !met {
                    told.push(Told::NotBelow { target: kind == 0x12, stat, value: v });
                }
                met
            }
            2 => {
                let met = !own || get(holder)? > v;
                if !met {
                    told.push(Told::AtLeast { target: kind == 0x12, stat, value: v + 1 });
                }
                met
            }
            // (0x10006f20) unequal
            0x18 => get(holder)? != v,
            // (0x10008a26, 0x10008a50, 0x10008a86, 0x10008abc) a stat against another (the value names it): equal, below,
            // above, unequal; the client's own character only, anyone else passes
            0x82..=0x85 => {
                if !own {
                    true
                } else {
                    let (a, b) = (get(holder)?, ctx.stat(holder, value as u32).ok_or(unknown)?);
                    match op {
                        0x82 => a == b,
                        0x83 => a < b,
                        0x84 => a > b,
                        _ => a != b,
                    }
                }
            }
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
            // (0x1000738b, 0x100073a7) the server's time below the value, above it (unsigned compares)
            5 => ctx.server_time().ok_or(unknown)? < value as u32,
            6 => ctx.server_time().ok_or(unknown)? > value as u32,
            // (0x100075a2) the character is the one the stat (its type) and the value (its instance) name
            9 => character == Some((stat, value as u32)),
            // (0x10007400, 0x1000741c) an item of the template the value names is worn on the clothes' page (3) or wielded
            // (the weapons', 7): no character, no; (0x10007466, 0x10007482) is not: no character, yes
            0x1f | 0x21 => match character {
                Some(c) => ctx.wears(c, value as u32, if op == 0x1f { 3 } else { 7 }).ok_or(unknown)?,
                None => false,
            },
            0x20 | 0x22 => match character {
                Some(c) => !ctx.wears(c, value as u32, if op == 0x20 { 3 } else { 7 }).ok_or(unknown)?,
                None => true,
            },
            // (0x10007eb6, 0x10007f85) has the nano program, has it not; a value of 0 passes either; no character, no.
            // (With the feedback flag a failure says `MustHaveFormula` / `MustNotHaveFormula` with the nano's name:
            // not told here.)
            0x23 | 0x24 => match character {
                None => false,
                Some(_) if value == 0 => true,
                Some(c) => ctx.knows_nano(c, value as u32).ok_or(unknown)? == (op == 0x23),
            },
            // (0x10008496, 0x10008571) the nano program runs on the character, does not; (0x10008605, 0x100086b2) one of
            // the strain runs, none does; no character, no. (Their lines, `Feedback_MustHaveRunning` and the rest, are not
            // told here.)
            0x5b | 0x65 => match character {
                Some(c) => ctx.runs_nano(c, value as u32).ok_or(unknown)? == (op == 0x5b),
                None => false,
            },
            0x5c | 0x66 => match character {
                Some(c) => ctx.runs_strain(c, value as u32).ok_or(unknown)? == (op == 0x5c),
                None => false,
            },
            // (0x10008830) the perk lock the value names is not running; no character, no
            0x61 => match character {
                Some(c) => !ctx.perk_locked(c, value as u32).ok_or(unknown)?,
                None => false,
            },
            // (0x10007ca7) stat 251's nibble (|value| / 1000 & 7) below |value| % 1000, or (a negative value) not below it
            0x42 => {
                let a = v.abs();
                let nibble = (ctx.stat(holder, 0xfb).ok_or(unknown)? >> ((a / 1000 & 7) * 4)) & 0xf;
                if v >= 0 { nibble < a % 1000 } else { nibble >= a % 1000 }
            }
            // (0x10007bdd) is flying: the dynel the picker the VALUE names finds is a character in movement mode 7
            // (FUN_10070a2f). The Ganimedes jetpack asks it of picker 2, the user, with a "not" after it. A character's
            // own user is itself (its `Beholder_t` +0x3c names it: inferred from the three places that set it), so
            // where the context cannot pick, 1 and 2 on a character are the character.
            0x46 => {
                let picked = match ctx.pick(holder, value) {
                    Some(p) => p,
                    None if matches!(value, 1 | 2) && character.is_some() => Some(holder),
                    None => return Err(unknown),
                };
                match picked.filter(|h| h.0 == CHARACTER) {
                    Some(h) => ctx.flying(h).ok_or(unknown)?,
                    None => false,
                }
            }
            // (0x1000754c) the character's playfield allows flying (its record's flag word lacks 0x80000); the stat
            // and the value are not read. With the feedback flag it says a line of its own.
            0x70 => {
                let allowed = !ctx.flying_forbidden(character.ok_or(unknown)?).ok_or(unknown)?;
                if !allowed {
                    told.push(Told::NoFlying);
                }
                allowed
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
    *ended = true;
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

    /// What a failed list tells: the comparisons that failed, each as the original words it; nothing of a list that
    /// passes (an alternative that failed beside one that held), of one cut short, or of anyone but oneself.
    #[test]
    fn a_failed_list_tells_what_failed() {
        let stats: BTreeMap<u32, i64> = [(161, 40), (54, 20), (60, 7)].into_iter().collect();
        let perks = HashSet::new();
        let c = own(&stats, &perks);
        assert_eq!(evaluate_told(&[(161, 75, 2)], ME, &c), (Ok(false), vec![Told::AtLeast { target: false, stat: 161, value: 76 }]));
        assert_eq!(evaluate_told(&[(161, 39, 2)], ME, &c), (Ok(true), vec![]));
        assert_eq!(evaluate_told(&[(54, 15, 1), (60, 3, 0), (0, 0, 4)], ME, &c), (Ok(false), vec![Told::NotBelow { target: false, stat: 54, value: 15 }, Told::Profession(3)]));
        // (either of two: the one that failed is not told when the other held)
        assert_eq!(evaluate_told(&[(161, 75, 2), (54, 10, 2), (0, 0, 3)], ME, &c), (Ok(true), vec![]));
        // (cut short by an operator outside the table: no line)
        assert_eq!(evaluate_told(&[(161, 75, 2), (0, 0, 86)], ME, &c), (Ok(false), vec![]));
        // (someone else's: equality fails for them too, and nothing is told)
        struct Other;
        impl Context for Other {
            fn stat(&self, _: Holder, _: u32) -> Option<i64> {
                Some(0)
            }
            fn is_own(&self, _: Holder) -> bool {
                false
            }
        }
        assert_eq!(evaluate_told(&[(60, 3, 0)], (CHARACTER, 9), &Other), (Ok(false), vec![]));
    }

    /// The Ganimedes jetpack's wield requirements (its real list): not flying already, and flying allowed where
    /// the character stands, with the line of its own when it is not.
    #[test]
    fn a_flight_item_asks_whether_one_flies_and_may_fly() {
        struct Flyer<'a>(Own<'a>, bool, bool);
        impl Context for Flyer<'_> {
            fn stat(&self, holder: Holder, stat: u32) -> Option<i64> {
                self.0.stat(holder, stat)
            }
            fn is_own(&self, holder: Holder) -> bool {
                self.0.is_own(holder)
            }
            fn flying(&self, _: Holder) -> Option<bool> {
                Some(self.1)
            }
            fn flying_forbidden(&self, _: Holder) -> Option<bool> {
                Some(self.2)
            }
        }
        let jetpack = [(0, 2, 70), (0, 0, 42), (438, 1, 107), (0, 0, 4), (139, 80, 2), (0, 0, 4), (410, 0, 0), (0, 0, 4), (359, 0, 0), (0, 0, 4), (389, 1, 22), (0, 0, 4), (531, 0, 0), (0, 0, 4), (0, 0, 112), (0, 0, 4)];
        let stats: BTreeMap<u32, i64> = [(139, 81), (389, 1)].into_iter().collect();
        let perks = HashSet::new();
        let at = |flying: bool, forbidden: bool| evaluate_told(&jetpack, ME, &Flyer(own(&stats, &perks), flying, forbidden));
        assert_eq!(at(false, false), (Ok(true), vec![]));
        assert_eq!(at(true, false), (Ok(false), vec![]));
        assert_eq!(at(false, true), (Ok(false), vec![Told::NoFlying]));
        // (a context that does not know of flight leaves the list unjudged, as before)
        assert_eq!(evaluate(&jetpack, ME, &own(&stats, &perks)), Err(Unknown { operator: 70 }));
    }

    #[test]
    fn below_and_above_pass_for_others() {
        // (a stat against another, 0x82..0x85: the own character's two stats; anyone else passes)
        let (stats, perks) = ([(16u32, 300i64), (17, 400)].into_iter().collect(), Default::default());
        let me = Own { id: (CHARACTER, 1), stats: &stats, perks: &perks };
        let all = |op| [(16, 17, op), (16, 16, op)].map(|c| evaluate(&[c], (CHARACTER, 1), &me));
        assert_eq!(all(0x82), [Ok(false), Ok(true)]);
        assert_eq!(all(0x83), [Ok(true), Ok(false)]);
        assert_eq!(all(0x84), [Ok(false), Ok(false)]);
        assert_eq!(all(0x85), [Ok(true), Ok(false)]);
        assert_eq!(evaluate(&[(17, 16, 0x84)], (CHARACTER, 1), &me), Ok(true));
        assert_eq!(evaluate(&[(16, 17, 0x82)], (CHARACTER, 2), &me), Ok(true));
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

    /// The operators that read the world through the context: the server's time (5, 6), who the character is (9), worn
    /// and wielded items (0x1f-0x22), known nanos (0x23, 0x24), running nanos and strains (0x5b, 0x65, 0x5c, 0x66) and
    /// perk locks (0x61); each without a character fails but for the "not worn" pair, which passes.
    #[test]
    fn world_operators() {
        struct W;
        impl Context for W {
            fn stat(&self, _: Holder, _: u32) -> Option<i64> {
                Some(0)
            }
            fn is_own(&self, holder: Holder) -> bool {
                holder == ME
            }
            fn server_time(&self) -> Option<u32> {
                Some(1000)
            }
            fn wears(&self, _: Holder, template: u32, page: u32) -> Option<bool> {
                Some((template, page) == (70254, 7) || (template, page) == (267907, 3))
            }
            fn knows_nano(&self, _: Holder, nano: u32) -> Option<bool> {
                Some(nano == 25994)
            }
            fn runs_nano(&self, _: Holder, nano: u32) -> Option<bool> {
                Some(nano == 211367)
            }
            fn runs_strain(&self, _: Holder, strain: u32) -> Option<bool> {
                Some(strain == 596)
            }
            fn perk_locked(&self, _: Holder, lock: u32) -> Option<bool> {
                Some(lock == 113)
            }
        }
        let item = (0xc73d, 5);
        let one = |c: Criterion| evaluate(&[c], ME, &W);
        assert_eq!([one((0, 1001, 5)), one((0, 1000, 5)), one((0, 999, 6)), one((0, 1000, 6))], [Ok(true), Ok(false), Ok(true), Ok(false)]);
        assert_eq!([one((CHARACTER, 7, 9)), one((CHARACTER, 8, 9))], [Ok(true), Ok(false)]);
        assert_eq!(evaluate(&[(CHARACTER, 5, 9)], (CHARACTER, 5), &W), Ok(true));
        // (the Damage Shield Projector's use: wielding either of its two templates)
        assert_eq!(one((0, 70254, 0x21)), Ok(true));
        assert_eq!(evaluate(&[(0, 70251, 33), (0, 70254, 33), (0, 0, 3)], ME, &W), Ok(true));
        assert_eq!([one((0, 70254, 0x1f)), one((0, 267907, 0x1f)), one((0, 267907, 0x20)), one((0, 70254, 0x22))], [Ok(false), Ok(true), Ok(false), Ok(false)]);
        assert_eq!([evaluate(&[(0, 1, 0x1f)], item, &W), evaluate(&[(0, 1, 0x20)], item, &W)], [Ok(false), Ok(true)]);
        assert_eq!([one((0, 25994, 0x23)), one((0, 25994, 0x24)), one((0, 1, 0x23)), one((0, 1, 0x24)), one((0, 0, 0x23))], [Ok(true), Ok(false), Ok(false), Ok(true), Ok(true)]);
        assert_eq!(evaluate(&[(0, 0, 0x24)], item, &W), Ok(false));
        assert_eq!([one((0, 211367, 0x5b)), one((0, 211367, 0x65)), one((0, 1, 0x5b)), one((0, 1, 0x65))], [Ok(true), Ok(false), Ok(false), Ok(true)]);
        assert_eq!([one((0, 596, 0x5c)), one((0, 596, 0x66)), one((0, 595, 0x66))], [Ok(true), Ok(false), Ok(true)]);
        assert_eq!(evaluate(&[(0, 596, 0x66)], item, &W), Ok(false));
        // (Dance of Fools: has the perk and its lock is not running)
        assert_eq!([one((0, 113, 0x61)), one((0, 114, 0x61))], [Ok(false), Ok(true)]);
        // (a context that knows none of it leaves them unjudged)
        let (stats, perks) = (BTreeMap::new(), HashSet::new());
        assert_eq!(evaluate(&[(0, 211367, 0x5b)], ME, &own(&stats, &perks)), Err(Unknown { operator: 0x5b }));
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
        // (an item of two actions, which the reader took for one of none: the Ganimedes Personal Jetpack, 290215, is
        // wielded with Vehicle Navigation, Airborne (139) above 80 among its conditions; its other action is 136)
        assert_eq!(actions(290215).iter().map(|a| a.kind).collect::<Vec<_>>(), [8, 136]);
        assert!(criteria(290215, 8).contains(&(139, 80, 2)));
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
