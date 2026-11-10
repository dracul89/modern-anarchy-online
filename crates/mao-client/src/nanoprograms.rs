//! The nano programs window: the programs the character has (the world-entry message's `UploadedNanoIds`), each with its icon and
//! name. A click casts the program at the target (or at oneself without one: the captured casts, `n3::cast_nano`);
//! a program can be dragged onto a hotbar. Pointing at one shows its description and what it changes.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};

use crate::gear::Icon;
use crate::hotbar;
use crate::theme::Palette;

/// The database type of nano programs.
const NANO: u32 = 0xfde85;

pub struct Program {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// The NCU memory it takes (its quality stat).
    pub cost: u32,
    /// What it changes in the stats (`derived::nano_modifiers`).
    pub effects: Vec<(u32, i64)>,
    /// Its school's grid ([`school`]).
    pub school: usize,
    /// What the list mode's columns show: the costs, range and delays (`ncu::Columns`), the nano's whole time in
    /// milliseconds (stat 8), and its stacking line: the nano strain (stat 75) and its place in it (stat 551).
    pub columns: crate::ncu::Columns,
    pub time_ms: u32,
    pub strain: u32,
    pub stacking: u32,
}

/// The list's columns as `NanoView_c` adds them to each school's list (FUN_100d5328): the id, its label's name in
/// text.mdb 10000 with this client's word, the width and the flags.
pub const COLUMNS: &[(i32, &str, &str, f32, u32)] = &[
    (0, "Tab_Icon", "", 16.0, 8),
    (1, "Tab_Name", "Name", 200.0, 0xe),
    (9, "Tab_NanoCost", "Nano", 60.0, 0xe),
    (10, "Tab_NCUcost", "NCU", 60.0, 0xe),
    (13, "Tab_Time", "Time", 55.0, 0xe),
    (11, "Tab_Radius", "Radius", 60.0, 0xe),
    (5, "Tab_AtkRange", "Range", 60.0, 0xe),
    (6, "Tab_AtkDelay", "Attack", 55.0, 0xe),
    (7, "Tab_RcgDelay", "Recharge", 55.0, 0xe),
    (16, "Tab_StackingLine", "Stacking line", 200.0, 0xe),
];

/// The list of hidden programs: the school the original keeps them in (`HideNano` moves a program to list 6, "Show
/// Hidden" shows that list).
pub const HIDDEN: usize = 6;

/// A stacking line's name: the nano strain's in text.mdb 2009 (FUN_1003dee6), nothing for no strain.
fn strain_name(strain: u32) -> String {
    if strain == 0 {
        return String::new();
    }
    let name = crate::gui::xml::ldb_text(&format!("#2009:{strain}"));
    if name == format!("2009:{strain}") { String::new() } else { name }
}

/// A window's "Auto Arrange" sub-menu (FUN_10041637, as the nano window asks for it): "Don't Auto Arrange" (not
/// offered here: programs are not placed by hand), then "Unsorted" for the icon's column and "By <label>" for each
/// other, the grid's sort column marked, then "Sort Ascending", marked when the order is ascending and dead while
/// unsorted. The entries' keys and what the menu shows.
pub fn arrange_menu(labels: &[(i32, String)], sort: (i32, i32)) -> (Vec<String>, Vec<(String, bool, Option<bool>)>) {
    let mut keys = vec!["none".to_string()];
    let mut entries = vec![("Don't Auto Arrange".to_string(), false, Some(false))];
    for (id, label) in labels {
        keys.push(format!("by:{id}"));
        entries.push((if *id == 0 { "Unsorted".to_string() } else { format!("By {label}") }, true, Some(sort.0 == *id)));
    }
    keys.push("ascending".to_string());
    entries.push(("Sort Ascending".to_string(), sort.0 != 0, Some(sort.1 == 0)));
    (keys, entries)
}

/// The programs a school's list shows: its own that are not hidden; the hidden list, every hidden one.
pub fn shown(program: &Program, school: usize, hidden: &BTreeSet<u32>) -> bool {
    let is_hidden = hidden.contains(&program.id) || program.school == HIDDEN;
    if school == HIDDEN { is_hidden } else { program.school == school && !is_hidden }
}

fn hidden_path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("hiddennanos.ini")
}

/// Whether the hidden programs are kept on disk (not in tests and previews).
fn kept() -> bool {
    !cfg!(test) && std::env::var_os("MAO_NO_LAYOUT").is_none() && std::env::var_os("MAO_UI_PREVIEW").is_none()
}

fn load_hidden() -> BTreeSet<u32> {
    if !kept() {
        return BTreeSet::new();
    }
    std::fs::read_to_string(hidden_path()).map(|t| t.lines().filter_map(|l| l.trim().parse().ok()).collect()).unwrap_or_default()
}

fn save_hidden(hidden: &BTreeSet<u32>) {
    if kept() {
        let _ = std::fs::write(hidden_path(), hidden.iter().map(|id| format!("{id}\n")).collect::<String>());
    }
}

/// The programs with their names, icons and effects from the database, by name.
pub fn describe(db: &ResourceDatabase, textures: &crate::Textures, ids: &[u32], level: i64) -> Vec<Program> {
    let mut icons: HashMap<u32, Option<Arc<Icon>>> = HashMap::new();
    let mut list: Vec<Program> = ids
        .iter()
        .map(|&id| {
            let data = db.get(Id { kind: NANO, instance: id }).ok().flatten();
            let template = data.as_deref().and_then(Item::parse);
            let icon = template.as_ref().and_then(|t| t.icon()).and_then(|icon_id| {
                icons
                    .entry(icon_id)
                    .or_insert_with(|| textures.get(item::ICON_TYPE, icon_id).map(|t| Arc::new(Icon { id: icon_id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })))
                    .clone()
            });
            Program {
                id,
                name: template.as_ref().map_or_else(|| format!("Nano #{id}"), |t| t.name.clone()),
                description: template.as_ref().map(|t| t.description.clone()).unwrap_or_default(),
                icon,
                cost: template.as_ref().and_then(|t| t.stat(54)).filter(|&c| c < 999).unwrap_or(0),
                school: school(template.as_ref().and_then(|t| t.stat(405))),
                columns: template.as_ref().map(|t| crate::ncu::Columns::of(|n| t.stat(n), false)).unwrap_or_default(),
                time_ms: template.as_ref().and_then(|t| t.stat(8)).unwrap_or(0),
                strain: template.as_ref().and_then(|t| t.stat(75)).filter(|s| *s != 1_234_567_890).unwrap_or(0),
                stacking: template.as_ref().and_then(|t| t.stat(551)).filter(|s| *s != 1_234_567_890).unwrap_or(0),
                effects: data.as_deref().map(|d| item::nano_modifiers_at(d, level)).unwrap_or_default().into_iter().map(|(s, a)| (s, a as i64)).collect(),
            }
        })
        .collect();
    list.sort_by_key(|p| p.name.to_lowercase());
    list
}

/// A program's school's grid (`NanoView_c` FUN_100d48a9: the template's stat 405 less one, 0 to 6, else 5).
pub fn school(stat: Option<u32>) -> usize {
    match stat.map(|s| s as i64 - 1) {
        Some(s) if (0..=6).contains(&s) => s as usize,
        _ => 5,
    }
}

/// The school buttons in their order (`SchoolSelector_c` FUN_100d6f3d): the button's school and its text.mdb 10000 label
/// (Misc, Combat, Medical, Prot, Psi, Space).
const SCHOOLS: [(usize, &str); 6] = [(5, "#10000:344211"), (0, "#10000:175102436"), (1, "#10000:251981772"), (2, "#10000:23966852"), (3, "#10000:236378921"), (4, "#10000:114743685")];

/// The nano programs window, GUI.dll's `NanoView_c` (FUN_100d5328): the `SchoolSelector_c` (a toggle button per school, in
/// rows as wide as the window: FUN_100d6051) over a tab of item grids, one per school, in the smallest square icon size
/// (`SetGridIconSize(0)`: 31 by 31 on `SLOT_32_CLOSED`), sorted by name; the school shown at first is Misc (5:
/// `selected_school`'s default). A click casts the program at the target (FUN_100d44cd: `N3Msg_CastNanoSpell`; with
/// `UseNewMouseFunc` on, a left click on a list that does not select is the use signal); a program dragged out goes onto
/// a hotbar.
///
/// The right button over a program opens its menu (FUN_100d49a0): "Look at" (its page) and "Hide", or "Unhide" in the
/// hidden list. Elsewhere in the window it opens the window's (FUN_100d4f90): "List Mode" (every school's list under a
/// column header with the columns of [`COLUMNS`], sorted by name until a label is pressed; kept, the original's
/// `listview_mode`), "Show Hidden" (the list of hidden programs, FUN_100d4b49(6); a school's button leads back) and
/// "Auto Arrange" (the grid's order, [`arrange_menu`]; dead in list mode). The school shown is kept
/// (`selected_school`), and the hidden programs in `hiddennanos.ini` beside `mao.ini`. Not ported: programs placed by
/// hand in the grid (`item_position_map`, "Don't Auto Arrange"), the radius column's value, and keeping the columns'
/// widths and the list's sort (`listview_config<n>`).
#[derive(Default)]
pub struct ProgramsWindow {
    pub open: bool,
    textures: HashMap<u32, egui::TextureHandle>,
    /// The info view pages asked for.
    pub infos: Vec<String>,
    school: Option<usize>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id, Vec<(crate::gui::view::Id, usize)>)>,
    /// The school selector and the row views made for the docked layout; whether the docked layout is in force.
    selector: Option<crate::gui::view::Id>,
    rows: Vec<crate::gui::view::Id>,
    docked_layout: Option<bool>,
    /// The menu up (what each entry stands for and where it opened) and its picture.
    menu: Option<(Vec<String>, egui::Pos2)>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// List mode, the grid's sort (column and order) and the hidden programs; not set until the kept ones are read.
    pub list: Option<bool>,
    pub grid_sort: Option<(i32, i32)>,
    pub hidden: Option<BTreeSet<u32>>,
    /// A preview's: where the window starts and its size.
    pub start: Option<(egui::Pos2, egui::Vec2)>,
}

thread_local! {
    /// The selector view `build` made last (handed to `arrange`, which runs after it).
    static SELECTOR: std::cell::Cell<Option<crate::gui::view::Id>> = const { std::cell::Cell::new(None) };
}

impl ProgramsWindow {
    fn build(ctx: &egui::Context) -> (crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id, Vec<(crate::gui::view::Id, usize)>) {
        use crate::gui::view::{Layout, Node, Tree};
        let mut tree = Tree::default();
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
        tree.get_mut(root).layout = Layout::Vertical;
        // (the school buttons, 3 in from the edges)
        let selector = tree.add(Node::new("SchoolSelector", Box::new(crate::gui::view::Plain)));
        tree.get_mut(selector).layout = Layout::Horizontal;
        tree.get_mut(selector).borders = crate::gui::geom::Rect::new(3.0, 3.0, 3.0, 3.0);
        let mut buttons = Vec::new();
        for (school, label) in SCHOOLS {
            let mut b = crate::gui::controls::Button::new(&crate::gui::xml::ldb_text(label));
            b.toggle = true;
            let mut node = Node::new(&format!("school{school}"), Box::new(b));
            node.borders = crate::gui::geom::Rect::new(0.0, 0.0, 3.0, 0.0);
            let id = tree.add(node);
            tree.add_child(selector, id);
            buttons.push((id, school));
        }
        tree.add_child(root, selector);
        SELECTOR.with(|c| c.set(Some(selector)));
        let list = crate::gui::multilist::create_grid(&mut tree, "NanoListView", -1, 6);
        crate::gui::multilist::set_grid(&mut tree, list, |g| g.set_icon_size(0));
        tree.get_mut(list).borders = crate::gui::geom::Rect::new(3.0, 3.0, 3.0, 3.0);
        // (the list mode's columns, sorted by name: `SetListSortColumn(1, 0)`)
        for &(id, label, own, width, flags) in COLUMNS {
            crate::gui::multilist::add_column(&mut tree, list, id, &crate::gui::xml::ldb_named(10000, label, own), width, flags);
        }
        crate::gui::multilist::set_sort(&mut tree, list, 1, 0);
        tree.add_child(root, list);
        (tree, root, list, buttons)
    }

    /// Lays the school buttons out for the docked (two a row, four grid columns) or the free (one row, six columns) window.
    fn arrange(&mut self, docked: bool) {
        use crate::gui::view::{Layout, Node};
        if self.selector.is_none() {
            self.selector = SELECTOR.with(|c| c.get());
        }
        let Some(selector) = self.selector else { return };
        let Some((tree, _, list, buttons)) = self.views.as_mut() else { return };
        for &r in &self.rows {
            tree.get_mut(r).children.clear();
        }
        tree.get_mut(selector).children.clear();
        if docked {
            tree.get_mut(selector).layout = Layout::Vertical;
            for (k, chunk) in buttons.chunks(2).enumerate() {
                let row = match self.rows.get(k) {
                    Some(&r) => r,
                    None => {
                        let mut node = Node::new(&format!("schools{k}"), Box::new(crate::gui::view::Plain));
                        node.layout = Layout::Horizontal;
                        let r = tree.add(node);
                        self.rows.push(r);
                        r
                    }
                };
                tree.add_child(selector, row);
                for &(b, _) in chunk {
                    tree.add_child(row, b);
                }
            }
        } else {
            tree.get_mut(selector).layout = Layout::Horizontal;
            for &(b, _) in buttons.iter() {
                tree.add_child(selector, b);
            }
        }
        crate::gui::multilist::set_grid(tree, *list, |g| g.columns = Some(if docked { 4 } else { 6 }));
        tree.relayout = true;
    }

    /// Shows a school's list (a preview's choice; the hidden list is [`HIDDEN`]).
    pub fn select(&mut self, school: usize) {
        self.school = Some(school.min(HIDDEN));
    }

    /// Shows the window; returns the program to cast (a click).
    pub fn show(&mut self, ctx: &egui::Context, programs: &[Program], palette: &Palette) -> Option<hotbar::Action> {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut cast = None;
        if self.views.is_none() {
            self.views = Some(Self::build(ctx));
        }
        let school = *self.school.get_or_insert_with(|| crate::layout::tab("NanoViewSchool").filter(|s| *s <= HIDDEN).unwrap_or(5));
        // (the kept choices as numbers: list mode; the grid's sort as twice its column and the order)
        let in_list = *self.list.get_or_insert_with(|| crate::layout::tab("NanoViewConfig") == Some(1));
        let grid_sort = *self.grid_sort.get_or_insert_with(|| crate::layout::tab("NanoViewGridSort").map_or((1, 0), |v| ((v / 2) as i32, (v % 2) as i32)));
        let hidden = self.hidden.get_or_insert_with(load_hidden).clone();
        // (docked in the rollup column the window is 175 wide: the original's `SchoolSelector_c` lays its buttons in rows as
        // wide as the window (FUN_100d6051) and the grid takes as many columns as fit; here two buttons a row and four columns)
        let docked = crate::rollup::docked("Nano programs");
        if self.docked_layout != Some(docked) {
            self.docked_layout = Some(docked);
            self.arrange(docked);
        }
        let textures = &mut self.textures;
        let (tree, root, list, buttons) = self.views.as_mut()?;
        let (root, list) = (*root, *list);
        // (the school's button down, the others up; the school's programs in its grid)
        for &(b, s) in buttons.iter() {
            if let Some(button) = tree.control_mut::<crate::gui::controls::Button>(b) {
                button.on = s == school;
            }
        }
        crate::gui::multilist::set_list_mode(tree, list, in_list);
        // (the grid in its own order; the list in the one its header was last pressed for)
        if !in_list && crate::gui::multilist::sort_state(tree, list) != grid_sort {
            crate::gui::multilist::set_sort(tree, list, grid_sort.0, grid_sort.1);
        }
        let rows: Vec<crate::gui::multilist::Item> = programs
            .iter()
            .filter(|p| shown(p, school, &hidden))
            .map(|p| {
                use crate::gui::multilist::CellView;
                let c = p.columns;
                // (the whole time in hundredths of a second, as the column writes it)
                let values: [(i32, i64); 7] = [(9, c.nano_cost.into()), (10, c.ncu.into()), (13, (p.time_ms / 10).into()), (11, 0), (5, c.range.into()), (6, c.attack_delay.into()), (7, c.recharge_delay.into())];
                let mut cells = vec![(1, CellView::Text(p.name.clone())), (16, CellView::Text(strain_name(p.strain)))];
                // (FUN_1003dee6: the stacking line sorts by the strain, then the place in it)
                let mut keys = vec![(1, p.name.to_lowercase()), (16, format!("{:010}{:05}", p.strain, p.stacking & 0xffff))];
                for (column, value) in values {
                    cells.push((column, CellView::Text(crate::ncu::column_text(column, value))));
                    keys.push((column, format!("{value:012}")));
                }
                (p, cells, keys)
            })
            .map(|(p, cells, keys)| crate::gui::multilist::Item {
                id: p.id as i64,
                icon: p.icon.as_ref().map(|icon| textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("program-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id()),
                cells,
                keys,
                ..Default::default()
            })
            .collect();
        // (the same programs, in whatever order the view has sorted them: nothing to do)
        let ids = |items: &[crate::gui::multilist::Item]| {
            let mut ids: Vec<i64> = items.iter().map(|i| i.id).collect();
            ids.sort_unstable();
            ids
        };
        if ids(crate::gui::multilist::items(tree, list)) != ids(&rows) {
            crate::gui::multilist::set_items(tree, list, rows);
        }
        crate::gui::multilist::update(tree, list);
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut hovered = None;
        let mut over = None;
        let title = crate::gui::xml::ldb_text("#10000:346191");
        let (start_at, start_size) = self.start.unwrap_or((egui::pos2(220.0, 90.0), egui::vec2(300.0, 300.0)));
        crate::chrome::window(ctx, palette, "Nano programs", &title, &mut open, |w| w.default_pos(crate::layout::pos("Nano programs", start_at)).default_size(start_size).resizable(true), |ui| {
            let area = ui.max_rect();
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                hovered = crate::gui::multilist::item_at(tree, list, response.rect.min, at).map(|id| id as u32);
                over = area.contains(at).then_some(at);
            }
        });
        let program = |id: i64| programs.iter().find(|p| p.id as i64 == id);
        let action = |p: &Program| hotbar::Action::Nano { id: p.id, name: p.name.clone(), icon: p.icon.as_ref().map(|i| i.id) };
        for (_, event) in crate::gui::multilist::take_events(tree) {
            match event {
                crate::gui::multilist::ItemEvent::Clicked(id, clicks) => {
                    // (a click casts: the list is not selectable, so FUN_1004141b's left click is the use signal -> FUN_100d44cd)
                    let _ = clicks;
                    if let Some(p) = program(id) {
                        cast = Some(action(p));
                    }
                }
                // (dragged out: onto a hotbar)
                crate::gui::multilist::ItemEvent::Dragged(id) => {
                    if let Some(p) = program(id) {
                        egui::DragAndDrop::set_payload(ctx, action(p));
                    }
                }
            }
        }
        // (a school's button pressed shows its grid)
        for (id, signal) in std::mem::take(&mut tree.signals) {
            if matches!(signal, crate::gui::host::Signal::Clicked | crate::gui::host::Signal::Toggled(_))
                && let Some(&(_, s)) = buttons.iter().find(|b| b.0 == id)
            {
                self.school = Some(s);
                crate::layout::set_tab("NanoViewSchool", s);
            }
        }
        // the right button: over a program its menu (FUN_100d49a0), elsewhere the window's (FUN_100d4f90)
        let text = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
        let labels: Vec<(i32, String)> = crate::gui::multilist::columns(tree, list).into_iter().map(|c| (c.id, c.label)).collect();
        if let Some(at) = over
            && ctx.input(|i| i.pointer.secondary_clicked())
        {
            let (keys, entries): (Vec<String>, Vec<(String, bool, Option<bool>)>) = match hovered {
                Some(id) => (
                    vec![format!("look:{id}"), format!("{}:{id}", if school == HIDDEN { "unhide" } else { "hide" })],
                    vec![(text("LookAtNano", "Look at"), true, None), (if school == HIDDEN { text("UnHideNano", "Show again") } else { text("HideNano", "Hide") }, true, None)],
                ),
                None => (
                    vec!["list".to_string(), "hidden".to_string(), "arrange".to_string()],
                    vec![(text("ListMode", "As a list"), true, Some(in_list)), (text("ShowHidden", "Show the hidden ones"), true, None), (text("AutoArrange", "Order"), !in_list, None)],
                ),
            };
            self.popup = Some(crate::chrome::menu_marked(ctx, at, &entries));
            self.menu = Some((keys, at));
        }
        if let Some(chosen) = crate::chrome::menu(ctx, palette, "program-menu", &mut self.popup)
            && let Some((keys, at)) = self.menu.take()
            && let Some(key) = keys.get(chosen)
        {
            let (word, value) = key.split_once(':').unwrap_or((key.as_str(), ""));
            let number = value.parse::<u32>().unwrap_or(0);
            match word {
                "look" => self.infos.push(format!("itemid://{}/{}", crate::gui::iteminfo::NANO_IDENTITY, number)),
                "hide" | "unhide" => {
                    let hidden = self.hidden.get_or_insert_with(BTreeSet::new);
                    if word == "hide" {
                        hidden.insert(number);
                    } else {
                        hidden.remove(&number);
                    }
                    save_hidden(hidden);
                }
                "list" => {
                    self.list = Some(!in_list);
                    crate::layout::set_tab("NanoViewConfig", usize::from(!in_list));
                }
                "hidden" => self.school = Some(HIDDEN),
                // (the sub-menu: here it takes the menu's place)
                "arrange" => {
                    let (keys, entries) = arrange_menu(&labels, grid_sort);
                    self.popup = Some(crate::chrome::menu_marked(ctx, at, &entries));
                    self.menu = Some((keys, at));
                }
                "by" | "ascending" => {
                    let sort = if word == "by" { (number as i32, grid_sort.1) } else { (grid_sort.0, 1 - grid_sort.1) };
                    self.grid_sort = Some(sort);
                    crate::layout::set_tab("NanoViewGridSort", (sort.0 * 2 + sort.1) as usize);
                }
                _ => {}
            }
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        if let Some(id) = hovered.filter(|_| self.popup.is_none()) {
            if let Some(p) = program(id as i64) {
                let at = ctx.input(|i| i.pointer.hover_pos()).unwrap_or_default();
                egui::Area::new(egui::Id::unique("program-tip")).order(egui::Order::Tooltip).fixed_pos(at + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_max_width(320.0);
                        ui.label(egui::RichText::new(&p.name).strong().color(accent));
                        if p.cost > 0 {
                            ui.label(egui::RichText::new(format!("NCU {}", p.cost)).small());
                        }
                        if !p.description.is_empty() {
                            ui.label(p.description.chars().take(420).collect::<String>().replace("\r\n", "\n"));
                        }
                        for &(stat, amount) in p.effects.iter().filter(|e| e.1 != 0).take(10) {
                            ui.horizontal(|ui| {
                                ui.label(crate::skills::stat_name(stat));
                                let color = if amount > 0 { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(230, 110, 110) };
                                ui.label(egui::RichText::new(format!("{amount:+}")).color(color));
                            });
                        }
                    });
                });
            }
        }
        self.open = open;
        cast
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(id: u32, school: usize) -> Program {
        Program { id, name: String::new(), description: String::new(), icon: None, cost: 0, effects: Vec::new(), school, columns: Default::default(), time_ms: 0, strain: 0, stacking: 0 }
    }

    #[test]
    fn a_hidden_program_is_in_the_hidden_list_only() {
        let hidden: BTreeSet<u32> = [7].into_iter().collect();
        assert!(shown(&program(5, 2), 2, &hidden) && !shown(&program(5, 2), 3, &hidden) && !shown(&program(5, 2), HIDDEN, &hidden));
        assert!(!shown(&program(7, 2), 2, &hidden) && shown(&program(7, 2), HIDDEN, &hidden));
        // (a program whose own school is the seventh list is there too)
        assert!(shown(&program(9, HIDDEN), HIDDEN, &hidden));
    }

    #[test]
    fn the_arrange_menu_marks_the_grids_order() {
        let labels = vec![(0, String::new()), (1, "Name".to_string()), (10, "NCU Cost".to_string())];
        let (keys, entries) = arrange_menu(&labels, (10, 1));
        assert_eq!(keys, ["none", "by:0", "by:1", "by:10", "ascending"]);
        assert_eq!(entries.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(), ["Don't Auto Arrange", "Unsorted", "By Name", "By NCU Cost", "Sort Ascending"]);
        // (the sort column marked; descending: "Sort Ascending" unmarked; placing by hand is not offered)
        assert_eq!(entries.iter().map(|e| e.2).collect::<Vec<_>>(), [Some(false), Some(false), Some(false), Some(true), Some(false)]);
        assert!(!entries[0].1 && entries[4].1);
        // (unsorted: nothing to turn round)
        assert!(!arrange_menu(&labels, (0, 0)).1[4].1);
        // (the columns as the original adds them, the name the one the list starts sorted by)
        assert_eq!(COLUMNS.iter().map(|c| c.0).collect::<Vec<_>>(), [0, 1, 9, 10, 13, 11, 5, 6, 7, 16]);
    }

    #[test]
    fn schools_are_the_stat_less_one() {
        assert_eq!(school(Some(1)), 0);
        assert_eq!(school(Some(7)), 6);
        assert_eq!(school(Some(0)), 5);
        assert_eq!(school(None), 5);
    }
}
