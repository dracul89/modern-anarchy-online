//! Where the player left the windows: each movable window's position is saved in `layout.ini` beside `mao.ini` (one line per
//! window, `name x y`) and used as the window's starting place the next time. `MAO_NO_LAYOUT=1` ignores it; the offline previews
//! (`MAO_UI_PREVIEW`) ignore it too so their pictures do not depend on the player's layout.
//!
//! Every window shown through `chrome::window` is kept (as the original keeps each window's place in its `*Config` value,
//! `Window::SaveWndConfig`), except the fixed dialogs ([`FIXED`]); the HUD areas are listed in [`NAMES`]. The tabs the
//! original keeps in a window's config are in `tabs.ini` beside it (`name tab`, by the config's name): the perk window's
//! `PerkWindowConfig` `SelectedTab` (FUN_10064a67), the wear and inspect windows' `WearViewConfig` / `InspectViewConfig`
//! `selected_tab` (FUN_100e1ce0). The research window keeps only its place (`ResearchConfig`, FUN_100f5461).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The HUD areas whose positions are kept (the ids their `egui::Area` uses); the windows are added as they are shown ([`track`]).
pub const NAMES: &[&str] = &["vitals", "compass", "CCHealthBarConfig", "CCNanoBarConfig", "CCXPBarConfig", "CCAlienXPBarConfig"];

/// The windows whose place is not kept: the dialogs held in the middle of the screen (`DialogBox_c`, which has no config) and
/// the windows anchored where they are (the login screens', the client's options and Whom-Pah list).
pub const FIXED: &[&str] = &["Login", "Progress", "NewCharacter", "Delete Character", "Options", "Whom-Pah", "remove-friend", "delete-mail", "perk-dialog", "TradeDoubleConfirmDlg", "team-invite", "disable-tips"];

/// Whether a window's place is kept.
pub fn kept(name: &str) -> bool {
    !FIXED.contains(&name)
}

struct Layout {
    spots: BTreeMap<String, [f32; 2]>,
    /// The original client's own layout for the character (`aoprefs.rs`), looked up once the character is known; the
    /// screen's size in points, noted each frame, to scale it.
    original: Option<Option<crate::aoprefs::Original>>,
    character: Option<u32>,
    screen: [f32; 2],
    /// The windows whose place was noted before the character was known (the loading screens' frames): their place is
    /// dropped once it is, and the window is moved to the original's frame (`relocate`).
    auto: BTreeSet<String>,
    relocate: BTreeSet<String>,
    /// The windows that asked for a place while the screen's size was not yet known (egui's first frames report a
    /// 10000-point screen): moved to the original's frame once it is.
    pending: BTreeSet<String>,
    /// The windows shown since the client started (their places are noted each frame).
    shown: BTreeSet<String>,
    loaded: bool,
    dirty: bool,
    written: Option<Instant>,
    /// `tabs.ini`, once read.
    tabs: Option<BTreeMap<String, usize>>,
}

static LAYOUT: Mutex<Layout> = Mutex::new(Layout { spots: BTreeMap::new(), original: None, character: None, screen: [0.0; 2], auto: BTreeSet::new(), relocate: BTreeSet::new(), pending: BTreeSet::new(), shown: BTreeSet::new(), loaded: false, dirty: false, written: None, tabs: None });

fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("layout.ini")
}

/// (the tests never read or write the player's files)
fn disabled() -> bool {
    cfg!(test) || std::env::var_os("MAO_NO_LAYOUT").is_some() || std::env::var_os("MAO_UI_PREVIEW").is_some()
}

/// The lines of a layout file: `name x y` (the name may hold spaces; the last two words are the position).
pub fn parse(text: &str) -> BTreeMap<String, [f32; 2]> {
    text.lines()
        .filter_map(|line| {
            let mut words: Vec<&str> = line.split_whitespace().collect();
            let y = words.pop()?.parse().ok()?;
            let x = words.pop()?.parse().ok()?;
            (!words.is_empty()).then(|| (words.join(" "), [x, y]))
        })
        .collect()
}

pub fn render(spots: &BTreeMap<String, [f32; 2]>) -> String {
    spots.iter().map(|(name, p)| format!("{name} {} {}\n", p[0].round(), p[1].round())).collect()
}

/// Where a window starts: its saved place, else `default`.
pub fn pos(name: &str, default: egui::Pos2) -> egui::Pos2 {
    saved(name).unwrap_or(default)
}

/// A window's saved place, else where the original client's layout for this character puts it (`aoprefs.rs`, scaled to
/// the screen), else none.
pub fn saved(name: &str) -> Option<egui::Pos2> {
    if disabled() {
        return None;
    }
    let mut layout = LAYOUT.lock().unwrap();
    if !layout.loaded {
        layout.loaded = true;
        layout.spots = std::fs::read_to_string(path()).map(|t| parse(&t)).unwrap_or_default();
    }
    if let Some(p) = layout.spots.get(name) {
        return Some(egui::pos2(p[0], p[1]));
    }
    if !name.is_empty() && !screen_known(&layout) {
        layout.pending.insert(name.to_string());
        return None;
    }
    original_frame(&mut layout, name).map(|f| egui::pos2(f[0], f[1]))
}

/// Whether the screen's size is a real one (egui's first frames, before the window is sized, report squares of 5000 or
/// 10000 points).
fn screen_known(layout: &Layout) -> bool {
    let [w, h] = layout.screen;
    w >= 200.0 && h >= 100.0 && w < 8000.0 && h < 8000.0 && !(w == h && w >= 4000.0)
}

/// The screen's size in points, once it is a real one.
pub fn screen() -> Option<[f32; 2]> {
    let layout = LAYOUT.lock().unwrap();
    screen_known(&layout).then_some(layout.screen)
}

/// The original's frame for a window, scaled: left, top, width, height.
fn original_frame(layout: &mut Layout, name: &str) -> Option<[f32; 4]> {
    let character = layout.character?;
    if layout.original.is_none() {
        layout.original = Some(crate::aoprefs::load(character));
        if std::env::var_os("MAO_TRACE_LAYOUT").is_some() {
            eprintln!("original layout for {character}: {:?}", layout.original.as_ref().unwrap().as_ref().map(|o| (o.display, o.frames.len(), crate::aoprefs::prefs_dirs(character))));
        }
    }
    let screen = layout.screen;
    let frame = layout.original.as_ref()?.as_ref()?.frame(name, screen);
    if std::env::var_os("MAO_TRACE_LAYOUT").is_some() {
        eprintln!("original frame for {name:?} on {screen:?}: {frame:?}");
    }
    frame
}

/// The size the original's layout gives a window, when it has one and the player has not placed the window here yet.
pub fn original_size(name: &str) -> Option<egui::Vec2> {
    if disabled() {
        return None;
    }
    let mut layout = LAYOUT.lock().unwrap();
    if layout.spots.contains_key(name) || !screen_known(&layout) {
        return None;
    }
    original_frame(&mut layout, name).map(|f| egui::vec2(f[2], f[3]))
}

/// The character playing (the original's per-character preferences are looked up by its id).
pub fn set_character(id: u32) {
    let mut layout = LAYOUT.lock().unwrap();
    if layout.character != Some(id) {
        layout.character = Some(id);
        layout.original = None;
        // (places noted while no character was known are not the player's: the windows go to the original's frames)
        let auto = std::mem::take(&mut layout.auto);
        for name in auto {
            layout.spots.remove(&name);
            layout.relocate.insert(name);
        }
    }
}

/// Whether a window is to be moved to its starting place now (once, after `set_character` dropped a place noted too early).
pub fn relocate(name: &str) -> bool {
    LAYOUT.lock().unwrap().relocate.remove(name)
}

/// A window shown (by `chrome::window`): its place is noted from now on, unless it is one of the [`FIXED`] ones.
pub fn track(name: &str) {
    if disabled() || !kept(name) {
        return;
    }
    let mut layout = LAYOUT.lock().unwrap();
    if !layout.shown.contains(name) {
        layout.shown.insert(name.to_string());
    }
}

/// The lines of `tabs.ini`: `name tab` (the config's name, then the tab's number).
pub fn parse_tabs(text: &str) -> BTreeMap<String, usize> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let tab = words.next()?.parse().ok()?;
            words.next().is_none().then(|| (name.to_string(), tab))
        })
        .collect()
}

pub fn render_tabs(tabs: &BTreeMap<String, usize>) -> String {
    tabs.iter().map(|(name, tab)| format!("{name} {tab}\n")).collect()
}

fn with_tabs<R>(f: impl FnOnce(&mut BTreeMap<String, usize>) -> R) -> R {
    let mut layout = LAYOUT.lock().unwrap();
    let tabs = layout.tabs.get_or_insert_with(|| std::fs::read_to_string(path().with_file_name("tabs.ini")).map(|t| parse_tabs(&t)).unwrap_or_default());
    f(tabs)
}

/// The tab a window's config kept (`PerkWindowConfig`, `WearViewConfig`, `InspectViewConfig`).
pub fn tab(config: &str) -> Option<usize> {
    if disabled() {
        return None;
    }
    with_tabs(|tabs| tabs.get(config).copied())
}

/// Keeps a window's selected tab (written when it changes; the original writes the config when the window goes).
pub fn set_tab(config: &str, tab: usize) {
    if disabled() {
        return;
    }
    with_tabs(|tabs| {
        if tabs.get(config) != Some(&tab) {
            tabs.insert(config.to_string(), tab);
            let _ = std::fs::write(path().with_file_name("tabs.ini"), render_tabs(tabs));
        }
    });
}

/// Notes where the windows are now (call once a frame); the file is written when something moved, at most every two seconds.
pub fn remember(ctx: &egui::Context) {
    if disabled() {
        return;
    }
    // (the saved places first, so the file written keeps the windows not shown this time)
    let _ = saved("");
    let mut layout = LAYOUT.lock().unwrap();
    let screen = ctx.content_rect().size();
    layout.screen = [screen.x, screen.y];
    if !screen_known(&layout) {
        return;
    }
    // (the windows placed before the screen had its size are moved to their starting places now)
    let pending = std::mem::take(&mut layout.pending);
    for name in pending {
        layout.spots.remove(&name);
        layout.relocate.insert(name);
    }
    let names: Vec<String> = NAMES.iter().map(|n| n.to_string()).chain(layout.shown.iter().cloned()).collect();
    for name in names {
        // (a window about to be moved to its starting place is noted once it is there)
        if layout.relocate.contains(&name) {
            continue;
        }
        if let Some(rect) = ctx.memory(|m| m.area_rect(egui::Id::unique(name.as_str()))) {
            let now = [rect.min.x, rect.min.y];
            if layout.spots.get(&name).is_none_or(|old| (old[0] - now[0]).abs() > 1.0 || (old[1] - now[1]).abs() > 1.0) {
                if layout.character.is_none() && !layout.spots.contains_key(&name) {
                    layout.auto.insert(name.clone());
                }
                layout.spots.insert(name, now);
                layout.dirty = true;
            }
        }
    }
    if layout.dirty && layout.written.is_none_or(|t| t.elapsed() > Duration::from_secs(2)) {
        layout.dirty = false;
        layout.written = Some(Instant::now());
        let _ = std::fs::write(path(), render(&layout.spots));
    }
}

/// Writes the windows' places now if any moved since the last write (the game closing: a logout, `/quit`, the window shut).
pub fn flush() {
    if disabled() {
        return;
    }
    let mut layout = LAYOUT.lock().unwrap();
    if layout.dirty {
        layout.dirty = false;
        layout.written = Some(Instant::now());
        let _ = std::fs::write(path(), render(&layout.spots));
    }
}

/// The windows that were open when the client last ran (`open.ini` beside `layout.ini`, one name per line); none when
/// there is no file yet (the first start).
pub fn load_open() -> Option<Vec<String>> {
    if disabled() {
        return Some(Vec::new());
    }
    std::fs::read_to_string(path().with_file_name("open.ini")).ok().map(|t| t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}

pub fn save_open(names: &[&str]) {
    if !disabled() {
        let _ = std::fs::write(path().with_file_name("open.ini"), names.iter().map(|n| format!("{n}
")).collect::<String>());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_round_trips() {
        let mut spots = BTreeMap::new();
        spots.insert("Nano programs".to_string(), [220.0, 90.0]);
        spots.insert("Chat".to_string(), [8.0, 450.5]);
        let text = render(&spots);
        assert_eq!(text, "Chat 8 451\nNano programs 220 90\n");
        let back = parse(&text);
        assert_eq!(back.get("Nano programs"), Some(&[220.0, 90.0]));
        assert_eq!(back.get("Chat"), Some(&[8.0, 451.0]));
        assert!(parse("junk\n 5 6\nname x y\n").is_empty());
    }

    #[test]
    fn the_tabs_file_round_trips() {
        let mut tabs = BTreeMap::new();
        tabs.insert("PerkWindowConfig".to_string(), 2);
        tabs.insert("WearViewConfig".to_string(), 0);
        let text = render_tabs(&tabs);
        assert_eq!(text, "PerkWindowConfig 2\nWearViewConfig 0\n");
        assert_eq!(parse_tabs(&text), tabs);
        // (a name with no number, a negative number, or more words is left out)
        assert!(parse_tabs("PerkWindowConfig\nWearViewConfig -1\nInspect View 3\n").is_empty());
    }

    #[test]
    fn dialogs_keep_no_place() {
        assert!(kept("Tradeskill") && kept("PetWindow") && kept("Inspect") && kept("Research") && kept("Faction"));
        assert!(!kept("perk-dialog") && !kept("Login") && !kept("TradeDoubleConfirmDlg"));
    }
}
