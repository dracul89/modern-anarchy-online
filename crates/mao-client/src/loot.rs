//! The loot window: what a corpse or a bag holds, as the server's `InventoryUpdate` lists it for a container the character used.
//! GUI.dll's `InventoryView_c` (FUN_100ccefe) for a container that is not one's own: an icon view of the items (its
//! `item_position_map` places them three to a row, 21 places), titled with the container's name; a corpse or chest that
//! is empty closes a second later (its `EventTimer_c` started for 1 000 000 µs). An item is taken by a double click (or
//! its menu), or all of them at once.
//!
//! One's own backpack (`N3Msg_IsItemMine`: a bag in the inventory) has a window of its own in the original, as many
//! open at once as there are bags opened: named `Container_<type>x<instance>` (so each keeps its place), 21 places
//! three to a row, titled with the bag's name, which the window's menu (FUN_100cbace) can change ("Rename":
//! FUN_100cb852's "Enter backpack name." box with OK, "Reset to default" and Cancel; the original keeps the name in
//! the bag's own file, `Containers/Container_<type>x<instance>.xml`, here in `backpacks.ini` beside `mao.ini`). It
//! does not close when empty and has no "Take all". Not ported for these windows: the menu's List Mode and Auto
//! Arrange, items placed by hand (`item_position_map`), closing by Escape (`esc_backpacks`), and telling the server
//! that a bag was closed (the original's message was not identified).

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use crate::gear::GearItem;
use crate::gui::view::{Id, Layout, Node, Tree};
use crate::theme::Palette;

/// A container's contents, ready to show.
pub struct LootView {
    pub title: String,
    /// The container's identity (type, instance): what an item put in names it by.
    pub identity: (u32, u32),
    /// The container's handle: what a take message names it by.
    pub handle: u32,
    /// The items; `slot` is the item's place in the container.
    pub items: Vec<GearItem>,
    /// One's own backpack (a bag in the inventory): a window of its own.
    pub own: bool,
}

/// The identity type of a bag or chest.
pub const BAG: u32 = 0xc749;

/// The places of one's own backpack (`ItemContainerView_c` FUN_100ce55d: `SetMaxItemCount(21)`).
pub const BAG_PLACES: usize = 21;

/// The names the player gave their backpacks, by the bag's identity (`InventoryGUIModule_c::SetBackpackName`).
static BAG_NAMES: Mutex<Option<BTreeMap<(u32, u32), String>>> = Mutex::new(None);

fn names_path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("backpacks.ini")
}

/// Whether the names are kept on disk (not in tests and previews).
fn names_kept() -> bool {
    !cfg!(test) && std::env::var_os("MAO_NO_LAYOUT").is_none() && std::env::var_os("MAO_UI_PREVIEW").is_none()
}

/// `backpacks.ini`: a line a bag, `<type> <instance> <name>`.
pub fn parse_names(text: &str) -> BTreeMap<(u32, u32), String> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.splitn(3, ' ');
            let identity = (words.next()?.parse().ok()?, words.next()?.parse().ok()?);
            let name = words.next()?.trim();
            (!name.is_empty()).then(|| (identity, name.to_string()))
        })
        .collect()
}

pub fn render_names(names: &BTreeMap<(u32, u32), String>) -> String {
    names.iter().map(|((kind, instance), name)| format!("{kind} {instance} {name}\n")).collect()
}

fn with_names<R>(f: impl FnOnce(&mut BTreeMap<(u32, u32), String>) -> R) -> R {
    let mut names = BAG_NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let names = names.get_or_insert_with(|| if names_kept() { std::fs::read_to_string(names_path()).map(|t| parse_names(&t)).unwrap_or_default() } else { BTreeMap::new() });
    f(names)
}

/// The name the player gave a backpack, if any.
pub fn bag_name(identity: (u32, u32)) -> Option<String> {
    with_names(|names| names.get(&identity).cloned())
}

/// Names a backpack, or with `None` gives it its own name back ("Reset to default").
pub fn set_bag_name(identity: (u32, u32), name: Option<&str>) {
    with_names(|names| {
        match name.map(str::trim).filter(|n| !n.is_empty()) {
            // (a name is one line: the file keeps a bag a line)
            Some(name) => names.insert(identity, name.replace(['\r', '\n'], " ")),
            None => names.remove(&identity),
        };
        if names_kept() {
            let _ = std::fs::write(names_path(), render_names(names));
        }
    });
}

/// A container window's name: one's own backpack's by its identity (the original's `Container_%ux%u`), the others'
/// the one shared window.
pub fn window_name(view: &LootView) -> String {
    if view.own { format!("Container_{}x{}", view.identity.0, view.identity.1) } else { "loot".to_string() }
}

/// The identity type of a corpse.
pub const CORPSE: u32 = 0xc76a;
/// The identity type of the bank (`BankIIR_t`'s `BankEntry_t`, FUN_10125b1b).
pub const BANK: u32 = 0xdead;

/// What the window asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Take {
    One { handle: u32, place: u32 },
    All { handle: u32 },
    /// An inventory item (its place) dropped on the window: put it in the container.
    Put { identity: (u32, u32), from: u32 },
}

#[derive(Default)]
pub struct LootWindow {
    pub open: bool,
    /// The icons' textures (the handles kept: a dropped handle frees its texture, and the icon draws as nothing).
    textures: HashMap<u32, egui::TextureHandle>,
    /// Which container (identity, handle) has been empty since when (it closes a second later).
    empty_since: Option<((u32, u32), u32, std::time::Instant)>,
    /// The window's views (the `ItemContainerView_c`'s grid), for the container they were built for.
    views: Option<(Tree, Id, Id, (u32, u32))>,
    /// The items last put in the grid (their places, counts and icons).
    shown: Vec<(u32, u16, Option<u32>)>,
    /// The place of the item being dragged out of the grid.
    dragging: Option<u32>,
    /// The place of an item let go over the world (not over a window): to be used on what is there.
    pub dropped_out: Option<u32>,
    /// The handle the window was last shown for: a backpack opened again comes with a new one and shows again.
    pub handle_seen: Option<u32>,
    /// A backpack's menu (`PopupMenu_c`'s look) and the name being typed in its rename box.
    popup: Option<crate::gui::popup::PopupMenu>,
    renaming: Option<String>,
}

/// The most items a container's grid shows slots for (`ItemContainerView_c` FUN_100ce55d: `SetMaxItemCount` 102 for the bank,
/// 21 for one's own backpack, none for a corpse or chest).
impl LootWindow {
    /// The identity type of the container shown.
    pub fn kind(&self) -> Option<u32> {
        self.views.as_ref().map(|v| v.3.0)
    }
}

/// The identity type of an organization: its contract is a container of that identity (`/org contract`; GUI.dll's
/// `ItemContainerView_c` FUN_100ce55d gives it 100 places and sorts it as it does the bank).
pub const ORGANIZATION: u32 = 0xdeaa;

pub fn max_items(kind: u32, own: bool) -> i32 {
    match kind {
        BANK => 0x66,
        ORGANIZATION => 100,
        _ if own => BAG_PLACES as i32,
        _ => -1,
    }
}

/// The columns the window opens with: the bank's eight, three for the rest (`InventoryView_c`'s position map: three to a
/// row).
fn columns(kind: u32) -> usize {
    if kind == BANK { 8 } else { 3 }
}

impl LootWindow {
    /// `InventoryView_c` (FUN_100ccefe) for a container: its `ItemContainerView_c` (FUN_100ce55d) of a grid-mode
    /// `MultiListView_c` (`gui::multilist`, feature flags 0x15) over the whole window.
    fn build(&mut self, ctx: &egui::Context, view: &LootView) {
        let mut tree = Tree::default();
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
        tree.get_mut(root).layout = Layout::Base;
        let list = crate::gui::multilist::create_grid(&mut tree, "ItemContainerView", max_items(view.identity.0, view.own), columns(view.identity.0));
        tree.add_child(root, list);
        self.views = Some((tree, root, list, view.identity));
        self.shown.clear();
    }

    pub fn show(&mut self, ctx: &egui::Context, view: &LootView, palette: &Palette) -> Option<Take> {
        let mut take = None;
        let mut open = self.open;
        // (FUN_100ccefe: an empty corpse or chest closes after a second; the bank and one's own backpack stay)
        if view.items.is_empty() && !matches!(view.identity.0, BANK | ORGANIZATION) && !view.own {
            if self.empty_since.is_none_or(|e| (e.0, e.1) != (view.identity, view.handle)) {
                self.empty_since = Some((view.identity, view.handle, std::time::Instant::now()));
            }
            if self.empty_since.is_some_and(|e| e.2.elapsed() >= std::time::Duration::from_secs(1)) {
                self.open = false;
                return None;
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        } else {
            self.empty_since = None;
        }
        if self.views.as_ref().is_none_or(|v| v.3 != view.identity) {
            self.build(ctx, view);
        }
        // (the items in the grid in their places' order, as `AddItem` puts them; auto arranged)
        let mut items: Vec<&GearItem> = view.items.iter().collect();
        items.sort_by_key(|i| i.slot);
        let now: Vec<(u32, u16, Option<u32>)> = items.iter().map(|i| (i.slot, i.count, i.icon.as_ref().map(|c| c.id))).collect();
        let textures = &mut self.textures;
        let (tree, root, list, _) = self.views.as_mut()?;
        let (root, list) = (*root, *list);
        if self.shown != now {
            let rows: Vec<crate::gui::multilist::Item> = items
                .iter()
                .map(|i| crate::gui::multilist::Item {
                    id: i.slot as i64,
                    icon: i.icon.as_ref().map(|icon| textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("loot-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id()),
                    count: i.count as u32,
                    cells: vec![(1, crate::gui::multilist::CellView::Text(i.name.clone()))],
                    keys: vec![(1, i.name.clone())],
                    pos: None,
                    left: None,
                    colour: None,
                })
                .collect();
            crate::gui::multilist::set_items(tree, list, rows);
            self.shown = now;
        }
        crate::gui::multilist::update(tree, list);
        let skin = crate::skin::skin(ctx);
        let cols = columns(view.identity.0);
        // (room for the columns and the scroll bar, and up to six rows: the bank's, or as many as the items fill)
        // (one's own backpack: its 21 places)
        let rows_shown = if view.identity.0 == BANK { 6 } else if view.own { BAG_PLACES / cols } else { view.items.len().div_ceil(cols).clamp(1, 6) };
        let size = egui::vec2(cols as f32 * crate::gui::multilist::grid::STEP + 2.0 * crate::gui::multilist::grid::BORDER + 32.0, crate::gui::multilist::grid::size(rows_shown * cols, cols).1 + if view.identity.0 == BANK || view.own { 0.0 } else { 26.0 });
        // (a backpack's window by its identity, titled with the name the player gave it or the bag's own)
        let name = window_name(view);
        let title = if view.own { bag_name(view.identity).unwrap_or_else(|| view.title.clone()) } else { view.title.clone() };
        let at = if view.own { egui::pos2(420.0 + (view.identity.1 % 5) as f32 * 30.0, 160.0 + (view.identity.1 % 4) as f32 * 30.0) } else { egui::pos2(380.0, 140.0) };
        let mut menu_at = None;
        let tip = |item: &GearItem| {
            // (an item's tip: its name, quality and description)
            let mut tip = item.name.clone();
            if item.quality > 1 {
                tip.push_str(&format!("\nQL {}", item.quality));
            }
            if !item.description.is_empty() {
                tip.push_str("\n\n");
                tip.push_str(&item.description.chars().take(300).collect::<String>());
            }
            tip
        };
        let shown = crate::chrome::window(ctx, palette, &name, &title, &mut open, |w| w.default_pos(crate::layout::pos(&name, at)).default_size(size).resizable(true), |ui| {
            // (the views over the window less the room of the "Take all" row under them)
            // (laid out from the bottom: the button first, the views over what is left, so the window does not grow)
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                if !view.items.is_empty() && !matches!(view.identity.0, BANK | ORGANIZATION) && !view.own && crate::chrome::button(ui, palette, "Take all", true).clicked() {
                    take = Some(Take::All { handle: view.handle });
                }
                let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
                let hovered = ui.input(|i| i.pointer.hover_pos()).and_then(|at| crate::gui::multilist::item_at(tree, list, response.rect.min, at));
                // (a right click takes the item: the list's use signal, FUN_1004141b's button 2)
                if let Some(slot) = hovered
                    && ui.input(|i| i.pointer.secondary_clicked())
                {
                    take = Some(Take::One { handle: view.handle, place: slot as u32 });
                }
                // (a backpack's own menu, FUN_100cbace: the right button beside the items)
                if view.own
                    && hovered.is_none()
                    && ui.input(|i| i.pointer.secondary_clicked())
                    && let Some(at) = ui.input(|i| i.pointer.interact_pos()).filter(|at| response.rect.contains(*at))
                {
                    menu_at = Some(at);
                }
                if let Some(item) = hovered.and_then(|slot| view.items.iter().find(|i| i.slot as i64 == slot)) {
                    response.on_hover_text(tip(item));
                }
            });
        });
        // (a click selects; a double click also takes the item)
        for (_, event) in crate::gui::multilist::take_events(tree) {
            match event {
                crate::gui::multilist::ItemEvent::Clicked(slot, clicks) => {
                    crate::gui::multilist::select(tree, list, Some(slot));
                    if clicks > 1 {
                        take = Some(Take::One { handle: view.handle, place: slot as u32 });
                    }
                }
                // (dragged out: let go over the inventory, it is taken)
                crate::gui::multilist::ItemEvent::Dragged(slot) => self.dragging = Some(slot as u32),
            }
        }
        tree.signals.clear();
        // (the item being dragged follows the pointer; let go over the inventory window, it is taken into the backpack)
        if let Some(place) = self.dragging {
            let pointer = ctx.input(|i| i.pointer.hover_pos());
            if let (Some(at), Some(icon)) = (pointer, view.items.iter().find(|i| i.slot == place).and_then(|i| i.icon.as_ref()).and_then(|icon| self.textures.get(&icon.id))) {
                ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("dragged-loot"))).image(icon.id(), egui::Rect::from_center_size(at, egui::vec2(40.0, 40.0)), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::from_white_alpha(200));
            }
            if ctx.input(|i| i.pointer.any_released()) {
                self.dragging = None;
                if !ctx.is_pointer_over_egui() {
                    self.dropped_out = Some(place);
                }
                let inventory = ctx.memory(|m| m.area_rect(egui::Id::unique("Inventory")));
                if let (Some(at), Some(r)) = (pointer, inventory)
                    && r.contains(at)
                {
                    take = Some(Take::One { handle: view.handle, place });
                }
            }
        }
        // An inventory item dropped on the window goes in (not into a corpse).
        if let Some(shown) = shown {
            let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
            if dragged != u32::MAX && view.identity.0 != CORPSE && ctx.input(|i| i.pointer.any_released()) && shown.response.contains_pointer() {
                take = Some(Take::Put { identity: view.identity, from: dragged });
            }
        }
        // a backpack's menu and its rename box ("Enter backpack name.", FUN_100cb852: OK, back to its own name, or not)
        let text = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
        if let Some(at) = menu_at {
            self.popup = Some(crate::chrome::menu_at(ctx, at, &[(text("Rename", "Rename"), true)]));
        }
        if crate::chrome::menu(ctx, palette, "backpack-menu", &mut self.popup) == Some(0) {
            self.renaming = Some(title.clone());
        }
        if let Some(typed) = self.renaming.as_mut() {
            let mut asking = true;
            let mut answer = None;
            crate::chrome::window(ctx, palette, "rename-backpack", "Enter backpack name.", &mut asking, |w| w.resizable(false).pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.content_rect().center()), |ui| {
                let field = crate::chrome::text_edit(ui, palette, egui::TextEdit::singleline(typed).desired_width(200.0));
                crate::chrome::focus_when_new(ctx, "rename-backpack", &field);
                if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    answer = Some(Some(()));
                }
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, &text("MsgBox_OK", "OK"), true).clicked() {
                        answer = Some(Some(()));
                    }
                    if crate::chrome::button(ui, palette, &text("Reset2Default", "Its own name"), true).clicked() {
                        typed.clear();
                        answer = Some(Some(()));
                    }
                    if crate::chrome::button(ui, palette, &text("MsgBox_Cancel", "Cancel"), true).clicked() {
                        answer = Some(None);
                    }
                });
            });
            if let Some(Some(())) = answer {
                set_bag_name(view.identity, Some(typed.as_str()).filter(|n| !n.trim().is_empty() && *n != view.title));
            }
            if answer.is_some() || !asking {
                self.renaming = None;
            }
        }
        self.open = open;
        take
    }
}

/// The window's title for a container's identity type: a corpse (`0xc76a`), a bag (`0xc749`) or something else.
pub fn title(kind: u32) -> &'static str {
    match kind {
        CORPSE => "Corpse",
        0xc749 => "Bag",
        BANK => "Bank",
        ORGANIZATION => "Organization contract",
        _ => "Container",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backpack_has_its_own_window_and_name() {
        let bag = LootView { title: "Backpack".into(), identity: (BAG, 77), handle: 0x71, items: Vec::new(), own: true };
        let chest = LootView { title: "Chest".into(), identity: (BAG, 78), handle: 0x72, items: Vec::new(), own: false };
        assert_eq!((window_name(&bag).as_str(), window_name(&chest).as_str()), ("Container_51017x77", "loot"));
        // (21 places for one's own, as many as there are for a chest or corpse, the bank's 102)
        assert_eq!((max_items(BAG, true), max_items(BAG, false), max_items(BANK, false)), (21, -1, 102));
        // (the names' file: a line a bag, a name may have spaces; a line without a name is none)
        let names: BTreeMap<(u32, u32), String> = [((BAG, 77), "Ammo and kits".to_string()), ((BAG, 9), "Loot".to_string())].into_iter().collect();
        assert_eq!(parse_names(&render_names(&names)), names);
        assert!(parse_names("51017 5\njunk\n51017 x name\n").is_empty());
        // (named, and given its own name back)
        set_bag_name((BAG, 77), Some("  Ammo  "));
        assert_eq!(bag_name((BAG, 77)).as_deref(), Some("Ammo"));
        set_bag_name((BAG, 77), None);
        assert_eq!(bag_name((BAG, 77)), None);
    }

    #[test]
    fn titles_follow_the_container_kind() {
        assert_eq!(title(0xc76a), "Corpse");
        assert_eq!(title(0xc749), "Bag");
        assert_eq!(title(1), "Container");
    }
}
