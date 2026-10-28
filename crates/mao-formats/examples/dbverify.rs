//! `dbverify <game> <dbscan.txt>`: each found record read at its position and decoded as a texture: how many decode.
use std::io::BufRead;

fn main() {
    let mut args = std::env::args().skip(1);
    let game = args.next().unwrap();
    let list = args.next().unwrap();
    let db = mao_formats::ctree::Database::open(&std::path::Path::new(&game).join("cd_image/data/db/ResourceDatabase")).unwrap();
    let (mut ok, mut bad, mut sizes) = (0, Vec::new(), Vec::new());
    for line in std::io::BufReader::new(std::fs::File::open(list).unwrap()).lines() {
        let line = line.unwrap();
        let f: Vec<&str> = line.split(' ').collect();
        let (kind, inst) = (u32::from_str_radix(f[0], 16).unwrap(), u32::from_str_radix(f[1], 16).unwrap());
        let pos = u64::from_str_radix(f[2].trim_start_matches("0x"), 16).unwrap();
        let key = mao_formats::ctree::key(kind, inst);
        let good = db.check(&key, pos).is_ok()
            && db.record_at(pos).ok().is_some_and(|r| kind != 0xf6954 || mao_formats::texture::decode(&r[12..]).map(|t| sizes.push((t.width, t.height))).is_ok());
        if good {
            ok += 1;
        } else if bad.len() < 10 {
            bad.push(line.clone());
        }
    }
    sizes.sort();
    sizes.dedup();
    println!("{ok} decode; failing (first): {bad:?}");
    println!("sizes: {:?}", &sizes[..sizes.len().min(20)]);
}
