//! The inventory and wear windows as the original builds them in code (GUI.dll's `InventoryGUIModule_c`), on the item grid of
//! `gui::multilist`:
//!
//! - The inventory (`inventory_window`, `SlotInventoryActivated` 0x100c766b -> `InventoryView_c` FUN_100ccefe type 0): an
//!   `ItemContainerView_c` (FUN_100ce55d) grid holding at most 30 (`SetMaxItemCount(0x1e)`), its items packed in order
//!   (feature flags 0x15), each place's slot picture under them.
//! - The wear window (`wear_window`, `SlotWearViewActivated` 0x100c76f1 -> `WearView_c` FUN_100e24b8): a `TabView` of four
//!   item grids, Weapon / Cloth / Implant / Social (text.mdb 10000's "Weapon", "Cloth", "Implant", and "Social"), each 3 by 5
//!   cells (`SetViewCellCounts`) 11 by 9 apart (`SetGridIconSpacing`) over `GFX_GUI_WEARVIEW_BG1` (the implants'
//!   `GFX_GUI_WEARVIEW_BG2`), every place at its own cell with its label (FUN_100ce223 -> `SetGridLabel`; the tables built
//!   by FUN_101a5910, FUN_101a5514, FUN_101a5734 and FUN_101a5b30; the labels text.mdb 505's 300+ for weapons, 100+ for
//!   clothes and social, 200+ for implants).
//!
//! The mouse is the original's "new mouse" (`UseNewMouseFunc`; `ItemListViewBase_c`'s mouse down FUN_10041aa4 and up
//! FUN_1004141b): a left click takes the item up, and it stays on the pointer until the next click puts it down (on a
//! wear cell it is worn there, on the inventory it goes back into the backpack, on a hotbar slot it is added to it, on
//! an open bag it goes in); a press that moves does the same in one stroke. A right click uses the item (button 2 is
//! the use signal, FUN_1004266a), a bag's use opens its window and the next closes it, worn or carried. A right click
//! with an item in hand uses that item on the one clicked (a tradeskill combine: the one in hand is the source).
//! Shift+click shows the item's info; Ctrl+double click puts it into an open trade (FUN_100cae1b); Shift+right click
//! opens the client's own menu (Info, Use, Delete). There is no use by double click any more: the first click of two
//! has the item in hand.

use std::collections::{BTreeMap, HashMap};

use crate::gear::GearItem;
use crate::gui::multilist::{self, Item};
use crate::gui::view::{Id, Layout, Node, Tree};
use crate::theme::Palette;
use mao_net::inventory::BACKPACK_FIRST;

/// The inventory's places.
pub const PACK_PLACES: u32 = 30;

/// A wear tab: its grid's cells (place, column, row) and labels' text.mdb keys, from the original's tables.
struct WearTab {
    name: &'static str,
    /// The first place's number less one (Weapon 0: places 1-15; Cloth 16; Implant 32; Social 48).
    base: u32,
    /// (slot within the tab, column, row), and the label key of each in order.
    cells: &'static [(u32, usize, usize)],
    label_base: i32,
    picture: &'static str,
}

const WEAPON_CELLS: &[(u32, usize, usize)] = &[(1, 0, 0), (15, 1, 0), (2, 2, 0), (3, 0, 1), (4, 1, 1), (5, 2, 1), (6, 0, 2), (7, 1, 2), (8, 2, 2), (9, 0, 3), (10, 1, 3), (11, 2, 3), (12, 0, 4), (13, 1, 4), (14, 2, 4)];
const FULL_CELLS: &[(u32, usize, usize)] = &[(1, 0, 0), (2, 1, 0), (3, 2, 0), (4, 0, 1), (5, 1, 1), (6, 2, 1), (7, 0, 2), (8, 1, 2), (9, 2, 2), (10, 0, 3), (11, 1, 3), (12, 2, 3), (13, 0, 4), (14, 1, 4), (15, 2, 4)];
const IMPLANT_CELLS: &[(u32, usize, usize)] = &[(1, 0, 0), (2, 1, 0), (3, 2, 0), (4, 0, 1), (5, 1, 1), (6, 2, 1), (7, 0, 2), (8, 1, 2), (9, 2, 2), (10, 0, 3), (11, 1, 3), (12, 2, 3), (13, 1, 4)];

const WEAR_TABS: [WearTab; 4] = [
    WearTab { name: "#10000:98273118", base: 0, cells: WEAPON_CELLS, label_base: 300, picture: "GFX_GUI_WEARVIEW_BG1" },
    WearTab { name: "#10000:147680743", base: 16, cells: FULL_CELLS, label_base: 100, picture: "GFX_GUI_WEARVIEW_BG1" },
    WearTab { name: "#10000:30802355", base: 32, cells: IMPLANT_CELLS, label_base: 200, picture: "GFX_GUI_WEARVIEW_BG2" },
    WearTab { name: "Social", base: 48, cells: FULL_CELLS, label_base: 100, picture: "GFX_GUI_WEARVIEW_BG1" },
];

/// One grid in its own small view tree.
struct GridView {
    tree: Tree,
    root: Id,
    list: Id,
    shown: Vec<(u32, u16, Option<u32>)>,
}

impl GridView {
    fn new(ctx: &egui::Context, max_items: i32, columns: usize) -> GridView {
        let mut tree = Tree::default();
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
        tree.get_mut(root).layout = Layout::Base;
        let list = multilist::create_grid(&mut tree, "ItemContainerView", max_items, columns);
        tree.add_child(root, list);
        GridView { tree, root, list, shown: Vec::new() }
    }
}

/// What the grids hand back this frame.
#[derive(Default)]
pub struct Events {
    pub moves: Vec<(u32, u32)>,
    pub uses: Vec<u32>,
    pub deletes: Vec<u32>,
    pub infos: Vec<String>,
    pub trade_adds: Vec<u32>,
    /// The backpack item in hand used on another by a right click: a tradeskill combine, `(the item, the one it went on)`.
    pub combines: Vec<(u32, u32)>,
    /// Item links for the chat's line (the menu's "Create Reference"): the markup of each.
    pub references: Vec<String>,
}

#[derive(Default)]
pub struct ItemWindows {
    inventory: Option<GridView>,
    wear: Vec<GridView>,
    /// The wear tab shown, and whether the wear window was closed on its own.
    pub tab: usize,
    pub wear_closed: bool,
    /// The item whose menu is open, and the place awaiting a second click to be destroyed.
    menu: Option<(u32, egui::Pos2)>,
    /// The item the right button went down on, and when: held there, its menu opens.
    right_held: Option<(u32, std::time::Instant)>,
    popup: Option<crate::gui::popup::PopupMenu>,
    confirm: Option<u32>,
}

/// What a click on an item of a grid does.
/// An item's link as the chat sends it (the menu's "Create Reference").
pub fn reference(item: &GearItem) -> String {
    format!("<a href=\"itemref://{}/{}/{}\">{}</a>", item.item_id, item.high_id, item.quality, item.name)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Click {
    TakeUp,
    Info,
    Trade,
    Nothing,
}

/// A click's meaning by its count (0: a press that moved, a drag), the keys held and whether an item is in hand
/// already. The original's lists look at the qualifiers first (FUN_10041aa4: Shift or Ctrl held, nothing is taken up).
pub fn click(clicks: u32, shift: bool, ctrl: bool, holding: bool) -> Click {
    match (shift, ctrl) {
        (true, _) if clicks > 0 => Click::Info,
        (false, true) if clicks > 1 => Click::Trade,
        (false, false) if !holding => Click::TakeUp,
        _ => Click::Nothing,
    }
}

/// An item's grid entry: its place, icon and count.
fn entry(item: &GearItem, textures: &mut HashMap<u32, egui::TextureHandle>, ctx: &egui::Context, pos: Option<(usize, usize)>) -> Item {
    Item {
        id: item.slot as i64,
        icon: item.icon.as_ref().map(|icon| textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("item-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id()),
        count: item.count as u32,
        cells: vec![(1, multilist::CellView::Text(item.name.clone()))],
        keys: vec![(1, item.name.clone())],
        pos,
        left: None,
        colour: None,
    }
}

/// Puts the items into a grid when they changed.
fn fill(view: &mut GridView, items: &[(&GearItem, Option<(usize, usize)>)], textures: &mut HashMap<u32, egui::TextureHandle>, ctx: &egui::Context) {
    let now: Vec<(u32, u16, Option<u32>)> = items.iter().map(|(i, _)| (i.slot, i.count, i.icon.as_ref().map(|c| c.id))).collect();
    if view.shown != now {
        let rows = items.iter().map(|(i, pos)| entry(i, textures, ctx, *pos)).collect();
        multilist::set_items(&mut view.tree, view.list, rows);
        view.shown = now;
    }
}

/// The wear window's four grids.
fn wear_grids(ctx: &egui::Context) -> Vec<GridView> {
    WEAR_TABS
        .iter()
        .map(|tab| {
            let mut g = GridView::new(ctx, -1, 3);
            let labels: Vec<((usize, usize), String)> = tab.cells.iter().enumerate().map(|(k, &(_, x, y))| ((x, y), crate::gui::xml::ldb_text(&format!("#505:{}", tab.label_base + k as i32)))).collect();
            multilist::set_grid(&mut g.tree, g.list, |m| {
                m.spacing = (11.0, 9.0);
                m.columns = Some(3);
                m.auto_arrange = false;
                m.labels = labels;
                m.background = Some(tab.picture.to_string());
            });
            g
        })
        .collect()
}

/// A wear grid drawn to fill its page's width: the three columns of five cells grown, squares still, until they reach
/// from side to side (never smaller than their own size). The original's cells keep their size and its window is as
/// wide as they; here the window is as wide as its four tabs in this client's fonts, or as its rollup page, and the
/// grid at its own size left empty room beside it (the user, 2026-11-11: the squares should fill the whole window).
fn filled_grid(ui: &mut egui::Ui, tree: &mut Tree, root: Id, list: Id, skin: Option<&crate::skin::Skin>, palette: &Palette) -> egui::Response {
    let natural = multilist::grid_natural(tree, list).unwrap_or_else(|| crate::gui::host::preferred_pixels(tree, root, false));
    let scale = (ui.available_width() / natural.x.max(1.0)).max(1.0);
    multilist::set_grid_scale(tree, list, scale);
    let size = multilist::grid_pixels(tree, list).unwrap_or(natural * scale);
    // (the cells are whole pixels, so a few may be left over: half on each side)
    let spare = ((ui.available_width() - size.x) * 0.5).floor().max(0.0);
    let place = egui::Rect::from_min_size(ui.available_rect_before_wrap().min + egui::vec2(spare, 0.0), size);
    ui.scope_builder(egui::UiBuilder::new().max_rect(place), |ui| {
        ui.set_min_size(size);
        crate::gui::host::show(ui, tree, root, skin, palette)
    })
    .inner
}

/// A wear cell's label as plain text.
fn cell_label(tab: &WearTab, slot: u32) -> String {
    tab.cells.iter().position(|c| tab.base + c.0 == slot).map(|k| crate::gui::xml::ldb_text(&format!("#505:{}", tab.label_base + k as i32))).unwrap_or_default().replace("<center>", "").replace("</center>", "").replace("<br>", " ")
}

/// The inspect window: the original's `WearView_c` (FUN_100e24b8) built with the inspected identity: the title "Inspect",
/// `inspect_window` and `InspectViewConfig` in place of the wear window's, the same four tabs of cells. The items are only
/// looked at: a tip under the pointer, a right click (the use button) shows the item's page; nothing is dragged.
#[derive(Default)]
pub struct InspectWindow {
    grids: Vec<GridView>,
    open: bool,
    tab: usize,
    /// The inspection shown (its count).
    seen: u32,
}

impl InspectWindow {
    pub fn show(&mut self, ctx: &egui::Context, inspected: &(u32, String, Vec<GearItem>), stats: &BTreeMap<u32, i64>, palette: &Palette, textures: &mut HashMap<u32, egui::TextureHandle>) -> Vec<String> {
        let mut infos = Vec::new();
        let (count, name, items) = inspected;
        if *count != self.seen {
            self.seen = *count;
            self.open = true;
        }
        if !self.open {
            return infos;
        }
        if self.grids.is_empty() {
            self.grids = wear_grids(ctx);
            // (`InspectViewConfig`'s `selected_tab`, kept apart from the wear window's)
            self.tab = crate::layout::tab("InspectViewConfig").unwrap_or(self.tab);
        }
        let by_slot: HashMap<u32, &GearItem> = items.iter().map(|i| (i.slot, i)).collect();
        for (k, tab) in WEAR_TABS.iter().enumerate() {
            let rows: Vec<(&GearItem, Option<(usize, usize)>)> = tab.cells.iter().filter_map(|&(slot, x, y)| by_slot.get(&(tab.base + slot)).map(|i| (*i, Some((x, y))))).collect();
            fill(&mut self.grids[k], &rows, textures, ctx);
        }
        let skin = crate::skin::skin(ctx);
        let accent = crate::chrome::rgb(palette.accent);
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let names: Vec<String> = WEAR_TABS.iter().map(|t| crate::gui::xml::ldb_text(t.name)).collect();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let mut tab = self.tab.min(WEAR_TABS.len() - 1);
        let mut hovered: Option<(u32, String)> = None;
        let grids = &mut self.grids;
        // (the title names the character too: several inspections look the same otherwise)
        let title = if name.is_empty() { "Inspect".to_string() } else { format!("Inspect: {name}") };
        crate::chrome::window(ctx, palette, "Inspect", &title, &mut self.open, |w| w.default_pos(crate::layout::pos("Inspect", egui::pos2(420.0, 90.0))).resizable(false), |ui| {
            ui.set_min_width(crate::chrome::tabs_width(ui, &name_refs));
            crate::chrome::tab_view(ui, palette, &name_refs, &mut tab, |ui, t| {
                let view = &mut grids[t];
                let (tree, root, list) = (&mut view.tree, view.root, view.list);
                let response = filled_grid(ui, tree, root, list, skin.as_deref(), palette);
                if let Some(at) = pointer
                    && let Some(slot) = multilist::item_at(tree, list, response.rect.min, at)
                {
                    hovered = Some((slot as u32, cell_label(&WEAR_TABS[t], slot as u32)));
                }
            });
        });
        if self.tab != tab {
            self.tab = tab;
            crate::layout::set_tab("InspectViewConfig", tab);
        }
        for view in self.grids.iter_mut() {
            for (_, event) in multilist::take_events(&mut view.tree) {
                if let multilist::ItemEvent::Clicked(slot, _) = event {
                    multilist::select(&mut view.tree, view.list, Some(slot));
                }
            }
            view.tree.signals.clear();
        }
        if let Some((slot, label)) = hovered
            && let Some(item) = by_slot.get(&slot)
        {
            if ctx.input(|i| i.pointer.secondary_clicked()) {
                infos.push(format!("itemref://{}/{}/{}", item.item_id, item.high_id, item.quality));
            }
            egui::Area::new(egui::Id::unique("inspect-tip")).order(egui::Order::Tooltip).fixed_pos(pointer.unwrap_or_default() + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| crate::gear::tooltip(ui, item, &label, stats, accent));
            });
        }
        infos
    }
}

impl ItemWindows {
    /// An item's menu: look at it, use it, put a link to it into the chat's line, destroy it.
    fn item_menu() -> [(String, bool); 4] {
        ["Look At", "Use", "Create Reference", "Delete"].map(|label| (label.to_string(), true))
    }

    fn build(&mut self, ctx: &egui::Context) {
        let mut inventory = GridView::new(ctx, PACK_PLACES as i32, 5);
        multilist::set_max_items(&mut inventory.tree, inventory.list, PACK_PLACES as i32);
        self.inventory = Some(inventory);
        self.wear = wear_grids(ctx);
        // (the tab shown when the window last went, `WearViewConfig`'s `selected_tab`: FUN_100e1ce0 saves it, FUN_100e24b8 reads it)
        self.tab = crate::layout::tab("WearViewConfig").unwrap_or(self.tab);
    }

    /// The two windows; what was done with the items.
    #[allow(clippy::too_many_arguments)]
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool, items: &[GearItem], stats: &BTreeMap<u32, i64>, palette: &Palette, textures: &mut HashMap<u32, egui::TextureHandle>) -> Events {
        let mut events = Events::default();
        if self.inventory.is_none() {
            self.build(ctx);
        }
        let skin = crate::skin::skin(ctx);
        let accent = crate::chrome::rgb(palette.accent);
        let by_slot: HashMap<u32, &GearItem> = items.iter().map(|i| (i.slot, i)).collect();
        let released = crate::gear::letting_go(ctx);
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        // (the item in hand; one that is no longer there, used up or moved by the server, leaves the hand)
        let mut dragged = crate::gear::held();
        if dragged.is_some_and(|d| !by_slot.contains_key(&d)) {
            crate::gear::put_down();
            dragged = None;
        }
        let mut hovered_item: Option<(u32, String)> = None;

        // The inventory.
        let mut pack: Vec<&GearItem> = items.iter().filter(|i| i.slot >= BACKPACK_FIRST).collect();
        pack.sort_by_key(|i| i.slot);
        let mut is_open = *open;
        if let Some(view) = self.inventory.as_mut() {
            let rows: Vec<(&GearItem, Option<(usize, usize)>)> = pack.iter().map(|i| (*i, None)).collect();
            fill(view, &rows, textures, ctx);
            multilist::update(&mut view.tree, view.list);
            let (w, h) = multilist::grid::size(PACK_PLACES as usize, 5);
            let (tree, root, list) = (&mut view.tree, view.root, view.list);
            let shown = crate::chrome::window(ctx, palette, "Inventory", "Inventory", &mut is_open, |w2| w2.default_pos(crate::layout::pos("Inventory", egui::pos2(40.0, 70.0))).default_size(egui::vec2(w + 32.0, h.min(4.0 * multilist::grid::STEP + 12.0))).resizable(true), |ui| {
                let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
                if let Some(at) = pointer
                    && let Some(slot) = multilist::item_at(tree, list, response.rect.min, at)
                {
                    hovered_item = Some((slot as u32, String::new()));
                }
                // (`MAO_ITEM_TRACE`: where each item is, in the window's pixels, for a script's clicks; told when it changes)
                if crate::gear::traced() {
                    let k = ui.ctx().pixels_per_point();
                    let mut places: BTreeMap<i64, (f32, f32, u32)> = BTreeMap::new();
                    let (mut x, rect) = (response.rect.left() + 4.0, response.rect.intersect(ui.clip_rect()));
                    while x < rect.right() {
                        let mut y = rect.top() + 4.0;
                        while y < rect.bottom() {
                            if let Some(slot) = multilist::item_at(tree, list, response.rect.min, egui::pos2(x, y)) {
                                let p = places.entry(slot).or_insert((0.0, 0.0, 0));
                                *p = (p.0 + x, p.1 + y, p.2 + 1);
                            }
                            y += 4.0;
                        }
                        x += 4.0;
                    }
                    let told: Vec<String> = places.iter().map(|(slot, p)| format!("{slot} at {:.0};{:.0} {:?}{}", p.0 / p.2 as f32 * k, p.1 / p.2 as f32 * k, by_slot.get(&(*slot as u32)).map_or("", |i| i.name.as_str()), if by_slot.get(&(*slot as u32)).is_some_and(|i| i.bag) { " (bag)" } else { "" })).collect();
                    let id = egui::Id::unique("item-trace-places");
                    if ui.ctx().data(|d| d.get_temp::<Vec<String>>(id)).as_ref() != Some(&told) {
                        eprintln!("items: the inventory's items: {told:?}");
                        ui.ctx().data_mut(|d| d.insert_temp(id, told));
                    }
                }
                response
            });
            // (an item from the wear window let go over the inventory goes back into the backpack: the first free place)
            if let (Some(from), Some(shown)) = (dragged, &shown)
                && released
                && from < BACKPACK_FIRST
                && shown.response.contains_pointer()
                && let Some(free) = (BACKPACK_FIRST..BACKPACK_FIRST + PACK_PLACES).find(|p| !by_slot.contains_key(p))
            {
                events.moves.push((from, free));
            }
            Self::events_of(&mut view.tree, view.list, &mut events, items, dragged);
        }
        *open = is_open;
        if !is_open {
            self.wear_closed = false;
        }

        // The wear window.
        if *open && !self.wear_closed {
            let mut wear_open = true;
            let names: Vec<String> = WEAR_TABS.iter().map(|t| crate::gui::xml::ldb_text(t.name)).collect();
            let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            for (k, tab) in WEAR_TABS.iter().enumerate() {
                let rows: Vec<(&GearItem, Option<(usize, usize)>)> = tab.cells.iter().filter_map(|&(slot, x, y)| by_slot.get(&(tab.base + slot)).map(|i| (*i, Some((x, y))))).collect();
                fill(&mut self.wear[k], &rows, textures, ctx);
            }
            let mut tab = self.tab.min(WEAR_TABS.len() - 1);
            let mut drop_on: Option<u32> = None;
            let wear = &mut self.wear;
            let title = crate::gui::xml::ldb_text("#10000:383874");
            crate::chrome::window(ctx, palette, "Wear", &title, &mut wear_open, |w| w.default_pos(crate::layout::pos("Wear", egui::pos2(380.0, 70.0))).resizable(false), |ui| {
                // (as wide as its four tabs: the window sizes itself by what is in it, and the grid is narrower than they)
                ui.set_min_width(crate::chrome::tabs_width(ui, &name_refs));
                crate::chrome::tab_view(ui, palette, &name_refs, &mut tab, |ui, t| {
                    let view = &mut wear[t];
                    let (tree, root, list) = (&mut view.tree, view.root, view.list);
                    let response = filled_grid(ui, tree, root, list, skin.as_deref(), palette);
                    if let Some(at) = pointer {
                        if let Some(slot) = multilist::item_at(tree, list, response.rect.min, at) {
                            hovered_item = Some((slot as u32, cell_label(&WEAR_TABS[t], slot as u32)));
                        }
                        // (an item let go over a wear cell is worn there)
                        if dragged.is_some() && released && let Some(cell) = multilist::cell_at(tree, list, response.rect.min, at) {
                            let tab = &WEAR_TABS[t];
                            drop_on = tab.cells.iter().find(|c| (c.1, c.2) == cell).map(|c| tab.base + c.0);
                        }
                    }
                });
            });
            if self.tab != tab {
                self.tab = tab;
                crate::layout::set_tab("WearViewConfig", tab);
            }
            if let (Some(from), Some(to)) = (dragged, drop_on)
                && from != to
            {
                events.moves.push((from, to));
            }
            for view in self.wear.iter_mut() {
                Self::events_of(&mut view.tree, view.list, &mut events, items, dragged);
            }
            if !wear_open {
                self.wear_closed = true;
            }
        }

        // (a right click uses the item, as the original's list does (`ItemListViewBase_c`'s FUN_1004141b: button 2 -> the
        // use signal); with an item in hand, that item is used on the one clicked (`UseItemOnItem`: the one in hand is
        // the source, so the order of the two matters) and goes back to its place; Shift+right click opens the client's
        // own menu where it was clicked)
        // (the right button held on an item for a moment opens its menu, and letting go then is not a use; the user's word
        // for the original's: "long right click on item should give a menu with look at item, delete and create reference")
        const HOLD: std::time::Duration = std::time::Duration::from_millis(400);
        if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
            self.right_held = hovered_item.as_ref().map(|(slot, _)| (*slot, std::time::Instant::now()));
        }
        if !ctx.input(|i| i.pointer.secondary_down()) && !ctx.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
            self.right_held = None;
        }
        if let Some((held, since)) = self.right_held {
            ctx.request_repaint();
            if since.elapsed() >= HOLD
                && dragged.is_none()
                && self.menu.is_none()
                && let (Some((slot, _)), Some(at)) = (&hovered_item, pointer)
                && *slot == held
            {
                if crate::gear::traced() {
                    eprintln!("items: the menu of place {held} opened by the held right button");
                }
                self.menu = Some((held, at));
                self.popup = Some(crate::chrome::menu_at(ctx, at, &Self::item_menu()));
                self.confirm = None;
                self.right_held = None;
            }
        }
        if let (Some((slot, _)), Some(at)) = (&hovered_item, pointer)
            && self.menu.is_none()
            && ctx.input(|i| i.pointer.secondary_clicked())
        {
            if let Some(from) = dragged {
                if from != *slot && from >= BACKPACK_FIRST && *slot >= BACKPACK_FIRST {
                    events.combines.push((from, *slot));
                    crate::gear::put_down();
                }
            } else if ctx.input(|i| i.modifiers.shift) {
                self.menu = Some((*slot, at));
                self.popup = Some(crate::chrome::menu_at(ctx, at, &Self::item_menu()));
                self.confirm = None;
            } else {
                events.uses.push(*slot);
            }
        }
        // (the item under the pointer: its tip)
        if let Some((slot, label)) = hovered_item
            && self.menu.is_none()
            && dragged.is_none()
            && let Some(item) = by_slot.get(&slot)
        {
            egui::Area::new(egui::Id::unique("item-tip")).order(egui::Order::Tooltip).fixed_pos(pointer.unwrap_or_default() + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| crate::gear::tooltip(ui, item, &label, stats, accent));
            });
        }
        // (the item in hand can be set on a hotbar slot: the slots take a drag's load, which goes with every button let
        // go, so it is given again while the item is held)
        if let Some(item) = dragged.and_then(|from| by_slot.get(&from))
            && !egui::DragAndDrop::has_any_payload(ctx)
        {
            egui::DragAndDrop::set_payload(ctx, crate::hotbar::Action::Item { id: item.item_id, name: item.name.clone(), icon: item.icon.as_ref().map(|i| i.id) });
        }
        // (the item in hand follows the pointer)
        if let (Some(from), Some(at)) = (dragged, pointer)
            && let Some(icon) = by_slot.get(&from).and_then(|i| i.icon.as_ref()).and_then(|icon| textures.get(&icon.id))
        {
            ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("dragged-item"))).image(icon.id(), egui::Rect::from_center_size(at, egui::vec2(40.0, 40.0)), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::from_white_alpha(200));
        }
        // (an item's menu: the client's own entries, in the original's popup; deleting asks first)
        if let Some(k) = crate::chrome::menu(ctx, palette, "item-menu", &mut self.popup)
            && let Some((slot, _)) = self.menu
            && let Some(item) = by_slot.get(&slot)
        {
            match k {
                0 => events.infos.push(format!("itemref://{}/{}/{}", item.item_id, item.high_id, item.quality)),
                1 => events.uses.push(slot),
                2 => {
                    if crate::gear::traced() {
                        eprintln!("items: a reference to place {slot}: {}", reference(item));
                    }
                    events.references.push(reference(item));
                }
                _ => self.confirm = Some(slot),
            }
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        if let Some(slot) = self.confirm {
            match by_slot.get(&slot) {
                Some(item) => {
                    let question = format!("Destroy <font color=yellow>{}</font>? It cannot be brought back.", item.name);
                    if let Some(yes) = crate::chrome::question(ctx, palette, "delete-item", &crate::gui::xml::ldb_named(10001, "Warning", "Warning"), &question) {
                        if yes {
                            events.deletes.push(slot);
                        }
                        self.confirm = None;
                    }
                }
                None => self.confirm = None,
            }
        }
        events
    }

    /// A grid's clicks and drags: a click takes the item up, as a drag does (with one in hand already, `held`, the click
    /// is the one that puts that down); Shift+click shows its info; Ctrl+double click puts it into the trade.
    fn events_of(tree: &mut Tree, list: Id, events: &mut Events, items: &[GearItem], held: Option<u32>) {
        let Some(ctx) = tree.measure.ctx.clone() else { return };
        let (ctrl, shift) = ctx.input(|i| (i.modifiers.ctrl, i.modifiers.shift));
        for (_, event) in multilist::take_events(tree) {
            let (slot, clicks) = match event {
                multilist::ItemEvent::Clicked(slot, clicks) => (slot, clicks),
                multilist::ItemEvent::Dragged(slot) => (slot, 0),
            };
            let Some(item) = items.iter().find(|i| i.slot as i64 == slot) else { continue };
            if clicks > 0 {
                multilist::select(tree, list, Some(slot));
            }
            match click(clicks, shift, ctrl, held.is_some()) {
                Click::Info => events.infos.push(format!("itemref://{}/{}/{}", item.item_id, item.high_id, item.quality)),
                Click::Trade => events.trade_adds.push(item.slot),
                Click::TakeUp => {
                    crate::gear::pick_up(item.slot);
                    egui::DragAndDrop::set_payload(&ctx, crate::hotbar::Action::Item { id: item.item_id, name: item.name.clone(), icon: item.icon.as_ref().map(|i| i.id) });
                }
                Click::Nothing => {}
            }
        }
        tree.signals.clear();
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_takes_up_shows_or_trades() {
        // (a plain click or a drag takes the item up; not with one in hand already)
        assert_eq!(click(1, false, false, false), Click::TakeUp);
        assert_eq!(click(0, false, false, false), Click::TakeUp);
        assert_eq!(click(1, false, false, true), Click::Nothing);
        assert_eq!(click(2, false, false, true), Click::Nothing);
        // (Shift: the item's info, whatever is in hand; a drag with Shift held is nothing)
        assert_eq!(click(1, true, false, false), Click::Info);
        assert_eq!(click(1, true, false, true), Click::Info);
        assert_eq!(click(0, true, false, false), Click::Nothing);
        // (Ctrl: the second click of two puts it into the trade; nothing is taken up with Ctrl held)
        assert_eq!(click(1, false, true, false), Click::Nothing);
        assert_eq!(click(2, false, true, false), Click::Trade);
    }
}
