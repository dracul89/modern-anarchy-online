//! `itemactions <game> <template>...`: an item template's name, stats and actions (the action's kind and its criteria as
//! `stat value operator`): what the server asks of a character that uses, wears or wields the item.
//! `itemactions <game> survey`: over every item template, how many have how many actions, and how often each kind occurs.
//! `itemactions <game> worn`: over the templates that can be worn (stat 76 of 1, 2, 3 or 5), by that class: how many
//! have a wield action (8) and how many of those ask something, the same for the wear action (6), and how many have
//! the wear bit (4) among their "can" flags (stat 30); with a few names of class 1 items that have no wield action.
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
    if args.get(1).is_some_and(|a| a == "worn") {
        // class -> [templates, with action 8, action 8 asking, with action 6, action 6 asking, with the wear bit]
        let mut by_class = std::collections::BTreeMap::<u32, [usize; 6]>::new();
        let mut without = Vec::new();
        for id in db.ids().expect("the ids").into_iter().flatten().filter(|i| i.kind == mao_formats::item::TEMPLATE) {
            let Some(data) = db.get(id).ok().flatten() else { continue };
            let Some(item) = mao_formats::item::Item::parse(&data) else { continue };
            let stat = |k: u32| item.stats.iter().find(|s| s.0 == k).map(|s| s.1);
            let Some(class) = stat(76).filter(|c| matches!(c, 1 | 2 | 3 | 5)) else { continue };
            let actions = mao_formats::item::actions(&data);
            let row = by_class.entry(class).or_default();
            row[0] += 1;
            for (kind, at) in [(8, 1), (6, 3)] {
                if let Some(a) = actions.iter().find(|a| a.kind == kind) {
                    row[at] += 1;
                    row[at + 1] += usize::from(!a.criteria.is_empty());
                }
            }
            row[5] += usize::from(stat(30).is_some_and(|can| can & 4 != 0));
            if class == 1 && !actions.iter().any(|a| a.kind == 8) && without.len() < 40 {
                without.push((id.instance, item.name.clone(), stat(30).unwrap_or(0), stat(298).unwrap_or(0)));
            }
        }
        println!("class: [templates, wield action, wield asking, wear action, wear asking, wear bit]");
        for (class, row) in by_class {
            println!("{class}: {row:?}");
        }
        for (id, name, can, places) in without {
            println!("  class 1 without a wield action: {id} {name:?} can {can:#x} places {places:#x}");
        }
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
