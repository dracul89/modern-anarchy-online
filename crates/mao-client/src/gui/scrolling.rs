//! `ScrollingTextView_c`: one line of text that fits its view or slides to and fro to show all of it (the trade
//! window's partner name).
//!
//! The XML constructor (FUN_10008da8) reads `initial_scroll_delay` (1500000 µs by default), `turn_scroll_delay`
//! (700000), `scroll_delay` (20000), `flash_on_time` (600000) and `flash_off_time` (300000) and holds a `TextView_c` in
//! the normal font. Setting the text (FUN_10008ad4; FUN_10008a02 also gives its colour and whether it flashes) takes the
//! text's preferred size and starts over (FUN_1000871e): the text centred upright, and across centred when it fits, else
//! put at the left with the initial delay started. Every `scroll_delay` (FUN_1000886a) the text moves 0.4 (`_DAT_101a9150`)
//! towards its far end; at an end it stops there and waits `turn_scroll_delay` (`LAB_10008853` then starts the steps
//! again) and turns back. Flashing (FUN_100089b4) shows the text for `flash_on_time` and hides it for `flash_off_time`.

use std::cell::Cell;
use std::time::{Duration, Instant};

use super::controls::{Colour, TextView};
use super::font::Measure;
use super::geom::Point;
use super::host::Paint;
use super::view::{Control, Id, Node, Tree};
use super::xml::Element;

/// How far a step moves the text (`_DAT_101a9150`).
const STEP: f32 = 0.4;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// The text fits: no steps.
    Still,
    /// Waiting (the initial or a turn delay) until then.
    Waiting(Instant),
    /// Stepping; the next step then.
    Stepping(Instant),
}

pub struct ScrollingText {
    pub text: TextView,
    pub initial_delay: Duration,
    pub turn_delay: Duration,
    pub step_delay: Duration,
    pub flash_on: Duration,
    pub flash_off: Duration,
    /// Flashing: whether the text shows and until when.
    flash: Cell<Option<(bool, Instant)>>,
    /// +0x12c: how far the text is moved left (not above 0); +0x128: moving towards its end (else back to its start).
    offset: Cell<f32>,
    forward: Cell<bool>,
    phase: Cell<Phase>,
    /// The width the text was last started over for (`None`: start over at the next drawing).
    width: Cell<Option<f32>>,
}

impl ScrollingText {
    fn new(us: impl Fn(&str, i32) -> i32) -> ScrollingText {
        let d = |name: &str, default: i32| Duration::from_micros(us(name, default).max(0) as u64);
        ScrollingText {
            text: TextView::new(""),
            initial_delay: d("initial_scroll_delay", 1_500_000),
            turn_delay: d("turn_scroll_delay", 700_000),
            step_delay: d("scroll_delay", 20_000).max(Duration::from_millis(1)),
            flash_on: d("flash_on_time", 600_000),
            flash_off: d("flash_off_time", 300_000),
            flash: Cell::new(None),
            offset: Cell::new(0.0),
            forward: Cell::new(false),
            phase: Cell::new(Phase::Still),
            width: Cell::new(None),
        }
    }

    /// FUN_1000871e: back to the start for a view `width` wide and text `text_width` wide.
    fn start_over(&self, width: f32, text_width: f32, now: Instant) {
        self.forward.set(false);
        self.offset.set(0.0);
        self.width.set(Some(width));
        self.phase.set(if text_width <= width { Phase::Still } else { Phase::Waiting(now + self.initial_delay) });
    }

    /// The timers up to `now` (FUN_1000886a's steps, `LAB_10008853`'s restart, FUN_100089b4's flashing).
    fn advance(&self, width: f32, text_width: f32, now: Instant) {
        loop {
            match self.phase.get() {
                Phase::Still => break,
                Phase::Waiting(until) if now >= until => self.phase.set(Phase::Stepping(until + self.step_delay)),
                Phase::Waiting(_) => break,
                Phase::Stepping(next) if now >= next => {
                    let mut turned = false;
                    if !self.forward.get() {
                        let o = self.offset.get() + STEP;
                        self.offset.set(o);
                        if o > 0.0 {
                            self.offset.set(0.0);
                            self.forward.set(true);
                            turned = true;
                        }
                    } else {
                        let over = text_width - width;
                        let o = self.offset.get() - STEP;
                        self.offset.set(o);
                        if over < -o {
                            self.offset.set(-over);
                            self.forward.set(false);
                            turned = true;
                        }
                    }
                    self.phase.set(if turned { Phase::Waiting(next + self.turn_delay) } else { Phase::Stepping(next + self.step_delay) });
                }
                Phase::Stepping(_) => break,
            }
        }
        while let Some((shown, until)) = self.flash.get().filter(|&(_, until)| now >= until) {
            let next = until + if shown { self.flash_off } else { self.flash_on };
            self.flash.set(Some((!shown, next)));
        }
    }

    /// The text's preferred size in pixels.
    fn text_size(&self, m: &Measure) -> egui::Vec2 {
        let p = self.text.preferred(m, false).unwrap_or_default();
        egui::vec2(p.x + 1.0, p.y + 1.0)
    }

    /// When the view next needs drawing.
    fn next_change(&self) -> Option<Instant> {
        let timer = match self.phase.get() {
            Phase::Still => None,
            Phase::Waiting(t) | Phase::Stepping(t) => Some(t),
        };
        let flash = self.flash.get().map(|(_, t)| t);
        [timer, flash].into_iter().flatten().min()
    }
}

impl Control for ScrollingText {
    fn class(&self) -> &'static str {
        "ScrollingTextView"
    }
    fn preferred(&self, m: &Measure, _max: bool) -> Option<Point> {
        // (as high as the text; across what its layout gives it)
        let s = self.text_size(m);
        Some(Point::new(-1.0, s.y - 1.0))
    }
    fn clips(&self) -> bool {
        true
    }
    fn paint(&self, p: &Paint) {
        let m = p.env.measure;
        let size = self.text_size(m);
        let width = p.rect.width();
        let now = Instant::now();
        if self.width.get() != Some(width) {
            self.start_over(width, size.x, now);
        }
        self.advance(width, size.x, now);
        if let Some(t) = self.next_change() {
            p.painter.ctx().request_repaint_after(t.saturating_duration_since(now));
        }
        if self.flash.get().is_some_and(|(shown, _)| !shown) {
            return;
        }
        let x = if size.x <= width { (width * 0.5 - 0.5 * size.x).floor() } else { self.offset.get().floor() };
        let y = (p.rect.height() * 0.5 - 0.5 * size.y).floor();
        let rect = egui::Rect::from_min_size(p.rect.min + egui::vec2(x, y), size);
        let clip = p.painter.with_clip_rect(p.rect.intersect(p.painter.clip_rect()));
        self.text.paint(&Paint { painter: &clip, env: p.env, rect, hovered: false, pressed: false, enabled: p.enabled });
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A `ScrollingTextView` element.
pub fn create(tree: &mut Tree, e: &Element) -> Id {
    tree.add(Node::from_element(e, 4, Box::new(ScrollingText::new(|name, default| e.int(name, default)))))
}

/// FUN_10008ad4 / FUN_10008a02: the text (in `colour`, flashing or not), started over.
pub fn set_text(tree: &mut Tree, id: Id, text: &str, colour: Option<Colour>, flash: bool) {
    if let Some(s) = tree.control_mut::<ScrollingText>(id) {
        s.text.text = text.to_string();
        if let Some(c) = colour {
            s.text.colour = c;
        }
        s.width.set(None);
        s.flash.set(flash.then(|| (true, Instant::now() + s.flash_on)));
    }
    tree.relayout = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slides_to_and_fro() {
        let s = ScrollingText::new(|_, d| d);
        let t0 = Instant::now();
        // 30 wider than its view: waits 1.5 s, steps 0.4 left every 20 ms (the first step bounces off the start and
        // waits 0.7 s), turns at -30 and comes back
        s.start_over(100.0, 130.0, t0);
        s.advance(100.0, 130.0, t0 + Duration::from_millis(1400));
        assert_eq!(s.offset.get(), 0.0);
        s.advance(100.0, 130.0, t0 + Duration::from_millis(1520));
        assert!(s.forward.get());
        // 0.7 s on, then 75 steps reach the end
        s.advance(100.0, 130.0, t0 + Duration::from_millis(1520 + 700 + 20 * 10));
        assert!((s.offset.get() + 4.0).abs() < 0.01, "{}", s.offset.get());
        s.advance(100.0, 130.0, t0 + Duration::from_millis(1520 + 700 + 20 * 76));
        assert_eq!(s.offset.get(), -30.0);
        assert!(!s.forward.get());
        // a text that fits does not move
        s.start_over(100.0, 80.0, t0);
        s.advance(100.0, 80.0, t0 + Duration::from_secs(10));
        assert_eq!((s.offset.get(), s.phase.get()), (0.0, Phase::Still));
    }
}
