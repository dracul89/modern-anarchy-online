//! `ldbget <text.mdb> <category> <key>...`: the texts at those keys of a category.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t = mao_formats::ldb::Texts::parse(&std::fs::read(&args[1]).expect("the text file")).expect("a text database");
    let category: i32 = args[2].parse().unwrap();
    for key in &args[3..] {
        let k: i32 = key.parse().unwrap();
        println!("{category}:{k} {:?}", t.get(category, k));
    }
}
