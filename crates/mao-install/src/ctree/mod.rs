//! The c-tree Plus files the resource database lives in, read and written: a data file of variable-length records
//! (`.dat`, continued in segment files `.dat.001`, `.dat.002`, ...) and an index file (`.idx`) whose B-tree maps an
//! 8-byte key to a record's position.
//!
//! This is `AnarchyReimagined`'s `ao-ctree` crate (written from the file format as the original client's
//! `DatabaseController.dll` uses it, and tested there against the original), brought over for the one thing the
//! client's own reader in `mao-formats` does not do: writing. The patcher needs it to put a patch's records into
//! the database the way the original patcher does, so that the original client reads the result too. The parts
//! that create an empty database and rebuild an index stayed behind.
//!
//! The rules are the engine's as the game opens the files (not in huge-file mode): positions are 32 bits wide, a
//! segment is picked by comparing those 32 bits with each segment's end, and no record can start past the last byte
//! the header says is in use.

pub mod file;
pub mod index;

use std::path::Path;

use file::CtFile;
use index::Index;

/// A c-tree error number (the engine's own numbering, which the game's
/// code tests for a few of).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub u16);

impl Error {
    /// A file or segment that can't be opened.
    pub const OPEN: Error = Error(12);
    /// A file that was being changed and never closed.
    pub const NOT_CLOSED: Error = Error(14);
    /// A segment file that is missing or doesn't belong to the file (not
    /// one of the engine's codes: it has several for the ways a segment
    /// can be wrong, and creates missing ones).
    pub const SEGMENT: Error = Error(0x7ffe);
    /// A record position past the last byte in use.
    pub const PAST_END: Error = Error(30);
    /// A record shorter than the fixed part.
    pub const SHORT_RECORD: Error = Error(149);
    /// No record header where a record's length is asked for.
    pub const NOT_RECORD: Error = Error(158);
    /// A write that failed.
    pub const WRITE: Error = Error(37);
    /// A change to a file opened read-only (what the engine's DELREC says).
    pub const READ_ONLY: Error = Error(54);
    /// A file that can't grow any more (past 4 GB).
    pub const FULL: Error = Error(0x7ffc);
    /// A key that is already there.
    pub const DUPLICATE: Error = Error(2);
    /// A read that came back short (past the end of a file).
    pub const READ: Error = Error(36);
    /// No key equal to the one looked for.
    pub const NOT_FOUND: Error = Error(101);
    /// A record position without a variable-length record header.
    pub const RECORD_HEADER: Error = Error(114);
    /// A record position of zero.
    pub const ZERO_POSITION: Error = Error(159);
    /// The record at the index's position holds another key.
    pub const KEY_MISMATCH: Error = Error(160);
    /// An index node that can't be right (not one of the engine's codes:
    /// the engine's own checks of a damaged tree aren't reproduced).
    pub const BAD_NODE: Error = Error(0x7fff);
}

pub type Result<T> = std::result::Result<T, Error>;

/// The marker that starts every variable-length record.
pub const RECORD_MARKER: u16 = 0xfafa;
/// The marker that starts a resource record (the files' own metadata).
pub const RESOURCE_MARKER: u16 = 0xfefe;

/// A record's position in the data file (the index keeps a high word too,
/// which the game's way of opening the files ignores).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Position {
    pub low: u32,
    pub high: u32,
}

/// The game's resource keys: a type and an instance, compared as unsigned
/// numbers in that order (the key segments are c-tree INTSEGs, flipped
/// big-endian from the little-endian record header).
pub fn key(kind: u32, instance: u32) -> [u8; 8] {
    let mut k = [0; 8];
    k[..4].copy_from_slice(&kind.to_be_bytes());
    k[4..].copy_from_slice(&instance.to_be_bytes());
    k
}

/// A data file and its index, opened read-only.
pub struct Database {
    data: CtFile,
    index: Index,
    /// The modes the headers get at the first change.
    modes: (u16, u16),
}

/// The fixed part of every record: its type and instance (what the key is
/// built from), then the length of the data after them.
pub const RECORD_FIXED: usize = 8;

impl Database {
    /// Opens `<base>.dat` and `<base>.idx` (and the data file's segments,
    /// as reads reach them).
    pub fn open(base: &Path) -> Result<Database> {
        let with = |ext: &str| {
            let mut s = base.as_os_str().to_owned();
            s.push(ext);
            std::path::PathBuf::from(s)
        };
        Ok(Database { data: CtFile::open(&with(".dat"))?, index: Index::new(CtFile::open(&with(".idx"))?), modes: CHANGE_MODES })
    }

    pub fn data(&self) -> &CtFile {
        &self.data
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    /// The position of the record with `key`, checked as the engine's
    /// EQLREC checks it: the record starts no later than the last byte in
    /// use, has a record header, and holds `key` itself.
    pub fn find(&self, key: &[u8; 8]) -> Result<Position> {
        let position = self.index.find(key)?.ok_or(Error::NOT_FOUND)?;
        if position.low == 0 && position.high == 0 {
            return Err(Error::ZERO_POSITION);
        }
        if position.low > self.data.header().in_use {
            return Err(Error::PAST_END);
        }
        let mut fixed = [0; RECORD_FIXED];
        self.data.read(position.low.wrapping_add(file::RECORD_HEADER as u32), &mut fixed)?;
        let (marker, used) = self.record_header(position)?;
        if marker != RECORD_MARKER || used == 0 {
            return Err(Error::RECORD_HEADER);
        }
        let mut own = [0; 8];
        own[..4].copy_from_slice(&u32::from_le_bytes(fixed[..4].try_into().unwrap()).to_be_bytes());
        own[4..].copy_from_slice(&u32::from_le_bytes(fixed[4..].try_into().unwrap()).to_be_bytes());
        if own != *key {
            return Err(Error::KEY_MISMATCH);
        }
        Ok(position)
    }

    /// The marker and used length in the record header at `position`.
    pub fn record_header(&self, position: Position) -> Result<(u16, u32)> {
        let mut header = [0; file::RECORD_HEADER];
        self.data.read(position.low, &mut header)?;
        Ok((u16::from_le_bytes([header[0], header[1]]), u32::from_le_bytes(header[6..10].try_into().unwrap())))
    }

    /// The used length of the record at `position` (the engine's GETVLEN):
    /// there must be a record header, and the length can't be shorter than
    /// the fixed part.
    pub fn record_length(&self, position: Position) -> Result<u32> {
        if position.low == 0 && position.high == 0 {
            return Err(Error::ZERO_POSITION);
        }
        let (marker, used) = self.record_header(position)?;
        if marker != RECORD_MARKER {
            return Err(Error::NOT_RECORD);
        }
        if (used as usize) < RECORD_FIXED {
            return Err(Error::SHORT_RECORD);
        }
        Ok(used)
    }

    /// Reads the record at `position` (header excluded) into `buf`, which
    /// is the length its header gives (the engine's REDVREC). On failure
    /// the bytes before the failed read are filled in.
    pub fn read_record(&self, position: Position, buf: &mut [u8]) -> Result<()> {
        self.data.read(position.low.wrapping_add(file::RECORD_HEADER as u32), buf)
    }

    /// The whole record (header excluded) with `key`.
    pub fn get(&self, key: &[u8; 8]) -> Result<Vec<u8>> {
        let position = self.find(key)?;
        let used = self.record_length(position)?;
        let mut record = vec![0; used as usize];
        self.read_record(position, &mut record)?;
        Ok(record)
    }
}

/// The file modes written into the headers at the first change (the
/// engine's for the files opened read-write: data file, index).
pub const CHANGE_MODES: (u16, u16) = (0xc7, 0x43);

/// What a deleted record's header starts with.
pub const DELETED_MARKER: u16 = 0xfdfd;

fn with_extension(base: &Path, ext: &str) -> std::path::PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push(ext);
    std::path::PathBuf::from(s)
}

impl Database {
    /// Opens `<base>.dat` and `<base>.idx` for reading and writing.
    pub fn open_writable(base: &Path) -> Result<Database> {
        Ok(Database {
            data: CtFile::open_with(&with_extension(base, ".dat"), true)?,
            index: Index::new(CtFile::open_with(&with_extension(base, ".idx"), true)?),
            modes: CHANGE_MODES,
        })
    }

    /// The modes the headers get at the first change (the engine's open
    /// modes: `CHANGE_MODES`, or 6 and 2 for files opened to keep their
    /// mode).
    pub fn set_change_modes(&mut self, data: u16, index: u16) {
        self.modes = (data, index);
    }

    fn begin_change(&mut self) -> Result<()> {
        self.data.begin_change(self.modes.0)?;
        self.index.file_mut().begin_change(self.modes.1)
    }

    /// Adds a record for `key` (which mustn't be there) at the end of the
    /// data file. Deleted records' space isn't used again.
    pub fn add(&mut self, key: &[u8; 8], record: &[u8]) -> Result<Position> {
        self.begin_change()?;
        if self.index.find(key)?.is_some() {
            return Err(Error::DUPLICATE);
        }
        let position = self.append(record)?;
        let records = self.data.header().records.wrapping_add(1);
        self.data.set_word(0x14, records);
        self.index.insert(key, position)?;
        Ok(position)
    }

    /// Writes a record at the end of the data file.
    fn append(&mut self, record: &[u8]) -> Result<Position> {
        let total = (file::RECORD_HEADER + record.len()) as u32;
        let at = self.data.header().in_use.checked_add(1).ok_or(Error::FULL)?;
        let last = at.checked_add(total - 1).ok_or(Error::FULL)?;
        self.data.grow(last)?;
        let mut bytes = Vec::with_capacity(total as usize);
        bytes.extend_from_slice(&RECORD_MARKER.to_le_bytes());
        bytes.extend_from_slice(&total.to_le_bytes());
        bytes.extend_from_slice(&(record.len() as u32).to_le_bytes());
        bytes.extend_from_slice(record);
        self.data.write(at, &bytes)?;
        self.data.set_word(0x08, last);
        self.data.count_change();
        Ok(Position { low: at, high: 0 })
    }

    /// Writes `record` for `key` over the one at `position`: in its place
    /// when it fits in the space that one has, else at the end of the data
    /// file (the old one deleted, the key pointed at the new one).
    pub fn rewrite(&mut self, key: &[u8; 8], position: Position, record: &[u8]) -> Result<Position> {
        self.begin_change()?;
        let mut header = [0; file::RECORD_HEADER];
        self.data.read(position.low, &mut header)?;
        let total = u32::from_le_bytes(header[2..6].try_into().unwrap()) as usize;
        if u16::from_le_bytes([header[0], header[1]]) != RECORD_MARKER {
            return Err(Error::RECORD_HEADER);
        }
        if file::RECORD_HEADER + record.len() <= total {
            self.data.write(position.low.wrapping_add(6), &(record.len() as u32).to_le_bytes())?;
            self.data.write(position.low.wrapping_add(file::RECORD_HEADER as u32), record)?;
            self.data.count_change();
            return Ok(position);
        }
        self.mark_deleted(position)?;
        let new = self.append(record)?;
        self.index.set_position(key, new)?;
        Ok(new)
    }

    fn mark_deleted(&mut self, position: Position) -> Result<()> {
        self.data.write(position.low, &DELETED_MARKER.to_le_bytes())?;
        self.data.write(position.low.wrapping_add(6), &0u32.to_le_bytes())?;
        self.data.count_change();
        Ok(())
    }

    /// Deletes the record at `position` and takes `key` out of the index.
    pub fn delete(&mut self, key: &[u8; 8], position: Position) -> Result<()> {
        self.begin_change()?;
        let (marker, _) = self.record_header(position)?;
        if marker != RECORD_MARKER {
            return Err(Error::RECORD_HEADER);
        }
        self.mark_deleted(position)?;
        let records = self.data.header().records.wrapping_sub(1);
        self.data.set_word(0x14, records);
        self.index.remove(key)?;
        Ok(())
    }

    /// Writes the headers back and flushes, the files still open.
    pub fn sync(&mut self) -> Result<()> {
        self.data.sync()?;
        self.index.file_mut().sync()
    }

    /// Ends the changes (the files can be opened again after this).
    pub fn close(&mut self) -> Result<()> {
        self.data.close()?;
        self.index.file_mut().close()
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
