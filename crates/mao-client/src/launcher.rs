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

/// What `account.ini` holds: the account's name, on a second line the character played last, and on the lines after the other
/// accounts used before (the original's username list).
pub fn parse_account(text: &str) -> (String, String, Vec<String>) {
    let mut lines = text.lines().map(str::trim);
    let (user, last) = (lines.next().unwrap_or("").to_string(), lines.next().unwrap_or("").to_string());
    let others: Vec<String> = lines.filter(|l| !l.is_empty() && !l.eq_ignore_ascii_case(&user)).map(str::to_string).collect();
    (user, last, others)
}

/// The text of `account.ini` for an account, the character played last and the other accounts.
pub fn render_account(user: &str, last: &str, others: &[String]) -> String {
    let mut out = format!("{user}\n{last}\n");
    for o in others.iter().filter(|o| !o.eq_ignore_ascii_case(user)) {
        out += &format!("{o}\n");
    }
    out
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
    /// Making a character (`charcreate`), the list to come back to.
    Create(Box<crate::charcreate::Creating>, Vec<Character>),
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
    /// The other accounts used before (the username list).
    accounts: Vec<String>,
    /// The interface colours (from the game's own skin when it is there).
    palette: crate::theme::Palette,
    password: String,
    server: String,
    message: String,
    answer: Option<Receiver<Result<Vec<Character>, String>>>,
    /// The character to play.
    chosen: Option<String>,
    /// Quit was pressed.
    quit: bool,
    /// A character just made: once the list comes back with it, it is played (the original logs a new character straight in,
    /// `Client_t::ProcessMessage`'s 0x11).
    play_new: Option<String>,
    /// The character played is a new one (the loading screen's `welcome_to_rubika.jpg`).
    new_character: bool,
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
        if !self.accounts.iter().any(|a| a.eq_ignore_ascii_case(&user)) {
            self.accounts.push(user.clone());
        }
        let _ = std::fs::write(account_path(), render_account(&user, &self.last, &self.accounts));
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

    /// The character screen as the original's (`Views/CharacterSelectionWindow.xml`, `CharacterSelectionItem.xml`): the list in a
    /// 276 px panel at the right (the selected character's name in red with its level, gender, breed, profession and location; the
    /// others in the border colour with one summary line), New Character under it; a bar across the bottom with Back at the left
    /// and Delete at the right, and Play centred above the bar.
    fn character_screen(&mut self, ctx: &egui::Context, list: &[Character], play: &mut Option<String>, back: &mut bool, new_character: &mut bool) {
        let palette = self.palette.clone();
        let col = |c: [u8; 3]| egui::Color32::from_rgb(c[0], c[1], c[2]);
        let (accent, text) = (col(palette.accent), col(palette.text));
        let red = egui::Color32::from_rgb(0xee, 0x44, 0x44);
        let skin = crate::skin::skin(ctx).filter(|s| s.has("GFX_GUI_TAB_BORDER_TL"));
        let shell = crate::font::shell(ctx);
        let chat = crate::font::chat(ctx);
        let screen = ctx.content_rect();
        // the panel: the tab border pieces around a tab background, 90% opaque
        let panel_width = 276.0;
        let panel = egui::Rect::from_min_max(egui::pos2(screen.right() - 15.0 - panel_width, screen.top() + 15.0), egui::pos2(screen.right() - 15.0, screen.bottom() - 130.0));
        egui::Area::new(egui::Id::unique("characters")).fixed_pos(panel.min).show(ctx, |ui| {
            ui.set_clip_rect(panel.expand(2.0));
            let painter = ui.painter().clone();
            let fill = egui::Color32::from_rgba_unmultiplied(palette.panel[0], palette.panel[1], palette.panel[2], 230);
            painter.rect_filled(panel, 2.0, fill);
            match &skin {
                Some(skin) => {
                    let sz = |n: &str| skin.get(&format!("GFX_GUI_TAB_BORDER_{n}")).map_or(egui::vec2(2.0, 2.0), |s| s.size);
                    let (tl, tr, bl, br) = (sz("TL"), sz("TR"), sz("BL"), sz("BR"));
                    let (l, t, r, b) = (panel.left(), panel.top(), panel.right(), panel.bottom());
                    skin.draw_in(&painter, "GFX_GUI_TAB_BORDER_TL", egui::Rect::from_min_size(egui::pos2(l, t), tl), accent);
                    skin.draw_in(&painter, "GFX_GUI_TAB_BORDER_TOP", egui::Rect::from_min_max(egui::pos2(l + tl.x, t), egui::pos2(r - tr.x, t + sz("TOP").y)), accent);
                    skin.draw_in(&painter, "GFX_GUI_TAB_BORDER_TR", egui::Rect::from_min_size(egui::pos2(r - tr.x, t), tr), accent);
                    skin.draw_in(&painter, "GFX_GUI_TAB_BORDER_LEFT", egui::Rect::from_min_max(egui::pos2(l, t + tl.y), egui::pos2(l + sz("LEFT").x, b - bl.y)), accent);
                    skin.draw_in(&painter, "GFX_GUI_TAB_BORDER_RIGHT", egui::Rect::from_min_max(egui::pos2(r - sz("RIGHT").x, t + tr.y), egui::pos2(r, b - br.y)), accent);
                    skin.draw_in(&painter, "GFX_GUI_TAB_BORDER_BL", egui::Rect::from_min_size(egui::pos2(l, b - bl.y), bl), accent);
                    skin.draw_in(&painter, "GFX_GUI_TAB_BORDER_BOTTOM", egui::Rect::from_min_max(egui::pos2(l + bl.x, b - sz("BOTTOM").y), egui::pos2(r - br.x, b)), accent);
                    skin.draw_in(&painter, "GFX_GUI_TAB_BORDER_BR", egui::Rect::from_min_size(egui::pos2(r - br.x, b - br.y), br), accent);
                }
                None => {
                    painter.rect_stroke(panel, 2.0, egui::Stroke::new(1.0, accent), egui::StrokeKind::Inside);
                }
            }
            let inner = panel.shrink2(egui::vec2(13.0, 10.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
            let ui = &mut child;
            // a line of text in the original's font, else egui's
            let line = |ui: &mut egui::Ui, text: &str, font: &Option<std::sync::Arc<crate::font::BitmapFont>>, size: f32, color: egui::Color32| -> egui::Response {
                match font {
                    Some(f) => f.label(ui, text, color),
                    None => ui.label(egui::RichText::new(text).size(size).color(color)),
                }
            };
            let list_height = inner.height() - 70.0;
            crate::chrome::scroll(ui, &palette, egui::ScrollArea::vertical().max_height(list_height).auto_shrink([false, false]), |ui| {
                for c in ordered(list, &self.last) {
                    let usable = c.created && !c.banned;
                    let picked = self.selected.as_deref() == Some(c.name.as_str());
                    // the name: a text button in the border colour, white under the pointer, red when it is the one picked
                    let name_width = shell.as_ref().map_or(0.0, |f| f.width(&c.name) * 1.5) + 8.0;
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(name_width.max(120.0)), 20.0), egui::Sense::click());
                    let color = if picked { red } else if response.hovered() { egui::Color32::WHITE } else if usable { accent } else { egui::Color32::GRAY };
                    match &shell {
                        Some(f) => {
                            // (the original's HUGE font: the shell font at one and a half times)
                            let w = f.width(&c.name);
                            let painter = ui.painter().with_clip_rect(rect);
                            let mut mesh_pos = rect.left_top();
                            mesh_pos.y += 2.0;
                            // paint at 1.5x by painting into a scaled rect: the font paints at pixel size, so draw it and let a second pass widen it
                            f.paint(&painter, mesh_pos, &c.name, color);
                            let _ = w;
                        }
                        None => {
                            ui.painter().text(rect.left_top(), egui::Align2::LEFT_TOP, &c.name, egui::FontId::proportional(18.0), color);
                        }
                    }
                    if response.clicked() && usable {
                        self.selected = Some(c.name.clone());
                    }
                    if response.double_clicked() && usable {
                        *play = Some(c.name.clone());
                    }
                    if picked {
                        // the detailed view: label and value on each row
                        let rows: Vec<(&str, String)> = vec![
                            ("Level:", c.level.to_string()),
                            ("Gender:", { let mut g = sex(c.sex).to_string(); if let Some(f) = g.get_mut(0..1) { f.make_ascii_uppercase(); } g }),
                            ("Breed:", breed(c.breed).to_string()),
                            ("Profession:", profession(c.profession).to_string()),
                            ("Location:", c.area.trim().to_string()),
                        ];
                        for (label, value) in rows {
                            let (row, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 12.0), egui::Sense::hover());
                            match &chat {
                                Some(f) => {
                                    f.paint(ui.painter(), row.left_top(), label, text);
                                    f.paint(ui.painter(), egui::pos2(row.right() - f.width(&value), row.top()), &value, text);
                                }
                                None => {
                                    ui.painter().text(row.left_top(), egui::Align2::LEFT_TOP, label, egui::FontId::proportional(11.0), text);
                                    ui.painter().text(row.right_top(), egui::Align2::RIGHT_TOP, &value, egui::FontId::proportional(11.0), text);
                                }
                            }
                        }
                        if c.banned {
                            line(ui, &format!("Status: banned {}", c.ban_reason.trim()), &chat, 11.0, red);
                        }
                    } else {
                        let summary = format!("Level {} {} {}{}", c.level, breed(c.breed), profession(c.profession), if c.banned { " (banned)" } else if !c.created { " (not created)" } else { "" });
                        line(ui, &summary, &chat, 11.0, text);
                    }
                    ui.add_space(8.0);
                }
            });
            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                if ui.add(egui::Button::new("New Character")).clicked() {
                    *new_character = true;
                }
                line(ui, &format!("{} characters", list.len()), &chat, 11.0, text);
            });
        });
        // the bottom bar: Back at the left, Delete at the right, Play above its middle
        let bar = egui::Rect::from_min_max(egui::pos2(screen.left(), screen.bottom() - 46.0), egui::pos2(screen.right(), screen.bottom()));
        egui::Area::new(egui::Id::unique("bottom-bar")).fixed_pos(bar.min).show(ctx, |ui| {
            ui.painter().rect_filled(bar, 0.0, egui::Color32::from_rgba_unmultiplied(palette.panel[0], palette.panel[1], palette.panel[2], 235));
            ui.painter().line_segment([bar.left_top(), bar.right_top()], egui::Stroke::new(1.0, accent));
            let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink2(egui::vec2(30.0, 12.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            if inner.button("Back").clicked() {
                *back = true;
            }
            inner.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled(false, egui::Button::new("Delete")).on_disabled_hover_text("Deleting a character is not in this client: use the original client for it");
            });
        });
        egui::Area::new(egui::Id::unique("play")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -56.0)).show(ctx, |ui| {
            if ui.add_enabled(self.selected.is_some(), egui::Button::new(egui::RichText::new("Play").size(14.0)).min_size(egui::vec2(60.0, 24.0))).clicked() {
                *play = self.selected.clone();
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
                        if let Some(name) = self.play_new.take().filter(|n| list.iter().any(|c| &c.name == n)) {
                            let _ = std::fs::write(account_path(), render_account(self.user.trim(), &name, &self.accounts));
                            self.selected = Some(name.clone());
                            self.chosen = Some(name);
                            self.new_character = true;
                        }
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
        let mut new_character = false;
        if let Stage::Characters(list) = &self.stage {
            let list = list.clone();
            self.character_screen(ctx, &list, &mut play, &mut back, &mut new_character);
        }
        if new_character {
            if let Stage::Characters(list) = &self.stage {
                self.stage = Stage::Create(Box::default(), list.clone());
            }
        }
        // (making a character: the four steps; made, the list is fetched again with it selected)
        let account = crate::charcreate::Account {
            user: self.user.trim().to_string(),
            password: self.password.clone(),
            server: self.server.trim().to_string(),
            version: std::fs::read_to_string(std::path::Path::new(&self.game).join("version.id")).map(|t| t.lines().next().unwrap_or("").trim().to_string()).unwrap_or_default(),
        };
        let palette_now = self.palette.clone();
        let game = self.game.clone();
        let mut made = None;
        if let Stage::Create(creating, list) = &mut self.stage {
            match creating.show(ctx, &palette_now, &account, &game) {
                crate::charcreate::Outcome::Stay => {}
                crate::charcreate::Outcome::Back => self.stage = Stage::Characters(list.clone()),
                crate::charcreate::Outcome::Created(name) => made = Some(name),
            }
        }
        if let Some(name) = made {
            self.play_new = Some(name);
            self.log_in();
        }
        if !matches!(self.stage, Stage::Characters(_) | Stage::Create(..)) {
            // The login dialog as the original's (`Views/LoginWindow.xml`): the username list with Remove, the password, Login and Quit.
            let palette = self.palette.clone();
            let mut open = true;
            let mut quit = false;
            crate::chrome::window(ctx, &palette, "Login", "Login", &mut open, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).default_width(300.0), |ui| match &self.stage {
                Stage::Form => {
                    egui::Grid::new("login").num_columns(3).spacing(egui::vec2(8.0, 6.0)).show(ui, |ui| {
                        ui.label("Username:");
                        let mut names = self.accounts.clone();
                        if !self.user.trim().is_empty() && !names.iter().any(|n| n.eq_ignore_ascii_case(self.user.trim())) {
                            names.insert(0, self.user.trim().to_string());
                        }
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.user).desired_width(110.0));
                            egui::ComboBox::from_id_salt("accounts").selected_text("").width(14.0).show_ui(ui, |ui| {
                                for name in &names {
                                    if ui.selectable_label(self.user.trim() == name, name).clicked() {
                                        self.user = name.clone();
                                    }
                                }
                            });
                        });
                        if ui.add_enabled(!self.user.trim().is_empty(), egui::Button::new("Remove")).on_hover_text("Forget this account name").clicked() {
                            let user = self.user.trim().to_string();
                            self.accounts.retain(|a| !a.eq_ignore_ascii_case(&user));
                            self.user = self.accounts.first().cloned().unwrap_or_default();
                            let _ = std::fs::write(account_path(), render_account(&self.user, &self.last, &self.accounts));
                        }
                        ui.end_row();
                        ui.label("Password:");
                        let second = ui.add(egui::TextEdit::singleline(&mut self.password).password(true).desired_width(150.0));
                        if second.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submit = true;
                        }
                        ui.end_row();
                        if self.server.trim() != session::RUBI_KA {
                            ui.label("Server:");
                            ui.add(egui::TextEdit::singleline(&mut self.server).desired_width(150.0));
                            ui.end_row();
                        }
                    });
                    if !self.message.is_empty() {
                        ui.colored_label(egui::Color32::from_rgb(240, 120, 110), &self.message);
                    }
                    ui.add_space(4.0);
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - 120.0).max(0.0) * 0.5);
                            if ui.add(egui::Button::new("Login").min_size(egui::vec2(52.0, 20.0))).clicked() {
                                submit = true;
                            }
                            if ui.add(egui::Button::new("Quit").min_size(egui::vec2(52.0, 20.0))).clicked() {
                                quit = true;
                            }
                        });
                    });
                }
                Stage::Busy => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Logging in...");
                    });
                }
                Stage::Characters(_) | Stage::Create(..) => {}
            });
            if quit || !open {
                self.quit = true;
            }
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
                let _ = std::fs::write(account_path(), render_account(self.user.trim(), name, &self.accounts));
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
                        // (the breed being made, as the list shows a character)
                        Stage::Create(creating, _) => creating.breed_and_sex().map(|(breed, sex)| Character { name: format!("(new {breed} {sex} {} {} {})", creating.head, creating.build, creating.height), breed, sex, width: creating.build, height: crate::charcreate::height(creating.height), ..Default::default() }),
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
                                // (a character being made wears its chosen head: the look's gear at attachment 0)
                                let look = match &self.stage {
                                    Stage::Create(creating, _) => creating.head_mesh(&self.game).map(|mesh| crate::look::Look { name: c.name.clone(), breed: c.breed as u8, sex: c.sex as u8, clothes: Vec::new(), textures: Vec::new(), gear: vec![(0, mesh, 0)] }),
                                    _ => crate::look::find(&self.looks, &c.name),
                                };
                                let shaped = match &self.stage {
                                    Stage::Create(..) => crate::net::character_model_shaped(db, c.breed, c.sex, look.as_ref(), c.width, c.height as f32),
                                    _ => crate::net::character_model(db, c.breed, c.sex, look.as_ref()),
                                };
                                if let Some(model) = shaped {
                                    let mut scene = gfx.renderer.upload(&gfx.gpu, &model, &mut |k, i| textures.get_for_upload(k, i));
                                    scene.sky = ([0.05, 0.09, 0.15], [0.01, 0.03, 0.07]);
                                    scene.fog = (400.0, 800.0);
                                    let mut camera = Camera::fit(&scene);
                                    // (the whole figure, from a little above its middle)
                                    camera.distance = scene.radius * 3.0;
                                    // (a character being made: the camera as for a normal height, so a short or tall one shows)
                                    if matches!(self.stage, Stage::Create(..)) && c.height > 0 {
                                        let k = 100.0 / c.height as f32;
                                        camera.distance *= k;
                                        camera.target.z *= k;
                                    }
                                    camera.pitch = 0.12;
                                    gfx.camera = Some(camera);
                                    gfx.scene = Some(scene);
                                }
                            }
                        }
                    }
                    gfx.frames += 1;
                    // (a picture of a screen with no character on it: an empty scene gives the background)
                    if self.shot.is_some() && gfx.scene.is_none() && gfx.frames >= 12 {
                        let mut scene = gfx.renderer.upload(&gfx.gpu, &mao_formats::model::Model::default(), &mut |_, _| None);
                        scene.sky = ([0.05, 0.09, 0.15], [0.01, 0.03, 0.07]);
                        scene.fog = (400.0, 800.0);
                        let mut camera = Camera::fit(&scene);
                        camera.distance = 10.0;
                        gfx.camera = Some(camera);
                        gfx.scene = Some(scene);
                    }
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
                    if self.chosen.is_some() || self.quit {
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
/// The exit code of a game that ended because the character logged out (`/camp`, sitting for 30 s): the launcher shows the
/// character screen again.
pub const LOGGED_OUT: i32 = 42;

pub fn run(game: &str) -> Result<(), String> {
    crate::theme::set_game_dir(game);
    let settings = Settings::load();
    crate::font::set_enabled(settings.original_font);
    let palette = crate::theme::load(&settings.theme, &settings.aou_mod);
    let (user, last, accounts) = parse_account(&std::fs::read_to_string(account_path()).unwrap_or_default());
    let mut launcher = Launcher { game: game.to_string(), settings, gfx: None, stage: Stage::Form, user, last, accounts, palette, quit: false, password: String::new(), server: std::env::var("MAO_SERVER").unwrap_or_else(|_| session::RUBI_KA.to_string()), message: String::new(), answer: None, chosen: None, play_new: None, new_character: false, selected: None, shot: std::env::var("MAO_LAUNCHER_SHOT").ok(), looks: crate::look::load() };
    // MAO_LAUNCHER_DEMO=1: three sample characters instead of a login (for pictures of the character screen).
    if std::env::var_os("MAO_LAUNCHER_DEMO").is_some_and(|v| v != "form") {
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
        launcher.stage = Stage::Characters(list.clone());
        // (MAO_LAUNCHER_CREATE=<step 0-3>: the New Character screens at that step, a Nanomage female Metaphysicist chosen)
        if let Some(step) = std::env::var("MAO_LAUNCHER_CREATE").ok().and_then(|v| v.parse().ok()) {
            let mut creating = crate::charcreate::Creating::default();
            (creating.step, creating.breed, creating.profession, creating.name) = (step, 5, 1, "Zibblet".into());
            creating.head = std::env::var("MAO_LAUNCHER_HEAD").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
            creating.build = std::env::var("MAO_LAUNCHER_BUILD").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
            creating.height = std::env::var("MAO_LAUNCHER_HEIGHT").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
            launcher.stage = Stage::Create(Box::new(creating), list);
        }
    }
    // After a logout the launcher starts again with the account (MAO_RELOGIN): it logs in at once and lists the characters.
    if std::env::var_os("MAO_RELOGIN").is_some() {
        launcher.password = std::env::var("MAO_PASSWORD").unwrap_or_default();
        launcher.server = std::env::var("MAO_SERVER").unwrap_or(launcher.server);
        launcher.user = std::env::var("MAO_USER").unwrap_or(launcher.user);
        launcher.log_in();
    }
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.run_app(&mut launcher).map_err(|e| e.to_string())?;
    let Some(name) = launcher.chosen.take() else { return Ok(()) };
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let status = std::process::Command::new(&exe)
        .args(["net", game, "enter", &name, "--live"])
        .env("MAO_USER", launcher.user.trim())
        .env("MAO_PASSWORD", &launcher.password)
        .env("MAO_SERVER", launcher.server.trim())
        .env("MAO_NEW_CHARACTER", if launcher.new_character { "1" } else { "" })
        .status()
        .map_err(|e| e.to_string())?;
    if status.code() == Some(LOGGED_OUT) {
        // (a new process: an event loop cannot be made twice in one)
        std::process::Command::new(&exe)
            .args([game, "play"])
            .env("MAO_RELOGIN", "1")
            .env("MAO_USER", launcher.user.trim())
            .env("MAO_PASSWORD", &launcher.password)
            .env("MAO_SERVER", launcher.server.trim())
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    } else if status.success() {
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
"), ("dracul".to_string(), "Laxative".to_string(), vec![]));
        assert_eq!(parse_account("dracul"), ("dracul".to_string(), String::new(), vec![]));
        assert_eq!(parse_account(""), (String::new(), String::new(), vec![]));
        // the other accounts follow, the current one is not repeated among them
        let (user, last, others) = parse_account("dracul\nLaxative\nother\ndracul\nthird\n");
        assert_eq!((user.as_str(), last.as_str(), others.clone()), ("dracul", "Laxative", vec!["other".to_string(), "third".to_string()]));
        assert_eq!(render_account(&user, &last, &others), "dracul\nLaxative\nother\nthird\n");
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
