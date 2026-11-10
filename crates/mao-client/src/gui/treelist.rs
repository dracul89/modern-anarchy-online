//! `ListViewBase_c`: a list of rows that can be folders of rows (the friends window's Chat Windows / Online / Offline /
//! Recent lists), each row a `StringListViewItem_c`.
//!
//! An item (`ListViewBaseItem_c` 0x101323e8) is as tall as its view and indented 15 (`+0x58`, `_DAT_101b00e8`) past its
//! parent's place (`_Layout` 0x10131af9: the sum of the parents' indents, so the top rows sit 15 in); the rows are one
//! pixel apart (`_FindVisibleItems` 0x10131983). A folder's children show while it is open (`OpenFolder` 0x101326fe).
//! A row's view (`StrLstItemView_c` FUN_10144ecc) is its picture (`+0x90`; a folder that is closed, or an item not
//! selected, shows the second picture `+0x94` when it has one: FUN_10144c68), 5 of room, and its label in font 5: white
//! (`+0x84`) when selected or not selectable, else 0xc0c0c0 (`+0x88`), the colours `SetLabelColor` changes. A flashing
//! row's picture blinks every half second (`FlashIcon`).
//!
//! A press on a row (`MouseDown` 0x101313ad) opens or closes a folder, selects the row when the list selects on clicks
//! (`+0x155`; the friends window does not) and is told to the window with its click count.

use std::time::Instant;

use super::font::{FontId, Measure};
use super::geom::Point;
use super::host::{Mouse, Paint, Signal};
use super::view::Control;

/// The indent of a level.
pub const INDENT: f32 = 15.0;
/// The room after a picture.
const GAP: f32 = 5.0;

#[derive(Clone, Debug, PartialEq)]
pub struct TreeItem {
    pub id: i64,
    pub label: String,
    /// The picture, and the one a closed folder or an unselected item shows instead.
    pub picture: Option<String>,
    pub picture_alt: Option<String>,
    /// `HideIcon`: the picture not drawn, its room kept.
    pub picture_hidden: bool,
    pub folder: bool,
    pub open: bool,
    pub selectable: bool,
    pub selected: bool,
    /// The label's colours: selected (or not selectable), and not selected.
    pub colours: (u32, u32),
    pub flash: bool,
    pub children: Vec<TreeItem>,
}

impl TreeItem {
    /// `StringListViewItem_c(id, label, picture, alt)`: white over 0xc0c0c0.
    pub fn new(id: i64, label: &str, picture: Option<&str>, alt: Option<&str>) -> TreeItem {
        TreeItem { id, label: label.to_string(), picture: picture.map(str::to_string), picture_alt: alt.map(str::to_string), picture_hidden: false, folder: false, open: false, selectable: true, selected: false, colours: (0xffffff, 0xc0c0c0), flash: false, children: Vec::new() }
    }

    /// A folder (`SetIsFolder`, `MakeSelectable(false)`).
    pub fn folder(id: i64, label: &str, open: bool) -> TreeItem {
        let mut f = TreeItem::new(id, label, Some("GFX_GUI_EXPAND_BUTTON_OPEN"), Some("GFX_GUI_EXPAND_BUTTON_CLOSED"));
        f.folder = true;
        f.open = open;
        f.selectable = false;
        f
    }

    fn picture_now(&self) -> Option<&str> {
        let alt = self.picture_alt.as_deref().filter(|_| if self.folder { !self.open } else { !self.selected });
        alt.or(self.picture.as_deref())
    }
}

/// The rows shown: each item's depth, in order.
fn visible(items: &[TreeItem]) -> Vec<(usize, &TreeItem)> {
    fn walk<'a>(items: &'a [TreeItem], depth: usize, out: &mut Vec<(usize, &'a TreeItem)>) {
        for i in items {
            out.push((depth, i));
            if i.folder && i.open {
                walk(&i.children, depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(items, 1, &mut out);
    out
}

pub struct TreeList {
    pub items: Vec<TreeItem>,
    /// Whether a click selects the row (`+0x155`).
    pub select_on_click: bool,
    row: std::cell::Cell<f32>,
    pressed: Option<i64>,
    last_click: Option<(i64, Instant)>,
    started: Instant,
}

impl Default for TreeList {
    fn default() -> TreeList {
        TreeList { items: Vec::new(), select_on_click: true, row: std::cell::Cell::new(16.0), pressed: None, last_click: None, started: Instant::now() }
    }
}

impl TreeList {
    /// A list; `select_on_click` as `+0x155`.
    pub fn new(select_on_click: bool) -> TreeList {
        TreeList { select_on_click, ..Default::default() }
    }

    /// The row height: the label's line or the picture, whichever is taller.
    fn row_height(&self, m: &Measure) -> f32 {
        m.height(FontId::NORMAL).max(16.0)
    }

    fn row_at(&self, y: f32) -> Option<i64> {
        let step = self.row.get() + 1.0;
        if y < 0.0 {
            return None;
        }
        let k = (y / step) as usize;
        visible(&self.items).get(k).filter(|_| y - k as f32 * step < self.row.get()).map(|(_, i)| i.id)
    }

    /// The row under a height in the list, by its item's id.
    pub fn id_at(&self, y: f32) -> Option<i64> {
        self.row_at(y)
    }

    /// The item with this id.
    pub fn find_mut(&mut self, id: i64) -> Option<&mut TreeItem> {
        fn walk(items: &mut [TreeItem], id: i64) -> Option<&mut TreeItem> {
            for i in items {
                if i.id == id {
                    return Some(i);
                }
                if let Some(f) = walk(&mut i.children, id) {
                    return Some(f);
                }
            }
            None
        }
        walk(&mut self.items, id)
    }
}

impl Control for TreeList {
    fn class(&self) -> &'static str {
        "ListViewBase"
    }
    fn preferred(&self, m: &Measure, max: bool) -> Option<Point> {
        if max {
            return Some(Point::new(16000.0, 16000.0));
        }
        let row = self.row_height(m);
        self.row.set(row);
        let rows = visible(&self.items);
        let width = rows.iter().map(|(d, i)| *d as f32 * INDENT + if i.picture.is_some() { 16.0 + GAP } else { 0.0 } + m.width(FontId::NORMAL, &i.label)).fold(0.0, f32::max);
        Some(Point::new(width, rows.len() as f32 * (row + 1.0)))
    }
    fn paint(&self, p: &Paint) {
        let row = self.row.get();
        let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        let blink = (self.started.elapsed().as_millis() / 500) % 2 == 0;
        for (k, (depth, item)) in visible(&self.items).into_iter().enumerate() {
            let top = p.rect.min.y + k as f32 * (row + 1.0);
            let mut x = p.rect.min.x + depth as f32 * INDENT;
            if let Some(name) = item.picture_now() {
                if let Some(s) = p.env.skin.and_then(|s| s.get(name))
                    && !(item.flash && blink)
                    && !item.picture_hidden
                {
                    p.painter.image(s.texture.id(), egui::Rect::from_min_size(egui::pos2(x, top + (row - s.size.y) / 2.0), s.size), uv, egui::Color32::WHITE);
                }
                x += p.env.skin.and_then(|s| s.get(name)).map_or(16.0, |s| s.size.x) + GAP;
            }
            let c = if !item.selectable || item.selected { item.colours.0 } else { item.colours.1 };
            let colour = egui::Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
            let h = p.env.measure.height(FontId::NORMAL);
            super::controls::text(p, FontId::NORMAL, egui::pos2(x, top + (row - h) / 2.0), &item.label, colour);
        }
        if self.items.iter().any(|i| i.flash || i.children.iter().any(|c| c.flash)) {
            p.painter.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    fn mouse(&mut self, m: Mouse, at: Point) -> Option<Signal> {
        match m {
            Mouse::Down => {
                let id = self.row_at(at.y)?;
                self.pressed = Some(id);
                let select = self.select_on_click;
                let item = self.find_mut(id)?;
                if item.folder {
                    item.open = !item.open;
                }
                if select && item.selectable {
                    item.selected = !item.selected;
                }
                let now = Instant::now();
                let clicks = match self.last_click {
                    Some((last, when)) if last == id && now.duration_since(when).as_millis() < 400 => 2,
                    _ => 1,
                };
                self.last_click = (clicks == 1).then_some((id, now));
                Some(Signal::ItemClicked(id, clicks))
            }
            Mouse::Up { .. } => {
                self.pressed = None;
                None
            }
            Mouse::Move => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_show_their_rows_while_open() {
        let mut online = TreeItem::folder(-1, "Online", true);
        online.children = vec![TreeItem::new(7, "bob", Some("GFX_GUI_FRIENDLIST_STATUS_GREEN"), None)];
        let mut offline = TreeItem::folder(-2, "Offline", false);
        offline.children = vec![TreeItem::new(8, "cy", None, None)];
        let items = vec![online, offline];
        let rows: Vec<(usize, i64)> = visible(&items).iter().map(|(d, i)| (*d, i.id)).collect();
        assert_eq!(rows, vec![(1, -1), (2, 7), (1, -2)]);
        assert_eq!(items[1].picture_now(), Some("GFX_GUI_EXPAND_BUTTON_CLOSED"));
        assert_eq!(items[0].picture_now(), Some("GFX_GUI_EXPAND_BUTTON_OPEN"));
        let mut t = TreeList { items, ..Default::default() };
        t.row.set(16.0);
        // (a press on a closed folder opens it)
        assert_eq!(t.mouse(Mouse::Down, Point::new(5.0, 2.0 * 17.0 + 3.0)), Some(Signal::ItemClicked(-2, 1)));
        assert!(t.items[1].open);
    }
}
