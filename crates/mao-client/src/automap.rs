//! The map. For a playfield on the original's planet map (`mao_formats::planetmap`: the `PlanetMap` window's tiles and
//! coordinates, placed as GUI.dll's `PlanetMapView_c` places them) the corner map and the overlay show that picture.
//! Elsewhere (interiors, instanced playfields) the map is the client's own, in the style of Diablo 2 and 3's: nothing is
//! drawn until you have walked past it, walls are bright outlines on a faint floor. The small map in the corner turns
//! with the camera; the large translucent overlay (toggled with a key) sits over the game view.
//!
//! The network thread owns the facts (the ground heights, the collision): [`MapBuilder`] reveals the cells
//! around the character as it walks, classifies them (floor or wall) and sends the new cells; the interface
//! keeps its own copy of the grid ([`AutoMap`]), paints it into a texture and draws the corner map and the overlay.

use mao_formats::collision::Collision;
use mao_formats::terrain::Terrain;

/// Known, wall, and a height in 0..=255 across the playfield's range, in a `u16`.
const KNOWN: u16 = 0x8000;
const WALL: u16 = 0x4000;

/// How far around the character the map is revealed, in world units.
const REVEAL: f32 = 75.0;

/// The most cells classified in one step (the collision test costs something).
const BUDGET: usize = 3000;

/// A cell grid over a playfield.
#[derive(Clone, Debug)]
pub struct Grid {
    pub origin: [f32; 2],
    pub cell: f32,
    pub w: usize,
    pub h: usize,
    pub cells: Vec<u16>,
}

impl Grid {
    fn new(origin: [f32; 2], extent: [f32; 2]) -> Grid {
        let cell = (extent[0].max(extent[1]) / 1024.0).max(2.0);
        let (w, h) = ((extent[0] / cell).ceil() as usize + 1, (extent[1] / cell).ceil() as usize + 1);
        Grid { origin, cell, w, h, cells: vec![0; w * h] }
    }

    pub fn index(&self, x: f32, z: f32) -> Option<usize> {
        let (ix, iz) = (((x - self.origin[0]) / self.cell).floor(), ((z - self.origin[1]) / self.cell).floor());
        (ix >= 0.0 && iz >= 0.0 && (ix as usize) < self.w && (iz as usize) < self.h).then(|| iz as usize * self.w + ix as usize)
    }
}

/// What the interface needs to follow the builder: a whole new grid (a new playfield) or some new cells.
pub struct MapPatch {
    pub reset: Option<Grid>,
    pub cells: Vec<(u32, u16)>,
    /// The playfield, with a new grid (to find it on the planet map).
    pub playfield: Option<u32>,
}

/// The network thread's side: classifies cells as they come into reach.
pub struct MapBuilder {
    grid: Grid,
    playfield: u32,
    /// The height range of the ground, to scale the shading.
    low: f32,
    span: f32,
    fresh: bool,
}

impl MapBuilder {
    /// For a playfield with ground: the whole ground's extent. For an interior (none): a box around `at`.
    pub fn new(terrain: Option<&Terrain>, at: [f32; 3], playfield: u32) -> MapBuilder {
        match terrain {
            Some(t) => {
                let extent = [(t.width - 1) as f32 * t.tile_size, (t.height - 1) as f32 * t.tile_size];
                let (lo, hi) = t.heights.iter().fold((u16::MAX, 0u16), |(l, h), &v| (l.min(v), h.max(v)));
                let to_world = t.height_scale / 256.0;
                MapBuilder { grid: Grid::new([0.0, 0.0], extent), playfield, low: lo as f32 * to_world, span: ((hi - lo) as f32 * to_world).max(1.0), fresh: true }
            }
            None => MapBuilder { grid: Grid::new([at[0] - 300.0, at[2] - 300.0], [600.0, 600.0]), playfield, low: at[1] - 20.0, span: 40.0, fresh: true },
        }
    }

    /// Reveals the cells around `at` (x, height, z); returns what changed. `ground` is the ground's height at a
    /// place (plus the offset the server's heights sit at), when there is ground.
    pub fn reveal(&mut self, at: [f32; 3], terrain: Option<&Terrain>, offset: f32, collision: &Collision) -> Option<MapPatch> {
        let g = &self.grid;
        let reach = (REVEAL / g.cell).ceil() as i64;
        let (cx, cz) = (((at[0] - g.origin[0]) / g.cell) as i64, ((at[2] - g.origin[1]) / g.cell) as i64);
        let mut done = Vec::new();
        'rows: for dz in -reach..=reach {
            for dx in -reach..=reach {
                if dx * dx + dz * dz > reach * reach {
                    continue;
                }
                let (ix, iz) = (cx + dx, cz + dz);
                if ix < 0 || iz < 0 || ix as usize >= g.w || iz as usize >= g.h {
                    continue;
                }
                let index = iz as usize * g.w + ix as usize;
                if g.cells[index] & KNOWN != 0 {
                    continue;
                }
                let (x, z) = (g.origin[0] + (ix as f32 + 0.5) * g.cell, g.origin[1] + (iz as f32 + 0.5) * g.cell);
                let height = terrain.map_or(at[1], |t| t.height_at(x, z, t.height_scale) + offset);
                // A cliff (the ground rising more than a step to a neighbour) or something solid is a wall.
                let steep = terrain.is_some_and(|t| {
                    let h = |px: f32, pz: f32| t.height_at(px, pz, t.height_scale) + offset;
                    let limit = g.cell * 1.2;
                    [(g.cell, 0.0), (-g.cell, 0.0), (0.0, g.cell), (0.0, -g.cell)].iter().any(|(ox, oz)| (h(x + ox, z + oz) - height).abs() > limit)
                });
                let solid = collision.blocked(x, z, height, g.cell * 0.5, 2.0, 0.8);
                let shade = (((height - self.low) / self.span).clamp(0.0, 1.0) * 255.0) as u16;
                done.push((index as u32, KNOWN | if steep || solid { WALL } else { 0 } | shade));
                if done.len() >= BUDGET {
                    break 'rows;
                }
            }
        }
        for &(i, v) in &done {
            self.grid.cells[i as usize] = v;
        }
        let reset = self.fresh.then(|| self.grid.clone());
        let playfield = self.fresh.then_some(self.playfield);
        self.fresh = false;
        (reset.is_some() || !done.is_empty()).then_some(MapPatch { reset, cells: done, playfield })
    }
}

/// A dot on the map: where (x, z) and what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DotKind {
    Player,
    Friendly,
    Hostile,
    Target,
}

/// The interface's side: the grid copy, its texture, and the corner map and overlay.
pub struct AutoMap {
    grid: Option<Grid>,
    texture: Option<egui::TextureHandle>,
    dirty: bool,
    last_upload: std::time::Instant,
    /// The big overlay is shown.
    pub overlay: bool,
    /// World units across the corner map, and pixels per unit on the overlay.
    pub mini_span: f32,
    pub zoom: f32,
    /// Where the overlay is panned to from the character (world units).
    pan: [f32; 2],
    /// Where the corner map's top left is (kept in `automap.ini`); `None` is the top right of the window.
    pub mini_at: Option<[f32; 2]>,
    /// The original's planet map (its biggest level as a texture), drawn in place of the grid for a playfield that is
    /// on it; tried once.
    planet: Option<Planet>,
    /// The playfield the planet map was picked for (`planetmaps::for_playfield`: the chosen Rubi-Ka or Shadowlands map).
    planet_tried: Option<u32>,
    playfield: u32,
    /// The planet map's own scales (it has about 21 world units to a pixel): pixels a unit on the overlay, and world
    /// units across the corner map.
    planet_zoom: f32,
    planet_span: f32,
}

/// The planet map's pixels to a world unit, on its biggest level.
const PLANET_NATIVE: f32 = 890.0 / mao_formats::planetmap::EXTENT.0;

struct Planet {
    map: mao_formats::planetmap::PlanetMap,
    level: usize,
    texture: egui::TextureHandle,
    size: [f32; 2],
}

impl Default for AutoMap {
    fn default() -> Self {
        AutoMap { grid: None, texture: None, dirty: false, last_upload: std::time::Instant::now(), overlay: false, mini_span: 150.0, zoom: 3.0, pan: [0.0; 2], mini_at: load_position(), planet: None, planet_tried: None, playfield: 0, planet_zoom: PLANET_NATIVE * 2.0, planet_span: 2500.0 }
    }
}

fn position_path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("automap.ini")
}

fn load_position() -> Option<[f32; 2]> {
    let text = std::fs::read_to_string(position_path()).ok()?;
    let v: Vec<f32> = text.split_whitespace().filter_map(|t| t.parse().ok()).collect();
    (v.len() >= 2).then(|| [v[0], v[1]])
}

fn save_position(at: [f32; 2]) {
    let path = position_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, format!("{:.0} {:.0}
", at[0], at[1]));
}

impl AutoMap {
    pub fn apply(&mut self, patch: MapPatch) {
        if let Some(grid) = patch.reset {
            self.grid = Some(grid);
            self.texture = None;
            self.pan = [0.0; 2];
        }
        if let Some(pf) = patch.playfield {
            self.playfield = pf;
        }
        if let Some(grid) = &mut self.grid {
            for (i, v) in patch.cells {
                if let Some(c) = grid.cells.get_mut(i as usize) {
                    *c = v;
                }
            }
            self.dirty = true;
        }
    }

    /// The grid painted as an image: unknown cells clear, floor as a faint tinted glow, walls as bright outlines.
    fn image(grid: &Grid) -> egui::ColorImage {
        let known = |x: i64, y: i64| -> Option<u16> { (x >= 0 && y >= 0 && (x as usize) < grid.w && (y as usize) < grid.h).then(|| grid.cells[y as usize * grid.w + x as usize]).filter(|c| c & KNOWN != 0) };
        let mut pixels = vec![egui::Color32::TRANSPARENT; grid.w * grid.h];
        for y in 0..grid.h {
            for x in 0..grid.w {
                let c = grid.cells[y * grid.w + x];
                if c & KNOWN == 0 {
                    continue;
                }
                let (xi, yi) = (x as i64, y as i64);
                pixels[y * grid.w + x] = if c & WALL == 0 {
                    // Floor: dark glass with the height as a little light.
                    let s = (c & 0xff) as u8 / 6;
                    egui::Color32::from_rgba_unmultiplied(28 + s, 52 + s, 78 + s, 120)
                } else if [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| known(xi + dx, yi + dy).is_some_and(|n| n & WALL == 0)) {
                    // A wall next to floor: the bright outline.
                    egui::Color32::from_rgb(190, 225, 255)
                } else {
                    egui::Color32::from_rgba_unmultiplied(10, 16, 28, 90)
                };
            }
        }
        egui::ColorImage { size: [grid.w, grid.h], source_size: egui::vec2(grid.w as f32, grid.h as f32), pixels }
    }

    /// Brings the texture up to date (a few times a second at most).
    pub fn refresh(&mut self, ctx: &egui::Context) {
        if self.planet_tried != Some(self.playfield) {
            self.planet_tried = Some(self.playfield);
            let chosen = crate::planetmaps::for_playfield(self.playfield);
            let same = |p: &Planet| chosen.as_ref().is_some_and(|c| c.root == p.map.root && c.name == p.map.name);
            if !self.planet.as_ref().is_some_and(same)
                && let (Some(map), Some(game)) = (chosen, crate::theme::game_dir())
            {
                // (one picture: the biggest level that fits a texture of 2048 a side)
                let level = (0..map.levels.len()).rev().find(|&l| map.levels[l].size.0 <= 2048 && map.levels[l].size.1 <= 2048).unwrap_or(0);
                if let Some(image) = map.image(game, level) {
                    let (w, h) = image.dimensions();
                    let texture = ctx.load_texture("planet-map", egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], image.as_raw()), egui::TextureOptions::LINEAR);
                    self.planet = Some(Planet { map: (*map).clone(), level, texture, size: [w as f32, h as f32] });
                }
            }
        }
        let Some(grid) = &self.grid else { return };
        if self.texture.is_some() && (!self.dirty || self.last_upload.elapsed() < std::time::Duration::from_millis(250)) {
            return;
        }
        let image = Self::image(grid);
        match &mut self.texture {
            Some(t) => t.set(image, egui::TextureOptions::LINEAR),
            None => self.texture = Some(ctx.load_texture("automap", image, egui::TextureOptions::LINEAR)),
        }
        self.dirty = false;
        self.last_upload = std::time::Instant::now();
    }

    /// Paints the map into `rect`: the world point `center` in its middle, `px_per_unit` pixels to a world unit,
    /// turned so that the camera's way (`yaw`) is up, or north up without one.
    fn paint(&self, painter: &egui::Painter, rect: egui::Rect, center: [f32; 2], px_per_unit: f32, yaw: Option<f32>, circle: bool) {
        // The planet map when the playfield is on it, else the walked grid.
        let planet = self.planet.as_ref().filter(|p| p.map.pixel(p.level, self.playfield, 0.0, 0.0).is_some());
        let (texture, grid) = match (planet, &self.grid, &self.texture) {
            (Some(p), _, _) => (&p.texture, None),
            (None, Some(g), Some(t)) => (t, Some(g)),
            _ => return,
        };
        let (fwd, right) = Self::basis(yaw);
        let corner = |u: f32, v: f32| -> ([f32; 2], egui::Pos2) {
            let world = [center[0] + (right[0] * u + fwd[0] * -v) / px_per_unit, center[1] + (right[1] * u + fwd[1] * -v) / px_per_unit];
            let uv = match (planet, grid) {
                (Some(p), _) => {
                    let (px, py) = p.map.pixel(p.level, self.playfield, world[0], world[1]).unwrap_or((0.0, 0.0));
                    egui::pos2(px / p.size[0], py / p.size[1])
                }
                (None, Some(grid)) => egui::pos2((world[0] - grid.origin[0]) / grid.cell / grid.w as f32, (world[1] - grid.origin[1]) / grid.cell / grid.h as f32),
                _ => egui::pos2(0.0, 0.0),
            };
            ([world[0], world[1]], uv)
        };
        let (hw, hh) = (rect.width() / 2.0, rect.height() / 2.0);
        let mut mesh = egui::Mesh::with_texture(texture.id());
        if circle {
            // A fan of triangles round the middle: the map shows through a round window.
            let radius = hw.min(hh);
            mesh.vertices.push(egui::epaint::Vertex { pos: rect.center(), uv: corner(0.0, 0.0).1, color: egui::Color32::WHITE });
            const SEGMENTS: usize = 72;
            for k in 0..=SEGMENTS {
                let a = k as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                let (u, v) = (a.cos() * radius, a.sin() * radius);
                mesh.vertices.push(egui::epaint::Vertex { pos: rect.center() + egui::vec2(u, v), uv: corner(u, v).1, color: egui::Color32::WHITE });
                if k > 0 {
                    mesh.indices.extend_from_slice(&[0, k as u32, k as u32 + 1]);
                }
            }
        } else {
            for (u, v, pos) in [(-hw, -hh, rect.left_top()), (hw, -hh, rect.right_top()), (hw, hh, rect.right_bottom()), (-hw, hh, rect.left_bottom())] {
                mesh.vertices.push(egui::epaint::Vertex { pos, uv: corner(u, v).1, color: egui::Color32::WHITE });
            }
            mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        }
        painter.add(egui::Shape::mesh(mesh));
    }

    /// The map's up and right as world directions (x, z).
    fn basis(yaw: Option<f32>) -> ([f32; 2], [f32; 2]) {
        match yaw {
            // The camera looks from (cos yaw, sin yaw) at the character: forward is the other way.
            Some(y) => ([-y.cos(), -y.sin()], [-y.sin(), y.cos()]),
            None => ([0.0, 1.0], [1.0, 0.0]),
        }
    }

    /// A world point on the screen.
    fn to_screen(rect: egui::Rect, center: [f32; 2], px_per_unit: f32, yaw: Option<f32>, p: [f32; 2]) -> egui::Pos2 {
        let (fwd, right) = Self::basis(yaw);
        let d = [(p[0] - center[0]) * px_per_unit, (p[1] - center[1]) * px_per_unit];
        rect.center() + egui::vec2(d[0] * right[0] + d[1] * right[1], -(d[0] * fwd[0] + d[1] * fwd[1]))
    }

    #[allow(clippy::too_many_arguments)]
    fn dots(&self, painter: &egui::Painter, rect: egui::Rect, center: [f32; 2], px_per_unit: f32, yaw: Option<f32>, me: ([f32; 2], f32), dots: &[([f32; 2], DotKind)], booths: &[[f32; 2]], circle: bool) {
        let inside = |p: egui::Pos2| if circle { (p - rect.center()).length() < rect.width().min(rect.height()) / 2.0 - 5.0 } else { rect.shrink(4.0).contains(p) };
        for b in booths {
            let p = Self::to_screen(rect, center, px_per_unit, yaw, *b);
            if inside(p) {
                painter.circle_filled(p, 4.5, egui::Color32::from_rgb(60, 200, 220));
                painter.circle_stroke(p, 6.5, egui::Stroke::new(1.0, egui::Color32::WHITE));
            }
        }
        for (pos, kind) in dots {
            let p = Self::to_screen(rect, center, px_per_unit, yaw, *pos);
            if !inside(p) {
                continue;
            }
            let colour = match kind {
                DotKind::Player => egui::Color32::from_rgb(90, 160, 255),
                DotKind::Friendly => egui::Color32::from_rgb(110, 220, 120),
                DotKind::Hostile => egui::Color32::from_rgb(230, 70, 60),
                DotKind::Target => egui::Color32::from_rgb(255, 220, 60),
            };
            painter.circle_filled(p, if *kind == DotKind::Target { 4.5 } else { 3.0 }, colour);
        }
        // The character: a gold arrow pointing the way it faces.
        let (fwd, right) = Self::basis(yaw);
        let dir = [me.1.sin(), me.1.cos()];
        let (u, v) = (dir[0] * right[0] + dir[1] * right[1], dir[0] * fwd[0] + dir[1] * fwd[1]);
        let tip = egui::vec2(u, -v);
        let side = egui::vec2(-tip.y, tip.x);
        let c = Self::to_screen(rect, center, px_per_unit, yaw, me.0);
        painter.add(egui::Shape::convex_polygon(vec![c + tip * 9.0, c - tip * 5.0 + side * 5.5, c - tip * 2.0, c - tip * 5.0 - side * 5.5], egui::Color32::from_rgb(255, 205, 70), egui::Stroke::new(1.0, egui::Color32::BLACK)));
    }

    /// The corner map and, when switched on, the overlay.
    #[allow(clippy::too_many_arguments)]
    pub fn show(&mut self, ctx: &egui::Context, me: Option<([f32; 2], f32)>, yaw: f32, dots: &[([f32; 2], DotKind)], booths: &[[f32; 2]], place: &str) {
        self.refresh(ctx);
        let Some((pos, heading)) = me else { return };
        let screen = ctx.content_rect();
        let planet = self.planet.as_ref().is_some_and(|p| p.map.pixel(p.level, self.playfield, 0.0, 0.0).is_some());
        if self.overlay {
            // Over the whole view, north up, panned by the mouse and zoomed with the wheel.
            egui::Area::new(egui::Id::unique("automap-overlay")).order(egui::Order::Background).fixed_pos(screen.min).show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::drag());
                let zoom = if planet { self.planet_zoom } else { self.zoom };
                if response.dragged() {
                    let d = response.drag_delta();
                    self.pan = [self.pan[0] - d.x / zoom, self.pan[1] + d.y / zoom];
                }
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    if planet {
                        self.planet_zoom = (self.planet_zoom * (1.0 + scroll * 0.002)).clamp(PLANET_NATIVE * 0.5, PLANET_NATIVE * 8.0);
                    } else {
                        self.zoom = (self.zoom * (1.0 + scroll * 0.002)).clamp(0.6, 12.0);
                    }
                }
                let zoom = if planet { self.planet_zoom } else { self.zoom };
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_black_alpha(110));
                let center = [pos[0] + self.pan[0], pos[1] + self.pan[1]];
                self.paint(ui.painter(), rect, center, zoom, None, false);
                self.dots(ui.painter(), rect, center, zoom, None, (pos, heading), dots, booths, false);
                ui.painter().text(rect.left_top() + egui::vec2(16.0, 14.0), egui::Align2::LEFT_TOP, place, egui::FontId::proportional(18.0), egui::Color32::from_rgb(255, 205, 70));
                ui.painter().text(rect.left_top() + egui::vec2(16.0, 38.0), egui::Align2::LEFT_TOP, "Drag to pan, wheel to zoom, O to close, Space re-centres", egui::FontId::proportional(12.0), egui::Color32::from_white_alpha(150));
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.pan = [0.0; 2];
            }
        } else {
            // The corner map: round, turning with the camera, and movable (drag it; the wheel over it zooms).
            let size = 220.0;
            let at = self.mini_at.map_or(egui::pos2(screen.right() - size - 12.0, 44.0), |p| egui::pos2(p[0], p[1]));
            let at = egui::pos2(at.x.clamp(screen.left(), (screen.right() - size).max(screen.left())), at.y.clamp(screen.top(), (screen.bottom() - size).max(screen.top())));
            let area = egui::Area::new(egui::Id::unique("automap-mini")).order(egui::Order::Background).current_pos(at).movable(true).show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
                let (c, r) = (rect.center(), size / 2.0);
                ui.painter().circle_filled(c, r, egui::Color32::from_black_alpha(130));
                let px = size / if planet { self.planet_span } else { self.mini_span };
                self.paint(ui.painter(), rect, pos, px, Some(yaw), true);
                self.dots(ui.painter(), rect, pos, px, Some(yaw), (pos, heading), dots, booths, true);
                ui.painter().circle_stroke(c, r, egui::Stroke::new(2.5, egui::Color32::from_rgb(120, 170, 210)));
                ui.painter().circle_stroke(c, r + 1.5, egui::Stroke::new(1.0, egui::Color32::from_black_alpha(160)));
                // North (the map's +z) is marked on the ring.
                let north = Self::to_screen(rect, pos, 1.0, Some(yaw), [pos[0], pos[1] + 1.0]) - c;
                let n = if north.length() > 0.0 { north.normalized() } else { egui::vec2(0.0, -1.0) };
                ui.painter().text(c + n * (r - 9.0), egui::Align2::CENTER_CENTER, "N", egui::FontId::proportional(12.0), egui::Color32::from_rgb(255, 205, 70));
                if response.hovered() {
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll != 0.0 {
                        if planet {
                            self.planet_span = (self.planet_span * (1.0 - scroll * 0.002)).clamp(600.0, 8000.0);
                        } else {
                            self.mini_span = (self.mini_span * (1.0 - scroll * 0.002)).clamp(50.0, 600.0);
                        }
                    }
                }
            });
            if area.response.dragged() || area.response.drag_stopped() {
                let min = area.response.rect.min;
                self.mini_at = Some([min.x, min.y]);
                if area.response.drag_stopped() {
                    save_position([min.x, min.y]);
                }
            }
        }
    }
}
