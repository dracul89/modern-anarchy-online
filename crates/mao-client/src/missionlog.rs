//! The mission journal: GUI.dll's `MissionView_c` (`mission_window`, "Missions", FUN_100d3644), the character's own
//! missions, not the booth's offers (`mission.rs`, `MissionSelectionView_c`).
//!
//! The server keeps the list in `QuestFullUpdate` (`465a4061`, every mission: its identity, title, text, icon and the
//! actions to do, each with a playfield and a place) and drops one with `Quest` action Delete (`212c487a`). The list
//! (`MissionListView_c`, FUN_100d3df7) shows each mission; its menu (FUN_100d3123) is "Look at Mission", "Upload to map"
//! (when the mission names a place: `N3Msg_GetQuestWorldPos`), "Update Team Members" (team missions) and "Delete
//! Mission" (asked first: "ReallyWant2DeleteMission"). Uploading (FUN_10067d5f) puts the place on the planet map and the
//! compass and says so: "MissionWptUploaded", then "MissionShownAsDot" in the mission's playfield, else
//! "MissionNotInArea"; a mission without a place "NoWptInfo".

use std::sync::Arc;

use mao_net::schema::Value;

use crate::gear::Icon;
use crate::theme::Palette;

/// One of the character's missions.
#[derive(Clone)]
pub struct Mission {
    pub id: (u32, u32),
    pub title: String,
    /// The text (the server's markup: `<BR>` breaks).
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// The first action's playfield and place (server coordinates), when it names one.
    pub place: Option<(u32, [f32; 3])>,
}

/// What the window asks the network thread to send.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Delete((u32, u32)),
}

/// The missions of a parsed `QuestFullUpdate`.
pub fn missions_from(v: &Value, icon: &dyn Fn(u32) -> Option<Arc<Icon>>) -> Vec<Mission> {
    let int = |v: &Value, f: &str| v.get(f).and_then(Value::as_int).unwrap_or(0);
    let float = |v: &Value, f: &str| match v.get(f) {
        Some(Value::Float(x)) => *x as f32,
        Some(other) => other.as_int().unwrap_or(0) as f32,
        None => 0.0,
    };
    let text = |v: &Value, f: &str| match v.get(f) {
        Some(Value::Str(s)) => s.clone(),
        Some(Value::Bytes(b)) => String::from_utf8_lossy(b).into_owned(),
        _ => String::new(),
    };
    let Some(Value::List(quests)) = v.get("Quests") else { return Vec::new() };
    quests
        .iter()
        .map(|q| {
            let id = q.get("QuestId").map_or((0, 0), |i| (int(i, "Type") as u32, int(i, "Instance") as u32));
            let place = match q.get("QuestActions") {
                Some(Value::List(actions)) => actions.iter().find_map(|a| {
                    let pf = a.get("PlayfieldId").map_or(0, |p| int(p, "Instance") as u32);
                    let p = a.get("Position")?;
                    (pf != 0).then(|| (pf, [float(p, "X"), float(p, "Y"), float(p, "Z")]))
                }),
                _ => None,
            };
            Mission { id, title: text(q, "ShortInfo"), description: text(q, "LongInfo"), icon: icon(int(q, "MissionIconId") as u32), place }
        })
        .collect()
}

/// The server's markup as plain lines (`<BR>` a break, other tags left out).
pub fn plain(markup: &str) -> String {
    let mut out = String::new();
    let mut rest = markup;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let tag = rest[start + 1..start + end].trim().to_ascii_lowercase();
        if tag == "br" || tag == "br/" {
            out.push('\n');
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    out
}

/// A GUI text (text.mdb category 10000, the names GUI.dll asks for).
fn text(name: &str) -> String {
    let english = match name {
        "Look@Mission" => "Look at Mission",
        "Upload2Map" => "Upload to map",
        "UpdateTeamMembers" => "Update Team Members",
        "DeleteMission" => "Delete Mission",
        "ReallyWant2DeleteMission" => "Do you really want to delete this mission?",
        "MissionWptUploaded" => "Waypoint uploaded to map and compass.",
        "MissionShownAsDot" => "The waypoint is shown as a white dot on the map and a yellow indicator on the compass.",
        "MissionNotInArea" => "Mission is not in this area. You will be informed when entering correct area.",
        "NoWptInfo" => "No waypoint info available in mission data.",
        _ => name,
    };
    crate::feedback::texts().and_then(|t| t.named(10000, name)).unwrap_or(english).to_string()
}

#[derive(Default)]
pub struct MissionLog {
    pub open: bool,
    pub missions: Vec<Mission>,
    selected: Option<(u32, u32)>,
    /// The mission whose deletion awaits an answer.
    confirm: Option<(u32, u32)>,
    textures: std::collections::HashMap<u32, egui::TextureHandle>,
}

/// What a frame of the window did.
#[derive(Default)]
pub struct Shown {
    pub commands: Vec<Command>,
    /// A waypoint uploaded, and the lines to tell.
    pub waypoint: Option<(u32, [f32; 3])>,
    pub lines: Vec<String>,
}

impl MissionLog {
    /// The server's full list.
    pub fn set(&mut self, missions: Vec<Mission>) {
        self.missions = missions;
        if self.selected.is_some_and(|s| !self.missions.iter().any(|m| m.id == s)) {
            self.selected = None;
        }
    }

    /// The server dropped a mission.
    pub fn remove(&mut self, id: (u32, u32)) {
        self.missions.retain(|m| m.id != id);
    }

    /// FUN_10067d5f: the mission's place as the waypoint, and what is told.
    fn upload(m: &Mission, playfield: u32, shown: &mut Shown) {
        match m.place {
            Some((pf, p)) => {
                shown.waypoint = Some((pf, p));
                shown.lines.push(text("MissionWptUploaded"));
                shown.lines.push(text(if pf == playfield { "MissionShownAsDot" } else { "MissionNotInArea" }));
            }
            None => shown.lines.push(text("NoWptInfo")),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, playfield: u32) -> Shown {
        let mut shown = Shown::default();
        if !self.open {
            return shown;
        }
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut open = self.open;
        let (missions, selected, confirm, textures) = (&self.missions, &mut self.selected, &mut self.confirm, &mut self.textures);
        crate::chrome::window(ctx, palette, "Missions", "Missions", &mut open, |w| w.default_size(egui::vec2(380.0, 420.0)).resizable(true), |ui| {
            if missions.is_empty() {
                ui.label(egui::RichText::new("You have no missions.").weak());
                return;
            }
            let list_height = (ui.available_height() * 0.45).max(90.0);
            crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("missions-list").max_height(list_height).auto_shrink([false, true]), |ui| {
                for m in missions {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::click());
                    if *selected == Some(m.id) {
                        ui.painter().rect_filled(rect, 2.0, accent.gamma_multiply(0.25));
                    } else if response.hovered() {
                        ui.painter().rect_filled(rect, 2.0, egui::Color32::from_white_alpha(12));
                    }
                    let icon_rect = egui::Rect::from_min_size(rect.min + egui::vec2(2.0, 1.0), egui::vec2(32.0, 32.0));
                    if let Some(icon) = &m.icon {
                        let t = textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("mission-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR));
                        ui.painter().image(t.id(), icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                    }
                    let colour = egui::Color32::from_rgb(palette.text[0], palette.text[1], palette.text[2]);
                    ui.painter().text(egui::pos2(icon_rect.right() + 6.0, rect.center().y), egui::Align2::LEFT_CENTER, &m.title, egui::FontId::proportional(13.0), colour);
                    if response.clicked() {
                        *selected = Some(m.id);
                    }
                    if response.double_clicked() {
                        MissionLog::upload(m, playfield, &mut shown);
                    }
                    response.context_menu(|ui| {
                        if ui.button(text("Look@Mission")).clicked() {
                            *selected = Some(m.id);
                            ui.close();
                        }
                        if m.place.is_some() && ui.button(text("Upload2Map")).clicked() {
                            MissionLog::upload(m, playfield, &mut shown);
                            ui.close();
                        }
                        // (team missions are not known to the client yet: `N3Msg_IsTeamMission`)
                        ui.add_enabled(false, egui::Button::new(text("UpdateTeamMembers")));
                        if ui.button(text("DeleteMission")).clicked() {
                            *confirm = Some(m.id);
                            ui.close();
                        }
                    });
                }
            });
            ui.separator();
            if let Some(m) = selected.and_then(|s| missions.iter().find(|m| m.id == s)) {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&m.title).strong());
                    if m.place.is_some() && crate::chrome::button(ui, palette, &text("Upload2Map"), true).clicked() {
                        MissionLog::upload(m, playfield, &mut shown);
                    }
                });
                crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("mission-text").auto_shrink([false, false]), |ui| {
                    ui.label(plain(&m.description));
                });
            } else {
                ui.label(egui::RichText::new("Pick a mission to read it; right-click it for more.").weak());
            }
        });
        // ("ReallyWant2DeleteMission")
        if let Some(id) = self.confirm {
            let mut keep = true;
            egui::Window::new("Delete Mission").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(ctx, |ui| {
                ui.label(text("ReallyWant2DeleteMission"));
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, "Yes", true).clicked() {
                        shown.commands.push(Command::Delete(id));
                        keep = false;
                    }
                    if crate::chrome::button(ui, palette, "No", true).clicked() {
                        keep = false;
                    }
                });
            });
            if !keep {
                self.confirm = None;
            }
        }
        self.open = open;
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_is_plain() {
        assert_eq!(plain("Place the Bomb<BR><BR>Use the <b>kit</b>."), "Place the Bomb\n\nUse the kit.");
    }
}
