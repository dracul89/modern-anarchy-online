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
    /// World units of one eight-bit height step: the tile-map's second float (0.2 in Old Athens, 0.3 and 0.4 elsewhere).
    pub height_scale: f32,
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
    Ok(Terrain { width, height, heights, tiles, tile_types: map.tile_types.clone(), height_scale: if map.second > 0.0 { map.second } else { 0.2 }, mask: if has_types { 0x3fff } else { 0xff }, tile_size: map.tile_size })
}

impl Terrain {
    /// The water over the ground, found the only way the data allows: a
    /// playfield whose ground has a big flat floor (a third of its points at the lowest
    /// height, well below the land) is a seabed,
    /// and the water lies a little above it. (The original builds its liquid
    /// from triangles this client can't read yet; `MAO_WATER=level` overrides.)
    pub fn water(&self, height_scale: f32) -> Option<crate::model::Water> {
        let mut counts: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
        for &h in &self.heights {
            *counts.entry(h).or_default() += 1;
        }
        let unit = height_scale / 256.0;
        let (&floor, &n) = counts.iter().max_by_key(|&(_, n)| *n)?;
        let top = *counts.keys().next_back()?;
        let bottom = *counts.keys().next()?;
        let level = match std::env::var("MAO_WATER").ok().and_then(|v| v.trim().parse::<f32>().ok()) {
            Some(level) => level,
            None if n * 3 >= self.heights.len() && floor == bottom && (top - floor) as f32 * unit > 5.0 => floor as f32 * unit + 1.5,
            None => return None,
        };
        Some(crate::model::Water { level, min: [0.0, 0.0], max: [(self.width - 1) as f32 * self.tile_size, (self.height - 1) as f32 * self.tile_size], surfaces: Vec::new() })
    }
}

impl Terrain {
    /// The ground's height at the horizontal position (`x`, `y`) in the
    /// units of [`Terrain::model`] (bilinear between grid points; outside
    /// the grid the nearest edge).
    pub fn height_at(&self, x: f32, y: f32, height_scale: f32) -> f32 {
        let fx = (x / self.tile_size).clamp(0.0, (self.width - 1) as f32);
        let fy = (y / self.tile_size).clamp(0.0, (self.height - 1) as f32);
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let h = |x: usize, y: usize| self.heights[y * self.width + x] as f32 / 256.0 * height_scale;
        let top = h(x0, y0) * (1.0 - tx) + h(x1, y0) * tx;
        let bottom = h(x0, y1) * (1.0 - tx) + h(x1, y1) * tx;
        top * (1.0 - ty) + bottom * ty
    }

    /// The ground's height and unit normal (y up) at (`x`, `z`) as N3.dll's tile-map has them (FUN_10017800 over
    /// FUN_1001769d): the tile's two triangles, split from (i+1, j) to (i, j+1) when i and j differ in parity, else from
    /// (i, j) to (i+1, j+1), each a plane; none off the map (the original's height 0 there).
    pub fn surface(&self, x: f32, z: f32, height_scale: f32) -> Option<(f32, [f32; 3])> {
        let t = self.tile_size;
        if x < 0.0 || t * self.width as f32 <= x || z < 0.0 || t * self.height as f32 <= z {
            return None;
        }
        let (i, j) = ((x / t) as usize, (z / t) as usize);
        let (i1, j1) = ((i + 1).min(self.width - 1), (j + 1).min(self.height - 1));
        let h = |a: usize, b: usize| self.heights[b.min(self.height - 1) * self.width + a.min(self.width - 1)] as f32 / 256.0 * height_scale;
        let (x0, z0) = (t * i as f32, t * j as f32);
        let p0 = [x0, h(i, j), z0];
        let p1 = [x0 + t, h(i1, j), z0];
        let p2 = [x0 + t, h(i1, j1), z0 + t];
        let p3 = [x0, h(i, j1), z0 + t];
        let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let (dx, dz) = (x - x0, z - z0);
        let same_parity = (!j ^ i) & 1 != 0;
        let (base, n) = if !same_parity {
            if t - dz < dx { (p1, cross(sub(p1, p3), sub(p1, p2))) } else { (p0, cross(sub(p0, p3), sub(p0, p1))) }
        } else if dx <= dz {
            (p0, cross(sub(p0, p3), sub(p0, p2)))
        } else {
            (p0, cross(sub(p0, p2), sub(p0, p1)))
        };
        let height = -((x - base[0]) * n[0] + (z - base[2]) * n[2]) / n[1] + base[1];
        let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        Some((height, [n[0] / length, n[1] / length, n[2] / length]))
    }

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
                    part.vertices.push(Vertex { position: position(gx, gy), normal: normal(gx, gy), uv, colour: Vertex::UNBAKED });
                }
                part.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        let parts = groups.into_values().flatten().collect();
        Model { parts, instances: Vec::new(), water: self.water(height_scale), lod_groups: Vec::new() }
    }
}
