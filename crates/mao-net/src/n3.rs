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
    /// The strafes (`MovementType`): the captures carry them for the other characters about.
    pub const STRAFE_RIGHT_START: u8 = 5;
    pub const STRAFE_RIGHT_STOP: u8 = 6;
    pub const STRAFE_LEFT_START: u8 = 7;
    pub const STRAFE_LEFT_STOP: u8 = 8;
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
    /// Swimming, and leaving it for the walk or run before (`SwitchToSwimModeTransitionAction_t`,
    /// `LeaveSwimModeTransitionAction_t`: the character asks for them as its vehicle enters and leaves deep liquid).
    pub const SWIM_MODE: u8 = 0x1a;
    pub const LEAVE_SWIM: u8 = 0x23;
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

/// A generated building's layout ([`Message::generated`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    /// The building's size in 10-unit blocks.
    pub width: u16,
    pub height: u16,
    /// How far up each floor is.
    pub floor_step: u16,
    /// The playfield whose rooms are copied.
    pub template: u32,
    pub ambient: [u8; 3],
    pub rooms: Vec<GeneratedRoom>,
}

/// One room of a [`Generated`] building: the template room, its floor, its block on the grid and its quarter turns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratedRoom {
    pub room: u16,
    pub floor: i8,
    pub x: u8,
    pub y: u8,
    pub turn: u8,
}

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

/// `CharacterAction` (`5e477770`) action `0x9a`: resets a skill, giving its IP back (`N3Msg_ResetSkill`, Gamecode
/// 0x1001ce37: no target, the first parameter nothing and the second the skill's stat; stat 0 is the full reset of
/// every ability and skill). Built from the decompile: no capture has one.
pub fn reset_skill(character: u32, stat: u32) -> Frame {
    let mut data = vec![0u8; 5];
    data[4] = 0x9a;
    data.extend_from_slice(&[0; 4]);
    for word in [0, 0, 0, stat] {
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

/// What a character's info packet says beyond [`Info`]: the bytes after the flag and the type are a flag, the
/// profession, the level, the title level and the look's profession; then the side's experience (a short), the health
/// and its maximum and the breed's hostility (words); for a member of an organization (types 65, 67 and 71) the
/// organization's id; the first name, the last name and the legacy title (each a `u16` length and the text) and a
/// short; for a member the rank's name; then (types 64 and 65; the others list their towers first, which is not read)
/// the city's playfield and the records: invaders killed, killed by invaders, the alien level, duels won and lost, a
/// word, and the solo and team kills. The layout of a member's packet is the schema's, not yet seen in a capture.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterInfo {
    pub profession: u8,
    pub level: u8,
    pub title_level: u8,
    pub first: String,
    pub last: String,
    pub legacy: String,
    pub rank: Option<String>,
    pub invaders_killed: u32,
    pub killed_by_invaders: u32,
    pub ai_level: u32,
    pub duel_wins: u32,
    pub duel_losses: u32,
    pub solo_kills: u32,
    pub team_kills: u32,
}

pub fn character_info(data: &[u8]) -> Option<CharacterInfo> {
    fn word(data: &[u8], at: &mut usize) -> Option<u32> {
        let v = u32::from_be_bytes(data.get(*at..*at + 4)?.try_into().ok()?);
        *at += 4;
        Some(v)
    }
    fn text(data: &[u8], at: &mut usize) -> Option<String> {
        let n = u16::from_be_bytes(data.get(*at..*at + 2)?.try_into().ok()?) as usize;
        let s = String::from_utf8_lossy(data.get(*at + 2..*at + 2 + n)?).trim_end_matches('\0').to_string();
        *at += 2 + n;
        Some(s)
    }
    let kind = *data.get(1)?;
    if !matches!(kind, 64 | 65 | 67 | 71) {
        return None;
    }
    let member = kind != 64;
    let mut at = 21;
    if member {
        word(data, &mut at)?;
    }
    let (first, last, legacy) = (text(data, &mut at)?, text(data, &mut at)?, text(data, &mut at)?);
    at += 2;
    let rank = if member { Some(text(data, &mut at)?) } else { None };
    let mut info = CharacterInfo { profession: *data.get(3)?, level: *data.get(4)?, title_level: *data.get(5)?, first, last, legacy, rank, ..Default::default() };
    if matches!(kind, 64 | 65) {
        at += 4;
        let mut records = [0u32; 8];
        for r in &mut records {
            match word(data, &mut at) {
                Some(v) => *r = v,
                None => return Some(info),
            }
        }
        info.invaders_killed = records[0];
        info.killed_by_invaders = records[1];
        info.ai_level = records[2];
        info.duel_wins = records[3];
        info.duel_losses = records[4];
        info.solo_kills = records[6];
        info.team_kills = records[7];
    }
    Some(info)
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

/// `PetCommand` (`6b333303`, `PetCommandIIR_c`): a command to the character's pets: a zero byte, 1, the command, 0, the pets (an
/// empty list: all of them; the server takes the target it knows) and two zeros, as the 2026-10-31 capture's `/pet attack`.
pub fn pet_command(character: u32, command: u32) -> Frame {
    let mut data = vec![0u8];
    for word in [1, command, 0, 0x3f1, 0, 0] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind: 0x6b333303, identity: (CHARACTER, character), data }.frame(character)
}

/// `PetCommand` to one pet, as the pet window's buttons send it: `N3Msg_SendPetCommand` (Gamecode 0x1001b2ea) puts the pet in the
/// list and `PetCommandIIR_c`'s writer (FUN_100766ec reads it back) gives the first word (the caller's 0 here, where `/pet`
/// passes 1), the command, 0, the pets counted `(n + 1) * 0x3f1`, whether the pet is a tower (0) and an empty text.
/// Not captured: the 2026-10-31 capture's commands were all `/pet` ones.
pub fn pet_command_to(character: u32, command: u32, pet: (u32, u32)) -> Frame {
    let mut data = vec![0u8];
    for word in [0, command, 0, 2 * 0x3f1, pet.0, pet.1, 0, 0] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind: 0x6b333303, identity: (CHARACTER, character), data }.frame(character)
}

/// `AddPet` (`194e4f76`) and `RemovePet` (`58742a0f`), to the character: the pet's identity after a header byte (the 2026-10-31
/// capture: `01 0000c350 7b1e6d64`). `Some((true, pet))` for an added pet, `Some((false, pet))` for a removed one.
pub fn pet_change(m: &Message) -> Option<(bool, (u32, u32))> {
    let added = match m.kind {
        0x194e4f76 => true,
        0x58742a0f => false,
        _ => return None,
    };
    let word = |at: usize| m.data.get(at..at + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
    Some((added, (word(1)?, word(5)?)))
}

/// `N3Teleport` (`43197d22`): the server moves a dynel. N3.dll's `n3TeleportIIR_t::ReadSubClass` (0x10029e7f) reads, after
/// the header byte: the destination (3 floats, +0x18), the rotation (a quaternion x y z w, +0x24), a `PlayfieldProxy_t`
/// (FUN_10038402, +0x34: the version byte `'a'`, the playfield identity, the server id, the group id and an identity: `9c50`
/// with the playfield's id, or all zero), two more identities (+0x4c, +0x54) and a byte string (an int length, then the bytes,
/// kept when under 0x3e9: `3f000000 0` before a zoning, the arrival point in a move within the server, the destination again
/// in a generated mission). `WriteSubClass` (0x10029f1b) writes the same.
#[derive(Clone, Debug, PartialEq)]
pub struct Teleport {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    /// The playfield identity: `c79c` plain, `c79e` instanced, `c79f` generated, `c7a0` the same playfield (all zero ids).
    pub playfield: (u32, u32),
    pub server: u32,
    pub group: u32,
    /// The proxy's last identity (+0x44): its instance zero means a move inside the playfield (`Activate` tests +0x48).
    pub proxy: (u32, u32),
    /// The identities at +0x4c and +0x54 (not understood: the second is `186a1` before most zonings, `186a7` in a mission).
    pub unknown: [(u32, u32); 2],
    pub tail: Vec<u8>,
}

impl Teleport {
    /// `n3TeleportIIR_t::Activate` (0x10029f87): with the proxy's instance zero the dynel is put at the destination with the
    /// rotation at once (`UpdateLastAllowedPosition`, `SetRelPosRot`); otherwise, for the client's own character only, the
    /// playfield is stopped for the zoning that follows (`n3EngineClient_t::StartTeleport`).
    pub fn within_playfield(&self) -> bool {
        self.proxy.1 == 0
    }
}

/// Reads an `N3Teleport` (`None` for another kind, or for a proxy whose version is not `'a'`, which the original throws on).
pub fn teleport(m: &Message) -> Option<Teleport> {
    if m.kind != 0x43197d22 {
        return None;
    }
    let word = |at: usize| m.data.get(at..at + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
    let f = |at: usize| word(at).map(f32::from_bits);
    let position = [f(1)?, f(5)?, f(9)?];
    let rotation = [f(13)?, f(17)?, f(21)?, f(25)?];
    if *m.data.get(29)? != b'a' {
        return None;
    }
    let length = word(70)? as usize;
    let tail = if length < 0x3e9 { m.data.get(74..74 + length)?.to_vec() } else { Vec::new() };
    Some(Teleport { position, rotation, playfield: (word(30)?, word(34)?), server: word(38)?, group: word(42)?, proxy: (word(46)?, word(50)?), unknown: [(word(54)?, word(58)?), (word(62)?, word(66)?)], tail })
}

/// `CharacterAction` `0x51` (`UseItemOnItem`): one item used on another, a tradeskill combine: the two items' identities (the
/// 2026-10-31 capture: backpack places `0x56` and `0x43`, two pattern pieces joined).
pub fn use_item_on_item(character: u32, item: (u32, u32), target: (u32, u32)) -> Frame {
    let mut data = vec![0u8; 5];
    data[4] = 0x51;
    data.extend_from_slice(&[0; 4]);
    for word in [item.0, item.1, target.0, target.1] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(&[0; 2]);
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// The tradeskill window's `CharacterAction`s (Gamecode builds them with FUN_10072a88 as for [`use_item_on_item`]: the action,
/// 0, a first identity, a second one as the two parameters): `N3Msg_InsertSourceAnalyzerItem` (0x10029a53) is `0xdc`
/// (`TradeskillSourceChanged`) with nobody first and the item second (none: taken out), `N3Msg_InsertTargetAnalyzerItem`
/// (0x10029af4) `0xdd` the same, and `N3Msg_BuildAnalyzerItem(ql)` (0x10029b95) `0xde` with `(0, ql)` first and nobody
/// second; the 2026-10-30 capture's: `dc` with `(0x68, 0x5a)`, `dc` with nothing, `de` with `(0, 1)`.
pub fn tradeskill(character: u32, action: u8, first: (u32, u32), second: (u32, u32)) -> Frame {
    let mut data = vec![0u8; 5];
    data[4] = action;
    data.extend_from_slice(&[0; 4]);
    for word in [first.0, first.1, second.0, second.1] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(&[0; 2]);
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// The server's tradeskill answers, `CharacterAction` `0xdf` to `0xe4` to the character: `(action, first identity, the two
/// parameters)`. Gamecode's handler (FUN_1005d61f, cases 0x1005f1a1..0x1005f212) passes them on to the window as: `0xdf`
/// (`TradeskillSource`) and `0xe0` (`TradeskillTarget`) the second parameter (how many processes the item is used in),
/// `0xe1` (`TradeskillNotValid`) nothing, `0xe2` (`TradeskillOutOfRange`) the second parameter (the level the source must
/// have), `0xe3` (`TradeskillRequirement`) the parameters (a skill and the value needed), `0xe4` (`TradeskillResult`) the
/// first parameter, the first identity's type, the second parameter and its instance (the result's lowest and highest
/// quality and its low and high templates). The 2026-10-30 capture's: `df` 0x154, `e0` 1, `e1`, `e3` (0xa5, 60) and
/// (0xa0, 25), `e4` with `(1, 0x26ad0)` and `(1, 0x26ad0)`.
pub fn tradeskill_reply(m: &Message) -> Option<(u8, (u32, u32), (u32, u32))> {
    if m.kind != 0x5e477770 || m.data.len() < 25 || !(0xdf..=0xe4).contains(&m.data[4]) || m.data[..4] != [0; 4] {
        return None;
    }
    let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
    Some((m.data[4], (word(9), word(13)), (word(17), word(21))))
}

/// `CharacterAction` `0xbb` (`N3Msg_TrainPerk`, Gamecode FUN_10053422) or `0xbc` (`N3Msg_UntrainPerk`, FUN_100534d2): train a perk
/// rank, or untrain one, by its id in the second parameter (the 2026-10-31 capture: rank `0x7d` both ways). The server echoes it.
pub fn train_perk(character: u32, perk: u32, train: bool) -> Frame {
    let mut data = vec![0u8; 27];
    data[4] = if train { 0xbb } else { 0xbc };
    data[21..25].copy_from_slice(&perk.to_be_bytes());
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

/// `ClientMoveItemToInventory` (`5469373f`) for an item named by its identity (`N3Msg_MoveItemToInventory`: the bank's items, as
/// the server's `Bank` message names them) into place `to` of the inventory (`0x6f`: any free place).
pub fn move_to_inventory(character: u32, item: (u32, u32), to: u32) -> Frame {
    let mut data = vec![0u8];
    for word in [item.0, item.1, to] {
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

/// `5e477770` action `0x1c`: the answer to a team invitation from `inviter`, 1 to join, 0 not to (the Arete Landing capture:
/// `00 | 0000001c | 0 | c350 inviter | 0 | 00000001 | 0000`).
pub fn team_reply(character: u32, inviter: u32, join: bool) -> Frame {
    let mut data = vec![0u8; 27];
    data[4] = 0x1c;
    data[9..13].copy_from_slice(&CHARACTER.to_be_bytes());
    data[13..17].copy_from_slice(&inviter.to_be_bytes());
    data[21..25].copy_from_slice(&(join as u32).to_be_bytes());
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }.frame(character)
}

/// `5e477770` action `0x18`: leave the team (no target).
pub const LEAVE_TEAM: u8 = 0x18;

/// `Skill` (`3e205660`): raise skills with IP, each skill with its new trained value (the Arete Landing capture: the six
/// abilities from 15, 6, 10, 3, 3, 3 to 18, 9, 13, 6, 6, 6 as `00 | 00000006 | 00000010 00000012 | ...`). The server answers
/// with the same pairs and the IP left (stat 53).
pub fn raise_skills(character: u32, raises: &[(u32, u32)]) -> Frame {
    let mut data = vec![0u8];
    data.extend_from_slice(&(raises.len() as u32).to_be_bytes());
    for &(stat, value) in raises {
        data.extend_from_slice(&stat.to_be_bytes());
        data.extend_from_slice(&value.to_be_bytes());
    }
    Message { kind: 0x3e205660, identity: (CHARACTER, character), data }.frame(character)
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

/// `52526858` action 5: use an item on something (`N3Msg_UseItemOnItem`, Gamecode 0x1002695c): a zero flag, the item's identity
/// (`(0x68, place)` in the backpack, `(0x6b, handle << 16 | place)` in an open container) and the target's. The original entering
/// the Garden of Lord Galahad (2026-10-31): the key, `(0x6b, 0x00700006)`, used on the statue `(0xc73d, 0x58f37539)`.
pub fn use_item_on(character: u32, sequence: u32, item: (u32, u32), target: (u32, u32)) -> Frame {
    let mut data = vec![1u8];
    for word in [0, sequence, 5, 0, CHARACTER, character, item.0, item.1, target.0, target.1] {
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

/// The NPC chat (`Knubot`) messages: the header byte, an `i16` 2 (as aosharp sends them to the live servers), the NPC, then
/// the message's own fields; from the character (Gamecode's `N3Msg_NPCChat*` pass the character and the NPC).
fn knubot(character: u32, kind: u32, npc: (u32, u32), tail: &[u32]) -> Frame {
    let mut data = vec![0u8];
    data.extend_from_slice(&2i16.to_be_bytes());
    for word in [npc.0, npc.1].iter().chain(tail) {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind, identity: (CHARACTER, character), data }.frame(character)
}

/// `KnubotOpenChatWindow` (`3b132d64`): starts talking to an NPC (two zero words).
pub fn npc_chat_open(character: u32, npc: (u32, u32)) -> Frame {
    knubot(character, 0x3b132d64, npc, &[0, 0])
}

/// `KnubotAnswer` (`2103247d`): the answer chosen, counted from 0 in the last `KnubotAnswerList`
/// (`N3Msg_SendNPCChatAnswer`).
pub fn npc_chat_answer(character: u32, npc: (u32, u32), answer: u32) -> Frame {
    knubot(character, 0x2103247d, npc, &[answer])
}

/// `KnubotCloseChatWindow` (`270a4c62`): the window closed (`N3Msg_NPCChatCloseWindow`: a zero and an empty string).
pub fn npc_chat_close(character: u32, npc: (u32, u32)) -> Frame {
    knubot(character, 0x270a4c62, npc, &[0, 0])
}

/// `KnubotStartTrade` (`7864401d`): Give, asking the NPC for its trade (`N3Msg_NPCChatStartTrade`: no slots, an empty text).
pub fn npc_trade_start(character: u32, npc: (u32, u32)) -> Frame {
    knubot(character, 0x7864401d, npc, &[0, 0])
}

/// `KnubotTrade` (`3a1b2c0c`): an inventory item put into the NPC trade (action 0, `N3Msg_NPCChatAddTradeItem`) or taken
/// out (1, `N3Msg_NPCChatRemoveTradeItem`): the action, an empty identity, the item `(0x68, slot)`.
pub fn npc_trade_item(character: u32, npc: (u32, u32), action: u32, slot: u32) -> Frame {
    knubot(character, 0x3a1b2c0c, npc, &[action, 0, 0, 0x68, slot])
}

/// `KnubotFinishTrade` (`55682b24`): the trade's end (`N3Msg_NPCChatEndTrade`): whether it is declined, and the credits given.
pub fn npc_trade_finish(character: u32, npc: (u32, u32), credits: u32, accept: bool) -> Frame {
    knubot(character, 0x55682b24, npc, &[if accept { 0 } else { 1 }, credits])
}

/// `KnubotNPCDescription` (`000a0c5a`): asks for the NPC's description (`N3Msg_NPCChatRequestDescription`).
pub fn npc_chat_describe(character: u32, npc: (u32, u32)) -> Frame {
    knubot(character, 0x000a0c5a, npc, &[])
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

/// `FanaticIIR_t` (`5c525a7b`, `n3InfoItemRemote_t::MapToKey("FanaticIIR_t")`): a command for the server, as Fanatic.dll's
/// `ClientInterface_c::Command` sends it (its IIR's flag byte 0, then FUN_10001112's writes): the chat window it was typed in
/// (GUI.dll's chat module +0x1ec, its "Window%d" number), the selected target (none: zeros) and the text without its slash
/// (`/stuck` is "stuck"; GUI.dll FUN_100b36da).
pub fn server_command(character: u32, window: u32, target: (u32, u32), text: &str) -> Frame {
    let mut data = vec![0u8];
    for word in [window, target.0, target.1, text.len() as u32] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(text.as_bytes());
    Message { kind: 0x5c525a7b, identity: (CHARACTER, character), data }.frame(character)
}

/// `Quest` (`212c487a`) action 1, Delete: drops one of the character's missions (laid out as the server's own Delete:
/// the header byte, the action, a zero, the mission, two zeros).
pub fn quest_delete(character: u32, mission: (u32, u32)) -> Frame {
    let mut data = vec![0u8];
    for word in [1, 0, mission.0, mission.1, 0, 0] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind: 0x212c487a, identity: (CHARACTER, character), data }.frame(character)
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

/// `3115534d`, the research request: the research window's `N3Msg_UpdateGlobalResearch` (Gamecode FUN_1002bf3b sends an
/// info item to the server) asks for the global research lines, which come back as `ResearchUpdate` (`253d0240`). One
/// byte, 0, as the 2026-10-30 and 2026-10-31 captures send it when the window opens.
pub fn research_request(character: u32) -> Frame {
    Message { kind: 0x3115534d, identity: (CHARACTER, character), data: vec![0] }.frame(character)
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
    position_update_pitched(character, movement, heading, 0.0, position, elapsed_ms)
}

/// The rotation of a heading about Y then an elevation (nose up positive) about the character's own X: `(x, y, z, w)`.
pub fn pitched_rotation(heading: f32, elevation: f32) -> [f32; 4] {
    let (s, c) = (heading * 0.5).sin_cos();
    let (ps, pc) = (-elevation * 0.5).sin_cos();
    [c * ps, s * pc, -s * ps, c * pc]
}

/// [`position_update`] for a flying character: the rotation the heading about Y then `elevation` (radians, nose up
/// positive) about the character's own X, which turns its +z up for a negative angle (the original climbing at 27 degrees
/// sends `x = -0.232`).
pub fn position_update_pitched(character: u32, movement: u8, heading: f32, elevation: f32, position: [f32; 3], elapsed_ms: u32) -> Frame {
    let [x, y, z, w] = pitched_rotation(heading, elevation);
    let mut d = vec![1, movement];
    for v in [x, y, z, w, position[0], position[1], position[2]] {
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
    fn teleports_read_as_the_captures_have_them() {
        let m = |h: &str| Message { kind: 0x43197d22, identity: (CHARACTER, 0x7b1a0021), data: (0..h.len() / 2).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap()).collect() };
        // (the 16:55 capture: a move in a generated mission, then a zoning to playfield 0x1bbd)
        let t = teleport(&m("0042f2dd0843146a3d42af80c40000000000000000000000003f800000610000c7a000000000000000000000000000000000000000000000000000000000000186a7000000000000001042f2dd084314752042af80c400000000")).unwrap();
        assert_eq!((t.playfield, t.proxy, t.unknown[1], t.tail.len()), ((0xc7a0, 0), (0, 0), (0x186a7, 0), 16));
        assert!(t.within_playfield());
        assert!((t.position[0] - 121.43).abs() < 0.01 && (t.position[1] - 148.41).abs() < 0.01 && (t.position[2] - 87.75).abs() < 0.01);
        assert_eq!(t.rotation, [0.0, 0.0, 0.0, 1.0]);
        let t = teleport(&m("004560b583424efae1444484de00000000bf0d22f0800000003f559487610000c79c00001bbd000000000000000000009c5000001bbd0000000000000000000186a100011bbd000000083f00000000000000")).unwrap();
        assert_eq!((t.playfield, t.proxy, t.unknown[1], t.tail.clone()), ((0xc79c, 0x1bbd), (0x9c50, 0x1bbd), (0x186a1, 0x11bbd), vec![0x3f, 0, 0, 0, 0, 0, 0, 0]));
        assert!(!t.within_playfield());
        assert!(teleport(&Message { kind: 0x5e477770, ..m("00") }).is_none());
    }

    #[test]
    fn a_pet_command_is_the_captured_shape() {
        let m = Message::parse(&pet_command(0x4bc82, 7)).unwrap();
        assert_eq!(m.kind, 0x6b333303);
        assert_eq!(m.data.iter().map(|b| format!("{b:02x}")).collect::<String>(), "00000000010000000700000000000003f10000000000000000");
    }

    #[test]
    fn pets_added_and_removed() {
        let m = Message { kind: 0x194e4f76, identity: (CHARACTER, 0x4bc82), data: vec![1, 0, 0, 0xc3, 0x50, 0x7b, 0x1e, 0x6d, 0x64] };
        assert_eq!(pet_change(&m), Some((true, (CHARACTER, 0x7b1e6d64))));
        let m = Message { kind: 0x58742a0f, ..m };
        assert_eq!(pet_change(&m), Some((false, (CHARACTER, 0x7b1e6d64))));
        let data = Message::parse(&pet_command_to(0x4bc82, 4, (CHARACTER, 0x7b1e6d64))).unwrap().data;
        assert_eq!(data.iter().map(|b| format!("{b:02x}")).collect::<String>(), "00000000000000000400000000000007e20000c3507b1e6d640000000000000000");
    }

    #[test]
    fn tradeskill_actions_are_the_captured_shape() {
        let hex = |f: Frame| Message::parse(&f).unwrap().data.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(hex(tradeskill(0x7b1a0021, 0xdc, (0, 0), (0x68, 0x5a))), "00000000dc000000000000000000000000000000680000005a0000");
        assert_eq!(hex(tradeskill(0x7b1a0021, 0xde, (0, 1), (0, 0))), "00000000de00000000000000000000000100000000000000000000");
        let reply = |h: &str| Message { kind: 0x5e477770, identity: (CHARACTER, 0x7b1a0021), data: (0..h.len() / 2).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap()).collect() };
        assert_eq!(tradeskill_reply(&reply("00000000df00000000000000000000000000000000000001540000")), Some((0xdf, (0, 0), (0, 0x154))));
        assert_eq!(tradeskill_reply(&reply("00000000e3000000000000000000000000000000a50000003c0000")), Some((0xe3, (0, 0), (0xa5, 60))));
        assert_eq!(tradeskill_reply(&reply("00000000e4000000000000000100026ad00000000100026ad00000")), Some((0xe4, (1, 0x26ad0), (1, 0x26ad0))));
        assert_eq!(tradeskill_reply(&reply("00000000dc000000000000000000000000000000680000005a0000")), None);
    }

    #[test]
    fn a_combine_is_the_captured_shape() {
        let data = Message::parse(&use_item_on_item(0x4bc82, (0x68, 0x56), (0x68, 0x43))).unwrap().data;
        assert_eq!(data.iter().map(|b| format!("{b:02x}")).collect::<String>(), "000000005100000000000000680000005600000068000000430000");
    }

    #[test]
    fn perk_training_is_the_captured_shape() {
        let hex = |f: Frame| Message::parse(&f).unwrap().data.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(hex(train_perk(0x4bc82, 0x7d, true)), "00000000bb000000000000000000000000000000000000007d0000");
        assert_eq!(hex(train_perk(0x4bc82, 0x7d, false)), "00000000bc000000000000000000000000000000000000007d0000");
    }

    #[test]
    fn use_item_on_is_the_captured_shape() {
        let m = Message::parse(&use_item_on(0x4bc80, 6, (0x6b, 0x0070_0006), (0xc73d, 0x58f3_7539))).unwrap();
        let hex: String = m.data.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "01000000000000000600000005000000000000c3500004bc800000006b007000060000c73d58f37539");
    }

    #[test]
    fn a_climb_is_sent_as_the_original_sends_it() {
        // (the original client climbing on a Phasefront Banshee, 2026-10-31: heading 0.1815, about 27 degrees up)
        let m = Message::parse(&position_update_pitched(0x4bc80, movement::FULL, 0.1815, 0.4705, [0.0; 3], 0)).unwrap();
        let (_, q, _) = m.movement().unwrap();
        for (got, want) in q.iter().zip([-0.2321f32, 0.0881, 0.0211, 0.9685]) {
            assert!((got - want).abs() < 0.001, "{q:?}");
        }
    }

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
    fn the_research_request_matches_the_capture() {
        // (arete-20261031-1505 #1771: C>S 3115534d of (c350, own), 1 byte: 00)
        let m = Message::parse(&research_request(0x4bc82)).unwrap();
        assert_eq!((m.kind, m.identity, m.data.clone()), (0x3115534d, (CHARACTER, 0x4bc82), vec![0]));
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

    /// The playfield message into ICC (1186, instance 0x20d9be), live: its runs of instance ids.
    #[test]
    fn instance_runs_match_icc() {
        let hex = "00000000044326800040a051ec43038000610000c79e000004a200000001c021028f00009c500020d9be0000c77d000000010000000100000001000000090000c75b00000001000000000000000f1324f4e20000c748000000010000000f0000000310c1c1780000c75b0000000100000012000000141324f4f10000c74800000001000000260000000110c1c17b0000c75b00000001000000270000000f1324f5050000c73d00000001000000360000000258c197d50000c74800000001000000380000000310c1c17c0000c73d000000010000003b0000000158c197d70000c75b000000010000003c0000000d1324f514ffffffffffffffff";
        let data: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        let m = Message { kind: PLAYFIELD, identity: (0x9c50, 0x20d9be), data };
        let runs = m.instance_runs();
        assert_eq!(runs.len(), 9);
        assert_eq!(runs[0], (0xc75b, 0, 15, 0x1324f4e2));
        assert_eq!(runs[5], (0xc73d, 0x36, 2, 0x58c197d5));
        assert_eq!(runs[7], (0xc73d, 0x3b, 1, 0x58c197d7));
        assert_eq!(m.arrival().map(|a| a.0), Some(0x4a2));
    }

    /// The Arete Landing capture's alien ship (instance 0x20da1b): its playfield message's generator data.
    #[test]
    fn a_generated_building_reads() {
        let hex = "00000000043fe6874041ab7ae1434d0290610000c79f00e0c856000000000000000000009c500020da1b0000c79f00e0c8560000000200030018001800800000017ec8c8c800000008000200000203000000030101000800060500000700070100000700040000000700040602000700090301000401090700ffffffffffffffff";
        let data: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        let m = Message { kind: PLAYFIELD, identity: (0x9c50, 0x20da1b), data };
        let g = m.generated().unwrap();
        assert_eq!((g.width, g.height, g.floor_step, g.template, g.ambient), (24, 24, 128, 382, [0xc8; 3]));
        assert_eq!(g.rooms.len(), 8);
        assert_eq!(g.rooms[0], GeneratedRoom { room: 2, floor: 0, x: 0, y: 2, turn: 3 });
        assert_eq!(g.rooms[7], GeneratedRoom { room: 4, floor: 1, x: 9, y: 7, turn: 0 });
        assert_eq!(m.arrival().map(|a| a.0), Some(382));
    }

    #[test]
    fn team_reply_matches_the_capture() {
        let hex = |f: Frame| -> String { Message::parse(&f).unwrap().data.iter().map(|b| format!("{b:02x}")).collect() };
        assert_eq!(hex(team_reply(0x7b1a0021, 0x7b1a0022, true)), "000000001c000000000000c3507b1a002200000000000000010000");
        assert_eq!(hex(team_reply(0x7b1a0021, 0x7b1a0022, false)), "000000001c000000000000c3507b1a002200000000000000000000");
        assert_eq!(hex(action(0x7b1a0021, LEAVE_TEAM)), "000000001800000000000000000000000000000000000000000000");
    }

    #[test]
    fn skill_raise_matches_the_capture() {
        let frame = raise_skills(0x7b1a0021, &[(16, 18), (17, 9), (18, 13), (19, 6), (20, 6), (21, 6)]);
        let hex: String = Message::parse(&frame).unwrap().data.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "000000000600000010000000120000001100000009000000120000000d000000130000000600000014000000060000001500000006");
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
    /// A creature's health and its maximum (flag bit 0): Gamecode's reader (FUN_1007916d) takes, after the second flags word
    /// and an `i32`, the family, line-of-sight height and one more value (each a byte with flag bits 17, 19 and 25, else an
    /// `i16`), an `i16` and a byte when that is above zero, the level (a byte, an `i16` with bit 12), the maximum (`+0x98`: an
    /// `i16` with bit 11, else an `i32`) and the health (`+0x9c`: with bit 14 a byte of damage taken from the maximum, else as
    /// wide as the maximum). The 2026-10-31 capture's pets agree with their info packets (23203 of 23203, 69609 of 69609).
    pub health: Option<(u32, u32)>,
    /// A creature's family (its first field after the second flags word and the `i32`, `+0xb2`), what the pet window asks as
    /// stat 455 `NPCFamily` (FUN_1007d29c: 0x60 is a healing pet; the capture's attack pets are 0x5f).
    pub family: Option<u16>,
}

/// Where the template id sits after the flags: a few bytes after the second
/// flags word and always before a level (`00 level 00 1f`), at a position
/// that moves by a byte with the fields in front.
fn npc_template(post: &[u8]) -> Option<u32> {
    (8..post.len().saturating_sub(7).min(72)).find_map(|p| {
        (post[p + 4] == 0 && post[p + 6] == 0 && post[p + 7] == 0x1f).then(|| u32::from_be_bytes(post[p..p + 4].try_into().unwrap()))
    })
}

/// A creature's family, and its health and maximum, from its entity message, `at` just past the second flags word and the
/// `i32` after it (see [`EntityInfo::health`], Gamecode FUN_1007916d's creature branch).
fn creature_fields(d: &[u8], mut at: usize, flags: u32) -> Option<(u16, (u32, u32))> {
    let mut take = |n: usize| -> Option<u32> {
        let b = d.get(at..at + n)?;
        at += n;
        Some(b.iter().fold(0u32, |v, &x| v << 8 | x as u32))
    };
    let mut first = None;
    for bit in [17, 19, 25] {
        let v = take(if flags >> bit & 1 != 0 { 1 } else { 2 })?;
        first.get_or_insert(v as u16);
    }
    if take(2)? as i16 > 0 {
        take(1)?;
    }
    take(if flags & 0x1000 != 0 { 2 } else { 1 })?;
    let max = take(if flags & 0x800 != 0 { 2 } else { 4 })?;
    let health = if flags & 0x4000 != 0 { max.saturating_sub(take(1)?) } else { take(if flags & 0x800 != 0 { 2 } else { 4 })? };
    Some((first?, (health, max)))
}

/// Where a monster's entity message keeps what the server changes (see [`set_monster`]): the playfield, the position, the name
/// (its length byte and its text), and the maximum and the health with their widths (`damage` when the health is the amount
/// lost from the maximum, one byte).
#[derive(Clone, Debug, PartialEq)]
struct MonsterPlaces {
    playfield: usize,
    position: usize,
    name_length: usize,
    name: std::ops::Range<usize>,
    max: (usize, usize),
    health: (usize, usize, bool),
}

/// The places of a monster's entity message, read the way [`Message::entity_info`] and `creature_fields` read them. None for a
/// character's message, or one without a playfield flag (the server does not add that flag).
fn monster_places(d: &[u8]) -> Option<MonsterPlaces> {
    let mut at = 1usize;
    let version = *d.get(at)?;
    at += 1;
    if !matches!(version, 0x39 | 0x3a) {
        return None;
    }
    let flags = u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?);
    at += 4;
    // (flag bit 0 is a monster's: no model word follows, the creature's fields do)
    if flags & 1 == 0 || flags & 0x40 == 0 {
        return None;
    }
    let playfield = at;
    at += 4;
    if flags & 0x20 != 0 {
        at += 8;
    }
    let position = at;
    at += 12;
    if flags & 0x200 != 0 {
        at += 16;
    }
    at += 4;
    let name_length = at;
    let length = *d.get(at)? as usize;
    at += 1;
    let name = at..at + length;
    at += length + 8;
    // creature_fields, from here: `cur` walks the fields, each read as its width says
    let mut cur = at;
    let read = |cur: &mut usize, n: usize| -> Option<u32> {
        let b = d.get(*cur..*cur + n)?;
        *cur += n;
        Some(b.iter().fold(0u32, |v, &x| v << 8 | x as u32))
    };
    for bit in [17, 19, 25] {
        read(&mut cur, if flags >> bit & 1 != 0 { 1 } else { 2 })?;
    }
    if read(&mut cur, 2)? as i16 > 0 {
        read(&mut cur, 1)?;
    }
    read(&mut cur, if flags & 0x1000 != 0 { 2 } else { 1 })?;
    let max_width = if flags & 0x800 != 0 { 2 } else { 4 };
    let max_at = cur;
    read(&mut cur, max_width)?;
    let (health_at, health_width, damage) = if flags & 0x4000 != 0 { (cur, 1, true) } else { (cur, if flags & 0x800 != 0 { 2 } else { 4 }, false) };
    Some(MonsterPlaces { playfield, position, name_length, name, max: (max_at, max_width), health: (health_at, health_width, damage) })
}

/// A monster's entity message (`271b3a6b`) with the server's own playfield, position, name, maximum and health, the rest kept
/// as captured: the decoder reads these fields, and the appearance (its template) is kept with the rest. Returns None where the
/// message is not a monster's, or the name is longer than a length byte allows.
pub fn set_monster(data: &[u8], name: &str, playfield: u32, position: [f32; 3], health: u32, max: u32) -> Option<Vec<u8>> {
    let p = monster_places(data)?;
    let name = name.as_bytes();
    // (the length byte counts the name's NUL terminator)
    if name.len() > 254 {
        return None;
    }
    let mut out = Vec::with_capacity(data.len() + name.len());
    out.extend_from_slice(&data[..p.playfield]);
    out.extend_from_slice(&playfield.to_be_bytes());
    out.extend_from_slice(&data[p.playfield + 4..p.position]);
    for v in position {
        out.extend_from_slice(&v.to_be_bytes());
    }
    out.extend_from_slice(&data[p.position + 12..p.name_length]);
    out.push(name.len() as u8 + 1);
    out.extend_from_slice(name);
    out.push(0);
    out.extend_from_slice(&data[p.name.end..p.max.0]);
    let (max_at, max_width) = p.max;
    out.extend_from_slice(&max.to_be_bytes()[4 - max_width..]);
    let (health_at, health_width, damage) = p.health;
    out.extend_from_slice(&data[max_at + max_width..health_at]);
    let value = if damage { max.saturating_sub(health).min(255) } else { health };
    out.extend_from_slice(&value.to_be_bytes()[4 - health_width..]);
    out.extend_from_slice(&data[health_at + health_width..]);
    Some(out)
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
        // (a generated building's playfield is drawn from its template playfield, which the generator data names)
        let template = match self.generated() {
            Some(g) => Some(g.template),
            None => (self.data.len() >= 26 && self.data[17] == b'a').then(|| u32::from_be_bytes(self.data[22..26].try_into().unwrap())).filter(|&t| t != 0),
        };
        Some((template.unwrap_or(self.identity.1), [f(5), f(9), f(13)]))
    }

    /// A generated building's layout (a mission dungeon: the proxy type `0xc79f` or `0xc7a1`), from the [`PLAYFIELD`] message's
    /// object after the proxy: `ACGBuildingGeneratorData_t` as Gamecode's FUN_100c773e reads it (the object's identity and a
    /// word, version 3, the width, the height and the floor step as shorts, the template playfield, the ambient colour's three
    /// bytes, then a count of `BuildingRoomInfo_t`: room, floor, x, y, turn; FUN_100cad28). The Arete Landing capture's ship:
    /// 24 x 24 blocks, step 128, template 382, ambient `c8c8c8`, 8 rooms.
    pub fn generated(&self) -> Option<Generated> {
        let d = &self.data;
        if self.kind != PLAYFIELD || d.len() < 73 || d[17] != b'a' {
            return None;
        }
        let w = |at: usize| u32::from_be_bytes(d[at..at + 4].try_into().unwrap());
        let s = |at: usize| u16::from_be_bytes(d[at..at + 2].try_into().unwrap());
        let proxy = w(18);
        if !matches!(proxy, 0xc79f | 0xc7a1) || w(42) != proxy || s(54) != 3 {
            return None;
        }
        let count = w(69) as usize;
        let rooms = (0..count.min(2500))
            .map_while(|k| {
                let at = 73 + k * 6;
                let r = d.get(at..at + 6)?;
                Some(GeneratedRoom { room: u16::from_be_bytes([r[0], r[1]]), floor: r[2] as i8, x: r[3], y: r[4], turn: r[5] })
            })
            .collect::<Vec<_>>();
        (rooms.len() == count).then(|| Generated { width: s(56), height: s(58), floor_step: s(60), template: w(62), ambient: [d[66], d[67], d[68]], rooms })
    }

    /// An instanced playfield's ids for its built-in objects (a [`PLAYFIELD`] message into ICC's shops): after the proxy, a
    /// `0xc77d` word and three more, the number of runs, then each run as five words: the kind (`0xc73d` terminal, `0xc748`
    /// door, `0xc75b` vending machine), 1, the first record's index in the playfield's dynel table, how many records, and the
    /// first record's id (the others counting on from it). Empty for a playfield whose objects keep their table ids.
    /// From live bytes (ICC 1186: terminal runs `(0xc73d, 54, 2, 0x58c197d5)` and `(0xc73d, 59, 1, 0x58c197d7)`, the bank
    /// being record 55).
    pub fn instance_runs(&self) -> Vec<(u32, u32, u32, u32)> {
        if self.kind != PLAYFIELD {
            return Vec::new();
        }
        let w = |at: usize| self.data.get(at..at + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
        let Some(start) = (17..self.data.len().saturating_sub(3)).find(|&at| w(at) == Some(0xc77d)) else { return Vec::new() };
        let Some(count) = w(start + 16) else { return Vec::new() };
        (0..count.min(256) as usize)
            .map_while(|k| {
                let at = start + 20 + k * 20;
                Some((w(at)?, w(at + 4)?, w(at + 8)?, w(at + 12)?, w(at + 16)?))
            })
            .filter(|r| r.1 == 1)
            .map(|(kind, _, first, n, id)| (kind, first, n, id))
            .collect()
    }

    /// A [`GAME_TIME`] message (17 bytes: a header byte, then the words Gamecode's handler 0x10039800 reads at +0x18 of the
    /// message): the seconds into the day (a float), the day period, the day count and the server's time (unix seconds).
    pub fn game_time(&self) -> Option<(f32, i32, i32, i32)> {
        if self.kind != GAME_TIME || self.data.len() < 17 {
            return None;
        }
        let w = |at: usize| u32::from_be_bytes(self.data[at..at + 4].try_into().unwrap());
        Some((f32::from_bits(w(1)), w(5) as i32, w(9) as i32, w(13) as i32))
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
        let (family, health) = match creature_fields(d, at, flags) {
            Some((family, health)) if flags & 1 != 0 => (Some(family), Some(health)),
            _ => (None, None),
        };
        Some(EntityInfo { version, flags, playfield, fighting, position, rotation, breed, sex, side, name, flags2, unknown_a0: model, textures, clothes, template, health, family })
    }
}

#[cfg(test)]
mod entity_tests {
    use super::*;

    #[test]
    fn a_skill_reset_is_a_character_action() {
        // (laid out as the info request is: the action, four zero bytes, no target, nothing, the skill's stat, two zero bytes)
        let (reset, info) = (reset_skill(7, 152).body, info_request(7, 9).body);
        assert_eq!(reset.len(), info.len());
        assert!(reset.ends_with(&[0x9a, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 152, 0, 0]));
        assert!(info.ends_with(&[0x69, 0, 0, 0, 0, 0, 0, 0xc3, 0x50, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn a_characters_info_packet_is_read() {
        // (a character of no organization: the header, then names, the city and the records)
        let mut data = vec![0, 64, 1, 4, 27, 3, 4, 0, 0];
        data.extend([0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0]);
        for name in ["Zachary", "", "Old hand"] {
            data.extend((name.len() as u16).to_be_bytes());
            data.extend(name.bytes());
        }
        data.extend([0, 0]);
        data.extend(0u32.to_be_bytes());
        for v in [7u32, 1, 12, 5, 2, 0, 9, 4, 0, 0, 0] {
            data.extend(v.to_be_bytes());
        }
        let info = character_info(&data).unwrap();
        assert_eq!((info.profession, info.level, info.title_level), (4, 27, 3));
        assert_eq!((info.first.as_str(), info.last.as_str(), info.legacy.as_str(), info.rank.clone()), ("Zachary", "", "Old hand", None));
        assert_eq!((info.invaders_killed, info.killed_by_invaders, info.ai_level, info.duel_wins, info.duel_losses, info.solo_kills, info.team_kills), (7, 1, 12, 5, 2, 9, 4));
        assert_eq!(info_packet(&data).map(|i| (i.health, i.max_health)), Some((512, 768)));
        // (a monster's is not a character's; a short one keeps what was read)
        data[1] = 80;
        assert_eq!(character_info(&data), None);
        data[1] = 64;
        data.truncate(50);
        assert_eq!(character_info(&data).map(|i| (i.first, i.invaders_killed)), Some(("Zachary".to_string(), 0)));
    }

    /// A `GameTime` message from a capture of the original (2026-11-08 18:11): 2:54 into the game's day, period 3,
    /// day 276880, the server's clock five seconds behind the capture's.
    #[test]
    fn the_game_time_follows_its_header_byte() {
        let data: Vec<u8> = (0..17).map(|i| u8::from_str_radix(&"014624000000000003000439906af10fb3"[2 * i..2 * i + 2], 16).unwrap()).collect();
        let m = Message { kind: GAME_TIME, identity: (0xc350, 0x39c16ee4), data };
        let (seconds, period, day, server) = m.game_time().unwrap();
        assert_eq!((seconds, period, day, server), (10496.0, 3, 0x43990, 1794183091));
        assert_eq!(((seconds as i32) / 3600, (seconds as i32) / 60 % 60), (2, 54));
    }

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

    /// A monster's entity message rewritten keeps everything but what the server sets, and the decoder reads the new values back.
    #[test]
    fn a_monster_is_set_and_read_back() {
        let data = bytes("003a0a0a4acb0020da15455d4df240c2b852443b140f00000000bf351336000000003f34f6af000006280a536861647920477579001088120100000000890000000014022f00000065da0063001f000000001c8000000000000000800000000301000100010001000100000002000000009ef345000003f1000017a6000000000000000000000000000000010003c89600000000000000020003c8b800000000000000030003c86800000000000000040003c8df0000000000000fc40000009ef3000000000401000076200000000002050004192500000000000000000000");
        let original = Message { kind: ENTITY_INFO, identity: (CHARACTER, 0x7b08289e), data: data.clone() };
        let info = original.entity_info().unwrap();
        // (the same values give back the same bytes)
        let same = set_monster(&data, &info.name, info.playfield.unwrap(), info.position, info.health.unwrap().0, info.health.unwrap().1).unwrap();
        assert_eq!(same, data);
        // (a new name, place and health)
        let changed = set_monster(&data, "Test Monster", 0x21d, [10.0, 2.0, -3.5], 40, 90).unwrap();
        let back = Message { kind: ENTITY_INFO, identity: (CHARACTER, 0x7b08289e), data: changed }.entity_info().unwrap();
        assert_eq!(back.name, "Test Monster");
        assert_eq!(back.playfield, Some(0x21d));
        assert_eq!(back.position, [10.0, 2.0, -3.5]);
        assert_eq!(back.health, Some((40, 90)));
        assert_eq!(back.template, info.template, "the appearance is kept");
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

    /// The 2026-10-31 15:05 capture's pets (#290 and the drone): their health and maximum, as their info packets give them.
    #[test]
    fn pet_health_from_capture() {
        let data = bytes(
            "00 3a 0a 2a 6a 53 00 00 02 1c 43 d3 52 8a 41 70 28 f6 43 cc 56 e3 80 00 00 00 00 00 00 00 80 00 00 00 3f 80 00 00 00 00 05 e8 \
             0e 52 61 76 65 6e 69 6e 67 20 4d 2d 36 30 00 10 08 52 01 00 00 00 00 5f 00 0c 00 00 dc 5a a3 00 00 03 56 47 00 82 00 1f",
        );
        let info = Message { kind: ENTITY_INFO, identity: (CHARACTER, 0x7b1e6d64), data }.entity_info().unwrap();
        assert_eq!((info.name.as_str(), info.health, info.family), ("Ravening M-60", Some((0x5aa3, 0x5aa3)), Some(0x5f)));
        let data = bytes(
            "00 3a 0a 2a 62 43 00 00 02 1c 43 d3 52 8a 41 70 28 f6 43 cc 56 e3 00 00 00 00 00 00 00 00 00 00 00 00 3f 80 00 00 00 00 05 e8 \
             18 57 69 64 6f 77 6d 61 6b 65 72 20 42 61 74 74 6c 65 20 44 72 6f 6e 65 00 10 08 12 01 00 00 00 00 5f 00 0a 00 00 dc 00 01 0f e9 \
             00 00 03 57 30 00 82 00 1f",
        );
        let info = Message { kind: ENTITY_INFO, identity: (CHARACTER, 0x7b1e6d65), data }.entity_info().unwrap();
        assert_eq!(info.health, Some((0x10fe9, 0x10fe9)));
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
