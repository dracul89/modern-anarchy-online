//! The system messages of the login. A system message (frame kind 1) has a
//! body of a big-endian `u32` type and a payload.
//!
//! | type | direction | payload |
//! |------|-----------|---------|
//! | 0x22 | client → server | `[i32 2][name: 0x28 bytes][version: 0x14 bytes]` (the first message) |
//! | 0x24 | server → client | the challenge: 32 characters |
//! | 0x25 | client → server | `[name: 0x28 bytes][i32 n+1][response, n characters and a NUL]` |
//! | 0x0e | server → client | the characters of the account |
//! | 0x16 | client → server | log a character in (not decoded yet) |
//!
//! Names and versions are NUL-padded text. Integers are big-endian; a
//! string in a stream is an `i32` length and its bytes.

use std::fmt;

use crate::frame::{Frame, SYSTEM};

pub const INIT_AUTH: u32 = 0x22;
pub const CHALLENGE: u32 = 0x24;
pub const AUTH_CLIENT: u32 = 0x25;
pub const CHARACTER_LIST: u32 = 0x0e;
pub const LOGIN_CHARACTER: u32 = 0x16;

const NAME_FIELD: usize = 0x28;
const VERSION_FIELD: usize = 0x14;

#[derive(Debug)]
pub enum Error {
    Truncated(&'static str),
    NotSystem(u16),
    Type { expected: u32, got: u32 },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated(what) => write!(f, "message truncated in {what}"),
            Error::NotSystem(kind) => write!(f, "a message of kind {kind}, not a system message"),
            Error::Type { expected, got } => write!(f, "system message {got:#x} where {expected:#x} was expected"),
        }
    }
}

impl std::error::Error for Error {}

fn system(kind: u32, payload: &[u8]) -> Frame {
    let mut body = kind.to_be_bytes().to_vec();
    body.extend_from_slice(payload);
    // A client's messages go from source 0 to destination 1 in the capture.
    Frame { kind: SYSTEM, source: 0, destination: 1, body }
}

fn padded(text: &str, size: usize) -> Vec<u8> {
    let mut v = text.as_bytes()[..text.len().min(size - 1)].to_vec();
    v.resize(size, 0);
    v
}

/// The first message: who is connecting and the client's version
/// (`version.id`'s first line, such as `18.8.62_EP1`).
pub fn init_auth(user: &str, version: &str) -> Frame {
    let mut p = 2i32.to_be_bytes().to_vec();
    p.extend(padded(user, NAME_FIELD));
    p.extend(padded(version, VERSION_FIELD));
    system(INIT_AUTH, &p)
}

/// The answer to the challenge.
pub fn auth_client(user: &str, response: &str) -> Frame {
    let mut p = padded(user, NAME_FIELD);
    p.extend_from_slice(&(response.len() as i32 + 1).to_be_bytes());
    p.extend_from_slice(response.as_bytes());
    p.push(0);
    system(AUTH_CLIENT, &p)
}

/// The type and payload of a system message.
pub fn split(frame: &Frame) -> Result<(u32, &[u8]), Error> {
    if frame.kind != SYSTEM {
        return Err(Error::NotSystem(frame.kind));
    }
    let b = frame.body.get(..4).ok_or(Error::Truncated("the type"))?;
    Ok((u32::from_be_bytes(b.try_into().unwrap()), &frame.body[4..]))
}

/// The challenge text of a 0x24 message.
pub fn challenge(frame: &Frame) -> Result<String, Error> {
    let (kind, payload) = split(frame)?;
    if kind != CHALLENGE {
        return Err(Error::Type { expected: CHALLENGE, got: kind });
    }
    Ok(String::from_utf8_lossy(payload).trim_end_matches('\0').to_string())
}

/// A reader of the original's `BinaryStream`: big-endian, unaligned.
pub struct Stream<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Stream<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Stream { data, at: 0 }
    }

    pub fn remaining(&self) -> &'a [u8] {
        &self.data[self.at..]
    }

    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len()).ok_or(Error::Truncated(what))?;
        let b = &self.data[self.at..end];
        self.at = end;
        Ok(b)
    }

    pub fn u8(&mut self, what: &'static str) -> Result<u8, Error> {
        Ok(self.take(1, what)?[0])
    }

    pub fn i32(&mut self, what: &'static str) -> Result<i32, Error> {
        Ok(i32::from_be_bytes(self.take(4, what)?.try_into().unwrap()))
    }

    /// A string: an `i32` length and the bytes.
    pub fn string(&mut self, what: &'static str) -> Result<String, Error> {
        let n = self.i32(what)?.max(0) as usize;
        Ok(String::from_utf8_lossy(self.take(n, what)?).into_owned())
    }
}

/// A character of the account (`CharacterData_t` with its `CharacterInfo_c`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Character {
    pub id: u32,
    /// The playfield the character is in: the proxy's six words (a type
    /// 0xc79c identity and its instance among them).
    pub proxy: [u32; 6],
    pub created: bool,
    pub name: String,
    pub breed: i32,
    pub sex: i32,
    pub profession: i32,
    pub level: i32,
    pub area: String,
    pub banned: bool,
    pub ban_reason: String,
    pub head: i32,
    pub height: i32,
    pub width: i32,
    /// The int after each character in the list (always 1 so far).
    pub flag: i32,
}

fn character(s: &mut Stream) -> Result<Character, Error> {
    let version = s.i32("a character's version")?;
    let mut c = Character { id: s.i32("the id")? as u32, ..Default::default() };
    // Version 4 data: the playfield proxy ('a' and six ints), whether the
    // character is created.
    if version >= 4 {
        s.u8("the proxy version")?;
        for w in &mut c.proxy {
            *w = s.i32("the proxy")? as u32;
        }
    }
    c.created = s.i32("created")? != 0;
    // The info, version 5.
    let info = s.i32("the info version")?;
    let _instance = s.i32("the instance")?;
    if info > 4 {
        s.i32("the org instance")?;
    }
    c.name = s.string("the name")?;
    c.breed = s.i32("the breed")?;
    c.sex = s.i32("the sex")?;
    c.profession = s.i32("the profession")?;
    c.level = s.i32("the level")?;
    c.area = s.string("the area")?;
    c.banned = s.i32("banned")? != 0;
    c.ban_reason = s.string("the ban reason")?;
    if info > 3 {
        c.head = s.i32("the head")?;
        c.height = s.i32("the height")?;
        c.width = s.i32("the width")?;
    }
    c.flag = s.i32("the flag")?;
    Ok(c)
}

/// The characters of a 0x0e message, and whatever follows them.
pub fn character_list(frame: &Frame) -> Result<(Vec<Character>, Vec<u8>), Error> {
    let (kind, payload) = split(frame)?;
    if kind != CHARACTER_LIST {
        return Err(Error::Type { expected: CHARACTER_LIST, got: kind });
    }
    let mut s = Stream::new(payload);
    let count = s.i32("the count")?.max(0);
    let mut out = Vec::new();
    for _ in 0..count {
        out.push(character(&mut s)?);
    }
    Ok((out, s.remaining().to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_auth_layout() {
        let f = init_auth("someone", "18.8.62_EP1");
        let bytes = f.encode(1);
        assert_eq!(bytes.len(), 84);
        assert_eq!(&bytes[..8], &[0, 1, 0, 1, 0, 1, 0, 0x54]);
        assert_eq!(&bytes[16..24], &[0, 0, 0, 0x22, 0, 0, 0, 2]);
        assert_eq!(&bytes[24..31], b"someone");
        assert_eq!(&bytes[64..75], b"18.8.62_EP1");
    }

    #[test]
    fn auth_client_layout() {
        // The captured one: 450 bytes for a response of 385 characters.
        let f = auth_client("someone", &"a".repeat(385));
        let bytes = f.encode(2);
        assert_eq!(bytes.len(), 452);
        assert_eq!(&bytes[6..8], &[0x01, 0xc2]);
        assert_eq!(&bytes[16..20], &[0, 0, 0, 0x25]);
        assert_eq!(&bytes[60..64], &[0, 0, 1, 0x82]);
    }

    /// One character as the capture has it (made-up name and ids).
    #[test]
    fn character_record() {
        let mut p = Vec::new();
        let int = |p: &mut Vec<u8>, v: i32| p.extend_from_slice(&v.to_be_bytes());
        int(&mut p, 1); // count
        int(&mut p, 4); // data version
        int(&mut p, 0x9210c); // id
        p.push(b'a');
        for w in [0xc79c, 0x21c, 0, 0, 0, 0] {
            int(&mut p, w);
        }
        int(&mut p, 1); // created
        int(&mut p, 5); // info version
        int(&mut p, 0x9210c);
        int(&mut p, 0);
        int(&mut p, 4);
        p.extend_from_slice(b"Test");
        for v in [1, 2, 1, 25] {
            int(&mut p, v);
        }
        int(&mut p, 12);
        p.extend_from_slice(b"area unknown");
        for v in [0, 0, 0, 0, 0, 1] {
            int(&mut p, v);
        }
        int(&mut p, 8); // the trailer
        let mut body = CHARACTER_LIST.to_be_bytes().to_vec();
        body.extend(p);
        let frame = Frame { kind: SYSTEM, source: 1, destination: 2, body };
        let (list, rest) = character_list(&frame).unwrap();
        assert_eq!(list.len(), 1);
        let c = &list[0];
        assert_eq!((c.id, c.name.as_str(), c.level, c.profession, c.area.as_str(), c.flag), (0x9210c, "Test", 25, 1, "area unknown", 1));
        assert_eq!(c.proxy[..2], [0xc79c, 0x21c]);
        assert_eq!(rest, 8i32.to_be_bytes());
    }
}

/// Log the character in (system message 0x16: its id).
pub fn login_character(id: u32) -> Frame {
    system(LOGIN_CHARACTER, &id.to_be_bytes())
}

pub const CHARACTER_TICKET: u32 = 0x17;
pub const ENTER_GAME_SERVER: u32 = 0x1b;

/// Where the character's game server is, and the key to enter it
/// (system message 0x17).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub id: u32,
    pub address: [u8; 4],
    pub port: u16,
    /// Eight bytes the server made for this login; the client hands them
    /// back to the game server.
    pub key: [u8; 8],
    /// What follows the key (a zero word and a four-byte value of unknown
    /// meaning; the original doesn't send them on).
    pub rest: Vec<u8>,
}

impl Ticket {
    pub fn host(&self) -> String {
        let a = self.address;
        format!("{}.{}.{}.{}:{}", a[0], a[1], a[2], a[3], self.port)
    }
}

pub fn ticket(frame: &Frame) -> Result<Ticket, Error> {
    let (kind, payload) = split(frame)?;
    if kind != CHARACTER_TICKET {
        return Err(Error::Type { expected: CHARACTER_TICKET, got: kind });
    }
    let mut s = Stream::new(payload);
    let id = s.i32("the id")? as u32;
    let ip = s.i32("the address")?.to_be_bytes();
    let port = u16::from_be_bytes([s.u8("the port")?, s.u8("the port")?]);
    let mut key = [0u8; 8];
    for b in &mut key {
        *b = s.u8("the key")?;
    }
    Ok(Ticket { id, address: ip, port, key, rest: s.remaining().to_vec() })
}

/// The first message to the game server: the character and the key. It
/// is sent from the character (source) to destination 2.
pub fn enter_game_server(ticket: &Ticket) -> Frame {
    let mut p = ticket.id.to_be_bytes().to_vec();
    p.extend_from_slice(&ticket.key);
    let mut f = system(ENTER_GAME_SERVER, &p);
    f.source = ticket.id;
    f.destination = 2;
    f
}
