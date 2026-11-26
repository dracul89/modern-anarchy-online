//! Funcom's database patches applied to the real resource database. Needs three folders, and passes without doing
//! anything when one is not given:
//!
//! - `MAO_FRESH_GAME`: a game folder as the installer leaves it (18.8.50_EP1), never patched;
//! - `MAO_PATCHES`: the patch servers' files for the steps from 18.8.50_EP1 on;
//! - `MAO_GAME`: an installed game the original patcher brought to 18.8.62_EP1.
//!
//! The fresh database is copied (about 1.4 GB, to `MAO_SCRATCH` or the system's temporary folder), the twelve
//! steps' `.RES` files are applied to the copy, and every record is compared with the patched game's. The two
//! cannot be compared as files: the original patcher leaves the old copies of the records it moved in the data file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mao_formats::resource::{Id, ResourceDatabase};
use mao_install::{ctree, patcher, respatch};

fn folder(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

/// Every record's id with a hash of its data (FNV-1a, 64 bits) and its length; and the ids whose records cannot be
/// read (an installed game can have some: the client's `mao-db verify` lists them).
fn records(game: &Path) -> (BTreeMap<Id, (u64, usize)>, Vec<Id>) {
    let db = ResourceDatabase::open_game(game).unwrap();
    let (mut out, mut unreadable) = (BTreeMap::new(), Vec::new());
    for id in db.ids().unwrap() {
        let id = id.unwrap();
        let Ok(Some(data)) = db.get(id) else {
            unreadable.push(id);
            continue;
        };
        let hash = data.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
        out.insert(id, (hash, data.len()));
    }
    (out, unreadable)
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn every_step_into_the_fresh_database() {
    let (Some(fresh), Some(patches), Some(game)) = (folder("MAO_FRESH_GAME"), folder("MAO_PATCHES"), folder("MAO_GAME")) else {
        return;
    };
    // A copy of the fresh game's database, laid out as a game folder so both readers find it.
    let scratch = folder("MAO_SCRATCH").unwrap_or_else(std::env::temp_dir).join(format!("mao-install-db-{}", std::process::id()));
    let scratch = Scratch(scratch);
    let db_folder = scratch.0.join("cd_image/data/db");
    std::fs::create_dir_all(&db_folder).unwrap();
    let from = patcher::database_path(&fresh);
    for entry in std::fs::read_dir(from.parent().unwrap()).unwrap().flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().to_ascii_lowercase().starts_with("resourcedatabase.") {
            std::fs::copy(entry.path(), db_folder.join(&name)).unwrap();
        }
    }
    let (before, unreadable) = records(&scratch.0);
    assert!(unreadable.is_empty(), "the fresh database has records that cannot be read: {unreadable:?}");

    let base = patcher::database_path(&scratch.0);
    let mut expected_changes = BTreeMap::new();
    let mut totals = respatch::Report::default();
    for from in 50..62u32 {
        let name = format!("V18-8-{from}_EP1-V18-8-{}_EP1.RES", from + 1);
        let patch = std::fs::read(patches.join(&name)).unwrap();
        let parsed = respatch::ResPatch::parse(&patch).unwrap();
        for record in &parsed.records {
            expected_changes.insert(Id { kind: record.kind, instance: record.instance }, from);
        }
        if parsed.is_empty() {
            continue;
        }
        let mut db = ctree::Database::open_writable(&base).unwrap_or_else(|e| panic!("{name}: c-tree error {}", e.0));
        let mut last = (0, 0);
        let report = respatch::apply(&patch, &mut db, &mut |done, total| last = (done, total)).unwrap_or_else(|e| panic!("{name}: {e}"));
        db.close().unwrap();
        assert_eq!(last.0, last.1, "{name}");
        assert_eq!((report.added + report.replaced) as usize, parsed.records.len(), "{name}: {report:?}");
        totals.added += report.added;
        totals.replaced += report.replaced;
        // (the same step once more: nothing left to do)
        let mut db = ctree::Database::open_writable(&base).unwrap();
        let again = respatch::apply(&patch, &mut db, &mut |_, _| {}).unwrap();
        db.close().unwrap();
        assert_eq!(again, respatch::Report::default(), "{name}");
    }
    println!("records added {} replaced {}", totals.added, totals.replaced);

    // What this client's reader sees in the patched copy, against the game the original patcher patched.
    let (after, unreadable) = records(&scratch.0);
    assert!(unreadable.is_empty(), "the patched database has records that cannot be read: {unreadable:?}");
    let (theirs, their_unreadable) = records(&game);
    let mut differing = Vec::new();
    for (id, record) in &after {
        if theirs.get(id) != Some(record) && !their_unreadable.contains(id) {
            differing.push(*id);
        }
    }
    let missing: Vec<&Id> = theirs.keys().filter(|id| !after.contains_key(id)).collect();
    println!(
        "records: fresh {}, patched {}, the original's {} and {} it cannot read; differing {}, only in the original's {}",
        before.len(),
        after.len(),
        theirs.len(),
        their_unreadable.len(),
        differing.len(),
        missing.len()
    );
    // By type: how many differ, how many of those the patches brought, and how many of the patches' records agree.
    let mut by_type: BTreeMap<u32, (usize, usize, usize, usize)> = BTreeMap::new();
    for id in &differing {
        let row = by_type.entry(id.kind).or_default();
        row.0 += 1;
        row.1 += expected_changes.contains_key(id) as usize;
    }
    for id in expected_changes.keys() {
        let row = by_type.entry(id.kind).or_default();
        row.2 += 1;
        row.3 += (after.get(id) == theirs.get(id)) as usize;
    }
    for (kind, (differ, differ_patched, patched, patched_same)) in &by_type {
        println!("  type {kind:#x}: {differ} differ ({differ_patched} of them brought by the patches); the patches bring {patched}, {patched_same} of them the same in the original's");
    }
    for id in missing.iter().take(20) {
        println!("  only in the original's {:#x}:{}", id.kind, id.instance);
    }
    let unreadable_patched = their_unreadable.iter().filter(|id| expected_changes.contains_key(id)).count();
    println!("of the records the original's cannot read, {unreadable_patched} are ones the patches bring");
    // Every record the patches name is now as the original patcher left it, but for the types the compared game's
    // owner has changed since (`MAO_MODDED_TYPES`, hex numbers with commas between: a texture pack's types).
    let modded: Vec<u32> = std::env::var("MAO_MODDED_TYPES")
        .unwrap_or_default()
        .split(',')
        .filter_map(|t| u32::from_str_radix(t.trim().trim_start_matches("0x"), 16).ok())
        .collect();
    for id in expected_changes.keys().filter(|id| !their_unreadable.contains(id) && !modded.contains(&id.kind)) {
        assert_eq!(after.get(id), theirs.get(id), "{:#x}:{}", id.kind, id.instance);
    }
    differing.retain(|id| !modded.contains(&id.kind));
    // And the patches touched nothing else.
    for (id, record) in &before {
        if !expected_changes.contains_key(id) {
            assert_eq!(after.get(id), Some(record), "{:#x}:{}", id.kind, id.instance);
        }
    }
    assert_eq!(after.len(), before.len() + totals.added as usize);
    assert!(differing.is_empty() && missing.is_empty(), "the patched database is not the original patcher's");
}

/// A game folder that `mao-client install` (or `patch`) made, `MAO_PATCHED_GAME`, against the game the original
/// patcher patched, `MAO_GAME`: the same keys, and the same data in every record but those of `MAO_MODDED_TYPES`.
#[test]
fn a_patched_game_has_the_originals_records() {
    let (Some(ours), Some(game)) = (folder("MAO_PATCHED_GAME"), folder("MAO_GAME")) else { return };
    let modded: Vec<u32> = std::env::var("MAO_MODDED_TYPES")
        .unwrap_or_default()
        .split(',')
        .filter_map(|t| u32::from_str_radix(t.trim().trim_start_matches("0x"), 16).ok())
        .collect();
    let (ours, unreadable) = records(&ours);
    assert!(unreadable.is_empty(), "records that cannot be read: {unreadable:?}");
    let (theirs, their_unreadable) = records(&game);
    let mut keys: Vec<&Id> = theirs.keys().chain(&their_unreadable).collect();
    keys.sort();
    assert!(ours.keys().eq(keys.iter().copied()), "the two databases do not hold the same keys");
    let differing: Vec<&Id> = ours.iter().filter(|(id, record)| theirs.get(id).is_some_and(|other| other != *record)).map(|(id, _)| id).collect();
    let unexplained: Vec<&&Id> = differing.iter().filter(|id| !modded.contains(&id.kind)).collect();
    println!("records {}; differing {} ({} of types not set aside); the original's unreadable {}", ours.len(), differing.len(), unexplained.len(), their_unreadable.len());
    assert!(unexplained.is_empty(), "{:?}", &unexplained[..unexplained.len().min(10)]);
}
