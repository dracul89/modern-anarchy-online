//! The chat filter: `/filter list | add <pattern> | del <number> | enable | disable | clear`. GUI.dll's, read on
//! 2026-11-12:
//!
//! * The rules are texts kept under numbers in the `ChatFilterRules` value, and `ChatFilterEnabled` switches them
//!   (FUN_100b933d, the command: its answers are "Added as %d.", "Deleted.", "Enabled.", "Disabled.", "Cleared.",
//!   and the list's "%3d: %s").
//! * A line of a chat group (a channel's, the vicinity's) whose text one of the rules matches is not shown
//!   (FUN_100854a8, called by `HandleGroupMessage` and `HandleVicinityMessage`: a `RegExp` of each rule is tried on
//!   the message's text). Tells are not looked at.
//! * `RegExp` is Utils.dll's, and is Henry Spencer's regular expressions of 1986 (its compiled program begins with
//!   that library's mark, 0234): `.`, `[...]` and `[^...]` with ranges, `*`, `+`, `?`, `( )`, `|`, `^`, `$`, and `\`
//!   before a character to take it as it is. Nothing else is special, and the case of letters counts. A pattern is
//!   looked for anywhere in the text unless it is anchored.
//!
//! Here the rules are a file beside the settings (`chatfilter.ini`). A rule's number is the count of rules when it
//! is added, as the original's; where that number is taken already (after a deletion) the original adds a second
//! text under it, which is never listed or tried again, and this client takes the next free number instead
//! (`docs/ORIGINAL_CODE_QUESTIONS.md`, 14). A pattern that does not compile matches nothing.

/// A compiled pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    branches: Vec<Vec<Piece>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Piece {
    Char(u8),
    Any,
    /// The characters of a `[...]`, and whether it is `[^...]`.
    Set(Vec<u8>, bool),
    Start,
    End,
    Group(Pattern),
    /// An atom repeated: at least this many times, and at most one time or any number.
    Repeat(Box<Piece>, usize, bool),
}

struct Parser<'a> {
    text: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    /// `reg`: branches between `|`, up to the end or a closing bracket.
    fn pattern(&mut self, depth: usize) -> Option<Pattern> {
        let mut branches = vec![self.branch(depth)?];
        while self.peek() == Some(b'|') {
            self.at += 1;
            branches.push(self.branch(depth)?);
        }
        Some(Pattern { branches })
    }

    fn branch(&mut self, depth: usize) -> Option<Vec<Piece>> {
        let mut pieces = Vec::new();
        while let Some(c) = self.peek() {
            if c == b'|' || (c == b')' && depth > 0) {
                break;
            }
            let atom = self.atom(depth)?;
            let piece = match self.peek() {
                Some(b'*') => Piece::Repeat(Box::new(atom), 0, true),
                Some(b'+') => Piece::Repeat(Box::new(atom), 1, true),
                Some(b'?') => Piece::Repeat(Box::new(atom), 0, false),
                _ => {
                    pieces.push(atom);
                    continue;
                }
            };
            self.at += 1;
            // ("nested *?+")
            if matches!(self.peek(), Some(b'*' | b'+' | b'?')) {
                return None;
            }
            pieces.push(piece);
        }
        Some(pieces)
    }

    fn atom(&mut self, depth: usize) -> Option<Piece> {
        let c = self.peek()?;
        self.at += 1;
        Some(match c {
            b'^' => Piece::Start,
            b'$' => Piece::End,
            b'.' => Piece::Any,
            b'[' => {
                let negated = self.peek() == Some(b'^');
                if negated {
                    self.at += 1;
                }
                let mut set = Vec::new();
                // (a `]` or `-` straight after the opening is itself)
                if let Some(first @ (b']' | b'-')) = self.peek() {
                    set.push(first);
                    self.at += 1;
                }
                loop {
                    let c = self.peek()?;
                    self.at += 1;
                    match c {
                        b']' => break,
                        // (a range from the character before to the one after; a `-` last is itself)
                        b'-' if self.peek().is_some_and(|n| n != b']') && !set.is_empty() => {
                            let (from, to) = (*set.last()?, self.peek()?);
                            self.at += 1;
                            if from > to {
                                return None;
                            }
                            set.extend(from..=to);
                        }
                        c => set.push(c),
                    }
                }
                Piece::Set(set, negated)
            }
            b'(' => {
                let inner = self.pattern(depth + 1)?;
                if self.peek() != Some(b')') {
                    return None;
                }
                self.at += 1;
                Piece::Group(inner)
            }
            // ("?+* follows nothing"; an unmatched `)`; a `\` at the end)
            b'*' | b'+' | b'?' | b')' | b'|' => return None,
            b'\\' => {
                let c = self.peek()?;
                self.at += 1;
                Piece::Char(c)
            }
            c => Piece::Char(c),
        })
    }
}

impl Pattern {
    /// The pattern compiled, or `None` when it is not one (`regcomp`'s failures).
    pub fn new(text: &str) -> Option<Pattern> {
        let mut parser = Parser { text: text.as_bytes(), at: 0 };
        let pattern = parser.pattern(0)?;
        (parser.at == parser.text.len()).then_some(pattern)
    }

    /// Whether the pattern is found anywhere in the text (`RegExp::Compare`).
    pub fn matches(&self, text: &str) -> bool {
        let text = text.as_bytes();
        (0..=text.len()).any(|start| self.at(text, start, &mut |_| true))
    }

    fn at(&self, text: &[u8], pos: usize, then: &mut dyn FnMut(usize) -> bool) -> bool {
        self.branches.iter().any(|b| pieces(b, text, pos, then))
    }
}

/// The pieces matched one after the other from `pos`, and then `then` on where they end.
fn pieces(list: &[Piece], text: &[u8], pos: usize, then: &mut dyn FnMut(usize) -> bool) -> bool {
    let Some((first, rest)) = list.split_first() else { return then(pos) };
    match first {
        Piece::Repeat(atom, least, many) => repeat(atom, *least, *many, rest, text, pos, then),
        piece => one(piece, text, pos, &mut |end| pieces(rest, text, end, then)),
    }
}

/// One piece that is not a repeat.
fn one(piece: &Piece, text: &[u8], pos: usize, then: &mut dyn FnMut(usize) -> bool) -> bool {
    let c = text.get(pos).copied();
    match piece {
        Piece::Char(want) => c == Some(*want) && then(pos + 1),
        Piece::Any => c.is_some() && then(pos + 1),
        Piece::Set(set, negated) => c.is_some_and(|c| set.contains(&c) != *negated) && then(pos + 1),
        Piece::Start => pos == 0 && then(pos),
        Piece::End => pos == text.len() && then(pos),
        Piece::Group(inner) => inner.at(text, pos, then),
        Piece::Repeat(atom, least, many) => repeat(atom, *least, *many, &[], text, pos, then),
    }
}

/// An atom as often as it will go (and at least `least` times), fewer when what follows needs it.
fn repeat(atom: &Piece, least: usize, many: bool, rest: &[Piece], text: &[u8], pos: usize, then: &mut dyn FnMut(usize) -> bool) -> bool {
    // (once more first, unless that took nothing; then without)
    let more = |then: &mut dyn FnMut(usize) -> bool| {
        one(atom, text, pos, &mut |end| {
            if end == pos {
                return false;
            }
            if many { repeat(atom, least.saturating_sub(1), true, rest, text, end, then) } else { pieces(rest, text, end, then) }
        })
    };
    if more(then) {
        return true;
    }
    least == 0 && pieces(rest, text, pos, then)
}

/// The rules and their switch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub enabled: bool,
    /// Each rule's number and text, in the order they were added.
    pub rules: Vec<(u32, String)>,
}

impl Filter {
    /// `/filter add`: the rule's number.
    pub fn add(&mut self, rule: &str) -> u32 {
        let mut number = self.rules.len() as u32;
        while self.rules.iter().any(|r| r.0 == number) {
            number += 1;
        }
        self.rules.push((number, rule.to_string()));
        number
    }

    /// `/filter del` (the original says "Deleted." whether there was such a rule or not).
    pub fn delete(&mut self, number: u32) {
        self.rules.retain(|r| r.0 != number);
    }

    /// Whether a group's line with this text is hidden.
    pub fn hides(&self, text: &str) -> bool {
        self.enabled && self.rules.iter().any(|(_, rule)| Pattern::new(rule).is_some_and(|p| p.matches(text)))
    }

    pub fn to_text(&self) -> String {
        let mut out = format!("; The chat filter (/filter): lines of chat groups a rule matches are not shown.\nenabled = {}\n", self.enabled);
        for (number, rule) in &self.rules {
            out += &format!("{number} = {rule}\n");
        }
        out
    }

    pub fn from_text(text: &str) -> Filter {
        let mut filter = Filter::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once(" = ") else { continue };
            match key.trim().parse::<u32>() {
                Ok(number) => filter.rules.push((number, value.to_string())),
                Err(_) if key.trim() == "enabled" => filter.enabled = value.trim() == "true",
                Err(_) => {}
            }
        }
        filter
    }

    pub fn path() -> std::path::PathBuf {
        crate::settings::Settings::path().with_file_name("chatfilter.ini")
    }

    pub fn load() -> Filter {
        std::fs::read_to_string(Self::path()).map(|t| Filter::from_text(&t)).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, self.to_text());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(pattern: &str, text: &str) -> bool {
        Pattern::new(pattern).unwrap_or_else(|| panic!("{pattern} does not compile")).matches(text)
    }

    #[test]
    fn patterns_are_the_old_regular_expressions() {
        // (found anywhere, case counting)
        assert!(found("credits", "selling credits cheap"));
        assert!(!found("Credits", "selling credits cheap"));
        assert!(found("^WTS", "WTS a pistol") && !found("^WTS", "hey, WTS a pistol"));
        assert!(found("cheap$", "pistols cheap") && !found("cheap$", "cheap pistols"));
        assert!(found("^$", "") && !found("^$", "x"));
        // (any character, sets, ranges, and sets turned round)
        assert!(found("w.s", "wts") && !found("w.s", "ws"));
        assert!(found("[Ww][Tt][Ss]", "wTs") && !found("[Ww][Tt][Ss]", "wtb"));
        assert!(found("[0-9]+k", "for 500k each") && !found("[0-9]+k", "for k each"));
        assert!(found("x[^0-9]y", "xay") && !found("x[^0-9]y", "x1y"));
        assert!(found("[]a-]+$", "a]-") && found("[a-]", "-"));
        // (repeats give back what the rest needs; one or none; groups and choices)
        assert!(found("^a.*c$", "abcabc") && !found("^a.*c$", "abcab"));
        assert!(found("^ab?c$", "ac") && found("^ab?c$", "abc") && !found("^ab?c$", "abbc"));
        assert!(found("^(ab)+$", "ababab") && !found("^(ab)+$", "ababa"));
        assert!(found("^(WTS|WTB) ", "WTB a ring") && !found("^(WTS|WTB) ", "WTT a ring"));
        assert!(found("^(a*)*b", "aab"));
        // (a backslash takes the next character as it is; braces and digits classes are not special)
        assert!(found("3\\.5", "3.5") && !found("3\\.5", "325"));
        assert!(found("a{2}", "a{2}") && !found("\\d", "5") && found("\\d", "d"));
        // (what is not a pattern)
        for bad in ["*a", "a**", "(a", "a)", "[a", "a\\", "[z-a]", "a|*"] {
            assert!(Pattern::new(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn rules_hide_when_the_filter_is_on() {
        let mut f = Filter::default();
        assert_eq!((f.add("credits"), f.add("^WTS"), f.add("(")), (0, 1, 2));
        assert!(!f.hides("selling credits"));
        f.enabled = true;
        assert!(f.hides("selling credits") && f.hides("WTS things") && !f.hides("hello"));
        // (a deleted rule's number is not given twice while a later one holds the count)
        f.delete(0);
        assert!(!f.hides("selling credits"));
        assert_eq!(f.add("spam"), 3);
        assert_eq!(f.rules.iter().map(|r| r.0).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(Filter::from_text(&f.to_text()), f);
        assert_eq!(Filter::from_text(""), Filter::default());
    }
}
