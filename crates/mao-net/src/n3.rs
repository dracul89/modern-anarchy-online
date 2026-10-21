//! N3 messages (frame kind 10): the game world's traffic. The body is a
//! `u32` message type, the identity of the object it concerns (a `u32`
//! type, 0xc350 for characters, and a `u32` instance) and the rest.
//!
//! Only what the captures show is decoded so far.

use crate::frame::{Frame, N3};

pub const CHARACTER: u32 = 0xc350;

/// A character or creature's description (its position among other
/// things; its name is at the end).
pub const ENTITY_INFO: u32 = 0x271b3a6b;
/// The client's own position update.
pub const POSITION_UPDATE: u32 = 0x54111123;
/// The first thing the client sends after entering.
pub const READY: u32 = 0x570c2039;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub kind: u32,
    pub identity: (u32, u32),
    pub data: Vec<u8>,
}

impl Message {
    pub fn parse(frame: &Frame) -> Option<Message> {
        if frame.kind != N3 || frame.body.len() < 12 {
            return None;
        }
        let long = |at: usize| u32::from_be_bytes(frame.body[at..at + 4].try_into().unwrap());
        Some(Message { kind: long(0), identity: (long(4), long(8)), data: frame.body[12..].to_vec() })
    }

    /// A frame from a character (`source`) to the server.
    pub fn frame(&self, source: u32) -> Frame {
        let mut body = self.kind.to_be_bytes().to_vec();
        body.extend_from_slice(&self.identity.0.to_be_bytes());
        body.extend_from_slice(&self.identity.1.to_be_bytes());
        body.extend_from_slice(&self.data);
        Frame { kind: N3, source, destination: 2, body }
    }

    /// The position of a [`POSITION_UPDATE`]: the movement type, the
    /// rotation (a quaternion, x y z w) and the position. The same message
    /// goes both ways (the server sends other characters' to the client).
    pub fn movement(&self) -> Option<(u8, [f32; 4], [f32; 3])> {
        if self.kind != POSITION_UPDATE || self.data.len() < 30 {
            return None;
        }
        let f = |at: usize| f32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        Some((self.data[1], [f(2), f(6), f(10), f(14)], [f(18), f(22), f(26)]))
    }

    /// The position of an [`ENTITY_INFO`]: `x y z` (the world is Y-up).
    /// Found by where the captures put it; the fields before it
    /// (a byte, a flag byte, a word, the playfield instance) aren't
    /// understood yet.
    pub fn position(&self) -> Option<[f32; 3]> {
        if self.kind != ENTITY_INFO || self.data.len() < 22 {
            return None;
        }
        let f = |at: usize| f32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        Some([f(10), f(14), f(18)])
    }
}

/// The client's first message after entering the world.
pub fn ready(character: u32) -> Frame {
    Message { kind: READY, identity: (CHARACTER, character), data: vec![1] }.frame(character)
}
