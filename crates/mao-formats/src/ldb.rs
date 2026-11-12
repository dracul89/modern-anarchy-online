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

/// The pieces `LDBformat::Init` cuts a text into (the ldb drop-in's `format.rs`), each with the number of the argument that
/// fills it (0: plain text). `%` and the characters after it up to the conversion are argument `next`, counted from 1
/// (`%%` is a percent sign). `#N{...}` is a choice made by argument `N` (a digit); `#N:` makes the next `%` argument `N`; any
/// other `#N` stays as text.
fn tokens(text: &str) -> Vec<(u32, String)> {
    let c: Vec<char> = text.chars().collect();
    let at = |i: usize| c.get(i).copied().unwrap_or('\0');
    let mut out: Vec<(u32, String)> = Vec::new();
    let mut token = |from: usize, to: usize, index: u32| {
        // (`[from, to]`, stepped back off the end of the text; nothing when it is empty)
        let to = to.min(c.len().saturating_sub(1));
        if from <= to && from < c.len() {
            out.push((index, c[from..=to].iter().collect()));
        }
    };
    let (mut start, mut p, mut next) = (0usize, 0usize, 1u32);
    while p < c.len() {
        if c[p] == '%' {
            if p != 0 {
                if let Some(to) = p.checked_sub(1) {
                    token(start, to, 0);
                }
            }
            let mut q = p + 1;
            if at(q) == '%' {
                token(q, q, 0);
                p = q + 1;
                start = p;
            } else {
                while q < c.len() && "-+ 0#123456789.hlL".contains(c[q]) {
                    q += 1;
                }
                token(p, q, next);
                p = q + 1;
                start = p;
                next += 1;
            }
        } else if c[p] == '#' {
            if let Some(to) = p.checked_sub(1) {
                token(start, to, 0);
            }
            start = p + 1;
            let digit = at(start);
            if digit == '#' {
                p = start;
                continue;
            }
            let index = (digit as u32).wrapping_sub('0' as u32);
            start = p;
            if digit == '\0' {
                p += 1;
                break;
            }
            let a = p + 2;
            match at(a) {
                '\0' => {
                    p = a;
                    break;
                }
                '{' => {
                    let mut q = a;
                    while q < c.len() && c[q] != '}' {
                        q += 1;
                    }
                    token(a, q, index);
                    p = q + 1;
                    start = p;
                }
                ':' => {
                    p = a + 1;
                    start = p;
                    next = index;
                }
                _ => p = a,
            }
        } else {
            p += 1;
        }
    }
    token(start, p, 0);
    out
}

/// `LDBformat::FeedInternal`: of a choice `{flags:text|flags:text...}` the text of the alternative with the most of
/// `flags` (the last of equals). An alternative's flags are the characters up to its `:` (a letter is a bit from 0, the
/// digits 0 to 5 the bits from 26: `MakeFlagBit`); one that begins with a space has none and its text begins at the
/// space. A choice that does not parse stays as it is.
fn choose(token: &str, flags: u32) -> Option<String> {
    let c: Vec<char> = token.chars().collect();
    if c.first() != Some(&'{') {
        return None;
    }
    let bit = |ch: char| -> u32 {
        let u = ch.to_ascii_uppercase() as i32;
        let index = u + if u < 0x41 { -0x16 } else { -0x41 };
        if (0..32).contains(&index) { 1 << index } else { 0 }
    };
    let mut choices: Vec<(u32, usize, usize)> = Vec::new();
    let mut p = 0usize;
    loop {
        p += 1;
        if p >= c.len() {
            break;
        }
        let mut mask = 0;
        while p < c.len() && c[p] != ':' && c[p] != ' ' {
            mask |= bit(c[p]);
            p += 1;
        }
        if p >= c.len() {
            return None;
        }
        if c[p] != ' ' {
            p += 1;
        }
        let start = p;
        if p >= c.len() {
            return None;
        }
        while p < c.len() && c[p] != '|' && c[p] != '}' {
            p += 1;
        }
        if p >= c.len() {
            return None;
        }
        choices.push((mask, start, p));
    }
    let mut best = None;
    let mut most = 0;
    for &(mask, start, end) in &choices {
        let count = (mask & flags).count_ones();
        if best.is_none() || most <= count {
            most = count;
            best = Some((start, end));
        }
    }
    best.map(|(start, end)| c[start..end].iter().collect())
}

/// A text with its arguments fed in order, as `LDBformat::Init` and `Feed` do it: argument `n` fills every piece that is
/// its own: a `%s` with a text, a `%d`/`%i`/`%u` with a number (either kind of value prints in either), and a choice
/// `#n{1:one | many}` with the alternative its value picks (a number 0 has the flag `0`, 1 the flag `1`, any other value
/// none). `%%` is a percent sign. Pieces with nothing fed print as they are.
pub fn format(text: &str, args: &[Arg]) -> String {
    let mut tokens = tokens(text);
    for (n, arg) in args.iter().enumerate() {
        let n = n as u32 + 1;
        for (index, piece) in tokens.iter_mut().filter(|t| t.0 == n) {
            if piece.starts_with('%') {
                let filled = match (piece.chars().last(), arg) {
                    (Some('s' | 'd' | 'i' | 'u'), Arg::Text(t)) => Some(t.to_string()),
                    (Some('u'), Arg::Int(v)) => Some((*v as u32).to_string()),
                    (Some('s' | 'd' | 'i'), Arg::Int(v)) => Some(v.to_string()),
                    _ => None,
                };
                if let Some(filled) = filled {
                    *piece = filled;
                    *index = 0;
                }
            } else {
                let flags = match arg {
                    Arg::Int(0) => 0x400_0000,
                    Arg::Int(1) => 0x800_0000,
                    _ => 0,
                };
                if let Some(chosen) = choose(piece, flags) {
                    *piece = chosen;
                }
            }
        }
    }
    tokens.into_iter().map(|t| t.1).collect()
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
    remote_at(texts, s.as_bytes()).0
}

/// `RemoteFormat::ParseString` (LDB.dll; the drop-in's `remote::parse_string`): a text with packed pieces in it, each
/// complete one (closed by its `~`) replaced by its text and everything else kept. The answers to the organization's
/// commands come so: a packed line with plain text after it.
pub fn parse_string(texts: &Texts, s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"~&") {
            let (text, consumed) = remote_at(texts, &b[i..]);
            if consumed != 0 {
                out.extend_from_slice(text.as_bytes());
                i += consumed;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// [`remote`] on the bytes at the start of `b`: the text, and how many bytes the packed piece took when it was closed
/// by a `~` (0 when it was not).
fn remote_at(texts: &Texts, b: &[u8]) -> (String, usize) {
    let whole = || String::from_utf8_lossy(b).into_owned();
    if !(b.len() > 11 && b.starts_with(b"~&")) {
        return (whole(), 0);
    }
    let (Some(category), Some(id)) = (from_base85(&b[2..]), from_base85(&b[7..])) else { return (whole(), 0) };
    let mut consumed = 0;
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
            b'~' => {
                consumed = i + 1;
                break;
            }
            _ => break,
        }
    }
    let args: Vec<Arg> = parts.iter().zip(&numbers).map(|(t, n)| n.map_or(Arg::Text(t), Arg::Int)).collect();
    (format(text, &args), consumed)
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
        // (a choice by the second argument, as the texts for a reward have it: 1 picks the flagged alternative, another
        // number the one without flags; the spaces about the bar belong to the alternatives, as in the original)
        let reward = "%d points, %d #2{1:coin | coins}.";
        assert_eq!(format(reward, &[Arg::Int(5), Arg::Int(1)]), "5 points, 1 coin .");
        assert_eq!(format(reward, &[Arg::Int(5), Arg::Int(40)]), "5 points, 40  coins.");
        assert_eq!(format(reward, &[Arg::Int(5)]), "5 points, %d {1:coin | coins}.");
        // (`#2:` makes the next `%` the second argument; a `#` that is neither stays)
        assert_eq!(format("#2:%s then #1:%s", &[Arg::Text("a"), Arg::Text("b")]), "b then a");
        assert_eq!(format("item #7 of %d", &[Arg::Int(9)]), "item #7 of 9");
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
