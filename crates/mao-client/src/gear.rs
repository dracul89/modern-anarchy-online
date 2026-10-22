//! The gear window: what the character wears (equipment, implants, NCU units)
//! and carries (the backpack), from the world-entry message's item list
//! ([`mao_net::inventory`]) with each item's name, description and icon read
//! from the game's database ([`mao_formats::item`]).

use std::collections::HashMap;
use std::sync::Arc;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};
use mao_net::inventory::{self, Entry};

use crate::theme::Palette;

/// An item's icon: 48x48 RGBA.
pub struct Icon {
    pub id: u32,
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// One item ready to show.
pub struct GearItem {
    pub slot: u32,
    /// The item's template id (what a hotbar slot remembers it by).
    pub item_id: u32,
    pub count: u16,
    pub quality: u32,
    pub bag: bool,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
}

/// The placements the original numbers, as far as the first capture shows
/// them (the armour, implant and NCU rows hold the items the names say).
pub fn slot_name(slot: u32) -> Option<&'static str> {
    Some(match slot {
        1 => "HUD 1",
        2 => "HUD 3",
        3 => "Utility 1",
        4 => "Utility 2",
        5 => "Utility 3",
        6 => "Right hand",
        7 => "Belt",
        8 => "Left hand",
        9 => "NCU 1",
        10 => "NCU 2",
        11 => "NCU 3",
        12 => "NCU 4",
        13 => "NCU 5",
        17 => "Neck",
        18 => "Head",
        19 => "Back",
        20 => "Right shoulder",
        21 => "Chest",
        22 => "Left shoulder",
        23 => "Right arm",
        24 => "Hands",
        25 => "Left arm",
        26 => "Right wrist",
        27 => "Legs",
        28 => "Left wrist",
        30 => "Feet",
        31 => "Right finger",
        32 => "Left finger",
        33 => "Eye implant",
        34 => "Head implant",
        35 => "Ear implant",
        36 => "Right-arm implant",
        37 => "Chest implant",
        38 => "Left-arm implant",
        39 => "Right-wrist implant",
        40 => "Waist implant",
        41 => "Left-wrist implant",
        42 => "Right-hand implant",
        43 => "Leg implant",
        44 => "Left-hand implant",
        45 => "Feet implant",
        _ => return None,
    })
}

/// The rows of the equipment panel: a title and its slots.
const SECTIONS: &[(&str, &[u32])] = &[
    ("Weapons and utilities", &[1, 2, 3, 4, 5, 6, 8]),
    ("Belt and NCU", &[7, 9, 10, 11, 12, 13]),
    ("Armour", &[17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 30, 31, 32]),
    ("Implants", &[33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45]),
];

/// The items of a world-entry message with their names and icons.
pub fn from_entries(db: &ResourceDatabase, textures: &crate::Textures, entries: &[Entry]) -> Vec<GearItem> {
    let mut icons: HashMap<u32, Option<Arc<Icon>>> = HashMap::new();
    let lookup = |id: u32| db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten().and_then(|d| Item::parse(&d));
    entries
        .iter()
        .map(|e| {
            let template = lookup(e.low_id).or_else(|| lookup(e.high_id));
            let icon = template.as_ref().and_then(|t| t.icon()).and_then(|id| {
                icons
                    .entry(id)
                    .or_insert_with(|| textures.get(item::ICON_TYPE, id).map(|t| Arc::new(Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })))
                    .clone()
            });
            GearItem {
                slot: e.slot,
                item_id: e.low_id,
                count: e.count,
                quality: e.quality,
                bag: e.is_bag(),
                name: template.as_ref().map_or_else(|| format!("Item #{:x}", e.low_id), |t| t.name.clone()),
                description: template.map(|t| t.description).unwrap_or_default(),
                icon,
            }
        })
        .collect()
}

/// The window's own state: the icons already handed to the interface.
#[derive(Default)]
pub struct GearWindow {
    textures: HashMap<u32, egui::TextureHandle>,
}

const CELL: f32 = 42.0;

impl GearWindow {
    pub fn texture(&mut self, ctx: &egui::Context, icon: &Icon) -> egui::TextureId {
        self.textures
            .entry(icon.id)
            .or_insert_with(|| ctx.load_texture(format!("item-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR))
            .id()
    }

    /// One square: the item's icon (and stack size), or an empty frame.
    fn cell(&mut self, ui: &mut egui::Ui, item: Option<&GearItem>, label: &str, accent: egui::Color32) {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::drag());
        let painter = ui.painter();
        painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(110));
        painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, if item.is_some() { accent } else { egui::Color32::from_white_alpha(30) }), egui::StrokeKind::Inside);
        let Some(item) = item else {
            response.on_hover_text(label);
            return;
        };
        match &item.icon {
            Some(icon) => {
                let id = self.texture(ui.ctx(), icon);
                ui.painter().image(id, rect.shrink(3.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            None => {
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, item.name.chars().next().unwrap_or('?'), egui::FontId::proportional(18.0), accent);
            }
        }
        if item.count > 1 {
            let text = if item.count >= 10000 { format!("{}k", item.count / 1000) } else { item.count.to_string() };
            ui.painter().text(rect.right_bottom() - egui::vec2(3.0, 2.0), egui::Align2::RIGHT_BOTTOM, text, egui::FontId::proportional(11.0), egui::Color32::WHITE);
        }
        // Drag an item onto a hotbar slot to add it to that slot's list.
        if response.drag_started() {
            response.dnd_set_drag_payload(crate::hotbar::Action::Item { id: item.item_id, name: item.name.clone(), icon: item.icon.as_ref().map(|i| i.id) });
        }
        response.on_hover_ui(|ui| {
            ui.set_max_width(320.0);
            ui.label(egui::RichText::new(&item.name).strong().color(accent));
            let mut line = String::new();
            if !label.is_empty() {
                line += label;
            }
            if item.quality > 1 {
                if !line.is_empty() {
                    line += "  ";
                }
                line += &format!("QL {}", item.quality);
            }
            if item.bag {
                line += "  (bag)";
            }
            if !line.is_empty() {
                ui.label(egui::RichText::new(line).small());
            }
            if !item.description.is_empty() {
                let text: String = item.description.chars().take(420).collect();
                ui.label(text.replace("\r\n", "\n"));
            }
        });
    }

    /// The window: the equipment on the left, the backpack on the right.
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool, items: &[GearItem], palette: &Palette) {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let by_slot: HashMap<u32, &GearItem> = items.iter().map(|i| (i.slot, i)).collect();
        let mut is_open = *open;
        egui::Window::new(egui::RichText::new("Inventory").color(accent)).open(&mut is_open).collapsible(false).resizable(true).default_pos(egui::pos2(40.0, 70.0)).default_width(760.0).show(ctx, |ui| {
            if items.is_empty() {
                ui.label("No items yet (they arrive when the world loads).");
                return;
            }
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(330.0);
                    for (title, slots) in SECTIONS {
                        ui.label(egui::RichText::new(*title).small().color(accent));
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                            for &slot in *slots {
                                self.cell(ui, by_slot.get(&slot).copied(), slot_name(slot).unwrap_or(""), accent);
                            }
                        });
                        ui.add_space(4.0);
                    }
                    let social: Vec<&GearItem> = items.iter().filter(|i| i.slot >= 46 && i.slot < inventory::BACKPACK_FIRST).collect();
                    if !social.is_empty() {
                        ui.label(egui::RichText::new("Social").small().color(accent));
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                            for item in social {
                                self.cell(ui, Some(item), "Social", accent);
                            }
                        });
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Backpack").small().color(accent));
                    let mut pack: Vec<&GearItem> = items.iter().filter(|i| i.slot >= inventory::BACKPACK_FIRST).collect();
                    pack.sort_by_key(|i| i.slot);
                    egui::ScrollArea::vertical().max_height(440.0).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.set_width(7.0 * (CELL + 3.0));
                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                            for item in pack {
                                self.cell(ui, Some(item), "", accent);
                            }
                        });
                    });
                });
            });
        });
        *open = is_open;
    }
}
