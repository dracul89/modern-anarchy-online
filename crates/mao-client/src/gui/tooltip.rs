//! Tool tips: what `WindowController_c` shows when the pointer rests on something that has one.
//!
//! `UpdateToolTip` (0x10157adc) takes the tip of what is under the pointer (a title and a body) and starts a timer of
//! 500 ms when it changes; `Render` (0x10157580) opens a `ToolTip_c` (0x10149c20) when it runs out: a window at the
//! pointer (`MoveToMouse`) holding an `InfoContainer_c` (FUN_101494d9): the title (font 5; in colour code 0x1000000,
//! `Default`, when there is no body) and under it the body (font 5, word-wrapped, `TVF_MULTILINE | TVF_WORD_WRAP`, two
//! wide to one high: `SetAspectRatio(2)`), framed in colour code 0x3000000 (`Hover`), its margin 2 with a body and 1
//! without. The frame's pieces here are lines of that width round a dark ground with the title ruled off from the body;
//! the original's exact surface sizes are not followed.
//!
//! Read again on 2026-11-25 (the click review): the title and the body are markup (`<font color=...>`, `<br>`); the
//! tip is placed once, where the pointer was when it came up (`Window::MoveToMouse` 0x101550de, [`place`]), and does
//! not follow; a button going down or coming up takes it away (`HandleMouseDown` 0x10157d8f and `HandleMouseUp`
//! 0x10157f13 call `UpdateToolTip(true)`), and it comes back only once the pointer has moved and rested again
//! (`HandleMouseMoved` arms it); a text that changes under a resting pointer (a running nano's time left) starts the
//! 500 ms again and the old tip stays up until the new one takes its place. The item lists' tips (FUN_10042193: items,
//! nano programs, shortcuts) and the NCU list's (FUN_10045b79) come through here too ([`show`]).

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
        // (two wide to one high: the line's width from the text's area, its tags not counted; no wider than the
        // screen leaves. Until 2026-11-25 no wider than 400, which the original does not have.)
        let total = m.width(FontId::NORMAL, &plain(body));
        let most = (ctx.content_rect().width() - 2.0 * pad - 8.0).max(120.0);
        let width = ((total * m.height(FontId::NORMAL) * 2.0).sqrt()).ceil().max(title_size.x + 1.0).min(most);
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
    let at = place(at, size, ctx.content_rect());
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

/// Where a tip of `size` goes for the pointer at `pointer` on `screen` (`Window::MoveToMouse`, 0x101550de: 16 right of
/// the pointer and 32 under it; in the screen's right half its right edge a point left of the pointer, in the lower
/// half its foot a point above it; then held inside the screen).
pub fn place(pointer: egui::Pos2, size: egui::Vec2, screen: egui::Rect) -> egui::Pos2 {
    let x = if screen.center().x <= pointer.x { pointer.x - (size.x + 1.0) } else { pointer.x + 16.0 };
    let y = if screen.center().y <= pointer.y { pointer.y - (size.y + 1.0) } else { pointer.y + 32.0 };
    egui::pos2(x.min(screen.right() - size.x).max(screen.left()), y.min(screen.bottom() - size.y).max(screen.top()))
}

/// Markup without its tags (a `<br>` a space), for measuring.
fn plain(markup: &str) -> String {
    let mut out = String::with_capacity(markup.len());
    let mut tag = false;
    for c in markup.chars() {
        match c {
            '<' => tag = true,
            '>' if tag => {
                tag = false;
                out.push(' ');
            }
            '\n' if !tag => out.push(' '),
            c if !tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
}

/// A tip being waited for or up: what the pointer rests on, its title and body, since when, and the tip that is up
/// (its title, body and the pointer's place when it came up).
type Resting = (egui::Id, String, String, f64, Option<(String, String, egui::Pos2)>);

/// What a frame makes of a resting pointer (the rule of `UpdateToolTip` and `Render`, apart from the clock's source):
/// `state` the tip waited for or up, `target` what the pointer rests on now with its texts, `button` whether a button
/// went down or came up this frame, `moved` whether the pointer moved, `waits` whether a button took the tip away and
/// the pointer has not moved since. Gives the new state and `waits`.
pub fn rest(state: Option<Resting>, target: Option<(egui::Id, &str, &str)>, button: bool, moved: bool, waits: bool, now: f64, pointer: Option<egui::Pos2>) -> (Option<Resting>, bool) {
    if button {
        return (None, true);
    }
    let Some((key, title, body)) = target.filter(|(_, t, b)| !(t.is_empty() && b.is_empty())) else { return (None, waits) };
    if waits && !moved {
        return (None, true);
    }
    let mut state = match state {
        // (the same thing: its texts changed starts the wait again, the tip that is up stays)
        Some((k, t, b, since, shown)) if k == key => {
            let since = if t == title && b == body { since } else { now };
            (key, title.to_string(), body.to_string(), since, shown)
        }
        _ => (key, title.to_string(), body.to_string(), now, None),
    };
    if now - state.3 >= DELAY
        && state.4.as_ref().is_none_or(|(t, b, _)| t != title || b != body)
        && let Some(at) = pointer
    {
        state.4 = Some((title.to_string(), body.to_string(), at));
    }
    (Some(state), false)
}

/// A source of tips (a window's views, the control center): what its pointer rests on this frame (`target`, with that
/// thing's title and body), shown once the pointer has rested on the same thing for [`DELAY`] ([`rest`]).
pub fn hover(ctx: &egui::Context, env: &Env, source: egui::Id, target: Option<egui::Id>, title: &str, body: &str) {
    let slot = source.with("mao-gui-tooltip-since");
    let waiting = source.with("mao-gui-tooltip-waits");
    let (button, moved, now) = ctx.input(|i| (i.pointer.any_pressed() || i.pointer.any_released(), i.pointer.delta() != egui::Vec2::ZERO, i.time));
    let (state, waits) = ctx.data(|d| (d.get_temp::<Resting>(slot), d.get_temp::<bool>(waiting).unwrap_or(false)));
    let (state, waits) = rest(state, target.map(|k| (k, title, body)), button, moved, waits, now, ctx.pointer_hover_pos());
    ctx.data_mut(|d| {
        match &state {
            Some(s) => {
                d.insert_temp(slot, s.clone());
            }
            None => d.remove::<Resting>(slot),
        }
        if waits {
            d.insert_temp(waiting, true);
        } else {
            d.remove::<bool>(waiting);
        }
    });
    let Some((_, _, _, since, shown)) = state else { return };
    if let Some((title, body, at)) = &shown {
        draw(ctx, env, *at, title, body);
    }
    if now - since < DELAY {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(DELAY - (now - since) + 0.01));
    }
}

/// [`hover`] for a window drawn with the interface library itself (an item window, the nano programs, the NCU window,
/// the shortcut bars): the tip of `target` in the game's skin and fonts.
pub fn show(ctx: &egui::Context, palette: &crate::theme::Palette, source: egui::Id, target: Option<egui::Id>, title: &str, body: &str) {
    let skin = crate::skin::skin(ctx);
    let measure = super::font::Measure { ctx: Some(ctx.clone()) };
    let env = Env { skin: skin.as_deref(), palette, measure: &measure };
    hover(ctx, &env, source, target, title, body);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Window::MoveToMouse` (0x101550de).
    #[test]
    fn a_tip_goes_right_of_and_under_the_pointer_or_flips() {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 800.0));
        let size = egui::vec2(200.0, 100.0);
        assert_eq!(place(egui::pos2(100.0, 100.0), size, screen), egui::pos2(116.0, 132.0));
        assert_eq!(place(egui::pos2(500.0, 400.0), size, screen), egui::pos2(299.0, 299.0));
        assert_eq!(place(egui::pos2(900.0, 100.0), size, screen), egui::pos2(699.0, 132.0));
        // (held inside the screen)
        assert_eq!(place(egui::pos2(50.0, 420.0), egui::vec2(200.0, 600.0), screen), egui::pos2(66.0, 0.0));
    }

    /// `WindowController_c::UpdateToolTip` (0x10157adc) and `Render` (0x10157580).
    #[test]
    fn a_tip_comes_after_half_a_second_and_a_button_takes_it_away() {
        let (a, b) = (egui::Id::unique("a"), egui::Id::unique("b"));
        let at = Some(egui::pos2(10.0, 10.0));
        let up = |s: &Option<Resting>| s.as_ref().and_then(|s| s.4.clone()).map(|t| t.0);
        // (resting on a thing: nothing for half a second, then its tip, where the pointer is then)
        let (s, w) = rest(None, Some((a, "A", "")), false, true, false, 0.0, at);
        assert_eq!((up(&s), w), (None, false));
        let (s, _) = rest(s, Some((a, "A", "")), false, false, false, 0.4, at);
        assert_eq!(up(&s), None);
        let (s, _) = rest(s, Some((a, "A", "")), false, false, false, 0.5, at);
        assert_eq!(up(&s).as_deref(), Some("A"));
        // (it does not follow the pointer)
        let (s, _) = rest(s, Some((a, "A", "")), false, true, false, 0.6, Some(egui::pos2(30.0, 30.0)));
        assert_eq!(s.as_ref().and_then(|s| s.4.clone()).map(|t| t.2), at);
        // (its text changed: the old tip stays for half a second more, then the new one)
        let (s, _) = rest(s, Some((a, "A2", "")), false, false, false, 1.0, at);
        assert_eq!(up(&s).as_deref(), Some("A"));
        let (s, _) = rest(s, Some((a, "A2", "")), false, false, false, 1.5, at);
        assert_eq!(up(&s).as_deref(), Some("A2"));
        // (another thing: the tip goes at once and the next waits its own half second)
        let (s, _) = rest(s, Some((b, "B", "")), false, true, false, 1.6, at);
        assert_eq!(up(&s), None);
        let (s, _) = rest(s, Some((b, "B", "")), false, false, false, 2.1, at);
        assert_eq!(up(&s).as_deref(), Some("B"));
        // (a button down or up takes it away, and it stays away until the pointer has moved and rested again)
        let (s, w) = rest(s, Some((b, "B", "")), true, false, false, 2.2, at);
        assert_eq!((up(&s), w), (None, true));
        let (s, w) = rest(s, Some((b, "B", "")), false, false, w, 5.0, at);
        assert_eq!((up(&s), w), (None, true));
        let (s, w) = rest(s, Some((b, "B", "")), false, true, w, 5.1, at);
        assert_eq!((up(&s), w), (None, false));
        let (s, _) = rest(s, Some((b, "B", "")), false, false, w, 5.6, at);
        assert_eq!(up(&s).as_deref(), Some("B"));
        // (nothing with a tip under the pointer: none)
        assert_eq!(rest(s, None, false, true, false, 6.0, at).0.is_none(), true);
        assert_eq!(plain("<font color=CCRed>Hot</font><br>NCU usage:&nbsp;5"), " Hot  NCU usage: 5");
    }
}
