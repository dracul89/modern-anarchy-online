//! `TextInputView_c`: a text field. The XML constructor (0x1014928a) reads `label` and `label_position` (FUN_101488df:
//! `left` 0, `right` 1, `above` 2, `below` 3, `inside` 4, anything else above) and `Initialize` (0x10148aff) builds:
//! - an inset border (`BorderView_c(rect, "", 0, 4)` 0x10126da4: the `GFX_GUI_INSET_*` pieces, no background) holding
//!   the editor, a `TextView_c` made from the same element (named `_editor`; its feature flags with 0xd added: it takes
//!   typing, the mouse and selections) 3 pixels in from the sides and 1 from the top and bottom (`SetClient`);
//! - without a label: the border alone over the view (a `LayoutNode`);
//! - left or right: the label (a `TextView_c` in font 5) 5 pixels from the border, side by side (`HLayoutNode`);
//! - above or below: the label in a row with a spacer after it (16000 at most), its borders 3 on the sides and 3 away
//!   from the border, over or under it (`VLayoutNode`);
//! - inside: the label drawn over the editor at 0.7 opacity while the editor is empty and has no keyboard
//!   (`UpdateLabelView` 0x10148986).
//! Enter in the editor is the field's Enter (`SlotEnterPressed`); its text is `GetText`/`SetText` (0x10148956, 0x10148947).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::controls::{BorderView, Colour, Pieces, TextView};
use super::font::{FontId, Measure};
use super::geom::{Point, Rect};
use super::host::Paint;
use super::view::{Control, Id, Layout, Node, Plain, Spacer, Tree};
use super::xml::Element;

/// The editor's name in the field.
pub const EDITOR: &str = "_editor";

/// `label_position`.
pub fn label_position(s: &str) -> u8 {
    match s.to_ascii_lowercase().as_str() {
        "left" => 0,
        "right" => 1,
        "below" => 3,
        "inside" => 4,
        _ => 2,
    }
}

/// The field itself (its layout and children do the work).
pub struct TextInputView {
    pub label: String,
    pub position: u8,
}

impl Control for TextInputView {
    fn class(&self) -> &'static str {
        "TextInputView"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// An inside label: drawn while the editor says it is empty and unfocused.
pub struct InsideLabel {
    pub text: String,
    pub show: Arc<AtomicBool>,
}

impl Control for InsideLabel {
    fn class(&self) -> &'static str {
        "TextView"
    }
    fn paint(&self, p: &Paint) {
        if self.show.load(Ordering::Relaxed) {
            let c = Colour::WHITE.rgb(p.env.palette);
            let faded = egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (255.0 * 0.7) as u8);
            super::controls::text(p, FontId::NORMAL, p.rect.min, &self.text, faded);
        }
    }
    fn preferred(&self, _m: &Measure, _max: bool) -> Option<Point> {
        Some(Point::new(0.0, 0.0))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The field from its XML element.
pub fn create(tree: &mut Tree, e: &Element) -> Id {
    let label = e.text("label", "");
    let position = label_position(&e.text("label_position", ""));
    // (View(element, 0, true): its own flags 0)
    let id = tree.add(Node::from_element(e, 0, Box::new(TextInputView { label: label.clone(), position })));
    // the editor: the element's text view, typing on
    let editor = TextView::create(tree, e);
    {
        let node = tree.get_mut(editor);
        node.name = EDITOR.to_string();
        node.borders = Rect::new(3.0, 1.0, 3.0, 1.0);
        node.criteria.clear();
        node.enable_expression.clear();
        node.width_group.clear();
        node.height_group.clear();
        node.width_group_owner.clear();
        node.height_group_owner.clear();
        node.fade_group.clear();
    }
    if let Some(t) = tree.control_mut::<TextView>(editor) {
        t.feature |= 0xd;
    }
    // the inset border round it
    let border = tree.add(Node::new("", Box::new(BorderView { pieces: Pieces::named("GFX_GUI_INSET", false), alpha: 1.0, colour: Colour::WHITE })));
    tree.get_mut(border).layout = Layout::Base;
    tree.add_child(border, editor);
    let label_view = |tree: &mut Tree, borders: Rect| {
        let mut t = TextView::new(&label);
        t.font = FontId::NORMAL;
        let mut n = Node::new("", Box::new(t));
        n.borders = borders;
        tree.add(n)
    };
    match (label.is_empty(), position) {
        (true, _) => {
            tree.get_mut(id).layout = Layout::Base;
            tree.add_child(id, border);
        }
        (false, 4) => {
            tree.get_mut(id).layout = Layout::Base;
            tree.add_child(id, border);
            let show = Arc::new(AtomicBool::new(false));
            if let Some(t) = tree.control_mut::<TextView>(editor) {
                t.empty_hint = Some(show.clone());
            }
            let mut n = Node::new("", Box::new(InsideLabel { text: label.clone(), show }));
            n.borders = Rect::new(3.0, 1.0, 3.0, 1.0);
            let inside = tree.add(n);
            tree.add_child(id, inside);
        }
        (false, 0) => {
            let l = label_view(tree, Rect::new(0.0, 0.0, 5.0, 0.0));
            tree.get_mut(id).layout = Layout::Horizontal;
            tree.add_child(id, l);
            tree.add_child(id, border);
        }
        (false, 1) => {
            let l = label_view(tree, Rect::new(5.0, 0.0, 0.0, 0.0));
            tree.get_mut(id).layout = Layout::Horizontal;
            tree.add_child(id, border);
            tree.add_child(id, l);
        }
        (false, above_or_below) => {
            // (a row: the label and a spacer after it)
            let l = label_view(tree, Rect::default());
            let mut row = Node::new("", Box::new(Plain));
            row.layout = Layout::Horizontal;
            row.borders = if above_or_below == 2 { Rect::new(3.0, 0.0, 3.0, 3.0) } else { Rect::new(3.0, 3.0, 3.0, 0.0) };
            let row = tree.add(row);
            tree.add_child(row, l);
            let spacer = tree.add(Node::new("", Box::new(Spacer { min: Point::new(0.0, 0.0), max: Point::new(16000.0, 1.0) })));
            tree.add_child(row, spacer);
            tree.get_mut(id).layout = Layout::Vertical;
            if above_or_below == 2 {
                tree.add_child(id, row);
                tree.add_child(id, border);
            } else {
                tree.add_child(id, border);
                tree.add_child(id, row);
            }
        }
    }
    id
}

/// The field's text (`GetText`).
pub fn text(tree: &Tree, field: Id) -> Option<&str> {
    let editor = tree.find(field, EDITOR)?;
    tree.control::<TextView>(editor).map(|t| t.text.as_str())
}

/// Sets the field's text (`SetText`).
pub fn set_text(tree: &mut Tree, field: Id, value: &str) {
    if let Some(editor) = tree.find(field, EDITOR)
        && let Some(t) = tree.control_mut::<TextView>(editor)
    {
        t.set_text(value);
        tree.relayout = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(xml: &str) -> (Tree, Id) {
        let doc = super::super::xml::parse(xml);
        let mut tree = Tree::default();
        let id = super::super::create(&mut tree, &doc[0]).unwrap();
        (tree, id)
    }

    #[test]
    fn builds_the_editor_in_its_border() {
        let (mut tree, id) = field(r#"<TextInputView feature_flags="TVF_NUMERIC" name="Cash" value="0"/>"#);
        assert_eq!(tree.get(id).control.class(), "TextInputView");
        let editor = tree.find(id, EDITOR).unwrap();
        let t = tree.control::<TextView>(editor).unwrap();
        assert_eq!(t.feature, super::super::text::flags::NUMERIC | 0xd);
        assert!(tree.get(editor).control.focusable());
        assert_eq!(text(&tree, id), Some("0"));
        set_text(&mut tree, id, "125");
        assert_eq!(text(&tree, id), Some("125"));
        // typing at the end, digits only
        let s = tree.get_mut(editor).control.text("7x");
        assert_eq!(s, Some(super::super::host::Signal::Edited));
        assert_eq!(text(&tree, id), Some("1257"));
    }

    #[test]
    fn places_the_label() {
        let (tree, id) = field(r#"<TextInputView label="Name" label_position="left"/>"#);
        assert_eq!(tree.get(id).layout, Layout::Horizontal);
        assert_eq!(tree.get(id).children.len(), 2);
        let (tree, id) = field(r#"<TextInputView label="Name"/>"#);
        assert_eq!(tree.get(id).layout, Layout::Vertical);
        let (tree, id) = field(r#"<TextInputView label="Search" label_position="inside"/>"#);
        assert_eq!(tree.get(id).layout, Layout::Base);
        assert_eq!(label_position("BELOW"), 3);
    }
}
