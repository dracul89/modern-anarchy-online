//! `planetmaps <maps folder>... [--image <index> <level> <out.png>]`: the planet maps found in the folders (their index,
//! name, type and levels), and one level's picture.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let split = args.iter().position(|a| a == "--image").unwrap_or(args.len());
    let roots: Vec<std::path::PathBuf> = args[..split].iter().map(std::path::PathBuf::from).collect();
    for f in mao_formats::planetmap::discover(&roots) {
        let map = mao_formats::planetmap::PlanetMap::open(&f.root, &f.index);
        let levels = map.as_ref().map(|m| m.levels.iter().map(|l| format!("{}x{}", l.size.0, l.size.1)).collect::<Vec<_>>().join(" ")).unwrap_or_default();
        println!("{} | {} | {} | {} places | {levels} | {}", f.index, f.name, f.kind, map.as_ref().map_or(0, |m| m.places.len()), f.root.display());
    }
    if let [_, index, level, out] = &args[split..] {
        let found = mao_formats::planetmap::discover(&roots).into_iter().find(|f| &f.index == index).expect("that index");
        let map = mao_formats::planetmap::PlanetMap::open(&found.root, index).expect("the map");
        let image = map.image(&found.root, level.parse().unwrap()).expect("the level");
        image.save(out).unwrap();
    }
}
