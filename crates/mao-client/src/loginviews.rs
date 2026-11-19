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

/// The account's character slots as the login's character list gave them (-1: not known).
static SLOTS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(-1);

/// The login's count of character slots (the list's trailer: `mao_net::session::login_account`).
pub fn set_slots(slots: Option<u32>) {
    SLOTS.store(slots.map_or(-1, i64::from), std::sync::atomic::Ordering::Relaxed);
}

/// The character screen's line under the list, as `CharSelectWindow_c::SlotCharListReceived` (0x1000ebd5) writes it:
/// the text `AvailableCharSlots` (category 10000) with the slots left and the slots in all. A character takes a
/// slot when the number after it in the list has bit 1 (the window counts those).
pub fn slots_line(text: &str, slots: i64, taking: i64) -> String {
    mao_formats::ldb::format(text, &[mao_formats::ldb::Arg::Int(slots - taking), mao_formats::ldb::Arg::Int(slots)])
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
    /// The window's height as last drawn (the error's page is put over it: [`LoginBrowser`]).
    pub height: f32,
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

    /// Shows the window over the screen's middle, `lower` under it; `user` and `password` follow the fields.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, user: &mut String, password: &mut String, accounts: &[String], message: &str, busy: bool, lower: f32) -> Option<LoginAction> {
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
        let shown = crate::chrome::window(ctx, palette, "Login", "Login", &mut open, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, lower)).default_size(min), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            crate::gui::combobox::prepare(tree, response.rect.min);
            if busy {
                ui.label("Logging in...");
            } else if !message.is_empty() {
                // (within the window's own width: a long line, as a refusal's was, made the window as wide as the line)
                ui.allocate_ui_with_layout(egui::vec2(response.rect.width(), 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_max_width(response.rect.width());
                    ui.add(egui::Label::new(egui::RichText::new(message).color(egui::Color32::from_rgb(240, 120, 110))).wrap());
                });
            }
        });
        self.height = shown.map_or(0.0, |r| r.response.rect.height());
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

/// `LoginBrowserWindow_c` (0x10010ead): the window a login's error is shown in, named "Browser" and without a title, its
/// inside a web page (`BrowserView_c`) of 780 by 410 (`_DAT_101a9c78`, `_DAT_101a9c7c`) whose links go to the system's
/// browser (`SetExternalLinksEnabled(true)`). `LoginModule_c::ShowError` (0x10011fd5, 0x10011b6b) sends it to the
/// error's page and shows the login again, and `Show(0)` (0x1001175a) puts the two in the screen's middle one over the
/// other: both centred, the login moved down by half the browser's height and 5, the browser up by half the login's and
/// 5. Closed (`SlotBrowserQuitPressed` 0x10011c33), the address is emptied and the login stands alone again.
///
/// The page itself is the launcher's (a WebView2 child put over [`LoginBrowser::area`]); where there is none, the
/// error is said in words.
#[derive(Default)]
pub struct LoginBrowser {
    /// Where the page goes, in screen points, as the window was last drawn (none while it is not shown).
    pub area: Option<egui::Rect>,
    /// The window's height as last drawn (the login window is put under it).
    pub height: f32,
}

impl LoginBrowser {
    /// The page's size.
    pub const PAGE: egui::Vec2 = egui::vec2(780.0, 410.0);
    /// How far each of the two windows is moved from the other's edge.
    pub const APART: f32 = 5.0;

    /// Shows the window `raise` above the screen's middle, `words` in the page's place when they are given; false once
    /// it was closed.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, raise: f32, words: &str) -> bool {
        let mut open = true;
        let mut area = None;
        // (on a screen too low for the two, the page gives way)
        let screen = ctx.content_rect();
        let size = egui::vec2(Self::PAGE.x.min(screen.width() - 40.0), Self::PAGE.y.min(screen.height() - 2.0 * raise - 60.0)).max(egui::vec2(200.0, 120.0));
        let shown = crate::chrome::window(ctx, palette, "LoginBrowser", "", &mut open, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -raise)), |ui| {
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            area = Some(rect);
            if !words.is_empty() {
                ui.put(rect.shrink(12.0), egui::Label::new(words).wrap());
            }
        });
        self.height = shown.map_or(0.0, |r| r.response.rect.height());
        self.area = area.filter(|_| open);
        open
    }
}

/// The game's network time-outs in seconds (`MainPrefs.xml`'s `GeneralNetworkTimeout`, 90, while logging in;
/// `DimensionNetworkTimeout`, 30, while a character is entered).
pub fn network_timeout(name: &str, default: f32) -> f32 {
    let text = crate::theme::game_dir().and_then(|g| mao_formats::files::read_to_string(g.join("cd_image/gui/Default/MainPrefs.xml")).ok()).unwrap_or_default();
    let quoted = format!("name=\"{name}\"");
    text.lines()
        .find(|l| l.contains(&quoted))
        .and_then(|l| l.split("value=\"").nth(1))
        .and_then(|v| v.split('"').next())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

/// `ProgressWindow_c` (0x1001672b) on `Views/ProgressDialog.xml`, as `LoginModule_c::Show` opens it while logging in (state
/// 1): "Please wait...", Cancel, and the bar filling over the time-out (`SlotFrameProcess` 0x10016620: the time passed
/// over the time-out, at most 1). Cancel (`SlotCancelPressed`) and the time running out both end it, and the login goes
/// back to its form (`SlotLoginTimeout`: `Show(0)`).
#[derive(Default)]
pub struct ProgressWindow {
    window: Option<XmlWindow>,
    timeout: f32,
    started: Option<std::time::Instant>,
}

impl ProgressWindow {
    pub fn start(&mut self, timeout: f32) {
        self.window = None;
        self.timeout = timeout.max(1.0);
        self.started = Some(std::time::Instant::now());
    }

    pub fn stop(&mut self) {
        self.started = None;
        self.window = None;
    }

    /// Shows the window; true when it was cancelled or the time ran out.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> bool {
        let Some(started) = self.started else { return false };
        if self.window.is_none() {
            let Some((mut tree, root)) = crate::gui::load_view(&dirs(ctx), "ProgressDialog.xml", "") else { return false };
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            self.window = Some(XmlWindow { name: "Progress".into(), title: "Progress".into(), tree, root, open: true, default_size: egui::vec2(220.0, 0.0) });
        }
        let Some(w) = self.window.as_mut() else { return false };
        let root = w.root;
        let done = started.elapsed().as_secs_f32() / self.timeout;
        if let Some(id) = w.tree.find(root, "progress_bar") {
            crate::gui::powerbar::set_value(&mut w.tree, id, done.min(1.0));
        }
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
        let tree = &mut w.tree;
        let mut open = true;
        crate::chrome::window(ctx, palette, "Progress", "Progress", &mut open, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).default_size(min.max(egui::vec2(220.0, 0.0))), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        ctx.request_repaint();
        let cancelled = std::mem::take(&mut w.tree.signals).into_iter().any(|(id, s)| s == crate::gui::host::Signal::Clicked && w.tree.get(id).name == "cancel_btn");
        if cancelled || !open || done >= 1.0 {
            self.started = None;
            self.window = None;
            return true;
        }
        false
    }
}

/// What Play does with the selected character (`CharSelectWindow_c::SlotLoginPressed` 0x1000dcf0): an active one (its
/// list flag's bit 1) logs in; an inactive one asks first (`CharActivateWindow_c`), or, with no slot free
/// (`slots <= active`), only says so (`CharacterSlotsExhausted`, category 10000, with OK) and sends nothing. Slots not
/// known (-1, the tests' pictures) ask as with a slot free.
#[derive(Debug, PartialEq, Eq)]
pub enum PlayAsk {
    Play,
    Activate,
    Exhausted,
}

pub fn play_ask(flag: i32, slots: i64, taking: i64) -> PlayAsk {
    if flag & 1 != 0 {
        PlayAsk::Play
    } else if slots >= 0 && slots <= taking {
        PlayAsk::Exhausted
    } else {
        PlayAsk::Activate
    }
}

/// The confirmation (`CharDeleteWindow_c`), and the activation's (`CharActivateWindow_c`).
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
    /// `Views/CharacterActivateWindow.xml` for an inactive character Play was pressed on.
    activate: Option<DeleteWindow>,
    /// The `CharacterSlotsExhausted` box is up.
    exhausted: bool,
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
        let Some(text) = dirs(ctx).iter().find_map(|d| mao_formats::files::read(d.join("Views").join("CharacterSelectionItem.xml")).ok()) else { return };
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
        // (the slots left of the account's, and New Character only while one is left; an account whose slots are not
        // known, as in the pictures the tests take, has the count of its characters there)
        let slots = SLOTS.load(std::sync::atomic::Ordering::Relaxed);
        let taking = list.iter().filter(|c| c.flag & 1 != 0).count() as i64;
        if slots >= 0 {
            set_text(tree, root, "slots_available", &slots_line(&crate::gui::xml::ldb_named(10000, "AvailableCharSlots", "%d of %d character slots available"), slots, taking));
            if let Some(b) = tree.find(root, "create_btn") {
                tree.get_mut(b).enabled = taking < slots;
            }
        } else {
            set_text(tree, root, "slots_available", &format!("{} characters", list.len()));
        }
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
                "login_btn" => {
                    if let Some(c) = selected.as_ref().filter(|n| usable(n)).and_then(|n| list.iter().find(|c| &c.name == n)) {
                        match play_ask(c.flag, slots, taking) {
                            PlayAsk::Play => action = Some(CharAction::Play(c.name.clone())),
                            PlayAsk::Exhausted => self.exhausted = true,
                            PlayAsk::Activate => {
                                // (CharActivateWindow_c 0x1000d688: text 700 `ActivateCharacterWarning` fed the name, the
                                // slots left and the slots in all)
                                if let Some((mut atree, aroot)) = crate::gui::load_view(&dirs(ctx), "CharacterActivateWindow.xml", "") {
                                    atree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
                                    let text = crate::gui::xml::ldb_named(700, "ActivateCharacterWarning", "Activate %s? %d of %d character slots are free.");
                                    let shown = mao_formats::ldb::format(&text, &[mao_formats::ldb::Arg::Text(&c.name), mao_formats::ldb::Arg::Int(slots.max(0) - taking), mao_formats::ldb::Arg::Int(slots.max(0))]);
                                    set_text(&mut atree, aroot, "confirmation_text", &shown);
                                    self.activate = Some(DeleteWindow { window: XmlWindow { name: "Activate Character".into(), title: "Activate Character".into(), tree: atree, root: aroot, open: true, default_size: egui::vec2(340.0, 0.0) }, character: (*c).clone() });
                                } else {
                                    action = Some(CharAction::Play(c.name.clone()));
                                }
                            }
                        }
                    }
                }
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
            let typed = d.window.tree.find(root, "name_input").and_then(|f| crate::gui::textinput::text(&d.window.tree, f)).unwrap_or("").to_string();
            let mut close = !d.window.open;
            for (id, signal) in signals {
                if signal != crate::gui::host::Signal::Clicked {
                    continue;
                }
                match d.window.tree.get(id).name.as_str() {
                    "ok_btn" if delete_name_matches(&typed, &d.character.name) => {
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
        // the activation: Yes logs in (SlotLoginConfirmPressed 0x1000da5c), Cancel closes
        if let Some(a) = self.activate.as_mut() {
            let signals = a.window.show(ctx, palette);
            let mut close = !a.window.open;
            for (id, signal) in signals {
                if signal != crate::gui::host::Signal::Clicked {
                    continue;
                }
                match a.window.tree.get(id).name.as_str() {
                    "ok_btn" => {
                        action = Some(CharAction::Play(a.character.name.clone()));
                        close = true;
                    }
                    "cancel_btn" => close = true,
                    _ => {}
                }
            }
            if close {
                self.activate = None;
            }
        }
        if self.exhausted {
            let text = crate::gui::xml::ldb_named(10000, "CharacterSlotsExhausted", "No character slot is free.");
            let ok = crate::gui::xml::ldb_named(10000, "MsgBox_OK", "OK");
            if crate::chrome::dialog(ctx, palette, "slots-exhausted", "", &text, &[&ok]).is_some() {
                self.exhausted = false;
            }
        }
        action
    }
}

/// Whether the name typed in the deletion's confirmation lets OK go on (`CharDeleteWindow_c::SlotOkayButtonActivated`
/// 0x1000db27: `std::string::compare` of the field's text with the character's name, so exactly the name, in its case,
/// nothing around it).
pub fn delete_name_matches(typed: &str, name: &str) -> bool {
    typed == name
}

/// The limbo window's seconds (`LoginModule_c::SlotLoginLimbo` 0x10011c56: the message's time-out, at least 10).
pub fn limbo_seconds(timeout: i32) -> f32 {
    timeout.max(10) as f32
}

/// The countdown bar after `elapsed` of `timeout` seconds, and whether the time is up (`LimboWindow_c::SlotFrameProcess`
/// 0x1001035b: the bar at one less the time gone over the time-out, held at 0; up once the time gone reaches it).
pub fn limbo_bar(elapsed: f32, timeout: f32) -> (f32, bool) {
    let gone = (elapsed / timeout).min(1.0);
    (1.0 - gone, elapsed >= timeout)
}

/// The profession line of the limbo window (`LimboWindow_c::SetCharacterInfo` 0x10010979): the profession's name
/// (text.mdb category 2004), or `NotChosenYet` (category 506) for one of 255 or below 1.
pub fn limbo_profession(profession: i32) -> String {
    let texts = crate::feedback::texts();
    if profession == 0xff || profession < 1 {
        return texts.and_then(|t| t.named(506, "NotChosenYet")).unwrap_or("Not chosen yet").to_string();
    }
    texts.and_then(|t| t.get(2004, profession)).map(str::to_string).unwrap_or_else(|| crate::launcher::profession(profession).to_string())
}

/// What the limbo window asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum LimboAction {
    /// Login: the character is entered (`LimboWindow_c::SlotLoginPressed` 0x100102ca → `LoginModule_c::SlotSelectCharacter`).
    Login(Character),
    /// The time ran out or the window was closed (`SlotFrameProcess`, `SlotQuitPressed` 0x1001025d →
    /// `LoginModule_c::SlotLimboTimeout` 0x10011c4e: `Show(0)`, the login form again).
    Leave,
}

/// `LimboWindow_c` on `Views/LimboWindow.xml` (constructor 0x10010502), shown by `LoginModule_c::Show(2)` in the place of
/// the character list when the login is answered with system message 0x4e: the two texts `LimboMessage1` and
/// `LimboMessage2`, the character's name, level, gender, breed and profession (`SetCharacterInfo` 0x10010979; the
/// location's lines are commented out in the view), a countdown bar going from full to empty over the time-out
/// (`SetTimeout` 0x100101f0, `SlotFrameProcess`) and Login. Centred (`MoveToCenter`).
#[derive(Default)]
pub struct LimboWindow {
    window: Option<XmlWindow>,
    character: Option<Character>,
    timeout: f32,
    started: Option<std::time::Instant>,
}

impl LimboWindow {
    /// `SlotLoginLimbo`: the character and the time-out (at least 10 seconds), the countdown from now.
    pub fn start(&mut self, timeout: i32, character: Character) {
        self.window = None;
        self.character = Some(character);
        self.timeout = limbo_seconds(timeout);
        self.started = Some(std::time::Instant::now());
    }

    pub fn stop(&mut self) {
        self.started = None;
        self.window = None;
        self.character = None;
    }

    /// Shows the window; what was asked for, if anything.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<LimboAction> {
        let started = self.started?;
        let character = self.character.clone()?;
        let (bar, up) = limbo_bar(started.elapsed().as_secs_f32(), self.timeout);
        // (without the view the window is empty in the original, and still times out: `SlotFrameProcess` counts with or
        // without its bar)
        let view = if self.window.is_none() { crate::gui::load_view(&dirs(ctx), "LimboWindow.xml", "") } else { None };
        if let Some((mut tree, root)) = view {
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            set_text(&mut tree, root, "name", &character.name);
            set_text(&mut tree, root, "level", &character.level.to_string());
            let gender = match crate::launcher::sex(character.sex) {
                "" if character.sex == 1 => "neuter",
                "" => "none",
                sex => sex,
            };
            set_text(&mut tree, root, "gender", gender);
            let breed = crate::launcher::breed(character.breed);
            set_text(&mut tree, root, "breed", if breed.is_empty() { "unknown" } else { breed });
            set_text(&mut tree, root, "profession", &limbo_profession(character.profession));
            self.window = Some(XmlWindow { name: "Limbo".into(), title: String::new(), tree, root, open: true, default_size: egui::vec2(240.0, 0.0) });
        }
        ctx.request_repaint();
        let Some(w) = self.window.as_mut() else {
            if up {
                self.stop();
                return Some(LimboAction::Leave);
            }
            return None;
        };
        let root = w.root;
        if let Some(id) = w.tree.find(root, "countdown_bar") {
            crate::gui::powerbar::set_value(&mut w.tree, id, bar);
        }
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
        let tree = &mut w.tree;
        let mut open = true;
        crate::chrome::window(ctx, palette, "Limbo", "", &mut open, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).default_size(min.max(egui::vec2(240.0, 0.0))), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        let login = std::mem::take(&mut w.tree.signals).into_iter().any(|(id, s)| s == crate::gui::host::Signal::Clicked && w.tree.get(id).name == "login_btn");
        if login {
            self.stop();
            return Some(LimboAction::Login(character));
        }
        if up || !open {
            self.stop();
            return Some(LimboAction::Leave);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The limbo window: at least 10 seconds (`SlotLoginLimbo`), the bar from full to empty and the time up when the
    /// time gone reaches the time-out (`SlotFrameProcess`), the profession's line (`SetCharacterInfo`).
    #[test]
    fn the_limbo_window_counts_down() {
        assert_eq!(limbo_seconds(3), 10.0);
        assert_eq!(limbo_seconds(-1), 10.0);
        assert_eq!(limbo_seconds(45), 45.0);
        assert_eq!(limbo_bar(0.0, 10.0), (1.0, false));
        assert_eq!(limbo_bar(2.5, 10.0), (0.75, false));
        assert_eq!(limbo_bar(10.0, 10.0), (0.0, true));
        assert_eq!(limbo_bar(12.0, 10.0), (0.0, true));
        // (a profession of 255 or below 1 is not chosen yet; others by name)
        assert_eq!(limbo_profession(0xff), limbo_profession(0));
        assert_ne!(limbo_profession(1), limbo_profession(0));
        assert!(!limbo_profession(11).is_empty());
    }

    /// The deletion asks for the name exactly as it is.
    #[test]
    fn deleting_asks_for_the_name_exactly() {
        assert!(delete_name_matches("Someone", "Someone"));
        assert!(!delete_name_matches("someone", "Someone"));
        assert!(!delete_name_matches("Someone ", "Someone"));
        assert!(!delete_name_matches("", "Someone"));
    }

    /// Play on an active character logs in; on an inactive one it asks, or with every slot taken only says so.
    #[test]
    fn play_asks_for_an_inactive_character() {
        assert_eq!(play_ask(1, 8, 8), PlayAsk::Play);
        assert_eq!(play_ask(3, -1, 0), PlayAsk::Play);
        assert_eq!(play_ask(0, 8, 7), PlayAsk::Activate);
        assert_eq!(play_ask(0, 8, 8), PlayAsk::Exhausted);
        assert_eq!(play_ask(2, 3, 5), PlayAsk::Exhausted);
        assert_eq!(play_ask(0, -1, 4), PlayAsk::Activate);
    }

    /// The slots line: the slots left, then the slots in all, in the game's own text (`MAO_GAME`).
    #[test]
    fn the_slots_line() {
        assert_eq!(slots_line("%d/%d slots", 8, 7), "1/8 slots");
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let texts = mao_formats::ldb::Texts::parse(&std::fs::read(std::path::Path::new(&game).join("cd_image/text/text.mdb")).unwrap()).unwrap();
        let line = slots_line(texts.named(10000, "AvailableCharSlots").expect("the text"), 18, 17);
        assert!(line.contains('1') && line.contains("18"), "{line}");
    }
}
