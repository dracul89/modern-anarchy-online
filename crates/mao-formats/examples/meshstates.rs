//! `meshstates <game> <mesh instance>...`: the delta states (`RDeltaState`) of a mesh's materials: every field with its
//! values as 32-bit words (the render states and texture stage states a material sets).
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    for id in &args[1..] {
        let instance: u32 = id.parse().unwrap();
        let object = db.get(Id { kind: 0xf6951, instance }).unwrap().expect("no such mesh");
        let mesh = mao_formats::mesh::parse(&object).unwrap();
        let a = &mesh.archive;
        for (i, o) in a.objects.iter().enumerate() {
            let class = a.class(i).unwrap_or("?");
            if !class.contains("DeltaState") && !class.contains("Material") {
                continue;
            }
            println!("mesh {instance} object {i} {class}:");
            for f in &o.fields {
                let words: Vec<String> = f.values.iter().take(24).map(|v| if v.len() == 4 { format!("{:#x}", u32::from_le_bytes(v[..4].try_into().unwrap())) } else { format!("[{} bytes]", v.len()) }).collect();
                println!("  {:?} {:?}: {}", f.prefix, f.name, words.join(" "));
            }
        }
    }
}
