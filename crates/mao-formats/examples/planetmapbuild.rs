//! `planetmapbuild <maps folder> <folder> <name> <type> <coords.xml> (<picture> <left> <top> <right> <bottom>)...`:
//! makes a planet map of pictures, one level each from the smallest: `<maps folder>/<folder>/<folder>.txt` (the
//! index), `<folder>.xml` (the coordinates file, copied) and `<folder>1.bin`, `<folder>2.bin`... (the tiles). A picture
//! is a PNG, JPEG or BMP, or a Photoshop file (its flattened picture); the four numbers are its `MapRect`. `<type>` is
//! `Rubika` or `Shadowlands`. The client finds the map when `<maps folder>` is its `PlanetMap` folder (beside
//! `mao.ini`) or the game's `cd_image/textures/PlanetMap`.
//!
//! The Atlas of Rubi-Ka's source pictures, with the coordinates measured for them (`assets/planetmaps/AoRK.xml`):
//! `planetmapbuild <maps folder> AoRK "Atlas of Rubi-Ka" Rubika assets/planetmaps/AoRK.xml AoRK_map1.psd 4 1 360 459
//! AoRK_map2.psd 54 3 944 1148 AoRK_map3.psd 8 0 3568 4582 AoRK_map4.psd 8 0 3568 4582`.
use mao_formats::planetmap::{PlanetMap, pack};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 10 || (args.len() - 5) % 5 != 0 {
        eprintln!("planetmapbuild <maps folder> <folder> <name> <type> <coords.xml> (<picture> <left> <top> <right> <bottom>)...");
        std::process::exit(2);
    }
    let (root, folder) = (std::path::PathBuf::from(&args[0]), &args[1]);
    let dir = root.join(folder);
    std::fs::create_dir_all(&dir).expect("the map's folder");
    let mut map = PlanetMap { name: args[2].clone(), kind: args[3].clone(), coords_file: format!("{folder}/{folder}.xml"), ..Default::default() };
    std::fs::copy(&args[4], dir.join(format!("{folder}.xml"))).expect("the coordinates file");
    for (n, level) in args[5..].chunks(5).enumerate() {
        let path = std::path::Path::new(&level[0]);
        let picture = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("psd")) {
            mao_formats::psd::flattened(&std::fs::read(path).expect("the picture")).expect("an 8-bit RGB Photoshop file")
        } else {
            image::open(path).expect("the picture").to_rgba8()
        };
        let (bin, mut l) = pack(&picture, 128).expect("a picture with something in it");
        l.file = format!("{folder}/{folder}{}.bin", n + 1);
        for (i, v) in level[1..].iter().enumerate() {
            l.rect[i] = v.parse().expect("a whole number for the rectangle");
        }
        std::fs::write(dir.join(format!("{folder}{}.bin", n + 1)), &bin).expect("the tiles' file");
        println!("{}: {}x{}, {} by {} tiles, {} bytes, MapRect {:?}", l.file, l.size.0, l.size.1, l.tiles.0, l.tiles.1, bin.len(), l.rect);
        map.levels.push(l);
    }
    std::fs::write(dir.join(format!("{folder}.txt")), map.index_text()).expect("the index");
}
