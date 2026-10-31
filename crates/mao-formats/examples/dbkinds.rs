//! `dbkinds <game>`: each record kind in the resource database with its count.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let mut kinds = std::collections::BTreeMap::new();
    for id in db.ids().unwrap().filter_map(Result::ok) {
        *kinds.entry(id.kind).or_insert(0u32) += 1;
    }
    for (k, n) in kinds {
        println!("{k:#x} {n}");
    }
}
