//! The server running a spell list on a character (`SpellListIIR_t`, message `4d450114`).
//!
//! The reader (FUN_100aabcb) takes a byte, the spells (FUN_100a71ae: an `x3f1`-counted list, `(n + 1) × 1009`, each a whole
//! `SpellData_t` as `GameData::operator>>` reads it: type, instance, version, criteria and the format's values, in the stream's
//! big-endian order, see `mao_formats::spell::read_network`), the caster's identity (+0x1c), the target's identity (+0x28), a
//! flag (+0x24: the list is ending), the list's name (+0x30, a short length and its bytes), a flag and, when set, an identity
//! (+0x4c, +0x54: the nano or item the list comes from, `cf1b` in the captures), a flag (+0x4d) and an int (+0x50).
//!
//! The apply (FUN_100aae0e) finds the message's character as a `Beholder_t` and calls its +0x30 (FUN_100026d0) with the list,
//! the caster, the ending flag, the target, the name, the item and the flag. FUN_100026d0 makes a run of each spell
//! (FUN_100a4dca), marks it ending (bit 0 of +0x24) from the flag, and runs it at once when it has one hit (stat 3), no delay
//! (stat 4) and is not a 0xcf16; others are queued on the character (FUN_10002595 with 9) and not ported. The spell's
//! target stat (0x20) picks whom it runs on: 3 the caster, 0xe and 0x17 characters found through the character, anything else
//! the character itself (the only ones ported). FUN_100a5f4b then runs the spell by its type, if FUN_100a57a0 lets the type
//! run on the client and the spell's criteria pass (FUN_100065d8: not ported, so a spell with criteria is left out).
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
//! * Not ported (rare in the captures): 0xcf82 "Stun" (FUN_100a879c: the character's state flags 6 / 0x400, the duel buffs),
//!   0xcf22 (FUN_100a4f8a: a stat set on the character, e.g. 689 on pets and 359 MonsterData), 0xcf92 (FUN_100a8107: a
//!   vehicle's animations 0x1d / 0x27), 0xcf4c (FUN_100a7c79: state flag of stat 0x49), 0xcf44 (FUN_100a7aa2: stat 0x167),
//!   0xcf7d (FUN_100a83d8: feedback lines), 0xcf35 (FUN_100a517a, an item's modify).

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

/// What a list run on a character shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// An effect template (`gfxtweak.bin`) made on the character, for its duration in seconds when the spell sets one.
    Effect { effect: i32, seconds: Option<f32> },
}

/// The spells of a list that run at once on the message's character, and what each shows (FUN_100026d0 → FUN_100a5f4b).
pub fn run(list: &SpellList) -> Vec<Outcome> {
    let mut out = Vec::new();
    for spell in &list.spells {
        // (queued runs, other targets and spells with criteria are not ported)
        if spell.kind == 0xcf16 || spell.stat(3) > 1 || spell.stat(4) != 0 || matches!(spell.stat(0x20), 3 | 0xe | 0x17) || !spell.criteria.is_empty() {
            continue;
        }
        match spell.kind {
            // (FUN_100a55d9, FUN_100a9159: ending deletes the effect whose handle is the spell's stat 0x42, which a spell fresh from
            // the message never has, so nothing is made or seen to end)
            0xcf26 | 0xcfd4 if !list.ending => {
                let hundredths = spell.stat(49);
                out.push(Outcome::Effect { effect: spell.stat(39), seconds: (hundredths != 0).then(|| hundredths as f32 / 100.0) });
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
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
        assert!(run(&l).is_empty());
    }

    /// The 16:55 capture's Malfunctioning Cleaning Robot: two effects (0xcf26), 0xa8e3 and 0xa871, with no caster or name.
    #[test]
    fn a_robots_effects() {
        let a = SpellList::parse(&hex("00000007e20000cf2644fb183e0000000400000000000000010000000000000000000000000000a8e300000000000000000000000000000000000000000000000000000000000000000000c3507b1a6629000000000000000000")).unwrap();
        let b = SpellList::parse(&hex("00000007e20000cf2644fb183f0000000400000000000000010000000000000000000000000000a87100000000000000000000000000000000000000000000000000000000000000000000c3507b1a6629000000000000000000")).unwrap();
        assert_eq!(a.target, (0xc350, 0x7b1a6629));
        assert_eq!(run(&a), vec![Outcome::Effect { effect: 0xa8e3, seconds: None }]);
        assert_eq!(run(&b), vec![Outcome::Effect { effect: 0xa871, seconds: None }]);
        // (ending: nothing)
        let mut c = a.clone();
        c.ending = true;
        assert!(run(&c).is_empty());
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
        assert!(run(&l).is_empty());
    }

    /// The 08:42 capture's Shadowland Anvian: a 0xcfd4 effect (sparks 0xaaa9, 5 s) behind two criteria, so not run here.
    #[test]
    fn a_spell_with_criteria_is_left_out() {
        let l = SpellList::parse(&hex("00000007e20000cfd4000394e600000004000000020000000000000000000000640000019a0000000000000002000000010000000000000002000000090000aaa9000001f400000064000000000000000000000000000000320000000000000000000000000000003200000001000007d30000c3507b15fc4e0000c3507b15fc4e000045416e7669616e20416e67727920536d6f6b6520616e6420537061726b7341494e616e6f536861646f776c616e6420416e7669616e204566666563745f706c635f20315f2032010000cf1b000394e60000000000")).unwrap();
        assert_eq!((l.spells[0].kind, l.spells[0].stat(39), l.spells[0].stat(49)), (0xcfd4, 0xaaa9, 500));
        assert!(run(&l).is_empty());
        let mut free = l.clone();
        free.spells[0].criteria.clear();
        assert_eq!(run(&free), vec![Outcome::Effect { effect: 0xaaa9, seconds: Some(5.0) }]);
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
