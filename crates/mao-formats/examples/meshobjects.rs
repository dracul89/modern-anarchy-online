//! `meshobjects <game> <mesh instance>...`: every object of a mesh's archive with its class and its fields (a field's
//! values as words when they are four bytes, as text when they read as text, else their length).
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    for id in &args[1..] {
        let instance: u32 = id.parse().unwrap();
        let object = db.get(Id { kind: 0xf6951, instance }).unwrap().expect("no such mesh");
        let mesh = mao_formats::mesh::parse(&object).unwrap();
        let a = &mesh.archive;
        for (i, o) in std::iter::once((usize::MAX, &a.root)).chain(a.objects.iter().enumerate()) {
            let class = if i == usize::MAX { "(root)" } else { a.class(i).unwrap_or("?") };
            println!("mesh {instance} object {} {class}:", i as isize);
            for f in &o.fields {
                let words: Vec<String> = f
                    .values
                    .iter()
                    .take(16)
                    .map(|v| {
                        if v.len() == 4 {
                            format!("{:#x}", u32::from_le_bytes(v[..4].try_into().unwrap()))
                        } else if v.len() <= 64 && !v.is_empty() && v.iter().all(|&b| b == 0 || (0x20..0x7f).contains(&b)) {
                            format!("{:?}", String::from_utf8_lossy(v).trim_end_matches('\0'))
                        } else if v.len() <= 16 {
                            format!("{v:02x?}")
                        } else {
                            format!("[{} bytes]", v.len())
                        }
                    })
                    .collect();
                println!("  {:?} {:?} kind {:?}: {}", f.prefix, f.name, f.kind, words.join(" "));
            }
        }
    }
}
