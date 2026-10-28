//! Showing a view tree in an egui window: the tree is sized to the space the window gives it (`Window`'s root view frame),
//! drawn bottom to top (`View::_CallRender`: a view, then its children over it), and the mouse goes to the topmost view
//! under it that takes the mouse, which keeps it while a button is held (`Window`'s capture: `MouseUp` goes to the view
//! the button went down on).
//!
//! The drawing goes through egui's painter (the original draws `ViewSurface_c` sprites through DisplaySystem); one unit of
//! the views is one egui point.

use super::font::Measure;
use super::geom::{Point, Rect};
use super::view::{Id, Tree};
use crate::theme::Palette;

/// What a view is drawn with.
pub struct Env<'a> {
    pub skin: Option<&'a crate::skin::Skin>,
    pub palette: &'a Palette,
    pub measure: &'a Measure,
}

/// A view being drawn: its rect on screen (pixels: the view's frame plus one), whether it is under the pointer, pressed
/// and enabled.
pub struct Paint<'a> {
    pub painter: &'a egui::Painter,
    pub env: &'a Env<'a>,
    pub rect: egui::Rect,
    pub hovered: bool,
    pub pressed: bool,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mouse {
    Down,
    /// The button let go: over the view or not.
    Up { inside: bool },
    Move,
}

/// What a view tells its window's code.
#[derive(Clone, Debug, PartialEq)]
pub enum Signal {
    /// A button was clicked (`Button_c`'s `SignalClicked`).
    Clicked,
    /// A toggle button's state changed.
    Toggled(bool),
    /// A link in a text view was clicked: its target (`text://...`, `itemref://...`, `chatcmd://...`).
    Link(String),
}

/// A view's rect on screen, the window's content starting at `origin`.
pub fn screen_rect(tree: &Tree, id: Id, origin: egui::Pos2) -> egui::Rect {
    let r = tree.window_frame(id);
    egui::Rect::from_min_max(origin + egui::vec2(r.left, r.top), origin + egui::vec2(r.right + 1.0, r.bottom + 1.0))
}

fn enabled(tree: &Tree, mut id: Id) -> bool {
    loop {
        if !tree.get(id).enabled {
            return false;
        }
        match tree.get(id).parent {
            Some(p) => id = p,
            None => return true,
        }
    }
}

/// The views in drawing order (bottom to top), visible ones only.
fn order(tree: &Tree, id: Id, out: &mut Vec<Id>) {
    if tree.get(id).hidden > 0 {
        return;
    }
    out.push(id);
    for &c in &tree.get(id).children {
        order(tree, c, out);
    }
}

/// Lays the tree out to `size` (pixels) when it is not that size already.
pub fn fit(tree: &mut Tree, root: Id, size: egui::Vec2) {
    let want = Rect::new(0.0, 0.0, size.x.floor() - 1.0, size.y.floor() - 1.0);
    if tree.get(root).frame != want {
        tree.set_frame(root, want);
    }
}

/// The tree's preferred size in pixels (its minimum, or its maximum).
pub fn preferred_pixels(tree: &Tree, root: Id, max: bool) -> egui::Vec2 {
    let p = tree.preferred(root, max);
    egui::vec2(p.x + 1.0, p.y + 1.0)
}

/// Shows the tree in the space `ui` has left (at least its minimum size), handles the mouse and draws it.
pub fn show(ui: &mut egui::Ui, tree: &mut Tree, root: Id, skin: Option<&crate::skin::Skin>, palette: &Palette) -> egui::Response {
    tree.measure = Measure { ctx: Some(ui.ctx().clone()) };
    let min = preferred_pixels(tree, root, false);
    let size = ui.available_size().max(min);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    fit(tree, root, rect.size());
    let origin = rect.min;
    let mut views = Vec::new();
    order(tree, root, &mut views);

    // the mouse
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let under = pointer.filter(|p| rect.contains(*p)).and_then(|p| {
        views.iter().rev().copied().find(|&v| tree.get(v).control.takes_mouse() && enabled(tree, v) && screen_rect(tree, v, origin).contains(p))
    });
    let at = |tree: &Tree, v: Id| {
        let p = pointer.unwrap_or_default();
        let r = screen_rect(tree, v, origin);
        Point::new(p.x - r.left(), p.y - r.top())
    };
    let (down, up) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released()));
    if down && response.hovered() {
        tree.pressed = under;
        if let Some(v) = under {
            let p = at(tree, v);
            if let Some(s) = tree.get_mut(v).control.mouse(super::host::Mouse::Down, p) {
                tree.signals.push((v, s));
            }
        }
    }
    if let Some(v) = tree.pressed {
        if ui.input(|i| i.pointer.is_moving()) {
            let p = at(tree, v);
            tree.get_mut(v).control.mouse(Mouse::Move, p);
        }
        if up {
            let p = at(tree, v);
            let inside = under == Some(v);
            if let Some(s) = tree.get_mut(v).control.mouse(Mouse::Up { inside }, p) {
                tree.signals.push((v, s));
            }
            tree.pressed = None;
        }
    }
    tree.hover = under;
    // (the wheel: the topmost view under the pointer that uses it)
    let wheel = ui.input(|i| i.smooth_scroll_delta.y);
    if wheel != 0.0 {
        if let Some(p) = pointer.filter(|p| rect.contains(*p)) {
            for &v in views.iter().rev() {
                let r = screen_rect(tree, v, origin);
                if r.contains(p) && tree.get_mut(v).control.wheel(wheel, r.height()) {
                    break;
                }
            }
        }
    }

    // the pictures
    let measure = tree.measure.clone();
    let env = Env { skin, palette, measure: &measure };
    let painter = ui.painter_at(rect);
    for v in views {
        let p = Paint {
            painter: &painter,
            env: &env,
            rect: screen_rect(tree, v, origin),
            hovered: tree.hover == Some(v) && (tree.pressed.is_none() || tree.pressed == Some(v)),
            pressed: tree.pressed == Some(v) && tree.hover == Some(v),
            enabled: enabled(tree, v),
        };
        tree.get(v).control.paint(&p);
    }
    response
}
