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
//!
//! The window's tabs (FUN_100c4315): "Preferences" (the pages above), "Fixed keys" when the skin's
//! `OptionPanel/HotKeys.xml` has anything, and "Key bindings". The fixed keys (`HotKeyPanel_c` FUN_100bf5a2, filled
//! by FUN_100beee3) are a tree of `HotKeyGroup`s (their `label` a text of category 700) holding `HotKey`s: a label
//! and the key command's id (`down_id`, as `id:KEY_...`), each shown with the key the command is on
//! (`InputConfig_t::GetHotkey`), written as FUN_100be60f writes it (in aqua, the original's key names; text.mdb's
//! `NoKey` for none). Here the key is this client's own for the same command ([`fixed_keys`], and the keymap's
//! `original_key`); a command this client has no key for shows none. The groups of the game's and the skins' files are
//! all `mode="fixed"`, which makes their keys not editable (FUN_100beee3; the "enter a key" box FUN_100bf286 opens on a
//! double click only on an editable one), so none is here either. "Key bindings" is the original's
//! editor of the rebindable keys (`BindView_c` FUN_100bcfb3, `bindview.rs`), over this client's own keys.

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
    /// `opt_bitnum`, a check box's: the box stands for that bit of the setting's number, not for the whole setting
    /// (`OptionCheckBox_c` FUN_100c22ba; the seven boxes of `VisualFlags`).
    pub bit: Option<u32>,
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
        bit: None,
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
    // (`atoi` of the attribute, the shift taken modulo 32)
    let bit = e.attribute("opt_bitnum").map(|v| v.trim().parse::<i32>().unwrap_or(0) as u32 & 0x1f);
    if let Some(c) = tree.control_mut::<OptionCtl>(id) {
        c.bit = bit;
    }
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
        let (var, kind, scale, fmt, t, bit) = (c.var.clone(), c.kind.clone(), c.scale, c.fmt.clone(), c.typ, c.bit);
        let value = store.get(&var).cloned().unwrap_or(Variant::Int32(0));
        match kind {
            Kind::Check(check) => CheckBox::set(tree, check, bit.map_or(value.as_bool(), |b| (1i32 << b) & value.as_int32() != 0)),
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
                match c.bit {
                    // (FUN_100c21b7: the number with the bit set or cleared, as an integer)
                    Some(bit) => {
                        let (now, mask) = (current.as_int32(), 1i32 << bit);
                        if on != (now & mask != 0) {
                            store.set_user(&c.var, Variant::Int32(if on { now | mask } else { now & !mask }));
                        }
                    }
                    None if on != current.as_bool() => store.set_user(&c.var, typed(c.typ, old.as_ref(), if on { 1.0 } else { 0.0 })),
                    None => {}
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
    page_dirs(dirs).iter().any(|d| mao_formats::files::read_to_string(d.join("OptionPanel/Root.xml")).is_ok_and(|t| t.contains("label")))
}

/// The folders whose `OptionPanel/Root.xml` may hold the pages: the skin's own first, then — since the game's shipped
/// `Default/OptionPanel/Root.xml` is empty (GUI.dll fills the original's panel from code) — any AOU skin at hand, whose
/// Root.xml carries the same pages: the themes the settings know (`aou_mod =`) and the user's own `Gui/<skin>` folders
/// under `%LOCALAPPDATA%/Funcom/Anarchy Online/<hash>/Anarchy Online`. So the panel has the original's pages with any skin's art.
pub fn page_dirs(skin_dirs: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut out: Vec<std::path::PathBuf> = skin_dirs.to_vec();
    let settings = crate::settings::Settings::load();
    for theme in crate::theme::available(&settings.aou_mod) {
        if let Some(d) = crate::theme::theme_dir(&theme, &settings.aou_mod) {
            out.push(d);
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let base = std::path::Path::new(&local).join("Funcom/Anarchy Online");
        if let Ok(hashes) = std::fs::read_dir(&base) {
            for h in hashes.flatten() {
                let gui = h.path().join("Anarchy Online/Gui");
                if let Ok(skins) = std::fs::read_dir(&gui) {
                    let mut names: Vec<std::path::PathBuf> = skins.flatten().map(|e| e.path()).filter(|p| p.join("OptionPanel/Root.xml").is_file()).collect();
                    names.sort();
                    out.extend(names);
                }
            }
        }
    }
    out.dedup();
    out
}

/// A key command of the "Fixed keys" tab: its label and the original's id for it (`KEY_...`, without `id:`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedKey {
    pub label: String,
    pub id: String,
}

/// A group of the "Fixed keys" tab: the folders it stands in (their labels, outermost first) and its keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyGroup {
    pub path: Vec<String>,
    pub keys: Vec<FixedKey>,
}

/// The groups of a `HotKeys.xml` (FUN_100beee3): every `HotKeyGroup` that holds `HotKey`s, in the file's order.
/// A label beginning with `#` is the text of that name in category 700; an element without a label is left out,
/// with all it holds.
pub fn fixed_keys(text: &str) -> Vec<KeyGroup> {
    fn walk(e: &Element, path: &[String], out: &mut Vec<KeyGroup>) {
        let mut keys = Vec::new();
        for c in &e.children {
            let label = c.text("label", "");
            if label.is_empty() {
                continue;
            }
            let label = super::xml::ldb_text(&label);
            if c.name.eq_ignore_ascii_case("HotKeyGroup") {
                if !keys.is_empty() {
                    out.push(KeyGroup { path: path.to_vec(), keys: std::mem::take(&mut keys) });
                }
                let mut inner = path.to_vec();
                inner.push(label);
                walk(c, &inner, out);
            } else if c.name.eq_ignore_ascii_case("HotKey") {
                let id = c.text("down_id", "");
                keys.push(FixedKey { label, id: id.strip_prefix("id:").unwrap_or(&id).to_string() });
            }
        }
        if !keys.is_empty() {
            out.push(KeyGroup { path: path.to_vec(), keys });
        }
    }
    let mut out = Vec::new();
    for root in super::xml::parse(text) {
        walk(&root, &[], &mut out);
    }
    out
}

#[derive(Default)]
pub struct OptionPanel {
    pub open: bool,
    /// The "Client settings" link was clicked (the client's own window: display, keys, theme).
    pub client_settings: bool,
    pages: Vec<Page>,
    selected: usize,
    loaded: bool,
    /// The "Fixed keys" tab's groups (the skin's `OptionPanel/HotKeys.xml`).
    fixed: Vec<KeyGroup>,
    /// The open tab.
    tab: usize,
    /// The "Key bindings" tab (`BindView_c`), made with its rows when the window opens.
    binds: Option<super::bindview::BindView>,
}

impl OptionPanel {
    /// The pages of the first skin folder whose `OptionPanel/Root.xml` has any.
    fn load(&mut self, dirs: &[std::path::PathBuf]) {
        self.loaded = true;
        // (the fixed keys: the first skin folder whose `HotKeys.xml` has any)
        self.fixed = dirs.iter().filter_map(|d| mao_formats::files::read_to_string(d.join("OptionPanel/HotKeys.xml")).ok()).map(|text| fixed_keys(&text)).find(|groups| !groups.is_empty()).unwrap_or_default();
        for d in page_dirs(dirs) {
            let Ok(text) = mao_formats::files::read_to_string(d.join("OptionPanel/Root.xml")) else { continue };
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

    /// Opens the n-th tab.
    pub fn select_tab(&mut self, n: usize) {
        self.tab = n;
    }

    /// Picks the n-th page.
    pub fn select(&mut self, n: usize) {
        self.selected = n;
    }

    /// Whether the "Key bindings" tab's "Bind Key" box waits for a key: the game's keys go to [`OptionPanel::key_entered`]
    /// then (`InputConfig_t` +0x24).
    pub fn wants_key(&self) -> bool {
        self.binds.as_ref().is_some_and(|b| b.wants_key())
    }

    /// A key pressed while [`OptionPanel::wants_key`].
    pub fn key_entered(&mut self, key: crate::settings::Binding) {
        if let Some(b) = self.binds.as_mut() {
            b.key_entered(key);
        }
    }

    /// The window over this client's keys (the fixed keys shown, the rebindable ones edited in "Key bindings"); whether
    /// the keys changed.
    pub fn show(&mut self, ctx: &egui::Context, palette: &crate::theme::Palette, store: &mut Store, stats: &BTreeMap<u32, i64>, keymap: &mut crate::settings::Keymap) -> bool {
        if !self.open {
            self.binds = None;
            return false;
        }
        let skin = crate::skin::skin(ctx);
        if !self.loaded {
            let dirs: Vec<std::path::PathBuf> = skin.as_ref().map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
            self.load(&dirs);
        }
        let mut open = self.open;
        let binds = self.binds.get_or_insert_with(|| super::bindview::BindView::new(keymap));
        let mut changed = binds.show_dialog(ctx, palette, keymap);
        let (pages, selected, client_settings, fixed, tab) = (&mut self.pages, &mut self.selected, &mut self.client_settings, &self.fixed, &mut self.tab);
        // (the original's window is "Preferences", `OptionWindowConfig` 200,200 to 1009,683 by default: 809 by 483)
        crate::chrome::window(ctx, palette, "Option panel", "Preferences", &mut open, |w| w.default_size([800.0, 480.0]).resizable(true), |ui| {
            // (the tabs, FUN_100c4315: the fixed keys only when the skin's file has any)
            let names: Vec<&str> = if fixed.is_empty() { vec!["Preferences", "Key bindings"] } else { vec!["Preferences", "Fixed keys", "Key bindings"] };
            let mut now = (*tab).min(names.len() - 1);
            crate::chrome::tab_view(ui, palette, &names, &mut now, |ui, t| {
            if names[t] == "Fixed keys" {
                let height = ui.available_height().max(200.0);
                crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("option-fixed-keys").max_height(height).auto_shrink([false, false]), |ui| {
                    // (a folder's label is drawn as the text it is, markup and all: the game's "Fixed keys" is green)
                    let mut folders: &[String] = &[];
                    for group in fixed {
                        let (inner, outer) = group.path.split_last().map_or(("", &[][..]), |(last, rest)| (last.as_str(), rest));
                        if outer != folders {
                            for label in outer {
                                super::controls::markup(ui, palette, label, super::font::FontId::NORMAL, crate::chrome::rgb(palette.text));
                            }
                            folders = outer;
                        }
                        egui::CollapsingHeader::new(mao_net::chat::plain(inner)).id_salt(("fixed-keys", &group.path)).default_open(true).show(ui, |ui| {
                            for key in &group.keys {
                                ui.horizontal(|ui| {
                                    ui.label(&key.label);
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.add_space(12.0);
                                        // (FUN_100be60f: the key in aqua as the original names it, text.mdb 10000's
                                        // `NoKey` without one)
                                        let keys = keymap.original_key(&key.id);
                                        if keys.is_empty() {
                                            ui.label(super::xml::ldb_named(10000, "NoKey", ""));
                                        } else {
                                            let text = keys.iter().map(|k| super::bindview::key_name(Some(k))).collect::<Vec<_>>().join(", ");
                                            ui.label(egui::RichText::new(text).color(egui::Color32::from_rgb(0, 255, 255)));
                                        }
                                    });
                                });
                            }
                        });
                    }
                });
                return;
            }
            if names[t] == "Key bindings" {
                changed |= binds.show(ui, palette, keymap);
                return;
            }
            if pages.is_empty() {
                ui.label("No option pages were found: the game's own OptionPanel/Root.xml is empty and no AOU skin is at hand.");
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
            *tab = now;
        });
        self.open = open;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hot keys file's groups and keys, in order; an element without a label is left out with what it holds.
    #[test]
    fn a_hot_keys_file_is_groups_of_key_commands() {
        let text = r#"<?xml version="1.0" ?><root>
            <HotKeyGroup label="Fixed" mode="fixed">
                <HotKeyGroup label="Chat">
                    <HotKey down_id="id:KEY_COMMAND_START_CHAT_REPLY_INPUT" label="Reply"/>
                    <HotKey down_id="id:KEY_HIDDEN"/>
                </HotKeyGroup>
                <HotKeyGroup>
                    <HotKey down_id="id:KEY_LOST" label="Lost"/>
                </HotKeyGroup>
                <HotKeyGroup label="Targets">
                    <HotKey down_id="id:KEY_NEXT_HOSTILE_TARGET" label="Next"/>
                    <HotKey down_id="KEY_PREV_HOSTILE_TARGET" label="Previous"/>
                </HotKeyGroup>
            </HotKeyGroup>
        </root>"#;
        let key = |label: &str, id: &str| FixedKey { label: label.into(), id: id.into() };
        assert_eq!(
            fixed_keys(text),
            vec![
                KeyGroup { path: vec!["Fixed".into(), "Chat".into()], keys: vec![key("Reply", "KEY_COMMAND_START_CHAT_REPLY_INPUT")] },
                KeyGroup { path: vec!["Fixed".into(), "Targets".into()], keys: vec![key("Next", "KEY_NEXT_HOSTILE_TARGET"), key("Previous", "KEY_PREV_HOSTILE_TARGET")] },
            ]
        );
        assert!(fixed_keys("").is_empty() && fixed_keys("<root/>").is_empty());
    }

    /// `MAO_GAME`: the game's own file has the three groups, each key command with a label.
    #[test]
    fn the_games_fixed_keys() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let text = mao_formats::files::read_to_string(std::path::Path::new(&game).join("cd_image/gui/Default/OptionPanel/HotKeys.xml")).unwrap();
        let groups = fixed_keys(&text);
        assert_eq!(groups.iter().map(|g| g.keys.len()).collect::<Vec<_>>(), [4, 4, 8]);
        assert!(groups.iter().all(|g| g.path.len() == 2 && g.keys.iter().all(|k| k.id.starts_with("KEY_") && !k.label.is_empty())));
    }

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

    /// A check box with `opt_bitnum` stands for one bit of its setting's number: the shoulder pads, the helmet and the
    /// social tab are seven boxes on the one `VisualFlags`.
    #[test]
    fn a_bit_box_shows_and_sets_its_bit() {
        let doc = crate::gui::xml::parse(
            r#"<root><ScrollView label="Char/Looks"><ScrollViewChild><View view_layout="vertical">
            <OptionCheckBox label="Helmet" opt_type="variant" opt_variable="VisualFlags" opt_bitnum="2"/>
            <OptionCheckBox label="Social" opt_type="variant" opt_variable="VisualFlags" opt_bitnum="5"/>
            </View></ScrollViewChild></ScrollView></root>"#,
        );
        let mut tree = Tree::default();
        crate::gui::create(&mut tree, &doc[0].children[0]).unwrap();
        let mut store = Store::default();
        store.load_config(r#"<Root><Value name="VisualFlags" value="31"/></Root>"#);
        sync(&mut tree, &store);
        let boxes: Vec<Id> = controls(&tree).iter().map(|&i| match tree.control::<OptionCtl>(i).unwrap().kind { Kind::Check(c) => c, _ => panic!() }).collect();
        // (31: the five low bits; the helmet's is among them, the social tab's is not)
        assert!(CheckBox::checked(&tree, boxes[0]) && !CheckBox::checked(&tree, boxes[1]));
        // nothing touched: nothing written
        read_back(&tree, &mut store);
        assert_eq!(store.get("VisualFlags").map(|v| v.as_int32()), Some(31));
        // the helmet hidden, the social tab on: each its own bit, the others kept
        CheckBox::set(&mut tree, boxes[0], false);
        read_back(&tree, &mut store);
        assert_eq!(store.get("VisualFlags"), Some(&Variant::Int32(27)));
        sync(&mut tree, &store);
        CheckBox::set(&mut tree, boxes[1], true);
        read_back(&tree, &mut store);
        assert_eq!(store.get("VisualFlags"), Some(&Variant::Int32(59)));
    }
}
