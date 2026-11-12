//! What the items say about making things: a guide drawn from the item templates' own descriptions.
//!
//! The game's files hold no table of tradeskill processes: which two things combine into which, and the skill it
//! takes, is the server's to say, a pair at a time (the tradeskill window asks it). What the files do hold is what
//! Funcom wrote on the items: some thousands of descriptions that say what a thing is for ("If combined with a Space
//! Symbol Library, it will make a Compiled Algorithm (...)", "Use this Emitter on the crystal to create ...", "This is
//! a tradeskill item"). This module reads those:
//!
//! * an item belongs to the guide when its description speaks of making ([`speaks_of_making`]) or when such a
//!   description names it;
//! * the other items a description names are found by their names ([`Guide::build`]: the longest name at each place,
//!   whole words, names of one short word left out as too easily an ordinary word);
//! * a sentence that says what comes of it ("it will make X", "to create X", "end up with X", ...) is a step: the
//!   item itself, the other items the sentence names, and X ([`Step`]). Sentences that name things without saying
//!   what comes of them stay as text with their names linked.
//!
//! Some of the items are books: a "Construction Manual" for a weapon, "Book: How To Create A Nano Crystal", a guide
//! to the medical libraries. Their descriptions are whole instructions, with the tools, the order and often the
//! skills. Those are marked ([`Entry::guide`]) so that they can be read as the guides they are.
//!
//! A name of the form "Symbol Library - Space" is also found where a text says "Space Symbol Library", and where a
//! text says only "Symbol Library" and nothing is called just that, the first of the family stands for it.
//!
//! Things made in several steps often carry what they are for in brackets at the end of their names: "Instruction
//! Disc (Grid Gateway)", "Compiled Algorithm (Grid Gateway)", "Nano Crystal (Grid Gateway)". Entries that share such
//! an ending are each other's kin ([`Guide::kin`]), and an item with the ending of an entry of the guide is in the
//! guide too, though its own description says nothing of making (the crystal at the end of the chain).
//!
//! So the guide is as complete as the descriptions are, and no more: a process nobody wrote on an item is not in it,
//! and no skill requirement is (the descriptions rarely give one). Items of one name (the many qualities of a thing)
//! are one entry.

use std::collections::HashMap;

/// The words that mark a description as speaking of making things.
const MAKING: &[&str] = &["tradeskill", "combine", "to create", "to make", "will make", "used on", "assemble", "can be used to", "use this", "upgrade", "refine", "process"];

/// Whether a description speaks of making things.
pub fn speaks_of_making(description: &str) -> bool {
    let text = description.to_lowercase();
    MAKING.iter().any(|w| text.contains(w))
}

/// The phrases after which a sentence names what is made.
const RESULT: &[&str] = &[
    "it will make",
    "will make",
    "to make",
    "to create",
    "will create",
    "creates",
    "will result in",
    "results in",
    "end up with",
    "to produce",
    "will produce",
    "to form",
    "will become",
    "turn it into",
    "turns it into",
    "made into",
    "converted into",
];

/// A step a description states: with these other things, this is made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// The other things the sentence names (entries of the guide).
    pub with: Vec<usize>,
    /// What is made (an entry of the guide).
    pub makes: usize,
    /// The sentence itself.
    pub said: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// The lowest template of the name, and how many templates have the name.
    pub id: u32,
    pub templates: u32,
    pub name: String,
    /// The description on one line (what is searched for names).
    pub description: String,
    /// The description with its lines, for reading.
    pub text: String,
    /// Whether its own description speaks of making.
    pub speaks: bool,
    /// A book or manual: a long text of instructions.
    pub guide: bool,
    /// The entries its description names, in the order named.
    pub names: Vec<usize>,
    /// The steps its description states.
    pub steps: Vec<Step>,
}

#[derive(Default)]
pub struct Guide {
    pub entries: Vec<Entry>,
    /// For an entry: the entries whose descriptions name it.
    pub named_by: Vec<Vec<usize>>,
    /// For an entry: the steps that make it, as (the entry whose description states it, which of its steps).
    pub made_by: Vec<Vec<(usize, usize)>>,
    /// The entries by what stands in brackets at the end of their names.
    by_subject: HashMap<String, Vec<usize>>,
    /// The books and manuals with their texts in small letters (for [`Guide::books_about`]).
    books: Vec<(usize, String)>,
}

/// A name without what it is for or which of its kind it is: "Instruction Disc (Grid Gateway)" is `Instruction Disc`,
/// "Symbol Library - Space" is `Symbol Library`.
pub fn kind_of(name: &str) -> &str {
    let name = name.trim();
    let end = [name.find(" ("), name.find(" - ")].into_iter().flatten().min().unwrap_or(name.len());
    name[..end].trim()
}

/// What stands in the brackets at the end of a name ("Compiled Algorithm (Grid Gateway)": `grid gateway`), small
/// letters; brackets inside it are its own.
pub fn subject(name: &str) -> Option<String> {
    let name = name.trim();
    if !name.ends_with(')') {
        return None;
    }
    let mut depth = 0;
    for (i, c) in name.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth == 0 {
                    let inside = name[i + 1..name.len() - 1].trim().to_lowercase();
                    return (i > 0 && inside.len() >= 3).then_some(inside);
                }
            }
            _ => {}
        }
    }
    None
}

/// A text without markup: tags out, entities for the common few, white space single.
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if in_tag => {}
            c => out.push(c),
        }
    }
    let out = out.replace("&quot;", "\"").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&#39;", "'");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A text without markup, its line breaks kept (`<br>` and line ends), blank lines single.
pub fn lines(text: &str) -> String {
    let broken = text.replace("<br>", "\n").replace("<BR>", "\n").replace("<br/>", "\n").replace("<br />", "\n").replace('\r', "");
    let mut out: Vec<String> = Vec::new();
    for line in broken.split('\n') {
        let line = plain(line);
        if line.is_empty() && out.last().is_none_or(|l| l.is_empty()) {
            continue;
        }
        out.push(line);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

/// The words in a name that mark a book of instructions, and the least such a text is long.
const BOOK: &[&str] = &["book", "guide", "manual", "engineering news", "how to", "secret revealed", "handbook", "notes", "creation with", "recipe"];
const BOOK_LENGTH: usize = 500;

/// Whether an item is a book of instructions: a long text that speaks of making, under a book's name.
pub fn is_guide(name: &str, description: &str) -> bool {
    let name = name.to_lowercase();
    description.len() >= BOOK_LENGTH && speaks_of_making(description) && BOOK.iter().any(|w| name.contains(w))
}

/// A name as it is matched: small letters, its words apart by single spaces, the punctuation at the words' ends kept
/// out of the way.
fn key(name: &str) -> String {
    name.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether a name is one worth looking for in a text: two words or more, or one long word. A short single word (a
/// profession, "Notum", "Pistol") is too often an ordinary word of a sentence.
fn findable(name: &str) -> bool {
    let words = name.split_whitespace().count();
    (words >= 2 && name.len() >= 8) || name.len() >= 14
}

/// The sentences of a text with where each begins (cut at `.`, `!`, `?` followed by a space, and at line ends).
fn sentences(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for (i, c) in text.char_indices() {
        let end = matches!(c, '.' | '!' | '?') && bytes.get(i + 1).is_none_or(|b| b.is_ascii_whitespace());
        if end || c == '\n' {
            if !text[start..=i].trim().is_empty() {
                out.push((start, &text[start..=i]));
            }
            start = i + 1;
        }
    }
    if start < text.len() && !text[start..].trim().is_empty() {
        out.push((start, &text[start..]));
    }
    out
}

impl Guide {
    /// The guide of some items, each `(template, name, description)`.
    pub fn build(items: impl IntoIterator<Item = (u32, String, String)>) -> Guide {
        // The names, one entry each.
        let mut all: Vec<Entry> = Vec::new();
        let mut by_key: HashMap<String, usize> = HashMap::new();
        for (id, name, description) in items {
            let name = name.trim().to_string();
            if name.is_empty() {
                continue;
            }
            match by_key.get(&key(&name)) {
                Some(&at) => {
                    let e = &mut all[at];
                    e.templates += 1;
                    e.id = e.id.min(id);
                    if e.description.is_empty() {
                        e.description = plain(&description);
                        e.text = lines(&description);
                        e.speaks = speaks_of_making(&e.description);
                        e.guide = is_guide(&e.name, &e.description);
                    }
                }
                None => {
                    by_key.insert(key(&name), all.len());
                    let text = lines(&description);
                    let description = plain(&description);
                    all.push(Entry { id, templates: 1, speaks: speaks_of_making(&description), guide: is_guide(&name, &description), name, description, text, ..Default::default() });
                }
            }
        }
        // The names by their first word, the longest first, for finding them in the texts.
        let mut by_first: HashMap<String, Vec<(usize, Vec<String>)>> = HashMap::new();
        let mut families: Vec<(String, usize)> = Vec::new();
        for (at, e) in all.iter().enumerate() {
            if findable(&e.name) {
                let words: Vec<String> = key(&e.name).split(' ').map(str::to_string).collect();
                by_first.entry(words[0].clone()).or_default().push((at, words));
            }
            // ("Symbol Library - Space" is also "Space Symbol Library"; and the family's name alone, below)
            if let Some((family, kind)) = e.name.split_once(" - ") {
                let turned = format!("{} {}", kind.trim(), family.trim());
                if findable(&turned) && !by_key.contains_key(&key(&turned)) {
                    let words: Vec<String> = key(&turned).split(' ').map(str::to_string).collect();
                    by_first.entry(words[0].clone()).or_default().push((at, words));
                }
                if findable(family.trim()) && !by_key.contains_key(&key(family)) {
                    families.push((key(family), at));
                }
            }
        }
        // (a family's name where nothing is called just that: its first member, by template)
        families.sort_by(|a, b| a.0.cmp(&b.0).then(all[a.1].id.cmp(&all[b.1].id)));
        families.dedup_by(|a, b| a.0 == b.0);
        for (family, at) in families {
            let words: Vec<String> = family.split(' ').map(str::to_string).collect();
            by_first.entry(words[0].clone()).or_default().push((at, words));
        }
        for list in by_first.values_mut() {
            list.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
        }
        // What each speaking description names, and where.
        let mut found: Vec<Vec<(usize, usize, usize)>> = vec![Vec::new(); all.len()];
        for (at, e) in all.iter().enumerate().filter(|(_, e)| e.speaks) {
            let lower = e.description.to_lowercase();
            // (the words with their places; a word's ends are trimmed of what is no letter, digit, bracket or quote)
            let mut words: Vec<(usize, usize)> = Vec::new();
            let mut start = None;
            for (i, c) in lower.char_indices() {
                if c.is_whitespace() {
                    if let Some(s) = start.take() {
                        words.push((s, i));
                    }
                } else if start.is_none() {
                    start = Some(i);
                }
            }
            if let Some(s) = start {
                words.push((s, lower.len()));
            }
            let trimmed = |(s, e): (usize, usize), last: bool| -> &str {
                let w = &lower[s..e];
                if last { w.trim_end_matches(['.', ',', ';', ':', '!', '?']) } else { w }
            };
            let mut k = 0;
            while k < words.len() {
                let first = trimmed(words[k], true);
                let first_whole = &lower[words[k].0..words[k].1];
                let candidates = by_first.get(first_whole).or_else(|| by_first.get(first));
                let mut matched = None;
                if let Some(list) = candidates {
                    for (other, name_words) in list {
                        if *other == at || k + name_words.len() > words.len() {
                            continue;
                        }
                        let fits = name_words.iter().enumerate().all(|(j, w)| trimmed(words[k + j], j + 1 == name_words.len()) == w);
                        if fits {
                            matched = Some((*other, name_words.len()));
                            break;
                        }
                    }
                }
                match matched {
                    Some((other, n)) => {
                        found[at].push((words[k].0, words[k + n - 1].1, other));
                        k += n;
                    }
                    None => k += 1,
                }
            }
        }
        // The guide's entries: those that speak, and those they name.
        let mut keep = vec![false; all.len()];
        for (at, e) in all.iter().enumerate() {
            if e.speaks {
                keep[at] = true;
                for f in &found[at] {
                    keep[f.2] = true;
                }
            }
        }
        // (and the items that end in the same brackets as one of those: the other links of its chain)
        let subjects: std::collections::HashSet<String> = all.iter().enumerate().filter(|(at, e)| keep[*at] && e.speaks).filter_map(|(_, e)| subject(&e.name)).collect();
        for (at, e) in all.iter().enumerate() {
            if !keep[at] && subject(&e.name).is_some_and(|s| subjects.contains(&s)) {
                keep[at] = true;
            }
        }
        let mut place = vec![usize::MAX; all.len()];
        let mut entries = Vec::new();
        for (at, e) in all.iter().enumerate() {
            if keep[at] {
                place[at] = entries.len();
                entries.push(e.clone());
            }
        }
        let mut named_by = vec![Vec::new(); entries.len()];
        let mut made_by = vec![Vec::new(); entries.len()];
        for (at, spans) in found.iter().enumerate() {
            if spans.is_empty() || place[at] == usize::MAX {
                continue;
            }
            let me = place[at];
            let text = entries[me].description.clone();
            let lower = text.to_lowercase();
            let mut names: Vec<usize> = Vec::new();
            for s in spans {
                let other = place[s.2];
                if !names.contains(&other) {
                    names.push(other);
                    named_by[other].push(me);
                }
            }
            entries[me].names = names;
            // The steps: a sentence where a result phrase stands just before a name.
            // (the places are the small-lettered text's; the sentence is quoted from the text as written where the two
            // are of one length, which they are but for a few letters of other alphabets)
            let same = text.len() == lower.len();
            for (begin, sentence) in sentences(&lower) {
                let sentence = if same { &text[begin..begin + sentence.len()] } else { sentence };
                let inside: Vec<&(usize, usize, usize)> = spans.iter().filter(|s| s.0 >= begin && s.1 <= begin + sentence.len()).collect();
                if inside.is_empty() {
                    continue;
                }
                let made = inside.iter().find(|s| {
                    let before = lower[begin..s.0].trim_end();
                    let before = before.strip_suffix(" a").or_else(|| before.strip_suffix(" an")).or_else(|| before.strip_suffix(" the")).unwrap_or(before).trim_end();
                    RESULT.iter().any(|phrase| before.ends_with(phrase))
                });
                if let Some(made) = made {
                    let makes = place[made.2];
                    let mut with: Vec<usize> = Vec::new();
                    for s in &inside {
                        let other = place[s.2];
                        if other != makes && !with.contains(&other) {
                            with.push(other);
                        }
                    }
                    made_by[makes].push((me, entries[me].steps.len()));
                    entries[me].steps.push(Step { with, makes, said: sentence.trim().to_string() });
                }
            }
        }
        let mut by_subject: HashMap<String, Vec<usize>> = HashMap::new();
        for (at, e) in entries.iter().enumerate() {
            if let Some(s) = subject(&e.name) {
                by_subject.entry(s).or_default().push(at);
            }
        }
        let books = entries.iter().enumerate().filter(|(_, e)| e.guide).map(|(at, e)| (at, e.description.to_lowercase())).collect();
        Guide { entries, named_by, made_by, by_subject, books }
    }

    /// The books and manuals whose texts speak of an entry: of it by name, or of its kind (a book on nano crystals
    /// says "Instruction Disc", not which one). By name; the entry itself is left out.
    pub fn books_about(&self, at: usize) -> Vec<usize> {
        let name = self.entries[at].name.to_lowercase();
        let kind = kind_of(&self.entries[at].name).to_lowercase();
        let wanted: Vec<&str> = [name.as_str(), kind.as_str()].into_iter().filter(|w| findable(w)).collect();
        let mut books: Vec<usize> = self.books.iter().filter(|(book, text)| *book != at && wanted.iter().any(|w| text.contains(w))).map(|b| b.0).collect();
        books.sort_by(|a, b| self.entries[*a].name.cmp(&self.entries[*b].name));
        books
    }

    /// The other entries that end in the same brackets as this one, by name.
    pub fn kin(&self, at: usize) -> Vec<usize> {
        let Some(list) = subject(&self.entries[at].name).and_then(|s| self.by_subject.get(&s)) else { return Vec::new() };
        let mut kin: Vec<usize> = list.iter().copied().filter(|&k| k != at).collect();
        kin.sort_by(|a, b| self.entries[*a].name.cmp(&self.entries[*b].name));
        kin
    }

    /// The guide of all the items of a game's database.
    pub fn of_database(db: &crate::resource::ResourceDatabase) -> Guide {
        let ids: Vec<crate::resource::Id> = db.ids().map(|ids| ids.filter_map(Result::ok).filter(|i| i.kind == crate::item::TEMPLATE).collect()).unwrap_or_default();
        Guide::build(ids.into_iter().filter_map(|id| db.get(id).ok().flatten().and_then(|d| crate::item::Item::parse(&d)).map(|i| (id.instance, i.name, i.description))))
    }

    /// The entries whose names hold every word of `query` (any case), those that begin with it first, then by name;
    /// at most `limit`. With `guides_only`, the books and manuals alone; and for them a word may also stand in the
    /// text (a book is found by what it is about).
    pub fn search(&self, query: &str, limit: usize) -> Vec<usize> {
        self.search_in(query, limit, false)
    }

    pub fn search_in(&self, query: &str, limit: usize, guides_only: bool) -> Vec<usize> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() && !guides_only {
            return Vec::new();
        }
        let whole = words.join(" ");
        let mut hits: Vec<(u8, &str, usize)> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !guides_only || e.guide)
            .filter_map(|(at, e)| {
                let name = e.name.to_lowercase();
                // (a book whose name fits comes first: it explains the hundreds of things of the name after it)
                let rank = if words.iter().all(|w| name.contains(w.as_str())) {
                    if e.guide { 0 } else { 1 + u8::from(!name.starts_with(&whole)) }
                } else if e.guide && words.iter().all(|w| name.contains(w.as_str()) || e.description.to_lowercase().contains(w.as_str())) {
                    3
                } else {
                    return None;
                };
                Some((rank, e.name.as_str(), at))
            })
            .collect();
        hits.sort();
        hits.into_iter().take(limit).map(|h| h.2).collect()
    }

    /// The entry of a name (any case), if the guide has it.
    pub fn named(&self, name: &str) -> Option<usize> {
        let wanted = key(name);
        self.entries.iter().position(|e| key(&e.name) == wanted)
    }

    /// The way to an entry, as far as the descriptions say it: each step that makes it, and under each the steps that
    /// make its parts, to `depth` steps down. A line is `(how deep, the entry made, the entry that says so, its step)`.
    pub fn way_to(&self, target: usize, depth: usize) -> Vec<(usize, usize, usize, usize)> {
        let mut out = Vec::new();
        let mut seen = vec![target];
        self.walk(target, 0, depth, &mut seen, &mut out);
        out
    }

    fn walk(&self, target: usize, level: usize, depth: usize, seen: &mut Vec<usize>, out: &mut Vec<(usize, usize, usize, usize)>) {
        if level >= depth {
            return;
        }
        // (one statement of a step is enough: the same step is often written on both of its parts)
        let mut told: Vec<Vec<usize>> = Vec::new();
        for &(by, step) in &self.made_by[target] {
            let s = &self.entries[by].steps[step];
            let mut parts: Vec<usize> = s.with.iter().copied().chain(std::iter::once(by)).collect();
            parts.sort_unstable();
            parts.dedup();
            if told.contains(&parts) {
                continue;
            }
            told.push(parts.clone());
            out.push((level, target, by, step));
            for part in parts {
                if !seen.contains(&part) {
                    seen.push(part);
                    self.walk(part, level + 1, depth, seen, out);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<(u32, String, String)> {
        let item = |id: u32, name: &str, text: &str| (id, name.to_string(), text.to_string());
        vec![
            item(10, "Blank Song Sheet", "Paper with lines. If combined with a Tuning Fork, it will make a Written Melody. This is a tradeskill item."),
            item(11, "Blank Song Sheet", "Paper with lines. If combined with a Tuning Fork, it will make a Written Melody. This is a tradeskill item."),
            item(20, "Tuning Fork", "It hums."),
            item(30, "Written Melody", "A melody on paper. Use this on a Music Box Frame to create a <b>Singing Music Box</b>.<br>This is a tradeskill item."),
            item(40, "Music Box Frame", "An empty box of brass."),
            item(50, "Singing Music Box", "It sings when opened."),
            item(60, "Brass Polish", "Can be used on a Music Box Frame. It smells of lemons."),
            item(70, "Garden Chair", "A chair for the garden."),
            item(80, "Notum", "A mineral."),
            item(90, "Lump", "A lump of Notum; combine it with patience."),
            item(130, "Song Draft (Evening Bells)", "Notes for a song. Combine it with patience to make the song."),
            item(131, "Finished Song (Evening Bells)", "A song about bells."),
            item(132, "Finished Song (Morning Rain)", "A song about rain."),
            item(100, "Reed Pipe - Alto", "A pipe."),
            item(101, "Reed Pipe - Bass", "A pipe."),
            item(110, "Pipe Cleaner Kit", "If combined with an Alto Reed Pipe, it will make a Singing Music Box. Any Reed Pipe can be used to practise."),
            item(120, "Book: How To Build Music Boxes", &format!("First combine the Blank Song Sheet with a Tuning Fork.<br><br>Then the box. {}", "It takes patience to assemble. ".repeat(20))),
        ]
    }

    #[test]
    fn the_descriptions_steps_are_read() {
        let guide = Guide::build(items());
        let at = |name: &str| guide.entries.iter().position(|e| e.name == name).unwrap_or_else(|| panic!("{name} is not in the guide"));
        // (the items that speak of making and those they name; the chair and the one-word mineral are not in it)
        let mut names: Vec<&str> = guide.entries.iter().map(|e| e.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["Blank Song Sheet", "Book: How To Build Music Boxes", "Brass Polish", "Finished Song (Evening Bells)", "Lump", "Music Box Frame", "Pipe Cleaner Kit", "Reed Pipe - Alto", "Singing Music Box", "Song Draft (Evening Bells)", "Tuning Fork", "Written Melody"]);
        // (the song that ends in the draft's brackets is in the guide as its kin; the other song is not)
        assert_eq!(guide.kin(at("Song Draft (Evening Bells)")), vec![at("Finished Song (Evening Bells)")]);
        assert!(guide.kin(at("Tuning Fork")).is_empty());
        assert_eq!(subject("Disc (Deprive Skills (Major))").as_deref(), Some("deprive skills (major)"));
        assert_eq!(subject("Plain Name"), None);
        // (the two sheets are one entry)
        assert_eq!((guide.entries[at("Blank Song Sheet")].templates, guide.entries[at("Blank Song Sheet")].id), (2, 10));
        // (a step: the sheet with the fork makes the melody, in the description's own sentence)
        let sheet = &guide.entries[at("Blank Song Sheet")];
        assert_eq!(sheet.steps, vec![Step { with: vec![at("Tuning Fork")], makes: at("Written Melody"), said: "If combined with a Tuning Fork, it will make a Written Melody.".into() }]);
        // (markup is no part of a name, and a line end closes a sentence)
        let melody = &guide.entries[at("Written Melody")];
        assert_eq!(melody.steps.len(), 1);
        assert_eq!((melody.steps[0].with.clone(), melody.steps[0].makes), (vec![at("Music Box Frame")], at("Singing Music Box")));
        // (named without a result: no step, but the name is linked both ways)
        let polish = &guide.entries[at("Brass Polish")];
        assert!(polish.steps.is_empty());
        assert_eq!(polish.names, vec![at("Music Box Frame")]);
        let mut by = guide.named_by[at("Music Box Frame")].clone();
        by.sort_unstable();
        // (a name with its kind after a dash is found the other way round, and the family's name alone is its first)
        let kit = &guide.entries[at("Pipe Cleaner Kit")];
        assert_eq!(kit.names, vec![at("Reed Pipe - Alto"), at("Singing Music Box")]);
        assert_eq!((kit.steps[0].with.clone(), kit.steps[0].makes), (vec![at("Reed Pipe - Alto")], at("Singing Music Box")));
        // (a book: marked, its lines kept for reading, found by what it speaks of)
        let book = &guide.entries[at("Book: How To Build Music Boxes")];
        assert!(book.guide && book.text.starts_with("First combine the Blank Song Sheet with a Tuning Fork.\n\nThen the box."));
        assert!(!guide.entries[at("Written Melody")].guide);
        assert_eq!(guide.search_in("tuning fork", 10, true), vec![at("Book: How To Build Music Boxes")]);
        assert_eq!(guide.search_in("", 10, true), vec![at("Book: How To Build Music Boxes")]);
        assert_eq!(guide.named("singing  MUSIC box"), Some(at("Singing Music Box")));
        let mut expected = vec![at("Written Melody"), at("Brass Polish")];
        expected.sort_unstable();
        assert_eq!(by, expected);
        // (the lump speaks of combining but names nothing findable)
        assert!(guide.entries[at("Lump")].names.is_empty());
    }

    #[test]
    fn the_way_to_a_thing_goes_back_through_its_parts() {
        let guide = Guide::build(items());
        let at = |name: &str| guide.entries.iter().position(|e| e.name == name).unwrap();
        let way = guide.way_to(at("Singing Music Box"), 4);
        let lines: Vec<(usize, &str, &str)> = way.iter().map(|&(level, made, by, _)| (level, guide.entries[made].name.as_str(), guide.entries[by].name.as_str())).collect();
        assert_eq!(lines, vec![(0, "Singing Music Box", "Written Melody"), (1, "Written Melody", "Blank Song Sheet"), (0, "Singing Music Box", "Pipe Cleaner Kit")]);
        assert!(guide.way_to(at("Tuning Fork"), 4).is_empty());
        // (a search by a part of the name; the one that begins with it first)
        let found: Vec<&str> = guide.search("music box", 10).into_iter().map(|k| guide.entries[k].name.as_str()).collect();
        assert_eq!(found, vec!["Book: How To Build Music Boxes", "Music Box Frame", "Singing Music Box"]);
        // (the books that speak of a thing, by its name or its kind)
        assert_eq!(guide.books_about(at("Blank Song Sheet")), vec![at("Book: How To Build Music Boxes")]);
        assert!(guide.books_about(at("Brass Polish")).is_empty());
        assert_eq!((kind_of("Instruction Disc (Grid Gateway)"), kind_of("Symbol Library - Space"), kind_of("Plain Name")), ("Instruction Disc", "Symbol Library", "Plain Name"));
        assert!(guide.search("", 10).is_empty());
    }

    #[test]
    fn texts_are_made_plain_and_cut_into_sentences() {
        assert_eq!(plain("A <font color=#fff>bright</font>   thing &amp; more.<br>Next"), "A bright thing & more. Next");
        assert_eq!(lines("One <b>line</b>.<br><br><br>Two\r\nThree\n\n"), "One line .\n\nTwo\nThree");
        assert!(is_guide("Rusty Bolter Construction Manual", &"Combine the parts. ".repeat(40)));
        assert!(!is_guide("Instruction Disc (Song)", &"Combine the parts. ".repeat(40)));
        assert!(!is_guide("Book: A Short Note", "Combine the parts."));
        let cut: Vec<&str> = sentences("One thing. Two things! Is it 2.5 long?\nLast").into_iter().map(|s| s.1.trim()).collect();
        assert_eq!(cut, vec!["One thing.", "Two things!", "Is it 2.5 long?", "Last"]);
        assert!(speaks_of_making("This is a Tradeskill item."));
        assert!(!speaks_of_making("A chair for the garden."));
    }
}
