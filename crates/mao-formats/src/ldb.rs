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
    /// `Feed(int)`: in a `%s` piece it prints as `int_value<n>`.
    Int(i64),
    /// `Feed(unsigned int)`: in a `%s` piece it prints as `uint_value<n>`.
    Uint(u32),
    /// `FeedFloat`: printed by the piece's own conversion (`%f`, `%.1f`, `%e`, `%g` ...).
    Float(f32),
    /// `FeedTime` (`FormatTime`): days, hours, minutes and seconds, printed as " 1 day 2 hours ..." whatever the piece's
    /// conversion, each part only when it is not 0.
    Time(i32, i32, i32, i32),
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

/// A `printf` conversion as the pieces hold it (`%-08.3lx`): its flags, width, precision, size and conversion letter.
/// `None` when the piece is not one (its letters out of order, or no letter at its end).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Spec {
    left: bool,
    plus: bool,
    space: bool,
    zero: bool,
    alternate: bool,
    width: usize,
    precision: Option<usize>,
    short: bool,
    conversion: char,
}

fn spec(piece: &str) -> Option<Spec> {
    let c: Vec<char> = piece.chars().collect();
    let mut s = Spec::default();
    let mut p = 1;
    while let Some(&ch) = c.get(p) {
        match ch {
            '-' => s.left = true,
            '+' => s.plus = true,
            ' ' => s.space = true,
            '0' => s.zero = true,
            '#' => s.alternate = true,
            _ => break,
        }
        p += 1;
    }
    while let Some(d) = c.get(p).and_then(|ch| ch.to_digit(10)) {
        s.width = s.width.saturating_mul(10).saturating_add(d as usize);
        p += 1;
    }
    if c.get(p) == Some(&'.') {
        p += 1;
        let mut precision = 0usize;
        while let Some(d) = c.get(p).and_then(|ch| ch.to_digit(10)) {
            precision = precision.saturating_mul(10).saturating_add(d as usize);
            p += 1;
        }
        s.precision = Some(precision);
    }
    while let Some(&ch) = c.get(p).filter(|ch| matches!(ch, 'h' | 'l' | 'L')) {
        s.short |= ch == 'h';
        p += 1;
    }
    s.conversion = *c.get(p)?;
    (p + 1 == c.len()).then_some(s)
}

/// `body` (digits, or a float's text) with its sign or prefix put in the conversion's width, as msvcr100's `_output`
/// does: spaces before, or zeros after the sign (`0`, not with `-` nor with an integer's precision), or spaces after (`-`).
fn pad(s: &Spec, sign: &str, body: &str, zeros_allowed: bool) -> String {
    let length = sign.chars().count() + body.chars().count();
    let fill = s.width.saturating_sub(length);
    if s.left {
        format!("{sign}{body}{}", " ".repeat(fill))
    } else if s.zero && zeros_allowed {
        format!("{sign}{}{body}", "0".repeat(fill))
    } else {
        format!("{}{sign}{body}", " ".repeat(fill))
    }
}

/// An integer conversion (`d i u o x X c`) of a 32-bit value.
fn print_int(s: &Spec, value: u32) -> Option<String> {
    let c = s.conversion;
    if c == 'c' {
        return Some(pad(s, "", &((value as u8) as char).to_string(), true));
    }
    let signed = matches!(c, 'd' | 'i');
    let (negative, magnitude): (bool, u64) = if signed {
        let v = if s.short { value as u16 as i16 as i64 } else { value as i32 as i64 };
        (v < 0, v.unsigned_abs())
    } else {
        (false, if s.short { value as u16 as u64 } else { value as u64 })
    };
    let mut digits = match c {
        'd' | 'i' | 'u' => magnitude.to_string(),
        'o' => format!("{magnitude:o}"),
        'x' => format!("{magnitude:x}"),
        'X' => format!("{magnitude:X}"),
        _ => return None,
    };
    if let Some(precision) = s.precision {
        if precision == 0 && magnitude == 0 {
            digits.clear();
        }
        if digits.len() < precision {
            digits = format!("{}{digits}", "0".repeat(precision - digits.len()));
        }
    }
    let sign = if signed && negative {
        "-"
    } else if signed && s.plus {
        "+"
    } else if signed && s.space {
        " "
    } else if s.alternate && magnitude != 0 && c == 'x' {
        "0x"
    } else if s.alternate && magnitude != 0 && c == 'X' {
        "0X"
    } else {
        ""
    };
    if s.alternate && c == 'o' && !digits.starts_with('0') {
        digits.insert(0, '0');
    }
    Some(pad(s, sign, &digits, s.precision.is_none()))
}

/// The exponent as msvcr100 writes it: a sign and at least three digits.
fn exponent(e: i32) -> String {
    format!("{}{:03}", if e < 0 { '-' } else { '+' }, e.unsigned_abs())
}

/// A float conversion (`f e E g G`) of a value.
fn print_float(s: &Spec, value: f64) -> Option<String> {
    let c = s.conversion;
    if !matches!(c, 'f' | 'e' | 'E' | 'g' | 'G') {
        return None;
    }
    let sign = if value.is_sign_negative() {
        "-"
    } else if s.plus {
        "+"
    } else if s.space {
        " "
    } else {
        ""
    };
    let v = value.abs();
    // (msvcr100's words for the values that are not numbers; a guess beyond the default precision)
    if v.is_infinite() || v.is_nan() {
        return Some(pad(s, sign, if v.is_nan() { "1.#QNAN0" } else { "1.#INF00" }, false));
    }
    let precision = s.precision.unwrap_or(6);
    let e_form = |precision: usize, upper: bool| -> String {
        let t = format!("{v:.precision$e}");
        let (mantissa, e) = t.split_once('e').unwrap_or((&t, "0"));
        let mut mantissa = mantissa.to_string();
        if s.alternate && precision == 0 {
            mantissa.push('.');
        }
        format!("{mantissa}{}{}", if upper { 'E' } else { 'e' }, exponent(e.parse().unwrap_or(0)))
    };
    let body = match c {
        'f' => {
            let mut t = format!("{v:.precision$}");
            if s.alternate && precision == 0 {
                t.push('.');
            }
            t
        }
        'e' | 'E' => e_form(precision, c == 'E'),
        _ => {
            // (`%g`: the shorter of the two by the exponent, its trailing zeros dropped unless `#`)
            let p = precision.max(1);
            let t = format!("{v:.prec$e}", prec = p - 1);
            let x: i32 = t.split_once('e').and_then(|(_, e)| e.parse().ok()).unwrap_or(0);
            let mut text = if x < -4 || x >= p as i32 { e_form(p - 1, c == 'G') } else { format!("{v:.prec$}", prec = (p as i32 - 1 - x).max(0) as usize) };
            if !s.alternate {
                let (number, rest) = match text.find(['e', 'E']) {
                    Some(at) => (text[..at].to_string(), text[at..].to_string()),
                    None => (text.clone(), String::new()),
                };
                let number = if number.contains('.') { number.trim_end_matches('0').trim_end_matches('.').to_string() } else { number };
                text = number + &rest;
            }
            text
        }
    };
    Some(pad(s, sign, &body, true))
}

/// `%s` (a text): its precision cuts it, its width pads it.
fn print_text(s: &Spec, text: &str) -> String {
    let cut: String = match s.precision {
        Some(precision) => text.chars().take(precision).collect(),
        None => text.to_string(),
    };
    pad(s, "", &cut, true)
}

thread_local! {
    /// LDB.dll's one 1 KB buffer every conversion is printed into: a time of all zeros prints nothing into it, and the
    /// piece gets what was printed there last.
    static PRINTED: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// What `Feed` prints into a `%` piece of its argument (the drop-in's `feed_*`, ACE's `snprintf` with the piece as the
/// format, at most 1023 bytes); `None` where the value and the conversion do not go together (the piece stays as it is).
fn print(piece: &str, arg: &Arg) -> Option<String> {
    let s = spec(piece)?;
    let printed = match arg {
        // (a number fed into a `%s` prints in a format of its own, whatever the piece's width: `conversion()`)
        Arg::Int(v) if s.conversion == 's' => format!("int_value<{}>", *v as i32),
        Arg::Uint(v) if s.conversion == 's' => format!("uint_value<{v}>"),
        Arg::Float(v) if s.conversion == 's' => format!("float_value<{:.6}>", *v as f64),
        Arg::Int(v) => print_int(&s, *v as u32)?,
        Arg::Uint(v) => print_int(&s, *v)?,
        // (a float handed to an integer conversion is read as its double's low word)
        Arg::Float(v) if matches!(s.conversion, 'd' | 'i' | 'u' | 'o' | 'x' | 'X' | 'c') => print_int(&s, (*v as f64).to_bits() as u32)?,
        Arg::Float(v) => print_float(&s, *v as f64)?,
        Arg::Text(t) if s.conversion == 's' => print_text(&s, t),
        // (a text in a number's place: the original prints the text's address; the client prints the text)
        Arg::Text(t) if matches!(s.conversion, 'd' | 'i' | 'u') => t.to_string(),
        Arg::Text(_) => return None,
        Arg::Time(days, hours, minutes, seconds) => {
            let mut t = String::new();
            for (n, one, many) in [(*days, "day", "days"), (*hours, "hour", "hours"), (*minutes, "minute", "minutes"), (*seconds, "second", "seconds")] {
                match n {
                    0 => {}
                    1 => t += &format!(" 1 {one}"),
                    _ => t += &format!(" {n} {many}"),
                }
            }
            if t.is_empty() {
                return Some(PRINTED.with(|p| p.borrow().clone()));
            }
            t
        }
    };
    let mut printed = printed;
    if printed.len() > 0x3ff {
        let mut end = 0x3ff;
        while !printed.is_char_boundary(end) {
            end -= 1;
        }
        printed.truncate(end);
    }
    PRINTED.with(|p| *p.borrow_mut() = printed.clone());
    Some(printed)
}

/// `Dump()`: the pieces joined, the spaces at the start dropped and each run of spaces made one.
fn dump(pieces: impl Iterator<Item = String>) -> String {
    let mut out = String::new();
    let mut space = true;
    for piece in pieces {
        for c in piece.chars() {
            if space {
                if c != ' ' {
                    space = false;
                    out.push(c);
                }
            } else {
                space = c == ' ';
                out.push(c);
            }
        }
    }
    out
}

/// A text with its arguments fed in order, as `LDBformat::Init`, `Feed` and `Dump` do it: argument `n` fills every
/// piece that is its own: a `%` conversion is printed with the value as `printf` would (a number in a `%s` as
/// `int_value<n>`), and a choice `#n{1:one | many}` takes the alternative its value picks (a number 0 has the flag `0`,
/// 1 the flag `1`, any other value none). `%%` is a percent sign. Pieces with nothing fed print as they are. The
/// result has no spaces at its start and no runs of spaces.
pub fn format(text: &str, args: &[Arg]) -> String {
    let mut tokens = tokens(text);
    for (n, arg) in args.iter().enumerate() {
        let n = n as u32 + 1;
        for (index, piece) in tokens.iter_mut().filter(|t| t.0 == n) {
            if piece.starts_with('%') {
                if let Some(filled) = print(piece, arg) {
                    *piece = filled;
                    *index = 0;
                }
            } else {
                let flags = match arg {
                    Arg::Int(0) | Arg::Uint(0) => 0x400_0000,
                    Arg::Int(1) | Arg::Uint(1) => 0x800_0000,
                    _ => 0,
                };
                if let Some(chosen) = choose(piece, flags) {
                    *piece = chosen;
                }
            }
        }
    }
    dump(tokens.into_iter().map(|t| t.1))
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
    // (the words: `i` fed as an int, `u` as an unsigned one, `f` as a float)
    let mut numbers: Vec<Option<(u8, u32)>> = Vec::new();
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
                parts.push(String::new());
                numbers.push(Some((c, value)));
                i += 6;
            }
            b'~' => {
                consumed = i + 1;
                break;
            }
            _ => break,
        }
    }
    let args: Vec<Arg> = parts
        .iter()
        .zip(&numbers)
        .map(|(t, n)| match *n {
            None => Arg::Text(t),
            Some((b'i', v)) => Arg::Int(v as i32 as i64),
            Some((b'u', v)) => Arg::Uint(v),
            // (`FeedFloat` of the word, a signalling NaN made quiet: the drop-in's `quiet`)
            Some((_, v)) => Arg::Float(f32::from_bits(if (v & 0x7f80_0000) == 0x7f80_0000 && (v & 0x007f_ffff) != 0 { v | 0x0040_0000 } else { v })),
        })
        .collect();
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
        // number the one without flags; the spaces about the bar belong to the alternatives, as in the original, and `Dump`
        // makes a run of them one)
        let reward = "%d points, %d #2{1:coin | coins}.";
        assert_eq!(format(reward, &[Arg::Int(5), Arg::Int(1)]), "5 points, 1 coin .");
        assert_eq!(format(reward, &[Arg::Int(5), Arg::Int(40)]), "5 points, 40 coins.");
        assert_eq!(format(reward, &[Arg::Int(5)]), "5 points, %d {1:coin | coins}.");
        // (`#2:` makes the next `%` the second argument; a `#` that is neither stays)
        assert_eq!(format("#2:%s then #1:%s", &[Arg::Text("a"), Arg::Text("b")]), "b then a");
        assert_eq!(format("item #7 of %d", &[Arg::Int(9)]), "item #7 of 9");
    }

    #[test]
    fn prints_each_conversion_as_printf() {
        // (widths, flags, precision and the other integer conversions, msvcr100's way)
        assert_eq!(format("[%5d|%-5d|%05d|%+d|% d]", &[Arg::Int(42), Arg::Int(42), Arg::Int(-42), Arg::Int(7), Arg::Int(7)]), "[ 42|42 |-0042|+7| 7]");
        assert_eq!(format("%x %X %#x %08X %o %#o %c", &[Arg::Int(255), Arg::Int(255), Arg::Int(255), Arg::Uint(0xbeef), Arg::Int(8), Arg::Int(8), Arg::Int(65)]), "ff FF 0xff 0000BEEF 10 010 A");
        assert_eq!(format("%.3d|%.0d|%hd|%u", &[Arg::Int(5), Arg::Int(0), Arg::Int(65535), Arg::Int(-1)]), "005||-1|4294967295");
        // (a text: its precision cuts it, its width pads it, though `Dump` makes the padding one space)
        assert_eq!(format("<%-6s><%6s><%.2s>", &[Arg::Text("ab"), Arg::Text("ab"), Arg::Text("abcdef")]), "<ab >< ab><ab>");
        // (a number in a `%s` prints in LDBformat's own words, whatever the width)
        assert_eq!(format("%s and %5s and %s", &[Arg::Int(-3), Arg::Uint(4), Arg::Float(1.5)]), "int_value<-3> and uint_value<4> and float_value<1.500000>");
    }

    #[test]
    fn prints_floats_and_times() {
        assert_eq!(format("%f %.1f %8.2f %e %E", &[Arg::Float(1.5), Arg::Float(2.25), Arg::Float(-3.0), Arg::Float(1234.5), Arg::Float(0.5)]), "1.500000 2.2 -3.00 1.234500e+003 5.000000E-001");
        assert_eq!(format("%g %g %g %G %.3g", &[Arg::Float(0.5), Arg::Float(100000.0), Arg::Float(1000000.0), Arg::Float(0.00001), Arg::Float(3.14159)]), "0.5 100000 1e+006 1E-005 3.14");
        // (a time: its parts that are not 0, whatever the conversion; a time of all zeros gives what was printed last)
        assert_eq!(format("Back in%s.", &[Arg::Time(1, 2, 0, 1)]), "Back in 1 day 2 hours 1 second.");
        assert_eq!(format("%d then%s", &[Arg::Int(9), Arg::Time(0, 0, 0, 0)]), "9 then9");
    }

    #[test]
    fn dump_drops_leading_spaces_and_runs() {
        assert_eq!(format("  a   b %s", &[Arg::Text("  c")]), "a b c");
        assert_eq!(format("end ", &[]), "end ");
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
