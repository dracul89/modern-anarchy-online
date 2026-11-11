//! The launcher (`mao-client <game> launch`): the window the original's `Anarchy.exe` opens before the game.
//!
//! What the original's is made of, and what is done with each here:
//!
//! - A window of 794 by 575 with one of the pictures of `cd_image/data/launcher/skins/backgrounds` over all of it (the
//!   art is along the bottom). The same picture here, one of those the folder has.
//! - In its upper 428 pixels a web page (an embedded browser control at `SHOWURL` of
//!   `cd_image/data/launcher/AnarchyLauncher.url`: the banner, the row of links, the customer service messages and the
//!   tip of the day are all that page's). The same page here in a WebView2 child (wry, as the in-game pages); a link
//!   that asks for a new window goes to the system's browser.
//! - A list of the dimensions, downloaded from the address in `DimensionServer.url` (`dimensions.rs`), the one chosen
//!   last selected.
//! - SETTINGS and PLAY, pictures in the program's own resources (`mao_formats::pe`: bitmaps 176, 178 and 187 are PLAY
//!   as it is, under the pointer and switched off; 181, 182 and 186 SETTINGS). Drawn with those when the game folder
//!   has `Anarchy.exe`, as plain buttons otherwise. PLAY starts the login (`mao-client <game> play`) for the chosen
//!   dimension's server and the launcher goes.
//! - The window's title with the game folder's version and the dimension's.
//!
//! The settings (the original's three pages, dialog templates 153, 161 and 107 of the program): the pages are kept,
//! their contents are this client's own where the original's no longer mean anything.
//!
//! | original | here |
//! |---|---|
//! | Rendering Device (a Direct3D device) | the graphics device wgpu draws with (`mao_render::adapter_names`), or the system's choice |
//! | Resolution: predefined, custom, Run In Window, Borderless | the same; neither box ticked is exclusive full screen; no colour depth |
//! | Texture Quality, Compressed Textures, Texture Color Depth | gone (textures are always drawn whole); in their place the edges' smoothing, the monitor's refresh, the drawing scale and how far the world is seen |
//! | Skin (a folder name) | the interface colours' theme (`theme::available`) and the skin folder |
//! | Font, size, style | the original's bitmap font on or off, and the interface's size |
//! | Folders: skins, scripts, screenshots | this client's folders: settings, screenshots, chat logs |
//! | Client-to-Server and Dimension-check time-outs | the same two, kept in `launcher.ini` (the login's progress bar and the dimension download use them) |
//! | Automatically report disconnects to Funcom | gone |
//!
//! Not done: patching (the original turns PLAY into PATCH when the versions differ; here the difference is only
//! told), the end-user licence dialog, the page of an error code (`ERRORURL`).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mao_render::{wgpu, Gpu};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::dimensions::{self, Dimension, Prefs};
use crate::settings::{Settings, WindowMode};
use crate::ui::Ui;

/// The original window's inside, and how much of its height the page takes.
const WIDTH: f32 = 794.0;
const HEIGHT: f32 = 575.0;
const PAGE_HEIGHT: f32 = 428.0;
/// Where the controls are in it (measured on the original's window).
const LIST: [f32; 4] = [403.0, 473.0, 240.0, 20.0];
const SETTINGS_BUTTON: [f32; 4] = [661.0, 470.0, 112.0, 26.0];
const PLAY_BUTTON: [f32; 4] = [661.0, 511.0, 112.0, 46.0];

/// A button's three pictures: as it is, under the pointer, switched off.
type ButtonPictures = [Option<egui::TextureHandle>; 3];

#[derive(Default)]
struct Pictures {
    background: Option<egui::TextureHandle>,
    play: ButtonPictures,
    settings: ButtonPictures,
}

/// The background pictures a game folder has.
fn backgrounds(game: &Path) -> Vec<PathBuf> {
    let dir = mao_formats::files::find(&game.join("cd_image/data/launcher/skins/backgrounds"));
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bmp"))).collect();
    files.sort();
    files
}

fn load_pictures(ctx: &egui::Context, game: &Path) -> Pictures {
    let mut pictures = Pictures::default();
    let files = backgrounds(game);
    if !files.is_empty() {
        // (which of them the original shows was not read: here one by the clock)
        let pick = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as usize) % files.len();
        if let Ok(image) = image::open(&files[pick]) {
            let image = image.to_rgba8();
            let (w, h) = image.dimensions();
            pictures.background = Some(ctx.load_texture("launcher-background", egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], image.as_raw()), egui::TextureOptions::LINEAR));
        }
    }
    if let Ok(program) = std::fs::read(mao_formats::files::find(&game.join("Anarchy.exe"))) {
        let bitmaps = mao_formats::pe::bitmaps(&program);
        let texture = |id: u32| {
            bitmaps.iter().find(|(i, _)| *i == id).map(|(_, b)| ctx.load_texture(format!("launcher-button-{id}"), egui::ColorImage::from_rgba_unmultiplied([b.width as usize, b.height as usize], &b.rgba), egui::TextureOptions::LINEAR))
        };
        pictures.play = [texture(176), texture(178), texture(187)];
        pictures.settings = [texture(181), texture(182), texture(186)];
    }
    pictures
}

/// Opens an address or a folder with the system's own program for it.
fn open_outside(what: &str) {
    #[cfg(windows)]
    let _ = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", what]).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(what).spawn();
}

/// The window modes as the original's two boxes: (run in a window, borderless).
pub fn mode_boxes(mode: WindowMode) -> (bool, bool) {
    match mode {
        WindowMode::Windowed => (true, false),
        WindowMode::Borderless => (true, true),
        WindowMode::Fullscreen => (false, false),
    }
}

pub fn boxes_mode(window: bool, borderless: bool) -> WindowMode {
    match (window, borderless) {
        (true, false) => WindowMode::Windowed,
        (true, true) => WindowMode::Borderless,
        (false, _) => WindowMode::Fullscreen,
    }
}

/// The predefined sizes: the monitor's, largest first, each once, with the size the settings have among them.
pub fn predefined(monitor: impl Iterator<Item = (u32, u32)>, current: (u32, u32)) -> Vec<(u32, u32)> {
    let mut sizes: Vec<(u32, u32)> = monitor.chain(std::iter::once(current)).filter(|s| s.0 >= 640 && s.1 >= 480).collect();
    sizes.sort_by(|a, b| (b.0 as u64 * b.1 as u64, b.0).cmp(&(a.0 as u64 * a.1 as u64, a.0)));
    sizes.dedup();
    sizes
}

/// What a folder must have to be the original game's: the resource database and the version file. The folder's version
/// when it has them, else what is missing.
pub fn check_game_dir(folder: &Path) -> Result<String, String> {
    if folder.as_os_str().is_empty() {
        return Err("No folder chosen.".into());
    }
    if !folder.is_dir() {
        return Err("There is no such folder.".into());
    }
    for needed in ["cd_image/data/db/ResourceDatabase.dat", "version.id"] {
        if !mao_formats::files::exists(folder.join(needed)) {
            return Err(format!("Not the game's folder: it has no {needed}."));
        }
    }
    let version = mao_formats::files::read_to_string(folder.join("version.id")).map(|t| t.lines().next().unwrap_or("").trim().to_string()).unwrap_or_default();
    Ok(version)
}

/// Where the game is usually installed, looked through when no folder was ever chosen.
fn usual_game_dirs() -> Vec<PathBuf> {
    // (`MAO_NO_USUAL_DIRS=1`: none, to see the launcher with no game)
    if std::env::var_os("MAO_NO_USUAL_DIRS").is_some() {
        return Vec::new();
    }
    let mut out = vec![PathBuf::from(r"C:\Funcom\Anarchy Online")];
    for base in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(base) = std::env::var_os(base).map(PathBuf::from) {
            out.push(base.join("Funcom").join("Anarchy Online"));
            out.push(base.join("Steam").join("steamapps").join("common").join("Anarchy Online"));
        }
    }
    out
}

/// The game's folder for a start of the launcher: the one named on the command line, else the settings', else the first
/// of the usual places that has the game (`found` says it was neither named nor kept).
pub fn resolve_game_dir(named: Option<&str>, kept: &str, usual: &[PathBuf]) -> Option<(String, bool)> {
    if let Some(named) = named {
        return Some((named.to_string(), false));
    }
    if check_game_dir(Path::new(kept)).is_ok() {
        return Some((kept.to_string(), false));
    }
    usual.iter().find(|p| check_game_dir(p).is_ok()).map(|p| (p.display().to_string(), true))
}

/// The window's title: the game folder's version and the chosen dimension's, as the original's has them.
pub fn title(local: &str, remote: &str) -> String {
    let or_unknown = |v: &str| if v.is_empty() { "unknown".to_string() } else { v.to_string() };
    format!("Modern Anarchy Online - Local Version: {} - Remote version: {}", or_unknown(local), or_unknown(remote))
}

/// The drawing scales offered: the share of the window's size the scene is drawn at before it is upscaled (FSR 1's own
/// steps), with their names.
pub const SCALES: [(f32, &str); 5] = [(1.0, "Native"), (0.77, "Ultra quality"), (0.67, "Quality"), (0.59, "Balanced"), (0.5, "Performance")];

/// A drawing scale's name: its step's, with the per cent; a scale that is no step (set by hand) by its per cent alone.
pub fn scale_name(scale: f32) -> String {
    let percent = (scale * 100.0).round();
    match SCALES.iter().find(|(s, _)| (s - scale).abs() < 0.005) {
        Some((_, name)) => format!("{name} ({percent:.0}%)"),
        None => format!("{percent:.0}%"),
    }
}

/// The frame limits offered (0: none; 100 is where the original stopped), and the background's (0: the same).
pub const FRAME_LIMITS: [u32; 6] = [0, 60, 100, 120, 144, 240];
pub const BACKGROUND_LIMITS: [u32; 4] = [0, 15, 30, 60];

/// The quality presets: each sets the edges' smoothing, the shadows, the drawing scale and the two distances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Low,
    Medium,
    High,
    Ultra,
}

impl Preset {
    pub const ALL: [Preset; 4] = [Preset::Low, Preset::Medium, Preset::High, Preset::Ultra];

    pub fn name(self) -> &'static str {
        match self {
            Preset::Low => "Low",
            Preset::Medium => "Medium",
            Preset::High => "High",
            Preset::Ultra => "Ultra",
        }
    }

    /// What the preset sets: smoothing, shadows, scale, view distance, character distance (0: all).
    fn values(self) -> (mao_render::Antialiasing, mao_render::Shadows, f32, f32, f32) {
        use mao_render::{Antialiasing, Shadows};
        match self {
            Preset::Low => (Antialiasing::Off, Shadows::Off, 0.59, 800.0, 150.0),
            Preset::Medium => (Antialiasing::Taa, Shadows::Low, 0.77, 1500.0, 250.0),
            Preset::High => (Antialiasing::Msaa, Shadows::Medium, 1.0, 1800.0, 0.0),
            Preset::Ultra => (Antialiasing::Msaa, Shadows::High, 1.0, 4000.0, 0.0),
        }
    }

    pub fn apply(self, settings: &mut Settings) {
        (settings.antialiasing, settings.shadows, settings.render_scale, settings.view_distance, settings.char_distance) = self.values();
    }

    /// Whether the settings are the preset's.
    pub fn matches(self, settings: &Settings) -> bool {
        let (antialiasing, shadows, scale, view, characters) = self.values();
        settings.antialiasing == antialiasing && settings.shadows == shadows && (settings.render_scale - scale).abs() < 0.005 && (settings.view_distance - view).abs() < 0.5 && (settings.char_distance - characters).abs() < 0.5
    }
}

/// What the settings' window is told of the screens: the sizes to offer and the monitors' names.
#[derive(Default)]
struct Screens {
    sizes: Vec<(u32, u32)>,
    monitors: Vec<String>,
}

/// The settings' window while it is open: copies that OK writes back.
struct Dialog {
    tab: usize,
    settings: Settings,
    prefs: Prefs,
    width: String,
    height: String,
    adapters: Vec<String>,
    choosing_device: bool,
    sizes: Vec<(u32, u32)>,
    monitors: Vec<String>,
    themes: Vec<String>,
    network: String,
    dimension: String,
    /// The game folder's field as it was last checked, and what the check said.
    checked: Option<(String, Result<String, String>)>,
}

enum Closed {
    Ok,
    Cancel,
}

impl Dialog {
    fn new(settings: &Settings, prefs: &Prefs, screens: Screens) -> Dialog {
        Dialog {
            tab: 0,
            settings: settings.clone(),
            prefs: prefs.clone(),
            width: settings.width.to_string(),
            height: settings.height.to_string(),
            adapters: mao_render::adapter_names(),
            choosing_device: false,
            sizes: predefined(screens.sizes.into_iter(), (settings.width, settings.height)),
            monitors: screens.monitors,
            themes: crate::theme::available(&settings.aou_mod),
            network: prefs.network_timeout.to_string(),
            dimension: prefs.dimension_timeout.to_string(),
            checked: None,
        }
    }

    /// The text fields into the copies (a field that is no number keeps what was there).
    fn take_fields(&mut self) {
        self.settings.width = self.width.trim().parse().unwrap_or(self.settings.width).clamp(320, 16384);
        self.settings.height = self.height.trim().parse().unwrap_or(self.settings.height).clamp(240, 16384);
        self.prefs.network_timeout = self.network.trim().parse().unwrap_or(self.prefs.network_timeout).clamp(5, 600);
        self.prefs.dimension_timeout = self.dimension.trim().parse().unwrap_or(self.prefs.dimension_timeout).clamp(2, 300);
    }

    fn visual(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Rendering Device");
            ui.horizontal(|ui| {
                let shown = if self.settings.adapter.is_empty() { "The system's choice".to_string() } else { self.settings.adapter.clone() };
                ui.label(shown);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Change").clicked() {
                        self.choosing_device = !self.choosing_device;
                    }
                });
            });
            if self.choosing_device {
                if ui.selectable_label(self.settings.adapter.is_empty(), "The system's choice").clicked() {
                    self.settings.adapter.clear();
                    self.choosing_device = false;
                }
                for name in self.adapters.clone() {
                    if ui.selectable_label(self.settings.adapter == name, &name).clicked() {
                        self.settings.adapter = name;
                        self.choosing_device = false;
                    }
                }
            }
        });
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Resolution");
            let (mut window, mut borderless) = mode_boxes(self.settings.mode);
            ui.horizontal(|ui| {
                ui.checkbox(&mut window, "Run In Window");
                ui.add_enabled(window, egui::Checkbox::new(&mut borderless, "Borderless"));
            });
            self.settings.mode = boxes_mode(window, borderless);
            // (a size is the window's own only: the borderless and full-screen modes take the monitor's)
            let sized = self.settings.mode == WindowMode::Windowed;
            ui.add_enabled_ui(sized, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Predefined");
                    let current = format!("{}x{}", self.width.trim(), self.height.trim());
                    egui::ComboBox::from_id_salt("launcher-sizes").selected_text(current).width(150.0).show_ui(ui, |ui| {
                        for (w, h) in self.sizes.clone() {
                            if ui.selectable_label(self.width.trim() == w.to_string() && self.height.trim() == h.to_string(), format!("{w}x{h}")).clicked() {
                                (self.width, self.height) = (w.to_string(), h.to_string());
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Custom");
                    ui.add(egui::TextEdit::singleline(&mut self.width).desired_width(60.0));
                    ui.label("x");
                    ui.add(egui::TextEdit::singleline(&mut self.height).desired_width(60.0));
                });
            });
            if !sized {
                ui.weak("The monitor's own size is used.");
            }
            ui.add_enabled_ui(!sized, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Monitor");
                    let name = |i: u32, monitors: &[String]| if i == 0 { "The one the window is on".to_string() } else { monitors.get(i as usize - 1).cloned().unwrap_or_else(|| format!("Monitor {i}")) };
                    egui::ComboBox::from_id_salt("launcher-monitors").selected_text(name(self.settings.monitor, &self.monitors)).width(220.0).show_ui(ui, |ui| {
                        for i in 0..=self.monitors.len() as u32 {
                            ui.selectable_value(&mut self.settings.monitor, i, name(i, &self.monitors));
                        }
                    });
                });
            });
        });
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Quality");
            ui.horizontal(|ui| {
                for preset in Preset::ALL {
                    if ui.selectable_label(preset.matches(&self.settings), preset.name()).clicked() {
                        preset.apply(&mut self.settings);
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Edges");
                for (value, name) in [(mao_render::Antialiasing::Off, "Off"), (mao_render::Antialiasing::Msaa, "MSAA 4x"), (mao_render::Antialiasing::Taa, "TAA")] {
                    ui.radio_value(&mut self.settings.antialiasing, value, name);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Shadows");
                for (value, name) in [(mao_render::Shadows::Off, "Off"), (mao_render::Shadows::Low, "Low"), (mao_render::Shadows::Medium, "Medium"), (mao_render::Shadows::High, "High")] {
                    ui.radio_value(&mut self.settings.shadows, value, name);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Drawing scale");
                egui::ComboBox::from_id_salt("launcher-scale").selected_text(scale_name(self.settings.render_scale)).width(170.0).show_ui(ui, |ui| {
                    for (scale, _) in SCALES {
                        ui.selectable_value(&mut self.settings.render_scale, scale, scale_name(scale));
                    }
                });
            });
            // (the sharpening is the upscaler's: it does nothing at the window's own size)
            if self.settings.render_scale < 0.999 {
                ui.add(egui::Slider::new(&mut self.settings.sharpness, 0.0..=2.0).text("Sharpening (0 is the most)").fixed_decimals(1));
            }
            ui.add(egui::Slider::new(&mut self.settings.view_distance, 150.0..=6000.0).text("View distance").fixed_decimals(0));
            ui.add(egui::Slider::new(&mut self.settings.char_distance, 0.0..=400.0).step_by(5.0).custom_formatter(|v, _| if v < 1.0 { "all".to_string() } else { format!("{v:.0}") }).text("Character distance"));
        });
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Frame rate");
            ui.checkbox(&mut self.settings.vsync, "Wait for the monitor's refresh (VSync)");
            let limit_name = |fps: u32, none: &str| if fps == 0 { none.to_string() } else { format!("{fps} a second") };
            ui.horizontal(|ui| {
                ui.label("Limit");
                egui::ComboBox::from_id_salt("launcher-limit").selected_text(limit_name(self.settings.fps_limit, "None")).width(120.0).show_ui(ui, |ui| {
                    for fps in FRAME_LIMITS {
                        ui.selectable_value(&mut self.settings.fps_limit, fps, limit_name(fps, "None"));
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label("In the background");
                egui::ComboBox::from_id_salt("launcher-background").selected_text(limit_name(self.settings.background_fps, "The same")).width(120.0).show_ui(ui, |ui| {
                    for fps in BACKGROUND_LIMITS {
                        ui.selectable_value(&mut self.settings.background_fps, fps, limit_name(fps, "The same"));
                    }
                });
            });
        });
    }

    fn gui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Skin");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("launcher-themes").selected_text(self.settings.theme.clone()).width(220.0).show_ui(ui, |ui| {
                    for name in self.themes.clone() {
                        ui.selectable_value(&mut self.settings.theme, name.clone(), name);
                    }
                });
                if ui.button("Set Default").clicked() {
                    self.settings.theme = Settings::default().theme;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Skin folder");
                ui.add(egui::TextEdit::singleline(&mut self.settings.gui_dir).desired_width(260.0));
            });
        });
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Font");
            ui.checkbox(&mut self.settings.original_font, "The original's own font");
            ui.horizontal(|ui| {
                ui.spacing_mut().slider_width = 170.0;
                ui.add(egui::Slider::new(&mut self.settings.ui_scale, 0.6..=2.0).step_by(0.05).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)).text("Interface size"));
                if ui.button("Set Default").clicked() {
                    self.settings.ui_scale = Settings::default().ui_scale;
                }
            });
        });
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Folders");
            // (the original game's own folder: everything the client shows is read from it)
            ui.horizontal(|ui| {
                ui.add_sized([80.0, 18.0], egui::Label::new("Game"));
                ui.add(egui::TextEdit::singleline(&mut self.settings.game_dir).desired_width(180.0));
                if ui.button("Browse").clicked() {
                    let mut picker = rfd::FileDialog::new().set_title("The Anarchy Online folder");
                    if Path::new(self.settings.game_dir.trim()).is_dir() {
                        picker = picker.set_directory(self.settings.game_dir.trim());
                    }
                    if let Some(folder) = picker.pick_folder() {
                        self.settings.game_dir = folder.display().to_string();
                    }
                }
                if ui.button("Open").clicked() && Path::new(self.settings.game_dir.trim()).is_dir() {
                    open_outside(self.settings.game_dir.trim());
                }
            });
            if self.checked.as_ref().is_none_or(|(text, _)| *text != self.settings.game_dir) {
                self.checked = Some((self.settings.game_dir.clone(), check_game_dir(Path::new(self.settings.game_dir.trim()))));
            }
            match self.checked.as_ref().map(|(_, result)| result) {
                Some(Ok(version)) => ui.colored_label(egui::Color32::from_rgb(120, 200, 120), format!("The game is there, version {version}.")),
                Some(Err(why)) => ui.colored_label(egui::Color32::from_rgb(230, 120, 90), why),
                None => ui.label(""),
            };
            let base = Settings::path();
            for (name, folder) in [("Settings", base.parent().map(Path::to_path_buf).unwrap_or_default()), ("Screenshots", base.with_file_name("screenshots")), ("Chat logs", base.with_file_name("chat"))] {
                ui.horizontal(|ui| {
                    ui.add_sized([80.0, 18.0], egui::Label::new(name));
                    let mut shown = folder.display().to_string();
                    ui.add_enabled(false, egui::TextEdit::singleline(&mut shown).desired_width(230.0));
                    if ui.button("Open").clicked() {
                        let _ = std::fs::create_dir_all(&folder);
                        open_outside(&folder.display().to_string());
                    }
                });
            }
        });
    }

    fn connection(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Timeouts");
            for (label, field) in [("Server answer time-out (seconds):", &mut self.network), ("Dimension list time-out (seconds):", &mut self.dimension)] {
                ui.horizontal(|ui| {
                    ui.add_sized([230.0, 18.0], egui::Label::new(label));
                    ui.add(egui::TextEdit::singleline(field).desired_width(50.0));
                });
            }
        });
    }

    fn show(&mut self, ctx: &egui::Context) -> Option<Closed> {
        let mut closed = None;
        egui::Window::new("Settings").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).fixed_size(egui::vec2(440.0, 520.0)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, name) in ["Visual", "GUI", "Connection"].into_iter().enumerate() {
                    if ui.selectable_label(self.tab == i, name).clicked() {
                        self.tab = i;
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().max_height(440.0).show(ui, |ui| match self.tab {
                0 => self.visual(ui),
                1 => self.gui(ui),
                _ => self.connection(ui),
            });
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    closed = Some(Closed::Cancel);
                }
                if ui.button("   OK   ").clicked() {
                    closed = Some(Closed::Ok);
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            closed = Some(Closed::Cancel);
        }
        closed
    }
}

struct Gfx {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: Gpu,
    ui: Ui,
    pictures: Option<Pictures>,
    #[cfg(windows)]
    page: Option<(wry::WebView, wry::WebContext)>,
    page_shown: bool,
    frames: u32,
    /// When the window drew its first frame, and whether the page was opened since.
    first_frame: Option<Instant>,
    page_tried: bool,
}

struct Launch {
    game: String,
    settings: Settings,
    prefs: Prefs,
    gfx: Option<Gfx>,
    dimensions: Vec<Dimension>,
    selected: usize,
    status: String,
    list: Option<Receiver<Result<String, String>>>,
    local_version: String,
    titled: String,
    dialog: Option<Dialog>,
    play: bool,
    /// The folder is the game's (PLAY needs it).
    game_ok: bool,
    /// The settings chose another game folder: the launcher starts again.
    restart: bool,
    started: Instant,
    /// `MAO_LAUNCH_QUIT=<seconds>`: the window closes by itself (for pictures of it).
    quit_after: Option<f32>,
    play_after: Option<f32>,
}

impl Launch {
    fn dimension(&self) -> Option<&Dimension> {
        self.dimensions.get(self.selected)
    }

    fn set_list(&mut self, text: &str) {
        let list = dimensions::parse(text);
        if list.is_empty() {
            return;
        }
        let wanted = self.dimension().map(|d| d.name.clone()).unwrap_or_else(|| self.prefs.dimension.clone());
        self.selected = list.iter().position(|d| d.name.eq_ignore_ascii_case(&wanted)).unwrap_or(0);
        self.dimensions = list;
    }

    /// The window's inside, drawn each frame; says whether the page may be shown over its top.
    fn screen(&mut self, ctx: &egui::Context, pictures: &Pictures, monitor: &dyn Fn() -> Screens) -> bool {
        if let Some(rx) = &self.list {
            if let Ok(result) = rx.try_recv() {
                self.list = None;
                match result {
                    Ok(text) => {
                        let _ = std::fs::write(dimensions::cache_path(), &text);
                        self.set_list(&text);
                        self.status.clear();
                    }
                    Err(e) => {
                        eprintln!("the dimension list: {e}");
                        self.status = if self.dimensions.is_empty() { "The dimension server did not answer.".into() } else { "The dimension server did not answer: this is the list kept from before.".into() };
                    }
                }
            }
        }
        let screen = ctx.content_rect();
        let k = screen.width() / WIDTH;
        let at = |r: [f32; 4]| egui::Rect::from_min_size(screen.min + egui::vec2(r[0], r[1]) * k, egui::vec2(r[2], r[3]) * k);
        let whole = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let enabled = self.dialog.is_none();
        let mut open_settings = false;
        egui::Area::new(egui::Id::unique("launcher")).order(egui::Order::Background).fixed_pos(screen.min).show(ctx, |ui| {
            ui.set_min_size(screen.size());
            let painter = ui.painter().clone();
            match &pictures.background {
                Some(texture) => {
                    painter.image(texture.id(), screen, whole, egui::Color32::WHITE);
                }
                None => {
                    painter.rect_filled(screen, 0.0, egui::Color32::from_rgb(12, 12, 12));
                }
            }
            // (the page's place while it is away: a line saying so, or what the list is waiting for)
            if !self.status.is_empty() {
                painter.text(at([403.0, 452.0, 0.0, 0.0]).min, egui::Align2::LEFT_TOP, &self.status, egui::FontId::proportional(11.0 * k), egui::Color32::from_gray(190));
            }
            let button = |ui: &mut egui::Ui, id: &'static str, r: [f32; 4], textures: &ButtonPictures, on: bool, label: &str| -> bool {
                let rect = at(r);
                let response = ui.interact(rect, egui::Id::unique(id), if on { egui::Sense::click() } else { egui::Sense::hover() });
                let which = if !on { 2 } else if response.hovered() { 1 } else { 0 };
                match textures[which].as_ref().or(textures[0].as_ref()) {
                    Some(texture) => {
                        painter.image(texture.id(), rect, whole, egui::Color32::WHITE);
                    }
                    None => {
                        let fill = if !on { egui::Color32::from_gray(60) } else if response.hovered() { egui::Color32::from_rgb(230, 120, 40) } else { egui::Color32::from_rgb(200, 90, 20) };
                        painter.rect_filled(rect, 2.0, fill);
                        painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(rect.height() * 0.5), egui::Color32::WHITE);
                    }
                }
                on && response.clicked()
            };
            if button(ui, "launcher-settings", SETTINGS_BUTTON, &pictures.settings, enabled, "SETTINGS") {
                open_settings = true;
            }
            if button(ui, "launcher-play", PLAY_BUTTON, &pictures.play, enabled && self.game_ok && self.dimension().is_some(), "PLAY") {
                self.play = true;
            }
            // (the dimensions: a plain white list box as the original's)
            let list = at(LIST);
            ui.scope_builder(egui::UiBuilder::new().max_rect(list), |ui| {
                if !enabled {
                    ui.disable();
                }
                *ui.visuals_mut() = egui::Visuals::light();
                ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                ui.spacing_mut().interact_size.y = list.height();
                let widgets = &mut ui.visuals_mut().widgets;
                for w in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active, &mut widgets.open] {
                    w.corner_radius = egui::CornerRadius::ZERO;
                    w.weak_bg_fill = egui::Color32::WHITE;
                    w.bg_fill = egui::Color32::WHITE;
                }
                let shown = self.dimension().map_or_else(|| "(no dimension)".to_string(), |d| d.name.clone());
                egui::ComboBox::from_id_salt("launcher-dimensions").selected_text(shown).width(list.width()).show_ui(ui, |ui| {
                    for (i, d) in self.dimensions.iter().enumerate() {
                        ui.selectable_value(&mut self.selected, i, &d.name);
                    }
                });
            });
        });
        if open_settings {
            self.dialog = Some(Dialog::new(&self.settings, &self.prefs, monitor()));
        }
        if let Some(dialog) = self.dialog.as_mut() {
            match dialog.show(ctx) {
                Some(Closed::Ok) => {
                    dialog.take_fields();
                    self.settings = dialog.settings.clone();
                    self.settings.game_dir = self.settings.game_dir.trim().to_string();
                    self.prefs = dialog.prefs.clone();
                    self.settings.save();
                    self.dialog = None;
                    // (another game folder: the launcher starts again with it, since its pictures, its page, its
                    // version and the interface's skin all came from the old one)
                    if self.settings.game_dir != self.game {
                        self.restart = true;
                    }
                }
                Some(Closed::Cancel) => self.dialog = None,
                None => {}
            }
        }
        if let Some(name) = self.dimension().map(|d| d.name.clone()).filter(|n| *n != self.prefs.dimension) {
            self.prefs.dimension = name;
        }
        // (the page is a window of its own over the top: away while the settings are open over it)
        self.dialog.is_none()
    }
}

/// The sizes of the monitor the window is on (its video modes and its own) and every monitor's name with its size.
fn screens(window: &Window) -> Screens {
    let sizes = window.current_monitor().map(|m| m.video_modes().map(|v| (v.size().width, v.size().height)).chain(std::iter::once((m.size().width, m.size().height))).collect()).unwrap_or_default();
    let monitors = window.available_monitors().enumerate().map(|(i, m)| format!("{}: {} ({}x{})", i + 1, m.name().unwrap_or_else(|| "monitor".into()), m.size().width, m.size().height)).collect();
    Screens { sizes, monitors }
}

#[cfg(windows)]
fn open_page(window: &Window, address: &str, bounds: wry::Rect) -> Option<(wry::WebView, wry::WebContext)> {
    let data = Settings::path().parent().map_or_else(|| PathBuf::from("Browser"), |d| d.join("Browser"));
    let mut context = wry::WebContext::new(Some(data));
    let view = wry::WebViewBuilder::new_with_web_context(&mut context)
        .with_bounds(bounds)
        .with_background_color((12, 12, 12, 255))
        .with_url(address)
        // (the page's links ask for a new window, as they did of the original's browser control: the system's browser has them)
        .with_new_window_req_handler(|url, _| {
            open_outside(&url);
            wry::NewWindowResponse::Deny
        })
        .build_as_child(window);
    match view {
        Ok(view) => Some((view, context)),
        Err(e) => {
            eprintln!("the launcher's page: {e}");
            None
        }
    }
}

/// The page's place in the window, in the window's own pixels.
#[cfg(windows)]
fn page_bounds(size: winit::dpi::PhysicalSize<u32>) -> wry::Rect {
    let height = (size.height as f32 * PAGE_HEIGHT / HEIGHT).round() as u32;
    wry::Rect { position: wry::dpi::PhysicalPosition::new(0, 0).into(), size: wry::dpi::PhysicalSize::new(size.width.max(1), height.max(1)).into() }
}

impl ApplicationHandler for Launch {
    /// The window is drawn about sixty times a second whatever else happens: asking for the next frame from inside a frame
    /// alone stopped once the page (a window of its own with its own messages) was there, and with it the buttons.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(gfx) = &self.gfx {
            gfx.window.request_redraw();
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(16)));
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title(title(&self.local_version, "")).with_window_icon(crate::icon::window_icon()).with_inner_size(LogicalSize::new(WIDTH, HEIGHT)).with_resizable(false).with_enabled_buttons(winit::window::WindowButtons::CLOSE | winit::window::WindowButtons::MINIMIZE))
                .expect("a window"),
        );
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("a surface");
        let gpu = Gpu::new(instance, Some(&surface)).expect("a graphics device");
        let size = window.inner_size();
        let config = surface.get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1)).expect("a surface format");
        surface.configure(&gpu.device, &config);
        let ui = Ui::new(&window, &gpu, config.format, &self.settings);
        window.request_redraw();
        self.gfx = Some(Gfx {
            window,
            surface,
            config,
            gpu,
            ui,
            pictures: None,
            #[cfg(windows)]
            page: None,
            page_shown: true,
            frames: 0,
            first_frame: None,
            page_tried: false,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(mut gfx) = self.gfx.take() else { return };
        let consumed = gfx.ui.event(&gfx.window, &event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                gfx.config.width = size.width.max(1);
                gfx.config.height = size.height.max(1);
                gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                #[cfg(windows)]
                if let Some((view, _)) = &gfx.page {
                    let _ = view.set_bounds(page_bounds(size));
                }
                gfx.window.request_redraw();
            }
            WindowEvent::RedrawRequested => match gfx.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let size = (gfx.config.width, gfx.config.height);
                    let game = self.game.clone();
                    let window = gfx.window.clone();
                    let monitor = move || screens(&window);
                    let pictures = &mut gfx.pictures;
                    let mut page_may_show = true;
                    let ui_frame = gfx.ui.run_custom(&gfx.window, &mut |ctx| {
                        let pictures = pictures.get_or_insert_with(|| load_pictures(ctx, Path::new(&game)));
                        page_may_show = self.screen(ctx, pictures, &monitor);
                    });
                    if page_may_show != gfx.page_shown {
                        gfx.page_shown = page_may_show;
                        #[cfg(windows)]
                        if let Some((page, _)) = &gfx.page {
                            let _ = page.set_visible(page_may_show);
                            if !page_may_show {
                                let _ = page.focus_parent();
                            }
                        }
                    }
                    let wanted = title(&self.local_version, self.dimension().map_or("", |d| d.version.as_str()));
                    if wanted != self.titled {
                        gfx.window.set_title(&wanted);
                        self.titled = wanted;
                    }
                    let mut encoder = gfx.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("launcher background"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), store: wgpu::StoreOp::Store } })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    }));
                    gfx.gpu.queue.submit(Some(encoder.finish()));
                    gfx.ui.paint(&gfx.gpu, &view, size, ui_frame);
                    gfx.window.pre_present_notify();
                    gfx.gpu.queue.present(frame);
                    // (the page is opened once the window has been drawing itself for half a second: opened before the
                    // first frame, the window's own drawing never showed under it, and once in a dozen starts neither did
                    // it when the page came after the third frame; the surface is set up anew after it for the same reason)
                    gfx.frames += 1;
                    let drawing_since = *gfx.first_frame.get_or_insert_with(Instant::now);
                    #[cfg(windows)]
                    if !gfx.page_tried && gfx.frames >= 10 && drawing_since.elapsed() > Duration::from_millis(500) && std::env::var_os("MAO_LAUNCH_NO_PAGE").is_none() {
                        gfx.page_tried = true;
                        if let Some(address) = dimensions::page_address(Path::new(&self.game)) {
                            gfx.page = open_page(&gfx.window, &address, page_bounds(gfx.window.inner_size()));
                            gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                            if !gfx.page_shown {
                                if let Some((page, _)) = &gfx.page {
                                    let _ = page.set_visible(false);
                                }
                            }
                        }
                    }
                    // (`MAO_LAUNCH_PLAY=<seconds>`: PLAY presses itself, for a test of the way into the login)
                    if self.play_after.is_some_and(|s| self.started.elapsed().as_secs_f32() > s) && self.list.is_none() && self.game_ok {
                        self.play = true;
                    }
                    if self.play || self.restart || self.quit_after.is_some_and(|s| self.started.elapsed().as_secs_f32() > s) {
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

/// Opens the launcher; PLAY starts the login for the chosen dimension.
pub fn run(named: Option<&str>) -> Result<(), String> {
    let mut settings = Settings::load();
    // (the game's folder: named on the command line, else the settings', else one of the usual places, which is then
    // kept; with none, the launcher opens on its settings to have one chosen)
    let resolved = resolve_game_dir(named, &settings.game_dir, &usual_game_dirs());
    let game_ok = resolved.as_ref().is_some_and(|(folder, _)| check_game_dir(Path::new(folder)).is_ok());
    let game_owned = resolved.as_ref().map(|(folder, _)| folder.clone()).unwrap_or_default();
    let game = game_owned.as_str();
    if game_ok && (settings.game_dir.is_empty() || resolved.as_ref().is_some_and(|(_, found)| *found)) && settings.game_dir != game {
        settings.game_dir = game.to_string();
        settings.save();
    }
    if game_ok {
        crate::theme::set_game_dir(game);
    }
    crate::font::set_enabled(settings.original_font);
    mao_render::prefer_adapter(&settings.adapter);
    let prefs = Prefs::load();
    // (the folder's version without what follows an underscore, the expansion's mark, as the original's title has it and
    // as the dimension list gives a version)
    let local_version = mao_formats::files::read_to_string(Path::new(game).join("version.id")).map(|t| t.lines().next().unwrap_or("").trim().split('_').next().unwrap_or("").to_string()).unwrap_or_default();
    let mut launch = Launch {
        game: game.to_string(),
        settings,
        prefs: prefs.clone(),
        gfx: None,
        dimensions: Vec::new(),
        selected: 0,
        status: String::new(),
        list: None,
        local_version,
        titled: String::new(),
        dialog: None,
        play: false,
        game_ok,
        restart: false,
        started: Instant::now(),
        quit_after: std::env::var("MAO_LAUNCH_QUIT").ok().and_then(|v| v.parse().ok()),
        play_after: std::env::var("MAO_LAUNCH_PLAY").ok().and_then(|v| v.parse().ok()),
    };
    // (the list kept from before until the server's comes; with neither, the one dimension the client knows itself)
    launch.set_list(&std::fs::read_to_string(dimensions::cache_path()).unwrap_or_default());
    if launch.dimensions.is_empty() {
        let (connect, port) = mao_net::session::RUBI_KA.split_once(':').unwrap_or((mao_net::session::RUBI_KA, "7505"));
        launch.dimensions = vec![Dimension { name: "Rubi-Ka".into(), connect: connect.into(), port: port.parse().unwrap_or(7505), version: String::new() }];
    }
    if let Some(address) = dimensions::list_address(Path::new(game)) {
        let (tx, rx) = channel();
        launch.list = Some(rx);
        launch.status = "Getting the dimensions...".into();
        let timeout = Duration::from_secs(prefs.dimension_timeout as u64);
        std::thread::spawn(move || {
            let _ = tx.send(dimensions::download(&address, timeout));
        });
    }
    // (no game: the settings open on the page with the folders)
    if !game_ok {
        launch.status = "Choose the game's folder in the settings.".into();
        let mut dialog = Dialog::new(&launch.settings, &launch.prefs, Screens::default());
        dialog.tab = 1;
        launch.dialog = Some(dialog);
    }
    // (MAO_LAUNCH_SETTINGS=<page 0-2>: the settings open at that page, for pictures)
    if let Some(tab) = std::env::var("MAO_LAUNCH_SETTINGS").ok().and_then(|v| v.parse::<usize>().ok()) {
        let mut dialog = Dialog::new(&launch.settings, &launch.prefs, Screens::default());
        dialog.tab = tab.min(2);
        launch.dialog = Some(dialog);
    }
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.run_app(&mut launch).map_err(|e| e.to_string())?;
    if launch.prefs != prefs {
        launch.prefs.save();
    }
    if launch.restart {
        launch.gfx = None;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        std::process::Command::new(exe).arg("launch").spawn().map_err(|e| e.to_string())?;
        return Ok(());
    }
    if !launch.play {
        return Ok(());
    }
    let Some(dimension) = launch.dimension().cloned() else { return Ok(()) };
    if !dimension.version.is_empty() && !launch.local_version.is_empty() && dimension.version != launch.local_version {
        eprintln!("the game folder is version {}, {} wants {}: the login may be refused (this client cannot patch the game)", launch.local_version, dimension.name, dimension.version);
    }
    // (this run is over before the login starts, or the login would take it for one that never ended: `crash.rs`)
    crate::crash::end("launcher", None);
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe).args([game, "play"]).env("MAO_SERVER", dimension.server()).env("MAO_DIMENSION", &dimension.name).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder is the game's when it has the resource database and the version file; the launcher takes the folder
    /// named, else the one kept, else the first usual place that has the game.
    #[test]
    fn the_game_folder_is_checked_and_resolved() {
        let base = std::env::temp_dir().join(format!("mao-gamedir-{}", std::process::id()));
        let (good, empty) = (base.join("good"), base.join("empty"));
        std::fs::create_dir_all(good.join("cd_image/data/db")).unwrap();
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::write(good.join("cd_image/data/db/ResourceDatabase.dat"), b"x").unwrap();
        assert!(check_game_dir(&good).unwrap_err().contains("version.id"));
        std::fs::write(good.join("version.id"), "18.8.62_EP1\r\n").unwrap();
        assert_eq!(check_game_dir(&good).as_deref(), Ok("18.8.62_EP1"));
        assert!(check_game_dir(&empty).unwrap_err().contains("ResourceDatabase"));
        assert!(check_game_dir(&base.join("missing")).is_err());
        assert!(check_game_dir(Path::new("")).is_err());
        let (good_text, empty_text) = (good.display().to_string(), empty.display().to_string());
        // (named: taken as it is; kept and good: kept; kept and bad: the usual places; nothing anywhere: none)
        assert_eq!(resolve_game_dir(Some("anything"), &good_text, &[]), Some(("anything".to_string(), false)));
        assert_eq!(resolve_game_dir(None, &good_text, &[empty.clone()]), Some((good_text.clone(), false)));
        assert_eq!(resolve_game_dir(None, &empty_text, &[empty.clone(), good.clone()]), Some((good_text.clone(), true)));
        assert_eq!(resolve_game_dir(None, "", &[empty.clone()]), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A preset sets its values and is known again by them; a scale has its step's name.
    #[test]
    fn the_presets_set_and_know_their_values() {
        let mut settings = Settings::default();
        for preset in Preset::ALL {
            preset.apply(&mut settings);
            assert_eq!(Preset::ALL.into_iter().filter(|p| p.matches(&settings)).collect::<Vec<_>>(), vec![preset]);
        }
        Preset::Low.apply(&mut settings);
        assert_eq!((settings.antialiasing, settings.shadows), (mao_render::Antialiasing::Off, mao_render::Shadows::Off));
        settings.view_distance = 1234.0;
        assert!(Preset::ALL.into_iter().all(|p| !p.matches(&settings)));
        assert_eq!(scale_name(0.77), "Ultra quality (77%)");
        assert_eq!(scale_name(1.0), "Native (100%)");
        assert_eq!(scale_name(0.9), "90%");
    }

    #[test]
    fn the_two_boxes_are_the_three_window_modes() {
        for mode in [WindowMode::Windowed, WindowMode::Borderless, WindowMode::Fullscreen] {
            let (window, borderless) = mode_boxes(mode);
            assert_eq!(boxes_mode(window, borderless), mode);
        }
        // (borderless means nothing without the window box)
        assert_eq!(boxes_mode(false, true), WindowMode::Fullscreen);
    }

    #[test]
    fn the_predefined_sizes_are_the_monitors_and_the_one_set() {
        let sizes = predefined([(1920, 1080), (5120, 1440), (1920, 1080), (320, 200), (2560, 1440)].into_iter(), (1280, 800));
        assert_eq!(sizes, vec![(5120, 1440), (2560, 1440), (1920, 1080), (1280, 800)]);
    }

    #[test]
    fn the_title_names_both_versions() {
        assert_eq!(title("18.8.62", "18.8.62"), "Modern Anarchy Online - Local Version: 18.8.62 - Remote version: 18.8.62");
        assert_eq!(title("18.8.62", ""), "Modern Anarchy Online - Local Version: 18.8.62 - Remote version: unknown");
    }
}
