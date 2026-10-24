//! The loot window: what a corpse or a bag holds, as the server's `InventoryUpdate` lists it for a container the character used.
//! Each item can be taken (it goes to the first free place of the backpack), or all of them at once.

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
    textures: HashMap<u32, egui::TextureId>,
}

const ICON: f32 = 28.0;

impl LootWindow {
    pub fn show(&mut self, ctx: &egui::Context, view: &LootView, palette: &Palette) -> Option<Take> {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut take = None;
        let mut open = self.open;
        let shown = egui::Window::new(egui::RichText::new(&view.title).color(accent)).id(egui::Id::new("loot")).open(&mut open).collapsible(false).default_pos(crate::layout::pos("loot", egui::pos2(380.0, 140.0))).default_width(280.0).show(ctx, |ui| {
            if view.items.is_empty() {
                ui.label("Nothing left.");
                return;
            }
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for item in &view.items {
                    ui.horizontal(|ui| {
                        match &item.icon {
                            Some(icon) => {
                                let id = *self.textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("loot-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR).id());
                                ui.image(egui::load::SizedTexture::new(id, egui::vec2(ICON, ICON)));
                            }
                            None => {
                                ui.allocate_exact_size(egui::vec2(ICON, ICON), egui::Sense::hover());
                            }
                        }
                        let label = if item.count > 1 { format!("{} x{}", item.name, item.count) } else { item.name.clone() };
                        let response = ui.label(label);
                        if !item.description.is_empty() {
                            response.on_hover_text(item.description.chars().take(300).collect::<String>().replace("\r\n", "\n"));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Take").clicked() {
                                take = Some(Take::One { handle: view.handle, place: item.slot });
                            }
                            if item.quality > 1 {
                                ui.label(egui::RichText::new(format!("QL {}", item.quality)).small().weak());
                            }
                        });
                    });
                }
            });
            ui.separator();
            if ui.button("Take all").clicked() {
                take = Some(Take::All { handle: view.handle });
            }
            if view.identity.0 != CORPSE {
                ui.label(egui::RichText::new("Drop an item here to put it in").small().weak());
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
