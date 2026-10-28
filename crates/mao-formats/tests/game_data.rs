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

#[test]
fn textures_decode() {
    use mao_formats::texture;
    let Some(db) = database() else { return };
    let object = db.get(Id { kind: 0xf6960, instance: 0x30e9c }).unwrap().unwrap();
    let t = texture::decode(&object).unwrap();
    assert_eq!((t.width, t.height, t.format), (128, 64, texture::Format::Jpeg));
    assert_eq!(t.rgba.len(), 128 * 64 * 4);
}

#[test]
fn a_tree_mesh() {
    use mao_formats::{mesh, model};
    let Some(db) = database() else { return };
    let object = db.get(Id { kind: 0xf6951, instance: 0x1893 }).unwrap().unwrap();
    let built = model::build(&mesh::parse(&object).unwrap()).unwrap();
    assert_eq!(built.parts.len(), 2);
    assert_eq!(built.parts.iter().map(|p| p.indices.len() / 3).sum::<usize>(), 812);
    for part in &built.parts {
        assert!(part.indices.iter().all(|&i| (i as usize) < part.vertices.len()));
    }
}

#[test]
fn every_mesh_parses() {
    use mao_formats::mesh;
    let Some(db) = database() else { return };
    let mut count = 0;
    for id in db.ids().unwrap() {
        let id = id.unwrap();
        if id.kind != 0xf6951 && id.kind != 0xf696a {
            continue;
        }
        let Ok(Some(object)) = db.get(id) else { continue };
        mesh::parse(&object).unwrap_or_else(|e| panic!("{id:?}: {e}"));
        count += 1;
    }
    assert!(count > 11_000);
}

#[test]
fn a_playfield_terrain() {
    use mao_formats::{terrain, tilemap};
    let Some(db) = database() else { return };
    let object = db.get(Id { kind: 0xf4249, instance: 0x254 }).unwrap().unwrap();
    let map = tilemap::parse_old(&object).unwrap();
    let t = terrain::build(&map).unwrap();
    assert_eq!((t.width, t.height), (97, 97));
    assert!(t.heights.iter().max() > t.heights.iter().min());
    assert_eq!(t.tiles.len(), 96 * 96);
}

#[test]
fn playfield_statels() {
    use mao_formats::statels;
    let Some(db) = database() else { return };
    let game = PathBuf::from(std::env::var_os("MAO_GAME").unwrap());
    let file = std::fs::read(game.join("cd_image/data/statels/100.pf")).unwrap();
    let list = statels::parse(&file).unwrap();
    assert_eq!(list.len(), 474);
    // Every statel names a mesh that is in the database: the layout is right.
    for s in &list {
        assert!(db.get(Id { kind: 0xf6951, instance: s.mesh }).unwrap().is_some(), "mesh {:#x}", s.mesh);
    }
}

#[test]
fn the_effect_templates_are_read_to_the_last_word() {
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let data = std::fs::read(game.join("Setupf").join("gfxtweak.bin")).unwrap();
    let gfx = mao_formats::gfx::Gfx::parse(&data).unwrap();
    assert_eq!(gfx.records.len(), 2687);
    // a nano's cast, hit, tracer and gather effects, and the buff of a blue aura
    assert_eq!(gfx.get(46133).unwrap().kind, 1010);
    assert_eq!(gfx.get(43010).unwrap().kind, 2001);
    assert_eq!(gfx.get(17300).unwrap().kind, 2004);
    assert_eq!(gfx.get(49999).unwrap().kind, 2007);
    assert_eq!(gfx.get(1000).unwrap().kind, 1001);
    // the cast effect's tree reaches its six parts
    assert!(gfx.tree(46133).len() >= 7);
}

#[test]
fn a_cast_effect_has_its_emitters() {
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let data = std::fs::read(game.join("Setupf").join("gfxtweak.bin")).unwrap();
    let gfx = mao_formats::gfx::Gfx::parse(&data).unwrap();
    // the cast of the first nano that names one: its six parts and what they refer to
    let emitters = gfx.emitters(46133);
    assert!(emitters.len() >= 6);
    assert!(emitters.iter().all(|e| e.life > 0.0 && e.rate > 0.0 && e.start[0] >= 0.0));
    // as `_GfxControlSpell1_t` reads it: a flare at each hand, 16 a second, phase 1 at 0–6 s, no projectile
    let cast = gfx.cast(46133).unwrap();
    assert_eq!(cast.times[..5], [0.0, 6.0, 8.0, 4.0, 5.0]);
    assert!(!cast.released);
    let flares: Vec<_> = cast.flares.iter().flatten().collect();
    assert_eq!(flares.iter().map(|f| f.attach).collect::<Vec<_>>(), vec![2001, 2000]);
    assert!(flares.iter().all(|f| f.rate == 16.0 && f.material == 15));
}

#[test]
fn effect_classes_come_to_visuals() {
    use mao_formats::gfx::Visual;
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let gfx = mao_formats::gfx::Gfx::parse(&std::fs::read(game.join("Setupf").join("gfxtweak.bin")).unwrap()).unwrap();
    // a buff, a gold tracer (Stars mode 18: its attach point 1000 is not the buff spiral), the common cyan ground glow, a hit
    // flash, a tracer ball
    assert!(matches!(gfx.visual(1000), Some(Visual::Spiral(_))));
    assert!(matches!(gfx.visual(17300), Some(Visual::Stars(s)) if s.mode == 18));
    assert!(matches!(gfx.visual(43608), Some(Visual::Shield(s)) if s.material == 32 && s.duration == 3.0 && s.flags & 0x400 != 0));
    assert!(matches!(gfx.visual(43010), Some(Visual::Flash { .. })));
    assert!(matches!(gfx.visual(17500), Some(Visual::Plasma { material: 8, duration, .. }) if duration == 1.0));
    // the null gather effect has none
    assert!(gfx.visual(49999).is_none());
}
