//! The game's text database (`cd_image/text/text.mdb`, read by `LDBface`: the `ldb` drop-in's `face.rs`) and its formatting
//! (`LDBformat`, the drop-in's `format.rs`).
//!
//! The file is "MMDB", the number of categories at +4, then a table of `{category, offset}` pairs (i32 LE) sorted by category.
//! Each category's offset points at its own table of `{key, offset}` pairs, as many as fit before the next category's table;
//! each text is a C string. A text is asked for by category and key (`LDBface::GetText(category, key)`); the game's code names
//! its keys and hashes the name with ElfHash (FUN_10038292 → `LDBface::GetText(110, ElfHash(name))` for the feedback lines).

use std::collections::HashMap;

/// The category of the client's feedback lines (FUN_10012c3e asks `GetText(0x6e, ...)`).
pub const FEEDBACK: i32 = 110;

/// The texts of a `text.mdb`, by category and key.
#[derive(Debug, Default)]
pub struct Texts {
    categories: HashMap<i32, HashMap<i32, String>>,
}

impl Texts {
    pub fn parse(data: &[u8]) -> Option<Texts> {
        let word = |at: usize| Some(i32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
        if data.get(..4)? != b"MMDB" {
            return None;
        }
        let count = word(4)?.max(0) as usize;
        let table: Vec<(i32, usize)> = (0..count).map(|i| Some((word(8 + 8 * i)?, word(12 + 8 * i)? as usize))).collect::<Option<_>>()?;
        let mut categories = HashMap::new();
        for (i, &(category, offset)) in table.iter().enumerate() {
            // (the last category's table runs to its first text)
            let end = match table.get(i + 1) {
                Some(&(_, next)) => next,
                None => word(offset + 4).map_or(offset, |first| first as usize),
            };
            let mut texts = HashMap::new();
            for j in 0..end.saturating_sub(offset) / 8 {
                let (Some(key), Some(at)) = (word(offset + 8 * j), word(offset + 8 * j + 4)) else { break };
                let Some(rest) = data.get(at as usize..) else { continue };
                let text = &rest[..rest.iter().position(|&b| b == 0).unwrap_or(rest.len())];
                texts.insert(key, text.iter().map(|&b| b as char).collect());
            }
            categories.insert(category, texts);
        }
        Some(Texts { categories })
    }

    /// A text by category and key.
    pub fn get(&self, category: i32, key: i32) -> Option<&str> {
        self.categories.get(&category)?.get(&key).map(String::as_str)
    }

    /// A text by category and the name its key is hashed from.
    pub fn named(&self, category: i32, name: &str) -> Option<&str> {
        self.get(category, elf_hash(name) as i32)
    }
}

/// The key of a text's name (the PJW/ELF hash the game's `ElfHash` computes).
pub fn elf_hash(name: &str) -> u32 {
    let mut h: u32 = 0;
    for &c in name.as_bytes() {
        h = (h << 4).wrapping_add(c as u32);
        let g = h & 0xf000_0000;
        if g != 0 {
            h ^= g >> 24;
        }
        h &= !g;
    }
    h
}

/// A value fed into a text.
#[derive(Clone, Debug)]
pub enum Arg<'a> {
    Text(&'a str),
    Int(i64),
}

/// A text with its placeholders filled in order, as `LDBformat::Feed` fills the next one each time: `%s` a text, `%d`/`%i`/`%u`
/// a number (either kind of value prints in either), `%%` a percent sign. Placeholders with nothing fed print as they are.
pub fn format(text: &str, args: &[Arg]) -> String {
    let mut out = String::new();
    let mut args = args.iter();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // (the flags and width the drop-in's `init` skips over before the conversion)
        let mut spec = String::from("%");
        while let Some(&f) = chars.peek() {
            if f.is_ascii_digit() || "-+ #.l".contains(f) {
                spec.push(f);
                chars.next();
            } else {
                break;
            }
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some(k @ ('s' | 'd' | 'i' | 'u')) => match args.next() {
                Some(Arg::Text(t)) => out.push_str(t),
                Some(Arg::Int(n)) if k == 'u' => out.push_str(&(*n as u32).to_string()),
                Some(Arg::Int(n)) => out.push_str(&n.to_string()),
                None => {
                    out.push_str(&spec);
                    out.push(k);
                }
            },
            Some(k) => {
                out.push_str(&spec);
                out.push(k);
            }
            None => out.push_str(&spec),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elf_hash_matches_the_games() {
        // (ElfHash("a") is 'a'; long names fold the top nibble back in)
        assert_eq!(elf_hash("a"), 0x61);
        assert_eq!(elf_hash("ab"), 0x672);
        assert_eq!(elf_hash("Feedback_HitWithSpecial") & 0xf000_0000, 0);
    }

    #[test]
    fn fills_in_order() {
        let s = format("You hit %s for %u points of %s damage.", &[Arg::Text("Rollerrat"), Arg::Int(49), Arg::Text("energy")]);
        assert_eq!(s, "You hit Rollerrat for 49 points of energy damage.");
        assert_eq!(format("100%% of %d", &[Arg::Int(-3)]), "100% of -3");
        assert_eq!(format("%s and %s", &[Arg::Text("one")]), "one and %s");
    }

    #[test]
    fn parses_a_small_database() {
        // (two categories; the second's table runs to its first text)
        let mut d = b"MMDB".to_vec();
        d.extend(2i32.to_le_bytes());
        d.extend(7i32.to_le_bytes());
        d.extend(24i32.to_le_bytes());
        d.extend(110i32.to_le_bytes());
        d.extend(32i32.to_le_bytes());
        d.extend(5i32.to_le_bytes());
        d.extend(40i32.to_le_bytes());
        d.extend((elf_hash("Name") as i32).to_le_bytes());
        d.extend(44i32.to_le_bytes());
        d.extend(b"one\0two\0");
        let t = Texts::parse(&d).unwrap();
        assert_eq!(t.get(7, 5), Some("one"));
        assert_eq!(t.named(110, "Name"), Some("two"));
        assert_eq!(t.get(110, 5), None);
    }
}
