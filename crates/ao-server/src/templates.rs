//! The messages the server answers with, in the shapes the captures show. Each kind has one captured message (`data/templates.txt`,
//! made by `mao-net`'s `capture_templates` example): a server message is that template with the fields it changes set, encoded
//! again by the schema. Whatever the schema does not describe at the end of a template is kept as captured.

use std::collections::HashMap;
use std::sync::OnceLock;

use mao_net::n3::Message;
use mao_net::schema::{Schema, Value};

const TEMPLATES: &str = include_str!("../data/templates.txt");

/// One captured message, parsed.
#[derive(Clone, Debug)]
pub struct Template {
    /// The identity the capture was sent about (the template's own).
    pub identity: (u32, u32),
    /// The leading byte of the message (the parser skips it; it is kept).
    pub header: u8,
    /// The fields, as the schema reads them.
    pub value: Value,
    /// The bytes at the end the schema does not describe, as captured.
    pub tail: Vec<u8>,
}

/// The captured templates by kind: their identity and data.
fn table() -> &'static HashMap<u32, ((u32, u32), Vec<u8>)> {
    static TABLE: OnceLock<HashMap<u32, ((u32, u32), Vec<u8>)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut out = HashMap::new();
        for line in TEMPLATES.lines() {
            // "kind direction identity-type identity-instance hex"
            let parts: Vec<&str> = line.split_whitespace().collect();
            let [kind, _direction, a, b, data] = parts[..] else { continue };
            let (Ok(kind), Ok(a), Ok(b)) = (u32::from_str_radix(kind, 16), u32::from_str_radix(a, 16), u32::from_str_radix(b, 16)) else { continue };
            let bytes: Option<Vec<u8>> = (0..data.len() / 2).map(|i| u8::from_str_radix(&data[2 * i..2 * i + 2], 16).ok()).collect();
            if let Some(bytes) = bytes {
                out.insert(kind, ((a, b), bytes));
            }
        }
        out
    })
}

/// The captured message of a kind as it is: its identity and its data (for a layout the schema does not parse whole, which the
/// server changes at the places it knows: see `monsters`).
pub fn raw(kind: u32) -> Option<((u32, u32), Vec<u8>)> {
    table().get(&kind).cloned()
}

/// The captured template of a kind, parsed (none when there is none, or its layout does not parse whole).
pub fn of(kind: u32) -> Option<Template> {
    let (identity, data) = table().get(&kind)?;
    let parsed = Schema::global().parse_n3(kind, data).ok()?;
    let tail = data[data.len() - parsed.left..].to_vec();
    Some(Template { identity: *identity, header: data[0], value: parsed.value, tail })
}

impl Template {
    /// Sets a field of the template (a field the struct lacks is added at its end).
    pub fn set(&mut self, name: &str, value: Value) -> &mut Self {
        if let Value::Struct(fields) = &mut self.value {
            match fields.iter_mut().find(|(n, _)| n == name) {
                Some(field) => field.1 = value,
                None => fields.push((name.to_string(), value)),
            }
        }
        self
    }

    /// The field `name` as it is now.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.value.get(name)
    }

    /// The message of `kind` with this template's fields, about `identity`: encoded by the schema, with the captured tail.
    pub fn message(&self, kind: u32, identity: (u32, u32)) -> Result<Message, String> {
        let mut data = Schema::global().encode_n3(kind, self.header, &self.value)?;
        data.extend_from_slice(&self.tail);
        Ok(Message { kind, identity, data })
    }
}

/// A struct value from its fields.
pub fn record(fields: Vec<(&str, Value)>) -> Value {
    Value::Struct(fields.into_iter().map(|(n, v)| (n.to_string(), v)).collect())
}

/// An identity value (its type and instance).
pub fn identity(id: (u32, u32)) -> Value {
    record(vec![("Type", Value::Int(id.0 as i64)), ("Instance", Value::Int(id.1 as i64))])
}

/// An integer value.
pub fn int(v: i64) -> Value {
    Value::Int(v)
}

/// A list value.
pub fn list(items: Vec<Value>) -> Value {
    Value::List(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_parses_and_encodes_back() {
        for (&kind, _) in table() {
            // (the monster's entity message is changed as raw bytes: `monsters`; the schema does not parse it whole yet)
            if kind == 0x271b3a6b {
                continue;
            }
            let t = of(kind).unwrap_or_else(|| panic!("{kind:08x} does not parse"));
            let m = t.message(kind, t.identity).unwrap_or_else(|e| panic!("{kind:08x}: {e}"));
            let original = &table()[&kind].1;
            assert_eq!(&m.data, original, "{kind:08x} does not come back to its bytes");
        }
    }

    #[test]
    fn a_changed_field_is_encoded_and_read_back() {
        let mut t = of(0x2b333d6e).unwrap();
        t.set("Stats", list(vec![record(vec![("Item1", int(0x36)), ("Item2", int(7))])]));
        let m = t.message(0x2b333d6e, t.identity).unwrap();
        let back = Schema::global().parse_n3(0x2b333d6e, &m.data).unwrap();
        let first = back.value.get("Stats").and_then(|s| match s {
            Value::List(items) => items.first(),
            _ => None,
        });
        assert_eq!(first.and_then(|f| f.get("Item1")).and_then(Value::as_int), Some(0x36));
        assert_eq!(first.and_then(|f| f.get("Item2")).and_then(Value::as_int), Some(7));
    }
}
