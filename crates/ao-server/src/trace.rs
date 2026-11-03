//! The server's trace: every frame that comes in and goes out, in a text file, so that what the original client sends and what it is
//! sent can be read, and what the server does not answer is found. The file is `MAO_TRACE` (default `ao-server-trace.log`).
//! Each line: the time, the direction, the connection (login or game), the message's kind and what it is called, its source and
//! destination, and its body in hex (cut at 1024 bytes). A line marked `UNANSWERED` or `UNHANDLED` is one the server did not act on.

use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use mao_net::frame::{Frame, N3, SYSTEM, TEXT};

static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Opens the trace file (the first call does; a file that cannot be opened is reported once, and tracing goes on without it).
pub fn open() {
    let path = std::env::var("MAO_TRACE").unwrap_or_else(|_| "ao-server-trace.log".to_string());
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => *FILE.lock().unwrap() = Some(f),
        Err(e) => eprintln!("ao-server: the trace file {path} cannot be opened ({e}); tracing is off"),
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

/// One line of the trace: a frame in (`in`) or out (`out`), on `connection` ("login" or "game"), with an optional `note`.
pub fn frame(direction: &str, connection: &str, frame: &Frame, note: &str) {
    let mut guard = FILE.lock().unwrap();
    let Some(file) = guard.as_mut() else { return };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let hex: String = frame.body.iter().take(1024).map(|b| format!("{b:02x}")).collect();
    let cut = if frame.body.len() > 1024 { " ..." } else { "" };
    let _ = writeln!(file, "{now:.3} {direction:<3} {connection:<5} {} src {:#x} dst {:#x} len {} {hex}{cut}{}", kind_name(frame), frame.source, frame.destination, frame.body.len(), if note.is_empty() { String::new() } else { format!(" [{note}]") });
}

/// A line of the trace that is not about a frame (what the server decided, what it refused).
pub fn note(connection: &str, text: &str) {
    let mut guard = FILE.lock().unwrap();
    let Some(file) = guard.as_mut() else { return };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let _ = writeln!(file, "{now:.3} --- {connection:<5} {text}");
}

/// The last `lines` lines of the trace (for the admin panel).
pub fn tail(lines: usize) -> String {
    let path = std::env::var("MAO_TRACE").unwrap_or_else(|_| "ao-server-trace.log".to_string());
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}
