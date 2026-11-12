//! The damage meter: who did how much damage, to whom, with what. This client's own (the original has no such window;
//! players used programs that read the chat log's combat lines).
//!
//! It is fed by the fight's own messages as the client reads them (`net.rs`): a weapon's hit (`AttackInfo`: the amount,
//! and a critical or glancing hit by its hit type 4 or 2), a special attack's (`SpecialAttackInfo`, named by its skill),
//! a miss (`MissedAttackInfo`), a loss or gain of health with its source (`HealthDamage`: a nano program's damage, a
//! heal), and a damage or reflect shield's hit. The server tells these of everyone near, so the meter counts other
//! players, their pets and the creatures as well as the client's own character. What it cannot see is not counted: a
//! loss of health told only as a new health value has no source and is left out.
//!
//! The events fall into fights: a fight begins with a hit and ends when nothing has been hit for [`GAP`] seconds. A
//! row's damage per second is its damage over the fight's length (first to last event, at least a second), so a
//! player who joins late or stops early has the lower figure, as in a raid's parse.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use crate::theme::Palette;

/// The seconds without a hit after which the next one begins a new fight.
pub const GAP: f64 = 12.0;
/// The fights kept (the oldest goes first).
pub const KEPT: usize = 40;

/// How a blow landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Normal,
    Critical,
    Glancing,
    Miss,
}

impl Hit {
    /// `AttackInfo`'s hit type: 4 a critical hit, 2 a glancing one (the lines `Feedback_CriticalHit`, `Feedback_GlancingHit`).
    pub fn of_type(hit_type: i64) -> Hit {
        match hit_type {
            4 => Hit::Critical,
            2 => Hit::Glancing,
            _ => Hit::Normal,
        }
    }
}

/// Someone in a fight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    pub id: u32,
    pub name: String,
    pub player: bool,
}

/// One thing that happened.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    /// Who did it; none when the client does not know.
    pub attacker: Option<Actor>,
    pub victim: Actor,
    /// Points of damage, or of healing when `heal`; nothing for a miss.
    pub amount: i64,
    pub heal: bool,
    pub hit: Hit,
    /// What it was done with: "Weapon", a special attack's skill, a nano program's name, "Damage shield".
    pub source: String,
}

/// A count of blows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tally {
    pub total: i64,
    pub hits: u32,
    pub criticals: u32,
    pub glancing: u32,
    pub misses: u32,
    pub largest: i64,
}

impl Tally {
    fn add(&mut self, amount: i64, hit: Hit) {
        match hit {
            Hit::Miss => self.misses += 1,
            _ => {
                self.total += amount;
                self.hits += 1;
                self.largest = self.largest.max(amount);
                self.criticals += u32::from(hit == Hit::Critical);
                self.glancing += u32::from(hit == Hit::Glancing);
            }
        }
    }

    fn join(&mut self, other: &Tally) {
        self.total += other.total;
        self.hits += other.hits;
        self.criticals += other.criticals;
        self.glancing += other.glancing;
        self.misses += other.misses;
        self.largest = self.largest.max(other.largest);
    }

    /// The share of the swings that landed, in percent (none without a swing).
    pub fn accuracy(&self) -> Option<f32> {
        let swings = self.hits + self.misses;
        (swings > 0).then(|| 100.0 * self.hits as f32 / swings as f32)
    }

    /// The share of the hits that were critical, in percent.
    pub fn critical_share(&self) -> Option<f32> {
        (self.hits > 0).then(|| 100.0 * self.criticals as f32 / self.hits as f32)
    }

    pub fn average(&self) -> i64 {
        if self.hits == 0 { 0 } else { self.total / self.hits as i64 }
    }
}

/// What one actor did and took in a fight.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Side {
    pub name: String,
    pub player: bool,
    pub dealt: Tally,
    pub taken: Tally,
    pub healed: i64,
    /// The damage dealt by what it was done with.
    pub sources: BTreeMap<String, Tally>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fight {
    pub started: f64,
    pub last: f64,
    pub sides: BTreeMap<u32, Side>,
    /// What the fight is called: the first one hit that is no player, else the first one hit.
    pub label: String,
    labelled_by_creature: bool,
}

impl Fight {
    /// The fight's length in seconds, at least one.
    pub fn seconds(&self) -> f64 {
        (self.last - self.started).max(1.0)
    }
}

/// What the rows count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Dealt,
    Taken,
    Healed,
}

/// Which fights the rows are of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    /// The fight going on, or the last one.
    #[default]
    Latest,
    /// A fight by its place among those kept (0 the oldest).
    Fight(usize),
    /// All the fights kept, together.
    All,
}

/// A line of the window.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: u32,
    pub name: String,
    pub player: bool,
    pub value: i64,
    pub per_second: f64,
    /// Of all the rows' values, in percent.
    pub share: f32,
    pub tally: Tally,
    pub sources: Vec<(String, Tally)>,
}

#[derive(Default)]
pub struct Meter {
    pub fights: VecDeque<Fight>,
    /// The pets whose owners are known: the pet, and its owner with the owner's name.
    pub pets: BTreeMap<u32, (u32, String)>,
}

impl Meter {
    /// Adds what happened at `at` (seconds on any clock that only goes forward).
    pub fn add(&mut self, at: f64, event: &Event) {
        let running = self.fights.back().is_some_and(|f| at - f.last <= GAP);
        if !running {
            // (a heal with no fight going on is no fight)
            if event.heal {
                return;
            }
            if self.fights.len() >= KEPT {
                self.fights.pop_front();
            }
            self.fights.push_back(Fight { started: at, last: at, ..Default::default() });
        }
        let fight = self.fights.back_mut().expect("a fight");
        if !event.heal {
            fight.last = fight.last.max(at);
            if fight.label.is_empty() || (!fight.labelled_by_creature && !event.victim.player) {
                fight.label = event.victim.name.clone();
                fight.labelled_by_creature = !event.victim.player;
            }
        }
        let side = |fight: &mut Fight, actor: &Actor| {
            let s = fight.sides.entry(actor.id).or_default();
            if s.name.is_empty() {
                s.name = actor.name.clone();
                s.player = actor.player;
            }
        };
        side(fight, &event.victim);
        if let Some(a) = &event.attacker {
            side(fight, a);
        }
        if event.heal {
            if let Some(a) = &event.attacker {
                fight.sides.get_mut(&a.id).expect("the side").healed += event.amount;
            }
            return;
        }
        fight.sides.get_mut(&event.victim.id).expect("the side").taken.add(event.amount, event.hit);
        if let Some(a) = &event.attacker {
            let s = fight.sides.get_mut(&a.id).expect("the side");
            s.dealt.add(event.amount, event.hit);
            s.sources.entry(event.source.clone()).or_default().add(event.amount, event.hit);
        }
    }

    /// The fights of a view and their length together.
    fn chosen(&self, view: View) -> (Vec<&Fight>, f64) {
        let fights: Vec<&Fight> = match view {
            View::Latest => self.fights.back().into_iter().collect(),
            View::Fight(k) => self.fights.get(k).into_iter().collect(),
            View::All => self.fights.iter().collect(),
        };
        let seconds = fights.iter().map(|f| f.seconds()).sum::<f64>().max(1.0);
        (fights, seconds)
    }

    /// The seconds a view's fights lasted, and what they are called.
    pub fn summary(&self, view: View) -> (f64, String) {
        let (fights, seconds) = self.chosen(view);
        let label = match (view, fights.as_slice()) {
            (View::All, _) => format!("{} fights", fights.len()),
            (_, [f]) => f.label.clone(),
            _ => String::new(),
        };
        (if fights.is_empty() { 0.0 } else { seconds }, label)
    }

    /// The rows of a view, the largest first. With `merge_pets` a pet's figures are its owner's, each of the pet's
    /// sources named for the pet; with `players_only` only the players' rows (and their pets', merged or not).
    pub fn rows(&self, view: View, mode: Mode, merge_pets: bool, players_only: bool) -> Vec<Row> {
        let (fights, seconds) = self.chosen(view);
        let mut sides: BTreeMap<u32, Side> = BTreeMap::new();
        for fight in fights {
            for (&id, side) in &fight.sides {
                let owner = self.pets.get(&id);
                let (into, pet) = match owner {
                    Some((owner, _)) if merge_pets => (*owner, true),
                    _ => (id, false),
                };
                let row = sides.entry(into).or_default();
                if pet {
                    if row.name.is_empty() {
                        row.name = owner.map(|o| o.1.clone()).unwrap_or_default();
                        row.player = true;
                    }
                } else {
                    row.name = side.name.clone();
                    row.player = side.player || owner.is_some();
                }
                row.dealt.join(&side.dealt);
                row.taken.join(&side.taken);
                row.healed += side.healed;
                for (source, tally) in &side.sources {
                    let name = if pet { format!("{}: {source}", side.name) } else { source.clone() };
                    row.sources.entry(name).or_default().join(tally);
                }
            }
        }
        let mut rows: Vec<Row> = sides
            .into_iter()
            .filter(|(_, s)| !players_only || s.player)
            .map(|(id, s)| {
                let (value, tally) = match mode {
                    Mode::Dealt => (s.dealt.total, s.dealt),
                    Mode::Taken => (s.taken.total, s.taken),
                    Mode::Healed => (s.healed, Tally::default()),
                };
                let mut sources: Vec<(String, Tally)> = if mode == Mode::Dealt { s.sources.into_iter().collect() } else { Vec::new() };
                sources.sort_by(|a, b| b.1.total.cmp(&a.1.total).then_with(|| a.0.cmp(&b.0)));
                Row { id, name: s.name, player: s.player, value, per_second: value as f64 / seconds, share: 0.0, tally, sources }
            })
            .filter(|r| r.value > 0 || (mode != Mode::Healed && r.tally.misses > 0))
            .collect();
        let all: i64 = rows.iter().map(|r| r.value).sum();
        for r in &mut rows {
            r.share = if all > 0 { 100.0 * r.value as f32 / all as f32 } else { 0.0 };
        }
        rows.sort_by(|a, b| b.value.cmp(&a.value).then_with(|| a.name.cmp(&b.name)));
        rows
    }

    /// The rows as lines of text, to paste somewhere: a heading and "1. Name 1,234 (56.7/s, 40%)".
    pub fn report(&self, view: View, mode: Mode, merge_pets: bool, players_only: bool) -> String {
        let (seconds, label) = self.summary(view);
        let what = match mode {
            Mode::Dealt => "Damage",
            Mode::Taken => "Damage taken",
            Mode::Healed => "Healing",
        };
        let mut out = format!("{what}: {label} ({})\n", clock(seconds));
        for (k, r) in self.rows(view, mode, merge_pets, players_only).iter().enumerate() {
            out += &format!("{}. {} {} ({:.1}/s, {:.0}%)\n", k + 1, r.name, grouped(r.value), r.per_second, r.share);
        }
        out
    }
}

/// A number with its thousands apart: 1,234,567.
pub fn grouped(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (k, c) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// Seconds as m:ss.
pub fn clock(seconds: f64) -> String {
    let s = seconds.round().max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

static METER: Mutex<Option<Meter>> = Mutex::new(None);
static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// The meter all the threads share.
pub fn with<R>(f: impl FnOnce(&mut Meter) -> R) -> R {
    f(METER.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Meter::default))
}

/// Something happened now (the network thread's call).
pub fn record(event: Event) {
    let at = STARTED.get_or_init(std::time::Instant::now).elapsed().as_secs_f64();
    with(|m| m.add(at, &event));
}

static OWN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The client's own character (its row is marked).
pub fn set_own(id: u32) {
    OWN.store(id, std::sync::atomic::Ordering::Relaxed);
}

pub fn own() -> u32 {
    OWN.load(std::sync::atomic::Ordering::Relaxed)
}

/// A pet and its owner, as the client learns of them.
pub fn pet(pet: u32, owner: u32, owner_name: &str) {
    with(|m| {
        m.pets.insert(pet, (owner, owner_name.to_string()));
    });
}

/// The window.
pub struct DamageWindow {
    pub open: bool,
    view: View,
    mode: Mode,
    merge_pets: bool,
    players_only: bool,
    /// The row whose sources are shown.
    selected: Option<u32>,
    /// What the Copy button put on the clipboard last (for the tests).
    pub copied: String,
}

impl Default for DamageWindow {
    fn default() -> Self {
        DamageWindow { open: false, view: View::Latest, mode: Mode::Dealt, merge_pets: true, players_only: false, selected: None, copied: String::new() }
    }
}

impl DamageWindow {
    /// A row's sources shown (a preview's).
    pub fn select(&mut self, row: usize) {
        self.selected = with(|m| m.rows(self.view, self.mode, self.merge_pets, self.players_only).get(row).map(|r| r.id));
    }

    /// Draws the window; the client's own character's row is marked ([`set_own`]).
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) {
        if !self.open {
            return;
        }
        let own = own();
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let text = egui::Color32::from_rgb(palette.text[0], palette.text[1], palette.text[2]);
        let (rows, (seconds, label), fights) = with(|m| {
            let fights: Vec<(usize, String)> = m.fights.iter().enumerate().map(|(k, f)| (k, format!("{} ({})", if f.label.is_empty() { "?" } else { &f.label }, clock(f.seconds())))).collect();
            (m.rows(self.view, self.mode, self.merge_pets, self.players_only), m.summary(self.view), fights)
        });
        let mut is_open = self.open;
        let (mut reset, mut copy) = (false, false);
        crate::chrome::window(ctx, palette, "Damage", "Damage", &mut is_open, |w| w.resizable(true).default_size(egui::vec2(330.0, 300.0)).min_size(egui::vec2(260.0, 160.0)).default_pos(crate::layout::pos("Damage", egui::pos2(80.0, 200.0))), |ui| {
            ui.horizontal(|ui| {
                let shown = match self.view {
                    View::Latest => "Latest fight".to_string(),
                    View::All => "All fights".to_string(),
                    View::Fight(k) => fights.iter().find(|f| f.0 == k).map_or_else(|| "A fight".to_string(), |f| f.1.clone()),
                };
                egui::ComboBox::from_id_salt("damage-view").selected_text(shown).width(150.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.view, View::Latest, "Latest fight");
                    ui.selectable_value(&mut self.view, View::All, "All fights");
                    for (k, name) in fights.iter().rev() {
                        ui.selectable_value(&mut self.view, View::Fight(*k), name);
                    }
                });
                egui::ComboBox::from_id_salt("damage-mode")
                    .selected_text(match self.mode {
                        Mode::Dealt => "Damage done",
                        Mode::Taken => "Damage taken",
                        Mode::Healed => "Healing done",
                    })
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.mode, Mode::Dealt, "Damage done");
                        ui.selectable_value(&mut self.mode, Mode::Taken, "Damage taken");
                        ui.selectable_value(&mut self.mode, Mode::Healed, "Healing done");
                    });
            });
            ui.horizontal(|ui| {
                crate::chrome::checkbox(ui, palette, &mut self.players_only, "Players only");
                crate::chrome::checkbox(ui, palette, &mut self.merge_pets, "Pets with their owners");
            });
            let total: i64 = rows.iter().map(|r| r.value).sum();
            ui.label(egui::RichText::new(if seconds > 0.0 { format!("{label}   {}   {} in all, {:.1} a second", clock(seconds), grouped(total), total as f64 / seconds) } else { "Nothing has been hit yet.".to_string() }).small().color(text));
            ui.separator();
            let detail_room = if self.selected.is_some_and(|s| rows.iter().any(|r| r.id == s && !r.sources.is_empty())) { 110.0 } else { 0.0 };
            let room = (ui.available_height() - 30.0 - detail_room).max(60.0);
            crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("damage-rows").max_height(room).auto_shrink([false, false]), |ui| {
                let largest = rows.first().map_or(1, |r| r.value.max(1));
                for (k, r) in rows.iter().enumerate() {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::click());
                    let painter = ui.painter();
                    // (the bar: the row's value against the largest; one's own row in the accent)
                    let bar = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * (r.value as f32 / largest as f32).clamp(0.0, 1.0), rect.height() - 2.0));
                    let fill = if r.id == own { accent.gamma_multiply(0.55) } else if r.player { egui::Color32::from_rgba_unmultiplied(70, 110, 170, 150) } else { egui::Color32::from_rgba_unmultiplied(150, 70, 60, 150) };
                    painter.rect_filled(bar, 2.0, fill);
                    if self.selected == Some(r.id) {
                        painter.rect_stroke(rect.shrink2(egui::vec2(0.0, 1.0)), 2.0, egui::Stroke::new(1.0, accent), egui::StrokeKind::Inside);
                    }
                    let font = egui::FontId::proportional(12.0);
                    painter.text(rect.left_center() + egui::vec2(4.0, -1.0), egui::Align2::LEFT_CENTER, format!("{}. {}", k + 1, r.name), font.clone(), text);
                    painter.text(rect.right_center() + egui::vec2(-4.0, -1.0), egui::Align2::RIGHT_CENTER, format!("{}  ({:.1}/s)  {:.0}%", grouped(r.value), r.per_second, r.share), font, text);
                    if response.clicked() {
                        self.selected = if self.selected == Some(r.id) { None } else { Some(r.id) };
                    }
                    if self.mode != Mode::Healed {
                        response.on_hover_text(format!("{} hits, {} missed{}; the largest {}, {} on average{}", r.tally.hits, r.tally.misses, r.tally.accuracy().map_or(String::new(), |a| format!(" ({a:.0}% landed)")), grouped(r.tally.largest), grouped(r.tally.average()), r.tally.critical_share().map_or(String::new(), |c| format!("; {c:.0}% critical"))));
                    }
                }
            });
            if let Some(r) = self.selected.and_then(|s| rows.iter().find(|r| r.id == s)).filter(|r| !r.sources.is_empty()) {
                ui.separator();
                ui.label(egui::RichText::new(format!("{}: what the damage was done with", r.name)).small().color(accent));
                crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("damage-sources").max_height(80.0).auto_shrink([false, false]), |ui| {
                    egui::Grid::new("damage-source-grid").num_columns(5).spacing(egui::vec2(10.0, 2.0)).show(ui, |ui| {
                        for (name, t) in &r.sources {
                            let small = |s: String| egui::RichText::new(s).small().color(text);
                            ui.label(small(name.clone()));
                            ui.label(small(grouped(t.total)));
                            ui.label(small(format!("{:.0}%", if r.value > 0 { 100.0 * t.total as f32 / r.value as f32 } else { 0.0 })));
                            ui.label(small(format!("{} hits, {} missed", t.hits, t.misses)));
                            ui.label(small(format!("{} largest{}", grouped(t.largest), t.critical_share().filter(|c| *c > 0.0).map_or(String::new(), |c| format!(", {c:.0}% critical")))));
                            ui.end_row();
                        }
                    });
                });
            }
            ui.horizontal(|ui| {
                reset = crate::chrome::button(ui, palette, "Reset", true).clicked();
                copy = crate::chrome::button(ui, palette, "Copy", !rows.is_empty()).on_hover_text("The rows as text, to paste into a chat line or a note.").clicked();
            });
        });
        self.open = is_open;
        if reset {
            with(|m| m.fights.clear());
            self.view = View::Latest;
            self.selected = None;
        }
        if copy {
            self.copied = with(|m| m.report(self.view, self.mode, self.merge_pets, self.players_only));
            ctx.copy_text(self.copied.clone());
        }
        // (a fight that is no longer kept: back to the latest)
        if let View::Fight(k) = self.view {
            if !fights.iter().any(|f| f.0 == k) {
                self.view = View::Latest;
            }
        }
        // (the figures move while a fight goes on)
        if seconds > 0.0 {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}

/// A fight for pictures and tests: two players, one with a pet, and a creature.
pub fn sample() -> Meter {
    let mut m = Meter::default();
    let (you, other, pet, rat) = (Actor { id: 1, name: "Ballbearing".into(), player: true }, Actor { id: 2, name: "Brasscasings".into(), player: true }, Actor { id: 3, name: "Automaton".into(), player: false }, Actor { id: 9, name: "Rollerrat".into(), player: false });
    m.pets.insert(3, (1, "Ballbearing".into()));
    let hit = |a: &Actor, v: &Actor, amount: i64, hit: Hit, source: &str| Event { attacker: Some(a.clone()), victim: v.clone(), amount, heal: false, hit, source: source.into() };
    m.add(0.0, &hit(&you, &rat, 120, Hit::Normal, "Weapon"));
    m.add(1.0, &hit(&other, &rat, 310, Hit::Critical, "Weapon"));
    m.add(1.5, &hit(&pet, &rat, 45, Hit::Normal, "Weapon"));
    m.add(2.0, &hit(&you, &rat, 0, Hit::Miss, "Weapon"));
    m.add(3.0, &hit(&other, &rat, 480, Hit::Normal, "Burst"));
    m.add(4.0, &hit(&rat, &you, 37, Hit::Glancing, "Weapon"));
    m.add(5.0, &hit(&you, &rat, 200, Hit::Normal, "Electrical Discharge"));
    m.add(6.0, &Event { attacker: Some(other.clone()), victim: you.clone(), amount: 150, heal: true, hit: Hit::Normal, source: "Lifeblood".into() });
    m.add(10.0, &hit(&pet, &rat, 55, Hit::Normal, "Weapon"));
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fight_is_counted_by_who_did_what() {
        let m = sample();
        assert_eq!(m.fights.len(), 1);
        assert_eq!(m.summary(View::Latest), (10.0, "Rollerrat".to_string()));
        // (the pet's with its owner: 120 + 200 and the pet's 45 + 55)
        let rows = m.rows(View::Latest, Mode::Dealt, true, false);
        let names: Vec<(&str, i64)> = rows.iter().map(|r| (r.name.as_str(), r.value)).collect();
        assert_eq!(names, vec![("Brasscasings", 790), ("Ballbearing", 420), ("Rollerrat", 37)]);
        assert!((rows[0].per_second - 79.0).abs() < 1e-9);
        assert!((rows[0].share - 100.0 * 790.0 / 1247.0).abs() < 1e-3);
        // (a miss is a swing, not a hit; the critical one counted)
        let own = &rows[1];
        assert_eq!((own.tally.hits, own.tally.misses, own.tally.largest), (4, 1, 200));
        assert_eq!(rows[0].tally.criticals, 1);
        assert_eq!(rows[0].tally.critical_share(), Some(50.0));
        assert_eq!(own.tally.accuracy(), Some(80.0));
        // (what it was done with, the largest first; the pet's named for the pet)
        let sources: Vec<(&str, i64)> = own.sources.iter().map(|s| (s.0.as_str(), s.1.total)).collect();
        assert_eq!(sources, vec![("Electrical Discharge", 200), ("Weapon", 120), ("Automaton: Weapon", 100)]);
        // (apart: the pet has its own row, and counts among the players')
        let apart = m.rows(View::Latest, Mode::Dealt, false, true);
        assert_eq!(apart.iter().map(|r| (r.name.as_str(), r.value)).collect::<Vec<_>>(), vec![("Brasscasings", 790), ("Ballbearing", 320), ("Automaton", 100)]);
        // (taken and healed)
        let taken = m.rows(View::Latest, Mode::Taken, true, false);
        assert_eq!(taken.iter().map(|r| (r.name.as_str(), r.value)).collect::<Vec<_>>(), vec![("Rollerrat", 1210), ("Ballbearing", 37)]);
        let healed = m.rows(View::Latest, Mode::Healed, true, false);
        assert_eq!(healed.iter().map(|r| (r.name.as_str(), r.value)).collect::<Vec<_>>(), vec![("Brasscasings", 150)]);
    }

    #[test]
    fn a_pause_ends_a_fight() {
        let mut m = sample();
        let (a, v) = (Actor { id: 1, name: "Ballbearing".into(), player: true }, Actor { id: 10, name: "Reet".into(), player: false });
        // (a heal long after is no fight; a hit is the next one)
        m.add(40.0, &Event { attacker: Some(a.clone()), victim: a.clone(), amount: 80, heal: true, hit: Hit::Normal, source: "Kit".into() });
        assert_eq!(m.fights.len(), 1);
        m.add(50.0, &Event { attacker: Some(a.clone()), victim: v.clone(), amount: 300, heal: false, hit: Hit::Normal, source: "Weapon".into() });
        m.add(50.0 + GAP, &Event { attacker: Some(a.clone()), victim: v.clone(), amount: 100, heal: false, hit: Hit::Normal, source: "Weapon".into() });
        assert_eq!(m.fights.len(), 2);
        assert_eq!(m.summary(View::Latest), (GAP, "Reet".to_string()));
        assert_eq!(m.summary(View::Fight(0)).1, "Rollerrat");
        // (all together: the lengths added, 10 s and 12 s)
        let all = m.rows(View::All, Mode::Dealt, true, false);
        assert_eq!(all[0].name, "Ballbearing");
        assert_eq!(all[0].value, 820);
        assert!((all[0].per_second - 820.0 / (10.0 + GAP)).abs() < 1e-9);
        assert_eq!(m.summary(View::All).1, "2 fights");
        // (a source the client does not know hurts all the same)
        m.add(60.0, &Event { attacker: None, victim: a.clone(), amount: 25, heal: false, hit: Hit::Normal, source: String::new() });
        assert_eq!(m.rows(View::Latest, Mode::Taken, true, false).iter().find(|r| r.id == 1).map(|r| r.value), Some(25));
        // (no more than are kept)
        for k in 0..KEPT + 5 {
            m.add(1000.0 + 100.0 * k as f64, &Event { attacker: Some(a.clone()), victim: v.clone(), amount: 1, heal: false, hit: Hit::Normal, source: "Weapon".into() });
        }
        assert_eq!(m.fights.len(), KEPT);
    }

    #[test]
    fn the_report_is_lines_to_paste() {
        let m = sample();
        let text = m.report(View::Latest, Mode::Dealt, true, true);
        assert_eq!(text, "Damage: Rollerrat (0:10)\n1. Brasscasings 790 (79.0/s, 65%)\n2. Ballbearing 420 (42.0/s, 35%)\n");
        assert_eq!(grouped(1234567), "1,234,567");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(-1000), "-1,000");
        assert_eq!(clock(125.4), "2:05");
        assert_eq!([Hit::of_type(4), Hit::of_type(2), Hit::of_type(0)], [Hit::Critical, Hit::Glancing, Hit::Normal]);
    }
}
