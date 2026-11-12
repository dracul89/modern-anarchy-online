//! Where things are: what each of the player's characters carries, written down as the client sees it, and searched
//! across all of them. This client's own (the original has nothing like it; players kept lists by hand or with outside
//! programs).
//!
//! The server only tells a character's things while that character is played, and a container's contents only while
//! the container is open. So the client keeps a file for each character it has played on this machine, in the
//! settings folder's `items` folder, with a section for each place and when the place was last seen:
//!
//! * what is worn and what is in the inventory: known all the time, kept up to date as things move;
//! * each bag in the inventory: its contents as they were when it was last opened. A bag never opened here is listed
//!   as one not looked into, so the search can say that there is more it cannot see;
//! * the bank: as it was when a bank terminal was last used.
//!
//! The window's "Look into my bags" has the client use each bag of the character played that it has not seen into yet,
//! one after the other and without opening their windows, so one press makes the character's bags known.
//!
//! A search goes through every character's file, so it finds the thing on a character that is not logged in, and says
//! which character, where on it, and how old that knowledge is. What the client never saw (a bag inside the bank, a
//! character never played with this client) it cannot find.
//!
//! The file is plain text, a line a field, tabs between: `name`, `dimension`, then `section <key> <seen> <looked>
//! <title>` followed by its `item <low> <high> <quality> <count> <name>` lines.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::theme::Palette;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub low: u32,
    pub high: u32,
    pub quality: u32,
    pub count: u32,
    pub name: String,
}

/// A place on a character and what was in it when last seen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// `worn`, `inventory`, `bank`, or `bag:<inventory place>:<the bag's template>`.
    pub key: String,
    /// What the place is called in the results: "Inventory", "Bank", the bag's name.
    pub title: String,
    /// When it was seen (seconds of the system's clock).
    pub seen: u64,
    /// False for a bag that is known to be there but was never looked into.
    pub looked: bool,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Character {
    pub name: String,
    /// The server the character is on (the login's address), to tell two dimensions' characters of one name apart.
    pub dimension: String,
    pub sections: Vec<Section>,
}

/// A bag in the inventory as the client has it now: opened (its contents) or not.
pub struct Bag {
    pub place: u32,
    pub template: u32,
    pub name: String,
    pub contents: Option<Vec<Item>>,
}

impl Character {
    /// Brings the character's sections up to what the client has now. The worn things and the inventory are replaced;
    /// a bag that was opened is replaced, one that was not keeps what was last seen in it (or is listed as not looked
    /// into), and a bag no longer in the inventory goes; the bank is replaced when it is open and kept otherwise.
    /// Says whether anything changed besides the times.
    pub fn update(&mut self, now: u64, worn: Vec<Item>, inventory: Vec<Item>, bags: Vec<Bag>, bank: Option<Vec<Item>>) -> bool {
        let before: Vec<(String, bool, Vec<Item>)> = self.sections.iter().map(|s| (s.key.clone(), s.looked, s.items.clone())).collect();
        let mut old: BTreeMap<String, Section> = std::mem::take(&mut self.sections).into_iter().map(|s| (s.key.clone(), s)).collect();
        let mut sections = vec![Section { key: "worn".into(), title: "Worn".into(), seen: now, looked: true, items: worn }, Section { key: "inventory".into(), title: "Inventory".into(), seen: now, looked: true, items: inventory }];
        for bag in bags {
            let key = format!("bag:{}:{}", bag.place, bag.template);
            sections.push(match (bag.contents, old.remove(&key)) {
                (Some(items), _) => Section { key, title: bag.name, seen: now, looked: true, items },
                (None, Some(kept)) => Section { title: bag.name, ..kept },
                (None, None) => Section { key, title: bag.name, seen: now, looked: false, items: Vec::new() },
            });
        }
        match (bank, old.remove("bank")) {
            (Some(items), _) => sections.push(Section { key: "bank".into(), title: "Bank".into(), seen: now, looked: true, items }),
            (None, Some(kept)) => sections.push(kept),
            (None, None) => {}
        }
        self.sections = sections;
        before != self.sections.iter().map(|s| (s.key.clone(), s.looked, s.items.clone())).collect::<Vec<_>>()
    }

    pub fn to_text(&self) -> String {
        let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
        let mut out = format!("mao-items\t1\nname\t{}\ndimension\t{}\n", clean(&self.name), clean(&self.dimension));
        for s in &self.sections {
            out += &format!("section\t{}\t{}\t{}\t{}\n", clean(&s.key), s.seen, u8::from(s.looked), clean(&s.title));
            for i in &s.items {
                out += &format!("item\t{}\t{}\t{}\t{}\t{}\n", i.low, i.high, i.quality, i.count, clean(&i.name));
            }
        }
        out
    }

    pub fn from_text(text: &str) -> Option<Character> {
        let mut lines = text.lines();
        if !lines.next()?.starts_with("mao-items\t") {
            return None;
        }
        let mut c = Character::default();
        for line in lines {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["name", name] => c.name = name.to_string(),
                ["dimension", d] => c.dimension = d.to_string(),
                ["section", key, seen, looked, title] => c.sections.push(Section { key: key.to_string(), title: title.to_string(), seen: seen.parse().ok()?, looked: *looked == "1", items: Vec::new() }),
                ["item", low, high, quality, count, name] => c.sections.last_mut()?.items.push(Item { low: low.parse().ok()?, high: high.parse().ok()?, quality: quality.parse().ok()?, count: count.parse().ok()?, name: name.to_string() }),
                _ => {}
            }
        }
        (!c.name.is_empty()).then_some(c)
    }

    /// The places the search cannot see into: bags never opened, and the bank when it was never seen.
    pub fn blind(&self) -> Vec<String> {
        let mut out: Vec<String> = self.sections.iter().filter(|s| !s.looked).map(|s| s.title.clone()).collect();
        if !self.sections.iter().any(|s| s.key == "bank") {
            out.push("the bank".to_string());
        }
        out
    }
}

/// Something found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub character: String,
    pub dimension: String,
    pub place: String,
    pub seen: u64,
    pub item: Item,
}

/// The things whose names hold every word of `query` (in any case), on any of the characters: those whose name begins
/// with the query first, then by name, quality and character. Nothing for a query with no word of two letters or more.
pub fn search(characters: &[Character], query: &str) -> Vec<Found> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if !words.iter().any(|w| w.chars().count() >= 2) {
        return Vec::new();
    }
    let whole = words.join(" ");
    let mut found: Vec<Found> = Vec::new();
    for c in characters {
        for s in &c.sections {
            for i in &s.items {
                let name = i.name.to_lowercase();
                if words.iter().all(|w| name.contains(w.as_str())) {
                    found.push(Found { character: c.name.clone(), dimension: c.dimension.clone(), place: s.title.clone(), seen: s.seen, item: i.clone() });
                }
            }
        }
    }
    found.sort_by(|a, b| {
        let starts = |f: &Found| !f.item.name.to_lowercase().starts_with(&whole);
        (starts(a), a.item.name.to_lowercase(), std::cmp::Reverse(a.item.quality), &a.character, &a.place).cmp(&(starts(b), b.item.name.to_lowercase(), std::cmp::Reverse(b.item.quality), &b.character, &b.place))
    });
    found
}

/// How long ago, in words: "just now", "5 minutes ago", "3 hours ago", "2 days ago".
pub fn ago(seen: u64, now: u64) -> String {
    let s = now.saturating_sub(seen);
    let plural = |n: u64, what: &str| format!("{n} {what}{} ago", if n == 1 { "" } else { "s" });
    match s {
        0..=89 => "just now".to_string(),
        90..=5399 => plural(s.div_ceil(60).max(2), "minute"),
        5400..=129_599 => plural((s + 1800) / 3600, "hour"),
        _ => plural((s + 43_200) / 86_400, "day"),
    }
}

/// The folder of the characters' files: `items` beside the settings.
pub fn folder() -> PathBuf {
    crate::settings::Settings::path().parent().map_or_else(|| PathBuf::from("items"), |d| d.join("items"))
}

/// A character's file: its name and the dimension's port, so that two dimensions' characters of one name have two.
pub fn file_name(name: &str, dimension: &str) -> String {
    let safe: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let port: String = dimension.rsplit(':').next().unwrap_or("").chars().filter(char::is_ascii_digit).collect();
    format!("{safe}.{}.txt", if port.is_empty() { "0" } else { &port })
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The characters known, read from the folder once and kept up to date by [`record`].
static KNOWN: Mutex<Option<Vec<Character>>> = Mutex::new(None);

fn known<R>(f: impl FnOnce(&mut Vec<Character>) -> R) -> R {
    let mut guard = KNOWN.lock().unwrap_or_else(|e| e.into_inner());
    let list = guard.get_or_insert_with(|| {
        let mut list: Vec<Character> = std::fs::read_dir(folder())
            .map(|dir| dir.filter_map(|e| e.ok()).filter_map(|e| std::fs::read_to_string(e.path()).ok()).filter_map(|t| Character::from_text(&t)).collect())
            .unwrap_or_default();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    });
    f(list)
}

/// The character played now (name and dimension), once its things were first written down.
static PLAYED: Mutex<Option<(String, String)>> = Mutex::new(None);
/// The window asked for the bags to be looked into.
static SCAN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Asks for the bags to be looked into (the window's button; a test script's step).
pub fn ask_scan() {
    SCAN.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the bags are to be looked into (asked once: the network thread's question).
pub fn take_scan() -> bool {
    SCAN.swap(false, std::sync::atomic::Ordering::Relaxed)
}

/// The bags of the character played that were never looked into.
pub fn unseen_bags() -> usize {
    let played = PLAYED.lock().unwrap_or_else(|e| e.into_inner()).clone();
    played.map_or(0, |(name, dimension)| known(|list| list.iter().find(|c| c.name == name && c.dimension == dimension).map_or(0, |c| c.sections.iter().filter(|s| !s.looked).count())))
}

/// The characters known, for the window.
pub fn all() -> Vec<Character> {
    known(|list| list.clone())
}

/// What the character played now carries (the network thread's call, when it changed): its file is brought up to it
/// and written when anything but the times is new.
pub fn record(name: &str, dimension: &str, worn: Vec<Item>, inventory: Vec<Item>, bags: Vec<Bag>, bank: Option<Vec<Item>>) {
    if name.is_empty() {
        return;
    }
    *PLAYED.lock().unwrap_or_else(|e| e.into_inner()) = Some((name.to_string(), dimension.to_string()));
    let text = known(|list| {
        let at = match list.iter().position(|c| c.name == name && c.dimension == dimension) {
            Some(at) => at,
            None => {
                list.push(Character { name: name.to_string(), dimension: dimension.to_string(), sections: Vec::new() });
                list.len() - 1
            }
        };
        list[at].update(now(), worn, inventory, bags, bank).then(|| list[at].to_text())
    });
    if let Some(text) = text {
        let dir = folder();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(file_name(name, dimension)), text);
    }
}

/// The window: a line to type in and what was found.
#[derive(Default)]
pub struct FindWindow {
    pub open: bool,
    pub query: String,
    /// The line takes the keys when the window opens.
    focus: bool,
    was_open: bool,
    /// Characters for a picture (instead of the files).
    pub preview: Option<Vec<Character>>,
}

impl FindWindow {
    /// Opens the window with something to look for.
    pub fn look_for(&mut self, query: &str) {
        self.query = query.trim().to_string();
        self.open = true;
    }

    /// Draws the window; gives back the info pages asked for (`itemref://` addresses).
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Vec<String> {
        let mut infos = Vec::new();
        if !self.open {
            self.was_open = false;
            return infos;
        }
        if !std::mem::replace(&mut self.was_open, true) {
            self.focus = true;
        }
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let text = egui::Color32::from_rgb(palette.text[0], palette.text[1], palette.text[2]);
        let characters = self.preview.clone().unwrap_or_else(all);
        let found = search(&characters, &self.query);
        let now = now();
        let mut is_open = self.open;
        crate::chrome::window(ctx, palette, "Find items", "Find items", &mut is_open, |w| w.resizable(true).default_size(egui::vec2(420.0, 320.0)).min_size(egui::vec2(300.0, 180.0)).default_pos(crate::layout::pos("Find items", egui::pos2(120.0, 160.0))), |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Find").color(text));
                let line = ui.add(egui::TextEdit::singleline(&mut self.query).desired_width(ui.available_width() - 8.0).hint_text("part of an item's name"));
                if std::mem::take(&mut self.focus) {
                    line.request_focus();
                }
            });
            let several_dimensions = characters.iter().map(|c| &c.dimension).collect::<std::collections::BTreeSet<_>>().len() > 1;
            let summary = if characters.is_empty() {
                "No character's things are known yet: they are written down as each character is played.".to_string()
            } else if self.query.trim().chars().count() < 2 {
                format!("{} character{} known: {}", characters.len(), if characters.len() == 1 { "" } else { "s" }, characters.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", "))
            } else {
                format!("{} found on {} character{}", found.len(), characters.len(), if characters.len() == 1 { "" } else { "s" })
            };
            ui.label(egui::RichText::new(summary).small().color(text));
            ui.separator();
            let notes: Vec<String> = characters.iter().filter_map(|c| Some((c, c.blind())).filter(|(_, b)| !b.is_empty())).map(|(c, blind)| format!("{}: {}", c.name, blind.join(", "))).collect();
            let room = (ui.available_height() - if notes.is_empty() { 4.0 } else { 46.0 }).max(60.0);
            crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("find-rows").max_height(room).auto_shrink([false, false]), |ui| {
                for f in &found {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::click());
                    let painter = ui.painter();
                    if response.hovered() {
                        painter.rect_filled(rect, 2.0, egui::Color32::from_white_alpha(18));
                    }
                    let count = if f.item.count > 1 { format!("  x{}", f.item.count) } else { String::new() };
                    painter.text(rect.left_top() + egui::vec2(4.0, 1.0), egui::Align2::LEFT_TOP, format!("{}  (QL {}){count}", f.item.name, f.item.quality), egui::FontId::proportional(12.5), accent);
                    let dimension = if several_dimensions { format!(" [{}]", f.dimension) } else { String::new() };
                    painter.text(rect.left_top() + egui::vec2(14.0, 15.0), egui::Align2::LEFT_TOP, format!("{}{dimension}: {}, seen {}", f.character, f.place, ago(f.seen, now)), egui::FontId::proportional(11.0), text);
                    if response.on_hover_text("Click for the item's description.").clicked() {
                        infos.push(format!("itemref://{}/{}/{}", f.item.low, f.item.high, f.item.quality));
                    }
                }
            });
            if !notes.is_empty() {
                ui.separator();
                ui.label(egui::RichText::new(format!("Not looked into (open them once on that character): {}", notes.join("; "))).small().color(text));
            }
            let unseen = if self.preview.is_some() { 0 } else { unseen_bags() };
            if unseen > 0 && crate::chrome::button(ui, palette, &format!("Look into my bags ({unseen})"), true).on_hover_text("Uses each bag of this character that was not looked into yet, one after the other; their windows stay shut.").clicked() {
                ask_scan();
            }
        });
        self.open = is_open;
        infos
    }
}

/// Characters for pictures and tests (made-up things), the first seen at `at` and the second three days before.
pub fn sample() -> Vec<Character> {
    sample_at(1_000_000)
}

pub fn sample_at(at: u64) -> Vec<Character> {
    let item = |low: u32, quality: u32, count: u32, name: &str| Item { low, high: low + 1, quality, count, name: name.to_string() };
    let mut a = Character { name: "Ballbearing".into(), dimension: "login.example.org:7505".into(), sections: Vec::new() };
    a.update(
        at,
        vec![item(10, 30, 1, "Plain Armor Sleeves")],
        vec![item(20, 12, 5, "Treatment Kit"), item(30, 44, 1, "Pillow with Important Stripes")],
        vec![Bag { place: 70, template: 99, name: "Backpack".into(), contents: Some(vec![item(40, 60, 1, "Solar-Powered Pistol"), item(20, 25, 3, "Treatment Kit")]) }, Bag { place: 71, template: 99, name: "Backpack".into(), contents: None }],
        None,
    );
    let mut b = Character { name: "Brasscasings".into(), dimension: "login.example.org:7505".into(), sections: Vec::new() };
    b.update(at - 3 * 86_400, vec![], vec![item(50, 100, 1, "Mission Key")], vec![], Some(vec![item(40, 120, 1, "Solar-Powered Pistol"), item(60, 1, 250, "Monster Parts")]));
    vec![a, b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thing_is_found_on_whoever_has_it() {
        let characters = sample();
        // (two pistols: the better first; on which character and where)
        let found = search(&characters, "pistol");
        let rows: Vec<(&str, &str, u32)> = found.iter().map(|f| (f.character.as_str(), f.place.as_str(), f.item.quality)).collect();
        assert_eq!(rows, vec![("Brasscasings", "Bank", 120), ("Ballbearing", "Backpack", 60)]);
        // (every word, in any order and case; the kits of the inventory and of the bag)
        assert_eq!(search(&characters, "KIT treat").len(), 2);
        assert_eq!(search(&characters, "kit pistol").len(), 0);
        // (a name that begins with what was typed comes before one that only holds it)
        let found = search(&characters, "pi");
        assert_eq!(found[0].item.name, "Pillow with Important Stripes");
        assert!(found.iter().any(|f| f.item.name == "Solar-Powered Pistol"));
        // (too little to go by)
        assert!(search(&characters, "p").is_empty());
        assert!(search(&characters, "  ").is_empty());
        // (what the search cannot see)
        assert_eq!(characters[0].blind(), vec!["Backpack".to_string(), "the bank".to_string()]);
        assert!(characters[1].blind().is_empty());
    }

    #[test]
    fn a_characters_file_keeps_what_was_seen() {
        let mut c = sample().remove(0);
        let text = c.to_text();
        assert_eq!(Character::from_text(&text), Some(c.clone()));
        assert_eq!(Character::from_text("something else"), None);
        let item = |low: u32, name: &str| Item { low, high: low, quality: 1, count: 1, name: name.to_string() };
        // (the opened bag shut again: its contents stay as they were seen; the other bag opened: now known; the bank
        // seen for the first time)
        let changed = c.update(
            2_000_000,
            vec![],
            vec![item(20, "Treatment Kit")],
            vec![Bag { place: 70, template: 99, name: "Backpack".into(), contents: None }, Bag { place: 71, template: 99, name: "Backpack".into(), contents: Some(vec![item(77, "Lockpick")]) }],
            Some(vec![item(88, "Old Coat")]),
        );
        assert!(changed);
        fn by_key(c: &Character, key: &str) -> Section {
            c.sections.iter().find(|s| s.key == key).unwrap().clone()
        }
        assert_eq!((by_key(&c, "bag:70:99").seen, by_key(&c, "bag:70:99").items.len(), by_key(&c, "bag:70:99").looked), (1_000_000, 2, true));
        assert_eq!((by_key(&c, "bag:71:99").seen, by_key(&c, "bag:71:99").items.len(), by_key(&c, "bag:71:99").looked), (2_000_000, 1, true));
        assert_eq!(by_key(&c, "bank").items[0].name, "Old Coat");
        assert_eq!(c.blind(), Vec::<String>::new());
        // (the same again is no change; the bank shut keeps what was in it; a bag gone from the inventory goes)
        assert!(!c.update(2_000_500, vec![], vec![item(20, "Treatment Kit")], vec![Bag { place: 70, template: 99, name: "Backpack".into(), contents: None }, Bag { place: 71, template: 99, name: "Backpack".into(), contents: None }], None));
        assert!(c.update(2_001_000, vec![], vec![item(20, "Treatment Kit")], vec![Bag { place: 71, template: 99, name: "Backpack".into(), contents: None }], None));
        assert!(c.sections.iter().all(|s| s.key != "bag:70:99"));
        assert_eq!(by_key(&c, "bank").seen, 2_000_000);
        // (a tab in a name does not break the file)
        c.sections[1].items[0].name = "Odd\tName".into();
        assert_eq!(Character::from_text(&c.to_text()).unwrap().sections[1].items[0].name, "Odd Name");
    }

    #[test]
    fn times_and_file_names() {
        assert_eq!(ago(1000, 1030), "just now");
        assert_eq!(ago(1000, 1000 + 300), "5 minutes ago");
        assert_eq!(ago(1000, 1000 + 3 * 3600), "3 hours ago");
        assert_eq!(ago(1000, 1000 + 2 * 86_400), "2 days ago");
        assert_eq!(ago(2000, 1000), "just now");
        assert_eq!(file_name("Ballbearing", "login.example.org:7505"), "Ballbearing.7505.txt");
        assert_eq!(file_name("Odd/..Name", ""), "OddName.0.txt");
    }
}
