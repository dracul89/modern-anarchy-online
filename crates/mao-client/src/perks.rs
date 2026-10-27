//! The perks window (`P`): the perks the character owns (the world-entry message's `Perks`), each perk once with the rank reached
//! (a perk's ranks are consecutive ids that share a name), its icon, description and what the top rank changes in the stats.
//! A perk id leads, through its record (type `0xf4264`, the item template at byte 12), to an item-type template with the name.

use std::collections::HashMap;
use std::sync::Arc;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};

use crate::gear::Icon;
use crate::theme::Palette;

const PERK_RECORD: u32 = 0xf4264;

pub struct PerkInfo {
    pub name: String,
    /// How many ranks are owned.
    pub rank: u32,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// What the highest owned rank changes (stat, amount).
    pub effects: Vec<(u32, i64)>,
}

/// The template of a perk (its name, icon, effects) from the perk's id.
fn template(db: &ResourceDatabase, perk: u32) -> Option<Vec<u8>> {
    let record = db.get(Id { kind: PERK_RECORD, instance: perk }).ok().flatten()?;
    let template = record.get(12..16).map(|b| u32::from_le_bytes(b.try_into().unwrap()))?;
    db.get(Id { kind: item::TEMPLATE, instance: template }).ok().flatten()
}

/// The owned perks, one entry per name with the number of ranks, by name.
pub fn describe(db: &ResourceDatabase, textures: &crate::Textures, ids: &[u32], level: i64) -> Vec<PerkInfo> {
    let mut icons: HashMap<u32, Option<Arc<Icon>>> = HashMap::new();
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    // name -> (ranks, the highest rank's template data)
    let mut by_name: Vec<(String, u32, Vec<u8>)> = Vec::new();
    for id in sorted {
        let Some(data) = template(db, id) else { continue };
        let Some(parsed) = Item::parse(&data) else { continue };
        match by_name.iter_mut().find(|(name, _, _)| *name == parsed.name) {
            Some(entry) => {
                entry.1 += 1;
                entry.2 = data;
            }
            None => by_name.push((parsed.name.clone(), 1, data)),
        }
    }
    let mut list: Vec<PerkInfo> = by_name
        .into_iter()
        .filter_map(|(name, rank, data)| {
            let parsed = Item::parse(&data)?;
            let icon = parsed.icon().and_then(|icon_id| {
                icons
                    .entry(icon_id)
                    .or_insert_with(|| textures.get(item::ICON_TYPE, icon_id).map(|t| Arc::new(Icon { id: icon_id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })))
                    .clone()
            });
            let effects = item::modifiers_at(&data, level).into_iter().map(|(s, a)| (s, a as i64)).collect();
            Some(PerkInfo { name, rank, description: parsed.description.clone(), icon, effects })
        })
        .collect();
    list.sort_by_key(|p| p.name.to_lowercase());
    list
}

/// The perks whose name contains `filter` (any case).
pub fn matching<'a>(perks: &'a [PerkInfo], filter: &str) -> Vec<&'a PerkInfo> {
    let filter = filter.trim().to_lowercase();
    perks.iter().filter(|p| filter.is_empty() || p.name.to_lowercase().contains(&filter)).collect()
}

#[derive(Default)]
pub struct PerksWindow {
    pub open: bool,
    filter: String,
    textures: HashMap<u32, egui::TextureId>,
}

const ICON: f32 = 28.0;

impl PerksWindow {
    pub fn show(&mut self, ctx: &egui::Context, perks: &[PerkInfo], palette: &Palette) {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut open = self.open;
        crate::chrome::window(ctx, palette, "Perks", "Perks", &mut open, |w| w.default_pos(crate::layout::pos("Perks", egui::pos2(260.0, 100.0))).default_size([320.0, 420.0]), |ui| {
            ui.horizontal(|ui| {
                ui.label("Find");
                ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(170.0));
                ui.label(egui::RichText::new(format!("{} perks", perks.len())).small().weak());
            });
            ui.separator();
            if perks.is_empty() {
                ui.label("The character has no perks.");
            }
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                for perk in matching(perks, &self.filter) {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ICON + 4.0), egui::Sense::hover());
                    let painter = ui.painter();
                    if response.hovered() {
                        painter.rect_filled(rect, 3.0, egui::Color32::from_white_alpha(25));
                    }
                    let icon_rect = egui::Rect::from_min_size(rect.min + egui::vec2(2.0, 2.0), egui::vec2(ICON, ICON));
                    match &perk.icon {
                        Some(icon) => {
                            let id = *self.textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("perk-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR).id());
                            painter.image(id, icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                        }
                        None => {
                            painter.rect_filled(icon_rect, 3.0, egui::Color32::from_black_alpha(140));
                        }
                    }
                    painter.text(egui::pos2(icon_rect.right() + 8.0, rect.center().y), egui::Align2::LEFT_CENTER, &perk.name, egui::FontId::proportional(14.0), egui::Color32::WHITE);
                    if perk.rank > 1 {
                        painter.text(egui::pos2(rect.right() - 6.0, rect.center().y), egui::Align2::RIGHT_CENTER, format!("rank {}", perk.rank), egui::FontId::proportional(12.0), egui::Color32::from_rgb(120, 200, 170));
                    }
                    response.on_hover_ui(|ui| {
                        ui.set_max_width(320.0);
                        ui.label(egui::RichText::new(&perk.name).strong().color(accent));
                        ui.label(egui::RichText::new(format!("Rank {}", perk.rank)).small());
                        if !perk.description.is_empty() {
                            ui.label(perk.description.chars().take(420).collect::<String>().replace("\r\n", "\n"));
                        }
                        let effects: Vec<_> = perk.effects.iter().filter(|e| e.1 != 0).take(10).collect();
                        if !effects.is_empty() {
                            ui.separator();
                            for &&(stat, amount) in &effects {
                                ui.horizontal(|ui| {
                                    ui.label(crate::skills::stat_name(stat));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let color = if amount > 0 { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(230, 110, 110) };
                                        ui.label(egui::RichText::new(format!("{amount:+}")).color(color));
                                    });
                                });
                            }
                        }
                    });
                }
            });
        });
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perk(name: &str, rank: u32) -> PerkInfo {
        PerkInfo { name: name.into(), rank, description: String::new(), icon: None, effects: Vec::new() }
    }

    #[test]
    fn the_filter_matches_any_part_of_a_name() {
        let perks = vec![perk("Bio Shielding", 10), perk("Blunt Mastery", 3), perk("Mutate", 1)];
        assert_eq!(matching(&perks, "bl").len(), 1);
        assert_eq!(matching(&perks, "S").len(), 2);
        assert_eq!(matching(&perks, " mut ")[0].name, "Mutate");
    }
}
