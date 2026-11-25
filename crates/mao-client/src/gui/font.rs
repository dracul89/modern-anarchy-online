//! GUI.dll's fonts: the names the XML uses (FUN_10164c94: TOOLTIP 1, SHELL 2, CLOCK 3, CC17 4, NORMAL 5, BOLD 6, SMALL 7,
//! LARGE 8, HUGE 9, TT_MIN12 10, ITALIC 11, CHAT 12; anything else NORMAL) and `FontSystem_t`'s table of them (0x10272df0,
//! read by its constructor 0x1012eef1): 0 and 1 the bitmap `FontTooltip9.fnt`, 2 and 3 `FontGameShell12.fnt`, then Verdana
//! (`CreateTTFont` 0x1012ed48, `CreateFontA` with the cell height) 4 bold 20, 5 13, 6 bold 13, 7 12, 8 16, 9 bold 24, 10 14,
//! 11 italic 16. CHAT (12) is no entry of the table: the constructor makes it last (0x1012f19a on), `CreateTTFont(12, ...)`
//! with the preferences' `ChatFontName`, `ChatFontStyle` and a tenth of `ChatFontSize` for its cell height (`MainPrefs.xml`:
//! Verdana, Regular, 140, between 50 and 480). Until 2026-11-24 the chat's lines were drawn in the bitmap `FontChat9.fnt`,
//! which the original's chat does not use: the user's picture of the original's chat beside this client's.
//!
//! The table's byte at +0x10 asks for bold, the one at +0x11 for italic (the constructor's "Bold" / "Regular" plus
//! "italic" style, which `CreateTTFont` turns into weight 700 and the italic flag): entries 4, 6 and 9 are bold, 11 italic.
//!
//! Two cell heights follow the screen's height (the constructor, before the fonts are made): CC17's (4) is its 20 times
//! the display height over 768 (0x1012f09c), TT_MIN12's (10) its 14 times one plus 0.4 of what
//! `GetScaleFromLowestResolution`'s height (over 480) is above one (0x1012ef0f), each cut to a whole number (`_ftol`). Here
//! the height is the egui screen's, in the points the views are laid out in.
//!
//! Departures: the TrueType fonts are egui's Verdana (the system's regular, bold or italic face) at the size whose ascent
//! plus descent is the cell height (Verdana's 2489/2048 of its em); CHAT is Verdana whatever `ChatFontName` and
//! `ChatFontStyle` say.

use super::geom::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontId(pub u8);

impl FontId {
    pub const NORMAL: FontId = FontId(5);

    /// FUN_10164c94.
    pub fn named(name: &str) -> FontId {
        const NAMES: [&str; 12] = ["TOOLTIP", "SHELL", "CLOCK", "CC17", "NORMAL", "BOLD", "SMALL", "LARGE", "HUGE", "TT_MIN12", "ITALIC", "CHAT"];
        NAMES.iter().position(|n| *n == name).map_or(FontId::NORMAL, |i| FontId(i as u8 + 1))
    }

    /// The bitmap font of a bitmap entry (`font.rs`'s names).
    fn bitmap(self) -> Option<&'static str> {
        match self.0 {
            0 | 1 => Some("tooltip"),
            2 | 3 => Some("shell"),
            _ => None,
        }
    }

    /// A TrueType entry's weight and slant: (bold, italic), the table's bytes at +0x10 and +0x11.
    pub fn style(self) -> (bool, bool) {
        (matches!(self.0, 4 | 6 | 9), self.0 == 11)
    }

    /// A TrueType entry's cell height on a screen `height` high (none or not laid out yet: the table's own).
    fn cell_on(self, height: Option<f32>) -> f32 {
        let table = self.cell();
        let Some(h) = height.filter(|h| *h > 0.0) else { return table };
        // (x87 at 53 bits: the scale kept as a float, the product cut toward zero)
        let scale = match self.0 {
            4 => (h as f64 / 768.0) as f32,
            10 => ((((h as f64 / 480.0) as f32 - 1.0) as f64) * 0.4 + 1.0) as f32,
            _ => return table,
        };
        (table as f64 * scale as f64).trunc() as f32
    }

    /// A TrueType entry's cell height in the table; CHAT's from the preferences ([`chat_cell`]).
    fn cell(self) -> f32 {
        match self.0 {
            12 => chat_cell(crate::prefs::get("ChatFontSize")),
            4 => 20.0,
            7 => 12.0,
            8 | 11 => 16.0,
            9 => 24.0,
            10 => 14.0,
            _ => 13.0,
        }
    }
}

/// The chat font's cell height: a tenth of `ChatFontSize`, cut to a whole number (`FontSystem_t`'s constructor,
/// 0x1012f1c0: `AsInt32() / 10`); 140 where the preference is not there, and never under the preference's least, 50.
pub fn chat_cell(size: Option<f64>) -> f32 {
    ((size.unwrap_or(140.0) as i32).clamp(50, 480) / 10) as f32
}

/// Text measuring for the views' preferred sizes: with an egui context the real fonts, without (tests) a fixed
/// approximation.
#[derive(Clone, Default)]
pub struct Measure {
    pub ctx: Option<egui::Context>,
}

/// Verdana's ascent plus descent over its em (2059 + 430 over 2048).
const VERDANA_CELL: f32 = 2489.0 / 2048.0;

impl Measure {
    /// The screen's height the fonts are made for.
    fn screen_height(&self) -> Option<f32> {
        self.ctx.as_ref().map(|ctx| ctx.content_rect().height())
    }

    /// The egui font of a TrueType entry.
    pub fn egui_font(&self, font: FontId) -> egui::FontId {
        let size = font.cell_on(self.screen_height()) / VERDANA_CELL;
        match &self.ctx {
            Some(ctx) => {
                let (bold, italic) = font.style();
                crate::font::verdana_styled(ctx, size, bold, italic)
            }
            None => egui::FontId::proportional(size),
        }
    }

    pub fn bitmap(&self, font: FontId) -> Option<std::sync::Arc<crate::font::BitmapFont>> {
        let ctx = self.ctx.as_ref()?;
        match font.bitmap()? {
            "chat" => crate::font::chat(ctx),
            "shell" => crate::font::shell(ctx),
            _ => crate::font::tooltip(ctx),
        }
    }

    /// `GetFontHeight`: a line's height in pixels.
    pub fn height(&self, font: FontId) -> f32 {
        if let Some(b) = self.bitmap(font) {
            return b.line_height;
        }
        match font.bitmap() {
            Some("chat") | Some("tooltip") => 9.0,
            Some(_) => 12.0,
            None => font.cell_on(self.screen_height()),
        }
    }

    /// A string's width in pixels.
    pub fn width(&self, font: FontId, text: &str) -> f32 {
        if let Some(b) = self.bitmap(font) {
            return b.width(text);
        }
        match &self.ctx {
            Some(ctx) if font.bitmap().is_none() => {
                let f = self.egui_font(font);
                ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text.to_string(), f, egui::Color32::WHITE).size().x)
            }
            _ => text.chars().count() as f32 * self.height(font) * 0.55,
        }
    }

    /// A single line's size in the views' units (pixels less one).
    pub fn line_size(&self, font: FontId, text: &str) -> Point {
        Point::new(self.width(font, text) - 1.0, self.height(font) - 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_table_weights() {
        // (FontSystem_t's table at 0x10272df0: BOLD, CC17 and HUGE are bold, ITALIC italic, the rest regular)
        assert_eq!(FontId::named("BOLD").style(), (true, false));
        assert_eq!(FontId::named("CC17").style(), (true, false));
        assert_eq!(FontId::named("HUGE").style(), (true, false));
        assert_eq!(FontId::named("ITALIC").style(), (false, true));
        for n in ["NORMAL", "SMALL", "LARGE", "TT_MIN12", "CHAT"] {
            assert_eq!(FontId::named(n).style(), (false, false), "{n}");
        }
    }

    /// CHAT is a TrueType font of the preferences' size, a tenth of `ChatFontSize` cut to a whole number: 14 by the
    /// game's own `MainPrefs.xml`, within the preference's 50 to 480.
    #[test]
    fn the_chat_font_is_the_preferences_size() {
        assert_eq!([chat_cell(None), chat_cell(Some(140.0)), chat_cell(Some(165.0)), chat_cell(Some(10.0)), chat_cell(Some(900.0))], [14.0, 14.0, 16.0, 5.0, 48.0]);
        assert!(FontId::named("CHAT").bitmap().is_none());
    }

    /// WIN-027: CC17 by the height over 768, TT_MIN12 by 1 + 0.4 of the height over 480 less one, cut to whole
    /// numbers; the others as the table has them; no screen, the table's.
    #[test]
    fn two_fonts_follow_the_screens_height() {
        let (cc17, tt, normal) = (FontId::named("CC17"), FontId::named("TT_MIN12"), FontId::NORMAL);
        assert_eq!([cc17.cell_on(Some(768.0)), cc17.cell_on(Some(1080.0)), cc17.cell_on(Some(600.0))], [20.0, 28.0, 15.0]);
        assert_eq!([tt.cell_on(Some(480.0)), tt.cell_on(Some(1080.0)), tt.cell_on(Some(768.0))], [14.0, 21.0, 17.0]);
        assert_eq!([normal.cell_on(Some(1080.0)), cc17.cell_on(None), tt.cell_on(Some(0.0))], [13.0, 20.0, 14.0]);
    }
}
