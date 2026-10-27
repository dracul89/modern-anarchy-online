//! The original's bitmap fonts, for text that should look exactly as it does in the original (the chat lines).
//!
//! A font is a strip in the skin atlas (`GFX_FONT_CHAT9`, 256 x 55, white glyphs on the green key) and a table beside the
//! game's textures: `cd_image/textures/fonts/FontChat9.fnt` — two `u32` words (256, 256), then one `(x0, y0, x1, y1)` entry
//! of four `u32` per character code from 0 (318 entries, the first 32 unused), the glyph's pixel rectangle in the strip. The
//! strip is laid out in rows of the glyph height plus two, and a short glyph's `y0` is offset inside its row.
//!
//! egui draws only vector fonts, so a line is painted as a mesh of textured quads, one per glyph, with the glyphs' own
//! pixel sizes (egui's zoom scales them with the rest of the interface; the texture is sampled nearest-neighbour so they
//! stay crisp).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Whether the chat uses the original's font (the `original_font` setting).
static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// The entries of a `.fnt` table: `(x0, y0, x1, y1)` per character code.
pub fn parse_fnt(bytes: &[u8]) -> Option<Vec<[u16; 4]>> {
    let words: Vec<u32> = bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
    let count = words.len().checked_sub(2)? / 4;
    let glyphs: Vec<[u16; 4]> = (0..count).map(|g| [words[2 + g * 4], words[3 + g * 4], words[4 + g * 4], words[5 + g * 4]]).take_while(|e| e.iter().all(|&v| v < 4096)).map(|e| e.map(|v| v as u16)).collect();
    (glyphs.len() > 64).then_some(glyphs)
}

pub struct BitmapFont {
    texture: egui::TextureHandle,
    strip: [f32; 2],
    glyphs: Vec<[u16; 4]>,
    /// The glyph height (9 or 12).
    pub height: f32,
    /// The height of a line of text as laid out.
    pub line_height: f32,
    space: f32,
}

impl BitmapFont {
    /// Loads a font: `"chat"`, `"shell"` (the 12 px one) or `"tooltip"`.
    pub fn load(ctx: &egui::Context, skin: &crate::skin::Skin, which: &str) -> Option<BitmapFont> {
        let (file, sprite) = match which {
            "shell" => ("FontGameShell12.fnt", "GFX_FONT_GAMESHELL12"),
            "tooltip" => ("FontTooltip9.fnt", "GFX_FONT_TOOLTIP9"),
            _ => ("FontChat9.fnt", "GFX_FONT_CHAT9"),
        };
        let glyphs = parse_fnt(&std::fs::read(crate::theme::game_dir()?.join("cd_image/textures/fonts").join(file)).ok()?)?;
        let (mut rgba, size) = crate::skin::decode(skin.bytes(sprite)?)?;
        // white glyphs: their brightness becomes the alpha, so the tint colours them
        for px in rgba.chunks_mut(4) {
            if px[3] != 0 {
                let a = px[0].max(px[1]).max(px[2]);
                px.copy_from_slice(&[255, 255, 255, a]);
            }
        }
        let texture = ctx.load_texture(sprite, egui::ColorImage::from_rgba_unmultiplied(size, &rgba), egui::TextureOptions::NEAREST);
        let height = glyphs.iter().skip(32).map(|g| g[3].saturating_sub(g[1])).max().unwrap_or(9) as f32;
        let space = glyphs.get(32).map(|g| (g[2].saturating_sub(g[0])) as f32).filter(|w| *w >= 2.0).unwrap_or(3.0);
        Some(BitmapFont { texture, strip: [size[0] as f32, size[1] as f32], glyphs, height, line_height: height + 4.0, space })
    }

    /// From a table alone (no picture), for measuring in tests.
    #[cfg(test)]
    fn for_test(ctx: &egui::Context, glyphs: Vec<[u16; 4]>, height: f32) -> BitmapFont {
        let texture = ctx.load_texture("test-font", egui::ColorImage::filled([4, 4], egui::Color32::WHITE), egui::TextureOptions::NEAREST);
        let space = glyphs.get(32).map(|g| (g[2] - g[0]) as f32).filter(|w| *w >= 2.0).unwrap_or(3.0);
        BitmapFont { texture, strip: [256.0, 256.0], glyphs, height, line_height: height + 4.0, space }
    }

    fn glyph(&self, c: char) -> Option<[u16; 4]> {
        if c == ' ' {
            return None;
        }
        self.glyphs.get(c as usize).copied().filter(|g| g[2] > g[0] && g[3] > g[1])
    }

    /// How far the pen moves for a character (a glyph not in the font takes a space's width).
    pub fn advance(&self, c: char) -> f32 {
        match self.glyph(c) {
            Some(g) => (g[2] - g[0]) as f32 + 1.0,
            None => self.space + 1.0,
        }
    }

    pub fn width(&self, text: &str) -> f32 {
        text.chars().map(|c| self.advance(c)).sum()
    }

    /// Paints one line with its top left at `pos`; returns its width.
    pub fn paint(&self, painter: &egui::Painter, pos: egui::Pos2, text: &str, color: egui::Color32) -> f32 {
        let mut mesh = egui::Mesh::with_texture(self.texture.id());
        let pitch = self.height as u16 + 2;
        let mut x = pos.x;
        for c in text.chars() {
            if let Some(g) = self.glyph(c) {
                let (w, h) = ((g[2] - g[0]) as f32, (g[3] - g[1]) as f32);
                // a short glyph sits where it sits in its row of the strip
                let offset = (g[1] % pitch) as f32;
                let rect = egui::Rect::from_min_size(egui::pos2(x, pos.y + 2.0 + offset), egui::vec2(w, h));
                let uv = egui::Rect::from_min_max(egui::pos2(g[0] as f32 / self.strip[0], g[1] as f32 / self.strip[1]), egui::pos2(g[2] as f32 / self.strip[0], g[3] as f32 / self.strip[1]));
                mesh.add_rect_with_uv(rect, uv, color);
            }
            x += self.advance(c);
        }
        painter.add(egui::Shape::mesh(mesh));
        x - pos.x
    }

    /// The text broken into lines no wider than `width`: at spaces, or inside a word longer than a line.
    pub fn wrap(&self, text: &str, width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        let mut line_width = 0.0;
        for word in text.split(' ') {
            let word_width = self.width(word);
            let space = if line.is_empty() { 0.0 } else { self.advance(' ') };
            if !line.is_empty() && line_width + space + word_width > width {
                lines.push(std::mem::take(&mut line));
                line_width = 0.0;
            }
            if word_width > width {
                // letter by letter
                for c in word.chars() {
                    let w = self.advance(c);
                    if line_width + w > width && !line.is_empty() {
                        lines.push(std::mem::take(&mut line));
                        line_width = 0.0;
                    }
                    line.push(c);
                    line_width += w;
                }
            } else {
                if !line.is_empty() {
                    line.push(' ');
                    line_width += space;
                }
                line.push_str(word);
                line_width += word_width;
            }
        }
        if !line.is_empty() || lines.is_empty() {
            lines.push(line);
        }
        lines
    }

    /// Word-wrapped text across the available width, as a widget.
    pub fn label(&self, ui: &mut egui::Ui, text: &str, color: egui::Color32) -> egui::Response {
        let width = ui.available_width().max(20.0);
        let lines = self.wrap(text, width);
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, lines.len() as f32 * self.line_height), egui::Sense::hover());
        for (i, line) in lines.iter().enumerate() {
            self.paint(ui.painter(), egui::pos2(rect.left(), rect.top() + i as f32 * self.line_height), line, color);
        }
        response
    }
}

/// The chat font of this context, loaded once per skin configuration (`skin::configure` drops it); None without the game's files.
pub fn chat(ctx: &egui::Context) -> Option<Arc<BitmapFont>> {
    let id = egui::Id::new("font-chat");
    if let Some(found) = ctx.data(|d| d.get_temp::<Option<Arc<BitmapFont>>>(id)) {
        return found;
    }
    let loaded = crate::skin::skin(ctx).and_then(|skin| BitmapFont::load(ctx, &skin, "chat")).map(Arc::new);
    ctx.data_mut(|d| d.insert_temp(id, loaded.clone()));
    loaded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Vec<[u16; 4]> {
        // 128 blanks, then: space 2 px wide, 'a'.. each 5 px wide in row 0 of a 9 px font
        let mut t = vec![[0u16; 4]; 128];
        t[32] = [0, 0, 2, 9];
        for (i, c) in ('a'..='z').enumerate() {
            t[c as usize] = [10 + i as u16 * 6, 0, 15 + i as u16 * 6, 9];
        }
        t[b'-' as usize] = [200, 4, 203, 5];
        t
    }

    #[test]
    fn the_table_is_read_from_the_words() {
        let mut bytes = Vec::new();
        for w in [256u32, 256].into_iter().chain(table().iter().flat_map(|g| g.iter().map(|&v| v as u32))).chain([u32::MAX; 3]) {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        let parsed = parse_fnt(&bytes).unwrap();
        assert_eq!(parsed.len(), 128);
        assert_eq!(parsed[b'a' as usize], [10, 0, 15, 9]);
        assert!(parse_fnt(&bytes[..100]).is_none());
    }

    #[test]
    fn widths_and_wrapping_follow_the_glyphs() {
        let ctx = egui::Context::default();
        let font = BitmapFont::for_test(&ctx, table(), 9.0);
        // five pixels and one of spacing per letter, three for a space
        assert_eq!(font.width("ab"), 12.0);
        assert_eq!(font.width("a b"), 15.0);
        assert_eq!(font.wrap("aa bb cc", 30.0), vec!["aa bb", "cc"]);
        assert_eq!(font.wrap("aaaaaaaa", 20.0), vec!["aaa", "aaa", "aa"]);
        assert_eq!(font.wrap("", 50.0), vec![""]);
    }
}
