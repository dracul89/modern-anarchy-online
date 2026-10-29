//! `TextButton_c`: a button that is only its text (the character selection's names).
//!
//! The XML constructor (0x1000507a) is a `ButtonBase_c` holding a `TextView_c` made from the same element (its `font`,
//! `feature_flags`, sizes) and reads `text`, then `pressed_text` and `hover_text` (both the text by default) and
//! `raised_text` (the text by default); `color` (none, -1, by default), `pressed_color` and `hover_color` (the colour
//! by default) and `raised_color` (the colour by default). `StateChanged` (0x10004ca2) shows the pressed text in the
//! pressed colour while a toggle button is on, else the hover text in the hover colour while the button is highlighted,
//! else the raised text in the raised colour; disabled, that colour's channels times two thirds
//! (`ButtonBase_c::GetDisabledColor` 0x10129421). The text sits at the button's top left, as big as it wants to be,
//! which is also the button's preferred size (`CalculatePreferredSize` 0x10004e3f).

use super::controls::{Colour, TextView};
use super::font::{FontId, Measure};
use super::geom::Point;
use super::host::{Mouse, Paint, Signal};
use super::view::{Control, Id, Node, Tree};
use super::xml::Element;

pub struct TextButton {
    /// The raised, pressed and hover texts and colours.
    pub texts: [String; 3],
    pub colours: [u32; 3],
    pub toggle: bool,
    pub on: bool,
    view: std::cell::RefCell<TextView>,
}

const RAISED: usize = 0;
const PRESSED: usize = 1;
const HOVER: usize = 2;

/// `ButtonBase_c::GetDisabledColor`: each channel times two thirds.
pub fn disabled(c: u32) -> u32 {
    let ch = |shift: u32| ((c >> shift & 0xff) * 2 / 3) << shift;
    (c & 0xff00_0000) | ch(16) | ch(8) | ch(0)
}

impl TextButton {
    /// Which of the three the button shows.
    fn state(&self, highlighted: bool) -> usize {
        if self.toggle && self.on {
            PRESSED
        } else if highlighted {
            HOVER
        } else {
            RAISED
        }
    }

    /// `SetText`: all three texts.
    pub fn set_text(&mut self, text: &str) {
        self.texts = [text.to_string(), text.to_string(), text.to_string()];
    }

    fn view_for(&self, state: usize, enabled: bool) -> std::cell::RefMut<'_, TextView> {
        let mut v = self.view.borrow_mut();
        v.set_text(&self.texts[state]);
        let c = self.colours[state];
        v.colour = Colour(if enabled { c } else { disabled(c) });
        v
    }
}

impl Control for TextButton {
    fn class(&self) -> &'static str {
        "TextButton"
    }
    fn preferred(&self, m: &Measure, max: bool) -> Option<Point> {
        // (the widest of the three: the button's size does not jump as the pointer comes and goes)
        let mut p = Point::new(-1.0, -1.0);
        for state in [RAISED, PRESSED, HOVER] {
            let q = self.view_for(state, true).preferred(m, max).unwrap_or_default();
            p = Point::new(p.x.max(q.x), p.y.max(q.y));
        }
        Some(p)
    }
    fn paint(&self, p: &Paint) {
        let v = self.view_for(self.state(p.hovered || p.pressed), p.enabled);
        let size = v.preferred(p.env.measure, false).unwrap_or_default();
        let rect = egui::Rect::from_min_size(p.rect.min, egui::vec2(size.x + 1.0, size.y + 1.0));
        v.paint(&Paint { painter: p.painter, env: p.env, rect, hovered: false, pressed: false, enabled: p.enabled });
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    fn mouse(&mut self, m: Mouse, _at: Point) -> Option<Signal> {
        match m {
            Mouse::Up { inside: true } if self.toggle => {
                self.on = !self.on;
                Some(Signal::Toggled(self.on))
            }
            Mouse::Up { inside: true } => Some(Signal::Clicked),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A `TextButton` element.
pub fn create(tree: &mut Tree, e: &Element) -> Id {
    let mut view = TextView::new("");
    view.font = FontId::named(&e.text("font", "NORMAL"));
    view.feature = super::text::flags::parse(&e.text("feature_flags", ""));
    view.min = e.point("min_size", Point::new(-1.0, -1.0));
    view.max = e.point("max_size", Point::new(-1.0, -1.0));
    let text = e.text("text", "");
    let pressed = e.text("pressed_text", &text);
    let hover = e.text("hover_text", &text);
    let raised = e.text("raised_text", &text);
    let colour = e.int("color", -1);
    let colours = [e.int("raised_color", colour) as u32, e.int("pressed_color", colour) as u32, e.int("hover_color", colour) as u32];
    let b = TextButton { texts: [raised, pressed, hover], colours, toggle: false, on: false, view: std::cell::RefCell::new(view) };
    tree.add(Node::from_element(e, 4, Box::new(b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_states() {
        let doc = super::super::xml::parse(r#"<TextButton name="name_btn" font="HUGE" text="Vhab" color="0x0080E9F3" hover_color="0x00FFFFFF" pressed_color="0x00EE4444"/>"#);
        let mut tree = Tree::default();
        let id = super::super::create(&mut tree, &doc[0]).unwrap();
        let b = tree.control::<TextButton>(id).unwrap();
        assert_eq!(b.texts, ["Vhab".to_string(), "Vhab".into(), "Vhab".into()]);
        assert_eq!(b.colours, [0x80e9f3, 0xee4444, 0xffffff]);
        assert_eq!((b.state(false), b.state(true)), (RAISED, HOVER));
        assert_eq!(disabled(0x0080e9f3), 0x0055_9ba2);
    }
}
