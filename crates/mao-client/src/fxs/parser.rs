//! `FXS_ScriptParser_t`, the effect scripts' reader (FXS.dll; `AnarchyReimagined`'s `dropin/fxs` `parser.rs`): a script
//! file (from the data path's `twk` folder, with a fallback and an extra one) read line by line, `#` comments cut,
//! characters outside its set made spaces, the `!include { file }` and `!parameters` commands carried out; each line kept
//! with a trailing space beside its file name and number, then words (or quoted texts) read off as tokens with one of
//! lookahead, each typed by its keyword.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::enums::ENUMS;

/// The token types (FUN_1000cee9): the value types 1 to 9, `Expansion` 0xa to `Criteria` 0x14, `}` 0x15.
pub const STRING: i32 = 1;
pub const OBJECT: i32 = 0x13;
pub const CLOSE: i32 = 0x15;
pub const OPERATOR: i32 = 0x16;
pub const ARRAY: i32 = 0x17;
pub const VECTOR_CHART: i32 = 0x18;
pub const ASSIGN: i32 = 0x19;

/// A line kept: its text (a space appended), its file's name and its number.
struct Line {
    text: Vec<u8>,
    file: Rc<str>,
    number: i32,
}

pub struct Parser {
    data_path: PathBuf,
    dir: String,
    fallback: Option<String>,
    extra: Option<String>,
    lines: Vec<Line>,
    /// The read position: the line and the place in it.
    line_at: usize,
    pos: usize,
    pub token: Vec<u8>,
    pub lookahead: Vec<u8>,
    pub file: Option<Rc<str>>,
    pub line: i32,
    next_file: Option<Rc<str>>,
    next_line: i32,
    pub kind: i32,
    pub next_kind: i32,
    /// The errors reported (the original prints them).
    pub errors: Vec<String>,
}

/// FUN_1000af3f: whether a character is kept in a line (letters, digits and `!"%&()*+-./<=>[\]_{|}~`).
fn is_script_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'!' | b'"' | b'%' | b'&' | b'(' | b')' | b'*' | b'+' | b'-' | b'.' | b'/' | b'<'..=b'>' | b'['..=b']' | b'_' | b'{'..=b'~')
}

/// A file read in `fgets(buf, 0x200)` pieces: up to 511 bytes, a piece ending after its newline.
fn pieces(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let mut end = at;
        while end < bytes.len() && end - at < 0x1ff {
            end += 1;
            if bytes[end - 1] == b'\n' {
                break;
            }
        }
        out.push(bytes[at..end].to_vec());
        at = end;
    }
    out
}

impl Parser {
    /// FUN_1000dbab.
    pub fn new(data_path: &Path, dir: &str, fallback: Option<&str>, extra: Option<&str>) -> Parser {
        Parser {
            data_path: data_path.to_path_buf(),
            dir: dir.to_string(),
            fallback: fallback.map(str::to_string),
            extra: extra.map(str::to_string),
            lines: Vec::new(),
            line_at: 0,
            pos: 0,
            token: Vec::new(),
            lookahead: Vec::new(),
            file: None,
            line: 0,
            next_file: None,
            next_line: 0,
            kind: 0,
            next_kind: 0,
            errors: Vec::new(),
        }
    }

    /// A parser over a script's text (for the tests): its lines as one file's.
    #[cfg(test)]
    pub fn from_text(text: &str) -> Parser {
        let mut p = Parser::new(Path::new("."), "test", None, None);
        p.load_bytes(text.as_bytes(), "test", None, &mut |_| None);
        p.reset();
        p
    }

    /// FUN_1000afbb: a script error, at the token's file and line.
    pub fn error(&mut self, message: &str) {
        let at = self.file.as_deref().unwrap_or("FXS_ScriptParser.cpp");
        self.errors.push(format!("{at}:{}: error: {message}", self.line));
    }

    /// FUN_1000d594: back to the first line, no tokens read.
    fn reset(&mut self) {
        self.pos = 0;
        self.line_at = 0;
        self.token.clear();
        self.lookahead.clear();
        self.file = None;
        self.line = 0;
        self.next_file = None;
        self.next_line = 0;
        self.kind = 0;
        self.next_kind = 0;
    }

    /// FUN_1000d5df: the next word (to a space or the line's end) or quoted text (to a quote or the line's end), moving on
    /// through lines of spaces; none past the last line.
    fn read_word(&mut self) -> Option<Vec<u8>> {
        let line = loop {
            let line = self.lines.get(self.line_at)?;
            while line.text.get(self.pos) == Some(&b' ') {
                self.pos += 1;
            }
            if self.pos < line.text.len() {
                break &line.text;
            }
            self.pos = 0;
            self.line_at += 1;
        };
        let mut out = Vec::new();
        if line[self.pos] == b'"' {
            self.pos += 1;
            while self.pos < line.len() && line[self.pos] != b'"' {
                out.push(line[self.pos]);
                self.pos += 1;
            }
            if self.pos < line.len() {
                self.pos += 1;
            }
        } else {
            while self.pos < line.len() && line[self.pos] != b' ' {
                out.push(line[self.pos]);
                self.pos += 1;
            }
        }
        Some(out)
    }

    /// FUN_1000d6dd: the next token: the lookahead (or, with none, a word read) with its type, file and line, and a new
    /// lookahead read after it.
    pub fn next(&mut self) -> bool {
        if self.lookahead.is_empty() {
            let Some(w) = self.read_word() else { return false };
            self.kind = token_type(&w);
            self.token = w;
            let l = &self.lines[self.line_at];
            self.file = Some(l.file.clone());
            self.line = l.number;
        } else {
            self.token = std::mem::take(&mut self.lookahead);
            self.kind = self.next_kind;
            self.file = self.next_file.clone();
            self.line = self.next_line;
        }
        self.lookahead = self.read_word().unwrap_or_default();
        self.next_kind = token_type(&self.lookahead);
        let l = self.lines.get(self.line_at);
        self.next_file = l.map(|l| l.file.clone());
        self.next_line = l.map_or(0, |l| l.number);
        true
    }

    /// FUN_1000d7dd: the next token's text.
    pub fn read_text(&mut self) -> Option<Vec<u8>> {
        if !self.next() {
            self.error("Expected text");
            return None;
        }
        Some(self.token.clone())
    }

    /// FUN_1000d848: the next token, which must be a type keyword (1 to 0x15); reported when asked.
    pub fn read_type(&mut self, report: bool) -> bool {
        if self.next() && (1..=CLOSE).contains(&self.kind) {
            return true;
        }
        if report {
            self.error("Expected a type statement");
            let found = String::from_utf8_lossy(&self.token).into_owned();
            self.errors.push(format!("FXS_ScriptParser.cpp:591: error: Found: {found}"));
        }
        false
    }

    /// FUN_1000d8e0: the next token, which must be `{`.
    pub fn read_brace(&mut self) -> bool {
        if self.next() && self.token == b"{" {
            return true;
        }
        self.error("Expected '{'");
        false
    }

    /// FUN_1000d952: the next token's name ID.
    pub fn read_name(&mut self) -> Option<u32> {
        if !self.next() {
            self.error("Expected name");
            return None;
        }
        Some(name_id(&self.token))
    }

    /// FUN_1000d9c4: the next token as `[size]`.
    pub fn read_array_size(&mut self) -> Option<i32> {
        if !self.next() {
            self.error("Expected array size");
            return None;
        }
        let t = &self.token;
        Some(t.strip_prefix(b"[").map_or(0, |r| scan_int(r).map_or(0, |(v, _)| v as i32)))
    }

    /// FUN_1000da3c: a vector of the next three tokens' floats (0 for those missing), the token after them skipped.
    pub fn read_vector(&mut self) -> [f32; 3] {
        let mut v = [0.0f32; 3];
        for c in &mut v {
            if let Some(t) = self.read_text() {
                *c = float_value(&t);
            }
        }
        self.read_text();
        v
    }

    /// FUN_1000dd04: a line kept (with a space appended); empty lines are not.
    fn add_line(&mut self, line: &[u8], file: &str, number: i32) {
        if line.is_empty() {
            return;
        }
        let mut text = line.to_vec();
        text.push(b' ');
        self.lines.push(Line { text, file: file.into(), number });
    }

    /// The data path's `twk\name`.
    fn script_path(&self, name: &str) -> PathBuf {
        self.data_path.join("twk").join(name.replace('\\', "/"))
    }

    /// FUN_1000de1f: a script file's lines added, with its commands carried out: `!include { name }` (the line before it
    /// kept, then the data path's `twk\name` loaded) and `!parameters` (the parameters read as the next line). A
    /// command's error stops the file.
    fn load(&mut self, path: &Path, params: Option<&[u8]>) {
        let Ok(bytes) = std::fs::read(path) else {
            self.errors.push(format!("FXS_ScriptParser.cpp:201: error: Failed to open {}", path.display()));
            return;
        };
        let name = path.to_string_lossy().into_owned();
        let data_path = self.data_path.clone();
        self.load_bytes(&bytes, &name, params, &mut |include| std::fs::read(data_path.join("twk").join(include)).ok().map(|b| (b, data_path.join("twk").join(include))));
    }

    fn load_bytes(&mut self, bytes: &[u8], name: &str, params: Option<&[u8]>, include: &mut dyn FnMut(&str) -> Option<(Vec<u8>, PathBuf)>) {
        let mut number = 0i32;
        let mut pieces = pieces(bytes).into_iter();
        let mut substitute = false;
        loop {
            let mut buf = if substitute {
                substitute = false;
                params.unwrap_or_default().to_vec()
            } else {
                number += 1;
                match pieces.next() {
                    Some(p) => p,
                    None => break,
                }
            };
            // (the text to its first NUL, as the original's string)
            if let Some(z) = buf.iter().position(|&c| c == 0) {
                buf.truncate(z);
            }
            let mut bang = None;
            let mut quoted = false;
            let mut i = 0;
            while i < buf.len() {
                let c = buf[i];
                if c == b'"' {
                    quoted = !quoted;
                } else if !quoted {
                    if c == b'#' {
                        buf.truncate(i);
                        break;
                    } else if !is_script_char(c) {
                        buf[i] = b' ';
                    } else if c == b'!' && bang.is_none() {
                        bang = Some(i);
                    }
                }
                i += 1;
            }
            let Some(bang) = bang else {
                self.add_line(&buf, name, number);
                continue;
            };
            let at = |j: usize| buf.get(j).copied().unwrap_or(0);
            let mut j = bang;
            let mut command = Vec::new();
            loop {
                j += 1;
                let c = at(j);
                if c == 0 || c == b' ' || c == b'{' {
                    break;
                }
                command.push(c.to_ascii_lowercase());
            }
            let fail = |p: &mut Parser, message: &str| p.errors.push(format!("{name}:{number}: error: {message}"));
            if command == b"include" {
                while at(j) == b' ' {
                    j += 1;
                }
                if at(j) != b'{' {
                    return fail(self, "Include command is missing begin bracket '{'");
                }
                j += 1;
                while at(j) == b' ' {
                    j += 1;
                }
                let mut file = Vec::new();
                while !matches!(at(j), 0 | b' ' | b'}') {
                    file.push(at(j));
                    j += 1;
                }
                if file.is_empty() {
                    return fail(self, "Include command is missing include name");
                }
                while at(j) == b' ' {
                    j += 1;
                }
                if at(j) != b'}' {
                    return fail(self, "Include command is missing end bracket '}'");
                }
                self.add_line(&buf[..bang], name, number);
                let file = String::from_utf8_lossy(&file).into_owned();
                match include(&file) {
                    Some((bytes, path)) => self.load_bytes(&bytes, &path.to_string_lossy(), params, include),
                    None => self.errors.push(format!("FXS_ScriptParser.cpp:201: error: Failed to open {}", self.script_path(&file).display())),
                }
            } else if command == b"parameters" {
                if params.is_some() {
                    substitute = true;
                }
            } else {
                return fail(self, "Unknown command found");
            }
        }
    }

    /// FUN_1000e1e3: the script `dir.txt` loaded (the fallback's when it is missing), then the extra one when there is
    /// one, and the parser reset.
    pub fn open(&mut self, params: Option<&[u8]>) {
        let mut path = self.script_path(&format!("{}.txt", self.dir));
        if !path.exists() {
            let Some(fallback) = self.fallback.clone() else { return self.reset() };
            path = self.script_path(&format!("{fallback}.txt"));
            if !path.exists() {
                self.errors.push(format!("FXS_ScriptParser.cpp:121: error: Did not find {}", self.dir));
                self.errors.push(format!("FXS_ScriptParser.cpp:122: error: Did not find {fallback}"));
                return self.reset();
            }
        }
        self.load(&path, params);
        if let Some(extra) = self.extra.clone() {
            let path = self.script_path(&format!("{extra}.txt"));
            if path.exists() {
                self.load(&path, params);
            } else {
                self.errors.push(format!("FXS_ScriptParser.cpp:159: error: Did not find {extra}"));
            }
        }
        self.reset();
    }
}

/// FUN_1000cee9: a token's type: its keyword's, 0x16 for an operator, 0x17 for `[...]`, else 0.
pub fn token_type(t: &[u8]) -> i32 {
    match t {
        b"String" => 1,
        b"Unsigned" => 2,
        b"Signed" => 3,
        b"Float" => 4,
        b"Double" => 5,
        b"Vector" => 6,
        b"Quaternion" => 7,
        b"Matrix" => 8,
        b"Sound" => 9,
        b"Object" => 0x13,
        b"}" => 0x15,
        b"=" => 0x19,
        b"Expansion" => 0xa,
        b"StateBlob" => 0xb,
        b"RenderState" => 0xc,
        b"TextureStage" => 0xd,
        b"Mesh" => 0xe,
        b"Texture" => 0xf,
        b"Transform" => 0x10,
        b"Material" => 0x11,
        b"RenderTarget" => 0x12,
        b"Criteria" => 0x14,
        b"VectorChart" => 0x18,
        _ if operator(t) != 0 => OPERATOR,
        [b'[', .., b']'] => ARRAY,
        _ => 0,
    }
}

/// FUN_1000d324: an operator's number, 1 to 0x1c, 0 for none.
pub fn operator(t: &[u8]) -> i32 {
    const OPERATORS: [&[u8]; 28] = [
        b"+", b"-", b"*", b"/", b"~", b"%", b"<|", b"|>", b"[DOT]", b"[LENGTH]", b"[CROSS]", b"[ROT]", b"[NORMALIZE]", b"|", b"&", b"\xa4", b"==", b"<>", b"<", b">", b"<=", b">=",
        b"[TRIGGER]", b"[RUN]", b"[STORE]", b"[AFCM]", b"[SEND]", b"[PRINT]",
    ];
    OPERATORS.iter().position(|o| *o == t).map_or(0, |k| k as i32 + 1)
}

/// FUN_1000c97e: the type a value literal is written as: 8 for a capital first letter (a name), 0 (unsigned) for `0x...`,
/// `e...` and a `u` suffix, 4 to 7 for `v(`, `q(`, `m(` and `s(`, 1 for an `s` suffix (signed), 3 for `d` (double), else
/// 2 (float).
pub fn literal_type(t: &[u8]) -> i32 {
    let c = t.first().copied().unwrap_or(0);
    let second = t.get(1).copied().unwrap_or(0);
    match c {
        b'A'..=b'Z' => return 8,
        b'0' if second == b'x' => return 0,
        b'e' => return 0,
        b'm' if second == b'(' => return 6,
        b'q' if second == b'(' => return 5,
        b's' if second == b'(' => return 7,
        b'v' if second == b'(' => return 4,
        _ => {}
    }
    match t.last() {
        Some(b'd') => 3,
        Some(b's') => 1,
        Some(b'u') => 0,
        _ => 2,
    }
}

/// `%d`'s scan: spaces, a sign, digits; the value and the bytes used.
fn scan_int(t: &[u8]) -> Option<(i64, usize)> {
    let mut i = t.iter().take_while(|c| c.is_ascii_whitespace()).count();
    let neg = match t.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let start = i;
    let mut v: i64 = 0;
    while let Some(c) = t.get(i).filter(|c| c.is_ascii_digit()) {
        v = v.wrapping_mul(10).wrapping_add((c - b'0') as i64);
        i += 1;
    }
    (i > start).then_some((if neg { -v } else { v }, i))
}

/// `%f`'s scan: the longest float prefix.
fn scan_float(t: &[u8]) -> Option<f32> {
    let t = &t[t.iter().take_while(|c| c.is_ascii_whitespace()).count()..];
    let digits = |t: &[u8], i: usize| i + t[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut i = usize::from(matches!(t.first(), Some(b'+' | b'-')));
    let int_end = digits(t, i);
    let mut end = int_end;
    if t.get(end) == Some(&b'.') {
        end = digits(t, end + 1);
    }
    if end == i || (end == i + 1 && int_end == i) {
        return None;
    }
    i = end;
    if matches!(t.get(i), Some(b'e' | b'E')) {
        let s = i + 1 + usize::from(matches!(t.get(i + 1), Some(b'+' | b'-')));
        let e = digits(t, s);
        if e > s {
            end = e;
        }
    }
    std::str::from_utf8(&t[..end]).ok()?.parse::<f32>().ok()
}

/// FUN_1000b015: an enum constant's value; 0 when unknown (reported by the caller's parser).
pub fn enum_value(t: &[u8]) -> Option<u32> {
    ENUMS.iter().find(|(name, _)| *name == t).map(|(_, v)| *v)
}

/// FUN_1000ca20: an unsigned literal (`0x` hex, an enum constant, or with a `u` suffix), else 0. (The original's literal
/// that doesn't scan is the address of its text; 0 here.)
pub fn unsigned_value(p: &mut Parser, t: &[u8]) -> u32 {
    if t.starts_with(b"0x") {
        let hex: Vec<u8> = t[2..].iter().copied().take_while(u8::is_ascii_hexdigit).collect();
        return u32::from_str_radix(std::str::from_utf8(&hex).unwrap_or("0"), 16).unwrap_or(0);
    }
    if t.first() == Some(&b'e') {
        return enum_value(t).unwrap_or_else(|| {
            p.error("Enum not found!");
            0
        });
    }
    if t.last() == Some(&b'u') {
        return scan_int(t).map_or(0, |(v, _)| v as u32);
    }
    0
}

/// FUN_1000ca7e: a signed literal.
pub fn signed_value(t: &[u8]) -> i32 {
    scan_int(t).map_or(0, |(v, _)| v as i32)
}

/// FUN_1000ca9d: a float literal, with an `f` suffix or none.
pub fn float_value(t: &[u8]) -> f32 {
    scan_float(t).unwrap_or(0.0)
}

/// FUN_1000cadb: a double literal (`d` suffix), read as a float.
pub fn double_value(t: &[u8]) -> f32 {
    scan_float(t).unwrap_or(0.0)
}

/// The ID of a name (FXS.dll FUN_1000fd15, the same as SandyInterface's sound IDs): the name's first 255 bytes, case
/// folded, taken four at a time as big-endian words, each rotated left by its index and XORed together; a last partial
/// word has its bytes in the same places (its third byte left out, as the original's), zeros after them.
pub fn name_id(name: &[u8]) -> u32 {
    let len = name.len().min(0xff);
    let lower = |k: usize| name[k].to_ascii_lowercase() as u32;
    let mut h = 0u32;
    let words = len / 4;
    for i in 0..words {
        let w = (lower(4 * i) << 24) | (lower(4 * i + 1) << 16) | (lower(4 * i + 2) << 8) | lower(4 * i + 3);
        h ^= w.rotate_left(i as u32 & 31);
    }
    if len & 3 != 0 {
        let i = words;
        let b1 = if 4 * i + 1 < len { lower(4 * i + 1) << 16 } else { 0 };
        let b2 = if 4 * i + 2 < len { lower(4 * i + 2) << 8 } else { 0 };
        let w = (lower(4 * i) << 24).wrapping_add(b2).wrapping_add(b1);
        h ^= w.rotate_left(i as u32 & 31);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_tokens_and_literals() {
        let mut p = Parser::from_text("Object GAME # a comment\n{\n  Unsigned Use: e_Automatic\n  String Dest \"A B\"\n}\n");
        let mut tokens = Vec::new();
        while p.next() {
            tokens.push((String::from_utf8_lossy(&p.token).into_owned(), p.kind, p.line));
        }
        assert_eq!(tokens[0], ("Object".into(), OBJECT, 1));
        assert_eq!(tokens[1].0, "GAME");
        assert_eq!(tokens[3], ("Unsigned".into(), 2, 3));
        // (the colon is not a script character)
        assert_eq!(tokens[4].0, "Use");
        assert_eq!(tokens[5].0, "e_Automatic");
        assert_eq!(tokens[7].0, "Dest");
        assert_eq!(tokens[8].0, "A B");
        assert_eq!(tokens[9], ("}".into(), CLOSE, 5));
        assert_eq!(literal_type(b"0u"), 0);
        assert_eq!(literal_type(b"v("), 4);
        assert_eq!(literal_type(b"This.X"), 8);
        assert_eq!(literal_type(b"1.5"), 2);
        assert_eq!(unsigned_value(&mut p, b"0x01000000"), 0x0100_0000);
        assert_eq!(unsigned_value(&mut p, b"255u"), 255);
        assert_eq!(unsigned_value(&mut p, b"e_Automatic"), 3);
        assert_eq!(float_value(b"45.0"), 45.0);
        assert_eq!(token_type(b"[5]"), ARRAY);
        assert_eq!(token_type(b"[ROT]"), OPERATOR);
        assert_eq!(name_id(b"Game"), name_id(b"GAME"));
    }
}
