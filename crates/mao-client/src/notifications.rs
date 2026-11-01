//! The notification window (GUI.dll `NotificationWindowModule_c`, 0x1000c8a1, the module `notification_window`).
//!
//! The module's `SlotInitialized` (0x1000c7c1) connects `SlotAddText` (0x1000c796) to `GlobalSignals_c` +0x298, a string
//! signal; activated (`SlotNotificationWindowActivated` 0x1000c82a: the game's `CharPrefs.xml` has `notification_window`
//! true) it makes the window, FUN_1000bf4b: a `Window` "NotificationWindow" whose one tab, "Notifications", holds a vertical
//! layout with a spacer first (`VLayoutSpacer` 0 to 16000, stretch 1, so the lines sit at the bottom); its frame the whole
//! display's width and a third of its height from the top left (FUN_1000b9df, DAT_101a8a28 = 3) unless the window's saved
//! config (`NotificationWindowConfig`, FUN_1000ba68, which also reads `ClickableLinks`) says otherwise; its look from the
//! `NotificationWindowVisibility` value (FUN_1000b848): true, window style 0 with flags 0x800 (the tab shown); false (the
//! game's default), style 3 with flags 0xa7c (`WndBorder::SetBorderGfx`: no border graphics, the tab view drawing nothing),
//! so only the lines show over the game.
//!
//! `SlotAddText` → FUN_1000be0e adds a line: a `FadingTextView_c` (FUN_10008355) with the text, font 4 (Verdana 12), colour 0,
//! HTML flags 0x1004 and times 0, 5 and 3 (DAT_101a8ba0, DAT_101a96dc): its timer runs 8 s, the alpha (FUN_100081e8) 1 until
//! the last 3 s, then down to 0 (`left / 3`; the fade-in, 0 s, never shows). When the lines are taller than the window, the
//! first fading line found from the bottom child up is deleted. Links in a line open in the info view (FUN_1000bbaa) when
//! `ClickableLinks` is set; not ported.
//!
//! The one sender in Gamecode is the spell function 0xcfff (FUN_100a5f4b → FUN_100a9af8; FUN_100a57a0 lets it run on the
//! client): not ending, its string 0 is told when its stat 11, a range, is 0 and it runs on the client's own character, or
//! else when the character it runs on is within that many metres of the client's. GUI.dll itself never emits +0x298.

use std::time::Instant;

/// FadingTextView_c's three times (FUN_10008355: in, hold, out).
pub const FADE_IN: f32 = 0.0;
pub const HOLD: f32 = 5.0;
pub const FADE_OUT: f32 = 3.0;

/// The lines told, oldest first, and when.
pub static LINES: std::sync::Mutex<Vec<(String, Instant)>> = std::sync::Mutex::new(Vec::new());

/// `SlotAddText`: a line added at the bottom.
pub fn add(text: &str) {
    LINES.lock().unwrap_or_else(|e| e.into_inner()).push((text.to_string(), Instant::now()));
}

/// A line's alpha `age` seconds after it was added (FUN_100081e8 over the timer's time left), `None` once its timer ran out.
pub fn alpha(age: f32) -> Option<f32> {
    let total = FADE_IN + HOLD + FADE_OUT;
    let left = total - age;
    if left <= 0.0 {
        return None;
    }
    Some(if left <= FADE_OUT {
        left / FADE_OUT
    } else if FADE_OUT + HOLD < left {
        1.0 - (left - HOLD - FADE_OUT) / FADE_IN
    } else {
        1.0
    })
}

/// FUN_100a9af8: the line a 0xcfff spell tells, if it does: its text (string 0) when its range (stat 11) is 0 and it runs on
/// the client's own character, or when the character is within the range.
pub fn spell_line(spell: &mao_formats::spell::Spell, ending: bool, on_own: bool, distance: Option<f32>) -> Option<String> {
    if spell.kind != 0xcfff || ending {
        return None;
    }
    let range = spell.stat(11);
    let told = if range == 0 { on_own } else { distance.is_some_and(|d| d <= range as f32) };
    told.then(|| spell.string(0).to_string())
}

/// Markup as plain text (the view's HTML: its tags are not drawn).
fn plain(markup: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in markup.chars() {
        match c {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// The window: its lines from the bottom of its frame up (the top third of the screen), each at its alpha. `framed`
/// (`NotificationWindowVisibility`) draws the window's tab and background as well.
pub fn draw(ctx: &egui::Context, screen: egui::Rect, framed: bool, palette: &crate::theme::Palette) {
    let frame = egui::Rect::from_min_size(screen.min, egui::vec2(screen.width(), screen.height() / 3.0));
    let lines: Vec<(String, f32)> = {
        let mut lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
        lines.retain(|(_, at)| alpha(at.elapsed().as_secs_f32()).is_some());
        lines.iter().map(|(t, at)| (plain(t), alpha(at.elapsed().as_secs_f32()).unwrap_or(0.0))).collect()
    };
    if lines.is_empty() && !framed {
        return;
    }
    let font = crate::font::verdana(ctx, 12.0);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::unique("notification-window")));
    let mut top = frame.top();
    if framed {
        let tab = egui::Rect::from_min_size(frame.min, egui::vec2(110.0, 18.0));
        painter.rect_filled(frame, 0.0, egui::Color32::from_black_alpha(90));
        painter.rect_filled(tab, 2.0, egui::Color32::from_rgb(palette.panel[0], palette.panel[1], palette.panel[2]));
        painter.text(tab.center(), egui::Align2::CENTER_CENTER, "Notifications", crate::font::verdana(ctx, 13.0), egui::Color32::WHITE);
        top = tab.bottom();
    }
    // (wrapped to the window's width; from the bottom up, the lines that no longer fit are the oldest and go)
    let galleys: Vec<_> = lines.iter().map(|(t, a)| painter.layout(t.clone(), font.clone(), egui::Color32::WHITE.gamma_multiply(*a), frame.width() - 8.0)).collect();
    let mut y = frame.bottom();
    let mut shown = 0;
    for g in galleys.iter().rev() {
        if y - g.size().y < top {
            break;
        }
        y -= g.size().y;
        shown += 1;
    }
    if shown < galleys.len() {
        let mut stored = LINES.lock().unwrap_or_else(|e| e.into_inner());
        let drop = stored.len().saturating_sub(shown);
        stored.drain(..drop);
    }
    for (g, (_, a)) in galleys.iter().zip(&lines).skip(galleys.len() - shown) {
        painter.galley(egui::pos2(frame.left() + 5.0, y + 1.0), g.clone(), egui::Color32::from_black_alpha((a * 255.0) as u8));
        painter.galley(egui::pos2(frame.left() + 4.0, y), g.clone(), egui::Color32::WHITE);
        y += g.size().y;
    }
    if !lines.is_empty() {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FUN_100081e8 with FUN_1000be0e's times: full for 5 s, then fading over 3 s, then gone.
    #[test]
    fn a_line_holds_then_fades() {
        assert_eq!(alpha(0.0), Some(1.0));
        assert_eq!(alpha(4.9), Some(1.0));
        assert!((alpha(6.5).unwrap() - 0.5).abs() < 1e-5);
        assert_eq!(alpha(8.0), None);
    }

    /// FUN_100a9af8: a range of 0 tells only on the client's own character, a range tells within it.
    #[test]
    fn a_text_spell_tells_its_line() {
        let spell = mao_formats::spell::Spell { kind: 0xcfff, stats: vec![(11, 0)], strings: vec![(0, "The ground shakes.".into())], ..Default::default() };
        assert_eq!(spell_line(&spell, false, true, None).as_deref(), Some("The ground shakes."));
        assert_eq!(spell_line(&spell, false, false, Some(1.0)), None);
        assert_eq!(spell_line(&spell, true, true, None), None);
        let near = mao_formats::spell::Spell { stats: vec![(11, 20)], ..spell.clone() };
        assert!(spell_line(&near, false, false, Some(19.5)).is_some());
        assert!(spell_line(&near, false, false, Some(20.5)).is_none());
        assert!(spell_line(&near, false, false, None).is_none());
    }
}
