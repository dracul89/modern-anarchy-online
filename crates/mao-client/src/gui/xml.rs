//! The skin's XML files as GUI.dll reads them: TinyXML elements (Utils.dll's `TiXml*`, the `utils` drop-in's `tinyxml`)
//! and `XMLObject_c`'s attribute getters (the drop-in's `xml_object.rs`): text with `#name` looked up in the text database,
//! numbers and `Point(...)`/`Rect(...)` through `Variant::LoadFromString`.

use super::geom::{Point, Rect};

/// An element: its name, attributes in order, child elements and its text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

/// The document's elements (the declaration, comments and other markup skipped): a document of a `<root>` holding the
/// views is `[root]`.
pub fn parse(text: &str) -> Vec<Element> {
    let mut p = Parser { s: text.as_bytes(), at: 0 };
    let mut out = Vec::new();
    p.content(&mut out, &mut String::new());
    out
}

struct Parser<'a> {
    s: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn starts(&self, t: &str) -> bool {
        self.s[self.at..].starts_with(t.as_bytes())
    }

    fn skip_past(&mut self, t: &str) {
        match self.s[self.at..].windows(t.len()).position(|w| w == t.as_bytes()) {
            Some(i) => self.at += i + t.len(),
            None => self.at = self.s.len(),
        }
    }

    fn space(&mut self) {
        while self.at < self.s.len() && self.s[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }

    fn name(&mut self) -> String {
        let start = self.at;
        while self.at < self.s.len() && !self.s[self.at].is_ascii_whitespace() && !b"/>=".contains(&self.s[self.at]) {
            self.at += 1;
        }
        latin1(&self.s[start..self.at])
    }

    /// Elements and text up to the parent's end tag (or the end).
    fn content(&mut self, out: &mut Vec<Element>, text: &mut String) {
        while self.at < self.s.len() {
            if self.starts("</") {
                self.skip_past(">");
                return;
            }
            if self.starts("<!--") {
                self.skip_past("-->");
            } else if self.starts("<![CDATA[") {
                let start = self.at + 9;
                self.skip_past("]]>");
                text.push_str(&latin1(&self.s[start..self.at.saturating_sub(3).max(start)]));
            } else if self.starts("<?") || self.starts("<!") {
                self.skip_past(">");
            } else if self.starts("<") {
                self.at += 1;
                out.push(self.element());
            } else {
                let start = self.at;
                while self.at < self.s.len() && self.s[self.at] != b'<' {
                    self.at += 1;
                }
                text.push_str(&unescape(&latin1(&self.s[start..self.at])));
            }
        }
    }

    fn element(&mut self) -> Element {
        let mut e = Element { name: self.name(), ..Default::default() };
        loop {
            self.space();
            if self.at >= self.s.len() {
                return e;
            }
            if self.starts("/>") {
                self.at += 2;
                return e;
            }
            if self.s[self.at] == b'>' {
                self.at += 1;
                let (mut children, mut text) = (Vec::new(), String::new());
                self.content(&mut children, &mut text);
                e.children = children;
                e.text = text.trim().to_string();
                return e;
            }
            let name = self.name();
            self.space();
            let mut value = String::new();
            if self.starts("=") {
                self.at += 1;
                self.space();
                let quote = self.s.get(self.at).copied().unwrap_or(b'"');
                if quote == b'"' || quote == b'\'' {
                    self.at += 1;
                    let start = self.at;
                    while self.at < self.s.len() && self.s[self.at] != quote {
                        self.at += 1;
                    }
                    value = unescape(&latin1(&self.s[start..self.at]));
                    self.at += 1;
                } else {
                    value = self.name();
                }
            }
            if name.is_empty() {
                self.at += 1;
                continue;
            }
            e.attributes.push((name, value));
        }
    }
}

fn latin1(b: &[u8]) -> String {
    // (the skin files are Latin-1 or UTF-8; read as UTF-8 when they are)
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => b.iter().map(|&c| c as char).collect(),
    }
}

/// TinyXML's entities (`&amp;` `&lt;` `&gt;` `&quot;` `&apos;`, `&#n;`, `&#xn;`).
fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest.find(';').filter(|&e| e < 12);
        let decoded = end.and_then(|e| {
            let name = &rest[1..e];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => name
                    .strip_prefix("#x")
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            };
            c.map(|c| (c, e))
        });
        match decoded {
            Some((c, e)) => {
                out.push(c);
                rest = &rest[e + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// A `scanf` of `format` over `text` with `%f` (skipping white space first) and literal characters: the floats read
/// before the first mismatch.
fn scan_floats(text: &str, format: &str) -> Vec<f32> {
    let (t, f) = (text.as_bytes(), format.as_bytes());
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while j < f.len() {
        if f[j] == b'%' && f.get(j + 1) == Some(&b'f') {
            while i < t.len() && t[i].is_ascii_whitespace() {
                i += 1;
            }
            let start = i;
            if i < t.len() && (t[i] == b'-' || t[i] == b'+') {
                i += 1;
            }
            while i < t.len() && (t[i].is_ascii_digit() || t[i] == b'.') {
                i += 1;
            }
            if i < t.len() && (t[i] == b'e' || t[i] == b'E') {
                let save = i;
                i += 1;
                if i < t.len() && (t[i] == b'-' || t[i] == b'+') {
                    i += 1;
                }
                if i < t.len() && t[i].is_ascii_digit() {
                    while i < t.len() && t[i].is_ascii_digit() {
                        i += 1;
                    }
                } else {
                    i = save;
                }
            }
            match text[start..i].parse::<f32>() {
                Ok(v) => out.push(v),
                Err(_) => return out,
            }
            j += 2;
        } else if f[j].is_ascii_whitespace() {
            while i < t.len() && t[i].is_ascii_whitespace() {
                i += 1;
            }
            j += 1;
        } else {
            if t.get(i) != Some(&f[j]) {
                return out;
            }
            i += 1;
            j += 1;
        }
    }
    out
}

/// What `Variant::LoadFromString` makes of an attribute's text.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f32),
    Text(String),
    Rect(Rect),
    Point(Point),
}

pub fn load_from_string(text: &str) -> Option<Value> {
    if text.eq_ignore_ascii_case("false") {
        return Some(Value::Bool(false));
    }
    if text.eq_ignore_ascii_case("true") {
        return Some(Value::Bool(true));
    }
    let digits = text.strip_prefix('-').unwrap_or(text);
    let dots = digits.bytes().filter(|&c| c == b'.').count();
    if dots <= 1 && digits.bytes().all(|c| c == b'.' || c.is_ascii_digit()) {
        if dots == 1 && !digits.is_empty() {
            return Some(Value::Float(text.parse().unwrap_or(0.0)));
        }
        return Some(Value::Int(text.parse().unwrap_or(0)));
    }
    if text.starts_with('"') {
        return Some(Value::Text(if text.len() < 3 { String::new() } else { text[1..text.len() - 1].to_string() }));
    }
    let r = scan_floats(text, "Rect(%f,%f,%f,%f)");
    if r.len() == 4 {
        return Some(Value::Rect(Rect::new(r[0], r[1], r[2], r[3])));
    }
    let p = scan_floats(text, "Point(%f,%f)");
    if p.len() == 2 {
        return Some(Value::Point(Point::new(p[0], p[1])));
    }
    None
}

impl Element {
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    /// `GetAttrString`: the text, `#<n>:<m>` the text database's category `n` key `m`, `#<name>` its category 700 entry
    /// named so (`LdbGetText(700, name)`).
    pub fn text(&self, name: &str, default: &str) -> String {
        match self.attribute(name) {
            Some(v) => ldb_text(v),
            None => default.to_string(),
        }
    }

    /// `GetAttrBool`: `AsBool` of the loaded value, the default when missing.
    pub fn bool(&self, name: &str, default: bool) -> bool {
        match self.attribute(name).and_then(load_from_string) {
            Some(Value::Bool(b)) => b,
            Some(Value::Int(i)) => i != 0,
            Some(Value::Float(f)) => f != 0.0,
            Some(Value::Text(t)) => t.eq_ignore_ascii_case("true"),
            Some(_) => false,
            None => default,
        }
    }

    /// `GetAttrInt`: the attribute as an expression's integer value (numbers, `|`-joined flag names the caller resolves:
    /// see `flags`), the default when missing or not a number.
    pub fn int(&self, name: &str, default: i32) -> i32 {
        let Some(v) = self.attribute(name) else { return default };
        let v = v.trim();
        if let Some(h) = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
            return u32::from_str_radix(h, 16).map_or(default, |n| n as i32);
        }
        v.parse::<i64>().map(|n| n as i32).or_else(|_| v.parse::<f32>().map(|f| f as i32)).unwrap_or(default)
    }

    pub fn float(&self, name: &str, default: f32) -> f32 {
        self.attribute(name).and_then(|v| v.trim().parse().ok()).unwrap_or(default)
    }

    /// `GetAttrPoint`: `AsPoint` of the loaded value (a rect's left and top; (0, 0) for other kinds), the default when
    /// missing or not loadable.
    pub fn point(&self, name: &str, default: Point) -> Point {
        match self.attribute(name).and_then(load_from_string) {
            Some(Value::Point(p)) => p,
            Some(Value::Rect(r)) => Point::new(r.left, r.top),
            Some(_) => Point::default(),
            None => default,
        }
    }

    /// `GetAttrRect`: `AsRect` of the loaded value (a point as a rect of no size there), the default when missing or not
    /// loadable.
    pub fn rect(&self, name: &str, default: Rect) -> Rect {
        match self.attribute(name).and_then(load_from_string) {
            Some(Value::Rect(r)) => r,
            Some(Value::Point(p)) => Rect::new(p.x, p.y, p.x, p.y),
            Some(_) => Rect::default(),
            None => default,
        }
    }

    /// The child elements named `name` (any case).
    pub fn named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.name.eq_ignore_ascii_case(name))
    }
}

/// A `|`-joined list of flag names (spaces allowed) by `table`; numbers are taken as they are.
pub fn flags(text: &str, table: &[(&str, u32)]) -> u32 {
    text.split('|')
        .map(str::trim)
        .map(|n| table.iter().find(|(t, _)| *t == n).map(|(_, v)| *v).or_else(|| n.parse().ok()).unwrap_or(0))
        .fold(0, |a, b| a | b)
}

/// The text database's entry for `#...` text (`XMLObject_c::GetAttrString` over `LdbGetText`/`LdbGetTextInt`); other text,
/// or a missing entry, as it is (the name without its `#` for a missing one).
pub fn ldb_text(v: &str) -> String {
    let Some(rest) = v.strip_prefix('#') else { return v.to_string() };
    let looked = crate::feedback::texts().and_then(|t| match rest.split_once(':') {
        Some((n, m)) if n.bytes().all(|c| c.is_ascii_digit()) && !n.is_empty() => {
            t.get(n.parse().ok()?, m.trim().parse::<i32>().unwrap_or(0)).map(str::to_string)
        }
        _ => t.named(700, rest).map(str::to_string),
    });
    looked.unwrap_or_else(|| rest.to_string())
}

/// The first view element of a views file: the root's first child element, or its first one whose `name` is `name`
/// (`XMLObject_c::LoadXMLObject`).
pub fn view_element(document: &[Element], name: &str) -> Option<Element> {
    let root = document.first()?;
    if name.is_empty() {
        return root.children.first().cloned();
    }
    root.children.iter().find(|c| c.attribute("name") == Some(name)).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_views_file() {
        let doc = parse(
            "<?xml version=\"1.0\" ?><root>\n<!-- c -->\n<View view_layout=\"vertical\">\n<TextView name=\"a\" min_size=\"Point( 0, 0 )\" layout_borders=\"Rect(3,3,3,3)\" value=\"x &amp;&amp; y\"/>\n<HLayoutSpacer/></View></root>",
        );
        assert_eq!(doc.len(), 1);
        let v = view_element(&doc, "").unwrap();
        assert_eq!(v.name, "View");
        assert_eq!(v.children.len(), 2);
        let t = &v.children[0];
        assert_eq!(t.point("min_size", Point::new(-1.0, -1.0)), Point::new(0.0, 0.0));
        assert_eq!(t.rect("layout_borders", Rect::default()), Rect::new(3.0, 3.0, 3.0, 3.0));
        assert_eq!(t.attribute("value"), Some("x && y"));
        assert_eq!(t.point("max_size", Point::new(-1.0, -1.0)), Point::new(-1.0, -1.0));
    }

    #[test]
    fn loads_values_as_the_variant_does() {
        assert_eq!(load_from_string("Point(16000,16000)"), Some(Value::Point(Point::new(16000.0, 16000.0))));
        // (a space before the comma stops the scan after one number: not a point)
        assert_eq!(load_from_string("Point(1 ,2)"), None);
        assert_eq!(load_from_string("12"), Some(Value::Int(12)));
        assert_eq!(load_from_string("-1.5"), Some(Value::Float(-1.5)));
        assert_eq!(flags("TVF_MULTILINE | TVF_WORD_WRAP", &[("TVF_MULTILINE", 1), ("TVF_WORD_WRAP", 2)]), 3);
    }
}
