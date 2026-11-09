//! The pieces that furnish a dungeon's cells — the walls, corners and doorways of a shop, a mission or a subway — as `N3.dll`
//! builds them: `n3GroundRenderer_t::CreateDungeonRoom` (FUN_10008563) makes a `DungeonMesh_t` per cell of a room (FUN_10001a91
//! picks the pieces, FUN_10001c3b lays their vertices on the cell's heights), from the cell words `n3Tilemap_t`'s constructor
//! (FUN_100173f8, the big FUN_100169ab) works out once per tile-map from the cell images. The floor itself is not a piece: it is
//! the tile-map's height surface (`rooms.rs`).
//!
//! # The pieces
//! A piece is a `VisualTile_t` (type 0xf695d, `dropin/displaysystem/src/rdbtile.rs`): a count of tiles, each a count of
//! vertices (eight floats: the position, which spans the cell as -1..1 about its centre, the normal, u and v), a count of
//! indices and the indices. Tile `k` of a piece is drawn with the cell's `k`th texture layer (`XTHA`): tile 0 the floor's,
//! tile 1 the wall's.
//!
//! # The cell words
//! For every cell the constructor stores a word `piece | turn << 14` and an info word `neighbour << 16 | flags << 8 | turn`
//! whose top byte holds the wall bits (`n3Tilemap_t` +0x10 and +0x14; the doors of the `IRHA` chunk set bit 28 of both cells
//! they join). The flags of a cell say which of its four sides face an empty cell (1 the row before, 2 the column after, 4 the
//! row after, 8 the column before, in the constructor's row order, which runs the image bottom-up) and whether a door opens in
//! one (0x10, with 0x20/0x40 variants). The piece comes from the type's table ([`crate::tilemap::TypeTable`]) for its
//! neighbouring type: the flags' wall bits are turned to their smallest value (which fixes the turn), joined with the door
//! bits and looked up in the pattern list `0f 07 05 03 01 00 31 51 71 1d 33 11 0f 1f` (`N3.dll` .data 0x1005bce0; its place
//! is the table slot, FUN_10017306 and FUN_10016967). A cell without walls gets piece 0: none.
//!
//! # Laying a piece
//! FUN_10001c3b turns the piece's vertices by `turn × 90° + 180°` about the up axis, then bends them onto the floor: the four
//! height samples around the cell give a bilinear floor, the twelve around those a cubic correction (FUN_100023ad in x,
//! FUN_10002479 in z, evaluated by FUN_1000244e / FUN_1000251c), both for the heights (`DHGA`, the floor) and for the codes
//! (`HCDA`, which scale the vertex's own height by `1 + 0.25 × code × step`). The normals bend with the floor's slope. The
//! height sample `(i, j)` sits at the corner `((i + 1) × scale, (j + 1) × scale)` of the tile-map (FUN_1001769d), so a cell
//! `(x, y)` rests on samples `(x - 1 .. x, y - 1 .. y)`.
//!
//! Each tile's new vertices are welded onto the earlier ones they coincide with ([`weld`], FUN_10001645, which also shifts the
//! tile's texture coordinates by whole tiles so neighbours share the vertex), and the room's packed light map (a 565 colour per
//! vertex in that welded order, `n3Room_t::DepackLightmap` and FUN_100082a7, [`light_colours`]) colours them.

use std::collections::{BTreeSet, HashMap};

use crate::tilemap::TypeTable;

/// The database type of a piece.
pub const PIECE: u32 = 0xf695d;

#[derive(Clone, Debug, Default)]
pub struct PieceTile {
    /// Position, normal, u, v.
    pub vertices: Vec<[f32; 8]>,
    pub indices: Vec<u16>,
}

#[derive(Clone, Debug, Default)]
pub struct Piece {
    pub tiles: Vec<PieceTile>,
}

/// `VisualTile_t::ReadBlob` (FUN_1006ee4e).
pub fn parse_piece(object: &[u8]) -> Option<Piece> {
    let mut at = crate::texture::OBJECT_HEAD;
    let u16_at = |at: &mut usize| -> Option<u16> {
        let v = u16::from_le_bytes(object.get(*at..*at + 2)?.try_into().ok()?);
        *at += 2;
        Some(v)
    };
    let count = u16_at(&mut at)? as usize;
    let mut tiles = Vec::with_capacity(count);
    for _ in 0..count {
        let vertices = u16_at(&mut at)? as usize;
        let mut list = Vec::with_capacity(vertices);
        for _ in 0..vertices {
            let b = object.get(at..at + 32)?;
            at += 32;
            let mut v = [0f32; 8];
            for (k, f) in v.iter_mut().enumerate() {
                *f = f32::from_le_bytes(b[4 * k..4 * k + 4].try_into().unwrap());
            }
            list.push(v);
        }
        let indices = u16_at(&mut at)? as usize / 3 * 3;
        let mut index_list = Vec::with_capacity(indices);
        for _ in 0..indices {
            index_list.push(u16_at(&mut at)?);
        }
        tiles.push(PieceTile { vertices: list, indices: index_list });
    }
    Some(Piece { tiles })
}

/// The wall-pattern tables of a `GNDA` map (FUN_10017306): for each wall code 0..0x80, the slot of its pattern in the type
/// tables (1-based, 0 for none) and the turn that brings its wall bits to the pattern.
fn wall_tables() -> ([u8; 128], [u8; 128]) {
    const PATTERNS: [u8; 14] = [0x0f, 0x07, 0x05, 0x03, 0x01, 0x00, 0x31, 0x51, 0x71, 0x1d, 0x33, 0x11, 0x0f, 0x1f];
    // (the turn each pattern asks for: all 0 for the new maps; 4 would mean any)
    const TURNS: [u8; 14] = [0; 14];
    let (mut slot, mut turn) = ([0u8; 128], [0u8; 128]);
    for code in 0..128u32 {
        let mut bits = code & 0xf;
        let (mut best, mut best_turn) = (0xffu32, 0u32);
        for t in 0..4 {
            if bits < best {
                best = bits;
                best_turn = t;
            }
            bits = (bits >> 1) + (bits & 1) * 8;
        }
        let key = (best | (code & 0x70)) as u8;
        match PATTERNS.iter().position(|&p| p == key) {
            Some(i) => {
                slot[code as usize] = i as u8 + 1;
                turn[code as usize] = if TURNS[i] == 4 { 4 } else { (TURNS[i] as u32 + best_turn) as u8 & 3 };
            }
            None => {
                slot[code as usize] = 0;
                turn[code as usize] = 0;
            }
        }
    }
    (slot, turn)
}

/// The table FUN_100169ab keeps at `N3.dll` 0x1003d718: the wall bits a cell's flags leave for it.
const WALL_BITS: [u8; 16] = [0x0f, 0x09, 0x03, 0x01, 0x06, 0x00, 0x02, 0x00, 0x0c, 0x08, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00];

/// A tile-map's piece tables, by the place of each type in its type list (which is what the `DCGA` image holds).
pub struct Tables {
    /// `[(type place × 32 + neighbour place) × 16 + slot]`, as `RDBTilemap_t` +0x23c.
    flat: Vec<u16>,
    /// Each type's alternative pieces (`RDBTilemap_t` +0x61c: the count, the type, the list).
    heights: Vec<(u16, Vec<u16>)>,
    /// The `TAHA` lists by their index.
    lists: HashMap<u8, Vec<u16>>,
    slot: [u8; 128],
    turn: [u8; 128],
}

impl Tables {
    pub fn new(types: &[u16], tables: &[TypeTable], lists: HashMap<u8, Vec<u16>>) -> Tables {
        let place = |tile: u16| types.iter().take(32).position(|&t| t == tile).unwrap_or(0);
        let mut flat = vec![0u16; 32 * 32 * 16];
        let mut heights = Vec::new();
        for table in tables {
            let at = place(table.tile);
            for (other, slots) in &table.neighbours {
                let nb = place(*other);
                flat[(at * 32 + nb) * 16..(at * 32 + nb) * 16 + 16].copy_from_slice(slots);
            }
            heights.push((table.tile, table.heights.clone()));
        }
        let (slot, turn) = wall_tables();
        Tables { flat, heights, lists, slot, turn }
    }

    /// FUN_10016967: the piece for a cell of type `place` beside one of type `neighbour` with wall code `code`, and its turn.
    pub fn piece(&self, place: u8, neighbour: u8, code: u32) -> (u16, u8) {
        let code = (code & 0x7f) as usize;
        let at = (place as i64 * 32 + neighbour as i64) * 16 + self.slot[code] as i64 - 1;
        let piece = if at < 0 { 0 } else { self.flat.get(at as usize).copied().unwrap_or(0) };
        (piece, self.turn[code])
    }

    /// FUN_10016324: one of the type's alternative pieces, by the hash's trailing ones.
    fn variant(&self, hash: u32, place: u8) -> u16 {
        let Some((tile, list)) = self.heights.get(place as usize) else { return 0 };
        let modulus = (1u32 << ((list.len() as u32 + 1) & 0x1f)).wrapping_sub(1).max(1);
        let mut v = hash % modulus;
        let mut index = 0usize;
        while v & 1 != 0 {
            index += 1;
            v /= 2;
        }
        if index == 0 { *tile } else { list.get(index - 1).copied().unwrap_or(0) }
    }
}

/// The images the constructor reads: the types (`DCGA`, a type's place with bit 7 as a door mark), the doors (`IRHA`) and the
/// optional `TAHA` image of explicit choices.
pub struct CellImages<'a> {
    pub width: usize,
    pub height: usize,
    pub places: &'a [u8],
    pub doors: &'a [u32],
    pub choices: Option<&'a [u8]>,
}

/// The cell words of a tile-map (`n3Tilemap_t` +0x10 and +0x14).
#[derive(Clone, Debug, Default)]
pub struct Cells {
    pub width: usize,
    pub height: usize,
    pub words: Vec<u16>,
    pub info: Vec<u32>,
}

impl Cells {
    /// The piece word of the cell at image column `x`, row `y`: `(piece, turn)`, piece 0 for none.
    pub fn piece(&self, x: usize, y: usize) -> (u16, u8) {
        let w = self.words.get(y * self.width + x).copied().unwrap_or(0);
        (w & 0x3fff, (w >> 14) as u8)
    }

    /// The wall bits of the cell (FUN_1001662d).
    pub fn wall_bits(&self, x: usize, y: usize) -> u8 {
        (self.info.get(y * self.width + x).copied().unwrap_or(0) >> 24) as u8 & 0xf
    }
}

struct Builder<'a> {
    w: i64,
    h: i64,
    images: &'a CellImages<'a>,
    tables: &'a Tables,
    cells: Cells,
}

impl Builder<'_> {
    fn index(&self, x: i64, row: i64) -> Option<usize> {
        let at = row * self.w + x;
        (at >= 0 && (at as usize) < self.cells.info.len()).then_some(at as usize)
    }

    /// FUN_100166c2: the type at column `x`, row `yp` counted from the image's bottom, clamped.
    fn kind(&self, x: i64, yp: i64) -> u8 {
        let row = (self.h - yp - 1).clamp(0, self.h - 1);
        let x = x.clamp(0, self.w - 1);
        self.images.places[(row * self.w + x) as usize]
    }

    /// FUN_1001660b.
    fn door(&self, x: i64, row: i64) -> bool {
        self.index(x, row).is_some_and(|i| self.cells.info[i] >> 28 & 1 != 0)
    }

    /// FUN_1001662d.
    fn wall_bits(&self, x: i64, row: i64) -> u8 {
        self.index(x, row).map_or(0, |i| (self.cells.info[i] >> 24) as u8 & 0xf)
    }

    /// FUN_1001664f.
    fn set_wall_bits(&mut self, x: i64, row: i64, bits: u8) {
        if let Some(i) = self.index(x, row) {
            self.cells.info[i] = (self.cells.info[i] & 0xf0ff_ffff) | (bits as u32) << 24;
        }
    }

    /// FUN_10016940: the `TAHA` image's byte.
    fn choice(&self, x: i64, row: i64) -> u8 {
        match self.images.choices {
            Some(image) if x >= 0 && x < self.w && row >= 0 && row < self.h => image[(row * self.w + x) as usize],
            _ => 0,
        }
    }

    /// FUN_100166fe: the type the cell at `(x, yp)` sees across its side — the neighbour's stored neighbour type when that
    /// neighbour has a wall toward it (`mode` 1: the row before; 0: the column before), else the neighbour's type.
    fn across(&self, mut x: i64, mut yp: i64, mode: i64) -> u8 {
        if x < 1 {
            if mode == 0 {
                return self.kind(0, yp);
            }
            x = 0;
        }
        let mut fall_back = false;
        if yp < 1 {
            if mode != 0 {
                fall_back = true;
            } else {
                yp = 0;
            }
        }
        if !fall_back && x < self.w && yp < self.h {
            let at = (self.h - yp - 1 + mode) * self.w + x + mode - 1;
            if at >= 0 {
                if let Some(&word) = self.cells.info.get(at as usize) {
                    let (flags, neighbour) = (word >> 8, (word >> 16) as u8);
                    if (mode == 1 && flags & 4 != 0) || (mode == 0 && flags & 2 != 0) {
                        return neighbour;
                    }
                }
            }
            yp -= mode;
            x += mode - 1;
        }
        self.kind(x, yp)
    }

    /// FUN_100168f6.
    fn store(&mut self, x: i64, row: i64, piece: u16, turn: u8, neighbour: u8, flags: u32) {
        if let Some(i) = self.index(x, row) {
            self.cells.words[i] = (turn as u16) << 14 | piece;
            self.cells.info[i] = (self.cells.info[i] & 0xff00_0000) | (neighbour as u32) << 16 | (flags & 0xff) << 8 | turn as u32;
        }
    }

    /// FUN_100169ab over the whole map (the constructor passes the rectangle `-1 .. width + 1`, `-1 .. height + 1`, clamped
    /// to the map): the last column and the top image row are left out, as the original leaves them.
    fn build(&mut self) {
        let (w, h) = (self.w, self.h);
        if w < 2 || h < 2 {
            return;
        }
        const MASK: u8 = 0x7f;
        // the rows yp - 1, yp, yp + 1, yp + 2 of types, each from column -1 to column w
        let row_of = |b: &Builder, yp: i64| -> Vec<u8> { (-1..=w).map(|x| b.kind(x, yp)).collect() };
        let mut rows = [row_of(self, 0), row_of(self, 1), row_of(self, 2), vec![0u8; (w + 2) as usize]];
        let mut below: Vec<u8> = (0..w - 1).map(|x| self.across(x, 1, 1)).collect();
        for yp in 1..h {
            rows[3][w as usize] = self.kind(w - 1, yp + 2);
            rows[3][w as usize + 1] = self.kind(w, yp + 2);
            rows[3][0] = self.kind(-1, yp + 2);
            let mut left = self.across(0, yp, 0);
            let row = h - yp - 1;
            let row_before = h - yp;
            for x in 0..w - 1 {
                let ahead = self.kind(x, yp + 2);
                rows[3][x as usize + 1] = ahead;
                let own = rows[1][x as usize + 1];
                let up = rows[2][x as usize + 1];
                let right = rows[1][x as usize + 2];
                let under = below[x as usize];
                let own_type = own & MASK;
                let mut flags = 0u32;
                if own_type != 0 {
                    flags = (under & MASK == 0) as u32;
                    if right & MASK == 0 {
                        flags |= 2;
                    }
                    if up & MASK == 0 {
                        flags |= 4;
                    }
                    if left & MASK == 0 {
                        flags |= 8;
                    }
                }
                let own_mark = own & !MASK;
                let door = self.door(x, row);
                let choice = self.choice(x, row);
                if own_type != 0 {
                    if own_mark != 0 || door {
                        let left_mark = self.kind(x - 1, yp) & !MASK;
                        let under_mark = self.kind(x, yp - 1) & !MASK;
                        if own_mark & under_mark != 0 {
                            flags |= 2;
                        }
                        if left_mark & own_mark != 0 {
                            flags |= 4;
                        }
                        if door {
                            let mut side = 0;
                            if flags & 1 != 0 && self.door(x, row_before) {
                                side = 1;
                            }
                            if flags & 2 != 0 && self.door(x + 1, row) {
                                side = 2;
                            }
                            if flags & 4 != 0 && self.door(x, row_before - 2) {
                                side = 4;
                            }
                            if flags & 8 != 0 && self.door(x - 1, row) {
                                side = 8;
                            }
                            if side != 0 {
                                flags |= 0x10;
                                if choice & 1 != 0 {
                                    let c_up = self.choice(x, row_before) & 1;
                                    let c_right = self.choice(x + 1, row) & 1;
                                    let c_down = self.choice(x, row_before - 2) & 1;
                                    let c_left = self.choice(x - 1, row) & 1;
                                    let mut v: u8 = 7;
                                    let mut more = true;
                                    if side == 1 {
                                        if c_left == 0 {
                                            v = 1;
                                        }
                                        if c_right == 0 {
                                            v &= 0xfc;
                                        } else {
                                            more = false;
                                        }
                                    }
                                    if more {
                                        if side == 2 {
                                            if c_up == 0 {
                                                v &= 0xf9;
                                            }
                                            if c_down == 0 {
                                                v &= 0xfc;
                                            }
                                        } else if side == 4 {
                                            if c_right == 0 {
                                                v &= 0xf9;
                                            }
                                            if c_left == 0 {
                                                v &= 0xfc;
                                            }
                                        } else if side == 8 {
                                            if c_down == 0 {
                                                v &= 0xf9;
                                            }
                                            if c_up == 0 {
                                                v &= 0xfc;
                                            }
                                        }
                                    }
                                    if v == 7 {
                                        flags = flags & !0x20 | 0x50;
                                    } else if v == 1 {
                                        flags = flags & !0x40 | 0x30;
                                    } else if v == 4 {
                                        flags |= 0x70;
                                    }
                                }
                            }
                        }
                    }
                    // the wall bits this cell keeps: not toward the map's edge, and not doubled by a neighbour's
                    let mut edge = 0xfu8;
                    if x == w - 2 {
                        edge &= 3;
                    }
                    if yp == h - 1 {
                        edge &= 6;
                    }
                    if x == 0 {
                        edge &= 0xc;
                    }
                    if yp == 1 {
                        edge &= 9;
                    }
                    let mut bits = WALL_BITS[(flags & 0xf) as usize] & edge;
                    if bits & 2 != 0 {
                        let other = self.wall_bits(x - 1, row + 1);
                        if other & 8 != 0 {
                            self.set_wall_bits(x - 1, row + 1, other & !8);
                            bits &= !2;
                        }
                    }
                    if bits & 4 != 0 {
                        let other = self.wall_bits(x + 1, row + 1);
                        if other & 1 != 0 {
                            self.set_wall_bits(x + 1, row + 1, other & !1);
                            bits &= !4;
                        }
                    }
                    let old = self.wall_bits(x, row);
                    self.set_wall_bits(x, row, (old & !edge) | bits);
                    if choice & 3 == 2 {
                        flags = 0;
                    }
                }
                let mut neighbour = own;
                if flags != 0 {
                    neighbour = 0;
                    if own == 0 {
                        flags = 0;
                    }
                }
                let (mut piece, mut turn) = self.tables.piece(own_type, neighbour & MASK, flags);
                let explicit = choice >> 2;
                if explicit != 0 {
                    piece = self.tables.lists.get(&own_type).and_then(|l| l.get(explicit as usize - 1)).copied().unwrap_or(piece);
                }
                if turn == 4 {
                    let hash = (yp as u32).wrapping_mul(0x13171) ^ (x as u32).wrapping_mul(0x10651);
                    turn = ((hash as i32 / 3) & 3) as u8;
                    if flags == 0 {
                        piece = self.tables.variant(hash, own_type);
                    }
                }
                self.store(x, row, piece, turn, neighbour, flags);
                left = if flags & 2 != 0 { neighbour } else { own };
                below[x as usize] = if flags & 4 != 0 { neighbour } else { own };
            }
            rows.rotate_left(1);
        }
    }
}

/// `n3Tilemap_t`'s constructor (FUN_100173f8): the doors marked, then the cell words of the whole map.
pub fn cells(images: &CellImages, tables: &Tables) -> Cells {
    let (w, h) = (images.width, images.height);
    let mut cells = Cells { width: w, height: h, words: vec![0; w * h], info: vec![0; w * h] };
    for &door in images.doors {
        let at = (door >> 1) as usize;
        let other = at + if door & 1 == 0 { 1 } else { w };
        for i in [at, other] {
            if let Some(word) = cells.info.get_mut(i) {
                *word |= 0x1000_0000;
            }
        }
    }
    let mut builder = Builder { w: w as i64, h: h as i64, images, tables, cells };
    builder.build();
    builder.cells
}

/// `DungeonMesh_t`'s choice of pieces for a cell (FUN_10001a91): `(piece, turn)` in the order they are laid — a wall piece
/// (the type's table beside type 0, pattern 0x1f) for each wall bit, turned by the bit's place, then the cell's own piece.
pub fn cell_pieces(cells: &Cells, tables: &Tables, place: u8, x: usize, y: usize) -> Vec<(u16, u8)> {
    let (piece, turn) = cells.piece(x, y);
    if piece == 0 {
        return Vec::new();
    }
    let mut list = Vec::new();
    let bits = cells.wall_bits(x, y);
    if bits != 0 {
        let (wall, _) = tables.piece(place & 0x7f, 0, 0x1f);
        for bit in 0..4 {
            if bits & 1 << bit != 0 {
                list.push((wall, bit));
            }
        }
    }
    list.push((piece, turn));
    list
}

/// Every piece a rectangle of cells needs.
pub fn pieces_needed(cells: &Cells, tables: &Tables, places: &[u8], rect: [usize; 4]) -> BTreeSet<u16> {
    let mut set = BTreeSet::new();
    for y in rect[1]..rect[3].min(cells.height) {
        for x in rect[0]..rect[2].min(cells.width) {
            for (piece, _) in cell_pieces(cells, tables, places[y * cells.width + x], x, y) {
                set.insert(piece);
            }
        }
    }
    set
}

/// The height images a piece is bent onto.
pub struct Surface<'a> {
    pub width: usize,
    pub height: usize,
    /// `DHGA`.
    pub heights: &'a [u8],
    /// `HCDA` as stored in the file (the loader adds 32 and the mesh takes them off again; a byte of 224 or more is negative).
    pub codes: &'a [u8],
    pub scale: f32,
    pub step: f32,
}

impl Surface<'_> {
    fn sample(&self, image: &[u8], x: i64, y: i64) -> u8 {
        let x = x.clamp(0, self.width as i64 - 1) as usize;
        let y = y.clamp(0, self.height as i64 - 1) as usize;
        image.get(y * self.width + x).copied().unwrap_or(0)
    }

    /// The floor height at sample `(x, y)` (FUN_10016454).
    pub fn floor(&self, x: i64, y: i64) -> f32 {
        self.sample(self.heights, x, y) as f32 * self.step
    }

    /// The code at sample `(x, y)` (FUN_10016484).
    pub fn code(&self, x: i64, y: i64) -> f32 {
        let c = self.sample(self.codes, x, y) as i32;
        (if c >= 224 { c - 256 } else { c }) as f32 * self.step
    }
}

/// A laid vertex: position and normal in the tile-map's Y-up frame, and the texture coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Laid {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// FUN_100023ad: the cubic correction along x from the differences `p1..p4` and the bilinear `[dx, dz, base, dxz]`.
fn cubic_x(p: [f32; 4], bilinear: [f64; 4]) -> [f64; 6] {
    let f6 = 0.5f32 / 2.0;
    let f5 = f6 * p[0];
    let f4 = f6 * p[2];
    let mut c = [0f64; 6];
    c[2] = f5 as f64;
    let d7 = (f6 * p[1]) as f64 - f5 as f64;
    let d1 = ((f6 + f6) as f64 * bilinear[0] - f5 as f64) * 4.0 - d7;
    c[1] = d1;
    let d7 = d7 - d1;
    c[0] = d7;
    c[5] = (f4 - c[2] as f32) as f64;
    let d8 = (f6 * p[3]) as f64 - f4 as f64;
    let d2 = ((bilinear[3] + bilinear[0]) * (f6 + f6) as f64 - f4 as f64) * 4.0 - d8;
    c[3] = (d8 - d2) - d7;
    c[4] = d2 - d1;
    c
}

/// FUN_10002479: the same along z.
fn cubic_z(p: [f32; 4], bilinear: [f64; 4]) -> [f64; 6] {
    let f6 = 0.5f32 / 2.0;
    let f5 = f6 * p[0];
    let f4 = f6 * p[1];
    let mut c = [0f64; 6];
    c[2] = f5 as f64;
    let d7 = (f6 * p[2]) as f64 - f5 as f64;
    let d1 = ((f6 + f6) as f64 * bilinear[1] - f5 as f64) * 4.0 - d7;
    c[1] = d1;
    let d7 = d7 - d1;
    c[0] = d7;
    c[5] = (f4 - c[2] as f32) as f64;
    let d8 = (f6 * p[3]) as f64 - f4 as f64;
    let d2 = ((bilinear[3] + bilinear[1]) * (f6 + f6) as f64 - f4 as f64) * 4.0 - d8;
    c[3] = (d8 - d2) - d7;
    c[4] = d2 - d1;
    c
}

/// FUN_1000244e.
fn eval_u(c: &[f64; 6], u: f64, v: f64) -> f64 {
    v * c[5] + (v * c[4] + (v * c[3] + c[0]) * u + c[1]) * u + c[2]
}

/// FUN_1000251c.
fn eval_v(c: &[f64; 6], u: f64, v: f64) -> f64 {
    u * c[5] + (u * c[4] + (u * c[3] + c[0]) * v + c[1]) * v + c[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// FUN_100014e4: `v` turned by the quaternion `q` (`v + 2w (q × v) + 2 q × (q × v)`).
fn turn(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let axis = [q[0], q[1], q[2]];
    let t = cross(axis, v);
    let t2 = cross(axis, t);
    let w2 = q[3] * 2.0;
    [v[0] + t[0] * w2 + t2[0] * 2.0, v[1] + t[1] * w2 + t2[1] * 2.0, v[2] + t[2] * w2 + t2[2] * 2.0]
}

/// FUN_10001c3b for one tile of one piece: its vertices laid on cell `(x, y)` (image column and row) of `surface`, turned by
/// `turn` quarter turns, with `offset` added (the cell's centre in the room and the room's floor base, `-n3Room_t::+0x38`).
pub fn lay(surface: &Surface, x: usize, y: usize, tile: &PieceTile, turn_by: u8, offset: [f32; 3]) -> Vec<Laid> {
    let (x, y) = (x as i64, y as i64);
    let a = |r: i64, c: i64| surface.floor(x - 2 + c, y + 1 - r) + offset[1];
    let b = |r: i64, c: i64| surface.code(x - 2 + c, y + 1 - r);
    let base = a(2, 1);
    let bilinear = [(a(2, 2) - a(2, 1)) as f64, (a(1, 1) - a(2, 1)) as f64, base as f64, 0.0];
    let bilinear = [bilinear[0], bilinear[1], bilinear[2], ((a(1, 2) - base) as f64 - bilinear[0]) - bilinear[1]];
    let ax = cubic_x([a(2, 2) - a(2, 0), a(2, 3) - a(2, 1), a(1, 2) - a(1, 0), a(1, 3) - a(1, 1)], bilinear);
    let az = cubic_z([a(0, 2) - a(2, 2), a(0, 1) - a(2, 1), a(1, 2) - a(3, 2), a(1, 1) - a(3, 1)], bilinear);
    let code_base = b(2, 1);
    let codes = [(b(2, 2) - b(2, 1)) as f64, (b(1, 1) - b(2, 1)) as f64, code_base as f64, 0.0];
    let codes = [codes[0], codes[1], codes[2], ((b(1, 2) - code_base) as f64 - codes[0]) - codes[1]];
    let bx = cubic_x([b(2, 2) - b(2, 0), b(2, 3) - b(2, 1), b(1, 2) - b(1, 0), b(1, 3) - b(1, 1)], codes);
    let bz = cubic_z([b(0, 2) - b(2, 2), b(0, 1) - b(2, 1), b(1, 2) - b(3, 2), b(1, 1) - b(3, 1)], codes);
    let angle = turn_by as f32 * std::f32::consts::FRAC_PI_2 + std::f32::consts::PI;
    let (s, c) = (angle * 0.5).sin_cos();
    let q = [0.0, s, 0.0, c];
    let inv = 1.0 / surface.scale;
    tile.vertices
        .iter()
        .map(|v| {
            let mut p = turn(q, [v[0], v[1], v[2]]);
            let mut n = turn(q, [v[3], v[4], v[5]]);
            let u = (inv * p[0] + 0.5) as f64;
            let w = (p[2] * inv + 0.5) as f64;
            let py = p[1] as f64;
            n[0] -= n[1] * (eval_u(&ax, u, w) + py * 0.25 * eval_u(&bx, u, w)) as f32;
            n[2] -= n[1] * (eval_v(&az, u, w) + py * 0.25 * eval_v(&bz, u, w)) as f32;
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 0.0 {
                n = n.map(|c| c / len);
            }
            let rise = u * bilinear[0] + (u * bilinear[3] + bilinear[1]) * w + bilinear[2] + py * 0.25 * (u * codes[0] + (codes[3] * u + codes[1]) * w + codes[2]);
            p[1] += rise as f32;
            p[0] += offset[0];
            p[2] += offset[2];
            Laid { position: p, normal: n, uv: [v[6], v[7]] }
        })
        .collect()
}

/// How close two laid vertices must be to weld (FUN_10001645's `.rdata` constants): the squared distance of the positions
/// (0x1003c8a0), of the normals (0x1003c898), and how far a texture coordinate may sit from a whole number apart
/// (0x1003c890).
const WELD_POSITION: f32 = 0.001;
const WELD_NORMAL: f32 = 0.2;
const WELD_UV: f32 = 0.1;

fn distance2(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

/// Whether the earlier vertex `old` is where the new `new` is (within [`WELD_POSITION`], normals within [`WELD_NORMAL`]) with
/// texture coordinates a whole number of tiles away: that shift `(du, dv)`, which welding adds to the new tile's coordinates
/// so the neighbours share the vertex.
fn coincide(old: &Laid, new: &Laid) -> Option<[i32; 2]> {
    if distance2(old.position, new.position) > WELD_POSITION || distance2(old.normal, new.normal) > WELD_NORMAL {
        return None;
    }
    let mut shift = [0i32; 2];
    for k in 0..2 {
        let d = old.uv[k] - new.uv[k] + WELD_UV;
        let whole = d.floor();
        if d - whole > WELD_UV * 2.0 {
            return None;
        }
        shift[k] = whole.round() as i32;
    }
    Some(shift)
}

/// FUN_10001645: a tile's vertices, just laid at `begin..` of a room's vertex stream with its triangles as absolute `indices`
/// into it, welded onto the earlier vertices they coincide with — those of `above` (the three cells of the row before, from
/// the one before this column to the one after) and those laid since the cell before began (`left..begin`). First every new
/// vertex's matches are collected with their texture shift (up to twenty): the shift two different vertices agree on, or
/// else the last one found, is the tile's; without a match the tile stays as it is. Then each new vertex that an earlier one
/// matches under that shift is dropped (the triangles pointed at the earlier one, later indices moved down) and the rest get
/// the shift added to their texture coordinates. The stream is shortened by the welded vertices.
pub fn weld(vertices: &mut Vec<Laid>, indices: &mut [u32], begin: usize, above: std::ops::Range<usize>, left: usize) {
    let candidates = || above.clone().chain(left..begin);
    let mut shifts: Vec<(usize, [i32; 2])> = Vec::new();
    let mut chosen = None;
    'scan: for n in begin..vertices.len() {
        for c in candidates() {
            let Some(shift) = coincide(&vertices[c], &vertices[n]) else { continue };
            match shifts.iter().position(|&(_, s)| s == shift) {
                Some(k) => {
                    if shifts[k].0 != n {
                        chosen = Some(shift);
                        break 'scan;
                    }
                }
                None if shifts.len() == 20 => shifts[19] = (n, shift),
                None => shifts.push((n, shift)),
            }
        }
    }
    let Some(shift) = chosen.or_else(|| shifts.last().map(|e| e.1)) else { return };
    let mut n = begin;
    while n < vertices.len() {
        let new = vertices[n];
        let found = candidates().find(|&c| {
            let old = &vertices[c];
            distance2(old.position, new.position) <= WELD_POSITION
                && distance2(old.normal, new.normal) <= WELD_NORMAL
                && (shift[0] as f32 - (old.uv[0] - new.uv[0])).abs() <= WELD_UV
                && (shift[1] as f32 - (old.uv[1] - new.uv[1])).abs() <= WELD_UV
        });
        match found {
            Some(c) => {
                vertices.remove(n);
                for i in indices.iter_mut() {
                    if *i as usize == n {
                        *i = c as u32;
                    } else if *i as usize > n {
                        *i -= 1;
                    }
                }
            }
            None => {
                vertices[n].uv[0] += shift[0] as f32;
                vertices[n].uv[1] += shift[1] as f32;
                n += 1;
            }
        }
    }
}

/// FUN_100082a7: the colours of a room's `count` welded vertices from its depacked light map — a 565 colour each, in the
/// stream's order, widened to RGBA with full alpha — or white for them all when the map is shorter than that.
pub fn light_colours(light: &[u8], count: usize) -> Vec<[u8; 4]> {
    if light.len() < count * 2 {
        return vec![[255; 4]; count];
    }
    (0..count)
        .map(|i| {
            let c = u16::from_le_bytes([light[2 * i], light[2 * i + 1]]);
            [((c >> 11) as u8) << 3, ((c >> 5) as u8 & 0x3f) << 2, (c as u8 & 0x1f) << 3, 255]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wall_patterns_turn_to_their_smallest_form() {
        let (slot, turn) = wall_tables();
        // no walls: the sixth pattern, no turn
        assert_eq!((slot[0], turn[0]), (6, 0));
        // one wall on each side: pattern 0x01 (the fifth), turned to bring the wall round
        assert_eq!((slot[1], turn[1]), (5, 0));
        assert_eq!((slot[2], turn[2]), (5, 1));
        assert_eq!((slot[4], turn[4]), (5, 2));
        assert_eq!((slot[8], turn[8]), (5, 3));
        // a corner is 0x03; the two others 0x05; three walls 0x07; a door on one wall 0x11
        assert_eq!(slot[3], 4);
        assert_eq!(slot[5], 3);
        assert_eq!(slot[7], 2);
        assert_eq!(slot[0x11], 12);
        assert_eq!(slot[0x1f], 14);
        // a door on a wall turned round is the same pattern; a code outside the list has no slot
        assert_eq!((slot[0x12], turn[0x12]), (12, 1));
        assert_eq!(slot[0x60], 0);
    }

    fn shop_tables() -> Tables {
        // the shop tile-map 0x474's types 0 and 85, as its stream carries them (the stored slots)
        let slots = |v: [u16; 16]| {
            let mut s = [0u16; 16];
            for (k, slot) in [0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15].into_iter().enumerate() {
                s[slot] = v[k];
            }
            s
        };
        let row = slots([100, 0, 0, 83, 84, 278, 279, 280, 87, 87, 86, 87, 87, 87, 87, 87]);
        let tables = vec![TypeTable { tile: 0, neighbours: vec![(85, row)], heights: Vec::new() }, TypeTable { tile: 85, neighbours: vec![(0, row)], heights: Vec::new() }];
        Tables::new(&[0, 85], &tables, HashMap::new())
    }

    #[test]
    fn a_wall_beside_nothing_takes_the_one_wall_piece() {
        let t = shop_tables();
        assert_eq!(t.piece(1, 0, 1), (84, 0));
        assert_eq!(t.piece(1, 0, 2), (84, 1));
        assert_eq!(t.piece(1, 0, 3), (83, 0));
        assert_eq!(t.piece(1, 0, 0xf), (100, 0));
        assert_eq!(t.piece(1, 0, 0x1f), (87, 0));
        assert_eq!(t.piece(1, 0, 0), (0, 0));
    }

    #[test]
    fn a_lone_floor_cell_is_walled_on_the_sides_the_constructor_sees() {
        // a 5 x 5 map with one floor cell in the middle
        let mut places = vec![0u8; 25];
        places[2 * 5 + 2] = 1;
        let images = CellImages { width: 5, height: 5, places: &places, doors: &[], choices: None };
        let cells = super::cells(&images, &shop_tables());
        let (piece, _) = cells.piece(2, 2);
        // all four sides face empty cells: the four-wall piece
        assert_eq!(piece, 100);
        // the empty cells around it get nothing
        assert_eq!(cells.piece(1, 2).0, 0);
        assert_eq!(cells.piece(2, 1).0, 0);
    }

    #[test]
    fn a_block_of_floor_has_corners_edges_and_an_open_middle() {
        // a 3 x 3 block of floor in a 7 x 7 map
        let mut places = vec![0u8; 49];
        for y in 2..5 {
            for x in 2..5 {
                places[y * 7 + x] = 1;
            }
        }
        let images = CellImages { width: 7, height: 7, places: &places, doors: &[], choices: None };
        let cells = super::cells(&images, &shop_tables());
        // the middle has no walls: no piece; the edges one wall; the corners two (the corner piece), each turned its own way
        assert_eq!(cells.piece(3, 3).0, 0);
        assert_eq!(cells.piece(3, 2).0, 84);
        assert_eq!(cells.piece(2, 3).0, 84);
        assert_eq!(cells.piece(4, 3).0, 84);
        assert_eq!(cells.piece(3, 4).0, 84);
        let corners: Vec<(u16, u8)> = [(2, 2), (4, 2), (4, 4), (2, 4)].iter().map(|&(x, y)| cells.piece(x, y)).collect();
        assert!(corners.iter().all(|c| c.0 == 83), "{corners:?}");
        let mut turns: Vec<u8> = corners.iter().map(|c| c.1).collect();
        turns.sort();
        assert_eq!(turns, vec![0, 1, 2, 3]);
        // the edges' turns differ too
        let edges: Vec<u8> = [(3, 2), (4, 3), (3, 4), (2, 3)].iter().map(|&(x, y)| cells.piece(x, y).1).collect();
        let mut sorted = edges.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3], "{edges:?}");
        // a one-wide corridor (two facing walls) and a dead end (three) have no piece in this table
        let mut places = vec![0u8; 49];
        for x in 1..6 {
            places[3 * 7 + x] = 1;
        }
        let images = CellImages { width: 7, height: 7, places: &places, doors: &[], choices: None };
        let cells = super::cells(&images, &shop_tables());
        assert_eq!((cells.piece(1, 3).0, cells.piece(3, 3).0), (0, 0));
    }

    #[test]
    fn a_flat_floor_leaves_a_piece_where_it_is() {
        let heights = vec![10u8; 16];
        let codes = vec![0u8; 16];
        let surface = Surface { width: 4, height: 4, heights: &heights, codes: &codes, scale: 2.0, step: 0.2 };
        let tile = PieceTile { vertices: vec![[0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.5, 0.5]], indices: vec![] };
        let laid = lay(&surface, 2, 2, &tile, 0, [10.0, 1.0, 20.0]);
        assert_eq!(laid.len(), 1);
        let p = laid[0].position;
        assert!((p[0] - 10.0).abs() < 1e-4 && (p[2] - 20.0).abs() < 1e-4, "{p:?}");
        // the floor is 10 steps up plus the base, the vertex a unit above it
        assert!((p[1] - (1.0 + 2.0 + 1.0)).abs() < 1e-4, "{p:?}");
        assert!((laid[0].normal[1] - 1.0).abs() < 1e-5);
        // half a turn (the constant 180 degrees) sends +x to -x
        let tile = PieceTile { vertices: vec![[1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]], indices: vec![] };
        let laid = lay(&surface, 2, 2, &tile, 0, [0.0; 3]);
        assert!((laid[0].position[0] + 1.0).abs() < 1e-4, "{:?}", laid[0]);
        let laid = lay(&surface, 2, 2, &tile, 1, [0.0; 3]);
        assert!(laid[0].position[0].abs() < 1e-4 && laid[0].position[2].abs() > 0.99, "{:?}", laid[0]);
    }
}
