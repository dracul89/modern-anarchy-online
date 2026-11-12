//! The chat server (`chat.d1.funcom.com:7105`, named by the system `0x43` ChatServerInfo): a second
//! connection with its own, simpler framing, decoded from the captures. Every packet is `u16 type`,
//! `u16 length` (of what follows), then the payload; all big-endian; strings are `u16 length` + bytes.
//! The type numbers belong to a direction.
//!
//! Client to server: type 0, the login: `u32 0, u32 character id, str16 user, str16 response` (the same
//! challenge response as the game login, made from the salt below).
//!
//! Server to client:
//! * 0: the salt, a `str16`;  5: login accepted (empty);
//! * 20: `u32 id, str16 name` (a name for an id);
//! * 34: `u32 sender, str16 text, str16 blob` vicinity speech (GUI.dll's ppj parser FUN_1016d6f4, `S2C_VIS_MESSAGE_FMT`, format
//!   "ISD");
//! * 35: `str16 name, str16 text, str16 blob` anonymous vicinity speech (`S2C_VIS_ANON_MESSAGE`, "SSD"): the system lines
//!   ("Welcome to AnarchyOnline") come this way, with an empty name;
//! * 40: `u32 channel id, 7 more bytes` (a channel membership flag, not decoded);
//! * 60: a channel: `u8 kind, u32 id, str16 name, u32 flags, str16` (the ppj parser's "GSID"; flag 0x40: no voice lines, GUI.dll's
//!   `HandleGroupMessage` and FUN_100b88b1);
//! * 30: `u32 sender, str16 text, str16 blob` a private message (a tell) from a character; the server's own "Players Online"
//!   notice arrives the same way;
//! * 21: the answer to a name lookup, laid out like 20;
//! * 65: `u8 kind, u32 id (the channel), u32 sender, str16 text, str16 blob` a channel message (the blob may carry a voice line,
//!   GUI.dll's FUN_10086075).
//!
//! Client to server, after the login (the numbers of the public AO chat protocol, which agree with the ones above; the tell and the
//! lookup were tested live): 21 looks a name up (`str16 name`), 30 sends a tell (`u32 target, str16 text, str16 blob`) and 65 sends to a
//! channel (`u8 kind, u32 id, str16 text, str16 blob`).
//!
//! Private chat groups (GUI.dll's chat library, FUN_1016d6f4 the server's and FUN_1016c84a's callers the client's; a group
//! is known by its owner's character id): the server sends 50 an invitation (`u32 owner`), 51 and 53 the end of the
//! membership (`u32 owner`), 55 and 56 a character joining and leaving (`u32 owner, u32 character`), 57 a line
//! (`u32 owner, u32 sender, str16 text, str16 blob`) and 58 an invitation declined (`u32 owner, u32 character`); the client
//! sends 50 to invite and 51 to kick (`u32 character`), 52 to join and 53 to leave or decline (`u32 owner`) and 57 a line
//! (`u32 owner, str16 text, str16 blob`). One invitation and its end are in a capture; the rest is from the decompile.
//!
//! Vicinity speech goes through the game connection, not this one; packets not decoded come out as [`Packet::Other`].

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Where the chat server was (the game names it in `0x43`; this is what the captures show).
pub const SERVER: &str = "chat.d1.funcom.com:7105";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    Salt(String),
    LoginOk,
    Name { id: u32, name: String },
    /// Anonymous vicinity speech (35): a name (empty for the system's lines), the text and the blob.
    Notice { name: String, text: String, blob: Vec<u8> },
    /// Vicinity speech (34): the speaker, the text and the blob (which may carry a voice line, GUI.dll's FUN_10086075).
    Vicinity { sender: u32, text: String, blob: Vec<u8> },
    Channel { kind: u8, id: u32, name: String, flags: u32 },
    /// A channel the character is no longer in (61: its kind and id), as the team's when the team is left.
    ChannelGone { kind: u8, id: u32 },
    /// A private message (30). Its blob begins with what it is (GUI.dll's `HandlePrivateMessage` 0x10087e59): 0 a
    /// tell, [`AWAY_REPLY`] the automatic answer of a character that is away.
    Tell { sender: u32, text: String, blob: Vec<u8> },
    Message { channel: (u8, u32), sender: u32, text: String, blob: Vec<u8> },
    /// A friend: its id, whether it is online, and the type string the client gave it (a byte 1 for a friend).
    Buddy { id: u32, online: bool, kind: String },
    /// An invitation to a character's private chat group (50).
    PrivateInvite { owner: u32 },
    /// The end of the client's membership of a private group (51 kicked, 53 left).
    PrivateEnded { owner: u32 },
    /// A character joined a private group the client is in (55), or left it (56).
    PrivateJoined { owner: u32, character: u32 },
    PrivateParted { owner: u32, character: u32 },
    /// A line in a private group (57).
    PrivateMessage { owner: u32, sender: u32, text: String, blob: Vec<u8> },
    /// A character declined the client's invitation (58).
    PrivateRefused { owner: u32, character: u32 },
    Other { kind: u16, data: Vec<u8> },
}

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.data.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }
    /// A `u16` length and the bytes (FUN_10171b9b's `D`).
    fn blob(&mut self) -> Option<Vec<u8>> {
        let n = u16::from_be_bytes(self.take(2)?.try_into().ok()?) as usize;
        Some(self.take(n)?.to_vec())
    }
    fn text(&mut self) -> Option<String> {
        let n = u16::from_be_bytes(self.take(2)?.try_into().ok()?) as usize;
        Some(String::from_utf8_lossy(self.take(n)?).trim_end_matches('\0').to_string())
    }
    /// A text as it is (a buddy's kind is a byte: 0 temporary, 1 a friend).
    fn raw_text(&mut self) -> Option<String> {
        let n = u16::from_be_bytes(self.take(2)?.try_into().ok()?) as usize;
        Some(String::from_utf8_lossy(self.take(n)?).to_string())
    }
}

/// A server packet's meaning.
pub fn parse(kind: u16, data: &[u8]) -> Packet {
    let mut c = Cursor { data, at: 0 };
    let parsed = (|| -> Option<Packet> {
        Some(match kind {
            0 => Packet::Salt(c.text()?),
            5 => Packet::LoginOk,
            20 | 21 => Packet::Name { id: c.u32()?, name: c.text()? },
            34 => {
                let (sender, text) = (c.u32()?, c.text()?);
                Packet::Vicinity { sender, text, blob: c.blob().unwrap_or_default() }
            }
            35 => {
                let (name, text) = (c.text()?, c.text()?);
                Packet::Notice { name, text, blob: c.blob().unwrap_or_default() }
            }
            60 => Packet::Channel { kind: c.u8()?, id: c.u32()?, name: c.text()?, flags: c.u32().unwrap_or(0) },
            61 => Packet::ChannelGone { kind: c.u8()?, id: c.u32()? },
            40 => Packet::Buddy { id: c.u32()?, online: c.u32()? != 0, kind: c.raw_text()? },
            30 => {
                let (sender, text) = (c.u32()?, c.text()?);
                Packet::Tell { sender, text, blob: c.blob().unwrap_or_default() }
            }
            50 => Packet::PrivateInvite { owner: c.u32()? },
            51 | 53 => Packet::PrivateEnded { owner: c.u32()? },
            55 => Packet::PrivateJoined { owner: c.u32()?, character: c.u32()? },
            56 => Packet::PrivateParted { owner: c.u32()?, character: c.u32()? },
            57 => {
                let (owner, sender, text) = (c.u32()?, c.u32()?, c.text()?);
                Packet::PrivateMessage { owner, sender, text, blob: c.blob().unwrap_or_default() }
            }
            58 => Packet::PrivateRefused { owner: c.u32()?, character: c.u32()? },
            65 => {
                let (channel, sender, text) = ((c.u8()?, c.u32()?), c.u32()?, c.text()?);
                Packet::Message { channel, sender, text, blob: c.blob().unwrap_or_default() }
            }
            _ => return None,
        })
    })();
    parsed.unwrap_or_else(|| Packet::Other { kind, data: data.to_vec() })
}

/// The client's login packet.
pub fn login_packet(character: u32, user: &str, response: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&0u32.to_be_bytes());
    payload.extend_from_slice(&character.to_be_bytes());
    for s in [user, response] {
        payload.extend_from_slice(&(s.len() as u16).to_be_bytes());
        payload.extend_from_slice(s.as_bytes());
    }
    let mut packet = vec![0, 0];
    packet.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    packet.extend(payload);
    packet
}

fn framed(kind: u16, payload: Vec<u8>) -> Vec<u8> {
    let mut packet = kind.to_be_bytes().to_vec();
    packet.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    packet.extend(payload);
    packet
}

fn str16(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Asks the server for a character's id by name (the answer is a [`Packet::Name`]).
pub fn lookup_packet(name: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    str16(&mut payload, name);
    framed(21, payload)
}

/// Adds a character to the friends list (the server then tells of its logins and logouts): for good, or `temporary`
/// (kind 0: the original's "Recent Messages", FUN_100a6487) till the session ends.
pub fn buddy_add_packet(id: u32, temporary: bool) -> Vec<u8> {
    let mut payload = id.to_be_bytes().to_vec();
    str16(&mut payload, if temporary { "\u{0}" } else { "\u{1}" });
    framed(40, payload)
}

/// Whether a buddy's kind ([`Packet::Buddy`]) is the temporary one: a single zero byte (GUI.dll FUN_100a6581).
pub fn buddy_is_temporary(kind: &str) -> bool {
    kind == "\u{0}"
}

/// Invites a character to the player's private chat group (GUI.dll FUN_1016d595: packet 50, the character's id).
pub fn private_invite_packet(id: u32) -> Vec<u8> {
    framed(50, id.to_be_bytes().to_vec())
}

/// Puts a character out of the player's private chat group (51, the character's id).
pub fn private_kick_packet(id: u32) -> Vec<u8> {
    framed(51, id.to_be_bytes().to_vec())
}

/// A private chat group's request as the chat commands number them (GUI.dll FUN_100bacca and its two neighbours): 3
/// an invitation into the player's group, 4 someone put out of it, 5 the group of the character named left.
pub fn private_request_packet(what: u8, id: u32) -> Vec<u8> {
    match what {
        3 => private_invite_packet(id),
        4 => private_kick_packet(id),
        _ => private_leave_packet(id),
    }
}

/// Takes a character off the friends list.
pub fn buddy_remove_packet(id: u32) -> Vec<u8> {
    framed(41, id.to_be_bytes().to_vec())
}

/// The first byte of the blob of a tell that is the automatic answer of a character away from the keyboard (GUI.dll's
/// `HandlePrivateMessage` 0x10087e59 queues it with the kind 6, and FUN_10089940 sends the kind as the blob's first
/// byte): the client that gets it shows it with no name in front and does not answer it.
pub const AWAY_REPLY: u8 = 6;

/// A tell to the character with this id. Its blob is one byte, 0: what the tell is (FUN_10089940 puts the queued
/// message's kind there, and a voice line's record after it when there is one). The original shows only tells whose
/// blob begins with 0, 2 or 6 (`HandlePrivateMessage`).
pub fn tell_packet(target: u32, text: &str) -> Vec<u8> {
    tell_of_kind(target, text, 0)
}

/// The automatic answer to a tell that came while away ([`AWAY_REPLY`]).
pub fn away_reply_packet(target: u32, text: &str) -> Vec<u8> {
    tell_of_kind(target, text, AWAY_REPLY)
}

fn tell_of_kind(target: u32, text: &str, kind: u8) -> Vec<u8> {
    let mut payload = target.to_be_bytes().to_vec();
    str16(&mut payload, text);
    payload.extend_from_slice(&[0, 1, kind]);
    framed(30, payload)
}

/// Joins the private chat group of the character who invited (52).
pub fn private_join_packet(owner: u32) -> Vec<u8> {
    framed(52, owner.to_be_bytes().to_vec())
}

/// Leaves a private chat group, or declines an invitation to it (53).
pub fn private_leave_packet(owner: u32) -> Vec<u8> {
    framed(53, owner.to_be_bytes().to_vec())
}

/// A line to a private chat group (57), laid out as a tell is.
pub fn private_message_packet(owner: u32, text: &str) -> Vec<u8> {
    let mut payload = owner.to_be_bytes().to_vec();
    str16(&mut payload, text);
    str16(&mut payload, "\u{0}");
    framed(57, payload)
}

/// A line to a channel (the kind and id of its [`Packet::Channel`]).
pub fn channel_packet(kind: u8, id: u32, text: &str) -> Vec<u8> {
    let mut payload = vec![kind];
    payload.extend_from_slice(&id.to_be_bytes());
    str16(&mut payload, text);
    str16(&mut payload, " ");
    framed(65, payload)
}

/// A channel message with its blob (a voice line's record: GUI.dll's FUN_1008a2c1 sends the group message with FUN_10089628's
/// record as the blob, `ppj` FUN_1016ca0c).
pub fn channel_packet_with_blob(kind: u8, id: u32, text: &str, blob: &[u8]) -> Vec<u8> {
    let mut payload = vec![kind];
    payload.extend_from_slice(&id.to_be_bytes());
    str16(&mut payload, text);
    payload.extend_from_slice(&(blob.len() as u16).to_be_bytes());
    payload.extend_from_slice(blob);
    framed(65, payload)
}

/// A character written by its number, `&#39;`, at the start of `text`: the character and how many bytes it took.
/// Utils.dll's `String::GetUCChar` (FUN_1000db51, which the game's markup reader goes through): `&#`, a digit, then
/// digits up to a `;` that comes before the thirteenth byte; the number is `atol`'s, decimal only, and one of 0 is no
/// character. The bots' pages write an apostrophe so, their links being quoted with it.
pub fn numbered_character(text: &str) -> Option<(char, usize)> {
    let b = text.as_bytes();
    if b.len() < 4 || b[0] != b'&' || b[1] != b'#' || !b[2].is_ascii_digit() {
        return None;
    }
    let end = (3..0xd.min(b.len())).find(|&k| !b[k].is_ascii_digit())?;
    if b[end] != b';' {
        return None;
    }
    let number: u32 = text[2..end].parse().ok()?;
    char::from_u32(number).filter(|c| *c != '\0').map(|c| (c, end + 1))
}

/// A chat line as plain text: the markup (`<font ...>`, `<a href=...>`) removed, the common entities decoded, a line
/// break or a tab made a space. A tag ends at the first `>` outside quotes, as the game's own reader has it
/// (`HTMLParser_c::_ParseTag`): a link's target may be a whole page of markup (`<a href="text://...">`, what a bot
/// answers with), tags and line breaks and all, and none of that is the line's text. A `<` that no `>` closes is text.
pub fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let mut quote = None;
        let end = rest[open + 1..].char_indices().find_map(|(i, c)| {
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == '"' || c == '\'' => quote = Some(c),
                None if c == '>' => return Some(open + 1 + i),
                None => {}
            }
            None
        });
        match end {
            Some(end) => rest = &rest[end + 1..],
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    // (the characters written by number, `numbered_character`)
    let mut numbered = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(at) = rest.find("&#") {
        numbered.push_str(&rest[..at]);
        match numbered_character(&rest[at..]) {
            Some((c, len)) => {
                numbered.push(c);
                rest = &rest[at + len..];
            }
            None => {
                numbered.push_str("&#");
                rest = &rest[at + 2..];
            }
        }
    }
    numbered.push_str(rest);
    let out: String = numbered.chars().map(|c| if matches!(c, '\n' | '\r' | '\t') { ' ' } else { c }).collect();
    out.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&")
}

/// A blocking connection to the chat server.
pub struct Chat {
    stream: TcpStream,
    buffer: Vec<u8>,
    timeout: Duration,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Closed,
    Refused(String),
    Random(getrandom::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "chat network: {e}"),
            Error::Closed => write!(f, "the chat server closed the connection"),
            Error::Refused(t) => write!(f, "the chat server refused the login: {t}"),
            Error::Random(e) => write!(f, "random numbers: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl Chat {
    pub fn connect(address: impl ToSocketAddrs) -> Result<Self, Error> {
        let stream = TcpStream::connect(address)?;
        crate::socket::prepare(&stream)?;
        Ok(Chat { stream, buffer: Vec::new(), timeout: Duration::from_secs(20) })
    }

    /// Logs the character in (the salt arrives first) and waits for the accept.
    pub fn login(&mut self, character: u32, user: &str, password: &str) -> Result<(), Error> {
        let Packet::Salt(salt) = self.receive()? else { return Err(Error::Refused("no salt".into())) };
        let response = crate::crypto::response(user, &salt, password).map_err(Error::Random)?;
        crate::socket::write_all(&mut self.stream, &login_packet(character, user, &response))?;
        match self.receive()? {
            Packet::LoginOk => Ok(()),
            other => Err(Error::Refused(format!("{other:?}"))),
        }
    }

    /// Sends a packet built by one of the functions above.
    pub fn send(&mut self, packet: &[u8]) -> Result<(), Error> {
        Ok(crate::socket::write_all(&mut self.stream, packet)?)
    }

    /// How long `receive` waits before failing with an io timeout (the connection stays usable).
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), Error> {
        self.timeout = timeout;
        Ok(())
    }

    /// The next packet, waiting for it.
    pub fn receive(&mut self) -> Result<Packet, Error> {
        loop {
            if self.buffer.len() >= 4 {
                let len = u16::from_be_bytes([self.buffer[2], self.buffer[3]]) as usize;
                if self.buffer.len() >= 4 + len {
                    let kind = u16::from_be_bytes([self.buffer[0], self.buffer[1]]);
                    let packet = parse(kind, &self.buffer[4..4 + len]);
                    self.buffer.drain(..4 + len);
                    return Ok(packet);
                }
            }
            let mut chunk = [0u8; 4096];
            let n = crate::socket::read(&mut self.stream, &mut chunk, self.timeout)?;
            if n == 0 {
                return Err(Error::Closed);
            }
            self.buffer.extend_from_slice(&chunk[..n]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn the_captured_packets() {
        assert_eq!(parse(0, &hex("00206135613138396266356230313232376564343966363064356339306632356462")), Packet::Salt("a5a189bf5b01227ed49f60d5c90f25db".into()));
        assert_eq!(parse(20, &hex("630193db00084469616265746535")), Packet::Name { id: 0x630193db, name: "Diabete5".into() });
        assert_eq!(parse(35, &hex("0000001857656c636f6d6520746f20416e61726368794f6e6c696e650000")), Packet::Notice { name: String::new(), text: "Welcome to AnarchyOnline".into(), blob: Vec::new() });
        assert_eq!(parse(34, &hex("0000c35000026869000201ff")), Packet::Vicinity { sender: 0xc350, text: "hi".into(), blob: vec![1, 0xff] });
        assert_eq!(parse(60, &hex("87028f0211000d4f54204765726d616e204f4f43000080440000")), Packet::Channel { kind: 0x87, id: 0x028f0211, name: "OT German OOC".into(), flags: 0x8044 });
        assert_eq!(parse(60, &hex("0a000000020014546f77657220426174746c65204f7574636f6d65000080060000")), Packet::Channel { kind: 0x0a, id: 2, name: "Tower Battle Outcome".into(), flags: 0x8006 });
        // (the team's channel going, as the captures have it: five bytes)
        assert_eq!(parse(61, &hex("8200012345")), Packet::ChannelGone { kind: 0x82, id: 0x12345 });
        assert_eq!(parse(40, &[0, 9, 33, 53, 0, 0, 0, 0, 0, 1, 0]), Packet::Buddy { id: 598325, online: false, kind: "\u{0}".into() });
        assert!(buddy_is_temporary("\u{0}") && !buddy_is_temporary("\u{1}") && !buddy_is_temporary(""));
        assert_eq!(buddy_add_packet(7, true)[4..], [0, 0, 0, 7, 0, 1, 0]);
        assert_eq!(private_invite_packet(7)[4..], [0, 0, 0, 7]);
        // (private groups: an invitation as a capture has it, the others as the original reads them)
        assert_eq!(parse(50, &hex("1391ed82")), Packet::PrivateInvite { owner: 0x1391ed82 });
        assert_eq!(parse(51, &hex("1391ed82")), Packet::PrivateEnded { owner: 0x1391ed82 });
        assert_eq!(parse(55, &hex("0000000700000009")), Packet::PrivateJoined { owner: 7, character: 9 });
        assert_eq!(parse(56, &hex("0000000700000009")), Packet::PrivateParted { owner: 7, character: 9 });
        assert_eq!(parse(57, &hex("00000007000000090002686900010000")), Packet::PrivateMessage { owner: 7, sender: 9, text: "hi".into(), blob: vec![0] });
        assert_eq!(parse(58, &hex("0000000700000009")), Packet::PrivateRefused { owner: 7, character: 9 });
        assert_eq!((private_join_packet(7)[..2].to_vec(), private_leave_packet(7)[..2].to_vec()), (vec![0, 52], vec![0, 53]));
        assert_eq!(private_message_packet(7, "hi")[..2], [0, 57]);
        assert_eq!(private_message_packet(7, "hi")[4..], [0, 0, 0, 7, 0, 2, b'h', b'i', 0, 1, 0]);
        assert_eq!(parse(40, &[0, 9, 33, 53, 0, 0, 0, 1, 0, 1, 1]), Packet::Buddy { id: 598325, online: true, kind: "\u{1}".into() });
        assert_eq!(parse(5, &[]), Packet::LoginOk);
        assert!(matches!(parse(99, &[1, 2]), Packet::Other { kind: 99, .. }));
    }

    #[test]
    fn the_sent_packets_are_framed() {
        assert_eq!(lookup_packet("Abc"), hex("0015000500034162 63".replace(' ', "").as_str()));
        let tell = tell_packet(0x630193db, "hi");
        assert_eq!(&tell[..4], &hex("001e000b")[..]);
        assert_eq!(&tell[4..8], &hex("630193db")[..]);
        assert_eq!(&tell[8..], &hex("00026869000100")[..]);
        assert_eq!(&away_reply_packet(0x630193db, "hi")[8..], &hex("00026869000106")[..]);
        let line = channel_packet(0x87, 0x028f0211, "yo");
        assert_eq!(&line[..4], &hex("0041000c")[..]);
        assert_eq!(&line[4..9], &hex("87028f0211")[..]);
        assert_eq!(parse(30, &hex("630193db0002686900010000")), Packet::Tell { sender: 0x630193db, text: "hi".into(), blob: vec![0] });
        assert_eq!(parse(30, &hex("630193db00026869000106")), Packet::Tell { sender: 0x630193db, text: "hi".into(), blob: vec![AWAY_REPLY] });
        assert_eq!(parse(21, &hex("000003e900034162 63".replace(' ', "").as_str())), Packet::Name { id: 1001, name: "Abc".into() });
    }

    #[test]
    fn friends_are_added_and_removed() {
        assert_eq!(buddy_add_packet(598325, false), hex("0028000700092135000101"));
        assert_eq!(buddy_remove_packet(598325), hex("0029000400092135"));
    }

    #[test]
    fn markup_is_stripped() {
        assert_eq!(plain("<font color='#89D2E8'><font color=#FFFF00>Players Online (1)</font></font> a &amp; b"), "Players Online (1) a & b");
        // (a bot's answer: a link whose target is a page of markup with tags, line breaks and links of its own)
        let page = "<a href=\"text://<font color=#ffff00>Buffs</font>\n<a href='chatcmd:///tell Bot cast 1'>One</a>\r\nmore\">Buff list</a> for\nyou";
        assert_eq!(plain(page), "Buff list for you");
        assert_eq!(plain("2 < 3 and so on"), "2 < 3 and so on");
        // (a character by its number: decimal, a digit first, the `;` before the thirteenth byte, not 0)
        assert_eq!(plain("Mocham&#39;s Gift &#8217; &amp;#39; &#x27; &#; &#0; &#39 &#12345678901;"), "Mocham's Gift \u{2019} &#39; &#x27; &#; &#0; &#39 &#12345678901;");
        assert_eq!((numbered_character("&#39;s"), numbered_character("&#0000000039;"), numbered_character("&#00000000039;")), (Some(('\'', 5)), Some(('\'', 13)), None));
    }

    #[test]
    fn the_login_packet_is_framed() {
        let p = login_packet(0x630193db, "derived", "abc");
        assert_eq!(&p[..2], &[0, 0]);
        assert_eq!(u16::from_be_bytes([p[2], p[3]]) as usize, p.len() - 4);
        assert_eq!(&p[8..12], &hex("630193db")[..]);
    }
}
