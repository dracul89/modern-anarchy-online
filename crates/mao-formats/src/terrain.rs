//! Terrain from an old-style tile-map ([`crate::tilemap`]): the
//! `AnarchyGroundDataDB_t` object in its archive, cut into square patches
//! whose maps are zlib-packed.
//!
//! Derived from the original's `AnarchyGroundData_t`
//! (`AnarchyReimagined`: `dropin/displaysystem/src/{grounddata,patch}.rs`):
//! * the patch size N is `1 << min(lowest set bit of (width - 1), lowest set
//!   bit of (height - 1), 6)`; there are `(width - 1) / N` columns and
//!   `(height - 1) / N` rows of patches;
//! * each patch has `heightmap_compressed_data` (zlib, `(N + 1)²` heights),
//!   `heightmap_small_data` (its four corners; 8 bytes means the heights are
//!   16-bit, 4 bytes 8-bit) and `tilemap_compressed_data` (zlib, N² words);
//! * heights are stored as differences: each is the sum of those before it
//!   down its column, then along its row (wrapping), and 8-bit heights are
//!   scaled up to 16-bit by 256;
//! * a tile word's low bits (mask 0x3fff, or 0xff without tile types) index
//!   the tile texture; its top two bits turn the texture.
//!
//! The tile texture for an index is the ground texture object whose
//! instance is `tile_types[index]` (type 0xf6966, 0xf6965 or 0xf6956, by
//! quality).

use std::fmt;
use std::io::Read;

use crate::archive::Stream;
use crate::model::{Material, Model, Part, Vertex, IDENTITY};
use crate::tilemap::OldTilemap;

#[derive(Debug)]
pub enum Error {
    Shape(&'static str),
    Zlib(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Shape(what) => write!(f, "unexpected terrain data: {what}"),
            Error::Zlib(e) => write!(f, "a packed map won't unpack: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// The ground: heights at the grid's points and a tile word for each cell.
#[derive(Debug)]
pub struct Terrain {
    /// The grid's size in points (cells are one less each way).
    pub width: usize,
    pub height: usize,
    /// 16-bit heights (unsigned), `width * height`, row by row.
    pub heights: Vec<u16>,
    /// Tile words, `(width - 1) * (height - 1)`.
    pub tiles: Vec<u16>,
    /// The ground texture instance of each tile index (the tile-map's list).
    pub tile_types: Vec<u16>,
    /// The mask for a tile word's index: 0x3fff when the ground data has
    /// tile-type data, else 0xff.
    pub mask: u16,
    /// A world unit per cell.
    pub tile_size: f32,
}

fn low_bit(v: u32) -> u32 {
    if v == 0 { 32 } else { v.trailing_zeros() }
}

fn unpack(packed: &[u8], size: usize) -> Result<Vec<u8>, Error> {
    let mut out = vec![0; size];
    let mut decoder = flate2::read::ZlibDecoder::new(packed);
    let mut done = 0;
    while done < size {
        match decoder.read(&mut out[done..]).map_err(Error::Zlib)? {
            0 => break,
            n => done += n,
        }
    }
    Ok(out)
}

fn values<'a>(s: &'a Stream, name: &str, index: usize) -> Option<&'a Vec<u8>> {
    s.field("", name)?.values.get(index)
}

/// The terrain of a tile-map.
pub fn build(map: &OldTilemap) -> Result<Terrain, Error> {
    let ground = map.archive.objects.first().ok_or(Error::Shape("an archive without objects"))?;
    let int = |name| ground.int("", name).ok_or(Error::Shape("a missing size"));
    let (width, height) = (int("map_width")? as usize, int("map_height")? as usize);
    if width < 2 || height < 2 || width > 4096 || height > 4096 {
        return Err(Error::Shape("a map size out of range"));
    }
    let n = 1usize << low_bit(height as u32 - 1).min(low_bit(width as u32 - 1)).min(6);
    let (columns, rows) = ((width - 1) / n, (height - 1) / n);
    let side = n + 1;
    let mut heights = vec![0u16; width * height];
    let mut tiles = vec![0u16; (width - 1) * (height - 1)];
    for row in 0..rows {
        for column in 0..columns {
            let index = row * columns + column;
            let small = values(ground, "heightmap_small_data", index).ok_or(Error::Shape("a patch without corner heights"))?;
            let wide = small.len() == 8;
            let packed = values(ground, "heightmap_compressed_data", index).ok_or(Error::Shape("a patch without heights"))?;
            let bytes = unpack(packed, side * side * if wide { 2 } else { 1 })?;
            let at = |x: usize, y: usize| side * y + x;
            let mut h: Vec<i16> = if wide { bytes.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect() } else { bytes.iter().map(|&b| b as i16).collect() };
            let add = |a: i16, b: i16| if wide { a.wrapping_add(b) } else { (a as u8).wrapping_add(b as u8) as i16 };
            for x in 0..side {
                let mut sum = 0i16;
                for y in 0..side {
                    sum = add(sum, h[at(x, y)]);
                    h[at(x, y)] = sum;
                }
            }
            for y in 0..side {
                let mut sum = 0i16;
                for x in 0..side {
                    sum = add(sum, h[at(x, y)]);
                    h[at(x, y)] = sum;
                }
            }
            for y in 0..side {
                for x in 0..side {
                    let v = h[at(x, y)];
                    heights[(row * n + y) * width + column * n + x] = if wide { v as u16 } else { (v as u8 as u16) << 8 };
                }
            }
            let packed = values(ground, "tilemap_compressed_data", index).ok_or(Error::Shape("a patch without tiles"))?;
            let words = unpack(packed, n * n * 2)?;
            for ty in 0..n {
                for tx in 0..n {
                    let w = u16::from_le_bytes([words[(ty * n + tx) * 2], words[(ty * n + tx) * 2 + 1]]);
                    tiles[(row * n + ty) * (width - 1) + column * n + tx] = w;
                }
            }
        }
    }
    let has_types = ground.field("", "tile_type_data").and_then(|f| f.values.first()).is_some_and(|v| v.len() >= 2);
    Ok(Terrain { width, height, heights, tiles, tile_types: map.tile_types.clone(), mask: if has_types { 0x3fff } else { 0xff }, tile_size: map.tile_size })
}

impl Terrain {
    /// The ground as drawable parts, one per tile texture. A height unit of
    /// 1/256 of an eight-bit step is `height_scale` world units.
    pub fn model(&self, quality_type: u32, height_scale: f32) -> Model {
        let mask = self.mask;
        let ts = self.tile_size;
        let z = |x: usize, y: usize| self.heights[y * self.width + x] as f32 / 256.0 * height_scale;
        let position = |x: usize, y: usize| [x as f32 * ts, y as f32 * ts, z(x, y)];
        let normal = |x: usize, y: usize| {
            let (x0, x1, y0, y1) = (x.saturating_sub(1), (x + 1).min(self.width - 1), y.saturating_sub(1), (y + 1).min(self.height - 1));
            let dx = (z(x1, y) - z(x0, y)) / ((x1 - x0).max(1) as f32 * ts);
            let dy = (z(x, y1) - z(x, y0)) / ((y1 - y0).max(1) as f32 * ts);
            let len = (dx * dx + dy * dy + 1.0).sqrt();
            [-dx / len, -dy / len, 1.0 / len]
        };
        // One part per tile texture, a new one when 16-bit indices run out.
        let mut groups: std::collections::BTreeMap<u16, Vec<Part>> = std::collections::BTreeMap::new();
        for cy in 0..self.height - 1 {
            for cx in 0..self.width - 1 {
                let word = self.tiles[cy * (self.width - 1) + cx];
                let index = word & mask;
                let turn = word & 0xc000;
                let parts = groups.entry(index).or_default();
                if parts.last().is_none_or(|p| p.vertices.len() + 4 > 65_532) {
                    let instance = self.tile_types.get(index as usize).copied().unwrap_or(index) as u32;
                    parts.push(Part {
                        name: format!("tile {index}"),
                        world: IDENTITY,
                        vertices: Vec::new(),
                        indices: Vec::new(),
                        material: Material { name: format!("tile {index}"), diffuse: [1.0; 3], opacity: 1.0, textures: vec![(quality_type, instance)], ..Default::default() },
                    });
                }
                let part = parts.last_mut().unwrap();
                let base = part.vertices.len() as u16;
                for (ox, oy) in [(0.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
                    let (gx, gy) = (cx + ox as usize, cy + oy as usize);
                    let uv = match turn {
                        0 => [ox, 1.0 - oy],
                        0x4000 => [1.0 - oy, 1.0 - ox],
                        0x8000 => [1.0 - ox, oy],
                        _ => [oy, ox],
                    };
                    part.vertices.push(Vertex { position: position(gx, gy), normal: normal(gx, gy), uv });
                }
                part.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        let parts = groups.into_values().flatten().collect();
        Model { parts }
    }
}
