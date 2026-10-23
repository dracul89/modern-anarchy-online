//! `mao-client net ...`: the network side.
//!
//! Credentials come from the environment (`MAO_USER`, `MAO_PASSWORD`),
//! never from arguments or files, and are never printed. The server is
//! Rubi-Ka unless `MAO_SERVER` (`host:port`) says otherwise.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use mao_net::frame::{self, Reader};
use mao_net::inventory;
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

/// Follow a 0x3c redirect (entering a shop and the like): log in again at the
/// default server (the redirect's address if that fails), ask for the same
/// character and enter the world there.
fn hop(game: &str, target: &str, character: u32) -> Result<Connection, String> {
    let (user, password) = credentials()?;
    let version = version(game)?;
    let default = std::env::var("MAO_SERVER").unwrap_or_else(|_| session::RUBI_KA.to_string());
    let mut last = String::new();
    // (The address in a redirect is not a login server: it closes the connection. The normal
    // login hands out a ticket for the right game server; the redirect address is the fallback.)
    for server in [default.as_str(), target] {
        let attempt = (|| -> Result<Connection, String> {
            let mut chat = Connection::connect(server).map_err(|e| e.to_string())?;
            session::login(&mut chat, &user, &password, &version).map_err(|e| e.to_string())?;
            let ticket = session::login_character(&mut chat, character).map_err(|e| e.to_string())?;
            eprintln!("game server {} (login at {server})", ticket.host());
            session::enter_world(&ticket).map_err(|e| e.to_string())
        })();
        match attempt {
            Ok(connection) => return Ok(connection),
            Err(e) => {
                eprintln!("login at {server} failed: {e}");
                last = e;
            }
        }
    }
    Err(last)
}

/// `MAO_TRACE=1`: print the server messages that matter for zoning and
/// the like (not the bulk of entity and position traffic), with the time.
fn trace(frame: &frame::Frame, clock: Instant) {
    const BULK: [u32; 12] = [0x54111123, 0x2b333d6e, 0x271b3a6b, 0x3b1d2268, 0x36510078, 0x4c7d403b, 0x3b11256f, 0x46002f16, 0x465a5d73, 0x260f3671, 0x195e496e, 0x1d3c0f1c];
    let t = clock.elapsed().as_secs_f32();
    if let Some(m) = Message::parse(frame) {
        if !BULK.contains(&m.kind) {
            let head: String = m.data.iter().take(28).map(|b| format!("{b:02x}")).collect();
            eprintln!("[{t:7.2}] server n3 {:08x} from {:x}:{:x} ({} bytes) {head}", m.kind, m.identity.0, m.identity.1, m.data.len());
        }
    } else if frame.kind != frame::PING {
        let head: String = frame.body.iter().take(28).map(|b| format!("{b:02x}")).collect();
        eprintln!("[{t:7.2}] server frame kind {} ({} bytes) {head}", frame.kind, frame.body.len());
    }
}

/// Tallies of what the world sends.
#[derive(Default)]
struct World {
    own: u32,
    rotations: BTreeMap<u32, [f32; 4]>,
    /// What the entity messages say about each: (name, is a player).
    names: BTreeMap<u32, (String, bool)>,
    infos: BTreeMap<u32, mao_net::n3::EntityInfo>,
    kinds: BTreeMap<u32, usize>,
    entities: BTreeMap<u32, ([f32; 3], usize)>,
    /// When each entity last changed position (for the walk cycle).
    last_move: BTreeMap<u32, Instant>,
    /// When each was last heard of and its speed in units a second.
    last_seen: BTreeMap<u32, Instant>,
    speeds: BTreeMap<u32, f32>,
    /// A playfield announcement not yet acted on: (playfield, arrival).
    arrival: Option<(u32, [f32; 3])>,
    /// The character's items, from the world-entry message (until taken).
    inventory: Option<Vec<inventory::Entry>>,
    /// The identity type each entity was first heard of with (what a use message names it by).
    types: BTreeMap<u32, u32>,
    /// The vending machines (booths: shops, and the mission booths' companions) with their positions.
    vendors: BTreeMap<u32, [f32; 3]>,
    /// The travel list of the playfield, from the server (until taken).
    destinations: Option<mao_net::destinations::DestinationList>,
    /// The usable objects the server announces (doors, mail terminals, booths: identity type
    /// 0xc76a, message `4f474e05`) with their positions; targeted with Ctrl+Tab and used with E.
    objects: BTreeMap<u32, [f32; 3]>,
}

/// The server's announcement of a usable object, and the identity type of those objects.
const OBJECT_ANNOUNCE: u32 = 0x4f474e05;
const OBJECT_TYPE: u32 = 0xc76a;

impl World {
    /// Where an entity or usable object is.
    fn position_of(&self, id: u32) -> Option<[f32; 3]> {
        self.entities.get(&id).map(|e| e.0).or_else(|| self.objects.get(&id).copied())
    }

    fn add(&mut self, frame: &frame::Frame) {
        let Some(m) = Message::parse(frame) else { return };
        if let Some(arrival) = m.arrival() {
            self.arrival = Some(arrival);
        }
        *self.kinds.entry(m.kind).or_default() += 1;
        if m.kind == 0x7f544905 && m.identity.0 == 0xc75b {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                if let Some(pos) = p.value.get("Position") {
                    let f = |n: &str| match pos.get(n) {
                        Some(mao_net::schema::Value::Float(x)) => *x as f32,
                        _ => 0.0,
                    };
                    self.vendors.insert(m.identity.1, [f("X"), f("Y"), f("Z")]);
                }
            }
        }
        if m.kind == mao_net::destinations::DESTINATION_LIST {
            self.destinations = mao_net::destinations::parse(&m.data);
        }
        if m.kind == OBJECT_ANNOUNCE && m.identity.0 == OBJECT_TYPE && m.data.len() >= 29 {
            // The position follows 17 bytes of header (the capture's objects stand where a
            // terminal or door of that playfield does).
            let f = |at: usize| f32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            self.objects.insert(m.identity.1, [f(17), f(21), f(25)]);
            self.names.entry(m.identity.1).or_insert_with(|| (format!("Object {:x}", m.identity.1 & 0xffff), false));
        }
        if m.identity.0 != 0 {
            self.types.entry(m.identity.1).or_insert(m.identity.0);
        }
        if m.kind == inventory::WORLD_ENTRY && m.identity.1 == self.own {
            self.inventory = Some(inventory::parse(&m.data));
        }
        if let Some(q) = m.rotation() {
            self.rotations.insert(m.identity.1, q);
        }
        if let Some(info) = m.entity_info() {
            let player = info.is_player();
            self.names.insert(m.identity.1, (info.name.clone(), player));
            self.infos.insert(m.identity.1, info);
        }
        if let Some((_, q, _)) = m.movement() {
            self.rotations.insert(m.identity.1, q);
        }
        let position = m.position().or_else(|| m.movement().map(|(_, _, p)| p));
        if let Some(p) = position {
            let e = self.entities.entry(m.identity.1).or_insert((p, 0));
            let moved = ((e.0[0] - p[0]).powi(2) + (e.0[2] - p[2]).powi(2)).sqrt();
            if moved > 0.05 {
                self.last_move.insert(m.identity.1, Instant::now());
                if let Some(before) = self.last_seen.get(&m.identity.1) {
                    let speed = moved / before.elapsed().as_secs_f32().max(0.05);
                    let old = self.speeds.get(&m.identity.1).copied().unwrap_or(speed);
                    self.speeds.insert(m.identity.1, 0.6 * old + 0.4 * speed);
                }
            }
            self.last_seen.insert(m.identity.1, Instant::now());
            e.0 = p;
            e.1 += 1;
        }
    }

    fn print(&self) {
        println!("N3 messages by type:");
        for (kind, n) in &self.kinds {
            println!("  {kind:#010x} x{n}");
        }
        for (id, i) in &self.infos {
            println!("  info {id:>10} {:<18} flags {:#010x} flags2 {:#010x} breed {} sex {} a0 {:?} textures {} clothes {:?}", i.name, i.flags, i.flags2, i.breed, i.sex, i.unknown_a0, i.textures.len(), i.clothes);
        }
        println!("{} entities with positions:", self.entities.len());
        for (id, (p, _)) in &self.entities {
            let (name, player) = self.names.get(id).map(|(n, p)| (n.as_str(), *p)).unwrap_or(("?", false));
            println!("  {id:>10} {name:<20} {} type {:#x} at ({:.1}, {:.1}, {:.1})", if player { "player" } else { "      " }, self.types.get(id).copied().unwrap_or(0), p[0], p[1], p[2]);
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
    let material = Material { name: "marker".into(), diffuse: colour, specular: [0.0; 3], ambient: colour, emissive: [0.0; 3], opacity: 1.0, textures: Vec::new(), ignore_alpha: false, foliage: false };
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

/// The players' models: bodies and heads by breed and sex (found by name
/// in the instance index), posed by a walk cycle.
struct Characters {
    db: Arc<ResourceDatabase>,
    index: mao_formats::instances::Instances,
    /// Animations by file name, with their lengths.
    anims: std::collections::HashMap<String, Option<(mao_formats::catanim::CatAnim, f32)>>,
    /// By (breed, sex): the body, its head and the head's attractor.
    bodies: std::collections::HashMap<(u8, u8), Option<(mao_formats::catmesh::CatMesh, Model, Option<mao_formats::catmesh::Attractor>)>>,
    /// Creatures by NPC template: the template and its CAT mesh.
    creatures: std::collections::HashMap<u32, Option<(mao_formats::npc::Template, mao_formats::catmesh::CatMesh, Option<Model>)>>,
    /// Creature animations by instance.
    creature_anims: std::collections::HashMap<u32, Option<(mao_formats::catanim::CatAnim, f32)>>,
    started: Instant,
}

impl Characters {
    fn new(db: Arc<ResourceDatabase>) -> Result<Characters, String> {
        use mao_formats::resource::Id;
        let get = |kind: u32, instance: u32| db.get(Id { kind, instance }).map_err(|e| e.to_string())?.ok_or_else(|| format!("no object {kind:#x}:{instance:#x}"));
        let index = mao_formats::instances::Instances::parse(&get(0xf424a, 1)?).map_err(|e| e.to_string())?;
        let _ = get;
        Ok(Characters { db, index, anims: Default::default(), bodies: Default::default(), creatures: Default::default(), creature_anims: Default::default(), started: Instant::now() })
    }

    /// The parts of a creature or NPC from its template (the CAT mesh and
    /// animations its record names), in Y-up local space.
    fn creature_parts(&mut self, template: u32, action: mao_formats::appearance::Action, phase: f32, textures: &[(String, u32)], clothes: &[(String, u32)]) -> Option<Vec<Part>> {
        use mao_formats::resource::Id;
        let db = self.db.clone();
        let entry = self.creatures.entry(template).or_insert_with(|| {
            let record = mao_formats::npc::Template::parse(&db.get(Id { kind: mao_formats::npc::TEMPLATE, instance: template }).ok()??)?;
            let mesh = mao_formats::catmesh::parse(&db.get(Id { kind: 0xf6952, instance: record.mesh? }).ok()??).ok()?;
            // A humanoid's head is a separate mesh on the body's head attractor.
            let head = record.head.filter(|&h| h != 0).and_then(|h| crate::load_model(&db, Id { kind: 0xf6951, instance: h }).ok());
            Some((record, mesh, head))
        });
        let (record, mesh, head) = entry.as_ref()?;
        let id = record.animation(action).or_else(|| record.animation(mao_formats::appearance::Action::Idle))?;
        let anim_entry = self.creature_anims.entry(id).or_insert_with(|| {
            let anim = mao_formats::catanim::parse(&db.get(Id { kind: 0xf6953, instance: id }).ok()??).ok()?;
            let length = anim.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0)).fold(1.0f32, f32::max);
            Some((anim, length))
        });
        let (anim, length) = anim_entry.as_ref()?;
        let t = (self.started.elapsed().as_secs_f32() * 1000.0 + phase) % length;
        let bones = mesh.pose(anim, t);
        let mut parts = mesh.skinned_model(&bones, 0xf6954).parts;
        for part in &mut parts {
            let wanted = clothes.iter().chain(textures.iter()).find(|(n, i)| *i != 0 && n.eq_ignore_ascii_case(&part.material.name));
            if let Some((_, instance)) = wanted {
                part.material.textures = vec![(0xf6954, *instance)];
            }
        }
        if let (Some(head), Some(attractor)) = (head, mesh.attractor("head")) {
            parts.extend(mesh.attach(&bones, attractor, head));
        }
        for p in &mut parts {
            p.world = model::IDENTITY;
        }
        Some(parts)
    }

    /// The parts of a player of this breed and sex at `t` ms into the walk,
    /// in Y-up local space (world matrices identity), if its meshes exist.
    fn parts(&mut self, breed: u8, sex: u8, action: mao_formats::appearance::Action, phase: f32, textures: &[(String, u32)], clothes: &[(String, u32)]) -> Option<Vec<Part>> {
        use mao_formats::resource::Id;
        let db = self.db.clone();
        let index = &self.index;
        let entry = self.bodies.entry((breed, sex)).or_insert_with(|| {
            let (body_name, head_name) = mao_formats::appearance::player_meshes(breed, sex)?;
            let body = mao_formats::catmesh::parse(&db.get(Id { kind: 0xf6952, instance: index.find(0xf6952, body_name)? }).ok()??).ok()?;
            let head = crate::load_model(&db, Id { kind: 0xf6951, instance: index.find(0xf6951, head_name)? }).ok()?;
            let attractor = body.attractor("head").cloned();
            Some((body, head, attractor))
        });
        let (body, head, attractor) = entry.as_ref()?;
        let name = mao_formats::appearance::animation_name(breed, sex, action);
        let anim_entry = self.anims.entry(name.clone()).or_insert_with(|| {
            let id = index.find(0xf6953, &name)?;
            let anim = mao_formats::catanim::parse(&db.get(Id { kind: 0xf6953, instance: id }).ok()??).ok()?;
            let length = anim.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0)).fold(1.0f32, f32::max);
            Some((anim, length))
        });
        let (anim, length) = anim_entry.as_ref()?;
        let t = (self.started.elapsed().as_secs_f32() * 1000.0 + phase) % length;
        let bones = body.pose(anim, t);
        let mut model = body.skinned_model(&bones, 0xf6954);
        // The character's own textures (its clothes) replace the mesh's by material name.
        for part in &mut model.parts {
            // Worn clothes win over the base textures.
            let wanted = clothes.iter().chain(textures.iter()).find(|(n, i)| *i != 0 && n.eq_ignore_ascii_case(&part.material.name));
            if let Some((_, instance)) = wanted {
                part.material.textures = vec![(0xf6954, *instance)];
            }
        }
        if let Some(a) = attractor {
            model.parts.extend(body.attach(&bones, a, head));
        }
        let mut parts = model.parts;
        for p in &mut parts {
            p.world = model::IDENTITY;
        }
        Some(parts)
    }
}

/// The entities' markers alone (own character red, the others blue): two
/// parts and an instance per entity, placed Z-up.
fn overlay_model(world: &World, characters: &mut Option<Characters>) -> Model {
    let mut model = Model { parts: vec![marker([0.9, 0.1, 0.1]), marker([0.2, 0.4, 0.95]), nose(), marker([0.2, 0.8, 0.3])], instances: Vec::new(), water: None };
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    for (&id, &(position, _)) in &world.entities {
        let q = world.rotations.get(&id).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let place = model::multiply(&model::trs_matrix(position, q, [1.0; 3]), &swap);
        if let (Some(info), Some(chars)) = (world.infos.get(&id).filter(|i| i.template.is_some()), characters.as_mut()) {
            let moving = world.last_move.get(&id).is_some_and(|t| t.elapsed() < Duration::from_millis(900));
            let action = if moving { mao_formats::appearance::Action::Walk } else { mao_formats::appearance::Action::Idle };
            if let Some(parts) = chars.creature_parts(info.template.unwrap(), action, (id % 997) as f32 * 3.0, &info.textures, &info.clothes) {
                let base = model.parts.len();
                for (k, p) in parts.into_iter().enumerate() {
                    model.parts.push(p);
                    model.instances.push((base + k, place));
                }
                continue;
            }
        }
        let info = world.infos.get(&id).filter(|i| i.is_humanoid());
        if let (Some(info), Some(chars)) = (info, characters.as_mut()) {
            // The walk cycle runs while it moves (and a second after), else the first frame.
            let moving = world.last_move.get(&id).is_some_and(|t| t.elapsed() < Duration::from_millis(900));
            let speed = world.speeds.get(&id).copied().unwrap_or(0.0);
            let action = if !moving { mao_formats::appearance::Action::Idle } else if speed > 5.5 { mao_formats::appearance::Action::Run } else { mao_formats::appearance::Action::Walk };
            // (A phase per entity keeps a crowd from moving in step.)
            if let Some(parts) = chars.parts(info.breed, info.sex, action, (id % 997) as f32 * 3.0, &info.textures, &info.clothes) {
                let base = model.parts.len();
                for (k, p) in parts.into_iter().enumerate() {
                    model.parts.push(p);
                    model.instances.push((base + k, place));
                }
                continue;
            }
        }
        let player = world.names.get(&id).is_some_and(|n| n.1);
        model.instances.push((if id == world.own { 0 } else if player { 3 } else { 1 }, place));
        model.instances.push((2, place));
    }
    model
}

/// The playfield with a marker at each entity (the character in red, the
/// others in blue).
fn show_world(game: &str, playfield: u32, world: &World, image: Option<&str>) -> Result<(), String> {
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    let mut scene: Model = crate::load_terrain(&db, playfield)?;
    let part_mesh = crate::add_props(game, &db, playfield, &mut scene);
    if scene.instances.is_empty() {
        let ground = scene.parts.len();
        scene.instances.extend((0..ground).map(|i| (i, model::IDENTITY)));
    }
    if let Ok(t) = crate::load_terrain_data(&db, playfield) {
        let started = Instant::now();
        let ground_parts = crate::load_terrain(&db, playfield).map(|m| m.parts.len()).unwrap_or(0);
        let collision = build_collision(&scene, ground_parts, &crate::Textures::new(db.clone()), &passable_parts(&db, &part_mesh));
        eprintln!("collision: {} triangles in {:?}", collision.len(), started.elapsed());
        for (id, (p, _)) in world.entities.iter().take(6) {
            eprintln!("entity {id}: server y {:.2}, ground model {:.2}", p[1], t.height_at(p[0], p[2], crate::HEIGHT_SCALE));
        }
    }
    let mut characters = Characters::new(db.clone()).ok();
    let overlay = overlay_model(world, &mut characters);
    let offset = scene.parts.len();
    scene.parts.extend(overlay.parts);
    scene.instances.extend(overlay.instances.into_iter().map(|(i, m)| (i + offset, m)));
    let textures = crate::Textures::new(db);
    match image {
        Some(out) => crate::viewer::headless(&scene, &textures, out),
        None => crate::viewer::window(scene, textures, None),
    }
}

/// What blocks a walking character: the props' opaque triangles (a part
/// whose texture has see-through pixels, such as foliage, is left out).
fn build_collision(scene: &Model, ground_parts: usize, textures: &crate::Textures, passable: &std::collections::HashSet<usize>) -> mao_formats::collision::Collision {
    let mut texture_opaque: std::collections::HashMap<(u32, u32), bool> = std::collections::HashMap::new();
    let mut part_opaque: std::collections::HashMap<usize, bool> = std::collections::HashMap::new();
    let mut collision = mao_formats::collision::Collision::default();
    for &(i, m) in &scene.instances {
        if i < ground_parts || passable.contains(&i) {
            continue;
        }
        let part = &scene.parts[i];
        let opaque = *part_opaque.entry(i).or_insert_with(|| {
            part.material.opacity >= 0.99
                && part.material.textures.first().is_none_or(|&(kind, instance)| {
                    *texture_opaque.entry((kind, instance)).or_insert_with(|| textures.get(kind, instance).is_none_or(|t| t.rgba.chunks_exact(4).all(|p| p[3] >= 250)))
                })
        });
        if !opaque {
            continue;
        }
        let w = model::multiply(&part.world, &m);
        let at = |k: u16| {
            let p = part.vertices[k as usize].position;
            [p[0] * w[0] + p[1] * w[4] + p[2] * w[8] + w[12], p[0] * w[1] + p[1] * w[5] + p[2] * w[9] + w[13], p[0] * w[2] + p[1] * w[6] + p[2] * w[10] + w[14]]
        };
        for t in part.indices.chunks_exact(3) {
            collision.add([at(t[0]), at(t[1]), at(t[2])]);
        }
    }
    collision
}

/// A map that can be emptied (to clear several of different types at once).
trait ClearMap {
    fn clear_all(&mut self);
}

impl<V> ClearMap for BTreeMap<u32, V> {
    fn clear_all(&mut self) {
        self.clear();
    }
}

/// The prop parts that are not solid: the meshes whose names say they are
/// whompah entrances, doors or gates (the server zones a character that
/// walks into them).
fn passable_parts(db: &ResourceDatabase, part_mesh: &std::collections::HashMap<usize, u32>) -> std::collections::HashSet<usize> {
    let object = db.get(mao_formats::resource::Id { kind: 0xf424a, instance: 1 }).ok().flatten();
    let Some(index) = object.and_then(|o| mao_formats::instances::Instances::parse(&o).ok()) else { return Default::default() };
    let names: std::collections::HashMap<u32, String> = index.iter().filter(|(kind, _, _)| *kind == 0xf6951).map(|(_, name, instance)| (instance, name.to_lowercase())).collect();
    part_mesh
        .iter()
        .filter(|(_, mesh)| names.get(mesh).is_some_and(|n| n.contains("whompa_entrance") || n.contains("_door") || n.contains("wallgate")))
        .map(|(&part, _)| part)
        .collect()
}

/// A playfield loaded for walking in: what to draw, the ground's heights (when
/// its tile-map is of a layout we can read) and what blocks.
struct Area {
    scene: Model,
    terrain: Option<mao_formats::terrain::Terrain>,
    collision: mao_formats::collision::Collision,
    /// The whompah booths (x, height, z).
    booths: Vec<[f32; 3]>,
}

fn load_area(game: &str, db: &Arc<ResourceDatabase>, playfield: u32) -> Result<Area, String> {
    // An interior has no ground at all (only props).
    let mut scene = crate::load_terrain(db, playfield).unwrap_or_default();
    let terrain = crate::load_terrain_data(db, playfield).ok();
    let ground_parts = scene.parts.len();
    let part_mesh = crate::add_props(game, db, playfield, &mut scene);
    if scene.instances.is_empty() {
        scene.instances.extend((0..ground_parts).map(|i| (i, model::IDENTITY)));
    }
    // Trigger areas are walked into, not blocked by: whompah entrances, doors and gates.
    let passable = passable_parts(db, &part_mesh);
    let collision = build_collision(&scene, ground_parts, &crate::Textures::new(db.clone()), &passable);
    eprintln!("playfield {playfield}: collision {} triangles", collision.len());
    let booths = crate::whompah_booths(game, db, playfield);
    Ok(Area { scene, terrain, collision, booths })
}

/// The label over each whompah booth: where it leads, when that is known.
pub(crate) fn booth_labels(playfield: u32, booths: &[[f32; 3]], destinations: &crate::whompah::Destinations, names: &std::collections::HashMap<u32, String>) -> Vec<([f32; 3], String)> {
    booths
        .iter()
        .map(|b| {
            let text = match destinations.get(playfield, b[0], b[2]) {
                Some(to) => format!("Whom-Pah  >  {}", names.get(&to).cloned().unwrap_or_else(|| format!("playfield {to}"))),
                None => "Whom-Pah".to_string(),
            };
            // Over the booth, Z-up.
            ([b[0], b[2], b[1] + 3.4], text)
        })
        .collect()
}

/// The window with the world updating in it: a thread reads the game
/// server and sends the entities' markers over.
fn live(game: &str, playfield: u32, mut connection: Connection, mut state: World) -> Result<(), String> {
    crate::theme::set_game_dir(game);
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    let Area { scene, terrain, collision, booths } = load_area(game, &db, playfield)?;
    let (mut terrain, mut collision) = (terrain, collision);
    // The interface: the names of the playfields, and where the booths lead.
    let names: std::collections::HashMap<u32, String> = mao_formats::playfield::all(&db).into_iter().map(|p| (p.number, p.name.trim().to_string())).collect();
    let mut destinations = crate::whompah::Destinations::load();
    let mut booths = booths;
    let mut booth_playfield = playfield;
    let mut pending_labels = Some(booth_labels(playfield, &booths, &destinations, &names));
    let mut pending_location = names.get(&playfield).cloned();
    let mut pending_messages: Vec<String> = Vec::new();
    // The booth the character stood at last, and when.
    let mut near_booth: Option<(u32, [f32; 3], Instant)> = None;
    let game_dir = game.to_string();
    let thread_db = db.clone();
    let mut current = playfield;
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
    // The world has loaded: now the server may move us.
    connection.send(&n3::ready(state.own)).map_err(|e| e.to_string())?;
    connection.send(&n3::action(state.own, n3::STAND)).map_err(|e| e.to_string())?;
    let focus = state.entities.get(&state.own).map(|&([x, y, z], _)| [x, z, y]);
    let mut characters = Characters::new(db.clone()).ok();
    let (tx, rx) = std::sync::mpsc::channel();
    let (input_tx, input_rx) = std::sync::mpsc::channel::<crate::viewer::Input>();
    let own = state.own;
    let start = state.entities.get(&own).map(|e| e.0).unwrap_or([0.0; 3]);
    let heading = state.rotations.get(&own).map(|q| 2.0 * q[1].atan2(q[3])).unwrap_or(0.0);
    let (gear_db, gear_textures) = (db.clone(), crate::Textures::new(db.clone()));
    std::thread::spawn(move || {
        use mao_net::n3::movement;
        const SPEED: f32 = 7.5; // units per second, from the capture
        const BACK_SPEED: f32 = 3.0;
        const TURN: f32 = 2.0; // radians per second
        // The server's heights sit a little off the ground model's; walk at
        // the ground's height plus the offset seen at the start.
        let ground = |terrain: &Option<mao_formats::terrain::Terrain>, p: [f32; 3]| terrain.as_ref().map_or(0.0, |t| t.height_at(p[0], p[2], crate::HEIGHT_SCALE));
        let mut offset = start[1] - ground(&terrain, start);
        let mut base = start[1];
        let clock = Instant::now();
        let tracing = std::env::var("MAO_TRACE").is_ok_and(|v| v == "1");
        let mut last_report = Instant::now();
        let mut fresh = false;
        state.arrival = None;
        let mut pending_scene: Option<Model> = None;
        let mut pending_gear = state.inventory.take().map(|e| crate::gear::from_entries(&gear_db, &gear_textures, &e));
        let mut keys = crate::viewer::Input::default();
        let mut raw = crate::viewer::Input::default();
        let (mut goto, mut goto_seen): (Option<[f32; 2]>, u8) = (None, 0);
        // Walking round obstacles: the best distance so far, when it last improved, the detour being tried
        // (a turn away from the straight line, until when) and how many have been tried.
        let (mut goto_best, mut goto_progress, mut detour, mut detours) = (f32::MAX, Instant::now(), None::<(f32, Instant)>, 0usize);
        // `MAO_GOTO=x,z` starts a walk as the Whom-Pah window's Go button does (for testing without a click).
        if let Some(v) = std::env::var("MAO_GOTO").ok().map(|t| t.split(',').filter_map(|n| n.trim().parse::<f32>().ok()).collect::<Vec<_>>()) {
            if let [x, z] = v[..] {
                goto = Some([x, z]);
            }
        }
        let mut pending_destinations = state.destinations.take();
        let (mut mission_wait, mut mission_used): (Option<Instant>, bool) = (None, false);
        let (mut mission_seen, mut pending_missions, mut pending_mission_status): (u8, Option<Vec<crate::mission::Offer>>, Option<String>) = (0, None, None);
        // `MAO_MISSION=type,instance[,scope[,difficulty]]` (decimal or 0x hex) asks that mission terminal
        // for missions once the character has arrived (or 6 s into the session without a walk), and prints the offers.
        let mut mission_request: Option<(u32, u32, u8, [u8; 7])> = std::env::var("MAO_MISSION").ok().and_then(|t| {
            let n: Vec<u32> = t.split(',').filter_map(|v| {
                let v = v.trim();
                v.strip_prefix("0x").map_or_else(|| v.parse().ok(), |h| u32::from_str_radix(h, 16).ok())
            }).collect();
            // type, instance, scope, then up to seven sliders (the captured values by default).
            let mut sliders = [2u8, 20, 161, 30, 184, 28, 0];
            for (i, v) in n.iter().skip(3).take(7).enumerate() {
                sliders[i] = *v as u8;
            }
            Some((*n.first()?, *n.get(1)?, n.get(2).copied().unwrap_or(5) as u8, sliders))
        });
        let (mut seen, mut target): ([u8; 7], Option<u32>) = ([0; 7], None);
        let (mut sitting, mut uses, mut fired) = (false, 0u32, 0u8);
        let (mut position, mut heading) = (start, heading);
        let (mut moving, mut turning, mut strafing) = (0i8, 0i8, 0i8);
        const JUMP_SPEED: f32 = 7.0;
        const GRAVITY: f32 = 10.0;
        let (mut airborne, mut rise, mut vy, mut was_jumping) = (false, 0.0f32, 0.0f32, false);
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
                Ok(frame) => {
                    // The server's pings are answered (or it ends the connection).
                    if let Some(reply) = frame::ping_reply(&frame, own, 70_000_000 + clock.elapsed().as_millis() as u32) {
                        if connection.send(&reply).is_err() {
                            break;
                        }
                    }
                    if let Some(target) = system::redirect(&frame) {
                        eprintln!("the server sends us to {target}");
                        match hop(&game_dir, &target, own).and_then(|mut c| c.set_timeout(Duration::from_millis(50)).map(|_| c).map_err(|e| e.to_string())) {
                            Ok(next) => {
                                connection = next;
                                // The new playfield is loaded even if its number repeats.
                                current = u32::MAX;
                                fresh = true;
                                continue;
                            }
                            Err(e) => {
                                eprintln!("can't follow the redirect: {e}");
                                break;
                            }
                        }
                    }
                    if tracing {
                        trace(&frame, clock);
                    }
                    if let Some(m) = Message::parse(&frame).filter(|m| m.kind == 0x5c436609) {
                        if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                            let item = |low: i64| {
                                let template = gear_db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: low as u32 }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d))?;
                                let icon = template.icon().and_then(|id| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|t| std::sync::Arc::new(crate::gear::Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })));
                                Some((template.name, icon))
                            };
                            let icon = |id: u32| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|t| std::sync::Arc::new(crate::gear::Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba }));
                            let offers = crate::mission::offers_from(&p.value, &item, &icon);
                            if !offers.is_empty() {
                                pending_missions = Some(offers);
                            }
                        }
                    }
                    if let Some(m) = Message::parse(&frame).filter(|m| matches!(m.kind, 0x5c436609 | 0x465a4061 | 0x212c487a | 0x333b2867 | 0x36284f6e | 0x58362220 | 0x50544d19 | 0x206b4b73 | 0x5f4b442a)) {
                        if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                            eprintln!("[{:6.1}] {} identity ({:x},{:x}) left {}{}", clock.elapsed().as_secs_f32(), p.name, m.identity.0, m.identity.1, p.left, p.value.pretty());
                        }
                    }
                    state.add(&frame);
                    if let Some(list) = state.destinations.take() {
                        pending_destinations = Some(list);
                    }
                    if let Some(entries) = state.inventory.take() {
                        pending_gear = Some(crate::gear::from_entries(&gear_db, &gear_textures, &entries));
                    }
                }
                Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                Err(e) => {
                    eprintln!("connection ended: {e}");
                    break;
                }
            }
            // Zoning: the server announces the new playfield and where we arrive.
            if let Some((playfield, arrival)) = state.arrival.take() {
                if playfield != current {
                    eprintln!("zoning to playfield {playfield}");
                    current = playfield;
                    state.last_move.clear_all();
                    state.last_seen.clear_all();
                    state.speeds.clear_all();
                    state.names.clear_all();
                    state.infos.clear_all();
                    state.rotations.clear_all();
                    state.entities.clear();
                    // Zoned from a booth: that booth leads here.
                    if let Some((pf, booth, when)) = near_booth.take() {
                        if when.elapsed() < Duration::from_secs(20) && pf != playfield {
                            let to = names.get(&playfield).cloned().unwrap_or_else(|| format!("playfield {playfield}"));
                            if destinations.learn(pf, booth[0], booth[2], playfield) {
                                pending_messages.push(format!("Whom-Pah: this booth leads to {to} (remembered)"));
                            } else {
                                pending_messages.push(format!("Whom-Pah to {to}"));
                            }
                        }
                    }
                    pending_location = Some(names.get(&playfield).cloned().unwrap_or_else(|| format!("playfield {playfield}")));
                    match load_area(&game_dir, &thread_db, playfield) {
                        Ok(area) => {
                            (terrain, collision) = (area.terrain, area.collision);
                            pending_scene = Some(area.scene);
                            booths = area.booths;
                            booth_playfield = playfield;
                            pending_labels = Some(booth_labels(playfield, &booths, &destinations, &names));
                            goto = None;
                        }
                        Err(e) => {
                            eprintln!("can't draw playfield {playfield}: {e}");
                            (terrain, collision) = (None, Default::default());
                            pending_scene = Some(Model::default());
                            booths = Vec::new();
                            pending_labels = Some(Vec::new());
                        }
                    }
                    position = arrival;
                    offset = arrival[1] - ground(&terrain, arrival);
                    base = arrival[1];
                    airborne = false;
                    rise = 0.0;
                    state.entities.insert(own, (position, 0));
                    // Ready again, as the original does after loading.
                    if connection.send(&n3::ready(own)).is_err() {
                        break;
                    }
                    // A new connection (a redirect) also starts with the stand-up.
                    if std::mem::take(&mut fresh) && connection.send(&n3::action(own, n3::STAND)).is_err() {
                        break;
                    }
                }
            }
            // (Bounded: the newest state is what counts, and a flood of them must not starve the connection.)
            for _ in 0..256 {
                match input_rx.try_recv() {
                    Ok(k) => raw = k,
                    Err(_) => break,
                }
            }
            keys = raw;
            // Go (the Whom-Pah window): walk to a place; any movement key of the player's own stops it.
            if raw.goto.0 != goto_seen {
                goto_seen = raw.goto.0;
                goto = Some(raw.goto.1);
                (goto_best, goto_progress, detour, detours) = (f32::MAX, Instant::now(), None, 0);
                pending_messages.push("Walking to the booth (any movement key stops)".into());
            }
            if goto.is_some() && (raw.forward || raw.back || raw.strafe != 0 || raw.turn != 0 || raw.jump) {
                goto = None;
                pending_messages.push("Stopped walking".into());
            }
            if let Some(g) = goto {
                let distance = ((g[0] - position[0]).powi(2) + (g[1] - position[2]).powi(2)).sqrt();
                if distance < 1.2 {
                    goto = None;
                    mission_wait = Some(Instant::now());
                } else {
                    if distance < goto_best - 0.5 {
                        (goto_best, goto_progress) = (distance, Instant::now());
                    }
                    // No progress for a second: turn away from the line for a moment, trying one side then the other.
                    if detour.is_none() && goto_progress.elapsed() > Duration::from_millis(900) {
                        let turns = [1.2f32, -1.2, 2.0, -2.0, 0.6, -0.6];
                        detour = Some((turns[detours % turns.len()], Instant::now() + Duration::from_millis(1600)));
                        detours += 1;
                    }
                    if detour.is_some_and(|d| Instant::now() > d.1) {
                        detour = None;
                        goto_progress = Instant::now();
                    }
                    if detours > 12 {
                        goto = None;
                        pending_messages.push("Can't find a way round (stopped)".into());
                    }
                    keys.face = Some((g[0] - position[0]).atan2(g[1] - position[2]) + detour.map_or(0.0, |d| d.0));
                    keys.forward = true;
                }
            }
            // Targeting: Tab and friends step through the nearby ones, nearest first; F attacks.
            for (slot, count) in keys.commands.iter().enumerate() {
                let presses = count.wrapping_sub(seen[slot]);
                seen[slot] = *count;
                if presses == 0 {
                    continue;
                }
                if slot == 6 {
                    // E: use the target (talk to it, open it).
                    match target {
                        Some(t) => {
                            uses += 1;
                            let kind = state.types.get(&t).copied().unwrap_or(n3::CHARACTER);
                            let _ = connection.send(&n3::use_object(own, uses, (kind, t)));
                            pending_messages.push(format!("Using {}", state.names.get(&t).map_or("the target", |n| n.0.as_str())));
                        }
                        None => pending_messages.push("Nothing targeted to use (Tab selects)".into()),
                    }
                    continue;
                }
                if slot == 5 {
                    // X: sit down, or stand up.
                    sitting = !sitting;
                    let _ = connection.send(&n3::action(own, if sitting { n3::LOGOUT } else { n3::STAND }));
                    pending_messages.push(if sitting { "Sitting: the server logs you out in about 30 s. Press X or move to stand up and stay.".into() } else { "Standing up: logout cancelled.".to_string() });
                    continue;
                }
                if slot == 4 {
                    match target {
                        Some(t) => {
                            let _ = connection.send(&n3::attack(own, t));
                            pending_messages.push(format!("Attacking {}", state.names.get(&t).map_or("the target", |n| n.0.as_str())));
                        }
                        None => pending_messages.push("No target (Tab selects one)".into()),
                    }
                    continue;
                }
                let friendly_name = |name: &str| ["Vanguard", "Watcher", "Commander", "Guard", "Merchant", "Trader", "Recruiter", "Helper"].iter().any(|w| name.contains(w));
                let wants_friendly = slot >= 2;
                let near = |p: [f32; 3]| ((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt();
                let mut candidates: Vec<(f32, u32)> = state.infos.iter().filter(|(id, i)| **id != own && (i.is_player() || friendly_name(&i.name)) == wants_friendly).filter_map(|(id, _)| Some((near(state.entities.get(id)?.0), *id))).filter(|(d, _)| *d < 80.0).collect();
                if wants_friendly {
                    candidates.extend(state.objects.iter().map(|(id, p)| (near(*p), *id)).filter(|(d, _)| *d < 80.0));
                }
                candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
                if candidates.is_empty() {
                    pending_messages.push("Nothing to target nearby".into());
                    continue;
                }
                let index = target.and_then(|t| candidates.iter().position(|c| c.1 == t));
                let forward = slot % 2 == 0;
                let next = match (index, forward) {
                    (Some(i), true) => (i + 1) % candidates.len(),
                    (Some(i), false) => (i + candidates.len() - 1) % candidates.len(),
                    (None, true) => 0,
                    (None, false) => candidates.len() - 1,
                };
                let id = candidates[next].1;
                target = Some(id);
                // Objects are targeted here only: the server's select names characters and creatures.
                if !state.objects.contains_key(&id) {
                    let _ = connection.send(&n3::select_target(own, id));
                    let _ = connection.send(&n3::set_target(own, id));
                }
            }
            // The mission window: ask the nearest booth for missions, or accept an offer.
            if raw.mission.0 != mission_seen {
                mission_seen = raw.mission.0;
                let cmd = raw.mission.1;
                if cmd.what == 1 {
                    let nearest = state.vendors.iter().map(|(id, p)| (((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt(), *id)).filter(|(d, _)| *d < 40.0).min_by(|a, b| a.0.total_cmp(&b.0));
                    match nearest {
                        Some((_, serial)) => {
                            uses += 1;
                            let _ = connection.send(&n3::use_object(own, uses, (0xdac1, serial)));
                            let scope = std::env::var("MAO_MISSION_SCOPE").ok().and_then(|v| v.parse().ok()).unwrap_or(crate::mission::DEFAULT_SCOPE);
                            let _ = connection.send(&n3::quest_alternative(own, cmd.sliders, scope, (0xdac1, serial)));
                        }
                        None => pending_mission_status = Some("No mission booth in reach (walk up to one).".into()),
                    }
                } else if cmd.what == 2 {
                    let _ = connection.send(&n3::create_quest(own, cmd.mission));
                    pending_mission_status = Some("Mission requested; see the mission log.".into());
                }
            }
            // A hotbar item: use the inventory slot it sits in.
            if keys.fire.0 != fired {
                fired = keys.fire.0;
                uses += 1;
                let _ = connection.send(&n3::use_object(own, uses, (0x68, keys.fire.1)));
            }
            // The mission request goes out a second after the walk ends (or 6 s into a session without one).
            if mission_request.is_some() && mission_wait.is_none() && goto.is_none() && clock.elapsed() > Duration::from_secs(6) {
                mission_wait = Some(Instant::now());
            }
            if let (Some(t), Some((kind, id, scope, sliders))) = (mission_wait, mission_request) {
                if t.elapsed() > Duration::from_secs(1) && !mission_used {
                    // The original uses the terminal first (the server echoes it), then asks for missions.
                    mission_used = true;
                    mission_wait = Some(Instant::now());
                    eprintln!("using mission terminal ({kind:#x}, {id:#x})");
                    uses += 1;
                    let _ = connection.send(&n3::use_object(own, uses, (kind, id)));
                } else if t.elapsed() > Duration::from_secs(2) && mission_used {
                    mission_request = None;
                    eprintln!("asking mission terminal ({kind:#x}, {id:#x}) for missions");
                    let _ = connection.send(&n3::quest_alternative(own, sliders, scope, (kind, id)));
                }
            }
            if target.is_some_and(|t| state.position_of(t).is_none()) {
                target = None;
            }
            // Moving stands a seated character up first.
            if sitting && (keys.forward || keys.back || keys.strafe != 0 || keys.turn != 0 || keys.jump) {
                sitting = false;
                let _ = connection.send(&n3::action(own, n3::STAND));
                pending_messages.push("Standing up: logout cancelled.".into());
            }
            // Walk: integrate the position and tell the server.
            let dt = last_tick.elapsed().as_secs_f32();
            last_tick = Instant::now();
            if let Some(face) = keys.face {
                heading = face;
            }
            heading += keys.turn as f32 * TURN * dt;
            let walk = keys.forward as i8 - keys.back as i8;
            let speed = if walk > 0 { SPEED } else { BACK_SPEED } * walk as f32;
            // Walls stop the step (sliding along them where one axis is free).
            // Strafing adds a step to the right of the heading; diagonals aren't faster.
            let sideways = keys.strafe as f32 * SPEED;
            let (mut vx, mut vz) = (heading.sin() * speed + heading.cos() * sideways, heading.cos() * speed - heading.sin() * sideways);
            let total = (vx * vx + vz * vz).sqrt();
            if total > SPEED {
                (vx, vz) = (vx * SPEED / total, vz * SPEED / total);
            }
            let speed = if walk == 0 && keys.strafe != 0 { SPEED } else { speed };
            let (dx, dz) = (vx * dt, vz * dt);
            for (tx, tz) in [(dx, dz), (dx, 0.0), (0.0, dz)] {
                if tx == 0.0 && tz == 0.0 {
                    continue;
                }
                let (nx, nz) = (position[0] + tx, position[2] + tz);
                if keys.noclip || !collision.blocked(nx, nz, base, 0.4, 1.8, 0.5) {
                    (position[0], position[2]) = (nx, nz);
                    break;
                }
            }
            let start_jump = keys.jump && !was_jumping && !airborne;
            was_jumping = keys.jump;
            let mut landed = false;
            if airborne {
                rise += vy * dt;
                vy -= GRAVITY * dt;
                if rise <= 0.0 {
                    (airborne, rise, landed) = (false, 0.0, true);
                }
            }
            if start_jump {
                (airborne, vy) = (true, JUMP_SPEED);
            }
            // Stand on the ground or on a floor or step within reach.
            let floor = if keys.noclip { f32::MIN } else { collision.floor(position[0], position[2], base, 0.6, 0.6).unwrap_or(f32::MIN) };
            base = (ground(&terrain, position) + offset).max(floor);
            position[1] = base + rise;
            // Standing at a booth?
            if let Some(b) = booths.iter().find(|b| ((b[0] - position[0]).powi(2) + (b[2] - position[2]).powi(2)).sqrt() < 4.0) {
                near_booth = Some((booth_playfield, *b, Instant::now()));
            }
            if tracing && walk != 0 && last_report.elapsed() >= Duration::from_secs(1) {
                last_report = Instant::now();
                eprintln!("[{:7.2}] walking at ({:.1}, {:.1}, {:.1}) heading {:.2} in playfield {current}", clock.elapsed().as_secs_f32(), position[0], position[1], position[2], heading);
            }
            let mut ok = true;
            let due = last_send.elapsed() >= Duration::from_millis(400);
            let mut send_kind = |kind: u8, connection: &mut Connection| {
                last_send = Instant::now();
                ok &= send(connection, kind, heading, position, &mut sent);
            };
            if start_jump {
                send_kind(movement::JUMP, &mut connection);
            }
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
            } else if ((walk != 0 || keys.turn != 0 || keys.strafe != 0 || keys.face.is_some() || airborne) && due) || landed {
                send_kind(movement::UPDATE, &mut connection);
            }
            if keys.strafe != strafing {
                // The strafe start / stop types (5..8) are a guess from the numbering; until verified the
                // server is told with ordinary position updates (MAO_STRAFE_TYPES=1 sends the guesses).
                if std::env::var_os("MAO_STRAFE_TYPES").is_some() {
                    let kind = match (strafing, keys.strafe) {
                        (_, -1) => 5,
                        (_, 1) => 7,
                        (-1, _) => 6,
                        _ => 8,
                    };
                    send_kind(kind, &mut connection);
                } else if !(keys.turn != turning) {
                    send_kind(movement::UPDATE, &mut connection);
                }
            }
            if !ok {
                break;
            }
            moving = walk;
            turning = keys.turn;
            strafing = keys.strafe;
            state.rotations.insert(own, [0.0, (heading * 0.5).sin(), 0.0, (heading * 0.5).cos()]);
            if walk != 0 || keys.turn != 0 || keys.strafe != 0 || moving != 0 || airborne {
                state.entities.insert(own, (position, 0));
                if walk != 0 || keys.strafe != 0 {
                    state.last_move.insert(own, Instant::now());
                    state.speeds.insert(own, speed.abs());
                }
            } else if let Some(e) = state.entities.get(&own) {
                position = e.0;
            }
            if last_overlay.elapsed() >= Duration::from_millis(60) {
                last_overlay = Instant::now();
                let update = crate::viewer::Update {
                    scene: pending_scene.take(),
                    model: overlay_model(&state, &mut characters),
                    own: Some(position),
                    location: pending_location.take(),
                    labels: pending_labels.take(),
                    messages: std::mem::take(&mut pending_messages),
                    gear: pending_gear.take(),
                    missions: pending_missions.take(),
                    mission_status: pending_mission_status.take(),
                    destinations: pending_destinations.take(),
                    heading: Some(heading),
                    target: target.and_then(|t| Some(format!("{}  ({:.0} m)", state.names.get(&t)?.0, ((state.position_of(t)?[0] - position[0]).powi(2) + (state.position_of(t)?[2] - position[2]).powi(2)).sqrt()))),
                };
                if tx.send(update).is_err() {
                    break;
                }
            }
        }
    });
    crate::viewer::window(scene, crate::Textures::new(db), Some(crate::viewer::Live { overlays: rx, input: input_tx, focus, distance: 6.0 }))
}

pub fn run(args: &[String]) -> Result<(), String> {
    match args {
        [game, word] if word == "chars" => {
            show(&login(game)?.1);
            Ok(())
        }
        // `net <game> action <name> <hex code> [seconds]`: enter, stand, send one action and show what the server does (an experiment for action codes).
        [game, word, who, code, rest @ ..] if word == "action" => {
            let code = u8::from_str_radix(code.trim_start_matches("0x"), 16).map_err(|_| "bad action code")?;
            let seconds: u64 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(6);
            let (mut chat, list) = login(game)?;
            let character = list.iter().find(|c| c.name.eq_ignore_ascii_case(who) || c.id.to_string() == *who).ok_or("no such character")?;
            let ticket = session::login_character(&mut chat, character.id).map_err(|e| e.to_string())?;
            let mut world = session::enter_world(&ticket).map_err(|e| e.to_string())?;
            world.set_timeout(Duration::from_millis(50)).map_err(|e| e.to_string())?;
            let mut state = World { own: character.id, ..World::default() };
            let clock = Instant::now();
            let mut phase = 0;
            let mut sent_at = Instant::now();
            loop {
                let t = clock.elapsed().as_secs_f32();
                if phase == 0 && t > 3.0 {
                    world.send(&n3::ready(character.id)).map_err(|e| e.to_string())?;
                    world.send(&n3::action(character.id, n3::STAND)).map_err(|e| e.to_string())?;
                    eprintln!("[{t:5.1}] ready + stand sent");
                    phase = 1;
                } else if phase == 1 && t > 6.0 {
                    if code == 0 {
                        // Code 0: use the nearest announced object instead.
                        let me = state.entities.get(&character.id).map(|e| e.0).unwrap_or([0.0; 3]);
                        let nearest = state.objects.iter().min_by(|a, b| ((a.1[0] - me[0]).powi(2) + (a.1[2] - me[2]).powi(2)).total_cmp(&((b.1[0] - me[0]).powi(2) + (b.1[2] - me[2]).powi(2))));
                        eprintln!("[{t:5.1}] {} usable objects announced; me at {me:?}", state.objects.len());
                        if let Some((id, p)) = nearest {
                            eprintln!("[{t:5.1}] USE object {id:#x} at {p:?}");
                            world.send(&n3::use_object(character.id, 1, (0xc76a, *id))).map_err(|e| e.to_string())?;
                        }
                    } else {
                        world.send(&n3::action(character.id, code)).map_err(|e| e.to_string())?;
                    }
                    eprintln!("[{t:5.1}] ACTION {code:#x} sent");
                    sent_at = Instant::now();
                    phase = 2;
                } else if phase == 2 && sent_at.elapsed() > Duration::from_secs(seconds) {
                    world.send(&n3::action(character.id, n3::STAND)).map_err(|e| e.to_string())?;
                    eprintln!("[{t:5.1}] STAND (0x57) sent again");
                    sent_at = Instant::now();
                    phase = 3;
                } else if phase == 3 && sent_at.elapsed() > Duration::from_secs(4) {
                    break;
                }
                match world.receive() {
                    Ok(frame) => {
                        if let Some(reply) = frame::ping_reply(&frame, character.id, 70_000_000 + clock.elapsed().as_millis() as u32) {
                            let _ = world.send(&reply);
                        }
                        if phase >= 1 {
                            if let Some(m) = Message::parse(&frame) {
                                if ![0x54111123, 0x2b333d6e, 0x271b3a6b, 0x3b1d2268, 0x36510078, 0x4c7d403b, 0x3b11256f, 0x46002f16, 0x465a5d73, 0x260f3671, 0x195e496e, 0x1d3c0f1c].contains(&m.kind) {
                                    eprintln!("[{:5.1}] <- {:08x} identity ({:x},{:x}) {} bytes: {}", t, m.kind, m.identity.0, m.identity.1, m.data.len(), m.data.iter().take(40).map(|b| format!("{b:02x}")).collect::<String>());
                                }
                            }
                        }
                        state.add(&frame);
                    }
                    Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(e) => {
                        eprintln!("[{t:5.1}] connection ended: {e}");
                        break;
                    }
                }
            }
            state.print();
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
        [game, word, who, rest @ ..] if word == "attack" => fight(game, who, rest.first().and_then(|s| s.parse().ok()).unwrap_or(40), rest.get(1..).unwrap_or(&[]).chunks_exact(2).filter_map(|c| Some((c[0].parse().ok()?, c[1].parse().ok()?))).collect()),
        [game, word, who, target] if word == "hop" => {
            // Log the character in at a redirect's address and say where the world puts it.
            let (_, list) = login(game)?;
            let character = list.iter().find(|c| c.name.eq_ignore_ascii_case(who) || c.id.to_string() == *who).ok_or("no such character")?;
            let mut connection = hop(game, target, character.id)?;
            connection.set_timeout(Duration::from_millis(50)).map_err(|e| e.to_string())?;
            let mut state = World { own: character.id, ..World::default() };
            let until = Instant::now() + Duration::from_secs(6);
            while Instant::now() < until {
                match connection.receive() {
                    Ok(f) => state.add(&f),
                    Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
            match state.arrival {
                Some((pf, at)) => eprintln!("IN the new area: playfield {pf}, arrival ({:.1}, {:.1}, {:.1}), {} entities", at[0], at[1], at[2], state.entities.len()),
                None => eprintln!("the server sent no playfield"),
            }
            Ok(())
        }
        [game, word, who, x, z, rest @ ..] if word == "walk" => walk(game, who, x.parse().map_err(|_| "bad x")?, z.parse().map_err(|_| "bad z")?, rest.first().and_then(|s| s.parse().ok()).unwrap_or(90)),
        [word, log] if word == "parse-capture" => parse_capture(log),
        [word, log] if word == "msg-stats" => msg_stats(log),
        [word, log] if word == "msg-check" => msg_check(log, None, 0),
        [word, log, kind, rest @ ..] if word == "msg-parse" => msg_check(log, u32::from_str_radix(kind, 16).ok(), rest.first().and_then(|n| n.parse().ok()).unwrap_or(2)),
        // `net <game dir> area <playfield>`: load a playfield as the live client does on zoning, and time it.
        [game, word, pf] if word == "area" => {
            let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
            let start = Instant::now();
            let area = load_area(game, &db, pf.parse().map_err(|_| "bad playfield")?)?;
            eprintln!("loaded in {:.1}s: {} parts, {} booths", start.elapsed().as_secs_f32(), area.scene.parts.len(), area.booths.len());
            for b in &area.booths {
                eprintln!("booth at ({:.1}, {:.1}, {:.1})", b[0], b[1], b[2]);
            }
            Ok(())
        }
        [game, word, log] if word == "inventory" => inventory_from_capture(game, log),
        [word, log, kind, rest @ ..] if word == "msg-dump" => msg_dump(log, kind, rest.first().and_then(|n| n.parse().ok()).unwrap_or(3)),
        [game, word, log, rest @ ..] if word == "capture-map" => {
            let world = parse_capture_world(log)?;
            show_world(game, 540, &world, rest.first().map(String::as_str))
        }
        _ => Err("usage: mao-client net <game dir> chars | net <game dir> enter <name or id> [seconds] | net parse-capture <capture.log> | net msg-stats <capture.log> | net <game dir> capture-map <capture.log> [out.png]".into()),
    }
}

/// `net <game> attack <name> [seconds]`: log in, find the nearest creature,
/// walk up to it, attack it as the original does (select, target, attack)
/// and print the combat the server reports.
fn fight(game: &str, who: &str, seconds: u64, waypoints: Vec<(f32, f32)>) -> Result<(), String> {
    use mao_net::n3::movement;
    let (mut chat, list) = login(game)?;
    let character = list.iter().find(|c| c.name.eq_ignore_ascii_case(who) || c.id.to_string() == who).ok_or_else(|| format!("no character {who:?} on the account"))?;
    let own = character.id;
    let ticket = session::login_character(&mut chat, own).map_err(|e| e.to_string())?;
    let mut connection = session::enter_world(&ticket).map_err(|e| e.to_string())?;
    connection.set_timeout(Duration::from_millis(50)).map_err(|e| e.to_string())?;
    let mut state = World { own, ..World::default() };
    let clock = Instant::now();
    let pump = |connection: &mut Connection, state: &mut World, millis: u64, print: bool| -> Result<(), String> {
        let until = Instant::now() + Duration::from_millis(millis);
        while Instant::now() < until {
            match connection.receive() {
                Ok(frame) => {
                    if let Some(reply) = frame::ping_reply(&frame, own, 70_000_000 + clock.elapsed().as_millis() as u32) {
                        connection.send(&reply).map_err(|e| e.to_string())?;
                    }
                    if print {
                        if let Some(m) = Message::parse(&frame) {
                            if !matches!(m.kind, 0x54111123 | 0x2b333d6e | 0x271b3a6b | 0x3b1d2268 | 0x36510078 | 0x4c7d403b | 0x3b11256f | 0x465a5d73 | 0x260f3671 | 0x195e496e | 0x7f544905) {
                                let head: String = m.data.iter().take(40).map(|b| format!("{b:02x}")).collect();
                                eprintln!("[{:6.1}] {:08x} from {:x}:{:x} {head}", clock.elapsed().as_secs_f32(), m.kind, m.identity.0, m.identity.1);
                            }
                        }
                    }
                    state.add(&frame);
                }
                Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    };
    let until = Instant::now() + Duration::from_secs(4);
    while Instant::now() < until && !state.entities.contains_key(&own) {
        pump(&mut connection, &mut state, 100, false)?;
    }
    connection.send(&n3::ready(own)).map_err(|e| e.to_string())?;
    connection.send(&n3::action(own, n3::STAND)).map_err(|e| e.to_string())?;
    pump(&mut connection, &mut state, 1500, false)?;
    let mut position = state.entities.get(&own).map(|e| e.0).ok_or("the character's position never arrived")?;
    let friendly = |name: &str| ["Vanguard", "Watcher", "Commander", "Guard", "Merchant", "Trader", "Recruiter", "Helper"].iter().any(|w| name.contains(w));
    let dist = |a: [f32; 3], b: [f32; 3]| ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    let hostile_near = |state: &World, position: [f32; 3]| state.infos.iter().any(|(id, i)| *id != own && !i.is_player() && !friendly(&i.name) && state.entities.get(id).is_some_and(|e| dist(position, e.0) < 60.0));
    for (gx, gz) in waypoints {
        if hostile_near(&state, position) {
            break;
        }
        // Walk to the next waypoint first (straight, at the starting height).
        eprintln!("walking to ({gx}, {gz})");
        let heading = (gx - position[0]).atan2(gz - position[2]);
        let mut since = clock;
        let mut last_tick = Instant::now();
        let mut last_send = Instant::now();
        let ms = |since: &mut Instant| {
            let m = since.elapsed().as_millis().min(u32::MAX as u128) as u32;
            *since = Instant::now();
            m
        };
        connection.send(&n3::position_update(own, movement::FORWARD_START, heading, position, ms(&mut since))).map_err(|e| e.to_string())?;
        loop {
            pump(&mut connection, &mut state, 50, false)?;
            let (dx, dz) = (gx - position[0], gz - position[2]);
            let left = (dx * dx + dz * dz).sqrt();
            let dt = last_tick.elapsed().as_secs_f32();
            last_tick = Instant::now();
            if left <= 1.0 {
                connection.send(&n3::position_update(own, movement::FORWARD_STOP, heading, position, ms(&mut since))).map_err(|e| e.to_string())?;
                break;
            }
            let step = (7.5 * dt).min(left);
            position[0] += dx / left * step;
            position[2] += dz / left * step;
            if last_send.elapsed() >= Duration::from_millis(400) {
                last_send = Instant::now();
                connection.send(&n3::position_update(own, movement::UPDATE, heading, position, ms(&mut since))).map_err(|e| e.to_string())?;
            }
        }
        pump(&mut connection, &mut state, 2500, false)?;
        let mut seen: Vec<String> = state.infos.iter().filter(|(id, i)| **id != own && !i.is_player()).filter_map(|(id, i)| Some(format!("{} ({:.0})", i.name, dist(position, state.entities.get(id)?.0)))).collect();
        seen.sort();
        eprintln!("  at ({:.0}, {:.0}): {} creatures: {}", position[0], position[2], seen.len(), seen.join(", "));
    }
    // The hostile creatures in view, nearest first.
    let mut mobs: Vec<(f32, u32, String)> = state.infos.iter().filter(|(id, i)| **id != own && !i.is_player() && !friendly(&i.name) && state.entities.contains_key(id)).map(|(id, i)| (dist(position, state.entities[id].0), *id, i.name.clone())).collect();
    mobs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    eprintln!("at ({:.1}, {:.1}, {:.1}); creatures in view:", position[0], position[1], position[2]);
    for (d, id, name) in mobs.iter().take(8) {
        eprintln!("  {d:7.1} {id:>10} {name}");
    }
    let Some(&(_, target, ref name)) = mobs.first() else { return Err("no creature in view".into()) };
    eprintln!("going for {name} ({target})");
    let mut since = clock;
    let mut send = |connection: &mut Connection, kind: u8, heading: f32, position: [f32; 3], since: &mut Instant| -> Result<(), String> {
        let ms = since.elapsed().as_millis().min(u32::MAX as u128) as u32;
        *since = Instant::now();
        connection.send(&n3::position_update(own, kind, heading, position, ms)).map_err(|e| e.to_string())
    };
    let (mut last_tick, mut last_send, mut started) = (Instant::now(), Instant::now(), false);
    let give_up = Instant::now() + Duration::from_secs(60);
    loop {
        pump(&mut connection, &mut state, 50, false)?;
        let goal = state.entities.get(&target).map(|e| e.0).ok_or("the creature went out of view")?;
        let (dx, dz) = (goal[0] - position[0], goal[2] - position[2]);
        let left = (dx * dx + dz * dz).sqrt();
        let heading = dx.atan2(dz);
        if left <= 2.5 {
            if started {
                send(&mut connection, movement::FORWARD_STOP, heading, position, &mut since)?;
            }
            break;
        }
        if Instant::now() > give_up {
            return Err("could not reach the creature".into());
        }
        let dt = last_tick.elapsed().as_secs_f32();
        last_tick = Instant::now();
        if !started {
            started = true;
            send(&mut connection, movement::FORWARD_START, heading, position, &mut since)?;
        }
        let step = (7.5 * dt).min(left);
        position[0] += dx / left * step;
        position[2] += dz / left * step;
        position[1] = goal[1];
        if last_send.elapsed() >= Duration::from_millis(400) {
            last_send = Instant::now();
            send(&mut connection, movement::UPDATE, heading, position, &mut since)?;
        }
    }
    eprintln!("[{:6.1}] next to it; attacking", clock.elapsed().as_secs_f32());
    connection.send(&n3::select_target(own, target)).map_err(|e| e.to_string())?;
    connection.send(&n3::set_target(own, target)).map_err(|e| e.to_string())?;
    connection.send(&n3::attack(own, target)).map_err(|e| e.to_string())?;
    pump(&mut connection, &mut state, seconds * 1000, true)?;
    connection.send(&n3::set_target(own, 0)).map_err(|e| e.to_string())?;
    eprintln!("[{:6.1}] done", clock.elapsed().as_secs_f32());
    Ok(())
}

/// `net <game> walk <name> <x> <z> [seconds]`: log in, walk in a straight
/// line to (x, z) the way the live window reports it, and say what the server
/// does (zoning, redirects). No window and no collision: a way to test
/// zone lines against the live server.
fn walk(game: &str, who: &str, target_x: f32, target_z: f32, seconds: u64) -> Result<(), String> {
    use mao_net::n3::movement;
    let (mut chat, list) = login(game)?;
    let character = list.iter().find(|c| c.name.eq_ignore_ascii_case(who) || c.id.to_string() == who).ok_or_else(|| format!("no character {who:?} on the account"))?;
    let own = character.id;
    let ticket = session::login_character(&mut chat, own).map_err(|e| e.to_string())?;
    let mut connection = session::enter_world(&ticket).map_err(|e| e.to_string())?;
    connection.set_timeout(Duration::from_millis(50)).map_err(|e| e.to_string())?;
    let mut state = World { own, ..World::default() };
    let clock = Instant::now();
    let until = Instant::now() + Duration::from_secs(4);
    while Instant::now() < until && !state.entities.contains_key(&own) {
        match connection.receive() {
            Ok(frame) => state.add(&frame),
            Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    let mut position = state.entities.get(&own).map(|e| e.0).ok_or("the character's position never arrived")?;
    state.arrival = None;
    connection.send(&n3::ready(own)).map_err(|e| e.to_string())?;
    if std::env::var("MAO_WALK_NOSTAND").is_err() {
        connection.send(&n3::action(own, n3::STAND)).map_err(|e| e.to_string())?;
    }
    std::thread::sleep(Duration::from_millis(std::env::var("MAO_WALK_DELAY").ok().and_then(|v| v.parse().ok()).unwrap_or(600)));
    let mut playfield = character.proxy[1];
    eprintln!("start at ({:.1}, {:.1}, {:.1}) in playfield {playfield}; walking to ({target_x}, {target_z})", position[0], position[1], position[2]);
    let (dx, dz) = (target_x - position[0], target_z - position[2]);
    let heading = dx.atan2(dz);
    // Heights follow the ground model, offset as the live window does.
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    let area = load_area(game, &db, playfield)?;
    let (terrain, collision) = (area.terrain, area.collision);
    let mut base = position[1];
    let ground = |p: [f32; 3]| terrain.as_ref().map_or(0.0, |t| t.height_at(p[0], p[2], crate::HEIGHT_SCALE));
    let offset = position[1] - ground(position);
    let mut since = clock;
    let mut send = |connection: &mut Connection, kind: u8, position: [f32; 3], since: &mut Instant| -> Result<(), String> {
        let ms = since.elapsed().as_millis().min(u32::MAX as u128) as u32;
        *since = Instant::now();
        let frame = n3::position_update(own, kind, heading, position, ms);
        if std::env::var("MAO_TRACE").is_ok() {
            eprintln!("  sent {}", frame.body.iter().map(|b| format!("{b:02x}")).collect::<String>());
        }
        connection.send(&frame).map_err(|e| e.to_string())
    };
    if std::env::var("MAO_WALK_NOHEAD").is_err() {
        send(&mut connection, movement::HEADING, position, &mut since)?;
    }
    send(&mut connection, movement::FORWARD_START, position, &mut since)?;
    let (mut last_tick, mut last_send, mut walking) = (Instant::now(), Instant::now(), true);
    let mut last_report = Instant::now();
    let end = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < end {
        match connection.receive() {
            Ok(frame) => {
                if let Some(reply) = frame::ping_reply(&frame, own, 70_000_000 + clock.elapsed().as_millis() as u32) {
                    connection.send(&reply).map_err(|e| e.to_string())?;
                }
                if let Some(target) = system::redirect(&frame) {
                    eprintln!("[{:6.1}] the server redirects us to {target}", clock.elapsed().as_secs_f32());
                    let mut next = hop(game, &target, own)?;
                    next.set_timeout(Duration::from_millis(50)).map_err(|e| e.to_string())?;
                    let mut inside = World { own, ..World::default() };
                    let until = Instant::now() + Duration::from_secs(5);
                    while Instant::now() < until && inside.arrival.is_none() {
                        match next.receive() {
                            Ok(f) => inside.add(&f),
                            Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                            Err(e) => return Err(format!("the shop connection ended: {e}")),
                        }
                    }
                    match inside.arrival {
                        Some((pf, at)) => eprintln!("[{:6.1}] IN the new area: playfield {pf}, arrival ({:.1}, {:.1}, {:.1})", clock.elapsed().as_secs_f32(), at[0], at[1], at[2]),
                        None => eprintln!("the new server sent no playfield"),
                    }
                    return Ok(());
                }
                trace(&frame, clock);
                if let Some(m) = Message::parse(&frame).filter(|m| m.kind == n3::POSITION_UPDATE && m.identity.1 == own) {
                    eprintln!("[{:6.1}] server says WE are at {:?}", clock.elapsed().as_secs_f32(), m.movement().map(|x| (x.0, x.2)));
                    if std::env::var("MAO_TRACE").is_ok() {
                        eprintln!("  echo {}", frame.body.iter().map(|b| format!("{b:02x}")).collect::<String>());
                    }
                }
                state.add(&frame);
                if let Some((pf, at)) = state.arrival.take() {
                    eprintln!("[{:6.1}] ZONED to playfield {pf}, arrival ({:.1}, {:.1}, {:.1}) (from {playfield} at ({:.1}, {:.1}))", clock.elapsed().as_secs_f32(), at[0], at[1], at[2], position[0], position[2]);
                    playfield = pf;
                    return Ok(());
                }
            }
            Err(session::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(e) => return Err(format!("connection ended: {e}")),
        }
        let dt = last_tick.elapsed().as_secs_f32();
        last_tick = Instant::now();
        if walking {
            let (rx, rz) = (target_x - position[0], target_z - position[2]);
            let left = (rx * rx + rz * rz).sqrt();
            let step = std::env::var("MAO_WALK_SPEED").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(7.5) * dt;
            if left <= step {
                (position[0], position[2]) = (target_x, target_z);
                walking = false;
                if std::env::var("MAO_WALK_STEP").is_ok() {
                    // As the original at a door: stop just short, start again from there, stop at the spot.
                    let mut short = position;
                    short[0] -= heading.sin() * 1.1;
                    short[2] -= heading.cos() * 1.1;
                    send(&mut connection, movement::FORWARD_STOP, short, &mut since)?;
                    send(&mut connection, movement::FORWARD_START, short, &mut since)?;
                    std::thread::sleep(Duration::from_millis(440));
                }
                if std::env::var("MAO_WALK_JUMP").is_ok() {
                    // As the original: a jump at the spot, the apex height 2.78 above.
                    let mut apex = position;
                    apex[1] += 2.78;
                    send(&mut connection, movement::JUMP, apex, &mut since)?;
                    eprintln!("[{:6.1}] jumped at ({:.1}, {:.2}, {:.1})", clock.elapsed().as_secs_f32(), apex[0], apex[1], apex[2]);
                }
                send(&mut connection, movement::FORWARD_STOP, position, &mut since)?;
                eprintln!("[{:6.1}] arrived at ({target_x}, {:.1}, {target_z}); waiting for the server", clock.elapsed().as_secs_f32(), position[1]);
            } else {
                position[0] += rx / left * step;
                position[2] += rz / left * step;
                let floor = collision.floor(position[0], position[2], base, 0.6, 0.6).unwrap_or(f32::MIN);
                base = (ground(position) + offset).max(floor);
                position[1] = base;
                // MAO_WALK_RAMP="x0,y0,x1,y1": the height follows a ramp along x (as at a gate).
                if let Some([x0, y0, x1, y1]) = std::env::var("MAO_WALK_RAMP").ok().and_then(|v| { let n: Vec<f32> = v.split(',').filter_map(|p| p.parse().ok()).collect(); <[f32; 4]>::try_from(n).ok() }) {
                    let t = ((position[0] - x0) / (x1 - x0)).clamp(0.0, 1.0);
                    position[1] = y0 + (y1 - y0) * t;
                }
                if let Some(y) = std::env::var("MAO_WALK_Y").ok().and_then(|v| v.parse::<f32>().ok()) {
                    position[1] = y;
                }
                if last_send.elapsed() >= Duration::from_millis(400) {
                    last_send = Instant::now();
                    send(&mut connection, movement::UPDATE, position, &mut since)?;
                }
            }
            if last_report.elapsed() >= Duration::from_secs(5) {
                last_report = Instant::now();
                eprintln!("[{:6.1}] at ({:.1}, {:.1}, {:.1})", clock.elapsed().as_secs_f32(), position[0], position[1], position[2]);
            }
        }
    }
    eprintln!("no zone within {seconds} s; ended at ({:.1}, {:.1})", position[0], position[2]);
    Ok(())
}

/// The server's side of a `tools/capture_proxy.py` log: its messages, the
/// characters of any list among them and the world's traffic.
/// `net msg-stats <capture.log>`: every kind of N3 message in a proxy
/// capture, by direction: how many, their sizes, the identity types they
/// carry and the start of an example. A way to find the windows' traffic
/// (inventory, nano, missions) in a real client's session.
fn msg_stats(log: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let mut readers: BTreeMap<(String, bool), Reader> = BTreeMap::new();
    // (direction, kind) -> (count, min size, max size, identity types, example)
    let mut stats: BTreeMap<(bool, u32), (usize, usize, usize, std::collections::BTreeSet<u32>, Vec<u8>)> = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(tag), Some(connection)) = (f.get(1), f.last()) else { continue };
        let server = match *tag {
            "S>C" => true,
            "C>S" => false,
            "OPEN" => {
                readers.remove(&(connection.to_string(), true));
                readers.remove(&(connection.to_string(), false));
                continue;
            }
            _ => continue,
        };
        if f.len() < 5 {
            continue;
        }
        let hex = f[3];
        let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()).collect();
        let reader = readers.entry((connection.to_string(), server)).or_default();
        if reader.push(&bytes).is_err() {
            continue;
        }
        while let Ok(Some(m)) = reader.next() {
            if m.kind != frame::N3 {
                continue;
            }
            if let Some(msg) = Message::parse(&m) {
                let entry = stats.entry((server, msg.kind)).or_insert((0, usize::MAX, 0, Default::default(), msg.data.iter().take(48).copied().collect()));
                entry.0 += 1;
                entry.1 = entry.1.min(msg.data.len());
                entry.2 = entry.2.max(msg.data.len());
                entry.3.insert(msg.identity.0);
            }
        }
    }
    for ((server, kind), (count, min, max, types, example)) in &stats {
        println!("{} {kind:08x} x{count:<5} {min}..{max} bytes, identity types {:x?}  {}", if *server { "S>C" } else { "C>S" }, types, example.iter().map(|b| format!("{b:02x}")).collect::<String>());
    }
    Ok(())
}

/// `net msg-dump <capture.log> <kind hex | text | system> [count]`: the first
/// messages of a kind in full (an N3 kind as 8 hex digits, `text` for the
/// text frames, `system` for the system ones), with direction and identity.
fn msg_dump(log: &str, kind: &str, count: usize) -> Result<(), String> {
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let want = u32::from_str_radix(kind, 16).ok();
    let mut readers: BTreeMap<(String, bool), Reader> = BTreeMap::new();
    let mut shown = 0;
    let mut ordinal = 0usize;
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(tag), Some(connection)) = (f.get(1), f.last()) else { continue };
        let server = match *tag {
            "S>C" => true,
            "C>S" => false,
            "OPEN" => {
                readers.remove(&(connection.to_string(), true));
                readers.remove(&(connection.to_string(), false));
                continue;
            }
            _ => continue,
        };
        if f.len() < 5 {
            continue;
        }
        let bytes: Vec<u8> = (0..f[3].len() / 2).filter_map(|i| u8::from_str_radix(&f[3][2 * i..2 * i + 2], 16).ok()).collect();
        let reader = readers.entry((connection.to_string(), server)).or_default();
        if reader.push(&bytes).is_err() {
            continue;
        }
        while let Ok(Some(m)) = reader.next() {
            ordinal += 1;
            let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
            let matches = match kind {
                "text" => m.kind == frame::TEXT,
                "system" => m.kind == frame::SYSTEM,
                "all" => true,
                _ => m.kind == frame::N3 && Message::parse(&m).is_some_and(|x| Some(x.kind) == want),
            };
            if matches && shown < count {
                shown += 1;
                let dir = if server { "S>C" } else { "C>S" };
                match Message::parse(&m) {
                    Some(x) if m.kind == frame::N3 => println!("#{ordinal} {dir} {connection} kind {:08x} identity ({:x},{:x}) {} bytes: {}", x.kind, x.identity.0, x.identity.1, x.data.len(), hex(&x.data)),
                    _ => println!("{dir} {connection} frame kind {} src {:x} dst {:x} {} bytes: {}", m.kind, m.source, m.destination, m.body.len(), hex(&m.body)),
                }
            }
        }
    }
    Ok(())
}

/// The items in the first world-entry message of a proxy capture.
pub(crate) fn capture_entries(log: &str) -> Result<Vec<inventory::Entry>, String> {
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let mut readers: BTreeMap<String, Reader> = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(tag), Some(connection)) = (f.get(1), f.last()) else { continue };
        if *tag == "OPEN" {
            readers.remove(*connection);
        }
        if *tag != "S>C" || f.len() < 5 {
            continue;
        }
        let bytes: Vec<u8> = (0..f[3].len() / 2).filter_map(|i| u8::from_str_radix(&f[3][2 * i..2 * i + 2], 16).ok()).collect();
        let reader = readers.entry(connection.to_string()).or_default();
        if reader.push(&bytes).is_err() {
            continue;
        }
        while let Ok(Some(m)) = reader.next() {
            if let Some(msg) = Message::parse(&m).filter(|x| m.kind == frame::N3 && x.kind == inventory::WORLD_ENTRY) {
                return Ok(inventory::parse(&msg.data));
            }
        }
    }
    Err("no world-entry message in the capture".into())
}

/// The first server `QuestAlternative` (the offered missions) of a proxy capture, parsed.
pub(crate) fn capture_quest_alternative(log: &str) -> Option<mao_net::schema::Value> {
    let text = std::fs::read_to_string(log).ok()?;
    let mut readers: BTreeMap<String, Reader> = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(tag), Some(connection)) = (f.get(1), f.last()) else { continue };
        if *tag == "OPEN" {
            readers.remove(*connection);
        }
        if *tag != "S>C" || f.len() < 5 {
            continue;
        }
        let bytes: Vec<u8> = (0..f[3].len() / 2).filter_map(|i| u8::from_str_radix(&f[3][2 * i..2 * i + 2], 16).ok()).collect();
        let reader = readers.entry(connection.to_string()).or_default();
        if reader.push(&bytes).is_err() {
            continue;
        }
        while let Ok(Some(m)) = reader.next() {
            if let Some(msg) = Message::parse(&m).filter(|x| m.kind == frame::N3 && x.kind == 0x5c436609) {
                if let Ok(p) = mao_net::schema::Schema::global().parse_n3(msg.kind, &msg.data) {
                    return Some(p.value);
                }
            }
        }
    }
    None
}

/// `net <game dir> inventory <capture.log>`: the items in the first
/// world-entry message of a capture, with the names from the game's database.
fn inventory_from_capture(game: &str, log: &str) -> Result<(), String> {
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    let items = crate::gear::from_entries(&db, &crate::Textures::new(db.clone()), &capture_entries(log)?);
    for i in &items {
        let place = match crate::gear::slot_name(i.slot) {
            Some(n) => n,
            None if i.slot >= inventory::BACKPACK_FIRST => format!("backpack {}", i.slot - inventory::BACKPACK_FIRST),
            None => format!("slot {}", i.slot),
        };
        println!("{place:<20} {} x{:<5} QL{:<4} {}  icon {}", if i.bag { "BAG" } else { "   " }, i.count, i.quality, i.name, if i.icon.is_some() { "yes" } else { "no" });
    }
    Ok(())
}

/// `net msg-check <capture.log>`: parses every N3 message of a capture with the protocol schema
/// and tells, per message kind and direction, how many parsed exactly, how many left bytes over
/// and how many failed (with the first error). `net msg-parse <log> <kind hex> [n]` prints the first
/// `n` messages of a kind as named fields.
fn msg_check(log: &str, only: Option<u32>, show: usize) -> Result<(), String> {
    use mao_net::schema::Schema;
    let schema = Schema::global();
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let mut readers: BTreeMap<(String, bool), Reader> = BTreeMap::new();
    // (direction, kind) -> (ok, leftover, failed, undefined, first error)
    let mut stats: BTreeMap<(bool, u32), (usize, usize, usize, usize, String)> = BTreeMap::new();
    let mut shown = 0;
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(tag), Some(connection)) = (f.get(1), f.last()) else { continue };
        let server = match *tag {
            "S>C" => true,
            "C>S" => false,
            "OPEN" => {
                readers.remove(&(connection.to_string(), true));
                readers.remove(&(connection.to_string(), false));
                continue;
            }
            _ => continue,
        };
        if f.len() < 5 {
            continue;
        }
        let bytes: Vec<u8> = (0..f[3].len() / 2).filter_map(|i| u8::from_str_radix(&f[3][2 * i..2 * i + 2], 16).ok()).collect();
        let reader = readers.entry((connection.to_string(), server)).or_default();
        if reader.push(&bytes).is_err() {
            continue;
        }
        while let Ok(Some(m)) = reader.next() {
            let Some(msg) = Message::parse(&m).filter(|_| m.kind == frame::N3) else { continue };
            let entry = stats.entry((server, msg.kind)).or_default();
            match schema.parse_n3(msg.kind, &msg.data) {
                Ok(p) => {
                    if p.left == 0 {
                        entry.0 += 1;
                    } else {
                        entry.1 += 1;
                        if entry.4.is_empty() {
                            entry.4 = format!("{} bytes left over of {}", p.left, msg.data.len());
                        }
                    }
                    if only == Some(msg.kind) && shown < show {
                        shown += 1;
                        println!("{} {} identity ({:x},{:x}), {} bytes, {} left over:{}", if server { "S>C" } else { "C>S" }, p.name, msg.identity.0, msg.identity.1, msg.data.len(), p.left, p.value.pretty());
                    }
                }
                Err(e) if e.starts_with("no definition") => entry.3 += 1,
                Err(e) => {
                    entry.2 += 1;
                    if entry.4.is_empty() {
                        entry.4 = e;
                    }
                }
            }
        }
    }
    if only.is_some() {
        return Ok(());
    }
    let (mut ok, mut all) = (0, 0);
    for ((server, kind), (good, left, failed, undefined, error)) in &stats {
        let name = schema.n3_name(*kind).unwrap_or("?");
        ok += good;
        all += good + left + failed + undefined;
        println!("{} {kind:08x} {name:<28} ok {good:<5} left {left:<4} failed {failed:<4} undefined {undefined:<4} {error}", if *server { "S>C" } else { "C>S" });
    }
    println!("{ok} of {all} messages parsed exactly");
    Ok(())
}

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
