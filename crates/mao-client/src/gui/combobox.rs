//! `ComboBox_c`: a text field with a list to pick its text from (the login window's user names).
//!
//! The XML constructor (0x10002454) is a `TextInputView_c` of the element; `Initialize` (0x10002216) gives the field's
//! inset border a horizontal layout (`HLayoutNode`) with, after the editor, a `BitmapView_c` of `GFX_GUI_COMBOBOX_CLOSED`
//! (0xb3) and `GFX_GUI_COMBOBOX_OPEN` (0xb4), value 0, 5 from the border's right (`_DAT_101a8ba0`). A press on that
//! picture (its bounds grown by 2, `_DAT_101a8b98`) flips its value and on 1 opens a `PopupMenu_c` (`Open` 0x10001ea4)
//! of the items (`SlotMenuOpening` 0x10001e10) at the editor's left, 3 (`_DAT_101a8a28`) under its bottom
//! (`SlotMenuOpened` 0x10001be6); it closes back to value 0 (`SlotMenuClosed`). Choosing an item makes it the field's
//! text, and a changed text is the editor's text-changed signal (`SlotMenuItemSelected` 0x100020f2). Items are a label
//! and an id (`AppendItem`, `InsertItem`, `DeleteItem`, `Clear`, `GetItemLabel`, `GetItemID`, `FindItemByID`).

use super::geom::{Point, Rect};
use super::host::{Env, Mouse, Paint, Signal};
use super::popup::{Entry, Outcome, PopupMenu};
use super::textinput::EDITOR;
use super::view::{Control, Id, Layout, Node, Tree};
use super::xml::Element;

pub const CLOSED: &str = "GFX_GUI_COMBOBOX_CLOSED";
pub const OPEN: &str = "GFX_GUI_COMBOBOX_OPEN";

/// The field: its items.
#[derive(Default)]
pub struct ComboBox {
    /// (label, id).
    pub items: Vec<(String, i32)>,
}

impl Control for ComboBox {
    fn class(&self) -> &'static str {
        "ComboBox"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The arrow beside the editor, and the list while it is open.
#[derive(Default)]
pub struct Arrow {
    menu: Option<PopupMenu>,
    want_open: bool,
    /// What the list shows and where (given before each frame's overlays: [`prepare`]).
    labels: Vec<String>,
    at: egui::Pos2,
}

impl Control for Arrow {
    fn class(&self) -> &'static str {
        "BitmapView"
    }
    fn preferred(&self, m: &super::font::Measure, _max: bool) -> Option<Point> {
        let s = m.ctx.as_ref().and_then(crate::skin::skin).and_then(|s| s.get(CLOSED)).map_or(egui::vec2(14.0, 14.0), |s| s.size);
        Some(Point::new(s.x - 1.0, s.y - 1.0))
    }
    fn paint(&self, p: &Paint) {
        let name = if self.menu.is_some() { OPEN } else { CLOSED };
        if let Some(skin) = p.env.skin {
            let size = skin.get(name).map_or(p.rect.size(), |s| s.size);
            let at = egui::Rect::from_min_size(egui::pos2(p.rect.left(), (p.rect.center().y - size.y * 0.5).floor()), size);
            skin.draw_in(p.painter, name, at, egui::Color32::WHITE);
        }
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    fn mouse(&mut self, m: Mouse, _at: Point) -> Option<Signal> {
        if m == Mouse::Down {
            if self.menu.is_some() {
                self.menu = None;
            } else {
                self.want_open = true;
            }
        }
        None
    }
    fn overlay(&mut self, ctx: &egui::Context, env: &Env, _rect: egui::Rect, id: egui::Id) -> Option<Signal> {
        if std::mem::take(&mut self.want_open) {
            let entries = self.labels.iter().map(|s| Entry { label: s.clone(), enabled: true, mark: None }).collect();
            self.menu = Some(PopupMenu::new(entries, self.at, None, ctx.cumulative_frame_nr()));
        }
        let menu = self.menu.as_mut()?;
        match menu.show(ctx, env, id.with("combobox")) {
            Outcome::None => None,
            Outcome::Closed => {
                self.menu = None;
                None
            }
            Outcome::Chosen(k) => {
                self.menu = None;
                Some(Signal::Selected(k as i64))
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

/// A `ComboBox` element: the text field with the arrow in its border.
pub fn create(tree: &mut Tree, e: &Element) -> Id {
    let id = super::textinput::create(tree, e);
    tree.get_mut(id).control = Box::new(ComboBox::default());
    if let Some(border) = tree.find(id, EDITOR).and_then(|ed| tree.get(ed).parent) {
        tree.get_mut(border).layout = Layout::Horizontal;
        let mut arrow = Node::new("", Box::new(Arrow::default()));
        arrow.borders = Rect::new(0.0, 0.0, 5.0, 0.0);
        let arrow = tree.add(arrow);
        tree.add_child(border, arrow);
    }
    id
}

/// The combo box a view is in.
fn field_of(tree: &Tree, mut id: Id) -> Option<Id> {
    while let Some(p) = tree.get(id).parent {
        if tree.control::<ComboBox>(p).is_some() {
            return Some(p);
        }
        id = p;
    }
    None
}

/// The arrows their lists' labels and places (the editor's left, 3 under its bottom), before the overlays.
pub fn prepare(tree: &mut Tree, origin: egui::Pos2) {
    let arrows: Vec<Id> = (0..tree.nodes.len()).filter(|&v| tree.control::<Arrow>(v).is_some()).collect();
    for a in arrows {
        let Some(field) = field_of(tree, a) else { continue };
        let labels = tree.control::<ComboBox>(field).map(|c| c.items.iter().map(|(l, _)| l.clone()).collect()).unwrap_or_default();
        let at = tree.find(field, EDITOR).map(|ed| super::host::screen_rect(tree, ed, origin)).map_or(egui::Pos2::ZERO, |r| egui::pos2(r.left(), r.bottom() - 1.0 + 3.0));
        if let Some(arrow) = tree.control_mut::<Arrow>(a) {
            arrow.labels = labels;
            arrow.at = at;
        }
    }
}

/// An arrow's chosen item becomes its field's text: the editor's [`Signal::Edited`] when that changes it.
pub fn take_signals(tree: &mut Tree) {
    let signals = std::mem::take(&mut tree.signals);
    for (id, s) in signals {
        let field = tree.control::<Arrow>(id).and_then(|_| field_of(tree, id));
        match (s, field) {
            (Signal::Selected(k), Some(field)) => {
                let label = tree.control::<ComboBox>(field).and_then(|c| c.items.get(k as usize)).map(|(l, _)| l.clone());
                if let Some(label) = label
                    && super::textinput::text(tree, field) != Some(label.as_str())
                {
                    super::textinput::set_text(tree, field, &label);
                    if let Some(editor) = tree.find(field, EDITOR) {
                        tree.signals.push((editor, Signal::Edited));
                    }
                }
            }
            (s, _) => tree.signals.push((id, s)),
        }
    }
}

/// `AppendItem`.
pub fn append_item(tree: &mut Tree, field: Id, label: &str, id: i32) {
    if let Some(c) = tree.control_mut::<ComboBox>(field) {
        c.items.push((label.to_string(), id));
    }
}

/// `Clear`.
pub fn clear(tree: &mut Tree, field: Id) {
    if let Some(c) = tree.control_mut::<ComboBox>(field) {
        c.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chosen_item_is_the_text() {
        let doc = super::super::xml::parse(r#"<ComboBox name="username" value="" min_size="Point(139,-1)"/>"#);
        let mut tree = Tree::default();
        let id = super::super::create(&mut tree, &doc[0]).unwrap();
        assert_eq!(tree.get(id).control.class(), "ComboBox");
        let editor = tree.find(id, EDITOR).unwrap();
        let border = tree.get(editor).parent.unwrap();
        assert_eq!(tree.get(border).layout, Layout::Horizontal);
        let arrow = tree.get(border).children[1];
        append_item(&mut tree, id, "sadlee", 1);
        append_item(&mut tree, id, "other", 2);
        tree.signals.push((arrow, Signal::Selected(1)));
        take_signals(&mut tree);
        assert_eq!(super::super::textinput::text(&tree, id), Some("other"));
        assert_eq!(tree.signals, vec![(editor, Signal::Edited)]);
        // the same again: no change, no signal
        tree.signals = vec![(arrow, Signal::Selected(1))];
        take_signals(&mut tree);
        assert!(tree.signals.is_empty());
    }
}
