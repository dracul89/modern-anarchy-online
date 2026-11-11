//! The white the world fades to while one's own character lies dead, gone again once the server has set it down where
//! it comes back to life.
//!
//! From the user's description of the original (2026-11-11: "the screen fades to white then you teleport to spawn",
//! when dying). The original's code for it was looked for and not found (not in GUI.dll's `DeadMessage` or
//! `TeleportStartedMessage`, not among Gamecode's `VisualFader_t`s, which are the weather's and the playfield's black
//! one), so the times here are this client's guesses: the white rises over the minute the server leaves the dead
//! character lying ([`RISE`]) and clears over [`CLEAR`] after the arrival. Only the world goes white: the windows and
//! the chat stay as they are.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long the white takes to cover the world (the server moved the dead character 60 s after its death in the live
/// runs of 2026-11-11).
pub const RISE: Duration = Duration::from_secs(60);
/// How long it takes to clear once the character is alive again.
pub const CLEAR: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, Debug)]
struct Fade {
    began: Instant,
    /// When the character was alive again, and how white the world was then.
    cleared: Option<(Instant, f32)>,
}

static FADE: Mutex<Option<Fade>> = Mutex::new(None);

fn fade() -> std::sync::MutexGuard<'static, Option<Fade>> {
    FADE.lock().unwrap_or_else(|e| e.into_inner())
}

/// One's own character died: the white begins (a death while it is already rising changes nothing).
pub fn begin() {
    let mut f = fade();
    if f.is_none_or(|f| f.cleared.is_some()) {
        *f = Some(Fade { began: Instant::now(), cleared: None });
    }
}

/// Alive again. `arrived` when the server set the character down somewhere (the world is wholly white as it arrives,
/// however long it lay); otherwise (health given back where it lies) the white clears from what it had reached.
pub fn clear(arrived: bool) {
    if let Some(f) = fade().as_mut() {
        if f.cleared.is_none() {
            let now = Instant::now();
            f.cleared = Some((now, if arrived { 1.0 } else { level(f, now).unwrap_or(0.0) }));
        }
    }
}

/// How white the world is now (0 to 1); none once it has cleared.
fn level(f: &Fade, now: Instant) -> Option<f32> {
    match f.cleared {
        None => Some((now.duration_since(f.began).as_secs_f32() / RISE.as_secs_f32()).min(1.0)),
        Some((at, from)) => {
            let left = 1.0 - (now.duration_since(at).as_secs_f32() / CLEAR.as_secs_f32()).min(1.0);
            (left > 0.0).then_some(from * left)
        }
    }
}

/// Paints the white over the world (under everything else the painter draws after it).
pub fn draw(ctx: &egui::Context, painter: &egui::Painter, screen: egui::Rect) {
    let Some(f) = *fade() else { return };
    let Some(level) = level(&f, Instant::now()) else {
        *fade() = None;
        return;
    };
    ctx.request_repaint();
    painter.rect_filled(screen, 0.0, egui::Color32::from_white_alpha((level * 255.0) as u8));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_white_rises_while_dead_and_clears_after() {
        let began = Instant::now();
        let dead = Fade { began, cleared: None };
        assert_eq!(level(&dead, began), Some(0.0));
        assert_eq!(level(&dead, began + RISE / 2), Some(0.5));
        assert_eq!(level(&dead, began + RISE * 3), Some(1.0));
        // (set down at the place of return after 20 s: white as it arrives, then clearing)
        let at = began + Duration::from_secs(20);
        let back = Fade { began, cleared: Some((at, 1.0)) };
        assert_eq!(level(&back, at), Some(1.0));
        assert_eq!(level(&back, at + CLEAR / 2), Some(0.5));
        assert_eq!(level(&back, at + CLEAR), None);
        // (given health where it lay, a third of the way up: clears from there)
        let healed = Fade { began, cleared: Some((at, 1.0 / 3.0)) };
        assert!((level(&healed, at + CLEAR / 2).unwrap() - 1.0 / 6.0).abs() < 1e-6);
    }
}
