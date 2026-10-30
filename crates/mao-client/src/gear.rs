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
    /// The item's template id (what a hotbar slot remembers it by), and its high template's.
    pub item_id: u32,
    pub high_id: u32,
    pub count: u16,
    pub quality: u32,
    pub bag: bool,
    /// The template's flags (stat 0: NoDrop and the like).
    pub flags: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// What the item adds to the stats when worn (`derived::effects`: stat, amount), for the tooltip.
    pub effects: Vec<(u32, i64)>,
    /// What the item needs to be used, worn or wielded, at its quality (`derived::actions`).
    pub actions: Vec<mao_formats::item::Action>,
}

/// The inventory place of the item being dragged (`u32::MAX` for none): other windows (an open bag) take drops from it.
pub static DRAGGED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);

/// How many of the pack's own places hold an item.
pub fn pack_used(items: &[GearItem]) -> usize {
    items.iter().filter(|i| (inventory::BACKPACK_FIRST..inventory::BACKPACK_FIRST + PACK_SLOTS).contains(&i.slot)).count()
}

/// Whether an item's name matches the find box (an empty box matches everything).
pub fn matches_filter(name: &str, filter: &str) -> bool {
    let filter = filter.trim().to_lowercase();
    filter.is_empty() || name.to_lowercase().contains(&filter)
}

/// What the window asks for besides moves: items to use and to destroy (by inventory place).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct GearEvents {
    pub moves: Vec<(u32, u32)>,
    pub uses: Vec<u32>,
    pub deletes: Vec<u32>,
    /// Info pages asked for (`itemref://` URLs for the info view).
    pub infos: Vec<String>,
    /// Items Ctrl+double-clicked: put into the open trade, as the original's inventory list does with a trade open
    /// (FUN_100cae1b on the list's double-click, the qualifiers' Ctrl bits 0xc: `N3Msg_TradeAddItem`).
    pub trade_adds: Vec<u32>,
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
                high_id: e.high_id,
                count: e.count,
                quality: e.quality,
                bag: e.is_bag(),
                flags: template.as_ref().and_then(|t| t.stat(0)).unwrap_or(0),
                name: template.as_ref().map_or_else(|| format!("Item #{:x}", e.low_id), |t| t.name.clone()),
                description: template.map(|t| t.description).unwrap_or_default(),
                icon,
                effects: if e.is_bag() { Vec::new() } else { crate::derived::effects(db, e, level) },
                actions: if e.is_bag() { Vec::new() } else { crate::derived::actions(db, e) },
            }
        })
        .collect()
}

/// A criteria list as lines of text, each with whether the character meets it (`None` when that cannot be told): the list is a
/// postfix expression, so it is evaluated with a stack; the top-level AND chain gives one line per condition, anything under an OR
/// is shown as one line.
pub fn requirement_lines(criteria: &[(u32, i32, u32)], stats: &std::collections::BTreeMap<u32, i64>) -> Vec<(String, Option<bool>)> {
    // each stack entry: its lines, and whether the whole holds
    let mut stack: Vec<(Vec<(String, Option<bool>)>, Option<bool>)> = Vec::new();
    for &(stat, value, op) in criteria {
        match op {
            4 | 3 => {
                let (Some(b), Some(a)) = (stack.pop(), stack.pop()) else { return Vec::new() };
                let holds = match (a.1, b.1, op) {
                    (Some(x), Some(y), 4) => Some(x && y),
                    (Some(x), Some(y), _) => Some(x || y),
                    _ => None,
                };
                if op == 4 {
                    let mut lines = a.0;
                    lines.extend(b.0);
                    stack.push((lines, holds));
                } else {
                    let join = |l: &[(String, Option<bool>)]| l.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>().join(" and ");
                    stack.push((vec![(format!("{} or {}", join(&a.0), join(&b.0)), holds)], holds));
                }
            }
            _ => {
                let have = stats.get(&stat).copied();
                let name = crate::skills::stat_name(stat);
                let (text, met) = match op {
                    0 => (format!("{name} = {value}"), have.map(|h| h == value as i64)),
                    1 => (format!("{name} below {value}"), have.map(|h| h < value as i64)),
                    // a "greater than" criterion is strict: "level above 214" is a requirement of level 215
                    2 => (format!("{name} {}", value as i64 + 1), have.map(|h| h > value as i64)),
                    24 => (format!("{name} not {value}"), have.map(|h| h != value as i64)),
                    22 => (format!("{name} has flag {value}"), have.map(|h| h & value as i64 == value as i64)),
                    93 => (format!("Perk {value}"), None),
                    _ => (format!("condition {op} on {name} {value}"), None),
                };
                stack.push((vec![(text, met)], met));
            }
        }
    }
    stack.into_iter().flat_map(|(lines, _)| lines).collect()
}

/// The window's own state: the icons already handed to the interface.
#[derive(Default)]
pub struct GearWindow {
    textures: HashMap<u32, egui::TextureHandle>,
    /// The wear tab shown.
    tab: usize,
    /// The place of the item being dragged, and the moves made (from, to) since the last `show` returned.
    dragging: Option<u32>,
    moves: Vec<(u32, u32)>,
    uses: Vec<u32>,
    deletes: Vec<u32>,
    /// Info pages asked for since the last frame.
    infos: Vec<String>,
    trade_adds: Vec<u32>,
    /// The place an item awaits a second click to be destroyed.
    confirm: Option<u32>,
    /// Items whose name does not contain this are dimmed (any case).
    filter: String,
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
    fn cell(&mut self, ui: &mut egui::Ui, item: Option<&GearItem>, place: Option<u32>, label: &str, accent: egui::Color32, stats: &std::collections::BTreeMap<u32, i64>) {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::click_and_drag());
        // An item dropped here moves to this place.
        if let (Some(place), Some(from)) = (place, self.dragging) {
            if from != place && response.contains_pointer() && ui.input(|i| i.pointer.any_released()) {
                self.moves.push((from, place));
            }
        }
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
        // (an item that does not match the find box is dimmed)
        if !matches_filter(&item.name, &self.filter) {
            ui.painter().rect_filled(rect, 3.0, egui::Color32::from_black_alpha(185));
        }
        if item.count > 1 {
            let text = if item.count >= 10000 { format!("{}k", item.count / 1000) } else { item.count.to_string() };
            ui.painter().text(rect.right_bottom() - egui::vec2(3.0, 2.0), egui::Align2::RIGHT_BOTTOM, text, egui::FontId::proportional(11.0), egui::Color32::WHITE);
        }
        if response.double_clicked() && ui.input(|i| i.modifiers.ctrl) {
            self.trade_adds.push(item.slot);
        }
        // Drag an item onto a hotbar slot to add it to that slot's list.
        if response.drag_started() {
            self.dragging = Some(item.slot);
            DRAGGED.store(item.slot, std::sync::atomic::Ordering::Relaxed);
            response.dnd_set_drag_payload(crate::hotbar::Action::Item { id: item.item_id, name: item.name.clone(), icon: item.icon.as_ref().map(|i| i.id) });
        }
        let menu_slot = item.slot;
        let menu_name = item.name.clone();
        let info_url = format!("itemref://{}/{}/{}", item.item_id, item.high_id, item.quality);
        let (mut uses, mut deletes, mut confirm) = (Vec::new(), Vec::new(), self.confirm);
        let mut infos = Vec::new();
        response.context_menu(|ui| {
            ui.label(egui::RichText::new(&menu_name).strong());
            // (the original's info view: `ChatGUIModule_c::ShowItemRefLink` / `InfoViewModule_c::ShowURL`)
            if ui.button("Info").clicked() {
                infos.push(info_url.clone());
                ui.close();
            }
            if ui.button("Use").clicked() {
                uses.push(menu_slot);
                ui.close();
            }
            // A second click destroys it: nothing is asked of the server before.
            let sure = confirm == Some(menu_slot);
            if ui.button(if sure { "Really delete?" } else { "Delete" }).clicked() {
                if sure {
                    deletes.push(menu_slot);
                    confirm = None;
                    ui.close();
                } else {
                    confirm = Some(menu_slot);
                }
            }
        });
        self.uses.extend(uses);
        self.infos.extend(infos);
        self.deletes.extend(deletes);
        self.confirm = confirm;
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
            for action in item.actions.iter().filter(|a| a.kind == 6 || a.kind == 8 || a.kind == 3) {
                let lines = requirement_lines(&action.criteria, stats);
                if lines.is_empty() {
                    continue;
                }
                ui.separator();
                let what = match action.kind {
                    6 => "To wear",
                    8 => "To wield",
                    _ => "To use",
                };
                ui.label(egui::RichText::new(what).small().color(accent));
                for (text, met) in lines {
                    let color = match met {
                        Some(true) => egui::Color32::from_rgb(120, 220, 120),
                        Some(false) => egui::Color32::from_rgb(230, 110, 110),
                        None => ui.visuals().text_color(),
                    };
                    ui.label(egui::RichText::new(text).color(color));
                }
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
    /// Returns the moves made by dragging an item onto a place: `(from, to)` inventory places.
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool, items: &[GearItem], stats: &std::collections::BTreeMap<u32, i64>, palette: &Palette) -> GearEvents {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let by_slot: HashMap<u32, &GearItem> = items.iter().map(|i| (i.slot, i)).collect();
        let mut is_open = *open;
        crate::chrome::window(ctx, palette, "Inventory", "Inventory", &mut is_open, |w| w.resizable(true).default_pos(crate::layout::pos("Inventory", egui::pos2(40.0, 70.0))).default_width(760.0), |ui| {
            if items.is_empty() {
                ui.label("No items yet (they arrive when the world loads).");
                return;
            }
            ui.horizontal(|ui| {
                ui.label("Find");
                crate::chrome::text_edit(ui, palette, egui::TextEdit::singleline(&mut self.filter).desired_width(160.0));
                ui.label(egui::RichText::new("drag an item onto a place to wear or move it, right-click for more").small().weak());
            });
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(3.0 * (CELL + 3.0) + 8.0);
                    let names: Vec<&str> = TABS.iter().map(|(t, _)| *t).collect();
                    let mut tab = self.tab;
                    crate::chrome::tab_view(ui, palette, &names, &mut tab, |ui, tab| {
                    let (_, cells) = TABS[tab.min(TABS.len() - 1)];
                    egui::Grid::new("wear").spacing(egui::vec2(3.0, 3.0)).show(ui, |ui| {
                        for (n, (label, slot)) in cells.iter().enumerate() {
                            match slot {
                                Some(slot) if !label.is_empty() => self.cell(ui, by_slot.get(slot).copied(), Some(*slot), label, accent, stats),
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
                    self.tab = tab;
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(format!("Backpack  {}/{}", pack_used(items), PACK_SLOTS)).small().color(accent));
                    let mut pack: Vec<&GearItem> = items.iter().filter(|i| i.slot >= inventory::BACKPACK_FIRST).collect();
                    pack.sort_by_key(|i| i.slot);
                    crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(440.0), |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.set_width(6.0 * (CELL + 3.0));
                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                            // The pack has 30 places: each item sits in its own, and the free ones are drawn empty.
                            for place in 0..PACK_SLOTS {
                                let item = pack.iter().find(|i| i.slot == inventory::BACKPACK_FIRST + place).copied();
                                self.cell(ui, item, Some(inventory::BACKPACK_FIRST + place), "", accent, stats);
                            }
                            // Anything outside those places (a larger pack, a bag's contents) follows.
                            for item in pack.iter().filter(|i| i.slot >= inventory::BACKPACK_FIRST + PACK_SLOTS) {
                                self.cell(ui, Some(item), None, "", accent, stats);
                            }
                        });
                    });
                });
            });
        });
        *open = is_open;
        if ctx.input(|i| i.pointer.any_released()) {
            self.dragging = None;
        }
        GearEvents { moves: std::mem::take(&mut self.moves), uses: std::mem::take(&mut self.uses), deletes: std::mem::take(&mut self.deletes), infos: std::mem::take(&mut self.infos), trade_adds: std::mem::take(&mut self.trade_adds) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pack_counts_only_its_own_places() {
        let item = |slot| GearItem { slot, item_id: 1, high_id: 1, count: 1, quality: 1, bag: false, flags: 0, name: String::new(), description: String::new(), icon: None, effects: Vec::new(), actions: Vec::new() };
        let items = vec![item(0x10), item(inventory::BACKPACK_FIRST), item(inventory::BACKPACK_FIRST + 5), item(inventory::BACKPACK_FIRST + PACK_SLOTS)];
        assert_eq!(pack_used(&items), 2);
    }

    #[test]
    fn the_find_box_matches_part_of_a_name() {
        assert!(matches_filter("Dust Brigade Notum Gloves", ""));
        assert!(matches_filter("Dust Brigade Notum Gloves", " notum "));
        assert!(!matches_filter("Dust Brigade Notum Gloves", "boots"));
    }

    #[test]
    fn requirements_read_as_conditions_met_or_not() {
        // Dustbrigade Notum Gloves: intelligence above 900 and level above 215 (so "Intelligence 901", "Level 216"), AND.
        let criteria = [(19, 900, 2), (54, 215, 2), (0, 0, 4)];
        let stats: std::collections::BTreeMap<u32, i64> = [(19, 1097), (54, 215)].into();
        assert_eq!(requirement_lines(&criteria, &stats), vec![("Intelligence 901".to_string(), Some(true)), ("Level 216".to_string(), Some(false))]);
        // an unknown stat cannot be judged
        assert_eq!(requirement_lines(&[(19, 5, 2)], &Default::default()), vec![("Intelligence 6".to_string(), None)]);
        // OR: one line, met when either is
        let either = [(54, 10, 2), (16, 50, 2), (0, 0, 3)];
        let met: std::collections::BTreeMap<u32, i64> = [(54, 5), (16, 100)].into();
        assert_eq!(requirement_lines(&either, &met), vec![("Level 11 or Strength 51".to_string(), Some(true))]);
    }
}
