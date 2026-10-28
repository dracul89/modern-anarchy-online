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

/// Where `TopBorderView_c` (GUI.dll, laid out by FUN_10147438) puts tabs `widths` wide in a strip `room` wide: the first 2 points
/// in, each overlapping the one before by 6, every tab its own width (a `Tab`'s text width plus a point plus its 5 and 16 point
/// margins, FUN_101463a4) when they fit, else all narrowed in proportion (`View::SpaceOut`'s share; not followed exactly). The
/// tabs' left edges and widths.
pub fn tab_places(widths: &[f32], room: f32) -> Vec<(f32, f32)> {
    let total: f32 = widths.iter().sum::<f32>() - 6.0 * widths.len().saturating_sub(1) as f32;
    let space = room - 2.0;
    let scale = if total > space && total > 0.0 { (space / total).max(0.2) } else { 1.0 };
    let mut x = 2.0;
    widths
        .iter()
        .map(|w| {
            let w = (w * scale).floor();
            let at = (x, w);
            x += w - 6.0;
            at
        })
        .collect()
}

/// A `TabView` (GUI.dll 0x10146767) in the skin's art with `names` for tabs, `selected` the open one, `add` filling it: its strip
/// (`TopBorderView_c`, FUN_10147b0f / FUN_1014706e: `GFX_GUI_TAB_BORDER_TL`, `_TOP`, `_TR` along its foot) with the tabs (`Tab`,
/// FUN_1014694d: `BorderView_c`s of `GFX_GUI_TAB_INACTIVE_LEFT/MIDDLE/RIGHT` at 0.6 of the alpha with black text and the open
/// one `GFX_GUI_TAB_ACTIVE_*` in front with white text, FUN_101462d8; the text in font 5, Verdana 13, from 5 points in), then
/// the body (`TabView::SetRenderFlags` 0x101464c4 and `LayoutBorders` 0x10145dad: `GFX_GUI_TAB_BORDER_LEFT`, `_RIGHT`, `_BL`,
/// `_BOTTOM`, `_BR` round it and `GFX_GUI_TAB_BACKGROUND` inside, the content 2 points inside the borders), all tinted with the
/// theme's border colour. A tab opens when it is pressed; `add` is told which is open. Without the pictures it is a row of egui's
/// selectable labels.
pub fn tab_view<R>(ui: &mut egui::Ui, palette: &Palette, names: &[&str], selected: &mut usize, add: impl FnOnce(&mut egui::Ui, usize) -> R) -> R {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_TAB_ACTIVE_MIDDLE") && s.has("GFX_GUI_TAB_BORDER_LEFT"));
    let Some(skin) = skin else {
        ui.horizontal(|ui| {
            for (i, name) in names.iter().enumerate() {
                if ui.selectable_label(*selected == i, *name).clicked() {
                    *selected = i;
                }
            }
        });
        return add(ui, *selected);
    };
    let tint = rgb(palette.accent);
    let size = |name: &str| skin.get(name).map(|s| s.size).unwrap_or(egui::vec2(1.0, 1.0));
    let height = size("GFX_GUI_TAB_ACTIVE_MIDDLE").y;
    let font = crate::font::verdana(ui.ctx(), 13.0);
    let widths: Vec<f32> = names.iter().map(|n| ui.painter().layout_no_wrap(n.to_string(), font.clone(), egui::Color32::WHITE).size().x.ceil() + 1.0 + 5.0 + 16.0).collect();
    let room = ui.available_width();
    let places = tab_places(&widths, room.max(40.0));
    let need = places.last().map(|(x, w)| x + w + 2.0).unwrap_or(40.0);
    let (strip, _) = ui.allocate_exact_size(egui::vec2(need.min(room), height + 1.0), egui::Sense::hover());
    let painter = ui.painter().clone();
    let under = painter.add(egui::Shape::Noop);
    let mut order: Vec<usize> = (0..names.len()).filter(|&i| i != *selected).collect();
    order.extend((*selected < names.len()).then_some(*selected));
    for i in order {
        let (x, w) = places[i];
        let rect = egui::Rect::from_min_size(egui::pos2(strip.left() + x, strip.top()), egui::vec2(w, height));
        let response = ui.interact(rect, ui.id().with(("tab", i)), egui::Sense::click());
        if response.hovered() && ui.input(|inp| inp.pointer.primary_pressed()) {
            *selected = i;
        }
        let active = i == *selected;
        skin.three_slice(&painter, if active { "GFX_GUI_TAB_ACTIVE" } else { "GFX_GUI_TAB_INACTIVE" }, rect, if active { tint } else { tint.gamma_multiply(0.6) });
        let text = if active { egui::Color32::WHITE } else { egui::Color32::BLACK };
        // (the text is the tab's child view: what does not fit inside its right margin is cut off)
        painter.with_clip_rect(rect.intersect(egui::Rect::from_min_max(rect.min, egui::pos2(rect.right() - 12.0, rect.bottom())))).text(egui::pos2(rect.left() + 5.0, rect.center().y), egui::Align2::LEFT_CENTER, names[i], font.clone(), text);
    }
    let (left, right, bottom) = (size("GFX_GUI_TAB_BORDER_LEFT").x, size("GFX_GUI_TAB_BORDER_RIGHT").x, size("GFX_GUI_TAB_BORDER_BOTTOM").y);
    let body = ui.painter().add(egui::Shape::Noop);
    let open = *selected;
    let inner = ui.scope(|ui| egui::Frame::NONE.inner_margin(egui::Margin { left: (left + 2.0) as i8, right: (right + 2.0) as i8, top: 2, bottom: (bottom + 2.0) as i8 }).show(ui, |ui| add(ui, open)).inner);
    let content = inner.response.rect;
    let (l, r) = (strip.left(), content.right().max(strip.right()));
    let piece = |shapes: &mut Vec<egui::Shape>, name: &str, rect: egui::Rect| {
        if let Some(s) = skin.get(name) {
            shapes.push(egui::Shape::image(s.texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), tint));
        }
    };
    // (the strip's foot: its corners at the bottom left and right, the top line between)
    let (tl, tr, top) = (size("GFX_GUI_TAB_BORDER_TL"), size("GFX_GUI_TAB_BORDER_TR"), size("GFX_GUI_TAB_BORDER_TOP").y);
    let foot = strip.bottom();
    let mut line = Vec::new();
    piece(&mut line, "GFX_GUI_TAB_BORDER_TL", egui::Rect::from_min_size(egui::pos2(l, foot - tl.y), tl));
    piece(&mut line, "GFX_GUI_TAB_BORDER_TOP", egui::Rect::from_min_max(egui::pos2(l + tl.x, foot - top), egui::pos2(r - tr.x, foot)));
    piece(&mut line, "GFX_GUI_TAB_BORDER_TR", egui::Rect::from_min_size(egui::pos2(r - tr.x, foot - tr.y), tr));
    painter.set(under, egui::Shape::Vec(line));
    // (the body round the content: the sides, the bottom corners and the bottom, the background inside)
    let (t, b) = (foot, content.bottom());
    let (bl, br) = (size("GFX_GUI_TAB_BORDER_BL"), size("GFX_GUI_TAB_BORDER_BR"));
    let mut frame = Vec::new();
    piece(&mut frame, "GFX_GUI_TAB_BACKGROUND", egui::Rect::from_min_max(egui::pos2(l + left + 1.0, t + 1.0), egui::pos2(r - right - 1.0, b - bottom - 1.0)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_LEFT", egui::Rect::from_min_max(egui::pos2(l, t), egui::pos2(l + left, b - bl.y)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_RIGHT", egui::Rect::from_min_max(egui::pos2(r - right, t), egui::pos2(r, b - br.y)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_BL", egui::Rect::from_min_size(egui::pos2(l, b - bl.y), bl));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_BOTTOM", egui::Rect::from_min_max(egui::pos2(l + bl.x, b - bottom), egui::pos2(r - br.x, b)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_BR", egui::Rect::from_min_size(egui::pos2(r - br.x, b - br.y), br));
    ui.painter().set(body, egui::Shape::Vec(frame));
    inner.inner
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
    fn tabs_overlap_by_six_from_two_in() {
        assert_eq!(tab_places(&[40.0, 30.0, 50.0], 500.0), vec![(2.0, 40.0), (36.0, 30.0), (60.0, 50.0)]);
        // (too wide: narrowed in proportion)
        let p = tab_places(&[100.0, 100.0], 98.0);
        assert!(p[1].0 + p[1].1 <= 98.0 + 1e-3);
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
