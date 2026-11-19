//! The laser target and airstrike window, GUI.dll's `LaserTarget_c` (FUN_100f2e32; the modules `lasertarget_window`
//! and `airstriketarget_window`): the players marked with a laser tag, one to be picked for an attack.
//!
//! * It is opened by the use of a terminal whose template names window kind 4 (the orbital attack's) or 5 (the
//!   airstrike's), for a character whose stat 389 has bit 0x20 (GUI.dll FUN_1002e2fe; `client_window`). The class
//!   has a third way of working, the artillery's, which nothing opens.
//! * The window is `Views/LaserTarget.xml`, 500 by 420 in the screen's middle, with one tab, "Target List" (not drawn
//!   here). `TargetView` holds a list of three columns: the player's name (180 wide), the area (180) and the level
//!   range (80), each row a tag of the server's list (`mao_net::notum::laser_tags`): the range is its two numbers
//!   as `%d-%d`, and the rows sort by their texts. A click on a row selects it. A new list keeps the selected tag
//!   selected (FUN_100f2cd0).
//! * The list is asked for as the window comes up, by "Refresh", and every 30 seconds (`N3Msg_RefreshLaserTags`).
//! * The attack button (FUN_100f2c4a), with a row selected: the orbital attack or the airstrike on that tag. With
//!   none it does nothing. The window stays up. "Close" closes it.
//!
//! None of its messages is in a capture, and no terminal that opens it was found in the game's data. The columns'
//! titles are English in the program; they are in this client's words here. Not ported: the columns' widths kept
//! (`LaserTargetConfig`).

use std::sync::Mutex;

use mao_net::notum::LaserTag;

use crate::theme::Palette;

/// What the window asks of the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Refresh,
    /// The attack on a tag: whether it is the airstrike (else the orbital attack), and the tag's number.
    Attack(bool, u32),
}

static COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());

pub fn take_commands() -> Vec<Command> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn send(c: Command) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(c);
}

/// How often the list is asked for again.
const REFRESH: std::time::Duration = std::time::Duration::from_secs(30);

/// A row's three texts.
pub fn row(tag: &LaserTag) -> [String; 3] {
    [tag.name.clone(), tag.area.clone(), format!("{}-{}", tag.levels.0, tag.levels.1)]
}

#[derive(Default)]
pub struct LaserWindow {
    pub open: bool,
    /// The airstrike's window (kind 5), else the orbital attack's (kind 4).
    pub airstrike: bool,
    tags: Vec<LaserTag>,
    window: Option<crate::gui::window::XmlWindow>,
    list: Option<crate::gui::view::Id>,
    shown: bool,
    asked: Option<std::time::Instant>,
}

impl LaserWindow {
    /// The window of a kind comes up (a terminal's use).
    pub fn show_kind(&mut self, airstrike: bool) {
        self.airstrike = airstrike;
        self.open = true;
    }

    /// The server's list.
    pub fn set(&mut self, tags: Vec<LaserTag>) {
        self.tags = tags;
        self.shown = false;
    }

    /// A sample for a headless preview.
    pub fn sample() -> LaserWindow {
        let mut w = LaserWindow { open: true, ..Default::default() };
        w.set(vec![
            LaserTag { id: 11, playfield: 585, name: "Somebody".into(), area: "A tower field".into(), levels: (100, 150) },
            LaserTag { id: 12, playfield: 620, name: "Another".into(), area: "Elsewhere".into(), levels: (1, 220) },
        ]);
        w
    }

    fn load(&mut self, ctx: &egui::Context) -> bool {
        let Some(mut w) = crate::gui::window::XmlWindow::load(ctx, "LaserTarget", "Vehicle", "LaserTarget.xml") else { return false };
        w.tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = w.root;
        let Some(target) = w.tree.find(root, "TargetView") else { return false };
        use crate::gui::multilist::{self, MOVABLE, RESIZABLE, SORTABLE};
        let list = multilist::create(&mut w.tree, "Targets");
        multilist::add_column(&mut w.tree, list, 0, "Player", 180.0, RESIZABLE | MOVABLE | SORTABLE);
        multilist::add_column(&mut w.tree, list, 1, "Area", 180.0, RESIZABLE | MOVABLE | SORTABLE);
        multilist::add_column(&mut w.tree, list, 2, "Levels", 80.0, RESIZABLE | MOVABLE | SORTABLE);
        w.tree.get_mut(target).layout = crate::gui::view::Layout::Base;
        w.tree.add_child(target, list);
        self.list = Some(list);
        self.window = Some(w);
        self.shown = false;
        self.asked = None;
        true
    }

    /// Draws the window.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) {
        if !self.open {
            self.window = None;
            return;
        }
        if self.window.is_none() && !self.load(ctx) {
            self.open = false;
            return;
        }
        // (asked for as it comes up and every 30 seconds after)
        if self.asked.is_none_or(|at| at.elapsed() >= REFRESH) {
            self.asked = Some(std::time::Instant::now());
            send(Command::Refresh);
        }
        let (Some(w), Some(list)) = (self.window.as_mut(), self.list) else { return };
        let root = w.root;
        if !self.shown {
            use crate::gui::multilist::{CellView, Item};
            // (the selected tag stays selected in a new list)
            let kept = crate::gui::multilist::selected(&w.tree, list);
            let items = self
                .tags
                .iter()
                .map(|t| {
                    let texts = row(t);
                    Item { id: i64::from(t.id), cells: texts.iter().enumerate().map(|(k, text)| (k as i32, CellView::Text(text.clone()))).collect(), keys: texts.iter().enumerate().map(|(k, text)| (k as i32, text.clone())).collect(), ..Default::default() }
                })
                .collect();
            crate::gui::multilist::set_items(&mut w.tree, list, items);
            crate::gui::multilist::select(&mut w.tree, list, kept.filter(|id| self.tags.iter().any(|t| i64::from(t.id) == *id)));
            w.tree.relayout = true;
            self.shown = true;
        }
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
        let tree = &mut w.tree;
        let mut is_open = true;
        // (`Rect(200, 180, 700, 600)` and `MoveToCenter`)
        let size = min.max(egui::vec2(500.0, 420.0));
        let middle = ctx.content_rect().center() - size / 2.0;
        crate::chrome::window(ctx, palette, "LaserTarget", "Vehicle", &mut is_open, |win| win.default_size(size).resizable(true).default_pos(crate::layout::pos("LaserTarget", middle)), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        for (_, id, _) in crate::gui::multilist::take_signals(&mut w.tree) {
            crate::gui::multilist::select(&mut w.tree, list, Some(id));
        }
        for (id, signal) in std::mem::take(&mut w.tree.signals) {
            if signal != crate::gui::host::Signal::Clicked {
                continue;
            }
            match w.tree.get(id).name.as_str() {
                "Refresh" => {
                    self.asked = Some(std::time::Instant::now());
                    send(Command::Refresh);
                }
                "Attack" => {
                    if let Some(tag) = crate::gui::multilist::selected(&w.tree, list) {
                        send(Command::Attack(self.airstrike, tag as u32));
                    }
                }
                "Quit" => is_open = false,
                _ => {}
            }
        }
        if !is_open {
            self.open = false;
            self.window = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_the_name_the_area_and_the_range() {
        let tag = LaserTag { id: 5, playfield: 585, name: "Somebody".into(), area: "A field".into(), levels: (100, 150) };
        assert_eq!(row(&tag), ["Somebody".to_string(), "A field".to_string(), "100-150".to_string()]);
        let mut w = LaserWindow::default();
        w.show_kind(true);
        assert!(w.open && w.airstrike);
    }
}
