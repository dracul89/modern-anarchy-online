//! The nano programs window: the programs the character has (the world-entry message's `UploadedNanoIds`), each with its icon and
//! name, searchable. A click casts the program at the target (or at oneself without one: the captured casts, `n3::cast_nano`);
//! a program can be dragged onto a hotbar. Pointing at one shows its description and what it changes.

use std::collections::HashMap;
use std::sync::Arc;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};

use crate::gear::Icon;
use crate::hotbar;
use crate::theme::Palette;

/// The database type of nano programs.
const NANO: u32 = 0xfde85;

pub struct Program {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// The NCU memory it takes (its quality stat).
    pub cost: u32,
    /// What it changes in the stats (`derived::nano_modifiers`).
    pub effects: Vec<(u32, i64)>,
}

/// The programs with their names, icons and effects from the database, by name.
pub fn describe(db: &ResourceDatabase, textures: &crate::Textures, ids: &[u32], level: i64) -> Vec<Program> {
    let mut icons: HashMap<u32, Option<Arc<Icon>>> = HashMap::new();
    let mut list: Vec<Program> = ids
        .iter()
        .map(|&id| {
            let data = db.get(Id { kind: NANO, instance: id }).ok().flatten();
            let template = data.as_deref().and_then(Item::parse);
            let icon = template.as_ref().and_then(|t| t.icon()).and_then(|icon_id| {
                icons
                    .entry(icon_id)
                    .or_insert_with(|| textures.get(item::ICON_TYPE, icon_id).map(|t| Arc::new(Icon { id: icon_id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })))
                    .clone()
            });
            Program {
                id,
                name: template.as_ref().map_or_else(|| format!("Nano #{id}"), |t| t.name.clone()),
                description: template.as_ref().map(|t| t.description.clone()).unwrap_or_default(),
                icon,
                cost: template.as_ref().and_then(|t| t.stat(54)).filter(|&c| c < 999).unwrap_or(0),
                effects: data.as_deref().map(|d| item::nano_modifiers_at(d, level)).unwrap_or_default().into_iter().map(|(s, a)| (s, a as i64)).collect(),
            }
        })
        .collect();
    list.sort_by_key(|p| p.name.to_lowercase());
    list
}

/// The programs whose name contains `filter` (any case).
pub fn matching<'a>(programs: &'a [Program], filter: &str) -> Vec<&'a Program> {
    let filter = filter.trim().to_lowercase();
    programs.iter().filter(|p| filter.is_empty() || p.name.to_lowercase().contains(&filter)).collect()
}

#[derive(Default)]
pub struct ProgramsWindow {
    pub open: bool,
    filter: String,
    textures: HashMap<u32, egui::TextureId>,
}

const ICON: f32 = 28.0;

impl ProgramsWindow {
    /// Shows the window; returns the program clicked (to cast).
    pub fn show(&mut self, ctx: &egui::Context, programs: &[Program], palette: &Palette) -> Option<hotbar::Action> {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut cast = None;
        let mut open = self.open;
        egui::Window::new(egui::RichText::new("Nano programs").color(accent)).open(&mut open).collapsible(false).default_pos(egui::pos2(220.0, 90.0)).default_size([300.0, 420.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Find");
                ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(170.0));
                ui.label(egui::RichText::new(format!("{} programs", programs.len())).small().weak());
            });
            ui.separator();
            let shown = matching(programs, &self.filter);
            if programs.is_empty() {
                ui.label("The character has no nano programs.");
            }
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                for program in shown {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ICON + 4.0), egui::Sense::click_and_drag());
                    let painter = ui.painter();
                    if response.hovered() {
                        painter.rect_filled(rect, 3.0, egui::Color32::from_white_alpha(25));
                    }
                    let icon_rect = egui::Rect::from_min_size(rect.min + egui::vec2(2.0, 2.0), egui::vec2(ICON, ICON));
                    match &program.icon {
                        Some(icon) => {
                            let id = *self.textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("program-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR).id());
                            painter.image(id, icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                        }
                        None => {
                            painter.rect_filled(icon_rect, 3.0, egui::Color32::from_black_alpha(140));
                        }
                    }
                    painter.text(egui::pos2(icon_rect.right() + 8.0, rect.center().y), egui::Align2::LEFT_CENTER, &program.name, egui::FontId::proportional(14.0), egui::Color32::WHITE);
                    if program.cost > 0 {
                        painter.text(egui::pos2(rect.right() - 6.0, rect.center().y), egui::Align2::RIGHT_CENTER, program.cost.to_string(), egui::FontId::proportional(12.0), egui::Color32::from_rgb(120, 200, 170));
                    }
                    let action = hotbar::Action::Nano { id: program.id, name: program.name.clone(), icon: program.icon.as_ref().map(|i| i.id) };
                    response.dnd_set_drag_payload(action.clone());
                    if response.clicked() {
                        cast = Some(action);
                    }
                    response.on_hover_ui(|ui| {
                        ui.set_max_width(320.0);
                        ui.label(egui::RichText::new(&program.name).strong().color(accent));
                        if program.cost > 0 {
                            ui.label(egui::RichText::new(format!("NCU {}", program.cost)).small());
                        }
                        if !program.description.is_empty() {
                            ui.label(program.description.chars().take(420).collect::<String>().replace("\r\n", "\n"));
                        }
                        let effects: Vec<_> = program.effects.iter().filter(|e| e.1 != 0).take(10).collect();
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
                        ui.label(egui::RichText::new("Click to cast, drag to a hotbar").small().weak());
                    });
                }
            });
        });
        self.open = open;
        cast
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(name: &str) -> Program {
        Program { id: 1, name: name.into(), description: String::new(), icon: None, cost: 0, effects: Vec::new() }
    }

    #[test]
    fn the_filter_matches_any_part_of_a_name_in_any_case() {
        let programs = vec![program("Heal Me"), program("Superior Omni-Plating"), program("Healing Wave")];
        assert_eq!(matching(&programs, "heal").len(), 2);
        assert_eq!(matching(&programs, "  PLATING ").len(), 1);
        assert_eq!(matching(&programs, "").len(), 3);
        assert!(matching(&programs, "xyz").is_empty());
    }
}
