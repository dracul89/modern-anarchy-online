//! The planet maps to use: GUI.dll's `PlanetMapIndexFile` (Rubi-Ka) and `ShadowlandMapIndexFile` options name an index file
//! under `textures/PlanetMap/` (FUN_1004cb15 lists every index there by its `Type` for the choice). Here the maps' folders
//! are the client's own `PlanetMap` folder (beside `mao.ini`: the aftermarket maps the client brings) and then the game's
//! `cd_image/textures/PlanetMap`; the defaults are Saavick's Map of Rubi-Ka and SLmap. A playfield is shown on the
//! Rubi-Ka map when it has a place there, else on the Shadowlands one.
//!
//! The tiles (128 px PNGs in the maps' `.bin` files) are decoded on a thread of their own and kept as textures while used
//! ([`Tiles`]): the big levels have thousands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use mao_formats::planetmap::{Found, PlanetMap};

/// The maps' folders, the client's first.
pub fn roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = crate::settings::Settings::path().parent() {
        out.push(dir.join("PlanetMap"));
    }
    if let Some(game) = crate::theme::game_dir() {
        out.push(game.join("cd_image").join("textures").join("PlanetMap"));
    }
    out
}

/// Every map found (once).
pub fn available() -> &'static [Found] {
    static FOUND: std::sync::OnceLock<Vec<Found>> = std::sync::OnceLock::new();
    FOUND.get_or_init(|| mao_formats::planetmap::discover(&roots()))
}

/// The two maps in use (Rubi-Ka's and the Shadowlands'), with the index they were opened from.
#[derive(Default)]
struct Chosen {
    rubika: Option<(String, Arc<PlanetMap>)>,
    shadowlands: Option<(String, Arc<PlanetMap>)>,
}

static CHOSEN: Mutex<Chosen> = Mutex::new(Chosen { rubika: None, shadowlands: None });

/// The map of an index (any of the folders), opened.
fn open(index: &str) -> Option<Arc<PlanetMap>> {
    let found = available().iter().find(|f| f.index.eq_ignore_ascii_case(index))?;
    PlanetMap::open(&found.root, &found.index).map(Arc::new)
}

/// The index to use for a kind (`Rubika`, `Shadowlands`): the option's, else the default, else the first of that kind.
fn index_for(kind: &str, wanted: &str) -> Option<String> {
    let of_kind = |f: &&Found| f.kind.eq_ignore_ascii_case(kind);
    let default = if kind.eq_ignore_ascii_case("Rubika") { crate::settings::DEFAULT_PLANET_MAP } else { crate::settings::DEFAULT_SHADOWLANDS_MAP };
    [wanted, default]
        .iter()
        .find_map(|w| available().iter().filter(of_kind).find(|f| f.index.eq_ignore_ascii_case(w)))
        .or_else(|| available().iter().find(of_kind))
        .map(|f| f.index.clone())
}

/// The map in use for a kind, opened again when the option changed.
fn chosen(kind: &str) -> Option<Arc<PlanetMap>> {
    let settings = crate::settings::Settings::load();
    let rubika = kind.eq_ignore_ascii_case("Rubika");
    let index = index_for(kind, if rubika { &settings.planet_map } else { &settings.shadowlands_map })?;
    let mut c = CHOSEN.lock().ok()?;
    let slot = if rubika { &mut c.rubika } else { &mut c.shadowlands };
    if slot.as_ref().is_none_or(|(i, _)| *i != index) {
        *slot = open(&index).map(|m| (index.clone(), m));
    }
    slot.as_ref().map(|(_, m)| m.clone())
}

/// The map a playfield is shown on: Rubi-Ka's when it has the playfield, else the Shadowlands' when it has it, else
/// Rubi-Ka's.
pub fn for_playfield(playfield: u32) -> Option<Arc<PlanetMap>> {
    let rubika = chosen("Rubika");
    if rubika.as_ref().is_some_and(|m| m.places.contains_key(&playfield)) {
        return rubika;
    }
    let shadowlands = chosen("Shadowlands");
    if shadowlands.as_ref().is_some_and(|m| m.places.contains_key(&playfield)) {
        return shadowlands;
    }
    rubika.or(shadowlands)
}

/// Uses another map for its kind (the option saved).
pub fn choose(index: &str) {
    let Some(found) = available().iter().find(|f| f.index.eq_ignore_ascii_case(index)) else { return };
    let mut settings = crate::settings::Settings::load();
    if found.kind.eq_ignore_ascii_case("Shadowlands") {
        settings.shadowlands_map = found.index.clone();
    } else {
        settings.planet_map = found.index.clone();
    }
    settings.save();
}

/// Whether an index is the one in use for its kind.
pub fn is_chosen(found: &Found) -> bool {
    let settings = crate::settings::Settings::load();
    let wanted = if found.kind.eq_ignore_ascii_case("Shadowlands") { settings.shadowlands_map } else { settings.planet_map };
    index_for(&found.kind, &wanted).is_some_and(|i| i.eq_ignore_ascii_case(&found.index))
}

/// A tile: the map (by its folder and first level's file), the level and the tile's place in its list.
type Key = (PathBuf, String, usize, usize);

fn key(map: &PlanetMap, level: usize, index: usize) -> Key {
    (map.root.clone(), map.levels.first().map_or(String::new(), |l| l.file.clone()), level, index)
}

/// The tiles as textures, decoded on a thread of their own.
pub struct Tiles {
    textures: HashMap<Key, (egui::TextureHandle, u64)>,
    asked: std::collections::HashSet<Key>,
    to_thread: std::sync::mpsc::Sender<(Key, Arc<PlanetMap>)>,
    from_thread: std::sync::mpsc::Receiver<(Key, Option<image::RgbaImage>)>,
    frame: u64,
}

/// How many tiles are kept (about 64 MB of 128 px tiles).
const KEPT: usize = 1000;

impl Default for Tiles {
    fn default() -> Tiles {
        let (to_thread, requests) = std::sync::mpsc::channel::<(Key, Arc<PlanetMap>)>();
        let (done, from_thread) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok((key, map)) = requests.recv() {
                let image = map.tile(key.2, key.3);
                if done.send((key, image)).is_err() {
                    break;
                }
            }
        });
        Tiles { textures: HashMap::new(), asked: Default::default(), to_thread, from_thread, frame: 0 }
    }
}

impl Tiles {
    /// Takes in what the thread decoded (call once a frame).
    pub fn update(&mut self, ctx: &egui::Context) {
        self.frame += 1;
        let mut got = false;
        while let Ok((key, image)) = self.from_thread.try_recv() {
            self.asked.remove(&key);
            if let Some(image) = image {
                let (w, h) = image.dimensions();
                let texture = ctx.load_texture(format!("planet-tile-{}-{}-{}", key.1, key.2, key.3), egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], image.as_raw()), egui::TextureOptions::LINEAR);
                self.textures.insert(key, (texture, self.frame));
                got = true;
            }
        }
        if got {
            ctx.request_repaint();
        }
        // (the least recently drawn go when there are too many)
        if self.textures.len() > KEPT {
            let mut by_age: Vec<(u64, Key)> = self.textures.iter().map(|(k, (_, f))| (*f, k.clone())).collect();
            by_age.sort_by_key(|(f, _)| *f);
            for (_, k) in by_age.into_iter().take(self.textures.len() - KEPT) {
                self.textures.remove(&k);
            }
        }
    }

    /// A tile's texture when it is ready (asked of the thread when it is not).
    pub fn get(&mut self, map: &Arc<PlanetMap>, level: usize, index: usize) -> Option<egui::TextureId> {
        let k = key(map, level, index);
        if let Some((t, used)) = self.textures.get_mut(&k) {
            *used = self.frame;
            return Some(t.id());
        }
        if self.asked.insert(k.clone()) {
            let _ = self.to_thread.send((k, map.clone()));
        }
        None
    }

    /// Draws a level's tiles that `place` (the level's pixel to the screen) puts in `clip`.
    pub fn draw(&mut self, painter: &egui::Painter, map: &Arc<PlanetMap>, level: usize, origin: egui::Pos2, scale: f32) {
        let Some(l) = map.levels.get(level) else { return };
        let clip = painter.clip_rect();
        let size = l.texture_size as f32 * scale;
        let (cols, rows) = (l.tiles.0 as i64, l.tiles.1 as i64);
        let first = |lo: f32, o: f32| (((lo - o) / size).floor() as i64).max(0);
        let (c0, r0) = (first(clip.left(), origin.x), first(clip.top(), origin.y));
        let (c1, r1) = ((((clip.right() - origin.x) / size).ceil() as i64).min(cols), (((clip.bottom() - origin.y) / size).ceil() as i64).min(rows));
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        for c in c0..c1 {
            for r in r0..r1 {
                let index = (c * rows + r) as usize;
                let rect = egui::Rect::from_min_size(origin + egui::vec2(c as f32 * size, r as f32 * size), egui::vec2(size, size));
                match self.get(map, level, index) {
                    Some(t) => {
                        painter.image(t, rect, uv, egui::Color32::WHITE);
                    }
                    None => {
                        painter.rect_filled(rect, 0.0, egui::Color32::from_gray(18));
                    }
                }
            }
        }
    }
}
