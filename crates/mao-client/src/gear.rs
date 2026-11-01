//! The items: what the character wears (equipment, implants, NCU units)
//! and carries (the backpack), from the world-entry message's item list
//! ([`mao_net::inventory`]) with each item's name, description and icon read
//! from the game's database ([`mao_formats::item`]).

use std::collections::HashMap;
use std::sync::Arc;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};
use mao_net::inventory::Entry;

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
    /// A backpack item let go on another: (the item, the one it went on).
    pub combines: Vec<(u32, u32)>,
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

/// An item's tip: its name, place, quality, stack, description, what it needs and what it adds.
pub fn tooltip(ui: &mut egui::Ui, item: &GearItem, label: &str, stats: &std::collections::BTreeMap<u32, i64>, accent: egui::Color32) {
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
}

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
/// is shown as one line. Whether a condition is met is the criteria evaluator's answer for it alone (`requirements`, FUN_100065d8
/// on the client's own character: the below / above and the flag operators, and those that always pass or fail on the client);
/// a condition on a perk, or after an "apply on" operator that moves it off the character (On Target, On User, On Caster ...),
/// cannot be told here.
pub fn requirement_lines(criteria: &[(u32, i32, u32)], stats: &std::collections::BTreeMap<u32, i64>) -> Vec<(String, Option<bool>)> {
    // (the character's stats only: its perks are not given here)
    struct Stats<'a>(&'a std::collections::BTreeMap<u32, i64>);
    impl crate::requirements::Context for Stats<'_> {
        fn stat(&self, holder: crate::requirements::Holder, stat: u32) -> Option<i64> {
            (holder == ME).then(|| self.0.get(&stat).copied()).flatten()
        }
        fn is_own(&self, holder: crate::requirements::Holder) -> bool {
            holder == ME
        }
    }
    const ME: crate::requirements::Holder = (crate::requirements::CHARACTER, 0);
    let ctx = Stats(stats);
    // each stack entry: its lines, and whether the whole holds
    let mut stack: Vec<(Vec<(String, Option<bool>)>, Option<bool>)> = Vec::new();
    // (whether the conditions are about the character: the apply-on operators move them, "On Self" (19) back)
    let mut on_self = true;
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
            0x12 | 0x13 | 0x15 | 0x1a | 0x64 | 0x6e => on_self = op == 0x13,
            _ => {
                let name = crate::skills::stat_name(stat);
                let met = if on_self { crate::requirements::evaluate(&[(stat, value, op)], ME, &ctx).ok() } else { None };
                let text = match op {
                    0 => format!("{name} = {value}"),
                    1 => format!("{name} below {value}"),
                    // a "greater than" criterion is strict: "level above 214" is a requirement of level 215
                    2 => format!("{name} {}", value as i64 + 1),
                    24 => format!("{name} not {value}"),
                    22 => format!("{name} has flag {value}"),
                    107 => format!("{name} has none of flags {value}"),
                    93 => format!("Perk {value}"),
                    _ => format!("condition {op} on {name} {value}"),
                };
                stack.push((vec![(text, met)], met));
            }
        }
    }
    stack.into_iter().flat_map(|(lines, _)| lines).collect()
}

/// The window's own state: the icons already handed to the interface, and the inventory and wear windows (`invwindows.rs`).
#[derive(Default)]
pub struct GearWindow {
    textures: HashMap<u32, egui::TextureHandle>,
    windows: crate::invwindows::ItemWindows,
    inspect: crate::invwindows::InspectWindow,
}

impl GearWindow {
    /// Shows the wear tab with this name ("Weap", "Cloth", "Impl", "Social"), when there is one (the previews use it).
    pub fn select_tab(&mut self, name: &str) {
        if let Some(i) = TABS.iter().position(|(tab, _)| tab.eq_ignore_ascii_case(name)) {
            self.windows.tab = i;
        }
    }

    pub fn texture(&mut self, ctx: &egui::Context, icon: &Icon) -> egui::TextureId {
        self.textures
            .entry(icon.id)
            .or_insert_with(|| ctx.load_texture(format!("item-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR))
            .id()
    }

    /// The inventory and wear windows; what was done with the items.
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool, items: &[GearItem], stats: &std::collections::BTreeMap<u32, i64>, palette: &Palette) -> GearEvents {
        let e = self.windows.show(ctx, open, items, stats, palette, &mut self.textures);
        GearEvents { moves: e.moves, uses: e.uses, deletes: e.deletes, infos: e.infos, trade_adds: e.trade_adds, combines: e.combines }
    }

    /// The inspect window (opened by each new inspection); the info pages asked for.
    pub fn show_inspect(&mut self, ctx: &egui::Context, inspected: &(u32, String, Vec<GearItem>), stats: &std::collections::BTreeMap<u32, i64>, palette: &Palette) -> Vec<String> {
        self.inspect.show(ctx, inspected, stats, palette, &mut self.textures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // a condition after "On Target" is about someone else; "On Self" brings them back; a perk cannot be told here
        let target = [(0, 0, 18), (204, 30, 2), (0, 0, 19), (54, 10, 2), (0, 0, 4), (0, 100, 93), (0, 0, 4)];
        let lines = requirement_lines(&target, &met);
        assert_eq!(lines.iter().map(|l| l.1).collect::<Vec<_>>(), vec![None, Some(false), None]);
        // (none of these flags)
        assert_eq!(requirement_lines(&[(389, 2, 107)], &[(389, 4)].into()), vec![("Stat 389 has none of flags 2".to_string(), Some(true))]);
    }
}
