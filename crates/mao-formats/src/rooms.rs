//! The rooms of an instanced interior (a shop): the playfield definition object (type `0xf4241`) of such a playfield lists
//! rectangles of its tile-map (`0xf4249`, a `GNDA` map holding a dungeon's rooms side by side), each placed in the world by a
//! centre position and a turn. Derived from `RDBPlayfield_t::ReadBlob` and `n3Room_t`'s reader in `N3.dll`.
//!
//! # The playfield object
//! After the 12-byte object head, little-endian: a version (10), the playfield's number, a 32-byte name, the tile-map's
//! instance, a word (10), the room count, then (version > 8) five words and 24 bytes that are skipped. When the tile-map's
//! instance differs from the playfield's number, `count` rooms follow (otherwise zones, the layout of an outdoor playfield).
//!
//! # A room
//! A flags byte (bits 0-1: the turn in quarter turns, bit 7: a 32-byte name follows the links), a byte (5), the rectangle
//! of tile-map cells `x0 y0 x1 y1` (`u16`, the far edge excluded), the centre `x y z` (`f32`, `y` is the floor's height), a
//! link count with that many `(u16, u16)` pairs, the name, a size word and a second word followed by that size minus 4 bytes
//! (a packed light map), a word `(n + 1) * 0x3f1` and `n` water surfaces (flags, a count of `f32` triples, a count of
//! `u16` triples), a count of camera attractors (version 10: position, rotation, direction, a float: 44 bytes each).
//!
//! # The shape
//! The tile-map's `DCGA` image says which cells exist (0 is empty), `DHGA` is the ground's height in steps of the tile-map's
//! second float (0.2) and `XTHA` the tiles. Cells are the tile-map's scale (2) wide. A room's cells are laid out around its
//! centre and turned by `flags & 3` quarter turns (a cell offset `(dx, dy)` becomes `(dy, -dx)` per turn, checked against the
//! shop booths the server places in them).

use std::fmt;

use crate::model::{Material, Model, Part, Vertex, IDENTITY};
use crate::tilemap::NewTilemap;
use crate::texture::OBJECT_HEAD;

#[derive(Debug)]
pub enum Error {
    Truncated(&'static str),
    Layout(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated(what) => write!(f, "playfield truncated in {what}"),
            Error::Layout(what) => write!(f, "{what}"),
        }
    }
}

impl std::error::Error for Error {}

/// The database type of a playfield's definition.
pub const KIND: u32 = 0xf4241;

#[derive(Clone, Debug)]
pub struct Room {
    pub flags: u8,
    /// `x0 y0 x1 y1`: the tile-map cells (the far edge excluded).
    pub rect: [u16; 4],
    /// The centre; `y` is the floor's height.
    pub position: [f32; 3],
    pub links: Vec<(u16, u16)>,
    pub name: String,
}

impl Room {
    /// Quarter turns.
    pub fn turn(&self) -> u8 {
        self.flags & 3
    }
}

#[derive(Clone, Debug)]
pub struct RoomPlayfield {
    pub number: u32,
    pub name: String,
    /// The instance of the tile-map (`0xf4249`) the rooms are cut from.
    pub tilemap: u32,
    pub rooms: Vec<Room>,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&[u8], Error> {
        let b = self.data.get(self.at..self.at + n).ok_or(Error::Truncated(what))?;
        self.at += n;
        Ok(b)
    }
    fn u8(&mut self, what: &'static str) -> Result<u8, Error> {
        Ok(self.take(1, what)?[0])
    }
    fn u16(&mut self, what: &'static str) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take(2, what)?.try_into().unwrap()))
    }
    fn u32(&mut self, what: &'static str) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }
    fn f32(&mut self, what: &'static str) -> Result<f32, Error> {
        Ok(f32::from_bits(self.u32(what)?))
    }
}

/// The rooms of a playfield definition object (an outdoor playfield, which has zones instead, gives `None`).
pub fn parse(object: &[u8]) -> Result<Option<RoomPlayfield>, Error> {
    let mut r = Reader { data: object, at: OBJECT_HEAD };
    let version = r.u32("the version")?;
    if version < 7 {
        return Err(Error::Layout("playfield data format older than 7"));
    }
    let number = r.u32("the number")?;
    let name_bytes = r.take(32, "the name")?;
    let name = String::from_utf8_lossy(&name_bytes[..name_bytes.iter().position(|&b| b == 0).unwrap_or(32)]).into_owned();
    let tilemap = r.u32("the tile-map")?;
    r.u32("a word")?;
    let count = r.u32("the room count")?;
    if version > 8 {
        r.take(20 + 24, "the extra header")?;
    }
    if tilemap == number {
        return Ok(None);
    }
    if count == 0 {
        return Ok(None);
    }
    if count >= 50_000 {
        return Err(Error::Layout("invalid room count"));
    }
    let mut rooms = Vec::new();
    for _ in 0..count {
        let flags = r.u8("a room's flags")?;
        r.u8("a room's second byte")?;
        let rect = [r.u16("a room's rectangle")?, r.u16("a room's rectangle")?, r.u16("a room's rectangle")?, r.u16("a room's rectangle")?];
        let position = [r.f32("a room's position")?, r.f32("a room's position")?, r.f32("a room's position")?];
        let links = r.u16("the link count")?;
        let mut link_list = Vec::new();
        for _ in 0..links {
            link_list.push((r.u16("a link")?, r.u16("a link")?));
        }
        let name = if flags & 0x80 != 0 {
            let b = r.take(32, "a room's name")?;
            String::from_utf8_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(32)]).into_owned()
        } else {
            String::new()
        };
        // The packed light map.
        let size = r.u32("the light map's size")? as usize;
        r.u32("the light map's unpacked size")?;
        r.take(size.checked_sub(4).ok_or(Error::Layout("light map too small"))?, "the light map")?;
        // The water surfaces.
        let water = r.u32("the water word")?;
        if water != 0 {
            if water % 0x3f1 != 0 {
                return Err(Error::Layout("broken water data"));
            }
            for _ in 0..(water / 0x3f1 - 1) {
                r.u32("a surface's flags")?;
                let points = r.u32("a surface's point count")? as usize;
                r.take(points * 12, "a surface's points")?;
                let triangles = r.u32("a surface's triangle count")? as usize;
                r.take(triangles * 6, "a surface's triangles")?;
            }
        }
        let attractors = r.u32("the attractor count")? as usize;
        if attractors >= 1000 {
            return Err(Error::Layout("too many camera attractors"));
        }
        r.take(attractors * 44, "the camera attractors")?;
        rooms.push(Room { flags, rect, position, links: link_list, name });
    }
    Ok(Some(RoomPlayfield { number, name, tilemap, rooms }))
}

/// The cell images of a `GNDA` tile-map that a room is cut from.
pub struct Grid {
    pub width: usize,
    pub height: usize,
    /// 0 where there is no ground.
    pub cells: Vec<u8>,
    pub heights: Vec<u8>,
    /// The tile layers (red, green, blue).
    pub tiles: Vec<[u8; 3]>,
    /// The width of a cell and the height of a step.
    pub scale: f32,
    pub step: f32,
    /// The texture (an instance of type `0xf6959`, with lower qualities in `0xf6967` and `0xf6968`) of each tile value: the
    /// table at the head of the `XTHA` chunk (a count, then that many words).
    pub palette: Vec<u32>,
}

/// The quality type of the room textures.
pub const TEXTURE: u32 = 0xf6959;

fn decode(body: &[u8]) -> Option<image::DynamicImage> {
    let start = body.windows(4).position(|w| w == b"\x89PNG")?;
    image::load_from_memory(&body[start..]).ok()
}

impl Grid {
    pub fn new(map: &NewTilemap) -> Option<Grid> {
        let chunk = |tag: &[u8; 4]| map.chunks.iter().find(|c| &c.tag == tag);
        let cells = decode(&chunk(b"DCGA")?.body)?.to_luma8();
        let heights = decode(&chunk(b"DHGA")?.body)?.to_luma8();
        let tiles = chunk(b"XTHA").and_then(|c| decode(&c.body)).map(|i| i.to_rgb8());
        let palette = chunk(b"XTHA")
            .and_then(|c| {
                let count = u16::from_le_bytes(c.body.get(..2)?.try_into().ok()?) as usize;
                (0..count).map(|i| Some(u32::from_le_bytes(c.body.get(2 + 4 * i..6 + 4 * i)?.try_into().ok()?))).collect::<Option<Vec<u32>>>()
            })
            .unwrap_or_default();
        let (width, height) = (cells.width() as usize, cells.height() as usize);
        let tiles = match tiles {
            Some(t) if t.width() as usize == width && t.height() as usize == height => t.pixels().map(|p| p.0).collect(),
            _ => vec![[0; 3]; width * height],
        };
        Some(Grid { width, height, cells: cells.into_raw(), heights: heights.into_raw(), tiles, scale: map.scale, step: map.second, palette })
    }

    fn at(&self, x: i64, y: i64) -> Option<(u8, u8)> {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return None;
        }
        let i = y as usize * self.width + x as usize;
        Some((self.cells[i], self.heights[i]))
    }

    /// The texture of a tile value (none for 0, which is the default, and for values past the table).
    fn texture(&self, value: u8) -> Option<u32> {
        self.palette.get(value as usize).copied().filter(|&t| value != 0 && t > 1)
    }

    fn tile(&self, x: i64, y: i64) -> [u8; 3] {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return [0; 3];
        }
        self.tiles[y as usize * self.width + x as usize]
    }
}

/// Where a statel of the room's zone stands in the world (and which way it faces): its position is relative to the room's
/// centre (the height is the world's) and the room's turn turns both.
pub fn place_statel(room: &Room, position: [f32; 3], rotation: [f32; 4]) -> ([f32; 3], [f32; 4]) {
    let (mut dx, mut dz) = (position[0], position[2]);
    for _ in 0..room.turn() {
        (dx, dz) = (dz, -dx);
    }
    let half = room.turn() as f32 * std::f32::consts::FRAC_PI_4;
    let (s, c) = half.sin_cos();
    let a = [0.0, s, 0.0, c];
    let b = rotation;
    let q = [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ];
    ([room.position[0] + dx, position[1], room.position[2] + dz], q)
}

/// The most two neighbouring cells may differ, in height steps, and still be one slope (6 steps of 0.2 is 1.2 units over a cell of
/// 2: the ramps are 5 steps a cell); a bigger difference is a wall.
const SLOPE: i32 = 6;

/// A room's cells as a surface (each cell's top follows its neighbours' heights so ramps are smooth, with a wall where the
/// difference is more than a slope or the neighbour is missing), in the viewer's Z-up world (x, the server's z, height).
/// `material` is given to the parts.
pub fn room_parts(grid: &Grid, room: &Room, material: &Material) -> Vec<Part> {
    let [x0, y0, x1, y1] = room.rect.map(|v| v as i64);
    let (w, h) = (x1 - x0, y1 - y0);
    let turn = room.turn();
    let cell = grid.scale;
    // A cell-space offset from the room's centre to the world's x and z offset.
    let rotate = |mut dx: f32, mut dy: f32| {
        for _ in 0..turn {
            (dx, dy) = (dy, -dx);
        }
        (dx * cell, dy * cell)
    };
    let corner = |cx: i64, cy: i64| rotate((cx - x0) as f32 - w as f32 / 2.0, (cy - y0) as f32 - h as f32 / 2.0);
    let height = |value: u8| room.position[1] + value as f32 * grid.step;
    let (cx0, cz0) = (room.position[0], room.position[2]);
    // The cell's height in steps, when it exists inside the room's rectangle.
    let steps = |cx: i64, cy: i64| -> Option<i32> {
        if cx < x0 || cx >= x1 || cy < y0 || cy >= y1 {
            return None;
        }
        grid.at(cx, cy).filter(|&(exists, _)| exists != 0).map(|(_, v)| v as i32)
    };
    // The height of one corner of a cell (0: its own (cx, cy), 1: (cx+1, cy), 2: (cx+1, cy+1), 3: (cx, cy+1)): the average of
    // the cells around that corner that are within a slope of this one.
    let corner_height = |cx: i64, cy: i64, k: usize| -> f32 {
        let own = steps(cx, cy).unwrap_or(0);
        let (vx, vy) = match k {
            0 => (cx, cy),
            1 => (cx + 1, cy),
            2 => (cx + 1, cy + 1),
            _ => (cx, cy + 1),
        };
        let (mut sum, mut n) = (0, 0);
        for (ax, ay) in [(vx - 1, vy - 1), (vx, vy - 1), (vx - 1, vy), (vx, vy)] {
            if let Some(v) = steps(ax, ay).filter(|v| (v - own).abs() <= SLOPE) {
                sum += v;
                n += 1;
            }
        }
        room.position[1] + sum as f32 / n.max(1) as f32 * grid.step
    };
    // One part per texture (a new one when the 16-bit indices run out).
    let mut groups: std::collections::BTreeMap<Option<u32>, Vec<Part>> = std::collections::BTreeMap::new();
    let mut quad = |texture: Option<u32>, p: [[f32; 3]; 4], normal: [f32; 3], uv: [[f32; 2]; 4]| {
        let parts = groups.entry(texture).or_default();
        if parts.last().is_none_or(|part| part.vertices.len() + 4 > 65_535) {
            let mut material = material.clone();
            if let Some(t) = texture {
                material.textures = vec![(TEXTURE, t)];
                material.diffuse = [1.0; 3];
                material.ambient = [1.0; 3];
            }
            parts.push(Part { name: if room.name.is_empty() { "room".into() } else { room.name.clone() }, world: IDENTITY, vertices: Vec::new(), indices: Vec::new(), material });
        }
        let part = parts.last_mut().unwrap();
        let base = part.vertices.len() as u16;
        for k in 0..4 {
            part.vertices.push(Vertex { position: p[k], normal, uv: uv[k] });
        }
        part.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    let world = |dx: f32, dz: f32, up: f32| [cx0 + dx, cz0 + dz, up];
    for cy in y0..y1 {
        for cx in x0..x1 {
            let Some((exists, value)) = grid.at(cx, cy) else { continue };
            if exists == 0 {
                continue;
            }
            let _ = (value, height);
            let tops = [corner_height(cx, cy, 0), corner_height(cx, cy, 1), corner_height(cx, cy, 2), corner_height(cx, cy, 3)];
            let c = [corner(cx, cy), corner(cx + 1, cy), corner(cx + 1, cy + 1), corner(cx, cy + 1)];
            let tile = grid.tile(cx, cy);
            let p = [world(c[0].0, c[0].1, tops[0]), world(c[1].0, c[1].1, tops[1]), world(c[2].0, c[2].1, tops[2]), world(c[3].0, c[3].1, tops[3])];
            // The normal of the (slightly bent) top: across its diagonals.
            let (d1, d2) = ([p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]], [p[3][0] - p[1][0], p[3][1] - p[1][1], p[3][2] - p[1][2]]);
            let n = [d1[1] * d2[2] - d1[2] * d2[1], d1[2] * d2[0] - d1[0] * d2[2], d1[0] * d2[1] - d1[1] * d2[0]];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
            let top_normal = if n[2] < 0.0 { [-n[0] / len, -n[1] / len, -n[2] / len] } else { [n[0] / len, n[1] / len, n[2] / len] };
            quad(grid.texture(tile[0]), p, top_normal, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);
            // The walls toward each neighbour that is missing or more than a slope lower.
            for (nx, ny, a, b, na, nb) in [(0i64, -1i64, 0usize, 1usize, 3usize, 2usize), (1, 0, 1, 2, 0, 3), (0, 1, 2, 3, 1, 0), (-1, 0, 3, 0, 2, 1)] {
                let (low_a, low_b) = match steps(cx + nx, cy + ny) {
                    Some(v) if (value as i32 - v) <= SLOPE => continue,
                    Some(_) => (corner_height(cx + nx, cy + ny, na), corner_height(cx + nx, cy + ny, nb)),
                    None => (room.position[1], room.position[1]),
                };
                let (top_a, top_b) = (tops[a], tops[b]);
                if low_a >= top_a && low_b >= top_b {
                    continue;
                }
                // The outward direction in the world.
                let (ex, ez) = rotate(nx as f32, ny as f32);
                let len = (ex * ex + ez * ez).sqrt().max(1e-6);
                let normal = [ex / len, ez / len, 0.0];
                let (pa, pb) = (c[a], c[b]);
                // Wound so the front faces the neighbour.
                let rise = ((top_a - low_a).max(top_b - low_b)) / cell;
                quad(grid.texture(tile[1]), [world(pb.0, pb.1, low_b), world(pa.0, pa.1, low_a), world(pa.0, pa.1, top_a), world(pb.0, pb.1, top_b)], normal, [[0.0, rise], [1.0, rise], [1.0, 0.0], [0.0, 0.0]]);
            }
        }
    }
    groups.into_values().flatten().collect()
}

/// Whether the world point `(x, z)` is over one of the room's existing cells. `reversed` turns the room the other way (the check
/// that told the two senses apart: the machines and terminals built into a playfield stand on floor cells).
pub fn covers(grid: &Grid, room: &Room, x: f32, z: f32, reversed: bool) -> bool {
    let [x0, y0, x1, y1] = room.rect.map(|v| v as i64);
    let (w, h) = (x1 - x0, y1 - y0);
    let cell = grid.scale;
    let (mut dx, mut dy) = ((x - room.position[0]) / cell, (z - room.position[2]) / cell);
    // undo the turn: the forward turn is (dx, dy) -> (dy, -dx), so its inverse is (dx, dy) -> (-dy, dx)
    for _ in 0..room.turn() {
        (dx, dy) = if reversed { (dy, -dx) } else { (-dy, dx) };
    }
    let (cx, cy) = ((dx + w as f32 / 2.0).floor() as i64, (dy + h as f32 / 2.0).floor() as i64);
    cx >= 0 && cy >= 0 && cx < w && cy < h && grid.at(x0 + cx, y0 + cy).is_some_and(|(exists, _)| exists != 0)
}

/// All the rooms of a playfield as one model.
pub fn model(grid: &Grid, rooms: &RoomPlayfield, material: &Material) -> Model {
    let parts: Vec<Part> = rooms.rooms.iter().flat_map(|r| room_parts(grid, r, material)).collect();
    Model { parts, instances: Vec::new(), water: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(rooms: &[Vec<u8>]) -> Vec<u8> {
        let mut d = vec![0u8; OBJECT_HEAD];
        d.extend_from_slice(&10u32.to_le_bytes());
        d.extend_from_slice(&1180u32.to_le_bytes());
        let mut name = [0u8; 32];
        name[..4].copy_from_slice(b"shop");
        d.extend_from_slice(&name);
        d.extend_from_slice(&1140u32.to_le_bytes());
        d.extend_from_slice(&10u32.to_le_bytes());
        d.extend_from_slice(&(rooms.len() as u32).to_le_bytes());
        d.extend_from_slice(&[0; 44]);
        for r in rooms {
            d.extend_from_slice(r);
        }
        d
    }

    fn room(flags: u8, rect: [u16; 4], position: [f32; 3], name: Option<&str>) -> Vec<u8> {
        let mut d = vec![flags, 5];
        for v in rect {
            d.extend_from_slice(&v.to_le_bytes());
        }
        for v in position {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&[5, 0, 10, 0]);
        if let Some(n) = name {
            let mut b = [0u8; 32];
            b[..n.len()].copy_from_slice(n.as_bytes());
            d.extend_from_slice(&b);
        }
        // A light map of 8 bytes, no water, one camera attractor.
        d.extend_from_slice(&12u32.to_le_bytes());
        d.extend_from_slice(&416u32.to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        d.extend_from_slice(&0x3f1u32.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&[0; 44]);
        d
    }

    #[test]
    fn rooms_are_read_in_order() {
        let data = object(&[room(0, [10, 115, 21, 124], [208.0, 5.0, 202.0], None), room(0x83, [8, 34, 29, 70], [146.0, 5.0, 174.0], Some("Clan_Basic_2"))]);
        let p = parse(&data).unwrap().unwrap();
        assert_eq!((p.number, p.tilemap, p.name.as_str()), (1180, 1140, "shop"));
        assert_eq!(p.rooms.len(), 2);
        assert_eq!(p.rooms[0].rect, [10, 115, 21, 124]);
        assert_eq!(p.rooms[0].position, [208.0, 5.0, 202.0]);
        assert_eq!(p.rooms[0].links, vec![(5, 10)]);
        assert_eq!((p.rooms[1].name.as_str(), p.rooms[1].turn()), ("Clan_Basic_2", 3));
    }

    #[test]
    fn a_flat_room_is_a_floor() {
        let grid = Grid { width: 4, height: 4, cells: vec![1; 16], heights: vec![0; 16], tiles: vec![[0; 3]; 16], scale: 2.0, step: 0.2, palette: Vec::new() };
        let room = Room { flags: 0, rect: [0, 0, 2, 2], position: [10.0, 5.0, 20.0], links: Vec::new(), name: String::new() };
        let part = room_parts(&grid, &room, &Material::default()).remove(0);
        // four cells, no walls (the edge is at the floor's height)
        assert_eq!(part.vertices.len(), 16);
        let xs: Vec<f32> = part.vertices.iter().map(|v| v.position[0]).collect();
        assert!(xs.iter().all(|x| (8.0..=12.0).contains(x)));
        assert!(part.vertices.iter().all(|v| v.position[2] == 5.0));
    }

    #[test]
    fn a_ramp_is_smooth_and_a_cliff_is_a_wall() {
        // three cells in a row rising 5 steps each: one surface, no walls; then a cell 30 steps up: a wall
        let mut heights = vec![0u8; 16];
        heights[0] = 0;
        heights[1] = 5;
        heights[2] = 10;
        heights[3] = 40;
        let grid = Grid { width: 4, height: 4, cells: vec![1; 16], heights, tiles: vec![[0; 3]; 16], scale: 2.0, step: 0.2, palette: Vec::new() };
        let room = Room { flags: 0, rect: [0, 0, 4, 1], position: [0.0, 0.0, 0.0], links: Vec::new(), name: String::new() };
        let part = room_parts(&grid, &room, &Material::default()).remove(0);
        // 4 tops, plus the two end walls up from the floor (the cell at 40 steps and the edge of the ramp's start are at height 0 / 8)
        let walls = part.vertices.len() / 4 - 4;
        assert!(walls >= 3, "{walls}");
        // the ramp's cells meet: the corners between cells 0 and 1 are at the average of the two (0.5), whichever cell they belong to
        assert!(!heights_at_seam_is_empty(&part));
        let heights_at_seam: Vec<f32> = part.vertices.iter().filter(|v| (v.position[0] + 2.0).abs() < 1e-4 && v.normal[2] > 0.5).map(|v| v.position[2]).collect();
        assert!(heights_at_seam.iter().all(|h| (h - 0.5).abs() < 1e-4), "{heights_at_seam:?}");
    }

    fn heights_at_seam_is_empty(part: &Part) -> bool {
        part.vertices.iter().filter(|v| (v.position[0] + 2.0).abs() < 1e-4 && v.normal[2] > 0.5).count() == 0
    }

    #[test]
    fn a_turned_room_swaps_its_sides() {
        let grid = Grid { width: 8, height: 8, cells: vec![1; 64], heights: vec![0; 64], tiles: vec![[0; 3]; 64], scale: 2.0, step: 0.2, palette: Vec::new() };
        let room = Room { flags: 1, rect: [0, 0, 4, 2], position: [0.0, 0.0, 0.0], links: Vec::new(), name: String::new() };
        let part = room_parts(&grid, &room, &Material::default()).remove(0);
        let (mut xs, mut zs) = (Vec::new(), Vec::new());
        for v in &part.vertices {
            xs.push(v.position[0]);
            zs.push(v.position[1]);
        }
        let span = |v: &[f32]| v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min);
        assert_eq!((span(&xs), span(&zs)), (4.0, 8.0));
    }
}
