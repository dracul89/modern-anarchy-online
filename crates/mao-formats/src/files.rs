//! Opening the game's files by the names the client builds for them, on any system.
//!
//! The game folder is a Windows install: its files' names are mixed case (`cd_image/sound/sfx/player`, `Setupf`,
//! `gui/Default/Graphics.uvga`) and the original opened them by names of any case, with `\` between folders, which Windows
//! takes as the same file. A case-sensitive file system (Linux's, and macOS's when formatted so) takes neither, so every
//! name the client builds into the game folder or a skin goes through [`find`]: the path as it is on the disk, each part
//! that isn't there as written matched against its folder's entries ignoring ASCII case, and `\` taken as a folder
//! separator. On Windows the path is used as it is, as before.

use std::io;
use std::path::{Component, Path, PathBuf};

/// The file or folder `path` names, as it is on the disk: `path` itself when it exists as written (always on Windows),
/// else each missing part matched to its folder's entry of the same name in another case. A part that has no match is
/// kept as written, so the result of a missing file is still missing.
pub fn find(path: &Path) -> PathBuf {
    if cfg!(windows) {
        return path.to_path_buf();
    }
    // (a name with `\` between its folders, as the game's data writes them)
    let path = match path.to_str() {
        Some(s) if s.contains('\\') => PathBuf::from(s.replace('\\', "/")),
        _ => path.to_path_buf(),
    };
    if path.exists() {
        return path;
    }
    let mut out = PathBuf::new();
    let mut lost = false;
    for part in path.components() {
        let Component::Normal(name) = part else {
            out.push(part);
            continue;
        };
        let exact = out.join(name);
        if lost || exact.exists() {
            out = exact;
            continue;
        }
        let dir = if out.as_os_str().is_empty() { Path::new(".") } else { out.as_path() };
        let found = name.to_str().and_then(|name| {
            std::fs::read_dir(dir).ok()?.flatten().map(|e| e.file_name()).find(|e| e.to_str().is_some_and(|e| e.eq_ignore_ascii_case(name)))
        });
        match found {
            Some(on_disk) => out.push(on_disk),
            None => {
                lost = true;
                out = exact;
            }
        }
    }
    out
}

/// `std::fs::read` of the file [`find`] finds.
pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    std::fs::read(find(path.as_ref()))
}

/// `std::fs::read_to_string` of the file [`find`] finds.
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    std::fs::read_to_string(find(path.as_ref()))
}

/// `std::fs::File::open` of the file [`find`] finds.
pub fn open(path: impl AsRef<Path>) -> io::Result<std::fs::File> {
    std::fs::File::open(find(path.as_ref()))
}

/// Whether the file or folder exists in any case.
pub fn exists(path: impl AsRef<Path>) -> bool {
    find(path.as_ref()).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_in_another_case_finds_the_file() {
        let root = std::env::temp_dir().join(format!("mao-files-test-{}", std::process::id()));
        let dir = root.join("Cd_Image").join("Sound").join("sfx").join("player");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Hello.WAV"), b"x").unwrap();
        assert_eq!(read(root.join("cd_image/sound/sfx/Player/hello.wav")).unwrap(), b"x");
        assert!(exists(root.join("CD_IMAGE/SOUND")));
        assert!(!exists(root.join("cd_image/sound/sfx/player/missing.wav")));
        assert!(read(root.join("cd_image/nothing/hello.wav")).is_err());
        if !cfg!(windows) {
            // (the parts as they are on the disk, and `\` a folder separator)
            assert_eq!(find(&root.join("cd_image\\sound\\SFX\\player\\hello.wav")), dir.join("Hello.WAV"));
            assert_eq!(find(&root.join("cd_image/sound/x/hello.wav")), root.join("Cd_Image/Sound/x/hello.wav"));
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}
