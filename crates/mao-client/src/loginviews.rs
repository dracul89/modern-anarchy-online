//! The login and character selection screens on the skin's views, as GUI.dll's `LoginWindow_c` and `CharSelectWindow_c` build
//! them: `Views/LoginWindow.xml` (the username list, the password, Login, Quit; Remove forgets a name), and the full-screen
//! `Views/CharacterSelectionWindow.xml` (stacked: the list panel at the right, Play centred above a bar with Back and Delete)
//! with a `Views/CharacterSelectionItem.xml` for each character in `characters_view` (`CharSelectItem_c` FUN_1000e99a, laid
//! out down the list: `SetLayoutNode(VLayoutNode)` in `CharSelectWindow_c`'s constructor). An item shows its summary line
//! (`summary_view`: level, breed, profession and a status in brackets) or, selected, the details (`detailed_view`: level,
//! gender, breed, profession, location, status; `SetupView` FUN_1000e48e); its name is a toggled `TextButton` (red while
//! selected). An inactive character's status is "Inactive" in red (0xee4444). Delete opens `Views/CharacterDeleteWindow.xml`
//! (`CharDeleteWindow_c`): text.mdb's `DeleteCharacter` with the name, and OK only once the name is typed (else `MatchError`).

use mao_net::system::Character;

use crate::gui::view::{Id, Tree};
use crate::gui::window::XmlWindow;
use crate::theme::Palette;

/// What the login window asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum LoginAction {
    Login,
    Quit,
    /// Forget this account name.
    Remove(String),
}

/// What the character screen asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum CharAction {
    Play(String),
    Back,
    New,
    /// Delete the character with this id (its name typed to confirm).
    Delete(u32),
}

fn dirs(ctx: &egui::Context) -> Vec<std::path::PathBuf> {
    crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default()
}

fn set_text(tree: &mut Tree, at: Id, name: &str, value: &str) {
    let Some(id) = tree.find(at, name) else { return };
    if let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id) {
        if t.text != value {
            t.text = value.to_string();
            tree.relayout = true;
        }
    }
}

fn show_view(tree: &mut Tree, id: Option<Id>, on: bool) {
    let Some(id) = id else { return };
    // (taken out of the layout while hidden, as `SetSelected` takes the view out of the item: `RemoveChild`)
    tree.get_mut(id).flags |= crate::gui::view::FLAG_COLLAPSE;
    let want = u32::from(!on);
    if tree.get(id).hidden.min(1) != want {
        tree.get_mut(id).hidden = want;
        tree.relayout = true;
    }
}

/// `LoginWindow_c` on `Views/LoginWindow.xml`.
#[derive(Default)]
pub struct LoginWindow {
    window: Option<XmlWindow>,
    /// The names the username list was filled with.
    listed: Vec<String>,
}

impl LoginWindow {
    fn load(&mut self, ctx: &egui::Context) {
        let Some((mut tree, root)) = crate::gui::load_view(&dirs(ctx), "LoginWindow.xml", "") else { return };
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        // (the password typed as stars: `TVF_PASSWORD` on the field's text)
        if let Some(f) = tree.find(root, "password")
            && let Some(editor) = tree.find(f, "_editor")
            && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(editor)
        {
            t.feature |= crate::gui::text::flags::PASSWORD;
        }
        self.window = Some(XmlWindow { name: "Login".into(), title: "Login".into(), tree, root, open: true, default_size: egui::vec2(300.0, 0.0) });
        self.listed.clear();
    }

    /// Shows the window over the screen's middle; `user` and `password` follow the fields.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, user: &mut String, password: &mut String, accounts: &[String], message: &str, busy: bool) -> Option<LoginAction> {
        if self.window.is_none() {
            self.load(ctx);
            if let Some(w) = self.window.as_mut() {
                let root = w.root;
                if let Some(id) = w.tree.find(root, "username") {
                    crate::gui::textinput::set_text(&mut w.tree, id, user);
                }
            }
        }
        let w = self.window.as_mut()?;
        let root = w.root;
        // (the username list: the accounts used before)
        if self.listed != accounts
            && let Some(id) = w.tree.find(root, "username")
        {
            crate::gui::combobox::clear(&mut w.tree, id);
            for (k, a) in accounts.iter().enumerate() {
                crate::gui::combobox::append_item(&mut w.tree, id, a, k as i32);
            }
            self.listed = accounts.to_vec();
        }
        if let Some(id) = w.tree.find(root, "login_btn") {
            w.tree.get_mut(id).enabled = !busy;
        }
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
        let tree = &mut w.tree;
        let mut open = true;
        crate::chrome::window(ctx, palette, "Login", "Login", &mut open, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).default_size(min), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            crate::gui::combobox::prepare(tree, response.rect.min);
            if busy {
                ui.label("Logging in...");
            } else if !message.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(240, 120, 110), message);
            }
        });
        crate::gui::combobox::take_signals(&mut w.tree);
        if let Some(id) = w.tree.find(root, "username") {
            *user = crate::gui::textinput::text(&w.tree, id).unwrap_or("").to_string();
        }
        if let Some(id) = w.tree.find(root, "password") {
            *password = crate::gui::textinput::text(&w.tree, id).unwrap_or("").to_string();
        }
        let mut action = (!open).then_some(LoginAction::Quit);
        for (id, signal) in std::mem::take(&mut w.tree.signals) {
            let name = w.tree.get(id).name.clone();
            match (name.as_str(), signal) {
                ("login_btn", crate::gui::host::Signal::Clicked) | ("password", crate::gui::host::Signal::Enter) => action = Some(LoginAction::Login),
                ("quit_btn", crate::gui::host::Signal::Clicked) => action = Some(LoginAction::Quit),
                ("remove_btn", crate::gui::host::Signal::Clicked) if !user.trim().is_empty() => {
                    action = Some(LoginAction::Remove(user.trim().to_string()));
                    if let Some(id) = w.tree.find(root, "username") {
                        crate::gui::textinput::set_text(&mut w.tree, id, "");
                    }
                }
                _ => {}
            }
        }
        // (Enter in the password field)
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !busy && w.tree.find(root, "password").is_some_and(|f| w.tree.focus.is_some_and(|k| w.tree.is_ancestor(f, k))) {
            action = Some(LoginAction::Login);
        }
        action
    }
}

/// The confirmation (`CharDeleteWindow_c`).
struct DeleteWindow {
    window: XmlWindow,
    character: Character,
}

/// `CharSelectWindow_c` on `Views/CharacterSelectionWindow.xml`.
#[derive(Default)]
pub struct CharSelectScreen {
    view: Option<(Tree, Id)>,
    /// The characters the items were made for, and each one's item.
    made_for: Vec<(u32, String, i32, String)>,
    /// Each character's item, in the list's order.
    items: Vec<Id>,
    delete: Option<DeleteWindow>,
    pub status: String,
}

/// An item's texts (`SetupView`).
fn fill_item(tree: &mut Tree, item: Id, c: &Character, selected: bool) {
    if let Some(b) = tree.find(item, "name_btn")
        && let Some(t) = tree.control_mut::<crate::gui::textbutton::TextButton>(b)
    {
        t.texts = [c.name.clone(), c.name.clone(), c.name.clone()];
        t.toggle = true;
        t.on = selected;
    }
    let gender = {
        let mut g = crate::launcher::sex(c.sex).to_string();
        if let Some(f) = g.get_mut(0..1) {
            f.make_ascii_uppercase();
        }
        if g.is_empty() { "Neuter".to_string() } else { g }
    };
    let status = if c.banned { format!("Banned {}", c.ban_reason.trim()) } else if c.flag & 1 == 0 { "Inactive".to_string() } else { String::new() };
    for view in ["summary_view", "detailed_view"] {
        let Some(v) = tree.find(item, view) else { continue };
        set_text(tree, v, "level", &c.level.to_string());
        set_text(tree, v, "breed", crate::launcher::breed(c.breed));
        set_text(tree, v, "profession", crate::launcher::profession(c.profession));
        set_text(tree, v, "gender", &gender);
        set_text(tree, v, "location", c.area.trim());
        set_text(tree, v, "status", if view == "detailed_view" && status.is_empty() { "Active" } else { &status });
        // (the brackets round the status only when there is one; an inactive or banned one red)
        for bracket in ["status_left", "status_right"] {
            let id = tree.find(v, bracket);
            show_view(tree, id, !status.is_empty());
        }
        if let Some(s) = tree.find(v, "status")
            && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(s)
        {
            t.colour = if status.is_empty() { crate::gui::controls::Colour::WHITE } else { crate::gui::controls::Colour(0xee4444) };
        }
    }
    let (summary, detailed) = (tree.find(item, "summary_view"), tree.find(item, "detailed_view"));
    show_view(tree, summary, !selected);
    show_view(tree, detailed, selected);
}

impl CharSelectScreen {
    fn load(&mut self, ctx: &egui::Context) {
        let Some((mut tree, root)) = crate::gui::load_view(&dirs(ctx), "CharacterSelectionWindow.xml", "") else { return };
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        if let Some(list) = tree.find(root, "characters_view") {
            tree.get_mut(list).layout = crate::gui::view::Layout::Vertical;
        }
        self.view = Some((tree, root));
        self.made_for.clear();
        self.items.clear();
    }

    /// The items again when the list changed.
    fn build(&mut self, ctx: &egui::Context, list: &[&Character]) {
        let key: Vec<(u32, String, i32, String)> = list.iter().map(|c| (c.id, c.name.clone(), c.level, c.area.clone())).collect();
        if self.made_for == key {
            return;
        }
        self.load(ctx);
        let Some((tree, root)) = self.view.as_mut() else { return };
        let Some(text) = dirs(ctx).iter().find_map(|d| std::fs::read(d.join("Views").join("CharacterSelectionItem.xml")).ok()) else { return };
        let doc = crate::gui::xml::parse(&String::from_utf8_lossy(&text));
        let Some(element) = crate::gui::xml::view_element(&doc, "") else { return };
        let Some(holder) = tree.find(*root, "characters_view") else { return };
        // (the empty view the file keeps in the list "so the layout does not go bonkers" stays first)
        for _ in list {
            if let Some(item) = crate::gui::create(tree, &element) {
                tree.add_child(holder, item);
                self.items.push(item);
            }
        }
        tree.relayout = true;
        self.made_for = key;
    }

    /// Shows the screen over the whole window; `selected` follows the clicks.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, list: &[&Character], selected: &mut Option<String>) -> Option<CharAction> {
        self.build(ctx, list);
        let mut action = None;
        let (tree, root) = self.view.as_mut()?;
        let root = *root;
        for (item, c) in self.items.clone().into_iter().zip(list) {
            fill_item(tree, item, c, selected.as_deref() == Some(c.name.as_str()));
        }
        let usable = |name: &str| list.iter().any(|c| c.name == name && c.created && !c.banned);
        if let Some(b) = tree.find(root, "login_btn") {
            tree.get_mut(b).enabled = selected.as_deref().is_some_and(usable);
        }
        if let Some(b) = tree.find(root, "delete_btn") {
            tree.get_mut(b).enabled = selected.is_some() && self.delete.is_none();
        }
        set_text(tree, root, "slots_available", &format!("{} characters", list.len()));
        let skin = crate::skin::skin(ctx);
        let screen = ctx.content_rect();
        egui::Area::new(egui::Id::unique("character-selection")).fixed_pos(screen.min).order(egui::Order::Background).show(ctx, |ui| {
            ui.set_min_size(screen.size());
            ui.set_max_size(screen.size());
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        for (id, signal) in std::mem::take(&mut tree.signals) {
            let name = tree.get(id).name.clone();
            if !matches!(signal, crate::gui::host::Signal::Clicked | crate::gui::host::Signal::Toggled(_)) {
                continue;
            }
            match name.as_str() {
                "name_btn" => {
                    // (the item the button is in: SlotCharClicked)
                    if let Some(c) = self.items.iter().position(|item| tree.is_ancestor(*item, id)).and_then(|k| list.get(k)) {
                        *selected = Some(c.name.clone());
                    }
                }
                "login_btn" => action = selected.clone().filter(|n| usable(n)).map(CharAction::Play),
                "create_btn" => action = Some(CharAction::New),
                "quit_btn" => action = Some(CharAction::Back),
                "delete_btn" => {
                    if let Some(c) = selected.as_ref().and_then(|n| list.iter().find(|c| &c.name == n))
                        && let Some((mut dtree, droot)) = crate::gui::load_view(&dirs(ctx), "CharacterDeleteWindow.xml", "")
                    {
                        dtree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
                        let question = crate::gui::xml::ldb_text("#DeleteCharacter").replace("%s", &c.name);
                        set_text(&mut dtree, droot, "confirmation_text", &question);
                        self.delete = Some(DeleteWindow { window: XmlWindow { name: "Delete Character".into(), title: "Delete Character".into(), tree: dtree, root: droot, open: true, default_size: egui::vec2(340.0, 0.0) }, character: (*c).clone() });
                    }
                }
                _ => {}
            }
        }
        // the confirmation: OK only with the name typed (SlotOkayButtonActivated: else `MatchError`)
        if let Some(d) = self.delete.as_mut() {
            let signals = d.window.show(ctx, palette);
            let root = d.window.root;
            let typed = d.window.tree.find(root, "name_input").and_then(|f| crate::gui::textinput::text(&d.window.tree, f)).unwrap_or("").trim().to_string();
            let mut close = !d.window.open;
            for (id, signal) in signals {
                if signal != crate::gui::host::Signal::Clicked {
                    continue;
                }
                match d.window.tree.get(id).name.as_str() {
                    "ok_btn" if typed.eq_ignore_ascii_case(&d.character.name) => {
                        action = Some(CharAction::Delete(d.character.id));
                        close = true;
                    }
                    "ok_btn" => {
                        let text = crate::gui::xml::ldb_text("#MatchError");
                        set_text(&mut d.window.tree, root, "confirmation_text", &text);
                    }
                    "cancel_btn" => close = true,
                    _ => {}
                }
            }
            if close {
                self.delete = None;
            }
        }
        action
    }
}
