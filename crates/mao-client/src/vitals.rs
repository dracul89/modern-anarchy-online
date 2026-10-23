//! The vitals on the game screen: health and nano energy, then the experience bar and the alien experience bar, drawn
//! from the same stats as the Stats window (`stats`). Drag the panel to move it.

use std::collections::BTreeMap;

use crate::stats::{bar, stat};
use crate::theme::Palette;

/// How far a stat has come between two others, 0 to 1, and the text for the bar: `value / limit`.
fn progress(value: Option<i64>, from: i64, to: Option<i64>) -> (Option<i64>, Option<i64>) {
    match (value, to) {
        (Some(v), Some(t)) if t > from => (Some((v - from).max(0)), Some(t - from)),
        _ => (None, None),
    }
}

pub fn show(ctx: &egui::Context, stats: &BTreeMap<u32, i64>, palette: &Palette) {
    if stats.is_empty() {
        return;
    }
    let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
    let get = |s: u32| stats.get(&s).copied();
    egui::Area::new(egui::Id::new("vitals")).movable(true).default_pos(egui::pos2(12.0, 74.0)).show(ctx, |ui| {
        egui::Frame::new().fill(egui::Color32::from_black_alpha(120)).corner_radius(4.0).inner_margin(6.0).show(ui, |ui| {
            ui.set_width(200.0);
            ui.spacing_mut().item_spacing.y = 3.0;
            bar(ui, egui::Color32::from_rgb(190, 40, 40), get(stat::HEALTH), get(stat::MAX_HEALTH));
            bar(ui, egui::Color32::from_rgb(40, 90, 200), get(stat::CURRENT_NANO), get(stat::MAX_NANO));
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
    }
}
