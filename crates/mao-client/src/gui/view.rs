//! GUI.dll's `View` (0x1014e42e and the class's methods) as a tree of nodes: each view's frame in its parent, layout
//! borders, size limits, weight, alignments, flags and its layout node; the class-specific part is a [`Control`].
//!
//! Sizes and frames are in the original's units (see `geom`). A view's preferred size is `View::UpdatePreferredSize`
//! (0x1014acda) over its control's `CalculatePreferredSize`, or its layout node's (`layout.rs`) when the control has none,
//! widened to its width/height group's (`View::PreferredSizeChanged` 0x1014d8f8).

use super::font::Measure;
use super::geom::{Point, Rect};
use super::host::{Mouse, Paint, Signal};
use super::xml::Element;
use std::any::Any;

pub type Id = usize;

/// `alignment` (FUN_10149ec0 reads "left", "right", "top", "bottom"; anything else is centred).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left = 0,
    Right = 1,
    Top = 2,
    Bottom = 3,
    Center = 4,
}

impl Align {
    pub fn parse(s: &str) -> Align {
        match s.to_ascii_lowercase().as_str() {
            "left" => Align::Left,
            "right" => Align::Right,
            "top" => Align::Top,
            "bottom" => Align::Bottom,
            _ => Align::Center,
        }
    }
}

/// A view's layout node (`View::SetLayoutNode`; `LoadChildViews` 0x1014e092 picks it from `view_layout`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// None: the view lays nothing out and has no preferred size of its own children.
    None,
    /// `LayoutNode`: each child over the whole view, less its borders.
    Base,
    /// `HLayoutNode`.
    Horizontal,
    /// `VLayoutNode` (also for no `view_layout`, or one not known).
    Vertical,
    /// "stacked": no layout node; the children are given the view's bounds and follow its size (resize mask 0xf).
    Stacked,
}

/// A view that is hidden is still laid out unless it has this flag (`GetFlags() & 0x100`).
pub const FLAG_COLLAPSE: u32 = 0x100;

/// What a view's class adds to the plain `View`: overridden virtuals.
pub trait Control: Any {
    /// The class name (the XML element it is registered under).
    fn class(&self) -> &'static str;
    /// `CalculatePreferredSize(max)` when the class overrides it; `None` for `View`'s (the layout node's).
    fn preferred(&self, _m: &Measure, _max: bool) -> Option<Point> {
        None
    }
    /// `Render`: the view's own pictures and text (its children are drawn after it, over it).
    fn paint(&self, _p: &Paint) {}
    /// Whether the view takes the mouse (a plain view lets it through to the one below).
    fn takes_mouse(&self) -> bool {
        false
    }
    /// `MouseDown` / `MouseUp` / `MouseMove` at a point in the view's bounds; a signal for the window's code.
    fn mouse(&mut self, _m: Mouse, _at: Point) -> Option<Signal> {
        None
    }
    /// The mouse wheel over the view (`View::WheelMoved`): `dy` points down, the view `height` pixels tall; whether it
    /// used it.
    fn wheel(&mut self, _dy: f32, _height: f32) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// The plain `View`.
pub struct Plain;

impl Control for Plain {
    fn class(&self) -> &'static str {
        "View"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// `LayoutSpacer` (0x10130bfd) and its `HLayoutSpacer`/`VLayoutSpacer`: a preferred size of its own, kept as given less
/// (1, 1) (`SetMinSize`/`SetMaxSize`).
pub struct Spacer {
    pub min: Point,
    pub max: Point,
}

impl Control for Spacer {
    fn class(&self) -> &'static str {
        "LayoutSpacer"
    }
    fn preferred(&self, _m: &Measure, max: bool) -> Option<Point> {
        Some(if max { self.max } else { self.min })
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct Node {
    pub name: String,
    pub control: Box<dyn Control>,
    pub parent: Option<Id>,
    /// Bottom to top (`GetBottomChild`, `GetHigherSibling`): the order they were added.
    pub children: Vec<Id>,
    /// In the parent's coordinates (+0xac's +0x20).
    pub frame: Rect,
    /// `layout_borders` (+0xb4).
    pub borders: Rect,
    /// `ExtendMinSize` (+0xc8), `ExtendMaxSize` (+0xd0), `LimitMaxSize` (+0xd8).
    pub min_size: Point,
    pub max_extend: Point,
    pub max_limit: Point,
    /// +0xc4, 1 by default.
    pub weight: f32,
    /// +0xe0 and +0xe4: how this view places its children across its layout's direction.
    pub h_align: Align,
    pub v_align: Align,
    pub flags: u32,
    /// `_IncHideCount`: hidden while above 0.
    pub hidden: u32,
    pub enabled: bool,
    pub layout: Layout,
    pub resize_mask: u32,
    pub width_group: String,
    pub width_group_owner: String,
    pub height_group: String,
    pub height_group_owner: String,
    pub fade_group: String,
    /// `activate_criteria` (the views that show only when it holds), and whether it last failed (holding one hide).
    pub criteria: String,
    pub criteria_off: bool,
    /// `view_enable_expression` (View 0x1014e42e): enabled only while it holds.
    pub enable_expression: String,
    pub tooltip: String,
}

impl Node {
    pub fn new(name: &str, control: Box<dyn Control>) -> Node {
        // (View::Initialize 0x1014cc8d)
        Node {
            name: name.to_string(),
            control,
            parent: None,
            children: Vec::new(),
            frame: Rect::default(),
            borders: Rect::default(),
            min_size: Point::new(-1.0, -1.0),
            max_extend: Point::new(-1.0, -1.0),
            max_limit: Point::new(16000.0, 16000.0),
            weight: 1.0,
            h_align: Align::Center,
            v_align: Align::Center,
            flags: 4,
            hidden: 0,
            criteria_off: false,
            enable_expression: String::new(),
            enabled: true,
            layout: Layout::None,
            resize_mask: 0,
            width_group: String::new(),
            width_group_owner: String::new(),
            height_group: String::new(),
            height_group_owner: String::new(),
            fade_group: String::new(),
            criteria: String::new(),
            tooltip: String::new(),
        }
    }

    /// The common attributes `View::View(TiXmlElement*, flags, ...)` (0x1014e42e) reads.
    pub fn from_element(e: &Element, flags: u32, control: Box<dyn Control>) -> Node {
        let mut n = Node::new(&e.text("name", ""), control);
        n.flags = e.int("view_flags", flags as i32) as u32;
        n.resize_mask = e.attribute("view_resize_mask").and_then(|v| v.trim().parse().ok()).unwrap_or(0);
        n.width_group_owner = e.text("width_group_owner", "");
        n.height_group_owner = e.text("height_group_owner", "");
        n.width_group = e.text("width_group", "");
        n.height_group = e.text("height_group", "");
        n.fade_group = e.text("fade_group", "");
        n.borders = e.rect("layout_borders", Rect::default());
        // (ExtendMinSize / ExtendMaxSize / LimitMaxSize: each replaces the field)
        n.min_size = e.point("min_size", Point::new(-1.0, -1.0));
        n.max_extend = e.point("max_size", Point::new(-1.0, -1.0));
        n.max_limit = e.point("max_size_limit", Point::new(16000.0, 16000.0));
        n.h_align = Align::parse(&e.text("h_alignment", ""));
        n.v_align = Align::parse(&e.text("v_alignment", ""));
        n.criteria = e.text("activate_criteria", "");
        n.enable_expression = e.attribute("view_enable_expression").unwrap_or("").to_string();
        n
    }

    pub fn bounds(&self) -> Rect {
        self.frame.bounds()
    }
}

/// A window's views.
#[derive(Default)]
pub struct Tree {
    pub nodes: Vec<Node>,
    /// Lay out again at the next fit even when the size is the same (texts or values changed).
    pub relayout: bool,
    /// A sample text's width when the tree was last fitted: when the fonts change (the real ones arriving after the
    /// first frame), every text's preferred size does and the tree is laid out again.
    pub fitted_probe: Option<f32>,
    /// How text is measured for the preferred sizes.
    pub measure: Measure,
    /// The views' signals since the window's code last took them.
    pub signals: Vec<(Id, Signal)>,
    /// The view under the pointer, and the one a button is held on (`Window`'s mouse capture).
    pub hover: Option<Id>,
    pub pressed: Option<Id>,
    /// Set while a group's members are sized (their own sizes, without their groups); and the sizes worked out in one
    /// call (cleared at its start): the original keeps each view's preferred size and recomputes it when told it changed.
    no_groups: std::cell::Cell<bool>,
    depth: std::cell::Cell<u32>,
    cache: std::cell::RefCell<std::collections::HashMap<(Id, bool, bool), Point>>,
}

impl Tree {
    pub fn add(&mut self, node: Node) -> Id {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    /// `View::AddChild`: on top of the others.
    pub fn add_child(&mut self, parent: Id, child: Id) {
        self.nodes[child].parent = Some(parent);
        self.nodes[parent].children.push(child);
    }

    pub fn get(&self, id: Id) -> &Node {
        &self.nodes[id]
    }

    pub fn get_mut(&mut self, id: Id) -> &mut Node {
        &mut self.nodes[id]
    }

    /// The control of a view as its class.
    pub fn control<T: 'static>(&self, id: Id) -> Option<&T> {
        self.nodes.get(id)?.control.as_any().downcast_ref()
    }

    pub fn control_mut<T: 'static>(&mut self, id: Id) -> Option<&mut T> {
        self.nodes.get_mut(id)?.control.as_any_mut().downcast_mut()
    }

    /// `View::FindChild(name, recursive)`: the first child named so, depth first.
    pub fn find(&self, from: Id, name: &str) -> Option<Id> {
        for &c in &self.nodes[from].children {
            if self.nodes[c].name == name {
                return Some(c);
            }
            if let Some(f) = self.find(c, name) {
                return Some(f);
            }
        }
        None
    }

    /// Each view's `view_enable_expression` evaluated: enabled while it holds.
    pub fn apply_enable(&mut self, holds: &dyn Fn(&str) -> bool) {
        for n in &mut self.nodes {
            if !n.enable_expression.trim().is_empty() {
                n.enabled = holds(&n.enable_expression);
            }
        }
    }

    /// Forgets the preferred sizes (a control's own size changed).
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    /// GUI.dll FUN_1006f697: each view's `activate_criteria` evaluated (a `CriteriaMonitor_c`) and the view shown or
    /// hidden by it (`View::Show`); true when one changed.
    pub fn apply_criteria(&mut self, holds: &dyn Fn(&str) -> bool) -> bool {
        let mut changed = false;
        for n in &mut self.nodes {
            if n.criteria.trim().is_empty() {
                continue;
            }
            let off = !holds(&n.criteria);
            if off != n.criteria_off {
                n.criteria_off = off;
                n.hidden = if off { n.hidden + 1 } else { n.hidden.saturating_sub(1) };
                changed = true;
            }
        }
        if changed {
            self.cache.borrow_mut().clear();
        }
        changed
    }

    /// `IsVisible(false)`: neither it nor a parent hidden (the hide count `_Attached` passes down).
    pub fn is_visible(&self, mut id: Id) -> bool {
        loop {
            if self.nodes[id].hidden > 0 {
                return false;
            }
            match self.nodes[id].parent {
                Some(p) => id = p,
                None => return true,
            }
        }
    }

    /// Whether a child takes part in its parent's layout: visible, or hidden without [`FLAG_COLLAPSE`].
    pub fn laid_out(&self, id: Id) -> bool {
        self.is_visible(id) || self.nodes[id].flags & FLAG_COLLAPSE == 0
    }

    /// `UpdatePreferredSize` (0x1014acda): the calculated size clamped by the view's own limits.
    fn own_preferred(&self, id: Id, max: bool) -> Point {
        let n = &self.nodes[id];
        let mut p = self.calculate_preferred(id, max);
        p.x = p.x.max(n.min_size.x);
        p.y = p.y.max(n.min_size.y);
        if max {
            p.x = p.x.max(n.max_extend.x);
            p.y = p.y.max(n.max_extend.y);
        }
        p.x = p.x.min(n.max_limit.x);
        p.y = p.y.min(n.max_limit.y);
        p
    }

    /// `CalculatePreferredSize` (vtable +0x7c): the control's, else the layout node's (`View::CalculatePreferredSize`
    /// 0x1014a335: (-1, -1) without one).
    fn calculate_preferred(&self, id: Id, max: bool) -> Point {
        if let Some(p) = self.nodes[id].control.preferred(&self.measure, max) {
            return p;
        }
        super::layout::preferred(self, id, max)
    }

    /// The owner of a view's width (or height) group: `":our_parent:"` its parent, else its nearest ancestor so named
    /// (`View::_Attached` 0x1014f320); none without an owner name.
    fn group_owner(&self, id: Id, height: bool) -> Option<Id> {
        let n = &self.nodes[id];
        let (group, owner) = if height { (&n.height_group, &n.height_group_owner) } else { (&n.width_group, &n.width_group_owner) };
        if group.is_empty() || owner.is_empty() {
            return None;
        }
        if owner == ":our_parent:" {
            return n.parent;
        }
        let mut at = n.parent;
        while let Some(p) = at {
            if &self.nodes[p].name == owner {
                return Some(p);
            }
            at = self.nodes[p].parent;
        }
        None
    }

    /// Whether `a` holds `b`.
    pub fn is_ancestor(&self, a: Id, b: Id) -> bool {
        let mut at = self.nodes[b].parent;
        while let Some(p) = at {
            if p == a {
                return true;
            }
            at = self.nodes[p].parent;
        }
        false
    }

    fn subtree(&self, id: Id, out: &mut Vec<Id>) {
        out.push(id);
        for &c in &self.nodes[id].children {
            self.subtree(c, out);
        }
    }

    /// `GetPreferredSize(max)` (0x1014b47d): the view's own preferred size, with the width taken as the largest of its
    /// width group's members (and the height of its height group's): `PreferredSizeChanged`'s +0x44/+0x4c. The members
    /// are taken at their own sizes worked out without groups: two groups can hold each other's views (a border view's
    /// client copies its element's group), which the original settles through its cached sizes.
    pub fn preferred(&self, id: Id, max: bool) -> Point {
        let outer = self.depth.get() == 0;
        if outer {
            self.cache.borrow_mut().clear();
        }
        let key = (id, max, self.no_groups.get());
        if let Some(p) = self.cache.borrow().get(&key) {
            return *p;
        }
        self.depth.set(self.depth.get() + 1);
        let p = self.preferred_uncached(id, max);
        self.depth.set(self.depth.get() - 1);
        self.cache.borrow_mut().insert(key, p);
        p
    }

    fn preferred_uncached(&self, id: Id, max: bool) -> Point {
        let mut p = self.own_preferred(id, max);
        if self.no_groups.get() {
            return p;
        }
        for height in [false, true] {
            let Some(owner) = self.group_owner(id, height) else { continue };
            let name = if height { &self.nodes[id].height_group } else { &self.nodes[id].width_group };
            let mut members = Vec::new();
            self.subtree(owner, &mut members);
            for m in members {
                if m == id || self.is_ancestor(m, id) || self.group_owner(m, height) != Some(owner) {
                    continue;
                }
                let theirs = if height { &self.nodes[m].height_group } else { &self.nodes[m].width_group };
                if theirs != name {
                    continue;
                }
                self.no_groups.set(true);
                let q = self.own_preferred(m, max);
                self.no_groups.set(false);
                if height {
                    p.y = p.y.max(q.y);
                } else {
                    p.x = p.x.max(q.x);
                }
            }
        }
        p
    }

    /// `View::SetFrame` then the layout of its children (`_ReLayout`), all the way down.
    pub fn set_frame(&mut self, id: Id, frame: Rect) {
        // (the sizes are kept for the whole layout)
        if self.depth.get() == 0 {
            self.cache.borrow_mut().clear();
        }
        self.depth.set(self.depth.get() + 1);
        self.frame_inner(id, frame);
        self.depth.set(self.depth.get() - 1);
    }

    fn frame_inner(&mut self, id: Id, frame: Rect) {
        self.nodes[id].frame = frame;
        super::layout::layout(self, id);
        let children = self.nodes[id].children.clone();
        for c in children {
            let f = self.nodes[c].frame;
            self.frame_inner(c, f);
        }
    }

    /// A view's frame in the root's coordinates (`ConvertToWindow`); the root's is its bounds.
    pub fn window_frame(&self, id: Id) -> Rect {
        if self.nodes[id].parent.is_none() {
            return self.nodes[id].bounds();
        }
        let mut r = self.nodes[id].frame;
        let mut at = self.nodes[id].parent;
        while let Some(p) = at {
            if self.nodes[p].parent.is_none() {
                break;
            }
            r = r.offset(self.nodes[p].frame.left_top());
            at = self.nodes[p].parent;
        }
        r
    }
}
