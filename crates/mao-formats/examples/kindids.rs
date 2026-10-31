//! `kindids <game> <kind hex> [high16]`: the instances of a record kind, optionally those whose high 16 bits are `high16`.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let kind = u32::from_str_radix(args[1].trim_start_matches("0x"), 16).unwrap();
    let high: Option<u32> = args.get(2).and_then(|a| a.parse().ok());
    for id in db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == kind && high.is_none_or(|h| i.instance >> 16 == h)) {
        println!("{:#x} (zone {})", id.instance, id.instance & 0xffff);
    }
}
