//! `RadioButtonGroup_c`: a view whose radio buttons (its children's, at any depth) are one choice.
//!
//! A button turned on turns the group's others off and the group tells its window the button's value
//! (`RadioButtonActivated` 0x10140453: `_ClearButtons`, `ValueChanged`); its value is the first button on
//! (`GetValue` 0x101402f5, `_GetButtonState`); setting it turns on the buttons whose value it is and off the others
//! (`SetValue` 0x101405cb, `_SetButtonState`). A button that is on stays on when clicked again (a radio button is not a
//! toggle that goes off).

use super::controls::{CheckBox, RadioButton};
use super::host::Signal;
use super::view::{Control, Id, Node, Tree};
use super::xml::Element;

pub struct RadioButtonGroup;

impl Control for RadioButtonGroup {
    fn class(&self) -> &'static str {
        "RadioButtonGroup"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A `RadioButtonGroup` element with its children.
pub fn create(tree: &mut Tree, e: &Element) -> Id {
    let id = tree.add(Node::from_element(e, 4, Box::new(RadioButtonGroup)));
    super::load_child_views(tree, id, e);
    id
}

/// The radio buttons under a view, in order.
fn radios(tree: &Tree, id: Id, out: &mut Vec<Id>) {
    for &c in &tree.get(id).children {
        if tree.control::<RadioButton>(c).is_some() {
            out.push(c);
        }
        radios(tree, c, out);
    }
}

/// The group a view is in (its nearest group ancestor).
fn group_of(tree: &Tree, mut id: Id) -> Option<Id> {
    while let Some(p) = tree.get(id).parent {
        if tree.control::<RadioButtonGroup>(p).is_some() {
            return Some(p);
        }
        id = p;
    }
    None
}

/// `GetValue`: the value of the first button on.
pub fn value(tree: &Tree, group: Id) -> Option<String> {
    let mut all = Vec::new();
    radios(tree, group, &mut all);
    all.into_iter().find(|&r| CheckBox::checked(tree, r)).and_then(|r| tree.control::<RadioButton>(r).map(|b| b.value.clone()))
}

/// `SetValue`: the buttons of that value on, the others off.
pub fn set_value(tree: &mut Tree, group: Id, value: &str) {
    let mut all = Vec::new();
    radios(tree, group, &mut all);
    for r in all {
        let on = tree.control::<RadioButton>(r).is_some_and(|b| b.value == value);
        CheckBox::set(tree, r, on);
    }
}

/// The groups' part in a frame's signals: a radio button's toggle becomes its group's choice (`ValueChanged`, as the
/// group's [`Signal::Value`]); a button clicked off goes back on.
pub fn take_signals(tree: &mut Tree) {
    let signals = std::mem::take(&mut tree.signals);
    for (id, s) in signals {
        let radio = tree.get(id).parent.filter(|&p| tree.control::<RadioButton>(p).is_some());
        match (&s, radio.and_then(|r| group_of(tree, r).map(|g| (r, g)))) {
            (Signal::Toggled(true), Some((r, g))) => {
                let mut all = Vec::new();
                radios(tree, g, &mut all);
                for other in all.into_iter().filter(|&o| o != r) {
                    CheckBox::set(tree, other, false);
                }
                let v = tree.control::<RadioButton>(r).map(|b| b.value.clone()).unwrap_or_default();
                tree.signals.push((g, Signal::Value(v)));
            }
            (Signal::Toggled(false), Some((r, _))) => CheckBox::set(tree, r, true),
            _ => tree.signals.push((id, s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_choice() {
        let doc = super::super::xml::parse(r#"<RadioButtonGroup view_layout="horizontal" name="DeliveryMode"><RadioButton label="Standard" value="0"/><RadioButton label="Express" value="1"/></RadioButtonGroup>"#);
        let mut tree = Tree::default();
        let g = super::super::create(&mut tree, &doc[0]).unwrap();
        assert_eq!(value(&tree, g), None);
        set_value(&mut tree, g, "0");
        assert_eq!(value(&tree, g).as_deref(), Some("0"));
        // a click on Express's button: on, Standard off, the group's value
        let express = tree.get(g).children[1];
        let button = tree.get(express).children[0];
        CheckBox::set(&mut tree, express, true);
        tree.signals.push((button, Signal::Toggled(true)));
        take_signals(&mut tree);
        assert_eq!(tree.signals, vec![(g, Signal::Value("1".into()))]);
        assert_eq!(value(&tree, g).as_deref(), Some("1"));
        // clicked again it stays on
        CheckBox::set(&mut tree, express, false);
        tree.signals = vec![(button, Signal::Toggled(false))];
        take_signals(&mut tree);
        assert!(tree.signals.is_empty());
        assert_eq!(value(&tree, g).as_deref(), Some("1"));
    }
}
