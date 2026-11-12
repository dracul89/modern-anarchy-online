//! Text macros: a name and a chat line, made with `/macro <name> <command>` and put on a hotbar, where a press runs
//! the line as if it were typed. GUI.dll's `TextMacroSystem_t`, read on 2026-11-12:
//!
//! * A macro is an identity of the type 0xc789 with a number, a name and a command (`CreateMacro` 0x1002338f: the
//!   number asked for, or with 0 the first from 1 that is free; a macro of the same number is replaced).
//! * They are kept in `TextMacro.bin` in the character's preferences folder (`Init` 0x100236ec, `Save` 0x10023201,
//!   `Load` 0x1002350b): a count, then for each its number, the name's length and bytes, the command's length and
//!   bytes; the numbers big-endian, four bytes. The layout was checked against the files the original wrote on this
//!   machine. Here the file is the client's own, beside its settings (the hotbars it serves are the client's own too).
//! * `/macro` (FUN_100b8c82) wants a name and a command and raises a signal; the shortcut bar's module (FUN_100d8c4a)
//!   makes the macro, saves, and starts a drag of it ("inventory/textmacro"), to be dropped on a bar. A bar's entry of
//!   kind 7 shows the name in yellow as its picture (FUN_100402e8), says the name and the command when pointed at
//!   (FUN_10042193; "Broken macro" when the number has none), and pressed raises the signal that carries a line out
//!   as typed (FUN_100d833f, `GlobalSignals_c` +0x180).
//!
//! One thing is not as the original has it: `CreateMacro` cuts a name or a command to 100 bytes, and `Load` gives up
//! at the first one over 99, dropping it and every macro after it in the file. Here they are cut to 99, so what was
//! saved is read again (`docs/ORIGINAL_CODE_QUESTIONS.md`, 13).

use std::collections::BTreeMap;
use std::sync::Mutex;

/// The identity type of a macro (`CreateMacro`).
pub const TYPE: u32 = 0xc789;
/// The most bytes of a name or a command that `Load` reads back.
pub const LONGEST: usize = 99;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Macro {
    pub name: String,
    pub command: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Macros {
    map: BTreeMap<i32, Macro>,
}

/// A text cut to [`LONGEST`] bytes, at a character's edge.
fn cut(text: &str) -> String {
    let mut end = text.len().min(LONGEST);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

impl Macros {
    /// `CreateMacro`: the macro of that number made or replaced; with 0, of the first number from 1 that is free.
    /// Its number.
    pub fn create(&mut self, name: &str, command: &str, id: i32) -> i32 {
        let id = if id == 0 { (1..).find(|n| !self.map.contains_key(n)).unwrap_or(1) } else { id };
        self.map.insert(id, Macro { name: cut(name), command: cut(command) });
        id
    }

    /// `GetMacro`.
    pub fn get(&self, id: i32) -> Option<&Macro> {
        self.map.get(&id)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// `Save`'s bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = (self.map.len() as i32).to_be_bytes().to_vec();
        for (id, m) in &self.map {
            out.extend_from_slice(&id.to_be_bytes());
            for text in [&m.name, &m.command] {
                out.extend_from_slice(&(text.len() as i32).to_be_bytes());
                out.extend_from_slice(text.as_bytes());
            }
        }
        out
    }

    /// `Load`: the macros of a file's bytes; reading stops at the first that is cut short or longer than the
    /// original allows, keeping those before it.
    pub fn from_bytes(bytes: &[u8]) -> Macros {
        let mut macros = Macros::default();
        let mut at = 0;
        let mut number = |at: &mut usize| -> Option<i32> {
            let v = i32::from_be_bytes(bytes.get(*at..*at + 4)?.try_into().ok()?);
            *at += 4;
            Some(v)
        };
        let Some(count) = number(&mut at) else { return macros };
        for _ in 0..count.max(0) {
            let read = (|| {
                let id = number(&mut at)?;
                let mut texts = [String::new(), String::new()];
                for text in &mut texts {
                    let length = usize::try_from(number(&mut at)?).ok().filter(|l| *l <= LONGEST)?;
                    *text = String::from_utf8_lossy(bytes.get(at..at + length)?).into_owned();
                    at += length;
                }
                let [name, command] = texts;
                Some((id, Macro { name, command }))
            })();
            let Some((id, m)) = read else { break };
            macros.map.insert(id, m);
        }
        macros
    }
}

/// The file: `TextMacro.bin` beside the settings.
pub fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("TextMacro.bin")
}

static MACROS: Mutex<Option<Macros>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Macros) -> R) -> R {
    let mut held = MACROS.lock().unwrap_or_else(|e| e.into_inner());
    let macros = held.get_or_insert_with(|| std::fs::read(path()).map(|b| Macros::from_bytes(&b)).unwrap_or_default());
    f(macros)
}

/// A new macro, saved (`CreateMacro(name, command, 0, true)`): its number.
pub fn create(name: &str, command: &str) -> i32 {
    with(|m| {
        let id = m.create(name, command, 0);
        let path = path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&path, m.to_bytes()) {
            eprintln!("can't save {}: {e}", path.display());
        }
        id
    })
}

/// The macro of that number.
pub fn get(id: i32) -> Option<Macro> {
    with(|m| m.get(id).cloned())
}

/// The lines of the macros pressed, for the chat window to carry out as if typed (the original's signal,
/// `GlobalSignals_c` +0x180).
static PRESSED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// A press of a macro on a hotbar: its line queued. `false` when there is no macro of that number (the original's
/// "Broken macro").
pub fn press(id: i32) -> bool {
    match get(id) {
        Some(m) => {
            PRESSED.lock().unwrap_or_else(|e| e.into_inner()).push(m.command);
            true
        }
        None => false,
    }
}

pub fn take_pressed() -> Vec<String> {
    std::mem::take(&mut *PRESSED.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macros_take_the_first_free_number_and_are_read_back() {
        let mut m = Macros::default();
        assert_eq!(m.create("Follow", "/follow", 0), 1);
        assert_eq!(m.create("Heal", "/pet heal", 0), 2);
        assert_eq!(m.create("Seven", "/say seven", 7), 7);
        // (a number asked for replaces the macro that had it; 0 takes the first gap)
        assert_eq!(m.create("Follow me", "/follow", 1), 1);
        assert_eq!(m.create("Three", "/say three", 0), 3);
        assert_eq!((m.len(), m.get(1).unwrap().name.as_str(), m.get(9)), (4, "Follow me", None));
        // (the file: a count, then number, name and command, the numbers big-endian)
        let bytes = m.to_bytes();
        assert_eq!(&bytes[..12], &[0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 9]);
        assert_eq!(&bytes[12..21], b"Follow me");
        assert_eq!(Macros::from_bytes(&bytes), m);
        // (a file cut short keeps what came whole before the cut)
        let short = Macros::from_bytes(&bytes[..bytes.len() - 3]);
        assert_eq!((short.len(), short.get(7)), (3, None));
        assert_eq!(Macros::from_bytes(&[]).len(), 0);
    }

    #[test]
    fn a_long_text_is_cut_to_what_is_read_back() {
        let mut m = Macros::default();
        let long = "x".repeat(150);
        let id = m.create(&long, &format!("/say {long}"), 0);
        assert_eq!((m.get(id).unwrap().name.len(), m.get(id).unwrap().command.len()), (LONGEST, LONGEST));
        m.create("after", "/say after", 0);
        // (both are there after a save and a load: the original would lose them, see the module's note)
        assert_eq!(Macros::from_bytes(&m.to_bytes()), m);
        // (a length over the limit in a file ends the reading, as `Load` does)
        let mut bytes = 2i32.to_be_bytes().to_vec();
        bytes.extend_from_slice(&1i32.to_be_bytes());
        bytes.extend_from_slice(&100i32.to_be_bytes());
        bytes.extend_from_slice(&[b'y'; 100]);
        assert_eq!(Macros::from_bytes(&bytes).len(), 0);
    }
}
