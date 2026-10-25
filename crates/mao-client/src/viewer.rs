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
    /// The heading the character should face (the camera's, while the right mouse button steers).
    pub face: Option<f32>,
    /// A hotbar item to use: a counter that steps with each, and the inventory slot.
    pub fire: (u8, u32),
    /// A special action of the Action window to do (a counter that steps with each, and its id), and a social
    /// action or dance (the same).
    pub special: (u8, u32),
    pub social: (u8, u32),
    /// A place to walk to (a counter that steps with each, and the world's x, z).
    pub goto: (u8, [f32; 2]),
    /// A mission window command: a counter that steps with each, and the command.
    pub mission: (u8, crate::mission::Command),
    /// Something clicked in the world: a counter that steps with each click, its id, and whether it was a double click (use it).
    pub pick: (u8, u32, bool),
    /// A nano program to cast: a counter that steps with each, and the program's id.
    pub cast: (u8, u32),
    /// An item to move: a counter that steps with each, the place it is in and the place it goes to.
    pub mv: (u8, u32, u32),
    /// An item to destroy: a counter that steps with each and the item's place.
    pub delete: (u8, u32),
    /// Follow the target (a counter that steps with each `/follow`).
    pub follow: u8,
    /// Say what is known of the target (a counter that steps with each `/inspect`).
    pub inspect: u8,
    /// Target the character itself (a counter that steps with each `/selectself`).
    pub select_self: u8,
    /// A running nano to cancel: a counter that steps with each and the nano's id.
    pub cancel: (u8, u32),
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
    /// New cells of the map (fog of war), and the people about (for its dots).
    pub map: Option<crate::automap::MapPatch>,
    /// The character's name and stats, when they change.
    pub stats: Option<(String, std::collections::BTreeMap<u32, i64>, std::collections::BTreeMap<u32, crate::derived::Breakdown>)>,
    pub map_dots: Vec<([f32; 2], crate::automap::DotKind)>,
    /// The names to float over the characters and creatures near (Z-up position over the head).
    pub nameplates: Vec<(crate::automap::DotKind, [f32; 3], String)>,
    /// Numbers to float over the fight: Z-up position, text and kind (see `ui::Floater`).
    pub floaters: Vec<([f32; 3], String, u8)>,
    /// Where a nano program landed (Z-up) and the glow's colour.
    pub glows: Vec<([f32; 3], [u8; 3])>,
    /// Who is under running nano programs: whether it is the character itself, the feet (Z-up) and the aura of each program. The whole
    /// list when it is given (an update without one leaves the last in place).
    pub auras: Option<Vec<(bool, [f32; 3], Vec<mao_formats::gfx::Visual>)>>,
    /// Cast effects to start: where (Z up) and the emitters of the program's cast template.
    pub bursts: Option<Vec<([f32; 3], Vec<mao_formats::gfx::Emitter>)>>,
    /// Spells in flight and landing: the tracer's and the hit effect's spirals.
    pub shots: Option<Vec<crate::effects::Shot>>,
    /// What the server said of the specials' recharge (skill stat, seconds; 0 = ready).
    pub special_cooldowns: Vec<(u32, f32)>,
    /// The ground (and how far the server's heights sit above it), to keep the camera above it.
    pub ground: Option<(std::sync::Arc<mao_formats::terrain::Terrain>, f32)>,
    pub own: Option<[f32; 3]>,
    /// The playfield's name, when it changes.
    pub location: Option<String>,
    /// The labels over the world (Z-up positions), when they change.
    pub labels: Option<Vec<([f32; 3], String)>>,
    /// What can be clicked in the world: ids and Z-up positions.
    pub pickables: Option<Vec<(u32, [f32; 3])>>,
    /// Messages for the log.
    pub messages: Vec<String>,
    /// The current target (name and distance), for the target frame.
    pub target: Option<String>,
    /// The character's items, when they arrive.
    pub gear: Option<Vec<crate::gear::GearItem>>,
    /// New lines for the chat window.
    pub chat_lines: Vec<crate::chatwindow::Line>,
    /// The friends list, when it changes.
    pub friends: Option<Vec<crate::friends::Friend>>,
    /// The character's nano programs, when they arrive.
    pub programs: Option<Vec<crate::nanoprograms::Program>>,
    /// The character's perks, when they arrive.
    pub perk_list: Option<Vec<crate::perks::PerkInfo>>,
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
            scene.particles = crate::effects::visual_particles(&visual, feet, time, crate::effects::Reach::Body);
            eprintln!("aura: {} sparks", scene.particles.len());
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
    // `MAO_PLAYER=x,y,z` (world, Z up) puts a player in the scene: plants near it bend away.
    if let Some(v) = std::env::var("MAO_PLAYER").ok().map(|t| t.split(',').filter_map(|n| n.trim().parse::<f32>().ok()).collect::<Vec<_>>()) {
        if let [x, y, z] = v[..] {
            scene.player = Some(mao_render::glam::Vec3::new(x, y, z));
        }
    }
    let mut camera = Camera::fit(&scene);
    camera_override(&mut camera);
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
    last_follow: std::time::Instant,
    /// The nano programs running on characters (see `Update::auras`) and the clock their sparks run on.
    auras: Vec<(bool, [f32; 3], Vec<mao_formats::gfx::Visual>)>,
    fx_clock: std::time::Instant,
    /// The camera's distance before it was pulled to the character's head (F8), when it is.
    home_distance: Option<f32>,
    /// The cast effects running: when each began, where, and its emitters.
    bursts: Vec<(std::time::Instant, [f32; 3], Vec<mao_formats::gfx::Emitter>)>,
    shots: Vec<(std::time::Instant, crate::effects::Shot)>,
    /// The effect sprite sheets already asked for.
    tried_sprites: std::collections::HashSet<u16>,
    /// The character's position and speed at the last update, and when it arrived (for the prediction).
    own_base: [f32; 3],
    own_velocity: [f32; 3],
    own_at: std::time::Instant,
    ground: Option<(std::sync::Arc<mao_formats::terrain::Terrain>, f32)>,
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
    let mut app = App { model, textures, live, state: None, dragging: false, last: (0.0, 0.0), buttons: [false; 2], heading: 0.0, behind_set: false, last_follow: std::time::Instant::now(), auras: Vec::new(), fx_clock: std::time::Instant::now(), home_distance: None, bursts: Vec::new(), shots: Vec::new(), tried_sprites: Default::default(), own_base: [0.0; 3], own_velocity: [0.0; 3], own_at: std::time::Instant::now(), ground: None, keys: Input::default(), last_sent: Input::default(), settings, modifiers: Default::default(), held: Default::default(), home: None, preferred: None, hud: Hud::default(), moved: 0.0, last_click: None, screenshot: false, autorun: false };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())
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

fn heading_from_yaw(yaw: f32) -> f32 {
    (-yaw.cos()).atan2(-yaw.sin())
}

fn yaw_behind(heading: f32) -> f32 {
    heading_from_yaw(heading)
}

impl ApplicationHandler for App {
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let DeviceEvent::MouseMotion { delta: (dx, dy) } = event else { return };
        if self.live.is_none() || self.buttons == [false; 2] {
            return;
        }
        self.moved += dx.abs() as f32 + dy.abs() as f32;
        let Some(w) = self.state.as_mut() else { return };
        w.camera.yaw -= dx as f32 * 0.005;
        w.camera.pitch = (w.camera.pitch + dy as f32 * 0.005).clamp(-0.6, 1.45);
        if self.buttons[1] {
            self.keys.face = Some(heading_from_yaw(w.camera.yaw));
            if let Some(live) = &self.live {
                let _ = live.input.send(self.keys);
            }
        }
        w.window.request_redraw();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
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
        let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("Modern Anarchy Online").with_inner_size(PhysicalSize::new(self.settings.width, self.settings.height))).expect("a window"));
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
        self.state = Some(View { window, surface, config, gpu, renderer, scene, camera, ui });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(w) = self.state.as_mut() else { return };
        if let Some(ui) = w.ui.as_mut() {
            // The key the options window waits for goes to it, not to the interface.
            let capturing = ui.rebinding.is_some() && matches!(event, WindowEvent::KeyboardInput { .. });
            if !capturing && !matches!(event, WindowEvent::RedrawRequested | WindowEvent::ModifiersChanged(_) | WindowEvent::CloseRequested | WindowEvent::Resized(_)) && ui.event(&w.window, &event) {
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
            // Playing: the left button orbits the camera, the right one steers the character
            // with it, both together walk forward (as in WoW).
            WindowEvent::MouseInput { state, button: button @ (MouseButton::Left | MouseButton::Right), .. } if self.live.is_some() => {
                if button == MouseButton::Left && state == ElementState::Pressed {
                    self.moved = 0.0;
                }
                let clicked = button == MouseButton::Left && state == ElementState::Released && self.buttons[0] && !self.buttons[1] && self.moved < 5.0;
                self.buttons[(button == MouseButton::Right) as usize] = state == ElementState::Pressed;
                if clicked {
                    let size = w.window.inner_size();
                    let (camera, radius) = (w.camera, w.scene.radius);
                    let aspect = size.width as f32 / size.height.max(1) as f32;
                    let (cx, cy) = (self.last.0 as f32, self.last.1 as f32);
                    let nearest = self
                        .hud
                        .pickables
                        .iter()
                        .filter_map(|&(id, p)| {
                            let [x, y] = camera.project(mao_render::glam::Vec3::from(p), aspect, radius)?;
                            Some((((x * size.width as f32 - cx).powi(2) + (y * size.height as f32 - cy).powi(2)).sqrt(), id))
                        })
                        .filter(|c| c.0 < 48.0)
                        .min_by(|a, b| a.0.total_cmp(&b.0));
                    if let Some((_, id)) = nearest {
                        let double = self.last_click.is_some_and(|(last, at)| last == id && at.elapsed() < std::time::Duration::from_millis(500));
                        self.last_click = Some((id, std::time::Instant::now()));
                        self.keys.pick = (self.keys.pick.0.wrapping_add(1), id, double);
                    }
                }
                let any = self.buttons[0] || self.buttons[1];
                w.window.set_cursor_visible(!any);
                if any {
                    if w.window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                        let _ = w.window.set_cursor_grab(CursorGrabMode::Confined);
                    }
                } else {
                    let _ = w.window.set_cursor_grab(CursorGrabMode::None);
                }
                self.keys.face = self.buttons[1].then(|| heading_from_yaw(w.camera.yaw));
                self.keys.forward = self.held.contains(&Action::Forward) || self.autorun || (self.buttons[0] && self.buttons[1]);
                if let Some(live) = &self.live {
                    let _ = live.input.send(self.keys);
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => self.dragging = state == ElementState::Pressed,
            WindowEvent::CursorMoved { position, .. } => {
                if self.dragging {
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
                        Action::Coordinates if !event.repeat => {
                            if let Some([x, _, z]) = self.hud.position {
                                let place = if self.hud.location.is_empty() { String::new() } else { format!("{}, ", self.hud.location) };
                                self.hud.say(format!("{place}{x:.0}, {z:.0}"));
                            }
                        }
                        Action::CameraToggleView if !event.repeat => {
                            // (between where the camera is and right behind the character's head)
                            match self.home_distance.take() {
                                Some(d) => w.camera.distance = d,
                                None => {
                                    self.home_distance = Some(w.camera.distance);
                                    w.camera.distance = 1.5;
                                }
                            }
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
                        Action::ChatInput | Action::ChatCommand if !event.repeat => {
                            if let Some(ui) = w.ui.as_mut() {
                                ui.chat.focus();
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
                                ui.mission.open = !ui.mission.open;
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
                                ui.options_open = !ui.options_open;
                                ui.rebinding = None;
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
                        }
                        Action::CameraRotateLeft => w.camera.yaw += 0.08,
                        Action::CameraRotateRight => w.camera.yaw -= 0.08,
                        Action::CameraRotateUp => w.camera.pitch = (w.camera.pitch + 0.06).clamp(-1.5, 1.5),
                        Action::CameraRotateDown => w.camera.pitch = (w.camera.pitch - 0.06).clamp(-1.5, 1.5),
                        Action::CameraZoomIn => w.camera.distance = (w.camera.distance * 0.9).max(1.5),
                        Action::CameraZoomOut => w.camera.distance *= 1.1,
                        Action::CameraSavePreferred if !event.repeat => {
                            self.preferred = Some(w.camera);
                            eprintln!("camera saved");
                        }
                        Action::CameraReset if !event.repeat => {
                            if let Some(c) = self.preferred.or(self.home) {
                                (w.camera.yaw, w.camera.pitch, w.camera.distance) = (c.yaw, c.pitch, c.distance);
                            }
                        }
                        _ => {}
                    }
                }
                let held = |a: Action| self.held.contains(&a);
                self.keys.forward = held(Action::Forward) || self.autorun || (self.buttons[0] && self.buttons[1]);
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
                // A live view (following a character) zooms in close; a viewer's limit depends on the scene.
                let closest = if self.live.is_some() { 1.5 } else { w.scene.radius * 0.1 };
                w.camera.distance = (w.camera.distance * (1.0 - lines * 0.1)).max(closest);
                w.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
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
                // Walking, the camera swings round behind the character, unless the mouse is steering it.
                let dt = self.last_follow.elapsed().as_secs_f32().min(0.1);
                self.last_follow = std::time::Instant::now();
                if self.live.is_some() && self.behind_set && self.buttons == [false; 2] && (self.keys.forward || self.keys.back || self.keys.turn != 0) {
                    let difference = (yaw_behind(self.heading) - w.camera.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    w.camera.yaw += difference * (1.0 - (-6.0 * dt).exp());
                }
                if let Some(live) = &self.live {
                    // Every update's messages count; the last one's picture is drawn.
                    let mut updates: Vec<Update> = live.overlays.try_iter().collect();
                    for u in &mut updates {
                        for m in std::mem::take(&mut u.messages) {
                            self.hud.say(m);
                        }
                        self.hud.target = u.target.take();
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
                            if let Some(state) = u.shop.take() {
                                ui.shop.set(state);
                            }
                            if let Some(status) = u.shop_status.take() {
                                ui.shop.status = status;
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
                            self.hud.perk_list = perks;
                        }
                        if let Some(programs) = u.programs.take() {
                            self.hud.programs = programs;
                        }
                        if let Some(friends) = u.friends.take() {
                            self.hud.friends = friends;
                        }
                        if let Some(items) = u.gear.take() {
                            self.hud.gear = items;
                        }
                        if let Some(nanos) = u.nanos.take() {
                            self.hud.nanos = nanos;
                        }
                        if let Some(name) = u.location.take() {
                            self.hud.location = name;
                        }
                        if let Some(p) = u.pickables.take() {
                            self.hud.pickables = p.into_iter().map(|(id, p)| (id, p)).collect();
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
                        for shot in u.shots.take().unwrap_or_default() {
                            self.shots.push((std::time::Instant::now(), shot));
                        }
                        for (at, emitters) in u.bursts.take().unwrap_or_default() {
                            self.bursts.push((std::time::Instant::now(), at, emitters));
                        }
                        self.hud.glows.retain(|g| g.born.elapsed().as_secs_f32() < crate::ui::GLOW_LIFE);
                        self.hud.floaters.retain(|f| f.born.elapsed().as_secs_f32() < crate::ui::FLOATER_LIFE);
                        if let Some(labels) = u.labels.take() {
                            self.hud.labels = labels.into_iter().map(|(p, text)| Label { world: mao_render::glam::Vec3::from(p), text }).collect();
                        }
                    }
                    let latest = updates.into_iter().filter(|u| !u.messages_only).last();
                    if let Some(mut update) = latest {
                        self.hud.position = update.own;
                        self.hud.heading = self.heading;
                        self.hud.map_dots = std::mem::take(&mut update.map_dots);
                        self.hud.nameplates = std::mem::take(&mut update.nameplates);
                        if let Some(h) = update.heading {
                            self.heading = h;
                            if !self.behind_set {
                                self.behind_set = true;
                                w.camera.yaw = yaw_behind(h);
                            }
                        }
                        self.ground = update.ground.take();
                        if let Some(model) = &update.scene {
                            let uploading = std::time::Instant::now();
                            w.scene = w.renderer.upload(&w.gpu, model, &mut |k, i| self.textures.get_for_upload(k, i));
                            profile!("[profile] area scene upload {:?} ({} parts)", uploading.elapsed(), model.parts.len());
                            w.scene.fog = fog_for(self.settings.view_distance);
                        }
                        let own_scene = w.renderer.upload(&w.gpu, &update.own_model, &mut |k, i| self.textures.get_for_upload(k, i));
                        w.scene.set_follow(own_scene);
                        if let Some(p) = update.own {
                            self.own_base = p;
                            self.own_velocity = update.own_velocity;
                            self.own_at = std::time::Instant::now();
                        }
                        let uploading = std::time::Instant::now();
                        let overlay = w.renderer.upload(&w.gpu, &update.model, &mut |k, i| self.textures.get_for_upload(k, i));
                        w.scene.set_overlay(overlay);
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
                        w.camera.target = mao_render::glam::Vec3::new(px, pz, y + 1.3);
                        // The player's feet: plants bend away from them.
                        w.scene.player = Some(mao_render::glam::Vec3::new(px, pz, y));
                    }
                }
                // The sparks of the running nano programs, where they are now (the character's own glides with it).
                {
                    let time = self.fx_clock.elapsed().as_secs_f32();
                    let ahead = self.own_at.elapsed().as_secs_f32().min(0.12);
                    let own_feet = [self.own_base[0] + self.own_velocity[0] * ahead, self.own_base[2] + self.own_velocity[2] * ahead, self.own_base[1]];
                    let mut sparks = Vec::new();
                    for (own, at, auras) in &self.auras {
                        let feet = if *own && self.hud.position.is_some() { own_feet } else { *at };
                        for aura in auras {
                            sparks.extend(crate::effects::visual_particles(aura, feet, time, crate::effects::Reach::Body));
                        }
                    }
                    // (a cast effect emits for a second and lives out its particles' lives)
                    self.bursts.retain(|(began, _, emitters)| began.elapsed().as_secs_f32() < 1.0 + emitters.iter().map(|e| e.life).fold(0.0, f32::max).min(6.0));
                    for (began, at, emitters) in &self.bursts {
                        let age = began.elapsed().as_secs_f32();
                        for e in emitters {
                            sparks.extend(crate::effects::emitter_particles(e, *at, age, 1.0));
                        }
                    }
                    self.shots.retain(|(began, shot)| began.elapsed().as_secs_f32() < shot.lasts());
                    for (began, shot) in &self.shots {
                        sparks.extend(shot.particles(began.elapsed().as_secs_f32()));
                    }
                    load_sprites(&mut w.renderer, &w.gpu, &self.textures, &sparks, &mut self.tried_sprites);
                    w.scene.particles = sparks;
                }
                let frame = match w.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                    _ => {
                        w.surface.configure(&w.gpu.device, &w.config);
                        return;
                    }
                };
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let size = (w.config.width, w.config.height);
                if self.live.is_some() {
                    w.scene.fog = fog_for(self.settings.view_distance);
                    // (an interior has no ground: it is seen against the dark, not the daylight sky)
                    w.scene.sky = if self.ground.is_none() && self.behind_set { mao_render::DARK_SKY } else { mao_render::DAY_SKY };
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
                if let Some((terrain, offset)) = &self.ground {
                    for _ in 0..24 {
                        let eye = camera.eye();
                        if eye.z >= terrain.height_at(eye.x, eye.y, terrain.height_scale) + offset + 0.6 || camera.distance < 0.8 {
                            break;
                        }
                        camera.distance *= 0.88;
                    }
                }
                if std::mem::take(&mut self.screenshot) {
                    let (width, height) = size;
                    let (aspect, radius, shot_camera) = (width as f32 / height.max(1) as f32, w.scene.radius, camera);
                    let format = w.config.format;
                    let pixels = w.renderer.render_to_image_with(&w.gpu, &w.scene, &camera, width, height, &mut |image| {
                        let project = |p: mao_render::glam::Vec3| shot_camera.project(p, aspect, radius);
                        let mut settings = Settings::load();
                        crate::ui::paint_headless(&w.gpu, image, format, (width, height), &mut settings, &self.hud, &project, false);
                    });
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
                    let (day, time) = crate::chatlog::stamp(now);
                    let dir = Settings::path().with_file_name("screenshots");
                    let path = dir.join(format!("{}-{}.png", day.trim_end_matches(".txt"), time.replace(':', "")));
                    let saved = std::fs::create_dir_all(&dir).is_ok() && image::save_buffer(&path, &pixels, width, height, image::ColorType::Rgba8).is_ok();
                    self.hud.say(if saved { format!("Screenshot saved: {}", path.display()) } else { "The screenshot could not be saved".to_string() });
                }
                w.renderer.draw(&w.gpu, &w.scene, &camera, &view, size);
                let stats = w.renderer.stats;
                self.hud.draw_info = format!("{} of {} parts, {} in shadow", stats.drawn, stats.parts, stats.shadow_drawn);
                if let Some(ui) = w.ui.as_mut() {
                    let (camera, radius) = (w.camera, w.scene.radius);
                    let aspect = size.0 as f32 / size.1.max(1) as f32;
                    let project = |p: mao_render::glam::Vec3| camera.project(p, aspect, radius);
                    self.hud.camera_yaw = w.camera.yaw;
                    let (frame, mut requests) = ui.run(&w.window, &mut self.settings, &self.hud, &project);
                    for out in requests.chat.drain(..) {
                        match out {
                            // (these two are for the client itself)
                            crate::chatwindow::Out::Camp => self.keys.commands[5] = self.keys.commands[5].wrapping_add(1),
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
                            other => crate::net::send_chat(other),
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
                    if let Some(which) = requests.target {
                        match which {
                            0 => self.keys.select_self = self.keys.select_self.wrapping_add(1),
                            1 => self.keys.commands[0] = self.keys.commands[0].wrapping_add(1),
                            // (the camera modes: right behind the head, or back where it was)
                            2 => {
                                if self.home_distance.is_none() {
                                    self.home_distance = Some(w.camera.distance);
                                    w.camera.distance = 1.5;
                                }
                            }
                            _ => {
                                if let Some(d) = self.home_distance.take() {
                                    w.camera.distance = d;
                                }
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
                    if let Some(nano) = requests.cancel_nano.take() {
                        self.keys.cancel = (self.keys.cancel.0.wrapping_add(1), nano);
                    }
                    for place in requests.item_uses.drain(..) {
                        self.keys.fire = (self.keys.fire.0.wrapping_add(1), place);
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
                    ui.paint(&w.gpu, &view, size, frame);
                    if let Some(theme) = requests.theme {
                        ui.set_theme(&theme, &self.settings.aou_mod);
                        self.settings.theme = theme;
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
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod camera_tests {
    use super::*;

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
