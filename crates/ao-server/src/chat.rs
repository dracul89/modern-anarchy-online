//! The chat manager: the second protocol of the login connection, the packets
//! of the public Anarchy Online chat protocol (the ones the captures show).

/// A chat packet's kind and payload as the original client frames it.
pub const SALT: u16 = 0;
pub const LOGIN_OK: u16 = 5;
pub const NAME: u16 = 20;
pub const NAME_ANSWER: u16 = 21;
pub const VICINITY: u16 = 34;
pub const NOTICE: u16 = 35;
pub const CHANNEL_INFO: u16 = 60;
pub const TELL: u16 = 30;
pub const BUDDY_ADD: u16 = 40;
pub const BUDDY_REMOVE: u16 = 41;
pub const MESSAGE: u16 = 65;

/// The answer to a chat login: the login ok (the salt was answered).
pub fn login_ok() -> Vec<u8> {
    let mut packet = vec![0u8, LOGIN_OK as u8];
    packet.extend_from_slice(&0u16.to_be_bytes()[..]);
    packet
}

/// The answer to a name lookup: the name of the character with this id.
pub fn name(id: u32, name: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&id.to_be_bytes());
    let n = name.as_bytes();
    payload.extend_from_slice(&(n.len() as u16).to_be_bytes()[..]);
    payload.extend_from_slice(n);
    let mut packet = vec![0u8, NAME_ANSWER as u8];
    packet.extend_from_slice(&(payload.len() as u16).to_be_bytes()[..]);
    packet.extend_from_slice(&payload[..]);
    packet
}

/// The answer to a tell: the server accepted it (for now).
pub fn tell_ok() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&[0, 0]);
    let mut packet = vec![0u8, LOGIN_OK as u8];
    packet.extend_from_slice(&(payload.len() as u16).to_be_bytes()[..]);
    packet.extend_from_slice(&payload[..]);
    packet
}

/// The answer to a channel message: the server accepted it (for now).
pub fn channel_ok(kind: u8, id: u32) -> Vec<u8> {
    let mut payload = vec![kind];
    payload.extend_from_slice(&id.to_be_bytes());
    let mut packet = vec![0u8, MESSAGE as u8];
    packet.extend_from_slice(&(payload.len() as u16).to_be_bytes()[..]);
    packet.extend_from_slice(&payload[..]);
    packet
}
