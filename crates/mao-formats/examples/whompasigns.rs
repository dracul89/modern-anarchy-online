//! Lists the Whom-Pah signs of every playfield: the statels with a texture whose name has `whompa` in it.
//! `whompasigns <game>` prints, for each, the playfield, the place and the texture's name.

use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let game = std::env::args().nth(1).expect("usage: whompasigns <game>");
    let db = ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
    let index = db.get(Id { kind: 0xf424a, instance: 1 }).ok().flatten().and_then(|o| mao_formats::instances::Instances::parse(&o).ok()).expect("the instances index");
    let signs: std::collections::HashMap<u32, String> = index.iter().filter(|&(kind, name, _)| kind == 0xf6954 && name.to_lowercase().contains("whompa")).map(|(_, name, instance)| (instance, name.to_string())).collect();
    let mut names: std::collections::BTreeMap<String, usize> = Default::default();
    let mut files: Vec<_> = std::fs::read_dir(std::path::Path::new(&game).join("cd_image/data/statels")).unwrap().flatten().map(|e| e.path()).collect();
    files.sort();
    for path in files {
        let Some(playfield) = path.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<u32>().ok()) else { continue };
        let Ok(file) = mao_formats::files::read(&path) else { continue };
        let Ok(statels) = mao_formats::statels::parse(&file) else { continue };
        for s in &statels {
            for e in &s.extras {
                if let Some(name) = signs.get(&e.2) {
                    println!("{playfield} ({:.1}, {:.1}, {:.1}) {name}", s.position[0], s.position[1], s.position[2]);
                    *names.entry(name.clone()).or_default() += 1;
                }
            }
        }
    }
    for p in mao_formats::playfield::all(&db) {
        eprintln!("playfield {} {}", p.number, p.name.trim());
    }
    eprintln!("{} names:", names.len());
    for (name, count) in names {
        eprintln!("  {count:4} {name}");
    }
}
