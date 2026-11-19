//! The battlestation's signup terminal, GUI.dll's `SignupTerminal_c` (FUN_100f499a, the module
//! `signupterminal_window`): the character's place in the queue for the battlestation, a side to pick, "Sign Up" and
//! "Leave Queue".
//!
//! * It is opened by the use of a terminal whose template names window kind 3 (the spell 0xcf7b's stat 39, Gamecode
//!   FUN_100a9cf4 -> GUI.dll FUN_1002e2fe; `client_window`), with no test of the character.
//! * The window is `Views/SignupTerminal.xml` in a window with one tab, "Sign Up" (not drawn here, as the other
//!   windows' single tabs are not), moved to the screen's middle. The `side` drop-down gets two entries, the texts
//!   8 and 9 of category 2005, numbered 0 and 1, and the one of the character's stat 0x29c (`BattlestationSide`)
//!   chosen. As it comes up it asks the server how the character stands (`N3Msg_GetInfo`, CharacterAction 0xfe).
//! * The server's `QueueUpdate` (`2c2f061c`, one number; Gamecode FUN_10131e18 hands it to FUN_100f47ae): below 0
//!   the window only says that the character is in the queue already; else `place` shows the number, and "Sign Up"
//!   and the drop-down are alive while it is 0 and dead otherwise.
//! * "Sign Up" (FUN_100f4595): with no side chosen it says so; with the character's stat 0x29e
//!   (`BattlestationRep`) below 1 it says the character was turned down; else the side is set (`N3Msg_SetStat` of
//!   0x29c) and the character joins the queue (`N3Msg_AddToQueue`, CharacterAction 0xfd with the side as its second
//!   parameter). "Leave Queue" is `N3Msg_LeaveQueue` (CharacterAction 0xff). "Close" closes the window.
//!
//! None of these messages is in a capture: the layouts are the decompile's. The three lines the window says are
//! English in the program; they are in this client's words here. Not read: what the drop-down's own change does.

use crate::theme::Palette;

/// What the window asks of the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Ask {
    #[default]
    None,
    /// How the character stands in the queue (as the window comes up).
    Info,
    /// The side set and the queue joined.
    Join(u32),
    Leave,
}

/// The character's side for the battlestation, and how it is thought of there.
const SIDE: u32 = 0x29c;
const REPUTE: u32 = 0x29e;

/// What "Sign Up" does with the side chosen and the character's stats: the queue joined, or the line said instead.
pub fn sign_up(side: Option<u32>, stats: &std::collections::BTreeMap<u32, i64>) -> Result<Ask, &'static str> {
    let Some(side) = side else { return Err("Choose a side before you sign up.") };
    if stats.get(&REPUTE).copied().unwrap_or(0) < 1 {
        return Err("The terminal knows you, and by what it has on record it turns you down.");
    }
    Ok(Ask::Join(side))
}

#[derive(Default)]
pub struct SignupWindow {
    pub open: bool,
    window: Option<crate::gui::window::XmlWindow>,
    /// The server's word on the place since the window came up, not yet shown.
    place: Option<i32>,
    asked: bool,
}

impl SignupWindow {
    /// The server's `QueueUpdate`.
    pub fn set_place(&mut self, place: i32) {
        self.place = Some(place);
    }

    fn load(&mut self, ctx: &egui::Context, stats: &std::collections::BTreeMap<u32, i64>) -> bool {
        let Some(mut w) = crate::gui::window::XmlWindow::load(ctx, "SignupTerminal", "Vehicle", "SignupTerminal.xml") else { return false };
        w.tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = w.root;
        if let Some(side) = w.tree.find(root, "side")
            && let Some(menu) = w.tree.control_mut::<crate::gui::dropdown::DropdownMenu>(side)
        {
            menu.clear();
            menu.append(&crate::gui::xml::ldb_text("#2005:8"));
            menu.append(&crate::gui::xml::ldb_text("#2005:9"));
            // (`SelectByID` of the stat's value: one of the two, else the choice stays as it is)
            if let Some(kept) = stats.get(&SIDE).copied().filter(|s| (0..2).contains(s)) {
                menu.select(Some(kept as usize));
            }
        }
        w.tree.relayout = true;
        self.window = Some(w);
        self.asked = false;
        true
    }

    /// Draws the window: the lines it says, and what it asks of the server.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, stats: &std::collections::BTreeMap<u32, i64>) -> (Vec<String>, Ask) {
        let mut lines = Vec::new();
        if !self.open {
            self.window = None;
            self.place = None;
            return (lines, Ask::None);
        }
        if self.window.is_none() && !self.load(ctx, stats) {
            self.open = false;
            return (lines, Ask::None);
        }
        let Some(w) = self.window.as_mut() else { return (lines, Ask::None) };
        let root = w.root;
        let mut ask = Ask::None;
        if !self.asked {
            self.asked = true;
            ask = Ask::Info;
        }
        // (FUN_100f47ae)
        if let Some(place) = self.place.take() {
            if place < 0 {
                lines.push("You are in the queue already.".to_string());
            } else {
                if let Some(text) = w.tree.find(root, "place").and_then(|id| w.tree.control_mut::<crate::gui::controls::TextView>(id)) {
                    text.set_text(&place.to_string());
                }
                for name in ["SignUp", "side"] {
                    if let Some(id) = w.tree.find(root, name) {
                        w.tree.get_mut(id).enabled = place == 0;
                    }
                }
                w.tree.relayout = true;
            }
        }
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
        let tree = &mut w.tree;
        let mut is_open = true;
        let middle = ctx.content_rect().center() - min / 2.0;
        crate::chrome::window(ctx, palette, "SignupTerminal", "Vehicle", &mut is_open, |win| win.default_size(min).resizable(false).default_pos(crate::layout::pos("SignupTerminal", middle)), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        for (id, signal) in std::mem::take(&mut w.tree.signals) {
            if signal != crate::gui::host::Signal::Clicked {
                continue;
            }
            match w.tree.get(id).name.as_str() {
                "SignUp" => {
                    let side = w.tree.find(root, "side").and_then(|id| w.tree.control_mut::<crate::gui::dropdown::DropdownMenu>(id)).and_then(|menu| menu.selection).map(|k| k as u32);
                    match sign_up(side, stats) {
                        Ok(join) => ask = join,
                        Err(line) => lines.push(line.to_string()),
                    }
                }
                "LeaveQueue" => ask = Ask::Leave,
                "Quit" => is_open = false,
                _ => {}
            }
        }
        if !is_open {
            self.open = false;
            self.window = None;
        }
        (lines, ask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_up_needs_a_side_and_a_good_name() {
        let stats = |repute: i64| [(REPUTE, repute)].into_iter().collect::<std::collections::BTreeMap<u32, i64>>();
        assert!(sign_up(None, &stats(5)).is_err());
        assert!(sign_up(Some(1), &stats(0)).is_err());
        assert!(sign_up(Some(1), &Default::default()).is_err());
        assert_eq!(sign_up(Some(1), &stats(1)), Ok(Ask::Join(1)));
        assert_eq!(sign_up(Some(0), &stats(40)), Ok(Ask::Join(0)));
    }
}
