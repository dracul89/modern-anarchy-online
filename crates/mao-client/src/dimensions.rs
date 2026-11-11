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
}

impl Default for Prefs {
    fn default() -> Self {
        // (the original's own defaults: its Connection page, `GeneralNetworkTimeout` and `DimensionNetworkTimeout`)
        Prefs { dimension: String::new(), network_timeout: 90, dimension_timeout: 30 }
    }
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
                "network_timeout" => p.network_timeout = value.parse().unwrap_or(p.network_timeout).clamp(5, 600),
                "dimension_timeout" => p.dimension_timeout = value.parse().unwrap_or(p.dimension_timeout).clamp(2, 300),
                _ => {}
            }
        }
        p
    }

    pub fn to_text(&self) -> String {
        format!("dimension = {}\nnetwork_timeout = {}\ndimension_timeout = {}\n", self.dimension, self.network_timeout, self.dimension_timeout)
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_url_files_give_their_addresses() {
        assert_eq!(first_address("# do not edit\n#\nlists.example.org:80/dimensions.txt\n\n").as_deref(), Some("lists.example.org:80/dimensions.txt"));
        assert_eq!(first_address("# only comments\n"), None);
        let file = "# codes\nSHOWURL=http://pages.example.org/launcher/index.html\nERRORURL=http://pages.example.org/errors/\n1=331\n";
        assert_eq!(named_value(file, "showurl").as_deref(), Some("http://pages.example.org/launcher/index.html"));
        assert_eq!(named_value(file, "LOGINURL"), None);
    }

    #[test]
    fn the_launchers_file_keeps_the_dimension_and_the_time_outs() {
        let p = Prefs { dimension: "Second World".into(), network_timeout: 120, dimension_timeout: 10 };
        assert_eq!(Prefs::from_text(&p.to_text()), p);
        assert_eq!(Prefs::from_text(""), Prefs::default());
        assert_eq!(Prefs::from_text("network_timeout = 100000\n").network_timeout, 600);
    }
}
