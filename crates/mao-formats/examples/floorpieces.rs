//! `floorpieces <game> <playfield>...`: for each cell type of a room playfield's tile-map, the piece its table gives a cell
//! with no wall (`FUN_10016967(type, type, 0)`, the piece `DungeonMesh_t` takes a floor from) and that piece's tiles; then
//! how many of the rooms' cells have a piece of their own and how many have none.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    for playfield in args[1..].iter().map(|a| a.parse::<u32>().unwrap()) {
        let Some(rooms) = db.get(Id { kind: mao_formats::rooms::KIND, instance: playfield }).ok().flatten().and_then(|o| mao_formats::rooms::parse(&o).ok().flatten()) else {
            println!("{playfield}: no rooms");
            continue;
        };
        let map = mao_formats::tilemap::parse_new(&db.get(Id { kind: 0xf4249, instance: rooms.tilemap }).unwrap().unwrap()).unwrap();
        let grid = mao_formats::rooms::Grid::new(&map).unwrap();
        println!("{playfield} \"{}\": tile-map {}, {} cell types {:?}", rooms.name, rooms.tilemap, map.tile_types.len(), map.tile_types);
        for place in 0..map.tile_types.len().min(32) as u8 {
            let (piece, turn) = grid.tables.piece(place, place, 0);
            let tiles = db.get(Id { kind: mao_formats::dungeon::PIECE, instance: piece as u32 }).ok().flatten().and_then(|o| mao_formats::dungeon::parse_piece(&o)).map(|p| p.tiles.iter().map(|t| t.vertices.len()).collect::<Vec<_>>());
            println!("  type {place}: the no-wall piece {piece} (turn {turn}), its tiles' vertices {tiles:?}");
        }
        let mut counts: std::collections::BTreeMap<(u8, bool), usize> = std::collections::BTreeMap::new();
        for room in &rooms.rooms {
            let [x0, y0, x1, y1] = room.rect.map(|v| v as usize);
            for y in y0..y1 {
                for x in x0..x1 {
                    let place = grid.cells[y * grid.width + x] & 0x7f;
                    if place != 0 {
                        *counts.entry((place, grid.words.piece(x, y).0 != 0)).or_default() += 1;
                    }
                }
            }
        }
        println!("  cells by (type, has a piece): {counts:?}");
        // (the floors' heights room by room, as the vehicle stands on them)
        for (k, room) in rooms.rooms.iter().enumerate() {
            let one = mao_formats::rooms::RoomPlayfield { rooms: vec![room.clone()], ..rooms.clone() };
            let mut heights: std::collections::BTreeMap<i32, usize> = std::collections::BTreeMap::new();
            for t in mao_formats::rooms::floor_triangles(&grid, &one) {
                for p in t {
                    *heights.entry((p[1] * 100.0).round() as i32).or_default() += 1;
                }
            }
            println!("  room {k} \"{}\" at height {}: floor corner heights (cm: count) {heights:?}", room.name, room.position[1]);
            // (and as they are drawn: the parts without pieces are the floor's quads)
            let mut drawn: std::collections::BTreeMap<(Option<u32>, i32), usize> = std::collections::BTreeMap::new();
            for part in mao_formats::rooms::room_parts(&grid, room, &Default::default(), &Default::default()) {
                for v in &part.vertices {
                    *drawn.entry((part.material.textures.first().map(|t| t.1), (v.position[2] * 100.0).round() as i32)).or_default() += 1;
                }
            }
            println!("    drawn floor vertices by (texture, height cm): {drawn:?}");
        }
    }
}
