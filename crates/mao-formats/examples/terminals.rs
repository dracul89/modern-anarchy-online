//! `terminals <game> [playfield]`: the mission terminals built into the playfields (dynel type 0xdac1), counted by the kind of
//! mission they give (stat 490, `QuestOriginator_e`: the record's own or its template's) and by name. The even kinds
//! 2, 4, 6 and 8 are a team's (`GameData::IsTeamOriginator`). With a playfield: each of its terminals, by its number in
//! the table, with its kind and place.
use mao_formats::resource::{Id, ResourceDatabase};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let mut seen: BTreeMap<(Option<u32>, bool, String), (u32, Vec<u32>)> = BTreeMap::new();
    let mut templates: BTreeMap<u32, (String, Option<u32>)> = BTreeMap::new();
    for p in mao_formats::playfield::all(&db) {
        let Some(object) = db.get(Id { kind: mao_formats::dynels::KIND, instance: p.number }).ok().flatten() else { continue };
        for d in mao_formats::dynels::parse(&object).into_iter().filter(|d| d.kind == 0xdac1) {
            let (name, from_template) = templates
                .entry(d.template)
                .or_insert_with(|| {
                    let item = db.get(Id { kind: mao_formats::item::TEMPLATE, instance: d.template }).ok().flatten().and_then(|data| mao_formats::item::Item::parse(&data));
                    (item.as_ref().map_or_else(String::new, |i| i.name.clone()), item.and_then(|i| i.stat(490)))
                })
                .clone();
            let own = d.stat(490);
            if args.get(1).and_then(|a| a.parse::<u32>().ok()) == Some(p.number) {
                println!("terminal {} ({:#x}): kind {:?}, {name:?} at {:?}", d.instance, d.instance, own.or(from_template), d.position);
            }
            let entry = seen.entry((own.or(from_template), own.is_some(), name)).or_default();
            entry.0 += 1;
            if entry.1.len() < 4 && !entry.1.contains(&p.number) {
                entry.1.push(p.number);
            }
        }
    }
    for ((kind, own, name), (count, playfields)) in seen {
        println!("kind {kind:?} ({}) {name:?}: {count} terminals, e.g. playfields {playfields:?}", if own { "its own stat" } else { "its template's" });
    }
}
