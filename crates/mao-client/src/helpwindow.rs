//! The help assistant's window ("Ask the guides"; `/open assistant`, or the Help menu): a question, and the pieces of
//! the help pages that fit it best, each with where it is from. This client's own: the original has nothing like it.
//!
//! The pages are the game's own help files (read from the installed game) and the guides of sites the player has
//! turned on under Sources, which are read slowly and by the site's rules into `help/` beside the settings and stay
//! there (`mao_help`; `docs/HELP_ASSISTANT.md`). With a language model set up under `[help]` in `mao.ini` the best
//! pieces are also handed to it for an answer in words; without one, nothing leaves the machine but the requests for
//! a turned-on site's pages.
//!
//! The window keeps its own state here (one window, behind a lock) so that the interface's struct and its settings
//! need only the few calls at the foot of this file's users: `show`, `set_open`, `is_open`, `configure`,
//! `settings_text`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

use mao_help::crawl::{self, Site, Step};
use mao_help::fetch::Curl;
use mao_help::index::{Hit, Index, Store};
use mao_help::llm::{self, Api, Model};
use mao_help::local;

use crate::theme::Palette;

/// `[help]` in `mao.ini`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// The sources that are on: `game` and the names of sites (`mao_help::crawl::SITES`).
    pub sources: Vec<String>,
    pub model: Model,
    /// The model inside the client that writes the answers (`mao_help::local::MODELS`); empty for none. It is used
    /// before a model behind an address, once its files are there.
    pub local: String,
}

impl Default for Config {
    fn default() -> Config {
        Config { sources: vec![mao_help::GAME.to_string()], model: Model::default(), local: String::new() }
    }
}

static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

pub fn config() -> Config {
    CONFIG.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default()
}

fn set_config(config: Config) {
    *CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = Some(config);
}

/// A line of the settings' `[help]` section; false for a name that is not one of its.
pub fn configure(name: &str, value: &str) -> bool {
    let mut c = config();
    match name {
        "sources" => c.sources = mao_help::source_names(value),
        "endpoint" => c.model.endpoint = value.to_string(),
        "model" => c.model.model = value.to_string(),
        "api" => c.model.api = if value.eq_ignore_ascii_case("anthropic") { Api::Anthropic } else { Api::OpenAi },
        "key_env" => c.model.key_env = value.to_string(),
        "context_chars" => c.model.context_chars = value.parse().unwrap_or(c.model.context_chars).clamp(1000, 200_000),
        "local_model" => c.local = if value.trim().is_empty() { String::new() } else { local::known(value).name.to_string() },
        _ => return false,
    }
    set_config(c);
    true
}

/// The settings' `[help]` section as it is written.
pub fn settings_text() -> String {
    let c = config();
    let sites: Vec<&str> = crawl::SITES.iter().map(|s| s.name).collect();
    format!(
        "[help]\n; the help assistant (/open assistant): where its pages come from. game = the game's own help files; sites: {}\n; (a site's pages are read into the help folder beside this file, for your own searching only)\nsources = {}\n; a language model for answers in words (none: search results only): the whole address of a chat endpoint,\n; for example http://localhost:11434/v1/chat/completions (ollama on this machine), and the model's name there\nendpoint = {}\nmodel = {}\n; openai (ollama, llama.cpp, LM Studio and the like) | anthropic\napi = {}\n; the name of the environment variable that holds the endpoint's key, when it wants one (the key itself is never written here)\nkey_env = {}\n; the most characters of help pages handed to the model with a question\ncontext_chars = {}\n\n",
        sites.join(", "),
        c.sources.join(", "),
        c.model.endpoint,
        c.model.model,
        if c.model.api == Api::Anthropic { "anthropic" } else { "openai" },
        c.model.key_env,
        c.model.context_chars
    )
    .trim_end()
    .to_string()
        + &format!("\n; the model inside the client, for answers in words with no other program: {} (empty: none). Its files are fetched\n; once, when asked for in the window, into the help folder's models folder\nlocal_model = {}\n\n", local::MODELS.iter().map(|m| m.name).collect::<Vec<_>>().join(", "), c.local)
}

/// Where the sites' pages are kept: `help` beside the settings (`MAO_HELP_DIR` puts it elsewhere, for trying things).
pub fn help_dir() -> PathBuf {
    std::env::var_os("MAO_HELP_DIR").map(PathBuf::from).unwrap_or_else(|| crate::settings::Settings::path().parent().map_or_else(|| PathBuf::from("help"), |d| d.join("help")))
}

/// What the answering thread sends: the answer as far as it is written, then how it ended.
enum Said {
    SoFar(String),
    Done(Result<llm::Answer, String>),
}

enum Asking {
    Idle,
    /// Waiting for the model, with what it has written so far.
    Waiting(Receiver<Said>, String),
    Done(Result<llm::Answer, String>),
}

/// The model inside the client, kept loaded between questions (its name, the model, when it last answered) and let go
/// after [`KEEP_LOADED`] without one: it holds about a gigabyte of memory.
static LOADED: Mutex<Option<(String, local::Local, std::time::Instant)>> = Mutex::new(None);
const KEEP_LOADED: std::time::Duration = std::time::Duration::from_secs(300);

/// A model's files being fetched.
struct Fetching {
    model: &'static local::Known,
    done: Receiver<Result<(), String>>,
    stop: Arc<AtomicBool>,
}

enum Progress {
    Page(String, usize, usize),
    Done(Result<(crawl::Report, usize), String>),
}

struct Crawling {
    site: &'static Site,
    progress: Receiver<Progress>,
    stop: Arc<AtomicBool>,
    line: String,
}

struct Assistant {
    open: bool,
    question: String,
    asked: String,
    hits: Vec<Hit>,
    /// The result shown whole.
    expanded: Option<usize>,
    asking: Asking,
    index: Option<Arc<Index>>,
    loading: Option<Receiver<Arc<Index>>>,
    /// A question waits for the index.
    pending: bool,
    crawling: Option<Crawling>,
    fetching: Option<Fetching>,
    /// What the last fetch of the model came to.
    model_status: String,
    /// What the last reading of a site came to.
    status: String,
    focus: bool,
    started: bool,
}

static WINDOW: Mutex<Option<Assistant>> = Mutex::new(None);

fn window() -> std::sync::MutexGuard<'static, Option<Assistant>> {
    WINDOW.lock().unwrap_or_else(|e| e.into_inner())
}

fn fresh() -> Assistant {
    Assistant { open: false, question: String::new(), asked: String::new(), hits: Vec::new(), expanded: None, asking: Asking::Idle, index: None, loading: None, pending: false, crawling: None, fetching: None, model_status: String::new(), status: String::new(), focus: false, started: false }
}

pub fn is_open() -> bool {
    window().as_ref().is_some_and(|w| w.open)
}

pub fn set_open(open: bool) {
    let mut guard = window();
    let w = guard.get_or_insert_with(fresh);
    w.focus = open && !w.open;
    w.open = open;
}

/// Opens an address in the system's browser: only the pages of the sites this client reads.
fn open_outside(url: &str) {
    if !crawl::SITES.iter().any(|s| crawl::path_on(s.origin, url).is_some()) {
        return;
    }
    #[cfg(windows)]
    let _ = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

impl Assistant {
    /// Reads the pages of the sources that are on, away from the frame.
    fn reload(&mut self) {
        let (tx, rx) = channel();
        let (dir, game, sources) = (help_dir(), crate::theme::game_dir(), config().sources);
        std::thread::spawn(move || {
            let _ = tx.send(Arc::new(mao_help::load(&Store::new(dir), game.as_deref(), &sources)));
        });
        self.loading = Some(rx);
    }

    fn search(&mut self) {
        let Some(index) = self.index.clone() else {
            self.pending = true;
            return;
        };
        self.pending = false;
        self.expanded = None;
        self.hits = index.search(&self.asked, 6);
        self.asking = Asking::Idle;
        let config = config();
        let model = config.model;
        let inside = (!config.local.is_empty()).then(|| local::known(&config.local)).filter(|k| k.present(&help_dir()));
        if let (Some(known), false) = (inside, self.hits.is_empty()) {
            // (the model inside the client: loaded on its first question, the answer sent on as it is written)
            let (tx, rx) = channel();
            let (question, hits, dir) = (self.asked.clone(), self.hits.clone(), help_dir());
            std::thread::spawn(move || {
                let kept = LOADED.lock().unwrap_or_else(|e| e.into_inner()).take().filter(|(name, ..)| name == known.name);
                let loaded = match kept {
                    Some((_, model, _)) => Ok(model),
                    None => local::Local::load(&known.weights(&dir), &known.tokenizer(&dir)),
                };
                let began = std::time::Instant::now();
                let result = loaded.and_then(|mut model| {
                    let progress = tx.clone();
                    let answer = local::answer(&mut model, &question, &hits, known.context_chars, &mut move |so_far: &str| progress.send(Said::SoFar(so_far.to_string())).is_ok());
                    *LOADED.lock().unwrap_or_else(|e| e.into_inner()) = Some((known.name.to_string(), model, std::time::Instant::now()));
                    answer
                });
                eprintln!("help model: {} after {:.1} s", result.as_ref().map_or_else(|e| format!("no answer ({e})"), |a| format!("an answer of {} letters", a.text.len())), began.elapsed().as_secs_f32());
                let _ = tx.send(Said::Done(result));
            });
            self.asking = Asking::Waiting(rx, String::new());
        } else if model.configured() && !self.hits.is_empty() {
            let (tx, rx) = channel();
            let (question, hits, dir) = (self.asked.clone(), self.hits.clone(), help_dir());
            std::thread::spawn(move || {
                let key = (!model.key_env.trim().is_empty()).then(|| std::env::var(model.key_env.trim()).unwrap_or_default());
                let _ = tx.send(Said::Done(llm::answer(&mut Curl::new(dir), &model, key.as_deref(), &question, &hits)));
            });
            self.asking = Asking::Waiting(rx, String::new());
        }
    }

    /// Starts fetching the model's files (the player's press of the button), away from the frame.
    fn fetch_model(&mut self, model: &'static local::Known) {
        if !Curl::available() {
            self.model_status = "The curl program, which fetches the files, is not on this system.".into();
            return;
        }
        let (tx, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let (dir, flag) = (help_dir(), stop.clone());
        std::thread::spawn(move || {
            let _ = tx.send(local::fetch_model(&mut Curl::new(&dir), model, &dir, &flag));
        });
        self.model_status.clear();
        self.fetching = Some(Fetching { model, done: rx, stop });
    }

    /// Starts reading a site's guides: every page its sitemap lists that is new or changed, a page every few seconds,
    /// until done or stopped. What is read is kept as it goes.
    fn read_site(&mut self, site: &'static Site) {
        if !Curl::available() {
            self.status = "The curl program, which fetches the pages, is not on this system.".into();
            return;
        }
        let (tx, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let (dir, flag) = (help_dir(), stop.clone());
        std::thread::spawn(move || {
            let store = Store::new(&dir);
            let known = store.load(site.name);
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let result = crawl::crawl(
                &mut Curl::new(&dir),
                site,
                known,
                usize::MAX,
                now,
                &mut |step| match step {
                    Step::Fetching(address, n, of) => {
                        let _ = tx.send(Progress::Page(address.to_string(), n, of));
                    }
                    Step::Checkpoint(docs) => {
                        let _ = store.save(site.name, docs);
                    }
                },
                &|| flag.load(Ordering::Relaxed),
            );
            let _ = tx.send(Progress::Done(result.and_then(|(docs, report)| store.save(site.name, &docs).map(|()| (report, docs.len())).map_err(|e| e.to_string()))));
        });
        self.crawling = Some(Crawling { site, progress: rx, stop, line: format!("Asking {} for its rules and its list of pages...", site.label) });
    }

    /// What the threads have sent since the last frame.
    fn receive(&mut self) {
        if let Some(index) = self.loading.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.loading = None;
            self.index = Some(index);
            if self.pending {
                self.search();
            }
        }
        if let Asking::Waiting(rx, so_far) = &mut self.asking {
            let mut done = None;
            while let Ok(said) = rx.try_recv() {
                match said {
                    Said::SoFar(text) => *so_far = text,
                    Said::Done(result) => done = Some(result),
                }
            }
            if let Some(result) = done {
                self.asking = Asking::Done(result);
            }
        }
        if let Some(result) = self.fetching.as_ref().and_then(|f| f.done.try_recv().ok()) {
            let model = self.fetching.take().map(|f| f.model);
            self.model_status = match (result, model) {
                (Ok(()), Some(m)) => format!("{} is here: answers are now written by it.", m.title),
                (Err(e), _) if e == "stopped" => "Stopped; what has come is kept, and the fetch goes on from there the next time.".to_string(),
                (Err(e), _) => format!("The model was not fetched: {e}"),
                (Ok(()), None) => String::new(),
            };
        }
        // (a model that has not answered for a while gives its memory back)
        if !matches!(self.asking, Asking::Waiting(..)) {
            let mut loaded = LOADED.lock().unwrap_or_else(|e| e.into_inner());
            if loaded.as_ref().is_some_and(|l| l.2.elapsed() > KEEP_LOADED) {
                *loaded = None;
            }
        }
        let mut done = None;
        if let Some(c) = self.crawling.as_mut() {
            while let Ok(progress) = c.progress.try_recv() {
                match progress {
                    Progress::Page(address, n, of) => c.line = format!("{n} of {of}: {}", crawl::path_on(c.site.origin, &address).unwrap_or(&address)),
                    Progress::Done(result) => done = Some((c.site, result)),
                }
            }
        }
        if let Some((site, result)) = done {
            self.crawling = None;
            self.status = match result {
                Ok((report, kept)) => format!("{}: {kept} pages kept ({} read now, {} unchanged, {} failed, {} still to read).", site.label, report.fetched, report.kept, report.failed, report.left),
                Err(e) => format!("{}: {e}", site.label),
            };
            self.reload();
        }
    }

    fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> bool {
        self.receive();
        if !self.open {
            return false;
        }
        if !std::mem::replace(&mut self.started, true) {
            self.reload();
        }
        if self.loading.is_some() || matches!(self.asking, Asking::Waiting(..)) || self.crawling.is_some() || self.fetching.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let weak = egui::Color32::from_gray(150);
        let mut open = self.open;
        let mut changed = false;
        let mut submit = false;
        let mut config_now = config();
        let mut read: Option<&'static Site> = None;
        let mut fetch: Option<&'static local::Known> = None;
        crate::chrome::window(ctx, palette, "Assistant", "Ask the guides", &mut open, |w| w.default_size([470.0, 520.0]), |ui| {
            ui.horizontal(|ui| {
                let field = crate::chrome::text_edit(ui, palette, egui::TextEdit::singleline(&mut self.question).desired_width(ui.available_width() - 60.0).hint_text("a question about the game"));
                if std::mem::take(&mut self.focus) {
                    field.request_focus();
                }
                let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (crate::chrome::button(ui, palette, "Ask", true).clicked() || entered) && !self.question.trim().is_empty() {
                    submit = true;
                }
            });
            ui.separator();
            crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(420.0).auto_shrink([false, true]), |ui| {
                // (the model's answer, when one is set up)
                match &self.asking {
                    Asking::Waiting(_, so_far) if so_far.is_empty() => {
                        ui.label(egui::RichText::new("The model is reading the results...").italics().color(weak));
                        ui.add_space(6.0);
                    }
                    Asking::Waiting(_, so_far) => {
                        ui.label(egui::RichText::new("Answer").strong().color(accent));
                        ui.add(egui::Label::new(so_far.as_str()).wrap());
                        ui.add_space(6.0);
                    }
                    Asking::Done(Ok(answer)) => {
                        ui.label(egui::RichText::new("Answer").strong().color(accent));
                        ui.add(egui::Label::new(&answer.text).wrap());
                        ui.label(egui::RichText::new(format!("Written by the model from results 1 to {}; check it against them.", answer.sources)).small().color(weak));
                        ui.add_space(6.0);
                    }
                    Asking::Done(Err(e)) => {
                        ui.add(egui::Label::new(egui::RichText::new(format!("No answer from the model: {e}")).color(egui::Color32::from_rgb(240, 120, 110))).wrap());
                        ui.add_space(6.0);
                    }
                    Asking::Idle => {}
                }
                if self.loading.is_some() && self.index.is_none() {
                    ui.label(egui::RichText::new("Reading the help pages...").italics().color(weak));
                } else if !self.asked.is_empty() && self.hits.is_empty() && !self.pending {
                    ui.label(format!("Nothing in the {} pages fits \"{}\".", self.index.as_ref().map_or(0, |i| i.docs.len()), self.asked));
                }
                let mut toggle = None;
                for (n, hit) in self.hits.iter().enumerate() {
                    let web = hit.url.starts_with("https://") || hit.url.starts_with("http://");
                    let heading = if hit.heading.is_empty() || hit.heading == hit.title { String::new() } else { format!(": {}", hit.heading) };
                    let title = ui.add(egui::Label::new(egui::RichText::new(format!("{}. {}{heading}", n + 1, hit.title)).strong().color(accent)).wrap().sense(egui::Sense::click()));
                    if title.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        toggle = Some(n);
                    }
                    let from = if hit.source == mao_help::GAME { "the game's help".to_string() } else { crawl::site(&hit.source).map_or(hit.source.clone(), |s| s.label.to_string()) };
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(from).small().color(weak));
                        if web && ui.add(egui::Label::new(egui::RichText::new("open the page").small().underline().color(weak)).sense(egui::Sense::click())).on_hover_text(&hit.url).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            open_outside(&hit.url);
                        }
                    });
                    ui.add(egui::Label::new(if self.expanded == Some(n) { hit.text.as_str() } else { hit.snippet.as_str() }).wrap());
                    ui.add_space(8.0);
                }
                if let Some(n) = toggle {
                    self.expanded = if self.expanded == Some(n) { None } else { Some(n) };
                }
                ui.separator();
                egui::CollapsingHeader::new("Sources and the model").id_salt("assistant-sources").default_open(self.asked.is_empty()).show(ui, |ui| {
                    let mut game = config_now.sources.iter().any(|s| s == mao_help::GAME);
                    if crate::chrome::checkbox(ui, palette, &mut game, "The game's own help files").changed() {
                        config_now.sources.retain(|s| s != mao_help::GAME);
                        if game {
                            config_now.sources.insert(0, mao_help::GAME.to_string());
                        }
                        changed = true;
                    }
                    for site in crawl::SITES {
                        let mut on = config_now.sources.iter().any(|s| s == site.name);
                        if crate::chrome::checkbox(ui, palette, &mut on, site.label).changed() {
                            config_now.sources.retain(|s| s != site.name);
                            if on {
                                config_now.sources.push(site.name.to_string());
                            }
                            changed = true;
                        }
                        ui.add(egui::Label::new(egui::RichText::new(site.terms).small().color(weak)).wrap());
                        if on {
                            ui.horizontal_wrapped(|ui| match self.crawling.as_ref().filter(|c| c.site.name == site.name) {
                                Some(c) => {
                                    if crate::chrome::button(ui, palette, "Stop", true).clicked() {
                                        c.stop.store(true, Ordering::Relaxed);
                                    }
                                    ui.label(egui::RichText::new(&c.line).small());
                                }
                                None => {
                                    if crate::chrome::button(ui, palette, "Read its guides now", self.crawling.is_none()).clicked() {
                                        read = Some(site);
                                    }
                                    ui.label(egui::RichText::new(format!("a page every {} seconds, in the background; new and changed pages only", site.gap)).small().color(weak));
                                }
                            });
                        }
                    }
                    if !self.status.is_empty() {
                        ui.add(egui::Label::new(egui::RichText::new(&self.status).small()).wrap());
                    }
                    if let Some(index) = &self.index {
                        ui.label(egui::RichText::new(format!("{} pages are searched.", index.docs.len())).small().color(weak));
                    }
                    // The model inside the client: a tick to use it; its files fetched when the player says so.
                    let known = local::known(&config_now.local);
                    let mut inside = !config_now.local.is_empty();
                    if crate::chrome::checkbox(ui, palette, &mut inside, "Write an answer in words with the small model inside the client").changed() {
                        config_now.local = if inside { known.name.to_string() } else { String::new() };
                        changed = true;
                    }
                    ui.add(egui::Label::new(egui::RichText::new(format!("{}, run on this machine by the client itself: no account, nothing sent anywhere. It only sees your question and the results found for it. Its answers can still be wrong: check them against the results.", known.title)).small().color(weak)).wrap());
                    if inside {
                        let dir = help_dir();
                        ui.horizontal_wrapped(|ui| match (self.fetching.as_ref(), known.present(&dir)) {
                            (Some(f), _) => {
                                if crate::chrome::button(ui, palette, "Stop", true).clicked() {
                                    f.stop.store(true, Ordering::Relaxed);
                                }
                                let have = local::fetched_bytes(f.model, &dir);
                                ui.label(egui::RichText::new(format!("Fetching the model: {} of {} MB", have / 1_000_000, f.model.weights_bytes / 1_000_000)).small());
                            }
                            (None, true) => {
                                ui.label(egui::RichText::new("The model's files are here.").small().color(weak));
                            }
                            (None, false) => {
                                let have = local::fetched_bytes(known, &dir);
                                let label = if have > 0 { format!("Go on fetching the model ({} of {} MB here)", have / 1_000_000, known.weights_bytes / 1_000_000) } else { format!("Fetch the model ({} MB, once)", known.weights_bytes / 1_000_000) };
                                if crate::chrome::button(ui, palette, &label, true).on_hover_text(known.weights_url).clicked() {
                                    fetch = Some(known);
                                }
                            }
                        });
                    }
                    if !self.model_status.is_empty() {
                        ui.add(egui::Label::new(egui::RichText::new(&self.model_status).small()).wrap());
                    }
                    let model = &config_now.model;
                    let line = if inside {
                        String::new()
                    } else if model.configured() { format!("Answers are written by the model {} at {}.", model.model, model.endpoint) } else { "No language model is set up: results only. One can be named under [help] in mao.ini (endpoint, model).".to_string() };
                    ui.add(egui::Label::new(egui::RichText::new(line).small().color(weak)).wrap());
                });
            });
        });
        self.open = open;
        if changed {
            set_config(config_now);
            self.reload();
        }
        if let Some(site) = read {
            self.read_site(site);
        }
        if let Some(model) = fetch {
            self.fetch_model(model);
        }
        if submit {
            self.asked = self.question.trim().to_string();
            self.search();
        }
        changed
    }
}

/// Draws the window while it is open; true when the player changed the sources (the settings are to be saved).
pub fn show(ctx: &egui::Context, palette: &Palette) -> bool {
    let mut guard = window();
    let w = guard.get_or_insert_with(|| {
        let mut w = fresh();
        // (MAO_UI_PREVIEW=assistant, MAO_HELP_QUESTION=<a question>: the window on that question, for a picture)
        if std::env::var("MAO_UI_PREVIEW").as_deref() == Ok("assistant") {
            w.open = true;
            w.started = true;
            w.index = Some(Arc::new(mao_help::load(&Store::new(help_dir()), crate::theme::game_dir().as_deref(), &config().sources)));
            if let Ok(question) = std::env::var("MAO_HELP_QUESTION") {
                w.question = question.clone();
                w.asked = question;
                w.search();
                // (a picture is taken of the first frames: the answer is waited for here, as it is nowhere else)
                if let Asking::Waiting(rx, _) = &w.asking {
                    let mut done = None;
                    while let Ok(said) = rx.recv() {
                        if let Said::Done(result) = said {
                            done = Some(result);
                            break;
                        }
                    }
                    if let Some(result) = done {
                        w.asking = Asking::Done(result);
                    }
                }
            }
        }
        w
    });
    w.show(ctx, palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_section_is_read_back_as_written() {
        // (the one test that touches the shared settings: it puts them back)
        let before = config();
        set_config(Config::default());
        assert_eq!(config().sources, ["game"]);
        assert!(!config().model.configured());
        for (name, value) in [("sources", "game, AO-Universe"), ("endpoint", "http://localhost:11434/v1/chat/completions"), ("model", "small"), ("api", "anthropic"), ("key_env", "MY_KEY"), ("context_chars", "9000")] {
            assert!(configure(name, value), "{name}");
        }
        assert!(!configure("nothing", "x"));
        let set = config();
        assert_eq!(set.sources, ["game", "ao-universe"]);
        assert_eq!(set.model, Model { api: Api::Anthropic, endpoint: "http://localhost:11434/v1/chat/completions".into(), model: "small".into(), key_env: "MY_KEY".into(), context_chars: 9000 });
        // (what is written is what was set: every line of the section that is not a remark reads back)
        let text = settings_text();
        set_config(Config::default());
        for line in text.lines().filter(|l| !l.starts_with(';') && !l.starts_with('[') && !l.trim().is_empty()) {
            let (name, value) = line.split_once('=').unwrap();
            assert!(configure(name.trim(), value.trim()), "{line}");
        }
        assert_eq!(config(), set);
        set_config(before);
    }
}
