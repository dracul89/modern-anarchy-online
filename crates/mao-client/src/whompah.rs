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

/// Whether a texture's name is the sign of a booth to Jobe. Every booth has a sign over it (a statel of
/// `sign_whompa_default` at the booth's place) with the texture of where it leads: `new_whompa_sign_tir`,
/// `new_whompa_sign_newland`, and `new_whompa_sign_jobe` for the Jobe platform (the older stations have
/// `teleportersigns_omni-whompa_jobe`). The signs inside Jobe, of the ways back, are `whompa_signs_from_jobe`.
pub fn sign_is_jobe(texture: &str) -> bool {
    let name = texture.to_lowercase();
    name.contains("whompa") && name.contains("jobe") && !name.contains("from_jobe")
}

/// The booths of a playfield that lead to Jobe (x, height, z): the statels whose texture is a Jobe sign's
/// ([`sign_is_jobe`]). The server alone decides where a booth leads; the sign is what the game's data says of it.
pub fn jobe_booths(game: &str, db: &mao_formats::resource::ResourceDatabase, playfield: u32) -> Vec<[f32; 3]> {
    let signs: std::collections::HashSet<u32> = db
        .get(mao_formats::resource::Id { kind: 0xf424a, instance: 1 })
        .ok()
        .flatten()
        .and_then(|o| mao_formats::instances::Instances::parse(&o).ok())
        .map(|index| index.iter().filter(|&(kind, name, _)| kind == 0xf6954 && sign_is_jobe(name)).map(|(_, _, instance)| instance).collect())
        .unwrap_or_default();
    let path = std::path::Path::new(game).join(format!("cd_image/data/statels/{playfield}.pf"));
    let Ok(file) = mao_formats::files::read(&path) else { return Vec::new() };
    let Ok(statels) = mao_formats::statels::parse(&file) else { return Vec::new() };
    statels.iter().filter(|s| s.extras.iter().any(|e| signs.contains(&e.2))).map(|s| s.position).collect()
}

/// Whether a character at `position` (x, height, z), touching a booth, is at one to Jobe that it cannot take: its
/// account's expansions (stat 389, none while the stats are not known) lack Shadowlands, bit 1 (the original's text
/// for a condition on that bit is `Feedback_YouMustHaveExpansionpack1`, Gamecode FUN_1000556a: bit 0 the Notum Wars
/// booster, 1 Shadowlands, 3 Alien Invasion, 5 Lost Eden, 7 Legacy of the Xan). The server says nothing to such a
/// character in the booth (the user's log of 2026-11-15: four tries, no answer), so the client says it.
pub fn needs_shadowlands(jobe_booths: &[[f32; 3]], position: [f32; 3], expansions: Option<i64>) -> bool {
    expansions.is_some_and(|e| e & 2 == 0) && jobe_booths.iter().any(|b| (b[0] - position[0]).hypot(b[2] - position[2]) < 2.0 && (b[1] - position[1]).abs() < 4.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Jobe booth is known by its sign, and an account without Shadowlands touching it is told; one with
    /// Shadowlands, one whose stats are not known yet, and anyone at another booth are not.
    #[test]
    fn the_jobe_booth_asks_for_shadowlands() {
        assert!(sign_is_jobe("new_whompa_sign_jobe.png") && sign_is_jobe("teleportersigns_omni-whompa_jobe.png"));
        assert!(!sign_is_jobe("new_whompa_sign_tir.png") && !sign_is_jobe("whompa_signs_from_jobe.png") && !sign_is_jobe("jobe_building_whompa.abiff.png_from_jobe"));
        let booths = [[449.0, 8.0, 317.7]];
        let at = [448.9, 8.3, 318.2];
        assert!(needs_shadowlands(&booths, at, Some(0)));
        assert!(needs_shadowlands(&booths, at, Some(1)));
        assert!(!needs_shadowlands(&booths, at, Some(2)));
        assert!(!needs_shadowlands(&booths, at, Some(0xab)));
        assert!(!needs_shadowlands(&booths, at, None));
        // (the booth to Tir, five units off; and a door a floor up)
        assert!(!needs_shadowlands(&booths, [444.2, 8.3, 313.0], Some(0)));
        assert!(!needs_shadowlands(&booths, [448.9, 14.0, 318.2], Some(0)));
    }

    /// `MAO_GAME`: Old Athen's fourth booth and ICC's eastern one are the ones to Jobe, and Tir has none.
    #[test]
    fn the_games_jobe_booths() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let game = game.to_string_lossy().to_string();
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let places = |pf: u32| jobe_booths(&game, &db, pf).iter().map(|b| ((b[0] * 10.0).round() as i32, (b[2] * 10.0).round() as i32)).collect::<Vec<_>>();
        assert_eq!(places(540), [(4490, 3177)]);
        assert_eq!(places(655), [(31831, 8660)]);
        assert_eq!(places(640), []);
    }

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
