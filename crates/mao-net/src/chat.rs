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
//! * 35: `u32 length, text, 2 bytes` a system line ("Welcome to AnarchyOnline");
//! * 40: `u32 channel id, 7 more bytes` (a channel membership flag, not decoded);
//! * 60: a channel: `u8 kind, u32 id, str16 name, 6 bytes flags`;
//! * 30: `u32 channel id, str16 text, ...` a line in a channel from the server (the "Players Online" notice);
//! * 65: `u8 kind, u32 id (the channel), u32 sender, str16 text, ...` a channel message.
//!
//! Vicinity, tell and friend packets are not in a capture yet: they come out as [`Packet::Other`].

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Where the chat server was (the game names it in `0x43`; this is what the captures show).
pub const SERVER: &str = "chat.d1.funcom.com:7105";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    Salt(String),
    LoginOk,
    Name { id: u32, name: String },
    Notice { text: String },
    Channel { kind: u8, id: u32, name: String },
    ChannelLine { id: u32, text: String },
    Message { channel: (u8, u32), sender: u32, text: String },
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
    fn text(&mut self) -> Option<String> {
        let n = u16::from_be_bytes(self.take(2)?.try_into().ok()?) as usize;
        Some(String::from_utf8_lossy(self.take(n)?).trim_end_matches('\0').to_string())
    }
}

/// A server packet's meaning.
pub fn parse(kind: u16, data: &[u8]) -> Packet {
    let mut c = Cursor { data, at: 0 };
    let parsed = (|| -> Option<Packet> {
        Some(match kind {
            0 => Packet::Salt(c.text()?),
            5 => Packet::LoginOk,
            20 => Packet::Name { id: c.u32()?, name: c.text()? },
            35 => {
                let n = c.u32()? as usize;
                Packet::Notice { text: String::from_utf8_lossy(c.take(n)?).to_string() }
            }
            60 => Packet::Channel { kind: c.u8()?, id: c.u32()?, name: c.text()? },
            30 => Packet::ChannelLine { id: c.u32()?, text: c.text()? },
            65 => Packet::Message { channel: (c.u8()?, c.u32()?), sender: c.u32()?, text: c.text()? },
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

/// A chat line as plain text: the markup (`<font ...>`, `<a href=...>`) removed, the common entities decoded.
pub fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for ch in text.chars() {
        match ch {
            '<' => tag = true,
            '>' if tag => tag = false,
            _ if !tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&")
}

/// A blocking connection to the chat server.
pub struct Chat {
    stream: TcpStream,
    buffer: Vec<u8>,
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
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(20)))?;
        Ok(Chat { stream, buffer: Vec::new() })
    }

    /// Logs the character in (the salt arrives first) and waits for the accept.
    pub fn login(&mut self, character: u32, user: &str, password: &str) -> Result<(), Error> {
        let Packet::Salt(salt) = self.receive()? else { return Err(Error::Refused("no salt".into())) };
        let response = crate::crypto::response(user, &salt, password).map_err(Error::Random)?;
        self.stream.write_all(&login_packet(character, user, &response))?;
        match self.receive()? {
            Packet::LoginOk => Ok(()),
            other => Err(Error::Refused(format!("{other:?}"))),
        }
    }

    /// How long `receive` waits before failing with an io timeout (the connection stays usable).
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), Error> {
        Ok(self.stream.set_read_timeout(Some(timeout))?)
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
            let n = self.stream.read(&mut chunk)?;
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
        assert_eq!(parse(35, &hex("0000001857656c636f6d6520746f20416e61726368794f6e6c696e650000")), Packet::Notice { text: "Welcome to AnarchyOnline".into() });
        assert_eq!(parse(60, &hex("87028f0211000d4f54204765726d616e204f4f43000080440000")), Packet::Channel { kind: 0x87, id: 0x028f0211, name: "OT German OOC".into() });
        assert_eq!(parse(60, &hex("0a000000020014546f77657220426174746c65204f7574636f6d65000080060000")), Packet::Channel { kind: 0x0a, id: 2, name: "Tower Battle Outcome".into() });
        assert_eq!(parse(5, &[]), Packet::LoginOk);
        assert!(matches!(parse(99, &[1, 2]), Packet::Other { kind: 99, .. }));
    }

    #[test]
    fn markup_is_stripped() {
        assert_eq!(plain("<font color='#89D2E8'><font color=#FFFF00>Players Online (1)</font></font> a &amp; b"), "Players Online (1) a & b");
    }

    #[test]
    fn the_login_packet_is_framed() {
        let p = login_packet(0x630193db, "derived", "abc");
        assert_eq!(&p[..2], &[0, 0]);
        assert_eq!(u16::from_be_bytes([p[2], p[3]]) as usize, p.len() - 4);
        assert_eq!(&p[8..12], &hex("630193db")[..]);
    }
}
