//! `placed <game> <mesh instance>... [limit]`: where the playfields' statels place a mesh: the playfield and the statel's
//! position (game coordinates), up to `limit` a playfield (3 unless the last word is `all`).
use mao_formats::resource::ResourceDatabase;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let all = args.last().is_some_and(|a| a == "all");
    if all {
        args.pop();
    }
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let wanted: Vec<u32> = args[1..].iter().map(|a| a.parse().expect("a mesh instance (decimal)")).collect();
    for p in mao_formats::playfield::all(&db) {
        let path = std::path::Path::new(&args[0]).join(format!("cd_image/data/statels/{}.pf", p.number));
        let Some(list) = mao_formats::files::read(&path).ok().and_then(|file| mao_formats::statels::parse(&file).ok()) else { continue };
        let hits: Vec<_> = list.iter().filter(|s| wanted.contains(&s.mesh)).collect();
        if hits.is_empty() {
            continue;
        }
        println!("{} {}: {}", p.number, p.name.trim(), hits.len());
        for s in hits.iter().take(if all { usize::MAX } else { 3 }) {
            println!("  mesh {} at ({:.1}, {:.1}, {:.1}) rot {:.3?} scale {:.2?}", s.mesh, s.position[0], s.position[1], s.position[2], s.rotation, s.scale_xyz);
        }
    }
}
