//! The help window (`Ctrl+H`): the keys as they are bound now (what the options window shows, readable at a glance), the chat commands
//! this client carries out and the mouse controls.

use crate::chat_commands::IMPLEMENTED;
use crate::settings::{Keymap, ACTIONS};
use crate::theme::Palette;

/// `camera_rotate_left` as "Camera rotate left".
pub fn pretty(name: &str) -> String {
    let mut s = name.replace('_', " ");
    if let Some(first) = s.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    s
}

/// The actions that have keys, as (name, keys) rows in the table's order.
pub fn rows(keymap: &Keymap) -> Vec<(String, String)> {
    ACTIONS.iter().map(|(action, name, _)| (pretty(name), keymap.keys(*action))).filter(|(_, keys)| !keys.is_empty()).collect()
}

/// The mouse controls of the live view.
pub const MOUSE: &[(&str, &str)] = &[
    ("Left button, drag", "turn the camera around the character"),
    ("Right button, drag", "steer the character with the camera"),
    ("Both buttons", "walk forward"),
    ("Left click", "select what is under the cursor (a character, a machine, a door)"),
    ("Double click", "use what is under the cursor"),
    ("Scroll", "move the camera closer or away"),
];

#[derive(Default)]
pub struct HelpWindow {
    pub open: bool,
    filter: String,
}

impl HelpWindow {
    pub fn show(&mut self, ctx: &egui::Context, keymap: &Keymap, palette: &Palette) {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut open = self.open;
        crate::chrome::window(ctx, palette, "Help", "Help", &mut open, |w| w.default_size([420.0, 460.0]), |ui| {
            ui.horizontal(|ui| {
                ui.label("Find");
                crate::chrome::text_edit(ui, palette, egui::TextEdit::singleline(&mut self.filter).desired_width(200.0));
            });
            ui.separator();
            let filter = self.filter.trim().to_lowercase();
            let shown = |a: &str, b: &str| filter.is_empty() || a.to_lowercase().contains(&filter) || b.to_lowercase().contains(&filter);
            crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(400.0), |ui| {
                ui.label(egui::RichText::new("Keys").strong().color(accent));
                for (name, keys) in rows(keymap).iter().filter(|(a, b)| shown(a, b)) {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(keys).monospace());
                        });
                    });
                }
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Mouse").strong().color(accent));
                for (what, does) in MOUSE.iter().filter(|(a, b)| shown(a, b)) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(*what).monospace());
                        ui.label(*does);
                    });
                }
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Chat commands").strong().color(accent));
                for (usage, what) in IMPLEMENTED.iter().filter(|(a, b)| shown(a, b)) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(format!("/{usage}")).monospace());
                        ui.label(*what);
                    });
                }
            });
        });
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_names_read_as_words() {
        assert_eq!(pretty("camera_rotate_left"), "Camera rotate left");
        assert_eq!(pretty("use"), "Use");
    }

    #[test]
    fn only_bound_actions_are_listed() {
        let keymap = Keymap::default();
        let list = rows(&keymap);
        assert!(list.iter().any(|(name, keys)| name == "Forward" && keys.contains('W')));
        assert!(list.iter().all(|(_, keys)| !keys.is_empty()));
    }
}
