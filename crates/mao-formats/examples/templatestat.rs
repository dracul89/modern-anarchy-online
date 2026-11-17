//! `templatestat <game> <stat> <template>...`: a stat of item templates (their names too).
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let stat: u32 = args[1].parse().expect("a stat's number");
    for id in &args[2..] {
        let Ok(instance) = id.parse() else { continue };
        let item = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d));
        println!("{id} {:?}: stat {stat} = {:?}", item.as_ref().map(|i| i.name.clone()), item.as_ref().and_then(|i| i.stat(stat)));
    }
}
