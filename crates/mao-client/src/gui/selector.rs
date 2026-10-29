//! `ViewSelector_c`: views of which one is shown at a time, over the whole selector.
//!
//! The XML constructor (0x10150634) reads `use_max_size` (true by default) and appends a view for each child element
//! (`AppendView`, `InsertView` 0x1015050e: the first is shown and the value 0, the others hidden). Its value is the shown
//! view's index (`GetValue` 0x1014ffbf); setting it hides that view and shows the new one sized to the selector's bounds
//! (`SetValue` 0x10150236, `FrameSized` 0x1015000e: the view's frame is the bounds, its own layout borders not counted).
//! Its preferred size is the largest of all its views' with `use_max_size`, else the shown view's
//! (`CalculatePreferredSize` 0x10150074).

use super::geom::Point;
use super::view::{Control, Id, Layout, Node, Tree};
use super::xml::Element;

pub struct ViewSelector {
    /// The shown view (-1 for none).
    pub value: i32,
    pub use_max: bool,
}

impl Control for ViewSelector {
    fn class(&self) -> &'static str {
        "ViewSelector"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A `ViewSelector` element with its views.
pub fn create(tree: &mut Tree, e: &Element) -> Id {
    let selector = ViewSelector { value: -1, use_max: e.bool("use_max_size", true) };
    let id = tree.add(Node::from_element(e, 4, Box::new(selector)));
    tree.get_mut(id).layout = Layout::Selector;
    for c in &e.children {
        if let Some(child) = super::create(tree, c) {
            tree.add_child(id, child);
            let first = tree.get(id).children.len() == 1;
            if first {
                if let Some(s) = tree.control_mut::<ViewSelector>(id) {
                    s.value = 0;
                }
            } else {
                tree.get_mut(child).hidden += 1;
            }
        }
    }
    id
}

/// `GetValue`.
pub fn value(tree: &Tree, id: Id) -> i32 {
    tree.control::<ViewSelector>(id).map_or(-1, |s| s.value)
}

/// `SetValue`: the view at `index` shown, the one before hidden.
pub fn set_value(tree: &mut Tree, id: Id, index: i32) {
    let Some(old) = tree.control::<ViewSelector>(id).map(|s| s.value) else { return };
    if old == index {
        return;
    }
    let children = tree.get(id).children.clone();
    if let Some(&c) = usize::try_from(old).ok().and_then(|k| children.get(k)) {
        tree.get_mut(c).hidden += 1;
    }
    if let Some(s) = tree.control_mut::<ViewSelector>(id) {
        s.value = index;
    }
    if let Some(&c) = usize::try_from(index).ok().and_then(|k| children.get(k)) {
        let n = tree.get_mut(c);
        n.hidden = n.hidden.saturating_sub(1);
    }
    tree.relayout = true;
}

/// `SetValue` by the shown view's name.
pub fn show_named(tree: &mut Tree, id: Id, name: &str) {
    if let Some(k) = tree.get(id).children.iter().position(|&c| tree.get(c).name.eq_ignore_ascii_case(name)) {
        set_value(tree, id, k as i32);
    }
}

/// `CalculatePreferredSize`: the largest of the views' (`use_max_size`), else the shown one's.
pub fn preferred(tree: &Tree, id: Id, max: bool) -> Point {
    let Some(s) = tree.control::<ViewSelector>(id) else { return Point::new(-1.0, -1.0) };
    let children = &tree.get(id).children;
    if !s.use_max {
        return usize::try_from(s.value).ok().and_then(|k| children.get(k)).map_or(Point::new(-1.0, -1.0), |&c| tree.preferred(c, max));
    }
    children.iter().fold(Point::new(-1.0, -1.0), |p, &c| {
        let q = tree.preferred(c, max);
        Point::new(p.x.max(q.x), p.y.max(q.y))
    })
}

/// `FrameSized`: every view over the whole bounds (only the shown one is drawn).
pub fn layout(tree: &mut Tree, id: Id) {
    let bounds = tree.get(id).bounds();
    for c in tree.get(id).children.clone() {
        tree.get_mut(c).frame = bounds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_view_at_a_time() {
        let doc = super::super::xml::parse(r#"<ViewSelector name="sel"><View name="a" min_size="Point(50,20)"/><View name="b" min_size="Point(80,10)"/></ViewSelector>"#);
        let mut tree = Tree::default();
        let id = super::super::create(&mut tree, &doc[0]).unwrap();
        let (a, b) = (tree.get(id).children[0], tree.get(id).children[1]);
        assert_eq!(value(&tree, id), 0);
        assert!(tree.is_visible(a) && !tree.is_visible(b));
        // the largest of both
        let p = tree.preferred(id, false);
        assert_eq!((p.x, p.y), (80.0, 20.0));
        show_named(&mut tree, id, "B");
        assert_eq!(value(&tree, id), 1);
        assert!(!tree.is_visible(a) && tree.is_visible(b));
        tree.set_frame(id, super::super::geom::Rect::new(0.0, 0.0, 99.0, 49.0));
        assert_eq!(tree.get(b).frame.right, 99.0);
    }
}
