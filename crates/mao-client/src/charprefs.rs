//! Each character's own preference files, kept apart from the install's and the account's as the original keeps them.
//!
//! The original's `Prefs` folder holds three levels (seen in the players' folders and in GUI.dll):
//!
//! - the install's: `Prefs/Prefs.xml`, the values its `MainPrefs.xml` declares (display, chat font, sound on/off);
//! - the account's: `Prefs/<account>/Prefs.xml`, the values of `LoginPrefs.xml` (mouse, sounds, Escape closes,
//!   targeting by bounding boxes, target's target, auto-reject duels, ...), loaded by
//!   `ControlCenterModule_c::LoadUserConfig` (0x1006c3b1) with `DistributedValue_c::LoadConfig(path, 2)`;
//! - the character's: `Prefs/<account>/Char<id>/`, loaded by the same function with `LoadConfig(path, 3)` and
//!   `Preferences_t::LoadCharPrefs` (0x10124b86, `Char.cfg`): its `Prefs.xml` holds the values of `CharPrefs.xml`
//!   (`NumHotbars`, `LockHotbars`, `ShowFightTargetNCU`, `LockEquipment`, ...) and every window's `*Config` (its
//!   `WindowFrame`, selected tab, `listview_config`, pin), `DockAreas/` the docked windows, `RollupArea`, `Chat/Windows/`
//!   the chat windows, `Containers/ShortcutBar_<n>.xml` the shortcut bars.
//!
//! This client keeps its files beside `mao.ini`; the ones the original keeps per character are kept in
//! `chars/<character id>/` beside it once the character is known (`net::live`'s start, before the game's window draws
//! anything: the game process is started for one character). The files there are [`FILES`]. On a character's first
//! start each of its files is copied from the shared one beside `mao.ini` (the setup every character shared before
//! the split), so nobody starts from nothing (and `character.ini`, the settings of `mao.ini` the original keeps per
//! character, from `mao.ini`'s: `settings.rs`); from then on the character's own files are read and written, and the
//! shared ones are left as they are, the seed of the next new character. (The original starts a new character from
//! `CharPrefs.xml`'s defaults; the seeding is this client's, for the players who set up their windows here before.)
//!
//! Not per character (yet): `places.ini` keeps every character's item cells in one file, by character already;
//! the macros (the original's `TextMacro.bin`), the ignore list (`IgnoreList.bin`), the pet names and the pet window's
//! switches (`PetName1-6`, `AutoPetRenaming`, `PetWindowConfig`) are still kept once for the install.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The files kept per character, by their names beside `mao.ini`.
pub const FILES: &[&str] = &[
    // (the windows: places, sizes, tabs, lists, the windows open, the rollup area, the docks, the pins, the chat windows)
    "layout.ini",
    "sizes.ini",
    "tabs.ini",
    "lists.ini",
    "open.ini",
    "rollup.ini",
    "docks.ini",
    "pins.ini",
    "chatwindows.ini",
    // (the bars: the original's shortcut bars, this client's smart hotbars; their number and lock are in the
    // character's `character.ini`, `settings.rs`)
    "shortcutbars.ini",
    "hotbar.ini",
];

static CHARACTER: Mutex<Option<u32>> = Mutex::new(None);

/// Whether the player's files are left alone (the tests and the offline previews).
fn detached() -> bool {
    cfg!(test) || std::env::var_os("MAO_UI_PREVIEW").is_some()
}

/// The character playing, once known.
pub fn character() -> Option<u32> {
    *CHARACTER.lock().unwrap()
}

/// The character's folder beside a settings file: `chars/<id>`.
pub fn dir_in(settings: &Path, character: u32) -> PathBuf {
    settings.with_file_name("chars").join(character.to_string())
}

/// The character's folder beside `mao.ini`.
pub fn dir(character: u32) -> PathBuf {
    dir_in(&crate::settings::Settings::path(), character)
}

/// Where a preference file is: the character's own once one is known, else the shared one beside the settings.
pub fn file_in(settings: &Path, character: Option<u32>, name: &str) -> PathBuf {
    match character {
        Some(id) => dir_in(settings, id).join(name),
        None => settings.with_file_name(name),
    }
}

/// A per-character file ([`FILES`]) where it is read and written now.
pub fn file(name: &str) -> PathBuf {
    file_in(&crate::settings::Settings::path(), character(), name)
}

/// The character is known: its folder is made, and each of its files it does not have yet is copied from the shared
/// one. Returns the files copied.
pub fn set_character(id: u32) -> Vec<String> {
    let mut guard = CHARACTER.lock().unwrap();
    if *guard == Some(id) {
        return Vec::new();
    }
    *guard = Some(id);
    drop(guard);
    if detached() {
        return Vec::new();
    }
    let copied = seed_in(&crate::settings::Settings::path(), id, FILES);
    if !copied.is_empty() {
        eprintln!("character {id}: its own preferences in {} start from the shared {}", dir(id).display(), copied.join(", "));
    }
    copied
}

/// Makes the character's folder and copies each of `files` it lacks from the shared one beside `settings` (a shared
/// file that does not exist is not copied: the character's starts empty, as the shared one would have). Returns the
/// files copied.
pub fn seed_in(settings: &Path, character: u32, files: &[&str]) -> Vec<String> {
    let own = dir_in(settings, character);
    if let Err(e) = std::fs::create_dir_all(&own) {
        eprintln!("can't make {}: {e}", own.display());
        return Vec::new();
    }
    let mut copied = Vec::new();
    for name in files {
        let (shared, mine) = (settings.with_file_name(name), own.join(name));
        if !mine.exists() && shared.is_file() && std::fs::copy(&shared, &mine).is_ok() {
            copied.push(name.to_string());
        }
    }
    copied
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A character's files are its own folder's once it is known; before, the shared ones beside the settings.
    #[test]
    fn a_characters_files_are_in_its_own_folder() {
        let settings = Path::new("conf").join("mao.ini");
        assert_eq!(file_in(&settings, None, "layout.ini"), Path::new("conf").join("layout.ini"));
        assert_eq!(file_in(&settings, Some(1234), "layout.ini"), Path::new("conf").join("chars").join("1234").join("layout.ini"));
    }

    /// The first start copies the shared files the character lacks; a later start copies nothing, and two characters'
    /// files go their own ways from the same seed.
    #[test]
    fn the_first_start_seeds_from_the_shared_files() {
        let root = std::env::temp_dir().join(format!("mao-charprefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let settings = root.join("mao.ini");
        std::fs::write(root.join("layout.ini"), "Skills 10 20\n").unwrap();
        std::fs::write(root.join("pins.ini"), "Skills\n").unwrap();
        assert_eq!(seed_in(&settings, 7, &["layout.ini", "sizes.ini", "pins.ini"]), ["layout.ini", "pins.ini"]);
        assert_eq!(std::fs::read_to_string(dir_in(&settings, 7).join("layout.ini")).unwrap(), "Skills 10 20\n");
        assert!(!dir_in(&settings, 7).join("sizes.ini").exists());
        // (the character moves a window: its own file changes, the shared one and another character's do not)
        std::fs::write(dir_in(&settings, 7).join("layout.ini"), "Skills 300 40\n").unwrap();
        assert!(seed_in(&settings, 7, &["layout.ini", "pins.ini"]).is_empty());
        assert_eq!(std::fs::read_to_string(dir_in(&settings, 7).join("layout.ini")).unwrap(), "Skills 300 40\n");
        assert_eq!(seed_in(&settings, 8, &["layout.ini"]), ["layout.ini"]);
        assert_eq!(std::fs::read_to_string(dir_in(&settings, 8).join("layout.ini")).unwrap(), "Skills 10 20\n");
        assert_eq!(std::fs::read_to_string(root.join("layout.ini")).unwrap(), "Skills 10 20\n");
        let _ = std::fs::remove_dir_all(&root);
    }
}
