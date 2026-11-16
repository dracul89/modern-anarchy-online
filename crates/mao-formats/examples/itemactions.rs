//! `itemactions <game> <template>...`: an item template's name, stats and actions (the action's kind and its criteria as
//! `stat value operator`): what the server asks of a character that uses, wears or wields the item.
//! `itemactions <game> survey`: over every item template, how many have how many actions, and how often each kind occurs.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    if args.get(1).is_some_and(|a| a == "survey") {
        let mut by_count = std::collections::BTreeMap::<usize, usize>::new();
        let mut kinds = std::collections::BTreeMap::<u32, usize>::new();
        for id in db.ids().expect("the ids").into_iter().flatten().filter(|i| i.kind == mao_formats::item::TEMPLATE) {
            let Some(data) = db.get(id).ok().flatten() else { continue };
            let actions = mao_formats::item::actions(&data);
            *by_count.entry(actions.len()).or_default() += 1;
            for a in actions {
                *kinds.entry(a.kind).or_default() += 1;
            }
        }
        println!("templates by their number of actions: {by_count:?}");
        println!("actions by kind: {kinds:?}");
        return;
    }
    for id in args[1..].iter().filter_map(|a| a.parse::<u32>().ok()) {
        let Some(data) = db.get(Id { kind: mao_formats::item::TEMPLATE, instance: id }).ok().flatten() else {
            println!("{id}: no such template");
            continue;
        };
        let item = mao_formats::item::Item::parse(&data);
        println!("{id} {:?} stats {:?}", item.as_ref().map(|i| i.name.as_str()), item.as_ref().map(|i| &i.stats));
        for action in mao_formats::item::actions(&data) {
            println!("  action {}: {:?}", action.kind, action.criteria);
        }
    }
}
