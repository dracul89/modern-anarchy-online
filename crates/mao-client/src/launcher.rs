//! The launcher (`mao-client play <game>`): a window with the login form (account, password, server), then the account's characters
//! with a Play button each. Playing a character starts the live client for it (`net <game> enter <name> --live`, as a new process
//! so it gets its own window and event loop) with the credentials in that process's environment only; nothing but the account name
//! is written to disk (`account.ini` beside `mao.ini`, so the next start has it filled in).

use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use mao_net::session::{self, Connection};
use mao_net::system::Character;
use mao_render::{wgpu, Gpu};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::settings::Settings;
use crate::ui::Ui;

/// The professions' names by their numbers.
pub fn profession(id: i32) -> &'static str {
    match id {
        1 => "Soldier",
        2 => "Martial Artist",
        3 => "Engineer",
        4 => "Fixer",
        5 => "Agent",
        6 => "Adventurer",
        7 => "Trader",
        8 => "Bureaucrat",
        9 => "Enforcer",
        10 => "Doctor",
        11 => "Nano-Technician",
        12 => "Meta-Physicist",
        14 => "Keeper",
        15 => "Shade",
        _ => "Unknown profession",
    }
}

/// One line about a character: `level 220 Enforcer, Old Athen`.
pub fn describe(c: &Character) -> String {
    let mut text = format!("level {} {}", c.level, profession(c.profession));
    if !c.area.trim().is_empty() {
        text += &format!(", {}", c.area.trim());
    }
    text
}

fn account_path() -> std::path::PathBuf {
    Settings::path().with_file_name("account.ini")
}

enum Stage {
    Form,
    Busy,
    Characters(Vec<Character>),
}

struct Gfx {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: Gpu,
    ui: Ui,
}

struct Launcher {
    game: String,
    settings: Settings,
    gfx: Option<Gfx>,
    stage: Stage,
    user: String,
    password: String,
    server: String,
    message: String,
    answer: Option<Receiver<Result<Vec<Character>, String>>>,
    /// The character to play.
    chosen: Option<String>,
}

impl Launcher {
    fn log_in(&mut self) {
        let (user, password, server) = (self.user.trim().to_string(), self.password.clone(), self.server.trim().to_string());
        if user.is_empty() || password.is_empty() {
            self.message = "Enter the account name and the password.".into();
            return;
        }
        let _ = std::fs::write(account_path(), &user);
        let version = std::fs::read_to_string(std::path::Path::new(&self.game).join("version.id")).map(|t| t.lines().next().unwrap_or("").trim().to_string());
        let (tx, rx) = channel();
        self.answer = Some(rx);
        self.stage = Stage::Busy;
        self.message.clear();
        std::thread::spawn(move || {
            let result = (|| -> Result<Vec<Character>, String> {
                let version = version.map_err(|e| format!("version.id: {e}"))?;
                let mut connection = Connection::connect(server.as_str()).map_err(|e| e.to_string())?;
                let list = session::login(&mut connection, &user, &password, &version).map_err(|e| e.to_string())?;
                // (the login connection must be gone before the game's own login)
                connection.close();
                Ok(list)
            })();
            let _ = tx.send(result);
        });
    }

    fn screen(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.answer {
            if let Ok(result) = rx.try_recv() {
                self.answer = None;
                match result {
                    Ok(list) if list.is_empty() => {
                        self.stage = Stage::Form;
                        self.message = "This account has no characters.".into();
                    }
                    Ok(list) => self.stage = Stage::Characters(list),
                    Err(e) => {
                        self.stage = Stage::Form;
                        self.message = format!("Login failed: {e}");
                    }
                }
            }
        }
        let mut play = None;
        let mut back = false;
        let mut submit = false;
        egui::Window::new("Modern Anarchy Online").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).default_width(360.0).show(ctx, |ui| match &self.stage {
            Stage::Form => {
                egui::Grid::new("login").num_columns(2).spacing(egui::vec2(10.0, 8.0)).show(ui, |ui| {
                    ui.label("Account");
                    let first = ui.add(egui::TextEdit::singleline(&mut self.user).desired_width(220.0));
                    ui.end_row();
                    ui.label("Password");
                    let second = ui.add(egui::TextEdit::singleline(&mut self.password).password(true).desired_width(220.0));
                    ui.end_row();
                    ui.label("Server");
                    ui.add(egui::TextEdit::singleline(&mut self.server).desired_width(220.0));
                    ui.end_row();
                    if first.lost_focus() && second.lost_focus() {
                        submit = true;
                    }
                    if second.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        submit = true;
                    }
                });
                if !self.message.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(240, 120, 110), &self.message);
                }
                ui.add_space(6.0);
                if ui.button("Log in").clicked() {
                    submit = true;
                }
            }
            Stage::Busy => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Logging in...");
                });
            }
            Stage::Characters(list) => {
                ui.label(format!("{} characters", list.len()));
                ui.separator();
                for c in list {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&c.name).strong());
                            ui.label(egui::RichText::new(describe(c)).small().weak());
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let usable = c.created && !c.banned;
                            if ui.add_enabled(usable, egui::Button::new("Play")).clicked() {
                                play = Some(c.name.clone());
                            }
                            if c.banned {
                                ui.label(egui::RichText::new("banned").small().color(egui::Color32::from_rgb(240, 120, 110)));
                            }
                        });
                    });
                }
                ui.separator();
                if ui.button("Back").clicked() {
                    back = true;
                }
            }
        });
        if submit && matches!(self.stage, Stage::Form) {
            self.log_in();
        }
        if back {
            self.stage = Stage::Form;
            self.password.clear();
        }
        if play.is_some() {
            self.chosen = play;
        }
    }
}

impl ApplicationHandler for Launcher {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("Modern Anarchy Online").with_inner_size(PhysicalSize::new(720, 520))).expect("a window"));
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("a surface");
        let gpu = Gpu::new(instance, Some(&surface)).expect("a graphics device");
        let size = window.inner_size();
        let config = surface.get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1)).expect("a surface format");
        surface.configure(&gpu.device, &config);
        let ui = Ui::new(&window, &gpu, config.format, &self.settings);
        window.request_redraw();
        self.gfx = Some(Gfx { window, surface, config, gpu, ui });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // (the form borrows `self` while the graphics are used: take the graphics out for the event)
        let Some(mut gfx) = self.gfx.take() else { return };
        let consumed = gfx.ui.event(&gfx.window, &event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                gfx.config.width = size.width.max(1);
                gfx.config.height = size.height.max(1);
                gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                gfx.window.request_redraw();
            }
            WindowEvent::RedrawRequested => match gfx.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                    // the background: a plain dark blue
                    let mut encoder = gfx.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("launcher background"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.01, g: 0.03, b: 0.06, a: 1.0 }), store: wgpu::StoreOp::Store } })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    }));
                    gfx.gpu.queue.submit(Some(encoder.finish()));
                    let size = (gfx.config.width, gfx.config.height);
                    let ui_frame = gfx.ui.run_custom(&gfx.window, &mut |ctx| self.screen(ctx));
                    gfx.ui.paint(&gfx.gpu, &view, size, ui_frame);
                    gfx.window.pre_present_notify();
                    gfx.gpu.queue.present(frame);
                    if self.chosen.is_some() {
                        event_loop.exit();
                    } else {
                        gfx.window.request_redraw();
                    }
                }
                _ => gfx.surface.configure(&gfx.gpu.device, &gfx.config),
            },
            _ => {
                if consumed {
                    gfx.window.request_redraw();
                }
            }
        }
        self.gfx = Some(gfx);
    }
}

/// Opens the launcher and, when a character is picked, starts the live client for it.
pub fn run(game: &str) -> Result<(), String> {
    let settings = Settings::load();
    let user = std::fs::read_to_string(account_path()).map(|t| t.trim().to_string()).unwrap_or_default();
    let mut launcher = Launcher { game: game.to_string(), settings, gfx: None, stage: Stage::Form, user, password: String::new(), server: std::env::var("MAO_SERVER").unwrap_or_else(|_| session::RUBI_KA.to_string()), message: String::new(), answer: None, chosen: None };
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.run_app(&mut launcher).map_err(|e| e.to_string())?;
    let Some(name) = launcher.chosen.take() else { return Ok(()) };
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let status = std::process::Command::new(exe)
        .args(["net", game, "enter", &name, "--live"])
        .env("MAO_USER", launcher.user.trim())
        .env("MAO_PASSWORD", &launcher.password)
        .env("MAO_SERVER", launcher.server.trim())
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("the game ended with {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn professions_have_names() {
        assert_eq!(profession(9), "Enforcer");
        assert_eq!(profession(15), "Shade");
        assert_eq!(profession(99), "Unknown profession");
    }
}
