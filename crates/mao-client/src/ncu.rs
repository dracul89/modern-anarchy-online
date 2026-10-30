//! The NCU window: the nano programs running on the character (what the server lists in the character's own `SimpleCharFullUpdate`:
//! the nano's id and the time it has left), each with its icon, name and a bar for the time left, and the NCU memory in use against
//! the maximum. Pointing at a nano shows what it does (its description and the stat changes it makes).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};

use crate::gear::Icon;
use crate::theme::Palette;

/// The database type of nano programs; the item `stat` of a nano's duration in milliseconds.
const NANO: u32 = 0xfde85;
const DURATION: u32 = 8;

/// One running nano, ready to show.
pub struct NanoInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// The time left when the server told us, the nano's whole duration (both in milliseconds) and when we were told.
    pub remaining_ms: u32,
    pub total_ms: u32,
    pub at: Instant,
    /// What the nano changes in the stats (`derived::nano_modifiers`), for the tooltip.
    pub effects: Vec<(u32, i64)>,
    /// The NCU memory the nano takes: the nano's own quality stat (the sum over the running nanos was the original's current NCU for two
    /// characters, exactly); the free nano programs carry 999 there and take none.
    pub cost: u32,
}

impl NanoInfo {
    /// The time left now, in milliseconds.
    pub fn left(&self, now: Instant) -> u32 {
        self.remaining_ms.saturating_sub(now.saturating_duration_since(self.at).as_millis().min(u32::MAX as u128) as u32)
    }
}

/// The running nanos `(id, time left in milliseconds)` with their names, icons and effects from the database.
pub fn describe(db: &ResourceDatabase, textures: &crate::Textures, running: &[(u32, u32)], level: i64, at: Instant) -> Vec<NanoInfo> {
    let mut icons: HashMap<u32, Option<Arc<Icon>>> = HashMap::new();
    running
        .iter()
        .map(|&(id, remaining_ms)| {
            let data = db.get(Id { kind: NANO, instance: id }).ok().flatten();
            let template = data.as_deref().and_then(Item::parse);
            let icon = template.as_ref().and_then(|t| t.icon()).and_then(|icon_id| {
                icons
                    .entry(icon_id)
                    .or_insert_with(|| textures.get(item::ICON_TYPE, icon_id).map(|t| Arc::new(Icon { id: icon_id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })))
                    .clone()
            });
            let effects = data.as_deref().map(|d| item::nano_modifiers_at(d, level)).unwrap_or_default().into_iter().map(|(s, a)| (s, a as i64)).collect();
            NanoInfo {
                id,
                name: template.as_ref().map_or_else(|| format!("Nano #{id}"), |t| t.name.clone()),
                description: template.as_ref().map(|t| t.description.clone()).unwrap_or_default(),
                icon,
                remaining_ms,
                total_ms: template.as_ref().and_then(|t| t.stat(DURATION)).unwrap_or(0),
                at,
                effects,
                cost: template.as_ref().and_then(|t| t.stat(54)).filter(|&c| c < 999).unwrap_or(0),
            }
        })
        .collect()
}

/// `mm:ss` (or `h:mm:ss`) of a time in milliseconds.
pub fn clock(ms: u32) -> String {
    let seconds = ms / 1000;
    if seconds >= 3600 {
        format!("{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

#[derive(Default)]
pub struct NcuWindow {
    pub open: bool,
    textures: HashMap<u32, egui::TextureHandle>,
}

const ICON: f32 = 32.0;

impl NcuWindow {
    /// Shows the window; returns the program the player chose to cancel (the row's right-click menu).
    pub fn show(&mut self, ctx: &egui::Context, nanos: &[NanoInfo], stats: &std::collections::BTreeMap<u32, i64>, palette: &Palette) -> Option<u32> {
        let mut cancel = None;
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let now = Instant::now();
        let mut open = self.open;
        crate::chrome::window(ctx, palette, "NCU", "NCU", &mut open, |w| w.default_pos(crate::layout::pos("NCU", egui::pos2(560.0, 90.0))).default_width(300.0), |ui| {
            // the memory in use against the maximum (current and maximum NCU)
            // the server does not send the memory in use: the sum of what the running nanos take
            let (used, max) = (stats.get(&180).copied().or_else(|| Some(nanos.iter().map(|n| n.cost as i64).sum())), stats.get(&181).copied());
            crate::stats::bar(ui, egui::Color32::from_rgb(60, 140, 120), used, max);
            ui.label(egui::RichText::new("NCU memory in use").small().weak());
            ui.separator();
            if nanos.is_empty() {
                ui.label("No nano programs are running.");
                return;
            }
            crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(420.0), |ui| {
                for nano in nanos {
                    let left = nano.left(now);
                    let row = ui.horizontal(|ui| {
                        match &nano.icon {
                            Some(icon) => {
                                let id = self.textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("nano-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id();
                                ui.image(egui::load::SizedTexture::new(id, egui::vec2(ICON, ICON)));
                            }
                            None => {
                                ui.allocate_exact_size(egui::vec2(ICON, ICON), egui::Sense::hover());
                            }
                        }
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&nano.name).color(accent));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(clock(left));
                                });
                            });
                            let fraction = if nano.total_ms > 0 { (left as f32 / nano.total_ms as f32).clamp(0.0, 1.0) } else { 0.0 };
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, egui::Color32::from_black_alpha(150));
                            ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction, rect.height())), 2.0, egui::Color32::from_rgb(70, 150, 200));
                        });
                    });
                    let response = row.response.interact(egui::Sense::click());
                    response.context_menu(|ui| {
                        ui.label(egui::RichText::new(&nano.name).strong());
                        if crate::chrome::button(ui, palette, "Cancel this nano", true).clicked() {
                            cancel = Some(nano.id);
                            ui.close();
                        }
                    });
                    response.on_hover_ui(|ui| {
                        ui.set_max_width(320.0);
                        ui.label(egui::RichText::new(&nano.name).strong().color(accent));
                        if nano.total_ms > 0 {
                            ui.label(egui::RichText::new(format!("{} of {} left", clock(left), clock(nano.total_ms))).small());
                        }
                        if !nano.description.is_empty() {
                            ui.label(nano.description.chars().take(420).collect::<String>().replace("\r\n", "\n"));
                        }
                        let effects: Vec<_> = nano.effects.iter().filter(|e| e.1 != 0).collect();
                        if !effects.is_empty() {
                            ui.separator();
                            for &&(stat, amount) in effects.iter().take(10) {
                                ui.horizontal(|ui| {
                                    ui.label(crate::skills::stat_name(stat));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let color = if amount > 0 { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(230, 110, 110) };
                                        ui.label(egui::RichText::new(format!("{amount:+}")).color(color));
                                    });
                                });
                            }
                            if effects.len() > 10 {
                                ui.label(egui::RichText::new(format!("... and {} more", effects.len() - 10)).small().weak());
                            }
                        }
                    });
                    ui.add_space(2.0);
                }
            });
        });
        self.open = open;
        cancel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_reads_minutes_and_hours() {
        assert_eq!(clock(0), "0:00");
        assert_eq!(clock(61_000), "1:01");
        assert_eq!(clock(1_371_193), "22:51");
        assert_eq!(clock(3_725_000), "1:02:05");
    }

    #[test]
    fn the_time_left_counts_down_from_when_we_were_told() {
        let at = Instant::now();
        let nano = NanoInfo { id: 1, name: String::new(), description: String::new(), icon: None, remaining_ms: 10_000, total_ms: 20_000, at, effects: Vec::new(), cost: 0 };
        assert_eq!(nano.left(at), 10_000);
        assert_eq!(nano.left(at + std::time::Duration::from_secs(4)), 6_000);
        assert_eq!(nano.left(at + std::time::Duration::from_secs(60)), 0);
    }
}
