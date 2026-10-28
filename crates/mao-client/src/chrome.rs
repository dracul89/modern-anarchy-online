//! Windows in the original's chrome. [`window`] is used in place of `egui::Window` by every window of the interface: when the
//! skin's pictures are there (`skin`), the window has no egui title bar and is drawn with the original's frame pieces
//! (`GFX_GUI_WINDOW3_BORDER_*`, greyscale masks tinted with the theme's border colour, as the original tints them with its
//! `GUIColors.xml`), the title inside the frame at the top left and the original's pin and close buttons at the top right;
//! dragging anywhere on the window moves it (egui's own fallback without a title bar). Without the pictures it is the plain
//! egui window it was.
//!
//! A pinned window (the pin button, as in the original) is left open by Escape; `pinned` tells `Ui::close_with_escape`.

use std::sync::Mutex;

use crate::theme::Palette;

/// The windows whose pin is down, by their name.
static PINNED: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn pinned(name: &str) -> bool {
    PINNED.lock().unwrap().iter().any(|n| n == name)
}

fn set_pinned(name: &str, on: bool) {
    let mut list = PINNED.lock().unwrap();
    list.retain(|n| n != name);
    if on {
        list.push(name.to_string());
    }
}

fn rgb(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

/// The frame's margins: the title row at the top, the border pieces around.
const LEFT: i8 = 7;
const RIGHT: i8 = 7;
const TOP: i8 = 24;
const BOTTOM: i8 = 10;

/// A window named `name` (its egui id and its `layout.ini` entry) titled `title`, shown while `*open`; `build` sets the egui window's
/// size and place (`|w| w.resizable(false).default_pos(..)`), `add` fills it. Returns what `egui::Window::show` returns.
pub fn window<R>(ctx: &egui::Context, palette: &Palette, name: &str, title: &str, open: &mut bool, build: impl FnOnce(egui::Window<'static>) -> egui::Window<'static>, add: impl FnOnce(&mut egui::Ui) -> R) -> Option<egui::InnerResponse<Option<R>>> {
    let accent = rgb(palette.accent);
    let id = egui::Id::new(name);
    let skin = crate::skin::skin(ctx).filter(|s| s.has("GFX_GUI_WINDOW3_BORDER_TL"));
    let Some(skin) = skin else {
        let mut is_open = *open;
        let shown = build(egui::Window::new(egui::RichText::new(title).color(accent)).collapsible(false)).id(id).open(&mut is_open).show(ctx, add);
        *open = is_open;
        return shown;
    };
    if !*open {
        return None;
    }
    let panel = egui::Color32::from_rgba_unmultiplied(palette.panel[0], palette.panel[1], palette.panel[2], 225);
    let hover = rgb(palette.hover);
    let frame = egui::Frame::NONE.inner_margin(egui::Margin { left: LEFT, right: RIGHT, top: TOP, bottom: BOTTOM });
    let mut close = false;
    let title = title.to_string();
    let name = name.to_string();
    let shown = build(egui::Window::new(title.as_str()).title_bar(false).collapsible(false).frame(frame)).id(id).show(ctx, |ui| {
        let painter = ui.painter().clone();
        // the frame goes under the content: its place in the paint list is kept, it is drawn once the size is known
        let under = painter.add(egui::Shape::Noop);
        let result = add(ui);
        // The whole window as laid out: egui's record of the area (last frame's for a window that just appeared, hence the fallback
        // from the content).
        let content = ui.min_rect();
        let outer = ctx.memory(|m| m.area_rect(id)).filter(|r| r.is_finite() && r.width() > 20.0).unwrap_or_else(|| egui::Rect::from_min_max(content.min - egui::vec2(LEFT as f32, TOP as f32), content.max + egui::vec2(RIGHT as f32, BOTTOM as f32)));
        painter.set(under, frame_shape(&skin, outer, panel, accent));
        // the title row
        crate::font::text(ctx, &painter, egui::pos2(outer.left() + 9.0, outer.top() + 5.0), &title, 15.0, accent);
        let mut x = outer.right() - RIGHT as f32 - 15.0;
        for (button, what) in [("CLOSE_X", "close"), ("PIN", "pin")] {
            let rect = egui::Rect::from_min_size(egui::pos2(x, outer.top() + 5.0), egui::vec2(15.0, 15.0));
            let response = ui.interact(rect, id.with(what), egui::Sense::click());
            let down = what == "pin" && pinned(&name);
            let state = if response.is_pointer_button_down_on() || down { "_STATE3" } else if response.hovered() { "_STATE2" } else { "" };
            let tint = if response.hovered() { hover } else if down { rgb(palette.text_selected) } else { accent };
            if !skin.draw_in(&painter, &format!("GFX_GUI_WINDOW_{button}{state}"), rect, tint) {
                skin.draw_in(&painter, &format!("GFX_GUI_WINDOW_{button}"), rect, tint);
            }
            if response.clicked() {
                match what {
                    "pin" => set_pinned(&name, !down),
                    _ => close = true,
                }
            }
            response.on_hover_text(if what == "pin" { "Pinned: stays open when Escape closes the windows" } else { "Close" });
            x -= 17.0;
        }
        result
    });
    if close {
        *open = false;
    }
    shown
}

/// The window's fill and the original's border pieces around `outer`: thin top and sides, rounded bottom corners.
fn frame_shape(skin: &crate::skin::Skin, outer: egui::Rect, panel: egui::Color32, tint: egui::Color32) -> egui::Shape {
    let mut shapes = vec![egui::Shape::rect_filled(outer.shrink(2.0), 2.0, panel)];
    let size = |name: &str| skin.get(&format!("GFX_GUI_WINDOW3_BORDER_{name}")).map(|s| s.size);
    let (Some(tl), Some(tr), Some(bl), Some(br), Some(top), Some(bottom), Some(left), Some(right)) = (size("TL"), size("TR"), size("BL"), size("BR"), size("TOP"), size("BOTTOM"), size("LEFT"), size("RIGHT")) else {
        shapes.push(egui::Shape::rect_stroke(outer, 2.0, egui::Stroke::new(1.0, tint), egui::StrokeKind::Inside));
        return egui::Shape::Vec(shapes);
    };
    let mut piece = |name: &str, rect: egui::Rect| {
        if let Some(sprite) = skin.get(&format!("GFX_GUI_WINDOW3_BORDER_{name}")) {
            shapes.push(egui::Shape::image(sprite.texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), tint));
        }
    };
    let (l, t, r, b) = (outer.left(), outer.top(), outer.right(), outer.bottom());
    piece("TL", egui::Rect::from_min_size(egui::pos2(l, t), tl));
    piece("TOP", egui::Rect::from_min_max(egui::pos2(l + tl.x, t), egui::pos2(r - tr.x, t + top.y)));
    piece("TR", egui::Rect::from_min_size(egui::pos2(r - tr.x, t), tr));
    piece("LEFT", egui::Rect::from_min_max(egui::pos2(l, t + tl.y), egui::pos2(l + left.x, b - bl.y)));
    piece("RIGHT", egui::Rect::from_min_max(egui::pos2(r - right.x, t + tr.y), egui::pos2(r, b - br.y)));
    piece("BL", egui::Rect::from_min_size(egui::pos2(l, b - bl.y), bl));
    piece("BOTTOM", egui::Rect::from_min_max(egui::pos2(l + bl.x, b - bottom.y), egui::pos2(r - br.x, b)));
    piece("BR", egui::Rect::from_min_size(egui::pos2(r - br.x, b - br.y), br));
    egui::Shape::Vec(shapes)
}

/// What `ScrollBar_c` (GUI.dll) is doing with the pointer: pressing an arrow (`HitItem_e` 1 the up, 2 the down; held, it repeats
/// after half a second every 0.08 s, `_SetButtonState` 0x10141175 / `SlotRepeatTimer` 0x10141d21) or dragging the knob (3, the
/// point grabbed below its top, `MouseDown` 0x101411fe).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ScrollPress {
    item: u8,
    grab: f32,
    repeat_at: f64,
}

/// The knob of a vertical `ScrollBar_c` in a bar `top`..`bottom` (screen points, the bottom exclusive) whose arrows are `up` and
/// `down` high, showing `proportion` of the content at `fraction` of the way down (`_CalcKnobSizeAndPos` 0x10141ab9): between
/// the arrows less a point each side, `floor(proportion × length)` long but at least 8 (less than the length when that is
/// short), placed `floor((length − knob) × fraction)` down; its top and its exclusive bottom (the original's frames are
/// inclusive, so its knob is a point longer than the number).
pub fn knob(top: f32, bottom: f32, up: f32, down: f32, proportion: f32, fraction: f32) -> (f32, f32) {
    let start = top + up;
    let length = (bottom - down - 1.0) - start;
    let mut size = (proportion * length).floor();
    if size < 8.0 {
        size = if length - 1.0 < 8.0 { length - 1.0 } else { 8.0 };
    }
    let at = start + ((length - size) * fraction.clamp(0.0, 1.0)).floor();
    (at, at + size + 1.0)
}

/// A vertical scroll area with the scroll bar of the original's scroll views (`ScrollView_c::_UpdateScrollBars` 0x10142b68 makes
/// a `ScrollBar_c` with `GFX_GUI_SIMPLE_SB_UP`, `_DOWN` and `_SLIDER_VERT` and no track, a point right of the content, as wide
/// as its arrows), tinted with the theme's border colour as the frame is: an arrow scrolls 5 points (`ScrollBar_c`'s small step,
/// 0.05 of the 0..100 its scroll view makes it with), a click above or below the knob 20 (its large step, 0.2 of it), the knob
/// drags. egui's own bar is hidden; without the skin's pictures it is egui's scroll area.
pub fn scroll<R>(ui: &mut egui::Ui, palette: &Palette, area: egui::ScrollArea, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_SIMPLE_SB_UP"));
    let Some(skin) = skin else { return area.show(ui, add).inner };
    let (Some(up), Some(down)) = (skin.get("GFX_GUI_SIMPLE_SB_UP"), skin.get("GFX_GUI_SIMPLE_SB_DOWN")) else { return area.show(ui, add).inner };
    let width = up.size.x;
    let key = ui.make_persistent_id(("mao-scroll", ui.next_auto_id()));
    let ctx = ui.ctx().clone();
    let pending = ctx.data_mut(|d| d.remove_temp::<f32>(key.with("to")));
    let mut area = area.scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).content_margin(egui::Margin { left: 0, right: (width + 1.0) as i8, top: 0, bottom: 0 });
    if let Some(to) = pending {
        area = area.vertical_scroll_offset(to);
    }
    let out = area.show(ui, add);
    let inner = out.inner_rect;
    let overflow = out.content_size.y - inner.height();
    if overflow <= 0.5 {
        return out.inner;
    }
    let bar = egui::Rect::from_min_max(egui::pos2(inner.right() - width, inner.top()), egui::pos2(inner.right(), inner.bottom()));
    let offset = out.state.clamped_offset().y.clamp(0.0, overflow);
    let (k0, k1) = knob(bar.top(), bar.bottom(), up.size.y, down.size.y, inner.height() / out.content_size.y, offset / overflow);
    let up_rect = egui::Rect::from_min_size(bar.min, up.size);
    let down_rect = egui::Rect::from_min_size(egui::pos2(bar.left(), bar.bottom() - down.size.y), down.size);
    let knob_rect = egui::Rect::from_min_max(egui::pos2(bar.left(), k0), egui::pos2(bar.right(), k1));
    let response = ui.interact(bar, key.with("bar"), egui::Sense::click_and_drag());
    let now = ctx.input(|i| i.time);
    let mut press: ScrollPress = ctx.data(|d| d.get_temp(key.with("press"))).unwrap_or_default();
    let mut to = None;
    if response.is_pointer_button_down_on() {
        let at = response.interact_pointer_pos().unwrap_or(bar.center());
        if response.drag_started() || (press.item == 0 && ctx.input(|i| i.pointer.primary_pressed())) {
            if knob_rect.contains(at) {
                press = ScrollPress { item: 3, grab: at.y - k0, repeat_at: 0.0 };
            } else if up_rect.contains(at) {
                press = ScrollPress { item: 1, grab: 0.0, repeat_at: now + 0.5 };
                to = Some(offset - 5.0);
            } else if down_rect.contains(at) {
                press = ScrollPress { item: 2, grab: 0.0, repeat_at: now + 0.5 };
                to = Some(offset + 5.0);
            } else {
                press = ScrollPress { item: 4, grab: 0.0, repeat_at: 0.0 };
                to = Some(if at.y < k0 { offset - 20.0 } else { offset + 20.0 });
            }
        } else if press.item == 3 {
            // (`MouseMove` 0x101416a0: the knob's top over the room it moves in)
            let (start, end) = (bar.top() + up.size.y, bar.bottom() - down.size.y - 1.0 - (k1 - k0 - 1.0));
            if end > start {
                to = Some(overflow * (at.y - press.grab - start) / (end - start));
            }
        } else if (press.item == 1 || press.item == 2) && now >= press.repeat_at {
            let over = if press.item == 1 { up_rect.contains(at) } else { down_rect.contains(at) };
            if over {
                to = Some(offset + if press.item == 1 { -5.0 } else { 5.0 });
            }
            press.repeat_at = now + 0.08;
        }
        ctx.request_repaint();
    } else {
        press = ScrollPress::default();
    }
    ctx.data_mut(|d| d.insert_temp(key.with("press"), press));
    if let Some(to) = to {
        ctx.data_mut(|d| d.insert_temp(key.with("to"), to.clamp(0.0, overflow).floor()));
        ctx.request_repaint();
    }
    let tint = rgb(palette.accent);
    let painter = ui.painter();
    skin.draw_in(painter, "GFX_GUI_SIMPLE_SB_UP", up_rect, tint);
    skin.draw_in(painter, "GFX_GUI_SIMPLE_SB_DOWN", down_rect, tint);
    skin.draw_in(painter, "GFX_GUI_SIMPLE_SB_SLIDER_VERT", knob_rect, tint);
    out.inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_are_kept_by_name() {
        assert!(!pinned("Test window"));
        set_pinned("Test window", true);
        assert!(pinned("Test window"));
        set_pinned("Test window", false);
        assert!(!pinned("Test window"));
    }

    #[test]
    fn the_knob_fills_its_share_between_the_arrows() {
        // (a 100-point bar with 9-point arrows: 81 points from a point below the one to a point above the other)
        let (a, b) = knob(0.0, 100.0, 9.0, 9.0, 0.5, 0.0);
        assert_eq!((a, b), (9.0, 50.0));
        let (a, b) = knob(0.0, 100.0, 9.0, 9.0, 0.5, 1.0);
        assert_eq!((a, b), (50.0, 91.0));
        // (never under 8)
        let (a, b) = knob(0.0, 100.0, 9.0, 9.0, 0.01, 0.0);
        assert_eq!(b - a, 9.0);
    }
}
