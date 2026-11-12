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
    let text = mao_formats::files::read_to_string(Path::new(game).join("version.id")).map_err(|e| format!("version.id: {e}"))?;
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

/// Where a creature is being walked to (see `World::paths`).
#[derive(Clone, Debug, PartialEq)]
enum Route {
    Waypoints(Vec<[f32; 3]>),
    Chase(u32),
}

/// How fast the walked creatures go (m/s) until their own paths show otherwise (wandering creatures, live: a steady 1.5),
/// and how near a chased one stops.
const PATH_SPEED: f32 = 1.5;
const CHASE_NEAR: f32 = 1.5;

/// The name a `CorpseFullUpdate` carries ("Remains of Leet"): a big-endian length, then that many bytes, the last a zero.
/// Found live (after the corpse's stats); read as the first such string of letters.
fn corpse_name(data: &[u8]) -> Option<String> {
    (0..data.len().saturating_sub(4)).find_map(|i| {
        let length = u32::from_be_bytes(data[i..i + 4].try_into().ok()?) as usize;
        let text = data.get(i + 4..i + 4 + length)?;
        let (last, name) = text.split_last()?;
        (*last == 0 && name.len() >= 3 && name[0].is_ascii_uppercase() && name.iter().all(|b| b.is_ascii_graphic() || *b == b' ')).then(|| String::from_utf8_lossy(name).into_owned())
    })
}

/// A creature's speed from two paths in a row: how far along the first one the second starts, over the time between them
/// (none when the first was already walked to its end, or the second does not start on it).
fn path_speed(previous: (Instant, [f32; 3], [f32; 3]), start: [f32; 3], now: Instant) -> Option<f32> {
    let flat = |a: [f32; 3], b: [f32; 3]| ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    let dt = now.duration_since(previous.0).as_secs_f32();
    let (moved, length) = (flat(previous.1, start), flat(previous.1, previous.2));
    (dt > 0.4 && moved < length - 0.3 && moved > 0.2).then(|| (moved / dt).clamp(0.5, 15.0))
}

/// `MAO_TRACE=1`: print the server messages that matter for zoning and
/// the like (not the bulk of entity and position traffic), with the time.
fn trace(frame: &frame::Frame, clock: Instant) {
    const BULK: [u32; 12] = [0x54111123, 0x2b333d6e, 0x271b3a6b, 0x3b1d2268, 0x36510078, 0x4c7d403b, 0x3b11256f, 0x46002f16, 0x465a5d73, 0x260f3671, 0x195e496e, 0x1d3c0f1c];
    let t = clock.elapsed().as_secs_f32();
    if let Some(m) = Message::parse(frame) {
        // (MAO_TRACE_ALL=1: the bulk messages' kinds and identities too)
        if BULK.contains(&m.kind) && std::env::var_os("MAO_TRACE_ALL").is_some() {
            eprintln!("[{t:7.2}] server bulk {:08x} from {:x}:{:x} ({} bytes)", m.kind, m.identity.0, m.identity.1, m.data.len());
        }
        if !BULK.contains(&m.kind) {
            let head: String = m.data.iter().take(if m.kind == 0x52526858 { 64 } else if m.kind == 0x64582a07 { 400 } else { 28 }).map(|b| format!("{b:02x}")).collect();
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
    /// The character's perk actions as the server lists them (`CharacterAction` 0xb4 at entry: the action's item template, its
    /// number and its four-letter code), and the perk locks running (0xcf: a lock and its seconds; 0xce: the lock is free).
    perk_actions: Vec<(u32, u32, [u8; 4])>,
    /// The duel messages the server sent the character, not yet shown: what, the flag, the other character.
    duel_events: Vec<(u32, u32, u32)>,
    /// The weapon hits just told (`AttackInfo`) by victim: the damage and when, so that the health's own message does
    /// not tell them again.
    recent_hits: BTreeMap<u32, (i64, Instant)>,
    /// The stats of the special attacks the server listed for the character (`SpecialAttackWeapon`), once it has, and
    /// whether the list changed.
    specials: Option<Vec<u32>>,
    specials_changed: bool,
    /// The weapons worn the last time their special actions were worked out (place and template) with the actions'
    /// numbers, and the whole list of numbers the window was last told.
    weapon_specials: (Vec<(u32, u32, Option<i64>)>, Vec<u32>),
    /// The item moves asked for and shown at once, until the server answers: from, to, and whether an item at `to`
    /// went the other way. A refusal takes the move back (seen live 2026-11-10: the server answers a move it takes with
    /// `ContainerAddItem` naming the old place and the new, and one it refuses with `CharacterAction` 0x92 naming the
    /// old place).
    moves_asked: Vec<(u32, u32, bool)>,
    /// The server has said the character can backstab (`CharacterAction` 0xb5): a sneak attack weapon in a hand
    /// then gives Backstab too.
    backstab: bool,
    specials_told: Option<Vec<u32>>,
    perk_locks: BTreeMap<u32, (Instant, f32)>,
    perk_actions_changed: bool,
    perks_changed: bool,
    /// The character an `/inspect` waits on: its info packet is told in the chat window when it comes.
    inspecting: Option<u32>,
    /// The worn items an `Inspect` answer brought, to be named and told.
    inspected: Option<(u32, Vec<mao_net::n3::Inspected>)>,
    /// Nano casts that involve the character, not yet told: (caster, target, nano).
    casts: Vec<(u32, u32, u32)>,
    /// The level of the characters and creatures whose info packets came.
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
    /// The playfield is the Grid (`PlayfieldAnarchy_t::IsGrid`: its first area is named "Grid"): everyone is drawn as
    /// [`GRID_TEMPLATE`].
    grid: bool,
    /// Which way a character is sidestepping (-1 left, 1 right) while it is: only the player's own is known.
    strafing: BTreeMap<u32, i8>,
    /// Whether a character is moving backwards (its own: the back key; the others: the move against their facing).
    backing: BTreeMap<u32, bool>,
    /// `Buff` messages not yet looked at: (character, nano program). The server repeats a character's buffs every twenty seconds or so.
    buffs_seen: Vec<(u32, u32)>,
    /// The programs other characters run, with when their buff was last announced (the server repeats them about every twenty seconds).
    active_buffs: BTreeMap<u32, BTreeMap<u32, Instant>>,
    /// When each of those was first announced (character, nano): the buff message has no time, so a target's NCU
    /// window counts its time left from here.
    buff_first: BTreeMap<(u32, u32), Instant>,
    /// The nanos another character's full update listed, with the time each had left then.
    other_nano_times: BTreeMap<u32, (Vec<(u32, u32)>, Instant)>,
    /// The polymorph of each other character: the creature and when its program was last announced.
    morphed_until: BTreeMap<u32, (u32, Instant)>,
    /// The vehicle program (a Phasefront hoverboard or hoverbike) each character runs: its `ChangeMesh` spells, keyed on the
    /// body, give the vehicle body the character is drawn as (`Characters::vehicle`).
    riding: BTreeMap<u32, u32>,
    /// The vehicle program of each other character and when its buff was last announced.
    riding_until: BTreeMap<u32, (u32, Instant)>,
    /// Health and maximum from the info packets (the server answers an info request with them).
    health: BTreeMap<u32, (i64, i64)>,
    /// The character's pets by their place in the pet window (`PetWindowModule_c`'s map: `AddPet` 0x1007f0f9 puts a new pet
    /// in the first free place from 1, `GetFreePetSlot` 0x1007ee0d), from the server's `AddPet` and `RemovePet`; a new
    /// playfield starts the list again, as the original's new client character does (its pets come again after it).
    pets: BTreeMap<u32, (u32, u32)>,
    /// The pets whose names flash red in the pet window, with how many times and since when: `PetToMaster` (`0d381f02`,
    /// `PetToMasterIIR_c`: read FUN_10076917, apply FUN_1007699d) to one's own character, command 1 with its count (the
    /// captures' 0x0a, with "Your pet is out of range") or command 2 with none, raises global signal +0x40 (FUN_10011fa1) with
    /// the pet and the count, which `PetView_c` (FUN_1007d273) takes as its flash counter (`pets::flash_value`).
    pet_flashes: BTreeMap<u32, (u32, Instant)>,
    /// The health each character was last known to have (from the damage events: the target's health after the hit).
    last_hp: BTreeMap<u32, i64>,
    /// The social action each character is doing: its name and when it began (from `SocialActionCmd`).
    socials: BTreeMap<u32, (&'static str, Instant)>,
    /// The characters sitting on the ground (their `SwitchToSit` movement, until a `LeaveSit` or another movement): drawn
    /// in the sitting-down animation (action 0xd5, `ground-start`), held.
    seated: std::collections::BTreeSet<u32>,
    /// The seated characters sleeping or lounging: their `WaitState` (stat 430), 0xf sleeping and 0x10 lounging, as the
    /// transitions `SwitchToSleepMode` (FUN_1006ec48) and `SwitchToLoungeMode` (FUN_1006edf2) set it. A `SocialActionCmd`
    /// of 0x44 or 0x45 (FUN_1007aec0) hands the movement controller 0x21 or 0x22; `CharacterAction` 0x57 (case 22) then
    /// asks `LeaveSleepMode` (0x29) or `LeaveLoungeMode` (0x2a) instead of `LeaveSit`, which go back to sitting on the
    /// ground (mode 8: FUN_1006ece8 and FUN_1006ee92 play the lie-down backwards into `idle-ground`).
    resting: BTreeMap<u32, u32>,
    /// The characters swimming (their `SwitchToSwimMode` movement, until a `LeaveSwimMode`; one's own also by the client's
    /// own test of the water's depth): drawn in the swim and idle-swim animations.
    swimming: std::collections::BTreeSet<u32>,
    /// The characters that died (`CharacterAction` 99, `Death`, from the one dying): the death's action (its `Parameter2`: 500
    /// `die-knees`, 503 `die-shot` ...) and when. Cleared by its `Despawn` or a new full update.
    dead: BTreeMap<u32, (u32, Instant)>,
    /// The corpse (its 0xc76a instance, from `CorpseFullUpdate`) left by each dead character: the one announced where the
    /// body lies. Using the body uses the corpse, as clicking a body does in the original (the corpse is what the server opens).
    corpses: BTreeMap<u32, u32>,
    /// The characters that died lately and where (kept past their `Despawn`, which the server sends just before the corpse).
    bodies: BTreeMap<u32, ([f32; 3], Instant)>,
    /// How the server moves the creatures (`FollowTarget`, `260f3671`, Gamecode's `FollowTargetIIR_c`, up to 30 places):
    /// along a path of waypoints (type 1) or after a target (type 2). The client walks them there between the server's
    /// own position messages; a `SetPos` (`195e496e`) puts one down where it says.
    paths: BTreeMap<u32, Route>,
    /// Whether one's own character is walking, running or sidestepping this moment (its animation follows it exactly).
    own_moving: bool,
    /// One's own movement mode is walking (the walk/run toggle; the original's mode 2, else 3).
    own_walking: bool,
    /// One's own character flies: a running program lets it (`mao_formats::item::grants_flight`).
    own_flying: bool,
    /// Each walked creature's last path (when it came, its start, its end) and its speed as the paths show it.
    path_seen: BTreeMap<u32, (Instant, [f32; 3], [f32; 3])>,
    path_speeds: BTreeMap<u32, f32>,
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
    /// The characters with the dynel flag 0x800 (`SimpleChar_t` +0x138): set by a full update's flag bit 21 (FUN_1007835c) and
    /// by `CharacterAction` 0xa7 with parameter 2 not zero, cleared by 0xa7 with 0 (case 57, 0x1005e919). Its readers are the
    /// weapon holder's (+0x1d4): without it a weapon equipped in a place other than 0 empties the place 0 shown in the hand
    /// (FUN_1006ac49 from 0x83's FUN_1006b2dd); with it the weapon list draws its place 100 there (FUN_1006ab10, FUN_1006b14c).
    weapon_flag: std::collections::BTreeSet<u32>,
    /// Their Energy (stat 26: the rounds left), by (owner, place): `WeaponItemFullUpdate`'s, then each attack's `AmmoCount`
    /// (FUN_1006ae3c sets it when the weapon has the stat).
    weapon_energy: BTreeMap<(u32, u8), i64>,
    /// What the last attack of each (attacker, place) put on the place's weapon (FUN_1006ae3c: the target, the hit type +0x2c
    /// and the damage +0x30), for the attack notes' sounds (FUN_1009b9f3).
    attacks: BTreeMap<(u32, u32), (u32, i32, i64)>,
    /// The characters that died since the last frame, for their death's sound (`CharDie_t`, FUN_1007b803).
    died: Vec<u32>,
    /// The character itself is dead (GUI.dll `FlowControlModule_t::DeadMessage`: `m_isAlive` false and the movement input
    /// mode cleared) until it is back (`AliveMessage`: the arrival in a playfield, or health again).
    own_dead: bool,
    /// When the line of the own character's death was told (a death's messages can each carry the reason).
    death_told: Option<Instant>,
    /// The health when it died: health above it is a revival.
    dead_health: i64,
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
    /// The effects the server's spell lists made on characters (`SpellListIIR_t`, `spelllist`), not yet started: (character,
    /// effect template, seconds when the spell sets them).
    spell_effects_made: Vec<(u32, i32, Option<f32>)>,
    /// The spell lists' effects running on each character, drawn as its running programs' are: effect template → (when it began,
    /// its seconds). One without seconds runs until the character goes, as the original's stays until the list ends it.
    spell_effects: BTreeMap<u32, BTreeMap<i32, (Instant, Option<f32>)>>,
    /// The spell lists' runs waiting on a delay or more hits (`spelllist::Queue`, each character's FUN_10002595 queue), and the
    /// moment the spell runner's clock starts (its seconds stand in for `GameTime_t` +0x28).
    spell_queue: crate::spelllist::Queue,
    spell_epoch: Option<Instant>,
    /// Each character's state flags (stat 224) as the spell lists' stuns change them (`spelllist::StateFlags`): one's own
    /// cannot move while neither bit 2 nor 4 is set.
    state_flags: BTreeMap<u32, crate::spelllist::StateFlags>,
    /// The lightning's strikes heard (where, seed) and the eye's jolts, for the viewer.
    strikes: Vec<([f32; 3], u32)>,
    jolts: Vec<f32>,
    /// The landings' places (Y up) for their effect 0x2c, for the viewer.
    landings: Vec<[f32; 3]>,
    /// The effect scripts' visuals this frame (the sky), for the viewer.
    sky_frame: Option<crate::fxs::visuals::Frame>,
    /// Each character's last attack (`AttackInfo`, `MissedAttackInfo`, `SpecialAttackInfo`): the weapon's place, when, a count
    /// picking the swing, and the animation class (0xb, or a special's).
    swings: BTreeMap<u32, (u32, Instant, u32, u32)>,
    /// The special each character has queued (`CharSecSpecAttack` from the server: FUN_10068cd9 pushes the special's stat on
    /// the attack part's queue at +0x8c, whose front FUN_10068aa3 gives the next `SpecialAttackInfo` its animation class).
    queued_specials: BTreeMap<u32, u32>,
    /// The delay (hundredths) of the last perk action the server queued (`QueuePerk`, 0x50), shown as the "Special" timer bar.
    queued_perk: Option<u32>,
    /// The character's timer list (Gamecode's +0x1e8: attacks, specials, nano casts, items, reloads) and its bars
    /// (`TimerSystemModule_t`), moved on each frame by [`World::tick_timers`].
    timers: crate::timers::TimerList,
    timer_bars: crate::timers::TimerBars,
    /// When the timers last moved on.
    timers_moved: Option<Instant>,
    /// The client's own nano cast under way (FUN_1005141d's cast, +0x18 its timer's place) and when it lands.
    own_cast: Option<(usize, Instant)>,
    /// The CRT's `rand()` for the attack timers' start (FUN_10068331).
    timer_rand: crate::audio::Rand,
    /// The characters the server put in the sneak mode (`CharacterAction` 0xa2: movement 0x1c), until another movement mode.
    sneaking: std::collections::BTreeSet<u32>,
    /// The specials the server timed (skill stat, seconds; 0 = ready again), not yet given to the hotbars.
    special_events: Vec<(u32, f32)>,
    /// Damage and healing events not yet shown as floating numbers: (target id, amount, stat).
    damage: Vec<(u32, i64, u32)>,
    /// Lines the server's game messages made (damage, healing, "You successfully perform ..."), for the chat window.
    game_lines: Vec<crate::chatwindow::Line>,
    /// The container (corpse, bag) the character opened, and whether that changed.
    loot: Option<Loot>,
    loot_changed: bool,
    /// One's own backpacks that are open (a bag of the inventory the character used), by the bag's identity, and
    /// whether any changed.
    bags: BTreeMap<(u32, u32), Loot>,
    /// The bags looked into for the item index only (`itemindex.rs`): known, but with no window.
    silent_bags: std::collections::BTreeSet<(u32, u32)>,
    bags_changed: bool,
    /// The last full position report (`0x16`) the server sent of one's own character (its echo of ours, or its own when it
    /// refuses a move: then it repeats the last place it took, every few seconds).
    own_report: Option<[f32; 3]>,
    /// An `N3Teleport` within the playfield of one's own character (a generated mission's move): the destination and rotation,
    /// for the play loop to put the character at at once.
    own_teleport: Option<([f32; 3], [f32; 4])>,
    /// The playfield stopped by an `N3Teleport` of one's own character to another place (`n3EngineClient_t::StartTeleport`,
    /// N3.dll 0x100076ad: `StopPlayfield` kills its dynels and the character stands still) until the playfield message: since when.
    playfield_stopped: Option<Instant>,
    /// A corpse whose contents just arrived and have not been auto-looted yet: its handle and the places to take.
    fresh_corpse: Option<(u32, Vec<u32>)>,
    worn: Vec<inventory::Entry>,
    mods_dirty: bool,
    /// The identity type each entity was first heard of with (what a use message names it by).
    types: BTreeMap<u32, u32>,
    /// The vending machines (booths: shops, and the mission booths' companions) with their positions.
    vendors: BTreeMap<u32, [f32; 3]>,
    /// The Notum Wars tower sites of each playfield heard of (`PlayfieldAllTowers`, then `PlayfieldTowerUpdateClient`;
    /// `mao_net::landcontrol`), the playfield the last list was for, and whether the map should be told.
    towers: BTreeMap<u32, Vec<mao_net::landcontrol::Tower>>,
    tower_playfield: u32,
    towers_changed: bool,
    /// The playfield's land-control map (`LandControlMap_t`, 0xf4248, loaded with its districts) and the character's
    /// district name and land control level range (`N3Msg_GetLCDistrictInfo`), with whether the map should be told.
    land_map: Option<std::sync::Arc<mao_formats::landcontrolmap::LandControlMap>>,
    land_map_changed: bool,
    lc_district: Option<(String, i32, i32)>,
    lc_district_changed: bool,
    /// The player cities' houses blob of each playfield (`PlayfieldAllCities`), not drawn (no city code is ported).
    cities: BTreeMap<u32, Vec<u8>>,
    /// The terminals the server announces (`SimpleItemFullUpdate` on a `0xc73d` with a place): in an instance (ICC's shops) the
    /// playfield's own terminals go by these ids, not by their dynel table's.
    terminals: BTreeMap<u32, [f32; 3]>,
    /// An instanced playfield's runs of ids for its built-in objects (kind, first table index, count, first id: the playfield
    /// message's [`Message::instance_runs`]); empty where the table's ids are the server's.
    instance_runs: Vec<(u32, u32, u32, u32)>,
    /// The item a `TemplateAction` into the overflow window (`0x6e`) named: it comes into the inventory with the
    /// `ContainerAddItem` from `(0x6e, 0)` that follows (a quest item, a reward; aosharp's `RegisterLastItem`).
    overflow_item: Option<inventory::Entry>,
    /// The doors of the playfield by number: the key of the door's model in `props` and its position (from the dynel table).
    door_slots: BTreeMap<u8, (u32, [f32; 3])>,
    /// The server's ids of the doors (from their status messages) and until when each door (by number) stays open.
    door_ids: std::collections::BTreeSet<u32>,
    /// The template (`StaticInstance`, stat 23) of each door the server announced (a generated mission's): its model.
    door_templates: BTreeMap<u32, u32>,
    doors_open: BTreeMap<u8, Instant>,
    /// The machines the server announces with a model (mesh instance, position, rotation): drawn like the other entities.
    props: BTreeMap<u32, (u32, [f32; 3], [f32; 4])>,
    /// The travel list of the playfield, from the server (until taken).
    destinations: Option<mao_net::destinations::DestinationList>,
    /// The usable objects the server announces (doors, mail terminals, booths: identity type
    /// 0xc76a, message `4f474e05`) with their positions; targeted with Ctrl+Tab and used with E.
    objects: BTreeMap<u32, [f32; 3]>,
    /// The playfield's built-in terminals, booths and signs (`crate::table_dynels`): picked with the mouse, used and
    /// described like the others.
    table: Vec<(u32, u32, [f32; 3], u32, String)>,
    /// The kind of mission each of the playfield's mission terminals gives, by its table number (`mission_terminals`).
    mission_kinds: std::collections::HashMap<u32, u8>,
    /// Pages for the info view (a target's description), until taken.
    info_urls: Vec<String>,
    /// Windows to open (by `Ui::set_window` name), until taken.
    open_windows: Vec<&'static str>,
    /// The game server's id: the source of its messages (`Client_t`'s +0xac, the `X-Anarchy-ServerID` of the web pages).
    server_id: u32,
    /// A web page to open (the market terminal's), until taken.
    browser: Option<crate::browser::Request>,
    /// The template (`StaticInstance`, stat 23) of each vending machine the server announced: its prices' stats.
    vendor_templates: BTreeMap<u32, u32>,
    /// Each vending machine's name (its `NameTemplate` item's, stat 446).
    vendor_names: BTreeMap<u32, String>,
    /// A machine's outer texture (`OverrideTexture`, stat 336), put on its mesh's outer materials.
    prop_textures: BTreeMap<u32, u32>,
    /// How a machine's or thing's model is animated: its `AnimPlay` (stat 501) and `AnimPos` (500) and when they were
    /// told (see [`prop_anim_time`]). One without an entry stands still in its first pose.
    prop_anim: BTreeMap<u32, (u32, u32, Instant)>,
    /// The statels' moving parts (`crate::AnimatedPart`), drawn each frame.
    animated_statels: Arc<Vec<crate::AnimatedPart>>,
    /// The doors that open as the character comes up to them (by number).
    approach_doors: std::collections::HashSet<u8>,
    /// The playfield's own door props (`static_props`' keys) and their numbers: a door the server announces takes the
    /// number's slot with its own id, and the table's prop still swings and goes with that door.
    door_props: BTreeMap<u32, u8>,
    /// Each door's last change: opening (true) or closing, and when (its animation runs from then).
    door_motion: BTreeMap<u8, (bool, Instant)>,
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

/// The last line the chat server told as somebody's spoken notice (its sender's name and the text, a line each) and when:
/// an NPC's answer to a use when it will not open its window (seen live: "busy" lines).
static NPC_SPOKE: std::sync::Mutex<Option<(String, Instant)>> = std::sync::Mutex::new(None);

/// A stat of an item template.
fn template_stat(template: u32, stat: u32) -> Option<i64> {
    let data = ITEM_DB.get()?.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: template }).ok().flatten()?;
    mao_formats::item::Item::parse(&data)?.stat(stat).map(|v| v as i64)
}

/// A stat of a nano program (type 0xfde85).
/// Whether a cast of a nano with this attack time (its stat 294, hundredths of a second) shows the casting stance.
/// The original stands a character in it only while its list of casts under way has one (FUN_1007c8af), and a cast
/// whose time comes to nothing is run at once and never waits in the list (FUN_1005141d: the time from the nano, less
/// the caster's skill; nothing for a nano the function at its +0x14 answers for). So the nanos with no attack time show
/// no casting: the ones the items cast (a recharger's regeneration), the auras' and the playfields' own (seen by the
/// user 2026-11-11, who named the rechargers and the keepers' auras). The caster's skill can bring a time down to
/// nothing too: [`cast_seconds`], used for one's own casts, whose skill is known.
fn takes_time_to_cast(attack_time: Option<i64>) -> bool {
    attack_time.is_some_and(|t| t > 0)
}

/// How long a cast takes (Gamecode FUN_1005141d, 0x100514c0 on): the nano's attack time (stat 294, hundredths of a
/// second) less what the caster's nano initiative takes off (stat 149: half of it up to 1200, a sixth of what is above
/// and 600) less the caster's aggression setting (stat 51, -100 defensive to 100 aggressive: a second either way), not
/// under nothing and not under the nano's own least (stat 523, where it has one: 1234567890 is none). Nothing: the cast
/// is run at once and the caster never takes its casting stance. (Not followed: the nano's flag 0x80000, which makes
/// the time nothing whatever the stats.)
fn cast_seconds(attack_time: i64, least: Option<i64>, initiative: i64, aggression: i64) -> f32 {
    let off = if initiative <= 1200 { initiative as f32 * 0.5 } else { (initiative - 1200) as f32 / 6.0 + 600.0 };
    let mut seconds = (attack_time as f32 / 100.0 - off / 100.0 - aggression as f32 / 100.0).max(0.0);
    if let Some(least) = least.filter(|l| *l != 1_234_567_890) {
        seconds = seconds.max(least as f32 / 100.0);
    }
    seconds
}

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
/// `SpecialAttackWeapon`: a character's special attacks.
const SPECIAL_ATTACK_WEAPON: u32 = 0x1d3c0f1c;
/// `TeamMember` and `TeamMemberInfo`: a team member and their health and nano.
const TEAM_MEMBER: u32 = 0x46312d2e;
const TEAM_MEMBER_INFO: u32 = 0x28784248;
/// `Skill`: skills raised with IP (both ways).
const SKILL: u32 = 0x3e205660;
/// `AddTemplate`: an item added to the inventory.
const ADD_TEMPLATE: u32 = 0x052e2f0c;
/// `TemplateAction`: an item template named to a place (into the overflow window: the next item to come in).
const TEMPLATE_ACTION: u32 = 0x35505644;

/// The items of a parsed `InventoryUpdate`'s `Items` list.
fn inventory_from_schema_items(list: Option<&mao_net::schema::Value>) -> Vec<inventory::Entry> {
    inventory_from_schema(list)
}

/// `DoorStatusUpdate` and the identity type of doors.
const DOOR_STATUS: u32 = 0x4c7d403b;
/// `Bank` (`BankMessage`): the bank's items.
const BANK_MESSAGE: u32 = 0x343c287f;
const DOOR_TYPE: u32 = 0xc748;
/// `DoorFullUpdate`: a door announced with its place.
const DOOR_FULL_UPDATE: u32 = 0x365a5071;
/// How long a door that was used stays open.
const DOOR_OPEN: Duration = Duration::from_secs(5);
/// How close the character comes to a door that opens on approach before it opens (its leaves' reach).
const APPROACH_REACH: f32 = 4.0;
/// A door the server opened, until it says closed.
const SERVER_OPEN: Duration = Duration::from_secs(24 * 3600);

/// Whether a `DoorStatusUpdate` says the door is open: its `DoorStatusUpdateIIR_t` (read by FUN_100a0330) after a byte: the
/// version (2), the locked flag, the open flag, stat 195 (`int`), a flag byte and a list (`000003f1` empty); none when the
/// version is another.
fn door_open(data: &[u8]) -> Option<bool> {
    (data.get(1..5)? == [0, 0, 0, 2]).then(|| data.get(6).map(|&b| b != 0)).flatten()
}
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

/// Every entry of a parsed world-entry message's `Perks` list with its last field: for a research rank, what is left of it
/// (0 once finished), the research window's personal research.
fn perk_entries(list: Option<&mao_net::schema::Value>) -> Vec<(u32, i64)> {
    let Some(mao_net::schema::Value::List(items)) = list else { return Vec::new() };
    items.iter().filter_map(|i| Some((i.get("SkillId")?.as_int()? as u32, i.get("Unknown3").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as i64))).collect()
}

/// The chat connection is opened once per run; the lines it receives wait here for the world loop, and what the chat window sends goes
/// out through the sender set when it opens.
static CHAT_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static CHAT_LINES: std::sync::Mutex<Vec<crate::chatwindow::Line>> = std::sync::Mutex::new(Vec::new());
static CHAT_OUT: std::sync::OnceLock<std::sync::Mutex<std::sync::mpsc::Sender<crate::chatwindow::Out>>> = std::sync::OnceLock::new();

/// The friends list as the chat server tells it: a version that steps with each change, and id -> (name, online).
static BUDDIES: std::sync::Mutex<(u64, BTreeMap<u32, (String, bool, bool)>)> = std::sync::Mutex::new((0, BTreeMap::new()));

/// One a tell goes to or comes from who is not on the list becomes a temporary buddy, the friends window's "Recent
/// Messages" (GUI.dll FUN_100a6487: the node of kind 2 and the buddy add of kind 0).
fn recent_message(chat: &mut mao_net::chat::Chat, id: u32, name: &str) -> Result<(), mao_net::chat::Error> {
    let mut buddies = BUDDIES.lock().unwrap();
    if !buddies.1.contains_key(&id) {
        chat.send(&mao_net::chat::buddy_add_packet(id, true))?;
        buddies.1.insert(id, (name.to_string(), false, true));
        buddies.0 += 1;
    }
    Ok(())
}


/// Sends what the chat window typed (a tell or a line for the organization channel).
/// Vicinity speech to send on the game connection: (kind: 0 say, 1 whisper, 2 shout; text; voice record).
static SPEECH: std::sync::Mutex<Vec<(u8, String, Vec<u8>)>> = std::sync::Mutex::new(Vec::new());

/// Commands for the server (`/stuck` and the like), sent by the game loop.
static SERVER_COMMANDS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// The universal effects a mesh carries: its connectors named `eff_universal_<id>` (DisplaySystem's
/// `GenericMeshObject_t`, `fxgmo.rs` `light_attractors`: each puts its number and its place on the environment's
/// attractor stack, which Gamecode's `_EffectHandler_t::RunFunction` 0x100d2423 takes off and makes with `CreateEffect2`).
/// The orbital strike is such a mesh: an unseen box whose connector names effect 71015.
fn universal_effects(db: &ResourceDatabase, index: &mao_formats::instances::Instances, mesh: &str) -> Vec<i32> {
    let Some(instance) = index.find(0xf6951, mesh).or_else(|| index.find(0xf6951, &mesh.to_lowercase())) else { return Vec::new() };
    let Some(object) = db.get(mao_formats::resource::Id { kind: 0xf6951, instance }).ok().flatten() else { return Vec::new() };
    let Ok(parsed) = mao_formats::mesh::parse(&object) else { return Vec::new() };
    let a = &parsed.archive;
    let mut out = Vec::new();
    for (i, o) in a.objects.iter().enumerate() {
        if a.class(i) != Some("RRefFrameConnector") {
            continue;
        }
        let name = o.field("1", "name").or_else(|| o.field("", "name")).and_then(|f| f.values.first()).map(|v| String::from_utf8_lossy(v).trim_end_matches('\0').to_string()).unwrap_or_default();
        // (`sscanf("%i")` of what follows the prefix)
        if let Some(id) = name.strip_prefix("eff_universal_").and_then(|rest| rest.trim().parse::<i32>().ok()) {
            out.push(id);
        }
    }
    out
}

/// `/fxscript` lines for the effect scripts' engine, which lives on the game loop's thread.
static FXS_COMMANDS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// A `/fxscript` line, run by the game loop before the scripts' next frame.
pub fn fxs_command(line: String) {
    FXS_COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(line);
}

/// What a `/fxscript` line did, for the chat window: the effect it started, or why nothing came of it.
fn fxs_report(line: &str, ran: &crate::fxs::Ran) -> Vec<String> {
    let name = line.split_whitespace().nth(1).unwrap_or("");
    let mut out = Vec::new();
    match (ran.made, ran.fxid) {
        (true, Some(fxid)) => out.push(format!("Script {name}: running (environment effect {fxid}).")),
        (true, None) => out.push(format!("Script {name}: running.")),
        (false, _) => out.push(format!("Script {name}: nothing was started.")),
    }
    // (the reader's own complaints, a few of them: a missing file, a line it could not read)
    out.extend(ran.errors.iter().take(4).map(|e| format!("  {e}")));
    if ran.errors.len() > 4 {
        out.push(format!("  ... and {} more", ran.errors.len() - 4));
    }
    out
}

/// The claims asked of the server (`n3::claims`; true: the check for what is waiting), sent by the game loop.
static CLAIM_REQUESTS: std::sync::Mutex<Vec<bool>> = std::sync::Mutex::new(Vec::new());

/// `/assist` asked for: the name given, if any.
static ASSISTS: std::sync::Mutex<Vec<Option<String>>> = std::sync::Mutex::new(Vec::new());

/// `/assist [name]` (GUI.dll FUN_100b81b4): the network thread fights whomever the target, or the character named,
/// is fighting.
pub fn assist(name: Option<String>) {
    ASSISTS.lock().unwrap_or_else(|e| e.into_inner()).push(name);
}

pub fn request_claims(check: bool) {
    CLAIM_REQUESTS.lock().unwrap_or_else(|e| e.into_inner()).push(check);
}

/// Gamecode's text commands waiting to be sent (`n3::text_command`): the action and its number.
static TEXT_COMMANDS: std::sync::Mutex<Vec<(u8, u32)>> = std::sync::Mutex::new(Vec::new());

/// One of Gamecode's text commands (`/played`, `/born`, `/team loot`, `/bank info`), sent by the game loop.
pub fn text_command(action: u8, value: u32) {
    TEXT_COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push((action, value));
}

/// The organization's commands waiting to be sent (`n3::org_command`): the number, whether the target goes with it,
/// the text.
static ORG_COMMANDS: std::sync::Mutex<Vec<(u8, bool, String)>> = std::sync::Mutex::new(Vec::new());

pub fn org_command(command: u8, targeted: bool, text: String) {
    ORG_COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push((command, targeted, text));
}

/// A command for the server (`n3::server_command`), sent by the game loop.
pub fn server_command(text: String) {
    SERVER_COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(text);
}

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

/// The spell runner's view of the world (`spelllist::Scene`): the client's own character's stats and perks, the characters in
/// the playfield, whom they fight. Others' stats are not kept (their below / above criteria pass, as in the original).
impl crate::requirements::Context for World {
    fn stat(&self, holder: crate::requirements::Holder, stat: u32) -> Option<i64> {
        (holder == (crate::requirements::CHARACTER, self.own)).then(|| self.stats.get(&stat).copied().unwrap_or(0))
    }
    fn is_own(&self, holder: crate::requirements::Holder) -> bool {
        holder == (crate::requirements::CHARACTER, self.own)
    }
    fn has_perk(&self, character: crate::requirements::Holder, perk: u32) -> Option<bool> {
        self.is_own(character).then(|| self.perks.contains(&perk))
    }
}

impl crate::spelllist::Scene for World {
    fn exists(&self, id: crate::requirements::Holder) -> bool {
        id.1 != 0 && (id.1 == self.own || self.entities.contains_key(&id.1))
    }
    fn is_npc(&self, id: crate::requirements::Holder) -> Option<bool> {
        if id.1 == self.own {
            return Some(false);
        }
        self.names.get(&id.1).map(|n| !n.1)
    }
    fn fight_target(&self, id: crate::requirements::Holder) -> Option<crate::requirements::Holder> {
        self.fighting.get(&id.1).filter(|&&t| t != 0).map(|&t| (crate::requirements::CHARACTER, t))
    }
    fn use_target(&self, _: crate::requirements::Holder) -> Option<crate::requirements::Holder> {
        // (the +0x1e4 object's target: not kept here)
        None
    }
}

impl World {
    /// Why an item cannot go to a wear place, as Gamecode's `N3Msg_IsItemPossibleToWear` (0x10026c57) decides before
    /// anything is sent: `None` when it can; `Some(None)` when it cannot and nothing is said (not a weapon, armour,
    /// implant or spirit by its class, stat 76; a place its placement mask, stat 298, does not have, FUN_10080659; no
    /// "wear" among its "can" flags, FUN_10081d5f); `Some(text)` when its requirements to wear it (action 6) are not
    /// met, with the feedback text's name. Not here: the original's locked equipment, the implants' own rule
    /// (FUN_10044dd8), the social armour's (FUN_10080aff) and the belt's places for decks (FUN_10046f26).
    fn wear_refusal(&self, entry: &inventory::Entry, to: u32, db: &ResourceDatabase) -> Option<Option<&'static str>> {
        let stat = |k: u32| template_stat(entry.low_id, k).unwrap_or(0) as u32;
        let slot = to % 16;
        if !matches!(stat(76), 1 | 2 | 3 | 5) || slot == 0 || stat(298) & (1 << slot) == 0 || stat(30) & 4 == 0 {
            return Some(None);
        }
        let own = (crate::requirements::CHARACTER, self.own);
        let unmet = crate::derived::actions(db, entry).iter().filter(|a| a.kind == 6).any(|a| crate::requirements::evaluate(&a.criteria, own, self) == Ok(false));
        unmet.then_some(Some("Feedback_UnableToWear"))
    }

    /// The numbers of the special actions the character has (`None` until the server has listed its special attacks):
    /// the server's list, then what each weapon in a hand gives (Gamecode FUN_1009e84b, `WeaponItem_t`'s part of being
    /// put on: its attacks by its "can" flags, stat 30, and Reload when its ammunition stat, 26, is not -1). The weapons
    /// are the items in the two hands, places 6 and 8: taking one off (FUN_1009d39a) keeps what the weapon in the other
    /// hand still gives. The ammunition stat is the one the server tells of the weapon in the hand (`weapon_energy`),
    /// else the template's, where few weapons have one (some melee weapons and shields say -1); a weapon with neither
    /// gives Reload, as a stat that is not there is not -1 in the original either.
    fn special_numbers(&mut self) -> Option<Vec<u32>> {
        let mut list = self.specials.clone()?;
        let worn: Vec<(u32, u32, Option<i64>)> = self.worn.iter().filter(|e| e.slot == 6 || e.slot == 8).map(|e| (e.slot, e.low_id, self.weapon_energy.get(&(self.own, e.slot as u8)).copied())).collect();
        if worn != self.weapon_specials.0 {
            let mut numbers = Vec::new();
            let mut reload = false;
            for &(slot, template, energy) in &worn {
                let stat = |k| template_stat(template, k).map(|v| v as u32);
                let energy = energy.or(stat(26).map(|v| v as i32 as i64));
                for number in crate::actions::weapon_numbers(stat(30).unwrap_or(0)) {
                    if !numbers.contains(&number) {
                        numbers.push(number);
                    }
                }
                reload |= energy != Some(-1);
                if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                    eprintln!("combat: weapon {template:x} in hand {slot}: can {:x?}, ammunition {energy:?}", stat(30));
                }
            }
            if reload {
                numbers.push(crate::actions::RELOAD);
            }
            self.weapon_specials = (worn, numbers);
        }
        for &number in &self.weapon_specials.1 {
            if !list.contains(&number) {
                list.push(number);
            }
            // (backstab beside a weapon's sneak attack, for a character the server has allowed it: FUN_1009e84b when the
            // weapon is put on, FUN_1009c5c8 when the server's word comes with the weapon already held)
            if number == crate::actions::SNEAK_ATTACK && self.backstab && !list.contains(&crate::actions::BACKSTAB) {
                list.push(crate::actions::BACKSTAB);
            }
        }
        Some(list)
    }

    /// A loss of health (negative) less what the weapon hits just told of that character already account for: what
    /// is left to tell as its own line (0: nothing). A hit cut short (a duel's last one leaves 1 health) covers the
    /// whole loss.
    fn untold_loss(&mut self, id: u32, change: i64) -> i64 {
        if change >= 0 {
            return change;
        }
        let told = self.recent_hits.remove(&id).filter(|h| h.1.elapsed() < Duration::from_secs(2)).map_or(0, |h| h.0);
        (change + told).min(0)
    }

    /// The nanos known to run on a character, for its NCU window: the nano, the time it had left (milliseconds) when
    /// that was told, and since when. The own character's are its list; another's are those its full update listed
    /// (while their time lasts) and those the server announces (no time: counted from the first announcement).
    fn nanos_on(&self, id: u32) -> Vec<(u32, Option<u32>, Instant)> {
        if id == self.own {
            let at = self.nano_times_at.unwrap_or_else(Instant::now);
            return self.nano_times.iter().map(|&(nano, ms)| (nano, Some(ms), at)).collect();
        }
        let mut out: Vec<(u32, Option<u32>, Instant)> = Vec::new();
        if let Some((times, at)) = self.other_nano_times.get(&id) {
            let elapsed = at.elapsed().as_millis();
            out.extend(times.iter().filter(|t| elapsed < u128::from(t.1)).map(|&(nano, ms)| (nano, Some(ms), *at)));
        }
        for &nano in self.active_buffs.get(&id).into_iter().flat_map(|buffs| buffs.keys()) {
            if !out.iter().any(|o| o.0 == nano) {
                out.push((nano, None, self.buff_first.get(&(id, nano)).copied().unwrap_or_else(Instant::now)));
            }
        }
        out
    }

    /// The spell runner's clock: seconds since its first use.
    fn spell_now(&mut self) -> f64 {
        self.spell_epoch.get_or_insert_with(Instant::now).elapsed().as_secs_f64()
    }

    /// What the spell runs did (`spelllist::Outcome`): effects to start, stuns on the state flags, stats set on one's own
    /// character (the client keeps no stats of others).
    fn spell_outcomes(&mut self, out: Vec<(crate::requirements::Holder, crate::spelllist::Outcome)>) {
        for (on, outcome) in out {
            match outcome {
                crate::spelllist::Outcome::Effect { effect, seconds } => self.spell_effects_made.push((on.1, effect, seconds)),
                crate::spelllist::Outcome::Stun { begin, npc } => {
                    // (FUN_1004496c starts the counts from stat 224; one's own taken as 6, free to move, when the server did not
                    // send it — a safety of this client's)
                    let start = if on.1 == self.own { self.stats.get(&224).map_or(6, |&v| v as u32) } else { 0 };
                    self.state_flags.entry(on.1).or_insert_with(|| crate::spelllist::StateFlags::new(start)).stun(begin, npc);
                }
                crate::spelllist::Outcome::SetStat { stat, value } => {
                    if on.1 == self.own {
                        self.stats.insert(stat, value as i64);
                        self.stats_changed = true;
                    }
                }
                crate::spelllist::Outcome::Text { spell } => {
                    let distance = self.position_of(on.1).zip(self.position_of(self.own)).map(|(a, b)| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt());
                    if let Some(line) = crate::notifications::spell_line(&spell, false, on.1 == self.own, distance) {
                        crate::notifications::add(&line);
                    }
                }
            }
        }
    }

    /// The waiting spell runs' turn (FUN_10002fc7), each frame.
    fn tick_spells(&mut self) {
        if self.spell_queue.runs.is_empty() {
            return;
        }
        let now = self.spell_now();
        let mut queue = std::mem::take(&mut self.spell_queue);
        let out = queue.tick(&*self, now);
        self.spell_queue = queue;
        self.spell_outcomes(out);
    }

    /// One's own character cannot move: a stun took both of its state flags 2 and 4 (`N3Msg_MouseMovement`, FUN_1006bd94).
    fn own_stunned(&self) -> bool {
        self.state_flags.get(&self.own).is_some_and(|f| !f.can_move())
    }

    /// The weapon template a character holds in a place: the server's `WeaponItemFullUpdate`, or the client's own inventory.
    fn weapon_of(&self, id: u32, slot: u32) -> Option<u32> {
        self.weapons.get(&(id, slot as u8)).copied().or_else(|| (id == self.own).then(|| self.inventory.as_ref()?.iter().find(|e| e.slot == slot).map(|e| e.low_id)).flatten())
    }

    /// The weapons' attack timers when the character starts fighting (FUN_1006a1b1 → FUN_1006872e): for each of the 16 weapon
    /// places with a weapon (FUN_100685bb), FUN_10068331 with the weapon's attack and recharge times (FUN_10067c74,
    /// FUN_10067d92, truncated): started again (FUN_1003d8c3, its time left then moved by `50 − rand() × first × 0.5 / 32767`)
    /// when forced — no attack entry is unheld (FUN_1003d0cb) — or when its entry is held, else only given the new times; then
    /// flag 2. The attack part is fighting again (+0x44 = 2) and every attack bar is shown again (FUN_1003d29b).
    fn start_attack_timers(&mut self) {
        use crate::timers::{ATTACK, HELD, TAKES_TURNS};
        let force = !self.timers.has_unheld(ATTACK);
        for slot in 0..16u32 {
            let Some(template) = self.weapon_of(self.own, slot) else { continue };
            let stats = &self.stats;
            let player = |k: u32| stats.get(&k).copied().unwrap_or(0);
            let item = |k: u32| template_stat(template, k).map(|v| v as i32);
            let first = crate::gui::iteminfo::swing_time(&item, &player, false) as i32;
            let second = crate::gui::iteminfo::swing_time(&item, &player, true) as i32;
            let (place, started) = match self.timers.find(ATTACK, slot) {
                Some(p) if !force && self.timers.get(p).is_some_and(|e| e.flags & HELD == 0) => {
                    let e = self.timers.entries[p].as_mut().unwrap();
                    (e.first, e.second) = (first, second);
                    (p, false)
                }
                Some(p) => {
                    self.timers.restart(p, first, second);
                    (self.timers.find(ATTACK, slot).unwrap(), true)
                }
                None => (self.timers.add(first, second, ATTACK, slot), true),
            };
            if started {
                let r = self.timer_rand.next();
                let jitter = 50 - (r as f64 * first as f64 * 0.5 / 32767.0) as i32;
                self.timers.entries[place].as_mut().unwrap().left += jitter as f32;
            }
            self.timers.entries[place].as_mut().unwrap().flags |= TAKES_TURNS;
        }
        self.timers.attacks_stopped = false;
        self.timers.signal_attacks();
    }

    /// The client's own nano cast (the server's `CastNanoSpell`: FUN_1007284f → FUN_10051c98 → FUN_1005141d): with no cast
    /// under way, its cast time ([`crate::timers::cast_seconds`]) as a kind-3 entry over that many hundredths, its second
    /// phase the nano's RechargeDelay (210), made the running entry, its bar shown (the one signal a nano entry gets). A cast
    /// of no time lands at once and has none. The entry goes when the cast lands (FUN_1004ebd0).
    fn start_cast_timer(&mut self, nano: u32) {
        if self.own_cast.is_some() {
            return;
        }
        let stats = &self.stats;
        let seconds = crate::timers::cast_seconds(&|k| nano_stat(nano, k), &|k| stats.get(&k).copied().unwrap_or(0));
        if seconds <= 0.0 {
            return;
        }
        let recharge = nano_stat(nano, 210).unwrap_or(0) as i32;
        let place = self.timers.add((seconds * 100.0) as i32, recharge, crate::timers::NANO, nano);
        self.timers.run(place);
        self.timers.events.push(crate::timers::Event::Start(place));
        self.own_cast = Some((place, Instant::now() + Duration::from_secs_f32(seconds)));
    }

    /// The timer list moved on (FUN_1003d517 each frame) and the bars made to follow it (`TimerSystemModule_t`), for the
    /// interface: the client's nano cast taken off when it lands (FUN_1004ebd0), an item's entry once its first phase is over
    /// (FUN_1003badb).
    fn tick_timers(&mut self) {
        let now = Instant::now();
        if self.own_cast.is_some_and(|(_, at)| now >= at) {
            let (place, _) = self.own_cast.take().unwrap();
            self.timers.remove(place);
        }
        let seconds = self.timers_moved.map_or(0.0, |t| now.duration_since(t).as_secs_f32());
        self.timers_moved = Some(now);
        // (Features, stat 224, bit 2: a character without the stat is taken to have it, as the captured player did)
        let features = self.stats.get(&224).is_none_or(|f| f & 2 != 0);
        self.timers.tick(seconds, features);
        while let Some(place) = self.timers.entries.iter().position(|e| e.as_ref().is_some_and(|e| e.kind == crate::timers::ITEM && e.flags & crate::timers::HELD != 0)) {
            self.timers.remove(place);
        }
        self.timer_bars.update(&mut self.timers);
        *crate::ui::TIMERS.lock().unwrap_or_else(|e| e.into_inner()) = self.timer_bars.shown(&self.timers);
    }

    /// An item of the backpack used (`(0x68, place)`; action 3 a plain use, 5 on something): FUN_1007ccb3 → FUN_1003bcdc →
    /// FUN_1003bb5a, unless an item timer runs: the item's ItemDelay (294) and RechargeDelay (210) as a kind-4 entry made the
    /// running one (flag 2) — except a plain use of a bag or of an item whose two times are under 31 hundredths, which is used
    /// at once (FUN_1003b8f3). (The original uses the item once the first phase is over; the client sends the use at once.)
    fn start_item_timer(&mut self, place: u32, action: u32) {
        use crate::timers::{ITEM, TAKES_TURNS};
        if self.timers.has(ITEM) {
            return;
        }
        let Some(e) = self.worn.iter().find(|e| e.slot == place) else { return };
        let (template, bag) = (e.low_id, e.is_bag());
        let recharge = template_stat(template, 210).unwrap_or(1234567890) as i32;
        let delay = template_stat(template, 294).unwrap_or(1234567890) as i32;
        if (delay.wrapping_add(recharge) < 31 || bag) && action == 3 {
            return;
        }
        let at = self.timers.add(delay, recharge, ITEM, place);
        self.timers.entries[at].as_mut().unwrap().flags |= TAKES_TURNS;
        self.timers.run(at);
    }

    /// The lines due by now: the end of a reload.
    fn tick_lines(&mut self) {
        if self.reload_done.is_some_and(|t| Instant::now() >= t) {
            self.reload_done = None;
            self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, crate::feedback::plain("Feedback_ReloadingCompleted")));
        }
    }

    /// Who a character is in a fight line: the client's own, one it knows by name, or one it doesn't.
    /// Someone as the damage meter counts them (`damage.rs`): by the name the client has for them; none for a stranger.
    fn fighter(&self, id: u32) -> Option<crate::damage::Actor> {
        match self.names.get(&id) {
            Some((name, player)) => Some(crate::damage::Actor { id, name: name.clone(), player: *player || id == self.own }),
            None if id == self.own => Some(crate::damage::Actor { id, name: "You".to_string(), player: true }),
            None => None,
        }
    }

    /// A blow, a miss or a heal for the damage meter.
    fn metered(&self, attacker: u32, victim: u32, amount: i64, heal: bool, hit: crate::damage::Hit, source: &str) {
        let Some(victim) = self.fighter(victim) else { return };
        crate::damage::record(crate::damage::Event { attacker: self.fighter(attacker), victim, amount: amount.abs(), heal, hit, source: source.to_string() });
    }

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
            self.own_flying = self.nanos.iter().any(|&n| db.get(id(0xfde85, n)).ok().flatten().is_some_and(|d| mao_formats::item::grants_flight(&d)));
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

    /// The server's word on a door (`DoorStatusUpdateIIR_t`'s apply, FUN_100a015a: open calls `DoorOpened`, closed
    /// `DoorClosed`): open stays open until the server says closed; closed ends what the server opened, not the few seconds
    /// a use of our own opened it for.
    fn door_status(&mut self, id: u32, open: bool) {
        let Some(n) = self.door_number(id) else { return };
        if open {
            self.doors_open.insert(n, Instant::now() + SERVER_OPEN);
        } else if self.doors_open.get(&n).is_some_and(|until| *until > Instant::now() + DOOR_OPEN) {
            self.doors_open.remove(&n);
        }
    }

    /// A frame's doors: those that open on approach opened while the character is at them (FUN_1007fccc on its touch: the
    /// door's state machine opens it, FUN_10080477 → FUN_10080319, and it closes again a while after), and each door's
    /// change from shut to open or back noted for its animation (FUN_1008563d plays the mesh's animation forward from 0,
    /// FUN_10085684 back from its end; FUN_10088970 steps it each frame).
    fn tick_doors(&mut self, own: [f32; 3]) {
        let now = Instant::now();
        for (&n, &(_, at)) in &self.door_slots {
            if self.approach_doors.contains(&n) && (at[0] - own[0]).hypot(at[2] - own[2]) < APPROACH_REACH && (at[1] - own[1]).abs() < 3.0 {
                let until = now + DOOR_OPEN;
                let open = self.doors_open.entry(n).or_insert(until);
                if *open < until {
                    *open = until;
                }
            }
        }
        for &n in self.door_slots.keys() {
            let open = self.doors_open.get(&n).is_some_and(|until| *until > now);
            match self.door_motion.get(&n) {
                Some(&(was, _)) if was == open => {}
                None if !open => {}
                _ => {
                    self.door_motion.insert(n, (open, now));
                }
            }
        }
    }

    /// A door's animation time: from 0 shut to its length open, run on from its last change.
    fn door_time(&self, n: u8, length: f32) -> f32 {
        match self.door_motion.get(&n) {
            Some(&(true, since)) => since.elapsed().as_secs_f32().min(length),
            Some(&(false, since)) => (length - since.elapsed().as_secs_f32()).max(0.0),
            None => 0.0,
        }
    }

    /// Opens the door that was used, if the id is one.
    fn door_used(&mut self, id: u32) {
        if self.types.get(&id) == Some(&DOOR_TYPE) {
            if let Some(n) = self.door_number(id) {
                eprintln!("door {id:#x} ({n}) used");
                self.doors_open.insert(n, Instant::now() + DOOR_OPEN);
            }
        }
    }

    /// Takes the playfield's built-in props: the doors are also remembered by number.
    fn set_static_props(&mut self, props: Vec<(u32, (u32, [f32; 3], [f32; 4]), Option<u8>)>) {
        self.terminals.clear();
        self.door_slots.clear();
        self.doors_open.clear();
        self.door_motion.clear();
        self.door_props.clear();
        self.prop_textures.retain(|k, _| *k < 0xfe00_0000);
        self.prop_anim.retain(|k, _| *k < 0xfe00_0000);
        for (key, prop, door) in props {
            if let Some(n) = door {
                self.door_slots.insert(n, (key, prop.1));
                self.door_props.insert(key, n);
            }
            self.props.insert(key, prop);
        }
        self.register_doors();
    }

    /// What a use of `(kind, id)` opens on the client's side (`client_window`), for the playfield's own terminals: the market's
    /// page, the mail window.
    fn opened_by_use(&mut self, db: &ResourceDatabase, kind: u32, id: u32) {
        let Some(template) = self.table.iter().find(|d| d.0 == kind && d.1 == id).map(|d| d.3) else { return };
        match crate::client_window(db, template) {
            Some(0) => {
                let ticket = mao_net::session::last_ticket();
                let stats = &self.stats;
                let headers = crate::browser::headers(self.own, ticket.as_ref(), self.server_id, &|s| stats.get(&s).copied().unwrap_or(0));
                self.browser = Some(crate::browser::Request { mode: crate::browser::Mode::Market, headers });
            }
            Some(8) => self.open_windows.push("mail"),
            _ => {}
        }
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
        if frame.kind == frame::N3 {
            self.server_id = frame.source;
        }
        let Some(m) = Message::parse(frame) else { return };
        // (MAO_COMBAT_LOG: the fight's messages as they come)
        if std::env::var_os("MAO_COMBAT_LOG").is_some() && matches!(m.kind, 0x28494070 | 0x46002f16 | 0x4a41203e | 0x5c654b28 | 0x206b4b73 | 0x50544d19 | 0x1d3c0f1c | 0x754f1115 | 0x51492120) || (m.kind == 0x5e477770 && m.data.get(4) == Some(&0x63)) {
            if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                let name = mao_net::schema::Schema::global().n3_name(m.kind).unwrap_or("?");
                eprintln!("combat: server {name} ({:#x}) from ({:x},{:x}) {} bytes", m.kind, m.identity.0, m.identity.1, m.data.len());
            }
        }
        if let Some(arrival) = m.arrival() {
            eprintln!("playfield message: identity ({:x},{:x}) -> playfield {} at {:?}", m.identity.0, m.identity.1, arrival.0, arrival.1);
            // (a generated building, a mission dungeon: its rooms stand in for the template playfield's)
            let building = m.generated();
            if let Some(b) = &building {
                eprintln!("  a generated building: {} x {} blocks, template {}, {} rooms", b.width, b.height, b.template, b.rooms.len());
            }
            crate::set_generated(building);
            if std::env::var_os("MAO_TRACE_ALL").is_some() {
                eprintln!("  playfield message data {}", m.data.iter().map(|b| format!("{b:02x}")).collect::<String>());
            }
            self.arrival = Some(arrival);
            self.pets.clear();
            self.instance_runs = m.instance_runs();
            if std::env::var_os("MAO_COMBAT_LOG").is_some() && !self.instance_runs.is_empty() {
                eprintln!("instance runs: {:x?}", self.instance_runs);
            }
            // (set down where it comes back to life: the white of its death clears)
            if std::mem::take(&mut self.own_dead) {
                crate::whiteout::clear(true);
            }
        }
        if let Some((seconds, period, day, server_time)) = m.game_time() {
            self.game_clock.set(seconds);
            self.game_clock.set_day(day);
            self.game_clock.set_server_time(server_time as i32);
            // (the perk window's untraining timer counts in the server's time)
            self.perks_changed = true;
            let (hour, minute) = self.game_clock.time();
            eprintln!("game time: {hour:02}:{minute:02} (period {period}, day {day}, server time {server_time})");
        }
        *self.kinds.entry(m.kind).or_default() += 1;
        // A terminal announced (`SimpleItemFullUpdate`, identity type `0xc73d`): usable by its id, named by its template
        // (`StaticInstance`, stat 23).
        if m.kind == 0x3b11256f && m.identity.0 == 0xc73d {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let f = |n: &str| match p.value.get("Position").and_then(|pos| pos.get(n)) {
                    Some(mao_net::schema::Value::Float(x)) => Some(*x as f32),
                    _ => None,
                };
                if let (Some(x), Some(y), Some(z)) = (f("X"), f("Y"), f("Z")) {
                    let template = match p.value.get("Stats") {
                        Some(mao_net::schema::Value::List(stats)) => stats.iter().find(|s| s.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(23)).and_then(|s| s.get("Item2")?.as_int()),
                        _ => None,
                    };
                    if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                        eprintln!("terminal ({:#x}, {:#x}) at {:?} template {template:?}", m.identity.0, m.identity.1, [x, y, z]);
                    }
                    self.terminals.insert(m.identity.1, [x, y, z]);
                    self.types.insert(m.identity.1, 0xc73d);
                    if let Some(name) = template.and_then(|t| template_name(t as u32)) {
                        self.names.insert(m.identity.1, (name, false));
                    }
                    // (one that is not the playfield's own, the Shadowlands' portal statues and the like, is drawn by its
                    // template's model, the `Mesh` stat 12, turned by the message's rotation, and can be picked and used)
                    if m.identity.1 >> 24 != 0xc0
                        && let Some(mesh) = template.and_then(|t| template_stat(t as u32, 12)).filter(|&mesh| mesh > 0)
                    {
                        let r = |n: &str| match p.value.get("Rotation").and_then(|q| q.get(n)) {
                            Some(mao_net::schema::Value::Float(v)) => *v as f32,
                            _ => 0.0,
                        };
                        let rotation = [r("X"), r("Y"), r("Z"), r("W")];
                        let rotation = if rotation == [0.0; 4] { [0.0, 0.0, 0.0, 1.0] } else { rotation };
                        self.props.insert(m.identity.1, (mesh as u32, [x, y, z], rotation));
                        self.objects.insert(m.identity.1, [x, y, z]);
                        // (how its model plays: the message's `AnimPlay` and `AnimPos`, else its template's)
                        let stat = |key: i64| match p.value.get("Stats") {
                            Some(mao_net::schema::Value::List(stats)) => stats.iter().find(|s| s.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(key)).and_then(|s| s.get("Item2")?.as_int()),
                            _ => None,
                        };
                        let of = |key: u32| stat(key as i64).or_else(|| template.and_then(|t| template_stat(t as u32, key))).unwrap_or(0) as u32;
                        self.prop_anim.insert(m.identity.1, (of(501), of(500), Instant::now()));
                    }
                }
            }
        }
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
                        // (an announced model takes the place of the playfield's own copy (its dynel table's); the
                        // usual announcement has none, only the template (`StaticInstance`), and the table's stays)
                        let at = [f("X"), f("Y"), f("Z")];
                        self.props.retain(|k, p| *k < 0xfe00_0000 || (p.1[0] - at[0]).powi(2) + (p.1[1] - at[1]).powi(2) + (p.1[2] - at[2]).powi(2) > 2.0);
                        let r = |n: &str| match p.value.get("Rotation").and_then(|q| q.get(n)) {
                            Some(mao_net::schema::Value::Float(x)) => *x as f32,
                            _ => 0.0,
                        };
                        self.props.insert(m.identity.1, (mesh as u32, [f("X"), f("Y"), f("Z")], [r("X"), r("Y"), r("Z"), r("W")]));
                    }
                    if let Some(mao_net::schema::Value::List(stats)) = p.value.get("Stats") {
                        if let Some(t) = stats.iter().find(|s| s.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(23)).and_then(|s| s.get("Item2")?.as_int()) {
                            self.vendor_templates.insert(m.identity.1, t as u32);
                        }
                    }
                    // Shops can be targeted (Ctrl+Tab) and used (E) like the other objects.
                    self.objects.insert(m.identity.1, [f("X"), f("Y"), f("Z")]);
                    // (its name: SimpleItem_t's FUN_100809a0 takes the `NameTemplate` stat's (446) item template's name, the
                    // `StaticInstance` being the machine all the shops share, "Specialist Commerce")
                    let stat = |key: i64| match p.value.get("Stats") {
                        Some(mao_net::schema::Value::List(stats)) => stats.iter().find(|s| s.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(key)).and_then(|s| s.get("Item2")?.as_int()),
                        _ => None,
                    };
                    let name_template = stat(446).filter(|&t| t > 0 && t != 1_234_567_890).map(|t| t as u32);
                    let name = name_template.and_then(template_name);
                    // (its skin: FUN_1008746a gives the visual `OverrideTexture` (336) after the mesh, `SetOverrideTexture`;
                    // the stat as the dynel has it, else its name template's: the ICC shops' sign is in that texture)
                    let skin = stat(336).filter(|&t| t > 0).map(|t| t as u32).or_else(|| name_template.and_then(|t| template_stat(t, 336)).filter(|&t| t > 0).map(|t| t as u32));
                    match skin {
                        Some(t) => {
                            self.prop_textures.insert(m.identity.1, t);
                        }
                        None => {
                            self.prop_textures.remove(&m.identity.1);
                        }
                    }
                    match name {
                        Some(name) => {
                            self.vendor_names.insert(m.identity.1, name.clone());
                            self.names.insert(m.identity.1, (name, false));
                        }
                        None => {
                            self.names.entry(m.identity.1).or_insert_with(|| (format!("Booth {:x}", m.identity.1 & 0xffff), false));
                        }
                    }
                }
            }
        }
        // The playfield's tower sites (`LandControl_t`: FUN_1012a1f4 replaces the list), a tower destroyed or planted
        // (FUN_1012b350), its houses (FUN_101304d5).
        if m.kind == mao_net::landcontrol::ALL_TOWERS {
            if let Some(list) = mao_net::landcontrol::all_towers(&m.data) {
                self.tower_playfield = m.identity.1;
                self.towers.insert(m.identity.1, list);
                self.towers_changed = true;
            }
        }
        if m.kind == mao_net::landcontrol::TOWER_UPDATE
            && let Some((site, planted)) = mao_net::landcontrol::tower_update(&m.data)
        {
            mao_net::landcontrol::apply_update(self.towers.entry(self.tower_playfield).or_default(), site, planted);
            self.towers_changed = true;
        }
        if m.kind == mao_net::landcontrol::ALL_CITIES
            && let Some(blob) = mao_net::landcontrol::all_cities(&m.data)
        {
            self.cities.insert(m.identity.1, blob);
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
                self.metered(attacker, defender, 0, false, crate::damage::Hit::Miss, skill.as_deref().unwrap_or("Weapon"));
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
            // (dead: no health left, whatever the last hit said)
            if let Some(h) = self.health.get_mut(&m.identity.1) {
                h.0 = 0;
            }
            self.last_hp.insert(m.identity.1, 0);
            if let Some(e) = self.entities.get(&m.identity.1) {
                self.bodies.retain(|_, b| b.1.elapsed() < Duration::from_secs(120));
                self.bodies.insert(m.identity.1, (e.0, Instant::now()));
            }
            // (the dead walk no further along the path they were on)
            if m.identity.1 != self.own {
                self.paths.remove(&m.identity.1);
            }
            self.fighting.remove(&m.identity.1);
            if m.identity.1 == self.own {
                // (dead until the server says there is health again or puts the character somewhere: its health is
                // nothing, as FUN_1005b3d8 sets it, whatever the last message of it said)
                self.own_dead = true;
                self.dead_health = 0;
                crate::whiteout::begin();
                if self.stats.insert(27, 0) != Some(0) {
                    self.stats_changed = true;
                }
            }
            self.died.push(m.identity.1);
        }
        // A `Despawn`: a dynel leaves. A body that died stays lying where it fell (the server despawns it just before it
        // announces the corpse there, and the corpse is what the original draws dead) until its corpse goes; anyone else
        // leaves the world.
        if m.kind == 0x36510078 && m.identity.1 != self.own {
            let id = m.identity.1;
            if m.identity.0 == OBJECT_TYPE {
                self.objects.remove(&id);
                let bodies: Vec<u32> = self.corpses.iter().filter(|(_, c)| **c == id).map(|(b, _)| *b).collect();
                for body in bodies {
                    self.corpses.remove(&body);
                    self.bodies.remove(&body);
                    self.dead.remove(&body);
                    self.entities.remove(&body);
                }
            } else if m.identity.0 == 0xc73d {
                // (a thing the server told of goes: a gas fire put out, a box taken away after its opening; the playfield's
                // own things are kept under keys of their own and stay)
                self.props.remove(&id);
                self.prop_anim.remove(&id);
                self.prop_textures.remove(&id);
                self.objects.remove(&id);
                self.terminals.remove(&id);
            } else if !self.dead.contains_key(&id) {
                self.spell_effects.remove(&id);
                self.spell_queue.forget((crate::requirements::CHARACTER, id));
                self.state_flags.remove(&id);
                self.entities.remove(&id);
                self.paths.remove(&id);
                self.fighting.remove(&id);
            }
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
            // (a spell list run on the character: FUN_100aabcb reads it, FUN_100aae0e → FUN_100026d0 runs its spells)
            crate::spelllist::SPELL_LIST => {
                if let Some(list) = crate::spelllist::SpellList::parse(&m.data) {
                    let now = self.spell_now();
                    let mut queue = std::mem::take(&mut self.spell_queue);
                    let mut out = crate::spelllist::run(&list, m.identity, &*self, &mut queue, now);
                    // (the queue's turn in the same moment, as the character's frame follows the message in the original's: a
                    // queued run's first hit can come at once)
                    out.extend(queue.tick(&*self, now));
                    self.spell_queue = queue;
                    self.spell_outcomes(out);
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
        // A new level (`NewLevel`, Gamecode FUN_10075f55): the message's numbers become the character's stats: the level
        // (54), the experience (52), the points to spend (53), the experience the level began at (57) and the next takes
        // (350), and the range of levels that give experience (275). No `Stat` message tells the last two, so the
        // experience bar had nothing to show after a level (seen live 2026-11-10). The client's own character is told
        // its level, and below level 4 that there are points to spend. Not ported: the line for the reset points gained
        // (with the level's title), the `got_ip`, `got_perk` and `got_tech` hints.
        if m.kind == 0x7f405a16 && m.identity.1 == self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let field = |name: &str| p.value.get(name).and_then(mao_net::schema::Value::as_int);
                for (stat, name) in [(54u32, "Level"), (52, "CurrentXp"), (53, "AvailableIp"), (57, "LastLevelXp"), (350, "NextLevelXp"), (275, "XpKillRange")] {
                    if let Some(v) = field(name) {
                        self.stats.insert(stat, v);
                    }
                }
                self.stats_changed = true;
                if let Some(level) = field("Level") {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, crate::feedback::with_number("Feedback_NewLevel", level)));
                    if level < 4 {
                        self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, crate::feedback::plain("Feedback_YouHaveReceivedIncreasePoints")));
                    }
                }
            }
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
                // (the client's own fight starting: its weapons' attack timers, FUN_1006a1b1)
                if m.identity.1 == self.own {
                    self.start_attack_timers();
                }
            }
        }
        if m.kind == 0x4a41203e {
            self.fighting.remove(&m.identity.1);
            // (one's own: FUN_1007a2be → FUN_100690c8 signals the end of every attack bar, (1, -1), and stops the attack part,
            // +0x44 = 1; the entries stay)
            if m.identity.1 == self.own {
                self.timers.events.push(crate::timers::Event::Stop(crate::timers::ATTACK, None));
                self.timers.attacks_stopped = true;
            }
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
                // (FUN_1006ae3c → FUN_1009b8ba: the weapon's attack entry is no longer held, so its recharge runs)
                if m.identity.1 == self.own {
                    if let Some(e) = self.timers.find(crate::timers::ATTACK, slot).and_then(|p| self.timers.entries[p].as_mut()) {
                        e.flags &= !crate::timers::HELD;
                    }
                }
                if let Some(e) = self.weapon_energy.get_mut(&(m.identity.1, slot as u8)) {
                    *e = int("AmmoCount");
                }
                // (the hit is remembered for its victim: the `Stat` that follows with the new health would else tell the
                // same loss a second time, as nanobots of unknown damage; seen live in a duel between two players)
                if int("Amount") > 0 {
                    let hit = self.recent_hits.entry(target).or_insert((0, Instant::now()));
                    *hit = (if hit.1.elapsed() < Duration::from_secs(2) { hit.0 } else { 0 } + int("Amount"), Instant::now());
                }
                let kind = crate::feedback::weapon_damage_type(self.weapons.get(&(m.identity.1, slot as u8)).and_then(|&t| template_stat(t, 436)));
                if int("Amount") != 0 {
                    self.metered(m.identity.1, target, int("Amount"), false, crate::damage::Hit::of_type(int("HitType")), "Weapon");
                }
                if let Some(line) = crate::feedback::attack(self.who(target), self.who(m.identity.1), int("Amount"), kind, int("HitType")) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                }
                // (the hit that kills names what the target died of, the word after the target, `Unk1`: FUN_1009f257 and
                // FUN_1009b6b7 hand it to FUN_1005b3d8, which tells the client's own character the game's line for it
                // and sets its health to nothing)
                if int("Unk1") != 0 && target == self.own {
                    self.own_died_of(int("Unk1"));
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
                // (the queued special's class: one's own queues its own, the others' come from the server's `CharSecSpecAttack`)
                let queued = self.queued_specials.remove(&m.identity.1).or((m.identity.1 == self.own).then_some(stat));
                let class = match queued.and_then(mao_formats::item::special_class) {
                    Some((class, false)) => class,
                    _ => 0xb,
                };
                let n = self.swings.get(&m.identity.1).map_or(0, |s| s.2.wrapping_add(1));
                self.swings.insert(m.identity.1, (slot, Instant::now(), n, class));
                // (the line, FUN_1006af0e: a weapon hit's with the special's skill named for the damage type; the rounds
                // left go to the weapon, FUN_10068869, and the hit is remembered as a weapon's is, for the `Stat` after it)
                let amount = p.value.get("Amount").and_then(mao_net::schema::Value::as_int).unwrap_or(0);
                let target = p.value.get("Target").and_then(|t| t.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                if let (Some(e), Some(left)) = (self.weapon_energy.get_mut(&(m.identity.1, slot as u8)), p.value.get("AmmoCount").and_then(mao_net::schema::Value::as_int)) {
                    *e = left;
                }
                if amount > 0 {
                    let hit = self.recent_hits.entry(target).or_insert((0, Instant::now()));
                    *hit = (if hit.1.elapsed() < Duration::from_secs(2) { hit.0 } else { 0 } + amount, Instant::now());
                }
                if amount != 0 {
                    self.metered(m.identity.1, target, amount, false, crate::damage::Hit::Normal, &crate::gui::criteria::stat_name(stat));
                }
                if let Some(line) = crate::feedback::special(self.who(target), self.who(m.identity.1), amount, &crate::gui::criteria::stat_name(stat)) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                }
            }
        }
        // `CharSecSpecAttack` (`51492120`) from the server, always just before the attacker's `SpecialAttackInfo`: a byte, the
        // target, the special's stat (0x94, 0x96). `CharSecSpecAttackIIR_t` (read FUN_10072cbc, apply FUN_10072d11) hands it to
        // the character's attack part (+0x1d4), FUN_10068cd9: with nothing queued (+0x9c) and the special not in the list at
        // +0x1bc it is queued (FUN_1006bacb on the queue at +0x8c), so the attack that follows swings the special's own class.
        if m.kind == 0x51492120 && m.data.len() >= 13 {
            let stat = u32::from_be_bytes(m.data[9..13].try_into().unwrap());
            self.queued_specials.entry(m.identity.1).or_insert(stat);
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
                // (the meshes that are drawn of those listed: a helmet in the head's place, `n3::shown_meshes`)
                let meshes: Vec<(u8, u8, u32, u32)> = list("Meshes").iter().map(|v| (field(v, "Layer") as u8, field(v, "Position") as u8, field(v, "Id"), field(v, "OverrideTextureId"))).collect();
                self.equipment.insert(m.identity.1, mao_net::n3::shown_meshes(&meshes));
            }
        }
        // `SpecialAttackWeapon`: the special attacks of a character, each an item, a stat and a four-letter code (the
        // plain attack 100 "MAAT", Dimach 144 "DIIT", Brawl 142 "BRAW" in the captures). One's own make the Action
        // window's special attacks (Gamecode FUN_1006b14c).
        if m.kind == SPECIAL_ATTACK_WEAPON && m.identity.1 == self.own && std::env::var_os("MAO_COMBAT_LOG").is_some() {
            eprintln!("combat: the server's special attacks: {}", m.data.iter().map(|b| format!("{b:02x}")).collect::<String>());
        }
        if m.kind == SPECIAL_ATTACK_WEAPON && m.identity.1 == self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let stats: Vec<u32> = match p.value.get("Specials") {
                    Some(mao_net::schema::Value::List(list)) => list.iter().filter_map(|s| s.get("Unknown3").and_then(mao_net::schema::Value::as_int)).map(|s| s as u32).collect(),
                    _ => Vec::new(),
                };
                if self.specials.as_ref() != Some(&stats) {
                    self.specials = Some(stats);
                    self.specials_changed = true;
                }
            }
        }
        if m.kind == 0x39343c68 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let nano = p.value.get("Buff").and_then(|b| b.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                if nano != 0 && m.identity.1 != self.own {
                    self.buffs_seen.push((m.identity.1, nano));
                    self.active_buffs.entry(m.identity.1).or_default().insert(nano, Instant::now());
                    self.buff_first.entry((m.identity.1, nano)).or_insert_with(Instant::now);
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
                // (FUN_10068fd2: a kind-5 entry over the stat, no second phase, flags 0x1a, made the running one)
                let place = match self.timers.find(crate::timers::RELOAD, weapon) {
                    Some(p) => {
                        self.timers.restart(p, delay as i32, 0);
                        self.timers.find(crate::timers::RELOAD, weapon).unwrap()
                    }
                    None => self.timers.add(delay as i32, 0, crate::timers::RELOAD, weapon),
                };
                self.timers.entries[place].as_mut().unwrap().flags |= 0x1a;
                self.timers.run(place);
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
                if amount != 0 {
                    self.metered(owner, victim, amount, false, crate::damage::Hit::Normal, if m.kind == 0x1c3a4f77 { "Reflect shield" } else { "Damage shield" });
                }
                if let Some(line) = crate::feedback::shield(self.who(victim), self.who(owner), amount, m.kind == 0x1c3a4f77) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                }
            }
        }
        // `AreaFormula` (`3129233b`): an area nano landing on the characters listed. `AreaFormulaIIR_t` reads (FUN_100721dc) a
        // byte, the characters (an `x3f1` list of identities), the nano (`0xcf1b` id: 0x3e380, 0x18762) and its source; the
        // apply (FUN_1007212d → FUN_100a9a99) runs the nano on each listed character the client has (its nano holder +0x1c0,
        // FUN_10051c98, as a cast landing: FUN_1005141d).
        if m.kind == 0x3129233b && m.data.len() >= 5 {
            let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            let count = (word(1) / 0x3f1).saturating_sub(1) as usize;
            let end = 5 + count * 8;
            if m.data.len() >= end + 12 {
                let nano = word(end);
                for i in 0..count {
                    let target = word(5 + i * 8 + 4);
                    if target == self.own || self.entities.contains_key(&target) || self.names.contains_key(&target) {
                        self.landed.push((target, nano));
                    }
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
                    if caster == self.own {
                        self.start_cast_timer(nano);
                    }
                    let plays = self.nano_casts.cast(caster, target, nano, &|n, s| nano_stat(n, s));
                    self.cast_sounds.extend(plays.into_iter().map(|(id, delay, volume)| (caster, id, delay, volume)));
                    self.landed.push((target, nano));
                    self.cast_bursts.push((caster, target, nano));
                    self.shots_pending.push((caster, target, nano));
                    // The caster plays its casting animation (at itself or at someone), when the nano takes time to cast.
                    // (one's own by its time with the initiative and the aggression setting taken off, `cast_seconds`;
                    // anyone else's, whose skill is not known here, by the nano's time alone)
                    let attack = nano_stat(nano, 294);
                    let takes_time = if caster == self.own {
                        let shown = crate::derived::shown(&self.stats, &self.item_mods);
                        attack.is_some_and(|t| t > 0 && cast_seconds(t, nano_stat(nano, 523), shown.get(&149).copied().unwrap_or(0), self.stats.get(&51).copied().unwrap_or(0)) > 0.0)
                    } else {
                        takes_time_to_cast(attack)
                    };
                    if takes_time {
                        self.socials.insert(caster, (if caster == target { "spell-self" } else { "spell-dir" }, Instant::now()));
                    }
                }
            }
        }
        // The cancels the server sends when a move starts (FUN_1005d61f, from the character it names):
        // - 0x89 (case 43, 0x1005d78f): FUN_1004f7b8 empties the character's casts under way (the list at +0x1c0): each cast's
        //   timer stopped and its end signalled (FUN_1004ebd0: `(cf1b, nano)` and the cast signal with 3), and with the list empty
        //   the character leaves its casting stance (FUN_1007c8af picks animation state 5 only while the list has a cast). So the
        //   cast's sounds stop and its casting animation ends.
        // - 0x92 (case 48, 0x1005e33a): FUN_1004949c ends the item use timer (+0x198, the item's stat 211 that FUN_100482c7 starts
        //   on a use: here the kind-4 item entry `start_item_timer` makes) and signals the container that holds the item named in the target as changed
        //   (`(0x6b, handle << 16 | place)`: the open container of that handle, FUN_1004892d; other types, FUN_100493eb): a
        //   refresh, not a close (the same signal follows `N3Msg_DropItem` and `N3Msg_ContainerAddItem`).
        // The duel's action as the server sends it to each of the two (Gamecode FUN_1005bd68, and seen live between two
        // of this client, 2026-11-10): the other character as the target, then what and a flag. A challenge (0) comes
        // with flag 0 to the one challenged and 1 to the challenger (the two boxes, in `ui.rs`); the rest are told with
        // a feedback text each: accepted (1), refused (2: by the other 0, taken back 1, by the other's option 2), ended
        // (3: lost 0, won 1, a draw 2), a draw proposed (4: by the other 0, one's own 1).
        // The pet duel's (FUN_1005ca5b, from the same place as the duel's): texts only, no box.
        if m.kind == 0x5e477770 && m.identity.1 == self.own && m.data.len() >= 21 {
            let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            if matches!(word(1), mao_net::n3::PET_DUEL_CHALLENGE | mao_net::n3::PET_DUEL_ANSWER | mao_net::n3::PET_DUEL_RESULT | mao_net::n3::PET_DUEL_CHALLENGED) {
                if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                    eprintln!("combat: pet duel message {:#x}, other {:x}, first {}: {}", word(1), word(13), word(17), m.data.iter().map(|b| format!("{b:02x}")).collect::<String>());
                }
                let other = self.names.get(&word(13)).map(|n| n.0.clone()).or_else(|| self.infos.get(&word(13)).map(|i| i.name.clone()));
                if let Some(line) = crate::feedback::pet_duel(word(1), word(17), other.as_deref()) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, line));
                }
            }
        }
        if m.kind == 0x5e477770 && m.identity.1 == self.own && m.data.len() >= 25 && u32::from_be_bytes(m.data[1..5].try_into().unwrap()) == mao_net::n3::DUEL {
            let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                eprintln!("combat: duel message, other {:x}, what {}, flag {}", word(13), word(17), word(21));
            }
            if let Some(name) = match (word(17), word(21)) {
                (1, _) => Some("Feedback_DuelAccepted"),
                (2, 0) => Some("Feedback_DuelRefused"),
                (2, 1) => Some("Feedback_DuelRetracted"),
                (2, 2) => Some("Feedback_DuelAutoRefused"),
                (3, 0) => Some("Feedback_DuelLost"),
                (3, 1) => Some("Feedback_DuelWon"),
                (3, 2) => Some("Feedback_DuelDraw"),
                (4, 0) => Some("Feedback_DuelProposeDraw"),
                (4, 1) => Some("Feedback_DuelDrawProposed"),
                _ => None,
            } {
                self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, crate::feedback::plain(name)));
            }
            self.duel_events.push((word(17), word(21), word(13)));
        }
        if m.kind == 0x5e477770 && m.data.len() >= 17 {
            let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            match word(1) {
                0x89 => {
                    let id = m.identity.1;
                    self.nano_casts.casts.remove(&id);
                    if id == self.own {
                        if let Some((place, _)) = self.own_cast.take() {
                            self.timers.remove(place);
                        }
                    }
                    if self.socials.get(&id).is_some_and(|s| s.0.starts_with("spell-")) {
                        self.socials.remove(&id);
                    }
                }
                // - 0x56 (case 21, 0x1005dc6a) and 0x57 (case 22, 0x1005dc76) give the character's movement controller (+0x50,
                //   FUN_1006f2e1: its +0x178, vtable +0x18) a movement: 0x56 sitting down (0x1e), 0x57 standing up (0x25
                //   `LeaveSit`; 0x29 or 0x2a when its `WaitState`, stat 430, is 0xf or 0x10, which this client does not keep).
                //   The play loop keeps one's own sitting itself (the server echoes the client's own 0x57).
                0x56 if m.identity.1 != self.own => {
                    self.seated.insert(m.identity.1);
                }
                // - 0x57 when sleeping or lounging (`WaitState` 0xf or 0x10): back to sitting on the ground, one's own too
                0x57 if self.resting.remove(&m.identity.1).is_some() => {}
                0x57 if m.identity.1 != self.own => {
                    self.stand_up(m.identity.1);
                }
                // - 0xa2 (case 55, 0x1005e54d, after the client's `/sneak` 0xa3): FUN_10044ff9 gives the movement controller the
                //   sneak mode (0x1c, `SwitchToSneakMode`): the character is sneaking
                0xa2 => {
                    self.sneaking.insert(m.identity.1);
                }
                // - 0x9b (case 51, 0x1005e361): one for each creature that has seen the sneaking character (the target): when the
                //   client has it, the feedback line "Feedback_YouHaveBeenDetected" fed its name (FUN_10012b6e, type 0xc)
                0x9b if m.identity.1 == self.own => {
                    if let Some(name) = self.names.get(&word(13)).map(|n| n.0.clone()) {
                        let line = crate::feedback::detected(&name);
                        self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, line));
                    }
                }
                // - 0x83 (case 41, 0x1005dcfe): FUN_1006b2dd equips the weapon named in the target (`(c74a, id)`) in the place in
                //   parameter 2 (6), out of a vehicle only, and draws it in the hand (FUN_1006ac49 under place 0x30)
                //   (FUN_1006ac49: then, a place other than 0 and the character without the flag 0x800, the place 0 is emptied)
                0x83 if m.data.len() >= 25 && word(9) == 0xc74a => {
                    if let Some(&template) = self.weapon_templates.get(&word(13)) {
                        let place = word(21);
                        self.weapons.insert((m.identity.1, place as u8), template);
                        if place != 0 && place < 0x30 && !self.weapon_flag.contains(&m.identity.1) {
                            self.weapons.remove(&(m.identity.1, 0));
                        }
                    }
                }
                // - 0xa7 (case 57, 0x1005e919): parameter 2 not zero sets the flag 0x800 (through the stats' vtable +0x18, an OR
                //   into +0x138), zero clears it
                0xa7 if m.data.len() >= 25 => {
                    if word(21) != 0 {
                        self.weapon_flag.insert(m.identity.1);
                    } else {
                        self.weapon_flag.remove(&m.identity.1);
                    }
                }
                0x92 if m.identity.1 == self.own => {
                    // (the item use timer ends: its bar goes, and the next use is free to start)
                    self.timers.remove_kind(crate::timers::ITEM);
                    let (kind, instance) = (word(9), word(13));
                    // (a move of an inventory item the server did not take: the item is back where it was)
                    if kind == 0x68
                        && let Some(at) = self.moves_asked.iter().position(|m| m.0 == instance)
                    {
                        let (from, to, swapped) = self.moves_asked.remove(at);
                        let (moved, other) = (self.worn.iter().position(|e| e.slot == to), self.worn.iter().position(|e| e.slot == from));
                        if let Some(i) = moved {
                            self.worn[i].slot = from;
                        }
                        if let (true, Some(j)) = (swapped, other) {
                            self.worn[j].slot = to;
                        }
                        self.inventory = Some(self.worn.clone());
                        self.mods_dirty = true;
                    }
                    if kind == 0x6b && self.loot.as_ref().is_some_and(|l| l.handle == instance >> 16) {
                        self.loot_changed = true;
                    }
                    if kind == 0x6b && self.bags.values().any(|b| b.handle == instance >> 16) {
                        self.bags_changed = true;
                    }
                }
                _ => {}
            }
        }
        // A character's social action (a dance, a bow): the last word of the message is the action's number.
        if m.kind == 0x3b290771 && m.data.len() >= 13 {
            let action = u32::from_be_bytes(m.data[m.data.len() - 4..].try_into().unwrap());
            if let Some(name) = crate::actions_data::SOCIAL.iter().find(|a| a.0 == action).map(|a| a.1) {
                self.socials.insert(m.identity.1, (name, Instant::now()));
            }
            // (sleeping, 0x44, and lounging, 0x45: FUN_1007aec0 hands a command of type 1 (+0x18) to the movement controller as
            // `SwitchToSleepMode` 0x21 and `SwitchToLoungeMode` 0x22, which set `WaitState` 0xf and 0x10)
            if m.data[1..5] == [0, 0, 0, 1] && (action == 0x44 || action == 0x45) {
                self.resting.insert(m.identity.1, if action == 0x44 { 0xf } else { 0x10 });
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
                // `UploadNano` (0xcc, after a nano crystal was used): the nano `(0xcf1b, id)` in the parameters is now one of the
                // character's programs (FUN_1005d61f's case 0x56: FUN_10050100 adds it to the nano holder at +0x1c0, then signal 6
                // refreshes the lists); the crystal itself goes with the `DeleteItem` (0x70) that follows
                0xcc if word(1) == 0xcc && word(17) == 0xcf1b => {
                    if !self.program_ids.contains(&word(21)) {
                        self.program_ids.push(word(21));
                        self.programs_changed = true;
                    }
                }
                // `PerkActionInfo` (0xb4): one of the character's perk actions (its item template, number and code)
                0xb4 => {
                    let code: [u8; 4] = m.data[21..25].try_into().unwrap();
                    let entry = (word(13), word(17), code);
                    match self.perk_actions.iter_mut().find(|p| p.1 == entry.1) {
                        Some(p) => *p = entry,
                        None => self.perk_actions.push(entry),
                    }
                    self.perk_actions_changed = true;
                }
                // 0xb5 with its second parameter not zero (FUN_1005d61f's case 0x44, FUN_10068976): the character may
                // backstab. The original sets bit 20 of the character's second flag word and has each hand's weapon add
                // Backstab beside its sneak attack (FUN_1009c5c8); nothing in it takes the bit away again. No capture has
                // the message: which characters are sent it was not seen.
                0xb5 if word(21) != 0 => self.backstab = true,
                // `PerkUnavailable` (0xcf): a perk lock (the lock-skill spell's group) runs for that many seconds; `PerkAvailable`
                // (0xce): it is over
                0xcf => {
                    self.perk_locks.insert(word(13), (Instant::now() + Duration::from_secs(word(17) as u64), word(17) as f32));
                    self.perk_actions_changed = true;
                }
                // `QueuePerk` (0x50, case 19, 0x1005f12e: just after the client's perk use 0xb3): parameters `(2, delay)`, the delay in
                // hundredths (0x64, 0xc8). The character's timer list (+0x1e8, FUN_10058b83) gets an entry of kind 2 for that long
                // (FUN_1003d9e0 → FUN_1003d8c3), which the interface shows as the "Special" timer bar (GUI.dll `StartTimerBar`);
                // parameters `(0, 0)` take the kind-2 entries away (FUN_1003d496)
                0x50 => match (word(17), word(21)) {
                    (0, 0) => self.timers.remove_kind(crate::timers::SPECIAL),
                    (_, delay) => {
                        use crate::timers::{NEVER_HELD, ONCE, SPECIAL, TAKES_TURNS};
                        self.queued_perk = Some(delay);
                        let at = self.timers.add(delay as i32, 0, SPECIAL, 0);
                        self.timers.entries[at].as_mut().unwrap().flags |= TAKES_TURNS | ONCE | NEVER_HELD;
                    }
                },
                // `StopLogout` (0x7a, case 37, 0x1005e02f: the server's answer when the timed logout is given up): Gamecode raises
                // interface message 0x2a (FUN_10012b1c), which GUI.dll takes as AFCM 0x20, `FlowControlModule_t::CancelCampMessage`
                // (0x1002a75a): the logout timer bar is deleted and "Timed logout aborted." told. (The client's own standing up
                // already did both when it was the one to give up.)
                0x7a => {
                    if crate::ui::CAMP.lock().unwrap_or_else(|e| e.into_inner()).take().is_some() {
                        self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, crate::feedback::gui("TimedLogoutAborted", &[])));
                    }
                }
                0xce => {
                    self.perk_locks.remove(&word(17));
                    self.perk_actions_changed = true;
                }
                // `DeleteItem` from the server (0x70): an item used up (a combine's pieces): `(0x68, place)` leaves the backpack
                0x70 if word(9) == 0x68 => {
                    let place = word(13);
                    if self.worn.iter().any(|e| e.slot == place) {
                        self.worn.retain(|e| e.slot != place);
                        self.inventory = Some(self.worn.clone());
                    }
                }
                // a perk action gone (0xb6, after an untraining: its item template, number and code)
                0xb6 => {
                    self.perk_actions.retain(|p| p.1 != word(17));
                    self.perk_actions_changed = true;
                }
                // `TrainPerk` / `UntrainPerk` echoed: the rank is now owned, or no longer
                0xbb | 0xbc => {
                    let rank = word(21);
                    self.perks.retain(|&p| p != rank);
                    if m.data[4] == 0xbb {
                        self.perks.push(rank);
                    }
                    self.perks_changed = true;
                    self.mods_dirty = true;
                }
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
                if info.level > 0 {
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
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, text.clone()));
                    // (and the info view on it, as the original's `charid://50000/<id>` page)
                    let page = match mao_net::n3::character_info(&m.data) {
                        // (a character: the original's page, FUN_1003482c)
                        Some(detail) => {
                            let id = m.identity.1;
                            let entity = self.infos.get(&id);
                            crate::charinfo::page(&crate::charinfo::Character {
                                name: name.clone(),
                                first: detail.first,
                                last: detail.last,
                                breed: entity.map_or(0, |e| e.breed as i32),
                                sex: entity.map_or(0, |e| e.sex as i32),
                                profession: info.profession as i32,
                                title_level: detail.title_level as i64,
                                level: info.level as i64,
                                side: entity.map(|e| e.side as i64),
                                org: self.orgs.get(&id).cloned(),
                                rank: detail.rank,
                                invaders_killed: detail.invaders_killed as i64,
                                killed_by_invaders: detail.killed_by_invaders as i64,
                                ai_level: detail.ai_level as i64,
                                duel_wins: detail.duel_wins as i64,
                                duel_losses: detail.duel_losses as i64,
                                solo_kills: detail.solo_kills as i64,
                                team_kills: detail.team_kills as i64,
                                own: id == self.own,
                            })
                        }
                        None => format!("<font color=CCInfoHeadline>{name}</font><br><br>Health: {} / {}<br>", info.health, info.max_health),
                    };
                    self.info_urls.push(format!("text://{page}"));
                }
            }
        }
        // The game's own text and the fight.
        if m.kind == 0x5f4b442a {
            if let Some(text) = mao_net::n3::chat_text(&m.data) {
                // (packed texts in it, `~&` and base-85 words naming a text.mdb text and its arguments up to a `~`, as the
                // answers to the organization's commands have: LDB.dll's `RemoteFormat::ParseString`)
                let text = match crate::feedback::texts() {
                    Some(texts) if text.contains("~&") => mao_formats::ldb::parse_string(texts, &text),
                    _ => text,
                };
                self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, mao_net::chat::plain(&text)).with_markup("", &text));
            }
        }
        // `Feedback` (`50544d19`): a text.mdb line by its category and key ("You are too far away; please move closer!" when a
        // fight is out of reach), shown in the chat as the original's feedback lines are.
        if m.kind == 0x50544d19 && m.identity.1 == self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let int = |f: &str| p.value.get(f).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as i32;
                if let Some(text) = crate::feedback::texts().and_then(|t| t.get(int("CategoryId"), int("MessageId"))).filter(|t| !t.is_empty()) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, text.to_string()));
                }
            }
        }
        if m.kind == 0x206b4b73 && m.identity.1 == self.own {
            // (its `Message` is packed, `~&` and base-85 words naming a text.mdb text and its arguments: LDB.dll's
            // `RemoteFormat` unpacks it)
            let packed = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data).ok().and_then(|p| match p.value.get("Message") {
                Some(mao_net::schema::Value::Str(t)) if t.starts_with("~&") => Some(t.clone()),
                _ => None,
            });
            let text = match (packed, crate::feedback::texts()) {
                (Some(t), Some(texts)) => Some(mao_formats::ldb::remote(texts, &t)).filter(|t| !t.is_empty()),
                _ => mao_net::n3::feedback_text(&m.data),
            };
            if let Some(text) = text {
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
                // (the damage meter: a loss is the source's damage, by the nano program when one is named; a gain its
                // healing; a fall, stat 0x1da, is nobody's)
                if amount < 0 && stat != 0x1da {
                    self.metered(source, target, amount, false, crate::damage::Hit::Normal, nano.as_deref().unwrap_or("Nano program or other"));
                } else if amount > 0 {
                    self.metered(if source == 0 { target } else { source }, target, amount, true, crate::damage::Hit::Normal, nano.as_deref().unwrap_or("Healing"));
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
                if std::env::var_os("MAO_COMBAT_LOG").is_some() && entries.is_empty() {
                    eprintln!("combat: container ({:#x}, {:#x}) is empty", identity.0, identity.1);
                }
                if identity.0 == crate::loot::BAG && self.worn.iter().any(|e| e.container == identity) {
                    // (one's own backpack: a window of its own, GUI.dll's `N3Msg_IsItemMine` branch of FUN_100ccefe)
                    let handle = p.value.get("Handle").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                    self.bags.insert(identity, Loot { identity, handle, entries });
                    self.bags_changed = true;
                } else if entries.is_empty() && identity.0 != 0xc76a && identity.0 != 0xc749 {
                    if self.loot.as_ref().is_some_and(|l| l.identity == identity) {
                        self.loot = None;
                        self.loot_changed = true;
                    }
                } else if identity.0 != 0 {
                    // (an empty corpse or chest still opens, and closes itself a second later: `InventoryView_c`'s timer)
                    let handle = p.value.get("Handle").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                    if identity.0 == 0xc76a {
                        self.fresh_corpse = Some((handle, entries.iter().map(|e| e.slot).collect()));
                    }
                    if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                        eprintln!("combat: container ({:#x}, {:#x}) handle {handle:#x} opened with {} items", identity.0, identity.1, entries.len());
                    }
                    self.loot = Some(Loot { identity, handle, entries });
                    self.loot_changed = true;
                }
            }
        }
        // The bank (`BankIIR_t`, the server's `Bank` message after a bank terminal's use): its items, shown as GUI.dll shows the
        // container `0xdead` (`InventoryView_c`'s `bank_window`, FUN_100ccefe).
        if m.kind == BANK_MESSAGE {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let entries = inventory_from_schema(p.value.get("BankSlots"));
                if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                    eprintln!("bank: {} items ({} bytes) {:?}", entries.len(), m.data.len(), entries.iter().take(4).map(|e| (e.slot, e.container, e.low_id)).collect::<Vec<_>>());
                }
                self.loot = Some(Loot { identity: (crate::loot::BANK, 0), handle: 0, entries });
                self.loot_changed = true;
            }
        }
        // Items that come to the inventory: `AddTemplate` (`052e2f0c`: bought in a shop, a reward) at once, and a
        // `TemplateAction` (`35505644`) into the overflow window, kept until its `ContainerAddItem` (aosharp's
        // `OnAddTemplateMessage` / `OnTemplateMessage`; the Arete Landing capture).
        // The team (`team.rs`): an invitation (action 0x1a naming who asks), who left (0x20; oneself: the team is gone), the
        // members (`TeamMember`) and their health and nano (`TeamMemberInfo`).
        if m.kind == 0x5e477770 && m.identity.1 == self.own && m.data.len() >= 17 {
            let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            let (action, who) = (word(1), word(13));
            if action == 0x1a {
                let name = self.names.get(&who).map_or_else(|| format!("Character {who:#x}"), |n| n.0.clone());
                self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, format!("{name} invites you to join a team.")));
                crate::team::with(|t| t.invite = Some((who, name)));
                crate::tips::trigger("OnInvitedToTeam", "");
            } else if action == 0x20 {
                crate::team::with(|t| {
                    if who == self.own {
                        t.members.clear();
                        t.leader = None;
                    } else {
                        t.members.retain(|m| m.id != who);
                    }
                });
            } else if action == 0x1b {
                // (case 6, 0x1005d7fc: interface message 0x10 with the target, whose instance is the team level)
                if let Some(line) = crate::team::with(|t| crate::team::set_level(t, who as i64)) {
                    self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, line));
                }
            } else if action == 0x23 {
                // (case 9, 0x1005d8cb: one's own team part takes the target, FUN_10065c5e, and interface message 0x17 names
                // it the leader: `SlotTeamEvent` keeps it at +0xe0 and `IsTeamLeader` is whether it is oneself)
                crate::team::with(|t| t.leader = Some(who));
            }
        }
        // The pets (`pets.rs`): `AddPet` names one, into the first free place (`GetFreePetSlot`, a pet already listed stays
        // where it is); `RemovePet` takes one away.
        if m.kind == 0x0d381f02 && m.identity.1 == self.own && m.data.len() >= 17 {
            let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            match word(9) {
                1 => {
                    self.pet_flashes.insert(word(5), (word(13), Instant::now()));
                }
                2 => {
                    self.pet_flashes.remove(&word(5));
                }
                _ => {}
            }
        }
        if let Some((added, pet)) = mao_net::n3::pet_change(&m) {
            if !added {
                self.pets.retain(|_, p| *p != pet);
            } else if !self.pets.values().any(|p| *p == pet) {
                let slot = (1..).find(|k| !self.pets.contains_key(k)).unwrap_or(1);
                self.pets.insert(slot, pet);
                // (the damage meter counts a pet's damage with its owner's)
                crate::damage::pet(pet.1, self.own, self.names.get(&self.own).map_or("You", |n| n.0.as_str()));
            }
        }
        if m.kind == TEAM_MEMBER || m.kind == TEAM_MEMBER_INFO {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let int = |f: &str| p.value.get(f).and_then(mao_net::schema::Value::as_int).unwrap_or(0);
                let who = p.value.get("Character").and_then(|v| v.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                crate::team::with(|t| {
                    let mate = t.member(who);
                    if m.kind == TEAM_MEMBER {
                        if let Some(mao_net::schema::Value::Str(name)) = p.value.get("Name") {
                            mate.name = name.clone();
                        }
                        (mate.level, mate.profession) = (int("Unknown2"), int("Unknown3"));
                    } else {
                        // (health, max health, max nano, nano: the capture's last one is a member at full, 227/227 and 248/248)
                        mate.health = (int("Unknown2"), int("Unknown4"));
                        mate.nano = (int("Unknown8"), int("Unknown6"));
                    }
                });
            }
        }
        // `Skill` (`3e205660`) from the server: the raised skills' new values and the IP left.
        if m.kind == SKILL && m.identity.1 == self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                self.read_stats(p.value.get("Skills"));
                self.stats_changed = true;
            }
        }
        if m.identity.1 == self.own && (m.kind == ADD_TEMPLATE || m.kind == TEMPLATE_ACTION) {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let int = |f: &str| p.value.get(f).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                if m.kind == ADD_TEMPLATE {
                    let entry = inventory::Entry { slot: 0, flags: 0, count: int("Count").max(1) as u16, container: (0, 0), low_id: int("LowId"), high_id: int("HighId"), quality: int("Quality"), extra: 0 };
                    self.add_to_backpack(entry);
                } else {
                    let placement = p.value.get("Placement").and_then(|v| v.get("Type")).and_then(mao_net::schema::Value::as_int).unwrap_or(0);
                    if placement == 0x6e {
                        self.overflow_item = Some(inventory::Entry { slot: 0, flags: 0, count: 1, container: (0, 0), low_id: int("ItemLowId"), high_id: int("ItemHighId"), quality: int("Quality"), extra: 0 });
                    }
                }
            }
        }
        if m.kind == CONTAINER_ADD_ITEM && m.identity.1 == self.own {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let source = (p.value.get("Source").and_then(|v| v.get("Type")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32, p.value.get("Source").and_then(|v| v.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32);
                let target = (p.value.get("Target").and_then(|v| v.get("Type")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32, p.value.get("Target").and_then(|v| v.get("Instance")).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32);
                // (a move of an inventory item the server took: nothing to take back)
                if source.0 == 0x68 {
                    self.moves_asked.retain(|m| m.0 != source.1);
                }
                // `(0x6e, 0)` to `(0x6e, own)`: the overflow window's item comes into the inventory
                if source == (0x6e, 0) && target == (0x6e, self.own) {
                    if let Some(entry) = self.overflow_item.take() {
                        self.add_to_backpack(entry);
                    }
                }
                // `(0x68, place)` into one of one's own open backpacks: the item leaves the inventory for the bag's first
                // free place (a guess, as for the other containers: the server's own place was not found)
                if source.0 == 0x68 && self.bags.contains_key(&target) {
                    if let Some(at) = self.worn.iter().position(|e| e.slot == source.1) {
                        let mut entry = self.worn.remove(at);
                        if let Some(bag) = self.bags.get_mut(&target) {
                            entry.slot = (0..).find(|p| !bag.entries.iter().any(|e| e.slot == *p)).unwrap_or(0);
                            bag.entries.push(entry);
                        }
                        self.inventory = Some(self.worn.clone());
                        self.bags_changed = true;
                    }
                }
                // `(0x6b, handle << 16 | place)` with a backpack's handle: an item came out of that bag into the inventory
                let bag_key = if source.0 == 0x6b { self.bags.iter().find(|(_, b)| b.handle == source.1 >> 16).map(|(k, _)| *k) } else { None };
                if let Some(key) = bag_key {
                    let place = source.1 & 0xffff;
                    let taken = self.bags.get_mut(&key).and_then(|b| b.entries.iter().position(|e| e.slot == place).map(|at| b.entries.remove(at)));
                    if let Some(mut entry) = taken {
                        if let Some(free) = (inventory::BACKPACK_FIRST..inventory::BACKPACK_FIRST + crate::gear::PACK_SLOTS).find(|s| !self.worn.iter().any(|e| e.slot == *s)) {
                            entry.slot = free;
                            self.worn.push(entry);
                            self.inventory = Some(self.worn.clone());
                        }
                        self.bags_changed = true;
                    }
                }
                // `(0x68, place)` into the open container: an item went in from the backpack (the bank by its type alone)
                if source.0 == 0x68 && self.loot.as_ref().is_some_and(|l| l.identity == target || (l.identity.0 == crate::loot::BANK && target.0 == crate::loot::BANK)) {
                    if let Some(at) = self.worn.iter().position(|e| e.slot == source.1) {
                        let mut entry = self.worn.remove(at);
                        if let Some(loot) = self.loot.as_mut() {
                            // (into the bank: its first free place, as aosharp's `Bank` takes the next available slot)
                            entry.slot = if loot.identity.0 == crate::loot::BANK {
                                (0..).find(|p| !loot.entries.iter().any(|e| e.slot == *p)).unwrap_or(0)
                            } else {
                                loot.entries.iter().map(|e| e.slot + 1).max().unwrap_or(0)
                            };
                            loot.entries.push(entry);
                        }
                        self.inventory = Some(self.worn.clone());
                        self.loot_changed = true;
                    }
                }
                // `(0x6b, handle << 16 | place)`: an item came out of the open container into the backpack (out of the bank: the
                // item's identity as the bank named it)
                let bank_item = self.loot.as_ref().filter(|l| l.identity.0 == crate::loot::BANK).and_then(|l| l.entries.iter().find(|e| e.container == source || (source.0 == 0x69 && e.slot == source.1)).map(|e| e.slot));
                if bag_key.is_none() && (source.0 == 0x6b || bank_item.is_some()) {
                    let place = bank_item.unwrap_or(source.1 & 0xffff);
                    if let Some(loot) = self.loot.as_mut() {
                        if let Some(at) = loot.entries.iter().position(|e| e.slot == place) {
                            let mut entry = loot.entries.remove(at);
                            if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                                eprintln!("combat: took item {} (place {place}) from the container", entry.low_id);
                            }
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
        // `InventoryUpdated` (`485e7202`: a byte and a value, 5 in the captures; sent to the character and to the shop after a
        // sale): `InventoryUpdatedIIR_t`'s apply (FUN_10075392) acts only on the client's own character, raising the
        // interface signal of that value (FUN_1001143e: 5 is the inventory's, +0xd8): the inventory windows are drawn again.
        if m.kind == 0x485e7202 && m.identity.1 == self.own && m.data.len() >= 5 {
            self.inventory = Some(self.worn.clone());
        }
        if m.kind == 0x36510078 && self.loot.as_ref().is_some_and(|l| l.identity == m.identity) {
            self.loot = None;
            self.loot_changed = true;
        }
        if m.kind == DOOR_STATUS && std::env::var("MAO_TRACE").is_ok_and(|v| v == "1") {
            eprintln!("door status {:x}:{:x}: {}", m.identity.0, m.identity.1, m.data.iter().map(|b| format!("{b:02x}")).collect::<String>());
        }
        if m.kind == DOOR_STATUS && m.identity.0 == DOOR_TYPE && self.door_ids.insert(m.identity.1) {
            self.register_doors();
        }
        if m.kind == DOOR_STATUS && m.identity.0 == DOOR_TYPE
            && let Some(open) = door_open(&m.data)
        {
            // (logged: a door that keeps opening and closing is this message or our own uses; see `door_used`)
            eprintln!("door {:#x} ({:?}) {} by the server", m.identity.1, self.door_number(m.identity.1), if open { "opened" } else { "closed" });
            self.door_status(m.identity.1, open);
        }
        // `DoorFullUpdate` (`365a5071`): a door the playfield's table does not have (a generated mission's), announced with its
        // place. Its `DoorFullUpdateIIR_t` (read by FUN_1009ff81 after the `SimpleItemFullUpdateIIR_t` part, made a dynel by
        // FUN_1009fff9) starts as a simple item's does: a byte, the version (0x0b), the owner (none: type and instance 0), then
        // the position and the rotation, then the playfield, the `f424f` state machine, a word, a short and the stats (`x3f1`
        // count, then stat and value pairs). The door becomes a usable object there, named as the others are, and is drawn by
        // its template's model (`StaticInstance`, stat 23, whose `Mesh` is stat 12) turned by its rotation, as the announced
        // terminals are.
        if m.kind == DOOR_FULL_UPDATE && m.identity.0 == DOOR_TYPE && m.data.len() >= 41 && m.data[1..13] == [0, 0, 0, 0x0b, 0, 0, 0, 0, 0, 0, 0, 0] {
            let f = |at: usize| f32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            let at = [f(13), f(17), f(21)];
            self.objects.insert(m.identity.1, at);
            self.names.entry(m.identity.1).or_insert_with(|| ("Door".to_string(), false));
            self.door_ids.insert(m.identity.1);
            let word = |at: usize| m.data.get(at..at + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()));
            let count = word(55).and_then(|n| usize::try_from(n / 0x3f1).ok()?.checked_sub(1)).unwrap_or(0).min(64);
            let stat = |key: u32| (0..count).map(|i| 59 + 8 * i).find(|&p| word(p) == Some(key)).and_then(|p| word(p + 4));
            // The door takes a number like the table's doors (its place in the run of ids), so the server's status messages, a
            // use of our own and the swing animation find it; the door flags (stat 259) bit 0x80 is "open" (`DoorFullUpdateIIR_t`'s
            // apply FUN_1009fff9 opens or closes the new door by it).
            if let Some(n) = self.door_number(m.identity.1) {
                self.door_slots.insert(n, (m.identity.1, at));
                if stat(259).is_some_and(|flags| flags & 0x80 != 0) {
                    self.doors_open.insert(n, Instant::now() + SERVER_OPEN);
                }
                // (and whether it opens at a touch: not when its flags say it is locked, `approach_doors`)
                match stat(259) {
                    Some(flags) if crate::door_locked(flags) => {
                        self.approach_doors.remove(&n);
                    }
                    Some(_) => {
                        self.approach_doors.insert(n);
                    }
                    None => {}
                }
            }
            let template = stat(23).filter(|&t| t > 0);
            if let Some(template) = template {
                self.door_templates.insert(m.identity.1, template);
                if let Some(mesh) = template_stat(template, 12).filter(|&mesh| mesh > 0) {
                    let rotation = [f(25), f(29), f(33), f(37)];
                    let rotation = if rotation == [0.0; 4] { [0.0, 0.0, 0.0, 1.0] } else { rotation };
                    self.props.insert(m.identity.1, (mesh as u32, at, rotation));
                    // (the announced door has its own model: the table's prop for the same door is not drawn as well)
                    if let Some(n) = self.door_number(m.identity.1) {
                        let table: Vec<u32> = self.door_props.iter().filter(|(_, d)| **d == n).map(|(k, _)| *k).collect();
                        for key in table {
                            self.props.remove(&key);
                            self.door_props.remove(&key);
                        }
                    }
                }
            }
        }
        if m.kind == mao_net::destinations::DESTINATION_LIST {
            self.destinations = mao_net::destinations::parse(&m.data);
        }
        if m.kind == 0x260f3671 {
            if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                let kind = p.value.get("Type").and_then(mao_net::schema::Value::as_int).unwrap_or(0);
                let info = p.value.get("Info");
                let vector = |v: &mao_net::schema::Value| -> Option<[f32; 3]> {
                    let f = |n: &str| match v.get(n) {
                        Some(mao_net::schema::Value::Float(x)) => Some(*x as f32),
                        _ => None,
                    };
                    Some([f("X")?, f("Y")?, f("Z")?])
                };
                let path = match kind {
                    1 => match info.and_then(|i| i.get("Waypoints")) {
                        Some(mao_net::schema::Value::List(points)) => Some(Route::Waypoints(points.iter().filter_map(vector).collect())),
                        _ => None,
                    },
                    2 => info.and_then(|i| i.get("Target")).and_then(|t| t.get("Instance")).and_then(mao_net::schema::Value::as_int).map(|t| Route::Chase(t as u32)),
                    _ => None,
                };
                if std::env::var_os("MAO_PATH_LOG").is_some() {
                    eprintln!("combat: [{:.2}] {:#x} {} follows {path:?}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64() % 1000.0), m.identity.1, self.names.get(&m.identity.1).map_or("?", |n| n.0.as_str()));
                }
                match path {
                    Some(Route::Waypoints(mut points)) if m.identity.1 != self.own && !points.is_empty() => {
                        // (the first place is where the server has it now: it starts from there, and the paths in a row tell
                        // its speed)
                        let start = points.remove(0);
                        if let Some(previous) = self.path_seen.get(&m.identity.1) {
                            if let Some(speed) = path_speed(*previous, start, Instant::now()) {
                                self.path_speeds.insert(m.identity.1, speed);
                            }
                        }
                        self.path_seen.insert(m.identity.1, (Instant::now(), start, *points.last().unwrap_or(&start)));
                        if let Some(e) = self.entities.get_mut(&m.identity.1) {
                            e.0 = start;
                        }
                        self.paths.insert(m.identity.1, Route::Waypoints(points));
                    }
                    Some(path) if m.identity.1 != self.own => {
                        self.paths.insert(m.identity.1, path);
                    }
                    _ => {
                        self.paths.remove(&m.identity.1);
                    }
                }
            }
        }
        // `N3Teleport` (`43197d22`, N3.dll's `n3TeleportIIR_t::Activate` 0x10029f87): within the playfield (the proxy's instance
        // zero: a generated mission's move, `c7a0`) any dynel is put at the destination with the rotation at once; one going
        // elsewhere is left alone (its `Despawn` follows), except one's own character, whose playfield stops for the zoning
        // (`StartTeleport` → `n3Playfield_t::StopPlayfield` 0x1000ccbd: every dynel in it dies; the redirect and the playfield
        // message follow). Nothing is sent back: `ToClientDynelAlive` (Gamecode 0x10015c4a) only tells the client's own modules.
        if let Some(t) = mao_net::n3::teleport(&m) {
            let id = m.identity.1;
            // (`PollStatus` 0x10029e60: only a dynel the client has)
            if t.within_playfield() && (self.entities.contains_key(&id) || id == self.own) {
                self.paths.remove(&id);
                self.rotations.insert(id, t.rotation);
                self.entities.entry(id).or_insert((t.position, 0)).0 = t.position;
                if id == self.own {
                    self.own_teleport = Some((t.position, t.rotation));
                }
            } else if !t.within_playfield() && id == self.own {
                self.playfield_stopped = Some(Instant::now());
                let own = self.own;
                self.entities.retain(|&e, _| e == own);
                self.paths.clear();
                self.fighting.retain(|&e, _| e == own);
            }
        }
        if m.kind == 0x195e496e && m.data.len() >= 12 && m.identity.1 != self.own {
            let f = |at: usize| f32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            self.paths.remove(&m.identity.1);
            if let Some(e) = self.entities.get_mut(&m.identity.1) {
                e.0 = [f(0), f(4), f(8)];
            }
        }
        // `SetWantedDirection` (`60201d0e`, aosharp's name): a character turned to face a direction (a byte, then x y z: the
        // Arete Landing capture's NPCs, `bf800000 0 0` and the like), facing along `(sin h, cos h)` as the rotations do.
        if m.kind == 0x60201d0e && m.data.len() >= 13 && m.identity.1 != self.own {
            let f = |at: usize| f32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            let (x, z) = (f(1), f(9));
            if x != 0.0 || z != 0.0 {
                let h = x.atan2(z);
                self.rotations.insert(m.identity.1, [0.0, (h / 2.0).sin(), 0.0, (h / 2.0).cos()]);
            }
        }
        // `ItemReplaced` (`3a223b50`): the item at a place of one's own becomes another (a byte, the place, then the item before
        // and after: low id, high id, quality and a word each). In the Arete Landing capture the worn starter armour (places
        // 0x11..0x1e) goes up a quality at each level.
        if m.kind == 0x3a223b50 && m.identity.1 == self.own && m.data.len() >= 37 {
            let word = |at: usize| u32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            let place = word(1);
            if let Some(e) = self.worn.iter_mut().find(|e| e.slot == place) {
                (e.low_id, e.high_id, e.quality) = (word(21), word(25), word(29));
                self.inventory = Some(self.worn.clone());
            }
        }
        // `StopMovingCmd` (`742e2314`): a character stops where it is, so the path it was walked along ends.
        if m.kind == 0x742e2314 && m.identity.1 != self.own {
            self.paths.remove(&m.identity.1);
        }
        if m.kind == OBJECT_ANNOUNCE && m.identity.0 == OBJECT_TYPE && m.data.len() >= 29 {
            // The position follows 17 bytes of header (the capture's objects stand where a
            // terminal or door of that playfield does).
            let f = |at: usize| f32::from_be_bytes(m.data[at..at + 4].try_into().unwrap());
            let at = [f(17), f(21), f(25)];
            self.objects.insert(m.identity.1, at);
            // (a corpse: announced on a body that died, within a few metres of it)
            let body = self.bodies.iter().filter(|(id, _)| !self.corpses.contains_key(id)).map(|(id, b)| (*id, b.0)).map(|(id, p)| (id, ((p[0] - at[0]).powi(2) + (p[2] - at[2]).powi(2)).sqrt())).filter(|(_, d)| *d < 4.0).min_by(|a, b| a.1.total_cmp(&b.1)).map(|b| b.0);
            if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                eprintln!("combat: object {:#x} at {at:?}; bodies {:?} -> {body:?}; named {:?}", m.identity.1, self.bodies.iter().map(|(id, b)| (*id, b.0)).collect::<Vec<_>>(), corpse_name(&m.data));
            }
            match body {
                Some(body) => {
                    // (the server names the corpse, "Remains of <name>"; the body lying there is shown by that name too)
                    self.corpses.insert(body, m.identity.1);
                    // (and it lies where the server puts the corpse, which is what the original draws: the body was left
                    // where this client had last walked it, up to some metres off and at times inside a wall or under a
                    // floor, where the remains could not be seen or clicked. The user, 2026-11-11.)
                    self.paths.remove(&body);
                    if let Some(e) = self.entities.get_mut(&body) {
                        e.0 = at;
                    }
                    if let Some(b) = self.bodies.get_mut(&body) {
                        b.0 = at;
                    }
                    let name = corpse_name(&m.data).unwrap_or_else(|| format!("Remains of {}", self.names.get(&body).map(|n| n.0.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| "the dead".into())));
                    self.names.insert(m.identity.1, (name.clone(), false));
                    self.names.insert(body, (name, false));
                }
                None => {
                    self.names.entry(m.identity.1).or_insert_with(|| (format!("Object {:x}", m.identity.1 & 0xffff), false));
                }
            }
        }
        if m.identity.0 != 0 {
            self.types.entry(m.identity.1).or_insert(m.identity.0);
        }
        // The research window's data (`research.rs`): the global research lines (`ResearchUpdate`) and the personal research
        // progress (`435f7023`, FUN_10063e88: what is left, and the goal as stat 265).
        if m.kind == 0x253d0240
            && let Some(lines) = crate::research::parse_update(&m.data)
        {
            crate::research::set_global(lines);
        }
        if m.kind == 0x435f7023
            && m.identity.1 == self.own
            && let Some((id, goal, left)) = crate::research::parse_progress(&m.data)
        {
            if self.stats.insert(265, goal as i64) != Some(goal as i64) {
                self.stats_changed = true;
            }
            crate::research::set_personal(id, left);
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
        if m.kind == mao_net::n3::ENTITY_INFO && m.identity.1 != self.own {
            let times = m.active_nano_times();
            if !times.is_empty() {
                self.other_nano_times.insert(m.identity.1, (times, Instant::now()));
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
                crate::research::entered(perk_entries(p.value.get("Perks")));
                self.perks_changed = true;
                if let Some(mao_net::schema::Value::List(ids)) = p.value.get("UploadedNanoIds") {
                    self.program_ids = ids.iter().filter_map(mao_net::schema::Value::as_int).map(|v| v as u32).collect();
                    self.programs_changed = true;
                    if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                        for &id in &self.program_ids {
                            let name = ITEM_DB.get().and_then(|db| db.get(mao_formats::resource::Id { kind: 0xfde85, instance: id }).ok().flatten()).as_deref().and_then(mao_formats::item::Item::parse).map(|i| i.name);
                            eprintln!("combat: nano program {id}: {}", name.unwrap_or_default());
                        }
                    }
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
                    // (a thing's `AnimPlay` or `AnimPos` told: its model plays from now, `prop_anim_time`)
                    if self.props.contains_key(&m.identity.1) {
                        let told = |key: i64| items.iter().filter(|i| i.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(key)).filter_map(|i| i.get("Item2")?.as_int()).last().map(|v| v as u32);
                        let (play, pos) = (told(501), told(500));
                        if play.is_some() || pos.is_some() {
                            let old = self.prop_anim.get(&m.identity.1).copied().unwrap_or((0, 0, Instant::now()));
                            self.prop_anim.insert(m.identity.1, (play.unwrap_or(old.0), pos.unwrap_or(old.1), Instant::now()));
                        }
                    }
                    let health = items.iter().filter(|i| i.get("Item1").and_then(mao_net::schema::Value::as_int) == Some(27)).filter_map(|i| i.get("Item2")?.as_int()).last();
                    if let Some(hp) = health {
                        let id = m.identity.1;
                        if let Some(old) = self.last_hp.insert(id, hp) {
                            let change = self.untold_loss(id, (hp - old).min(0));
                            if let Some(line) = crate::feedback::health(self.who(id), crate::feedback::Who::Unknown, change, 0, None) {
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
                // (MAO_COMBAT_LOG: the stats told of the character, but its health and nano)
                if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                    if let Some(mao_net::schema::Value::List(items)) = p.value.get("Stats") {
                        let told: Vec<(i64, i64)> = items.iter().filter_map(|i| Some((i.get("Item1")?.as_int()?, i.get("Item2")?.as_int()?))).filter(|t| t.0 != 27 && t.0 != 214).collect();
                        if !told.is_empty() {
                            eprintln!("own stats told: {told:?}");
                        }
                    }
                }
                self.read_stats(p.value.get("Stats"));
                for (stat, old) in before {
                    let (Some(old), Some(&new)) = (old, self.stats.get(&stat)) else { continue };
                    let change = if stat == 27 && new < old { self.untold_loss(self.own, new - old) } else { new - old };
                    if let Some(line) = crate::feedback::own_stat(stat, change) {
                        self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                    }
                }
            }
        }
        if let Some(q) = m.rotation() {
            self.rotations.insert(m.identity.1, q);
        }
        if let Some(info) = m.entity_info() {
            // (MAO_COMBAT_LOG: a creature whose update gives the client no template and no humanoid body, with its bytes)
            if info.template.is_none() && !info.is_humanoid() && std::env::var_os("MAO_COMBAT_LOG").is_some() {
                eprintln!("no template: {:#x} {:?} flags {:#x} breed {} kind {:#x}: {}", m.identity.1, info.name, info.flags, info.breed, m.kind, m.data.iter().map(|b| format!("{b:02x}")).collect::<String>());
            }
            // (the meshes the full update lists are worn from the start: hair, shoulder pads, what is on the back; an
            // appearance update replaces the list)
            if !info.meshes.is_empty() {
                self.equipment.insert(m.identity.1, info.meshes.clone());
            }
            // (the full update's flag bit 21 sets the dynel flag 0x800, FUN_1007835c through vtable +0x18)
            if info.flags & 0x20_0000 != 0 {
                self.weapon_flag.insert(m.identity.1);
            }
            self.dead.remove(&m.identity.1);
            match info.fighting {
                Some((_, target)) => {
                    self.fighting.insert(m.identity.1, target);
                }
                None => {
                    self.fighting.remove(&m.identity.1);
                }
            }
            // (a creature's health and maximum, as Gamecode's reader FUN_1007916d takes them: the pet window's bars)
            if let Some((health, max)) = info.health {
                self.health.insert(m.identity.1, (health as i64, max as i64));
            }
            let player = info.is_player();
            self.names.insert(m.identity.1, (info.name.clone(), player));
            self.infos.insert(m.identity.1, info);
        }
        if let Some((kind, q, _)) = m.movement() {
            self.rotations.insert(m.identity.1, q);
            match kind {
                mao_net::n3::movement::SIT => {
                    self.seated.insert(m.identity.1);
                }
                // (the reports and the turns keep a seated character seated: the original's turn transitions, cases 9
                // to 0xe of FUN_1006cb58, leave the movement mode alone)
                mao_net::n3::movement::FULL | 9..=14 => {}
                _ => {
                    self.stand_up(m.identity.1);
                }
            }
            // (another movement mode, 0x17..0x1d, or leaving the sneak mode, 0x24, ends the sneaking)
            if matches!(kind, 0x17..=0x1d | 0x24) && kind != 0x1c {
                self.sneaking.remove(&m.identity.1);
            } else if kind == 0x1c {
                self.sneaking.insert(m.identity.1);
            }
            if m.identity.1 != self.own {
                match kind {
                    mao_net::n3::movement::SWIM_MODE => {
                        self.swimming.insert(m.identity.1);
                    }
                    mao_net::n3::movement::LEAVE_SWIM => {
                        self.swimming.remove(&m.identity.1);
                    }
                    // (the strafes, for their animation)
                    mao_net::n3::movement::STRAFE_LEFT_START => {
                        self.strafing.insert(m.identity.1, -1);
                    }
                    mao_net::n3::movement::STRAFE_RIGHT_START => {
                        self.strafing.insert(m.identity.1, 1);
                    }
                    mao_net::n3::movement::STRAFE_LEFT_STOP | mao_net::n3::movement::STRAFE_RIGHT_STOP => {
                        self.strafing.remove(&m.identity.1);
                    }
                    _ => {}
                }
            }
            if kind == mao_net::n3::movement::JUMP {
                self.jumped.insert(m.identity.1, Instant::now());
            }
        }
        let position = m.position().or_else(|| m.movement().map(|(_, _, p)| p));
        // (a place that is no place of this world is not taken: some message read as carrying one gave numbers like 6e-39,
        // an integer's bytes, and the creature stood at the world's corner, unseen; `MAO_COMBAT_LOG=1` names the message)
        let position = position.filter(|p| {
            let sane = p.iter().all(|v| v.is_finite() && v.abs() < 1.0e6 && (*v == 0.0 || v.abs() > 1.0e-12));
            if !sane && std::env::var_os("MAO_COMBAT_LOG").is_some() {
                eprintln!("combat: message {:#x} of {:#x} carries no sane place: {p:?}", m.kind, m.identity.1);
            }
            sane
        });
        if let Some((mao_net::n3::movement::FULL, _, p)) = m.movement().filter(|_| m.identity.1 == self.own) {
            self.own_report = Some(p);
        }
        if m.identity.1 == self.own && position.is_some() && std::env::var("MAO_TRACE").is_ok_and(|v| v == "1") {
            eprintln!("own place from the server: {:#x} {:?} (movement {:?})", m.kind, position, m.movement().map(|x| x.0));
        }
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

    /// Walks the creatures the server set on a path or after a target (`paths`) for `dt` seconds; `own` is one's own place.
    fn walk_paths(&mut self, dt: f32, own: [f32; 3]) {
        let ids: Vec<u32> = self.paths.keys().copied().collect();
        for id in ids {
            if self.dead.contains_key(&id) {
                self.paths.remove(&id);
                continue;
            }
            let Some(at) = self.entities.get(&id).map(|e| e.0) else {
                self.paths.remove(&id);
                continue;
            };
            let (goal, near) = match &self.paths[&id] {
                Route::Waypoints(points) => match points.first() {
                    Some(p) => (*p, 0.1),
                    None => {
                        self.paths.remove(&id);
                        continue;
                    }
                },
                Route::Chase(t) => match if *t == self.own { Some(own) } else { self.entities.get(t).map(|e| e.0) } {
                    Some(p) => (p, CHASE_NEAR),
                    None => {
                        self.paths.remove(&id);
                        continue;
                    }
                },
            };
            let (dx, dz) = (goal[0] - at[0], goal[2] - at[2]);
            let left = (dx * dx + dz * dz).sqrt();
            if left <= near {
                if let Some(Route::Waypoints(points)) = self.paths.get_mut(&id) {
                    points.remove(0);
                }
                continue;
            }
            let speed = self.path_speeds.get(&id).copied().unwrap_or(PATH_SPEED);
            let step = (speed * dt).min(left - near);
            let to = [at[0] + dx / left * step, at[1] + (goal[1] - at[1]) * (step / left), at[2] + dz / left * step];
            if let Some(e) = self.entities.get_mut(&id) {
                e.0 = to;
            }
            self.last_move.insert(id, Instant::now());
            self.speeds.insert(id, speed);
            self.backing.insert(id, false);
            let heading = dx.atan2(dz);
            if !self.fighting.contains_key(&id) {
                self.rotations.insert(id, [0.0, (heading * 0.5).sin(), 0.0, (heading * 0.5).cos()]);
            }
        }
    }

    /// The action a character's body holds: its death's, else lying down's while sleeping (0xed `sleep-ground`, FUN_1006ec48)
    /// or lounging (0xef `lounging`, FUN_1006edf2), else sitting down's (0xd5) while seated.
    fn held_action(&self, id: u32) -> Option<u32> {
        let resting = |w| match w {
            0xf => Some(0xed),
            0x10 => Some(0xef),
            _ => None,
        };
        self.dead.get(&id).map(|d| d.0).or_else(|| self.resting.get(&id).copied().and_then(resting)).or_else(|| self.seated.contains(&id).then_some(0xd5))
    }

    /// A character stands up: no longer seated, nor sleeping or lounging.
    fn stand_up(&mut self, id: u32) {
        self.seated.remove(&id);
        self.resting.remove(&id);
    }

    /// What using `(kind, id)` uses: a dead body's corpse, else itself.
    /// An item into the first free place of the backpack (aosharp's `AddToNextAvailableSlot`).
    fn add_to_backpack(&mut self, mut entry: inventory::Entry) {
        let Some(free) = (inventory::BACKPACK_FIRST..inventory::BACKPACK_FIRST + crate::gear::PACK_SLOTS).find(|s| !self.worn.iter().any(|e| e.slot == *s)) else { return };
        if std::env::var_os("MAO_COMBAT_LOG").is_some() {
            eprintln!("item {} (QL {}) into the backpack at {free:#x}", entry.low_id, entry.quality);
        }
        entry.slot = free;
        self.worn.push(entry);
        self.inventory = Some(self.worn.clone());
    }

    /// The client's own character died of `reason` (Gamecode FUN_1005b3d8): the game's line for it, once for a death,
    /// and no health.
    fn own_died_of(&mut self, reason: i64) {
        if self.stats.insert(27, 0) != Some(0) {
            self.stats_changed = true;
        }
        if self.death_told.is_none_or(|t| t.elapsed() > Duration::from_secs(5)) {
            if let Some(line) = crate::feedback::death(reason) {
                self.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                self.death_told = Some(Instant::now());
            }
        }
    }

    fn used(&self, kind: u32, id: u32) -> (u32, u32) {
        if let Some(&corpse) = self.corpses.get(&id).filter(|c| self.objects.contains_key(c)) {
            return (OBJECT_TYPE, corpse);
        }
        // (a table object in an instance goes by the instance's id: its place among the table's records of its kind, in
        // the table's order, counted through the playfield message's runs of that kind. Not the number in its own id,
        // `0xc0NNpppp`: where the numbers have a gap the two differ. Arete Landing's machines are numbered 0 to 4 and 6 to
        // 8, so the ammunition machine, number 6, is the sixth record and `first + 5`; taken as `first + 6` the use went
        // to a machine 90 m off and the server answered that it was too far away. The original's capture of 2026-10-30
        // has the machines' announcements with their places to check this by.)
        if id >> 24 == 0xc0 {
            if let Some(nth) = self.table.iter().filter(|d| d.0 == kind).position(|d| d.1 == id) {
                if let Some(instance) = instance_id(&self.instance_runs, kind, nth as u32) {
                    return (kind, instance);
                }
            }
        }
        (kind, id)
    }

    fn print(&self) {
        println!("N3 messages by type:");
        for (kind, n) in &self.kinds {
            println!("  {kind:#010x} x{n}");
        }
        for (id, i) in &self.infos {
            println!("  info {id:>10} {:<18} side {} flags {:#010x} flags2 {:#010x} breed {} sex {} a0 {:?} template {:?} textures {} clothes {:?}", i.name, i.side, i.flags, i.flags2, i.breed, i.sex, i.unknown_a0, i.template.map(|t| format!("{t:#x}")), i.textures.len(), i.clothes);
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


#[cfg(test)]
mod instance_tests {
    /// ICC's terminals (1186): the clinic and the bank (terminals 0 and 1) in the first run, the second clinic in the next.
    #[test]
    fn icc_terminals_get_the_instance_ids() {
        let runs = [(0xc75b, 0, 15, 0x1324f4e2), (0xc73d, 0x36, 2, 0x58c197d5), (0xc748, 0x38, 3, 0x10c1c17c), (0xc73d, 0x3b, 1, 0x58c197d7)];
        assert_eq!(super::instance_id(&runs, 0xc73d, 1), Some(0x58c197d6));
        assert_eq!(super::instance_id(&runs, 0xc73d, 2), Some(0x58c197d7));
        assert_eq!(super::instance_id(&runs, 0xc73d, 3), None);
        assert_eq!(super::instance_id(&runs, 0xc75b, 14), Some(0x1324f4f0));
    }
}

/// The instance's id of the `number`th table record of a kind: the runs of that kind cover its records in order.
fn instance_id(runs: &[(u32, u32, u32, u32)], kind: u32, number: u32) -> Option<u32> {
    let mut before = 0;
    for &(_, _, count, first) in runs.iter().filter(|r| r.0 == kind) {
        if number < before + count {
            return Some(first + (number - before));
        }
        before += count;
    }
    None
}

/// A flat floor under an interior whose own floor this client can't read (its ground is of the unread tile-map
/// layout): a square of 200 units around (`x`, `z`) at `height`, drawn from both sides.
fn floor_part(x: f32, z: f32, height: f32) -> Part {
    let corners = [(-100.0, -100.0), (100.0, -100.0), (100.0, 100.0), (-100.0, 100.0)];
    let vertices = corners.iter().map(|(dx, dz)| Vertex { position: [x + dx, height, z + dz], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], colour: Vertex::UNBAKED }).collect();
    let colour = [0.42, 0.40, 0.38];
    let material = Material { name: "floor".into(), diffuse: colour, specular: [0.0; 3], ambient: colour, emissive: [0.0; 3], opacity: 1.0, textures: Vec::new(), ignore_alpha: false, alpha_lights: false, foliage: false, two_sided: true, ..Default::default() };
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
            vertices.push(Vertex { position: c, normal, uv: [0.0, 0.0], colour: Vertex::UNBAKED });
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let material = Material { name: "marker".into(), diffuse: colour, specular: [0.0; 3], ambient: colour, emissive: [0.0; 3], opacity: 1.0, textures: Vec::new(), ignore_alpha: false, alpha_lights: false, foliage: false, two_sided: true, ..Default::default() };
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
    /// Machine models by mesh instance, with each part's animation.
    props: std::collections::HashMap<u32, Option<(Vec<Part>, Vec<Option<mao_formats::model::Animated>>)>>,
    /// Worn static meshes (hair, helmets, weapons, shoulder pads) by instance.
    statics: std::collections::HashMap<u32, Option<Model>>,
    /// The player head meshes (`head_solitusmale12.abiff` and so on): an appearance update that names one at position 0 replaces
    /// the breed's default head.
    head_meshes: std::collections::HashSet<u32>,
    /// The body's build (0 small, 1 medium, 2 large: GUI.dll's `CCCharacter_t::ChangeMesh` names the bodies
    /// `<breed>_<sex>_thin.cir` and `_fat.cir`), where such a body exists; medium for the world's characters.
    build: i32,
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
    // (a mesh in the head's place is held where the head is: `n3::HEAD_REPLACED`)
    format!("Attractor{:02}", (position & !mao_net::n3::HEAD_REPLACED) as u32 + 1)
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

/// Swimming's speed (`CharVehicle_t` in mode 4, FUN_1006f9eb): `RunSpeed / 440 + 3` (`_DAT_10161ab0`, `_DAT_1015e6dc`) within
/// 1.5 and 8 (`_DAT_1015e7ac`, `_DAT_10161a78`), the run speed falling off at low health as running's does.
pub(crate) fn swim_speed(stats: &BTreeMap<u32, i64>) -> f32 {
    let stat = |n: u32| stats.get(&n).copied().unwrap_or(0) as f32;
    let (run_speed, health, max_health) = (stat(156), stat(27), stat(1));
    let ratio = if max_health > 0.0 { health / (max_health * 0.15) } else { 1.0 };
    let effective = if ratio < 1.0 { ratio * (run_speed + 1000.0) - 1000.0 } else { run_speed };
    (effective * 0.002_272_727_3 + 3.0).clamp(1.5, 8.0)
}

/// How near a door's place a report starts a teleport try (see the touch test).
const DOOR_REACH: f32 = 0.5;

/// The vehicle's liquid test (`Vehicle_t::EnsureSurfaceAlignment`): the feet within this of the surface count as in it, the
/// water must be deeper than this to swim, and a swimmer's feet are held this far under the surface.
const SWIM_ABOVE: f32 = 0.1;
const SWIM_DEPTH: f32 = 1.19;
const SWIM_SINK: f32 = 0.01;

/// How far a flying vehicle's body is lifted as flight starts (FUN_1006e4f8's `SetRelPos` by `_DAT_1015e0e4`).
const FLY_LIFT: f32 = 0.5;

/// Walking's speed (`CharVehicle_t` in mode 2, FUN_1006f9eb: `_DAT_1015e7ac`). Sneaking (mode 6) takes the same: FUN_1006f9eb has
/// no case of its own for it and falls to walking's (`LAB_1006fd78`), forwards and backwards alike.
const WALK_SPEED: f32 = 1.5;

/// A character's movement mode as `CharVehicle_t` keeps it (FUN_10070a2f): walking (2), running (3), swimming (4), sneaking (6).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Walk,
    Run,
    Swim,
    Sneak,
}

/// The movement animation of a character (FUN_1006c370), `None` standing in a plain mode: swimming plays `swim` or `idle-swim`;
/// in the walking, running and sneaking modes a sidestep its `walk-left`/`walk-right` (0x86, 0x87); sneaking plays the sneak
/// action (0x66) moving, backwards too (the original plays it reversed), and standing; else backing the back animations and
/// moving the walk or the run.
fn movement_action(mode: Mode, moving: bool, strafe: i8, backing: bool) -> Option<mao_formats::appearance::Action> {
    use mao_formats::appearance::Action;
    match strafe {
        _ if mode == Mode::Swim => Some(if moving { Action::Swim } else { Action::IdleSwim }),
        -1 if moving => Some(Action::StrafeLeft),
        1 if moving => Some(Action::StrafeRight),
        _ if mode == Mode::Sneak => Some(if moving { Action::Sneak } else { Action::SneakStand }),
        _ if !moving => None,
        _ if backing => Some(if mode == Mode::Run { Action::RunBack } else { Action::WalkBack }),
        _ => Some(if mode == Mode::Run { Action::Run } else { Action::Walk }),
    }
}

/// One step of the character's vehicle (`Vehicle_t::Run`'s FUN_1000e3d3 with `CharVehicle_t`'s steering FUN_10071537): the
/// horizontal velocity after `dt` seconds pushed along the facing (`walk` 1 forwards, −1 back) with the force
/// `2 × mass × top`, kept within `top`; none pushed, or the push turned round since the last step (`pushed`), halts it.
fn vehicle_step(velocity: [f32; 2], heading: f32, walk: i8, pushed: i8, top: f32, dt: f32) -> [f32; 2] {
    if walk == 0 || walk != pushed {
        // (`Vehicle_t::Halt`, or `SetDirection` turning round; a start from rest pushes from nothing)
        if walk == 0 {
            return [0.0; 2];
        }
        return vehicle_step([0.0; 2], heading, walk, walk, top, dt);
    }
    let accel = 2.0 * top * walk as f32;
    let mut v = [velocity[0] + heading.sin() * accel * dt, velocity[1] + heading.cos() * accel * dt];
    let speed = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if speed > top {
        v = [v[0] * top / speed, v[1] * top / speed];
    }
    v
}

/// The movement keys of a seated character: forward, back, the sidesteps and the jump do nothing (it neither moves nor
/// stands up for them; the turn keys are left as they are).
fn seated_keys(_forward: bool, _back: bool, _strafe: i8, _jump: bool) -> (bool, bool, i8, bool) {
    (false, false, 0, false)
}

/// How fast the character turns by key (radians a second), as FUN_1006ca4f sets `CharVehicle_t`'s +0x368: `TurnSpeed`
/// (stat 267) / 11000, halved while moving; without the stat 3.5 standing and 1.5 moving.
fn turn_rate(stats: &BTreeMap<u32, i64>, moving: bool) -> f32 {
    match stats.get(&267).copied().unwrap_or(0) {
        0 if moving => 1.5,
        0 => 3.5,
        s if moving => s as f32 / 11000.0 * 0.5,
        s => s as f32 / 11000.0,
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
        Ok(Characters { head_meshes, build: 1, db, index, anims: Default::default(), ids: Default::default(), bodies: Default::default(), creatures: Default::default(), creature_anims: Default::default(), motions: Default::default(), segment: 0, stacks: Default::default(), anims_by_id: Default::default(), notes: Vec::new(), movement_keys: Default::default(), movement_handles: Default::default(), socials_started: Default::default(), swings_started: Default::default(), drawn: Default::default(), wielded: Default::default(), started: Instant::now(), props: Default::default(), statics: Default::default(), vehicles: Default::default(), frames: Default::default() })
    }

    /// The parts of a machine's model (Y-up local space), loaded once.
    fn prop_parts(&mut self, mesh: u32) -> Option<&(Vec<Part>, Vec<Option<mao_formats::model::Animated>>)> {
        let db = self.db.clone();
        self.props
            .entry(mesh)
            .or_insert_with(|| {
                crate::load_model_dynel_animated(&db, mao_formats::resource::Id { kind: 0xf6951, instance: mesh }).ok().map(|(m, a)| {
                    // (only the animations that move anything are kept)
                    let a = a.into_iter().map(|a| a.filter(|a| a.moves())).collect();
                    (m.parts, a)
                })
            })
            .as_ref()
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
        let custom_head = gear.iter().any(|g| g.0 == mao_net::n3::HEAD_REPLACED || (g.0 == 0 && self.head_meshes.contains(&g.1)));
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
        let build = self.build;
        self.bodies.entry((breed, sex)).or_insert_with(|| {
            let (body_name, head_name) = mao_formats::appearance::player_meshes(breed, sex)?;
            let built = match build {
                0 => index.find(0xf6952, &body_name.replace(".cir", "_thin.cir")),
                2 => index.find(0xf6952, &body_name.replace(".cir", "_fat.cir")),
                _ => None,
            };
            let body = mao_formats::catmesh::parse(&db.get(Id { kind: 0xf6952, instance: built.or_else(|| index.find(0xf6952, body_name))? }).ok()??).ok()?;
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
        self.file_loop_points(&name)
    }

    /// A loaded animation file's cycle: its `loopstart` and `loopend` notes and its length (the whole of it without
    /// them), as FUN_10010929's modes take them of whatever animation is set.
    fn file_loop_points(&self, name: &str) -> Option<(f32, f32, f32)> {
        let (anim, length) = self.anims.get(name)?.as_ref()?;
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
        // (as the original: `ForwardStartTransitionAction_t` (FUN_1006ef8d) plays the walk's or run's loop, segment 2, from the
        // first step, and the stop (FUN_1006f0ac → FUN_1006c5ae → FUN_1006c370) goes straight to the idle's loop; neither the
        // start before `loopstart` nor the tail after `loopend` is played, and the mesh's own blending eases the change)
        match (st.stage, moving) {
            (Stage::Idle | Stage::Stop, Some(a)) => *st = Motion { stage: Stage::Loop, action: a, clock: 0.0, tick: now },
            (Stage::Start | Stage::Loop, Some(a)) if a != st.action => *st = Motion { stage: Stage::Loop, action: a, clock: 0.0, tick: now },
            (Stage::Start | Stage::Loop, None) => *st = Motion { stage: Stage::Idle, action: Idle, clock: 0.0, tick: now },
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
        let custom_head = gear.iter().any(|g| g.0 == mao_net::n3::HEAD_REPLACED || (g.0 == 0 && self.head_meshes.contains(&g.1)));
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
        // (a weapon's own movement has its cycle between its own notes, as the plain one has: a rifle's `run-2h` is four
        // seconds of a start, the strides and a stop with the cycle from 1166 to 1966 ms. Looped whole, as it was, the
        // runner started, ran a few strides and stopped, over and over. An idle has no notes and loops whole.)
        let points = match (&weapon_idle, movement) {
            (Some((file, _)), Some((_, length))) => self.file_loop_points(file).unwrap_or((0.0, length, length)),
            _ => self.loop_points(breed, sex, action).unwrap_or((0.0, 0.0, 0.0)),
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
    character_model_shaped(db, breed, sex, look, 1, 100.0)
}

/// [`character_model`] with a build (0 small, 1 medium, 2 large) and a height in percent (character creation's preview).
pub(crate) fn character_model_shaped(db: &Arc<ResourceDatabase>, breed: i32, sex: i32, look: Option<&crate::look::Look>, build: i32, height: f32) -> Option<Model> {
    // (the Atrox has one body, whatever the list says)
    let (breed, sex) = (breed as u8, if breed == 4 { 1 } else { sex as u8 });
    let mut chars = Characters::new(db.clone()).ok()?;
    chars.build = build;
    let (textures, clothes, gear) = look.map_or((&[][..], &[][..], &[][..]), |l| (l.textures.as_slice(), l.clothes.as_slice(), l.gear.as_slice()));
    let parts = chars.parts(0, breed, sex, None, None, 0.0, 0.0, height, textures, clothes, gear, None, None, None, false, None, None, None)?;
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

/// The NPC template everyone is drawn with in the Grid (FUN_10058088's 0x1863e: a blue triangle, `Features` bit 0x800).
const GRID_TEMPLATE: u32 = 0x1863e;

/// `PlayfieldAnarchy_t::IsGrid` (0x101222d7): the playfield's first area is named "Grid".
fn is_grid(areas: &[mao_formats::areas::Area]) -> bool {
    areas.first().is_some_and(|a| a.name.eq_ignore_ascii_case("Grid"))
}

/// The entities' markers alone (own character red, the others blue): two
/// parts and an instance per entity, placed Z-up.
fn overlay_model(world: &World, props: &BTreeMap<u32, (u32, [f32; 3], [f32; 4])>, characters: &mut Option<Characters>, include: &dyn Fn(u32) -> bool, statels: bool) -> (Model, Vec<crate::blobshadow::Caster>, Vec<(u32, std::ops::Range<usize>)>) {
    let mut model = Model { parts: vec![marker([0.9, 0.1, 0.1]), marker([0.2, 0.4, 0.95]), nose(), marker([0.2, 0.8, 0.3])], instances: Vec::new(), water: None, lod_groups: Vec::new() };
    // (the characters' meshes throw the original's blob shadows: each is noted as a caster)
    let mut casters = Vec::new();
    // (which parts are which character's, for the effects that tint a body: `Update::character_parts`)
    let mut ranges = Vec::new();
    // (`frames`: the body's attractors in its local space, carried by the caster in the server's frame for the effects on them)
    fn push_character(model: &mut Model, casters: &mut Vec<crate::blobshadow::Caster>, ranges: &mut Vec<(u32, std::ops::Range<usize>)>, id: u32, parts: Vec<mao_formats::model::Part>, place: mao_formats::model::Matrix, own: bool, frames: Option<&Vec<(String, mao_formats::model::Matrix)>>) {
        let mut caster = crate::blobshadow::Caster::new(&parts, &place, own);
        caster.id = id;
        let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        caster.attractors = frames.map_or(Vec::new(), |f| f.iter().map(|(name, m)| (name.clone(), model::multiply(&model::multiply(m, &place), &swap))).collect());
        casters.push(caster);
        let base = model.parts.len();
        for (k, p) in parts.into_iter().enumerate() {
            model.parts.push(p);
            model.instances.push((base + k, place));
        }
        ranges.push((id, base..model.parts.len()));
    }
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    for (&id, &(position, _)) in &world.entities {
        if !include(id) {
            continue;
        }
        let q = world.rotations.get(&id).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let place = model::multiply(&model::trs_matrix(position, q, [1.0; 3]), &swap);
        let deform = world.deforms.get(&id).map(|(d, at)| (d, at.elapsed().as_secs_f32()));
        if let (Some(info), Some(chars)) = (world.infos.get(&id).filter(|i| i.template.is_some() && !world.grid), characters.as_mut()) {
            let moving = world.last_move.get(&id).is_some_and(|t| t.elapsed() < Duration::from_millis(900));
            let action = if moving { mao_formats::appearance::Action::Walk } else { mao_formats::appearance::Action::Idle };
            // (an attack swings the weapon the server said the creature holds in the place it names)
            let swing = world.swings.get(&id).and_then(|&(slot, began, pick, class)| Some((*world.weapons.get(&(id, slot as u8))?, slot, began, pick, class)));
            if let Some(parts) = chars.creature_parts(id, info.template.unwrap(), action, (id % 997) as f32 * 3.0, 1.0, &info.textures, &info.clothes, deform, world.held_action(id), swing) {
                // (the creature at the size its update gives it, a percentage of its template's: the original's dynel
                // takes its `Scale` stat as the body's scale, `SetBodyScale`, Gamecode FUN_100892ca; the bosses of Arete
                // Landing are ordinary creatures at 600)
                let place = match info.scale.filter(|&s| s != 100 && s != 0) {
                    Some(s) => {
                        let k = s as f32 / 100.0;
                        model::multiply(&[k, 0.0, 0.0, 0.0, 0.0, k, 0.0, 0.0, 0.0, 0.0, k, 0.0, 0.0, 0.0, 0.0, 1.0], &place)
                    }
                    None => place,
                };
                push_character(&mut model, &mut casters, &mut ranges, id, parts, place, id == world.own, chars.frames.get(&id));
                continue;
            }
        }
        // (a polymorphed character is drawn as the creature it became; in the Grid everyone is its triangle: the stat getter
        // FUN_10059399 sets `Features` bit 0x800 there and FUN_100524b2 then takes FUN_10058088's template)
        let creature = if world.grid { Some(GRID_TEMPLATE) } else { world.morphs.get(&id).copied() };
        if let (Some(creature), Some(chars)) = (creature, characters.as_mut()) {
            let moving = if id == world.own { world.own_moving } else { world.last_move.get(&id).is_some_and(|t| t.elapsed() < Duration::from_millis(700)) };
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
            if let Some(parts) = chars.creature_parts(id, creature, action, (id % 997) as f32 * 3.0, MORPH_ANIM_RATE, &[], &[], deform, world.held_action(id), None) {
                push_character(&mut model, &mut casters, &mut ranges, id, parts, place, id == world.own, chars.frames.get(&id));
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
            let moving = if id == world.own { world.own_moving } else { world.last_move.get(&id).is_some_and(|t| t.elapsed() < Duration::from_millis(700)) };
            let speed = world.speeds.get(&id).copied().unwrap_or(0.0);
            // The original's choice (FUN_1006c370): a sidestep plays walk-left/right, moving against the facing the back
            // animations; walking is 1.5 units a second and running from 3 (backwards) or 5, so 2.2 tells them apart.
            // (one's own character runs or walks by its mode, as the original's FUN_1006c370 does; the others by their speed)
            let running = if id == world.own { !world.own_walking } else { speed > 2.2 };
            let mode = if world.swimming.contains(&id) {
                Mode::Swim
            } else if world.sneaking.contains(&id) {
                Mode::Sneak
            } else if running {
                Mode::Run
            } else {
                Mode::Walk
            };
            let action = movement_action(mode, moving, world.strafing.get(&id).copied().unwrap_or(0), world.backing.get(&id).copied().unwrap_or(false));
            let scale = if id == world.own { world.stats.get(&360).copied().filter(|&s| s > 0).unwrap_or(100) as f32 } else { 100.0 };
            let jumped = world.jumped.get(&id).map(|t| t.elapsed());
            // (A phase per entity keeps a crowd from moving in step.)
            // (a character running a vehicle program rides its board or bike)
            let vehicle = world.riding.get(&id).and_then(|&nano| chars.vehicle(nano, info.breed, info.sex));
            // (a wielded weapon hangs on its hand, in a fight or out of one: Gamecode's FUN_1009cda2, when the weapon is
            // wielded, adds the mesh its stat 209 names (not 12, which is the box an item lies on the ground as) to the
            // character's mesh at attractor place 1 for wear place 6 and 2 for place 8 (`AddAttractorMesh`), and takes it
            // off when the weapon is put away. A weapon without the stat hangs nothing. Not ported: the other side's
            // mirroring for a left-hand weapon with the can-flag 0x800000, the social places 0x3d and 0x3f. Not where
            // the worn meshes the server lists already have something on that hand.)
            let mut gear: Vec<(u8, u32, u32)> = world.equipment.get(&id).cloned().unwrap_or_default();
            for (slot, at) in [(6u32, 1u8), (8, 2)] {
                let template = world.weapons.get(&(id, slot as u8)).copied().or_else(|| (id == world.own).then(|| world.inventory.as_ref()?.iter().find(|e| e.slot == slot).map(|e| e.low_id)).flatten());
                if let Some(mesh) = template.and_then(|t| template_stat(t, 209)).filter(|&m| m > 0) {
                    if !gear.iter().any(|g| g.0 == at) {
                        gear.push((at, mesh as u32, 0));
                    }
                }
            }
            if let Some(parts) = chars.parts(id, info.breed, info.sex, action, jumped, (id % 997) as f32 * 3.0, speed, scale, &info.textures, &info.clothes, &gear, social, swing, weapon, fighting, world.held_action(id), deform, vehicle) {
                push_character(&mut model, &mut casters, &mut ranges, id, parts, place, id == world.own, chars.frames.get(&id));
                continue;
            }
        }
        let player = world.names.get(&id).is_some_and(|n| n.1);
        // (MAO_COMBAT_LOG: who is drawn as a marker for want of a model, once each)
        if std::env::var_os("MAO_COMBAT_LOG").is_some() {
            static TOLD: std::sync::Mutex<std::collections::BTreeSet<u32>> = std::sync::Mutex::new(std::collections::BTreeSet::new());
            if TOLD.lock().is_ok_and(|mut t| t.insert(id)) {
                let info = world.infos.get(&id);
                eprintln!("no model: {id:#x} {:?} template {:x?} breed {:?} humanoid {:?} flags {:x?}", world.names.get(&id).map(|n| n.0.as_str()), info.and_then(|i| i.template), info.map(|i| i.breed), info.map(|i| i.is_humanoid()), info.map(|i| i.flags));
            }
        }
        model.instances.push((if id == world.own { 0 } else if player { 3 } else { 1 }, place));
        model.instances.push((2, place));
    }
    // The machines (shop booths): one copy of each model's parts, an instance of each per machine.
    if let Some(chars) = characters.as_mut() {
        let mut copies: std::collections::HashMap<(u32, Option<u32>), Option<std::ops::Range<usize>>> = std::collections::HashMap::new();
        // (the doors that were used and have not closed again: a door with an animation swings, one without is not drawn)
        let mut door_of: BTreeMap<u32, u8> = world.door_slots.iter().map(|(&n, s)| (s.0, n)).collect();
        // (the table's own door props, whose slot an announced door may have taken: `door_props`)
        for (&key, &n) in &world.door_props {
            door_of.entry(key).or_insert(n);
        }
        let now = Instant::now();
        let open: std::collections::HashSet<u32> = door_of.iter().filter(|(_, n)| world.doors_open.get(n).is_some_and(|until| *until > now)).map(|(id, _)| *id).collect();
        for (&id, &(mesh, position, q)) in props {
            if !include(id) {
                continue;
            }
            let skin = world.prop_textures.get(&id).copied();
            let animated = chars.prop_parts(mesh).is_some_and(|(_, a)| a.iter().any(Option::is_some));
            if open.contains(&id) && !animated {
                continue;
            }
            // (a door's time is its state's; another thing's what its `AnimPlay` and `AnimPos` say, and without them it
            // stands in its first pose like a thing that has no animation)
            let length = if animated { chars.prop_parts(mesh).map_or(0.0, |(_, a)| a.iter().flatten().map(|a| a.length()).fold(0.0, f32::max)) } else { 0.0 };
            let time = match door_of.get(&id) {
                _ if !animated => None,
                Some(&n) => Some(world.door_time(n, length)),
                None => world.prop_anim.get(&id).and_then(|&(play, pos, since)| prop_anim_time(play, pos, since.elapsed().as_secs_f32(), animation_clock(), length)),
            };
            if let Some(time) = time {
                // (a moving machine or door: its own copy of the parts at its time)
                let Some((parts, anims)) = chars.prop_parts(mesh) else { continue };
                let place = model::multiply(&model::trs_matrix(position, q, [1.0; 3]), &swap);
                for (p, a) in parts.iter().zip(anims) {
                    let mut p = p.clone();
                    if let Some(a) = a {
                        let (w, visible) = a.world_at(time);
                        if !visible {
                            continue;
                        }
                        p.world = w;
                        if let Some(t) = a.texture_at(time) {
                            shift_uv(&mut p, t);
                        }
                    }
                    outer_texture(&mut p, skin);
                    model.parts.push(p);
                    model.instances.push((model.parts.len() - 1, place));
                }
                continue;
            }
            let range = copies
                .entry((mesh, skin))
                .or_insert_with(|| {
                    let (parts, _) = chars.prop_parts(mesh)?;
                    let start = model.parts.len();
                    model.parts.extend(parts.iter().cloned().map(|mut p| {
                        outer_texture(&mut p, skin);
                        p
                    }));
                    Some(start..model.parts.len())
                })
                .clone();
            let Some(range) = range else { continue };
            let place = model::multiply(&model::trs_matrix(position, q, [1.0; 3]), &swap);
            model.instances.extend(range.map(|i| (i, place)));
        }
    }
    // The statels' moving parts near the character, at the clock's time.
    if statels {
        let time = animation_clock();
        let own = world.position_of(world.own);
        for a in world.animated_statels.iter() {
            let (part, anim) = &*a.part;
            // (Z-up place: x, z, y)
            if let Some(o) = own {
                let (dx, dz) = (a.place[12] - o[0], a.place[13] - o[2]);
                if dx * dx + dz * dz > STATEL_ANIMATION_RANGE * STATEL_ANIMATION_RANGE {
                    continue;
                }
            }
            let (w, visible) = anim.world_at(time);
            if !visible {
                continue;
            }
            let mut p = part.clone();
            p.world = w;
            if let Some(t) = anim.texture_at(time) {
                shift_uv(&mut p, t);
            }
            model.parts.push(p);
            model.instances.push((model.parts.len() - 1, a.place));
        }
    }
    (model, casters, ranges)
}

/// How far from the character the statels' moving parts are drawn.
const STATEL_ANIMATION_RANGE: f32 = 250.0;

/// The time a thing's model is shown at, or `None` for its first pose, as Gamecode steps it each frame (FUN_10088970,
/// called from the things' frame step FUN_10089f0a) from its `AnimPlay` (stat 501): 2 goes forward for ever and 4 backward
/// (the mesh's loop wraps the time); 3 goes forward once and stays at the end, 5 backward once and stays at the start (the
/// original then sets `AnimPlay` to 1; opening a thing is `AnimPos` 1 with `AnimPlay` 3, closing it `AnimPos` 2 with
/// `AnimPlay` 5: FUN_1008563d, FUN_10085684); any other value leaves the time alone: at the start, or at the end with
/// `AnimPos` (stat 500) 2 (FUN_100877ec). `elapsed`: the seconds since the stats were told; `clock`: the looping time.
pub(crate) fn prop_anim_time(play: u32, pos: u32, elapsed: f32, clock: f32, length: f32) -> Option<f32> {
    if length <= 0.0 {
        return None;
    }
    match play {
        2 => Some(clock),
        4 => Some(length - clock % length),
        3 => Some(elapsed.min(length)),
        5 => Some((length - elapsed).max(0.0)),
        _ => (pos == 2).then_some(length),
    }
}

/// The seconds the looping animations (the statels', the machines') are at.
fn animation_clock() -> f32 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f32()
}

/// A part's texture coordinates through its animation's texture transform (randy31's `rasterize`: the texture matrix
/// `u * a + c`, `v * b + d` on stages 0 and 1).
fn shift_uv(p: &mut Part, [a, b, c, d]: [f32; 4]) {
    for v in &mut p.vertices {
        v.uv = [v.uv[0] * a + c, v.uv[1] * b + d];
    }
}

/// `ChangeOuterTexture`: DisplaySystem's FUN_10071064 puts the outer request's texture on each material whose name has
/// neither "face" nor "under" in it (FUN_10070f1f).
fn outer_texture(p: &mut Part, skin: Option<u32>) {
    if let Some(t) = skin.filter(|_| !p.material.name.contains("face") && !p.material.name.contains("under")) {
        for tex in &mut p.material.textures {
            *tex = (0xf6954, t);
        }
    }
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
        let collision = surface_collision(&db, playfield).unwrap_or_else(|| build_collision(&scene, ground_parts, &crate::Textures::new(db.clone()), &passable_parts(&db, &part_mesh)));
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
    let (overlay, _, _) = overlay_model(world, &world_props, &mut characters, &|_| true, true);
    let offset = scene.parts.len();
    scene.parts.extend(overlay.parts);
    scene.instances.extend(overlay.instances.into_iter().map(|(i, m)| (i + offset, m)));
    let textures = crate::Textures::new(db);
    match image {
        Some(out) => crate::viewer::headless(&scene, &textures, out),
        None => crate::viewer::window(scene, textures, None),
    }
}

/// What blocks a walking character as the original has it: the playfield's zones' collision surfaces (database type
/// 0xf424d, the zone's id the instance, which `n3Zone_t::LoadSurface` 0x1001a947 loads into each cell). None when the
/// playfield has none (a generated dungeon) or `MAO_VISUAL_COLLISION=1` asks for the props' meshes instead.
fn surface_collision(db: &ResourceDatabase, playfield: u32) -> Option<mao_formats::collision::Collision> {
    if std::env::var("MAO_VISUAL_COLLISION").is_ok_and(|v| v == "1") {
        return None;
    }
    zone_surfaces(db, playfield).0
}

/// The playfield's zones' surfaces: the collision built from their triangles (none without any surface) and their ways out
/// (the teleportals, `n3SurfaceResource_t::GetTeleportalArea`).
fn zone_surfaces(db: &ResourceDatabase, playfield: u32) -> (Option<mao_formats::collision::Collision>, Vec<mao_formats::surface::Teleportal>, Option<Arc<mao_formats::zones::Zones>>) {
    let mut teleportals = Vec::new();
    let mut collision = mao_formats::collision::Collision::default();
    let mut records = mao_formats::zones::Zones::default();
    let (mut zones, mut failed) = (0, 0);
    let Ok(ids) = db.ids() else { return (None, teleportals, None) };
    // (a generated building: each room's surfaces are its template room's zone's, moved from the template's layout to the room)
    let generated = crate::generated_rooms(db, playfield).zip(crate::template_rooms(db, playfield));
    let ids: Vec<mao_formats::resource::Id> = ids.flatten().filter(|i| i.kind == mao_formats::surface::KIND && i.instance >> 16 == playfield).collect();
    let copies: Vec<(mao_formats::resource::Id, Option<(mao_formats::rooms::Room, mao_formats::rooms::Room)>)> = match &generated {
        Some(((rooms, sources), template)) => rooms
            .rooms
            .iter()
            .zip(sources)
            .filter_map(|(room, &source)| {
                let id = *ids.iter().find(|i| (i.instance & 0xffff) as usize == source)?;
                Some((id, Some((template.rooms.get(source)?.clone(), room.clone()))))
            })
            .collect(),
        None => ids.into_iter().map(|i| (i, None)).collect(),
    };
    for (id, moved) in copies {
        let Some(mut surface) = db.get(id).ok().flatten().and_then(|o| mao_formats::surface::parse(&o)) else {
            failed += 1;
            continue;
        };
        if let Some((from, to)) = &moved {
            for t in &mut surface.triangles {
                for v in t.iter_mut() {
                    *v = mao_formats::rooms::move_between(from, to, *v);
                }
            }
            surface.teleportal = None;
        }
        zones += 1;
        teleportals.extend(surface.teleportal);
        // (as they are for the vehicle; the game's y up, the viewer's z, for the old tests)
        records.add(&surface.triangles);
        for t in surface.triangles {
            collision.add(t.map(|v| [v[0], v[2], v[1]]));
        }
    }
    if failed > 0 {
        eprintln!("playfield {playfield}: {failed} collision surfaces could not be read");
    }
    // (an interior's floors are its rooms' tile-map heights, not in the surfaces: `n3RoomSurface_t`)
    if zones > 0
        && let Some((floors, cells)) = crate::room_floors(db, playfield)
    {
        records.add(&floors);
        records.add_rooms(&cells);
    }
    ((zones > 0).then_some(collision), teleportals, (zones > 0).then(|| Arc::new(records)))
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
    /// The zones' ways out (standing in one starts a teleport try).
    pub(crate) teleportals: Vec<mao_formats::surface::Teleportal>,
    /// The zones' surfaces for the vehicle (none without any).
    pub(crate) zones: Option<Arc<mao_formats::zones::Zones>>,
    /// The statels' moving parts, drawn each frame.
    pub(crate) animated: Vec<crate::AnimatedPart>,
    /// An interior's rooms (their doorways and which parts are each room's), when the playfield is made of rooms.
    pub(crate) interior: Option<Arc<crate::interior::Interior>>,
    /// The feet of the waterfalls (x, height, z), for the fall effect (`liquid::fall_effects`).
    pub(crate) falls: Vec<[f32; 3]>,
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
    let mut animated = Vec::new();
    let part_mesh = crate::add_props_with(game, db, playfield, &mut scene, Some(&mut animated));
    profile!("[profile] area {playfield}: props {:?} ({} parts, {} moving)", started.elapsed(), scene.parts.len(), animated.len());
    if scene.instances.is_empty() {
        scene.instances.extend((0..ground_parts).map(|i| (i, model::IDENTITY)));
    }
    // An interior made of rooms (a shop instance): their floors and walls.
    let mut interior = None;
    let ground_parts = if terrain.is_none() {
        let before = scene.parts.len();
        if let Some((shell, mut rooms)) = crate::load_room_shell_parts(db, playfield) {
            let base = scene.parts.len();
            scene.instances.extend((0..shell.parts.len()).map(|i| (base + i, model::IDENTITY)));
            scene.parts.extend(shell.parts);
            for range in &mut rooms.parts {
                *range = range.start + base..range.end + base;
            }
            interior = Some(Arc::new(rooms));
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
    let (surfaces, teleportals, zones) = zone_surfaces(db, playfield);
    let surfaces = surfaces.filter(|_| !std::env::var("MAO_VISUAL_COLLISION").is_ok_and(|v| v == "1"));
    let collision = match surfaces {
        Some(c) => {
            if std::env::var("MAO_COMPARE_COLLISION").is_ok() {
                let visual = build_collision(&scene, ground_parts, textures, &passable);
                eprintln!("collision: surfaces {} triangles {:?}; visual meshes {} triangles {:?}", c.len(), c.bounds(), visual.len(), visual.bounds());
            }
            c
        }
        None => build_collision(&scene, ground_parts, textures, &passable),
    };
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
    let liquid = anarchy.as_ref().map(|a| std::sync::Arc::new(crate::liquid::Liquid::new(&a.water)));
    crate::liquid::set_current(liquid.clone());
    // (the feet of its waterfalls, for the fall effect: `liquid::fall_effects` over the playfield's surface as the vehicle has it,
    // or without surfaces the tile-map ground held 1.2 under the liquid)
    let ground = zones.clone().map(|z| crate::vehicle::Ground { zones: z, terrain: terrain.as_ref().map(|t| (t.clone(), t.height_scale)), offset: 0.0 });
    let closest = |p: [f32; 3]| -> Option<f32> {
        match &ground {
            Some(g) => {
                let mut q = p;
                if g.veto(&mut q, None) {
                    return None;
                }
                Some(g.closest(p).0[1])
            }
            None => {
                let h = terrain.as_ref()?.height_at(p[0], p[2], terrain.as_ref()?.height_scale);
                Some(liquid.as_ref().and_then(|l| l.at(p)).map_or(h, |s| h.max(s.height - 1.2)))
            }
        }
    };
    let falls = anarchy.as_ref().map_or(Vec::new(), |a| crate::liquid::fall_effects(&a.water, &closest));
    if !falls.is_empty() {
        eprintln!("playfield {playfield}: the fall effect at {} places", falls.len());
    }
    let base_environment = anarchy.and_then(|a| a.environment);
    if let Some(e) = &base_environment {
        eprintln!("playfield {playfield}: environment weather {:?} ambient {:?} fog {:?} density {}", &e[..8], &e[19..22], &e[22..25], e[25]);
    }
    Ok(Area { scene, terrain, collision, booths, areas, base_environment, teleportals, zones, animated, interior, falls })
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
    // (the windows start where the original client's layout for this character has them: `layout.rs`, `aoprefs.rs`;
    // known before the first frame draws a window)
    crate::layout::set_character(state.own);
    crate::damage::set_own(state.own);
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
    // (before the first message is read: the things the server announces on entering are named and drawn by their item
    // templates. Set after the entering, as it was, the first ones announced in a run had neither name nor model and could
    // not be used: Arete Landing's cargo box and the junk about it, until the next zoning.)
    let _ = ITEM_DB.set(db.clone());
    crate::effects::load_calibration(game);
    let textures = crate::Textures::new(db.clone());
    let Area { scene, terrain, collision, booths, areas, base_environment, teleportals, zones, animated, interior, falls } = load_area_with(game, &db, playfield, Some(&textures))?;
    // The rooms of an interior: where the character is among them, which doors it may pass, which rooms are drawn.
    let mut interior = interior;
    let mut room_state = crate::interior::RoomState::default();
    let mut pending_interior: Option<Option<Arc<crate::interior::Interior>>> = Some(interior.clone());
    let mut rooms_shown: Vec<bool> = Vec::new();
    let mut pending_rooms_shown: Option<Vec<bool>> = None;
    // (the server set the character down elsewhere in its playfield: the viewer makes a new camera with the next update)
    let mut pending_camera = false;
    state.animated_statels = Arc::new(animated);
    let mut teleportals = teleportals;
    let mut zones = zones;
    let mut areas = areas;
    state.grid = is_grid(&areas);
    let mut base_environment = base_environment;
    let thread_textures = textures.clone_handle();
    let started_props = Instant::now();
    state.set_static_props(crate::static_props(&db, playfield));
    state.prop_textures.extend(crate::static_prop_textures(&db, playfield));
    state.prop_anim.extend(crate::static_prop_anims(&db, playfield).into_iter().map(|(k, (play, pos))| (k, (play, pos, Instant::now()))));
    state.approach_doors = crate::approach_doors(&db, playfield);
    state.table = crate::table_dynels(&db, playfield);
    state.mission_kinds = crate::mission_terminals(&db, playfield);
    profile!("[profile] static_props {:?}", started_props.elapsed());
    // (the collision is shared with the viewer, for the camera's lines of sight where the playfield has no surfaces)
    let (mut terrain, mut collision) = (terrain, Arc::new(collision));
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
    let (npc_chat_tx, npc_chat_rx) = std::sync::mpsc::channel::<crate::npcchat::Command>();
    let (mission_log_tx, mission_log_rx) = std::sync::mpsc::channel::<crate::missionlog::Command>();
    let own = state.own;
    let start = state.entities.get(&own).map(|e| e.0).unwrap_or([0.0; 3]);
    if std::env::var_os("MAO_COMBAT_LOG").is_some() {
        eprintln!("start: the entity's place {start:?}, the playfield message's {:?}", state.arrival.map(|a| a.1));
    }
    let heading = state.rotations.get(&own).map(|q| 2.0 * q[1].atan2(q[3])).unwrap_or(0.0);
    let (gear_db, gear_textures) = (db.clone(), crate::Textures::new(db.clone()));
    // The chat server (once per run, kept across zonings): its lines go to the chat window (and the on-screen log), and what the
    // window sends goes out through `send_chat`.
    if std::env::var_os("MAO_NO_CHAT").is_none() && !CHAT_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        if let Ok((user, password)) = credentials() {
            let (out_tx, out_rx) = std::sync::mpsc::channel::<crate::chatwindow::Out>();
            let _ = CHAT_OUT.set(std::sync::Mutex::new(out_tx));
            std::thread::spawn(move || {
                let server = std::env::var("MAO_CHAT_SERVER").unwrap_or_else(|_| mao_net::chat::SERVER.to_string());
                // (a connection that ends is made again, as the original's is (FUN_1008a2c1 counts its attempts): here
                // after some seconds, more after each that did not get as far as logging in, and no more after five such
                // in a row. The loop's body is not indented again.)
                let (mut failed, mut again) = (0u32, false);
                loop {
                let mut logged_in = false;
                let result = (|| -> Result<(), mao_net::chat::Error> {
                    use crate::chatwindow::{Kind, Line, Out};
                    use mao_net::chat::Packet;
                    let mut chat = mao_net::chat::Chat::connect(server.as_str())?;
                    chat.login(own, &user, &password)?;
                    logged_in = true;
                    if again {
                        // (what was typed while there was no connection is not sent late)
                        while out_rx.try_recv().is_ok() {}
                        CHAT_LINES.lock().unwrap().push(Line::new(Kind::System, "Connected to the chat server again."));
                    }
                    // The read waits a moment, so what the window sends goes out promptly.
                    chat.set_timeout(Duration::from_millis(200))?;
                    let mut names: BTreeMap<u32, String> = BTreeMap::new();
                    let mut ids: BTreeMap<String, u32> = BTreeMap::new();
                    let mut channels: BTreeMap<u32, (u8, String)> = BTreeMap::new();
                    // the private chat groups the character is in: the owner's id and the group's name (the owner's)
                    let mut private: BTreeMap<u32, String> = BTreeMap::new();
                    // each channel's flags (0x40: no voice lines)
                    let mut channel_flags: BTreeMap<u32, u32> = BTreeMap::new();
                    // tells waiting for the id of the character they go to: (lower-case name, text)
                    let mut waiting: Vec<(String, String)> = Vec::new();
                    // names of characters to put on the friends list once their id is known
                    let mut befriending: Vec<String> = Vec::new();
                    // (the private chat group's requests waiting for a name's id: the name, and the request)
                    let mut private_asked: Vec<(String, u8)> = Vec::new();
                    // names of characters a tell window is to open for once the chat server has said it knows them
                    let mut tell_asked: Vec<String> = Vec::new();
                    // the senders whose voice lines were heard lately
                    let mut voices = crate::voices::Limiter::default();
                    loop {
                        let packet = match chat.receive() {
                            Ok(p) => Some(p),
                            Err(mao_net::chat::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => None,
                            Err(e) => return Err(e),
                        };
                        if std::env::var_os("MAO_TRACE_CHAT").is_some()
                            && let Some(Packet::Name { id, name }) = &packet
                        {
                            eprintln!("chat name {name:?}: id {id:#x}");
                        }
                        let line = match packet {
                            // (no character of that name: the answer's id is all ones. FUN_1008a2c1 says so with
                            // `ChatCmdFeedback_UnknownUser` and drops what waited for the name. A tell sent on to that id
                            // made the chat server close the connection.)
                            Some(Packet::Name { id: u32::MAX, name }) => {
                                let key = name.to_lowercase();
                                let asked = waiting.iter().any(|w| w.0 == key) || tell_asked.contains(&key) || befriending.contains(&key) || private_asked.iter().any(|w| w.0 == key);
                                private_asked.retain(|w| w.0 != key);
                                waiting.retain(|w| w.0 != key);
                                tell_asked.retain(|w| *w != key);
                                befriending.retain(|w| *w != key);
                                asked.then(|| Line::new(Kind::System, crate::gui::xml::ldb_named(10001, "ChatCmdFeedback_UnknownUser", "There is no character named %s.").replace("%s", &name)))
                            }
                            Some(Packet::Name { id, name }) => {
                                ids.insert(name.to_lowercase(), id);
                                names.insert(id, name.clone());
                                // (the tell window that waited for this name: `OpenTellWindow`, which lists the character
                                // among the recent ones, FUN_100a6487)
                                if tell_asked.contains(&name.to_lowercase()) {
                                    tell_asked.retain(|w| *w != name.to_lowercase());
                                    recent_message(&mut chat, id, &name)?;
                                    crate::chatgroups::open_tell(&name);
                                }
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
                                if !sent.is_empty() {
                                    recent_message(&mut chat, id, &name)?;
                                }
                                for text in sent {
                                    chat.send(&mao_net::chat::tell_packet(id, &text))?;
                                }
                                // (the private chat group's requests that waited for this name)
                                let mut requests = Vec::new();
                                private_asked.retain(|(who, what)| {
                                    if *who == name.to_lowercase() {
                                        requests.push(*what);
                                        false
                                    } else {
                                        true
                                    }
                                });
                                for what in requests {
                                    chat.send(&mao_net::chat::private_request_packet(what, id))?;
                                }
                                if befriending.contains(&name.to_lowercase()) {
                                    befriending.retain(|w| *w != name.to_lowercase());
                                    chat.send(&mao_net::chat::buddy_add_packet(id, false))?;
                                    let mut buddies = BUDDIES.lock().unwrap();
                                    buddies.1.entry(id).and_modify(|b| b.2 = false).or_insert((name.clone(), false, false));
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
                                crate::chatgroups::note_channel(&name, kind, flags);
                                channels.insert(id, (kind, name));
                                None
                            }
                            // (a channel left: the team's when the team is)
                            Some(Packet::ChannelGone { id, .. }) => {
                                channel_flags.remove(&id);
                                if let Some((_, name)) = channels.remove(&id) {
                                    crate::chatgroups::forget(&name);
                                }
                                None
                            }
                            Some(Packet::Notice { name, text, blob }) => {
                                // (`HandleVicinityMessage` → FUN_1008cdb3: its voice line, if any)
                                crate::voices::vicinity_message(&mut voices, &name, &blob);
                                // (an NPC that answers a use with a spoken line has answered: `NPC_SPOKE`)
                                if let Ok(mut spoke) = NPC_SPOKE.lock() {
                                    *spoke = Some((format!("{name}\n{}", mao_net::chat::plain(&text)), Instant::now()));
                                }
                                Some(Line::new(Kind::System, mao_net::chat::plain(&text)).with_markup("", &text))
                            }
                            Some(Packet::Vicinity { sender, text, blob }) => {
                                let who = names.get(&sender).cloned().unwrap_or_else(|| format!("{sender:x}"));
                                crate::voices::vicinity_message(&mut voices, &who, &blob);
                                Some(Line { kind: Kind::Channel, text: format!("{who}: {}", mao_net::chat::plain(&text)), from: Some(who.clone()), markup: None, group: crate::chatgroups::VICINITY.to_string(), at: None }.with_markup(&format!("{}: ", crate::chatwindow::name_link(&who)), &text))
                            }
                            Some(Packet::Tell { sender, text, blob }) => {
                                let raw = text;
                                let text = mao_net::chat::plain(&raw);
                                match names.get(&sender) {
                                    // (the automatic answer of a character that is away, `HandlePrivateMessage`'s kind
                                    // 6: FUN_100a681d with no name, so nothing in front of it, in that character's
                                    // tells; it is not answered and makes no tip)
                                    Some(who) if blob.first() == Some(&mao_net::chat::AWAY_REPLY) => Some(Line { kind: Kind::AwayReply, text, from: Some(who.clone()), markup: None, group: String::new(), at: None }.with_markup("", &raw)),
                                    Some(who) => {
                                        recent_message(&mut chat, sender, who)?;
                                        crate::tips::trigger("OnReceiveTellMessage", "");
                                        // (FUN_100a681d: the sender's name in brackets, a link)
                                        Some(Line { kind: Kind::Tell, text: format!("[{who}]: {text}"), from: Some(who.clone()), markup: None, group: String::new(), at: None }.with_markup(&format!("[{}]: ", crate::chatwindow::name_link(who)), &raw))
                                    }
                                    // the server's own notices (the players online) come the same way
                                    None => Some(Line::new(Kind::System, text).with_markup("", &raw)),
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
                                Some(Line { kind: Kind::Channel, text: format!("[{place}] {who}: {}", mao_net::chat::plain(&text)), from: Some(who.clone()), markup: None, group: place.clone(), at: None }.with_markup(&format!("[{place}] {}: ", crate::chatwindow::name_link(&who)), &text))
                            }
                            Some(Packet::Buddy { id, online, kind }) => {
                                let mut buddies = BUDDIES.lock().unwrap();
                                let name = names.get(&id).cloned().unwrap_or_else(|| format!("#{id}"));
                                let temporary = mao_net::chat::buddy_is_temporary(&kind);
                                if std::env::var_os("MAO_TRACE_CHAT").is_some() {
                                    eprintln!("buddy {id} online {online} kind {:?}", kind.as_bytes());
                                }
                                // (a friend already on the list who comes or goes is told; the first listing is not, nor one
                                // of the recent messages)
                                let changed = !temporary && buddies.1.get(&id).is_some_and(|(_, was, recent)| *was != online && !*recent);
                                let shown = buddies.1.get(&id).map_or(name.clone(), |(n, _, _)| n.clone());
                                buddies.1.insert(id, (name, online, temporary));
                                buddies.0 += 1;
                                changed.then(|| Line::new(Kind::System, format!("{shown} {}.", if online { "is now online" } else { "has gone offline" })))
                            }
                            // Private chat groups (GUI.dll's chat library, FUN_1016d6f4): a group goes by its owner's name.
                            Some(Packet::PrivateInvite { owner }) => {
                                crate::chatgroups::invite(owner, &names.get(&owner).cloned().unwrap_or_else(|| format!("#{owner}")));
                                None
                            }
                            Some(Packet::PrivateJoined { owner, character }) => {
                                let said = |key: &str, own_words: &str| crate::gui::xml::ldb_named(10001, key, own_words);
                                let group = private.entry(owner).or_insert_with(|| names.get(&owner).cloned().unwrap_or_else(|| format!("#{owner}"))).clone();
                                crate::chatgroups::note(&group, 0x0e);
                                if character == own {
                                    Some(Line::new(Kind::System, format!("{}{group}", said("YouJoinedPrivateChat", "You joined the private group "))))
                                } else {
                                    let who = names.get(&character).cloned().unwrap_or_else(|| format!("#{character}"));
                                    Some(Line::new(Kind::Channel, format!("[{group}] {who}{}", said("JoinedGroup", " joined the group."))).in_group(&group))
                                }
                            }
                            Some(Packet::PrivateParted { owner, character }) => {
                                let group = private.get(&owner).cloned().unwrap_or_else(|| names.get(&owner).cloned().unwrap_or_else(|| format!("#{owner}")));
                                let who = names.get(&character).cloned().unwrap_or_else(|| format!("#{character}"));
                                Some(Line::new(Kind::Channel, format!("[{group}] {who}{}", crate::gui::xml::ldb_named(10001, "LeftGroup", " left the group."))).in_group(&group))
                            }
                            Some(Packet::PrivateEnded { owner }) => private.remove(&owner).map(|group| {
                                crate::chatgroups::forget(&group);
                                Line::new(Kind::System, format!("{}{group}", crate::gui::xml::ldb_named(10001, "LeftPrivateGroup", "You left the private group ")))
                            }),
                            Some(Packet::PrivateRefused { character, .. }) => {
                                let who = names.get(&character).cloned().unwrap_or_else(|| format!("#{character}"));
                                Some(Line::new(Kind::System, format!("{who}{}", crate::gui::xml::ldb_named(10001, "DeclinedInvite", " declined the invitation."))))
                            }
                            Some(Packet::PrivateMessage { owner, sender, text, .. }) => {
                                let group = private.entry(owner).or_insert_with(|| names.get(&owner).cloned().unwrap_or_else(|| format!("#{owner}"))).clone();
                                crate::chatgroups::note(&group, 0x0e);
                                let who = names.get(&sender).cloned().unwrap_or_else(|| format!("{sender:x}"));
                                Some(Line { kind: Kind::Channel, text: format!("[{group}] {who}: {}", mao_net::chat::plain(&text)), from: Some(who.clone()), markup: None, group: group.clone(), at: None }.with_markup(&format!("[{group}] {}: ", crate::chatwindow::name_link(&who)), &text))
                            }
                            Some(Packet::Other { kind, data }) => {
                                eprintln!("chat packet {kind}: {}", data.iter().map(|b| format!("{b:02x}")).collect::<String>());
                                None
                            }
                            _ => None,
                        };
                        // A line with control characters is a packet of another layout read wrongly (seen once live): skip it.
                        // (Its plain text, that is, where a line break is a space and a link's target is left out,
                        // `chat::plain`: a bot's page of buffs has line breaks in it, and was dropped whole.)
                        if let Some(line) = line {
                            if line.text.chars().any(char::is_control) {
                                eprintln!("chat: a line with control characters dropped ({} characters)", line.text.chars().count());
                            } else {
                                CHAT_LINES.lock().unwrap().push(line);
                            }
                        }
                        while let Ok(out) = out_rx.try_recv() {
                            match out {
                                Out::Tell { name, text } => match ids.get(&name.to_lowercase()) {
                                    Some(&id) => {
                                        recent_message(&mut chat, id, &name)?;
                                        chat.send(&mao_net::chat::tell_packet(id, &text))?
                                    }
                                    None => {
                                        waiting.push((name.to_lowercase(), text));
                                        chat.send(&mao_net::chat::lookup_packet(&name))?;
                                    }
                                },
                                // (FUN_10089940: a text over 0x400 long is dropped; the character's id is known,
                                // their tell has just come)
                                Out::AwayReply { name, text } => {
                                    if let Some(&id) = ids.get(&name.to_lowercase()).filter(|_| text.len() <= 0x400) {
                                        chat.send(&mao_net::chat::away_reply_packet(id, &text))?;
                                    }
                                }
                                Out::TellWindow { name } => match ids.get(&name.to_lowercase()).and_then(|id| Some((*id, names.get(id)?.clone()))) {
                                    Some((id, known)) => {
                                        recent_message(&mut chat, id, &known)?;
                                        crate::chatgroups::open_tell(&known);
                                    }
                                    None => {
                                        tell_asked.push(name.to_lowercase());
                                        chat.send(&mao_net::chat::lookup_packet(&name))?;
                                    }
                                },
                                Out::Camp | Out::Sit | Out::Perf | Out::Pet(_) | Out::Duel(_) | Out::PetDuel(_) | Out::Server(_) | Out::FxScript(_) | Out::Quit | Out::Follow | Out::Inspect | Out::Window { .. } | Out::ToggleWindow { .. } | Out::Assist { .. } | Out::Waypoint { .. } | Out::SelectSelf | Out::SetOption { .. } | Out::DValue { .. } | Out::TextCommand { .. } | Out::OrgCommand { .. } | Out::ViewDist(_) | Out::CharDist(_) => {}
                                Out::Befriend { name } => match ids.get(&name.to_lowercase()) {
                                    Some(&id) => {
                                        chat.send(&mao_net::chat::buddy_add_packet(id, false))?;
                                        let mut buddies = BUDDIES.lock().unwrap();
                                        buddies.1.entry(id).and_modify(|b| b.2 = false).or_insert((name, false, false));
                                        buddies.0 += 1;
                                    }
                                    None => {
                                        befriending.push(name.to_lowercase());
                                        chat.send(&mao_net::chat::lookup_packet(&name))?;
                                    }
                                },
                                Out::PrivateInvite { id } => chat.send(&mao_net::chat::private_invite_packet(id))?,
                                // (`/invite`, `/kick`, `/leave` by a name: done at once for a name already known, else
                                // when the server has said whose it is)
                                Out::PrivateByName { what, name } => match ids.get(&name.to_lowercase()).copied() {
                                    Some(id) => chat.send(&mao_net::chat::private_request_packet(what, id))?,
                                    None => {
                                        private_asked.push((name.to_lowercase(), what));
                                        chat.send(&mao_net::chat::lookup_packet(&name))?;
                                    }
                                },
                                // (a line typed in a window whose output group is a channel)
                                Out::PrivateJoin { owner } => chat.send(&mao_net::chat::private_join_packet(owner))?,
                                Out::PrivateDecline { owner } => chat.send(&mao_net::chat::private_leave_packet(owner))?,
                                Out::PrivateLeave { name } => {
                                    if let Some((&owner, _)) = private.iter().find(|(_, n)| n.eq_ignore_ascii_case(&name)) {
                                        chat.send(&mao_net::chat::private_leave_packet(owner))?;
                                    }
                                }
                                // (a line to a private group the character is in goes there)
                                Out::Channel { name, text } if private.values().any(|n| n.eq_ignore_ascii_case(&name)) => {
                                    if let Some((&owner, _)) = private.iter().find(|(_, n)| n.eq_ignore_ascii_case(&name)) {
                                        chat.send(&mao_net::chat::private_message_packet(owner, &text))?;
                                    }
                                }
                                Out::Channel { name, text } => match channels.iter().find(|(_, (_, n))| n.eq_ignore_ascii_case(&name)) {
                                    Some((&id, (kind, _))) => chat.send(&mao_net::chat::channel_packet(*kind, id, &text))?,
                                    None => CHAT_LINES.lock().unwrap().push(Line::new(Kind::System, format!("Error: Chat group {name} is currently not available."))),
                                },
                                Out::Unfriend { id } => {
                                    chat.send(&mao_net::chat::buddy_remove_packet(id))?;
                                    let mut buddies = BUDDIES.lock().unwrap();
                                    buddies.1.remove(&id);
                                    buddies.0 += 1;
                                }
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
                failed = if logged_in { 0 } else { failed + 1 };
                if failed >= 5 {
                    CHAT_LINES.lock().unwrap().push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, "The chat server cannot be reached: no more tries until the client is started again."));
                    break;
                }
                std::thread::sleep(Duration::from_secs(5 + 10 * u64::from(failed)));
                again = true;
                }
            });
        }
    }
    std::thread::spawn(move || {
        use mao_net::n3::movement;
        // (the character as the original's steered vehicle: its velocity, the direction it was pushed in, the walk toggle seen)
        let mut velocity = [0.0f32; 2];
        let mut pushed = 0i8;
        let mut walk_toggle_seen = 0u8;
        // (the movement mode told the server on entering a playfield: it keeps the last one it heard, even from another session,
        // and checks the reports against that mode's speed)
        let mut mode_told = false;
        let mut target_told = false;
        // (the heading the last movement message carried: `CheckMotionUpdate` reports a turn of more than 0.17 rad since)
        let mut sent_heading = 0.0f32;
        let old_reports = std::env::var_os("MAO_OLD_REPORTS").is_some();
        // The server's heights sit a little off the ground model's; walk at
        // the ground's height plus the offset seen at the start.
        let ground = |terrain: &Option<Arc<mao_formats::terrain::Terrain>>, p: [f32; 3]| terrain.as_ref().map_or(0.0, |t| t.height_at(p[0], p[2], t.height_scale));
        // (not where the character arrives swimming: its height is the water's then, not the ground's)
        let ground_offset = |terrain: &Option<Arc<mao_formats::terrain::Terrain>>, p: [f32; 3]| {
            let g = ground(terrain, p);
            let wet = crate::liquid::current().and_then(|l| l.at(p)).is_some_and(|l| p[1] < l.height + SWIM_ABOVE && l.height - g > SWIM_DEPTH);
            if wet { 0.0 } else { p[1] - g }
        };
        let mut offset = ground_offset(&terrain, start);
        let mut base = start[1];
        let clock = Instant::now();
        let tracing = std::env::var("MAO_TRACE").is_ok_and(|v| v == "1");
        let mut last_report = Instant::now();
        let mut fresh = false;
        state.arrival = None;
        let mut pending_scene: Option<Model> = None;
        // (the new playfield's waterfalls' feet, for the viewer's fall effects)
        let mut pending_falls: Option<Vec<[f32; 3]>> = Some(falls);
        let first_inventory = state.inventory.take();
        if let Some(entries) = &first_inventory {
            state.worn = entries.clone();
            if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                for e in entries.iter().filter(|e| e.slot >= inventory::BACKPACK_FIRST) {
                    eprintln!("combat: inventory place {:#x}: {:x} {}", e.slot, e.low_id, template_name(e.low_id).unwrap_or_default());
                }
            }
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
        let mut fxs_visuals = crate::fxs::visuals::Visuals::default();
        // (the environment's own wind for the scripts' high wind: FUN_100becbe takes it before the frame's step)
        let mut fxs_wind = crate::wind::Wind::default();
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
        // (the NPC chat: what the server said for the window, and the NPC talked to)
        let (mut pending_npc_chat, mut chat_npc): (Vec<crate::npcchat::Event>, Option<(u32, u32)>) = (Vec::new(), None);
        // The NPC asked to talk and not yet heard from: who, when, and whether it was asked again. An NPC whose chat was
        // open when a client went without closing it answers no new opening until it is told that chat is closed (seen
        // live 2026-11-10), so one that stays silent for two seconds is told so and asked once more. This client's own
        // way round it: the original has nothing of the kind.
        let mut npc_asked: Option<((u32, u32), Instant, bool)> = None;
        // (the mission journal: the server's whole list when it sends one, and the missions it dropped)
        let (mut pending_mission_log, mut pending_missions_removed): (Option<Vec<crate::missionlog::Mission>>, Vec<(u32, u32)>) = (None, Vec::new());
        let mut shop_booth: Option<(u32, u32)> = None;
        // (the player a trade is open with)
        let mut player_trade: Option<u32> = None;
        let (mut shop_rates, mut shop_rows, mut pending_shop_changes) = (crate::shop::Rates { buy: 0, sell: 0, kind: 0 }, Vec::<crate::shop::Row>::new(), Vec::<crate::shop::Change>::new());
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
        // `MAO_AUTOPLAY=5:tab,6:follow,9:attack,...`: the keys pressed by themselves that many seconds in (for testing
        // the fight live without a keyboard): tab, attack, use, follow (the target), stop (following), loot (all of the open container), camp, x (sit or stand), walk (the walk/run toggle).
        let mut autoplay: Vec<(f32, String)> = std::env::var("MAO_AUTOPLAY").ok().map_or_else(Vec::new, |v| v.split(',').filter_map(|s| s.split_once(':').and_then(|(t, c)| Some((t.trim().parse().ok()?, c.trim().to_string())))).collect());
        let autoplay_clock = Instant::now();
        let (mut sitting, mut uses, mut fired) = (false, 0u32, 0u8);
        let mut camp_seen = 0u8;
        let (mut special_seen, mut social_seen, mut pick_seen, mut use_on_seen) = (0u8, 0u8, 0u8, 0u8);
        // The item index (`itemindex.rs`): when the character's things were last looked at for it, and what they were.
        let (mut index_checked, mut index_signature) = (Instant::now(), 0u64);
        let mut index_names: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
        // (the bags still to be looked into for it, by inventory place, and when the last was used)
        let mut index_scan: std::collections::VecDeque<(u32, (u32, u32))> = std::collections::VecDeque::new();
        let mut index_scanned = Instant::now();
        let mut aggdef_seen = 0u8;
        let mut market_seen = 0u8;
        // (an attack sent and not yet answered: FUN_1006817d's +0x79)
        let mut attack_sent: Option<Instant> = None;
        let mut headers_made = Instant::now() - Duration::from_secs(60);
        let mut mission_booth: Option<u32> = None;
        // (when missions were last asked for: the server's `Feedback` soon after, "too far away" and the like, is the answer)
        let mut mission_asked: Option<Instant> = None;
        let mut friends_seen = u64::MAX;
        let mut cast_seen = 0u8;
        let mut move_seen = 0u8;
        let mut take_seen = 0u8;
        let mut put_seen = 0u8;
        let mut saved_look: Option<crate::look::Look> = None;
        let mut shot_cache: std::collections::HashMap<u32, (Option<mao_formats::gfx::Visual>, Option<mao_formats::gfx::Visual>)> = std::collections::HashMap::new();
        let mut burst_cache: std::collections::HashMap<u32, Option<mao_formats::gfx::Cast>> = std::collections::HashMap::new();
        let mut aura_cache: std::collections::HashMap<u32, Option<mao_formats::gfx::Visual>> = std::collections::HashMap::new();
        // (the target NCU windows: whose nanos were told last, and the nanos' whole times)
        let mut target_nanos_told: [Option<(u32, Vec<u32>)>; 2] = [None, None];
        let mut duration_cache: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        // (the effect templates: what a running nano program looks like)
        let gfx = mao_formats::files::read(Path::new(&game_dir).join("Setupf").join("gfxtweak.bin")).ok().and_then(|d| mao_formats::gfx::Gfx::parse(&d).ok());
        // (the lightning's splash, gfx 0x859b)
        crate::lightning::set_splash(gfx.as_ref().and_then(|g| mao_formats::gfx::Splash::read(g.get(0x859b)?)));
        let mut polymorph_cache: std::collections::HashMap<u32, Option<u32>> = std::collections::HashMap::new();
        let mut vehicle_cache: std::collections::HashMap<u32, bool> = std::collections::HashMap::new();
        let mut delete_seen = 0u8;
        let mut combine_seen = 0u8;
        let mut cancel_seen = 0u8;
        let mut train_perk_seen = 0u8;
        let mut pet_seen = 0u8;
        let mut duel_seen = 0u8;
        let mut pet_duel_seen = 0u8;
        let mut select_id_seen = 0u8;
        let mut inspect_seen = 0u8;
        let mut self_seen = 0u8;
        let (mut following, mut follow_seen): (Option<u32>, u8) = (None, 0);
        let (mut position, mut heading) = (start, heading);
        let (mut moving, mut turning, mut strafing) = (0i8, 0i8, 0i8);
        const JUMP_SPEED: f32 = 7.0;
        const GRAVITY: f32 = 10.0;
        let (mut airborne, mut rise, mut vy, mut was_jumping) = (false, 0.0f32, 0.0f32, false);
        // (the vehicle's body, made anew on entering a playfield, and whether the character is in a jump)
        let (mut body, mut body_reset, mut jumping) = (crate::vehicle::Body::default(), false, false);
        // (`MAO_AUTOPLAY`'s jump: one press)
        let mut autoplay_jump = false;
        // (the effect scripts' meshes' universal effects by mesh name, and the names' index to find the meshes by)
        let mut fxs_mesh_effects: std::collections::HashMap<String, Vec<i32>> = std::collections::HashMap::new();
        // (the universal effects running: a number of its own, the object it goes with, the effect, and whether the
        // viewer has been given it)
        let mut fxs_universal: Vec<(u64, crate::fxs::Obj, i32, bool)> = Vec::new();
        let mut fxs_universal_next = 0u64;
        let fxs_index = gear_db.get(mao_formats::resource::Id { kind: 0xf424a, instance: 1 }).ok().flatten().and_then(|o| mao_formats::instances::Instances::parse(&o).ok());
        let mut last_tick = Instant::now();
        let mut last_send = Instant::now();
        let mut last_overlay = Instant::now() - Duration::from_secs(1);
        // The map: fog of war revealed as the character walks (see `automap`).
        // (the playfield's map picture: the ground's tile textures from above, `automap::ground_picture`)
        let ground_of = |terrain: Option<&mao_formats::terrain::Terrain>| {
            let began = Instant::now();
            let ground = terrain.map(|t| crate::automap::ground_picture(t, |instance| gear_textures.get(0xf6966, instance).map(|t| (t.width as usize, t.height as usize, t.rgba))));
            if let (Some(g), true) = (&ground, std::env::var_os("MAO_PROFILE").is_some()) {
                eprintln!("profile: the map's ground picture, {} by {}, took {:.0} ms", g.size[0], g.size[1], began.elapsed().as_secs_f32() * 1000.0);
            }
            ground
        };
        let mut map = crate::automap::MapBuilder::new(terrain.as_deref(), start, playfield).with_ground(ground_of(terrain.as_deref()));
        let mut pending_map: Option<crate::automap::MapPatch> = None;
        let mut last_reveal = Instant::now() - Duration::from_secs(1);
        let profile = std::env::var_os("MAO_PROFILE").is_some();
        let mut fake_done = false;
        let mut last_own = (start, Instant::now());
        // (where we told the server we were, the last few seconds: a report of ours echoed back is one of these)
        let mut told: std::collections::VecDeque<([f32; 3], Instant)> = std::collections::VecDeque::new();
        // (flying, the vehicle's elevation goes into the reports' rotation)
        let own_pitch = std::cell::Cell::new(0.0f32);
        let send = |connection: &mut Connection, kind: u8, heading: f32, position: [f32; 3], since: &mut Instant| {
            let ms = since.elapsed().as_millis().min(u32::MAX as u128) as u32;
            *since = Instant::now();
            connection.send(&n3::position_update_pitched(own, kind, heading, own_pitch.get(), position, ms)).is_ok()
        };
        let (mut was_flying, mut sent_pitch, mut flying_told) = (false, 0.0f32, false);
        let mut sent = Instant::now();
        let mut prof_iter = Instant::now();
        // (a message that came while one was handled waits here: up to `BURST` are handled in a row before the frame's
        // own work, so a busy area does not queue up behind it)
        const BURST: u32 = 200;
        let (mut stash, mut burst): (Option<frame::Frame>, u32) = (None, 0);
        loop {
            prof_iter = crate::prof::lap("net-iteration", prof_iter);
            // MAO_FAKE_REDIRECT=<seconds>: behave as if the server sent a redirect then (to test the hop without a zone line).
            let fake_due = !fake_done && std::env::var("MAO_FAKE_REDIRECT").ok().and_then(|v| v.parse::<u64>().ok()).is_some_and(|s| clock.elapsed() > Duration::from_secs(s));
            let received = if fake_due {
                fake_done = true;
                Ok(system::redirect_frame([37, 18, 193, 19], 7509))
            } else if let Some(frame) = stash.take() {
                Ok(frame)
            } else {
                let started = Instant::now();
                let r = connection.receive();
                crate::prof::add(if r.is_ok() { "net-receive-got" } else { "net-receive-wait" }, started.elapsed());
                r
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
                    // The NPC chat (`npcchat.rs`): the window opened with the NPC's name, its lines and answers.
                    if let Some(event) = Message::parse(&frame).and_then(|m| crate::npcchat::event(m.kind, &m.data)) {
                        let npc = match &event {
                            crate::npcchat::Event::Open(n, _) | crate::npcchat::Event::Text(n, ..) | crate::npcchat::Event::Answers(n, _) | crate::npcchat::Event::Close(n) | crate::npcchat::Event::StartTrade(n, ..) | crate::npcchat::Event::Rejected(n, _) => *n,
                        };
                        let name = state.names.get(&npc.1).map(|n| n.0.clone()).or_else(|| state.infos.get(&npc.1).map(|i| i.name.clone())).unwrap_or_default();
                        if npc_asked.is_some_and(|a| a.0 == npc) {
                            npc_asked = None;
                        }
                        if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                            eprintln!("npc chat: {event:?}");
                        }
                        match event {
                            crate::npcchat::Event::Open(..) => {
                                chat_npc = Some(npc);
                                pending_npc_chat.push(crate::npcchat::Event::Open(npc, name));
                            }
                            crate::npcchat::Event::Close(_) => {
                                if chat_npc == Some(npc) {
                                    chat_npc = None;
                                }
                                pending_npc_chat.push(event);
                            }
                            event => {
                                if chat_npc != Some(npc) {
                                    chat_npc = Some(npc);
                                    pending_npc_chat.push(crate::npcchat::Event::Open(npc, name));
                                }
                                pending_npc_chat.push(event);
                            }
                        }
                    }
                    // The tradeskill window's answers (`tradeskill.rs`): the result's name and icon from its low template.
                    if let Some((action, first, params)) = Message::parse(&frame).filter(|m| m.identity.1 == own).as_ref().and_then(n3::tradeskill_reply) {
                        let event = match action {
                            0xdf => crate::tradeskill::Event::Source(params.1 as i32),
                            0xe0 => crate::tradeskill::Event::Target(params.1 as i32),
                            0xe1 => crate::tradeskill::Event::NotValid,
                            0xe2 => crate::tradeskill::Event::OutOfRange(params.1 as i32),
                            0xe3 => crate::tradeskill::Event::Requirement(params.0, params.1 as i32),
                            _ => {
                                let (low, high) = (params.1, first.1);
                                let template = [low, high].into_iter().find_map(|t| gear_db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: t }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)));
                                let icon = template.as_ref().and_then(|t| t.icon()).and_then(|id| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|x| std::sync::Arc::new(crate::gear::Icon { id, width: x.width as usize, height: x.height as usize, rgba: x.rgba })));
                                let name = template.map_or_else(|| format!("Item #{low:x}"), |t| t.name);
                                crate::tradeskill::Event::Result { min: params.0 as i32, max: first.0 as i32, low, high, name, icon }
                            }
                        };
                        crate::tradeskill::push_event(event);
                    }
                    if let Some(m) = Message::parse(&frame).filter(|m| m.kind == 0x58362220) {
                        if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                            let int = |v: &mao_net::schema::Value, f: &str| v.get(f).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                            if let Some(mao_net::schema::Value::List(list)) = p.value.get("VendingMachineSlots") {
                                // (the shop's prices are its template's stats: the playfield table's record, else the
                                // announcement's `StaticInstance`)
                                let table = state.table.iter().find(|d| (d.0, d.1) == m.identity);
                                let template = table.map(|d| d.3).or_else(|| state.vendor_templates.get(&m.identity.1).copied());
                                let item = template.and_then(|t| gear_db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: t }).ok().flatten()).and_then(|d| mao_formats::item::Item::parse(&d));
                                shop_rates = item.as_ref().map_or(crate::shop::Rates { buy: 0, sell: 0, kind: 0 }, crate::shop::Rates::of);
                                let literacy = state.stats.get(&161).copied().unwrap_or(0);
                                let icon = |id: u32| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|x| std::sync::Arc::new(crate::gear::Icon { id, width: x.width as usize, height: x.height as usize, rgba: x.rgba }));
                                let stock: Vec<crate::shop::Slot> = list.iter().map(|s| crate::shop::slot(&gear_db, &icon, (int(s, "ItemLowId"), int(s, "ItemHighId"), int(s, "Quality")), shop_rates, literacy)).collect();
                                shop_rows = stock.iter().map(|s| crate::shop::Row { item: [0; 4], name: s.name.clone(), icon: s.icon.clone(), count: 1, quality: s.quality, price: s.price }).collect();
                                let name = table.map(|d| d.4.clone()).filter(|n| !n.is_empty()).or_else(|| state.vendor_names.get(&m.identity.1).cloned()).or_else(|| item.map(|i| i.name)).unwrap_or_default();
                                shop_booth = Some(m.identity);
                                pending_shop = Some(crate::shop::ShopState { shop: m.identity, name, stock });
                            }
                        }
                    }
                    // The trade's echoes (to us): an item put in (5) shows in its list, one taken out (6) leaves it, and
                    // action 4 (captured after both a purchase and a sale) is the trade completed.
                    // (a trade with a player: the partner's own echoes come under their identity, as the 2026-10-31 15:05
                    // capture's last Complete)
                    if let Some(m) = Message::parse(&frame).filter(|m| m.kind == 0x36284f6e && (m.identity == (0xc350, own) || Some(m.identity.1) == player_trade) && m.data.len() >= 22) {
                        let word = |i: usize| u32::from_be_bytes(m.data[6 + 4 * i..10 + 4 * i].try_into().unwrap());
                        let item = [word(0), word(1), word(2), word(3)];
                        let naming = |who: u32| item[0] == n3::CHARACTER && item[1] == who;
                        match m.data[5] {
                            // (Open naming another character: a trade with a player, `N3Msg_TradeStart` or theirs)
                            0 if item[0] == n3::CHARACTER && item[1] != own && shop_booth.is_none() => {
                                player_trade = Some(item[1]);
                                let name = state.names.get(&item[1]).map_or_else(|| format!("Character {:x}", item[1]), |n| n.0.clone());
                                pending_shop_changes.push(crate::shop::Change::PlayerOpen { partner: item[1], name });
                            }
                            7 if player_trade.is_some() && m.identity.1 != own => pending_shop_changes.push(crate::shop::Change::PartnerCredits(word(1) as i64)),
                            3 if player_trade.is_some() && naming(own) => pending_shop_changes.push(crate::shop::Change::Confirmed { ours: true }),
                            1 | 3 if player_trade.is_some_and(|p| naming(p)) => pending_shop_changes.push(crate::shop::Change::Confirmed { ours: false }),
                            2 if player_trade.is_some() => {
                                player_trade = None;
                                pending_shop_changes.push(crate::shop::Change::Declined);
                            }
                            4 if player_trade.is_some() => {
                                player_trade = None;
                                pending_shop_changes.push(crate::shop::Change::Complete);
                            }
                            5 if item[0] == 0xc75b && item[2] == 0x6f => {
                                if let Some(row) = shop_rows.get(item[3] as usize) {
                                    pending_shop_changes.push(crate::shop::Change::Bought(crate::shop::Row { item, ..row.clone() }));
                                }
                            }
                            5 if item[0] == n3::CHARACTER && item[2] == 0x68 => {
                                if let Some(entry) = state.worn.iter().find(|e| e.slot == item[3]) {
                                    let literacy = state.stats.get(&161).copied().unwrap_or(0);
                                    let icon = |id: u32| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|x| std::sync::Arc::new(crate::gear::Icon { id, width: x.width as usize, height: x.height as usize, rgba: x.rgba }));
                                    pending_shop_changes.push(crate::shop::Change::Sold(crate::shop::sold_row(&gear_db, &icon, item, entry, shop_rates, literacy)));
                                }
                            }
                            6 => pending_shop_changes.push(crate::shop::Change::Removed(item)),
                            4 if shop_booth.is_some() => {
                                pending_shop_changes.push(crate::shop::Change::Complete);
                                shop_reuse = Some(Instant::now() + Duration::from_millis(300));
                            }
                            _ => {}
                        }
                    }
                    // The mission journal (`missionlog.rs`): `QuestFullUpdate` is the whole list, `Quest` action 1 a deletion.
                    if let Some(m) = Message::parse(&frame).filter(|m| m.kind == 0x465a4061 || m.kind == 0x212c487a) {
                        if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                            if m.kind == 0x465a4061 {
                                let icon = |id: u32| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|t| std::sync::Arc::new(crate::gear::Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba }));
                                pending_mission_log = Some(crate::missionlog::missions_from(&p.value, &icon));
                            } else if p.value.get("Action").and_then(mao_net::schema::Value::as_int) == Some(1) {
                                let id = p.value.get("Mission").map(|i| (i.get("Type").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32, i.get("Instance").and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32));
                                pending_missions_removed.extend(id);
                            }
                        }
                    }
                    if let Some(m) = Message::parse(&frame).filter(|m| m.kind == 0x50544d19 && m.identity.1 == own && mission_asked.is_some_and(|t| t.elapsed() < Duration::from_secs(5))) {
                        if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                            let int = |f: &str| p.value.get(f).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as i32;
                            if let Some(text) = crate::feedback::texts().and_then(|t| t.get(int("CategoryId"), int("MessageId"))).filter(|t| !t.is_empty()) {
                                pending_mission_status = Some(text.to_string());
                            }
                        }
                    }
                    if let Some(m) = Message::parse(&frame).filter(|m| m.kind == 0x5c436609) {
                        mission_asked = None;
                        if let Ok(p) = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data) {
                            let item = |low: i64| {
                                let template = gear_db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: low as u32 }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d))?;
                                let icon = template.icon().and_then(|id| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|t| std::sync::Arc::new(crate::gear::Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })));
                                Some((template.name, icon))
                            };
                            let icon = |id: u32| gear_textures.get(mao_formats::item::ICON_TYPE, id).map(|t| std::sync::Arc::new(crate::gear::Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba }));
                            let mut offers = crate::mission::offers_from(&p.value, &item, &icon);
                            // (each offer's place by its name, for the window's lines and the helper's lists)
                            for offer in &mut offers {
                                offer.playfield = names.get(&offer.destination.0).cloned().unwrap_or_default();
                            }
                            if !offers.is_empty() {
                                pending_missions = Some(offers);
                            } else {
                                // (GUI.dll FUN_100d0e71 says so in the window; in this client's words)
                                pending_mission_status = Some("The terminal has no missions just now; try again later.".into());
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
                        if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                            for e in entries.iter().filter(|e| e.slot >= inventory::BACKPACK_FIRST) {
                                eprintln!("combat: inventory place {:#x}: {:x} {}", e.slot, e.low_id, template_name(e.low_id).unwrap_or_default());
                            }
                        }
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
            // (more already here: handled first, unless a zoning is to be done, which clears the old playfield's state)
            if burst < BURST && state.arrival.is_none() {
                if let Ok(Some(frame)) = connection.try_receive() {
                    stash = Some(frame);
                    burst += 1;
                    continue;
                }
            }
            crate::prof::add("net-burst", Duration::from_micros(burst as u64));
            burst = 0;
            // The server refusing a move: it sends back the last place it took (a full report far from us that is none of
            // ours), and keeps to it: uses there are "too far away", so the character goes back to it.
            // (kept a minute, for the log when the server puts us back; only the last 4 s count as our own reports' echoes)
            while told.front().is_some_and(|t| t.1.elapsed() > Duration::from_secs(60)) {
                told.pop_front();
            }
            if let Some(p) = state.own_report.take() {
                let far = |a: [f32; 3], b: [f32; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
                if far(p, position) > 5.0 && !told.iter().any(|t| t.1.elapsed() < Duration::from_secs(4) && far(t.0, p) < 1.0) {
                    eprintln!("the server put us back at {p:?} (we were at {position:?}); our reports of the last minute:");
                    for (at, when) in &told {
                        eprintln!("  {:5.1}s ago ({:.2}, {:.2}, {:.2}){}", when.elapsed().as_secs_f32(), at[0], at[1], at[2], if far(*at, p) < 1.0 { "  <- where it put us" } else { "" });
                    }
                    position = p;
                    base = p[1];
                    airborne = false;
                    rise = 0.0;
                    jumping = false;
                    body_reset = true;
                }
            }
            // The server moved us within the playfield (`N3Teleport`, a generated mission's move): at the destination with its
            // rotation at once, as `n3TeleportIIR_t::Activate` does (`SetRelPosRot`; the camera put back behind the character,
            // FUN_10022b2d), nothing sent back.
            if let Some((p, q)) = state.own_teleport.take() {
                eprintln!("[{:6.1}] the server moved us to {p:?}", clock.elapsed().as_secs_f32());
                position = p;
                heading = 2.0 * q[1].atan2(q[3]);
                offset = ground_offset(&terrain, p);
                base = p[1];
                airborne = false;
                rise = 0.0;
                jumping = false;
                body_reset = true;
            }
            // Zoning: the server announces the new playfield and where we arrive.
            if let Some((playfield, arrival)) = state.arrival.take() {
                // (a playfield an `N3Teleport` stopped starts again: in the same playfield too, at the arrival point)
                if state.playfield_stopped.take().is_some() && playfield == current {
                    position = arrival;
                    base = arrival[1];
                    airborne = false;
                    rise = 0.0;
                    jumping = false;
                    body_reset = true;
                    state.entities.insert(own, (position, 0));
                    pending_camera = true;
                }
                if playfield != current {
                    // (a zoning: the loading screen until the new playfield is drawn; an arrival in the same playfield, as
                    // the playfield message at login, draws no new one, so it must not raise the screen)
                    crate::loading::begin(Duration::ZERO, crate::loading::ZONE_FADE_OUT);
                    eprintln!("zoning to playfield {playfield}");
                    // (nobody is followed into another playfield, and a walk begun in the old one is over: a target left
                    // behind was still followed, to wherever its number stood in the new playfield; seen live 2026-11-10)
                    following = None;
                    goto = None;
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
                    // (a new playfield makes the characters anew: their spell queues go and their state flags start again from
                    // stat 224, FUN_1004496c)
                    state.spell_queue = Default::default();
                    state.state_flags.clear();
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
                            (terrain, collision, teleportals, zones) = (area.terrain, Arc::new(area.collision), area.teleportals, area.zones);
                            interior = area.interior.clone();
                            room_state = Default::default();
                            pending_interior = Some(interior.clone());
                            rooms_shown = Vec::new();
                            state.set_static_props(crate::static_props(&thread_db, playfield));
                            state.prop_textures.extend(crate::static_prop_textures(&thread_db, playfield));
                            state.prop_anim.extend(crate::static_prop_anims(&thread_db, playfield).into_iter().map(|(k, (play, pos))| (k, (play, pos, Instant::now()))));
                            state.approach_doors = crate::approach_doors(&thread_db, playfield);
                            state.animated_statels = Arc::new(area.animated);
                            state.table = crate::table_dynels(&thread_db, playfield);
                            state.mission_kinds = crate::mission_terminals(&thread_db, playfield);
                            let mut scene = area.scene;
                            if terrain.is_none() && crate::load_rooms(&thread_db, playfield).is_none() {
                                // An interior: the floor isn't readable, so lay a plain one at the arrival height.
                                scene.parts.push(floor_part(arrival[0], arrival[2], arrival[1] - 0.03));
                                scene.instances.push((scene.parts.len() - 1, model::IDENTITY));
                            }
                            pending_scene = Some(scene);
                            booths = area.booths;
                            pending_falls = Some(area.falls);
                            areas = area.areas;
                            state.grid = is_grid(&areas);
                            base_environment = area.base_environment;
                            booth_playfield = playfield;
                            pending_labels = Some(booth_labels(playfield, &booths, &destinations, &names));
                            goto = None;
                        }
                        Err(e) => {
                            eprintln!("can't draw playfield {playfield}: {e}");
                            (terrain, collision) = (None, Default::default());
                            pending_scene = Some(Model::default());
                            pending_falls = Some(Vec::new());
                            booths = Vec::new();
                            pending_labels = Some(Vec::new());
                        }
                    }
                    position = arrival;
                    offset = ground_offset(&terrain, arrival);
                    map = crate::automap::MapBuilder::new(terrain.as_deref(), arrival, playfield).with_ground(ground_of(terrain.as_deref()));
                    pending_map = None;
                    base = arrival[1];
                    airborne = false;
                    rise = 0.0;
                    state.entities.insert(own, (position, 0));
                    mode_told = false;
                    state.swimming.clear();
                    body_reset = true;
                    jumping = false;
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
            // (dead: no moving, as the original clears its movement input mode, until health comes back or a playfield is entered)
            if state.own_dead && state.stats.get(&27).is_some_and(|&h| h > state.dead_health.max(0)) {
                state.own_dead = false;
                state.dead.remove(&own);
                crate::whiteout::clear(false);
            }
            // (the playfield stopped for a zoning: the character stands still until the playfield message; after 30 s, the
            // `TeleportTrier_t`'s time, it is let go, a safety of this client's: the original stays stopped)
            if state.playfield_stopped.is_some_and(|t| t.elapsed() > Duration::from_secs(30)) {
                eprintln!("no playfield message 30 s after the server's teleport: moving again");
                state.playfield_stopped = None;
            }
            // (stunned by a spell list: no movement action is started and the mouse does not turn the character while neither
            // state flag 2 nor 4 is set, FUN_1006bd94 and `N3Msg_MouseMovement`)
            state.tick_spells();
            if state.own_dead || state.playfield_stopped.is_some() || state.own_stunned() {
                (keys.forward, keys.back, keys.jump, keys.turn, keys.strafe, keys.face) = (false, false, false, 0, 0, None);
                goto = None;
            }
            // (dead: nobody is followed either; the attack and use keys do nothing, below)
            if state.own_dead {
                following = None;
            }
            // Vicinity speech: a text message to the server, at the current target (a character's; none for an object or nobody).
            // The pet window: the pets as they are now, and its buttons' commands to one pet (`N3Msg_SendPetCommand`).
            crate::pets::set(state.pets.values().map(|&pet| crate::pets::Pet {
                id: pet,
                name: state.names.get(&pet.1).map(|n| n.0.clone()).unwrap_or_default(),
                health: state.health.get(&pet.1).copied().unwrap_or((0, 0)),
                in_play: state.entities.contains_key(&pet.1),
                selected: target == Some(pet.1),
                family: state.infos.get(&pet.1).and_then(|i| i.family),
                flash: state.pet_flashes.get(&pet.1).map_or(-1, |&(count, since)| crate::pets::flash_value(count, since.elapsed())),
            }).collect());
            // The tradeskill window's source, target and Build (`CharacterAction` 0xdc, 0xdd, 0xde).
            // The research window's request (`3115534d`) and its sliders' and goals' `SetStat`s (263 to 266).
            for command in crate::research::take_commands() {
                let _ = match command {
                    crate::research::Command::Request => connection.send(&n3::research_request(own)),
                    crate::research::Command::SetStat(stat, value) => connection.send(&n3::set_stat(own, stat, value)),
                };
            }
            for command in crate::tradeskill::take_commands() {
                let item = |place: Option<u32>| place.map_or((0, 0), |p| (0x68, p));
                let _ = match command {
                    crate::tradeskill::Command::Source(place) => connection.send(&n3::tradeskill(own, 0xdc, (0, 0), item(place))),
                    crate::tradeskill::Command::Target(place) => connection.send(&n3::tradeskill(own, 0xdd, (0, 0), item(place))),
                    crate::tradeskill::Command::Build(ql) => connection.send(&n3::tradeskill(own, 0xde, (0, ql.max(0) as u32), (0, 0))),
                };
            }
            for (pet, command) in crate::pets::take_commands() {
                let _ = connection.send(&n3::pet_command_to(own, command, pet));
            }
            // The team window's answers.
            // (the server tells the other members' health and nano, `TeamMemberInfo`; none came for oneself in the live
            // team of 2026-11-10, so one's own bars are filled from the character's own stats: this client's reading, the
            // original's member view was not read for it)
            if let Some((npc, when, false)) = npc_asked
                && when.elapsed() > Duration::from_secs(2)
            {
                // (not when the NPC answered with a line of its own instead of its window: it is not waiting on an old
                // chat, and asked again it says the line again)
                let name = state.names.get(&npc.1).map(|n| n.0.clone()).unwrap_or_default();
                let spoke = NPC_SPOKE.lock().ok().and_then(|s| s.clone()).is_some_and(|(line, at)| at > when && !name.is_empty() && line.split('\n').any(|part| part.starts_with(&name)));
                if !spoke {
                    let _ = connection.send(&n3::npc_chat_close(own, npc));
                    let _ = connection.send(&n3::npc_chat_open(own, npc));
                }
                npc_asked = Some((npc, when, true));
            }
            crate::team::with(|t| {
                t.own = own;
                if let Some(me) = t.members.iter_mut().find(|m| m.id == own) {
                    let stat = |k: u32| state.stats.get(&k).copied().unwrap_or(0);
                    (me.health, me.nano) = ((stat(27), stat(1)), (stat(214), stat(221)));
                }
            });
            for command in crate::team::take_commands() {
                let _ = match command {
                    crate::team::Command::Reply(who, join) => connection.send(&n3::team_reply(own, who, join)),
                    crate::team::Command::Leave => connection.send(&n3::action(own, n3::LEAVE_TEAM)),
                    // A member's view (GUI.dll FUN_1007a23a): clicked, the member is the target; its menu's info, and
                    // the leader's two (Gamecode sends neither for oneself).
                    crate::team::Command::Select(who) => {
                        target = Some(who);
                        let _ = connection.send(&n3::select_target(own, who));
                        connection.send(&n3::set_target(own, who))
                    }
                    crate::team::Command::Info(who) => {
                        state.inspecting = Some(who);
                        connection.send(&n3::info_request(own, who))
                    }
                    crate::team::Command::Lead(who) if who != own => connection.send(&n3::team_leader(own, who)),
                    crate::team::Command::Kick(who) if who != own => connection.send(&n3::team_kick(own, who)),
                    crate::team::Command::Lead(_) | crate::team::Command::Kick(_) => Ok(()),
                    // The Recruit button (GUI.dll FUN_100782b2): the target is asked to join, with a line saying so; no
                    // target and oneself each have their text; one already in the team is not asked (Gamecode
                    // `N3Msg_TeamJoinRequest` sends nothing for a character marked as a team mate).
                    crate::team::Command::Invite => {
                        let text = |category: i32, name: &str, own_words: &str| crate::gui::xml::ldb_named(category, name, own_words);
                        let line = match target {
                            None => Some(text(10000, "MustSelectChar2Join", "Select the character to ask first.")),
                            Some(t) if t == own => Some(text(10000, "CantInviteSelf", "You cannot ask yourself.")),
                            Some(t) if crate::team::with(|team| team.members.iter().any(|m| m.id == t)) => None,
                            Some(t) => {
                                let _ = connection.send(&n3::team_invite(own, t));
                                let name = state.names.get(&t).map(|n| n.0.clone()).or_else(|| state.infos.get(&t).map(|i| i.name.clone())).unwrap_or_default();
                                Some(format!("{}{name}", text(100, "JoinTeamRequestSentTo", "Asked to join your team: ")))
                            }
                        };
                        if let Some(line) = line {
                            state.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, line));
                        }
                        Ok(())
                    }
                };
            }
            // Skills raised in the Skills window.
            for raises in crate::skills::take_raises() {
                if !raises.is_empty() {
                    let _ = connection.send(&n3::raise_skills(own, &raises));
                }
            }
            // A skill reset confirmed there (or all of them: 0).
            for stat in crate::skills::take_resets() {
                eprintln!("[{:6.1}] skill reset {stat}", clock.elapsed().as_secs_f32());
                let _ = connection.send(&n3::reset_skill(own, stat));
            }
            // (the claims: asked for when the daily rewards' window goes, and by the menu's "Pending Rewards". The original
            // holds either back while its lock 0x8c runs, FUN_1006414f, and says so; here the server is asked each time.)
            // `/assist` (`N3Msg_AssistFight`, Gamecode 0x100274xx): the one meant has to be in a fight
            // (`Feedback_TargetIsNotInFight` else); its opponent is selected and attacked. A name is one of the
            // characters here (`N3Msg_NameToID`), else "Can't find target >name<."
            for name in std::mem::take(&mut *ASSISTS.lock().unwrap_or_else(|e| e.into_inner())) {
                let who = match &name {
                    Some(n) => state.names.iter().find(|(_, v)| v.0.eq_ignore_ascii_case(n)).map(|(id, _)| *id),
                    None => target,
                };
                match (who, &name) {
                    (None, Some(n)) => pending_messages.push(format!("Can't find target >{n}<.")),
                    (None, None) => {}
                    (Some(who), _) => match state.fighting.get(&who).copied().filter(|&t| t != 0 && t != own) {
                        None => pending_messages.push(crate::feedback::plain("Feedback_TargetIsNotInFight")),
                        Some(opponent) => {
                            target = Some(opponent);
                            let _ = connection.send(&n3::select_target(own, opponent));
                            let _ = connection.send(&n3::set_target(own, opponent));
                            let _ = connection.send(&n3::attack(own, opponent));
                        }
                    },
                }
            }
            for check in std::mem::take(&mut *CLAIM_REQUESTS.lock().unwrap_or_else(|e| e.into_inner())) {
                eprintln!("[{:6.1}] claims asked for ({})", clock.elapsed().as_secs_f32(), if check { "the check for what is waiting" } else { "what was claimed" });
                let _ = connection.send(&n3::claims(own, check));
            }
            // (the chat window's number is 0 here, as for the server commands below)
            for (action, value) in std::mem::take(&mut *TEXT_COMMANDS.lock().unwrap_or_else(|e| e.into_inner())) {
                eprintln!("[{:6.1}] text command {action:#x} ({value})", clock.elapsed().as_secs_f32());
                let _ = connection.send(&n3::text_command(own, action, 0, value));
            }
            for (command, targeted, text) in std::mem::take(&mut *ORG_COMMANDS.lock().unwrap_or_else(|e| e.into_inner())) {
                let at = target.filter(|_| targeted).map_or((0, 0), |t| (state.types.get(&t).copied().unwrap_or(n3::CHARACTER), t));
                eprintln!("[{:6.1}] organization command {command:#x} at {:x}:{:x} {text:?}", clock.elapsed().as_secs_f32(), at.0, at.1);
                let _ = connection.send(&n3::org_command(own, command, at, 0, &text, false));
            }
            for text in std::mem::take(&mut *SERVER_COMMANDS.lock().unwrap_or_else(|e| e.into_inner())) {
                let at = target.map_or((0, 0), |t| (state.types.get(&t).copied().unwrap_or(n3::CHARACTER), t));
                eprintln!("[{:6.1}] server command {text:?}", clock.elapsed().as_secs_f32());
                let _ = connection.send(&n3::server_command(own, 0, at, &text));
            }
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
            // (a test script's target: the character with that id, selected as a click on it would)
            if raw.select_id.0 != select_id_seen {
                select_id_seen = raw.select_id.0;
                let id = raw.select_id.1;
                target = Some(id);
                let _ = connection.send(&n3::select_target(own, id));
                let _ = connection.send(&n3::set_target(own, id));
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
                let kind = target.and_then(|t| state.types.get(&t).copied());
                let table = target.and_then(|t| state.table.iter().find(|d| Some(d.0) == kind && d.1 == t));
                match target {
                    // (a built-in terminal or booth: its item template's description, `itemid://` in the original)
                    Some(_) if table.is_some() => {
                        let template = table.map_or(0, |d| d.3);
                        state.info_urls.push(format!("itemref://{template}/{template}/1"));
                    }
                    Some(t) if state.objects.contains_key(&t) => {
                        let name = state.names.get(&t).map_or_else(|| "Object".to_string(), |n| n.0.clone());
                        state.info_urls.push(format!("text://<font color=#ffffff>{name}</font>"));
                    }
                    Some(t) => {
                        state.inspecting = Some(t);
                        let _ = connection.send(&n3::info_request(own, t));
                    }
                    None => pending_messages.push("Nothing targeted to inspect (Tab selects)".into()),
                }
            }
            while autoplay.first().is_some_and(|(t, _)| autoplay_clock.elapsed().as_secs_f32() >= *t) {
                let (_, what) = autoplay.remove(0);
                eprintln!("autoplay: {what} (target {target:?})");
                match what.as_str() {
                    "tab" => seen[0] = seen[0].wrapping_sub(1),
                    "attack" => seen[4] = seen[4].wrapping_sub(1),
                    "use" => seen[6] = seen[6].wrapping_sub(1),
                    "camp" => camp_seen = camp_seen.wrapping_sub(1),
                    // `who`: the characters about, nearest first; `target=<part of a name>`: the nearest of that name
                    // becomes the target, as a click on it would make it
                    // `npcclose`: tells the target NPC its chat is closed (a client that went with the window open leaves
                    // the NPC waiting, and it answers no new opening until then; seen live 2026-11-10)
                    // (`answer=<n>`: the open NPC chat's answer of that place, from 0, as a click on it; nothing when no chat is
                    // open, so it can be said again and again until the NPC talks)
                    w if w.starts_with("answer=") => match (chat_npc, w["answer=".len()..].parse::<u32>()) {
                        (Some(npc), Ok(k)) => {
                            eprintln!("autoplay: answer {k} to {npc:x?}");
                            let _ = connection.send(&n3::npc_chat_answer(own, npc, k));
                        }
                        _ => eprintln!("autoplay: no chat open"),
                    },
                    // (`talk`: uses the target unless an NPC's chat is open already, so it can be said again and again while
                    // the NPC is busy)
                    "talk" => {
                        if chat_npc.is_none() {
                            seen[6] = seen[6].wrapping_sub(1);
                        }
                    }
                    "npcclose" => {
                        if let Some(t) = target {
                            let _ = connection.send(&n3::npc_chat_close(own, (n3::CHARACTER, t)));
                        }
                    }
                    // `what`: the things about that can be used (terminals, doors and the like), nearest first
                    "what" => {
                        let mut about: Vec<(f32, String)> = state.table.iter().map(|d| (((d.2[0] - position[0]).powi(2) + (d.2[2] - position[2]).powi(2)).sqrt(), format!("({:#x}, {:#x}) {:?} at ({:.1}, {:.1}, {:.1})", d.0, d.1, d.4, d.2[0], d.2[1], d.2[2]))).collect();
                        about.sort_by(|a, b| a.0.total_cmp(&b.0));
                        for (far, what) in about.iter().take(30) {
                            eprintln!("autoplay: {far:6.1} m  {what}");
                        }
                        eprintln!("autoplay: {} things", state.table.len());
                        // (and the objects the server tells of: chests, items lying about and the like)
                        let mut lying: Vec<(f32, String)> = state.objects.iter().map(|(id, p)| (((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt(), format!("({:#x}, {id:#x}) {:?} at ({:.1}, {:.1}, {:.1})", state.types.get(id).copied().unwrap_or(0), state.names.get(id).map_or("", |n| n.0.as_str()), p[0], p[1], p[2]))).collect();
                        lying.sort_by(|a, b| a.0.total_cmp(&b.0));
                        for (far, what) in lying.iter().take(40) {
                            eprintln!("autoplay: {far:6.1} m  object {what}");
                        }
                    }
                    "who" => {
                        let mut about: Vec<(f32, u32, String)> = state.entities.iter().filter(|(id, _)| **id != own).map(|(id, (p, _))| (((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt(), *id, state.names.get(id).map_or(String::new(), |n| n.0.clone()))).collect();
                        about.sort_by(|a, b| a.0.total_cmp(&b.0));
                        for (far, id, name) in about.iter().take(80) {
                            let at = state.entities.get(id).map_or([0.0; 3], |e| e.0);
                            eprintln!("autoplay: {far:6.1} m  {id:x} ({id})  {name:?}{} at ({:.1}, {:.1}, {:.1})", if state.infos.get(id).is_some_and(|i| i.is_player()) { " (player)" } else { "" }, at[0], at[1], at[2]);
                        }
                    }
                    // (`kill=<part of a name>`: the same, a living one only, then followed and attacked unless it is being
                    // fought already)
                    w if w.starts_with("target=") || w.starts_with("kill=") => {
                        let kill = w.starts_with("kill=");
                        let name = w[w.find('=').map_or(0, |k| k + 1)..].to_ascii_lowercase();
                        let far = |id: &u32| state.entities.get(id).map_or(f32::MAX, |(p, _)| (p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2));
                        let alive = |id: &u32| !kill || (!state.dead.contains_key(id) && !state.corpses.contains_key(id) && state.health.get(id).is_none_or(|h| h.0 > 0));
                        match state.names.iter().filter(|(id, n)| **id != own && n.0.to_ascii_lowercase().contains(&name) && state.entities.contains_key(id) && alive(id)).map(|(id, _)| *id).min_by(|a, b| far(a).total_cmp(&far(b))) {
                            Some(id) => {
                                target = Some(id);
                                let _ = connection.send(&n3::select_target(own, id));
                                let _ = connection.send(&n3::set_target(own, id));
                                eprintln!("autoplay: target {id:x} {:?}, {:.1} m away", state.names.get(&id).map(|n| n.0.as_str()), far(&id).sqrt());
                                if kill {
                                    following = Some(id);
                                    if state.fighting.get(&own) != Some(&id) {
                                        seen[4] = seen[4].wrapping_sub(1);
                                    }
                                }
                            }
                            None => eprintln!("autoplay: nobody named {name:?} about"),
                        }
                    }
                    "x" => seen[5] = seen[5].wrapping_sub(1),
                    "walk" => walk_toggle_seen = walk_toggle_seen.wrapping_sub(1),
                    "jump" => autoplay_jump = true,
                    // (`corpse`: the nearest remains within 10 m are used, which opens them; `loot` then takes what is in them)
                    "corpse" => {
                        let near = |p: &[f32; 3]| ((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt();
                        match state.objects.iter().filter(|(id, p)| state.types.get(id) == Some(&OBJECT_TYPE) && near(p) < 10.0).min_by(|a, b| near(a.1).total_cmp(&near(b.1))).map(|(id, p)| (*id, near(p))) {
                            Some((id, far)) => {
                                eprintln!("autoplay: remains {id:#x} {far:.1} m away");
                                uses += 1;
                                let _ = connection.send(&n3::use_object(own, uses, (OBJECT_TYPE, id)));
                            }
                            None => eprintln!("autoplay: no remains near"),
                        }
                    }
                    "loot" => {
                        if let Some(l) = state.loot.as_ref() {
                            eprintln!("autoplay: taking {} things", l.entries.len());
                            for e in &l.entries {
                                let _ = connection.send(&n3::take_from_container(own, l.handle, e.slot));
                            }
                        }
                    }
                    "follow" => following = target.filter(|t| state.entities.contains_key(t)),
                    // `usenear=<name>`: target the nearest table dynel with the name (as a click on it) and use it
                    w if w.starts_with("usenear=") => {
                        let name = w["usenear=".len()..].to_ascii_lowercase();
                        let near = |p: &[f32; 3]| ((p[0] - position[0]).powi(2) + (p[1] - position[1]).powi(2) + (p[2] - position[2]).powi(2)).sqrt();
                        match state.table.iter().filter(|d| d.4.to_ascii_lowercase().contains(&name)).min_by(|a, b| near(&a.2).total_cmp(&near(&b.2))).map(|d| (d.0, d.1, d.2, d.4.clone())) {
                            Some((kind, id, at, label)) => {
                                eprintln!("autoplay: {label:?} ({kind:#x}, {id:#x}) at {at:?}, {:.2} m from us at {position:?}", near(&at));
                                state.types.insert(id, kind);
                                state.names.insert(id, (label, false));
                                target = Some(id);
                                seen[6] = seen[6].wrapping_sub(1);
                            }
                            // (else one the server told of: a chest, a thing lying about)
                            None => match state.objects.iter().filter(|(id, _)| state.names.get(id).is_some_and(|n| n.0.to_ascii_lowercase().contains(&name))).min_by(|a, b| near(a.1).total_cmp(&near(b.1))).map(|(id, at)| (*id, *at)) {
                                Some((id, at)) => {
                                    eprintln!("autoplay: object {id:#x} at {at:?}, {:.2} m from us at {position:?}", near(&at));
                                    target = Some(id);
                                    seen[6] = seen[6].wrapping_sub(1);
                                }
                                None => eprintln!("autoplay: no {name:?} in the table or among the objects"),
                            },
                        }
                    }
                    _ => {
                        following = None;
                        goto = None;
                    }
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
                        // (the one fought is followed into reach: 3.5 m is past a fist's, where the server only says "too far")
                        let near = if state.fighting.get(&own) == Some(&id) { 2.0 } else { 3.5 };
                        // (stopped, it starts again only a metre past that: else it would start and stop every step at the edge,
                        // the character now taking half a second to reach its speed)
                        if distance > near + if goto.is_some() { 0.0 } else { 1.0 } {
                            // (the progress watch only restarts when a walk starts: the target keeps moving)
                            if goto.is_none() {
                                (goto_best, goto_progress, detour, detours) = (f32::MAX, Instant::now(), None, 0);
                                if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                                    eprintln!("combat: following {id:x}: it is at ({:.2}, {:.2}), {distance:.2} m from ({:.2}, {:.2})", p[0], p[2], position[0], position[2]);
                                }
                            }
                            goto = Some([p[0], p[2]]);
                        } else {
                            if goto.is_some() && std::env::var_os("MAO_COMBAT_LOG").is_some() {
                                eprintln!("combat: following {id:x}: near enough at {distance:.2} m, at ({:.2}, {:.2})", position[0], position[2]);
                            }
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
            // Go (the Whom-Pah window) and `/waypoint`: walk to a place; any movement key of the player's own stops it.
            if raw.goto.0 != goto_seen {
                goto_seen = raw.goto.0;
                goto = Some(raw.goto.1);
                // (the later order wins: a target followed until now is let go, or the walk never left its side)
                following = None;
                (goto_best, goto_progress, detour, detours) = (f32::MAX, Instant::now(), None, 0);
                pending_messages.push("Walking there (any movement key stops)".into());
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
            // `/camp` (`FlowControlModule_t::StartQuitToLoginMessage` → `N3Msg_StartCamping`, Gamecode 0x1001c9c6): not while
            // anyone fights the character ("Feedback_CantLogOutInAFight"); an attack of one's own stops; the character sits and
            // the server is told to log it out (CharacterAction 0x78), and the logout bar starts (`CampStartedMessage`).
            if keys.camp != camp_seen {
                camp_seen = keys.camp;
                if state.fighting.iter().any(|(&who, &whom)| whom == own && who != own) {
                    pending_messages.push(crate::feedback::plain("Feedback_CantLogOutInAFight"));
                } else if crate::ui::CAMP.lock().unwrap_or_else(|e| e.into_inner()).is_none() {
                    if state.fighting.contains_key(&own) {
                        let _ = connection.send(&n3::stop_fight(own));
                    }
                    // (one already seated stays so: the original sits only a character not sitting)
                    if !sitting {
                        sitting = true;
                        send(&mut connection, n3::movement::SIT, heading, position, &mut sent);
                        state.seated.insert(own);
                    }
                    let _ = connection.send(&n3::action(own, n3::LOGOUT));
                    *crate::ui::CAMP.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
                    pending_messages.push(crate::feedback::gui("LogoutStartedXseconds", &[mao_formats::ldb::Arg::Int(30)]));
                }
            }
            // Targeting: Tab and friends step through the nearby ones, nearest first; F attacks.
            for (slot, count) in keys.commands.iter().enumerate() {
                let presses = count.wrapping_sub(seen[slot]);
                seen[slot] = *count;
                if presses == 0 {
                    continue;
                }
                // (dead: no attack and no use. The original's interface is told of the death, `DeadMessage`, and of the
                // life again, `AliveMessage`; the server went on taking a dead character's attacks in the live run of
                // 2026-11-11, which the original's player cannot send)
                if state.own_dead && matches!(slot, 4 | 6) {
                    continue;
                }
                if slot == 6 {
                    // E: use the target (talk to it, open it).
                    match target {
                        Some(t) => {
                            uses += 1;
                            let (kind, t) = state.used(state.types.get(&t).copied().unwrap_or(n3::CHARACTER), t);
                            if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                                let there = state.objects.get(&t).copied().or_else(|| state.entities.get(&t).map(|e| e.0));
                                eprintln!("combat: use ({kind:#x}, {t:#x}) at {there:?}, {:?} m away", there.map(|p| ((p[0] - position[0]).powi(2) + (p[1] - position[1]).powi(2) + (p[2] - position[2]).powi(2)).sqrt()));
                            }
                            // (an NPC is talked to: `KnubotOpenChatWindow`, as aosharp opens a conversation)
                            if kind == n3::CHARACTER && t != own && state.infos.get(&t).is_some_and(|i| !i.is_player()) {
                                let _ = connection.send(&n3::npc_chat_open(own, (kind, t)));
                                npc_asked = Some(((kind, t), Instant::now(), false));
                            } else {
                                let _ = connection.send(&n3::use_object(own, uses, (kind, t)));
                            }
                            state.door_used(t);
                            state.opened_by_use(&gear_db, kind, t);
                            pending_messages.push(format!("Using {}", state.names.get(&t).map_or("the target", |n| n.0.as_str())));
                        }
                        None => pending_messages.push("Nothing targeted to use (Tab selects)".into()),
                    }
                    continue;
                }
                if slot == 5 {
                    // X (`/sit`): sit down on the ground (`SwitchToSit`; a fight of one's own stops first), or stand up
                    // (`LeaveSit`), which also gives up a `/camp` under way. Sleeping or lounging (`WaitState` 0xf, 0x10),
                    // `N3Msg_SitToggle` (0x10028fcb) sends `CharacterAction` 0x57 instead, which only sits the character up.
                    if sitting && state.resting.remove(&own).is_some() {
                        let _ = connection.send(&n3::action(own, n3::STAND));
                    } else if sitting {
                        sitting = false;
                        send(&mut connection, n3::movement::LEAVE_SIT, heading, position, &mut sent);
                        state.stand_up(own);
                        if crate::ui::CAMP.lock().unwrap_or_else(|e| e.into_inner()).take().is_some() {
                            let _ = connection.send(&n3::action(own, n3::STOP_LOGOUT));
                            pending_messages.push(crate::feedback::gui("TimedLogoutAborted", &[]));
                        }
                    } else {
                        if state.fighting.contains_key(&own) {
                            let _ = connection.send(&n3::stop_fight(own));
                        }
                        sitting = true;
                        send(&mut connection, n3::movement::SIT, heading, position, &mut sent);
                        state.seated.insert(own);
                    }
                    continue;
                }
                if slot == 4 {
                    // The attack key (`N3Msg_DefaultAttack(selection, false)`): stop a fight, else attack the selection.
                    let kind = match target {
                        None => crate::combat::Target::Nothing,
                        Some(t) if t == own => crate::combat::Target::Yourself,
                        Some(t) if state.infos.contains_key(&t) => crate::combat::Target::Character { dead: state.dead.contains_key(&t) },
                        Some(_) => crate::combat::Target::Other,
                    };
                    let pending = attack_sent.is_some_and(|t| t.elapsed() < Duration::from_secs(3));
                    let decision = crate::combat::default_attack(state.fighting.contains_key(&own), false, pending, target, kind);
                    if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                        let there = target.and_then(|t| state.entities.get(&t).map(|e| e.0));
                        let distance = there.map(|p| ((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt());
                        eprintln!("combat: attack key: target {target:?} {kind:?}, fighting {:?}, pending {pending} -> {decision:?}; own at {position:?}, target at {there:?}, {distance:?} m", state.fighting.get(&own));
                    }
                    match decision {
                        crate::combat::Attack::Stop => {
                            let _ = connection.send(&n3::stop_fight(own));
                        }
                        crate::combat::Attack::Start(t) => {
                            crate::tips::trigger("OnStartingAttack", "");
                            let _ = connection.send(&n3::attack(own, t));
                            attack_sent = Some(Instant::now());
                        }
                        crate::combat::Attack::Refuse(name) => pending_messages.push(crate::feedback::plain(name)),
                    }
                    continue;
                }
                // Tab and friends (`N3Msg_GetCloseTarget`): the next character of the kind, by distance past the target's
                // (the living ones; a side the same as one's own is a friend's)
                let list: Vec<crate::combat::Candidate> = state
                    .infos
                    .iter()
                    .filter(|(id, _)| **id != own && !state.dead.contains_key(id))
                    .filter_map(|(id, i)| Some(crate::combat::Candidate { id: *id, position: state.entities.get(id)?.0, side: i.side }))
                    .collect();
                let own_side = state.stats.get(&33).copied().unwrap_or(0) as u8;
                if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                    eprintln!("combat: command {slot}: count {count}, {presses} presses");
                }
                let picked = crate::combat::close_target(position, own_side, target, &list, slot % 2 == 0, slot >= 2);
                if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                    eprintln!("combat: tab key {slot}: own side {own_side}, {} characters, from {target:?} to {picked:?} {:?}", list.len(), picked.and_then(|p| state.names.get(&p).map(|n| n.0.clone())));
                }
                let Some(id) = picked else { continue };
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
            while let Ok(command) = mission_log_rx.try_recv() {
                match command {
                    crate::missionlog::Command::Delete(id) => {
                        let _ = connection.send(&n3::quest_delete(own, id));
                    }
                }
            }
            while let Ok(command) = npc_chat_rx.try_recv() {
                match command {
                    crate::npcchat::Command::Answer(npc, k) => {
                        let _ = connection.send(&n3::npc_chat_answer(own, npc, k));
                    }
                    crate::npcchat::Command::Describe(npc) => {
                        let _ = connection.send(&n3::npc_chat_describe(own, npc));
                    }
                    crate::npcchat::Command::StartTrade(npc) => {
                        let _ = connection.send(&n3::npc_trade_start(own, npc));
                    }
                    crate::npcchat::Command::TradeItem(npc, slot, add) => {
                        let _ = connection.send(&n3::npc_trade_item(own, npc, if add { 0 } else { 1 }, slot));
                    }
                    crate::npcchat::Command::FinishTrade(npc, credits, accept) => {
                        let _ = connection.send(&n3::npc_trade_finish(own, npc, credits, accept));
                    }
                    crate::npcchat::Command::Close(npc) => {
                        let _ = connection.send(&n3::npc_chat_close(own, npc));
                        if chat_npc == Some(npc) {
                            chat_npc = None;
                        }
                    }
                }
            }
            while let Ok(command) = shop_rx.try_recv() {
                match command {
                    crate::shop::ShopCommand::Close => {
                        let _ = connection.send(&n3::trade_decline(own));
                        shop_booth = None;
                        player_trade = None;
                    }
                    // (a player trade: `N3Msg_TradeSetCash`, action 7 with the credits second; `N3Msg_TradeConfirm`, action 3)
                    crate::shop::ShopCommand::Credits(credits) => {
                        let _ = connection.send(&n3::trade(own, 7, [0, credits.clamp(0, u32::MAX as i64) as u32, 0, 0]));
                    }
                    crate::shop::ShopCommand::Confirm => {
                        let _ = connection.send(&n3::trade(own, 3, [0, 0, 0, 0]));
                    }
                    // As captured: a stock item put in (action 5: the booth, 0x6f, the stock index); the echo lists it.
                    crate::shop::ShopCommand::Buy(index) => match shop_booth {
                        Some(booth) => {
                            let _ = connection.send(&n3::trade(own, 5, [booth.0, booth.1, 0x6f, index as u32]));
                        }
                        None => pending_shop_status = Some("No open trade to buy in.".into()),
                    },
                    // As captured: one of your own items put in (action 5: yourself, the inventory page 0x68, the slot).
                    crate::shop::ShopCommand::Sell(slot) => match shop_booth.or(player_trade.map(|p| (n3::CHARACTER, p))) {
                        Some(_) => {
                            let _ = connection.send(&n3::trade(own, 5, [0xc350, own, 0x68, slot]));
                        }
                        None => pending_shop_status = Some("No open trade to sell in.".into()),
                    },
                    // (action 6, as captured: the item as it went in)
                    crate::shop::ShopCommand::Remove(item) => {
                        let _ = connection.send(&n3::trade(own, 6, item));
                    }
                    crate::shop::ShopCommand::Accept => {
                        let _ = connection.send(&n3::trade(own, 1, [0, 0, 0, 0]));
                    }
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
            // The item finder's "Look into my bags": each bag not seen into yet is used, half a second apart, and kept
            // without a window (`silent_bags`).
            if crate::itemindex::take_scan() {
                index_scan = state.worn.iter().filter(|e| e.is_bag() && e.slot >= inventory::BACKPACK_FIRST && !state.bags.contains_key(&e.container)).map(|e| (e.slot, e.container)).collect();
            }
            if index_scanned.elapsed() > Duration::from_millis(500)
                && let Some((place, identity)) = index_scan.pop_front()
            {
                index_scanned = Instant::now();
                if state.worn.iter().any(|e| e.slot == place && e.container == identity) && !state.bags.contains_key(&identity) {
                    state.silent_bags.insert(identity);
                    uses += 1;
                    let _ = connection.send(&n3::use_object(own, uses, (0x68, place)));
                }
            }
            // The item index: what the character carries is written down when it has changed (looked at every few
            // seconds), so that the item finder knows it while another character is played.
            if index_checked.elapsed() > Duration::from_secs(3) {
                index_checked = Instant::now();
                let bank = state.loot.as_ref().filter(|l| l.identity.0 == crate::loot::BANK);
                let signature = {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    for e in state.worn.iter().chain(state.bags.values().flat_map(|b| b.entries.iter())).chain(bank.into_iter().flat_map(|b| b.entries.iter())) {
                        (e.slot, e.container, e.low_id, e.high_id, e.quality, e.count).hash(&mut h);
                    }
                    (state.bags.len(), bank.is_some()).hash(&mut h);
                    h.finish()
                };
                if signature != index_signature && !state.worn.is_empty() {
                    index_signature = signature;
                    let mut item = |e: &inventory::Entry| crate::itemindex::Item {
                        low: e.low_id,
                        high: e.high_id,
                        quality: e.quality,
                        count: u32::from(e.count).max(1),
                        name: index_names.entry(e.low_id).or_insert_with(|| template_name(e.low_id).unwrap_or_else(|| format!("item #{}", e.low_id))).clone(),
                    };
                    let worn: Vec<crate::itemindex::Item> = state.worn.iter().filter(|e| e.slot < inventory::BACKPACK_FIRST).map(&mut item).collect();
                    let carried: Vec<crate::itemindex::Item> = state.worn.iter().filter(|e| e.slot >= inventory::BACKPACK_FIRST).map(&mut item).collect();
                    let bags: Vec<crate::itemindex::Bag> = state
                        .worn
                        .iter()
                        .filter(|e| e.is_bag() && e.slot >= inventory::BACKPACK_FIRST)
                        .map(|e| crate::itemindex::Bag { place: e.slot, template: e.low_id, name: item(e).name, contents: state.bags.get(&e.container).map(|b| b.entries.iter().map(&mut item).collect()) })
                        .collect();
                    let banked: Option<Vec<crate::itemindex::Item>> = bank.map(|b| b.entries.iter().map(&mut item).collect());
                    let dimension = std::env::var("MAO_SERVER").unwrap_or_else(|_| session::RUBI_KA.to_string());
                    if let Some(name) = state.names.get(&own).map(|n| n.0.clone()) {
                        crate::itemindex::record(&name, &dimension, worn, carried, bags, banked);
                    }
                }
            }
            // The mission window: ask the nearest booth for missions, or accept an offer.
            if raw.mission.0 != mission_seen {
                mission_seen = raw.mission.0;
                let cmd = raw.mission.1;
                if cmd.what == 1 {
                    // (the booth last used, else the nearest of the playfield's own (its dynel table's `0xdac1`s))
                    let near = |p: &[f32; 3]| ((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt();
                    let nearest = mission_booth
                        .and_then(|b| state.table.iter().find(|d| d.0 == 0xdac1 && d.1 == b))
                        .filter(|d| near(&d.2) < 40.0)
                        .or_else(|| state.table.iter().filter(|d| d.0 == 0xdac1 && near(&d.2) < 40.0).min_by(|a, b| near(&a.2).total_cmp(&near(&b.2))))
                        .map(|d| (near(&d.2), d.1));
                    // (what the original's button checks before it asks: the credits, FUN_100d09cb; and the terminal's
                    // own rule again, for a window left open while the team went)
                    let carried = state.worn.iter().filter(|e| e.slot >= inventory::BACKPACK_FIRST).count();
                    let refused = crate::mission::request_refused(state.stats.get(&54).copied().unwrap_or(1), state.stats.get(&61).copied().unwrap_or(i64::MAX))
                        .or_else(|| crate::mission::slots_refused(carried))
                        .or_else(|| nearest.and_then(|(_, serial)| crate::mission::use_refused(state.mission_kinds.get(&serial).copied(), crate::team::with(|t| !t.members.is_empty()))));
                    if let Some(text) = &refused {
                        crate::chrome::alert(text.clone());
                        pending_mission_status = Some(String::new());
                    }
                    match nearest.filter(|_| refused.is_none()) {
                        Some((_, serial)) => {
                            uses += 1;
                            let booth = state.used(0xdac1, serial);
                            send(&mut connection, movement::HEADING, heading, position, &mut sent);
                            let _ = connection.send(&n3::use_object(own, uses, booth));
                            // (the request names the terminal's own kind of mission: `MissionGenerateInfo_t`'s eighth
                            // number, which `N3Msg_GenerateMissions` 0x10016276 sends on. It was always 5, the clan
                            // terminal's of the capture, whatever the terminal.)
                            let scope = std::env::var("MAO_MISSION_SCOPE").ok().and_then(|v| v.parse().ok()).or_else(|| state.mission_kinds.get(&serial).copied()).unwrap_or(crate::mission::DEFAULT_SCOPE);
                            let _ = connection.send(&n3::quest_alternative(own, cmd.sliders, scope, booth));
                            mission_asked = Some(Instant::now());
                        }
                        None if refused.is_some() => {}
                        None => pending_mission_status = Some("No mission booth in reach (walk up to one).".into()),
                    }
                } else if cmd.what == 2 {
                    // (FUN_100d0c61: accepting wants the two free places too)
                    match crate::mission::slots_refused(state.worn.iter().filter(|e| e.slot >= inventory::BACKPACK_FIRST).count()) {
                        Some(text) => {
                            crate::chrome::alert(text);
                            pending_mission_status = Some(String::new());
                        }
                        None => {
                            let _ = connection.send(&n3::create_quest(own, cmd.mission));
                            pending_mission_status = Some("Mission requested; see the mission log.".into());
                        }
                    }
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
            // An item used on something in the world (`N3Msg_UseItemOnItem`: action 5 of the use message).
            if keys.use_on.0 != use_on_seen {
                use_on_seen = keys.use_on.0;
                uses += 1;
                let _ = connection.send(&n3::use_item_on(own, uses, keys.use_on.1, keys.use_on.2));
                if keys.use_on.1.0 == 0x68 {
                    state.start_item_timer(keys.use_on.1.1, 5);
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
                    // (a special attack the server listed that the catalogue does not name: its skill's stat, as the others)
                    _ if name == "that action" && crate::actions::number_of(id).is_some_and(|n| state.specials_told.as_ref().is_some_and(|s| s.contains(&n))) => match target {
                        Some(t) => {
                            let _ = connection.send(&n3::special_attack(own, t, crate::actions::number_of(id).unwrap_or(0)));
                        }
                        None => pending_messages.push("That attack needs a target (Tab selects one)".into()),
                    },
                    // (a perk action the server listed: `N3Msg_PerformSpecialAction`'s 0xc78e case, FUN_100427d6, a `UsePerk`)
                    _ if state.perk_actions.iter().any(|p| p.0 == id) => {
                        let &(_, number, code) = state.perk_actions.iter().find(|p| p.0 == id).unwrap();
                        let _ = connection.send(&n3::use_perk(own, number, code));
                    }
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
            // The AGG/DEF slider (GUI.dll FUN_1006f283 → `N3Msg_SetAggDef` → `N3Msg_SetStat(51)`): a `SetStat` to the server.
            if keys.aggdef.0 != aggdef_seen {
                aggdef_seen = keys.aggdef.0;
                let _ = connection.send(&n3::set_stat(own, 51, keys.aggdef.1.clamp(-100, 100)));
            }
            // (the web pages' headers, for the pages the menus open: kept up to date every few seconds)
            if headers_made.elapsed() > Duration::from_secs(3) {
                headers_made = Instant::now();
                let ticket = mao_net::session::last_ticket();
                let stats = &state.stats;
                crate::browser::set_game_headers(crate::browser::headers(own, ticket.as_ref(), state.server_id, &|s| stats.get(&s).copied().unwrap_or(0)));
            }
            // "Deposit to Market"'s Send (`N3Msg_SendMarketItem`): the items from the backpack and the credits.
            if keys.market.0 != market_seen {
                market_seen = keys.market.0;
                let d = keys.market.1;
                let items: Vec<(u32, u32)> = d.slots[..d.count as usize].iter().map(|&s| (0x68, s)).collect();
                let _ = connection.send(&n3::market_send(own, d.credits, &items));
                pending_messages.push(format!("Sent {} items and {} credits to the market.", items.len(), d.credits));
            }
            if state.fighting.contains_key(&own) {
                attack_sent = None;
            }
            // A click on something in the world selects it; a double click uses it.
            if keys.pick.0 != pick_seen {
                pick_seen = keys.pick.0;
                let (_, kind, id, double) = keys.pick;
                target = Some(id);
                // (a table dynel's instance can be another's too: the kind picked is the one meant)
                state.types.insert(id, kind);
                if let Some(d) = state.table.iter().find(|d| d.0 == kind && d.1 == id) {
                    let name = if d.4.is_empty() { format!("Terminal {:x}", id & 0xffff) } else { d.4.clone() };
                    state.names.insert(id, (name, false));
                }
                if kind == n3::CHARACTER {
                    let _ = connection.send(&n3::select_target(own, id));
                    let _ = connection.send(&n3::set_target(own, id));
                    let _ = connection.send(&n3::info_request(own, id));
                }
                // (the tips of selecting: a player, a creature or NPC by its name, anything else by its name)
                if id != own {
                    let name = state.names.get(&id).map(|n| n.0.clone()).unwrap_or_default();
                    match kind {
                        n3::CHARACTER if state.infos.get(&id).is_some_and(|i| i.is_player()) => crate::tips::trigger("OnSelectingPlayer", &name),
                        n3::CHARACTER => crate::tips::trigger("OnSelectingNPC", &name),
                        _ => crate::tips::trigger("OnSelecting", &name),
                    }
                }
                if double {
                    uses += 1;
                    // (the original reports where it stands with every use, a heading update: the server checks a
                    // terminal's reach against it)
                    send(&mut connection, movement::HEADING, heading, position, &mut sent);
                    let table_id = id;
                    let (kind, id) = state.used(state.types.get(&id).copied().unwrap_or(n3::CHARACTER), id);
                    // (a team's mission terminal used by someone in no team: an error box, and the terminal is not used,
                    // Gamecode FUN_10086b3e)
                    let refused = (kind == 0xdac1).then(|| crate::mission::use_refused(state.mission_kinds.get(&table_id).copied(), crate::team::with(|t| !t.members.is_empty()))).flatten();
                    if let Some(text) = &refused {
                        crate::chrome::alert(text.clone());
                    } else if kind == n3::CHARACTER && id != own && state.infos.get(&id).is_some_and(|i| !i.is_player()) {
                        let _ = connection.send(&n3::npc_chat_open(own, (kind, id)));
                        npc_asked = Some(((kind, id), Instant::now(), false));
                    } else if kind == n3::CHARACTER && id != own && state.infos.get(&id).is_some_and(|i| i.is_player()) && !state.fighting.contains_key(&own) {
                        // (another player: the default action on them is a trade, `N3Msg_DefaultActionOnDynel` ->
                        // `N3Msg_TradeStart`, an Open naming them; the server echoes it)
                        let _ = connection.send(&n3::trade(own, 0, [n3::CHARACTER, id, 0, 0]));
                    } else {
                        let _ = connection.send(&n3::use_object(own, uses, (kind, id)));
                    }
                    state.door_used(id);
                    // (a mail terminal opens the mail window on the inbox, as its own button does; a mission booth the
                    // mission window, which is the client's own: the server only echoes the use)
                    if kind == 0xc773 {
                        inbox_due = Some(Instant::now() + Duration::from_millis(900));
                        state.open_windows.push("mail");
                    }
                    if kind == 0xdac1 && refused.is_none() {
                        // (by its table id, which the request finds it by)
                        mission_booth = Some(table_id);
                        state.open_windows.push("missionselection");
                    }
                    state.opened_by_use(&gear_db, kind, id);
                    pending_messages.push(format!("Using {}", state.names.get(&id).map_or("the target", |n| n.0.as_str())));
                }
            }
            // (`/pet`: a command to all the pets)
            // `/duel`: a challenge to the targeted player (GUI.dll's handler says so in its own words when no player is targeted), or
            // an answer to one.
            if keys.duel.0 != duel_seen {
                duel_seen = keys.duel.0;
                let what = keys.duel.1;
                if what == 0 {
                    match target.filter(|t| *t != own && state.infos.get(t).is_some_and(|i| i.is_player())) {
                        Some(t) => {
                            let _ = connection.send(&n3::duel(own, Some(t), 0, 0));
                        }
                        None => pending_messages.push("A duel needs a player as the target.".into()),
                    }
                } else {
                    // (the high byte: the second parameter, 1 for the refusal the client sends by itself)
                    let _ = connection.send(&n3::duel(own, None, what & 0xff, what >> 8));
                }
            }
            // `/petduel` (GUI.dll FUN_100b8d78): a challenge to the targeted character, or an answer to one. The
            // original asks only that the target is a character; what comes of it is the server's to say.
            if keys.pet_duel.0 != pet_duel_seen {
                pet_duel_seen = keys.pet_duel.0;
                let what = keys.pet_duel.1;
                match (what, target.filter(|t| *t != own)) {
                    (0, None) => pending_messages.push("A pet duel needs a player as the target.".into()),
                    (_, t) => {
                        let _ = connection.send(&n3::pet_duel(own, t, what));
                    }
                }
            }
            if keys.pet.0 != pet_seen {
                pet_seen = keys.pet.0;
                let _ = connection.send(&n3::pet_command(own, keys.pet.1));
            }
            // (the perks window's Train and Untrain: the server's echo changes the owned ranks)
            if keys.train_perk.0 != train_perk_seen {
                train_perk_seen = keys.train_perk.0;
                let _ = connection.send(&n3::train_perk(own, keys.train_perk.1, keys.train_perk.2));
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
            // (a tradeskill combine: the server answers with the result and deletes what it used up)
            if keys.combine.0 != combine_seen {
                combine_seen = keys.combine.0;
                let _ = connection.send(&n3::use_item_on_item(own, (0x68, keys.combine.1), (0x68, keys.combine.2)));
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
                // (into the bank: `N3Msg_ContainerAddItem` on the bank's container, named with the character's instance)
                let id = if kind == crate::loot::BANK { own } else { id };
                let _ = connection.send(&n3::container_add_item(own, (kind, id), (0x68, from)));
            }
            // An item taken from the open container (or all of them).
            if keys.take.0 != take_seen {
                take_seen = keys.take.0;
                let (_, handle, place) = keys.take;
                let places: Vec<u32> = if place == u32::MAX { state.loot.iter().flat_map(|l| l.entries.iter().map(|e| e.slot)).collect() } else { vec![place] };
                // (the bank's items by its own way of naming them: only when it is the bank's window that asks, handle 0)
                let bank = state.loot.as_ref().filter(|l| l.identity.0 == crate::loot::BANK && l.handle == handle).map(|l| l.entries.clone());
                for place in places {
                    match &bank {
                        // (`N3Msg_MoveItemToInventory` with the item's place in the bank, `BankByRef` (0x69), as aosharp's
                        // clientless `Bank` names its items)
                        Some(entries) => {
                            let item = entries.iter().find(|e| e.slot == place).map_or((0x69, place), |e| (0x69, e.slot));
                            let _ = connection.send(&n3::move_to_inventory(own, item, 0x6f));
                        }
                        None => {
                            let _ = connection.send(&n3::take_from_container(own, handle, place));
                        }
                    }
                }
            }
            // An item dragged to another place: ask the server, and show the change at once; the server's refusal takes it back
            // (`World::moves_asked`).
            if keys.mv.0 != move_seen {
                move_seen = keys.mv.0;
                let (from, to) = (keys.mv.1, keys.mv.2);
                // (to a wear place only what may be worn there: the original asks itself before it asks the server)
                let refusal = state.worn.iter().find(|e| e.slot == from).filter(|_| to < inventory::BACKPACK_FIRST).and_then(|e| state.wear_refusal(e, to, &gear_db));
                if let Some(Some(name)) = refusal {
                    state.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, crate::feedback::plain(name)));
                }
                if refusal.is_some() {
                    // (the window shows the move as it is made: the list again puts the item back where it was)
                    pending_gear = Some(crate::gear::from_entries(&gear_db, &gear_textures, &state.worn, state.stats.get(&0x36).copied().unwrap_or(1)));
                } else if let Some(i) = state.worn.iter().position(|e| e.slot == from) {
                    let _ = connection.send(&n3::move_item(own, from, to));
                    let other = state.worn.iter().position(|e| e.slot == to);
                    if let Some(j) = other {
                        state.worn[j].slot = from;
                    }
                    state.worn[i].slot = to;
                    state.moves_asked.push((from, to, other.is_some()));
                    state.mods_dirty = true;
                    pending_gear = Some(crate::gear::from_entries(&gear_db, &gear_textures, &state.worn, state.stats.get(&0x36).copied().unwrap_or(1)));
                }
            }
            // A nano program to cast, at the target (not a usable object) or at oneself.
            if keys.cast.0 != cast_seen {
                cast_seen = keys.cast.0;
                'cast: {
                // (the client's own check first: FUN_1004fbe4)
                let stat = |s: u32| state.stats.get(&s).copied();
                let caster = crate::combat::Caster { dead: state.own_dead, swimming: state.swimming.contains(&own), falling: airborne, moving: keys.forward || keys.back || keys.strafe != 0 || goto.is_some(), nano: stat(214).unwrap_or(0) };
                let modifier = crate::combat::nano_cost_modifier(stat(318).unwrap_or(100), stat(4).unwrap_or(0), true);
                if let Some((name, number)) = crate::combat::cast_refusal(caster, nano_stat(keys.cast.1, 407).unwrap_or(0), modifier) {
                    if !name.is_empty() {
                        let line = number.map_or_else(|| crate::feedback::plain(name), |n| crate::feedback::with_number(name, n));
                        state.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, line));
                    }
                    break 'cast;
                }
                state.asked_casts.insert(keys.cast.1, Instant::now());
                let plays = state.nano_casts.gather(own, keys.cast.1, &|n, s| nano_stat(n, s));
                state.cast_sounds.extend(plays.into_iter().map(|(id, delay, volume)| (own, id, delay, volume)));
                let aim = target.filter(|t| !state.objects.contains_key(t));
                let _ = connection.send(&n3::cast_nano(own, aim, keys.cast.1));
                let name = thread_db.get(mao_formats::resource::Id { kind: 0xfde85, instance: keys.cast.1 }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)).map_or_else(|| format!("nano #{}", keys.cast.1), |t| t.name);
                // (N3Msg_CastNanoSpell, 0x1001b5d4: feedback 0x47 with the nano's name)
                state.game_lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::Combat, crate::feedback::executing(&name)));
                }
            }
            // A hotbar item: use the inventory slot it sits in.
            if keys.fire.0 != fired {
                fired = keys.fire.0;
                // (a bag the player uses is one with a window, whatever looked into it before)
                if let Some(bag) = state.worn.iter().find(|e| e.slot == keys.fire.1 && e.is_bag()).map(|e| e.container) {
                    state.silent_bags.remove(&bag);
                }
                uses += 1;
                let _ = connection.send(&n3::use_object(own, uses, (0x68, keys.fire.1)));
                state.start_item_timer(keys.fire.1, 3);
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
                let _ = connection.send(&n3::set_target(own, 0));
            }
            // Nothing is selected on entering, and the server is told so: a character whose client went without
            // logging out stands in the world for a while with the target it had, and a session that takes it over
            // would have its targetless casts (a buff meant for oneself) land on that target. Seen live 2026-11-10; the
            // original sends its selection whenever it changes (`N3Msg_SelectedTarget`), what it does on entering was
            // not read.
            if !target_told {
                target_told = true;
                let _ = connection.send(&n3::set_target(own, 0));
            }
            // A seated character stays seated until the sit key (the user's word for the original, 2026-11-11; until then
            // any movement key stood it up here). It turns on the spot and does not move: in the original's table of
            // movement transitions (Gamecode FUN_1006cb58) only the sit toggle's `LeaveSit` (0x25) ends the sit mode, the
            // turns (cases 9 to 0xe) are taken in any mode, and the sit mode's vehicle has the lowest top speed there is
            // (FUN_1006f9eb gives mode 8 the constant the walking modes use as their floor). What a jump does seated was
            // not read; it does nothing here.
            if sitting {
                (keys.forward, keys.back, keys.strafe, keys.jump) = seated_keys(keys.forward, keys.back, keys.strafe, keys.jump);
            }
            // Walk: integrate the position and tell the server.
            let dt = last_tick.elapsed().as_secs_f32();
            last_tick = Instant::now();
            if let Some(face) = keys.face {
                heading = face;
            }
            let walk = keys.forward as i8 - keys.back as i8;
            // The walk/run toggle (`MOVEMENT_TOGGLEWALK`): the mode changes and the server hears the switch (0x18, 0x19).
            if keys.walk_toggle != walk_toggle_seen || !mode_told {
                if keys.walk_toggle != walk_toggle_seen {
                    state.own_walking = !state.own_walking;
                }
                walk_toggle_seen = keys.walk_toggle;
                mode_told = true;
                send(&mut connection, if state.own_walking { movement::WALK_MODE } else { movement::RUN_MODE }, heading, position, &mut sent);
            }
            heading += keys.turn as f32 * turn_rate(&state.stats, walk != 0 || keys.strafe != 0) * dt;
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
            // (a sneaking character does not tell the vicinity that it is away: `afk.rs`)
            crate::afk::SNEAKING.store(state.sneaking.contains(&own), std::sync::atomic::Ordering::Relaxed);
            // (the speeds the original moves at, from the character's RunSpeed and health)
            #[allow(non_snake_case)]
            let (SPEED, BACK_SPEED) = if state.swimming.contains(&own) {
                let s = swim_speed(&state.stats);
                (s, s)
            } else if state.own_walking || state.sneaking.contains(&own) {
                (WALK_SPEED, WALK_SPEED)
            } else {
                movement_speeds(&state.stats)
            };
            // (`CharVehicle_t`'s steering, FUN_10071537: pushed along the facing, forwards or back, with the maximum force, which
            // FUN_1006f9eb makes `2 × mass × the maximum speed`, so full speed comes in half a second; a stop or a change of
            // direction halts it at once, `Vehicle_t::Halt` from the stop transition FUN_1006f0ac and `SetDirection`)
            let top = if walk > 0 { SPEED } else { BACK_SPEED };
            velocity = vehicle_step(velocity, heading, walk, pushed, top, dt);
            pushed = walk;
            // Walls stop the step (sliding along them where one axis is free).
            // Strafing adds a step to the right of the heading; diagonals aren't faster.
            let sideways = keys.strafe as f32 * SPEED;
            let (mut vx, mut vz) = (velocity[0] + heading.cos() * sideways, velocity[1] - heading.sin() * sideways);
            let total = (vx * vx + vz * vz).sqrt();
            if total > SPEED {
                (vx, vz) = (vx * SPEED / total, vz * SPEED / total);
            }
            let speed = (vx * vx + vz * vz).sqrt() * if walk < 0 { -1.0 } else { 1.0 };
            // Flying (fly mode 7, FUN_1006e4f8): falling off and the body lifted half a metre as it starts (`SetRelPos` by
            // 0x1015e0e4); the vehicle faces its motion (`Vehicle_t`'s orientation mode 4, FUN_1000c616), so going forward
            // follows the elevation the right button gave it.
            let flying = state.own_flying;
            if flying != was_flying {
                was_flying = flying;
                if flying {
                    position[1] += FLY_LIFT;
                }
                (airborne, vy) = (false, 0.0);
                body_reset = true;
            }
            // The rooms: the character crosses from one to another only through an open door (`IsDynelRoomTransitionAllowed`);
            // the step that would have taken it elsewhere is undone. The rooms it can see are told to the viewer when they change.
            if let Some(interior) = &interior {
                room_state.register(interior, &state.door_slots);
                let now = Instant::now();
                let open = |n: u8| state.doors_open.get(&n).is_some_and(|until| *until > now);
                let to = interior.room_at(position, room_state.current);
                match (room_state.current, to) {
                    (Some(from), Some(to)) if from != to && !keys.noclip && !room_state.allowed(interior, from, to, &open) => {
                        position = room_state.last_position;
                    }
                    (_, Some(to)) => {
                        room_state.current = Some(to);
                        room_state.last_position = position;
                    }
                    _ => {}
                }
                let shown = room_state.shown(interior, now, &open);
                if shown != rooms_shown {
                    rooms_shown = shown.clone();
                    pending_rooms_shown = Some(shown);
                }
            }
            let elevation = if flying { keys.pitch } else { 0.0 };
            own_pitch.set(elevation);
            let start_jump = (keys.jump || std::mem::take(&mut autoplay_jump)) && !was_jumping && !airborne && !state.swimming.contains(&own) && !flying;
            was_jumping = keys.jump;
            let mut landed = false;
            // The original's vehicle where the playfield has its collision surfaces (`vehicle.rs`: `Vehicle_t::Run` and
            // `EnsureSurfaceAlignment` against `n3TilemapSurface_t`); the client's own steps elsewhere (a generated dungeon,
            // or noclip).
            let surface = zones.clone().filter(|_| !keys.noclip).map(|z| crate::vehicle::Ground { zones: z, terrain: terrain.clone().map(|t| {
                let s = t.height_scale;
                (t, s)
            }), offset: 0.0 });
            if flying {
                // (forward and back along the pitched facing, the sidestep level; no gravity; walls stop it and the ground
                // holds it up)
                let (rise_share, level_share) = elevation.sin_cos();
                let forward = velocity[0].hypot(velocity[1]) * if walk < 0 { -1.0 } else { 1.0 };
                let (dx, dz) = ((velocity[0] * level_share + heading.cos() * sideways) * dt, (velocity[1] * level_share - heading.sin() * sideways) * dt);
                if keys.noclip || !collision.blocked(position[0] + dx, position[2] + dz, position[1], 0.4, 1.8, 0.5) {
                    (position[0], position[2]) = (position[0] + dx, position[2] + dz);
                }
                position[1] += forward * rise_share * dt;
                let floor = if keys.noclip { f32::MIN } else { collision.floor(position[0], position[2], position[1], 0.6, 0.6).unwrap_or(f32::MIN) };
                position[1] = position[1].max(ground(&terrain, position).max(floor));
                (base, rise, offset) = (position[1], 0.0, 0.0);
                airborne = false;
            } else if let Some(surface) = surface.as_ref() {
                let mut events = Vec::new();
                if start_jump {
                    // (the jump's height, FUN_10058994: (Strength + Agility) / 200 + 1, at least 0.5, the sum held to 800
                    // unless stat 215 is set; the body's scale, stat 360 in hundredths)
                    let stat = |n: u32| state.stats.get(&n).copied().unwrap_or(0) as f32;
                    let mut sum = stat(16) + stat(17);
                    if sum > 800.0 && stat(215) == 0.0 {
                        sum = 800.0;
                    }
                    let height = (sum / 200.0 + 1.0).max(0.5);
                    let scale = if stat(360) > 0.0 { stat(360) / 100.0 } else { 1.0 };
                    body.jump(surface, position, height, scale, &mut events);
                    jumping = true;
                }
                if std::mem::take(&mut body_reset) {
                    body = crate::vehicle::Body::default();
                }
                body.run(surface, &mut position, [vx, vz], dt, &mut events);
                airborne = body.falling;
                (base, rise, offset) = (position[1], 0.0, 0.0);
                for event in events {
                    match event {
                        // (`LandNow`'s callback, FUN_1006f442: the jump's stop when the character was jumping)
                        crate::vehicle::Event::Landed(_) if jumping => {
                            jumping = false;
                            landed = true;
                        }
                        crate::vehicle::Event::Swim | crate::vehicle::Event::LeaveSwim => {
                            let swim = event == crate::vehicle::Event::Swim;
                            if swim {
                                state.swimming.insert(own);
                            } else {
                                state.swimming.remove(&own);
                            }
                            if tracing {
                                eprintln!("[{:7.2}] {} at ({:.2}, {:.2}, {:.2})", clock.elapsed().as_secs_f32(), if swim { "swimming" } else { "out of the water" }, position[0], position[1], position[2]);
                            }
                            last_send = Instant::now();
                            send(&mut connection, if swim { movement::SWIM_MODE } else { movement::LEAVE_SWIM }, heading, position, &mut sent);
                        }
                        _ => {}
                    }
                }
            } else {
                let (dx, dz) = (vx * dt, vz * dt);
                // (a blocked step slides along the walls in the way, the part of it into them taken off, before an axis alone is tried)
                let slide = if keys.noclip || !collision.blocked(position[0] + dx, position[2] + dz, base + rise, 0.4, 1.8, 0.5) {
                    (0.0, 0.0)
                } else {
                    collision.wall_normal(position[0] + dx, position[2] + dz, base + rise, 0.4, 1.8, 0.5).map_or((0.0, 0.0), |n| {
                        let into = dx * n[0] + dz * n[1];
                        if into < 0.0 { (dx - into * n[0], dz - into * n[1]) } else { (0.0, 0.0) }
                    })
                };
                for (tx, tz) in [(dx, dz), slide, (dx, 0.0), (0.0, dz)] {
                    if tx == 0.0 && tz == 0.0 {
                        continue;
                    }
                    let (nx, nz) = (position[0] + tx, position[2] + tz);
                    if keys.noclip || !collision.blocked(nx, nz, base + rise, 0.4, 1.8, 0.5) {
                        (position[0], position[2]) = (nx, nz);
                        break;
                    }
                }
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
                let previous_height = position[1];
                position[1] = base + rise;
                // Swimming (Vehicle.dll `Vehicle_t::EnsureSurfaceAlignment` 0x1000d1aa, its liquid mode 0): the liquid where the
                // character is (`LiquidMediumData_t::m_vLiquidHeight`, from the surface's `GetLiquidSurfaceHeight`); with the feet
                // under its surface plus 0.1 (`_DAT_100127a8`), more than 1.19 (+0x100, `_DAT_100127a4`) between the surface and
                // the ground is swimming and holds the feet at the surface less 0.01 (`_DAT_100124e0`), less is wading; out of it,
                // or wading, swimming ends. The vehicle's in-liquid flag (+0x120) changing makes the character ask for the swim
                // mode (0x1a; FUN_10044aac) or to leave it (0x23), which the server hears.
                let swimming = state.swimming.contains(&own);
                let probe = if swimming { previous_height.max(position[1]) } else { position[1] };
                let liquid = if keys.noclip { None } else { crate::liquid::current().and_then(|l| l.at([position[0], probe, position[2]])) };
                let swim = liquid.and_then(|l| (position[1] < l.height + SWIM_ABOVE).then_some(l.height)).filter(|&surface| surface - base > SWIM_DEPTH);
                if let Some(surface) = swim {
                    position[1] = position[1].max(surface - SWIM_SINK);
                    (airborne, vy) = (false, 0.0);
                }
                if swim.is_some() != swimming {
                    if swim.is_some() {
                        state.swimming.insert(own);
                    } else {
                        state.swimming.remove(&own);
                    }
                    if tracing {
                        eprintln!("[{:7.2}] {} at ({:.2}, {:.2}, {:.2})", clock.elapsed().as_secs_f32(), if swim.is_some() { "swimming" } else { "out of the water" }, position[0], position[1], position[2]);
                    }
                    last_send = Instant::now();
                    send(&mut connection, if swim.is_some() { movement::SWIM_MODE } else { movement::LEAVE_SWIM }, heading, position, &mut sent);
                }
            }
            // Standing at a booth?
            if let Some(b) = booths.iter().find(|b| ((b[0] - position[0]).powi(2) + (b[2] - position[2]).powi(2)).sqrt() < 4.0) {
                near_booth = Some((booth_playfield, *b, Instant::now()));
            }
            if tracing && walk == 0 && last_report.elapsed() >= Duration::from_secs(2) {
                if let Some(f) = state.fighting.get(&own).and_then(|f| state.entities.get(f)) {
                    last_report = Instant::now();
                    eprintln!("[{:7.2}] standing at ({:.2}, {:.2}), the one fought at ({:.2}, {:.2}), {:.2} m", clock.elapsed().as_secs_f32(), position[0], position[2], f.0[0], f.0[2], ((f.0[0] - position[0]).powi(2) + (f.0[2] - position[2]).powi(2)).sqrt());
                }
            }
            if tracing && walk != 0 && last_report.elapsed() >= Duration::from_secs(1) {
                last_report = Instant::now();
                eprintln!("[{:7.2}] walking at ({:.1}, {:.1}, {:.1}) heading {:.2} in playfield {current}", clock.elapsed().as_secs_f32(), position[0], position[1], position[2], heading);
            }
            let mut ok = true;
            // (`CheckMotionUpdate`, Gamecode 0x10018f20: a full report after 5 s in motion without another movement message, or
            // after 0.25 s when the facing turned more than 0.17 rad since the last one other than by the turn keys, which send
            // their own; `MAO_OLD_REPORTS=1` sends one every 0.4 s in motion as the client did)
            let since = last_send.elapsed();
            let turned = keys.turn == 0 && (heading - sent_heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI > 0.17 || keys.turn == 0 && (heading - sent_heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI < -0.17;
            let due = if old_reports {
                since >= Duration::from_millis(400) && (walk != 0 || keys.turn != 0 || keys.strafe != 0 || keys.face.is_some() || airborne)
            } else {
                (since > Duration::from_secs(5) && (walk != 0 || keys.strafe != 0 || airborne)) || (since > Duration::from_millis(250) && (turned || (elevation - sent_pitch).abs() > 0.17))
            };
            let sent_this_pass = std::cell::Cell::new(false);
            let mut send_kind = |kind: u8, connection: &mut Connection| {
                sent_this_pass.set(true);
                if tracing {
                    eprintln!("[{:7.2}] sent move {kind} at ({:.2}, {:.2}, {:.2}) heading {heading:.2}", clock.elapsed().as_secs_f32(), position[0], position[1], position[2]);
                }
                last_send = Instant::now();
                sent_heading = heading;
                sent_pitch = elevation;
                told.push_back((position, Instant::now()));
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
            // `LoadPFTweakData` with the environment the playfield's own environment data names, `playfield::sky_scripts` —
            // then the automatic-run objects), Notum Wars merged in (FUN_100b1f3c: `SetExpansion` of stat 389 once), the script events
            // (FUN_10130a1b), then the frame (`VisualEnvFX_t::FrameProcess` from FUN_100b05b4: FXS's frame, then the game
            // interface's variables: the day time is the clock's seconds over its pace, the server's clock, the character's
            // place as the eye's; no storm, precipitation or wind) and the sounds it played.
            if fxs_playfield != Some(current) {
                // (the old playfield's objects die with it: FUN_100b2972's `FXS_t::DeleteInstance`)
                if fxs_playfield.is_some() {
                    crate::fxs::reset();
                    fxs_expansion = false;
                }
                fxs_playfield = Some(current);
                fxs_visuals.clear();
                crate::fxs::set_data_path(&Path::new(&game_dir).join("cd_image"));
                let sky = mao_formats::playfield::sky_scripts(terrain.is_some(), base_environment.map_or(0, |e| e[18]));
                crate::fxs::load_playfield(sky.playfield.unwrap_or(current), sky.environment);
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
            // (`/fxscript`: the typed lines, each through the scripts' signal as the original's command sends it)
            for line in std::mem::take(&mut *FXS_COMMANDS.lock().unwrap_or_else(|e| e.into_inner())) {
                let ran = crate::fxs::command(line.as_bytes());
                let mut lines = CHAT_LINES.lock().unwrap();
                for text in fxs_report(&line, &ran) {
                    lines.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, text));
                }
            }
            crate::fxs::frame_process();
            let (system_time, system_second) = state.game_clock.server_clock();
            // (FUN_100b8cf3's suns at the clock's seconds into the day; the weather's clouds and storm as FUN_100becbe gives
            // them the frame block: +0xf0 the strongest precipitation, +0xf4 entry 0, +0x114 entry 2, +0x1134)
            let (sun1, sun2) = crate::fxs::sun::rotations(state.game_clock.seconds());
            let w = &ambient_weather;
            let (wind, base) = fxs_wind.raw();
            let high_wind = [0.004 * (wind[0] + base[0]) * dt * 0.5, 0.004 * (wind[2] + base[2]) * dt * 0.5];
            fxs_wind.advance(w[7], dt);
            // (FUN_100b8e6f, with Notum Wars: the eye's offset on the planet from the headquarters)
            let offset_from_hq = if state.expansion { crate::fxs::planet::offset_from_hq(current, position) } else { [0.0; 2] };
            state.game_clock.publish();
            crate::fxs::set_game(&crate::fxs::Game {
                delta_time: dt,
                day_time: state.game_clock.seconds() / crate::gametime::PACE,
                system_time,
                system_second,
                camera_position: position,
                camera_rotation: [0.0, 1.0, 0.0, 0.0],
                storm: w[3].max(w[4]).max(w[5]).max(w[6]),
                precipitation: w[0],
                thick_clouds: w[2],
                cloud_transparency: 1.0 - w[2] / 2.333,
                high_wind,
                offset_from_hq,
                sun1,
                sun2,
                ..Default::default()
            });
            for s in crate::fxs::take_sounds() {
                let r = crate::fxs::math::round;
                crate::audio::with(|m| m.play_game_sound(s.id, s.pos, s.f10, s.volume, s.i18, s.f1c, r(s.f20), r(s.f24), r(s.f28) as u16 as u32, s.vel));
            }
            let started = crate::fxs::take_started();
            if std::env::var_os("MAO_FXS").is_some() {
                for (fxid, o) in &started {
                    eprintln!("fxs: effect {fxid} started ({:x})", o.name);
                }
            }
            // (a mesh object's universal effects: `universal_effects`. Each goes with its object from then on, as the
            // original's stays on its connector while `RunFunction` gives it the object's `Position` every frame, and
            // ends with the object. The connector's own place on the mesh is not taken: the meshes that carry one are
            // small.)
            for (fxid, o) in &started {
                if *fxid != 4 {
                    continue;
                }
                let Some(mesh) = o.variable_named("Mesh").and_then(|v| crate::fxs::string_of(&v)) else { continue };
                let effects = fxs_mesh_effects.entry(mesh.clone()).or_insert_with(|| fxs_index.as_ref().map_or(Vec::new(), |index| universal_effects(&gear_db, index, &mesh)));
                if effects.is_empty() {
                    continue;
                }
                let at = o.variable_named("Position").map_or(position, |v| crate::fxs::vector_of(&v));
                for effect in effects.iter() {
                    if std::env::var_os("MAO_FXS").is_some() {
                        eprintln!("fxs: universal effect {effect} of {mesh} at {at:?}");
                    }
                    if let Some(g) = gfx.as_ref() {
                        for s in crate::sounds::effect_sounds(g, *effect) {
                            crate::audio::with(|m| m.play_sound_command(s.id, at, s.velocity, s.volume, s.range, s.delay, s.defer, s.pitch));
                        }
                    }
                    fxs_universal_next += 1;
                    fxs_universal.push((fxs_universal_next, o.clone(), *effect, false));
                }
            }
            fxs_visuals.start(started);
            // (the frame block the single clouds read: the eye — the character's place here —, `GameSunDirection` toward the two
            // suns together, the storm and the clouds)
            let (s1, s2) = crate::fxs::sun::places(state.game_clock.seconds());
            let sky = crate::fxs::clouds::Sky {
                camera: position,
                sun_direction: crate::fxs::math::with_len(&crate::fxs::math::add(&s1, &s2), 1.0),
                storm: w[3].max(w[4]).max(w[5]).max(w[6]),
                thick_clouds: w[2],
                ground: None,
                wind: [high_wind[0], 0.0, high_wind[1]],
            };
            state.sky_frame = Some(fxs_visuals.frame(&sky));
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
                    // (FUN_1011f406: the land-control map is read with the area and district info)
                    let object = gear_db.get(mao_formats::resource::Id { kind: mao_formats::landcontrolmap::KIND, instance: current }).ok().flatten();
                    state.land_map = object.and_then(|o| mao_formats::landcontrolmap::parse(&o)).map(std::sync::Arc::new);
                    state.land_map_changed = true;
                }
                // (the district info under the map's land-control view, which GUI.dll's FUN_100eb04b asks for twice a second)
                let info = districts.as_ref().and_then(|d| d.1.as_ref()).and_then(|d| d.land_control_info(position));
                if info != state.lc_district {
                    state.lc_district = info;
                    state.lc_district_changed = true;
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
            // (the server zones from a door only within about half a metre of its place: measured live, a report 1.1 m off was
            // ignored and one 0.24 m off zoned; a zone's way out is its outline, `n3Zone_t::IsPosInTeleportal` FUN_10058b35)
            let at_door = state.door_slots.values().any(|(_, d)| ((d[0] - position[0]).powi(2) + (d[2] - position[2]).powi(2)).sqrt() < DOOR_REACH && (d[1] - position[1]).abs() < 3.0)
                || teleportals.iter().any(|t| t.contains(position[0], position[2]));
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
            if walk != moving && walk == 0 {
                last_overlay = Instant::now() - Duration::from_secs(1);
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
            } else if (due && !sent_this_pass.get()) || landed {
                // (a movement message just sent this pass counts as the last one: no report on its heels)
                // (the periodic report is the original's full one, 0x16; 15 is what the captures show as the character lands)
                send_kind(if landed { movement::JUMP_STOP } else { movement::FULL }, &mut connection);
            }
            if keys.strafe != strafing {
                // The strafe start and stop types (the protocol's `MovementType`: `StrafeRightStart` 5, `StrafeRightStop` 6,
                // `StrafeLeftStart` 7, `StrafeLeftStop` 8; the captures carry them for the other characters about).
                let kind = match (strafing, keys.strafe) {
                    (_, -1) => movement::STRAFE_LEFT_START,
                    (_, 1) => movement::STRAFE_RIGHT_START,
                    (-1, _) => movement::STRAFE_LEFT_STOP,
                    _ => movement::STRAFE_RIGHT_STOP,
                };
                send_kind(kind, &mut connection);
            }
            if !ok {
                break;
            }
            moving = walk;
            turning = keys.turn;
            strafing = keys.strafe;
            // (flying, the vehicle is drawn nose up or down as it is reported)
            state.rotations.insert(own, n3::pitched_rotation(heading, elevation));
            // (the creatures the server walks: along their paths, after their targets)
            state.walk_paths(dt, position);
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
            state.own_moving = walk != 0 || keys.strafe != 0;
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
                            old.ground = patch.ground;
                            old
                        }
                        None => patch,
                    });
                }
            }
            if last_overlay.elapsed() >= Duration::from_millis(33) {
                last_overlay = Instant::now();
                let built = Instant::now();
                state.tick_doors(position);
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
                // The spell lists' effects (`spelllist`): each made on its character with its sound controls; a flare (kind 1005,
                // `_GfxControlFlare_t`) burns where the character stood, as the mines' blasts do, any other runs on the character
                // with its running programs. The same effect made again starts again.
                for (who, effect, seconds) in std::mem::take(&mut state.spell_effects_made) {
                    let Some(g) = gfx.as_ref() else { continue };
                    let Some(p) = (if who == own { Some(position) } else { state.position_of(who) }) else { continue };
                    for s in crate::sounds::effect_sounds(g, effect) {
                        crate::audio::with(|m| m.play_sound_command(s.id, p, s.velocity, s.volume, s.range, s.delay, s.defer, s.pitch));
                    }
                    if g.get(effect).and_then(mao_formats::gfx::Flare::read).is_some() {
                        state.placed_effects.push((effect, p));
                    } else if g.visual(effect).is_some() {
                        state.spell_effects.entry(who).or_default().insert(effect, (Instant::now(), seconds));
                    }
                }
                state.spell_effects.retain(|_, effects| {
                    effects.retain(|_, (began, seconds)| seconds.is_none_or(|s| began.elapsed().as_secs_f32() < s));
                    !effects.is_empty()
                });
                let (model, mut casters, character_parts) = overlay_model(&state, &state.props, &mut characters, &|id| id != own && near(id), true);
                let (own_model, own_casters, _) = overlay_model(&state, &state.props, &mut characters, &|id| id == own, false);
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
                // (each body's size as it stands, for its name tag and the mouse: `blobshadow::Size`)
                let sizes: std::collections::HashMap<u32, crate::blobshadow::Size> = casters.iter().filter(|c| c.id != 0).map(|c| (c.id, c.size)).collect();
                let deforms_landed: std::cell::RefCell<Vec<(u32, mao_formats::gfx::Deform)>> = std::cell::RefCell::new(Vec::new());
                // (the speed over the time the position itself took: the position is the last step's, `last_tick`, not this
                // moment's, which comes some milliseconds later, after the characters are posed; and the window is told that
                // time too, `own_time`. Timed by the clock as they were built and as they arrived, the character jumped a few
                // centimetres on or back with every update, more the faster it ran, and the camera with it)
                let own_velocity = if walk == 0 && keys.strafe == 0 && !airborne {
                    [0.0; 3]
                } else {
                    let dt = last_tick.duration_since(last_own.1).as_secs_f32().max(0.001);
                    [(position[0] - last_own.0[0]) / dt, 0.0, (position[2] - last_own.0[2]) / dt]
                };
                last_own = (position, last_tick);
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
                state.tick_timers();
                state.refresh_mods(&gear_db);
                // (an inspection: the items for the inspect window, the original's `WearView_c` with the identity)
                let inspected_now = state.inspected.take().map(|(who, items)| {
                    let name = state.names.get(&who).map_or_else(String::new, |n| n.0.clone());
                    let entries: Vec<mao_net::inventory::Entry> = items.iter().map(|it| mao_net::inventory::Entry { slot: it.slot, flags: 1, count: 1, container: (0, 0), low_id: it.low_id, high_id: it.high_id, quality: it.ql, extra: 0 }).collect();
                    (name, crate::gear::from_entries(&gear_db, &gear_textures, &entries, state.stats.get(&0x36).copied().unwrap_or(1)))
                });
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
                        buddies.1.iter().map(|(&id, (name, online, recent))| crate::friends::Friend { id, name: name.clone(), online: *online, recent: *recent }).collect::<Vec<_>>()
                    })
                };
                // The target NCU windows' lists: the selected target's nanos and the fight target's, told when either's
                // target or set of nanos changes.
                let target_nanos = {
                    let ids = [target, state.fighting.get(&own).copied()];
                    let lists = ids.map(|id| id.map(|id| state.nanos_on(id)));
                    let key = [0, 1].map(|k| ids[k].map(|id| (id, lists[k].iter().flatten().map(|n| n.0).collect::<Vec<u32>>())));
                    (key != target_nanos_told).then(|| {
                        target_nanos_told = key;
                        let level = state.stats.get(&0x36).copied().unwrap_or(1);
                        lists.map(|list| {
                            list.map(|list| {
                                let now = Instant::now();
                                let running: Vec<(u32, u32)> = list
                                    .iter()
                                    .map(|&(nano, left, since)| {
                                        // (a nano announced without its time: its whole duration from when it was first seen)
                                        let whole = left.unwrap_or_else(|| {
                                            *duration_cache.entry(nano).or_insert_with(|| {
                                                let data = gear_db.get(mao_formats::resource::Id { kind: 0xfde85, instance: nano }).ok().flatten();
                                                data.as_deref().and_then(mao_formats::item::Item::parse).and_then(|t| t.stat(8)).unwrap_or(0)
                                            })
                                        });
                                        (nano, whole.saturating_sub(since.elapsed().as_millis().min(u128::from(u32::MAX)) as u32))
                                    })
                                    .collect();
                                crate::ncu::describe(&gear_db, &gear_textures, &running, level, now)
                            })
                        })
                    })
                };
                let update = crate::viewer::Update {
                    messages_only: false,
                    target_nanos,
                    scene: pending_scene.take(),
                    falls: pending_falls.take().map(|feet| (feet, gfx.as_ref().and_then(|g| g.visual(0x30d6)))),
                    interior: pending_interior.take(),
                    rooms_shown: pending_rooms_shown.take(),
                    new_camera: std::mem::take(&mut pending_camera),
                    model,
                    own_model,
                    own_velocity,
                    own_time: Some(last_tick),
                    casters,
                    character_parts,
                    map: pending_map.take(),
                    stats: std::mem::take(&mut state.stats_changed).then(|| {
                        let parts = crate::derived::breakdown(&state.stats, &state.mods_by_source[0], &state.mods_by_source[1], &state.mods_by_source[2], &state.mods_by_source[3]);
                        (state.names.get(&own).map(|n| n.0.clone()).unwrap_or_default(), crate::derived::totals(&parts), parts)
                    }),
                    towers: std::mem::take(&mut state.towers_changed).then(|| state.towers.get(&state.tower_playfield).cloned().unwrap_or_default()),
                    land_map: std::mem::take(&mut state.land_map_changed).then(|| state.land_map.clone()),
                    lc_district: std::mem::take(&mut state.lc_district_changed).then(|| state.lc_district.clone()),
                    map_dots: {
                        let own_side = state.stats.get(&33).copied().unwrap_or(0) as u8;
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
                                } else if crate::combat::friendly(own_side, info.side) {
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
                        let still = &state.active_buffs;
                        state.buff_first.retain(|(character, nano), _| still.get(character).is_some_and(|nanos| nanos.contains_key(nano)));
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
                        // (the spell lists' effects on each character, keyed apart from the programs by the top bit)
                        let spell_visuals = |id: u32| -> Vec<(u32, mao_formats::gfx::Visual)> {
                            let Some(effects) = state.spell_effects.get(&id) else { return Vec::new() };
                            effects.keys().filter_map(|&e| Some((0x8000_0000 | e as u32, gfx.as_ref()?.visual(e)?))).collect()
                        };
                        let mut add = |id: u32, own_character: bool, at: [f32; 3], nanos: Vec<u32>| {
                            let mut auras: Vec<(u32, mao_formats::gfx::Visual)> = nanos.into_iter().filter_map(|n| aura_of(n).map(|v| (n, v))).take(6).collect();
                            auras.extend(spell_visuals(id));
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
                        for id in state.spell_effects.keys().filter(|id| **id != own && !state.active_buffs.contains_key(id)) {
                            if let Some((p, _)) = state.entities.get(id) {
                                add(*id, false, *p, Vec::new());
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
                    sky_frame: state.sky_frame.take(),
                    placed_gone: {
                        let gone = fxs_universal.iter().filter(|u| !u.1.alive()).map(|u| u.0).collect();
                        fxs_universal.retain(|u| u.1.alive());
                        gone
                    },
                    // (each where its object is now; the effect itself goes with the first of them)
                    placed: fxs_universal
                        .iter_mut()
                        .map(|(key, o, effect, sent)| {
                            let at = o.variable_named("Position").map_or(position, |v| crate::fxs::vector_of(&v));
                            let visual = if std::mem::replace(sent, true) { None } else { gfx.as_ref().and_then(|g| g.visual(*effect)) };
                            (*key, [at[0], at[2], at[1]], visual)
                        })
                        .collect(),
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
                    // (`TargetingModule_t`'s tags: the selection's and the attack's indicators, everyone's with `ShowAllNames`)
                    nameplates: {
                        let own_level = state.stats.get(&54).copied().unwrap_or(1);
                        let range = state.stats.get(&275).copied();
                        let orgs_shown = crate::prefs::get("IsOrgNameShownOverHead").is_some_and(|v| v != 0.0);
                        let attacking = state.fighting.get(&own).copied();
                        let tag = |id: u32, brackets: Option<bool>| -> Option<crate::nametag::Tag> {
                            let p = if id == own { position } else { state.entities.get(&id)?.0 };
                            let info = state.infos.get(&id);
                            let name = state.names.get(&id).map(|n| n.0.clone()).filter(|n| !n.is_empty()).or_else(|| info.map(|i| i.name.clone()).filter(|n| !n.is_empty()))?;
                            let side = info.map_or_else(|| state.stats.get(&33).copied().unwrap_or(0) as u8, |i| i.side);
                            let level = if id == own { own_level } else { state.levels.get(&id).map_or(0, |l| *l as i64) };
                            Some(crate::nametag::Tag {
                                at: [p[0], p[2], p[1] + crate::nametag::height(sizes.get(&id).map(|s| s.height))],
                                name,
                                org: state.orgs.get(&id).filter(|_| orgs_shown).map(|o| (o.clone(), side)),
                                brackets,
                                health: state.health.get(&id).copied(),
                                consider: crate::nametag::consider(own_level, range, level),
                                hidden: false,
                            })
                        };
                        let mut tags = Vec::new();
                        if crate::prefs::on("ShowAllNames") {
                            tags.extend(state.entities.keys().filter(|id| **id != own && Some(**id) != target && Some(**id) != attacking).filter_map(|id| tag(*id, None)));
                        }
                        tags.extend(target.filter(|t| *t == own || state.infos.contains_key(t)).and_then(|t| tag(t, Some(false))));
                        tags.extend(attacking.and_then(|a| tag(a, Some(true))));
                        tags
                    },
                    ground: terrain.clone().map(|t| (t, offset)),
                    zones: zones.clone(),
                    collision: Some(collision.clone()),
                    own: Some(position),
                    environment: Some(crate::environment::compute(&areas, base_environment.as_ref(), position, state.game_clock.day(), state.game_clock.seconds(), state.game_clock.day_intensity())),
                    daylight: Some(state.game_clock.day_intensity()),
                    location: pending_location.take(),
                    labels: pending_labels.take(),
                    // (what the mouse can pick, as cylinders standing on their feet: the original casts the camera's line
                    // through the mouse against the dynels' collision, `n3Camera_t::SetMousePos` / `GetObjectUnderColLine`)
                    // (the character itself last: picked only where nothing else is on the line)
                    pickables: Some({
                        let size = |kind: u32| match kind {
                            0xc75b => (1.0, 2.6),
                            DOOR_TYPE => (1.0, 3.0),
                            _ => (0.8, 2.2),
                        };
                        let kind_of = |id: &u32| state.types.get(id).copied().unwrap_or(n3::CHARACTER);
                        state
                            .objects
                            .iter()
                            .map(|(id, p)| {
                                let (r, h) = size(kind_of(id));
                                (kind_of(id), *id, [p[0], p[2], p[1]], r, h, false)
                            })
                            .chain(state.table.iter().map(|&(kind, id, p, _, _)| {
                                let (r, h) = size(kind);
                                (kind, id, [p[0], p[2], p[1]], r, h, false)
                            }))
                            .chain(state.entities.iter().filter(|(id, _)| **id != own).map(|(id, (p, _))| {
                                // (as large as its mesh stands: a cylinder of a person's size missed the large and overlapped
                                // the small)
                                let (r, h) = sizes.get(id).map_or((0.5, 1.9), |s| s.pick());
                                (kind_of(id), *id, [p[0], p[2], p[1]], r, h, false)
                            }))
                            .chain(std::iter::once({
                                let (r, h) = sizes.get(&own).map_or((0.5, 1.9), |s| s.pick());
                                (n3::CHARACTER, own, [position[0], position[2], position[1]], r, h, true)
                            }))
                            .collect()
                    }),
                    info_urls: std::mem::take(&mut state.info_urls),
                    open_windows: std::mem::take(&mut state.open_windows),
                    browser: state.browser.take(),
                    messages: {
                        for line in &chat_lines_now {
                            pending_messages.push(line.text.clone());
                        }
                        std::mem::take(&mut pending_messages)
                    },
                    chat_lines: chat_lines_now,
                    friends: friends_now,
                    loot: std::mem::take(&mut state.loot_changed).then(|| {
                        state.loot.as_ref().map(|l| crate::loot::LootView { own: false, title: state.names.get(&l.identity.1).map(|n| n.0.clone()).filter(|n| !n.starts_with("Object ")).unwrap_or_else(|| crate::loot::title(l.identity.0).to_string()), identity: l.identity, handle: l.handle, items: crate::gear::from_entries(&gear_db, &gear_textures, &l.entries, state.stats.get(&0x36).copied().unwrap_or(1)) })
                    }),
                    bags: std::mem::take(&mut state.bags_changed).then(|| {
                        // (a bag that left the inventory has no window any more)
                        let worn = &state.worn;
                        state.bags.retain(|identity, _| worn.iter().any(|e| e.container == *identity));
                        let level = state.stats.get(&0x36).copied().unwrap_or(1);
                        state
                            .bags
                            .values()
                            .filter(|bag| !state.silent_bags.contains(&bag.identity))
                            .map(|bag| {
                                // (titled with the bag's own name: the item's)
                                let item = state.worn.iter().find(|e| e.container == bag.identity).cloned();
                                let title = item.and_then(|e| crate::gear::from_entries(&gear_db, &gear_textures, &[e], level).pop()).map_or_else(|| crate::loot::title(bag.identity.0).to_string(), |g| g.name);
                                crate::loot::LootView { own: true, title, identity: bag.identity, handle: bag.handle, items: crate::gear::from_entries(&gear_db, &gear_textures, &bag.entries, level) }
                            })
                            .collect()
                    }),
                    perk_list: std::mem::take(&mut state.perks_changed).then(|| {
                        let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
                        crate::perks::PerkState { catalog: crate::perks::catalog(&gear_db), owned: state.perks.clone(), server_offset: state.game_clock.server_time() as i64 - unix }
                    }),
                    programs: std::mem::take(&mut state.programs_changed).then(|| crate::nanoprograms::describe(&gear_db, &gear_textures, &state.program_ids, state.stats.get(&0x36).copied().unwrap_or(1))),
                    duel: std::mem::take(&mut state.duel_events)
                        .into_iter()
                        .map(|(what, flag, other)| (what, flag, state.names.get(&other).map(|n| n.0.clone()).or_else(|| state.infos.get(&other).map(|i| i.name.clone())).unwrap_or_default()))
                        .collect(),
                    specials: {
                        let now = state.special_numbers();
                        (std::mem::take(&mut state.specials_changed) | (now != state.specials_told)).then(|| {
                            state.specials_told = now;
                            crate::actions::describe_specials(&gear_db, &gear_textures, state.specials_told.as_deref().unwrap_or_default())
                        })
                    },
                    perk_actions: std::mem::take(&mut state.perk_actions_changed).then(|| {
                        let now = Instant::now();
                        let locks: BTreeMap<u32, (Instant, f32)> = state.perk_locks.iter().filter(|l| l.1.0 > now).map(|(&k, &v)| (k, v)).collect();
                        crate::actions::describe_perks(&gear_db, &gear_textures, &state.perk_actions, &locks)
                    }),
                    gear: pending_gear.take(),
                    flying: (state.own_flying != flying_told).then(|| {
                        flying_told = state.own_flying;
                        flying_told
                    }),
                    inspected: inspected_now,
                    nanos: std::mem::take(&mut state.nano_list_changed).then(|| crate::ncu::describe(&gear_db, &gear_textures, &state.nano_times, state.stats.get(&0x36).copied().unwrap_or(1), state.nano_times_at.unwrap_or_else(Instant::now))),
                    missions: pending_missions.take(),
                    mission_status: pending_mission_status.take(),
                    mail_count: pending_mail_count.take(),
                    mail_entries: pending_mail_entries.take(),
                    mail_opened: pending_mail_opened.take(),
                    shop: pending_shop.take(),
                    shop_status: pending_shop_status.take(),
                    shop_changes: std::mem::take(&mut pending_shop_changes),
                    npc_chat: std::mem::take(&mut pending_npc_chat),
                    mission_log: pending_mission_log.take(),
                    missions_removed: std::mem::take(&mut pending_missions_removed),
                    mail_flags: pending_mail_flags.take(),
                    mail_status: pending_mail_status.take(),
                    destinations: pending_destinations.take(),
                    heading: Some(heading),
                    // (the control center's two target bars, `CCTargetControl_c`'s update FUN_10074349: the friendly one the
                    // selection unless it is the one fought, the hostile one the one fought)
                    target_bars: Some({
                        let fighting = state.fighting.get(&own).copied();
                        let own_level = state.stats.get(&54).copied().unwrap_or(1);
                        let range = state.stats.get(&275).copied();
                        let targets_target = crate::prefs::get("Targetstarget").is_some_and(|v| v != 0.0);
                        let bar = |t: u32, hostile: bool, title: &str| -> Option<crate::ui::TargetBar> {
                            let name = state.names.get(&t).map(|n| n.0.clone()).filter(|n| !n.is_empty()).or_else(|| state.infos.get(&t).map(|i| i.name.clone()))?;
                            let p = if t == own { position } else { state.position_of(t)? };
                            let distance = ((p[0] - position[0]).powi(2) + (p[2] - position[2]).powi(2)).sqrt();
                            let hp = state.health.get(&t).map(|&(hp, max)| (hp as i64, max as i64)).or_else(|| state.last_hp.get(&t).map(|&hp| (hp as i64, 0)));
                            let character = t == own || state.infos.contains_key(&t);
                            let level = if t == own { Some(own_level) } else { state.levels.get(&t).map(|&l| l as i64) };
                            let fill = if character { crate::nametag::consider_colour(crate::nametag::consider(own_level, range, level.unwrap_or(0))) } else { 0xff_ffff };
                            let kind = if hostile { crate::automap::DotKind::Hostile } else { crate::automap::DotKind::Friendly };
                            let target_of = state.fighting.get(&t).filter(|_| targets_target).and_then(|f| state.names.get(f).map(|n| n.0.clone()).or_else(|| state.infos.get(f).map(|i| i.name.clone())));
                            Some(crate::ui::TargetBar { hostile, selected: target == Some(t), title: title.to_string(), name_colour: if hostile { 0xff_4444 } else { 0xff_ffff }, fill, target_of, name, level, hp, distance, kind })
                        };
                        let mut bars = Vec::new();
                        if let Some(s) = target.filter(|s| Some(*s) != fighting) {
                            bars.extend(bar(s, false, if s == own || state.infos.contains_key(&s) { "Nano Target" } else { "Selection" }));
                        }
                        if let Some(f) = fighting {
                            bars.extend(bar(f, true, if target == Some(f) { "Nano & Fighting Target" } else { "Fighting Target" }));
                        }
                        bars
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
    crate::viewer::window(scene, textures, Some(crate::viewer::Live { overlays: rx, input: input_tx, mail: mail_tx, shop: shop_tx, npc_chat: npc_chat_tx, mission_log: mission_log_tx, focus, distance: 6.0 }))
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
        // `net <game dir> talk <name> [npc name part]`: log in, open a chat with the nearest NPC (named so, if given), print
        // the server's NPC chat messages, answer the first answer once, then close the chat; a plain use instead when the
        // open draws no answer.
        [game, word, who, rest @ ..] if word == "talk" => {
            let wanted = rest.first().map(|s| s.to_ascii_lowercase());
            let (mut chat, list) = login(game)?;
            let character = list.iter().find(|c| c.name.eq_ignore_ascii_case(who)).ok_or("no such character")?;
            let ticket = session::login_character(&mut chat, character.id).map_err(|e| e.to_string())?;
            let mut world = session::enter_world(&ticket).map_err(|e| e.to_string())?;
            world.set_timeout(Duration::from_millis(50)).map_err(|e| e.to_string())?;
            let own = character.id;
            let mut state = World { own, ..World::default() };
            let clock = Instant::now();
            let (mut phase, mut at, mut npc, mut answered) = (0, Instant::now(), None::<(u32, u32)>, false);
            loop {
                let t = clock.elapsed().as_secs_f32();
                if phase == 0 && t > 3.0 {
                    world.send(&n3::ready(own)).map_err(|e| e.to_string())?;
                    world.send(&n3::action(own, n3::STAND)).map_err(|e| e.to_string())?;
                    phase = 1;
                } else if phase == 1 && t > 6.0 {
                    let me = state.entities.get(&own).map(|e| e.0).unwrap_or([0.0; 3]);
                    let d = |p: [f32; 3]| (p[0] - me[0]).powi(2) + (p[2] - me[2]).powi(2);
                    let found = state
                        .infos
                        .iter()
                        .filter(|(id, i)| **id != own && !i.is_player() && wanted.as_ref().is_none_or(|w| i.name.to_ascii_lowercase().contains(w)))
                        .filter_map(|(id, i)| Some((*id, i.name.clone(), d(state.entities.get(id)?.0))))
                        .min_by(|a, b| a.2.total_cmp(&b.2));
                    let Some((id, name, d2)) = found else { return Err("no NPC near".into()) };
                    eprintln!("[{t:5.1}] OPEN chat with {name} ({id:#x}), {:.1} m away", d2.sqrt());
                    npc = Some((n3::CHARACTER, id));
                    world.send(&n3::npc_chat_open(own, (n3::CHARACTER, id))).map_err(|e| e.to_string())?;
                    at = Instant::now();
                    phase = 2;
                } else if phase == 2 && at.elapsed() > Duration::from_secs(4) {
                    eprintln!("[{t:5.1}] no answer to the open: USE instead");
                    world.send(&n3::use_object(own, 1, npc.unwrap())).map_err(|e| e.to_string())?;
                    at = Instant::now();
                    phase = 3;
                } else if (phase == 3 || phase == 4) && at.elapsed() > Duration::from_secs(6) {
                    world.send(&n3::npc_chat_close(own, npc.unwrap())).map_err(|e| e.to_string())?;
                    eprintln!("[{t:5.1}] CLOSE sent");
                    at = Instant::now();
                    phase = 5;
                } else if phase == 5 && at.elapsed() > Duration::from_secs(2) {
                    break;
                }
                match world.receive() {
                    Ok(frame) => {
                        if let Some(reply) = frame::ping_reply(&frame, own, 70_000_000 + clock.elapsed().as_millis() as u32) {
                            let _ = world.send(&reply);
                        }
                        if let Some(m) = Message::parse(&frame) {
                            if let Some(e) = crate::npcchat::event(m.kind, &m.data) {
                                eprintln!("[{t:5.1}] <- {:08x} from ({:x},{:x}): {e:?} raw {}", m.kind, m.identity.0, m.identity.1, m.data.iter().take(48).map(|b| format!("{b:02x}")).collect::<String>());
                                if phase == 2 {
                                    phase = 4;
                                    at = Instant::now();
                                }
                                if let crate::npcchat::Event::Answers(n, list) = &e
                                    && !answered
                                    && !list.is_empty()
                                {
                                    answered = true;
                                    eprintln!("[{t:5.1}] ANSWER 0 ({:?})", list[0]);
                                    world.send(&n3::npc_chat_answer(own, *n, 0)).map_err(|e| e.to_string())?;
                                    at = Instant::now();
                                }
                            } else if phase >= 2 && [0x52526858, 0x0a0c5a, 0x5e477770].contains(&m.kind) {
                                eprintln!("[{t:5.1}] <- {:08x} from ({:x},{:x}) {}", m.kind, m.identity.0, m.identity.1, m.data.iter().take(40).map(|b| format!("{b:02x}")).collect::<String>());
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
            crate::layout::set_character(character.id);
            state.print();
            if let Some(i) = rest.iter().position(|a| a == "--show") {
                let image = rest.get(i + 1).filter(|a| a.as_str() == "--image").and(rest.get(i + 2)).map(String::as_str);
                return show_world(game, character.proxy[1], &state, image);
            }
            Ok(())
        }
        // `net <game> suggest <breed choice 1-7> <profession choice 1-14>`: the login server's suggested name.
        [game, word, breed, prof] if word == "suggest" => {
            let (b, s) = crate::charcreate::breed_and_sex(breed.parse().map_err(|_| "bad breed")?).ok_or("the breed choice is 1 to 7")?;
            let (mut connection, _) = login(game)?;
            let reply = session::ask_creation(&mut connection, &mao_net::create::suggest_name(b, s, crate::charcreate::profession(prof.parse().map_err(|_| "bad profession")?))).map_err(|e| e.to_string())?;
            connection.close();
            println!("{reply:?}");
            Ok(())
        }
        // `net <game> create <name> <breed 1-7> <profession 1-14> <head n> <height 0-2> <build 0-2>`: makes a character (the
        // original's `CreateCharacter`), the head the n-th the breed may have.
        [game, word, name, breed, prof, head, height, width] if word == "create" => {
            let (b, s) = crate::charcreate::breed_and_sex(breed.parse().map_err(|_| "bad breed")?).ok_or("the breed choice is 1 to 7")?;
            let name = crate::charcreate::proper(name);
            if let Some(problem) = crate::charcreate::name_problem(&name) {
                return Err(crate::charcreate::text(problem));
            }
            let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
            let index = mao_formats::instances::Instances::parse(&db.get(mao_formats::resource::Id { kind: 0xf424a, instance: 1 }).map_err(|e| e.to_string())?.ok_or("no instance index")?).map_err(|_| "the instance index is cut short")?;
            let heads = crate::charcreate::heads(b, s, &|n| index.find(0xf6951, n));
            let n: usize = head.parse().map_err(|_| "bad head")?;
            let (mesh, race, file) = heads.get(n).cloned().ok_or_else(|| format!("{} heads for that breed", heads.len()))?;
            let c = mao_net::create::NewCharacter { name, breed: b, sex: s, profession: crate::charcreate::profession(prof.parse().map_err(|_| "bad profession")?), head: mesh as i32, height: crate::charcreate::height(height.parse().map_err(|_| "bad height")?), width: width.parse().map_err(|_| "bad build")? };
            println!("making {c:?} (head {file}, race {race})");
            let (mut connection, _) = login(game)?;
            let reply = session::ask_creation(&mut connection, &mao_net::create::create_character(&c)).map_err(|e| e.to_string())?;
            connection.close();
            match reply {
                mao_net::create::Reply::Refused(code) => println!("refused: {code} {}", crate::charcreate::text(crate::charcreate::refusal(code))),
                other => println!("{other:?}"),
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
        // `net <game dir> caught <playfield> <x> <y> <z>`: where a body standing there gets to in a second's run each of eight
        // ways (to see a place a character is caught in).
        [game, word, pf, x, y, z] if word == "caught" => {
            let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
            let area = load_area(game, &db, pf.parse().map_err(|_| "bad playfield")?)?;
            let from: [f32; 3] = [x.parse().map_err(|_| "bad x")?, y.parse().map_err(|_| "bad y")?, z.parse().map_err(|_| "bad z")?];
            let ground = crate::vehicle::Ground { zones: area.zones.clone().unwrap_or_default(), terrain: area.terrain.clone().map(|t| { let s = t.height_scale; (t, s) }), offset: 0.0 };
            println!("closest from there: {:?}", ground.closest(from));
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let (mut body, mut pos, mut events) = (crate::vehicle::Body::default(), from, Vec::new());
                for _ in 0..30 {
                    body.run(&ground, &mut pos, [a.cos() * 5.0, a.sin() * 5.0], 1.0 / 30.0, &mut events);
                }
                println!("towards ({:+.1}, {:+.1}): to ({:.2}, {:.2}, {:.2}), {:.2} m", a.cos(), a.sin(), pos[0], pos[1], pos[2], ((pos[0] - from[0]).powi(2) + (pos[2] - from[2]).powi(2)).sqrt());
            }
            Ok(())
        }
        // `net <game dir> ground <playfield> <x> <z> [y]`: the vehicle's ground there (the tile-map's, the closest point from
        // y, 100 by default, and a ray straight down), against the old bilinear height.
        [game, word, pf, x, z, rest @ ..] if word == "ground" => {
            let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
            let area = load_area(game, &db, pf.parse().map_err(|_| "bad playfield")?)?;
            let (x, z): (f32, f32) = (x.parse().map_err(|_| "bad x")?, z.parse().map_err(|_| "bad z")?);
            let y: f32 = rest.first().and_then(|v| v.parse().ok()).unwrap_or(100.0);
            let ground = crate::vehicle::Ground { zones: area.zones.clone().unwrap_or_default(), terrain: area.terrain.clone().map(|t| { let s = t.height_scale; (t, s) }), offset: 0.0 };
            eprintln!("zone triangles: {}", ground.zones.len());
            eprintln!("tile-map ground: {:?}", ground.terrain_at(x, z));
            eprintln!("bilinear ground: {:?}", area.terrain.as_ref().map(|t| t.height_at(x, z, t.height_scale)));
            eprintln!("closest from {y}: {:?}", ground.closest([x, y, z]));
            eprintln!("ray down from {y}: {:?}", ground.line([x, y, z], [x, 0.0, z]));
            // MAO_POINTS=<file of "x y z" lines> (a capture's reported places): each one with the ground under it, a line down
            // from 3 above, and the height map's own
            if let Ok(path) = std::env::var("MAO_POINTS") {
                for line in std::fs::read_to_string(&path).map_err(|e| e.to_string())?.lines() {
                    let v: Vec<f32> = line.split_whitespace().filter_map(|n| n.parse().ok()).collect();
                    let [px, py, pz, ..] = v[..] else { continue };
                    let line_hit = ground.line([px, py + 3.0, pz], [px, py - 30.0, pz]).map_or(f32::NAN, |(h, _)| h[1]);
                    let map = area.terrain.as_ref().map_or(f32::NAN, |t| t.height_at(px, pz, t.height_scale));
                    println!("{px:.2} {py:.2} {pz:.2} {line_hit:.2} {map:.2}");
                }
            }
            // MAO_GRID=<x1>,<z1>,<step>: the heights from (x, z) to (x1, z1), a row per z, from y down
            if let Some([x1, z1, step]) = std::env::var("MAO_GRID").ok().and_then(|v| <[f32; 3]>::try_from(v.split(',').filter_map(|p| p.parse().ok()).collect::<Vec<f32>>()).ok()) {
                let mut zz = z;
                while zz <= z1 {
                    let mut row = format!("z {zz:6.1}:");
                    let mut xx = x;
                    while xx <= x1 {
                        row += &match ground.line([xx, y, zz], [xx, 0.0, zz]) {
                            Some((h, _)) => format!(" {:5.1}", h[1]),
                            None => "     -".into(),
                        };
                        xx += step;
                    }
                    println!("{row}");
                    zz += step;
                }
            }
            Ok(())
        }
        // `net <game dir> simwalk <playfield> <x> <z> <heading> <seconds>`: the vehicle run offline at 5 a second from the
        // ground at (x, z) along the heading (radians, 0 north): where it is each half second, and what happened.
        [game, word, pf, x, z, heading, seconds] if word == "simwalk" => {
            let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
            let area = load_area(game, &db, pf.parse().map_err(|_| "bad playfield")?)?;
            let ground = crate::vehicle::Ground { zones: area.zones.clone().unwrap_or_default(), terrain: area.terrain.clone().map(|t| { let s = t.height_scale; (t, s) }), offset: 0.0 };
            let (x, z, heading, seconds): (f32, f32, f32, f32) = (x.parse().map_err(|_| "bad x")?, z.parse().map_err(|_| "bad z")?, heading.parse().map_err(|_| "bad heading")?, seconds.parse().map_err(|_| "bad seconds")?);
            // (`MAO_SIM_Y`: the height to look down from, the highest face under it the start; 1000 by default)
            let (start, _, _) = ground.closest([x, std::env::var("MAO_SIM_Y").ok().and_then(|v| v.parse().ok()).unwrap_or(1000.0), z]);
            let mut pos = [x, start[1] + 0.01, z];
            let mut body = crate::vehicle::Body::default();
            let velocity = [heading.sin() * 5.0, heading.cos() * 5.0];
            let started = Instant::now();
            for frame in 0..(seconds * 60.0) as usize {
                let mut events = Vec::new();
                body.run(&ground, &mut pos, velocity, 1.0 / 60.0, &mut events);
                if frame % 30 == 0 || !events.is_empty() {
                    eprintln!("{:5.2} s ({:.2}, {:.2}, {:.2}){}{}", frame as f32 / 60.0, pos[0], pos[1], pos[2], if body.falling { " falling" } else { "" }, if events.is_empty() { String::new() } else { format!(" {events:?}") });
                }
            }
            eprintln!("{} frames in {:?}", (seconds * 60.0) as usize, started.elapsed());
            Ok(())
        }
        // `net <game dir> area <playfield>`: load a playfield as the live client does on zoning, and time it.
        [game, word, pf] if word == "area" => {
            let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
            let start = Instant::now();
            let textures = crate::Textures::new(db.clone());
            let area = load_area_with(game, &db, pf.parse().map_err(|_| "bad playfield")?, Some(&textures))?;
            eprintln!("loaded in {:.1}s: {} parts, {} booths", start.elapsed().as_secs_f32(), area.scene.parts.len(), area.booths.len());
            // MAO_COLLISION_PNG=out.png: the collision from above, two pixels a metre: red where a character standing on the
            // ground would be blocked, blue where the liquid is deep enough to swim (light blue: wading), the ground's height
            // in grey.
            if let Ok(out) = std::env::var("MAO_COLLISION_PNG")
                && let Some((lo, hi)) = area.collision.bounds()
            {
                let (w, h) = (((hi[0] - lo[0]) * 2.0) as u32 + 1, ((hi[1] - lo[1]) * 2.0) as u32 + 1);
                let mut image = image::RgbImage::new(w, h);
                for py in 0..h {
                    for px in 0..w {
                        let (x, z) = (lo[0] + px as f32 * 0.5, lo[1] + py as f32 * 0.5);
                        let ground = area.terrain.as_deref().map_or(lo[2], |t| t.height_at(x, z, t.height_scale));
                        let shade = (ground.rem_euclid(50.0) * 3.0) as u8 + 40;
                        // (blue where the liquid is deep enough to swim in, light blue where it is waded)
                        let water = crate::liquid::current().and_then(|l| l.at([x, ground, z])).map(|l| l.height - ground);
                        let colour = if area.collision.blocked(x, z, ground, 0.25, 1.8, 0.5) {
                            [230, 30, 30]
                        } else if water.is_some_and(|d| d > SWIM_DEPTH) {
                            [30, 60, 200]
                        } else if water.is_some() {
                            [120, 170, 230]
                        } else {
                            [shade; 3]
                        };
                        image.put_pixel(px, h - 1 - py, image::Rgb(colour));
                    }
                }
                image.save(&out).map_err(|e| e.to_string())?;
                eprintln!("collision picture {out}: {w} by {h}, x {:.0}..{:.0}, z {:.0}..{:.0} (north up)", lo[0], hi[0], lo[1], hi[1]);
            }
            for b in &area.booths {
                eprintln!("booth at ({:.1}, {:.1}, {:.1})", b[0], b[1], b[2]);
            }
            let t = Instant::now();
            let props = crate::static_props(&db, pf.parse().unwrap_or(0));
            eprintln!("static props: {} in {:?}", props.len(), t.elapsed());
            for (_, (mesh, at, _), door) in &props {
                if let Some(n) = door {
                    eprintln!("door {n} (mesh {mesh:#x}) at ({:.1}, {:.1}, {:.1})", at[0], at[1], at[2]);
                }
            }
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
            let _ = ITEM_DB.set(Arc::new(ResourceDatabase::open_game(std::path::Path::new(game)).map_err(|e| e.to_string())?));
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
                    // (the machines, and the items the server announces with a place: the Shadowlands' portal statues)
                    let Some(msg) = Message::parse(&m).filter(|x| m.kind == frame::N3 && (x.kind == 0x7f544905 || (x.kind == 0x3b11256f && x.identity.0 == 0xc73d))) else { continue };
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
        // (MAO_ATTACK_RANGE: stop this far off, to see what the server says about range)
        if left <= std::env::var("MAO_ATTACK_RANGE").ok().and_then(|v| v.parse().ok()).unwrap_or(2.5f32) {
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
    // (MAO_WALK_ZONES=1: the height from the zone surfaces, a line down from just above the feet, as the live client's vehicle)
    let zones = std::env::var_os("MAO_WALK_ZONES").map(|_| crate::vehicle::Ground { zones: area.zones.clone().unwrap_or_default(), terrain: area.terrain.clone().map(|t| { let s = t.height_scale; (t, s) }), offset: 0.0 });
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
    // MAO_WALK_USE=<text>: at the end of the walk, use the nearest table dynel whose name has the text (a bank terminal)
    // and print what the server answers.
    let use_wanted = std::env::var("MAO_WALK_USE").ok().map(|w| w.to_ascii_lowercase());
    let table = crate::table_dynels(&db, playfield);
    let mut used_at: Option<Instant> = None;
    let end = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < end {
        match connection.receive() {
            Ok(frame) => {
                if let Some(reply) = frame::ping_reply(&frame, own, 70_000_000 + clock.elapsed().as_millis() as u32) {
                    connection.send(&reply).map_err(|e| e.to_string())?;
                }
                if let Some(target) = system::redirect(&frame) {
                    eprintln!("[{:6.1}] the server redirects us to {target}", clock.elapsed().as_secs_f32());
                    // (as the original: straight to the new server with the same key; a new login only if that fails)
                    let mut next = hop_direct(&target, own, ticket.key).or_else(|_| hop(game, &target, own, &|t| eprintln!("{t}")))?;
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
                if let (Some(m), Some(_)) = (Message::parse(&frame), used_at) {
                    if m.kind != n3::POSITION_UPDATE && m.kind != 0x5e477770 || m.identity.1 == own {
                        eprintln!("[{:6.1}] <- {:08x} ({:x},{:x}) {} bytes {}", clock.elapsed().as_secs_f32(), m.kind, m.identity.0, m.identity.1, m.data.len(), m.data.iter().take(48).map(|b| format!("{b:02x}")).collect::<String>());
                    }
                    if m.kind == BANK_MESSAGE {
                        let entries = mao_net::schema::Schema::global().parse_n3(m.kind, &m.data).map(|p| inventory_from_schema(p.value.get("BankSlots"))).unwrap_or_default();
                        eprintln!("  BANK: {} items, first {:?}", entries.len(), entries.iter().take(3).map(|e| (e.slot, e.container, e.low_id)).collect::<Vec<_>>());
                    }
                }
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
        if let (false, None, Some(w)) = (walking, used_at, use_wanted.as_ref()) {
            let near = |p: &[f32; 3]| ((p[0] - position[0]).powi(2) + (p[1] - position[1]).powi(2) + (p[2] - position[2]).powi(2)).sqrt();
            match table.iter().filter(|d| d.4.to_ascii_lowercase().contains(w)).min_by(|a, b| near(&a.2).total_cmp(&near(&b.2))) {
                Some(d) => {
                    eprintln!("[{:6.1}] USE {:?} ({:#x}, {:#x}) at {:?}, {:.2} m from us at {:?}", clock.elapsed().as_secs_f32(), d.4, d.0, d.1, d.2, near(&d.2), position);
                    connection.send(&n3::use_object(own, 1, (d.0, d.1))).map_err(|e| e.to_string())?;
                }
                None => eprintln!("no {w:?} in playfield {playfield}'s table"),
            }
            used_at = Some(Instant::now());
        }
        if used_at.is_some_and(|t| t.elapsed() > Duration::from_secs(8)) {
            break;
        }
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
                if let Some(z) = zones.as_ref() {
                    if let Some((hit, _)) = z.line([position[0], position[1] + 1.5, position[2]], [position[0], position[1] - 8.0, position[2]]) {
                        base = hit[1];
                    }
                }
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
                    Some(x) if m.kind == frame::N3 => println!("#{ordinal} {} {dir} {connection} kind {:08x} identity ({:x},{:x}) {} bytes: {}", f[0], x.kind, x.identity.0, x.identity.1, x.data.len(), hex(&x.data)),
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

    /// The Arete Landing capture's `SpecialAttackWeapon` for the character: the plain attack (100), Dimach (144) and
    /// Brawl (142) are its listed specials; another character's list is not taken for one's own.
    #[test]
    fn the_servers_special_attacks_are_listed() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x7b1a0021;
        let data = hex("0100000fc40000aac000023569000000644d4141540000a4310000a430000000904449495400011294000112950000008e425241570000000500000005000000050000000600000064");
        let mut w = World { own, ..World::default() };
        w.add(&Message { kind: SPECIAL_ATTACK_WEAPON, identity: (0xc350, own + 1), data: data.clone() }.frame(1));
        assert!(w.specials.is_none() && !w.specials_changed);
        w.add(&Message { kind: SPECIAL_ATTACK_WEAPON, identity: (0xc350, own), data: data.clone() }.frame(1));
        assert_eq!(w.specials.as_deref(), Some(&[100, 144, 142][..]));
        assert!(w.specials_changed);
        // (told again the same: nothing changed)
        w.specials_changed = false;
        w.add(&Message { kind: SPECIAL_ATTACK_WEAPON, identity: (0xc350, own), data }.frame(1));
        assert!(!w.specials_changed);
    }

    /// `MAO_GAME`: what may go to a wear place (`N3Msg_IsItemPossibleToWear`): a nano crystal nowhere and a rifle not in
    /// a place its placement mask lacks, both without a word; the rifle in the right hand is not refused that way.
    #[test]
    fn only_what_may_be_worn_goes_to_a_wear_place() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        let _ = ITEM_DB.set(db.clone());
        let entry = |slot: u32, low_id: u32| inventory::Entry { slot, flags: 0, count: 1, container: (0, 0), low_id, high_id: low_id, quality: 1, extra: 0 };
        let w = World { own: 7, ..World::default() };
        let (crystal, rifle) = (entry(0x45, 0x46147), entry(0x46, 0x1dae1));
        assert_eq!(w.wear_refusal(&crystal, 6, &db), Some(None));
        assert_eq!(w.wear_refusal(&rifle, 0x11, &db), Some(None));
        assert_ne!(w.wear_refusal(&rifle, 6, &db), Some(None));
    }

    /// An item move the server refuses is taken back, one it takes stays (the two answers seen live on 2026-11-10: a nano
    /// crystal dragged to the right hand, then a rifle).
    #[test]
    fn a_refused_item_move_is_taken_back() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x7b1e5338;
        let entry = |slot: u32, low_id: u32| inventory::Entry { slot, flags: 0, count: 1, container: (0, 0), low_id, high_id: low_id, quality: 1, extra: 0 };
        let places = |w: &World| { let mut v: Vec<(u32, u32)> = w.worn.iter().map(|e| (e.slot, e.low_id)).collect(); v.sort(); v };
        let mut w = World { own, ..World::default() };
        // (the crystal shown in the hand at once, as the loop does on asking)
        w.worn = vec![entry(6, 0x46147), entry(0x46, 0x1dae1)];
        w.moves_asked.push((0x45, 6, false));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000009200000000000000680000004500000000000000000000") }.frame(1));
        assert_eq!((places(&w), w.moves_asked.len()), (vec![(0x45, 0x46147), (0x46, 0x1dae1)], 0));
        // (the rifle: asked, shown, and the server's `ContainerAddItem` names the old place and the new)
        w.worn[1].slot = 6;
        w.moves_asked.push((0x46, 6, false));
        w.add(&Message { kind: CONTAINER_ADD_ITEM, identity: (0xc350, own), data: hex("0000000068000000460000c3507b1e533800000006") }.frame(1));
        assert_eq!((places(&w), w.moves_asked.len()), (vec![(6, 0x1dae1), (0x45, 0x46147)], 0));
    }

    /// The server's pet duel messages (the two seen live on 2026-11-10: no duel pet, and not challenged) are told to
    /// the character they are for, once each; a code the original has no text for is not told.
    #[test]
    fn pet_duel_messages_are_told() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x795ab032;
        let mut w = World { own, ..World::default() };
        let (no_pet, not_asked) = ("00000000f0000000000000c3500009210c00000005000000000000", "00000000f000000000000000000000000000000004000000000000");
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own + 1), data: hex(no_pet) }.frame(1));
        assert!(w.game_lines.is_empty());
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex(no_pet) }.frame(1));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex(not_asked) }.frame(1));
        let lines: Vec<String> = w.game_lines.drain(..).map(|l| l.text).collect();
        assert!(lines.len() == 2 && lines[0] != lines[1] && lines.iter().all(|l| !l.is_empty()), "{lines:?}");
        // (an answer's code past the seven, and a result's past the three)
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("00000000f000000000000000000000000000000007000000000000") }.frame(1));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("00000000f300000000000000000000000000000003000000000000") }.frame(1));
        assert!(w.game_lines.is_empty());
        // (the texts by what is known of the other: named, or not)
        assert_ne!(crate::feedback::pet_duel(0xf0, 1, Some("Other")), crate::feedback::pet_duel(0xf0, 1, None));
        assert!(crate::feedback::pet_duel(0xf8, 0, Some("Other")).is_some_and(|l| l.contains("Other")));
        assert!(crate::feedback::pet_duel(0xef, 0, Some("Other")).is_some_and(|l| l.contains("Other")));
        assert_eq!(crate::feedback::pet_duel(0xf1, 0, None), None);
    }

    /// `MAO_GAME`: the weapons in the hands add their attacks and Reload to the server's list (the two characters of the
    /// live test of 2026-11-10: a Neutrino Flash in the right hand; a Kyr'Ozch Energy Pistol and an Electrical Surge
    /// Pistol), and a weapon taken off takes its own away.
    #[test]
    fn the_weapons_in_the_hands_give_their_attacks() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let _ = ITEM_DB.set(Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap()));
        let entry = |slot: u32, low_id: u32| inventory::Entry { slot, flags: 0, count: 1, container: (0, 0), low_id, high_id: low_id, quality: 1, extra: 0 };
        let mut w = World { own: 7, ..World::default() };
        w.worn.push(entry(6, 0x251db));
        // (nothing before the server has listed its own)
        assert_eq!(w.special_numbers(), None);
        w.specials = Some(vec![100, 144, 142]);
        assert_eq!(w.special_numbers(), Some(vec![100, 144, 142, 148, 110]));
        // (two pistols: the one with burst and fling shot is in the left hand; an item elsewhere gives nothing)
        w.worn = vec![entry(6, 0x3e2a9), entry(8, 0x3ca17), entry(3, 0x251db), entry(inventory::BACKPACK_FIRST, 0x251db)];
        assert_eq!(w.special_numbers(), Some(vec![100, 144, 142, 148, 150, 110]));
        // (the server says the left hand's takes no ammunition, and the right hand's has none of its own: Reload stays
        // for the right hand's; with both at -1 it goes)
        w.weapon_energy.insert((7, 8), -1);
        assert_eq!(w.special_numbers(), Some(vec![100, 144, 142, 148, 150, 110]));
        w.weapon_energy.insert((7, 6), -1);
        assert_eq!(w.special_numbers(), Some(vec![100, 144, 142, 148, 150]));
        // (bare hands: the server's only)
        w.worn.clear();
        assert_eq!(w.special_numbers(), Some(vec![100, 144, 142]));
        // (a blade with sneak attack and fast attack; its template has no ammunition stat and the server has told none,
        // so Reload stays. Backstab comes beside the sneak attack once the server has said so, and only then)
        w.weapon_energy.clear();
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        w.worn.push(entry(6, 0x1dae4));
        assert_eq!(w.special_numbers(), Some(vec![100, 144, 142, 146, 147, 110]));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, 7), data: hex("00000000b500000000000000000000000000000000000000000000") }.frame(1));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, 8), data: hex("00000000b500000000000000000000000000000000000000010000") }.frame(1));
        assert_eq!(w.special_numbers(), Some(vec![100, 144, 142, 146, 147, 110]));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, 7), data: hex("00000000b500000000000000000000000000000000000000010000") }.frame(1));
        assert_eq!(w.special_numbers(), Some(vec![100, 144, 142, 146, 489, 147, 110]));
        // (no backstab without a sneak attack weapon)
        w.worn = vec![entry(6, 0x251db)];
        assert_eq!(w.special_numbers(), Some(vec![100, 144, 142, 148, 110]));
    }

    /// One's own open backpack: an item of the inventory goes into it with a `ContainerAddItem` naming the bag, and
    /// comes out with one naming the bag's handle; the shared container window's contents are left alone.
    #[test]
    fn items_go_in_and_out_of_an_own_backpack() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x7b1a0021;
        let bag = (crate::loot::BAG, 0x4d);
        let entry = |slot: u32, container: (u32, u32), low_id: u32| inventory::Entry { slot, flags: 0, count: 1, container, low_id, high_id: low_id, quality: 1, extra: 0 };
        let mut w = World { own, ..World::default() };
        w.worn.push(entry(inventory::BACKPACK_FIRST, bag, 100));
        w.worn.push(entry(inventory::BACKPACK_FIRST + 1, (0, 0), 200));
        w.bags.insert(bag, Loot { identity: bag, handle: 0x71, entries: vec![entry(0, (0, 0), 300)] });
        w.loot = Some(Loot { identity: (0xc76a, 5), handle: 0x70, entries: vec![entry(0, (0, 0), 400)] });
        // (`(0x68, 0x41)` into the bag `(0xc749, 0x4d)`: the item takes the bag's first free place)
        w.add(&Message { kind: CONTAINER_ADD_ITEM, identity: (0xc350, own), data: hex("0000000068000000410000c7490000004d00000000") }.frame(1));
        assert_eq!(w.bags[&bag].entries.iter().map(|e| (e.slot, e.low_id)).collect::<Vec<_>>(), [(0, 300), (1, 200)]);
        assert!(w.worn.len() == 1 && w.bags_changed);
        // (`(0x6b, 0x71 << 16 | 0)`: the bag's item at place 0 comes into the inventory's first free place)
        w.bags_changed = false;
        w.add(&Message { kind: CONTAINER_ADD_ITEM, identity: (0xc350, own), data: hex("000000006b007100000000c3507b1a00210000006f") }.frame(1));
        assert_eq!(w.bags[&bag].entries.iter().map(|e| e.low_id).collect::<Vec<_>>(), [200]);
        assert_eq!(w.worn.last().map(|e| (e.slot, e.low_id)), Some((inventory::BACKPACK_FIRST + 1, 300)));
        assert!(w.bags_changed && w.loot.as_ref().is_some_and(|l| l.entries.len() == 1));
    }

    /// The Arete Landing capture: a quest item named into the overflow window comes in with its `ContainerAddItem`, and a
    /// bought one with `AddTemplate`; each takes the backpack's first free place.
    #[test]
    fn items_come_into_the_backpack() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x7b1a0021;
        let mut w = World { own, ..World::default() };
        w.worn.push(inventory::Entry { slot: inventory::BACKPACK_FIRST, flags: 0, count: 1, container: (0, 0), low_id: 1, high_id: 1, quality: 1, extra: 0 });
        w.add(&Message { kind: TEMPLATE_ACTION, identity: (0xc350, own), data: hex("0000048854000488540000000100000001000000570000006e000000000000000000000000") }.frame(1));
        assert!(w.worn.len() == 1, "nothing comes in before its ContainerAddItem");
        w.add(&Message { kind: CONTAINER_ADD_ITEM, identity: (0xc350, own), data: hex("000000006e000000000000006e7b1a00210000006f") }.frame(1));
        assert_eq!(w.worn.last().map(|e| (e.slot, e.low_id)), Some((inventory::BACKPACK_FIRST + 1, 0x48854)));
        w.add(&Message { kind: ADD_TEMPLATE, identity: (0xc350, own), data: hex("000001fe3f0001fe3f0000000100000001") }.frame(1));
        assert_eq!(w.worn.last().map(|e| (e.slot, e.low_id, e.count)), Some((inventory::BACKPACK_FIRST + 2, 0x1fe3f, 1)));
        assert!(w.inventory.is_some());
        // (a worn armour piece at place 0x11 goes from quality 2 to 3 at a level, as the capture's ItemReplaced says)
        w.worn.push(inventory::Entry { slot: 0x11, flags: 0, count: 1, container: (0, 0), low_id: 0x4a09b, high_id: 0x4a09c, quality: 2, extra: 0 });
        w.add(&Message { kind: 0x3a223b50, identity: (0xc350, own), data: hex("00000000110004a09b0004a09c00000003000000000004a09b0004a09c0000000300000000") }.frame(1));
        assert_eq!(w.worn.iter().find(|e| e.slot == 0x11).map(|e| e.quality), Some(3));
    }

    /// The Arete Landing capture's nano crystals: each use is answered by `CharacterAction` 0xcc naming the nano, which joins
    /// the programs once.
    #[test]
    fn an_uploaded_nano_joins_the_programs() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x7b1a0021;
        let mut w = World { own, program_ids: vec![0x461f6], ..World::default() };
        let upload = Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("00000000cc000000000000c3507b1a00210000cf1b0000a96a0000") }.frame(1);
        w.add(&upload);
        assert_eq!(w.program_ids, vec![0x461f6, 0xa96a]);
        assert!(w.programs_changed);
        w.programs_changed = false;
        w.add(&upload);
        assert_eq!(w.program_ids.len(), 2, "an uploaded nano is listed once");
        assert!(!w.programs_changed);
    }

    /// The Arete Landing capture's second ship door, as announced (`DoorFullUpdate`, identity `0x10c32dc2`).
    const DOOR_BYTES: &str = "000000000b000000000000000041f0020d41ab31c4434d0000000000003f3504f1000000003f3504f50020da1b000f424f00000001006f00002f4c00000000c0081403000000170003de4d000002bd00000000000002be00000000000002bf000000000000019c00000001000000fc00000000000000c000000000000000c100000000000000c3000000000000010300000000000000000000000200000032000003f10000000200010000";

    /// The Arete Landing capture's generated ship (playfield 0x20da1b): its second door's `DoorFullUpdate` puts a usable door at
    /// (30, 21.4, 205), where the test `the_ship_rooms_meet_at_the_servers_doors` has it.
    #[test]
    fn a_generated_missions_doors_are_usable() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        // (`MAO_GAME`: the door's template is looked up, so the door gets its model)
        if let Some(game) = std::env::var_os("MAO_GAME") {
            let _ = ITEM_DB.set(std::sync::Arc::new(ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap()));
        }
        let mut w = World { own: 0x7b1a0021, ..World::default() };
        let door = 0x10c32dc2;
        w.add(&Message { kind: DOOR_FULL_UPDATE, identity: (DOOR_TYPE, door), data: hex(DOOR_BYTES) }.frame(1));
        let at = w.objects.get(&door).copied().expect("the door is a usable object");
        assert!((at[0] - 30.0).abs() < 0.01 && (at[1] - 21.4).abs() < 0.01 && (at[2] - 205.0).abs() < 0.01, "{at:?}");
        assert_eq!(w.names.get(&door).map(|n| n.0.as_str()), Some("Door"));
        assert_eq!(w.types.get(&door), Some(&DOOR_TYPE));
        // (its template, stat 23 of the announcement, names the model it is drawn with when the game's database is at hand)
        assert_eq!(w.door_templates.get(&door), Some(&0x3de4d));
        if let Some(mesh) = template_stat(0x3de4d, 12) {
            eprintln!("the door's template {:#x} has mesh {mesh:#x}", 0x3de4d);
            assert!(mesh > 0, "the door template names a model");
            assert_eq!(w.props.get(&door).map(|p| (p.0, p.1)), Some((mesh as u32, at)));
        } else if std::env::var_os("MAO_GAME").is_some() {
            panic!("the door's template has no Mesh stat");
        }
        // (the door is numbered and placed like the table's doors; its flags, stat 259, say it is shut: 0)
        let n = w.door_number(door).expect("a number");
        assert_eq!(w.door_slots.get(&n), Some(&(door, at)));
        assert!(!w.doors_open.contains_key(&n));
        // (the next door of the run announced open: flags 0x80)
        let open_door = door + 1;
        let open_bytes = DOOR_BYTES.replace("0000010300000000", "0000010300000080");
        assert_ne!(open_bytes, DOOR_BYTES);
        w.add(&Message { kind: DOOR_FULL_UPDATE, identity: (DOOR_TYPE, open_door), data: hex(&open_bytes) }.frame(1));
        let n_open = w.door_number(open_door).expect("a number");
        assert_ne!(n_open, n);
        assert!(w.doors_open.get(&n_open).is_some_and(|until| *until > Instant::now()));
    }

    /// The 16:55 capture's Malfunctioning Cleaning Robot: its two `SpellList`s make their effects on it; Cedric Harding's
    /// event loop (a nano cast) makes none.
    #[test]
    fn a_spell_list_makes_its_effects() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let mut w = World { own: 0x7b1a0021, ..World::default() };
        w.add(&Message { kind: crate::spelllist::SPELL_LIST, identity: (0xc350, 0x7b1a6433), data: hex("00000007e20000cf3b00048329000000040000000000000001000000000000000200000009000483290000c3507b1a64330000c3507b1a64330000194576656e74204c6f6f70204365647269632048617264696e67010000cf1b000483290000000000") }.frame(1));
        assert!(w.spell_effects_made.is_empty());
        w.add(&Message { kind: crate::spelllist::SPELL_LIST, identity: (0xc350, 0x7b1a6629), data: hex("00000007e20000cf2644fb183e0000000400000000000000010000000000000000000000000000a8e300000000000000000000000000000000000000000000000000000000000000000000c3507b1a6629000000000000000000") }.frame(1));
        w.add(&Message { kind: crate::spelllist::SPELL_LIST, identity: (0xc350, 0x7b1a6629), data: hex("00000007e20000cf2644fb183f0000000400000000000000010000000000000000000000000000a87100000000000000000000000000000000000000000000000000000000000000000000c3507b1a6629000000000000000000") }.frame(1));
        assert_eq!(w.spell_effects_made, vec![(0x7b1a6629, 0xa8e3, None), (0x7b1a6629, 0xa871, None)]);
    }

    /// A spell list's stun on one's own character holds it still until the list ends it (state flags 2 and 4 of stat 224); a
    /// stat set lands in its stats.
    #[test]
    fn a_stun_holds_ones_own_character() {
        let mut w = World { own: 0x7b1a0021, ..World::default() };
        let me = (crate::requirements::CHARACTER, w.own);
        w.stats.insert(224, 6);
        assert!(!w.own_stunned());
        w.spell_outcomes(vec![(me, crate::spelllist::Outcome::Stun { begin: true, npc: false })]);
        assert!(w.own_stunned());
        w.spell_outcomes(vec![(me, crate::spelllist::Outcome::Stun { begin: false, npc: false })]);
        assert!(!w.own_stunned());
        // (another character's stun does not hold one's own)
        w.spell_outcomes(vec![((crate::requirements::CHARACTER, 5), crate::spelllist::Outcome::Stun { begin: true, npc: true })]);
        assert!(!w.own_stunned());
        w.spell_outcomes(vec![(me, crate::spelllist::Outcome::SetStat { stat: 689, value: 3 }), ((crate::requirements::CHARACTER, 5), crate::spelllist::Outcome::SetStat { stat: 690, value: 1 })]);
        assert_eq!((w.stats.get(&689), w.stats.get(&690)), (Some(&3), None));
    }

    /// The Arete Landing capture's team: the invitation from Nihiltech (0x7b1a0022), both members, a health update, then
    /// leaving.
    #[test]
    fn the_team_follows_the_server() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x7b1a0021;
        let mut w = World { own, ..World::default() };
        w.names.insert(0x7b1a0022, ("Nihiltech".into(), false));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000001a000000000000c3507b1a002200000000000000000000") }.frame(1));
        assert_eq!(crate::team::with(|t| t.invite.clone()), Some((0x7b1a0022, "Nihiltech".to_string())));
        w.add(&Message { kind: TEAM_MEMBER, identity: (0xc350, own), data: hex("000000c3507b1a00210000dea902a3b8ffffffffff0000000500040000000b5768657265736d796e6375") }.frame(1));
        w.add(&Message { kind: TEAM_MEMBER, identity: (0xc350, own), data: hex("000000c3507b1a00220000dea902a3b8ffffffffff000000070003000000094e6968696c74656368") }.frame(1));
        w.add(&Message { kind: TEAM_MEMBER_INFO, identity: (0xc350, own), data: hex("000000c3507b1a0022000000c4000000d1000000e4000000d7") }.frame(1));
        let team = crate::team::with(|t| t.clone());
        assert_eq!(team.members.iter().map(|m| (m.name.as_str(), m.level)).collect::<Vec<_>>(), vec![("Wheresmyncu", 5), ("Nihiltech", 7)]);
        assert_eq!((team.members[1].health, team.members[1].nano), ((196, 209), (215, 228)));
        // (0x23 names the leader, as the 15:05 capture's does after joining: the target and the team; the 16:55 capture's 0x1b
        // gives the team level 2 in the target's instance, told in the chat)
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("0000000023000000000000c3507b1a00220000dea902a3b8b20000") }.frame(1));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000001b00000000000000000000000200000000000000000000") }.frame(1));
        let team = crate::team::with(|t| t.clone());
        assert_eq!((team.leader, team.level), (Some(0x7b1a0022), Some(2)));
        assert!(!crate::team::is_leader(&team, 0) && crate::team::is_leader(&team, 1));
        assert!(w.game_lines.last().is_some_and(|l| l.text.contains('2')), "{:?}", w.game_lines.last().map(|l| &l.text));
        // (leaving: the server's 0x20 naming oneself)
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("0000000020000000000000c3507b1a002102a3b8ffffffffff0000") }.frame(1));
        assert!(crate::team::with(|t| t.members.is_empty() && t.leader.is_none()));
    }

    /// The 16:55 capture's other players: Nihiltech (0x7b1a0022) sits and the server's 0x57 stands him up; he equips a weapon
    /// (0x83 naming it, place 6); his Fling Shot (`CharSecSpecAttack` 0x96, then `SpecialAttackInfo`) swings the special's class.
    #[test]
    fn others_stand_up_wield_and_swing_their_specials() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let (own, other) = (0x7b1a0021, 0x7b1a0022);
        let mut w = World { own, ..World::default() };
        w.add(&Message { kind: n3::POSITION_UPDATE, identity: (0xc350, other), data: hex("001e000000003f32e84e000000003f371b5b4561914842510f5b44447cb3000000000000000000000000") }.frame(1));
        assert_eq!(w.held_action(other), Some(0xd5));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, other), data: hex("000000005700000000000000000000000000000000000000000000") }.frame(1));
        assert_eq!(w.held_action(other), None);
        w.weapon_templates.insert(0x25f1e6f5, 0x1dae3);
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, other), data: hex("0000000083000000000000c74a25f1e6f500000000000000060000") }.frame(1));
        assert_eq!(w.weapons.get(&(other, 6)), Some(&0x1dae3));
        w.add(&Message { kind: 0x51492120, identity: (0xc350, other), data: hex("000000c3507b1a670a00000096") }.frame(1));
        w.add(&Message { kind: 0x754f1115, identity: (0xc350, other), data: hex("000000000600000004ffffffff0000c3507b1a670a0000009600000000") }.frame(1));
        assert_eq!(w.swings[&other].3, mao_formats::item::special_class(0x96).unwrap().0);
        assert!(w.queued_specials.is_empty(), "the special is used by its attack");
    }

    /// The dynel flag 0x800: the captures' `CharacterAction` 0xa7 (all with parameter 2 zero) clears it, and equipping a weapon
    /// (0x83) in place 6 then empties place 0 (FUN_1006ac49); with the flag set (0xa7 with 1, or a full update's bit 21) place 0
    /// is kept.
    #[test]
    fn the_flag_0x800_keeps_the_weapon_in_place_zero() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let (own, other) = (0x7b1a0021, 0x7b1a0022);
        let mut w = World { own, ..World::default() };
        w.weapon_templates.insert(0x25f1e6f5, 0x1dae3);
        let equip = Message { kind: 0x5e477770, identity: (0xc350, other), data: hex("0000000083000000000000c74a25f1e6f500000000000000060000") }.frame(1);
        w.weapons.insert((other, 0), 0x1234);
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, other), data: hex("00000000a700000000000000000000000000000000000000000000") }.frame(1));
        w.add(&equip);
        assert_eq!((w.weapons.get(&(other, 6)), w.weapons.get(&(other, 0))), (Some(&0x1dae3), None));
        w.weapons.insert((other, 0), 0x1234);
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, other), data: hex("00000000a700000000000000000000000000000000000000010000") }.frame(1));
        assert!(w.weapon_flag.contains(&other));
        w.add(&equip);
        assert_eq!(w.weapons.get(&(other, 0)), Some(&0x1234));
    }

    /// The 15:05 capture's `PetToMaster`: command 1 with 0x0a for each pet (the names flash ten times), then command 2 for each
    /// (the flashing stops). Only to one's own character.
    #[test]
    fn pet_to_master_flashes_the_pets_names() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x4bc82;
        let mut w = World { own, ..World::default() };
        let tell = |w: &mut World, to: u32, h: &str| w.add(&Message { kind: 0x0d381f02, identity: (0xc350, to), data: hex(h) }.frame(1));
        tell(&mut w, own, "010000c3507b1e6d64000000010000000a0000000000000000");
        tell(&mut w, own, "010000c3507b1e6d65000000010000000a0000000000000000");
        tell(&mut w, 0x4bc83, "010000c3507b1e6d66000000010000000a0000000000000000");
        assert_eq!(w.pet_flashes.iter().map(|(&p, &(n, _))| (p, n)).collect::<Vec<_>>(), vec![(0x7b1e6d64, 10), (0x7b1e6d65, 10)]);
        tell(&mut w, own, "010000c3507b1e6d6400000002000000000000000000000000");
        assert_eq!(w.pet_flashes.keys().copied().collect::<Vec<_>>(), vec![0x7b1e6d65]);
    }

    /// Sleeping and lounging: a seated character's `SocialActionCmd` 0x44 lays it down (`WaitState` 0xf, `sleep-ground` held),
    /// 0x45 lounges it (0x10, `lounging`); `CharacterAction` 0x57 then sits it up again (back to `ground-start`), and a second
    /// 0x57 stands it. (The social message as the captures' `3b290771`: a byte, the type 1, a word, the action.)
    #[test]
    fn sleeping_and_lounging_end_sitting_again() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let (own, other) = (0x7b1a0021, 0x7b1a0022);
        let mut w = World { own, ..World::default() };
        let sit = hex("001e000000003f32e84e000000003f371b5b4561914842510f5b44447cb3000000000000000000000000");
        let stand = || Message { kind: 0x5e477770, identity: (0xc350, other), data: hex("000000005700000000000000000000000000000000000000000000") }.frame(1);
        w.add(&Message { kind: n3::POSITION_UPDATE, identity: (0xc350, other), data: sit.clone() }.frame(1));
        w.add(&Message { kind: 0x3b290771, identity: (0xc350, other), data: hex("00000000010000000000000044") }.frame(1));
        assert_eq!((w.resting.get(&other), w.held_action(other)), (Some(&0xf), Some(0xed)));
        w.add(&stand());
        assert_eq!(w.held_action(other), Some(0xd5));
        w.add(&Message { kind: 0x3b290771, identity: (0xc350, other), data: hex("00000000010000000000000045") }.frame(1));
        assert_eq!(w.held_action(other), Some(0xef));
        w.add(&stand());
        w.add(&stand());
        assert_eq!(w.held_action(other), None);
        // (getting up by moving ends lounging too)
        w.add(&Message { kind: n3::POSITION_UPDATE, identity: (0xc350, other), data: sit }.frame(1));
        w.add(&Message { kind: 0x3b290771, identity: (0xc350, other), data: hex("00000000010000000000000045") }.frame(1));
        w.add(&Message { kind: n3::POSITION_UPDATE, identity: (0xc350, other), data: hex("0025000000003f32e84e000000003f371b5b4561914842510f5b44447cb3000000000000000000000000") }.frame(1));
        assert!(w.resting.is_empty() && w.held_action(other).is_none());
        // (`MAO_GAME`: the bodies have both)
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = std::sync::Arc::new(ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        let mut chars = Characters::new(db).unwrap();
        for action in [0xed, 0xef] {
            assert!(chars.ensure_action_file(1, mao_formats::appearance::FEMALE, action).is_some(), "{action:#x}");
        }
    }

    /// The 08:42 capture's perk actions: each use is answered by `QueuePerk` with its delay (1 s, then 2 s), which runs the
    /// "Special" timer bar for that long.
    #[test]
    fn a_queued_perk_runs_the_special_timer() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x4bc80;
        let mut w = World { own, ..World::default() };
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000005000000000000000000000000000000002000000c80000") }.frame(1));
        assert_eq!(w.queued_perk, Some(200));
        let e = w.timers.get(0).unwrap().clone();
        assert_eq!((e.kind, e.first, e.second, e.flags), (crate::timers::SPECIAL, 200, 0, 0x1b));
        w.timers.tick(0.0, true);
        w.timer_bars.update(&mut w.timers);
        assert_eq!(w.timer_bars.shown(&w.timers), vec![(0, crate::timers::SPECIAL, 0.0)]);
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000005000000000000000000000000000000000000000000000") }.frame(1));
        w.timer_bars.update(&mut w.timers);
        assert!(!w.timers.has(crate::timers::SPECIAL) && w.timer_bars.bars.is_empty());
    }

    /// The 08:42 capture's fight with two weapons (places 6 and 8): the server's echo of one's own `Attack` starts an attack
    /// timer per weapon (two bars, as the capture's screenshots show at the top left), each attack in a place (`AttackInfo`'s
    /// `WeaponSlot`) lets that weapon's held timer recharge, and `StopFight` takes the attack bars away.
    #[test]
    fn a_fight_runs_an_attack_timer_per_weapon() {
        use crate::timers::{ATTACK, HELD};
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x4bc80;
        let mut w = World { own, ..World::default() };
        w.weapons.insert((own, 6), 1);
        w.weapons.insert((own, 8), 2);
        w.add(&Message { kind: 0x28494070, identity: (0xc350, own), data: hex("000000c3507b1d0e0000") }.frame(1));
        let slots = |w: &World| w.timers.entries.iter().flatten().filter(|e| e.kind == ATTACK).map(|e| e.tag).collect::<Vec<_>>();
        assert_eq!(slots(&w), vec![6, 8]);
        w.timer_bars.update(&mut w.timers);
        assert_eq!(w.timer_bars.bars.iter().map(|b| b.place).collect::<Vec<_>>(), vec![0, 1]);
        // (held after its first phase, the weapon in place 8 recharges once its attack comes)
        let p8 = w.timers.find(ATTACK, 8).unwrap();
        w.timers.entries[p8].as_mut().unwrap().flags = (w.timers.entries[p8].as_ref().unwrap().flags | HELD) & !crate::timers::RUNNING;
        w.add(&Message { kind: 0x46002f16, identity: (0xc350, own), data: hex("0000000738ffffffff000000080000c3507b1d0e00000000000000000300000000") }.frame(1));
        assert_eq!(w.timers.get(p8).unwrap().flags & HELD, 0);
        w.add(&Message { kind: 0x4a41203e, identity: (0xc350, own), data: hex("0000000001") }.frame(1));
        w.timer_bars.update(&mut w.timers);
        assert!(w.timer_bars.bars.is_empty() && w.timers.attacks_stopped);
        assert_eq!(slots(&w).len(), 2, "the entries stay");
    }

    /// The 16:55 capture's `/sneak` at Arete Landing: the server names each creature that sees the character (0x9b), then puts
    /// it in the sneak mode (0xa2).
    #[test]
    fn sneaking_tells_who_detected_the_character() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x7b1a0021;
        let mut w = World { own, ..World::default() };
        w.names.insert(0x7b08289d, ("Patrolling Guard".into(), false));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000009b000000000000c3507b08289d00000000000000000000") }.frame(1));
        // (one the client does not know says nothing)
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000009b000000000000c3507b08293800000000000000000000") }.frame(1));
        assert_eq!(w.game_lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), vec!["You have been detected by Patrolling Guard!"]);
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("00000000a200000000000000000000000000000000000000000000") }.frame(1));
        assert!(w.sneaking.contains(&own));
    }

    /// The 16:55 capture's area nanos: one landing on the character (from 0x7b1b1847), one on two others.
    #[test]
    fn an_area_formula_lands_on_each_listed_character() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x7b1a0021;
        let mut w = World { own, ..World::default() };
        w.add(&Message { kind: 0x3129233b, identity: (0xc350, own), data: hex("00000007e20000c3507b1a00210003e3800000c3507b1b1847") }.frame(1));
        assert_eq!(w.landed, vec![(own, 0x3e380)]);
        w.landed.clear();
        w.names.insert(0x7b1ab833, ("A".into(), false));
        w.names.insert(0x7b1abca9, ("B".into(), false));
        w.add(&Message { kind: 0x3129233b, identity: (0xc350, 0x7b1a00b3), data: hex("0000000bd30000c3507b1ab8330000c3507b1abca9000187620000c3507b1a00b3") }.frame(1));
        assert_eq!(w.landed, vec![(0x7b1ab833, 0x18762), (0x7b1abca9, 0x18762)]);
    }

    /// The 16:55 capture's sale: `InventoryUpdated` to the shop and to the character; only the character's refreshes the inventory.
    #[test]
    fn inventory_updated_refreshes_ones_own_inventory() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x7b1a0021;
        let mut w = World { own, ..World::default() };
        w.add(&Message { kind: 0x485e7202, identity: (0xc75b, 0x1324d835), data: hex("0000000005") }.frame(1));
        assert!(w.inventory.is_none());
        w.add(&Message { kind: 0x485e7202, identity: (0xc350, own), data: hex("0000000005") }.frame(1));
        assert!(w.inventory.is_some());
    }

    /// The 15:05 capture's cancelled logout: the server's `StopLogout` (0x7a) takes the logout bar away and says so.
    #[test]
    fn stop_logout_ends_the_logout_bar() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let own = 0x4bc82;
        let mut w = World { own, ..World::default() };
        *crate::ui::CAMP.lock().unwrap() = Some(Instant::now());
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000007a00000000000000000000000000000000000000000000") }.frame(1));
        assert!(crate::ui::CAMP.lock().unwrap().is_none());
        assert_eq!(w.game_lines.len(), 1);
        // (again, with no bar: nothing more to say)
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000007a00000000000000000000000000000000000000000000") }.frame(1));
        assert_eq!(w.game_lines.len(), 1);
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
        // (a captured death: 0x7b017c2a dies kneeling, 500, and is despawned ten seconds on; it stays lying there as its
        // corpse, announced where it fell, until the corpse goes)
        w.entities.insert(0x7b017c2a, ([277.1, 8.1, 278.3], 0));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, 0x7b017c2a), data: hex("000000006300000000000000000000000000000000000001f40000") }.frame(1));
        assert_eq!(w.dead.get(&0x7b017c2a).map(|d| d.0), Some(500));
        w.add(&Message { kind: 0x36510078, identity: (0xc350, 0x7b017c2a), data: hex("00") }.frame(1));
        assert!(w.dead.contains_key(&0x7b017c2a) && w.entities.contains_key(&0x7b017c2a));
        w.add(&Message { kind: 0x4f474e05, identity: (0xc76a, 0x103b002), data: hex("00000000080000000b0000000000000000438a90bd410228f6438b24b000000000") }.frame(1));
        assert_eq!(w.corpses.get(&0x7b017c2a), Some(&0x103b002));
        assert_eq!(w.used(0xc350, 0x7b017c2a), (0xc76a, 0x103b002));
        w.add(&Message { kind: 0x36510078, identity: (0xc76a, 0x103b002), data: hex("00") }.frame(1));
        assert!(!w.dead.contains_key(&0x7b017c2a) && !w.entities.contains_key(&0x7b017c2a));
        // (a living one despawned leaves)
        w.entities.insert(9, ([0.0; 3], 0));
        w.add(&Message { kind: 0x36510078, identity: (0xc350, 9), data: hex("00") }.frame(1));
        assert!(!w.entities.contains_key(&9));
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
        // (another's health dropping in a Stat: 0x7aedad06 renamed to the rat, 2381 then 2300; the hit above was long ago)
        w.recent_hits.clear();
        w.add(&Message { kind: 0x2b333d6e, identity: (0xc350, 0x7b017c26), data: hex("00000000010000001b0000094d") }.frame(1));
        w.add(&Message { kind: 0x2b333d6e, identity: (0xc350, 0x7b017c26), data: hex("00000000010000001b000008fc") }.frame(1));
        assert_eq!(lines(&mut w), ["Rollerrat was attacked with nanobots for 81 points of unknown damage."]);
        // (a hit just told and the Stat with the health it left: one line, not a second one for nanobots. Seen live in
        // a duel, where the server sends both for every weapon hit; the same hit, then 2300 down to 387)
        w.add(&Message { kind: 0x46002f16, identity: (0xc350, 0x4bc80), data: hex("0000000779ffffffff000000080000c3507b017c26000000040000000300000000") }.frame(1));
        w.add(&Message { kind: 0x2b333d6e, identity: (0xc350, 0x7b017c26), data: hex("00000000010000001b00000183") }.frame(1));
        assert_eq!(lines(&mut w), ["You hit Rollerrat for 1913 points of projectile damage."]);
        // (and a loss beyond the hit is still told: the same hit, the health down 100 more than it)
        w.add(&Message { kind: 0x2b333d6e, identity: (0xc350, 0x7b017c26), data: hex("00000000010000001b00000bb8") }.frame(1));
        w.add(&Message { kind: 0x46002f16, identity: (0xc350, 0x4bc80), data: hex("0000000779ffffffff000000080000c3507b017c26000000040000000300000000") }.frame(1));
        w.add(&Message { kind: 0x2b333d6e, identity: (0xc350, 0x7b017c26), data: hex("00000000010000001b000003db") }.frame(1));
        assert_eq!(lines(&mut w), ["You hit Rollerrat for 1913 points of projectile damage.", "Rollerrat was attacked with nanobots for 100 points of unknown damage."]);
        // (a special attack's hit, `SpecialAttackInfo`: the client's burst at the rat for 175, then the rat's at the
        // client; the skill is named where the damage type is, and the health that follows tells nothing more)
        w.add(&Message { kind: 0x2b333d6e, identity: (0xc350, 0x7b017c26), data: hex("00000000010000001b00000bb8") }.frame(1));
        w.add(&Message { kind: 0x754f1115, identity: (0xc350, 0x4bc80), data: hex("0000000006000000af000000050000c3507b017c260000009400000000") }.frame(1));
        w.add(&Message { kind: 0x2b333d6e, identity: (0xc350, 0x7b017c26), data: hex("00000000010000001b00000b09") }.frame(1));
        assert_eq!(lines(&mut w), [format!("You hit Rollerrat for 175 points of {} damage.", crate::gui::criteria::stat_name(148))]);
        w.add(&Message { kind: 0x754f1115, identity: (0xc350, 0x7b017c26), data: hex("0000000006000000af000000050000c3500004bc800000009400000000") }.frame(1));
        assert_eq!(lines(&mut w), [format!("Rollerrat hit you for 175 points of {} damage.", crate::gui::criteria::stat_name(148))]);
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
        // the server's word: the live bytes of a closed door, and the same with the open flag set
        let closed = [0x00, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x03, 0xf1];
        let mut open = closed;
        open[6] = 1;
        assert_eq!(door_open(&closed), Some(false));
        assert_eq!(door_open(&open), Some(true));
        assert_eq!(door_open(&[0; 16]), None);
        // a closed word leaves our own use's few seconds; an open word holds until a closed one
        world.door_status(0x10c1a5a7, false);
        assert!(world.doors_open.contains_key(&2));
        world.door_status(0x10c1a5a9, true);
        assert!(world.doors_open.get(&4).is_some_and(|until| *until > Instant::now() + Duration::from_secs(3600)));
        world.door_status(0x10c1a5a9, false);
        assert!(!world.doors_open.contains_key(&4));
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

    /// Sneaking (FUN_1006c370 in mode 6): the sneak action moving and standing, a sidestep still the walk-left/right, and the
    /// mode's speed walking's 1.5 (FUN_1006f9eb). `MAO_GAME`: every body has the `sneakcool` animation.
    #[test]
    fn sneaking_plays_the_sneak_action_at_walking_speed() {
        use mao_formats::appearance::{Action, ATROX, FEMALE, MALE};
        assert_eq!(movement_action(Mode::Sneak, true, 0, false), Some(Action::Sneak));
        assert_eq!(movement_action(Mode::Sneak, true, 0, true), Some(Action::Sneak));
        assert_eq!(movement_action(Mode::Sneak, false, 0, false), Some(Action::SneakStand));
        assert_eq!(movement_action(Mode::Sneak, true, -1, false), Some(Action::StrafeLeft));
        assert_eq!(movement_action(Mode::Run, true, 0, true), Some(Action::RunBack));
        assert_eq!(movement_action(Mode::Walk, false, 0, false), None);
        assert_eq!(movement_action(Mode::Swim, false, 1, false), Some(Action::IdleSwim));
        assert_eq!((Action::Sneak.reference_speed(), Action::SneakStand.reference_speed()), (Some(WALK_SPEED), None));
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = std::sync::Arc::new(ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        let mut chars = Characters::new(db).unwrap();
        for (breed, sex) in [(1, MALE), (1, FEMALE), (ATROX, MALE)] {
            let name = mao_formats::appearance::animation_name(breed, sex, Action::Sneak);
            assert!(chars.loop_points(breed, sex, Action::Sneak).is_some(), "{name}");
        }
    }

    #[test]
    fn the_character_moves_as_a_steered_vehicle() {
        // from rest to full speed (6) in half a second, by 1/60 s steps
        let mut v = [0.0; 2];
        let mut pushed = 0;
        for _ in 0..15 {
            v = vehicle_step(v, 0.0, 1, pushed, 6.0, 1.0 / 60.0);
            pushed = 1;
        }
        assert!((v[1] - 3.0).abs() < 1e-3, "{v:?}");
        for _ in 0..30 {
            v = vehicle_step(v, 0.0, 1, 1, 6.0, 1.0 / 60.0);
        }
        assert!((v[1] - 6.0).abs() < 1e-3);
        // turning a quarter while running: the velocity swings round, not at once
        let turned = vehicle_step(v, std::f32::consts::FRAC_PI_2, 1, 1, 6.0, 1.0 / 60.0);
        assert!(turned[1] > 5.0 && turned[0] > 0.0 && turned[0] < 1.0, "{turned:?}");
        // a stop halts at once; a turn round starts from rest the other way
        assert_eq!(vehicle_step(v, 0.0, 0, 1, 6.0, 1.0 / 60.0), [0.0, 0.0]);
        assert!((vehicle_step(v, 0.0, -1, 1, 3.5, 1.0 / 60.0)[1] + 3.5 * 2.0 / 60.0).abs() < 1e-4);
        // the turn rates: 3.5 standing and 1.5 moving, or TurnSpeed / 11000 (halved while moving)
        let mut stats = BTreeMap::new();
        assert_eq!((turn_rate(&stats, false), turn_rate(&stats, true)), (3.5, 1.5));
        stats.insert(267, 22000);
        assert_eq!((turn_rate(&stats, false), turn_rate(&stats, true)), (2.0, 1.0));
    }

    /// A nano with no attack time (an item's, an aura's) shows no casting stance; one with a time does.
    #[test]
    fn only_a_nano_that_takes_time_shows_the_casting() {
        assert!(takes_time_to_cast(Some(100)));
        assert!(!takes_time_to_cast(Some(0)));
        assert!(!takes_time_to_cast(None));
        // (a cast's time: 3 s with no skill; 600 initiative takes 3 s off; above 1200 a sixth counts; the aggression
        // setting a second either way; never under the nano's own least)
        assert_eq!(cast_seconds(300, None, 0, 0), 3.0);
        assert_eq!(cast_seconds(300, None, 600, 0), 0.0);
        assert_eq!(cast_seconds(900, None, 1800, 0), 2.0);
        assert_eq!(cast_seconds(300, None, 0, 100), 2.0);
        assert_eq!(cast_seconds(300, None, 0, -100), 4.0);
        assert_eq!(cast_seconds(300, Some(100), 2000, 100), 1.0);
        assert_eq!(cast_seconds(300, Some(1_234_567_890), 2000, 0), 0.0);
    }

    #[test]
    fn sitting_is_held_until_getting_up() {
        let mut w = World { own: 1, ..World::default() };
        w.add(&n3::position_update(7, n3::movement::SIT, 0.0, [1.0, 2.0, 3.0], 0));
        assert_eq!(w.held_action(7), Some(0xd5));
        // (turning on the spot keeps it seated)
        for kind in [n3::movement::TURN_LEFT_START, n3::movement::TURN_RIGHT_START, n3::movement::TURN_STOP, n3::movement::HEADING] {
            w.add(&n3::position_update(7, kind, 1.0, [1.0, 2.0, 3.0], 0));
            assert_eq!(w.held_action(7), Some(0xd5));
        }
        // (the keys of a seated character of one's own: nothing but the turns gets through)
        assert_eq!(seated_keys(true, true, 1, true), (false, false, 0, false));
        // (the periodic report keeps it seated; getting up or walking does not)
        w.add(&n3::position_update(7, n3::movement::FULL, 0.0, [1.0, 2.0, 3.0], 0));
        assert_eq!(w.held_action(7), Some(0xd5));
        w.add(&n3::position_update(7, n3::movement::LEAVE_SIT, 0.0, [1.0, 2.0, 3.0], 0));
        assert_eq!(w.held_action(7), None);
    }

    #[test]
    fn a_corpse_names_itself() {
        let hex = "00000000080000000b000000000000000042449be54167ab1442b70037000000003ec6fc78000000003f6be00f0020d97c0000000000000000006f000046f200000000001818050000001700000000000002bd00000000000002be00000000000002bf000000000000019c00000001000001680000005a000000df000000000000003b00000001000000040000000600000059000000010000019f0000c350000001a07b15cf090000002a000058f50000003d000000010000000800004650000000220000003c0000001d52656d61696e73206f6620566976697061726f7573204c697a617264000000000200000032000003f100000003000007e20000cf273a4acb800000000400000000000000010000000000000000000000000000000000000000000001f600000001000000040000590a000000000000c3507b15cf09000017a600000000000000000000000000000001000000000000000000000002000000000000000000000003000000000000000000000004000000000000000000000000";
        let data: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        assert_eq!(corpse_name(&data).as_deref(), Some("Remains of Viviparous Lizard"));
        assert_eq!(corpse_name(&[0, 0, 0, 2, b'x', 0]), None);
    }

    #[test]
    fn creatures_walk_their_paths() {
        // a path's speed: 3 m along a 5 m path in 2 s is 1.5 m/s; a path walked to its end tells nothing
        let then = Instant::now();
        let now = then + Duration::from_secs(2);
        assert_eq!(path_speed((then, [0.0; 3], [0.0, 0.0, 5.0]), [0.0, 0.0, 3.0], now), Some(1.5));
        assert_eq!(path_speed((then, [0.0; 3], [0.0, 0.0, 3.0]), [0.0, 0.0, 3.0], now), None);
        // walked at 1.5 m/s toward the path's end, then stopped there
        let mut w = World { own: 1, ..World::default() };
        w.entities.insert(7, ([0.0; 3], 0));
        w.paths.insert(7, Route::Waypoints(vec![[0.0, 0.0, 3.0]]));
        w.walk_paths(1.0, [50.0, 0.0, 0.0]);
        assert!((w.entities[&7].0[2] - 1.5).abs() < 1e-4);
        for _ in 0..3 {
            w.walk_paths(1.0, [50.0, 0.0, 0.0]);
        }
        assert!((w.entities[&7].0[2] - 3.0).abs() < 0.11);
        // after oneself: stops 1.5 m short
        w.paths.insert(7, Route::Chase(1));
        for _ in 0..40 {
            w.walk_paths(1.0, [0.0, 0.0, 20.0]);
        }
        assert!((w.entities[&7].0[2] - 18.5).abs() < 1e-3);
    }

    #[test]
    fn the_shuttle_pads_teleport_is_a_move_within_the_playfield() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        // (the 2026-11-08 capture's last message: the Unicorn shuttle pad in ICC HQ sends a `c7a0` teleport with every id zero,
        // into an instance not made yet; the original client crashed on it. Here it is a move to the pad's place, nothing more.)
        let shuttle = hex("0045454000425000004458800000000000bf31dbb0800000003f382052610000c7a00000000000000000000000000000000000000000000000000000000000000000000000000000000c454540004250000044588000");
        let mut w = World { own: 0x39c16ee4, ..World::default() };
        w.entities.insert(0x39c16ee4, ([3182.0, 36.0, 880.0], 0));
        w.add(&Message { kind: 0x43197d22, identity: (0xc350, 0x39c16ee4), data: shuttle }.frame(1));
        let (p, _) = w.own_teleport.take().expect("a move within the playfield");
        assert!((p[0] - 3156.0).abs() < 0.01 && (p[1] - 52.0).abs() < 0.01 && (p[2] - 866.0).abs() < 0.01, "{p:?}");
        assert!(w.playfield_stopped.is_none() && w.entities.contains_key(&0x39c16ee4));
    }

    #[test]
    fn the_server_teleports_dynels() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        // (the 16:55 capture's move in a generated mission, c7a0: own character 0x7b1a0021 put at (121.4, 148.4, 87.8) facing 0)
        let mission = hex("0042f2dd0843146a3d42af80c40000000000000000000000003f800000610000c7a000000000000000000000000000000000000000000000000000000000000186a7000000000000001042f2dd084314752042af80c400000000");
        let mut w = World { own: 0x7b1a0021, ..World::default() };
        w.entities.insert(0x7b1a0021, ([10.0, 0.0, 10.0], 0));
        w.entities.insert(0x7b000001, ([20.0, 0.0, 20.0], 0));
        w.add(&Message { kind: 0x43197d22, identity: (0xc350, 0x7b1a0021), data: mission.clone() }.frame(1));
        let (p, q) = w.own_teleport.take().unwrap();
        assert!((p[0] - 121.43).abs() < 0.01 && (p[2] - 87.75).abs() < 0.01);
        assert_eq!(q, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(w.entities[&0x7b1a0021].0, p);
        assert!(w.playfield_stopped.is_none() && w.entities.contains_key(&0x7b000001));
        // (another character moved the same way is put there at once, its path dropped; one the client does not have is not made)
        w.paths.insert(0x7b000001, Route::Waypoints(vec![[30.0, 0.0, 30.0]]));
        w.add(&Message { kind: 0x43197d22, identity: (0xc350, 0x7b000001), data: mission.clone() }.frame(1));
        assert_eq!(w.entities[&0x7b000001].0, p);
        assert!(!w.paths.contains_key(&0x7b000001) && w.own_teleport.is_none());
        w.add(&Message { kind: 0x43197d22, identity: (0xc350, 0x7b000002), data: mission }.frame(1));
        assert!(!w.entities.contains_key(&0x7b000002));
        // (a zoning to playfield 0x1bbd: another character's is left to its Despawn; one's own stops the playfield, its dynels gone)
        let zoning = hex("004560b583424efae1444484de00000000bf0d22f0800000003f559487610000c79c00001bbd000000000000000000009c5000001bbd0000000000000000000186a100011bbd000000083f00000000000000");
        w.add(&Message { kind: 0x43197d22, identity: (0xc350, 0x7b000001), data: zoning.clone() }.frame(1));
        assert_eq!(w.entities[&0x7b000001].0, p);
        assert!(w.playfield_stopped.is_none());
        w.add(&Message { kind: 0x43197d22, identity: (0xc350, 0x7b1a0021), data: zoning }.frame(1));
        assert!(w.playfield_stopped.is_some() && w.own_teleport.is_none());
        assert_eq!(w.entities.keys().copied().collect::<Vec<_>>(), vec![0x7b1a0021]);
    }

    #[test]
    fn a_move_start_cancels_the_cast_and_the_use() {
        let hex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let mut w = World { own: 0x4bc82, ..World::default() };
        w.nano_casts.gather(0x4bc82, 0x1234, &|_, _| None);
        w.socials.insert(0x4bc82, ("spell-dir", Instant::now()));
        w.socials.insert(9, ("spell-self", Instant::now()));
        // (the 15:05 capture's 0x89 then 0x92, as the server sends them on every move start)
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, 0x4bc82), data: hex("000000008900000000000000000000000000000000000000000000") }.frame(1));
        assert!(w.nano_casts.casts.is_empty() && !w.socials.contains_key(&0x4bc82) && w.socials.contains_key(&9));
        w.socials.insert(0x4bc82, ("wave", Instant::now()));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, 0x4bc82), data: hex("000000008900000000000000000000000000000000000000000000") }.frame(1));
        assert!(w.socials.contains_key(&0x4bc82));
        // (the 08:42 capture's 0x92 naming the item used out of the open container of handle 0x70: that container refreshed;
        // any 0x92 ends the item use timer)
        w.loot = Some(Loot { identity: (0xc76a, 5), handle: 0x70, entries: Vec::new() });
        w.timers.add(500, 0, crate::timers::ITEM, 0x40);
        assert!(w.timers.has(crate::timers::ITEM));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, 0x4bc82), data: hex("000000009200000000000000000000000000000000000000000000") }.frame(1));
        assert!(!w.loot_changed && !w.timers.has(crate::timers::ITEM));
        w.add(&Message { kind: 0x5e477770, identity: (0xc350, 0x4bc82), data: hex("0000000092000000000000006b0070000100000000000000000000") }.frame(1));
        assert!(w.loot_changed && w.loot.is_some());
    }
}

#[cfg(test)]
mod prop_anim_tests {
    use super::prop_anim_time;

    /// The hit that kills the client's own character names what it died of: the game's line is told once, the health
    /// is nothing, and the death's own message makes the character dead (no moving, attacking or using until the
    /// server gives it health or a place again).
    #[test]
    fn the_own_death_is_told_and_leaves_no_health() {
        let hex = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect() };
        let (own, robot) = (0x7b1e5338u32, 0x7b431329u32);
        let mut w = super::World { own, ..Default::default() };
        w.stats.insert(27, 7);
        // (an `AttackInfo` from the robot on the own character: 7 points, the reason 4, a weapon)
        let hit = format!("0000000007ffffffff000000000000c350{own:08x}000000040000000300000000");
        w.add(&super::Message { kind: 0x46002f16, identity: (0xc350, robot), data: hex(&hit) }.frame(own));
        assert_eq!(w.stats.get(&27), Some(&0));
        let told = w.game_lines.len();
        assert!(told >= 1);
        // (the same reason again within the death: no second line)
        w.add(&super::Message { kind: 0x46002f16, identity: (0xc350, robot), data: hex(&hit) }.frame(own));
        assert_eq!(w.game_lines.iter().filter(|l| l.text == crate::feedback::death(4).unwrap()).count(), 1);
        // (the death: `CharacterAction` 99 from the own character with its animation)
        w.add(&super::Message { kind: 0x5e477770, identity: (0xc350, own), data: hex("000000006300000000000000000000000000000000000001f40000") }.frame(own));
        assert!(w.own_dead && w.dead_health == 0);
        // (a hit on somebody else that kills tells the client nothing of a death)
        let mut w = super::World { own, ..Default::default() };
        let other = format!("0000000007ffffffff000000000000c350{robot:08x}000000040000000300000000");
        w.add(&super::Message { kind: 0x46002f16, identity: (0xc350, own), data: hex(&other) }.frame(own));
        assert!(w.game_lines.iter().all(|l| l.text != crate::feedback::death(4).unwrap()));
    }

    /// A built-in machine is used by the id the server gave its record: by the record's place among its kind in the
    /// table, not by the number in its table id (Arete Landing: the machines' numbers skip 5; the runs are those of a
    /// live entering, 2026-11-10, and the original's capture has the sixth machine announced as `first + 5` where the
    /// table has number 6).
    #[test]
    fn a_built_in_machine_goes_by_its_place_in_the_table() {
        let mut w = super::World { own: 1, ..Default::default() };
        w.instance_runs = vec![(0xc748, 0, 1, 0x10c16884), (0xc73d, 1, 1, 0x58b158e2), (0xc75b, 2, 8, 0x1324d82c), (0xc73d, 0xa, 0x10, 0x58b158e3), (0xc748, 0x1a, 1, 0x10c16885)];
        let row = |kind: u32, number: u32| (kind, 0xc000_1999 | number << 16, [0.0f32; 3], 0u32, String::new());
        w.table = vec![row(0xc73d, 0x10)];
        w.table.extend([0, 1, 2, 3, 4, 6, 7, 8].map(|n| row(0xc75b, n)));
        w.table.extend((0..0x10).map(|n| row(0xc73d, n)));
        assert_eq!(w.used(0xc75b, 0xc006_1999), (0xc75b, 0x1324d831));
        assert_eq!(w.used(0xc75b, 0xc008_1999), (0xc75b, 0x1324d833));
        assert_eq!(w.used(0xc75b, 0xc000_1999), (0xc75b, 0x1324d82c));
        // (the terminals: the first of the table has a run of its own, the others follow in the second)
        assert_eq!(w.used(0xc73d, 0xc010_1999), (0xc73d, 0x58b158e2));
        assert_eq!(w.used(0xc73d, 0xc000_1999), (0xc73d, 0x58b158e3));
        // (a thing that is not the table's keeps its id)
        assert_eq!(w.used(0xc73d, 0x58b15906), (0xc73d, 0x58b15906));
    }

    /// A new level's message (this client's test character reaching level 3, 2026-11-10: the first 28 bytes as the trace
    /// showed them, the rest laid out as the original's capture has it) gives the character its level, its experience
    /// and the span the experience bar shows.
    #[test]
    fn a_new_level_sets_the_experience_span() {
        let hex = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect() };
        let mut w = super::World { own: 7, ..Default::default() };
        w.stats.extend([(54, 2), (52, 4240), (57, 1450), (350, 4050)]);
        w.add(&super::Message { kind: 0x7f405a16, identity: (0xc350, 7), data: hex("00000000030000209b0000109000000fd200001bee000000000000000400000104") }.frame(7));
        let stat = |k: u32| w.stats.get(&k).copied();
        assert_eq!((stat(54), stat(52), stat(53), stat(57), stat(350)), (Some(3), Some(4240), Some(8347), Some(4050), Some(7150)));
        assert_eq!(crate::stats::experience(&w.stats), (Some(190), Some(3100)));
        assert_eq!(w.game_lines.len(), 2);
        // (another character's level is not the client's)
        let mut w = super::World { own: 7, ..Default::default() };
        w.add(&super::Message { kind: 0x7f405a16, identity: (0xc350, 8), data: hex("00000000030000209b0000109000000fd200001bee000000000000000400000104") }.frame(7));
        assert!(w.stats.get(&54).is_none() && w.game_lines.is_empty());
    }

    /// `MAO_GAME`: a thing the server tells of (a gas fire of Arete Landing, as the original's capture of 2026-10-30 has
    /// it) is drawn by its template's model and can be used; when the server takes it away it is gone.
    #[test]
    fn a_thing_told_of_goes_when_the_server_takes_it_away() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let _ = super::ITEM_DB.set(std::sync::Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap()));
        let hex = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect() };
        let mut w = super::World { own: 1, ..Default::default() };
        let told = "000000000b0000000000000000456318f3422ba0324450cde2000000003b521a7100000000bf7fffaa0020da15000f424f00000000006f0000237900000000800a222100000017000483cb000002bd00000001000002be000483cb000002bf000483cb0000019c00000001000001f500000000000001f40000000000000000";
        w.add(&super::Message { kind: 0x3b11256f, identity: (0xc73d, 0x58dd9b36), data: hex(told) }.frame(1));
        assert!(w.props.contains_key(&0x58dd9b36) && w.objects.contains_key(&0x58dd9b36));
        // (told to stand still: `AnimPlay` and `AnimPos` 0)
        assert_eq!(w.prop_anim.get(&0x58dd9b36).map(|a| (a.0, a.1)), Some((0, 0)));
        w.add(&super::Message { kind: 0x36510078, identity: (0xc73d, 0x58dd9b36), data: vec![1] }.frame(1));
        assert!(!w.props.contains_key(&0x58dd9b36) && !w.objects.contains_key(&0x58dd9b36) && !w.terminals.contains_key(&0x58dd9b36));
    }

    /// A thing's model stands still unless its `AnimPlay` says otherwise (Gamecode FUN_10088970): a crate's doors stay
    /// shut, a looping one follows the clock, an opening one runs once to its end and stays there.
    #[test]
    fn a_things_model_plays_as_its_stats_say() {
        // (no play: the first pose, or the end with `AnimPos` 2)
        assert_eq!(prop_anim_time(0, 0, 5.0, 7.3, 1.0), None);
        assert_eq!(prop_anim_time(1, 0, 5.0, 7.3, 1.0), None);
        assert_eq!(prop_anim_time(1, 2, 5.0, 7.3, 1.0), Some(1.0));
        // (round and round, forward and backward)
        assert_eq!(prop_anim_time(2, 0, 5.0, 7.25, 1.0), Some(7.25));
        assert_eq!(prop_anim_time(4, 0, 5.0, 7.25, 1.0), Some(0.75));
        // (once: opening from the start, closing from the end)
        assert_eq!(prop_anim_time(3, 1, 0.25, 7.3, 1.0), Some(0.25));
        assert_eq!(prop_anim_time(3, 1, 5.0, 7.3, 1.0), Some(1.0));
        assert_eq!(prop_anim_time(5, 2, 0.25, 7.3, 1.0), Some(0.75));
        assert_eq!(prop_anim_time(5, 2, 5.0, 7.3, 1.0), Some(0.0));
        // (a model without an animation)
        assert_eq!(prop_anim_time(2, 0, 5.0, 7.3, 0.0), None);
    }
}
