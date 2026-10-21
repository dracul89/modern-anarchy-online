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
    /// The chunks after the types, in order (`DCGA`, `DHGA`, `HCDA`, `IRHA`,
    /// optionally `TAHA`, `XTHA`, `THSA`).
    pub chunks: Vec<Chunk>,
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
    for _ in 0..count {
        let head = take(&mut at, 2, "a tile type's counts")?;
        let (neighbours, heights) = (head[0] as usize, head[1] as usize);
        take(&mut at, neighbours * (2 + 16 * 2), "a tile type's neighbours")?;
        take(&mut at, heights * 2, "a tile type's heights")?;
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
    Ok(NewTilemap { tag: *b"GNDA", width, height, scale, second, tile_types, chunks })
}
