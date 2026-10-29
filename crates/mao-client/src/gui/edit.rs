//! Editing a text view's text: `TextRenderer_c::KeyDown` (0x10164224), `TextInput` (0x10163496), `MoveCursorLeft`
//! (0x10164000) / `MoveCursorRight` (0x10164151), `DeleteSelection` (0x10160212), `BeginSelection` / `ExpandSelection` /
//! `ClearSelection`. The cursor is a place in the text (a byte offset on a character boundary), the selection the text
//! between it and an anchor.
//!
//! The original keeps the text as markup and escapes `<`, `>` and `&` in what is typed (`String::Escape`); here an editable
//! view's text is plain characters, which is what that escaping amounts to. Home and End go to the start and end of the
//! line between new lines (the original's to those of the wrapped line).

use super::host::{Key, Mods};
use super::text::flags;

/// The cursor and the selection's other end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Caret {
    pub cursor: usize,
    pub anchor: Option<usize>,
}

/// What a key or typing did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    None,
    /// The cursor or the selection moved.
    Moved,
    /// The text changed (`SlotEdited`).
    Changed,
    /// Enter (`SlotEnterPressed`).
    Enter,
}

impl Caret {
    /// The selection (start, end), when there is one (`GetSelectionRange`).
    pub fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        (a != self.cursor).then(|| (a.min(self.cursor), a.max(self.cursor)))
    }

    /// Keeps the cursor and the anchor on the text's character boundaries.
    pub fn clamp(&mut self, text: &str) {
        let fix = |mut i: usize| {
            i = i.min(text.len());
            while !text.is_char_boundary(i) {
                i -= 1;
            }
            i
        };
        self.cursor = fix(self.cursor);
        self.anchor = self.anchor.map(fix);
    }
}

fn prev_char(text: &str, i: usize) -> usize {
    text[..i].char_indices().next_back().map_or(0, |(k, _)| k)
}

fn next_char(text: &str, i: usize) -> usize {
    text[i..].chars().next().map_or(i, |c| i + c.len_utf8())
}

/// The start of the word before `i` (Ctrl+Left, Ctrl+Backspace): back over white space, then over the word.
pub fn word_left(text: &str, mut i: usize) -> usize {
    while i > 0 && text[..i].chars().next_back().is_some_and(char::is_whitespace) {
        i = prev_char(text, i);
    }
    while i > 0 && text[..i].chars().next_back().is_some_and(|c| !c.is_whitespace()) {
        i = prev_char(text, i);
    }
    i
}

/// The start of the word after `i` (Ctrl+Right): over the word, then over white space.
pub fn word_right(text: &str, mut i: usize) -> usize {
    while i < text.len() && text[i..].chars().next().is_some_and(|c| !c.is_whitespace()) {
        i = next_char(text, i);
    }
    while i < text.len() && text[i..].chars().next().is_some_and(char::is_whitespace) {
        i = next_char(text, i);
    }
    i
}

/// `DeleteSelection`: the selected text taken out, the cursor at its start; whether there was any.
fn delete_selection(text: &mut String, caret: &mut Caret) -> bool {
    match caret.selection() {
        Some((a, b)) => {
            text.replace_range(a..b, "");
            *caret = Caret { cursor: a, anchor: None };
            true
        }
        None => {
            caret.anchor = None;
            false
        }
    }
}

/// `TextInput`: typed or pasted text in place of the selection (only its digits in a `TVF_NUMERIC` view; control
/// characters other than a new line in a multi-line one dropped).
pub fn insert(text: &mut String, caret: &mut Caret, typed: &str, feature: u32) -> Edit {
    caret.clamp(text);
    let multiline = feature & flags::MULTILINE != 0;
    let typed: String = typed
        .chars()
        .filter(|c| if feature & flags::NUMERIC != 0 { c.is_ascii_digit() } else { !c.is_control() || (*c == '\n' && multiline) })
        .collect();
    let deleted = delete_selection(text, caret);
    if typed.is_empty() {
        return if deleted { Edit::Changed } else { Edit::None };
    }
    text.insert_str(caret.cursor, &typed);
    caret.cursor += typed.len();
    Edit::Changed
}

/// The start and end of the line the cursor is on (between new lines).
fn line_bounds(text: &str, i: usize) -> (usize, usize) {
    let start = text[..i].rfind('\n').map_or(0, |k| k + 1);
    let end = text[i..].find('\n').map_or(text.len(), |k| i + k);
    (start, end)
}

/// A move of the cursor to `to`: with Shift the selection grows from where it was (`BeginSelection`, `ExpandSelection`),
/// without it the selection goes (`ClearSelection`).
fn move_to(caret: &mut Caret, to: usize, extend: bool) -> Edit {
    if extend {
        caret.anchor.get_or_insert(caret.cursor);
    } else {
        caret.anchor = None;
    }
    caret.cursor = to;
    Edit::Moved
}

/// `KeyDown`. `vertical` moves the cursor a line up (-1) or down (1) in a multi-line view: the place on that line nearest
/// the cursor's (none at the first or last line).
pub fn key(text: &mut String, caret: &mut Caret, key: Key, mods: Mods, feature: u32, vertical: &dyn Fn(usize, i32) -> Option<usize>) -> Edit {
    caret.clamp(text);
    let at = caret.cursor;
    match key {
        // (Enter: a new line with Shift or in `TVF_ALT_ENTER_MODE`, in a multi-line view; else the view's Enter)
        Key::Enter => {
            if (mods.shift || feature & flags::ALT_ENTER_MODE != 0) && feature & flags::MULTILINE != 0 {
                insert(text, caret, "\n", feature)
            } else {
                Edit::Enter
            }
        }
        Key::Backspace => {
            if delete_selection(text, caret) {
                return Edit::Changed;
            }
            if at == 0 {
                return Edit::None;
            }
            let from = if mods.ctrl { word_left(text, at) } else { prev_char(text, at) };
            text.replace_range(from..at, "");
            caret.cursor = from;
            Edit::Changed
        }
        Key::Delete => {
            if delete_selection(text, caret) {
                return Edit::Changed;
            }
            if at >= text.len() {
                return Edit::None;
            }
            text.replace_range(at..next_char(text, at), "");
            Edit::Changed
        }
        Key::Home => {
            let to = if mods.ctrl { 0 } else { line_bounds(text, at).0 };
            move_to(caret, to, mods.shift)
        }
        Key::End => {
            let to = if mods.ctrl { text.len() } else { line_bounds(text, at).1 };
            move_to(caret, to, mods.shift)
        }
        Key::Left => {
            let to = if mods.ctrl { word_left(text, at) } else if !mods.shift && let Some((a, _)) = caret.selection() { a } else { prev_char(text, at) };
            move_to(caret, to, mods.shift)
        }
        Key::Right => {
            let to = if mods.ctrl { word_right(text, at) } else if !mods.shift && let Some((_, b)) = caret.selection() { b } else { next_char(text, at) };
            move_to(caret, to, mods.shift)
        }
        Key::Up | Key::Down if feature & flags::MULTILINE != 0 => match vertical(at, if key == Key::Up { -1 } else { 1 }) {
            Some(to) => move_to(caret, to, mods.shift),
            None => Edit::None,
        },
        _ => Edit::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: usize, _: i32) -> Option<usize> {
        None
    }

    fn press(text: &mut String, caret: &mut Caret, k: Key, mods: Mods, feature: u32) -> Edit {
        key(text, caret, k, mods, feature, &none)
    }

    #[test]
    fn typing_moving_and_deleting() {
        let (mut text, mut caret, f) = (String::new(), Caret::default(), flags::ACCEPT_TXT_INPUT);
        assert_eq!(insert(&mut text, &mut caret, "hello world", f), Edit::Changed);
        assert_eq!((text.as_str(), caret.cursor), ("hello world", 11));
        let ctrl = Mods { ctrl: true, ..Mods::default() };
        let shift = Mods { shift: true, ..Mods::default() };
        press(&mut text, &mut caret, Key::Left, ctrl, f);
        assert_eq!(caret.cursor, 6);
        press(&mut text, &mut caret, Key::Backspace, ctrl, f);
        assert_eq!((text.as_str(), caret.cursor), ("world", 0));
        press(&mut text, &mut caret, Key::End, shift, f);
        assert_eq!(caret.selection(), Some((0, 5)));
        insert(&mut text, &mut caret, "là", f);
        assert_eq!((text.as_str(), caret.cursor), ("là", 3));
        press(&mut text, &mut caret, Key::Backspace, Mods::default(), f);
        assert_eq!(text, "l");
        press(&mut text, &mut caret, Key::Home, Mods::default(), f);
        press(&mut text, &mut caret, Key::Delete, Mods::default(), f);
        assert_eq!((text.as_str(), caret.cursor), ("", 0));
    }

    #[test]
    fn enter_numbers_and_new_lines() {
        let (mut text, mut caret) = (String::new(), Caret::default());
        assert_eq!(insert(&mut text, &mut caret, "1a2-3", flags::NUMERIC), Edit::Changed);
        assert_eq!(text, "123");
        assert_eq!(press(&mut text, &mut caret, Key::Enter, Mods::default(), 0), Edit::Enter);
        let mail = flags::MULTILINE | flags::ALT_ENTER_MODE;
        assert_eq!(press(&mut text, &mut caret, Key::Enter, Mods::default(), mail), Edit::Changed);
        assert_eq!(text, "123\n");
        insert(&mut text, &mut caret, "45", mail);
        press(&mut text, &mut caret, Key::Home, Mods::default(), mail);
        assert_eq!(caret.cursor, 4);
        // (a single-line view keeps no new line)
        let (mut one, mut c) = (String::from("ab"), Caret { cursor: 2, anchor: None });
        assert_eq!(press(&mut one, &mut c, Key::Enter, Mods { shift: true, ..Mods::default() }, 0), Edit::Enter);
        assert_eq!(insert(&mut one, &mut c, "c\nd", 0), Edit::Changed);
        assert_eq!(one, "abcd");
    }
}
