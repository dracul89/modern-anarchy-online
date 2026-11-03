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
    let name = if login { "login" } else { "game" };
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "unknown".to_string());
    crate::trace::begin(name, &peer);
    // (the reads wait with a timeout, so the connection's clock runs: a stream must be non-blocking for that)
    if let Err(e) = mao_net::socket::prepare(&stream) {
        crate::trace::end(&format!("could not be prepared: {e}"));
        return Err(e);
    }
    let result = if login { handle_login(stream, db, current, game) } else { handle_game(stream, db, current) };
    // (the trace says how each connection ended: a client that left, or a link that broke)
    match &result {
        Ok(()) => crate::trace::end("finished: the client closed the connection (see the line above)"),
        Err(e) => crate::trace::end(&format!("error: {e} ({:?})", e.kind())),
    }
    result
}

/// Writes bytes to the client, recording them; a failure is recorded with its kind before it is returned.
fn send(stream: &mut TcpStream, bytes: &[u8]) -> std::io::Result<()> {
    crate::trace::raw_out(bytes);
    let result = socket::write_all(stream, bytes);
    if let Err(e) = &result {
        crate::trace::note(&format!("write failed: {e} ({:?}) after {} bytes", e.kind(), bytes.len()));
    }
    result
}

fn handle_login(mut stream: TcpStream, db: Arc<Db>, current: Arc<Mutex<u32>>, game: ([u8; 4], u16)) -> std::io::Result<()> {
    let mut session = crate::login::LoginSession::with_game(stream.try_clone()?, game);
    let mut reader = Reader::default();
    let mut sequence: u16 = 0;
    loop {
        let Some(frame) = read_frame(&mut stream, &mut reader)? else { return Ok(()) };
        crate::trace::frame("in", &frame, "");
        let chars = db.characters_of(&session.account);
        let (_phase, mut replies) = session.handle(&frame, &chars);
        let kind = system_kind(&frame);
        // (accounts are made in the admin panel: a login for one that is not there is refused)
        let (slots, expansions) = db.account_settings(&session.account).unwrap_or((0, 0));
        if kind == crate::login::AUTH_CLIENT {
            if db.account_exists(&session.account) {
                replies = vec![crate::frames::character_list_slots(1, session.character_id(), &chars, slots as i32, expansions as i32)];
            } else {
                crate::trace::note(&format!("account {:?} is not known: refused", session.account));
                replies = vec![crate::frames::refused(1, session.character_id(), 1)];
            }
        }
        // (a creation past the account's slots is refused with code 8, "CantCreateMoreChars" in the client's table)
        if kind == crate::login::CREATE_CHARACTER && chars.len() as i64 >= slots {
            crate::trace::note(&format!("no slot left for {:?} ({} of {slots} used)", session.account, chars.len()));
            replies = vec![crate::frames::refused(1, session.character_id(), 8)];
        }
        if kind != 0 && replies.is_empty() {
            crate::trace::note(&format!("UNANSWERED system kind {kind:#x}"));
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
            crate::trace::frame("out", &reply, "");
            sequence = sequence.wrapping_add(1);
            if sequence == 0xffff {
                sequence = 0;
            }
            let bytes = reply.encode(sequence);
            send(&mut stream, &bytes)?;
        }
    }
}

/// How often the game connection wakes without a frame, to let the world move on (a fight's swings).
const TICK: Duration = Duration::from_millis(250);

/// The control packet the game server sends when it starts compressing: the captures' sixteen bytes (sequence 0xdfdf, kind 0x7f00,
/// one, size 16, the source 0x01000000 and destination 0). From then on the server's frames are one zlib stream, the frames inside it.
const COMPRESS_ON: [u8; 16] = [0xdf, 0xdf, 0x7f, 0x00, 0x00, 0x01, 0x00, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// The game connection's compression: each frame is written into one zlib stream, flushed after every frame so the client reads
/// it as it comes.
struct Compressed(flate2::write::ZlibEncoder<Vec<u8>>);

impl Compressed {
    fn new() -> Self {
        Compressed(flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast()))
    }

    /// The bytes of a frame as they go out: the part of the zlib stream this frame makes. Inside a compressed stream a frame is not
    /// padded to four bytes (the captures' frames follow each other exactly), so the padding is cut here.
    fn pack(&mut self, bytes: &[u8]) -> std::io::Result<Vec<u8>> {
        use std::io::Write;
        let size = u16::from_be_bytes([bytes[6], bytes[7]]) as usize;
        self.0.write_all(&bytes[..size.min(bytes.len())])?;
        self.0.flush()?;
        Ok(std::mem::take(self.0.get_mut()))
    }
}

fn handle_game(mut stream: TcpStream, db: Arc<Db>, current: Arc<Mutex<u32>>) -> std::io::Result<()> {
    let id = *current.lock().unwrap();
    let hero = db.load_hero(id).unwrap_or_else(|| Hero::new(id));
    let mut game = GameSession::new(id, hero, Some(db));
    let mut reader = Reader::default();
    let mut sequence: u16 = 0;
    let mut zip: Option<Compressed> = None;
    loop {
        // (the entry into the world turns the compression on: the control packet goes out before the frames of the answer)
        let mut entering = false;
        let mut replies = match read_frame_within(&mut stream, &mut reader, TICK) {
            Ok(Some(frame)) => {
                crate::trace::frame("in", &frame, "");
                if !crate::game::answers(&frame) {
                    crate::trace::note(&format!("UNHANDLED: {}", crate::game::describe(&frame)));
                }
                entering = crate::game::is_enter(&frame);
                game.handle(&frame).1
            }
            Ok(None) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Vec::new(),
            Err(e) => return Err(e),
        };
        replies.extend(game.tick(std::time::Instant::now()));
        if entering && zip.is_none() {
            send(&mut stream, &COMPRESS_ON)?;
            zip = Some(Compressed::new());
        }
        for reply in replies {
            crate::trace::frame("out", &reply, "");
            sequence = sequence.wrapping_add(1);
            if sequence == 0xffff {
                sequence = 0;
            }
            let bytes = reply.encode(sequence);
            let bytes = match zip.as_mut() {
                Some(z) => z.pack(&bytes)?,
                None => bytes,
            };
            send(&mut stream, &bytes)?;
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
        match reader.next() {
            Ok(Some(frame)) => return Ok(Some(frame)),
            Ok(None) => {}
            Err(e) => {
                crate::trace::note(&format!("frame decoding failed: {e}"));
                return Err(std::io::Error::other(e.to_string()));
            }
        }
        let n = match socket::read(stream, &mut buffer, within) {
            Ok(n) => n,
            // (a quiet spell is normal: the game connection wakes for its ticks)
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => return Err(e),
            Err(e) => {
                crate::trace::note(&format!("read failed: {e} ({:?})", e.kind()));
                return Err(e);
            }
        };
        if n == 0 {
            crate::trace::note("the client closed its end of the connection (end of stream)");
            return Ok(None);
        }
        crate::trace::raw_in(&buffer[..n]);
        if let Err(e) = reader.push(&buffer[..n]) {
            crate::trace::note(&format!("frame decoding failed on {n} bytes: {e}"));
            return Err(std::io::Error::other(e.to_string()));
        }
    }
}

#[cfg(test)]
mod compression_tests {
    use super::*;

    #[test]
    fn compressed_frames_read_back_after_the_control_packet() {
        let mut stream: Vec<u8> = COMPRESS_ON.to_vec();
        let mut zip = Compressed::new();
        let mut seq = 1u16;
        for body in [vec![1u8, 2, 3, 4, 5, 6, 7, 8], vec![9u8; 40]] {
            let f = Frame { kind: SYSTEM, source: 1, destination: 2, body };
            stream.extend(zip.pack(&f.encode(seq)).unwrap());
            seq += 1;
        }
        // (fed in pieces, as the socket delivers them)
        let mut reader = Reader::default();
        for piece in stream.chunks(7) {
            reader.push(piece).unwrap();
        }
        let a = reader.next().unwrap().expect("the first frame");
        assert_eq!(a.body, vec![1u8, 2, 3, 4, 5, 6, 7, 8]);
        let b = reader.next().unwrap().expect("the second frame");
        assert_eq!(b.body.len(), 40);
    }
}
