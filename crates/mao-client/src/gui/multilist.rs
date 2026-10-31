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

/// A row (`MultiListViewItem_c`): its id (`GetID`), its cells by column id and what it sorts by in each; in a grid, its
/// icon and stack count (`InventoryListViewItem_c`'s `ItemIconView_c`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    pub id: i64,
    pub cells: Vec<(i32, CellView)>,
    pub keys: Vec<(i32, String)>,
    pub icon: Option<egui::TextureId>,
    pub count: u32,
    /// In a grid that is not auto arranged, the item's cell (column, row): `AddItem`'s grid position.
    pub pos: Option<(usize, usize)>,
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

/// The grid mode's cells (`LayoutMode_e` 0, the `IconSize_e` 1 of the item windows): 47 by 47 (`+0x180`), 10 apart
/// (`+0x188`, kept as it is with feature flag 0x10), 6 in from the edges (`+0x170`), so one every 58 (`+0x198`: the cell, the
/// spacing and 1). With feature flag 4 (auto arrange) the items fill the cells in their order, as many columns as the width
/// holds (`RecalcCellCount` 0x10134ad2: `width / 58`, at least 1). Each cell up to the most the container holds
/// (`SetMaxItemCount`: the inventory 30, a backpack 21, the bank 102) shows its slot picture (`UpdateBackgroundSurfaces`
/// 0x101340ff: `GFX_GUI_MULTILISTVIEW_SLOT_48_CLOSED`, centred on the cell), and an item its icon over the cell, its stack
/// count in yellow small text at the top right when above 1 (`ItemIconView_c` FUN_10040574: `<font color=yellow>%u</font>`,
/// font 7) and, selected, `GFX_GUI_SELECTED_ICON` over it (FUN_10040d0b).
pub mod grid {
    pub const CELL: f32 = 47.0;
    pub const SPACING: f32 = 10.0;
    pub const BORDER: f32 = 6.0;
    pub const STEP: f32 = CELL + SPACING + 1.0;
    pub const SLOT: &str = "GFX_GUI_MULTILISTVIEW_SLOT_48_CLOSED";
    /// A labelled cell's slot (`UpdateBackgroundSurfaces` with grid labels: 0x11e).
    pub const SLOT_OPEN: &str = "GFX_GUI_MULTILISTVIEW_SLOT_48_OPEN";
    pub const SELECTED: &str = "GFX_GUI_SELECTED_ICON";

    /// The columns a width holds.
    pub fn columns(width: f32) -> usize {
        ((width / STEP).floor() as usize).max(1)
    }

    /// A cell's top left (`GridPosToViewPos` 0x101330b7, auto arranged).
    pub fn place(k: usize, columns: usize) -> (f32, f32) {
        (BORDER + (k % columns) as f32 * STEP, BORDER + (k / columns) as f32 * STEP)
    }

    /// The size of `cells` cells in `columns` columns (`CellCountToPreferredSize` 0x101332b8).
    pub fn size(cells: usize, columns: usize) -> (f32, f32) {
        let rows = cells.max(1).div_ceil(columns);
        (2.0 * BORDER + columns as f32 * STEP - 1.0 - SPACING, 2.0 * BORDER + rows as f32 * STEP - 1.0 - SPACING)
    }
}

/// Grid mode: the width the cells are laid out in (the scroll view's, set by [`update`]), the most items it holds (`+0x234`,
/// -1 for no limit), and what the wear view sets: the spacing (`SetGridIconSpacing`, 10 by 10 by default), a fixed number
/// of columns (`SetViewCellCounts`), the cells' labels (`SetGridLabel` 0x101361d9: font-5 text centred on the cell, white
/// over the icons while `labels_on_top`, else 0xc0c0c0 under them), whether items fill the cells in order (feature flag 4)
/// or sit at their own cells, and a picture under the cells (`SetBackgroundBitmap` 0x1013313a, at the top left).
#[derive(Clone, Debug, PartialEq)]
pub struct GridMode {
    pub width: f32,
    pub max_items: i32,
    pub spacing: (f32, f32),
    pub columns: Option<usize>,
    pub labels: Vec<((usize, usize), String)>,
    pub labels_on_top: bool,
    pub auto_arrange: bool,
    pub background: Option<String>,
}

impl GridMode {
    pub fn new(max_items: i32, columns: usize) -> GridMode {
        GridMode { width: columns as f32 * grid::STEP + 2.0 * grid::BORDER, max_items, spacing: (grid::SPACING, grid::SPACING), columns: None, labels: Vec::new(), labels_on_top: true, auto_arrange: true, background: None }
    }
    /// One cell to the next (`+0x198`: the cell, the spacing and 1).
    fn step(&self) -> (f32, f32) {
        (grid::CELL + self.spacing.0 + 1.0, grid::CELL + self.spacing.1 + 1.0)
    }
    fn cols(&self) -> usize {
        self.columns.unwrap_or_else(|| ((self.width / self.step().0).floor() as usize).max(1))
    }
    /// A cell's top left.
    fn place(&self, (x, y): (usize, usize)) -> (f32, f32) {
        let (sx, sy) = self.step();
        (grid::BORDER + x as f32 * sx, grid::BORDER + y as f32 * sy)
    }
    /// The cell under a point (`ViewPosToGridPos`): not the spacing beside it.
    fn cell_at(&self, p: Point) -> Option<(usize, usize)> {
        let (sx, sy) = self.step();
        let (cx, cy) = (((p.x - grid::BORDER) / sx).floor(), ((p.y - grid::BORDER) / sy).floor());
        if cx < 0.0 || cy < 0.0 || cx as usize >= self.cols() {
            return None;
        }
        let cell = (cx as usize, cy as usize);
        let (x, y) = self.place(cell);
        (p.x - x < grid::CELL && p.y - y < grid::CELL).then_some(cell)
    }
}

/// The rows (the scroll view's client): the items, their order and the selection.
pub struct Rows {
    pub columns: Vec<Column>,
    pub items: Vec<Item>,
    pub selected: Option<i64>,
    /// Grid mode instead of the list.
    pub grid: Option<GridMode>,
    last_click: Option<(i64, Instant)>,
    pressed: Option<(usize, Point)>,
    dragged: bool,
    /// The grid's picture's size, once drawn.
    picture: Shared<Option<egui::Vec2>>,
}

impl Rows {
    fn new(grid: Option<GridMode>) -> Rows {
        Rows { columns: Vec::new(), items: Vec::new(), selected: None, grid, last_click: None, pressed: None, dragged: false, picture: Shared::new(None) }
    }
    fn step() -> f32 {
        ROW_HEIGHT + 1.0 + SPACE.y
    }
    /// The cells a grid shows (a slot each).
    fn cells(&self, g: &GridMode) -> usize {
        if g.max_items >= 0 { (g.max_items as usize).max(self.items.len()) } else { self.items.len() }
    }
    /// An item's cell.
    fn item_cell(&self, g: &GridMode, k: usize) -> (usize, usize) {
        let cols = g.cols();
        match self.items[k].pos {
            Some(pos) if !g.auto_arrange => pos,
            _ => (k % cols, k / cols),
        }
    }
    /// The grid's size: its cells' (the rows its items, labels or slots reach) and its picture's.
    fn grid_size(&self, g: &GridMode, picture: Option<egui::Vec2>) -> Point {
        let cols = g.cols();
        let mut rows = self.cells(g).div_ceil(cols);
        for k in 0..self.items.len() {
            rows = rows.max(self.item_cell(g, k).1 + 1);
        }
        for ((_, y), _) in &g.labels {
            rows = rows.max(y + 1);
        }
        let (sx, sy) = g.step();
        let (w, h) = (2.0 * grid::BORDER + cols as f32 * sx - 1.0 - g.spacing.0, 2.0 * grid::BORDER + rows.max(1) as f32 * sy - 1.0 - g.spacing.1);
        let pic = picture.unwrap_or(egui::Vec2::ZERO);
        Point::new(w.max(pic.x), h.max(pic.y))
    }
    fn at(&self, p: Point) -> Option<usize> {
        if let Some(g) = &self.grid {
            let cell = g.cell_at(p)?;
            return (0..self.items.len()).find(|&k| self.item_cell(g, k) == cell);
        }
        if p.y < 0.0 {
            return None;
        }
        let k = (p.y / Self::step()) as usize;
        (k < self.items.len() && p.y - k as f32 * Self::step() < ROW_HEIGHT).then_some(k)
    }

    fn paint_grid(&self, p: &Paint, g: &GridMode) {
        let cols = g.cols();
        let clip = p.painter.clip_rect();
        let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        let skin = p.env.skin;
        let rect_of = |cell: (usize, usize)| {
            let (x, y) = g.place(cell);
            egui::Rect::from_min_size(p.rect.min + egui::vec2(x, y), egui::vec2(grid::CELL, grid::CELL))
        };
        let slot = |name: &str, cell: egui::Rect| {
            if let Some(slot) = skin.and_then(|s| s.get(name)) {
                p.painter.image(slot.texture.id(), egui::Rect::from_center_size(cell.center(), slot.size), uv, egui::Color32::WHITE);
            }
        };
        // (the picture under the cells, at its own size)
        if let Some(bg) = g.background.as_deref().and_then(|n| skin.and_then(|s| s.get(n))) {
            self.picture.set(Some(bg.size));
            p.painter.image(bg.texture.id(), egui::Rect::from_min_size(p.rect.min, bg.size), uv, egui::Color32::WHITE);
        }
        // (labelled cells show the open slot; else every cell up to the most the container holds the closed one)
        if g.labels.is_empty() {
            for k in 0..self.cells(g) {
                let cell = rect_of((k % cols, k / cols));
                if cell.intersects(clip) {
                    slot(grid::SLOT, cell);
                }
            }
        } else {
            for (at, _) in &g.labels {
                slot(grid::SLOT_OPEN, rect_of(*at));
            }
        }
        let labels = |colour: egui::Color32| {
            for (at, text) in &g.labels {
                let cell = rect_of(*at);
                // (the label's markup: `<center>Right<br>Hand</center>`, a line each, centred)
                let plain = text.replace("<center>", "").replace("</center>", "").replace("<br>", "\n");
                let lines: Vec<&str> = plain.split('\n').collect();
                let h = p.env.measure.height(FontId::NORMAL);
                let top = cell.center().y - h * lines.len() as f32 / 2.0;
                for (i, line) in lines.iter().enumerate() {
                    let w = p.env.measure.width(FontId::NORMAL, line);
                    super::controls::text(p, FontId::NORMAL, egui::pos2(cell.center().x - w / 2.0, top + i as f32 * h), line, colour);
                }
            }
        };
        if !g.labels_on_top {
            labels(egui::Color32::from_rgb(0xc0, 0xc0, 0xc0));
        }
        for (k, item) in self.items.iter().enumerate() {
            let cell = rect_of(self.item_cell(g, k));
            if !cell.intersects(clip) {
                continue;
            }
            if let Some(icon) = item.icon {
                p.painter.image(icon, cell, uv, egui::Color32::WHITE);
            }
            if item.count > 1 {
                let font = FontId(7);
                let text = item.count.to_string();
                let w = p.env.measure.width(font, &text);
                super::controls::text(p, font, egui::pos2(cell.right() - w, cell.top()), &text, egui::Color32::YELLOW);
            }
            if self.selected == Some(item.id)
                && let Some(s) = skin.and_then(|s| s.get(grid::SELECTED))
            {
                p.painter.image(s.texture.id(), cell, uv, egui::Color32::WHITE);
            }
        }
        if g.labels_on_top {
            labels(egui::Color32::WHITE);
        }
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
        if let Some(g) = &self.grid {
            return Some(self.grid_size(g, self.picture.get()));
        }
        let height = if self.items.is_empty() { 0.0 } else { self.items.len() as f32 * Self::step() - 1.0 - SPACE.y };
        Some(Point::new(places(&self.columns).1.max(0.0), height))
    }
    fn paint(&self, p: &Paint) {
        if let Some(g) = &self.grid {
            self.paint_grid(p, g);
            return;
        }
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
                self.pressed = self.at(at).map(|k| (k, at));
                self.dragged = false;
                None
            }
            Mouse::Up { inside } => {
                let (pressed, _) = self.pressed.take()?;
                if !inside || self.dragged || self.at(at) != Some(pressed) {
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
            // (a press moved more than 3 away drags the item: `MultiListView_c`'s drag signal)
            Mouse::Move => match self.pressed {
                Some((k, from)) if !self.dragged && ((at.x - from.x).abs() > 3.0 || (at.y - from.y).abs() > 3.0) => {
                    self.dragged = true;
                    Some(Signal::ItemDragged(self.items.get(k)?.id))
                }
                _ => None,
            },
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
    let rows = tree.add(Node::new("", Box::new(Rows::new(None))));
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

/// A grid-mode `MultiListView_c` (`SetLayoutMode(0)`, feature flags 0x15) holding at most `max_items` (-1: no limit): no
/// header, the cells in a scroll view, `columns` of them until it is laid out.
pub fn create_grid(tree: &mut Tree, name: &str, max_items: i32, columns: usize) -> Id {
    let id = create(tree, name);
    let (header, _, rows) = parts(tree, id).expect("a list");
    tree.get_mut(header).flags |= super::view::FLAG_COLLAPSE;
    tree.get_mut(header).hidden = 1;
    if let Some(r) = tree.control_mut::<Rows>(rows) {
        r.grid = Some(GridMode::new(max_items, columns));
    }
    id
}

/// Changes a grid's settings (the wear view's spacing, cells, labels and picture).
pub fn set_grid(tree: &mut Tree, id: Id, f: impl FnOnce(&mut GridMode)) {
    let Some((_, _, rows)) = parts(tree, id) else { return };
    if let Some(g) = tree.control_mut::<Rows>(rows).and_then(|r| r.grid.as_mut()) {
        f(g);
        tree.relayout = true;
    }
}

/// The grid cell under a point on screen (the window's content at `origin`), item or not: where a drop lands.
pub fn cell_at(tree: &Tree, id: Id, origin: egui::Pos2, at: egui::Pos2) -> Option<(usize, usize)> {
    let (_, scroll, rows) = parts(tree, id)?;
    if !super::host::screen_rect(tree, scroll, origin).contains(at) {
        return None;
    }
    let r = super::host::screen_rect(tree, rows, origin);
    tree.control::<Rows>(rows)?.grid.as_ref()?.cell_at(Point::new(at.x - r.min.x, at.y - r.min.y))
}

/// `SetMaxItemCount`.
pub fn set_max_items(tree: &mut Tree, id: Id, max_items: i32) {
    let Some((_, _, rows)) = parts(tree, id) else { return };
    if let Some(g) = tree.control_mut::<Rows>(rows).and_then(|r| r.grid.as_mut())
        && g.max_items != max_items
    {
        g.max_items = max_items;
        tree.relayout = true;
    }
}

/// Before showing: a grid lays its cells out in the width its scroll view gives it (`FrameSized` 0x101363c5 ->
/// `RecalcCellCount`).
pub fn update(tree: &mut Tree, id: Id) {
    let Some((_, scroll, rows)) = parts(tree, id) else { return };
    let width = tree.control::<ScrollView>(scroll).map_or(0.0, |v| v.visible.get().x);
    if width <= 0.0 {
        return;
    }
    if let Some(g) = tree.control_mut::<Rows>(rows).and_then(|r| r.grid.as_mut())
        && g.columns.is_none()
        && g.width != width
    {
        g.width = width;
        tree.relayout = true;
    }
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

/// The item under a point on screen (the window's content at `origin`), for its tip.
pub fn item_at(tree: &Tree, id: Id, origin: egui::Pos2, at: egui::Pos2) -> Option<i64> {
    let (_, scroll, rows) = parts(tree, id)?;
    // (only where the scroll view shows the rows)
    if !super::host::screen_rect(tree, scroll, origin).contains(at) {
        return None;
    }
    let r = super::host::screen_rect(tree, rows, origin);
    let rows = tree.control::<Rows>(rows)?;
    rows.at(Point::new(at.x - r.min.x, at.y - r.min.y)).map(|k| rows.items[k].id)
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

/// What the rows told: a click (with its count) or a drag of an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemEvent {
    Clicked(i64, u32),
    Dragged(i64),
}

/// Acts on the header's signals (sorting, resizing) and leaves the rows' clicks, as `(list, item id, clicks)`.
pub fn take_signals(tree: &mut Tree) -> Vec<(Id, i64, u32)> {
    take_events(tree).into_iter().filter_map(|(l, e)| if let ItemEvent::Clicked(i, n) = e { Some((l, i, n)) } else { None }).collect()
}

/// [`take_signals`] with the drags.
pub fn take_events(tree: &mut Tree) -> Vec<(Id, ItemEvent)> {
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
            Signal::ItemClicked(item, n) => clicks.push((list, ItemEvent::Clicked(item, n))),
            Signal::ItemDragged(item) => clicks.push((list, ItemEvent::Dragged(item))),
            other => rest.push((v, other)),
        }
    }
    tree.signals = rest;
    clicks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_cells() {
        // (a 360-wide view holds six 58-apart columns; the bank's 102 slots are 17 rows)
        assert_eq!(grid::columns(360.0), 6);
        assert_eq!(grid::columns(10.0), 1);
        assert_eq!(grid::place(7, 6), (6.0 + 58.0, 6.0 + 58.0));
        assert_eq!(grid::size(102, 6), (12.0 + 6.0 * 58.0 - 11.0, 12.0 + 17.0 * 58.0 - 11.0));
        let mut g = GridMode::new(102, 6);
        g.width = 360.0;
        let mut r = Rows::new(Some(g));
        r.items = vec![item(1, "a"), item(2, "b")];
        assert_eq!(r.at(Point::new(6.0 + 58.0 + 3.0, 10.0)), Some(1));
        assert_eq!(r.at(Point::new(6.0 + 50.0, 10.0)), None);
        assert_eq!(r.at(Point::new(6.0 + 2.0 * 58.0, 10.0)), None);
        r.mouse(Mouse::Down, Point::new(10.0, 10.0));
        assert_eq!(r.mouse(Mouse::Move, Point::new(20.0, 10.0)), Some(Signal::ItemDragged(1)));
        assert_eq!(r.mouse(Mouse::Up { inside: true }, Point::new(20.0, 10.0)), None);
    }

    fn item(id: i64, from: &str) -> Item {
        Item { id, cells: vec![(1, CellView::Text(from.into()))], keys: vec![(1, from.into())], ..Default::default() }
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
        let mut r = Rows::new(None);
        r.items = vec![item(1, "a"), item(2, "b")];
        assert_eq!(r.at(Point::new(5.0, 3.0)), Some(0));
        assert_eq!(r.at(Point::new(5.0, 16.0)), None);
        assert_eq!(r.at(Point::new(5.0, 20.0)), Some(1));
        r.mouse(Mouse::Down, Point::new(5.0, 20.0));
        assert_eq!(r.mouse(Mouse::Up { inside: true }, Point::new(5.0, 21.0)), Some(Signal::ItemClicked(2, 1)));
        r.mouse(Mouse::Down, Point::new(5.0, 20.0));
        assert_eq!(r.mouse(Mouse::Up { inside: true }, Point::new(5.0, 21.0)), Some(Signal::ItemClicked(2, 2)));
    }
}
