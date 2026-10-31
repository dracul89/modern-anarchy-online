//! `MultiListView_c` in its list mode (`SetLayoutMode(1)`), as the mail window and the other list windows build it: a
//! column header (`ColumnHeaderView_c`) over a `ScrollView_c` of the rows.
//!
//! The columns (`AddColumn` 0x10137983: an id, a label, a width and flags; 1 hidden, 2 resizable, 4 movable, 8 sortable)
//! sit side by side, each `width + 1 + 5` after the last (the spacing `+0x1a0` is `Point(5, 3)`), so the rows are
//! `sum - 6` wide (`+0x260`). The header (`FUN_1013a6bd`) has a `ColumnHeaderButton_c` per shown column (FUN_10139c17: a
//! `BorderView_c` of `GFX_GUI_PROGRESS01_*` round a font-5 label 3 in), `width + 5` wide, and an empty one after them.
//! Pressing a label and letting go on it selects its column (`ColumnHeaderView_c` FUN_1013b23f / FUN_1013abef:
//! `SlotColumnSelected` 0x10137548 sorts by it, again the other way round); pressing within 2 of a resizable label's
//! right edge (or of the next one's left) drags its width (FUN_1013ad6a: `SetColumnWidth` 0x101359b6, no less than 5),
//! and a double click there fits it (`ResizeColumnToFit`). Dragging a label to swap columns is not done here.
//!
//! The rows (`UpdateItemPositions` 0x10136434) are the item's height (`MailListItem_c`: 15) plus 1 plus the spacing's 3
//! apart; each is a `StringListColView_c` (FUN_100f21bb) of its cells at the columns' places (FUN_100f2097): a picture,
//! or font-5 text 2 in (FUN_100f2271). A selected row is filled with 0x88aadd behind its cells (FUN_100f1f68). A click on
//! a row (`MouseDown` 0x10133c3f / `MouseUp` 0x10133d04) is told to the window with its click count, which selects it
//! (single selection, flag 0x40: `ItemSelectionStateChanged` 0x10134846 drops the others).

use std::cell::Cell as Shared;
use std::time::Instant;

use super::font::{FontId, Measure};
use super::geom::Point;
use super::host::{Mouse, Paint, Signal};
use super::scroll::{ScrollChild, ScrollView};
use super::view::{Control, Id, Layout, Node, Tree};

/// The spacing between columns and between rows (`+0x1a0`).
const SPACE: Point = Point { x: 5.0, y: 3.0 };
/// A row's height (`MailListItem_c`'s FUN_101041c8).
pub const ROW_HEIGHT: f32 = 15.0;
/// How near a label's edge a press resizes (`_DAT_101a8b98`).
const EDGE: f32 = 2.0;
/// The selected row's fill (FUN_100f1f68).
const SELECTED: u32 = 0x88aadd;

pub const HIDDEN: u32 = 1;
pub const RESIZABLE: u32 = 2;
pub const MOVABLE: u32 = 4;
pub const SORTABLE: u32 = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub id: i32,
    pub label: String,
    pub width: f32,
    pub flags: u32,
}

/// What a row shows in a column.
#[derive(Clone, Debug, PartialEq)]
pub enum CellView {
    Text(String),
    /// A skin picture, centred.
    Picture(String),
}

/// A row (`MultiListViewItem_c`): its id (`GetID`), its cells by column id and what it sorts by in each.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: i64,
    pub cells: Vec<(i32, CellView)>,
    pub keys: Vec<(i32, String)>,
}

impl Item {
    fn cell(&self, column: i32) -> Option<&CellView> {
        self.cells.iter().find(|(c, _)| *c == column).map(|(_, v)| v)
    }
    fn key(&self, column: i32) -> &str {
        self.keys.iter().find(|(c, _)| *c == column).map_or("", |(_, k)| k.as_str())
    }
}

fn shown(columns: &[Column]) -> impl Iterator<Item = &Column> {
    columns.iter().filter(|c| c.flags & HIDDEN == 0)
}

/// Where each shown column starts (`FUN_100f2097`), and the rows' width (`+0x260`).
fn places(columns: &[Column]) -> (Vec<(i32, f32, f32)>, f32) {
    let mut x = 0.0;
    let mut out = Vec::new();
    for c in shown(columns) {
        out.push((c.id, x, c.width));
        x += c.width + 1.0 + SPACE.x;
    }
    (out, x - 1.0 - SPACE.x)
}

/// The rows (the scroll view's client): the items, their order and the selection.
pub struct Rows {
    pub columns: Vec<Column>,
    pub items: Vec<Item>,
    pub selected: Option<i64>,
    last_click: Option<(i64, Instant)>,
    pressed: Option<usize>,
}

impl Rows {
    fn step() -> f32 {
        ROW_HEIGHT + 1.0 + SPACE.y
    }
    fn at(&self, p: Point) -> Option<usize> {
        if p.y < 0.0 {
            return None;
        }
        let k = (p.y / Self::step()) as usize;
        (k < self.items.len() && p.y - k as f32 * Self::step() < ROW_HEIGHT).then_some(k)
    }
}

impl Control for Rows {
    fn class(&self) -> &'static str {
        "MultiListViewRows"
    }
    fn preferred(&self, _m: &Measure, max: bool) -> Option<Point> {
        if max {
            return Some(Point::new(16000.0, 16000.0));
        }
        let height = if self.items.is_empty() { 0.0 } else { self.items.len() as f32 * Self::step() - 1.0 - SPACE.y };
        Some(Point::new(places(&self.columns).1.max(0.0), height))
    }
    fn paint(&self, p: &Paint) {
        let (places, width) = places(&self.columns);
        let colour = super::controls::Colour::DEFAULT.rgb(p.env.palette);
        let font = FontId::NORMAL;
        let clip = p.painter.clip_rect();
        for (k, item) in self.items.iter().enumerate() {
            let top = p.rect.min.y + k as f32 * Self::step();
            if top > clip.bottom() || top + ROW_HEIGHT < clip.top() {
                continue;
            }
            if self.selected == Some(item.id) {
                let fill = egui::Color32::from_rgb((SELECTED >> 16) as u8, (SELECTED >> 8) as u8, SELECTED as u8);
                p.painter.rect_filled(egui::Rect::from_min_size(egui::pos2(p.rect.min.x, top), egui::vec2(width.max(p.rect.width()), ROW_HEIGHT)), 0.0, fill.gamma_multiply(0.45));
            }
            for &(column, x, w) in &places {
                let cell = egui::Rect::from_min_size(egui::pos2(p.rect.min.x + x, top), egui::vec2(w, ROW_HEIGHT));
                let painter = p.painter.with_clip_rect(cell.intersect(clip));
                match item.cell(column) {
                    Some(CellView::Text(t)) => {
                        let q = Paint { painter: &painter, env: p.env, rect: cell, hovered: false, pressed: false, enabled: p.enabled };
                        super::controls::text(&q, font, cell.min + egui::vec2(2.0, 2.0), t, colour);
                    }
                    Some(CellView::Picture(name)) => {
                        if let Some(sprite) = p.env.skin.and_then(|s| s.get(name)) {
                            let at = egui::Rect::from_center_size(cell.center(), sprite.size);
                            painter.image(sprite.texture.id(), at, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                        }
                    }
                    None => {}
                }
            }
        }
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    fn mouse(&mut self, m: Mouse, at: Point) -> Option<Signal> {
        match m {
            Mouse::Down => {
                self.pressed = self.at(at);
                None
            }
            Mouse::Up { inside } => {
                let pressed = self.pressed.take()?;
                if !inside || self.at(at) != Some(pressed) {
                    return None;
                }
                let id = self.items[pressed].id;
                // (the click count: a second click on the same row soon after is a double click)
                let now = Instant::now();
                let clicks = match self.last_click {
                    Some((last, when)) if last == id && now.duration_since(when).as_millis() < 400 => 2,
                    _ => 1,
                };
                self.last_click = (clicks == 1).then_some((id, now));
                Some(Signal::ItemClicked(id, clicks))
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

/// What a press on the header is doing (`ColumnHeaderView_c` +0x148).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Press {
    None,
    /// Resizing a column: its id and where the pointer was.
    Resize(i32, f32),
    /// A label pressed.
    Label(i32),
}

/// `ColumnHeaderView_c`.
pub struct Header {
    pub columns: Vec<Column>,
    press: Press,
    last_edge: Option<(i32, Instant)>,
    height: Shared<f32>,
}

impl Header {
    /// The labels: each shown column's id and its rect's left and right, then the empty one's.
    fn labels(&self) -> Vec<(i32, f32, f32)> {
        let mut x = 0.0;
        let mut out = Vec::new();
        for c in shown(&self.columns) {
            out.push((c.id, x, x + c.width + SPACE.x));
            x += c.width + 1.0 + SPACE.x;
        }
        out.push((-1, x, x + 100000.0));
        out
    }

    fn flags(&self, id: i32) -> u32 {
        self.columns.iter().find(|c| c.id == id).map_or(0, |c| c.flags)
    }
}

impl Control for Header {
    fn class(&self) -> &'static str {
        "ColumnHeaderView"
    }
    fn preferred(&self, m: &Measure, max: bool) -> Option<Point> {
        let h = m.height(FontId::NORMAL) + 6.0;
        self.height.set(h);
        Some(Point::new(if max { 16000.0 } else { 0.0 }, h))
    }
    fn paint(&self, p: &Paint) {
        let tint = egui::Color32::WHITE;
        let colour = super::controls::Colour::DEFAULT.rgb(p.env.palette);
        let h = p.rect.height();
        for (id, l, r) in self.labels() {
            let rect = egui::Rect::from_min_max(egui::pos2(p.rect.min.x + l, p.rect.min.y), egui::pos2(p.rect.min.x + r.min(p.rect.width() + 1.0), p.rect.min.y + h));
            if rect.width() <= 0.0 {
                continue;
            }
            if let Some(skin) = p.env.skin {
                if let Some(bg) = skin.get("GFX_GUI_PROGRESS01_BACKGROUND") {
                    p.painter.image(bg.texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), tint);
                }
                skin.nine_slice(p.painter, "GFX_GUI_PROGRESS01_BORDER", rect, tint, false);
            }
            if let Some(c) = self.columns.iter().find(|c| c.id == id) {
                let painter = p.painter.with_clip_rect(rect.shrink(1.0).intersect(p.painter.clip_rect()));
                let q = Paint { painter: &painter, env: p.env, rect, hovered: false, pressed: false, enabled: p.enabled };
                super::controls::text(&q, FontId::NORMAL, rect.min + egui::vec2(3.0, 3.0), &c.label, colour);
            }
        }
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    fn mouse(&mut self, m: Mouse, at: Point) -> Option<Signal> {
        match m {
            Mouse::Down => {
                // (FUN_1013b23f: the label under the pointer; within 2 of its right edge, or of the next one's left, a resize)
                let labels = self.labels();
                let Some(k) = labels.iter().position(|&(_, l, r)| at.x >= l && at.x < r) else { return None };
                let (id, l, r) = labels[k];
                let edge = if at.x >= r - EDGE && id != -1 {
                    Some(id)
                } else if at.x < l + EDGE && k > 0 {
                    Some(labels[k - 1].0)
                } else {
                    None
                };
                match edge {
                    Some(col) if self.flags(col) & RESIZABLE != 0 => {
                        let now = Instant::now();
                        if self.last_edge.is_some_and(|(c, t)| c == col && now.duration_since(t).as_millis() < 400) {
                            self.last_edge = None;
                            return Some(Signal::ColumnFit(col));
                        }
                        self.last_edge = Some((col, now));
                        self.press = Press::Resize(col, at.x);
                    }
                    Some(_) => {}
                    None if id != -1 => self.press = Press::Label(id),
                    None => {}
                }
                None
            }
            Mouse::Move => {
                if let Press::Resize(col, from) = self.press {
                    let width = self.columns.iter().find(|c| c.id == col)?.width;
                    let to = (width + at.x - from).max(5.0);
                    self.press = Press::Resize(col, from + (to - width));
                    return Some(Signal::ColumnResized(col, to));
                }
                None
            }
            Mouse::Up { inside } => {
                let press = std::mem::replace(&mut self.press, Press::None);
                match press {
                    Press::Label(id) if inside && self.labels().iter().any(|&(c, l, r)| c == id && at.x >= l && at.x < r) => Some(Signal::ColumnSelected(id)),
                    _ => None,
                }
            }
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The list's own node: the sort it keeps (`+0x15c` the column, `+0x160` the order, 0 ascending).
pub struct MultiList {
    pub sort_column: i32,
    pub sort_order: i32,
    pub header: Id,
    pub scroll: Id,
    pub rows: Id,
}

impl Control for MultiList {
    fn class(&self) -> &'static str {
        "MultiListView"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A list-mode `MultiListView_c` (no columns yet), to be added to a view.
pub fn create(tree: &mut Tree, name: &str) -> Id {
    let header = tree.add(Node::new("", Box::new(Header { columns: Vec::new(), press: Press::None, last_edge: None, height: Shared::new(0.0) })));
    // (the scroll view: bars when needed, `SetVScrollBarMode(2)` / `SetHScrollBarMode(2)`)
    let scroll = tree.add(Node::new("", Box::new(ScrollView::new(2, 2))));
    tree.get_mut(scroll).layout = Layout::Scroll;
    let child = tree.add(Node::new("", Box::new(ScrollChild)));
    tree.get_mut(child).layout = Layout::ScrollChild;
    tree.get_mut(child).resize_mask = 0xf;
    let rows = tree.add(Node::new("", Box::new(Rows { columns: Vec::new(), items: Vec::new(), selected: None, last_click: None, pressed: None })));
    tree.add_child(child, rows);
    tree.add_child(scroll, child);
    let id = tree.add(Node::new(name, Box::new(MultiList { sort_column: -1, sort_order: 0, header, scroll, rows })));
    tree.add_child(id, header);
    tree.add_child(id, scroll);
    tree.get_mut(id).layout = Layout::Vertical;
    // (the header no taller than its labels, the rows taking the rest)
    tree.get_mut(header).weight = 0.0;
    tree.get_mut(scroll).weight = 1.0;
    id
}

fn parts(tree: &Tree, id: Id) -> Option<(Id, Id, Id)> {
    tree.control::<MultiList>(id).map(|l| (l.header, l.scroll, l.rows))
}

fn set_columns(tree: &mut Tree, id: Id, columns: Vec<Column>) {
    let Some((header, _, rows)) = parts(tree, id) else { return };
    if let Some(h) = tree.control_mut::<Header>(header) {
        h.columns = columns.clone();
    }
    if let Some(r) = tree.control_mut::<Rows>(rows) {
        r.columns = columns;
    }
    tree.relayout = true;
}

pub fn columns(tree: &Tree, id: Id) -> Vec<Column> {
    parts(tree, id).and_then(|(_, _, rows)| tree.control::<Rows>(rows)).map(|r| r.columns.clone()).unwrap_or_default()
}

/// `AddColumn`: the first sortable column becomes the sort column.
pub fn add_column(tree: &mut Tree, id: Id, column: i32, label: &str, width: f32, flags: u32) {
    let mut cols = columns(tree, id);
    cols.push(Column { id: column, label: label.to_string(), width, flags });
    set_columns(tree, id, cols);
    if tree.control::<MultiList>(id).is_some_and(|l| l.sort_column == -1) && flags & SORTABLE != 0 {
        set_sort(tree, id, column, 0);
    }
}

/// `SetColumnWidth`: no less than 5.
pub fn set_column_width(tree: &mut Tree, id: Id, column: i32, width: f32) {
    let mut cols = columns(tree, id);
    if let Some(c) = cols.iter_mut().find(|c| c.id == column) {
        c.width = width.max(5.0);
    }
    set_columns(tree, id, cols);
}

/// `ResizeColumnToFit`: as wide as its widest text (2 in on each side) or its label.
pub fn fit_column(tree: &mut Tree, id: Id, column: i32) {
    let Some((_, _, rows)) = parts(tree, id) else { return };
    let measure = &tree.measure;
    let width = tree.control::<Rows>(rows).map_or(0.0, |r| {
        r.items
            .iter()
            .filter_map(|i| match i.cell(column) {
                Some(CellView::Text(t)) => Some(measure.width(FontId::NORMAL, t) + 4.0),
                _ => None,
            })
            .fold(0.0, f32::max)
    });
    if width > 0.0 {
        set_column_width(tree, id, column, width);
    }
}

/// `SetListSortColumn`: only a sortable column; the rows sorted at once.
pub fn set_sort(tree: &mut Tree, id: Id, column: i32, order: i32) {
    let cols = columns(tree, id);
    if !cols.iter().any(|c| c.id == column && c.flags & SORTABLE != 0) {
        return;
    }
    if let Some(l) = tree.control_mut::<MultiList>(id) {
        l.sort_column = column;
        l.sort_order = order;
    }
    sort(tree, id);
}

pub fn sort_state(tree: &Tree, id: Id) -> (i32, i32) {
    tree.control::<MultiList>(id).map_or((-1, 0), |l| (l.sort_column, l.sort_order))
}

/// `Sort`: by the sort column's keys, descending when the order is 1.
fn sort(tree: &mut Tree, id: Id) {
    let Some((_, _, rows)) = parts(tree, id) else { return };
    let (column, order) = sort_state(tree, id);
    if column == -1 {
        return;
    }
    if let Some(r) = tree.control_mut::<Rows>(rows) {
        r.items.sort_by(|a, b| {
            let o = a.key(column).cmp(b.key(column));
            if order == 1 { o.reverse() } else { o }
        });
    }
}

/// The rows (`Clear` and `AddItem` for each), sorted.
pub fn set_items(tree: &mut Tree, id: Id, items: Vec<Item>) {
    let Some((_, _, rows)) = parts(tree, id) else { return };
    if let Some(r) = tree.control_mut::<Rows>(rows) {
        if r.selected.is_some_and(|s| !items.iter().any(|i| i.id == s)) {
            r.selected = None;
        }
        r.items = items;
    }
    sort(tree, id);
    tree.relayout = true;
}

pub fn items(tree: &Tree, id: Id) -> &[Item] {
    parts(tree, id).and_then(|(_, _, rows)| tree.control::<Rows>(rows)).map_or(&[], |r| r.items.as_slice())
}

pub fn selected(tree: &Tree, id: Id) -> Option<i64> {
    parts(tree, id).and_then(|(_, _, rows)| tree.control::<Rows>(rows)).and_then(|r| r.selected)
}

/// `MultiListViewItem_c::Select`: one row at a time.
pub fn select(tree: &mut Tree, id: Id, item: Option<i64>) {
    let Some((_, _, rows)) = parts(tree, id) else { return };
    if let Some(r) = tree.control_mut::<Rows>(rows) {
        r.selected = item;
    }
}

/// Acts on the header's signals (sorting, resizing) and leaves the rows' clicks, as `(list, item id, clicks)`.
pub fn take_signals(tree: &mut Tree) -> Vec<(Id, i64, u32)> {
    let signals = std::mem::take(&mut tree.signals);
    let mut clicks = Vec::new();
    let mut rest = Vec::new();
    let owner = |tree: &Tree, v: Id| -> Option<Id> {
        let mut at = tree.get(v).parent;
        while let Some(p) = at {
            if tree.control::<MultiList>(p).is_some() {
                return Some(p);
            }
            at = tree.get(p).parent;
        }
        None
    };
    for (v, s) in signals {
        let Some(list) = owner(tree, v) else {
            rest.push((v, s));
            continue;
        };
        match s {
            // (`SlotColumnSelected`: the sort column again turns the order round, another sorts ascending)
            Signal::ColumnSelected(c) => {
                let (column, order) = sort_state(tree, list);
                set_sort(tree, list, c, if c == column { i32::from(order == 0) } else { 0 });
            }
            Signal::ColumnResized(c, w) => set_column_width(tree, list, c, w),
            Signal::ColumnFit(c) => fit_column(tree, list, c),
            Signal::ItemClicked(item, n) => clicks.push((list, item, n)),
            other => rest.push((v, other)),
        }
    }
    tree.signals = rest;
    clicks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: i64, from: &str) -> Item {
        Item { id, cells: vec![(1, CellView::Text(from.into()))], keys: vec![(1, from.into())] }
    }

    #[test]
    fn columns_and_sorting() {
        let mut tree = Tree::default();
        let id = create(&mut tree, "list");
        add_column(&mut tree, id, 0, "Icon", 16.0, SORTABLE);
        add_column(&mut tree, id, 1, "From", 96.0, 0xe);
        // (the first sortable column is the sort column)
        assert_eq!(sort_state(&tree, id), (0, 0));
        let (places, width) = places(&columns(&tree, id));
        assert_eq!(places, vec![(0, 0.0, 16.0), (1, 22.0, 96.0)]);
        assert_eq!(width, 118.0);
        set_items(&mut tree, id, vec![item(1, "b"), item(2, "c"), item(3, "a")]);
        set_sort(&mut tree, id, 1, 0);
        assert_eq!(items(&tree, id).iter().map(|i| i.id).collect::<Vec<_>>(), [3, 1, 2]);
        // (clicking the sort column's label again turns the order round)
        let header = tree.control::<MultiList>(id).unwrap().header;
        tree.signals.push((header, Signal::ColumnSelected(1)));
        take_signals(&mut tree);
        assert_eq!(sort_state(&tree, id), (1, 1));
        assert_eq!(items(&tree, id).iter().map(|i| i.id).collect::<Vec<_>>(), [2, 1, 3]);
        tree.signals.push((header, Signal::ColumnResized(1, 2.0)));
        take_signals(&mut tree);
        assert_eq!(columns(&tree, id)[1].width, 5.0);
    }

    #[test]
    fn rows_are_nineteen_apart() {
        let mut r = Rows { columns: Vec::new(), items: vec![item(1, "a"), item(2, "b")], selected: None, last_click: None, pressed: None };
        assert_eq!(r.at(Point::new(5.0, 3.0)), Some(0));
        assert_eq!(r.at(Point::new(5.0, 16.0)), None);
        assert_eq!(r.at(Point::new(5.0, 20.0)), Some(1));
        r.mouse(Mouse::Down, Point::new(5.0, 20.0));
        assert_eq!(r.mouse(Mouse::Up { inside: true }, Point::new(5.0, 21.0)), Some(Signal::ItemClicked(2, 1)));
        r.mouse(Mouse::Down, Point::new(5.0, 20.0));
        assert_eq!(r.mouse(Mouse::Up { inside: true }, Point::new(5.0, 21.0)), Some(Signal::ItemClicked(2, 2)));
    }
}
