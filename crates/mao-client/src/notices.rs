//! The notice buttons of a new level (GUI.dll's `NotifyButton`, FUN_1005370d): a picture near the screen's lower right
//! that opens the skill, perk or research window.
//!
//! * **Set** by Gamecode as distributed values, for the player's own character only: a new level (`NewLevel`,
//!   FUN_10075f55) sets `got_ip` always, `got_perk` at every tenth level when the account has Shadowlands (stat 389,
//!   `Expansion`, & 2) and `got_tech` at level 5 when it has Lost Eden (& 0x20); a new shadow level (`ShadowLevel`,
//!   FUN_10077ae7) sets `got_ip` and `got_perk`.
//! * **Shown** by `GUIModuleActivator_c`'s watchers of the three values (FUN_1002e9d4, FUN_1002eb2d, FUN_1002ec86):
//!   set, with the preference `ShowIPPerkbutton` off the value is put back to false (and its button goes); else, unless
//!   its button is up already, the button is made: `GFX_GUI_NEW_IP`, `GFX_GUI_NEW_PERK` or `GFX_GUI_NEW_TECH` (AFCM's
//!   pictures 0xaf, 0xb0, 0xb1) as its three pictures, its corner at the screen's size less (60, 273), (60, 333) or
//!   (60, 393). `got_tech` is put back to false instead when the research window is open. Put back to false, the
//!   button goes.
//! * **A click** (FUN_100536a6) sets the window's own value (`skill_window`, `perk_window`, `research_window`): the
//!   window opens. **The window's making puts the value back** (the skill window's rows, FUN_100fef4c: `got_ip`; the
//!   perk window, FUN_10061a07: `got_perk`; the research window: `got_tech`), so the button stays until that window
//!   is next opened, by the button or otherwise.

/// The three notices: the value's name, the client's window (`Ui::set_window`), the picture and how far above the
/// screen's bottom the button's top is.
pub const NOTICES: [(&str, &str, &str, f32); 3] = [
    ("got_ip", "skills", "GFX_GUI_NEW_IP", 273.0),
    ("got_perk", "perks", "GFX_GUI_NEW_PERK", 333.0),
    ("got_tech", "research", "GFX_GUI_NEW_TECH", 393.0),
];

/// How far left of the screen's right edge the buttons' left is.
const FROM_RIGHT: f32 = 60.0;

pub const IP: usize = 0;
pub const PERK: usize = 1;
pub const TECH: usize = 2;

/// The notices a new level sets (FUN_10075f55): `expansion` is stat 389.
pub fn new_level(level: i64, expansion: i64) -> Vec<usize> {
    let mut out = vec![IP];
    if level % 10 == 0 && expansion & 2 != 0 {
        out.push(PERK);
    }
    if level == 5 && expansion & 0x20 != 0 {
        out.push(TECH);
    }
    out
}

/// The notices a new shadow level sets (FUN_10077ae7).
pub fn shadow_level() -> Vec<usize> {
    vec![IP, PERK]
}

#[derive(Default)]
pub struct Notices {
    /// Each value: whether it is set, which is whether its button is up.
    up: [bool; 3],
    /// Whether each window was open last frame (its making is its opening).
    was_open: [bool; 3],
}

impl Notices {
    /// A value set (the watchers FUN_1002e9d4, FUN_1002eb2d, FUN_1002ec86): `shown` is the preference
    /// `ShowIPPerkbutton`, `research_open` whether the research window is open.
    pub fn set(&mut self, which: usize, shown: bool, research_open: bool) {
        self.up[which] = shown && !(which == TECH && research_open);
    }

    #[cfg(test)]
    fn is_up(&self, which: usize) -> bool {
        self.up[which]
    }

    /// Each frame: a window opened since the last puts its value back; the buttons up are drawn. `open` is whether each
    /// window is open now. The window a button was clicked for is given back, to be opened.
    pub fn frame(&mut self, ctx: &egui::Context, open: [bool; 3]) -> Option<&'static str> {
        for (i, &now) in open.iter().enumerate() {
            if now && !self.was_open[i] {
                self.up[i] = false;
            }
            self.was_open[i] = now;
        }
        let screen = ctx.content_rect();
        let skin = crate::skin::skin(ctx);
        let mut clicked = None;
        for (i, &(_, window, picture, above)) in NOTICES.iter().enumerate() {
            if !self.up[i] {
                continue;
            }
            let size = skin.as_ref().and_then(|s| s.get(picture)).map(|s| s.size).unwrap_or(egui::vec2(32.0, 32.0));
            let at = egui::pos2(screen.max.x - FROM_RIGHT, screen.max.y - above);
            egui::Area::new(egui::Id::unique(("level-notice", i))).fixed_pos(at).order(egui::Order::Foreground).show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                let drawn = skin.as_ref().is_some_and(|s| s.draw_in(ui.painter(), picture, rect, egui::Color32::WHITE));
                if !drawn {
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "!", egui::FontId::proportional(20.0), egui::Color32::YELLOW);
                }
                if response.clicked() {
                    clicked = Some(window);
                }
            });
        }
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `NewLevel` (FUN_10075f55): IP always, a perk every tenth level with Shadowlands, research at 5 with Lost Eden.
    #[test]
    fn a_new_level_sets_its_notices() {
        assert_eq!(new_level(7, 0xff), vec![IP]);
        assert_eq!(new_level(10, 0), vec![IP]);
        assert_eq!(new_level(10, 2), vec![IP, PERK]);
        assert_eq!(new_level(200, 2), vec![IP, PERK]);
        assert_eq!(new_level(5, 0x20), vec![IP, TECH]);
        assert_eq!(new_level(5, 2), vec![IP]);
        assert_eq!(new_level(15, 0x22), vec![IP]);
        assert_eq!(shadow_level(), vec![IP, PERK]);
    }

    /// The watchers: no button with the preference off; `got_tech` not while the research window is open; the
    /// window's opening takes its button away.
    #[test]
    fn a_notice_stands_until_its_window_opens() {
        let ctx = egui::Context::default();
        let mut n = Notices::default();
        n.set(IP, false, false);
        assert!(!n.is_up(IP));
        n.set(IP, true, false);
        n.set(TECH, true, true);
        assert!(n.is_up(IP) && !n.is_up(TECH));
        n.set(TECH, true, false);
        // (a window already open when the value is set: the button stays until it is opened again)
        let _ = ctx.run_ui(Default::default(), |_| {});
        let once = |n: &mut Notices, open| {
            let mut out = None;
            let _ = ctx.run_ui(Default::default(), |ui| out = n.frame(ui.ctx(), open));
            out
        };
        once(&mut n, [false, false, true]);
        assert!(n.is_up(IP) && !n.is_up(TECH));
        n.set(TECH, true, false);
        once(&mut n, [false, false, true]);
        assert!(n.is_up(TECH));
        once(&mut n, [true, false, false]);
        assert!(!n.is_up(IP) && n.is_up(TECH));
    }
}
