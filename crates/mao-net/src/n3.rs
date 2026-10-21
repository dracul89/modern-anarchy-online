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
/// Movement types of a [`POSITION_UPDATE`] (guessed from when the original
/// client sends them in the capture).
pub mod movement {
    pub const FORWARD_START: u8 = 1;
    pub const FORWARD_STOP: u8 = 2;
    pub const BACKWARD_START: u8 = 3;
    pub const BACKWARD_STOP: u8 = 4;
    pub const TURN_LEFT_START: u8 = 9;
    pub const TURN_RIGHT_START: u8 = 11;
    pub const TURN_STOP: u8 = 12;
    /// A new heading while standing.
    pub const HEADING: u8 = 14;
    /// The periodic update while moving.
    pub const UPDATE: u8 = 15;
}

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

    /// The rotation of an [`ENTITY_INFO`] (a quaternion, x y z w), found
    /// where the captures put it, after the position.
    pub fn rotation(&self) -> Option<[f32; 4]> {
        if self.kind != ENTITY_INFO || self.data.len() < 38 {
            return None;
        }
        let f = |at: usize| f32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        Some([f(22), f(26), f(30), f(34)])
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

/// The client's position update: `movement` type, the heading (radians
/// about Y; 0 faces +z), the position, and the milliseconds since the last
/// update.
pub fn position_update(character: u32, movement: u8, heading: f32, position: [f32; 3], elapsed_ms: u32) -> Frame {
    let (s, c) = (heading * 0.5).sin_cos();
    let mut d = vec![1, movement];
    for v in [0.0, s, 0.0, c, position[0], position[1], position[2]] {
        d.extend_from_slice(&f32::to_be_bytes(v));
    }
    d.extend_from_slice(&elapsed_ms.to_be_bytes());
    d.extend_from_slice(&[0; 8]);
    Message { kind: POSITION_UPDATE, identity: (CHARACTER, character), data: d }.frame(character)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The client's first update in the capture.
    #[test]
    fn update_matches_the_capture() {
        let hex = "541111230000c3500009210c010c000000003f741240000000003e9a782c43f6c8ec410028f643dfae6e00001ded0000000000000000";
        let bytes: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        let m = Message { kind: POSITION_UPDATE, identity: (CHARACTER, 0x9210c), data: bytes[12..].to_vec() };
        let (kind, q, p) = m.movement().unwrap();
        assert_eq!(kind, 12);
        let heading = 2.0 * q[1].atan2(q[3]);
        let ours = position_update(0x9210c, 12, heading, p, 7661);
        assert_eq!(ours.body.len(), bytes.len());
        let mine = Message::parse(&ours).unwrap();
        let (_, q2, p2) = mine.movement().unwrap();
        assert!((q2[1] - q[1]).abs() < 1e-5 && (q2[3] - q[3]).abs() < 1e-5);
        assert_eq!(p2, p);
        assert_eq!(mine.data[30..34], 7661u32.to_be_bytes());
    }
}
