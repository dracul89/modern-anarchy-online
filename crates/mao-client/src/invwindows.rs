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
//! an open bag it goes in). A press that moves takes nothing up: the list signals a click only when the button comes
//! up on the cell it went down on (`MultiListView_c::MouseUp`, 0x10133d04). A right click uses the item (button 2 is
//! the use signal, FUN_1004266a), a bag's use opens its window and the next closes it, worn or carried. A right click
//! with an item in hand uses that item on the one clicked (`UseItemOnItem`, the use message's action 5; the one in
//! hand is the source), in the inventory and on a worn item alike; with Shift, in the inventory, it is a tradeskill
//! combine of the two (FUN_100cc855, FUN_100e220f). Shift+click shows the item's info; Ctrl + right click puts it
//! into an open trade (FUN_100cae1b). There is no use by double click, in the original either: the first click of
//! two has the item in hand, and the second puts it back where it was.
//!
//! The backpack's items stand where they are put (`places.rs`; the original's drop handler FUN_100cc855): the item in
//! hand let go on an empty cell of the inventory moves there, which is the client's own matter (nothing is sent: a
//! cell is not the item's place on the server); let go on another item the two join when they are stacks of one kind
//! (`N3Msg_JoinItems`), else they change cells and the other is in hand. Ctrl+click on a stack asks how many to take
//! off it (the original's split mode counts by the mouse's way sideways while the button is down, FUN_10041926 /
//! FUN_10041ca4; here a small box asks for the number), and that many, in hand, let go on an empty cell become a
//! stack of their own (`N3Msg_SplitItem`); anywhere else the split is given up.

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
    /// The items last put in the grid: their places, counts, icons and cells.
    shown: Vec<(u32, u16, Option<u32>, Option<(usize, usize)>)>,
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
    /// The backpack item in hand put on another of the backpack by Shift + right click: a tradeskill combine, `(the
    /// item, the one it went on)`.
    pub combines: Vec<(u32, u32)>,
    /// The item in hand used on another, carried or worn, by a right click: `(the item, the one it went on)`.
    pub use_ons: Vec<(u32, u32)>,
    /// Item links for the chat's line (the menu's "Create Reference"): the markup of each.
    pub references: Vec<String>,
    /// The stack in hand let go on another of its kind: `(the stack that stays, the one in hand)`.
    pub joins: Vec<(u32, u32)>,
    /// A split let go on an empty cell: `(the stack, how many to take off it)`.
    pub splits: Vec<(u32, u32)>,
    /// What the game says of a join or a split it does not make (its texts' names).
    pub feedback: Vec<&'static str>,
}

/// What an entry of an item's menu does ([`ItemWindows::item_menu`]).
#[derive(Clone, Copy, Debug, PartialEq)]
enum ItemChoice {
    LookAt,
    Reference,
    Delete,
    /// Lock the worn item to its place (true) or unlock it.
    Lock(bool),
}

#[derive(Default)]
pub struct ItemWindows {
    inventory: Option<GridView>,
    wear: Vec<GridView>,
    /// The wear tab shown, and whether the wear window is open: a window of its own (`wear_window`,
    /// `SlotWearViewActivated` 0x100c76f1), opened and closed apart from the inventory (`SlotInventoryActivated` 0x100c766b).
    pub tab: usize,
    pub wear_open: bool,
    /// The item whose menu is open, and the place awaiting a second click to be destroyed.
    menu: Option<(u32, egui::Pos2)>,
    /// The item the right button went down on, and when: held there, its menu opens.
    right_held: Option<(u32, std::time::Instant)>,
    popup: Option<crate::gui::popup::PopupMenu>,
    confirm: Option<u32>,
    /// The stack a split is asked of (Ctrl+click), and the count typed so far.
    split_box: Option<(u32, String)>,
    /// A backpack place about to get an item (a split's new stack, an item taken off), the cell it was let go on,
    /// and since when: the item is shown there when it comes (the original's `+0x150`, FUN_100ce25d).
    awaited: Option<(u32, usize, std::time::Instant)>,
    /// The wear pages' labels over their items or under them (`WearView_c`'s menu, FUN_100e20d6: "Labels On Top",
    /// kept as `WearViewConfig`'s `labels_on_top`; on at first), once read from what is kept; and the menu while it
    /// is up.
    labels_on_top: Option<bool>,
    /// The social page's labels under its items: only the menu's entry puts them there ([`SOCIAL_PAGE`]).
    social_labels_under: bool,
    wear_popup: Option<crate::gui::popup::PopupMenu>,
    /// The inventory's list mode and order (`Containers/Inventory.xml`'s `listview_mode`, `grid_sort_column` and
    /// `grid_sort_order`; `None`: not auto arranged, each item at the cell it was put in), once read from what is
    /// kept; its columns' widths and list sort; and its window menu while it is up, with the entries' keys.
    inv_list: Option<bool>,
    inv_sort: Option<Option<(i32, i32)>>,
    inv_keeper: multilist::Keeper,
    inv_menu: Option<Vec<Vec<String>>>,
    inv_popup: Option<crate::gui::popup::PopupMenu>,
}

/// What a click on an item of a grid does.
/// An item's link as the chat sends it (the menu's "Create Reference").
pub fn reference(item: &GearItem) -> String {
    format!("<a href=\"itemref://{}/{}/{}\">{}</a>", item.item_id, item.high_id, item.quality, item.name)
}

/// What a right click on an item does with another in hand.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum HeldRightClick {
    /// `UseItemOnItem`: the use message's action 5.
    UseOn,
    /// `N3Msg_TradeskillCombine`: `CharacterAction` 0x51.
    Combine,
    Nothing,
}

/// The right click's meaning by the place of the item in hand, the place clicked and Shift (GUI.dll FUN_100cc855 for
/// the inventory's cells, FUN_100e220f for the wear pages'): see where it is used.
pub fn held_right_click(from: u32, onto: u32, shift: bool) -> HeldRightClick {
    match (from == onto, shift && onto >= BACKPACK_FIRST) {
        (true, _) => HeldRightClick::Nothing,
        (false, false) => HeldRightClick::UseOn,
        (false, true) if from >= BACKPACK_FIRST => HeldRightClick::Combine,
        (false, true) => HeldRightClick::Nothing,
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Click {
    TakeUp,
    Info,
    /// A split of the stack is asked for.
    Split,
    Nothing,
}

/// The box that asks how many of a stack of `count` to split off it (this client's own: the original counts by the
/// mouse's sideways movement, FUN_10041ca4), `name` one of `chrome::DIALOGS`. Nothing while it waits; then the count
/// (`places::split_count`), or none when it was closed, cancelled, or what was typed is no number.
pub fn split_dialog(ctx: &egui::Context, palette: &Palette, name: &'static str, typed: &mut String, count: u32) -> Option<Option<u32>> {
    let mut asking = count > 1;
    let mut answer = None;
    let text = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
    crate::chrome::window(ctx, palette, name, "Split stack", &mut asking, |w| w.resizable(false).pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.content_rect().center()), |ui| {
        ui.label(format!("How many of the {count}?"));
        let field = crate::chrome::text_edit(ui, palette, egui::TextEdit::singleline(typed).desired_width(80.0));
        crate::chrome::focus_when_new(ctx, name, &field);
        if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            answer = Some(true);
        }
        ui.horizontal(|ui| {
            if crate::chrome::button(ui, palette, &text("MsgBox_OK", "OK"), true).clicked() {
                answer = Some(true);
            }
            if crate::chrome::button(ui, palette, &text("MsgBox_Cancel", "Cancel"), true).clicked() {
                answer = Some(false);
            }
        });
    });
    match answer {
        Some(true) => Some(crate::places::split_count(typed, count)),
        Some(false) => Some(None),
        None if !asking => Some(None),
        None => None,
    }
}

/// A click's meaning by its count, the keys held, whether an item is in hand
/// already and whether the item is a stack of the backpack (more than one). The original's lists look at the
/// qualifiers first (FUN_10041aa4: Shift or Ctrl held, nothing is taken up; FUN_10041926: Ctrl on a stack of more
/// than one starts a split).
pub fn click(clicks: u32, shift: bool, ctrl: bool, holding: bool, stack: bool) -> Click {
    match (shift, ctrl) {
        (true, _) => Click::Info,
        (false, true) if clicks == 1 && stack && !holding => Click::Split,
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
        // (FUN_1003dee6: the name, how many (nothing counted reads 1) and the quality; sorted by the cell, the name
        // without its case, and the two numbers, FUN_1003d5f3)
        cells: vec![(1, multilist::CellView::Text(item.name.clone())), (2, multilist::CellView::Text(item.count.max(1).to_string())), (4, multilist::CellView::Text(item.quality.to_string()))],
        keys: vec![
            (0, pos.map_or_else(|| format!("{:08}", item.slot), |(x, y)| format!("{y:04}{x:04}"))),
            (1, item.name.to_lowercase()),
            (2, format!("{:010}", item.count.max(1))),
            (4, format!("{:010}", item.quality)),
        ],
        pos,
        // (an item whose use is locked, a stim while First Aid is: faded for the time left, as a locked action is)
        left: crate::actions::lock_share(crate::actions::item_lock_left(item.item_id)),
        colour: None,
    }
}

/// Puts the items into a grid when they changed.
fn fill(view: &mut GridView, items: &[(&GearItem, Option<(usize, usize)>)], textures: &mut HashMap<u32, egui::TextureHandle>, ctx: &egui::Context, own: bool) {
    let now: Vec<(u32, u16, Option<u32>, Option<(usize, usize)>)> = items.iter().map(|(i, pos)| (i.slot, i.count, i.icon.as_ref().map(|c| c.id), *pos)).collect();
    if view.shown != now {
        let rows = items.iter().map(|(i, pos)| entry(i, textures, ctx, *pos)).collect();
        multilist::set_items(&mut view.tree, view.list, rows);
        view.shown = now;
    }
    // (the items that wait on the city controller's map and the worn ones locked by their menu: the lock's picture on
    // them, FUN_10040bda. Not in the inspect window, whose places are another character's.)
    multilist::set_locked(&mut view.tree, view.list, items.iter().filter(|(i, _)| own && crate::gear::locked(i.slot)).map(|(i, _)| i.slot as i64).collect());
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
    /// The labels over the items or under them, as the wear window's ([`ItemWindows::labels_on_top`]), kept as
    /// `InspectViewConfig`'s own; and the menu while it is up.
    labels_on_top: Option<bool>,
    social_labels_under: bool,
    popup: Option<crate::gui::popup::PopupMenu>,
}

/// The wear pages' fourth, which the kept "labels under the items" does not reach when the window is made: the
/// original sets it on three pages only (FUN_100e24b8 calls `SetGridLabelsOnTop` three times, before the social page
/// is made), while the menu's entry sets all four (FUN_100e1c7b).
const SOCIAL_PAGE: usize = 3;

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
            fill(&mut self.grids[k], &rows, textures, ctx, false);
        }
        // (the labels over the items or under them, kept apart from the wear window's: `labels_on_top` of
        // `InspectViewConfig`, read as the wear window reads its own)
        let labels_on_top = *self.labels_on_top.get_or_insert_with(|| crate::layout::tab("InspectLabelsOnTop") != Some(0));
        for (k, view) in self.grids.iter_mut().enumerate() {
            let on_top = if k == SOCIAL_PAGE { !self.social_labels_under } else { labels_on_top };
            multilist::set_grid(&mut view.tree, view.list, |m| m.labels_on_top = on_top);
        }
        let mut menu_at = None;
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
                if let Some(at) = pointer {
                    if let Some(slot) = multilist::item_at(tree, list, response.rect.min, at) {
                        hovered = Some((slot as u32, cell_label(&WEAR_TABS[t], slot as u32)));
                    } else if response.rect.contains(at) && ui.input(|i| i.pointer.secondary_clicked()) {
                        // (the window's menu, FUN_100e20d6, by the right button beside the items)
                        menu_at = Some(at);
                    }
                }
            });
        });
        if self.tab != tab {
            self.tab = tab;
            crate::layout::set_tab("InspectViewConfig", tab);
        }
        // (the same menu as the wear window's without "Unlock All Items", which is the character's own window's; it is
        // the frame's menu, so the frame's "Close Window" ends it)
        if let Some(at) = menu_at.or_else(|| crate::chrome::frame_menu("Inspect")) {
            self.popup = Some(crate::chrome::menu_marked(ctx, at, &[(crate::gui::xml::ldb_named(10000, "LabelsOnTop", "Labels over the items"), true, Some(labels_on_top)), (crate::chrome::close_entry(), true, None)]));
        }
        match crate::chrome::menu(ctx, palette, "inspect-menu", &mut self.popup) {
            Some(0) => {
                self.labels_on_top = Some(!labels_on_top);
                self.social_labels_under = labels_on_top;
                crate::layout::set_tab("InspectLabelsOnTop", usize::from(!labels_on_top));
            }
            Some(1) => crate::chrome::close_next("Inspect"),
            _ => {}
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
    /// An item's menu as the original's (GUI.dll FUN_100caf07; no entry to use it): look at it, put a link to it into
    /// the chat's line, then for an item in the backpack destroy it, and for a worn one lock it to its place or
    /// unlock it. The original leaves the lock's entry out for an item whose stat 8 (`TimeExist`) is above 0; the
    /// template's is asked here.
    fn item_menu(item: &GearItem) -> Vec<(ItemChoice, String)> {
        let mut menu = vec![(ItemChoice::LookAt, "Look At".to_string()), (ItemChoice::Reference, "Create Reference".to_string())];
        if item.slot >= BACKPACK_FIRST {
            menu.push((ItemChoice::Delete, "Delete".to_string()));
        } else if crate::net::item_template_stat(item.item_id, 8).is_none_or(|v| (v as i32) < 1) {
            let locked = crate::gear::equipment_locked(item.slot);
            let label = if locked { crate::gui::xml::ldb_named(10000, "UnlockItem", "Unlock") } else { crate::gui::xml::ldb_named(10000, "LockItem", "Lock") };
            menu.push((ItemChoice::Lock(!locked), label));
        }
        menu
    }

    /// Asks how many to split off the stack at a backpack place: the box Ctrl+click opens.
    pub fn ask_split(&mut self, place: u32) {
        self.split_box = Some((place, "1".to_string()));
    }

    fn build(&mut self, ctx: &egui::Context) {
        let mut inventory = GridView::new(ctx, PACK_PLACES as i32, 5);
        multilist::set_max_items(&mut inventory.tree, inventory.list, PACK_PLACES as i32);
        // (the list mode's columns, sorted by name; whether the grid is arranged is the frame's matter: one's own
        // container starts not arranged, FUN_100ce55d, each item at the cell it is given, `places.rs`)
        for (id, label, width, flags) in crate::loot::COLUMNS {
            multilist::add_column(&mut inventory.tree, inventory.list, id, label, width, flags);
        }
        multilist::set_sort(&mut inventory.tree, inventory.list, 1, 0);
        self.inv_keeper = Default::default();
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
        // (what is in hand let go: an item, or so many of a stack to be split off it, which only an empty cell of the
        // inventory takes)
        let mut released = crate::gear::hand_opening(ctx);
        let split = crate::gear::split_count();
        let stack = |i: &GearItem| crate::places::Stack { place: i.slot, low: i.item_id, high: i.high_id, quality: i.quality, can: i.can, count: i.count as u32 };
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
        // (the inventory's list mode and order, as kept; `MAO_INV_LIST=1` and `MAO_INV_SORT=<n>`, the kept number, for
        // a picture)
        let in_list = *self.inv_list.get_or_insert_with(|| std::env::var_os("MAO_INV_LIST").is_some() || crate::layout::tab("InventoryList") == Some(1));
        let arranged = *self.inv_sort.get_or_insert_with(|| match std::env::var("MAO_INV_SORT").ok().and_then(|v| v.parse().ok()).or_else(|| crate::layout::tab("InventoryGridSort")) {
            Some(n) if n > 0 => Some((((n - 1) / 2) as i32, ((n - 1) % 2) as i32)),
            _ => None,
        });
        // (placed by hand: the grid while it is not arranged)
        let hand = arranged.is_none() && !in_list;
        let mut inv_menu_at = None;
        if *open && let Some(view) = self.inventory.as_mut() {
            multilist::set_list_mode(&mut view.tree, view.list, in_list);
            if !in_list {
                multilist::set_grid(&mut view.tree, view.list, |m| m.auto_arrange = !hand);
            }
            self.inv_keeper.before("InventoryConfig", &mut view.tree, view.list, (!in_list).then(|| arranged.unwrap_or((0, 0))));
            multilist::update(&mut view.tree, view.list);
            // (each item at its own cell, counted along the rows the window's width makes: the one it was put in by
            // hand, the one awaited for it, else the first free one; before the items have come nothing is forgotten)
            let columns = multilist::grid_columns(&view.tree, view.list).unwrap_or(5).max(1);
            let places: Vec<u32> = pack.iter().map(|i| i.slot).collect();
            let cells = if items.is_empty() { crate::places::Cells::new() } else { crate::places::arrange(crate::places::INVENTORY, &places, PACK_PLACES as usize, self.awaited.map(|a| (a.0, a.1))) };
            if self.awaited.is_some_and(|(place, _, since)| by_slot.contains_key(&place) || since.elapsed() > std::time::Duration::from_secs(5)) {
                self.awaited = None;
            }
            let mut hovered_cell: Option<usize> = None;
            let rows: Vec<(&GearItem, Option<(usize, usize)>)> = pack.iter().map(|i| (*i, cells.get(&i.slot).map(|c| (c % columns, c / columns)))).collect();
            fill(view, &rows, textures, ctx, true);
            let (w, h) = multilist::grid::size(PACK_PLACES as usize, 5);
            let (tree, root, list) = (&mut view.tree, view.root, view.list);
            let shown = crate::chrome::window(ctx, palette, "Inventory", "Inventory", &mut is_open, |w2| w2.default_pos(crate::layout::pos("Inventory", egui::pos2(40.0, 70.0))).default_size(egui::vec2(w + 32.0, h.min(4.0 * multilist::grid::STEP + 12.0))).resizable(true), |ui| {
                let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
                if let Some(at) = pointer
                    && let Some(slot) = multilist::item_at(tree, list, response.rect.min, at)
                {
                    hovered_item = Some((slot as u32, String::new()));
                }
                // (the cell under the pointer, with an item or not: where what is in hand is let go)
                hovered_cell = pointer.and_then(|at| multilist::cell_at(tree, list, response.rect.min, at)).map(|(x, y)| y * columns + x).filter(|c| hand && *c < PACK_PLACES as usize);
                // (the window's menu, FUN_100cbace: the right button beside the items; the original's is the frame's
                // menu button)
                if hovered_item.is_none()
                    && dragged.is_none()
                    && ui.input(|i| i.pointer.secondary_clicked())
                    && let Some(at) = pointer.filter(|at| response.rect.contains(*at))
                {
                    inv_menu_at = Some(at);
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
                    // (and the empty cells in view, for a script's placing by hand)
                    let free: Vec<String> = (0..PACK_PLACES as usize)
                        .filter(|c| crate::places::at(&cells, *c).is_none())
                        .filter_map(|c| multilist::cell_rect(tree, list, response.rect.min, (c % columns, c / columns)).map(|r| (c, r.center())))
                        .filter(|(_, at)| rect.contains(*at))
                        .map(|(c, at)| format!("{c} at {:.0};{:.0}", at.x * k, at.y * k))
                        .collect();
                    let id = egui::Id::unique("item-trace-free-cells");
                    if ui.ctx().data(|d| d.get_temp::<Vec<String>>(id)).as_ref() != Some(&free) {
                        eprintln!("items: the inventory's empty cells: {free:?}");
                        ui.ctx().data_mut(|d| d.insert_temp(id, free));
                    }
                }
                response
            });
            // (what is in hand let go over the inventory, FUN_100cc855)
            if let (Some(from), Some(shown)) = (dragged, &shown)
                && released
                && shown.response.contains_pointer()
            {
                let free = (BACKPACK_FIRST..BACKPACK_FIRST + PACK_PLACES).find(|p| !by_slot.contains_key(p));
                // (in list mode and in an arranged grid there are no cells to put an item in: what it is let go on is
                // the item under the pointer; stacks still join and a split's new stack takes the first free place,
                // FUN_100cc855's `GetLayoutMode() != 1` branches and `MoveItem` under auto arrange)
                let onto: Option<&GearItem> = if hand {
                    hovered_cell.and_then(|c| crate::places::at(&cells, c)).and_then(|p| by_slot.get(&p).copied())
                } else {
                    hovered_item.as_ref().map(|h| h.0).filter(|p| *p != from).and_then(|p| by_slot.get(&p).copied())
                };
                if from < BACKPACK_FIRST {
                    // (an item from the wear window goes back into the backpack: the first free place, shown in the
                    // cell it was let go on when that is empty)
                    if let Some(free) = free {
                        events.moves.push((from, free));
                        if let (Some(cell), None) = (hovered_cell, onto) {
                            self.awaited = Some((free, cell, std::time::Instant::now()));
                        }
                    }
                } else if onto.is_some_and(|o| crate::gear::locked(o.slot)) {
                    // (let go on an item that waits on the city's map: told, and nothing done, FUN_100cc855)
                    events.feedback.push("DroppedItemWasLocked");
                } else if let (Some(cell), Some(held)) = (hovered_cell.or((!hand).then_some(0)), by_slot.get(&from).copied()) {
                    use crate::places::Placing;
                    match (crate::places::placing(&stack(held), split, onto.map(stack).as_ref()), onto) {
                        // (an empty cell takes the item: `MultiListView_c::MoveItem`, the client's own matter)
                        (Placing::Move, _) if hand => {
                            crate::places::put(crate::places::INVENTORY, from, cell);
                            if crate::gear::traced() {
                                eprintln!("items: place {from} put into cell {cell}");
                            }
                        }
                        // (two stacks of a kind: the one in hand goes into the other, `N3Msg_JoinItems(the other, the held)`)
                        (Placing::Join, Some(other)) => {
                            events.joins.push((other.slot, from));
                            if crate::gear::traced() {
                                eprintln!("items: place {from} joined to the stack at place {}", other.slot);
                            }
                        }
                        // (anything else: the two change cells and the other is in hand, taken up as by a click
                        // (FUN_100cc6e8: a bag's window closes); stacks of one kind and another level are told so)
                        (Placing::Swap, Some(other)) if hand => {
                            if crate::places::other_level(&stack(held), &stack(other)) {
                                events.feedback.push("Feedback_CanOnlyStackItemsOfSameLevel");
                            }
                            crate::places::exchange(crate::places::INVENTORY, from, other.slot);
                            if crate::gear::traced() {
                                eprintln!("items: places {from} and {} change cells: place {from} into cell {cell}, the other in hand", other.slot);
                            }
                            crate::gear::pick_up(other.slot);
                            dragged = Some(other.slot);
                            released = false;
                        }
                        // (so many of the stack become a stack of their own, shown in this cell: `N3Msg_SplitItem`,
                        // which looks first whether the item may be split and a place is free)
                        (Placing::Split, _) => match crate::places::split_refusal(&stack(held), split, free.is_some()) {
                            Some(why) => events.feedback.extend(why),
                            None => {
                                events.splits.push((from, split));
                                self.awaited = free.filter(|_| hand).map(|place| (place, cell, std::time::Instant::now()));
                                if crate::gear::traced() {
                                    eprintln!("items: {split} of place {from} split off into cell {cell}");
                                }
                            }
                        },
                        // (let go on itself, or a split on an item: back in place)
                        _ => {}
                    }
                }
            }
            Self::events_of(&mut view.tree, view.list, &mut events, items, dragged, &mut self.split_box);
            self.inv_keeper.after("InventoryConfig", &view.tree, view.list, in_list);
            // (FUN_100cbace: List Mode with its mark, and Auto Arrange with its sub-menu, dead in list mode)
            if let Some(at) = inv_menu_at.or_else(|| crate::chrome::frame_menu("Inventory")) {
                use crate::gui::popup::Entry;
                let text = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
                let (arrange_keys, arrange) = crate::loot::arrange_menu(arranged, crate::loot::Arrange::Own);
                self.inv_menu = Some(vec![vec!["list".to_string()], arrange_keys, vec!["close".to_string()]]);
                self.inv_popup = Some(crate::chrome::menu_of(ctx, at, vec![Entry::marked(text("ListMode", "As a list"), true, in_list), Entry::submenu(text("AutoArrange", "Order"), !in_list, arrange), Entry::text(crate::chrome::close_entry(), true)]));
            }
            if let Some(crate::chrome::Picked::Entry(path)) = crate::chrome::menu_path(ctx, palette, "inventory-menu", &mut self.inv_popup)
                && let Some(keys) = self.inv_menu.take()
                && let Some(chosen) = path.first().and_then(|k| keys.get(*k)).and_then(|k| k.get(path.get(1).copied().unwrap_or(0)))
            {
                if chosen == "list" {
                    self.inv_list = Some(!in_list);
                    crate::layout::set_tab("InventoryList", usize::from(!in_list));
                } else if chosen == "close" {
                    crate::chrome::close_next("Inventory");
                } else if let Some(new) = crate::loot::arrange_choice(chosen, arranged) {
                    // (`SetAutoArrange(false)`: each item stays in the cell the arranging had it in)
                    if new.is_none() {
                        for (cell, id) in multilist::order(&view.tree, view.list).into_iter().enumerate() {
                            crate::places::put(crate::places::INVENTORY, id as u32, cell);
                        }
                    }
                    self.inv_sort = Some(new);
                    crate::layout::set_tab("InventoryGridSort", new.map_or(0, |s| 1 + (s.0 * 2 + s.1) as usize));
                }
            }
            if self.inv_popup.is_none() {
                self.inv_menu = None;
            }
        }
        *open = is_open;

        // The wear window.
        if self.wear_open {
            let mut wear_open = true;
            let names: Vec<String> = WEAR_TABS.iter().map(|t| crate::gui::xml::ldb_text(t.name)).collect();
            let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            for (k, tab) in WEAR_TABS.iter().enumerate() {
                let rows: Vec<(&GearItem, Option<(usize, usize)>)> = tab.cells.iter().filter_map(|&(slot, x, y)| by_slot.get(&(tab.base + slot)).map(|i| (*i, Some((x, y))))).collect();
                fill(&mut self.wear[k], &rows, textures, ctx, true);
            }
            let mut tab = self.tab.min(WEAR_TABS.len() - 1);
            let mut drop_on: Option<u32> = None;
            // (`SetGridLabelsOnTop`: what is kept on the first three pages, the menu's choice on all four, FUN_100e24b8
            // and FUN_100e1c7b; `MAO_WEAR_LABELS=0`: under the items, for a picture)
            let labels_on_top = *self.labels_on_top.get_or_insert_with(|| std::env::var("MAO_WEAR_LABELS").as_deref() != Ok("0") && crate::layout::tab("WearLabelsOnTop") != Some(0));
            for (k, view) in self.wear.iter_mut().enumerate() {
                let on_top = if k == SOCIAL_PAGE { !self.social_labels_under } else { labels_on_top };
                multilist::set_grid(&mut view.tree, view.list, |m| m.labels_on_top = on_top);
            }
            let mut wear_menu_at = None;
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
                        } else if response.rect.contains(at) && dragged.is_none() && ui.input(|i| i.pointer.secondary_clicked()) {
                            // (the window's menu, FUN_100e20d6: the right button beside the items; the original's is the
                            // frame's menu button)
                            wear_menu_at = Some(at);
                        }
                        // (an item let go over a wear cell is worn there; not a split, FUN_100e220f)
                        if dragged.is_some() && released && split == 0 && let Some(cell) = multilist::cell_at(tree, list, response.rect.min, at) {
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
                Self::events_of(&mut view.tree, view.list, &mut events, items, dragged, &mut self.split_box);
            }
            // ("Labels On Top" with its mark, then "Unlock All Items", never greyed: it takes the lock off every wear
            // place, FUN_100e1f9b)
            if let Some(at) = wear_menu_at.or_else(|| crate::chrome::frame_menu("Wear")) {
                self.wear_popup = Some(crate::chrome::menu_marked(ctx, at, &[(crate::gui::xml::ldb_named(10000, "LabelsOnTop", "Labels over the items"), true, Some(labels_on_top)), (crate::gui::xml::ldb_named(10000, "UnlockAllItems", "Unlock every item"), true, None), (crate::chrome::close_entry(), true, None)]));
            }
            match crate::chrome::menu(ctx, palette, "wear-menu", &mut self.wear_popup) {
                Some(0) => {
                    self.labels_on_top = Some(!labels_on_top);
                    self.social_labels_under = labels_on_top;
                    crate::layout::set_tab("WearLabelsOnTop", usize::from(!labels_on_top));
                }
                Some(1) => crate::gear::unlock_all_equipment(),
                Some(2) => crate::chrome::close_next("Wear"),
                _ => {}
            }
            if !wear_open {
                self.wear_open = false;
            }
        }

        // (a right click uses the item, as the original's list does (`ItemListViewBase_c`'s FUN_1004141b: button 2 -> the
        // use signal); with an item in hand, that item is used on the one clicked (`UseItemOnItem`: the one in hand is
        // the source, so the order of the two matters) and goes back to its place; Shift+right click opens the client's
        // own menu where it was clicked)
        // (the right button held on an item for a moment opens its menu, and letting go then is not a use; the user's word
        // for the original's: "long right click on item should give a menu with look at item, delete and create reference")
        // (`MouseButtonHoldDelay`, 0.3 s: the original's list raises its menu signal then, FUN_10041c04)
        const HOLD: std::time::Duration = std::time::Duration::from_millis(300);
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
                let entries: Vec<(String, bool)> = by_slot.get(&held).map(|item| Self::item_menu(item)).unwrap_or_default().into_iter().map(|(_, label)| (label, true)).collect();
                self.popup = Some(crate::chrome::menu_at(ctx, at, &entries));
                self.confirm = None;
                self.right_held = None;
            }
        }
        if let (Some((slot, _)), Some(_)) = (&hovered_item, pointer)
            && self.menu.is_none()
            && ctx.input(|i| i.pointer.secondary_clicked())
        {
            if let Some(from) = dragged {
                if split > 0 {
                    // (a split is used on nothing: the right button ends it, FUN_100cc855)
                    crate::gear::put_down();
                } else if crate::gear::locked(*slot) {
                    events.feedback.push("DroppedItemWasLocked");
                    crate::gear::put_down();
                } else {
                    // (FUN_100cc855's right button on a cell with an item: without Shift the item in hand is used
                    // on it, `UseItemOnItem`; with Shift, in the inventory and both of it, the two are combined,
                    // `N3Msg_TradeskillCombine`, and anywhere else the drag ends. A worn item is used on whatever the
                    // keys, FUN_100e220f. Either way the hand is empty after. Neither is sent for an item on
                    // itself: Gamecode 0x1002695c and 0x10028707 test that.)
                    match held_right_click(from, *slot, ctx.input(|i| i.modifiers.shift)) {
                        HeldRightClick::UseOn => events.use_ons.push((from, *slot)),
                        HeldRightClick::Combine => events.combines.push((from, *slot)),
                        HeldRightClick::Nothing => {}
                    }
                    crate::gear::put_down();
                }
            } else if !crate::gear::locked(*slot) {
                // (with Shift too: the original's menu comes by the held button only. A locked item, one that waits
                // on the city's map or a worn one locked by its menu, is not used: FUN_100cae1b.)
                // (with Ctrl while a trade's window is up the item goes into the trade instead: the same function's
                // `N3Msg_TradeAddItem` for the qualifiers 0xc)
                if crate::shop::trading() && ctx.input(|i| i.modifiers.ctrl) {
                    events.trade_adds.push(*slot);
                } else {
                    events.uses.push(*slot);
                }
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
            && split == 0
            && !egui::DragAndDrop::has_any_payload(ctx)
        {
            egui::DragAndDrop::set_payload(ctx, crate::hotbar::Action::Item { id: item.item_id, name: item.name.clone(), icon: item.icon.as_ref().map(|i| i.id) });
        }
        // (the item in hand follows the pointer)
        if let (Some(from), Some(at)) = (dragged, pointer)
            && let Some(icon) = by_slot.get(&from).and_then(|i| i.icon.as_ref()).and_then(|icon| textures.get(&icon.id))
        {
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("dragged-item")));
            painter.image(icon.id(), egui::Rect::from_center_size(at, egui::vec2(40.0, 40.0)), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::from_white_alpha(200));
            // (a split in hand: how many, in the stack count's yellow at the icon's top right)
            if crate::gear::split_count() > 0 {
                painter.text(at + egui::vec2(20.0, -20.0), egui::Align2::RIGHT_TOP, crate::gear::split_count().to_string(), egui::FontId::proportional(12.0), egui::Color32::YELLOW);
            }
        }
        // (an item's menu: the client's own entries, in the original's popup; deleting asks first)
        if let Some(k) = crate::chrome::menu(ctx, palette, "item-menu", &mut self.popup)
            && let Some((slot, _)) = self.menu
            && let Some(item) = by_slot.get(&slot)
        {
            match Self::item_menu(item).get(k).map(|(choice, _)| *choice) {
                Some(ItemChoice::LookAt) => events.infos.push(format!("itemref://{}/{}/{}", item.item_id, item.high_id, item.quality)),
                Some(ItemChoice::Reference) => {
                    if crate::gear::traced() {
                        eprintln!("items: a reference to place {slot}: {}", reference(item));
                    }
                    events.references.push(reference(item));
                }
                Some(ItemChoice::Delete) => self.confirm = Some(slot),
                Some(ItemChoice::Lock(on)) => crate::gear::set_equipment_lock(slot, on),
                None => {}
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
        // (a split asked for: how many of the stack; OK has that many in hand, to be put into an empty cell)
        if let Some((place, typed)) = self.split_box.as_mut() {
            let place = *place;
            let count = by_slot.get(&place).map_or(0, |i| i.count as u32);
            if let Some(answer) = split_dialog(ctx, palette, "split-stack", typed, count) {
                if let Some(n) = answer {
                    crate::gear::pick_up_split(place, n);
                }
                self.split_box = None;
            }
        }
        events
    }

    /// A grid's clicks: a click takes the item up (with one in hand already, `held`, the click is the one that puts
    /// that down); Shift+click shows its info; Ctrl+click on a stack of the backpack asks how many to split off it
    /// (`split_box`). A press that moves to another cell is nothing (the list tells of none).
    fn events_of(tree: &mut Tree, list: Id, events: &mut Events, items: &[GearItem], held: Option<u32>, split_box: &mut Option<(u32, String)>) {
        let Some(ctx) = tree.measure.ctx.clone() else { return };
        let (ctrl, shift) = ctx.input(|i| (i.modifiers.ctrl, i.modifiers.shift));
        multilist::click_only(tree, list);
        for (_, event) in multilist::take_events(tree) {
            let multilist::ItemEvent::Clicked(slot, clicks) = event else { continue };
            let Some(item) = items.iter().find(|i| i.slot as i64 == slot) else { continue };
            if clicks > 0 {
                multilist::select(tree, list, Some(slot));
            }
            // (said in the game's log: a Ctrl+click that did not split says what it saw)
            if clicks > 0 && ctx.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                eprintln!("items: Ctrl+click on place {} ({:?}): count {}, {} clicks, ctrl {ctrl}, shift {shift}, in hand {:?}", item.slot, item.name, item.count, clicks, held);
            }
            match click(clicks, shift, ctrl, held.is_some(), item.slot >= BACKPACK_FIRST && item.count > 1) {
                // (an item that waits on the city controller's map is locked: not taken up, split or traded,
                // FUN_100cc6e8 and FUN_100cae1b)
                Click::Split | Click::TakeUp if crate::gear::locked(item.slot) => {}
                Click::Info => events.infos.push(format!("itemref://{}/{}/{}", item.item_id, item.high_id, item.quality)),
                Click::Split => {
                    if crate::gear::traced() {
                        eprintln!("items: a split of place {} asked for", item.slot);
                    }
                    // (the same box as `ask_split`'s)
                    *split_box = Some((item.slot, "1".to_string()));
                }
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

    /// FUN_100cc855 and FUN_100e220f: a plain right click uses the item in hand on the other, Shift in the
    /// inventory combines two of its items, and nothing comes of an item on itself.
    #[test]
    fn a_right_click_with_an_item_in_hand() {
        let (pack, other, worn) = (BACKPACK_FIRST + 2, BACKPACK_FIRST + 9, 6);
        assert_eq!(held_right_click(pack, other, false), HeldRightClick::UseOn);
        assert_eq!(held_right_click(pack, worn, false), HeldRightClick::UseOn);
        assert_eq!(held_right_click(worn, pack, false), HeldRightClick::UseOn);
        assert_eq!(held_right_click(pack, other, true), HeldRightClick::Combine);
        // (Shift on a wear page's cell is still the use: its own handler looks at no key)
        assert_eq!(held_right_click(pack, worn, true), HeldRightClick::UseOn);
        // (a worn item in hand on one of the inventory with Shift: not both of the inventory, the drag ends)
        assert_eq!(held_right_click(worn, pack, true), HeldRightClick::Nothing);
        assert_eq!(held_right_click(pack, pack, false), HeldRightClick::Nothing);
        assert_eq!(held_right_click(pack, pack, true), HeldRightClick::Nothing);
    }

    #[test]
    fn a_click_takes_up_shows_or_trades() {
        // (a plain click takes the item up; not with one in hand already)
        assert_eq!(click(1, false, false, false, false), Click::TakeUp);
        assert_eq!(click(1, false, false, true, false), Click::Nothing);
        assert_eq!(click(2, false, false, true, false), Click::Nothing);
        // (a stack is taken up whole by a plain click)
        assert_eq!(click(1, false, false, false, true), Click::TakeUp);
        // (Shift: the item's info, whatever is in hand)
        assert_eq!(click(1, true, false, false, false), Click::Info);
        assert_eq!(click(1, true, false, true, false), Click::Info);
        assert_eq!(click(1, true, true, false, true), Click::Info);
        // (Ctrl: nothing is taken up with it held, and a second click does no more: the trade takes an item by
        // Ctrl + right click, FUN_100cae1b)
        assert_eq!(click(1, false, true, false, false), Click::Nothing);
        assert_eq!(click(2, false, true, false, false), Click::Nothing);
        // (Ctrl+click on a stack asks for a split; not with something in hand, not by a drag)
        assert_eq!(click(1, false, true, false, true), Click::Split);
        assert_eq!(click(1, false, true, true, true), Click::Nothing);
        assert_eq!(click(2, false, true, false, true), Click::Nothing);
    }
}
