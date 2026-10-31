fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t = mao_formats::ldb::Texts::parse(&std::fs::read(&args[1]).unwrap()).unwrap();
    for name in &args[2..] {
        for cat in [200, 10000, 110, 100, 0, 1] {
            if let Some(s) = t.named(cat, name) {
                println!("{name} [{cat}] {s:?}");
            }
        }
    }
}
