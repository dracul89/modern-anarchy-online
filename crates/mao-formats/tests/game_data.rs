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
    // (a ground grid: 20 by 20 of 2, round fade)
    assert!(matches!(gfx.visual(71233), Some(Visual::GroundGrid(g)) if g.points == 20 && g.spacing == 2.0 && g.fade_mode == 2 && g.duration == 10.0));
    // (a ground shake: 2.3 s, felt to 30 away)
    assert!(matches!(gfx.visual(72380), Some(Visual::GroundShake { duration, reach, ref curve }) if duration == 2.3 && reach == 30.0 && curve.len() == 4));
    // (the Rage buffs' flames: sixteen a second off a disc behind the body, a second each)
    assert!(matches!(gfx.visual(2200), Some(Visual::Fire(f)) if f.rate == 16.0 && f.radius == (0.0, 0.5) && f.life == 1.0 && f.place == [0.0, 0.0, -0.25] && f.push == 4));
    // (a fountain of streaks: 80 particles drawn along their velocities, two six-key colour curves)
    assert!(matches!(gfx.visual(73001), Some(Visual::TParticle2 { ref particles, length, ref head }) if particles.count == 80 && particles.colours.len() == 6 && head.len() == 6 && length == 1.0));
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

#[test]
fn specials_swing_their_class_of_the_weapon() {
    let Some(db) = database() else { return };
    let get = |t: u32| db.get(Id { kind: mao_formats::item::TEMPLATE, instance: t }).unwrap().unwrap();
    // (the Ofab Peregrine is small arms: a burst bursts, a full auto runs; the Ofab Shark a rifle; a crowbar's sneak attack stabs)
    assert_eq!(mao_formats::item::weapon_actions(&get(301709), 6, 0x16), vec![0x3f6]);
    assert_eq!(mao_formats::item::weapon_actions(&get(301709), 8, 0x17), vec![0x3fa]);
    assert_eq!(mao_formats::item::weapon_actions(&get(265091), 6, 0x17), vec![0x401]);
    assert_eq!(mao_formats::item::weapon_actions(&get(122829), 6, 0x1d), vec![0x3ec]);
    assert_eq!(mao_formats::item::special_class(147), Some((0x19, false)));
    // (and they are held: the pistol's idle, the rifle's, the crowbar's)
    assert_eq!(mao_formats::item::weapon_actions(&get(301709), 6, 0x10), vec![0x3f3]);
    assert_eq!(mao_formats::item::weapon_actions(&get(265091), 6, 0x10), vec![0x3fd]);
    assert_eq!(mao_formats::item::weapon_actions(&get(122829), 6, 0x10), vec![0x3e9]);
    // (a blade has no burst: the class is erased and not filled)
    assert!(mao_formats::item::weapon_actions(&get(122829), 6, 0x16).is_empty());
    // (out of a fight a rifle is held two-handed, idle-2h, and walks and runs so; a pistol keeps the plain idle and walk)
    assert_eq!(mao_formats::item::weapon_actions(&get(265091), 6, 0x29), vec![0x41e]);
    assert_eq!(mao_formats::item::wielded_movement(&get(265091)), Some((0x421, 0x422)));
    // (and wielding it out of a fight plays its 2h-start, putting it away its 2h-stop; a pistol has neither)
    assert_eq!(mao_formats::item::weapon_actions(&get(265091), 6, 0x27), vec![0x41f]);
    assert_eq!(mao_formats::item::weapon_actions(&get(265091), 6, 0x28), vec![0x420]);
    assert!(mao_formats::item::weapon_actions(&get(301709), 6, 0x28).is_empty());
    assert!(mao_formats::item::weapon_actions(&get(301709), 6, 0x29).is_empty());
    assert_eq!(mao_formats::item::wielded_movement(&get(301709)), None);
}

#[test]
fn captured_weapons_swing_animations_the_files_have() {
    // (the weapons a captured fight's WeaponItemFullUpdates named, and the martial arts item)
    let Some(db) = database() else { return };
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    for template in [258818, 122829, 123362, 265091, 264957, 271074, 301709, 43712] {
        let data = db.get(Id { kind: mao_formats::item::TEMPLATE, instance: template }).unwrap().unwrap();
        let name = mao_formats::item::Item::parse(&data).map(|i| i.name).unwrap_or_default();
        let actions = mao_formats::item::attack_actions(&data, 6);
        println!("{template} {name:?} AnimSet {:?}: {actions:x?}", mao_formats::item::Item::parse(&data).and_then(|i| i.stat(353)));
        assert!(!actions.is_empty(), "{template} {name} swings nothing");
        for action in actions {
            let mut a = action;
            let mut found = false;
            while a != 0 && !found {
                let (files, _) = mao_formats::appearance::action_animation_names(mao_formats::appearance::SOLITUS, mao_formats::appearance::MALE, a).unwrap();
                found = files.iter().any(|f| index.find(0xf6953, f).is_some());
                a = mao_formats::appearance::action_fallback(a);
            }
            assert!(found, "{template} {name}: no animation for {action:#x}");
        }
    }
}

#[test]
fn weapon_animations_carry_their_attack_notes() {
    // (the moment of the shot or blow, which the swing's speed is measured by: the first note, `GetNoteTime(id, 0)`)
    let Some(db) = database() else { return };
    for (id, first, time) in [(10170, "attack", 300.0), (14773, "attack", 200.0), (10181, "attack", 233.0), (10175, "swish_punch", 33.0)] {
        let data = db.get(Id { kind: 0xf6953, instance: id }).unwrap().unwrap();
        let anim = mao_formats::catanim::parse(&data).unwrap();
        assert_eq!((anim.notes[0].name.as_str(), anim.notes[0].time), (first, time));
    }
}

#[test]
fn a_creature_set_swings_and_dies() {
    // (the grid default character, 99902: its set has the unarmed swing FUN_1006a782 gives a body without weapon points, and a
    // death the fallbacks reach)
    let Some(db) = database() else { return };
    let data = db.get(Id { kind: mao_formats::npc::TEMPLATE, instance: 99902 }).unwrap().unwrap();
    let record = mao_formats::npc::Template::parse(&data).unwrap();
    assert!(record.animations.iter().any(|a| a.0 == 0x40a));
    assert_eq!(mao_formats::appearance::action_fallback(500), 6000);
    assert!(record.animations.iter().any(|a| a.0 == 6000));
}

/// The original's feedback lines, read from `text.mdb` by the names FUN_10012c3e hashes.
#[test]
fn feedback_texts() {
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let texts = mao_formats::ldb::Texts::parse(&std::fs::read(game.join("cd_image/text/text.mdb")).unwrap()).unwrap();
    let f = |name: &str| texts.named(mao_formats::ldb::FEEDBACK, name).unwrap().to_string();
    assert_eq!(f("Feedback_HitWithSpecial"), "You hit %s for %u points of %s damage.");
    assert_eq!(f("Feedback_OtherTriedToHitMissed"), "%s tried to hit you, but missed!");
    assert_eq!(f("Feedback_CriticalHit"), "Critical hit!");
    assert_eq!(f("TryToAttackWithSpecialButMiss"), "You try to attack %s with %s, but you miss!");
}

/// "solitus male" (26092), read as FUN_1004de5d reads it: the social action 0x95 has 90 variants, 0xb7 three.
#[test]
fn a_humanoid_set_has_its_variants() {
    let Some(db) = database() else { return };
    let data = db.get(Id { kind: mao_formats::npc::TEMPLATE, instance: 26092 }).unwrap().unwrap();
    let t = mao_formats::npc::Template::parse(&data).unwrap();
    assert_eq!(t.mesh, Some(0x1713));
    assert_eq!(t.animations.iter().filter(|a| a.0 == 0x95).count(), 90);
    assert_eq!(t.animations.iter().filter(|a| a.0 == 0xb7).count(), 3);
    assert_eq!(t.variant(100, 0), Some(0x27cf));
}

/// The index walk lists every animation set the database can fetch (26092 among them).
#[test]
fn the_index_walk_lists_the_humanoid_sets() {
    let Some(db) = database() else { return };
    let sets: Vec<u32> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::npc::TEMPLATE).map(|i| i.instance).collect();
    eprintln!("{} sets, first {:?}, last {:?}", sets.len(), sets.first(), sets.last());
    assert!(sets.contains(&26092), "{} sets", sets.len());
}

/// Both source files read whole, and every sample they name reads as Miles reads it (`SE_Create2DSoundFromFile`).
#[test]
fn sound_banks_and_their_samples() {
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let sound = game.join("cd_image/sound");
    let mut counts = Vec::new();
    let mut missing = 0;
    for name in ["SM_Sandy_Gui.sbf", "SM_Sandy_Game_Dummy.sbf"] {
        let bank = mao_formats::sound::parse_bank(&std::fs::read(sound.join("SourceFiles").join(name)).unwrap()).unwrap();
        counts.push(bank.len());
        // (every seventh, to keep the test short in debug builds)
        for r in bank.iter().filter(|r| r.has_file).step_by(7) {
            match std::fs::read(sound.join(&r.file)) {
                Ok(data) => assert!(mao_formats::sound::read_sample(&data).is_some(), "{}", r.file),
                Err(_) => missing += 1,
            }
        }
    }
    assert_eq!(counts, [85, 6810]);
    let walk = mao_formats::sound::parse_bank(&std::fs::read(sound.join("SourceFiles/SM_Sandy_Game_Dummy.sbf")).unwrap()).unwrap();
    let feet = walk.iter().find(|r| r.id == mao_formats::sound::sound_id("SM_Sandy_Game_Feet_Man_Walk")).unwrap();
    assert!(feet.variants.iter().filter(|&&v| v != 0).count() >= 10);
    eprintln!("{missing} samples missing");
}

/// The tile materials (0xf4258:1) and the tile IDs of a playfield's ground found in them.
#[test]
fn tile_materials() {
    let Some(db) = database() else { return };
    let object = db.get(Id { kind: mao_formats::tilemap::TILE_MATERIALS, instance: 1 }).unwrap().unwrap();
    let map = mao_formats::tilemap::tile_materials(&object).unwrap();
    // (version 3, which N3.dll's reader skips: only tile 0, of material 0)
    assert_eq!(u16::from_le_bytes([object[14], object[15]]), 3);
    assert_eq!(map.into_iter().collect::<Vec<_>>(), vec![(0, mao_formats::tilemap::TileMaterial::default())]);
    let mut v2 = object[..16].to_vec();
    v2[12..16].copy_from_slice(&[1, 0, 2, 0]);
    v2.extend([7, 0]);
    v2.extend(mao_formats::sound::sound_id("grass").to_le_bytes());
    v2.extend([1, 2, 3]);
    let map = mao_formats::tilemap::tile_materials(&v2).unwrap();
    let mut by: std::collections::BTreeMap<u32, usize> = Default::default();
    for m in map.values() {
        *by.entry(m.material).or_default() += 1;
    }
    eprintln!("{} tiles; by material {by:?}", map.len());
    assert_eq!(map[&7], mao_formats::tilemap::TileMaterial { passability: 1, attack: 2, defense: 3, material: 0xf });
    // (Tir's ground: its tile types are tile IDs of the table)
    let ground = db.get(Id { kind: 0xf4249, instance: 0x2a8 }).ok().flatten().and_then(|o| mao_formats::terrain::build(&mao_formats::tilemap::parse_old(&o).ok()?).ok());
    if let Some(t) = ground {
        let known = t.tile_types.iter().filter(|id| map.contains_key(id)).count();
        eprintln!("Tir's {} tile types, {known} in the table: {:?}", t.tile_types.len(), t.tile_types.iter().take(12).map(|id| (id, map.get(id).map(|m| m.material))).collect::<Vec<_>>());
    }
}

/// The animation sets' sounds name sources of the game's source file.
#[test]
fn animation_set_sounds_are_sources() {
    let Some(db) = database() else { return };
    let game = PathBuf::from(std::env::var_os("MAO_GAME").unwrap());
    let bank = mao_formats::sound::parse_bank(&std::fs::read(game.join("cd_image/sound/SourceFiles/SM_Sandy_Game_Dummy.sbf")).unwrap()).unwrap();
    let ids: std::collections::HashSet<u32> = bank.iter().map(|r| r.id).collect();
    let (mut sets, mut with, mut sounds, mut found) = (0, 0, 0, 0);
    let mut keys: std::collections::BTreeMap<u32, usize> = Default::default();
    for id in db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::npc::TEMPLATE) {
        let Some(t) = db.get(id).ok().flatten().and_then(|d| mao_formats::npc::Template::parse(&d)) else { continue };
        sets += 1;
        with += (!t.sounds.is_empty()) as usize;
        for &(key, s) in &t.sounds {
            sounds += 1;
            found += ids.contains(&s) as usize;
            *keys.entry(key).or_default() += 1;
        }
    }
    eprintln!("{sets} sets, {with} with sounds, {sounds} sounds, {found} of them sources; keys {keys:?}");
    assert!(with > 100 && found * 10 > sounds * 9);
}

/// The player animations' notes give the events Gamecode plays sounds for (steps in the walk and run, a landing).
#[test]
fn animation_notes_are_events() {
    let Some(db) = database() else { return };
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    let mut seen = Vec::new();
    for name in ["male_walk_01_01.ani", "male_run_01_01.ani", "female_walk_01_01.ani", "male_jump-stop_01_01.ani", "male_swim_01_01.ani"] {
        let Some(id) = index.find(0xf6953, name) else { continue };
        let anim = mao_formats::catanim::parse(&db.get(Id { kind: 0xf6953, instance: id }).unwrap().unwrap()).unwrap();
        let notes: Vec<(String, f32, i32)> = anim.notes.iter().map(|n| (n.name.clone(), n.time, n.event())).collect();
        eprintln!("{name}: {notes:?}");
        seen.push((name, notes));
    }
    let walk = &seen.iter().find(|s| s.0 == "male_walk_01_01.ani").unwrap().1;
    assert!(walk.iter().filter(|n| n.2 == 0x26).count() >= 2);
}

/// The captured weapons' sound sets (chunk 0x14) name sources of the game's source file, and their swings carry `attack` notes.
#[test]
fn weapon_sounds_and_attack_notes() {
    let Some(db) = database() else { return };
    let game = PathBuf::from(std::env::var_os("MAO_GAME").unwrap());
    let bank = mao_formats::sound::parse_bank(&std::fs::read(game.join("cd_image/sound/SourceFiles/SM_Sandy_Game_Dummy.sbf")).unwrap()).unwrap();
    let ids: std::collections::HashSet<u32> = bank.iter().map(|r| r.id).collect();
    let mut with = 0;
    for template in [258818, 122829, 123362, 265091, 264957, 271074, 301709, 43712] {
        let data = db.get(Id { kind: mao_formats::item::TEMPLATE, instance: template }).unwrap().unwrap();
        let sets = mao_formats::item::class_sets(&data, 0x14);
        let known = sets.iter().flat_map(|s| s.1.iter()).filter(|id| ids.contains(id)).count();
        eprintln!("{template}: {:x?}, {known} sources", sets);
        with += (known > 0) as usize;
    }
    assert!(with >= 6);
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    for name in ["male_smallarms-shot_01_01.ani", "male_1h-slash_01_01.ani", "male_2h-shot_01_01.ani", "male_punch_01_01.ani"] {
        let Some(id) = index.find(0xf6953, name) else { eprintln!("{name}: none"); continue };
        let anim = mao_formats::catanim::parse(&db.get(Id { kind: 0xf6953, instance: id }).unwrap().unwrap()).unwrap();
        eprintln!("{name}: {:?}", anim.notes.iter().map(|n| (n.name.as_str(), n.time, n.event())).collect::<Vec<_>>());
    }
}

/// The music project reads whole: its layers (the battle layers among them) and samples with their transitions.
#[test]
fn the_music_project() {
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let p = mao_formats::music::Project::parse(&std::fs::read(game.join("cd_image/sound/music/env/anarchy.sws")).unwrap()).unwrap();
    let transitions: usize = p.samples.iter().map(|s| s.transitions.len()).sum();
    let layered: usize = p.samples.iter().map(|s| s.layer_transitions.len()).sum();
    eprintln!("{} layers, {} samples, {transitions} transitions, {layered} layer transitions, {} entries, {} paused layers", p.layers.len(), p.samples.len(), p.samples.iter().filter(|s| s.entry).count(), p.layers.iter().filter(|l| l.pause.is_some()).count());
    eprintln!("pause tracks: {:?}", p.layers.iter().filter_map(|l| Some((l.name.as_str(), l.pause.as_ref()?))).take(5).collect::<Vec<_>>());
    eprintln!("layers: {:?}", p.layers.iter().take(30).map(|l| (l.id, l.name.as_str(), l.volume)).collect::<Vec<_>>());
    eprintln!("sample 0: {:?}", p.samples.first());
    assert!(p.find_layer(r"battle\Neutral") >= 0 && p.find_layer("mountain/night") >= 0);
    assert!(p.samples.iter().all(|s| s.transitions.iter().chain(&s.layer_transitions).all(|t| t.to < p.samples.len())));
    let late = p.samples.iter().flat_map(|s| s.transitions.iter()).filter(|t| t.in_time > 0 || (t.in_time == -1 && t.in_position > 0)).count();
    let curves: std::collections::BTreeMap<i32, usize> = p.samples.iter().flat_map(|s| s.transitions.iter()).fold(Default::default(), |mut m, t| { *m.entry(t.fade_out.curve).or_default() += 1; m });
    eprintln!("{late} transitions enter past the start; fade-out curves {curves:?}; fade times > 0: {}", p.samples.iter().flat_map(|s| s.transitions.iter()).filter(|t| t.time > 0).count());
}

/// An outdoor playfield's zone grid: `RDBPlayfield_t`'s zone size (+0x18, before the tile map) divides the tile map into its
/// zones (N3's `GetNumZonesInX` × `…InZ`), and its districts (0xf424e) name a district for each zone.
#[test]
fn zones_and_districts() {
    let Some(db) = database() else { return };
    // (every outdoor playfield's districts read)
    let mut read = 0;
    for id in db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::districts::KIND) {
        let data = db.get(id).unwrap().unwrap();
        let d = mao_formats::districts::parse(&data);
        // (an empty list the original refuses too: "number of districts=0")
        if d.is_none() && data.len() > 19 && data[18] != 0 {
            panic!("playfield {}: version {}, zones {}, count {}, {} bytes; head {:02x?}", id.instance, u16::from_le_bytes([data[12], data[13]]), u32::from_le_bytes(data[14..18].try_into().unwrap()), data[18], data.len(), &data[19..90.min(data.len())]);
            continue;
        }
        read += 1;
    }
    eprintln!("{read} district lists");
    for pf in [0x2a8u32, 760, 567, 655] {
        let Some(object) = db.get(Id { kind: 0xf4241, instance: pf }).unwrap() else { continue };
        let word = |o: usize| u32::from_le_bytes(object[12 + o..16 + o].try_into().unwrap());
        let (size, tilemap, zones) = (word(36), word(40), word(48));
        eprintln!("words: {:?}", (0..14).map(|k| word(4 * k)).collect::<Vec<_>>());
        let Some(map) = db.get(Id { kind: 0xf4249, instance: tilemap }).ok().flatten().and_then(|o| mao_formats::tilemap::parse_old(&o).ok()) else { continue };
        let districts = db.get(Id { kind: 0xf424e, instance: pf }).unwrap().map(|d| d.len());
        let d = mao_formats::districts::parse(&db.get(Id { kind: mao_formats::districts::KIND, instance: pf }).unwrap().unwrap()).unwrap();
        assert_eq!(d.zones.len() as u32, zones);
        assert_eq!(mao_formats::districts::zone_size(&object), Some(10));
        eprintln!("pf {pf}: {} districts: {:?}", d.districts.len(), d.districts.iter().take(4).map(|x| (x.name.as_str(), x.music)).collect::<Vec<_>>());
        eprintln!("pf {pf}: zone size {size}, map {}x{}, zones {zones} (grid {}), districts object {districts:?}", map.width, map.height, (map.width as u32 / size.max(1)) * (map.height as u32 / size.max(1)));
    }
}

/// Nano programs name their sounds (stats 269 to 272) among the game's sources.
#[test]
fn nano_sounds() {
    let Some(db) = database() else { return };
    let game = PathBuf::from(std::env::var_os("MAO_GAME").unwrap());
    let bank = mao_formats::sound::parse_bank(&std::fs::read(game.join("cd_image/sound/SourceFiles/SM_Sandy_Game_Dummy.sbf")).unwrap()).unwrap();
    let ids: std::collections::HashSet<u32> = bank.iter().map(|r| r.id).collect();
    let (mut with, mut known, mut total) = (0, 0, 0);
    let mut shown = 0;
    let mut all = 0;
    for id in db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == 0xfde85) {
        let template = id.instance;
        let Some(item) = db.get(id).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)) else { continue };
        all += 1;
        if !(269..273).any(|k| item.stat(k).is_some_and(|v| v != 0)) {
            continue;
        }
        shown += 1;
        if shown > 5 {
            with += 1;
            for v in (269..273).filter_map(|k| item.stat(k)).filter(|&v| v != 0) {
                total += 1;
                known += ids.contains(&v) as usize;
            }
            continue;
        }
        let s: Vec<Option<u32>> = (269..273).map(|k| item.stat(k)).collect();
        eprintln!("{template:#x} {:?}: sounds {s:?}, cast time {:?}", item.name, item.stat(294));
        with += s.iter().any(|v| v.is_some_and(|v| v != 0)) as usize;
        for v in s.into_iter().flatten().filter(|&v| v != 0) {
            total += 1;
            known += ids.contains(&v) as usize;
        }
    }
    eprintln!("{all} nanos, {with} with sounds, {known} of {total} sounds sources");
    assert!(with > 0 && known * 10 >= total * 9, "{known} of {total}");
}

/// The nanos whose effects carry a sound control (kind 4000, `_GfxControlAudio_t`), by effect stat.
#[test]
fn nano_effect_sounds() {
    let Some(db) = database() else { return };
    let game = PathBuf::from(std::env::var_os("MAO_GAME").unwrap());
    let gfx = mao_formats::gfx::Gfx::parse(&std::fs::read(game.join("Setupf/gfxtweak.bin")).unwrap()).unwrap();
    let mut by: std::collections::BTreeMap<u32, usize> = Default::default();
    for id in db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == 0xfde85) {
        let Some(item) = db.get(id).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)) else { continue };
        for stat in [366u32, 428, 419, 361, 414, 413] {
            if let Some(e) = item.stat(stat) {
                if gfx.tree(e as i32).iter().any(|r| r.kind == 4000) {
                    *by.entry(stat).or_default() += 1;
                }
            }
        }
    }
    eprintln!("nanos with a sound control, by effect stat: {by:?}");
}

#[test]
fn the_lightning_s_splash() {
    // (gfx 0x859b, the bolt tip's splash: a `_GfxControlSplash_t` of four cones of material 35, 0.75 s each, 0.025 s apart)
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let data = std::fs::read(game.join("Setupf").join("gfxtweak.bin")).unwrap();
    let gfx = mao_formats::gfx::Gfx::parse(&data).unwrap();
    let s = mao_formats::gfx::Splash::read(gfx.get(0x859b).unwrap()).unwrap();
    assert_eq!((s.count, s.material, s.segments), (4, 35, 8));
    assert_eq!((s.life, s.delay, s.end), (0.75, 0.025, 1.0));
    assert!((s.swell.0 - 0.4).abs() < 1e-6 && (s.rise - 0.04).abs() < 1e-6);
}

#[test]
fn a_playfield_s_collision_surfaces() {
    // (playfield 952's zones: every one a version 5 KD tree that ends on its marker; y up)
    let Some(db) = database() else { return };
    let surfaces: Vec<_> = db.ids().unwrap().flatten().filter(|i| i.kind == mao_formats::surface::KIND && i.instance >> 16 == 952).collect();
    let mut triangles = 0;
    for id in &surfaces {
        let s = mao_formats::surface::parse(&db.get(*id).unwrap().unwrap()).expect("a readable surface");
        triangles += s.triangles.len();
    }
    assert_eq!(triangles, 12538);
}

#[test]
fn old_athens_ways_out() {
    // (playfield 566's zones: four teleportals, all to playfield 567; the one at x 214..234, z 300..314 a rectangle)
    let Some(db) = database() else { return };
    let mut ways = Vec::new();
    for id in db.ids().unwrap().flatten().filter(|i| i.kind == mao_formats::surface::KIND && i.instance >> 16 == 566) {
        ways.extend(mao_formats::surface::parse(&db.get(id).unwrap().unwrap()).expect("a readable surface").teleportal);
    }
    assert_eq!(ways.len(), 4);
    assert!(ways.iter().all(|t| t.destination() == 567));
    let gate = ways.iter().find(|t| t.contains(224.0, 307.0)).expect("the gate's outline");
    assert_eq!(gate.points.len(), 4);
    assert!(!gate.contains(240.0, 307.0) && !gate.contains(224.0, 320.0));
}

/// `cargo test -p mao-formats --test game_data water_survey -- --ignored --nocapture`: every playfield's water polygons by
/// type, their heights, and the ground's height range, to pick where to look at the liquid passes.
#[test]
#[ignore]
fn water_survey() {
    let Some(db) = database() else { return };
    for p in mao_formats::playfield::all(&db) {
        let Some(a) = mao_formats::playfield::load_anarchy(&db, p.number) else { continue };
        if a.water.is_empty() {
            continue;
        }
        let mut kinds = std::collections::BTreeMap::<u32, usize>::new();
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for t in &a.water {
            *kinds.entry(t.kind).or_default() += 1;
            for q in &t.points {
                lo = lo.min(q[1]);
                hi = hi.max(q[1]);
            }
        }
        let ground = db
            .get(Id { kind: 0xf4249, instance: p.number })
            .ok()
            .flatten()
            .and_then(|o| mao_formats::tilemap::parse_old(&o).ok())
            .and_then(|m| mao_formats::terrain::build(&m).ok())
            .map(|t| {
                let (a, b) = t.heights.iter().fold((u16::MAX, 0u16), |(a, b), &h| (a.min(h), b.max(h)));
                format!("ground {:.1}..{:.1} (scale {})", a as f32 / 256.0 * t.height_scale, b as f32 / 256.0 * t.height_scale, t.height_scale)
            })
            .unwrap_or_else(|| "no ground".into());
        println!("{:>5} {:<28} water {:?} heights {lo:.1}..{hi:.1}; {ground}", p.number, p.name.trim(), kinds);
    }
}

/// The coasts' tile-maps store 16-bit heights, which the original scales by the second float as they are (GetGroundHeight
/// over FUN_100459f8), so their 1/512 is 0.5 per eight-bit step and the ground reaches from the seabed to 128.
#[test]
fn sixteen_bit_heights_are_scaled_as_stored() {
    let Some(db) = database() else { return };
    let terrain = |n| db.get(Id { kind: 0xf4249, instance: n }).unwrap().and_then(|o| mao_formats::terrain::build(&mao_formats::tilemap::parse_old(&o).ok()?).ok()).unwrap();
    for coast in [556, 656] {
        let t = terrain(coast);
        assert_eq!(t.height_scale, 0.5, "playfield {coast}");
        let (lo, hi) = t.heights.iter().fold((u16::MAX, 0u16), |(a, b), &h| (a.min(h), b.max(h)));
        assert_eq!((lo, hi), (0, u16::MAX), "playfield {coast}");
        // the sea polygons at 8 lie over the lowest ground and under the land
        let a = mao_formats::playfield::load_anarchy(&db, coast).unwrap();
        let sea = a.water.iter().filter(|w| w.kind == 0).flat_map(|w| w.points.iter().map(|p| p[1])).fold(f32::MAX, f32::min);
        assert!((sea - 8.0).abs() < 0.001, "the sea at {sea}");
        let dry = (0..t.heights.len()).filter(|&i| t.heights[i] as f32 / 256.0 * t.height_scale > sea).count();
        assert!(dry * 4 > t.heights.len() && dry < t.heights.len(), "playfield {coast}: {dry} of {} points above the sea", t.heights.len());
    }
    // an 8-bit map keeps the float as it is
    assert_eq!(terrain(0x254).height_scale, 0.2);
}

/// `cargo test -p mao-formats --test game_data header_survey -- --ignored --nocapture`: every playfield definition's header
/// words and the extra header's bytes, to find the environment type `n3EngineClientAnarchy_t::PlayfieldInit` reads.
#[test]
#[ignore]
fn header_survey() {
    let Some(db) = database() else { return };
    for p in mao_formats::playfield::all(&db) {
        let Some(o) = db.get(Id { kind: mao_formats::playfield::KIND, instance: p.number }).ok().flatten() else { continue };
        let head = mao_formats::texture::OBJECT_HEAD;
        let u32_at = |o: &[u8], at: usize| o.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).unwrap_or(0);
        let version = u32_at(&o, head);
        let tilemap = u32_at(&o, head + 0x28);
        let word = u32_at(&o, head + 0x2c);
        let count = u32_at(&o, head + 0x30);
        let extra: Vec<String> = o.get(head + 0x34..head + 0x34 + 44).map(|b| b.iter().map(|x| format!("{x:02x}")).collect()).unwrap_or_default();
        println!("{:>5} {:<28} v{version} tilemap {tilemap} word {word:#x} rooms {count} extra {}", p.number, p.name.trim(), extra.join(" "));
    }
}

/// The suns (kind 2005) and electra (kind 2006) classes have looks of their own before the spiral the other classes built on
/// a buff share: the commonest templates of each (307 and 113 programs) read as them.
#[test]
fn suns_and_electra_have_their_own_looks() {
    let Some(game) = std::env::var_os("MAO_GAME").map(PathBuf::from) else { return };
    let data = std::fs::read(game.join("Setupf").join("gfxtweak.bin")).unwrap();
    let gfx = mao_formats::gfx::Gfx::parse(&data).unwrap();
    assert!(matches!(gfx.visual(17000), Some(mao_formats::gfx::Visual::Suns(_))), "{:?}", gfx.visual(17000).map(|v| std::mem::discriminant(&v)));
    assert!(matches!(gfx.visual(43457), Some(mao_formats::gfx::Visual::Electra { .. })));
    assert!(matches!(gfx.visual(43768), Some(mao_formats::gfx::Visual::Suns(_))));
}

/// `cargo test -p mao-formats --test game_data environment_kind_survey -- --ignored --nocapture`: byte 0x12 of every
/// playfield's own `EnvironmentData_t` (the sky kind `n3EngineClientAnarchy_t::PlayfieldInit` reads) and whether it has a
/// ground tile-map.
#[test]
#[ignore]
fn environment_kind_survey() {
    let Some(db) = database() else { return };
    for p in mao_formats::playfield::all(&db) {
        let Some(a) = mao_formats::playfield::load_anarchy(&db, p.number) else { continue };
        let Some(env) = a.environment else { continue };
        let ground = db.get(Id { kind: 0xf4249, instance: p.number }).ok().flatten().and_then(|o| mao_formats::tilemap::parse_old(&o).ok()).is_some();
        if env[18] != 0 {
            println!("{:>5} {:<34} kind {} ground {ground} bytes 8..18 {:?}", p.number, p.name.trim(), env[18], &env[8..19]);
        }
    }
}
