//! What Funcom's downloader and patcher ask the servers for, and the asking: plain HTTP `GET`s, as theirs are.
//!
//! - The installer's list. Funcom's download `AnarchyOnline_EP1.exe` is an NSIS 2.46 script that fetches
//!   [`INSTALLER_LIST`] and reads it a line at a time, three quoted words to a line: `"dl" "<address>" "<file>"`
//!   downloads the address to the file, `"in" "<file>"` runs the file ([`installer_list`]).
//! - The patch servers: `patchservers.txt` in the folder of the dimension list that the game's
//!   `cd_image/data/launcher/DimensionServer.url` names; lines of `name;weight;address` ([`patch_servers`]).
//! - The map, `<server>/map`: lines of two versions, a step from the first to the second. The patcher walks it once
//!   from the version in `version.id` (`AnarchyPatcher.exe` `FUN_0045b1c0`), naming each step `v<from>-v<to>`
//!   ([`steps`]).
//! - A step's four files, `<server>/<step>/<STEP>.<EXT>`: the step's name with its dots turned into dashes and in
//!   capitals, and `RTP`, `RES`, `CHK`, `SIG` (`FUN_0045d0e0`: the format `%s/%s/%s.%s` at 0x45dc90, the extensions'
//!   table at 0x66d438) ([`Step::address`]).

use std::fmt;
use std::fs::OpenOptions;
use std::io::{self, Read, Seek, Write};
use std::path::Path;
use std::time::Duration;

/// Where Funcom's downloader gets its list of the installer's parts.
pub const INSTALLER_LIST: &str = "http://cdn-web-nj4.funcom.com/download/AO/current_EP1.txt";

#[derive(Debug)]
pub enum Error {
    /// The server could not be reached, or answered with an error.
    Http { address: String, why: String },
    /// A file on the disk could not be written.
    Io(io::Error),
    /// The download was stopped by the caller.
    Stopped,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Http { address, why } => write!(f, "{address}: {why}"),
            Error::Io(e) => write!(f, "{e}"),
            Error::Stopped => write!(f, "stopped"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// One line of the installer's list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListLine {
    /// Download the address to the file.
    Download { address: String, file: String },
    /// Run the file (the installer itself).
    Run { file: String },
}

/// The installer's list, read as the NSIS script reads it: each line's words are what stands between its quotes;
/// the first says what to do, and a line that says anything else than `dl` or `in` ends the reading.
pub fn installer_list(text: &str) -> Vec<ListLine> {
    let mut lines = Vec::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
        match words[..] {
            ["dl", address, file, ..] => lines.push(ListLine::Download { address: address.to_owned(), file: file.to_owned() }),
            ["in", file, ..] => lines.push(ListLine::Run { file: file.to_owned() }),
            _ => break,
        }
    }
    lines
}

/// A file name that stays in the folder it is downloaded to.
pub fn plain_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':'])
}

/// The versions the dimension list gives its dimensions (`version = 18.8.62` between `STARTINFO` and `ENDINFO`).
pub fn dimension_versions(text: &str) -> Vec<String> {
    let mut versions = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.starts_with('#')) {
        if let Some((name, value)) = line.split_once('=')
            && name.trim().eq_ignore_ascii_case("version")
            && !value.trim().is_empty()
        {
            versions.push(value.trim().to_owned());
        }
    }
    versions
}

/// The first address in a `.url` file of the game's (`DimensionServer.url`): the first line that is no remark.
pub fn first_address(text: &str) -> Option<String> {
    text.lines().map(str::trim).find(|line| !line.is_empty() && !line.starts_with('#')).map(str::to_owned)
}

/// An address as the game's files write it (`host:80/path`), as a URL.
pub fn url(address: &str) -> String {
    if address.contains("://") { address.to_owned() } else { format!("http://{address}") }
}

/// The address of `patchservers.txt`: in the folder of the dimension list.
pub fn patch_servers_address(dimension_list: &str) -> String {
    let list = url(dimension_list);
    match list.rfind('/') {
        Some(slash) if slash > list.find("://").map_or(0, |at| at + 2) => format!("{}/patchservers.txt", &list[..slash]),
        _ => format!("{list}/patchservers.txt"),
    }
}

/// The patch servers' addresses, the most wanted first: `name;weight;address` lines, the lowest weight first (the
/// patcher's own order among them was not read; it tries another when one fails, and so does [`crate::install`]).
pub fn patch_servers(text: &str) -> Vec<String> {
    let mut servers: Vec<(u32, String)> = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')) {
        let mut parts = line.splitn(3, ';');
        if let (Some(_), Some(weight), Some(address)) = (parts.next(), parts.next(), parts.next()) {
            servers.push((weight.trim().parse().unwrap_or(u32::MAX), address.trim().trim_end_matches('/').to_owned()));
        }
    }
    servers.sort_by_key(|(weight, _)| *weight);
    servers.into_iter().map(|(_, address)| address).collect()
}

/// A step of the map: the patch from one version to the next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub from: String,
    pub to: String,
}

/// The kinds of file a step has.
pub const STEP_FILES: [&str; 4] = ["RTP", "RES", "CHK", "SIG"];

impl Step {
    /// The step's folder on the server.
    pub fn name(&self) -> String {
        format!("v{}-v{}", self.from, self.to)
    }

    /// The name of one of the step's files (`extension` one of [`STEP_FILES`]): the folder's name with dashes for
    /// its dots, in capitals.
    pub fn file(&self, extension: &str) -> String {
        format!("{}.{extension}", self.name().replace('.', "-").to_uppercase())
    }

    /// A file's address on a patch server.
    pub fn address(&self, server: &str, extension: &str) -> String {
        format!("{}/{}/{}", server.trim_end_matches('/'), self.name(), self.file(extension))
    }
}

/// The version the dimension list's `remote` means for a game at `local`: the list says `18.8.62` and the game's
/// `version.id` `18.8.50_EP1`, so the local version's ending from its `_` goes onto the remote one.
pub fn target_version(local: &str, remote: &str) -> String {
    match (remote.contains('_'), local.find('_')) {
        (false, Some(at)) => format!("{remote}{}", &local[at..]),
        _ => remote.to_owned(),
    }
}

/// The steps from `local` to `target`, walking the map once from its top as the patcher does: a line whose first
/// version is the one arrived at is a step to its second. `None` when the walk does not reach the target (the
/// patcher's "Couldn't find any valid patches to apply").
pub fn steps(map: &str, local: &str, target: &str) -> Option<Vec<Step>> {
    let mut at = local.to_owned();
    let mut steps = Vec::new();
    if at.eq_ignore_ascii_case(target) {
        return Some(steps);
    }
    for line in map.lines() {
        let mut words = line.split_whitespace();
        let (Some(from), Some(to)) = (words.next(), words.next()) else { continue };
        if from.eq_ignore_ascii_case(&at) {
            steps.push(Step { from: from.to_owned(), to: to.to_owned() });
            at = to.to_owned();
            if at.eq_ignore_ascii_case(target) {
                return Some(steps);
            }
        }
    }
    None
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(30)).timeout_read(Duration::from_secs(60)).build()
}

fn http(address: &str, e: impl ToString) -> Error {
    Error::Http { address: address.to_owned(), why: e.to_string() }
}

/// A small file's bytes (a list, a map, a checksum file): at most `limit` of them.
pub fn bytes(address: &str, limit: u64) -> Result<Vec<u8>> {
    let address = url(address);
    let response = agent().get(&address).call().map_err(|e| http(&address, e))?;
    let mut out = Vec::new();
    response.into_reader().take(limit + 1).read_to_end(&mut out).map_err(|e| http(&address, e))?;
    if out.len() as u64 > limit {
        return Err(http(&address, "the answer is larger than this file can be"));
    }
    Ok(out)
}

/// A small text file.
pub fn text(address: &str) -> Result<String> {
    Ok(String::from_utf8_lossy(&bytes(address, 1 << 20)?).into_owned())
}

/// Downloads `address` to `path`, going on from what a download that was cut short left there (`<path>.part`).
/// `progress` is told the bytes there are so far and, when the server says it, the file's size; it returns `false`
/// to stop. A file already at `path` is taken to be whole.
pub fn file(address: &str, path: &Path, progress: &mut dyn FnMut(u64, Option<u64>) -> bool) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let address = url(address);
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let part = std::path::PathBuf::from(part);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = OpenOptions::new().create(true).append(true).open(&part)?;
    let mut have = out.metadata()?.len();
    let mut request = agent().get(&address);
    if have > 0 {
        request = request.set("Range", &format!("bytes={have}-"));
    }
    let response = match request.call() {
        Ok(response) => response,
        // (416: the part already holds the whole file)
        Err(ureq::Error::Status(416, _)) if have > 0 => {
            drop(out);
            std::fs::rename(&part, path)?;
            return Ok(());
        }
        Err(e) => return Err(http(&address, e)),
    };
    if have > 0 && response.status() != 206 {
        // The server sends the whole file again: start over.
        out.set_len(0)?;
        out.seek(io::SeekFrom::Start(0))?;
        have = 0;
    }
    let total = response.header("Content-Length").and_then(|n| n.parse::<u64>().ok()).map(|n| n + have);
    let mut reader = response.into_reader();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        if !progress(have, total) {
            return Err(Error::Stopped);
        }
        let n = reader.read(&mut buffer).map_err(|e| http(&address, e))?;
        if n == 0 {
            break;
        }
        out.write_all(&buffer[..n])?;
        have += n as u64;
    }
    if total.is_some_and(|total| have != total) {
        return Err(http(&address, "the connection closed before the file's end"));
    }
    out.flush()?;
    drop(out);
    std::fs::rename(&part, path)?;
    progress(have, total);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_installer_list() {
        let text = "\"dl\" \"http://h/a/install.exe\" \"install.exe\"\r\n\"dl\" \"http://h/a/install-1.bin\" \"install-1.bin\"\r\n\"in\" \"install.exe\"\r\n\r\n\"dl\" \"x\" \"y\"\r\n";
        assert_eq!(
            installer_list(text),
            [
                ListLine::Download { address: "http://h/a/install.exe".into(), file: "install.exe".into() },
                ListLine::Download { address: "http://h/a/install-1.bin".into(), file: "install-1.bin".into() },
                ListLine::Run { file: "install.exe".into() },
            ]
        );
        assert!(plain_name("install-1.bin"));
        assert!(!plain_name("..\\x") && !plain_name("a/b") && !plain_name("c:x") && !plain_name(""));
    }

    #[test]
    fn the_lists() {
        let dimensions = "# remark\nSTARTINFO\ndisplayname = A\nversion         =       18.8.62\nENDINFO\nSTARTINFO\nversion = 18.8.61\nENDINFO\n";
        assert_eq!(dimension_versions(dimensions), ["18.8.62", "18.8.61"]);
        assert_eq!(first_address("# a\r\n#b\r\nhost:80/dir/list.txt\r\n").as_deref(), Some("host:80/dir/list.txt"));
        assert_eq!(patch_servers_address("host:80/dir/list.txt"), "http://host:80/dir/patchservers.txt");
        assert_eq!(patch_servers("Live;5;http://b:80/c\r\nLive;1;http://a:80/c/\r\n"), ["http://a:80/c", "http://b:80/c"]);
    }

    #[test]
    fn the_map() {
        let map = "18.8.4_EP1 18.8.5_EP1\r\n18.8.49_EP1 18.8.50_EP1\r\n18.8.49_EP2 18.8.50_EP2\r\n18.8.50_EP1 18.8.51_EP1\r\n18.8.50_EP2 18.8.51_EP2\r\n18.8.51_EP1 18.8.52_EP1\r\n";
        assert_eq!(target_version("18.8.50_EP1", "18.8.52"), "18.8.52_EP1");
        assert_eq!(target_version("18.8.50_EP1", "18.8.52_EP2"), "18.8.52_EP2");
        assert_eq!(target_version("18.8.50", "18.8.52"), "18.8.52");
        let steps = steps(map, "18.8.50_EP1", "18.8.52_EP1").unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].name(), "v18.8.50_EP1-v18.8.51_EP1");
        assert_eq!(steps[0].file("RTP"), "V18-8-50_EP1-V18-8-51_EP1.RTP");
        assert_eq!(
            steps[1].address("http://update.example:80/c/", "CHK"),
            "http://update.example:80/c/v18.8.51_EP1-v18.8.52_EP1/V18-8-51_EP1-V18-8-52_EP1.CHK"
        );
        assert_eq!(super::steps(map, "18.8.52_EP1", "18.8.52_EP1"), Some(Vec::new()));
        assert_eq!(super::steps(map, "18.8.50_EP1", "18.8.53_EP1"), None);
        assert_eq!(super::steps(map, "17.0_EP1", "18.8.52_EP1"), None);
    }
}
