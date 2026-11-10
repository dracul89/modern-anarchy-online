//! The rollup area: GUI.dll's `RollupController_c` in the control center's `RollupControllerDock`, the column of
//! `RollupPage_c`s down the right edge of the screen. A `DockableView` whose `DockableViewDockName` is `RollupArea` lives
//! there as a page: a header strip with the window's title and the `GFX_GUI_ROLLUP_COLLAPSE/EXPAND` button, and under it the
//! window's own body while the page is expanded. The original docks the wear, NCU, nano program, team and faction windows
//! there by default (the user's `Prefs.xml`: `WearViewConfig`, `NanoViewConfig` … `DockableViewDockName="RollupArea"`), and
//! the player drags a page out to make it a window again.
//!
//! Here a window is a page while its name is on the list ([`docked`]); `chrome::window` draws such a window through
//! [`page`] instead of as a free window, so the window's code is the same either way. The pages stack from the top of the
//! screen in the list's order, each as tall as its body (last frame's height places the next); the header's undock button
//! makes the window free again and the title-bar dock button of a free [`DOCKABLE`] window puts it back at the end. The
//! list and each page's expanded state are kept in `rollup.ini` beside `mao.ini` (`name 1|0` per line, in order); without
//! the file the original's default set is docked, all expanded.

use std::sync::Mutex;

use crate::theme::Palette;

/// The windows the original docks in the rollup area by default (their `chrome::window` names), in the original's order.
pub const DEFAULT: &[&str] = &["Wear", "NCU", "Nano programs", "Team", "Faction"];

/// The windows that may be docked (the dock button shows in their title bar).
pub const DOCKABLE: &[&str] = &["Wear", "NCU", "Nano programs", "Team", "Faction", "Friends", "PetWindow", "Stats", "Missions", "MissionView"];

/// The column's width (the original's docked containers are 175 wide) and the header's height.
pub const WIDTH: f32 = 196.0;
pub const HEADER: f32 = 18.0;
const GAP: f32 = 2.0;

struct Page {
    name: String,
    expanded: bool,
    /// The body's height and the page's width last time it was drawn (a body wider than the column widens its page, which
    /// keeps its right edge at the screen's).
    height: f32,
    width: f32,
    /// The frame number the page was last drawn in (a window not shown takes no room).
    shown: u64,
}

struct Rollup {
    pages: Vec<Page>,
    loaded: bool,
    frame: u64,
}

static STATE: Mutex<Rollup> = Mutex::new(Rollup { pages: Vec::new(), loaded: false, frame: 0 });

fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("rollup.ini")
}

fn disabled() -> bool {
    cfg!(test) || std::env::var_os("MAO_NO_LAYOUT").is_some() || std::env::var_os("MAO_UI_PREVIEW").is_some()
}

/// The lines of `rollup.ini`: `name 1|0` (the name may hold spaces; the last word is the expanded flag).
pub fn parse(text: &str) -> Vec<(String, bool)> {
    text.lines()
        .filter_map(|line| {
            let mut words: Vec<&str> = line.split_whitespace().collect();
            let flag = words.pop()?;
            let expanded = match flag {
                "1" => true,
                "0" => false,
                _ => return None,
            };
            (!words.is_empty()).then(|| (words.join(" "), expanded))
        })
        .collect()
}

pub fn render(pages: &[(String, bool)]) -> String {
    pages.iter().map(|(name, expanded)| format!("{name} {}\n", *expanded as u8)).collect()
}

fn load(state: &mut Rollup) {
    if state.loaded {
        return;
    }
    state.loaded = true;
    let list = if disabled() { None } else { std::fs::read_to_string(path()).ok().map(|t| parse(&t)) };
    let list = list.unwrap_or_else(|| DEFAULT.iter().map(|n| (n.to_string(), true)).collect());
    state.pages = list.into_iter().map(|(name, expanded)| Page { name, expanded, height: 0.0, width: WIDTH, shown: 0 }).collect();
}

fn save(state: &Rollup) {
    if disabled() {
        return;
    }
    let list: Vec<(String, bool)> = state.pages.iter().map(|p| (p.name.clone(), p.expanded)).collect();
    let _ = std::fs::write(path(), render(&list));
}

/// Whether the window is a page of the rollup area now.
pub fn docked(name: &str) -> bool {
    let mut state = STATE.lock().unwrap();
    load(&mut state);
    state.pages.iter().any(|p| p.name == name)
}

/// Puts a window at the end of the column.
pub fn dock(name: &str) {
    let mut state = STATE.lock().unwrap();
    load(&mut state);
    if !state.pages.iter().any(|p| p.name == name) {
        state.pages.push(Page { name: name.to_string(), expanded: true, height: 0.0, width: WIDTH, shown: 0 });
        save(&state);
    }
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

/// How wide the column is on screen now: the widest page drawn lately, with its margin; 0 without pages.
pub fn column_width() -> f32 {
    let state = STATE.lock().unwrap();
    state.pages.iter().filter(|p| p.shown + 1 >= state.frame).map(|p| p.width.max(WIDTH) + 2.0).fold(0.0, f32::max)
}

/// A page's starting height: what the window's body is given when it takes the room it has (the original's pages: the wear
/// view's three-by-five grid, the NCU's rows, the nano programs' school rows over a grid).
fn default_height(name: &str) -> f32 {
    match name {
        "Wear" => 330.0,
        "NCU" => 110.0,
        "Nano programs" => 190.0,
        "Team" => 130.0,
        "Faction" => 170.0,
        "Friends" => 215.0,
        _ => 150.0,
    }
}

/// The docked windows in the column's order (for the tests and the layout file).
pub fn names() -> Vec<String> {
    let mut state = STATE.lock().unwrap();
    load(&mut state);
    state.pages.iter().map(|p| p.name.clone()).collect()
}

/// A new frame: the pages drawn from now on belong to it.
pub fn begin_frame() {
    let mut state = STATE.lock().unwrap();
    state.frame += 1;
}

/// Where a page's top is: under every page before it in the list that was drawn this frame or the last.
fn top_of(state: &Rollup, name: &str, screen_top: f32) -> f32 {
    let mut y = screen_top;
    for p in &state.pages {
        if p.name == name {
            break;
        }
        if p.shown + 1 >= state.frame {
            y += HEADER + if p.expanded { p.height } else { 0.0 } + GAP;
        }
    }
    y
}

/// Draws a docked window as a page of the column: the header with the title and its buttons, the body under it while
/// expanded. The result has the shape `chrome::window` returns.
pub fn page<R>(ctx: &egui::Context, palette: &Palette, name: &str, title: &str, open: &mut bool, add: impl FnOnce(&mut egui::Ui) -> R) -> Option<egui::InnerResponse<Option<R>>> {
    let (top, expanded, width) = {
        let mut state = STATE.lock().unwrap();
        load(&mut state);
        let screen_top = ctx.content_rect().top();
        let top = top_of(&state, name, screen_top);
        let page = state.pages.iter().find(|p| p.name == name);
        (top, page.map_or(true, |p| p.expanded), page.map_or(WIDTH, |p| p.width.max(WIDTH)))
    };
    let screen = ctx.content_rect();
    let left = screen.right() - width - 2.0;
    let accent = crate::chrome::rgb(palette.accent);
    let hover = crate::chrome::rgb(palette.hover);
    let panel = egui::Color32::from_rgba_unmultiplied(palette.panel[0], palette.panel[1], palette.panel[2], 225);
    let skin = crate::skin::skin(ctx);
    let id = egui::Id::new(("rollup", name));
    let mut toggle = false;
    let mut undock_it = false;
    let mut close = false;
    let mut body_height = 0.0f32;
    let mut body_width = WIDTH;
    // The header: its own area at the page's top.
    egui::Area::new(id).order(egui::Order::Middle).fixed_pos(egui::pos2(left, top)).constrain(false).show(ctx, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let (header, _) = ui.allocate_exact_size(egui::vec2(width, HEADER), egui::Sense::hover());
        let painter = ui.painter().clone();
        painter.rect_filled(header, 2.0, panel);
        painter.rect_stroke(header, 2.0, egui::Stroke::new(1.0, accent.gamma_multiply(0.6)), egui::StrokeKind::Inside);
        crate::font::text(ctx, &painter, egui::pos2(header.left() + 5.0, header.top() + 2.0), title, 13.0, accent);
        let mut x = header.right() - 16.0;
        for (what, tip) in [("close", "Close"), ("undock", "Undock: make it a window again"), ("roll", if expanded { "Roll the page up" } else { "Roll the page down" })] {
            let rect = egui::Rect::from_min_size(egui::pos2(x, header.top() + 2.0), egui::vec2(14.0, 14.0));
            let response = ui.interact(rect, id.with(what), egui::Sense::click());
            let state = if response.is_pointer_button_down_on() { "3" } else if response.hovered() { "2" } else { "1" };
            let tint = if response.hovered() { hover } else { accent };
            let sprite = match what {
                "roll" => format!("GFX_GUI_ROLLUP_{}_STATE{state}", if expanded { "COLLAPSE" } else { "EXPAND" }),
                "undock" => format!("GFX_GUI_WINDOW_MAXIMIZE{}", if state == "1" { String::new() } else { format!("_STATE{state}") }),
                _ => format!("GFX_GUI_WINDOW_CLOSE_X{}", if state == "1" { String::new() } else { format!("_STATE{state}") }),
            };
            let drawn = skin.as_deref().is_some_and(|s| s.draw_in(&painter, &sprite, rect, tint));
            if !drawn {
                let glyph = match what {
                    "roll" => if expanded { "-" } else { "+" },
                    "undock" => "^",
                    _ => "x",
                };
                painter.text(rect.center(), egui::Align2::CENTER_CENTER, glyph, egui::FontId::monospace(12.0), tint);
            }
            if response.clicked() {
                match what {
                    "roll" => toggle = true,
                    "undock" => undock_it = true,
                    _ => close = true,
                }
            }
            response.on_hover_text(tip);
            x -= 16.0;
        }
    });
    // The body: an egui window with no title bar at a fixed place under the header, so the content is sized exactly as a
    // free window sizes it; the original's frame pieces are painted under it once its size is known.
    let shown = if expanded {
        let body_id = egui::Id::new(("rollup-body", name));
        let margin = egui::Margin { left: 3, right: 3, top: 4, bottom: 8 };
        let known = ctx.memory(|m| m.area_rect(body_id)).filter(|r| r.is_finite() && r.width() > 20.0);
        let framed = skin.as_deref().filter(|s| s.has("GFX_GUI_WINDOW3_BORDER_TL"));
        egui::Window::new(title).id(body_id).title_bar(false).collapsible(false).resizable(false).movable(false).default_size(egui::vec2(width - 6.0, default_height(name))).fixed_pos(egui::pos2(left, top + HEADER)).constrain(false).frame(egui::Frame::NONE.inner_margin(margin)).show(ctx, |ui| {
            // (the body is the page's width: a view that takes the room it has fills it, one that needs more widens the page)
            ui.set_min_width(width - 6.0);
            ui.set_max_width(width - 6.0);
            let painter = ui.painter().clone();
            let under = painter.add(egui::Shape::Noop);
            let result = add(ui);
            let content = ui.min_rect();
            let outer = known.unwrap_or_else(|| egui::Rect::from_min_max(content.min - egui::vec2(3.0, 4.0), content.max + egui::vec2(3.0, 8.0)));
            let shape = match framed {
                Some(s) => crate::chrome::frame_shape(s, outer, panel, accent),
                None => egui::Shape::Vec(vec![egui::Shape::rect_filled(outer, 2.0, panel), egui::Shape::rect_stroke(outer, 2.0, egui::Stroke::new(1.0, accent), egui::StrokeKind::Inside)]),
            };
            painter.set(under, shape);
            body_height = outer.height();
            body_width = outer.width();
            result
        })
    } else {
        None
    };
    {
        let mut state = STATE.lock().unwrap();
        let frame = state.frame;
        if let Some(p) = state.pages.iter_mut().find(|p| p.name == name) {
            p.shown = frame;
            if expanded {
                p.height = body_height;
                p.width = body_width.max(WIDTH);
            }
            if toggle {
                p.expanded = !p.expanded;
            }
        }
        if toggle {
            save(&state);
        }
    }
    if undock_it {
        undock(name);
    }
    if close {
        *open = false;
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_round_trips() {
        let list = vec![("Wear".to_string(), true), ("Nano programs".to_string(), false)];
        let text = render(&list);
        assert_eq!(text, "Wear 1\nNano programs 0\n");
        assert_eq!(parse(&text), list);
        assert!(parse("Wear\nNCU yes\n 1\n").is_empty());
    }

    #[test]
    fn the_pages_stack_in_order() {
        let mut state = Rollup { pages: Vec::new(), loaded: true, frame: 5 };
        state.pages.push(Page { name: "Wear".into(), expanded: true, height: 100.0, width: WIDTH, shown: 5 });
        state.pages.push(Page { name: "NCU".into(), expanded: false, height: 60.0, width: WIDTH, shown: 4 });
        state.pages.push(Page { name: "Team".into(), expanded: true, height: 50.0, width: WIDTH, shown: 1 });
        state.pages.push(Page { name: "Faction".into(), expanded: true, height: 40.0, width: WIDTH, shown: 5 });
        assert_eq!(top_of(&state, "Wear", 0.0), 0.0);
        assert_eq!(top_of(&state, "NCU", 0.0), HEADER + 100.0 + GAP);
        // (a rolled page takes its header only; a page not drawn lately takes nothing)
        assert_eq!(top_of(&state, "Faction", 10.0), 10.0 + HEADER + 100.0 + GAP + HEADER + GAP);
    }
}
