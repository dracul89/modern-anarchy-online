//! `PowerbarView_c` (the XML `PowerBar`): a bar filled to a fraction (the limbo countdown, progress dialogs, the raid
//! window's health and nano).
//!
//! The XML constructor (0x1013f846) reads `direction` (`right`, the default; `left`, `up`, `down`), `gfx` (`small_blue`:
//! `GFX_GUI_HOR_BAR_SMALL_EMPTY` and `_BLUE`), `bg_gfx`, `full_gfx`, `left_gfx`, `right_gfx` (picture names) and
//! `Initialize`s (0x1013edfc): the background stretched over the bar between the end caps (`left_gfx` at the start and
//! `right_gfx` at the end, across for right/left bars, upright for up/down ones, in colour code 0x1000000), and the full
//! picture at its own size, centred in the background's size (the halves rounded down) and cut to the value from the bar's
//! start (`Recalculate` 0x1013e946: the kept part's far edge at the floor of the size times the value). The value is 1
//! at first and held to 0 to 1 (`SetValue` 0x1013f283). Its preferred size is the background's (else the full picture's)
//! plus the caps (`CalculatePreferredSize` 0x1013ecb4). The labels (`label`, `left_label`, `right_label`) are not drawn
//! here: no skin gives them.

use super::font::Measure;
use super::geom::Point;
use super::host::Paint;
use super::view::{Control, Id, Node, Tree};
use super::xml::Element;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Down,
    Right,
    Left,
    Up,
}

impl Direction {
    fn across(self) -> bool {
        matches!(self, Direction::Right | Direction::Left)
    }
}

pub struct PowerBar {
    pub direction: Direction,
    pub background: Option<String>,
    pub full: Option<String>,
    pub start_cap: Option<String>,
    pub end_cap: Option<String>,
    /// +0x148: 0 to 1.
    pub value: f32,
    /// `SetBarColor`: the full picture's tint.
    pub colour: egui::Color32,
}

impl PowerBar {
    /// `SetValue`: held to 0 to 1.
    pub fn set_value(&mut self, v: f32) {
        self.value = if v < 0.0 { 0.0 } else if v > 1.0 { 1.0 } else { v };
    }

    /// The kept part of a picture `size` big (from its top left): `Recalculate`'s cut.
    pub fn kept(&self, size: egui::Vec2) -> egui::Rect {
        // (BeOS rects: the far edge is the size less 1 times the value, floored, and the last pixel is kept)
        let (w, h) = (size.x - 1.0, size.y - 1.0);
        let v = self.value;
        let (l, t, r, b) = match self.direction {
            Direction::Down => (0.0, 0.0, w, (h * v).floor()),
            Direction::Right => (0.0, 0.0, (w * v).floor(), h),
            Direction::Left => (((1.0 - v) * w).floor(), 0.0, w, h),
            Direction::Up => (0.0, ((1.0 - v) * h).floor(), w, h),
        };
        egui::Rect::from_min_max(egui::pos2(l, t), egui::pos2(r + 1.0, b + 1.0))
    }
}

fn size_of(m: &Measure, name: &Option<String>) -> Option<egui::Vec2> {
    let name = name.as_ref()?;
    m.ctx.as_ref().and_then(crate::skin::skin).and_then(|s| s.get(name)).map(|s| s.size)
}

impl Control for PowerBar {
    fn class(&self) -> &'static str {
        "PowerBar"
    }
    fn preferred(&self, m: &Measure, _max: bool) -> Option<Point> {
        let body = size_of(m, &self.background).or_else(|| size_of(m, &self.full)).unwrap_or(egui::vec2(1.0, 1.0));
        let caps = [&self.start_cap, &self.end_cap].iter().filter_map(|c| size_of(m, c)).map(|s| if self.direction.across() { s.x } else { s.y }).sum::<f32>();
        let s = if self.direction.across() { body + egui::vec2(caps, 0.0) } else { body + egui::vec2(0.0, caps) };
        Some(Point::new(s.x - 1.0, s.y - 1.0))
    }
    fn paint(&self, p: &Paint) {
        let Some(skin) = p.env.skin else { return };
        let m = p.env.measure;
        let default = super::controls::Colour::DEFAULT.rgb(p.env.palette);
        // (the caps at the ends; the background between them)
        let mut body = p.rect;
        let start = size_of(m, &self.start_cap).unwrap_or(egui::Vec2::ZERO);
        let end = size_of(m, &self.end_cap).unwrap_or(egui::Vec2::ZERO);
        if self.direction.across() {
            body.min.x += start.x;
            body.max.x -= end.x;
        } else {
            body.min.y += start.y;
            body.max.y -= end.y;
        }
        if let Some(c) = &self.start_cap {
            skin.draw_in(p.painter, c, egui::Rect::from_min_size(p.rect.min, if self.direction.across() { egui::vec2(start.x, p.rect.height()) } else { egui::vec2(p.rect.width(), start.y) }), default);
        }
        if let Some(c) = &self.end_cap {
            let at = if self.direction.across() { egui::Rect::from_min_max(egui::pos2(body.right(), p.rect.top()), p.rect.max) } else { egui::Rect::from_min_max(egui::pos2(p.rect.left(), body.bottom()), p.rect.max) };
            skin.draw_in(p.painter, c, at, default);
        }
        if let Some(bg) = &self.background {
            skin.draw_in(p.painter, bg, body, egui::Color32::WHITE);
        }
        let Some(full) = self.full.as_ref().and_then(|n| skin.get(n)) else { return };
        // (centred in the background's size, the halves rounded down)
        let bg = size_of(m, &self.background).unwrap_or(full.size);
        let offset = egui::vec2(((bg.x - full.size.x) * 0.5).floor(), ((bg.y - full.size.y) * 0.5).floor());
        let kept = self.kept(full.size);
        if kept.width() <= 0.0 || kept.height() <= 0.0 {
            return;
        }
        let uv = egui::Rect::from_min_max(egui::pos2(kept.left() / full.size.x, kept.top() / full.size.y), egui::pos2(kept.right() / full.size.x, kept.bottom() / full.size.y));
        p.painter.image(full.texture.id(), kept.translate(body.min.to_vec2() + offset), uv, self.colour);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A `PowerBar` element.
pub fn create(tree: &mut Tree, e: &Element) -> Id {
    let direction = match e.text("direction", "").to_ascii_lowercase().as_str() {
        "left" => Direction::Left,
        "up" => Direction::Up,
        "down" => Direction::Down,
        _ => Direction::Right,
    };
    let named = |key: &str| Some(e.text(key, "")).filter(|s| !s.is_empty());
    let small_blue = e.text("gfx", "").eq_ignore_ascii_case("small_blue");
    let bar = PowerBar {
        direction,
        background: named("bg_gfx").or_else(|| small_blue.then(|| "GFX_GUI_HOR_BAR_SMALL_EMPTY".to_string())),
        full: named("full_gfx").or_else(|| small_blue.then(|| "GFX_GUI_HOR_BAR_SMALL_BLUE".to_string())),
        start_cap: named("left_gfx"),
        end_cap: named("right_gfx"),
        value: 1.0,
        colour: egui::Color32::WHITE,
    };
    tree.add(Node::from_element(e, 4, Box::new(bar)))
}

/// `SetValue` on a bar by id.
pub fn set_value(tree: &mut Tree, id: Id, v: f32) {
    if let Some(b) = tree.control_mut::<PowerBar>(id) {
        b.set_value(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_to_the_value() {
        let doc = super::super::xml::parse(r#"<PowerBar bg_gfx="GFX_GUI_HOR_BAR_EMPTY" full_gfx="GFX_GUI_HOR_BAR_YELLOW" name="progress_bar"/>"#);
        let mut tree = Tree::default();
        let id = super::super::create(&mut tree, &doc[0]).unwrap();
        let b = tree.control::<PowerBar>(id).unwrap();
        assert_eq!((b.direction, b.value), (Direction::Right, 1.0));
        assert_eq!(b.kept(egui::vec2(101.0, 10.0)), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(101.0, 10.0)));
        set_value(&mut tree, id, 0.25);
        let b = tree.control::<PowerBar>(id).unwrap();
        assert_eq!(b.kept(egui::vec2(101.0, 10.0)).right(), 26.0);
        set_value(&mut tree, id, 7.0);
        assert_eq!(tree.control::<PowerBar>(id).unwrap().value, 1.0);
        let up = PowerBar { direction: Direction::Up, background: None, full: None, start_cap: None, end_cap: None, value: 0.5, colour: egui::Color32::WHITE };
        assert_eq!(up.kept(egui::vec2(10.0, 21.0)).top(), 10.0);
    }
}
