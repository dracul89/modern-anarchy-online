//! GUI.dll's fonts: the names the XML uses (FUN_10164c94: TOOLTIP 1, SHELL 2, CLOCK 3, CC17 4, NORMAL 5, BOLD 6, SMALL 7,
//! LARGE 8, HUGE 9, TT_MIN12 10, ITALIC 11, CHAT 12; anything else NORMAL) and `FontSystem_t`'s table of them (0x10272df0,
//! read by its constructor 0x1012eef1): 0 and 1 the bitmap `FontTooltip9.fnt`, 2 and 3 `FontGameShell12.fnt`, then Verdana
//! (`CreateTTFont` 0x1012ed48, `CreateFontA` with the cell height) 4 bold 20, 5 13, 6 bold 13, 7 12, 8 16, 9 bold 24, 10 14,
//! 11 italic 16.
//!
//! Departures: the TrueType fonts are egui's Verdana at the size whose ascent plus descent is the cell height (Verdana's
//! 2489/2048 of its em), regular whatever their weight; the original also scales them by the screen's height
//! (`GetScaleFromLowestResolution`), which egui's points do here; CHAT (12, empty in the table) is the chat bitmap font.

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
            12 => Some("chat"),
            _ => None,
        }
    }

    /// A TrueType entry's cell height.
    fn cell(self) -> f32 {
        match self.0 {
            4 => 20.0,
            7 => 12.0,
            8 | 11 => 16.0,
            9 => 24.0,
            10 => 14.0,
            _ => 13.0,
        }
    }
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
    /// The egui font of a TrueType entry.
    pub fn egui_font(&self, font: FontId) -> egui::FontId {
        let size = font.cell() / VERDANA_CELL;
        match &self.ctx {
            Some(ctx) => crate::font::verdana(ctx, size),
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
            None => font.cell(),
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
