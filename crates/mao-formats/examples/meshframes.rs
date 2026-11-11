//! `meshframes <game> <mesh instance>...`: a mesh's frame tree: each frame's class, its local position, rotation and scale,
//! whether it stores an animation matrix, and its animation's keys (how many of each kind, the first of each, its length).
use mao_formats::archive::{Archive, Stream};
use mao_formats::resource::{Id, ResourceDatabase};

fn floats(s: &Stream, prefix: &str, name: &str) -> Vec<f32> {
    s.field(prefix, name).and_then(|f| f.values.first()).map(|v| v.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect()).unwrap_or_default()
}

fn refs(s: &Stream, name: &str) -> Vec<i32> {
    s.field("0", name).map(|f| f.values.iter().filter(|v| v.len() >= 4).map(|v| i32::from_le_bytes(v[..4].try_into().unwrap())).collect()).unwrap_or_default()
}

fn walk(a: &Archive, index: i32, depth: usize) {
    let Some(frame) = (index >= 0).then(|| a.objects.get(index as usize)).flatten() else { return };
    let pad = "  ".repeat(depth);
    println!("{pad}{} #{index}: pos {:?} rot {:?} scale {:?} anim_matrix {:?}", a.class(index as usize).unwrap_or("?"), floats(frame, "", "local_pos"), floats(frame, "", "local_rot"), floats(frame, "", "scale"), floats(frame, "", "anim_matrix"));
    if let Some(anim) = frame.int("0", "anim").and_then(|i| a.objects.get(usize::try_from(i).ok()?)) {
        for (name, size) in [("trans_keys", 4), ("rot_keys", 5), ("uv_keys", 6), ("vis_keys", 2)] {
            let k = floats(anim, "", name);
            if !k.is_empty() {
                println!("{pad}  {name}: {} keys, first {:?}, last {:?}", k.len() / size, &k[..size], &k[k.len() - size..]);
            }
        }
        println!("{pad}  length {:?} loop {:?}", floats(anim, "", "tot_time"), anim.field("", "loop").and_then(|f| f.values.first()).map(|v| v.to_vec()));
    }
    for child in refs(frame, "chld") {
        walk(a, child, depth + 1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    for id in &args[1..] {
        let instance: u32 = id.parse().unwrap();
        let object = db.get(Id { kind: 0xf6951, instance }).unwrap().expect("no such mesh");
        let mesh = mao_formats::mesh::parse(&object).unwrap();
        println!("mesh {instance}:");
        for root in refs(&mesh.archive.root, "obj") {
            walk(&mesh.archive, root, 1);
        }
    }
}
