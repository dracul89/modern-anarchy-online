//! The launcher (`mao-client play <game>`): a window with the login form (account, password, server), then the account's characters
//! with a Play button each. Playing a character starts the live client for it (`net <game> enter <name> --live`, as a new process
//! so it gets its own window and event loop) with the credentials in that process's environment only; nothing but the account name
//! is written to disk (`account.ini` beside `mao.ini`, so the next start has it filled in).

use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use mao_net::session::{self, Connection};
use mao_net::system::Character;
use mao_formats::resource::ResourceDatabase;
use mao_render::{wgpu, Camera, Gpu, Renderer, Scene};
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

/// The breeds' names (the breed stat's values).
pub fn breed(id: i32) -> &'static str {
    match id {
        1 => "Solitus",
        2 => "Opifex",
        3 => "Nanomage",
        4 => "Atrox",
        _ => "",
    }
}

/// The sex stat's values: 2 male, 3 female (1 is neuter).
pub fn sex(id: i32) -> &'static str {
    match id {
        2 => "male",
        3 => "female",
        _ => "",
    }
}

/// One line about a character: `level 220 male Solitus Enforcer, Old Athen`.
pub fn describe(c: &Character) -> String {
    let who = [sex(c.sex), breed(c.breed), profession(c.profession)].iter().filter(|w| !w.is_empty()).copied().collect::<Vec<_>>().join(" ");
    let mut text = format!("level {} {who}", c.level);
    // (the list says "area unknown" when it does not know)
    if !c.area.trim().is_empty() && !c.area.trim().eq_ignore_ascii_case("area unknown") {
        text += &format!(", {}", c.area.trim());
    }
    text
}

fn account_path() -> std::path::PathBuf {
    Settings::path().with_file_name("account.ini")
}

/// What `account.ini` holds: the account's name and, on a second line, the character played last.
pub fn parse_account(text: &str) -> (String, String) {
    let mut lines = text.lines().map(str::trim);
    (lines.next().unwrap_or("").to_string(), lines.next().unwrap_or("").to_string())
}

/// The characters with the one played last first (the rest keep their order).
pub fn ordered<'a>(list: &'a [Character], last: &str) -> Vec<&'a Character> {
    let mut rows: Vec<&Character> = list.iter().collect();
    rows.sort_by_key(|c| !(!last.is_empty() && c.name.eq_ignore_ascii_case(last)));
    rows
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
    renderer: Renderer,
    /// The character shown in the window: its name, the scene and the textures that made it.
    shown: Option<String>,
    scene: Option<Scene>,
    camera: Option<Camera>,
    db: Option<(Arc<ResourceDatabase>, crate::Textures)>,
    started: std::time::Instant,
    frames: u32,
}

struct Launcher {
    game: String,
    settings: Settings,
    gfx: Option<Gfx>,
    stage: Stage,
    user: String,
    /// The character played last, to list first.
    last: String,
    password: String,
    server: String,
    message: String,
    answer: Option<Receiver<Result<Vec<Character>, String>>>,
    /// The character to play.
    chosen: Option<String>,
    /// The character picked in the list (shown in the window; Play starts it).
    selected: Option<String>,
    /// `MAO_LAUNCHER_SHOT=out.png`: save a picture of the character screen and quit (with `MAO_LAUNCHER_DEMO=1`, three sample characters).
    shot: Option<String>,
    /// What the characters looked like when last played (`looks.ini`).
    looks: Vec<crate::look::Look>,
}

impl Launcher {
    fn log_in(&mut self) {
        let (user, password, server) = (self.user.trim().to_string(), self.password.clone(), self.server.trim().to_string());
        if user.is_empty() || password.is_empty() {
            self.message = "Enter the account name and the password.".into();
            return;
        }
        let _ = std::fs::write(account_path(), format!("{user}
{}
", self.last));
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

    /// The character screen: the list on the left, the selected character in the window, Play at the bottom.
    fn character_screen(&mut self, ctx: &egui::Context, list: &[Character], play: &mut Option<String>, back: &mut bool) {
        let accent = egui::Color32::from_rgb(120, 200, 255);
        egui::Window::new("Characters").title_bar(false).collapsible(false).resizable(false).anchor(egui::Align2::LEFT_CENTER, egui::vec2(16.0, 0.0)).fixed_size([280.0, 0.0]).show(ctx, |ui| {
            ui.label(egui::RichText::new(format!("{} characters", list.len())).small().weak());
            ui.separator();
            for c in ordered(list, &self.last) {
                let usable = c.created && !c.banned;
                let picked = self.selected.as_deref() == Some(c.name.as_str());
                let last = !self.last.is_empty() && c.name.eq_ignore_ascii_case(&self.last);
                let title = if last { format!("{} (played last)", c.name) } else { c.name.clone() };
                let mut text = egui::RichText::new(title).strong();
                if c.banned {
                    text = text.color(egui::Color32::from_rgb(240, 120, 110));
                }
                let response = ui.add_enabled(usable, egui::Button::selectable(picked, text).min_size(egui::vec2(ui.available_width(), 0.0)));
                ui.label(egui::RichText::new(describe(c)).small().weak());
                if c.banned {
                    ui.label(egui::RichText::new("banned").small().color(egui::Color32::from_rgb(240, 120, 110)));
                }
                if response.clicked() {
                    self.selected = Some(c.name.clone());
                }
                if response.double_clicked() {
                    *play = Some(c.name.clone());
                }
                ui.add_space(4.0);
            }
        });
        if let Some(c) = list.iter().find(|c| self.selected.as_deref() == Some(c.name.as_str())) {
            egui::Area::new(egui::Id::new("who")).anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 18.0)).interactable(false).show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(&c.name).size(26.0).strong().color(egui::Color32::WHITE));
                    ui.label(egui::RichText::new(describe(c)).color(accent));
                });
            });
        }
        egui::Area::new(egui::Id::new("bottom")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -22.0)).show(ctx, |ui| {
            let ready = self.selected.is_some();
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                if ui.add_enabled(ready, egui::Button::new(egui::RichText::new("Play").size(22.0).strong()).min_size(egui::vec2(240.0, 46.0))).clicked() {
                    *play = self.selected.clone();
                }
            });
        });
        egui::Area::new(egui::Id::new("back")).anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -22.0)).show(ctx, |ui| {
            if ui.button("Back").clicked() {
                *back = true;
            }
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
                    Ok(list) => {
                        self.selected = ordered(&list, &self.last).into_iter().find(|c| c.created && !c.banned).map(|c| c.name.clone());
                        self.stage = Stage::Characters(list);
                    }
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
        if let Stage::Characters(list) = &self.stage {
            let list = list.clone();
            self.character_screen(ctx, &list, &mut play, &mut back);
        }
        if !matches!(self.stage, Stage::Characters(_)) {
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
            Stage::Characters(_) => {}
        });
        }
        if submit && matches!(self.stage, Stage::Form) {
            self.log_in();
        }
        if back {
            self.stage = Stage::Form;
            self.password.clear();
        }
        if play.is_some() {
            // (the account's name and the character stay for the next start; the password is never written)
            if let Some(name) = &play {
                let _ = std::fs::write(account_path(), format!("{}
{name}
", self.user.trim()));
            }
            self.chosen = play;
        }
    }
}

impl ApplicationHandler for Launcher {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("Modern Anarchy Online").with_inner_size(PhysicalSize::new(1000, 640))).expect("a window"));
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("a surface");
        let gpu = Gpu::new(instance, Some(&surface)).expect("a graphics device");
        let size = window.inner_size();
        let config = surface.get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1)).expect("a surface format");
        surface.configure(&gpu.device, &config);
        // (a picture of the screen is drawn off screen in a format both the scene and the interface can use)
        let format = if self.shot.is_some() { wgpu::TextureFormat::Rgba8UnormSrgb } else { config.format };
        let ui = Ui::new(&window, &gpu, format, &self.settings);
        let renderer = Renderer::new(&gpu, format);
        window.request_redraw();
        self.gfx = Some(Gfx { window, surface, config, gpu, ui, renderer, shown: None, scene: None, camera: None, db: None, started: std::time::Instant::now(), frames: 0 });
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
                    let size = (gfx.config.width, gfx.config.height);
                    let ui_frame = gfx.ui.run_custom(&gfx.window, &mut |ctx| self.screen(ctx));
                    // the character picked in the list is built once and turned slowly
                    let want = match &self.stage {
                        Stage::Characters(list) => self.selected.as_ref().and_then(|n| list.iter().find(|c| &c.name == n)).cloned(),
                        _ => None,
                    };
                    if want.as_ref().map(|c| &c.name) != gfx.shown.as_ref() {
                        gfx.shown = want.as_ref().map(|c| c.name.clone());
                        gfx.scene = None;
                        if let Some(c) = &want {
                            if gfx.db.is_none() {
                                if let Ok(db) = ResourceDatabase::open_game(std::path::Path::new(&self.game)) {
                                    let db = Arc::new(db);
                                    gfx.db = Some((db.clone(), crate::Textures::new(db)));
                                }
                            }
                            if let Some((db, textures)) = &gfx.db {
                                if let Some(model) = crate::net::character_model(db, c.breed, c.sex, crate::look::find(&self.looks, &c.name).as_ref()) {
                                    let mut scene = gfx.renderer.upload(&gfx.gpu, &model, &mut |k, i| textures.get(k, i));
                                    scene.sky = ([0.05, 0.09, 0.15], [0.01, 0.03, 0.07]);
                                    scene.fog = (400.0, 800.0);
                                    let mut camera = Camera::fit(&scene);
                                    // (the whole figure, from a little above its middle)
                                    camera.distance = scene.radius * 3.0;
                                    camera.pitch = 0.12;
                                    gfx.camera = Some(camera);
                                    gfx.scene = Some(scene);
                                }
                            }
                        }
                    }
                    gfx.frames += 1;
                    if let (Some(scene), Some(camera)) = (&gfx.scene, gfx.camera.as_mut()) {
                        camera.yaw = 0.9 + gfx.started.elapsed().as_secs_f32() * 0.5;
                        if let Some(out) = self.shot.clone().filter(|_| gfx.frames >= 12) {
                            camera.yaw = 0.9;
                            let camera = *camera;
                            let mut frame_slot = Some(ui_frame);
                            let pixels = gfx.renderer.render_to_image_with(&gfx.gpu, scene, &camera, size.0, size.1, &mut |target| {
                                if let Some(f) = frame_slot.take() {
                                    gfx.ui.paint(&gfx.gpu, target, size, f);
                                }
                            });
                            let _ = image::save_buffer(&out, &pixels, size.0, size.1, image::ColorType::Rgba8);
                            event_loop.exit();
                            self.gfx = Some(gfx);
                            return;
                        }
                        let camera = *camera;
                        gfx.renderer.draw(&gfx.gpu, scene, &camera, &view, size);
                        gfx.ui.paint(&gfx.gpu, &view, size, ui_frame);
                    } else {
                        // no character shown: a plain dark blue background
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
                        gfx.ui.paint(&gfx.gpu, &view, size, ui_frame);
                    }
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
    let (user, last) = parse_account(&std::fs::read_to_string(account_path()).unwrap_or_default());
    let mut launcher = Launcher { game: game.to_string(), settings, gfx: None, stage: Stage::Form, user, last, password: String::new(), server: std::env::var("MAO_SERVER").unwrap_or_else(|_| session::RUBI_KA.to_string()), message: String::new(), answer: None, chosen: None, selected: None, shot: std::env::var("MAO_LAUNCHER_SHOT").ok(), looks: crate::look::load() };
    // MAO_LAUNCHER_DEMO=1: three sample characters instead of a login (for pictures of the character screen).
    if std::env::var_os("MAO_LAUNCHER_DEMO").is_some() {
        // (the characters of a capture's list, as the server sent them: name, breed, sex, profession, level)
        let sample = |name: &str, breed, sex, profession, level| Character { name: name.into(), breed, sex, profession, level, created: true, area: "area unknown".into(), ..Default::default() };
        let list = vec![sample("Brasscasings", 1, 2, 1, 25), sample("Arctangent", 1, 3, 6, 40), sample("Beetlez", 2, 2, 2, 8), sample("Pregnantmule", 4, 1, 5, 160), sample("Diabete5", 3, 3, 10, 60), sample("Fallacy", 2, 3, 15, 220)];
        // (Diabete5's look is the one its appearance update carried in the capture)
        launcher.looks = vec![crate::look::Look {
            name: "Diabete5".into(),
            breed: 3,
            sex: 3,
            clothes: vec![("hands".into(), 8816), ("body".into(), 303241), ("feet".into(), 303243), ("arms".into(), 303240), ("legs".into(), 303245)],
            textures: Vec::new(),
            gear: vec![(0, 0x4e39, 0), (0, 0x36aa4, 0), (1, 0x409f7, 0), (2, 0x409f7, 0), (3, 0x2cf7, 0x4a0b1), (4, 0x2cf7, 0x4a0b1), (5, 0x2cff, 0x4a0b1)],
        }];
        // (and whatever the live client has written down on this machine)
        launcher.looks.extend(crate::look::load());
        launcher.selected = Some(std::env::var("MAO_LAUNCHER_PICK").ok().filter(|n| list.iter().any(|c| &c.name == n)).unwrap_or_else(|| list[0].name.clone()));
        launcher.stage = Stage::Characters(list);
    }
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
    fn a_character_is_described_by_level_sex_breed_and_profession() {
        // Laxative of the captured list: level 220, profession 9, breed 1, sex 2
        let c = Character { id: 22488, created: true, name: "Laxative".into(), breed: 1, sex: 2, profession: 9, level: 220, area: "area unknown".into(), ..Default::default() };
        assert_eq!(describe(&c), "level 220 male Solitus Enforcer");
        let c = Character { breed: 3, sex: 3, profession: 10, level: 60, area: "Old Athen".into(), ..c };
        assert_eq!(describe(&c), "level 60 female Nanomage Doctor, Old Athen");
    }

    #[test]
    fn the_account_file_holds_the_name_and_the_last_character() {
        assert_eq!(parse_account("dracul
Laxative
"), ("dracul".to_string(), "Laxative".to_string()));
        assert_eq!(parse_account("dracul"), ("dracul".to_string(), String::new()));
        assert_eq!(parse_account(""), (String::new(), String::new()));
        let list = vec![Character { name: "Abe".into(), ..Default::default() }, Character { name: "Zed".into(), ..Default::default() }, Character { name: "Cy".into(), ..Default::default() }];
        let names: Vec<&str> = ordered(&list, "zed").iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["Zed", "Abe", "Cy"]);
        assert_eq!(ordered(&list, "").iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["Abe", "Zed", "Cy"]);
    }

    #[test]
    fn professions_have_names() {
        assert_eq!(profession(9), "Enforcer");
        assert_eq!(profession(15), "Shade");
        assert_eq!(profession(99), "Unknown profession");
    }
}
