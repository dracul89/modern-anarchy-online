//! Utils' `Variant` for the GUI's expressions and distributed values: the scalar types and strings, with the original's
//! conversions, arithmetic and comparisons (after `ao_utils::variant` of the AnarchyReimagined drop-ins, which ports
//! `src/utils/Variant.cpp`). The geometry, identity, raw and message types are not needed here. The process runs the
//! x87 at 53 bits, so float and double arithmetic is plain `f64` arithmetic.
//!
//! The rules are the original's and not those of a well-behaved type (void < void is true, `|` ANDs), so they are
//! methods and operators on references rather than `PartialEq`/`PartialOrd`.

use std::ops::{Add, BitAnd, BitOr, Div, Mul, Neg, Shl, Shr, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeCode {
    Void,
    Int32,
    Int64,
    Bool,
    Float,
    Double,
    String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum Variant {
    #[default]
    Void,
    Int32(i32),
    Int64(i64),
    Bool(bool),
    Float(f32),
    Double(f64),
    String(Vec<u8>),
}

use Variant as V;

impl Variant {
    pub const fn new() -> Variant {
        V::Void
    }

    pub fn type_code(&self) -> TypeCode {
        match self {
            V::Void => TypeCode::Void,
            V::Int32(_) => TypeCode::Int32,
            V::Int64(_) => TypeCode::Int64,
            V::Bool(_) => TypeCode::Bool,
            V::Float(_) => TypeCode::Float,
            V::Double(_) => TypeCode::Double,
            V::String(_) => TypeCode::String,
        }
    }

    pub fn set_int32(&mut self, v: i32) {
        *self = V::Int32(v);
    }
    pub fn set_float(&mut self, v: f32) {
        *self = V::Float(v);
    }
    pub fn set_bool(&mut self, v: bool) {
        *self = V::Bool(v);
    }
    pub fn set_string(&mut self, v: &[u8]) {
        *self = V::String(c_text(v).to_vec());
    }

    pub fn as_int32(&self) -> i32 {
        match self {
            V::Int32(v) => *v,
            V::Int64(v) => *v as i32,
            V::Bool(b) => *b as i32,
            V::Float(f) => *f as i32,
            V::Double(d) => *d as i32,
            V::String(s) => scan_i64(s).unwrap_or(0) as i32,
            V::Void => 0,
        }
    }

    pub fn as_int64(&self) -> i64 {
        match self {
            V::Int64(v) => *v,
            V::Float(f) => *f as i64,
            V::Double(d) => *d as i64,
            V::String(s) => scan_i64(s).unwrap_or(0),
            other => i64::from(other.as_int32()),
        }
    }

    pub fn as_float(&self) -> f32 {
        self.as_double() as f32
    }

    pub fn as_double(&self) -> f64 {
        match self {
            V::Int32(v) => f64::from(*v),
            V::Int64(v) => *v as f64,
            V::Bool(b) => f64::from(u8::from(*b)),
            V::Float(f) => f64::from(*f),
            V::Double(d) => *d,
            V::String(s) => std::str::from_utf8(s).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0.0),
            V::Void => 0.0,
        }
    }

    /// Strings: "false"/"true" in any case, else a nonzero `%d`.
    pub fn as_bool(&self) -> bool {
        match self {
            V::Int32(v) => *v != 0,
            V::Int64(v) => *v != 0,
            V::Bool(b) => *b,
            V::Float(f) => *f != 0.0,
            V::Double(d) => *d != 0.0,
            V::String(s) => {
                if s.eq_ignore_ascii_case(b"false") {
                    false
                } else if s.eq_ignore_ascii_case(b"true") {
                    true
                } else {
                    scan_i64(s).is_some_and(|v| v as i32 != 0)
                }
            }
            V::Void => false,
        }
    }

    fn bool_byte(&self) -> u8 {
        u8::from(self.as_bool())
    }

    pub fn as_string(&self) -> Vec<u8> {
        match self {
            V::Int32(v) => v.to_string().into_bytes(),
            V::Int64(v) => (*v as i32).to_string().into_bytes(),
            V::Bool(b) => if *b { b"true".to_vec() } else { b"false".to_vec() },
            V::Float(f) => format!("{:.6}", f64::from(*f)).into_bytes(),
            V::Double(d) => format!("{d:.6}").into_bytes(),
            V::String(s) => s.clone(),
            V::Void => Vec::new(),
        }
    }

    /// `operator==`: numbers by value across types, bools against a number's truth, strings only with strings.
    pub fn equals(&self, other: &Variant) -> bool {
        match (self, other) {
            (V::Void, o) => matches!(o, V::Void),
            (V::String(a), V::String(b)) => a == b,
            (V::String(_), _) | (_, V::String(_)) | (_, V::Void) => false,
            (V::Bool(a), o) | (o, V::Bool(a)) => *a == o.as_bool(),
            (V::Int32(a), V::Int32(b)) => a == b,
            (V::Int32(_) | V::Int64(_), V::Int32(_) | V::Int64(_)) => self.as_int64() == other.as_int64(),
            _ => self.as_double() == other.as_double(),
        }
    }

    pub fn not_equals(&self, other: &Variant) -> bool {
        !self.equals(other)
    }

    /// The ordering of `self` against `other` converted to `self`'s type; `None` where the type has none.
    fn order(&self, other: &Variant) -> Option<std::cmp::Ordering> {
        match self {
            V::Int32(v) => Some(v.cmp(&other.as_int32())),
            V::Int64(v) => Some(v.cmp(&other.as_int64())),
            V::Bool(_) => Some(self.bool_byte().cmp(&other.bool_byte())),
            V::Float(_) | V::Double(_) => self.as_double().partial_cmp(&other.as_double()),
            V::String(s) => Some(s.as_slice().cmp(other.as_string().as_slice())),
            V::Void => None,
        }
    }

    /// `operator<` (void < void is true; void is > and >= anything else).
    pub fn less(&self, other: &Variant) -> bool {
        match self {
            V::Void => matches!(other, V::Void),
            _ => self.order(other) == Some(std::cmp::Ordering::Less),
        }
    }
    pub fn less_equal(&self, other: &Variant) -> bool {
        match self {
            V::Void => matches!(other, V::Void),
            _ => matches!(self.order(other), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)),
        }
    }
    pub fn greater(&self, other: &Variant) -> bool {
        match self {
            V::Void => !matches!(other, V::Void),
            _ => self.order(other) == Some(std::cmp::Ordering::Greater),
        }
    }
    pub fn greater_equal(&self, other: &Variant) -> bool {
        match self {
            V::Void => !matches!(other, V::Void),
            _ => matches!(self.order(other), Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)),
        }
    }

    /// `LoadFromString` for the forms the prefs files hold: "true"/"false", a number (with a '.' a float, else int32
    /// when it fits, else int64), "\"text\"" or "void". False, unchanged, for anything else.
    pub fn load_from_string(&mut self, text: &[u8]) -> bool {
        let text = c_text(text);
        if text.eq_ignore_ascii_case(b"false") {
            self.set_bool(false);
            return true;
        }
        if text.eq_ignore_ascii_case(b"true") {
            self.set_bool(true);
            return true;
        }
        let digits = text.strip_prefix(b"-").unwrap_or(text);
        let dots = digits.iter().filter(|&&c| c == b'.').count();
        if dots <= 1 && digits.iter().all(|&c| c == b'.' || c.is_ascii_digit()) {
            if dots == 1 && !digits.is_empty() {
                self.set_float(std::str::from_utf8(text).ok().and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0) as f32);
                return true;
            }
            let value = scan_i64(text).unwrap_or(0);
            *self = match i32::try_from(value) {
                Ok(v) => V::Int32(v),
                Err(_) => V::Int64(value),
            };
            return true;
        }
        if text.first() == Some(&b'"') {
            let inner = if text.len() < 3 { &[][..] } else { &text[1..text.len() - 1] };
            self.set_string(inner);
            return true;
        }
        if text.eq_ignore_ascii_case(b"void") {
            *self = V::Void;
            return true;
        }
        false
    }
}

/// The text up to its first NUL.
fn c_text(text: &[u8]) -> &[u8] {
    text.split(|&c| c == 0).next().unwrap_or(&[])
}

/// `%d`/`%I64d`: leading spaces, a sign and digits; `None` when there are no digits.
fn scan_i64(text: &[u8]) -> Option<i64> {
    let t = std::str::from_utf8(c_text(text)).ok()?.trim_start();
    let (neg, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let v = digits.bytes().fold(0i64, |v, d| v.wrapping_mul(10).wrapping_add(i64::from(d - b'0')));
    Some(if neg { v.wrapping_neg() } else { v })
}

macro_rules! from {
    ($t:ty, $v:ident, $e:expr) => {
        impl From<$t> for Variant {
            fn from($v: $t) -> Variant {
                $e
            }
        }
    };
}
from!(i32, v, V::Int32(v));
from!(i64, v, V::Int64(v));
from!(bool, v, V::Bool(v));
from!(f32, v, V::Float(v));
from!(f64, v, V::Double(v));
from!(&[u8], v, V::String(c_text(v).to_vec()));
from!(&str, v, V::String(c_text(v.as_bytes()).to_vec()));

/// Arithmetic is in the left side's type (bool gives int32); division by zero gives 0; strings join with `+`.
impl Neg for &Variant {
    type Output = Variant;
    fn neg(self) -> Variant {
        match self {
            V::Int32(v) => V::Int32(v.wrapping_neg()),
            V::Int64(v) => V::Int64(v.wrapping_neg()),
            V::Bool(b) => V::Bool(!b),
            V::Float(f) => V::Float(-f),
            V::Double(d) => V::Double(-d),
            _ => V::Void,
        }
    }
}

impl Add for &Variant {
    type Output = Variant;
    fn add(self, o: &Variant) -> Variant {
        match self {
            V::Void => V::Bool(matches!(o, V::Void)),
            V::Int32(v) => V::Int32(o.as_int32().wrapping_add(*v)),
            V::Int64(v) => V::Int64(o.as_int64().wrapping_add(*v)),
            V::Bool(_) => V::Int32(i32::from(self.bool_byte()) + i32::from(o.bool_byte())),
            V::Float(f) => V::Float((f64::from(*f) + f64::from(o.as_float())) as f32),
            V::Double(d) => V::Double(d + o.as_double()),
            V::String(s) => V::String([s.clone(), o.as_string()].concat()),
        }
    }
}

impl Sub for &Variant {
    type Output = Variant;
    fn sub(self, o: &Variant) -> Variant {
        match self {
            V::Void => V::Bool(matches!(o, V::Void)),
            V::Int32(v) => V::Int32(v.wrapping_sub(o.as_int32())),
            V::Int64(v) => V::Int64(v.wrapping_sub(o.as_int64())),
            V::Bool(_) => V::Int32(i32::from(self.bool_byte()) - i32::from(o.bool_byte())),
            V::Float(f) => V::Float((f64::from(*f) - o.as_double()) as f32),
            V::Double(d) => V::Double(d - o.as_double()),
            V::String(_) => V::Void,
        }
    }
}

impl Mul for &Variant {
    type Output = Variant;
    fn mul(self, o: &Variant) -> Variant {
        match self {
            V::Int32(v) => V::Int32(o.as_int32().wrapping_mul(*v)),
            V::Int64(v) => V::Int64(o.as_int64().wrapping_mul(*v)),
            V::Bool(_) => V::Int32(i32::from(self.bool_byte()) * i32::from(o.bool_byte())),
            V::Float(f) => V::Float((f64::from(*f) * f64::from(o.as_float())) as f32),
            V::Double(d) => V::Double(d * o.as_double()),
            _ => V::Void,
        }
    }
}

impl Div for &Variant {
    type Output = Variant;
    fn div(self, o: &Variant) -> Variant {
        match self {
            V::Int32(v) => V::Int32(if o.as_int32() == 0 { 0 } else { v.wrapping_div(o.as_int32()) }),
            V::Int64(v) => V::Int64(if o.as_int64() == 0 { 0 } else { v.wrapping_div(o.as_int64()) }),
            V::Bool(_) => V::Bool(false),
            V::Float(f) => V::Float(if o.as_double() == 0.0 { 0.0 } else { (f64::from(*f) / o.as_double()) as f32 }),
            V::Double(d) => V::Double(if o.as_double() == 0.0 { 0.0 } else { d / o.as_double() }),
            _ => V::Void,
        }
    }
}

/// 64-bit when either side is int64, else 32-bit.
impl BitAnd for &Variant {
    type Output = Variant;
    fn bitand(self, o: &Variant) -> Variant {
        if matches!(self, V::Int64(_)) || matches!(o, V::Int64(_)) { V::Int64(o.as_int64() & self.as_int64()) } else { V::Int32(o.as_int32() & self.as_int32()) }
    }
}

/// The original's `|` is a copy of `&` that was never changed.
impl BitOr for &Variant {
    type Output = Variant;
    #[allow(clippy::suspicious_arithmetic_impl, reason = "the original's quirk")]
    fn bitor(self, o: &Variant) -> Variant {
        self & o
    }
}

impl Shl for &Variant {
    type Output = Variant;
    fn shl(self, o: &Variant) -> Variant {
        let count = o.as_int32() as u32;
        match self {
            V::Int64(v) => V::Int64(if count & 0xff >= 64 { 0 } else { ((*v as u64) << (count & 0xff)) as i64 }),
            _ => V::Int32(((self.as_int32() as u32) << (count & 0x1f)) as i32),
        }
    }
}

impl Shr for &Variant {
    type Output = Variant;
    fn shr(self, o: &Variant) -> Variant {
        let count = o.as_int32() as u32;
        match self {
            V::Int64(v) => V::Int64(if count & 0xff >= 64 { v >> 63 } else { v >> (count & 0xff) }),
            _ => V::Int32(self.as_int32() >> (count & 0x1f)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_prefs_values() {
        let mut v = Variant::new();
        assert!(v.load_from_string(b"true") && v == Variant::Bool(true));
        assert!(v.load_from_string(b"0.5") && v.type_code() == TypeCode::Float);
        assert!(v.load_from_string(b"\"Verdana\"") && v.as_string() == b"Verdana");
        assert!(v.load_from_string(b"-12") && v.as_int32() == -12);
        assert!(!v.load_from_string(b"Rect(1,2,3,4)"));
        assert!(Variant::from("true").as_bool());
        assert!(Variant::Bool(true).equals(&Variant::Int32(5)));
    }
}
