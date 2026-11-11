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

/// The bearing (clockwise from north) of the mission's waypoint when it is in this playfield: the compass's yellow indicator
/// ("MissionShownAsDot"), set each frame by the interface.
pub static WAYPOINT_BEARING: std::sync::Mutex<Option<f32>> = std::sync::Mutex::new(None);

/// The compass: the strip (north, east, south, west and north again across 256 pixels) slides behind the frame so the bearing the
/// camera looks along is under the frame's pointers. `yaw` is the camera's (the camera sits at (cos yaw, sin yaw) from the character).
fn compass(ctx: &egui::Context, skin: &Skin, yaw: f32) {
    let (Some(strip), Some(frame)) = (skin.get("GFX_GUI_COMPASS_SCALE"), skin.get("GFX_GUI_COMPASS_FRAME")) else { return };
    let (strip, frame) = (strip.size, frame.size);
    // (GUI.dll's `compass_window`: a window of its own, dragged anywhere; it starts at the top's middle)
    let start = egui::pos2(ctx.content_rect().center().x - frame.x * 0.5, 6.0);
    egui::Area::new(egui::Id::unique("compass")).default_pos(crate::layout::pos("compass", start)).movable(true).show(ctx, |ui| {
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
        // (the waypoint: a yellow mark at its bearing on the strip)
        if let Some(b) = WAYPOINT_BEARING.lock().ok().and_then(|w| *w) {
            let mut off = (b.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU - fraction) * strip.x;
            off = (off + strip.x * 0.5).rem_euclid(strip.x) - strip.x * 0.5;
            let x = inside.center().x + off;
            let (y0, y1) = (inside.top(), inside.bottom());
            let yellow = egui::Color32::from_rgb(255, 220, 40);
            painter.add(egui::Shape::convex_polygon(vec![egui::pos2(x - 5.0, y0), egui::pos2(x + 5.0, y0), egui::pos2(x, y0 + 7.0)], yellow, egui::Stroke::new(1.0, egui::Color32::BLACK)));
            painter.add(egui::Shape::convex_polygon(vec![egui::pos2(x - 5.0, y1), egui::pos2(x, y1 - 7.0), egui::pos2(x + 5.0, y1)], yellow, egui::Stroke::new(1.0, egui::Color32::BLACK)));
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
        egui::Area::new(egui::Id::unique(window)).anchor(anchor, egui::vec2(x, -8.0)).show(ctx, |ui| {
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
    ctx.data(|d| d.get_temp::<Option<(&'static str, egui::Pos2)>>(egui::Id::unique("cc-menu"))).flatten()
}

fn set_menu_state(ctx: &egui::Context, state: Option<(&'static str, egui::Pos2)>) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique("cc-menu"), state));
}

/// The open sub-menu as a list over its icon; a click on a window entry or a camera mode closes it and is returned like an icon's.
fn sub_menu(ctx: &egui::Context, accent: egui::Color32, open: &dyn Fn(&str) -> bool, clicked: &mut Option<&'static str>) {
    // MAO_CC_MENU=command|char|map|camera opens that menu in the previews.
    let preview = std::env::var("MAO_CC_MENU").ok().and_then(|m| ["command", "camera", "help", "char", "map"].into_iter().find(|n| *n == m)).map(|m| (m, egui::pos2(15.0, ctx.content_rect().bottom() - 25.0)));
    let Some((menu, at)) = menu_state(ctx).or(preview) else { return };
    let (back, items) = menu_items(menu);
    let mut next = Some((menu, at));
    let response = egui::Area::new(egui::Id::unique("cc-popup")).order(egui::Order::Foreground).fixed_pos(at).pivot(egui::Align2::LEFT_BOTTOM).show(ctx, |ui| {
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

/// A `CharBarWindow_c` holding a `CharacterBar_c` (GUI.dll FUN_1006b8d1, FUN_100673ba): the health, nano, experience or
/// alien experience bar as a window of its own the player drags anywhere (its place kept under the original's config
/// name, `CCHealthBarConfig` ...). The bar is a `PowerbarView_c` that fills upwards (direction 3), made of four of the
/// skin's pictures (FUN_100673ba's bitmap numbers, named by AFCM.dll's table): `GFX_GUI_ACTIONVIEW_<kind>BAR_BACKGROUND`
/// stretched between the end pieces `_LEFT` (at the top) and `_RIGHT` (at the bottom), and `GFX_GUI_ACTIONVIEW_<kind>BAR`
/// at its own size in the middle of the background's, kept from the bottom up as far as the value goes
/// (`PowerbarView_c::Initialize` and `Recalculate`, as `gui::powerbar`). The value and maximum are its tooltip
/// (FUN_10066f7a: `%d / %d`), not a text on it. `at`: where it stands when the player has not moved it, in its wing of
/// the control center. (Until 2026-11-11 this drew the horizontal `GFX_GUI_HOR_BAR_*` pictures, a guess, the full one
/// at the empty one's corner though it is smaller, with the numbers on it.)
fn char_bar(ctx: &egui::Context, skin: &Skin, at: impl Fn(egui::Vec2) -> egui::Pos2, name: &str, kind: &str, value: Option<i64>, max: Option<i64>, label: &str) -> Option<egui::Response> {
    let picture = |part: &str| skin.get(&format!("GFX_GUI_ACTIONVIEW_{kind}BAR{part}"));
    let (full, background) = (picture(""), picture("_BACKGROUND"));
    let (top, bottom) = (picture("_LEFT"), picture("_RIGHT"));
    // (`CalculatePreferredSize`: the background's size, else the full picture's, and the end pieces' heights)
    let body = background.as_ref().or(full.as_ref()).map_or(egui::vec2(12.0, 152.0), |s| s.size);
    let caps = [&top, &bottom].map(|c| c.as_ref().map_or(egui::Vec2::ZERO, |s| s.size));
    let size = egui::vec2(body.x, body.y + caps[0].y + caps[1].y);
    // (a value over the maximum, which is short of what the client does not know yet, counts as full)
    let fraction = match (value, max) {
        (Some(v), Some(m)) if m > 0 => (v as f32 / m as f32).clamp(0.0, 1.0),
        _ => 0.0,
    };
    let mut out = None;
    // (with no place kept it stands where `at` says, which follows the control center as that is laid out, until the
    // player drags it: its first frame's place would be the not yet laid out wing's)
    let id = egui::Id::unique(name);
    let moved = ctx.data(|d| d.get_temp::<bool>(id.with("moved")).unwrap_or(false));
    let area = egui::Area::new(id).movable(true);
    let area = match crate::layout::saved(name) {
        Some(kept) => area.default_pos(kept),
        None if !moved => area.current_pos(at(size)),
        None => area,
    };
    let shown = area.show(ctx, |ui| {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
        let painter = ui.painter();
        let white = egui::Color32::WHITE;
        let inner = egui::Rect::from_min_max(rect.min + egui::vec2(0.0, caps[0].y), rect.max - egui::vec2(0.0, caps[1].y));
        let whole = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        if let Some(c) = &top {
            painter.image(c.texture.id(), egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), caps[0].y)), whole, white);
        }
        if let Some(c) = &bottom {
            painter.image(c.texture.id(), egui::Rect::from_min_max(egui::pos2(rect.left(), inner.bottom()), rect.max), whole, white);
        }
        match &background {
            Some(b) => drop(painter.image(b.texture.id(), inner, whole, white)),
            None => drop(painter.rect_filled(inner, 0.0, egui::Color32::from_black_alpha(160))),
        }
        if let Some(f) = &full {
            let bar = crate::gui::powerbar::PowerBar { direction: crate::gui::powerbar::Direction::Up, background: None, full: None, start_cap: None, end_cap: None, value: fraction, colour: white };
            let kept = bar.kept(f.size);
            // (in the middle of the background's size, the halves rounded down)
            let offset = egui::vec2(((body.x - f.size.x) * 0.5).floor(), ((body.y - f.size.y) * 0.5).floor());
            if fraction > 0.0 && kept.width() > 0.0 && kept.height() > 0.0 {
                let uv = egui::Rect::from_min_max(egui::pos2(kept.left() / f.size.x, kept.top() / f.size.y), egui::pos2(kept.right() / f.size.x, kept.bottom() / f.size.y));
                painter.image(f.texture.id(), kept.translate(inner.min.to_vec2() + offset), uv, white);
            }
        }
        let text = match (value, max) {
            (Some(v), Some(m)) => format!("{v} / {m}"),
            (Some(v), None) => v.to_string(),
            _ => String::new(),
        };
        out = Some(response.on_hover_text(format!("{label}: {text}")));
    });
    if shown.response.dragged() {
        ctx.data_mut(|d| d.insert_temp(id.with("moved"), true));
    }
    out
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
    ui.interact(rect, ui.scope_id().with(name).with(label), egui::Sense::hover()).on_hover_text(text)
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
    let rolled_id = egui::Id::unique("cc-rolled");
    let mut rolled = ctx.data(|d| d.get_temp::<bool>(rolled_id)).unwrap_or(false);
    for left in [true, false] {
        let anchor = if left { egui::Align2::LEFT_BOTTOM } else { egui::Align2::RIGHT_BOTTOM };
        let offset = egui::vec2(if left { 5.0 } else { -5.0 }, -5.0);
        egui::Area::new(egui::Id::unique(if left { "cc-left" } else { "cc-right" })).anchor(anchor, offset).show(ctx, |ui| {
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
                let response = ui.interact(egui::Rect::from_min_size(at, size), ui.scope_id().with("rollup"), egui::Sense::click());
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
                let response = ui.interact(egui::Rect::from_min_size(at, size), ui.scope_id().with(window), egui::Sense::click()).on_hover_text(tip);
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

/// Where the skin's control center (`gui::controlcenter`, its `ControlCenter.xml`) placed the parts this file draws, on screen,
/// and which of them its distributed values show: a dock is `None` when its view is hidden.
#[derive(Clone, Default)]
pub struct Docks {
    pub left_wing: Option<egui::Rect>,
    pub right_wing: Option<egui::Rect>,
    pub left_bar: Option<egui::Rect>,
    pub right_bar: Option<egui::Rect>,
    pub rollup: Option<egui::Rect>,
    pub left_target: Option<egui::Rect>,
    pub right_target: Option<egui::Rect>,
    /// `cc_section1 && cc_compass`, `cc_health_bar`, `cc_nano_bar`, `cc_xp_bar`, `cc_alienxp_bar`.
    pub compass: bool,
    pub health: bool,
    pub nano: bool,
    pub xp: bool,
    pub alien_xp: bool,
    /// The wings are folded away (the rollup's state).
    pub rolled: bool,
}

/// What the docked parts were asked to do: a target button's or the rollup's click.
#[derive(Default)]
pub struct DockClicks {
    pub target: Option<&'static str>,
    pub rollup: bool,
    /// The AGG/DEF slider let go at this value.
    pub aggdef: Option<i32>,
}

/// `FormatNumeric`: the number with the thousands grouped (the user's locale's grouping; commas here).
fn grouped(v: i64) -> String {
    let digits = v.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 { format!("-{out}") } else { out }
}

/// A strip's text (the strips' `TextView_c`s, font 5: NORMAL), its left edge at `x`, centred in `row` from top to bottom;
/// its width.
fn strip_text(painter: &egui::Painter, m: &crate::gui::font::Measure, x: f32, row: egui::Rect, text: &str, colour: egui::Color32) -> f32 {
    let font = crate::gui::font::FontId::NORMAL;
    let w = m.width(font, text);
    painter.text(egui::pos2(x, row.center().y), egui::Align2::LEFT_CENTER, text, m.egui_font(font), colour);
    w
}

/// The control center's parts drawn where the skin's `ControlCenter.xml` docks them (GUI.dll FUN_1006f911 puts a
/// `CollapsingBitmapView_c` wing in `LeftWingDock` and `RightWingDock`, `LeftBarView_c` and `RightBarView_c` strips in the bar
/// docks, a `CCTargetControl_c` in each target dock and the rollup's `DockArea_c` in `RollupControllerDock`): the wings with
/// their bars, the strips, the rollup button and the target buttons, each only where its dock is shown.
pub fn show_docked(ctx: &egui::Context, yaw: f32, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, crate::derived::Breakdown>, palette: &Palette, d: &Docks) -> DockClicks {
    let mut out = DockClicks::default();
    let Some(skin) = crate::skin::skin(ctx) else { return out };
    let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
    let get = |s: u32| stats.get(&s).copied();
    if d.compass {
        compass(ctx, &skin, yaw);
    }
    // the strips (under the wings: drawn first)
    let measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
    let white = egui::Color32::WHITE;
    let label = {
        let c = crate::chrome::code1(palette);
        egui::Color32::from_rgb(c[0], c[1], c[2])
    };
    // `LeftBarView_c` (FUN_10070040): within borders 6, 2, 23, 2, "NCU" and the NCU in use of the most (stats 180 and 181,
    // `%u/%u`), then at the right end "CRED" and the credits (stat 61, `FormatNumeric`), 5 pixels after each label
    if let Some(rect) = d.left_bar {
        egui::Area::new(egui::Id::unique("cc-left-bar")).fixed_pos(rect.min).interactable(false).show(ctx, |ui| {
            ui.allocate_exact_size(rect.size(), egui::Sense::hover());
            let painter = ui.painter();
            skin.draw(painter, "GFX_GUI_CONTROLCENTER_BOTTOM_LEFT", rect.min, 1.0, white);
            let row = egui::Rect::from_min_max(rect.min + egui::vec2(6.0, 2.0), rect.max - egui::vec2(23.0, 2.0));
            let w = strip_text(painter, &measure, row.left(), row, "NCU", label);
            strip_text(painter, &measure, row.left() + w + 5.0, row, &format!("{}/{}", get(180).unwrap_or(0).max(0), get(181).unwrap_or(0).max(0)), white);
            let cash = grouped(get(61).unwrap_or(0));
            let font = crate::gui::font::FontId::NORMAL;
            let x = row.right() - measure.width(font, &cash) - 5.0 - measure.width(font, "CRED");
            let w = strip_text(painter, &measure, x, row, "CRED", label);
            strip_text(painter, &measure, x + w + 5.0, row, &cash, white);
        });
    }
    // `RightBarView_c` (FUN_1007077c): within borders 23, 2, 6, 2, "DEF", the slider of the aggression setting (stat 51,
    // -100 defensive to 100 aggressive; `N3Msg_SetAggDef` when let go) and "AGG", 5 pixels between them
    if let Some(rect) = d.right_bar {
        egui::Area::new(egui::Id::unique("cc-right-bar")).fixed_pos(rect.min).show(ctx, |ui| {
            ui.allocate_exact_size(rect.size(), egui::Sense::hover());
            let painter = ui.painter().clone();
            skin.draw(&painter, "GFX_GUI_CONTROLCENTER_BOTTOM_RIGHT", rect.min, 1.0, white);
            let row = egui::Rect::from_min_max(rect.min + egui::vec2(23.0, 2.0), rect.max - egui::vec2(6.0, 2.0));
            let font = crate::gui::font::FontId::NORMAL;
            let def = strip_text(&painter, &measure, row.left(), row, "DEF", label);
            let agg_w = measure.width(font, "AGG");
            strip_text(&painter, &measure, row.right() - agg_w, row, "AGG", label);
            let track = egui::Rect::from_min_max(egui::pos2(row.left() + def + 5.0, row.top()), egui::pos2(row.right() - agg_w - 5.0, row.bottom()));
            let knob = skin.get("GFX_GUI_CONTROLCENTER_AGGDEF_SLIDER").map_or(egui::vec2(8.0, 12.0), |s| s.size);
            let bg = skin.get("GFX_GUI_CONTROLCENTER_AGGDEF_SLIDER_BACKGROUND").map_or(egui::vec2(track.width(), 4.0), |s| s.size);
            let bg_rect = egui::Rect::from_center_size(track.center(), egui::vec2(track.width(), bg.y.min(track.height())));
            if !skin.draw_in(&painter, "GFX_GUI_CONTROLCENTER_AGGDEF_SLIDER_BACKGROUND", bg_rect, white) {
                painter.rect_filled(bg_rect, 1.0, egui::Color32::from_black_alpha(160));
            }
            let response = ui.interact(track, ui.scope_id().with("aggdef"), egui::Sense::click_and_drag());
            let drag_id = egui::Id::unique("cc-aggdef-drag");
            let span = (track.width() - knob.x).max(1.0);
            let value_at = |x: f32| (((x - track.left() - knob.x * 0.5) / span).clamp(0.0, 1.0) * 200.0 - 100.0).floor();
            let mut value = ctx.data(|m| m.get_temp::<f32>(drag_id)).unwrap_or(get(51).unwrap_or(0).clamp(-100, 100) as f32);
            if let Some(p) = response.interact_pointer_pos().filter(|_| response.dragged() || response.is_pointer_button_down_on()) {
                value = value_at(p.x);
                ctx.data_mut(|m| m.insert_temp(drag_id, value));
            }
            if response.drag_stopped() || response.clicked() {
                ctx.data_mut(|m| m.remove::<f32>(drag_id));
                out.aggdef = Some(value as i32);
            }
            let x = track.left() + (value + 100.0) / 200.0 * span;
            let at = egui::pos2(x, track.center().y - knob.y * 0.5);
            if skin.draw(&painter, "GFX_GUI_CONTROLCENTER_AGGDEF_SLIDER", at, 1.0, white).is_none() {
                painter.rect_filled(egui::Rect::from_min_size(at, knob), 1.0, label);
            }
            response.on_hover_text(format!("Aggression {value:+.0} (DEF -100 to AGG +100)"));
        });
    }
    // the wings with their bars (standing in the wing's tall side, on its floor)
    for (rect, left) in [(d.left_wing, true), (d.right_wing, false)] {
        let Some(rect) = rect.filter(|_| !d.rolled) else { continue };
        // (under the strips, which ControlCenter.xml lists after the wings: a stacked view draws its later children on top)
        egui::Area::new(egui::Id::unique(if left { "cc-left" } else { "cc-right" })).fixed_pos(rect.min).interactable(false).order(egui::Order::Background).show(ctx, |ui| {
            ui.allocate_exact_size(rect.size(), egui::Sense::hover());
            skin.draw(ui.painter(), if left { "GFX_GUI_CONTROLCENTER_WING_LEFT" } else { "GFX_GUI_CONTROLCENTER_WING_RIGHT" }, rect.min, 1.0, egui::Color32::WHITE);
        });
    }
    // the bars: windows of their own
    let (xp, xp_span) = progress(get(stat::XP), get(stat::LAST_XP).unwrap_or(0), get(stat::NEXT_XP));
    let (alien, alien_span) = progress(get(stat::ALIEN_XP), 0, get(stat::ALIEN_NEXT_XP));
    // (where each stands until the player moves it: in the wings' tall sides, on their floors, health and nano in the left
    // wing and the experience bars in the right, as the original's own layout has them; without the wings, in a row
    // under the top of the screen)
    let bars = [
        (d.health, "CCHealthBarConfig", "HEALTH", get(stat::HEALTH), get(stat::MAX_HEALTH), "Health", Some((stat::MAX_HEALTH, "Max health")), (true, 8.0)),
        (d.nano, "CCNanoBarConfig", "NANO", get(stat::CURRENT_NANO), get(stat::MAX_NANO), "Nano", Some((stat::MAX_NANO, "Max nano energy")), (true, 24.0)),
        (d.xp, "CCXPBarConfig", "XP", xp, xp_span, "Experience", None, (false, 20.0)),
        (d.alien_xp && get(stat::ALIEN_LEVEL).is_some_and(|l| l > 0), "CCAlienXPBarConfig", "ALIENXP", alien, alien_span, "Alien experience", None, (false, 36.0)),
    ];
    for (i, (shown, name, art, value, max, label, breakdown, (left, inset))) in bars.into_iter().enumerate() {
        if shown {
            let wing = if left { d.left_wing } else { d.right_wing };
            let middle = ctx.content_rect().center().x;
            // (on the wing's floor: where the strip lies over the wing's lower end, the floor is the strip's top; a skin
            // that docks the strip elsewhere, the AOU mod's CYT at the top of the screen, leaves the wing its own floor:
            // the bars stood at the top of the screen there)
            let floor = (if left { d.left_bar } else { d.right_bar }).filter(|b| wing.is_some_and(|w| b.intersects(w) && b.top() > w.top())).map(|b| b.top());
            let at = move |size: egui::Vec2| match wing {
                Some(w) if left => egui::pos2(w.left() + inset, floor.unwrap_or(w.bottom()) - size.y - 2.0),
                Some(w) => egui::pos2(w.right() - (inset - 12.0) - size.x, floor.unwrap_or(w.bottom()) - size.y - 2.0),
                None => egui::pos2(middle - 40.0 + i as f32 * 20.0, 80.0),
            };
            let response = char_bar(ctx, &skin, at, name, art, value, max, label);
            if let (Some(r), Some((stat, title))) = (response, breakdown) {
                crate::stats::hover(r, title, parts.get(&stat), accent);
            }
        }
    }
    // the rollup button: folds the wings away
    if let Some(dock) = d.rollup {
        // (at the bottom of its dock, whose area above is where windows dock)
        let size = skin.get("GFX_GUI_ROLLUP_COLLAPSE_STATE1").map_or(egui::vec2(15.0, 15.0), |s| s.size);
        let at = egui::pos2(dock.right() - size.x, dock.bottom() - size.y);
        egui::Area::new(egui::Id::unique("cc-rollup")).fixed_pos(at).order(egui::Order::Foreground).show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            let state = if response.is_pointer_button_down_on() { 3 } else if response.hovered() { 2 } else { 1 };
            skin.draw(ui.painter(), &format!("GFX_GUI_ROLLUP_{}_STATE{state}", if d.rolled { "EXPAND" } else { "COLLAPSE" }), rect.min, 1.0, accent);
            if response.on_hover_text(if d.rolled { "Show the control center" } else { "Fold the control center away" }).clicked() {
                out.rollup = true;
            }
        });
    }
    // the target buttons: the left one targets the character itself (F1), the right one the next hostile (Tab)
    for (rect, window, icon, tip) in [
        (d.left_target, "target_self", "GFX_GUI_CONTROLCENTER_TARGET_ICON_SELF", "Target yourself"),
        (d.right_target, "target_next", "GFX_GUI_CONTROLCENTER_TARGET_ICON_OTHER", "Target the next enemy"),
    ] {
        let Some(rect) = rect else { continue };
        egui::Area::new(egui::Id::unique(window)).fixed_pos(rect.min).order(egui::Order::Foreground).show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(rect.size(), egui::Sense::click());
            let state = if response.is_pointer_button_down_on() { "_STATE3" } else if response.hovered() { "_STATE2" } else { "" };
            skin.draw(ui.painter(), &format!("GFX_GUI_CONTROLCENTER_TARGET_BUTTON{state}"), rect.min, 1.0, egui::Color32::WHITE);
            if let Some(icon_size) = skin.get(icon).map(|s| s.size) {
                skin.draw(ui.painter(), icon, rect.center() - icon_size * 0.5, 1.0, egui::Color32::WHITE);
            }
            if response.on_hover_text(tip).clicked() {
                out.target = Some(window);
            }
        });
    }
    out
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
