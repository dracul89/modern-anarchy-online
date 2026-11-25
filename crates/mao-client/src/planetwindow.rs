//! The planet map window: GUI.dll's `PlanetMapView_c` (`#WindowPlanetMap`, FUN_1004d92d) over the map `planetmaps.rs`
//! picks for the playfield. A level is drawn tile by tile at its own size. The mouse on the map, as the original's
//! tile view has it (`BitmapTileView_c`, MouseDown 0x1005077c; read 2026-11-25):
//!
//! * a **double left click** steps a level closer and a **double right click** a level further (FUN_1004bb0a,
//!   FUN_1004bb2b), and the point clicked comes to the middle of the view (`SetLevel` FUN_1004c2e2: the click's
//!   fraction of the level's rectangle, kept by FUN_1004bae5, is put at half the view, no further left or up than the
//!   level's own corner; a fraction with a zero in it is not aimed at, and one kept from a click at the last level
//!   waits for the next change of level);
//! * any other press of **any button** grabs the map, which then goes one and a half times the mouse's way
//!   (FUN_100505d2; the original also holds the pointer still meanwhile, which is not done here);
//! * the **wheel** scrolls the map up and down, 10 a notch, while the level is taller than the view, and does nothing
//!   else (`ScrollView_c::WheelMoved` 0x10142820 with `_Initialize`'s step): it does not zoom. Until 2026-11-25 a
//!   notch of the wheel was a level, around the pointer.
//!
//! The buttons `PlanetMap_ZoomIn` / `PlanetMap_ZoomOut` step a level and bring the character to the middle when it is on
//! the map (`SetLevel` calls FUN_1004bd97; else the level's corner is shown); each is dead at its end of the levels.
//! `PlanetMap_CentreOnChar` and `PlanetMap_CentreOnQuest` bring the character or the mission's waypoint to the middle.
//! The character is placed as FUN_1004bedd places it (`PlanetMap::pixel`), an arrow turned to its heading; the
//! waypoint uploaded from a mission is a white dot ("MissionShownAsDot"). The pointer's place is not written on the
//! map (the tile view has no tip; this client wrote it at the lower left until 2026-11-25).

use std::sync::Arc;

use mao_formats::planetmap::PlanetMap;

use crate::theme::Palette;

#[derive(Default)]
pub struct PlanetWindow {
    pub open: bool,
    map: Option<Arc<PlanetMap>>,
    /// The playfield and ExpansionPlayfield the map was picked for (to notice a new choice).
    picked_for: Option<(u32, i64)>,
    level: usize,
    /// The level's pixel in the middle of the view, and whether it follows the character.
    centre: Option<egui::Pos2>,
    follow: bool,
    tiles: crate::planetmaps::Tiles,
    /// A new map was chosen: pick again.
    repick: bool,
    /// A double click's fraction of the level's rectangle, to be aimed at when the level next changes
    /// (the window's +0x1ec / +0x1f0; zero: none).
    aim: egui::Vec2,
    /// The buttons' orders (they sit under the map, so they act the frame after).
    pending_zoom: Option<i32>,
    pending_centre: Option<(u32, [f32; 3])>,
    /// How tall the button rows were last frame (the map takes the rest of the window).
    buttons_height: f32,
}

/// Where the window's zoom level is kept (the original's `PlanetMapViewConfig` `ZoomLev`).
const ZOOM_KEY: &str = "PlanetMapZoomLev";

/// The level a map opens at: the kept `ZoomLev` (FUN_1004d92d reads it when it is there; the view starts at 0), held
/// to the map's levels as FUN_1004c2e2 holds it (no more than the last, no less than 0).
fn opening_level(kept: Option<usize>, levels: usize) -> usize {
    kept.unwrap_or(0).min(levels.saturating_sub(1))
}

/// The level's pixel of a fraction of the map's rectangle, and back: how a point stays put from one level to another.
fn fraction(map: &PlanetMap, level: usize, p: egui::Pos2) -> egui::Vec2 {
    let Some(l) = map.levels.get(level) else { return egui::Vec2::ZERO };
    let [left, top, right, bottom] = l.rect.map(|v| v as f32);
    egui::vec2((p.x - left) / (right - left).max(1.0), (p.y - top) / (bottom - top).max(1.0))
}

fn from_fraction(map: &PlanetMap, level: usize, f: egui::Vec2) -> egui::Pos2 {
    let Some(l) = map.levels.get(level) else { return egui::Pos2::ZERO };
    let [left, top, right, bottom] = l.rect.map(|v| v as f32);
    egui::pos2(left + f.x * (right - left), top + f.y * (bottom - top))
}

/// The level's pixel that comes to the middle of a view `half` its size across and down when the fraction `aim` of the
/// level's rectangle is aimed at (`SetLevel`, 0x1004c508..0x1004c61d): the point itself, but no nearer the level's
/// left and top than half the view (the scroll offset `half - point` is held at 0 and under). None when the fraction
/// has a zero in it (both are tested: no aiming then).
fn aimed(map: &PlanetMap, level: usize, aim: egui::Vec2, half: egui::Vec2) -> Option<egui::Pos2> {
    if aim.x == 0.0 || aim.y == 0.0 {
        return None;
    }
    let p = from_fraction(map, level, aim);
    Some(egui::pos2(p.x.max(half.x), p.y.max(half.y)))
}

/// The middle's pixel for the character at `me` (FUN_1004bb43: the offset is `-max(0, place - half the view)`), or the
/// level's corner shown when the character is not on the map.
fn on_character(me: Option<(f32, f32)>, half: egui::Vec2) -> egui::Pos2 {
    me.map_or(half.to_pos2(), |(x, y)| egui::pos2(x.max(half.x), y.max(half.y)))
}

/// The middle's pixel after the wheel turned `notches` over a view `half` its size down on a level `height` tall
/// (`ScrollView_c::WheelMoved`, 0x10142820): 10 a notch up or down, only while the level is taller than the view, and
/// no further than its top and foot.
fn wheeled(centre_y: f32, notches: f32, half: f32, height: f32) -> f32 {
    if height <= half * 2.0 {
        return centre_y;
    }
    (centre_y - notches * 10.0).clamp(half, height - half)
}

impl PlanetWindow {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.follow = true;
        }
    }

    /// Steps to another level (FUN_1004bb0a, FUN_1004bb2b: nothing at an end of the levels), as `SetLevel`
    /// (FUN_1004c2e2) does: the character comes to the middle (`me` its place on the NEW level's map, none when it is not
    /// on it: the level's corner is shown then), unless a double click's point is kept, which is aimed at and
    /// forgotten. `half` is half the view.
    fn zoom(&mut self, by: i32, playfield: u32, own: Option<[f32; 3]>, half: egui::Vec2) {
        let Some(map) = self.map.clone() else { return };
        let new = (self.level as i32 + by).clamp(0, map.levels.len() as i32 - 1) as usize;
        if new == self.level {
            return;
        }
        self.level = new;
        crate::layout::set_tab(ZOOM_KEY, new);
        let me = own.and_then(|p| map.pixel(new, playfield, p[0], p[2]));
        match aimed(&map, new, self.aim, half) {
            Some(centre) => {
                self.centre = Some(centre);
                self.follow = false;
                self.aim = egui::Vec2::ZERO;
            }
            None => {
                self.centre = Some(on_character(me, half));
                self.follow = me.is_some();
            }
        }
    }

    /// Draws the window: `own` the character's playfield, place (server x, y, z) and heading; `waypoint` the mission's;
    /// `expansion_playfield` the character's stat 531, which picks the map (`planetmaps::window_kind`).
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, playfield: u32, expansion_playfield: i64, own: Option<[f32; 3]>, heading: f32, waypoint: Option<(u32, [f32; 3])>) {
        self.tiles.update(ctx);
        if !self.open {
            return;
        }
        if self.picked_for != Some((playfield, expansion_playfield)) || std::mem::take(&mut self.repick) {
            let old = self.map.as_ref().map(|m| (m.root.clone(), m.name.clone()));
            // (the original's rule, FUN_1004d92d: the Shadowlands map when the ExpansionPlayfield's bit 0 is set, else
            // Rubi-Ka's. Until 2026-11-19 the map that had the playfield, so a Shadowlands dungeon showed Rubi-Ka.)
            self.map = crate::planetmaps::for_window(expansion_playfield);
            self.picked_for = Some((playfield, expansion_playfield));
            if self.map.as_ref().map(|m| (m.root.clone(), m.name.clone())) != old {
                // (a map opens at the level kept in PlanetMapViewConfig's `ZoomLev`, else the first, FUN_1004d92d)
                self.level = self.map.as_ref().map_or(0, |m| opening_level(crate::layout::tab(ZOOM_KEY), m.levels.len()));
                self.centre = None;
                self.follow = true;
            }
        }
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut open = self.open;
        let title = self.map.as_ref().map_or("Planet Map".to_string(), |m| m.name.clone());
        // The original's window (PlanetMapView_c, FUN_1004d92d) is made with the frame `Rect(0, 0, 300, 400)` (the floats at
        // 0x101b1844 and 0x101b1848): 300 wide and 400 tall at the screen's top left until the player moves it (the
        // window's place is kept), the map over two rows of buttons: Zoom in / Zoom out, Character / Mission.
        let at = ctx.content_rect().min;
        crate::chrome::window(ctx, palette, "PlanetMap", &title, &mut open, |w| w.default_pos(crate::layout::pos("PlanetMap", at)).default_size(egui::vec2(300.0, 400.0)).min_size(egui::vec2(200.0, 240.0)).resizable(true), |ui| {
            let mut zoom: Option<i32> = self.pending_zoom.take();
            let centre_on: Option<(u32, [f32; 3])> = self.pending_centre.take();
            // (the two rows of buttons under the map, and the list of maps when there is a choice)
            let choice = self.map.as_ref().is_some_and(|map| crate::planetmaps::available().iter().filter(|f| f.kind.eq_ignore_ascii_case(&map.kind)).count() > 1);
            // (not yet measured, the first frame: more than they take, so the window is short for a frame and not grown —
            // a window never shrinks back, and a kept size would grow by the difference at every start)
            let rows = if self.buttons_height > 0.0 { self.buttons_height } else { 150.0 + if choice { 30.0 } else { 0.0 } };
            let map_size = (ui.available_size() - egui::vec2(0.0, rows)).max(egui::vec2(180.0, 120.0));
            let Some(map) = self.map.clone() else {
                ui.label("No planet map was found (textures/PlanetMap).");
                return;
            };
            let (rect, response) = ui.allocate_exact_size(map_size, egui::Sense::click_and_drag());
            let mid = rect.center();
            // (the character's place on this level)
            let me = own.and_then(|p| map.pixel(self.level, playfield, p[0], p[2]));
            let half = map_size * 0.5;
            // (the character kept in the middle, no nearer the level's corner than half the view: FUN_1004bb43)
            if self.follow && me.is_some() {
                self.centre = Some(on_character(me, half));
            }
            if let Some((pf, p)) = centre_on
                && let Some((x, y)) = map.pixel(self.level, pf, p[0], p[2])
            {
                self.centre = Some(egui::pos2(x, y));
            }
            let level_size = map.levels.get(self.level).map_or(egui::vec2(1.0, 1.0), |l| egui::vec2(l.size.0 as f32, l.size.1 as f32));
            let mut centre = self.centre.unwrap_or(egui::pos2(level_size.x / 2.0, level_size.y / 2.0));
            // (a grab by any button: the map goes one and a half times the mouse's way, FUN_100505d2)
            if response.dragged() {
                centre -= response.drag_delta() * 1.5;
                self.follow = false;
            }
            if response.hovered() {
                // (the wheel scrolls up and down, 10 a notch: the raw events, not the smoothed scrolling)
                let wheel: f32 = ui.input(|i| i.events.iter().map(|e| if let egui::Event::MouseWheel { delta, .. } = e { delta.y } else { 0.0 }).sum());
                let scrolled = wheeled(centre.y, wheel, half.y, level_size.y);
                if scrolled != centre.y {
                    centre.y = scrolled;
                    self.follow = false;
                }
            }
            // (a double click, left a level closer, right a level further, aimed at the point clicked: 0x1005077c)
            let double = [(egui::PointerButton::Primary, 1), (egui::PointerButton::Secondary, -1)].into_iter().find(|(button, _)| response.double_clicked_by(*button));
            if let (Some((_, by)), Some(p)) = (double, response.interact_pointer_pos().or(response.hover_pos())) {
                self.aim = fraction(&map, self.level, centre + (p - mid));
                zoom = Some(by);
            }
            self.centre = Some(centre);
            if let Some(by) = zoom {
                self.zoom(by, playfield, own, half);
                centre = self.centre.unwrap_or(centre);
            }
            // (the character's place again: the level may be another now)
            let me = own.and_then(|p| map.pixel(self.level, playfield, p[0], p[2]));
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
            let origin = mid - centre.to_vec2();
            self.tiles.draw(&painter, &map, self.level, origin, 1.0);
            // The waypoint: a white dot.
            if let Some((pf, p)) = waypoint
                && let Some((x, y)) = map.pixel(self.level, pf, p[0], p[2])
            {
                let at = origin + egui::vec2(x, y);
                painter.circle_filled(at, 5.0, egui::Color32::WHITE);
                painter.circle_stroke(at, 5.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
            }
            // (this client's own: the mission terminal an item goes back to, a blue dot)
            if let Some((pf, p)) = crate::missionlog::terminal_mark()
                && let Some((x, y)) = map.pixel(self.level, pf, p[0], p[2])
            {
                let at = origin + egui::vec2(x, y);
                painter.circle_filled(at, 5.0, crate::missionlog::TERMINAL_BLUE);
                painter.circle_stroke(at, 5.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
            }
            // The character: an arrow along its heading (north up).
            if let Some((x, y)) = me {
                let at = origin + egui::vec2(x, y);
                let (s, c) = heading.sin_cos();
                let fwd = egui::vec2(s, -c);
                let side = egui::vec2(-fwd.y, fwd.x);
                let points = vec![at + fwd * 9.0, at - fwd * 6.0 + side * 6.0, at - fwd * 3.0, at - fwd * 6.0 - side * 6.0];
                painter.add(egui::Shape::convex_polygon(points, accent, egui::Stroke::new(1.5, egui::Color32::BLACK)));
            }
            // The buttons, as the original's rows (`PlanetMap_ZoomIn`, `PlanetMap_ZoomOut`, `PlanetMap_CentreOnChar`,
            // `PlanetMap_CentreOnQuest`): each half the width.
            // (everything under the map, spacing included, is what the map leaves room for next frame)
            let buttons_top = rect.bottom();
            ui.add_space(4.0);
            let row = |ui: &mut egui::Ui, left: (&str, bool, &str), right: (&str, bool, &str)| -> (bool, bool) {
                let mut out = (false, false);
                ui.columns(2, |cols| {
                    out.0 = crate::chrome::button(&mut cols[0], palette, left.0, left.1).on_hover_text(left.2).clicked();
                    out.1 = crate::chrome::button(&mut cols[1], palette, right.0, right.1).on_hover_text(right.2).clicked();
                });
                out
            };
            // (each dead at its end of the levels: `SetLevel` 0x1004c2e2 disables it there)
            let (zoom_in, zoom_out) = row(ui, ("Zoom in", self.level + 1 < map.levels.len(), "A level closer"), ("Zoom out", self.level > 0, "A level further"));
            let (character, mission) = row(ui, ("Character", own.is_some(), "Centre on the character"), ("Mission", waypoint.is_some(), "Centre on the mission's waypoint"));
            if zoom_in {
                self.pending_zoom = Some(1);
            }
            if zoom_out {
                self.pending_zoom = Some(-1);
            }
            if character {
                self.follow = true;
            }
            if mission {
                self.pending_centre = waypoint;
                self.follow = false;
            }
            // (the maps of this planet: FUN_1004cb15's choice, `PlanetMapIndexFile` / `ShadowlandMapIndexFile`)
            if choice {
                let kind = map.kind.clone();
                let mut chosen = None;
                egui::ComboBox::from_id_salt("planet-map-choice").width(ui.available_width()).selected_text(egui::RichText::new(&map.name).small()).show_ui(ui, |ui| {
                    for f in crate::planetmaps::available().iter().filter(|f| f.kind.eq_ignore_ascii_case(&kind)) {
                        if ui.selectable_label(crate::planetmaps::is_chosen(f), &f.name).clicked() {
                            chosen = Some(f.index.clone());
                        }
                    }
                });
                if let Some(index) = chosen {
                    crate::planetmaps::choose(&index);
                    self.repick = true;
                }
            }
            self.buttons_height = (ui.min_rect().bottom() - buttons_top).max(1.0);
        });
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ScrollView_c::WheelMoved` (0x10142820) and the re-aim and the centring of `SetLevel` (FUN_1004c2e2, FUN_1004bb43).
    #[test]
    fn the_wheel_scrolls_and_a_point_is_aimed_at_the_middle() {
        // (the wheel: 10 a notch, up for a notch up, held to the level; nothing when the level fits the view)
        assert_eq!(wheeled(500.0, 1.0, 100.0, 1000.0), 490.0);
        assert_eq!(wheeled(500.0, -2.0, 100.0, 1000.0), 520.0);
        assert_eq!(wheeled(105.0, 1.0, 100.0, 1000.0), 100.0);
        assert_eq!(wheeled(895.0, -1.0, 100.0, 1000.0), 900.0);
        assert_eq!(wheeled(80.0, 1.0, 100.0, 150.0), 80.0);
        // (the character in the middle, no nearer the corner than half the view; not on the map: the corner)
        let half = egui::vec2(150.0, 100.0);
        assert_eq!(on_character(Some((400.0, 300.0)), half), egui::pos2(400.0, 300.0));
        assert_eq!(on_character(Some((40.0, 300.0)), half), egui::pos2(150.0, 300.0));
        assert_eq!(on_character(None, half), egui::pos2(150.0, 100.0));
    }

    #[test]
    fn a_map_opens_at_the_kept_level_held_to_its_levels() {
        assert_eq!(opening_level(None, 4), 0);
        assert_eq!(opening_level(Some(2), 4), 2);
        assert_eq!(opening_level(Some(7), 4), 3);
        assert_eq!(opening_level(Some(3), 1), 0);
        assert_eq!(opening_level(None, 0), 0);
    }
}
