//! The friends list window (`L`): the characters on the list the chat server keeps, those online first with a green dot, and a
//! field to add one by name. A friend can be told (the button fills the chat input with `/tell name `) or taken off the list.

use crate::chatwindow::Out;
use crate::theme::Palette;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Friend {
    pub id: u32,
    pub name: String,
    pub online: bool,
}

#[derive(Default)]
pub struct FriendsWindow {
    pub open: bool,
    add: String,
}

/// The order of the list: those online first, then by name.
pub fn sorted(friends: &[Friend]) -> Vec<&Friend> {
    let mut list: Vec<&Friend> = friends.iter().collect();
    list.sort_by(|a, b| b.online.cmp(&a.online).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    list
}

impl FriendsWindow {
    /// Shows the window; returns what to send to the chat server and the name to start a tell to, if one was chosen.
    pub fn show(&mut self, ctx: &egui::Context, friends: &[Friend], palette: &Palette) -> (Vec<Out>, Option<String>) {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let (mut outs, mut tell) = (Vec::new(), None);
        let mut open = self.open;
        let online = friends.iter().filter(|f| f.online).count();
        crate::chrome::window(ctx, palette, "Friends", "Friends", &mut open, |w| w.default_pos(crate::layout::pos("Friends", egui::pos2(300.0, 120.0))).default_width(260.0), |ui| {
            ui.label(egui::RichText::new(format!("{online} of {} online", friends.len())).small().weak());
            ui.separator();
            if friends.is_empty() {
                ui.label("Nobody on your list yet.");
            }
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for friend in sorted(friends) {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                        let dot = if friend.online { egui::Color32::from_rgb(90, 220, 110) } else { egui::Color32::from_gray(90) };
                        ui.painter().circle_filled(rect.center(), 4.0, dot);
                        let name = egui::RichText::new(&friend.name);
                        ui.label(if friend.online { name.color(egui::Color32::WHITE) } else { name.weak() });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("x").on_hover_text("Take off your friends list").clicked() {
                                outs.push(Out::Unfriend { id: friend.id });
                            }
                            if ui.small_button("Tell").clicked() {
                                tell = Some(friend.name.clone());
                            }
                        });
                    });
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                let response = ui.add(egui::TextEdit::singleline(&mut self.add).desired_width(150.0).hint_text("Name"));
                let go = ui.button("Add").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                let name = self.add.trim().to_string();
                if go && !name.is_empty() {
                    outs.push(Out::Befriend { name });
                    self.add.clear();
                }
            });
        });
        self.open = open;
        (outs, tell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn those_online_come_first_then_by_name() {
        let friends = vec![
            Friend { id: 1, name: "zed".into(), online: true },
            Friend { id: 2, name: "Abe".into(), online: false },
            Friend { id: 3, name: "bob".into(), online: true },
            Friend { id: 4, name: "Cy".into(), online: false },
        ];
        let names: Vec<&str> = sorted(&friends).iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["bob", "zed", "Abe", "Cy"]);
    }
}
