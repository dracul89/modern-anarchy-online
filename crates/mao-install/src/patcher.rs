//! Bringing an installed game up to the version the servers run, as Funcom's `AnarchyPatcher.exe` does: the patch
//! servers' map walked from the game's `version.id`, and each step's files fetched, checked and applied.
//!
//! A step (`docs/INSTALL_WITHOUT_WINDOWS.md` has the reading of the original):
//!
//! 1. its four files into the game's `Patches` folder, where the original keeps them too (files already there are
//!    used, so patches fetched by hand or by the original patcher serve);
//! 2. the `.SIG` checked to be Funcom's signature of the `.CHK`, with the key the game itself carries
//!    (`cd_image/data/launcher/funcom.pub`), and the `.RTP` and `.RES` checked against the `.CHK`'s MD5s;
//! 3. the `.RES` into the resource database ([`crate::respatch`]), then the `.RTP` over the loose files
//!    ([`crate::rtpatch`]): the original's order, and the right one, since the `.RTP` is what moves `version.id` on.
//!    A step cut short between the two starts again from the top, and both leave alone what is already done;
//! 4. `rename.ini`, which a step leaves behind for the launcher (`Anarchy.exe`: "Performing patch cleanup..."): each
//!    line `new=old` a file to move over another (the new patcher arrives as `ap.exe`); then it is deleted;
//! 5. the step's files deleted from `Patches`.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::fetch::{self, Step, STEP_FILES};
use crate::rtpatch::{Folder, Tree};
use crate::{crypto, ctree, respatch, rtpatch, sha1};

#[derive(Debug)]
pub enum Error {
    /// The folder is no game folder, or misses a file the patcher goes by.
    Game(String),
    Fetch(fetch::Error),
    /// A downloaded file is not what Funcom signed.
    Verify(String),
    /// The map has no way from the game's version to the servers'.
    NoPath { local: String, target: String },
    Files(rtpatch::Error),
    Records(respatch::Error),
    Io(std::io::Error),
    Stopped,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Game(what) => write!(f, "{what}"),
            Error::Fetch(e) => write!(f, "{e}"),
            Error::Verify(what) => write!(f, "{what}"),
            Error::NoPath { local, target } => {
                write!(f, "the patch servers have no patches that lead from version {local} to {target}")
            }
            Error::Files(e) => write!(f, "{e}"),
            Error::Records(e) => write!(f, "{e}"),
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

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// What the patcher is doing, for whoever shows it.
#[derive(Clone, Copy, Debug)]
pub enum Event<'a> {
    /// A line to show.
    Status(&'a str),
    /// A file being downloaded: the bytes there so far, and its size when the server says it.
    Download { file: &'a str, done: u64, total: Option<u64> },
    /// A step being applied: which of how many.
    Step { index: usize, count: usize, from: &'a str, to: &'a str },
    /// The step's database records: how many of how many are in.
    Records { done: usize, total: usize },
    /// The installer's files being unpacked: the one being written, and the bytes written of all there are.
    Unpack { file: &'a str, done: u64, total: u64 },
    /// A step's files are fetched and checked, and the game's own files change from here until the next `Step` or
    /// the end: the one stretch in which the program must not be ended (the database would be left half written).
    /// `false` in answer stops before anything is changed.
    Writing,
}

/// Told every [`Event`]; returns `false` to stop: a download where it is (it goes on from there the next time), the
/// unpacking at the file it is at (the folder is unpacked anew the next time), the patching before a step's
/// [`Event::Writing`]. Once a step is writing it is finished.
pub type Progress<'a> = dyn FnMut(Event) -> bool + 'a;

/// What there is to do for a game folder.
#[derive(Clone, Debug)]
pub struct Plan {
    /// The game's version (`version.id`).
    pub local: String,
    /// The version the dimension list gives, with the game's own ending (`_EP1`).
    pub target: String,
    /// The patch servers, the most wanted first.
    pub servers: Vec<String>,
    pub steps: Vec<Step>,
}

/// The first line of the game's `version.id`.
pub fn local_version(game: &Path) -> Result<String> {
    let path = Folder::new(game).locate("version.id");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| Error::Game(format!("{}: {e} (is this an Anarchy Online folder?)", path.display())))?;
    match text.lines().next().map(str::trim) {
        Some(version) if !version.is_empty() => Ok(version.to_owned()),
        _ => Err(Error::Game(format!("{} is empty", path.display()))),
    }
}

/// Asks the servers what there is to do: the dimension list for the version, the patch servers, the map.
pub fn plan(game: &Path, progress: &mut Progress) -> Result<Plan> {
    let local = local_version(game)?;
    let list_file = Folder::new(game).locate("cd_image\\data\\launcher\\DimensionServer.url");
    let list_address = std::fs::read_to_string(&list_file)
        .ok()
        .and_then(|text| fetch::first_address(&text))
        .ok_or_else(|| Error::Game(format!("{} does not name the dimension server", list_file.display())))?;
    progress(Event::Status("Downloading dimension information..."));
    let versions = fetch::dimension_versions(&fetch::text(&list_address)?);
    // (the dimensions all run one version; were they ever to differ, the newest the map leads to is the one)
    let Some(remote) = versions.first() else {
        return Err(Error::Verify("the dimension server's answer names no version".into()));
    };
    let target = fetch::target_version(&local, remote);
    if local.eq_ignore_ascii_case(&target) {
        return Ok(Plan { local, target, servers: Vec::new(), steps: Vec::new() });
    }
    progress(Event::Status("Downloading patchserver information, please wait..."));
    let servers = fetch::patch_servers(&fetch::text(&fetch::patch_servers_address(&list_address))?);
    let mut last = None;
    for server in &servers {
        progress(Event::Status("Getting mapfile..."));
        match fetch::text(&format!("{server}/map")) {
            Ok(map) => {
                let Some(steps) = fetch::steps(&map, &local, &target) else {
                    return Err(Error::NoPath { local, target });
                };
                // (the server that answered first from here on)
                let mut servers = servers.clone();
                servers.sort_by_key(|s| s != server);
                return Ok(Plan { local, target, servers, steps });
            }
            Err(e) => last = Some(e),
        }
    }
    Err(last.map_or_else(|| Error::Verify("the patch-server list names no server".into()), Error::from))
}

/// Checks a step's files as the patcher does: the `.SIG` is the signature of the `.CHK` for `key` (the text of
/// `funcom.pub`), and each other file has the MD5 the `.CHK` gives for its name.
pub fn verify_step(key: &[u8], chk: &[u8], sig: &[u8], files: &[(&str, &[u8])]) -> Result<()> {
    let mut hash = sha1::Sha1::new();
    hash.update(chk);
    if !crypto::rsa_sha1_verify(&crypto::unhex(key), &hash.finish(), &crypto::unhex(sig)) {
        return Err(Error::Verify("a patch's checksum list is not signed by Funcom's key".into()));
    }
    let list = crypto::checksum_list(&String::from_utf8_lossy(chk));
    for (name, data) in files {
        match list.iter().find(|(listed, _)| listed.eq_ignore_ascii_case(name)) {
            Some((_, sum)) if *sum == crypto::md5(data) => {}
            Some(_) => return Err(Error::Verify(format!("{name} is not the file Funcom's checksum list names (damaged download?)"))),
            None => return Err(Error::Verify(format!("{name} is not in its checksum list"))),
        }
    }
    Ok(())
}

/// The resource database's path without its extensions.
pub fn database_path(game: &Path) -> PathBuf {
    Folder::new(game).locate("cd_image\\data\\db\\ResourceDatabase.dat").with_extension("")
}

/// `rename.ini`, as the launcher works it off after a patch: `new=old` lines; the old file moved over the new.
pub fn finish_renames(game: &Path) -> Result<Vec<(String, String)>> {
    let mut tree = Folder::new(game);
    let Some(text) = tree.read("rename.ini")? else { return Ok(Vec::new()) };
    let mut done = Vec::new();
    for line in String::from_utf8_lossy(&text).lines() {
        let Some((new, old)) = line.split_once('=') else { continue };
        let (new, old) = (new.trim(), old.trim());
        if new.is_empty() || old.is_empty() || !fetch::plain_name(new) || !fetch::plain_name(old) {
            continue;
        }
        let from = tree.locate(old);
        if from.exists() {
            let to = tree.locate(new);
            let _ = std::fs::remove_file(&to);
            std::fs::rename(&from, &to)?;
            done.push((new.to_owned(), old.to_owned()));
        }
    }
    tree.remove("rename.ini")?;
    Ok(done)
}

/// Fetches, checks and applies one step.
fn step(game: &Path, servers: &[String], step: &Step, progress: &mut Progress) -> Result<()> {
    let patches = Folder::new(game).locate("Patches");
    let mut data = Vec::new();
    for extension in STEP_FILES {
        let name = step.file(extension);
        let path = patches.join(&name);
        let mut last = None;
        for server in servers {
            let mut going = true;
            let mut tell = |done, total| {
                going = progress(Event::Download { file: &name, done, total });
                going
            };
            match fetch::file(&step.address(server, extension), &path, &mut tell) {
                Ok(()) => {
                    last = None;
                    break;
                }
                Err(fetch::Error::Stopped) => return Err(Error::Stopped),
                Err(e) => last = Some(e),
            }
        }
        if let Some(e) = last {
            return Err(e.into());
        }
        data.push((name, std::fs::read(&path)?));
    }
    let [(rtp_name, rtp), (res_name, res), (_, chk), (_, sig)] = &data[..] else { unreachable!() };
    let key_path = Folder::new(game).locate("cd_image\\data\\launcher\\funcom.pub");
    let key = std::fs::read(&key_path).map_err(|e| Error::Game(format!("{}: {e}", key_path.display())))?;
    if let Err(e) = verify_step(&key, chk, sig, &[(rtp_name, rtp), (res_name, res)]) {
        // (a bad download must not be met again on the next try)
        for (name, _) in &data {
            let _ = std::fs::remove_file(patches.join(name));
        }
        return Err(e);
    }
    if !progress(Event::Writing) {
        return Err(Error::Stopped);
    }
    progress(Event::Status("Patching the resource database..."));
    if respatch::ResPatch::parse(res).map_err(Error::Records)?.is_empty() {
        // (nothing to do: the database's files are not opened for writing at all)
    } else {
        let base = database_path(game);
        let failed = |e: ctree::Error| Error::Game(format!("{}: the database could not be opened (c-tree error {})", base.display(), e.0));
        let mut db = ctree::Database::open_writable(&base).map_err(failed)?;
        let mut tell = |done, total| {
            progress(Event::Records { done, total });
        };
        respatch::apply(res, &mut db, &mut tell).map_err(Error::Records)?;
        db.close().map_err(failed)?;
    }
    progress(Event::Status("Patching files..."));
    rtpatch::apply(rtp, &mut Folder::new(game)).map_err(Error::Files)?;
    finish_renames(game)?;
    for (name, _) in &data {
        let _ = std::fs::remove_file(patches.join(name));
    }
    Ok(())
}

/// Applies a plan's steps, one after the other. The game's `version.id` says how far it got if it is cut short.
pub fn run(game: &Path, plan: &Plan, progress: &mut Progress) -> Result<()> {
    for (index, s) in plan.steps.iter().enumerate() {
        if !progress(Event::Step { index, count: plan.steps.len(), from: &s.from, to: &s.to }) {
            return Err(Error::Stopped);
        }
        step(game, &plan.servers, s, progress)?;
        let now = local_version(game)?;
        if !now.eq_ignore_ascii_case(&s.to) {
            return Err(Error::Verify(format!("after the patch to {} the game says it is {now}", s.to)));
        }
    }
    Ok(())
}

/// Brings the game at `game` up to the servers' version: [`plan`], then [`run`].
pub fn patch(game: &Path, progress: &mut Progress) -> Result<Plan> {
    let plan = plan(game, progress)?;
    run(game, &plan, progress)?;
    Ok(plan)
}
