//! Where the player left the windows: each movable window's position is saved in `layout.ini` beside `mao.ini` (one line per
//! window, `name x y`) and used as the window's starting place the next time. `MAO_NO_LAYOUT=1` ignores it; the offline previews
//! (`MAO_UI_PREVIEW`) ignore it too so their pictures do not depend on the player's layout.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The windows whose positions are kept: the ids their `egui::Window` / `Area` use.
pub const NAMES: &[&str] = &["Actions", "Help", "Chat", "Friends", "Inventory", "loot", "Mail", "Message", "Missions", "Nano programs", "NCU", "Perks", "Shop", "Skills", "Stats", "vitals"];

struct Layout {
    spots: BTreeMap<String, [f32; 2]>,
    loaded: bool,
    dirty: bool,
    written: Option<Instant>,
}

static LAYOUT: Mutex<Layout> = Mutex::new(Layout { spots: BTreeMap::new(), loaded: false, dirty: false, written: None });

fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("layout.ini")
}

fn disabled() -> bool {
    std::env::var_os("MAO_NO_LAYOUT").is_some() || std::env::var_os("MAO_UI_PREVIEW").is_some()
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
    if disabled() {
        return default;
    }
    let mut layout = LAYOUT.lock().unwrap();
    if !layout.loaded {
        layout.loaded = true;
        layout.spots = std::fs::read_to_string(path()).map(|t| parse(&t)).unwrap_or_default();
    }
    layout.spots.get(name).map_or(default, |p| egui::pos2(p[0], p[1]))
}

/// Notes where the windows are now (call once a frame); the file is written when something moved, at most every two seconds.
pub fn remember(ctx: &egui::Context) {
    if disabled() {
        return;
    }
    let mut layout = LAYOUT.lock().unwrap();
    for &name in NAMES {
        if let Some(rect) = ctx.memory(|m| m.area_rect(egui::Id::unique(name))) {
            let now = [rect.min.x, rect.min.y];
            if layout.spots.get(name).is_none_or(|old| (old[0] - now[0]).abs() > 1.0 || (old[1] - now[1]).abs() > 1.0) {
                layout.spots.insert(name.to_string(), now);
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
}
