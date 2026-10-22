//! `mao-db <game dir> info | tilemap-new <playfield> <dir> | scan-tilemaps | tilemap <playfield> | list <type> [count] | chunks <type> <instance> | find-meshes <max parts> <min triangles> | mesh-preview <type> <instance> <out.png> | dump-mesh <type> <instance> | survey-meshes | verify | texture <type> <instance> <out.png> | scan-textures | get <type> <instance> (hex) | count-types`

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use mao_formats::resource::{Id, ResourceDatabase};
use mao_formats::texture;

const TEXTURE_TYPES: [u32; 9] = [0xf6954, 0xf6959, 0xf695b, 0xf6960, 0xf6961, 0xf6963, 0xf6964, 0xf6967, 0xf6968];

/// A hexadecimal number, with or without `0x`.
fn number(text: &str) -> Option<u32> {
    u32::from_str_radix(text.strip_prefix("0x").unwrap_or(text), 16).ok()
}

fn run(args: &[String]) -> Result<(), String> {
    let [game, command, rest @ ..] = args else {
        return Err("usage: mao-db <game dir> info | tilemap-new <playfield> <dir> | scan-tilemaps | tilemap <playfield> | list <type> [count] | chunks <type> <instance> | find-meshes <max parts> <min triangles> | mesh-preview <type> <instance> <out.png> | dump-mesh <type> <instance> | survey-meshes | verify | texture <type> <instance> <out.png> | scan-textures | get <type> <instance> (hex) | count-types".into());
    };
    let db = ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?;
    match (command.as_str(), rest) {
        ("info", []) => println!("{} objects", db.len()),
        ("get", [kind, instance]) => {
            let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
            match db.get(id).map_err(|e| e.to_string())? {
                Some(data) => {
                    println!("{} bytes", data.len());
                    let head: Vec<String> = data.iter().take(std::env::var("MAO_BYTES").ok().and_then(|n| n.parse().ok()).unwrap_or(32)).map(|b| format!("{b:02x}")).collect();
                    println!("{}", head.join(" "));
                }
                None => return Err("no such object".into()),
            }
        }
        ("anim", [kind, instance]) => {
            let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
            let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
            let a = mao_formats::catanim::parse(&object).map_err(|e| e.to_string())?;
            println!("{:?}: {} tracks, duration {}, first {}, {} notes", a.name, a.tracks.len(), a.duration, a.first, a.notes.len());
            for n in a.notes.iter().take(6) {
                println!("  note {} at {}", n.name, n.time);
            }
            let keys: Vec<usize> = a.tracks.iter().map(|t| t.rotations.len()).collect();
            println!("  rotation keys per track: {keys:?}");
            if let Some(t) = a.tracks.get(1) {
                println!("  track 1 times {:?}", t.rotations.iter().map(|k| k.0).collect::<Vec<_>>());
                println!("  track 1 first rotation {:?} translation {:?}", t.rotations.first().map(|k| k.1), t.translations.first().map(|k| k.1));
            }
        }
        ("anims", [kind, count]) => {
            let kind = number(kind).ok_or("bad type")?;
            let mut shown = 0;
            for id in db.ids().map_err(|e| e.to_string())?.filter_map(|i| i.ok()).filter(|i| i.kind == kind) {
                if let Some(object) = db.get(id).map_err(|e| e.to_string())? {
                    match mao_formats::catanim::parse(&object) {
                        Ok(a) => println!("{:#x} tracks {} len {} notes {:?}", id.instance, a.tracks.len(), a.first, a.notes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>()),
                        Err(e) => println!("{:#x} error {e}", id.instance),
                    }
                    shown += 1;
                    if shown >= count.parse().unwrap_or(10) {
                        break;
                    }
                }
            }
        }
        ("cats", [kind]) => {
            let kind = number(kind).ok_or("bad type")?;
            for id in db.ids().map_err(|e| e.to_string())?.filter_map(|i| i.ok()).filter(|i| i.kind == kind) {
                if let Some(object) = db.get(id).map_err(|e| e.to_string())? {
                    match mao_formats::catmesh::parse(&object) {
                        Ok(m) => {
                            let vertices: usize = m.groups.iter().flat_map(|g| &g.lists).map(|l| l.vertices.len()).sum();
                            let textures: Vec<&str> = m.textures.iter().map(|t| t.name.as_str()).collect();
                            println!("{:#x} {:?} bones {} vertices {vertices} textures {textures:?}", id.instance, m.name, m.bones.len());
                        }
                        Err(e) => println!("{:#x} error {e}", id.instance),
                    }
                }
            }
        }
        ("grep", [kind, hex]) => {
            // Objects of a type containing the bytes (hex).
            let kind = number(kind).ok_or("bad type")?;
            let needle: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()).collect();
            for id in db.ids().map_err(|e| e.to_string())?.filter_map(|i| i.ok()).filter(|i| i.kind == kind) {
                if let Some(object) = db.get(id).map_err(|e| e.to_string())? {
                    if let Some(at) = object.windows(needle.len()).position(|w| w == needle.as_slice()) {
                        println!("{:#x}:{:#x} at {at} (of {})", id.kind, id.instance, object.len());
                    }
                }
            }
        }
        ("names", rest) => {
            let object = db.get(Id { kind: 0xf424a, instance: 1 }).map_err(|e| e.to_string())?.ok_or("no instance index")?;
            let index = mao_formats::instances::Instances::parse(&object).map_err(|e| e.to_string())?;
            println!("{} names", index.len());
            let filter = rest.first().map(|s| s.to_lowercase());
            let mut rows: Vec<_> = index.iter().filter(|(_, n, _)| filter.as_ref().is_none_or(|f| n.to_lowercase().contains(f))).collect();
            rows.sort();
            let kind_filter: Option<u32> = rest.get(1).and_then(|k| number(k));
            for (kind, name, instance) in rows.into_iter().filter(|r| kind_filter.is_none_or(|k| r.0 == k)).take(100000) {
                println!("{kind:#x} {instance:#x} {name}");
            }
        }
        ("statels-in", [pf, x, z, r]) => {
            let (x, z, r): (f32, f32, f32) = (x.parse().map_err(|_| "x")?, z.parse().map_err(|_| "z")?, r.parse().map_err(|_| "r")?);
            let file = std::fs::read(Path::new(game).join(format!("cd_image/data/statels/{pf}.pf"))).map_err(|e| e.to_string())?;
            let list = mao_formats::statels::parse(&file).map_err(|e| e.to_string())?;
            for s in list.iter().filter(|s| (s.position[0] - x).hypot(s.position[2] - z) < r) {
                let tilted = s.rotation[0].abs() > 0.05 || s.rotation[2].abs() > 0.05;
                println!("{:?} mesh {:#x} scale {:?} rot {:.2?} layout{} {}", s.position.map(|v| (v * 10.0).round() / 10.0), s.mesh, s.scale_xyz, s.rotation, if s.unread_rotation { "2" } else { "1" }, if tilted { "TILTED" } else { "" });
            }
        }
        ("cat", [kind, instance]) => {
            let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
            let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
            let m = mao_formats::catmesh::parse(&object).map_err(|e| e.to_string())?;
            println!("{:?} version {:#x}, colour {:#010x}", m.name, m.version, m.colour);
            for t in &m.textures {
                println!("  texture {:?} {:#x} {:#x} flag {}", t.name, t.value1, t.value2, t.flag);
            }
            for (i, mat) in m.materials.iter().enumerate() {
                println!("  material {i} {:?} flags {:#x} texture {:?} env {:?} diffuse {:?} opacity {}", mat.name, mat.flags, mat.texture, mat.environment, mat.diffuse, mat.opacity);
            }
            println!("  {} bones, {} spheres", m.bones.len(), m.spheres.len());
            for b in m.bones.iter().take(6) {
                println!("    bone {:?} scale {} children {:?}", b.name, b.scale, b.children);
            }
            for g in &m.groups {
                println!("  group {:?}: {} attractors", g.name, g.attractors.len());
                for a in &g.attractors {
                    println!("    attractor {:?} {:.2?} extra {}", a.name, a.values, a.extra);
                }
                for l in &g.lists {
                    let (lo, hi) = l.vertices.iter().fold((i32::MAX, i32::MIN), |(lo, hi), v| (lo.min(v.bones[0]).min(v.bones[1]), hi.max(v.bones[0]).max(v.bones[1])));
                    let (mn, mx) = l.vertices.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(mn, mx), v| ([mn[0].min(v.rest[0]), mn[1].min(v.rest[1]), mn[2].min(v.rest[2])], [mx[0].max(v.rest[0]), mx[1].max(v.rest[1]), mx[2].max(v.rest[2])]));
                    println!("    list material {} {} vertices {} indices, bones {lo}..{hi}, rest box {:.2?}..{:.2?}", l.material, l.vertices.len(), l.indices.len(), mn, mx);
                }
            }
        }
        ("texture", [kind, instance, out]) => {
            let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
            let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
            let t = if matches!(id.kind, 0xf6956 | 0xf6965 | 0xf6966) { texture::decode_ground(&object) } else { texture::decode(&object) }.map_err(|e| e.to_string())?;
            println!("{}x{} {:?}", t.width, t.height, t.format);
            image::save_buffer(out, &t.rgba, t.width, t.height, image::ColorType::Rgba8).map_err(|e| e.to_string())?;
        }
        ("scan-textures", []) => {
            let (mut ok, mut formats, mut failed) = (0u64, BTreeMap::new(), Vec::new());
            for id in db.ids().map_err(|e| e.to_string())? {
                let id = id.map_err(|e| e.to_string())?;
                if !TEXTURE_TYPES.contains(&id.kind) {
                    continue;
                }
                let Ok(Some(object)) = db.get(id) else { continue };
                match texture::decode(&object) {
                    Ok(t) => {
                        ok += 1;
                        *formats.entry(format!("{:?}", t.format)).or_insert(0u64) += 1;
                    }
                    Err(e) => failed.push((id, e.to_string())),
                }
            }
            println!("{ok} decoded {formats:?}, {} failed", failed.len());
            for (id, why) in failed.iter().take(10) {
                println!("failed {:#x}:{:#x}: {why}", id.kind, id.instance);
            }
        }
        ("mesh-preview", [kind, instance, out]) => {
            let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
            let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
            let mesh = mao_formats::mesh::parse(&object).map_err(|e| e.to_string())?;
            let mut model = mao_formats::model::build(&mesh).map_err(|e| e.to_string())?;
            if let Some(only) = std::env::var("MAO_PART").ok().and_then(|p| p.parse::<usize>().ok()) {
                model.parts = model.parts.get(only).cloned().into_iter().collect();
            }
            for p in model.parts.iter().filter(|p| p.indices.iter().max().is_some_and(|&m| m as usize >= p.vertices.len())).take(5) {
                println!("part {} verts {} indices {} max index {:?} matrix {:?}", p.name, p.vertices.len(), p.indices.len(), p.indices.iter().max(), p.world);
            }
            println!("{} parts, {} triangles", model.parts.len(), model.parts.iter().map(|p| p.indices.len() / 3).sum::<usize>());
            preview(&model, out).map_err(|e| e.to_string())?;
        }
        ("find-meshes", [max_parts, min_tris]) => {
            let (max_parts, min_tris): (usize, usize) = (max_parts.parse().map_err(|_| "bad number")?, min_tris.parse().map_err(|_| "bad number")?);
            let mut shown = 0;
            for id in db.ids().map_err(|e| e.to_string())? {
                let id = id.map_err(|e| e.to_string())?;
                if id.kind != 0xf6951 {
                    continue;
                }
                let Ok(Some(object)) = db.get(id) else { continue };
                let Ok(mesh) = mao_formats::mesh::parse(&object) else { continue };
                let Ok(model) = mao_formats::model::build(&mesh) else { continue };
                let tris: usize = model.parts.iter().map(|p| p.indices.len() / 3).sum();
                if model.parts.len() <= max_parts && !model.parts.is_empty() && tris >= min_tris {
                    println!("{:#x} {:#x}: {} parts, {tris} triangles", id.kind, id.instance, model.parts.len());
                    shown += 1;
                    if shown == 12 {
                        break;
                    }
                }
            }
        }
        ("list", [kind, rest @ ..]) => {
            let kind = number(kind).ok_or("bad type")?;
            let count: usize = rest.first().and_then(|c| c.parse().ok()).unwrap_or(10);
            for id in db.ids().map_err(|e| e.to_string())?.filter(|i| i.as_ref().is_ok_and(|i| i.kind == kind)).take(count) {
                let id = id.map_err(|e| e.to_string())?;
                let size = db.get(id).ok().flatten().map(|o| o.len());
                println!("{:#x}:{:#x} {:?} bytes", id.kind, id.instance, size);
            }
        }
        ("tilemap", [instance]) => {
            let id = Id { kind: 0xf4249, instance: number(instance).ok_or("bad instance")? };
            let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
            let map = mao_formats::tilemap::parse_old(&object).map_err(|e| e.to_string())?;
            println!("{}x{} tile {} second {} types {:?} rest {} bytes", map.width, map.height, map.tile_size, map.second, map.tile_types, map.rest.len());
            for (i, stream) in map.archive.objects.iter().enumerate() {
                println!("#{i} {}", map.archive.class(i).unwrap_or("?"));
                for f in &stream.fields {
                    let size: usize = f.values.iter().map(Vec::len).sum();
                    println!("    {}:{} [{}] {} values, {size} bytes", f.prefix, f.name, f.kind, f.values.len());
                }
            }
            println!("root: {:?}", map.archive.root.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>());
        }
        ("tilemap-new", [instance, dir]) => {
            let id = Id { kind: 0xf4249, instance: number(instance).ok_or("bad instance")? };
            let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
            let map = mao_formats::tilemap::parse_new(&object).map_err(|e| e.to_string())?;
            println!("{}x{} scale {} second {} types {}", map.width, map.height, map.scale, map.second, map.tile_types.len());
            for c in &map.chunks {
                let png = c.body.windows(4).position(|w| w == [0x89, b'P', b'N', b'G'] || w == [0xff, 0xd8, 0xff, 0xe0]);
                if &c.tag == b"IRHA" || &c.tag == b"HSTA" {
                    println!("   words {:?}", c.body.chunks_exact(4).take(40).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect::<Vec<_>>());
                    println!("   floats {:?}", c.body.chunks_exact(4).take(12).map(|w| f32::from_le_bytes(w.try_into().unwrap())).collect::<Vec<_>>());
                }
                println!("{} v{} {} bytes, image at {png:?}", String::from_utf8_lossy(&c.tag), c.version, c.body.len());
                if let Some(at) = png {
                    let name = format!("{dir}/{}_{}.png", instance, String::from_utf8_lossy(&c.tag));
                    if let Ok(img) = image::load_from_memory(&c.body[at..]) {
                        println!("   {}x{} {:?}", img.width(), img.height(), img.color());
                        let _ = img.save(&name);
                    }
                }
            }
        }
        ("statels", [playfield]) => {
            let pf = std::path::Path::new(game).join(format!("cd_image/data/statels/{}.pf", playfield.parse::<u32>().map_err(|_| "decimal playfield")?));
            let file = std::fs::read(&pf).map_err(|e| format!("{}: {e}", pf.display()))?;
            let (list, zones, failed) = mao_formats::statels::parse_report(&file).map_err(|e| e.to_string())?;
            println!("{zones} zones, {failed} unreadable");
            let unread = list.iter().filter(|s| s.unread_rotation).count();
            let present = list.iter().filter(|s| db.get(Id { kind: 0xf6951, instance: s.mesh }).ok().flatten().is_some()).count();
            println!("{} statels, {unread} with the other rotation layout, {present} with a mesh in the database", list.len());
            for s in list.iter().take(8) {
                println!("{:.1?} mesh {:#x} scale {:.2} rot {:.2?}", s.position, s.mesh, s.scale, s.rotation);
            }
        }
        ("scan-statels", []) => {
            let dir = Path::new(game).join("cd_image/data/statels");
            let (mut files, mut zones, mut bad_zones, mut statels, mut bad_files) = (0, 0, 0, 0, 0);
            for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                if path.extension().is_none_or(|e| e != "pf") {
                    continue;
                }
                files += 1;
                match mao_formats::statels::parse_report(&std::fs::read(&path).map_err(|e| e.to_string())?) {
                    Ok((list, z, bad)) => {
                        zones += z;
                        bad_zones += bad;
                        statels += list.len();
                    }
                    Err(_) => bad_files += 1,
                }
            }
            println!("{files} files ({bad_files} unreadable), {zones} zones ({bad_zones} unreadable), {statels} statels");
        }
        ("scan-tilemaps", []) => {
            let (mut old, mut other, mut bad) = (0, 0, 0);
            for id in db.ids().map_err(|e| e.to_string())? {
                let id = id.map_err(|e| e.to_string())?;
                if id.kind != 0xf4249 {
                    continue;
                }
                let Ok(Some(object)) = db.get(id) else { bad += 1; continue };
                match mao_formats::tilemap::parse_old(&object) {
                    Ok(map) => match mao_formats::terrain::build(&map) {
                        Ok(t) => {
                            old += 1;
                            let (lo, hi) = (t.heights.iter().min().copied().unwrap_or(0), t.heights.iter().max().copied().unwrap_or(0));
                            if old <= 40 && (hi as i32 - lo as i32) > 2000 && t.width < 400 {
                                println!("playfield {:#x}: {}x{} heights {lo}..{hi}", id.instance, t.width, t.height);
                            }
                        }
                        Err(e) => {
                            bad += 1;
                            println!("playfield {:#x}: terrain failed: {e}", id.instance);
                        }
                    },
                    Err(mao_formats::tilemap::Error::Chunk(tag)) => {
                        other += 1;
                        if other <= 6 {
                            println!("other: {:#x} {:?} {} bytes", id.instance, tag.to_le_bytes().map(|b| b as char), object.len());
                        }
                    }
                    Err(e) => {
                        bad += 1;
                        println!("playfield {:#x}: {e}", id.instance);
                    }
                }
            }
            println!("{old} old-style tile-maps, {other} of other chunk kinds, {bad} unreadable");
        }
        ("chunks", [kind, instance]) => {
            let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
            let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
            println!("{} bytes; head {:02x?}", object.len(), &object[..object.len().min(48)]);
            // Every place a 4-letter chunk tag stands (they read backwards: GNDA is "ADNG").
            for at in 0..object.len().saturating_sub(12) {
                let tag = &object[at..at + 4];
                if tag.iter().all(|b| b.is_ascii_uppercase()) && tag[3] == b'A' {
                    let size = u32::from_le_bytes(object[at + 4..at + 8].try_into().unwrap());
                    let version = u32::from_le_bytes(object[at + 8..at + 12].try_into().unwrap());
                    println!("{at:>8}: {} size {size} version {version}", String::from_utf8_lossy(tag));
                }
            }
        }
        ("dump-mesh", [kind, instance]) => {
            let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
            let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
            let mesh = mao_formats::mesh::parse(&object).map_err(|e| e.to_string())?;
            println!("version {} colour {:#x} notes {}", mesh.version, mesh.colour, mesh.notes.len());
            let show = |stream: &mao_formats::archive::Stream| {
                for f in &stream.fields {
                    let values: Vec<String> = f.values.iter().take(4).map(|v| match f.kind {
                        3 | 17 => format!("{}", i32::from_le_bytes(v[..4].try_into().unwrap_or([0; 4]))),
                        10 => format!("{}", f32::from_le_bytes(v[..4].try_into().unwrap_or([0; 4]))),
                        12 | 13 | 15 | 16 => v.chunks(4).map(|c| format!("{:.3}", f32::from_le_bytes(c.try_into().unwrap_or([0; 4])))).collect::<Vec<_>>().join(","),
                        6 => String::from_utf8_lossy(v).trim_end_matches(' ').to_string(),
                        9 if v.len() == 16 => v.chunks(4).map(|c| format!("{:#x}", u32::from_le_bytes(c.try_into().unwrap()))).collect::<Vec<_>>().join(" "),
                        _ => format!("{}B", v.len()),
                    }).collect();
                    println!("    {}:{} [{}] {}", f.prefix, f.name, f.kind, values.join(" | "));
                }
            };
            println!("root:");
            show(&mesh.archive.root);
            for (i, stream) in mesh.archive.objects.iter().enumerate() {
                println!("#{i} {}", mesh.archive.class(i).unwrap_or("?"));
                show(stream);
            }
        }
        ("survey-meshes", []) => {
            // Which classes the mesh archives hold, and the fields of each.
            let mut classes: BTreeMap<String, (u64, BTreeMap<String, u64>)> = BTreeMap::new();
            let (mut meshes, mut failed) = (0u64, Vec::new());
            for id in db.ids().map_err(|e| e.to_string())? {
                let id = id.map_err(|e| e.to_string())?;
                if id.kind != 0xf6951 && id.kind != 0xf696a {
                    continue;
                }
                let Ok(Some(object)) = db.get(id) else { continue };
                match mao_formats::mesh::parse(&object) {
                    Ok(mesh) => {
                        meshes += 1;
                        for (i, stream) in mesh.archive.objects.iter().enumerate() {
                            let class = mesh.archive.class(i).unwrap_or("?").to_string();
                            let entry = classes.entry(class).or_default();
                            entry.0 += 1;
                            for f in &stream.fields {
                                *entry.1.entry(format!("{}:{} ({})", f.prefix, f.name, f.kind)).or_default() += 1;
                            }
                        }
                    }
                    Err(e) => failed.push((id, e.to_string())),
                }
            }
            println!("{meshes} meshes parsed, {} failed", failed.len());
            for (id, why) in failed.iter().take(5) {
                println!("failed {:#x}:{:#x}: {why}", id.kind, id.instance);
            }
            for (class, (count, fields)) in classes {
                println!("{count:>7} {class}");
                for (field, n) in fields {
                    println!("          {n:>7} {field}");
                }
            }
        }
        ("verify", []) => {
            let (count, bytes, highest, failures, failed) = db.verify().map_err(|e| e.to_string())?;
            println!("{count} objects, {bytes} bytes, highest position {highest:#x}, {failures} unreadable");
            for (id, position, why) in failed {
                println!("failed {:#x}:{:#x} at {position:#x}: {why}", id.kind, id.instance);
            }
        }
        ("count-types", []) => {
            let mut counts: BTreeMap<u32, u64> = BTreeMap::new();
            for id in db.ids().map_err(|e| e.to_string())? {
                *counts.entry(id.map_err(|e| e.to_string())?.kind).or_default() += 1;
            }
            for (kind, count) in counts {
                println!("{kind:#x}: {count}");
            }
        }
        _ => return Err("unknown command".into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mao-db: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Wireframes of the model from the front (x, y), the side (z, y) and the
/// top (x, z), each scaled to fit its panel.
fn preview(model: &mao_formats::model::Model, out: &str) -> Result<(), image::ImageError> {
    const PANEL: u32 = 320;
    let world: Vec<Vec<[f32; 3]>> = model
        .parts
        .iter()
        .map(|p| {
            let m = &p.world;
            p.vertices.iter().map(|v| {
                let [x, y, z] = v.position;
                [x * m[0] + y * m[4] + z * m[8] + m[12], x * m[1] + y * m[5] + z * m[9] + m[13], x * m[2] + y * m[6] + z * m[10] + m[14]]
            }).collect()
        })
        .collect();
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for v in world.iter().flatten() {
        for k in 0..3 {
            lo[k] = lo[k].min(v[k]);
            hi[k] = hi[k].max(v[k]);
        }
    }
    let size = (0..3).map(|k| hi[k] - lo[k]).fold(0.0f32, f32::max).max(1e-6);
    let mut img = image::RgbImage::new(PANEL * 3, PANEL);
    for (panel, (h, v)) in [(0usize, 1usize), (2, 1), (0, 2)].into_iter().enumerate() {
        let to = |p: &[f32; 3]| {
            let x = (p[h] - lo[h]) / size * (PANEL - 20) as f32 + 10.0;
            let y = PANEL as f32 - 10.0 - (p[v] - lo[v]) / size * (PANEL - 20) as f32;
            (x + (panel as u32 * PANEL) as f32, y)
        };
        for (part, w) in model.parts.iter().zip(&world) {
            for t in part.indices.chunks_exact(3) {
                for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                    let (Some(p), Some(q)) = (w.get(t[a] as usize), w.get(t[b] as usize)) else { continue };
                    line(&mut img, to(p), to(q));
                }
            }
        }
    }
    img.save(out)
}

fn line(img: &mut image::RgbImage, a: (f32, f32), b: (f32, f32)) {
    let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0) as u32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let (x, y) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        if x >= 0.0 && y >= 0.0 && (x as u32) < img.width() && (y as u32) < img.height() {
            img.put_pixel(x as u32, y as u32, image::Rgb([230, 230, 230]));
        }
    }
}
