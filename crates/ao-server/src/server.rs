//! The two listeners of the server: the login / chat manager and the game. The accounts, the characters and their progress are in
//! the database (`db`), read as the messages need them.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mao_net::frame::Frame;
use mao_net::frame::Reader;
use mao_net::frame::SYSTEM;
use mao_net::socket;

use crate::db::Db;
use crate::game::GameSession;
use crate::progress::Hero;

/// The most characters an account has.
pub const MAX_CHARACTERS: usize = 6;

pub struct Server {
    pub login_addr: SocketAddr,
    pub game_addr: SocketAddr,
    db: Arc<Db>,
    /// The character of the connection that is in the world.
    current: Arc<Mutex<u32>>,
}

impl Server {
    pub fn new(login_addr: SocketAddr, game_addr: SocketAddr, db_path: &str) -> Self {
        Server { login_addr, game_addr, db: Arc::new(Db::open(db_path)), current: Arc::new(Mutex::new(0)) }
    }

    /// The database the server keeps its accounts and characters in.
    pub fn db(&self) -> Arc<Db> {
        self.db.clone()
    }

    /// The two listeners of the server: the login / chat manager and the game.
    pub fn spawn_listeners(&self) -> std::io::Result<()> {
        // (the tickets name the game server as the address it listens on, an IPv4 one)
        let game = match self.game_addr {
            SocketAddr::V4(v4) => (v4.ip().octets(), v4.port()),
            SocketAddr::V6(_) => ([127, 0, 0, 1], self.game_addr.port()),
        };
        let login_listener = TcpListener::bind(self.login_addr)?;
        let (db, current) = (self.db.clone(), self.current.clone());
        std::thread::spawn(move || {
            let _ = relay(login_listener, db, current, true, game);
        });
        let game_listener = TcpListener::bind(self.game_addr)?;
        let (db, current) = (self.db.clone(), self.current.clone());
        std::thread::spawn(move || {
            let _ = relay(game_listener, db, current, false, game);
        });
        Ok(())
    }
}

fn relay(listener: TcpListener, db: Arc<Db>, current: Arc<Mutex<u32>>, login: bool, game: ([u8; 4], u16)) -> std::io::Result<()> {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let (db, current) = (db.clone(), current.clone());
        std::thread::spawn(move || {
            let _ = handle(stream, db, current, login, game);
        });
    }
    Ok(())
}

fn handle(stream: TcpStream, db: Arc<Db>, current: Arc<Mutex<u32>>, login: bool, game: ([u8; 4], u16)) -> std::io::Result<()> {
    // (the reads wait with a timeout, so the connection's clock runs: a stream must be non-blocking for that)
    mao_net::socket::prepare(&stream)?;
    if login {
        return handle_login(stream, db, current, game);
    }
    handle_game(stream, db, current)
}

fn handle_login(mut stream: TcpStream, db: Arc<Db>, current: Arc<Mutex<u32>>, game: ([u8; 4], u16)) -> std::io::Result<()> {
    let mut session = crate::login::LoginSession::with_game(stream.try_clone()?, game);
    let mut reader = Reader::default();
    let mut sequence: u16 = 0;
    loop {
        let Some(frame) = read_frame(&mut stream, &mut reader)? else { return Ok(()) };
        crate::trace::frame("in", "login", &frame, "");
        let chars = db.characters_of(&session.account);
        let (_phase, mut replies) = session.handle(&frame, &chars);
        let kind = system_kind(&frame);
        // (accounts are made in the admin panel: a login for one that is not there is refused)
        let (slots, expansions) = db.account_settings(&session.account).unwrap_or((0, 0));
        if kind == crate::login::AUTH_CLIENT {
            if db.account_exists(&session.account) {
                replies = vec![crate::frames::character_list_slots(1, session.character_id(), &chars, slots as i32, expansions as i32)];
            } else {
                crate::trace::note("login", &format!("account {:?} is not known: refused", session.account));
                replies = vec![crate::frames::refused(1, session.character_id(), 1)];
            }
        }
        // (a creation past the account's slots is refused with code 8, "CantCreateMoreChars" in the client's table)
        if kind == crate::login::CREATE_CHARACTER && chars.len() as i64 >= slots {
            crate::trace::note("login", &format!("no slot left for {:?} ({} of {slots} used)", session.account, chars.len()));
            replies = vec![crate::frames::refused(1, session.character_id(), 8)];
        }
        if kind != 0 && replies.is_empty() {
            crate::trace::note("login", &format!("UNANSWERED system kind {kind:#x}"));
        }
        if kind == crate::login::LOGIN_CHARACTER {
            let id = u32::from_be_bytes(frame.body.get(4..8).map(|b| b.try_into().unwrap()).unwrap_or([0u8; 4]));
            *current.lock().unwrap() = id;
        }
        if kind == crate::login::DELETE_CHARACTER {
            let id = u32::from_be_bytes(frame.body.get(4..8).map(|b| b.try_into().unwrap()).unwrap_or([0u8; 4]));
            let _ = db.delete_character(&session.account, id as i64);
        }
        if kind == crate::login::CREATE_CHARACTER {
            let made = session.take_created();
            let id = replies.iter().find_map(created_id_of);
            if let (Some(c), Some(id)) = (made, id) {
                if (chars.len() as i64) < slots {
                    let character = mao_net::system::Character { id, created: true, name: c.name.clone(), breed: c.breed, sex: c.sex, profession: c.profession, level: 1, area: "area unknown".to_string(), banned: false, ban_reason: String::new(), head: c.head, height: c.height, width: c.width, flag: 1, proxy: [0; 6] };
                    let _ = db.insert_character(&session.account, &character, id as i64);
                }
            }
        }
        for reply in replies {
            crate::trace::frame("out", "login", &reply, "");
            sequence = sequence.wrapping_add(1);
            if sequence == 0xffff {
                sequence = 0;
            }
            let bytes = reply.encode(sequence);
            socket::write_all(&mut stream, &bytes)?;
        }
    }
}

/// How often the game connection wakes without a frame, to let the world move on (a fight's swings).
const TICK: Duration = Duration::from_millis(250);

fn handle_game(mut stream: TcpStream, db: Arc<Db>, current: Arc<Mutex<u32>>) -> std::io::Result<()> {
    let id = *current.lock().unwrap();
    let hero = db.load_hero(id).unwrap_or_else(|| Hero::new(id));
    let mut game = GameSession::new(id, hero, Some(db));
    let mut reader = Reader::default();
    let mut sequence: u16 = 0;
    loop {
        let mut replies = match read_frame_within(&mut stream, &mut reader, TICK) {
            Ok(Some(frame)) => {
                crate::trace::frame("in", "game", &frame, "");
                if !crate::game::answers(&frame) {
                    crate::trace::note("game", &format!("UNHANDLED: {}", crate::game::describe(&frame)));
                }
                game.handle(&frame).1
            }
            Ok(None) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Vec::new(),
            Err(e) => return Err(e),
        };
        replies.extend(game.tick(std::time::Instant::now()));
        for reply in replies {
            crate::trace::frame("out", "game", &reply, "");
            sequence = sequence.wrapping_add(1);
            if sequence == 0xffff {
                sequence = 0;
            }
            let bytes = reply.encode(sequence);
            socket::write_all(&mut stream, &bytes)?;
        }
    }
}

fn system_kind(frame: &Frame) -> u32 {
    if frame.kind != SYSTEM {
        return 0;
    }
    u32::from_be_bytes(frame.body.get(..4).map(|b| b.try_into().unwrap()).unwrap_or([0u8; 4]))
}

fn created_id_of(reply: &Frame) -> Option<u32> {
    if system_kind(reply) != crate::login::CREATED {
        return None;
    }
    Some(u32::from_be_bytes(reply.body.get(4..8).map(|b| b.try_into().unwrap()).unwrap_or([0u8; 4])))
}

/// The next frame of a login connection, however long the wait (a client may sit at the login screen for a while).
fn read_frame(stream: &mut TcpStream, reader: &mut Reader) -> std::io::Result<Option<Frame>> {
    loop {
        match read_frame_within(stream, reader, Duration::from_secs(30)) {
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            other => return other,
        }
    }
}

/// The next frame, waiting up to `within` for one to come (a `TimedOut` error when none did; the reader keeps what it has).
fn read_frame_within(stream: &mut TcpStream, reader: &mut Reader, within: Duration) -> std::io::Result<Option<Frame>> {
    let mut buffer = [0u8; 8192];
    loop {
        if let Some(frame) = reader.next().map_err(|e| std::io::Error::other(e.to_string()))? {
            return Ok(Some(frame));
        }
        let n = socket::read(stream, &mut buffer, within)?;
        if n == 0 {
            return Ok(None);
        }
        reader.push(&buffer[..n]).map_err(|e| std::io::Error::other(e.to_string()))?;
    }
}
