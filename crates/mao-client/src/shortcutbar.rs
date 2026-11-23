//! The original's shortcut bars (GUI.dll `ShortcutBarWindow_c` FUN_100d9e5f, `ShortcutBarView_c` FUN_100d9588): up to ten
//! bars (the `NumHotbars` preference, 1 to 10, one by default), each a row of ten slots over ten pages (a 10 by 10 grid of
//! which one row shows: slot `s` of page `p` is cell `s + 10 p`, FUN_100d8665). One bar is the primary: the keys 1 to 0 press
//! its slots (`SHORTUCT_BAR_ACTIVE_n`), Shift+1 to 0 pick its page (`SHORTUCT_BAR_ROW_n`, FUN_100d8f71) and Alt+1 to 0 make a
//! bar the primary (`SHORTUCT_BAR_SELECT_n`, FUN_100d8622). A slot holds one shortcut: an item, a nano program, a special
//! action or a text macro (FUN_100d825b), and a click does it (FUN_100d833f). The bar is moved by its control strip, turned
//! upright by a double click on it, and neither while `LockHotbars` is set (FUN_100dbc76, FUN_100dbe17).
//!
//! The smart hotbars (`hotbar.rs`) are this client's own, kept as a setting.

use crate::hotbar::Action;

pub const BARS: usize = 10;
pub const PAGES: usize = 10;
pub const SLOTS: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    /// The shortcuts, cell `slot + 10 * page`.
    pub cells: Vec<Option<Action>>,
    /// The page shown (`selected_bar` in the original's file).
    pub page: usize,
    /// Upright (`bar_orientation` not 0).
    pub vertical: bool,
    /// Where the bar's top left is, in points (none: its first place).
    pub pos: Option<[f32; 2]>,
}

impl Default for Bar {
    fn default() -> Bar {
        Bar { cells: vec![None; PAGES * SLOTS], page: 0, vertical: false, pos: None }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShortcutBars {
    pub bars: Vec<Bar>,
    /// The primary bar (`IsPrimary`).
    pub primary: usize,
}

impl Default for ShortcutBars {
    fn default() -> ShortcutBars {
        ShortcutBars { bars: vec![Bar::default(); BARS], primary: 0 }
    }
}

impl ShortcutBars {
    /// The shortcut in a slot of a bar's shown page.
    pub fn shown(&self, bar: usize, slot: usize) -> Option<&Action> {
        let b = self.bars.get(bar)?;
        b.cells.get(slot + SLOTS * b.page)?.as_ref()
    }

    pub fn shown_mut(&mut self, bar: usize, slot: usize) -> Option<&mut Option<Action>> {
        let b = self.bars.get_mut(bar)?;
        let page = b.page;
        b.cells.get_mut(slot + SLOTS * page)
    }

    /// A slot key (`SHORTUCT_BAR_ACTIVE_n`, FUN_100d8f45): the primary bar's shortcut there, on its shown page.
    pub fn press(&self, slot: usize) -> Option<Action> {
        self.shown(self.primary, slot).cloned()
    }

    /// Makes bar `n` the primary (`SHORTUCT_BAR_SELECT_n`, FUN_100d8622), when there is a bar `n` (`count` bars shown).
    pub fn select(&mut self, n: usize, count: usize) {
        if n < count.min(BARS) {
            self.primary = n;
        }
    }

    /// Shows page `n` of the primary bar (`SHORTUCT_BAR_ROW_n`).
    pub fn row(&mut self, n: usize) {
        let primary = self.primary;
        if let Some(b) = self.bars.get_mut(primary) {
            b.page = n % PAGES;
        }
    }

    /// The wheel over a bar (FUN_100dbef9): up the page before, down the next, round from 0 to 9.
    pub fn wheel(&mut self, bar: usize, up: bool) {
        if let Some(b) = self.bars.get_mut(bar) {
            b.page = if up { (b.page + PAGES - 1) % PAGES } else { (b.page + 1) % PAGES };
        }
    }

    /// The first run's shortcuts (FUN_100d9e5f, while `IsFirstTime`): on the first bar Start Combat, Walk, Sit, a macro
    /// "Follow" that says `/follow` (the macro's number is given) and Suspended Animation in the last slot.
    pub fn first_run(follow: Option<i32>) -> ShortcutBars {
        let mut bars = ShortcutBars::default();
        let special = |id: u32, name: &str, icon: u32| Some(Action::Special { id, name: name.to_string(), icon: Some(icon) });
        let cells = &mut bars.bars[0].cells;
        cells[0] = special(0xc1a5, "Start Combat", 49565);
        cells[1] = special(0xc1a2, "Walk", 49560);
        cells[2] = special(0xc1a8, "Sit", 49564);
        cells[3] = follow.map(|id| Action::Macro { id, name: "Follow".to_string() });
        cells[9] = special(0x14124, "Suspended Animation", 82199);
        bars
    }
}

// --- Saving: `shortcutbars.ini` (the original keeps `Prefs/.../Char<n>/Containers/ShortcutBar_<n>.xml`, which this client
// does not write) ---

fn action_text(action: &Action) -> String {
    let icon = |icon: &Option<u32>| icon.map_or("-".to_string(), |i| i.to_string());
    match action {
        Action::Item { id, name, icon: i } => format!("item {id} {} {name}", icon(i)),
        Action::Attack => "attack".to_string(),
        Action::Special { id, name, icon: i } => format!("special {id} {} {name}", icon(i)),
        Action::Social { id, name } => format!("social {id} {name}"),
        Action::Nano { id, name, icon: i } => format!("nano {id} {} {name}", icon(i)),
        Action::Macro { id, name } => format!("macro {id} {name}"),
    }
}

fn parse_action(text: &str) -> Option<Action> {
    let words: Vec<&str> = text.split_whitespace().collect();
    Some(match words.as_slice() {
        ["attack"] => Action::Attack,
        ["item", id, icon, name @ ..] => Action::Item { id: id.parse().ok()?, icon: icon.parse().ok(), name: name.join(" ") },
        ["special", id, icon, name @ ..] => Action::Special { id: id.parse().ok()?, icon: icon.parse().ok(), name: name.join(" ") },
        ["nano", id, icon, name @ ..] => Action::Nano { id: id.parse().ok()?, icon: icon.parse().ok(), name: name.join(" ") },
        ["social", id, name @ ..] => Action::Social { id: id.parse().ok()?, name: name.join(" ") },
        ["macro", id, name @ ..] => Action::Macro { id: id.parse().ok()?, name: name.join(" ") },
        _ => return None,
    })
}

/// One line a bar (`bar <n> = <page> <horizontal|vertical> <x y | - ->`) and one a shortcut (`<bar>.<page>.<slot> = ...`).
pub fn to_text(bars: &ShortcutBars) -> String {
    let mut out = format!("; Shortcut bars: bar <n> = page orientation x y; <bar>.<page>.<slot> = item|special|nano <id> <icon> <name>, social|macro <id> <name>\nprimary = {}\n", bars.primary + 1);
    for (n, b) in bars.bars.iter().enumerate() {
        let pos = b.pos.map_or("- -".to_string(), |[x, y]| format!("{x:.0} {y:.0}"));
        out += &format!("bar {} = {} {} {pos}\n", n + 1, b.page + 1, if b.vertical { "vertical" } else { "horizontal" });
        for (k, cell) in b.cells.iter().enumerate() {
            if let Some(action) = cell {
                out += &format!("{}.{}.{} = {}\n", n + 1, k / SLOTS + 1, k % SLOTS + 1, action_text(action).replace('\n', " "));
            }
        }
    }
    out
}

pub fn from_text(text: &str) -> ShortcutBars {
    let mut bars = ShortcutBars::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim());
        if key == "primary" {
            bars.primary = value.parse::<usize>().ok().filter(|n| (1..=BARS).contains(n)).map_or(0, |n| n - 1);
        } else if let Some(n) = key.strip_prefix("bar ").and_then(|n| n.trim().parse::<usize>().ok()).filter(|n| (1..=BARS).contains(n)) {
            let words: Vec<&str> = value.split_whitespace().collect();
            let b = &mut bars.bars[n - 1];
            b.page = words.first().and_then(|p| p.parse::<usize>().ok()).filter(|p| (1..=PAGES).contains(p)).map_or(0, |p| p - 1);
            b.vertical = words.get(1) == Some(&"vertical");
            b.pos = words.get(2).and_then(|x| x.parse().ok()).zip(words.get(3).and_then(|y| y.parse().ok())).map(|(x, y)| [x, y]);
        } else {
            let parts: Vec<usize> = key.split('.').filter_map(|p| p.trim().parse().ok()).collect();
            if let [bar, page, slot] = parts[..]
                && (1..=BARS).contains(&bar)
                && (1..=PAGES).contains(&page)
                && (1..=SLOTS).contains(&slot)
            {
                bars.bars[bar - 1].cells[(page - 1) * SLOTS + slot - 1] = parse_action(value);
            }
        }
    }
    bars
}

pub fn path() -> std::path::PathBuf {
    crate::charprefs::file("shortcutbars.ini")
}

/// The bars as kept, or the first run's (with its macro made) when none were kept.
pub fn load() -> ShortcutBars {
    match std::fs::read_to_string(path()) {
        Ok(text) => from_text(&text),
        Err(_) => {
            let bars = ShortcutBars::first_run(Some(crate::macros::create("Follow", "/follow")));
            save(&bars);
            bars
        }
    }
}

pub fn save(bars: &ShortcutBars) {
    let path = path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, to_text(bars)) {
        eprintln!("can't save {}: {e}", path.display());
    }
}

/// The icon a shortcut is drawn with (a number of the database's icons, type 0xf6958): the one kept with it, else its own
/// record's (stat 79): an item's template, a nano program's record (0xfde85), a special action's template, as the original
/// draws every shortcut by its item or nano's icon (`ShortcutBarView_c`'s cells are `ItemIconView_c`s, FUN_100d825b).
/// None for a macro (drawn as `GFX_GUI_ICON_MACRO`) and the attack. Looked up once each.
pub fn icon_id(action: &Action) -> Option<u32> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static KEPT: Mutex<Option<HashMap<(u32, u32), Option<u32>>>> = Mutex::new(None);
    let own = |kind: u32, id: u32| {
        let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
        let kept = kept.get_or_insert_with(HashMap::new);
        if let Some(icon) = kept.get(&(kind, id)) {
            return *icon;
        }
        let db = crate::net::item_db()?;
        let icon = db.get(mao_formats::resource::Id { kind, instance: id }).ok().flatten().as_deref().and_then(mao_formats::item::Item::parse).and_then(|t| t.icon());
        kept.insert((kind, id), icon);
        icon
    };
    match action {
        Action::Item { id, icon, .. } | Action::Special { id, icon, .. } => icon.or_else(|| own(mao_formats::item::TEMPLATE, *id)),
        Action::Nano { id, icon, .. } => icon.or_else(|| own(0xfde85, *id)),
        Action::Attack | Action::Social { .. } | Action::Macro { .. } => None,
    }
}

/// The option panel's shortcut bar options as they changed this frame (`changed`: the control center's values that
/// changed, by name, as numbers): "Number of shortcut bars" (`NumHotbars`, 1 to 10, which `ShortcutBarWindow_c` FUN_100d9e5f
/// reads for the bars it shows) and "Lock shortcut bars" (`LockHotbars`), taken into this client's own settings of them,
/// which the bars are drawn by. Whether either changed.
pub fn follow_options(changed: &[(String, f64)], hotbars: &mut u8, locked: &mut bool) -> bool {
    let mut any = false;
    if let Some(n) = changed.iter().rev().find(|c| c.0 == "NumHotbars").map(|c| c.1.round().clamp(1.0, BARS as f64) as u8)
        && *hotbars != n
    {
        *hotbars = n;
        any = true;
    }
    if let Some(on) = changed.iter().rev().find(|c| c.0 == "LockHotbars").map(|c| c.1 != 0.0)
        && *locked != on
    {
        *locked = on;
        any = true;
    }
    any
}

/// A shortcut's time left as the original writes it over the icon (`ItemIconView_c`, `IconTimerText`): seconds up to a
/// minute (`%2ds`), else whole minutes (`%2dm`).
pub fn timer_text(left: f32) -> String {
    let s = left.ceil() as i32;
    if s <= 60 { format!("{s:2}s") } else { format!("{:2}m", s / 60) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keys: 1 to 0 press the primary bar's shown page, Shift picks its page, Alt the primary (only a bar there is).
    #[test]
    fn the_keys_go_to_the_primary_bars_shown_page() {
        let mut bars = ShortcutBars::first_run(Some(4));
        assert_eq!(bars.press(1), Some(Action::Special { id: 0xc1a2, name: "Walk".into(), icon: Some(49560) }));
        assert_eq!(bars.press(3), Some(Action::Macro { id: 4, name: "Follow".into() }));
        assert_eq!(bars.press(5), None);
        bars.row(2);
        assert_eq!((bars.bars[0].page, bars.press(1)), (2, None));
        *bars.shown_mut(0, 1).unwrap() = Some(Action::Attack);
        assert_eq!((bars.press(1), bars.bars[0].cells[21].clone()), (Some(Action::Attack), Some(Action::Attack)));
        bars.select(3, 1);
        assert_eq!(bars.primary, 0);
        bars.select(3, 4);
        assert_eq!((bars.primary, bars.press(1)), (3, None));
        // (the wheel: up the page before, round from 0 to 9)
        bars.wheel(0, true);
        bars.wheel(3, true);
        assert_eq!((bars.bars[0].page, bars.bars[3].page), (1, 9));
        bars.wheel(3, false);
        assert_eq!(bars.bars[3].page, 0);
    }

    #[test]
    fn the_bars_round_trip_through_text() {
        let mut bars = ShortcutBars::first_run(Some(2));
        bars.primary = 1;
        bars.bars[1].vertical = true;
        bars.bars[1].pos = Some([12.0, 300.0]);
        bars.bars[1].page = 4;
        bars.bars[1].cells[47] = Some(Action::Nano { id: 75000, name: "Heal".into(), icon: None });
        bars.bars[2].cells[0] = Some(Action::Item { id: 291043, name: "Health and Nano Stim".into(), icon: Some(17) });
        assert_eq!(from_text(&to_text(&bars)), bars);
        assert_eq!(from_text(""), ShortcutBars::default());
    }

    /// The option panel's slider and check box set the bars shown and the lock (they set values nothing read before).
    #[test]
    fn the_option_panel_sets_the_bars_shown_and_their_lock() {
        let (mut n, mut locked) = (1u8, false);
        assert!(!follow_options(&[("ViewDistance".into(), 0.5)], &mut n, &mut locked));
        assert!(follow_options(&[("NumHotbars".into(), 3.0)], &mut n, &mut locked));
        assert_eq!((n, locked), (3, false));
        assert!(!follow_options(&[("NumHotbars".into(), 3.0)], &mut n, &mut locked));
        assert!(follow_options(&[("NumHotbars".into(), 14.0), ("LockHotbars".into(), 1.0)], &mut n, &mut locked));
        assert_eq!((n, locked), (10, true));
        assert!(follow_options(&[("NumHotbars".into(), 0.0), ("LockHotbars".into(), 0.0)], &mut n, &mut locked));
        assert_eq!((n, locked), (1, false));
    }

    #[test]
    fn the_timer_says_seconds_then_minutes() {
        assert_eq!((timer_text(4.2).as_str(), timer_text(60.0).as_str(), timer_text(61.0).as_str(), timer_text(600.0).as_str()), (" 5s", "60s", " 1m", "10m"));
    }
}
