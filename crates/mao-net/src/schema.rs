//! The protocol as data: an interpreter for `schema/protocol.json`, which
//! `tools/gen_protocol_schema.py` writes from the SmokeLounge message definitions (WTFPL) that
//! ship with the AOSharp sources. Parsing a message gives its fields by name, with enum members
//! named, so captures and live traffic can be read without hand decoding.
//!
//! Wire rules (all big-endian): `bool` and `byte` one byte, `short` two, `int` / `uint` / `float`
//! four, `long` / `double` eight; a string is ASCII, sized by a byte / `i16` / `i32` prefix, or fixed,
//! or null-terminated; an array is sized the same way, or by an `i32` holding `(count + 1) * 0x3f1`
//! (`x3f1`); an `Identity` is two `i32` (type, instance); a field with `when` conditions exists only
//! when an earlier flag field has one of the listed values, and then has that entry's type.
//! A message's data starts with the unknown byte of the common header (the kind and identity are
//! already in [`crate::n3::Message`]).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::OnceLock;

use serde::Deserialize;

const SCHEMA: &str = include_str!("../schema/protocol.json");

/// Fields the generated schema has no size for, found from the captures (the length prefix of the string): (the message's class,
/// the field, the size as `Field::size` names it).
const SIZE_OVERRIDES: &[(&str, &str, &str)] = &[("SimpleCharFullUpdateMessage", "Name", "byte")];

/// The field as read or written: with the size its class needs where the schema has none (see [`SIZE_OVERRIDES`]).
fn sized(class: &str, f: &Field) -> Option<Field> {
    let (_, _, size) = SIZE_OVERRIDES.iter().find(|(c, n, _)| *c == class && *n == f.name)?;
    let mut f = f.clone();
    f.size = Some(size.to_string());
    Some(f)
}

#[derive(Deserialize)]
struct Raw {
    classes: HashMap<String, Class>,
    enums: HashMap<String, Enum>,
    n3: HashMap<String, N3>,
}

#[derive(Deserialize)]
struct Class {
    base: Option<String>,
    fields: Vec<Field>,
}

#[derive(Clone, Default, Deserialize)]
struct Field {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    size: Option<String>,
    fixed: Option<i64>,
    flag: Option<String>,
    #[serde(default)]
    when: Vec<When>,
}

#[derive(Clone, Deserialize)]
struct When {
    flag: String,
    #[serde(rename = "type")]
    ty: String,
    values: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct Enum {
    under: String,
    values: HashMap<String, i64>,
}

#[derive(Deserialize)]
struct N3 {
    class: String,
    name: String,
}

/// A parsed value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    /// An enum member: its number and name (empty when the number has no name).
    Enum(i64, String),
    Float(f64),
    Bool(bool),
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<Value>),
    /// A class: its fields in order.
    Struct(Vec<(String, Value)>),
    /// A conditional field that was absent.
    Null,
}

impl Value {
    /// The field `name` of a struct.
    pub fn get(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Struct(f) => f.iter().find(|(n, _)| n == name).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) | Value::Enum(i, _) => Some(*i),
            Value::Bool(b) => Some(*b as i64),
            _ => None,
        }
    }

    /// An indented, readable form.
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        let pad = "  ".repeat(depth);
        match self {
            Value::Int(i) => {
                let _ = write!(out, "{i}");
            }
            Value::Enum(i, name) => {
                let _ = if name.is_empty() { write!(out, "{i}") } else { write!(out, "{name} ({i})") };
            }
            Value::Float(f) => {
                let _ = write!(out, "{f}");
            }
            Value::Bool(b) => {
                let _ = write!(out, "{b}");
            }
            Value::Str(s) => {
                let _ = write!(out, "{s:?}");
            }
            Value::Bytes(b) => {
                let _ = write!(out, "{} bytes {}", b.len(), b.iter().take(24).map(|x| format!("{x:02x}")).collect::<String>());
            }
            Value::Null => out.push('-'),
            Value::List(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                }
                for (i, item) in items.iter().enumerate().take(40) {
                    let _ = write!(out, "\n{pad}  [{i}] ");
                    item.write(out, depth + 2);
                }
                if items.len() > 40 {
                    let _ = write!(out, "\n{pad}  ... {} more", items.len() - 40);
                }
            }
            Value::Struct(fields) => {
                for (name, v) in fields {
                    let _ = write!(out, "\n{pad}{name}: ");
                    v.write(out, depth + 1);
                }
            }
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize, what: &str) -> Result<&[u8], String> {
        if self.at + n > self.data.len() {
            return Err(format!("{what}: needs {n} bytes at {}, {} left", self.at, self.data.len() - self.at));
        }
        let s = &self.data[self.at..self.at + n];
        self.at += n;
        Ok(s)
    }

    fn int(&mut self, bytes: usize, signed: bool, what: &str) -> Result<i64, String> {
        let b = self.take(bytes, what)?;
        let mut v: u64 = 0;
        for x in b {
            v = v << 8 | *x as u64;
        }
        Ok(if signed {
            let shift = 64 - 8 * bytes as u32;
            ((v << shift) as i64) >> shift
        } else {
            v as i64
        })
    }

    fn float(&mut self) -> Result<f64, String> {
        Ok(f32::from_bits(self.int(4, false, "float")? as u32) as f64)
    }
}

pub struct Schema {
    raw: Raw,
}

/// The result of parsing a message.
pub struct Parsed {
    pub name: String,
    pub value: Value,
    /// Bytes of the message's data left over after the last field.
    pub left: usize,
}

impl Schema {
    pub fn global() -> &'static Schema {
        static S: OnceLock<Schema> = OnceLock::new();
        S.get_or_init(|| Schema { raw: serde_json::from_str(SCHEMA).expect("the protocol schema") })
    }

    /// The name of an N3 message kind.
    pub fn n3_name(&self, kind: u32) -> Option<&str> {
        self.raw.n3.get(&format!("{kind:08x}")).map(|m| m.name.as_str())
    }

    /// An enum member's name.
    pub fn enum_name(&self, enum_name: &str, value: i64) -> Option<&str> {
        self.enum_of(enum_name)?.values.iter().find(|(_, v)| **v == value).map(|(n, _)| n.as_str())
    }

    fn enum_of(&self, name: &str) -> Option<&Enum> {
        self.raw.enums.get(name).or_else(|| self.raw.enums.get(name.rsplit('.').next()?))
    }

    fn class_of(&self, name: &str) -> Option<&Class> {
        self.raw.classes.get(name).or_else(|| self.raw.classes.get(name.rsplit('.').next()?))
    }

    /// Parses the data of an N3 message of `kind` (what follows its identity).
    pub fn parse_n3(&self, kind: u32, data: &[u8]) -> Result<Parsed, String> {
        let entry = self.raw.n3.get(&format!("{kind:08x}")).ok_or_else(|| format!("no definition for {kind:08x}"))?;
        let mut r = Reader { data, at: 0 };
        r.take(1, "header byte")?;
        let value = self.read_class(&entry.class, &mut r, 0)?;
        Ok(Parsed { name: entry.name.clone(), value, left: data.len() - r.at })
    }

    fn read_class(&self, name: &str, r: &mut Reader, depth: usize) -> Result<Value, String> {
        if depth > 12 {
            return Err(format!("{name}: nested too deep"));
        }
        let class = self.class_of(name).ok_or_else(|| format!("unknown class {name}"))?;
        let mut fields: Vec<(String, Value)> = Vec::new();
        let mut flags: HashMap<String, i64> = HashMap::new();
        let mut chain: Vec<&Class> = vec![class];
        let mut base = class.base.as_deref();
        while let Some(b) = base {
            match self.class_of(b) {
                Some(c) if b != "N3Message" && b != "MessageBody" => {
                    chain.insert(0, c);
                    base = c.base.as_deref();
                }
                _ => break,
            }
        }
        for class in chain {
            for f in &class.fields {
                let f = &sized(name, f).unwrap_or_else(|| f.clone());
                let (ty, present) = if f.when.is_empty() {
                    (f.ty.as_str(), true)
                } else {
                    match f.when.iter().find(|w| flags.get(&w.flag).is_some_and(|v| w.values.iter().any(|x| x.as_i64() == Some(*v)))) {
                        Some(w) => (w.ty.as_str(), true),
                        None => (f.ty.as_str(), false),
                    }
                };
                if !present {
                    fields.push((f.name.clone(), Value::Null));
                    continue;
                }
                let v = self.read_type(ty, f, r, depth).map_err(|e| format!("{name}.{}: {e}", f.name))?;
                if let (Some(flag), Some(i)) = (&f.flag, v.as_int()) {
                    flags.insert(flag.clone(), i);
                }
                fields.push((f.name.clone(), v));
            }
        }
        Ok(Value::Struct(fields))
    }

    fn count(&self, f: &Field, r: &mut Reader) -> Result<usize, String> {
        let n = match f.size.as_deref() {
            Some("byte") => r.int(1, false, "count")?,
            Some("int16") => r.int(2, true, "count")?,
            Some("int32") => r.int(4, true, "count")?,
            Some("x3f1") => r.int(4, true, "count")? / 0x3f1 - 1,
            _ => f.fixed.filter(|n| *n >= 0).ok_or("a sequence without a size")?,
        };
        usize::try_from(n).map_err(|_| format!("a count of {n}"))
    }

    fn read_type(&self, ty: &str, f: &Field, r: &mut Reader, depth: usize) -> Result<Value, String> {
        let ty = ty.trim();
        if let Some(inner) = ty.strip_suffix("[]") {
            let n = self.count(f, r)?;
            if inner == "byte" {
                return Ok(Value::Bytes(r.take(n, "bytes")?.to_vec()));
            }
            // A count larger than the data left is a misparse, not a reason to allocate.
            if n > r.data.len() - r.at + 8 {
                return Err(format!("an array of {n} elements with {} bytes left", r.data.len() - r.at));
            }
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(self.read_type(inner, &Field::default(), r, depth + 1)?);
            }
            return Ok(Value::List(items));
        }
        let t = ty.strip_suffix('?').unwrap_or(ty);
        match t {
            "bool" => Ok(Value::Bool(r.int(1, false, "bool")? != 0)),
            "byte" => Ok(Value::Int(r.int(1, false, "byte")?)),
            "short" => Ok(Value::Int(r.int(2, true, "short")?)),
            "ushort" => Ok(Value::Int(r.int(2, false, "ushort")?)),
            "int" => Ok(Value::Int(r.int(4, true, "int")?)),
            "uint" => Ok(Value::Int(r.int(4, false, "uint")?)),
            "long" => Ok(Value::Int(r.int(8, true, "long")?)),
            "float" => Ok(Value::Float(r.float()?)),
            "double" => Ok(Value::Float(f64::from_bits(r.int(8, false, "double")? as u64))),
            "string" => {
                let bytes = match f.size.as_deref() {
                    Some("nullterm") => {
                        let mut s = Vec::new();
                        loop {
                            let b = r.take(1, "string")?[0];
                            if b == 0 {
                                break;
                            }
                            s.push(b);
                        }
                        s
                    }
                    _ => {
                        let n = self.count(f, r)?;
                        r.take(n, "string")?.to_vec()
                    }
                };
                Ok(Value::Str(String::from_utf8_lossy(&bytes).trim_end_matches('\0').to_string()))
            }
            "Identity" => Ok(Value::Struct(vec![("Type".into(), Value::Int(r.int(4, true, "identity type")?)), ("Instance".into(), Value::Int(r.int(4, true, "identity instance")?))])),
            "Vector3" => Ok(Value::Struct(vec![("X".into(), Value::Float(r.float()?)), ("Y".into(), Value::Float(r.float()?)), ("Z".into(), Value::Float(r.float()?))])),
            "Quaternion" => Ok(Value::Struct(vec![("X".into(), Value::Float(r.float()?)), ("Y".into(), Value::Float(r.float()?)), ("Z".into(), Value::Float(r.float()?)), ("W".into(), Value::Float(r.float()?))])),
            "IPAddress" => Ok(Value::Str(r.take(4, "ip")?.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."))),
            _ if t.starts_with("GameTuple<") => {
                let inner = &t["GameTuple<".len()..t.len() - 1];
                let args = split_args(inner);
                let a = self.read_type(args[0], &Field::default(), r, depth + 1)?;
                let b = self.read_type(args.get(1).copied().unwrap_or("int"), &Field::default(), r, depth + 1)?;
                Ok(Value::Struct(vec![("Item1".into(), a), ("Item2".into(), b)]))
            }
            _ => {
                if let Some(e) = self.enum_of(t) {
                    let width = match e.under.as_str() {
                        "byte" | "sbyte" => 1,
                        "short" | "ushort" => 2,
                        "long" => 8,
                        _ => 4,
                    };
                    let v = r.int(width, true, t)?;
                    let name = e.values.iter().find(|(_, x)| **x == v).map(|(n, _)| n.clone()).unwrap_or_default();
                    return Ok(Value::Enum(v, name));
                }
                if self.class_of(t).is_some() {
                    return self.read_class(t, r, depth + 1);
                }
                // A type defined elsewhere in the sources (Stat, Profession ...): an int enum.
                Ok(Value::Int(r.int(4, true, t)?))
            }
        }
    }
}

impl Schema {
    /// Encodes an N3 message of `kind` from its fields by name: the inverse of [`Schema::parse_n3`], with the same layout and
    /// flags. `header` is the message's leading byte (the one the parser skips; the captures send 1 for most). A field the
    /// struct leaves out is written as zero (or empty), and a conditional field that its flag does not select is left out.
    pub fn encode_n3(&self, kind: u32, header: u8, value: &Value) -> Result<Vec<u8>, String> {
        let entry = self.raw.n3.get(&format!("{kind:08x}")).ok_or_else(|| format!("no definition for {kind:08x}"))?;
        let mut out = vec![header];
        self.write_class(&entry.class, value, &mut out, 0).map_err(|e| format!("{}: {e}", entry.name))?;
        Ok(out)
    }

    fn write_class(&self, name: &str, value: &Value, out: &mut Vec<u8>, depth: usize) -> Result<(), String> {
        if depth > 12 {
            return Err(format!("{name}: nested too deep"));
        }
        let class = self.class_of(name).ok_or_else(|| format!("unknown class {name}"))?;
        let mut chain: Vec<&Class> = vec![class];
        let mut base = class.base.as_deref();
        while let Some(b) = base {
            match self.class_of(b) {
                Some(c) if b != "N3Message" && b != "MessageBody" => {
                    chain.insert(0, c);
                    base = c.base.as_deref();
                }
                _ => break,
            }
        }
        let null = Value::Null;
        let mut flags: HashMap<String, i64> = HashMap::new();
        for class in chain {
            for f in &class.fields {
                let f = &sized(name, f).unwrap_or_else(|| f.clone());
                // (the same choice of type the reader makes: a conditional field is present only when its flag selects it)
                let ty = if f.when.is_empty() {
                    f.ty.as_str()
                } else {
                    match f.when.iter().find(|w| flags.get(&w.flag).is_some_and(|v| w.values.iter().any(|x| x.as_i64() == Some(*v)))) {
                        Some(w) => w.ty.as_str(),
                        None => continue,
                    }
                };
                let v = value.get(&f.name).unwrap_or(&null);
                self.write_type(ty, f, v, out, depth).map_err(|e| format!("{name}.{}: {e}", f.name))?;
                if let (Some(flag), Some(i)) = (&f.flag, v.as_int()) {
                    flags.insert(flag.clone(), i);
                }
            }
        }
        Ok(())
    }

    /// The count a sized field writes before its elements (`n`), or checks against its fixed size.
    fn write_count(&self, f: &Field, n: usize, out: &mut Vec<u8>) -> Result<(), String> {
        match f.size.as_deref() {
            Some("byte") => push_int(out, n as i64, 1),
            Some("int16") => push_int(out, n as i64, 2),
            Some("int32") => push_int(out, n as i64, 4),
            Some("x3f1") => push_int(out, (n as i64 + 1) * 0x3f1, 4),
            _ => match f.fixed {
                Some(k) if k >= 0 && k as usize != n => return Err(format!("{n} elements where {k} are fixed")),
                Some(_) => {}
                None => return Err("a sequence without a size".into()),
            },
        }
        Ok(())
    }

    /// Bytes of a string or a byte array: their count (when sized by one) and the bytes, or the fixed size padded with zeros.
    fn write_sized(&self, f: &Field, bytes: &[u8], out: &mut Vec<u8>) -> Result<(), String> {
        match f.size.as_deref() {
            Some("byte") | Some("int16") | Some("int32") | Some("x3f1") => {
                self.write_count(f, bytes.len(), out)?;
                out.extend_from_slice(bytes);
            }
            _ => {
                let n = f.fixed.filter(|n| *n >= 0).ok_or("a sequence without a size")? as usize;
                let mut b = bytes.to_vec();
                b.resize(n, 0);
                out.extend_from_slice(&b);
            }
        }
        Ok(())
    }

    fn write_type(&self, ty: &str, f: &Field, v: &Value, out: &mut Vec<u8>, depth: usize) -> Result<(), String> {
        let ty = ty.trim();
        if let Some(inner) = ty.strip_suffix("[]") {
            if inner == "byte" {
                let bytes: Vec<u8> = match v {
                    Value::Bytes(b) => b.clone(),
                    Value::List(items) => items.iter().map(|i| int_of(i) as u8).collect(),
                    _ => Vec::new(),
                };
                return self.write_sized(f, &bytes, out);
            }
            let items: &[Value] = match v {
                Value::List(items) => items,
                _ => &[],
            };
            self.write_count(f, items.len(), out)?;
            for item in items {
                self.write_type(inner, &Field::default(), item, out, depth + 1)?;
            }
            return Ok(());
        }
        let t = ty.strip_suffix('?').unwrap_or(ty);
        match t {
            "bool" | "byte" => push_int(out, int_of(v), 1),
            "short" | "ushort" => push_int(out, int_of(v), 2),
            "int" | "uint" => push_int(out, int_of(v), 4),
            "long" => push_int(out, int_of(v), 8),
            "float" => out.extend_from_slice(&(float_of(v) as f32).to_be_bytes()),
            "double" => out.extend_from_slice(&float_of(v).to_be_bytes()),
            "string" => {
                let s: &[u8] = match v {
                    Value::Str(s) => s.as_bytes(),
                    _ => &[],
                };
                match f.size.as_deref() {
                    Some("nullterm") => {
                        out.extend_from_slice(s);
                        out.push(0);
                    }
                    _ => self.write_sized(f, s, out)?,
                }
            }
            "Identity" => {
                push_int(out, int_of(v.get("Type").unwrap_or(&Value::Null)), 4);
                push_int(out, int_of(v.get("Instance").unwrap_or(&Value::Null)), 4);
            }
            "Vector3" | "Quaternion" => {
                let names: &[&str] = if t == "Vector3" { &["X", "Y", "Z"] } else { &["X", "Y", "Z", "W"] };
                for name in names {
                    out.extend_from_slice(&(float_of(v.get(name).unwrap_or(&Value::Null)) as f32).to_be_bytes());
                }
            }
            "IPAddress" => {
                let text = match v {
                    Value::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let mut octets = text.split('.').map(|p| p.parse::<u8>().unwrap_or(0));
                for _ in 0..4 {
                    out.push(octets.next().unwrap_or(0));
                }
            }
            _ if t.starts_with("GameTuple<") => {
                let inner = &t["GameTuple<".len()..t.len() - 1];
                let args = split_args(inner);
                self.write_type(args[0], &Field::default(), v.get("Item1").unwrap_or(&Value::Null), out, depth + 1)?;
                self.write_type(args.get(1).copied().unwrap_or("int"), &Field::default(), v.get("Item2").unwrap_or(&Value::Null), out, depth + 1)?;
            }
            _ => {
                if let Some(e) = self.enum_of(t) {
                    let width = match e.under.as_str() {
                        "byte" | "sbyte" => 1,
                        "short" | "ushort" => 2,
                        "long" => 8,
                        _ => 4,
                    };
                    push_int(out, int_of(v), width);
                    return Ok(());
                }
                if self.class_of(t).is_some() {
                    return self.write_class(t, v, out, depth + 1);
                }
                push_int(out, int_of(v), 4);
            }
        }
        Ok(())
    }
}

/// An integer's value (a bool is 0 or 1, anything else 0).
fn int_of(v: &Value) -> i64 {
    match v {
        Value::Float(f) => *f as i64,
        other => other.as_int().unwrap_or(0),
    }
}

fn float_of(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        other => other.as_int().unwrap_or(0) as f64,
    }
}

/// The low `width` bytes of `v`, big-endian (the two's complement for a negative value).
fn push_int(out: &mut Vec<u8>, v: i64, width: usize) {
    out.extend_from_slice(&v.to_be_bytes()[8 - width..]);
}

fn split_args(s: &str) -> Vec<&str> {
    let (mut out, mut depth, mut start) = (Vec::new(), 0, 0);
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(s[start..].trim());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn a_use_message_has_its_fields_by_name() {
        // The client's first use in the capture (after the kind and identity): action 3 = Use.
        let data = hex("01000000000000000100000003000000010000c3500004c2de0000c76a0101e30a");
        let parsed = Schema::global().parse_n3(0x52526858, &data).unwrap();
        assert_eq!(parsed.name, "GenericCmd");
        assert_eq!(parsed.left, 0);
        assert_eq!(parsed.value.get("Action"), Some(&Value::Enum(3, "Use".into())));
        let target = parsed.value.get("Target").unwrap();
        assert_eq!(target.get("Type").and_then(Value::as_int), Some(0xc76a));
    }

    #[test]
    fn a_truncated_message_is_an_error_not_a_panic() {
        let data = hex("000000000b000000000000000043e700004170006943cf000000000000bf3504b0000000003f350536");
        assert!(Schema::global().parse_n3(0x7f544905, &data).is_err());
    }

    #[test]
    fn a_message_encodes_back_to_its_bytes() {
        let data = hex("01000000000000000100000003000000010000c3500004c2de0000c76a0101e30a");
        let parsed = Schema::global().parse_n3(0x52526858, &data).unwrap();
        assert_eq!(Schema::global().encode_n3(0x52526858, data[0], &parsed.value).unwrap(), data);
    }

    /// The captured messages (`MAO_CAPTURES`: a folder of `dump.txt` files from `mao-client net msg-dump`) that parse whole come back
    /// to the same bytes. Prints how many did; skipped without the folder.
    #[test]
    fn captured_messages_encode_back_to_their_bytes() {
        let Some(dir) = std::env::var_os("MAO_CAPTURES") else { return };
        let (mut tried, mut same, mut differ, mut failed) = (0usize, 0usize, Vec::new(), 0usize);
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path().join("dump.txt")) else { continue };
            for line in text.lines() {
                // "#N ts S>C cX kind KKKKKKKK identity (a,b) LEN bytes: HEX"
                let Some(kind) = line.split(" kind ").nth(1).and_then(|r| u32::from_str_radix(r.split(' ').next()?, 16).ok()) else { continue };
                let Some(hex_text) = line.rsplit("bytes: ").next().filter(|_| line.contains("bytes: ")) else { continue };
                let data = hex(hex_text.trim());
                let Ok(parsed) = Schema::global().parse_n3(kind, &data) else { failed += 1; continue };
                if parsed.left != 0 {
                    failed += 1;
                    continue;
                }
                tried += 1;
                match Schema::global().encode_n3(kind, data[0], &parsed.value) {
                    Ok(bytes) if bytes == data => same += 1,
                    _ => {
                        if differ.len() < 8 {
                            differ.push(format!("{} {kind:08x}", parsed.name));
                        }
                    }
                }
            }
        }
        eprintln!("round trip: {same} of {tried} captured messages encode to the same bytes ({failed} do not parse whole); first differences: {differ:?}");
        assert!(tried == 0 || same * 10 >= tried * 9, "{same} of {tried}");
    }

    #[test]
    fn the_message_names_come_from_the_schema() {
        assert_eq!(Schema::global().n3_name(0x5e477770), Some("CharacterAction"));
        assert_eq!(Schema::global().enum_name("CharacterActionType", 0x78), Some("Logout"));
    }
}
