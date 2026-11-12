//! `meshnames <game> <word>...`: the static meshes (type 0xf6951) whose file names hold one of the words, with their
//! instances; or, for a number, that instance's name.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    for (kind, name, instance) in index.iter() {
        if kind == 0xf6951 && args[1..].iter().any(|w| name.to_lowercase().contains(&w.to_lowercase()) || w.parse() == Ok(instance)) {
            println!("{instance} {name}");
        }
    }
}
