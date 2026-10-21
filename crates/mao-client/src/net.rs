//! `mao-client net ...`: the network side.
//!
//! Credentials come from the environment (`MAO_USER`, `MAO_PASSWORD`),
//! never from arguments or files, and are never printed. The server is
//! Rubi-Ka unless `MAO_SERVER` (`host:port`) says otherwise.

use std::path::Path;

use mao_net::frame::{self, Reader};
use mao_net::session::{self, Connection};
use mao_net::system::{self, Character};

fn show(list: &[Character]) {
    for c in list {
        println!("{:>10} {:<16} level {:>3} profession {} breed {} sex {} in {:?} (proxy {:x?})", c.id, c.name, c.level, c.profession, c.breed, c.sex, c.area, &c.proxy[..2]);
    }
}

/// The game's version: the first line of `version.id`.
fn version(game: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(Path::new(game).join("version.id")).map_err(|e| format!("version.id: {e}"))?;
    Ok(text.lines().next().unwrap_or("").trim().to_string())
}

pub fn run(args: &[String]) -> Result<(), String> {
    match args {
        [game, word] if word == "chars" => {
            let user = std::env::var("MAO_USER").map_err(|_| "set MAO_USER and MAO_PASSWORD")?;
            let password = std::env::var("MAO_PASSWORD").map_err(|_| "set MAO_USER and MAO_PASSWORD")?;
            let server = std::env::var("MAO_SERVER").unwrap_or_else(|_| session::RUBI_KA.to_string());
            let mut connection = Connection::connect(server.as_str()).map_err(|e| e.to_string())?;
            let list = session::login(&mut connection, &user, &password, &version(game)?).map_err(|e| e.to_string())?;
            show(&list);
            Ok(())
        }
        [word, log] if word == "parse-capture" => parse_capture(log),
        _ => Err("usage: mao-client net <game dir> chars | net parse-capture <capture.log>".into()),
    }
}

/// The server's side of a `tools/capture_proxy.py` log: its messages, and
/// the characters of any list among them.
fn parse_capture(log: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let mut reader = Reader::default();
    for line in text.lines() {
        let mut f = line.split_whitespace();
        let (_time, tag, _len, hex) = (f.next(), f.next(), f.next(), f.next());
        match (tag, hex) {
            (Some("OPEN"), _) => reader = Reader::default(),
            (Some("S>C"), Some(hex)) => {
                let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()).collect();
                reader.push(&bytes);
                while let Some(m) = reader.next().map_err(|e| e.to_string())? {
                    let kind = if m.kind == frame::SYSTEM { system::split(&m).map(|(k, _)| k).unwrap_or(0) } else { 0 };
                    println!("server: kind {} system type {kind:#x}, {} bytes", m.kind, m.body.len());
                    if kind == system::CHARACTER_LIST {
                        let (list, rest) = system::character_list(&m).map_err(|e| e.to_string())?;
                        show(&list);
                        println!("after the list: {rest:02x?}");
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
