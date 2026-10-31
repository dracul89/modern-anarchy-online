//! `ldbnamed <text.mdb> <category> <name>...`: the texts named so in a category.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t = mao_formats::ldb::Texts::parse(&std::fs::read(&args[1]).expect("the text file")).expect("a text database");
    let category: i32 = args[2].parse().unwrap();
    for name in &args[3..] {
        println!("{name}: {:?}", t.named(category, name));
    }
}
