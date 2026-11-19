//! Where each whompah booth leads.
//!
//! The server decides a booth's destination. The client learns it the way a
//! player does: when the server zones a character that stands at a booth, the
//! playfield it arrives in is that booth's destination. What is learned is
//! kept in `whompah.ini` beside the settings (one `playfield x z =
//! destination` line per booth).
//!
//! The game's data says it too, on the booths' signs ([`signs`], since
//! 2026-11-16): a booth not yet taken is labelled by its sign. What was
//! learned from the server goes first.

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

/// The Jobe platform's playfield.
pub const JOBE: u32 = 4530;

/// Where a booth's sign says it leads: the playfield, and the station's own name where the sign names a place in
/// the playfield and not the playfield.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sign {
    pub playfield: u32,
    pub station: Option<&'static str>,
}

/// The signs: the part of the texture's name after `new_whompa_sign_` (or `..._signs_`), the playfield the booth
/// under it leads to, and the station's name where the texture names a town. The playfields are not in the names;
/// they follow from the game's own data, in which the network is the same both ways (the station a sign leads to
/// has a sign back: `the_signs_lead_both_ways`), together with the playfields' names and the one booth checked
/// against the server (Old Athen's `tir`). Three playfields have two stations each (Broken Shores and The Reck,
/// one of each side; The Longest Road, whose clan one is Bliss).
const SIGNS: &[(&str, u32, Option<&str>)] = &[
    ("20k", 630, Some("20K")),
    ("2ho", 635, Some("2HO")),
    ("4_holes", 760, None),
    ("avalon", 505, None),
    ("bliss", 795, Some("Bliss")),
    ("borealis", 800, None),
    ("broken_shores_clan", 665, None),
    ("broken_shores_omni", 665, None),
    ("central_artery_valley", 590, None),
    ("galway", 685, None),
    ("hope", 560, Some("Hope")),
    ("icc_hq", 655, Some("ICC")),
    ("jobe", JOBE, None),
    ("mutant_domain", 696, None),
    ("newland", 566, None),
    ("newland_desert", 565, None),
    ("old_athens", 540, None),
    ("perpetual_wastelands", 570, None),
    ("reck_clan", 750, None),
    ("reck_ot", 750, None),
    ("rome", 730, Some("Rome")),
    ("southern_artery_valley", 610, None),
    ("southern_fouls_hills", 615, None),
    ("stret_west_bank", 790, None),
    ("the_longest_road", 795, None),
    ("tir", 640, None),
    ("varmint_woods", 600, None),
    ("wailing_wastes", 551, None),
    ("wine", 605, Some("Wine")),
    ("omni_ent", 705, None),
    ("omnitrade", 710, None),
];

/// Where the sign of this texture leads, when the texture is a booth's sign.
pub fn sign_destination(texture: &str) -> Option<Sign> {
    let name = texture.to_lowercase();
    if sign_is_jobe(&name) {
        return Some(Sign { playfield: JOBE, station: None });
    }
    let name = name.strip_suffix(".png").unwrap_or(&name).trim_end_matches('.');
    let key = name.strip_prefix("new_whompa_signs_").or_else(|| name.strip_prefix("new_whompa_sign_"))?;
    SIGNS.iter().find(|s| s.0 == key).map(|&(_, playfield, station)| Sign { playfield, station })
}

/// The booths' signs of a playfield (x, height, z, and where each leads): every booth has a statel of
/// `sign_whompa_default` at its own place whose texture is named for the destination.
pub fn signs(game: &str, db: &mao_formats::resource::ResourceDatabase, playfield: u32) -> Vec<([f32; 3], Sign)> {
    let textures: std::collections::HashMap<u32, Sign> = db
        .get(mao_formats::resource::Id { kind: 0xf424a, instance: 1 })
        .ok()
        .flatten()
        .and_then(|o| mao_formats::instances::Instances::parse(&o).ok())
        .map(|index| index.iter().filter(|&(kind, _, _)| kind == 0xf6954).filter_map(|(_, name, instance)| Some((instance, sign_destination(name)?))).collect())
        .unwrap_or_default();
    let path = std::path::Path::new(game).join(format!("cd_image/data/statels/{playfield}.pf"));
    let Ok(file) = mao_formats::files::read(&path) else { return Vec::new() };
    let Ok(statels) = mao_formats::statels::parse_as(&file, crate::statel_layout(db, playfield)) else { return Vec::new() };
    statels.iter().filter_map(|s| Some((s.position, s.extras.iter().find_map(|e| textures.get(&e.2).copied())?))).collect()
}

/// The sign of the booth at (x, z): the one within two units.
pub fn sign_at(signs: &[([f32; 3], Sign)], x: f32, z: f32) -> Option<Sign> {
    signs.iter().find(|s| (s.0[0] - x).hypot(s.0[2] - z) < 2.0).map(|s| s.1)
}

/// Whether a texture's name is the sign of a booth to Jobe. Every booth has a sign over it (a statel of
/// `sign_whompa_default` at the booth's place) with the texture of where it leads: `new_whompa_sign_tir`,
/// `new_whompa_sign_newland`, and `new_whompa_sign_jobe` for the Jobe platform (the older stations have
/// `teleportersigns_omni-whompa_jobe`). The signs inside Jobe, of the ways back, are `whompa_signs_from_jobe`.
pub fn sign_is_jobe(texture: &str) -> bool {
    let name = texture.to_lowercase();
    name.contains("whompa") && name.contains("jobe") && !name.contains("from_jobe")
}


#[cfg(test)]
mod tests {
    use super::*;

    /// The Jobe booth is known by its sign.
    #[test]
    fn the_jobe_booth_is_known_by_its_sign() {
        assert!(sign_is_jobe("new_whompa_sign_jobe.png") && sign_is_jobe("teleportersigns_omni-whompa_jobe.png"));
        assert!(!sign_is_jobe("new_whompa_sign_tir.png") && !sign_is_jobe("whompa_signs_from_jobe.png") && !sign_is_jobe("jobe_building_whompa.abiff.png_from_jobe"));
    }

    /// `MAO_GAME`: Old Athen's fourth booth and ICC's eastern one are the ones to Jobe, and Tir has none.
    #[test]
    fn the_games_jobe_booths() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let game = game.to_string_lossy().to_string();
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let places = |pf: u32| signs(&game, &db, pf).iter().filter(|s| s.1.playfield == JOBE).map(|s| ((s.0[0] * 10.0).round() as i32, (s.0[2] * 10.0).round() as i32)).collect::<Vec<_>>();
        assert_eq!(places(540), [(4490, 3177)]);
        assert_eq!(places(655), [(31831, 8660)]);
        assert_eq!(places(640), []);
    }

    /// The signs' names are read whatever their case and their ending, and a texture that is no sign is none.
    #[test]
    fn a_signs_name_is_its_destination() {
        assert_eq!(sign_destination("new_whompa_sign_tir.png"), Some(Sign { playfield: 640, station: None }));
        assert_eq!(sign_destination("new_whompa_sign_20K.png"), Some(Sign { playfield: 630, station: Some("20K") }));
        assert_eq!(sign_destination("new_whompa_signs_omnitrade.png"), Some(Sign { playfield: 710, station: None }));
        assert_eq!(sign_destination("new_whompa_sign_perpetual_wastelands..png"), Some(Sign { playfield: 570, station: None }));
        assert_eq!(sign_destination("teleportersigns_omni-whompa_jobe.png"), Some(Sign { playfield: JOBE, station: None }));
        assert_eq!((sign_destination("whompa_new_omni.png"), sign_destination("whompa_signs_from_jobe.png"), sign_destination("new_whompa_sign_nowhere.png")), (None, None, None));
        let signs = [([449.0, 8.0, 317.7], Sign { playfield: JOBE, station: None }), ([444.2, 8.0, 313.0], Sign { playfield: 640, station: None })];
        assert_eq!((sign_at(&signs, 444.9, 312.6).map(|s| s.playfield), sign_at(&signs, 454.0, 313.0)), (Some(640), None));
    }

    /// `MAO_GAME`: every sign the game has placed is known, every booth of a station has its sign, the booth
    /// checked against the server reads as it was found, and the network is the same both ways: where a sign leads,
    /// a sign leads back (but for the Jobe platform, whose signs are of another kind).
    #[test]
    fn the_signs_lead_both_ways() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let game = game.to_string_lossy().to_string();
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let index = db.get(mao_formats::resource::Id { kind: 0xf424a, instance: 1 }).unwrap().and_then(|o| mao_formats::instances::Instances::parse(&o).ok()).unwrap();
        // (the one sign texture that names no place is the blank one the sign's mesh comes with)
        let unknown: Vec<&str> = index.iter().filter(|&(kind, name, _)| kind == 0xf6954 && name.to_lowercase().starts_with("new_whompa_sign") && sign_destination(name).is_none()).map(|(_, name, _)| name).collect();
        assert_eq!(unknown, ["new_whompa_signs_default.png"]);
        let stations: Vec<u32> = SIGNS.iter().map(|s| s.1).filter(|&p| p != JOBE).collect();
        let all: std::collections::HashMap<u32, Vec<([f32; 3], Sign)>> = stations.iter().map(|&p| (p, signs(&game, &db, p))).collect();
        assert_eq!(sign_at(&all[&540], 444.2, 313.0), Some(Sign { playfield: 640, station: None }));
        let mut counted = 0;
        for (&from, list) in &all {
            assert!(!list.is_empty(), "no signs in {from}");
            for (at, sign) in list {
                counted += 1;
                assert_ne!(sign.playfield, from, "a sign of {from} at {at:?} leads to its own playfield");
                if sign.playfield != JOBE {
                    assert!(all[&sign.playfield].iter().any(|back| back.1.playfield == from), "{from} leads to {} at {at:?}, and nothing leads back", sign.playfield);
                }
            }
        }
        // (77 booths with a sign; one of them has its sign's texture twice)
        assert_eq!(counted, 77);
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
