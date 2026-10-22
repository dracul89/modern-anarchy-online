//! Where each whompah booth leads.
//!
//! The server decides a booth's destination, and nothing in the client's
//! game data says which, so the client learns it the way a player does: when
//! the server zones a character that stands at a booth, the playfield it
//! arrives in is that booth's destination. What is learned is kept in
//! `whompah.ini` beside the settings (one `playfield x z = destination` line
//! per booth) and the labels over the booths read it.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Booths that were checked against the live server: Old Athens' station
/// (playfield 540) leads to Tir (640).
const KNOWN: &[(u32, f32, f32, u32)] = &[(540, 444.2, 313.0, 640)];

/// A booth: its playfield and position in tenths of a unit.
type Key = (u32, i32, i32);

fn key(playfield: u32, x: f32, z: f32) -> Key {
    (playfield, (x * 10.0).round() as i32, (z * 10.0).round() as i32)
}

/// How near a position must be to a booth's to count as it (in tenths of a unit).
const NEAR: i32 = 10;

fn near(a: Key, b: Key) -> bool {
    a.0 == b.0 && (a.1 - b.1).abs() <= NEAR && (a.2 - b.2).abs() <= NEAR
}

pub struct Destinations {
    path: PathBuf,
    map: BTreeMap<Key, u32>,
}

impl Destinations {
    pub fn path() -> PathBuf {
        crate::settings::Settings::path().with_file_name("whompah.ini")
    }

    pub fn from_text(text: &str, path: PathBuf) -> Destinations {
        let mut map = BTreeMap::new();
        for &(pf, x, z, to) in KNOWN {
            map.insert(key(pf, x, z), to);
        }
        for line in text.lines() {
            let line = line.split(';').next().unwrap_or("").trim();
            let Some((from, to)) = line.split_once('=') else { continue };
            let from: Vec<&str> = from.split_whitespace().collect();
            if let ([pf, x, z], Ok(to)) = (&from[..], to.trim().parse::<u32>()) {
                if let (Ok(pf), Ok(x), Ok(z)) = (pf.parse::<u32>(), x.parse::<f32>(), z.parse::<f32>()) {
                    map.insert(key(pf, x, z), to);
                }
            }
        }
        Destinations { path, map }
    }

    pub fn load() -> Destinations {
        let path = Destinations::path();
        Destinations::from_text(&std::fs::read_to_string(&path).unwrap_or_default(), path)
    }

    pub fn to_text(&self) -> String {
        let mut out = String::from("; Where the whompah booths lead: playfield x z = destination playfield\n; (learned when the server zones a character standing at a booth)\n");
        for (&(pf, x, z), to) in &self.map {
            out += &format!("{pf} {:.1} {:.1} = {to}\n", x as f32 / 10.0, z as f32 / 10.0);
        }
        out
    }

    /// The playfield the booth at (x, z) in `playfield` leads to, if known.
    pub fn get(&self, playfield: u32, x: f32, z: f32) -> Option<u32> {
        let here = key(playfield, x, z);
        self.map.iter().find(|(booth, _)| near(**booth, here)).map(|(_, to)| *to)
    }

    /// Notes where a booth leads (saving it when that is new).
    pub fn learn(&mut self, playfield: u32, x: f32, z: f32, destination: u32) -> bool {
        if self.get(playfield, x, z) == Some(destination) {
            return false;
        }
        let here = key(playfield, x, z);
        self.map.retain(|booth, _| !near(*booth, here));
        self.map.insert(here, destination);
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&self.path, self.to_text()) {
            eprintln!("can't save {}: {e}", self.path.display());
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_booths_and_learning() {
        let path = std::env::temp_dir().join("mao-test-whompah.ini");
        let mut d = Destinations::from_text("", path.clone());
        // The verified booth is known from the start, found within a unit.
        assert_eq!(d.get(540, 444.2, 313.0), Some(640));
        assert_eq!(d.get(540, 444.7, 312.6), Some(640));
        assert_eq!(d.get(540, 449.0, 307.7), None);
        // Learning a booth saves it, and it reads back.
        assert!(d.learn(540, 449.0, 307.7, 545));
        assert!(!d.learn(540, 449.3, 307.9, 545), "already known");
        let again = Destinations::from_text(&d.to_text(), path.clone());
        assert_eq!(again.get(540, 449.0, 307.7), Some(545));
        assert_eq!(again.get(540, 444.2, 313.0), Some(640));
        // A booth that leads somewhere else now is corrected.
        assert!(d.learn(540, 449.0, 307.7, 595));
        assert_eq!(d.get(540, 449.0, 307.7), Some(595));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn text_lines_are_read() {
        let d = Destinations::from_text("; comment\n545 703.5 424.8 = 540 ; Old Athens\nnonsense\n", PathBuf::from("x"));
        assert_eq!(d.get(545, 703.5, 424.8), Some(540));
    }
}
