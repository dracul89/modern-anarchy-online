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
    /// The hall behind the login (`hall.rs`): its scene and the place it is seen from; `Some(None)` once it was looked
    /// for and not found.
    hall: Option<Option<(Scene, crate::hall::Place)>>,
    /// The character list's character stands in the hall (the hall scene's overlay holds it).
    hall_character: bool,
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
    /// The screens on the skin's views (`loginviews.rs`).
    login_view: crate::loginviews::LoginWindow,
    char_view: crate::loginviews::CharSelectScreen,
    /// The progress window while logging in.
    progress: crate::loginviews::ProgressWindow,
    /// A deletion waiting on the login server.
    deleting: Option<Receiver<Result<u32, String>>>,
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
        let version = mao_formats::files::read_to_string(std::path::Path::new(&self.game).join("version.id")).map(|t| t.lines().next().unwrap_or("").trim().to_string());
        let (tx, rx) = channel();
        self.answer = Some(rx);
        self.stage = Stage::Busy;
        // (the time-out of the launcher's Connection page when it was ever saved, else the game's own preference)
        let timeout = std::fs::read_to_string(crate::dimensions::Prefs::path()).ok().map(|t| crate::dimensions::Prefs::from_text(&t).network_timeout as f32);
        self.progress.start(timeout.unwrap_or_else(|| crate::loginviews::network_timeout("GeneralNetworkTimeout", 90.0)));
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

    /// Deletes a character (system 0x14, answered by 0x15) on a login connection of its own; the list loses it once the
    /// server says so.
    fn delete(&mut self, id: u32) {
        let (user, password, server) = (self.user.trim().to_string(), self.password.clone(), self.server.trim().to_string());
        let version = mao_formats::files::read_to_string(std::path::Path::new(&self.game).join("version.id")).map(|t| t.lines().next().unwrap_or("").trim().to_string()).unwrap_or_default();
        let (tx, rx) = channel();
        self.deleting = Some(rx);
        std::thread::spawn(move || {
            let result = (|| -> Result<u32, String> {
                let mut connection = Connection::connect(server.as_str()).map_err(|e| e.to_string())?;
                session::login(&mut connection, &user, &password, &version).map_err(|e| e.to_string())?;
                let reply = session::ask_creation(&mut connection, &mao_net::create::delete_character(id)).map_err(|e| e.to_string());
                connection.close();
                match reply? {
                    mao_net::create::Reply::Deleted(gone) => Ok(gone),
                    other => Err(format!("the server answered {other:?}")),
                }
            })();
            let _ = tx.send(result);
        });
    }

    fn screen(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.answer {
            if let Ok(result) = rx.try_recv() {
                self.answer = None;
                self.progress.stop();
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
        // (a deletion answered: the character leaves the list)
        if let Some(rx) = &self.deleting
            && let Ok(result) = rx.try_recv()
        {
            self.deleting = None;
            match result {
                Ok(gone) => {
                    if let Stage::Characters(list) = &mut self.stage {
                        list.retain(|c| c.id != gone);
                    }
                    self.selected = None;
                }
                Err(e) => self.char_view.status = format!("Deleting failed: {e}"),
            }
        }
        if let Stage::Characters(list) = &self.stage {
            let list = list.clone();
            let shown = ordered(&list, &self.last);
            let palette = self.palette.clone();
            match self.char_view.show(ctx, &palette, &shown, &mut self.selected) {
                Some(crate::loginviews::CharAction::Play(name)) => play = Some(name),
                Some(crate::loginviews::CharAction::Back) => back = true,
                Some(crate::loginviews::CharAction::New) => new_character = true,
                Some(crate::loginviews::CharAction::Delete(id)) => self.delete(id),
                None => {}
            }
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
            version: mao_formats::files::read_to_string(std::path::Path::new(&self.game).join("version.id")).map(|t| t.lines().next().unwrap_or("").trim().to_string()).unwrap_or_default(),
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
        // While logging in, the progress window in the login window's place (`LoginModule_c::Show(1)`); cancelled or
        // timed out, back to the form (the answer, if it still comes, is not waited for).
        if matches!(self.stage, Stage::Busy) && self.answer.is_some() {
            let palette = self.palette.clone();
            if self.progress.show(ctx, &palette) {
                self.answer = None;
                self.play_new = None;
                self.stage = Stage::Form;
            }
        } else if !matches!(self.stage, Stage::Characters(_) | Stage::Create(..)) {
            // The login dialog (`Views/LoginWindow.xml`, `loginviews.rs`).
            let palette = self.palette.clone();
            let busy = matches!(self.stage, Stage::Busy);
            let accounts = self.accounts.clone();
            match self.login_view.show(ctx, &palette, &mut self.user, &mut self.password, &accounts, &self.message, busy) {
                Some(crate::loginviews::LoginAction::Login) => submit = true,
                Some(crate::loginviews::LoginAction::Quit) => self.quit = true,
                Some(crate::loginviews::LoginAction::Remove(user)) => {
                    self.accounts.retain(|a| !a.eq_ignore_ascii_case(&user));
                    self.user = self.accounts.first().cloned().unwrap_or_default();
                    let _ = std::fs::write(account_path(), render_account(&self.user, &self.last, &self.accounts));
                }
                None => {}
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

impl Launcher {
    /// The hall's scene, built the first time it is asked for (after the window's first frames, so the window is there
    /// before the game's database is opened).
    fn hall_ready(&self, gfx: &mut Gfx) -> bool {
        if gfx.hall.is_none() && gfx.frames >= 2 && self.shot.is_none() {
            if gfx.db.is_none() {
                if let Ok(db) = ResourceDatabase::open_game(std::path::Path::new(&self.game)) {
                    let db = Arc::new(db);
                    gfx.db = Some((db.clone(), crate::Textures::new(db)));
                }
            }
            let built = gfx.db.as_ref().and_then(|(db, textures)| {
                let place = crate::hall::login_place();
                let model = crate::hall::model(db)?;
                let mut scene = gfx.renderer.upload(&gfx.gpu, &model, &mut |k, i| textures.get_for_upload(k, i));
                scene.sky = ([0.02, 0.03, 0.05], [0.0, 0.0, 0.0]);
                scene.fog = (900.0, 1000.0);
                Some((scene, place))
            });
            gfx.hall = Some(built);
        }
        matches!(gfx.hall, Some(Some(_)))
    }
}

impl ApplicationHandler for Launcher {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        // (the login is the game's window already in the original: the size and the mode of the settings; the pictures the
        // tests take keep the small window)
        let small = self.shot.is_some();
        let size = if small { PhysicalSize::new(1000, 640) } else { PhysicalSize::new(self.settings.width, self.settings.height) };
        let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("Modern Anarchy Online").with_window_icon(crate::icon::window_icon()).with_inner_size(size)).expect("a window"));
        if !small {
            crate::viewer::apply_mode(&window, self.settings.mode, self.settings.monitor);
        }
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("a surface");
        let gpu = Gpu::new(instance, Some(&surface)).expect("a graphics device");
        let size = window.inner_size();
        let config = surface.get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1)).expect("a surface format");
        surface.configure(&gpu.device, &config);
        // (a picture of the screen is drawn off screen in a format both the scene and the interface can use)
        let format = if self.shot.is_some() { wgpu::TextureFormat::Rgba8UnormSrgb } else { config.format };
        let ui = Ui::new(&window, &gpu, format, &self.settings);
        let mut renderer = Renderer::new(&gpu, format);
        renderer.compress_textures = self.settings.texture_compression;
        window.request_redraw();
        self.gfx = Some(Gfx { window, surface, config, gpu, ui, renderer, shown: None, scene: None, camera: None, db: None, hall: None, hall_character: false, started: std::time::Instant::now(), frames: 0 });
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
                    // (the hall, once it is there: the character list's character stands in it; the first time it is
                    // there, whoever was shown alone is built again to stand there)
                    let had_hall = matches!(gfx.hall, Some(Some(_)));
                    let hall = self.hall_ready(&mut gfx);
                    if hall && !had_hall {
                        gfx.shown = None;
                    }
                    if want.as_ref().map(|c| &c.name) != gfx.shown.as_ref() {
                        gfx.shown = want.as_ref().map(|c| c.name.clone());
                        gfx.scene = None;
                        // (whoever stood in the hall leaves it)
                        if std::mem::take(&mut gfx.hall_character) {
                            if let Some(Some((scene, _))) = gfx.hall.as_mut() {
                                gfx.renderer.reupload_overlay(&gfx.gpu, scene, &mao_formats::model::Model::default(), &mut |_, _| None);
                            }
                        }
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
                                // (the character list's character stands in the hall where the original's viewer puts it,
                                // `hall::stand`; a character being made is still shown alone)
                                let in_hall = hall && matches!(self.stage, Stage::Characters(_));
                                if let (true, Some(model), Some(Some((scene, _)))) = (in_hall, shaped.as_ref(), gfx.hall.as_mut()) {
                                    gfx.renderer.reupload_overlay(&gfx.gpu, scene, &crate::hall::stand(model.clone()), &mut |k, i| textures.get_for_upload(k, i));
                                    gfx.hall_character = true;
                                } else if let Some(model) = shaped {
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
                    } else if hall {
                        let Some(Some((scene, place))) = &gfx.hall else { unreachable!() };
                        // (the hall from the login world's camera, with the picked character in it when there is one)
                        let p = place.position;
                        let forward = crate::hall::rotate(place.rotation, [0.0, 0.0, 1.0]);
                        // (the game's frame has Y up, the renderer's Z)
                        let eye = mao_render::glam::Vec3::new(p[0], p[2], p[1]);
                        let target = eye + mao_render::glam::Vec3::new(forward[0], forward[2], forward[1]) * 10.0;
                        let mut camera = Camera::looking(eye, target);
                        let aspect = size.0 as f32 / size.1.max(1) as f32;
                        // (`MAO_HALL_ORIGINAL=1`: the original's rule, the angle across the window, to compare with its pictures)
                        camera.half_height = Some(if std::env::var_os("MAO_HALL_ORIGINAL").is_some() { (place.fov_degrees.to_radians() * 0.5).tan() / aspect } else { crate::hall::half_height(place.fov_degrees, aspect) });
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
    mao_render::prefer_adapter(&settings.adapter);
    let palette = crate::theme::load(&settings.theme, &settings.aou_mod);
    let (user, last, accounts) = parse_account(&std::fs::read_to_string(account_path()).unwrap_or_default());
    let mut launcher = Launcher { game: game.to_string(), settings, gfx: None, stage: Stage::Form, user, last, accounts, palette, quit: false, password: String::new(), server: std::env::var("MAO_SERVER").unwrap_or_else(|_| session::RUBI_KA.to_string()), message: String::new(), answer: None, chosen: None, play_new: None, new_character: false, selected: None, shot: std::env::var("MAO_LAUNCHER_SHOT").ok(), looks: crate::look::load(), login_view: Default::default(), char_view: Default::default(), progress: Default::default(), deleting: None };
    // MAO_LAUNCHER_DEMO=1: three sample characters instead of a login (for pictures of the character screen).
    // MAO_LAUNCHER_DEMO=progress: logging in, an answer that never comes (the progress window).
    if std::env::var_os("MAO_LAUNCHER_DEMO").is_some_and(|v| v == "progress") {
        let (tx, rx) = channel();
        std::mem::forget(tx);
        launcher.answer = Some(rx);
        launcher.stage = Stage::Busy;
        launcher.progress.start(20.0);
    } else if std::env::var_os("MAO_LAUNCHER_DEMO").is_some_and(|v| v != "form") {
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
            // (MAO_LAUNCHER_BREED / _PROFESSION: other choices than the Nanomage female Meta-Physicist)
            creating.breed = std::env::var("MAO_LAUNCHER_BREED").ok().and_then(|v| v.parse().ok()).unwrap_or(creating.breed);
            creating.profession = std::env::var("MAO_LAUNCHER_PROFESSION").ok().and_then(|v| v.parse().ok()).unwrap_or(creating.profession);
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
    // (the login's window goes before the game's comes)
    launcher.gfx = None;
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
        // (this run is over before the next starts, or the next would take it for one that never ended: `crash.rs`)
        crate::crash::end("launcher", None);
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
