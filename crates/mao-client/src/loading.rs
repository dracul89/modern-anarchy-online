//! The loading screen: GUI.dll's `ServerLogin3DModule_t` (`SetupScreen` 0x10017331, `FrameProcess` 0x1001713c,
//! `SetAlpha` 0x10016eed). Over the whole screen a black sheet (a 16 by 16 sprite scaled by a sixteenth of the display);
//! in the middle the picture `gfx/ai_loading_login.png` from the GUI folder (`welcome_to_rubika.jpg` instead after a
//! character was made: `SetLoadingScreen(rand % 3)`, types 1 and 2), scaled from 1024 by 768 to the display
//! (`ScaleToResolution(1024, 768)`, filtered, centred); text.mdb's `AO_Loading` in font 10, colour 0xdddddd, centred 50
//! pixels above the bottom. Entering the game (`InitialiseMessage`) the picture and the text fade in over 4 s on the black;
//! once the world is in (`StartClosingLoadscreenMessage`) all of it fades out over 7 s.
//!
//! This client's own: the picture is tinted blue ([`TINT`], each channel multiplied), and the screen also covers a zoning
//! (shown at once, fading out over [`ZONE_FADE_OUT`] when the new playfield is drawn).
//!
//! The game is a process of its own here, which logs in and reads the playfield before it has a window: the character
//! list's window puts the screen up when Play is pressed and keeps it until the game's window has drawn (`launcher.rs`),
//! and the game's goes on from there ([`begin_since`]), so the fade in is one.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The blue the picture is tinted with (red, green, blue multipliers; the blue may brighten).
pub const TINT: [f32; 3] = [0.35, 0.6, 1.45];
/// The original's fades: in while entering, out once in (`_DAT_101aa1a0`, `_DAT_101aa198`).
pub const FADE_IN: Duration = Duration::from_millis(4000);
pub const FADE_OUT: Duration = Duration::from_millis(7000);
/// A zoning's fade out (the client's own).
pub const ZONE_FADE_OUT: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, Debug)]
struct Load {
    began: Instant,
    fade_in: Duration,
    fade_out: Duration,
    done: Option<Instant>,
}

static LOAD: Mutex<Option<Load>> = Mutex::new(None);
static PICTURE: Mutex<Option<egui::TextureHandle>> = Mutex::new(None);

fn load() -> std::sync::MutexGuard<'static, Option<Load>> {
    LOAD.lock().unwrap_or_else(|e| e.into_inner())
}

/// The screen goes up (unless it already is): the picture fading in over `fade_in`, out over `fade_out` once [`done`].
pub fn begin(fade_in: Duration, fade_out: Duration) {
    let mut l = load();
    if l.is_none_or(|l| l.done.is_some()) {
        *l = Some(Load { began: Instant::now(), fade_in, fade_out, done: None });
    }
}

/// The screen goes up as one that has been up for `elapsed` already (in another window: the character list's).
pub fn begin_since(elapsed: Duration, fade_in: Duration, fade_out: Duration) {
    let now = Instant::now();
    *load() = Some(Load { began: now.checked_sub(elapsed).unwrap_or(now), fade_in, fade_out, done: None });
}

/// When a screen put up `since` (milliseconds of the system's clock, as [`clock_millis`] gives them) was put up, as
/// time passed until `now`; nothing for a clock that went backwards.
pub fn elapsed_since(since: u64, now: u64) -> Duration {
    Duration::from_millis(now.saturating_sub(since))
}

/// The system's clock in milliseconds (handed from the character list's process to the game's).
pub fn clock_millis() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// The world is in: the screen fades out.
pub fn done() {
    if let Some(l) = load().as_mut() {
        l.done.get_or_insert_with(Instant::now);
    }
}

/// The screen's and the picture's opacity now (`FrameProcess`); none once it has faded out.
fn alphas(l: &Load, now: Instant) -> Option<(f32, f32)> {
    let ratio = |d: Duration, of: Duration| if of.is_zero() { 1.0 } else { (d.as_secs_f32() / of.as_secs_f32()).min(1.0) };
    let shown = ratio(now.duration_since(l.began), l.fade_in);
    match l.done {
        None => Some((1.0, shown)),
        Some(at) => {
            let left = 1.0 - ratio(now.duration_since(at), l.fade_out);
            (left > 0.0).then_some((left, shown * left))
        }
    }
}

/// The picture, tinted (made once).
fn picture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let mut slot = PICTURE.lock().unwrap_or_else(|e| e.into_inner());
    if slot.is_none() {
        // (a new character: `SetLoadingScreen(rand() % 3)` at its creation, types 1 and 2 the welcome picture)
        let welcome = std::env::var("MAO_NEW_CHARACTER").is_ok_and(|v| v == "1") && std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos()) % 3 != 0;
        let file = if welcome { "welcome_to_rubika.jpg" } else { "ai_loading_login.png" };
        let path = crate::theme::game_dir()?.join("cd_image/gui/Default/gfx").join(file);
        let image = image::open(mao_formats::files::find(&path)).ok()?.to_rgba8();
        let (w, h) = image.dimensions();
        let mut rgba = image.into_raw();
        tint(&mut rgba, TINT);
        *slot = Some(ctx.load_texture("loading-screen", egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba), egui::TextureOptions::LINEAR));
    }
    slot.clone()
}

/// Multiplies each pixel's colour by `tint` (clamped), leaving its alpha.
pub fn tint(rgba: &mut [u8], tint: [f32; 3]) {
    for px in rgba.chunks_exact_mut(4) {
        for c in 0..3 {
            px[c] = (px[c] as f32 * tint[c]).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Draws the screen over everything while it is up.
pub fn draw(ctx: &egui::Context, screen: egui::Rect) {
    let Some(l) = *load() else { return };
    let Some((sheet, shown)) = alphas(&l, Instant::now()) else {
        *load() = None;
        return;
    };
    ctx.request_repaint();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("loading-screen")));
    painter.rect_filled(screen, 0.0, egui::Color32::from_black_alpha((sheet * 255.0) as u8));
    let white = egui::Color32::from_white_alpha((shown * 255.0) as u8);
    if let Some(texture) = picture(ctx) {
        // (from 1024 by 768 to the display, kept in proportion)
        let scale = (screen.width() / 1024.0).min(screen.height() / 768.0);
        let size = egui::vec2(1024.0, 768.0) * scale;
        painter.image(texture.id(), egui::Rect::from_center_size(screen.center(), size), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), white);
    }
    let text = crate::feedback::texts().and_then(|t| t.named(10000, "AO_Loading")).unwrap_or("Loading...").to_string();
    let colour = egui::Color32::from_rgba_unmultiplied(0xdd, 0xdd, 0xdd, (shown * 255.0) as u8);
    let at = egui::pos2(screen.center().x, screen.bottom() - 50.0);
    match crate::font::shell(ctx) {
        Some(font) => {
            font.paint(&painter, egui::pos2(at.x - font.width(&text) * 0.5, at.y), &text, colour);
        }
        None => {
            painter.text(at, egui::Align2::CENTER_TOP, text, egui::FontId::proportional(14.0), colour);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_fades_in_then_out() {
        let t = Instant::now();
        let l = Load { began: t, fade_in: FADE_IN, fade_out: FADE_OUT, done: None };
        assert_eq!(alphas(&l, t), Some((1.0, 0.0)));
        assert_eq!(alphas(&l, t + Duration::from_secs(2)), Some((1.0, 0.5)));
        let l = Load { done: Some(t + Duration::from_secs(4)), ..l };
        assert_eq!(alphas(&l, t + Duration::from_secs(4)), Some((1.0, 1.0)));
        let (sheet, shown) = alphas(&l, t + Duration::from_millis(7500)).unwrap();
        assert!((sheet - 0.5).abs() < 1e-4 && (shown - 0.5).abs() < 1e-4);
        assert_eq!(alphas(&l, t + Duration::from_secs(11)), None);
        // (a screen taken over from the character list's window, up for three seconds there: a quarter left to fade in)
        assert_eq!(elapsed_since(10_000, 13_000), Duration::from_secs(3));
        assert_eq!(elapsed_since(13_000, 10_000), Duration::ZERO);
        let l = Load { began: t, fade_in: FADE_IN, fade_out: FADE_OUT, done: None };
        assert_eq!(alphas(&l, t + elapsed_since(10_000, 13_000)), Some((1.0, 0.75)));
        // (the blue: green-teal turned blue, the alpha kept)
        let mut px = [40, 200, 160, 255];
        tint(&mut px, TINT);
        assert_eq!(px, [14, 120, 232, 255]);
    }
}
