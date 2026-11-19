//! The white the world turns while one's own character lies dead, gone again once it lives.
//!
//! The original's `CharDie_t` (its frame, Gamecode FUN_1007bad5) makes the effect 3000 on the client's own character
//! when its death's animation has played out (and three seconds later tells the server, which sets the character down
//! where it comes back to life: `net.rs`, `OwnDeath`). Template 3000 of `gfxtweak.bin` is of kind 1015, which
//! `_EffectHandler_t::CreateGfxControl` (0x100ce912) makes a `_GfxControlWhiteOut_t` (FUN_101052b2): a sprite over the
//! world (no depth test, no lighting: FUN_10105186), its life -1 (`_DAT_10156dd0`: until deleted), whose frame
//! (FUN_1010509a) colours it white with an alpha of its age over the template's first word, clamped to 0..1. Template
//! 3000 has no words, so that time is 0 and the white is whole at once. The effect is deleted (FUN_1003e61c, from the
//! revival FUN_1003ec1f, which the character's alive message FUN_10076f06 calls) the moment the character lives again:
//! the white goes at once. Only the world goes white: the windows and the chat stay as they are.

use std::sync::atomic::{AtomicBool, Ordering};

static WHITE: AtomicBool = AtomicBool::new(false);

/// One's own character's death has played out: the white effect is made.
pub fn begin() {
    WHITE.store(true, Ordering::Relaxed);
}

/// Alive again (set down somewhere, or given health where it lies): the effect is deleted.
pub fn clear() {
    WHITE.store(false, Ordering::Relaxed);
}

/// How white the world is (`_GfxControlWhiteOut_t`'s frame FUN_1010509a: the age over the template's time, 0 for 3000,
/// clamped to 0..1; the alpha `round(level * 255 - 0.49999)`).
fn alpha(time: f32, age: f32) -> u8 {
    let level = (age / time).clamp(0.0, 1.0);
    (level * 255.0 - 0.49999).round() as u8
}

/// Paints the white over the world (under everything else the painter draws after it).
pub fn draw(_ctx: &egui::Context, painter: &egui::Painter, screen: egui::Rect) {
    if WHITE.load(Ordering::Relaxed) {
        painter.rect_filled(screen, 0.0, egui::Color32::from_white_alpha(alpha(0.0, 1.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DTH-007: template 3000's time is 0, so the white is whole from the first frame it has an age.
    #[test]
    fn the_white_is_whole_at_once() {
        assert_eq!(alpha(0.0, 0.016), 255);
        assert_eq!(alpha(0.0, 100.0), 255);
        // (a template with a time would rise over it)
        assert_eq!(alpha(2.0, 1.0), 127);
    }
}
