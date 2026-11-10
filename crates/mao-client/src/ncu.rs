//! The NCU window: the nano programs running on the character (what the server lists in the character's own `SimpleCharFullUpdate`:
//! the nano's id and the time it has left), each with its icon, name and a bar for the time left, and the NCU memory in use against
//! the maximum. Pointing at a nano shows what it does (its description and the stat changes it makes).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};

use crate::gear::Icon;
use crate::theme::Palette;

/// The database type of nano programs; the item `stat` of a nano's duration in milliseconds.
const NANO: u32 = 0xfde85;
const DURATION: u32 = 8;

/// One running nano, ready to show.
pub struct NanoInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// The time left when the server told us, the nano's whole duration (both in milliseconds) and when we were told.
    pub remaining_ms: u32,
    pub total_ms: u32,
    pub at: Instant,
    /// What the nano changes in the stats (`derived::nano_modifiers`), for the tooltip.
    pub effects: Vec<(u32, i64)>,
    /// The NCU memory the nano takes: the nano's own quality stat (the sum over the running nanos was the original's current NCU for two
    /// characters, exactly); the free nano programs carry 999 there and take none.
    pub cost: u32,
    /// What the list mode's other columns show (`InventoryListViewItem_c` FUN_1003dee6).
    pub columns: Columns,
}

/// A nano's values for the list's columns, from its record: the type (FUN_1003f4cf: 2 hostile when its flags have
/// 0x8000, 4 a team nano, else 1 friendly; 3, self only, is a flag of Gamecode's nano object that was not traced to
/// the record, so no nano is told to be it), the nano points to run it (stat 407), the raw NCU stat (54; 999 none),
/// the attack range (287), the attack delay and the recharge delay in hundredths of a second (294, 210), and whether
/// the list leaves it out (FUN_100458b0: flags 0x1000000).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Columns {
    pub kind: u8,
    pub nano_cost: u32,
    pub ncu: u32,
    pub range: u32,
    pub attack_delay: u32,
    pub recharge_delay: u32,
    pub hidden: bool,
}

/// The spell function a team nano carries (`N3Msg_IsTeamNano`: FUN_10002483(0, 0xcf4a)).
const TEAM_CAST: u32 = 0xcf4a;

impl Columns {
    /// From a nano's stats (by number) and whether one of its spells is the team cast.
    pub fn of(stat: impl Fn(u32) -> Option<u32>, team: bool) -> Columns {
        let flags = stat(0).unwrap_or(0);
        // (a stat the record does not have reads as 1234567890 in the original, shown as 0)
        let value = |n: u32| stat(n).filter(|v| *v != 1_234_567_890).unwrap_or(0);
        Columns {
            kind: if flags & 0x8000 != 0 { 2 } else if team { 4 } else { 1 },
            nano_cost: value(407),
            ncu: stat(54).unwrap_or(0),
            range: value(287),
            attack_delay: value(294),
            recharge_delay: value(210),
            hidden: flags & 0x0100_0000 != 0,
        }
    }
}

/// A column's text for a value (FUN_1003d7d0): `%dm` or `---` for the ranges (5, 11), seconds with two places for
/// the delays (6, 7), `---` for an NCU cost of 999 (10), hours, minutes and seconds for the times in hundredths of a
/// second (12, 13), the number otherwise.
pub fn column_text(column: i32, value: i64) -> String {
    match column {
        5 | 11 if value == 0 => "---".to_string(),
        5 | 11 => format!("{value}m"),
        6 | 7 => format!("{:.2}s", value as f64 / 100.0),
        10 if value == 999 => "---".to_string(),
        12 | 13 => {
            let seconds = value / 100;
            format!("{:02}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
        }
        _ => value.to_string(),
    }
}

/// The type column's word and colour (FUN_1003d5a5).
pub fn kind_text(kind: u8) -> (&'static str, u32) {
    match kind {
        2 => ("Hostile", 0xff0000),
        1 => ("Friendly", 0xffffff),
        4 => ("Team", 0x00ff00),
        3 => ("Self", 0xffff00),
        _ => ("Unknown", 0xffffff),
    }
}

/// The list's columns as `NCUView_c` adds them (FUN_100d774b): the id, its label's name in text.mdb 10000 with this
/// client's word for it, the width, and the flags (the icon's only sortable; the others also resizable and movable).
pub const COLUMNS: &[(i32, &str, &str, f32, u32)] = &[
    (0, "Tab_Icon", "", 16.0, 8),
    (1, "Tab_Name", "Name", 200.0, 0xe),
    (8, "Tab_Type", "Type", 50.0, 0xe),
    (9, "Tab_NanoCost", "Nano", 60.0, 0xe),
    (10, "Tab_NCUcost", "NCU", 60.0, 0xe),
    (13, "Tab_Time", "Time", 55.0, 0xe),
    (12, "Tab_Remaning", "Left", 55.0, 0xe),
    (11, "Tab_Radius", "Radius", 60.0, 0xe),
    (5, "Tab_AtkRange", "Range", 60.0, 0xe),
    (6, "Tab_AtkDelay", "Attack", 55.0, 0xe),
    (7, "Tab_RcgDelay", "Recharge", 55.0, 0xe),
];

impl NanoInfo {
    /// The time left now, in milliseconds.
    pub fn left(&self, now: Instant) -> u32 {
        self.remaining_ms.saturating_sub(now.saturating_duration_since(self.at).as_millis().min(u32::MAX as u128) as u32)
    }
}

/// The running nanos `(id, time left in milliseconds)` with their names, icons and effects from the database.
pub fn describe(db: &ResourceDatabase, textures: &crate::Textures, running: &[(u32, u32)], level: i64, at: Instant) -> Vec<NanoInfo> {
    let mut icons: HashMap<u32, Option<Arc<Icon>>> = HashMap::new();
    running
        .iter()
        .map(|&(id, remaining_ms)| {
            let data = db.get(Id { kind: NANO, instance: id }).ok().flatten();
            let template = data.as_deref().and_then(Item::parse);
            let icon = template.as_ref().and_then(|t| t.icon()).and_then(|icon_id| {
                icons
                    .entry(icon_id)
                    .or_insert_with(|| textures.get(item::ICON_TYPE, icon_id).map(|t| Arc::new(Icon { id: icon_id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })))
                    .clone()
            });
            let effects = data.as_deref().map(|d| item::nano_modifiers_at(d, level)).unwrap_or_default().into_iter().map(|(s, a)| (s, a as i64)).collect();
            let team = data.as_deref().is_some_and(|d| mao_formats::spell::lists(d).iter().any(|(_, spells)| spells.iter().any(|s| s.kind == TEAM_CAST)));
            let columns = template.as_ref().map(|t| Columns::of(|n| t.stat(n), team)).unwrap_or_default();
            NanoInfo {
                id,
                name: template.as_ref().map_or_else(|| format!("Nano #{id}"), |t| t.name.clone()),
                description: template.as_ref().map(|t| t.description.clone()).unwrap_or_default(),
                icon,
                remaining_ms,
                total_ms: template.as_ref().and_then(|t| t.stat(DURATION)).unwrap_or(0),
                at,
                effects,
                cost: template.as_ref().and_then(|t| t.stat(54)).filter(|&c| c < 999).unwrap_or(0),
                columns,
            }
        })
        .collect()
}

/// `mm:ss` (or `h:mm:ss`) of a time in milliseconds.
pub fn clock(ms: u32) -> String {
    let seconds = ms / 1000;
    if seconds >= 3600 {
        format!("{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

/// The NCU window, GUI.dll's `NCUView_c` (FUN_100d774b): an `NCUListView_c` (FUN_10045d42) grid of the running nanos
/// (`N3Msg_GetNanoTemplateInfoList`) in the small icon size (`SetGridIconSize(2)`: 39 by 23 on `SLOT_40X24`), packed in the
/// order of their time left (`FUN_10041258(0xc)`: the grid sorted by column 12, auto arranged), no horizontal scroll bar,
/// 160 by 160 to start; each icon fades from the right as its time runs out (`ItemIconView_c` FUN_1003f9ea). The title is
/// text.mdb 10000's "NCU (%u/%u)" with NCU in use and the most (stats 180 and 181: FUN_100d7689). A click shows the nano's
/// page (FUN_10045259: `itemid://53019/<id>`).
///
/// The right button opens the window's menu (FUN_100d758a): "List Mode" turns the grid into the list (`SetLayoutMode`,
/// kept as `listview_mode`): the eleven columns of [`COLUMNS`] under their header, a nano a row with its icon, sorted by
/// the time left until a label is pressed, a hostile nano's row in red. Over a nano the menu also has this client's own
/// entry that cancels it. Nanos whose flags have 0x1000000 are in neither (FUN_100458b0). Not ported: the menu's "Auto
/// Arrange" sub-menu, the radius (`N3Msg_GetFormularRadius`, a field of Gamecode's nano object not traced to the
/// record: `---`), and keeping the columns' widths and the sort (`listview_config`).
pub struct NcuWindow {
    pub open: bool,
    textures: HashMap<u32, egui::TextureHandle>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id)>,
    /// The info page asked for (for the info view).
    pub info_url: Option<String>,
    /// The program whose menu is up (none: the menu was opened beside the nanos), and the menu (`PopupMenu_c`'s look).
    menu: Option<(Option<u32>, egui::Pos2)>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// List mode (`listview_mode`); not set until the kept choice is read.
    pub list: Option<bool>,
    /// A preview's: the two target windows to show with the same nanos.
    pub target_preview: Option<[TargetNcu; 2]>,
    /// Where the window starts and its size (the original's 160 by 160; a preview's are its own).
    pub start: (egui::Pos2, egui::Vec2),
}

impl Default for NcuWindow {
    fn default() -> NcuWindow {
        NcuWindow { open: false, textures: HashMap::new(), views: None, info_url: None, menu: None, popup: None, list: None, target_preview: None, start: (egui::pos2(560.0, 90.0), egui::vec2(160.0, 160.0)) }
    }
}

impl NcuWindow {
    fn build(ctx: &egui::Context) -> (crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id) {
        use crate::gui::view::{Layout, Node, Tree};
        let mut tree = Tree::default();
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
        tree.get_mut(root).layout = Layout::Base;
        let list = crate::gui::multilist::create_grid(&mut tree, "NCUListView", -1, 3);
        crate::gui::multilist::set_grid(&mut tree, list, |g| g.set_icon_size(2));
        // (the list mode's columns; both modes sorted by the time left: `SetListSortColumn(0xc, 0)`, FUN_10041258)
        for &(id, label, own, width, flags) in COLUMNS {
            crate::gui::multilist::add_column(&mut tree, list, id, &crate::gui::xml::ldb_named(10000, label, own), width, flags);
        }
        crate::gui::multilist::set_sort(&mut tree, list, 12, 0);
        tree.add_child(root, list);
        (tree, root, list)
    }

    /// Shows the window; returns the program the player chose to cancel (the client's right-click menu).
    pub fn show(&mut self, ctx: &egui::Context, nanos: &[NanoInfo], stats: &std::collections::BTreeMap<u32, i64>, palette: &Palette) -> Option<u32> {
        let mut cancel = None;
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let now = Instant::now();
        if self.views.is_none() {
            self.views = Some(Self::build(ctx));
        }
        let textures = &mut self.textures;
        let in_list = *self.list.get_or_insert_with(|| crate::layout::tab("NCUWindowConfig") == Some(1));
        let (tree, root, list) = self.views.as_mut()?;
        let (root, list) = (*root, *list);
        crate::gui::multilist::set_list_mode(tree, list, in_list);
        // (the nanos by their time left, each icon faded past its share of time left; drawn again as the time runs)
        let mut order: Vec<&NanoInfo> = nanos.iter().filter(|n| !n.columns.hidden).collect();
        order.sort_by_key(|n| n.left(now));
        let rows = nano_rows(ctx, textures, &order, now);
        if crate::gui::multilist::items(tree, list) != rows.as_slice() {
            crate::gui::multilist::set_items(tree, list, rows);
        }
        crate::gui::multilist::update(tree, list);
        let used = stats.get(&180).copied().unwrap_or_else(|| nanos.iter().map(|n| n.cost as i64).sum());
        let max = stats.get(&181).copied().unwrap_or(0);
        let title = crate::gui::xml::ldb_text("#10000:89261993").replacen("%u", &used.to_string(), 1).replacen("%u", &max.to_string(), 1);
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut hovered = None;
        let mut over = None;
        let (start_at, start_size) = self.start;
        crate::chrome::window(ctx, palette, "NCU", &title, &mut open, |w| w.default_pos(crate::layout::pos("NCU", start_at)).default_size(start_size).resizable(true), |ui| {
            let area = ui.max_rect();
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                hovered = crate::gui::multilist::item_at(tree, list, response.rect.min, at).map(|id| (id as u32, at));
                over = area.contains(at).then_some(at);
            }
        });
        if !nanos.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        for (_, event) in crate::gui::multilist::take_events(tree) {
            if let crate::gui::multilist::ItemEvent::Clicked(id, _) = event {
                self.info_url = Some(format!("itemid://{}/{}", crate::gui::iteminfo::NANO_IDENTITY, id));
            }
        }
        tree.signals.clear();
        // (the window's menu, FUN_100d758a: "List Mode" with its mark; over a nano, this client's own entry too)
        if let Some(at) = over
            && ctx.input(|i| i.pointer.secondary_clicked())
        {
            let nano = hovered.map(|h| h.0);
            let mut entries = vec![(crate::gui::xml::ldb_named(10000, "ListMode", "As a list"), true, Some(in_list))];
            if nano.is_some() {
                entries.push(("Cancel this nano".to_string(), true, None));
            }
            self.menu = Some((nano, at));
            self.popup = Some(crate::chrome::menu_marked(ctx, at, &entries));
        }
        // (a nano's tip: its name, time left, description and what it changes)
        if let Some((id, at)) = hovered.filter(|_| self.menu.is_none())
            && let Some(nano) = nanos.iter().find(|n| n.id == id)
        {
            let left = nano.left(now);
            egui::Area::new(egui::Id::unique("nano-tip")).order(egui::Order::Tooltip).fixed_pos(at + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(320.0);
                    ui.label(egui::RichText::new(&nano.name).strong().color(accent));
                    if nano.total_ms > 0 {
                        ui.label(egui::RichText::new(format!("{} of {} left", clock(left), clock(nano.total_ms))).small());
                    }
                    if !nano.description.is_empty() {
                        ui.label(nano.description.chars().take(420).collect::<String>().replace("\r\n", "\n"));
                    }
                    for &(stat, amount) in nano.effects.iter().filter(|e| e.1 != 0).take(10) {
                        ui.horizontal(|ui| {
                            ui.label(crate::skills::stat_name(stat));
                            let color = if amount > 0 { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(230, 110, 110) };
                            ui.label(egui::RichText::new(format!("{amount:+}")).color(color));
                        });
                    }
                });
            });
        }
        match (crate::chrome::menu(ctx, palette, "nano-menu", &mut self.popup), self.menu) {
            (Some(0), _) => {
                self.list = Some(!in_list);
                crate::layout::set_tab("NCUWindowConfig", usize::from(!in_list));
            }
            (Some(1), Some((Some(id), _))) if nanos.iter().any(|n| n.id == id) => cancel = Some(id),
            _ => {}
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        self.open = open;
        cancel
    }
}

/// The nanos as the list view's items: the icon (fading from the right as the time runs out), and the cells and sort
/// keys of every column of [`COLUMNS`].
fn nano_rows(ctx: &egui::Context, textures: &mut HashMap<u32, egui::TextureHandle>, order: &[&NanoInfo], now: Instant) -> Vec<crate::gui::multilist::Item> {
    let rows: Vec<crate::gui::multilist::Item> = order
        .iter()
        .map(|n| {
            use crate::gui::multilist::CellView;
            let c = n.columns;
            // (the times in hundredths of a second, as `N3Msg_GetBuffCurrentTime` and `GetBuffTotalTime` give them)
            let values: [(i32, i64); 8] = [(9, c.nano_cost.into()), (10, c.ncu.into()), (13, (n.total_ms / 10).into()), (12, (n.left(now) / 10).into()), (11, 0), (5, c.range.into()), (6, c.attack_delay.into()), (7, c.recharge_delay.into())];
            let (kind, tint) = kind_text(c.kind);
            let mut cells = vec![(1, CellView::Text(n.name.clone())), (8, CellView::Tinted(kind.to_string(), tint))];
            let mut keys = vec![(1, n.name.clone()), (8, kind.to_string())];
            for (column, value) in values {
                cells.push((column, CellView::Text(column_text(column, value))));
                keys.push((column, format!("{value:012}")));
            }
            (n, cells, keys)
        })
        .map(|(n, cells, keys)| crate::gui::multilist::Item {
            id: n.id as i64,
            icon: n.icon.as_ref().map(|icon| textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("nano-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id()),
            left: (n.total_ms > 0).then(|| n.left(now) as f32 / n.total_ms as f32),
            cells,
            keys,
            // (FUN_1004537b: a hostile nano's row red)
            colour: (n.columns.kind == 2).then_some(0xff0000),
            ..Default::default()
        })
        .collect();
    rows
}

/// A target's NCU window, GUI.dll's `TargetNCUView_c` (FUN_100dda60): the nanos running on the selected target ("Nano
/// Target NCU") or on the one fought ("Fight Target NCU"), in the same list view as the NCU window, 167 by 112 to start,
/// in list mode with three columns (icon, name, time left) sorted by the time left. Its menu (FUN_100dd94d) has "List
/// Mode" and "Hostile Only", which is on to start with (the view's flags are 1): only nanos whose flags have 0x8000 and
/// whose NCU cost is not 999 are listed (FUN_100458b0). With no target the list is hidden (FUN_100dd68f). The original
/// shows the window while `cc_section1 && Show<Nano|Fight>TargetNCU` holds, and closing it turns the option off
/// (FUN_100dd390).
///
/// Not ported: the borderless style (Alt and a double click, FUN_100dd7ea) and the window's own tab strip.
pub struct TargetNcu {
    pub fight: bool,
    textures: HashMap<u32, egui::TextureHandle>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id)>,
    pub info_url: Option<String>,
    menu: Option<egui::Pos2>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// List mode and "Hostile Only"; not set until the kept choices are read.
    pub list: Option<bool>,
    pub hostile_only: Option<bool>,
}

impl TargetNcu {
    pub fn new(fight: bool) -> TargetNcu {
        TargetNcu { fight, textures: HashMap::new(), views: None, info_url: None, menu: None, popup: None, list: None, hostile_only: None }
    }

    /// The window's name here and its title, and the name its choices are kept under (the original's config).
    fn names(&self) -> (&'static str, &'static str, &'static str) {
        if self.fight { ("FightTargetNCU", "Fight Target NCU", "FightTargetNCUWindowConfig") } else { ("NanoTargetNCU", "Nano Target NCU", "NanoTargetNCUWindowConfig") }
    }

    /// What the window lists of a target's nanos (FUN_100458b0).
    pub fn listed(nano: &NanoInfo, hostile_only: bool) -> bool {
        !nano.columns.hidden && (!hostile_only || (nano.columns.kind == 2 && nano.columns.ncu != 999))
    }

    /// Shows the window with the target's nanos (`None`: no target); false when the player closed it.
    pub fn show(&mut self, ctx: &egui::Context, nanos: Option<&[NanoInfo]>, palette: &Palette) -> bool {
        let (name, title, config) = self.names();
        let now = Instant::now();
        // (the choices kept as bits: 1 list mode, 2 hostile only; both on to start)
        let kept = crate::layout::tab(config).unwrap_or(3);
        let in_list = *self.list.get_or_insert(kept & 1 != 0);
        let hostile_only = *self.hostile_only.get_or_insert(kept & 2 != 0);
        if self.views.is_none() {
            use crate::gui::view::{Layout, Node, Tree};
            let mut tree = Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
            tree.get_mut(root).layout = Layout::Base;
            let list = crate::gui::multilist::create_grid(&mut tree, "NCUListView", -1, 3);
            crate::gui::multilist::set_grid(&mut tree, list, |g| g.set_icon_size(2));
            for &(id, label, own, width, flags) in COLUMNS.iter().filter(|c| matches!(c.0, 0 | 1 | 12)) {
                crate::gui::multilist::add_column(&mut tree, list, id, &crate::gui::xml::ldb_named(10000, label, own), width, flags);
            }
            crate::gui::multilist::set_sort(&mut tree, list, 12, 0);
            tree.add_child(root, list);
            self.views = Some((tree, root, list));
        }
        let textures = &mut self.textures;
        let Some((tree, root, list)) = self.views.as_mut() else { return true };
        let (root, list) = (*root, *list);
        crate::gui::multilist::set_list_mode(tree, list, in_list);
        let mut order: Vec<&NanoInfo> = nanos.unwrap_or_default().iter().filter(|n| Self::listed(n, hostile_only)).collect();
        order.sort_by_key(|n| n.left(now));
        let rows = nano_rows(ctx, textures, &order, now);
        if crate::gui::multilist::items(tree, list) != rows.as_slice() {
            crate::gui::multilist::set_items(tree, list, rows);
        }
        crate::gui::multilist::update(tree, list);
        let skin = crate::skin::skin(ctx);
        let mut open = true;
        let mut over = None;
        let centre = ctx.content_rect().center() - egui::vec2(83.0, 56.0) + egui::vec2(if self.fight { 100.0 } else { -100.0 }, 0.0);
        crate::chrome::window(ctx, palette, name, title, &mut open, |w| w.default_pos(crate::layout::pos(name, centre)).default_size(egui::vec2(167.0, 112.0)).resizable(true), |ui| {
            let area = ui.max_rect();
            // (no target: the window stays, its list does not)
            if nanos.is_some() {
                crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            } else {
                ui.allocate_space(ui.available_size());
            }
            over = ui.input(|i| i.pointer.hover_pos()).filter(|at| area.contains(*at));
        });
        if !order.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        for (_, event) in crate::gui::multilist::take_events(tree) {
            if let crate::gui::multilist::ItemEvent::Clicked(id, _) = event {
                self.info_url = Some(format!("itemid://{}/{}", crate::gui::iteminfo::NANO_IDENTITY, id));
            }
        }
        tree.signals.clear();
        if let Some(at) = over
            && ctx.input(|i| i.pointer.secondary_clicked())
        {
            let entries = [(crate::gui::xml::ldb_named(10000, "ListMode", "As a list"), true, Some(in_list)), (crate::gui::xml::ldb_named(10000, "HostileOnly", "Hostile nanos only"), true, Some(hostile_only))];
            self.menu = Some(at);
            self.popup = Some(crate::chrome::menu_marked(ctx, at, &entries));
        }
        if let Some(chosen) = crate::chrome::menu(ctx, palette, if self.fight { "fight-ncu-menu" } else { "nano-ncu-menu" }, &mut self.popup) {
            let (list_now, hostile_now) = if chosen == 0 { (!in_list, hostile_only) } else { (in_list, !hostile_only) };
            self.list = Some(list_now);
            self.hostile_only = Some(hostile_now);
            crate::layout::set_tab(config, usize::from(list_now) | usize::from(hostile_now) << 1);
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_columns_are_written_as_the_originals() {
        // (FUN_1003d7d0: ranges in metres or dashes, delays in seconds, an NCU cost of 999 as dashes, times as h:m:s)
        assert_eq!([column_text(5, 0), column_text(5, 25), column_text(11, 0)], ["---", "25m", "---"]);
        assert_eq!([column_text(6, 150), column_text(7, 2500)], ["1.50s", "25.00s"]);
        assert_eq!([column_text(10, 999), column_text(10, 42), column_text(9, 310)], ["---", "42", "310"]);
        assert_eq!([column_text(12, 0), column_text(13, 6_150), column_text(13, 1_440_000)], ["00:00:00", "00:01:01", "04:00:00"]);
        // (the type and what the list leaves out: flags 0x8000 hostile, a team cast, 0x1000000 not listed)
        let stats = |flags: u32| move |n: u32| match n {
            0 => Some(flags),
            407 => Some(310),
            54 => Some(42),
            287 => Some(1_234_567_890),
            294 => Some(150),
            _ => None,
        };
        assert_eq!(Columns::of(stats(0), false), Columns { kind: 1, nano_cost: 310, ncu: 42, range: 0, attack_delay: 150, recharge_delay: 0, hidden: false });
        assert_eq!((Columns::of(stats(0x8000), true).kind, Columns::of(stats(0), true).kind, Columns::of(stats(0x0100_0000), false).hidden), (2, 4, true));
        assert_eq!((kind_text(2), kind_text(4).0, kind_text(9).0), (("Hostile", 0xff0000), "Team", "Unknown"));
        // (the columns in the original's order, the time left the one the list starts sorted by)
        assert_eq!(COLUMNS.iter().map(|c| c.0).collect::<Vec<_>>(), [0, 1, 8, 9, 10, 13, 12, 11, 5, 6, 7]);
    }

    #[test]
    fn a_target_window_lists_the_hostile_nanos_to_start_with() {
        let nano = |kind: u8, ncu: u32, hidden: bool| NanoInfo { id: 1, name: String::new(), description: String::new(), icon: None, remaining_ms: 0, total_ms: 0, at: Instant::now(), effects: Vec::new(), cost: 0, columns: Columns { kind, ncu, hidden, ..Default::default() } };
        // (FUN_100458b0: with "Hostile Only" a nano needs the hostile flag and an NCU cost; without, only the unlisted are out)
        assert!(TargetNcu::listed(&nano(2, 30, false), true) && !TargetNcu::listed(&nano(1, 30, false), true) && !TargetNcu::listed(&nano(2, 999, false), true));
        assert!(TargetNcu::listed(&nano(1, 30, false), false) && TargetNcu::listed(&nano(2, 999, false), false) && !TargetNcu::listed(&nano(2, 30, true), false));
        assert_eq!((TargetNcu::new(true).names().1, TargetNcu::new(false).names().2), ("Fight Target NCU", "NanoTargetNCUWindowConfig"));
    }

    #[test]
    fn the_clock_reads_minutes_and_hours() {
        assert_eq!(clock(0), "0:00");
        assert_eq!(clock(61_000), "1:01");
        assert_eq!(clock(1_371_193), "22:51");
        assert_eq!(clock(3_725_000), "1:02:05");
    }

    #[test]
    fn the_time_left_counts_down_from_when_we_were_told() {
        let at = Instant::now();
        let nano = NanoInfo { id: 1, name: String::new(), description: String::new(), icon: None, remaining_ms: 10_000, total_ms: 20_000, at, effects: Vec::new(), cost: 0, columns: Columns::default() };
        assert_eq!(nano.left(at), 10_000);
        assert_eq!(nano.left(at + std::time::Duration::from_secs(4)), 6_000);
        assert_eq!(nano.left(at + std::time::Duration::from_secs(60)), 0);
    }
}
