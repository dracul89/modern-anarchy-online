//! The knowledge window, GUI.dll's `KnowledgeView_c` (FUN_100cfa6c, `knowledge_window`, the mini toolbar's "Knowledge" with
//! the `GFX_GUI_SB_REF` picture: a reference board). It is a grid (`MultiListView_c` in grid mode) of item pictures
//! (`InventoryListViewItem_c`) the player drops items on to keep them at hand:
//!
//! * a drop (FUN_100cf299, a type-3 drag object): the item's identity goes to the cell under the pointer, or the first free
//!   one when that is taken; an item already on the board is moved there instead;
//! * a click (FUN_100cf49f) shows the item's page in the info view (`itemid://low/high`; a character, `charref://`);
//! * a right click (FUN_100cf89e) opens a two-entry menu: show the page (FUN_100cf583), take it off (FUN_100cf464 → FUN_100cf17f);
//! * the board is saved in `KnowledgeViewConfig` as the window closes (FUN_100cf66a: `item_id` and `item_pos` per item) and
//!   read back as it opens (FUN_100cf779).
//!
//! The client keeps the board in `knowledge.ini` beside `layout.ini` (`low high quality x y name` per line), takes drops from
//! the inventory window's drag ([`crate::gear::DRAGGED`]) and shows the pages through the info view. An item dropped keeps its
//! picture; one read back from the file shows its name until the inventory lends a picture again.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::gear::{GearItem, GearWindow, Icon};
use crate::theme::Palette;

/// The board's columns and rows (the original's grid grows; six by five fits its first window).
pub const COLUMNS: usize = 6;
pub const ROWS: usize = 5;
const CELL: f32 = 40.0;

/// An item on the board.
#[derive(Clone)]
pub struct Entry {
    pub low: u32,
    pub high: u32,
    pub quality: u32,
    pub name: String,
    pub icon: Option<Arc<Icon>>,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        (self.low, self.high, self.quality, &self.name) == (other.low, other.high, other.quality, &other.name)
    }
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entry({}/{} ql {} {:?})", self.low, self.high, self.quality, self.name)
    }
}

#[derive(Default)]
pub struct KnowledgeWindow {
    pub open: bool,
    pub cells: BTreeMap<(usize, usize), Entry>,
    loaded: bool,
    dirty: bool,
    /// The cell whose menu is open.
    menu: Option<(usize, usize)>,
}

fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("knowledge.ini")
}

/// (the tests and the previews never read or write the player's files)
fn disabled() -> bool {
    cfg!(test) || std::env::var_os("MAO_NO_LAYOUT").is_some() || std::env::var_os("MAO_UI_PREVIEW").is_some()
}

/// The file's lines: `low high quality x y name`.
pub fn parse(text: &str) -> BTreeMap<(usize, usize), Entry> {
    let mut cells = BTreeMap::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let (Some(low), Some(high), Some(quality), Some(x), Some(y)) = (words.next(), words.next(), words.next(), words.next(), words.next()) else { continue };
        let (Ok(low), Ok(high), Ok(quality), Ok(x), Ok(y)) = (low.parse(), high.parse(), quality.parse(), x.parse::<usize>(), y.parse::<usize>()) else { continue };
        if x < COLUMNS && y < ROWS {
            cells.insert((x, y), Entry { low, high, quality, name: words.collect::<Vec<_>>().join(" "), icon: None });
        }
    }
    cells
}

pub fn render(cells: &BTreeMap<(usize, usize), Entry>) -> String {
    cells.iter().map(|((x, y), e)| format!("{} {} {} {x} {y} {}\n", e.low, e.high, e.quality, e.name)).collect()
}

impl KnowledgeWindow {
    /// The first free cell, in rows.
    fn first_free(&self) -> Option<(usize, usize)> {
        (0..ROWS).flat_map(|y| (0..COLUMNS).map(move |x| (x, y))).find(|c| !self.cells.contains_key(c))
    }

    /// Puts an item on the board at `cell` (the first free one when `cell` is taken by another); an item already on the
    /// board moves instead. Whether it went on.
    pub fn drop(&mut self, cell: (usize, usize), entry: Entry) -> bool {
        let already = self.cells.iter().find(|(_, e)| e.low == entry.low && e.high == entry.high && e.quality == entry.quality).map(|(c, _)| *c);
        let target = if self.cells.contains_key(&cell) && already != Some(cell) { self.first_free() } else { Some(cell) };
        let Some(target) = target else { return false };
        if let Some(from) = already {
            let moved = self.cells.remove(&from).unwrap();
            self.cells.insert(target, moved);
        } else {
            self.cells.insert(target, entry);
        }
        self.dirty = true;
        true
    }

    pub fn remove(&mut self, cell: (usize, usize)) {
        if self.cells.remove(&cell).is_some() {
            self.dirty = true;
        }
    }

    fn load(&mut self) {
        self.loaded = true;
        if disabled() {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(path()) {
            self.cells = parse(&text);
        }
    }

    fn save(&mut self) {
        self.dirty = false;
        if !disabled() {
            let _ = std::fs::write(path(), render(&self.cells));
        }
    }

    /// Draws the board; the item pages asked for (`itemref://` URLs for the info view).
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, items: &[GearItem], palette: &Palette) -> Vec<String> {
        let mut pages = Vec::new();
        if !self.open {
            if self.dirty {
                self.save();
            }
            return pages;
        }
        if !self.loaded {
            self.load();
        }
        // (an item of the inventory lends its picture to a board entry read back without one)
        for e in self.cells.values_mut().filter(|e| e.icon.is_none()) {
            if let Some(item) = items.iter().find(|i| i.item_id == e.low) {
                e.icon = item.icon.clone();
                if e.name.is_empty() {
                    e.name = item.name.clone();
                }
            }
        }
        let accent = crate::chrome::rgb(palette.accent);
        let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
        let released = ctx.input(|i| i.pointer.any_released());
        let pointer = ctx.input(|i| i.pointer.interact_pos());
        let mut is_open = self.open;
        let mut drop_at = None;
        let mut clicked = None;
        let mut menu_action: Option<(usize, usize, bool)> = None;
        let menu = &mut self.menu;
        let cells = &self.cells;
        crate::chrome::window(ctx, palette, "Knowledge", "Knowledge", &mut is_open, |w| w.resizable(false).default_pos(crate::layout::pos("Knowledge", egui::pos2(80.0, 120.0))), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
            for y in 0..ROWS {
                ui.horizontal(|ui| {
                    for x in 0..COLUMNS {
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::click());
                        let painter = ui.painter();
                        painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(120));
                        painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(40)), egui::StrokeKind::Inside);
                        if let Some(e) = cells.get(&(x, y)) {
                            match &e.icon {
                                Some(icon) => {
                                    let id = gear.texture(ctx, icon);
                                    ui.painter().image(id, rect.shrink(3.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                                }
                                None => {
                                    let short: String = e.name.chars().take(3).collect();
                                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, if short.is_empty() { "?" } else { &short }, egui::FontId::proportional(12.0), accent);
                                }
                            }
                            let response = response.on_hover_text(if e.name.is_empty() { format!("Item {}", e.low) } else { format!("{} (QL {})", e.name, e.quality) });
                            if response.clicked() {
                                clicked = Some((x, y));
                            }
                            if response.secondary_clicked() {
                                *menu = Some((x, y));
                            }
                            if *menu == Some((x, y)) {
                                let mut keep = true;
                                egui::Area::new(egui::Id::unique(format!("knowledge-menu-{x}-{y}"))).fixed_pos(rect.right_top()).order(egui::Order::Foreground).show(ctx, |ui| {
                                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                                        if ui.button("Show information").clicked() {
                                            menu_action = Some((x, y, false));
                                            keep = false;
                                        }
                                        if ui.button("Remove").clicked() {
                                            menu_action = Some((x, y, true));
                                            keep = false;
                                        }
                                    });
                                });
                                if !keep || (released && pointer.is_some_and(|p| !rect.expand(140.0).contains(p))) {
                                    *menu = None;
                                }
                            }
                        }
                        if dragged != u32::MAX && released && pointer.is_some_and(|p| rect.contains(p)) {
                            drop_at = Some((x, y));
                        }
                    }
                });
            }
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Drag items from the inventory onto the board. Click one for its information.").small().weak());
        });
        if let Some(cell) = drop_at
            && let Some(item) = items.iter().find(|i| i.slot == dragged)
        {
            self.drop(cell, Entry { low: item.item_id, high: item.high_id, quality: item.quality, name: item.name.clone(), icon: item.icon.clone() });
            crate::gear::DRAGGED.store(u32::MAX, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(cell) = clicked
            && let Some(e) = self.cells.get(&cell)
        {
            pages.push(format!("itemref://{}/{}/{}", e.low, e.high, e.quality));
        }
        if let Some((x, y, remove)) = menu_action {
            if remove {
                self.remove((x, y));
            } else if let Some(e) = self.cells.get(&(x, y)) {
                pages.push(format!("itemref://{}/{}/{}", e.low, e.high, e.quality));
            }
        }
        if self.dirty {
            self.save();
        }
        self.open = is_open;
        pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(low: u32) -> Entry {
        Entry { low, high: low, quality: 1, name: format!("Item {low}"), icon: None }
    }

    #[test]
    fn drops_fill_cells_move_items_and_round_trip_through_the_file() {
        let mut w = KnowledgeWindow::default();
        assert!(w.drop((2, 1), entry(10)));
        // (a drop on a taken cell goes to the first free one)
        assert!(w.drop((2, 1), entry(11)));
        assert_eq!(w.cells.get(&(0, 0)).map(|e| e.low), Some(11));
        // (an item already on the board moves)
        assert!(w.drop((5, 4), entry(10)));
        assert_eq!(w.cells.get(&(5, 4)).map(|e| e.low), Some(10));
        assert!(!w.cells.contains_key(&(2, 1)));
        let text = render(&w.cells);
        let back = parse(&text);
        assert_eq!(back.len(), 2);
        assert_eq!(back.get(&(5, 4)).map(|e| (e.low, e.name.as_str())), Some((10, "Item 10")));
        w.remove((0, 0));
        assert_eq!(w.cells.len(), 1);
        // (a full board refuses)
        for y in 0..ROWS {
            for x in 0..COLUMNS {
                w.drop((x, y), entry(100 + (y * COLUMNS + x) as u32));
            }
        }
        assert_eq!(w.cells.len(), COLUMNS * ROWS);
        assert!(!w.drop((0, 0), entry(999)));
    }
}
