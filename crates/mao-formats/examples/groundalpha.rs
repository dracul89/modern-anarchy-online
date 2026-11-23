//! `groundalpha <game>`: every ground texture (types 0xf6956, 0xf6965 and 0xf6966) whose picture is not opaque
//! throughout: its instance, type, size and the least and most alpha. The terrain draws its tiles with an alpha test
//! at a half (`terrain.rs`): this says which ground textures that leaves out, in whole or in part.
fn main() {
    let game = std::env::args().nth(1).expect("the game's folder");
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).expect("the database");
    let ids: Vec<mao_formats::resource::Id> = db.ids().expect("the ids").filter_map(Result::ok).filter(|id| matches!(id.kind, 0xf6956 | 0xf6965 | 0xf6966)).collect();
    let (mut opaque, mut other) = (0usize, 0usize);
    for id in ids {
        let Some(data) = db.get(id).ok().flatten() else { continue };
        let Some(start) = [&[0xff, 0xd8, 0xff][..], &[0x89, b'P', b'N', b'G'][..]].iter().filter_map(|sig| data.windows(sig.len()).position(|w| w == *sig)).min().filter(|&at| at < 64) else { continue };
        let Ok(picture) = image::load_from_memory(&data[start..]) else { continue };
        let rgba = picture.to_rgba8();
        let (lo, hi) = rgba.pixels().fold((255u8, 0u8), |(lo, hi), p| (lo.min(p[3]), hi.max(p[3])));
        if lo == 255 {
            opaque += 1;
        } else {
            other += 1;
            let under = rgba.pixels().filter(|p| p[3] < 128).count() as f32 / (rgba.width() * rgba.height()) as f32;
            println!("{:#x} {:#x} {}x{} alpha {lo}..{hi}, under a half {:.0}%", id.instance, id.kind, rgba.width(), rgba.height(), under * 100.0);
        }
    }
    println!("{opaque} opaque, {other} not");
}
