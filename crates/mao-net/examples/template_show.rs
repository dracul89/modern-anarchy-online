//! `template_show <templates.txt>`: prints each captured template's parsed fields (the server's message layouts, by name).

use mao_net::schema::Schema;

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2).map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap_or(0)).collect()
}

fn main() {
    let path = std::env::args().nth(1).expect("the templates file");
    for line in std::fs::read_to_string(path).expect("the file").lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 5 {
            continue;
        }
        let kind = u32::from_str_radix(parts[0], 16).unwrap_or(0);
        let data = hex(parts[4]);
        match Schema::global().parse_n3(kind, &data) {
            Ok(p) => {
                println!("== {kind:08x} {} ({} {}, left {})\n{}", p.name, parts[1], parts[2..4].join(" "), p.left, p.value.pretty());
            }
            Err(e) => println!("== {kind:08x}: {e}"),
        }
    }
}
