//! The XML classes ported so far: `BorderView_c`, `Button_c`, `CheckBox_c`, `BitmapView_c` and a first `TextView_c`.

use super::font::{FontId, Measure};
use super::geom::{Point, Rect};
use super::host::{Mouse, Paint, Signal};
use super::view::{Control, Id, Layout, Node, Tree};
use super::xml::Element;
use crate::theme::Palette;
use std::any::Any;

macro_rules! any {
    () => {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    };
}

/// A colour as the views keep it: a colour code (`GUIConfig_c::GetColorID`: "DEFAULT" 0x1000000, "SELECTED" 0x2000000,
/// "HOVER" 0x3000000, "TEXT" 0x4000000, "TEXTSELECTED" 0x5000000, "TEXTHOVER" 0x6000000, the skin's `GUIColors.xml`) or
/// 0x00rrggbb.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colour(pub u32);

impl Colour {
    pub const DEFAULT: Colour = Colour(0x100_0000);
    pub const SELECTED: Colour = Colour(0x200_0000);
    pub const HOVER: Colour = Colour(0x300_0000);
    pub const TEXT: Colour = Colour(0x400_0000);
    pub const WHITE: Colour = Colour(0xff_ffff);

    /// A `color` attribute: a colour name, else the number (`GetAttrInt`, white when missing).
    pub fn attribute(e: &Element, name: &str) -> Colour {
        match e.text(name, "").as_str() {
            "DEFAULT" => Colour::DEFAULT,
            "SELECTED" => Colour::SELECTED,
            "HOVER" => Colour::HOVER,
            "TEXT" => Colour::TEXT,
            "TEXTSELECTED" => Colour(0x500_0000),
            "TEXTHOVER" => Colour(0x600_0000),
            _ => Colour(e.int(name, 0xff_ffff) as u32),
        }
    }

    pub fn rgb(self, palette: &Palette) -> egui::Color32 {
        let c = match self.0 >> 24 {
            // (on a dark theme the border colour, as `chrome::code1` draws the client's other controls)
            1 => crate::chrome::code1(palette),
            2 => palette.selected,
            3 => palette.hover,
            4 => palette.text,
            5 => palette.text_selected,
            6 => palette.hover,
            _ => [(self.0 >> 16) as u8, (self.0 >> 8) as u8, self.0 as u8],
        };
        egui::Color32::from_rgb(c[0], c[1], c[2])
    }
}

fn faded(c: egui::Color32, alpha: f32) -> egui::Color32 {
    c.gamma_multiply(alpha.clamp(0.0, 1.0))
}

/// The greying of a disabled button (`Button_c::StateChanged` 0x101287ea: times 0x909090).
fn greyed(c: egui::Color32, enabled: bool) -> egui::Color32 {
    if enabled { c } else { c * egui::Color32::from_rgb(0x90, 0x90, 0x90) }
}

fn sprite_size(p: &Paint, name: &str) -> egui::Vec2 {
    p.env.skin.and_then(|s| s.get(name)).map_or(egui::Vec2::ZERO, |s| s.size)
}

fn image(p: &Paint, name: &str, rect: egui::Rect, tint: egui::Color32) {
    if let Some(skin) = p.env.skin {
        skin.draw_in(p.painter, name, rect, tint);
    }
}

/// `BorderView_c`'s nine pieces (`CreateGfx` 0x10126212): the corners at their size, the sides stretched between them,
/// the background stretched inside the sides.
pub fn nine_slice(p: &Paint, pieces: &Pieces, rect: egui::Rect, tint: egui::Color32) {
    let size = |n: &Option<String>| n.as_deref().map_or(egui::Vec2::ZERO, |n| sprite_size(p, n));
    let (l, t, r, b) = (size(&pieces.left).x, size(&pieces.top).y, size(&pieces.right).x, size(&pieces.bottom).y);
    let inner = egui::Rect::from_min_max(rect.min + egui::vec2(l, t), rect.max - egui::vec2(r, b));
    let draw = |n: &Option<String>, at: egui::Rect| {
        if let Some(n) = n {
            image(p, n, at, tint);
        }
    };
    draw(&pieces.background, inner);
    let (tl, tr, bl, br) = (size(&pieces.tl), size(&pieces.tr), size(&pieces.bl), size(&pieces.br));
    draw(&pieces.tl, egui::Rect::from_min_size(rect.min, tl));
    draw(&pieces.tr, egui::Rect::from_min_size(egui::pos2(rect.max.x - tr.x, rect.min.y), tr));
    draw(&pieces.bl, egui::Rect::from_min_size(egui::pos2(rect.min.x, rect.max.y - bl.y), bl));
    draw(&pieces.br, egui::Rect::from_min_size(rect.max - br, br));
    draw(&pieces.top, egui::Rect::from_min_max(egui::pos2(rect.min.x + tl.x, rect.min.y), egui::pos2(rect.max.x - tr.x, rect.min.y + t)));
    draw(&pieces.bottom, egui::Rect::from_min_max(egui::pos2(rect.min.x + bl.x, rect.max.y - b), egui::pos2(rect.max.x - br.x, rect.max.y)));
    draw(&pieces.left, egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.min.y + tl.y), egui::pos2(rect.min.x + l, rect.max.y - bl.y)));
    draw(&pieces.right, egui::Rect::from_min_max(egui::pos2(rect.max.x - r, rect.min.y + tr.y), egui::pos2(rect.max.x, rect.max.y - br.y)));
}

/// A border view's sprites by name.
#[derive(Clone, Debug, Default)]
pub struct Pieces {
    pub tl: Option<String>,
    pub tr: Option<String>,
    pub bl: Option<String>,
    pub br: Option<String>,
    pub left: Option<String>,
    pub top: Option<String>,
    pub right: Option<String>,
    pub bottom: Option<String>,
    pub background: Option<String>,
}

impl Pieces {
    /// A nine-slice named `<prefix>_TL` ... `<prefix>_BOTTOM`, with `<prefix>_BACKGROUND` when `background`.
    pub fn named(prefix: &str, background: bool) -> Pieces {
        let n = |s: &str| Some(format!("{prefix}_{s}"));
        Pieces {
            tl: n("TL"),
            tr: n("TR"),
            bl: n("BL"),
            br: n("BR"),
            left: n("LEFT"),
            top: n("TOP"),
            right: n("RIGHT"),
            bottom: n("BOTTOM"),
            background: if background { n("BACKGROUND") } else { None },
        }
    }
}

/// `BorderView_c` (the XML constructor 0x10127243): its pieces (`tl_gfx` ... `bg_gfx`, by default `GFX_GUI_INSET_*` and no
/// background), `alpha` and `color`. With child elements it holds a client view made from the same element (its own
/// `view_layout`, its borders cleared by `SetClient`), over its whole bounds (a `LayoutNode`).
pub struct BorderView {
    pub pieces: Pieces,
    pub alpha: f32,
    pub colour: Colour,
}

impl BorderView {
    pub fn create(tree: &mut Tree, e: &Element) -> Id {
        let piece = |attr: &str, default: Option<&str>| {
            let v = e.text(attr, "");
            if v.is_empty() { default.map(str::to_string) } else { Some(v) }
        };
        let pieces = Pieces {
            tl: piece("tl_gfx", Some("GFX_GUI_INSET_TL")),
            tr: piece("tr_gfx", Some("GFX_GUI_INSET_TR")),
            bl: piece("bl_gfx", Some("GFX_GUI_INSET_BL")),
            br: piece("br_gfx", Some("GFX_GUI_INSET_BR")),
            left: piece("left_gfx", Some("GFX_GUI_INSET_LEFT")),
            top: piece("top_gfx", Some("GFX_GUI_INSET_TOP")),
            right: piece("right_gfx", Some("GFX_GUI_INSET_RIGHT")),
            bottom: piece("bottom_gfx", Some("GFX_GUI_INSET_BOTTOM")),
            background: piece("bg_gfx", None),
        };
        let control = BorderView { pieces, alpha: e.float("alpha", 1.0), colour: Colour::attribute(e, "color") };
        let id = tree.add(Node::from_element(e, 4, Box::new(control)));
        if !e.children.is_empty() {
            let mut client = Node::from_element(e, 4, Box::new(super::view::Plain));
            client.name.clear();
            client.borders = Rect::default();
            let client = tree.add(client);
            super::load_child_views(tree, client, e);
            tree.get_mut(id).layout = Layout::Base;
            tree.add_child(id, client);
        }
        id
    }
}

impl Control for BorderView {
    fn class(&self) -> &'static str {
        "BorderView"
    }
    fn paint(&self, p: &Paint) {
        nine_slice(p, &self.pieces, p.rect, faded(self.colour.rgb(p.env.palette), self.alpha));
    }
    any!();
}

/// `Button_c` (0x10129125 from XML: `label`, `toggled_label`; `Initialize` 0x10128e46): a `GFX_GUI_BORDER01_*` nine-slice
/// in `Default` with its label (font 5) centred in the same colour; pressed `GFX_GUI_BORDER04_*` in `Selected`, under the
/// pointer the `GFX_GUI_BORDER03_*` frame in `Hover` over it, disabled all of it times 0x909090 (`StateChanged`
/// 0x101287ea). A button given one picture instead (`SetGfx`, the BorderView's mode 2: a check box's) draws only that
/// picture, `pictures[1]` when on.
pub struct Button {
    pub label: String,
    pub toggled_label: String,
    pub font: FontId,
    pub toggle: bool,
    pub on: bool,
    /// The single pictures of a mode-2 button: off, on.
    pub pictures: Option<[String; 2]>,
    pub held: bool,
}

impl Button {
    pub fn new(label: &str) -> Button {
        Button { label: label.to_string(), toggled_label: String::new(), font: FontId::NORMAL, toggle: false, on: false, pictures: None, held: false }
    }

    pub fn create(tree: &mut Tree, e: &Element) -> Id {
        let mut b = Button::new(&e.text("label", ""));
        b.toggled_label = e.text("toggled_label", "");
        tree.add(Node::from_element(e, 4, Box::new(b)))
    }

    fn shown_label(&self) -> &str {
        if self.on && !self.toggled_label.is_empty() { &self.toggled_label } else { &self.label }
    }
}

impl Control for Button {
    fn class(&self) -> &'static str {
        "Button"
    }
    /// `Button_c::CalculatePreferredSize` (0x10128396): in the nine-slice mode the label (its width and the toggled
    /// label's, the larger) plus 1, plus fixed borders of 8 each side, less 1, plus 10 across; the label plus 1, 4 and 2
    /// twice down. In the single-picture mode the picture's size.
    fn preferred(&self, m: &Measure, _max: bool) -> Option<Point> {
        if let Some(pictures) = &self.pictures {
            let size = m.ctx.as_ref().and_then(|c| crate::skin::skin(c)).and_then(|s| s.get(&pictures[0])).map_or(egui::vec2(11.0, 11.0), |s| s.size);
            return Some(Point::new(size.x - 1.0, size.y - 1.0));
        }
        let mut text = Point::new(0.0, 0.0);
        if !self.label.is_empty() {
            text = m.line_size(self.font, &self.label);
            if !self.toggled_label.is_empty() {
                text.x = text.x.max(m.line_size(self.font, &self.toggled_label).x);
            }
            text.x += 1.0;
        }
        let (left, top, right, bottom) = (8.0, 1.0, 8.0, 4.0);
        let w = right + left + text.x - 1.0 + 10.0;
        let h = 2.0 + bottom + top + 2.0 + text.y;
        Some(Point::new(w, h))
    }
    fn paint(&self, p: &Paint) {
        if let Some(pictures) = &self.pictures {
            let name = &pictures[usize::from(self.on)];
            let colour = if self.on { p.env.palette.selected } else { p.env.palette.default };
            let size = sprite_size(p, name);
            let at = egui::Rect::from_center_size(p.rect.center(), size);
            image(p, name, at, greyed(egui::Color32::from_rgb(colour[0], colour[1], colour[2]), p.enabled));
            return;
        }
        let down = p.pressed || (self.toggle && self.on);
        let colour = greyed((if down { Colour::SELECTED } else { Colour::DEFAULT }).rgb(p.env.palette), p.enabled);
        nine_slice(p, &Pieces::named(if down { "GFX_GUI_BORDER04" } else { "GFX_GUI_BORDER01" }, true), p.rect, colour);
        if p.enabled && p.hovered {
            nine_slice(p, &Pieces::named("GFX_GUI_BORDER03", false), p.rect, greyed(Colour::HOVER.rgb(p.env.palette), true));
        }
        // (LayoutLabel 0x10127d88: the label's rect centred in the bounds)
        let label = self.shown_label();
        let size = p.env.measure.line_size(self.font, label);
        let at = egui::pos2((p.rect.center().x - (size.x + 1.0) * 0.5).floor(), (p.rect.center().y - (size.y + 1.0) * 0.5).floor());
        text(p, self.font, at, label, colour);
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    /// `ButtonBase_c::MouseDown` (0x101298db) / `MouseUp` (0x101294c9): clicked when let go over it; a toggle button
    /// changes state.
    fn mouse(&mut self, m: Mouse, _at: Point) -> Option<Signal> {
        match m {
            Mouse::Down => {
                self.held = true;
                None
            }
            Mouse::Up { inside } => {
                self.held = false;
                if !inside {
                    return None;
                }
                if self.toggle {
                    self.on = !self.on;
                    return Some(Signal::Toggled(self.on));
                }
                Some(Signal::Clicked)
            }
            Mouse::Move => None,
        }
    }
    any!();
}

/// Text in a GUI font at `at` (its top left), one line.
pub fn text(p: &Paint, font: FontId, at: egui::Pos2, s: &str, colour: egui::Color32) {
    if let Some(b) = p.env.measure.bitmap(font) {
        b.paint(p.painter, at, s, colour);
    } else {
        p.painter.text(at, egui::Align2::LEFT_TOP, s, p.env.measure.egui_font(font), colour);
    }
}

/// `CheckBox_c` (`Initialize` 0x10129eb6): a horizontal view of a toggle `Button_c` showing only
/// `GFX_GUI_CHECKBOX_UNCHECKED` (0x8b) and `GFX_GUI_CHECKBOX_CHECKED` (0x8a) and its label, a font-5 text view 10 to the
/// right in `Default`.
pub struct CheckBox;

impl CheckBox {
    pub fn create(tree: &mut Tree, e: &Element) -> Id {
        let id = tree.add(Node::from_element(e, 4, Box::new(CheckBox)));
        tree.get_mut(id).layout = Layout::Horizontal;
        let mut b = Button::new("");
        b.toggle = true;
        b.pictures = Some(["GFX_GUI_CHECKBOX_UNCHECKED".to_string(), "GFX_GUI_CHECKBOX_CHECKED".to_string()]);
        let button = tree.add(Node::new("", Box::new(b)));
        let mut label = TextView::new(&e.text("label", ""));
        label.colour = Colour::DEFAULT;
        let mut label = Node::new("", Box::new(label));
        label.borders = Rect::new(10.0, 0.0, 0.0, 0.0);
        let label = tree.add(label);
        tree.add_child(id, button);
        tree.add_child(id, label);
        id
    }

    /// Whether a check box is checked.
    pub fn checked(tree: &Tree, id: Id) -> bool {
        tree.get(id).children.first().and_then(|&b| tree.control::<Button>(b)).is_some_and(|b| b.on)
    }

    pub fn set(tree: &mut Tree, id: Id, on: bool) {
        if let Some(&b) = tree.get(id).children.first() {
            if let Some(b) = tree.control_mut::<Button>(b) {
                b.on = on;
            }
        }
    }
}

impl Control for CheckBox {
    fn class(&self) -> &'static str {
        "CheckBox"
    }
    any!();
}

/// `BitmapView_c` (0x10125cc7 from XML: `bitmap_id`): a picture of the skin at its size.
pub struct BitmapView {
    pub bitmap: String,
}

impl Control for BitmapView {
    fn class(&self) -> &'static str {
        "BitmapView"
    }
    fn preferred(&self, m: &Measure, _max: bool) -> Option<Point> {
        let size = m.ctx.as_ref().and_then(|c| crate::skin::skin(c)).and_then(|s| s.get(&self.bitmap)).map_or(egui::Vec2::ZERO, |s| s.size);
        Some(Point::new(size.x - 1.0, size.y - 1.0))
    }
    fn paint(&self, p: &Paint) {
        image(p, &self.bitmap, p.rect, egui::Color32::WHITE);
    }
    any!();
}

/// `TextView_c` (the XML constructor 0x1016538c, `Initialize` 0x10164f66): a scroll view holding a `TextRenderer_c` of
/// `value` (looked up when it starts with `#`) in `font` (NORMAL by default), its `feature_flags`, `min_size`/`max_size`
/// also given to the renderer as its preferred sizes, its colour `color` (white by default: the colour of text with
/// none of its own). The text is the markup of `text.rs`; a click on a link is a [`Signal::Link`]; the wheel scrolls
/// text taller than the view.
pub struct TextView {
    pub text: String,
    pub font: FontId,
    pub feature: u32,
    pub colour: Colour,
    /// `SetMinPreferredSize` / `SetMaxPreferredSize` (+0x278, +0x280): (-1, -1) when not given.
    pub min: Point,
    pub max: Point,
    /// How far the text is scrolled up, and how tall it was when last drawn.
    pub scroll: f32,
    content: std::cell::Cell<f32>,
    /// The link under the pointer when the button went down.
    pressed_link: Option<String>,
    /// Where each drawn link is (relative to the view's top left), for the mouse.
    links: std::cell::RefCell<Vec<(egui::Rect, String)>>,
}

impl TextView {
    pub fn new(text: &str) -> TextView {
        TextView {
            text: text.to_string(),
            font: FontId::NORMAL,
            feature: 0,
            colour: Colour::WHITE,
            min: Point::new(-1.0, -1.0),
            max: Point::new(-1.0, -1.0),
            scroll: 0.0,
            content: std::cell::Cell::new(0.0),
            pressed_link: None,
            links: Default::default(),
        }
    }

    pub fn create(tree: &mut Tree, e: &Element) -> Id {
        let mut t = TextView::new(&e.text("value", ""));
        t.font = FontId::named(&e.text("font", "NORMAL"));
        t.feature = super::text::flags::parse(&e.text("feature_flags", ""));
        t.min = e.point("min_size", Point::new(-1.0, -1.0));
        t.max = e.point("max_size", Point::new(-1.0, -1.0));
        t.colour = Colour(e.int("color", 0xff_ffff) as u32);
        tree.add(Node::from_element(e, 4, Box::new(t)))
    }

    /// `TextView_c::SetText`.
    pub fn set_text(&mut self, text: &str) {
        if self.text != text {
            self.text = text.to_string();
            self.scroll = 0.0;
        }
    }

    fn words(&self, m: &Measure, named: &dyn Fn(&str) -> Option<u32>) -> Vec<super::text::Word> {
        let items = super::text::parse(&self.text, named);
        super::text::words(&items, m, self.font, self.feature, &|_| None)
    }

    /// How tall the text is when wrapped to `width`.
    pub fn height_at(&self, m: &Measure, width: f32) -> f32 {
        let words = self.words(m, &|_| None);
        let (lines, _) = super::text::wrap(&words, self.feature, width, m.height(self.font));
        lines.last().map_or(0.0, |l| l.y + l.height)
    }

    /// The link drawn at a point (relative to the view's top left), as last painted.
    pub fn link_at(&self, at: egui::Pos2) -> Option<String> {
        self.links.borrow().iter().find(|(r, _)| r.contains(at)).map(|(_, l)| l.clone())
    }
}

/// Markup (`HTMLParser_c`'s) drawn into an egui `Ui` the width it has, as a word-wrapped `TextView_c` in `colour` with
/// `font`: returns the link clicked, if any.
pub fn markup(ui: &mut egui::Ui, palette: &Palette, text: &str, font: FontId, colour: egui::Color32) -> Option<String> {
    let measure = Measure { ctx: Some(ui.ctx().clone()) };
    let env = super::host::Env { skin: None, palette, measure: &measure };
    let mut view = TextView::new(text);
    view.font = font;
    view.feature = super::text::flags::MULTILINE | super::text::flags::WORD_WRAP;
    view.colour = Colour(u32::from(colour.r()) << 16 | u32::from(colour.g()) << 8 | u32::from(colour.b()));
    let width = ui.available_width().max(10.0);
    let height = view.height_at(&measure, width).max(measure.height(font));
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return None;
    }
    view.paint(&Paint { painter: ui.painter(), env: &env, rect, hovered: response.hovered(), pressed: false, enabled: true });
    let over = response.hover_pos().and_then(|p| view.link_at(p - rect.min.to_vec2()));
    if over.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() { over } else { None }
}

impl Control for TextView {
    fn class(&self) -> &'static str {
        "TextView"
    }
    fn preferred(&self, m: &Measure, max: bool) -> Option<Point> {
        let words = self.words(m, &|_| None);
        let given = if max { self.max } else { self.min };
        let min_width = (self.min.x >= 0.0).then_some(self.min.x);
        Some(super::text::preferred(&words, self.feature, m.height(self.font), Some(given), min_width))
    }
    fn paint(&self, p: &Paint) {
        use super::text::{DEFAULT_COLOUR, LINK_COLOUR};
        let palette = p.env.palette;
        let named = |n: &str| palette.text_colors.iter().find(|(k, _)| k.eq_ignore_ascii_case(n)).map(|(_, c)| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]));
        let m = p.env.measure;
        let words = self.words(m, &named);
        let width = p.rect.width();
        let (lines, _) = super::text::wrap(&words, self.feature, width, m.height(self.font));
        let total = lines.last().map_or(0.0, |l| l.y + l.height);
        self.content.set(total);
        let view_colour = self.colour.rgb(palette);
        let clip = p.painter.with_clip_rect(p.rect.intersect(p.painter.clip_rect()));
        let sub = Paint { painter: &clip, env: p.env, rect: p.rect, hovered: p.hovered, pressed: p.pressed, enabled: p.enabled };
        let mut links = self.links.borrow_mut();
        links.clear();
        for line in &lines {
            let top = p.rect.top() + line.y - self.scroll;
            if top + line.height < p.rect.top() || top > p.rect.bottom() {
                continue;
            }
            // (the alignment of the line's first text, `_RenderLine`)
            let align = words[line.first..line.end].iter().flat_map(|w| w.pieces.first()).next().map_or(0, |pc| pc.style.align);
            let mut x = p.rect.left() + line.indent;
            match align {
                1 => x += (width - 1.0) - line.width + 1.0,
                4 => x += ((width - line.width) * 0.5).floor(),
                _ => {}
            }
            for w in &words[line.first..line.end] {
                if let Some(s) = w.space_style.as_ref().filter(|s| (s.underline || s.link_look) && w.space > 0.0) {
                    let colour = if s.link_look { LINK_COLOUR & 0xff_ffff } else if s.colour == DEFAULT_COLOUR { u32::from(view_colour.r()) << 16 | u32::from(view_colour.g()) << 8 | u32::from(view_colour.b()) } else { s.colour };
                    let c = egui::Color32::from_rgb((colour >> 16) as u8, (colour >> 8) as u8, colour as u8);
                    let h = m.height(s.font.unwrap_or(self.font));
                    let y = top + line.height - h;
                    clip.rect_filled(egui::Rect::from_min_max(egui::pos2(x, y + h - 2.0), egui::pos2(x + w.space, y + h - 1.0)), 0.0, c);
                }
                x += w.space;
                for piece in &w.pieces {
                    let font = piece.style.font.unwrap_or(self.font);
                    let colour = if piece.style.link_look {
                        LINK_COLOUR & 0xff_ffff
                    } else if piece.style.colour == DEFAULT_COLOUR {
                        u32::from(view_colour.r()) << 16 | u32::from(view_colour.g()) << 8 | u32::from(view_colour.b())
                    } else {
                        piece.style.colour
                    };
                    let c = egui::Color32::from_rgb((colour >> 16) as u8, (colour >> 8) as u8, colour as u8);
                    // (the words of a line stand on its bottom: a smaller font lower)
                    let h = m.height(font);
                    let y = top + line.height - h;
                    text(&sub, font, egui::pos2(x, y), &piece.text, c);
                    if piece.style.underline || piece.style.link_look {
                        // (`_RenderString`: a line a pixel high two above the glyphs' bottom)
                        clip.rect_filled(egui::Rect::from_min_max(egui::pos2(x, y + h - 2.0), egui::pos2(x + piece.width, y + h - 1.0)), 0.0, c);
                    }
                    if let Some(link) = &piece.style.link {
                        let r = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(piece.width, h));
                        links.push((r.translate(-p.rect.min.to_vec2()), link.clone()));
                    }
                    x += piece.width;
                }
            }
        }
    }
    fn takes_mouse(&self) -> bool {
        self.feature & super::text::flags::ACCEPT_MOUSE_INPUT != 0
    }
    fn mouse(&mut self, m: Mouse, at: Point) -> Option<Signal> {
        let link_at = |links: &Vec<(egui::Rect, String)>| links.iter().find(|(r, _)| r.contains(egui::pos2(at.x, at.y))).map(|(_, l)| l.clone());
        match m {
            Mouse::Down => {
                self.pressed_link = link_at(&self.links.borrow());
                None
            }
            Mouse::Up { .. } => {
                let up = link_at(&self.links.borrow());
                let pressed = self.pressed_link.take();
                if up.is_some() && up == pressed { up.map(Signal::Link) } else { None }
            }
            Mouse::Move => None,
        }
    }
    fn wheel(&mut self, dy: f32, height: f32) -> bool {
        let room = (self.content.get() - height).max(0.0);
        if room <= 0.0 {
            return false;
        }
        self.scroll = (self.scroll - dy).clamp(0.0, room);
        true
    }
    any!();
}
