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
    // (the most used second shield: red in and out over 20 s on the target's own mesh, pushed 0.01)
    assert!(matches!(gfx.visual(72274), Some(Visual::Shield2(s)) if s.material == 32 && s.duration == 20.0 && s.mesh == -1 && s.alpha == 2
        && s.colours == vec![(0.0, 0), (0.4, 0xffff_3030), (0.6, 0xffff_3030), (1.0, 0)] && s.pushes.len() == 2 && s.passes == 1));
    assert!(matches!(gfx.visual(96113), Some(Visual::Shield2(s)) if s.colours.len() == 7 && s.material == 77));
    assert!(matches!(gfx.visual(43010), Some(Visual::Coil { material: 32, duration, .. }) if duration == 4.0));
    assert!(matches!(gfx.visual(17500), Some(Visual::Plasma { material: 8, duration, .. }) if duration == 1.0));
    // the null gather effect has none
    assert!(gfx.visual(49999).is_none());
    // (five spiralling ribbons, their colour and radius curves read)
    assert!(matches!(gfx.visual(71108), Some(Visual::Spiral2(s)) if s.ribbons == 5 && s.segments == 25 && s.colours.len() == 4 && s.radii == vec![(0.0, 1.0), (0.9, 1.0), (1.0, 1.0)]));
    // (Gift of Life's sky flash: six cones 60 high, rising 0.4 s and falling 4.5)
    assert!(matches!(gfx.visual(43108), Some(Visual::SkyFlash(s)) if s.cones == 6 && s.height == 60.0 && s.rise == 0.4 && s.fall == 4.5 && s.cycles == 1));
    // (a burst of spark streaks: twenty, falling at 6.5, their heads white at half the second)
    assert!(matches!(gfx.visual(71342), Some(Visual::TParticle(p)) if p.kind == 0 && p.count == 20 && p.lift == -6.5 && p.drag == 0.2 && p.colours[1].1 == 0xffff_ffff));
    // (a nano dragged along the line: 50 a second, blue; its nano 300 sprites a second, half a second each)
    assert!(matches!(gfx.visual(2660), Some(Visual::Tracer3(t)) if t.speed == 50.0 && t.colour == [1.0, 0.1, 0.3, 1.0] && t.child.rate == 300.0 && t.child.life == (0.5, 0.5) && t.child.burst == 1));
    assert!(matches!(gfx.visual(2683), Some(Visual::Nano(n)) if n.material == 10 && n.scale == 0.5 && n.end[0] == 0.0));
    // (the commonest streak model: three streaks, turned a quarter about x, blue-violet)
    assert!(matches!(gfx.visual(45694), Some(Visual::Tracer1(t)) if t.streaks.len() == 3 && t.copies == 1 && t.size == 0.06 && t.material == 16 && t.colour == 0xff4c_99ff));
    // (the hoverbike's toggles: a puff as it moves off, a stream stopped with it)
    assert!(matches!(gfx.visual(72412), Some(Visual::Toggle { flags: 0x2c03, ref places, .. }) if places.is_empty()));
    assert!(matches!(gfx.visual(72416), Some(Visual::Toggle { flags, .. }) if flags & 0x8000 != 0));
    // (the commonest streak: white, 50 a second, 0.025 wide)
    assert!(matches!(gfx.visual(45689), Some(Visual::Tracer5(t)) if t.material == 15 && t.speed == 50.0 && t.lead == 0.5 && t.width == 0.025 && t.colour == 0xffff_ffff));
    // (Pulse of Frost's pops: ten of kind 8, 5 by 5, pale blue)
    assert!(matches!(gfx.visual(97121), Some(Visual::BParticle(b)) if b.kind == 8 && b.count == 10 && b.material == 100 && b.sizes[1] == (5.0, 5.0) && b.spin == 5.5 && b.phase_rate == 2.0));
    // (a hoverboard's exhaust trail: ten samples blown back, 0.2 to 0.22 wide)
    assert!(matches!(gfx.visual(72301), Some(Visual::Trail2(t)) if t.attach == 2006 && t.material == 45 && t.samples == 10 && t.rate == 0.5
        && t.thrust == 4.0 && t.colours.len() == 4 && t.widths.0 == vec![(0.0, 0.2), (0.5, 0.22), (1.0, 0.2)] && t.widths.1.len() == 3));
}

#[test]
fn the_effect_meshes_are_in_the_name_index() {
    use mao_formats::gfx::Visual;
    let (Some(db), Some(game)) = (database(), std::env::var_os("MAO_GAME").map(PathBuf::from)) else { return };
    let gfx = mao_formats::gfx::Gfx::parse(&std::fs::read(game.join("Setupf").join("gfxtweak.bin")).unwrap()).unwrap();
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    // (the EMP blast of Boom, the blast wave of Orbital Strike, the grid box of Summon Towers, a hoverboard's fan and circle)
    for (template, file) in [(71224, "EP03_EMP_blast.abiff"), (71028, "EP03_blast_wave_effect.abiff"), (72623, "xan_grid_dependencybox.abiff"), (72452, "hoverboard_b_fan.abiff"), (72346, "hoverbike_a_effect_circle.abiff")] {
        let Some(Visual::EffectMesh(m)) = gfx.visual(template) else { panic!("{template} is not an effect mesh") };
        assert_eq!(m.mesh_file(false), Some(file));
        assert!(index.find(0xf6951, file).is_some(), "{file}");
    }
    let Some(Visual::EffectMesh(m)) = gfx.visual(71224) else { unreachable!() };
    assert!(m.duration == 2.5 && m.scale == [2.0, 10.0, 20.0] && m.effect == 4 && m.opacity == vec![(0.0, 0.0), (0.1, 1.0), (1.0, 0.0)]);
}

#[test]
fn a_hoverboard_program_puts_each_body_on_its_board() {
    let Some(db) = database() else { return };
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    let program = |nano: u32| db.get(Id { kind: mao_formats::item::NANO_TEMPLATE, instance: nano }).unwrap().unwrap();
    // (Phasefront Phantom - Beige: a Solitus male's plain body (5907) rides template 270044, an Atrox's (5900) 270331)
    let phantom = program(270431);
    assert!(mao_formats::item::is_vehicle_program(&phantom));
    let body = |breed: u32, sex: u32, file: &str| mao_formats::item::Body { breed, sex, mesh: index.find(0xf6952, file).unwrap() };
    assert_eq!(body(1, 2, "solitus_male.cir").mesh, 5907);
    assert_eq!(mao_formats::item::vehicle_template(&phantom, &body(1, 2, "solitus_male.cir")), Some(270044));
    assert_eq!(mao_formats::item::vehicle_template(&phantom, &body(4, 1, "athrox_male.cir")), Some(270331));
    // (every breed and sex has a board or bike in each of the four programs: Spectre's are keyed on breed and sex)
    let bodies = [(1, 2, "solitus_male.cir"), (1, 3, "solitus_female.cir"), (2, 2, "opifex_male.cir"), (2, 3, "opifex_female.cir"), (3, 2, "nanomage_male.cir"), (3, 3, "nanomage_female.cir"), (4, 1, "athrox_male.cir")];
    for nano in [270431, 270327, 270836, 284003] {
        let data = program(nano);
        assert!(mao_formats::item::is_vehicle_program(&data), "{nano}");
        for (breed, sex, file) in bodies {
            assert!(mao_formats::item::vehicle_template(&data, &body(breed, sex, file)).is_some(), "{nano} {file}");
        }
    }
    assert_eq!(mao_formats::item::vehicle_template(&program(270836), &body(4, 1, "athrox_male.cir")), Some(270823));
    // the template: the rider and board as one CAT body, with the attractors the program's effects hang on, and its own animations
    let template = mao_formats::npc::Template::parse(&db.get(Id { kind: mao_formats::npc::TEMPLATE, instance: 270044 }).unwrap().unwrap()).unwrap();
    let mesh = template.mesh.unwrap();
    assert_eq!(index.find(0xf6952, "Hoverboard_b_male.cir"), Some(mesh));
    let body = mao_formats::catmesh::parse(&db.get(Id { kind: 0xf6952, instance: mesh }).unwrap().unwrap()).unwrap();
    for name in ["Attractor01_head", "Attractor07_special", "Attractor08_special", "Attractor09_special"] {
        assert!(body.attractor_named(name).is_some(), "{name}");
    }
    assert_eq!(template.animation(mao_formats::appearance::Action::Run), index.find(0xf6953, "Hoverboard_a_male_run_001.ani"));
    assert_eq!(template.animation(mao_formats::appearance::Action::Idle), index.find(0xf6953, "Hoverboard_a_male_idle_stand_001.ani"));
}

#[test]
fn a_bundle_runs_every_child() {
    use mao_formats::gfx::Visual;
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let gfx = mao_formats::gfx::Gfx::parse(&std::fs::read(game.join("Setupf").join("gfxtweak.bin")).unwrap()).unwrap();
    // (Phasefront Spectre's running effect, 72303: the hoverbike's smoke, trails, particles and its circle, 72346, the sixth child)
    let Some(Visual::Meta(children)) = gfx.visual(72303) else { panic!("72303 is not a bundle of several looks") };
    assert!(children.len() >= 2);
    assert!(children.iter().any(|c| matches!(c, Visual::EffectMesh(m) if m.attach == 2006 && m.mesh == 0x14)));
    // (a bundle with one child that has a look is that child: Phantom's fan bundle, 72307)
    assert!(matches!(gfx.visual(72307), Some(Visual::Meta(_)) | Some(Visual::EffectMesh(_))));
}
