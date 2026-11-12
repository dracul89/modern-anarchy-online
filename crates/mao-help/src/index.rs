//! The index: the pages cut into pieces of a few sentences, and the pieces ranked for a question by the words they share
//! with it (BM25, the measure the search engines of the 2000s used: a word counts for more the rarer it is among the
//! pieces and the more often it stands in a short one). Everything is in memory: a few thousand pages are a few
//! megabytes of text and are indexed in well under a second, so nothing but the pages themselves is kept on disk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::text::Section;

/// A page as it is kept: where it is from and what it says.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Doc {
    /// The source's name (`game`, `ao-universe`, ...).
    pub source: String,
    /// The page's address: what a result links to.
    pub url: String,
    pub title: String,
    /// When it was fetched (seconds since 1970), and the site's own date for it then (a sitemap's `lastmod`).
    #[serde(default)]
    pub fetched: u64,
    #[serde(default)]
    pub stamp: String,
    pub sections: Vec<Section>,
}

/// A piece of a page: what a question is matched against, and what a language model is given to answer from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub doc: usize,
    pub heading: String,
    pub text: String,
}

/// A piece shorter than this is a scrap.
pub const SCRAP: usize = 20;

/// The least a result's match is worth, as a part of the best result's.
pub const WORTH: f32 = 0.3;

/// The longest a piece is, in characters (about 150 words: several fit into a small model's context).
pub const CHUNK_CHARS: usize = 900;

/// Words that say nothing about what a question is about.
const STOP: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "to", "in", "on", "at", "for", "is", "are", "was", "be", "it", "this", "that", "with", "as", "by", "from", "do", "does", "i", "you", "my", "me", "can", "how", "what", "where", "which", "when", "there", "have", "has", "if", "so", "not", "but", "will", "your", "get", "its",
];

/// A text's words for the index: small letters, digits kept, a plural's `s` dropped, the empty words left out.
pub fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .map(|w| w.trim_matches('\'').to_lowercase())
        .map(|w| w.strip_suffix("'s").map(str::to_string).unwrap_or(w))
        .filter(|w| !w.is_empty() && !STOP.contains(&w.as_str()))
        .map(|w| if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") && !w.ends_with("us") { w[..w.len() - 1].to_string() } else { w })
        .collect()
}

/// A section's text in pieces of at most `limit` characters: whole paragraphs together while they fit, a longer
/// paragraph cut at its sentences (and a sentence longer than that at a space).
pub fn pieces(text: &str, limit: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let push = |part: &str, current: &mut String, out: &mut Vec<String>| {
        if !current.is_empty() && current.chars().count() + 1 + part.chars().count() > limit {
            out.push(std::mem::take(current));
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(part);
    };
    for paragraph in text.split('\n').map(str::trim).filter(|p| !p.is_empty()) {
        if paragraph.chars().count() <= limit {
            push(paragraph, &mut current, &mut out);
            continue;
        }
        // (a long paragraph: its sentences, and what is still too long by its words)
        let mut sentence = String::new();
        for word in paragraph.split(' ') {
            if sentence.chars().count() + 1 + word.chars().count() > limit {
                push(&std::mem::take(&mut sentence), &mut current, &mut out);
            }
            if !sentence.is_empty() {
                sentence.push(' ');
            }
            sentence.push_str(word);
            if word.ends_with(['.', '!', '?']) && sentence.chars().count() > limit / 2 {
                push(&std::mem::take(&mut sentence), &mut current, &mut out);
            }
        }
        if !sentence.is_empty() {
            push(&sentence, &mut current, &mut out);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// A result: the piece, its page, how well it matched and a few lines of it around the question's words.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub source: String,
    pub url: String,
    pub title: String,
    pub heading: String,
    pub text: String,
    pub snippet: String,
    pub score: f32,
}

/// The pages and the words of their pieces.
#[derive(Default)]
pub struct Index {
    pub docs: Vec<Doc>,
    pub chunks: Vec<Chunk>,
    /// For a word: the pieces it stands in and how often.
    postings: HashMap<String, Vec<(u32, u16)>>,
    lengths: Vec<u32>,
    average: f32,
}

impl Index {
    pub fn build(docs: Vec<Doc>) -> Index {
        let mut index = Index { docs, ..Default::default() };
        for (d, doc) in index.docs.iter().enumerate() {
            // (a few words alone, a link back to a list of pages, say nothing: left out where the page has more)
            let has_more = doc.sections.iter().any(|s| s.text.chars().count() >= SCRAP);
            for section in &doc.sections {
                for text in pieces(&section.text, CHUNK_CHARS) {
                    if has_more && text.chars().count() < SCRAP {
                        continue;
                    }
                    let n = index.chunks.len() as u32;
                    // (the title's and the heading's words count as the piece's own, twice: a page about a thing
                    // names it there)
                    let mut all = words(&text);
                    for _ in 0..2 {
                        all.extend(words(&doc.title));
                        all.extend(words(&section.heading));
                    }
                    let mut counts: HashMap<String, u16> = HashMap::new();
                    for w in &all {
                        *counts.entry(w.clone()).or_default() += 1;
                    }
                    for (w, c) in counts {
                        index.postings.entry(w).or_default().push((n, c));
                    }
                    index.lengths.push(all.len() as u32);
                    index.chunks.push(Chunk { doc: d, heading: section.heading.clone(), text });
                }
            }
        }
        index.average = if index.lengths.is_empty() { 1.0 } else { index.lengths.iter().sum::<u32>() as f32 / index.lengths.len() as f32 };
        index
    }

    /// The best pieces for a question, best first: at most `limit`, at most two of one page, and none that matches
    /// less than [`WORTH`] as well as the best (a piece that only shares a common word with the question).
    pub fn search(&self, question: &str, limit: usize) -> Vec<Hit> {
        const K1: f32 = 1.2;
        const B: f32 = 0.75;
        let asked = words(question);
        let mut scores: HashMap<u32, f32> = HashMap::new();
        let total = self.chunks.len() as f32;
        for word in &asked {
            let Some(list) = self.postings.get(word) else { continue };
            let rarity = ((total - list.len() as f32 + 0.5) / (list.len() as f32 + 0.5) + 1.0).ln();
            for &(chunk, count) in list {
                let count = count as f32;
                let length = self.lengths[chunk as usize] as f32 / self.average;
                *scores.entry(chunk).or_default() += rarity * count * (K1 + 1.0) / (count + K1 * (1.0 - B + B * length));
            }
        }
        let mut ranked: Vec<(u32, f32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut per_doc: HashMap<usize, u32> = HashMap::new();
        let mut out = Vec::new();
        let best = ranked.first().map_or(0.0, |r| r.1);
        for (chunk, score) in ranked {
            if score < best * WORTH {
                break;
            }
            let c = &self.chunks[chunk as usize];
            let seen = per_doc.entry(c.doc).or_default();
            if *seen >= 2 {
                continue;
            }
            *seen += 1;
            let doc = &self.docs[c.doc];
            out.push(Hit { source: doc.source.clone(), url: doc.url.clone(), title: doc.title.clone(), heading: c.heading.clone(), text: c.text.clone(), snippet: snippet(&c.text, &asked, 240), score });
            if out.len() >= limit {
                break;
            }
        }
        out
    }
}

/// About `width` characters of a text where most of the asked words stand, cut at spaces, with `...` where it is cut.
pub fn snippet(text: &str, asked: &[String], width: usize) -> String {
    let flat = text.replace('\n', " ");
    let chars: Vec<char> = flat.chars().collect();
    if chars.len() <= width {
        return flat;
    }
    // (where the asked words are, by character)
    let mut marks: Vec<usize> = Vec::new();
    let mut at = 0;
    for word in flat.split(' ') {
        if words(word).iter().any(|w| asked.contains(w)) {
            marks.push(at);
        }
        at += word.chars().count() + 1;
    }
    // (the window with the most of them)
    let start = marks.iter().map(|&m| (marks.iter().filter(|&&o| o >= m && o < m + width).count(), std::cmp::Reverse(m))).max().map_or(0, |(_, m)| m.0);
    let start = start.saturating_sub(width / 6);
    let mut from = start;
    while from > 0 && from < chars.len() && chars[from - 1] != ' ' {
        from -= 1;
    }
    let mut to = (from + width).min(chars.len());
    while to < chars.len() && to > from && chars[to - 1] != ' ' {
        to -= 1;
    }
    let body: String = chars[from..to].iter().collect();
    format!("{}{}{}", if from > 0 { "... " } else { "" }, body.trim(), if to < chars.len() { " ..." } else { "" })
}

/// The pages kept on the player's machine: one file a source, a page a line (`<name>.jsonl` in the help folder).
pub struct Store {
    pub dir: PathBuf,
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Store {
        Store { dir: dir.into() }
    }

    fn file(&self, source: &str) -> PathBuf {
        let name: String = source.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        self.dir.join(format!("{name}.jsonl"))
    }

    /// The sources that have pages kept.
    pub fn sources(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.dir)
            .map(|entries| entries.flatten().filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".jsonl")).map(str::to_string)).collect())
            .unwrap_or_default();
        names.sort();
        names
    }

    pub fn load(&self, source: &str) -> Vec<Doc> {
        let text = std::fs::read_to_string(self.file(source)).unwrap_or_default();
        text.lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
    }

    pub fn save(&self, source: &str, docs: &[Doc]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let mut text = String::new();
        for doc in docs {
            text += &serde_json::to_string(doc).map_err(std::io::Error::other)?;
            text.push('\n');
        }
        // (written whole beside the old file, then put in its place: a crawl stopped midway loses nothing kept)
        let file = self.file(source);
        let fresh = file.with_extension("jsonl.new");
        std::fs::write(&fresh, text)?;
        std::fs::rename(&fresh, &file)
    }

    /// Forgets a source's pages.
    pub fn forget(&self, source: &str) -> std::io::Result<()> {
        match std::fs::remove_file(self.file(source)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// The folder the help pages are kept in when the client does not say: `help` beside the settings (`MAO_SETTINGS`'
/// folder, else the system's place for this client's settings).
pub fn default_dir() -> PathBuf {
    let settings = std::env::var_os("MAO_SETTINGS").map(PathBuf::from).and_then(|p| p.parent().map(Path::to_path_buf));
    let base = settings.or_else(|| std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("ModernAnarchyOnline"))).or_else(|| {
        let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
        config.map(|c| c.join("ModernAnarchyOnline"))
    });
    base.unwrap_or_else(|| PathBuf::from(".")).join("help")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(url: &str, title: &str, sections: &[(&str, &str)]) -> Doc {
        Doc { source: "test".into(), url: url.into(), title: title.into(), fetched: 0, stamp: String::new(), sections: sections.iter().map(|(h, t)| Section { heading: h.to_string(), text: t.to_string() }).collect() }
    }

    #[test]
    fn words_are_small_and_singular() {
        assert_eq!(words("How do I plant the Turnips in Bob's garden? Level 20 moss, glass"), ["plant", "turnip", "bob", "garden", "level", "20", "moss", "glass"]);
    }

    #[test]
    fn a_text_is_cut_at_paragraphs_then_sentences() {
        assert_eq!(pieces("one two\n\nthree four\nfive", 16), ["one two", "three four\nfive"]);
        let long = "Aaa bbb ccc. Ddd eee fff. Ggg hhh iii. Jjj kkk lll.";
        let parts = pieces(long, 26);
        assert!(parts.iter().all(|p| p.chars().count() <= 26), "{parts:?}");
        assert_eq!(parts.join(" "), long);
        assert!(parts.len() >= 2);
    }

    #[test]
    fn the_page_about_the_thing_asked_comes_first() {
        let index = Index::build(vec![
            doc("u/turnips", "Planting Turnips", &[("Soil", "Turnips want cold loam. Dig twice and rake once before planting."), ("Water", "Water daily in spring.")]),
            doc("u/boats", "Rowing a Boat", &[("", "Pull both oars together. A boat does not need soil, only water under it.")]),
            doc("u/bread", "Baking Bread", &[("", "Flour, water, salt and time. Knead until smooth.")]),
        ]);
        assert_eq!(index.chunks.len(), 4);
        let hits = index.search("what soil do turnips need?", 5);
        assert_eq!(hits[0].url, "u/turnips");
        assert_eq!(hits[0].heading, "Soil");
        // (the boat's page names soil once: found, but after)
        assert!(hits.iter().any(|h| h.url == "u/boats"));
        assert!(hits.iter().all(|h| h.url != "u/bread"));
        assert!(index.search("zeppelin", 5).is_empty());
        // (a word that every page has says little: the rarer one decides)
        assert_eq!(index.search("water oars", 1)[0].url, "u/boats");
    }

    #[test]
    fn a_scrap_is_left_out_where_the_page_has_more() {
        let index = Index::build(vec![doc("u/a", "Moss", &[("", "Moss grows on the north side of stones."), ("", "Main Index")]), doc("u/b", "Moss", &[("", "Short.")])]);
        assert_eq!(index.chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(), ["Moss grows on the north side of stones.", "Short."]);
    }

    #[test]
    fn a_page_gives_two_pieces_at_most() {
        let sections: Vec<(String, String)> = (0..6).map(|n| (format!("Part {n}"), "Moss grows on the north side of the moss stone.".to_string())).collect();
        let refs: Vec<(&str, &str)> = sections.iter().map(|(h, t)| (h.as_str(), t.as_str())).collect();
        let index = Index::build(vec![doc("u/moss", "Moss", &refs), doc("u/other", "Stones", &[("", "A moss stone is a stone.")])]);
        let hits = index.search("moss", 10);
        assert_eq!(hits.iter().filter(|h| h.url == "u/moss").count(), 2);
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn a_snippet_is_cut_around_the_asked_words() {
        let filler = "lorem ipsum dolor sit amet ".repeat(20);
        let text = format!("{filler}the turnip wants cold loam and a turnip wants water {filler}");
        let s = snippet(&text, &words("turnip loam"), 80);
        assert!(s.starts_with("... ") && s.ends_with(" ..."), "{s}");
        assert!(s.contains("turnip wants cold loam"), "{s}");
        assert!(s.chars().count() <= 80 + 8);
        assert_eq!(snippet("short text", &words("text"), 80), "short text");
    }

    #[test]
    fn pages_are_kept_a_line_each() {
        let dir = std::env::temp_dir().join(format!("mao-help-store-{}", std::process::id()));
        let store = Store::new(&dir);
        let docs = vec![doc("u/a", "A", &[("h", "text one")]), doc("u/b", "B \"quoted\"\nline", &[("", "text\ntwo")])];
        store.save("my site", &docs).unwrap();
        assert_eq!(store.sources(), ["my_site"]);
        assert_eq!(store.load("my site"), docs);
        assert!(store.load("none").is_empty());
        store.forget("my site").unwrap();
        store.forget("my site").unwrap();
        assert!(store.sources().is_empty());
        let _ = std::fs::remove_dir(&dir);
    }
}
