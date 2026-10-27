//! The planet map: `cd_image/textures/PlanetMap/normal/PlanetMapIndexNormal.txt` names the map (`Name`, `Type Rubika`,
//! `CoordsFile`) and its zoom levels, each `File <bin>`, `TextureSize 128`, `Size w h`, `Tiles cols rows`,
//! `MapRect left top right bottom` and a `FilePos` per tile: a 128×128 PNG at that offset of the file, the tiles laid
//! out column by column (the first `rows` tiles are the leftmost column, top to bottom). `MapCoordinates.xml` gives each playfield's place on it: `<Playfield id x xscale z zscale>`.
//!
//! GUI.dll's `PlanetMapView_c` (FUN_1004d1cf reads the index, FUN_1004c6cb the coordinates) puts a world position on
//! the map as FUN_1004bedd does: the playfield's `x`/`z` plus the position are world coordinates, less the map's
//! origin (31022, 24880, `_DAT_101b1260` / `_DAT_101b1258`), scaled by the level's rectangle over the map's extent
//! (18751 by 24115, `_DAT_101b1250` / `_DAT_101b1248`) and the playfield's scales, from the rectangle's left; the
//! vertical is turned over (north up).

use std::path::Path;

pub const ORIGIN: (f32, f32) = (31022.0, 24880.0);
pub const EXTENT: (f32, f32) = (18751.0, 24115.0);

#[derive(Clone, Debug)]
pub struct Level {
    pub file: String,
    pub texture_size: u32,
    pub size: (u32, u32),
    pub tiles: (u32, u32),
    /// left, top, right, bottom
    pub rect: [i32; 4],
    pub positions: Vec<u64>,
}

#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub x: f32,
    pub xscale: f32,
    pub z: f32,
    pub zscale: f32,
}

#[derive(Clone, Debug, Default)]
pub struct PlanetMap {
    pub name: String,
    pub levels: Vec<Level>,
    pub places: std::collections::HashMap<u32, Place>,
}

impl PlanetMap {
    /// The default (Rubi-Ka) map's index and coordinates under the game directory.
    pub fn load(game: &Path) -> Option<PlanetMap> {
        let dir = game.join("cd_image").join("textures").join("PlanetMap");
        let index = std::fs::read_to_string(dir.join("normal").join("PlanetMapIndexNormal.txt")).ok()?;
        let mut map = PlanetMap::parse_index(&index);
        let coords = std::fs::read_to_string(dir.join("normal").join("MapCoordinates.xml")).ok()?;
        map.places = parse_coordinates(&coords);
        Some(map)
    }

    pub fn parse_index(text: &str) -> PlanetMap {
        let mut map = PlanetMap::default();
        let mut level: Option<Level> = None;
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let Some(key) = words.next() else { continue };
            let rest: Vec<&str> = words.collect();
            let num = |i: usize| rest.get(i).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
            match key {
                "Name" => map.name = line[4..].trim().trim_matches('"').to_string(),
                "File" => {
                    if let Some(l) = level.take() {
                        map.levels.push(l);
                    }
                    level = Some(Level { file: rest.first().map_or(String::new(), |s| s.to_string()), texture_size: 128, size: (0, 0), tiles: (0, 0), rect: [0; 4], positions: Vec::new() });
                }
                "TextureSize" => {
                    if let Some(l) = &mut level {
                        l.texture_size = num(0) as u32;
                    }
                }
                "Size" => {
                    if let Some(l) = &mut level {
                        l.size = (num(0) as u32, num(1) as u32);
                    }
                }
                "Tiles" => {
                    if let Some(l) = &mut level {
                        l.tiles = (num(0) as u32, num(1) as u32);
                    }
                }
                "MapRect" => {
                    if let Some(l) = &mut level {
                        l.rect = [num(0) as i32, num(1) as i32, num(2) as i32, num(3) as i32];
                    }
                }
                "FilePos" => {
                    if let Some(l) = &mut level {
                        l.positions.push(num(0) as u64);
                    }
                }
                _ => {}
            }
        }
        if let Some(l) = level {
            map.levels.push(l);
        }
        map
    }

    /// A level's whole picture, its tiles decoded and laid row by row.
    pub fn image(&self, game: &Path, level: usize) -> Option<image::RgbaImage> {
        let l = self.levels.get(level)?;
        let data = std::fs::read(game.join("cd_image").join("textures").join("PlanetMap").join(l.file.replace('/', std::path::MAIN_SEPARATOR_STR))).ok()?;
        let mut out = image::RgbaImage::new(l.size.0.max(1), l.size.1.max(1));
        // (the tiles go down each column in turn)
        for (i, &at) in l.positions.iter().enumerate() {
            let (col, row) = (i as u32 / l.tiles.1.max(1), i as u32 % l.tiles.1.max(1));
            let Some(tail) = data.get(at as usize..) else { continue };
            let Ok(tile) = image::load_from_memory(tail) else { continue };
            let tile = tile.to_rgba8();
            image::imageops::replace(&mut out, &tile, (col * l.texture_size) as i64, (row * l.texture_size) as i64);
        }
        Some(out)
    }

    /// Where a world position (server x and z) in `playfield` lands on `level`, in pixels of its picture (FUN_1004bedd);
    /// none for a playfield that is not on the map.
    pub fn pixel(&self, level: usize, playfield: u32, x: f32, z: f32) -> Option<(f32, f32)> {
        let l = self.levels.get(level)?;
        let p = self.places.get(&playfield)?;
        let [left, top, right, bottom] = l.rect.map(|v| v as f32);
        let u = p.x + x - ORIGIN.0;
        let v = p.z + z - ORIGIN.1;
        let sx = (right - left) / EXTENT.0;
        let sy = (bottom - top) / EXTENT.1;
        let px = left + u * sx * p.xscale;
        let py = top + v * sy * p.zscale;
        Some((px, (bottom - top) - py))
    }
}

fn parse_coordinates(text: &str) -> std::collections::HashMap<u32, Place> {
    let mut out = std::collections::HashMap::new();
    for tag in text.split("<Playfield").skip(1) {
        let attr = |name: &str| -> Option<&str> {
            let key = format!("{name}=\"");
            let start = tag.find(&key)? + key.len();
            let end = tag[start..].find('"')? + start;
            Some(&tag[start..end])
        };
        let (Some(id), Some(x), Some(z)) = (attr("id"), attr("x"), attr("z")) else { continue };
        let (Ok(id), Ok(x), Ok(z)) = (id.parse::<u32>(), x.parse::<f32>(), z.parse::<f32>()) else { continue };
        let xscale = attr("xscale").and_then(|v| v.parse().ok()).unwrap_or(1.0);
        let zscale = attr("zscale").and_then(|v| v.parse().ok()).unwrap_or(1.0);
        out.insert(id, Place { x, xscale, z, zscale });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_and_coordinates_are_read() {
        let index = "Name \"Default planet map\"\nType Rubika\nCoordsFile normal/MapCoordinates.xml\n\nFile normal/PlanetMapGfxNormal.bin\nTextureSize 128\n\nSize 384 640\nTiles 3 5\nMapRect 22 0 378 458\nFilePos 8192\nFilePos 28654\n\nFile normal/PlanetMapGfxNormal.bin\nTextureSize 128\n\nSize 1024 1536\nTiles 8 12\nMapRect 55 0 945 1145\nFilePos 1\n";
        let mut map = PlanetMap::parse_index(index);
        assert_eq!(map.name, "Default planet map");
        assert_eq!(map.levels.len(), 2);
        assert_eq!(map.levels[0].size, (384, 640));
        assert_eq!(map.levels[0].positions, vec![8192, 28654]);
        assert_eq!(map.levels[1].rect, [55, 0, 945, 1145]);
        map.places = parse_coordinates(r#"<root><Playfield id="540" name="Old Athen" x="39521" xscale="1.00000" z="40670" zscale="1.00000"/></root>"#);
        // Old Athen's origin on the big level: 8499 east of the map's origin over 18751 across 890 pixels from 55, and
        // 15790 north over 24115 across 1145 pixels, turned over
        let (px, py) = map.pixel(1, 540, 0.0, 0.0).unwrap();
        assert!((px - (55.0 + 8499.0 / 18751.0 * 890.0)).abs() < 0.01, "{px}");
        assert!((py - (1145.0 - 15790.0 / 24115.0 * 1145.0)).abs() < 0.01, "{py}");
        assert!(map.pixel(1, 999, 0.0, 0.0).is_none());
    }
}
