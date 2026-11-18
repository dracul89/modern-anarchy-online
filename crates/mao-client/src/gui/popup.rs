//! `PopupMenu_c`: a list of entries in a small window of its own over everything else.
//!
//! `Go` (0x1013c13a) makes the window: a `BorderView_c` of the `GFX_GUI_INSET_*` pieces over `GFX_GUI_WINDOW_BACKGROUND`
//! (0xf9 ... 0xf5, 0x1bf) holding, 3 in on every side, a scroll view (vertical bar on `auto`) of the menu; moved inside the
//! screen, and it takes the keyboard. `_Layout` (0x1013b9c9) stacks the entries from the top, 3 apart (`_DAT_101a8a28`),
//! each as wide as the widest icon, label and tail together plus 2 (`_DAT_101a8b98`), with 3 more of room at the right;
//! the menu is at most the screen's height less 100 tall (`CalculatePreferredSize` 0x1013b7f3).
//!
//! **The entries** (`PopupMenuItem_c` 0x1013dee8):
//! * a string entry (`PopupMenuStringItem_c` 0x1013e6ce): its label (a `TextView_c` in font 5) a pixel right of the
//!   icon column, centred in its height; white, or 0xa0a0a0 when disabled (`EnabledStateChanged` 0x1013e8ed). A
//!   boolean one (`MakeBoolean`) shows `GFX_GUI_MENU_CHECK` in the **tail** column, right of the labels, while it is
//!   checked, and keeps that column's room when it is not; one with a sub-menu shows `GFX_GUI_MENU_SUB_ARROW` there
//!   (and cannot be boolean). The tail column is the widest tail picture and 10 (`GetTailWidth` 0x1013da3e);
//! * a separator (`PopupMenuSeparatorItem_c` 0x1013e184): 5 tall, two lines across the entry, 0x606060 at 2 and
//!   0xd0d0d0 at 3; the pointer, the keys and the search pass over it (`IsStatic`);
//! * a slider (`PopupMenuSliderItem_c` 0x1013e41a): a `Slider_c` in the label column, 80 wide at least, no label and
//!   no highlight. Only its knob is grabbed; every change is told while it is dragged and the letting go as the last,
//!   and the menu stays open. The chat window's two transparency levels are the only ones in the original.
//!
//! No entry of any menu in GUI.dll has an icon (every builder passes -1), and `EnableMultiselect` has no caller:
//! neither is here.
//!
//! **The mouse** (`MouseMove` 0x1013bdc2, `MouseDown` 0x1013d179, `MouseUp` 0x1013ca36): the entry under the pointer
//! is highlighted (a 0x2020c0 band, `Highlight` 0x1013dd7d), disabled ones too, and the highlight stays when the
//! pointer leaves the entries. A button, any of them, let go after a press in the menu chooses the highlighted entry
//! (`ActivateHighlightedItem` 0x1013c90f: nothing for one with a sub-menu; a disabled one closes the menu without a
//! choice). A press outside every menu of the chain closes it (`SlotGlobalMouseDown` 0x1013cafc).
//!
//! **Sub-menus** (`HighlightItem` 0x1013bc01, `SlotSubMenuTimer` 0x1013cbb6): the highlight coming to an entry with
//! one opens it after 300 ms, or after 200 ms when another is open (which closes: the same 200 ms closes one when
//! the highlight goes to an entry without); a press on the entry and the Right key open it at once. It stands with
//! its top level with the entry's, 6 right of the menu's entries, or left of them when it would leave the screen
//! or when its parent already stands left of its own; the pointer coming into it highlights its entry in the
//! parent again. A choice in it closes the whole chain and is told with its path; so does a cancel; the Left key
//! closes that sub-menu only. (Not kept: the original leaves a sub-menu closed by Left marked as open, so Right
//! does nothing until the highlight has moved away and back. And here a disabled entry's sub-menu does not open.)
//!
//! **The keys** (`KeyDown` 0x1013d1cc), for the newest menu of the chain: Escape, Delete and Backspace cancel; Enter
//! chooses; Up and Down go to the next entry that is no separator, without wrapping (Up from the top or from
//! nothing: the first); Tab goes a page on, with Shift back; Right and Left as above. **Typed text**
//! (`TextInput` 0x1013c59b, `HighlightSearchMatch` 0x1013bf6f) gathers while less than a second passes between
//! letters, and highlights the first entry whose label begins with it, letters' case and leading slashes aside.
//!
//! The wheel scrolls a menu taller than its room, and the highlighted entry is kept in view (`MakeItemVisible`
//! 0x1013b8f6); no scroll bar is drawn.

use std::time::{Duration, Instant};

use super::font::FontId;
use super::host::Env;

/// What an entry is.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Kind {
    #[default]
    Text,
    Separator,
    Slider {
        min: f64,
        max: f64,
        value: f64,
    },
}

/// An entry: its label, whether it can be chosen, its check mark when it is one that has one (ticked or not), what
/// it is, and its sub-menu.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub label: String,
    pub enabled: bool,
    pub mark: Option<bool>,
    pub kind: Kind,
    pub sub: Option<Vec<Entry>>,
    /// The tip while the pointer rests on it (the chat window's two sliders have one each and no label).
    pub tip: String,
}

impl Entry {
    pub fn text(label: impl Into<String>, enabled: bool) -> Entry {
        Entry { label: label.into(), enabled, ..Entry::default() }
    }

    pub fn marked(label: impl Into<String>, enabled: bool, mark: bool) -> Entry {
        Entry { label: label.into(), enabled, mark: Some(mark), ..Entry::default() }
    }

    pub fn separator() -> Entry {
        Entry { enabled: true, kind: Kind::Separator, ..Entry::default() }
    }

    pub fn slider(min: f64, max: f64, value: f64) -> Entry {
        Entry { enabled: true, kind: Kind::Slider { min, max, value }, ..Entry::default() }
    }

    pub fn submenu(label: impl Into<String>, enabled: bool, entries: Vec<Entry>) -> Entry {
        Entry { label: label.into(), enabled, sub: Some(entries), ..Entry::default() }
    }

    pub fn tipped(mut self, tip: impl Into<String>) -> Entry {
        self.tip = tip.into();
        self
    }

    /// `IsStatic`: the pointer, the keys and the search pass over it.
    pub fn is_static(&self) -> bool {
        self.kind == Kind::Separator
    }
}

/// What a frame of a flat menu did ([`PopupMenu::show`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    None,
    /// An entry was chosen (its index); the menu closed.
    Chosen(usize),
    /// It closed without a choice.
    Closed,
}

/// What a frame of a menu did ([`PopupMenu::frame`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    None,
    /// An entry was chosen: its place, through the sub-menus that led to it; the menus closed.
    Chosen(Vec<usize>),
    /// They closed without a choice.
    Closed,
    /// A slider entry's value changed (the last of a drag: `true`); the menus stay.
    Slider(Vec<usize>, f64, bool),
}

/// What a menu of the chain tells its parent.
enum Step {
    None,
    Chosen(Vec<usize>),
    Cancel,
    /// The Left key: this sub-menu closes, the rest stays.
    Back,
    Slider(Vec<usize>, f64, bool),
}

/// Where a sub-menu stands by: its parent's entries from left to right, the top of its entry, and whether the
/// parent itself stands left of its own parent.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Anchor {
    left: f32,
    right: f32,
    top: f32,
    leftwards: bool,
}

/// An open menu.
#[derive(Clone, Debug, Default)]
pub struct PopupMenu {
    pub entries: Vec<Entry>,
    /// The entry under the pointer or picked by the keys (`HighlightItem`).
    pub highlighted: Option<usize>,
    /// Where it opened (its top left on the screen).
    pub at: egui::Pos2,
    /// The frame it opened in (the click that opened it does not close it).
    opened: u64,
    /// A button went down in the menu (+0x259): its letting go chooses.
    pressed: bool,
    /// The open sub-menu and the entry it is of (+0x190).
    sub: Option<(usize, Box<PopupMenu>)>,
    /// When the sub-menu timer is due (+0x1e0).
    sub_due: Option<Instant>,
    /// The text typed (+0x220) and when its last letter came.
    search: String,
    typed: Option<Instant>,
    /// How far the entries are scrolled up.
    scroll: f32,
    anchor: Option<Anchor>,
    /// The slider being dragged: its entry and where its knob was grabbed (from the knob's left).
    grabbed: Option<(usize, f32)>,
    /// Where the menu was drawn last, its window and its entries.
    rect: Option<egui::Rect>,
}

/// The entries' gap and the frame's room round them.
const GAP: f32 = 3.0;
const INSET: f32 = 3.0;
/// The room right of the entries (the menu's margins, `Rect(0, 0, 3, 0)`).
const MARGIN: f32 = 3.0;
const CHECK: &str = "GFX_GUI_MENU_CHECK";
const ARROW: &str = "GFX_GUI_MENU_SUB_ARROW";
/// The sub-menu timer: to open one when none is open, and to close or change one.
const OPEN_AFTER: Duration = Duration::from_millis(300);
const CHANGE_AFTER: Duration = Duration::from_millis(200);
/// How long the typed text gathers (`SlotSearchResetTimer`).
const SEARCH_FOR: Duration = Duration::from_secs(1);

/// The entries' tops and the height of them all (`_Layout`: each 3 under the one before; one less than the last
/// one's gap at the end).
pub fn stack(heights: &[f32]) -> (Vec<f32>, f32) {
    let mut y = 0.0;
    let tops = heights
        .iter()
        .map(|h| {
            let top = y;
            y += h + GAP;
            top
        })
        .collect();
    (tops, (y - 1.0).max(0.0))
}

/// The entry Up or Down goes to (`KeyDown`): the next one that is no separator, none further: where it was; Up from
/// the first or from nothing: the first; Down from nothing: the first that is no separator.
pub fn step(entries: &[Entry], from: Option<usize>, down: bool) -> Option<usize> {
    let free = |k: &usize| !entries[*k].is_static();
    match (from, down) {
        (None, true) => (0..entries.len()).find(free),
        (Some(at), true) => (at + 1..entries.len()).find(free).or(from),
        (Some(at), false) if at > 0 => (0..at).rev().find(free).or(from),
        (_, false) => (!entries.is_empty()).then_some(0),
    }
}

/// The entry Tab goes to: `by` entries on (back with Shift: negative), held inside the menu, then on past separators.
pub fn page(entries: &[Entry], from: Option<usize>, by: i32) -> Option<usize> {
    if entries.is_empty() {
        return None;
    }
    let last = entries.len() as i32 - 1;
    let at = (from.map_or(0, |k| k as i32) + by).clamp(0, last) as usize;
    (at..entries.len()).find(|k| !entries[*k].is_static()).or(Some(at))
}

/// The entry the typed text finds (`HighlightSearchMatch`): the first that is no separator whose label begins with
/// it, the letters' case and the slashes before either aside.
pub fn search_match(entries: &[Entry], typed: &str) -> Option<usize> {
    let typed = typed.trim_start_matches('/').to_lowercase();
    entries.iter().position(|e| !e.is_static() && e.label.trim_start_matches('/').to_lowercase().starts_with(&typed))
}

/// A sub-menu's left edge (`SlotSubMenuTimer`'s placing): 6 right of the parent's entries, or, `leftwards`, its
/// width and 6 left of them; turned to the other side when it would leave the screen there.
pub fn sub_left(entries: (f32, f32), width: f32, screen: (f32, f32), leftwards: bool) -> f32 {
    let (left, right) = entries;
    let turn = (right - left) + width + 12.0;
    if leftwards {
        let x = left - 6.0 - width;
        if x < screen.0 { x + turn } else { x }
    } else {
        let x = right + 6.0;
        if screen.1 < x + width { x - turn } else { x }
    }
}

impl PopupMenu {
    pub fn new(entries: Vec<Entry>, at: egui::Pos2, highlighted: Option<usize>, frame: u64) -> PopupMenu {
        PopupMenu { entries, highlighted, at, opened: frame, ..PopupMenu::default() }
    }

    /// One frame of a menu without sub-menus: drawn at `at`, the mouse and keys handled.
    pub fn show(&mut self, ctx: &egui::Context, env: &Env, id: egui::Id) -> Outcome {
        match self.frame(ctx, env, id) {
            Event::Chosen(path) if path.len() == 1 => Outcome::Chosen(path[0]),
            Event::Chosen(_) | Event::Closed => Outcome::Closed,
            Event::None | Event::Slider(..) => Outcome::None,
        }
    }

    /// One frame of the menu and its open sub-menus.
    pub fn frame(&mut self, ctx: &egui::Context, env: &Env, id: egui::Id) -> Event {
        // (`Go` takes the keyboard: while a menu is up the keys are its own)
        super::host::take_keyboard();
        let step = self.level(ctx, env, id, 0);
        // (a press outside every menu of the chain closes them; not the one that opened it)
        let outside = ctx.cumulative_frame_nr() > self.opened + 1 && ctx.input(|i| i.pointer.any_pressed()) && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| !self.holds(p));
        ctx.request_repaint();
        match step {
            Step::Chosen(path) => Event::Chosen(path),
            Step::Cancel => Event::Closed,
            Step::Slider(path, value, done) => Event::Slider(path, value, done),
            Step::None | Step::Back if outside => Event::Closed,
            Step::None | Step::Back => Event::None,
        }
    }

    /// Whether a point is in the menu's window or in one of its sub-menus'.
    fn holds(&self, point: egui::Pos2) -> bool {
        self.rect.is_some_and(|r| r.contains(point)) || self.sub.as_ref().is_some_and(|s| s.1.holds(point))
    }

    /// `HighlightItem`: the entry is the highlighted one, and the sub-menu timer runs unless the open sub-menu is
    /// already that entry's (or it has none and none is open).
    fn highlight(&mut self, entry: usize, now: Instant) {
        if self.highlighted == Some(entry) {
            return;
        }
        self.highlighted = Some(entry);
        let open = self.sub.as_ref().map(|s| s.0);
        let wanted = self.entries[entry].sub.is_some().then_some(entry);
        self.sub_due = (wanted != open).then(|| now + if wanted.is_some() && open.is_none() { OPEN_AFTER } else { CHANGE_AFTER });
    }

    /// The highlighted entry's sub-menu is the open one from now (`SlotSubMenuTimer`, `SelectHighlightedItem`).
    fn open_sub(&mut self, frame: u64) {
        self.sub_due = None;
        let Some(entry) = self.highlighted else { return };
        if self.sub.as_ref().is_some_and(|s| s.0 == entry) {
            return;
        }
        self.sub = None;
        if let Some(entries) = self.entries[entry].sub.clone().filter(|_| self.entries[entry].enabled) {
            self.sub = Some((entry, Box::new(PopupMenu::new(entries, self.at, None, frame))));
        }
    }

    /// `ActivateHighlightedItem`.
    fn activate(&self) -> Step {
        match self.highlighted.map(|k| &self.entries[k]) {
            None => Step::None,
            Some(e) if e.sub.is_some() => Step::None,
            Some(e) if !e.enabled => Step::Cancel,
            Some(_) => Step::Chosen(vec![self.highlighted.unwrap_or(0)]),
        }
    }

    fn level(&mut self, ctx: &egui::Context, env: &Env, id: egui::Id, depth: usize) -> Step {
        let m = env.measure;
        let h = m.height(FontId::NORMAL).ceil();
        let size = |n: &str, own: egui::Vec2| env.skin.and_then(|s| s.get(n)).map_or(own, |s| s.size);
        let (check, arrow, knob) = (size(CHECK, egui::vec2(9.0, 9.0)), size(ARROW, egui::vec2(5.0, 9.0)), size(super::controls::SLIDER_KNOB, egui::vec2(8.0, 12.0)));
        let heights: Vec<f32> = self
            .entries
            .iter()
            .map(|e| match e.kind {
                Kind::Text => h,
                Kind::Separator => 5.0,
                Kind::Slider { .. } => knob.y.max(4.0),
            })
            .collect();
        let labels = self
            .entries
            .iter()
            .map(|e| match e.kind {
                Kind::Text => m.width(FontId::NORMAL, &e.label),
                Kind::Separator => 10.0,
                Kind::Slider { .. } => 80.0,
            })
            .fold(0.0f32, f32::max)
            .ceil();
        let tail = self.entries.iter().map(|e| if e.sub.is_some() { arrow.x + 10.0 } else if e.mark.is_some() { check.x + 10.0 } else { 0.0 }).fold(0.0f32, f32::max);
        let width = labels + tail + 2.0;
        let (tops, total) = stack(&heights);
        // (the inset frame's pieces round the room)
        let piece = |n: &str| env.skin.and_then(|s| s.get(n)).map_or(egui::Vec2::splat(2.0), |s| s.size);
        let (l, t, r, b) = (piece("GFX_GUI_INSET_LEFT").x, piece("GFX_GUI_INSET_TOP").y, piece("GFX_GUI_INSET_RIGHT").x, piece("GFX_GUI_INSET_BOTTOM").y);
        let screen = ctx.content_rect();
        let room = (screen.height() - 100.0 - 2.0 * INSET - t - b).max(h);
        let shown = total.min(room);
        let outer = egui::vec2(width + MARGIN + 2.0 * INSET + l + r, shown + 2.0 * INSET + t + b);
        // (a sub-menu stands by its entry; `MoveInsideScreen`)
        let wanted = match self.anchor {
            Some(a) => egui::pos2(sub_left((a.left, a.right), outer.x, (screen.left(), screen.right()), a.leftwards), a.top),
            None => self.at,
        };
        let at = egui::pos2(wanted.x.min(screen.right() - outer.x).max(screen.left()), wanted.y.min(screen.bottom() - outer.y).max(screen.top()));
        let rect = egui::Rect::from_min_size(at, outer);
        self.rect = Some(rect);
        let first = egui::pos2(rect.left() + l + INSET, rect.top() + t + INSET);
        let view = egui::Rect::from_min_size(first, egui::vec2(width + MARGIN, shown));
        self.scroll = self.scroll.clamp(0.0, (total - shown).max(0.0));
        let scroll = self.scroll;
        let entry_rect = |k: usize| egui::Rect::from_min_size(egui::pos2(first.x, first.y + tops[k] - scroll), egui::vec2(width, heights[k]));
        let track = |k: usize| egui::Rect::from_min_size(egui::pos2(first.x + 1.0, first.y + tops[k] - scroll), egui::vec2(labels, heights[k]));
        let now = Instant::now();
        let frame = ctx.cumulative_frame_nr();
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let in_sub = pointer.is_some_and(|p| self.sub.as_ref().is_some_and(|s| s.1.holds(p)));
        let inside = !in_sub && pointer.is_some_and(|p| rect.contains(p));
        let under = pointer.filter(|_| inside).and_then(|p| (0..self.entries.len()).find(|&k| entry_rect(k).contains(p) && view.contains(p)));
        let mut out = Step::None;
        super::tooltip::hover(ctx, env, id.with("tip"), under.map(|k| id.with(("tip", k))), under.map_or("", |k| self.entries[k].tip.as_str()), "");

        // The pointer: into the sub-menu, its entry is the highlighted one again; over an entry, that one.
        if in_sub {
            if let Some(entry) = self.sub.as_ref().map(|s| s.0) {
                self.highlighted = Some(entry);
                self.sub_due = None;
            }
        } else if let Some(k) = under.filter(|k| !self.entries[*k].is_static() && ctx.input(|i| i.pointer.is_moving())) {
            self.highlight(k, now);
        }
        if inside {
            let wheel = ctx.input(|i| i.smooth_scroll_delta.y);
            self.scroll = (self.scroll - wheel).clamp(0.0, (total - shown).max(0.0));
        }
        if self.sub_due.is_some_and(|due| now >= due) {
            self.open_sub(frame);
        }

        // The buttons, whichever: a press in the menu, and the letting go that chooses.
        let (down, up) = ctx.input(|i| (i.pointer.any_pressed(), i.pointer.any_released()));
        if down && inside && frame > self.opened + 1 {
            self.pressed = true;
            let knob_at = |k: usize, value: f64, min: f64, max: f64| {
                let t = track(k);
                let part = if max > min { ((value - min) / (max - min)).clamp(0.0, 1.0) as f32 } else { 0.0 };
                egui::Rect::from_min_size(egui::pos2(t.left() + part * (t.width() - knob.x).max(0.0), t.top()), egui::vec2(knob.x, t.height()))
            };
            match under.map(|k| (k, self.entries[k].kind.clone())) {
                // (only the knob is grabbed: `Slider_c::MouseDown`)
                Some((k, Kind::Slider { min, max, value })) => {
                    if let Some(p) = pointer.filter(|p| knob_at(k, value, min, max).contains(*p)) {
                        self.grabbed = Some((k, p.x - knob_at(k, value, min, max).left()));
                    }
                }
                _ => self.open_sub_now(frame),
            }
        }
        if let (Some((k, grab)), Some(p)) = (self.grabbed, pointer) {
            if let Kind::Slider { min, max, value } = &mut self.entries[k].kind {
                let t = track(k);
                let run = (t.width() - knob.x).max(1.0);
                let new = *min + (*max - *min) * f64::from(((p.x - grab - t.left()) / run).clamp(0.0, 1.0));
                if up {
                    *value = new;
                    out = Step::Slider(vec![k], new, true);
                } else if (new - *value).abs() > f64::EPSILON {
                    *value = new;
                    out = Step::Slider(vec![k], new, false);
                }
            }
        }
        if up {
            if self.grabbed.take().is_none() && std::mem::take(&mut self.pressed) {
                out = self.activate();
            }
            self.pressed = false;
        }

        // The keys and the typed text, for the newest menu.
        if self.sub.is_none() {
            if self.typed.is_some_and(|t| now.duration_since(t) >= SEARCH_FOR) {
                self.search.clear();
                self.typed = None;
            }
            let (events, shift) = ctx.input(|i| (i.events.clone(), i.modifiers.shift));
            for event in events {
                match event {
                    egui::Event::Text(text) => {
                        self.search.push_str(&text);
                        self.typed = Some(now);
                        if let Some(k) = search_match(&self.entries, &self.search) {
                            self.highlight(k, now);
                        }
                    }
                    egui::Event::Key { key, pressed: true, .. } => match key {
                        egui::Key::Escape | egui::Key::Delete | egui::Key::Backspace => out = Step::Cancel,
                        egui::Key::Enter => out = self.activate(),
                        egui::Key::Tab => {
                            let by = (((shown + 1.0) / (heights.first().copied().unwrap_or(h) + GAP + 1.0)) as i32).max(1);
                            if let Some(k) = page(&self.entries, self.highlighted, if shift { -by } else { by }) {
                                self.highlight(k, now);
                            }
                        }
                        egui::Key::ArrowDown | egui::Key::ArrowUp => {
                            if let Some(k) = step(&self.entries, self.highlighted, key == egui::Key::ArrowDown) {
                                self.highlight(k, now);
                            }
                        }
                        egui::Key::ArrowRight => self.open_sub_now(frame),
                        egui::Key::ArrowLeft if depth > 0 => out = Step::Back,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        // (`MakeItemVisible`)
        if let Some(k) = self.highlighted {
            if tops[k] < self.scroll {
                self.scroll = tops[k];
            } else if tops[k] + heights[k] > self.scroll + shown {
                self.scroll = tops[k] + heights[k] - shown;
            }
        }

        let highlighted = self.highlighted;
        egui::Area::new(id).order(egui::Order::Foreground).fixed_pos(at).interactable(true).show(ctx, |ui| {
            ui.allocate_exact_size(outer, egui::Sense::click());
            let painter = ui.painter();
            if let Some(skin) = env.skin {
                skin.draw_in(painter, "GFX_GUI_WINDOW_BACKGROUND", rect.shrink2(egui::vec2(l, t)), egui::Color32::WHITE);
                let pieces = super::controls::Pieces::named("GFX_GUI_INSET", false);
                let p = super::host::Paint { painter, env, rect, hovered: false, pressed: false, enabled: true };
                super::controls::nine_slice(&p, &pieces, rect, egui::Color32::WHITE);
            } else {
                painter.rect_filled(rect, 0.0, egui::Color32::from_gray(30));
            }
            let clip = painter.with_clip_rect(view.intersect(rect.shrink2(egui::vec2(l, t))));
            for (k, e) in self.entries.iter().enumerate() {
                let r = entry_rect(k);
                let colour = if e.enabled { egui::Color32::WHITE } else { egui::Color32::from_gray(0xa0) };
                match e.kind {
                    Kind::Separator => {
                        clip.rect_filled(egui::Rect::from_min_size(egui::pos2(r.left(), r.top() + 2.0), egui::vec2(r.width(), 1.0)), 0.0, egui::Color32::from_gray(0x60));
                        clip.rect_filled(egui::Rect::from_min_size(egui::pos2(r.left(), r.top() + 3.0), egui::vec2(r.width(), 1.0)), 0.0, egui::Color32::from_gray(0xd0));
                    }
                    Kind::Slider { min, max, value } => {
                        let t = track(k);
                        let part = if max > min { ((value - min) / (max - min)).clamp(0.0, 1.0) as f32 } else { 0.0 };
                        let knob_rect = egui::Rect::from_min_size(egui::pos2(t.left() + part * (t.width() - knob.x).max(0.0), t.top()), egui::vec2(knob.x, t.height()));
                        let drawn = env.skin.is_some_and(|s| s.draw_in(&clip, super::controls::SLIDER_BACKGROUND, t, egui::Color32::WHITE) && s.draw_in(&clip, super::controls::SLIDER_KNOB, knob_rect, egui::Color32::WHITE));
                        if !drawn {
                            clip.rect_filled(egui::Rect::from_center_size(t.center(), egui::vec2(t.width(), 2.0)), 0.0, egui::Color32::from_gray(0x60));
                            clip.rect_filled(knob_rect, 0.0, egui::Color32::from_gray(0xd0));
                        }
                    }
                    Kind::Text => {
                        if highlighted == Some(k) {
                            clip.rect_filled(r, 0.0, egui::Color32::from_rgb(0x20, 0x20, 0xc0));
                        }
                        let p = super::host::Paint { painter: &clip, env, rect: r, hovered: false, pressed: false, enabled: e.enabled };
                        super::controls::text(&p, FontId::NORMAL, egui::pos2(r.left() + 1.0, r.top() + ((h - m.height(FontId::NORMAL)) * 0.5).floor()), &e.label, colour);
                        // (the tail: the sub-menu's arrow, or the check while it is ticked, at the column's right)
                        let picture = if e.sub.is_some() { Some((ARROW, arrow)) } else if e.mark == Some(true) { Some((CHECK, check)) } else { None };
                        if let Some((name, size)) = picture {
                            let spot = egui::Rect::from_min_size(egui::pos2(r.left() + labels + tail - size.x, r.top() + ((r.height() - size.y) * 0.5).floor()), size);
                            if !env.skin.is_some_and(|s| s.draw_in(&clip, name, spot, colour)) {
                                let c = spot.center();
                                let line = if name == ARROW { vec![c + egui::vec2(-2.0, -4.0), c + egui::vec2(2.0, 0.0), c + egui::vec2(-2.0, 4.0)] } else { vec![c + egui::vec2(-4.0, 0.0), c + egui::vec2(-1.0, 3.0), c + egui::vec2(4.0, -3.0)] };
                                clip.line(line, egui::Stroke::new(1.5, colour));
                            }
                        }
                    }
                }
            }
        });

        // The sub-menu: placed by its entry, and what it did.
        if let Some((entry, sub)) = self.sub.as_mut() {
            let leftwards = self.anchor.is_some_and(|a| rect.left() < a.left);
            sub.anchor = Some(Anchor { left: view.left(), right: view.left() + width + MARGIN, top: first.y + tops[*entry] - scroll, leftwards });
            let entry = *entry;
            match sub.level(ctx, env, id.with(("sub", entry)), depth + 1) {
                Step::Chosen(mut path) => {
                    path.insert(0, entry);
                    return Step::Chosen(path);
                }
                Step::Cancel => return Step::Cancel,
                Step::Back => self.sub = None,
                Step::Slider(mut path, value, done) => {
                    // (the value is the entry's from now on, for the next time its sub-menu opens)
                    if let Some(Kind::Slider { value: kept, .. }) = self.entries[entry].sub.as_mut().and_then(|s| s.get_mut(path[0])).map(|e| &mut e.kind) {
                        *kept = value;
                    }
                    path.insert(0, entry);
                    return Step::Slider(path, value, done);
                }
                Step::None => {}
            }
        }
        out
    }

    /// `SelectHighlightedItem`: the highlighted entry's sub-menu opens now, when it has one that is not open.
    fn open_sub_now(&mut self, frame: u64) {
        if self.highlighted.is_some_and(|k| self.entries[k].sub.is_some()) {
            self.open_sub(frame);
        }
    }

    /// The menu with the sub-menus of these entries open, one inside the other (for a picture of it).
    pub fn open_path(&mut self, path: &[usize], frame: u64) {
        let Some((&entry, rest)) = path.split_first() else { return };
        if entry >= self.entries.len() {
            return;
        }
        self.highlighted = Some(entry);
        self.open_sub(frame);
        if let Some((_, sub)) = self.sub.as_mut() {
            sub.open_path(rest, frame);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Entry> {
        vec![Entry::separator(), Entry::text("Look at", true), Entry::separator(), Entry::marked("/list mode", false, true), Entry::submenu("Looter", true, vec![Entry::text("Free for all", true)]), Entry::slider(0.0, 1.0, 0.5), Entry::separator()]
    }

    /// The entries stand 3 apart, each at its own height, and the whole is one short of the last gap.
    #[test]
    fn the_entries_stack_three_apart() {
        assert_eq!(stack(&[15.0, 5.0, 15.0]), (vec![0.0, 18.0, 26.0], 43.0));
        assert_eq!(stack(&[]), (vec![], 0.0));
    }

    /// Up and Down pass over separators and stop at the ends; disabled entries are stopped at like any other.
    #[test]
    fn the_keys_pass_over_separators() {
        let e = sample();
        assert_eq!((step(&e, None, true), step(&e, Some(1), true), step(&e, Some(3), true), step(&e, Some(5), true)), (Some(1), Some(3), Some(4), Some(5)));
        assert_eq!((step(&e, Some(5), false), step(&e, Some(3), false), step(&e, Some(1), false), step(&e, Some(0), false), step(&e, None, false)), (Some(4), Some(1), Some(1), Some(0), Some(0)));
        assert_eq!(step(&[], None, true), None);
        // (a page on, held inside the menu, then past a separator; back with a negative count)
        assert_eq!((page(&e, Some(1), 1), page(&e, Some(1), 3), page(&e, Some(1), 30), page(&e, Some(5), -4), page(&e, None, -3)), (Some(3), Some(4), Some(6), Some(1), Some(1)));
    }

    /// Typed text finds the first entry that begins with it, case and leading slashes aside, never a separator.
    #[test]
    fn typed_text_finds_an_entry() {
        let e = sample();
        assert_eq!((search_match(&e, "l"), search_match(&e, "LI"), search_match(&e, "/loo"), search_match(&e, "look at"), search_match(&e, "x")), (Some(1), Some(3), Some(1), Some(1), None));
        assert_eq!(search_match(&e, "loot"), Some(4));
    }

    /// A sub-menu stands 6 right of its parent's entries; turned left when it would leave the screen, and one whose
    /// parent already went left goes on leftwards, unless that leaves the screen.
    #[test]
    fn a_sub_menu_stands_beside_its_parent() {
        let screen = (0.0, 1000.0);
        assert_eq!(sub_left((100.0, 200.0), 150.0, screen, false), 206.0);
        assert_eq!(sub_left((800.0, 900.0), 150.0, screen, false), 800.0 - 6.0 - 150.0);
        assert_eq!(sub_left((800.0, 900.0), 150.0, screen, true), 644.0);
        assert_eq!(sub_left((50.0, 150.0), 150.0, screen, true), 156.0);
    }

    /// The highlight coming to an entry with a sub-menu starts the 300 ms; with one open, another entry the 200 ms;
    /// the open one's own entry none. A press opens it at once; a disabled entry is chosen as a cancel; one with a
    /// sub-menu is not chosen at all.
    #[test]
    fn a_sub_menu_opens_by_the_timer_or_at_once() {
        let mut menu = PopupMenu::new(sample(), egui::Pos2::ZERO, None, 0);
        let now = Instant::now();
        menu.highlight(1, now);
        assert_eq!(menu.sub_due, None);
        menu.highlight(4, now);
        assert_eq!(menu.sub_due, Some(now + OPEN_AFTER));
        assert!(matches!(menu.activate(), Step::None));
        menu.open_sub_now(1);
        assert!(menu.sub.as_ref().is_some_and(|s| s.0 == 4 && s.1.entries.len() == 1) && menu.sub_due.is_none());
        menu.highlight(1, now);
        assert_eq!(menu.sub_due, Some(now + CHANGE_AFTER));
        assert!(matches!(menu.activate(), Step::Chosen(path) if path == [1]));
        menu.highlight(4, now);
        assert_eq!(menu.sub_due, None);
        menu.highlight(3, now);
        assert!(matches!(menu.activate(), Step::Cancel));
        menu.open_sub(2);
        assert!(menu.sub.is_none());
        // (for a picture: the path's sub-menus open)
        let mut nested = PopupMenu::new(vec![Entry::submenu("A", true, vec![Entry::text("x", true), Entry::submenu("B", true, vec![Entry::text("y", true)])])], egui::Pos2::ZERO, None, 0);
        nested.open_path(&[0, 1], 0);
        assert!(nested.sub.as_ref().is_some_and(|s| s.1.sub.as_ref().is_some_and(|t| t.0 == 1)));
    }
}
