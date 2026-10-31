//! The loot window: what a corpse or a bag holds, as the server's `InventoryUpdate` lists it for a container the character used.
//! GUI.dll's `InventoryView_c` (FUN_100ccefe) for a container that is not one's own: an icon view of the items (its
//! `item_position_map` places them three to a row, 21 places), titled with the container's name; a corpse or chest that
//! is empty closes a second later (its `EventTimer_c` started for 1 000 000 µs). An item is taken by a double click (or
//! its menu), or all of them at once.

use std::collections::HashMap;

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
}

/// The identity type of a corpse.
const CORPSE: u32 = 0xc76a;
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
}

/// The most items a container's grid shows slots for (`ItemContainerView_c` FUN_100ce55d: `SetMaxItemCount` 102 for the bank,
/// 21 for one's own backpack, none for a corpse or chest).
pub fn max_items(kind: u32) -> i32 {
    match kind {
        BANK => 0x66,
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
        let list = crate::gui::multilist::create_grid(&mut tree, "ItemContainerView", max_items(view.identity.0), columns(view.identity.0));
        tree.add_child(root, list);
        self.views = Some((tree, root, list, view.identity));
        self.shown.clear();
    }

    pub fn show(&mut self, ctx: &egui::Context, view: &LootView, palette: &Palette) -> Option<Take> {
        let mut take = None;
        let mut open = self.open;
        // (FUN_100ccefe: an empty corpse or chest closes after a second; the bank stays)
        if view.items.is_empty() && view.identity.0 != BANK {
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
                })
                .collect();
            crate::gui::multilist::set_items(tree, list, rows);
            self.shown = now;
        }
        crate::gui::multilist::update(tree, list);
        let skin = crate::skin::skin(ctx);
        let cols = columns(view.identity.0);
        // (room for the columns and the scroll bar, and up to six rows: the bank's, or as many as the items fill)
        let rows_shown = if view.identity.0 == BANK { 6 } else { view.items.len().div_ceil(cols).clamp(1, 6) };
        let size = egui::vec2(cols as f32 * crate::gui::multilist::grid::STEP + 2.0 * crate::gui::multilist::grid::BORDER + 32.0, crate::gui::multilist::grid::size(rows_shown * cols, cols).1 + if view.identity.0 == BANK { 0.0 } else { 26.0 });
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
        let shown = crate::chrome::window(ctx, palette, "loot", &view.title, &mut open, |w| w.id(egui::Id::unique("loot")).default_pos(crate::layout::pos("loot", egui::pos2(380.0, 140.0))).default_size(size).resizable(true), |ui| {
            // (the views over the window less the room of the "Take all" row under them)
            // (laid out from the bottom: the button first, the views over what is left, so the window does not grow)
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                if !view.items.is_empty() && view.identity.0 != BANK && crate::chrome::button(ui, palette, "Take all", true).clicked() {
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
                crate::gui::multilist::ItemEvent::Dragged(_) => {}
            }
        }
        tree.signals.clear();
        // An inventory item dropped on the window goes in (not into a corpse).
        if let Some(shown) = shown {
            let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
            if dragged != u32::MAX && view.identity.0 != CORPSE && ctx.input(|i| i.pointer.any_released()) && shown.response.contains_pointer() {
                take = Some(Take::Put { identity: view.identity, from: dragged });
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
        _ => "Container",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_follow_the_container_kind() {
        assert_eq!(title(0xc76a), "Corpse");
        assert_eq!(title(0xc749), "Bag");
        assert_eq!(title(1), "Container");
    }
}
