//! A read-only reader for the c-tree Plus files the resource database lives
//! in: a data file of variable-length records (`.dat`, continued in segment
//! files `.dat.001`, `.dat.002`, …) and an index file (`.idx`) whose B-tree
//! maps an 8-byte key to a record's position.
//!
//! Derived from the format as the original client's `DatabaseController.dll`
//! uses it (`AnarchyReimagined`: `crates/ao-ctree`). One deliberate
//! difference: the original opens the files without c-tree's huge-file mode,
//! so positions past 4 GB wrap around and it cannot reach them. Here a
//! position is the full 64 bits (the index entry's low and high words).
//!
//! # The data file
//! A 0x200-byte header (little-endian words):
//!
//! | offset | field |
//! |---|---|
//! | 0x08 | the last byte in use: no record starts after it |
//! | 0x0c | the node size (an index) |
//! | 0x18 | the number of keys (an index) |
//! | 0x1c | the B-tree's root node (an index) |
//! | 0xb0 | flags: 0x200000 for automatic segments |
//! | 0xb4, 0xb8 | the segment size (high and low words) |
//! | 0xc4 | the size of a segment file's own header |
//! | 0xcc | the most segments there can be |
//! | 0xdc | the number of segments the file has |
//!
//! With automatic segments the first segment is the file itself, up to the
//! segment size, and every later one a file of the same size that holds that
//! much less its own header: segment `k` ends at `size + k * (size -
//! header)`. A position belongs to the first segment whose end is above it,
//! at `position - (previous end - header)` in that file.
//!
//! A record is `[0xfafa u16][space u32][used u32]` and then `used` bytes,
//! which begin with the key's two words (type and instance, little-endian).
//!
//! # The index file
//! Nodes of node-size bytes, each with a 0x20-byte header (next node on its
//! level at 0, previous at 4, the entry count as a u16 at 8, 1 at 0x11 in a
//! leaf) and 16-byte entries — the position's low word, the 8-byte key, the
//! position's high word — in ascending key order (bytewise). In a leaf the
//! position is a record's; above, a child node's. A node ends with a 12-byte
//! trailer (its high key and a zero word) that no entry may overlap.

use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug)]
pub enum Error {
    /// A file couldn't be opened or read.
    Io(std::io::Error),
    /// The file's header or a segment doesn't say what the format requires.
    Format(&'static str),
    /// No record has the key.
    NotFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Format(what) => write!(f, "bad c-tree file: {what}"),
            Error::NotFound => write!(f, "no such record"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

const HEADER: usize = 0x200;
const AUTO_SEGMENTS: u32 = 0x20_0000;
const RECORD_MARKER: u16 = 0xfafa;
const RECORD_HEADER: usize = 10;
const NODE_HEADER: usize = 0x20;
const ENTRY: usize = 16;
const TRAILER: usize = 12;

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

/// An open c-tree file and its segments.
pub struct CtFile {
    header: [u8; HEADER],
    path: PathBuf,
    segments: Mutex<Vec<File>>,
}

impl CtFile {
    pub fn open(path: &Path) -> Result<CtFile> {
        let mut first = File::open(path)?;
        let mut header = [0; HEADER];
        first.read_exact(&mut header)?;
        if header[0x38] != 0 {
            return Err(Error::Format("the file was being changed and never closed"));
        }
        let file = CtFile { header, path: path.to_owned(), segments: Mutex::new(vec![first]) };
        if file.flags() & AUTO_SEGMENTS != 0 {
            let count = word(&header, 0xdc);
            if count == 0 {
                return Err(Error::Format("automatic segments, and none"));
            }
            let mut segments = file.segments.lock().unwrap();
            for k in 1..count.min(word(&header, 0xcc)) {
                let mut name = path.as_os_str().to_owned();
                name.push(format!(".{k:03}"));
                let mut segment = File::open(PathBuf::from(name))?;
                let mut head = vec![0; file.segment_header() as usize];
                segment.read_exact(&mut head)?;
                if &head[..7] != b"FC!SEGM" || word(&head, 0xcc) != k {
                    return Err(Error::Format("a segment file that doesn't belong"));
                }
                segments.push(segment);
            }
        }
        Ok(file)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn flags(&self) -> u32 {
        word(&self.header, 0xb0)
    }

    fn segment_size(&self) -> u64 {
        (word(&self.header, 0xb4) as u64) << 32 | word(&self.header, 0xb8) as u64
    }

    fn segment_header(&self) -> u64 {
        word(&self.header, 0xc4) as u64
    }

    /// Where segment `k` ends.
    fn segment_end(&self, k: u64) -> u64 {
        self.segment_size() + k * (self.segment_size() - self.segment_header())
    }

    /// The last byte in use (no record starts after it).
    pub fn in_use(&self) -> u32 {
        word(&self.header, 0x08)
    }

    pub fn node_size(&self) -> usize {
        word(&self.header, 0x0c) as usize
    }

    pub fn keys(&self) -> u32 {
        word(&self.header, 0x18)
    }

    pub fn root(&self) -> u32 {
        word(&self.header, 0x1c)
    }

    /// Reads `buf.len()` bytes at `position`, across segments.
    pub fn read(&self, mut position: u64, buf: &mut [u8]) -> Result<()> {
        let mut segments = self.segments.lock().unwrap();
        let auto = self.flags() & AUTO_SEGMENTS != 0;
        let mut done = 0;
        while done < buf.len() {
            let (k, offset, room) = if auto {
                let k = (0..segments.len() as u64)
                    .find(|&k| position < self.segment_end(k))
                    .ok_or(Error::Format("a position past the last segment"))?;
                let start = if k == 0 { 0 } else { self.segment_end(k - 1) - self.segment_header() };
                (k as usize, position - start, self.segment_end(k) - position)
            } else {
                (0, position, u64::MAX)
            };
            let n = (buf.len() - done).min(room.min(usize::MAX as u64) as usize);
            let file = &mut segments[k];
            file.seek(SeekFrom::Start(offset))?;
            file.read_exact(&mut buf[done..done + n])?;
            done += n;
            position += n as u64;
        }
        Ok(())
    }
}

/// A record's position in the data file.
pub type Position = u64;

/// The index: a B-tree from 8-byte keys to positions.
pub struct Index {
    file: CtFile,
}

struct Node {
    leaf: bool,
    next: u32,
    bytes: Vec<u8>,
    count: usize,
}

impl Node {
    fn key(&self, i: usize) -> [u8; 8] {
        let at = NODE_HEADER + i * ENTRY + 4;
        self.bytes[at..at + 8].try_into().unwrap()
    }

    fn position(&self, i: usize) -> Position {
        let at = NODE_HEADER + i * ENTRY;
        (word(&self.bytes, at + 12) as u64) << 32 | word(&self.bytes, at) as u64
    }
}

impl Index {
    pub fn open(path: &Path) -> Result<Index> {
        let file = CtFile::open(path)?;
        if file.node_size() < NODE_HEADER + ENTRY + TRAILER {
            return Err(Error::Format("a node size too small for an entry"));
        }
        Ok(Index { file })
    }

    pub fn keys(&self) -> u32 {
        self.file.keys()
    }

    fn node(&self, at: u32) -> Result<Node> {
        let size = self.file.node_size();
        let mut bytes = vec![0; size];
        self.file.read(at as u64, &mut bytes)?;
        let count = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        if NODE_HEADER + count * ENTRY > size - TRAILER {
            return Err(Error::Format("an index node with more entries than fit"));
        }
        Ok(Node { leaf: bytes[0x11] == 1, next: word(&bytes, 0), bytes, count })
    }

    /// The leaf that would hold `key` and the first entry in it not below it.
    fn seek(&self, key: &[u8; 8]) -> Result<Option<(Node, usize)>> {
        let mut at = self.file.root();
        if at == 0 {
            return Ok(None);
        }
        for _ in 0..32 {
            let node = self.node(at)?;
            let i = (0..node.count).find(|&i| &node.key(i) >= key).unwrap_or(node.count);
            if node.leaf {
                return Ok(Some((node, i)));
            }
            if i == node.count {
                return Ok(None);
            }
            at = node.position(i) as u32;
        }
        Err(Error::Format("an index deeper than any tree can be"))
    }

    /// The position the index keeps for `key`.
    pub fn find(&self, key: &[u8; 8]) -> Result<Option<Position>> {
        Ok(match self.seek(key)? {
            Some((node, i)) if i < node.count && &node.key(i) == key => Some(node.position(i)),
            _ => None,
        })
    }

    /// Every key and position, in key order, walking the leaves.
    pub fn entries(&self) -> Result<impl Iterator<Item = Result<([u8; 8], Position)>> + '_> {
        let mut leaf = self.seek(&[0; 8])?.map(|(node, _)| node);
        let mut i = 0;
        Ok(std::iter::from_fn(move || {
            loop {
                let node = leaf.as_ref()?;
                if i < node.count {
                    let entry = (node.key(i), node.position(i));
                    i += 1;
                    return Some(Ok(entry));
                }
                if node.next == 0 {
                    return None;
                }
                match self.node(node.next) {
                    Ok(next) => {
                        leaf = Some(next);
                        i = 0;
                    }
                    Err(e) => {
                        leaf = None;
                        return Some(Err(e));
                    }
                }
            }
        }))
    }
}

/// A data file and its index.
pub struct Database {
    data: CtFile,
    index: Index,
}

/// The key for a type and instance: both big-endian, so that bytewise order
/// is numeric order.
pub fn key(kind: u32, instance: u32) -> [u8; 8] {
    let mut k = [0; 8];
    k[..4].copy_from_slice(&kind.to_be_bytes());
    k[4..].copy_from_slice(&instance.to_be_bytes());
    k
}

/// A key's type and instance.
pub fn unkey(key: &[u8; 8]) -> (u32, u32) {
    (u32::from_be_bytes(key[..4].try_into().unwrap()), u32::from_be_bytes(key[4..].try_into().unwrap()))
}

impl Database {
    /// Opens `<base>.dat` and `<base>.idx`.
    pub fn open(base: &Path) -> Result<Database> {
        let with = |ext: &str| {
            let mut s = base.as_os_str().to_owned();
            s.push(ext);
            PathBuf::from(s)
        };
        Ok(Database { data: CtFile::open(&with(".dat"))?, index: Index::open(&with(".idx"))? })
    }

    pub fn len(&self) -> u32 {
        self.index.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every key, in order, with the position of its record.
    pub fn entries(&self) -> Result<impl Iterator<Item = Result<([u8; 8], Position)>> + '_> {
        self.index.entries()
    }

    /// The record at `position` (its header excluded), checked to be one.
    pub fn record_at(&self, position: Position) -> Result<Vec<u8>> {
        if position == 0 {
            return Err(Error::Format("a record at position 0"));
        }
        let mut header = [0; RECORD_HEADER];
        self.data.read(position, &mut header)?;
        if u16::from_le_bytes([header[0], header[1]]) != RECORD_MARKER {
            return Err(Error::Format("no record header where the index says"));
        }
        let used = word(&header, 6) as usize;
        let mut record = vec![0; used];
        self.data.read(position + RECORD_HEADER as u64, &mut record)?;
        Ok(record)
    }

    /// Checks the record at `position` is the one for `key` by reading only
    /// its header and key words; its length.
    pub fn check(&self, key: &[u8; 8], position: Position) -> Result<u32> {
        let mut head = [0; RECORD_HEADER + 8];
        self.data.read(position, &mut head)?;
        if u16::from_le_bytes([head[0], head[1]]) != RECORD_MARKER {
            return Err(Error::Format("no record header where the index says"));
        }
        if key != &self::key(word(&head, RECORD_HEADER), word(&head, RECORD_HEADER + 4)) {
            return Err(Error::Format("the record at the index's position holds another key"));
        }
        Ok(word(&head, 6))
    }

    /// The record with `key`, whole (its header excluded): the type and
    /// instance words, and the rest.
    pub fn record(&self, key: &[u8; 8]) -> Result<Vec<u8>> {
        let position = self.index.find(key)?.ok_or(Error::NotFound)?;
        let record = self.record_at(position)?;
        if record.len() < 8 || key != &self::key(word(&record, 0), word(&record, 4)) {
            return Err(Error::Format("the record at the index's position holds another key"));
        }
        Ok(record)
    }
}
