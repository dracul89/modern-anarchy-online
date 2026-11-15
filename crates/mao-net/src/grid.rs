//! The places a grid exit leads to: `GridDestinationSelectIIR_t` from the server and `GridSelectedIIR_t` back.
//!
//! The Grid's "Organisation City" and "Notum Controller" exits (and whatever else the server sends the list for) lead to
//! more than one place: the server sends the character a list and the client shows it for one to be picked (GUI.dll's
//! `TeleportTarget_c`, the "teleport target" window). Both messages' numbers are their class names through
//! `n3InfoItemRemote_t::MapToKey` (N3.dll 0x10009826: the name's bytes XORed into a word, byte `i` at `8 * (i & 3)`).
//!
//! * The list (Gamecode FUN_101295f0 → FUN_10129471, an entry FUN_1012905a): after the lead byte a count (the
//!   `0x3f1 * (n + 1)` word of the game's lists), then per entry a playfield number, an identity, a name (a 16-bit
//!   length and the bytes) and two words nothing in the client reads (stored at +0x2c and +0x28 of the entry, in that
//!   order); then a `Token_c` (FUN_1013d67a: the byte `'a'`, a length, the bytes), which is the server's and goes back
//!   with the answer as it came.
//! * The answer (FUN_1012993c): the token, the row picked (from 0), the row's playfield and its identity. A window
//!   closed without a pick answers row -1, playfield 0 and identity (0, 0) (GUI.dll FUN_100ffedf).

use crate::frame::Frame;
use crate::n3::{Message, CHARACTER};

/// `GridDestinationSelectIIR_t`.
pub const DESTINATION_SELECT: u32 = 0x0639_474d;
/// `GridSelectedIIR_t`.
pub const SELECTED: u32 = 0x3a32_2a4a;

/// `n3InfoItemRemote_t::MapToKey`: a message class's name to its number.
pub fn map_to_key(name: &str) -> u32 {
    name.bytes().enumerate().fold(0, |key, (i, b)| key ^ ((b as i8 as i32 as u32) << (8 * (i & 3))))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
    pub playfield: u32,
    pub identity: (u32, u32),
    /// The place's name: the window's "Location" column.
    pub name: String,
    /// The two words after the name, as they come (the client reads neither).
    pub words: [u32; 2],
}

/// The server's list and its token.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct List {
    pub entries: Vec<Destination>,
    pub token: Vec<u8>,
}

/// A `GridDestinationSelectIIR_t`'s body (from its lead byte).
pub fn parse(data: &[u8]) -> Option<List> {
    let word = |at: usize| data.get(at..at + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
    let coded = word(1)?;
    if coded % 0x3f1 != 0 || coded / 0x3f1 == 0 || coded / 0x3f1 - 1 >= 0x7531 {
        return None;
    }
    let mut at = 5;
    let mut entries = Vec::new();
    for _ in 0..coded / 0x3f1 - 1 {
        let (playfield, identity) = (word(at)?, (word(at + 4)?, word(at + 8)?));
        let length = u16::from_be_bytes(data.get(at + 12..at + 14)?.try_into().unwrap()) as usize;
        if length >= 0x8000 {
            return None;
        }
        let name = String::from_utf8_lossy(data.get(at + 14..at + 14 + length)?).into_owned();
        at += 14 + length;
        entries.push(Destination { playfield, identity, name, words: [word(at)?, word(at + 4)?] });
        at += 8;
    }
    if *data.get(at)? != b'a' {
        return None;
    }
    let length = word(at + 1)? as usize;
    Some(List { entries, token: data.get(at + 5..at + 5 + length)?.to_vec() })
}

/// `GridSelectedIIR_t`: the row picked of the list the token came with (its number, its playfield and its identity;
/// `None`: the window closed without one).
pub fn selected(character: u32, token: &[u8], pick: Option<(u32, u32, (u32, u32))>) -> Frame {
    let mut data = vec![0u8, b'a'];
    data.extend_from_slice(&(token.len() as u32).to_be_bytes());
    data.extend_from_slice(token);
    let (row, playfield, identity) = match pick {
        Some((row, playfield, identity)) => (row as i32, playfield, identity),
        None => (-1, 0, (0, 0)),
    };
    data.extend_from_slice(&row.to_be_bytes());
    for word in [playfield, identity.0, identity.1] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind: SELECTED, identity: (CHARACTER, character), data }.frame(character)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn the_numbers_are_the_class_names() {
        assert_eq!(map_to_key("GridDestinationSelectIIR_t"), DESTINATION_SELECT);
        assert_eq!(map_to_key("GridSelectedIIR_t"), SELECTED);
        // (two the captures name)
        assert_eq!(map_to_key("ActionIIR_t"), 0x2049_527c);
        assert_eq!(map_to_key("InventoryUpdateIIR_t"), 0x4e53_6976);
    }

    /// The list the user's client was sent at the Grid's organisation city exit (their log of 2026-11-15, 54 bytes of
    /// which the log kept the first 28: one entry, the city playfield 6010, a name of 18 bytes that starts "City Grid").
    /// The rest here is made up to the lengths the layout gives: the name's other nine bytes, the two words and a token
    /// of four bytes.
    #[test]
    fn the_list_of_the_organisation_city_exit() {
        let mut data = hex("01000007e20000177a0000c4190000c0000012436974792047726964");
        data.extend_from_slice(b" somewhere");
        data.truncate(28 + 9);
        data.extend_from_slice(&hex("0000000100000002"));
        data.extend_from_slice(&hex("6100000004deadbeef"));
        assert_eq!(data.len(), 54);
        let list = parse(&data).unwrap();
        assert_eq!(list.entries.len(), 1);
        let d = &list.entries[0];
        assert_eq!((d.playfield, d.identity, d.name.len(), d.words), (6010, (0xc419, 0xc000), 18, [1, 2]));
        assert!(d.name.starts_with("City Grid"));
        assert_eq!(list.token, hex("deadbeef"));
        // (cut short, or without the token's mark, it is no list)
        assert_eq!(parse(&data[..50]), None);
        data[45] = b'b';
        assert_eq!(parse(&data), None);
        // (an empty list: the count's word alone, and the token)
        assert_eq!(parse(&hex("01000003f16100000000")), Some(List::default()));
    }

    #[test]
    fn the_answer_carries_the_token_back() {
        let body = |f: Frame| f.body[f.body.len() - 25..].to_vec();
        assert_eq!(body(selected(7, &hex("deadbeef"), Some((0, 6010, (0xc419, 0xc000))))), hex("6100000004deadbeef000000000000177a0000c4190000c000"));
        assert_eq!(body(selected(7, &hex("deadbeef"), None)), hex("6100000004deadbeefffffffff000000000000000000000000"));
    }
}
