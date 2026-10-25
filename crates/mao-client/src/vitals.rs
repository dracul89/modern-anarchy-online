//! The vitals on the game screen: health and nano energy, then the experience bar and the alien experience bar, drawn
//! from the same stats as the Stats window (`stats`). Drag the panel to move it.

use std::collections::BTreeMap;

use crate::stats::{bar, stat};
use crate::theme::Palette;

/// How far a stat has come between two others, 0 to 1, and the text for the bar: `value / limit`.
fn progress(value: Option<i64>, from: i64, to: Option<i64>) -> (Option<i64>, Option<i64>) {
    match (value, to) {
        // Past the next level's figure (a character at the level cap keeps collecting): nothing meaningful to show.
        (Some(v), Some(t)) if t > from && v <= t => (Some((v - from).max(0)), Some(t - from)),
        _ => (None, None),
    }
}

/// The control center's menus, as in the original's `LeftMenu.xml` and `RightMenu.xml`: (label, window name for `Ui::set_window`, tooltip).
pub const LEFT_MENU: &[(&str, &str, &str)] = &[("Actions", "actions", "Special actions"), ("Character", "stats", "Character: your stats"), ("Nano", "programs", "Nano programs"), ("Inv", "inventory", "Inventory and worn gear")];
pub const RIGHT_MENU: &[(&str, &str, &str)] = &[("NCU", "ncu", "Nano control units"), ("Map", "map", "The map"), ("Mission", "missions", "Missions"), ("Friends", "friends", "Friends list")];

/// The original's bottom control center: the left menu, the bars (health, nano, experience, alien experience) and the right menu in one
/// strip at the bottom of the screen. `open` says which windows are shown; returns the menu entry clicked, to toggle.
pub fn show(ctx: &egui::Context, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, crate::derived::Breakdown>, palette: &Palette, open: &dyn Fn(&str) -> bool) -> Option<&'static str> {
    let mut clicked = None;
    show_inner(ctx, stats, parts, palette, open, &mut clicked);
    clicked
}

fn menu(ui: &mut egui::Ui, entries: &'static [(&'static str, &'static str, &'static str)], accent: egui::Color32, open: &dyn Fn(&str) -> bool, clicked: &mut Option<&'static str>) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
        for pair in entries.chunks(2) {
            ui.horizontal(|ui| {
                for &(label, window, tip) in pair {
                    let text = egui::RichText::new(label).small().color(if open(window) { accent } else { egui::Color32::LIGHT_GRAY });
                    if ui.add_sized([56.0, 28.0], egui::Button::new(text)).on_hover_text(tip).clicked() {
                        *clicked = Some(window);
                    }
                }
            });
        }
    });
}

fn show_inner(ctx: &egui::Context, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, crate::derived::Breakdown>, palette: &Palette, open: &dyn Fn(&str) -> bool, clicked: &mut Option<&'static str>) {
    let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
    let get = |s: u32| stats.get(&s).copied();
    egui::Area::new(egui::Id::unique("vitals")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -4.0)).show(ctx, |ui| {
        egui::Frame::new().fill(egui::Color32::from_black_alpha(140)).corner_radius(4.0).inner_margin(6.0).show(ui, |ui| {
          ui.horizontal(|ui| {
            menu(ui, LEFT_MENU, accent, open, clicked);
            ui.vertical(|ui| {
            ui.set_width(230.0);
            ui.spacing_mut().item_spacing.y = 3.0;
            let health = bar(ui, egui::Color32::from_rgb(190, 40, 40), get(stat::HEALTH), get(stat::MAX_HEALTH));
            crate::stats::hover(health, "Max health", parts.get(&stat::MAX_HEALTH), accent);
            let nano = bar(ui, egui::Color32::from_rgb(40, 90, 200), get(stat::CURRENT_NANO), get(stat::MAX_NANO));
            crate::stats::hover(nano, "Max nano energy", parts.get(&stat::MAX_NANO), accent);
            // Experience towards the next level: from the experience at the last level to the next level's.
            let (value, span) = progress(get(stat::XP), get(stat::LAST_XP).unwrap_or(0), get(stat::NEXT_XP));
            bar(ui, egui::Color32::from_rgb(190, 150, 40), value, span);
            // Alien experience towards the next alien level (the server's own next-level figure).
            let (value, span) = progress(get(stat::ALIEN_XP), 0, get(stat::ALIEN_NEXT_XP));
            bar(ui, egui::Color32::from_rgb(130, 60, 170), value, span);
            if let Some(level) = get(stat::ALIEN_LEVEL).filter(|&l| l > 0) {
                ui.label(egui::RichText::new(format!("Alien level {level}")).small().color(accent));
            }
            });
            menu(ui, RIGHT_MENU, accent, open, clicked);
          });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::progress;

    #[test]
    fn progress_counts_from_the_last_level() {
        assert_eq!(progress(Some(150), 100, Some(300)), (Some(50), Some(200)));
        assert_eq!(progress(Some(50), 100, Some(300)), (Some(0), Some(200)));
        assert_eq!(progress(Some(5), 0, None), (None, None));
        assert_eq!(progress(Some(5), 10, Some(10)), (None, None));
        assert_eq!(progress(Some(500), 100, Some(300)), (None, None));
    }
}
