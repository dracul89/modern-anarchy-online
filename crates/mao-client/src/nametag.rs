//! The tags over the characters: GUI.dll's `Indicator_t` (made by FUN_10026116, drawn by FUN_1002596c, its health bar
//! FUN_1002575b, its colour FUN_10025649, its place FUN_10025603 / FUN_100258b3) as `TargetingModule_t` keeps them
//! (`SetTarget` 0x10026308, `FrameProcess` 0x10026afc, `HandleNametags` 0x100268ee): one on the selected character with
//! the selection's brackets (`GFX_GUI_INDICATOR_SELECTED`, 230), one on the character fought (`N3Msg_GetAttackingID`) with
//! the attack's (`GFX_GUI_INDICATOR_ATTACKING`, 229), and, with the option `ShowAllNames`, a plain one on every other
//! character near.
//!
//! A tag is a sprite 128 pixels wide (256 when the text is wider), 32 tall, standing in the world 1/128 m a pixel across
//! and 0.3 m high, 0.4 m over the character's mesh (1.9 m when it has none). Its text is the name (in the game shell's
//! bitmap font, white) and, with `IsOrgNameShownOverHead`, the organization's under it in the side's colour (Clan
//! 0x8888aa, Omni 0xaa88aa, else 0x88aaaa); with brackets, the brackets' halves at its ends and a 64-pixel health bar
//! (pixels 14 to 18) in the consider colour over 0x333333.

use crate::theme::Palette;

/// A tag to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Tag {
    /// Where its middle stands (Z up): the top of the head plus 0.4.
    pub at: [f32; 3],
    pub name: String,
    pub org: Option<(String, u8)>,
    /// The selection's (false) or the attack's (true) brackets, with the health bar; none for a plain tag.
    pub brackets: Option<bool>,
    /// Health and its maximum.
    pub health: Option<(i64, i64)>,
    /// `N3Msg_Consider`'s ratio.
    pub consider: f32,
}

/// The height a tag stands over the feet (FUN_10025603: the mesh's height, 1.9 without one, plus 0.4).
pub fn height(mesh: Option<f32>) -> f32 {
    mesh.filter(|h| *h > 0.0).unwrap_or(1.9) + 0.4
}

/// `N3Msg_Consider` (Gamecode 0x100174fe): `(target level − own level + range) / (2 range)` with the range one's own
/// `XPKillRange` (stat 275), when the target is no more than the range below; else (or without a range) −1.
pub fn consider(own_level: i64, range: Option<i64>, target_level: i64) -> f32 {
    match range.filter(|&r| r != 1_234_567_890) {
        Some(r) if own_level - r <= target_level && r != 0 => ((r - own_level) + target_level) as f32 / (r as f32 + r as f32),
        _ => -1.0,
    }
}

/// FUN_10025649's colour (0xRRGGBB): up to 1; below 0 grey; below half green to yellow, from half yellow to red.
pub fn consider_colour(ratio: f32) -> u32 {
    let f = ratio.min(1.0);
    if f < 0.0 {
        0xaa_aaaa
    } else if f < 0.5 {
        ((f as f64 * 255.0 * 2.0) as i32 as u32) << 16 | 0xff00
    } else {
        ((((1.0 - f) as f64 * 240.0 * 2.0) as i32 as u32) | 0xff00) << 8
    }
}

/// The organization line's colour by side (FUN_1002596c).
pub fn org_colour(side: u8) -> u32 {
    match side {
        2 => 0xaa_88aa,
        1 => 0x88_88aa,
        _ => 0x88_aaaa,
    }
}

fn rgb(c: u32) -> egui::Color32 {
    egui::Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// Draws the tags: each projected to the screen, sized by its 0.3 m height there.
pub fn draw(ctx: &egui::Context, painter: &egui::Painter, screen: egui::Rect, tags: &[Tag], project: &dyn Fn(mao_render::glam::Vec3) -> Option<[f32; 2]>, _palette: &Palette) {
    let skin = crate::skin::skin(ctx);
    let font = crate::font::shell(ctx);
    let to_screen = |p: [f32; 2]| egui::pos2(screen.left() + p[0] * screen.width(), screen.top() + p[1] * screen.height());
    for tag in tags {
        let centre = mao_render::glam::Vec3::from(tag.at);
        let (Some(c), Some(low)) = (project(centre), project(centre - mao_render::glam::Vec3::new(0.0, 0.0, 0.3))) else { continue };
        let (c, low) = (to_screen(c), to_screen(low));
        let tall = (low.y - c.y).abs();
        if !(1.0..=400.0).contains(&tall) {
            continue;
        }
        // (pixels of the 32-tall sprite to the screen: 1/128 m across, 0.3/32 m down)
        let per_metre = tall / 0.3;
        let text_width = font.as_ref().map_or(tag.name.len() as f32 * 7.0, |f| f.width(&tag.name));
        let org_width = tag.org.as_ref().map_or(0.0, |(o, _)| font.as_ref().map_or(o.len() as f32 * 7.0, |f| f.width(o)));
        let width_px = if text_width.max(org_width) < 129.0 { 128.0 } else { 256.0 };
        let (sx, sy) = (per_metre / 128.0, per_metre * 0.3 / 32.0);
        let origin = egui::pos2(c.x - width_px * sx * 0.5, c.y - 16.0 * sy);
        let at = |x: f32, y: f32| egui::pos2(origin.x + x * sx, origin.y + y * sy);
        // the brackets: the left half of the picture at the left, the right half at the right
        if let (Some(which), Some(skin)) = (tag.brackets, skin.as_ref()) {
            if let Some(sprite) = skin.get(if which { "GFX_GUI_INDICATOR_ATTACKING" } else { "GFX_GUI_INDICATOR_SELECTED" }) {
                let (w, h) = (sprite.size.x, sprite.size.y);
                let half = (w / 2.0).floor();
                painter.image(sprite.texture.id(), egui::Rect::from_min_max(at(0.0, 0.0), at(half, h)), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(half / w, 1.0)), egui::Color32::WHITE);
                painter.image(sprite.texture.id(), egui::Rect::from_min_max(at(width_px - (w - half), 0.0), at(width_px, h)), egui::Rect::from_min_max(egui::pos2(half / w, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
        }
        // the health bar (FUN_1002575b): 64 pixels across the middle, rows 14 to 18
        if let (Some(_), Some((health, max))) = (tag.brackets, tag.health) {
            let max = max.max(1);
            let filled = (health.clamp(0, max) as f32 * 64.0 / max as f32) as i32 as f32;
            let left = ((width_px - 64.0) / 2.0).floor();
            painter.rect_filled(egui::Rect::from_min_max(at(left, 14.0), at(left + filled, 18.0)), 0.0, rgb(consider_colour(tag.consider)));
            painter.rect_filled(egui::Rect::from_min_max(at(left + filled, 14.0), at(left + 64.0, 18.0)), 0.0, rgb(0x33_3333));
        }
        // the name (row 1) and the organization (row 19), each centred
        let mut line = |text: &str, row: f32, colour: u32| {
            let w = font.as_ref().map_or(text.len() as f32 * 7.0, |f| f.width(text));
            let pos = at((width_px - w) / 2.0, row);
            match font.as_ref() {
                Some(f) => {
                    f.paint_scaled(painter, pos, text, rgb(colour), egui::vec2(sx, sy));
                }
                None => {
                    painter.text(pos, egui::Align2::LEFT_TOP, text, egui::FontId::proportional(12.0 * sy), rgb(colour));
                }
            }
        };
        line(&tag.name, 1.0, 0xff_ffff);
        if let Some((org, side)) = &tag.org {
            line(org, 19.0, org_colour(*side));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consider_runs_green_to_red() {
        // level 10 with a range of 5: a level 5 is 0, level 10 a half, level 15 and up 1 or more; below 5 grey
        assert_eq!(consider(10, Some(5), 5), 0.0);
        assert_eq!(consider(10, Some(5), 10), 0.5);
        assert_eq!(consider(10, Some(5), 15), 1.0);
        assert_eq!(consider(10, Some(5), 4), -1.0);
        assert_eq!(consider(10, None, 10), -1.0);
        assert_eq!(consider_colour(0.0), 0x00ff00);
        assert_eq!(consider_colour(0.25), 0x7fff00);
        assert_eq!(consider_colour(0.5), 0xfff000);
        assert_eq!(consider_colour(1.0), 0xff0000);
        assert_eq!(consider_colour(2.0), 0xff0000);
        assert_eq!(consider_colour(-1.0), 0xaaaaaa);
        assert_eq!(height(None), 2.3);
    }
}
