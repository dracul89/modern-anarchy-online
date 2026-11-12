//! `roomcells <game> <playfield> <room>`: a room's cells, their pieces and the textures their tiles take
//! (`rooms::describe_room`).
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let (playfield, room): (u32, usize) = (args[1].parse().unwrap(), args[2].parse().unwrap());
    let rooms = mao_formats::rooms::parse(&db.get(Id { kind: mao_formats::rooms::KIND, instance: playfield }).unwrap().unwrap()).unwrap().unwrap();
    let map = mao_formats::tilemap::parse_new(&db.get(Id { kind: 0xf4249, instance: rooms.tilemap }).unwrap().unwrap()).unwrap();
    let grid = mao_formats::rooms::Grid::new(&map).unwrap();
    let mut pieces = std::collections::HashMap::new();
    for id in grid.pieces_needed(&rooms) {
        if let Some(piece) = db.get(Id { kind: mao_formats::dungeon::PIECE, instance: id as u32 }).ok().flatten().and_then(|o| mao_formats::dungeon::parse_piece(&o)) {
            pieces.insert(id, piece);
        }
    }
    for line in mao_formats::rooms::describe_room(&grid, &rooms.rooms[room], &pieces) {
        println!("{line}");
    }
}
