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
}

/// The overlay to draw and where the player's character is (the view
/// follows it).
#[derive(Default)]
pub struct Update {
    /// A new playfield to replace the one drawn (zoning).
    pub scene: Option<Model>,
    pub model: Model,
    pub own: Option<[f32; 3]>,
    /// The playfield's name, when it changes.
    pub location: Option<String>,
    /// The labels over the world (Z-up positions), when they change.
    pub labels: Option<Vec<([f32; 3], String)>>,
    /// Messages for the log.
    pub messages: Vec<String>,
    /// The current target (name and distance), for the target frame.
    pub target: Option<String>,
    /// The character's items, when they arrive.
    pub gear: Option<Vec<crate::gear::GearItem>>,
    /// The way the character faces (radians about the vertical, 0 along +Z of the server's world).
    pub heading: Option<f32>,
}

/// Something that changes while the window is open: models to draw over the
/// scene (the latest one is shown) and where to start looking.
pub struct Live {
    pub overlays: std::sync::mpsc::Receiver<Update>,
    /// Where the arrow keys' state goes.
    pub input: std::sync::mpsc::Sender<Input>,
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
    let mut scene = renderer.upload(&gpu, model, &mut |k, i| textures.get(k, i));
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
    keys: Input,
    settings: Settings,
    modifiers: winit::keyboard::ModifiersState,
    /// The movement actions whose keys are down.
    held: std::collections::HashSet<Action>,
    /// The camera as first set, and as the player saved it.
    home: Option<Camera>,
    preferred: Option<Camera>,
    hud: Hud,
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
    let mut app = App { model, textures, live, state: None, dragging: false, last: (0.0, 0.0), buttons: [false; 2], heading: 0.0, behind_set: false, last_follow: std::time::Instant::now(), keys: Input::default(), settings, modifiers: Default::default(), held: Default::default(), home: None, preferred: None, hud: Hud::default() };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())
}

/// The heading (about the vertical, 0 along the server's +Z) a camera at `yaw` looks along, and the yaw
/// of a camera right behind a character facing `heading`. (The same function both ways.)
/// Does what a hotbar action says: use an item from the backpack, or attack.
fn fire_action(action: &crate::hotbar::Action, keys: &mut Input, hud: &mut Hud) {
    match action {
        crate::hotbar::Action::Attack => keys.commands[4] = keys.commands[4].wrapping_add(1),
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
        let mut scene = renderer.upload(&gpu, &self.model, &mut |k, i| self.textures.get(k, i));
        if self.live.is_some() {
            // Close-up view: distance fades into the sky.
            scene.fog = (250.0, 1800.0);
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
                self.buttons[(button == MouseButton::Right) as usize] = state == ElementState::Pressed;
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
                self.keys.forward = self.held.contains(&Action::Forward) || (self.buttons[0] && self.buttons[1]);
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
                        Action::Forward | Action::Back | Action::TurnLeft | Action::TurnRight | Action::StrafeLeft | Action::StrafeRight | Action::Jump => {
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
                self.keys.forward = held(Action::Forward) || (self.buttons[0] && self.buttons[1]);
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
                    if let Some(live) = &self.live {
                        let _ = live.input.send(self.keys);
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
                        if let Some(items) = u.gear.take() {
                            self.hud.gear = items;
                        }
                        if let Some(name) = u.location.take() {
                            self.hud.location = name;
                        }
                        if let Some(labels) = u.labels.take() {
                            self.hud.labels = labels.into_iter().map(|(p, text)| Label { world: mao_render::glam::Vec3::from(p), text }).collect();
                        }
                    }
                    let latest = updates.into_iter().last();
                    if let Some(update) = latest {
                        self.hud.position = update.own;
                        if let Some(h) = update.heading {
                            self.heading = h;
                            if !self.behind_set {
                                self.behind_set = true;
                                w.camera.yaw = yaw_behind(h);
                            }
                        }
                        if let Some(model) = &update.scene {
                            w.scene = w.renderer.upload(&w.gpu, model, &mut |k, i| self.textures.get(k, i));
                            w.scene.fog = (250.0, 1800.0);
                        }
                        let overlay = w.renderer.upload(&w.gpu, &update.model, &mut |k, i| self.textures.get(k, i));
                        w.scene.set_overlay(overlay);
                        if let Some([x, y, z]) = update.own {
                            w.camera.target = mao_render::glam::Vec3::new(x, z, y + 1.3);
                            // The player's feet: plants bend away from them.
                            w.scene.player = Some(mao_render::glam::Vec3::new(x, z, y));
                        }
                    }
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
                w.renderer.render_scale = self.settings.render_scale;
                w.renderer.sharpness = self.settings.sharpness;
                w.renderer.draw(&w.gpu, &w.scene, &w.camera, &view, size);
                let stats = w.renderer.stats;
                self.hud.draw_info = format!("{} of {} parts, {} in shadow", stats.drawn, stats.parts, stats.shadow_drawn);
                if let Some(ui) = w.ui.as_mut() {
                    let (camera, radius) = (w.camera, w.scene.radius);
                    let aspect = size.0 as f32 / size.1.max(1) as f32;
                    let project = |p: mao_render::glam::Vec3| camera.project(p, aspect, radius);
                    let (frame, requests) = ui.run(&w.window, &mut self.settings, &self.hud, &project);
                    if let Some(slot) = requests.press {
                        press_slot(ui, slot, &mut self.keys, &mut self.hud);
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
