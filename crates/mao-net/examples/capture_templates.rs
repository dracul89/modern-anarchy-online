//! `capture_templates <captures folder> <out.txt> <kind>...`: for each kind, one captured message that parses whole and encodes back
//! to the same bytes (the first of its direction that does), written as `kind direction identity hex` lines. The server's templates
//! are made this way: a captured message of each kind, changed field by field and encoded again.

use std::io::Write;

use mao_net::schema::Schema;

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2).map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap_or(0)).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (folder, out, kinds) = (&args[0], &args[1], &args[2..]);
    let wanted: Vec<u32> = kinds.iter().filter_map(|k| u32::from_str_radix(k, 16).ok()).collect();
    // (direction, identity, data) per kind, the first that round-trips
    let mut found: std::collections::BTreeMap<u32, (String, (u32, u32), Vec<u8>)> = std::collections::BTreeMap::new();
    let mut dirs: Vec<std::path::PathBuf> = std::fs::read_dir(folder).expect("the folder").flatten().map(|e| e.path()).collect();
    dirs.sort();
    for dir in dirs {
        let Ok(text) = std::fs::read_to_string(dir.join("dump.txt")) else { continue };
        for line in text.lines() {
            // "#N ts S>C cX kind KKKKKKKK identity (a,b) LEN bytes: HEX"
            let Some(kind) = line.split(" kind ").nth(1).and_then(|r| u32::from_str_radix(r.split(' ').next().unwrap_or(""), 16).ok()) else { continue };
            if !wanted.contains(&kind) || found.contains_key(&kind) {
                continue;
            }
            let direction = if line.contains(" S>C ") { "S>C" } else { "C>S" };
            let identity = line
                .split("identity (")
                .nth(1)
                .and_then(|r| r.split(')').next())
                .and_then(|r| r.split_once(','))
                .and_then(|(a, b)| Some((u32::from_str_radix(a, 16).ok()?, u32::from_str_radix(b, 16).ok()?)));
            let Some(identity) = identity else { continue };
            let Some(data) = line.rsplit("bytes: ").next().filter(|_| line.contains("bytes: ")).map(|t| hex(t.trim())) else { continue };
            let parsed = match Schema::global().parse_n3(kind, &data) {
                Ok(parsed) => parsed,
                Err(e) => {
                    if std::env::var_os("TEMPLATE_DEBUG").is_some() && wanted.contains(&kind) {
                        eprintln!("{kind:08x}: {e}");
                    }
                    continue;
                }
            };
            if parsed.left != 0 {
                if std::env::var_os("TEMPLATE_DEBUG").is_some() && wanted.contains(&kind) {
                    eprintln!("{kind:08x} {}: {} bytes left over", parsed.name, parsed.left);
                }
                continue;
            }
            let encoded = Schema::global().encode_n3(kind, data[0], &parsed.value);
            // (TEMPLATE_PARSE_ONLY: a message that parses whole is enough, for a layout that re-encodes a little differently)
            let parse_only = std::env::var_os("TEMPLATE_PARSE_ONLY").is_some();
            if !parse_only && encoded.as_ref().ok().map(Vec::as_slice) != Some(&data[..]) {
                // (the first place they differ, to see what a layout gets wrong)
                if std::env::var_os("TEMPLATE_DEBUG").is_some() && wanted.contains(&kind) {
                    match encoded {
                        Ok(bytes) => {
                            let at = bytes.iter().zip(&data).position(|(a, b)| a != b).unwrap_or(bytes.len().min(data.len()));
                            eprintln!("{kind:08x} {}: lengths {} vs {}, first difference at {at}", parsed.name, bytes.len(), data.len());
                        }
                        Err(e) => eprintln!("{kind:08x} {}: {e}", parsed.name),
                    }
                }
                continue;
            }
            found.insert(kind, (direction.to_string(), identity, data));
        }
    }
    let mut file = std::fs::File::create(out).expect("the output");
    for kind in &wanted {
        match found.get(kind) {
            Some((direction, identity, data)) => {
                let name = Schema::global().n3_name(*kind).unwrap_or("?");
                let text: String = data.iter().map(|b| format!("{b:02x}")).collect();
                writeln!(file, "{kind:08x} {direction} {:08x} {:08x} {text}", identity.0, identity.1).unwrap();
                println!("{kind:08x} {name:<28} {direction} identity ({:x},{:x}) {} bytes", identity.0, identity.1, data.len());
            }
            None => println!("{kind:08x} not found"),
        }
    }
}
