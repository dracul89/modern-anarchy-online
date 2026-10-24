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
    /// Jump (sent as the jump starts; the capture's heights rise about 2.5
    /// units and fall back in about a second).
    pub const JUMP: u8 = 22;
    /// A new heading while standing.
    pub const HEADING: u8 = 14;
    /// The periodic update while moving.
    pub const UPDATE: u8 = 15;
}

/// The server's announcement of the playfield the character is in (at login
/// and after zoning): the identity's instance is the playfield number and
/// the data holds the arrival position.
pub const PLAYFIELD: u32 = 0x5f4b1a39;

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

/// The action a new connection starts with (`0x57`, sent after ready as the
/// original does): characters enter the world in a state where the server
/// ignores their movement, and this frees them.
pub const STAND: u8 = 0x57;

/// `CharacterActionType.Logout` (named so in the AOSharp sources): the character sits and the
/// server starts a logout countdown (about 30 s) that standing up (`STAND`) cancels. The server
/// answers with a `0x7a` (StopLogout) broadcast. This is what the original sends at the end of a
/// session: players sit to log out.
pub const LOGOUT: u8 = 0x78;

/// A `5e477770` action message (the action byte after four zero bytes, 27
/// bytes of data).
pub fn action(character: u32, code: u8) -> Frame {
    let mut data = vec![0u8; 27];
    data[4] = code;
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// `5e477770` action `0x13`: cast a nano program (by its id) at a character, or at oneself when `target` is `None`. The layout
/// is from the captured casts: four zero bytes, the action, four zero bytes, the target identity (type, id; zeros for none), the
/// nano identity (type `0xcf1b`, id) and two zero bytes.
pub fn cast_nano(character: u32, target: Option<u32>, nano: u32) -> Frame {
    let mut data = vec![0u8; 5];
    data[4] = 0x13;
    data.extend_from_slice(&[0; 4]);
    let (kind, id) = target.map_or((0, 0), |t| (CHARACTER, t));
    for word in [kind, id, 0xcf1b, nano] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(&[0; 2]);
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// `ClientMoveItemToInventory` (`5469373f`): moves the item in inventory place `from` to place `to` (a wear slot or a place of the
/// backpack: wearing, taking off and rearranging). The captured moves are `(0x68, 0x59)` to `0x14` and `(0x68, 0x53)` to `1`: a
/// zero byte, the source identity (type `0x68` and the place) and the destination place.
pub fn move_item(character: u32, from: u32, to: u32) -> Frame {
    let mut data = vec![0u8];
    for word in [0x68, from, to] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind: 0x5469373f, identity: (CHARACTER, character), data }.frame(character)
}

/// `ClientMoveItemToInventory` for an item in a container the character used (a corpse, a bag): the source is `(0x6b, handle << 16
/// | place)`, the container's handle from its `InventoryUpdate` and the item's place in it, and the destination `0x6f` (any free
/// place). Checked against the captured loot of a corpse (handle `0x70`, places 1, 0, 3, 2).
pub fn take_from_container(character: u32, handle: u32, place: u32) -> Frame {
    let mut data = vec![0u8];
    for word in [0x6b, handle << 16 | (place & 0xffff), 0x6f] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind: 0x5469373f, identity: (CHARACTER, character), data }.frame(character)
}

/// `52526858`: use something (a door, a terminal, an item; talk to an NPC): action 3 on the
/// target's identity. `sequence` counts the character's uses from 1 (as the original's capture
/// shows); the server echoes the message back with the first byte 0.
pub fn use_object(character: u32, sequence: u32, target: (u32, u32)) -> Frame {
    let mut data = vec![1u8];
    for word in [0, sequence, 3, 1, CHARACTER, character, target.0, target.1] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind: 0x52526858, identity: (CHARACTER, character), data }.frame(character)
}

/// `Mail` (`333b2867`) asking a mail terminal for the inbox: action 1 and eight zero bytes (as the
/// original sends it, after using the terminal). The answer is a `Mail` with action 0 and a list
/// (empty in the capture: `00 00 00 | 00 00 03 f1`).
pub fn mail_request(character: u32) -> Frame {
    let mut data = vec![0u8, 0, 1];
    data.extend_from_slice(&[0; 8]);
    Message { kind: 0x333b2867, identity: (CHARACTER, character), data }.frame(character)
}

/// `Trade` (`36284f6e`): the header byte, an `i32` 2, an action byte (0 open, 1 accept, 2 decline, 3 confirm,
/// 4 complete, 5 add item, 6 remove item, 7 update credits) and four `i32` parameters. A shop is a trade with the
/// booth: the server opens it (parameters: the booth's identity, then the temporary bag's), the client closes it
/// with a decline (`0, 1`: as captured) and, from the AOSharp definitions, accepts with the booth's identity.
pub fn trade(character: u32, action: u8, params: [u32; 4]) -> Frame {
    let mut data = vec![0u8];
    data.extend_from_slice(&2u32.to_be_bytes());
    data.push(action);
    for p in params {
        data.extend_from_slice(&p.to_be_bytes());
    }
    Message { kind: 0x36284f6e, identity: (CHARACTER, character), data }.frame(character)
}

/// Closes a shop (or declines a trade): action 2 with parameters `0, 1`, as the original sends it.
pub fn trade_decline(character: u32) -> Frame {
    trade(character, 2, [0, 1, 0, 0])
}

/// `ClientContainerAddItem` (`1f4d5f7e`): puts the item at `source` into the container `target` (the
/// capture: an inventory item `(0x68, slot)` into a bag `(0xc749, id)`).
pub fn container_add_item(character: u32, target: (u32, u32), source: (u32, u32)) -> Frame {
    let mut data = vec![0u8];
    for word in [target.0, target.1, source.0, source.1] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind: 0x1f4d5f7e, identity: (CHARACTER, character), data }.frame(character)
}

/// The action and the number of messages in a `Mail` reply (`333b2867`, server to client): a header byte,
/// an `i16` action and an array counted the way the protocol counts (`(n + 1) * 0x3f1`). The captured
/// reply to an empty inbox is `00 | 00 00 | 00 00 03 f1`: action 0, no messages.
pub fn mail_count(data: &[u8]) -> Option<(i16, usize)> {
    let action = i16::from_be_bytes(data.get(1..3)?.try_into().ok()?);
    let marker = i32::from_be_bytes(data.get(3..7)?.try_into().ok()?);
    usize::try_from(marker / 0x3f1 - 1).ok().map(|n| (action, n))
}

/// `Mail` (`333b2867`) sending a mail: action 6, the recipient, subject and body (each with an `i16`
/// length), the attached item's identity (an inventory slot is `(0x68, slot)`; `(0, 0)` for none), the
/// credits and whether it goes express. Free accounts are refused ("Sending mail is limited to paying
/// customers"): the server answers with a `FormatFeedback`.
pub fn mail_send(character: u32, recipient: &str, subject: &str, body: &str, item: (u32, u32), credits: u32, express: bool) -> Frame {
    let mut data = vec![0u8, 0, 6];
    for text in [recipient, subject, body] {
        data.extend_from_slice(&(text.len() as u16).to_be_bytes());
        data.extend_from_slice(text.as_bytes());
    }
    data.extend_from_slice(&item.0.to_be_bytes());
    data.extend_from_slice(&item.1.to_be_bytes());
    data.extend_from_slice(&credits.to_be_bytes());
    data.push(express as u8);
    Message { kind: 0x333b2867, identity: (CHARACTER, character), data }.frame(character)
}

/// `QuestAlternative` (`5c436609`): asks a mission terminal for missions. The sliders are the
/// terminal window's seven (difficulty, good/bad, order/chaos, open/hidden, physical/mystical,
/// head-on/stealth, credits/xp; 255 leaves one to the server), `scope` is 1 for a solo booth and 2 for
/// a team one, `terminal` the terminal's identity (type 0xdac1). The server answers with the same
/// message holding the offered missions. (From the AOSharp definitions; not yet seen in a capture.)
pub fn quest_alternative(character: u32, sliders: [u8; 7], scope: u8, terminal: (u32, u32)) -> Frame {
    let mut data = vec![1u8, 4];
    data.extend_from_slice(&sliders);
    data.extend_from_slice(&0u32.to_be_bytes());
    data.push(scope);
    data.extend_from_slice(&terminal.0.to_be_bytes());
    data.extend_from_slice(&terminal.1.to_be_bytes());
    data.push(0);
    Message { kind: 0x5c436609, identity: (CHARACTER, character), data }.frame(character)
}

/// `CreateQuest` (`291f361b`): takes one of the offered missions (its identity from the offer).
pub fn create_quest(character: u32, mission: (u32, u32)) -> Frame {
    let mut data = vec![1u8];
    data.extend_from_slice(&mission.0.to_be_bytes());
    data.extend_from_slice(&mission.1.to_be_bytes());
    Message { kind: 0x291f361b, identity: (CHARACTER, character), data }.frame(character)
}

/// Select a target (an `0x69` action naming it): the server answers with its
/// name and level (`4d38242e`).
pub fn select_target(character: u32, target: u32) -> Frame {
    let mut data = vec![0u8; 27];
    data[4] = 0x69;
    data[9..13].copy_from_slice(&CHARACTER.to_be_bytes());
    data[13..17].copy_from_slice(&target.to_be_bytes());
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// `2252445f`: make `target` the current target (0 clears it).
pub fn set_target(character: u32, target: u32) -> Frame {
    let mut data = vec![0u8; 13];
    if target != 0 {
        data[1..5].copy_from_slice(&CHARACTER.to_be_bytes());
        data[5..9].copy_from_slice(&target.to_be_bytes());
        data[12] = 1;
    }
    Message { kind: 0x2252445f, identity: (CHARACTER, character), data }.frame(character)
}

/// `28494070`: attack the target (the server's auto-attack then runs).
pub fn attack(character: u32, target: u32) -> Frame {
    let mut data = vec![0u8; 10];
    data[1..5].copy_from_slice(&CHARACTER.to_be_bytes());
    data[5..9].copy_from_slice(&target.to_be_bytes());
    Message { kind: 0x28494070, identity: (CHARACTER, character), data }.frame(character)
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
    #[test]
    fn the_empty_inbox_reply_has_no_messages() {
        assert_eq!(mail_count(&[0, 0, 0, 0, 0, 3, 0xf1]), Some((0, 0)));
        assert_eq!(mail_count(&[0, 0, 0]), None);
    }

    #[test]
    fn trade_and_container_messages_match_the_capture() {
        let hex = |m: Frame| -> String { Message::parse(&m).unwrap().data.iter().map(|b| format!("{b:02x}")).collect() };
        // The captured close of a shop: unknown 2, decline, parameters 0, 1, 0, 0.
        assert_eq!(hex(trade_decline(0x4c2e2)), "00000000020200000000000000010000000000000000");
        // The captured move of the inventory item in slot 0x5a into the bag (0xc749, 0xbccf0).
        assert_eq!(hex(container_add_item(0x4c2de, (0xc749, 0xbccf0), (0x68, 0x5a))), "000000c749000bccf0000000680000005a");
        // The fifth capture: buying stock item 2 (add, then accept), and selling the item in slot 0x50.
        assert_eq!(hex(trade(0x1323e, 5, [0xc75b, 0xc006021c, 0x6f, 2])), "0000000002050000c75bc006021c0000006f00000002");
        assert_eq!(hex(cast_nano(0x4c2de, Some(0x7af69bd7), 0xaad6)), "0000000013000000000000c3507af69bd70000cf1b0000aad60000");
        assert_eq!(hex(cast_nano(0x4bc80, None, 0x43576)), "00000000130000000000000000000000000000cf1b000435760000");
        assert_eq!(hex(take_from_container(0x4bc80, 0x70, 1)), "000000006b007000010000006f");
        assert_eq!(hex(take_from_container(0x4bc80, 0x70, 0)), "000000006b007000000000006f");
        assert_eq!(hex(take_from_container(0x4bc7f, 0x7a, 1)), "000000006b007a00010000006f");
        assert_eq!(hex(move_item(0x4bc80, 0x53, 1)), "00000000680000005300000001");
        assert_eq!(hex(move_item(0x4c2de, 0x59, 0x14)), "00000000680000005900000014");
        assert_eq!(hex(trade(0x1323e, 1, [0; 4])), "00000000020100000000000000000000000000000000");
        assert_eq!(hex(trade(0x1323e, 5, [0xc350, 0x1323e, 0x68, 0x50])), "0000000002050000c3500001323e0000006800000050");
    }

    #[test]
    fn mail_matches_the_capture() {
        let hex = |m: Frame| -> String { Message::parse(&m).unwrap().data.iter().map(|b| format!("{b:02x}")).collect() };
        assert_eq!(hex(mail_request(0x4c2e2)), "0000010000000000000000");
        // The captured send: to "Muggles", subject "hi", no body, the item in inventory slot 0x44, express.
        assert_eq!(hex(mail_send(0x4c2e2, "Muggles", "hi", "", (0x68, 0x44), 0, true)), "00000600074d7567676c657300026869000000000068000000440000000001");
    }

    #[test]
    fn use_matches_the_capture() {
        // The client's first use in the capture: character 0x4c2de, target (0xc76a, 0x0101e30a).
        let frame = use_object(0x4c2de, 1, (0xc76a, 0x0101e30a));
        let hex: String = Message::parse(&frame).unwrap().data.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "01000000000000000100000003000000010000c3500004c2de0000c76a0101e30a");
    }

    use super::*;

    /// The client's first update in the capture.
    #[test]
    fn combat_messages_match_the_capture() {
        let hex = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect() };
        // Captured: select, target and attack a creature (7af69bd8) as character 0x4c2de.
        assert_eq!(select_target(0x4c2de, 0x7af69bd8).body, hex("5e4777700000c350 0004c2de 0000000069000000000000c3507af69bd800000000000000000000".replace(' ', "").as_str()));
        assert_eq!(set_target(0x4c2de, 0x7af69bd8).body, hex("2252445f0000c3500004c2de000000c3507af69bd800000001"));
        assert_eq!(set_target(0x4c2de, 0).body, hex("2252445f0000c3500004c2de00000000000000000000000000"));
        assert_eq!(attack(0x4c2de, 0x7af69bd8).body, hex("284940700000c3500004c2de000000c3507af69bd800"));
    }

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

/// What an [`ENTITY_INFO`] says about a character or creature, as far as
/// it is decoded. Derived from the reader of `SimpleCharFullUpdateIIR_t`
/// in `Gamecode.dll` (big-endian `BinaryStream`) and checked against a
/// capture.
///
/// After the identity: a byte 0 (header), a version byte (0x39 or 0x3a), a
/// flags `i32`, then, by the flags:
/// * bit 6: the playfield (`i32`);
/// * bit 5: an identity (not decoded here: a capture without it);
/// * always the position (three floats);
/// * bit 9: the rotation (four floats, x y z w);
/// * always a packed `i32`: three bits, two bits, breed (3 bits at 5), sex (2
///   bits at 8), two bits, five bits from bit 12;
/// * the name: a length byte (counting the NUL) and the bytes;
/// * `i32` flags 2, an `i32` of two 16-bit values, and (without flag bit 0)
///   an `i32` (not a model id), an `i32`, then seven `i16`s; the rest (stats,
///   textures, clothing and more) is not decoded.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityInfo {
    pub version: u8,
    pub flags: u32,
    pub playfield: Option<u32>,
    pub position: [f32; 3],
    pub rotation: Option<[f32; 4]>,
    pub breed: u8,
    pub sex: u8,
    pub name: String,
    pub flags2: u32,
    /// A number after the flags that varies widely per character (365 to
    /// 20838 in the capture; probably health). NOT a model id. Absent for
    /// creatures whose flags have bit 0 set.
    pub unknown_a0: Option<u32>,
    /// The texture overrides (flag bit 4): material name and the texture's
    /// instance (type 0xf6954). Found by scanning for the vector's marker,
    /// not by decoding the fields before it.
    pub textures: Vec<(String, u32)>,
    /// The worn clothes: body part name and the texture's instance (types
    /// 0xf6954, 0xf6960, 0xf6961 share instances). Found like `textures`.
    pub clothes: Vec<(String, u32)>,
    /// The NPC template (an object of type 0xfde97) of a creature or NPC:
    /// its record names the CAT mesh (entry 12) and the animations.
    pub template: Option<u32>,
}

/// Where the template id sits after the flags: a few bytes after the second
/// flags word and always before a level (`00 level 00 1f`), at a position
/// that moves by a byte with the fields in front.
fn npc_template(post: &[u8]) -> Option<u32> {
    (8..post.len().saturating_sub(7).min(40)).find_map(|p| {
        (post[p + 4] == 0 && post[p + 6] == 0 && post[p + 7] == 0x1f).then(|| u32::from_be_bytes(post[p..p + 4].try_into().unwrap()))
    })
}

impl EntityInfo {
    /// Whether it is dressed from the breed and sex tables: the characters
    /// (flag bit 0 clear) of a breed 1 to 4. Sex is the game's `Sex` stat:
    /// 1 neuter, 2 male, 3 female.
    pub fn is_player(&self) -> bool {
        self.unknown_a0.is_some() && (1..=4).contains(&self.breed)
    }

    /// Whether the breed and sex tables dress it (players and the human
    /// characters among the NPCs alike).
    pub fn is_humanoid(&self) -> bool {
        matches!((self.breed, self.sex), (1..=3, 2 | 3) | (4, 1))
    }
}

/// The texture list in the rest of an entity message: an `i32` of
/// `(count + 1) * 0x3f1`, then per texture a 32-byte NUL-padded name and
/// three `i32`s (the instance, a second number, a flag). Searched for byte by
/// byte: a marker for 1 to 12 entries followed by a plausible name.
fn texture_overrides(rest: &[u8]) -> Vec<(String, u32)> {
    for start in 0..rest.len().saturating_sub(48) {
        let marker = u32::from_be_bytes(rest[start..start + 4].try_into().unwrap());
        if marker % 0x3f1 != 0 || !(2..=13).contains(&(marker / 0x3f1)) {
            continue;
        }
        let count = (marker / 0x3f1 - 1) as usize;
        let mut at = start + 4;
        let mut out = Vec::new();
        for _ in 0..count {
            let Some(entry) = rest.get(at..at + 44) else { break };
            let name = &entry[..32];
            let end = name.iter().position(|&c| c == 0).unwrap_or(32);
            let plausible = end > 0 && name[..end].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_') && name[end..].iter().all(|&c| c == 0);
            if !plausible {
                out.clear();
                break;
            }
            out.push((String::from_utf8_lossy(&name[..end]).into_owned(), u32::from_be_bytes(entry[32..36].try_into().unwrap())));
            at += 44;
        }
        if out.len() == count && count > 0 {
            return out;
        }
    }
    Vec::new()
}

/// The body parts a cloth record's slot indexes (`ClothData_t::GetName`).
const CLOTH_PARTS: [&str; 6] = ["hands", "body", "feet", "arms", "legs", "head"];

/// The cloth list in the rest of an entity message: the same counted-vector
/// marker as the textures, then a record per slot 0, 1, 2...: an `i32` id (the
/// slot; its high 16 bits above 0 mean two more `i32`s follow the record), the
/// texture's instance and a second number. Slot 0 and above with a nonzero
/// instance are kept.
fn cloth_list(rest: &[u8]) -> Vec<(String, u32)> {
    let int = |at: usize| rest.get(at..at + 4).map(|b| i32::from_be_bytes(b.try_into().unwrap()));
    for start in 0..rest.len().saturating_sub(16) {
        let Some(marker) = int(start) else { break };
        if marker <= 0 || marker % 0x3f1 != 0 || !(2..=7).contains(&(marker / 0x3f1)) {
            continue;
        }
        let count = (marker / 0x3f1 - 1) as usize;
        let mut at = start + 4;
        let mut out = Vec::new();
        let mut valid = true;
        for slot in 0..count {
            let (Some(packed), Some(instance), Some(_)) = (int(at), int(at + 4), int(at + 8)) else {
                valid = false;
                break;
            };
            if packed & 0xffff != slot as i32 {
                valid = false;
                break;
            }
            at += 12;
            if (packed >> 16) > 0 {
                at += 8;
            }
            if instance > 0 {
                if let Some(part) = CLOTH_PARTS.get(slot) {
                    out.push((part.to_string(), instance as u32));
                }
            }
        }
        if valid && !out.is_empty() {
            return out;
        }
    }
    Vec::new()
}

impl Message {
    /// The playfield announced by a [`PLAYFIELD`] message and where the
    /// character arrives in it: after the identity, a zero word, a flags
    /// byte, the position (three floats) and a playfield proxy (`'a'` and six
    /// words). The playfield to draw is the proxy's second word: for an
    /// ordinary playfield the same as the identity's instance, for an
    /// instanced one (a shop interior) the template the instance is of.
    pub fn arrival(&self) -> Option<(u32, [f32; 3])> {
        if self.kind != PLAYFIELD || self.data.len() < 17 {
            return None;
        }
        let f = |at: usize| f32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        let template = (self.data.len() >= 26 && self.data[17] == b'a').then(|| u32::from_be_bytes(self.data[22..26].try_into().unwrap())).filter(|&t| t != 0);
        Some((template.unwrap_or(self.identity.1), [f(5), f(9), f(13)]))
    }

    /// The nano programs active on a character (`SimpleCharFullUpdate`'s `ActiveNanos`: a count word `(n + 1) * 0x3f1`, then
    /// per nano the identity (type 0xcf1b, the nano's id), a word, and two times). The list is not at a fixed place in the
    /// message, so it is found by its shape: a count word followed by `n` entries that each begin with the type 0xcf1b.
    pub fn active_nanos(&self) -> Vec<u32> {
        self.active_nano_times().into_iter().map(|(id, _)| id).collect()
    }

    /// The same with the time each nano has left, in milliseconds (the entry's first time; the second is the same at login).
    pub fn active_nano_times(&self) -> Vec<(u32, u32)> {
        const NANO_TYPE: u32 = 0xcf1b;
        if self.kind != ENTITY_INFO {
            return Vec::new();
        }
        let d = &self.data;
        let word = |at: usize| d.get(at..at + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
        for at in 0..d.len().saturating_sub(24) {
            let Some(count) = word(at).filter(|&c| c % 0x3f1 == 0 && c / 0x3f1 >= 2 && c / 0x3f1 <= 65) else { continue };
            let n = (count / 0x3f1 - 1) as usize;
            if (0..n).all(|i| word(at + 4 + 20 * i) == Some(NANO_TYPE)) {
                return (0..n).filter_map(|i| Some((word(at + 8 + 20 * i)?, word(at + 16 + 20 * i)?))).collect();
            }
        }
        Vec::new()
    }

    pub fn entity_info(&self) -> Option<EntityInfo> {
        if self.kind != ENTITY_INFO {
            return None;
        }
        let d = &self.data;
        let mut at = 1usize; // the header byte
        let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
            let b = d.get(*at..*at + n)?;
            *at += n;
            Some(b)
        };
        let version = take(&mut at, 1)?[0];
        if !matches!(version, 0x39 | 0x3a) {
            return None;
        }
        let u32_at = |at: &mut usize| take(at, 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
        let f32_at = |at: &mut usize| take(at, 4).map(|b| f32::from_be_bytes(b.try_into().unwrap()));
        let flags = u32_at(&mut at)?;
        let playfield = if flags & 0x40 != 0 { Some(u32_at(&mut at)?) } else { None };
        if flags & 0x20 != 0 {
            return None;
        }
        let position = [f32_at(&mut at)?, f32_at(&mut at)?, f32_at(&mut at)?];
        let rotation = if flags & 0x200 != 0 { Some([f32_at(&mut at)?, f32_at(&mut at)?, f32_at(&mut at)?, f32_at(&mut at)?]) } else { None };
        let packed = u32_at(&mut at)?;
        // The breed is the three bits at 5 (a Nanomage female packs 0x768, a Solitus female 0x729); the sex the two at 8.
        let (breed, sex) = (((packed >> 5) & 7) as u8, ((packed >> 8) & 3) as u8);
        let length = take(&mut at, 1)?[0] as usize;
        let text = take(&mut at, length)?;
        let name = String::from_utf8_lossy(text).trim_end_matches('\0').to_string();
        let flags2 = u32_at(&mut at)?;
        u32_at(&mut at)?;
        let model = if flags & 1 == 0 { u32_at(&mut at) } else { None };
        let tail = &d[at.min(d.len())..];
        let textures = if flags & 0x10 != 0 { texture_overrides(tail) } else { Vec::new() };
        let clothes = cloth_list(tail);
        let template = if flags & 1 != 0 { d.get(at - 8..).and_then(npc_template) } else { None };
        Some(EntityInfo { version, flags, playfield, position, rotation, breed, sex, name, flags2, unknown_a0: model, textures, clothes, template })
    }
}

#[cfg(test)]
mod entity_tests {
    use super::*;

    /// The template ids of creatures from the capture (the bytes after the name).
    #[test]
    fn creature_templates() {
        let h = |s: &str| -> Vec<u8> { s.split_whitespace().map(|x| u8::from_str_radix(x, 16).unwrap()).collect() };
        // Waste Collector, Draculeet, Female High Commander (the last is dressed by the tables)
        assert_eq!(npc_template(&h("10 08 12 01 00 00 00 00 4b 00 00 00 00 05 00 a1 00 00 45 32 00 5d 00 1f 00 00 00 00 1c")), Some(0x4532));
        assert_eq!(npc_template(&h("10 0a 12 01 00 00 00 00 24 00 00 00 00 01 00 14 00 00 03 ea 06 00 b4 00 1f 00 00 00 00 1c")), Some(0x3ea06));
        assert_eq!(npc_template(&h("10 08 12 01 00 00 00 00 01 00 00 00 00 c8 8e 52 00 00 00 65 ea 00 79 00 1f 00 00 00 00 1c")), Some(0x65ea));
    }

    fn bytes(hex: &str) -> Vec<u8> {
        let hex: String = hex.split_whitespace().collect();
        (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    /// The start of a monster's entity message from a capture.
    #[test]
    fn monster_from_capture() {
        let data = bytes(
            "00 3a 04 20 6a ca 00 00 02 1c 43 f5 f0 c7 41 00 28 f6 43 dc 31 c6 00 00 00 00 3f 38 be c4 00 00 00 00 3f 31 37 16 \
             00 00 07 41 09 4d 6f 70 68 69 6e 67 34 00 00 18 12 41 00 01 00 01 00 00 0f 7e 00 00 00 00",
        );
        let info = Message { kind: ENTITY_INFO, identity: (CHARACTER, 1933326420), data }.entity_info().unwrap();
        assert_eq!(info.name, "Mophing4");
        assert_eq!(info.playfield, Some(0x21c));
        assert!((info.position[0] - 491.9).abs() < 0.1 && (info.position[1] - 8.0).abs() < 0.1);
        // (Packed 0x741: bits 5-7 give 2, an Opifex; a Nanomage female packs 3 there, which the user's character confirmed.)
        assert_eq!((info.breed, info.sex), (2, 3));
        assert!(info.is_player());
        assert_eq!(info.unknown_a0, Some(0xf7e));
    }

    /// The character from the capture (a player: sex 2).
    #[test]
    fn player_from_capture() {
        let data = bytes(
            "00 3a 00 00 4a c8 00 00 02 1c 43 f6 c8 ec 41 00 28 f6 43 df ae 6e 00 00 00 00 3f 74 12 40 00 00 00 00 3e 9a 78 2f \
             00 00 06 29 0d 42 72 61 73 73 63 61 73 69 6e 67 73 00 00 18 12 41 00 01 00 01 00 00 01 6d 00 00 00 00",
        );
        let info = Message { kind: ENTITY_INFO, identity: (CHARACTER, 598284), data }.entity_info().unwrap();
        assert_eq!(info.name, "Brasscasings");
        assert_eq!((info.breed, info.sex), (1, 2));
        assert!(info.is_player());
        assert_eq!(info.unknown_a0, Some(365));
        assert!(info.textures.is_empty() || info.textures.iter().all(|t| !t.0.is_empty()));
    }
}

#[cfg(test)]
mod zoning_tests {
    use super::*;

    /// The announcement of West Athens from a capture.
    #[test]
    fn arrival_from_capture() {
        let data: Vec<u8> = [0u8, 0, 0, 0, 4, 0x44, 0x2f, 0xdf, 0xf2, 0x41, 0x20, 0x1d, 0x07, 0x43, 0xd4, 0x61, 0x42, 0x61, 0, 0, 0xc7, 0x9c, 0, 0, 2, 0x21].to_vec();
        let m = Message { kind: PLAYFIELD, identity: (0x9c50, 0x221), data };
        let (playfield, at) = m.arrival().unwrap();
        assert_eq!(playfield, 545);
        assert!((at[0] - 703.5).abs() < 0.01 && (at[1] - 10.007).abs() < 0.01 && (at[2] - 424.76).abs() < 0.01);
    }
}

#[cfg(test)]
mod instance_tests {
    use super::*;

    /// A shop interior: an instance (0x20d8c4) of playfield 0x1260.
    #[test]
    fn instanced_playfield_uses_its_template() {
        let mut data = vec![0u8, 0, 0, 0, 4];
        for v in [192.2f32, 9.06, 153.99] {
            data.extend_from_slice(&v.to_be_bytes());
        }
        data.push(b'a');
        for w in [0xc79eu32, 0x1260, 0, 0xc02b021c, 0x9c50, 0x20d8c4] {
            data.extend_from_slice(&w.to_be_bytes());
        }
        let m = Message { kind: PLAYFIELD, identity: (0x9c50, 0x20d8c4), data };
        assert_eq!(m.arrival().unwrap().0, 0x1260);
    }
}

#[cfg(test)]
mod nano_tests {
    use super::*;

    /// Two active nanos, shaped as in a captured update of a player.
    #[test]
    fn the_active_nanos_are_found() {
        let mut data = vec![0x00, 0x3a, 0x01, 0x00, 0x4a, 0xc8];
        data.extend_from_slice(&(3u32 * 0x3f1).to_be_bytes());
        for (nano, t) in [(0x27cbdu32, 0x13ad21u32), (0x23fddu32, 0x13a99du32)] {
            for w in [0xcf1bu32, nano, 0, t, t] {
                data.extend_from_slice(&w.to_be_bytes());
            }
        }
        data.extend_from_slice(&[0, 0, 0x17, 0xa6, 0, 0]);
        let m = Message { kind: ENTITY_INFO, identity: (0xc350, 1), data };
        assert_eq!(m.active_nanos(), vec![0x27cbd, 0x23fdd]);
        assert_eq!(m.active_nano_times(), vec![(0x27cbd, 0x13ad21), (0x23fdd, 0x13a99d)]);
        assert!(Message { kind: ENTITY_INFO, identity: (0xc350, 1), data: vec![0; 40] }.active_nanos().is_empty());
    }
}
