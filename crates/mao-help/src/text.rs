//! A page's text out of its HTML: the article's own words under their headings, without the scripts, menus, forms and
//! footers around them. Not a browser: tags are read as they come, with no tree, which is enough for guides and
//! forum posts and forgiving of markup that is not closed.

use serde::{Deserialize, Serialize};

/// A stretch of a page under one heading (the heading empty for what comes before the first).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    pub heading: String,
    pub text: String,
}

/// A page: its title and its sections in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Page {
    pub title: String,
    pub sections: Vec<Section>,
}

/// How a page is read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reading {
    /// The page's own line ends are line ends (the game's help files, which are text with a few tags); otherwise they
    /// are spaces, as in HTML.
    pub keep_newlines: bool,
}

/// Elements whose contents are never the article's words.
const DROPPED: &[&str] = &["script", "style", "nav", "header", "footer", "form", "select", "textarea", "noscript", "svg", "button", "aside", "iframe", "head", "title"];
/// Words in an element's class or id that mark it as something around the article.
const DROPPED_MARKS: &[&str] = &["toc", "nav", "menu", "sidebar", "breadcrumb", "cookie", "footer", "share", "comment-form", "advert", "totop", "bbtotop"];
/// The heading a contents anchor starts (see [`extract`]): it has no closing tag of its own.
const ANCHORED: &str = "(anchor)";
/// Elements without a closing tag.
const VOID: &[&str] = &["br", "img", "input", "hr", "meta", "link", "area", "base", "col", "embed", "source", "track", "wbr"];
/// Elements that start a new line.
const BLOCKS: &[&str] = &["p", "div", "br", "li", "tr", "table", "ul", "ol", "section", "article", "blockquote", "pre", "dd", "dt", "hr", "h1", "h2", "h3", "h4", "h5", "h6"];

/// `&amp;`, `&#39;`, `&#x27;` and the like as their characters; what is not known stays as written.
pub fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest[1..].find(|c: char| !(c.is_ascii_alphanumeric() || c == '#')).map(|e| e + 1);
        let decoded = end.filter(|&e| rest[e..].starts_with(';') && e > 1).and_then(|e| {
            let name = &rest[1..e];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                "bull" | "middot" => Some('\u{2022}'),
                "ndash" => Some('-'),
                "mdash" => Some('-'),
                "hellip" => Some('\u{2026}'),
                "rsquo" | "lsquo" => Some('\''),
                "rdquo" | "ldquo" => Some('"'),
                "copy" => Some('\u{a9}'),
                "deg" => Some('\u{b0}'),
                "times" => Some('x'),
                "raquo" | "laquo" | "uarr" | "darr" | "larr" | "rarr" | "uArr" | "dArr" | "lArr" | "rArr" | "trade" | "reg" => Some(' '),
                _ => name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))
                    .map(|h| u32::from_str_radix(h, 16).ok())
                    .unwrap_or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            };
            c.map(|c| (c, e + 1))
        });
        match decoded {
            Some((c, used)) => {
                out.push(c);
                rest = &rest[used..];
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

enum Token<'a> {
    Text(&'a str),
    Open(String, &'a str),
    Close(String),
}

/// The page as text and tags; comments, the doctype and what stands in scripts and styles are left out.
fn tokens(html: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = html.as_bytes();
    while i < html.len() {
        let Some(lt) = html[i..].find('<').map(|p| p + i) else {
            out.push(Token::Text(&html[i..]));
            break;
        };
        // (a `<` that begins no tag is text)
        let next = bytes.get(lt + 1).copied().unwrap_or(b' ');
        if !(next.is_ascii_alphabetic() || next == b'/' || next == b'!' || next == b'?') {
            out.push(Token::Text(&html[i..=lt]));
            i = lt + 1;
            continue;
        }
        if lt > i {
            out.push(Token::Text(&html[i..lt]));
        }
        if html[lt..].starts_with("<!--") {
            i = html[lt + 4..].find("-->").map_or(html.len(), |p| lt + 4 + p + 3);
            continue;
        }
        let Some(gt) = html[lt..].find('>').map(|p| p + lt) else { break };
        let inside = &html[lt + 1..gt];
        i = gt + 1;
        if let Some(name) = inside.strip_prefix('/') {
            out.push(Token::Close(name.trim().to_ascii_lowercase()));
        } else if !(inside.starts_with('!') || inside.starts_with('?')) {
            let end = inside.find(|c: char| c.is_whitespace() || c == '/').unwrap_or(inside.len());
            let name = inside[..end].to_ascii_lowercase();
            // (a script's or a style's text is not the page's: up to its closing tag)
            if name == "script" || name == "style" {
                let close = format!("</{name}");
                let lower = html[i..].to_ascii_lowercase();
                i = lower.find(&close).map_or(html.len(), |p| i + p);
            }
            out.push(Token::Open(name, &inside[end..]));
        }
    }
    out
}

/// An attribute's value in a tag's attribute text (`class="a b"`, `id=x`), small letters.
fn attribute(attributes: &str, name: &str) -> String {
    let lower = attributes.to_ascii_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find(name).map(|p| p + from) {
        from = at + name.len();
        let before_ok = at == 0 || !lower.as_bytes()[at - 1].is_ascii_alphanumeric() && lower.as_bytes()[at - 1] != b'-';
        let rest = lower[from..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        let value = rest[1..].trim_start();
        return match value.chars().next() {
            Some(q @ ('"' | '\'')) => value[1..].split(q).next().unwrap_or("").to_string(),
            _ => value.split(|c: char| c.is_whitespace() || c == '>').next().unwrap_or("").to_string(),
        };
    }
    String::new()
}

/// Whether an element is one of those around the article, by its class and id.
fn marked(attributes: &str) -> bool {
    let words = format!("{} {}", attribute(attributes, "class"), attribute(attributes, "id"));
    words.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).any(|w| DROPPED_MARKS.contains(&w))
}

/// The part of the page the article is in: its `<article>`, else its `<main>`, else its `<body>`, else all of it.
fn article(html: &str) -> &str {
    let lower = html.to_ascii_lowercase();
    for name in ["article", "main", "body"] {
        let open = format!("<{name}");
        let close = format!("</{name}");
        if let Some(start) = lower.find(&open).filter(|&s| lower[s + open.len()..].starts_with(|c: char| c == '>' || c.is_whitespace())) {
            let end = lower.rfind(&close).filter(|&e| e > start).unwrap_or(html.len());
            return &html[start..end];
        }
    }
    html
}

/// Spaces and line ends made single; with `lines`, line ends kept (at most one empty line in a row).
fn tidy(text: &str, lines: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split('\n') {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if lines {
            if line.is_empty() && (out.is_empty() || out.ends_with("\n\n")) {
                continue;
            }
            out.push_str(&line);
            out.push('\n');
        } else if !line.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&line);
        }
    }
    out.trim().to_string()
}

/// The page's title: its `<title>` up to the site's name after it, else nothing.
fn title_of(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let Some(start) = lower.find("<title").and_then(|s| lower[s..].find('>').map(|g| s + g + 1)) else { return String::new() };
    let end = lower[start..].find("</title").map_or(html.len(), |e| start + e);
    let whole = tidy(&decode_entities(&html[start..end]), false);
    for mark in [" \u{2022} ", " | ", " \u{2013} ", " - "] {
        if let Some((first, _)) = whole.split_once(mark) {
            return first.trim().to_string();
        }
    }
    whole
}

/// Reads a page.
pub fn extract(html: &str, reading: Reading) -> Page {
    let mut page = Page { title: title_of(html), sections: Vec::new() };
    let mut section = Section::default();
    let mut heading: Option<(String, String)> = None;
    // (the element being left out and how many of its name are open inside it)
    let mut dropped: Option<(String, u32)> = None;
    let finish = |section: &mut Section, page: &mut Page| {
        let done = Section { heading: std::mem::take(&mut section.heading), text: tidy(&std::mem::take(&mut section.text), true) };
        if !done.text.is_empty() {
            page.sections.push(done);
        }
    };
    for token in tokens(article(html)) {
        if let Some((name, depth)) = dropped.as_mut() {
            match &token {
                Token::Open(n, _) if n == name => *depth += 1,
                Token::Close(n) if n == name => {
                    *depth -= 1;
                    if *depth == 0 {
                        dropped = None;
                    }
                }
                _ => {}
            }
            continue;
        }
        match token {
            Token::Text(text) => {
                let text = decode_entities(text);
                let text = if reading.keep_newlines { text.replace('\r', "") } else { text.replace(['\r', '\n'], " ") };
                match heading.as_mut() {
                    Some((_, words)) => words.push_str(&text),
                    None => section.text.push_str(&text),
                }
            }
            Token::Open(name, attributes) => {
                let void = VOID.contains(&name.as_str()) || attributes.trim_end().ends_with('/');
                if !void && (DROPPED.contains(&name.as_str()) || marked(attributes)) {
                    dropped = Some((name, 1));
                    continue;
                }
                // (a heading begun by a contents anchor ends with its line)
                if heading.as_ref().is_some_and(|(open, _)| open == ANCHORED) && (BLOCKS.contains(&name.as_str())) {
                    let (_, words) = heading.take().unwrap_or_default();
                    section.heading = tidy(&words, false).replace('\n', " ");
                }
                // (a heading: `<h1>` to `<h6>`, and in the game's help files the headline's colour)
                let headline = name == "font" && attribute(attributes, "color").contains("headline");
                if heading.is_none() && (headline || (name.len() == 2 && name.starts_with('h') && name.as_bytes()[1].is_ascii_digit())) {
                    finish(&mut section, &mut page);
                    heading = Some((name, String::new()));
                    continue;
                }
                // (guides written in a forum's markup have no heading tags: a part begins at the anchor its table of
                // contents points to, `<a name="toc3">`, and its heading is the rest of that line)
                if heading.is_none() && name == "a" && attribute(attributes, "name").starts_with("toc") {
                    finish(&mut section, &mut page);
                    heading = Some((ANCHORED.to_string(), String::new()));
                    continue;
                }
                if BLOCKS.contains(&name.as_str()) {
                    section.text.push('\n');
                    if name == "li" {
                        section.text.push_str("- ");
                    }
                } else if name == "td" || name == "th" {
                    section.text.push(' ');
                }
            }
            Token::Close(name) => {
                if heading.as_ref().is_some_and(|(open, _)| *open == name) {
                    let (_, words) = heading.take().unwrap_or_default();
                    section.heading = tidy(&words, false).replace('\n', " ");
                } else if BLOCKS.contains(&name.as_str()) {
                    section.text.push('\n');
                }
            }
        }
    }
    // (a heading never closed is still text)
    if let Some((_, words)) = heading {
        section.text.push_str(&words);
    }
    finish(&mut section, &mut page);
    if page.title.is_empty() {
        page.title = page.sections.iter().map(|s| s.heading.clone()).find(|h| !h.is_empty()).unwrap_or_default();
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entities_become_their_characters() {
        assert_eq!(decode_entities("Fish &amp; chips &lt;3 &#39;x&#x27; a&nbsp;b &bogus; & alone"), "Fish & chips <3 'x' a b &bogus; & alone");
    }

    #[test]
    fn the_article_is_read_without_what_is_around_it() {
        // (a made-up page in the shape of a guide site's: a menu, an article with a contents box and a picker, a footer)
        let html = r##"<!DOCTYPE html><html><head><title>Planting Turnips &bull; Some Site</title><script>var a = "<p>not text</p>";</script></head>
<body><nav><a href="/">Home</a> <a href="/guides">Guides</a></nav>
<div id="content"><article><h1>Planting Turnips</h1>
<div class="tbox toc goright"><ul><li><a href="#a">Soil</a></li><li><a href="#b">Water</a></li></ul></div>
<div class="tbox"><strong>Level:</strong> 10-20<br><select name="x"><option>one</option><option>two</option></select></div>
<p>Turnips grow in <b>cold</b> soil.<br>Plant them early.</p>
<h2 id="a">Soil</h2><p>Dig twice &amp; rake once.</p>
<ul><li>Sand: poor</li><li>Loam: good</li></ul>
<!-- a comment <p>not text</p> -->
<h2 id="b">Water</h2><table><tr><td>Spring</td><td>daily</td></tr><tr><td>Autumn</td><td>weekly</td></tr></table>
</article></div><footer>Content may not be copied.</footer></body></html>"##;
        let page = extract(html, Reading::default());
        assert_eq!(page.title, "Planting Turnips");
        let headings: Vec<&str> = page.sections.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(headings, ["Planting Turnips", "Soil", "Water"]);
        assert_eq!(page.sections[0].text, "Level: 10-20\n\nTurnips grow in cold soil.\nPlant them early.");
        assert_eq!(page.sections[1].text, "Dig twice & rake once.\n\n- Sand: poor\n\n- Loam: good");
        assert_eq!(page.sections[2].text, "Spring daily\n\nAutumn weekly");
        let all = format!("{page:?}");
        for gone in ["not text", "Home", "one", "may not be copied", "var a"] {
            assert!(!all.contains(gone), "{gone} is in {all}");
        }
    }

    #[test]
    fn a_contents_anchor_begins_a_part() {
        let html = "<article><h1>Kites</h1>Kites fly.<br><br><span style=\"font-size: 18px\"><a name=\"toc0\"></a><div class=\"bbtotop\" onclick=\"x()\">&uArr;</div><span><b>String</b> length</span></span><br><br>Fifty paces.<br><a name=\"toc1\"></a>Wind<br>Steady is best.</article>";
        let page = extract(html, Reading::default());
        assert_eq!(page.sections, vec![
            Section { heading: "Kites".into(), text: "Kites fly.".into() },
            Section { heading: "String length".into(), text: "Fifty paces.".into() },
            Section { heading: "Wind".into(), text: "Steady is best.".into() },
        ]);
    }

    #[test]
    fn a_help_file_keeps_its_lines() {
        // (the shape of the game's help files: a coloured headline, then text whose line ends count)
        let text = "<font color=CCInfoHeadline>Rowing</font><font color=CCInfoText>\r\n\r\nPull both oars.\r\nSee <a href=\"file://Boats.html\">boats</a> too.\r\n\r\n\r\n\r\nRest often.</font>";
        let page = extract(text, Reading { keep_newlines: true });
        assert_eq!(page.title, "Rowing");
        assert_eq!(page.sections, vec![Section { heading: "Rowing".into(), text: "Pull both oars.\nSee boats too.\n\nRest often.".into() }]);
    }

    #[test]
    fn a_page_without_an_article_is_its_body() {
        let page = extract("<html><body><div class=\"sidebar\"><div>Links</div></div><p>One.</p><p>Two < three.</p></body></html>", Reading::default());
        assert_eq!(page.sections, vec![Section { heading: String::new(), text: "One.\n\nTwo < three.".into() }]);
        assert_eq!(page.title, "");
    }
}
