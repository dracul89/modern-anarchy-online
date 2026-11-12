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
/// `NotEnoughCredits_NeedX` with the number in an error box and asks nothing. (Its other refusal, fewer than two free
/// places in the inventory, `MustHave2FreeSlots`, is not here yet.)
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
}

impl Default for MissionWindow {
    fn default() -> MissionWindow {
        MissionWindow { open: false, sliders: [2, 128, 128, 128, 128, 128, 128], offers: Vec::new(), selected: None, status: String::new() }
    }
}

impl MissionWindow {
    /// Replaces the offers (a new answer from the booth).
    pub fn set_offers(&mut self, offers: Vec<Offer>) {
        self.status = if offers.is_empty() { "The booth offered no missions.".into() } else { format!("{} missions offered.", offers.len()) };
        self.selected = None;
        self.offers = offers;
    }

    /// Draws the window; returns what to send, if anything.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, palette: &Palette) -> Command {
        let mut command = Command::default();
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut is_open = self.open;
        let mut close = false;
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
                if crate::chrome::button(ui, palette, "Request Missions", true).clicked() {
                    command = Command { what: 1, sliders: self.sliders, mission: (0, 0) };
                    self.status = "Asking the booth ...".into();
                }
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
                ui.label(egui::RichText::new(format!("Playfield {pf}, at {:.0}, {:.0}", at[0], at[2])).small().weak());
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
        command
    }
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
    fn asking_costs_a_credit_a_level() {
        assert!(request_refused(60, 60).is_none());
        assert!(request_refused(60, 100_000).is_none());
        let said = request_refused(60, 59).unwrap();
        assert!(said.contains("60"), "{said}");
    }
}
