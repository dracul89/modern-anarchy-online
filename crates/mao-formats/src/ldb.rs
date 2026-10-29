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

/// `FromBase85`: five digits (`!` + digit, most significant first); `None` at a byte outside `!`..`u`.
fn from_base85(b: &[u8]) -> Option<u32> {
    let digits = b.get(..5)?;
    digits.iter().try_fold(0u32, |v, &c| (c.wrapping_sub(0x21) <= 0x54).then(|| v.wrapping_mul(85).wrapping_add((c - 0x21) as u32)))
}

/// The UTF-8 character at the start of `b` and its length (the ldb drop-in's `utf8_decode` / `utf8_length`: 0x92 reads as `'`,
/// a byte that doesn't begin a well-formed sequence as itself).
fn utf8_char(b: &[u8]) -> (u32, usize) {
    let Some(&c) = b.first() else { return (0, 1) };
    let length = if c >= 0x80 && c & 0xc0 != 0xc0 { 1 } else { ((0xe500_0000u32 >> ((c >> 3) & 0x1e)) & 3) as usize + 1 };
    if c == 0x92 {
        return (0x27, length);
    }
    match std::str::from_utf8(b.get(..length).unwrap_or(b)).ok().and_then(|t| t.chars().next()) {
        Some(ch) if length > 1 => (ch as u32, length),
        _ => (c as u32, length),
    }
}

/// `RemoteFormat(text)` (LDB.dll; the drop-in's `remote::init`): a text the server packs as `~&`, the category and id as five
/// base-85 digits each, then the arguments (`i`/`u`/`f` a word in base 85, `R` another text's category and id, `s` a string and
/// `F` a nested packed text, both as their length + 1 as a UTF-8 character and the bytes) up to a closing `~`. The text is
/// looked up and filled with the arguments. A text not packed so comes back as it is.
pub fn remote(texts: &Texts, s: &str) -> String {
    let b = s.as_bytes();
    if !(b.len() > 11 && b.starts_with(b"~&")) {
        return s.to_string();
    }
    let (Some(category), Some(id)) = (from_base85(&b[2..]), from_base85(&b[7..])) else { return s.to_string() };
    let text = texts.get(category as i32, id as i32).unwrap_or("");
    let mut parts: Vec<String> = Vec::new();
    let mut numbers: Vec<Option<i64>> = Vec::new();
    let mut i = 12;
    while i < b.len() {
        let c = b[i];
        let at = &b[i + 1..];
        match c {
            b'F' | b's' => {
                let (n, skip) = utf8_char(at);
                if n == 0 {
                    break;
                }
                i += 1 + skip;
                let n = (n - 1) as usize;
                let part = String::from_utf8_lossy(&b[i..(i + n).min(b.len())]).into_owned();
                parts.push(remote(texts, &part));
                numbers.push(None);
                i += n;
            }
            b'R' => {
                let (Some(category), Some(id)) = (from_base85(at), from_base85(at.get(5..).unwrap_or(&[]))) else { break };
                parts.push(texts.get(category as i32, id as i32).unwrap_or("").to_string());
                numbers.push(None);
                i += 11;
            }
            b'f' | b'i' | b'u' => {
                let Some(value) = from_base85(at) else { break };
                match c {
                    b'f' => {
                        parts.push(format!("{}", f32::from_bits(value)));
                        numbers.push(None);
                    }
                    b'i' => {
                        parts.push(String::new());
                        numbers.push(Some(value as i32 as i64));
                    }
                    _ => {
                        parts.push(String::new());
                        numbers.push(Some(value as i64));
                    }
                }
                i += 6;
            }
            _ => break,
        }
    }
    let args: Vec<Arg> = parts.iter().zip(&numbers).map(|(t, n)| n.map_or(Arg::Text(t), Arg::Int)).collect();
    format(text, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_packed_text_is_looked_up_and_filled() {
        // (the server's answer to a clan cell scanner used by someone else: 110/55195541 with the text 2005/1)
        let mut texts = Texts::default();
        texts.categories.insert(110, HashMap::from([(55195541, "Target side must be %s!".to_string())]));
        texts.categories.insert(2005, HashMap::from([(1, "clan".to_string())]));
        assert_eq!(remote(&texts, "~&!!!\":\"%kM;R!!!8S!!!!\""), "Target side must be clan!");
        // (a string and a word, then the closing `~`)
        texts.categories.insert(1, HashMap::from([(2, "%s has %d".to_string())]));
        assert_eq!(remote(&texts, "~&!!!!\"!!!!#s\u{4}Bobi!!!!#~"), "Bob has 2");
        assert_eq!(remote(&texts, "plain"), "plain");
    }

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
