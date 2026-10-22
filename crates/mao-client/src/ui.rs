//! The interface drawn over the world: the heads-up display (where you are,
//! the messages, labels over the whompah booths) and the options window
//! (window mode, size, keys). `egui` does the widgets; the look is the AOU
//! mod's translucent theme: dark warm panels with silver and gold accents.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use mao_render::glam::Vec3;
use mao_render::{wgpu, Gpu};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::settings::{Action, Settings, WindowMode, ACTIONS};

/// Text floating over a point of the world (a whompah booth's destination).
pub struct Label {
    pub world: Vec3,
    pub text: String,
}

/// What the display shows besides the options.
#[derive(Default)]
pub struct Hud {
    /// The playfield's name.
    pub location: String,
    /// Where the character stands (x, height, z).
    pub position: Option<[f32; 3]>,
    pub labels: Vec<Label>,
    pub log: VecDeque<(Instant, String)>,
}

impl Hud {
    /// Adds a message to the log (the last few stay visible for a while).
    pub fn say(&mut self, text: impl Into<String>) {
        self.log.push_back((Instant::now(), text.into()));
        while self.log.len() > 8 {
            self.log.pop_front();
        }
    }
}

/// What the options window asks the window to do.
#[derive(Default)]
pub struct Requests {
    pub mode: Option<WindowMode>,
    pub size: Option<(u32, u32)>,
    pub save: bool,
    pub quit: bool,
}

/// One frame of interface, ready to paint.
pub struct Frame {
    jobs: Vec<egui::ClippedPrimitive>,
    textures: egui::TexturesDelta,
    pixels_per_point: f32,
}

pub struct Ui {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    pub options_open: bool,
    /// The action the options window is waiting for a key for.
    pub rebinding: Option<Action>,
    last_frame: Instant,
    fps: f32,
}

const GOLD: egui::Color32 = egui::Color32::from_rgb(212, 175, 55);
const SILVER: egui::Color32 = egui::Color32::from_rgb(190, 192, 200);
const PANEL: egui::Color32 = egui::Color32::from_rgba_premultiplied(18, 14, 10, 215);

/// The AOU mod's translucent theme.
fn theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = PANEL;
    visuals.panel_fill = PANEL;
    visuals.window_stroke = egui::Stroke::new(1.0, SILVER);
    visuals.override_text_color = Some(egui::Color32::from_rgb(230, 226, 214));
    visuals.selection.bg_fill = egui::Color32::from_rgb(120, 96, 24);
    visuals.selection.stroke = egui::Stroke::new(1.0, GOLD);
    visuals.hyperlink_color = GOLD;
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(44, 38, 30);
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(44, 38, 30);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, GOLD);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, GOLD);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 64, 54));
    ctx.set_visuals(visuals);
}

/// `camera_rotate_left` as "Camera rotate left".
fn pretty(name: &str) -> String {
    let mut s = name.replace('_', " ");
    if let Some(first) = s.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    s
}

impl Ui {
    pub fn new(window: &Window, gpu: &Gpu, format: wgpu::TextureFormat) -> Ui {
        let ctx = egui::Context::default();
        theme(&ctx);
        let state = egui_winit::State::new(ctx.clone(), egui::ViewportId::ROOT, window, Some(window.scale_factor() as f32), window.theme(), None);
        let renderer = egui_wgpu::Renderer::new(&gpu.device, format, egui_wgpu::RendererOptions::default());
        Ui { ctx, state, renderer, options_open: false, rebinding: None, last_frame: Instant::now(), fps: 60.0 }
    }

    /// Gives the interface a window event; true when it used the event (the
    /// pointer is over one of its windows) and the game should not.
    pub fn event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.state.on_window_event(window, event).consumed
    }

    /// Lays the interface out for one frame.
    /// `project` finds where a world point lands on the screen (0..1 from the top left).
    pub fn run(&mut self, window: &Window, settings: &mut Settings, hud: &Hud, project: &dyn Fn(Vec3) -> Option<[f32; 2]>) -> (Frame, Requests) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().max(1e-4);
        self.last_frame = now;
        self.fps += (1.0 / dt - self.fps) * 0.05;
        let raw = self.state.take_egui_input(window);
        let mut requests = Requests::default();
        let (options_open, rebinding, fps) = (&mut self.options_open, &mut self.rebinding, self.fps);
        let output = self.ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx().clone();
            hud_layer(&ctx, hud, project, fps, settings.mode);
            if *options_open {
                options_window(&ctx, options_open, rebinding, settings, &mut requests);
            }
        });
        self.state.handle_platform_output(window, output.platform_output);
        let jobs = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        (Frame { jobs, textures: output.textures_delta, pixels_per_point: output.pixels_per_point }, requests)
    }

    /// Paints a frame over what is already in `target`.
    pub fn paint(&mut self, gpu: &Gpu, target: &wgpu::TextureView, size: (u32, u32), frame: Frame) {
        let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [size.0, size.1], pixels_per_point: frame.pixels_per_point };
        for (id, deltas) in &frame.textures.set {
            for delta in deltas {
                self.renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let extra = self.renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &frame.jobs, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("interface"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &frame.jobs, &screen);
        }
        gpu.queue.submit(extra.into_iter().chain(Some(encoder.finish())));
        for id in &frame.textures.free {
            self.renderer.free_texture(id);
        }
    }
}

/// Paints the interface over `target` with no window behind it (a headless
/// preview): the heads-up display, and the options window when asked.
pub fn paint_headless(gpu: &Gpu, target: &wgpu::TextureView, format: wgpu::TextureFormat, size: (u32, u32), settings: &mut Settings, hud: &Hud, project: &dyn Fn(Vec3) -> Option<[f32; 2]>, options: bool) {
    let ctx = egui::Context::default();
    theme(&ctx);
    let mut renderer = egui_wgpu::Renderer::new(&gpu.device, format, egui_wgpu::RendererOptions::default());
    let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [size.0, size.1], pixels_per_point: 1.0 };
    let (mut open, mut rebinding, mut requests) = (options, None, Requests::default());
    let mut last = None;
    // Several frames: the first lays out and uploads the fonts, and the windows
    // fade in over the next ones; the last is what is drawn.
    for frame in 0..12 {
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size.0 as f32, size.1 as f32))), time: Some(frame as f64 * 0.1), ..Default::default() };
        let output = ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx().clone();
            hud_layer(&ctx, hud, project, 60.0, settings.mode);
            if open {
                options_window(&ctx, &mut open, &mut rebinding, settings, &mut requests);
            }
        });
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let jobs = ctx.tessellate(output.shapes, 1.0);
        last = Some(jobs);
    }
    let jobs = last.unwrap_or_default();
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let extra = renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &jobs, &screen);
    {
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("interface"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            })
            .forget_lifetime();
        renderer.render(&mut pass, &jobs, &screen);
    }
    gpu.queue.submit(extra.into_iter().chain(Some(encoder.finish())));
}

/// Where you are, the messages and the labels over the world.
fn hud_layer(ctx: &egui::Context, hud: &Hud, project: &dyn Fn(Vec3) -> Option<[f32; 2]>, fps: f32, mode: WindowMode) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("labels")));
    for label in &hud.labels {
        let Some([x, y]) = project(label.world) else { continue };
        let at = egui::pos2(screen.left() + x * screen.width(), screen.top() + y * screen.height());
        let galley = painter.layout_no_wrap(label.text.clone(), egui::FontId::proportional(14.0), egui::Color32::WHITE);
        let rect = egui::Rect::from_min_size(egui::pos2(at.x - galley.size().x * 0.5, at.y - galley.size().y), galley.size());
        painter.rect_filled(rect.expand(4.0), 4.0, egui::Color32::from_black_alpha(150));
        painter.rect_stroke(rect.expand(4.0), 4.0, egui::Stroke::new(1.0, GOLD), egui::StrokeKind::Outside);
        painter.galley(rect.min, galley, egui::Color32::WHITE);
    }
    let frame = || egui::Frame::new().fill(egui::Color32::from_black_alpha(130)).corner_radius(4).inner_margin(6);
    egui::Area::new(egui::Id::new("info")).anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 8.0)).interactable(false).show(ctx, |ui| {
        frame().show(ui, |ui| {
            if !hud.location.is_empty() {
                ui.label(egui::RichText::new(&hud.location).color(GOLD).strong());
            }
            if let Some([x, y, z]) = hud.position {
                ui.label(format!("{x:.0}, {z:.0}  (height {y:.1})"));
            }
        });
    });
    egui::Area::new(egui::Id::new("fps")).anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 8.0)).interactable(false).show(ctx, |ui| {
        frame().show(ui, |ui| ui.label(format!("{fps:.0} fps  {}  (Esc: options)", mode.name())));
    });
    let recent: Vec<&String> = hud.log.iter().filter(|(t, _)| t.elapsed() < Duration::from_secs(14)).map(|(_, m)| m).collect();
    if !recent.is_empty() {
        egui::Area::new(egui::Id::new("log")).anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(8.0, -8.0)).interactable(false).show(ctx, |ui| {
            frame().show(ui, |ui| {
                for line in recent {
                    ui.label(line);
                }
            });
        });
    }
}

/// The options window: how the window is shown, its size, and the keys.
fn options_window(ctx: &egui::Context, open: &mut bool, rebinding: &mut Option<Action>, settings: &mut Settings, requests: &mut Requests) {
    let mut is_open = *open;
    let mut close = false;
    egui::Window::new(egui::RichText::new("Options").color(GOLD)).open(&mut is_open).collapsible(false).resizable(true).default_width(520.0).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
        ui.heading("Window");
        ui.horizontal(|ui| {
            for mode in [WindowMode::Windowed, WindowMode::Borderless, WindowMode::Fullscreen] {
                if ui.selectable_label(settings.mode == mode, mode.name()).clicked() && settings.mode != mode {
                    requests.mode = Some(mode);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Window size");
            ui.add(egui::DragValue::new(&mut settings.width).range(640..=7680).speed(8));
            ui.label("x");
            ui.add(egui::DragValue::new(&mut settings.height).range(480..=4320).speed(8));
            if ui.button("Apply").clicked() {
                requests.size = Some((settings.width, settings.height));
            }
        });
        ui.separator();
        ui.heading("Keys");
        ui.label("Click a key to change it, then press the new one (Esc cancels).");
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            egui::Grid::new("keys").striped(true).num_columns(3).show(ui, |ui| {
                for &(action, name, _) in ACTIONS {
                    ui.label(pretty(name));
                    let keys = settings.keymap.keys(action);
                    let text = if *rebinding == Some(action) {
                        "press a key...".to_string()
                    } else if keys.is_empty() {
                        "(none)".to_string()
                    } else {
                        keys
                    };
                    if ui.add_sized([180.0, 20.0], egui::Button::new(text)).clicked() {
                        *rebinding = Some(action);
                    }
                    if ui.small_button("x").on_hover_text("Unbind").clicked() {
                        settings.keymap.set(action, "");
                        requests.save = true;
                    }
                    ui.end_row();
                }
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Default keys").clicked() {
                settings.keymap = crate::settings::Keymap::default();
                requests.save = true;
            }
            if ui.button("Save settings").clicked() {
                requests.save = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Quit game").clicked() {
                    requests.quit = true;
                }
                if ui.button("Resume").clicked() {
                    close = true;
                }
            });
        });
    });
    *open = is_open && !close;
}
