//! Tool tips: what `WindowController_c` shows when the pointer rests on something that has one.
//!
//! `UpdateToolTip` (0x10157adc) takes the tip of what is under the pointer (a title and a body) and starts a timer of
//! 500 ms when it changes; `Render` (0x10157580) opens a `ToolTip_c` (0x10149c20) when it runs out: a window at the
//! pointer (`MoveToMouse`) holding an `InfoContainer_c` (FUN_101494d9): the title (font 5; in colour code 0x1000000,
//! `Default`, when there is no body) and under it the body (font 5, word-wrapped, `TVF_MULTILINE | TVF_WORD_WRAP`, two
//! wide to one high: `SetAspectRatio(2)`), framed in colour code 0x3000000 (`Hover`), its margin 2 with a body and 1
//! without. The frame's pieces here are lines of that width round a dark ground with the title ruled off from the body;
//! the original's exact surface sizes are not followed.

use super::controls::{Colour, TextView};
use super::font::FontId;
use super::geom::Point;
use super::host::{Env, Paint};
use super::view::Control;

/// How long the pointer rests before the tip shows.
pub const DELAY: f64 = 0.5;

/// Draws a tip at `at` (the pointer).
pub fn draw(ctx: &egui::Context, env: &Env, at: egui::Pos2, title: &str, body: &str) {
    let m = env.measure;
    let margin = if body.is_empty() { 1.0 } else { 2.0 };
    let pad = margin + 3.0;
    let mut title_view = TextView::new(title);
    title_view.font = FontId::NORMAL;
    title_view.colour = if body.is_empty() { Colour::DEFAULT } else { Colour::WHITE };
    let title_size = title_view.preferred(m, false).unwrap_or_default();
    let body_view = (!body.is_empty()).then(|| {
        let mut v = TextView::new(body);
        v.font = FontId::NORMAL;
        v.feature = super::text::flags::MULTILINE | super::text::flags::WORD_WRAP;
        // (two wide to one high: the line's width from the text's area)
        let total = m.width(FontId::NORMAL, &body.replace('\n', " "));
        let width = ((total * m.height(FontId::NORMAL) * 2.0).sqrt()).ceil().max(title_size.x + 1.0).min(400.0);
        v.min = Point::new(width - 1.0, -1.0);
        v
    });
    let body_size = body_view.as_ref().map(|v| {
        let w = v.min.x + 1.0;
        Point::new(w, v.height_at(m, w))
    });
    let inner_w = (title_size.x + 1.0).max(body_size.map_or(0.0, |b| b.x));
    let rule = if body_size.is_some() { margin + 6.0 } else { 0.0 };
    let inner_h = title_size.y + 1.0 + rule + body_size.map_or(0.0, |b| b.y);
    let size = egui::vec2(inner_w + 2.0 * pad, inner_h + 2.0 * pad);
    let screen = ctx.content_rect();
    let at = egui::pos2((at.x + 12.0).min(screen.right() - size.x).max(screen.left()), (at.y + 16.0).min(screen.bottom() - size.y).max(screen.top()));
    let rect = egui::Rect::from_min_size(at, size);
    let hover = {
        let c = env.palette.hover;
        egui::Color32::from_rgb(c[0], c[1], c[2])
    };
    egui::Area::new(egui::Id::unique("mao-gui-tooltip")).order(egui::Order::Tooltip).fixed_pos(at).interactable(false).show(ctx, |ui| {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0xe0));
        painter.rect_stroke(rect.shrink(margin * 0.5), 0.0, egui::Stroke::new(margin, hover), egui::StrokeKind::Middle);
        let title_rect = egui::Rect::from_min_size(rect.min + egui::vec2(pad, pad), egui::vec2(title_size.x + 1.0, title_size.y + 1.0));
        title_view.paint(&Paint { painter, env, rect: title_rect, hovered: false, pressed: false, enabled: true });
        if let (Some(v), Some(b)) = (&body_view, body_size) {
            let y = title_rect.bottom() + 3.0;
            painter.rect_filled(egui::Rect::from_min_max(egui::pos2(rect.left() + margin, y), egui::pos2(rect.right() - margin, y + margin)), 0.0, hover);
            let body_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + pad, y + margin + 3.0), egui::vec2(b.x, b.y));
            v.paint(&Paint { painter, env, rect: body_rect, hovered: false, pressed: false, enabled: true });
        }
    });
}

/// A source of tips (a window's views, the control center): what its pointer rests on this frame (`target`, with that
/// thing's title and body), shown once the pointer has rested on the same thing for [`DELAY`].
pub fn hover(ctx: &egui::Context, env: &Env, source: egui::Id, target: Option<egui::Id>, title: &str, body: &str) {
    let slot = source.with("mao-gui-tooltip-since");
    let Some(key) = target.filter(|_| !(title.is_empty() && body.is_empty())) else {
        ctx.data_mut(|d| d.remove::<(egui::Id, f64)>(slot));
        return;
    };
    let now = ctx.input(|i| i.time);
    let since = match ctx.data(|d| d.get_temp::<(egui::Id, f64)>(slot)) {
        Some((k, t)) if k == key => t,
        _ => {
            ctx.data_mut(|d| d.insert_temp(slot, (key, now)));
            now
        }
    };
    if now - since >= DELAY {
        if let Some(at) = ctx.pointer_hover_pos() {
            draw(ctx, env, at, title, body);
        }
    } else {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(DELAY - (now - since) + 0.01));
    }
}
