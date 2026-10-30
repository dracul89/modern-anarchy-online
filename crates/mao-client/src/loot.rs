//! The loot window: what a corpse or a bag holds, as the server's `InventoryUpdate` lists it for a container the character used.
//! GUI.dll's `InventoryView_c` (FUN_100ccefe) for a container that is not one's own: an icon view of the items (its
//! `item_position_map` places them three to a row, 21 places), titled with the container's name; a corpse or chest that
//! is empty closes a second later (its `EventTimer_c` started for 1 000 000 µs). An item is taken by a double click (or
//! its menu), or all of them at once.

use std::collections::HashMap;

use crate::gear::GearItem;
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
}

/// An icon's square, and the icons in a row (`InventoryView_c`'s position map: place % 3, place / 3).
const CELL: f32 = 42.0;
const COLUMNS: usize = 3;
const BANK_COLUMNS: usize = 8;

impl LootWindow {
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
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        // (the bank's window is wider: its items run to a hundred and more)
        let columns = if view.identity.0 == BANK { BANK_COLUMNS } else { COLUMNS };
        let rows = view.items.len().div_ceil(columns).max(1);
        // (the title and the close button fit over the icons)
        let title_width = (view.title.chars().count() as f32 * 7.5 + 40.0).max(columns as f32 * (CELL + 2.0));
        let shown = crate::chrome::window(ctx, palette, "loot", &view.title, &mut open, |w| w.id(egui::Id::unique("loot")).default_pos(crate::layout::pos("loot", egui::pos2(380.0, 140.0))).resizable(false).min_width(title_width), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
            egui::Grid::new("loot-grid").spacing(egui::vec2(2.0, 2.0)).show(ui, |ui| {
                for row in 0..rows {
                    for column in 0..columns {
                        let item = view.items.get(row * columns + column);
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::click());
                        let painter = ui.painter();
                        painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(110));
                        painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, if item.is_some() { accent } else { egui::Color32::from_white_alpha(30) }), egui::StrokeKind::Inside);
                        let Some(item) = item else { continue };
                        match &item.icon {
                            Some(icon) => {
                                let id = self.textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("loot-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id();
                                painter.image(id, rect.shrink(3.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                            None => {
                                painter.text(rect.center(), egui::Align2::CENTER_CENTER, item.name.chars().next().unwrap_or('?'), egui::FontId::proportional(18.0), accent);
                            }
                        }
                        if item.count > 1 {
                            painter.text(rect.right_bottom() - egui::vec2(3.0, 2.0), egui::Align2::RIGHT_BOTTOM, item.count.to_string(), egui::FontId::proportional(11.0), egui::Color32::WHITE);
                        }
                        let mut tip = item.name.clone();
                        if item.quality > 1 {
                            tip.push_str(&format!("
QL {}", item.quality));
                        }
                        if !item.description.is_empty() {
                            tip.push_str("

");
                            tip.push_str(&item.description.chars().take(300).collect::<String>().replace("
", "
"));
                        }
                        let response = response.on_hover_text(tip);
                        if response.double_clicked() {
                            take = Some(Take::One { handle: view.handle, place: item.slot });
                        }
                        response.context_menu(|ui| {
                            if ui.button("Take").clicked() {
                                take = Some(Take::One { handle: view.handle, place: item.slot });
                                ui.close();
                            }
                        });
                    }
                    ui.end_row();
                }
            });
            if !view.items.is_empty() && view.identity.0 != BANK {
                if crate::chrome::button(ui, palette, "Take all", true).clicked() {
                    take = Some(Take::All { handle: view.handle });
                }
            }
        });
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
