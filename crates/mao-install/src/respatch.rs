//! Applying one of Funcom's resource-database patches (`.RES`) to the resource database.
//!
//! Read from `AnarchyPatcher.exe` 1.0.3.1, `FUN_004662f0`. The file, all words big-endian:
//!
//! | | |
//! |---|---|
//! | `u32` | 0xf1576, 0xf1577 or 0xf1578: the format's version |
//! | `u32` | the number of records the file deletes and brings, together (the patcher's progress bar) |
//! | `u32`, then that many (`u32` type, `u32` instance) | records to delete |
//! | `u32`, then that many records | records to add, or to put in place of the one there |
//!
//! A record is `u32` type, `u32` instance, `u32` size, and then by kind:
//!
//! - a whole record (every type in version 0xf1576, and since then every type but four): nothing more when the
//!   size is 0; else `u32` the packed size and the record's data packed with zlib, `size` bytes unpacked;
//! - a patch (types 0xf4241, 0xf4249, 0xf424a and 0xf425a since version 0xf1577): `size` bytes that are an RTPatch
//!   file for one file called `tmp.dat`. The patcher writes the record now in the database to `tmp.dat` (no file
//!   when there is no such record), runs RTPatch on it, and takes the new record from `tmp.dat`
//!   ([`crate::rtpatch`]). A record that is new comes as a patch that adds the file;
//! - type 0xf4254, instance 0 is something else again (`FUN_0046f0e0`). None of Funcom's patches from 18.8.50 on
//!   has one, so it is refused here.
//!
//! In the database a record is the type and the instance, the data's length (three little-endian words) and the
//! data (`FUN_0046c250`, which rewrites the record with the key when there is one and adds it when there is not).
//!
//! An empty patch is 16 bytes.

use std::fmt;
use std::io::Read;

use crate::ctree::{self, Database};
use crate::rtpatch::{self, Tree};

#[derive(Debug)]
pub enum Error {
    /// The patch file is not what the format says.
    Format(String),
    /// A part of the format that Funcom's patches never used.
    Unsupported(String),
    /// A record's own patch could not be applied.
    Record { kind: u32, instance: u32, why: rtpatch::Error },
    /// The database refused (the c-tree engine's error number).
    Database { kind: u32, instance: u32, code: u16 },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Format(what) => write!(f, "bad database patch: {what}"),
            Error::Unsupported(what) => write!(f, "the database patch uses something this patcher does not know: {what}"),
            Error::Record { kind, instance, why } => write!(f, "record {kind}:{instance}: {why}"),
            Error::Database { kind, instance, code } => write!(f, "record {kind}:{instance}: the database gave error {code}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// The types whose records come as patches of the record there.
pub const PATCHED_TYPES: [u32; 4] = [0xf4249, 0xf4241, 0xf425a, 0xf424a];

/// What a patch brings for one record.
#[derive(Clone, Debug)]
pub enum Body<'a> {
    /// The record's data.
    Whole(Vec<u8>),
    /// An RTPatch file that makes the record's data from the one in the database.
    Patch(&'a [u8]),
}

#[derive(Clone, Debug)]
pub struct Record<'a> {
    pub kind: u32,
    pub instance: u32,
    pub body: Body<'a>,
}

/// A database patch, read.
#[derive(Clone, Debug)]
pub struct ResPatch<'a> {
    pub version: u32,
    pub deletes: Vec<(u32, u32)>,
    pub records: Vec<Record<'a>>,
}

struct Reader<'a> {
    d: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    fn raw(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.p.checked_add(n).filter(|&end| end <= self.d.len());
        let Some(end) = end else { return Err(Error::Format(format!("it ends inside a field at {:#x}", self.p))) };
        let s = &self.d[self.p..end];
        self.p = end;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.raw(4)?.try_into().unwrap()))
    }
}

impl<'a> ResPatch<'a> {
    pub fn parse(data: &'a [u8]) -> Result<ResPatch<'a>> {
        let mut r = Reader { d: data, p: 0 };
        let version = r.u32()?;
        if !(0xf1576..=0xf1578).contains(&version) {
            return Err(Error::Format(format!("it starts with {version:#x}")));
        }
        r.u32()?;
        let mut deletes = Vec::new();
        for _ in 0..r.u32()? {
            deletes.push((r.u32()?, r.u32()?));
        }
        let mut records = Vec::new();
        for _ in 0..r.u32()? {
            let (kind, instance, size) = (r.u32()?, r.u32()?, r.u32()? as usize);
            let body = if kind == 0xf4254 && instance == 0 {
                return Err(Error::Unsupported("a record 0xf4254:0".into()));
            } else if version >= 0xf1577 && PATCHED_TYPES.contains(&kind) {
                Body::Patch(r.raw(size)?)
            } else if size == 0 {
                Body::Whole(Vec::new())
            } else {
                let packed_size = r.u32()? as usize;
                let packed = r.raw(packed_size)?;
                let mut whole = Vec::with_capacity(size);
                // (one byte more than there should be, to see a stream that is longer than it says)
                let read = flate2::read::ZlibDecoder::new(packed).take(size as u64 + 1).read_to_end(&mut whole);
                if read.is_err() || whole.len() != size {
                    return Err(Error::Format(format!("record {kind}:{instance} does not unpack to its size")));
                }
                Body::Whole(whole)
            };
            records.push(Record { kind, instance, body });
        }
        if r.p != data.len() {
            return Err(Error::Format("there is more after the last record".into()));
        }
        Ok(ResPatch { version, deletes, records })
    }

    /// A patch that changes nothing (Funcom sends one with every step that has no database changes).
    pub fn is_empty(&self) -> bool {
        self.deletes.is_empty() && self.records.is_empty()
    }
}

/// What applying a database patch did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub added: u32,
    pub replaced: u32,
    pub deleted: u32,
    /// Records the patch deletes that were not there (the original patcher writes a line in its log and goes on).
    pub already_gone: u32,
}

/// A record's data in the database: what follows the type, the instance and the length.
fn data_of(db: &Database, kind: u32, instance: u32) -> Result<Option<(ctree::Position, Vec<u8>)>> {
    let key = ctree::key(kind, instance);
    let failed = |e: ctree::Error| Error::Database { kind, instance, code: e.0 };
    let position = match db.find(&key) {
        Ok(position) => position,
        Err(ctree::Error::NOT_FOUND) => return Ok(None),
        Err(e) => return Err(failed(e)),
    };
    let used = db.record_length(position).map_err(failed)?;
    let mut record = vec![0; used as usize];
    db.read_record(position, &mut record).map_err(failed)?;
    record.drain(..12.min(record.len()));
    Ok(Some((position, record)))
}

/// The name of the file a record's patch works on.
const RECORD_FILE: &str = "tmp.dat";

/// Applies a database patch. The database's files are changed as the original patcher changes them: a record is
/// rewritten where it is when the new one fits, and added at the end when not. A record that is already what the
/// patch makes of it is left alone, so a patch can be applied again after a run that was cut short.
///
/// `progress` is told how many of the patch's records are done, and how many there are.
pub fn apply(patch: &[u8], db: &mut Database, progress: &mut dyn FnMut(usize, usize)) -> Result<Report> {
    let patch = ResPatch::parse(patch)?;
    let total = patch.deletes.len() + patch.records.len();
    let mut report = Report::default();
    let mut done = 0;
    for &(kind, instance) in &patch.deletes {
        let failed = |e: ctree::Error| Error::Database { kind, instance, code: e.0 };
        let key = ctree::key(kind, instance);
        match db.find(&key) {
            Ok(position) => {
                db.delete(&key, position).map_err(failed)?;
                report.deleted += 1;
            }
            Err(ctree::Error::NOT_FOUND) => report.already_gone += 1,
            Err(e) => return Err(failed(e)),
        }
        done += 1;
        progress(done, total);
    }
    for record in &patch.records {
        let (kind, instance) = (record.kind, record.instance);
        let failed = |e: ctree::Error| Error::Database { kind, instance, code: e.0 };
        let there = data_of(db, kind, instance)?;
        let data = match &record.body {
            Body::Whole(data) => data.clone(),
            Body::Patch(patch) => {
                let mut files = rtpatch::Memory::default();
                if let Some((_, old)) = &there {
                    files.write(RECORD_FILE, old, None).expect("memory");
                }
                rtpatch::apply(patch, &mut files).map_err(|why| Error::Record { kind, instance, why })?;
                match files.read(RECORD_FILE).expect("memory") {
                    Some(new) => new,
                    None => return Err(Error::Format(format!("record {kind}:{instance}: its patch leaves no record"))),
                }
            }
        };
        let key = ctree::key(kind, instance);
        let mut whole = Vec::with_capacity(12 + data.len());
        whole.extend_from_slice(&kind.to_le_bytes());
        whole.extend_from_slice(&instance.to_le_bytes());
        whole.extend_from_slice(&(data.len() as u32).to_le_bytes());
        whole.extend_from_slice(&data);
        match there {
            Some((_, old)) if old == data => {}
            Some((position, _)) => {
                db.rewrite(&key, position, &whole).map_err(failed)?;
                report.replaced += 1;
            }
            None => {
                db.add(&key, &whole).map_err(failed)?;
                report.added += 1;
            }
        }
        done += 1;
        progress(done, total);
    }
    db.sync().map_err(|e| Error::Database { kind: 0, instance: 0, code: e.0 })?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_patch() {
        let patch = ResPatch::parse(&[0, 0xf, 0x15, 0x78, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!((patch.version, patch.deletes.len(), patch.records.len()), (0xf1578, 0, 0));
        assert!(ResPatch::parse(&[0, 0xf, 0x15, 0x79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
        assert!(ResPatch::parse(&[0, 0xf, 0x15, 0x78, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
    }

    #[test]
    fn records() {
        // one delete; a whole record of five bytes; a whole record of none; a patched type's record
        let packed = {
            let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            std::io::Write::write_all(&mut e, b"hello").unwrap();
            e.finish().unwrap()
        };
        let mut d = Vec::new();
        let word = |d: &mut Vec<u8>, v: u32| d.extend_from_slice(&v.to_be_bytes());
        for v in [0xf1578, 4, 1, 1000020, 7, 3, 1000020, 8, 5, packed.len() as u32] {
            word(&mut d, v);
        }
        d.extend_from_slice(&packed);
        for v in [1000020, 9, 0, 0xf4241, 2, 3] {
            word(&mut d, v);
        }
        d.extend_from_slice(b"K*x");
        let patch = ResPatch::parse(&d).unwrap();
        assert_eq!(patch.deletes, [(1000020, 7)]);
        assert!(matches!(&patch.records[0].body, Body::Whole(data) if data == b"hello"));
        assert!(matches!(&patch.records[1].body, Body::Whole(data) if data.is_empty()));
        assert!(matches!(&patch.records[2].body, Body::Patch(data) if data == b"K*x"));
        // (a record that unpacks to another size than it says)
        d[4 * 8 + 3] = 4;
        assert!(ResPatch::parse(&d).is_err());
    }
}
