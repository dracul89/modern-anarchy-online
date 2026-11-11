//! `dynelroots <game>`: the meshes of the playfields' built-in things whose root frame has an animation, with how many
//! things use each, the root's saved animation matrix and how many keys of each kind its animation has.
use mao_formats::resource::{Id, ResourceDatabase};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let tables: Vec<Id> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::dynels::KIND).collect();
    let mut templates: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    let mut meshes: BTreeMap<u32, (usize, u32)> = BTreeMap::new();
    for id in tables {
        let Some(table) = db.get(id).ok().flatten() else { continue };
        for d in mao_formats::dynels::parse(&table) {
            let mesh = d.stat(12).filter(|&m| m > 0).or_else(|| {
                *templates.entry(d.template).or_insert_with(|| {
                    let data = db.get(Id { kind: mao_formats::item::TEMPLATE, instance: d.template }).ok().flatten()?;
                    mao_formats::item::Item::parse(&data)?.stat(12).filter(|&m| m > 0).map(|m| m as u32)
                })
            });
            if let Some(m) = mesh {
                let e = meshes.entry(m).or_insert((0, d.kind));
                e.0 += 1;
            }
        }
    }
    println!("{} meshes", meshes.len());
    let floats = |v: &[u8]| -> Vec<f32> { v.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect() };
    for (mesh, (count, kind)) in meshes {
        let Some(object) = db.get(Id { kind: 0xf6951, instance: mesh }).ok().flatten() else { continue };
        let Ok(m) = mao_formats::mesh::parse(&object) else { continue };
        let a = &m.archive;
        let roots: Vec<i32> = a.root.field("0", "obj").map(|f| f.values.iter().filter(|v| v.len() >= 4).map(|v| i32::from_le_bytes(v[..4].try_into().unwrap())).collect()).unwrap_or_default();
        for r in roots {
            let Some(frame) = a.objects.get(r as usize) else { continue };
            let matrix = frame.field("", "anim_matrix").and_then(|f| f.values.first()).map(|v| floats(v));
            let anim = frame.int("0", "anim").and_then(|i| a.objects.get(usize::try_from(i).ok()?));
            let identity = matrix.as_ref().is_none_or(|m| m.iter().enumerate().all(|(i, v)| (v - if i % 5 == 0 { 1.0 } else { 0.0 }).abs() < 1e-4));
            if anim.is_none() && identity {
                continue;
            }
            let keys: Vec<String> = ["trans_keys", "rot_keys", "uv_keys", "vis_keys"].iter().map(|n| format!("{n} {}", anim.and_then(|s| s.field("", n)).and_then(|f| f.values.first()).map_or(0, |v| v.len()))).collect();
            let children = frame.field("0", "chld").map_or(0, |f| f.values.len());
            println!("mesh {mesh} used {count} kind {kind:#x} class {:?} children {children} identity {identity} {:?} matrix {:.2?}", a.class(r as usize), keys, matrix.unwrap_or_default());
        }
    }
}
