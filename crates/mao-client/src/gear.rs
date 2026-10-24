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
    /// What the item adds to the stats when worn (`derived::effects`: stat, amount), for the tooltip.
    pub effects: Vec<(u32, i64)>,
}

/// How many places the backpack has.
pub const PACK_SLOTS: u32 = 30;

/// The original's "Wear" window: four tabs of three columns, with each cell's placement
/// (from the original client's own window and the capture: the belt "Deck" holds the NCU units
/// "Deck 1-6", the social tab starts at placement 49). `None` is a cell whose placement is not known.
pub const TABS: &[(&str, &[(&str, Option<u32>)])] = &[
    (
        "Weap",
        &[
            ("Hud 1", Some(1)), ("Hud 2", Some(15)), ("Hud 3", Some(2)),
            ("Utils 1", Some(3)), ("Utils 2", Some(4)), ("Utils 3", Some(5)),
            ("Right Hand", Some(6)), ("Belt", Some(7)), ("Left Hand", Some(8)),
            ("Deck 1", Some(9)), ("Deck 2", Some(10)), ("Deck 3", Some(11)),
            ("Deck 4", Some(12)), ("Deck 5", Some(13)), ("Deck 6", Some(14)),
        ],
    ),
    (
        "Cloth",
        &[
            ("Neck", Some(17)), ("Head", Some(18)), ("Back", Some(19)),
            ("Right Shoulder", Some(20)), ("Chest", Some(21)), ("Left Shoulder", Some(22)),
            ("Right Arm", Some(23)), ("Hands", Some(24)), ("Left Arm", Some(25)),
            ("Right Wrist", Some(26)), ("Legs", Some(27)), ("Left Wrist", Some(28)),
            ("Right Finger", Some(29)), ("Feet", Some(30)), ("Left Finger", Some(31)),
        ],
    ),
    (
        "Impl",
        &[
            ("Eyes", Some(33)), ("Head", Some(34)), ("Ears", Some(35)),
            ("Right Arm", Some(36)), ("Chest", Some(37)), ("Left Arm", Some(38)),
            ("Right Wrist", Some(39)), ("Waist", Some(40)), ("Left Wrist", Some(41)),
            ("Right Hand", Some(42)), ("Legs", Some(43)), ("Left Hand", Some(44)),
            ("", None), ("Feet", Some(45)), ("", None),
        ],
    ),
    (
        "Social",
        &[
            ("Neck", Some(49)), ("Head", Some(50)), ("Back", Some(51)),
            ("Right Shoulder", Some(52)), ("Chest", Some(53)), ("Left Shoulder", Some(54)),
            ("Right Arm", Some(55)), ("Hands", Some(56)), ("Left Arm", Some(57)),
            ("Right Wrist", Some(58)), ("Legs", Some(59)), ("Left Wrist", Some(60)),
            ("Right Hand", Some(61)), ("Feet", Some(62)), ("Left Hand", Some(63)),
        ],
    ),
];

/// A placement's name, with its tab ("Weap: Right Hand").
pub fn slot_name(slot: u32) -> Option<String> {
    TABS.iter().find_map(|(tab, cells)| cells.iter().find(|c| c.1 == Some(slot)).map(|c| format!("{tab}: {}", c.0)))
}

/// The items of a world-entry message with their names and icons.
pub fn from_entries(db: &ResourceDatabase, textures: &crate::Textures, entries: &[Entry], level: i64) -> Vec<GearItem> {
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
                effects: if e.is_bag() { Vec::new() } else { crate::derived::effects(db, e, level) },
            }
        })
        .collect()
}

/// The window's own state: the icons already handed to the interface.
#[derive(Default)]
pub struct GearWindow {
    textures: HashMap<u32, egui::TextureHandle>,
    /// The wear tab shown.
    tab: usize,
}

const CELL: f32 = 42.0;

impl GearWindow {
    /// Shows the wear tab with this name ("Weap", "Cloth", "Impl", "Social"), when there is one (the previews use it).
    pub fn select_tab(&mut self, name: &str) {
        if let Some(i) = TABS.iter().position(|(tab, _)| tab.eq_ignore_ascii_case(name)) {
            self.tab = i;
        }
    }

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
            if item.count > 1 {
                ui.label(egui::RichText::new(format!("{} in the stack", item.count)).small());
            }
            if !item.description.is_empty() {
                let text: String = item.description.chars().take(420).collect();
                ui.label(text.replace("\r\n", "\n"));
            }
            if !item.effects.is_empty() {
                ui.separator();
                ui.label(egui::RichText::new("When worn").small().color(accent));
                for &(stat, amount) in item.effects.iter().filter(|e| e.1 != 0).take(10) {
                    ui.horizontal(|ui| {
                        ui.label(crate::skills::stat_name(stat));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let color = if amount > 0 { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(230, 110, 110) };
                            ui.label(egui::RichText::new(format!("{amount:+}")).color(color));
                        });
                    });
                }
                let more = item.effects.iter().filter(|e| e.1 != 0).count().saturating_sub(10);
                if more > 0 {
                    ui.label(egui::RichText::new(format!("... and {more} more")).small().weak());
                }
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
                    ui.set_width(3.0 * (CELL + 3.0) + 8.0);
                    ui.horizontal(|ui| {
                        for (i, (tab, _)) in TABS.iter().enumerate() {
                            if ui.selectable_label(self.tab == i, *tab).clicked() {
                                self.tab = i;
                            }
                        }
                    });
                    let (_, cells) = TABS[self.tab.min(TABS.len() - 1)];
                    egui::Grid::new("wear").spacing(egui::vec2(3.0, 3.0)).show(ui, |ui| {
                        for (n, (label, slot)) in cells.iter().enumerate() {
                            match slot {
                                Some(slot) if !label.is_empty() => self.cell(ui, by_slot.get(slot).copied(), label, accent),
                                _ => {
                                    ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::hover());
                                }
                            }
                            if n % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Backpack").small().color(accent));
                    let mut pack: Vec<&GearItem> = items.iter().filter(|i| i.slot >= inventory::BACKPACK_FIRST).collect();
                    pack.sort_by_key(|i| i.slot);
                    egui::ScrollArea::vertical().max_height(440.0).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.set_width(6.0 * (CELL + 3.0));
                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                            // The pack has 30 places: each item sits in its own, and the free ones are drawn empty.
                            for place in 0..PACK_SLOTS {
                                let item = pack.iter().find(|i| i.slot == inventory::BACKPACK_FIRST + place).copied();
                                self.cell(ui, item, "", accent);
                            }
                            // Anything outside those places (a larger pack, a bag's contents) follows.
                            for item in pack.iter().filter(|i| i.slot >= inventory::BACKPACK_FIRST + PACK_SLOTS) {
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
