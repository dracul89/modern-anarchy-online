//! `ldbnames <text.mdb> <name>...`: the texts of the names in the categories the client uses.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t = mao_formats::ldb::Texts::parse(&std::fs::read(&args[1]).expect("the text file")).expect("a text database");
    for name in &args[2..] {
        for cat in [200, 10000, 110] {
            if let Some(s) = t.named(cat, name) {
                println!("{name} [{cat}] {s:?}");
            }
        }
    }
}
