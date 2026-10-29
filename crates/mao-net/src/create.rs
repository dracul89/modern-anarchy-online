//! Making a character, as the original's character creation sends it to the login server (Interfaces.dll's `Client_t`):
//! `CreateCharacter` (0x10001928) is system message 0x0f carrying a `CharacterData_t` written by MessageProtocol.dll
//! (`CharacterData_t::WriteStream` 0x100012b1: `WriteDataStream` then the `CharacterInfo_c`'s `WriteStream` 0x10001784)
//! and one more int; `SuggestNickName` (0x1000188e) is 0x55 with the breed, sex and profession. The server answers
//! (`Client_t::ProcessMessage` 0x10002a9e) with 0x10 and a code (the creation refused), 0x11 and the new character's id
//! (made: the original then logs it in), or 0x56 with a suggested name (a `u16` length and the bytes).
//!
//! The values are the game's (GUI.dll's `SceneBase_t::ConvertCCBreedToGCBreedAndSex` / `ConvertCCProfToGCProf` turn the
//! screens' choices into them): the breed 1 Solitus, 2 Opifex, 3 Nanomage, 4 Atrox; the sex 1 neuter (Atrox), 2 male,
//! 3 female; the head the head mesh's instance; the height 90, 100 or 110 (short, normal, tall); the width (build) 0, 1
//! or 2 (small, medium, large).

use crate::frame::Frame;
use crate::system::{system, Error, Stream};

pub const CREATE_CHARACTER: u32 = 0x0f;
pub const CREATE_REFUSED: u32 = 0x10;
pub const CREATED: u32 = 0x11;
pub const SUGGEST_NAME: u32 = 0x55;
pub const SUGGESTED_NAME: u32 = 0x56;

/// A character to make.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewCharacter {
    pub name: String,
    pub breed: i32,
    pub sex: i32,
    pub profession: i32,
    pub head: i32,
    pub height: i32,
    pub width: i32,
}

fn int(p: &mut Vec<u8>, v: i32) {
    p.extend_from_slice(&v.to_be_bytes());
}

fn string(p: &mut Vec<u8>, s: &str) {
    int(p, s.len() as i32);
    p.extend_from_slice(s.as_bytes());
}

/// System message 0x0f: the character's data as `CharacterData_t::WriteStream` writes it for a new one, then 0 (the
/// int `NameScene_t::CreatePressed` leaves at its +0x74).
pub fn create_character(c: &NewCharacter) -> Frame {
    let mut p = Vec::new();
    // `WriteDataStream`: version 4, the id, the playfield proxy ('a', two identities and two ints), "created" as an int
    int(&mut p, 4);
    int(&mut p, 0);
    p.push(b'a');
    for _ in 0..6 {
        int(&mut p, 0);
    }
    int(&mut p, 0);
    // `CharacterInfo_c::WriteStream`: version 5, the instance and the organization's, the name, breed, sex, profession,
    // level, the area ("area unknown", `InitDefault`), banned and its reason, head, height, width
    int(&mut p, 5);
    int(&mut p, 0);
    int(&mut p, 0);
    string(&mut p, &c.name);
    for v in [c.breed, c.sex, c.profession, 0] {
        int(&mut p, v);
    }
    string(&mut p, "area unknown");
    int(&mut p, 0);
    string(&mut p, "");
    for v in [c.head, c.height, c.width] {
        int(&mut p, v);
    }
    int(&mut p, 0);
    system(CREATE_CHARACTER, &p)
}

/// System message 0x55: a name for a character of this breed, sex and profession, please.
pub fn suggest_name(breed: i32, sex: i32, profession: i32) -> Frame {
    let mut p = Vec::new();
    for v in [breed, sex, profession] {
        int(&mut p, v);
    }
    system(SUGGEST_NAME, &p)
}

/// What the login server said about a creation or a name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// 0x10: refused, with the server's code.
    Refused(i32),
    /// 0x11: made; the new character's id.
    Created(u32),
    /// 0x56: a suggested name.
    Name(String),
}

/// The reply in a system message, if it is one of these.
pub fn reply(frame: &Frame) -> Result<Option<Reply>, Error> {
    let (kind, payload) = crate::system::split(frame)?;
    let mut s = Stream::new(payload);
    Ok(match kind {
        CREATE_REFUSED => Some(Reply::Refused(s.i32("the code")?)),
        CREATED => Some(Reply::Created(s.i32("the id")? as u32)),
        SUGGESTED_NAME => {
            let n = u16::from_be_bytes([s.u8("the name's length")?, s.u8("the name's length")?]) as usize;
            // (`FUN_10005dc1`: a length of 0x8000 or more is no name)
            let bytes: Vec<u8> = if n < 0x8000 { (0..n).map(|_| s.u8("the name")).collect::<Result<_, _>>()? } else { Vec::new() };
            Some(Reply::Name(String::from_utf8_lossy(&bytes).into_owned()))
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_new_character_reads_back_as_a_listed_one() {
        let c = NewCharacter { name: "Testabc".into(), breed: 3, sex: 3, profession: 15, head: 0x8e3c, height: 100, width: 1 };
        let f = create_character(&c);
        let (kind, payload) = crate::system::split(&f).unwrap();
        assert_eq!(kind, 0x0f);
        // (the character list's reader takes the same `CharacterData_t`, and its trailing int is the 0)
        let mut list = 1i32.to_be_bytes().to_vec();
        list.extend_from_slice(payload);
        let listed = Frame { kind: f.kind, source: 1, destination: 2, body: [crate::system::CHARACTER_LIST.to_be_bytes().to_vec(), list].concat() };
        let (chars, rest) = crate::system::character_list(&listed).unwrap();
        let got = &chars[0];
        assert_eq!((got.name.as_str(), got.breed, got.sex, got.profession, got.level, got.head, got.height, got.width), ("Testabc", 3, 3, 15, 0, 0x8e3c, 100, 1));
        assert_eq!((got.id, got.created, got.area.as_str(), got.banned, got.flag), (0, false, "area unknown", false, 0));
        assert!(rest.is_empty());
        assert_eq!(payload.len(), 4 + 4 + 1 + 24 + 4 + 12 + 4 + 7 + 16 + 4 + 12 + 4 + 4 + 12 + 4);
    }

    #[test]
    fn replies() {
        let msg = |kind: u32, data: &[u8]| Frame { kind: crate::frame::SYSTEM, source: 1, destination: 0, body: [kind.to_be_bytes().to_vec(), data.to_vec()].concat() };
        assert_eq!(reply(&msg(0x11, &0x123456i32.to_be_bytes())).unwrap(), Some(Reply::Created(0x123456)));
        assert_eq!(reply(&msg(0x10, &7i32.to_be_bytes())).unwrap(), Some(Reply::Refused(7)));
        assert_eq!(reply(&msg(0x56, b"\x00\x05Zorba")).unwrap(), Some(Reply::Name("Zorba".into())));
        assert_eq!(reply(&msg(0x0e, &[])).unwrap(), None);
        let s = suggest_name(1, 2, 6);
        assert_eq!(s.body, [0, 0, 0, 0x55, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 6]);
    }
}
