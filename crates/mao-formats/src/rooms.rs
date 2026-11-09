//! The rooms of an instanced interior (a shop): the playfield definition object (type `0xf4241`) of such a playfield lists
//! rectangles of its tile-map (`0xf4249`, a `GNDA` map holding a dungeon's rooms side by side), each placed in the world by a
//! centre position and a turn. Derived from `RDBPlayfield_t::ReadBlob` and `n3Room_t`'s reader (FUN_10012803) in `N3.dll`.
//!
//! # The playfield object
//! After the 12-byte object head, little-endian: a version (10), the playfield's number, a 32-byte name, the tile-map's
//! instance, a word (10), the room count, then (version > 8) five words and 24 bytes that are skipped. When the tile-map's
//! instance differs from the playfield's number, `count` rooms follow (otherwise zones, the layout of an outdoor playfield).
//!
//! # A room
//! A flags byte (bits 0-1: the turn in quarter turns, bit 7: a 32-byte name follows the links), a byte (5), the rectangle
//! of tile-map cells `x0 y0 x1 y1` (`u16`, the far edge excluded), the centre `x y z` (`f32`), a link count with that many
//! `(u16, u16)` pairs, the name, a size word and a count followed by that size minus 4 bytes (the light map: the count's 565
//! colours, one per vertex of the room's welded piece stream, zlib-packed; `n3Room_t::DepackLightmap` inflates them to two
//! bytes each, six in a file older than version 8; see [`room_parts`]), a word `(n + 1) * 0x3f1` and `n` water
//! surfaces (flags, a count of `f32` triples with the height rounded to a hundredth, a count of `u16` triples: a surface with
//! the low flag bit clear and other than three points is fanned round its centre, the others take their triangles), a count
//! of camera attractors (version 10: position, rotation, direction, a float: 44 bytes each).
//!
//! # The shape
//! The tile-map's `DCGA` image says the type of each cell (its place in the map's type list; 0 is empty), `DHGA` is the
//! floor's height in steps of the tile-map's second float (0.2), `HCDA` the codes that scale the pieces, `XTHA` the texture
//! layers. The height sample `(i, j)` is the corner `((i + 1) × scale, (j + 1) × scale)` of the map (FUN_1001769d), so a cell's
//! floor rests on the four samples before and at it, and the floor sits `1.0` above the room's position (`n3Room_t` +0x38 is
//! -1.0, taken off in `GetFloorHeight` FUN_10010db5 and in the pieces' base). A room's cells are laid out from its centre:
//! cell `(x, y)` of the rectangle is centred at `((x - x0) - oddW / 2) × scale` along x and likewise along z, with `oddW` the
//! width rounded down to odd (`CreateDungeonRoom` FUN_10008563), and the whole room is turned by `flags & 3` quarter turns about
//! its centre (`VisualRoom_t::SetPosition(position, turn × π/2)`; a cell offset `(dx, dy)` becomes `(dy, -dx)` per turn,
//! checked against the shop booths the server places in them). The walls are the pieces of `dungeon.rs`, welded into one
//! vertex stream per room and coloured from the light map in that order (`CreateDungeonRoom` → FUN_100082a7). The maps in
//! the files hold more colours than the stream has vertices (playfield 1180: 416 for 226 welded vertices; N3.dll's own
//! weld gives the same 226 over the same tiles), so they were baked from other geometry and the original client, like this
//! one, colours the stream from the map's start regardless.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crate::dungeon::{self, Cells, Laid, Piece, Surface, Tables};
use crate::model::{Material, Model, Part, Vertex, IDENTITY};
use crate::texture::OBJECT_HEAD;
use crate::tilemap::NewTilemap;

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

/// How far the floor sits above a room's position when the room has no cells: `-n3Room_t::+0x38` as the constructor
/// sets it (0x100100d8: -1).
pub const FLOOR_BASE: f32 = 1.0;

#[derive(Clone, Debug)]
pub struct Room {
    pub flags: u8,
    /// `x0 y0 x1 y1`: the tile-map cells (the far edge excluded).
    pub rect: [u16; 4],
    /// The centre.
    pub position: [f32; 3],
    pub links: Vec<(u16, u16)>,
    pub name: String,
    /// The water surfaces' triangles (the room's own frame, Y-up) with their flags.
    pub water: Vec<([[f32; 3]; 3], u32)>,
    /// The depacked light map: a 565 colour (`u16`) per vertex of the room's welded vertex stream, in its order (see
    /// `dungeon::light_colours`); empty when the room has none or it did not inflate.
    pub light: Vec<u8>,
}

impl Room {
    /// Quarter turns.
    pub fn turn(&self) -> u8 {
        self.flags & 3
    }

    /// A room-frame offset `(dx, dz)` turned the room's way.
    fn turn_xz(&self, mut dx: f32, mut dz: f32) -> (f32, f32) {
        for _ in 0..self.turn() {
            (dx, dz) = (dz, -dx);
        }
        (dx, dz)
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

/// One room of a generated building (a mission dungeon: the server's `BuildingRoomInfo_t`, read by Gamecode's FUN_100cad28):
/// the template room to copy, its floor (-16..16), its place on the building's grid of 10-unit blocks and its turn in
/// quarter turns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildingRoom {
    pub room: u16,
    pub floor: i8,
    pub x: u8,
    pub y: u8,
    pub turn: u8,
}

/// A generated building's rooms (Gamecode's FUN_100ca6f8, `ACGBuildingGeneratorData_t`'s playfield): each record's template
/// room copied, turned and placed. From the block's corner `(x × 10, 0, height × 10 − y × 10 − ⌊scale⌋ × size)` (`size` the
/// room's tiles across the world's z once turned, FUN_100cacbc), its centre is the room's half `(oddW / 2, 0, oddH / 2)` (`n3Room_t`
/// +0x68, FUN_10012803) times the tile-map's scale plus `(1, 0, 1)`, turned with the room, then moved back into the block by the
/// turn (90°: z + tilesX × scale; 180°: x + tilesX × scale, z + tilesZ × scale; 270°: x + tilesZ × scale). The height is the
/// template room's, raised `floor_step` per floor above the lowest. Returns the rooms and, for each, the template room it
/// came from (whose zone's statels it takes). `None` for a record naming no template room.
pub fn generate(template: &RoomPlayfield, scale: f32, height: u32, floor_step: f32, records: &[BuildingRoom]) -> Option<(RoomPlayfield, Vec<usize>)> {
    let lowest = records.iter().map(|r| r.floor).min().unwrap_or(0);
    let mut rooms = Vec::new();
    let mut sources = Vec::new();
    for r in records {
        let source = template.rooms.get(r.room as usize)?;
        let tiles_x = source.rect[2].saturating_sub(source.rect[0]) as f32;
        let tiles_z = source.rect[3].saturating_sub(source.rect[1]) as f32;
        let odd = |n: u16| ((n.saturating_sub(1)) & !1) + 1;
        let half = (odd(source.rect[2].saturating_sub(source.rect[0])) as f32 * 0.5, odd(source.rect[3].saturating_sub(source.rect[1])) as f32 * 0.5);
        let turn = r.turn & 3;
        let (mut dx, mut dz) = (half.0 * scale + 1.0, half.1 * scale + 1.0);
        for _ in 0..turn {
            (dx, dz) = (dz, -dx);
        }
        match turn {
            1 => dz += tiles_x * scale,
            2 => {
                dx += tiles_x * scale;
                dz += tiles_z * scale;
            }
            3 => dx += tiles_z * scale,
            _ => {}
        }
        let size = if turn & 1 == 1 { tiles_x } else { tiles_z };
        let corner = (r.x as f32 * 10.0, height as f32 - r.y as f32 * 10.0 - (scale as i32) as f32 * size);
        let mut room = source.clone();
        room.flags = (source.flags & !3) | turn;
        room.position = [corner.0 + dx, source.position[1] + (r.floor - lowest) as f32 * floor_step, corner.1 + dz];
        rooms.push(room);
        sources.push(r.room as usize);
    }
    Some((RoomPlayfield { rooms, ..template.clone() }, sources))
}

/// A point of the room `from` as it stands in its template, moved with the room to where `to` (a generated copy of it)
/// stands: taken into the room's own frame (its turn undone about its centre), turned the copy's way and put at the copy's
/// centre (the height moved by the copy's).
pub fn move_between(from: &Room, to: &Room, p: [f32; 3]) -> [f32; 3] {
    let (mut dx, mut dz) = (p[0] - from.position[0], p[2] - from.position[2]);
    for _ in 0..from.turn() {
        (dx, dz) = (-dz, dx);
    }
    let (dx, dz) = to.turn_xz(dx, dz);
    [to.position[0] + dx, p[1] - from.position[1] + to.position[1], to.position[2] + dz]
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
    parse_with_end(object).map(|(rooms, _)| rooms)
}

/// [`parse`] with the offset just past the rooms (where Gamecode's `RDBPlayfieldAnarchy_t` reads on; for an outdoor
/// playfield the offset is past the header only, the zones left to the caller).
pub fn parse_with_end(object: &[u8]) -> Result<(Option<RoomPlayfield>, usize), Error> {
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
        return Ok((None, r.at));
    }
    if count == 0 {
        return Ok((None, r.at));
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
        // The packed light map: `count` colours, `DepackLightmap`'s two bytes each (six before version 8).
        let size = r.u32("the light map's size")? as usize;
        let count = r.u32("the light map's vertex count")? as usize;
        let packed = r.take(size.checked_sub(4).ok_or(Error::Layout("light map too small"))?, "the light map")?;
        let light = if count == 0 { Vec::new() } else { depack(packed, count.saturating_mul(if version > 7 { 2 } else { 6 })) };
        // The water surfaces.
        let water_word = r.u32("the water word")?;
        let mut water = Vec::new();
        if water_word != 0 {
            if water_word % 0x3f1 != 0 {
                return Err(Error::Layout("broken water data"));
            }
            for _ in 0..(water_word / 0x3f1 - 1) {
                let surface_flags = r.u32("a surface's flags")?;
                let n = r.u32("a surface's point count")? as usize;
                if n > 100_000 {
                    return Err(Error::Layout("too many water points"));
                }
                let mut points = Vec::with_capacity(n);
                for _ in 0..n {
                    let x = r.f32("a surface's point")?;
                    let y = r.f32("a surface's point")?;
                    let z = r.f32("a surface's point")?;
                    points.push([x, (y * 100.0 + 0.5).floor() / 100.0, z]);
                }
                let triangles = r.u32("a surface's triangle count")? as usize;
                let indices = r.take(triangles * 6, "a surface's triangles")?;
                let index = |k: usize| u16::from_le_bytes([indices[2 * k], indices[2 * k + 1]]) as usize;
                if surface_flags & 1 == 0 && n != 3 {
                    if n > 0 {
                        let mut centre = [0.0f32; 3];
                        for p in &points {
                            for c in 0..3 {
                                centre[c] += p[c];
                            }
                        }
                        let centre = centre.map(|v| v / n as f32);
                        for k in 0..n {
                            let before = if k == 0 { n - 1 } else { k - 1 };
                            water.push(([centre, points[before], points[k]], surface_flags));
                        }
                    }
                } else {
                    for t in 0..triangles {
                        let (a, b, c) = (index(t * 3), index(t * 3 + 1), index(t * 3 + 2));
                        if a < n && b < n && c < n {
                            water.push(([points[a], points[b], points[c]], surface_flags));
                        }
                    }
                }
            }
        }
        let attractors = r.u32("the attractor count")? as usize;
        if attractors >= 1000 {
            return Err(Error::Layout("too many camera attractors"));
        }
        r.take(attractors * 44, "the camera attractors")?;
        rooms.push(Room { flags, rect, position, links: link_list, name, water, light });
    }
    Ok((Some(RoomPlayfield { number, name, tilemap, rooms }), r.at))
}

/// `n3Room_t::DepackLightmap` (0x10010eb7): the zlib-packed light map inflated to `size` bytes; nothing when it does not
/// inflate.
fn depack(packed: &[u8], size: usize) -> Vec<u8> {
    use std::io::Read;
    let mut out = Vec::with_capacity(size.min(1 << 24));
    match flate2::read::ZlibDecoder::new(packed).take(size as u64).read_to_end(&mut out) {
        Ok(_) => out,
        Err(_) => Vec::new(),
    }
}

/// The cell images of a `GNDA` tile-map that a room is cut from, and the cell words worked out from them.
pub struct Grid {
    pub width: usize,
    pub height: usize,
    /// The type of each cell (its place in the type list; 0 where there is no ground).
    pub cells: Vec<u8>,
    pub heights: Vec<u8>,
    /// The codes (`HCDA`) that scale the pieces' heights.
    pub codes: Vec<u8>,
    /// The texture layers of each cell, `layers` to a cell: the floor's, the ceiling's, the walls' (a piece's tile `k` takes
    /// layer `k`).
    pub layers: usize,
    pub tiles: Vec<u8>,
    /// The width of a cell and the height of a step.
    pub scale: f32,
    pub step: f32,
    /// The texture (an instance of type `0xf6959`, with lower qualities in `0xf6967` and `0xf6968`) of each tile value: the
    /// table at the head of the `XTHA` chunk (a count, then that many words).
    pub palette: Vec<u32>,
    pub tables: Tables,
    pub words: Cells,
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
        let (width, height) = (cells.width() as usize, cells.height() as usize);
        let codes = chunk(b"HCDA").and_then(|c| decode(&c.body)).map(|i| i.to_luma8().into_raw()).filter(|v| v.len() == width * height).unwrap_or_else(|| vec![0; width * height]);
        let doors: Vec<u32> = chunk(b"IRHA").map(|c| c.body.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect()).unwrap_or_default();
        // The `TAHA` chunk: a count of lists (a short), each an index byte, a length byte and that many shorts; then an image.
        let (lists, choices) = match chunk(b"TAHA") {
            Some(c) => {
                let mut lists = HashMap::new();
                let mut at = 2;
                let count = c.body.get(..2).map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0);
                for _ in 0..count {
                    let Some(&[index, len]) = c.body.get(at..at + 2) else { break };
                    at += 2;
                    let list: Vec<u16> = (0..len as usize).filter_map(|k| c.body.get(at + 2 * k..at + 2 * k + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))).collect();
                    at += 2 * len as usize;
                    lists.insert(index, list);
                }
                let image = c.body.get(at..).and_then(decode).map(|i| i.to_luma8().into_raw()).filter(|v| v.len() == width * height);
                (lists, image)
            }
            None => (HashMap::new(), None),
        };
        // The `XTHA` chunk (FUN_10077f0a): a count and that many words (the palette; shorts in version 1), then the layer
        // count (a short, unless a PNG follows at once: 3) and the layers' images, three to a PNG (a two-channel remainder is
        // the red and green of one, a one-channel remainder a grey one).
        let (palette, layers, tiles) = match chunk(b"XTHA") {
            Some(c) => {
                let count = c.body.get(..2).map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0) as usize;
                let entry = if c.version == 1 { 2 } else { 4 };
                let palette: Vec<u32> = (0..count).filter_map(|i| c.body.get(2 + entry * i..2 + entry * i + entry).map(|b| if entry == 2 { u16::from_le_bytes([b[0], b[1]]) as u32 } else { u32::from_le_bytes(b.try_into().unwrap()) })).collect();
                let mut at = 2 + entry * count;
                let layers = if c.body.get(at + 1..at + 4) == Some(b"PNG") {
                    3
                } else {
                    let n = c.body.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0) as usize;
                    at += 2;
                    n
                };
                let mut tiles = vec![0u8; width * height * layers];
                let mut base = 0;
                while base < layers {
                    let Some(start) = c.body.get(at..).and_then(|b| b.windows(4).position(|w| w == b"\x89PNG")) else { break };
                    let Some(image) = decode(&c.body[at + start..]) else { break };
                    let left = layers - base;
                    let (pixels, stride): (Vec<u8>, usize) = if left == 1 { (image.to_luma8().into_raw(), 1) } else { (image.to_rgb8().into_raw(), 3) };
                    if pixels.len() == width * height * stride {
                        for cell in 0..width * height {
                            for k in 0..left.min(3) {
                                tiles[cell * layers + base + k] = pixels[cell * stride + k];
                            }
                        }
                    }
                    // (the PNG's own length: the next one follows it)
                    at += start + 8;
                    base += 3;
                }
                (palette, layers, tiles)
            }
            None => (Vec::new(), 0, Vec::new()),
        };
        let cells = cells.into_raw();
        let tables = Tables::new(&map.tile_types, &map.type_tables, lists);
        let images = dungeon::CellImages { width, height, places: &cells, doors: &doors, choices: choices.as_deref() };
        let words = dungeon::cells(&images, &tables);
        Some(Grid { width, height, cells, heights: heights.into_raw(), codes, layers, tiles, scale: map.scale, step: map.second, palette, tables, words })
    }

    fn at(&self, x: i64, y: i64) -> Option<(u8, u8)> {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return None;
        }
        let i = y as usize * self.width + x as usize;
        Some((self.cells[i] & 0x7f, self.heights[i]))
    }

    /// The texture of a tile value (none for 0, which is the default, and for values past the table).
    fn texture(&self, value: u8) -> Option<u32> {
        self.palette.get(value as usize).copied().filter(|&t| value != 0 && t > 1)
    }

    /// The cell's texture layers.
    fn tile(&self, x: i64, y: i64) -> &[u8] {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return &[];
        }
        let at = (y as usize * self.width + x as usize) * self.layers;
        self.tiles.get(at..at + self.layers).unwrap_or(&[])
    }

    /// How far the room's floor sits above its position: `-n3Room_t::+0x38`, which `RDBPlayfield_t::CalculateRoomHeights`
    /// (0x1001bd82) sets to the lowest height of the room's cells that are there (so the lowest floor is at the room's
    /// height); [`FLOOR_BASE`] for a room without any.
    pub fn floor_base(&self, room: &Room) -> f32 {
        let [x0, y0, x1, y1] = room.rect.map(|v| v as i64);
        let mut lowest = f32::MAX;
        for y in y0..y1 {
            for x in x0..x1 {
                if let Some((place, height)) = self.at(x, y)
                    && place != 0
                {
                    lowest = lowest.min(height as f32 * self.step);
                }
            }
        }
        if lowest == f32::MAX { FLOOR_BASE } else { -lowest }
    }

    fn surface(&self) -> Surface<'_> {
        Surface { width: self.width, height: self.height, heights: &self.heights, codes: &self.codes, scale: self.scale, step: self.step }
    }

    /// Every piece the playfield's rooms need, to load before [`model`].
    pub fn pieces_needed(&self, rooms: &RoomPlayfield) -> std::collections::BTreeSet<u16> {
        let mut set = std::collections::BTreeSet::new();
        for room in &rooms.rooms {
            let rect = room.rect.map(|v| v as usize);
            set.extend(dungeon::pieces_needed(&self.words, &self.tables, &self.cells, rect));
        }
        set
    }
}

/// Where a statel of the room's zone stands in the world (and which way it faces): its position is relative to the room's
/// centre, its height too (`n3Room_t::CalcGlobalFromTemplatePos` adds the room's whole position, +0x50), and the room's
/// turn turns both.
pub fn place_statel(room: &Room, position: [f32; 3], rotation: [f32; 4]) -> ([f32; 3], [f32; 4]) {
    let (dx, dz) = room.turn_xz(position[0], position[2]);
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
    ([room.position[0] + dx, room.position[1] + position[1], room.position[2] + dz], q)
}

/// The room's water triangles in the viewer's Z-up world, placed like its statels, with each one's liquid kind.
pub fn water(room: &Room) -> Vec<([[f32; 3]; 3], u32)> {
    room.water
        .iter()
        .map(|(points, flags)| {
            let placed = points.map(|p| {
                let (dx, dz) = room.turn_xz(p[0], p[2]);
                [room.position[0] + dx, room.position[2] + dz, p[1]]
            });
            (placed, flags & 0x1f)
        })
        .collect()
}

/// The width (or height) of a room rounded down to odd: the cells are laid from the centre by half of it.
fn odd(n: i64) -> i64 {
    ((n - 1) & !1) + 1
}

/// A room's cells as a surface (its floor on the tile-map's height samples) with the pieces that wall it, in the viewer's Z-up
/// world (x, the server's z, height). `material` is given to the parts; `pieces` holds the loaded pieces by instance.
pub fn room_parts(grid: &Grid, room: &Room, material: &Material, pieces: &HashMap<u16, Piece>) -> Vec<Part> {
    let [x0, y0, x1, y1] = room.rect.map(|v| v as i64);
    let (w, h) = (x1 - x0, y1 - y0);
    let cell = grid.scale;
    let surface = grid.surface();
    // The centre of a cell in the room's own frame (before its turn).
    let centre = |cx: i64, cy: i64| (((cx - x0) as f32 - odd(w) as f32 / 2.0) * cell, ((cy - y0) as f32 - odd(h) as f32 / 2.0) * cell);
    let (cx0, cz0) = (room.position[0], room.position[2]);
    let base = grid.floor_base(room);
    let floor = room.position[1] + base;
    // A point of the room's frame (Y-up, relative to the centre) in the world.
    let world = |dx: f32, y: f32, dz: f32| {
        let (dx, dz) = room.turn_xz(dx, dz);
        [cx0 + dx, cz0 + dz, y]
    };
    let name = if room.name.is_empty() { "room".to_string() } else { room.name.clone() };
    let textured = |texture: Option<u32>| {
        let mut material = material.clone();
        if let Some(t) = texture {
            material.textures = vec![(TEXTURE, t)];
            material.diffuse = [1.0; 3];
            material.ambient = [1.0; 3];
        }
        material
    };
    // One part per texture (a new one when the 16-bit indices run out).
    let mut groups: BTreeMap<Option<u32>, Vec<Part>> = BTreeMap::new();
    let mut push = |texture: Option<u32>, vertices: &[Vertex], indices: &[u16]| {
        let parts = groups.entry(texture).or_default();
        if parts.last().is_none_or(|part| part.vertices.len() + vertices.len() > 65_535) {
            parts.push(Part { name: name.clone(), world: IDENTITY, vertices: Vec::new(), indices: Vec::new(), material: textured(texture) });
        }
        let part = parts.last_mut().unwrap();
        let base = part.vertices.len() as u16;
        part.vertices.extend_from_slice(vertices);
        part.indices.extend(indices.iter().map(|&i| base + i));
    };
    // The floor: a quad per cell on the four corners' samples (the one before the cell at its near corner). The original
    // draws it apart from the room's pieces (from the height map, not light-mapped), so it stays outside the coloured stream.
    for cy in y0..y1 {
        for cx in x0..x1 {
            let Some((place, _)) = grid.at(cx, cy) else { continue };
            if place == 0 {
                continue;
            }
            let (mx, mz) = centre(cx, cy);
            let half = cell / 2.0;
            let corners = [(mx - half, mz - half, surface.floor(cx - 1, cy - 1)), (mx + half, mz - half, surface.floor(cx, cy - 1)), (mx + half, mz + half, surface.floor(cx, cy)), (mx - half, mz + half, surface.floor(cx - 1, cy))];
            let p: Vec<[f32; 3]> = corners.iter().map(|&(x, z, y)| world(x, floor + y, z)).collect();
            let (d1, d2) = ([p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]], [p[3][0] - p[1][0], p[3][1] - p[1][1], p[3][2] - p[1][2]]);
            let n = [d1[1] * d2[2] - d1[2] * d2[1], d1[2] * d2[0] - d1[0] * d2[2], d1[0] * d2[1] - d1[1] * d2[0]];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
            let normal = if n[2] < 0.0 { [-n[0] / len, -n[1] / len, -n[2] / len] } else { [n[0] / len, n[1] / len, n[2] / len] };
            let uv = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
            let vertices: Vec<Vertex> = (0..4).map(|k| Vertex { position: p[k], normal, uv: uv[k], colour: Vertex::UNBAKED }).collect();
            push(grid.tile(cx, cy).first().and_then(|&v| grid.texture(v)), &vertices, &[0, 1, 2, 0, 2, 3]);
        }
    }
    // The pieces: the room's welded vertex stream with the light map's colour per vertex (FUN_100082a7: white when the map
    // has too few), each texture's triangles gathering the vertices they use.
    let (vertices, lists) = lay_room(grid, room, pieces);
    let colours = dungeon::light_colours(&room.light, vertices.len());
    for (texture, indices) in lists {
        let mut local: HashMap<u32, u16> = HashMap::new();
        let (mut gathered, mut remapped): (Vec<Vertex>, Vec<u16>) = (Vec::new(), Vec::new());
        for triangle in indices.chunks_exact(3) {
            if gathered.len() + 3 > 65_535 {
                push(texture, &gathered, &remapped);
                gathered.clear();
                remapped.clear();
                local.clear();
            }
            for &i in triangle {
                let at = *local.entry(i).or_insert_with(|| {
                    let v = &vertices[i as usize];
                    let (nx, nz) = room.turn_xz(v.normal[0], v.normal[2]);
                    gathered.push(Vertex { position: world(v.position[0], room.position[1] + v.position[1], v.position[2]), normal: [nx, nz, v.normal[1]], uv: v.uv, colour: colours[i as usize] });
                    (gathered.len() - 1) as u16
                });
                remapped.push(at);
            }
        }
        if !remapped.is_empty() {
            push(texture, &gathered, &remapped);
        }
    }
    groups.into_values().flatten().collect()
}

/// The room's pieces' vertices in its own frame (Y-up about the centre, the floor `floor_base` up), one stream for the whole
/// room as the original's `CreateDungeonRoom` fills it — cell by cell along the rows, each tile's vertices welded onto the
/// earlier ones they coincide with — and its triangles by texture, as indices into the stream. The tiles are laid whether or
/// not their texture layer has a texture (the light map colours the stream in this order), but only textured ones are drawn.
/// Replayed through N3.dll's own weld over playfield 1180 (`ao-difftest`'s `n3_weld`), the stream came out identical.
fn lay_room(grid: &Grid, room: &Room, pieces: &HashMap<u16, Piece>) -> (Vec<Laid>, BTreeMap<Option<u32>, Vec<u32>>) {
    let [x0, y0, x1, y1] = room.rect.map(|v| v as i64);
    let (w, h) = (x1 - x0, y1 - y0);
    let cell = grid.scale;
    let surface = grid.surface();
    // The centre of a cell in the room's own frame (before its turn).
    let centre = |cx: i64, cy: i64| (((cx - x0) as f32 - odd(w) as f32 / 2.0) * cell, ((cy - y0) as f32 - odd(h) as f32 / 2.0) * cell);
    let base = grid.floor_base(room);
    let mut vertices: Vec<Laid> = Vec::new();
    let mut lists: BTreeMap<Option<u32>, Vec<u32>> = BTreeMap::new();
    // Where each cell's vertices begin (the original's sliding window of counts): a tile is welded against the cells of the
    // row before, from the column before to the one after, and against everything laid since the cell before began.
    let mut starts = vec![0usize; (w * h).max(0) as usize];
    let start = |starts: &[usize], k: i64| if k < 0 { 0 } else { starts.get(k as usize).copied().unwrap_or(0) };
    for cy in y0..y1 {
        for cx in x0..x1 {
            let k = (cy - y0) * w + (cx - x0);
            starts[k as usize] = vertices.len();
            let above = start(&starts, k - w - 1)..start(&starts, k - w + 2);
            let left = start(&starts, k - 1);
            let Some((place, _)) = grid.at(cx, cy) else { continue };
            if place == 0 {
                continue;
            }
            let (mx, mz) = centre(cx, cy);
            let mut add = |texture: Option<u32>, laid: Vec<Laid>, triangles: &[u16], drawn: bool| {
                let begin = vertices.len();
                vertices.extend(laid);
                let mut indices: Vec<u32> = triangles.iter().map(|&i| (begin + i as usize) as u32).collect();
                dungeon::weld(&mut vertices, &mut indices, begin, above.clone(), left);
                if drawn {
                    lists.entry(texture).or_default().extend(indices);
                }
            };
            let tile = grid.tile(cx, cy);
            // The pieces: each tile of each with the cell's texture layer of its place.
            for (id, turn) in dungeon::cell_pieces(&grid.words, &grid.tables, place, cx as usize, cy as usize) {
                let Some(piece) = pieces.get(&id) else { continue };
                for (k, tile_mesh) in piece.tiles.iter().enumerate() {
                    if tile_mesh.vertices.is_empty() {
                        continue;
                    }
                    let texture = tile.get(k).and_then(|&v| grid.texture(v));
                    let laid = dungeon::lay(&surface, cx as usize, cy as usize, tile_mesh, turn, [mx, base, mz]);
                    let count = laid.len();
                    let triangles: Vec<u16> = tile_mesh.indices.iter().copied().filter(|&i| (i as usize) < count).collect();
                    add(texture, laid, &triangles[..triangles.len() / 3 * 3], texture.is_some());
                }
            }
        }
    }
    (vertices, lists)
}

/// The rooms' floors as the vehicle stands on them (`n3RoomSurface_t::CalculateClosestPoint` 0x10013ee6), in the game's
/// Y-up world: each cell's four height samples (as [`room_parts`] lays them) as two planes, split from the near corner to the
/// far one unless bit 0x4000 of the cell word before it (the turn's low bit) splits them the other way; wound to face up.
pub fn floor_triangles(grid: &Grid, rooms: &RoomPlayfield) -> Vec<[[f32; 3]; 3]> {
    let surface = grid.surface();
    let mut out = Vec::new();
    for room in &rooms.rooms {
        let [x0, y0, x1, y1] = room.rect.map(|v| v as i64);
        let (w, h) = (x1 - x0, y1 - y0);
        let cell = grid.scale;
        let floor = room.position[1] + grid.floor_base(room);
        let world = |dx: f32, y: f32, dz: f32| {
            let (dx, dz) = room.turn_xz(dx, dz);
            [room.position[0] + dx, y, room.position[2] + dz]
        };
        for cy in y0..y1 {
            for cx in x0..x1 {
                let Some((place, _)) = grid.at(cx, cy) else { continue };
                if place == 0 {
                    continue;
                }
                let (mx, mz) = (((cx - x0) as f32 - odd(w) as f32 / 2.0) * cell, ((cy - y0) as f32 - odd(h) as f32 / 2.0) * cell);
                let half = cell / 2.0;
                let p = [
                    world(mx - half, floor + surface.floor(cx - 1, cy - 1), mz - half),
                    world(mx + half, floor + surface.floor(cx, cy - 1), mz - half),
                    world(mx + half, floor + surface.floor(cx, cy), mz + half),
                    world(mx - half, floor + surface.floor(cx - 1, cy), mz + half),
                ];
                let (wx, wy) = ((cx - 1).max(0) as usize, (cy - 1).max(0) as usize);
                let other = grid.words.words.get(wy * grid.words.width + wx).is_some_and(|w| w & 0x4000 != 0);
                let pair = if other { [[p[0], p[1], p[3]], [p[1], p[2], p[3]]] } else { [[p[0], p[1], p[2]], [p[0], p[2], p[3]]] };
                for t in pair {
                    // (the KD records' normal is (p0 − p2) × (p1 − p0): up)
                    let (a, b) = ([t[0][0] - t[2][0], t[0][1] - t[2][1], t[0][2] - t[2][2]], [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]]);
                    let ny = a[2] * b[0] - a[0] * b[2];
                    out.push(if ny >= 0.0 { t } else { [t[0], t[2], t[1]] });
                }
            }
        }
    }
    out
}

/// The ground plane's squares (min x, min z, max x, max z) of the rooms' existing cells: where a character may be
/// (`n3RoomSurface_t::VetoRoomTransition` refuses a place in no room).
pub fn floor_cells(grid: &Grid, rooms: &RoomPlayfield) -> Vec<[f32; 4]> {
    let mut out = Vec::new();
    for room in &rooms.rooms {
        let [x0, y0, x1, y1] = room.rect.map(|v| v as i64);
        let (w, h) = (x1 - x0, y1 - y0);
        let cell = grid.scale;
        for cy in y0..y1 {
            for cx in x0..x1 {
                if !grid.at(cx, cy).is_some_and(|(place, _)| place != 0) {
                    continue;
                }
                let (mx, mz) = (((cx - x0) as f32 - odd(w) as f32 / 2.0) * cell, ((cy - y0) as f32 - odd(h) as f32 / 2.0) * cell);
                let half = cell / 2.0;
                let (ax, az) = room.turn_xz(mx - half, mz - half);
                let (bx, bz) = room.turn_xz(mx + half, mz + half);
                let (ax, az, bx, bz) = (room.position[0] + ax, room.position[2] + az, room.position[0] + bx, room.position[2] + bz);
                out.push([ax.min(bx), az.min(bz), ax.max(bx), az.max(bz)]);
            }
        }
    }
    out
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
    let (cx, cy) = ((dx + odd(w) as f32 / 2.0 + 0.5).floor() as i64, (dy + odd(h) as f32 / 2.0 + 0.5).floor() as i64);
    cx >= 0 && cy >= 0 && cx < w && cy < h && grid.at(x0 + cx, y0 + cy).is_some_and(|(place, _)| place != 0)
}

/// All the rooms of a playfield as one model.
pub fn model(grid: &Grid, rooms: &RoomPlayfield, material: &Material, pieces: &HashMap<u16, Piece>) -> Model {
    let parts: Vec<Part> = rooms.rooms.iter().flat_map(|r| room_parts(grid, r, material, pieces)).collect();
    Model { parts, instances: Vec::new(), water: None, lod_groups: Vec::new() }
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

    fn room(flags: u8, rect: [u16; 4], position: [f32; 3], name: Option<&str>, water: bool) -> Vec<u8> {
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
        // A light map of 8 bytes, water or none, one camera attractor.
        d.extend_from_slice(&12u32.to_le_bytes());
        d.extend_from_slice(&416u32.to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        if water {
            // one square surface, fanned
            d.extend_from_slice(&(2 * 0x3f1u32).to_le_bytes());
            d.extend_from_slice(&2u32.to_le_bytes());
            d.extend_from_slice(&4u32.to_le_bytes());
            for p in [[0.0f32, 1.004, 0.0], [4.0, 1.004, 0.0], [4.0, 1.004, 4.0], [0.0, 1.004, 4.0]] {
                for v in p {
                    d.extend_from_slice(&v.to_le_bytes());
                }
            }
            d.extend_from_slice(&0u32.to_le_bytes());
        } else {
            d.extend_from_slice(&0x3f1u32.to_le_bytes());
        }
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&[0; 44]);
        d
    }

    #[test]
    fn rooms_are_read_in_order() {
        let data = object(&[room(0, [10, 115, 21, 124], [208.0, 5.0, 202.0], None, false), room(0x83, [8, 34, 29, 70], [146.0, 5.0, 174.0], Some("Clan_Basic_2"), false)]);
        let p = parse(&data).unwrap().unwrap();
        assert_eq!((p.number, p.tilemap, p.name.as_str()), (1180, 1140, "shop"));
        assert_eq!(p.rooms.len(), 2);
        assert_eq!(p.rooms[0].rect, [10, 115, 21, 124]);
        assert_eq!(p.rooms[0].position, [208.0, 5.0, 202.0]);
        assert_eq!(p.rooms[0].links, vec![(5, 10)]);
        assert_eq!((p.rooms[1].name.as_str(), p.rooms[1].turn()), ("Clan_Basic_2", 3));
    }

    #[test]
    fn a_rooms_water_is_fanned_and_placed() {
        let data = object(&[room(1, [0, 0, 4, 4], [100.0, 5.0, 200.0], None, true)]);
        let p = parse(&data).unwrap().unwrap();
        let w = &p.rooms[0].water;
        assert_eq!(w.len(), 4);
        // the height is rounded to a hundredth; the centre is the square's middle
        assert_eq!(w[0].0[0], [2.0, 1.0, 2.0]);
        let placed = water(&p.rooms[0]);
        // a quarter turn: (x, z) -> (z, -x); the viewer's frame is (x, z, y)
        assert_eq!(placed[0].0[0], [102.0, 198.0, 1.0]);
        assert_eq!(placed[0].1, 2);
    }

    impl Grid {
        fn clone_heights(&self, h: u8) -> Grid {
            let mut g = grid(self.width, self.height, vec![h; self.width * self.height]);
            g.step = self.step;
            g
        }
    }

    fn grid(width: usize, height: usize, heights: Vec<u8>) -> Grid {
        let cells = vec![1u8; width * height];
        let tables = Tables::new(&[0, 1], &[], HashMap::new());
        let images = dungeon::CellImages { width, height, places: &cells, doors: &[], choices: None };
        let words = dungeon::cells(&images, &tables);
        Grid { width, height, cells, heights, codes: vec![0; width * height], layers: 0, tiles: Vec::new(), scale: 2.0, step: 0.2, palette: Vec::new(), tables, words }
    }

    #[test]
    fn a_flat_room_is_a_floor_at_its_height() {
        let grid = grid(4, 4, vec![0; 16]);
        let room = Room { flags: 0, rect: [0, 0, 2, 2], position: [10.0, 5.0, 20.0], links: Vec::new(), name: String::new(), water: Vec::new(), light: Vec::new() };
        let part = room_parts(&grid, &room, &Material::default(), &HashMap::new()).remove(0);
        // four cells, no walls
        assert_eq!(part.vertices.len(), 16);
        let xs: Vec<f32> = part.vertices.iter().map(|v| v.position[0]).collect();
        // two cells wide: odd width 1, so the cells are centred at -1 and +1 (relative), spanning -2 .. 2
        assert!(xs.iter().all(|x| (8.0..=12.0).contains(x)), "{xs:?}");
        // (the room's lowest cell is at its height: `CalculateRoomHeights`)
        assert!(part.vertices.iter().all(|v| v.position[2] == 5.0));
        let mut raised = grid.clone_heights(3);
        assert_eq!(raised.floor_base(&room), -0.6);
        raised.heights[0] = 0;
        assert_eq!(raised.floor_base(&room), 0.0);
    }

    #[test]
    fn the_floor_rests_on_the_samples_before_each_cell() {
        // sample (1, 1) is the far corner of cell (1, 1) and the near corner of cell (2, 2)
        let mut heights = vec![0u8; 16];
        heights[5] = 10;
        let grid = grid(4, 4, heights);
        let room = Room { flags: 0, rect: [0, 0, 4, 4], position: [0.0, 0.0, 0.0], links: Vec::new(), name: String::new(), water: Vec::new(), light: Vec::new() };
        let part = room_parts(&grid, &room, &Material::default(), &HashMap::new()).remove(0);
        let raised: Vec<&Vertex> = part.vertices.iter().filter(|v| v.position[2] > 1.5).collect();
        // the corner is shared by four cells, whose quads tilt apart there: the weld keeps vertices whose normals differ
        assert_eq!(raised.len(), 4, "{raised:?}");
        assert!(raised.iter().all(|v| (v.position[2] - 2.0).abs() < 1e-5));
        // cells 0..4 centred at (k - 1.5) * 2 (odd width 3): the corner between cells 1 and 2 is at 0
        assert!(raised.iter().all(|v| v.position[0].abs() < 1e-5 && v.position[1].abs() < 1e-5), "{raised:?}");
    }

    #[test]
    fn a_turned_room_swaps_its_sides() {
        let grid = grid(8, 8, vec![0; 64]);
        let room = Room { flags: 1, rect: [0, 0, 4, 2], position: [0.0, 0.0, 0.0], links: Vec::new(), name: String::new(), water: Vec::new(), light: Vec::new() };
        let part = room_parts(&grid, &room, &Material::default(), &HashMap::new()).remove(0);
        let (mut xs, mut zs) = (Vec::new(), Vec::new());
        for v in &part.vertices {
            xs.push(v.position[0]);
            zs.push(v.position[1]);
        }
        let span = |v: &[f32]| v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min);
        assert_eq!((span(&xs), span(&zs)), (4.0, 8.0));
    }

    #[test]
    fn covers_agrees_with_the_floor() {
        let grid = grid(8, 8, vec![0; 64]);
        let room = Room { flags: 0, rect: [0, 0, 3, 3], position: [100.0, 0.0, 200.0], links: Vec::new(), name: String::new(), water: Vec::new(), light: Vec::new() };
        let part = room_parts(&grid, &room, &Material::default(), &HashMap::new()).remove(0);
        let min_x = part.vertices.iter().map(|v| v.position[0]).fold(f32::MAX, f32::min);
        let max_x = part.vertices.iter().map(|v| v.position[0]).fold(f32::MIN, f32::max);
        assert!(covers(&grid, &room, min_x + 0.1, 200.0, false));
        assert!(covers(&grid, &room, max_x - 0.1, 200.0, false));
        assert!(!covers(&grid, &room, min_x - 0.1, 200.0, false));
        assert!(!covers(&grid, &room, max_x + 0.1, 200.0, false));
    }

    fn dump_weld_scenario(grid: &Grid, room: &Room, pieces: &HashMap<u16, Piece>, out: &mut Vec<u8>) -> usize {
        let [x0, y0, x1, y1] = room.rect.map(|v| v as i64);
        let (w, h) = (x1 - x0, y1 - y0);
        let cell = grid.scale;
        let surface = grid.surface();
        let centre = |cx: i64, cy: i64| (((cx - x0) as f32 - odd(w) as f32 / 2.0) * cell, ((cy - y0) as f32 - odd(h) as f32 / 2.0) * cell);
        let base = grid.floor_base(room);
        let mut vertices: Vec<Laid> = Vec::new();
        let mut starts = vec![0usize; (w * h).max(0) as usize];
        let start = |starts: &[usize], k: i64| if k < 0 { 0 } else { starts.get(k as usize).copied().unwrap_or(0) };
        let mut tiles = 0usize;
        let put_vertex = |out: &mut Vec<u8>, v: &Laid| {
            for c in v.position.iter().chain(v.normal.iter()) {
                out.extend_from_slice(&c.to_le_bytes());
            }
            out.extend_from_slice(&0u32.to_le_bytes());
            for c in &v.uv {
                out.extend_from_slice(&c.to_le_bytes());
            }
        };
        for cy in y0..y1 {
            for cx in x0..x1 {
                let k = (cy - y0) * w + (cx - x0);
                starts[k as usize] = vertices.len();
                let above = start(&starts, k - w - 1)..start(&starts, k - w + 2);
                let left = start(&starts, k - 1);
                let Some((place, _)) = grid.at(cx, cy) else { continue };
                if place == 0 {
                    continue;
                }
                let (mx, mz) = centre(cx, cy);
                for (id, turn) in dungeon::cell_pieces(&grid.words, &grid.tables, place, cx as usize, cy as usize) {
                    let Some(piece) = pieces.get(&id) else { continue };
                    for tile_mesh in &piece.tiles {
                        if tile_mesh.vertices.is_empty() {
                            continue;
                        }
                        let laid = dungeon::lay(&surface, cx as usize, cy as usize, tile_mesh, turn, [mx, base, mz]);
                        let count = laid.len();
                        let begin = vertices.len();
                        let mut indices: Vec<u32> = tile_mesh.indices.iter().copied().filter(|&i| (i as usize) < count).map(|i| (begin + i as usize) as u32).collect();
                        indices.truncate(indices.len() / 3 * 3);
                        for v in [begin, k as usize, w as usize, count] {
                            out.extend_from_slice(&(v as u32).to_le_bytes());
                        }
                        for v in &laid {
                            put_vertex(out, v);
                        }
                        out.extend_from_slice(&(indices.len() as u32).to_le_bytes());
                        for &i in &indices {
                            out.extend_from_slice(&(i as u16).to_le_bytes());
                        }
                        vertices.extend(laid);
                        dungeon::weld(&mut vertices, &mut indices, begin, above.clone(), left);
                        out.extend_from_slice(&((vertices.len() - begin) as u32).to_le_bytes());
                        for v in &vertices[begin..] {
                            put_vertex(out, v);
                        }
                        for &i in &indices {
                            out.extend_from_slice(&(i as u16).to_le_bytes());
                        }
                        tiles += 1;
                    }
                }
            }
        }
        tiles
    }

    /// With `MAO_GAME`: the shop's rooms weld to the counts N3.dll's own weld gives over the same tiles (`ao-difftest`'s
    /// `n3_weld` replay, which `MAO_WELD_DUMP=<file>` here feeds). Their light maps hold more colours than that (416, 646,
    /// 132, 132, 705, 132 and 2382): they were baked from other geometry, and the original colours its vertices from the
    /// start of the map just the same.
    #[test]
    fn the_shops_rooms_weld_as_the_original_does() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        use crate::resource::{Id, ResourceDatabase};
        let db = ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let rooms = parse(&db.get(Id { kind: KIND, instance: 1180 }).unwrap().unwrap()).unwrap().unwrap();
        let map = crate::tilemap::parse_new(&db.get(Id { kind: 0xf4249, instance: rooms.tilemap }).unwrap().unwrap()).unwrap();
        let grid = Grid::new(&map).unwrap();
        let mut pieces = HashMap::new();
        for id in grid.pieces_needed(&rooms) {
            if let Ok(Some(o)) = db.get(Id { kind: dungeon::PIECE, instance: id as u32 })
                && let Some(p) = dungeon::parse_piece(&o)
            {
                pieces.insert(id, p);
            }
        }
        let counts: Vec<usize> = rooms.rooms.iter().map(|room| lay_room(&grid, room, &pieces).0.len()).collect();
        assert_eq!(counts, [226, 349, 108, 108, 515, 108, 941]);
        let colours: Vec<usize> = rooms.rooms.iter().map(|room| room.light.len() / 2).collect();
        assert_eq!(colours, [416, 646, 132, 132, 705, 132, 2382]);
        // every room is coloured (the map is never short), and a wall vertex takes the map's colour with full alpha
        let parts = room_parts(&grid, &rooms.rooms[0], &Material::default(), &pieces);
        let walls: Vec<&Vertex> = parts.iter().flat_map(|p| &p.vertices).filter(|v| v.colour[3] == 255).collect();
        assert!(!walls.is_empty() && walls.iter().any(|v| v.colour[..3] != [255, 255, 255]), "the light map colours the walls");
        if let Some(path) = std::env::var_os("MAO_WELD_DUMP") {
            let mut body = Vec::new();
            let mut total = 0usize;
            for room in &rooms.rooms {
                total += dump_weld_scenario(&grid, room, &pieces, &mut body);
                // a room boundary: a tile record with begin = u32::MAX, then the light map's colour count and our total
                body.extend_from_slice(&u32::MAX.to_le_bytes());
                body.extend_from_slice(&((room.light.len() / 2) as u32).to_le_bytes());
                body.extend_from_slice(&(lay_room(&grid, room, &pieces).0.len() as u32).to_le_bytes());
            }
            let mut out = (total as u32).to_le_bytes().to_vec();
            out.extend_from_slice(&body);
            std::fs::write(&path, out).unwrap();
        }
    }
}
