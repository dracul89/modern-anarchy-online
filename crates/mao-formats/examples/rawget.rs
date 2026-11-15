//! `rawget <game> <kind hex> <instance hex> <out file>`: one record of the resource database, its bytes as stored
//! (a texture is a JPEG or a PNG after its header: the file is written from the picture's signature on when one is found).
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let kind = u32::from_str_radix(args[1].trim_start_matches("0x"), 16).unwrap();
    let instance = u32::from_str_radix(args[2].trim_start_matches("0x"), 16).unwrap();
    let Some(data) = db.get(mao_formats::resource::Id { kind, instance }).ok().flatten() else {
        println!("no such record");
        return;
    };
    let start = [&[0xff, 0xd8, 0xff][..], &[0x89, b'P', b'N', b'G'][..]].iter().filter_map(|sig| data.windows(sig.len()).position(|w| w == *sig)).min().filter(|&at| at < 64).unwrap_or(0);
    println!("{} bytes, written from {start}; the first: {:02x?}", data.len(), &data[..data.len().min(32)]);
    std::fs::write(&args[3], &data[start..]).unwrap();
}
