//! `meshclasses <game> <mesh instance>...`: every object of a mesh's archive: its class and its fields' names with how
//! many values each has and the first few as words or text (to find what a mesh carries beside its triangles).
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
            println!("object {i} {}:", a.class(i).unwrap_or("?"));
            for f in &o.fields {
                let shown: Vec<String> = f
                    .values
                    .iter()
                    .take(6)
                    .map(|v| {
                        if v.len() == 4 {
                            format!("{:#x}", u32::from_le_bytes(v[..4].try_into().unwrap()))
                        } else if v.len() < 64 && v.iter().all(|c| c.is_ascii_graphic() || *c == b' ' || *c == 0) {
                            format!("{:?}", String::from_utf8_lossy(v).trim_end_matches('\0'))
                        } else {
                            format!("{} bytes", v.len())
                        }
                    })
                    .collect();
                println!("  {:?} {:?} x{}: {}", f.prefix, f.name, f.values.len(), shown.join(" "));
            }
        }
    }
}
