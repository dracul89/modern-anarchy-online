//! The dimensions (the game's worlds, each a login server): the list the original launcher (`Anarchy.exe`) downloads
//! before it lets the player in, from the address in the game's `cd_image/data/launcher/DimensionServer.url`, and keeps a
//! copy of (`dimensions.txt`) for when the server cannot be reached ("Could not retrieve dimension information. Using
//! cached data."). The file is blocks of `name = value` lines between `STARTINFO` and `ENDINFO`: `displayname`,
//! `connect` (the login server's host), `ports`, `version` (what the launcher compares the game's `version.id` with).

use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dimension {
    pub name: String,
    pub connect: String,
    pub port: u16,
    pub version: String,
}

impl Dimension {
    /// The login server's address as the client's connection takes it.
    pub fn server(&self) -> String {
        format!("{}:{}", self.connect, self.port)
    }
}

/// The dimensions of a list's text (a block without a host or a port is left out; `#` starts a comment line).
pub fn parse(text: &str) -> Vec<Dimension> {
    let mut out = Vec::new();
    let mut current: Option<Dimension> = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        if line.eq_ignore_ascii_case("STARTINFO") {
            current = Some(Dimension::default());
        } else if line.eq_ignore_ascii_case("ENDINFO") {
            if let Some(d) = current.take().filter(|d| !d.connect.is_empty() && d.port != 0) {
                out.push(d);
            }
        } else if let (Some(d), Some((key, value))) = (current.as_mut(), line.split_once('=')) {
            let value = value.trim();
            match key.trim().to_ascii_lowercase().as_str() {
                "displayname" => d.name = value.to_string(),
                "description" if d.name.is_empty() => d.name = value.to_string(),
                "connect" => d.connect = value.to_string(),
                // (the first of the ports)
                "ports" => d.port = value.split(|c: char| !c.is_ascii_digit()).find(|p| !p.is_empty()).and_then(|p| p.parse().ok()).unwrap_or(0),
                "version" => d.version = value.to_string(),
                _ => {}
            }
        }
    }
    out
}

/// The first address of a `.url` list file (`DimensionServer.url`: `host:port/path` lines, `#` comments), and a named
/// value of one (`AnarchyLauncher.url`: `SHOWURL=...`).
pub fn first_address(text: &str) -> Option<String> {
    text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#') && !l.contains('=')).map(str::to_string)
}

pub fn named_value(text: &str, name: &str) -> Option<String> {
    text.lines().map(str::trim).filter(|l| !l.starts_with('#')).find_map(|l| l.split_once('=').filter(|(k, _)| k.trim().eq_ignore_ascii_case(name)).map(|(_, v)| v.trim().to_string())).filter(|v| !v.is_empty())
}

/// The address the game folder's launcher files name for the dimension list.
pub fn list_address(game: &Path) -> Option<String> {
    first_address(&mao_formats::files::read_to_string(game.join("cd_image/data/launcher/DimensionServer.url")).ok()?)
}

/// The page of a login error under `base` (`LoginModule_c::ShowError` 0x10011fd5): the number of the message that
/// refused (13, the login; 1 when the login server is not reached), a dash and its code when it has one, `.html`.
pub fn error_page_under(base: &str, kind: u32, code: u32) -> String {
    if code != 0 { format!("{base}{kind}-{code}.html") } else { format!("{base}{kind}.html") }
}

/// The page of a login error: under `ERRORURL` of `AnarchyLauncher.url` (GUI.dll's `LoginModule_c` reads that file's
/// `name=value` lines into its addresses, the names in small letters, and asks for `errorurl`).
pub fn error_page(game: &Path, kind: u32, code: u32) -> Option<String> {
    let base = named_value(&mao_formats::files::read_to_string(game.join("cd_image/data/launcher/AnarchyLauncher.url")).ok()?, "ERRORURL")?;
    Some(error_page_under(&base, kind, code))
}

/// The page the original launcher shows (`SHOWURL` of `AnarchyLauncher.url`).
pub fn page_address(game: &Path) -> Option<String> {
    named_value(&mao_formats::files::read_to_string(game.join("cd_image/data/launcher/AnarchyLauncher.url")).ok()?, "SHOWURL")
}

/// Where the copy of the list is kept: beside the settings.
pub fn cache_path() -> PathBuf {
    crate::settings::Settings::path().with_file_name("dimensions.txt")
}

/// Downloads the list's text (plain HTTP, as the original asks for it) within `timeout`.
pub fn download(address: &str, timeout: Duration) -> Result<String, String> {
    let url = if address.contains("://") { address.to_string() } else { format!("http://{address}") };
    let agent = ureq::AgentBuilder::new().timeout_connect(timeout).timeout(timeout).build();
    let text = agent.get(&url).call().map_err(|e| e.to_string())?.into_string().map_err(|e| e.to_string())?;
    if parse(&text).is_empty() {
        return Err("the dimension server's answer lists no dimension".into());
    }
    Ok(text)
}

/// The launcher's own small file beside the settings (`launcher.ini`): the dimension chosen last and the two time-outs
/// of the original's Connection page (seconds).
#[derive(Clone, Debug, PartialEq)]
pub struct Prefs {
    pub dimension: String,
    pub network_timeout: u32,
    pub dimension_timeout: u32,
    /// The launcher's background: `game` (one of the game's pictures), `stars` (this client's night sky with the
    /// game's steel emblem) or a picture file (`launchskin.rs`).
    pub background: String,
    /// The game's news page over the launcher's top.
    pub page: bool,
    /// The game's licence (`anarchy.eula`) was accepted (the client's own rule: the original's place for this was not
    /// found, so the launcher asks once).
    pub eula: bool,
    /// PATCH starts Funcom's `AnarchyPatcher.exe`, as the original's launcher does, and does not patch the game
    /// with the client's own patcher (`mao-install`), which is the default: `patcher = funcom`. Only Windows has
    /// the choice: nothing else can run Funcom's.
    pub funcom_patcher: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        // (the original's own defaults: its Connection page, `GeneralNetworkTimeout` and `DimensionNetworkTimeout`)
        Prefs { dimension: String::new(), network_timeout: 90, dimension_timeout: 30, background: crate::launchskin::GAME.into(), page: true, eula: false, funcom_patcher: false }
    }
}

/// A time-out field of the Connection page as the original takes it on OK (Anarchy.exe 0x40ecdc: `atoi` of the text, kept
/// unchecked until the next start raises it to 5): leading blanks, a sign and the digits; 0 for none (and for a negative,
/// which the client's whole seconds cannot hold).
pub fn page_number(text: &str) -> u32 {
    let t = text.trim_start();
    let (negative, digits) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let value = digits.bytes().take_while(u8::is_ascii_digit).fold(0u64, |v, d| (v * 10 + (d - b'0') as u64).min(u32::MAX as u64)) as u32;
    if negative { 0 } else { value }
}

impl Prefs {
    pub fn path() -> PathBuf {
        crate::settings::Settings::path().with_file_name("launcher.ini")
    }

    pub fn from_text(text: &str) -> Prefs {
        let mut p = Prefs::default();
        for (key, value) in text.lines().filter_map(|l| l.split_once('=')) {
            let value = value.trim();
            match key.trim().to_ascii_lowercase().as_str() {
                "dimension" => p.dimension = value.to_string(),
                // (the original launcher raises either time-out below 5 to 5 as it starts, and keeps a larger one: Anarchy.exe
                // 0x404bad and 0x404cc1)
                "network_timeout" => p.network_timeout = value.parse().unwrap_or(p.network_timeout).max(5),
                "dimension_timeout" => p.dimension_timeout = value.parse().unwrap_or(p.dimension_timeout).max(5),
                "background" if !value.is_empty() => p.background = value.to_string(),
                "page" => p.page = !matches!(value.to_ascii_lowercase().as_str(), "off" | "false" | "no" | "0"),
                "eula" => p.eula = matches!(value.to_ascii_lowercase().as_str(), "accepted" | "on" | "yes" | "1"),
                "patcher" => p.funcom_patcher = value.eq_ignore_ascii_case("funcom"),
                _ => {}
            }
        }
        p
    }

    pub fn to_text(&self) -> String {
        let mut text = format!("dimension = {}\nnetwork_timeout = {}\ndimension_timeout = {}\nbackground = {}\npage = {}\n", self.dimension, self.network_timeout, self.dimension_timeout, self.background, if self.page { "on" } else { "off" });
        if self.eula {
            text.push_str("eula = accepted\n");
        }
        if self.funcom_patcher {
            text.push_str("patcher = funcom\n");
        }
        text
    }

    pub fn load() -> Prefs {
        Prefs::from_text(&std::fs::read_to_string(Prefs::path()).unwrap_or_default())
    }

    pub fn save(&self) {
        let path = Prefs::path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, self.to_text());
    }
}

/// How the game folder's version stands to a dimension's (Anarchy.exe 0x412560-0x412761 reads both with `%u.%u.%u.%u`
/// and keeps two flags on the dimension: +0x32 needs a patch, +0x33 an old server; the list shows the first that is
/// set and the click checks the label in that order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionState {
    Same,
    NeedPatch,
    OldServer,
}

/// `sscanf(text, "%u.%u.%u.%u", ...)`: how many numbers were read (-1 when the text ends before the first, as sscanf
/// gives EOF) and the numbers (0 where none was read). A number may have a sign and wraps as an unsigned one.
fn scan_version(text: &str) -> (i32, [u32; 4]) {
    let b = text.as_bytes();
    let mut at = 0;
    let mut out = [0u32; 4];
    for n in 0..4 {
        if n > 0 {
            if b.get(at) != Some(&b'.') {
                return (n as i32, out);
            }
            at += 1;
        }
        while b.get(at).is_some_and(|c| c.is_ascii_whitespace()) {
            at += 1;
        }
        if at >= b.len() {
            return (if n == 0 { -1 } else { n as i32 }, out);
        }
        let negative = b[at] == b'-';
        if matches!(b[at], b'+' | b'-') {
            at += 1;
        }
        let digits = at;
        let mut value = 0u32;
        while let Some(c) = b.get(at).filter(|c| c.is_ascii_digit()) {
            value = value.wrapping_mul(10).wrapping_add((c - b'0') as u32);
            at += 1;
        }
        if at == digits {
            return (n as i32, out);
        }
        out[n] = if negative { value.wrapping_neg() } else { value };
    }
    (4, out)
}

pub fn version_state(local: &str, dimension: &str) -> VersionState {
    let (local_count, l) = scan_version(local);
    let (dimension_count, d) = scan_version(dimension);
    // (the first number that differs, game's less the dimension's: below nought needs a patch, above is an old server,
    // unless the dimension's has more than two numbers and the game's just two; numbers of another count need a patch)
    let first = (0..4).map(|i| l[i].wrapping_sub(d[i]) as i32).find(|&x| x != 0).unwrap_or(0);
    let mut patch = first < 0;
    let mut old = false;
    if first > 0 {
        old = true;
        if dimension_count > 2 && local_count == 2 {
            old = false;
            patch = true;
        }
    }
    if dimension_count != local_count {
        patch = true;
    }
    if patch {
        VersionState::NeedPatch
    } else if old {
        VersionState::OldServer
    } else {
        VersionState::Same
    }
}

/// The game's expansion mark in `version.id` (Anarchy.exe 0x4035fc: `_EP1` is looked for first, then `_EP2`; neither
/// is the small client).
pub fn episode(version_id: &str) -> &'static str {
    if version_id.contains("_EP1") {
        "_EP1"
    } else if version_id.contains("_EP2") {
        "_EP2"
    } else {
        ""
    }
}

/// What the patcher is given: the dimension's version with the game's expansion mark (Anarchy.exe 0x40c1b3-0x40c1dd).
pub fn patcher_argument(dimension_version: &str, episode: &str) -> String {
    format!("{dimension_version}{episode}")
}

/// The words after a dimension's name in the list (the original's " (Need patch)" and " (Old server)").
pub fn version_label(state: VersionState) -> &'static str {
    match state {
        VersionState::Same => "",
        VersionState::NeedPatch => " (needs a patch)",
        VersionState::OldServer => " (older server)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_their_numbers() {
        assert_eq!(version_state("18.8.62", "18.8.62"), VersionState::Same);
        assert_eq!(version_state("18.8.62_EP1", "18.8.62"), VersionState::Same);
        assert_eq!(version_state("18.8.62", "18.8.63"), VersionState::NeedPatch);
        assert_eq!(version_state("18.8.62", "18.10.1"), VersionState::NeedPatch);
        assert_eq!(version_state("18.8.62", "18.8.9"), VersionState::OldServer);
        assert_eq!(version_state("18.8", "18.8.0.1"), VersionState::NeedPatch);
        // (numbers of another count need a patch, even an empty version: sscanf's EOF)
        assert_eq!(version_state("18.8.62", "18.8.62.0"), VersionState::NeedPatch);
        assert_eq!(version_state("18.8.62", ""), VersionState::NeedPatch);
        assert_eq!(version_state("18.9", "18.8.62"), VersionState::NeedPatch);
        assert_eq!(version_state("18.8.62", "18.8.x"), VersionState::NeedPatch);
        assert_eq!(scan_version(""), (-1, [0; 4]));
        assert_eq!(scan_version("abc"), (0, [0; 4]));
        assert_eq!(scan_version(" 18. 8.62_EP1"), (3, [18, 8, 62, 0]));
    }

    #[test]
    fn the_patcher_is_given_the_dimensions_version_and_the_expansion() {
        assert_eq!(episode("18.8.62_EP1"), "_EP1");
        assert_eq!(episode("18.8.62_EP2"), "_EP2");
        assert_eq!(episode("18.8.62"), "");
        assert_eq!(patcher_argument("18.8.63", episode("18.8.62_EP1")), "18.8.63_EP1");
    }

    #[test]
    fn the_licence_is_remembered() {
        let mut p = Prefs::from_text("");
        assert!(!p.eula);
        p.eula = true;
        assert!(Prefs::from_text(&p.to_text()).eula);
        assert!(!Prefs::from_text(&Prefs::default().to_text()).eula);
        // (the patcher: the client's own unless the file says Funcom's)
        assert!(!Prefs::from_text(&p.to_text()).funcom_patcher);
        p.funcom_patcher = true;
        assert!(p.to_text().contains("patcher = funcom\n") && Prefs::from_text(&p.to_text()).funcom_patcher);
        assert!(!Prefs::from_text("patcher = own\n").funcom_patcher);
    }

    #[test]
    fn a_list_gives_its_dimensions() {
        let text = "# a comment\n\nSTARTINFO\ndescription     =       First World\ndisplayname     =       First\nconnect         =       login.example.org\nports           =       7505\nurl             =       \nversion         =       18.8.62\nENDINFO\n\nSTARTINFO\ndescription = Second World\nconnect = login.example.org\nports = 7506, 7507\nversion = 18.8.62\nENDINFO\nSTARTINFO\ndisplayname = No server\nENDINFO\n";
        let list = parse(text);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], Dimension { name: "First".into(), connect: "login.example.org".into(), port: 7505, version: "18.8.62".into() });
        assert_eq!((list[1].name.as_str(), list[1].port), ("Second World", 7506));
        assert_eq!(list[0].server(), "login.example.org:7505");
        assert!(parse("nothing").is_empty());
    }

    #[test]
    fn an_error_has_its_page() {
        assert_eq!(error_page_under("http://pages.example.org/errors/", 13, 0), "http://pages.example.org/errors/13.html");
        assert_eq!(error_page_under("http://pages.example.org/errors/", 13, 2), "http://pages.example.org/errors/13-2.html");
        assert_eq!(error_page_under("http://pages.example.org/errors/", 1, 0), "http://pages.example.org/errors/1.html");
        assert_eq!(named_value("# c\nSHOWURL=a\nERRORURL=http://pages.example.org/errors/\n1=331\n", "errorurl").as_deref(), Some("http://pages.example.org/errors/"));
    }

    #[test]
    fn the_url_files_give_their_addresses() {
        assert_eq!(first_address("# do not edit\n#\nlists.example.org:80/dimensions.txt\n\n").as_deref(), Some("lists.example.org:80/dimensions.txt"));
        assert_eq!(first_address("# only comments\n"), None);
        let file = "# codes\nSHOWURL=http://pages.example.org/launcher/index.html\nERRORURL=http://pages.example.org/errors/\n1=331\n";
        assert_eq!(named_value(file, "showurl").as_deref(), Some("http://pages.example.org/launcher/index.html"));
        assert_eq!(named_value(file, "LOGINURL"), None);
    }

    #[test]
    fn the_launchers_file_keeps_the_dimension_and_the_time_outs() {
        let p = Prefs { dimension: "Second World".into(), network_timeout: 120, dimension_timeout: 10, background: "stars".into(), page: false, eula: false, funcom_patcher: false };
        assert_eq!(Prefs::from_text(&p.to_text()), p);
        // (a picture's path with a drive's colon and spaces in it is kept whole; the page is on unless it says off)
        let own = Prefs::from_text("background = D:/My Pictures/sky one.png\npage = on\n");
        assert_eq!((own.background.as_str(), own.page), ("D:/My Pictures/sky one.png", true));
        assert_eq!((Prefs::default().background.as_str(), Prefs::default().page), ("game", true));
        assert_eq!(Prefs::from_text(""), Prefs::default());
        assert_eq!(Prefs::from_text("network_timeout = 100000\n").network_timeout, 100000);
        assert_eq!((Prefs::from_text("network_timeout = 1\n").network_timeout, Prefs::from_text("dimension_timeout = 2\n").dimension_timeout), (5, 5));
        assert_eq!((page_number(" 45s"), page_number("-3"), page_number("x")), (45, 0, 0));
    }
}
