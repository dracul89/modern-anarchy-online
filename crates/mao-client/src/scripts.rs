//! Script files: a text file of chat lines in the `scripts` folder, run with `/<file name> [arguments]` or
//! `/script <file name> [arguments]`. GUI.dll's, read on 2026-11-12:
//!
//! * **Loading** (FUN_100b1517; `/script` is FUN_1009d4e9, which hands it what follows the command; a command the
//!   input line does not know is handed over whole). The line, under 0xb5 bytes or `Script2Long` is said, is split
//!   into words; the first, its slashes taken out, is the file's name in `AnarchyPath_t::GetScriptsPath()`. No such
//!   file: `Unable2FindScriptX` with the name. An empty file, or one over 0x1000 bytes: said so, nothing run. Each
//!   line that is not empty and does not begin with `#` is queued, after its `%N` are filled in: `%` and a digit is
//!   replaced by word N of the line (0 is the file's name), or by nothing when there are not that many. Two
//!   characters are replaced whatever follows, while the number is read as far as its digits go (`atoi`): `%12` is
//!   word 12 followed by a `2`. The looking goes on from the character after the `%`, so a `%N` straight after one
//!   that stood for nothing is left as it is.
//! * **Running** (FUN_100b1d6c, each frame): one queued line a frame, carried out as if typed; a line that begins
//!   `/delay ` (any case) and a number waits that many milliseconds instead, counted in frame times.
//!
//! The original's folder is beside its preferences (`%LOCALAPPDATA%/Funcom/Anarchy Online/<install>/Anarchy
//! Online/scripts`). Here a `scripts` folder beside the client's settings is looked in first, then the original's
//! (of the install the character's preferences are in, [`crate::aoprefs`]); the file names are offered with the
//! commands when a `/` is typed, as the original offers them (FUN_100a44e8). How the original's `String::Tokenize`
//! treats quotes was not read: the words here are split at blanks.

use std::path::PathBuf;

/// A script's command line is under this many bytes (FUN_100b1517).
pub const LINE_LIMIT: usize = 0xb5;
/// A script file is at most this many bytes.
pub const FILE_LIMIT: usize = 0x1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The command line is too long (`Script2Long`).
    LineTooLong,
    /// No file of that name (`Unable2FindScriptX`).
    NotFound(String),
    Empty,
    TooBig,
}

impl Problem {
    /// What the player is told (text.mdb 10001 by name, or the original's own words in the code).
    pub fn text(&self) -> String {
        match self {
            Problem::LineTooLong => crate::gui::xml::ldb_named(10001, "Script2Long", "The line is too long for a script."),
            Problem::NotFound(name) => mao_formats::ldb::format(&crate::gui::xml::ldb_named(10001, "Unable2FindScriptX", "There is no command or script named '%s'."), &[mao_formats::ldb::Arg::Text(name)]),
            Problem::Empty => "Script file is empty. Not loaded.".to_string(),
            Problem::TooBig => "Script file is bigger than 4096 bytes. Not loaded.".to_string(),
        }
    }
}

/// The folders scripts are looked for in: the client's own, then the original's.
pub fn dirs() -> Vec<PathBuf> {
    let mut dirs = vec![crate::settings::Settings::path().with_file_name("scripts")];
    if let Some((prefs, _)) = crate::layout::character().and_then(crate::aoprefs::prefs_dirs)
        && let Some(root) = prefs.parent()
    {
        dirs.push(root.join("scripts"));
    }
    dirs
}

/// The names of the script files there are (those not beginning with a dot), for the list of commands offered.
/// Read at most every few seconds.
pub fn names() -> Vec<String> {
    static SEEN: std::sync::Mutex<Option<(std::time::Instant, Vec<PathBuf>, Vec<String>)>> = std::sync::Mutex::new(None);
    let dirs = dirs();
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((when, of, names)) = seen.as_ref()
        && when.elapsed().as_secs() < 5
        && *of == dirs
    {
        return names.clone();
    }
    let mut names: Vec<String> = Vec::new();
    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.starts_with('.') && e.path().is_file() && !names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
                names.push(name);
            }
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    *seen = Some((std::time::Instant::now(), dirs, names.clone()));
    names
}

/// The words of a command line; the first is the script's name, its slashes taken out.
pub fn words(line: &str) -> Vec<String> {
    let mut words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    if let Some(first) = words.first_mut() {
        *first = first.replace('/', "");
    }
    words
}

/// A script's text as the lines to run, with the command line's words put in.
pub fn lines(text: &str, words: &[String]) -> Vec<String> {
    text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| fill(l, words)).collect()
}

/// One line's `%N` filled in (see the module's note for the two-character rule).
fn fill(line: &str, words: &[String]) -> String {
    let mut line: Vec<u8> = line.as_bytes().to_vec();
    let mut from = 0;
    while let Some(at) = line[from.min(line.len())..].iter().position(|&b| b == b'%').map(|p| p + from) {
        from = at + 1;
        if !line.get(at + 1).is_some_and(u8::is_ascii_digit) {
            continue;
        }
        let digits: String = line[at + 1..].iter().take_while(|b| b.is_ascii_digit()).map(|&b| b as char).collect();
        let number = digits.parse::<usize>().unwrap_or(usize::MAX);
        let word = words.get(number).map(|w| w.as_bytes().to_vec()).unwrap_or_default();
        line.splice(at..at + 2, word);
    }
    String::from_utf8_lossy(&line).into_owned()
}

/// The lines of the script a command line names (what was typed, with or without its slash), from `dirs`.
pub fn load_from(dirs: &[PathBuf], line: &str) -> Result<Vec<String>, Problem> {
    if line.len() >= LINE_LIMIT {
        return Err(Problem::LineTooLong);
    }
    let words = words(line);
    let name = words.first().cloned().unwrap_or_default();
    // (a name is a file's name in the folder, nothing more)
    let plain = !name.is_empty() && !name.contains(['\\', ':']) && name != "." && name != "..";
    let Some(path) = dirs.iter().map(|d| d.join(&name)).find(|p| plain && p.is_file()) else { return Err(Problem::NotFound(name)) };
    let bytes = std::fs::read(&path).map_err(|_| Problem::NotFound(name))?;
    if bytes.is_empty() {
        return Err(Problem::Empty);
    }
    if bytes.len() > FILE_LIMIT {
        return Err(Problem::TooBig);
    }
    Ok(lines(&String::from_utf8_lossy(&bytes), &words))
}

pub fn load(line: &str) -> Result<Vec<String>, Problem> {
    load_from(&dirs(), line)
}

/// The queued lines of the scripts being run.
#[derive(Debug, Default)]
pub struct Runner {
    lines: Vec<String>,
    at: usize,
    /// A `/delay` being waited out: the seconds gone and wanted.
    waiting: Option<(f32, f32)>,
}

impl Runner {
    pub fn add(&mut self, lines: Vec<String>) {
        self.lines.extend(lines);
    }

    pub fn running(&self) -> bool {
        self.at < self.lines.len()
    }

    /// A frame of `dt` seconds: the line to carry out now, if any.
    pub fn tick(&mut self, dt: f32) -> Option<String> {
        let Some(line) = self.lines.get(self.at) else {
            self.lines.clear();
            self.at = 0;
            self.waiting = None;
            return None;
        };
        if line.get(..7).is_some_and(|p| p.eq_ignore_ascii_case("/delay ")) {
            match &mut self.waiting {
                // (the frame the delay is met only starts it)
                None => {
                    let ms: i32 = line[7..].trim().split(|c: char| !c.is_ascii_digit() && c != '-').next().and_then(|n| n.parse().ok()).unwrap_or(0);
                    self.waiting = Some((0.0, ms as f32 / 1000.0));
                }
                Some((gone, wanted)) => {
                    *gone += dt;
                    if *wanted <= *gone {
                        self.at += 1;
                        self.waiting = None;
                    }
                }
            }
            return None;
        }
        self.at += 1;
        Some(line.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scripts_lines_take_the_command_lines_words() {
        let w = words("/greet Zed  there");
        assert_eq!(w, ["greet", "Zed", "there"]);
        let text = "# a comment\r\n/say hello %1, %2!\r\n\r\n/me runs %0 for %3%1\n/say 100%% of %12 and %x\n";
        assert_eq!(lines(text, &w), ["/say hello Zed, there!", "/me runs greet for %1", "/say 100%% of 2 and %x"]);
        // (after a word is put in, the looking goes on one character further: a `%N` that slid into the place of one
        // filled with nothing is passed by, as the original passes it)
        assert_eq!(fill("%3 %1", &w), " Zed");
        // (a word put in is looked through again from its second character on)
        assert_eq!(fill("%1", &["s".into(), "a%2".into(), "b".into()]), "ab");
    }

    #[test]
    fn a_script_is_found_by_its_name_in_the_folders() {
        let dir = std::env::temp_dir().join(format!("mao-scripts-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hello"), "/say hi %1\n/delay 500\n/say bye\n").unwrap();
        std::fs::write(dir.join("nothing"), "").unwrap();
        std::fs::write(dir.join("big"), vec![b'x'; FILE_LIMIT + 1]).unwrap();
        let dirs = [dir.join("missing"), dir.clone()];
        assert_eq!(load_from(&dirs, "/hello Zed"), Ok(vec!["/say hi Zed".to_string(), "/delay 500".to_string(), "/say bye".to_string()]));
        assert_eq!(load_from(&dirs, "hello"), Ok(vec!["/say hi ".to_string(), "/delay 500".to_string(), "/say bye".to_string()]));
        assert_eq!(load_from(&dirs, "/nosuch a b"), Err(Problem::NotFound("nosuch".into())));
        assert_eq!(load_from(&dirs, "/nothing"), Err(Problem::Empty));
        assert_eq!(load_from(&dirs, "/big"), Err(Problem::TooBig));
        assert_eq!(load_from(&dirs, &format!("/hello {}", "x".repeat(LINE_LIMIT))), Err(Problem::LineTooLong));
        assert_eq!(load_from(&dirs, "/..\\hello"), Err(Problem::NotFound("..\\hello".into())));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_line_a_frame_and_delays_are_waited_out() {
        let mut r = Runner::default();
        assert_eq!(r.tick(0.1), None);
        r.add(vec!["/say one".into(), "/DELAY 250".into(), "/say two".into()]);
        assert!(r.running());
        assert_eq!(r.tick(0.1).as_deref(), Some("/say one"));
        // (the delay: the frame it is met, then frames until a quarter of a second has gone)
        assert_eq!((r.tick(0.1), r.tick(0.1), r.tick(0.1)), (None, None, None));
        assert_eq!(r.tick(0.1), None);
        assert_eq!(r.tick(0.1).as_deref(), Some("/say two"));
        assert!(!r.running());
        assert_eq!(r.tick(0.1), None);
        // (another script's lines go after those still to run)
        r.add(vec!["a".into()]);
        r.add(vec!["b".into()]);
        assert_eq!((r.tick(0.1).as_deref(), r.tick(0.1).as_deref()), (Some("a"), Some("b")));
    }
}
