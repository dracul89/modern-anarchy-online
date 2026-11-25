//! The rollup area: GUI.dll's `RollupController_c` (FUN_100489f9) in the control center's `RollupControllerDock`, the
//! column of `RollupPage_c`s (FUN_10049a3c) down the right edge of the screen. A `DockableView` whose
//! `DockableViewDockName` is `RollupArea` lives there as a page. The original docks the wear, NCU, nano program, team
//! and faction windows there by default (the user's `Prefs.xml`: `WearViewConfig`, `NanoViewConfig` …
//! `DockableViewDockName="RollupArea"`).
//!
//! * **The column** is 191 wide in the views' units (FUN_100486cb: 192 pixels). Its pages stand one under the other
//!   from its top, touching (FUN_100488e5), and it scrolls (`ScrollBy`, the offset held between none and the pages'
//!   height past the column's). Behind them a black surface over the column at `cc_rollup_controller_fade_level`'s
//!   alpha (FUN_100487ff; 0 by default, the option panel's slider sets it).
//! * **A page** (FUN_10049a3c, laid out by FUN_10049797) is `GFX_GUI_TAB_BACKGROUND` over its whole bounds; its header
//!   (`PageHeaderView_c` FUN_1004a740) on black at the top: the menu button `GFX_GUI_WINDOW_ICON_I`, the title in font
//!   5 eight to its right, and at the right end the roll button (`GFX_GUI_ROLLUP_EXPAND/COLLAPSE`) and the close
//!   button, each 2 in; its view 3 under the header, the page's width and as tall as the page's `page_height`; and
//!   under the view a 5 high grip in 0x404040. Rolled up it is the header and one row (FUN_1004844c).
//! * **The page's own mouse** (FUN_10049e00, FUN_100484da, FUN_100484be; its view takes its own): a left press below
//!   the view's bottom takes the grip, and the page's height follows the pointer (FUN_100483ca: kept within the view's
//!   own smallest and largest); a left double click above it rolls the page up or down; a left press there held
//!   scrolls the column with the pointer; a right press takes the page up (FUN_100493f2: a `dockableview/view` drag with
//!   the title as its picture, put down by the next click, `docks.rs`).
//! * **Putting a page or a tab down on the column** (FUN_10049121): the view leaves its area and goes in before the
//!   page it is put on, else last; nothing happens when it is the column's only page. A page put down anywhere else
//!   goes out of the column (`docks.rs`).
//! * **The menu button** (FUN_10048c5c, FUN_10048de9): the view's own menu five under the button, its entries and
//!   "CloseView" (text.mdb 10000), which closes the view.
//! * Each page's height and rolled state are kept with its view (FUN_10048e92: `page_height`, `is_page_expanded`); a
//!   view coming in takes its frame's height (FUN_10049f4a).
//!
//! Here a window is a page while its name is on the list ([`docked`]); `chrome::window` draws such a window through
//! [`page`] instead of as a free window, so the window's code is the same either way. A window with a menu of its own
//! shows it with its own last entry ("Close Window"). The list, each page's rolled state and its view's height in
//! pixels are kept in the character's `rollup.ini` (`charprefs.rs`; `name 1|0 height` per line, in order; an older file's lines have no
//! height); without the file the original's own `DockAreas/RollupArea.xml` for the character gives them once the
//! character is known (`aoprefs::rollup_file`), else the original's default set is docked, all expanded. A window
//! coming in takes the room its
//! free window had (`layout::saved_size`), else a starting height of this client's ([`default_height`]). The column's
//! scroll is kept too (a `scroll_offset` line, FUN_10048e92 / FUN_100489f9), and with `MouseWheel` 1 the wheel over the
//! world scrolls it 40 a notch ([`wheel`]). Not done: the views' own smallest and largest heights and their grid steps
//! (a page's height is only kept above none here). A view is laid out in its page's rectangle and cut there (2026-11-21:
//! a view wanting more than the column's width widened its page, and one taller than its page ran over the next).

use std::sync::Mutex;

use crate::theme::Palette;

/// The windows the original docks in the rollup area by default (their `chrome::window` names), in the original's order.
pub const DEFAULT: &[&str] = &["Wear", "NCU", "Nano programs", "Team", "Faction"];

/// Whether a window can be a page of the column: one of the original's dockable views (`docks::dockable`; any
/// `DockableView_c` can be carried there). The pet window, which a button of this client's docked until 2026-11-25,
/// is a plain window in the original and no page.
pub fn may_dock(name: &str) -> bool {
    crate::docks::dockable(name)
}

/// The column's width: `RollupController_c`'s 191 (FUN_100486cb, 0x101b0d88), in pixels.
pub const WIDTH: f32 = 192.0;
/// The rows between the header and the view, and the grip under the view (`RollupPage_c` +0x154: `Rect(0, 3, 0, 5)`).
const ABOVE: f32 = 3.0;
const GRIP: f32 = 5.0;
/// The grip's colour (`+0x130`).
const GRIP_COLOUR: egui::Color32 = egui::Color32::from_rgb(0x40, 0x40, 0x40);

#[derive(Clone, Debug, PartialEq)]
struct Page {
    name: String,
    expanded: bool,
    /// The view's height in pixels (`page_height` plus one).
    height: f32,
    /// The view's width last time it was drawn (a view wider than the column widens its page).
    width: f32,
    /// The frame number the page was last drawn in (a window not shown takes no room).
    shown: u64,
}

/// The page's mouse while its left button is held (`RollupPage_c` +0x17c).
#[derive(Clone, Debug, PartialEq)]
enum Held {
    /// Scrolling the column: the pointer's height in the page at the press.
    Scroll(f32),
    /// Sizing the page by its grip: the pointer's height on screen and the view's height at the press.
    Size(f32, f32),
}

struct Rollup {
    pages: Vec<Page>,
    loaded: bool,
    frame: u64,
    /// How far the column is scrolled (down from its top, in pixels).
    scroll: f32,
    /// `cc_rollup_controller_fade_level`.
    fade: f32,
    /// The frame the column's black was last drawn in.
    backdrop: u64,
    /// The page whose mouse is held, and how.
    held: Option<(String, Held)>,
    /// Whether the pages are this client's own (`rollup.ini` was read) or still to be taken from the original's files.
    seeded: bool,
}

static STATE: Mutex<Rollup> = Mutex::new(Rollup { pages: Vec::new(), loaded: false, frame: 0, scroll: 0.0, fade: 0.0, backdrop: 0, held: None, seeded: false });

thread_local! {
    /// A page's own menu while it is up, and whose it is.
    static MENU: std::cell::RefCell<Option<(String, crate::gui::popup::PopupMenu)>> = const { std::cell::RefCell::new(None) };
}

fn path() -> std::path::PathBuf {
    crate::charprefs::file("rollup.ini")
}

fn disabled() -> bool {
    cfg!(test) || std::env::var_os("MAO_NO_LAYOUT").is_some() || std::env::var_os("MAO_UI_PREVIEW").is_some()
}

/// A line of `rollup.ini`: the window, whether its page is expanded, and its view's height when the line has it.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub name: String,
    pub expanded: bool,
    pub height: Option<f32>,
}

/// The lines of `rollup.ini`: `name 1|0 height` (the name may hold spaces; the height, a number past the flag, may be
/// missing).
pub fn parse(text: &str) -> Vec<Line> {
    text.lines()
        .filter(|line| !line.starts_with(SCROLL_KEY))
        .filter_map(|line| {
            let mut words: Vec<&str> = line.split_whitespace().collect();
            let mut flag = words.pop()?;
            let mut height = None;
            if !matches!(flag, "1" | "0") {
                height = Some(flag.parse::<f32>().ok().filter(|h| h.is_finite() && *h >= 0.0)?);
                flag = words.pop()?;
            }
            let expanded = match flag {
                "1" => true,
                "0" => false,
                _ => return None,
            };
            (!words.is_empty()).then(|| Line { name: words.join(" "), expanded, height })
        })
        .collect()
}

/// The line of `rollup.ini` that keeps the column's scroll (the controller's `scroll_offset`, saved by FUN_10048e92 and
/// read back by FUN_100489f9).
const SCROLL_KEY: &str = "scroll_offset ";

/// The scroll `rollup.ini` keeps, if it has one.
pub fn parse_scroll(text: &str) -> Option<f32> {
    text.lines().find_map(|l| l.strip_prefix(SCROLL_KEY)).and_then(|v| v.trim().parse::<f32>().ok()).filter(|v| v.is_finite())
}

/// The scroll's line (always with a decimal, so that no older reader takes it for a page's).
pub fn render_scroll(scroll: f32) -> String {
    format!("{SCROLL_KEY}{:.1}\n", scroll)
}

pub fn render(lines: &[Line]) -> String {
    lines.iter().map(|l| format!("{} {}{}\n", l.name, l.expanded as u8, l.height.map_or(String::new(), |h| format!(" {}", h.round())))).collect()
}

fn load(state: &mut Rollup) {
    if state.loaded {
        return;
    }
    state.loaded = true;
    let text = if disabled() { None } else { std::fs::read_to_string(path()).ok() };
    // (the column's scroll as it was left, FUN_100489f9's `scroll_offset`)
    state.scroll = text.as_deref().and_then(parse_scroll).unwrap_or(0.0).max(0.0);
    let list = text.map(|t| parse(&t));
    state.seeded = disabled() || list.is_some();
    let mut list = list.unwrap_or_else(|| DEFAULT.iter().map(|n| Line { name: n.to_string(), expanded: true, height: None }).collect());
    // (a page kept from when this client docked windows that are no dockable views: a free window again)
    list.retain(|l| may_dock(&l.name));
    state.pages = list.into_iter().map(|l| Page { height: l.height.unwrap_or_else(|| default_height(&l.name)), name: l.name, expanded: l.expanded, width: WIDTH, shown: 0 }).collect();
}

fn save(state: &Rollup) {
    if disabled() {
        return;
    }
    let list: Vec<Line> = state.pages.iter().map(|p| Line { name: p.name.clone(), expanded: p.expanded, height: Some(p.height) }).collect();
    let _ = std::fs::write(path(), render(&list) + &render_scroll(state.scroll));
}

/// Whether the window is a page of the rollup area now.
pub fn docked(name: &str) -> bool {
    let mut state = STATE.lock().unwrap();
    load(&mut state);
    state.pages.iter().any(|p| p.name == name)
}

/// The height a window coming into the column takes (FUN_10049f4a: its frame's): the room its free window had (its
/// kept size less the frame's 24 above and 10 below), else this client's starting height for it.
fn coming_height(name: &str) -> f32 {
    crate::layout::saved_size(name).map_or_else(|| default_height(name), |s| (s.y - 34.0).max(0.0))
}

/// Makes a page a free window again.
pub fn undock(name: &str) {
    let mut state = STATE.lock().unwrap();
    load(&mut state);
    if state.pages.iter().any(|p| p.name == name) {
        state.pages.retain(|p| p.name != name);
        save(&state);
    }
}

/// How wide the column is on screen now: the widest page drawn lately; 0 without pages.
pub fn column_width() -> f32 {
    let state = STATE.lock().unwrap();
    state.pages.iter().filter(|p| live(&state, p)).map(|p| p.width.max(WIDTH)).fold(0.0, f32::max)
}

/// A page's starting height when nothing gives one, its view's: this client's guesses at the original's pages (the
/// wear view's three-by-five grid, the NCU's rows, the nano programs' school rows over a grid).
fn default_height(name: &str) -> f32 {
    match name {
        "Wear" => 318.0,
        "NCU" => 98.0,
        "Nano programs" => 178.0,
        "Team" => 118.0,
        "Faction" => 158.0,
        "Friends" => 203.0,
        _ => 138.0,
    }
}

/// The docked windows in the column's order (for the tests and the layout file).
pub fn names() -> Vec<String> {
    let mut state = STATE.lock().unwrap();
    load(&mut state);
    state.pages.iter().map(|p| p.name.clone()).collect()
}

/// The pages of the original's rollup area file (`aoprefs::rollup_pages`) that are windows here, each its view's
/// `page_height` (plus one: pixels) high, or this client's starting height without one; none when no view is known.
fn seed_pages(file: &[(String, bool, f32)]) -> Option<Vec<Page>> {
    let pages: Vec<Page> = file
        .iter()
        .filter_map(|(view, expanded, height)| {
            let name = crate::docks::client_name(view)?;
            let height = if *height >= 0.0 { height + 1.0 } else { default_height(&name) };
            Some(Page { name, expanded: *expanded, height, width: WIDTH, shown: 0 })
        })
        .collect();
    (!pages.is_empty()).then_some(pages)
}

/// A new frame: the pages drawn from now on belong to it. Without `rollup.ini`, once the character is known, the
/// pages are taken from the original's file for it, when it has one.
pub fn begin_frame() {
    let mut guard = STATE.lock().unwrap();
    let state = &mut *guard;
    state.frame += 1;
    load(state);
    if !state.seeded
        && let Some(character) = crate::layout::character()
    {
        state.seeded = true;
        if let Some(pages) = crate::aoprefs::rollup_file(character).and_then(|file| seed_pages(&file)) {
            state.pages = pages;
        }
        // (and its scroll, the file's `scroll_offset`)
        if let Some(scroll) = crate::aoprefs::rollup_file_scroll(character) {
            state.scroll = scroll.max(0.0);
        }
        save(state);
    }
}

/// `cc_rollup_controller_fade_level`, the alpha of the black behind the column (FUN_100487ff).
pub fn set_fade(level: f32) {
    STATE.lock().unwrap().fade = level.clamp(0.0, 1.0);
}

/// A page's height on screen (FUN_1004844c, in pixels): its header and the row under it, or, expanded, the header,
/// the 3 rows above its view, the view and the grip.
pub fn page_pixels(header: f32, expanded: bool, view: f32) -> f32 {
    if expanded { header + ABOVE + view + GRIP } else { header + 1.0 }
}

/// Whether a page counts in the column now: drawn this frame or the last.
fn live(state: &Rollup, p: &Page) -> bool {
    p.shown + 1 >= state.frame
}

/// The pages' places down the column (FUN_100488e5): each live page's name, top and height, touching, from `top`.
fn places(state: &Rollup, top: f32, header: f32) -> Vec<(String, f32, f32)> {
    let mut y = top;
    let mut out = Vec::new();
    for p in state.pages.iter().filter(|p| live(state, p)) {
        let h = page_pixels(header, p.expanded, p.height);
        out.push((p.name.clone(), y, h));
        y += h;
    }
    out
}

/// The wheel over the world (no window under the pointer) with `MouseWheel` 1: the controller connects its `WheelMoved`
/// to the global wheel signal (GlobalSignals +0x21c, FUN_10048822), which scrolls the column 40 a notch
/// (FUN_100487aa: `ScrollBy(0, wheel * 40)`, the wheel positive downwards); `lines` are notches, up positive. The scroll
/// is held to the pages when they are next laid out ([`clamp_scroll`]).
pub fn wheel(lines: f32) {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    state.scroll = wheel_scroll(state.scroll, lines).max(0.0);
    save(&state);
}

/// The scroll a wheel of `lines` notches (up positive) leaves: 40 a notch, up towards the top.
fn wheel_scroll(scroll: f32, lines: f32) -> f32 {
    scroll - lines * 40.0
}

/// The column's scroll held between none and the pages' height past the column's (`View::ScrollBy` 0x1014d269).
pub fn clamp_scroll(scroll: f32, content: f32, room: f32) -> f32 {
    scroll.min((content - room).max(0.0)).max(0.0)
}

/// The header's height: its tallest child with the 2 above and below each (`PageHeaderView_c`'s `HLayoutNode`): the
/// menu button, the title's line, the roll button, the close button.
fn header_height(ctx: &egui::Context) -> f32 {
    let skin = crate::skin::skin(ctx);
    let picture = |name: &str| skin.as_ref().and_then(|s| s.get(name)).map_or(14.0, |s| s.size.y);
    let text = crate::gui::font::Measure { ctx: Some(ctx.clone()) }.height(crate::gui::font::FontId::NORMAL);
    [picture("GFX_GUI_WINDOW_ICON_I"), text, picture("GFX_GUI_ROLLUP_EXPAND_STATE1"), picture("GFX_GUI_WINDOW_CLOSE_X")].into_iter().fold(0.0, f32::max) + 4.0
}

/// The column on screen: the right edge's strip, `WIDTH` wide.
fn column(ctx: &egui::Context) -> egui::Rect {
    let screen = ctx.content_rect();
    egui::Rect::from_min_max(egui::pos2(screen.right() - WIDTH, screen.top()), screen.right_bottom())
}

/// Where a carried view would be put down at `at`: `Some` over the column (which takes a `dockableview/view` drop
/// there, FUN_1004862a), with the page under the pointer, if any.
pub fn column_target(ctx: &egui::Context, at: egui::Pos2) -> Option<Option<String>> {
    let column = column(ctx);
    if !column.contains(at) {
        return None;
    }
    let header = header_height(ctx);
    let mut state = STATE.lock().unwrap();
    load(&mut state);
    let on = places(&state, column.top() - state.scroll, header).into_iter().find(|(_, y, h)| at.y >= *y && at.y < y + h).map(|(n, _, _)| n);
    Some(on)
}

/// A view put down on the column (FUN_10049121) in a list of pages: out of the list and in before the page it was put
/// on, else last; refused when it is the only page the column shows. A view new to the column comes in expanded,
/// `height` high.
fn put_in(pages: &mut Vec<Page>, live: &dyn Fn(&Page) -> bool, view: &str, on: Option<&str>, height: f32) -> bool {
    let here = pages.iter().position(|p| p.name == view);
    if here.is_some_and(|k| live(&pages[k])) && pages.iter().filter(|p| live(p)).count() == 1 {
        return false;
    }
    let page = match here {
        Some(k) => pages.remove(k),
        None => Page { name: view.to_string(), expanded: true, height, width: WIDTH, shown: 0 },
    };
    let index = on.and_then(|o| pages.iter().position(|p| p.name == o)).unwrap_or(pages.len());
    pages.insert(index, page);
    true
}

/// A carried view put down on the column, on the page `on` or past the pages; whether it went in. One coming in takes
/// its free window's room as its height.
pub fn put(view: &str, on: Option<&str>) -> bool {
    let height = coming_height(view);
    let mut guard = STATE.lock().unwrap();
    let state = &mut *guard;
    load(state);
    let frame = state.frame;
    let done = put_in(&mut state.pages, &|p: &Page| p.shown + 1 >= frame, view, on, height);
    if done {
        save(state);
    }
    done
}

/// What a press on the page itself did (FUN_10049e00).
#[derive(Default)]
struct Pressed {
    held: Option<Held>,
    toggle: bool,
    carry: Option<egui::Vec2>,
}

/// A press on the page itself (FUN_10049e00): the right button takes the page up (where it was taken hold of, from the
/// page's corner); the left one below the view's bottom takes the grip, above it rolls the page on a double click,
/// else holds the column to scroll it.
fn press(ctx: &egui::Context, response: &egui::Response, bounds: egui::Rect, view_bottom: f32, height: f32, out: &mut Pressed) {
    let Some(at) = ctx.input(|i| i.pointer.interact_pos()).filter(|_| response.hovered()) else { return };
    let (left, right, double) = ctx.input(|i| (i.pointer.button_pressed(egui::PointerButton::Primary), i.pointer.button_pressed(egui::PointerButton::Secondary), i.pointer.button_double_clicked(egui::PointerButton::Primary)));
    if right {
        out.carry = Some(at - bounds.min);
    } else if at.y > view_bottom {
        if left {
            out.held = Some(Held::Size(at.y, height));
        }
    } else if double {
        out.toggle = true;
    } else if left {
        out.held = Some(Held::Scroll(at.y - bounds.top()));
    }
}

/// Draws a docked window as a page of the column: the header with the title and its buttons, the view under it
/// while expanded, the grip under that. The result has the shape `chrome::window` returns.
pub fn page<R>(ctx: &egui::Context, palette: &Palette, name: &str, title: &str, open: &mut bool, add: impl FnOnce(&mut egui::Ui) -> R) -> Option<egui::InnerResponse<Option<R>>> {
    let header = header_height(ctx);
    let column = column(ctx);
    let (top, expanded, height, width, fade, backdrop) = {
        let mut guard = STATE.lock().unwrap();
        let state = &mut *guard;
        load(state);
        let frame = state.frame;
        // (counted from this frame: a page drawn for the first time takes its place at once)
        if let Some(p) = state.pages.iter_mut().find(|p| p.name == name) {
            p.shown = frame;
        }
        let content: f32 = places(state, 0.0, header).iter().map(|p| p.2).sum();
        state.scroll = clamp_scroll(state.scroll, content, column.height());
        let top = places(state, column.top() - state.scroll, header).into_iter().find(|p| p.0 == name).map_or(column.top(), |p| p.1);
        let page = state.pages.iter().find(|p| p.name == name);
        let backdrop = state.backdrop != frame;
        state.backdrop = frame;
        (top, page.is_none_or(|p| p.expanded), page.map_or(default_height(name), |p| p.height), page.map_or(WIDTH, |p| p.width.max(WIDTH)), state.fade, backdrop)
    };
    // (the column's black, `cc_rollup_controller_fade_level`, under everything)
    if backdrop && fade > 0.0 {
        ctx.layer_painter(egui::LayerId::background()).rect_filled(column, 0.0, egui::Color32::from_black_alpha((fade * 255.0).round() as u8));
    }
    let left = column.right() - width;
    let bounds = egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, page_pixels(header, expanded, height)));
    let view_bottom = bounds.bottom() - GRIP;
    let skin = crate::skin::skin(ctx);
    let accent = crate::chrome::rgb(palette.accent);
    let hover = crate::chrome::rgb(palette.hover);
    let panel = egui::Color32::from_rgba_unmultiplied(palette.panel[0], palette.panel[1], palette.panel[2], 225);
    // (the page's `GFX_GUI_TAB_BACKGROUND` over its bounds: the part of it under one of the page's parts)
    let background = |piece: egui::Rect| -> egui::Shape {
        match skin.as_ref().and_then(|s| s.get("GFX_GUI_TAB_BACKGROUND")) {
            Some(s) => {
                let uv = |p: egui::Pos2| egui::pos2((p.x - bounds.left()) / bounds.width().max(1.0), (p.y - bounds.top()) / bounds.height().max(1.0));
                let mut mesh = egui::Mesh::with_texture(s.texture.id());
                mesh.add_rect_with_uv(piece, egui::Rect::from_min_max(uv(piece.min), uv(piece.max)), egui::Color32::WHITE);
                egui::Shape::mesh(mesh)
            }
            None => egui::Shape::rect_filled(piece, 0.0, panel),
        }
    };
    let id = egui::Id::unique(("rollup", name));
    let mut pressed = Pressed::default();
    let (mut close, mut menu_at) = (false, None);
    // The header and the rows under it: their own area.
    let head = egui::Rect::from_min_max(bounds.min, egui::pos2(bounds.right(), if expanded { bounds.top() + header + ABOVE } else { bounds.bottom() }));
    egui::Area::new(id).order(egui::Order::Middle).fixed_pos(head.min).constrain(false).show(ctx, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let (_, response) = ui.allocate_exact_size(head.size(), egui::Sense::click_and_drag());
        let painter = ui.painter().clone();
        painter.add(background(head));
        let strip = egui::Rect::from_min_size(head.min, egui::vec2(width, header));
        painter.rect_filled(strip, 0.0, egui::Color32::BLACK);
        // (the menu button 2 in at the left, the close button 2 in at the right, the roll button 2 left of it)
        let size_of = |n: &str| skin.as_ref().and_then(|s| s.get(n)).map_or(egui::vec2(14.0, 14.0), |s| s.size);
        let icon = egui::Rect::from_min_size(strip.min + egui::vec2(2.0, 2.0), size_of("GFX_GUI_WINDOW_ICON_I"));
        let close_size = size_of("GFX_GUI_WINDOW_CLOSE_X");
        let close_rect = egui::Rect::from_min_size(egui::pos2(strip.right() - 2.0 - close_size.x, strip.top() + 2.0), close_size);
        let roll_size = size_of("GFX_GUI_ROLLUP_EXPAND_STATE1");
        let roll = egui::Rect::from_min_size(egui::pos2(close_rect.left() - 2.0 - roll_size.x, strip.top() + 2.0), roll_size);
        let mut on_button = false;
        for (what, rect) in [("menu", icon), ("roll", roll), ("close", close_rect)] {
            // (`PageHeaderView_c`'s three buttons are toggles, FUN_1004a740: they act as the button goes down)
            let r = crate::chrome::on_press(ui.interact(rect, id.with(what), egui::Sense::click()));
            on_button |= r.hovered();
            let down = r.is_pointer_button_down_on();
            let sprite = match what {
                "menu" => "GFX_GUI_WINDOW_ICON_I".to_string(),
                "roll" if down => "GFX_GUI_ROLLUP_EXPAND_STATE3".to_string(),
                "roll" => format!("GFX_GUI_ROLLUP_{}_STATE1", if expanded { "COLLAPSE" } else { "EXPAND" }),
                _ if down => "GFX_GUI_WINDOW_CLOSE_X_STATE3".to_string(),
                _ if r.hovered() => "GFX_GUI_WINDOW_CLOSE_X_STATE2".to_string(),
                _ => "GFX_GUI_WINDOW_CLOSE_X".to_string(),
            };
            let tint = if r.hovered() { hover } else { accent };
            if !skin.as_deref().is_some_and(|s| s.draw_in(&painter, &sprite, rect, tint)) {
                let glyph = match what {
                    "menu" => "i",
                    "roll" if expanded => "-",
                    "roll" => "+",
                    _ => "x",
                };
                painter.text(rect.center(), egui::Align2::CENTER_CENTER, glyph, egui::FontId::monospace(12.0), tint);
            }
            if r.clicked() {
                match what {
                    "menu" => menu_at = Some(egui::pos2(rect.left(), rect.bottom() + 5.0)),
                    "roll" => pressed.toggle = true,
                    _ => close = true,
                }
            }
        }
        // (the title, font 5, 8 right of the menu button and 2 down, cut where the roll button begins)
        let font = crate::gui::font::Measure { ctx: Some(ctx.clone()) }.egui_font(crate::gui::font::FontId::NORMAL);
        let room = egui::Rect::from_min_max(egui::pos2(icon.right() + 8.0, strip.top()), egui::pos2(roll.left(), strip.bottom()));
        painter.with_clip_rect(room.intersect(painter.clip_rect())).text(egui::pos2(room.left(), strip.top() + 2.0), egui::Align2::LEFT_TOP, title, font, crate::chrome::rgb(palette.text));
        if !on_button {
            press(ctx, &response, bounds, view_bottom, height, &mut pressed);
        }
    });
    // The view: an egui window with no title bar at a fixed place under the header, its room the page's view.
    let shown = if expanded {
        let view = egui::Rect::from_min_size(egui::pos2(left, bounds.top() + header + ABOVE), egui::vec2(width, height));
        // (the view is laid out in its page's rectangle, the column's width by the page's height, and cut there, as the
        // original's page frames its view (FUN_10049797): a view wanting more width no longer widens its page, nor does
        // one taller than its page run over the next; the grip gives it more height)
        let shown = egui::Window::new(title).id(egui::Id::unique(("rollup-body", name))).title_bar(false).collapsible(false).resizable(false).movable(false).fixed_pos(view.min).fixed_size(view.size()).constrain(false).frame(egui::Frame::NONE).show(ctx, |ui| {
            ui.set_clip_rect(view.intersect(ui.clip_rect()));
            ui.set_max_width(width);
            let under = ui.painter().add(egui::Shape::Noop);
            let result = add(ui);
            ui.painter().set(under, background(view));
            result
        });
        // The grip under the view: its own area, which sizes the page.
        let grip = egui::Rect::from_min_max(egui::pos2(left, view_bottom), bounds.right_bottom());
        egui::Area::new(id.with("grip")).order(egui::Order::Middle).fixed_pos(grip.min).constrain(false).show(ctx, |ui| {
            let (_, response) = ui.allocate_exact_size(grip.size(), egui::Sense::click_and_drag());
            ui.painter().rect_filled(grip, 0.0, GRIP_COLOUR);
            press(ctx, &response, bounds, view_bottom, height, &mut pressed);
            if response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
        });
        if let Some(p) = STATE.lock().unwrap().pages.iter_mut().find(|p| p.name == name) {
            p.width = WIDTH;
        }
        shown
    } else {
        None
    };
    // (the left button held: the column follows the pointer, or the page's height does; let go, it ends, FUN_100484be)
    {
        let pointer = ctx.input(|i| i.pointer.interact_pos());
        let down = ctx.input(|i| i.pointer.primary_down());
        let mut guard = STATE.lock().unwrap();
        let state = &mut *guard;
        if let Some(h) = pressed.held.take() {
            state.held = Some((name.to_string(), h));
        }
        let mut save_now = pressed.toggle;
        if let Some((_, held)) = state.held.clone().filter(|(n, _)| n == name) {
            match (held, pointer) {
                (Held::Scroll(start), Some(at)) => {
                    state.scroll -= (at.y - bounds.top()) - start;
                    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                    // (kept when let go, as the controller keeps `scroll_offset`)
                    save_now |= !down;
                }
                (Held::Size(start, from), Some(at)) => {
                    if let Some(p) = state.pages.iter_mut().find(|p| p.name == name) {
                        p.height = (from + (at.y - start)).max(0.0);
                    }
                    ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    save_now |= !down;
                }
                _ => {}
            }
            if !down {
                state.held = None;
            }
        }
        if pressed.toggle
            && let Some(p) = state.pages.iter_mut().find(|p| p.name == name)
        {
            p.expanded = !p.expanded;
        }
        if save_now {
            save(state);
        }
    }
    // (the menu button: the view's own menu, when it has one, else the page's, "CloseView")
    if let Some(at) = menu_at {
        crate::chrome::ask_frame_menu(name, at, ctx.cumulative_frame_nr());
    }
    if let Some(at) = crate::chrome::frame_menu_left(name, ctx.cumulative_frame_nr()) {
        let entries = [(crate::gui::xml::ldb_named(10000, "CloseView", "Close"), true)];
        MENU.with(|m| *m.borrow_mut() = Some((name.to_string(), crate::chrome::menu_at(ctx, at, &entries))));
    }
    let mut own = MENU.with(|m| m.borrow_mut().take_if(|(n, _)| n == name)).map(|(_, popup)| popup);
    if own.is_some() {
        if crate::chrome::menu(ctx, palette, "rollup-page-menu", &mut own) == Some(0) {
            close = true;
        }
        if let Some(popup) = own {
            MENU.with(|m| *m.borrow_mut() = Some((name.to_string(), popup)));
        }
    }
    if let Some(grab) = pressed.carry {
        crate::docks::carry(name, title, grab);
    }
    if close {
        *open = false;
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(name: &str, expanded: bool, height: f32, shown: u64) -> Page {
        Page { name: name.into(), expanded, height, width: WIDTH, shown }
    }

    #[test]
    fn the_file_round_trips() {
        let list = vec![Line { name: "Wear".into(), expanded: true, height: Some(300.0) }, Line { name: "Nano programs".into(), expanded: false, height: Some(120.0) }];
        let text = render(&list);
        assert_eq!(text, "Wear 1 300\nNano programs 0 120\n");
        assert_eq!(parse(&text), list);
        // (an older file's lines have no height)
        assert_eq!(parse("Nano programs 1\n"), [Line { name: "Nano programs".into(), expanded: true, height: None }]);
        assert!(parse("Wear\nNCU yes\n 1\nTeam 1 tall\n").is_empty());
    }

    /// A page is its header and a row rolled up; expanded, the header, 3 rows, its view and the 5 of the grip.
    #[test]
    fn a_page_is_as_tall_as_its_parts() {
        assert_eq!(page_pixels(18.0, false, 200.0), 19.0);
        assert_eq!(page_pixels(18.0, true, 200.0), 18.0 + 3.0 + 200.0 + 5.0);
    }

    /// The pages touch, from the column's top; a page not drawn lately takes no room.
    #[test]
    fn the_pages_stack_in_order() {
        let mut state = Rollup { pages: Vec::new(), loaded: true, frame: 5, scroll: 0.0, fade: 0.0, backdrop: 0, held: None, seeded: true };
        state.pages.push(page("Wear", true, 100.0, 5));
        state.pages.push(page("NCU", false, 60.0, 4));
        state.pages.push(page("Team", true, 50.0, 1));
        state.pages.push(page("Faction", true, 40.0, 5));
        let at = places(&state, 10.0, 18.0);
        assert_eq!(at.iter().map(|p| (p.0.as_str(), p.1)).collect::<Vec<_>>(), [("Wear", 10.0), ("NCU", 10.0 + 126.0), ("Faction", 10.0 + 126.0 + 19.0)]);
    }

    /// The original's file gives the pages its views are here, their rolled state and their heights (one more pixel
    /// than the original's sizes); a file of none known gives none.
    #[test]
    fn the_originals_pages_are_taken() {
        let file = vec![("ncu_window".to_string(), true, 103.0), ("tip_window".to_string(), true, 50.0), ("nano_window".to_string(), false, -1.0)];
        assert_eq!(seed_pages(&file), Some(vec![page("NCU", true, 104.0, 0), page("Nano programs", false, default_height("Nano programs"), 0)]));
        assert_eq!(seed_pages(&[("tip_window".to_string(), true, 50.0)]), None);
    }

    /// The scroll stays between none and what the pages have past the column.
    #[test]
    fn the_column_scrolls_within_its_pages() {
        // (FUN_100487aa: 40 a notch, a notch up towards the top)
        assert_eq!(wheel_scroll(100.0, 1.0), 60.0);
        assert_eq!(clamp_scroll(wheel_scroll(20.0, 1.0), 400.0, 300.0), 0.0);
        assert_eq!(wheel_scroll(0.0, -2.0), 80.0);
        // (the scroll's line is kept beside the pages' and no page is made of it)
        let text = render(&[Line { name: "Wear".into(), expanded: true, height: Some(330.0) }]) + &render_scroll(1.0);
        assert_eq!(parse(&text), vec![Line { name: "Wear".into(), expanded: true, height: Some(330.0) }]);
        assert_eq!(parse_scroll(&text), Some(1.0));
        assert_eq!(parse_scroll("Wear 1 330\n"), None);
        assert_eq!(clamp_scroll(50.0, 400.0, 300.0), 50.0);
        assert_eq!(clamp_scroll(150.0, 400.0, 300.0), 100.0);
        assert_eq!(clamp_scroll(-20.0, 400.0, 300.0), 0.0);
        assert_eq!(clamp_scroll(30.0, 200.0, 300.0), 0.0);
    }

    /// Put down on the column: in before the page it is put on, else last; the column's only page stays; a new view
    /// comes in expanded at the height given; a page keeps its height and rolled state as it moves.
    #[test]
    fn a_page_goes_in_before_the_page_it_is_put_on() {
        let all = |_: &Page| true;
        let order = |pages: &[Page]| pages.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
        let mut pages = vec![page("Wear", true, 100.0, 5), page("NCU", false, 60.0, 5), page("Team", true, 50.0, 5)];
        assert!(put_in(&mut pages, &all, "Team", Some("Wear"), 0.0));
        assert_eq!(order(&pages), ["Team", "Wear", "NCU"]);
        assert!(put_in(&mut pages, &all, "Team", None, 0.0));
        assert_eq!(order(&pages), ["Wear", "NCU", "Team"]);
        // (on its own place: out of the list first, so last)
        assert!(put_in(&mut pages, &all, "Wear", Some("Wear"), 0.0));
        assert_eq!(order(&pages), ["NCU", "Team", "Wear"]);
        assert_eq!(pages[0], page("NCU", false, 60.0, 5));
        assert!(put_in(&mut pages, &all, "Friends", Some("Team"), 150.0));
        assert_eq!(pages[1], page("Friends", true, 150.0, 0));
        let mut lone = vec![page("Wear", true, 100.0, 5), page("NCU", true, 100.0, 1)];
        assert!(!put_in(&mut lone, &|p: &Page| p.shown == 5, "Wear", None, 0.0));
    }
}
