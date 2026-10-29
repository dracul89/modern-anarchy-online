//! The option panel (GUI.dll's `OptionPanelModule_c`, `optionpanel_window`): the skin's `OptionPanel/Root.xml` (the game's
//! own is empty; the AOU skins fill it), one page per top element (a `ScrollView` of option controls), filed by its
//! `label` (`#name` looked up in the text database's category 700) split at `/` into folders, as FUN_100c3d62 fills
//! its folder list (a `StringListView` of folders and pages) and shows the page picked.
//!
//! The option controls (`OptionControl_c`, FUN_100c29cd) each show one setting, `opt_variable`: with `opt_type`
//! "variant" a distributed value (`dvalue.rs`), "int" or "float" one of `IndependentPrefs_t`'s preferences (in the same
//! store here; their defaults and ranges are GUI.dll's, [`super::dvalue::INDEPENDENT_PREFS`]). `OptionCheckBox` is a
//! check box with the label; `OptionSlider` (FUN_100c1178) the label, the value printed by `value_fmt` (the value times
//! `value_scale`) at the right and a slider over the setting's range under them; `OptionRadioButtonGroup` its label and
//! its `RadioButton`s, each standing for its `value`. A view's `view_enable_expression` enables it while it holds.
//! What the player changes is kept in their preferences (`Store::set_user`).

use std::collections::BTreeMap;

use super::controls::{CheckBox, RadioButton, Slider, TextView};
use super::dvalue::Store;
use super::geom::{Point, Rect};
use super::variant::Variant;
use super::view::{Control, Id, Layout, Node, Plain, Spacer, Tree};
use super::xml::Element;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptType {
    Variant,
    Int,
    Float,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Check(Id),
    Slider { slider: Id, value: Id },
    Radio(Vec<Id>),
}

/// An option control: the setting it shows and its parts.
pub struct OptionCtl {
    pub var: String,
    pub typ: OptType,
    pub kind: Kind,
    pub scale: f64,
    pub fmt: String,
}

impl Control for OptionCtl {
    fn class(&self) -> &'static str {
        "OptionControl"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn typ(e: &Element) -> OptType {
    match e.attribute("opt_type").map(str::to_ascii_lowercase).as_deref() {
        Some("int") => OptType::Int,
        Some("float") => OptType::Float,
        _ => OptType::Variant,
    }
}

/// An element holding only a label (for the inner check box and radio buttons).
fn labelled(name: &str, label: &str, value: Option<&str>) -> Element {
    let mut attributes = vec![("label".to_string(), label.to_string())];
    if let Some(v) = value {
        attributes.push(("value".to_string(), v.to_string()));
    }
    Element { name: name.to_string(), attributes, children: Vec::new(), text: String::new() }
}

fn wrapper(tree: &mut Tree, e: &Element, kind: Kind) -> Id {
    let ctl = OptionCtl {
        var: e.attribute("opt_variable").unwrap_or("").to_string(),
        typ: typ(e),
        kind,
        scale: e.attribute("value_scale").and_then(|v| v.trim().parse().ok()).unwrap_or(1.0),
        fmt: e.attribute("value_fmt").unwrap_or("%.0f").to_string(),
    };
    let id = tree.add(Node::from_element(e, 4, Box::new(ctl)));
    tree.get_mut(id).layout = Layout::Vertical;
    // (as wide as the page lets it be: a vertical layout's own maximum is its narrowest child's, which would cut a
    // group's label to its shortest button's)
    tree.get_mut(id).max_extend.x = 16000.0;
    tree.get_mut(id).h_align = super::view::Align::Left;
    id
}

fn set_kind(tree: &mut Tree, id: Id, kind: Kind) {
    if let Some(c) = tree.control_mut::<OptionCtl>(id) {
        c.kind = kind;
    }
}

/// `OptionCheckBox`.
pub fn create_check(tree: &mut Tree, e: &Element) -> Id {
    let id = wrapper(tree, e, Kind::Check(0));
    let check = CheckBox::create(tree, &labelled("CheckBox", e.attribute("label").unwrap_or(""), None));
    tree.add_child(id, check);
    set_kind(tree, id, Kind::Check(check));
    id
}

/// `OptionSlider` (FUN_100c1178): a row of the label, a spacer and the value, then the slider.
pub fn create_slider(tree: &mut Tree, e: &Element) -> Id {
    let id = wrapper(tree, e, Kind::Radio(Vec::new()));
    let row = tree.add(Node::new("", Box::new(Plain)));
    tree.get_mut(row).layout = Layout::Horizontal;
    let label = tree.add(Node::new("", Box::new(TextView::new(&e.text("label", "")))));
    let spacer = tree.add(Node::new("", Box::new(Spacer { min: Point::new(-1.0, -1.0), max: Point::new(16000.0, -1.0) })));
    let value = tree.add(Node::new("", Box::new(TextView::new(""))));
    for c in [label, spacer, value] {
        tree.add_child(row, c);
    }
    let slider = tree.add(Node::new("", Box::new(Slider::new(0.0, 1.0, 0.0))));
    tree.add_child(id, row);
    tree.add_child(id, slider);
    set_kind(tree, id, Kind::Slider { slider, value });
    id
}

/// `OptionRadioButtonGroup`: its label, then a radio button for each `RadioButton` child, 10 pixels in.
pub fn create_radio_group(tree: &mut Tree, e: &Element) -> Id {
    let id = wrapper(tree, e, Kind::Radio(Vec::new()));
    let label = tree.add(Node::new("", Box::new(TextView::new(&e.text("label", "")))));
    tree.add_child(id, label);
    let mut radios = Vec::new();
    for c in e.children.iter().filter(|c| c.name.eq_ignore_ascii_case("RadioButton")) {
        let r = RadioButton::create(tree, &labelled("RadioButton", c.attribute("label").unwrap_or(""), c.attribute("value")));
        tree.get_mut(r).borders = Rect::new(10.0, 0.0, 0.0, 0.0);
        tree.add_child(id, r);
        radios.push(r);
    }
    set_kind(tree, id, Kind::Radio(radios));
    id
}

/// A value written back in the setting's type (a distributed value keeps the type it has).
fn typed(t: OptType, old: Option<&Variant>, x: f64) -> Variant {
    match t {
        OptType::Int => Variant::Int32(x.round() as i32),
        OptType::Float => Variant::Float(x as f32),
        OptType::Variant => match old {
            Some(Variant::Bool(_)) => Variant::Bool(x != 0.0),
            Some(Variant::Int32(_)) => Variant::Int32(x.round() as i32),
            Some(Variant::Int64(_)) => Variant::Int64(x.round() as i64),
            Some(Variant::Double(_)) => Variant::Double(x),
            _ => Variant::Float(x as f32),
        },
    }
}

/// A radio button's value as a variant (`LoadFromString`).
fn radio_value(v: &str) -> Variant {
    let mut out = Variant::new();
    if !out.load_from_string(v.as_bytes()) {
        out = Variant::from(v);
    }
    out
}

/// The option controls of a tree.
fn controls(tree: &Tree) -> Vec<Id> {
    (0..tree.nodes.len()).filter(|&i| tree.get(i).control.as_any().is::<OptionCtl>()).collect()
}

/// Shows the settings' values in the controls (before the page is drawn).
pub fn sync(tree: &mut Tree, store: &Store) {
    for id in controls(tree) {
        let Some(c) = tree.control::<OptionCtl>(id) else { continue };
        let (var, kind, scale, fmt, t) = (c.var.clone(), c.kind.clone(), c.scale, c.fmt.clone(), c.typ);
        let value = store.get(&var).cloned().unwrap_or(Variant::Int32(0));
        match kind {
            Kind::Check(check) => CheckBox::set(tree, check, value.as_bool()),
            Kind::Radio(radios) => {
                for r in radios {
                    let on = tree.control::<RadioButton>(r).is_some_and(|b| radio_value(&b.value).equals(&value));
                    CheckBox::set(tree, r, on);
                }
            }
            Kind::Slider { slider, value: text } => {
                let (lo, hi) = store.range(&var).unwrap_or(if t == OptType::Float { (0.0, 1.0) } else { (0.0, 100.0) });
                let dragging = tree.pressed == Some(slider);
                let shown = match tree.control_mut::<Slider>(slider) {
                    Some(s) => {
                        s.min = lo;
                        s.max = hi;
                        if !dragging {
                            s.value = value.as_double();
                        }
                        s.value
                    }
                    None => value.as_double(),
                };
                if let Some(v) = tree.control_mut::<TextView>(text) {
                    v.set_text(&super::spells::printf_floats(&fmt, &[(shown * scale) as f32]));
                }
            }
        }
    }
}

/// The values the player changed in the controls, back to the settings (after the page is drawn).
pub fn read_back(tree: &Tree, store: &mut Store) {
    for id in controls(tree) {
        let Some(c) = tree.control::<OptionCtl>(id) else { continue };
        let old = store.get(&c.var).cloned();
        let current = old.clone().unwrap_or(Variant::Int32(0));
        match &c.kind {
            Kind::Check(check) => {
                let on = CheckBox::checked(tree, *check);
                if on != current.as_bool() {
                    store.set_user(&c.var, typed(c.typ, old.as_ref(), if on { 1.0 } else { 0.0 }));
                }
            }
            Kind::Radio(radios) => {
                // (the one turned on that is not the value now)
                for &r in radios {
                    let Some(b) = tree.control::<RadioButton>(r) else { continue };
                    let v = radio_value(&b.value);
                    if CheckBox::checked(tree, r) && !v.equals(&current) {
                        store.set_user(&c.var, typed(c.typ, old.as_ref(), v.as_double()));
                        break;
                    }
                }
            }
            Kind::Slider { slider, .. } => {
                if let Some(s) = tree.control::<Slider>(*slider) {
                    let changed = match c.typ {
                        OptType::Int => s.value.round() as i64 != current.as_int64(),
                        _ => (s.value - current.as_double()).abs() > 1e-6,
                    };
                    if changed {
                        store.set_user(&c.var, typed(c.typ, old.as_ref(), s.value));
                    }
                }
            }
        }
    }
}

struct Page {
    /// The folders and the page's name.
    path: Vec<String>,
    tree: Tree,
    root: Id,
}

/// Whether a skin's folders hold an option panel (an `OptionPanel/Root.xml` with something in it).
pub fn skin_has_panel(dirs: &[std::path::PathBuf]) -> bool {
    dirs.iter().any(|d| std::fs::read_to_string(d.join("OptionPanel/Root.xml")).is_ok_and(|t| t.contains("label")))
}

#[derive(Default)]
pub struct OptionPanel {
    pub open: bool,
    /// The "Client settings" link was clicked (the client's own window: display, keys, theme).
    pub client_settings: bool,
    pages: Vec<Page>,
    selected: usize,
    loaded: bool,
}

impl OptionPanel {
    /// The pages of the first skin folder whose `OptionPanel/Root.xml` has any.
    fn load(&mut self, dirs: &[std::path::PathBuf]) {
        self.loaded = true;
        for d in dirs {
            let Ok(text) = std::fs::read_to_string(d.join("OptionPanel/Root.xml")) else { continue };
            let doc = super::xml::parse(&text);
            let Some(root) = doc.first() else { continue };
            for e in &root.children {
                let label = e.text("label", "");
                if label.is_empty() {
                    continue;
                }
                let mut tree = Tree::default();
                let Some(id) = super::create(&mut tree, e) else { continue };
                let path = label.split('/').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                self.pages.push(Page { path, tree, root: id });
            }
            if !self.pages.is_empty() {
                return;
            }
        }
    }

    /// Picks the n-th page.
    pub fn select(&mut self, n: usize) {
        self.selected = n;
    }

    pub fn show(&mut self, ctx: &egui::Context, palette: &crate::theme::Palette, store: &mut Store, stats: &BTreeMap<u32, i64>) {
        if !self.open {
            return;
        }
        let skin = crate::skin::skin(ctx);
        if !self.loaded {
            let dirs: Vec<std::path::PathBuf> = skin.as_ref().map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
            self.load(&dirs);
        }
        let mut open = self.open;
        let (pages, selected, client_settings) = (&mut self.pages, &mut self.selected, &mut self.client_settings);
        crate::chrome::window(ctx, palette, "Option panel", "Options", &mut open, |w| w.default_size([640.0, 460.0]).resizable(true), |ui| {
            if pages.is_empty() {
                ui.label("This skin has no option panel (its OptionPanel/Root.xml is empty).");
                return;
            }
            let height = ui.available_height().max(200.0);
            ui.horizontal_top(|ui| {
                // the folder list
                ui.vertical(|ui| {
                    ui.set_width(170.0);
                    crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("option-folders").max_height(height), |ui| {
                        let mut folders: Vec<&str> = Vec::new();
                        for p in pages.iter() {
                            if p.path.len() > 1 && !folders.contains(&p.path[0].as_str()) {
                                folders.push(&p.path[0]);
                            }
                        }
                        let mut pick = None;
                        for (i, p) in pages.iter().enumerate().filter(|(_, p)| p.path.len() <= 1) {
                            if ui.selectable_label(*selected == i, p.path.first().map_or("", String::as_str)).clicked() {
                                pick = Some(i);
                            }
                        }
                        for f in folders {
                            egui::CollapsingHeader::new(f).default_open(true).show(ui, |ui| {
                                for (i, p) in pages.iter().enumerate().filter(|(_, p)| p.path.len() > 1 && p.path[0] == f) {
                                    if ui.selectable_label(*selected == i, p.path[1..].join("/")).clicked() {
                                        pick = Some(i);
                                    }
                                }
                            });
                        }
                        if let Some(i) = pick {
                            *selected = i;
                        }
                        // (what the original has no option for: the window mode, resolution, keys and theme)
                        ui.separator();
                        if ui.link("Client settings").clicked() {
                            *client_settings = true;
                        }
                    });
                });
                ui.separator();
                // the page
                let Some(page) = pages.get_mut(*selected) else { return };
                let s: &Store = store;
                page.tree.apply_criteria(&|c| s.criteria(c, stats, &|_| None));
                page.tree.apply_enable(&|c| s.criteria(c, stats, &|_| None));
                sync(&mut page.tree, store);
                // (the values' texts change, and the fonts' sizes settle after the first frames)
                page.tree.relayout = true;
                crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt(("option-page", *selected)).max_height(height).auto_shrink([false, false]), |ui| {
                    super::host::show(ui, &mut page.tree, page.root, skin.as_deref(), palette);
                });
                read_back(&page.tree, store);
                page.tree.signals.clear();
            });
        });
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_follow_and_set_their_settings() {
        let doc = crate::gui::xml::parse(
            r#"<root><ScrollView label="Audio/Audio Settings"><ScrollViewChild><View view_layout="vertical">
            <OptionCheckBox label="Sound" opt_type="variant" opt_variable="SoundOnOff"/>
            <OptionSlider label="Master" opt_type="variant" opt_variable="MasterVolume" value_fmt="%.0f%%" value_scale="100"/>
            <OptionRadioButtonGroup label="Fog" opt_type="int" opt_variable="FogMode"><RadioButton label="Off" value="0"/><RadioButton label="Full" value="3"/></OptionRadioButtonGroup>
            </View></ScrollViewChild></ScrollView></root>"#,
        );
        let mut tree = Tree::default();
        crate::gui::create(&mut tree, &doc[0].children[0]).unwrap();
        let mut store = Store::default();
        store.load_config(r#"<Root><Value name="SoundOnOff" value="true"/><Value max="1.0" min="0.0" name="MasterVolume" value="0.5"/><Value name="FogMode" value="3"/></Root>"#);
        sync(&mut tree, &store);
        let ids = controls(&tree);
        assert_eq!(ids.len(), 3);
        let kinds: Vec<Kind> = ids.iter().map(|&i| tree.control::<OptionCtl>(i).unwrap().kind.clone()).collect();
        let Kind::Check(check) = kinds[0] else { panic!() };
        assert!(CheckBox::checked(&tree, check));
        let Kind::Slider { slider, value } = kinds[1] else { panic!() };
        assert_eq!(tree.control::<Slider>(slider).unwrap().value, 0.5);
        assert_eq!(tree.control::<TextView>(value).unwrap().text, "50%");
        let Kind::Radio(radios) = kinds[2].clone() else { panic!() };
        assert!(!CheckBox::checked(&tree, radios[0]) && CheckBox::checked(&tree, radios[1]));
        // the player turns the sound off, picks fog off and slides the volume
        CheckBox::set(&mut tree, check, false);
        CheckBox::set(&mut tree, radios[0], true);
        tree.control_mut::<Slider>(slider).unwrap().value = 0.25;
        read_back(&tree, &mut store);
        assert!(!store.get_bool("SoundOnOff"));
        assert_eq!(store.get("FogMode"), Some(&Variant::Int32(0)));
        assert_eq!(store.get("MasterVolume").map(|v| v.as_double()), Some(0.25));
    }
}
