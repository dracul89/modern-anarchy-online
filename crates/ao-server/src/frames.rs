//! Server-side frame builders, in the shapes the captures show.

use mao_net::frame::{Frame, N3, SYSTEM, TEXT};
use mao_net::system::Character;

/// A system frame: kind 1, body `[type u32][payload...]`.
pub fn system(source: u32, destination: u32, kind: u32, payload: &[u8]) -> Frame {
    let mut body = kind.to_be_bytes().to_vec();
    body.extend_from_slice(payload);
    Frame { kind: SYSTEM, source, destination, body }
}

/// system 0x24: the login challenge, the 32-character text.
pub fn challenge(source: u32, destination: u32, text: &str) -> Frame {
    let mut payload = Vec::with_capacity(32);
    payload.extend_from_slice(text.as_bytes());
    system(source, destination, 0x24, &payload)
}

fn int(p: &mut Vec<u8>, v: i32) {
    p.extend_from_slice(&v.to_be_bytes());
}

fn string(p: &mut Vec<u8>, s: &str) {
    int(p, s.len() as i32);
    p.extend_from_slice(s.as_bytes());
}

/// system 0x0e: the character list, one `CharacterData_t` per character.
pub fn character_list(source: u32, destination: u32, chars: &[Character]) -> Frame {
    character_list_slots(source, destination, chars, 6, 1)
}

/// system 0x0e with the account's slots and expansion flags after the characters (the captures' trailer: the client reads the
/// slots it shows as "x/y slots available" from it, and the flags as `ExpansionFlags`), then a zero.
pub fn character_list_slots(source: u32, destination: u32, chars: &[Character], slots: i32, expansions: i32) -> Frame {
    let mut payload = Vec::new();
    int(&mut payload, chars.len() as i32);
    for c in chars {
        int(&mut payload, 4);
        int(&mut payload, c.id as i32);
        payload.push(b'a');
        for w in [0xc79c, 0x21c, 0, 0, 0, 0] {
            int(&mut payload, w);
        }
        int(&mut payload, i32::from(c.created));
        int(&mut payload, 5);
        int(&mut payload, c.id as i32);
        int(&mut payload, 0);
        string(&mut payload, &c.name);
        int(&mut payload, c.breed);
        int(&mut payload, c.sex);
        int(&mut payload, c.profession);
        int(&mut payload, c.level);
        string(&mut payload, &c.area);
        int(&mut payload, i32::from(c.banned));
        string(&mut payload, &c.ban_reason);
        int(&mut payload, c.head);
        int(&mut payload, c.height);
        int(&mut payload, c.width);
        int(&mut payload, c.flag);
    }
    int(&mut payload, slots);
    int(&mut payload, expansions);
    int(&mut payload, 0);
    system(source, destination, 0x0e, &payload)
}

/// system 0x11: the answer to a creation (`Created` with the new character's id).
pub fn created(source: u32, destination: u32, id: u32) -> Frame {
    system(source, destination, 0x11, &id.to_be_bytes())
}

/// system 0x10: the answer to a creation or a login (`Refused` with the code).
pub fn refused(source: u32, destination: u32, code: i32) -> Frame {
    system(source, destination, 0x10, &code.to_be_bytes())
}

/// system 0x15: the answer to a deletion (`CHARACTER_DELETED` with the id).
pub fn deleted(source: u32, destination: u32, id: u32) -> Frame {
    system(source, destination, crate::create::CHARACTER_DELETED, &id.to_be_bytes())
}

/// system 0x56: the suggested name of a creation (`SUGGESTED_NAME` with a `u16` length and the bytes).
pub fn suggested_name(source: u32, destination: u32, name: &str) -> Frame {
    let mut payload = Vec::new();
    let n = name.as_bytes();
    payload.extend_from_slice(&(n.len() as u16).to_be_bytes()[..]);
    payload.extend_from_slice(n);
    system(source, destination, crate::create::SUGGESTED_NAME, &payload)
}

/// system 0x17: the ticket of the character that logged in.
pub fn ticket(source: u32, destination: u32, ticket: &mao_net::system::Ticket) -> Frame {
    let mut payload = ticket.id.to_be_bytes().to_vec();
    payload.extend_from_slice(&ticket.address);
    payload.extend_from_slice(&ticket.port.to_be_bytes());
    payload.extend_from_slice(&ticket.key);
    payload.extend_from_slice(&ticket.rest);
    system(source, destination, 0x17, &payload)
}

/// system 0x3c: the redirect to another game server (address and port).
pub fn redirect(source: u32, destination: u32, address: [u8; 4], port: u16) -> Frame {
    let mut payload = address.to_vec();
    payload.extend_from_slice(&port.to_be_bytes()[..]);
    system(source, destination, 0x3c, &payload)
}

/// system 0x43: the chat server info (the address as text, a length and the bytes).
pub fn chat_server_info(source: u32, destination: u32, text: &str) -> Frame {
    let mut payload = Vec::new();
    let b = text.as_bytes();
    payload.extend_from_slice(&(b.len() as u32).to_be_bytes());
    payload.extend_from_slice(b);
    system(source, destination, 0x43, &payload)
}

/// A text frame (kind 5): the object, the target, the data length and the data.
pub fn text(source: u32, destination: u32, body: &[u8]) -> Frame {
    Frame { kind: TEXT, source, destination, body: body.to_vec() }
}

/// An N3 frame (kind 10): the message type, the identity and the data.
pub fn n3(source: u32, destination: u32, kind: u32, identity: (u32, u32), data: &[u8]) -> Frame {
    let mut body = kind.to_be_bytes().to_vec();
    body.extend_from_slice(&identity.0.to_be_bytes());
    body.extend_from_slice(&identity.1.to_be_bytes());
    body.extend_from_slice(data);
    Frame { kind: N3, source, destination, body }
}

/// The playfield the character is in (0x5f4b1a39): the instance, the arrival position.
pub fn playfield(source: u32, destination: u32, instance: u32, position: [f32; 3]) -> Frame {
    // (the client reads the arrival as the captures lay it out: four zero bytes, a byte of 4, then the position's three floats at
    // 5; the playfield is the identity's instance, of the type 0x9c50 the captures show)
    let mut data = vec![0, 0, 0, 0, 4];
    for v in position {
        data.extend_from_slice(&v.to_be_bytes());
    }
    n3(source, destination, 0x5f4b1a39, (0x9c50, instance), &data)
}

/// The server clock (0x5f52412e): seconds into the 27-hour day, the day period, the day count, the server time.
pub fn game_time(source: u32, destination: u32, seconds: f32, period: u32, days: u32, server_time: u32) -> Frame {
    let mut data = Vec::with_capacity(20);
    data.extend_from_slice(&seconds.to_be_bytes());
    data.extend_from_slice(&period.to_be_bytes());
    data.extend_from_slice(&days.to_be_bytes());
    data.extend_from_slice(&server_time.to_be_bytes());
    n3(source, destination, 0x5f52412e, (0xc79c, 0), &data)
}

/// The client's first message after entering the world (0x570c2039).
pub fn ready(character: u32) -> Frame {
    mao_net::n3::ready(character)
}

/// The stand-up action (0x57) that frees a new connection.
pub fn stand_up(character: u32) -> Frame {
    mao_net::n3::action(character, 0x57)
}

/// The attack (0x28494070).
pub fn attack(character: u32, target: u32) -> Frame {
    mao_net::n3::attack(character, target)
}

/// The target info request (0x5e477770 action 0x69).
pub fn info_request(character: u32, target: u32) -> Frame {
    mao_net::n3::info_request(character, target)
}

/// The character's position update: the movement type, the rotation, the position, and the elapsed ms.
pub fn position_update(character: u32, movement: u8, rotation: [f32; 4], position: [f32; 3], elapsed_ms: u32) -> Frame {
    mao_net::n3::position_update_pitched(character, movement, rotation[1], rotation[2], position, elapsed_ms)
}
