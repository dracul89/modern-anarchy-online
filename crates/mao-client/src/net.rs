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
/// Follows a redirect the way the original does: straight to the game server it names, with the character and the session key the
/// login handed out (every zoning of a session reuses the same key: the captures of the original show it, and it connects within
/// a few milliseconds of the old connection's end). Logging in again instead is refused until the server lets go of the old session,
/// which took a minute or more.
fn hop_direct(target: &str, character: u32, key: [u8; 8]) -> Result<Connection, String> {
    let (host, port) = target.rsplit_once(':').ok_or("a redirect without a port")?;
    let octets: Vec<u8> = host.split('.').filter_map(|n| n.parse().ok()).collect();
    let address: [u8; 4] = octets.try_into().map_err(|_| "a redirect that is not an address")?;
    let ticket = system::Ticket { id: character, address, port: port.parse().map_err(|_| "a bad redirect port")?, key, rest: Vec::new() };
    session::enter_world(&ticket).map_err(|e| e.to_string())
}

fn hop(game: &str, target: &str, character: u32, say: &dyn Fn(String)) -> Result<Connection, String> {
    let (user, password) = credentials()?;
    let version = version(game)?;
    let default = std::env::var("MAO_SERVER").unwrap_or_else(|_| session::RUBI_KA.to_string());
    let mut last = String::new();
    // (The address in a redirect is not a login server: it closes the connection. The normal
    // login hands out a ticket for the right game server; the redirect address is the fallback.)
    // The old session lingers a moment on the server (the login is refused until it is gone): try again a few times.
    // (After a real zone line the server takes a while to let go of the character: the logins are refused for
    // some seconds, so keep trying for a minute and say so.)
    for attempt in 0..90 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_secs(1));
            if attempt % 8 == 1 {
                say(format!("Zoning: waiting for the server to let go of the character ({attempt}s)..."));
            }
        }
    for server in [default.as_str(), target] {
        let outcome = (|| -> Result<Connection, String> {
            let mut chat = Connection::connect(server).map_err(|e| e.to_string())?;
            session::login(&mut chat, &user, &password, &version).map_err(|e| e.to_string())?;
            let ticket = session::login_character(&mut chat, character).map_err(|e| e.to_string())?;
            eprintln!("game server {} (login at {server})", ticket.host());
            session::enter_world(&ticket).map_err(|e| e.to_string())
        })();
        match outcome {
            Ok(connection) => return Ok(connection),
            Err(e) => {
                eprintln!("login at {server} failed: {e}");
                last = e;
            }
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
    /// When each last jumped (the jump and landing animations).
    jumped: BTreeMap<u32, Instant>,
    /// When each was last heard of and its speed in units a second.
    last_seen: BTreeMap<u32, Instant>,
    speeds: BTreeMap<u32, f32>,
    /// A playfield announcement not yet acted on: (playfield, arrival).
    arrival: Option<(u32, [f32; 3])>,
    /// The game's clock (the server's `GameTime` message sets it).
    game_clock: crate::gametime::Clock,
    /// The character's items, from the world-entry message (until taken).
    inventory: Option<Vec<inventory::Entry>>,
    /// The character's stats by stat number (the world-entry message and the later `Stat` messages), and whether
    /// they changed since they were last sent to the interface.
    stats: BTreeMap<u32, i64>,
    stats_changed: bool,
    /// The summed effects of the worn items (see `derived`), added to the stored stats for display.
    item_mods: BTreeMap<u32, i64>,
    /// The same bonuses by where they come from: worn items, perks, active nanos (for the hover breakdown).
    mods_by_source: [BTreeMap<u32, i64>; 4],
    /// The perks (and research ranks) the world-entry message lists for the character.
    perks: Vec<u32>,
    /// The organization benefits the server lists (`OrgServer` type 6): quality, low and high item template.
    org_benefits: Vec<inventory::Entry>,
    /// The nano programs active on the character (`SimpleCharFullUpdate`), the inventory the effects are summed over, and
    /// whether any of the three changed since `item_mods` was last worked out.
    nanos: Vec<u32>,
    /// The running nanos with the time each had left when the server told us, and when that was (for the NCU window).
    nano_times: Vec<(u32, u32)>,
    nano_times_at: Option<Instant>,
    nano_list_changed: bool,
    /// The nano programs the character has (from the world-entry message), and whether the list is new.
    program_ids: Vec<u32>,
    programs_changed: bool,
    perks_changed: bool,
    /// The character an `/inspect` waits on: its info packet is told in the chat window when it comes.
    inspecting: Option<u32>,
    /// The worn items an `Inspect` answer brought, to be named and told.
    inspected: Option<(u32, Vec<mao_net::n3::Inspected>)>,
    /// Nano casts that involve the character, not yet told: (caster, target, nano).
    casts: Vec<(u32, u32, u32)>,
    /// The level of the characters (not creatures) whose info packets came.
    levels: BTreeMap<u32, u8>,
    /// The organization name of the characters an `OrgInfoPacket` told about.
    orgs: BTreeMap<u32, String>,
    /// Nano programs that landed: (target, nano id), to be drawn as glows.
    landed: Vec<(u32, u32)>,
    /// Casts to show the cast effect of: (caster, nano program).
    cast_bursts: Vec<(u32, u32, u32)>,
    /// When a character last cast a nano on itself, by (character, nano): the server repeats an aura's cast every few seconds.
    self_casts: std::collections::HashMap<(u32, u32), Instant>,
    /// The nanos the player asked to cast and when: those casts always show.
    asked_casts: std::collections::HashMap<u32, Instant>,
    /// Casts to show the flight and landing of: (caster, target, nano program).
    shots_pending: Vec<(u32, u32, u32)>,
    /// The hits that bend a character's mesh (`_GfxControlDeformer_t`), by the character and when they landed.
    deforms: std::collections::HashMap<u32, (mao_formats::gfx::Deform, Instant)>,
    /// The worn meshes of each character from its `AppearanceUpdate`: attachment position, static mesh instance and the texture
    /// instance that replaces the mesh's own (0 for none).
    equipment: BTreeMap<u32, Vec<(u8, u32, u32)>>,
    /// The creature (NPC template) a character is polymorphed into by a running nano program.
    morphs: BTreeMap<u32, u32>,
    /// Which way a character is sidestepping (-1 left, 1 right) while it is: only the player's own is known.
    strafing: BTreeMap<u32, i8>,
    /// Whether a character is moving backwards (its own: the back key; the others: the move against their facing).
    backing: BTreeMap<u32, bool>,
    /// `Buff` messages not yet looked at: (character, nano program). The server repeats a character's buffs every twenty seconds or so.
    buffs_seen: Vec<(u32, u32)>,
    /// The programs other characters run, with when their buff was last announced (the server repeats them about every twenty seconds).
    active_buffs: BTreeMap<u32, BTreeMap<u32, Instant>>,
    /// The polymorph of each other character: the creature and when its program was last announced.
    morphed_until: BTreeMap<u32, (u32, Instant)>,
    /// The vehicle program (a Phasefront hoverboard or hoverbike) each character runs: its `ChangeMesh` spells, keyed on the
    /// body, give the vehicle body the character is drawn as (`Characters::vehicle`).
    riding: BTreeMap<u32, u32>,
    /// The vehicle program of each other character and when its buff was last announced.
    riding_until: BTreeMap<u32, (u32, Instant)>,
    /// Health and maximum from the info packets (the server answers an info request with them).
    health: BTreeMap<u32, (i64, i64)>,
    /// The health each character was last known to have (from the damage events: the target's health after the hit).
    last_hp: BTreeMap<u32, i64>,
    /// The social action each character is doing: its name and when it began (from `SocialActionCmd`).
    socials: BTreeMap<u32, (&'static str, Instant)>,
    /// The characters that died (`CharacterAction` 99, `Death`, from the one dying): the death's action (its `Parameter2`: 500
    /// `die-knees`, 503 `die-shot` ...) and when. Cleared by its `Despawn` or a new full update.
    dead: BTreeMap<u32, (u32, Instant)>,
    /// Whom each character is fighting (the `Attack` message from it, its full update's `FightingTarget`; `StopFight` ends it):
    /// a fighting character has its weapon drawn (`WeaponHolder_t` +0x44 at 2, FUN_1006a1b1; 1 again at FUN_100690c8).
    fighting: BTreeMap<u32, u32>,
    /// Each fighting character's turn towards its target (`Facing`).
    facings: BTreeMap<u32, Facing>,
    /// The weapons' own ids (their `WeaponItemFullUpdate` identity) and templates.
    weapon_templates: BTreeMap<u32, u32>,
    /// When the client's reload is done (the weapon's stat 210 after the `Reload`), to tell "Reloading completed".
    reload_done: Option<Instant>,
    /// The weapons the characters hold, by (owner, place): the item's template (`WeaponItemFullUpdate`: the owner, the place in
    /// `Unknown2`'s low byte, the template in stat 702, `ACGItemTemplateID`).
    weapons: BTreeMap<(u32, u8), u32>,
    /// Their Energy (stat 26: the rounds left), by (owner, place): `WeaponItemFullUpdate`'s, then each attack's `AmmoCount`
    /// (FUN_1006ae3c sets it when the weapon has the stat).
    weapon_energy: BTreeMap<(u32, u8), i64>,
    /// What the last attack of each (attacker, place) put on the place's weapon (FUN_1006ae3c: the target, the hit type +0x2c
    /// and the damage +0x30), for the attack notes' sounds (FUN_1009b9f3).
    attacks: BTreeMap<(u32, u32), (u32, i32, i64)>,
    /// The characters that died since the last frame, for their death's sound (`CharDie_t`, FUN_1007b803).
    died: Vec<u32>,
    /// The nano casts under way, for their sounds (`CharCastNano_t`).
    nano_casts: crate::sounds::NanoCasts,
    /// The casts' sounds to play: (at whom, sound, delay, volume).
    cast_sounds: Vec<(u32, u32, f32, f32)>,
    /// The mines, traps and central controllers, for their sounds (`hazards`).
    hazards: crate::hazards::Hazards,
    /// The playfield's district music overrides (`ScriptIIR_t`).
    music_overrides: crate::sounds::MusicOverrides,
    /// Notum Wars seen on the character (the sound module's +0xbc: stat 389 bit 0 once, kept).
    expansion: bool,
    /// The script events come (`ScriptIIR_t`), run by the game loop.
    script_events: Vec<crate::sounds::ScriptIir>,
    /// The hazards' sounds to play: (sound, where).
    hazard_sounds: Vec<(u32, [f32; 3])>,
    /// Effects to make at a place (`CreateEffect2` at a dynel): (template, where), for the viewer.
    placed_effects: Vec<(i32, [f32; 3])>,
    /// The lightning's strikes heard (where, seed) and the eye's jolts, for the viewer.
    strikes: Vec<([f32; 3], u32)>,
    jolts: Vec<f32>,
    /// The landings' places (Y up) for their effect 0x2c, for the viewer.
    landings: Vec<[f32; 3]>,
    /// Each character's last attack (`AttackInfo`, `MissedAttackInfo`, `SpecialAttackInfo`): the weapon's place, when, a count
    /// picking the swing, and the animation class (0xb, or a special's).
    swings: BTreeMap<u32, (u32, Instant, u32, u32)>,
    /// The specials the server timed (skill stat, seconds; 0 = ready again), not yet given to the hotbars.
    special_events: Vec<(u32, f32)>,
    /// Damage and healing events not yet shown as floating numbers: (target id, amount, stat).
    damage: Vec<(u32, i64, u32)>,
    /// Lines the server's game messages made (damage, healing, "You successfully perform ..."), for the chat window.
    game_lines: Vec<crate::chatwindow::Line>,
    /// The container (corpse, bag) the character opened, and whether that changed.
    loot: Option<Loot>,
    loot_changed: bool,
    /// A corpse whose contents just arrived and have not been auto-looted yet: its handle and the places to take.
    fresh_corpse: Option<(u32, Vec<u32>)>,
    worn: Vec<inventory::Entry>,
    mods_dirty: bool,
    /// The identity type each entity was first heard of with (what a use message names it by).
    types: BTreeMap<u32, u32>,
    /// The vending machines (booths: shops, and the mission booths' companions) with their positions.
    vendors: BTreeMap<u32, [f32; 3]>,
    /// The doors of the playfield by number: the key of the door's model in `props` and its position (from the dynel table).
    door_slots: BTreeMap<u8, (u32, [f32; 3])>,
    /// The server's ids of the doors (from their status messages) and until when each door (by number) stays open.
    door_ids: std::collections::BTreeSet<u32>,
    doors_open: BTreeMap<u8, Instant>,
    /// The machines the server announces with a model (mesh instance, position, rotation): drawn like the other entities.
    props: BTreeMap<u32, (u32, [f32; 3], [f32; 4])>,
    /// The travel list of the playfield, from the server (until taken).
    destinations: Option<mao_net::destinations::DestinationList>,
    /// The usable objects the server announces (doors, mail terminals, booths: identity type
    /// 0xc76a, message `4f474e05`) with their positions; targeted with Ctrl+Tab and used with E.
    objects: BTreeMap<u32, [f32; 3]>,
}

/// A fighting character turning to face its target, as its state machine does: `CharFight_t` (FUN_1007bd5f, its step
/// FUN_1007bdd3) looks every 0.67 s (0x101627e4) while the character stands, and when its facing is more than 0.26 rad
/// (0x101627e0) off the line to the target it becomes `CharTurn_t` (FUN_1007c9b0, the angle from FUN_1007c793), which turns at
/// 9 (0x101629a4) × 100/125 rad a second (FUN_1007c8af) until less than 0.005 rad (0x10162988) is left, then fights on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Facing {
    /// The time since the last look (CharFight_t +0xc).
    timer: f32,
    /// A turn under way: the angle left and its direction (CharTurn_t +0x10, +0x18).
    turn: Option<(f32, f32)>,
}

impl Facing {
    /// One frame: the new heading (`atan2(dx, dz)` of the facing) of a character at `at` fighting one at `target`.
    fn step(&mut self, heading: f32, at: [f32; 3], target: [f32; 3], moving: bool, dt: f32) -> f32 {
        if let Some((left, dir)) = self.turn {
            let step = (9.0 * 100.0 / 125.0 * dt).min(left);
            let left = left - step;
            // (a fresh CharFight_t starts its timer at 0)
            self.turn = if left < 0.005 { None } else { Some((left, dir)) };
            if self.turn.is_none() {
                self.timer = 0.0;
            }
            return heading + dir * step;
        }
        self.timer += dt;
        if moving {
            self.timer = 0.0;
            return heading;
        }
        if self.timer <= 0.67 {
            return heading;
        }
        self.timer = 0.0;
        let (dx, dz) = (target[0] - at[0], target[2] - at[2]);
        if dx == 0.0 && dz == 0.0 {
            return heading;
        }
        // (the signed angle from the facing to the target, flat on the ground)
        let off = (dx.atan2(dz) - heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        if off.abs() > 0.26 {
            self.turn = Some((off.abs(), off.signum()));
        }
        heading
    }
}

/// The game's resources, for the item names and stats the fight lines need (set when play starts).
static ITEM_DB: std::sync::OnceLock<Arc<ResourceDatabase>> = std::sync::OnceLock::new();

/// A stat of an item template.
fn template_stat(template: u32, stat: u32) -> Option<i64> {
    let data = ITEM_DB.get()?.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: template }).ok().flatten()?;
    mao_formats::item::Item::parse(&data)?.stat(stat).map(|v| v as i64)
}

/// A stat of a nano program (type 0xfde85).
fn nano_stat(nano: u32, stat: u32) -> Option<i64> {
    let data = ITEM_DB.get()?.get(mao_formats::resource::Id { kind: 0xfde85, instance: nano }).ok().flatten()?;
    mao_formats::item::Item::parse(&data)?.stat(stat).map(|v| v as i64)
}

/// The name of an item template.
fn template_name(template: u32) -> Option<String> {
    let data = ITEM_DB.get()?.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: template }).ok().flatten()?;
    Some(mao_formats::item::Item::parse(&data)?.name)
}

/// The line for an `Absorb` (the character's armour soaked up damage of a type) or a `ShieldAttack` (a damage shield struck the character).
/// What an `InventoryUpdate` lists for a container the character used.
struct Loot {
    identity: (u32, u32),
    handle: u32,
    entries: Vec<inventory::Entry>,
}

/// Entities spawned by the game (the backyard's monsters, which are dressed like players) have ids from here up;
/// characters have lower ones (seen in the captures and live).
const DYNAMIC_NPC: u32 = 0x7a00_0000;

/// The server's announcement of a usable object, and the identity type of those objects.
const OBJECT_ANNOUNCE: u32 = 0x4f474e05;
/// `InventoryUpdate` (a container's contents), `ContainerAddItem` (an item moved) and the despawn of an object.
const INVENTORY_UPDATE: u32 = 0x4e536976;
const CONTAINER_ADD_ITEM: u32 = 0x47537a24;

/// The items of a parsed `InventoryUpdate`'s `Items` list.
fn inventory_from_schema_items(list: Option<&mao_net::schema::Value>) -> Vec<inventory::Entry> {
    inventory_from_schema(list)
}

/// `DoorStatusUpdate` and the identity type of doors.
const DOOR_STATUS: u32 = 0x4c7d403b;
const DOOR_TYPE: u32 = 0xc748;
/// How long a door that was used stays open.
const DOOR_OPEN: Duration = Duration::from_secs(5);
const OBJECT_TYPE: u32 = 0xc76a;

/// The items of a parsed world-entry message's `InventorySlots` list.
fn inventory_from_schema(list: Option<&mao_net::schema::Value>) -> Vec<inventory::Entry> {
    let Some(mao_net::schema::Value::List(items)) = list else { return Vec::new() };
    let int = |item: &mao_net::schema::Value, name: &str| item.get(name).and_then(mao_net::schema::Value::as_int).unwrap_or(0);
    items
        .iter()
        .map(|i| {
            let identity = i.get("Identity");
            let part = |name: &str| identity.and_then(|v| v.get(name)).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
            inventory::Entry {
                slot: int(i, "Placement") as u32,
                flags: int(i, "Flags") as u16,
                count: int(i, "Count") as u16,
                container: (part("Type"), part("Instance")),
                low_id: int(i, "ItemLowId") as u32,
                high_id: int(i, "ItemHighId") as u32,
                quality: int(i, "Quality") as u32,
                extra: int(i, "Unknown") as u32,
            }
        })
        .filter(|e| e.low_id != 0)
        .collect()
}

/// The perk ids of a parsed world-entry message's `Perks` list: perks and research ranks the character owns. The entry of a
/// research goal still in progress carries a time in its last field (`Unknown3`) and does not count yet.
fn perk_ids(list: Option<&mao_net::schema::Value>) -> Vec<u32> {
    let Some(mao_net::schema::Value::List(items)) = list else { return Vec::new() };
    items
        .iter()
        .filter(|i| i.get("Unknown3").and_then(mao_net::schema::Value::as_int).unwrap_or(0) == 0)
        .filter_map(|i| i.get("SkillId").and_then(mao_net::schema::Value::as_int))
        .map(|v| v as u32)
        .collect()
}

/// The chat connection is opened once per run; the lines it receives wait here for the world loop, and what the chat window sends goes
/// out through the sender set when it opens.
static CHAT_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static CHAT_LINES: std::sync::Mutex<Vec<crate::chatwindow::Line>> = std::sync::Mutex::new(Vec::new());
static CHAT_OUT: std::sync::OnceLock<std::sync::Mutex<std::sync::mpsc::Sender<crate::chatwindow::Out>>> = std::sync::OnceLock::new();

/// The friends list as the chat server tells it: a version that steps with each change, and id -> (name, online).
static BUDDIES: std::sync::Mutex<(u64, BTreeMap<u32, (String, bool)>)> = std::sync::Mutex::new((0, BTreeMap::new()));

/// The kind of the chat channel that is the character's organization's (the public chat protocol's group type 3).
const ORG_CHANNEL_KIND: u8 = 3;

/// Sends what the chat window typed (a tell or a line for the organization channel).
/// Vicinity speech to send on the game connection: (kind: 0 say, 1 whisper, 2 shout; text; voice record).
static SPEECH: std::sync::Mutex<Vec<(u8, String, Vec<u8>)>> = std::sync::Mutex::new(Vec::new());

/// Vicinity speech (`/say`, `/shout`, `/whisper`, plain text, `/voice`), sent by the game loop.
pub fn speak(kind: u8, text: String, record: Vec<u8>) {
    SPEECH.lock().unwrap_or_else(|e| e.into_inner()).push((kind, text, record));
}

pub fn send_chat(out: crate::chatwindow::Out) {
    match CHAT_OUT.get() {
        Some(tx) => {
            let _ = tx.lock().unwrap().send(out);
        }
        None => CHAT_LINES.lock().unwrap().push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, "The chat server is not connected.")),
    }
}

impl World {
    /// The weapon template a character holds in a place: the server's `WeaponItemFullUpdate`, or the client's own inventory.
    fn weapon_of(&self, id: u32, slot: u32) -> Option<u32> {
        self.weapons.get(&(id, slot as u8)).copied().or_else(|| (id == self.own).then(|| self.inventory.as_ref()?.iter().find(|e| e.slot == slot).map(|e| e.low_id)).flatten())
    }

    /// The lines due by now: the end of a reload.
    fn tick_lines(&mut self) {
        if self.reload_done.is_some_and(|t| Instant::now() >= t) {
            self.reload_done = None;
            self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, crate::feedback::plain("Feedback_ReloadingCompleted")));
        }
    }

    /// Who a character is in a fight line: the client's own, one it knows by name, or one it doesn't.
    fn who(&self, id: u32) -> crate::feedback::Who<'_> {
        if id == self.own {
            crate::feedback::Who::You
        } else {
            self.names.get(&id).map_or(crate::feedback::Who::Unknown, |n| crate::feedback::Who::Named(&n.0, n.1))
        }
    }

    /// A nano landed on the character (or its duration changed): it joins the running list with that time, the others' times first
    /// brought up to now.
    fn nano_duration(&mut self, nano: u32, ms: u32) {
        let elapsed = self.nano_times_at.map_or(0, |t| t.elapsed().as_millis().min(u32::MAX as u128) as u32);
        for entry in &mut self.nano_times {
            entry.1 = entry.1.saturating_sub(elapsed);
        }
        match self.nano_times.iter_mut().find(|e| e.0 == nano) {
            Some(entry) => entry.1 = ms,
            None => {
                self.nano_times.push((nano, ms));
                self.nanos.push(nano);
                self.mods_dirty = true;
            }
        }
        self.nano_times_at = Some(Instant::now());
        self.nano_list_changed = true;
    }

    /// Takes the nanos whose time has run out off the running list.
    fn prune_nanos(&mut self) {
        let Some(at) = self.nano_times_at else { return };
        let elapsed = at.elapsed().as_millis().min(u32::MAX as u128) as u32;
        let before = self.nano_times.len();
        let ended: Vec<u32> = self.nano_times.iter().filter(|e| e.1 <= elapsed).map(|e| e.0).collect();
        if !ended.is_empty() {
            self.nano_times.retain(|e| e.1 > elapsed);
            self.nanos.retain(|n| !ended.contains(n));
            self.nano_list_changed = true;
            self.mods_dirty = true;
        }
        let _ = before;
    }

    /// Works out the bonuses to the stored stats again when the worn items, perks or active nanos changed.
    fn refresh_mods(&mut self, db: &ResourceDatabase) {
        if std::mem::take(&mut self.mods_dirty) {
            let level = self.stats.get(&0x36).copied().unwrap_or(1);
            self.mods_by_source = [crate::derived::worn_modifiers(db, &self.worn, level), crate::derived::perk_modifiers(db, &self.perks, level), crate::derived::nano_modifiers(db, &self.nanos, level), crate::derived::org_modifiers(db, &self.org_benefits, level)];
            self.item_mods = crate::derived::modifiers(db, &self.worn, &self.perks, &self.nanos, &self.org_benefits, level);
            self.stats_changed = true;
            // A running polymorph program turns the character into a creature.
            let id = |kind: u32, instance: u32| mao_formats::resource::Id { kind, instance };
            let morph = self.nanos.iter().find_map(|&n| {
                let data = db.get(id(0xfde85, n)).ok().flatten()?;
                let t = mao_formats::item::Item::parse(&data).filter(|t| t.stat(75) == Some(mao_formats::item::POLYMORPH_STRAIN))?;
                let _ = t;
                let creature = mao_formats::item::polymorph_template(&data)?;
                db.get(id(0xfde97, creature)).ok().flatten().map(|_| creature)
            });
            match morph {
                Some(creature) => self.morphs.insert(self.own, creature),
                None => self.morphs.remove(&self.own),
            };
            // A running vehicle program puts the character on its hoverboard or hoverbike.
            let vehicle = self.nanos.iter().copied().find(|&n| db.get(id(0xfde85, n)).ok().flatten().is_some_and(|d| mao_formats::item::is_vehicle_program(&d)));
            match vehicle {
                Some(nano) => self.riding.insert(self.own, nano),
                None => self.riding.remove(&self.own),
            };
        }
    }

    /// The number of a door from its id: `0xc0 | number << 16 | playfield` outdoors, and the ids of an instance's doors run on from
    /// the lowest one (the shop's seven are `0x10c1a5a5` to `0x10c1a5ab`, in the table's order).
    fn door_number(&self, id: u32) -> Option<u8> {
        if id >> 24 == 0xc0 {
            return Some((id >> 16) as u8);
        }
        // the run of consecutive ids this one is in
        let mut first = id;
        while first > 0 && self.door_ids.contains(&(first - 1)) {
            first -= 1;
        }
        self.door_ids.contains(&id).then(|| u8::try_from(id - first).ok()).flatten()
    }

    /// Makes each announced door a usable object at its place.
    fn register_doors(&mut self) {
        let ids: Vec<u32> = self.door_ids.iter().copied().collect();
        for id in ids {
            let Some((_, position)) = self.door_number(id).and_then(|n| self.door_slots.get(&n).copied()) else { continue };
            self.objects.insert(id, position);
            self.names.entry(id).or_insert_with(|| ("Door".to_string(), false));
        }
    }

    /// Opens the door that was used, if the id is one.
    fn door_used(&mut self, id: u32) {
        if self.types.get(&id) == Some(&DOOR_TYPE) {
            if let Some(n) = self.door_number(id) {
                self.doors_open.insert(n, Instant::now() + DOOR_OPEN);
            }
        }
    }

    /// Takes the playfield's built-in props: the doors are also remembered by number.
    fn set_static_props(&mut self, props: Vec<(u32, (u32, [f32; 3], [f32; 4]), Option<u8>)>) {
        self.door_slots.clear();
        self.doors_open.clear();
        for (key, prop, door) in props {
            if let Some(n) = door {
                self.door_slots.insert(n, (key, prop.1));
            }
            self.props.insert(key, prop);
        }
        self.register_doors();
    }

    /// Where an entity or usable object is.
    fn position_of(&self, id: u32) -> Option<[f32; 3]> {
        self.entities.get(&id).map(|e| e.0).or_else(|| self.objects.get(&id).copied())
    }

    /// Takes a list of (stat, value) pairs of a parsed message into the character's stats.
    fn read_stats(&mut self, list: Option<&mao_net::schema::Value>) {
        let Some(mao_net::schema::Value::List(items)) = list else { return };
        for item in items {
            if let (Some(s), Some(v)) = (item.get("Item1").and_then(mao_net::schema::Value::as_int), item.get("Item2").and_then(mao_net::schema::Value::as_int)) {
                if std::env::var_os("MAO_STATS").is_some() && matches!(s, 1 | 11 | 27 | 214 | 221) {
                    eprintln!("stat {s} = {v} (was {:?})", self.stats.get(&(s as u32)));
                }
                if self.stats.insert(s as u32, v) != Some(v) {
                    self.stats_changed = true;
                }
            }
        }
    }

    fn add(&mut self, frame: &frame::Frame) {
        let Some(m) = Message::parse(frame) else { return };
        if let Some(arrival) = m.arrival() {
            eprintln!("playfield message: identity ({:x},{:x}) -> playfield {} at {:?}", m.identity.0, m.identity.1, arrival.0, arrival.1);
            self.arrival = Some(arrival);
        }
        if let Some((seconds, period, day, server_time)) = m.game_time() {
            self.game_clock.set(seconds);
            self.game_clock.set_day(day);
            self.game_clock.set_server_time(server_time as i32);
            let (hour, minute) = self.game_clock.time();
            eprintln!("game time: {hour:02}:{minute:02} (period {period}, day {day}, server time {server_time})");
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
                    // The machine's model is the `Mesh` stat (12); the booth faces the way its rotation says.
                    let mesh = match p.value.get("Stats") {
                        Some(mao_net::schema::Value::List(stats)) => stats.iter().find(|s| s.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(12)).and_then(|s| s.get("Item2")?.as_int()),
                        _ => None,
                    };
                    if let Some(mesh) = mesh.filter(|&m| m > 0) {
                        let r = |n: &str| match p.value.get("Rotation").and_then(|q| q.get(n)) {
                            Some(mao_net::schema::Value::Float(x)) => *x as f32,
                            _ => 0.0,
                        };
                        self.props.insert(m.identity.1, (mesh as u32, [f("X"), f("Y"), f("Z")], [r("X"), r("Y"), r("Z"), r("W")]));
                    }
                    // Shops can be targeted (Ctrl+Tab) and used (E) like the other objects.
                    self.objects.insert(m.identity.1, [f("X"), f("Y"), f("Z")]);
                    self.names.entry(m.identity.1).or_insert_with(|| (format!("Booth {:x}", m.identity.1 & 0xffff), false));
                }
            }
        }
        // A missed attack at or by the character.
        if m.kind == 0x5c654b28 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let id = |name: &str| p.value.get(name).and_then(|v| v.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let (attacker, defender) = (id("Attacker"), id("Defender"));
                // (a miss swings the weapon too: FUN_100a1077 → FUN_1006b399 → FUN_1006ae3c with the place in `Unknown2`)
                let slot = p.value.get("Unknown2").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let n = self.swings.get(&attacker).map_or(0, |s| s.2.wrapping_add(1));
                self.swings.insert(attacker, (slot, Instant::now(), n, 0xb));
                // (FUN_1006b399 → FUN_1006ae3c: no damage, no hit)
                self.attacks.insert((attacker, slot), (defender, 0, 0));
                // (the line: FUN_1006b399's feedback 0x3b, with the skill its stat names, `Unknown3`)
                let skill = p.value.get("Unknown3").and_then(mao_net::schema::Value::as_int).filter(|&s| s != 0).map(|s| crate::skills::stat_name(s as u32).to_string());
                if let Some(line) = crate::feedback::miss(self.who(defender), self.who(attacker), skill.as_deref()) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                }
            }
        }
        // A character dies (`CharacterAction` 99 from it, the death's action in `Parameter2`: `CharDie_t`, FUN_1007b803, has the
        // controller play it, its +0x14, FUN_1003cc42) and goes (`Despawn`).
        if m.kind == 0x5e477770 && m.data.len() >= 25 && u32::from_be_bytes(m.data[1..5].try_into().unwrap()) == 99 {
            let action = u32::from_be_bytes(m.data[21..25].try_into().unwrap());
            self.dead.insert(m.identity.1, (action, Instant::now()));
            self.fighting.remove(&m.identity.1);
            self.died.push(m.identity.1);
        }
        if m.kind == 0x36510078 {
            self.dead.remove(&m.identity.1);
        }
        // The hazards (`hazards`): their full updates, a controller's state, and what befalls a mine or trap.
        match m.kind {
            crate::hazards::MINE_FULL_UPDATE | crate::hazards::TRAP_FULL_UPDATE | crate::hazards::CONTROLLER_FULL_UPDATE => {
                self.hazards.full_update(m.kind, m.identity.1, &m.data);
            }
            crate::hazards::CONTROLLER_STATE => self.hazards.state(m.identity.1, &m.data),
            // (a script event: FUN_10130a1b, run as it comes)
            crate::sounds::SCRIPT_IIR => {
                if let Some(iir) = crate::sounds::ScriptIir::parse(&m.data) {
                    self.script_events.push(iir);
                }
            }
            crate::hazards::ACTION => {
                let out = self.hazards.action(m.identity.1, &m.data, self.own);
                self.hazard_sounds.extend(out.sounds);
                self.placed_effects.extend(out.effects);
                if let Some(name) = out.line {
                    // (FUN_10059047: the feedback told as it is)
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, crate::feedback::plain(name)));
                }
            }
            _ => {}
        }
        // The jingles: a new level (`NewLevel`, FUN_10075f55: `SM_Sandy_Game_Level_Complete`), a new shadow level
        // (`ShadowLevel`, FUN_10077ae7: `..._Neutral`, `..._Clan` or `..._Omni` by the side, stat 33) and a mission done
        // (`CharacterAction` 59, FUN_1005d61f's case 0xf: `SM_Sandy_Gui_Mission_Complete_Neutral`, `_Clan`, `_Omni` by the
        // side, else `SM_Sandy_Gui_Mission_Complete`), each the client's own character's, at no place.
        if m.identity.1 == self.own {
            let side = self.stats.get(&33).copied().unwrap_or(0);
            let jingle = match m.kind {
                0x7f405a16 => Some("SM_Sandy_Game_Level_Complete"),
                0x3c1e4e03 => match side {
                    0 => Some("SM_Sandy_Game_Level_Complete_Neutral"),
                    1 => Some("SM_Sandy_Game_Level_Complete_Clan"),
                    2 => Some("SM_Sandy_Game_Level_Complete_Omni"),
                    _ => None,
                },
                0x5e477770 if m.data.len() >= 5 && u32::from_be_bytes(m.data[1..5].try_into().unwrap()) == 59 => Some(match side {
                    0 => "SM_Sandy_Gui_Mission_Complete_Neutral",
                    1 => "SM_Sandy_Gui_Mission_Complete_Clan",
                    2 => "SM_Sandy_Gui_Mission_Complete_Omni",
                    _ => "SM_Sandy_Gui_Mission_Complete",
                }),
                _ => None,
            };
            if let Some(name) = jingle {
                let id = mao_formats::sound::sound_id(name);
                crate::audio::with(|m| m.play_game_sound(id, [0.0; 3], 0.0, 1.0, 0, 0.0, 100, 1, 0, [0.0; 3]));
            }
        }
        // A character starts fighting (`Attack`, from it: its target) and stops (`StopFight`, from it).
        if m.kind == 0x28494070 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let target = p.value.get("Target").and_then(|t| t.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                self.fighting.insert(m.identity.1, target);
            }
        }
        if m.kind == 0x4a41203e {
            self.fighting.remove(&m.identity.1);
        }
        // An attack (`AttackInfo`, from the attacker): it swings the weapon in the place the message names (FUN_1009f257 →
        // FUN_1006ae3c → FUN_1006a782, which `Characters::ensure_swing` follows).
        if m.kind == 0x46002f16 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let slot = p.value.get("WeaponSlot").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let n = self.swings.get(&m.identity.1).map_or(0, |s| s.2.wrapping_add(1));
                self.swings.insert(m.identity.1, (slot, Instant::now(), n, 0xb));
                // (the line: FUN_1009b6b7's feedback, the damage type the attacking weapon's, FUN_1009b525)
                let int = |name: &str| p.value.get(name).and_then(mao_net::schema::Value::as_int).unwrap_or(0);
                let target = p.value.get("Target").and_then(|t| t.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                // (FUN_1006ae3c: the place's weapon given the damage and hit type, and its Energy the rounds left)
                self.attacks.insert((m.identity.1, slot), (target, int("HitType") as i32, int("Amount")));
                if let Some(e) = self.weapon_energy.get_mut(&(m.identity.1, slot as u8)) {
                    *e = int("AmmoCount");
                }
                let kind = crate::feedback::weapon_damage_type(self.weapons.get(&(m.identity.1, slot as u8)).and_then(|&t| template_stat(t, 436)));
                if let Some(line) = crate::feedback::attack(self.who(target), self.who(m.identity.1), int("Amount"), kind, int("HitType")) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                }
            }
        }
        // A special attack (`SpecialAttackInfo`, from the attacker: FUN_100a1f10 → FUN_1006af0e → FUN_1006a782 with the special
        // flag): the weapon in `EquipSlot` swings the special's class; only the client's own character has a queued special
        // (FUN_10068aa3 reads +0x9c), the others swing the plain attack. The specials that take their own item (Brawl,
        // Dimach) find none here and swing the weapon's plain attack, as the original does when FUN_10068c19 has no item.
        if m.kind == 0x754f1115 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let slot = p.value.get("EquipSlot").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let stat = p.value.get("Stat").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let class = match mao_formats::item::special_class(stat) {
                    Some((class, false)) if m.identity.1 == self.own => class,
                    _ => 0xb,
                };
                let n = self.swings.get(&m.identity.1).map_or(0, |s| s.2.wrapping_add(1));
                self.swings.insert(m.identity.1, (slot, Instant::now(), n, class));
            }
        }
        // A weapon a character holds: its owner, its place and its template.
        if m.kind == 0x3b1d2268 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let owner = p.value.get("Owner").and_then(|o| o.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let weapon_id = m.identity.1;
                let slot = (p.value.get("Unknown2").and_then(mao_net::schema::Value::as_int).unwrap_or(0) & 0xff) as u8;
                let template = match p.value.get("Stats") {
                    Some(mao_net::schema::Value::List(stats)) => stats.iter().find(|s| s.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(702)).and_then(|s| s.get("Item2")?.as_int()),
                    _ => None,
                };
                if let Some(template) = template.filter(|&t| t > 0) {
                    self.weapons.insert((owner, slot), template as u32);
                    self.weapon_templates.insert(weapon_id, template as u32);
                }
                let energy = match p.value.get("Stats") {
                    Some(mao_net::schema::Value::List(stats)) => stats.iter().find(|s| s.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(26)).and_then(|s| s.get("Item2")?.as_int()),
                    _ => None,
                };
                match energy {
                    Some(e) => self.weapon_energy.insert((owner, slot), e),
                    None => self.weapon_energy.remove(&(owner, slot)),
                };
            }
        }
        // What a character wears: its clothes' textures and the meshes held at its body's attachment points.
        if m.kind == 0x41624f0d {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let list = |name: &str| match p.value.get(name) {
                    Some(mao_net::schema::Value::List(l)) => l.clone(),
                    _ => Vec::new(),
                };
                let field = |v: &mao_net::schema::Value, name: &str| v.get(name).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                if let Some(info) = self.infos.get_mut(&m.identity.1) {
                    for t in list("Textures") {
                        if let Some(part) = CLOTH_PARTS.get(field(&t, "Place") as usize) {
                            info.clothes.retain(|(n, _)| n != part);
                            if field(&t, "Id") != 0 {
                                info.clothes.push((part.to_string(), field(&t, "Id")));
                            }
                        }
                    }
                }
                let mut meshes: Vec<(u32, u8, u32, u32)> = list("Meshes").iter().map(|v| (field(v, "Layer"), field(v, "Position") as u8, field(v, "Id"), field(v, "OverrideTextureId"))).collect();
                meshes.sort();
                self.equipment.insert(m.identity.1, meshes.into_iter().map(|(_, at, mesh, texture)| (at, mesh, texture)).collect());
            }
        }
        if m.kind == 0x39343c68 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let nano = p.value.get("Buff").and_then(|b| b.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                if nano != 0 && m.identity.1 != self.own {
                    self.buffs_seen.push((m.identity.1, nano));
                    self.active_buffs.entry(m.identity.1).or_default().insert(nano, Instant::now());
                }
            }
        }
        if m.kind == 0x2e2a4a6b {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                if let Some(mao_net::schema::Value::Str(name)) = p.value.get("Name") {
                    if !name.is_empty() {
                        self.orgs.insert(m.identity.1, name.clone());
                    }
                }
            }
        }
        // A reload of the client's weapon (`Reload`, FUN_10076cc9 → FUN_100697bd): "Reloading..." now and "Reloading completed"
        // once the weapon's reload time, its stat 210, is over (FUN_10068fd2 → FUN_1006884a; the stat in hundredths of a
        // second, as the attack time 294 is). (A `StopFight` tells nothing: FUN_1007a2be only holsters.)
        if m.kind == 0x26515e61 && m.identity.1 == self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let weapon = p.value.get("WeaponIdentity").and_then(|w| w.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, crate::feedback::plain("Feedback_Reloading")));
                let delay = self.weapon_templates.get(&weapon).and_then(|&t| template_stat(t, 210)).unwrap_or(0).max(0) as u64;
                self.reload_done = Some(Instant::now() + Duration::from_millis(delay * 10));
            }
        }
        // Damage a character's armour absorbed (`Absorb`, FUN_1009ef78: from the one absorbing).
        if m.kind == 0x264e5f61 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let amount = p.value.get("Amount").and_then(mao_net::schema::Value::as_int).unwrap_or(0);
                let stat = p.value.get("DmgType").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                if let Some(line) = crate::feedback::absorb(self.who(m.identity.1), amount, stat) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                }
            }
        }
        // A damage shield (`ShieldAttack`, FUN_100a134d) or reflect shield (`ReflectAttack`, FUN_100a11c2) struck: the one hurt
        // is the message's identity (its health goes down by the amount), the shield's owner `Target`.
        if m.kind == 0x25192476 || m.kind == 0x1c3a4f77 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let amount = p.value.get("Amount").and_then(mao_net::schema::Value::as_int).unwrap_or(0);
                let owner = p.value.get("Target").and_then(|t| t.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let victim = m.identity.1;
                if let Some(hp) = self.last_hp.get_mut(&victim) {
                    *hp -= amount;
                    if let Some(entry) = self.health.get_mut(&victim) {
                        entry.0 = *hp;
                    }
                }
                if victim == self.own {
                    if let Some(hp) = self.stats.get_mut(&27) {
                        *hp -= amount;
                    }
                }
                if let Some(line) = crate::feedback::shield(self.who(victim), self.who(owner), amount, m.kind == 0x1c3a4f77) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                }
            }
        }
        // A nano cast: told when it is the character's own or is cast on it.
        if m.kind == 0x25314d6d {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let id = |name: &str| p.value.get(name).and_then(|v| v.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let nano = p.value.get("NanoId").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let (caster, target) = (id("Caster"), id("Target"));
                // An aura is cast again and again by the server while it runs: that is the nano staying on, not a new cast, so
                // the repeats say nothing, show nothing and play no casting animation.
                let mut repeat = false;
                if caster == target {
                    let asked = caster == self.own && self.asked_casts.get(&nano).is_some_and(|t| t.elapsed() < Duration::from_secs(3));
                    repeat = !asked && (self.self_casts.get(&(caster, nano)).is_some_and(|t| t.elapsed() < Duration::from_secs(600)) || (caster == self.own && self.nanos.contains(&nano)));
                    self.self_casts.insert((caster, nano), Instant::now());
                }
                if !repeat {
                    if caster == self.own || target == self.own {
                        self.casts.push((caster, target, nano));
                    }
                    let plays = self.nano_casts.cast(caster, target, nano, &|n, s| nano_stat(n, s));
                    self.cast_sounds.extend(plays.into_iter().map(|(id, delay, volume)| (caster, id, delay, volume)));
                    self.landed.push((target, nano));
                    self.cast_bursts.push((caster, target, nano));
                    self.shots_pending.push((caster, target, nano));
                    // The caster plays its casting animation (at itself or at someone).
                    self.socials.insert(caster, (if caster == target { "spell-self" } else { "spell-dir" }, Instant::now()));
                }
            }
        }
        // A character's social action (a dance, a bow): the last word of the message is the action's number.
        if m.kind == 0x3b290771 && m.data.len() >= 13 {
            let action = u32::from_be_bytes(m.data[m.data.len() - 4..].try_into().unwrap());
            if let Some(name) = crate::actions_data::SOCIAL.iter().find(|a| a.0 == action).map(|a| a.1) {
                self.socials.insert(m.identity.1, (name, Instant::now()));
            }
        }
        // The server times the character's special attacks: `SpecialUsed` (the stat and the seconds) and `SpecialAvailable` (the stat).
        if m.kind == 0x5e477770 && m.identity.1 == self.own && m.data.len() >= 25 {
            let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            match m.data[4] {
                0xaa => self.special_events.push((word(17), word(21) as f32)),
                // `SetNanoDuration`: the nano `(0xcf1b, id)` is on the character (the first parameter) for that long (ms)
                0x62 if word(9) == 0xcf1b && word(17) == self.own => self.nano_duration(word(13), word(21)),
                0xa4 => self.special_events.push((word(21), 0.0)),
                _ => {}
            }
        }
        if m.kind == 0x5a585f65 {
            self.inspected = Some((m.identity.1, mao_net::n3::inspect_items(&m.data)));
        }
        // An info packet: the health and its maximum of the character or creature it describes.
        if m.kind == 0x4d38242e {
            if let Some(info) = mao_net::n3::info_packet(&m.data) {
                self.health.insert(m.identity.1, (info.health as i64, info.max_health as i64));
                if !info.monster && info.level > 0 {
                    self.levels.insert(m.identity.1, info.level);
                }
                if self.inspecting == Some(m.identity.1) {
                    self.inspecting = None;
                    let name = self.names.get(&m.identity.1).map_or_else(|| "The target".to_string(), |n| n.0.clone());
                    let text = if info.monster {
                        format!("{name}: health {} of {}.", info.health, info.max_health)
                    } else {
                        format!("{name}: level {} {}, health {} of {}.", info.level, crate::launcher::profession(info.profession as i32), info.health, info.max_health)
                    };
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, text));
                }
            }
        }
        // The game's own text and the fight.
        if m.kind == 0x5f4b442a {
            if let Some(text) = mao_net::n3::chat_text(&m.data) {
                self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, text));
            }
        }
        if m.kind == 0x206b4b73 && m.identity.1 == self.own {
            if let Some(text) = mao_net::n3::feedback_text(&m.data) {
                self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, text));
            }
        }
        if m.kind == 0x3710256c {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let amount = p.value.get("Amount").and_then(mao_net::schema::Value::as_int).unwrap_or(0);
                let stat = p.value.get("Stat").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                // (FUN_100a0611: the one hurt is the message's own identity, its health `TargetHp`; `Target` is the source and the
                // last field, `Unk2`, an item template the hit came from)
                let target = m.identity.1;
                let source = p.value.get("Target").and_then(|t| t.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let nano = p.value.get("Unk2").and_then(mao_net::schema::Value::as_int).filter(|&n| n != 0).map(|n| template_name(n as u32).unwrap_or_else(|| format!("item #{n}")));
                self.damage.push((target, amount, stat));
                if let Some(hp) = p.value.get("TargetHp").and_then(mao_net::schema::Value::as_int) {
                    self.last_hp.insert(target, hp);
                    // (the original sets the stat itself, so a later `Stat` message with the same health tells no change)
                    if target == self.own {
                        self.stats.insert(27, hp);
                    }
                    // (the maximum stays; the health follows the fight)
                    if let Some(entry) = self.health.get_mut(&target) {
                        entry.0 = hp;
                    }
                }
                if let Some(line) = crate::feedback::health(self.who(target), self.who(source), amount, stat, nano.as_deref()) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                }
            }
        }
        // A container the character used lists its contents; the server moves an item out with a `ContainerAddItem`; a corpse that
        // despawns closes the window.
        if m.kind == INVENTORY_UPDATE && m.identity.1 == self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let part = |name: &str, field: &str| p.value.get(name).and_then(|v| v.get(field)).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                let identity = (part("InventoryIdentity", "Type"), part("InventoryIdentity", "Instance"));
                let entries = inventory_from_schema_items(p.value.get("Items"));
                if entries.is_empty() {
                    if self.loot.as_ref().is_some_and(|l| l.identity == identity) {
                        self.loot = None;
                        self.loot_changed = true;
                    }
                } else if identity.0 != 0 {
                    let handle = p.value.get("Handle").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                    if identity.0 == 0xc76a {
                        self.fresh_corpse = Some((handle, entries.iter().map(|e| e.slot).collect()));
                    }
                    self.loot = Some(Loot { identity, handle, entries });
                    self.loot_changed = true;
                }
            }
        }
        if m.kind == CONTAINER_ADD_ITEM && m.identity.1 == self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let source = (p.value.get("Source").and_then(|v| v.get("Type")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32, p.value.get("Source").and_then(|v| v.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32);
                let target = (p.value.get("Target").and_then(|v| v.get("Type")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32, p.value.get("Target").and_then(|v| v.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32);
                // `(0x68, place)` into the open container: an item went in from the backpack
                if source.0 == 0x68 && self.loot.as_ref().is_some_and(|l| l.identity == target) {
                    if let Some(at) = self.worn.iter().position(|e| e.slot == source.1) {
                        let mut entry = self.worn.remove(at);
                        if let Some(loot) = self.loot.as_mut() {
                            entry.slot = loot.entries.iter().map(|e| e.slot + 1).max().unwrap_or(0);
                            loot.entries.push(entry);
                        }
                        self.inventory = Some(self.worn.clone());
                        self.loot_changed = true;
                    }
                }
                // `(0x6b, handle << 16 | place)`: an item came out of the open container into the backpack
                if source.0 == 0x6b {
                    let place = source.1 & 0xffff;
                    if let Some(loot) = self.loot.as_mut() {
                        if let Some(at) = loot.entries.iter().position(|e| e.slot == place) {
                            let mut entry = loot.entries.remove(at);
                            if let Some(free) = (inventory::BACKPACK_FIRST..inventory::BACKPACK_FIRST + crate::gear::PACK_SLOTS).find(|s| !self.worn.iter().any(|e| e.slot == *s)) {
                                entry.slot = free;
                                self.worn.push(entry);
                                self.inventory = Some(self.worn.clone());
                            }
                            self.loot_changed = true;
                        }
                    }
                }
            }
        }
        if m.kind == 0x36510078 && self.loot.as_ref().is_some_and(|l| l.identity == m.identity) {
            self.loot = None;
            self.loot_changed = true;
        }
        if m.kind == DOOR_STATUS && m.identity.0 == DOOR_TYPE && self.door_ids.insert(m.identity.1) {
            self.register_doors();
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
        if m.kind == 0x253d0240 && std::env::var_os("MAO_STATS").is_some() {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                if let Some(mao_net::schema::Value::List(lines)) = p.value.get("ResearchLine") {
                    let nonzero: Vec<_> = lines.iter().filter_map(|l| Some((l.get("ResearchId")?.as_int()?, l.get("Unknown1")?.as_int()?, l.get("Unknown2")?.as_int()?, l.get("Unknown3")?.as_int()?))).filter(|l| l.1 != 0 || l.2 != 0 || l.3 != 0).collect();
                    eprintln!("research update from {:x}:{:x}: {} lines, non-zero {nonzero:?}", m.identity.0, m.identity.1, lines.len());
                }
            }
        }
        // An organization benefit: type 6, then (after the organization's identity) a quality and a low and high item template.
        if m.kind == 0x64582a07 && m.identity.1 == self.own && m.data.len() >= 31 && m.data[..2] == [0, 6] {
            let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            let entry = inventory::Entry { slot: 33, flags: 1, count: 1, container: (0, 0), low_id: word(22), high_id: word(26), quality: word(18), extra: 0 };
            if !self.org_benefits.iter().any(|e| e.low_id == entry.low_id && e.high_id == entry.high_id && e.quality == entry.quality) {
                self.org_benefits.push(entry);
                self.mods_dirty = true;
            }
        }
        if m.kind == mao_net::n3::ENTITY_INFO && m.identity.1 == self.own {
            let times = m.active_nano_times();
            let nanos: Vec<u32> = times.iter().map(|t| t.0).collect();
            if !times.is_empty() || !self.nanos.is_empty() {
                self.nano_times = times;
                self.nano_times_at = Some(Instant::now());
                self.nano_list_changed = true;
            }
            if nanos != self.nanos {
                self.nanos = nanos;
                self.mods_dirty = true;
            }
        }
        if m.kind == inventory::WORLD_ENTRY && m.identity.1 == self.own {
            self.inventory = Some(inventory::parse(&m.data));
            // The schema reads every item list (the hand reader stops at the first entry whose flags it does not know).
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let from_schema = inventory_from_schema(p.value.get("InventorySlots"));
                if from_schema.len() > self.inventory.as_ref().map_or(0, Vec::len) {
                    self.inventory = Some(from_schema);
                }
            }
            if std::env::var_os("MAO_STATS").is_some() {
                let parsed = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data);
                eprintln!("world entry: {} bytes, {} items by the item reader, schema: {}", m.data.len(), self.inventory.as_ref().map_or(0, Vec::len), match &parsed { Ok(p) => format!("ok, {} bytes left, slots {}", p.left, match p.value.get("InventorySlots") { Some(mao_net::schema::Value::List(l)) => l.len(), _ => 0 }), Err(e) => format!("error {e}") });
            }
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                for list in ["Stats1", "Stats2", "Stats3", "Stats4"] {
                    self.read_stats(p.value.get(list));
                }
                self.perks = perk_ids(p.value.get("Perks"));
                self.perks_changed = true;
                if let Some(mao_net::schema::Value::List(ids)) = p.value.get("UploadedNanoIds") {
                    self.program_ids = ids.iter().filter_map(mao_net::schema::Value::as_int).map(|v| v as u32).collect();
                    self.programs_changed = true;
                }
                self.mods_dirty = true;
            }
        }
        // A stat of the character changed.
        // Another character's health in a `Stat` (FUN_100a2006): held, and a drop told as the original tells it (0x1b, no source:
        // "... was attacked with nanobots for N points of unknown damage.").
        if m.kind == 0x2b333d6e && m.identity.1 != self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                if let Some(mao_net::schema::Value::List(items)) = p.value.get("Stats") {
                    let health = items.iter().filter(|i| i.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(27)).filter_map(|i| i.get("Item2")?.as_int()).last();
                    if let Some(hp) = health {
                        let id = m.identity.1;
                        if let Some(old) = self.last_hp.insert(id, hp) {
                            if let Some(line) = crate::feedback::health(self.who(id), crate::feedback::Who::Unknown, (hp - old).min(0), 0, None) {
                                self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                            }
                        }
                        if let Some(entry) = self.health.get_mut(&id) {
                            entry.0 = hp;
                        }
                    }
                }
            }
        }
        if m.kind == 0x2b333d6e && m.identity.1 == self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                // (FUN_100a2006 tells the change of the health, experience, alien experience and shadowknowledge from the value
                // held; a stat not held yet has no change to tell)
                let before: Vec<(u32, Option<i64>)> = [27u32, 52, 40, 573].iter().map(|&s| (s, self.stats.get(&s).copied())).collect();
                self.read_stats(p.value.get("Stats"));
                for (stat, old) in before {
                    let (Some(old), Some(&new)) = (old, self.stats.get(&stat)) else { continue };
                    if let Some(line) = crate::feedback::own_stat(stat, new - old) {
                        self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                    }
                }
            }
        }
        if let Some(q) = m.rotation() {
            self.rotations.insert(m.identity.1, q);
        }
        if let Some(info) = m.entity_info() {
            self.dead.remove(&m.identity.1);
            match info.fighting {
                Some((_, target)) => {
                    self.fighting.insert(m.identity.1, target);
                }
                None => {
                    self.fighting.remove(&m.identity.1);
                }
            }
            let player = info.is_player();
            self.names.insert(m.identity.1, (info.name.clone(), player));
            self.infos.insert(m.identity.1, info);
        }
        if let Some((kind, q, _)) = m.movement() {
            self.rotations.insert(m.identity.1, q);
            if kind == mao_net::n3::movement::JUMP {
                self.jumped.insert(m.identity.1, Instant::now());
            }
        }
        let position = m.position().or_else(|| m.movement().map(|(_, _, p)| p));
        if let Some(p) = position {
            let e = self.entities.entry(m.identity.1).or_insert((p, 0));
            let moved = ((e.0[0] - p[0]).powi(2) + (e.0[2] - p[2]).powi(2)).sqrt();
            if moved > 0.05 {
                self.last_move.insert(m.identity.1, Instant::now());
                // (moving against the facing is backing: the run-back and walk-back animations)
                if let Some(q) = self.rotations.get(&m.identity.1) {
                    let h = 2.0 * q[1].atan2(q[3]);
                    let along = (p[0] - e.0[0]) * h.sin() + (p[2] - e.0[2]) * h.cos();
                    self.backing.insert(m.identity.1, along < -0.3 * moved);
                }
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
            println!("  info {id:>10} {:<18} flags {:#010x} flags2 {:#010x} breed {} sex {} a0 {:?} template {:?} textures {} clothes {:?}", i.name, i.flags, i.flags2, i.breed, i.sex, i.unknown_a0, i.template.map(|t| format!("{t:#x}")), i.textures.len(), i.clothes);
        }
        println!("{} usable objects:", self.objects.len());
        for (id, p) in &self.objects {
            println!("  object {id:>10x} type {:#x} at ({:.1}, {:.1}, {:.1})", self.types.get(id).copied().unwrap_or(0), p[0], p[1], p[2]);
        }
        println!("{} entities with positions:", self.entities.len());
        for (id, (p, _)) in &self.entities {
            let (name, player) = self.names.get(id).map(|(n, p)| (n.as_str(), *p)).unwrap_or(("?", false));
            println!("  {id:>10} {name:<20} {} type {:#x} at ({:.1}, {:.1}, {:.1})", if player { "player" } else { "      " }, self.types.get(id).copied().unwrap_or(0), p[0], p[1], p[2]);
        }
    }
}


/// A flat floor under an interior whose own floor this client can't read (its ground is of the unread tile-map
/// layout): a square of 200 units around (`x`, `z`) at `height`, drawn from both sides.
fn floor_part(x: f32, z: f32, height: f32) -> Part {
    let corners = [(-100.0, -100.0), (100.0, -100.0), (100.0, 100.0), (-100.0, 100.0)];
    let vertices = corners.iter().map(|(dx, dz)| Vertex { position: [x + dx, height, z + dz], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] }).collect();
    let colour = [0.42, 0.40, 0.38];
    let material = Material { name: "floor".into(), diffuse: colour, specular: [0.0; 3], ambient: colour, emissive: [0.0; 3], opacity: 1.0, textures: Vec::new(), ignore_alpha: false, alpha_lights: false, foliage: false };
    Part { name: "floor".into(), world: model::IDENTITY, vertices, indices: vec![0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2], material }
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
    let material = Material { name: "marker".into(), diffuse: colour, specular: [0.0; 3], ambient: colour, emissive: [0.0; 3], opacity: 1.0, textures: Vec::new(), ignore_alpha: false, alpha_lights: false, foliage: false };
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
    /// Instances looked up by (type, name).
    ids: std::collections::HashMap<(u32, String), Option<u32>>,
    /// By (breed, sex): the body, its head and the head's attractor.
    bodies: std::collections::HashMap<(u8, u8), Option<(mao_formats::catmesh::CatMesh, Model, Option<mao_formats::catmesh::Attractor>)>>,
    /// Creatures by NPC template: the template and its CAT mesh.
    creatures: std::collections::HashMap<u32, Option<(mao_formats::npc::Template, mao_formats::catmesh::CatMesh, Option<Model>)>>,
    /// Creature animations by instance.
    creature_anims: std::collections::HashMap<u32, Option<(mao_formats::catanim::CatAnim, f32)>>,
    /// Where each character is in its movement animation (start, loop, stop).
    motions: std::collections::HashMap<u32, Motion>,
    /// The segment `motion` last chose (0 idle, 1 start, 2 cycle, 3 stop, 10 jump, 11 landing).
    segment: u8,
    /// Each character's running animations, as `VisualCATMesh_t` keeps them (`mao_formats::animstack`), and when they were
    /// last run on.
    stacks: std::collections::HashMap<u32, (mao_formats::animstack::Stack, Instant)>,
    /// The animations the stacks play, by instance, with their lengths.
    anims_by_id: std::collections::HashMap<u32, (Arc<mao_formats::catanim::CatAnim>, f32)>,
    /// The notes the stacks' last runs went past (FUN_1003c248): the character, the note's event and the animation set it has
    /// (its NPC template), for `sounds::note`.
    pub notes: Vec<(u32, i32, Option<u32>)>,
    /// What each character's movement was last set to (the action and segment, or a creature's animation) and its handle.
    movement_keys: std::collections::HashMap<u32, (mao_formats::appearance::Action, u8, u32)>,
    movement_handles: std::collections::HashMap<u32, u32>,
    /// When each character's current social action was set on its stack.
    socials_started: std::collections::HashMap<u32, Instant>,
    /// When each character's last attack swing was set on its stack.
    swings_started: std::collections::HashMap<u32, Instant>,
    /// The weapon each character had drawn when last drawn (template and place), to play the draw and the holster.
    drawn: std::collections::HashMap<u32, Option<(u32, u32)>>,
    /// The weapon each character held when last drawn, in or out of a fight, to play the wield and the unwield.
    wielded: std::collections::HashMap<u32, Option<(u32, u32)>>,
    started: Instant,
    /// Machine models by mesh instance.
    props: std::collections::HashMap<u32, Option<Vec<Part>>>,
    /// Worn static meshes (hair, helmets, weapons, shoulder pads) by instance.
    statics: std::collections::HashMap<u32, Option<Model>>,
    /// The player head meshes (`head_solitusmale12.abiff` and so on): an appearance update that names one at position 0 replaces
    /// the breed's default head.
    head_meshes: std::collections::HashSet<u32>,
    /// The vehicle body (an NPC template) a vehicle program gives each body, by (program, body CAT mesh).
    vehicles: std::collections::HashMap<(u32, u32), Option<u32>>,
    /// Each character's attractors on its last posed body, by name: frames in its local space (Y up, the body's scale
    /// applied), for the effects attached to them (`effects::Locator`).
    frames: std::collections::HashMap<u32, Vec<(String, model::Matrix)>>,
}

/// An NPC template and its CAT mesh and head (the creature or the vehicle body a character is drawn as).
/// The events of the notes a stack's last run went past (FUN_1003c248: each animation's notes from FUN_1003c0b7's first to last,
/// within its range; an event of 0 is none).
fn fired_notes(stack: &mao_formats::animstack::Stack, anims: &std::collections::HashMap<u32, (Arc<mao_formats::catanim::CatAnim>, f32)>) -> Vec<i32> {
    let mut out = Vec::new();
    for e in &stack.entries {
        let Some((anim, _)) = anims.get(&e.id) else { continue };
        if anim.notes.is_empty() {
            continue;
        }
        let times: Vec<f32> = anim.notes.iter().map(|n| n.time).collect();
        out.extend(e.fired(&times).into_iter().map(|k| anim.notes[k].event()).filter(|&event| event != 0));
    }
    out
}

impl Characters {
    /// A creature's animation set (its NPC template), if loaded.
    fn set_of(&self, template: u32) -> Option<&mao_formats::npc::Template> {
        self.creatures.get(&template)?.as_ref().map(|c| &c.0)
    }
}

/// The startup music played once a run.
static MUSIC_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn load_template(db: &ResourceDatabase, template: u32) -> Option<(mao_formats::npc::Template, mao_formats::catmesh::CatMesh, Option<Model>)> {
    use mao_formats::resource::Id;
    let record = mao_formats::npc::Template::parse(&db.get(Id { kind: mao_formats::npc::TEMPLATE, instance: template }).ok()??)?;
    let mesh = mao_formats::catmesh::parse(&db.get(Id { kind: 0xf6952, instance: record.mesh? }).ok()??).ok()?;
    // A humanoid's head is a separate mesh on the body's head attractor.
    let head = record.head.filter(|&h| h != 0).and_then(|h| crate::load_model_attached(db, Id { kind: 0xf6951, instance: h }).ok());
    Some((record, mesh, head))
}

/// Whether a static mesh's file name is a player head (`head_opifexfemale37.abiff`, `head_athrox05.abiff`).
fn is_head_name(name: &str) -> bool {
    let name = name.to_lowercase();
    ["head_solitus", "head_opifex", "head_nano", "head_athrox", "head_oldguy"].iter().any(|p| name.starts_with(p)) && name.ends_with(".abiff")
}

/// A further factor on a polymorphed character's creature animation (1: as the original's calibration file has it, see
/// `effects::calibration`; the user found the uncalibrated speed a little fast).
const MORPH_ANIM_RATE: f32 = 1.0;

/// The body parts a texture place of an `AppearanceUpdate` names (the cloth list's order).
const CLOTH_PARTS: [&str; 6] = ["hands", "body", "feet", "arms", "legs", "head"];

/// The attachment point of a worn mesh: its position counts from the body's `Attractor01_head`
/// (head, right hand, left hand, right shoulder, left shoulder, back).
fn attractor_name(position: u8) -> String {
    format!("Attractor{:02}", position as u32 + 1)
}

/// The worn static meshes of a character held at their attachment points of the posed body.
/// `gear` is (attachment position, static mesh instance, replacement texture instance or 0).
pub(crate) fn wear(db: &ResourceDatabase, cache: &mut std::collections::HashMap<u32, Option<Model>>, body: &mao_formats::catmesh::CatMesh, bones: &[mao_formats::catmesh::BoneMatrix], gear: &[(u8, u32, u32)]) -> Vec<Part> {
    use mao_formats::resource::Id;
    let mut parts = Vec::new();
    for &(position, mesh, texture) in gear {
        let Some(point) = body.attractor(&attractor_name(position)) else { continue };
        let piece = cache.entry(mesh).or_insert_with(|| crate::load_model_attached(db, Id { kind: 0xf6951, instance: mesh }).ok());
        if let Some(piece) = piece {
            let mut worn = body.attach(bones, point, piece);
            if texture != 0 {
                for part in &mut worn {
                    part.material.textures = vec![(0xf6954, texture)];
                }
            }
            parts.extend(worn);
        }
    }
    parts
}

/// The parts of a movement animation: the original's walk and run animations carry `loopstart` and `loopend`
/// notes: from the start to `loopstart` is the first step, between the notes is the cycle, the rest is the stop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Idle,
    Start,
    Loop,
    Stop,
}

#[derive(Clone, Copy)]
struct Motion {
    stage: Stage,
    action: mao_formats::appearance::Action,
    /// How far into the stage, in the animation's ms (advanced at the playback rate).
    clock: f32,
    tick: Instant,
}

/// How long a jump is in the air (twice the rise time) and how long the landing plays, in ms.
const AIR_MS: f32 = 1400.0;
const LAND_MS: f32 = 350.0;
/// The playback rate of a movement animation, as the original's FUN_1007009f: the calibration factor of the mesh and animation
/// (`Setupf/animcalibration.txt`), times 100 over the character's scale (stat 360), times the speed over the action's reference
/// speed; over 1.3 it is held at 1.3 when the character moves faster than 4 units a second.
fn movement_rate(calibration: f32, scale: f32, speed: f32, reference: f32) -> f32 {
    let rate = calibration * (100.0 / scale.max(1.0)) * (speed / reference);
    if rate > 1.3 && speed > 4.0 {
        1.3
    } else {
        rate
    }
}

/// The speeds a character moves at, in units a second, as `CharVehicle_t` sets them (FUN_1006f9eb, FUN_1006f2fc): running
/// forward `RunSpeed / 275 + 5` within 1.5 and 13, running backwards `RunSpeed / 392.86 + 3` within 1.05 and 9.1, where
/// `RunSpeed` (stat 156) falls off below 15% health (`ratio * (RunSpeed + 1000) - 1000`); walking is 1.5.
pub(crate) fn movement_speeds(stats: &BTreeMap<u32, i64>) -> (f32, f32) {
    let stat = |n: u32| stats.get(&n).copied().unwrap_or(0) as f32;
    let (run_speed, health, max_health) = (stat(156), stat(27), stat(1));
    let ratio = if max_health > 0.0 { health / (max_health * 0.15) } else { 1.0 };
    let effective = if ratio < 1.0 { ratio * (run_speed + 1000.0) - 1000.0 } else { run_speed };
    ((effective / 275.0 + 5.0).clamp(1.5, 13.0), (effective / 392.857_1 + 3.0).clamp(1.05, 9.1))
}

impl Characters {
    fn new(db: Arc<ResourceDatabase>) -> Result<Characters, String> {
        use mao_formats::resource::Id;
        let get = |kind: u32, instance: u32| db.get(Id { kind, instance }).map_err(|e| e.to_string())?.ok_or_else(|| format!("no object {kind:#x}:{instance:#x}"));
        let index = mao_formats::instances::Instances::parse(&get(0xf424a, 1)?).map_err(|e| e.to_string())?;
        let _ = get;
        let head_meshes = index.iter().filter(|(kind, name, _)| *kind == 0xf6951 && is_head_name(name)).map(|(_, _, instance)| instance).collect();
        Ok(Characters { head_meshes, db, index, anims: Default::default(), ids: Default::default(), bodies: Default::default(), creatures: Default::default(), creature_anims: Default::default(), motions: Default::default(), segment: 0, stacks: Default::default(), anims_by_id: Default::default(), notes: Vec::new(), movement_keys: Default::default(), movement_handles: Default::default(), socials_started: Default::default(), swings_started: Default::default(), drawn: Default::default(), wielded: Default::default(), started: Instant::now(), props: Default::default(), statics: Default::default(), vehicles: Default::default(), frames: Default::default() })
    }

    /// The parts of a machine's model (Y-up local space), loaded once.
    fn prop_parts(&mut self, mesh: u32) -> Option<&Vec<Part>> {
        let db = self.db.clone();
        self.props.entry(mesh).or_insert_with(|| crate::load_model_dynel(&db, mao_formats::resource::Id { kind: 0xf6951, instance: mesh }).ok().map(|m| m.parts)).as_ref()
    }

    /// The parts of a creature or NPC from its template (the CAT mesh and
    /// animations its record names), in Y-up local space.
    /// A dead one (`dying`: the death's action) plays its death once and lies in its last pose: the template's animation for the
    /// action (FUN_1003cac9 asks the creature's own set), else along FUN_10010b3c's fallbacks (`die-knees` to `die-pain`).
    fn creature_parts(&mut self, id: u32, template: u32, action: mao_formats::appearance::Action, _phase: f32, rate: f32, textures: &[(String, u32)], clothes: &[(String, u32)], deform: Option<(&mao_formats::gfx::Deform, f32)>, dying: Option<u32>, swing: Option<(u32, u32, Instant, u32, u32)>) -> Option<Vec<Part>> {
        use mao_formats::resource::Id;
        let db = self.db.clone();
        let entry = self.creatures.entry(template).or_insert_with(|| load_template(&db, template));
        let (record, mesh, head) = entry.as_ref()?;
        // (an action with variants: the one this character picks, standing in for FUN_10045976's `rand()` at each start)
        let seed = id;
        let death = dying.and_then(|mut a| {
            for _ in 0..8 {
                if let Some(anim) = record.variant(a, seed) {
                    return Some((a, anim));
                }
                a = mao_formats::appearance::action_fallback(a);
                if a == 0 {
                    break;
                }
            }
            None
        });
        // (the first of the action's animations the database really holds)
        let mut chosen = None;
        let candidates = match death {
            Some(d) => vec![d],
            None => record.animation_candidates(action, seed),
        };
        for (key, id) in candidates {
            let loaded = self.creature_anims.entry(id).or_insert_with(|| {
                let anim = mao_formats::catanim::parse(&db.get(Id { kind: 0xf6953, instance: id }).ok()??).ok()?;
                let length = anim.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0)).fold(1.0f32, f32::max);
                Some((anim, length))
            });
            if loaded.is_some() {
                chosen = Some((key, id));
                break;
            }
        }
        let (key, anim_id) = chosen?;
        let anim_entry = self.creature_anims.get(&anim_id)?;
        let (anim, length) = anim_entry.as_ref()?;
        // (a creature that has no standing animation holds its walk's first frame while it stands)
        // (the original's calibration file says how fast each movement animation of a creature plays)
        let rate = rate * crate::effects::calibration(record.mesh.unwrap_or(0), anim_id);
        let hold = death.is_none() && action == mao_formats::appearance::Action::Idle && matches!(key, 100 | 101);
        if !self.anims_by_id.contains_key(&anim_id) {
            self.anims_by_id.insert(anim_id, (Arc::new(anim.clone()), *length));
        }
        let length = *length;
        // (an attack swings as FUN_1006a782 makes it: a body with weapon attachment points (`Attractor03_lefthand`,
        // `HasWeaponAttractors`) the weapon's actions, one without `unarmed-rswing` (0x40a); the animation from the creature's own
        // set, one of the action's at random (FUN_1003cac9 → FUN_10045976), along FUN_10010b3c's fallbacks; once at priority −2 on the
        // action's layer, at its own speed: FUN_1006a782 sets the speed scale only for players, `[0x21c] == 0`)
        let swing = match swing {
            Some((weapon, slot, began, pick, class)) if death.is_none() && self.swings_started.get(&id) != Some(&began) => {
                let data = db.get(Id { kind: mao_formats::item::TEMPLATE, instance: weapon }).ok().flatten();
                let actions = match &data {
                    Some(d) if mesh.attractor_named("Attractor03_lefthand").is_some() => {
                        let a = mao_formats::item::weapon_actions(d, slot, class);
                        if a.is_empty() && class != 0xb { mao_formats::item::attack_actions(d, slot) } else { a }
                    }
                    Some(_) => vec![0x40a],
                    None => vec![],
                };
                let mut found = None;
                if !actions.is_empty() {
                    let mut action = actions[pick as usize % actions.len()];
                    for _ in 0..8 {
                        let matches: Vec<u32> = record.animations.iter().filter(|x| x.0 == action).map(|x| x.1).collect();
                        if !matches.is_empty() {
                            found = Some((action, matches[pick as usize % matches.len()]));
                            break;
                        }
                        action = mao_formats::appearance::action_fallback(action);
                        if action == 0 {
                            break;
                        }
                    }
                }
                let found = found.and_then(|(action, anim)| {
                    let loaded = self.creature_anims.entry(anim).or_insert_with(|| {
                        let a = mao_formats::catanim::parse(&db.get(Id { kind: 0xf6953, instance: anim }).ok()??).ok()?;
                        let length = a.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0)).fold(1.0f32, f32::max);
                        Some((a, length))
                    });
                    let (a, length) = loaded.as_ref()?;
                    let speed = 1.0;
                    if !self.anims_by_id.contains_key(&anim) {
                        self.anims_by_id.insert(anim, (Arc::new(a.clone()), *length));
                    }
                    let claims = mao_formats::appearance::action_name(action).map_or(0, |(_, mask)| (!mask & 3) as u32);
                    Some((anim, *length, claims, speed))
                });
                Some((began, found))
            }
            _ => None,
        };
        // The creature's stack: its movement looping at priority 0 on layer 2, as a player's.
        let (stack, tick) = self.stacks.entry(id).or_insert_with(|| (Default::default(), Instant::now()));
        let dt = tick.elapsed().as_secs_f32().min(0.5);
        *tick = Instant::now();
        let stack_key = (action, if death.is_some() { 9 } else { hold as u8 }, anim_id);
        if self.movement_keys.get(&id) != Some(&stack_key) {
            // (a death is held at its end: direction 3 plays it once and keeps the last frame)
            let handle = stack.set_animation(anim_id, 0, -1, 2, if death.is_some() { 3 } else { 0 }, 1.0, length);
            stack.set_time(handle, 0.0, if hold { 0.0 } else { length });
            self.movement_keys.insert(id, stack_key);
            self.movement_handles.insert(id, handle);
        }
        if let Some(&handle) = self.movement_handles.get(&id) {
            stack.set_speed_scale(handle, rate.max(0.05));
        }
        if let Some((began, found)) = swing {
            if let Some((anim, total, claims, speed)) = found {
                let handle = stack.set_animation(anim, -2, 1, claims, 0, 1.0, total);
                stack.set_time(handle, 0.0, total);
                stack.set_speed_scale(handle, speed);
            }
            self.swings_started.insert(id, began);
        }
        stack.advance(dt);
        let anims = &self.anims_by_id;
        self.notes.extend(fired_notes(stack, anims).into_iter().map(|e| (id, e, Some(template))));
        let bones = stack.pose(mesh, &|a| anims.get(&a).map(|(anim, _)| anim.as_ref()));
        let mut parts = mesh.skinned_model(&bones, 0xf6954).parts;
        if let Some((d, age)) = deform {
            crate::effects::deform_parts(&mut parts, d, age);
        }
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
        self.frames.insert(id, mesh.attractor_frames(&bones));
        Some(parts)
    }

    /// The vehicle body a vehicle program (`nano`) puts a character of this breed and sex on: the NPC template of its `ChangeMesh`
    /// spell whose criteria the body meets (`mao_formats::item::vehicle_template`). The character's own mesh is the breed's
    /// plain body (the thin, fat and robed ones the spells also name are not drawn here).
    fn vehicle(&mut self, nano: u32, breed: u8, sex: u8) -> Option<u32> {
        use mao_formats::resource::Id;
        let mesh = self.find_cached(0xf6952, mao_formats::appearance::player_meshes(breed, sex)?.0)?;
        let db = self.db.clone();
        *self.vehicles.entry((nano, mesh)).or_insert_with(|| {
            let data = db.get(Id { kind: 0xfde85, instance: nano }).ok()??;
            let template = mao_formats::item::vehicle_template(&data, &mao_formats::item::Body { breed: breed as u32, sex: sex as u32, mesh })?;
            db.get(Id { kind: mao_formats::npc::TEMPLATE, instance: template }).ok()?.map(|_| template)
        })
    }

    /// A player on a vehicle body (`template`, from [`Characters::vehicle`]), Y-up local space: the template's CAT mesh (the
    /// rider and board as one body, `Hoverboard_b_male.cir`) posed by the template's animations as a creature's are (its
    /// `Hoverboard_a_male_run_001.ani` for walking and running, `turn_left` / `turn_right` for the sidesteps, `runbackwards`,
    /// `idle_stand`), as the original does once `MonsterData` (359) names the template (FUN_10052686: FUN_100525f3 sets the
    /// template's stats on the character, among them its `Mesh` (12), so the body is the template's; the animations follow
    /// the template's list). The board's body has no head of its own: the rider keeps the character's head (and worn
    /// meshes) on the body's head attractor, and its clothes by material name.
    fn vehicle_parts(&mut self, id: u32, breed: u8, sex: u8, template: u32, moving: Option<mao_formats::appearance::Action>, speed: f32, scale: f32, textures: &[(String, u32)], clothes: &[(String, u32)], gear: &[(u8, u32, u32)], deform: Option<(&mao_formats::gfx::Deform, f32)>) -> Option<Vec<Part>> {
        use mao_formats::resource::Id;
        let db = self.db.clone();
        self.ensure_body(breed, sex);
        self.creatures.entry(template).or_insert_with(|| load_template(&db, template));
        let action = moving.unwrap_or(mao_formats::appearance::Action::Idle);
        let (candidates, mesh_id) = {
            let (record, _, _) = self.creatures.get(&template)?.as_ref()?;
            (record.animation_candidates(action, id), record.mesh.unwrap_or(0))
        };
        // (the first of the action's animations the database really holds)
        let mut chosen = None;
        for (_, anim) in candidates {
            let loaded = self.creature_anims.entry(anim).or_insert_with(|| {
                let a = mao_formats::catanim::parse(&db.get(Id { kind: 0xf6953, instance: anim }).ok()??).ok()?;
                let length = a.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0)).fold(1.0f32, f32::max);
                Some((a, length))
            });
            if let Some((a, length)) = loaded {
                if !self.anims_by_id.contains_key(&anim) {
                    self.anims_by_id.insert(anim, (Arc::new(a.clone()), *length));
                }
                chosen = Some((anim, *length));
                break;
            }
        }
        let (anim, length) = chosen?;
        // (the movement plays at the original's rate for the board's body and animation, FUN_1007009f)
        let rate = match action.reference_speed() {
            Some(reference) if moving.is_some() => movement_rate(crate::effects::calibration(mesh_id, anim), scale, speed, reference),
            _ => 1.0,
        };
        let (stack, tick) = self.stacks.entry(id).or_insert_with(|| (Default::default(), Instant::now()));
        let dt = tick.elapsed().as_secs_f32().min(0.5);
        *tick = Instant::now();
        // (the movement looping at priority 0 on layer 2, as a player's and a creature's)
        let key = (action, 20, anim);
        if self.movement_keys.get(&id) != Some(&key) {
            let handle = stack.set_animation(anim, 0, -1, 2, 0, 1.0, length);
            stack.set_time(handle, 0.0, length);
            self.movement_keys.insert(id, key);
            self.movement_handles.insert(id, handle);
        }
        if let Some(&handle) = self.movement_handles.get(&id) {
            stack.set_speed_scale(handle, rate.max(0.05));
        }
        stack.advance(dt);
        let (_, mesh, _) = self.creatures.get(&template)?.as_ref()?;
        let anims = &self.anims_by_id;
        self.notes.extend(fired_notes(stack, anims).into_iter().map(|e| (id, e, Some(template))));
        let bones = stack.pose(mesh, &|a| anims.get(&a).map(|(anim, _)| anim.as_ref()));
        let mut parts = mesh.skinned_model(&bones, 0xf6954).parts;
        if let Some((d, age)) = deform {
            crate::effects::deform_parts(&mut parts, d, age);
        }
        for part in &mut parts {
            let wanted = clothes.iter().chain(textures.iter()).find(|(n, i)| *i != 0 && n.eq_ignore_ascii_case(&part.material.name));
            if let Some((_, instance)) = wanted {
                part.material.textures = vec![(0xf6954, *instance)];
            }
        }
        let custom_head = gear.iter().any(|g| g.0 == 0 && self.head_meshes.contains(&g.1));
        if let (Some(Some((_, head, _))), Some(attractor), false) = (self.bodies.get(&(breed, sex)), mesh.attractor("head"), custom_head) {
            parts.extend(mesh.attach(&bones, attractor, head));
        }
        parts.extend(wear(&db, &mut self.statics, mesh, &bones, gear));
        let k = (scale / 100.0).max(0.01);
        let mut world = model::IDENTITY;
        (world[0], world[5], world[10]) = (k, k, k);
        for p in &mut parts {
            p.world = world;
        }
        let frames = mesh.attractor_frames(&bones).into_iter().map(|(name, m)| (name, model::multiply(&m, &world))).collect();
        self.frames.insert(id, frames);
        Some(parts)
    }

    /// The breed's and sex's body, its default head and the head's attractor, loaded once.
    fn ensure_body(&mut self, breed: u8, sex: u8) {
        use mao_formats::resource::Id;
        let db = self.db.clone();
        let index = &self.index;
        self.bodies.entry((breed, sex)).or_insert_with(|| {
            let (body_name, head_name) = mao_formats::appearance::player_meshes(breed, sex)?;
            let body = mao_formats::catmesh::parse(&db.get(Id { kind: 0xf6952, instance: index.find(0xf6952, body_name)? }).ok()??).ok()?;
            let head = crate::load_model_attached(&db, Id { kind: 0xf6951, instance: index.find(0xf6951, head_name)? }).ok()?;
            let attractor = body.attractor("head").cloned();
            Some((body, head, attractor))
        });
    }

    /// Loads a player animation by action (cached) and returns its file name.
    /// An instance of the index by type and name, remembered.
    fn find_cached(&mut self, kind: u32, name: &str) -> Option<u32> {
        if let Some(found) = self.ids.get(&(kind, name.to_string())) {
            return *found;
        }
        let found = self.index.find(kind, name);
        self.ids.insert((kind, name.to_string()), found);
        found
    }

    fn ensure_anim(&mut self, breed: u8, sex: u8, action: mao_formats::appearance::Action) -> String {
        use mao_formats::resource::Id;
        let name = mao_formats::appearance::animation_name(breed, sex, action);
        if !self.anims.contains_key(&name) {
            let loaded = (|| {
                let id = self.index.find(0xf6953, &name)?;
                let anim = mao_formats::catanim::parse(&self.db.get(Id { kind: 0xf6953, instance: id }).ok()??).ok()?;
                let length = anim.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0)).fold(1.0f32, f32::max);
                Some((anim, length))
            })();
            self.anims.insert(name.clone(), loaded);
        }
        name
    }

    /// Loads a social action's animation (cached) and returns the key it is stored under, when the files have it.
    fn ensure_social(&mut self, breed: u8, sex: u8, name: &str) -> Option<String> {
        use mao_formats::resource::Id;
        for file in mao_formats::appearance::social_animation_names(breed, sex, name) {
            if !self.anims.contains_key(&file) {
                let loaded = (|| {
                    let id = self.index.find(0xf6953, &file)?;
                    let anim = mao_formats::catanim::parse(&self.db.get(Id { kind: 0xf6953, instance: id }).ok()??).ok()?;
                    let length = anim.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0)).fold(1.0f32, f32::max);
                    Some((anim, length))
                })();
                self.anims.insert(file.clone(), loaded);
            }
            if self.anims.get(&file).is_some_and(|a| a.is_some()) {
                return Some(file);
            }
        }
        None
    }

    /// A loaded animation's instance and length, registered for the stacks.
    fn anim_by_name(&mut self, name: &str) -> Option<(u32, f32)> {
        let id = self.find_cached(0xf6953, name)?;
        if !self.anims_by_id.contains_key(&id) {
            let (anim, length) = self.anims.get(name)?.as_ref()?;
            self.anims_by_id.insert(id, (Arc::new(anim.clone()), *length));
        }
        self.anims_by_id.get(&id).map(|(_, length)| (id, *length))
    }

    /// The seconds since the character's stack was last run on.
    fn stack_tick(&mut self, id: u32) -> f32 {
        let (_, tick) = self.stacks.entry(id).or_insert_with(|| (Default::default(), Instant::now()));
        let dt = tick.elapsed().as_secs_f32().min(0.5);
        *tick = Instant::now();
        dt
    }

    /// The character's stack told what the original's movement and social code tell `VisualCATMesh_t`: the movement
    /// (`SetAnimation` at priority 0 claiming layer 2 — FUN_1006c370's `local_14` — looping, or counted once for a start, a
    /// stop, a jump and a landing, its range from the notes as FUN_10010929's modes set it, its speed scaled every frame as
    /// `SetSpeedScale` is by FUN_1007009f) and a social action (FUN_10010a46: priority 0 claiming nothing, once over its whole
    /// length). `movement` is the animation and its total time, `points` the loop notes and the length, `key` what the
    /// movement is now (a change sets it anew; a looping one already playing is only topped up).
    /// An attack swing (FUN_10010dee: `SetAnimation` at priority −2 claiming the action's layer, once over its whole length) is
    /// set too, sped up by its scale (FUN_1006a782's `SetSpeedScale`). A weapon's draw or holster (`flourish`: FUN_1003cce2's
    /// start, FUN_1003ce27's stop) plays once before the movement, and the idle set with it waits for it (`SetAnimationDelay`,
    /// FUN_1003c6b2, by its length less 300 ms).
    fn drive(&mut self, id: u32, key: (mao_formats::appearance::Action, u8, u32), movement: Option<(u32, f32)>, points: (f32, f32, f32), rate: f32, social: Option<((u32, f32), Instant)>, swing: Option<((u32, f32), u32, f32, Instant)>, flourish: Option<(u32, f32)>) {
        let (stack, _) = self.stacks.entry(id).or_insert_with(|| (Default::default(), Instant::now()));
        let flourish = flourish.map(|(anim, total)| {
            let handle = stack.set_animation(anim, 0, 1, 2, 0, 1.0, total);
            stack.set_time(handle, 0.0, total);
            total
        });
        if let Some((anim, total)) = movement {
            if self.movement_keys.get(&id) != Some(&key) {
                let (start, end, length) = points;
                let length = if length > 0.0 { length } else { total };
                let (loops, range) = match key.1 {
                    0 | 9 => (-1, (0.0, length)),
                    1 => (1, (0.0, start)),
                    2 => (-1, (start, end)),
                    3 => (1, (end, length)),
                    _ => (1, (0.0, length)),
                };
                // (a death, key 9, is held at its end: direction 3)
                let handle = stack.set_animation(anim, 0, loops, 2, if key.1 == 9 { 3 } else { 0 }, 1.0, total);
                stack.set_time(handle, range.0, range.1);
                if let Some(length) = flourish {
                    stack.set_delay(handle, (length - 300.0).max(0.0));
                }
                self.movement_keys.insert(id, key);
                self.movement_handles.insert(id, handle);
            }
            if let Some(&handle) = self.movement_handles.get(&id) {
                stack.set_speed_scale(handle, rate.max(0.05));
            }
        }
        if let Some(((anim, total), began)) = social {
            if self.socials_started.get(&id) != Some(&began) {
                let handle = stack.set_animation(anim, 0, 1, 0, 0, 1.0, total);
                stack.set_time(handle, 0.0, total);
                self.socials_started.insert(id, began);
            }
        }
        if let Some(((anim, total), claims, speed, began)) = swing {
            if self.swings_started.get(&id) != Some(&began) {
                let handle = stack.set_animation(anim, -2, 1, claims, 0, 1.0, total);
                stack.set_time(handle, 0.0, total);
                stack.set_speed_scale(handle, speed);
                self.swings_started.insert(id, began);
            }
        }
    }

    /// Loads an animation file (cached); whether the files have it.
    fn ensure_file(&mut self, file: &str) -> bool {
        use mao_formats::resource::Id;
        if !self.anims.contains_key(file) {
            let loaded = (|| {
                let id = self.index.find(0xf6953, file)?;
                let anim = mao_formats::catanim::parse(&self.db.get(Id { kind: 0xf6953, instance: id }).ok()??).ok()?;
                let length = anim.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0)).fold(1.0f32, f32::max);
                Some((anim, length))
            })();
            self.anims.insert(file.to_string(), loaded);
        }
        self.anims.get(file).is_some_and(|a| a.is_some())
    }

    /// The swing of an attack with the weapon of template `template` in place `slot`, as FUN_1006a782 makes it for a body with
    /// weapon attachment points: one of the weapon's attack actions (`item::attack_actions`; `pick` stands for the original's
    /// `rand()`), its animation by FUN_10010cc2's file names, falling back along FUN_10010b3c's chain (a rifle's shot to the small
    /// arms'); no action at all plays the social 0x3e, `wave` (FUN_10010dee). Its speed: the animation's first note over the
    /// weapon's attack time (stat 294 × 10 ms), held to 1..2. The animation, its length, the layer it claims and the speed. Not
    /// followed: the character's own animation set (its 0xfde97 map, which picks among an action's variants), the height of
    /// the target (FUN_1006a782's `param_3`, which FUN_1003cac9 does not pass on) and the crawling variants.
    /// The animation file of an action (along FUN_10010b3c's fallbacks) and the layer it claims.
    fn ensure_action_file(&mut self, breed: u8, sex: u8, mut action: u32) -> Option<(String, u32)> {
        for _ in 0..8 {
            if let Some((files, claims)) = mao_formats::appearance::action_animation_names(breed, sex, action) {
                if let Some(file) = files.into_iter().find(|f| self.ensure_file(f)) {
                    return Some((file, claims));
                }
            }
            action = mao_formats::appearance::action_fallback(action);
            if action == 0 {
                break;
            }
        }
        None
    }

    /// The animation file of a wielded weapon's action of one class (its first action, along FUN_10010b3c's fallbacks) and the
    /// layer it claims.
    fn ensure_weapon_action(&mut self, breed: u8, sex: u8, template: u32, slot: u32, class: u32) -> Option<(String, u32)> {
        use mao_formats::resource::Id;
        let data = self.db.get(Id { kind: mao_formats::item::TEMPLATE, instance: template }).ok()??;
        let mut action = *mao_formats::item::weapon_actions(&data, slot, class).first()?;
        for _ in 0..8 {
            if let Some((files, claims)) = mao_formats::appearance::action_animation_names(breed, sex, action) {
                if let Some(file) = files.into_iter().find(|f| self.ensure_file(f)) {
                    return Some((file, claims));
                }
            }
            action = mao_formats::appearance::action_fallback(action);
            if action == 0 {
                break;
            }
        }
        None
    }

    /// A special's class (FUN_1003c7a6) swings its own actions, else the plain attack's.
    fn ensure_swing(&mut self, breed: u8, sex: u8, template: u32, slot: u32, pick: u32, class: u32) -> Option<((u32, f32), u32, f32)> {
        use mao_formats::resource::Id;
        let data = self.db.get(Id { kind: mao_formats::item::TEMPLATE, instance: template }).ok()??;
        let mut actions = mao_formats::item::weapon_actions(&data, slot, class);
        if actions.is_empty() && class != 0xb {
            actions = mao_formats::item::attack_actions(&data, slot);
        }
        let attack_ms = mao_formats::item::Item::parse(&data).and_then(|i| i.stat(294)).map_or(0.0, |d| d as f32 * 10.0);
        let (file, claims) = if actions.is_empty() {
            (self.ensure_social(breed, sex, "wave")?, 0)
        } else {
            let mut action = actions[pick as usize % actions.len()];
            let mut found = None;
            for _ in 0..8 {
                if let Some((files, claims)) = mao_formats::appearance::action_animation_names(breed, sex, action) {
                    found = files.into_iter().find(|f| self.ensure_file(f)).map(|f| (f, claims));
                }
                action = mao_formats::appearance::action_fallback(action);
                if found.is_some() || action == 0 {
                    break;
                }
            }
            found?
        };
        let note = self.anims.get(&file)?.as_ref()?.0.notes.first().map_or(0.0, |n| n.time);
        let speed = if attack_ms > 0.0 && note > 0.0 { (note / attack_ms).clamp(1.0, 2.0) } else { 1.0 };
        Some((self.anim_by_name(&file)?, claims, speed))
    }

    /// An animation's `loopstart`, `loopend` and length in ms (the whole length as the loop if it has no notes).
    fn loop_points(&mut self, breed: u8, sex: u8, action: mao_formats::appearance::Action) -> Option<(f32, f32, f32)> {
        let name = self.ensure_anim(breed, sex, action);
        let (anim, length) = self.anims.get(&name)?.as_ref()?;
        let note = |n: &str| anim.notes.iter().find(|x| x.name == n).map(|x| x.time);
        Some(match (note("loopstart"), note("loopend")) {
            (Some(s), Some(e)) if e > s => (s, e.min(*length), *length),
            _ => (0.0, *length, *length),
        })
    }

    /// The animation and time (ms; `None` = the idle loop on the clock) for a character: a jump and its landing, else
    /// the movement in three parts, a start, the cycle and a stop, each played once and the cycle repeated.
    fn motion(&mut self, id: u32, breed: u8, sex: u8, moving: Option<mao_formats::appearance::Action>, jumped: Option<Duration>, rate: f32) -> (mao_formats::appearance::Action, Option<f32>) {
        use mao_formats::appearance::Action::*;
        let now = Instant::now();
        if let Some(since) = jumped {
            let ms = since.as_secs_f32() * 1000.0;
            if ms < AIR_MS + LAND_MS {
                // After the jump the movement carries on as a cycle (or idles).
                self.motions.insert(id, Motion { stage: if moving.is_some() { Stage::Loop } else { Stage::Idle }, action: moving.unwrap_or(Idle), clock: 0.0, tick: now });
                let (jump, land) = match moving {
                    Some(Run) => (JumpForward, LandRun),
                    Some(_) => (JumpForward, LandWalk),
                    None => (JumpStand, LandIdle),
                };
                self.segment = if ms < AIR_MS { 10 } else { 11 };
                if ms < AIR_MS {
                    let length = self.loop_points(breed, sex, jump).map_or(1.0, |p| p.2);
                    return (jump, Some(ms / AIR_MS * length));
                }
                return (land, Some(ms - AIR_MS));
            }
        }
        let st = self.motions.entry(id).or_insert(Motion { stage: Stage::Idle, action: Idle, clock: 0.0, tick: now });
        match (st.stage, moving) {
            (Stage::Idle | Stage::Stop, Some(a)) => *st = Motion { stage: Stage::Start, action: a, clock: 0.0, tick: now },
            (Stage::Start | Stage::Loop, Some(a)) if a != st.action => *st = Motion { stage: Stage::Loop, action: a, clock: 0.0, tick: now },
            (Stage::Start | Stage::Loop, None) => *st = Motion { stage: Stage::Stop, action: st.action, clock: 0.0, tick: now },
            _ => {}
        }
        // (the clock runs at the playback rate: a fast runner's cycle turns faster)
        st.clock += st.tick.elapsed().as_secs_f32() * 1000.0 * rate.max(0.05);
        st.tick = now;
        let Motion { stage, action, clock: elapsed, .. } = *st;
        self.segment = match stage {
            Stage::Idle => 0,
            Stage::Start => 1,
            Stage::Loop => 2,
            Stage::Stop => 3,
        };
        if stage == Stage::Idle {
            return (Idle, None);
        }
        let Some((start, end, length)) = self.loop_points(breed, sex, action) else { return (Idle, None) };
        let cycle = (end - start).max(1.0);
        let Some(st) = self.motions.get_mut(&id) else { return (Idle, None) };
        match stage {
            Stage::Start if elapsed < start => (action, Some(elapsed)),
            Stage::Start | Stage::Loop => {
                let into = if stage == Stage::Start { elapsed - start } else { elapsed };
                if stage == Stage::Start {
                    *st = Motion { stage: Stage::Loop, action, clock: into, tick: now };
                }
                (action, Some(start + into % cycle))
            }
            _ => {
                if end + elapsed >= length {
                    st.stage = Stage::Idle;
                    (Idle, None)
                } else {
                    (action, Some(end + elapsed))
                }
            }
        }
    }

    /// The parts of a player of this breed and sex at `t` ms into the walk,
    /// in Y-up local space (world matrices identity), if its meshes exist.
    /// `vehicle`: the vehicle body (NPC template) a vehicle program puts the character on, drawn instead of its own.
    fn parts(&mut self, id: u32, breed: u8, sex: u8, moving: Option<mao_formats::appearance::Action>, jumped: Option<Duration>, _phase: f32, speed: f32, scale: f32, textures: &[(String, u32)], clothes: &[(String, u32)], gear: &[(u8, u32, u32)], social: Option<(&str, Instant)>, swing: Option<(u32, u32, Instant, u32, u32)>, weapon: Option<(u32, u32)>, fighting: bool, dying: Option<u32>, deform: Option<(&mao_formats::gfx::Deform, f32)>, vehicle: Option<u32>) -> Option<Vec<Part>> {
        if let Some(template) = vehicle {
            if let Some(parts) = self.vehicle_parts(id, breed, sex, template, moving, speed, scale, textures, clothes, gear, deform) {
                return Some(parts);
            }
        }
        // The movement plays at the original's rate for this body, animation and speed.
        let rate = match moving.and_then(|a| a.reference_speed().map(|r| (a, r))) {
            Some((a, reference)) => {
                let body = mao_formats::appearance::player_meshes(breed, sex).and_then(|(body, _)| self.find_cached(0xf6952, body)).unwrap_or(0);
                let name = mao_formats::appearance::animation_name(breed, sex, a);
                let anim = self.find_cached(0xf6953, &name).unwrap_or(0);
                movement_rate(crate::effects::calibration(body, anim), scale, speed, reference)
            }
            None => 1.0,
        };
        let (action, _) = self.motion(id, breed, sex, moving, jumped, rate);
        let custom_head = gear.iter().any(|g| g.0 == 0 && self.head_meshes.contains(&g.1));
        let name = self.ensure_anim(breed, sex, action);
        // (with a weapon in hand, its movement: in a fight the drawn weapon's idle, FUN_1003cce2 looping class 0x10 of the weapon
        // in place 6, else 8, else 0; out of one the idle the wielding set, FUN_1009cda2's class 0x29 (a rifle's `idle-2h`, a
        // bow's, a bazooka's), and a rifle's two-handed walk and run)
        use mao_formats::appearance::Action as A;
        // (dead: the death's animation, by the original's file names along the fallbacks, played once and held)
        let death = dying.and_then(|a| self.ensure_action_file(breed, sex, a));
        let weapon_idle = match (action, weapon) {
            _ if death.is_some() => death.clone(),
            (A::Idle, Some((template, slot))) if fighting => self.ensure_weapon_action(breed, sex, template, slot, 0x10),
            (A::Idle, Some((template, slot))) => self.ensure_weapon_action(breed, sex, template, slot, 0x29),
            (A::Walk | A::Run, Some((template, _))) => {
                let data = self.db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: template }).ok().flatten();
                match data.as_deref().and_then(mao_formats::item::wielded_movement) {
                    Some((walk, run)) => self.ensure_action_file(breed, sex, if action == A::Walk { walk } else { run }),
                    None => None,
                }
            }
            _ => None,
        };
        let movement = match &weapon_idle {
            Some((file, _)) => self.anim_by_name(file),
            None => self.anim_by_name(&name),
        };
        let key = (action, if death.is_some() { 9 } else { self.segment }, movement.map_or(0, |m| m.0));
        let points = match movement.filter(|_| weapon_idle.is_some()) {
            Some((_, length)) => (0.0, length, length),
            None => self.loop_points(breed, sex, action).unwrap_or((0.0, 0.0, 0.0)),
        };
        let social = social.and_then(|(n, began)| {
            let file = self.ensure_social(breed, sex, n)?;
            self.anim_by_name(&file).map(|a| (a, began))
        });
        let swing = swing.and_then(|(template, slot, began, pick, class)| self.ensure_swing(breed, sex, template, slot, pick, class).map(|(anim, claims, speed)| (anim, claims, speed, began)));
        // (drawing the weapon plays its start, class 0x1a; putting it away its stop, 0x1b)
        let before = self.drawn.insert(id, weapon.filter(|_| fighting)).flatten();
        let held = weapon;
        let weapon = weapon.filter(|_| fighting);
        let mut flourish = match (before, weapon) {
            (None, Some((template, slot))) => self.ensure_weapon_action(breed, sex, template, slot, 0x1a),
            (Some((template, slot)), None) => self.ensure_weapon_action(breed, sex, template, slot, 0x1b),
            _ => None,
        };
        // (out of a fight, wielding a weapon plays its class 0x27 before its idle 0x29, or, without that idle, its 0x28 before
        // `idle-stand`; putting one away its 0x28: FUN_1009cda2 through the controller's vtable +0xc. A rifle's `2h-start` and
        // `2h-stop`; most weapons have neither)
        let was_held = self.wielded.insert(id, held).flatten();
        if !fighting && flourish.is_none() && was_held != held {
            flourish = match (was_held, held) {
                (_, Some((template, slot))) => {
                    let data = self.db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: template }).ok().flatten();
                    let has_idle = data.as_deref().is_some_and(|d| !mao_formats::item::weapon_actions(d, slot, 0x29).is_empty());
                    self.ensure_weapon_action(breed, sex, template, slot, if has_idle { 0x27 } else { 0x28 })
                }
                (Some((template, slot)), None) => self.ensure_weapon_action(breed, sex, template, slot, 0x28),
                (None, None) => None,
            };
        }
        let flourish = flourish.and_then(|(file, _)| self.anim_by_name(&file));
        self.drive(id, key, movement, points, rate, social, swing, flourish);
        let dt = self.stack_tick(id);
        let db = self.db.clone();
        self.ensure_body(breed, sex);
        let (body, head, attractor) = self.bodies.get(&(breed, sex))?.as_ref()?;
        let bones = {
            let (stack, _) = self.stacks.get_mut(&id)?;
            stack.advance(dt);
            let anims = &self.anims_by_id;
            self.notes.extend(fired_notes(stack, anims).into_iter().map(|e| (id, e, None)));
            stack.pose(body, &|a| anims.get(&a).map(|(anim, _)| anim.as_ref()))
        };
        let mut model = body.skinned_model(&bones, 0xf6954);
        // (a deformer hit ripples the body's vertices, as its callback on the body's CATRender does)
        if let Some((d, age)) = deform {
            crate::effects::deform_parts(&mut model.parts, d, age);
        }
        // The character's own textures (its clothes) replace the mesh's by material name.
        for part in &mut model.parts {
            // Worn clothes win over the base textures.
            let wanted = clothes.iter().chain(textures.iter()).find(|(n, i)| *i != 0 && n.eq_ignore_ascii_case(&part.material.name));
            if let Some((_, instance)) = wanted {
                part.material.textures = vec![(0xf6954, *instance)];
            }
        }
        if let (Some(a), false) = (attractor, custom_head) {
            model.parts.extend(body.attach(&bones, a, head));
        }
        model.parts.extend(wear(&db, &mut self.statics, body, &bones, gear));
        let mut parts = model.parts;
        // The body's scale is the Scale stat (360) over 100 (`n3VisualDynel_t::SetBodyScale`, from FUN_10058800 and its kin).
        let k = (scale / 100.0).max(0.01);
        let mut world = model::IDENTITY;
        (world[0], world[5], world[10]) = (k, k, k);
        for p in &mut parts {
            p.world = world;
        }
        let frames = body.attractor_frames(&bones).into_iter().map(|(name, m)| (name, model::multiply(&m, &world))).collect();
        self.frames.insert(id, frames);
        Some(parts)
    }
}

/// A character of the launcher's list as the world draws it (its breed's and sex's body and head, standing), Z-up.
pub(crate) fn character_model(db: &Arc<ResourceDatabase>, breed: i32, sex: i32, look: Option<&crate::look::Look>) -> Option<Model> {
    // (the Atrox has one body, whatever the list says)
    let (breed, sex) = (breed as u8, if breed == 4 { 1 } else { sex as u8 });
    let mut chars = Characters::new(db.clone()).ok()?;
    let (textures, clothes, gear) = look.map_or((&[][..], &[][..], &[][..]), |l| (l.textures.as_slice(), l.clothes.as_slice(), l.gear.as_slice()));
    let parts = chars.parts(0, breed, sex, None, None, 0.0, 0.0, 100.0, textures, clothes, gear, None, None, None, false, None, None, None)?;
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    Some(Model { instances: (0..parts.len()).map(|i| (i, swap)).collect(), parts, water: None, lod_groups: Vec::new() })
}

/// `creature <game> <npc template> <out.png>`: a creature as the world draws it (what a polymorph turns a character into).
pub(crate) fn creature_preview(db: Arc<ResourceDatabase>, template: u32, out: &str) -> Result<(), String> {
    let mut chars = Characters::new(db.clone())?;
    // (`MAO_ACTION=left|right|walk|run` shows another animation)
    let action = match std::env::var("MAO_ACTION").unwrap_or_default().as_str() {
        "left" => mao_formats::appearance::Action::StrafeLeft,
        "right" => mao_formats::appearance::Action::StrafeRight,
        "walk" => mao_formats::appearance::Action::Walk,
        "run" => mao_formats::appearance::Action::Run,
        _ => mao_formats::appearance::Action::Idle,
    };
    let parts = chars.creature_parts(0, template, action, 0.0, 1.0, &[], &[], None, None, None).ok_or("that template has no mesh")?;
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let model = Model { instances: (0..parts.len()).map(|i| (i, swap)).collect(), parts, water: None, lod_groups: Vec::new() };
    crate::viewer::headless(&model, &crate::Textures::new(db), out)
}

/// `ride <game> <breed> <sex> <nano> <out.png>`: a character on the vehicle body its vehicle program gives it, as the world draws
/// it, with the effect meshes of `MAO_AURA` at the body's attractors.
pub(crate) fn ride_preview(db: Arc<ResourceDatabase>, breed: u8, sex: u8, nano: u32, out: &str) -> Result<(), String> {
    let mut chars = Characters::new(db.clone())?;
    let template = chars.vehicle(nano, breed, sex).ok_or("that program puts this body on no vehicle")?;
    eprintln!("vehicle body: NPC template {template}");
    use mao_formats::appearance::Action;
    let action = match std::env::var("MAO_ACTION").unwrap_or_default().as_str() {
        "left" => Some(Action::StrafeLeft),
        "right" => Some(Action::StrafeRight),
        "walk" => Some(Action::Walk),
        "run" => Some(Action::Run),
        _ => None,
    };
    let parts = chars.parts(0, breed, sex, action, None, 0.0, 5.0, 100.0, &[], &[], &[], None, None, None, false, None, None, Some(template)).ok_or("the vehicle body did not load")?;
    // (at the origin facing the server's +z: the local frame is the server's)
    let attractors = chars.frames.get(&0).cloned().unwrap_or_default();
    for (name, m) in attractors.iter().filter(|(n, _)| n.contains("special") || n.contains("head")) {
        eprintln!("  {name} at {:.2} {:.2} {:.2}, axes {:.2?} {:.2?} {:.2?}", m[12], m[13], m[14], &m[0..3], &m[4..7], &m[8..11]);
    }
    // (`MAO_PROBE=x,y,z;...`, Z up: the nearest vertex of the posed body to each point, and the body's extent there)
    if let Ok(probe) = std::env::var("MAO_PROBE") {
        let points: Vec<[f32; 3]> = parts.iter().flat_map(|p| p.vertices.iter().map(|v| [v.position[0], v.position[2], v.position[1]])).collect();
        for q in probe.split(';') {
            let c: Vec<f32> = q.split(',').filter_map(|n| n.trim().parse().ok()).collect();
            let [x, y, z] = c[..] else { continue };
            let near = points.iter().map(|p| ((p[0] - x).powi(2) + (p[1] - y).powi(2) + (p[2] - z).powi(2)).sqrt()).fold(f32::MAX, f32::min);
            let column: Vec<f32> = points.iter().filter(|p| (p[0] - x).abs() < 0.15 && (p[1] - y).abs() < 0.15).map(|p| p[2]).collect();
            let (lo, hi) = column.iter().fold((f32::MAX, f32::MIN), |(a, b), &h| (a.min(h), b.max(h)));
            eprintln!("  probe {x:.2} {y:.2} {z:.2}: nearest vertex {near:.2}, body from {lo:.2} to {hi:.2} above it");
        }
        let (ymin, ymax) = points.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[1]), b.max(p[1])));
        eprintln!("  body from {ymin:.2} to {ymax:.2} along its length");
    }
    // (each special attractor's own placement and the bone it hangs on)
    if let Some(Some((_, mesh, _))) = chars.creatures.get(&template) {
        for a in mesh.groups.iter().flat_map(|g| &g.attractors).filter(|a| a.name.contains("special")) {
            let bone = usize::try_from(a.extra).ok().and_then(|b| mesh.bones.get(b)).map_or("-", |b| b.name.as_str());
            eprintln!("  {} placed {:.3?} on bone {} ({bone})", a.name, a.values, a.extra);
        }
    }
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let model = Model { instances: (0..parts.len()).map(|i| (i, swap)).collect(), parts, water: None, lod_groups: Vec::new() };
    crate::viewer::headless_on(&model, &crate::Textures::new(db), out, None, &attractors, breed == mao_formats::appearance::ATROX)
}

/// The entities' markers alone (own character red, the others blue): two
/// parts and an instance per entity, placed Z-up.
fn overlay_model(world: &World, props: &BTreeMap<u32, (u32, [f32; 3], [f32; 4])>, characters: &mut Option<Characters>, include: &dyn Fn(u32) -> bool) -> (Model, Vec<crate::blobshadow::Caster>) {
    let mut model = Model { parts: vec![marker([0.9, 0.1, 0.1]), marker([0.2, 0.4, 0.95]), nose(), marker([0.2, 0.8, 0.3])], instances: Vec::new(), water: None, lod_groups: Vec::new() };
    // (the characters' meshes throw the original's blob shadows: each is noted as a caster)
    let mut casters = Vec::new();
    // (`frames`: the body's attractors in its local space, carried by the caster in the server's frame for the effects on them)
    fn push_character(model: &mut Model, casters: &mut Vec<crate::blobshadow::Caster>, parts: Vec<mao_formats::model::Part>, place: mao_formats::model::Matrix, own: bool, frames: Option<&Vec<(String, mao_formats::model::Matrix)>>) {
        let mut caster = crate::blobshadow::Caster::new(&parts, &place, own);
        let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        caster.attractors = frames.map_or(Vec::new(), |f| f.iter().map(|(name, m)| (name.clone(), model::multiply(&model::multiply(m, &place), &swap))).collect());
        casters.push(caster);
        let base = model.parts.len();
        for (k, p) in parts.into_iter().enumerate() {
            model.parts.push(p);
            model.instances.push((base + k, place));
        }
    }
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    for (&id, &(position, _)) in &world.entities {
        if !include(id) {
            continue;
        }
        let q = world.rotations.get(&id).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let place = model::multiply(&model::trs_matrix(position, q, [1.0; 3]), &swap);
        let deform = world.deforms.get(&id).map(|(d, at)| (d, at.elapsed().as_secs_f32()));
        if let (Some(info), Some(chars)) = (world.infos.get(&id).filter(|i| i.template.is_some()), characters.as_mut()) {
            let moving = world.last_move.get(&id).is_some_and(|t| t.elapsed() < Duration::from_millis(900));
            let action = if moving { mao_formats::appearance::Action::Walk } else { mao_formats::appearance::Action::Idle };
            // (an attack swings the weapon the server said the creature holds in the place it names)
            let swing = world.swings.get(&id).and_then(|&(slot, began, pick, class)| Some((*world.weapons.get(&(id, slot as u8))?, slot, began, pick, class)));
            if let Some(parts) = chars.creature_parts(id, info.template.unwrap(), action, (id % 997) as f32 * 3.0, 1.0, &info.textures, &info.clothes, deform, world.dead.get(&id).map(|d| d.0), swing) {
                push_character(&mut model, &mut casters, parts, place, id == world.own, chars.frames.get(&id));
                continue;
            }
        }
        // (a polymorphed character is drawn as the creature it became)
        if let (Some(&creature), Some(chars)) = (world.morphs.get(&id), characters.as_mut()) {
            let moving = world.last_move.get(&id).is_some_and(|t| t.elapsed() < Duration::from_millis(if id == world.own { 90 } else { 700 }));
            let speed = world.speeds.get(&id).copied().unwrap_or(0.0);
            use mao_formats::appearance::Action;
            let action = match world.strafing.get(&id).copied().unwrap_or(0) {
                _ if !moving => Action::Idle,
                -1 => Action::StrafeLeft,
                1 => Action::StrafeRight,
                _ if world.backing.get(&id).copied().unwrap_or(false) => if speed > 2.2 { Action::RunBack } else { Action::WalkBack },
                _ if speed > 2.2 => Action::Run,
                _ => Action::Walk,
            };
            if let Some(parts) = chars.creature_parts(id, creature, action, (id % 997) as f32 * 3.0, MORPH_ANIM_RATE, &[], &[], deform, world.dead.get(&id).map(|d| d.0), None) {
                push_character(&mut model, &mut casters, parts, place, id == world.own, chars.frames.get(&id));
                continue;
            }
        }
        let info = world.infos.get(&id).filter(|i| i.is_humanoid());
        if let (Some(info), Some(chars)) = (info, characters.as_mut()) {
            // (a social action in progress plays over the movement on the character's animation stack)
            let social = world.socials.get(&id).map(|&(social, began)| (social, began));
            // (an attack swings the weapon held in the place it names: the server's WeaponItemFullUpdate, or our own inventory)
            let swing = world.swings.get(&id).and_then(|&(slot, began, pick, class)| {
                let template = world.weapons.get(&(id, slot as u8)).copied().or_else(|| (id == world.own).then(|| world.inventory.as_ref()?.iter().find(|e| e.slot == slot).map(|e| e.low_id)).flatten())?;
                Some((template, slot, began, pick, class))
            });
            // (the weapon in hand: place 6, else 8, else 0, as FUN_1003cce2 looks; drawn while fighting)
            let fighting = world.fighting.contains_key(&id);
            let weapon = [6u32, 8, 0].into_iter().find_map(|slot| {
                let template = world.weapons.get(&(id, slot as u8)).copied().or_else(|| (id == world.own).then(|| world.inventory.as_ref()?.iter().find(|e| e.slot == slot).map(|e| e.low_id)).flatten())?;
                Some((template, slot))
            });
            // Movement animates while the position changes (our own is sampled every frame, the others' every few hundred ms).
            let moving = world.last_move.get(&id).is_some_and(|t| t.elapsed() < Duration::from_millis(if id == world.own { 90 } else { 700 }));
            let speed = world.speeds.get(&id).copied().unwrap_or(0.0);
            // The original's choice (FUN_1006c370): a sidestep plays walk-left/right, moving against the facing the back
            // animations; walking is 1.5 units a second and running from 3 (backwards) or 5, so 2.2 tells them apart.
            use mao_formats::appearance::Action;
            let running = speed > 2.2;
            let action = match world.strafing.get(&id).copied().unwrap_or(0) {
                _ if !moving => None,
                -1 => Some(Action::StrafeLeft),
                1 => Some(Action::StrafeRight),
                _ if world.backing.get(&id).copied().unwrap_or(false) => Some(if running { Action::RunBack } else { Action::WalkBack }),
                _ => Some(if running { Action::Run } else { Action::Walk }),
            };
            let scale = if id == world.own { world.stats.get(&360).copied().filter(|&s| s > 0).unwrap_or(100) as f32 } else { 100.0 };
            let jumped = world.jumped.get(&id).map(|t| t.elapsed());
            // (A phase per entity keeps a crowd from moving in step.)
            // (a character running a vehicle program rides its board or bike)
            let vehicle = world.riding.get(&id).and_then(|&nano| chars.vehicle(nano, info.breed, info.sex));
            if let Some(parts) = chars.parts(id, info.breed, info.sex, action, jumped, (id % 997) as f32 * 3.0, speed, scale, &info.textures, &info.clothes, world.equipment.get(&id).map_or(&[][..], |v| v.as_slice()), social, swing, weapon, fighting, world.dead.get(&id).map(|d| d.0), deform, vehicle) {
                push_character(&mut model, &mut casters, parts, place, id == world.own, chars.frames.get(&id));
                continue;
            }
        }
        let player = world.names.get(&id).is_some_and(|n| n.1);
        model.instances.push((if id == world.own { 0 } else if player { 3 } else { 1 }, place));
        model.instances.push((2, place));
    }
    // The machines (shop booths): one copy of each model's parts, an instance of each per machine.
    if let Some(chars) = characters.as_mut() {
        let mut copies: std::collections::HashMap<u32, Option<std::ops::Range<usize>>> = std::collections::HashMap::new();
        // (the doors that were used and have not closed again are not drawn)
        let open: std::collections::HashSet<u32> = world.doors_open.iter().filter(|(_, until)| **until > Instant::now()).filter_map(|(n, _)| world.door_slots.get(n).map(|s| s.0)).collect();
        for (&id, &(mesh, position, q)) in props {
            if !include(id) || open.contains(&id) {
                continue;
            }
            let range = copies
                .entry(mesh)
                .or_insert_with(|| {
                    let parts = chars.prop_parts(mesh)?;
                    let start = model.parts.len();
                    model.parts.extend(parts.iter().cloned());
                    Some(start..model.parts.len())
                })
                .clone();
            let Some(range) = range else { continue };
            let place = model::multiply(&model::trs_matrix(position, q, [1.0; 3]), &swap);
            model.instances.extend(range.map(|i| (i, place)));
        }
    }
    (model, casters)
}

/// The playfield with a marker at each entity (the character in red, the
/// others in blue).
fn show_world(game: &str, playfield: u32, world: &World, image: Option<&str>) -> Result<(), String> {
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    let mut scene: Model = crate::load_terrain(&db, playfield).unwrap_or_default();
    let part_mesh = crate::add_props(game, &db, playfield, &mut scene);
    if scene.instances.is_empty() {
        let ground = scene.parts.len();
        scene.instances.extend((0..ground).map(|i| (i, model::IDENTITY)));
    }
    if let Some(shell) = crate::load_room_shell(&db, playfield) {
        let base = scene.parts.len();
        scene.instances.extend((0..shell.parts.len()).map(|i| (base + i, model::IDENTITY)));
        scene.parts.extend(shell.parts);
    }
    if let Ok(t) = crate::load_terrain_data(&db, playfield) {
        let started = Instant::now();
        let ground_parts = crate::load_terrain(&db, playfield).map(|m| m.parts.len()).unwrap_or(0);
        let collision = build_collision(&scene, ground_parts, &crate::Textures::new(db.clone()), &passable_parts(&db, &part_mesh));
        eprintln!("collision: {} triangles in {:?}", collision.len(), started.elapsed());
        for (id, (p, _)) in world.entities.iter().take(6) {
            eprintln!("entity {id}: server y {:.2}, ground model {:.2}", p[1], t.height_at(p[0], p[2], t.height_scale));
        }
    }
    let started_characters = Instant::now();
    let mut characters = Characters::new(db.clone()).ok();
    profile!("[profile] Characters::new {:?}", started_characters.elapsed());
    let mut world_props = world.props.clone();
    world_props.extend(crate::static_props(&db, playfield).into_iter().map(|(k, p, _)| (k, p)));
    eprintln!("{} machines with a model", world_props.len());
    let (overlay, _) = overlay_model(world, &world_props, &mut characters, &|_| true);
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
                    *texture_opaque.entry((kind, instance)).or_insert_with(|| textures.is_opaque(kind, instance))
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
pub(crate) struct Area {
    pub(crate) scene: Model,
    pub(crate) terrain: Option<Arc<mao_formats::terrain::Terrain>>,
    pub(crate) collision: mao_formats::collision::Collision,
    /// The whompah booths (x, height, z).
    pub(crate) booths: Vec<[f32; 3]>,
    /// The playfield's areas and their environments (light and fog by where the character stands), and the playfield's
    /// own environment bytes (the base the areas blend into).
    pub(crate) areas: Vec<mao_formats::areas::Area>,
    pub(crate) base_environment: Option<[u8; 26]>,
}

pub(crate) fn load_area(game: &str, db: &Arc<ResourceDatabase>, playfield: u32) -> Result<Area, String> {
    load_area_with(game, db, playfield, None)
}

/// [`load_area`] that also decodes the textures the scene uses into `prefetch` while the collision is built (the two are independent,
/// so the playfield is ready after the longer of them, not both).
pub(crate) fn load_area_with(game: &str, db: &Arc<ResourceDatabase>, playfield: u32, prefetch: Option<&crate::Textures>) -> Result<Area, String> {
    let started = Instant::now();
    // An interior has no ground at all (only props).
    let mut scene = crate::load_terrain(db, playfield).unwrap_or_default();
    let terrain = crate::load_terrain_data(db, playfield).ok().map(Arc::new);
    let ground_parts = scene.parts.len();
    profile!("[profile] area {playfield}: terrain {:?}", started.elapsed());
    let part_mesh = crate::add_props(game, db, playfield, &mut scene);
    profile!("[profile] area {playfield}: props {:?} ({} parts)", started.elapsed(), scene.parts.len());
    if scene.instances.is_empty() {
        scene.instances.extend((0..ground_parts).map(|i| (i, model::IDENTITY)));
    }
    // An interior made of rooms (a shop instance): their floors and walls.
    let ground_parts = if terrain.is_none() {
        let before = scene.parts.len();
        if let Some(shell) = crate::load_room_shell(db, playfield) {
            let base = scene.parts.len();
            scene.instances.extend((0..shell.parts.len()).map(|i| (base + i, model::IDENTITY)));
            scene.parts.extend(shell.parts);
        }
        // (the shell is solid ground, not a prop: collision counts it with the ground)
        let _ = before;
        ground_parts
    } else {
        ground_parts
    };
    // Trigger areas are walked into, not blocked by: whompah entrances, doors and gates.
    let passable = passable_parts(db, &part_mesh);
    let before_collision = started.elapsed();
    // (the textures are decoded once, on every core, for the upload and for the collision's opacity check)
    let own_textures;
    let textures = match prefetch {
        Some(textures) => {
            textures.prefetch(&scene);
            profile!("[profile] area {playfield}: textures decoded {:?}", started.elapsed());
            textures
        }
        None => {
            own_textures = crate::Textures::new(db.clone());
            &own_textures
        }
    };
    let collision = build_collision(&scene, ground_parts, textures, &passable);
    profile!("[profile] area {playfield}: collision {:?} (after {before_collision:?})", started.elapsed());
    eprintln!("playfield {playfield}: collision {} triangles", collision.len());
    let mut booths = crate::whompah_booths(game, db, playfield);
    if terrain.is_none() {
        // An interior's way out is marked by its door blocker: offer it like a booth (labelled, and the Go button walks there).
        booths.extend(crate::door_markers(game, db, playfield));
    }
    let areas = mao_formats::areas::load(db, playfield);
    if !areas.is_empty() {
        eprintln!("playfield {playfield}: {} areas ({} with an environment)", areas.len(), areas.iter().filter(|a| a.takes_part()).count());
    }
    let anarchy = mao_formats::playfield::load_anarchy(db, playfield);
    // (the playfield's liquid, for the effects that ask for it: `liquid.rs`)
    crate::liquid::set_current(anarchy.as_ref().map(|a| std::sync::Arc::new(crate::liquid::Liquid::new(&a.water))));
    let base_environment = anarchy.and_then(|a| a.environment);
    if let Some(e) = &base_environment {
        eprintln!("playfield {playfield}: environment weather {:?} ambient {:?} fog {:?} density {}", &e[..8], &e[19..22], &e[22..25], e[25]);
    }
    Ok(Area { scene, terrain, collision, booths, areas, base_environment })
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
fn live(game: &str, playfield: u32, mut connection: Connection, mut state: World, session_key: [u8; 8]) -> Result<(), String> {
    crate::theme::set_game_dir(game);
    // The sound module (made and started once) and the game's sources, as `PlayfieldInit` (0x10016e94) asks with
    // `ActivateGameZone`.
    crate::audio::start(Path::new(game));
    crate::audio::with(|m| m.activate_game_zone());
    // (GUI.dll's `CharacterLoggedInMessage` (0x10016f8f) starts the music: `PlayStartupMusic`)
    if !MUSIC_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        crate::audio::with(|m| m.play_startup_music());
    }
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    crate::effects::load_calibration(game);
    let textures = crate::Textures::new(db.clone());
    let Area { scene, terrain, collision, booths, areas, base_environment } = load_area_with(game, &db, playfield, Some(&textures))?;
    let mut areas = areas;
    let mut base_environment = base_environment;
    let thread_textures = textures.clone_handle();
    let started_props = Instant::now();
    state.set_static_props(crate::static_props(&db, playfield));
    profile!("[profile] static_props {:?}", started_props.elapsed());
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
    // A teleport try under way: when it began (the original's `TeleportTrier_t`, 30 s before it gives up).
    let mut teleport_try: Option<Instant> = None;
    let game_dir = game.to_string();
    let thread_db = db.clone();
    let mut current = playfield;
    connection.set_timeout(Duration::from_millis(12)).map_err(|e| e.to_string())?;
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
    let (mail_tx, mail_rx) = std::sync::mpsc::channel::<crate::mailbox::MailCommand>();
    let (shop_tx, shop_rx) = std::sync::mpsc::channel::<crate::shop::ShopCommand>();
    let own = state.own;
    let start = state.entities.get(&own).map(|e| e.0).unwrap_or([0.0; 3]);
    let heading = state.rotations.get(&own).map(|q| 2.0 * q[1].atan2(q[3])).unwrap_or(0.0);
    let (gear_db, gear_textures) = (db.clone(), crate::Textures::new(db.clone()));
    let _ = ITEM_DB.set(db.clone());
    // The chat server (once per run, kept across zonings): its lines go to the chat window (and the on-screen log), and what the
    // window sends goes out through `send_chat`.
    if std::env::var_os("MAO_NO_CHAT").is_none() && !CHAT_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        if let Ok((user, password)) = credentials() {
            let (out_tx, out_rx) = std::sync::mpsc::channel::<crate::chatwindow::Out>();
            let _ = CHAT_OUT.set(std::sync::Mutex::new(out_tx));
            std::thread::spawn(move || {
                let server = std::env::var("MAO_CHAT_SERVER").unwrap_or_else(|_| mao_net::chat::SERVER.to_string());
                let result = (|| -> Result<(), mao_net::chat::Error> {
                    use crate::chatwindow::{Kind, Line, Out};
                    use mao_net::chat::Packet;
                    let mut chat = mao_net::chat::Chat::connect(server.as_str())?;
                    chat.login(own, &user, &password)?;
                    // The read waits a moment, so what the window sends goes out promptly.
                    chat.set_timeout(Duration::from_millis(200))?;
                    let mut names: BTreeMap<u32, String> = BTreeMap::new();
                    let mut ids: BTreeMap<String, u32> = BTreeMap::new();
                    let mut channels: BTreeMap<u32, (u8, String)> = BTreeMap::new();
                    // each channel's flags (0x40: no voice lines)
                    let mut channel_flags: BTreeMap<u32, u32> = BTreeMap::new();
                    // tells waiting for the id of the character they go to: (lower-case name, text)
                    let mut waiting: Vec<(String, String)> = Vec::new();
                    // names of characters to put on the friends list once their id is known
                    let mut befriending: Vec<String> = Vec::new();
                    // the senders whose voice lines were heard lately
                    let mut voices = crate::voices::Limiter::default();
                    loop {
                        let packet = match chat.receive() {
                            Ok(p) => Some(p),
                            Err(mao_net::chat::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => None,
                            Err(e) => return Err(e),
                        };
                        let line = match packet {
                            Some(Packet::Name { id, name }) => {
                                ids.insert(name.to_lowercase(), id);
                                names.insert(id, name.clone());
                                // the tells that were waiting for this name
                                let mut sent = Vec::new();
                                waiting.retain(|(who, text)| {
                                    if *who == name.to_lowercase() {
                                        sent.push(text.clone());
                                        false
                                    } else {
                                        true
                                    }
                                });
                                for text in sent {
                                    chat.send(&mao_net::chat::tell_packet(id, &text))?;
                                }
                                if befriending.contains(&name.to_lowercase()) {
                                    befriending.retain(|w| *w != name.to_lowercase());
                                    chat.send(&mao_net::chat::buddy_add_packet(id))?;
                                    let mut buddies = BUDDIES.lock().unwrap();
                                    buddies.1.entry(id).or_insert((name.clone(), false));
                                    buddies.0 += 1;
                                }
                                // a friend whose name came after its status
                                let mut buddies = BUDDIES.lock().unwrap();
                                if let Some(entry) = buddies.1.get_mut(&id) {
                                    if entry.0 != name {
                                        entry.0 = name.clone();
                                        buddies.0 += 1;
                                    }
                                }
                                None
                            }
                            Some(Packet::Channel { kind, id, name, flags }) => {
                                channel_flags.insert(id, flags);
                                channels.insert(id, (kind, name));
                                None
                            }
                            Some(Packet::Notice { name, text, blob }) => {
                                // (`HandleVicinityMessage` → FUN_1008cdb3: its voice line, if any)
                                crate::voices::vicinity_message(&mut voices, &name, &blob);
                                Some(Line::new(Kind::System, mao_net::chat::plain(&text)))
                            }
                            Some(Packet::Vicinity { sender, text, blob }) => {
                                let who = names.get(&sender).cloned().unwrap_or_else(|| format!("{sender:x}"));
                                crate::voices::vicinity_message(&mut voices, &who, &blob);
                                Some(Line { kind: Kind::Channel, text: format!("{who}: {}", mao_net::chat::plain(&text)), from: Some(who) })
                            }
                            Some(Packet::Tell { sender, text }) => {
                                let text = mao_net::chat::plain(&text);
                                match names.get(&sender) {
                                    Some(who) => Some(Line { kind: Kind::Tell, text: format!("{who} tells you: {text}"), from: Some(who.clone()) }),
                                    // the server's own notices (the players online) come the same way
                                    None => Some(Line::new(Kind::System, text)),
                                }
                            }
                            // (a voice line in a channel that allows none is dropped, line and all: `HandleGroupMessage`)
                            Some(Packet::Message { channel, blob, .. }) if channel_flags.get(&channel.1).is_some_and(|f| f & 0x40 != 0) && crate::voices::parse(&blob).0.is_some_and(|v| !v.line.is_empty()) => None,
                            Some(Packet::Message { channel, sender, text, blob }) => {
                                let who = names.get(&sender).cloned().unwrap_or_else(|| format!("{sender:x}"));
                                let place = channels.get(&channel.1).map(|c| c.1.clone()).unwrap_or_default();
                                // (its voice line, if it carries one: `HandleGroupMessage` → FUN_1008cdb3, by the group's kind)
                                let kind = channels.get(&channel.1).map_or(channel.0, |c| c.0);
                                crate::voices::group_message(&mut voices, &who, kind, &blob);
                                Some(Line { kind: Kind::Channel, text: format!("[{place}] {who}: {}", mao_net::chat::plain(&text)), from: Some(who) })
                            }
                            Some(Packet::Buddy { id, online, .. }) => {
                                let mut buddies = BUDDIES.lock().unwrap();
                                let name = names.get(&id).cloned().unwrap_or_else(|| format!("#{id}"));
                                // (a friend already on the list who comes or goes is told; the first listing is not)
                                let changed = buddies.1.get(&id).is_some_and(|(_, was)| *was != online);
                                let shown = buddies.1.get(&id).map_or(name.clone(), |(n, _)| n.clone());
                                buddies.1.insert(id, (name, online));
                                buddies.0 += 1;
                                changed.then(|| Line::new(Kind::System, format!("{shown} {}.", if online { "is now online" } else { "has gone offline" })))
                            }
                            Some(Packet::Other { kind, data }) => {
                                eprintln!("chat packet {kind}: {}", data.iter().map(|b| format!("{b:02x}")).collect::<String>());
                                None
                            }
                            _ => None,
                        };
                        // A line with control characters is a packet of another layout read wrongly (seen once live): skip it.
                        if let Some(line) = line.filter(|l| !l.text.chars().any(char::is_control)) {
                            CHAT_LINES.lock().unwrap().push(line);
                        }
                        while let Ok(out) = out_rx.try_recv() {
                            match out {
                                Out::Tell { name, text } => match ids.get(&name.to_lowercase()) {
                                    Some(&id) => chat.send(&mao_net::chat::tell_packet(id, &text))?,
                                    None => {
                                        waiting.push((name.to_lowercase(), text));
                                        chat.send(&mao_net::chat::lookup_packet(&name))?;
                                    }
                                },
                                Out::Camp | Out::Quit | Out::Follow | Out::Inspect | Out::Window { .. } | Out::Waypoint { .. } | Out::SelectSelf | Out::SetOption { .. } | Out::ViewDist(_) | Out::CharDist(_) => {}
                                Out::Befriend { name } => match ids.get(&name.to_lowercase()) {
                                    Some(&id) => {
                                        chat.send(&mao_net::chat::buddy_add_packet(id))?;
                                        let mut buddies = BUDDIES.lock().unwrap();
                                        buddies.1.entry(id).or_insert((name, false));
                                        buddies.0 += 1;
                                    }
                                    None => {
                                        befriending.push(name.to_lowercase());
                                        chat.send(&mao_net::chat::lookup_packet(&name))?;
                                    }
                                },
                                Out::Unfriend { id } => {
                                    chat.send(&mao_net::chat::buddy_remove_packet(id))?;
                                    let mut buddies = BUDDIES.lock().unwrap();
                                    buddies.1.remove(&id);
                                    buddies.0 += 1;
                                }
                                Out::Org { text } => match channels.iter().find(|(_, (kind, _))| *kind == ORG_CHANNEL_KIND) {
                                    Some((&id, _)) => chat.send(&mao_net::chat::channel_packet(ORG_CHANNEL_KIND, id, &text))?,
                                    None => CHAT_LINES.lock().unwrap().push(Line::new(Kind::System, "You are not in an organization.")),
                                },
                                // `/voice` (FUN_100b88b1): its line and record, said in the vicinity (the chat window's selected group in the
                                // original, Vicinity unless another is picked: this client has no other to pick)
                                Out::Voice { line } => match crate::voices::command(&line) {
                                    Err(e) => CHAT_LINES.lock().unwrap().push(Line::new(Kind::System, e)),
                                    Ok((text, blob)) => speak(0, text, blob),
                                },
                                Out::Vicinity { kind, text } => speak(kind, text, Vec::new()),
                                Out::Social { .. } => {}
                            }
                        }
                    }
                })();
                if let Err(e) = result {
                    eprintln!("chat: {e}");
                    CHAT_LINES.lock().unwrap().push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, format!("The chat connection ended: {e}")));
                }
            });
        }
    }
    std::thread::spawn(move || {
        use mao_net::n3::movement;
        const TURN: f32 = 2.0; // radians per second
        // The server's heights sit a little off the ground model's; walk at
        // the ground's height plus the offset seen at the start.
        let ground = |terrain: &Option<Arc<mao_formats::terrain::Terrain>>, p: [f32; 3]| terrain.as_ref().map_or(0.0, |t| t.height_at(p[0], p[2], t.height_scale));
        let mut offset = start[1] - ground(&terrain, start);
        let mut base = start[1];
        let clock = Instant::now();
        let tracing = std::env::var("MAO_TRACE").is_ok_and(|v| v == "1");
        let mut last_report = Instant::now();
        let mut fresh = false;
        state.arrival = None;
        let mut pending_scene: Option<Model> = None;
        let first_inventory = state.inventory.take();
        if let Some(entries) = &first_inventory {
            state.worn = entries.clone();
            state.mods_dirty = true;
        }
        state.refresh_mods(&gear_db);
        let mut pending_gear = first_inventory.map(|e| crate::gear::from_entries(&gear_db, &gear_textures, &e, state.stats.get(&0x36).copied().unwrap_or(1)));
        let mut keys = crate::viewer::Input::default();
        let mut raw = crate::viewer::Input::default();
        // The notes' sounds: the controller's own (FUN_1004496c), the tiles' materials (N3's `n3TilemapController_t`, from
        // 0xf4258:1) and the CRT `rand()` FUN_10045976 picks an event's variant with.
        let generic = crate::sounds::Generic::default();
        let tile_materials = gear_db.get(mao_formats::resource::Id { kind: mao_formats::tilemap::TILE_MATERIALS, instance: 1 }).ok().flatten().and_then(|o| mao_formats::tilemap::tile_materials(&o)).unwrap_or_default();
        let mut sound_rand = crate::audio::Rand::default();
        // The mines' bleeps: the game clock's turns of a second and the mines' meshes' sphere words, by mesh.
        let mut second_ticks = crate::hazards::SecondTicks::default();
        // The rain's lightning (its schedules made with the environment, at the first frame).
        let mut lightning: Option<crate::ambience::Lightning> = None;
        let mine_spheres: std::cell::RefCell<std::collections::HashMap<u32, Option<u32>>> = Default::default();
        // The district's music (FUN_100b72bf, once a second) and the playfield whose districts are loaded.
        let (mut districts, mut district_tick): (Option<(u32, Option<crate::sounds::DistrictMap>)>, Instant) = (None, Instant::now());
        // The district and the weather at the character, for the environment's sounds (refreshed with the music).
        let (mut ambient_district, mut ambient_weather): (Option<u32>, [f32; 14]) = (None, [0.0; 14]);
        // (the effect scripts' playfield and expansion, and Gamecode's `rand()` for the script events)
        let (mut fxs_playfield, mut fxs_expansion, mut script_rand): (Option<u32>, bool, crate::audio::Rand) = (None, false, Default::default());
        // `MAO_MAILBOX=type,instance` (a mail terminal, 0xc773 and its serial) uses it 8 s into the session (or
        // after the walk) and asks for the inbox; the answer is printed (raw when the schema can't read it).
        let mut mailbox: Option<(u32, u32)> = std::env::var("MAO_MAILBOX").ok().and_then(|t| {
            let n: Vec<u32> = t.split(',').filter_map(|v| {
                let v = v.trim();
                v.strip_prefix("0x").map_or_else(|| v.parse().ok(), |h| u32::from_str_radix(h, 16).ok())
            }).collect();
            Some((*n.first()?, *n.get(1)?))
        });
        let mut mailbox_stage: (u8, Instant) = (0, Instant::now());
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
        // Mail: the answer's count and notes for the window, when the inbox request is due, and when the last send went out.
        let (mut pending_mail_count, mut pending_mail_status): (Option<usize>, Option<String>) = (None, None);
        // Shops: the stock to show, a note, the open booth and the trade's bag (from the server's Trade open).
        let (mut pending_shop, mut pending_shop_status): (Option<crate::shop::ShopState>, Option<String>) = (None, None);
        let mut shop_booth: Option<(u32, u32)> = None;
        // After a trade completes the server wants the booth used again (as the original does) for the next one.
        let mut shop_reuse: Option<Instant> = None;
        let (mut pending_mail_entries, mut pending_mail_opened, mut pending_mail_flags): (Option<Vec<mao_net::mail::Entry>>, Option<crate::mailbox::Opened>, Option<(u32, u32)>) = (None, None, None);
        let (mut inbox_due, mut mail_sent): (Option<Instant>, Option<Instant>) = (None, None);
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
        let (mut special_seen, mut social_seen, mut pick_seen) = (0u8, 0u8, 0u8);
        let mut friends_seen = u64::MAX;
        let mut cast_seen = 0u8;
        let mut move_seen = 0u8;
        let mut take_seen = 0u8;
        let mut put_seen = 0u8;
        let mut saved_look: Option<crate::look::Look> = None;
        let mut shot_cache: std::collections::HashMap<u32, (Option<mao_formats::gfx::Visual>, Option<mao_formats::gfx::Visual>)> = std::collections::HashMap::new();
        let mut burst_cache: std::collections::HashMap<u32, Option<mao_formats::gfx::Cast>> = std::collections::HashMap::new();
        let mut aura_cache: std::collections::HashMap<u32, Option<mao_formats::gfx::Visual>> = std::collections::HashMap::new();
        // (the effect templates: what a running nano program looks like)
        let gfx = std::fs::read(Path::new(&game_dir).join("Setupf").join("gfxtweak.bin")).ok().and_then(|d| mao_formats::gfx::Gfx::parse(&d).ok());
        // (the lightning's splash, gfx 0x859b)
        crate::lightning::set_splash(gfx.as_ref().and_then(|g| mao_formats::gfx::Splash::read(g.get(0x859b)?)));
        let mut polymorph_cache: std::collections::HashMap<u32, Option<u32>> = std::collections::HashMap::new();
        let mut vehicle_cache: std::collections::HashMap<u32, bool> = std::collections::HashMap::new();
        let mut delete_seen = 0u8;
        let mut cancel_seen = 0u8;
        let mut inspect_seen = 0u8;
        let mut self_seen = 0u8;
        let (mut following, mut follow_seen): (Option<u32>, u8) = (None, 0);
        let (mut position, mut heading) = (start, heading);
        let (mut moving, mut turning, mut strafing) = (0i8, 0i8, 0i8);
        const JUMP_SPEED: f32 = 7.0;
        const GRAVITY: f32 = 10.0;
        let (mut airborne, mut rise, mut vy, mut was_jumping) = (false, 0.0f32, 0.0f32, false);
        let mut last_tick = Instant::now();
        let mut last_send = Instant::now();
        let mut last_overlay = Instant::now() - Duration::from_secs(1);
        // The map: fog of war revealed as the character walks (see `automap`).
        let mut map = crate::automap::MapBuilder::new(terrain.as_deref(), start, playfield);
        let mut pending_map: Option<crate::automap::MapPatch> = None;
        let mut last_reveal = Instant::now() - Duration::from_secs(1);
        let profile = std::env::var_os("MAO_PROFILE").is_some();
        let mut fake_done = false;
        let mut last_own = (start, Instant::now());
        let send = |connection: &mut Connection, kind: u8, heading: f32, position: [f32; 3], since: &mut Instant| {
            let ms = since.elapsed().as_millis().min(u32::MAX as u128) as u32;
            *since = Instant::now();
            connection.send(&n3::position_update(own, kind, heading, position, ms)).is_ok()
        };
        let mut sent = Instant::now();
        loop {
            // MAO_FAKE_REDIRECT=<seconds>: behave as if the server sent a redirect then (to test the hop without a zone line).
            let fake_due = !fake_done && std::env::var("MAO_FAKE_REDIRECT").ok().and_then(|v| v.parse::<u64>().ok()).is_some_and(|s| clock.elapsed() > Duration::from_secs(s));
            let received = if fake_due {
                fake_done = true;
                Ok(system::redirect_frame([37, 18, 193, 19], 7509))
            } else {
                connection.receive()
            };
            match received {
                Ok(frame) => {
                    // The server's pings are answered (or it ends the connection).
                    if let Some(reply) = frame::ping_reply(&frame, own, 70_000_000 + clock.elapsed().as_millis() as u32) {
                        if connection.send(&reply).is_err() {
                            break;
                        }
                    }
                    if let Some(target) = system::redirect(&frame) {
                        eprintln!("the server sends us to {target}");
                        // The old session must be gone first, or the login is refused (the character is still logged in).
                        connection.close();
                        let say = |text: String| {
                            let _ = tx.send(crate::viewer::Update { messages: vec![text], messages_only: true, ..Default::default() });
                        };
                        say("Zoning...".into());
                        // (straight to the new server with the session key; logging in again is the fallback)
                        let hopped = hop_direct(&target, own, session_key).or_else(|e| {
                            eprintln!("the direct hop failed ({e}); logging in again");
                            std::thread::sleep(Duration::from_millis(1500));
                            hop(&game_dir, &target, own, &say)
                        });
                        match hopped.and_then(|mut c| c.set_timeout(Duration::from_millis(12)).map(|_| c).map_err(|e| e.to_string())) {
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
                    if let Some(m) = Message::parse(&frame).filter(|m| m.kind == 0x333b2867) {
                        if let Some(entries) = mao_net::mail::parse_list(&m.data) {
                            pending_mail_entries = Some(entries);
                        }
                        if let Some(mail) = mao_net::mail::parse_mail(&m.data) {
                            let item = mail.item.and_then(|(low, high, _)| {
                                let lookup = |id: u32| gear_db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: id }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d));
                                let template = lookup(low).or_else(|| lookup(high))?;
                                let icon = template.icon().and_then(|id| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|t| std::sync::Arc::new(crate::gear::Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })));
                                Some((template.name, icon))
                            });
                            pending_mail_opened = Some(crate::mailbox::Opened { mail, item });
                        }
                        if let Some(change) = mao_net::mail::parse_flags(&m.data) {
                            pending_mail_flags = Some(change);
                        }
                        match n3::mail_count(&m.data) {
                            Some((0, n)) => pending_mail_count = Some(n),
                            // Action 8: the mail was accepted (the capture's successful send).
                            Some((8, _)) => {
                                mail_sent = None;
                                pending_mail_status = Some("Mail sent.".into());
                            }
                            _ => {}
                        }
                    }
                    // Right after a send: a notice (a free account is refused) or action 0xba with a message code (the
                    // capture's declined attempts) means the server did not take the mail.
                    if mail_sent.is_some_and(|t| t.elapsed() < Duration::from_secs(5)) {
                        if let Some(m) = Message::parse(&frame) {
                            if m.kind == 0x206b4b73 {
                                mail_sent = None;
                                pending_mail_status = Some("The server declined the mail (sending is limited to paying accounts).".into());
                            } else if m.kind == 0x5e477770 && m.data.get(4) == Some(&0xba) {
                                mail_sent = None;
                                let code = m.data.get(21..25).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
                                pending_mail_status = Some(format!("The server declined the mail (message code {code})."));
                            }
                        }
                    }
                    if let Some(m) = Message::parse(&frame).filter(|m| m.kind == 0x58362220) {
                        if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                            let int = |v: &mao_net::schema::Value, f: &str| v.get(f).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                            if let Some(mao_net::schema::Value::List(list)) = p.value.get("VendingMachineSlots") {
                                let stock = list
                                    .iter()
                                    .map(|s| {
                                        let (low, high, quality) = (int(s, "ItemLowId"), int(s, "ItemHighId"), int(s, "Quality"));
                                        let lookup = |id: u32| gear_db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: id }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d));
                                        match lookup(low).or_else(|| lookup(high)) {
                                            Some(t) => {
                                                let icon = t.icon().and_then(|id| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|x| std::sync::Arc::new(crate::gear::Icon { id, width: x.width as usize, height: x.height as usize, rgba: x.rgba })));
                                                crate::shop::Slot { low, high, quality, name: t.name, description: t.description, icon }
                                            }
                                            None => crate::shop::Slot { low, high, quality, name: format!("Item #{low:x}"), description: String::new(), icon: None },
                                        }
                                    })
                                    .collect();
                                shop_booth = Some(m.identity);
                                pending_shop = Some(crate::shop::ShopState { shop: m.identity, stock });
                            }
                        }
                    }
                    // Trade action 4 from the server (captured after both a purchase and a sale): the trade completed.
                    if Message::parse(&frame).is_some_and(|m| m.kind == 0x36284f6e && m.identity.0 == 0xc350 && m.data.get(5) == Some(&4)) && shop_booth.is_some() {
                        pending_shop_status = Some("Trade complete.".into());
                        shop_reuse = Some(Instant::now() + Duration::from_millis(300));
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
                        match mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                            Ok(p) if m.kind != 0x333b2867 => eprintln!("[{:6.1}] {} identity ({:x},{:x}) left {}{}", clock.elapsed().as_secs_f32(), p.name, m.identity.0, m.identity.1, p.left, p.value.pretty()),
                            // Mail is shown raw too: the schema only models sending.
                            _ => eprintln!("[{:6.1}] kind {:08x} identity ({:x},{:x}) {} bytes: {}", clock.elapsed().as_secs_f32(), m.kind, m.identity.0, m.identity.1, m.data.len(), m.data.iter().map(|b| format!("{b:02x}")).collect::<String>()),
                        }
                    }
                    state.add(&frame);
                    state.refresh_mods(&gear_db);
                    if let Some(list) = state.destinations.take() {
                        pending_destinations = Some(list);
                    }
                    if let Some(entries) = state.inventory.take() {
                        state.worn = entries.clone();
                        state.mods_dirty = true;
                        pending_gear = Some(crate::gear::from_entries(&gear_db, &gear_textures, &entries, state.stats.get(&0x36).copied().unwrap_or(1)));
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
                    teleport_try = None;
                    current = playfield;
                    state.last_move.clear_all();
                    state.jumped.clear_all();
                    state.last_seen.clear_all();
                    state.speeds.clear_all();
                    state.names.clear_all();
                    state.infos.clear_all();
                    state.rotations.clear_all();
                    state.entities.clear();
                    state.props.clear();
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
                    match load_area_with(&game_dir, &thread_db, playfield, Some(&thread_textures)) {
                        Ok(area) => {
                            (terrain, collision) = (area.terrain, area.collision);
                            state.set_static_props(crate::static_props(&thread_db, playfield));
                            let mut scene = area.scene;
                            if terrain.is_none() && crate::load_rooms(&thread_db, playfield).is_none() {
                                // An interior: the floor isn't readable, so lay a plain one at the arrival height.
                                scene.parts.push(floor_part(arrival[0], arrival[2], arrival[1] - 0.03));
                                scene.instances.push((scene.parts.len() - 1, model::IDENTITY));
                            }
                            pending_scene = Some(scene);
                            booths = area.booths;
                            areas = area.areas;
                            base_environment = area.base_environment;
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
                    map = crate::automap::MapBuilder::new(terrain.as_deref(), arrival, playfield);
                    pending_map = None;
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
            // Vicinity speech: a text message to the server, at the current target (a character's; none for an object or nobody).
            for (kind, text, record) in std::mem::take(&mut *SPEECH.lock().unwrap_or_else(|e| e.into_inner())) {
                let object = match kind {
                    1 => 2,
                    2 => 4,
                    _ => 3,
                };
                let at = target.filter(|t| !state.objects.contains_key(t)).map_or((0, 0), |t| (n3::CHARACTER, t));
                if let Some(data) = n3::vicinity_data(&text, kind, &record) {
                    let _ = connection.send(&n3::text_message(own, object, at, &data));
                }
            }
            // `/selectself`: the character is its own target.
            if raw.select_self != self_seen {
                self_seen = raw.select_self;
                target = Some(own);
                let _ = connection.send(&n3::select_target(own, own));
                let _ = connection.send(&n3::set_target(own, own));
            }
            // `/inspect`: ask for the target's info packet and tell what comes.
            if raw.inspect != inspect_seen {
                inspect_seen = raw.inspect;
                match target.filter(|t| !state.objects.contains_key(t)) {
                    Some(t) => {
                        state.inspecting = Some(t);
                        let _ = connection.send(&n3::info_request(own, t));
                    }
                    None => pending_messages.push("Nothing targeted to inspect (Tab selects)".into()),
                }
            }
            // Follow (`/follow`): walk after the target; a movement key of the player's own stops it.
            if raw.follow != follow_seen {
                follow_seen = raw.follow;
                if following.take().is_some() {
                    goto = None;
                    pending_messages.push("Stopped following".into());
                } else if let Some(t) = target.filter(|t| state.entities.contains_key(t)) {
                    following = Some(t);
                    pending_messages.push(format!("Following {}", state.names.get(&t).map_or("the target", |n| n.0.as_str())));
                } else {
                    pending_messages.push("Nothing to follow (Tab selects a target)".into());
                }
            }
            if following.is_some() && (raw.forward || raw.back || raw.strafe != 0 || raw.turn != 0 || raw.jump) {
                following = None;
                pending_messages.push("Stopped following".into());
            }
            if let Some(id) = following {
                match state.entities.get(&id) {
                    Some((p, _)) => {
                        let distance = ((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt();
                        if distance > 3.5 {
                            // (the progress watch only restarts when a walk starts: the target keeps moving)
                            if goto.is_none() {
                                (goto_best, goto_progress, detour, detours) = (f32::MAX, Instant::now(), None, 0);
                            }
                            goto = Some([p[0], p[2]]);
                        } else {
                            goto = None;
                        }
                    }
                    None => {
                        following = None;
                        goto = None;
                        pending_messages.push("The target is gone: stopped following".into());
                    }
                }
            }
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
                            state.door_used(t);
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
                let mut candidates: Vec<(f32, u32)> = state.infos.iter().filter(|(id, i)| **id != own && ((i.is_player() && **id < DYNAMIC_NPC) || friendly_name(&i.name)) == wants_friendly).filter_map(|(id, _)| Some((near(state.entities.get(id)?.0), *id))).filter(|(d, _)| *d < 80.0).collect();
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
                    let _ = connection.send(&n3::info_request(own, id));
                }
            }
            // The mail windows: check the inbox at the nearest known terminal, or send a mail.
            while let Ok(command) = mail_rx.try_recv() {
                match command {
                    crate::mailbox::MailCommand::Inbox => match crate::mailbox::nearest_terminal(&thread_db, current, position[0], position[2]) {
                        Some(serial) => {
                            uses += 1;
                            let _ = connection.send(&n3::use_object(own, uses, (0xc773, serial)));
                            inbox_due = Some(Instant::now() + Duration::from_millis(900));
                        }
                        None => pending_mail_status = Some("No known mail terminal here (they come from captures: Old Athens has two).".into()),
                    },
                    crate::mailbox::MailCommand::Open(id) => {
                        let _ = connection.send(&mao_net::mail::open(own, id));
                    }
                    crate::mailbox::MailCommand::Take(id) => {
                        let _ = connection.send(&mao_net::mail::take(own, id));
                    }
                    crate::mailbox::MailCommand::Delete(id) => {
                        let _ = connection.send(&mao_net::mail::delete(own, id));
                        // The list again, a moment later (the server doesn't push it).
                        inbox_due = Some(Instant::now() + Duration::from_millis(600));
                    }
                    crate::mailbox::MailCommand::Send { recipient, subject, body, item, credits, express } => {
                        let _ = connection.send(&n3::mail_send(own, &recipient, &subject, &body, item, credits, express));
                        mail_sent = Some(Instant::now());
                    }
                }
            }
            while let Ok(command) = shop_rx.try_recv() {
                match command {
                    crate::shop::ShopCommand::Close => {
                        let _ = connection.send(&n3::trade_decline(own));
                        shop_booth = None;
                    }
                    // As captured: add the stock item (action 5: the booth, 0x6f, the stock index), then accept (action 1).
                    crate::shop::ShopCommand::Buy(index) => match shop_booth {
                        Some(booth) => {
                            let _ = connection.send(&n3::trade(own, 5, [booth.0, booth.1, 0x6f, index as u32]));
                            let _ = connection.send(&n3::trade(own, 1, [0, 0, 0, 0]));
                            pending_shop_status = Some("Buying...".into());
                        }
                        None => pending_shop_status = Some("No open trade to buy in.".into()),
                    },
                    // As captured: add one of your own items (action 5: yourself, the inventory page 0x68, the slot), then accept.
                    crate::shop::ShopCommand::Sell(slot) => match shop_booth {
                        Some(_) => {
                            let _ = connection.send(&n3::trade(own, 5, [0xc350, own, 0x68, slot]));
                            let _ = connection.send(&n3::trade(own, 1, [0, 0, 0, 0]));
                            pending_shop_status = Some("Selling...".into());
                        }
                        None => pending_shop_status = Some("No open trade to sell in.".into()),
                    },
                }
            }
            if shop_reuse.is_some_and(|t| Instant::now() >= t) {
                shop_reuse = None;
                if let Some(booth) = shop_booth {
                    uses += 1;
                    let _ = connection.send(&n3::use_object(own, uses, booth));
                }
            }
            if inbox_due.is_some_and(|t| Instant::now() >= t) {
                inbox_due = None;
                let _ = connection.send(&n3::mail_request(own));
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
            // The mailbox test: use the terminal once nothing is walking, then ask for the inbox.
            if let Some((kind, id)) = mailbox {
                if goto.is_none() && clock.elapsed() > Duration::from_secs(8) {
                    if mailbox_stage.0 == 0 {
                        mailbox_stage = (1, Instant::now());
                        eprintln!("using mail terminal ({kind:#x}, {id:#x})");
                        uses += 1;
                        let _ = connection.send(&n3::use_object(own, uses, (kind, id)));
                    } else if mailbox_stage.0 == 1 && mailbox_stage.1.elapsed() > Duration::from_secs(2) {
                        mailbox = None;
                        eprintln!("asking for the inbox");
                        let _ = connection.send(&n3::mail_request(own));
                    }
                }
            }
            // The Action window and the hotbar: special actions and social actions.
            if keys.special.0 != special_seen {
                special_seen = keys.special.0;
                let id = keys.special.1;
                let name = crate::actions_data::ACTIONS.iter().find(|a| a.0 == id).map_or("that action", |a| a.2);
                match name {
                    "Stand" => {
                        let _ = connection.send(&n3::action(own, n3::STAND));
                    }
                    "Sneak" => {
                        let _ = connection.send(&n3::action(own, 0xa3));
                    }
                    "Search" => {
                        let _ = connection.send(&n3::action(own, 0x66));
                    }
                    "Reload" => {
                        let _ = connection.send(&n3::action(own, 0xd2));
                    }
                    // The weapon specials are skills: the message names the skill's stat.
                    "Brawl" | "Dimach" | "Burst" | "Fling Shot" | "Full Auto" => match target {
                        Some(t) => {
                            let stat = match name {
                                "Brawl" => 142,
                                "Dimach" => 144,
                                "Burst" => 148,
                                "Fling Shot" => 150,
                                _ => 167,
                            };
                            let _ = connection.send(&n3::special_attack(own, t, stat));
                            pending_messages.push(format!("{name} at {}", state.names.get(&t).map_or("the target", |n| n.0.as_str())));
                        }
                        None => pending_messages.push(format!("{name} needs a target (Tab selects one)")),
                    },
                    "Start Combat" => match target {
                        Some(t) => {
                            let _ = connection.send(&n3::attack(own, t));
                            pending_messages.push(format!("Attacking {}", state.names.get(&t).map_or("the target", |n| n.0.as_str())));
                        }
                        None => pending_messages.push("No target (Tab selects one)".into()),
                    },
                    other => match crate::actions_data::ACTIONS.iter().find(|a| a.0 == id && a.1 == crate::actions::Kind::Perk && a.3.len() == 4) {
                        // A perk: its id and its four-letter hash name it (the server uses the current target).
                        Some(&(_, _, perk_name, hash, perk_id)) => {
                            let h: [u8; 4] = hash.as_bytes().try_into().unwrap_or([0; 4]);
                            let _ = connection.send(&n3::use_perk(own, perk_id, h));
                            pending_messages.push(format!("Using {perk_name}"));
                        }
                        None => pending_messages.push(format!("{other}: its network message is not captured yet")),
                    },
                }
            }
            // A social action (Gamecode's `N3Msg_DoSocialAction`, 0x10026b4f): lounging and sleeping (0x45, 0x44) only sitting
            // ("Feedback_MustSitToLoungeOrSleep"), else a `SocialActionCmd_t` of the character to the server.
            if keys.social.0 != social_seen {
                social_seen = keys.social.0;
                let id = keys.social.1;
                if (id == 0x44 || id == 0x45) && !sitting {
                    pending_messages.push(crate::feedback::plain("Feedback_MustSitToLoungeOrSleep"));
                } else {
                    let _ = connection.send(&n3::social_action(own, id));
                }
            }
            // A click on something in the world selects it; a double click uses it.
            if keys.pick.0 != pick_seen {
                pick_seen = keys.pick.0;
                let (_, id, double) = keys.pick;
                target = Some(id);
                if !state.objects.contains_key(&id) {
                    let _ = connection.send(&n3::select_target(own, id));
                    let _ = connection.send(&n3::set_target(own, id));
                    let _ = connection.send(&n3::info_request(own, id));
                }
                if double {
                    uses += 1;
                    let kind = state.types.get(&id).copied().unwrap_or(n3::CHARACTER);
                    let _ = connection.send(&n3::use_object(own, uses, (kind, id)));
                    state.door_used(id);
                    pending_messages.push(format!("Using {}", state.names.get(&id).map_or("the target", |n| n.0.as_str())));
                }
            }
            // A running nano cancelled: asked of the server and taken off the NCU window at once.
            if keys.cancel.0 != cancel_seen {
                cancel_seen = keys.cancel.0;
                let nano = keys.cancel.1;
                let _ = connection.send(&n3::cancel_nano(own, nano));
                state.nano_times.retain(|t| t.0 != nano);
                state.nanos.retain(|n| *n != nano);
                state.nano_list_changed = true;
                state.mods_dirty = true;
                pending_messages.push("Nano cancelled".into());
            }
            // An item destroyed: asked of the server and taken out of the inventory window at once.
            if keys.delete.0 != delete_seen {
                delete_seen = keys.delete.0;
                let place = keys.delete.1;
                if let Some(i) = state.worn.iter().position(|e| e.slot == place) {
                    let entry = state.worn.remove(i);
                    let _ = connection.send(&n3::delete_item(own, place, entry.low_id, entry.high_id));
                    state.mods_dirty = true;
                    pending_gear = Some(crate::gear::from_entries(&gear_db, &gear_textures, &state.worn, state.stats.get(&0x36).copied().unwrap_or(1)));
                    pending_messages.push("Item deleted".into());
                }
            }
            // A corpse that just opened is emptied when the setting says so.
            if let Some((handle, places)) = state.fresh_corpse.take() {
                if keys.auto_loot {
                    for place in places {
                        let _ = connection.send(&n3::take_from_container(own, handle, place));
                    }
                }
            }
            // An inventory item put into the open container.
            if keys.put.0 != put_seen {
                put_seen = keys.put.0;
                let (_, kind, id, from) = keys.put;
                let _ = connection.send(&n3::container_add_item(own, (kind, id), (0x68, from)));
            }
            // An item taken from the open container (or all of them).
            if keys.take.0 != take_seen {
                take_seen = keys.take.0;
                let (_, handle, place) = keys.take;
                let places: Vec<u32> = if place == u32::MAX { state.loot.iter().flat_map(|l| l.entries.iter().map(|e| e.slot)).collect() } else { vec![place] };
                for place in places {
                    let _ = connection.send(&n3::take_from_container(own, handle, place));
                }
            }
            // An item dragged to another place: ask the server, and show the change at once (the server's answer to a move is not
            // known, so a refused move is not taken back here).
            if keys.mv.0 != move_seen {
                move_seen = keys.mv.0;
                let (from, to) = (keys.mv.1, keys.mv.2);
                if let Some(i) = state.worn.iter().position(|e| e.slot == from) {
                    let _ = connection.send(&n3::move_item(own, from, to));
                    if let Some(j) = state.worn.iter().position(|e| e.slot == to) {
                        state.worn[j].slot = from;
                    }
                    state.worn[i].slot = to;
                    state.mods_dirty = true;
                    pending_gear = Some(crate::gear::from_entries(&gear_db, &gear_textures, &state.worn, state.stats.get(&0x36).copied().unwrap_or(1)));
                }
            }
            // A nano program to cast, at the target (not a usable object) or at oneself.
            if keys.cast.0 != cast_seen {
                cast_seen = keys.cast.0;
                state.asked_casts.insert(keys.cast.1, Instant::now());
                let plays = state.nano_casts.gather(own, keys.cast.1, &|n, s| nano_stat(n, s));
                state.cast_sounds.extend(plays.into_iter().map(|(id, delay, volume)| (own, id, delay, volume)));
                let aim = target.filter(|t| !state.objects.contains_key(t));
                let _ = connection.send(&n3::cast_nano(own, aim, keys.cast.1));
                let name = thread_db.get(mao_formats::resource::Id { kind: 0xfde85, instance: keys.cast.1 }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)).map_or_else(|| format!("nano #{}", keys.cast.1), |t| t.name);
                // (N3Msg_CastNanoSpell, 0x1001b5d4: feedback 0x47 with the nano's name)
                state.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, crate::feedback::executing(&name)));
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
            // (fighting, the character turns to its target while it stands: `Facing`; any steering counts as moving)
            if let Some(target_at) = state.fighting.get(&own).and_then(|&t| state.position_of(t)) {
                let steering = walk != 0 || keys.turn != 0 || keys.strafe != 0 || keys.face.is_some() || airborne;
                heading = state.facings.entry(own).or_default().step(heading, position, target_at, steering, dt);
            } else {
                state.facings.remove(&own);
            }
            // (a sidestep on its own shows a polymorph's rolling or sidestepping animation)
            state.strafing.insert(own, if walk == 0 { keys.strafe.signum() } else { 0 });
            state.backing.insert(own, walk < 0);
            // (the speeds the original moves at, from the character's RunSpeed and health)
            #[allow(non_snake_case)]
            let (SPEED, BACK_SPEED) = movement_speeds(&state.stats);
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
                if keys.noclip || !collision.blocked(nx, nz, base + rise, 0.4, 1.8, 0.5) {
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
            }
            if start_jump {
                (airborne, vy) = (true, JUMP_SPEED);
            }
            // Stand on the ground or on a floor or step within reach.
            // (In the air the surfaces under the feet count too: a jump clears a rock and lands on top of it.)
            let height = base + rise;
            let floor = if keys.noclip { f32::MIN } else { collision.floor(position[0], position[2], height, 0.6, if airborne { rise + 0.6 } else { 0.6 }).unwrap_or(f32::MIN) };
            base = (ground(&terrain, position) + offset).max(floor);
            rise = height - base;
            if airborne && vy <= 0.0 && rise <= 0.0 {
                (airborne, rise, landed) = (false, 0.0, true);
            } else if airborne && rise < 0.0 {
                rise = 0.0;
            } else if !airborne {
                rise = 0.0;
            }
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
            // Touching a door starts a teleport try, as the original's collision with a `Door_t` does (`StartTeleportTry`): one full
            // position report, and the server zones us (a playfield message) or does not; after 30 s the try is given up with the
            // original's feedback. A new try needs the door left and touched again.
            // The sound system's frame (AFCM's frame processor: `SandyInterfaceModule_t::FrameProcess`), heard from the
            // character (FUN_100b1f3c: `SetListenerRotPos` with the controlled dynel's global position).
            crate::audio::with(|m| {
                m.set_listener(position);
                m.frame_process(dt);
            });
            // The effect scripts (`fxs`): the playfield's tweaks once it is current (PlayfieldInit FUN_100b2478: FUN_100b1e95's
            // `LoadPFTweakData`, outdoors 0 or indoors 1 — the environment types 2 to 12 are not known — then the automatic-run
            // objects), Notum Wars merged in (FUN_100b1f3c: `SetExpansion` of stat 389 once), the script events
            // (FUN_10130a1b), then the frame (`VisualEnvFX_t::FrameProcess` from FUN_100b05b4: FXS's frame, then the game
            // interface's variables: the day time is the clock's seconds over its pace, the server's clock, the character's
            // place as the eye's; no storm, precipitation or wind) and the sounds it played.
            if fxs_playfield != Some(current) {
                fxs_playfield = Some(current);
                crate::fxs::set_data_path(&Path::new(&game_dir).join("cd_image"));
                crate::fxs::load_playfield(current, if terrain.is_some() { 0 } else { 1 });
                crate::fxs::start_automatic();
            }
            if state.expansion && !fxs_expansion {
                fxs_expansion = true;
                crate::fxs::set_expansion(state.stats.get(&389).copied().unwrap_or(0) as u32);
            }
            for iir in std::mem::take(&mut state.script_events) {
                let now = state.game_clock.server_time();
                let start = state.music_overrides.script(&iir, now, crate::fxs::has_event());
                let text = mao_formats::ldb::format(&iir.script, &[mao_formats::ldb::Arg::Int(iir.length as i64), mao_formats::ldb::Arg::Int(start as i64)]);
                if iir.kind != 3 {
                    crate::fxs::script_event(iir.kind, &text, iir.place, position, now, &mut script_rand);
                }
            }
            crate::fxs::frame_process();
            let (system_time, system_second) = state.game_clock.server_clock();
            crate::fxs::set_game(&crate::fxs::Game {
                delta_time: dt,
                day_time: state.game_clock.seconds() / crate::gametime::PACE,
                system_time,
                system_second,
                camera_position: position,
                camera_rotation: [0.0, 1.0, 0.0, 0.0],
                ..Default::default()
            });
            for s in crate::fxs::take_sounds() {
                let r = crate::fxs::math::round;
                crate::audio::with(|m| m.play_game_sound(s.id, s.pos, s.f10, s.volume, s.i18, s.f1c, r(s.f20), r(s.f24), r(s.f28) as u16 as u32, s.vel));
            }
            for (fxid, _) in crate::fxs::take_started() {
                if std::env::var_os("MAO_FXS").is_some() {
                    eprintln!("fxs: effect {fxid} started");
                }
            }
            for m in crate::fxs::take_messages() {
                if std::env::var_os("MAO_FXS").is_some() {
                    eprintln!("fxs: {m}");
                }
            }
            // (the district's music each second: FUN_100b72bf → `ActivateEnvZone`)
            if district_tick.elapsed() >= Duration::from_secs(1) {
                district_tick = Instant::now();
                if districts.as_ref().is_none_or(|d| d.0 != current) {
                    // (a new playfield: its own overrides)
                    if districts.is_some() {
                        state.music_overrides.clear();
                    }
                    districts = Some((current, crate::sounds::DistrictMap::load(&gear_db, current)));
                }
                ambient_weather = crate::environment::compute(&areas, base_environment.as_ref(), position, state.game_clock.day(), state.game_clock.seconds(), state.game_clock.day_intensity()).weather;
                // (FUN_100b1f3c: the expansion flag, set once the character's stat 389 has bit 0)
                state.expansion |= state.stats.get(&389).is_some_and(|s| s & 1 != 0);
                let expansion = state.expansion;
                let (overrides, now) = (&mut state.music_overrides, state.game_clock.server_time());
                let over = |name: &str| {
                    // (FindLayerID of a named override, when the sound system is up)
                    let o = overrides.get(name, now).filter(|o| !o.is_empty())?;
                    crate::audio::with(|m| m.find_layer(&o))
                };
                let music = districts.as_ref().and_then(|d| d.1.as_ref()).and_then(|d| d.music(position, state.game_clock.time().0, &ambient_weather, expansion, over));
                if let Some((district, layer)) = music {
                    crate::audio::with(|m| m.activate_env_zone(district, layer));
                }
                ambient_district = music.map(|m| m.0);
            }
            // (the environment's sounds each frame, FUN_100becbe: the district's background by the time of day, the rain)
            if let Some(district) = ambient_district {
                for id in crate::ambience::background(state.game_clock.seconds(), district) {
                    crate::ambience::play(id, 1.0);
                }
            }
            let (rain, volume) = crate::ambience::rain(&ambient_weather);
            crate::ambience::play(rain, volume);
            // (the lightning's thunder, FUN_100b9da8: `PlayEnvSound(NearLightning, origin, 0, volume, 0, deferral, 100)`)
            {
                let zone_side = districts.as_ref().and_then(|d| d.1.as_ref()).and_then(|d| d.zone_side());
                let now = state.game_clock.seconds();
                let thunder = lightning.get_or_insert_with(|| crate::ambience::Lightning::new(now)).frame(&ambient_weather, now, position, zone_side);
                for t in thunder {
                    let id = mao_formats::sound::sound_id("SM_Sandy_Env_NearLightning");
                    crate::audio::with(|m| m.play_env_sound(id, [0.0; 3], 0.0, t.volume, 0, t.defer, 100));
                    state.strikes.push((t.at, t.seed));
                    state.jolts.extend(t.jolt);
                }
            }
            // (the fight's report for the combat music, FUN_10059c7d: the character's health in percent of its maximum, at
            // least 1 while alive, its stat 421, side (33), instance, not an NPC, and level (54))
            {
                let stat = |s: u32| state.stats.get(&s).copied().unwrap_or(0);
                let (health, max) = (stat(27), stat(1));
                if max != 0 {
                    let mut percent = ((health as f64 / max as f64) * 100.0) as i64;
                    percent = percent.min(100);
                    if percent < 1 && health > 0 {
                        percent = 1;
                    }
                    crate::audio::with(|m| m.combat_update(true, percent as i32, stat(421) as i32, stat(33) as i32, own as i32, false, stat(54) as i32));
                }
            }
            let at_door = state.door_slots.values().any(|(_, d)| ((d[0] - position[0]).powi(2) + (d[2] - position[2]).powi(2)).sqrt() < 2.0 && (d[1] - position[1]).abs() < 3.0);
            match teleport_try {
                None if at_door => {
                    teleport_try = Some(Instant::now());
                    send_kind(movement::FULL, &mut connection);
                    eprintln!("[{:6.1}] teleport try at a door, from {position:?}", clock.elapsed().as_secs_f32());
                }
                Some(began) if began.elapsed() >= Duration::from_secs(30) => {
                    teleport_try = None;
                    pending_messages.push("Area change not initiated".into());
                }
                Some(_) if !at_door => teleport_try = None,
                _ => {}
            }
            if start_jump {
                state.jumped.insert(own, Instant::now());
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
                // (the periodic report is the original's full one, 0x16; 15 is what the captures show as the character lands)
                send_kind(if landed { movement::JUMP_STOP } else { movement::FULL }, &mut connection);
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
            // (and the others fighting turn the same way, between the server's rotations)
            let fighters: Vec<(u32, u32)> = state.fighting.iter().map(|(&a, &b)| (a, b)).filter(|&(id, _)| id != own).collect();
            state.facings.retain(|id, _| *id == own || state.fighting.contains_key(id));
            for (id, target) in fighters {
                let (Some(at), Some(target_at)) = (state.position_of(id), if target == own { Some(position) } else { state.position_of(target) }) else { continue };
                let moving = state.last_move.get(&id).is_some_and(|t| t.elapsed() < Duration::from_millis(900));
                let q = state.rotations.get(&id).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]);
                let before = 2.0 * q[1].atan2(q[3]);
                let after = state.facings.entry(id).or_default().step(before, at, target_at, moving, dt);
                if after != before {
                    state.rotations.insert(id, [0.0, (after * 0.5).sin(), 0.0, (after * 0.5).cos()]);
                }
            }
            if walk != 0 || keys.turn != 0 || keys.strafe != 0 || moving != 0 || airborne {
                state.entities.insert(own, (position, 0));
                if walk != 0 || keys.strafe != 0 {
                    state.last_move.insert(own, Instant::now());
                    state.speeds.insert(own, speed.abs());
                }
            } else if let Some(e) = state.entities.get(&own) {
                if (0..3).map(|k| (e.0[k] - position[k]).powi(2)).sum::<f32>().sqrt() > 3.0 {
                    eprintln!("[{:6.1}] the server moved us from {position:?} to {:?}", clock.elapsed().as_secs_f32(), e.0);
                }
                position = e.0;
            }
            if last_reveal.elapsed() >= Duration::from_millis(120) {
                last_reveal = Instant::now();
                if let Some(patch) = map.reveal(position, terrain.as_deref(), offset, &collision) {
                    // (Patches pile up until the interface takes them; a new grid replaces what came before.)
                    pending_map = Some(match pending_map.take() {
                        Some(mut old) if patch.reset.is_none() => {
                            old.cells.extend(patch.cells);
                            old
                        }
                        Some(mut old) => {
                            old.cells.clear();
                            old.cells.extend(patch.cells);
                            old.reset = patch.reset;
                            old
                        }
                        None => patch,
                    });
                }
            }
            if last_overlay.elapsed() >= Duration::from_millis(33) {
                last_overlay = Instant::now();
                let built = Instant::now();
                // (characters farther than the setting are not drawn; 0 draws all)
                let reach = keys.char_distance;
                let near = |id: u32| reach <= 0.0 || state.entities.get(&id).is_none_or(|e| ((e.0[0] - position[0]).powi(2) + (e.0[2] - position[2]).powi(2)).sqrt() <= reach);
                // Other characters under a polymorph program: the server repeats their buffs, so one seen lately is still running.
                for (who, nano) in std::mem::take(&mut state.buffs_seen) {
                    let creature = *polymorph_cache.entry(nano).or_insert_with(|| {
                        let data = gear_db.get(mao_formats::resource::Id { kind: 0xfde85, instance: nano }).ok().flatten()?;
                        mao_formats::item::Item::parse(&data).filter(|t| t.stat(75) == Some(mao_formats::item::POLYMORPH_STRAIN))?;
                        let creature = mao_formats::item::polymorph_template(&data)?;
                        gear_db.get(mao_formats::resource::Id { kind: 0xfde97, instance: creature }).ok().flatten().map(|_| creature)
                    });
                    if let Some(creature) = creature {
                        state.morphed_until.insert(who, (creature, Instant::now()));
                    }
                    // (and on a vehicle program, its board or bike)
                    let vehicle = *vehicle_cache.entry(nano).or_insert_with(|| gear_db.get(mao_formats::resource::Id { kind: 0xfde85, instance: nano }).ok().flatten().is_some_and(|d| mao_formats::item::is_vehicle_program(&d)));
                    if vehicle {
                        state.riding_until.insert(who, (nano, Instant::now()));
                    }
                }
                state.riding_until.retain(|_, (_, at)| at.elapsed() < Duration::from_secs(30));
                for (who, (nano, _)) in &state.riding_until {
                    state.riding.insert(*who, *nano);
                }
                state.riding.retain(|who, _| *who == own || state.riding_until.contains_key(who));
                state.morphed_until.retain(|_, (_, at)| at.elapsed() < Duration::from_secs(30));
                for (who, (creature, _)) in &state.morphed_until {
                    state.morphs.insert(*who, *creature);
                }
                state.morphs.retain(|who, _| *who == own || state.morphed_until.contains_key(who));
                // (the deformer hits run their course: the control ends at its duration)
                state.deforms.retain(|_, (d, at)| d.duration <= 0.0 || at.elapsed().as_secs_f32() < d.duration);
                let (model, mut casters) = overlay_model(&state, &state.props, &mut characters, &|id| id != own && near(id));
                let (own_model, own_casters) = overlay_model(&state, &state.props, &mut characters, &|id| id == own);
                // The notes the animations went past play their sounds at the characters (FUN_100452d3), on the material
                // under each one; and the deaths theirs.
                if let Some(c) = &mut characters {
                    // (the effects' sound controls, made with the effects: a cast's at the caster, a tracer's at the caster
                    // and a hit's at the target as the shot lands)
                    if let Some(g) = gfx.as_ref() {
                        let place = |id: u32| if id == own { Some(position) } else { state.position_of(id) };
                        let mut made: Vec<(u32, i32, f32)> = Vec::new();
                        for &(caster, _, nano) in &state.cast_bursts {
                            made.extend(nano_stat(nano, 428).map(|e| (caster, e as i32, 0.0)));
                        }
                        for &(caster, target, nano) in &state.shots_pending {
                            made.extend(nano_stat(nano, 419).map(|e| (caster, e as i32, 0.0)));
                            made.extend(nano_stat(nano, 361).map(|e| (target, e as i32, if caster != target { crate::effects::FLIGHT } else { 0.0 })));
                        }
                        for (at, effect, later) in made {
                            let Some(p) = place(at) else { continue };
                            for s in crate::sounds::effect_sounds(g, effect) {
                                crate::audio::with(|m| m.play_sound_command(s.id, p, s.velocity, s.volume, s.range, s.delay, s.defer + later, s.pitch));
                            }
                        }
                    }
                    let frame = state.nano_casts.frame(&|n, s| nano_stat(n, s));
                    state.cast_sounds.extend(frame);
                    for (at, id, delay, volume) in std::mem::take(&mut state.cast_sounds) {
                        let Some(p) = (if at == own { Some(position) } else { state.position_of(at) }) else { continue };
                        crate::audio::with(|m| m.play_game_sound(id, p, delay, volume, 0, 0.0, 100, 1, 0, [0.0; 3]));
                    }
                    // (the hazards' sounds: `PlayGameSound(id, at, 0, 1, 0, 0, 100, 1)`, FUN_10085b15, FUN_10099536, FUN_1007e0a8)
                    crate::voices::set_stat_389(state.stats.get(&389).copied().unwrap_or(0));
                    // (who speaks a `/voice` line: the character's name, breed (stat 4) and sex (stat 59))
                    crate::voices::set_own(
                        state.names.get(&own).map(|n| n.0.clone()).unwrap_or_default(),
                        state.stats.get(&4).copied().unwrap_or(0) as u8,
                        state.stats.get(&59).copied().unwrap_or(0) as u8,
                    );
                    let controllers = state.hazards.frame();
                    state.hazard_sounds.extend(controllers);
                    for (id, at) in std::mem::take(&mut state.hazard_sounds) {
                        crate::audio::with(|m| m.play_game_sound(id, at, 0.0, 1.0, 0, 0.0, 100, 1, 0, [0.0; 3]));
                    }
                    // (a mine's bleep: the client's body sphere from its breed's and sex's body mesh, the mine's from its
                    // `Mesh` stat's mesh, each by its scale, stat 360 / 100; the game clock's turns of a second)
                    let turned = second_ticks.step();
                    if state.hazards.map.values().any(|h| h.kind == crate::hazards::Kind::Mine) {
                        let own_sphere = state.infos.get(&own).and_then(|i| {
                            c.ensure_body(i.breed, i.sex);
                            let packed = c.bodies.get(&(i.breed, i.sex))?.as_ref()?.0.colour;
                            let scale = state.stats.get(&360).map_or(1.0, |&s| s as f32 / 100.0);
                            Some(crate::hazards::body_sphere(packed, scale))
                        });
                        if let Some(own_sphere) = own_sphere {
                            let db = c.db.clone();
                            let meshes = &mine_spheres;
                            let sphere = |h: &crate::hazards::Hazard| -> Option<(f32, f32)> {
                                let mesh = *h.stats.get(&12)? as u32;
                                let packed = *meshes.borrow_mut().entry(mesh).or_insert_with(|| {
                                    let object = db.get(mao_formats::resource::Id { kind: 0xf6951, instance: mesh }).ok()??;
                                    Some(mao_formats::mesh::parse(&object).ok()?.colour)
                                });
                                let scale = h.stats.get(&360).map_or(1.0, |&s| s as f32 / 100.0);
                                Some(crate::hazards::body_sphere(packed?, scale))
                            };
                            let frame = crate::hazards::Frame { second: turned.0, other_second: turned.1, own: position, own_sphere, sphere: &sphere };
                            for (id, at, pitch) in state.hazards.bleeps(&frame) {
                                crate::audio::with(|m| m.play_game_sound(id, at, 0.0, 1.0, 0, 0.0, pitch as i32, 1, 0, [0.0; 3]));
                            }
                        }
                    }
                    for id in std::mem::take(&mut state.died) {
                        let Some(at) = (if id == own { Some(position) } else { state.position_of(id) }) else { continue };
                        let info = state.infos.get(&id);
                        let set = info.and_then(|i| i.template).and_then(|t| c.set_of(t));
                        if let Some(sound) = crate::sounds::death(set.map(|s| (s, sound_rand.next() as u32)), info.map_or(0, |i| i.sex)) {
                            crate::audio::with(|m| m.play_game_sound(sound, at, 0.0, 1.0, 0, 0.0, 100, 1, 0, [0.0; 3]));
                        }
                    }
                    for (id, event, template) in std::mem::take(&mut c.notes) {
                        let Some(at) = (if id == own { Some(position) } else { state.position_of(id) }) else { continue };
                        let set = template.and_then(|t| c.set_of(t)).map(|t| (t, sound_rand.next() as u32));
                        // (an attack's notes, FUN_10068e42: the weapon of the swing's place, the animation's +8, and what the
                        // attack put on it)
                        if matches!(event, 0xb | 0x8e..=0x91) {
                            let Some(&(slot, ..)) = state.swings.get(&id) else { continue };
                            let Some(weapon) = state.weapon_of(id, slot) else { continue };
                            let data = gear_db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: weapon }).ok().flatten();
                            let sounds = data.map(|d| mao_formats::item::class_sets(&d, 0x14)).unwrap_or_default();
                            let (target, hit, amount) = state.attacks.get(&(id, slot)).copied().unwrap_or((0, 0, 0));
                            let target = (if target == own { Some(position) } else { state.position_of(target) }).map(|position| {
                                let info = state.infos.get(&target);
                                let max_health = if target == own { state.stats.get(&1).copied().unwrap_or(0) } else { state.health.get(&target).map_or(0, |h| h.1) };
                                let set = info.and_then(|i| i.template).and_then(|t| c.set_of(t)).map(|t| (t, sound_rand.next() as u32));
                                crate::sounds::Target { position, max_health, set, breed: info.map_or(0, |i| i.breed), sex: info.map_or(0, |i| i.sex) }
                            });
                            let npc = id != own && !state.infos.get(&id).is_some_and(|i| i.is_player());
                            let attack = crate::sounds::Attack { event, position: at, npc, weapon: &sounds, energy: state.weapon_energy.get(&(id, slot as u8)).copied(), hit, amount, target };
                            for p in crate::sounds::attack(&attack, || sound_rand.next() as u32) {
                                crate::audio::with(|m| m.play_game_sound(p.id, p.position, 0.0, p.volume, p.material, p.defer, 100, p.mode, 0, [0.0; 3]));
                            }
                            continue;
                        }
                        let (liquid, kind) = crate::sounds::liquid_depth(crate::liquid::current().as_deref(), at);
                        let material = crate::sounds::liquid_material(crate::sounds::ground_material(terrain.as_deref(), &tile_materials, at), liquid, kind);
                        let note = crate::sounds::Note { event, position: at, set, falling: id == own && airborne, material, liquid };
                        if let Some(p) = crate::sounds::note(&generic, &note) {
                            crate::audio::with(|m| m.play_game_sound(p.id, at, 0.0, p.volume, p.material, 0.0, 100, 1, 0, [0.0; 3]));
                            // (FUN_100452d3: a landing played without a set starts the environment's effect 0x2c there)
                            if event == 0x85 && set.is_none() {
                                state.landings.push(at);
                            }
                        }
                    }
                }
                casters.extend(own_casters);
                let deforms_landed: std::cell::RefCell<Vec<(u32, mao_formats::gfx::Deform)>> = std::cell::RefCell::new(Vec::new());
                let own_velocity = if walk == 0 && keys.strafe == 0 && !airborne {
                    [0.0; 3]
                } else {
                    let dt = last_own.1.elapsed().as_secs_f32().max(0.001);
                    [(position[0] - last_own.0[0]) / dt, 0.0, (position[2] - last_own.0[2]) / dt]
                };
                last_own = (position, Instant::now());
                if profile && built.elapsed() > Duration::from_millis(4) {
                    eprintln!("[profile] overlay_model {:?} ({} parts, {} instances)", built.elapsed(), model.parts.len(), model.instances.len());
                }
                if state.stats_changed && std::env::var_os("MAO_STATS").is_some() {
                    let shown = crate::derived::shown(&state.stats, &state.item_mods);
                    let pick = |ids: &[u32]| ids.iter().map(|i| format!("{i:#x}={}", shown.get(i).map_or("-".to_string(), |v| v.to_string()))).collect::<Vec<_>>().join(" ");
                    eprintln!("stored: {:?} mods: {:?} perks {} nanos {:?} org {}", [1u32, 0x1b, 0xd6, 0xdd, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15].map(|i| (i, state.stats.get(&i).copied())), [1u32, 0xdd, 0x10, 0x12].map(|i| (i, state.item_mods.get(&i).copied())), state.perks.len(), state.nanos, state.org_benefits.len());
                    for (title, group) in crate::skills::GROUPS {
                        eprintln!("skills {title}: {}", crate::skills::rows(group, &shown, "").iter().map(|(_, n, v)| format!("{n}={}", v.map_or("-".to_string(), |v| v.to_string()))).collect::<Vec<_>>().join(", "));
                    }
                    {
                        let parts = crate::derived::breakdown(&state.stats, &state.mods_by_source[0], &state.mods_by_source[1], &state.mods_by_source[2], &state.mods_by_source[3]);
                        for stat in [16u32, 18, 100, 122, 152] {
                            eprintln!("parts {stat}: {:?}", parts.get(&stat));
                        }
                    }
                    for e in state.worn.iter().filter(|e| !e.is_bag() && e.slot < 64) {
                        for action in crate::derived::actions(&gear_db, e).iter().filter(|a| matches!(a.kind, 3 | 6 | 8)) {
                            let lines = crate::gear::requirement_lines(&action.criteria, &shown);
                            let unmet: Vec<_> = lines.iter().filter(|l| l.1 == Some(false)).map(|l| l.0.clone()).collect();
                            let unknown = lines.iter().filter(|l| l.1.is_none()).count();
                            eprintln!("requirements slot {} item {} action {}: {} conditions, unmet {:?}, unjudged {}", e.slot, e.low_id, action.kind, lines.len(), unmet, unknown);
                        }
                    }
                    eprintln!("running nanos (id, ms left): {:?}; NCU {:?} of {:?}", state.nano_times, shown.get(&180), shown.get(&181));
                    eprintln!("worn: {:?}", state.worn.iter().map(|e| (e.slot, e.low_id, e.high_id, e.quality)).collect::<Vec<_>>());
                    eprintln!("stats: {} stored, {} modifiers; {}", state.stats.len(), state.item_mods.len(), pick(&[0x1b, 0x01, 0xd6, 0xdd, 0x36, 0x34, 0x39, 0x15e, 0x28, 0xb2, 0xa9, 0x5b]));
                }
                // (the server's `CastNanoSpell` has no line in the original: its handler, FUN_1007284f → FUN_10051c98, only
                // starts the cast)
                state.casts.clear();
                state.prune_nanos();
                state.tick_lines();
                state.refresh_mods(&gear_db);
                if let Some((who, items)) = state.inspected.take() {
                    let name = state.names.get(&who).map_or_else(|| "The target".to_string(), |n| n.0.clone());
                    state.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, format!("{name} wears {} items:", items.len())));
                    for it in items {
                        let template = [it.low_id, it.high_id].into_iter().find_map(|id| gear_db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: id }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)));
                        let item = template.map_or_else(|| format!("item #{}", it.low_id), |t| t.name);
                        let slot = crate::gear::slot_name(it.slot).unwrap_or_else(|| format!("slot {}", it.slot));
                        state.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, format!("  {slot}: {item} (QL {})", it.ql)));
                    }
                }
                // What the character looks like is written down for the character screen.
                if let (Some(info), Some(name)) = (state.infos.get(&own), state.names.get(&own).map(|n| n.0.clone()).filter(|n| !n.is_empty())) {
                    let look = crate::look::Look { name, breed: info.breed, sex: info.sex, clothes: info.clothes.clone(), textures: info.textures.clone(), gear: state.equipment.get(&own).cloned().unwrap_or_default() };
                    if saved_look.as_ref() != Some(&look) {
                        crate::look::save(&look);
                        saved_look = Some(look);
                    }
                }
                let mut chat_lines_now: Vec<crate::chatwindow::Line> = std::mem::take(&mut *CHAT_LINES.lock().unwrap());
                chat_lines_now.extend(std::mem::take(&mut state.game_lines));
                let friends_now = {
                    let buddies = BUDDIES.lock().unwrap();
                    (buddies.0 != friends_seen).then(|| {
                        friends_seen = buddies.0;
                        buddies.1.iter().map(|(&id, (name, online))| crate::friends::Friend { id, name: name.clone(), online: *online }).collect::<Vec<_>>()
                    })
                };
                let update = crate::viewer::Update {
                    messages_only: false,
                    scene: pending_scene.take(),
                    model,
                    own_model,
                    own_velocity,
                    casters,
                    map: pending_map.take(),
                    stats: std::mem::take(&mut state.stats_changed).then(|| {
                        let parts = crate::derived::breakdown(&state.stats, &state.mods_by_source[0], &state.mods_by_source[1], &state.mods_by_source[2], &state.mods_by_source[3]);
                        (state.names.get(&own).map(|n| n.0.clone()).unwrap_or_default(), crate::derived::totals(&parts), parts)
                    }),
                    map_dots: {
                        let friendly_name = |name: &str| ["Vanguard", "Watcher", "Commander", "Guard", "Merchant", "Trader", "Recruiter", "Helper"].iter().any(|w| name.contains(w));
                        state
                            .entities
                            .iter()
                            .filter(|(id, _)| **id != own)
                            .filter_map(|(id, (p, _))| {
                                let info = state.infos.get(id)?;
                                let kind = if target == Some(*id) {
                                    crate::automap::DotKind::Target
                                } else if info.is_player() && *id < DYNAMIC_NPC {
                                    crate::automap::DotKind::Player
                                } else if friendly_name(&info.name) {
                                    crate::automap::DotKind::Friendly
                                } else {
                                    crate::automap::DotKind::Hostile
                                };
                                Some(([p[0], p[2]], kind))
                            })
                            .collect()
                    },
                    special_cooldowns: std::mem::take(&mut state.special_events),
                    auras: Some({
                        // (the character's own programs from its list, the others' from the buffs seen lately)
                        state.active_buffs.retain(|_, nanos| {
                            nanos.retain(|_, at| at.elapsed() < Duration::from_secs(30));
                            !nanos.is_empty()
                        });
                        let mut aura_of = |nano: u32| -> Option<mao_formats::gfx::Visual> {
                            aura_cache
                                .entry(nano)
                                .or_insert_with(|| {
                                    let data = gear_db.get(mao_formats::resource::Id { kind: 0xfde85, instance: nano }).ok().flatten()?;
                                    // (the running effect of a program is its `EffectType` stat)
                                    let effect = mao_formats::item::Item::parse(&data)?.stat(413)?;
                                    gfx.as_ref()?.visual(effect as i32)
                                })
                                .clone()
                        };
                        let mut list: Vec<(u32, bool, [f32; 3], f32, Option<(i32, i32, i32)>, Vec<(u32, mao_formats::gfx::Visual)>)> = Vec::new();
                        // (the body the ring is fitted to: breed, sex and the fatness stat 47)
                        let body = |id: u32| -> Option<(i32, i32, i32)> {
                            let info = state.infos.get(&id)?;
                            let fatness = if id == own { state.stats.get(&47).copied().unwrap_or(1) as i32 } else { 1 };
                            Some((info.breed as i32, info.sex as i32, fatness))
                        };
                        let mut add = |id: u32, own_character: bool, at: [f32; 3], nanos: Vec<u32>| {
                            let auras: Vec<(u32, mao_formats::gfx::Visual)> = nanos.into_iter().filter_map(|n| aura_of(n).map(|v| (n, v))).take(6).collect();
                            if !auras.is_empty() {
                                let facing = state.rotations.get(&id).map(|q| 2.0 * q[1].atan2(q[3])).unwrap_or(0.0);
                                list.push((id, own_character, [at[0], at[2], at[1]], facing, body(id), auras));
                            }
                        };
                        add(own, true, position, state.nanos.clone());
                        for (id, nanos) in &state.active_buffs {
                            if let Some((p, _)) = state.entities.get(id).filter(|_| *id != own) {
                                add(*id, false, *p, nanos.keys().copied().collect());
                            }
                        }
                        list
                    }),
                    bursts: Some(
                        std::mem::take(&mut state.cast_bursts)
                            .into_iter()
                            .filter_map(|(caster, target, nano)| {
                                let at = if caster == own { position } else { state.entities.get(&caster)?.0 };
                                let cast = burst_cache
                                    .entry(nano)
                                    .or_insert_with(|| {
                                        let data = gear_db.get(mao_formats::resource::Id { kind: 0xfde85, instance: nano }).ok().flatten()?;
                                        // (the cast effect of a program is its `CastEffectType` stat, a `_GfxControlSpell1_t` template)
                                        let effect = mao_formats::item::Item::parse(&data)?.stat(428)?;
                                        gfx.as_ref()?.cast(effect as i32)
                                    })
                                    .clone()?;
                                let heading = state.rotations.get(&caster).map(|q| 2.0 * q[1].atan2(q[3])).unwrap_or(0.0);
                                // (the effect lasts the cast: the casting animation the caster plays, at itself or at someone)
                                let length = if caster == target { crate::effects::CAST_AT_SELF } else { crate::effects::CAST_AT_OTHER };
                                Some(crate::effects::CastShow { feet: [at[0], at[2], at[1]], heading, cast, length })
                            })
                            .collect(),
                    ),
                    shots: Some(
                        std::mem::take(&mut state.shots_pending)
                            .into_iter()
                            .filter_map(|(caster, target, nano)| {
                                let place = |id: u32| if id == own { Some(position) } else { state.entities.get(&id).map(|e| e.0) };
                                let (from, to) = (place(caster)?, place(target).unwrap_or(place(caster)?));
                                let (tracer, hit) = shot_cache
                                    .entry(nano)
                                    .or_insert_with(|| {
                                        let data = gear_db.get(mao_formats::resource::Id { kind: 0xfde85, instance: nano }).ok().flatten();
                                        let template = data.as_deref().and_then(mao_formats::item::Item::parse);
                                        let effect = |stat: u32| template.as_ref().and_then(|t| t.stat(stat)).and_then(|id| gfx.as_ref()?.visual(id as i32));
                                        // (a program's tracer is its `TracerEffectType`, what it does on landing its `HitEffectType`)
                                        (effect(419), effect(361))
                                    })
                                    .clone();
                                if tracer.is_none() && hit.is_none() {
                                    return None;
                                }
                                // (a hit that bends the target's mesh is noted for its character)
                                // (a bundle's deformer too)
                                let bent = match &hit {
                                    Some(mao_formats::gfx::Visual::Deform(d)) => Some(d),
                                    Some(mao_formats::gfx::Visual::Meta(children)) => children.iter().find_map(|c| if let mao_formats::gfx::Visual::Deform(d) = c { Some(d) } else { None }),
                                    _ => None,
                                };
                                if let Some(d) = bent {
                                    deforms_landed.borrow_mut().push((target, d.clone()));
                                }
                                // (Z up: from the caster's hands to the target's chest)
                                Some(crate::effects::Shot { from: [from[0], from[2], from[1] + 1.1], to: [to[0], to[2], to[1] + 1.2], tracer, hit })
                            })
                            .collect(),
                    ),
                    strikes: std::mem::take(&mut state.strikes),
                    jolts: std::mem::take(&mut state.jolts),
                    landings: std::mem::take(&mut state.landings),
                    flares: std::mem::take(&mut state.placed_effects)
                        .into_iter()
                        .filter_map(|(id, at)| Some(([at[0], at[2], at[1]], mao_formats::gfx::Flare::read(gfx.as_ref()?.get(id)?)?)))
                        .collect(),
                    glows: std::mem::take(&mut state.landed)
                        .into_iter()
                        .filter_map(|(id, nano)| {
                            let at = if id == own { position } else { state.entities.get(&id)?.0 };
                            let data = gear_db.get(mao_formats::resource::Id { kind: 0xfde85, instance: nano }).ok().flatten()?;
                            let t = mao_formats::item::Item::parse(&data)?;
                            Some(([at[0], at[2], at[1] + 1.2], crate::ui::glow_color(t.stat(460), t.stat(461), t.stat(462), t.stat(405).unwrap_or(0))))
                        })
                        .collect(),
                    floaters: std::mem::take(&mut state.damage)
                        .into_iter()
                        .filter(|(_, amount, _)| *amount != 0)
                        .filter_map(|(id, amount, _)| {
                            // over the one hit (your own character stands at `position`)
                            let at = if id == own { position } else { state.entities.get(&id)?.0 };
                            let kind = if amount > 0 { 2 } else if id == own { 1 } else { 0 };
                            Some(([at[0], at[2], at[1] + 2.0], amount.abs().to_string(), kind))
                        })
                        .collect(),
                    nameplates: {
                        let friendly_name = |name: &str| ["Vanguard", "Watcher", "Commander", "Guard", "Merchant", "Trader", "Recruiter", "Helper"].iter().any(|w| name.contains(w));
                        let mut plates: Vec<(f32, crate::automap::DotKind, [f32; 3], String)> = state
                            .entities
                            .iter()
                            .filter(|(id, _)| **id != own)
                            .filter_map(|(id, (p, _))| {
                                let info = state.infos.get(id)?;
                                let name = state.names.get(id).map(|n| n.0.clone()).filter(|n| !n.is_empty()).or_else(|| Some(info.name.clone()).filter(|n| !n.is_empty()))?;
                                let distance = ((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt();
                                if distance > 40.0 {
                                    return None;
                                }
                                let kind = if target == Some(*id) {
                                    crate::automap::DotKind::Target
                                } else if info.is_player() && *id < DYNAMIC_NPC {
                                    crate::automap::DotKind::Player
                                } else if friendly_name(&info.name) {
                                    crate::automap::DotKind::Friendly
                                } else {
                                    crate::automap::DotKind::Hostile
                                };
                                // (a character whose level is known carries it)
                                let name = state.levels.get(id).map_or(name.clone(), |level| format!("{name} ({level})"));
                                let name = state.orgs.get(id).map_or(name.clone(), |org| format!("{name} <{org}>"));
                                Some((distance, kind, [p[0], p[2], p[1] + 2.3], name))
                            })
                            .collect();
                        // the nearest forty
                        plates.sort_by(|a, b| a.0.total_cmp(&b.0));
                        plates.into_iter().take(40).map(|(_, kind, at, name)| (kind, at, name)).collect()
                    },
                    ground: terrain.clone().map(|t| (t, offset)),
                    own: Some(position),
                    environment: Some(crate::environment::compute(&areas, base_environment.as_ref(), position, state.game_clock.day(), state.game_clock.seconds(), state.game_clock.day_intensity())),
                    daylight: Some(state.game_clock.day_intensity()),
                    location: pending_location.take(),
                    labels: pending_labels.take(),
                    pickables: Some(state.objects.iter().map(|(id, p)| (*id, [p[0], p[2], p[1] + 1.2])).chain(state.entities.iter().filter(|(id, _)| **id != own).map(|(id, (p, _))| (*id, [p[0], p[2], p[1] + 1.0]))).collect()),
                    messages: {
                        for line in &chat_lines_now {
                            pending_messages.push(line.text.clone());
                        }
                        std::mem::take(&mut pending_messages)
                    },
                    chat_lines: chat_lines_now,
                    friends: friends_now,
                    loot: std::mem::take(&mut state.loot_changed).then(|| {
                        state.loot.as_ref().map(|l| crate::loot::LootView { title: crate::loot::title(l.identity.0).to_string(), identity: l.identity, handle: l.handle, items: crate::gear::from_entries(&gear_db, &gear_textures, &l.entries, state.stats.get(&0x36).copied().unwrap_or(1)) })
                    }),
                    perk_list: std::mem::take(&mut state.perks_changed).then(|| crate::perks::describe(&gear_db, &gear_textures, &state.perks, state.stats.get(&0x36).copied().unwrap_or(1))),
                    programs: std::mem::take(&mut state.programs_changed).then(|| crate::nanoprograms::describe(&gear_db, &gear_textures, &state.program_ids, state.stats.get(&0x36).copied().unwrap_or(1))),
                    gear: pending_gear.take(),
                    nanos: std::mem::take(&mut state.nano_list_changed).then(|| crate::ncu::describe(&gear_db, &gear_textures, &state.nano_times, state.stats.get(&0x36).copied().unwrap_or(1), state.nano_times_at.unwrap_or_else(Instant::now))),
                    missions: pending_missions.take(),
                    mission_status: pending_mission_status.take(),
                    mail_count: pending_mail_count.take(),
                    mail_entries: pending_mail_entries.take(),
                    mail_opened: pending_mail_opened.take(),
                    shop: pending_shop.take(),
                    shop_status: pending_shop_status.take(),
                    mail_flags: pending_mail_flags.take(),
                    mail_status: pending_mail_status.take(),
                    destinations: pending_destinations.take(),
                    heading: Some(heading),
                    target_bar: target.and_then(|t| {
                        let name = state.names.get(&t).map(|n| n.0.clone()).filter(|n| !n.is_empty()).or_else(|| state.infos.get(&t).map(|i| i.name.clone()))?;
                        let p = state.position_of(t)?;
                        let distance = ((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt();
                        let hp = state.health.get(&t).map(|&(hp, max)| (hp as i64, max as i64)).or_else(|| state.last_hp.get(&t).map(|&hp| (hp as i64, 0)));
                        let kind = match state.infos.get(&t) {
                            Some(info) if info.is_player() && t < DYNAMIC_NPC => crate::automap::DotKind::Player,
                            Some(info) if ["Vanguard", "Watcher", "Commander", "Guard", "Merchant", "Trader", "Recruiter", "Helper"].iter().any(|w| info.name.contains(w)) => crate::automap::DotKind::Friendly,
                            Some(_) => crate::automap::DotKind::Hostile,
                            None => crate::automap::DotKind::Friendly,
                        };
                        Some(crate::ui::TargetBar { name, level: state.levels.get(&t).map(|&l| l as i64), hp, distance, kind })
                    }),
                    target: target.and_then(|t| Some(format!("{}{}  ({:.0} m)", state.names.get(&t)?.0, state.health.get(&t).map(|(hp, max)| format!("  HP {hp}/{max}")).or_else(|| state.last_hp.get(&t).map(|hp| format!("  HP {hp}"))).unwrap_or_default(), ((state.position_of(t)?[0] - position[0]).powi(2) + (state.position_of(t)?[2] - position[2]).powi(2)).sqrt()))),
                };
                for (target, d) in deforms_landed.into_inner() {
                    state.deforms.insert(target, (d, Instant::now()));
                }
                if tx.send(update).is_err() {
                    break;
                }
            }
        }
    });
    crate::viewer::window(scene, textures, Some(crate::viewer::Live { overlays: rx, input: input_tx, mail: mail_tx, shop: shop_tx, focus, distance: 6.0 }))
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
        // `net <game dir> relogin <name> [chat]`: log in, enter the world, leave and log in again (what a zoning needs), to see what the server refuses.
        [game, word, who, rest @ ..] if word == "relogin" => {
            let (mut first, list) = login(game)?;
            let character = list.iter().find(|c| c.name.eq_ignore_ascii_case(who)).ok_or("no such character")?;
            let ticket = session::login_character(&mut first, character.id).map_err(|e| e.to_string())?;
            let world = session::enter_world(&ticket).map_err(|e| e.to_string())?;
            let (user, password) = credentials()?;
            let mut chat = None;
            if rest.iter().any(|a| a == "chat") {
                let mut c = mao_net::chat::Chat::connect(mao_net::chat::SERVER).map_err(|e| e.to_string())?;
                c.login(character.id, &user, &password).map_err(|e| e.to_string())?;
                eprintln!("chat session open");
                chat = Some(c);
            }
            std::thread::sleep(Duration::from_secs(4));
            if !rest.iter().any(|a| a == "keep-login") {
                first.close();
                drop(first);
            }
            if !rest.iter().any(|a| a == "keep-game") {
                world.close();
                drop(world);
            }
            for n in 0..4 {
                std::thread::sleep(Duration::from_secs(2));
                match login(game) {
                    Ok(_) => {
                        eprintln!("login {n}: accepted");
                        break;
                    }
                    Err(e) => eprintln!("login {n}: {e}"),
                }
            }
            drop(chat);
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
            // The login connection must go: while it is open the server refuses the login a zoning needs.
            chat.close();
            drop(chat);
            let mut state = World { own: character.id, ..World::default() };
            if rest.iter().any(|a| a == "--live") {
                return live(game, character.proxy[1], world, state, ticket.key);
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
            let mut connection = hop(game, target, character.id, &|t| eprintln!("{t}"))?;
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
            let textures = crate::Textures::new(db.clone());
            let area = load_area_with(game, &db, pf.parse().map_err(|_| "bad playfield")?, Some(&textures))?;
            eprintln!("loaded in {:.1}s: {} parts, {} booths", start.elapsed().as_secs_f32(), area.scene.parts.len(), area.booths.len());
            for b in &area.booths {
                eprintln!("booth at ({:.1}, {:.1}, {:.1})", b[0], b[1], b[2]);
            }
            let t = Instant::now();
            let props = crate::static_props(&db, pf.parse().unwrap_or(0));
            eprintln!("static props: {} in {:?}", props.len(), t.elapsed());
            let t = Instant::now();
            let _ = Characters::new(db.clone());
            eprintln!("Characters::new in {:?}", t.elapsed());
            Ok(())
        }
        // `net <game dir> chat [seconds]`: log in on the chat server as the first character and print what arrives.
        [game, word, rest @ ..] if word == "chat" => {
            let (conn, list) = login(game)?;
            let character = list.first().ok_or("no characters")?;
            conn.close();
            drop(conn);
            let (user, password) = credentials()?;
            let mut chat = mao_net::chat::Chat::connect(std::env::var("MAO_CHAT_SERVER").unwrap_or_else(|_| mao_net::chat::SERVER.to_string()).as_str()).map_err(|e| e.to_string())?;
            chat.login(character.id, &user, &password).map_err(|e| e.to_string())?;
            eprintln!("chat login ok for {}", character.name);
            chat.set_timeout(Duration::from_secs(2)).map_err(|e| e.to_string())?;
            let end = Instant::now() + Duration::from_secs(rest.first().and_then(|n| n.parse().ok()).unwrap_or(10));
            while Instant::now() < end {
                match chat.receive() {
                    Ok(p) => println!("{p:?}"),
                    Err(mao_net::chat::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
            Ok(())
        }
        // `net <game dir> chat-tell <from> <to> <text>`: log in on the chat server as <from>, look <to> up and send it a tell, and print what comes
        // back for a few seconds (a tell to oneself proves the packet layouts both ways).
        [game, word, from, to, text] if word == "chat-tell" => {
            let (conn, list) = login(game)?;
            let character = list.iter().find(|c| c.name.eq_ignore_ascii_case(from)).ok_or_else(|| format!("no character {from:?} on the account"))?;
            conn.close();
            drop(conn);
            let (user, password) = credentials()?;
            let mut chat = mao_net::chat::Chat::connect(std::env::var("MAO_CHAT_SERVER").unwrap_or_else(|_| mao_net::chat::SERVER.to_string()).as_str()).map_err(|e| e.to_string())?;
            chat.login(character.id, &user, &password).map_err(|e| e.to_string())?;
            chat.set_timeout(Duration::from_millis(500)).map_err(|e| e.to_string())?;
            chat.send(&mao_net::chat::lookup_packet(to)).map_err(|e| e.to_string())?;
            let end = Instant::now() + Duration::from_secs(8);
            let mut sent = false;
            while Instant::now() < end {
                match chat.receive() {
                    Ok(p) => {
                        println!("{p:?}");
                        if let mao_net::chat::Packet::Name { id, name } = &p {
                            if !sent && name.eq_ignore_ascii_case(to) {
                                chat.send(&mao_net::chat::tell_packet(*id, text)).map_err(|e| e.to_string())?;
                                println!("tell sent to {name} ({id})");
                                sent = true;
                            }
                        }
                    }
                    Err(mao_net::chat::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
            Ok(())
        }
        // `net <game dir> gear-preview <capture.log> <connection> <playfield> <out.png>`: the characters one connection of a capture
        // shows (with the clothes and meshes of their appearance updates) on a playfield's map.
        [game, word, log, wanted, pf, out] if word == "gear-preview" => {
            let pf: u32 = pf.parse().map_err(|_| "bad playfield")?;
            let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
            let mut reader = Reader::default();
            let mut world = World::default();
            for line in text.lines() {
                let f: Vec<&str> = line.split_whitespace().collect();
                let (Some(tag), Some(connection)) = (f.get(1), f.last()) else { continue };
                if connection != wanted {
                    continue;
                }
                if *tag == "OPEN" {
                    reader = Reader::default();
                }
                if *tag != "S>C" || f.len() < 5 {
                    continue;
                }
                let bytes: Vec<u8> = (0..f[3].len() / 2).filter_map(|i| u8::from_str_radix(&f[3][2 * i..2 * i + 2], 16).ok()).collect();
                if reader.push(&bytes).is_err() {
                    continue;
                }
                while let Ok(Some(m)) = reader.next() {
                    if m.kind == frame::N3 {
                        world.add(&m);
                    }
                }
            }
            eprintln!("{} entities, {} with equipment", world.entities.len(), world.equipment.len());
            show_world(game, pf, &world, Some(out))
        }
        [game, word, log] if word == "inventory" => inventory_from_capture(game, log),
        [word, log, kind, rest @ ..] if word == "msg-dump" => msg_dump(log, kind, rest.first().and_then(|n| n.parse().ok()).unwrap_or(3)),
        // `net <game dir> props-preview <capture.log> <playfield> <out.png>`: the machines a capture announces for that playfield, drawn on its map.
        [game, word, log, pf, out] if word == "props-preview" => {
            let pf: u32 = pf.parse().map_err(|_| "bad playfield")?;
            let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
            let mut readers: BTreeMap<String, Reader> = BTreeMap::new();
            let mut world = World::default();
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
                    let Some(msg) = Message::parse(&m).filter(|x| m.kind == frame::N3 && x.kind == 0x7f544905) else { continue };
                    let here = mao_net::schema::Schema::global().parse_n3(msg.kind, &msg.data).ok().and_then(|p| p.value.get("PlayfieldId").and_then(mao_net::schema::Value::as_int));
                    if here == Some(std::env::var("MAO_INSTANCE").ok().and_then(|v| v.parse().ok()).unwrap_or(pf as i64)) {
                        world.add(&m);
                    }
                }
            }
            show_world(game, pf, &world, Some(out))
        }
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
    let send = |connection: &mut Connection, kind: u8, heading: f32, position: [f32; 3], since: &mut Instant| -> Result<(), String> {
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
    let ground = |p: [f32; 3]| terrain.as_ref().map_or(0.0, |t| t.height_at(p[0], p[2], t.height_scale));
    let offset = position[1] - ground(position);
    let mut since = clock;
    let send = |connection: &mut Connection, kind: u8, position: [f32; 3], since: &mut Instant| -> Result<(), String> {
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
                    let mut next = hop(game, &target, own, &|t| eprintln!("{t}"))?;
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
/// Every server-to-client N3 message of one type in a capture log, as (connection, parsed text).
pub(crate) fn capture_n3(log: &str, kind: u32) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let mut readers: BTreeMap<String, Reader> = BTreeMap::new();
    let mut out = Vec::new();
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
            if kind == 0 {
                if let Some(msg) = Message::parse(&m).filter(|_| m.kind == frame::N3) {
                    let name = mao_net::schema::Schema::global().n3_name(msg.kind).unwrap_or("?").to_string();
                    out.push(format!("{} {connection} {:08x} {name} from {:x}:{:x} len {}{}", f[0], msg.kind, msg.identity.0, msg.identity.1, msg.data.len(), std::env::var("MAO_FIND").ok().and_then(|h| { let pat: Vec<u8> = (0..h.len() / 2).filter_map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).ok()).collect(); let n = msg.data.windows(pat.len()).filter(|w| *w == &pat[..]).count(); (n > 0).then(|| format!(" HAS {h} x{n}")) }).unwrap_or_default()));
                }
                continue;
            }
            if let Some(msg) = Message::parse(&m).filter(|x| m.kind == frame::N3 && x.kind == kind) {
                let parsed = mao_net::schema::Schema::global().parse_n3(kind, &msg.data);
                out.push(match parsed {
                    Ok(p) => format!("{} on {connection} from {:x}:{:x} raw {}:{}", f[0], msg.identity.0, msg.identity.1, msg.data.iter().map(|b| format!("{b:02x}")).collect::<String>(), p.value.pretty()),
                    Err(e) => format!("{} on {connection}: parse error {e}: {}", f[0], msg.data.iter().map(|b| format!("{b:02x}")).collect::<String>()),
                });
            }
        }
    }
    Ok(out)
}

/// The `nth` non-empty `InventoryUpdate` of a capture: the container's identity type, its handle and its items (the loot preview).
pub(crate) fn capture_loot(log: &str, nth: usize) -> Result<(u32, u32, Vec<inventory::Entry>), String> {
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let mut readers: BTreeMap<String, Reader> = BTreeMap::new();
    let mut seen = 0;
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
            let Some(msg) = Message::parse(&m).filter(|x| m.kind == frame::N3 && x.kind == INVENTORY_UPDATE) else { continue };
            let Ok(p) = mao_net::schema::Schema::global().parse_n3(msg.kind, &msg.data) else { continue };
            let entries = inventory_from_schema_items(p.value.get("Items"));
            let kind = p.value.get("InventoryIdentity").and_then(|v| v.get("Type")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
            if entries.is_empty() || kind == 0 {
                continue;
            }
            if seen == nth {
                return Ok((kind, p.value.get("Handle").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32, entries));
            }
            seen += 1;
        }
    }
    Err(format!("fewer than {} containers in the capture", nth + 1))
}

/// The last world-entry message of the character `instance` in a capture: its items.
pub(crate) fn capture_entries_of(log: &str, instance: u32) -> Result<Vec<inventory::Entry>, String> {
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let mut readers: BTreeMap<String, Reader> = BTreeMap::new();
    let mut found = None;
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
            if let Some(msg) = Message::parse(&m).filter(|x| m.kind == frame::N3 && x.kind == inventory::WORLD_ENTRY && x.identity.1 == instance) {
                found = Some(inventory::parse(&msg.data));
            }
        }
    }
    found.ok_or_else(|| "no world-entry message of that character in the capture".into())
}

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
    capture_message(log, 0x5c436609)
}

/// The first server N3 message of this kind in a capture, parsed by the schema.
pub(crate) fn capture_message(log: &str, kind: u32) -> Option<mao_net::schema::Value> {
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
            if let Some(msg) = Message::parse(&m).filter(|x| m.kind == frame::N3 && x.kind == kind) {
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
    let items = crate::gear::from_entries(&db, &crate::Textures::new(db.clone()), &capture_entries(log)?, 220);
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

#[cfg(test)]
mod door_tests {
    use super::*;

    #[test]
    fn nanos_join_and_leave_the_running_list() {
        let mut w = World::default();
        w.nano_duration(10, 5000);
        w.nano_duration(11, 300);
        assert_eq!(w.nanos, vec![10, 11]);
        // a second later the short one has run out, the long one has not
        w.nano_times_at = Some(Instant::now() - Duration::from_secs(1));
        w.prune_nanos();
        assert_eq!(w.nanos, vec![10]);
        assert_eq!(w.nano_times.len(), 1);
        // casting it again sets its time
        w.nano_duration(10, 9000);
        assert_eq!(w.nano_times, vec![(10, 9000)]);
    }

    #[test]
    fn an_attack_swings_the_weapon_the_server_said_is_held() {
        // (a captured WeaponItemFullUpdate: owner 0x630193db holds template 301709 in place 6; then that owner's AttackInfo)
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let mut w = World::default();
        let weapon = Message { kind: 0x3b1d2268, identity: (0xc74a, 0x1f6bdd5c), data: hex("000000000b0000c350630193db0000028f000f424f00000000010600001f8800000000000004210000001700049a8d000002bd00000001000002be00049a8d000002bf00049a8d0000019c000000010000001a0000001000000000") };
        w.add(&weapon.frame(1));
        assert_eq!(w.weapons.get(&(0x630193db, 6)), Some(&301709));
        let attack = Message { kind: 0x46002f16, identity: (0xc350, 0x630193db), data: hex("0000000008ffffffff000000060000c3500004bc80000000000000000300000000") };
        w.add(&attack.frame(1));
        let (slot, _, pick, class) = w.swings[&0x630193db];
        assert_eq!((slot, pick, class), (6, 0, 0xb));
        w.add(&attack.frame(1));
        assert_eq!(w.swings[&0x630193db].2, 1);
        // (a captured Attack: the player 0x4bc80 attacks 0x7b017c2a; its StopFight ends it)
        w.add(&Message { kind: 0x28494070, identity: (0xc350, 0x4bc80), data: hex("000000c3507b017c2a00") }.frame(1));
        assert_eq!(w.fighting.get(&0x4bc80), Some(&0x7b017c2a));
        w.add(&Message { kind: 0x4a41203e, identity: (0xc350, 0x4bc80), data: hex("0000000001") }.frame(1));
        assert!(!w.fighting.contains_key(&0x4bc80));
        // (a captured death: 0x7b017c2a dies kneeling, 500, and is despawned ten seconds on)
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, 0x7b017c2a), data: hex("000000006300000000000000000000000000000000000001f40000") }.frame(1));
        assert_eq!(w.dead.get(&0x7b017c2a).map(|d| d.0), Some(500));
        w.add(&Message { kind: 0x36510078, identity: (0xc350, 0x7b017c2a), data: hex("00") }.frame(1));
        assert!(!w.dead.contains_key(&0x7b017c2a));
    }

    #[test]
    fn a_fighter_turns_to_its_target_as_charturn_does() {
        let mut f = Facing::default();
        // (the target straight to the side: a quarter turn; nothing for 0.67 s, then a turn at 7.2 rad/s)
        let (at, target) = ([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
        let mut h = 0.0;
        for _ in 0..6 {
            h = f.step(h, at, target, false, 0.1);
        }
        assert_eq!((h, f.turn), (0.0, None));
        h = f.step(h, at, target, false, 0.1);
        assert!(f.turn.is_some_and(|(left, dir)| (left - std::f32::consts::FRAC_PI_2).abs() < 1e-5 && dir == 1.0));
        h = f.step(h, at, target, false, 0.1);
        assert!((h - 0.72).abs() < 1e-5);
        h = f.step(h, at, target, false, 0.1);
        h = f.step(h, at, target, false, 0.1);
        assert!((h - std::f32::consts::FRAC_PI_2).abs() < 1e-5 && f.turn.is_none());
        // (within 0.26 rad it stays; moving holds the look off)
        let mut g = Facing::default();
        assert_eq!(g.step(1.4, at, target, false, 0.7), 1.4);
        assert_eq!(g.turn, None);
        let mut m = Facing::default();
        assert_eq!(m.step(0.0, at, target, true, 0.7), 0.0);
        assert_eq!((m.timer, m.turn), (0.0, None));
    }

    #[test]
    fn fight_lines_follow_the_originals_feedback() {
        // (captured, the client 0x4bc80 against 0x7b017c26 and 0x7b017c5b)
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let mut w = World { own: 0x4bc80, ..World::default() };
        w.names.insert(0x7b017c26, ("Rollerrat".into(), false));
        w.names.insert(0x7b017c5b, ("Tower Shark".into(), false));
        let lines = |w: &mut World| w.game_lines.drain(..).map(|l| l.text).collect::<Vec<String>>();
        // an AttackInfo from the client: 1913 at 0x7b017c26 (no weapon known: projectile)
        w.add(&Message { kind: 0x46002f16, identity: (0xc350, 0x4bc80), data: hex("0000000779ffffffff000000080000c3507b017c26000000040000000300000000") }.frame(1));
        assert_eq!(lines(&mut w), ["You hit Rollerrat for 1913 points of projectile damage."]);
        // a miss at the client
        w.add(&Message { kind: 0x5c654b28, identity: (0xc350, 0x4bc80), data: hex("01ffffffff000000060000c3507b017c260000c3500004bc8000000000") }.frame(1));
        assert_eq!(lines(&mut w), ["Rollerrat tried to hit you, but missed!"]);
        // a HealthDamage from 0x7b017c5b: it is the one hurt (its health 19908), the client the source
        w.add(&Message { kind: 0x3710256c, identity: (0xc350, 0x7b017c5b), data: hex("0000004dc4fffff4ef0000005b000000000000c3500004bc8000000000") }.frame(1));
        assert_eq!(lines(&mut w), ["You hit Tower Shark with nanobots for 2833 points of melee damage."]);
        assert_eq!(w.last_hp.get(&0x7b017c5b), Some(&19908));
        assert_eq!(w.damage.last().map(|d| d.0), Some(0x7b017c5b));
        // and the client healing itself
        w.add(&Message { kind: 0x3710256c, identity: (0xc350, 0x4bc80), data: hex("000000ce670000007400000000000000000000c3500004bc8000000000") }.frame(1));
        assert_eq!(lines(&mut w), ["You were healed for 116 points."]);
        assert_eq!(w.stats.get(&27), Some(&52839));
        // (another's health dropping in a Stat: 0x7aedad06 renamed to the rat, 2381 then 2300)
        w.add(&Message { kind: 0x2b333d6e, identity: (0xc350, 0x7b017c26), data: hex("00000000010000001b0000094d") }.frame(1));
        w.add(&Message { kind: 0x2b333d6e, identity: (0xc350, 0x7b017c26), data: hex("00000000010000001b000008fc") }.frame(1));
        assert_eq!(lines(&mut w), ["Rollerrat was attacked with nanobots for 81 points of unknown damage."]);
        // the client's damage shield striking 0x7b017c26 (renamed from the capture's 0x7b017c24), and a reflect of nothing
        w.add(&Message { kind: 0x25192476, identity: (0xc350, 0x7b017c26), data: hex("00000000ad0000c3500004bc8000000003") }.frame(1));
        assert_eq!(lines(&mut w), ["Your damage shield hit Rollerrat for 173 points of damage."]);
        w.add(&Message { kind: 0x1c3a4f77, identity: (0xc350, 0x7b017c53), data: hex("00000000000000c3500004bc8000000000") }.frame(1));
        assert!(lines(&mut w).is_empty());
    }

    #[test]
    fn fight_lines_name_who_and_what() {
    }

    #[test]
    fn doors_are_numbered_from_their_ids() {
        let mut world = World::default();
        // an instance's doors: consecutive ids, in the dynel table's order
        for id in 0x10c1a5a5..=0x10c1a5ab {
            world.door_ids.insert(id);
        }
        world.door_ids.insert(0x10c1b000);
        assert_eq!(world.door_number(0x10c1a5a5), Some(0));
        assert_eq!(world.door_number(0x10c1a5ab), Some(6));
        assert_eq!(world.door_number(0x10c1b000), Some(0));
        assert_eq!(world.door_number(0x10c1a5ac), None);
        // outdoors the number is in the id
        assert_eq!(world.door_number(0xc01c028f), Some(0x1c));
        world.door_slots.insert(2, (0xfe000007, [1.0, 2.0, 3.0]));
        world.types.insert(0x10c1a5a7, DOOR_TYPE);
        world.door_used(0x10c1a5a7);
        assert!(world.doors_open.get(&2).is_some_and(|until| *until > Instant::now()));
        world.door_used(0x10c1a5a8);
        assert_eq!(world.doors_open.len(), 1);
    }
}

#[cfg(test)]
mod movement_tests {
    use super::*;

    #[test]
    fn speeds_follow_the_originals_formula() {
        let stats = |rs: i64, hp: i64, max: i64| BTreeMap::from([(156u32, rs), (27u32, hp), (1u32, max)]);
        // RunSpeed 275 at full health: 6 forward, 3.7 backwards
        let (run, back) = movement_speeds(&stats(275, 100, 100));
        assert!((run - 6.0).abs() < 0.01 && (back - 3.7).abs() < 0.01);
        // capped at 13 and 9.1
        let (run, back) = movement_speeds(&stats(5000, 100, 100));
        assert!((run - 13.0).abs() < 0.01 && (back - 9.1).abs() < 0.01);
        // nearly dead (1% health): the fall-off, 1.67 (the floor is 1.5)
        let (run, _) = movement_speeds(&stats(275, 1, 100));
        assert!((run - 1.673).abs() < 0.01, "{run}");
        // the rate: a 1.15-calibrated run at 6 units a second over the 5.0 reference, held at 1.3 past 4 units a second
        assert!((movement_rate(1.15, 100.0, 6.0, 5.0) - 1.3).abs() < 0.001);
        assert!((movement_rate(0.9, 100.0, 1.5, 1.5) - 0.9).abs() < 0.001);
        assert!((movement_rate(1.0, 200.0, 5.0, 5.0) - 0.5).abs() < 0.001);
    }
}

