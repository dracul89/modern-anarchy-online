//! GUI.dll's text: `HTMLParser_c` (the markup), its words (`_UpdateWordSizes` 0x1015dab9, `_UpdateWordSize` 0x1015c7a9) and
//! `TextRenderer_c`'s lines (`_ReWrap` 0x10161e82, `_AddLineDesc` 0x10161e22), size (`CalculatePreferredSize` 0x101626c8,
//! `GetContentSize` 0x10162978) and drawing (`_RenderLine` 0x101613f0, `_RenderString` 0x101602bb).
//!
//! The markup (`_ParseTag` 0x1015cced, `_ParseTextAttributes` 0x1015d874): a tag runs from `<` to the first `>` outside
//! quotes; its name is lower-cased, its attributes `name=value` with the value quoted (`"`/`'`, `\"` escapes) or up to a
//! space (FUN_1015cb03). Known: `br` and `p` (a new line), `center` and `div align=center|right indent=wrapped` (a block:
//! a new line unless at one's start, alignment; wrapped lines after a `indent=wrapped` div's first are indented 10),
//! `/center` `/div`, `font color=#rrggbb|<name> face=small|bold|italic|large|huge` (its colour, or the default colour when
//! it has none: not the enclosing one), `i` (italic, the colour kept), `/font` `/i`, `a href= style=text-decoration:none
//! title=` (a link: the link colour and underlined unless so styled), `/a`, `u` `/u`, `img src=rdb://<n>|tdb://<name>`,
//! `title`...`/title` (hidden), `section` `/section` `tvoptions wordwrap=yes|no` and any other tag (dropped). A tag that
//! does not parse (`<>`, `<a>` without `href`, `<font>` with neither attribute, `<img>` without a picture) stays as text.
//! There are no character entities. Each closing tag returns to what was in force before its nearest unclosed opener
//! (`_UpdateInheritedData` 0x1015bf54).

use super::font::{FontId, Measure};
use super::geom::Point;

/// The default colour (`0xff000000`: the text's own colour, `TextRenderer_c` +0x1bc, else the view's).
pub const DEFAULT_COLOUR: u32 = 0xff00_0000;
/// `TextRenderer_c`'s link colour (+0x1c0, set by its constructor 0x101637f0).
pub const LINK_COLOUR: u32 = 0xff22_99ff;

/// `TextRenderer_c` +0x194 / `HTMLParser_c` +0x4c (`TextView_c`'s XML names, 0x1016538c).
pub mod flags {
    pub const ACCEPT_TXT_INPUT: u32 = 0x1;
    pub const ACCEPT_MOUSE_INPUT: u32 = 0x4;
    pub const ALLOW_TEXT_SELECTION: u32 = 0x8;
    pub const PASSWORD: u32 = 0x10;
    pub const MULTILINE: u32 = 0x20;
    pub const WORD_WRAP: u32 = 0x40;
    pub const WORD_SPLIT: u32 = 0x80;
    pub const IGNORE_NEWLINES: u32 = 0x100;
    pub const DISABLE_RC_MENU: u32 = 0x200;
    pub const FILL_BOTTOM_UP: u32 = 0x400;
    pub const ENABLE_SHADOW: u32 = 0x800;
    pub const RENDER_SHADOW: u32 = 0x1000;
    pub const NUMERIC: u32 = 0x2000;
    pub const ALT_ENTER_MODE: u32 = 0x4000;
    pub const NAMES: [(&str, u32); 14] = [
        ("TVF_ACCEPT_TXT_INPUT", ACCEPT_TXT_INPUT),
        ("TVF_ACCEPT_MOUSE_INPUT", ACCEPT_MOUSE_INPUT),
        ("TVF_ALLOW_TEXT_SELECTION", ALLOW_TEXT_SELECTION),
        ("TVF_PASSWORD", PASSWORD),
        ("TVF_MULTILINE", MULTILINE),
        ("TVF_WORD_WRAP", WORD_WRAP),
        ("TVF_WORD_SPLIT", WORD_SPLIT),
        ("TVF_IGNORE_NEWLINES", IGNORE_NEWLINES),
        ("TVF_DISABLE_RC_MENU", DISABLE_RC_MENU),
        ("TVF_FILL_BOTTOM_UP", FILL_BOTTOM_UP),
        ("TVF_ENABLE_SHADOW", ENABLE_SHADOW),
        ("TVF_RENDER_SHADOW", RENDER_SHADOW),
        ("TVF_NUMERIC", NUMERIC),
        ("TVF_ALT_ENTER_MODE", ALT_ENTER_MODE),
    ];

    /// `feature_flags`: names split at `|`, `,` and spaces, in any case.
    pub fn parse(text: &str) -> u32 {
        text.split(['|', ',', ' ']).map(str::trim).filter(|t| !t.is_empty()).fold(0, |acc, t| {
            acc | NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(t)).map_or(0, |(_, v)| *v)
        })
    }
}

/// How a stretch of text is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    /// 0x00rrggbb, or [`DEFAULT_COLOUR`].
    pub colour: u32,
    /// The font, or `None` for the view's.
    pub font: Option<FontId>,
    pub underline: bool,
    /// The link's target (and whether it is drawn as one: in the link colour, underlined).
    pub link: Option<String>,
    pub link_look: bool,
    /// The block's alignment: 0 left, 1 right, 4 centred.
    pub align: u8,
}

impl Default for Style {
    fn default() -> Style {
        Style { colour: DEFAULT_COLOUR, font: None, underline: false, link: None, link_look: false, align: 0 }
    }
}

/// What the markup becomes, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Text(String, Style),
    /// `br`, `p`: the next word starts a line.
    Break,
    /// A block's start or end (`div`, `center`): a new line unless at a line's start; `wrapped` when it starts an
    /// `indent=wrapped` block, `end` for its end.
    Block { wrapped: bool, end: bool },
    Image(String, Style),
}

/// `ParseHTMLColor` (0x1015c53e): `#rrggbb`, else a named colour (the skin's `TextColors.xml`), else black.
pub fn html_colour(s: &str, named: &dyn Fn(&str) -> Option<u32>) -> u32 {
    if let Some(h) = s.strip_prefix('#') {
        let digits: String = h.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        if let Ok(v) = u32::from_str_radix(&digits, 16) {
            return v & 0xff_ffff;
        }
    }
    named(s).unwrap_or(0)
}

/// FUN_1015cb03: a tag's lower-cased name and its attributes (names lower-cased).
fn tokenize(tag: &str) -> (String, Vec<(String, String)>) {
    let b = tag.as_bytes();
    let mut i = 0;
    while i < b.len() && !b[i].is_ascii_whitespace() {
        i += 1;
    }
    let name = tag[..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    while i < b.len() {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        let start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' {
            i += 1;
        }
        let key = tag[start..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        while i < b.len() && b[i] != b'=' {
            i += 1;
        }
        i += 1;
        if i >= b.len() {
            attrs.push((key, String::new()));
            break;
        }
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            attrs.push((key, String::new()));
            break;
        }
        let mut end_char = b' ';
        if b[i] == b'"' || b[i] == b'\'' {
            end_char = b[i];
            i += 1;
        }
        let mut value = Vec::new();
        while i < b.len() && b[i] != end_char {
            if b[i] == b'\\' && matches!(b.get(i + 1), Some(b'"') | Some(b'\'')) {
                i += 1;
            }
            value.push(b[i]);
            i += 1;
        }
        attrs.push((key, String::from_utf8_lossy(&value).into_owned()));
    }
    (name, attrs)
}

/// The markup's items.
pub fn parse(text: &str, named: &dyn Fn(&str) -> Option<u32>) -> Vec<Item> {
    #[derive(Clone)]
    struct Font(u32, Option<FontId>);
    let mut out = Vec::new();
    let mut style = Style::default();
    // the openers in force: (colour, font) of font/i, underline of u, the style before an a, the alignment of a block
    let mut fonts: Vec<Font> = Vec::new();
    let mut unders: Vec<bool> = Vec::new();
    let mut links: Vec<(u32, bool, bool)> = Vec::new();
    let mut blocks: Vec<u8> = Vec::new();
    let mut title = 0;
    let mut text_buf = String::new();
    let flush = |buf: &mut String, out: &mut Vec<Item>, style: &Style, title: i32| {
        if !buf.is_empty() && title == 0 {
            out.push(Item::Text(std::mem::take(buf), style.clone()));
        }
        buf.clear();
    };
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // the tag's end: the first '>' outside quotes
            let mut j = i + 1;
            let mut quote = None;
            while j < bytes.len() {
                let c = bytes[j];
                match quote {
                    Some(q) if c == q => quote = None,
                    Some(_) => {}
                    None if c == b'"' || c == b'\'' => quote = Some(c),
                    None if c == b'>' => break,
                    None => {}
                }
                j += 1;
            }
            if j < bytes.len() {
                let (name, attrs) = tokenize(&text[i + 1..j]);
                let get = |k: &str| attrs.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
                let mut known = true;
                match name.as_str() {
                    "" => known = false,
                    "br" | "p" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        out.push(Item::Break);
                    }
                    "center" | "div" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        blocks.push(style.align);
                        let mut wrapped = false;
                        if name == "center" {
                            style.align = 4;
                        } else {
                            match get("align").map(|a| a.to_ascii_lowercase()).as_deref() {
                                Some("center") => style.align = 4,
                                Some("right") => style.align = 1,
                                _ => {}
                            }
                            wrapped = get("indent").is_some_and(|v| v.eq_ignore_ascii_case("wrapped"));
                        }
                        out.push(Item::Block { wrapped, end: false });
                    }
                    "/div" | "/center" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        style.align = blocks.pop().unwrap_or(0);
                        out.push(Item::Block { wrapped: false, end: true });
                    }
                    "i" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        fonts.push(Font(style.colour, style.font));
                        style.font = Some(FontId(11));
                    }
                    "font" => {
                        let (colour, face) = (get("color"), get("face"));
                        if colour.is_none() && face.is_none() {
                            known = false;
                        } else {
                            flush(&mut text_buf, &mut out, &style, title);
                            fonts.push(Font(style.colour, style.font));
                            style.colour = colour.map_or(DEFAULT_COLOUR, |c| html_colour(&c, named));
                            style.font = match face.as_deref() {
                                Some("small") => Some(FontId(10)),
                                Some("bold") => Some(FontId(6)),
                                Some("italic") => Some(FontId(11)),
                                Some("large") => Some(FontId(8)),
                                Some("huge") => Some(FontId(9)),
                                _ => None,
                            };
                        }
                    }
                    "/font" | "/i" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        let f = fonts.pop().unwrap_or(Font(DEFAULT_COLOUR, None));
                        style.colour = f.0;
                        style.font = f.1;
                    }
                    "a" => match get("href") {
                        None => known = false,
                        Some(href) => {
                            flush(&mut text_buf, &mut out, &style, title);
                            links.push((style.colour, style.underline, style.link_look));
                            let plain = get("style").is_some_and(|s| s.eq_ignore_ascii_case("text-decoration:none"));
                            style.link = Some(href);
                            style.link_look = !plain;
                        }
                    },
                    "/a" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        let (colour, _, _) = links.pop().unwrap_or((DEFAULT_COLOUR, false, false));
                        style.colour = colour;
                        style.link = None;
                        style.link_look = false;
                    }
                    "u" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        unders.push(style.underline);
                        style.underline = true;
                    }
                    "/u" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        style.underline = unders.pop().unwrap_or(false);
                    }
                    "img" => match get("src").filter(|s| s.len() > 6) {
                        None => known = false,
                        Some(src) => {
                            flush(&mut text_buf, &mut out, &style, title);
                            if title == 0 {
                                out.push(Item::Image(src, style.clone()));
                            }
                        }
                    },
                    "title" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        title += 1;
                    }
                    "/title" => {
                        flush(&mut text_buf, &mut out, &style, title);
                        title = (title - 1).max(0);
                    }
                    _ => {}
                }
                if known {
                    i = j + 1;
                    continue;
                }
            }
        }
        // (the character entities Utils.dll's text reader knows: a space that is not a break, a quote, the angle
        // brackets and the ampersand)
        if bytes[i] == b'&'
            && let Some((name, c)) = ENTITIES.iter().find(|(name, _)| text[i + 1..].starts_with(name))
        {
            text_buf.push(*c);
            i += 1 + name.len();
            continue;
        }
        let c = text[i..].chars().next().unwrap_or('?');
        text_buf.push(c);
        i += c.len_utf8();
    }
    flush(&mut text_buf, &mut out, &style, title);
    out
}

/// The character entities of the markup (Utils.dll's table: `nbsp`, `quot`, `gt`, `lt`, `amp`), each with its `;`.
const ENTITIES: [(&str, char); 5] = [("nbsp;", ' '), ("quot;", '"'), ("gt;", '>'), ("lt;", '<'), ("amp;", '&')];

/// A piece of a word in one style.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub text: String,
    pub style: Style,
    pub image: Option<String>,
    pub width: f32,
}

/// `HTMLParser_c::WordAttrib_c`: the white space before it (+0xc), its text (+0x10), its height (+0x14), whether it
/// starts a line (+0x18) or a block (+0x19).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Word {
    pub space: f32,
    pub width: f32,
    pub height: f32,
    pub newline: bool,
    pub block: bool,
    /// When it starts a block: whether that is an `indent=wrapped` one, and whether it ends one.
    pub wrapped: bool,
    pub block_end: bool,
    /// The style of its white space (drawn as the space glyph: underlined inside a link or `u`).
    pub space_style: Option<Style>,
    pub pieces: Vec<Piece>,
}

/// The words of the items, measured (`_UpdateWordSizes`): white space ends a word and its width goes to the next one's
/// `space`; a newline does too and starts a line (its width the space's under `TVF_IGNORE_NEWLINES`, else none); `br`/`p`
/// start a line, blocks a block.
pub fn words(items: &[Item], m: &Measure, default_font: FontId, feature: u32, image_size: &dyn Fn(&str) -> Option<(f32, f32)>) -> Vec<Word> {
    let mut out: Vec<Word> = Vec::new();
    let mut cur = Word::default();
    let mut in_text = false;
    let start = |out: &mut Vec<Word>, cur: &mut Word| {
        if !cur.pieces.is_empty() || cur.space > 0.0 || cur.newline || cur.block {
            out.push(std::mem::take(cur));
        }
    };
    for item in items {
        match item {
            Item::Break => {
                start(&mut out, &mut cur);
                cur.newline = true;
                in_text = false;
            }
            Item::Block { wrapped, end } => {
                start(&mut out, &mut cur);
                cur.block = true;
                cur.wrapped = *wrapped;
                cur.block_end = *end;
                in_text = false;
            }
            Item::Image(src, style) => {
                if let Some((w, h)) = image_size(src) {
                    cur.width += w;
                    cur.height = cur.height.max(h);
                    cur.pieces.push(Piece { text: String::new(), style: style.clone(), image: Some(src.clone()), width: w });
                    in_text = true;
                }
            }
            Item::Text(text, style) => {
                let font = style.font.unwrap_or(default_font);
                let height = m.height(font);
                let mut piece = String::new();
                let push_piece = |cur: &mut Word, piece: &mut String| {
                    if !piece.is_empty() {
                        let shown: String = if feature & flags::PASSWORD != 0 { "*".repeat(piece.chars().count()) } else { piece.clone() };
                        let width = m.width(font, &shown);
                        cur.width += width;
                        cur.pieces.push(Piece { text: shown, style: style.clone(), image: None, width });
                        piece.clear();
                    }
                };
                for c in text.chars() {
                    let newline = c == '\n';
                    if newline || (c.is_ascii() && c.is_ascii_whitespace()) {
                        push_piece(&mut cur, &mut piece);
                        // (a space starts the next word: a word that has text ends here)
                        if in_text || newline {
                            start(&mut out, &mut cur);
                            in_text = false;
                        }
                        if newline {
                            cur.newline = true;
                            if feature & flags::IGNORE_NEWLINES != 0 {
                                cur.space += m.width(font, " ");
                            }
                        } else if c != '\r' {
                            cur.space += m.width(font, " ");
                        }
                        cur.space_style = Some(style.clone());
                    } else {
                        piece.push(c);
                        in_text = true;
                        cur.height = cur.height.max(height);
                    }
                }
                push_piece(&mut cur, &mut piece);
            }
        }
    }
    start(&mut out, &mut cur);
    out
}

/// `TextRenderer_c::LineDesc_c`: its first word, its indent, width, height and top.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    pub first: usize,
    pub end: usize,
    /// It began where a word did not fit on the line before: its first word's white space stays on that line (the line
    /// starts at the word's text) and is neither counted nor drawn here.
    pub wrapped: bool,
    pub indent: f32,
    pub width: f32,
    pub height: f32,
    pub y: f32,
}

/// `_ReWrap` to `avail` pixels: the lines and the widest. Single-line text is one line; multi-line text without
/// `TVF_WORD_WRAP` breaks only at new lines and blocks; with it a word that does not fit starts a line (one wider than
/// the line is split where it overflows).
pub fn wrap(words: &[Word], feature: u32, avail: f32, default_height: f32) -> (Vec<Line>, f32) {
    let mut lines = Vec::new();
    let mut widest: f32 = 0.0;
    let mut line = Line::default();
    // the wrapped-indent counter (-1 outside an indent=wrapped block)
    let mut counter: i32 = -1;
    let add = |lines: &mut Vec<Line>, line: &mut Line, end: usize, counter: &mut i32, widest: &mut f32| {
        if line.height == 0.0 {
            line.height = default_height;
        }
        line.width += line.indent;
        *widest = widest.max(line.width);
        line.end = end;
        if *counter != -1 {
            *counter += 1;
        }
        let y = line.y + line.height;
        lines.push(line.clone());
        *line = Line { first: end, end, wrapped: false, indent: if *counter > 0 { 10.0 } else { 0.0 }, width: 0.0, height: 0.0, y };
    };
    if feature & flags::MULTILINE == 0 {
        let width: f32 = words.iter().map(|w| w.space + w.width).sum();
        line.width = width;
        line.height = words.iter().map(|w| w.height).fold(0.0, f32::max);
        add(&mut lines, &mut line, words.len(), &mut counter, &mut widest);
        return (lines, widest);
    }
    let avail = avail - if feature & flags::ACCEPT_TXT_INPUT != 0 { 4.0 } else { 0.0 };
    for (k, w) in words.iter().enumerate() {
        if w.block {
            counter = if w.block_end { -1 } else if w.wrapped { 0 } else { counter };
        }
        let whole = w.space + w.width;
        let starts = w.newline || (w.block && line.width != 0.0);
        if feature & flags::WORD_WRAP == 0 || w.width < avail - line.indent {
            if starts || (feature & flags::WORD_WRAP != 0 && avail < line.indent + whole + line.width) {
                add(&mut lines, &mut line, k, &mut counter, &mut widest);
                line.wrapped = !starts;
            }
            line.width += if line.wrapped && k == line.first { w.width } else { whole };
            line.height = line.height.max(w.height);
        } else {
            // (wider than a line: on a line of its own, its overflow left to the edge: the original splits it at a
            // character, `GetBufferLocation`)
            if line.width != 0.0 {
                add(&mut lines, &mut line, k, &mut counter, &mut widest);
                line.wrapped = !w.newline;
            }
            line.width += if line.wrapped && k == line.first { w.width } else { whole };
            line.height = line.height.max(w.height);
        }
        if w.block {
            line.indent = 0.0;
        }
    }
    add(&mut lines, &mut line, words.len(), &mut counter, &mut widest);
    (lines, widest)
}

/// The text's preferred size (`CalculatePreferredSize`, in the views' units): a single line its width and height; a
/// multi-line one wrapped to its minimum width (`min_size`, else `sqrt(13 x total width) x 1.5`, the aspect ratio) and
/// as tall as that makes it. The size given by `min_size`/`max_size` is used as it is when both its sides are at least 0.
pub fn preferred(words: &[Word], feature: u32, default_height: f32, given: Option<Point>, min_width: Option<f32>) -> Point {
    if let Some(p) = given.filter(|p| p.x >= 0.0 && p.y >= 0.0) {
        return p;
    }
    let mut p = if feature & flags::MULTILINE == 0 {
        let width: f32 = words.iter().map(|w| w.space + w.width).sum();
        let height = words.iter().map(|w| w.height).fold(0.0, f32::max);
        Point::new(width - 1.0, if height == 0.0 { default_height } else { height } - 1.0)
    } else {
        let total: f32 = words.iter().map(|w| w.space + w.width).sum();
        let avail = match min_width {
            Some(w) => (w as i32 + 1) as f32,
            None => ((total * 13.0).sqrt() * 1.5) as i32 as f32,
        };
        let (lines, widest) = wrap(words, feature, avail, default_height);
        let height = lines.last().map_or(0.0, |l| l.y + l.height);
        Point::new(widest - 1.0, height - 1.0)
    };
    if feature & flags::ACCEPT_TXT_INPUT != 0 {
        p.x += 4.0;
    }
    if let Some(g) = given {
        if g.x != -1.0 {
            p.x = g.x;
        }
        if g.y != -1.0 {
            p.y = g.y;
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_entities_are_read() {
        let items = parse("a&nbsp;b &lt;c&gt; &amp; &quot;d&quot; &other;", &|_| None);
        let text: String = items.iter().filter_map(|i| if let Item::Text(t, _) = i { Some(t.as_str()) } else { None }).collect();
        assert_eq!(text, "a b <c> & \"d\" &other;");
    }

    fn none(_: &str) -> Option<u32> {
        None
    }

    #[test]
    fn markup_nests_and_restores() {
        let items = parse("a<font color=#ff0000>b<i>c</i>d</font><a href=\"text://x\">e</a><br>f<center>g</center><zz>h", &none);
        let text: Vec<(String, u32, bool)> = items
            .iter()
            .filter_map(|i| match i {
                Item::Text(t, s) => Some((t.clone(), s.colour, s.link.is_some())),
                _ => None,
            })
            .collect();
        assert_eq!(
            text,
            [("a".into(), DEFAULT_COLOUR, false), ("b".into(), 0xff0000, false), ("c".into(), 0xff0000, false), ("d".into(), 0xff0000, false), ("e".into(), DEFAULT_COLOUR, true), ("f".into(), DEFAULT_COLOUR, false), ("g".into(), DEFAULT_COLOUR, false), ("h".into(), DEFAULT_COLOUR, false)]
        );
        assert!(items.contains(&Item::Break));
        let centred = items.iter().find_map(|i| match i {
            Item::Text(t, s) if t == "g" => Some(s.align),
            _ => None,
        });
        assert_eq!(centred, Some(4));
        // (a tag that does not parse stays as text)
        assert!(matches!(&parse("x<font>y", &none)[0], Item::Text(t, _) if t == "x<font>y"));
        // (a font with only a face takes the default colour, not the enclosing one)
        let items = parse("<font color=#00ff00><font face=bold>z</font></font>", &none);
        assert!(matches!(&items[0], Item::Text(t, s) if t == "z" && s.colour == DEFAULT_COLOUR && s.font == Some(FontId(6))));
    }

    #[test]
    fn wraps_words_to_the_width() {
        let m = Measure::default();
        let items = parse("aaa bbb ccc", &none);
        let words = words(&items, &m, FontId::NORMAL, flags::MULTILINE | flags::WORD_WRAP, &|_| None);
        assert_eq!(words.len(), 3);
        let one = words[0].width;
        let space = words[1].space;
        let (lines, _) = wrap(&words, flags::MULTILINE | flags::WORD_WRAP, one * 2.0 + space + 1.0, 13.0);
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[0].first, lines[0].end, lines[1].first), (0, 2, 2));
    }
}
