//! `mao-client net ...`: the network side.
//!
//! Credentials come from the environment (`MAO_USER`, `MAO_PASSWORD`),
//! never from arguments or files, and are never printed. The server is
//! Rubi-Ka unless `MAO_SERVER` (`host:port`) says otherwise.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use mao_net::frame::{self, Reader};
use mao_net::n3::{self, Message};
use mao_net::session::{self, Connection};
use mao_net::system::{self, Character};

use mao_formats::model::{self, Material, Model, Part, Vertex};
use mao_formats::resource::ResourceDatabase;
use std::sync::Arc;

fn show(list: &[Character]) {
    for c in list {
        println!("{:>10} {:<16} level {:>3} profession {} breed {} sex {} in {:?} (proxy {:x?})", c.id, c.name, c.level, c.profession, c.breed, c.sex, c.area, &c.proxy[..2]);
    }
}

/// The game's version: the first line of `version.id`.
fn version(game: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(Path::new(game).join("version.id")).map_err(|e| format!("version.id: {e}"))?;
    Ok(text.lines().next().unwrap_or("").trim().to_string())
}

fn credentials() -> Result<(String, String), String> {
    let user = std::env::var("MAO_USER").map_err(|_| "set MAO_USER and MAO_PASSWORD")?;
    let password = std::env::var("MAO_PASSWORD").map_err(|_| "set MAO_USER and MAO_PASSWORD")?;
    Ok((user, password))
}

fn login(game: &str) -> Result<(Connection, Vec<Character>), String> {
    let (user, password) = credentials()?;
    let server = std::env::var("MAO_SERVER").unwrap_or_else(|_| session::RUBI_KA.to_string());
    let mut connection = Connection::connect(server.as_str()).map_err(|e| e.to_string())?;
    let list = session::login(&mut connection, &user, &password, &version(game)?).map_err(|e| e.to_string())?;
    Ok((connection, list))
}

/// Tallies of what the world sends.
#[derive(Default)]
struct World {
    own: u32,
    rotations: BTreeMap<u32, [f32; 4]>,
    kinds: BTreeMap<u32, usize>,
    entities: BTreeMap<u32, ([f32; 3], usize)>,
}

impl World {
    fn add(&mut self, frame: &frame::Frame) {
        let Some(m) = Message::parse(frame) else { return };
        *self.kinds.entry(m.kind).or_default() += 1;
        if let Some(q) = m.rotation() {
            self.rotations.insert(m.identity.1, q);
        }
        if let Some((_, q, _)) = m.movement() {
            self.rotations.insert(m.identity.1, q);
        }
        let position = m.position().or_else(|| m.movement().map(|(_, _, p)| p));
        if let Some(p) = position {
            let e = self.entities.entry(m.identity.1).or_insert((p, 0));
            e.0 = p;
            e.1 += 1;
        }
    }

    fn print(&self) {
        println!("N3 messages by type:");
        for (kind, n) in &self.kinds {
            println!("  {kind:#010x} x{n}");
        }
        println!("{} entities with positions:", self.entities.len());
        for (id, (p, _)) in &self.entities {
            println!("  {id:>10} at ({:.1}, {:.1}, {:.1})", p[0], p[1], p[2]);
        }
    }
}


/// A box of 0.6 x 1.8 x 0.6 standing on the origin (Y-up), in `colour`.
fn marker(colour: [f32; 3]) -> Part {
    let (hx, h) = (0.3f32, 1.8f32);
    let mut vertices = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    // Each face: its normal and four corners.
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([0.0, 0.0, 1.0], [[-hx, 0.0, hx], [hx, 0.0, hx], [hx, h, hx], [-hx, h, hx]]),
        ([0.0, 0.0, -1.0], [[hx, 0.0, -hx], [-hx, 0.0, -hx], [-hx, h, -hx], [hx, h, -hx]]),
        ([1.0, 0.0, 0.0], [[hx, 0.0, hx], [hx, 0.0, -hx], [hx, h, -hx], [hx, h, hx]]),
        ([-1.0, 0.0, 0.0], [[-hx, 0.0, -hx], [-hx, 0.0, hx], [-hx, h, hx], [-hx, h, -hx]]),
        ([0.0, 1.0, 0.0], [[-hx, h, hx], [hx, h, hx], [hx, h, -hx], [-hx, h, -hx]]),
        ([0.0, -1.0, 0.0], [[-hx, 0.0, -hx], [hx, 0.0, -hx], [hx, 0.0, hx], [-hx, 0.0, hx]]),
    ];
    for (normal, corners) in faces {
        let base = vertices.len() as u16;
        for c in corners {
            vertices.push(Vertex { position: c, normal, uv: [0.0, 0.0] });
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let material = Material { name: "marker".into(), diffuse: colour, specular: [0.0; 3], ambient: colour, emissive: [0.0; 3], opacity: 1.0, textures: Vec::new() };
    Part { name: "marker".into(), world: model::IDENTITY, vertices, indices, material }
}

/// A small box on the front (+z) of a marker, to show which way it faces.
fn nose() -> Part {
    let mut part = marker([1.0, 0.9, 0.2]);
    for v in &mut part.vertices {
        v.position = [v.position[0] * 0.35, 1.0 + (v.position[1] / 1.8) * 0.4, 0.3 + (v.position[2] + 0.3) * 1.0];
    }
    part
}

/// The entities' markers alone (own character red, the others blue): two
/// parts and an instance per entity, placed Z-up.
fn overlay_model(world: &World) -> Model {
    let mut model = Model { parts: vec![marker([0.9, 0.1, 0.1]), marker([0.2, 0.4, 0.95]), nose()], instances: Vec::new() };
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    for (&id, &(position, _)) in &world.entities {
        let q = world.rotations.get(&id).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let place = model::multiply(&model::trs_matrix(position, q, [1.0; 3]), &swap);
        model.instances.push((usize::from(id != world.own), place));
        model.instances.push((2, place));
    }
    model
}

/// The playfield with a marker at each entity (the character in red, the
/// others in blue).
fn show_world(game: &str, playfield: u32, world: &World, image: Option<&str>) -> Result<(), String> {
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    let mut scene: Model = crate::load_terrain(&db, playfield)?;
    crate::add_props(game, &db, playfield, &mut scene);
    if scene.instances.is_empty() {
        let ground = scene.parts.len();
        scene.instances.extend((0..ground).map(|i| (i, model::IDENTITY)));
    }
    let overlay = overlay_model(world);
    let offset = scene.parts.len();
    scene.parts.extend(overlay.parts);
    scene.instances.extend(overlay.instances.into_iter().map(|(i, m)| (i + offset, m)));
    let textures = crate::Textures::new(db);
    match image {
        Some(out) => crate::viewer::headless(&scene, &textures, out),
        None => crate::viewer::window(scene, textures, None),
    }
}

/// The window with the world updating in it: a thread reads the game
/// server and sends the entities' markers over.
fn live(game: &str, playfield: u32, mut connection: Connection, mut state: World) -> Result<(), String> {
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    let mut scene = crate::load_terrain(&db, playfield)?;
    crate::add_props(game, &db, playfield, &mut scene);
    if scene.instances.is_empty() {
        let ground = scene.parts.len();
        scene.instances.extend((0..ground).map(|i| (i, model::IDENTITY)));
    }
    connection.set_timeout(Duration::from_millis(50)).map_err(|e| e.to_string())?;
    // The first couple of seconds say where the character is.
    let until = Instant::now() + Duration::from_secs(3);
    while Instant::now() < until && !state.entities.contains_key(&state.own) {
        match connection.receive() {
            Ok(frame) => state.add(&frame),
            Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    let focus = state.entities.get(&state.own).map(|&([x, y, z], _)| [x, z, y]);
    let (tx, rx) = std::sync::mpsc::channel();
    let (input_tx, input_rx) = std::sync::mpsc::channel::<crate::viewer::Input>();
    let own = state.own;
    let start = state.entities.get(&own).map(|e| e.0).unwrap_or([0.0; 3]);
    let heading = state.rotations.get(&own).map(|q| 2.0 * q[1].atan2(q[3])).unwrap_or(0.0);
    std::thread::spawn(move || {
        use mao_net::n3::movement;
        const SPEED: f32 = 7.5; // units per second, from the capture
        const BACK_SPEED: f32 = 3.0;
        const TURN: f32 = 2.0; // radians per second
        let mut keys = crate::viewer::Input::default();
        let (mut position, mut heading) = (start, heading);
        let (mut moving, mut turning) = (0i8, 0i8);
        let mut last_tick = Instant::now();
        let mut last_send = Instant::now();
        let mut last_overlay = Instant::now() - Duration::from_secs(1);
        let send = |connection: &mut Connection, kind: u8, heading: f32, position: [f32; 3], since: &mut Instant| {
            let ms = since.elapsed().as_millis().min(u32::MAX as u128) as u32;
            *since = Instant::now();
            connection.send(&n3::position_update(own, kind, heading, position, ms)).is_ok()
        };
        let mut sent = Instant::now();
        loop {
            match connection.receive() {
                Ok(frame) => state.add(&frame),
                Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                Err(e) => {
                    eprintln!("connection ended: {e}");
                    break;
                }
            }
            while let Ok(k) = input_rx.try_recv() {
                keys = k;
            }
            // Walk: integrate the position and tell the server.
            let dt = last_tick.elapsed().as_secs_f32();
            last_tick = Instant::now();
            heading += keys.turn as f32 * TURN * dt;
            let walk = keys.forward as i8 - keys.back as i8;
            let speed = if walk > 0 { SPEED } else { BACK_SPEED } * walk as f32;
            position[0] += heading.sin() * speed * dt;
            position[2] += heading.cos() * speed * dt;
            let mut ok = true;
            let due = last_send.elapsed() >= Duration::from_millis(400);
            let mut send_kind = |kind: u8, connection: &mut Connection| {
                last_send = Instant::now();
                ok &= send(connection, kind, heading, position, &mut sent);
            };
            if walk != moving {
                match (moving, walk) {
                    (_, 1) => send_kind(movement::FORWARD_START, &mut connection),
                    (_, -1) => send_kind(movement::BACKWARD_START, &mut connection),
                    (1, 0) => send_kind(movement::FORWARD_STOP, &mut connection),
                    _ => send_kind(movement::BACKWARD_STOP, &mut connection),
                }
            }
            if keys.turn != turning {
                match keys.turn {
                    -1 => send_kind(movement::TURN_LEFT_START, &mut connection),
                    1 => send_kind(movement::TURN_RIGHT_START, &mut connection),
                    _ => send_kind(movement::TURN_STOP, &mut connection),
                }
            } else if (walk != 0 || keys.turn != 0) && due {
                send_kind(movement::UPDATE, &mut connection);
            }
            if !ok {
                break;
            }
            moving = walk;
            turning = keys.turn;
            state.rotations.insert(own, [0.0, (heading * 0.5).sin(), 0.0, (heading * 0.5).cos()]);
            if walk != 0 || keys.turn != 0 || moving != 0 {
                state.entities.insert(own, (position, 0));
            } else if let Some(e) = state.entities.get(&own) {
                position = e.0;
            }
            if last_overlay.elapsed() >= Duration::from_millis(60) {
                last_overlay = Instant::now();
                let update = crate::viewer::Update { model: overlay_model(&state), own: Some(position) };
                if tx.send(update).is_err() {
                    break;
                }
            }
        }
    });
    crate::viewer::window(scene, crate::Textures::new(db), Some(crate::viewer::Live { overlays: rx, input: input_tx, focus }))
}

pub fn run(args: &[String]) -> Result<(), String> {
    match args {
        [game, word] if word == "chars" => {
            show(&login(game)?.1);
            Ok(())
        }
        [game, word, who, rest @ ..] if word == "enter" => {
            let seconds: u64 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(10);
            let (mut chat, list) = login(game)?;
            let character = list
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(who) || c.id.to_string() == *who)
                .ok_or_else(|| format!("no character {who:?} on the account (try `chars`)"))?;
            eprintln!("logging in {} ({})", character.name, character.id);
            let ticket = session::login_character(&mut chat, character.id).map_err(|e| e.to_string())?;
            eprintln!("game server {}", ticket.host());
            let mut world = session::enter_world(&ticket).map_err(|e| e.to_string())?;
            let mut state = World { own: character.id, ..World::default() };
            if rest.iter().any(|a| a == "--live") {
                return live(game, character.proxy[1], world, state);
            }
            let until = Instant::now() + Duration::from_secs(seconds);
            while Instant::now() < until {
                match world.receive() {
                    Ok(frame) => state.add(&frame),
                    Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => break,
                    Err(e) => return Err(e.to_string()),
                }
            }
            state.own = character.id;
            state.print();
            if let Some(i) = rest.iter().position(|a| a == "--show") {
                let image = rest.get(i + 1).filter(|a| a.as_str() == "--image").and(rest.get(i + 2)).map(String::as_str);
                return show_world(game, character.proxy[1], &state, image);
            }
            Ok(())
        }
        [word, log] if word == "parse-capture" => parse_capture(log),
        [game, word, log, rest @ ..] if word == "capture-map" => {
            let world = parse_capture_world(log)?;
            show_world(game, 540, &world, rest.first().map(String::as_str))
        }
        _ => Err("usage: mao-client net <game dir> chars | net <game dir> enter <name or id> [seconds] | net parse-capture <capture.log> | net <game dir> capture-map <capture.log> [out.png]".into()),
    }
}

/// The server's side of a `tools/capture_proxy.py` log: its messages, the
/// characters of any list among them and the world's traffic.
fn parse_capture(log: &str) -> Result<(), String> {
    parse_capture_world(log)?.print();
    Ok(())
}

fn parse_capture_world(log: &str) -> Result<World, String> {
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let mut readers: BTreeMap<String, Reader> = BTreeMap::new();
    let mut world = World::default();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(tag), Some(connection)) = (f.get(1), f.last()) else { continue };
        match *tag {
            "OPEN" => {
                readers.insert(connection.to_string(), Reader::default());
            }
            "S>C" if f.len() > 4 => {
                let hex = f[3];
                let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()).collect();
                let reader = readers.entry(connection.to_string()).or_default();
                reader.push(&bytes).map_err(|e| e.to_string())?;
                while let Some(m) = reader.next().map_err(|e| e.to_string())? {
                    if m.kind == frame::N3 {
                        world.add(&m);
                        continue;
                    }
                    let kind = system::split(&m).map(|(k, _)| k).unwrap_or(0);
                    println!("{connection} server: kind {} system type {kind:#x}, {} bytes", m.kind, m.body.len());
                    if kind == system::CHARACTER_LIST {
                        let (list, _) = system::character_list(&m).map_err(|e| e.to_string())?;
                        show(&list);
                    } else if kind == system::CHARACTER_TICKET {
                        println!("{:?}", system::ticket(&m).map_err(|e| e.to_string())?.host());
                    }
                }
            }
            _ => {}
        }
    }
    world.own = 598284;
    let _ = n3::CHARACTER;
    Ok(world)
}
