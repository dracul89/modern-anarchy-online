//! The tradeskill guide: what the game itself says about making things, to look through and follow. This client's own
//! window (`/crafting`); the original has only the tradeskill window, which answers for one pair of items at a time.
//!
//! The guide's matter is `mao_formats::crafting`: the item descriptions of the player's own game, read when the window
//! is first opened (about two seconds, away from the frame). The game's files hold no table of the tradeskill
//! processes (the server has it), so the guide is what Funcom wrote on the items: the books and construction manuals
//! as pages to read, every item that speaks of making with the items its text names linked, the steps a text states
//! outright ("with this, that makes that"), the kin of a thing made in several steps, and the way back from a thing to
//! its parts as far as the texts say it.
//!
//! Two things are added from the player's own side:
//!
//! * where the player has a thing, from the item finder's files (`itemindex.rs`): on which character, in which place;
//! * what the server said in the tradeskill window: every result and every skill requirement it showed for a pair
//!   of items is written down ([`Book`], `tradeskills.txt` beside the settings), so the pairs the player has tried
//!   stand in the guide with their exact results, quality ranges and requirements. The server never tells the
//!   whole table, so this part grows only with use.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

use mao_formats::crafting::Guide;

use crate::theme::Palette;

/// A pair of items the tradeskill window was asked about, and what the server said.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tried {
    pub source: String,
    pub target: String,
    /// The result's name and its lowest and highest quality; empty when no result was shown.
    pub result: String,
    pub min: i32,
    pub max: i32,
    /// The requirements shown: the skill, the value needed, and the source's and the target's quality then.
    pub needs: Vec<(String, i32, i64, i64)>,
}

/// What the tradeskill window has taught: one entry a pair.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Book {
    pub tried: Vec<Tried>,
}

impl Book {
    fn pair(&mut self, source: &str, target: &str) -> &mut Tried {
        let at = match self.tried.iter().position(|t| t.source.eq_ignore_ascii_case(source) && t.target.eq_ignore_ascii_case(target)) {
            Some(at) => at,
            None => {
                self.tried.push(Tried { source: source.to_string(), target: target.to_string(), ..Default::default() });
                self.tried.len() - 1
            }
        };
        &mut self.tried[at]
    }

    /// The server showed a result for a pair; true when that is new.
    pub fn result(&mut self, source: &str, target: &str, result: &str, min: i32, max: i32) -> bool {
        if source.is_empty() || target.is_empty() || result.is_empty() {
            return false;
        }
        let t = self.pair(source, target);
        let new = (t.result.as_str(), t.min, t.max) != (result, min, max);
        (t.result, t.min, t.max) = (result.to_string(), min, max);
        new
    }

    /// The server showed a requirement for a pair at these qualities; true when that is new.
    pub fn need(&mut self, source: &str, source_quality: i64, target: &str, target_quality: i64, skill: &str, value: i32) -> bool {
        if source.is_empty() || target.is_empty() || skill.is_empty() {
            return false;
        }
        let t = self.pair(source, target);
        let need = (skill.to_string(), value, source_quality, target_quality);
        if t.needs.contains(&need) {
            return false;
        }
        // (one line a skill and pair of qualities: a new value for the same replaces the old)
        t.needs.retain(|n| !(n.0 == need.0 && n.2 == need.2 && n.3 == need.3));
        t.needs.push(need);
        true
    }

    pub fn to_text(&self) -> String {
        let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
        let mut out = String::from("mao-tradeskills\t1\n");
        for t in &self.tried {
            out += &format!("pair\t{}\t{}\t{}\t{}\t{}\n", clean(&t.source), clean(&t.target), clean(&t.result), t.min, t.max);
            for (skill, value, sq, tq) in &t.needs {
                out += &format!("need\t{}\t{value}\t{sq}\t{tq}\n", clean(skill));
            }
        }
        out
    }

    pub fn from_text(text: &str) -> Book {
        let mut book = Book::default();
        for line in text.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["pair", source, target, result, min, max] => book.tried.push(Tried { source: source.to_string(), target: target.to_string(), result: result.to_string(), min: min.parse().unwrap_or(0), max: max.parse().unwrap_or(0), needs: Vec::new() }),
                ["need", skill, value, sq, tq] => {
                    if let (Some(t), Ok(value), Ok(sq), Ok(tq)) = (book.tried.last_mut(), value.parse(), sq.parse(), tq.parse()) {
                        t.needs.push((skill.to_string(), value, sq, tq));
                    }
                }
                _ => {}
            }
        }
        book
    }

    /// The pairs a thing has a part in: as source, target or result.
    pub fn about(&self, name: &str) -> Vec<&Tried> {
        self.tried.iter().filter(|t| [&t.source, &t.target, &t.result].iter().any(|n| n.eq_ignore_ascii_case(name))).collect()
    }
}

/// A pair as a line: "A + B makes C (QL 10 to 50); needs 120 Mechanical Engineering (with QL 40 on QL 30)".
pub fn tried_line(t: &Tried) -> String {
    let mut line = format!("{} + {}", t.source, t.target);
    if !t.result.is_empty() {
        line += &format!(" makes {}", t.result);
        if t.max > 0 {
            line += &if t.min == t.max { format!(" (QL {})", t.min) } else { format!(" (QL {} to {})", t.min, t.max) };
        }
    }
    for (skill, value, sq, tq) in &t.needs {
        line += &format!("; needs {value} {skill} (with QL {sq} on QL {tq})");
    }
    line
}

fn book_path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("tradeskills.txt")
}

static BOOK: Mutex<Option<Book>> = Mutex::new(None);

fn with_book<R>(f: impl FnOnce(&mut Book) -> R) -> R {
    let mut guard = BOOK.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(|| std::fs::read_to_string(book_path()).map(|t| Book::from_text(&t)).unwrap_or_default()))
}

fn learned(change: impl FnOnce(&mut Book) -> bool) {
    let text = with_book(|book| change(book).then(|| book.to_text()));
    if let Some(text) = text {
        let _ = std::fs::write(book_path(), text);
    }
}

/// The tradeskill window got a result for the pair in its slots.
pub fn learned_result(source: &str, target: &str, result: &str, min: i32, max: i32) {
    learned(|book| book.result(source, target, result, min, max));
}

/// The tradeskill window got a requirement for the pair in its slots.
pub fn learned_need(source: &str, source_quality: i64, target: &str, target_quality: i64, skill: &str, value: i32) {
    learned(|book| book.need(source, source_quality, target, target_quality, skill, value));
}

/// The guide, read once: not asked for yet, being read, or there.
enum Reading {
    No,
    Now(Receiver<Arc<Guide>>),
    Done(Arc<Guide>),
}

static GUIDE: Mutex<Reading> = Mutex::new(Reading::No);

/// The guide when it has been read; the first call starts the reading.
pub fn guide() -> Option<Arc<Guide>> {
    let mut state = GUIDE.lock().unwrap_or_else(|e| e.into_inner());
    if let Reading::Now(rx) = &*state {
        if let Ok(guide) = rx.try_recv() {
            *state = Reading::Done(guide);
        }
    }
    match &*state {
        Reading::Done(guide) => Some(guide.clone()),
        Reading::Now(_) => None,
        Reading::No => {
            let (tx, rx) = channel();
            let game = crate::theme::game_dir();
            std::thread::spawn(move || {
                let started = std::time::Instant::now();
                let guide = game.and_then(|g| mao_formats::resource::ResourceDatabase::open_game(&g).ok()).map(|db| Guide::of_database(&db)).unwrap_or_default();
                eprintln!("tradeskill guide: {} entries, {} of them books, read in {:.1} s", guide.entries.len(), guide.entries.iter().filter(|e| e.guide).count(), started.elapsed().as_secs_f32());
                let _ = tx.send(Arc::new(guide));
            });
            *state = Reading::Now(rx);
            None
        }
    }
}

/// The window.
#[derive(Default)]
pub struct CraftWindow {
    pub open: bool,
    pub query: String,
    pub books_only: bool,
    /// The entry whose page is shown, by its name, and the ones shown before it.
    pub page: Option<String>,
    back: Vec<String>,
    focus: bool,
    was_open: bool,
    /// Where the player has things, by name in small letters (from the item finder), and when that was read.
    owned: HashMap<String, Vec<String>>,
    owned_at: Option<std::time::Instant>,
    /// A guide for a picture (instead of the one read in the background), with what the player has and has tried.
    pub preview: Option<(Arc<Guide>, Vec<crate::itemindex::Character>, Book)>,
}

/// Where each thing is, by its name in small letters: "Character: place".
pub fn owned_from(characters: &[crate::itemindex::Character]) -> HashMap<String, Vec<String>> {
    let mut owned: HashMap<String, Vec<String>> = HashMap::new();
    for c in characters {
        for s in &c.sections {
            for i in &s.items {
                let place = format!("{}: {}", c.name, s.title);
                let list = owned.entry(i.name.to_lowercase()).or_default();
                if !list.contains(&place) {
                    list.push(place);
                }
            }
        }
    }
    owned
}

impl CraftWindow {
    /// Opens the window with something to look for.
    pub fn look_for(&mut self, query: &str) {
        self.query = query.trim().to_string();
        self.page = None;
        self.open = true;
    }

    fn go(&mut self, name: &str) {
        if let Some(now) = self.page.replace(name.to_string()) {
            if now != name {
                self.back.push(now);
            }
        }
    }

    /// Draws the window; gives back the item pages asked for (`itemref://` addresses, for the info view).
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
        let have = egui::Color32::from_rgb(110, 220, 120);
        let weak = egui::Color32::from_gray(150);
        let guide = self.preview.as_ref().map(|p| p.0.clone()).or_else(guide);
        // (where the player has things: read again every few seconds, the files change as a character is played)
        if self.owned_at.is_none_or(|t| t.elapsed() > std::time::Duration::from_secs(5)) {
            self.owned = match &self.preview {
                Some(p) => owned_from(&p.1),
                None => owned_from(&crate::itemindex::all()),
            };
            self.owned_at = Some(std::time::Instant::now());
        }
        let book = match &self.preview {
            Some(p) => p.2.clone(),
            None => with_book(|b| b.clone()),
        };
        let mut is_open = self.open;
        let mut go: Option<String> = None;
        let mut step_back = false;
        crate::chrome::window(ctx, palette, "Tradeskill guide", "Tradeskill guide", &mut is_open, |w| w.resizable(true).default_size(egui::vec2(640.0, 460.0)).min_size(egui::vec2(420.0, 240.0)).default_pos(crate::layout::pos("Tradeskill guide", egui::pos2(140.0, 110.0))), |ui| {
            let Some(guide) = guide.as_ref() else {
                ui.label(egui::RichText::new("Reading what the game's items say about making things ...").italics().color(weak));
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
                return;
            };
            ui.horizontal(|ui| {
                if crate::chrome::button(ui, palette, "Back", !self.back.is_empty()).clicked() {
                    step_back = true;
                }
                ui.label(egui::RichText::new("Find").color(text));
                let line = ui.add(egui::TextEdit::singleline(&mut self.query).desired_width(220.0).hint_text("a thing, or what a book is about"));
                if std::mem::take(&mut self.focus) {
                    line.request_focus();
                }
                crate::chrome::checkbox(ui, palette, &mut self.books_only, "Books and manuals only");
            });
            ui.separator();
            let empty = self.query.trim().chars().count() < 2;
            let found = if empty { guide.search_in("", 2000, true) } else { guide.search_in(&self.query, 300, self.books_only) };
            let height = ui.available_height();
            ui.horizontal_top(|ui| {
                // The list: what was found, or the books when nothing is looked for.
                ui.allocate_ui_with_layout(egui::vec2(230.0, height), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.label(egui::RichText::new(if empty { format!("{} books and manuals of {} entries", found.len(), guide.entries.len()) } else { format!("{} found", found.len()) }).small().color(weak));
                    crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("craft-list").max_height(height - 20.0).auto_shrink([false, false]), |ui| {
                        for &at in &found {
                            let e = &guide.entries[at];
                            let shown = self.page.as_ref().is_some_and(|p| p.eq_ignore_ascii_case(&e.name));
                            let owned = self.owned.contains_key(&e.name.to_lowercase());
                            let colour = if shown { accent } else if owned { have } else { text };
                            let label = if e.guide && !empty { format!("{} (book)", e.name) } else { e.name.clone() };
                            if ui.add(egui::Label::new(egui::RichText::new(label).small().color(colour)).sense(egui::Sense::click()).truncate()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                go = Some(e.name.clone());
                            }
                        }
                    });
                });
                ui.separator();
                // The page of the entry chosen.
                ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), height), egui::Layout::top_down(egui::Align::Min), |ui| {
                    let Some(at) = self.page.as_ref().and_then(|p| guide.named(p)) else {
                        ui.add(egui::Label::new(egui::RichText::new("Choose a thing on the left. The guide is what the game's own items say about making things: its books and construction manuals, and every item whose description speaks of combining, with the things it names. The server keeps the full table of processes to itself, so what nobody wrote on an item is not here; the pairs you try in the tradeskill window are added as you try them.").color(weak)).wrap());
                        return;
                    };
                    let e = &guide.entries[at];
                    crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("craft-page").auto_shrink([false, false]), |ui| {
                        let mut link = |ui: &mut egui::Ui, k: usize| {
                            let name = &guide.entries[k].name;
                            let colour = if self.owned.contains_key(&name.to_lowercase()) { have } else { accent };
                            let response = ui.add(egui::Label::new(egui::RichText::new(name).small().underline().color(colour)).sense(egui::Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand);
                            let response = match self.owned.get(&name.to_lowercase()) {
                                Some(places) => response.on_hover_text(format!("You have it: {}", places.join("; "))),
                                None => response,
                            };
                            if response.clicked() {
                                go = Some(name.clone());
                            }
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(&e.name).strong().color(accent));
                            if ui.add(egui::Label::new(egui::RichText::new("the item").small().underline().color(weak)).sense(egui::Sense::click())).on_hover_text("The item's own window.").clicked() {
                                infos.push(format!("itemref://{}/{}/1", e.id, e.id));
                            }
                        });
                        if let Some(places) = self.owned.get(&e.name.to_lowercase()) {
                            ui.add(egui::Label::new(egui::RichText::new(format!("You have it: {}", places.join("; "))).small().color(have)).wrap());
                        }
                        if !e.text.is_empty() {
                            ui.add_space(4.0);
                            ui.add(egui::Label::new(egui::RichText::new(&e.text).color(text)).wrap());
                        }
                        let heading = |ui: &mut egui::Ui, words: &str| {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(words).small().strong().color(accent));
                        };
                        if !e.steps.is_empty() {
                            heading(ui, "What its text says outright");
                            for s in &e.steps {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new("with").small().color(weak));
                                    if s.with.is_empty() {
                                        ui.label(egui::RichText::new("(what its text describes)").small().color(weak));
                                    }
                                    for &w in &s.with {
                                        link(ui, w);
                                    }
                                    ui.label(egui::RichText::new("it makes").small().color(weak));
                                    link(ui, s.makes);
                                });
                            }
                        }
                        let books = guide.books_about(at);
                        if !books.is_empty() {
                            heading(ui, "Books and manuals that speak of it or of its kind");
                            ui.horizontal_wrapped(|ui| {
                                for &k in books.iter().take(12) {
                                    link(ui, k);
                                }
                                if books.len() > 12 {
                                    ui.label(egui::RichText::new(format!("and {} more", books.len() - 12)).small().color(weak));
                                }
                            });
                        }
                        let way = guide.way_to(at, 5);
                        if !way.is_empty() {
                            heading(ui, "The way to it, as far as the texts say");
                            for (level, made, by, step) in way {
                                ui.horizontal_wrapped(|ui| {
                                    ui.add_space(14.0 * level as f32);
                                    link(ui, made);
                                    ui.label(egui::RichText::new("from").small().color(weak));
                                    link(ui, by);
                                    for &w in &guide.entries[by].steps[step].with {
                                        ui.label(egui::RichText::new("and").small().color(weak));
                                        link(ui, w);
                                    }
                                });
                            }
                        }
                        let tried = book.about(&e.name);
                        if !tried.is_empty() {
                            heading(ui, "What the tradeskill window told you");
                            for t in tried {
                                ui.add(egui::Label::new(egui::RichText::new(tried_line(t)).small().color(text)).wrap());
                            }
                        }
                        let kin = guide.kin(at);
                        if !kin.is_empty() {
                            heading(ui, "Of the same making (the same name in brackets)");
                            ui.horizontal_wrapped(|ui| {
                                for k in kin {
                                    link(ui, k);
                                }
                            });
                        }
                        if !e.names.is_empty() {
                            heading(ui, "Its text names");
                            ui.horizontal_wrapped(|ui| {
                                for &k in &e.names {
                                    link(ui, k);
                                }
                            });
                        }
                        let named_by = &guide.named_by[at];
                        if !named_by.is_empty() {
                            heading(ui, &format!("Named in the text of {} other{}", named_by.len(), if named_by.len() == 1 { "" } else { "s" }));
                            ui.horizontal_wrapped(|ui| {
                                for &k in named_by.iter().take(60) {
                                    link(ui, k);
                                }
                                if named_by.len() > 60 {
                                    ui.label(egui::RichText::new(format!("and {} more", named_by.len() - 60)).small().color(weak));
                                }
                            });
                        }
                    });
                });
            });
        });
        self.open = is_open;
        if step_back {
            self.page = self.back.pop();
        }
        if let Some(name) = go {
            self.go(&name);
        }
        infos
    }
}

/// A guide, what the player has and has tried, for pictures and tests (made-up things).
pub fn sample() -> (Arc<Guide>, Vec<crate::itemindex::Character>, Book) {
    let item = |id: u32, name: &str, text: &str| (id, name.to_string(), text.to_string());
    let guide = Guide::build(vec![
        item(10, "Blank Song Sheet", "Paper with lines. If combined with a Tuning Fork, it will make a Written Melody. This is a tradeskill item."),
        item(20, "Tuning Fork", "It hums."),
        item(30, "Written Melody", "A melody on paper. Use this on a Music Box Frame to create a Singing Music Box. This is a tradeskill item."),
        item(40, "Music Box Frame", "An empty box of brass."),
        item(50, "Singing Music Box", "It sings when opened."),
        item(60, "Brass Polish", "Can be used on a Music Box Frame. It smells of lemons."),
        item(120, "Book: How To Build Music Boxes", &format!("First combine the Blank Song Sheet with a Tuning Fork; the fork must be of at least half the sheet's quality. This takes a steady hand.<br><br>Then use the Written Melody on a Music Box Frame.<br><br>{}", "Polish the brass before you assemble it. ".repeat(12))),
    ]);
    let mut owner = crate::itemindex::Character { name: "Ballbearing".into(), dimension: "login.example.org:7505".into(), sections: Vec::new() };
    let thing = |low: u32, name: &str| crate::itemindex::Item { low, high: low, quality: 20, count: 1, name: name.to_string() };
    owner.update(1_000_000, vec![], vec![thing(20, "Tuning Fork")], vec![crate::itemindex::Bag { place: 70, template: 99, name: "Backpack".into(), contents: Some(vec![thing(40, "Music Box Frame")]) }], None);
    let mut book = Book::default();
    book.result("Blank Song Sheet", "Tuning Fork", "Written Melody", 10, 40);
    book.need("Blank Song Sheet", 40, "Tuning Fork", 20, "Mechanical Engineering", 180);
    (Arc::new(guide), vec![owner], book)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_the_tradeskill_window_says_is_written_down() {
        let mut book = Book::default();
        assert!(book.result("Blank Song Sheet", "Tuning Fork", "Written Melody", 10, 40));
        // (the same again is nothing new; another range is)
        assert!(!book.result("blank song sheet", "TUNING FORK", "Written Melody", 10, 40));
        assert!(book.result("Blank Song Sheet", "Tuning Fork", "Written Melody", 10, 60));
        assert!(book.need("Blank Song Sheet", 40, "Tuning Fork", 20, "Mechanical Engineering", 180));
        assert!(!book.need("Blank Song Sheet", 40, "Tuning Fork", 20, "Mechanical Engineering", 180));
        // (a requirement before any result: the pair is there without one)
        assert!(book.need("Written Melody", 30, "Music Box Frame", 30, "Electrical Engineering", 90));
        assert!(!book.result("", "Tuning Fork", "Written Melody", 1, 1));
        assert_eq!(book.tried.len(), 2);
        assert_eq!(tried_line(&book.tried[0]), "Blank Song Sheet + Tuning Fork makes Written Melody (QL 10 to 60); needs 180 Mechanical Engineering (with QL 40 on QL 20)");
        assert_eq!(tried_line(&book.tried[1]), "Written Melody + Music Box Frame; needs 90 Electrical Engineering (with QL 30 on QL 30)");
        // (as a file and back)
        assert_eq!(Book::from_text(&book.to_text()), book);
        assert_eq!(Book::from_text("nothing of the kind"), Book::default());
        // (what a thing has a part in: as a part or as the result)
        assert_eq!(book.about("written melody").len(), 2);
        assert_eq!(book.about("Tuning Fork").len(), 1);
        assert!(book.about("Garden Chair").is_empty());
    }

    /// `MAO_GAME`: the game's own items give a guide: its book on nano crystals, a nano program's chain by the names
    /// in brackets, and the step the instruction disc's text states.
    #[test]
    fn the_games_items_give_a_guide() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let guide = Guide::of_database(&db);
        assert!(guide.entries.len() > 10_000, "{}", guide.entries.len());
        let book = guide.named("Book: How To Create A Nano Crystal").expect("the book");
        assert!(guide.entries[book].guide && guide.entries[book].text.lines().count() > 3);
        assert!(guide.entries.iter().filter(|e| e.guide).count() > 200);
        let disc = guide.named("Instruction Disc (Grid Gateway)").expect("the disc");
        let (library, algorithm, crystal) = (guide.named("Symbol Library - Space").unwrap(), guide.named("Compiled Algorithm (Grid Gateway)").unwrap(), guide.named("Nano Crystal (Grid Gateway)").unwrap());
        assert_eq!(guide.entries[disc].steps.len(), 1);
        assert_eq!((guide.entries[disc].steps[0].with.clone(), guide.entries[disc].steps[0].makes), (vec![library], algorithm));
        assert!(guide.kin(disc).contains(&crystal) && guide.kin(disc).contains(&algorithm));
        assert!(guide.books_about(disc).contains(&book));
        assert_eq!(guide.search("how to create a nano crystal", 5).first(), Some(&book));
    }

    #[test]
    fn the_guide_knows_where_the_player_has_things() {
        let (guide, characters, book) = sample();
        let owned = owned_from(&characters);
        assert_eq!(owned.get("tuning fork"), Some(&vec!["Ballbearing: Inventory".to_string()]));
        assert_eq!(owned.get("music box frame"), Some(&vec!["Ballbearing: Backpack".to_string()]));
        assert!(!owned.contains_key("written melody"));
        // (the sample's book is a guide, and the way to the box goes through the melody)
        let at = guide.named("Singing Music Box").unwrap();
        assert_eq!(guide.way_to(at, 5).len(), 2);
        assert!(guide.entries[guide.named("Book: How To Build Music Boxes").unwrap()].guide);
        assert_eq!(book.about("Written Melody").len(), 1);
    }
}
