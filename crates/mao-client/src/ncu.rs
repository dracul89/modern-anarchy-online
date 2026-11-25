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

/// The database type of nano programs; the item `stat` of a nano's duration, in hundredths of a second (1440000 for
/// a four-hour program; taken for milliseconds until 2026-11-18, which showed a tenth of every time).
const NANO: u32 = 0xfde85;
const DURATION: u32 = 8;

/// How long the right button is held on a nano before its menu opens (`MouseButtonHoldDelay`, 0.3 s).
const HOLD: std::time::Duration = std::time::Duration::from_millis(300);

/// Whether a left click on a running nano shows its info: with Shift only (GUI.dll FUN_10045259 on the list's
/// qualifier signal; a plain click's pick-up signal has no listener, FUN_10045d42).
pub fn left_click_shows(shift: bool) -> bool {
    shift
}

/// A running nano's tip in the NCU window (GUI.dll FUN_10045b79, which takes the place of the item lists' own tip
/// there): the title is the nano's name as it is; the text is its type in a colour of its own (FUN_1003d5a5: 1
/// "Friendly" white, 2 "Hostile" red, 3 "Self" yellow, 4 "Team" green, else "Unknown"), "NCU usage:" with the nano's
/// raw stat 54, "Remaining:" with the time left as hours, minutes and seconds of two digits each
/// (`N3Msg_GetBuffCurrentTime` gives hundredths of a second), and, when the nano changes anything, "Modifies:" with
/// its modifier string (`N3Msg_GetModifierString(nano, 0, .., 0)`). The words are in GUI.dll itself. No description.
pub fn tip(name: &str, kind: u8, ncu: u32, left_ms: u32, modifies: &str) -> (String, String) {
    let kind = match kind {
        1 => "<font color=\"white\">Friendly</font>",
        2 => "<font color=\"red\">Hostile</font>",
        3 => "<font color=\"yellow\">Self</font>",
        4 => "<font color=\"green\">Team</font>",
        _ => "Unknown",
    };
    let seconds = left_ms / 1000;
    let minutes = seconds / 60;
    let mut text = format!("{kind}<br>NCU usage:&nbsp;{ncu}<br>Remaining:&nbsp;{:02}:{:02}:{:02}", minutes / 60, minutes % 60, seconds % 60);
    if !modifies.is_empty() {
        text += "<br>Modifies:<br>";
        text += modifies;
    }
    (name.to_string(), text)
}

/// A right click on one of one's own running nanos (GUI.dll FUN_1004561f): whether it is taken off, and the line said.
/// `N3Msg_RemoveBuff` (Gamecode 0x1001bee9) refuses a nano whose flags have one of 0x8000, 0x4000 and 0x100
/// (`stays`) and asks the server to end any other (`CharacterAction` 0x41). The words are in GUI.dll itself.
pub fn right_click(stays: bool) -> (bool, &'static str) {
    if stays { (false, "Can't remove hostile timed nanoprogram.") } else { (true, "Deactivating friendly timed nanoprogram.") }
}

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
    /// The NCU memory the nano takes ([`ncu_cost`]): the nano's own quality stat when its flags say it takes any (the sum over
    /// the running nanos was the original's current NCU for two characters, exactly); the free nano programs carry 999 there
    /// and, but for a few, lack the flags that count.
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
    /// The player cannot take it off: its flags have one of 0x8000 (hostile), 0x4000 and 0x100
    /// (`N3Msg_RemoveBuff`, Gamecode 0x1001bee9: `GetSkill(nano, 0) & 0xc100`).
    pub stays: bool,
}

/// The spell function a team nano carries (`N3Msg_IsTeamNano`: FUN_10002483(0, 0xcf4a)).
const TEAM_CAST: u32 = 0xcf4a;

impl Columns {
    /// From a nano's stats (by number) and whether one of its spells is the team cast.
    pub fn of(stat: impl Fn(u32) -> Option<u32>, team: bool, self_only: bool) -> Columns {
        let flags = stat(0).unwrap_or(0);
        // (a stat the record does not have reads as 1234567890 in the original, shown as 0)
        let value = |n: u32| stat(n).filter(|v| *v != 1_234_567_890).unwrap_or(0);
        Columns {
            kind: if flags & 0x8000 != 0 { 2 } else if team { 4 } else if self_only { 3 } else { 1 },
            stays: flags & 0xc100 != 0,
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
            let lists = data.as_deref().map(mao_formats::spell::lists).unwrap_or_default();
            let team = lists.iter().any(|(_, spells)| spells.iter().any(|s| s.kind == TEAM_CAST));
            let alone = data.is_some() && self_only(&lists, template.as_ref().and_then(|t| t.stat(30)).unwrap_or(0));
            let columns = template.as_ref().map(|t| Columns::of(|n| t.stat(n), team, alone)).unwrap_or_default();
            NanoInfo {
                id,
                name: template.as_ref().map_or_else(|| format!("Nano #{id}"), |t| t.name.clone()),
                description: template.as_ref().map(|t| t.description.clone()).unwrap_or_default(),
                icon,
                remaining_ms,
                total_ms: template.as_ref().and_then(|t| t.stat(DURATION)).unwrap_or(0).saturating_mul(10),
                at,
                effects,
                cost: template.as_ref().map_or(0, |t| ncu_cost(t.stat(0).unwrap_or(0) as i64, t.stat(54).unwrap_or(0))),
                columns,
            }
        })
        .collect()
}

/// The NCU a running nano takes, as the original adds it to stat 180 when the nano starts (Gamecode FUN_100517f3,
/// 0x100518a7..0x10051922) and takes it away again when the nano ends (FUN_10051041, 0x1005131b..0x1005138e, after the
/// nano left the list; all three callers ask for the taking-away): its stat 54 when its flags (stat 0) have 0x10000 and
/// none of 0x4000, 0x8000 (hostile) and 0x100; nothing otherwise (heals and other instant nanos lack 0x10000). Both use
/// the same rule and the same stat, so the sum over the running list is the original's running total. A cost of 999
/// counts as it is, as there (no free program has the flags that count, but the three "Destruction" ones).
pub fn ncu_cost(flags: i64, ncu: u32) -> u32 {
    if flags & 0x10000 != 0 && flags & (0x4000 | 0x8000 | 0x100) == 0 { ncu } else { 0 }
}

/// Whether a nano is for oneself only (Gamecode FUN_100a49b1, kept at the nano's +0xbc and read by
/// `N3Msg_IsNanoSelfOnly`): every spell of all its lists aims at target 1 or 2 (spell stat 0x20), or its stat 30 has
/// 0x800000. A nano with no spells at all counts as one too, as there.
pub fn self_only(lists: &[(u32, Vec<mao_formats::spell::Spell>)], can: u32) -> bool {
    lists.iter().all(|(_, spells)| spells.iter().all(|s| matches!(s.stat(0x20), 1 | 2))) || can & 0x80_0000 != 0
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
/// the time left until a label is pressed, a hostile nano's row in red. The menu's second entry is "Auto Arrange" (dead in list mode), the sub-menu of FUN_10041637(0):
/// "By <label>" for each column but the icon's and "Sort Ascending" ([`crate::nanoprograms::arrange_menu`]); the grid's
/// sort chosen there is kept (`grid_sort_column`, `grid_sort_order`; here `NCUWindowGridSort`). Nanos whose flags have
/// 0x1000000 are in neither (FUN_100458b0). The radius is the original's `---`: `N3Msg_GetFormularRadius` (Gamecode
/// 0x1001655a) reads the nano item's `+0xb8`, which only FUN_10086142 sets, to 0. The columns' widths and the list's sort
/// are kept (`listview_config`: `multilist::Keeper`).
pub struct NcuWindow {
    pub open: bool,
    textures: HashMap<u32, egui::TextureHandle>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id)>,
    keeper: crate::gui::multilist::Keeper,
    /// The info page asked for (for the info view).
    pub info_url: Option<String>,
    /// The program whose menu is up (none: the menu was opened beside the nanos), the keys of its entries, and the menu
    /// (`PopupMenu_c`'s look).
    menu: Option<(Option<u32>, Vec<Vec<String>>)>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// List mode (`listview_mode`) and the grid's sort (column and order); not set until the kept choices are read.
    pub list: Option<bool>,
    pub grid_sort: Option<(i32, i32)>,
    /// The nano the right button went down on, and when: held there, its menu opens ([`HOLD`]).
    right_held: Option<(u32, Instant)>,
    /// What the game says of a nano taken off or not (the original's own words, not from the texts' file).
    pub lines: Vec<&'static str>,
    /// A preview's: the two target windows to show with the same nanos.
    pub target_preview: Option<[TargetNcu; 2]>,
    /// Where the window starts and its size (the original's 160 by 160; a preview's are its own).
    pub start: (egui::Pos2, egui::Vec2),
}

impl Default for NcuWindow {
    fn default() -> NcuWindow {
        NcuWindow { open: false, textures: HashMap::new(), views: None, keeper: Default::default(), info_url: None, menu: None, popup: None, list: None, grid_sort: None, right_held: None, lines: Vec::new(), target_preview: None, start: (egui::pos2(560.0, 90.0), egui::vec2(160.0, 160.0)) }
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

    /// Shows the window; returns the program the player takes off (a right click on it).
    pub fn show(&mut self, ctx: &egui::Context, nanos: &[NanoInfo], stats: &std::collections::BTreeMap<u32, i64>, palette: &Palette) -> Option<u32> {
        let mut cancel = None;
        let now = Instant::now();
        if self.views.is_none() {
            self.views = Some(Self::build(ctx));
        }
        let textures = &mut self.textures;
        let in_list = *self.list.get_or_insert_with(|| crate::layout::tab("NCUWindowConfig") == Some(1));
        // (the grid by the time left to start with, `FUN_10041258(0xc)`; else as the "Auto Arrange" menu set it, kept as
        // twice the column and the order)
        let grid_sort = *self.grid_sort.get_or_insert_with(|| crate::layout::tab("NCUWindowGridSort").map_or((12, 0), |v| ((v / 2) as i32, (v % 2) as i32)));
        let (tree, root, list) = self.views.as_mut()?;
        let (root, list) = (*root, *list);
        crate::gui::multilist::set_list_mode(tree, list, in_list);
        // (the kept widths and the list's kept sort; the grid by its own sort)
        self.keeper.before("NCUWindowConfig", tree, list, (!in_list).then_some(grid_sort));
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
        // (the list's mouse is the item lists' own (`ItemListViewBase_c` without `+0x319`, FUN_10045d42): a left click
        // raises the pick-up signal, which nothing here listens to, so a plain click does nothing; with Shift it is
        // the qualifier signal, FUN_10045259: the nano's info. Until 2026-11-25 any click showed the info.)
        let shift = ctx.input(|i| i.modifiers.shift);
        for (_, event) in crate::gui::multilist::take_events(tree) {
            if let crate::gui::multilist::ItemEvent::Clicked(id, _) = event
                && left_click_shows(shift)
            {
                self.info_url = Some(format!("itemid://{}/{}", crate::gui::iteminfo::NANO_IDENTITY, id));
            }
        }
        self.keeper.after("NCUWindowConfig", tree, list, in_list);
        tree.signals.clear();
        let text = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
        // (the right button held on a nano for `MouseButtonHoldDelay`: its menu, FUN_100457d1, whose one entry is
        // "LookAtNano" (FUN_100452c2: the info). Letting go then does nothing more. The client's own "Cancel this
        // nano" entry (NAN-060) went on 2026-11-25: the right click below is the original's cancel.)
        if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
            self.right_held = hovered.map(|h| (h.0, Instant::now()));
        }
        if !ctx.input(|i| i.pointer.secondary_down()) && !ctx.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
            self.right_held = None;
        }
        if let Some((nano, since)) = self.right_held {
            ctx.request_repaint();
            if since.elapsed() >= HOLD
                && self.popup.is_none()
                && let Some(at) = ctx.input(|i| i.pointer.interact_pos())
            {
                self.menu = Some((Some(nano), vec![vec!["look".to_string()]]));
                self.popup = Some(crate::chrome::menu_at(ctx, at, &[(text("LookAtNano", "Look at"), true)]));
                self.right_held = None;
            }
        }
        // (the right button let go on a nano before that: the list's click signal, FUN_1004561f, which takes the
        // nano off: `N3Msg_RemoveBuff`, and a line either way. No key is looked at.)
        if let Some((id, _)) = hovered
            && self.popup.is_none()
            && ctx.input(|i| i.pointer.secondary_clicked())
            && let Some(nano) = nanos.iter().find(|n| n.id == id)
        {
            let (taken_off, line) = right_click(nano.columns.stays);
            self.lines.push(line);
            if taken_off {
                cancel = Some(id);
            }
        }
        // (the window's menu, FUN_100d758a: "List Mode" with its mark and "Auto Arrange" with its sub-menu, dead in list
        // mode, then the frame's own entry: the frame's menu button, or the page's in the rollup column. Until
        // 2026-11-25 the right button beside the nanos opened it, where the original's list does nothing, FUN_10041aa4.)
        let labels: Vec<(i32, String)> = crate::gui::multilist::columns(tree, list).into_iter().map(|c| (c.id, c.label)).collect();
        if let Some(at) = crate::chrome::frame_menu("NCU") {
            use crate::gui::popup::Entry;
            let (arrange_keys, arrange) = crate::nanoprograms::arrange_menu(&labels, Some(grid_sort), false);
            let keys = vec![vec!["list".to_string()], arrange_keys, vec!["close".to_string()]];
            let entries = vec![Entry::marked(text("ListMode", "As a list"), true, in_list), Entry::submenu(text("AutoArrange", "Order"), !in_list, arrange), Entry::text(crate::chrome::close_entry(), true)];
            self.menu = Some((None, keys));
            self.popup = Some(crate::chrome::menu_of(ctx, at, entries));
        }
        // (a running nano's tip: the NCU list's own, FUN_10045b79, [`tip`]. Until 2026-11-25 a tip of this client's
        // layout: the name, "x of y left", the description and a list of what it changes (NAN-059).)
        {
            let tipped = hovered.filter(|_| self.menu.is_none()).and_then(|(id, _)| nanos.iter().find(|n| n.id == id));
            let (title, text) = tipped.map_or_else(Default::default, |nano| {
                let modifies = crate::net::item_db().map_or_else(String::new, |db| crate::gui::iteminfo::nano_modifies(&db, nano.id, stats));
                tip(&nano.name, nano.columns.kind, nano.columns.ncu, nano.left(now), &modifies)
            });
            let source = egui::Id::unique("nano-tip");
            crate::gui::tooltip::show(ctx, palette, source, tipped.map(|nano| source.with(nano.id)), &title, &text);
        }
        if let Some(crate::chrome::Picked::Entry(path)) = crate::chrome::menu_path(ctx, palette, "nano-menu", &mut self.popup)
            && let Some((nano, keys)) = self.menu.take()
            && let Some(key) = keys.get(path[0]).and_then(|k| k.get(path.get(1).copied().unwrap_or(0)))
        {
            match key.as_str() {
                "list" => {
                    self.list = Some(!in_list);
                    crate::layout::set_tab("NCUWindowConfig", usize::from(!in_list));
                }
                "look" => self.info_url = nano.map(|id| format!("itemid://{}/{}", crate::gui::iteminfo::NANO_IDENTITY, id)),
                "close" => crate::chrome::close_next("NCU"),
                // (FUN_10041303 / FUN_10041331: a column not marked yet, keeping the order; the order turned round)
                _ => {
                    if let Some(Some(sort)) = crate::loot::arrange_choice(key, Some(grid_sort)) {
                        self.grid_sort = Some(sort);
                        crate::layout::set_tab("NCUWindowGridSort", (sort.0 * 2 + sort.1) as usize);
                    }
                }
            }
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
pub(crate) fn nano_rows(ctx: &egui::Context, textures: &mut HashMap<u32, egui::TextureHandle>, order: &[&NanoInfo], now: Instant) -> Vec<crate::gui::multilist::Item> {
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
    keeper: crate::gui::multilist::Keeper,
    pub info_url: Option<String>,
    menu: Option<egui::Pos2>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// List mode and "Hostile Only"; not set until the kept choices are read.
    pub list: Option<bool>,
    pub hostile_only: Option<bool>,
}

impl TargetNcu {
    pub fn new(fight: bool) -> TargetNcu {
        TargetNcu { fight, textures: HashMap::new(), views: None, keeper: Default::default(), info_url: None, menu: None, popup: None, list: None, hostile_only: None }
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
        self.keeper.before(config, tree, list, (!in_list).then_some((12, 0)));
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
        // (the same list as the NCU window's, FUN_100dda60 builds FUN_10045d42: the info with Shift only)
        let shift = ctx.input(|i| i.modifiers.shift);
        for (_, event) in crate::gui::multilist::take_events(tree) {
            if let crate::gui::multilist::ItemEvent::Clicked(id, _) = event
                && left_click_shows(shift)
            {
                self.info_url = Some(format!("itemid://{}/{}", crate::gui::iteminfo::NANO_IDENTITY, id));
            }
        }
        self.keeper.after(config, tree, list, in_list);
        tree.signals.clear();
        // (the window's menu, FUN_100dd94d, is put into the frame's icon menu (`GetIconMenu`, 0x100de008): the frame's
        // menu button opens it, with the frame's own entry after it. Until 2026-11-25 a right click on the window did.)
        if let Some(at) = crate::chrome::frame_menu(name) {
            let entries = [
                (crate::gui::xml::ldb_named(10000, "ListMode", "As a list"), true, Some(in_list)),
                (crate::gui::xml::ldb_named(10000, "HostileOnly", "Hostile nanos only"), true, Some(hostile_only)),
                (crate::chrome::close_entry(), true, None),
            ];
            self.menu = Some(at);
            self.popup = Some(crate::chrome::menu_marked(ctx, at, &entries));
        }
        match crate::chrome::menu(ctx, palette, if self.fight { "fight-ncu-menu" } else { "nano-ncu-menu" }, &mut self.popup) {
            Some(chosen @ (0 | 1)) => {
                let (list_now, hostile_now) = if chosen == 0 { (!in_list, hostile_only) } else { (in_list, !hostile_only) };
                self.list = Some(list_now);
                self.hostile_only = Some(hostile_now);
                crate::layout::set_tab(config, usize::from(list_now) | usize::from(hostile_now) << 1);
            }
            Some(_) => crate::chrome::close_next(name),
            None => {}
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

    /// FUN_10045b79: the type's word, the NCU, the time left as hh:mm:ss, and what the nano changes when it changes
    /// anything.
    #[test]
    fn a_running_nanos_tip() {
        assert_eq!(
            tip("Essence", 1, 25, 3_725_900, ""),
            ("Essence".to_string(), "<font color=\"white\">Friendly</font><br>NCU usage:&nbsp;25<br>Remaining:&nbsp;01:02:05".to_string())
        );
        let (_, text) = tip("Root", 2, 999, 0, "  Run speed -500\n");
        assert_eq!(text, "<font color=\"red\">Hostile</font><br>NCU usage:&nbsp;999<br>Remaining:&nbsp;00:00:00<br>Modifies:<br>  Run speed -500\n");
        assert!(tip("x", 3, 1, 0, "").1.starts_with("<font color=\"yellow\">Self</font>"));
        assert!(tip("x", 4, 1, 0, "").1.starts_with("<font color=\"green\">Team</font>"));
        assert!(tip("x", 0, 1, 0, "").1.starts_with("Unknown<br>"));
        // (more than a day: the hours go on)
        assert!(tip("x", 1, 1, 100 * 3_600_000, "").1.ends_with("Remaining:&nbsp;100:00:00"));
    }

    /// FUN_1004561f and `N3Msg_RemoveBuff`: a right click takes a friendly nano off and says so; one whose flags
    /// have 0x8000, 0x4000 or 0x100 stays, with the other line. FUN_10045259: the info by Shift + click only.
    #[test]
    fn a_right_click_takes_a_friendly_nano_off() {
        let stats = |flags: u32| move |n: u32| (n == 0).then_some(flags);
        assert_eq!(right_click(Columns::of(stats(0x10000), false, false).stays), (true, "Deactivating friendly timed nanoprogram."));
        for flags in [0x8000, 0x4000, 0x100] {
            assert_eq!(right_click(Columns::of(stats(flags), false, false).stays), (false, "Can't remove hostile timed nanoprogram."));
        }
        assert!(left_click_shows(true) && !left_click_shows(false));
    }

    /// NAN-004: the running nanos that count for the NCU in use (FUN_100517f3 at 0x100518a7, FUN_10051041 at 0x1005131b):
    /// flag 0x10000 and none of 0x4000, 0x8000, 0x100; their stat 54. A heal (0x80000002) or a hostile nano (0x8000) counts nothing.
    #[test]
    fn only_lasting_friendly_nanos_take_ncu() {
        assert_eq!(ncu_cost(0x10000, 18), 18);
        assert_eq!(ncu_cost(0x8001_0000u32 as i64, 7), 7);
        assert_eq!([ncu_cost(0x80000002u32 as i64, 2), ncu_cost(0x18000, 6), ncu_cost(0x14000, 6), ncu_cost(0x10100, 6)], [0; 4]);
        // (the original adds stat 54 whatever it is: a 999 counts)
        assert_eq!(ncu_cost(0x90002, 999), 999);
    }

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
        assert_eq!(Columns::of(stats(0), false, false), Columns { kind: 1, nano_cost: 310, ncu: 42, range: 0, attack_delay: 150, recharge_delay: 0, hidden: false, stays: false });
        assert_eq!((Columns::of(stats(0x8000), true, true).kind, Columns::of(stats(0), true, true).kind, Columns::of(stats(0x0100_0000), false, false).hidden), (2, 4, true));
        // (NAN-045: FUN_1003f4cf's type 3, self only, after hostile and team)
        assert_eq!(Columns::of(stats(0), false, true).kind, 3);
        let spell = |target: i32| mao_formats::spell::Spell { kind: 0xcf1a, instance: 0, criteria: Vec::new(), special: Vec::new(), stats: vec![(0x20, target)], strings: Vec::new() };
        assert!(self_only(&[(1, vec![spell(1), spell(2)])], 0));
        assert!(!self_only(&[(1, vec![spell(1)]), (14, vec![spell(3)])], 0));
        assert!(self_only(&[(1, vec![spell(3)])], 0x80_0000));
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
