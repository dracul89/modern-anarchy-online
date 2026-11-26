//! From nothing to a game folder: what Funcom's downloader (`AnarchyOnline_EP1.exe`) and the installer it fetches
//! do, without running either.
//!
//! 1. The downloader's list is fetched ([`fetch::INSTALLER_LIST`]) and the parts it names downloaded into a folder
//!    of their own: `install.exe` and its `install-N.bin` slices, 2.1 GB. A download cut short goes on where it
//!    stopped.
//! 2. `install.exe` is checked to be one this code has been tried on ([`KNOWN_INSTALLERS`]). The servers speak plain
//!    HTTP only and the installer carries no signature, so its SHA-1 is kept here; the SHA-1 of every file in the
//!    package is in `install.exe`, so the slices are checked through it as they are unpacked.
//! 3. The game's licence is put before the player. Funcom's package has no licence page of its own (the downloader
//!    shows the licence before it fetches the package, and the game's launcher shows `anarchy.eula` before the
//!    first start), so it is the package's `anarchy.eula`, unpacked by itself before anything else.
//! 4. The files that the installer copies into the game folder are unpacked there ([`crate::inno`]). What else it
//!    does on Windows (shortcuts, the uninstaller, registry entries) is left out: nothing in the game folder comes
//!    of it, and this client reads none of it.
//! 5. The game is patched from the installer's version to the servers' ([`crate::patcher`]).
//!
//! Nothing downloaded is ever run.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::fetch::{self, ListLine};
use crate::patcher::{self, Event, Progress};
use crate::{crypto, inno, sha1};

const CARRIAGE_RETURN: char = 13 as char;

/// The installers this code was tried on: the SHA-1 of `install.exe`, and the version it installs.
pub const KNOWN_INSTALLERS: [(&str, &str); 1] = [("7a2ba7559485bfa9362601be7a1a482ccdf3c9c0", "18.8.50_EP1")];

#[derive(Debug)]
pub enum Error {
    Fetch(fetch::Error),
    /// The list of the installer's parts is not what the downloader's script reads.
    List(String),
    /// `install.exe` is not one of [`KNOWN_INSTALLERS`] (its SHA-1 is given).
    UnknownInstaller(String),
    Package(inno::Error),
    Patch(patcher::Error),
    /// The folder to install into has something in it already.
    NotEmpty(PathBuf),
    /// The licence was not accepted.
    Declined,
    Io(std::io::Error),
    Stopped,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Fetch(e) => write!(f, "{e}"),
            Error::List(what) => write!(f, "Funcom's list of the installer's parts: {what}"),
            Error::UnknownInstaller(sum) => write!(
                f,
                "Funcom's installer is not one this version knows (install.exe has the SHA-1 {sum}): it may be a newer one, or a damaged download"
            ),
            Error::Package(e) => write!(f, "{e}"),
            Error::Patch(e) => write!(f, "{e}"),
            Error::NotEmpty(folder) => write!(f, "{} is not empty", folder.display()),
            Error::Declined => write!(f, "the licence was not accepted"),
            Error::Io(e) => write!(f, "{e}"),
            Error::Stopped => write!(f, "stopped"),
        }
    }
}

impl std::error::Error for Error {}

impl From<fetch::Error> for Error {
    fn from(e: fetch::Error) -> Error {
        match e {
            fetch::Error::Stopped => Error::Stopped,
            e => Error::Fetch(e),
        }
    }
}

impl From<inno::Error> for Error {
    fn from(e: inno::Error) -> Error {
        Error::Package(e)
    }
}

impl From<patcher::Error> for Error {
    fn from(e: patcher::Error) -> Error {
        match e {
            patcher::Error::Stopped => Error::Stopped,
            e => Error::Patch(e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// The installer's parts as the downloader's list names them: each part's address and file name, and the name of
/// the one the downloader would run.
pub fn parts(list: &str) -> Result<(Vec<(String, String)>, String)> {
    let mut parts = Vec::new();
    let mut run = None;
    for line in fetch::installer_list(list) {
        match line {
            ListLine::Download { address, file } => {
                if !fetch::plain_name(&file) || !(address.starts_with("http://") || address.starts_with("https://")) {
                    return Err(Error::List(format!("an entry that is no plain download ({address} to {file})")));
                }
                parts.push((address, file));
            }
            ListLine::Run { file } => {
                run = Some(file);
                break;
            }
        }
    }
    match run {
        Some(run) if parts.iter().any(|(_, file)| *file == run) => Ok((parts, run)),
        _ => Err(Error::List("it names no installer to run".into())),
    }
}

/// The installer, downloaded and opened.
pub struct Installer {
    pub package: inno::Package,
    /// The version it installs, when `install.exe` is one of [`KNOWN_INSTALLERS`].
    pub version: Option<&'static str>,
    plan: Vec<inno::PlannedFile>,
}

/// SHA-1 of a file, read in pieces.
fn sha1_of(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = sha1::Sha1::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let n = std::io::Read::read(&mut file, &mut buffer)?;
        if n == 0 {
            return Ok(crypto::hex(&hash.finish()));
        }
        hash.update(&buffer[..n]);
    }
}

impl Installer {
    /// Downloads the installer's parts into `downloads` (what is there already is kept) and opens the package.
    /// An installer that is not one of [`KNOWN_INSTALLERS`] is refused unless `allow_unknown`.
    pub fn fetch(downloads: &Path, allow_unknown: bool, progress: &mut Progress) -> Result<Installer> {
        progress(Event::Status("Asking Funcom's server for the installer's parts..."));
        let (parts, run) = parts(&fetch::text(fetch::INSTALLER_LIST)?)?;
        std::fs::create_dir_all(downloads)?;
        progress(Event::Status("Downloading Funcom's installer..."));
        for (address, file) in &parts {
            let mut tell = |done, total| progress(Event::Download { file, done, total });
            fetch::file(address, &downloads.join(file), &mut tell)?;
        }
        progress(Event::Status("Checking the installer..."));
        Installer::open(&downloads.join(run), allow_unknown)
    }

    /// Opens a downloaded installer by its `install.exe`.
    pub fn open(exe: &Path, allow_unknown: bool) -> Result<Installer> {
        let sum = sha1_of(exe)?;
        let version = KNOWN_INSTALLERS.iter().find(|(known, _)| *known == sum).map(|(_, version)| *version);
        if version.is_none() && !allow_unknown {
            return Err(Error::UnknownInstaller(sum));
        }
        let package = inno::Package::open_exe(exe)?;
        let plan = package.plan_app();
        Ok(Installer { package, version, plan })
    }

    /// The game's licence: the package's own licence page when it has one, else its `anarchy.eula` (Funcom's
    /// package has no page: its `license_text` is empty), unpacked by itself into a folder under `scratch` and
    /// read (it lies 1.3 MB into the packed stream, so this is quick). Empty when the package has neither.
    pub fn licence(&self, scratch: &Path) -> Result<String> {
        let own = &self.package.header().license_text;
        if !own.trim().is_empty() {
            return Ok(own.clone());
        }
        let Some(planned) = self.plan.iter().find(|f| f.path.eq_ignore_ascii_case("anarchy.eula")) else {
            return Ok(String::new());
        };
        let folder = scratch.join("mao-licence");
        let one = [inno::PlannedFile { file: planned.file, path: "anarchy.eula".into() }];
        self.package.extract(&folder, &one, &mut |_| true)?;
        let bytes = std::fs::read(folder.join("anarchy.eula"))?;
        let _ = std::fs::remove_dir_all(&folder);
        // (read as the bytes are, one character each, as the game's launcher shows it)
        Ok(bytes.iter().map(|&b| b as char).filter(|&c| c != CARRIAGE_RETURN).collect())
    }

    /// The bytes the game folder will hold, and the number of files.
    pub fn size(&self) -> (u64, usize) {
        (self.package.plan_size(&self.plan), self.plan.len())
    }

    /// Unpacks the game's files into `game`.
    pub fn unpack(&self, game: &Path, progress: &mut Progress) -> Result<()> {
        std::fs::create_dir_all(game)?;
        let mut tell = |p: inno::Progress| progress(Event::Unpack { file: p.current, done: p.bytes_done, total: p.bytes_total });
        match self.package.extract(game, &self.plan, &mut tell) {
            Ok(_) => Ok(()),
            // (the file it was at is taken away again; the folder keeps its mark of not being whole)
            Err(inno::Error::Stopped) => Err(Error::Stopped),
            Err(e) => Err(e.into()),
        }
    }
}

/// Whether a folder is free to install into: not there, or empty (an earlier try's leftovers apart: the folder
/// the downloads are kept in when it lies inside).
pub fn is_free(game: &Path, downloads: &Path) -> bool {
    match std::fs::read_dir(game) {
        Err(_) => !game.exists(),
        Ok(entries) => entries.flatten().all(|entry| entry.path() == downloads),
    }
}

/// Where the installer's parts are kept while a game is installed into `game`, unless the caller says otherwise.
pub fn default_downloads(game: &Path) -> PathBuf {
    game.join("mao-installer")
}

/// The file that marks a game folder as not yet wholly unpacked (`version.id` is among the first files to arrive,
/// so its being there says nothing).
pub const UNPACKING: &str = "mao-install.unpacking";

/// The whole install: download, check, licence, unpack, patch. `accept` is shown the licence and says whether the
/// player accepts it (it is not called for a package that has none). A folder that already holds the unpacked game
/// is only patched: so a run that was cut short after the unpacking goes on with the patches, and the licence is
/// not asked for twice. One that was cut short in the unpacking unpacks again, without asking again.
///
/// The downloaded parts are deleted at the end unless `keep_downloads`.
pub fn install(
    game: &Path,
    downloads: &Path,
    keep_downloads: bool,
    allow_unknown: bool,
    accept: &mut dyn FnMut(&str) -> bool,
    progress: &mut Progress,
) -> Result<()> {
    let marker = game.join(UNPACKING);
    if marker.exists() || patcher::local_version(game).is_err() {
        if !marker.exists() && !is_free(game, downloads) {
            return Err(Error::NotEmpty(game.to_owned()));
        }
        let installer = Installer::fetch(downloads, allow_unknown, progress)?;
        // (the mark is only there once the licence was accepted)
        if !marker.exists() {
            let licence = installer.licence(downloads)?;
            if !licence.trim().is_empty() && !accept(&licence) {
                return Err(Error::Declined);
            }
        }
        std::fs::create_dir_all(game)?;
        std::fs::write(&marker, b"")?;
        progress(Event::Status("Unpacking the game..."));
        installer.unpack(game, progress)?;
        std::fs::remove_file(&marker)?;
    }
    patcher::patch(game, progress)?;
    if !keep_downloads && downloads.is_dir() {
        // (only what the list named and its unfinished pieces: never a folder's other files)
        for entry in std::fs::read_dir(downloads)?.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name.starts_with("install") && (name.ends_with(".exe") || name.ends_with(".bin") || name.ends_with(".part")) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(downloads);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parts() {
        let list = "\"dl\" \"http://h/a/install.exe\" \"install.exe\"\r\n\"dl\" \"http://h/a/install-1.bin\" \"install-1.bin\"\r\n\"in\" \"install.exe\"\r\n";
        let (parts, run) = parts(list).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(run, "install.exe");
        // (a part that would land outside the folder, a list that runs something it did not download, an empty one)
        assert!(super::parts("\"dl\" \"http://h/x\" \"..\\\\x.exe\"\r\n\"in\" \"x.exe\"\r\n").is_err());
        assert!(super::parts("\"dl\" \"http://h/x\" \"x.exe\"\r\n\"in\" \"y.exe\"\r\n").is_err());
        assert!(super::parts("\"dl\" \"file:///etc/passwd\" \"x.exe\"\r\n\"in\" \"x.exe\"\r\n").is_err());
        assert!(super::parts("").is_err());
    }

    #[test]
    fn a_free_folder() {
        let folder = std::env::temp_dir().join(format!("mao-install-free-{}", std::process::id()));
        let downloads = default_downloads(&folder);
        assert!(is_free(&folder, &downloads));
        std::fs::create_dir_all(&downloads).unwrap();
        assert!(is_free(&folder, &downloads));
        std::fs::write(folder.join("other.txt"), b"x").unwrap();
        assert!(!is_free(&folder, &downloads));
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
