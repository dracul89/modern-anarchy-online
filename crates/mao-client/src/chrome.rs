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
        painter.text(egui::pos2(outer.left() + 9.0, outer.top() + 5.0), egui::Align2::LEFT_TOP, &title, egui::FontId::proportional(15.0), accent);
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
}
