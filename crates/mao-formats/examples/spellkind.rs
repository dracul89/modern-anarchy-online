//! `spellkind <game> <spell type hex> [limit]`: the item templates (and nano programs) that have a spell of that type
//! among their events: the template, its name, the event and the spell's own values. For finding out what a spell type
//! does from the items that carry it.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let kind = u32::from_str_radix(args[1].trim_start_matches("0x"), 16).expect("a spell type in hex");
    let limit: usize = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(60);
    let mut found = 0usize;
    for id in db.ids().expect("the ids").into_iter().flatten().filter(|i| i.kind == mao_formats::item::TEMPLATE || i.kind == 0xfde85) {
        let Some(data) = db.get(Id { kind: id.kind, instance: id.instance }).ok().flatten() else { continue };
        for (event, spells) in mao_formats::spell::lists(&data) {
            for s in spells.iter().filter(|s| s.kind == kind) {
                found += 1;
                if found <= limit {
                    let name = mao_formats::item::Item::parse(&data).map(|i| i.name).unwrap_or_default();
                    println!("{:x}:{} {name:?} event {event}: criteria {:?} stats {:?} strings {:?}", id.kind, id.instance, s.criteria, s.stats, s.strings);
                }
            }
        }
    }
    println!("{found} spells of type {kind:#x}");
}
