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
    /// Enter in a text field (`TextView_c::SlotEnterPressed`, a `TextInputView_c`'s `SlotEnterPressed`).
    Enter,
    /// A text field's text changed (`SlotEdited`).
    Edited,
    /// A scroll view scrolled (the tree is laid out again; not passed on).
    Scrolled,
    /// A drop-down's entry was chosen (`SelectionChanged`): its index.
    Selected(i64),
    /// A radio button group's choice changed (`ValueChanged`): the chosen button's value.
    Value(String),
    /// A list's column label was clicked (`ColumnHeaderView_c`'s `SignalColumnSelected`): its id.
    ColumnSelected(i32),
    /// A list's column was dragged to a width (`SignalColumnResized`).
    ColumnResized(i32, f32),
    /// A double click on a list column's edge (`ResizeColumnToFit`).
    ColumnFit(i32),
    /// A list's row was clicked: its id and the click count (`MultiListView_c`'s item-clicked signal).
    ItemClicked(i64, u32),
}

/// The keys a text view acts on (`TextRenderer_c::KeyDown`'s cases: 0x18 Enter, 0x19 Backspace, 0x1b Delete, 0x1c Home,
/// 0x1d End, 0x20 Left, 0x21 Right, 0x22 Up, 0x23 Down; 0x1a Insert and 0x67 V paste with Shift or Ctrl).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Enter,
    Backspace,
    Delete,
    Home,
    End,
    Left,
    Right,
    Up,
    Down,
    Insert,
    V,
    A,
}

/// The modifiers of a key (the original's bits 3 Shift, 0xc Ctrl, 0x30 Alt).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Whether a view tree has the keyboard this frame (the game's keys then go to the interface, as for a text field).
static KEYBOARD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn keyboard_taken() -> bool {
    KEYBOARD.load(std::sync::atomic::Ordering::Relaxed)
}

/// A new frame: no tree has the keyboard until one says so.
pub fn begin_frame() {
    KEYBOARD.store(false, std::sync::atomic::Ordering::Relaxed);
}

/// Gives a view the keyboard (`View::MakeFocus`), or takes it away.
pub fn set_focus(tree: &mut Tree, id: Option<Id>) {
    if tree.focus == id {
        return;
    }
    if let Some(old) = tree.focus {
        tree.get_mut(old).control.focus(false);
    }
    tree.focus = id;
    tree.focus_new = id.is_some();
    if let Some(new) = id {
        tree.get_mut(new).control.focus(true);
    }
}

/// The next view that takes the keyboard after `from`, in the tree's order, wrapping round (Tab, `SlotTabPressed`).
fn next_focus(tree: &Tree, root: Id, from: Id) -> Option<Id> {
    let mut views = Vec::new();
    order(tree, root, &mut views);
    let ring: Vec<Id> = views.into_iter().filter(|&v| tree.get(v).control.focusable() && enabled(tree, v)).collect();
    let at = ring.iter().position(|&v| v == from)?;
    ring.get((at + 1) % ring.len()).copied()
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

/// A view's signal: a scroll lays the tree out again, the others go to the window's code.
fn push(tree: &mut Tree, v: Id, s: Signal) {
    if s == Signal::Scrolled {
        tree.relayout = true;
    } else {
        tree.signals.push((v, s));
    }
}

/// Where a view may draw and take the mouse: the window's rect within every clipping ancestor's (a scroll child's).
fn clip(tree: &Tree, v: Id, origin: egui::Pos2, window: egui::Rect) -> egui::Rect {
    let mut r = window;
    let mut at = tree.get(v).parent;
    while let Some(a) = at {
        if tree.get(a).control.clips() {
            r = r.intersect(screen_rect(tree, a, origin));
        }
        at = tree.get(a).parent;
    }
    r
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
    if tree.get(root).frame != want || tree.relayout {
        tree.relayout = false;
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
    let probe = Some(tree.measure.width(super::font::FontId::NORMAL, "Mg"));
    if tree.fitted_probe != probe {
        tree.fitted_probe = probe;
        tree.relayout = true;
    }
    // (the scroll bars as wide as the skin's arrows: `ScrollBar_c::GetDefaultWidth`)
    if let Some(w) = skin.and_then(|s| s.get(super::scroll::UP)).map(|s| s.size.x) {
        for n in tree.nodes.iter_mut() {
            if let Some(v) = n.control.as_any_mut().downcast_mut::<super::scroll::ScrollView>()
                && v.bar_width.get() != w
            {
                v.bar_width.set(w);
                tree.relayout = true;
            }
        }
    }
    let min = preferred_pixels(tree, root, false);
    let size = ui.available_size().max(min);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    fit(tree, root, rect.size());
    let origin = rect.min;
    let mut views = Vec::new();
    order(tree, root, &mut views);

    // the mouse
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let clip_of = |tree: &Tree, v: Id| clip(tree, v, origin, rect);
    let under = pointer.filter(|p| rect.contains(*p)).and_then(|p| {
        views.iter().rev().copied().find(|&v| tree.get(v).control.takes_mouse() && enabled(tree, v) && screen_rect(tree, v, origin).contains(p) && clip_of(tree, v).contains(p))
    });
    let at = |tree: &Tree, v: Id| {
        let p = pointer.unwrap_or_default();
        let r = screen_rect(tree, v, origin);
        Point::new(p.x - r.left(), p.y - r.top())
    };
    let (down, up) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released()));
    if down && response.hovered() {
        tree.pressed = under;
        // (a click on a view that types takes the keyboard; elsewhere in the window it lets it go)
        let focus = under.filter(|&v| tree.get(v).control.focusable());
        set_focus(tree, focus);
        if let Some(v) = under {
            let p = at(tree, v);
            if let Some(s) = tree.get_mut(v).control.mouse(super::host::Mouse::Down, p) {
                push(tree, v, s);
            }
        }
    }
    if let Some(v) = tree.pressed {
        if ui.input(|i| i.pointer.is_moving()) {
            let p = at(tree, v);
            if let Some(s) = tree.get_mut(v).control.mouse(Mouse::Move, p) {
                push(tree, v, s);
            }
        }
        if up {
            let p = at(tree, v);
            let inside = under == Some(v);
            if let Some(s) = tree.get_mut(v).control.mouse(Mouse::Up { inside }, p) {
                push(tree, v, s);
            }
            tree.pressed = None;
        }
    }
    tree.hover = under;

    // the keyboard: the focused view's, while egui's focus stays on the tree (a click elsewhere takes it away)
    if let Some(f) = tree.focus.filter(|&f| !tree.is_visible(f) || !enabled(tree, f)) {
        let _ = f;
        set_focus(tree, None);
    }
    if tree.focus.is_some() {
        if tree.focus_new {
            tree.focus_new = false;
            response.request_focus();
        } else if !response.has_focus() {
            set_focus(tree, None);
        }
    }
    if let Some(f) = tree.focus {
        ui.memory_mut(|m| m.set_focus_lock_filter(response.id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true }));
        KEYBOARD.store(true, std::sync::atomic::Ordering::Relaxed);
        let events = ui.input(|i| i.events.clone());
        let mut focus = Some(f);
        for event in events {
            let Some(v) = focus else { break };
            let signal = match event {
                egui::Event::Text(t) => tree.get_mut(v).control.text(&t),
                egui::Event::Paste(t) => tree.get_mut(v).control.text(&t),
                egui::Event::Copy => {
                    if let Some(t) = tree.get(v).control.copy() {
                        ui.ctx().copy_text(t);
                    }
                    None
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    let mods = Mods { shift: modifiers.shift, ctrl: modifiers.ctrl || modifiers.command, alt: modifiers.alt };
                    let k = match key {
                        egui::Key::Tab => {
                            focus = next_focus(tree, root, v).or(Some(v));
                            None
                        }
                        egui::Key::Escape => {
                            focus = None;
                            None
                        }
                        egui::Key::Enter => Some(Key::Enter),
                        egui::Key::Backspace => Some(Key::Backspace),
                        egui::Key::Delete => Some(Key::Delete),
                        egui::Key::Home => Some(Key::Home),
                        egui::Key::End => Some(Key::End),
                        egui::Key::ArrowLeft => Some(Key::Left),
                        egui::Key::ArrowRight => Some(Key::Right),
                        egui::Key::ArrowUp => Some(Key::Up),
                        egui::Key::ArrowDown => Some(Key::Down),
                        egui::Key::Insert => Some(Key::Insert),
                        egui::Key::A => Some(Key::A),
                        _ => None,
                    };
                    k.and_then(|k| tree.get_mut(v).control.key(k, mods))
                }
                _ => None,
            };
            if let Some(s) = signal {
                tree.signals.push((v, s));
            }
        }
        if focus != tree.focus {
            set_focus(tree, focus);
            tree.focus_new = false;
            if focus.is_none() {
                response.surrender_focus();
            }
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    }
    // (the wheel: the topmost view under the pointer that uses it)
    let wheel = ui.input(|i| i.smooth_scroll_delta.y);
    if wheel != 0.0 {
        if let Some(p) = pointer.filter(|p| rect.contains(*p)) {
            for &v in views.iter().rev() {
                let r = screen_rect(tree, v, origin);
                if r.contains(p) && clip_of(tree, v).contains(p) && tree.get_mut(v).control.wheel(wheel, r.height()) {
                    tree.relayout = true;
                    break;
                }
            }
        }
    }

    // the pictures
    let measure = tree.measure.clone();
    let env = Env { skin, palette, measure: &measure };
    let painter = ui.painter_at(rect);
    for &v in &views {
        let clipped = painter.with_clip_rect(clip(tree, v, origin, rect));
        let p = Paint {
            painter: &clipped,
            env: &env,
            rect: screen_rect(tree, v, origin),
            hovered: tree.hover == Some(v) && (tree.pressed.is_none() || tree.pressed == Some(v)),
            pressed: tree.pressed == Some(v) && tree.hover == Some(v),
            enabled: enabled(tree, v),
        };
        tree.get(v).control.paint(&p);
    }
    // (the tip of the topmost view under the pointer that has one: `WindowController_c::UpdateToolTip`)
    let tipped = pointer.filter(|p| rect.contains(*p) && response.hovered()).and_then(|p| {
        views.iter().rev().copied().find(|&v| (!tree.get(v).tooltip.is_empty() || !tree.get(v).tooltip_body.is_empty()) && screen_rect(tree, v, origin).contains(p) && clip(tree, v, origin, rect).contains(p))
    });
    {
        let (title, body) = tipped.map_or((String::new(), String::new()), |v| (tree.get(v).tooltip.clone(), tree.get(v).tooltip_body.clone()));
        super::tooltip::hover(ui.ctx(), &env, response.id, tipped.map(|v| response.id.with(("tip", v))), &title, &body);
    }
    // (the radio button groups' part in the signals)
    super::radio::take_signals(tree);
    // (what is drawn over everything: an open drop-down's list; the combo boxes' lists placed first)
    super::combobox::prepare(tree, origin);
    let ctx = ui.ctx().clone();
    let mut overlay_views = Vec::new();
    order(tree, root, &mut overlay_views);
    for v in overlay_views {
        let r = screen_rect(tree, v, origin);
        let id = response.id.with(("overlay", v));
        if let Some(s) = tree.get_mut(v).control.overlay(&ctx, &env, r, id) {
            push(tree, v, s);
        }
    }
    super::combobox::take_signals(tree);
    response
}
