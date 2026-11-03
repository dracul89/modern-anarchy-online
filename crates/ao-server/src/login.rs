//! The login / chat-manager connection: the state machine of the original
//! client's first messages (system 0x22, 0x24, 0x25, 0x0e, 0x16, 0x17, 0x1b).

use std::net::TcpStream;

use mao_net::frame::{Frame, Reader};
use mao_net::frame::SYSTEM;
use mao_net::system::Character;
use mao_net::create::NewCharacter;

pub const INIT_AUTH: u32 = 0x22;
pub const CHALLENGE: u32 = 0x24;
pub const AUTH_CLIENT: u32 = 0x25;
pub const CHARACTER_LIST: u32 = 0x0e;
pub const LOGIN_CHARACTER: u32 = 0x16;
pub const TICKET: u32 = 0x17;
pub const ENTER_GAME_SERVER: u32 = 0x1b;
pub const REDIRECT: u32 = 0x3c;
pub const CHAT_SERVER_INFO: u32 = 0x43;
pub const CREATE_CHARACTER: u32 = 0x0f;
pub const CREATE_REFUSED: u32 = 0x10;
pub const CREATED: u32 = 0x11;
pub const DELETE_CHARACTER: u32 = 0x14;
pub const CHARACTER_DELETED: u32 = 0x15;
pub const SUGGEST_NAME: u32 = 0x55;
pub const SUGGESTED_NAME: u32 = 0x56;

/// The chat server of the test server: the login / chat manager listens here.
pub const CHAT_SERVER: &str = "127.0.0.1:7105";

/// The slots of an account: the ones the original's character creation refuses over.
pub const MAX_CHARACTERS: usize = 6;

/// The next character id the server makes.
pub fn next_character_id() -> u32 {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x7b1a0021);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    InitAuth,
    AwaitingAuth,
    Authenticated,
    CharacterSelected,
}

pub struct LoginSession {
    pub stream: TcpStream,
    pub reader: Reader,
    /// The game server the tickets and redirects name: its address and port.
    game: ([u8; 4], u16),
    phase: Phase,
    pub account: String,
    pub character_id: u32,
    challenge_text: String,
    ticket: mao_net::system::Ticket,
    created: Option<NewCharacter>,
}

impl LoginSession {
    pub fn new(stream: TcpStream) -> Self {
        Self::with_game(stream, ([127, 0, 0, 1], 7500))
    }

    /// A login whose tickets and redirects name the game server at `game` (its address and port).
    pub fn with_game(stream: TcpStream, game: ([u8; 4], u16)) -> Self {
        let mut seed = 0x1234_5678u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as usize
        };
        let charset: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let challenge_text: String = (0..32).map(|_| charset[next() % charset.len()] as char).collect();
        LoginSession {
            stream,
            reader: Reader::default(),
            game,
            phase: Phase::InitAuth,
            account: String::new(),
            character_id: 0,
            challenge_text,
            ticket: mao_net::system::Ticket { id: 0, address: [0; 4], port: 0, key: [1, 2, 3, 4, 5, 6, 7, 8], rest: Vec::new() },
            created: None,
        }
    }

    pub fn phase(&self) -> Phase { self.phase }

    pub fn character_id(&self) -> u32 { self.character_id }

    /// The character the server is making, if the creation is being read.
    pub fn take_created(&mut self) -> Option<NewCharacter> { self.created.take() }

    /// The frames to send back in answer, and the phase they leave the login in.
    pub fn handle(&mut self, frame: &Frame, chars: &[Character]) -> (Phase, Vec<Frame>) {
        let mut out = Vec::new();
        if frame.kind != SYSTEM {
            return (self.phase, out);
        }
        let kind = u32::from_be_bytes(frame.body.get(..4).map(|b| b.try_into().unwrap()).unwrap_or([0u8; 4]));
        match kind {
            INIT_AUTH => {
                self.phase = Phase::AwaitingAuth;
                self.account = account_name(frame);
                out.push(crate::frames::challenge(1, 0, &self.challenge_text));
            }
            AUTH_CLIENT => {
                self.phase = Phase::Authenticated;
                out.push(crate::frames::character_list(1, self.character_id, chars));
            }
            LOGIN_CHARACTER => {
                self.phase = Phase::CharacterSelected;
                self.character_id = frame.source;
                // (the rest: the event server type, 0, and the player id, the captured ticket's last eight bytes)
                let mut rest = 0u32.to_be_bytes().to_vec();
                rest.extend_from_slice(&self.character_id.to_be_bytes());
                self.ticket = mao_net::system::Ticket { id: 1, address: self.game.0, port: self.game.1, key: [1, 2, 3, 4, 5, 6, 7, 8], rest };
                out.push(crate::frames::ticket(1, self.character_id, &self.ticket));
            }
            REDIRECT => {
                out.push(crate::frames::redirect(1, self.character_id, self.game.0, self.game.1));
            }
            CHAT_SERVER_INFO => {
                out.push(crate::frames::chat_server_info(1, self.character_id, CHAT_SERVER));
            }
            CREATE_CHARACTER => {
                self.phase = Phase::CharacterSelected;
                let id = next_character_id();
                self.character_id = id;
                if chars.len() >= MAX_CHARACTERS {
                    // The account is full: the creation is refused (the code the original's client shows in its slot window).
                    out.push(crate::frames::refused(1, id, 2));
                    return (self.phase, out);
                }
                let new_character = crate::create::read_new_character(frame.body.get(4..).unwrap_or(&[])).unwrap_or(NewCharacter { name: self.account.clone(), breed: 1, sex: 2, profession: 1, head: 0, height: 100, width: 0 });
                self.created = Some(new_character);
                out.push(crate::frames::created(1, id, id));
            }
            CREATE_REFUSED => {
                self.phase = Phase::CharacterSelected;
                out.push(crate::frames::refused(1, self.character_id, u32::from_be_bytes(frame.body.get(4..8).map(|b| b.try_into().unwrap()).unwrap_or([0u8; 4])) as i32));
            }
            DELETE_CHARACTER => {
                let id = u32::from_be_bytes(frame.body.get(4..8).map(|b| b.try_into().unwrap()).unwrap_or([0u8; 4]));
                out.push(crate::frames::deleted(1, id, id));
            }
            SUGGEST_NAME => {
                // The server suggests a name for the look the client asked about (the window's Suggest Name).
                out.push(crate::frames::suggested_name(1, self.character_id, &suggested_name_text()));
            }
            _ => {}
        }
        (self.phase, out)
    }
}

/// The name the server suggests for a new character of the requested look.
fn suggested_name_text() -> String {
    static NAMES: &[&str] = &["Dorian", "Elena", "Farid", "Gita", "Hassan", "Ilse", "Joric", "Kara", "Lukas", "Mira", "Nils", "Oona"];
    let n = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) % NAMES.len() as u64) as usize;
    NAMES[n].to_string()
}

/// The account name of an `INIT_AUTH` message: the NUL-padded name after the
/// count, at the payload's start (the payload is `[count][name: 0x28][version: 0x14]`).
fn account_name(frame: &Frame) -> String {
    let mut name = String::new();
    for b in frame.body.get(8..40).unwrap_or(&[]) {
        if *b == 0 { break; }
        name.push(*b as char);
    }
    name
}
