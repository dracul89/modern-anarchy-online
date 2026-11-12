//! `craftguide <game> [part of a name]...`: the crafting guide read from the item descriptions (`crafting.rs`): how
//! many entries, how many speak of making, how many steps were read; and for each name asked the entries found with
//! their steps and the way to them.
use mao_formats::crafting::Guide;
use mao_formats::item::Item;
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let started = std::time::Instant::now();
    let ids: Vec<Id> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::item::TEMPLATE).collect();
    let items = ids.into_iter().filter_map(|id| db.get(id).ok().flatten().and_then(|d| Item::parse(&d)).map(|i| (id.instance, i.name, i.description)));
    let guide = Guide::build(items);
    let steps: usize = guide.entries.iter().map(|e| e.steps.len()).sum();
    let made = guide.made_by.iter().filter(|m| !m.is_empty()).count();
    println!(
        "{} entries in {:.1} s: {} books and manuals, {} speak of making, {} name other items, {} steps read, {} things have a step that makes them",
        guide.entries.len(),
        started.elapsed().as_secs_f32(),
        guide.entries.iter().filter(|e| e.guide).count(),
        guide.entries.iter().filter(|e| e.speaks).count(),
        guide.entries.iter().filter(|e| !e.names.is_empty()).count(),
        steps,
        made
    );
    for query in &args[1..] {
        println!("== {query}");
        for at in guide.search(query, 6) {
            let e = &guide.entries[at];
            println!("{} (#{}, {} templates; named by {} others)", e.name, e.id, e.templates, guide.named_by[at].len());
            for s in &e.steps {
                println!("   with {:?} makes {:?}", s.with.iter().map(|&w| guide.entries[w].name.as_str()).collect::<Vec<_>>(), guide.entries[s.makes].name);
            }
            for (level, made, by, step) in guide.way_to(at, 5) {
                let s = &guide.entries[by].steps[step];
                let mut parts: Vec<&str> = vec![guide.entries[by].name.as_str()];
                parts.extend(s.with.iter().map(|&w| guide.entries[w].name.as_str()));
                println!("   {}way: {} <- {}", "  ".repeat(level), guide.entries[made].name, parts.join(" + "));
            }
        }
    }
}
