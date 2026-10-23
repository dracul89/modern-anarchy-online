//! The map, in the style of Diablo 2 and 3's: nothing is drawn until you have walked past it, walls are bright
//! outlines on a faint floor, and there is a small map in the corner that turns with the camera and a large
//! translucent overlay (toggled with a key) that sits over the game view.
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
}

/// The network thread's side: classifies cells as they come into reach.
pub struct MapBuilder {
    grid: Grid,
    /// The height range of the ground, to scale the shading.
    low: f32,
    span: f32,
    fresh: bool,
}

impl MapBuilder {
    /// For a playfield with ground: the whole ground's extent. For an interior (none): a box around `at`.
    pub fn new(terrain: Option<&Terrain>, at: [f32; 3]) -> MapBuilder {
        match terrain {
            Some(t) => {
                let extent = [(t.width - 1) as f32 * t.tile_size, (t.height - 1) as f32 * t.tile_size];
                let (lo, hi) = t.heights.iter().fold((u16::MAX, 0u16), |(l, h), &v| (l.min(v), h.max(v)));
                let to_world = t.height_scale / 256.0;
                MapBuilder { grid: Grid::new([0.0, 0.0], extent), low: lo as f32 * to_world, span: ((hi - lo) as f32 * to_world).max(1.0), fresh: true }
            }
            None => MapBuilder { grid: Grid::new([at[0] - 300.0, at[2] - 300.0], [600.0, 600.0]), low: at[1] - 20.0, span: 40.0, fresh: true },
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
        self.fresh = false;
        (reset.is_some() || !done.is_empty()).then_some(MapPatch { reset, cells: done })
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
}

impl Default for AutoMap {
    fn default() -> Self {
        AutoMap { grid: None, texture: None, dirty: false, last_upload: std::time::Instant::now(), overlay: false, mini_span: 150.0, zoom: 3.0, pan: [0.0; 2] }
    }
}

impl AutoMap {
    pub fn apply(&mut self, patch: MapPatch) {
        if let Some(grid) = patch.reset {
            self.grid = Some(grid);
            self.texture = None;
            self.pan = [0.0; 2];
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
    fn paint(&self, painter: &egui::Painter, rect: egui::Rect, center: [f32; 2], px_per_unit: f32, yaw: Option<f32>) {
        let (Some(grid), Some(texture)) = (&self.grid, &self.texture) else { return };
        let (fwd, right) = Self::basis(yaw);
        let corner = |u: f32, v: f32| -> ([f32; 2], egui::Pos2) {
            let world = [center[0] + (right[0] * u + fwd[0] * -v) / px_per_unit, center[1] + (right[1] * u + fwd[1] * -v) / px_per_unit];
            ([world[0], world[1]], egui::pos2((world[0] - grid.origin[0]) / grid.cell / grid.w as f32, (world[1] - grid.origin[1]) / grid.cell / grid.h as f32))
        };
        let (hw, hh) = (rect.width() / 2.0, rect.height() / 2.0);
        let mut mesh = egui::Mesh::with_texture(texture.id());
        for (u, v, pos) in [(-hw, -hh, rect.left_top()), (hw, -hh, rect.right_top()), (hw, hh, rect.right_bottom()), (-hw, hh, rect.left_bottom())] {
            mesh.vertices.push(egui::epaint::Vertex { pos, uv: corner(u, v).1, color: egui::Color32::WHITE });
        }
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
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

    fn dots(&self, painter: &egui::Painter, rect: egui::Rect, center: [f32; 2], px_per_unit: f32, yaw: Option<f32>, me: ([f32; 2], f32), dots: &[([f32; 2], DotKind)], booths: &[[f32; 2]]) {
        let inside = |p: egui::Pos2| rect.shrink(4.0).contains(p);
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
        if self.overlay {
            // Over the whole view, north up, panned by the mouse and zoomed with the wheel.
            egui::Area::new(egui::Id::new("automap-overlay")).order(egui::Order::Background).fixed_pos(screen.min).show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::drag());
                if response.dragged() {
                    let d = response.drag_delta();
                    self.pan = [self.pan[0] - d.x / self.zoom, self.pan[1] + d.y / self.zoom];
                }
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    self.zoom = (self.zoom * (1.0 + scroll * 0.002)).clamp(0.6, 12.0);
                }
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_black_alpha(110));
                let center = [pos[0] + self.pan[0], pos[1] + self.pan[1]];
                self.paint(ui.painter(), rect, center, self.zoom, None);
                self.dots(ui.painter(), rect, center, self.zoom, None, (pos, heading), dots, booths);
                ui.painter().text(rect.left_top() + egui::vec2(16.0, 14.0), egui::Align2::LEFT_TOP, place, egui::FontId::proportional(18.0), egui::Color32::from_rgb(255, 205, 70));
                ui.painter().text(rect.left_top() + egui::vec2(16.0, 38.0), egui::Align2::LEFT_TOP, "Drag to pan, wheel to zoom, O to close, Space re-centres", egui::FontId::proportional(12.0), egui::Color32::from_white_alpha(150));
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.pan = [0.0; 2];
            }
        } else {
            let size = 210.0;
            let rect = egui::Rect::from_min_size(egui::pos2(screen.right() - size - 12.0, 44.0), egui::vec2(size, size));
            egui::Area::new(egui::Id::new("automap-mini")).order(egui::Order::Background).fixed_pos(rect.min).interactable(false).show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(rect.size(), egui::Sense::hover());
                ui.painter().rect_filled(rect, 6.0, egui::Color32::from_black_alpha(120));
                let clipped = ui.painter().with_clip_rect(rect);
                let px = size / self.mini_span;
                self.paint(&clipped, rect, pos, px, Some(yaw));
                self.dots(&clipped, rect, pos, px, Some(yaw), (pos, heading), dots, booths);
                ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 170, 210)), egui::StrokeKind::Inside);
            });
        }
    }
}
