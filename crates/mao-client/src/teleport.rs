//! The teleport target window, GUI.dll's `TeleportTarget_c` (FUN_10100022, the module `teleport_target_window`): the
//! places a grid exit leads to, one to be picked. The server sends the list (`GridDestinationSelectIIR_t`,
//! `mao_net::grid`; Gamecode's handler FUN_10129529 keeps it and turns the window on), as it does at the Grid's
//! "Organisation City" and "Notum Controller" exits.
//!
//! * The window is `Views/TeleportTarget.xml` in a window the original titles "Vehicle" (its constructor's second
//!   string; the first, the name, is empty), 500 by 420 and moved to the screen's middle, with one tab, "Target List"
//!   (not drawn here, as the other windows' single tabs are not).
//! * `TargetView` holds a list (`MultiListView_c` in list mode) of two columns, "Location" and "Playfield", 200 wide
//!   each (flags 0xe: resizable, movable, sortable). A row per entry of the list (`TeleportTargetItem_c`,
//!   FUN_101006c9): the entry's name and its playfield's (`N3Msg_GetPFName`, "unknown pf" without one). A click on a
//!   row selects it (FUN_100ffec8); nothing else does anything.
//! * "Go!" (FUN_100fffb8), with a row selected: `N3Msg_GridDestinationSelected(row, the row's playfield, its
//!   identity)`, and then the window's close request, as "Close" and the frame's cross make it.
//! * The close request (FUN_100ffedf), while the module is on: `N3Msg_GridDestinationSelected(-1, 0, (0, 0))` and the
//!   module off. So a "Go!" sends two answers, the pick and then the one that says none: the original does, and so
//!   does this (the network loop sends the second after a pick).

use crate::theme::Palette;

/// A row: the entry's playfield and identity, its name and the playfield's.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub playfield: u32,
    pub identity: (u32, u32),
    pub location: String,
    pub playfield_name: String,
}

/// The rows of a list, the playfields named.
pub fn rows(list: &mao_net::grid::List, names: &std::collections::HashMap<u32, String>) -> Vec<Row> {
    list.entries.iter().map(|d| Row { playfield: d.playfield, identity: d.identity, location: d.name.clone(), playfield_name: names.get(&d.playfield).cloned().unwrap_or_else(|| "unknown pf".into()) }).collect()
}

/// What the window answers as it closes: the row "Go!" picked (its number in the list, its playfield and identity), or
/// none.
pub type Answer = Option<(u32, u32, (u32, u32))>;

#[derive(Default)]
pub struct TeleportWindow {
    pub open: bool,
    pub rows: Vec<Row>,
    window: Option<crate::gui::window::XmlWindow>,
    list: Option<crate::gui::view::Id>,
    shown: bool,
}

impl TeleportWindow {
    /// A list from the server: the window comes up with it.
    pub fn set(&mut self, rows: Vec<Row>) {
        self.rows = rows;
        self.open = true;
        self.shown = false;
    }

    /// A sample for a headless preview.
    pub fn sample() -> TeleportWindow {
        let mut w = TeleportWindow::default();
        w.set(vec![
            Row { playfield: 6010, identity: (0xc419, 0xc000), location: "A city".into(), playfield_name: "A playfield".into() },
            Row { playfield: 585, identity: (0xc419, 0xc001), location: "A tower field".into(), playfield_name: "Another playfield".into() },
        ]);
        w
    }

    fn load(&mut self, ctx: &egui::Context) -> bool {
        let Some(mut w) = crate::gui::window::XmlWindow::load(ctx, "TeleportTarget", "Vehicle", "TeleportTarget.xml") else { return false };
        w.tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = w.root;
        let Some(target) = w.tree.find(root, "TargetView") else { return false };
        use crate::gui::multilist::{self, MOVABLE, RESIZABLE, SORTABLE};
        let list = multilist::create(&mut w.tree, "Targets");
        multilist::add_column(&mut w.tree, list, 0, "Location", 200.0, RESIZABLE | MOVABLE | SORTABLE);
        multilist::add_column(&mut w.tree, list, 1, "Playfield", 200.0, RESIZABLE | MOVABLE | SORTABLE);
        // (the view's `LayoutNode`: the list over all of it)
        w.tree.get_mut(target).layout = crate::gui::view::Layout::Base;
        w.tree.add_child(target, list);
        self.list = Some(list);
        self.window = Some(w);
        self.shown = false;
        true
    }

    /// Draws the window; its answer as it closes.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<Answer> {
        if !self.open {
            self.window = None;
            return None;
        }
        if self.window.is_none() && !self.load(ctx) {
            // (no view to show the list on: as the original, which shows no window then and sends nothing)
            self.open = false;
            return None;
        }
        let (Some(w), Some(list)) = (self.window.as_mut(), self.list) else { return None };
        let mut picked = None;
        let root = w.root;
        if !self.shown {
            use crate::gui::multilist::{CellView, Item};
            let items = self
                .rows
                .iter()
                .enumerate()
                .map(|(i, r)| Item { id: i as i64, cells: vec![(0, CellView::Text(r.location.clone())), (1, CellView::Text(r.playfield_name.clone()))], keys: vec![(0, r.location.clone()), (1, r.playfield_name.clone())], ..Default::default() })
                .collect();
            crate::gui::multilist::set_items(&mut w.tree, list, items);
            crate::gui::multilist::select(&mut w.tree, list, None);
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
        crate::chrome::window(ctx, palette, "TeleportTarget", "Vehicle", &mut is_open, |win| win.default_size(size).resizable(true).default_pos(crate::layout::pos("TeleportTarget", middle)), |ui| {
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
                "Go" => {
                    picked = crate::gui::multilist::selected(&w.tree, list).and_then(|i| self.rows.get(i as usize).map(|r| (i as u32, r.playfield, r.identity)));
                    is_open = picked.is_none();
                }
                "Quit" => is_open = false,
                _ => {}
            }
        }
        if is_open {
            return None;
        }
        self.open = false;
        self.window = None;
        Some(picked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_names_its_playfields() {
        let list = mao_net::grid::List {
            entries: vec![
                mao_net::grid::Destination { playfield: 6010, identity: (0xc419, 0xc000), name: "A city".into(), words: [0, 0] },
                mao_net::grid::Destination { playfield: 9, identity: (0xc419, 0xc001), name: "A field".into(), words: [0, 0] },
            ],
            token: vec![1],
        };
        let names = [(6010, "Somewhere".to_string())].into_iter().collect();
        let rows = rows(&list, &names);
        assert_eq!(rows.iter().map(|r| (r.location.as_str(), r.playfield_name.as_str())).collect::<Vec<_>>(), [("A city", "Somewhere"), ("A field", "unknown pf")]);
        let mut w = TeleportWindow::default();
        assert!(!w.open);
        w.set(rows);
        assert!(w.open && w.rows.len() == 2);
    }
}
