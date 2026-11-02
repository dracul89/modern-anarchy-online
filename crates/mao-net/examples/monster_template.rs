//! `monster_template <hex of a captured monster's entity message>`: prints it with the name, place and health the server sets,
//! so the captured name is not kept (the server's template is made this way).

fn main() {
    let hex = std::env::args().nth(1).expect("the captured message's hex");
    let data: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
    let out = mao_net::n3::set_monster(&data, "Monster", 540, [0.0, 0.0, 0.0], 200, 200).expect("a monster's message");
    let text: String = out.iter().map(|b| format!("{b:02x}")).collect();
    println!("{text}");
}
