//! Tests against Funcom's real installer package (Anarchy Online 18.8.50, Inno Setup 5.4.2).
//!
//! They read the folder that holds `install.exe` and its slices from the environment variable `MAO_INSTALLER` and
//! pass without doing anything when it is not set. Unpacking the whole package (3 GB, two to three minutes in a release
//! build) is done only when `MAO_INNO_OUT` names a folder to unpack into; with `MAO_GAME` set as well, the result
//! is compared with that installed copy of the game. Nothing of the package is kept in the repository.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use mao_install::inno::{self, Compression, Package, PlannedFile, Root};

fn package() -> Option<Package> {
    let dir = std::env::var_os("MAO_INSTALLER")?;
    Some(Package::open(dir).expect("the package opens"))
}

#[test]
fn package_reads() {
    let Some(p) = package() else { return };
    let h = p.header();
    assert_eq!(h.app_name, "Anarchy Online");
    // It has no licence page: the downloader shows the licence, and the game's `anarchy.eula` is among the files,
    // near the head of the packed stream.
    assert!(h.license_text.is_empty() && h.info_before.is_empty());
    let eula = p.files().iter().find(|f| f.destination.to_lowercase().ends_with("anarchy.eula")).expect("anarchy.eula");
    assert!(p.data_of(eula).unwrap().file_offset < 4 << 20);
    assert_eq!(h.compression, Some(Compression::Lzma1));
    assert_eq!(h.slices_per_disk, 1);
    assert_eq!(h.flags & (inno::header_flags::ENCRYPTION_USED | inno::header_flags::PASSWORD), 0);
    // The installer has no script, so nothing it does is hidden from the entry lists.
    assert!(h.compiled_code.is_empty());
    assert_eq!(p.offsets().data_offset, 0);
    assert_eq!(p.slice_names().len(), 5);
    assert_eq!(p.slice_names()[0], "install-1.bin");

    // Every file but the uninstaller's own entry is packed, goes below {app} and carries no condition.
    assert!(p.files().len() > 7000);
    let mut packed = 0;
    for file in p.files() {
        if file.is_uninstaller {
            assert!(file.location.is_none());
            continue;
        }
        packed += 1;
        let data = p.data_of(file).expect("a packed file");
        let target = file.target();
        assert_eq!(target.root, Root::App, "{}", file.destination);
        assert!(target.app_relative().is_some(), "{}", file.destination);
        assert!(file.conditions.is_unconditional(), "{}", file.destination);
        assert_eq!(p.default_install(&file.conditions), Some(true));
        assert_eq!(data.flags & inno::data_flags::CHUNK_ENCRYPTED, 0);
        assert!(data.modified().is_some());
    }
    assert_eq!(packed, p.data_entries().len());
    let plan = p.plan_app();
    assert_eq!(plan.len(), packed);
    assert_eq!(p.plan_size(&plan), p.data_entries().iter().map(|d| d.file_size).sum::<u64>());
    for wanted in ["version.id", "anarchy.cfg", "Anarchy.exe", "cd_image/data/db/ResourceDatabase.dat"] {
        assert!(plan.iter().any(|f| f.path.eq_ignore_ascii_case(wanted)), "{wanted} is in the plan");
    }

    // What the installer does besides copying files: two shortcuts (the desktop one only if its box is ticked,
    // which it is not by default) and the offer to start the game. No folders, ini or registry entries, nothing
    // deleted first.
    assert!(p.directories().is_empty());
    assert!(p.ini_entries().is_empty());
    assert!(p.registry_entries().is_empty());
    assert!(p.install_delete().is_empty());
    assert!(p.types().is_empty() && p.components().is_empty());
    assert_eq!(p.tasks().len(), 1);
    assert!(!p.task_is_default(&p.tasks()[0].name));
    assert_eq!(p.icons().len(), 2);
    assert_eq!(p.run_entries().len(), 1);
    assert_ne!(p.run_entries()[0].flags & inno::run_flags::POST_INSTALL, 0);
}

/// The files at the start of the one big chunk are cheap to reach: unpack those (checked against their SHA-1s on
/// the way) and look at what lands on disk.
#[test]
fn first_files_unpack() {
    let Some(p) = package() else { return };
    let plan: Vec<PlannedFile> = p
        .plan_app()
        .into_iter()
        .filter(|f| p.data_of(&p.files()[f.file]).is_some_and(|d| d.file_offset + d.file_size <= 6 << 20))
        .collect();
    assert!(plan.len() >= 4, "{} files in the first 6 MB", plan.len());
    // At least one of them is a program file stored with the call filter.
    assert!(plan.iter().any(|f| {
        p.data_of(&p.files()[f.file]).unwrap().flags & inno::data_flags::CALL_INSTRUCTION_OPTIMIZED != 0
    }));

    let out = std::env::temp_dir().join(format!("mao-install-inno-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let total = p.plan_size(&plan);
    let mut calls = 0;
    let mut last = 0;
    let done = p
        .extract(&out, &plan, &mut |progress| {
            calls += 1;
            assert!(progress.bytes_done >= last && progress.bytes_done <= progress.bytes_total);
            assert_eq!(progress.bytes_total, total);
            assert_eq!(progress.files_total, plan.len());
            assert!(plan.iter().any(|f| f.path == progress.current));
            last = progress.bytes_done;
            true
        })
        .expect("the files unpack");
    assert_eq!((done.files, done.bytes), (plan.len(), total));
    assert_eq!(last, total);
    assert!(calls >= plan.len());
    for planned in &plan {
        let data = p.data_of(&p.files()[planned.file]).unwrap();
        let meta = fs::metadata(out.join(&planned.path)).expect("the file is there");
        assert_eq!(meta.len(), data.file_size, "{}", planned.path);
        let wanted = data.modified().unwrap();
        let got = meta.modified().unwrap();
        let apart = got.duration_since(wanted).or_else(|_| wanted.duration_since(got)).unwrap();
        assert!(apart.as_secs() <= 2, "{} has its time from the package", planned.path);
    }
    assert!(tree(&out).keys().all(|name| !name.ends_with(".mao-part")));
    let _ = fs::remove_dir_all(&out);
}

/// Every file below `root`, by its `/`-separated path in lower case.
fn tree(root: &Path) -> BTreeMap<String, PathBuf> {
    fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            let path = entry.path();
            if path.is_dir() {
                walk(&path, &format!("{name}/"), out);
            } else {
                out.insert(name.to_lowercase(), path);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, "", &mut out);
    out
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    let (Ok(mut a), Ok(mut b)) = (File::open(a), File::open(b)) else { return false };
    if a.metadata().map(|m| m.len()).ok() != b.metadata().map(|m| m.len()).ok() {
        return false;
    }
    let mut x = vec![0u8; 1 << 20];
    let mut y = vec![0u8; 1 << 20];
    loop {
        let n = a.read(&mut x).unwrap();
        if n == 0 {
            return true;
        }
        if b.read_exact(&mut y[..n]).is_err() || x[..n] != y[..n] {
            return false;
        }
    }
}

/// The whole package: every file must come out with the SHA-1 its entry gives (the unpacker refuses otherwise).
#[test]
fn whole_package_unpacks() {
    let Some(p) = package() else { return };
    let Some(out) = std::env::var_os("MAO_INNO_OUT").map(PathBuf::from) else { return };
    let plan = p.plan_app();
    let total = p.plan_size(&plan);
    let started = Instant::now();
    let mut shown = Instant::now();
    let done = p
        .extract(&out, &plan, &mut |progress| {
            if shown.elapsed().as_secs() >= 10 {
                shown = Instant::now();
                println!(
                    "{:>5.1}%  {}/{} files  {}",
                    progress.bytes_done as f64 * 100.0 / progress.bytes_total as f64,
                    progress.files_done,
                    progress.files_total,
                    progress.current
                );
            }
            true
        })
        .expect("the package unpacks");
    println!("unpacked {} files, {} bytes in {:.1} s", done.files, done.bytes, started.elapsed().as_secs_f64());
    assert_eq!((done.files, done.bytes), (plan.len(), total));

    let unpacked = tree(&out);
    for planned in &plan {
        let data = p.data_of(&p.files()[planned.file]).unwrap();
        let path = unpacked.get(&planned.path.to_lowercase()).unwrap_or_else(|| panic!("{} is missing", planned.path));
        assert_eq!(fs::metadata(path).unwrap().len(), data.file_size, "{}", planned.path);
    }
    // The package's version file names the version the installer's header gives.
    let version = fs::read_to_string(out.join("version.id")).expect("version.id");
    assert_eq!(version.trim(), p.header().app_version);

    // Against an installed copy (patched since, so its programs and a few data files are newer).
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let installed = tree(&game);
    let (mut same, mut different, mut only_package) = (0, Vec::new(), Vec::new());
    for planned in &plan {
        let key = planned.path.to_lowercase();
        match installed.get(&key) {
            None => only_package.push(planned.path.clone()),
            Some(theirs) if same_bytes(&unpacked[&key], theirs) => same += 1,
            Some(_) => different.push(planned.path.clone()),
        }
    }
    let in_plan: BTreeSet<String> = plan.iter().map(|f| f.path.to_lowercase()).collect();
    let only_installed = installed.keys().filter(|k| !in_plan.contains(*k)).count();
    println!(
        "against the installed game: {same} the same, {} different, {} only in the package, {} only installed",
        different.len(),
        only_package.len(),
        only_installed
    );
    for path in &different {
        println!("  different: {path}");
    }
    for path in &only_package {
        println!("  only in the package: {path}");
    }
    // Most of the game's data was never patched.
    assert!(same > plan.len() * 9 / 10, "{same} of {} files match the installed game", plan.len());
}
