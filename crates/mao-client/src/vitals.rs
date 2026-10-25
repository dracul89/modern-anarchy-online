//! The vitals on the game screen: health and nano energy, then the experience bar and the alien experience bar, drawn
//! from the same stats as the Stats window (`stats`). Drag the panel to move it.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

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

/// The original skin's pictures of the control center (`Graphics.uvga` and its index `Graphics.uvgi` in the game's
/// `cd_image/gui/Default`): the index lists `NAME offset length` and each entry is a whole PNG whose pure green is transparent.
struct Skin {
    sprites: HashMap<String, (egui::TextureHandle, egui::Vec2)>,
}

impl Skin {
    fn load(ctx: &egui::Context) -> Option<Skin> {
        let dir = crate::theme::game_dir()?.join("cd_image/gui/Default");
        let index = String::from_utf8_lossy(&std::fs::read(dir.join("Graphics.uvgi")).ok()?).into_owned();
        let atlas = std::fs::read(dir.join("Graphics.uvga")).ok()?;
        let mut sprites = HashMap::new();
        for line in index.lines().skip(2) {
            let mut parts = line.split_whitespace();
            let (Some(name), Some(offset), Some(length)) = (parts.next(), parts.next().and_then(|v| v.parse::<usize>().ok()), parts.next().and_then(|v| v.parse::<usize>().ok())) else { continue };
            if !(name.starts_with("GFX_GUI_CONTROLCENTER_") || name.starts_with("GFX_GUI_CC_ICONS_") || name.starts_with("GFX_GUI_COMPASS")) {
                continue;
            }
            let Some(bytes) = atlas.get(offset..offset + length) else { continue };
            let Ok(image) = image::load_from_memory(bytes) else { continue };
            let mut rgba = image.to_rgba8();
            for pixel in rgba.pixels_mut() {
                if pixel.0[..3] == [0, 255, 0] {
                    pixel.0 = [0, 0, 0, 0];
                }
            }
            let size = [rgba.width() as usize, rgba.height() as usize];
            let texture = ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()), egui::TextureOptions::LINEAR);
            sprites.insert(name.to_string(), (texture, egui::vec2(size[0] as f32, size[1] as f32)));
        }
        sprites.contains_key("GFX_GUI_CONTROLCENTER_WING_LEFT").then_some(Skin { sprites })
    }

    fn get(&self, name: &str) -> Option<&(egui::TextureHandle, egui::Vec2)> {
        self.sprites.get(name)
    }

    /// Draws a sprite at `at` (its top left), only its lower `fraction` when that is below 1 (a bar filling up), tinted.
    fn draw(&self, painter: &egui::Painter, name: &str, at: egui::Pos2, fraction: f32, tint: egui::Color32) -> Option<egui::Rect> {
        let (texture, size) = self.get(name)?;
        let shown = size.y * fraction;
        let rect = egui::Rect::from_min_size(at + egui::vec2(0.0, size.y - shown), egui::vec2(size.x, shown));
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 1.0 - fraction), egui::pos2(1.0, 1.0));
        painter.image(texture.id(), rect, uv, tint);
        Some(rect)
    }
}

/// The skin of this context, loaded once (None when the game's files are not there).
fn skin(ctx: &egui::Context) -> Option<Arc<Skin>> {
    let id = egui::Id::new("control-center-skin");
    if let Some(found) = ctx.data(|d| d.get_temp::<Option<Arc<Skin>>>(id)) {
        return found;
    }
    let loaded = Skin::load(ctx).map(Arc::new);
    ctx.data_mut(|d| d.insert_temp(id, loaded.clone()));
    loaded
}

/// The compass: the strip (north, east, south, west and north again across 256 pixels) slides behind the frame so the bearing the
/// camera looks along is under the frame's pointers. `yaw` is the camera's (the camera sits at (cos yaw, sin yaw) from the character).
fn compass(ctx: &egui::Context, skin: &Skin, yaw: f32) {
    let (Some((_, strip)), Some((_, frame))) = (skin.get("GFX_GUI_COMPASS_SCALE"), skin.get("GFX_GUI_COMPASS_FRAME")) else { return };
    egui::Area::new(egui::Id::new("cc-compass")).anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 6.0)).interactable(false).show(ctx, |ui| {
        let (rect, _) = ui.allocate_exact_size(*frame, egui::Sense::hover());
        // the bearing, clockwise from north (the map's +z), of the way the camera looks
        let bearing = (-yaw.cos()).atan2(-yaw.sin());
        let fraction = bearing.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
        let inside = rect.shrink2(egui::vec2(6.0, 4.0));
        let painter = ui.painter().with_clip_rect(inside);
        let top = inside.center().y - strip.y * 0.5;
        for copy in [-1.0f32, 0.0, 1.0] {
            skin.draw(&painter, "GFX_GUI_COMPASS_SCALE", egui::pos2(inside.center().x - fraction * strip.x + copy * strip.x, top), 1.0, egui::Color32::WHITE);
        }
        skin.draw(ui.painter(), "GFX_GUI_COMPASS_FRAME", rect.min, 1.0, egui::Color32::WHITE);
    });
}

/// One bar of the original's wings: a vertical groove (`BG`, 12 x 152) with the coloured bar (`FG`) rising in it.
fn wing_bar(ui: &mut egui::Ui, skin: &Skin, painter: &egui::Painter, at: egui::Pos2, name: &str, tint: egui::Color32, value: Option<i64>, max: Option<i64>, label: &str) -> egui::Response {
    let max = match (value, max) {
        (Some(v), Some(m)) => Some(m.max(v)),
        (_, m) => m,
    };
    let fraction = match (value, max) {
        (Some(v), Some(m)) if m > 0 => (v as f32 / m as f32).clamp(0.0, 1.0),
        _ => 0.0,
    };
    let rect = skin.draw(painter, &format!("GFX_GUI_CONTROLCENTER_{name}_BG"), at, 1.0, egui::Color32::WHITE).unwrap_or(egui::Rect::from_min_size(at, egui::vec2(12.0, 152.0)));
    let fg = format!("GFX_GUI_CONTROLCENTER_{name}_FG");
    if let Some((_, size)) = skin.get(&fg) {
        skin.draw(painter, &fg, at + egui::vec2((rect.width() - size.x) * 0.5, rect.height() - size.y - 5.0), fraction, tint);
    }
    let text = match (value, max) {
        (Some(v), Some(m)) => format!("{label}: {v} / {m}"),
        (Some(v), None) => format!("{label}: {v}"),
        _ => format!("{label}: -"),
    };
    ui.interact(rect, ui.id().with(name).with(label), egui::Sense::hover()).on_hover_text(text)
}

/// The original's control center drawn from its own pictures: the left wing (health and nano) and the right wing (experience), with
/// the menu strips under them. False when the pictures are not there.
fn show_skinned(ctx: &egui::Context, yaw: f32, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, crate::derived::Breakdown>, palette: &Palette, open: &dyn Fn(&str) -> bool, clicked: &mut Option<&'static str>) -> bool {
    let Some(skin) = skin(ctx) else { return false };
    let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
    let get = |s: u32| stats.get(&s).copied();
    compass(ctx, &skin, yaw);
    for left in [true, false] {
        let anchor = if left { egui::Align2::LEFT_BOTTOM } else { egui::Align2::RIGHT_BOTTOM };
        let offset = egui::vec2(if left { 5.0 } else { -5.0 }, -5.0);
        egui::Area::new(egui::Id::new(if left { "cc-left" } else { "cc-right" })).anchor(anchor, offset).show(ctx, |ui| {
            let wing = if left { "GFX_GUI_CONTROLCENTER_WING_LEFT" } else { "GFX_GUI_CONTROLCENTER_WING_RIGHT" };
            let strip = if left { "GFX_GUI_CONTROLCENTER_BOTTOM_LEFT" } else { "GFX_GUI_CONTROLCENTER_BOTTOM_RIGHT" };
            let (Some((_, wing_size)), Some((_, strip_size))) = (skin.get(wing), skin.get(strip)) else { return };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(strip_size.x.max(wing_size.x), wing_size.y + strip_size.y), egui::Sense::hover());
            let painter = ui.painter().clone();
            let wing_at = if left { rect.min } else { egui::pos2(rect.right() - wing_size.x, rect.top()) };
            skin.draw(&painter, wing, wing_at, 1.0, egui::Color32::WHITE);
            skin.draw(&painter, strip, egui::pos2(rect.left(), rect.top() + wing_size.y), 1.0, egui::Color32::WHITE);
            // the bars stand in the wing's tall side, on its floor
            let top = wing_at.y + wing_size.y - 152.0 - 4.0;
            if left {
                let health = wing_bar(ui, &skin, &painter, egui::pos2(wing_at.x + 8.0, top), "HEALTH_BAR", egui::Color32::WHITE, get(stat::HEALTH), get(stat::MAX_HEALTH), "Health");
                crate::stats::hover(health, "Max health", parts.get(&stat::MAX_HEALTH), accent);
                let nano = wing_bar(ui, &skin, &painter, egui::pos2(wing_at.x + 24.0, top), "NANO_BAR", egui::Color32::WHITE, get(stat::CURRENT_NANO), get(stat::MAX_NANO), "Nano");
                crate::stats::hover(nano, "Max nano energy", parts.get(&stat::MAX_NANO), accent);
            } else {
                let (value, span) = progress(get(stat::XP), get(stat::LAST_XP).unwrap_or(0), get(stat::NEXT_XP));
                wing_bar(ui, &skin, &painter, egui::pos2(wing_at.x + wing_size.x - 20.0, top), "XP_BAR", egui::Color32::WHITE, value, span, "Experience");
                if get(stat::ALIEN_LEVEL).is_some_and(|l| l > 0) {
                    let (value, span) = progress(get(stat::ALIEN_XP), 0, get(stat::ALIEN_NEXT_XP));
                    wing_bar(ui, &skin, &painter, egui::pos2(wing_at.x + wing_size.x - 36.0, top), "XP_BAR", egui::Color32::from_rgb(190, 110, 255), value, span, "Alien experience");
                }
            }
            // the menu's icons lie along the strip
            let entries = if left { LEFT_MENU } else { RIGHT_MENU };
            let icons: &[&str] = if left { &["ACTIONS", "SKILLS", "NANO", "INVENTORY"] } else { &["NCU", "MAP", "MISSIONS", "FRIENDS"] };
            for (i, (&(_, window, tip), icon)) in entries.iter().zip(icons).enumerate() {
                let name = format!("GFX_GUI_CC_ICONS_{icon}");
                let Some((_, size)) = skin.get(&name) else { continue };
                let at = egui::pos2(rect.left() + 10.0 + i as f32 * (size.x + 1.0), rect.top() + wing_size.y + (strip_size.y - size.y) * 0.5);
                let response = ui.interact(egui::Rect::from_min_size(at, *size), ui.id().with(window), egui::Sense::click()).on_hover_text(tip);
                let tint = if open(window) { accent } else if response.hovered() { egui::Color32::WHITE } else { egui::Color32::from_gray(200) };
                skin.draw(&painter, &name, at, 1.0, tint);
                if response.clicked() {
                    *clicked = Some(window);
                }
            }
        });
    }
    true
}

/// The control center's menus, as in the original's `LeftMenu.xml` and `RightMenu.xml`: (label, window name for `Ui::set_window`, tooltip).
pub const LEFT_MENU: &[(&str, &str, &str)] = &[("Actions", "actions", "Special actions"), ("Character", "stats", "Character: your stats"), ("Nano", "programs", "Nano programs"), ("Inv", "inventory", "Inventory and worn gear")];
pub const RIGHT_MENU: &[(&str, &str, &str)] = &[("NCU", "ncu", "Nano control units"), ("Map", "map", "The map"), ("Mission", "missions", "Missions"), ("Friends", "friends", "Friends list")];

/// The original's bottom control center: the left menu, the bars (health, nano, experience, alien experience) and the right menu in one
/// strip at the bottom of the screen. `open` says which windows are shown; returns the menu entry clicked, to toggle.
pub fn show(ctx: &egui::Context, yaw: f32, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, crate::derived::Breakdown>, palette: &Palette, open: &dyn Fn(&str) -> bool) -> Option<&'static str> {
    let mut clicked = None;
    if show_skinned(ctx, yaw, stats, parts, palette, open, &mut clicked) {
        return clicked;
    }
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
