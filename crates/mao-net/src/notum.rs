//! The messages of the Notum Wars windows and of the battlestation: the laser target and airstrike window's list, the
//! battlestation's arrival, score, end and invitation, and the tower build menu's checklist.
//!
//! **None of these is in a capture**: every layout here is the original's reader or writer as decompiled, and each
//! message's number is its class name through `MapToKey` ([`crate::grid::map_to_key`]).
//!
//! * What the client asks goes out as `CharacterAction`s ([`crate::n3::character_action`]): the laser tags asked for
//!   (0x100, `N3Msg_RefreshLaserTags`, Gamecode 0x1001d80e), an orbital attack (0x102, 0x1001d943) or an airstrike
//!   (0x103, 0x1001d9db) on a tag `(0xc73d, its number)`, the battlestation's points asked for (0x104,
//!   `N3Msg_GetPointLocations`, 0x1001c7bf), and a tower item's checklist asked for (0xaf, `N3Msg_RequestChecklist`,
//!   0x10029668: the item's template low and high as the first identity and its quality as the second's first word).
//! * `LaserTagListIIR_t` (reader Gamecode FUN_1013178d): a count, then per tag three numbers (its own, one nothing
//!   reads, its playfield), two texts (a 16-bit length and the bytes: the area, then the player's name) and four
//!   numbers (the level range's two, and two nothing reads).
//! * `ArriveAtBsIIR_t` (reader FUN_10131128): the battlestation's points, a list (the game's `0x3f1 * (n + 1)` count)
//!   of three floats each. `SendScoreIIR_t` (FUN_10132050): two such lists of numbers, the sides' scores (the clan's,
//!   then Omni-Tek's) and each point's side. `BattleOverIIR_t` (FUN_1013120d): the character's points and how it
//!   ended. `InfromPlayerIIR_t` (sic; FUN_1013142d): an identity, the invitation to a battle.
//! * `AcceptBSInviteIIR_t` (writer FUN_10130ee1, made by FUN_10130f0b for `N3Msg_GoToBattle`): the invitation's
//!   identity and a byte, 1. `LeaveBattleIIR_t` (FUN_10131d66, `N3Msg_LeaveBattle`): nothing. Both keep the lead
//!   byte's 1 of a message that is to be passed on (`n3InfoItemRemote_t`'s constructor sets it; a
//!   `CharacterAction` clears it).

use crate::frame::Frame;
use crate::n3::{Message, CHARACTER};

pub const LASER_TAGS: u32 = 0x2933_154f;
pub const ARRIVE: u32 = 0x540e_3b27;
pub const SCORE: u32 = 0x4448_3b3a;
pub const BATTLE_OVER: u32 = 0x4b06_2919;
pub const INVITATION: u32 = 0x3301_337a;
pub const ACCEPT_INVITATION: u32 = 0x166a_435e;
pub const LEAVE_BATTLE: u32 = 0x3f3a_1914;

/// The `CharacterAction`s of these windows.
pub const REFRESH_LASER_TAGS: u32 = 0x100;
pub const ORBITAL_ATTACK: u32 = 0x102;
pub const AIRSTRIKE: u32 = 0x103;
pub const POINT_LOCATIONS: u32 = 0x104;
pub const REQUEST_CHECKLIST: u32 = 0xaf;
/// The server's answer to a checklist asked for: a `CharacterAction` whose second parameter is the flags of what
/// failed (Gamecode FUN_1005d61f's case for it hands them to the build menu, FUN_1001262d).
pub const CHECKLIST: u32 = 0xb0;
/// The identity type of a laser tag.
pub const LASER_TAG: u32 = 0xc73d;

/// A laser tag of the list: somebody marked for an attack.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LaserTag {
    pub id: u32,
    pub playfield: u32,
    pub name: String,
    pub area: String,
    /// The level range's two numbers, as the window prints them.
    pub levels: (i32, i32),
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn word(&mut self) -> Option<u32> {
        let w = u32::from_be_bytes(self.data.get(self.at..self.at + 4)?.try_into().ok()?);
        self.at += 4;
        Some(w)
    }

    /// A text: a 16-bit length (below 0x8000) and the bytes (Gamecode FUN_10038af8).
    fn text(&mut self) -> Option<String> {
        let length = u16::from_be_bytes(self.data.get(self.at..self.at + 2)?.try_into().ok()?) as usize;
        if length >= 0x8000 {
            return None;
        }
        let bytes = self.data.get(self.at + 2..self.at + 2 + length)?;
        self.at += 2 + length;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }

    /// The count of one of the game's lists: `0x3f1 * (n + 1)`, no more than 30000 (FUN_101321d1).
    fn count(&mut self) -> Option<usize> {
        let w = self.word()?;
        (w != 0 && w % 0x3f1 == 0 && w / 0x3f1 - 1 <= 0x7530).then(|| (w / 0x3f1 - 1) as usize)
    }
}

/// The laser tags of a `LaserTagList`.
pub fn laser_tags(m: &Message) -> Option<Vec<LaserTag>> {
    if m.kind != LASER_TAGS {
        return None;
    }
    let mut r = Reader { data: &m.data, at: 1 };
    let count = r.word()? as usize;
    // (a tag is 30 bytes at the least)
    if count > m.data.len() / 30 {
        return None;
    }
    let mut tags = Vec::with_capacity(count);
    for _ in 0..count {
        let (id, _, playfield) = (r.word()?, r.word()?, r.word()?);
        let (area, name) = (r.text()?, r.text()?);
        let levels = (r.word()? as i32, r.word()? as i32);
        let _ = (r.word()?, r.word()?);
        tags.push(LaserTag { id, playfield, name, area, levels });
    }
    Some(tags)
}

/// The battlestation's points, from an `ArriveAtBs`: where each stands (x, y, z).
pub fn arrival(m: &Message) -> Option<Vec<[f32; 3]>> {
    if m.kind != ARRIVE {
        return None;
    }
    let mut r = Reader { data: &m.data, at: 1 };
    let count = r.count()?;
    (0..count).map(|_| Some([f32::from_bits(r.word()?), f32::from_bits(r.word()?), f32::from_bits(r.word()?)])).collect()
}

/// A `SendScore`: the sides' scores (the clan's, then Omni-Tek's) and each point's side (0 red, 1 blue, 2 white).
pub fn score(m: &Message) -> Option<(Vec<i32>, Vec<i32>)> {
    if m.kind != SCORE {
        return None;
    }
    let mut r = Reader { data: &m.data, at: 1 };
    let mut list = |r: &mut Reader| -> Option<Vec<i32>> {
        let count = r.count()?;
        (0..count).map(|_| r.word().map(|w| w as i32)).collect()
    };
    let scores = list(&mut r)?;
    let sides = list(&mut r)?;
    Some((scores, sides))
}

/// A `BattleOver`: the character's points, and how the battle ended (0 red took the station, 1 blue did, 3 red won
/// for want of blue fighters, 4 blue for want of red).
pub fn battle_over(m: &Message) -> Option<(i32, i32)> {
    if m.kind != BATTLE_OVER {
        return None;
    }
    let mut r = Reader { data: &m.data, at: 1 };
    Some((r.word()? as i32, r.word()? as i32))
}

/// An `InfromPlayer`: a place in a battle is free; the identity goes back with the answer.
pub fn invitation(m: &Message) -> Option<(u32, u32)> {
    if m.kind != INVITATION {
        return None;
    }
    let mut r = Reader { data: &m.data, at: 1 };
    Some((r.word()?, r.word()?))
}

/// The invitation taken (`N3Msg_GoToBattle`).
pub fn accept(character: u32, invitation: (u32, u32)) -> Frame {
    let mut data = vec![1u8];
    data.extend_from_slice(&invitation.0.to_be_bytes());
    data.extend_from_slice(&invitation.1.to_be_bytes());
    data.push(1);
    Message { kind: ACCEPT_INVITATION, identity: (CHARACTER, character), data }.frame(character)
}

/// The battle left (`N3Msg_LeaveBattle`).
pub fn leave(character: u32) -> Frame {
    Message { kind: LEAVE_BATTLE, identity: (CHARACTER, character), data: vec![1] }.frame(character)
}

/// The checklist's flags from the server's answer, a `CharacterAction` 0xb0 (the second parameter).
pub fn checklist(m: &Message) -> Option<u32> {
    if m.kind != 0x5e47_7770 || m.data.len() < 25 || m.data[1..5] != CHECKLIST.to_be_bytes() {
        return None;
    }
    Some(u32::from_be_bytes(m.data[21..25].try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::map_to_key;

    fn message(kind: u32, data: Vec<u8>) -> Message {
        Message { kind, identity: (CHARACTER, 7), data }
    }

    #[test]
    fn the_numbers_are_the_class_names() {
        for (kind, name) in [
            (LASER_TAGS, "LaserTagListIIR_t"),
            (ARRIVE, "ArriveAtBsIIR_t"),
            (SCORE, "SendScoreIIR_t"),
            (BATTLE_OVER, "BattleOverIIR_t"),
            (INVITATION, "InfromPlayerIIR_t"),
            (ACCEPT_INVITATION, "AcceptBSInviteIIR_t"),
            (LEAVE_BATTLE, "LeaveBattleIIR_t"),
        ] {
            assert_eq!(map_to_key(name), kind, "{name}");
        }
    }

    #[test]
    fn a_laser_tag_list_reads_in_the_readers_order() {
        let mut data = vec![0u8];
        let word = |data: &mut Vec<u8>, w: u32| data.extend_from_slice(&w.to_be_bytes());
        let text = |data: &mut Vec<u8>, t: &str| {
            data.extend_from_slice(&(t.len() as u16).to_be_bytes());
            data.extend_from_slice(t.as_bytes());
        };
        word(&mut data, 2);
        for (id, playfield, area, name, low, high) in [(0x11, 585, "A field", "Somebody", 100, 150), (0x12, 620, "", "Another", 1, 220)] {
            word(&mut data, id);
            word(&mut data, 9);
            word(&mut data, playfield);
            text(&mut data, area);
            text(&mut data, name);
            word(&mut data, low);
            word(&mut data, high);
            word(&mut data, 0);
            word(&mut data, 0);
        }
        let tags = laser_tags(&message(LASER_TAGS, data.clone())).unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], LaserTag { id: 0x11, playfield: 585, name: "Somebody".into(), area: "A field".into(), levels: (100, 150) });
        assert_eq!((tags[1].area.as_str(), tags[1].levels), ("", (1, 220)));
        // (cut short, or a count that cannot be: nothing)
        assert!(laser_tags(&message(LASER_TAGS, data[..data.len() - 3].to_vec())).is_none());
        assert!(laser_tags(&message(LASER_TAGS, vec![0, 0xff, 0xff, 0xff, 0xff])).is_none());
        assert_eq!(laser_tags(&message(LASER_TAGS, vec![0, 0, 0, 0, 0])), Some(Vec::new()));
    }

    #[test]
    fn the_battlestations_messages_read() {
        let word = |data: &mut Vec<u8>, w: u32| data.extend_from_slice(&w.to_be_bytes());
        let mut points = vec![1u8];
        word(&mut points, 0x3f1 * 3);
        for f in [300.0f32, 5.0, 1600.0, 1000.0, 6.0, 500.0] {
            word(&mut points, f.to_bits());
        }
        assert_eq!(arrival(&message(ARRIVE, points)), Some(vec![[300.0, 5.0, 1600.0], [1000.0, 6.0, 500.0]]));
        let mut scores = vec![1u8];
        for w in [0x3f1 * 3, 120, 340, 0x3f1 * 3, 1, 2] {
            word(&mut scores, w);
        }
        assert_eq!(score(&message(SCORE, scores)), Some((vec![120, 340], vec![1, 2])));
        // (a count that is no list's: nothing)
        assert!(score(&message(SCORE, vec![1, 0, 0, 0, 5])).is_none());
        assert_eq!(battle_over(&message(BATTLE_OVER, vec![1, 0, 0, 0, 40, 0, 0, 0, 1])), Some((40, 1)));
        assert_eq!(invitation(&message(INVITATION, vec![1, 0, 0, 0xc3, 0x50, 0, 0, 0, 9])), Some((0xc350, 9)));
        let frame = accept(7, (0xc350, 9));
        assert_eq!(frame.body[frame.body.len() - 10..], [1, 0, 0, 0xc3, 0x50, 0, 0, 0, 9, 1]);
        let frame = leave(7);
        assert_eq!(frame.body[frame.body.len() - 1], 1);
    }

    #[test]
    fn a_checklists_answer_is_the_second_parameter() {
        let mut data = vec![0u8; 27];
        data[1..5].copy_from_slice(&CHECKLIST.to_be_bytes());
        data[21..25].copy_from_slice(&0x1040u32.to_be_bytes());
        assert_eq!(checklist(&message(0x5e47_7770, data.clone())), Some(0x1040));
        data[4] = 0xaf;
        assert_eq!(checklist(&message(0x5e47_7770, data)), None);
    }
}
