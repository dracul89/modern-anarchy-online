//! `areanames <game> <playfield>...`: each playfield's areas by name, in order.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    for pf in &args[1..] {
        let names: Vec<String> = mao_formats::areas::load(&db, pf.parse().unwrap()).into_iter().map(|a| a.name).collect();
        println!("{pf}: {names:?}");
    }
}
