//! The server running a spell list on a character (`SpellListIIR_t`, message `4d450114`).
//!
//! The reader (FUN_100aabcb) takes a byte, the spells (FUN_100a71ae: an `x3f1`-counted list, `(n + 1) × 1009`, each a whole
//! `SpellData_t` as `GameData::operator>>` reads it: type, instance, version, criteria and the format's values, in the stream's
//! big-endian order, see `mao_formats::spell::read_network`), the caster's identity (+0x1c), the target's identity (+0x28), a
//! flag (+0x24: the list is ending), the list's name (+0x30, a short length and its bytes), a flag and, when set, an identity
//! (+0x4c, +0x54: the nano or item the list comes from, `cf1b` in the captures), a flag (+0x4d) and an int (+0x50).
//!
//! The apply (FUN_100aae0e) finds the message's character as a `Beholder_t` and calls its +0x30 (FUN_100026d0) with the list,
//! the caster (+0x1c), the ending flag, the +0x28 identity (which becomes each run's +0x10, the "caster" the criteria's "On
//! Caster" reads), the name, the item and the flag. FUN_100026d0 ([`run`]) makes a run of each spell (FUN_100a4dca), marks it
//! ending (bit 0 of +0x24) from the flag, picks whom it runs on by the spell's target stat (0x20: 3 the caster, 0xe the
//! character's fighting target, 0x17 its use target, anything else the character), and runs it at once when it has one hit
//! (stat 3), no delay (stat 4) and is not a 0xcf16, else queues it on that dynel (FUN_10002595 with 9: [`Queue`], run each frame
//! by FUN_10002fc7). FUN_100a5f4b ([`fire`]) runs a spell when its delay has passed, if FUN_100a57a0 lets the type run on the
//! client and the spell's criteria pass (FUN_100065d8, `requirements`), by its type.
//!
//! What the client does with the spell types the captures' lists hold (FUN_100a5f4b's jump table at 0x100a6cb6):
//! * 0xcf26 (FUN_100a55d9): the effect of stat 39 made on the character (`_EffectHandler_t::CreateEffect2`, the handler's
//!   +0x50 1 and +0x54 0x20), its duration stat 49 in hundredths when set (`SetDuration`, DAT_10159670 = 100); ending, the
//!   effect whose handle the spell keeps in stat 0x42 is deleted. Ported: [`Outcome::Effect`].
//! * 0xcfd4 (FUN_100a9159): the same effect, also with repetitions (stat 0xa1), start and stop colours (0x99-0xa0 over 255)
//!   and a scale (stat 0x32 / 100 + 1). Ported as the effect only (no colours or scale).
//! * 0xcf3b "Cast nano" (FUN_100a7814, and 0xcfd6/0xcfdf/0xcfe0 through it): an empty function. The most common lists ("Event
//!   Loop ...", "Arete Landing", "Effect of Inferno Environment", the AI lists) are only these: the client draws nothing for
//!   them, the server casts the nano itself and tells it with its own messages.
//! * 0xcf0a, 0xcf56, 0xcf4a, 0xcf70, 0xcf1b, 0xd003: outside the table or its default, nothing on the client.
//! * 0xcf82 "Stun" (FUN_100a879c, the duel buffs' begin and end): the character's state flags, stat 224 ([`StateFlags`]): it
//!   loses 6 (2 and 4, which let it move) and gains 0x400, an NPC also 0x100 and 0x4200; the ending list gives them back.
//!   Ported: [`Outcome::Stun`]; one's own character cannot move meanwhile. (Its call to the vehicle's +0x9c/+0x98,
//!   FUN_1005a02c(1), is not.)
//! * 0xcf22 (FUN_100a4f8a): stat 0 set to stat 39 (689 on pets, 359 MonsterData on two NPCs in the captures), except the
//!   stats it leaves alone. Ported: [`Outcome::SetStat`], kept for one's own character only (the client has no stats of others).
//! * 0xcf92 (FUN_100a8107, a hoverbike's program: "Phasefront Banshee - Red" in the 08:42 capture): on a character, its
//!   movement is handed the action 0x1d (`SwitchToFlyModeTransitionAction_t`, FUN_1006cb58's case: mode 7), or 0x27
//!   (`LeaveFlyModeTransitionAction_t`) when the list is ending. Ported: [`Outcome::Fly`]. (It first makes a crawling,
//!   sitting, sleeping or lounging character get up, 0x28 / 0x29 / 0x2a / 0x25: not ported.)
//! * Not ported (rare in the captures): 0xcf4c (FUN_100a7c79: state
//!   flag of stat 0x49), 0xcf44 (FUN_100a7aa2: stat 0x167), 0xcf7d (FUN_100a83d8: feedback lines), 0xcf35 (FUN_100a517a, an
//!   item's modify), and 0xcf16 runs (left out).
//! * 0xcfff (FUN_100a9af8): its string 0 told in the notification window (`GlobalSignals_c` +0x298), see
//!   [`crate::notifications`]. Ported: [`Outcome::Text`]. None of the captures has one.

use mao_formats::spell::Spell;

pub const SPELL_LIST: u32 = 0x4d450114;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellList {
    pub spells: Vec<Spell>,
    pub caster: (u32, u32),
    pub target: (u32, u32),
    /// +0x24: the list is ending (each run's bit 0 of +0x24).
    pub ending: bool,
    pub name: String,
    /// +0x54: the nano or item the list comes from, when the flag +0x4c is set.
    pub item: Option<(u32, u32)>,
    pub flag: bool,
    pub value: i32,
}

impl SpellList {
    /// FUN_100aabcb.
    pub fn parse(data: &[u8]) -> Option<SpellList> {
        let word = |o: usize| data.get(o..o + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
        let mut p = 1;
        // (FUN_100a71ae: a count that is a positive multiple of 1009 below 1000 spells, else the stream fails)
        let count = word(p)?;
        p += 4;
        if count == 0 || count % 0x3f1 != 0 || count / 0x3f1 - 1 >= 1000 {
            return None;
        }
        let mut list = SpellList::default();
        for _ in 0..count / 0x3f1 - 1 {
            let (spell, end) = mao_formats::spell::read_network(data, p)?;
            list.spells.push(spell);
            p = end;
        }
        list.caster = (word(p)?, word(p + 4)?);
        list.target = (word(p + 8)?, word(p + 12)?);
        p += 16;
        list.ending = *data.get(p)? != 0;
        let len = u16::from_be_bytes(data.get(p + 1..p + 3)?.try_into().unwrap()) as usize;
        list.name = String::from_utf8_lossy(data.get(p + 3..p + 3 + len)?).into_owned();
        p += 3 + len;
        if *data.get(p)? != 0 {
            list.item = Some((word(p + 1)?, word(p + 5)?));
            p += 8;
        }
        p += 1;
        list.flag = *data.get(p)? != 0;
        list.value = word(p + 1)? as i32;
        Some(list)
    }
}

/// What a spell run shows on the client, on the dynel it ran on.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// An effect template (`gfxtweak.bin`) made on the character, for its duration in seconds when the spell sets one.
    Effect { effect: i32, seconds: Option<f32> },
    /// 0xcf82 "Stun" (FUN_100a879c) on a character: its state flags (stat 224, [`StateFlags`]) lose 6 and gain 0x400 (an NPC,
    /// +0x21c, also 0x100 and 0x4200) when it begins; the other way when the list ends it.
    Stun { begin: bool, npc: bool },
    /// 0xcf22 (FUN_100a4f8a): the stat set to the value (vtable +0x40, or +0x44 for a stat the dynel does not have yet).
    SetStat { stat: u32, value: i32 },
    /// A 0xcfff spell (FUN_100a9af8): a line for the notification window, told on the client's own character (range 0) or
    /// within the range (stat 11, metres) of it ([`crate::notifications::spell_line`]).
    Text { spell: Spell },
    /// 0xcf92 (FUN_100a8107) on a character: it switches to fly mode (movement action 0x1d) when the spell is applied and
    /// leaves it (0x27) when the list ends.
    Fly { begin: bool },
}

/// What the spell runner needs to know of the world besides the criteria's stats.
pub trait Scene: crate::requirements::Context {
    /// `n3Dynel_t::GetDynel` finds the dynel (it is in the client's playfield).
    fn exists(&self, id: Holder) -> bool;
    /// A character is an NPC (`SimpleChar_t` +0x21c); `None` when unknown.
    fn is_npc(&self, id: Holder) -> Option<bool>;
    /// The character's fighting target (its +0x1d4 object's +0x4c identity, as `N3Msg_GetTargetTarget` reads it).
    fn fight_target(&self, id: Holder) -> Option<Holder>;
    /// The target the character uses things on (its +0x1e4 object's +0x14 identity, which FUN_1003fd9f sets with the
    /// selection).
    fn use_target(&self, id: Holder) -> Option<Holder>;
}

pub use crate::requirements::Holder;

/// A spell run (FUN_100a4dca's 0x34 bytes): +0 the hits left, +8 when it last ran (the game time, seconds), +0x10 the identity
/// the list names (the message's +0x28; "On Caster" criteria read it), +0x20 the spell, +0x24 bit 0 the list is ending.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub spell: Spell,
    pub hits: i32,
    pub time: f64,
    pub caster: Holder,
    pub ending: bool,
    /// Whom it runs on (the queue's owner, for a queued run).
    pub on: Holder,
}

/// FUN_100a57a0: the spell types the client runs at all (others are only the server's: their run is spent at once).
fn client_runs(kind: u32) -> bool {
    matches!(
        kind,
        0xcf0b | 0xcf0c | 0xcf11 | 0xcf13 | 0xcf14 | 0xcf16 | 0xcf17 | 0xcf1b | 0xcf1f | 0xcf20 | 0xcf21 | 0xcf22 | 0xcf23 | 0xcf24 | 0xcf26
            | 0xcf27 | 0xcf28 | 0xcf29 | 0xcf2a | 0xcf30 | 0xcf31 | 0xcf32 | 0xcf34 | 0xcf35 | 0xcf37 | 0xcf38 | 0xcf3a | 0xcf3b | 0xcf3c
            | 0xcf3d | 0xcf41 | 0xcf44 | 0xcf49 | 0xcf4a | 0xcf4b | 0xcf4c | 0xcf4d | 0xcf4e | 0xcf50 | 0xcf53 | 0xcf54 | 0xcf57 | 0xcf5e
            | 0xcf6b | 0xcf6c | 0xcf6d | 0xcf76 | 0xcf77 | 0xcf7b | 0xcf7d | 0xcf7e | 0xcf81 | 0xcf82 | 0xcf83 | 0xcf87 | 0xcf88 | 0xcf8a
            | 0xcf8c | 0xcf8e | 0xcf92 | 0xcf93 | 0xcf94 | 0xcf95 | 0xcf9d | 0xcf9e | 0xcfa1 | 0xcfa4 | 0xcfa5 | 0xcfa7 | 0xcfaa | 0xcfb7
            | 0xcfc0 | 0xcfc9 | 0xcfd1 | 0xcfd4 | 0xcfd6 | 0xcfdd | 0xcff0 | 0xcff5 | 0xcfff
    )
}

/// The criteria's view of a run: the scene's stats, with the run's caster for "On Caster".
struct RunContext<'a> {
    scene: &'a dyn Scene,
    caster: Holder,
}

impl crate::requirements::Context for RunContext<'_> {
    fn stat(&self, holder: Holder, stat: u32) -> Option<i64> {
        self.scene.stat(holder, stat)
    }
    fn is_own(&self, holder: Holder) -> bool {
        self.scene.is_own(holder)
    }
    fn pick(&self, holder: Holder, which: i32) -> Option<Option<Holder>> {
        self.scene.pick(holder, which)
    }
    fn caster(&self) -> Option<Holder> {
        // (FUN_100065d8 at 0x100068ae: `GetDynel` of the run's +0x10, none when it is not in the playfield)
        self.scene.exists(self.caster).then_some(self.caster)
    }
    fn has_perk(&self, character: Holder, perk: u32) -> Option<bool> {
        self.scene.has_perk(character, perk)
    }
}

/// FUN_100a5f4b: a run's turn on `run.on` at game time `now`: what it shows, when it runs. The hits left go down by one each
/// time it runs and each time its criteria fail (a criterion the client cannot tell counts as failing).
pub fn fire(run: &mut Run, scene: &dyn Scene, now: f64) -> Vec<Outcome> {
    if !client_runs(run.spell.kind) {
        run.hits = 0;
        return Vec::new();
    }
    if run.hits == 0 {
        return Vec::new();
    }
    // (a run made this moment with a delay and several hits is put back by the delay's whole seconds, so its first hit can come
    // at once: 0x100a5fa4..0x100a5fbe; DAT_10159670 = 100, hundredths)
    let delay = run.spell.stat(4);
    if run.time == now && delay > 0 && run.hits > 1 {
        run.time -= (delay / 100) as f64;
    }
    if (((now - run.time) * 100.0) as i32) < delay.abs() {
        return Vec::new();
    }
    if !run.spell.criteria.is_empty() && !(run.ending && run.spell.kind == 0xcf87) {
        let ctx = RunContext { scene, caster: run.caster };
        if crate::requirements::evaluate(&run.spell.criteria, run.on, &ctx) != Ok(true) {
            run.hits -= 1;
            return Vec::new();
        }
    }
    run.time = now;
    if run.hits > 0 {
        run.hits -= 1;
    }
    let spell = &run.spell;
    match spell.kind {
        // (FUN_100a55d9, FUN_100a9159: ending deletes the effect whose handle is the spell's stat 0x42, which a spell fresh
        // from the message never has, so nothing is made or seen to end)
        0xcf26 | 0xcfd4 if !run.ending => {
            let hundredths = spell.stat(49);
            vec![Outcome::Effect { effect: spell.stat(39), seconds: (hundredths != 0).then(|| hundredths as f32 / 100.0) }]
        }
        // (FUN_100a879c: on a character only; on anything else the caster, when it is the client's, is told
        // "Feedback_YouNeedToSelectATargetForProgram", not ported)
        0xcf82 if run.on.0 == crate::requirements::CHARACTER => vec![Outcome::Stun { begin: !run.ending, npc: scene.is_npc(run.on).unwrap_or(false) }],
        0xcf22 => {
            // (FUN_100a4f8a: stat 0 → the dynel's vtable +0x10 with the value, not known; stats 0x3c, 0x25, 0x2f, 4, 0x3b, 0x16d,
            // 0x1b and 0xd6 are not set: on a character 0x2f goes to its selection (FUN_1003e464) and 0x3c to FUN_10056552, not
            // ported; the rest is set)
            let (stat, value) = (spell.stat(0), spell.stat(39));
            if stat == 0 || matches!(stat, 0x3c | 0x25 | 0x2f | 4 | 0x3b | 0x16d | 0x1b | 0xd6) {
                Vec::new()
            } else {
                vec![Outcome::SetStat { stat: stat as u32, value }]
            }
        }
        // (FUN_100a9af8: string 0 told in the notification window, by the range rule of `notifications::spell_line`)
        0xcfff if !run.ending => vec![Outcome::Text { spell: spell.clone() }],
        // (FUN_100a8107: on a `SimpleChar_t` only; 0x1d, or 0x27 from an ending list)
        mao_formats::item::FLY_SPELL if run.on.0 == crate::requirements::CHARACTER => vec![Outcome::Fly { begin: !run.ending }],
        _ => Vec::new(),
    }
}

/// The runs waiting on the characters (each `Beholder_t`'s queue, FUN_10002595 with 9; FUN_10002fc7 runs them each frame).
#[derive(Default)]
pub struct Queue {
    pub runs: Vec<Run>,
}

impl Queue {
    /// FUN_10002fc7: each waiting run's turn, then the runs with no hits left go.
    pub fn tick(&mut self, scene: &dyn Scene, now: f64) -> Vec<(Holder, Outcome)> {
        let mut out = Vec::new();
        for run in &mut self.runs {
            out.extend(fire(run, scene, now).into_iter().map(|o| (run.on, o)));
        }
        self.runs.retain(|r| r.hits != 0);
        out
    }

    /// The runs waiting on a dynel that left (its queue goes with it).
    pub fn forget(&mut self, id: Holder) {
        self.runs.retain(|r| r.on != id);
    }
}

/// FUN_100026d0 (`Beholder_t` +0x30): the list run on `character` (the message's dynel) at game time `now`. Each spell becomes a
/// run (FUN_100a4dca, its hits spell stat 3, FUN_100a4f16) with the list's +0x28 identity as its caster; a run is queued when it is
/// a 0xcf16, has more than one hit or a delay (an ending list's queued run gets one hit), else runs at once. Whom it runs on is
/// the spell's target (stat 0x20):
/// * 3: the list's caster (+0x1c); without one in the playfield the rest of the list is not run;
/// * 0xe: the character's fighting target, or (an ending list) the caster; nobody: the spell is not run;
/// * 0x17: the character's use target (+0x1e4's +0x14); nobody: not run;
/// * else the character (target 2 through its `Beholder_t` +0x3c, its own identity).
/// What runs at once is returned with whom it ran on; the queued runs go on `queue`. (0xcf16 runs are not ported: they are left
/// out.)
pub fn run(list: &SpellList, character: Holder, scene: &dyn Scene, queue: &mut Queue, now: f64) -> Vec<(Holder, Outcome)> {
    let mut out = Vec::new();
    for spell in &list.spells {
        if spell.kind == 0xcf16 {
            continue;
        }
        let hits = spell.stat(3);
        let queued = hits > 1 || spell.stat(4) != 0;
        let on = match spell.stat(0x20) {
            3 => {
                if !scene.exists(list.caster) {
                    break;
                }
                list.caster
            }
            0xe => match if list.ending { scene.exists(list.caster).then_some(list.caster) } else { scene.fight_target(character) } {
                Some(t) if t.0 == crate::requirements::CHARACTER => t,
                _ => continue,
            },
            0x17 => match scene.use_target(character) {
                Some(t) if t.0 == crate::requirements::CHARACTER => t,
                _ => continue,
            },
            _ => character,
        };
        let mut r = Run { spell: spell.clone(), hits: if queued && list.ending { 1 } else { hits }, time: now, caster: list.target, ending: list.ending, on };
        if queued {
            queue.runs.push(r);
        } else {
            out.extend(fire(&mut r, scene, now).into_iter().map(|o| (on, o)));
        }
    }
    out
}

/// A character's state flags (stat 224) as FUN_10044aac (add) and FUN_10044c71 (remove) keep them: a count for each bit,
/// started at 1 for each bit the stat has (FUN_1004496c); a bit is set while its count is above 0. Bits 2 and 4 let the character
/// move (`N3Msg_MouseMovement` turns it, and FUN_1006bd94 starts a movement action, only with one of them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateFlags {
    counts: [i8; 32],
    pub bits: u32,
}

impl StateFlags {
    /// FUN_1004496c.
    pub fn new(stat: u32) -> StateFlags {
        let mut counts = [0; 32];
        for (b, c) in counts.iter_mut().enumerate() {
            if stat & 1 << b != 0 {
                *c = 1;
            }
        }
        StateFlags { counts, bits: stat }
    }

    /// FUN_10044aac.
    pub fn add(&mut self, mask: u32) {
        for b in 0..32 {
            if mask & 1 << b != 0 {
                self.counts[b] = self.counts[b].wrapping_add(1);
                if self.counts[b] > 0 {
                    self.bits |= 1 << b;
                }
            }
        }
    }

    /// FUN_10044c71.
    pub fn remove(&mut self, mask: u32) {
        for b in 0..32 {
            if mask & 1 << b != 0 {
                self.counts[b] = self.counts[b].wrapping_sub(1);
                if self.counts[b] < 1 {
                    self.bits &= !(1 << b);
                }
            }
        }
    }

    /// FUN_100a879c's flags.
    pub fn stun(&mut self, begin: bool, npc: bool) {
        if begin {
            self.remove(6);
            self.add(0x400);
            if npc {
                self.add(0x100);
                self.add(0x4200);
            }
        } else {
            self.add(6);
            self.remove(0x400);
            if npc {
                self.remove(0x100);
                self.remove(0x4200);
            }
        }
    }

    /// Bit 2 or bit 4: the character may move.
    pub fn can_move(&self) -> bool {
        self.bits & 6 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    /// A playfield of NPCs (every dynel named is in it unless left out), and the client's own character.
    #[derive(Default)]
    struct World {
        own: Holder,
        stats: std::collections::BTreeMap<u32, i64>,
        absent: Vec<Holder>,
        fighting: Option<Holder>,
    }

    impl crate::requirements::Context for World {
        fn stat(&self, holder: Holder, stat: u32) -> Option<i64> {
            (holder == self.own).then(|| self.stats.get(&stat).copied().unwrap_or(0))
        }
        fn is_own(&self, holder: Holder) -> bool {
            holder == self.own
        }
    }

    impl Scene for World {
        fn exists(&self, id: Holder) -> bool {
            id != (0, 0) && !self.absent.contains(&id)
        }
        fn is_npc(&self, id: Holder) -> Option<bool> {
            Some(id != self.own)
        }
        fn fight_target(&self, _: Holder) -> Option<Holder> {
            self.fighting
        }
        fn use_target(&self, _: Holder) -> Option<Holder> {
            None
        }
    }

    /// The list run at once on its message's dynel (its +0x28), what shows.
    fn shown(l: &SpellList) -> Vec<Outcome> {
        let mut q = Queue::default();
        let out = run(l, l.target, &World::default(), &mut q, 100.0);
        assert!(q.runs.is_empty());
        out.into_iter()
            .map(|(on, o)| {
                assert_eq!(on, l.target);
                o
            })
            .collect()
    }

    fn spell(kind: u32, hits: i32, delay: i32, target: i32, more: &[(i32, i32)]) -> Spell {
        let mut stats = vec![(3, hits), (4, delay), (0x20, target), (0x23, 9)];
        stats.extend_from_slice(more);
        Spell { kind, instance: 1, criteria: Vec::new(), stats, strings: Vec::new() }
    }

    fn list(spells: Vec<Spell>, ending: bool) -> SpellList {
        SpellList { spells, caster: (0xc350, 2), target: (0xc350, 1), ending, ..Default::default() }
    }

    /// The 16:55 capture's NPC loop: one "Cast nano" spell (0xcf3b, nano 0x48329) on Cedric Harding, from the nano 0x48329.
    #[test]
    fn reads_an_npc_event_loop() {
        let l = SpellList::parse(&hex("00000007e20000cf3b00048329000000040000000000000001000000000000000200000009000483290000c3507b1a64330000c3507b1a64330000194576656e74204c6f6f70204365647269632048617264696e67010000cf1b000483290000000000")).unwrap();
        assert_eq!(l.spells.len(), 1);
        assert_eq!((l.spells[0].kind, l.spells[0].stat(39)), (0xcf3b, 0x48329));
        assert_eq!((l.caster, l.target), ((0xc350, 0x7b1a6433), (0xc350, 0x7b1a6433)));
        assert_eq!(l.name, "Event Loop Cedric Harding");
        assert_eq!(l.item, Some((0xcf1b, 0x48329)));
        assert!(!l.ending && !l.flag && l.value == 0);
        // (FUN_100a7814: a cast spell shows nothing on the client)
        assert!(shown(&l).is_empty());
    }

    /// The 16:55 capture's Malfunctioning Cleaning Robot: two effects (0xcf26), 0xa8e3 and 0xa871, with no caster or name.
    #[test]
    fn a_robots_effects() {
        let a = SpellList::parse(&hex("00000007e20000cf2644fb183e0000000400000000000000010000000000000000000000000000a8e300000000000000000000000000000000000000000000000000000000000000000000c3507b1a6629000000000000000000")).unwrap();
        let b = SpellList::parse(&hex("00000007e20000cf2644fb183f0000000400000000000000010000000000000000000000000000a87100000000000000000000000000000000000000000000000000000000000000000000c3507b1a6629000000000000000000")).unwrap();
        assert_eq!(a.target, (0xc350, 0x7b1a6629));
        assert_eq!(shown(&a), vec![Outcome::Effect { effect: 0xa8e3, seconds: None }]);
        assert_eq!(shown(&b), vec![Outcome::Effect { effect: 0xa871, seconds: None }]);
        // (ending: nothing)
        let mut c = a.clone();
        c.ending = true;
        assert!(shown(&c).is_empty());
    }

    /// The 16:55 capture's lists with a string spell (0xcf70, five criteria), a modify (0xcf0a) and a cast: all read, none
    /// shows anything.
    #[test]
    fn reads_a_list_with_a_string_spell() {
        let l = SpellList::parse(&hex("0000000fc40000cf70000483290000000400000005000001d600000000000000000000020d0000005a000000010000000000000000000000040000001b0000000000000002000000000000000000000004000000010000000000000003000000090000001a7b73746f636b5f7065745f61747461636b5f6163636570747d00000000010000cf0a000483290000000400000005000001d600000000000000000000020d0000005a000000010000000000000000000000040000001b000000000000000200000000000000000000000400000001000000000000000200000009000001d60000000100000001000000000000cf3b00048329000000040000000000000001000000000000000200000009000483290000c3507b1a64330000c3507b1a64330000194576656e74204c6f6f70204365647269632048617264696e67010000cf1b000483290000000000")).unwrap();
        assert_eq!(l.spells.iter().map(|s| s.kind).collect::<Vec<_>>(), vec![0xcf70, 0xcf0a, 0xcf3b]);
        assert_eq!(l.spells[0].criteria.len(), 5);
        assert_eq!(l.spells[0].string(0), "{stock_pet_attack_accept}");
        assert_eq!(l.name, "Event Loop Cedric Harding");
        assert!(shown(&l).is_empty());
    }

    /// The 08:42 capture's Shadowland Anvian: a 0xcfd4 effect (sparks 0xaaa9, 5 s) behind two criteria, "On Caster" and stat 410
    /// above 0: the caster (the list's +0x28, the Anvian itself) is not the client's character, so the comparison passes
    /// (FUN_100065d8 checks below / above only on the client's own) and the sparks show. Without the Anvian in the playfield
    /// "On Caster" finds nobody and the criteria fail.
    #[test]
    fn the_anvians_sparks_pass_their_criteria() {
        let l = SpellList::parse(&hex("00000007e20000cfd4000394e600000004000000020000000000000000000000640000019a0000000000000002000000010000000000000002000000090000aaa9000001f400000064000000000000000000000000000000320000000000000000000000000000003200000001000007d30000c3507b15fc4e0000c3507b15fc4e000045416e7669616e20416e67727920536d6f6b6520616e6420537061726b7341494e616e6f536861646f776c616e6420416e7669616e204566666563745f706c635f20315f2032010000cf1b000394e60000000000")).unwrap();
        assert_eq!((l.spells[0].kind, l.spells[0].stat(39), l.spells[0].stat(49)), (0xcfd4, 0xaaa9, 500));
        assert_eq!(l.spells[0].criteria, vec![(0, 0, 100), (410, 0, 2)]);
        assert_eq!(shown(&l), vec![Outcome::Effect { effect: 0xaaa9, seconds: Some(5.0) }]);
        let away = World { absent: vec![l.target], ..Default::default() };
        assert!(run(&l, l.target, &away, &mut Queue::default(), 0.0).is_empty());
        // (on the client's own character the comparison is made: stat 410 at 0 is not above 0)
        let me = World { own: l.target, ..Default::default() };
        assert!(run(&l, l.target, &me, &mut Queue::default(), 0.0).is_empty());
    }

    /// FUN_100a5f4b's timing: three hits 1.5 s apart (delay 150). Made this moment, the run is put back the delay's whole
    /// seconds (one), so its first hit waits half a second; each hit after waits the whole delay; with no hits left it leaves
    /// the queue.
    #[test]
    fn a_repeated_spell_hits_on_its_delay() {
        let l = list(vec![spell(0xcf26, 3, 150, 2, &[(39, 0xa8e3)])], false);
        let w = World::default();
        let mut q = Queue::default();
        assert!(run(&l, l.target, &w, &mut q, 10.0).is_empty());
        assert_eq!(q.runs.len(), 1);
        let effect = (l.target, Outcome::Effect { effect: 0xa8e3, seconds: None });
        assert!(q.tick(&w, 10.0).is_empty());
        assert!(q.tick(&w, 10.25).is_empty());
        assert_eq!(q.tick(&w, 10.5), vec![effect.clone()]);
        assert!(q.tick(&w, 11.75).is_empty());
        assert_eq!(q.tick(&w, 12.0), vec![effect.clone()]);
        assert_eq!(q.tick(&w, 13.5), vec![effect.clone()]);
        assert!(q.runs.is_empty());
        // (a delay of whole seconds: the first hit comes at once)
        let l = list(vec![spell(0xcf26, 2, 200, 2, &[(39, 0xa8e3)])], false);
        run(&l, l.target, &w, &mut q, 10.0);
        assert_eq!(q.tick(&w, 10.0), vec![effect.clone()]);
        assert!(q.tick(&w, 11.99).is_empty());
        assert_eq!(q.tick(&w, 12.0), vec![effect.clone()]);
        assert!(q.runs.is_empty());
        // (one hit with a delay waits it)
        let l = list(vec![spell(0xcf26, 1, 50, 2, &[(39, 0xa8e3)])], false);
        run(&l, l.target, &w, &mut q, 10.0);
        assert!(q.tick(&w, 10.4).is_empty());
        assert_eq!(q.tick(&w, 10.5), vec![effect.clone()]);
        assert!(q.runs.is_empty());
        // (an ending list's queued run has one hit)
        let l = list(vec![spell(0xcf82, 5, 100, 2, &[])], true);
        run(&l, l.target, &w, &mut q, 10.0);
        assert_eq!(q.runs[0].hits, 1);
        // (failing criteria spend a hit)
        let mut r = Run { spell: spell(0xcf26, 2, 100, 2, &[]), hits: 2, time: 0.0, caster: (0, 0), ending: false, on: l.target };
        r.spell.criteria = vec![(0, 0, 99)];
        assert!(fire(&mut r, &w, 5.0).is_empty());
        assert_eq!(r.hits, 1);
    }

    /// Whom a spell runs on (FUN_100026d0): 3 the list's caster (none: the rest of the list stops), 0xe the character's
    /// fighting target (an ending list: the caster), 0x17 its use target (none here: left out), anything else the character.
    #[test]
    fn the_targets() {
        let s = |t: i32| spell(0xcf26, 1, 0, t, &[(39, 7)]);
        let l = list(vec![s(3), s(0xe), s(1), s(0x17)], false);
        let w = World { fighting: Some((0xc350, 5)), ..Default::default() };
        let out = run(&l, l.target, &w, &mut Queue::default(), 0.0);
        assert!(out.iter().all(|o| o.1 == Outcome::Effect { effect: 7, seconds: None }));
        assert_eq!(out.iter().map(|o| o.0).collect::<Vec<_>>(), vec![(0xc350, 2), (0xc350, 5), (0xc350, 1)]);
        // (an ending list: 0xe is the caster; a stun's end shows it)
        let l = list(vec![spell(0xcf82, 1, 0, 0xe, &[])], true);
        assert_eq!(run(&l, l.target, &w, &mut Queue::default(), 0.0), vec![((0xc350, 2), Outcome::Stun { begin: false, npc: true })]);
        let w = World { absent: vec![(0xc350, 2)], ..Default::default() };
        let l = list(vec![s(3), s(1)], false);
        assert!(run(&l, l.target, &w, &mut Queue::default(), 0.0).is_empty());
    }

    /// The three Arete captures' 4128 lists (when the main checkout has them), each run on its message's dynel in a playfield of
    /// NPCs that holds every dynel named: what the client now runs. Before the criteria, the queue and the stun, only the 573
    /// criteria-free 0xcf26 effects ran.
    #[test]
    fn captured_lists_run() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../modern-anarchy-online/captures");
        let mut tally: std::collections::BTreeMap<&str, usize> = Default::default();
        let mut lists = 0;
        for cap in ["arete-20261030-1655", "arete-20261031-0842", "arete-20261031-1505"] {
            let Ok(text) = std::fs::read_to_string(root.join(cap).join("dump.txt")) else { return };
            for line in text.lines().filter(|l| l.contains(" kind 4d450114 ")) {
                let id = line.split("identity (").nth(1).and_then(|r| r.split(')').next()).unwrap();
                let (a, b) = id.split_once(',').unwrap();
                let identity = (u32::from_str_radix(a, 16).unwrap(), u32::from_str_radix(b, 16).unwrap());
                let l = SpellList::parse(&hex(line.split("bytes: ").nth(1).unwrap())).expect(line);
                lists += 1;
                let mut q = Queue::default();
                let mut out = run(&l, identity, &World::default(), &mut q, 0.0);
                out.extend(q.tick(&World::default(), 0.0));
                assert!(q.runs.is_empty());
                for (_, o) in out {
                    *tally
                        .entry(match o {
                            Outcome::Effect { .. } => "effect",
                            Outcome::Stun { begin: true, .. } => "stun",
                            Outcome::Stun { begin: false, .. } => "stun ends",
                            Outcome::SetStat { .. } => "stat",
                            Outcome::Text { .. } => "text",
                            Outcome::Fly { begin: true } => "fly",
                            Outcome::Fly { begin: false } => "fly ends",
                        })
                        .or_default() += 1;
                }
            }
        }
        assert_eq!(lists, 4128);
        // (the two flights of the 08:42 capture: a Banshee's and a Wraith's program begun and ended)
        assert_eq!(tally, [("effect", 576), ("stun", 720), ("stun ends", 716), ("stat", 36), ("fly", 2), ("fly ends", 2)].into_iter().collect());
    }

    /// 0xcf82 (the duel buffs' "Stun") and 0xcf22.
    #[test]
    fn stun_and_set_stat() {
        let w = World::default();
        let stun = list(vec![spell(0xcf82, 1, 0, 2, &[])], false);
        assert_eq!(shown(&stun), vec![Outcome::Stun { begin: true, npc: true }]);
        let mut flags = StateFlags::new(6);
        assert!(flags.can_move());
        flags.stun(true, false);
        assert_eq!(flags.bits, 0x400);
        assert!(!flags.can_move());
        flags.stun(false, false);
        assert_eq!(flags.bits, 6);
        // (counted: two stuns need two ends)
        flags.stun(true, true);
        flags.stun(true, true);
        flags.stun(false, true);
        assert!(!flags.can_move());
        flags.stun(false, true);
        assert_eq!(flags.bits, 6);
        let set = list(vec![spell(0xcf22, 1, 0, 2, &[(0, 689), (39, 5)])], false);
        assert_eq!(run(&set, set.target, &w, &mut Queue::default(), 0.0), vec![((0xc350, 1), Outcome::SetStat { stat: 689, value: 5 })]);
        // (the profession is not set this way)
        let set = list(vec![spell(0xcf22, 1, 0, 2, &[(0, 0x3c), (39, 5)])], false);
        assert!(shown(&set).is_empty());
    }

    /// 0xcf92 (FUN_100a8107): a hoverbike's list switches its character to fly mode, the same list ending switches it back;
    /// on anything but a character it does nothing.
    #[test]
    fn the_fly_spell() {
        let fly = |ending: bool| list(vec![spell(0xcf35, 1, 0, 1, &[(0, 156), (39, 800)]), spell(0xcf4c, 1, 0, 1, &[(73, 2), (25, 0)]), spell(0xcf92, 1, 0, 1, &[])], ending);
        assert_eq!(shown(&fly(false)), vec![Outcome::Fly { begin: true }]);
        assert_eq!(shown(&fly(true)), vec![Outcome::Fly { begin: false }]);
        let mut q = Queue::default();
        assert!(run(&fly(false), (0xc73d, 7), &World::default(), &mut q, 0.0).is_empty() && q.runs.is_empty());
    }

    /// `MAO_GAME`: the robot's two effects are a flare (0xa8e3, kind 1005, four seconds) and a star burst at the pelvis (0xa871,
    /// kind 2004), both of which the client draws.
    #[test]
    fn the_robots_effects_are_drawn() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let data = std::fs::read(std::path::Path::new(&game).join("Setupf").join("gfxtweak.bin")).unwrap();
        let gfx = mao_formats::gfx::Gfx::parse(&data).unwrap();
        let flare = gfx.get(0xa8e3).and_then(mao_formats::gfx::Flare::read).expect("0xa8e3 is a flare");
        assert_eq!(flare.duration, 4.0);
        assert!(matches!(gfx.visual(0xa871), Some(mao_formats::gfx::Visual::Stars(_))));
    }
}
