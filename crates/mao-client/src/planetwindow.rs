//! The planet map window: GUI.dll's `PlanetMapView_c` (`#WindowPlanetMap`, FUN_1004d92d) over the map `planetmaps.rs`
//! picks for the playfield. A level is drawn tile by tile at its own size; zooming steps through the index's levels
//! (`PlanetMap_ZoomIn` / `PlanetMap_ZoomOut`) keeping the point under the pointer (or the middle) in place, a drag moves
//! the map, `PlanetMap_CentreOnChar` and `PlanetMap_CentreOnQuest` bring the character or the mission's waypoint to the
//! middle. The character is placed as FUN_1004bedd places it (`PlanetMap::pixel`), an arrow turned to its heading; the
//! waypoint uploaded from a mission is a white dot ("MissionShownAsDot").

use std::sync::Arc;

use mao_formats::planetmap::PlanetMap;

use crate::theme::Palette;

#[derive(Default)]
pub struct PlanetWindow {
    pub open: bool,
    map: Option<Arc<PlanetMap>>,
    /// The playfield the map was picked for, and the map's index (to notice a new choice).
    picked_for: Option<u32>,
    level: usize,
    /// The level's pixel in the middle of the view, and whether it follows the character.
    centre: Option<egui::Pos2>,
    follow: bool,
    tiles: crate::planetmaps::Tiles,
    /// A new map was chosen: pick again.
    repick: bool,
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

impl PlanetWindow {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.follow = true;
        }
    }

    /// Steps to another level, keeping the level's pixel `at` where it is on the screen (`offset` from the middle).
    fn zoom(&mut self, by: i32, at: Option<egui::Vec2>) {
        let Some(map) = self.map.clone() else { return };
        let new = (self.level as i32 + by).clamp(0, map.levels.len() as i32 - 1) as usize;
        if new == self.level {
            return;
        }
        let centre = self.centre.unwrap_or_default();
        let offset = at.unwrap_or(egui::Vec2::ZERO);
        let point = fraction(&map, self.level, centre + offset);
        let moved = from_fraction(&map, new, point);
        self.centre = Some(moved - offset);
        self.level = new;
    }

    /// Draws the window: `own` the character's playfield, place (server x, y, z) and heading; `waypoint` the mission's.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, playfield: u32, own: Option<[f32; 3]>, heading: f32, waypoint: Option<(u32, [f32; 3])>) {
        self.tiles.update(ctx);
        if !self.open {
            return;
        }
        if self.picked_for != Some(playfield) || std::mem::take(&mut self.repick) {
            let old = self.map.as_ref().map(|m| (m.root.clone(), m.name.clone()));
            self.map = crate::planetmaps::for_playfield(playfield);
            self.picked_for = Some(playfield);
            if self.map.as_ref().map(|m| (m.root.clone(), m.name.clone())) != old {
                // (a new map opens at its second level, or its first)
                self.level = self.map.as_ref().map_or(0, |m| 1.min(m.levels.len().saturating_sub(1)));
                self.centre = None;
                self.follow = true;
            }
        }
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut open = self.open;
        let title = self.map.as_ref().map_or("Planet Map".to_string(), |m| m.name.clone());
        crate::chrome::window(ctx, palette, "PlanetMap", &title, &mut open, |w| w.default_size(egui::vec2(720.0, 620.0)).min_size(egui::vec2(320.0, 240.0)).resizable(true), |ui| {
            let mut zoom: Option<(i32, Option<egui::Vec2>)> = None;
            let mut centre_on: Option<(u32, [f32; 3])> = None;
            ui.horizontal(|ui| {
                if crate::chrome::button(ui, palette, "-", true).on_hover_text("Zoom out").clicked() {
                    zoom = Some((-1, None));
                }
                if crate::chrome::button(ui, palette, "+", true).on_hover_text("Zoom in").clicked() {
                    zoom = Some((1, None));
                }
                if crate::chrome::button(ui, palette, "Me", own.is_some()).on_hover_text("Centre on the character").clicked() {
                    self.follow = true;
                }
                if crate::chrome::button(ui, palette, "Mission", waypoint.is_some()).on_hover_text("Centre on the mission's waypoint").clicked() {
                    centre_on = waypoint;
                    self.follow = false;
                }
                // (the maps of this planet: FUN_1004cb15's choice, `PlanetMapIndexFile` / `ShadowlandMapIndexFile`)
                if let Some(map) = &self.map {
                    ui.label(egui::RichText::new(format!("level {} of {}", self.level + 1, map.levels.len())).small().weak());
                    let kind = map.kind.clone();
                    let mut chosen = None;
                    egui::ComboBox::from_id_salt("planet-map-choice").selected_text(egui::RichText::new(&map.name).small()).show_ui(ui, |ui| {
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
            });
            let Some(map) = self.map.clone() else {
                ui.label("No planet map was found (textures/PlanetMap).");
                return;
            };
            let (rect, response) = ui.allocate_exact_size(ui.available_size().max(egui::vec2(200.0, 150.0)), egui::Sense::click_and_drag());
            let mid = rect.center();
            // (the character's place on this level)
            let me = own.and_then(|p| map.pixel(self.level, playfield, p[0], p[2]));
            if self.follow
                && let Some((x, y)) = me
            {
                self.centre = Some(egui::pos2(x, y));
            }
            if let Some((pf, p)) = centre_on
                && let Some((x, y)) = map.pixel(self.level, pf, p[0], p[2])
            {
                self.centre = Some(egui::pos2(x, y));
            }
            let level_size = map.levels.get(self.level).map_or(egui::vec2(1.0, 1.0), |l| egui::vec2(l.size.0 as f32, l.size.1 as f32));
            let mut centre = self.centre.unwrap_or(egui::pos2(level_size.x / 2.0, level_size.y / 2.0));
            if response.dragged() {
                centre -= response.drag_delta();
                self.follow = false;
            }
            if response.hovered() {
                // (a notch of the wheel is one level: the raw events, not the smoothed scrolling)
                let wheel: f32 = ui.input(|i| i.events.iter().map(|e| if let egui::Event::MouseWheel { delta, .. } = e { delta.y } else { 0.0 }).sum());
                if wheel.abs() > 0.5 && zoom.is_none() {
                    let at = response.hover_pos().map(|p| p - mid);
                    zoom = Some((if wheel > 0.0 { 1 } else { -1 }, at));
                }
            }
            self.centre = Some(centre);
            if let Some((by, at)) = zoom {
                if by != 0 {
                    self.follow = self.follow && at.is_none();
                }
                self.zoom(by, at);
                centre = self.centre.unwrap_or(centre);
            }
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
            // The character: an arrow along its heading (north up).
            if let Some((x, y)) = me {
                let at = origin + egui::vec2(x, y);
                let (s, c) = heading.sin_cos();
                let fwd = egui::vec2(s, -c);
                let side = egui::vec2(-fwd.y, fwd.x);
                let points = vec![at + fwd * 9.0, at - fwd * 6.0 + side * 6.0, at - fwd * 3.0, at - fwd * 6.0 - side * 6.0];
                painter.add(egui::Shape::convex_polygon(points, accent, egui::Stroke::new(1.5, egui::Color32::BLACK)));
            }
            // (where the pointer is, in the playfield's coordinates, as the original's status line)
            if let Some(p) = response.hover_pos()
                && let Some((wx, wz)) = map.world(self.level, playfield, p.x - origin.x, p.y - origin.y)
            {
                painter.text(rect.left_bottom() + egui::vec2(6.0, -6.0), egui::Align2::LEFT_BOTTOM, format!("{wx:.0} x {wz:.0}"), egui::FontId::proportional(12.0), egui::Color32::from_gray(220));
            }
        });
        self.open = open;
    }
}
