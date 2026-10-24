//! The Stats window: name, level, the health and nano bars and the armour classes, as the original's. The numbers
//! are the stats the server sends (the world-entry message and `Stat` updates), keyed by the game's stat numbers.
//! What the server does not send (armour classes are worked out by the client from the worn items and the
//! active nanos, and so is the attack rating) shows as a dash until this client has that calculation.

use std::collections::BTreeMap;

use crate::derived::Breakdown;
use crate::theme::Palette;

/// Stat numbers (`GameData.Stat`).
pub mod stat {
    pub const MAX_HEALTH: u32 = 0x01;
    pub const HEALTH: u32 = 0x1b;
    pub const LEVEL: u32 = 0x36;
    pub const PROJECTILE_AC: u32 = 0x5a;
    pub const MELEE_AC: u32 = 0x5b;
    pub const ENERGY_AC: u32 = 0x5c;
    pub const CHEMICAL_AC: u32 = 0x5d;
    pub const RADIATION_AC: u32 = 0x5e;
    pub const COLD_AC: u32 = 0x5f;
    pub const POISON_AC: u32 = 0x60;
    pub const FIRE_AC: u32 = 0x61;
    pub const CURRENT_NANO: u32 = 0xd6;
    pub const MAX_NANO: u32 = 0xdd;
    pub const ADD_ALL_OFF: u32 = 0x114;
    pub const XP: u32 = 0x34;
    pub const LAST_XP: u32 = 0x39;
    pub const NEXT_XP: u32 = 0x15e;
    pub const ALIEN_XP: u32 = 0x28;
    pub const ALIEN_NEXT_XP: u32 = 0xb2;
    pub const ALIEN_LEVEL: u32 = 0xa9;
}

/// The rows under the bars, in the original's order.
const ROWS: &[(&str, u32)] = &[
    ("Attack rating:", stat::ADD_ALL_OFF),
    ("Melee/ma AC:", stat::MELEE_AC),
    ("Imp/Proj AC:", stat::PROJECTILE_AC),
    ("Energy AC:", stat::ENERGY_AC),
    ("Disease AC:", stat::POISON_AC),
    ("Chemical AC:", stat::CHEMICAL_AC),
    ("Radiation AC:", stat::RADIATION_AC),
    ("Cold AC:", stat::COLD_AC),
    ("Fire AC:", stat::FIRE_AC),
];

#[derive(Default)]
pub struct StatsWindow {
    pub open: bool,
}

pub(crate) fn bar(ui: &mut egui::Ui, fill: egui::Color32, value: Option<i64>, max: Option<i64>) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(170));
    // A current value above the computed maximum means the maximum is short of something this client does not know yet
    // (the active nano buffs): the bar then counts the current value as the maximum rather than overflowing.
    let max = match (value, max) {
        (Some(v), Some(m)) => Some(m.max(v)),
        (_, m) => m,
    };
    let fraction = match (value, max) {
        (Some(v), Some(m)) if m > 0 => (v as f32 / m as f32).clamp(0.0, 1.0),
        _ => 0.0,
    };
    painter.rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction, rect.height())), 3.0, fill);
    painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(90)), egui::StrokeKind::Inside);
    let text = match (value, max) {
        (Some(v), Some(m)) => format!("{v} / {m}"),
        (Some(v), None) => format!("{v}"),
        _ => "-".to_string(),
    };
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(13.0), egui::Color32::WHITE);
    response
}

/// Shows what a value is made of when the pointer rests on `response`.
pub(crate) fn hover(response: egui::Response, title: &str, parts: Option<&Breakdown>, accent: egui::Color32) {
    if let Some(b) = parts {
        response.on_hover_ui(|ui| crate::skills::breakdown_text(ui, title, b, accent));
    }
}

impl StatsWindow {
    pub fn show(&mut self, ctx: &egui::Context, name: &str, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, Breakdown>, palette: &Palette) {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut open = self.open;
        egui::Window::new(egui::RichText::new("Stats").color(accent)).open(&mut open).collapsible(false).resizable(false).default_pos(crate::layout::pos("Stats", egui::pos2(40.0, 120.0))).show(ctx, |ui| {
            ui.set_width(210.0);
            let get = |s: u32| stats.get(&s).copied();
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.label(egui::RichText::new(if name.is_empty() { "-" } else { name }).color(accent));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(get(stat::LEVEL).map_or("-".to_string(), |l| l.to_string())).color(accent));
                    ui.label("Level:");
                });
            });
            let health = bar(ui, egui::Color32::from_rgb(190, 40, 40), get(stat::HEALTH), get(stat::MAX_HEALTH));
            hover(health, "Max health", parts.get(&stat::MAX_HEALTH), accent);
            let nano = bar(ui, egui::Color32::from_rgb(40, 90, 200), get(stat::CURRENT_NANO), get(stat::MAX_NANO));
            hover(nano, "Max nano energy", parts.get(&stat::MAX_NANO), accent);
            ui.add_space(4.0);
            for (label, number) in ROWS {
                let row = ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(*label).color(accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(get(*number).map_or("-".to_string(), |v| v.to_string()));
                    });
                });
                hover(row.response.interact(egui::Sense::hover()), label.trim_end_matches(':'), parts.get(number), accent);
            }
        });
        self.open = open;
    }
}
