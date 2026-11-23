//! `areafog <game> <playfield>...`: each playfield's own environment and its areas' (name, centre, radius, ambient light,
//! fog colour as bytes, fog density, whether it takes part in the blend).
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    for pf in &args[1..] {
        let number: u32 = pf.parse().unwrap();
        let base = mao_formats::playfield::load_anarchy(&db, number).and_then(|a| a.environment);
        println!("{pf}: playfield environment {}", base.map_or("none".to_string(), |e| format!("ambient {:?} fog {:?} density {} weather {:?}", &e[19..22], &e[22..25], e[25], &e[..8])));
        for a in mao_formats::areas::load(&db, number) {
            let e = a.environment;
            println!("  {:<32} at ({:.0}, {:.0}, {:.0}) radius {:.0}: ambient {:?} fog {:?} density {} takes part {}", a.name, a.centre[0], a.centre[1], a.centre[2], a.radius, &e[19..22], &e[22..25], e[25], a.takes_part());
        }
    }
}
