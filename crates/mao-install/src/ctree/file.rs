//! A c-tree file: its 0x200-byte header and its bytes, which may run on
//! into segment files (`<name>.001`, `<name>.002`, …) that each start with
//! a header of their own.
//!
//! The header fields this crate needs (little-endian words):
//!
//! | offset | field |
//! |---|---|
//! | 0x00 | the last byte the file has room for |
//! | 0x08 | the last byte in use: no record starts after it |
//! | 0x0c | the node size |
//! | 0x14 | the number of records (a data file) |
//! | 0x18 | the number of keys (an index) |
//! | 0x1c | the B-tree's root node (an index) |
//! | 0x38 | nonzero while the file is being changed (until it is closed) |
//! | 0x20, 0x24, 0x28 (u16), 0x40 (u16), 0x58, 0x5c (u16) | copied into every segment's header, which must match |
//! | 0xb0 | flags: 0x200000 for automatic segments |
//! | 0xb4, 0xb8 | the segment size (high and low words) |
//! | 0xc4 | the size of a segment file's own header |
//! | 0xcc | the most segments there can be |
//! | 0xdc | the number of segments the file has |
//!
//! With automatic segments the first segment is the file itself, up to
//! the segment size, and every later one a file of the same size holding
//! that much less its header: segment k ends at `size + k * (size -
//! header)`. Those ends are compared with the 32-bit position of a read
//! (only their low words: the game doesn't open its files in huge-file
//! mode, so past 4 GB the ends wrap around and the later segments can't be
//! picked); the first segment whose end is above it holds it, at
//! `position - (previous end - header)` in its file. A segment file's
//! header has "FC!SEGM" and the fields above, its size at 0xc4 and its
//! number at 0xcc.
//!
//! The engine opens the file's segments when it opens the file, and fails
//! if one is missing or doesn't match (or the header says there are none).
//! With the file opened read-only, a segment whose word at 0x20 is 0 needn't
//! match at 0x20, 0x24 and 0x58 (when the file's own word at 0x20 isn't 0).
//! A read past the last segment the header counts makes it start the next
//! one, even with the file opened read-only: it creates the segment file,
//! or, finding one that is already there and matches, renames that one out
//! of the way (to `<name>.NNN.<hex>`) and creates a fresh one, writes the
//! new segment's header and reads nothing from it. Here such a read just
//! fails, as the engine's does, and no file is touched.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::{Error, Result};

/// The size of a variable-length record's header: the marker, the space
/// the record has and the length it uses.
pub const RECORD_HEADER: usize = 10;

/// The header fields, as read when the file was opened.
#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub bytes: [u8; Header::SIZE],
    pub capacity: u32,
    pub in_use: u32,
    pub node_size: u32,
    pub records: u32,
    pub keys: u32,
    pub root: u32,
    pub flags: u32,
    pub segment_size: u64,
    pub segment_header: u32,
    pub max_segments: u32,
    pub segments: u32,
}

/// The flag for automatic segments.
pub const AUTO_SEGMENTS: u32 = 0x20_0000;
/// What a segment file's header starts with.
pub const SEGMENT_SIGNATURE: &[u8; 8] = b"FC!SEGM\0";

impl Header {
    pub const SIZE: usize = 0x200;

    pub fn parse(bytes: &[u8; Header::SIZE]) -> Header {
        let word = |offset: usize| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        Header {
            bytes: *bytes,
            capacity: word(0x00),
            in_use: word(0x08),
            node_size: word(0x0c),
            records: word(0x14),
            keys: word(0x18),
            root: word(0x1c),
            flags: word(0xb0),
            segment_size: (word(0xb4) as u64) << 32 | word(0xb8) as u64,
            segment_header: word(0xc4),
            max_segments: word(0xcc),
            segments: word(0xdc),
        }
    }

    /// Whether the file was being changed and never closed (the engine
    /// won't open it again).
    pub fn updating(&self) -> bool {
        self.bytes[0x38] != 0
    }

    pub fn auto_segments(&self) -> bool {
        self.flags & AUTO_SEGMENTS != 0
    }

    /// Where segment `k` ends, as the engine compares it (the low word of
    /// `size + k * (size - header)`).
    pub fn segment_end(&self, k: u32) -> u32 {
        let step = self.segment_size.wrapping_sub(self.segment_header as u64);
        self.segment_size.wrapping_add(step.wrapping_mul(k as u64)) as u32
    }

    /// Whether `segment` (a segment file's first bytes) is segment number
    /// `k` of this file, opened read-only or not.
    pub fn matches_segment(&self, segment: &[u8], k: u32, read_only: bool) -> bool {
        let at = |b: &[u8], o: usize, n: usize| b.get(o..o + n).map(|x| x.to_vec());
        let same = |o: usize, n: usize| at(segment, o, n).is_some() && at(segment, o, n) == at(&self.bytes, o, n);
        let word = |o: usize| at(segment, o, 4).map(|x| u32::from_le_bytes(x.try_into().unwrap()));
        let own = |o: usize| u32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap());
        let stamped = (same(0x20, 4) && same(0x24, 4) && same(0x58, 4)) || (read_only && word(0x20) == Some(0) && own(0x20) != 0);
        word(0xcc) == Some(k) && word(0xc4) == Some(self.segment_header) && stamped && same(0x28, 2) && same(0x40, 2) && same(0x5c, 2)
    }
}

/// An open c-tree file.
pub struct CtFile {
    path: PathBuf,
    header: Header,
    segments: Mutex<Vec<File>>,
    writable: bool,
    /// Whether the header says the file is being changed (from the first
    /// change until it is closed).
    updating: bool,
}

fn read_exact_at(file: &mut File, offset: u64, buf: &mut [u8]) -> Result<()> {
    file.seek(SeekFrom::Start(offset)).map_err(|_| Error::READ)?;
    file.read_exact(buf).map_err(|_| Error::READ)
}

fn write_all_at(file: &mut File, offset: u64, data: &[u8]) -> Result<()> {
    file.seek(SeekFrom::Start(offset)).map_err(|_| Error::WRITE)?;
    file.write_all(data).map_err(|_| Error::WRITE)
}

/// The time as the headers keep it (seconds since 1970).
pub fn now() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as u32).unwrap_or(0)
}

impl CtFile {
    /// Opens the file read-only, and its segments (each must be there and
    /// match).
    pub fn open(path: &Path) -> Result<CtFile> {
        CtFile::open_with(path, false)
    }

    /// Opens the file, for reading and writing when `writable`.
    pub fn open_with(path: &Path, writable: bool) -> Result<CtFile> {
        CtFile::open_raw(path, writable, false)
    }

    /// Opens the file, a file still marked as being changed too when
    /// `changing` (for rebuilding it).
    pub fn open_raw(path: &Path, writable: bool, changing: bool) -> Result<CtFile> {
        let mut file = std::fs::OpenOptions::new().read(true).write(writable).open(path).map_err(|_| Error::OPEN)?;
        let mut bytes = [0; Header::SIZE];
        read_exact_at(&mut file, 0, &mut bytes)?;
        let header = Header::parse(&bytes);
        if header.updating() && !changing {
            return Err(Error::NOT_CLOSED);
        }
        let ct = CtFile { path: path.to_owned(), header, segments: Mutex::new(vec![file]), writable, updating: false };
        if header.auto_segments() {
            if header.segments == 0 {
                return Err(Error::SEGMENT);
            }
            let mut segments = ct.segments.lock().unwrap();
            for k in 1..header.segments.min(header.max_segments) {
                segments.push(ct.open_segment(k)?);
            }
        }
        Ok(ct)
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The name of segment file `k` (1 and up).
    pub fn segment_path(&self, k: u32) -> PathBuf {
        let mut name = self.path.as_os_str().to_owned();
        name.push(format!(".{k:03}"));
        PathBuf::from(name)
    }

    /// Opens segment file `k` and checks its header.
    fn open_segment(&self, k: u32) -> Result<File> {
        let mut file = std::fs::OpenOptions::new().read(true).write(self.writable).open(self.segment_path(k)).map_err(|_| Error::OPEN)?;
        let mut head = vec![0; self.header.segment_header as usize];
        read_exact_at(&mut file, 0, &mut head).map_err(|_| Error::SEGMENT)?;
        if !self.header.matches_segment(&head, k, !self.writable) {
            return Err(Error::SEGMENT);
        }
        Ok(file)
    }

    /// Creates segment file `k` (it must not be there yet), with its header.
    fn create_segment(&self, k: u32) -> Result<File> {
        let h = &self.header;
        let mut head = vec![0xff; h.segment_header as usize];
        head[..8].copy_from_slice(SEGMENT_SIGNATURE);
        for (at, n) in [(0x20, 8), (0x28, 2), (0x40, 2), (0x58, 4), (0x5c, 2), (0x64, 4)] {
            head[at..at + n].copy_from_slice(&h.bytes[at..at + n]);
        }
        let step = h.segment_size.wrapping_sub(h.segment_header as u64);
        let end = h.segment_size.wrapping_add(step.wrapping_mul(k as u64));
        let mut word = |at: usize, value: u32| head[at..at + 4].copy_from_slice(&value.to_le_bytes());
        word(0x7c, now());
        word(0xb4, (end >> 32) as u32);
        word(0xb8, end as u32);
        word(0xc4, h.segment_header);
        word(0xcc, k);
        let mut file = std::fs::OpenOptions::new().read(true).write(true).create_new(true).open(self.segment_path(k)).map_err(|_| Error::SEGMENT)?;
        write_all_at(&mut file, 0, &head)?;
        Ok(file)
    }

    /// Where `position` is: the segment and the offset in its file, and
    /// how many bytes from there are in that segment (None: past the
    /// segments the file has).
    fn locate(&self, count: usize, position: u32) -> Option<(usize, u64, usize)> {
        let h = &self.header;
        if !h.auto_segments() {
            return Some((0, position as u64, usize::MAX));
        }
        (0..count).find_map(|k| {
            let end = h.segment_end(k as u32);
            (position < end).then(|| {
                let start = if k == 0 { 0 } else { h.segment_end(k as u32 - 1).wrapping_sub(h.segment_header) };
                (k, position.wrapping_sub(start) as u64, (end - position) as usize)
            })
        })
    }

    /// Reads `buf.len()` bytes at `position`, through the segments (a read
    /// past the end of a file fails as short, with the bytes before it
    /// filled in).
    pub fn read(&self, position: u32, buf: &mut [u8]) -> Result<()> {
        let mut segments = self.segments.lock().unwrap();
        let mut position = position;
        let mut done = 0;
        while done < buf.len() {
            let (k, offset, room) = self.locate(segments.len(), position).ok_or(Error::SEGMENT)?;
            let n = (buf.len() - done).min(room);
            read_exact_at(&mut segments[k], offset, &mut buf[done..done + n])?;
            done += n;
            position = position.wrapping_add(n as u32);
        }
        Ok(())
    }

    /// Writes `data` at `position`, through the segments, starting the
    /// segment files it reaches.
    pub fn write(&mut self, position: u32, data: &[u8]) -> Result<()> {
        if !self.writable {
            return Err(Error::READ_ONLY);
        }
        let mut position = position;
        let mut done = 0;
        while done < data.len() {
            let count = self.segments.get_mut().unwrap().len();
            let Some((k, offset, room)) = self.locate(count, position) else {
                if count as u32 >= self.header.max_segments {
                    return Err(Error::SEGMENT);
                }
                let file = self.create_segment(count as u32)?;
                self.segments.get_mut().unwrap().push(file);
                self.set_word(0xdc, count as u32 + 1);
                continue;
            };
            let n = (data.len() - done).min(room);
            write_all_at(&mut self.segments.get_mut().unwrap()[k], offset, &data[done..done + n])?;
            done += n;
            position = position.wrapping_add(n as u32);
        }
        Ok(())
    }

    /// Sets a header word (written out with the header).
    pub fn set_word(&mut self, offset: usize, value: u32) {
        self.header.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        self.header = Header::parse(&self.header.bytes);
    }

    pub fn word(&self, offset: usize) -> u32 {
        u32::from_le_bytes(self.header.bytes[offset..offset + 4].try_into().unwrap())
    }

    fn write_header(&mut self) -> Result<()> {
        let bytes = self.header.bytes;
        write_all_at(&mut self.segments.get_mut().unwrap()[0], 0, &bytes)
    }

    /// Counts a change in the header's change counter (kept twice).
    pub fn count_change(&mut self) {
        let n = self.word(0x6c).wrapping_add(1);
        self.set_word(0x6c, n);
        self.set_word(0xec, n);
    }

    /// Before the first change: marks the file as being changed (opened
    /// with `mode`) and writes the header, so a file that is never closed
    /// won't be opened again.
    pub fn begin_change(&mut self, mode: u16) -> Result<()> {
        if !self.writable {
            return Err(Error::READ_ONLY);
        }
        if !self.updating {
            self.header.bytes[0x30..0x32].copy_from_slice(&mode.to_le_bytes());
            self.header.bytes[0x38] = 0xff;
            self.count_change();
            self.write_header()?;
            self.updating = true;
        }
        Ok(())
    }

    /// Makes room up to byte `last`: the file grows by its extension size
    /// (header 0xa8) at a time, the new bytes all 0xff.
    pub fn grow(&mut self, last: u32) -> Result<()> {
        let step = match self.word(0xa8) {
            0 => 0x10_0000,
            n => n,
        };
        while self.header.capacity < last {
            let from = self.header.capacity.wrapping_add(1);
            let to = self.header.capacity.checked_add(step).ok_or(Error::FULL)?;
            let fill = vec![0xff; 0x10000];
            let mut at = from;
            while at <= to {
                let n = ((to - at) as usize + 1).min(fill.len());
                self.write(at, &fill[..n])?;
                at = match at.checked_add(n as u32) {
                    Some(next) => next,
                    None => break,
                };
            }
            self.set_word(0x00, to);
        }
        Ok(())
    }

    /// Writes the header back and flushes, keeping the file marked as
    /// being changed (BlockDatabase_t::Sync).
    pub fn sync(&mut self) -> Result<()> {
        if self.updating {
            self.write_header()?;
            for file in self.segments.get_mut().unwrap().iter_mut() {
                file.flush().map_err(|_| Error::WRITE)?;
            }
        }
        Ok(())
    }

    /// Ends the changes: the header loses its mark and gets the time.
    pub fn close(&mut self) -> Result<()> {
        if self.updating {
            self.header.bytes[0x38] = 0;
            self.count_change();
            self.set_word(0x70, now());
            self.write_header()?;
            for file in self.segments.get_mut().unwrap().iter_mut() {
                file.flush().map_err(|_| Error::WRITE)?;
            }
            self.updating = false;
        }
        Ok(())
    }
}
