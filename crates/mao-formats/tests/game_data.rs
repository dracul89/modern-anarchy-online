//! Against an installed game: set `MAO_GAME` to its folder (the tests do
//! nothing without it). The sizes are what the original client's
//! `DatabaseController.dll` returns for the same keys.

use std::path::PathBuf;

use mao_formats::resource::{Id, ResourceDatabase};

fn database() -> Option<ResourceDatabase> {
    let game = PathBuf::from(std::env::var_os("MAO_GAME")?);
    Some(ResourceDatabase::open_game(&game).expect("the game's resource database"))
}

#[test]
fn known_objects() {
    let Some(db) = database() else { return };
    assert_eq!(db.len(), 460_193);
    let mesh = db.get(Id { kind: 0xf6951, instance: 0x30e9e }).unwrap().unwrap();
    assert_eq!(mesh.len(), 1_205_865);
    assert_eq!(mesh[..8], [0x51, 0x69, 0x0f, 0x00, 0x9e, 0x0e, 0x03, 0x00]);
    let info = db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap();
    assert_eq!(info.len(), 1_741_320);
    // A texture the original client can't read either (it falls back to another type).
    assert!(db.get(Id { kind: 0xf6954, instance: 0x1727 }).unwrap().is_none());
}

#[test]
fn every_index_entry_is_walked_in_order() {
    let Some(db) = database() else { return };
    let mut count = 0u32;
    let mut last = None;
    for id in db.ids().unwrap() {
        let id = id.unwrap();
        assert!(last < Some(id), "keys ascend");
        last = Some(id);
        count += 1;
    }
    assert_eq!(count, db.len());
}
