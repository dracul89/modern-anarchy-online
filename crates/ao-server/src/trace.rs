//! The server's trace. Every connection gets a number, and each connection writes:
//!
//! * in `ao-server-trace.log` (or `MAO_TRACE`), one line per event: the connection's number and name, the time since it began,
//!   each frame that came in or went out (kind, what it is called, source, destination, body in hex), and every decision the
//!   server made (refusals, unanswered messages, UNHANDLED frames, how the connection ended);
//! * in the folder `MAO_LOG_DIR` (default `ao-server-logs`), the raw bytes as they crossed the socket: `cNNN-name-in.bin` for what
//!   the client sent and `cNNN-name-out.bin` for what the server sent (the compressed bytes, after the control packet).
//!
//! A line marked `UNANSWERED` or `UNHANDLED` is one the server did not act on.

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use mao_net::frame::{Frame, N3, SYSTEM, TEXT};

static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// The connection this thread serves: its number, its name ("login" or "game"), when it began.
    static CONN: Cell<(u64, &'static str, Option<Instant>)> = const { Cell::new((0, "none", None)) };
    /// The raw files of the connection: what came in, what went out.
    static RAW: RefCell<Option<(std::fs::File, std::fs::File)>> = const { RefCell::new(None) };
}

/// Opens the trace file (the first call does; a file that cannot be opened is reported once, and tracing goes on without it).
pub fn open() {
    let path = std::env::var("MAO_TRACE").unwrap_or_else(|_| "ao-server-trace.log".to_string());
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => *FILE.lock().unwrap() = Some(f),
        Err(e) => eprintln!("ao-server: the trace file {path} cannot be opened ({e}); tracing is off"),
    }
    let _ = std::fs::create_dir_all(log_dir());
}

fn log_dir() -> String {
    std::env::var("MAO_LOG_DIR").unwrap_or_else(|_| "ao-server-logs".to_string())
}

fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Starts a connection's trace on this thread: its number is returned, and its raw files are opened.
pub fn begin(name: &'static str, peer: &str) -> u64 {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    CONN.set((id, name, Some(Instant::now())));
    let dir = log_dir();
    let open = |dir_: &str, kind: &str| std::fs::OpenOptions::new().create(true).append(true).open(format!("{dir_}/c{id:04}-{name}-{kind}.bin")).ok();
    let raw = match (open(&dir, "in"), open(&dir, "out")) {
        (Some(i), Some(o)) => Some((i, o)),
        _ => None,
    };
    RAW.with(|r| *r.borrow_mut() = raw);
    note(&format!("begin from {peer}"));
    id
}

/// Ends the connection's trace on this thread, saying how it ended and how long it lasted.
pub fn end(how: &str) {
    let (id, name, started) = CONN.get();
    let took = started.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
    note(&format!("end after {took:.3} s: {how}"));
    RAW.with(|r| *r.borrow_mut() = None);
    CONN.set((id, name, None));
}

fn prefix() -> String {
    let (id, name, started) = CONN.get();
    let took = started.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
    format!("{:.3} c{id:04} {name:<5} +{took:7.3}s", now())
}

fn write_line(line: &str) {
    let mut guard = FILE.lock().unwrap();
    if let Some(file) = guard.as_mut() {
        let _ = writeln!(file, "{line}");
    }
}

/// The name the server knows a system message by (the login and chat messages of the captures).
fn system_name(kind: u32) -> &'static str {
    match kind {
        0x0e => "CharacterList",
        0x0f => "CreateCharacter",
        0x10 => "Refused",
        0x11 => "Created",
        0x14 => "DeleteCharacter",
        0x15 => "CharacterDeleted",
        0x16 => "LoginCharacter",
        0x17 => "Ticket",
        0x1b => "EnterGameServer",
        0x22 => "InitAuth",
        0x24 => "Challenge",
        0x25 => "AuthClient",
        0x3c => "Redirect",
        0x43 => "ChatServerInfo",
        0x55 => "SuggestName",
        0x56 => "SuggestedName",
        _ => "?",
    }
}

fn kind_name(frame: &Frame) -> String {
    match frame.kind {
        SYSTEM => {
            let kind = frame.body.get(..4).map(|b| u32::from_be_bytes(b.try_into().unwrap())).unwrap_or(0);
            format!("system {kind:#x} {}", system_name(kind))
        }
        N3 => match mao_net::n3::Message::parse(frame) {
            Some(m) => format!("n3 {:#010x} {} identity ({:x},{:x})", m.kind, mao_net::schema::Schema::global().n3_name(m.kind).unwrap_or("?"), m.identity.0, m.identity.1),
            None => "n3 (too short)".to_string(),
        },
        TEXT => "text".to_string(),
        other => format!("kind {other:#x}"),
    }
}

/// A frame in (`in`) or out (`out`), with an optional `note` of what the server did about it.
pub fn frame(direction: &str, frame: &Frame, note: &str) {
    let hex: String = frame.body.iter().take(1024).map(|b| format!("{b:02x}")).collect();
    let cut = if frame.body.len() > 1024 { " ..." } else { "" };
    let note = if note.is_empty() { String::new() } else { format!(" [{note}]") };
    write_line(&format!(
        "{} {direction:<3} {} src {:#x} dst {:#x} len {} {hex}{cut}{note}",
        prefix(),
        kind_name(frame),
        frame.source,
        frame.destination,
        frame.body.len()
    ));
}

/// A line of the trace that is not about a frame (what the server decided, what it refused, what went wrong).
pub fn note(text: &str) {
    write_line(&format!("{} --- {text}", prefix()));
}

/// The bytes that came in on the socket, as they came.
pub fn raw_in(bytes: &[u8]) {
    RAW.with(|r| {
        if let Some((i, _)) = r.borrow_mut().as_mut() {
            let _ = i.write_all(bytes);
        }
    });
}

/// The bytes that go out on the socket, as they are sent.
pub fn raw_out(bytes: &[u8]) {
    RAW.with(|r| {
        if let Some((_, o)) = r.borrow_mut().as_mut() {
            let _ = o.write_all(bytes);
        }
    });
}

/// The last `lines` lines of the trace (for the admin panel).
pub fn tail(lines: usize) -> String {
    let path = std::env::var("MAO_TRACE").unwrap_or_else(|_| "ao-server-trace.log".to_string());
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}
