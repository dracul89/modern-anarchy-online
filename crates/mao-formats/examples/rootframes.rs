//! `rootframes <game>`: the meshes the playfields' statels use whose root frame is not plain: a rotation or a scale of
//! its own, or a saved matrix that moves it. Each with how many statels use it and one place it stands.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let mut used: std::collections::BTreeMap<u32, (usize, u32, [f32; 3])> = std::collections::BTreeMap::new();
    for p in mao_formats::playfield::all(&db) {
        let path = std::path::Path::new(&args[0]).join(format!("cd_image/data/statels/{}.pf", p.number));
        let Some(list) = mao_formats::files::read(&path).ok().and_then(|file| mao_formats::statels::parse(&file).ok()) else { continue };
        for s in &list {
            used.entry(s.mesh).or_insert((0, p.number, s.position)).0 += 1;
        }
    }
    let (mut turned, mut scaled, mut moved, mut shifted) = (0, 0, 0, 0);
    for (&instance, &(count, playfield, place)) in &used {
        let Some(mesh) = db.get(Id { kind: 0xf6951, instance }).ok().flatten().and_then(|o| mao_formats::mesh::parse(&o).ok()) else { continue };
        for root in mao_formats::model::root_frames(&mesh) {
            let q = root.rotation;
            let is_turned = q[0].abs() > 1e-4 || q[1].abs() > 1e-4 || q[2].abs() > 1e-4;
            let is_scaled = (root.scale - 1.0).abs() > 1e-4;
            let shift = root.anim_matrix.map(|m| [m[12], m[13], m[14]]).filter(|t| t.iter().any(|v| v.abs() > 1e-3));
            let is_moved = root.position.iter().any(|v| v.abs() > 1e-3);
            if !(is_turned || is_scaled || shift.is_some()) {
                moved += is_moved as usize;
                continue;
            }
            turned += is_turned as usize;
            scaled += is_scaled as usize;
            shifted += shift.is_some() as usize;
            println!("mesh {instance}: {count} statels (playfield {playfield} at {place:.1?}); local pos {:.3?} rot {:.3?} scale {}; matrix shift {shift:.3?}{}{}", root.position, q, root.scale, if root.anim_matrix.is_some() { " (has a matrix)" } else { "" }, if root.animated { " animated" } else { "" });
        }
    }
    println!("{} meshes used; root turned {turned}, scaled {scaled}, shifted by its matrix {shifted}; only a position {moved}", used.len());
}
