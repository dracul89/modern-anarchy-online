//! Mesh objects: the resource database's `RDBMesh_t` (types 0xf6951 and
//! 0xf696a).
//!
//! Derived from `RDBMesh_t::ReadBlob` (`AnarchyReimagined`:
//! `dropin/displaysystem/src/rdb.rs`). After the 12-byte object head
//! (type, instance, revision): a version (1 to 3), what that version adds
//! (version 1 a count and records to skip; version 3 the animation notes,
//! each a time and a 0x20-byte name), a packed colour, and then the rest of
//! the object: an object [`archive`](crate::archive) of the mesh's frames.

use std::fmt;

use crate::archive::{Archive, Error as ArchiveError};

#[derive(Debug)]
pub enum Error {
    Truncated,
    Version(i32),
    Archive(ArchiveError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(f, "the mesh object is too short"),
            Error::Version(v) => write!(f, "mesh version {v} (1 to 3 are known)"),
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

/// A note on the mesh's animation: a time and a name.
#[derive(Clone, Debug)]
pub struct Note {
    pub time: f32,
    pub name: String,
}

/// A mesh object's parts.
#[derive(Debug)]
pub struct MeshObject {
    pub version: i32,
    pub notes: Vec<Note>,
    /// The packed colour word.
    pub colour: u32,
    pub archive: Archive,
}

pub fn parse(object: &[u8]) -> Result<MeshObject, Error> {
    let mut at = crate::texture::OBJECT_HEAD;
    let int = |at: &mut usize| -> Result<i32, Error> {
        let b = object.get(*at..*at + 4).ok_or(Error::Truncated)?;
        *at += 4;
        Ok(i32::from_le_bytes(b.try_into().unwrap()))
    };
    let version = int(&mut at)?;
    if !(1..=3).contains(&version) {
        return Err(Error::Version(version));
    }
    let mut notes = Vec::new();
    if version == 1 {
        let count = int(&mut at)? as u32;
        // `count / 0x3f1 - 1` records of 0x2c bytes are skipped.
        at += (count / 0x3f1).wrapping_sub(1) as usize * 0x2c;
    } else if version > 2 {
        let count = int(&mut at)? as u32;
        for _ in 0..count {
            let time = int(&mut at)? as f32;
            let name = object.get(at..at + 0x20).ok_or(Error::Truncated)?;
            at += 0x20;
            let end = name.iter().position(|&b| b == 0).unwrap_or(0x20);
            notes.push(Note { time, name: String::from_utf8_lossy(&name[..end]).into_owned() });
        }
    }
    let colour = int(&mut at)? as u32;
    let archive = Archive::parse(object.get(at..).ok_or(Error::Truncated)?)?;
    Ok(MeshObject { version, notes, colour, archive })
}
