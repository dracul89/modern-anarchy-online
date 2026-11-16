//! `pfsizes <game> <playfield>...`: each playfield's ground in world units (east and north), from its tile-map: what a
//! planet map's maker needs to know where a playfield ends.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    for playfield in args[1..].iter().filter_map(|a| a.parse::<u32>().ok()) {
        let terrain = db.get(Id { kind: 0xf4249, instance: playfield }).ok().flatten().and_then(|o| mao_formats::terrain::build(&mao_formats::tilemap::parse_old(&o).ok()?).ok());
        match terrain {
            Some(t) => println!("{playfield} {} {}", (t.width - 1) as f32 * t.tile_size, (t.height - 1) as f32 * t.tile_size),
            None => println!("{playfield} none"),
        }
    }
}
