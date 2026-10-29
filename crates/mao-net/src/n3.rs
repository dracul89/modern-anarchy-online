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
/// Movement types of a [`POSITION_UPDATE`] (`Movement_n::MovementAction_e`): the start/stop and turn types from when the original
/// client sends them in the captures; `FULL` from `n3EngineClientAnarchy_t::CheckMotionUpdate` and `StartTeleportTry` in
/// Gamecode.dll, which send `MovementChanged(0x16)`.
pub mod movement {
    pub const FORWARD_START: u8 = 1;
    pub const FORWARD_STOP: u8 = 2;
    pub const BACKWARD_START: u8 = 3;
    pub const BACKWARD_STOP: u8 = 4;
    pub const TURN_LEFT_START: u8 = 9;
    pub const TURN_RIGHT_START: u8 = 11;
    pub const TURN_STOP: u8 = 12;
    /// The jump's start and its end (`JumpStartTransitionAction_t`, `JumpStopTransitionAction_t`: cases 0xf and 0x10 of the
    /// original's transition table, FUN_1006cb58; the captures show 15 as the height begins to rise).
    pub const JUMP_START: u8 = 15;
    pub const JUMP_STOP: u8 = 16;
    pub const JUMP: u8 = JUMP_START;
    /// The full position report: the original sends it after 5 s in motion without another movement message, when the heading
    /// turns more than 0.17 rad within 0.25 s, and at once to ask the server to zone (a teleport try, when the character touches
    /// a door or stands in a zone's teleportal).
    pub const FULL: u8 = 22;
    /// The periodic report while moving is the full one.
    pub const UPDATE: u8 = FULL;
    /// Switching the movement mode (cases 0x17..0x1d: frozen, walk, run, swim, crawl, sneak, fly).
    pub const WALK_MODE: u8 = 0x18;
    pub const RUN_MODE: u8 = 0x19;
    /// A new heading while standing.
    pub const HEADING: u8 = 14;
    /// Sitting down on the ground and getting up (`SwitchToSitGroundModeTransitionAction_t`, `LeaveSitModeTransitionAction_t`:
    /// `N3Msg_StartCamping` sends the first, `N3Msg_MovementChanged(0x1e)`).
    pub const SIT: u8 = 30;
    pub const LEAVE_SIT: u8 = 37;
}

/// The server's announcement of the playfield the character is in (at login
/// and after zoning): the identity's instance is the playfield number and
/// the data holds the arrival position.
pub const PLAYFIELD: u32 = 0x5f4b1a39;

/// The server's game clock (`GameTimeMessage`): the handler at 0x10039800 in Gamecode gives `GameTime_t::Update` the
/// message's float (seconds into the 27-hour day), the day period, the day count and the server's time.
pub const GAME_TIME: u32 = 0x5f52412e;

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
/// `N3Msg_StopCamping` (Gamecode 0x1001cb63): the timed logout given up (CharacterAction 0x79).
pub const STOP_LOGOUT: u8 = 0x79;

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

/// The text of a `ChatText` (`5f4b442a`: the game's own lines, such as "Tarasque is no longer immortal."): two zero bytes, the text's
/// length and the text.
pub fn chat_text(data: &[u8]) -> Option<String> {
    let n = *data.get(2)? as usize;
    let text = data.get(3..3 + n)?;
    (data[..2] == [0, 0] && n > 0).then(|| String::from_utf8_lossy(text).into_owned())
}

/// The plain text of a `FormatFeedback` (`206b4b73`: "You successfully perform a Quick Bash attack."): the message is a compact
/// markup of tags and strings, and a string is a letter `s`, its length with the closing zero counted, the characters and a zero.
/// The first such string of printable text is returned (lines made of several parts, with names and numbers, are not rebuilt).
pub fn feedback_text(data: &[u8]) -> Option<String> {
    for i in 7..data.len().saturating_sub(3) {
        if data[i] != b's' {
            continue;
        }
        let n = data[i + 1] as usize;
        let (start, end) = (i + 2, i + 1 + n);
        if n < 4 || end >= data.len() || data[end] != 0 {
            continue;
        }
        let text = &data[start..end];
        if text.iter().all(|b| (0x20..0x7f).contains(b)) && text[0].is_ascii_alphabetic() {
            return Some(String::from_utf8_lossy(text).into_owned());
        }
    }
    None
}

/// `CharacterAction` (`5e477770`) action `0x69` (`InfoRequest`): asks for a character's or creature's info packet (the original sends it
/// when it targets something: the answer carries the target's health). The target is `(0xc350, id)`; the parameters are zero.
pub fn info_request(character: u32, target: u32) -> Frame {
    let mut data = vec![0u8; 5];
    data[4] = 0x69;
    data.extend_from_slice(&[0; 4]);
    for word in [CHARACTER, target, 0, 0] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(&[0; 2]);
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// What an `InfoPacket` (`4d38242e`) says of a character or creature: after the flag byte and the type (64-71 characters, 80 a
/// monster) come five bytes (a flag, the profession, the level, the title level, the look's profession), the side's experience (a
/// short) and the health and its maximum (words). The level only means something for a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Info {
    pub monster: bool,
    pub profession: u8,
    pub level: u8,
    pub health: u32,
    pub max_health: u32,
}

pub fn info_packet(data: &[u8]) -> Option<Info> {
    let kind = *data.get(1)?;
    if !matches!(kind, 64 | 65 | 67 | 71 | 80) {
        return None;
    }
    let word = |at: usize| data.get(at..at + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
    Some(Info { monster: kind == 80, profession: *data.get(3)?, level: *data.get(4)?, health: word(9)?, max_health: word(13)? })
}

/// One worn item of an `Inspect` answer.
#[derive(Debug, PartialEq, Eq)]
pub struct Inspected {
    pub slot: u32,
    pub low_id: u32,
    pub high_id: u32,
    pub ql: u32,
}

/// `Inspect` (`5a585f65`): the identity inspected, a count, then 32-byte records (slot, flags, an identity, low id, high id, quality, 0).
pub fn inspect_items(data: &[u8]) -> Vec<Inspected> {
    let word = |at: usize| u32::from_be_bytes(data[at..at + 4].try_into().unwrap());
    let mut items = Vec::new();
    let mut at = 13;
    while at + 32 <= data.len() {
        items.push(Inspected { slot: word(at), low_id: word(at + 16), high_id: word(at + 20), ql: word(at + 24) });
        at += 32;
    }
    items
}

/// `CharacterAction` (`5e477770`) action `0x41` (`RemoveFriendlyNano`): ends a running nano program. The same layout as a cast with no
/// target: the nano identity `(0xcf1b, id)` in the two parameters (the captured cancels are `0x42aba`, `0x42af6`...).
pub fn cancel_nano(character: u32, nano: u32) -> Frame {
    let mut data = vec![0u8; 5];
    data[4] = 0x41;
    data.extend_from_slice(&[0; 4]);
    for word in [0, 0, 0xcf1b, nano] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(&[0; 2]);
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// `CharacterAction` (`5e477770`) action `0x70` (`DeleteItem`): destroys the item in an inventory place. The target is `(0x68, place)`
/// and both parameters are the item's id (low and high: the same for a plain item), as in the captured delete.
pub fn delete_item(character: u32, place: u32, low: u32, high: u32) -> Frame {
    let mut data = vec![0u8; 5];
    data[4] = 0x70;
    data.extend_from_slice(&[0; 4]);
    for word in [0x68, place, low, high] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(&[0; 2]);
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// `CharacterAction` (`5e477770`) action `0xb3` (`UsePerk`): uses a perk. The target is the character itself (the perk acts on the
/// current target the server knows from `set_target`); the first parameter is the perk's id and the second its four-letter hash as
/// ASCII (big-endian), as in the captured uses (`0x2991` with `QISH`, `0x2994` with `DBSH`).
pub fn use_perk(character: u32, perk: u32, hash: [u8; 4]) -> Frame {
    let mut data = vec![0u8; 5];
    data[4] = 0xb3;
    data.extend_from_slice(&[0; 4]);
    for word in [CHARACTER, character, perk, u32::from_be_bytes(hash)] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(&[0; 2]);
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// `CharSecSpecAttack` (`51492120`): a weapon special attack at a target: a zero byte, the target identity and the skill's stat (the
/// captured ones: 142 Brawl, 144 Dimach, 147 Fast Attack, 148 Burst, 150 Fling Shot).
pub fn special_attack(character: u32, target: u32, stat: u32) -> Frame {
    let mut data = vec![0u8];
    for word in [CHARACTER, target, stat] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind: 0x51492120, identity: (CHARACTER, character), data }.frame(character)
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
/// Vicinity speech's data (GUI.dll FUN_10089669): the text's length (`u16`), the text, the kind (0 say, 1 whisper, 2 shout)
/// and the voice record (`voices::pack`, or nothing); none for a text over 1024 bytes.
pub fn vicinity_data(text: &str, kind: u8, record: &[u8]) -> Option<Vec<u8>> {
    if text.len() > 0x400 {
        return None;
    }
    let mut data = (text.len() as u16).to_be_bytes().to_vec();
    data.extend_from_slice(text.as_bytes());
    data.push(kind);
    data.extend_from_slice(record);
    Some(data)
}

/// Vicinity speech sent (Interfaces.dll `Client_t::SendVicinityMessage`, `SendShoutMessage`, `SendWhisperMessage`): a
/// `TextMessage_t` (frame kind 5) of object 3 (say), 4 (shout) or 2 (whisper) from the character to 2, its body the current
/// target's identity (`InputConfig_t` +0xc0; none 0), the data's length and the data ([`vicinity_data`]).
pub fn text_message(character: u32, object: u32, target: (u32, u32), data: &[u8]) -> Frame {
    let mut body = object.to_be_bytes().to_vec();
    body.extend_from_slice(&target.0.to_be_bytes());
    body.extend_from_slice(&target.1.to_be_bytes());
    body.extend_from_slice(&(data.len() as u32).to_be_bytes());
    body.extend_from_slice(data);
    Frame { kind: crate::frame::TEXT, source: character, destination: 2, body }
}

/// N3's `n3Command_t::s_nCommandRefCntr`: each command made takes the next number (from 1).
static COMMANDS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

/// A social action (Gamecode's `SocialActionCmd_t`, `N3Msg_DoSocialAction`): an `n3Command_t` (`IIR::Write`: its key, the
/// character, the command's byte 1; `n3Command_t::WriteSubClass`: a word 0 and the command's number) and the action
/// (`AbstractAnimID`, FUN_1007af4c).
pub fn social_action(character: u32, action: u32) -> Frame {
    let mut data = vec![1u8];
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&COMMANDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed).to_be_bytes());
    data.extend_from_slice(&action.to_be_bytes());
    Message { kind: 0x3b290771, identity: (CHARACTER, character), data }.frame(character)
}

/// `SetStat` (`6e5f566e`, Gamecode's `SetStatIIR_t` from `N3Msg_SetStat` 0x10026dc5: the value, then the stat): the client
/// asks for one of its own settings stats, as the control center's AGG/DEF slider does for stat 51 (-100 to 100; the
/// server's copy refuses others). The captured ones carry a leading 1.
pub fn set_stat(character: u32, stat: u32, value: i32) -> Frame {
    let mut data = vec![1u8];
    data.extend_from_slice(&value.to_be_bytes());
    data.extend_from_slice(&stat.to_be_bytes());
    Message { kind: 0x6e5f566e, identity: (CHARACTER, character), data }.frame(character)
}

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

/// `StopFight` (`4a41203e`): the character stops fighting (`N3Msg_StopAttack` → `StopFightIIR_t` FUN_1007a321 with 1, the
/// value the server's own `StopFight`s carry).
pub fn stop_fight(character: u32) -> Frame {
    let mut data = vec![0u8];
    data.extend_from_slice(&1u32.to_be_bytes());
    Message { kind: 0x4a41203e, identity: (CHARACTER, character), data }.frame(character)
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

/// `MarketSend` (`470b2e14`, Gamecode's `MarketSendIIR_c` from `N3Msg_SendMarketItem`): the market deposit window's Send,
/// the character, the credits and up to eight inventory items `(0x68, slot)`, counted the protocol's way (`(n + 1) * 0x3f1`).
pub fn market_send(character: u32, credits: i32, items: &[(u32, u32)]) -> Frame {
    let mut data = vec![0u8];
    data.extend_from_slice(&CHARACTER.to_be_bytes());
    data.extend_from_slice(&character.to_be_bytes());
    data.extend_from_slice(&credits.to_be_bytes());
    let items = &items[..items.len().min(8)];
    data.extend_from_slice(&((items.len() as u32 + 1) * 0x3f1).to_be_bytes());
    for (kind, id) in items {
        data.extend_from_slice(&kind.to_be_bytes());
        data.extend_from_slice(&id.to_be_bytes());
    }
    Message { kind: 0x470b2e14, identity: (CHARACTER, character), data }.frame(character)
}

#[cfg(test)]
mod tests {
    #[test]
    fn stop_fight_is_the_servers_shape() {
        let m = Message::parse(&stop_fight(0x4bc80)).unwrap();
        assert_eq!(m.identity, (CHARACTER, 0x4bc80));
        let p = crate::schema::Schema::global().parse_n3(m.kind, &m.data).unwrap();
        assert_eq!(crate::schema::Schema::global().n3_name(m.kind), Some("StopFight"));
        assert_eq!(p.value.get("Unk").and_then(crate::schema::Value::as_int), Some(1));
    }

    #[test]
    fn market_send_reads_back_as_the_schemas() {
        let m = Message::parse(&market_send(0x4bc80, 1500, &[(0x68, 0x40), (0x68, 0x41)])).unwrap();
        let p = crate::schema::Schema::global().parse_n3(m.kind, &m.data).unwrap();
        assert_eq!(p.value.get("Credits").and_then(crate::schema::Value::as_int), Some(1500));
        assert_eq!(p.value.get("Sender").and_then(|s| s.get("Instance")).and_then(crate::schema::Value::as_int), Some(0x4bc80));
        match p.value.get("Items") {
            Some(crate::schema::Value::List(items)) => assert_eq!(items.len(), 2),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn game_text_is_read_from_the_captured_messages() {
        let hex = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect() };
        assert_eq!(chat_text(&hex("00003254617261737175652077696c6c2072656d61696e20696d6d6f7274616c20666f72203135206d6f7265206d696e757465732e100000000000")).as_deref(), Some("Tarasque will remain immortal for 15 more minutes."));
        assert_eq!(chat_text(&hex("00001f5461726173717565206973206e6f206c6f6e67657220696d6d6f7274616c2e100000000000")).as_deref(), Some("Tarasque is no longer immortal."));
        assert_eq!(chat_text(&[1, 2, 3]), None);
        assert_eq!(
            feedback_text(&hex("0100000000003b7e26212121223a212121293c732e596f75207375636365737366756c6c7920706572666f726d206120517569636b20426173682061747461636b2e00000000")).as_deref(),
            Some("You successfully perform a Quick Bash attack.")
        );
        assert_eq!(feedback_text(&hex("010000000000267e26212121223a234852624b7318426f78207769746820612050756d706b696e20486561647e00000000")).as_deref(), None);
    }

    #[test]
    fn set_stat_matches_the_capture() {
        // capture-225436: C>S 6e5f566e of (c350, 4bc80): 01 00000064 00000107
        let m = Message::parse(&set_stat(0x4bc80, 0x107, 100)).unwrap();
        assert_eq!((m.kind, m.data.clone()), (0x6e5f566e, vec![1, 0, 0, 0, 0x64, 0, 0, 1, 7]));
    }

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
        assert_eq!(hex(info_request(0x4c2de, 0x4c2de)), "0000000069000000000000c3500004c2de00000000000000000000");
        let hexb = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect() };
        // a captured monster (health 92 of 92) and a captured level-200 character (health 54304)
        assert_eq!(info_packet(&hexb("015001000101000 14b0000005c0000005c00000060".replace(' ', "").as_str())), Some(Info { monster: true, profession: 0, level: 1, health: 92, max_health: 92 }));
        assert_eq!(info_packet(&hexb("01430109c8070906a00000d4200000d420000000000000120600054265616c7300")).map(|i| (i.monster, i.profession, i.level, i.health, i.max_health)), Some((false, 9, 200, 54304, 54304)));
        assert_eq!(info_packet(&[1, 0x50]), None);
        assert_eq!(hex(cancel_nano(0x4bc7f, 0x42aba)), "00000000410000000000000000000000000000cf1b00042aba0000");
        assert_eq!(hex(delete_item(0x4bc7f, 0x5b, 0x40d13, 0x40d13)), "000000007000000000000000680000005b00040d1300040d130000");
        let inspect: Vec<u8> = (0..("000000c3500020eb3f0000d4d6000000010081000100000000000000000004ab520004ab5200000064000000000000000200810001000000000000000000049f4b00049f4b00000064").len() / 2).map(|i| u8::from_str_radix(&("000000c3500020eb3f0000d4d6000000010081000100000000000000000004ab520004ab5200000064000000000000000200810001000000000000000000049f4b00049f4b00000064")[2 * i..2 * i + 2], 16).unwrap()).collect();
        assert_eq!(inspect_items(&inspect), vec![Inspected { slot: 1, low_id: 0x4ab52, high_id: 0x4ab52, ql: 100 }]);
        assert_eq!(hex(use_perk(0x4bc7f, 0x2991, *b"QISH")), "00000000b3000000000000c3500004bc7f00002991514953480000");
        assert_eq!(hex(use_perk(0x4bc7f, 0x2a0a, *b"WRTK")), "00000000b3000000000000c3500004bc7f00002a0a5752544b0000");
        assert_eq!(hex(use_perk(0x4bc80, 0x27bc, *b"BICC")), "00000000b3000000000000c3500004bc80000027bc424943430000");
        assert_eq!(hex(special_attack(0x4bc80, 0x7b017c2a, 142)), "000000c3507b017c2a0000008e");
        assert_eq!(hex(special_attack(0x4bc80, 0x7b017c2a, 150)), "000000c3507b017c2a00000096");
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
    fn vicinity_speech_is_a_text_message() {
        // (say "hi" at nobody, with a voice record: object 3, no target, the data's length, then u16 2, "hi", kind 0, the record)
        let data = vicinity_data("hi", 0, &[1, 4, 2]).unwrap();
        assert_eq!(data, [0, 2, b'h', b'i', 0, 1, 4, 2]);
        let f = text_message(0x4c2de, 3, (0, 0), &data);
        assert_eq!((f.kind, f.source, f.destination), (crate::frame::TEXT, 0x4c2de, 2));
        assert_eq!(f.body, [&[0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8][..], &data].concat());
        assert!(vicinity_data(&"x".repeat(0x401), 0, &[]).is_none());
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
/// * bit 5: the identity the character is fighting (`FightingTarget`, type and instance);
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
    /// Whom the character is fighting (flag bit 5): the target's identity.
    pub fighting: Option<(u32, u32)>,
    pub position: [f32; 3],
    pub rotation: Option<[f32; 4]>,
    pub breed: u8,
    pub sex: u8,
    /// The side (the packed word's low three bits, Gamecode FUN_1007916d's +0x8b: 0 neutral, 1 clan, 2 omni, more for
    /// monsters), what the Tab keys tell friend from foe by (`N3Msg_GetCloseTarget`, stat 33).
    pub side: u8,
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
    (8..post.len().saturating_sub(7).min(72)).find_map(|p| {
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

    /// A [`GAME_TIME`] message: the seconds into the day, the day period, the day count and the server's time.
    pub fn game_time(&self) -> Option<(f32, i32, i32, i32)> {
        if self.kind != GAME_TIME || self.data.len() < 16 {
            return None;
        }
        let w = |at: usize| u32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        Some((f32::from_bits(w(0)), w(4) as i32, w(8) as i32, w(12) as i32))
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
        let fighting = if flags & 0x20 != 0 { Some((u32_at(&mut at)?, u32_at(&mut at)?)) } else { None };
        let position = [f32_at(&mut at)?, f32_at(&mut at)?, f32_at(&mut at)?];
        let rotation = if flags & 0x200 != 0 { Some([f32_at(&mut at)?, f32_at(&mut at)?, f32_at(&mut at)?, f32_at(&mut at)?]) } else { None };
        let packed = u32_at(&mut at)?;
        // The breed is the three bits at 5 (a Nanomage female packs 0x768, a Solitus female 0x729); the sex the two at 8.
        let (breed, sex) = (((packed >> 5) & 7) as u8, ((packed >> 8) & 3) as u8);
        let side = (packed & 7) as u8;
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
        Some(EntityInfo { version, flags, playfield, fighting, position, rotation, breed, sex, side, name, flags2, unknown_a0: model, textures, clothes, template })
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

#[cfg(test)]
mod npc_template_tests {
    use super::*;

    #[test]
    fn the_template_is_found_after_a_long_name_too() {
        // "Chimeran Canine" (a 15-character name) from the original client's capture: the template 0x33115 sits further on than a short name's
        let tail = "001008120100000000cf000000007421ac00000331150064001f000000001c3fbffb";
        let bytes: Vec<u8> = (0..tail.len() / 2).map(|i| u8::from_str_radix(&tail[2 * i..2 * i + 2], 16).unwrap()).collect();
        let mut post = vec![0u8; 32];
        post.extend_from_slice(&bytes);
        assert_eq!(npc_template(&post), Some(0x33115));
    }
}
