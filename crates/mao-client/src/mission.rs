//! The mission booth's window, built from the real exchange in the user's capture:
//!
//! * the booth is used (`GenericCmd` Use on `(0xdac1, serial)`), which only echoes: the window is client side;
//! * "Request Missions" sends `QuestAlternative` (`5c436609`): seven slider bytes, a scope (5 for the
//!   Old Athens booth in the capture; the AOSharp sources' 1 / 2 did not work) and the terminal;
//! * the server answers with the same message holding up to five offers (`MissionInfo`: identity of kind
//!   `0xdac3`, title, description, credits, xp, reward items, icon, destination playfield and position);
//! * "Accept Mission" sends `CreateQuest` (`291f361b`) with the offer's identity, and the server then
//!   sends an updated `QuestFullUpdate` (the mission log).

use std::sync::Arc;

use mao_net::schema::Value;

use crate::gear::{GearWindow, Icon};
use crate::theme::Palette;

/// The kind of mission the Old Athens terminal of the capture gives, sent with the request: the terminal's
/// `QuestOriginator_e` ([`ORIGINATOR_STAT`]), 5 a clan terminal's own missions. A request names its terminal's kind
/// (`crate::mission_terminals`); this one is for a terminal whose kind is not known.
pub const DEFAULT_SCOPE: u8 = 5;

/// A mission terminal's stat with the kind of mission it gives (`QuestOriginator_e`; Gamecode FUN_10086b3e reads it of
/// the terminal that is used).
pub const ORIGINATOR_STAT: u32 = 490;

/// `GameData::IsTeamOriginator` (Gamedata.dll 0x10002d23): the kinds that are a team's missions.
pub fn team_originator(kind: u8) -> bool {
    matches!(kind, 2 | 4 | 6 | 8)
}

/// What stands in the way of using a mission terminal of this kind (Gamecode FUN_10086b3e): a team's terminal used by
/// someone who is in no team is refused with text.mdb's `QuestBooth_NeedTeam` (category 102) in an error box, and
/// the terminal is not used.
pub fn use_refused(kind: Option<u8>, in_team: bool) -> Option<String> {
    (kind.is_some_and(team_originator) && !in_team).then(|| crate::feedback::texts().and_then(|t| t.named(102, "QuestBooth_NeedTeam")).unwrap_or("You must be in a team to use a team mission terminal.").to_string())
}

/// What stands in the way of asking a terminal for missions (GUI.dll FUN_100d09cb, the Request Missions button): the
/// asking costs as many credits as the character has levels, and with fewer (stat 61 under stat 54) the window says
/// `NotEnoughCredits_NeedX` with the number in an error box and asks nothing. Its other refusal is [`slots_refused`].
pub fn request_refused(level: i64, credits: i64) -> Option<String> {
    (credits < level).then(|| {
        let text = crate::feedback::texts().and_then(|t| t.named(10000, "NotEnoughCredits_NeedX")).unwrap_or("You do not have enough credits: you need %d.");
        // (`LDBformat::Feed`: the number in the text's place for it)
        match text.find('%') {
            Some(at) => {
                let end = text[at + 1..].find(|c: char| c.is_ascii_alphabetic()).map_or(text.len(), |e| at + 2 + e);
                format!("{}{level}{}", &text[..at], &text[end..])
            }
            None => format!("{text} {level}"),
        }
    })
}

/// The seven sliders: (left label, right label, largest value). The captured values (2, 20, 161, 30, 184,
/// 28, 0) fit a difficulty of 0-7 and 0-255 for the others, which is a guess until more captures show it.
pub const SLIDERS: [(&str, &str, u8); 7] = [("Easy", "Hard", 7), ("Good", "Bad", 255), ("Order", "Chaos", 255), ("Open", "Hidden", 255), ("Physical", "Mystical", 255), ("Head on", "Stealth", 255), ("Credits", "XP", 255)];

/// A reward item of an offer.
pub struct Reward {
    pub name: String,
    pub quality: i64,
    pub icon: Option<Arc<Icon>>,
}

/// One offered mission.
pub struct Offer {
    pub id: (u32, u32),
    pub title: String,
    pub description: String,
    pub credits: i64,
    pub xp: i64,
    pub rewards: Vec<Reward>,
    pub icon: Option<Arc<Icon>>,
    /// (playfield number, x, y, z) of the destination.
    pub destination: (u32, [f32; 3]),
    /// The destination playfield's name, when the client knows it (the network thread fills it in).
    pub playfield: String,
}

/// The mission helper: what the player is asking the terminal for, in the manner of the old outside helpers that read
/// the offers off the screen. This client's own. The player lists parts of the names of reward items and of places
/// (playfields); an offer fits when one of its rewards holds one of the item parts and its place one of the place
/// parts, a list left empty asking nothing. With nothing listed, nothing fits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wanted {
    pub items: Vec<String>,
    pub places: Vec<String>,
}

impl Wanted {
    /// From the two boxes' text, a part a line (blank lines and case set aside).
    pub fn from_lines(items: &str, places: &str) -> Wanted {
        let parts = |text: &str| text.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect();
        Wanted { items: parts(items), places: parts(places) }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.places.is_empty()
    }

    /// What of an offer fits (the rewards and the place that do, as they are named), or none when the offer does not.
    pub fn fits(&self, rewards: &[&str], place: &str) -> Option<Vec<String>> {
        if self.is_empty() {
            return None;
        }
        let mut what = Vec::new();
        if !self.items.is_empty() {
            let found: Vec<String> = rewards.iter().filter(|r| self.items.iter().any(|part| r.to_lowercase().contains(part.as_str()))).map(|r| r.to_string()).collect();
            if found.is_empty() {
                return None;
            }
            what.extend(found);
        }
        if !self.places.is_empty() {
            if !self.places.iter().any(|part| place.to_lowercase().contains(part.as_str())) {
                return None;
            }
            what.push(place.to_string());
        }
        Some(what)
    }

    fn of(&self, offer: &Offer) -> Option<Vec<String>> {
        let rewards: Vec<&str> = offer.rewards.iter().map(|r| r.name.as_str()).collect();
        self.fits(&rewards, &offer.playfield)
    }
}

/// The helper's file beside the settings: `item <part>` and `place <part>` lines, `limit <n>` and `keep <0|1>`.
fn helper_path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("missionhelper.txt")
}

/// The helper's settings as its file's text, and back: (items, places, the most requests in a row, keep asking).
pub fn helper_text(items: &str, places: &str, limit: u32, keep: bool) -> String {
    let mut out = String::new();
    for (word, text) in [("item", items), ("place", places)] {
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            out += &format!("{word} {line}\n");
        }
    }
    out + &format!("limit {limit}\nkeep {}\n", u8::from(keep))
}

pub fn helper_from(text: &str) -> (String, String, u32, bool) {
    let (mut items, mut places, mut limit, mut keep) = (String::new(), String::new(), DEFAULT_LIMIT, false);
    for line in text.lines() {
        match line.split_once(' ') {
            Some(("item", part)) => items += &format!("{}\n", part.trim()),
            Some(("place", part)) => places += &format!("{}\n", part.trim()),
            Some(("limit", n)) => limit = n.trim().parse().unwrap_or(DEFAULT_LIMIT).clamp(1, MOST_REQUESTS),
            Some(("keep", n)) => keep = n.trim() == "1",
            _ => {}
        }
    }
    (items, places, limit, keep)
}

/// How many requests in a row the helper makes unless told otherwise, and the most it can be told.
pub const DEFAULT_LIMIT: u32 = 20;
pub const MOST_REQUESTS: u32 = 200;
/// The pause between an answer that did not fit and the next request, and how long an answer is waited for.
const ROLL_PAUSE: std::time::Duration = std::time::Duration::from_millis(1200);
const ROLL_PATIENCE: std::time::Duration = std::time::Duration::from_secs(10);

/// Asking again and again: the requests made, the answers seen when the last was made, and when.
#[derive(Clone, Copy, Debug)]
struct Roll {
    requests: u32,
    answers: u32,
    asked: std::time::Instant,
    /// When the answer that did not fit came (the next request goes out a pause later).
    answered: Option<std::time::Instant>,
}

/// What the helper does next (its rule, apart from the clock and the window).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Still waiting for the answer, or for the pause to pass.
    Wait,
    /// An offer fits: stop, and point at it.
    Found(usize),
    /// Ask again.
    Ask,
    /// The limit is reached with nothing fitting.
    GiveUp,
    /// No answer came.
    Silent,
}

/// The rule: with a new answer, the first offer that fits ends it; else another request after the pause while the
/// limit allows; with no answer in time, it ends.
pub fn next_step(fits: Option<usize>, answered: bool, since_answer: Option<std::time::Duration>, since_asked: std::time::Duration, requests: u32, limit: u32) -> Next {
    if !answered {
        return if since_asked > ROLL_PATIENCE { Next::Silent } else { Next::Wait };
    }
    match fits {
        Some(k) => Next::Found(k),
        None if requests >= limit => Next::GiveUp,
        None if since_answer.is_some_and(|d| d >= ROLL_PAUSE) => Next::Ask,
        None => Next::Wait,
    }
}

/// What the window asks the network thread to do (carried in `Input`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Command {
    /// 1 request missions, 2 accept one; 0 nothing.
    pub what: u8,
    pub sliders: [u8; 7],
    pub mission: (u32, u32),
}

/// The offers in a parsed `QuestAlternative` (`items` finds a reward item's name and icon, `icon` a mission icon).
pub fn offers_from(value: &Value, item: &dyn Fn(i64) -> Option<(String, Option<Arc<Icon>>)>, icon: &dyn Fn(u32) -> Option<Arc<Icon>>) -> Vec<Offer> {
    let Some(Value::List(list)) = value.get("MissionDetails") else { return Vec::new() };
    let int = |v: &Value, f: &str| v.get(f).and_then(Value::as_int).unwrap_or(0);
    let text = |v: &Value, f: &str| match v.get(f) {
        Some(Value::Str(s)) => s.clone(),
        _ => String::new(),
    };
    list.iter()
        .map(|m| {
            let id = m.get("MissionIdentity").map(|i| (int(i, "Type") as u32, int(i, "Instance") as u32)).unwrap_or((0, 0));
            let rewards = match m.get("MissionItemData") {
                Some(Value::List(items)) => items
                    .iter()
                    .map(|r| {
                        let found = item(int(r, "LowId")).or_else(|| item(int(r, "HighId")));
                        let (name, icon) = found.unwrap_or_else(|| (format!("Item #{:x}", int(r, "LowId")), None));
                        Reward { name, quality: int(r, "Ql"), icon }
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let place = m.get("Playfield").map(|p| int(p, "Instance") as u32).unwrap_or(0);
            let at = m.get("Location").map(|l| {
                let f = |n: &str| match l.get(n) {
                    Some(Value::Float(x)) => *x as f32,
                    _ => 0.0,
                };
                [f("X"), f("Y"), f("Z")]
            });
            Offer {
                id,
                title: text(m, "Title"),
                description: text(m, "Description"),
                credits: int(m, "Credits"),
                xp: int(m, "XpReward"),
                rewards,
                icon: icon(int(m, "MissionIcon") as u32),
                destination: (place, at.unwrap_or([0.0; 3])),
                playfield: String::new(),
            }
        })
        .collect()
}

/// The window's state.
pub struct MissionWindow {
    pub open: bool,
    pub sliders: [u8; 7],
    pub offers: Vec<Offer>,
    pub selected: Option<usize>,
    /// What the player is told ("Asking the booth ...", "No booth in reach").
    pub status: String,
    /// The helper: the two boxes' text (parts of reward names, parts of place names), whether to keep asking until an
    /// offer fits and how many times at most; read from its file when the window is first shown.
    pub want_items: String,
    pub want_places: String,
    pub keep_asking: bool,
    pub limit: u32,
    helper_loaded: bool,
    /// The answers the terminal has given (counted, to tell a new one).
    answers: u32,
    roll: Option<Roll>,
}

impl Default for MissionWindow {
    fn default() -> MissionWindow {
        MissionWindow { open: false, sliders: [2, 128, 128, 128, 128, 128, 128], offers: Vec::new(), selected: None, status: String::new(), want_items: String::new(), want_places: String::new(), keep_asking: false, limit: DEFAULT_LIMIT, helper_loaded: false, answers: 0, roll: None }
    }
}

impl MissionWindow {
    /// Replaces the offers (a new answer from the booth).
    pub fn set_offers(&mut self, offers: Vec<Offer>) {
        self.status = if offers.is_empty() { "The booth offered no missions.".into() } else { format!("{} missions offered.", offers.len()) };
        self.selected = None;
        self.offers = offers;
        self.answers = self.answers.wrapping_add(1);
    }

    /// The helper's settings from a picture's or a test's hand (not read from the file after).
    pub fn set_helper(&mut self, items: &str, places: &str, keep: bool, limit: u32) {
        (self.want_items, self.want_places, self.keep_asking, self.limit, self.helper_loaded) = (items.to_string(), places.to_string(), keep, limit.clamp(1, MOST_REQUESTS), true);
    }

    /// Whether the helper is asking again and again just now.
    pub fn rolling(&self) -> bool {
        self.roll.is_some()
    }

    /// Draws the window; returns what to send, if anything.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, palette: &Palette) -> Command {
        let mut command = Command::default();
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let fitting = egui::Color32::from_rgb(110, 220, 120);
        let mut is_open = self.open;
        let mut close = false;
        if !std::mem::replace(&mut self.helper_loaded, true) {
            if let Ok(text) = std::fs::read_to_string(helper_path()) {
                (self.want_items, self.want_places, self.limit, self.keep_asking) = helper_from(&text);
            }
        }
        let helper_before = helper_text(&self.want_items, &self.want_places, self.limit, self.keep_asking);
        let wanted = Wanted::from_lines(&self.want_items, &self.want_places);
        let fits: Vec<Option<Vec<String>>> = self.offers.iter().map(|o| wanted.of(o)).collect();
        // The helper asking again and again: an answer that fits ends it, one that does not is followed by the next
        // request after a pause, up to the limit.
        if let Some(roll) = self.roll {
            let answered = self.answers != roll.answers;
            let mut roll = roll;
            if answered && roll.answered.is_none() {
                roll.answered = Some(std::time::Instant::now());
            }
            match next_step(fits.iter().position(|f| f.is_some()), answered, roll.answered.map(|t| t.elapsed()), roll.asked.elapsed(), roll.requests, self.limit) {
                Next::Wait => self.roll = Some(roll),
                Next::Found(k) => {
                    self.roll = None;
                    self.selected = Some(k);
                    self.status = format!("An offer fits, after {} request{}.", roll.requests, if roll.requests == 1 { "" } else { "s" });
                }
                Next::Ask => {
                    command = Command { what: 1, sliders: self.sliders, mission: (0, 0) };
                    self.roll = Some(Roll { requests: roll.requests + 1, answers: self.answers, asked: std::time::Instant::now(), answered: None });
                    self.status = format!("Nothing fitting yet: asking again ({} of at most {}) ...", roll.requests + 1, self.limit);
                }
                Next::GiveUp => {
                    self.roll = None;
                    self.status = format!("Nothing fitting in {} requests.", roll.requests);
                }
                Next::Silent => {
                    self.roll = None;
                    self.status = "The terminal did not answer: stopped asking.".into();
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        crate::chrome::window(ctx, palette, "Missions", "Missions", &mut is_open, |w| w.resizable(false).default_pos(crate::layout::pos("Missions", egui::pos2(60.0, 80.0))), |ui| {
            ui.set_width(330.0);
            // The six offer boxes (three by two), as in the original.
            egui::Grid::new("mission-offers").spacing(egui::vec2(6.0, 6.0)).show(ui, |ui| {
                for slot in 0..6 {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(96.0, 64.0), egui::Sense::click());
                    let selected = self.selected == Some(slot);
                    let painter = ui.painter();
                    painter.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(130));
                    painter.rect_stroke(rect, 4.0, egui::Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { accent } else { egui::Color32::from_white_alpha(50) }), egui::StrokeKind::Inside);
                    if let Some(offer) = self.offers.get(slot) {
                        match &offer.icon {
                            Some(icon) => {
                                let id = gear.texture(ctx, icon);
                                ui.painter().image(id, egui::Rect::from_center_size(rect.center(), egui::vec2(48.0, 48.0)), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                            None => {
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, slot + 1, egui::FontId::proportional(22.0), accent);
                            }
                        }
                        // (an offer the helper's lists fit is framed for it)
                        if fits.get(slot).is_some_and(|f| f.is_some()) {
                            ui.painter().rect_stroke(rect.shrink(2.0), 4.0, egui::Stroke::new(2.0, fitting), egui::StrokeKind::Inside);
                        }
                        if response.clicked() {
                            self.selected = Some(slot);
                        }
                        response.on_hover_text(&offer.title);
                    }
                    if slot % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
            ui.add_space(4.0);
            for (i, (left, right, max)) in SLIDERS.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_sized([60.0, 16.0], egui::Label::new(*left));
                    ui.spacing_mut().slider_width = 150.0;
                    ui.add(egui::Slider::new(&mut self.sliders[i], 0..=*max).show_value(false));
                    ui.label(*right);
                });
            }
            ui.horizontal(|ui| {
                if self.roll.is_some() {
                    if crate::chrome::button(ui, palette, "Stop asking", true).clicked() {
                        self.roll = None;
                        self.status = "Stopped.".into();
                    }
                } else if crate::chrome::button(ui, palette, "Request Missions", true).clicked() {
                    command = Command { what: 1, sliders: self.sliders, mission: (0, 0) };
                    self.status = "Asking the booth ...".into();
                    // (with the helper told to keep asking, and something to look for)
                    if self.keep_asking && !wanted.is_empty() {
                        self.roll = Some(Roll { requests: 1, answers: self.answers, asked: std::time::Instant::now(), answered: None });
                    }
                }
            });
            // The helper: every offer in a line (its rewards, its place, its pay), what fits the lists marked; and the
            // lists themselves.
            if !self.offers.is_empty() {
                ui.separator();
                for (k, offer) in self.offers.iter().enumerate() {
                    let fit = fits.get(k).and_then(|f| f.as_ref());
                    let rewards = if offer.rewards.is_empty() { "no item".to_string() } else { offer.rewards.iter().map(|r| format!("{} (QL {})", r.name, r.quality)).collect::<Vec<_>>().join(", ") };
                    let place = if offer.playfield.is_empty() { format!("playfield {}", offer.destination.0) } else { offer.playfield.clone() };
                    let line = format!("{}. {rewards} - {place} - {} cr", k + 1, offer.credits);
                    let colour = if fit.is_some() { fitting } else if self.selected == Some(k) { accent } else { ui.visuals().text_color() };
                    let response = ui.add(egui::Label::new(egui::RichText::new(line).small().color(colour)).sense(egui::Sense::click()).wrap());
                    if response.clicked() {
                        self.selected = Some(k);
                    }
                    if let Some(what) = fit {
                        response.on_hover_text(format!("Fits: {}", what.join(", ")));
                    }
                }
            }
            egui::CollapsingHeader::new(egui::RichText::new("Looking for").small().color(accent)).id_salt("mission-helper").default_open(!wanted.is_empty()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Reward items (a part of a name a line)").small());
                        ui.add(egui::TextEdit::multiline(&mut self.want_items).desired_rows(3).desired_width(155.0));
                    });
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Places (a part a line)").small());
                        ui.add(egui::TextEdit::multiline(&mut self.want_places).desired_rows(3).desired_width(155.0));
                    });
                });
                ui.horizontal(|ui| {
                    crate::chrome::checkbox(ui, palette, &mut self.keep_asking, "Keep asking until one fits, at most");
                    ui.add(egui::DragValue::new(&mut self.limit).range(1..=MOST_REQUESTS));
                    ui.label("times");
                });
                ui.label(egui::RichText::new("Each request costs as many credits as you have levels.").small().weak());
            });
            if let Some(offer) = self.selected.and_then(|s| self.offers.get(s)) {
                ui.separator();
                ui.label(egui::RichText::new(&offer.title).strong().color(accent));
                crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(140.0), |ui| {
                    ui.label(offer.description.replace("<BR>", "\n").replace("<br>", "\n"));
                });
                ui.label(format!("Reward: {} credits, {} xp", offer.credits, offer.xp));
                for r in &offer.rewards {
                    ui.horizontal(|ui| {
                        if let Some(icon) = &r.icon {
                            let id = gear.texture(ctx, icon);
                            ui.add(egui::Image::new((id, egui::vec2(24.0, 24.0))));
                        }
                        ui.label(format!("{} (QL {})", r.name, r.quality));
                    });
                }
                let (pf, at) = offer.destination;
                let place = if offer.playfield.is_empty() { format!("Playfield {pf}") } else { offer.playfield.clone() };
                ui.label(egui::RichText::new(format!("{place}, at {:.0}, {:.0}", at[0], at[2])).small().weak());
            }
            if !self.status.is_empty() {
                ui.label(egui::RichText::new(&self.status).small());
            }
            ui.horizontal(|ui| {
                let can = self.selected.is_some_and(|s| s < self.offers.len());
                if crate::chrome::button(ui, palette, "Accept Mission", can).clicked() {
                    if let Some(offer) = self.selected.and_then(|s| self.offers.get(s)) {
                        command = Command { what: 2, sliders: self.sliders, mission: offer.id };
                        self.status = format!("Accepting {} ...", offer.title);
                    }
                }
                if crate::chrome::button(ui, palette, "Cancel", true).clicked() {
                    close = true;
                }
            });
        });
        self.open = is_open && !close;
        // (a window shut stops the asking; the helper's lists are kept in its file as they change)
        if !self.open {
            self.roll = None;
        }
        let helper_now = helper_text(&self.want_items, &self.want_places, self.limit, self.keep_asking);
        if helper_now != helper_before {
            let _ = std::fs::write(helper_path(), helper_now);
        }
        command
    }
}

#[cfg(test)]
mod helper_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn an_offer_fits_by_its_rewards_and_its_place() {
        let wanted = Wanted::from_lines("  Pillow\n\nnano crystal", "");
        assert_eq!(wanted.items, vec!["pillow".to_string(), "nano crystal".to_string()]);
        assert_eq!(wanted.fits(&["Pillow with Important Stripes", "Medical Kit"], "Athen Shire"), Some(vec!["Pillow with Important Stripes".to_string()]));
        assert_eq!(wanted.fits(&["Medical Kit"], "Athen Shire"), None);
        // (a place asked for as well: both have to fit)
        let both = Wanted::from_lines("pillow", "shire\naegean");
        assert_eq!(both.fits(&["Pillow with Important Stripes"], "Athen Shire"), Some(vec!["Pillow with Important Stripes".to_string(), "Athen Shire".to_string()]));
        assert_eq!(both.fits(&["Pillow with Important Stripes"], "Galway County"), None);
        assert_eq!(both.fits(&["Medical Kit"], "Aegean"), None);
        // (only a place: any reward, or none)
        let place = Wanted::from_lines("", "AEGEAN");
        assert_eq!(place.fits(&[], "Aegean"), Some(vec!["Aegean".to_string()]));
        // (nothing asked for fits nothing)
        assert!(Wanted::from_lines(" \n", "").is_empty());
        assert_eq!(Wanted::default().fits(&["Anything"], "Anywhere"), None);
    }

    #[test]
    fn the_helper_asks_again_until_one_fits_or_the_limit() {
        let (s, ms) = (Duration::from_secs, Duration::from_millis);
        // (no answer yet: wait, and give up on a terminal that stays silent)
        assert_eq!(next_step(None, false, None, s(2), 1, 20), Next::Wait);
        assert_eq!(next_step(None, false, None, s(11), 1, 20), Next::Silent);
        // (an answer that fits ends it, whatever the count)
        assert_eq!(next_step(Some(3), true, Some(ms(0)), s(1), 20, 20), Next::Found(3));
        // (one that does not: the next request after the pause, until the limit)
        assert_eq!(next_step(None, true, Some(ms(300)), s(1), 1, 20), Next::Wait);
        assert_eq!(next_step(None, true, Some(ms(1300)), s(2), 1, 20), Next::Ask);
        assert_eq!(next_step(None, true, Some(ms(1300)), s(2), 20, 20), Next::GiveUp);
    }

    #[test]
    fn the_helpers_file_keeps_the_lists() {
        let text = helper_text("pillow\n  nano crystal \n", "aegean", 35, true);
        assert_eq!(text, "item pillow\nitem nano crystal\nplace aegean\nlimit 35\nkeep 1\n");
        assert_eq!(helper_from(&text), ("pillow\nnano crystal\n".to_string(), "aegean\n".to_string(), 35, true));
        assert_eq!(helper_from("limit 100000\n"), (String::new(), String::new(), MOST_REQUESTS, false));
        assert_eq!(helper_from(""), (String::new(), String::new(), DEFAULT_LIMIT, false));
    }
}

/// The inventory's places (Gamecode FUN_10047720: thirty, less what lies in the inventory from place 0x40 on).
pub const INVENTORY_PLACES: usize = 30;

/// The terminal's other refusal, on asking for missions and on accepting one (GUI.dll FUN_100d09cb, FUN_100d0c61): with
/// fewer than two free places in the inventory (`N3Msg_GetNumberOfFreeInventorySlots`), `MustHave2FreeSlots` in an
/// error box, and nothing is asked or accepted.
pub fn slots_refused(carried: usize) -> Option<String> {
    (INVENTORY_PLACES.saturating_sub(carried) < 2).then(|| crate::feedback::texts().and_then(|t| t.named(10000, "MustHave2FreeSlots")).unwrap_or("You need two free places in your inventory for a mission.").to_string())
}

#[cfg(test)]
mod refusal_tests {
    use super::*;

    #[test]
    fn a_teams_terminal_wants_a_team() {
        // (the kinds: 1, 3 and 5 one's own missions, 2, 4 and 6 a team's)
        assert_eq!([1u8, 2, 3, 4, 5, 6, 7, 8].map(team_originator), [false, true, false, true, false, true, false, true]);
        assert!(use_refused(Some(6), false).is_some());
        assert!(use_refused(Some(6), true).is_none());
        assert!(use_refused(Some(5), false).is_none());
        assert!(use_refused(None, false).is_none());
    }

    #[test]
    fn a_mission_wants_two_free_places() {
        assert!(slots_refused(28).is_none());
        assert!(slots_refused(0).is_none());
        assert!(slots_refused(29).is_some());
        assert!(slots_refused(30).is_some());
        assert!(slots_refused(31).is_some());
    }

    #[test]
    fn asking_costs_a_credit_a_level() {
        assert!(request_refused(60, 60).is_none());
        assert!(request_refused(60, 100_000).is_none());
        let said = request_refused(60, 59).unwrap();
        assert!(said.contains("60"), "{said}");
    }
}
