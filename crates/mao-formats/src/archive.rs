//! The object archive (`fun::ObjectArchive_c` of the original's
//! `serialize.dll`): a graph of objects each stored as a message of named,
//! typed fields. Meshes, animations and much else in the resource database
//! are archives.
//!
//! Derived from the original's reader (`AnarchyReimagined`:
//! `dropin/serialize/src/archive.rs`, `message.rs`). All integers are
//! little-endian.
//!
//! # The file
//! `[1 u32]`, the symbol count, each symbol a NUL-terminated prefix and a
//! NUL-terminated name; the object count; the root stream and then each
//! object's stream, each as `[size u32][bytes]`.
//!
//! # A stream
//! `[1 u32][field count u32]` and the fields. A field is its symbol index
//! (one byte below 0x80; two bytes, the first `0x80 | high 6 bits`; or four
//! bytes, the first `0xc0 | high 6 bits`; big-endian), then `[type u32]
//! [element size u32][used u32]` and `used` bytes of values. An element
//! size of 0 means each value is its own `[length u32][bytes]`.
//!
//! A reference to another object is a field of type 17 under the prefix
//! "0", holding the object's index. An object's class is the symbol its
//! `__class_id__` field (an int) names.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// The data ends inside something, or a count or size is impossible.
    Truncated(&'static str),
    /// The first word isn't 1.
    Version(u32),
    /// A field names a symbol the archive doesn't have.
    Symbol(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated(what) => write!(f, "archive truncated in {what}"),
            Error::Version(v) => write!(f, "archive format version {v} (only 1 is known)"),
            Error::Symbol(i) => write!(f, "field of symbol {i}, which doesn't exist"),
        }
    }
}

impl std::error::Error for Error {}

/// Field types the archive uses.
pub mod kind {
    pub const BOOL: u32 = 0;
    pub const INT16: u32 = 2;
    pub const INT32: u32 = 3;
    pub const MESSAGE: u32 = 5;
    pub const STRING: u32 = 6;
    pub const DATA: u32 = 9;
    pub const FLOAT: u32 = 10;
    pub const VECTOR3: u32 = 12;
    pub const QUATERNION: u32 = 13;
    pub const MATRIX4: u32 = 15;
    pub const RGB: u32 = 16;
    pub const OBJECT: u32 = 17;
}

/// One named field of a stream: its values, each as bytes.
#[derive(Clone, Debug)]
pub struct Field {
    pub prefix: String,
    pub name: String,
    pub kind: u32,
    pub values: Vec<Vec<u8>>,
}

/// An object's (or the root's) fields.
#[derive(Clone, Debug, Default)]
pub struct Stream {
    pub fields: Vec<Field>,
}

impl Stream {
    /// The first field called `name` under `prefix` (an empty prefix matches
    /// any).
    pub fn field(&self, prefix: &str, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name && (prefix.is_empty() || f.prefix == prefix))
    }

    pub fn int(&self, prefix: &str, name: &str) -> Option<i32> {
        let f = self.field(prefix, name)?;
        if f.kind != kind::INT32 && f.kind != kind::OBJECT {
            return None;
        }
        Some(i32::from_le_bytes(f.values.first()?.get(..4)?.try_into().ok()?))
    }
}

/// A whole archive.
#[derive(Clone, Debug)]
pub struct Archive {
    /// The symbols, `(prefix, name)`, by index.
    pub symbols: Vec<(String, String)>,
    pub root: Stream,
    pub objects: Vec<Stream>,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len()).ok_or(Error::Truncated(what))?;
        let bytes = &self.data[self.at..end];
        self.at = end;
        Ok(bytes)
    }

    fn u32(&mut self, what: &'static str) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }

    fn text(&mut self, what: &'static str) -> Result<String, Error> {
        let rest = &self.data[self.at..];
        let n = rest.iter().position(|&b| b == 0).ok_or(Error::Truncated(what))?;
        let text = String::from_utf8_lossy(&rest[..n]).into_owned();
        self.at += n + 1;
        Ok(text)
    }
}

fn stream(bytes: &[u8], symbols: &[(String, String)]) -> Result<Stream, Error> {
    let mut r = Reader { data: bytes, at: 0 };
    r.u32("a stream's version")?;
    let count = r.u32("a stream's field count")?;
    let mut fields = Vec::new();
    for _ in 0..count {
        let first = r.take(1, "a field's index")?[0];
        let index = if first & 0x80 == 0 {
            first as u32
        } else if first & 0xc0 == 0x80 {
            ((first & 0x3f) as u32) << 8 | r.take(1, "a field's index")?[0] as u32
        } else {
            let rest = r.take(3, "a field's index")?;
            ((first & 0x3f) as u32) << 24 | (rest[0] as u32) << 16 | (rest[1] as u32) << 8 | rest[2] as u32
        };
        let kind = r.u32("a field's type")?;
        let element = r.u32("a field's element size")? as usize;
        let used = r.u32("a field's size")? as usize;
        let data = r.take(used, "a field's values")?;
        let (prefix, name) = symbols.get(index as usize).ok_or(Error::Symbol(index))?;
        let mut values = Vec::new();
        if element == 0 {
            let mut v = Reader { data, at: 0 };
            while v.at < data.len() {
                let n = v.u32("a value's length")? as usize;
                values.push(v.take(n, "a value")?.to_vec());
            }
        } else {
            values.extend(data.chunks(element).map(<[u8]>::to_vec));
        }
        fields.push(Field { prefix: prefix.clone(), name: name.clone(), kind, values });
    }
    Ok(Stream { fields })
}

impl Archive {
    pub fn parse(bytes: &[u8]) -> Result<Archive, Error> {
        let mut r = Reader { data: bytes, at: 0 };
        let version = r.u32("the version")?;
        if version != 1 {
            return Err(Error::Version(version));
        }
        let count = r.u32("the symbol count")?;
        let mut symbols = Vec::new();
        for _ in 0..count {
            let prefix = r.text("a symbol")?;
            let name = r.text("a symbol")?;
            symbols.push((prefix, name));
        }
        let objects = r.u32("the object count")?;
        let mut streams = Vec::new();
        for _ in 0..=objects {
            let size = r.u32("a stream's size")? as usize;
            streams.push(stream(r.take(size, "a stream")?, &symbols)?);
        }
        let root = streams.remove(0);
        Ok(Archive { symbols, root, objects: streams })
    }

    /// An object's class name.
    pub fn class(&self, object: usize) -> Option<&str> {
        let id = self.objects.get(object)?.int("", "__class_id__")?;
        self.symbols.get(id as usize).map(|(_, name)| name.as_str())
    }
}
