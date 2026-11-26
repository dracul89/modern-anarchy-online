//! Funcom's patches applied to the real files. Needs `MAO_PATCHES`, a folder holding the patch servers' files for
//! the steps from 18.8.50_EP1 on (`V18-8-50_EP1-V18-8-51_EP1.RTP` and so on), and `MAO_GAME`, an installed game whose
//! `PatchesUndo` folder still has the files RTPatch kept at each step (`Gamecode.A257.dll`, ...): the folder says what
//! every loose file was at every version. Without them the tests pass without doing anything.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use mao_install::rtpatch::{self, Tree};

fn folders() -> Option<(PathBuf, PathBuf)> {
    let patches = PathBuf::from(std::env::var_os("MAO_PATCHES")?);
    let game = PathBuf::from(std::env::var_os("MAO_GAME")?);
    Some((patches, game))
}

/// `PatchesUndo`'s marks by the version whose files they are: `version.A257.id` holds `18.8.50_EP1`.
fn marks(undo: &Path) -> HashMap<String, Vec<String>> {
    let mut by_version: HashMap<String, Vec<String>> = HashMap::new();
    for entry in std::fs::read_dir(undo).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let parts: Vec<&str> = name.split('.').collect();
        if let ["version", mark, "id"] = parts[..] {
            let version = std::fs::read_to_string(entry.path()).unwrap().trim().to_owned();
            by_version.entry(version).or_default().push(mark.to_owned());
        }
    }
    // (two installs went through the same steps: the first mark in the alphabet is the one whose step kept
    // everything the patch touches)
    by_version.values_mut().for_each(|marks| marks.sort());
    by_version
}

/// The old files of one step, read from `PatchesUndo` by their mark; what the patch writes is kept in memory.
struct Kept<'a> {
    undo: &'a Path,
    mark: &'a str,
    written: HashMap<String, Vec<u8>>,
}

fn kept_path(undo: &Path, mark: &str, path: &str) -> PathBuf {
    let name = path.rsplit('\\').next().unwrap();
    let kept = match name.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem}.{mark}.{ext}"),
        None => format!("{name}.{mark}"),
    };
    undo.join(kept)
}

impl Tree for Kept<'_> {
    fn read(&mut self, path: &str) -> io::Result<Option<Vec<u8>>> {
        match std::fs::read(kept_path(self.undo, self.mark, path)) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn write(&mut self, path: &str, data: &[u8], _modified: Option<SystemTime>) -> io::Result<()> {
        self.written.insert(path.to_owned(), data.to_vec());
        Ok(())
    }

    fn remove(&mut self, path: &str) -> io::Result<()> {
        self.written.remove(path);
        Ok(())
    }
}

fn step_name(from: u32) -> String {
    format!("V18-8-{from}_EP1-V18-8-{}_EP1", from + 1)
}

/// Every step's `.RTP` applied to the files of its version, and the result compared with the files `PatchesUndo`
/// has for the next version.
#[test]
fn loose_files_of_every_step() {
    let Some((patches, game)) = folders() else { return };
    let undo = game.join("PatchesUndo");
    let marks = marks(&undo);
    let mut steps = 0;
    for from in 50..62u32 {
        let version = format!("18.8.{from}_EP1");
        let next = format!("18.8.{}_EP1", from + 1);
        let Some(mark) = marks.get(&version).and_then(|m| m.first()) else { continue };
        let patch = std::fs::read(patches.join(step_name(from) + ".RTP")).unwrap();
        let mut tree = Kept { undo: &undo, mark, written: HashMap::new() };
        let report = rtpatch::apply(&patch, &mut tree).unwrap_or_else(|e| panic!("{version}: {e}"));
        assert!(report.changed.len() >= 32, "{version}: {report:?}");
        assert!(report.passed_over.is_empty(), "{version}: {report:?}");
        assert_eq!(tree.written.get("version.id").map(|v| String::from_utf8_lossy(v).trim().to_owned()), Some(next.clone()));
        // (the next version's kept files: the same bytes)
        let mut compared = 0;
        if let Some(next_mark) = marks.get(&next).and_then(|m| m.first()) {
            for (path, data) in &tree.written {
                if let Ok(there) = std::fs::read(kept_path(&undo, next_mark, path)) {
                    assert!(*data == there, "{version}: {path} differs from the kept {next} file");
                    compared += 1;
                }
            }
            assert!(compared >= 34, "{version}: only {compared} files to compare");
        }
        // (and a second time over its own result: nothing left to do)
        let mut again = Kept { undo: &undo, mark, written: HashMap::new() };
        let patched = tree.written.clone();
        struct Both<'a>(&'a HashMap<String, Vec<u8>>, &'a mut Kept<'a>);
        impl Tree for Both<'_> {
            fn read(&mut self, path: &str) -> io::Result<Option<Vec<u8>>> {
                match self.0.get(path) {
                    Some(data) => Ok(Some(data.clone())),
                    None => self.1.read(path),
                }
            }
            fn write(&mut self, path: &str, data: &[u8], modified: Option<SystemTime>) -> io::Result<()> {
                self.1.write(path, data, modified)
            }
            fn remove(&mut self, path: &str) -> io::Result<()> {
                self.1.remove(path)
            }
        }
        let second = rtpatch::apply(&patch, &mut Both(&patched, &mut again)).unwrap();
        assert!(second.changed.is_empty() && second.added.is_empty(), "{version}: {second:?}");
        steps += 1;
    }
    assert_eq!(steps, 12);
}

/// A patch for another version of a file is refused, and nothing is written.
#[test]
fn the_wrong_version_is_refused() {
    let Some((patches, game)) = folders() else { return };
    let undo = game.join("PatchesUndo");
    let marks = marks(&undo);
    let Some(mark) = marks.get("18.8.55_EP1").and_then(|m| m.first()) else { return };
    let patch = std::fs::read(patches.join(step_name(50) + ".RTP")).unwrap();
    let mut tree = Kept { undo: &undo, mark, written: HashMap::new() };
    match rtpatch::apply(&patch, &mut tree) {
        Err(rtpatch::Error::OldFile { missing: false, .. }) => {}
        other => panic!("{other:?}"),
    }
    assert!(tree.written.is_empty());
}

/// Every step's checksum list carries Funcom's signature for the key the game has, and names the MD5s of the step's
/// two patch files; a list with one byte changed does not pass. The game's own `checksums.chk` is signed the same way.
#[test]
fn signatures_and_checksums_of_every_step() {
    let Some((patches, game)) = folders() else { return };
    let launcher = game.join("cd_image/data/launcher");
    let key = std::fs::read(launcher.join("funcom.pub")).unwrap();
    for from in 50..62u32 {
        let name = step_name(from);
        let read = |extension: &str| std::fs::read(patches.join(format!("{name}.{extension}"))).unwrap();
        let (rtp, res, chk, sig) = (read("RTP"), read("RES"), read("CHK"), read("SIG"));
        let files = [(format!("{name}.RTP"), rtp), (format!("{name}.RES"), res)];
        let files: Vec<(&str, &[u8])> = files.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
        mao_install::patcher::verify_step(&key, &chk, &sig, &files).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut other = chk.clone();
        *other.last_mut().unwrap() ^= 1;
        assert!(mao_install::patcher::verify_step(&key, &other, &sig, &files).is_err());
        let mut damaged = files.clone();
        let short = &files[0].1[1..];
        damaged[0].1 = short;
        assert!(mao_install::patcher::verify_step(&key, &chk, &sig, &damaged).is_err());
    }
    let (chk, sig) = (std::fs::read(launcher.join("checksums.chk")).unwrap(), std::fs::read(launcher.join("checksums.chk.sig")).unwrap());
    mao_install::patcher::verify_step(&key, &chk, &sig, &[]).unwrap();
}
