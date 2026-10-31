//! The viewer (a mesh or a playfield): headless to an image, or in a window.
//! In a plain viewer W/A/S/D move the view point, Q/E go down/up, Esc quits;
//! in the live window the keys come from [`crate::settings`] (`mao.ini`), and
//! F11 / Alt+Enter change the window mode.

use std::sync::Arc;

use mao_formats::model::Model;
use mao_render::wgpu;
use mao_render::{Camera, Gpu, Renderer, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

use crate::settings::{Action, Binding, Settings, WindowMode};
use crate::ui::{Hud, Label, Ui};
use crate::Textures;

/// What the player is doing with the keyboard: forward held, and turning
/// (-1 left, 1 right).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub forward: bool,
    pub back: bool,
    pub jump: bool,
    /// Walk through props (toggled with C).
    pub noclip: bool,
    pub turn: i8,
    /// Sideways: -1 left, 1 right.
    pub strafe: i8,
    /// Counters that step each time the key is pressed (the live loop acts on a change):
    /// next/previous hostile, next/previous friendly target, attack.
    pub commands: [u8; 7],
    /// `/camp` (steps once a use).
    pub camp: u8,
    /// The walk/run toggle (steps once a press).
    pub walk_toggle: u8,
    /// The heading the character should face (the camera's, while the right mouse button steers).
    pub face: Option<f32>,
    /// An item used on something in the world: a counter that steps with each, the item's identity and the target's.
    pub use_on: (u8, (u32, u32), (u32, u32)),
    /// Flying: the vehicle's elevation (radians, nose up positive), set by the right button's vertical mouse movement.
    pub pitch: f32,
    /// A hotbar item to use: a counter that steps with each, and the inventory slot.
    pub fire: (u8, u32),
    /// A special action of the Action window to do (a counter that steps with each, and its id), and a social
    /// action or dance (the same).
    pub special: (u8, u32),
    pub social: (u8, u32),
    /// The control center's AGG/DEF slider let go: a counter that steps with each, and the value (stat 51).
    pub aggdef: (u8, i32),
    /// A place to walk to (a counter that steps with each, and the world's x, z).
    pub goto: (u8, [f32; 2]),
    /// A mission window command: a counter that steps with each, and the command.
    pub mission: (u8, crate::mission::Command),
    /// Something clicked in the world: a counter that steps with each click, its kind and id, and whether to use it (a
    /// right or double click).
    pub pick: (u8, u32, u32, bool),
    /// "Deposit to Market"'s Send: a counter that steps with each, and what to deposit.
    pub market: (u8, crate::marketsend::Deposit),
    /// A nano program to cast: a counter that steps with each, and the program's id.
    pub cast: (u8, u32),
    /// An item to move: a counter that steps with each, the place it is in and the place it goes to.
    pub mv: (u8, u32, u32),
    /// An item to destroy: a counter that steps with each and the item's place.
    pub delete: (u8, u32),
    /// A backpack item used on another (a tradeskill combine): a counter that steps with each, the item and the one it went on.
    pub combine: (u8, u32, u32),
    /// Follow the target (a counter that steps with each `/follow`).
    pub follow: u8,
    /// Say what is known of the target (a counter that steps with each `/inspect`).
    pub inspect: u8,
    /// Target the character itself (a counter that steps with each `/selectself`).
    pub select_self: u8,
    /// A running nano to cancel: a counter that steps with each and the nano's id.
    pub cancel: (u8, u32),
    /// A perk rank to train or untrain: a counter that steps with each, the rank and whether to train it.
    pub train_perk: (u8, u32, bool),
    /// A command to the pets (`/pet`): a counter that steps with each, and the `PetCommand` number.
    pub pet: (u8, u32),
    /// How far characters are drawn (0: all), from the settings.
    pub char_distance: f32,
    /// Take everything from a corpse when it opens (a setting).
    pub auto_loot: bool,
    /// An item to take from the open container: a counter that steps with each, the container's handle and the item's place
    /// (`u32::MAX` for all of them).
    pub take: (u8, u32, u32),
    /// An inventory item to put in the open container: a counter that steps with each, the container's identity and the item's place.
    pub put: (u8, u32, u32, u32),
}

/// The sheets of the lightning's bolt and flashes (`newsun_frame01.png`, `dustcloud.png`).
const BOLT_SHEET: u16 = 0xb100;
const CLOUD_SHEET: u16 = 0xb101;

/// The overlay to draw and where the player's character is (the view
/// follows it).
#[derive(Default)]
pub struct Update {
    /// An update that only carries messages (nothing to draw): the picture stays as it was.
    pub messages_only: bool,
    /// A new playfield to replace the one drawn (zoning).
    pub scene: Option<Model>,
    pub model: Model,
    /// The player's own character, drawn separately so it can move smoothly between updates, and its speed
    /// (units a second, server coordinates) for the prediction.
    pub own_model: Model,
    pub own_velocity: [f32; 3],
    /// The characters' meshes, for their blob shadows (`blobshadow.rs`).
    pub casters: Vec<crate::blobshadow::Caster>,
    /// New cells of the map (fog of war), and the people about (for its dots).
    pub map: Option<crate::automap::MapPatch>,
    /// The character's name and stats, when they change.
    pub stats: Option<(String, std::collections::BTreeMap<u32, i64>, std::collections::BTreeMap<u32, crate::derived::Breakdown>)>,
    pub map_dots: Vec<([f32; 2], crate::automap::DotKind)>,
    /// The playfield's Notum Wars tower sites, when the list changes (zoning in, a tower destroyed or planted).
    pub towers: Option<Vec<mao_net::landcontrol::Tower>>,
    /// The names to float over the characters and creatures near (Z-up position over the head).
    pub nameplates: Vec<crate::nametag::Tag>,
    /// Numbers to float over the fight: Z-up position, text and kind (see `ui::Floater`).
    pub floaters: Vec<([f32; 3], String, u8)>,
    /// Where a nano program landed (Z-up) and the glow's colour.
    pub glows: Vec<([f32; 3], [u8; 3])>,
    /// Who is under running nano programs: the character's id (0 for the player's own), whether it is the character itself, the feet
    /// (Z-up), its heading, the body (breed, sex, fatness) when known, and each program with its running effect. The whole list when it is given
    /// (an update without one leaves the last in place).
    pub auras: Option<Vec<(u32, bool, [f32; 3], f32, Option<(i32, i32, i32)>, Vec<(u32, mao_formats::gfx::Visual)>)>>,
    /// Cast effects to start: the caster, its cast template and the cast's length.
    pub bursts: Option<Vec<crate::effects::CastShow>>,
    /// Spells in flight and landing: the tracer's and the hit effect's spirals.
    pub shots: Option<Vec<crate::effects::Shot>>,
    /// Flare effects made alone at a place (Z up), as a sprung mine's blast (template 2100).
    pub flares: Vec<([f32; 3], mao_formats::gfx::Flare)>,
    /// The lightning's strikes to light (server coordinates, Y up) and their seeds (`lightning.rs`).
    pub strikes: Vec<([f32; 3], u32)>,
    /// The eye's jolts from strikes near (the quake's kick, `Quake::jolt`).
    pub jolts: Vec<f32>,
    /// The characters' landings (server coordinates, Y up): the environment's effect 0x2c at each (`Sky::landing`).
    pub landings: Vec<[f32; 3]>,
    /// The effect scripts' visuals and lights this frame (`fxs::visuals`): the sky.
    pub sky_frame: Option<crate::fxs::visuals::Frame>,
    /// What the server said of the specials' recharge (skill stat, seconds; 0 = ready).
    pub special_cooldowns: Vec<(u32, f32)>,
    /// The ground (and how far the server's heights sit above it), to keep the camera above it.
    pub ground: Option<(std::sync::Arc<mao_formats::terrain::Terrain>, f32)>,
    /// The zones' surfaces (server coordinates), for the camera's lines of sight.
    pub zones: Option<std::sync::Arc<mao_formats::zones::Zones>>,
    pub own: Option<[f32; 3]>,
    /// The area's environment where the character stands (light and fog), and the day's intensity.
    pub environment: Option<crate::environment::Environment>,
    pub daylight: Option<f32>,
    /// The playfield's name, when it changes.
    pub location: Option<String>,
    /// The labels over the world (Z-up positions), when they change.
    pub labels: Option<Vec<([f32; 3], String)>>,
    /// What can be clicked in the world: ids and Z-up positions.
    pub pickables: Option<Vec<(u32, u32, [f32; 3], f32, f32, bool)>>,
    /// Pages for the info view (what T says of the target).
    pub info_urls: Vec<String>,
    /// The shop trade's changes (the server's echoes).
    pub shop_changes: Vec<crate::shop::Change>,
    /// What the server said in NPC chats (`npcchat.rs`).
    pub npc_chat: Vec<crate::npcchat::Event>,
    /// The mission journal's whole list, when the server sent it, and the missions it dropped.
    pub mission_log: Option<Vec<crate::missionlog::Mission>>,
    pub missions_removed: Vec<(u32, u32)>,
    /// A web page to open (the market terminal used).
    pub browser: Option<crate::browser::Request>,
    /// Windows to open (a mail terminal or mission booth used).
    pub open_windows: Vec<&'static str>,
    /// Messages for the log.
    pub messages: Vec<String>,
    /// The current target (name and distance), for the target frame.
    pub target: Option<String>,
    /// The same target for the original's target frame: name, health, distance and what it is.
    pub target_bars: Option<Vec<crate::ui::TargetBar>>,
    /// The character's items, when they arrive.
    pub gear: Option<Vec<crate::gear::GearItem>>,
    /// Whether one's own character flies (a flying vehicle program runs), when it changes.
    pub flying: Option<bool>,
    /// An inspected character's name and worn items.
    pub inspected: Option<(String, Vec<crate::gear::GearItem>)>,
    /// New lines for the chat window.
    pub chat_lines: Vec<crate::chatwindow::Line>,
    /// The friends list, when it changes.
    pub friends: Option<Vec<crate::friends::Friend>>,
    /// The character's nano programs, when they arrive.
    pub programs: Option<Vec<crate::nanoprograms::Program>>,
    /// The character's perk actions, when they or their locks change.
    pub perk_actions: Option<Vec<crate::actions::PerkAction>>,
    /// The perk records and the owned ranks, when the ranks change.
    pub perk_list: Option<crate::perks::PerkState>,
    /// The container (corpse, bag) the character opened: `Some(None)` when it closed.
    pub loot: Option<Option<crate::loot::LootView>>,
    /// The running nano programs, when the list changes (the NCU window).
    pub nanos: Option<Vec<crate::ncu::NanoInfo>>,
    /// The way the character faces (radians about the vertical, 0 along +Z of the server's world).
    pub heading: Option<f32>,
    /// The travel list the server sent for this playfield, when it arrives.
    pub destinations: Option<mao_net::destinations::DestinationList>,
    /// The missions a booth offered, when it answers.
    pub missions: Option<Vec<crate::mission::Offer>>,
    /// A message for the mission window's status line.
    pub mission_status: Option<String>,
    /// The inbox's message count when the server reports it, and a note for the mail window.
    pub mail_count: Option<usize>,
    pub mail_status: Option<String>,
    /// The inbox's entries, an opened mail and a flag change, when the server sends them.
    pub mail_entries: Option<Vec<mao_net::mail::Entry>>,
    pub mail_opened: Option<crate::mailbox::Opened>,
    pub mail_flags: Option<(u32, u32)>,
    /// A shop the server opened (stock), and a note for the shop window.
    pub shop: Option<crate::shop::ShopState>,
    pub shop_status: Option<String>,
}

/// Something that changes while the window is open: models to draw over the
/// scene (the latest one is shown) and where to start looking.
pub struct Live {
    pub overlays: std::sync::mpsc::Receiver<Update>,
    /// Where the arrow keys' state goes.
    pub input: std::sync::mpsc::Sender<Input>,
    /// Mail commands (send, check the inbox).
    pub mail: std::sync::mpsc::Sender<crate::mailbox::MailCommand>,
    /// Shop commands (buy, close).
    pub shop: std::sync::mpsc::Sender<crate::shop::ShopCommand>,
    /// NPC chat commands (answer, describe, close).
    pub npc_chat: std::sync::mpsc::Sender<crate::npcchat::Command>,
    /// Mission journal commands (delete).
    pub mission_log: std::sync::mpsc::Sender<crate::missionlog::Command>,
    /// The view point (Z-up), if known.
    pub focus: Option<[f32; 3]>,
    /// The starting distance from it.
    pub distance: f32,
}

/// Renders the model once to `out`.
/// `MAO_CAMERA=x,y,z,distance,yaw,pitch` overrides the fitted camera.
fn camera_override(camera: &mut Camera) {
    let Ok(text) = std::env::var("MAO_CAMERA") else { return };
    let v: Vec<f32> = text.split(',').filter_map(|n| n.trim().parse().ok()).collect();
    if let [x, y, z, distance, yaw, pitch] = v[..] {
        *camera = Camera { target: mao_render::glam::Vec3::new(x, y, z), distance, yaw, pitch };
    }
}

pub fn headless(model: &Model, textures: &Textures, out: &str) -> Result<(), String> {
    headless_with(model, textures, out, None)
}

/// Puts the sprite sheets the particles are cut from on the card (once each; a sheet that cannot be read is not asked for again).
pub(crate) fn load_sprites(renderer: &mut Renderer, gpu: &Gpu, textures: &Textures, particles: &[mao_render::Particle], tried: &mut std::collections::HashSet<u16>) {
    for p in particles {
        if p.sprite == mao_render::NO_SPRITE || renderer.has_sprite(p.sprite) || !tried.insert(p.sprite) {
            continue;
        }
        if let Some(texture) = mao_formats::gfx::material(p.sprite as i32).and_then(|(name, ..)| textures.sheet(name)) {
            renderer.add_sprite(gpu, p.sprite, &texture);
        }
    }
}

/// [`headless`] with the interface painted over the image: the heads-up
/// display and, when the flag is set, the options window.
pub fn headless_with(model: &Model, textures: &Textures, out: &str, ui: Option<(&Hud, bool)>) -> Result<(), String> {
    headless_on(model, textures, out, ui, &[], false)
}

/// [`headless_with`] for a scene that is one posed body at the origin: `attractors` are its attractors' frames (the server's
/// frame), where `MAO_AURA`'s effect meshes at attach points 2000–2023 are held; `atrox`, whether the body is an Atrox's (the
/// hoverbike circles have Atrox meshes).
pub fn headless_on(model: &Model, textures: &Textures, out: &str, ui: Option<(&Hud, bool)>, attractors: &[(String, mao_formats::model::Matrix)], atrox: bool) -> Result<(), String> {
    let gpu = Gpu::headless()?;
    eprintln!("adapter: {:?}", gpu.adapter.get_info());
    let mut renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
    // `MAO_RENDER_SCALE=0.67` draws at that share of the image and upscales it with FSR 1.
    if let Some(s) = std::env::var("MAO_RENDER_SCALE").ok().and_then(|v| v.parse().ok()) {
        renderer.render_scale = s;
    }
    let decoding = std::time::Instant::now();
    textures.prefetch(model);
    profile!("[profile] textures decoded in {:?}", decoding.elapsed());
    let uploading = std::time::Instant::now();
    let mut scene = renderer.upload(&gpu, model, &mut |k, i| textures.get_for_upload(k, i));
    profile!("[profile] scene upload {:?} ({} parts)", uploading.elapsed(), model.parts.len());
    // `MAO_AURA=<effect id>` (with `MAO_GFX=<gfxtweak.bin>`, `MAO_AURA_AT=x,y,z` for the feet, `MAO_AURA_TIME=seconds`): the running
    // effect of a nano program round that point, for pictures of it.
    if let (Ok(id), Ok(file)) = (std::env::var("MAO_AURA"), std::env::var("MAO_GFX")) {
        let gfx = std::fs::read(&file).ok().and_then(|d| mao_formats::gfx::Gfx::parse(&d).ok());
        let at: Vec<f32> = std::env::var("MAO_AURA_AT").unwrap_or_default().split(',').filter_map(|n| n.trim().parse().ok()).collect();
        let feet = if let [x, y, z] = at[..] { [x, y, z] } else { [0.0, 0.0, 0.0] };
        let time: f32 = std::env::var("MAO_AURA_TIME").ok().and_then(|t| t.parse().ok()).unwrap_or(0.8);
        if let Some(visual) = gfx.and_then(|g| g.visual(id.parse().unwrap_or(0))) {
            scene.particles = crate::effects::pulse_particles(&visual, feet, 0.0, time, None);
            eprintln!("aura: {} sparks", scene.particles.len());
            // (and its effect meshes, their textures made sheets from 0xa000)
            let mut sheets: std::collections::HashMap<(u32, u32), u16> = std::collections::HashMap::new();
            let eye = {
                let mut c = Camera::fit(&scene);
                camera_override(&mut c);
                let e = c.eye();
                [e.x, e.y, e.z]
            };
            for (m, at, t) in crate::effects::visual_meshes(&visual, crate::effects::Locator::feet(feet, 0.0), time, &|attach| crate::effects::Locator::attractor(attach, attractors)) {
                let Some(model) = m.mesh_file(atrox).and_then(|name| textures.mesh_named(name)) else { continue };
                for part in &model.parts {
                    let Some(&key) = part.material.textures.first() else { continue };
                    if !sheets.contains_key(&key) {
                        let id = 0xa000 + sheets.len() as u16;
                        if let Some(texture) = textures.get(key.0, key.1) {
                            renderer.add_sprite(&gpu, id, &texture);
                        }
                        sheets.insert(key, id);
                    }
                }
                scene.shells.extend(crate::effects::effect_mesh_shells(&m, &model, &|k, i| sheets.get(&(k, i)).copied().unwrap_or(mao_render::NO_SPRITE), &at, Some(eye), t));
            }
            scene.shells.extend(crate::effects::visual_ground_shells(&visual, feet, time, &|_, _| None));
            // (its rocks, over a level ground at the feet)
            for (r, from, t) in crate::effects::visual_rocks(&visual, feet, time) {
                for rock in crate::effects::vulcan_rocks(&r, from, t, 7, &|_, _| None) {
                    let Some(model) = textures.mesh_named(rock.mesh) else { continue };
                    for part in &model.parts {
                        let Some(&key) = part.material.textures.first() else { continue };
                        if !sheets.contains_key(&key) {
                            let id = 0xa000 + sheets.len() as u16;
                            if let Some(texture) = textures.get(key.0, key.1) {
                                renderer.add_sprite(&gpu, id, &texture);
                            }
                            sheets.insert(key, id);
                        }
                    }
                    scene.shells.extend(crate::effects::rock_shells(&rock, &model, &|k, i| sheets.get(&(k, i)).copied().unwrap_or(mao_render::NO_SPRITE)));
                }
            }
            // (its trails, run at thirty frames a second up to the time, the body moving along +y at `MAO_RIDE_SPEED` units a
            // second, 0 by default)
            let speed: f32 = std::env::var("MAO_RIDE_SPEED").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let mut trails: Vec<crate::effects::TrailState> = Vec::new();
            let mut last = Vec::new();
            for frame in 0..=(time * 30.0) as i32 {
                let t = frame as f32 / 30.0;
                let shift = [0.0, speed * (t - time), 0.0];
                let moved = |l: crate::effects::Locator| crate::effects::Locator { origin: [l.origin[0] + shift[0], l.origin[1] + shift[1], l.origin[2] + shift[2]], ..l };
                let found = crate::effects::visual_trails(&visual, moved(crate::effects::Locator::feet(feet, 0.0)), t, &|attach| crate::effects::Locator::attractor(attach, attractors).map(moved));
                trails.resize_with(found.len(), Default::default);
                for (state, (trail, place, age)) in trails.iter_mut().zip(&found) {
                    state.step(trail, place, *age, t, [0.0, 1.0, 0.0]);
                }
                last = found;
            }
            for (state, (trail, _, _)) in trails.iter().zip(&last) {
                scene.shells.extend(state.shells(trail));
            }
            eprintln!("aura: {} shells", scene.shells.len());
            for shell in &scene.shells {
                let (lo, hi) = shell.vertices.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(lo, hi), v| ([0, 1, 2].map(|k| lo[k].min(v.position[k])), [0, 1, 2].map(|k| hi[k].max(v.position[k]))));
                eprintln!("  shell of {} vertices from {lo:.2?} to {hi:.2?}, alpha {:.2}", shell.vertices.len(), shell.vertices.first().map_or(0.0, |v| v.colour[3]));
            }
        }
    }
    // `MAO_CAST=<effect id>` (with `MAO_GFX`, `MAO_AURA_AT` for the origin and `MAO_AURA_TIME` for the age): the emitters of a cast, hit or
    // tracer template running (they emit for one second).
    if let (Ok(id), Ok(file)) = (std::env::var("MAO_CAST"), std::env::var("MAO_GFX")) {
        let gfx = std::fs::read(&file).ok().and_then(|d| mao_formats::gfx::Gfx::parse(&d).ok());
        let at: Vec<f32> = std::env::var("MAO_AURA_AT").unwrap_or_default().split(',').filter_map(|n| n.trim().parse().ok()).collect();
        let origin = if let [x, y, z] = at[..] { [x, y, z] } else { [0.0, 0.0, 1.0] };
        let age: f32 = std::env::var("MAO_AURA_TIME").ok().and_then(|t| t.parse().ok()).unwrap_or(0.6);
        if let Some(gfx) = gfx {
            for e in gfx.emitters(id.parse().unwrap_or(0)) {
                scene.particles.extend(crate::effects::emitter_particles(&e, origin, age, 1.0));
            }
            eprintln!("cast: {} particles", scene.particles.len());
        }
    }
    let mut tried = std::collections::HashSet::new();
    load_sprites(&mut renderer, &gpu, textures, &scene.particles, &mut tried);
    // (the shells' sheets too)
    let sheets: Vec<mao_render::Particle> = scene.shells.iter().map(|s| mao_render::Particle { sprite: s.sprite, ..mao_render::Particle::dot([0.0; 3], 0.0, [0.0; 4]) }).collect();
    load_sprites(&mut renderer, &gpu, textures, &sheets, &mut tried);
    // `MAO_DAYLIGHT=<0..1>` sets the day's intensity and `MAO_AREA_FOG=r,g,b,density` an area's fog, for pictures of them.
    if let Some(d) = std::env::var("MAO_DAYLIGHT").ok().and_then(|v| v.parse::<f32>().ok()) {
        scene.daylight = d.clamp(0.0, 1.0);
    }
    if let Some(v) = std::env::var("MAO_AREA_FOG").ok().map(|t| t.split(',').filter_map(|n| n.trim().parse::<f32>().ok()).collect::<Vec<_>>()) {
        if let [r, g, b, density] = v[..] {
            let light = scene.daylight.max(0.5);
            scene.area_fog = Some(([r * light, g * light, b * light], density));
            // (the fog reaches to the far plane: a picture without one gets the default view distance)
            if scene.fog.1 > 1.0e8 {
                scene.fog = fog_for(1800.0);
            }
        }
    }
    // `MAO_PLAYER=x,y,z` (world, Z up) puts a player in the scene: plants near it bend away.
    if let Some(v) = std::env::var("MAO_PLAYER").ok().map(|t| t.split(',').filter_map(|n| n.trim().parse::<f32>().ok()).collect::<Vec<_>>()) {
        if let [x, y, z] = v[..] {
            scene.player = Some(mao_render::glam::Vec3::new(x, y, z));
        }
    }
    let mut camera = Camera::fit(&scene);
    camera_override(&mut camera);
    // `MAO_SKY=<hour>`: the effect scripts' outdoor sky at that hour (the playfield's tweaks, or the outdoor default's, run
    // a few frames at it), its ground light the scene's sun.
    if let (Some(hour), Some(game)) = (std::env::var("MAO_SKY").ok().and_then(|v| v.parse::<f32>().ok()), crate::theme::game_dir()) {
        if let Ok(v) = std::env::var("MAO_SKY_VARIANT") {
            crate::fxs::ROT_REVERSED.store(v.contains("rev"), std::sync::atomic::Ordering::Relaxed);
        }
        let number = std::env::var("MAO_SKY_PLAYFIELD").ok().and_then(|v| v.parse().ok()).unwrap_or(999_999);
        crate::fxs::set_data_path(&game.join("cd_image"));
        crate::fxs::load_playfield(number, 0);
        crate::fxs::start_automatic();
        let seconds = hour * 3600.0;
        let (sun1, sun2) = crate::fxs::sun::rotations(seconds);
        let e = camera.eye();
        let game_frame = crate::fxs::Game { delta_time: 0.05, day_time: seconds / crate::gametime::PACE, camera_position: [e.x, e.z, e.y], sun1, sun2, cloud_transparency: 1.0, ..Default::default() };
        let mut visuals = crate::fxs::visuals::Visuals::default();
        for _ in 0..3 {
            crate::fxs::set_game(&game_frame);
            crate::fxs::frame_process();
            visuals.start(crate::fxs::take_started());
        }
        crate::fxs::set_game(&game_frame);
        let (s1, s2) = crate::fxs::sun::places(seconds);
        let sky = crate::fxs::clouds::Sky { camera: [e.x, e.z, e.y], sun_direction: crate::fxs::math::with_len(&crate::fxs::math::add(&s1, &s2), 1.0), ..Default::default() };
        let mut frame = visuals.frame(&sky);
        // (`MAO_SKY_ONLY=<sorting>,…`: only those)
        if let Ok(only) = std::env::var("MAO_SKY_ONLY") {
            let keep: Vec<u32> = only.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            frame.draws.retain(|d| keep.contains(&d.sorting));
        }
        for m in crate::fxs::take_messages() {
            eprintln!("fxs: {m}");
        }
        let target = camera.target;
        let forward = (target - e).normalize_or_zero();
        let (mut meshes, mut sheets, mut named) = Default::default();
        let mut caches = SkyCaches { meshes: &mut meshes, mesh_sheets: &mut sheets, named: &mut named };
        let (backdrops, others) = sky_draws(&frame, [e.x, e.z, e.y], [forward.x, forward.z, forward.y], 1000.0, textures, &mut renderer, &gpu, &mut caches);
        eprintln!("sky: {} draws, {} backdrops, {} others; lights {:?}", frame.draws.len(), backdrops.len(), others.len(), frame.lights.ground);
        scene.backdrops = backdrops;
        scene.shells.extend(others);
        scene.sun = frame.lights.ground.map(|l| mao_render::glam::Vec3::new(-l.direction[0], -l.direction[2], -l.direction[1]));
        scene.lighting = frame.lights.ground.map(|g| (frame.lights.ambient, g.colour));
        if let Some(fog) = frame.fog {
            scene.sky.0 = fog;
        }
    }
    let (w, h) = (960, 720);
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let radius = scene.radius;
    let pixels = renderer.render_to_image_with(&gpu, &scene, &camera, w, h, &mut |view| {
        if let Some((hud, options)) = ui {
            let aspect = w as f32 / h as f32;
            let project = |p: mao_render::glam::Vec3| camera.project(p, aspect, radius);
            let mut settings = Settings::load();
            crate::ui::paint_headless(&gpu, view, format, (w, h), &mut settings, hud, &project, options);
        }
    });
    eprintln!("drew {} of {} parts ({} in the shadow pass)", renderer.stats.drawn, renderer.stats.parts, renderer.stats.shadow_drawn);
    image::save_buffer(out, &pixels, w, h, image::ColorType::Rgba8).map_err(|e| e.to_string())
}

struct View {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: Gpu,
    renderer: Renderer,
    scene: Scene,
    camera: Camera,
    /// The interface (heads-up display, options): in a live window.
    ui: Option<Ui>,
    /// The web page over the window (the market's), while its window is open.
    #[cfg(windows)]
    page: Option<crate::browser::Page>,
}

struct App {
    model: Model,
    textures: Textures,
    live: Option<Live>,
    state: Option<View>,
    dragging: bool,
    last: (f64, f64),
    /// The left and right mouse buttons, in a live view (the camera is steered like a third-person game's).
    buttons: [bool; 2],
    /// The character's heading, and whether the camera has been put behind it yet.
    heading: f32,
    behind_set: bool,
    /// Whether a world update has come (the login's loading screen goes with the first one).
    world_in: bool,
    last_follow: std::time::Instant,
    /// The nano programs running on characters (see `Update::auras`) and the clock their sparks run on.
    auras: Vec<(u32, bool, [f32; 3], f32, Option<(i32, i32, i32)>, Vec<(u32, mao_formats::gfx::Visual)>)>,
    /// When each buff of each character pulses (the original's schedule).
    pulses: crate::pulses::Pulses,
    fx_clock: std::time::Instant,
    /// The camera's distance before it was pulled to the character's head (F8), when it is.
    /// The game's camera (the original's steered vehicle), and the frame's mouse look, wheel and reset gathered for it.
    steering: crate::camera::SteeringCamera,
    mouse_look: (f32, f32),
    wheel: f32,
    cam_reset: bool,
    /// The cast effects running: when each began, and what.
    bursts: Vec<(std::time::Instant, crate::effects::CastShow)>,
    shots: Vec<(std::time::Instant, crate::effects::Shot)>,
    flares: Vec<(std::time::Instant, [f32; 3], mao_formats::gfx::Flare)>,
    /// The lightning's flashes and bolts under way.
    sky: crate::lightning::Sky,
    /// The effect sprite sheets already asked for.
    tried_sprites: std::collections::HashSet<u16>,
    /// The effect meshes by file name (`GfxControlEffectMesh_t`), and the sheets their textures were made (from 0xa000).
    effect_meshes: std::collections::HashMap<&'static str, Option<Model>>,
    mesh_sheets: std::collections::HashMap<(u32, u32), u16>,
    /// The effect scripts' last frame, their meshes by name and their textures' sheets by name (from 0xb200).
    fxs_frame: Option<crate::fxs::visuals::Frame>,
    script_meshes: std::collections::HashMap<String, Option<Model>>,
    named_sheets: std::collections::HashMap<String, u16>,
    /// The running trails (`GfxControlTrail2_t`) by (character, program, trail), and whether each ran this frame.
    trails: std::collections::HashMap<(u32, u32, usize), (crate::effects::TrailState, bool)>,
    /// Each character's place a moment ago (and when), and its vehicle's motion for the toggles (`GfxControlToggle_t`).
    motions: std::collections::HashMap<u32, ([f32; 3], f32, crate::effects::Motion)>,
    /// How far the camera is shaken this frame (`GfxControlGroundShake_t`), and a count of frames for its random direction.
    shake: (f32, u32),
    /// The character's position and speed at the last update, and when it arrived (for the prediction).
    own_base: [f32; 3],
    own_velocity: [f32; 3],
    own_at: std::time::Instant,
    ground: Option<(std::sync::Arc<mao_formats::terrain::Terrain>, f32)>,
    zones: Option<std::sync::Arc<mao_formats::zones::Zones>>,
    /// The characters' meshes from the last update, and how far the own one has been moved on since.
    casters: Vec<crate::blobshadow::Caster>,
    follow_shift: [f32; 3],
    /// The area's environment where the character stands, and the day's intensity (from the last update).
    environment: crate::environment::Environment,
    /// The environment's quake (`ambience::Quake`) and the eye's offset it gave this frame.
    quake: crate::ambience::Quake,
    quake_offset: [f32; 3],
    /// The wind (the environment weather object's, see `wind`), stepped every frame from the weather's wind.
    wind: crate::wind::Wind,
    wind_at: std::time::Instant,
    daylight: f32,
    keys: Input,
    /// The keys as last sent to the network thread.
    last_sent: Input,
    settings: Settings,
    modifiers: winit::keyboard::ModifiersState,
    /// The movement actions whose keys are down.
    held: std::collections::HashSet<Action>,
    /// The camera as first set, and as the player saved it.
    home: Option<Camera>,
    preferred: Option<Camera>,
    hud: Hud,
    /// How far the mouse moved since the left button went down (a click is a press that barely moved), and the last click.
    /// A screenshot was asked for: the next frame saves it.
    screenshot: bool,
    /// Walking forward on its own (the auto-run key).
    autorun: bool,
    /// The server ended the connection (the character logged out): back to the character screen.
    logged_out: bool,
    moved: f32,
    last_click: Option<(u32, std::time::Instant)>,
}

/// The fog's start and end for a view distance: the haze begins a seventh of the way out (250 of the default 1800), or at 40.
pub fn fog_for(distance: f32) -> (f32, f32) {
    ((distance / 7.2).max(40.0), distance)
}

/// Shows the window the way `mode` says: a framed window, a borderless one
/// covering the screen, or exclusive fullscreen at the monitor's best video
/// mode (falling back to borderless where there is none).
fn apply_mode(window: &Window, mode: WindowMode) {
    match mode {
        WindowMode::Windowed => {
            window.set_fullscreen(None);
            window.set_decorations(true);
        }
        WindowMode::Borderless => window.set_fullscreen(Some(Fullscreen::Borderless(None))),
        WindowMode::Fullscreen => {
            let best = window.current_monitor().and_then(|monitor| {
                let size = monitor.size();
                monitor.video_modes().filter(|m| m.size() == size).max_by_key(|m| (m.refresh_rate_millihertz(), m.bit_depth()))
            });
            match best {
                Some(video_mode) => window.set_fullscreen(Some(Fullscreen::Exclusive(video_mode))),
                None => {
                    eprintln!("no exclusive video mode for this monitor: using a borderless window instead");
                    window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                }
            }
        }
    }
}

pub fn window(model: Model, textures: Textures, live: Option<Live>) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    let settings = Settings::load();
    crate::ui::set_debug_overlay(settings.debug_overlay);
    crate::font::set_enabled(settings.original_font);
    let mut app = App { model, textures, live, state: None, dragging: false, last: (0.0, 0.0), buttons: [false; 2], heading: 0.0, behind_set: false, world_in: false, last_follow: std::time::Instant::now(), auras: Vec::new(), pulses: Default::default(), fx_clock: std::time::Instant::now(), bursts: Vec::new(), shots: Vec::new(), flares: Vec::new(), sky: Default::default(), tried_sprites: Default::default(), effect_meshes: Default::default(), mesh_sheets: Default::default(), fxs_frame: None, script_meshes: Default::default(), named_sheets: Default::default(), trails: Default::default(), motions: Default::default(), shake: (0.0, 0), own_base: [0.0; 3], own_velocity: [0.0; 3], own_at: std::time::Instant::now(), ground: None, zones: None, casters: Vec::new(), follow_shift: [0.0; 3], environment: Default::default(), quake: Default::default(), quake_offset: [0.0; 3], wind: Default::default(), wind_at: std::time::Instant::now(), daylight: 1.0, steering: crate::camera::SteeringCamera::new(camera_mode(&settings.camera_mode)), mouse_look: (0.0, 0.0), wheel: 0.0, cam_reset: false, keys: Input::default(), last_sent: Input::default(), settings, modifiers: Default::default(), held: Default::default(), home: None, preferred: None, hud: Hud::default(), moved: 0.0, last_click: None, screenshot: false, autorun: false, logged_out: false };
    // (entering the game: the loading screen, `ServerLogin3DModule_t::InitialiseMessage`)
    if app.live.is_some() {
        crate::loading::begin(crate::loading::FADE_IN, crate::loading::FADE_OUT);
    }
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    // (the game closing, `FlowControlModule_t::ActivateGameClosing`: `ControlCenterModule_c::SaveUserConfig`, where the windows
    // are written down at once)
    crate::layout::flush();
    if app.logged_out {
        // (the launcher that started the game sees this code and shows the character screen again)
        crate::crash::end("game", None);
        std::process::exit(crate::launcher::LOGGED_OUT);
    }
    Ok(())
}

/// The heading (about the vertical, 0 along the server's +Z) a camera at `yaw` looks along, and the yaw
/// of a camera right behind a character facing `heading`. (The same function both ways.)
/// Does what a hotbar action says: use an item from the backpack, or attack.
fn fire_action(action: &crate::hotbar::Action, keys: &mut Input, hud: &mut Hud) {
    match action {
        crate::hotbar::Action::Attack => keys.commands[4] = keys.commands[4].wrapping_add(1),
        crate::hotbar::Action::Special { id, .. } => keys.special = (keys.special.0.wrapping_add(1), *id),
        crate::hotbar::Action::Social { id, .. } => keys.social = (keys.social.0.wrapping_add(1), *id),
        crate::hotbar::Action::Nano { id, .. } => keys.cast = (keys.cast.0.wrapping_add(1), *id),
        crate::hotbar::Action::Item { id, name, .. } => {
            // The first stack of that item (the backpack before anything worn).
            let found = hud.gear.iter().filter(|i| i.item_id == *id).max_by_key(|i| i.slot >= mao_net::inventory::BACKPACK_FIRST).map(|i| i.slot);
            match found {
                Some(slot) => keys.fire = (keys.fire.0.wrapping_add(1), slot),
                None => hud.say(format!("{name} is not in your inventory")),
            }
        }
    }
}

/// A press of slot `slot` of the shown hotbar: fire, or tell the player it is not ready (the press is queued).
fn press_slot(ui: &mut Ui, slot: usize, keys: &mut Input, hud: &mut Hud) {
    let now = ui.now();
    match ui.hotbars.press(slot, now) {
        crate::hotbar::Press::Fire(action) => fire_action(&action, keys, hud),
        crate::hotbar::Press::NotReady(message) => hud.say(message),
        crate::hotbar::Press::Empty => {}
    }
}

/// How far a flying vehicle pitches up or down (the camera's own limit, `PITCH_LIMIT`).
const FLY_PITCH_LIMIT: f32 = 1.4;

/// What is under a point of the window (physical pixels): the first pickable the line from the eye through it meets, as a
/// click picks.
fn pick(hud: &Hud, camera: mao_render::Camera, radius: f32, size: (u32, u32), at: (f32, f32)) -> Option<(u32, u32)> {
    let aspect = size.0 as f32 / size.1.max(1) as f32;
    let (from, dir) = camera.ray(at.0 / size.0.max(1) as f32, at.1 / size.1.max(1) as f32, aspect, radius);
    hud.pickables
        .iter()
        .filter_map(|&(kind, id, feet, r, h, last)| ray_cylinder(from, dir, feet, r, h).map(|t| ((last, t), (kind, id))))
        .min_by(|a, b| a.0 .0.cmp(&b.0 .0).then(a.0 .1.total_cmp(&b.0 .1)))
        .map(|(_, k)| k)
}

fn heading_from_yaw(yaw: f32) -> f32 {
    (-yaw.cos()).atan2(-yaw.sin())
}

fn yaw_behind(heading: f32) -> f32 {
    heading_from_yaw(heading)
}

/// The settings' camera mode as the original's `PreferredCameraMode`: "trail" is the free camera (1), "rubber" the one
/// that steers behind the character (2), "lock" the one held behind it (3).
fn camera_mode(name: &str) -> crate::camera::Mode {
    match name {
        "lock" => crate::camera::Mode::Locked,
        "trail" => crate::camera::Mode::Free,
        _ => crate::camera::Mode::Rubber,
    }
}

/// Where a line (from `from`, along the unit `dir`) first meets an upright cylinder standing at `feet` (Z up), as a
/// distance along it; none when it misses or meets it behind the start.
fn ray_cylinder(from: mao_render::glam::Vec3, dir: mao_render::glam::Vec3, feet: [f32; 3], r: f32, h: f32) -> Option<f32> {
    let (ox, oy) = (from.x - feet[0], from.y - feet[1]);
    let a = dir.x * dir.x + dir.y * dir.y;
    let mut best: Option<f32> = None;
    let mut take = |t: f32| {
        if t > 0.0 && best.is_none_or(|b| t < b) {
            best = Some(t);
        }
    };
    // the side
    if a > 1e-9 {
        let b = 2.0 * (ox * dir.x + oy * dir.y);
        let c = ox * ox + oy * oy - r * r;
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            for t in [(-b - disc.sqrt()) / (2.0 * a), (-b + disc.sqrt()) / (2.0 * a)] {
                let z = from.z + t * dir.z;
                if z >= feet[2] && z <= feet[2] + h {
                    take(t);
                }
            }
        }
    }
    // the top and the bottom
    if dir.z.abs() > 1e-9 {
        for cap in [feet[2], feet[2] + h] {
            let t = (cap - from.z) / dir.z;
            let (x, y) = (ox + t * dir.x, oy + t * dir.y);
            if x * x + y * y <= r * r {
                take(t);
            }
        }
    }
    best
}

impl ApplicationHandler for App {
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let DeviceEvent::MouseMotion { delta: (dx, dy) } = event else { return };
        if self.live.is_none() || self.buttons == [false; 2] {
            return;
        }
        // (how far the mouse went while held, in screen heights: past 0.02 it is a drag, not a click, as
        // ActionViewMouseHandler_c's FUN_1002ccbe counts it against 0x101aeafc)
        let height = self.state.as_ref().map_or(1080.0, |w| w.window.inner_size().height.max(1) as f32);
        self.moved += (dx * dx + dy * dy).sqrt() as f32 / height;
        if !self.buttons[1] && !crate::prefs::on("LMBMouseLook") {
            return;
        }
        let Some(w) = self.state.as_mut() else { return };
        // The mouse look goes to the game's camera at the next frame (across: right positive; up: mouse up positive).
        self.mouse_look.0 += dx as f32 * 0.005;
        // (`MouseLookInverted`: the option panel's Mouse page, Gamecode's `StaticMouseLookInvertCallback`)
        let invert = if crate::prefs::on("MouseLookInverted") { -1.0 } else { 1.0 };
        // (flying, `N3Msg_MouseMovement` in fly mode: the right button's vertical movement pitches the vehicle, not the
        // camera, `MouseCameraControl(0, 0)`, and goes to the vehicle with the across movement, 0x2b)
        if self.hud.flying && self.buttons[1] && !(self.modifiers.shift_key() || self.modifiers.control_key()) {
            self.keys.pitch = (self.keys.pitch - invert * dy as f32 * 0.005).clamp(-FLY_PITCH_LIMIT, FLY_PITCH_LIMIT);
        } else {
            self.mouse_look.1 -= invert * dy as f32 * 0.005;
        }
        // (the right button turns the character, `N3Msg_MouseMovement`; with Shift or Ctrl only the camera, as the left does)
        if self.buttons[1] && !(self.modifiers.shift_key() || self.modifiers.control_key()) {
            self.keys.face = Some(heading_from_yaw(w.camera.yaw));
            if let Some(live) = &self.live {
                let _ = live.input.send(self.keys);
            }
        }
        w.window.request_redraw();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // (the window is made hidden and shown once its first frame is presented; a hidden window is sent no redraws, so
        // that first frame is drawn from here)
        if let Some(id) = self.state.as_ref().filter(|w| w.window.is_visible() == Some(false)).map(|w| w.window.id()) {
            self.window_event(event_loop, id, WindowEvent::RedrawRequested);
        }
        if self.live.is_some() {
            // No frame cap: draw as fast as the present mode allows (the original stopped at 100 fps).
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
            if let Some(w) = &self.state {
                w.window.request_redraw();
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        // (hidden until its first frame is presented: otherwise the system's white fills it first, then the black)
        let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("Modern Anarchy Online").with_inner_size(PhysicalSize::new(self.settings.width, self.settings.height)).with_visible(false)).expect("a window"));
        apply_mode(&window, self.settings.mode);
        eprintln!("window mode: {} (F11 or Alt+Enter change it)", self.settings.mode.name());
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("a surface");
        let gpu = Gpu::new(instance, Some(&surface)).expect("a graphics device");
        let size = window.inner_size();
        let mut config = surface.get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1)).expect("a surface format");
        // Uncapped unless `vsync = on`: Immediate, else Mailbox, else the always-available Fifo.
        let modes = surface.get_capabilities(&gpu.adapter).present_modes;
        config.present_mode = if self.settings.vsync {
            wgpu::PresentMode::Fifo
        } else {
            [wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox].into_iter().find(|m| modes.contains(m)).unwrap_or(wgpu::PresentMode::Fifo)
        };
        eprintln!("present mode: {:?}", config.present_mode);
        surface.configure(&gpu.device, &config);
        let renderer = Renderer::new(&gpu, config.format);
        let mut scene = renderer.upload(&gpu, &self.model, &mut |k, i| self.textures.get_for_upload(k, i));
        if self.live.is_some() {
            // Close-up view: distance fades into the sky.
            scene.fog = fog_for(self.settings.view_distance);
        }
        let mut camera = Camera::fit(&scene);
        if let Some(([x, y, z], distance)) = self.live.as_ref().and_then(|l| l.focus.map(|f| (f, l.distance))) {
            camera = Camera { target: mao_render::glam::Vec3::new(x, y, z), distance, yaw: 0.6, pitch: 0.5 };
        }
        window.request_redraw();
        self.home = Some(camera);
        let ui = self.live.as_ref().map(|_| Ui::new(&window, &gpu, config.format, &self.settings));
        self.state = Some(View {
            window,
            surface,
            config,
            gpu,
            renderer,
            scene,
            camera,
            ui,
            #[cfg(windows)]
            page: None,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(w) = self.state.as_mut() else { return };
        if let Some(ui) = w.ui.as_mut() {
            // The key the options window waits for goes to it, not to the interface.
            let capturing = ui.rebinding.is_some() && matches!(event, WindowEvent::KeyboardInput { .. });
            // (a button let go that went down in the world is the world's, wherever the pointer is now)
            let world_release = match &event {
                WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => self.buttons[0],
                WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Right, .. } => self.buttons[1],
                _ => false,
            };
            let taken = !capturing && !matches!(event, WindowEvent::RedrawRequested | WindowEvent::ModifiersChanged(_) | WindowEvent::CloseRequested | WindowEvent::Resized(_)) && ui.event(&w.window, &event);
            if taken && !world_release {
                // (MAO_MOUSE_LOG=1: where the buttons go and what the mouse picks)
                if let (WindowEvent::MouseInput { state, button, .. }, true) = (&event, std::env::var_os("MAO_MOUSE_LOG").is_some()) {
                    eprintln!("mouse {button:?} {state:?} at {:?}: taken by the interface ({})", self.last, ui.under_pointer());
                }
                if matches!(event, WindowEvent::MouseInput { state: ElementState::Released, .. }) {
                    self.dragging = false;
                    if self.buttons != [false; 2] {
                        self.buttons = [false; 2];
                        w.window.set_cursor_visible(true);
                        let _ = w.window.set_cursor_grab(CursorGrabMode::None);
                        self.keys.face = None;
                        if let Some(live) = &self.live {
                            let _ = live.input.send(self.keys);
                        }
                    }
                }
                w.window.request_redraw();
                return;
            }
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                w.config.width = size.width.max(1);
                w.config.height = size.height.max(1);
                w.surface.configure(&w.gpu.device, &w.config);
                w.window.request_redraw();
            }
            // Playing: GUI.dll's `ActionViewMouseHandler_c`. Held and moved (FUN_1002ccbe), the left button looks
            // round with the camera (`LMBMouseLook`, on by default: `N3Msg_CameraMouseLookMovement`) and the right
            // one turns the character (`N3Msg_MouseMovement`; with Shift or Ctrl the camera only). A click without
            // moving the mouse (its mouse-up FUN_1002cfac): the left button selects what is under the pointer, the
            // right one uses it (a character's default action, `N3Msg_DefaultActionOnDynel`; anything else
            // `N3Msg_UseItem`). The middle button resets the camera, its wheel zooms.
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Middle, .. } if self.live.is_some() => {
                self.cam_reset = true;
            }
            WindowEvent::MouseInput { state, button: button @ (MouseButton::Left | MouseButton::Right), .. } if self.live.is_some() => {
                if state == ElementState::Pressed && !self.buttons[0] && !self.buttons[1] {
                    self.moved = 0.0;
                }
                let released = state == ElementState::Released && self.moved < 0.02;
                let clicked = if released && button == MouseButton::Left && self.buttons[0] && !self.buttons[1] {
                    Some(false)
                } else if released && button == MouseButton::Right && self.buttons[1] && !self.buttons[0] {
                    Some(true)
                } else {
                    None
                };
                self.buttons[(button == MouseButton::Right) as usize] = state == ElementState::Pressed;
                let log = std::env::var_os("MAO_MOUSE_LOG").is_some();
                if log {
                    eprintln!("mouse {button:?} {state:?} at {:?}: moved {:.3}, click {:?}, {} pickables", self.last, self.moved, clicked, self.hud.pickables.len());
                }
                if let Some(use_it) = clicked {
                    let size = w.window.inner_size();
                    let (camera, radius) = (w.camera, w.scene.radius);
                    let aspect = size.width as f32 / size.height.max(1) as f32;
                    let (cx, cy) = (self.last.0 as f32, self.last.1 as f32);
                    // (the first thing the line from the eye through the pointer meets, as the original's camera line
                    // finds the first dynel on it: each a cylinder standing on its feet)
                    let (from, dir) = camera.ray(cx / size.width.max(1) as f32, cy / size.height.max(1) as f32, aspect, radius);
                    let nearest = self
                        .hud
                        .pickables
                        .iter()
                        .filter_map(|&(kind, id, feet, r, h, last)| ray_cylinder(from, dir, feet, r, h).map(|t| ((last, t), (kind, id))))
                        .min_by(|a, b| a.0 .0.cmp(&b.0 .0).then(a.0 .1.total_cmp(&b.0 .1)));
                    if log {
                        eprintln!("  first on the line {nearest:?} of {}", self.hud.pickables.len());
                    }
                    if let Some((_, (kind, id))) = nearest {
                        let double = use_it || self.last_click.is_some_and(|(last, at)| last == id && at.elapsed() < std::time::Duration::from_millis(500));
                        self.last_click = Some((id, std::time::Instant::now()));
                        self.keys.pick = (self.keys.pick.0.wrapping_add(1), kind, id, double);
                    }
                }
                // (the left button looks round only with `LMBMouseLook`)
                let any = (self.buttons[0] && crate::prefs::on("LMBMouseLook")) || self.buttons[1];
                w.window.set_cursor_visible(!any);
                if any {
                    if w.window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                        let _ = w.window.set_cursor_grab(CursorGrabMode::Confined);
                    }
                } else {
                    let _ = w.window.set_cursor_grab(CursorGrabMode::None);
                }
                self.keys.face = (self.buttons[1] && !(self.modifiers.shift_key() || self.modifiers.control_key())).then(|| heading_from_yaw(w.camera.yaw));
                self.keys.forward = self.held.contains(&Action::Forward) || self.autorun;
                if let Some(live) = &self.live {
                    let _ = live.input.send(self.keys);
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => self.dragging = state == ElementState::Pressed,
            WindowEvent::CursorMoved { position, .. } => {
                if self.dragging && self.live.is_none() {
                    w.camera.yaw -= (position.x - self.last.0) as f32 * 0.01;
                    w.camera.pitch = (w.camera.pitch + (position.y - self.last.1) as f32 * 0.01).clamp(-1.5, 1.5);
                    w.window.request_redraw();
                }
                self.last = (position.x, position.y);
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            // Playing (a live window): the keys come from the settings.
            WindowEvent::KeyboardInput { event, .. } if self.live.is_some() => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                let down = event.state == ElementState::Pressed;
                if let Some(action) = w.ui.as_ref().and_then(|u| u.rebinding).filter(|_| down) {
                    // The options window is waiting for this key.
                    let modifier_only = matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight | KeyCode::ControlLeft | KeyCode::ControlRight | KeyCode::AltLeft | KeyCode::AltRight | KeyCode::SuperLeft | KeyCode::SuperRight);
                    if !modifier_only {
                        if let Some(ui) = w.ui.as_mut() {
                            ui.rebinding = None;
                        }
                        if code != KeyCode::Escape || self.modifiers.shift_key() || self.modifiers.control_key() || self.modifiers.alt_key() {
                            let binding = Binding { key: code, shift: self.modifiers.shift_key(), ctrl: self.modifiers.control_key(), alt: self.modifiers.alt_key() };
                            self.settings.keymap.assign(action, binding);
                            self.settings.save();
                        }
                        w.window.request_redraw();
                    }
                    return;
                }
                if !down {
                    // A released key ends every held action it started.
                    for action in self.settings.keymap.actions_for_key(code) {
                        self.held.remove(&action);
                    }
                } else if let Some(action) = self.settings.keymap.action(code, self.modifiers) {
                    match action {
                        Action::AutoRun if !event.repeat => {
                            self.autorun = !self.autorun;
                        }
                        Action::ToggleWalk if !event.repeat => {
                            self.keys.walk_toggle = self.keys.walk_toggle.wrapping_add(1);
                        }
                        Action::Forward | Action::Back | Action::TurnLeft | Action::TurnRight | Action::StrafeLeft | Action::StrafeRight | Action::Jump => {
                            // (walking by hand ends the auto-run)
                            if matches!(action, Action::Forward | Action::Back) {
                                self.autorun = false;
                            }
                            self.held.insert(action);
                        }
                        Action::NoClip if !event.repeat => {
                            self.keys.noclip = !self.keys.noclip;
                            eprintln!("walking through props: {}", if self.keys.noclip { "on" } else { "off" });
                        }
                        Action::CycleWindowMode | Action::ToggleFullscreen if !event.repeat => {
                            self.settings.mode = if action == Action::CycleWindowMode {
                                self.settings.mode.next()
                            } else if self.settings.mode == WindowMode::Windowed {
                                WindowMode::Borderless
                            } else {
                                WindowMode::Windowed
                            };
                            apply_mode(&w.window, self.settings.mode);
                            eprintln!("window mode: {} (set `mode` in {} to keep it)", self.settings.mode.name(), Settings::path().display());
                        }
                        Action::CloseWindows if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.close_with_escape();
                            }
                        }
                        Action::SelectSelf if !event.repeat => {
                            self.keys.select_self = self.keys.select_self.wrapping_add(1);
                        }
                        Action::TargetInfo if !event.repeat => {
                            self.keys.inspect = self.keys.inspect.wrapping_add(1);
                        }
                        Action::Coordinates if !event.repeat => {
                            if let Some([x, _, z]) = self.hud.position {
                                let place = if self.hud.location.is_empty() { String::new() } else { format!("{}, ", self.hud.location) };
                                self.hud.say(format!("{place}{x:.0}, {z:.0}"));
                            }
                        }
                        Action::CameraToggleView if !event.repeat => {
                            // (`ToggleCameraView`: first person and back)
                            self.steering.toggle_view();
                        }
                        Action::Quick(n) if !event.repeat => {
                            if let Some(&(_, _, id, name)) = crate::settings::QUICK.get(n as usize) {
                                fire_action(&crate::hotbar::Action::Special { id, name: name.to_string(), icon: None }, &mut self.keys, &mut self.hud);
                            }
                        }
                        Action::HotbarBar(n) if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.hotbars.current = (n as usize).min(crate::hotbar::BARS - 1);
                            }
                        }
                        Action::HotbarSlot(n) if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                press_slot(ui, n as usize, &mut self.keys, &mut self.hud);
                            }
                        }
                        // (`COMMAND_TEXT_INPUT_START_TEXT_INPUT` on Enter, `COMMAND_START_CHAT_CMD_INPUT` on `/`: the line
                        // starts with the slash)
                        Action::ChatInput if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.chat.focus();
                            }
                        }
                        Action::ChatCommand if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.chat.type_text("/");
                            }
                        }
                        Action::UiLarger | Action::UiSmaller => {
                            self.settings.step_ui_scale(if action == Action::UiLarger { 1 } else { -1 });
                            self.settings.save();
                        }
                        Action::HelpWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.help.open = !ui.help.open;
                            }
                        }
                        Action::Screenshot if !event.repeat => {
                            self.screenshot = true;
                        }
                        Action::PerkWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.perks.open = !ui.perks.open;
                            }
                        }
                        Action::ProgramsWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.programs.open = !ui.programs.open;
                            }
                        }
                        Action::FriendsWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.friends.open = !ui.friends.open;
                            }
                        }
                        Action::NcuWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.ncu.open = !ui.ncu.open;
                            }
                        }
                        Action::SkillsWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.skills.open = !ui.skills.open;
                            }
                        }
                        Action::StatsWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.stats.open = !ui.stats.open;
                            }
                        }
                        Action::MapOverlay if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.automap.overlay = !ui.automap.overlay;
                            }
                        }
                        Action::ActionsWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.actions.open = !ui.actions.open;
                                if ui.actions.open {
                                    ui.actions.load_icons(&self.textures);
                                }
                            }
                        }
                        Action::MailWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.mail.open = !ui.mail.open;
                            }
                        }
                        Action::MissionWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.mission_log.open = !ui.mission_log.open;
                            }
                        }
                        Action::PlanetMapWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.planet.toggle();
                            }
                        }
                        Action::WhompahWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.whompah_open = !ui.whompah_open;
                            }
                        }
                        Action::InventoryWindow if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.gear_open = !ui.gear_open;
                            }
                        }
                        Action::ToggleOptions if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.toggle_options();
                            }
                        }
                        Action::Quit => {
                            event_loop.exit();
                            return;
                        }
                        Action::NextHostileTarget | Action::PrevHostileTarget | Action::NextFriendlyTarget | Action::PrevFriendlyTarget | Action::Attack | Action::SitStand | Action::Use if !event.repeat => {
                            let slot = match action {
                                Action::NextHostileTarget => 0,
                                Action::PrevHostileTarget => 1,
                                Action::NextFriendlyTarget => 2,
                                Action::PrevFriendlyTarget => 3,
                                Action::Attack => 4,
                                Action::SitStand => 5,
                                _ => 6,
                            };
                            self.keys.commands[slot] = self.keys.commands[slot].wrapping_add(1);
                            if std::env::var_os("MAO_COMBAT_LOG").is_some() {
                                eprintln!("combat: key {:?} -> command {slot} now {}", event.physical_key, self.keys.commands[slot]);
                            }
                        }
                        // (the rotate and zoom keys act while held: the frame reads them from `held`)
                        Action::CameraRotateLeft | Action::CameraRotateRight | Action::CameraRotateUp | Action::CameraRotateDown | Action::CameraZoomIn | Action::CameraZoomOut => {}
                        Action::CameraSavePreferred if !event.repeat => {
                            self.preferred = Some(w.camera);
                            eprintln!("camera saved");
                        }
                        Action::CameraReset if !event.repeat => self.cam_reset = true,
                        _ => {}
                    }
                }
                let held = |a: Action| self.held.contains(&a);
                self.keys.forward = held(Action::Forward) || self.autorun;
                self.keys.back = held(Action::Back);
                self.keys.jump = held(Action::Jump);
                self.keys.turn = held(Action::TurnRight) as i8 - held(Action::TurnLeft) as i8;
                self.keys.strafe = held(Action::StrafeRight) as i8 - held(Action::StrafeLeft) as i8;
                if let Some(live) = &self.live {
                    let _ = live.input.send(self.keys);
                }
                w.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && !event.repeat && matches!(self.settings.keymap.action_any(event.physical_key, self.modifiers), Some(Action::CycleWindowMode | Action::ToggleFullscreen)) => {
                let action = self.settings.keymap.action_any(event.physical_key, self.modifiers);
                self.settings.mode = if action == Some(Action::CycleWindowMode) {
                    self.settings.mode.next()
                } else if self.settings.mode == WindowMode::Windowed {
                    WindowMode::Borderless
                } else {
                    WindowMode::Windowed
                };
                apply_mode(&w.window, self.settings.mode);
                eprintln!("window mode: {}", self.settings.mode.name());
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                // WASD moves the view point over the ground (relative to where the
                // camera faces), Q and E down and up; the step grows with the distance.
                let step = w.camera.distance * 0.05;
                let (sin, cos) = w.camera.yaw.sin_cos();
                let forward = mao_render::glam::Vec3::new(-cos, -sin, 0.0);
                let right = mao_render::glam::Vec3::new(sin, -cos, 0.0);
                let delta = match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyW) => forward * step,
                    PhysicalKey::Code(KeyCode::KeyS) => -forward * step,
                    PhysicalKey::Code(KeyCode::KeyA) => -right * step,
                    PhysicalKey::Code(KeyCode::KeyD) => right * step,
                    PhysicalKey::Code(KeyCode::KeyE) => mao_render::glam::Vec3::Z * step,
                    PhysicalKey::Code(KeyCode::KeyQ) => -mao_render::glam::Vec3::Z * step,
                    PhysicalKey::Code(KeyCode::Escape) => {
                        event_loop.exit();
                        return;
                    }
                    _ => return,
                };
                w.camera.target += delta;
                w.window.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                if self.live.is_some() {
                    // (a notch up zooms the game's camera in)
                    self.wheel -= lines;
                } else {
                    let closest = w.scene.radius * 0.1;
                    w.camera.distance = (w.camera.distance * (1.0 - lines * 0.1)).max(closest);
                }
                w.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let mut prof_t = std::time::Instant::now();
                // Queued hotbar presses fire as soon as something in their slot is ready.
                if let Some(ui) = w.ui.as_mut() {
                    let now = ui.now();
                    for action in ui.hotbars.tick(now) {
                        fire_action(&action, &mut self.keys, &mut self.hud);
                    }
                    // Only a change goes to the network thread (a redraw can come thousands of times a second).
                    if let Some(live) = &self.live {
                        if self.keys != self.last_sent {
                            self.last_sent = self.keys;
                            let _ = live.input.send(self.keys);
                        }
                    }
                }
                // The game's camera is stepped once the character's place for this frame is known (below).
                let dt = self.last_follow.elapsed().as_secs_f32().min(0.25);
                self.last_follow = std::time::Instant::now();
                if let Some(live) = &self.live {
                    // Every update's messages count; the last one's picture is drawn.
                    let mut updates: Vec<Update> = live.overlays.try_iter().collect();
                    // The game's connection thread ends with the connection: the server logged the character out.
                    if matches!(live.overlays.try_recv(), Err(std::sync::mpsc::TryRecvError::Disconnected)) && !self.logged_out {
                        self.logged_out = true;
                        event_loop.exit();
                    }
                    for u in &mut updates {
                        for m in std::mem::take(&mut u.messages) {
                            self.hud.say(m);
                        }
                        self.hud.target = u.target.take();
                        if let Some(bars) = u.target_bars.take() {
                            self.hud.target_bars = bars;
                        }
                        if let Some(d) = u.destinations.take() {
                            self.hud.destinations = Some(d);
                        }
                        if let Some(ui) = w.ui.as_mut() {
                            if let Some(offers) = u.missions.take() {
                                ui.mission.set_offers(offers);
                            }
                            if let Some(status) = u.mission_status.take() {
                                ui.mission.status = status;
                            }
                            if let Some(entries) = u.mail_entries.take() {
                                ui.mail.set_entries(entries);
                            }
                            if let Some(opened) = u.mail_opened.take() {
                                ui.mail.opened = Some(opened);
                            }
                            for url in std::mem::take(&mut u.info_urls) {
                                ui.info.show_url(&url);
                            }
                            for name in std::mem::take(&mut u.open_windows) {
                                ui.set_window(name, true);
                            }
                            if let Some(request) = u.browser.take() {
                                ui.browser.show_page(request);
                            }
                            if let Some(state) = u.shop.take() {
                                ui.shop.set(state);
                            }
                            if let Some(status) = u.shop_status.take() {
                                ui.shop.status = status;
                            }
                            for change in std::mem::take(&mut u.shop_changes) {
                                ui.shop.apply(change);
                            }
                            if let Some(list) = u.mission_log.take() {
                                ui.mission_log.set(list);
                            }
                            for id in std::mem::take(&mut u.missions_removed) {
                                ui.mission_log.remove(id);
                            }
                            if !u.npc_chat.is_empty() {
                                ui.npc_chat.own_name = self.hud.character_name.clone();
                                for event in std::mem::take(&mut u.npc_chat) {
                                    ui.npc_chat.apply(event);
                                }
                            }
                            if let Some((id, flags)) = u.mail_flags.take() {
                                ui.mail.set_flags(id, flags);
                            }
                            if let Some(n) = u.mail_count.take() {
                                ui.mail.count = Some(n);
                                ui.mail.status = format!("The inbox holds {n} messages.");
                            }
                            if let Some(status) = u.mail_status.take() {
                                ui.mail.status = status;
                            }
                        }
                        if let Some((name, stats, parts)) = u.stats.take() {
                            self.hud.character_name = name;
                            self.hud.stats = stats;
                            self.hud.stat_parts = parts;
                        }
                        if let (Some(patch), Some(ui)) = (u.map.take(), w.ui.as_mut()) {
                            if let Some(pf) = patch.playfield {
                                self.hud.playfield = pf;
                            }
                            ui.automap.apply(patch);
                        }
                        if let Some(ui) = w.ui.as_mut() {
                            for line in std::mem::take(&mut u.chat_lines) {
                                ui.chat.log = true;
                                ui.chat.push(line);
                            }
                        }
                        if let Some(loot) = u.loot.take() {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.loot.open = loot.is_some();
                            }
                            self.hud.loot = loot;
                        }
                        if let Some(perks) = u.perk_list.take() {
                            self.hud.perk_state = perks;
                        }
                        if let Some(programs) = u.programs.take() {
                            self.hud.programs = programs;
                        }
                        if let Some(perks) = u.perk_actions.take() {
                            self.hud.perk_actions = perks;
                        }
                        if let Some(friends) = u.friends.take() {
                            self.hud.friends = friends;
                        }
                        if let Some(items) = u.gear.take() {
                            self.hud.gear = items;
                        }
                        if let Some(flying) = u.flying.take() {
                            self.hud.flying = flying;
                            if !flying {
                                self.keys.pitch = 0.0;
                            }
                        }
                        if let Some((name, items)) = u.inspected.take() {
                            self.hud.inspected = (self.hud.inspected.0 + 1, name, items);
                        }
                        if let Some(nanos) = u.nanos.take() {
                            self.hud.nanos = nanos;
                        }
                        if let Some(name) = u.location.take() {
                            self.hud.location = name;
                        }
                        if let Some(p) = u.pickables.take() {
                            self.hud.pickables = p;
                        }
                        if let Some(ui) = w.ui.as_mut() {
                            let now = ui.now();
                            for (stat, seconds) in std::mem::take(&mut u.special_cooldowns) {
                                ui.hotbars.set_special_cooldown(stat, seconds as f64, now);
                            }
                        }
                        for (world, text, kind) in std::mem::take(&mut u.floaters) {
                            self.hud.floaters.push(crate::ui::Floater { world: mao_render::glam::Vec3::from(world), text, kind, born: std::time::Instant::now() });
                        }
                        for (world, color) in std::mem::take(&mut u.glows) {
                            self.hud.glows.push(crate::ui::Glow { world: mao_render::glam::Vec3::from(world), color, born: std::time::Instant::now() });
                        }
                        if let Some(auras) = u.auras.take() {
                            self.auras = auras;
                        }
                        // (each effect made only while its category is on: Gamecode's `isEffectInCategoryActive`, the
                        // option panel's Effects switches; a shot's trail and flash are the tracers', a cast's the nano
                        // effects', a lone flare and a landing others', a strike the environment's)
                        let shown = crate::prefs::effect_category_active;
                        for shot in u.shots.take().unwrap_or_default() {
                            if shown(crate::prefs::TRACERS) {
                                self.shots.push((std::time::Instant::now(), shot));
                            }
                        }
                        for cast in u.bursts.take().unwrap_or_default() {
                            if shown(crate::prefs::NANO) {
                                self.bursts.push((std::time::Instant::now(), cast));
                            }
                        }
                        for (at, flare) in std::mem::take(&mut u.flares) {
                            if shown(crate::prefs::OTHERS) {
                                self.flares.push((std::time::Instant::now(), at, flare));
                            }
                        }
                        let ground = self.ground.clone();
                        for (at, seed) in std::mem::take(&mut u.strikes) {
                            if shown(crate::prefs::ENVIRONMENT) {
                                self.sky.strike(at, seed, &|x, z| ground.as_ref().map(|(t, o)| t.height_at(x, z, t.height_scale) + o));
                            }
                        }
                        if let Some(f) = u.sky_frame.take() {
                            self.fxs_frame = Some(f);
                        }
                        for at in std::mem::take(&mut u.landings) {
                            if shown(crate::prefs::OTHERS) {
                                self.sky.landing(at, &|x, z| ground.as_ref().map(|(t, o)| t.height_at(x, z, t.height_scale) + o));
                            }
                        }
                        for level in std::mem::take(&mut u.jolts) {
                            self.quake.jolt(level);
                        }
                        self.hud.glows.retain(|g| g.born.elapsed().as_secs_f32() < crate::ui::GLOW_LIFE);
                        self.hud.floaters.retain(|f| f.born.elapsed().as_secs_f32() < crate::ui::FLOATER_LIFE);
                        if let Some(labels) = u.labels.take() {
                            self.hud.labels = labels.into_iter().map(|(p, text)| Label { world: mao_render::glam::Vec3::from(p), text }).collect();
                        }
                    }
                    // (a scene, an environment or a daylight comes once, in one update: one that came before the last must
                    // not be lost with it, or a zoning's new playfield is never drawn and the loading screen stays)
                    let (mut scene, mut environment, mut daylight) = (None, None, None);
                    for u in updates.iter_mut().filter(|u| !u.messages_only) {
                        if let Some(s) = u.scene.take() {
                            scene = Some(s);
                        }
                        if let Some(e) = u.environment.take() {
                            environment = Some(e);
                        }
                        if let Some(d) = u.daylight.take() {
                            daylight = Some(d);
                        }
                    }
                    let latest = updates.into_iter().filter(|u| !u.messages_only).last();
                    if let Some(mut update) = latest {
                        (update.scene, update.environment, update.daylight) = (scene, environment, daylight);
                        // (the first world update after logging in: the world is in and the loading screen goes,
                        // `StartClosingLoadscreenMessage`; the login's playfield was drawn before the window opened, so no
                        // scene comes with it)
                        if !std::mem::replace(&mut self.world_in, true) {
                            crate::loading::done();
                        }
                        self.hud.position = update.own;
                        self.hud.heading = self.heading;
                        self.hud.map_dots = std::mem::take(&mut update.map_dots);
                        if let Some(towers) = update.towers.take() {
                            self.hud.towers = towers;
                        }
                        self.hud.nameplates = std::mem::take(&mut update.nameplates);
                        if let Some(h) = update.heading {
                            self.heading = h;
                            if !self.behind_set {
                                self.behind_set = true;
                                w.camera.yaw = yaw_behind(h);
                            }
                        }
                        self.ground = update.ground.take();
                        self.zones = update.zones.take();
                        if let Some(e) = update.environment.take() {
                            self.environment = e;
                        }
                        if let Some(d) = update.daylight.take() {
                            self.daylight = d;
                        }
                        if let Some(model) = &update.scene {
                            let uploading = std::time::Instant::now();
                            w.scene = w.renderer.upload(&w.gpu, model, &mut |k, i| self.textures.get_for_upload(k, i));
                            profile!("[profile] area scene upload {:?} ({} parts)", uploading.elapsed(), model.parts.len());
                            w.scene.fog = fog_for(self.settings.view_distance);
                            // (the world is in: the loading screen goes, `StartClosingLoadscreenMessage`)
                            crate::loading::done();
                        }
                        let own_started = std::time::Instant::now();
                        // (into the last update's buffers where the parts kept their sizes: `Renderer::reupload`)
                        w.renderer.reupload_follow(&w.gpu, &mut w.scene, &update.own_model, &mut |k, i| self.textures.get_for_upload(k, i));
                        crate::prof::add("upload-own", own_started.elapsed());
                        self.casters = std::mem::take(&mut update.casters);
                        // (a playfield entered, the first one too: the original makes a new camera behind the character)
                        if let (true, Some(p)) = (update.scene.is_some(), update.own) {
                            self.steering.create(p, self.heading);
                        }
                        if let Some(p) = update.own {
                            self.own_base = p;
                            self.own_velocity = update.own_velocity;
                            self.own_at = std::time::Instant::now();
                        }
                        let uploading = std::time::Instant::now();
                        w.renderer.reupload_overlay(&w.gpu, &mut w.scene, &update.model, &mut |k, i| self.textures.get_for_upload(k, i));
                        crate::prof::add("upload-overlay", uploading.elapsed());
                        if std::env::var_os("MAO_PROFILE").is_some() && uploading.elapsed() > std::time::Duration::from_millis(2) {
                            eprintln!("[profile] overlay upload {:?}", uploading.elapsed());
                        }
                    }
                    // Between updates the character and the camera move on at the speed it was walking at,
                    // so they glide at the frame rate instead of stepping with the network.
                    if self.hud.position.is_some() {
                        let ahead = self.own_at.elapsed().as_secs_f32().min(0.12);
                        let [x, y, z] = self.own_base;
                        let [vx, _, vz] = self.own_velocity;
                        let (px, pz) = (x + vx * ahead, z + vz * ahead);
                        w.scene.move_follow(&w.gpu, mao_render::glam::Vec3::new(vx * ahead, vz * ahead, 0.0));
                        self.follow_shift = [vx * ahead, vz * ahead, 0.0];
                        // The player's feet: plants bend away from them.
                        w.scene.player = Some(mao_render::glam::Vec3::new(px, pz, y));
                        // The game's camera: the original's steered vehicle (see `camera.rs`), looking at the head, in the
                        // server's frame; the terrain stands in for its lines of sight.
                        let held = |a: Action| self.held.contains(&a);
                        let inputs = crate::camera::Inputs {
                            rotate_left: held(Action::CameraRotateLeft),
                            rotate_right: held(Action::CameraRotateRight),
                            rotate_up: held(Action::CameraRotateUp),
                            rotate_down: held(Action::CameraRotateDown),
                            zoom_in: held(Action::CameraZoomIn),
                            zoom_out: held(Action::CameraZoomOut),
                            mouse: if self.buttons == [false; 2] { (0.0, 0.0) } else { self.mouse_look },
                            wheel: std::mem::take(&mut self.wheel),
                            reset: std::mem::take(&mut self.cam_reset),
                        };
                        self.mouse_look = (0.0, 0.0);
                        let ground = self.ground.clone();
                        let zones = self.zones.clone();
                        let mut los = |a: [f32; 3], b: [f32; 3]| -> bool {
                            // (the playfield's surfaces block the view too, the walls and floors indoors: the camera vehicle's
                            // `GetLineIntersection` against the whole surface)
                            if zones.as_ref().is_some_and(|z| z.line(a, b).is_some()) {
                                return false;
                            }
                            let Some((terrain, offset)) = &ground else { return true };
                            let steps = ((a[0] - b[0]).hypot(a[2] - b[2]) / 0.5).ceil().clamp(1.0, 64.0) as usize;
                            (0..=steps).all(|i| {
                                let t = i as f32 / steps as f32;
                                let (x, y, z) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t);
                                y >= terrain.height_at(x, z, terrain.height_scale) + offset + 0.3
                            })
                        };
                        let speed = (vx * vx + vz * vz).sqrt();
                        let (eye, at) = self.steering.update(dt, [px, y + 1.3, pz], self.heading, speed, inputs, &mut los);
                        w.camera = mao_render::Camera::looking(mao_render::glam::Vec3::new(eye[0], eye[2], eye[1]), mao_render::glam::Vec3::new(at[0], at[2], at[1]));
                        if self.steering.is_first_person() {
                            // (looking out of the head: the target is a step ahead, the eye at the head)
                            w.camera.distance = 0.01;
                        }
                    }
                }
                // The sparks of the running nano programs, where they are now (the character's own glides with it).
                {
                    let time = self.fx_clock.elapsed().as_secs_f32();
                    // (the wind run on from the weather's wind, entry 7, and handed to the effects that it pushes)
                    let frame_time = self.wind_at.elapsed().as_secs_f32();
                    self.wind.advance(self.environment.weather[7], frame_time);
                    // (the quake: its sound, or stopped, and the eye's shake)
                    let (quake, offset) = self.quake.step(self.environment.weather[12], frame_time.min(0.5));
                    let quake_id = mao_formats::sound::sound_id("SM_Sandy_Env_Quake");
                    match quake {
                        Some(volume) => crate::ambience::play(quake_id, volume),
                        None if self.quake_offset != [0.0; 3] => {
                            crate::audio::with(|m| m.stop_sample(quake_id));
                        }
                        None => {}
                    }
                    self.quake_offset = offset;
                    // (the winds' sounds, FUN_100bff68, at the weather's shares and the gust)
                    for (id, volume) in crate::ambience::winds(&self.environment.weather, self.wind.gust_length()) {
                        crate::ambience::play(id, volume);
                    }
                    self.wind_at = std::time::Instant::now();
                    crate::effects::set_wind(self.wind.now(), self.wind.smooth());
                    let ahead = self.own_at.elapsed().as_secs_f32().min(0.12);
                    let own_feet = [self.own_base[0] + self.own_velocity[0] * ahead, self.own_base[2] + self.own_velocity[2] * ahead, self.own_base[1]];
                    let mut sparks = Vec::new();
                    let mut shells = Vec::new();
                    // (the lightning's flashes and bolts, with the environment's textures 0 and 7 as their sheets)
                    let terrain_ground = self.ground.clone();
                    self.sky.step(frame_time.min(0.5), &|x, z| terrain_ground.as_ref().map(|(t, o)| t.height_at(x, z, t.height_scale) + o));
                    if !self.sky.is_empty() {
                        for (id, name) in [(BOLT_SHEET, "newsun_frame01.png"), (CLOUD_SHEET, "dustcloud.png")] {
                            if !w.renderer.has_sprite(id) && self.tried_sprites.insert(id) {
                                if let Some(texture) = self.textures.sheet(name) {
                                    w.renderer.add_sprite(&w.gpu, id, &texture);
                                }
                            }
                        }
                        shells.extend(self.sky.shells(BOLT_SHEET, CLOUD_SHEET));
                        shells.extend(self.sky.splash_shells());
                    }
                    // (a character's posed mesh: the caster standing nearest a point, and the feet it stands on)
                    let follow = self.follow_shift;
                    let body_at = |point: [f32; 3]| {
                        self.casters
                            .iter()
                            .filter(|c| !c.surface.positions.is_empty())
                            .map(|c| ((c.translation[0] - point[0]).hypot(c.translation[1] - point[1]), c))
                            .filter(|(d, _)| *d < 2.0)
                            .min_by(|a, b| a.0.total_cmp(&b.0))
                            .map(|(_, c)| if c.own { c.shifted(follow) } else { c.clone() })
                    };
                    self.pulses.begin_frame();
                    let mut meshes = Vec::new();
                    let mut rocks = Vec::new();
                    let eye_now = {
                        let e = w.camera.eye();
                        [e.x, e.y, e.z]
                    };
                    let mut shaking = 0.0f32;
                    let terrain = self.ground.clone();
                    let ground = |x: f32, y: f32| terrain.as_ref().map(|(t, o)| t.height_at(x, y, t.height_scale) + o);
                    for (id, own, at, heading, body, auras) in &self.auras {
                        let feet = if *own && self.hud.position.is_some() { own_feet } else { *at };
                        let heading = if *own { self.heading } else { *heading };
                        // (moving forward: over a tenth of a second at least, faster than 0.5 a second along the facing, as
                        // FUN_10112580 asks the vehicle for a speed above 0 forwards)
                        let motion = {
                            let entry = self.motions.entry(*id).or_insert((feet, time, Default::default()));
                            let dt = time - entry.1;
                            if dt >= 0.1 {
                                let d = [feet[0] - entry.0[0], feet[1] - entry.0[1]];
                                let forward = d[0] * heading.sin() + d[1] * heading.cos();
                                let moving = d[0].hypot(d[1]) / dt > 0.5 && forward > 0.0;
                                if moving && !entry.2.moving {
                                    entry.2.started = Some(time);
                                }
                                entry.2.moving = moving;
                                (entry.0, entry.1) = (feet, time);
                            }
                            entry.2
                        };
                        // (the character's posed body, found once: its surface for the shells, its attractors for the meshes on them)
                        let mut posed: Option<Option<crate::blobshadow::Caster>> = None;
                        // (the running nano programs' visuals on the characters: the buffs, while `BuffsFX` is on)
                        let buffs = crate::prefs::effect_category_active(crate::prefs::BUFFS);
                        for (nano, aura) in auras.iter().filter(|_| buffs) {
                            let (period, life, plays) = crate::effects::pulse_timing(aura);
                            if let Some(age) = self.pulses.tick(*id, *nano, period, life, plays, time) {
                                // (its toggles started by the vehicle's motion, the times as the effect's)
                                let motion = crate::effects::Motion { moving: motion.moving, started: motion.started.map(|s| age - (time - s)) };
                                let resolved = crate::effects::resolve_toggles(aura, age, &motion);
                                let aura = &resolved;
                                let c = posed.get_or_insert_with(|| body_at(feet));
                                if let Some(c) = c {
                                    shells.extend(crate::effects::visual_shells(aura, &c.surface, c.translation, age));
                                }
                                let attractors = c.as_ref().map_or(&[][..], |c| c.attractors.as_slice());
                                // (an Atrox's hoverbike circles are its own meshes: FUN_1010d457 with the breed 4)
                                let atrox = body.is_some_and(|b| b.0 == 4);
                                meshes.extend(crate::effects::visual_meshes(aura, crate::effects::Locator::feet(feet, heading), age, &|attach| crate::effects::Locator::attractor(attach, attractors)).into_iter().map(|(m, l, t)| (m, l, t, atrox)));
                                shells.extend(crate::effects::visual_ground_shells(aura, feet, age, &ground));
                                shaking = shaking.max(crate::effects::shake(aura, feet, age, eye_now));
                                rocks.extend(crate::effects::visual_rocks(aura, feet, age).into_iter().map(|(r, at, t)| (r, at, t, id ^ nano.rotate_left(16))));
                                // (the trails, each keeping its samples from frame to frame)
                                let forward = [heading.sin(), heading.cos(), 0.0];
                                for (k, (trail, place, t)) in crate::effects::visual_trails(aura, crate::effects::Locator::feet(feet, heading), age, &|attach| crate::effects::Locator::attractor(attach, attractors)).into_iter().enumerate() {
                                    let (state, ran) = self.trails.entry((*id, *nano, k)).or_default();
                                    state.step(&trail, &place, t, time, forward);
                                    *ran = true;
                                    shells.extend(state.shells(&trail));
                                }
                                sparks.extend(crate::effects::pulse_particles(aura, feet, heading, age, *body));
                            }
                        }
                    }
                    self.pulses.end_frame();
                    self.shake.0 = shaking;
                    // (a trail that did not run this frame has ended)
                    self.trails.retain(|_, (_, ran)| std::mem::take(ran));
                    // (a cast effect emits for a second and lives out its particles' lives)
                    self.bursts.retain(|(began, cast)| began.elapsed().as_secs_f32() < cast.length);
                    for (began, cast) in &self.bursts {
                        sparks.extend(crate::effects::cast_particles(cast, began.elapsed().as_secs_f32()));
                    }
                    // (a lone flare lasts its duration, or a while for one that runs for ever)
                    self.flares.retain(|(began, _, f)| began.elapsed().as_secs_f32() < if f.duration >= 0.0 { f.duration } else { 30.0 });
                    for (began, at, f) in &self.flares {
                        sparks.extend(crate::effects::flare_particles(f, *at, began.elapsed().as_secs_f32(), f32::INFINITY, None));
                    }
                    self.shots.retain(|(began, shot)| began.elapsed().as_secs_f32() < shot.lasts());
                    for (began, shot) in &self.shots {
                        // (the target's posed surface, for the hits thrown from it: the character nearest the shot's end)
                        let body = self
                            .casters
                            .iter()
                            .filter(|c| !c.points.is_empty() && c.points.len() == c.normals.len())
                            .map(|c| ((c.translation[0] - shot.to[0]).hypot(c.translation[1] - shot.to[1]), c))
                            .filter(|(d, _)| *d < 2.0)
                            .min_by(|a, b| a.0.total_cmp(&b.0))
                            .map(|(_, c)| crate::effects::Body { points: &c.points, normals: &c.normals });
                        sparks.extend(shot.particles(began.elapsed().as_secs_f32(), body));
                        // (its streaks, across the line as the last frame's eye sees it)
                        let eye = w.camera.eye();
                        shells.extend(shot.shells(began.elapsed().as_secs_f32(), [eye.x, eye.y, eye.z]));
                        // (a hit that is a shield: the shell over the target from the landing)
                        if let Some(hit) = &shot.hit {
                            let landed = began.elapsed().as_secs_f32() - if shot.from != shot.to { crate::effects::FLIGHT } else { 0.0 };
                            if landed >= 0.0 {
                                let c = body_at(shot.to);
                                if let Some(c) = &c {
                                    shells.extend(crate::effects::visual_shells(hit, &c.surface, c.translation, landed));
                                }
                                let attractors = c.as_ref().map_or(&[][..], |c| c.attractors.as_slice());
                                meshes.extend(crate::effects::visual_meshes(hit, crate::effects::Locator::feet(shot.to, 0.0), landed, &|attach| crate::effects::Locator::attractor(attach, attractors)).into_iter().map(|(m, l, t)| (m, l, t, false)));
                                shells.extend(crate::effects::visual_ground_shells(hit, shot.to, landed, &ground));
                                shaking = shaking.max(crate::effects::shake(hit, shot.to, landed, eye_now));
                                rocks.extend(crate::effects::visual_rocks(hit, shot.to, landed).into_iter().map(|(r, at, t)| (r, at, t, shot.to[0].to_bits() ^ shot.to[1].to_bits().rotate_left(7))));
                            }
                        }
                    }
                    // (the effect meshes as shells: each loaded once by name, its textures made sheets once; seen from the last frame's eye)
                    let eye = {
                        let e = w.camera.eye();
                        [e.x, e.y, e.z]
                    };
                    for (m, at, t, atrox) in &meshes {
                        let Some(name) = m.mesh_file(*atrox) else { continue };
                        let textures = &self.textures;
                        let Some(model) = self.effect_meshes.entry(name).or_insert_with(|| textures.mesh_named(name)).as_ref() else { continue };
                        for part in &model.parts {
                            let Some(&key) = part.material.textures.first() else { continue };
                            if !self.mesh_sheets.contains_key(&key) {
                                let id = 0xa000 + self.mesh_sheets.len() as u16;
                                if let Some(texture) = self.textures.get(key.0, key.1) {
                                    w.renderer.add_sprite(&w.gpu, id, &texture);
                                }
                                self.mesh_sheets.insert(key, id);
                            }
                        }
                        let sheets = &self.mesh_sheets;
                        shells.extend(crate::effects::effect_mesh_shells(m, model, &|k, i| sheets.get(&(k, i)).copied().unwrap_or(mao_render::NO_SPRITE), at, Some(eye), *t));
                    }
                    // (the thrown rocks, their meshes loaded and their textures made sheets as the effect meshes')
                    // (the vulcan rocks' and the lightning's)
                    let mut thrown: Vec<crate::effects::Rock> = rocks.iter().flat_map(|(r, from, t, seed)| crate::effects::vulcan_rocks(r, *from, *t, *seed, &ground)).collect();
                    thrown.extend(self.sky.rocks());
                    {
                        for rock in thrown {
                            let textures = &self.textures;
                            let Some(model) = self.effect_meshes.entry(rock.mesh).or_insert_with(|| textures.mesh_named(rock.mesh)).as_ref() else { continue };
                            for part in &model.parts {
                                let Some(&key) = part.material.textures.first() else { continue };
                                if !self.mesh_sheets.contains_key(&key) {
                                    let id = 0xa000 + self.mesh_sheets.len() as u16;
                                    if let Some(texture) = self.textures.get(key.0, key.1) {
                                        w.renderer.add_sprite(&w.gpu, id, &texture);
                                    }
                                    self.mesh_sheets.insert(key, id);
                                }
                            }
                            let sheets = &self.mesh_sheets;
                            shells.extend(crate::effects::rock_shells(&rock, model, &|k, i| sheets.get(&(k, i)).copied().unwrap_or(mao_render::NO_SPRITE)));
                        }
                    }
                    // (the shells' sheets too)
                    let sheets: Vec<mao_render::Particle> = shells
                        .iter()
                        .map(|s: &mao_render::Shell| mao_render::Particle { sprite: s.sprite, ..mao_render::Particle::dot([0.0; 3], 0.0, [0.0; 4]) })
                        .collect();
                    load_sprites(&mut w.renderer, &w.gpu, &self.textures, &sheets, &mut self.tried_sprites);
                    load_sprites(&mut w.renderer, &w.gpu, &self.textures, &sparks, &mut self.tried_sprites);
                    // (the effect scripts' sky: its backdrops behind the world, its other visuals among the shells, its ground
                    // light the world's sun)
                    if let Some(frame) = &self.fxs_frame {
                        let e = w.camera.eye();
                        let t = w.camera.target;
                        let (eye, forward) = ([e.x, e.z, e.y], {
                            let f = (t - e).normalize_or_zero();
                            [f.x, f.z, f.y]
                        });
                        let mut caches = SkyCaches { meshes: &mut self.script_meshes, mesh_sheets: &mut self.mesh_sheets, named: &mut self.named_sheets };
                        let (backdrops, others) = sky_draws(frame, eye, forward, self.settings.view_distance as f32, &self.textures, &mut w.renderer, &w.gpu, &mut caches);
                        w.scene.backdrops = backdrops;
                        shells.extend(others);
                        w.scene.sun = frame.lights.ground.map(|l| mao_render::glam::Vec3::new(-l.direction[0], -l.direction[2], -l.direction[1]));
                    }
                    w.scene.particles = sparks;
                    w.scene.shells = shells;
                }
                prof_t = crate::prof::lap("update", prof_t);
                let frame = match w.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                    _ => {
                        w.surface.configure(&w.gpu.device, &w.config);
                        return;
                    }
                };
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                prof_t = crate::prof::lap("acquire", prof_t);
                let size = (w.config.width, w.config.height);
                if self.live.is_some() {
                    w.scene.fog = fog_for(self.settings.view_distance);
                    // (an interior has no ground: it is seen against the dark, not the daylight sky)
                    let interior = self.ground.is_none() && self.behind_set;
                    w.scene.sky = if interior { mao_render::DARK_SKY } else { mao_render::DAY_SKY };
                    // The day's light and the area's environment (`Environment_t`'s per-frame work, see `mao_formats::areas`): an
                    // interior is lit as it is; outside, the area's ambient is kept against the sky light and its fog, when it has
                    // one, is lit by the fog light (`AddFogLightIntensity`: the day's intensity, at least a half).
                    // (the effect scripts' lighting and, outdoors, their haze: the atmosphere's fog colour)
                    w.scene.lighting = self.fxs_frame.as_ref().and_then(|f| f.lights.ground.map(|g| (f.lights.ambient, g.colour)));
                    if let Some(fog) = self.fxs_frame.as_ref().and_then(|f| f.fog).filter(|_| !interior) {
                        w.scene.sky.0 = fog;
                    }
                    let e = self.environment;
                    w.scene.daylight = if interior { 1.0 } else { self.daylight * e.sky };
                    w.scene.ambient = e.ambient;
                    w.scene.area_fog = if interior { None } else { e.fog };
                    if self.keys.char_distance != self.settings.char_distance || self.keys.auto_loot != self.settings.auto_loot {
                        self.keys.char_distance = self.settings.char_distance;
                        self.keys.auto_loot = self.settings.auto_loot;
                        if let Some(live) = &self.live {
                            let _ = live.input.send(self.keys);
                        }
                    }
                }
                w.renderer.render_scale = self.settings.render_scale;
                w.renderer.sharpness = self.settings.sharpness;
                // The camera stays above the ground (else the ground's back is culled and the sky shows through it).
                let mut camera = w.camera;
                // (a ground shake moves the eye by a random vector of its size each frame: `n3Camera_t` +0x1d4)
                // (the environment's quake shakes it too: Y up made Z up)
                camera.target += mao_render::glam::Vec3::new(self.quake_offset[0], self.quake_offset[2], self.quake_offset[1]);
                if self.shake.0 > 0.0 {
                    self.shake.1 = self.shake.1.wrapping_add(1);
                    let r = |salt: u32| {
                        let mut x = self.shake.1.wrapping_mul(0x9e37_79b1) ^ salt.wrapping_mul(0x85eb_ca6b);
                        x ^= x >> 15;
                        x = x.wrapping_mul(0x2c1b_3c6d);
                        x ^= x >> 12;
                        (x & 0xffff) as f32 / 65535.0 * 2.0 - 1.0
                    };
                    camera.target += mao_render::glam::Vec3::new(r(1), r(2), r(3)) * self.shake.0;
                }
                if let Some((terrain, offset)) = &self.ground {
                    for _ in 0..24 {
                        let eye = camera.eye();
                        if eye.z >= terrain.height_at(eye.x, eye.y, terrain.height_scale) + offset + 0.6 || camera.distance < 0.8 {
                            break;
                        }
                        camera.distance *= 0.88;
                    }
                }
                // The characters' blob shadows (the original's only cast shadows), laid on the ground under each.
                {
                    let eye = camera.eye();
                    let light = -mao_render::sun();
                    let interior = self.ground.is_none();
                    let height = self.ground.as_ref().map(|(t, o)| (t.clone(), *o));
                    let height_at = |x: f32, y: f32| height.as_ref().map_or(0.0, |(t, o)| t.height_at(x, y, t.height_scale) + o);
                    let mut blobs = Vec::new();
                    // (the ground shadows while `Shadows` is on: the option panel's Effects page)
                    for caster in self.casters.iter().filter(|_| crate::prefs::on("Shadows")) {
                        let shifted;
                        let caster = if caster.own {
                            shifted = caster.shifted(self.follow_shift);
                            &shifted
                        } else {
                            caster
                        };
                        let feet = caster.translation;
                        let (ground, alpha) = if interior {
                            (crate::blobshadow::Ground { point: feet, normal: [0.0, 0.0, 1.0], height: None }, crate::blobshadow::ALPHA_DUNGEON)
                        } else {
                            let h = height_at(feet[0], feet[1]);
                            if (feet[2] - h).abs() > 0.2 {
                                // standing on something other than the ground: the plane at the feet, nothing laid on the heightmap
                                (crate::blobshadow::Ground { point: feet, normal: [0.0, 0.0, 1.0], height: None }, crate::blobshadow::ALPHA_OUTDOORS)
                            } else {
                                let (dx, dy) = ((height_at(feet[0] + 0.5, feet[1]) - height_at(feet[0] - 0.5, feet[1])) / 1.0, (height_at(feet[0], feet[1] + 0.5) - height_at(feet[0], feet[1] - 0.5)) / 1.0);
                                let len = (dx * dx + dy * dy + 1.0).sqrt();
                                (crate::blobshadow::Ground { point: [feet[0], feet[1], h], normal: [-dx / len, -dy / len, 1.0 / len], height: Some(&height_at) }, crate::blobshadow::ALPHA_OUTDOORS)
                            }
                        };
                        let light = if interior { [0.0, 0.0, -1.0] } else { [light.x, light.y, light.z] };
                        for (p, c) in crate::blobshadow::blob(caster, light, [eye.x, eye.y, eye.z], &ground, alpha) {
                            blobs.push(mao_render::BlobVertex { position: p, colour: c });
                        }
                    }
                    w.scene.blobs = blobs;
                }
                if std::mem::take(&mut self.screenshot) {
                    // (the window as it shows, with its open windows and the web page: `screenshot.rs`; else the scene and
                    // the HUD rendered offscreen)
                    #[cfg(windows)]
                    let shown = crate::screenshot::capture(&w.window);
                    #[cfg(not(windows))]
                    let shown = None;
                    let (width, height, pixels) = shown.unwrap_or_else(|| {
                        let (width, height) = size;
                        let (aspect, radius, shot_camera) = (width as f32 / height.max(1) as f32, w.scene.radius, camera);
                        let format = w.config.format;
                        let pixels = w.renderer.render_to_image_with(&w.gpu, &w.scene, &camera, width, height, &mut |image| {
                            let project = |p: mao_render::glam::Vec3| shot_camera.project(p, aspect, radius);
                            let mut settings = Settings::load();
                            crate::ui::paint_headless(&w.gpu, image, format, (width, height), &mut settings, &self.hud, &project, false);
                        });
                        (width, height, pixels)
                    });
                    let name = crate::screenshot::local_stamp().unwrap_or_else(|| {
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
                        let (day, time) = crate::chatlog::stamp(now);
                        format!("{}-{}", day.trim_end_matches(".txt"), time.replace(':', ""))
                    });
                    let dir = Settings::path().with_file_name("screenshots");
                    let path = dir.join(format!("{name}.png"));
                    let saved = std::fs::create_dir_all(&dir).is_ok() && image::save_buffer(&path, &pixels, width, height, image::ColorType::Rgba8).is_ok();
                    self.hud.say(if saved { format!("Screenshot saved: {}", path.display()) } else { "The screenshot could not be saved".to_string() });
                }
                prof_t = crate::prof::lap("scene", prof_t);
                w.renderer.draw(&w.gpu, &w.scene, &camera, &view, size);
                prof_t = crate::prof::lap("draw", prof_t);
                let stats = w.renderer.stats;
                for (name, took) in stats.times {
                    if !name.is_empty() {
                        crate::prof::add(name, took);
                    }
                }
                self.hud.draw_info = format!("{} of {} parts, {} in shadow", stats.drawn, stats.parts, stats.shadow_drawn);
                if let Some(ui) = w.ui.as_mut() {
                    let (camera, radius) = (w.camera, w.scene.radius);
                    let aspect = size.0 as f32 / size.1.max(1) as f32;
                    let project = |p: mao_render::glam::Vec3| camera.project(p, aspect, radius);
                    self.hud.camera_yaw = w.camera.yaw;
                    // (a name behind a wall is hidden, as the original's tag sprite in the world is)
                    if let Some(z) = self.zones.as_ref() {
                        let e = w.camera.eye();
                        let eye = [e.x, e.z, e.y];
                        for t in &mut self.hud.nameplates {
                            t.hidden = z.line(eye, [t.at[0], t.at[2], t.at[1]]).is_some();
                        }
                    }
                    let (frame, mut requests) = ui.run(&w.window, &mut self.settings, &self.hud, &project);
                    prof_t = crate::prof::lap("ui", prof_t);
                    // (the web page follows its window: opened when it opens, moved with it, gone when it closes)
                    #[cfg(windows)]
                    match ui.browser_bounds() {
                        Some(bounds) => {
                            // (a page for another request replaces it, the old one let go only once the new one is up so that
                            // WebView2's browser, and the site's session, stay)
                            let mut retired = None;
                            if ui.browser.fresh {
                                ui.browser.fresh = false;
                                match w.page.as_mut() {
                                    Some(page) if ui.browser.request.as_ref() == Some(&page.request) => page.reopen(),
                                    _ => retired = w.page.take(),
                                }
                            }
                            if w.page.is_none() {
                                if let Some(request) = ui.browser.request.clone() {
                                    let data = crate::settings::Settings::path().parent().map_or_else(|| std::path::PathBuf::from("Browser"), |d| d.join("Browser"));
                                    match crate::browser::Page::open(&w.window, &request, bounds, data, ui.browser.event_sink()) {
                                        Ok(page) => w.page = Some(page),
                                        Err(e) => {
                                            ui.browser.open = false;
                                            self.hud.say(format!("The web page could not be opened: {e}"));
                                        }
                                    }
                                }
                            }
                            if let Some(page) = w.page.as_mut() {
                                page.place(bounds);
                                for what in std::mem::take(&mut ui.browser.navigate) {
                                    page.navigate(what);
                                }
                                ui.browser.status = page.url();
                            }
                            drop(retired);
                        }
                        None => {
                            if let Some(page) = w.page.as_mut() {
                                page.hide();
                            }
                        }
                    }
                    for event in ui.browser.take_events() {
                        match event {
                            // (SlotOpenMarketSendWindow: only with the Alien Invasion expansion, stat 389's bit 8)
                            crate::browser::Event::OpenMarketSendWindow => {
                                if self.hud.stats.get(&389).copied().unwrap_or(0) & 8 != 0 {
                                    ui.market_send.show_window();
                                } else {
                                    self.hud.say(crate::feedback::plain("Feedback_YouMustHaveAlienInvasion"));
                                }
                            }
                        }
                    }
                    ui.pointer(event_loop, &w.window, frame.cursor);
                    for out in requests.chat.drain(..) {
                        match out {
                            // (these two are for the client itself)
                            crate::chatwindow::Out::Camp => self.keys.camp = self.keys.camp.wrapping_add(1),
                            crate::chatwindow::Out::Sit => self.keys.commands[5] = self.keys.commands[5].wrapping_add(1),
                            crate::chatwindow::Out::Pet(command) => self.keys.pet = (self.keys.pet.0.wrapping_add(1), command),
                            crate::chatwindow::Out::Quit => event_loop.exit(),
                            crate::chatwindow::Out::Window { name, open } => match ui.set_window(&name, open) {
                                true => {}
                                false => self.hud.say(format!("No window called {name:?} (inventory, stats, skills, ncu, friends, programs, perks, help, map, actions, missions, mail, chat)")),
                            },
                            crate::chatwindow::Out::Waypoint { x, z } => {
                                self.keys.goto = (self.keys.goto.0.wrapping_add(1), [x, z]);
                                if let Some(live) = &self.live {
                                    let _ = live.input.send(self.keys);
                                }
                            }
                            crate::chatwindow::Out::SelectSelf => self.keys.select_self = self.keys.select_self.wrapping_add(1),
                            crate::chatwindow::Out::SetOption { name, value } => {
                                let line = self.settings.set_option(&name, &value);
                                ui.chat.push(crate::chatwindow::Line::new(crate::chatwindow::Kind::System, line));
                                self.settings.save();
                            }
                            crate::chatwindow::Out::Inspect => self.keys.inspect = self.keys.inspect.wrapping_add(1),
                            crate::chatwindow::Out::Follow => self.keys.follow = self.keys.follow.wrapping_add(1),
                            crate::chatwindow::Out::ViewDist(d) => {
                                self.settings.view_distance = d as f32;
                                self.settings.save();
                            }
                            crate::chatwindow::Out::CharDist(d) => {
                                self.settings.char_distance = d as f32;
                                self.settings.save();
                            }
                            crate::chatwindow::Out::Vicinity { kind, text } => crate::net::speak(kind, text, Vec::new()),
                            crate::chatwindow::Out::Social { id } => self.keys.social = (self.keys.social.0.wrapping_add(1), id),
                            other => crate::net::send_chat(other),
                        }
                    }
                    if let Some(deposit) = requests.market_send.take() {
                        self.keys.market = (self.keys.market.0.wrapping_add(1), deposit);
                    }
                    for command in std::mem::take(&mut requests.mission_log) {
                        if let Some(live) = &self.live {
                            let _ = live.mission_log.send(command);
                        }
                    }
                    if let Some(waypoint) = requests.waypoint.take() {
                        self.hud.waypoint = Some(waypoint);
                    }
                    for line in std::mem::take(&mut requests.lines) {
                        self.hud.say(line);
                    }
                    for name in std::mem::take(&mut requests.feedback) {
                        self.hud.say(crate::feedback::plain(name));
                    }
                    for command in std::mem::take(&mut requests.npc_chat) {
                        if let Some(live) = &self.live {
                            let _ = live.npc_chat.send(command);
                        }
                    }
                    if let Some(command) = requests.shop.take() {
                        if let Some(live) = &self.live {
                            let _ = live.shop.send(command);
                        }
                    }
                    if let Some(command) = requests.mail.take() {
                        if let Some(live) = &self.live {
                            let _ = live.mail.send(command);
                        }
                    }
                    if let Some(command) = requests.mission {
                        self.keys.mission = (self.keys.mission.0.wrapping_add(1), command);
                        if let Some(live) = &self.live {
                            let _ = live.input.send(self.keys);
                        }
                    }
                    if let Some(v) = requests.aggdef {
                        self.keys.aggdef = (self.keys.aggdef.0.wrapping_add(1), v);
                        self.hud.stats.insert(51, i64::from(v));
                    }
                    if let Some(which) = requests.target {
                        match which {
                            0 => self.keys.select_self = self.keys.select_self.wrapping_add(1),
                            1 => self.keys.commands[0] = self.keys.commands[0].wrapping_add(1),
                            // (the camera modes: first person, third person, and which third person)
                            2 => self.steering.set_mode(crate::camera::Mode::First),
                            3 => {
                                if self.steering.is_first_person() {
                                    self.steering.toggle_view();
                                }
                            }
                            _ => {
                                self.settings.camera_mode = ["lock", "rubber", "trail"][(which as usize - 4).min(2)].to_string();
                                self.settings.save();
                                self.steering.set_mode(camera_mode(&self.settings.camera_mode));
                                self.hud.say(format!("Camera: third person {}.", match self.settings.camera_mode.as_str() { "lock" => "locked", "trail" => "trailing (free)", _ => "rubber" }));
                            }
                        }
                        if let Some(live) = &self.live {
                            let _ = live.input.send(self.keys);
                        }
                    }
                    if let Some(g) = requests.goto {
                        self.keys.goto = (self.keys.goto.0.wrapping_add(1), g);
                        if let Some(live) = &self.live {
                            let _ = live.input.send(self.keys);
                        }
                    }
                    if let Some(slot) = requests.press {
                        press_slot(ui, slot, &mut self.keys, &mut self.hud);
                    }
                    if let Some(take) = requests.loot_take.take() {
                        match take {
                            crate::loot::Take::One { handle, place } => {
                                self.keys.take = (self.keys.take.0.wrapping_add(1), handle, place);
                                if let Some(loot) = self.hud.loot.as_mut() {
                                    loot.items.retain(|i| i.slot != place);
                                }
                            }
                            crate::loot::Take::Put { identity, from } => {
                                self.keys.put = (self.keys.put.0.wrapping_add(1), identity.0, identity.1, from);
                            }
                            crate::loot::Take::All { handle } => {
                                self.keys.take = (self.keys.take.0.wrapping_add(1), handle, u32::MAX);
                                if let Some(loot) = self.hud.loot.as_mut() {
                                    loot.items.clear();
                                }
                            }
                        }
                    }
                    if let Some((rank, train)) = requests.train_perk.take() {
                        self.keys.train_perk = (self.keys.train_perk.0.wrapping_add(1), rank, train);
                    }
                    if let Some(nano) = requests.cancel_nano.take() {
                        self.keys.cancel = (self.keys.cancel.0.wrapping_add(1), nano);
                    }
                    // (the info view's item pages: N3's item descriptions, built here from the database)
                    for url in ui.info.take_requests() {
                        if let Some(page) = crate::gui::iteminfo::page(self.textures.db(), &url, &self.hud.stats) {
                            ui.info.show_page(&url, &page);
                        }
                    }
                    // (an item let go over something in the world is used on it: `N3Msg_UseItemOnItem`)
                    if let Some((item, at)) = requests.use_on.take() {
                        let size = w.window.inner_size();
                        if let Some((kind, id)) = pick(&self.hud, camera, radius, (size.width, size.height), at) {
                            self.keys.use_on = (self.keys.use_on.0.wrapping_add(1), item, (kind, id));
                            if let Some(live) = &self.live {
                                let _ = live.input.send(self.keys);
                            }
                        }
                    }
                    for place in requests.item_uses.drain(..) {
                        self.keys.fire = (self.keys.fire.0.wrapping_add(1), place);
                    }
                    for (from, onto) in requests.item_combines.drain(..) {
                        self.keys.combine = (self.keys.combine.0.wrapping_add(1), from, onto);
                    }
                    for place in requests.item_deletes.drain(..) {
                        self.keys.delete = (self.keys.delete.0.wrapping_add(1), place);
                    }
                    for (from, to) in requests.item_moves.drain(..) {
                        self.keys.mv = (self.keys.mv.0.wrapping_add(1), from, to);
                    }
                    if let Some(action) = requests.action.take() {
                        fire_action(&action, &mut self.keys, &mut self.hud);
                        if let Some(live) = &self.live {
                            let _ = live.input.send(self.keys);
                        }
                    }
                    if requests.hotbars_changed {
                        crate::hotbar::save(&ui.hotbars);
                    }
                    prof_t = crate::prof::lap("ui-requests", prof_t);
                    ui.paint(&w.gpu, &view, size, frame);
                    prof_t = crate::prof::lap("paint", prof_t);
                    if let Some(theme) = requests.theme {
                        ui.set_theme(&theme, &self.settings.aou_mod);
                        self.settings.theme = theme;
                        ui.set_skin(&self.settings);
                        self.settings.save();
                    }
                    if let Some(dir) = requests.gui_dir {
                        self.settings.gui_dir = dir;
                        ui.set_skin(&self.settings);
                        self.settings.save();
                    }
                    if let Some(mode) = requests.mode {
                        self.settings.mode = mode;
                        apply_mode(&w.window, mode);
                        self.settings.save();
                    }
                    if let Some((width, height)) = requests.size {
                        let _ = w.window.request_inner_size(PhysicalSize::new(width, height));
                        self.settings.save();
                    }
                    if requests.save {
                        self.settings.save();
                    }
                    if requests.quit {
                        event_loop.exit();
                    }
                }
                w.window.pre_present_notify();
                w.gpu.queue.present(frame);
                if w.window.is_visible() == Some(false) {
                    w.window.set_visible(true);
                }
                crate::prof::lap("present", prof_t);
                crate::prof::frame();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod camera_tests {
    use super::*;

    #[test]
    fn a_line_meets_the_near_side_of_a_cylinder() {
        use mao_render::glam::Vec3;
        // from 10 m west at chest height, looking east at a 0.5 m cylinder 2 m tall
        let t = ray_cylinder(Vec3::new(-10.0, 0.0, 1.0), Vec3::X, [0.0, 0.0, 0.0], 0.5, 2.0).unwrap();
        assert!((t - 9.5).abs() < 1e-4);
        // over its top, or behind the start: missed
        assert!(ray_cylinder(Vec3::new(-10.0, 0.0, 3.0), Vec3::X, [0.0, 0.0, 0.0], 0.5, 2.0).is_none());
        assert!(ray_cylinder(Vec3::new(10.0, 0.0, 1.0), Vec3::X, [0.0, 0.0, 0.0], 0.5, 2.0).is_none());
        // looking down on it: its top
        let t = ray_cylinder(Vec3::new(0.1, 0.0, 10.0), -Vec3::Z, [0.0, 0.0, 0.0], 0.5, 2.0).unwrap();
        assert!((t - 8.0).abs() < 1e-4);
    }

    #[test]
    fn the_camera_sits_behind_the_character() {
        // A character facing +Z (heading 0) is looked at from -Z: the camera's offset from it points to -Y of the Z-up world.
        let yaw = yaw_behind(0.0);
        assert!((yaw.cos()).abs() < 1e-5 && (yaw.sin() + 1.0).abs() < 1e-5);
        // Facing +X (heading pi/2): the camera is at -X.
        let yaw = yaw_behind(std::f32::consts::FRAC_PI_2);
        assert!((yaw.cos() + 1.0).abs() < 1e-5 && yaw.sin().abs() < 1e-5);
        // Steering with the camera and reading the heading back round-trips.
        for h in [-2.5f32, -0.7, 0.0, 1.3, 3.0] {
            let back = heading_from_yaw(yaw_behind(h));
            assert!(((back - h + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI).abs() < 1e-4, "{h} -> {back}");
        }
    }
}

/// The viewer's caches for the effect scripts' sky: their meshes by name, the meshes' texture sheets, the named sheets.
struct SkyCaches<'a> {
    meshes: &'a mut std::collections::HashMap<String, Option<Model>>,
    mesh_sheets: &'a mut std::collections::HashMap<(u32, u32), u16>,
    named: &'a mut std::collections::HashMap<String, u16>,
}

/// The effect scripts' draws made triangles (`fxs::visuals`): each placed at the camera (`place`, the view distance 1000),
/// a mesh's parts through their own frames then the draw's, lit as D3D lights them when the draw is (the blob's material,
/// else the part's), times the draw's factor; the server's frame (Y up) made the world's (Z up). The backdrops in order,
/// the others as shells.
fn sky_draws(frame: &crate::fxs::visuals::Frame, eye: [f32; 3], forward: [f32; 3], distance: f32, textures: &crate::Textures, renderer: &mut mao_render::Renderer, gpu: &mao_render::Gpu, c: &mut SkyCaches) -> (Vec<mao_render::Backdrop>, Vec<mao_render::Shell>) {
    use crate::fxs::visuals::{self, Blend, Shape};
    let world = |p: [f32; 3]| [p[0], p[2], p[1]];
    let named_sheet = |named: &mut std::collections::HashMap<String, u16>, renderer: &mut mao_render::Renderer, name: &str| -> u16 {
        if let Some(&id) = named.get(name) {
            return id;
        }
        let id = 0xb200 + named.len() as u16;
        if let Some(t) = textures.sheet(name) {
            renderer.add_sprite(gpu, id, &t);
        }
        named.insert(name.to_string(), id);
        id
    };
    let (mut backdrops, mut shells) = (Vec::new(), Vec::new());
    for d in &frame.draws {
        let m = visuals::place(d, eye, forward, distance);
        let mut parts: Vec<(u16, Vec<mao_render::ShellVertex>)> = Vec::new();
        let scaled = |c: [f32; 4]| [c[0] * d.factor[0], c[1] * d.factor[1], c[2] * d.factor[2], c[3] * d.factor[3]];
        match &d.shape {
            Shape::Triangles(t) => {
                let sprite = d.texture.as_deref().map_or(mao_render::NO_SPRITE, |n| named_sheet(c.named, renderer, n));
                parts.push((sprite, t.iter().map(|v| mao_render::ShellVertex { position: world(visuals::transform(&m, v.pos)), uv: visuals::texture_uv(d, v.uv), colour: scaled(v.colour) }).collect()));
            }
            Shape::Mesh(name) => {
                let Some(model) = c.meshes.entry(name.clone()).or_insert_with(|| textures.mesh_named(name)).as_ref() else { continue };
                let placements: Vec<(usize, mao_formats::model::Matrix)> =
                    if model.instances.is_empty() { (0..model.parts.len()).map(|i| (i, mao_formats::model::IDENTITY)).collect() } else { model.instances.clone() };
                for (index, instance) in placements {
                    let Some(part) = model.parts.get(index) else { continue };
                    // (the mesh pool's data: the triangle mesh as stored, its frames not applied, FUN_10061505; its z negated, as
                    // the left-handed device takes it — with quaternion `[ROT]` composed the other way, see `fxs`, the only
                    // reading that lays the horizon's dish level. `MAO_SKY_VARIANT` (shown, zneg) compares others.)
                    let variant = std::env::var("MAO_SKY_VARIANT").unwrap_or_else(|_| "zneg".into());
                    let own = if variant.contains("shown") { mao_formats::model::multiply(&part.world, &instance) } else { instance };
                    let zneg = variant.contains("zneg");
                    let sprite = match part.material.textures.first() {
                        Some(&key) => match c.mesh_sheets.get(&key) {
                            Some(&id) => id,
                            None => {
                                let id = 0xa000 + c.mesh_sheets.len() as u16;
                                if let Some(t) = textures.get(key.0, key.1) {
                                    renderer.add_sprite(gpu, id, &t);
                                }
                                c.mesh_sheets.insert(key, id);
                                id
                            }
                        },
                        None => d.texture.as_deref().map_or(mao_render::NO_SPRITE, |n| named_sheet(c.named, renderer, n)),
                    };
                    // (the pool makes the mesh's material plain white with nothing else; a blob's material over it)
                    let material = match d.material {
                        Some(b) => ([b[0], b[1], b[2], b[3]], [b[4], b[5], b[6]], [b[12], b[13], b[14]]),
                        None => ([1.0, 1.0, 1.0, 1.0], [0.0; 3], [0.0; 3]),
                    };
                    let vertices = part
                        .indices
                        .iter()
                        .filter_map(|&i| part.vertices.get(i as usize))
                        .map(|v| {
                            let flip = |p: [f32; 3]| if zneg { [p[0], p[1], -p[2]] } else { p };
                            let p = visuals::transform(&m, visuals::transform(&own, flip(v.position)));
                            let n = visuals::turn(&m, visuals::turn(&own, flip(v.normal)));
                            let colour = if d.light.is_some() { visuals::lit(d, n, material) } else { [1.0, 1.0, 1.0, material.0[3].max(0.0)] };
                            mao_render::ShellVertex { position: world(p), uv: visuals::texture_uv(d, v.uv), colour: scaled(colour) }
                        })
                        .collect();
                    parts.push((sprite, vertices));
                }
            }
        }
        if std::env::var_os("MAO_SKY_DEBUG").is_some() {
            let el: Vec<f32> = parts.iter().flat_map(|(_, v)| v.iter()).map(|v| {
                let r = [v.position[0] - eye[0], v.position[1] - eye[2], v.position[2] - eye[1]];
                (r[2] / (r[0] * r[0] + r[1] * r[1]).sqrt()).atan().to_degrees()
            }).collect();
            let (lo, hi) = el.iter().fold((f32::MAX, f32::MIN), |(a, b), &x| (a.min(x), b.max(x)));
            eprintln!("sky draw {} sorting {}: elevations {lo:.1}..{hi:.1} matrix {:?}", d.kind, d.sorting, &m[..]);
        }
        for (sprite, vertices) in parts {
            if d.backdrop {
                let blend = match d.blend {
                    Blend::Opaque => mao_render::BackdropBlend::Opaque,
                    Blend::Alpha => mao_render::BackdropBlend::Alpha,
                    Blend::Add => mao_render::BackdropBlend::Add,
                };
                backdrops.push(mao_render::Backdrop { sprite, blend, vertices });
            } else {
                shells.push(mao_render::Shell { sprite, additive: d.blend == Blend::Add, vertices });
            }
        }
    }
    (backdrops, shells)
}
