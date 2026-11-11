//! The map. For a playfield with ground the corner map and the overlay show the ground itself from above, as the
//! original's playfield map does: GUI.dll's `PFMapGroundRenderer_c` (FUN_10047964) draws each ground patch's own
//! texture (`AnarchyGroundData_t::CreateHiTexture`) over the patch's shadow texture lightened with white
//! ([`ground_picture`]: the tiles' textures laid out cell by cell, dimmed by that veil; today's game files have no
//! shadow maps for the shadow textures, see `AutoMap::paint`). The planet map's
//! picture (`mao_formats::planetmap`) is the planet map window's, and stands in here only when a playfield on it has
//! no ground picture.
//! Elsewhere (interiors, instanced playfields) the map is the client's own, in the style of Diablo 2 and 3's: nothing is
//! drawn until you have walked past it, walls are bright outlines on a faint floor. The small map in the corner turns
//! with the camera; the large translucent overlay (toggled with a key) sits over the game view.
//!
//! The network thread owns the facts (the ground heights, the collision): [`MapBuilder`] reveals the cells
//! around the character as it walks, classifies them (floor or wall) and sends the new cells; the interface
//! keeps its own copy of the grid ([`AutoMap`]), paints it into a texture and draws the corner map and the overlay.

use mao_formats::collision::Collision;
use mao_net::landcontrol::Tower;
use mao_formats::terrain::Terrain;

/// Known, wall, and a height in 0..=255 across the playfield's range, in a `u16`.
const KNOWN: u16 = 0x8000;
const WALL: u16 = 0x4000;

/// The white veil the original's playfield map lays under the ground's textures (`PFMapGroundRenderer_c`, FUN_10047964:
/// a quad of 0xbbffffff blended over the shadow textures): its strength.
const VEIL: u8 = 0xbb;

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
    /// The ground from above, with a new grid of a playfield that has ground.
    pub ground: Option<Ground>,
}

/// A playfield's ground from above: the picture and the world units it spans (from the world's origin).
pub struct Ground {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
    pub extent: [f32; 2],
}

/// The ground's picture: each cell's tile texture (the terrain's tile word: its index's texture, turned by its top two
/// bits as the ground's model turns it) shrunk to a few pixels a side, so that the whole ground is at most 2048 pixels
/// across. `tile` gives a ground texture by its instance: width, height and RGBA.
pub fn ground_picture(terrain: &Terrain, mut tile: impl FnMut(u32) -> Option<(usize, usize, Vec<u8>)>) -> Ground {
    let (cw, ch) = (terrain.width - 1, terrain.height - 1);
    let side = [8usize, 4, 2, 1].into_iter().find(|s| cw.max(ch) * s <= 2048).unwrap_or(1);
    let (w, h) = (cw * side, ch * side);
    let mut rgba = vec![0u8; w * h * 4];
    // (a tile's texture as a block of `side` by `side`, each pixel the mean of its part of the texture)
    let mut blocks: std::collections::HashMap<u16, Vec<[u8; 3]>> = std::collections::HashMap::new();
    for cy in 0..ch {
        for cx in 0..cw {
            let word = terrain.tiles[cy * cw + cx];
            let index = word & terrain.mask;
            let block = blocks.entry(index).or_insert_with(|| {
                let instance = terrain.tile_types.get(index as usize).copied().unwrap_or(index) as u32;
                match tile(instance).filter(|t| t.0 > 0 && t.1 > 0 && t.2.len() >= t.0 * t.1 * 4) {
                    Some((tw, th, pixels)) => (0..side * side)
                        .map(|k| {
                            let (bx, by) = (k % side, k / side);
                            let (x0, x1, y0, y1) = (bx * tw / side, ((bx + 1) * tw / side).max(bx * tw / side + 1).min(tw), by * th / side, ((by + 1) * th / side).max(by * th / side + 1).min(th));
                            let mut sum = [0u32; 3];
                            for y in y0..y1 {
                                for x in x0..x1 {
                                    for c in 0..3 {
                                        sum[c] += pixels[(y * tw + x) * 4 + c] as u32;
                                    }
                                }
                            }
                            let n = ((x1 - x0) * (y1 - y0)).max(1) as u32;
                            [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
                        })
                        .collect(),
                    None => vec![[70, 70, 70]; side * side],
                }
            });
            for sy in 0..side {
                for sx in 0..side {
                    // (the place in the cell, and the texture's by the tile's turn: `Terrain::model`'s four)
                    let (ox, oy) = ((sx as f32 + 0.5) / side as f32, (sy as f32 + 0.5) / side as f32);
                    let (u, v) = match word & 0xc000 {
                        0 => (ox, 1.0 - oy),
                        0x4000 => (1.0 - oy, 1.0 - ox),
                        0x8000 => (1.0 - ox, oy),
                        _ => (oy, ox),
                    };
                    let (bx, by) = (((u * side as f32) as usize).min(side - 1), ((v * side as f32) as usize).min(side - 1));
                    let at = ((cy * side + sy) * w + cx * side + sx) * 4;
                    rgba[at..at + 3].copy_from_slice(&block[by * side + bx]);
                    rgba[at + 3] = 255;
                }
            }
        }
    }
    Ground { size: [w, h], rgba, extent: [cw as f32 * terrain.tile_size, ch as f32 * terrain.tile_size] }
}

/// The network thread's side: classifies cells as they come into reach.
pub struct MapBuilder {
    grid: Grid,
    playfield: u32,
    /// The ground's picture, until the first patch takes it along.
    ground: Option<Ground>,
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
                MapBuilder { grid: Grid::new([0.0, 0.0], extent), playfield, ground: None, low: lo as f32 * to_world, span: ((hi - lo) as f32 * to_world).max(1.0), fresh: true }
            }
            None => MapBuilder { grid: Grid::new([at[0] - 300.0, at[2] - 300.0], [600.0, 600.0]), playfield, ground: None, low: at[1] - 20.0, span: 40.0, fresh: true },
        }
    }

    /// The ground's picture, for the first patch to take to the interface.
    pub fn with_ground(mut self, ground: Option<Ground>) -> MapBuilder {
        self.ground = ground;
        self
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
        let ground = if reset.is_some() { self.ground.take() } else { None };
        (reset.is_some() || !done.is_empty()).then_some(MapPatch { reset, cells: done, playfield, ground })
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
    /// The corner map's diameter in points (kept with its place; dragged at its lower right, or Ctrl and the wheel).
    pub mini_size: f32,
    /// The playfield's ground from above (the original's playfield map): the picture waiting to be a texture, the
    /// texture, and the world units it spans.
    ground_image: Option<egui::ColorImage>,
    ground: Option<(egui::TextureHandle, [f32; 2])>,
    ground_extent: [f32; 2],
    /// The original's planet map (its biggest level as a texture), drawn in place of the grid for a playfield that is
    /// on it and has no ground picture; tried once.
    planet: Option<Planet>,
    /// The playfield the planet map was picked for (`planetmaps::for_playfield`: the chosen Rubi-Ka or Shadowlands map).
    planet_tried: Option<u32>,
    playfield: u32,
    /// The planet map's own scales (it has about 21 world units to a pixel): pixels a unit on the overlay, and world
    /// units across the corner map.
    planet_zoom: f32,
    planet_span: f32,
    /// The map control's land-control button (`RenderLandControlData`, off until switched on; kept between sessions as
    /// the original archives it, FUN_100ebfdc, and reads it back building the control, FUN_100ecdf7).
    pub land_control: bool,
    /// The playfield's land-control map as the picture `PFMapRenderer_c` draws over the map (FUN_100ebc3f), with whether it
    /// was coloured for a playfield with a map picture.
    land_map: Option<LandMap>,
    /// The character's district, its name and land control level range (`N3Msg_GetLCDistrictInfo`).
    lc_district: Option<(String, i32, i32)>,
    /// The map control's land-control button (MapControlView_c's `Info` toggle) as a GUI view tree: the tree, its root and
    /// the button.
    controls: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id)>,
    land_source: Option<std::sync::Arc<mao_formats::landcontrolmap::LandControlMap>>,
}

struct LandMap {
    source: std::sync::Arc<mao_formats::landcontrolmap::LandControlMap>,
    on_planet: bool,
    texture: egui::TextureHandle,
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
        AutoMap { grid: None, texture: None, dirty: false, last_upload: std::time::Instant::now(), overlay: false, mini_span: 150.0, zoom: 3.0, pan: [0.0; 2], mini_at: load_position(), mini_size: load_size(), ground_image: None, ground: None, ground_extent: [0.0; 2], planet: None, planet_tried: None, playfield: 0, planet_zoom: PLANET_NATIVE * 2.0, planet_span: 2500.0, land_control: load_land_control(), land_map: None, lc_district: None, controls: None, land_source: None }
    }
}

fn position_path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("automap.ini")
}

fn load_position() -> Option<[f32; 2]> {
    let text = std::fs::read_to_string(position_path()).ok()?;
    let v: Vec<f32> = text.lines().next()?.split_whitespace().filter_map(|t| t.parse().ok()).collect();
    (v.len() >= 2).then(|| [v[0], v[1]])
}

/// The corner map's diameter as saved (`Size <n>`, a line of the map's file), else 220.
fn load_size() -> f32 {
    std::fs::read_to_string(position_path()).ok().and_then(|t| t.lines().find_map(|l| l.trim().strip_prefix("Size ")?.trim().parse().ok())).unwrap_or(220.0)
}

/// The land-control button as saved (`RenderLandControlData`, the second line of the map's file).
fn load_land_control() -> bool {
    std::fs::read_to_string(position_path()).is_ok_and(|t| t.lines().any(|l| l.trim() == "RenderLandControlData 1"))
}

/// The corner map's place and the land-control button, saved together.
fn save_prefs(at: Option<[f32; 2]>, land_control: bool, size: f32) {
    let path = position_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let place = at.map(|at| format!("{:.0} {:.0}\n", at[0], at[1])).unwrap_or_default();
    let _ = std::fs::write(path, format!("{place}RenderLandControlData {}\nSize {size:.0}\n", land_control as u8));
}

impl AutoMap {
    pub fn apply(&mut self, patch: MapPatch) {
        if let Some(grid) = patch.reset {
            self.grid = Some(grid);
            self.texture = None;
            self.pan = [0.0; 2];
            // (a new playfield: its ground's picture, or none for one without ground)
            self.ground = None;
            self.ground_image = patch.ground.map(|g| {
                self.ground_extent = g.extent;
                egui::ColorImage::from_rgba_unmultiplied(g.size, &g.rgba)
            });
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
        if let Some(image) = self.ground_image.take() {
            self.ground = Some((ctx.load_texture("playfield-map", image, egui::TextureOptions::LINEAR), self.ground_extent));
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
        // The ground from above when the playfield has ground; else the planet map when the playfield is on it, else the
        // walked grid.
        if let Some((texture, extent)) = &self.ground {
            let uv = |world: [f32; 2]| egui::pos2(world[0] / extent[0], world[1] / extent[1]);
            let mut mesh = Self::window_mesh(texture.id(), rect, center, px_per_unit, yaw, circle, uv);
            // (the original multiplies the ground over what its first two passes left: the patches' shadow textures
            // under a white veil at 0xbb. The shadow maps they are made of, resource type 0xf4247 with six a
            // playfield for the sun's places, are not in today's game files, so no patch has a shadow texture and
            // what is left is the veil over the map's black: the ground at 0xbb of its brightness)
            for v in &mut mesh.vertices {
                v.color = egui::Color32::from_gray(VEIL);
            }
            painter.add(egui::Shape::mesh(mesh));
            return;
        }
        let planet = self.planet.as_ref().filter(|p| p.map.pixel(p.level, self.playfield, 0.0, 0.0).is_some());
        let (texture, grid) = match (planet, &self.grid, &self.texture) {
            (Some(p), _, _) => (&p.texture, None),
            (None, Some(g), Some(t)) => (t, Some(g)),
            _ => return,
        };
        let uv = |world: [f32; 2]| match (planet, grid) {
            (Some(p), _) => {
                let (px, py) = p.map.pixel(p.level, self.playfield, world[0], world[1]).unwrap_or((0.0, 0.0));
                egui::pos2(px / p.size[0], py / p.size[1])
            }
            (None, Some(grid)) => egui::pos2((world[0] - grid.origin[0]) / grid.cell / grid.w as f32, (world[1] - grid.origin[1]) / grid.cell / grid.h as f32),
            _ => egui::pos2(0.0, 0.0),
        };
        painter.add(egui::Shape::mesh(Self::window_mesh(texture.id(), rect, center, px_per_unit, yaw, circle, uv)));
    }

    /// The map window's mesh, a texture laid over the world: a quad over `rect`, or a fan of triangles round its middle (the
    /// map showing through a round window), each corner's texture place from its world place by `uv`.
    #[allow(clippy::too_many_arguments)]
    fn window_mesh(texture: egui::TextureId, rect: egui::Rect, center: [f32; 2], px_per_unit: f32, yaw: Option<f32>, circle: bool, uv: impl Fn([f32; 2]) -> egui::Pos2) -> egui::Mesh {
        let (fwd, right) = Self::basis(yaw);
        let corner = |u: f32, v: f32| -> ([f32; 2], egui::Pos2) {
            let world = [center[0] + (right[0] * u + fwd[0] * -v) / px_per_unit, center[1] + (right[1] * u + fwd[1] * -v) / px_per_unit];
            (world, uv(world))
        };
        let (hw, hh) = (rect.width() / 2.0, rect.height() / 2.0);
        let mut mesh = egui::Mesh::with_texture(texture);
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
        mesh
    }

    /// The playfield's land-control map and the character's district for the land-control view; the picture is made again
    /// for a new map.
    pub fn set_land_control(&mut self, map: Option<&std::sync::Arc<mao_formats::landcontrolmap::LandControlMap>>, district: Option<&(String, i32, i32)>) {
        let same = match (&self.land_source, map) {
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
            (a, b) => a.is_none() && b.is_none(),
        };
        if !same {
            self.land_source = map.cloned();
            self.land_map = None;
        }
        self.lc_district = district.cloned();
    }

    /// FUN_100ebc3f's picture of the land-control map (A4R4G4B4): a tile a tower may stand on `0xf392`, green; any other
    /// `0xf555`, grey, or on a playfield with a map picture (the map data's +0x80) `0x4000`, a faint black. The rows run
    /// north from row 0 at z 0 (the original fills its 128 x 128 window bottom up); a clear texel round the edge keeps the
    /// world outside the map uncovered.
    fn land_image(map: &mao_formats::landcontrolmap::LandControlMap, on_planet: bool) -> egui::ColorImage {
        let nibbles = |v: u16| egui::Color32::from_rgba_unmultiplied(((v >> 8) & 15) as u8 * 17, ((v >> 4) & 15) as u8 * 17, (v & 15) as u8 * 17, (v >> 12) as u8 * 17);
        let (set, clear) = (nibbles(0xf392), nibbles(if on_planet { 0x4000 } else { 0xf555 }));
        let (w, h) = (map.width as usize + 2, map.height as usize + 2);
        let mut pixels = vec![egui::Color32::TRANSPARENT; w * h];
        for z in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                pixels[(z as usize + 1) * w + x as usize + 1] = if map.placeable(x, z) { set } else { clear };
            }
        }
        egui::ColorImage { size: [w, h], source_size: egui::vec2(w as f32, h as f32), pixels }
    }

    /// The land-control map over the map (FUN_100ea88a with the land-control button on: the picture over the window, normal
    /// alpha blending, the state block at +0x600), each tile 4 world units a side.
    #[allow(clippy::too_many_arguments)]
    fn land(&mut self, painter: &egui::Painter, rect: egui::Rect, center: [f32; 2], px_per_unit: f32, yaw: Option<f32>, circle: bool) {
        if !self.land_control {
            return;
        }
        let on_planet = self.ground.is_some() || self.planet.as_ref().is_some_and(|p| p.map.pixel(p.level, self.playfield, 0.0, 0.0).is_some());
        if self.land_map.as_ref().is_some_and(|m| m.on_planet != on_planet) {
            self.land_map = None;
        }
        if self.land_map.is_none()
            && let Some(source) = self.land_source.clone()
        {
            // (no filtering is set for it: Direct3D's default, the nearest texel)
            let texture = painter.ctx().load_texture("land-control-map", Self::land_image(&source, on_planet), egui::TextureOptions::NEAREST);
            self.land_map = Some(LandMap { source, on_planet, texture });
        }
        let Some(map) = &self.land_map else { return };
        let tile = mao_formats::landcontrolmap::TILE;
        let (w, h) = (map.source.width as f32 + 2.0, map.source.height as f32 + 2.0);
        let uv = |world: [f32; 2]| egui::pos2((world[0] / tile + 1.0) / w, (world[1] / tile + 1.0) / h);
        painter.add(egui::Shape::mesh(Self::window_mesh(map.texture.id(), rect, center, px_per_unit, yaw, circle, uv)));
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

    /// The Notum Wars tower sites as GUI.dll's `PFMapRenderer_c` draws them (FUN_100ea243): first each tower's 16 x 16 icon
    /// at its place, then, with the land-control button on (`+0x47a`, FUN_100ebac9) and the land-control map's picture made
    /// (`+0x4b4`), an 80-pixel circle over each. Both keep their size whatever the zoom (FUN_100467e8 sizes the quad in
    /// screen pixels). Their vertex colours carry alpha 0 (`0xffffff`, `0xffff04`), which the state block they are drawn
    /// with (+0x57c, set up in the constructor FUN_100eb7d5) never reads: alpha is the texture's alone (stage 0 `ALPHAOP`
    /// `SELECTARG1` of `TEXTURE`), colour the texture's times the vertex's, blended `ONE`, `INVSRCALPHA`. The skin's circle is
    /// white on the green key, so it draws opaque in the tint, as here.
    #[allow(clippy::too_many_arguments)]
    fn towers(&self, painter: &egui::Painter, rect: egui::Rect, center: [f32; 2], px_per_unit: f32, yaw: Option<f32>, towers: &[Tower], circle: bool) {
        let Some(skin) = crate::skin::skin(painter.ctx()) else { return };
        let inside = |p: egui::Pos2| if circle { (p - rect.center()).length() < rect.width().min(rect.height()) / 2.0 - 5.0 } else { rect.shrink(4.0).contains(p) };
        let places: Vec<(egui::Pos2, &Tower)> = towers.iter().map(|t| (Self::to_screen(rect, center, px_per_unit, yaw, [t.position[0], t.position[2]]), t)).filter(|(p, _)| inside(*p)).collect();
        for (p, t) in &places {
            skin.draw_in(painter, tower_sprite(t.side, t.class), egui::Rect::from_center_size(*p, egui::vec2(16.0, 16.0)), egui::Color32::WHITE);
        }
        if self.land_control && self.land_map.is_some() {
            // (the 128 x 128 texture holds the 80 x 80 `GFX_GUI_MAP_CIRCLE` at its corner; 82 x 82 of it is drawn centred)
            let painter = painter.with_clip_rect(rect);
            for (p, t) in &places {
                skin.draw_in(&painter, "GFX_GUI_MAP_CIRCLE", egui::Rect::from_min_size(*p - egui::vec2(41.0, 41.0), egui::vec2(80.0, 80.0)), circle_tint(t.class));
            }
        }
    }

    /// FUN_100eb04b's text in the land-control view (the TextView at 5, 5 of the map renderer, HTML): the district's name and
    /// `Lvl Rng: <low> - <high>`, nothing unless both ends are 1 or more. Only with the button on (FUN_100ebac9 makes the
    /// view and starts its half-second timer; switched off the view goes).
    fn district_text(&self) -> Option<String> {
        if !self.land_control {
            return None;
        }
        let (name, low, high) = self.lc_district.as_ref()?;
        (*low >= 1 && *high >= 1).then(|| format!("{name}\nLvl Rng: {low} - {high}"))
    }

    /// MapControlView_c's land-control button (FUN_100ecdf7): a toggle `Button_c` labelled with text.mdb's 10000 `Info`, its
    /// tip `ShowLConMap` ("Show land control areas on the map"), set from the archived `RenderLandControlData`, disabled
    /// without Notum Wars (stat 389 bit 0); toggling it switches the renderer's land control (FUN_100ec034 -> FUN_100ebac9).
    /// The control's `Build` and `War` buttons (the land control build and war menus) are not ported. Returns the bar's
    /// bottom.
    fn control_bar(&mut self, ui: &mut egui::Ui, at: egui::Pos2, notum_wars: bool, palette: &crate::theme::Palette) -> f32 {
        use crate::gui::view::{Layout, Node, Plain, Tree};
        let (tree, root, button) = self.controls.get_or_insert_with(|| {
            let text = |name: &str| crate::feedback::texts().and_then(|t| t.named(10000, name)).unwrap_or(name).to_string();
            let mut tree = Tree::default();
            let root = tree.add(Node::new("", Box::new(Plain)));
            tree.get_mut(root).layout = Layout::Horizontal;
            let mut b = crate::gui::controls::Button::new(&text("Info"));
            b.toggle = true;
            let mut node = Node::new("LandControl", Box::new(b));
            node.tooltip = text("ShowLConMap");
            let button = tree.add(node);
            tree.add_child(root, button);
            (tree, root, button)
        });
        let (root, button) = (*root, *button);
        if let Some(b) = tree.control_mut::<crate::gui::controls::Button>(button) {
            b.on = self.land_control;
        }
        tree.get_mut(button).enabled = notum_wars;
        tree.measure = crate::gui::font::Measure { ctx: Some(ui.ctx().clone()) };
        let size = crate::gui::host::preferred_pixels(tree, root, false);
        let skin = crate::skin::skin(ui.ctx());
        let bar = egui::Rect::from_min_size(at, size);
        ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| crate::gui::host::show(ui, tree, root, skin.as_deref(), palette));
        for (id, signal) in std::mem::take(&mut tree.signals) {
            if id == button
                && let crate::gui::host::Signal::Toggled(on) = signal
                && on != self.land_control
            {
                self.land_control = on;
                save_prefs(self.mini_at, on, self.mini_size);
            }
        }
        bar.bottom()
    }

    /// The corner map and, when switched on, the overlay. `notum_wars`: the account has the expansion (stat 389, bit 0), which
    /// the land-control button needs (MapControlView_c, FUN_100ecdf7, disables it otherwise).
    #[allow(clippy::too_many_arguments)]
    pub fn show(&mut self, ctx: &egui::Context, me: Option<([f32; 2], f32)>, yaw: f32, dots: &[([f32; 2], DotKind)], booths: &[[f32; 2]], towers: &[Tower], notum_wars: bool, place: &str, palette: &crate::theme::Palette) {
        self.refresh(ctx);
        let Some((pos, heading)) = me else { return };
        let screen = ctx.content_rect();
        // (the planet map's own scales only while its picture stands in)
        let planet = self.ground.is_none() && self.planet.as_ref().is_some_and(|p| p.map.pixel(p.level, self.playfield, 0.0, 0.0).is_some());
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
                self.land(ui.painter(), rect, center, zoom, None, false);
                self.towers(ui.painter(), rect, center, zoom, None, towers, false);
                self.dots(ui.painter(), rect, center, zoom, None, (pos, heading), dots, booths, false);
                ui.painter().text(rect.left_top() + egui::vec2(16.0, 14.0), egui::Align2::LEFT_TOP, place, egui::FontId::proportional(18.0), egui::Color32::from_rgb(255, 205, 70));
                ui.painter().text(rect.left_top() + egui::vec2(16.0, 38.0), egui::Align2::LEFT_TOP, "Drag to pan, wheel to zoom, O to close, Space re-centres", egui::FontId::proportional(12.0), egui::Color32::from_white_alpha(150));
                // The land-control toggle (the map control's button, `RenderLandControlData`), on Notum Wars playfields,
                // and under it the character's district.
                if !towers.is_empty() || self.land_source.is_some() {
                    let at = rect.left_top() + egui::vec2(16.0, 58.0);
                    let bottom = self.control_bar(ui, at, notum_wars, palette);
                    if let Some(text) = self.district_text() {
                        ui.painter().text(egui::pos2(at.x, bottom + 6.0), egui::Align2::LEFT_TOP, text, egui::FontId::proportional(13.0), egui::Color32::WHITE);
                    }
                }
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.pan = [0.0; 2];
            }
        } else {
            // The corner map: round, turning with the camera, movable (drag it; the wheel over it zooms) and sizeable (drag
            // the handle at its lower right, or Ctrl and the wheel).
            let size = self.mini_size.clamp(120.0, 600.0);
            // (left of the rollup column when windows are docked there)
            let at = self.mini_at.map_or(egui::pos2(screen.right() - size - 12.0 - crate::rollup::column_width(), 44.0), |p| egui::pos2(p[0], p[1]));
            let at = egui::pos2(at.x.clamp(screen.left(), (screen.right() - size).max(screen.left())), at.y.clamp(screen.top(), (screen.bottom() - size).max(screen.top())));
            let mut resized = false;
            let area = egui::Area::new(egui::Id::unique("automap-mini")).order(egui::Order::Background).current_pos(at).movable(true).show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
                let (c, r) = (rect.center(), size / 2.0);
                ui.painter().circle_filled(c, r, egui::Color32::from_black_alpha(130));
                let px = size / if planet { self.planet_span } else { self.mini_span };
                self.paint(ui.painter(), rect, pos, px, Some(yaw), true);
                self.land(ui.painter(), rect, pos, px, Some(yaw), true);
                self.towers(ui.painter(), rect, pos, px, Some(yaw), towers, true);
                self.dots(ui.painter(), rect, pos, px, Some(yaw), (pos, heading), dots, booths, true);
                ui.painter().circle_stroke(c, r, egui::Stroke::new(2.5, egui::Color32::from_rgb(120, 170, 210)));
                ui.painter().circle_stroke(c, r + 1.5, egui::Stroke::new(1.0, egui::Color32::from_black_alpha(160)));
                // North (the map's +z) is marked on the ring.
                let north = Self::to_screen(rect, pos, 1.0, Some(yaw), [pos[0], pos[1] + 1.0]) - c;
                let n = if north.length() > 0.0 { north.normalized() } else { egui::vec2(0.0, -1.0) };
                ui.painter().text(c + n * (r - 9.0), egui::Align2::CENTER_CENTER, "N", egui::FontId::proportional(12.0), egui::Color32::from_rgb(255, 205, 70));
                if response.hovered() {
                    let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.ctrl));
                    if scroll != 0.0 && ctrl {
                        self.mini_size = (self.mini_size * (1.0 + scroll * 0.002)).clamp(120.0, 600.0);
                        resized = true;
                    } else if scroll != 0.0 {
                        if planet {
                            self.planet_span = (self.planet_span * (1.0 - scroll * 0.002)).clamp(600.0, 8000.0);
                        } else {
                            self.mini_span = (self.mini_span * (1.0 - scroll * 0.002)).clamp(50.0, 600.0);
                        }
                    }
                }
                // the size handle on the ring's lower right
                let handle = egui::Rect::from_center_size(c + egui::vec2(r * 0.72, r * 0.72), egui::vec2(16.0, 16.0));
                let grip = ui.interact(handle, ui.scope_id().with("map-size"), egui::Sense::drag());
                let tint = if grip.hovered() || grip.dragged() { egui::Color32::WHITE } else { egui::Color32::from_rgb(120, 170, 210) };
                if !crate::skin::skin(ui.ctx()).is_some_and(|s| s.draw_in(ui.painter(), "GFX_GUI_WINDOW_CORNER_DRAG", handle, tint)) {
                    ui.painter().rect_stroke(handle.shrink(4.0), 2.0, egui::Stroke::new(1.5, tint), egui::StrokeKind::Inside);
                }
                if grip.dragged() {
                    let d = grip.drag_delta();
                    self.mini_size = (self.mini_size + (d.x + d.y) * 0.5).clamp(120.0, 600.0);
                }
                if grip.drag_stopped() {
                    resized = true;
                }
                grip.on_hover_text("Drag to size the map (Ctrl and the wheel too)");
            });
            if area.response.dragged() || area.response.drag_stopped() {
                let min = area.response.rect.min;
                self.mini_at = Some([min.x, min.y]);
                if area.response.drag_stopped() {
                    save_prefs(Some([min.x, min.y]), self.land_control, self.mini_size);
                }
            }
            if resized {
                save_prefs(self.mini_at, self.land_control, self.mini_size);
            }
        }
    }
}

/// A tower's map icon (FUN_100ea243). The 64 x 64 atlas FUN_100ea4b9 builds holds twelve GUI textures in 16-pixel cells,
/// `0x10a 0x10b 0x10c 0x111 0x113 0x112 0x114 0x116 0x115 0x10d 0x10f 0x10e`; a tower takes cell `[4, 7, 10, 3, 6, 9, 5, 8,
/// 11][side * 3 + kind]`, its side Clan (1) 0, Omni (2) 1, else 2, its kind a control tower (1, and any unknown) 0, a turret
/// (2, 5, 6) 1, a conductor (3, 4) 2. A GUI texture id indexes AFCM.dll's table of GFX names (the pointers at RVA 0x16060,
/// id 0x8a `GFX_GUI_CHECKBOX_CHECKED`): 0x10a `GFX_GUI_MAP_ARROW`, 0x10b `ARROW_SELECTED`, 0x10c `BIG_ARROW`, 0x10d-0x10f
/// `BUFFTOWER_BLUE`, `_GREY`, `_RED`, 0x110 `CIRCLE` (the 128-pixel texture's), 0x111-0x113 `CONTROLLER_BLUE`, `_GREY`,
/// `_RED`, 0x114-0x116 `GUARDTOWER_BLUE`, `_GREY`, `_RED`: Clan red, Omni blue and the unclaimed grey.
pub fn tower_sprite(side: i32, class: i32) -> &'static str {
    let colour = match side {
        1 => 0,
        2 => 1,
        _ => 2,
    };
    let kind = match class {
        2 | 5 | 6 => 1,
        3 | 4 => 2,
        _ => 0,
    };
    [
        ["GFX_GUI_MAP_CONTROLLER_RED", "GFX_GUI_MAP_GUARDTOWER_RED", "GFX_GUI_MAP_BUFFTOWER_RED"],
        ["GFX_GUI_MAP_CONTROLLER_BLUE", "GFX_GUI_MAP_GUARDTOWER_BLUE", "GFX_GUI_MAP_BUFFTOWER_BLUE"],
        ["GFX_GUI_MAP_CONTROLLER_GREY", "GFX_GUI_MAP_GUARDTOWER_GREY", "GFX_GUI_MAP_BUFFTOWER_GREY"],
    ][colour][kind]
}

/// The land-control circle's colour (FUN_100ea243, its second loop, on the tower's kind): a control tower or a turret (1, 2)
/// `0xffff04`, anything else `0x00dcff`.
pub fn circle_tint(class: i32) -> egui::Color32 {
    if matches!(class, 1 | 2) { egui::Color32::from_rgb(0xff, 0xff, 0x04) } else { egui::Color32::from_rgb(0x00, 0xdc, 0xff) }
}

#[cfg(test)]
mod tower_tests {
    use super::*;

    #[test]
    fn tower_icons_follow_the_atlas_cells() {
        assert_eq!(tower_sprite(1, 1), "GFX_GUI_MAP_CONTROLLER_RED");
        assert_eq!(tower_sprite(2, 2), "GFX_GUI_MAP_GUARDTOWER_BLUE");
        assert_eq!(tower_sprite(0, 4), "GFX_GUI_MAP_BUFFTOWER_GREY");
        assert_eq!(tower_sprite(7, 9), "GFX_GUI_MAP_CONTROLLER_GREY");
        assert_eq!(circle_tint(4), egui::Color32::from_rgb(0, 0xdc, 0xff));
    }

    #[test]
    fn the_skin_has_the_tower_sprites() {
        // (the game's Default skin; skipped without MAO_GAME)
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let layer = crate::skin::Layer::open(&std::path::Path::new(&game).join("cd_image/gui/Default")).unwrap();
        for side in 0..3 {
            for class in [1, 2, 4] {
                let (_, size) = crate::skin::decode(layer.bytes(tower_sprite(side, class)).unwrap()).unwrap();
                assert_eq!(size, [16, 16]);
            }
        }
        assert_eq!(crate::skin::decode(layer.bytes("GFX_GUI_MAP_CIRCLE").unwrap()).unwrap().1, [80, 80]);
    }

    /// AFCM.dll's GFX names by GUI texture id: the table of name pointers at RVA 0x16060, id 0x8a at
    /// `GFX_GUI_CHECKBOX_CHECKED`.
    fn gfx_names(afcm: &[u8]) -> Vec<String> {
        let u16_at = |o: usize| u16::from_le_bytes(afcm[o..o + 2].try_into().unwrap()) as usize;
        let u32_at = |o: usize| u32::from_le_bytes(afcm[o..o + 4].try_into().unwrap()) as usize;
        let pe = u32_at(0x3c);
        let (sections, optional) = (u16_at(pe + 6), u16_at(pe + 20));
        let base = u32_at(pe + 24 + 28);
        let offset = |rva: usize| {
            (0..sections).map(|i| pe + 24 + optional + i * 40).find_map(|h| {
                let (va, size, raw) = (u32_at(h + 12), u32_at(h + 16), u32_at(h + 20));
                (va..va + size).contains(&rva).then(|| rva - va + raw)
            })
        };
        let mut names = Vec::new();
        let mut at = offset(0x16060).unwrap();
        while let Some(o) = u32_at(at).checked_sub(base).and_then(offset) {
            let end = afcm[o..].iter().position(|&b| b == 0).unwrap();
            match std::str::from_utf8(&afcm[o..o + end]) {
                Ok(n) if n.starts_with("GFX_") => names.push(n.to_string()),
                _ => break,
            }
            at += 4;
        }
        names
    }

    #[test]
    fn the_atlas_ids_name_the_tower_sprites() {
        // (the game's AFCM.dll; skipped without MAO_GAME)
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let names = gfx_names(&std::fs::read(std::path::Path::new(&game).join("AFCM.dll")).unwrap());
        let first = names.iter().position(|n| n == "GFX_GUI_CHECKBOX_CHECKED").unwrap();
        let id = |gui: usize| names[gui - 0x8a + first].as_str();
        // (FUN_100ea4b9's twelve cells, and the cell each side and kind takes in FUN_100ea243)
        let cells = [0x10a, 0x10b, 0x10c, 0x111, 0x113, 0x112, 0x114, 0x116, 0x115, 0x10d, 0x10f, 0x10e];
        let pick = [4, 7, 10, 3, 6, 9, 5, 8, 11];
        for (side, colour) in [(1, 0), (2, 1), (0, 2)] {
            for (class, kind) in [(1, 0), (2, 1), (3, 2)] {
                assert_eq!(id(cells[pick[colour * 3 + kind]]), tower_sprite(side, class));
            }
        }
        assert_eq!(id(0x110), "GFX_GUI_MAP_CIRCLE");
    }

    #[test]
    fn the_land_control_picture_has_the_originals_colours() {
        let map = mao_formats::landcontrolmap::LandControlMap { width: 3, height: 2, bits: vec![0b001, 0b100] };
        let on_planet = AutoMap::land_image(&map, true);
        assert_eq!(on_planet.size, [5, 4]);
        // (a clear edge; 0xf392 on the set tiles, 0x4000 elsewhere)
        assert_eq!(on_planet.pixels[0], egui::Color32::TRANSPARENT);
        assert_eq!(on_planet.pixels[5 + 1], egui::Color32::from_rgb(0x33, 0x99, 0x22));
        assert_eq!(on_planet.pixels[5 + 2], egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0x44));
        assert_eq!(on_planet.pixels[2 * 5 + 3], egui::Color32::from_rgb(0x33, 0x99, 0x22));
        assert_eq!(AutoMap::land_image(&map, false).pixels[5 + 2], egui::Color32::from_rgb(0x55, 0x55, 0x55));
    }

    #[test]
    fn the_ground_picture_lays_the_tiles_out_and_turns_them() {
        // (two cells side by side: tile 0 as it is, tile 1 turned a half; the textures are 2 by 2, left half red and
        // right half blue in the first, all green in the second)
        let terrain = Terrain { width: 3, height: 2, heights: vec![0; 6], tiles: vec![0, 0x8000 | 1], tile_types: vec![10, 11], height_scale: 1.0, mask: 0x3fff, tile_size: 4.0 };
        let ground = ground_picture(&terrain, |instance| {
            let (r, b, g) = ([255, 0, 0, 255], [0, 0, 255, 255], [0, 255, 0, 255]);
            Some((2, 2, if instance == 10 { [r, b, r, b].concat() } else { [g, g, g, g].concat() }))
        });
        assert_eq!((ground.size, ground.extent), ([16, 8], [8.0, 4.0]));
        let pixel = |x: usize, y: usize| ground.rgba[(y * 16 + x) * 4..(y * 16 + x) * 4 + 3].to_vec();
        // (the first cell: red at its left, blue at its right; the second: green)
        assert_eq!((pixel(1, 3), pixel(6, 3), pixel(12, 3)), (vec![255, 0, 0], vec![0, 0, 255], vec![0, 255, 0]));
        // (a tile without a texture is grey, and a big ground gets fewer pixels a cell)
        let bare = ground_picture(&Terrain { width: 601, height: 2, heights: vec![0; 1202], tiles: vec![0; 600], tile_types: vec![10], height_scale: 1.0, mask: 0x3fff, tile_size: 4.0 }, |_| None);
        assert_eq!((bare.size, &bare.rgba[..4]), ([1200, 2], &[70, 70, 70, 255][..]));
    }

    #[test]
    fn the_district_shows_with_a_level_range_only() {
        let mut map = AutoMap { land_control: true, lc_district: Some(("jucha".into(), 60, 80)), ..AutoMap::default() };
        assert_eq!(map.district_text().as_deref(), Some("jucha\nLvl Rng: 60 - 80"));
        map.lc_district = Some(("Somewhere".into(), 0, 0));
        assert_eq!(map.district_text(), None);
        map.lc_district = Some(("jucha".into(), 60, 80));
        map.land_control = false;
        assert_eq!(map.district_text(), None);
    }
}
