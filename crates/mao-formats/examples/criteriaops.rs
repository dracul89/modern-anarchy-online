//! `criteriaops <game>`: how often each criteria operator occurs in the item templates' actions and spells, with an example.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let mut acts: std::collections::BTreeMap<u32, (usize, String)> = Default::default();
    let mut spells: std::collections::BTreeMap<u32, (usize, String)> = Default::default();
    for id in db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::item::TEMPLATE) {
        let Ok(Some(data)) = db.get(id) else { continue };
        for a in mao_formats::item::actions(&data) {
            for c in &a.criteria {
                let e = acts.entry(c.2).or_insert((0, String::new()));
                e.0 += 1;
                if e.1.is_empty() {
                    e.1 = format!("{} action {} {:?}", id.instance, a.kind, a.criteria);
                }
            }
        }
        for (ev, list) in mao_formats::spell::lists(&data) {
            for s in list {
                for c in &s.criteria {
                    let e = spells.entry(c.2).or_insert((0, String::new()));
                    e.0 += 1;
                    if e.1.is_empty() {
                        e.1 = format!("{} event {ev} spell {:#x} {:?}", id.instance, s.kind, s.criteria);
                    }
                }
            }
        }
    }
    println!("actions");
    for (op, (n, ex)) in &acts {
        println!("{op} ({op:#x}): {n}  {}", &ex[..ex.len().min(160)]);
    }
    println!("spells");
    for (op, (n, ex)) in &spells {
        println!("{op} ({op:#x}): {n}  {}", &ex[..ex.len().min(160)]);
    }
}
