//! The looking-for-team window, GUI.dll's `lft_window` (the control center's Team menu "#FindTeam"): the skin's
//! `Views/LFTView.xml`, drop-downs for the side, the location and the profession, Search, the candidates (`CandidateView`),
//! "Invite to team" and "Send tell", and the character's own "Looking for team" flag with a description.
//!
//! The original keeps the list on the chat server; no capture has that traffic, so Search finds nobody and the flag and the
//! description are kept in the window only. The drop-downs are filled with the game's sides and professions, so the window
//! looks and works as the original's does up to the search.

use crate::theme::Palette;

/// The sides, as the original's drop-down lists them.
pub const SIDES: [&str; 4] = ["Any", "Neutral", "Clan", "Omni-Tek"];

/// The professions by their number (the stat's values 1 to 15; 13 is unused).
pub const PROFESSIONS: [&str; 15] = [
    "Soldier",
    "Martial Artist",
    "Engineer",
    "Fixer",
    "Agent",
    "Adventurer",
    "Trader",
    "Bureaucrat",
    "Enforcer",
    "Doctor",
    "Nano-Technician",
    "Meta-Physicist",
    "",
    "Keeper",
    "Shade",
];

#[derive(Default)]
pub struct LftWindow {
    pub open: bool,
    /// The character's own flag and description, as set in the window.
    pub looking: bool,
    pub description: String,
    window: Option<crate::gui::window::XmlWindow>,
}

/// Fills a drop-down with `entries` and selects the first.
fn fill(tree: &mut crate::gui::view::Tree, root: crate::gui::view::Id, name: &str, entries: &[&str]) {
    if let Some(id) = tree.find(root, name)
        && let Some(d) = tree.control_mut::<crate::gui::dropdown::DropdownMenu>(id)
    {
        d.clear();
        for e in entries {
            d.append(e);
        }
        d.select(Some(0));
    }
}

impl LftWindow {
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) {
        if !self.open {
            self.window = None;
            return;
        }
        if self.window.is_none() {
            let Some(mut w) = crate::gui::window::XmlWindow::load(ctx, "LFT", "Looking for team", "LFTView.xml") else {
                self.open = false;
                return;
            };
            w.tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            w.default_size = egui::vec2(520.0, 360.0);
            let root = w.root;
            fill(&mut w.tree, root, "Side", &SIDES);
            fill(&mut w.tree, root, "Location", &["Any"]);
            let professions: Vec<&str> = std::iter::once("Any").chain(PROFESSIONS.iter().copied().filter(|p| !p.is_empty())).collect();
            fill(&mut w.tree, root, "Profession", &professions);
            self.window = Some(w);
        }
        let Some(w) = self.window.as_mut() else { return };
        let signals = w.show(ctx, palette);
        for (id, signal) in signals {
            let name = w.tree.get(id).name.clone();
            match (name.as_str(), signal) {
                ("LFT", crate::gui::host::Signal::Toggled(on)) => self.looking = on,
                ("Description", crate::gui::host::Signal::Edited | crate::gui::host::Signal::Enter) => {
                    if let Some(t) = crate::gui::textinput::text(&w.tree, id) {
                        self.description = t.to_string();
                    }
                }
                // (Search, Invite, Tell: the list's messages are not known; nothing is sent)
                _ => {}
            }
        }
        self.open = w.open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MAO_GAME`: the game's `LFTView.xml` has the drop-downs, the buttons, the flag and the description.
    #[test]
    fn the_skin_has_the_lft_view() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let dir = std::path::Path::new(&game).join("cd_image/gui/Default");
        let (mut tree, root) = crate::gui::load_view(&[dir], "LFTView.xml", "").unwrap();
        for name in ["Side", "Location", "Profession", "Search", "CandidateView", "Invite", "Tell", "LFT", "Description"] {
            assert!(tree.find(root, name).is_some(), "{name}");
        }
        fill(&mut tree, root, "Side", &SIDES);
        let id = tree.find(root, "Side").unwrap();
        assert_eq!(tree.control::<crate::gui::dropdown::DropdownMenu>(id).unwrap().current_text(), "Any");
    }

    #[test]
    fn the_professions_are_numbered_as_the_stat_is() {
        assert_eq!(PROFESSIONS[6 - 1], "Adventurer");
        assert_eq!(PROFESSIONS[15 - 1], "Shade");
        assert_eq!(PROFESSIONS[13 - 1], "");
    }
}
