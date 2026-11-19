//! The battlestation's view, GUI.dll's `BattlestationView_c` (FUN_100f8967, the module `battlestation_window`): a
//! small map of the battlestation with its points, each in its holder's colour, the two sides' scores and where the
//! character is; and the two boxes of a battle, the invitation and the end.
//!
//! * The server's `ArriveAtBs` turns the view on, gives it the points' places and closes the signup terminal
//!   (Gamecode FUN_10131030); `BattleOver` turns it off (FUN_10131266). As it is made the view asks for the points
//!   (`N3Msg_GetPointLocations`).
//! * The view is a square a tenth of the screen's width, at the middle of the top, 30 down, without a frame: the
//!   picture `GFX_GUI_BS_MAIN` over all of it; a star a tenth of that for each point, made once, at its place on the
//!   map (FUN_100f80c8: x from 292 to 1696 across, z from 1653 down to 442), showing nothing until a score names its
//!   side: `GFX_GUI_BS_REDSTAR`, `_BLUESTAR`, `_WHITESTAR` for 0, 1, 2; the dot `GFX_GUI_MAPDOT_GREEN` at the
//!   character's own place, moved every frame (FUN_100f823d); and the two scores at half its height, the clan's at
//!   the left in red and Omni-Tek's at the right in light blue, "000" until a score comes.
//! * `SendScore` (FUN_10132085 -> FUN_100f8758): the clan's score is the first number, Omni-Tek's the second; each
//!   point's side is set when the list of sides is as long as the points are many.
//! * `InfromPlayer` (FUN_1013146a -> `GuiSystem_c::BattleStationSpot`, GUI.dll 0x100306f0): the signup terminal
//!   closes and a Yes/No box asks whether to go; it goes by itself after 30 seconds. Yes answers with the
//!   invitation's identity (`N3Msg_GoToBattle`); anything else answers nothing.
//! * `BattleOver` -> `GuiSystem_c::BattleFinished` (0x10030e3f): a box that tells how it ended and the character's
//!   points, with one button, which leaves the battle (`N3Msg_LeaveBattle`).
//!
//! None of these messages is in a capture. The boxes' lines are English in the program; they are in this client's
//! words here. Not ported: `BattleOver`'s release of two of the character's state flags and its setting of the
//! character's stat 27 to 40000 (what they are for was not read), the view's place kept (`BattlestationConfig`), and
//! whether the end's box, left alone for its 30 seconds, leaves the battle by itself (here it does not).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::theme::Palette;

/// What the server said.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The battlestation's points: the view comes up.
    Arrived(Vec<[f32; 3]>),
    /// The sides' scores and each point's side.
    Score(Vec<i32>, Vec<i32>),
    /// The battle's end: the character's points, and how it ended.
    Over(i32, i32),
    /// A place in a battle is free.
    Invited((u32, u32)),
}

/// What the view asks of the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Points,
    Accept((u32, u32)),
    Leave,
}

static COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());

pub fn take_commands() -> Vec<Command> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn send(c: Command) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(c);
}

/// How long a box waits for its answer.
const BOX_TIME: Duration = Duration::from_secs(30);

/// A world place on a map `size` wide and high (FUN_100f80c8).
pub fn map_place(at: [f32; 3], size: f32) -> [f32; 2] {
    [(at[0] - 292.0) / (1696.0 - 292.0) * size, (1653.0 - at[2]) / (1653.0 - 442.0) * size]
}

/// A side's star.
fn star(side: i32) -> Option<&'static str> {
    ["GFX_GUI_BS_REDSTAR", "GFX_GUI_BS_BLUESTAR", "GFX_GUI_BS_WHITESTAR"].get(usize::try_from(side).ok()?).copied()
}

/// The end's line, by how the battle ended, and the character's points.
pub fn finished_text(points: i32, how: i32) -> String {
    let outcome = match how {
        1 => "The blue side holds the battlestation. ",
        0 => "The red side holds the battlestation. ",
        4 => "The red side had too few fighters left: the blue side has won. ",
        3 => "The blue side had too few fighters left: the red side has won. ",
        _ => "",
    };
    format!("{outcome}You have {points} points. Leave the battlestation?")
}

#[derive(Default)]
pub struct BattleWindow {
    pub open: bool,
    points: Vec<[f32; 3]>,
    sides: Vec<i32>,
    scores: Option<(i32, i32)>,
    asked: bool,
    invitation: Option<((u32, u32), Instant)>,
    finished: Option<((i32, i32), Instant)>,
}

impl BattleWindow {
    /// The server's word; whether the signup terminal is to close with it.
    pub fn apply(&mut self, event: Event) -> bool {
        match event {
            Event::Arrived(points) => {
                if !self.open {
                    (self.points, self.sides, self.scores, self.asked) = (Vec::new(), Vec::new(), None, false);
                }
                self.open = true;
                // (the stars are made once: a second list changes nothing)
                if self.points.is_empty() {
                    self.points = points;
                }
                true
            }
            Event::Score(scores, sides) => {
                if let [clan, omni, ..] = scores[..] {
                    self.scores = Some((clan, omni));
                }
                if !sides.is_empty() && sides.len() == self.points.len() {
                    self.sides = sides;
                }
                false
            }
            Event::Over(points, how) => {
                self.open = false;
                self.finished = Some(((points, how), Instant::now()));
                false
            }
            Event::Invited(identity) => {
                self.invitation = Some((identity, Instant::now()));
                true
            }
        }
    }

    /// A sample for a headless preview.
    pub fn sample() -> BattleWindow {
        let mut w = BattleWindow::default();
        w.apply(Event::Arrived(vec![[600.0, 0.0, 1300.0], [1000.0, 0.0, 1050.0], [1400.0, 0.0, 800.0]]));
        w.apply(Event::Score(vec![120, 340], vec![0, 2, 1]));
        w
    }

    /// Draws the view and the boxes; `own` is where the character stands.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, own: Option<[f32; 3]>) {
        if let Some((identity, since)) = self.invitation {
            if since.elapsed() >= BOX_TIME {
                self.invitation = None;
            } else if let Some(yes) = crate::chrome::question(ctx, palette, "BattleStationSpot", "Battlestation", "A place in the battle at the battlestation is free. Do you want to go there?") {
                if yes {
                    send(Command::Accept(identity));
                }
                self.invitation = None;
            }
        }
        if let Some(((points, how), since)) = self.finished {
            let ok = crate::gui::xml::ldb_named(10000, "MsgBox_OK", "OK");
            if since.elapsed() >= BOX_TIME {
                self.finished = None;
            } else if let Some(answer) = crate::chrome::dialog(ctx, palette, "BattleFinished", "The battle is over", &finished_text(points, how), &[&ok]) {
                if answer == Some(0) {
                    send(Command::Leave);
                }
                self.finished = None;
            }
        }
        if !self.open {
            return;
        }
        if !self.asked {
            self.asked = true;
            send(Command::Points);
        }
        let Some(skin) = crate::skin::skin(ctx) else { return };
        let screen = ctx.content_rect();
        let size = (screen.width() / 10.0).round();
        let min = egui::pos2((screen.center().x - size / 2.0).round(), 30.0);
        let (points, sides, scores) = (&self.points, &self.sides, self.scores);
        egui::Area::new(egui::Id::unique("Battlestation")).order(egui::Order::Middle).fixed_pos(min).interactable(false).show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
            let painter = ui.painter();
            skin.draw_in(painter, "GFX_GUI_BS_MAIN", rect, egui::Color32::WHITE);
            let star_size = egui::vec2(size / 10.0, size / 10.0);
            for (k, at) in points.iter().enumerate() {
                if let Some(name) = sides.get(k).and_then(|s| star(*s)) {
                    let p = map_place(*at, size);
                    skin.draw_in(painter, name, egui::Rect::from_center_size(rect.min + egui::vec2(p[0], p[1]), star_size), egui::Color32::WHITE);
                }
            }
            if let (Some(at), Some(dot)) = (own, skin.get("GFX_GUI_MAPDOT_GREEN")) {
                let p = map_place(at, size);
                painter.image(dot.texture.id(), egui::Rect::from_center_size(rect.min + egui::vec2(p[0], p[1]), dot.size), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            let text = |n: Option<i32>| n.map_or("000".to_string(), |n| n.to_string());
            let font = egui::FontId::proportional(13.0);
            painter.text(egui::pos2(rect.left() + 5.0, rect.center().y), egui::Align2::LEFT_TOP, text(scores.map(|s| s.0)), font.clone(), egui::Color32::from_rgb(0xff, 0x00, 0x00));
            painter.text(egui::pos2(rect.right() - 5.0, rect.center().y), egui::Align2::RIGHT_TOP, text(scores.map(|s| s.1)), font, egui::Color32::from_rgb(0x42, 0xdb, 0xfb));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_on_the_map_is_by_the_stations_bounds() {
        assert_eq!(map_place([292.0, 0.0, 1653.0], 200.0), [0.0, 0.0]);
        assert_eq!(map_place([1696.0, 9.0, 442.0], 200.0), [200.0, 200.0]);
        let middle = map_place([994.0, 0.0, 1047.5], 100.0);
        assert!((middle[0] - 50.0).abs() < 1e-3 && (middle[1] - 50.0).abs() < 1e-3);
    }

    #[test]
    fn the_view_follows_the_servers_words() {
        let mut w = BattleWindow::default();
        // (the arrival turns it on and closes the signup terminal; the points are taken once)
        assert!(w.apply(Event::Arrived(vec![[300.0, 0.0, 500.0], [400.0, 0.0, 600.0]])));
        assert!(w.open && w.points.len() == 2);
        w.apply(Event::Arrived(vec![[1.0, 2.0, 3.0]]));
        assert_eq!(w.points.len(), 2);
        // (a score: the clan's first, Omni-Tek's second; the sides only when there is one for each point)
        assert!(!w.apply(Event::Score(vec![5, 9], vec![1])));
        assert_eq!((w.scores, w.sides.len()), (Some((5, 9)), 0));
        w.apply(Event::Score(vec![6, 9], vec![1, 0]));
        assert_eq!((w.scores, w.sides.clone()), (Some((6, 9)), vec![1, 0]));
        assert_eq!((star(0), star(2), star(3), star(-1)), (Some("GFX_GUI_BS_REDSTAR"), Some("GFX_GUI_BS_WHITESTAR"), None, None));
        // (the end turns it off and asks about leaving; an invitation closes the signup terminal)
        assert!(!w.apply(Event::Over(40, 1)));
        assert!(!w.open && w.finished.is_some());
        assert!(finished_text(40, 1).contains("40 points") && finished_text(3, 9).starts_with("You have 3"));
        assert!(w.apply(Event::Invited((0xc350, 9))));
        assert!(w.invitation.is_some());
    }
}
