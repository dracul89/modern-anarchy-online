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
use crate::gear::{GearItem, GearWindow};
use crate::hotbar::{self, Hotbars};
use crate::theme::{self, Palette};

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
    /// The current target, for the target frame.
    pub target: Option<String>,
    /// The character's items (the inventory window), and whether a headless preview shows it.
    pub gear: Vec<crate::gear::GearItem>,
    pub gear_open: bool,
    /// Hotbars shown by a headless preview (a live window keeps its own in the `Ui`).
    pub preview_hotbars: Hotbars,
    /// The travel list the server sent for this playfield.
    pub destinations: Option<mao_net::destinations::DestinationList>,
    /// Whether a headless preview shows the Whom-Pah window.
    pub whompah_open: bool,
    /// What the last frame drew (parts), shown beside the frame rate.
    pub draw_info: String,
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
    /// A theme picked in the options window.
    pub theme: Option<String>,
    pub mode: Option<WindowMode>,
    pub size: Option<(u32, u32)>,
    pub save: bool,
    pub quit: bool,
    /// A hotbar slot clicked with the mouse.
    pub press: Option<usize>,
    /// A place to walk to (world x, z), from the Whom-Pah window.
    pub goto: Option<[f32; 2]>,
    /// The hotbars were edited (to be saved).
    pub hotbars_changed: bool,
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
    pub gear_open: bool,
    pub whompah_open: bool,
    gear: crate::gear::GearWindow,
    /// The hotbars, and the clock their cooldowns run on.
    pub hotbars: Hotbars,
    clock: Instant,
    /// The action the options window is waiting for a key for.
    pub rebinding: Option<Action>,
    /// The colours in use, and the themes the options window offers.
    pub palette: Palette,
    themes: Vec<String>,
    last_frame: Instant,
    fps: f32,
}

fn rgb(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

fn lifted(c: [u8; 3], by: u8) -> egui::Color32 {
    egui::Color32::from_rgb(c[0].saturating_add(by), c[1].saturating_add(by), c[2].saturating_add(by))
}

/// Dresses the interface in a palette (a theme's GUIColors.xml).
fn theme(ctx: &egui::Context, p: &Palette) {
    let mut visuals = egui::Visuals::dark();
    let panel = egui::Color32::from_rgba_unmultiplied(p.panel[0], p.panel[1], p.panel[2], 215);
    visuals.window_fill = panel;
    visuals.panel_fill = panel;
    visuals.window_stroke = egui::Stroke::new(1.0, rgb(p.accent));
    visuals.override_text_color = Some(rgb(p.text));
    visuals.selection.bg_fill = rgb(p.accent.map(|v| (v as f32 * 0.4) as u8));
    visuals.selection.stroke = egui::Stroke::new(1.0, rgb(p.text_selected));
    visuals.hyperlink_color = rgb(p.accent);
    visuals.widgets.inactive.bg_fill = lifted(p.panel, 26);
    visuals.widgets.inactive.weak_bg_fill = lifted(p.panel, 26);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, rgb(p.hover));
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, rgb(p.hover));
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, lifted(p.panel, 60));
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
    pub fn new(window: &Window, gpu: &Gpu, format: wgpu::TextureFormat, settings: &Settings) -> Ui {
        let ctx = egui::Context::default();
        let palette = theme::load(&settings.theme, &settings.aou_mod);
        theme(&ctx, &palette);
        let state = egui_winit::State::new(ctx.clone(), egui::ViewportId::ROOT, window, Some(window.scale_factor() as f32), window.theme(), None);
        let renderer = egui_wgpu::Renderer::new(&gpu.device, format, egui_wgpu::RendererOptions::default());
        Ui { ctx, state, renderer, options_open: false, gear_open: false, whompah_open: false, gear: Default::default(), hotbars: hotbar::load(), clock: Instant::now(), rebinding: None, palette, themes: theme::available(&settings.aou_mod), last_frame: Instant::now(), fps: 60.0 }
    }

    /// Switches to the theme `name` (a `theme =` setting).
    pub fn set_theme(&mut self, name: &str, aou_mod: &str) {
        self.palette = theme::load(name, aou_mod);
        theme(&self.ctx, &self.palette);
    }

    /// Gives the interface a window event; true when it used the event (the
    /// pointer is over one of its windows) and the game should not.
    pub fn event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.state.on_window_event(window, event).consumed
    }

    /// Seconds on the hotbars' clock.
    pub fn now(&self) -> f64 {
        self.clock.elapsed().as_secs_f64()
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
        let (options_open, rebinding, fps, palette, themes) = (&mut self.options_open, &mut self.rebinding, self.fps, &self.palette, &self.themes);
        let (gear_open, gear, whompah_open) = (&mut self.gear_open, &mut self.gear, &mut self.whompah_open);
        let (hotbars, clock_now) = (&mut self.hotbars, self.clock.elapsed().as_secs_f64());
        let output = self.ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx().clone();
            hud_layer(&ctx, hud, project, fps, settings.mode, palette);
            hotbar_layer(&ctx, hotbars, gear, &hud.gear, clock_now, palette, &mut requests);
            if *whompah_open {
                whompah_window(&ctx, whompah_open, hud, palette, &mut requests);
            }
            if *gear_open {
                gear.show(&ctx, gear_open, &hud.gear, palette);
            }
            if *options_open {
                options_window(&ctx, options_open, rebinding, settings, &mut requests, palette, themes);
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
    let palette = theme::load(&settings.theme, &settings.aou_mod);
    let themes = theme::available(&settings.aou_mod);
    theme(&ctx, &palette);
    let mut renderer = egui_wgpu::Renderer::new(&gpu.device, format, egui_wgpu::RendererOptions::default());
    let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [size.0, size.1], pixels_per_point: 1.0 };
    let (mut open, mut rebinding, mut requests) = (options, None, Requests::default());
    let (mut gear_open, mut gear) = (hud.gear_open, crate::gear::GearWindow::default());
    let mut hotbars = hud.preview_hotbars.clone();
    let mut last = None;
    // Several frames: the first lays out and uploads the fonts, and the windows
    // fade in over the next ones; the last is what is drawn.
    for frame in 0..12 {
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size.0 as f32, size.1 as f32))), time: Some(frame as f64 * 0.1), ..Default::default() };
        let output = ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx().clone();
            hud_layer(&ctx, hud, project, 60.0, settings.mode, &palette);
            hotbar_layer(&ctx, &mut hotbars, &mut gear, &hud.gear, 1.5, &palette, &mut requests);
            if hud.whompah_open {
                whompah_window(&ctx, &mut true, hud, &palette, &mut requests);
            }
            if gear_open {
                gear.show(&ctx, &mut gear_open, &hud.gear, &palette);
            }
            if open {
                options_window(&ctx, &mut open, &mut rebinding, settings, &mut requests, &palette, &themes);
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
/// The Whom-Pah window: the booths of this area (their labels say where each leads, once known), how
/// far they are, and a button that walks to one (the server zones a character that walks into a booth).
/// Below it, the travel list the server sent for this playfield, when there is one.
fn whompah_window(ctx: &egui::Context, open: &mut bool, hud: &Hud, palette: &Palette, requests: &mut Requests) {
    let accent = rgb(palette.accent);
    let mut is_open = *open;
    egui::Window::new(egui::RichText::new("Whom-Pah").color(accent)).open(&mut is_open).collapsible(false).resizable(true).default_width(340.0).anchor(egui::Align2::RIGHT_CENTER, egui::vec2(-16.0, 0.0)).show(ctx, |ui| {
        let here = hud.position;
        let mut booths: Vec<(f32, &Label)> = hud.labels.iter().map(|l| (here.map_or(0.0, |p| ((l.world.x - p[0]).powi(2) + (l.world.y - p[2]).powi(2)).sqrt()), l)).collect();
        booths.sort_by(|a, b| a.0.total_cmp(&b.0));
        if booths.is_empty() {
            ui.label("No Whom-Pah booths in this area.");
        }
        for (distance, label) in booths {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&label.text).strong());
                ui.label(egui::RichText::new(format!("{distance:.0} m")).weak());
                if ui.button("Go").on_hover_text("Walk to the booth (any movement key stops)").clicked() {
                    requests.goto = Some([label.world.x, label.world.y]);
                }
            });
        }
        if let Some(list) = &hud.destinations {
            ui.separator();
            ui.label(egui::RichText::new("Travel list for this area (from the server)").small().color(accent));
            for d in &list.entries {
                ui.label(&d.name);
            }
        }
    });
    *open = is_open;
}

/// The hotbar: tabs for the six bars and ten slots. A slot shows the action that would fire (its icon,
/// a cooldown sweep and the time left); a yellow frame means a press is queued. Left click presses,
/// right click edits the slot's list, and items dragged from the inventory window are added to it.
fn hotbar_layer(ctx: &egui::Context, bars: &mut Hotbars, gear: &mut GearWindow, items: &[GearItem], now: f64, palette: &Palette, requests: &mut Requests) {
    let accent = rgb(palette.accent);
    let bar = bars.current;
    egui::Area::new(egui::Id::new("hotbar")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -12.0)).show(ctx, |ui| {
        egui::Frame::new().fill(egui::Color32::from_black_alpha(130)).corner_radius(4).inner_margin(6).show(ui, |ui| {
            ui.horizontal(|ui| {
                for b in 0..hotbar::BARS {
                    if ui.selectable_label(bars.current == b, format!("{}", b + 1)).clicked() {
                        bars.current = b;
                    }
                }
                ui.label(egui::RichText::new("F1-F6 bars, 1-0 slots, drag items here, right-click to edit").small().weak());
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                for slot in 0..hotbar::SLOTS {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::click());
                    let queued = bars.is_queued(bar, slot);
                    let shown = bars.slot(bar, slot).display(now).map(|d| (d.entry.action.clone(), d.entry.cooldown, d.remaining));
                    let count = bars.slot(bar, slot).entries.len();
                    let hovering = response.dnd_hover_payload::<hotbar::Action>().is_some();
                    let painter = ui.painter();
                    painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(140));
                    let border = if queued { egui::Color32::YELLOW } else if hovering { accent } else if shown.is_some() { lifted(palette.panel, 90) } else { egui::Color32::from_white_alpha(30) };
                    if let Some((action, cooldown, remaining)) = &shown {
                        let icon = match action {
                            hotbar::Action::Item { icon: Some(id), .. } => items.iter().find_map(|i| i.icon.as_ref().filter(|ic| ic.id == *id)).cloned(),
                            _ => None,
                        };
                        match icon {
                            Some(icon) => {
                                let id = gear.texture(ctx, &icon);
                                ui.painter().image(id, rect.shrink(3.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                            None => {
                                let letter = action.name().chars().next().unwrap_or('?');
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, letter, egui::FontId::proportional(20.0), accent);
                            }
                        }
                        if *remaining > 0.0 {
                            // The cooldown sweeps down from the top as it runs out.
                            let fraction = (remaining / cooldown.max(0.1)).clamp(0.0, 1.0);
                            let cover = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + rect.height() * fraction));
                            ui.painter().rect_filled(cover, 3.0, egui::Color32::from_black_alpha(170));
                            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{remaining:.1}"), egui::FontId::proportional(13.0), egui::Color32::WHITE);
                        }
                        if count > 1 {
                            ui.painter().text(rect.right_bottom() - egui::vec2(3.0, 2.0), egui::Align2::RIGHT_BOTTOM, format!("+{}", count - 1), egui::FontId::proportional(10.0), egui::Color32::LIGHT_GRAY);
                        }
                    }
                    ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(if queued || hovering { 2.0 } else { 1.0 }, border), egui::StrokeKind::Inside);
                    ui.painter().text(rect.left_top() + egui::vec2(3.0, 2.0), egui::Align2::LEFT_TOP, format!("{}", (slot + 1) % 10), egui::FontId::proportional(10.0), egui::Color32::from_white_alpha(160));
                    if let Some(payload) = response.dnd_release_payload::<hotbar::Action>() {
                        bars.add(bar, slot, (*payload).clone(), 5.0);
                        requests.hotbars_changed = true;
                    }
                    if response.clicked() {
                        requests.press = Some(slot);
                    }
                    let tooltip: Vec<String> = bars.slot(bar, slot).entries.iter().enumerate().map(|(i, e)| format!("{}. {} ({:.0}s){}", i + 1, e.action.name(), e.cooldown, if e.ready_at > now { format!(", {:.1}s left", e.ready_at - now) } else { String::new() })).collect();
                    if !tooltip.is_empty() {
                        response.clone().on_hover_text(tooltip.join("\n"));
                    }
                    response.context_menu(|ui| {
                        ui.label(egui::RichText::new(format!("Bar {} slot {}: the first ready action fires", bar + 1, (slot + 1) % 10)).small());
                        let mut changed = false;
                        let (mut remove, mut up, mut down) = (None, None, None);
                        let entries = &mut bars.slot_mut(bar, slot).entries;
                        let last = entries.len().saturating_sub(1);
                        for (i, entry) in entries.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}. {}", i + 1, entry.action.name()));
                                changed |= ui.add(egui::DragValue::new(&mut entry.cooldown).range(0.0..=3600.0).speed(0.1).suffix(" s")).changed();
                                if i > 0 && ui.small_button("^").clicked() {
                                    up = Some(i);
                                }
                                if i < last && ui.small_button("v").clicked() {
                                    down = Some(i);
                                }
                                if ui.small_button("x").clicked() {
                                    remove = Some(i);
                                }
                            });
                        }
                        if let Some(i) = up {
                            entries.swap(i, i - 1);
                            changed = true;
                        }
                        if let Some(i) = down {
                            entries.swap(i, i + 1);
                            changed = true;
                        }
                        if let Some(i) = remove {
                            entries.remove(i);
                            changed = true;
                        }
                        if ui.button("Add Attack").clicked() {
                            entries.push(hotbar::Entry::new(hotbar::Action::Attack, 2.0));
                            changed = true;
                            ui.close();
                        }
                        if ui.button("Clear slot").clicked() {
                            entries.clear();
                            changed = true;
                            ui.close();
                        }
                        if changed {
                            requests.hotbars_changed = true;
                            if bars.slot(bar, slot).entries.is_empty() {
                                bars.cleared(bar, slot);
                            }
                        }
                    });
                }
            });
        });
    });
    // While an item is being dragged, say what to do with it.
    if egui::DragAndDrop::has_payload_of_type::<hotbar::Action>(ctx) {
        if let Some(pos) = ctx.pointer_interact_pos() {
            ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("drag-hint"))).text(pos + egui::vec2(14.0, 14.0), egui::Align2::LEFT_TOP, "Drop on a hotbar slot", egui::FontId::proportional(12.0), egui::Color32::WHITE);
        }
    }
}

fn hud_layer(ctx: &egui::Context, hud: &Hud, project: &dyn Fn(Vec3) -> Option<[f32; 2]>, fps: f32, mode: WindowMode, palette: &Palette) {
    let accent = rgb(palette.accent);
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("labels")));
    for label in &hud.labels {
        let Some([x, y]) = project(label.world) else { continue };
        let at = egui::pos2(screen.left() + x * screen.width(), screen.top() + y * screen.height());
        let galley = painter.layout_no_wrap(label.text.clone(), egui::FontId::proportional(14.0), egui::Color32::WHITE);
        let rect = egui::Rect::from_min_size(egui::pos2(at.x - galley.size().x * 0.5, at.y - galley.size().y), galley.size());
        painter.rect_filled(rect.expand(4.0), 4.0, egui::Color32::from_black_alpha(150));
        painter.rect_stroke(rect.expand(4.0), 4.0, egui::Stroke::new(1.0, accent), egui::StrokeKind::Outside);
        painter.galley(rect.min, galley, egui::Color32::WHITE);
    }
    let frame = || egui::Frame::new().fill(egui::Color32::from_black_alpha(130)).corner_radius(4).inner_margin(6);
    egui::Area::new(egui::Id::new("info")).anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 8.0)).interactable(false).show(ctx, |ui| {
        frame().show(ui, |ui| {
            if !hud.location.is_empty() {
                ui.label(egui::RichText::new(&hud.location).color(accent).strong());
            }
            if let Some([x, y, z]) = hud.position {
                ui.label(format!("{x:.0}, {z:.0}  (height {y:.1})"));
            }
        });
    });
    egui::Area::new(egui::Id::new("fps")).anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 8.0)).interactable(false).show(ctx, |ui| {
        frame().show(ui, |ui| {
            ui.label(format!("{fps:.0} fps  {}  (Esc: options)", mode.name()));
            if !hud.draw_info.is_empty() {
                ui.label(egui::RichText::new(&hud.draw_info).small());
            }
        });
    });
    if let Some(target) = &hud.target {
        egui::Area::new(egui::Id::new("target")).anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 8.0)).interactable(false).show(ctx, |ui| {
            frame().show(ui, |ui| {
                ui.label(egui::RichText::new("Target").small().color(accent));
                ui.label(egui::RichText::new(target).strong());
            });
        });
    }
    let recent: Vec<&String> = hud.log.iter().filter(|(t, _)| t.elapsed() < Duration::from_secs(14)).map(|(_, m)| m).collect();
    if !recent.is_empty() {
        egui::Area::new(egui::Id::new("log")).anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(8.0, -72.0)).interactable(false).show(ctx, |ui| {
            frame().show(ui, |ui| {
                for line in recent {
                    ui.label(line);
                }
            });
        });
    }
}

/// The options window: how the window is shown, its size, and the keys.
fn options_window(ctx: &egui::Context, open: &mut bool, rebinding: &mut Option<Action>, settings: &mut Settings, requests: &mut Requests, palette: &Palette, themes: &[String]) {
    let mut is_open = *open;
    let mut close = false;
    egui::Window::new(egui::RichText::new("Options").color(rgb(palette.accent))).open(&mut is_open).collapsible(false).resizable(true).default_width(520.0).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
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
        ui.horizontal(|ui| {
            ui.label("Render scale");
            ui.add(egui::Slider::new(&mut settings.render_scale, 0.4..=1.0).step_by(0.01).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
            ui.label(egui::RichText::new("(below 100% the scene is upscaled with AMD FSR 1)").weak());
        });
        ui.horizontal(|ui| {
            ui.label("Sharpness");
            ui.add(egui::Slider::new(&mut settings.sharpness, 0.0..=2.0).text("stops down").step_by(0.05));
        });
        ui.separator();
        ui.heading("Interface");
        ui.horizontal(|ui| {
            ui.label("Theme");
            egui::ComboBox::from_id_salt("theme").selected_text(&settings.theme).show_ui(ui, |ui| {
                for name in themes {
                    if ui.selectable_label(&settings.theme == name, name).clicked() && &settings.theme != name {
                        requests.theme = Some(name.clone());
                    }
                }
            });
            ui.label(egui::RichText::new("(the AOU mod's themes need its folder in the settings)").weak());
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
