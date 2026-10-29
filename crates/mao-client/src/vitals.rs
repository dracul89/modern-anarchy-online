//! The control center at the bottom of the game screen, drawn from the original skin's pictures (`skin`): the wings with the
//! health, nano, experience and alien experience bars (the same stats as the Stats window), the menu strips, the compass and
//! the target buttons. Without the pictures a plain strip shows the same bars and menus.

use std::collections::BTreeMap;

use crate::skin::Skin;
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

/// The compass: the strip (north, east, south, west and north again across 256 pixels) slides behind the frame so the bearing the
/// camera looks along is under the frame's pointers. `yaw` is the camera's (the camera sits at (cos yaw, sin yaw) from the character).
fn compass(ctx: &egui::Context, skin: &Skin, yaw: f32) {
    let (Some(strip), Some(frame)) = (skin.get("GFX_GUI_COMPASS_SCALE"), skin.get("GFX_GUI_COMPASS_FRAME")) else { return };
    let (strip, frame) = (strip.size, frame.size);
    egui::Area::new(egui::Id::new("cc-compass")).anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 6.0)).interactable(false).show(ctx, |ui| {
        let (rect, _) = ui.allocate_exact_size(frame, egui::Sense::hover());
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

/// The target buttons beside the menu strips: the left one targets the character itself (the original's F1), the right one the next
/// hostile (Tab). Drawn from `TARGET_BUTTON` (its second and third pictures are hovered and pressed) and the icon on it.
fn target_buttons(ctx: &egui::Context, skin: &Skin, clicked: &mut Option<&'static str>) {
    // (beside the menu strips, whose width is the skin's: 214 in the game's own, 280 and 205 in the AOU skins)
    let strip = |name: &str| skin.get(name).map_or(214.0, |s| s.size.x);
    let (left_strip, right_strip) = (strip("GFX_GUI_CONTROLCENTER_BOTTOM_LEFT"), strip("GFX_GUI_CONTROLCENTER_BOTTOM_RIGHT"));
    for left in [true, false] {
        let (anchor, x, window, icon, tip) = if left {
            (egui::Align2::LEFT_BOTTOM, 5.0 + left_strip + 10.0, "target_self", "GFX_GUI_CONTROLCENTER_TARGET_ICON_SELF", "Target yourself")
        } else {
            (egui::Align2::RIGHT_BOTTOM, -(5.0 + right_strip + 10.0), "target_next", "GFX_GUI_CONTROLCENTER_TARGET_ICON_OTHER", "Target the next enemy")
        };
        let Some(size) = skin.get("GFX_GUI_CONTROLCENTER_TARGET_BUTTON").map(|s| s.size) else { return };
        egui::Area::new(egui::Id::new(window)).anchor(anchor, egui::vec2(x, -8.0)).show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            let state = if response.is_pointer_button_down_on() { "_STATE3" } else if response.hovered() { "_STATE2" } else { "" };
            skin.draw(ui.painter(), &format!("GFX_GUI_CONTROLCENTER_TARGET_BUTTON{state}"), rect.min, 1.0, egui::Color32::WHITE);
            if let Some(icon_size) = skin.get(icon).map(|s| s.size) {
                skin.draw(ui.painter(), icon, rect.center() - icon_size * 0.5, 1.0, egui::Color32::WHITE);
            }
            if response.on_hover_text(tip).clicked() {
                *clicked = Some(window);
            }
        });
    }
}

/// What an entry of a control center sub-menu does.
#[derive(Clone, Copy)]
enum Entry {
    /// Toggle a window (the names of `Ui::set_window`, and "options").
    Window(&'static str),
    /// Open another menu.
    Menu(&'static str),
    /// A camera mode: "camera_1st", "camera_3rd", or the third-person follow "camera_lock" / "camera_rubber" / "camera_trail".
    Camera(&'static str),
}

/// The sub-menus of the original's `CommandMenu.xml`, `CharMenu.xml`, `MapMenu.xml`, `CameraMenu.xml` and `Windows.xml`, as far as this
/// client has the windows (no pets, research, faction or planet map yet). The second field is the menu to go back to.
fn menu_items(menu: &str) -> (Option<&'static str>, &'static [(&'static str, Entry)]) {
    match menu {
        "command" => (None, &[("Emotes", Entry::Window("actions")), ("Camera", Entry::Menu("camera")), ("Stats", Entry::Window("stats")), ("Control panel", Entry::Window("actions")), ("Help", Entry::Menu("help"))]),
        "camera" => (Some("command"), &[("1st person", Entry::Camera("camera_1st")), ("3rd person", Entry::Camera("camera_3rd")), ("3rd person locked", Entry::Camera("camera_lock")), ("3rd person rubber", Entry::Camera("camera_rubber")), ("3rd person trailing", Entry::Camera("camera_trail"))]),
        "help" => (Some("command"), &[("Help", Entry::Window("help")), ("Options", Entry::Window("options"))]),
        "char" => (None, &[("Skills", Entry::Window("skills")), ("Perks", Entry::Window("perks"))]),
        "map" => (None, &[("Map", Entry::Window("map"))]),
        _ => (None, &[]),
    }
}

/// The menu a control center icon opens, if it opens one.
fn icon_menu(window: &str) -> Option<&'static str> {
    match window {
        "actions" => Some("command"),
        "stats" => Some("char"),
        "map" => Some("map"),
        _ => None,
    }
}

/// The open sub-menu (which one and where its bottom left corner is), kept in the context.
fn menu_state(ctx: &egui::Context) -> Option<(&'static str, egui::Pos2)> {
    ctx.data(|d| d.get_temp::<Option<(&'static str, egui::Pos2)>>(egui::Id::new("cc-menu"))).flatten()
}

fn set_menu_state(ctx: &egui::Context, state: Option<(&'static str, egui::Pos2)>) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("cc-menu"), state));
}

/// The open sub-menu as a list over its icon; a click on a window entry or a camera mode closes it and is returned like an icon's.
fn sub_menu(ctx: &egui::Context, accent: egui::Color32, open: &dyn Fn(&str) -> bool, clicked: &mut Option<&'static str>) {
    // MAO_CC_MENU=command|char|map|camera opens that menu in the previews.
    let preview = std::env::var("MAO_CC_MENU").ok().and_then(|m| ["command", "camera", "help", "char", "map"].into_iter().find(|n| *n == m)).map(|m| (m, egui::pos2(15.0, ctx.content_rect().bottom() - 25.0)));
    let Some((menu, at)) = menu_state(ctx).or(preview) else { return };
    let (back, items) = menu_items(menu);
    let mut next = Some((menu, at));
    let response = egui::Area::new(egui::Id::new("cc-popup")).order(egui::Order::Foreground).fixed_pos(at).pivot(egui::Align2::LEFT_BOTTOM).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            if let Some(parent) = back {
                if ui.add_sized([140.0, 22.0], egui::Button::new(egui::RichText::new("< Back").weak())).clicked() {
                    next = Some((parent, at));
                }
            }
            for &(label, entry) in items {
                let lit = matches!(entry, Entry::Window(w) if open(w));
                let text = egui::RichText::new(match entry { Entry::Menu(_) => format!("{label} >"), _ => label.to_string() }).color(if lit { accent } else { egui::Color32::LIGHT_GRAY });
                if ui.add_sized([140.0, 22.0], egui::Button::new(text)).clicked() {
                    match entry {
                        Entry::Window(w) => {
                            *clicked = Some(w);
                            next = None;
                        }
                        Entry::Camera(c) => {
                            *clicked = Some(c);
                            next = None;
                        }
                        Entry::Menu(m) => next = Some((m, at)),
                    }
                }
            }
        });
    });
    // A press anywhere else closes it (an icon's own click then opens its menu again).
    if ctx.input(|i| i.pointer.any_pressed()) && !response.response.contains_pointer() {
        next = None;
    }
    set_menu_state(ctx, next);
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
    if let Some(size) = skin.get(&fg).map(|s| s.size) {
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
fn show_skinned(ctx: &egui::Context, yaw: f32, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, crate::derived::Breakdown>, palette: &Palette, open: &dyn Fn(&str) -> bool, menus: bool, clicked: &mut Option<&'static str>) -> bool {
    let Some(skin) = crate::skin::skin(ctx) else { return false };
    let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
    let get = |s: u32| stats.get(&s).copied();
    compass(ctx, &skin, yaw);
    if menus {
        sub_menu(ctx, accent, open, clicked);
    }
    // The rollup (the original's `cc_rollup_panel` button): folds the wings and their bars away, leaving the menu strips.
    let rolled_id = egui::Id::new("cc-rolled");
    let mut rolled = ctx.data(|d| d.get_temp::<bool>(rolled_id)).unwrap_or(false);
    for left in [true, false] {
        let anchor = if left { egui::Align2::LEFT_BOTTOM } else { egui::Align2::RIGHT_BOTTOM };
        let offset = egui::vec2(if left { 5.0 } else { -5.0 }, -5.0);
        egui::Area::new(egui::Id::new(if left { "cc-left" } else { "cc-right" })).anchor(anchor, offset).show(ctx, |ui| {
            let wing = if left { "GFX_GUI_CONTROLCENTER_WING_LEFT" } else { "GFX_GUI_CONTROLCENTER_WING_RIGHT" };
            let strip = if left { "GFX_GUI_CONTROLCENTER_BOTTOM_LEFT" } else { "GFX_GUI_CONTROLCENTER_BOTTOM_RIGHT" };
            let (Some(wing_size), Some(strip_size)) = (skin.get(wing).map(|s| s.size), skin.get(strip).map(|s| s.size)) else { return };
            let wing_h = if rolled { 0.0 } else { wing_size.y };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(strip_size.x.max(wing_size.x), wing_h + strip_size.y), egui::Sense::hover());
            let painter = ui.painter().clone();
            let wing_at = if left { rect.min } else { egui::pos2(rect.right() - wing_size.x, rect.top()) };
            if !rolled {
                skin.draw(&painter, wing, wing_at, 1.0, egui::Color32::WHITE);
            }
            skin.draw(&painter, strip, egui::pos2(rect.left(), rect.top() + wing_h), 1.0, egui::Color32::WHITE);
            // the rollup button sits on the right strip's end
            if !left {
                let size = egui::vec2(15.0, 15.0);
                // (at the strip's left end, clear of the bars)
                let at = egui::pos2(rect.left() + 6.0, rect.top() + wing_h - size.y - 3.0);
                let response = ui.interact(egui::Rect::from_min_size(at, size), ui.id().with("rollup"), egui::Sense::click());
                let state = if response.is_pointer_button_down_on() { 3 } else if response.hovered() { 2 } else { 1 };
                skin.draw(&painter, &format!("GFX_GUI_ROLLUP_{}_STATE{state}", if rolled { "EXPAND" } else { "COLLAPSE" }), at, 1.0, accent);
                if response.on_hover_text(if rolled { "Show the control center" } else { "Fold the control center away" }).clicked() {
                    rolled = !rolled;
                    ctx.data_mut(|d| d.insert_temp(rolled_id, rolled));
                }
            }
            // the bars stand in the wing's tall side, on its floor
            let top = wing_at.y + wing_size.y - 152.0 - 4.0;
            if rolled {
            } else if left {
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
            for (i, (&(_, window, tip), icon)) in entries.iter().zip(icons).enumerate().filter(|_| menus) {
                let name = format!("GFX_GUI_CC_ICONS_{icon}");
                let Some(size) = skin.get(&name).map(|s| s.size) else { continue };
                let at = egui::pos2(rect.left() + 10.0 + i as f32 * (size.x + 1.0), rect.top() + wing_h + (strip_size.y - size.y) * 0.5);
                let response = ui.interact(egui::Rect::from_min_size(at, size), ui.id().with(window), egui::Sense::click()).on_hover_text(tip);
                let tint = if open(window) { accent } else if response.hovered() { egui::Color32::WHITE } else { egui::Color32::from_gray(200) };
                skin.draw(&painter, &name, at, 1.0, tint);
                if response.clicked() {
                    match icon_menu(window) {
                        Some(menu) => set_menu_state(ctx, Some((menu, egui::pos2(at.x, rect.top() + wing_h - 2.0)))),
                        None => *clicked = Some(window),
                    }
                }
            }
        });
    }
    // (after the wings, which the AOU skins' larger ones would otherwise cover)
    target_buttons(ctx, &skin, clicked);
    true
}

/// The control center's menus, as in the original's `LeftMenu.xml` and `RightMenu.xml`: (label, window name for `Ui::set_window`, tooltip).
pub const LEFT_MENU: &[(&str, &str, &str)] = &[("Actions", "actions", "Special actions"), ("Character", "stats", "Character: your stats"), ("Nano", "programs", "Nano programs"), ("Inv", "inventory", "Inventory and worn gear")];
pub const RIGHT_MENU: &[(&str, &str, &str)] = &[("NCU", "ncu", "Nano control units"), ("Map", "map", "The map"), ("Mission", "missions", "Missions"), ("Friends", "friends", "Friends list")];

/// The original's bottom control center: the left menu, the bars (health, nano, experience, alien experience) and the right menu in one
/// strip at the bottom of the screen. `open` says which windows are shown; returns the menu entry clicked, to toggle. With
/// `menus` false the skin's own control center (`gui::controlcenter`) has the menus, and only the wings and bars are drawn.
pub fn show(ctx: &egui::Context, yaw: f32, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, crate::derived::Breakdown>, palette: &Palette, open: &dyn Fn(&str) -> bool, menus: bool) -> Option<&'static str> {
    let mut clicked = None;
    if show_skinned(ctx, yaw, stats, parts, palette, open, menus, &mut clicked) {
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
