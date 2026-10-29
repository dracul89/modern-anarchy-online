//! `PopupMenu_c`: a list of entries in a small window of its own over everything else.
//!
//! `Go` (0x1013c13a) makes the window: a `BorderView_c` of the `GFX_GUI_INSET_*` pieces over `GFX_GUI_WINDOW_BACKGROUND`
//! (0xf9 ... 0xf5, 0x1bf) holding, 3 in on every side, a scroll view (vertical bar on `auto`) of the menu; moved inside the
//! screen. `_Layout` (0x1013b9c9) stacks the entries from the top, 3 apart (`_DAT_101a8a28`), each as wide as the widest
//! icon, label and tail together plus 2 (`_DAT_101a8b98`); the menu is at most the screen's height less 100 tall
//! (`CalculatePreferredSize` 0x1013b7f3). An entry (`PopupMenuItem_c` 0x1013dee8, `PopupMenuStringItem_c` 0x1013e6ce)
//! is its label (a `TextView_c` in font 5) a pixel right of its icon, centred in its height; white, or 0xa0a0a0 when
//! disabled (`EnabledStateChanged` 0x1013e8ed); the one under the pointer lies on a 0x2020c0 band (`Highlight`
//! 0x1013dd7d). Releasing the button on an entry chooses it (`MouseUp`, `ActivateHighlightedItem`) and closes the menu;
//! a click outside it closes it (`SlotGlobalMouseDown`); Up, Down, Enter and Escape work it from the keys (`KeyDown`).

use super::font::FontId;
use super::host::Env;

/// An entry: its label and whether it can be chosen.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub label: String,
    pub enabled: bool,
}

/// What a frame of the menu did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    None,
    /// An entry was chosen (its index); the menu closed.
    Chosen(usize),
    /// It closed without a choice.
    Closed,
}

/// An open menu.
#[derive(Clone, Debug, Default)]
pub struct PopupMenu {
    pub entries: Vec<Entry>,
    /// The entry under the pointer or picked by the keys (`HighlightItem`).
    pub highlighted: Option<usize>,
    /// Where it opened (its top left on the screen).
    pub at: egui::Pos2,
    /// The frame it opened in (the click that opened it does not close it).
    opened: u64,
}

/// The entries' gap and the frame's room round them.
const GAP: f32 = 3.0;
const INSET: f32 = 3.0;

impl PopupMenu {
    pub fn new(entries: Vec<Entry>, at: egui::Pos2, highlighted: Option<usize>, frame: u64) -> PopupMenu {
        PopupMenu { entries, highlighted, at, opened: frame }
    }

    /// One frame of the menu: drawn at `at`, the mouse and keys handled.
    pub fn show(&mut self, ctx: &egui::Context, env: &Env, id: egui::Id) -> Outcome {
        let m = env.measure;
        let h = m.height(FontId::NORMAL).ceil();
        let widest = self.entries.iter().map(|e| m.width(FontId::NORMAL, &e.label)).fold(0.0f32, f32::max);
        let width = (widest + 2.0).ceil();
        let pitch = h + GAP;
        let height = (self.entries.len() as f32 * pitch - GAP).max(h);
        // (the inset frame's pieces round the room)
        let piece = |n: &str| env.skin.and_then(|s| s.get(n)).map_or(egui::Vec2::splat(2.0), |s| s.size);
        let (l, t, r, b) = (piece("GFX_GUI_INSET_LEFT").x, piece("GFX_GUI_INSET_TOP").y, piece("GFX_GUI_INSET_RIGHT").x, piece("GFX_GUI_INSET_BOTTOM").y);
        let screen = ctx.content_rect();
        let max_height = screen.height() - 100.0;
        let outer = egui::vec2(width + 2.0 * INSET + l + r, (height + 2.0 * INSET + t + b).min(max_height));
        // (`MoveInsideScreen`)
        let at = egui::pos2(self.at.x.min(screen.right() - outer.x).max(screen.left()), self.at.y.min(screen.bottom() - outer.y).max(screen.top()));
        let rect = egui::Rect::from_min_size(at, outer);
        let first = egui::pos2(rect.left() + l + INSET, rect.top() + t + INSET);
        let entry_rect = |k: usize| egui::Rect::from_min_size(egui::pos2(first.x, first.y + k as f32 * pitch), egui::vec2(width, h));
        let mut outcome = Outcome::None;
        let frame = ctx.cumulative_frame_nr();
        egui::Area::new(id).order(egui::Order::Foreground).fixed_pos(at).interactable(true).show(ctx, |ui| {
            let (_, response) = ui.allocate_exact_size(outer, egui::Sense::click());
            let painter = ui.painter();
            if let Some(skin) = env.skin {
                skin.draw_in(painter, "GFX_GUI_WINDOW_BACKGROUND", rect.shrink2(egui::vec2(l, t)), egui::Color32::WHITE);
                let pieces = super::controls::Pieces::named("GFX_GUI_INSET", false);
                let p = super::host::Paint { painter, env, rect, hovered: false, pressed: false, enabled: true };
                super::controls::nine_slice(&p, &pieces, rect, egui::Color32::WHITE);
            } else {
                painter.rect_filled(rect, 0.0, egui::Color32::from_gray(30));
            }
            let pointer = ui.input(|i| i.pointer.hover_pos());
            if let Some(p) = pointer.filter(|p| response.rect.contains(*p) && ui.input(|i| i.pointer.is_moving())) {
                self.highlighted = (0..self.entries.len()).find(|&k| entry_rect(k).contains(p) && self.entries[k].enabled);
            }
            let clip = painter.with_clip_rect(rect.shrink2(egui::vec2(l, t)));
            for (k, e) in self.entries.iter().enumerate() {
                let r = entry_rect(k);
                if self.highlighted == Some(k) {
                    clip.rect_filled(r, 0.0, egui::Color32::from_rgb(0x20, 0x20, 0xc0));
                }
                let colour = if e.enabled { egui::Color32::WHITE } else { egui::Color32::from_gray(0xa0) };
                let p = super::host::Paint { painter: &clip, env, rect: r, hovered: false, pressed: false, enabled: e.enabled };
                super::controls::text(&p, FontId::NORMAL, egui::pos2(r.left() + 1.0, r.top() + ((h - m.height(FontId::NORMAL)) * 0.5).floor()), &e.label, colour);
            }
            if response.clicked()
                && let Some(p) = response.interact_pointer_pos()
                && let Some(k) = (0..self.entries.len()).find(|&k| entry_rect(k).contains(p) && self.entries[k].enabled)
            {
                outcome = Outcome::Chosen(k);
            }
            // (a press elsewhere closes it; not the one that opened it)
            if outcome == Outcome::None && frame > self.opened + 1 && ui.input(|i| i.pointer.any_pressed()) && pointer.is_some_and(|p| !rect.contains(p)) {
                outcome = Outcome::Closed;
            }
            // (the keys)
            ui.input(|i| {
                let enabled: Vec<usize> = (0..self.entries.len()).filter(|&k| self.entries[k].enabled).collect();
                if i.key_pressed(egui::Key::ArrowDown) {
                    self.highlighted = match self.highlighted {
                        Some(h) => enabled.iter().copied().find(|&k| k > h).or(Some(h)),
                        None => enabled.first().copied(),
                    };
                }
                if i.key_pressed(egui::Key::ArrowUp) {
                    self.highlighted = match self.highlighted {
                        Some(h) => enabled.iter().copied().rev().find(|&k| k < h).or(Some(h)),
                        None => enabled.last().copied(),
                    };
                }
                if i.key_pressed(egui::Key::Enter)
                    && let Some(h) = self.highlighted
                {
                    outcome = Outcome::Chosen(h);
                }
                if i.key_pressed(egui::Key::Escape) {
                    outcome = Outcome::Closed;
                }
            });
        });
        ctx.request_repaint();
        outcome
    }
}
