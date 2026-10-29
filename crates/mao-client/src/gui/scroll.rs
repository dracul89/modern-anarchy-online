//! `ScrollView_c` and `ScrollViewChild_c`: a view that shows part of a taller (or wider) one and a scroll bar for the rest.
//!
//! `ScrollView_c` (the XML constructor 0x101437ae): `h_scrollbar_mode` / `v_scrollbar_mode` (`always` 1, `auto` 2,
//! `auto_reserve` 3, else never 0), `scroll_client` (the child, by name, it scrolls; else its first). `_Initialize`
//! (0x10143434): a wheel step of 10 (`_DAT_101a98ec`), no snapping. The client fills the view less a bar's width and a
//! pixel on the right (and at the bottom for a horizontal bar) where a bar is shown: always with modes 1 and 3, with `auto`
//! when the client's content does not fit (`_UpdateScrollBars` 0x10142b68); its preferred size is the client's plus that
//! room (`CalculatePreferredSize` 0x10142a4b). The bar is a `ScrollBar_c` of `GFX_GUI_SIMPLE_SB_UP`, `_DOWN` and
//! `_SLIDER_VERT` as wide as its arrows (`GetDefaultWidth` 0x10142507), drawn and handled as `chrome::scroll` does: an arrow
//! scrolls 5, a click beside the knob 20, the knob drags. The wheel scrolls 10 a notch (`WheelMoved` 0x10142820; egui's
//! notch is 50 points).
//!
//! `ScrollViewChild_c` (0x10143eca): its children in one inner view laid out by the element's `view_layout`
//! (`SetClient` 0x10144016), sized to at least the frame and its minimum, at most its maximum (`LayoutChilds`
//! 0x10143dd4), and moved by the scroll; its minimum size is the inner view's without the axes its scroll view scrolls
//! (`CalculatePreferredSize` 0x10143ca5). What lies outside it is not drawn.

use std::cell::Cell;

use super::geom::{Point, Rect};
use super::host::{Mouse, Paint, Signal};
use super::view::{Control, Id, Layout, Node, Plain, Tree};
use super::xml::Element;

/// `ScrollBarMode_e`.
pub fn mode(s: &str) -> u8 {
    match s.to_ascii_lowercase().as_str() {
        "always" => 1,
        "auto" => 2,
        "auto_reserve" => 3,
        _ => 0,
    }
}

/// The bar's arrows and knob (the default `ScrollBar_c`'s pictures).
pub const UP: &str = "GFX_GUI_SIMPLE_SB_UP";
pub const DOWN: &str = "GFX_GUI_SIMPLE_SB_DOWN";
pub const KNOB: &str = "GFX_GUI_SIMPLE_SB_SLIDER_VERT";

/// What is pressed on the bar.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Press {
    #[default]
    None,
    Up,
    Down,
    Knob(f32),
    Page,
}

pub struct ScrollView {
    pub h_mode: u8,
    pub v_mode: u8,
    pub step: f32,
    /// How far the client is scrolled (right, down).
    pub offset: Point,
    /// Set by the layout: the client's content size, the room it is shown in, which bars are shown, the view's size and
    /// the bar's width.
    pub content: Cell<Point>,
    pub visible: Cell<Point>,
    pub v_bar: Cell<bool>,
    pub h_bar: Cell<bool>,
    pub size: Cell<Point>,
    pub bar_width: Cell<f32>,
    /// The arrows' heights (from the skin, when drawn).
    arrows: Cell<(f32, f32)>,
    press: Press,
}

impl ScrollView {
    pub fn new(h_mode: u8, v_mode: u8) -> ScrollView {
        ScrollView {
            h_mode,
            v_mode,
            step: 10.0,
            offset: Point::new(0.0, 0.0),
            content: Cell::new(Point::new(0.0, 0.0)),
            visible: Cell::new(Point::new(0.0, 0.0)),
            v_bar: Cell::new(false),
            h_bar: Cell::new(false),
            size: Cell::new(Point::new(0.0, 0.0)),
            bar_width: Cell::new(16.0),
            arrows: Cell::new((16.0, 16.0)),
            press: Press::None,
        }
    }

    /// How far down the client can go.
    pub fn overflow(&self) -> f32 {
        (self.content.get().y - self.visible.get().y).max(0.0)
    }

    /// Scrolls to `y`, kept within the content.
    pub fn scroll_to(&mut self, y: f32) {
        self.offset.y = y.clamp(0.0, self.overflow()).floor();
    }

    /// The bar's rect in the view (its right edge, above any horizontal bar).
    fn bar(&self) -> egui::Rect {
        let s = self.size.get();
        let w = self.bar_width.get();
        let bottom = s.y + 1.0 - if self.h_bar.get() { w + 1.0 } else { 0.0 };
        egui::Rect::from_min_max(egui::pos2(s.x + 1.0 - w, 0.0), egui::pos2(s.x + 1.0, bottom))
    }

    fn knob(&self) -> (f32, f32) {
        let bar = self.bar();
        let (up, down) = self.arrows.get();
        let content = self.content.get().y.max(1.0);
        let overflow = self.overflow();
        let fraction = if overflow > 0.0 { self.offset.y / overflow } else { 0.0 };
        crate::chrome::knob(bar.top(), bar.bottom(), up, down, self.visible.get().y / content, fraction)
    }
}

impl Control for ScrollView {
    fn class(&self) -> &'static str {
        "ScrollView"
    }
    fn paint(&self, p: &Paint) {
        let Some(skin) = p.env.skin else { return };
        if let (Some(up), Some(down)) = (skin.get(UP), skin.get(DOWN)) {
            self.arrows.set((up.size.y, down.size.y));
        }
        if !self.v_bar.get() {
            return;
        }
        let (up, down) = self.arrows.get();
        let bar = self.bar().translate(p.rect.min.to_vec2());
        let tint = crate::chrome::rgb(p.env.palette.accent);
        let w = self.bar_width.get();
        skin.draw_in(p.painter, UP, egui::Rect::from_min_size(bar.min, egui::vec2(w, up)), tint);
        skin.draw_in(p.painter, DOWN, egui::Rect::from_min_size(egui::pos2(bar.left(), bar.bottom() - down), egui::vec2(w, down)), tint);
        if self.overflow() > 0.0 {
            let (k0, k1) = self.knob();
            let off = p.rect.min.y;
            skin.draw_in(p.painter, KNOB, egui::Rect::from_min_max(egui::pos2(bar.left(), off + k0), egui::pos2(bar.right(), off + k1)), tint);
        }
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    fn mouse(&mut self, m: Mouse, at: Point) -> Option<Signal> {
        let bar = self.bar();
        let (up, down) = self.arrows.get();
        let pos = egui::pos2(at.x, at.y);
        match m {
            Mouse::Down if self.v_bar.get() && bar.contains(pos) => {
                let (k0, k1) = self.knob();
                let y = self.offset.y;
                if at.y < bar.top() + up {
                    self.press = Press::Up;
                    self.scroll_to(y - 5.0);
                } else if at.y >= bar.bottom() - down {
                    self.press = Press::Down;
                    self.scroll_to(y + 5.0);
                } else if (k0..k1).contains(&at.y) {
                    self.press = Press::Knob(at.y - k0);
                } else {
                    self.press = Press::Page;
                    self.scroll_to(if at.y < k0 { y - 20.0 } else { y + 20.0 });
                }
                Some(Signal::Scrolled)
            }
            Mouse::Move => {
                if let Press::Knob(grab) = self.press {
                    // (`MouseMove` 0x101416a0: the knob's top over the room it moves in)
                    let (k0, k1) = self.knob();
                    let (start, end) = (bar.top() + up, bar.bottom() - down - 1.0 - (k1 - k0 - 1.0));
                    if end > start {
                        let to = self.overflow() * (at.y - grab - start) / (end - start);
                        self.scroll_to(to);
                        return Some(Signal::Scrolled);
                    }
                }
                None
            }
            Mouse::Up { .. } => {
                self.press = Press::None;
                None
            }
            _ => None,
        }
    }
    fn wheel(&mut self, dy: f32, _height: f32) -> bool {
        if !self.v_bar.get() || self.overflow() <= 0.0 || self.step <= 0.0 {
            return false;
        }
        // (a notch of egui's 50 points is the original's one)
        let y = self.offset.y - dy / 50.0 * self.step;
        self.scroll_to(y);
        true
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// `ScrollViewChild_c`: what is outside it is not drawn.
pub struct ScrollChild;

impl Control for ScrollChild {
    fn class(&self) -> &'static str {
        "ScrollViewChild"
    }
    fn clips(&self) -> bool {
        true
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A `ScrollView` element.
pub fn create_view(tree: &mut Tree, e: &Element) -> Id {
    let view = ScrollView::new(mode(&e.text("h_scrollbar_mode", "")), mode(&e.text("v_scrollbar_mode", "")));
    let id = tree.add(Node::from_element(e, 4, Box::new(view)));
    for c in &e.children {
        if let Some(child) = super::create(tree, c) {
            tree.add_child(id, child);
        }
    }
    // (`scroll_client`, else the first child, is the client: the others stay as they are)
    let named = e.text("scroll_client", "");
    let client = if named.is_empty() { tree.get(id).children.first().copied() } else { tree.find(id, &named) };
    if let Some(c) = client {
        tree.get_mut(c).resize_mask = 0xf;
    }
    tree.get_mut(id).layout = Layout::Scroll;
    id
}

/// A `ScrollViewChild` element: the node and its inner view of the element's children.
pub fn create_child(tree: &mut Tree, e: &Element) -> Id {
    let id = tree.add(Node::from_element(e, 4, Box::new(ScrollChild)));
    let inner = tree.add(Node::new("", Box::new(Plain)));
    super::load_child_views(tree, inner, e);
    tree.add_child(id, inner);
    tree.get_mut(id).layout = Layout::ScrollChild;
    id
}

/// The client of a scroll view (its first laid-out child).
fn client(tree: &Tree, id: Id) -> Option<Id> {
    tree.get(id).children.iter().copied().find(|&c| tree.laid_out(c))
}

/// The scroll view's `CalculatePreferredSize`: the client's, with the room of the bars it reserves or shows.
pub fn preferred_view(tree: &Tree, id: Id, max: bool) -> Point {
    let Some(c) = client(tree, id) else { return Point::new(-1.0, -1.0) };
    let r = tree.get(c).borders;
    let mut p = tree.preferred(c, max);
    p.x += r.left + r.right;
    p.y += r.top + r.bottom;
    if let Some(v) = tree.control::<ScrollView>(id) {
        let w = v.bar_width.get() + 1.0;
        if v.v_mode == 3 || v.v_bar.get() {
            p.x += w;
        }
        if v.h_mode == 3 || v.h_bar.get() {
            p.y += w;
        }
    }
    p
}

/// The scroll view's layout: the bars it shows and the client's frame.
pub fn layout_view(tree: &mut Tree, id: Id) {
    let bounds = tree.get(id).bounds();
    let Some(c) = client(tree, id) else { return };
    let r = tree.get(c).borders;
    // (the content: the client's inner view's minimum, or the client's own)
    let inner = tree.get(c).children.first().copied().filter(|_| tree.get(c).layout == Layout::ScrollChild);
    let content = inner.map_or_else(|| tree.preferred(c, false), |i| tree.preferred(i, false));
    let (h_mode, v_mode, w) = match tree.control::<ScrollView>(id) {
        Some(v) => (v.h_mode, v.v_mode, v.bar_width.get() + 1.0),
        None => return,
    };
    let room = |bar: bool, whole: f32| whole - if bar { w } else { 0.0 };
    let mut v_bar = v_mode == 1 || v_mode == 3;
    let mut h_bar = h_mode == 1 || h_mode == 3;
    for _ in 0..2 {
        let (aw, ah) = (room(v_bar, bounds.width() - r.left - r.right), room(h_bar, bounds.height() - r.top - r.bottom));
        v_bar |= v_mode == 2 && content.y > ah;
        h_bar |= h_mode == 2 && content.x > aw;
    }
    let frame = Rect::new(bounds.left, bounds.top, bounds.right - if v_bar { w } else { 0.0 }, bounds.bottom - if h_bar { w } else { 0.0 }).inset(&r);
    tree.get_mut(c).frame = frame;
    if let Some(v) = tree.control_mut::<ScrollView>(id) {
        v.v_bar.set(v_bar);
        v.h_bar.set(h_bar);
        v.content.set(content);
        v.visible.set(Point::new(frame.width(), frame.height()));
        v.size.set(Point::new(bounds.width(), bounds.height()));
        let y = v.offset.y;
        v.scroll_to(y);
    }
}

/// The scroll child's `CalculatePreferredSize`: its inner view's maximum, or its minimum without the scrolled axes.
pub fn preferred_child(tree: &Tree, id: Id, max: bool) -> Point {
    let Some(inner) = tree.get(id).children.first().copied() else { return Point::new(0.0, 0.0) };
    let mut p = tree.preferred(inner, max);
    if !max {
        let modes = tree.get(id).parent.and_then(|s| tree.control::<ScrollView>(s)).map(|v| (v.h_mode, v.v_mode));
        match modes {
            Some((h, v)) => {
                if h != 0 {
                    p.x = 0.0;
                }
                if v != 0 {
                    p.y = 0.0;
                }
            }
            None => return Point::new(0.0, 0.0),
        }
    }
    p
}

/// `LayoutChilds`: the inner view at least the frame's size and its minimum, at most its maximum, moved by the scroll.
pub fn layout_child(tree: &mut Tree, id: Id) {
    let Some(inner) = tree.get(id).children.first().copied() else { return };
    let bounds = tree.get(id).bounds();
    let (lo, hi) = (tree.preferred(inner, false), tree.preferred(inner, true));
    let mut size = lo;
    if size.x < bounds.width() {
        size.x = bounds.width().min(hi.x);
    }
    if size.y < bounds.height() {
        size.y = bounds.height().min(hi.y);
    }
    let offset = tree.get(id).parent.and_then(|s| tree.control::<ScrollView>(s)).map_or(Point::new(0.0, 0.0), |v| v.offset);
    tree.get_mut(inner).frame = Rect::new(-offset.x, -offset.y, size.x - offset.x, size.y - offset.y);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(xml: &str) -> (Tree, Id) {
        let doc = super::super::xml::parse(xml);
        let mut tree = Tree::default();
        let id = super::super::create(&mut tree, &doc[0]).unwrap();
        (tree, id)
    }

    #[test]
    fn scrolls_a_tall_column() {
        // twenty 19-high rows in a 100-high view with an automatic bar
        let rows: String = (0..20).map(|k| format!(r#"<View name="r{k}" min_size="Point(50,18)" max_size="Point(50,18)"/>"#)).collect();
        let (mut tree, id) = load(&format!(r#"<ScrollView v_scrollbar_mode="auto"><ScrollViewChild view_layout="vertical" name="list">{rows}</ScrollViewChild></ScrollView>"#));
        tree.set_frame(id, Rect::new(0.0, 0.0, 120.0, 99.0));
        let v = tree.control::<ScrollView>(id).unwrap();
        assert!(v.v_bar.get());
        assert!(v.overflow() > 200.0, "{}", v.overflow());
        let list = tree.find(id, "list").unwrap();
        // the client is the view less the bar and a pixel
        assert_eq!(tree.get(list).frame.right, 120.0 - 17.0);
        // the wheel scrolls it 10 a notch, and the rows move up
        let row = tree.find(id, "r1").unwrap();
        let before = tree.window_frame(row).top;
        assert!(tree.get_mut(id).control.wheel(-50.0, 100.0));
        tree.set_frame(id, Rect::new(0.0, 0.0, 120.0, 99.0));
        assert_eq!(tree.window_frame(row).top, before - 10.0);
        // a short list shows no bar
        let (mut tree, id) = load(r#"<ScrollView v_scrollbar_mode="auto"><ScrollViewChild view_layout="vertical"><View min_size="Point(50,18)"/></ScrollViewChild></ScrollView>"#);
        tree.set_frame(id, Rect::new(0.0, 0.0, 120.0, 99.0));
        assert!(!tree.control::<ScrollView>(id).unwrap().v_bar.get());
        assert_eq!(mode("AUTO_RESERVE"), 3);
    }
}
