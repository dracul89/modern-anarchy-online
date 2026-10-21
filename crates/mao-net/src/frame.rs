//! Framing. Every message is a 16-byte header and a body, all big-endian:
//!
//! ```text
//! u16 sequence   counts up from 1 in each direction (0xdfdf for control packets)
//! u16 kind       1 system message, 5 text, 10 N3, 11 ping, 14 operator, 0x7f control
//! u16 1
//! u16 size       the whole message, header included, before padding
//! u32 source     (the message header's own words)
//! u32 destination
//! body
//! ```
//!
//! Without compression a message is padded with zeros to a multiple of four
//! bytes. A control packet (kind 0x7f) switches zlib compression on or off;
//! this reader doesn't handle compression yet and reports it as an error.

use std::fmt;

pub const HEADER: usize = 16;
pub const MAX_SIZE: usize = 0x20000;

pub const SYSTEM: u16 = 1;
pub const TEXT: u16 = 5;
pub const N3: u16 = 10;
pub const PING: u16 = 11;
pub const OPERATOR: u16 = 14;
/// A control packet: sequence 0xdfdf and (as the bytes `7f 00` read big-endian) this kind.
pub const CONTROL: u16 = 0x7f00;
pub const CONTROL_SEQUENCE: u16 = 0xdfdf;

#[derive(Debug)]
pub enum Error {
    /// A header whose size is outside what a connection accepts.
    BadSize(usize),
    /// A sequence number that doesn't grow.
    Sequence { last: u16, got: u16 },
    /// The compressed stream is damaged.
    Inflate(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadSize(n) => write!(f, "a message of {n} bytes"),
            Error::Sequence { last, got } => write!(f, "sequence number {got} after {last}"),
            Error::Inflate(e) => write!(f, "compressed data: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub kind: u16,
    pub source: u32,
    pub destination: u32,
    pub body: Vec<u8>,
}

impl Frame {
    /// The bytes to send, with sequence number `sequence`.
    pub fn encode(&self, sequence: u16) -> Vec<u8> {
        let size = HEADER + self.body.len();
        let mut out = Vec::with_capacity(size + 3);
        out.extend_from_slice(&sequence.to_be_bytes());
        out.extend_from_slice(&self.kind.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&(size as u16).to_be_bytes());
        out.extend_from_slice(&self.source.to_be_bytes());
        out.extend_from_slice(&self.destination.to_be_bytes());
        out.extend_from_slice(&self.body);
        out.resize(size.next_multiple_of(4), 0);
        out
    }
}

/// Gathers received bytes into messages. A control packet from the server
/// (sequence 0xdfdf, kind bytes `7f 00`) can switch zlib compression on:
/// everything after it is then one zlib stream (flushed after each send)
/// that holds the plain frames, unpadded.
#[derive(Default)]
pub struct Reader {
    buffer: Vec<u8>,
    inflate: Option<flate2::Decompress>,
    last: u16,
}

impl Reader {
    pub fn push(&mut self, bytes: &[u8]) -> Result<(), Error> {
        match &mut self.inflate {
            None => self.buffer.extend_from_slice(bytes),
            Some(z) => {
                let mut input = bytes;
                loop {
                    let room = input.len() * 8 + 65536;
                    self.buffer.reserve(room);
                    let (before_in, before_out) = (z.total_in(), self.buffer.len());
                    z.decompress_vec(input, &mut self.buffer, flate2::FlushDecompress::Sync).map_err(|e| Error::Inflate(e.to_string()))?;
                    input = &input[(z.total_in() - before_in) as usize..];
                    if input.is_empty() && self.buffer.len() - before_out < room {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// Whether the server has switched compression on.
    pub fn compressed(&self) -> bool {
        self.inflate.is_some()
    }

    /// The next whole message, if all of it has arrived. The body is cut
    /// to the header's size (the padding is dropped).
    pub fn next(&mut self) -> Result<Option<Frame>, Error> {
        loop {
            if self.buffer.len() < HEADER {
                return Ok(None);
            }
            let word = |at: usize| u16::from_be_bytes([self.buffer[at], self.buffer[at + 1]]);
            let size = word(6) as usize;
            if !(HEADER..=MAX_SIZE).contains(&size) {
                return Err(Error::BadSize(size));
            }
            let padded = if self.inflate.is_some() { size } else { size.next_multiple_of(4) };
            if self.buffer.len() < padded {
                return Ok(None);
            }
            let (sequence, kind) = (word(0), word(2));
            if sequence == CONTROL_SEQUENCE && kind == CONTROL {
                // The first of the two native 16-bit words after the
                // header is the compression level for what follows.
                let level = u16::from_le_bytes([self.buffer[8], self.buffer[9]]);
                let rest = self.buffer.split_off(16);
                self.buffer.clear();
                self.inflate = (level != 0).then(|| flate2::Decompress::new(true));
                self.push(&rest)?;
                continue;
            }
            if sequence <= self.last && !(self.last == 0xffff && sequence == 0) {
                return Err(Error::Sequence { last: self.last, got: sequence });
            }
            self.last = sequence;
            let long = |at: usize| u32::from_be_bytes(self.buffer[at..at + 4].try_into().unwrap());
            let frame = Frame { kind, source: long(8), destination: long(12), body: self.buffer[HEADER..size].to_vec() };
            self.buffer.drain(..padded);
            return Ok(Some(frame));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_with_padding() {
        let f = Frame { kind: SYSTEM, source: 0, destination: 1, body: vec![1, 2, 3, 4, 5, 6] };
        let bytes = f.encode(1);
        assert_eq!(bytes.len(), 24); // 22 bytes, padded to 24
        assert_eq!(&bytes[..8], &[0, 1, 0, 1, 0, 1, 0, 22]);
        let mut r = Reader::default();
        r.push(&bytes[..10]).unwrap();
        assert_eq!(r.next().unwrap(), None);
        r.push(&bytes[10..]).unwrap();
        assert_eq!(r.next().unwrap(), Some(f));
        assert_eq!(r.next().unwrap(), None);
    }

    /// The server's challenge from a capture of a real login: a 4-byte
    /// type and 32 characters.
    #[test]
    fn captured_challenge() {
        let mut bytes = vec![0, 1, 0, 1, 0, 1, 0, 0x34, 0, 0, 0, 1, 0, 0, 0xba, 0x20, 0, 0, 0, 0x24];
        bytes.extend_from_slice(b"75185ea0b3e792676b76b0741410e9d4");
        let mut r = Reader::default();
        r.push(&bytes).unwrap();
        let f = r.next().unwrap().unwrap();
        assert_eq!((f.kind, f.source, f.destination), (SYSTEM, 1, 0xba20));
        assert_eq!(&f.body[..4], &[0, 0, 0, 0x24]);
        assert_eq!(&f.body[4..], b"75185ea0b3e792676b76b0741410e9d4");
    }

    #[test]
    fn sequence_must_grow() {
        let f = Frame { kind: SYSTEM, source: 0, destination: 0, body: vec![0; 4] };
        let mut r = Reader::default();
        r.push(&f.encode(1)).unwrap();
        r.push(&f.encode(1)).unwrap();
        assert!(r.next().unwrap().is_some());
        assert!(matches!(r.next(), Err(Error::Sequence { .. })));
    }
}
