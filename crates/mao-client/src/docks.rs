//! Windows docked as tabs of one frame: GUI.dll's `DockingController_c` (FUN_1003b01d), its `DockArea_c`s and the
//! `DockWindow_c` that shows an area on screen (`DockTabbedWindow`, made by FUN_1003c78b).
//!
//! In the original every `DockableView_c` (FUN_100395fd: the inventory, a backpack, the bank, a corpse's loot, the
//! mission log, the mission terminal, the info view, the tradeskill kit, knowledge, actions, and the views the rollup
//! area holds by default) lives in a dock area named by its `DockableViewDockName`: the rollup area (`rollup.rs`), or a
//! tabbed window, `DockArea<n>`, whose tabs are its views. A view opened for the first time gets an area of its own,
//! the first free `DockArea%u` (FUN_1003aca6); one opened again goes back into the area it was in, at its old place
//! (FUN_1003974e → `DockArea_c::AddView` FUN_10039d56 with -1). A closed view keeps its place in its area's list
//! (FUN_100396a9 → `RemoveView(view, 0)`), so the list holds closed views too; the window shows a tab for each open
//! one, and goes away once its last tab is closed (FUN_1003c2f9). The tab opened is the one shown (FUN_1003c384); the
//! one shown being closed, the tab now at its place is shown, else the last (`TabView` FUN_10147fbc).
//!
//! A tab is taken up by the right button let go on it (`TabView::SlotBeginDrag` 0x101459dc; the `Tab`'s second button,
//! as the chat windows' tabs here) and follows the pointer as a picture of itself (FUN_1003c428, a `DragObject_c` of
//! `dockableview/view` naming the area and the view). Put down on a tab of a window, the view leaves its area and goes
//! in before that tab; on the strip past the tabs, last (FUN_1003c5a9 with `TabView::SlotItemDropped` 0x10145a95);
//! not on its own window when it is that window's only tab (cancelled). Put down anywhere else, it goes into a new area
//! of its own whose window's corner is there (FUN_1003b263), but only when its area lists more than one view.
//! Closing the window asks each of its views to close (FUN_1003c94d).
//!
//! The areas are kept in `docks.ini` beside `mao.ini` (`area<TAB>shown view<TAB>view…` a line, the views in their tabs'
//! order), as the original keeps `DockAreas/<area>.xml` (FUN_1003aad8: `dock_name`, `dock_type`, and `dock_config`
//! with `docked_view_identities`, the frame and `selected_tab`). Without the file the areas start as the original's
//! own files for the character have them (`aoprefs::dock_files`), and their windows at its frames. A window's place is
//! kept under its area's name (`layout.rs`); the pin and the fading are the area window's too.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::theme::Palette;

/// The original's dockable views that are windows here: their identity (`DockableView_c` +0x130, the name their area
/// file lists) and the window's name here. Backpacks (`Container_<type>x<instance>.xml`) and other containers
/// (`TempContainer_<n>.xml`) are dockable views too ([`dockable`]).
pub const VIEWS: &[(&str, &str)] = &[
    ("inventory_window", "Inventory"),
    ("bank_window", "bank_window"),
    ("mission_window", "MissionView"),
    ("missionselection_window", "Missions"),
    ("info_window", "Info"),
    ("tradeskill_window", "Tradeskill"),
    ("knowledge_window", "Knowledge"),
    ("specialaction_window", "Actions"),
    ("team_view", "Team"),
    ("friends_window", "Friends"),
    ("nano_window", "Nano programs"),
    ("ncu_window", "NCU"),
    ("wear_window", "Wear"),
];

/// Whether the window is one of the original's dockable views.
pub fn dockable(name: &str) -> bool {
    VIEWS.iter().any(|v| v.1 == name) || name.starts_with("Container_") || name.starts_with("TempContainer_")
}

/// The window here for an identity of the original's area files.
pub fn client_name(identity: &str) -> Option<String> {
    let id = identity.strip_suffix(".xml").unwrap_or(identity);
    if id.starts_with("Container_") || id.starts_with("TempContainer_") {
        return Some(id.to_string());
    }
    VIEWS.iter().find(|v| v.0 == id).map(|v| v.1.to_string())
}

/// A dock area: its name, its views in their tabs' order (open or not) and the one shown.
#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub name: String,
    pub views: Vec<String>,
    pub selected: Option<String>,
}

/// The lines of `docks.ini`: the area's name, the view shown (empty for none) and the views, separated by tabs.
pub fn parse(text: &str) -> Vec<Area> {
    let mut areas: Vec<Area> = Vec::new();
    for line in text.lines() {
        let mut words = line.split('\t').map(str::trim);
        let (Some(name), Some(selected)) = (words.next(), words.next()) else { continue };
        let views: Vec<String> = words.filter(|w| !w.is_empty()).map(String::from).collect();
        if name.is_empty() || views.is_empty() || areas.iter().any(|a| a.name == name || a.views.iter().any(|v| views.contains(v))) {
            continue;
        }
        let selected = (!selected.is_empty() && views.iter().any(|v| v == selected)).then(|| selected.to_string());
        areas.push(Area { name: name.to_string(), views, selected });
    }
    areas
}

pub fn render(areas: &[Area]) -> String {
    areas.iter().map(|a| format!("{}\t{}\t{}\n", a.name, a.selected.as_deref().unwrap_or(""), a.views.join("\t"))).collect()
}

/// The area a view is in.
pub fn area_of(areas: &[Area], view: &str) -> Option<usize> {
    areas.iter().position(|a| a.views.iter().any(|v| v == view))
}

/// The first `DockArea%u` no area has (FUN_1003aca6 with an empty name).
pub fn free_name(areas: &[Area]) -> String {
    (0u32..).map(|n| format!("DockArea{n}")).find(|n| !areas.iter().any(|a| a.name == *n)).unwrap_or_default()
}

/// `DockArea_c::AddView` (FUN_10039d56): the view into the area `target` (made when there is none), before its entry
/// `index` — with none, where it is already, else last; past the end, last — after taking it out of any other area it
/// was in (the old area's `RemoveView(view, 1)`). An area left with no views is gone. (The original's branch for a view
/// moved inside its own area is not reached: its callers take the view out first.)
pub fn add_view(areas: &mut Vec<Area>, target: &str, index: Option<usize>, view: &str) {
    for a in areas.iter_mut().filter(|a| a.name != target) {
        a.views.retain(|v| v != view);
        if a.selected.as_deref() == Some(view) {
            a.selected = None;
        }
    }
    areas.retain(|a| a.name == target || !a.views.is_empty());
    let t = match areas.iter().position(|a| a.name == target) {
        Some(t) => t,
        None => {
            areas.push(Area { name: target.to_string(), views: Vec::new(), selected: None });
            areas.len() - 1
        }
    };
    let area = &mut areas[t];
    let present = area.views.iter().position(|v| v == view);
    let len = area.views.len();
    if present.is_none() {
        let at = index.filter(|i| *i <= len).unwrap_or(len);
        area.views.insert(at, view.to_string());
    }
}

/// A view out of its area (`RemoveView(view, 1)`: its entry erased); an area left with no views is gone.
pub fn remove_view(areas: &mut Vec<Area>, view: &str) {
    for a in areas.iter_mut() {
        a.views.retain(|v| v != view);
        if a.selected.as_deref() == Some(view) {
            a.selected = None;
        }
    }
    areas.retain(|a| !a.views.is_empty());
}

/// The view shown in an area, the one shown being closed: the open one at its place in the list, else the last open
/// one before it (`TabView` FUN_10147fbc: the tab at the deleted one's place, else the last); with none shown, the
/// first open one.
pub fn fix_selection(area: &mut Area, live: impl Fn(&str) -> bool) {
    if area.selected.as_deref().is_some_and(|s| live(s) && area.views.iter().any(|v| v == s)) {
        return;
    }
    let at = area.selected.as_ref().and_then(|s| area.views.iter().position(|v| v == s));
    let next = match at {
        Some(k) => area.views[k + 1..].iter().find(|v| live(v)).or_else(|| area.views[..k].iter().rev().find(|v| live(v))),
        None => area.views.iter().find(|v| live(v)),
    };
    if let Some(next) = next.cloned() {
        area.selected = Some(next);
    }
}

/// Where a carried tab is put down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target<'a> {
    /// On a tab of an area's window: the area and that tab's view.
    Tab(&'a str, &'a str),
    /// On an area's tab strip past its tabs.
    Strip(&'a str),
    /// On no window's tabs.
    Nowhere,
}

/// What putting a tab down did.
#[derive(Clone, Debug, PartialEq)]
pub enum Dropped {
    Cancelled,
    Moved,
    /// Into a new area of its own, of this name.
    NewArea(String),
}

/// A carried view put down ([`Target`]): on a window's tabs, out of its area and in before the tab it is put on (or
/// last), shown there (FUN_1003c5a9; not on its own window when that is its only open tab); anywhere else into a new
/// area of its own when its area lists more than one view (FUN_1003c428 connects FUN_1003b263 only then).
pub fn drop_on(areas: &mut Vec<Area>, view: &str, target: Target, live: impl Fn(&str) -> bool) -> Dropped {
    let Some(source) = area_of(areas, view) else { return Dropped::Cancelled };
    match target {
        Target::Tab(area, _) | Target::Strip(area) => {
            if areas[source].name == area && areas[source].views.iter().filter(|v| live(v)).count() == 1 {
                return Dropped::Cancelled;
            }
            if !areas.iter().any(|a| a.name == area) {
                return Dropped::Cancelled;
            }
            let area = area.to_string();
            remove_view_keeping(areas, view, &area);
            let t = areas.iter().position(|a| a.name == area).unwrap_or(0);
            // (the index of the tab put on, once the view is out of the list: none — the view itself — is last)
            let index = match target {
                Target::Tab(_, on) => areas[t].views.iter().position(|v| v == on),
                _ => None,
            };
            add_view(areas, &area, Some(index.unwrap_or(areas[t].views.len())), view);
            if let Some(a) = areas.iter_mut().find(|a| a.name == area) {
                a.selected = Some(view.to_string());
            }
            Dropped::Moved
        }
        Target::Nowhere => {
            if areas[source].views.len() < 2 {
                return Dropped::Cancelled;
            }
            let name = free_name(areas);
            add_view(areas, &name, None, view);
            if let Some(a) = areas.iter_mut().find(|a| a.name == name) {
                a.selected = Some(view.to_string());
            }
            Dropped::NewArea(name)
        }
    }
}

/// [`remove_view`], keeping the area `keep` even when it is left empty.
fn remove_view_keeping(areas: &mut Vec<Area>, view: &str, keep: &str) {
    for a in areas.iter_mut() {
        a.views.retain(|v| v != view);
        if a.selected.as_deref() == Some(view) {
            a.selected = None;
        }
    }
    areas.retain(|a| a.name == keep || !a.views.is_empty());
}

/// A tab strip drawn this frame: its area, where it is and where each view's tab is.
struct Strip {
    area: String,
    strip: egui::Rect,
    tabs: Vec<(String, egui::Rect)>,
}

struct State {
    areas: Vec<Area>,
    loaded: bool,
    /// Whether the areas are this client's own (`docks.ini` was read) or still to be taken from the original's files.
    seeded: bool,
    frame: u64,
    /// The frame each view was last open in, its title, and the frame each area's window was last drawn in.
    shown: BTreeMap<String, u64>,
    titles: BTreeMap<String, String>,
    drawn: BTreeMap<String, u64>,
    strips: Vec<Strip>,
    /// The view whose tab is carried, where it was taken hold of from the tab's corner, and whether it is a page of
    /// the rollup area taken up (`rollup.rs`) rather than a tab.
    carried: Option<(String, egui::Vec2, bool)>,
}

static STATE: Mutex<State> = Mutex::new(State { areas: Vec::new(), loaded: false, seeded: false, frame: 1, shown: BTreeMap::new(), titles: BTreeMap::new(), drawn: BTreeMap::new(), strips: Vec::new(), carried: None });

fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("docks.ini")
}

/// (the tests and the previews never read or write the player's files)
fn disabled() -> bool {
    cfg!(test) || std::env::var_os("MAO_NO_LAYOUT").is_some() || std::env::var_os("MAO_UI_PREVIEW").is_some()
}

fn load(state: &mut State) {
    if state.loaded {
        return;
    }
    state.loaded = true;
    if disabled() {
        state.seeded = true;
        return;
    }
    if let Ok(text) = std::fs::read_to_string(path()) {
        state.areas = parse(&text);
        state.seeded = true;
    }
}

fn save(state: &State) {
    if !disabled() {
        let _ = std::fs::write(path(), render(&state.areas));
    }
}

/// An area's window as a dockable view's `chrome::window` draws it: the area's name (the window's), its open views and
/// their titles in their tabs' order, and the one shown.
pub struct Shown {
    pub area: String,
    pub tabs: Vec<(String, String)>,
    pub selected: usize,
}

/// A dockable view open this frame (called by `chrome::window` each frame it is open): its area's window to draw, when
/// it is the view shown there; none while another tab of its area is shown (the view waits, open, behind its tab).
pub fn enter(name: &str, title: &str) -> Option<Shown> {
    let mut guard = STATE.lock().unwrap();
    let state = &mut *guard;
    load(state);
    let frame = state.frame;
    let fresh = state.shown.get(name).is_none_or(|f| f + 1 < frame);
    state.shown.insert(name.to_string(), frame);
    if state.titles.get(name).is_none_or(|t| t != title) {
        state.titles.insert(name.to_string(), title.to_string());
    }
    let index = match area_of(&state.areas, name) {
        Some(i) => i,
        None => {
            // (a view opened the first time: an area of its own, FUN_1003974e with an empty dock name)
            let area = free_name(&state.areas);
            state.areas.push(Area { name: area, views: vec![name.to_string()], selected: None });
            state.areas.len() - 1
        }
    };
    let shown = &state.shown;
    let live = |v: &str| shown.get(v).is_some_and(|f| f + 1 >= frame);
    let area = &mut state.areas[index];
    let before = area.selected.clone();
    if fresh {
        area.selected = Some(name.to_string());
    }
    fix_selection(area, live);
    let changed = area.selected != before;
    let selected = area.selected.clone();
    let tabs: Vec<(String, String)> = area.views.iter().filter(|v| live(v)).map(|v| (v.clone(), state.titles.get(v).cloned().unwrap_or_else(|| v.clone()))).collect();
    let area_name = area.name.clone();
    if fresh || changed {
        save(state);
    }
    if selected.as_deref() != Some(name) || state.drawn.get(&area_name) == Some(&frame) {
        return None;
    }
    state.drawn.insert(area_name.clone(), frame);
    let selected = tabs.iter().position(|t| t.0 == name).unwrap_or(0);
    Some(Shown { area: area_name, tabs, selected })
}

/// After an area's window was drawn with its tab strip: where the strip and its tabs are (for a carried tab to be put
/// down on), the tab now shown, and a tab taken up by the right button.
pub fn after(area: &str, views: &[String], selected: usize, hits: &crate::chrome::TabHits, pointer: Option<egui::Pos2>) {
    let mut state = STATE.lock().unwrap();
    if let Some(strip) = hits.strip {
        state.strips.push(Strip { area: area.to_string(), strip, tabs: views.iter().cloned().zip(hits.tabs.iter().copied()).collect() });
    }
    if let Some(view) = views.get(selected)
        && let Some(a) = state.areas.iter_mut().find(|a| a.name == area)
        && a.selected.as_ref() != Some(view)
    {
        a.selected = Some(view.clone());
        save(&state);
    }
    if let (Some(i), None) = (hits.right_released, state.carried.as_ref())
        && let (Some(view), Some(tab)) = (views.get(i), hits.tabs.get(i))
    {
        let grab = pointer.map_or(egui::Vec2::ZERO, |at| at - tab.min);
        state.carried = Some((view.clone(), grab, false));
    }
}

/// The area window's close button: every other open view of the area closes too (FUN_1003c94d asks each to).
pub fn close_others(area: &str, except: &str) {
    let state = STATE.lock().unwrap();
    let frame = state.frame;
    if let Some(a) = state.areas.iter().find(|a| a.name == area) {
        for v in a.views.iter().filter(|v| *v != except && state.shown.get(*v).is_some_and(|f| f + 1 >= frame)) {
            crate::chrome::close_next(v);
        }
    }
}

/// A rollup page taken up by the right button (`RollupPage_c`'s FUN_100493f2): carried as a tab is, its picture its
/// title, where it was taken hold of from the page's corner.
pub fn carry(view: &str, title: &str, grab: egui::Vec2) {
    let mut state = STATE.lock().unwrap();
    if state.carried.is_none() {
        state.titles.insert(view.to_string(), title.to_string());
        state.carried = Some((view.to_string(), grab, true));
    }
}

/// A rollup page put down off the column ([`Target`]): on a window's tabs into that area, before the tab it is put on
/// (or last), shown there; anywhere else into a new area of its own (FUN_100493f2 connects FUN_1003b263 whatever the
/// page's area holds).
pub fn drop_from_column(areas: &mut Vec<Area>, view: &str, target: Target) -> Dropped {
    let area = match target {
        Target::Tab(area, _) | Target::Strip(area) => {
            let Some(t) = areas.iter().position(|a| a.name == area) else { return Dropped::Cancelled };
            let index = match target {
                Target::Tab(_, on) => areas[t].views.iter().position(|v| v == on),
                _ => None,
            };
            let index = index.unwrap_or(areas[t].views.len());
            add_view(areas, area, Some(index), view);
            area.to_string()
        }
        Target::Nowhere => {
            let name = free_name(areas);
            add_view(areas, &name, None, view);
            name
        }
    };
    if let Some(a) = areas.iter_mut().find(|a| a.name == area) {
        a.selected = Some(view.to_string());
    }
    if matches!(target, Target::Nowhere) { Dropped::NewArea(area) } else { Dropped::Moved }
}

/// A view out of its area (docked into the rollup area instead).
pub fn take_out(name: &str) {
    let mut state = STATE.lock().unwrap();
    load(&mut state);
    if area_of(&state.areas, name).is_some() {
        remove_view(&mut state.areas, name);
        save(&state);
    }
}

/// The end of a frame: the areas taken from the original's files once the character is known, and a carried tab drawn
/// at the pointer and put down by the left button (Escape lets go of it: this client's own, as for the chat tabs).
pub fn end_frame(ctx: &egui::Context, palette: &Palette) {
    let mut guard = STATE.lock().unwrap();
    let state = &mut *guard;
    load(state);
    if !state.seeded
        && let Some(character) = crate::layout::character()
    {
        state.seeded = true;
        for file in crate::aoprefs::dock_files(character) {
            let views: Vec<String> = file.views.iter().filter_map(|v| client_name(v)).filter(|v| area_of(&state.areas, v).is_none()).collect();
            if views.is_empty() {
                continue;
            }
            let name = if file.name.is_empty() || state.areas.iter().any(|a| a.name == file.name) { free_name(&state.areas) } else { file.name.clone() };
            let selected = usize::try_from(file.selected).ok().and_then(|k| file.views.get(k)).and_then(|v| client_name(v)).filter(|v| views.contains(v));
            state.areas.push(Area { name, views, selected });
        }
        save(state);
    }
    let strips = std::mem::take(&mut state.strips);
    state.frame += 1;
    let Some((view, grab, page)) = state.carried.clone() else { return };
    ctx.request_repaint();
    let Some(at) = ctx.input(|i| i.pointer.latest_pos()) else { return };
    let title = state.titles.get(&view).cloned().unwrap_or_else(|| view.clone());
    // (a rollup page's picture: its title in font 5, FUN_100493f2's `TextView_c`)
    if page {
        egui::Area::new(egui::Id::unique("rollup-page-carried")).order(egui::Order::Tooltip).interactable(false).fixed_pos(at - grab).show(ctx, |ui| {
            let font = crate::gui::font::Measure { ctx: Some(ctx.clone()) }.egui_font(crate::gui::font::FontId::NORMAL);
            ui.label(egui::RichText::new(&title).font(font).color(crate::chrome::rgb(palette.text)));
        });
    }
    // (the tab's picture: an open tab with its title, `TabView::CreateDragImage` 0x10146b1e)
    if !page {
    egui::Area::new(egui::Id::unique("dock-tab-carried")).order(egui::Order::Tooltip).interactable(false).fixed_pos(at - grab).show(ctx, |ui| {
        let font = crate::font::verdana(ctx, 13.0);
        let width = ui.painter().layout_no_wrap(title.clone(), font.clone(), egui::Color32::WHITE).size().x.ceil() + 22.0;
        let skin = crate::skin::skin(ctx);
        let height = skin.as_ref().and_then(|s| s.get("GFX_GUI_TAB_ACTIVE_MIDDLE")).map_or(18.0, |s| s.size.y);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        if !skin.is_some_and(|s| s.three_slice(ui.painter(), "GFX_GUI_TAB_ACTIVE", rect, crate::chrome::rgb(palette.accent))) {
            ui.painter().rect_filled(rect, 3.0, crate::chrome::rgb(palette.panel));
        }
        ui.painter().text(egui::pos2(rect.left() + 5.0, rect.center().y), egui::Align2::LEFT_CENTER, &title, font, egui::Color32::WHITE);
    });
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.carried = None;
        return;
    }
    if !ctx.input(|i| i.pointer.primary_clicked()) {
        return;
    }
    state.carried = None;
    // (put down on the rollup area's column: the column takes it, FUN_10049121)
    if let Some(on) = crate::rollup::column_target(ctx, at) {
        if crate::rollup::put(&view, on.as_deref()) && area_of(&state.areas, &view).is_some() {
            remove_view(&mut state.areas, &view);
            save(state);
        }
        return;
    }
    let target = match strips.iter().find(|s| s.strip.contains(at)) {
        Some(s) => match s.tabs.iter().find(|t| t.1.contains(at)) {
            Some(t) => Target::Tab(&s.area, &t.0),
            None => Target::Strip(&s.area),
        },
        None => Target::Nowhere,
    };
    // (a rollup page put down elsewhere leaves the column; a window that is no dockable view here is put down free)
    if page {
        if !dockable(&view) {
            crate::rollup::undock(&view);
            crate::layout::place(&view, at - grab);
            return;
        }
        let dropped = drop_from_column(&mut state.areas, &view, target);
        if dropped != Dropped::Cancelled {
            crate::rollup::undock(&view);
            if let Dropped::NewArea(name) = &dropped {
                crate::layout::place(name, at - grab);
            }
            save(state);
        }
        return;
    }
    let frame = state.frame - 1;
    let shown = &state.shown;
    let live = |v: &str| shown.get(v).is_some_and(|f| f + 1 >= frame);
    let dropped = drop_on(&mut state.areas, &view, target, live);
    if let Dropped::NewArea(name) = &dropped {
        crate::layout::place(name, at - grab);
    }
    if dropped != Dropped::Cancelled {
        save(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(name: &str, views: &[&str], selected: Option<&str>) -> Area {
        Area { name: name.into(), views: views.iter().map(|v| v.to_string()).collect(), selected: selected.map(String::from) }
    }

    #[test]
    fn the_file_round_trips() {
        let areas = vec![area("DockArea0", &["Inventory", "MissionView"], Some("MissionView")), area("DockArea1", &["Container_1x2"], None)];
        let text = render(&areas);
        assert_eq!(text, "DockArea0\tMissionView\tInventory\tMissionView\nDockArea1\t\tContainer_1x2\n");
        assert_eq!(parse(&text), areas);
        // (an area with no views, a view in two areas, a shown view not listed: left out or dropped)
        let odd = parse("Empty\t\t\nA\tX\tInventory\nB\t\tInventory\tNCU\n");
        assert_eq!(odd, vec![area("A", &["Inventory"], None)]);
    }

    #[test]
    fn identities_name_the_windows_here() {
        assert_eq!(client_name("inventory_window").as_deref(), Some("Inventory"));
        assert_eq!(client_name("Container_57347x1074.xml").as_deref(), Some("Container_57347x1074"));
        assert_eq!(client_name("TempContainer_0.xml").as_deref(), Some("TempContainer_0"));
        assert_eq!(client_name("tip_window"), None);
        assert!(dockable("Inventory") && dockable("TempContainer_3") && dockable("Nano programs") && !dockable("Skills"));
    }

    /// The first free `DockArea%u`; a view added again stays where it was, a new one goes last or before an entry.
    #[test]
    fn views_are_added_as_the_area_adds_them() {
        let mut areas = vec![area("DockArea0", &["Inventory"], None), area("DockArea2", &["Info"], None)];
        assert_eq!(free_name(&areas), "DockArea1");
        add_view(&mut areas, "DockArea0", None, "MissionView");
        add_view(&mut areas, "DockArea0", Some(1), "Container_1x2");
        add_view(&mut areas, "DockArea0", None, "Inventory");
        add_view(&mut areas, "DockArea0", Some(99), "Knowledge");
        assert_eq!(areas[0].views, ["Inventory", "Container_1x2", "MissionView", "Knowledge"]);
        // (out of its old area first: one left with nothing is gone)
        add_view(&mut areas, "DockArea0", Some(0), "Info");
        assert_eq!(areas.len(), 1);
        assert_eq!(areas[0].views[0], "Info");
    }

    /// The tab shown being closed, the open one at its place is shown, else the last open one before it.
    #[test]
    fn a_closed_tab_gives_way_to_the_next() {
        let mut a = area("DockArea0", &["A", "B", "C", "D"], Some("B"));
        fix_selection(&mut a, |v| v != "B");
        assert_eq!(a.selected.as_deref(), Some("C"));
        let mut a = area("DockArea0", &["A", "B", "C", "D"], Some("D"));
        fix_selection(&mut a, |v| v == "A" || v == "B");
        assert_eq!(a.selected.as_deref(), Some("B"));
        let mut a = area("DockArea0", &["A", "B"], None);
        fix_selection(&mut a, |v| v == "B");
        assert_eq!(a.selected.as_deref(), Some("B"));
        // (the shown one open: it stays)
        let mut a = area("DockArea0", &["A", "B"], Some("A"));
        fix_selection(&mut a, |_| true);
        assert_eq!(a.selected.as_deref(), Some("A"));
    }

    #[test]
    fn a_tab_put_on_a_window_goes_before_the_tab_it_is_put_on() {
        let all = |_: &str| true;
        let mut areas = vec![area("DockArea0", &["Inventory", "MissionView"], Some("Inventory")), area("DockArea1", &["Container_1x2"], Some("Container_1x2"))];
        assert_eq!(drop_on(&mut areas, "Container_1x2", Target::Tab("DockArea0", "MissionView"), all), Dropped::Moved);
        // (its own area, left with nothing, is gone; the view is shown where it went)
        assert_eq!(areas, vec![area("DockArea0", &["Inventory", "Container_1x2", "MissionView"], Some("Container_1x2"))]);
        // (on the strip past the tabs: last; on its own tab: out and in again, last)
        assert_eq!(drop_on(&mut areas, "Inventory", Target::Strip("DockArea0"), all), Dropped::Moved);
        assert_eq!(areas[0].views, ["Container_1x2", "MissionView", "Inventory"]);
        assert_eq!(drop_on(&mut areas, "Container_1x2", Target::Tab("DockArea0", "Container_1x2"), all), Dropped::Moved);
        assert_eq!(areas[0].views, ["MissionView", "Inventory", "Container_1x2"]);
    }

    /// A rollup page put down on a window's tabs goes into that area before the tab, or last on the strip; anywhere
    /// else into a new area of its own.
    #[test]
    fn a_page_put_down_off_the_column_goes_into_an_area() {
        let mut areas = vec![area("DockArea0", &["Inventory", "MissionView"], Some("Inventory"))];
        assert_eq!(drop_from_column(&mut areas, "NCU", Target::Tab("DockArea0", "MissionView")), Dropped::Moved);
        assert_eq!(areas, vec![area("DockArea0", &["Inventory", "NCU", "MissionView"], Some("NCU"))]);
        assert_eq!(drop_from_column(&mut areas, "Wear", Target::Strip("DockArea0")), Dropped::Moved);
        assert_eq!(areas[0].views, ["Inventory", "NCU", "MissionView", "Wear"]);
        assert_eq!(drop_from_column(&mut areas, "Team", Target::Nowhere), Dropped::NewArea("DockArea1".into()));
        assert_eq!(areas[1], area("DockArea1", &["Team"], Some("Team")));
        assert_eq!(drop_from_column(&mut areas, "Friends", Target::Strip("DockArea7")), Dropped::Cancelled);
    }

    #[test]
    fn a_lone_tab_stays_and_a_tab_put_nowhere_gets_a_window_of_its_own() {
        let mut areas = vec![area("DockArea0", &["Inventory", "MissionView"], Some("Inventory"))];
        // (the only open tab of its window, put on that window: nothing happens)
        assert_eq!(drop_on(&mut areas, "Inventory", Target::Strip("DockArea0"), |v| v == "Inventory"), Dropped::Cancelled);
        // (put nowhere: a new area when its own lists more than one view, the closed ones too)
        assert_eq!(drop_on(&mut areas, "MissionView", Target::Nowhere, |v| v == "Inventory"), Dropped::NewArea("DockArea1".into()));
        assert_eq!(areas, vec![area("DockArea0", &["Inventory"], Some("Inventory")), area("DockArea1", &["MissionView"], Some("MissionView"))]);
        assert_eq!(drop_on(&mut areas, "MissionView", Target::Nowhere, |_| true), Dropped::Cancelled);
        assert_eq!(areas.len(), 2);
    }
}
