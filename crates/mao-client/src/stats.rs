//! The Stats window, GUI.dll's `StatView_c` (FUN_1008057d), top to bottom:
//!
//! - a line of the name and the level, each a white word and its value in 0xbbbbff (`Name:` FUN_1007ffaa, `Level:`
//!   FUN_1008002b);
//! - the bars, each a `PowerbarView_c` over `GFX_GUI_HOR_BAR_EMPTY` labelled "value / max" (FUN_1007fed3), 3 from the
//!   sides and 2 apart: health in `GFX_GUI_HOR_BAR_RED` (stats 27 of 1, FUN_10080319), nano in `_BLUE`, and those the
//!   player turns on: experience in `_YELLOW` (FUN_1008038f: what is gained of the level's span, shadowknowledge from
//!   level 200), alien experience in `_GREEN` and the duel, solo and team scores in `_PURPLE` (stats 684, 682 and 683,
//!   FUN_1008049c). Which of the five show is `StatViewConfig`'s `AIXPBar`, `XPBar`, `DuelScoreBar`, `TeamScoreBar`
//!   and `SoloScoreBar` (off in the player's own files; ticked in a menu, FUN_1007fe87): here the right button's menu,
//!   kept in `tabs.ini`. Each bar's tip is its name (text.mdb 10000's `TooltipHealth` and so on);
//! - nine rows, the stat's name from text.mdb 2003 and a colon in white at the left, its value in 0xbbbbff at the
//!   right: attack rating (22), then the armour classes melee (91), projectile (90), energy (92), disease (96),
//!   chemical (93), radiation (94), cold (95) and fire (97) (FUN_10080089, FUN_100800e7).
//!
//! The numbers are the stats the server sends and those the client works out (`derived`); one it has not is a dash.
//! A score bar's maximum is the next rank's score ([`next_rank_score`], FUN_1008049c). Resting the pointer on a value
//! still shows what it is made of, which is this client's own.

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

/// The rows under the bars, in the original's order: the stat whose name text.mdb 2003 has, this client's own word
/// for it, and the stat shown.
const ROWS: &[(i32, &str, u32)] = &[
    (22, "Attack rating", stat::ADD_ALL_OFF),
    (91, "Melee AC", stat::MELEE_AC),
    (90, "Projectile AC", stat::PROJECTILE_AC),
    (92, "Energy AC", stat::ENERGY_AC),
    (96, "Disease AC", stat::POISON_AC),
    (93, "Chemical AC", stat::CHEMICAL_AC),
    (94, "Radiation AC", stat::RADIATION_AC),
    (95, "Cold AC", stat::COLD_AC),
    (97, "Fire AC", stat::FIRE_AC),
];

/// The bars the player can turn on, in the order of `StatViewConfig`'s flags (FUN_1007fd7a): the tip's name in
/// text.mdb 10000, this client's own word, and the picture.
const OPTIONAL: [(&str, &str, &str); 5] = [
    ("TooltipAlienExperience", "Alien experience", "GFX_GUI_HOR_BAR_GREEN"),
    ("TooltipExperience", "Experience", "GFX_GUI_HOR_BAR_YELLOW"),
    ("TooltipPVPDuelScore", "Duel score", "GFX_GUI_HOR_BAR_PURPLE"),
    ("TooltipPVPTeamScore", "Team score", "GFX_GUI_HOR_BAR_PURPLE"),
    ("TooltipPVPSoloScore", "Solo score", "GFX_GUI_HOR_BAR_PURPLE"),
];

/// The experience bar's value and span (FUN_1008038f): what is gained since the level began of what the level takes;
/// from level 200 the shadowknowledge (573), counted from nothing unless stat 576's first bit says otherwise.
pub fn experience(stats: &BTreeMap<u32, i64>) -> (Option<i64>, Option<i64>) {
    let get = |s: u32| stats.get(&s).copied();
    let (next, last) = (get(stat::NEXT_XP), get(stat::LAST_XP).unwrap_or(0));
    if get(stat::LEVEL).unwrap_or(1) < 200 {
        (get(stat::XP).map(|x| x - last), next.map(|n| n - last))
    } else {
        let last = if get(0x240).unwrap_or(0) & 1 == 0 { 0 } else { last };
        (get(0x23d).map(|x| x - last), next.map(|n| n - last))
    }
}

/// A bar as the original's `PowerbarView_c`: `full` cut to the value's share over `GFX_GUI_HOR_BAR_EMPTY`, "value /
/// max" in the middle (FUN_1007fed3: the share no more than the whole). Without the pictures, the client's plain bar.
fn picture_bar(ui: &mut egui::Ui, full: &str, value: Option<i64>, max: Option<i64>) -> egui::Response {
    let skin = crate::skin::skin(ui.ctx());
    let Some((skin, empty, fill)) = skin.as_ref().and_then(|s| Some((s, s.get("GFX_GUI_HOR_BAR_EMPTY")?, s.get(full)?))) else {
        return bar(ui, egui::Color32::from_rgb(120, 120, 160), value, max);
    };
    let _ = skin;
    // (3 from the window's sides)
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), empty.size.y), egui::Sense::hover());
    let rect = rect.shrink2(egui::vec2(3.0, 0.0));
    let uv = |to: f32| egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(to, 1.0));
    let painter = ui.painter();
    painter.image(empty.texture.id(), rect, uv(1.0), egui::Color32::WHITE);
    let share = match (value, max) {
        (Some(v), Some(m)) if m > 0 => (v as f32 / m as f32).clamp(0.0, 1.0),
        _ => 0.0,
    };
    if share > 0.0 {
        painter.image(fill.texture.id(), egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * share, rect.height())), uv(share), egui::Color32::WHITE);
    }
    let label = match (value, max) {
        (Some(v), Some(m)) => format!("{} / {}", v.max(0), m.max(0)),
        (Some(v), None) => v.max(0).to_string(),
        _ => "-".to_string(),
    };
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, crate::font::verdana(ui.ctx(), 13.0), egui::Color32::WHITE);
    response
}

#[derive(Default)]
pub struct StatsWindow {
    pub open: bool,
    /// The menu of the bars to show, while it is up.
    popup: Option<crate::gui::popup::PopupMenu>,
}

pub(crate) fn bar(ui: &mut egui::Ui, fill: egui::Color32, value: Option<i64>, max: Option<i64>) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(170));
    // (`PowerbarView_c`'s fill, FUN_1007fed3: the value over the maximum, no more than the whole; the text the two numbers
    // as they are)
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
        let (white, value_colour) = (egui::Color32::WHITE, egui::Color32::from_rgb(0xbb, 0xbb, 0xff));
        // (`StatViewConfig`'s five flags, a bit each)
        let shown = crate::layout::tab("StatViewConfig").unwrap_or(0);
        let tip = |key: &str, own: &str| crate::gui::xml::ldb_named(10000, key, own);
        let mut open = self.open;
        let mut menu_at = None;
        crate::chrome::window(ctx, palette, "Stats", "Stats", &mut open, |w| w.resizable(false).default_pos(crate::layout::pos("Stats", egui::pos2(40.0, 120.0))), |ui| {
            ui.set_width(210.0);
            let area = ui.max_rect();
            let get = |s: u32| stats.get(&s).copied();
            let pair = |ui: &mut egui::Ui, word: &str, value: String| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(egui::RichText::new(word).color(white));
                ui.label(egui::RichText::new(value).color(value_colour));
            };
            ui.horizontal(|ui| {
                ui.add_space(3.0);
                pair(ui, "Name:", if name.is_empty() { "-".to_string() } else { name.to_string() });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(3.0);
                    ui.label(egui::RichText::new(get(stat::LEVEL).map_or("-".to_string(), |l| l.to_string())).color(value_colour));
                    ui.label(egui::RichText::new("Level:").color(white));
                });
            });
            ui.spacing_mut().item_spacing.y = 2.0;
            let framed = picture_bar;
            let health = framed(ui, "GFX_GUI_HOR_BAR_RED", get(stat::HEALTH), get(stat::MAX_HEALTH));
            match parts.get(&stat::MAX_HEALTH) {
                Some(_) => hover(health, "Max health", parts.get(&stat::MAX_HEALTH), accent),
                None => drop(health.on_hover_text(tip("TooltipHealth", "Health"))),
            }
            let nano = framed(ui, "GFX_GUI_HOR_BAR_BLUE", get(stat::CURRENT_NANO), get(stat::MAX_NANO));
            match parts.get(&stat::MAX_NANO) {
                Some(_) => hover(nano, "Max nano energy", parts.get(&stat::MAX_NANO), accent),
                None => drop(nano.on_hover_text(tip("TooltipNano", "Nano"))),
            }
            // (the bars the player turned on, as the original adds them: experience, alien experience, then the scores)
            for k in [1usize, 0, 2, 4, 3] {
                if shown & (1 << k) == 0 {
                    continue;
                }
                let (key, own, picture) = OPTIONAL[k];
                let (value, max) = match k {
                    1 => experience(stats),
                    0 => (get(stat::ALIEN_XP), get(stat::ALIEN_NEXT_XP)),
                    2 => (get(684), get(684).map(next_rank_score)),
                    3 => (get(683), get(683).map(next_rank_score)),
                    _ => (get(682), get(682).map(next_rank_score)),
                };
                framed(ui, picture, value, max).on_hover_text(tip(key, own));
            }
            ui.add_space(4.0);
            ui.spacing_mut().item_spacing.y = 1.0;
            for (key, own, number) in ROWS {
                let label = crate::feedback::texts().and_then(|t| t.get(2003, *key)).unwrap_or(own);
                let row = ui.horizontal(|ui| {
                    ui.add_space(3.0);
                    ui.label(egui::RichText::new(format!("{label}:")).color(white));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(3.0);
                        ui.label(egui::RichText::new(get(*number).map_or("-".to_string(), |v| v.to_string())).color(value_colour));
                    });
                });
                hover(row.response.interact(egui::Sense::hover()), label, parts.get(number), accent);
            }
            if ui.input(|i| i.pointer.secondary_clicked())
                && let Some(at) = ui.input(|i| i.pointer.interact_pos()).filter(|p| area.contains(*p))
            {
                menu_at = Some(at);
            }
        });
        // (the bars to show: a ticked entry each)
        if let Some(at) = menu_at {
            let entries: Vec<(String, bool, Option<bool>)> = OPTIONAL.iter().enumerate().map(|(k, (key, own, _))| (tip(key, own), true, Some(shown & (1 << k) != 0))).collect();
            self.popup = Some(crate::chrome::menu_marked(ctx, at, &entries));
        }
        if let Some(k) = crate::chrome::menu(ctx, palette, "stats-menu", &mut self.popup) {
            crate::layout::set_tab("StatViewConfig", shown ^ (1 << k));
        }
        self.open = open;
    }
}

/// The score of a PvP rank (Gamecode FUN_1003ea89, `N3Msg_GetPVPScoreForRank`): `pow(2500, 1/9)` once as a float
/// (0x1015e758, 0x1015e760), raised to `rank - 1` by repeated float products (FUN_1003dfc0), times 20 (0x1015e750),
/// cut to a whole number (`_ftol`), times 100. Rank 1 is 2000, rank 10 is 4,999,900.
pub fn pvp_score_for_rank(rank: i32) -> i64 {
    let base = 2500f64.powf(0.1111111119389534) as f32;
    let n = rank - 1;
    let mut x = base;
    let mut e = n.unsigned_abs();
    let mut p = 1.0f32;
    loop {
        if e & 1 != 0 {
            p *= x;
        }
        e >>= 1;
        if e == 0 {
            break;
        }
        x *= x;
    }
    if n < 0 {
        p = 1.0 / p;
    }
    (p as f64 * 20.0) as i32 as i64 * 100
}

/// A score's PvP rank (Gamecode FUN_1003eae2): the first rank from 1 to 10 whose score is above it, less one (10 when
/// none is).
pub fn pvp_rank(score: i64) -> i32 {
    let mut rank = 1;
    while rank < 11 {
        if score < pvp_score_for_rank(rank) {
            break;
        }
        rank += 1;
    }
    rank - 1
}

/// The maximum of a score bar (GUI.dll FUN_1008049c): the score of the rank after the score's
/// (`N3Msg_GetPVPScoreForRank(N3Msg_GetPVPRank(stat) + 1)`).
pub fn next_rank_score(score: i64) -> i64 {
    pvp_score_for_rank(pvp_rank(score) + 1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_pvp_ranks_are_gamecodes() {
        use super::*;
        assert_eq!(pvp_score_for_rank(1), 2000);
        // (the float products fall a little short of 2500 at rank 10: 49999.x times 20, cut, as the original's x87 code does)
        assert_eq!(pvp_score_for_rank(10), 4_999_900);
        assert!((2..=10).all(|r| pvp_score_for_rank(r) > pvp_score_for_rank(r - 1)));
        assert_eq!((pvp_rank(0), pvp_rank(1999), pvp_rank(2000), pvp_rank(4_999_900)), (0, 0, 1, 10));
        assert_eq!(next_rank_score(0), 2000);
        assert_eq!(next_rank_score(2500), pvp_score_for_rank(2));
    }

    use super::*;

    #[test]
    fn the_experience_bar_counts_from_the_levels_start() {
        let stats: BTreeMap<u32, i64> = [(stat::LEVEL, 27), (stat::XP, 1500), (stat::LAST_XP, 1000), (stat::NEXT_XP, 3000)].into_iter().collect();
        assert_eq!(experience(&stats), (Some(500), Some(2000)));
        // (from level 200 the shadowknowledge, from nothing)
        let stats: BTreeMap<u32, i64> = [(stat::LEVEL, 205), (stat::XP, 1), (0x23d, 700), (stat::LAST_XP, 400), (stat::NEXT_XP, 3000)].into_iter().collect();
        assert_eq!(experience(&stats), (Some(700), Some(3000)));
        assert_eq!(experience(&BTreeMap::new()), (None, None));
    }
}
