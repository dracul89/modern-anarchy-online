//! The keys a character rebound in the original client, read from its preference file and laid over this client's.
//!
//! The original keeps the rebindable keys (movement, actions, windows, the shortcut bar) per character, as the
//! `KeyBindings` archive of `<Prefs>/<user name>/Char<id>/Prefs.xml` (`ControlCenterModule_c::LoadUserConfig`
//! 0x1006c3b1 loads the file, GUI.dll FUN_10018e37 the archive): a `Version` and an array `Bind` of archives, each
//! an `Input` and a `Provider`. The shipped default is the same archive in the game's `gui/Default/CharPrefs.xml`
//! (version 7, 99 binds).
//!
//! * **Input** is the game's own key number (`InputConfig_t::m_asInputMethodIDTable`; the program makes it from the
//!   Windows virtual key) with 0x20000 for Shift, 0x40000 for Ctrl and 0x80000 for Alt: [`binding`].
//! * **Provider** is the action, by the hash of its name (FUN_10019281: [`provider`]). A key may hold several
//!   actions and an action several keys.
//! * **Loading** (FUN_10018e37): the character's archive stands for all its keys, not only the changed ones (the
//!   one archive `DistributedValue_c::LoadConfig` does not merge into the default); an action no bind names has no
//!   key. Only when the shipped version is newer than the file's are the shipped binds added whose action the file
//!   never names: [`merged`]. A file without the archive, or no file, is the shipped default.
//!
//! Here: at the game's start the character's file is read ([`apply`]) and, for each of this client's actions that
//! has a counterpart ([`PROVIDERS`]), the keys are taken over **when they differ from the shipped default's**: a
//! character whose keys were never changed in the original changes nothing here. (The rule is this client's: its
//! own defaults are not the shipped file's in every key (the mouse's, the client's own windows), and a key the
//! player set here must not be moved by a file the original last wrote long ago.)
//! They lie over the client's own keys for the session and are not written to `mao.ini`
//! (`settings::Keymap::set_character`); an action whose key was changed in this client keeps that key, and nothing
//! is ever written to the original's folder.
//!
//! Not read: the fixed keys (the camera's, Tab's, the chat's: `KEY_...` lines of the account's `Login.cfg`, another
//! mechanism); mouse buttons as keys (the shipped default has the middle button walk forward; this client's keys
//! are keyboard keys); the actions this client has none for (`CAMERA_NEXTVIEW`, the team and pet selection keys,
//! `WINDOW_TEAM` and the like). The original's numbers are virtual keys, this client's are places on the keyboard:
//! on a keyboard that is not QWERTY a letter lands on the letter's QWERTY place.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::gui::xml::Element;
use crate::settings::{Action, Binding, Keymap};

/// An action's number in a bind: the hash of its provider's name (GUI.dll FUN_10019281:
/// `h ^= (h << 6) + (h >> 2) + byte + 0x9e3779b9` over the name's bytes, each as a signed char).
pub fn provider(name: &str) -> u32 {
    name.bytes().fold(0u32, |h, c| h ^ (h << 6).wrapping_add(h >> 2).wrapping_add(c as i8 as i32 as u32).wrapping_add(0x9e37_79b9))
}

/// The original's providers and this client's actions (two of the original's go to one here where the client has
/// one action for both: the two sets of movement keys, the two maps). The
/// shortcut bar's name is the original's spelling.
pub const PROVIDERS: &[(&str, Action)] = &[
    ("MOVEMENT_FORWARD_V3", Action::Forward),
    ("MOVEMENT_FORWARD_GLOBAL_V2", Action::Forward),
    ("MOVEMENT_BACK_V2", Action::Back),
    ("MOVEMENT_BACK_GLOBAL_V2", Action::Back),
    ("MOVEMENT_LEFT_V2", Action::TurnLeft),
    ("MOVEMENT_LEFT_GLOBAL_V2", Action::TurnLeft),
    ("MOVEMENT_RIGHT_V2", Action::TurnRight),
    ("MOVEMENT_RIGHT_GLOBAL_V2", Action::TurnRight),
    ("MOVEMENT_STRAFELEFT", Action::StrafeLeft),
    ("MOVEMENT_STRAFERIGHT", Action::StrafeRight),
    ("MOVEMENT_JUMP", Action::Jump),
    ("MOVEMENT_AUTORUN", Action::AutoRun),
    ("MOVEMENT_TOGGLEWALK", Action::ToggleWalk),
    ("CAMERA_TOGGLE3RD", Action::CameraToggleView),
    ("CAMERA_SCREENSHOT", Action::Screenshot),
    ("ACTION_ATTACK", Action::Attack),
    ("ACTION_USE", Action::Use),
    ("ACTION_SIT", Action::SitStand),
    ("ACTION_LOOKAT", Action::TargetInfo),
    ("ACTION_BRAWL", Action::Quick(0)),
    ("ACTION_BURST", Action::Quick(1)),
    ("ACTION_DIMACH", Action::Quick(2)),
    ("ACTION_FLINGSHOT", Action::Quick(3)),
    ("ACTION_FULLAUTO", Action::Quick(4)),
    ("ACTION_RELOAD", Action::Quick(5)),
    ("ACTION_PICKUPITEM", Action::Quick(6)),
    ("ACTION_SNEAK", Action::Quick(7)),
    ("ACTION_SNEAKATTACK", Action::Quick(8)),
    ("ACTION_FASTATTACK", Action::Quick(9)),
    ("ACTION_AIMEDSHOT", Action::Quick(10)),
    ("ACTION_BOWSPECIALATTACK", Action::Quick(11)),
    ("TARGET_SELECTSELF", Action::SelectSelf),
    ("WINDOW_INVENTORY", Action::InventoryWindow),
    ("WINDOW_WEAR", Action::WearWindow),
    ("WINDOW_SKILLS", Action::SkillsWindow),
    ("WINDOW_PLANETMAP", Action::PlanetMapWindow),
    ("WINDOW_MAP", Action::MapWindow),
    ("WINDOW_OPTIONS", Action::ToggleOptions),
    ("WINDOW_SPECIALACTION", Action::ActionsWindow),
    ("WINDOW_MISSION", Action::MissionWindow),
    ("WINDOW_FRIENDS", Action::FriendsWindow),
    ("WINDOW_NANO", Action::ProgramsWindow),
    ("WINDOW_STAT", Action::StatsWindow),
    ("WINDOW_NCU", Action::NcuWindow),
    ("WINDOW_TS", Action::TradeskillWindow),
    ("WINDOW_TEAM", Action::TeamWindow),
    ("WINDOW_RAID", Action::RaidWindow),
    ("WINDOW_LFT", Action::LftWindow),
    ("WINDOW_RESEARCH", Action::ResearchWindow),
    ("WINDOW_KNOWLEDGE", Action::KnowledgeWindow),
    ("WINDOW_DL", Action::DailyLoginWindow),
    ("WINDOW_ITEMSHOP", Action::ItemShopWindow),
    ("SHORTUCT_BAR_ACTIVE_0", Action::HotbarSlot(0)),
    ("SHORTUCT_BAR_ACTIVE_1", Action::HotbarSlot(1)),
    ("SHORTUCT_BAR_ACTIVE_2", Action::HotbarSlot(2)),
    ("SHORTUCT_BAR_ACTIVE_3", Action::HotbarSlot(3)),
    ("SHORTUCT_BAR_ACTIVE_4", Action::HotbarSlot(4)),
    ("SHORTUCT_BAR_ACTIVE_5", Action::HotbarSlot(5)),
    ("SHORTUCT_BAR_ACTIVE_6", Action::HotbarSlot(6)),
    ("SHORTUCT_BAR_ACTIVE_7", Action::HotbarSlot(7)),
    ("SHORTUCT_BAR_ACTIVE_8", Action::HotbarSlot(8)),
    ("SHORTUCT_BAR_ACTIVE_9", Action::HotbarSlot(9)),
    ("SHORTUCT_BAR_SELECT_0", Action::HotbarBar(0)),
    ("SHORTUCT_BAR_SELECT_1", Action::HotbarBar(1)),
    ("SHORTUCT_BAR_SELECT_2", Action::HotbarBar(2)),
    ("SHORTUCT_BAR_SELECT_3", Action::HotbarBar(3)),
    ("SHORTUCT_BAR_SELECT_4", Action::HotbarBar(4)),
    ("SHORTUCT_BAR_SELECT_5", Action::HotbarBar(5)),
    ("SHORTUCT_BAR_SELECT_6", Action::HotbarBar(6)),
    ("SHORTUCT_BAR_SELECT_7", Action::HotbarBar(7)),
    ("SHORTUCT_BAR_SELECT_8", Action::HotbarBar(8)),
    ("SHORTUCT_BAR_SELECT_9", Action::HotbarBar(9)),
    ("SHORTUCT_BAR_ROW_0", Action::HotbarRow(0)),
    ("SHORTUCT_BAR_ROW_1", Action::HotbarRow(1)),
    ("SHORTUCT_BAR_ROW_2", Action::HotbarRow(2)),
    ("SHORTUCT_BAR_ROW_3", Action::HotbarRow(3)),
    ("SHORTUCT_BAR_ROW_4", Action::HotbarRow(4)),
    ("SHORTUCT_BAR_ROW_5", Action::HotbarRow(5)),
    ("SHORTUCT_BAR_ROW_6", Action::HotbarRow(6)),
    ("SHORTUCT_BAR_ROW_7", Action::HotbarRow(7)),
    ("SHORTUCT_BAR_ROW_8", Action::HotbarRow(8)),
    ("SHORTUCT_BAR_ROW_9", Action::HotbarRow(9)),
];

/// A `KeyBindings` archive: its version and its binds, each a key (`Input`) and an action (`Provider`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bindings {
    pub version: i64,
    pub binds: Vec<(u32, u32)>,
}

fn named<'a>(e: &'a Element, kind: &str, name: &str) -> impl Iterator<Item = &'a Element> {
    let (kind, name) = (kind.to_string(), name.to_string());
    e.children.iter().filter(move |c| c.name == kind && c.attribute("name") == Some(name.as_str()))
}

fn number(e: &Element) -> Option<u64> {
    e.attribute("value")?.trim().parse::<i64>().ok().map(|v| v as u64)
}

/// The `KeyBindings` archive of a preferences file (a character's `Prefs.xml`, the game's `CharPrefs.xml`); nothing
/// when the file has none. A bind's every `Provider` is taken, beside its `Input` or in an array of that name (no
/// file seen has two; the loader reads as many as there are).
pub fn parse(text: &str) -> Option<Bindings> {
    fn find(elements: &[Element]) -> Option<&Element> {
        elements.iter().find_map(|e| if e.name == "Archive" && e.attribute("name") == Some("KeyBindings") { Some(e) } else { find(&e.children) })
    }
    fn providers(e: &Element, out: &mut Vec<u32>) {
        for c in &e.children {
            if c.name == "Int64" && c.attribute("name").is_none_or(|n| n == "Provider") {
                out.extend(number(c).map(|v| v as u32));
            } else if c.name == "Array" && c.attribute("name") == Some("Provider") {
                providers(c, out);
            }
        }
    }
    let roots = crate::gui::xml::parse(text);
    let archive = find(&roots)?;
    let version = named(archive, "Int32", "Version").next().and_then(number).map_or(0, |v| v as i64);
    let mut binds = Vec::new();
    for bind in named(archive, "Array", "Bind").flat_map(|array| array.children.iter()) {
        let Some(input) = named(bind, "Int32", "Input").next().and_then(number) else { continue };
        let mut actions = Vec::new();
        providers(bind, &mut actions);
        binds.extend(actions.into_iter().map(|action| (input as u32, action)));
    }
    Some(Bindings { version, binds })
}

/// The binds in force for a character, as the loader makes them (FUN_10018e37): the character's own, and, only when
/// the shipped version is the newer, the shipped binds of the actions its file never names. Without an archive of
/// its own the shipped ones.
pub fn merged(own: Option<&Bindings>, shipped: &Bindings) -> Vec<(u32, u32)> {
    let Some(own) = own else { return shipped.binds.clone() };
    let mut binds = own.binds.clone();
    if shipped.version > own.version {
        let seen: BTreeSet<u32> = own.binds.iter().map(|b| b.1).collect();
        binds.extend(shipped.binds.iter().filter(|b| !seen.contains(&b.1)));
    }
    binds
}

/// The name this client has for one of the original's key numbers (`settings::KEY_NAMES`): the keyboard's keys.
/// None for the mouse's inputs (1 to 13, 121 to 126), the modifier keys by themselves, the lock keys, and what is
/// no key at all.
pub fn key_name(key: u32) -> Option<String> {
    Some(match key {
        14 => "Escape".into(),
        15 => "Tab".into(),
        23 => "Space".into(),
        24 => "Enter".into(),
        25 => "Backspace".into(),
        26 => "Insert".into(),
        27 => "Delete".into(),
        28 => "Home".into(),
        29 => "End".into(),
        30 => "PageUp".into(),
        31 => "PageDown".into(),
        32 => "Left".into(),
        33 => "Right".into(),
        34 => "Up".into(),
        35 => "Down".into(),
        // (the original's PIPE, the key right of the brackets on a US keyboard)
        36 => "Backslash".into(),
        37 => "Period".into(),
        38 => "Comma".into(),
        39..=50 => format!("F{}", key - 38),
        66 => "NumpadDivide".into(),
        67 => "NumpadMultiply".into(),
        68 => "NumpadSubtract".into(),
        69 => "NumpadAdd".into(),
        70..=79 => format!("Numpad{}", key - 70),
        82..=107 => ((b'A' + (key - 82) as u8) as char).to_string(),
        108..=116 => (key - 107).to_string(),
        117 => "0".into(),
        118 => "Slash".into(),
        _ => return None,
    })
}

/// A bind's `Input` as a key of this client's: the key number in the low 17 bits, Shift 0x20000, Ctrl 0x40000, Alt
/// 0x80000 (anarchyonline.exe FUN_00404a66 puts them on; 0x100000, a release, is never in a file).
pub fn binding(input: u32) -> Option<Binding> {
    let mut binding = crate::settings::parse_binding(&key_name(input & 0x1ffff)?)?;
    binding.shift = input & 0x20000 != 0;
    binding.ctrl = input & 0x40000 != 0;
    binding.alt = input & 0x80000 != 0;
    Some(binding)
}

/// The actions of this client whose keys the character has changed in the original, each with its keys now: those
/// whose keys in force ([`merged`]) are not the shipped default's. An action with no key in force is given none (it
/// was unbound there); one whose only keys are ones this client cannot express (a mouse button) is left out.
pub fn changed(own: Option<&Bindings>, shipped: &Bindings) -> Vec<(Action, Vec<Binding>)> {
    let now = merged(own, shipped);
    let inputs = |binds: &[(u32, u32)], hashes: &[u32]| -> Vec<u32> {
        let mut inputs: Vec<u32> = binds.iter().filter(|b| hashes.contains(&b.1)).map(|b| b.0).collect();
        inputs.sort_unstable();
        inputs.dedup();
        inputs
    };
    let mut out: Vec<(Action, Vec<Binding>)> = Vec::new();
    for &(_, action) in PROVIDERS {
        if out.iter().any(|(a, _)| *a == action) {
            continue;
        }
        let hashes: Vec<u32> = PROVIDERS.iter().filter(|p| p.1 == action).map(|p| provider(p.0)).collect();
        let (mine, theirs) = (inputs(&now, &hashes), inputs(&shipped.binds, &hashes));
        if mine == theirs {
            // (kept out of `out` altogether, the check above looks at what is in it)
            continue;
        }
        let keys: Vec<Binding> = mine.iter().filter_map(|&input| binding(input)).collect();
        if keys.is_empty() && !mine.is_empty() {
            continue;
        }
        out.push((action, keys));
    }
    out
}

fn read(path: &Path) -> Option<String> {
    std::fs::read(path).ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// Lays the keys the character rebound in the original over the keymap (see the module's head): the game's shipped
/// `CharPrefs.xml` for what the default is, the character's `Prefs.xml` (`aoprefs::character_prefs`) for its own.
/// Nothing without either. What was taken over is said in the log.
pub fn apply(keymap: &mut Keymap, character: u32, game: &Path) {
    let Some(shipped) = read(&game.join("cd_image/gui/Default/CharPrefs.xml")).and_then(|text| parse(&text)) else { return };
    let Some(file) = crate::aoprefs::character_prefs(character) else { return };
    let own = read(&file).and_then(|text| parse(&text));
    let keys: HashMap<Action, Vec<Binding>> = changed(own.as_ref(), &shipped).into_iter().collect();
    if keys.is_empty() {
        return;
    }
    let taken = keymap.set_character(keys);
    let name = |action: Action| crate::settings::ACTIONS.iter().find(|a| a.0 == action).map_or("?", |a| a.1);
    eprintln!(
        "keys: the original's own bindings of character {character} ({}) taken over for this session: {}",
        file.display(),
        if taken.is_empty() { "none (each changed here too)".to_string() } else { taken.iter().map(|a| format!("{} = {}", name(*a), keymap.keys(*a))).collect::<Vec<_>>().join("; ") }
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::parse_binding;

    fn archive(version: i64, binds: &[(u32, &str)]) -> String {
        let mut text = format!("<Root>\n    <Value name=\"x\" value=\"1\" />\n    <Archive name=\"KeyBindings\" code=\"0\">\n        <Int32 name=\"Version\" value=\"{version}\" />\n        <Array name=\"Bind\">\n");
        for (input, name) in binds {
            text += &format!("            <Archive code=\"0\">\n                <Int32 name=\"Input\" value=\"{input}\" />\n                <Int64 name=\"Provider\" value=\"{}\" />\n            </Archive>\n", provider(name));
        }
        text + "        </Array>\n    </Archive>\n</Root>\n"
    }

    const SHIPPED: &[(u32, &str)] = &[(9, "MOVEMENT_FORWARD_V3"), (104, "MOVEMENT_FORWARD_V3"), (34, "MOVEMENT_FORWARD_GLOBAL_V2"), (98, "ACTION_ATTACK"), (86, "ACTION_USE"), (131173, "WINDOW_TS"), (25, "MOVEMENT_TOGGLEWALK"), (262190, "CAMERA_NEXTVIEW")];

    /// The hash is the original's (the numbers are the shipped file's), and every provider has its own.
    #[test]
    fn a_provider_is_the_hash_of_its_name() {
        assert_eq!(provider("MOVEMENT_FORWARD_V3"), 587315832);
        assert_eq!(provider("SHORTUCT_BAR_ACTIVE_0"), 3551010340);
        assert_eq!(provider("WINDOW_TS"), 2073189714);
        assert_eq!(provider("ACTION_BURST"), 2376032);
        let hashes: BTreeSet<u32> = PROVIDERS.iter().map(|p| provider(p.0)).collect();
        assert_eq!(hashes.len(), PROVIDERS.len());
    }

    /// A bind's input is a key number and the modifiers' bits; every keyboard key of the table has a name this
    /// client knows; the mouse's inputs are no keys here.
    #[test]
    fn an_input_is_a_key_and_its_modifiers() {
        assert_eq!(binding(104), parse_binding("W"));
        assert_eq!(binding(131173), parse_binding("Shift+T"));
        assert_eq!(binding(262252), parse_binding("Ctrl+1"));
        assert_eq!(binding(524405), parse_binding("Alt+0"));
        assert_eq!(binding(0x60000 | 99), parse_binding("Ctrl+Shift+R"));
        assert_eq!((binding(46), binding(70), binding(38), binding(23)), (parse_binding("F8"), parse_binding("Numpad0"), parse_binding("Comma"), parse_binding("Space")));
        assert_eq!((binding(9), binding(17), binding(121), binding(0)), (None, None, None, None));
        for key in (14..=15).chain(23..=50).chain(66..=79).chain(82..=118) {
            assert!(binding(key).is_some(), "key {key}");
        }
    }

    /// The archive is read wherever it is in the file; a bind's every provider counts; a file without it has none.
    #[test]
    fn the_archive_is_read() {
        let read = parse(&archive(7, SHIPPED)).unwrap();
        assert_eq!((read.version, read.binds.len()), (7, SHIPPED.len()));
        assert_eq!(read.binds[1], (104, provider("MOVEMENT_FORWARD_V3")));
        assert_eq!(parse("<Root><Archive name=\"SkillConfig\" code=\"0\"><Int32 name=\"Version\" value=\"3\" /></Archive></Root>"), None);
        let two = "<Root><Archive name=\"KeyBindings\" code=\"0\"><Int32 name=\"Version\" value=\"7\" /><Array name=\"Bind\"><Archive code=\"0\"><Int32 name=\"Input\" value=\"98\" /><Array name=\"Provider\"><Int64 value=\"11\" /><Int64 value=\"12\" /></Array></Archive><Archive code=\"0\"><Int64 name=\"Provider\" value=\"13\" /></Archive></Array></Archive></Root>";
        assert_eq!(parse(two).unwrap().binds, [(98, 11), (98, 12)]);
    }

    /// A file of an older version gains the shipped binds of the actions it does not name, and only those; one of
    /// the same version gains nothing (an action without a bind stays without a key); no archive is the default.
    #[test]
    fn an_older_file_gains_the_new_defaults() {
        let shipped = parse(&archive(7, SHIPPED)).unwrap();
        let without_ts: Vec<(u32, &str)> = SHIPPED.iter().copied().filter(|b| b.1 != "WINDOW_TS").collect();
        let old = parse(&archive(5, &without_ts)).unwrap();
        assert!(merged(Some(&old), &shipped).contains(&(131173, provider("WINDOW_TS"))));
        assert!(changed(Some(&old), &shipped).is_empty());
        let same = parse(&archive(7, &without_ts)).unwrap();
        assert!(!merged(Some(&same), &shipped).iter().any(|b| b.1 == provider("WINDOW_TS")));
        assert_eq!(changed(Some(&same), &shipped), [(Action::TradeskillWindow, Vec::new())]);
        assert_eq!(merged(None, &shipped), shipped.binds);
        assert!(changed(None, &shipped).is_empty() && changed(Some(&shipped), &shipped).is_empty());
    }

    /// Only the actions whose keys differ from the shipped default's are taken over: attack moved to F and use given
    /// a second key; the forward keys, the same but for the order, are not. An action with only a mouse button
    /// left is left alone, and one this client has no action for is passed over.
    #[test]
    fn only_changed_keys_are_taken_over() {
        let shipped = parse(&archive(7, SHIPPED)).unwrap();
        let own = parse(&archive(7, &[(34, "MOVEMENT_FORWARD_GLOBAL_V2"), (104, "MOVEMENT_FORWARD_V3"), (9, "MOVEMENT_FORWARD_V3"), (87, "ACTION_ATTACK"), (86, "ACTION_USE"), (0x40000 | 86, "ACTION_USE"), (131173, "WINDOW_TS"), (9, "MOVEMENT_TOGGLEWALK"), (98, "CAMERA_NEXTVIEW")])).unwrap();
        assert_eq!(changed(Some(&own), &shipped), [(Action::Attack, vec![parse_binding("F").unwrap()]), (Action::Use, vec![parse_binding("E").unwrap(), parse_binding("Ctrl+E").unwrap()])]);
    }

    /// `MAO_GAME`: the shipped file is version 7 with 99 binds, and every provider of the table is in it with the
    /// key the notes give (attack on Q, the tradeskill window on Shift+T, walking on Backspace).
    #[test]
    fn the_shipped_defaults_are_read() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let shipped = parse(&read(&Path::new(&game).join("cd_image/gui/Default/CharPrefs.xml")).unwrap()).unwrap();
        assert_eq!((shipped.version, shipped.binds.len()), (7, 99));
        // (the daily login's and the item shop's windows have no key in it)
        for (name, _) in PROVIDERS.iter().filter(|p| !matches!(p.0, "WINDOW_DL" | "WINDOW_ITEMSHOP")) {
            assert!(shipped.binds.iter().any(|b| b.1 == provider(name)), "{name}");
        }
        let key = |name: &str| shipped.binds.iter().filter(|b| b.1 == provider(name)).filter_map(|b| binding(b.0)).collect::<Vec<_>>();
        assert_eq!(key("ACTION_ATTACK"), [parse_binding("Q").unwrap()]);
        assert_eq!(key("WINDOW_TS"), [parse_binding("Shift+T").unwrap()]);
        assert_eq!(key("MOVEMENT_TOGGLEWALK"), [parse_binding("Backspace").unwrap()]);
        assert_eq!(key("MOVEMENT_FORWARD_V3"), [parse_binding("W").unwrap()]);
        assert_eq!((key("WINDOW_TEAM"), key("WINDOW_RAID"), key("WINDOW_KNOWLEDGE")), (vec![parse_binding("Ctrl+5").unwrap()], vec![parse_binding("Ctrl+Shift+R").unwrap()], vec![parse_binding("Ctrl+3").unwrap()]));
        assert!(changed(None, &shipped).is_empty());
    }
}
