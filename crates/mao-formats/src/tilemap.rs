//! Tile-maps: the resource database's `RDBTilemap_t` objects (type 0xf4249),
//! one per playfield (the instance is the playfield's number).
//!
//! Derived from `RDBTilemap_t::ReadBlob` (`AnarchyReimagined`:
//! `dropin/displaysystem/src/rdbtilemap.rs`). After the 12-byte object head
//! the data is chunks, each `[tag u32][size u32][version u32]` and a body,
//! padded to four bytes (tags are ASCII, so little-endian they read
//! backwards: `CHGA` is "AGHC"). Two layouts exist: `GNDA`/`UOHA` (with
//! images and heights) and the older `CHGA`, which this module reads:
//!
//! `CHGA` (version 1): two shorts (the map's size in tiles), a float (the
//! tile size), a float, the number of tile types and the type numbers
//! (shorts), then an object [`archive`](crate::archive) holding the tile
//! object ("obj") — whose own size isn't stated, so it is read to its end —
//! and then a `CITA` chunk of names.

use std::fmt;

use crate::archive::{Archive, Error as ArchiveError};
use crate::texture::OBJECT_HEAD;

#[derive(Debug)]
pub enum Error {
    Truncated(&'static str),
    /// The first chunk isn't one this module reads.
    Chunk(u32),
    Archive(ArchiveError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated(what) => write!(f, "tile-map truncated in {what}"),
            Error::Chunk(tag) => write!(f, "tile-map chunk {:?} isn't read here", tag.to_le_bytes().map(|b| b as char)),
            Error::Archive(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ArchiveError> for Error {
    fn from(e: ArchiveError) -> Error {
        Error::Archive(e)
    }
}

/// The tag of an old-style ground chunk, "CHGA".
pub const OLD: u32 = u32::from_le_bytes(*b"CHGA");

/// An old-style tile-map.
#[derive(Debug)]
pub struct OldTilemap {
    pub width: u16,
    pub height: u16,
    /// The tile's size in world units.
    pub tile_size: f32,
    pub second: f32,
    pub tile_types: Vec<u16>,
    pub archive: Archive,
    /// The bytes after the archive (the `CITA` chunk of names).
    pub rest: Vec<u8>,
}

pub fn parse_old(object: &[u8]) -> Result<OldTilemap, Error> {
    let mut at = OBJECT_HEAD;
    let take = |at: &mut usize, n: usize, what| -> Result<&[u8], Error> {
        let b = object.get(*at..*at + n).ok_or(Error::Truncated(what))?;
        *at += n;
        Ok(b)
    };
    let word = |b: &[u8]| u32::from_le_bytes(b[..4].try_into().unwrap());
    let tag = word(take(&mut at, 4, "the tag")?);
    if tag != OLD {
        return Err(Error::Chunk(tag));
    }
    take(&mut at, 8, "the chunk head")?;
    let width = u16::from_le_bytes(take(&mut at, 2, "the width")?.try_into().unwrap());
    let height = u16::from_le_bytes(take(&mut at, 2, "the height")?.try_into().unwrap());
    let tile_size = f32::from_bits(word(take(&mut at, 4, "the tile size")?));
    let second = f32::from_bits(word(take(&mut at, 4, "a float")?));
    let count = u16::from_le_bytes(take(&mut at, 2, "the type count")?.try_into().unwrap()) as usize;
    let mut tile_types = Vec::new();
    for _ in 0..count {
        tile_types.push(u16::from_le_bytes(take(&mut at, 2, "a tile type")?.try_into().unwrap()));
    }
    // The archive's own length isn't given: it ends where its last stream does.
    let (archive, used) = Archive::parse_prefix(&object[at..])?;
    Ok(OldTilemap { width, height, tile_size, second, tile_types, archive, rest: object[at + used..].to_vec() })
}

/// A chunk after the head: its tag and its body (the size's 12-byte head
/// excluded).
#[derive(Debug, Clone)]
pub struct Chunk {
    pub tag: [u8; 4],
    pub version: u32,
    pub body: Vec<u8>,
}

/// A newer-style tile-map (`GNDA` or `UOHA`): a header, the tile types'
/// tables and then image and data chunks.
#[derive(Debug)]
pub struct NewTilemap {
    /// The first chunk's tag.
    pub tag: [u8; 4],
    pub width: u16,
    pub height: u16,
    pub scale: f32,
    pub second: f32,
    pub tile_types: Vec<u16>,
    /// Each tile type's tables, in the order of `tile_types`.
    pub type_tables: Vec<TypeTable>,
    /// The chunks after the types, in order (`DCGA`, `DHGA`, `HCDA`, `IRHA`,
    /// optionally `TAHA`, `XTHA`, `THSA`).
    pub chunks: Vec<Chunk>,
}

/// A tile type's tables (`RDBTilemap_t::ReadBlob`'s FUN_10077aaa): for each neighbouring type, the pieces (type 0xf695d) that
/// furnish a cell of this type beside one of that type, by wall pattern, and a list of heights (the type's alternative floor
/// pieces, picked at random where the pattern allows any turn).
///
/// The stream carries 16 words per neighbour; the original stores the first five in slots 0-4, the next ten in slots 6-15 (slot 5,
/// the pattern of no walls, stays 0, so a cell without walls gets no piece) and drops the last. `slots` is that stored table.
#[derive(Debug, Clone)]
pub struct TypeTable {
    pub tile: u16,
    /// `(neighbouring type, the 16 slots)`.
    pub neighbours: Vec<(u16, [u16; 16])>,
    pub heights: Vec<u16>,
}

pub fn parse_new(object: &[u8]) -> Result<NewTilemap, Error> {
    let mut at = OBJECT_HEAD;
    let take = |at: &mut usize, n: usize, what| -> Result<&[u8], Error> {
        let b = object.get(*at..*at + n).ok_or(Error::Truncated(what))?;
        *at += n;
        Ok(b)
    };
    let word = |b: &[u8]| u32::from_le_bytes(b[..4].try_into().unwrap());
    let short = |b: &[u8]| u16::from_le_bytes(b[..2].try_into().unwrap());
    let tag: [u8; 4] = take(&mut at, 4, "the tag")?.try_into().unwrap();
    if &tag != b"GNDA" && &tag != b"UOHA" {
        return Err(Error::Chunk(u32::from_le_bytes(tag)));
    }
    let version = word(take(&mut at, 8, "the chunk head")?[4..].as_ref());
    if version != 1 {
        return Err(Error::Chunk(version));
    }
    let width = short(take(&mut at, 2, "the width")?);
    let height = short(take(&mut at, 2, "the height")?);
    let scale = f32::from_bits(word(take(&mut at, 4, "the scale")?));
    let second = f32::from_bits(word(take(&mut at, 4, "a float")?));
    let count = short(take(&mut at, 2, "the type count")?) as usize;
    if count >= 0x4000 {
        return Err(Error::Truncated("too many tile types"));
    }
    let mut tile_types = Vec::new();
    for _ in 0..count {
        tile_types.push(short(take(&mut at, 2, "a tile type")?));
    }
    // Each type: how many neighbours it lists and how many heights it has,
    // each neighbour a type number and 16 shorts, then the heights.
    let mut type_tables = Vec::new();
    for &tile in &tile_types {
        let head = take(&mut at, 2, "a tile type's counts")?;
        let (neighbours, heights) = (head[0] as usize, head[1] as usize);
        let mut list = Vec::new();
        for _ in 0..neighbours {
            let other = short(take(&mut at, 2, "a neighbouring type")?);
            let words = take(&mut at, 32, "a tile type's neighbours")?;
            let mut slots = [0u16; 16];
            for (k, slot) in [0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15].into_iter().enumerate() {
                slots[slot] = short(&words[2 * k..]);
            }
            list.push((other, slots));
        }
        let mut height_list = Vec::new();
        for _ in 0..heights {
            height_list.push(short(take(&mut at, 2, "a tile type's heights")?));
        }
        type_tables.push(TypeTable { tile, neighbours: list, heights: height_list });
    }
    let mut chunks = Vec::new();
    while at + 12 <= object.len() {
        let tag: [u8; 4] = object[at..at + 4].try_into().unwrap();
        let size = word(&object[at + 4..]) as usize;
        let version = word(&object[at + 8..]);
        if size < 12 || at + size > object.len() {
            break;
        }
        chunks.push(Chunk { tag, version, body: object[at + 12..at + size].to_vec() });
        at += size + ((4 - (size & 3)) & 3);
    }
    Ok(NewTilemap { tag: *b"GNDA", width, height, scale, second, tile_types, type_tables, chunks })
}

/// The tiles' materials: N3.dll's `RDBTileMaterial_t` (type 0xf4258, instance 1, which `n3TilemapController_t`'s constructor
/// loads into its map by tile ID for `GetTileMaterialType`, `GetTileLevelOfPassability` and the bonuses).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileMaterial {
    pub passability: u8,
    pub attack: u8,
    pub defense: u8,
    /// The sound material (`MapGameMaterialToSoundMaterial`'s input): 1 metal, 2 sand, 3 dirt, 4 swamp, 5 concrete, 6 dry
    /// grass and leaves, 0xa lava, 0xe rock, 0xf grass, 0x10 cyber, 0x11 snow and ice, 0 none.
    pub material: u32,
}

/// The object type of the tile materials.
pub const TILE_MATERIALS: u32 = 0xf4258;

/// `RDBTileMaterial_t::ReadBlob`: after the object head a count and a version (shorts); version 1 has per tile its ID (short),
/// material, passability, attack and defence (bytes); version 2 its ID, the material as a word, then the three bytes, a material
/// of 0x13 or more being a name's hash (FUN_1003823c, the sound IDs' hash) that the reader maps to a number. Any other version
/// is not read (0x1001c582), so the table is left out and `GetTileDescriptor` gives zeros for every entry: the controller then
/// holds only tile 0, of material 0. The game's own object is version 3 (1690 tiles), so in the original every ground tile's
/// material is 0.
pub fn tile_materials(object: &[u8]) -> Option<std::collections::HashMap<u16, TileMaterial>> {
    let short = |at: usize| Some(u16::from_le_bytes(object.get(at..at + 2)?.try_into().ok()?));
    let (count, version) = (short(OBJECT_HEAD)?, short(OBJECT_HEAD + 2)?);
    let mut at = OBJECT_HEAD + 4;
    let mut map = std::collections::HashMap::new();
    for _ in 0..count {
        let id = short(at)?;
        let (material, bytes) = match version {
            1 => (*object.get(at + 2)? as u32, object.get(at + 3..at + 6)?),
            2 => (u32::from_le_bytes(object.get(at + 2..at + 6)?.try_into().ok()?), object.get(at + 6..at + 9)?),
            _ => {
                map.insert(0, TileMaterial::default());
                return Some(map);
            }
        };
        at += if version == 1 { 6 } else { 9 };
        let material = if version == 2 && material >= 0x13 { material_of_name(material) } else { material };
        map.insert(id, TileMaterial { passability: bytes[0], attack: bytes[1], defense: bytes[2], material });
    }
    Some(map)
}

/// The material numbers of version 2's names (the chain in `RDBTileMaterial_t::ReadBlob`), 0 for a name not listed.
fn material_of_name(hash: u32) -> u32 {
    const NAMES: &[(&str, u32)] = &[
        ("athens", 0xe), ("avarock", 0xe), ("brightconcreteblock", 5), ("brightconcretepattern", 5), ("brock", 0xe), ("concreteslabs", 5),
        ("crackedground", 3), ("cyber", 0x10), ("darkconcreteblock", 5), ("darkconcretepattern", 5), ("darkgrass", 0xf), ("darkrock", 0xe),
        ("dense", 3), ("desert", 2), ("drygrass", 6), ("fungus", 3), ("grass", 0xf), ("greenash", 2), ("growel", 3), ("hqground", 5),
        ("jobe1", 1), ("jobe2", 1), ("metalgrid", 1), ("miruinedconcretepattern", 5), ("mudsand", 3), ("phos", 3), ("rock", 0xe),
        ("ruinedconcretepattern", 5), ("salt", 2), ("sand", 2), ("shadornament4", 5), ("shadornament5", 5), ("shadow_darkrock", 0xe),
        ("shadow_myrkrock", 0xe), ("shadow_snow", 0x11), ("shadow_underwater", 3), ("shadow1", 0xe), ("shadow1_ash", 2), ("shadow1_dark", 0xe),
        ("shadow1_lava", 10), ("shadow1_lavadry", 0xe), ("shadowbrink", 0), ("shadowgrass", 0xf), ("shadowgrassgreen", 0xf),
        ("shadowgrasslight", 0xf), ("shadowice", 0x11), ("shadowleafs", 6), ("shadowrock", 0xe), ("shadowsand", 2), ("shadowskulls", 5),
        ("shadowtile1", 0xe), ("shadowtile2", 0xe), ("shadowtile3", 0xe), ("shadowtile4", 0xe), ("shadowtrampeledsnow", 0x11),
        ("shallows", 4), ("shinymet", 1), ("smallblockb", 0xe), ("swamp", 4), ("warsand", 2),
    ];
    NAMES.iter().find(|(name, _)| crate::sound::sound_id(name) == hash).map_or(0, |&(_, m)| m)
}
