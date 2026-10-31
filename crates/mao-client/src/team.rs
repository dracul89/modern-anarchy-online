//! The team: an invitation to answer, the members with their health and nano, and leaving. The protocol is the Arete Landing
//! capture's (2026-10-30, the original client): an invitation is the server's `CharacterAction` 0x1a naming who asks; the
//! answer is the client's 0x1c naming them with 1 (join) or 0 (no); the server then sends `TeamMember` (`46312d2e`: the
//! character, the team, a word, the level, the profession, the name) for each member and `TeamMemberInfo` (`28784248`:
//! health, max health, max nano, nano) as they change. Leaving is the client's 0x18; the server's 0x20 names who left (oneself:
//! the team is gone). aosharp's `Team` names the same actions.

use std::sync::Mutex;

use crate::theme::Palette;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mate {
    pub id: u32,
    pub name: String,
    pub level: i64,
    pub profession: i64,
    /// (now, max)
    pub health: (i64, i64),
    pub nano: (i64, i64),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Team {
    pub members: Vec<Mate>,
    /// Who asks us to join (their id and name).
    pub invite: Option<(u32, String)>,
}

impl Team {
    /// A member, added when new.
    pub fn member(&mut self, id: u32) -> &mut Mate {
        if let Some(at) = self.members.iter().position(|m| m.id == id) {
            return &mut self.members[at];
        }
        self.members.push(Mate { id, ..Mate::default() });
        self.members.last_mut().unwrap()
    }
}

/// What the window asks the connection to send.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Answer an invitation from this character.
    Reply(u32, bool),
    Leave,
}

static TEAM: Mutex<Team> = Mutex::new(Team { members: Vec::new(), invite: None });
static COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());

/// The team as the connection last heard it.
pub fn with<R>(f: impl FnOnce(&mut Team) -> R) -> R {
    f(&mut TEAM.lock().unwrap_or_else(|e| e.into_inner()))
}

pub fn take_commands() -> Vec<Command> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn send(c: Command) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(c);
}

#[derive(Default)]
pub struct TeamWindow {
    pub open: bool,
}

/// A bar of `now` out of `max`.
fn bar(ui: &mut egui::Ui, now: i64, max: i64, colour: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 6.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 1.0, egui::Color32::from_black_alpha(140));
    if max > 0 {
        let share = (now as f32 / max as f32).clamp(0.0, 1.0);
        painter.rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * share, rect.height())), 1.0, colour);
    }
}

impl TeamWindow {
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) {
        let team = with(|t| t.clone());
        // (an invitation opens the window)
        if team.invite.is_some() {
            self.open = true;
        }
        if !self.open {
            return;
        }
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut open = self.open;
        crate::chrome::window(ctx, palette, "Team", "Team", &mut open, |w| w.default_pos(crate::layout::pos("Team", egui::pos2(20.0, 300.0))).default_width(220.0), |ui| {
            if let Some((id, name)) = &team.invite {
                ui.label(format!("{name} asks you to join a team."));
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, "Join", true).clicked() {
                        send(Command::Reply(*id, true));
                        with(|t| t.invite = None);
                    }
                    if crate::chrome::button(ui, palette, "Decline", true).clicked() {
                        send(Command::Reply(*id, false));
                        with(|t| t.invite = None);
                    }
                });
                ui.separator();
            }
            if team.members.is_empty() {
                ui.label(egui::RichText::new("Not in a team.").weak());
                return;
            }
            for m in &team.members {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&m.name).color(accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{}", m.level)).small());
                    });
                });
                if m.health.1 > 0 {
                    bar(ui, m.health.0, m.health.1, egui::Color32::from_rgb(200, 50, 40));
                    bar(ui, m.nano.0, m.nano.1, egui::Color32::from_rgb(50, 110, 220));
                }
            }
            ui.separator();
            if crate::chrome::button(ui, palette, "Leave team", true).clicked() {
                send(Command::Leave);
            }
        });
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_are_added_once() {
        let mut t = Team::default();
        t.member(5).name = "A".into();
        t.member(5).level = 7;
        t.member(6);
        assert_eq!(t.members.len(), 2);
        assert_eq!((t.members[0].name.as_str(), t.members[0].level), ("A", 7));
    }
}
