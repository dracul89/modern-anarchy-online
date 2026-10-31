//! `kinddump <game> <kind hex>`: every record of a kind, as hex.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let kind = u32::from_str_radix(args[1].trim_start_matches("0x"), 16).unwrap();
    let ids: Vec<_> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == kind).collect();
    for id in ids {
        let data = db.get(id).ok().flatten().unwrap_or_default();
        let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
        println!("{:#x} {} {}", id.instance, data.len(), hex);
    }
}
