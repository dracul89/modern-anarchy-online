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
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Fullscreen, Window, WindowId};

use crate::settings::{Action, Settings, WindowMode};
use crate::Textures;

/// What the player is doing with the keyboard: forward held, and turning
/// (-1 left, 1 right).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub forward: bool,
    pub back: bool,
    pub jump: bool,
    /// Walk through props (toggled with C).
    pub noclip: bool,
    pub turn: i8,
}

/// The overlay to draw and where the player's character is (the view
/// follows it).
pub struct Update {
    /// A new playfield to replace the one drawn (zoning).
    pub scene: Option<Model>,
    pub model: Model,
    pub own: Option<[f32; 3]>,
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
    let gpu = Gpu::headless()?;
    eprintln!("adapter: {:?}", gpu.adapter.get_info());
    let mut renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
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
    let pixels = renderer.render_to_image(&gpu, &scene, &camera, w, h);
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
}

struct App {
    model: Model,
    textures: Textures,
    live: Option<Live>,
    state: Option<View>,
    dragging: bool,
    last: (f64, f64),
    keys: Input,
    settings: Settings,
    modifiers: winit::keyboard::ModifiersState,
    /// The movement actions whose keys are down.
    held: std::collections::HashSet<Action>,
    /// The camera as first set, and as the player saved it.
    home: Option<Camera>,
    preferred: Option<Camera>,
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
    let mut app = App { model, textures, live, state: None, dragging: false, last: (0.0, 0.0), keys: Input::default(), settings, modifiers: Default::default(), held: Default::default(), home: None, preferred: None };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())
}

impl ApplicationHandler for App {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.live.is_some() {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(16)));
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
        let config = surface.get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1)).expect("a surface format");
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
        self.state = Some(View { window, surface, config, gpu, renderer, scene, camera });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(w) = self.state.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                w.config.width = size.width.max(1);
                w.config.height = size.height.max(1);
                w.surface.configure(&w.gpu.device, &w.config);
                w.window.request_redraw();
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
                if !down {
                    // A released key ends every held action it started.
                    for action in self.settings.keymap.actions_for_key(code) {
                        self.held.remove(&action);
                    }
                } else if let Some(action) = self.settings.keymap.action(code, self.modifiers) {
                    match action {
                        Action::Forward | Action::Back | Action::TurnLeft | Action::TurnRight | Action::Jump => {
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
                        Action::Quit => {
                            event_loop.exit();
                            return;
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
                self.keys.forward = held(Action::Forward);
                self.keys.back = held(Action::Back);
                self.keys.jump = held(Action::Jump);
                self.keys.turn = held(Action::TurnRight) as i8 - held(Action::TurnLeft) as i8;
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
                if let Some(live) = &self.live {
                    let latest = live.overlays.try_iter().last();
                    if let Some(update) = latest {
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
                w.renderer.draw(&w.gpu, &w.scene, &w.camera, &view, (w.config.width, w.config.height));
                w.window.pre_present_notify();
                w.gpu.queue.present(frame);
            }
            _ => {}
        }
    }
}
