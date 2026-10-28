//! GUI.dll's view system, ported: the view tree and its layout nodes (`view.rs`, `layout.rs`), the skin's XML layout files
//! (`xml.rs`, `Views/*.xml`) and the classes they name. `docs/GUI_DLL_PARITY.md` lists what GUI.dll holds and what is
//! here.
//!
//! The classes are registered as GUI.dll registers them for `XMLObject_c::CreateObject` (FUN_1012fb49: `BorderView`,
//! `Button`, `CheckBox`, `DropdownMenu`, `RadioButton`, `RadioButtonGroup`, `StringListView`, `TextInputView`, `TextView`,
//! `View`, `ViewSelector`, `VLayoutSpacer`, `HLayoutSpacer`, `Slider`, `ScrollViewChild`, `ScrollView`, `PowerBar`,
//! `BitmapView`, `ComboBox`, `TextButton`; FUN_100316b2: `ItemSlotView`, `ItemIconView`, `CCMenu`, `CCMiniToolbar`,
//! `ScrollingTextView`; FUN_100c306d: `OptionCheckBox`, `OptionRadioButtonGroup`). A class not ported yet is made as a
//! plain view that keeps its class name ([`Unported`]), so a file still lays out.

pub mod geom;
pub mod layout;
pub mod view;
pub mod xml;

use geom::Point;
use view::{Control, Id, Layout, Node, Plain, Spacer, Tree};
use xml::Element;

/// The XML classes GUI.dll registers (names are matched in any case).
pub const CLASSES: [&str; 27] = [
    "BorderView", "Button", "CheckBox", "DropdownMenu", "RadioButton", "RadioButtonGroup", "StringListView", "TextInputView",
    "TextView", "View", "ViewSelector", "VLayoutSpacer", "HLayoutSpacer", "Slider", "ScrollViewChild", "ScrollView", "PowerBar",
    "BitmapView", "ComboBox", "TextButton", "ItemSlotView", "ItemIconView", "CCMenu", "CCMiniToolbar", "ScrollingTextView",
    "OptionCheckBox", "OptionRadioButtonGroup",
];

/// A registered class whose own behaviour is not ported yet: laid out as a plain `View` (its children loaded).
pub struct Unported(pub &'static str);

impl Control for Unported {
    fn class(&self) -> &'static str {
        self.0
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// `XMLObject_c::CreateObject`: the view for an element of a registered class, with its children; `None` for an element
/// no class is registered for.
pub fn create(tree: &mut Tree, e: &Element) -> Option<Id> {
    let class = CLASSES.iter().find(|c| c.eq_ignore_ascii_case(&e.name))?;
    let id = match *class {
        "View" => {
            // (View::Instantiate 0x100010b6: flags 4, children loaded)
            let id = tree.add(Node::from_element(e, 4, Box::new(Plain)));
            load_child_views(tree, id, e);
            id
        }
        "HLayoutSpacer" | "VLayoutSpacer" => {
            // (HLayoutSpacer 0x10130e49 / VLayoutSpacer 0x10130db4: min_size and max_size read as plain numbers, FUN_10130d5d,
            // for the length; across, 0 to 16000; kept less (1, 1))
            let length = |name: &str, default: f32| e.attribute(name).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(default);
            let (lo, hi) = (length("min_size", 0.0), length("max_size", 16000.0));
            let (min, max) = if class.starts_with('H') {
                (Point::new(lo, 0.0), Point::new(hi, 16000.0))
            } else {
                (Point::new(0.0, lo), Point::new(16000.0, hi))
            };
            let spacer = Spacer { min: min - Point::new(1.0, 1.0), max: max - Point::new(1.0, 1.0) };
            // (View's own constructor reads the same min_size/max_size as points: a number loads as (0, 0), so a spacer given
            // either is at least a pixel across)
            tree.add(Node::from_element(e, 4, Box::new(spacer)))
        }
        other => {
            let id = tree.add(Node::from_element(e, 4, Box::new(Unported(other))));
            load_child_views(tree, id, e);
            id
        }
    };
    Some(id)
}

/// `View::LoadChildViews` (0x1014e092): the layout node from `view_layout` ("horizontal", "stacked", else vertical) and a
/// view for each child element.
pub fn load_child_views(tree: &mut Tree, id: Id, e: &Element) {
    if e.children.is_empty() {
        return;
    }
    tree.get_mut(id).layout = match e.attribute("view_layout") {
        Some(v) if v.eq_ignore_ascii_case("stacked") => Layout::Stacked,
        Some(v) if v.eq_ignore_ascii_case("horizontal") => Layout::Horizontal,
        _ => Layout::Vertical,
    };
    let stacked = tree.get(id).layout == Layout::Stacked;
    for c in &e.children {
        if let Some(child) = create(tree, c) {
            if stacked {
                tree.get_mut(child).resize_mask = 0xf;
            }
            tree.add_child(id, child);
        }
    }
}

/// A views file's view (`XMLObject_c::LoadXMLObject(path, name)`, FUN_1000a1a1): the tree and its root.
pub fn load(text: &str, name: &str) -> Option<(Tree, Id)> {
    let doc = xml::parse(text);
    let e = xml::view_element(&doc, name)?;
    let mut tree = Tree::default();
    let root = create(&mut tree, &e)?;
    Some((tree, root))
}

/// A views file of the skin: `Views/<file>` from the first of the skin folders that has it (`skin::dirs`: the user's skin,
/// the theme's, the game's Default).
pub fn load_view(dirs: &[std::path::PathBuf], file: &str, name: &str) -> Option<(Tree, Id)> {
    let text = dirs.iter().find_map(|d| std::fs::read(d.join("Views").join(file)).ok())?;
    load(&String::from_utf8_lossy(&text), name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_lays_out_the_tip_view() {
        let (mut t, root) = load(include_str!("testdata/TipView.xml"), "").unwrap();
        assert_eq!(t.get(root).layout, Layout::Vertical);
        let browser = t.find(root, "BrowserView").unwrap();
        assert_eq!(t.get(browser).control.class(), "TextView");
        t.set_frame(root, geom::Rect::new(0.0, 0.0, 399.0, 299.0));
        let boxes = t.find(root, "DisableTipsCheckBox").unwrap();
        // (the text takes what the check boxes leave; both stay inside the window)
        let f = t.window_frame(browser);
        assert!(f.left >= 3.0 && f.right <= 396.0, "{f:?}");
        assert!(t.window_frame(boxes).bottom <= 299.0);
    }

    #[test]
    fn every_view_file_of_the_game_loads() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let dir = std::path::Path::new(&game).join("cd_image/gui/Default/Views");
        let mut count = 0;
        for f in std::fs::read_dir(&dir).unwrap() {
            let path = f.unwrap().path();
            let text = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
            let (mut t, root) = load(&text, "").unwrap_or_else(|| panic!("{} did not load", path.display()));
            let size = t.preferred(root, false);
            // (a window the size of a screen, or the view's own if larger: a stacked view has no preferred size)
            t.set_frame(root, geom::Rect::new(0.0, 0.0, size.x.max(1023.0), size.y.max(767.0)));
            for n in &t.nodes {
                assert!(n.frame.width() >= -1.0 && n.frame.height() >= -1.0, "{}: {} {:?}", path.display(), n.name, n.frame);
            }
            count += 1;
        }
        assert_eq!(count, 27);
    }
}
