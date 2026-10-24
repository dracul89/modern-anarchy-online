//! Smart hotbars: 6 bars of 10 slots, each slot a priority list of actions.
//!
//! Pressing a slot fires the first action in its list that is ready and puts
//! it on cooldown, so while it recovers the slot offers the next one (an
//! opener, a filler, a finisher ...). When nothing in the slot is ready the
//! press is remembered (queued): the slot fires the moment something frees up,
//! and the player is told it was not ready. Time is passed in (seconds), so the
//! rules are testable without a clock.

use std::collections::VecDeque;

pub const BARS: usize = 6;
pub const SLOTS: usize = 10;

/// What an entry does.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Use an item from the inventory (by its template id; found in the backpack when fired).
    Item { id: u32, name: String, icon: Option<u32> },
    /// Attack the current target.
    Attack,
    /// A special action of the Action window (a weapon attack, a basic action, a perk), by its id; `icon` is the
    /// instance of its icon.
    Special { id: u32, name: String, icon: Option<u32> },
    /// A social action or dance (by its `AbstractAnimID`).
    Social { id: u32, name: String },
    /// Cast a nano program (by its id) at the target.
    Nano { id: u32, name: String, icon: Option<u32> },
}

impl Action {
    pub fn name(&self) -> &str {
        match self {
            Action::Item { name, .. } => name,
            Action::Attack => "Attack",
            Action::Special { name, .. } | Action::Social { name, .. } | Action::Nano { name, .. } => name,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub action: Action,
    /// How long the action stays unavailable after firing, in seconds.
    pub cooldown: f32,
    /// When it is ready again (seconds on the caller's clock).
    pub ready_at: f64,
}

impl Entry {
    pub fn new(action: Action, cooldown: f32) -> Entry {
        Entry { action, cooldown, ready_at: 0.0 }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Slot {
    pub entries: Vec<Entry>,
}

/// What a slot shows: the entry that would fire, and how long until it can.
pub struct Display<'a> {
    pub entry: &'a Entry,
    /// 0 when ready.
    pub remaining: f32,
}

impl Slot {
    /// The first entry that is ready at `now`.
    pub fn ready(&self, now: f64) -> Option<usize> {
        self.entries.iter().position(|e| e.ready_at <= now)
    }

    /// The entry to show: the first ready one, else the one ready soonest.
    pub fn display(&self, now: f64) -> Option<Display<'_>> {
        if let Some(i) = self.ready(now) {
            return Some(Display { entry: &self.entries[i], remaining: 0.0 });
        }
        self.entries.iter().min_by(|a, b| a.ready_at.total_cmp(&b.ready_at)).map(|entry| Display { entry, remaining: (entry.ready_at - now).max(0.0) as f32 })
    }

    /// Fires the first ready entry: puts it on cooldown and returns its action.
    fn fire(&mut self, now: f64) -> Option<Action> {
        let i = self.ready(now)?;
        let entry = &mut self.entries[i];
        entry.ready_at = now + entry.cooldown as f64;
        Some(entry.action.clone())
    }
}

/// The result of pressing a slot.
#[derive(Debug, PartialEq)]
pub enum Press {
    /// Do this now.
    Fire(Action),
    /// Nothing was ready: the press is queued; the message says what to wait for.
    NotReady(String),
    /// The slot is empty.
    Empty,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hotbars {
    pub slots: Vec<Slot>,
    /// The bar shown.
    pub current: usize,
    /// Where the bar sits (the middle of its bottom edge, in points; `None` = bottom centre) and whether it is
    /// laid out vertically.
    pub pos: Option<[f32; 2]>,
    pub vertical: bool,
    /// Presses waiting for something to become ready: (bar, slot).
    queued: VecDeque<(usize, usize)>,
}

impl Default for Hotbars {
    fn default() -> Hotbars {
        Hotbars { slots: vec![Slot::default(); BARS * SLOTS], current: 0, pos: None, vertical: false, queued: VecDeque::new() }
    }
}

/// The skill stat the server names a weapon special by (its `SpecialUsed` and `SpecialAvailable` messages), from the action's name.
pub fn special_stat(name: &str) -> Option<u32> {
    Some(match name {
        "Brawl" => 142,
        "Dimach" => 144,
        "Fast Attack" => 147,
        "Burst" => 148,
        "Fling Shot" => 150,
        "Full Auto" => 167,
        _ => return None,
    })
}

impl Hotbars {
    /// The server says a special was used (it recharges for `seconds`) or is ready again (`seconds` 0): every slot entry for it
    /// follows.
    pub fn set_special_cooldown(&mut self, stat: u32, seconds: f64, now: f64) {
        for slot in &mut self.slots {
            for entry in &mut slot.entries {
                if matches!(&entry.action, Action::Special { name, .. } if special_stat(name) == Some(stat)) {
                    entry.ready_at = now + seconds;
                }
            }
        }
    }

    pub fn slot(&self, bar: usize, slot: usize) -> &Slot {
        &self.slots[bar * SLOTS + slot]
    }

    pub fn slot_mut(&mut self, bar: usize, slot: usize) -> &mut Slot {
        &mut self.slots[bar * SLOTS + slot]
    }

    pub fn is_queued(&self, bar: usize, slot: usize) -> bool {
        self.queued.contains(&(bar, slot))
    }

    /// A press of slot `slot` (0..10) on the shown bar.
    pub fn press(&mut self, slot: usize, now: f64) -> Press {
        let bar = self.current;
        let s = self.slot_mut(bar, slot);
        if s.entries.is_empty() {
            return Press::Empty;
        }
        if let Some(action) = s.fire(now) {
            return Press::Fire(action);
        }
        let (name, remaining) = s.display(now).map(|d| (d.entry.action.name().to_string(), d.remaining)).unwrap_or_default();
        if !self.is_queued(bar, slot) {
            self.queued.push_back((bar, slot));
        }
        Press::NotReady(format!("{name} is not ready ({remaining:.1}s): queued"))
    }

    /// Fires the queued presses whose slots have an action ready (oldest first).
    pub fn tick(&mut self, now: f64) -> Vec<Action> {
        let mut fired = Vec::new();
        let mut waiting = VecDeque::new();
        while let Some((bar, slot)) = self.queued.pop_front() {
            match self.slot_mut(bar, slot).fire(now) {
                Some(action) => fired.push(action),
                None => waiting.push_back((bar, slot)),
            }
        }
        self.queued = waiting;
        fired
    }

    /// Forgets queued presses of a slot that was emptied.
    pub fn cleared(&mut self, bar: usize, slot: usize) {
        self.queued.retain(|q| *q != (bar, slot));
    }

    /// Adds an action to the end of a slot's list.
    pub fn add(&mut self, bar: usize, slot: usize, action: Action, cooldown: f32) {
        self.slot_mut(bar, slot).entries.push(Entry::new(action, cooldown));
    }
}

// --- Saving: `hotbar.ini` ---

/// One line per slot: `bar.slot = item 123 6.0 icon 456 Health Stim | attack 2.0`.
pub fn to_text(bars: &Hotbars) -> String {
    let mut out = String::from("; Hotbars: bar.slot = action | action ...   (an action: `item <id> <cooldown> <icon or -> <name>` or `attack <cooldown>`)\n");
    if let Some([x, y]) = bars.pos {
        out += &format!("layout = {x:.0} {y:.0} {}
", if bars.vertical { "vertical" } else { "horizontal" });
    } else if bars.vertical {
        out += "layout = - - vertical
";
    }
    for bar in 0..BARS {
        for slot in 0..SLOTS {
            let entries = &bars.slot(bar, slot).entries;
            if entries.is_empty() {
                continue;
            }
            let parts: Vec<String> = entries
                .iter()
                .map(|e| match &e.action {
                    Action::Item { id, name, icon } => format!("item {id} {} {} {name}", e.cooldown, icon.map_or("-".to_string(), |i| i.to_string())),
                    Action::Attack => format!("attack {}", e.cooldown),
                    Action::Special { id, name, icon } => format!("special {id} {} {} {name}", e.cooldown, icon.map_or("-".to_string(), |i| i.to_string())),
                    Action::Social { id, name } => format!("social {id} {} {name}", e.cooldown),
                    Action::Nano { id, name, icon } => format!("nano {id} {} {} {name}", e.cooldown, icon.map_or("-".to_string(), |i| i.to_string())),
                })
                .collect();
            out += &format!("{}.{} = {}\n", bar + 1, slot + 1, parts.join(" | "));
        }
    }
    out
}

pub fn from_text(text: &str) -> Hotbars {
    let mut bars = Hotbars::default();
    for line in text.lines() {
        let line = line.trim();
        let Some((place, list)) = line.split_once('=') else { continue };
        if place.trim() == "layout" {
            let words: Vec<&str> = list.split_whitespace().collect();
            if let [x, y, how] = words.as_slice() {
                bars.pos = x.parse().ok().zip(y.parse().ok()).map(|(x, y)| [x, y]);
                bars.vertical = *how == "vertical";
            }
            continue;
        }
        let Some((bar, slot)) = place.trim().split_once('.') else { continue };
        let (Ok(bar), Ok(slot)) = (bar.trim().parse::<usize>(), slot.trim().parse::<usize>()) else { continue };
        if !(1..=BARS).contains(&bar) || !(1..=SLOTS).contains(&slot) {
            continue;
        }
        for part in list.split('|') {
            let words: Vec<&str> = part.split_whitespace().collect();
            let entry = match words.as_slice() {
                ["attack", cooldown] => cooldown.parse().ok().map(|c| Entry::new(Action::Attack, c)),
                ["special", id, cooldown, icon, name @ ..] => (|| Some(Entry::new(Action::Special { id: id.parse().ok()?, name: name.join(" "), icon: icon.parse().ok() }, cooldown.parse().ok()?)))(),
                ["nano", id, cooldown, icon, name @ ..] => (|| Some(Entry::new(Action::Nano { id: id.parse().ok()?, name: name.join(" "), icon: icon.parse().ok() }, cooldown.parse().ok()?)))(),
                ["social", id, cooldown, name @ ..] => (|| Some(Entry::new(Action::Social { id: id.parse().ok()?, name: name.join(" ") }, cooldown.parse().ok()?)))(),
                ["item", id, cooldown, icon, name @ ..] => (|| Some(Entry::new(Action::Item { id: id.parse().ok()?, name: name.join(" "), icon: icon.parse().ok() }, cooldown.parse().ok()?)))(),
                _ => None,
            };
            if let Some(entry) = entry {
                bars.slot_mut(bar - 1, slot - 1).entries.push(entry);
            }
        }
    }
    bars
}

pub fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("hotbar.ini")
}

pub fn load() -> Hotbars {
    std::fs::read_to_string(path()).map(|t| from_text(&t)).unwrap_or_default()
}

pub fn save(bars: &Hotbars) {
    let path = path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, to_text(bars)) {
        eprintln!("can't save {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str) -> Action {
        Action::Item { id: 1, name: name.into(), icon: None }
    }

    fn bars() -> Hotbars {
        let mut b = Hotbars::default();
        b.add(0, 0, item("Opener"), 10.0);
        b.add(0, 0, item("Filler"), 3.0);
        b.add(0, 0, Action::Attack, 1.0);
        b
    }

    #[test]
    fn a_press_fires_the_first_ready_action_and_the_slot_moves_on() {
        let mut b = bars();
        assert_eq!(b.press(0, 0.0), Press::Fire(item("Opener")));
        // The opener recovers for 10 s: meanwhile the slot offers the next ones in order.
        assert_eq!(b.slot(0, 0).display(0.5).unwrap().entry.action, item("Filler"));
        assert_eq!(b.press(0, 0.5), Press::Fire(item("Filler")));
        assert_eq!(b.press(0, 0.6), Press::Fire(Action::Attack));
        // Everything is cooling down: not ready.
        assert!(matches!(b.press(0, 0.7), Press::NotReady(_)));
        // The attack is back after 1 s; the opener only after 10 s (priority: opener first once ready).
        assert_eq!(b.press(0, 1.7), Press::Fire(Action::Attack));
        assert_eq!(b.press(0, 10.1), Press::Fire(item("Opener")));
    }

    #[test]
    fn a_press_with_nothing_ready_is_queued_and_fires_when_something_frees_up() {
        let mut b = bars();
        for _ in 0..3 {
            b.press(0, 0.0);
        }
        let Press::NotReady(message) = b.press(0, 0.2) else { panic!("should not be ready") };
        assert!(message.contains("not ready") && message.contains("queued"), "{message}");
        assert!(b.is_queued(0, 0));
        // Pressing again doesn't queue twice.
        b.press(0, 0.3);
        assert!(b.tick(0.5).is_empty());
        // The attack recovers at 1 s: the queued press fires it.
        assert_eq!(b.tick(1.05), vec![Action::Attack]);
        assert!(!b.is_queued(0, 0));
        assert!(b.tick(1.1).is_empty());
    }

    #[test]
    fn the_display_shows_the_soonest_ready_action_when_all_are_cooling_down() {
        let mut b = bars();
        for _ in 0..3 {
            b.press(0, 0.0);
        }
        let d = b.slot(0, 0).display(0.4).unwrap();
        assert_eq!(d.entry.action, Action::Attack);
        assert!((d.remaining - 0.6).abs() < 1e-4);
    }

    #[test]
    fn the_server_sets_a_specials_cooldown() {
        let mut b = Hotbars::default();
        b.add(0, 0, Action::Special { id: 82210, name: "Brawl".into(), icon: None }, 1.0);
        b.add(0, 1, Action::Attack, 1.0);
        b.set_special_cooldown(142, 15.0, 100.0);
        assert_eq!(b.slot(0, 0).entries[0].ready_at, 115.0);
        assert_eq!(b.slot(0, 1).entries[0].ready_at, 0.0);
        b.set_special_cooldown(142, 0.0, 104.0);
        assert_eq!(b.slot(0, 0).entries[0].ready_at, 104.0);
        // another special is left alone
        b.set_special_cooldown(150, 8.0, 104.0);
        assert_eq!(b.slot(0, 0).entries[0].ready_at, 104.0);
    }

    #[test]
    fn empty_slots_do_nothing() {
        assert_eq!(Hotbars::default().press(5, 0.0), Press::Empty);
    }

    #[test]
    fn hotbars_round_trip_through_text() {
        let mut b = bars();
        b.add(2, 9, Action::Item { id: 77, name: "Health and Nano Stim".into(), icon: Some(1234) }, 6.5);
        let again = from_text(&to_text(&b));
        assert_eq!(again.slots, b.slots);
    }

    #[test]
    fn the_layout_is_kept() {
        let mut b = Hotbars::default();
        b.pos = Some([400.0, 300.0]);
        b.vertical = true;
        let back = from_text(&to_text(&b));
        assert_eq!((back.pos, back.vertical), (Some([400.0, 300.0]), true));
        assert_eq!(from_text(&to_text(&Hotbars::default())).pos, None);
    }
}
