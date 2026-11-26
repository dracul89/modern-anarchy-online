//! The launcher's own way to the game's files (`mao-install`; `docs/INSTALL_WITHOUT_WINDOWS.md`): what Funcom's
//! downloader, installer and patcher do, in the launcher's window, for a machine that cannot run them.
//!
//! - **No game folder**: PLAY's place holds INSTALL, and the launcher opens on a window that says what is missing
//!   and offers the two ways on: name the folder of a game that is installed (the settings' GUI page), or have the
//!   game downloaded from Funcom's servers into a folder ([`Offer`]). The original's launcher has no such window: it
//!   is part of the installed game.
//! - **A game that needs a patch**: PATCH patches it here ([`Getting::patch`]). On Windows, the one system that can
//!   run Funcom's `AnarchyPatcher.exe`, the launcher's settings can ask for that one (`patcher = funcom` in
//!   `launcher.ini`), started as the original's launcher starts it; the client's own is the default there too (the
//!   user's choice, 2026-11-25). The launcher stays, and PLAY comes when the patch is in.
//!
//! The work is done off the window's thread ([`Job`]) and tells the window what it is at. It can be stopped at any
//! time and goes on from where it stopped the next time: a download keeps what it has, a game folder that is not
//! wholly unpacked is marked so, a patch step is all or nothing. The one stretch that must not be cut is a patch
//! step's writing (the resource database would be left half written): the launcher's window stays for those seconds
//! when it is closed then, and goes when the step is in.
//!
//! For pictures and unattended tests: `MAO_LAUNCH_INSTALL=<folder>` (with no game: the install into that folder
//! starts by itself), `MAO_LAUNCH_ACCEPT=<seconds>` (the licence is accepted that long after it is shown),
//! `MAO_LAUNCH_PATCH=<seconds>` (PATCH presses itself), `MAO_LAUNCH_STOP=<seconds>` (Stop presses itself that long
//! into the work), `MAO_INSTALL_DOWNLOADS=<folder>` (where the installer's parts are, or are kept: not deleted).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mao_install::patcher::{self, Event};
use mao_install::install;

use crate::installcmd::size;

/// Whether PATCH patches here and not with Funcom's patcher.
pub fn own_patcher(prefs: &crate::dimensions::Prefs) -> bool {
    !cfg!(windows) || !prefs.funcom_patcher
}

/// Where the game is installed to unless the player says otherwise: Funcom's own default on Windows (one of the
/// places the launcher looks through by itself), a `Games` folder at home elsewhere.
pub fn default_folder() -> PathBuf {
    if cfg!(windows) {
        return PathBuf::from(r"C:\Funcom\Anarchy Online");
    }
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Games").join("Anarchy Online")
}

/// What a folder is to an install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderState {
    /// No folder that can be used, and why.
    No(&'static str),
    /// Not there yet, or empty.
    Free,
    /// An install into it was cut short.
    Unfinished,
    /// The game is there (its version).
    Game(String),
    NotEmpty,
}

pub fn folder_state(text: &str) -> FolderState {
    let text = text.trim();
    let folder = Path::new(text);
    if text.is_empty() {
        return FolderState::No("Name a folder.");
    }
    if !folder.is_absolute() {
        return FolderState::No("Name the folder by its whole path.");
    }
    if folder.is_file() {
        return FolderState::No("That is a file.");
    }
    if folder.join(install::UNPACKING).exists() {
        return FolderState::Unfinished;
    }
    if let Ok(version) = crate::launchwindow::check_game_dir(folder) {
        return FolderState::Game(version);
    }
    // (the unpacked game cut short in its patching has its version file: the install goes on with the patches)
    if patcher::local_version(folder).is_ok() {
        return FolderState::Unfinished;
    }
    if install::is_free(folder, &install::default_downloads(folder)) { FolderState::Free } else { FolderState::NotEmpty }
}

/// Whether two texts are the same but for their blanks, line ends and marks (the licence in the installer and the
/// game folder's `anarchy.eula`).
pub fn same_words(a: &str, b: &str) -> bool {
    let letters = |t: &str| t.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect::<Vec<char>>();
    let (a, b) = (letters(a), letters(b));
    !a.is_empty() && a == b
}

/// A message of the library's with a capital at its head, as a sentence of the window's.
fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// What the work tells the window.
#[derive(Debug, PartialEq)]
enum Told {
    Status(String),
    /// A download, the unpacking or a step's records: what, how far and of how much (`bytes`: counted in bytes).
    Amount { what: String, done: u64, total: Option<u64>, bytes: bool },
    Step { index: usize, count: usize, to: String },
    /// The licence, which the work waits to have answered.
    Licence(String),
    Done(Result<String, Failure>),
}

/// Why the work ended without the game.
#[derive(Clone, Debug, PartialEq)]
enum Failure {
    Stopped,
    Declined,
    Other(String),
}

#[derive(Default)]
struct Flags {
    /// The window wants the work to end.
    stop: AtomicBool,
    /// A patch step is changing the game's files: not to be cut.
    writing: AtomicBool,
}

/// The work's side: every event told on, the amounts no more often than the window draws.
struct Teller {
    tx: Sender<Told>,
    flags: Arc<Flags>,
    last: Instant,
    what: String,
}

impl Teller {
    fn new(tx: Sender<Told>, flags: Arc<Flags>) -> Teller {
        Teller { tx, flags, last: Instant::now(), what: String::new() }
    }

    fn amount(&mut self, what: &str, done: u64, total: Option<u64>, bytes: bool) {
        if what != self.what || total == Some(done) || self.last.elapsed() >= Duration::from_millis(50) {
            let _ = self.tx.send(Told::Amount { what: what.to_owned(), done, total, bytes });
            self.last = Instant::now();
            self.what = what.to_owned();
        }
    }

    /// The library's progress function: `false` stops it. A step that is writing is never stopped; the mark that it
    /// is writing is set before the wish to stop is looked at, and the window sets its wish before it looks at the
    /// mark, so one of the two always sees the other.
    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Writing => {
                self.flags.writing.store(true, SeqCst);
                if self.flags.stop.load(SeqCst) {
                    self.flags.writing.store(false, SeqCst);
                    return false;
                }
                return true;
            }
            Event::Status(text) => {
                let _ = self.tx.send(Told::Status(text.to_owned()));
            }
            Event::Step { index, count, to, .. } => {
                self.flags.writing.store(false, SeqCst);
                let _ = self.tx.send(Told::Step { index, count, to: to.to_owned() });
            }
            Event::Download { file, done, total } => self.amount(file, done, total, true),
            Event::Unpack { done, total, .. } => self.amount("Unpacking", done, Some(total), true),
            Event::Records { done, total } => self.amount("Database records", done as u64, Some(total as u64), false),
        }
        self.flags.writing.load(SeqCst) || !self.flags.stop.load(SeqCst)
    }
}

/// How an amount reads under its bar.
fn amount_text(what: &str, done: u64, total: Option<u64>, bytes: bool) -> String {
    match (bytes, total) {
        (true, Some(total)) => format!("{what}: {} of {}", size(done), size(total)),
        (true, None) => format!("{what}: {}", size(done)),
        (false, Some(total)) => format!("{what}: {done} of {total}"),
        (false, None) => format!("{what}: {done}"),
    }
}

/// The hooks of the unattended tests.
#[derive(Default)]
struct Hooks {
    install: Option<String>,
    accept_after: Option<f32>,
    patch_after: Option<f32>,
    stop_after: Option<f32>,
}

impl Hooks {
    fn read() -> Hooks {
        let seconds = |name: &str| std::env::var(name).ok().and_then(|v| v.parse().ok());
        Hooks { install: std::env::var("MAO_LAUNCH_INSTALL").ok().filter(|f| !f.is_empty()), accept_after: seconds("MAO_LAUNCH_ACCEPT"), patch_after: seconds("MAO_LAUNCH_PATCH"), stop_after: seconds("MAO_LAUNCH_STOP") }
    }
}

/// An install or a patch at work, and what the window shows of it.
struct Job {
    installing: bool,
    game: PathBuf,
    told: Receiver<Told>,
    answer: Sender<bool>,
    flags: Arc<Flags>,
    status: String,
    amount: Option<(String, u64, Option<u64>, bool)>,
    step: Option<(usize, usize, String)>,
    /// The licence while it waits for its answer, and since when.
    licence: Option<(String, Instant)>,
    /// The licence the player accepted.
    accepted: Option<String>,
    done: Option<Result<String, Failure>>,
    started: Instant,
}

/// How the window's part in a job ended.
enum Ended {
    /// The game is there, at this version.
    Game(String),
    Stopped,
    Declined,
    Again,
    Funcom,
    Closed,
}

impl Job {
    fn start(installing: bool, game: PathBuf) -> Job {
        let (tx, told) = channel();
        let (answer, answers) = channel::<bool>();
        let flags = Arc::<Flags>::default();
        let (folder, worker_flags) = (game.clone(), flags.clone());
        // (a panic in there is in the crash log, and the window is told by the channel's end)
        let spawned = std::thread::Builder::new().name("game-files".into()).spawn(move || {
            let mut teller = Teller::new(tx.clone(), worker_flags.clone());
            let result = if installing { work_install(&folder, &mut teller, &answers) } else { work_patch(&folder, &mut teller) };
            worker_flags.writing.store(false, SeqCst);
            let _ = tx.send(Told::Done(result));
        });
        let done = spawned.err().map(|e| Err(Failure::Other(format!("The work could not be started: {e}"))));
        Job { installing, game, told, answer, flags, status: "Starting...".into(), amount: None, step: None, licence: None, accepted: None, done, started: Instant::now() }
    }

    fn poll(&mut self) {
        loop {
            match self.told.try_recv() {
                Ok(Told::Status(text)) => {
                    self.status = text;
                    self.amount = None;
                }
                Ok(Told::Amount { what, done, total, bytes }) => self.amount = Some((what, done, total, bytes)),
                Ok(Told::Step { index, count, to }) => {
                    self.step = Some((index, count, to));
                    self.amount = None;
                }
                Ok(Told::Licence(text)) => self.licence = Some((text, Instant::now())),
                Ok(Told::Done(result)) => self.done = Some(result),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.done.is_none() {
                        self.done = Some(Err(Failure::Other(format!("It ended without an answer. The details are in {}.", crate::crash::log_path().display()))));
                    }
                    break;
                }
            }
        }
    }

    fn running(&self) -> bool {
        self.done.is_none()
    }

    fn stop(&mut self) {
        self.flags.stop.store(true, SeqCst);
        // (a licence that waits is answered, or the work would wait for ever)
        if self.licence.take().is_some() {
            let _ = self.answer.send(false);
        }
    }

    fn stopping(&self) -> bool {
        self.flags.stop.load(SeqCst)
    }

    fn answer_licence(&mut self, accepted: bool) {
        if let Some((text, _)) = self.licence.take() {
            if accepted {
                self.accepted = Some(text);
            }
            let _ = self.answer.send(accepted);
        }
    }

    /// The job's window for one frame; how it ended, once it has.
    fn show(&mut self, ctx: &egui::Context, hooks: &Hooks) -> Option<Ended> {
        self.poll();
        if self.running() && !self.stopping() && hooks.stop_after.is_some_and(|s| self.started.elapsed().as_secs_f32() > s) {
            self.stop();
        }
        match self.done.clone() {
            Some(Ok(version)) => return Some(Ended::Game(version)),
            Some(Err(Failure::Stopped)) => return Some(Ended::Stopped),
            Some(Err(Failure::Declined)) => return Some(Ended::Declined),
            Some(Err(Failure::Other(why))) => return self.failed(ctx, &why),
            None => {}
        }
        let screen = ctx.content_rect();
        if let Some((text, since)) = &self.licence {
            // (the game's licence, before anything is unpacked: `mao_install::install::Installer::licence`)
            let mut answer = hooks.accept_after.filter(|&s| since.elapsed().as_secs_f32() > s).map(|_| true);
            egui::Window::new("End User License Agreement").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).fixed_size(egui::vec2(screen.width().min(560.0) - 40.0, screen.height().min(480.0) - 90.0)).show(ctx, |ui| {
                ui.label("Funcom's licence for the game (anarchy.eula in Funcom's installer):");
                ui.separator();
                egui::ScrollArea::vertical().max_height(screen.height().min(480.0) - 180.0).show(ui, |ui| {
                    ui.label(text.as_str());
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("I accept").clicked() {
                        answer = Some(true);
                    }
                    if ui.button("I decline").clicked() {
                        answer = Some(false);
                    }
                });
            });
            if let Some(accepted) = answer {
                self.answer_licence(accepted);
            }
            return None;
        }
        let title = if self.installing { "Installing Anarchy Online" } else { "Patching Anarchy Online" };
        let mut stop = false;
        egui::Window::new(title).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
            ui.set_width(440.0);
            ui.label(self.game.display().to_string());
            ui.separator();
            // (the same rows whatever it is at, so the window does not jump)
            ui.label(if self.stopping() { "Stopping..." } else { self.status.as_str() });
            ui.label(match &self.step {
                Some((index, count, to)) => format!("Patch {} of {count}: to version {to}", index + 1),
                None => String::new(),
            });
            let (fraction, text) = match &self.amount {
                Some((what, done, total, bytes)) => (total.map_or(0.0, |t| *done as f32 / t.max(1) as f32), amount_text(what, *done, *total, *bytes)),
                None => (0.0, String::new()),
            };
            ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).desired_width(ui.available_width()));
            ui.label(text);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.stopping(), egui::Button::new("   Stop   ")).clicked() {
                    stop = true;
                }
                ui.weak("It goes on from where it stopped the next time.");
            });
        });
        if stop {
            self.stop();
        }
        None
    }

    /// The window of a job that failed: why, and what can be done.
    fn failed(&mut self, ctx: &egui::Context, why: &str) -> Option<Ended> {
        let mut ended = None;
        let title = if self.installing { "The game could not be installed" } else { "The game could not be patched" };
        egui::Window::new(title).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
            ui.set_width(440.0);
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                ui.label(why);
            });
            ui.add_space(4.0);
            ui.weak("What was done so far is kept: another try goes on from there.");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Try again").clicked() {
                    ended = Some(Ended::Again);
                }
                // (Funcom's own patcher, where it can be run: it asks Windows for an administrator's rights, which a
                // game under the program folders needs)
                if !self.installing && cfg!(windows) && ui.button("Use Funcom's patcher").clicked() {
                    ended = Some(Ended::Funcom);
                }
                if ui.button("Close").clicked() {
                    ended = Some(Ended::Closed);
                }
            });
        });
        ended
    }
}

/// Where the installer's parts go, and whether they stay when the game is in.
fn downloads(game: &Path) -> (PathBuf, bool) {
    match std::env::var_os("MAO_INSTALL_DOWNLOADS").filter(|f| !f.is_empty()) {
        Some(folder) => (PathBuf::from(folder), true),
        None => (install::default_downloads(game), false),
    }
}

fn work_install(game: &Path, teller: &mut Teller, answers: &Receiver<bool>) -> Result<String, Failure> {
    let (downloads, keep) = downloads(game);
    let tx = teller.tx.clone();
    // (the licence goes to the window, and the work waits here for the answer; a window that is gone declines)
    let mut accept = |licence: &str| tx.send(Told::Licence(licence.to_owned())).is_ok() && answers.recv().unwrap_or(false);
    match install::install(game, &downloads, keep, false, &mut accept, &mut |e| teller.event(e)) {
        Ok(()) => patcher::local_version(game).map_err(|e| Failure::Other(sentence(&e.to_string()))),
        Err(install::Error::Stopped) => Err(Failure::Stopped),
        Err(install::Error::Declined) => Err(Failure::Declined),
        // (a part that came down damaged stays damaged: it has to go before the next try)
        Err(e @ install::Error::Package(_)) => Err(Failure::Other(format!("{}.\n\nIf the download was damaged, delete the folder {} and try again.", sentence(&e.to_string()), downloads.display()))),
        // (the system's own words name no file: the folders at least)
        Err(install::Error::Io(e)) => Err(Failure::Other(format!("A file could not be read or written: {e}\n\nThe game's folder: {}\nThe installer's parts: {}", game.display(), downloads.display()))),
        Err(e) => Err(Failure::Other(format!("{}.", sentence(&e.to_string())))),
    }
}

fn work_patch(game: &Path, teller: &mut Teller) -> Result<String, Failure> {
    match patcher::patch(game, &mut |e| teller.event(e)) {
        Ok(_) => patcher::local_version(game).map_err(|e| Failure::Other(sentence(&e.to_string()))),
        Err(patcher::Error::Stopped) => Err(Failure::Stopped),
        // (the system's own words name no file: the folder at least. A game under Windows' program folders cannot
        // be written to without an administrator's rights, which Funcom's patcher asks for)
        Err(patcher::Error::Io(e)) => Err(Failure::Other(format!("A file could not be read or written: {e}\n\nThe game's folder: {}", game.display()))),
        Err(e) => Err(Failure::Other(format!("{}.", sentence(&e.to_string())))),
    }
}

/// The window a launcher without a game opens on.
struct Offer {
    folder: String,
    /// The folder's field as it was last looked at, and what it is.
    checked: Option<(String, FolderState)>,
}

enum Answer {
    ChooseFolder,
    Install,
    Use(String),
    Close,
}

impl Offer {
    fn state(&mut self) -> FolderState {
        if self.checked.as_ref().is_none_or(|(text, _)| *text != self.folder) {
            self.checked = Some((self.folder.clone(), folder_state(&self.folder)));
        }
        self.checked.as_ref().map_or(FolderState::No(""), |(_, state)| state.clone())
    }

    fn show(&mut self, ctx: &egui::Context) -> Option<Answer> {
        let mut answer = None;
        let state = self.state();
        egui::Window::new("Anarchy Online is not on this machine yet").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
            ui.set_width(470.0);
            ui.label("Modern Anarchy Online shows the game from the original game's own files, and found no Anarchy Online folder.");
            ui.add_space(4.0);
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong("The game is installed");
                ui.horizontal(|ui| {
                    ui.label("Name its folder in the settings.");
                    if ui.button("Choose its folder").clicked() {
                        answer = Some(Answer::ChooseFolder);
                    }
                });
            });
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong("Download it from Funcom");
                ui.label("Funcom's own installer is fetched from Funcom's servers (2.1 GB) and the game unpacked from it, without running it; then the game is patched to the version the servers run. The game takes 3 GB, and 5 GB while it is being installed.");
                ui.horizontal(|ui| {
                    ui.label("Into");
                    ui.add(egui::TextEdit::singleline(&mut self.folder).desired_width(320.0));
                    if ui.button("Browse").clicked() {
                        let mut picker = rfd::FileDialog::new().set_title("The folder to install Anarchy Online into");
                        // (the nearest folder above that is there)
                        if let Some(there) = Path::new(self.folder.trim()).ancestors().find(|p| p.is_dir()) {
                            picker = picker.set_directory(there);
                        }
                        if let Some(folder) = picker.pick_folder() {
                            self.folder = folder.display().to_string();
                        }
                    }
                });
                let (good, bad) = (egui::Color32::from_rgb(120, 200, 120), egui::Color32::from_rgb(230, 120, 90));
                match &state {
                    FolderState::No(why) => ui.colored_label(bad, *why),
                    FolderState::Free => ui.colored_label(good, "The folder is free."),
                    FolderState::Unfinished => ui.colored_label(good, "An install into this folder was cut short: it goes on from where it stopped."),
                    FolderState::Game(version) => ui.colored_label(good, format!("The game is in this folder, version {version}.")),
                    FolderState::NotEmpty => ui.colored_label(bad, "This folder has other things in it: name an empty or a new one."),
                };
                ui.horizontal(|ui| match &state {
                    FolderState::Game(_) => {
                        if ui.button("Use this folder").clicked() {
                            answer = Some(Answer::Use(self.folder.trim().to_string()));
                        }
                    }
                    state => {
                        if ui.add_enabled(matches!(state, FolderState::Free | FolderState::Unfinished), egui::Button::new("Download and install")).clicked() {
                            answer = Some(Answer::Install);
                        }
                    }
                });
            });
            ui.separator();
            // (a row of its own height: a layout from the right takes all the height there is in a window that
            // sizes itself)
            ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), ui.spacing().interact_size.y), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    answer = Some(Answer::Close);
                }
            });
        });
        answer
    }
}

/// What the launcher's window is to do.
pub enum Outcome {
    /// The settings are to open at the game folder's page.
    ChooseFolder,
    /// An install into this folder has begun: the settings keep it, so the next start offers it again.
    Began(String),
    /// The folder has the game (named, or installed just now): the launcher starts again with it. `licence`: the
    /// game's licence was accepted on the way.
    Use { folder: String, licence: bool },
    /// The game was patched to this version (`version.id`'s line).
    Patched(String),
    /// Funcom's own patcher is wanted.
    FuncomPatcher,
    /// A line for the window's foot.
    Said(String),
}

/// The launcher's part of getting the game's files: the offer, the job, and the window's closing while a job runs.
#[derive(Default)]
pub struct Getting {
    offer: Option<Offer>,
    job: Option<Job>,
    hooks: Hooks,
    /// The window was closed while a step was writing: it goes when the job has ended.
    closing: bool,
}

impl Getting {
    pub fn new() -> Getting {
        Getting { hooks: Hooks::read(), ..Default::default() }
    }

    /// A window of this is up (the launcher's own controls rest, and its page is away).
    pub fn busy(&self) -> bool {
        self.offer.is_some() || self.job.is_some()
    }

    /// Opens the offer; `kept` is the folder the settings have (an install begun before went there).
    pub fn offer(&mut self, kept: &str) {
        let folder = if kept.trim().is_empty() { default_folder().display().to_string() } else { kept.trim().to_string() };
        self.offer = Some(Offer { folder, checked: None });
    }

    /// Starts patching the game at `game`.
    pub fn patch(&mut self, game: &Path) {
        if self.job.is_none() {
            self.job = Some(Job::start(false, game.to_path_buf()));
        }
    }

    /// `MAO_LAUNCH_PATCH`: PATCH is to press itself now (once).
    pub fn patch_due(&mut self, elapsed: f32) -> bool {
        if self.job.is_none() && self.hooks.patch_after.is_some_and(|s| elapsed > s) {
            self.hooks.patch_after = None;
            return true;
        }
        false
    }

    /// The window is asked to close: whether it may go now. A job is told to stop; while a step is writing the
    /// window stays until the job has ended ([`Getting::closed`]).
    pub fn may_close(&mut self) -> bool {
        let Some(job) = self.job.as_mut().filter(|job| job.running()) else { return true };
        job.stop();
        if job.flags.writing.load(SeqCst) {
            self.closing = true;
            return false;
        }
        true
    }

    /// The window was closed earlier and the job it waited for has ended.
    pub fn closed(&self) -> bool {
        self.closing && self.job.as_ref().is_none_or(|job| !job.running())
    }

    /// The windows for one frame.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Outcome> {
        // (`MAO_LAUNCH_INSTALL`: the offer's folder, and its install pressed)
        let mut answer = None;
        if let (Some(offer), Some(folder)) = (self.offer.as_mut(), self.hooks.install.take()) {
            offer.folder = folder;
            answer = matches!(offer.state(), FolderState::Free | FolderState::Unfinished).then_some(Answer::Install);
        }
        if let Some(offer) = self.offer.as_mut() {
            match answer.or_else(|| offer.show(ctx)) {
                Some(Answer::ChooseFolder) => {
                    self.offer = None;
                    return Some(Outcome::ChooseFolder);
                }
                Some(Answer::Use(folder)) => {
                    self.offer = None;
                    return Some(Outcome::Use { folder, licence: false });
                }
                Some(Answer::Install) => {
                    let folder = offer.folder.trim().to_string();
                    self.offer = None;
                    self.job = Some(Job::start(true, PathBuf::from(&folder)));
                    return Some(Outcome::Began(folder));
                }
                Some(Answer::Close) => self.offer = None,
                None => {}
            }
            return None;
        }
        let job = self.job.as_mut()?;
        let ended = job.show(ctx, &self.hooks)?;
        let job = self.job.take()?;
        match ended {
            Ended::Game(_) if job.installing => {
                // (the licence accepted for the installer is the one the launcher asks for once: `anarchy.eula`)
                let eula = std::fs::read(mao_formats::files::find(&job.game.join("anarchy.eula"))).ok().map(|b| b.iter().map(|&c| c as char).collect::<String>());
                let licence = matches!((&job.accepted, &eula), (Some(accepted), Some(eula)) if same_words(accepted, eula));
                Some(Outcome::Use { folder: job.game.display().to_string(), licence })
            }
            Ended::Game(version) => Some(Outcome::Patched(version)),
            Ended::Stopped => Some(Outcome::Said("Stopped. It goes on from where it stopped the next time.".into())),
            Ended::Declined => Some(Outcome::Said("The licence was declined: nothing was installed.".into())),
            Ended::Again => {
                self.job = Some(Job::start(job.installing, job.game));
                None
            }
            Ended::Funcom => Some(Outcome::FuncomPatcher),
            Ended::Closed => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder is free when it is not there or empty, the game's when it has the game, an unfinished install's
    /// when it has that mark or only the version file.
    #[test]
    fn a_folder_is_told_apart() {
        let base = std::env::temp_dir().join(format!("mao-offer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let text = |name: &str| base.join(name).display().to_string();
        assert_eq!(folder_state(""), FolderState::No("Name a folder."));
        assert_eq!(folder_state("games/ao"), FolderState::No("Name the folder by its whole path."));
        assert_eq!(folder_state(&text("new")), FolderState::Free);
        std::fs::create_dir_all(base.join("empty")).unwrap();
        assert_eq!(folder_state(&format!("  {} ", text("empty"))), FolderState::Free);
        // (only the installer's parts, as after a download that was cut short)
        std::fs::create_dir_all(base.join("parts/mao-installer")).unwrap();
        assert_eq!(folder_state(&text("parts")), FolderState::Free);
        std::fs::create_dir_all(base.join("full")).unwrap();
        std::fs::write(base.join("full/notes.txt"), b"x").unwrap();
        assert_eq!(folder_state(&text("full")), FolderState::NotEmpty);
        assert_eq!(folder_state(&text("full/notes.txt")), FolderState::No("That is a file."));
        std::fs::create_dir_all(base.join("cut")).unwrap();
        std::fs::write(base.join("cut").join(install::UNPACKING), b"").unwrap();
        std::fs::write(base.join("cut/version.id"), "18.8.50_EP1\r\n").unwrap();
        assert_eq!(folder_state(&text("cut")), FolderState::Unfinished);
        std::fs::create_dir_all(base.join("game/cd_image/data/db")).unwrap();
        std::fs::write(base.join("game/version.id"), "18.8.50_EP1\r\n").unwrap();
        assert_eq!(folder_state(&text("game")), FolderState::Unfinished);
        std::fs::write(base.join("game/cd_image/data/db/ResourceDatabase.dat"), b"x").unwrap();
        assert_eq!(folder_state(&text("game")), FolderState::Game("18.8.50_EP1".into()));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The work stops when the window wants it, but never a step that is writing; and a step that has not begun
    /// to write does not begin once the window wants the work to stop.
    #[test]
    fn a_writing_step_is_not_stopped() {
        let (tx, rx) = channel();
        let flags = Arc::<Flags>::default();
        let mut teller = Teller::new(tx, flags.clone());
        assert!(teller.event(Event::Step { index: 0, count: 2, from: "1", to: "2" }));
        assert!(teller.event(Event::Download { file: "A.RTP", done: 5, total: Some(10) }));
        assert!(teller.event(Event::Writing));
        assert!(flags.writing.load(SeqCst));
        flags.stop.store(true, SeqCst);
        // (the step goes on to its end)
        assert!(teller.event(Event::Records { done: 1, total: 3 }));
        assert!(teller.event(Event::Status("Patching files...")));
        // (the next step does not begin)
        assert!(!teller.event(Event::Step { index: 1, count: 2, from: "2", to: "3" }));
        assert!(!flags.writing.load(SeqCst));
        assert!(!teller.event(Event::Download { file: "B.RTP", done: 0, total: None }));
        assert!(!teller.event(Event::Writing));
        assert!(!flags.writing.load(SeqCst));
        let told: Vec<Told> = rx.try_iter().collect();
        assert_eq!(told[0], Told::Step { index: 0, count: 2, to: "2".into() });
        assert_eq!(told[1], Told::Amount { what: "A.RTP".into(), done: 5, total: Some(10), bytes: true });
        assert_eq!(told[2], Told::Amount { what: "Database records".into(), done: 1, total: Some(3), bytes: false });
        assert_eq!(told[3], Told::Status("Patching files...".into()));
    }

    /// The amounts are told when the thing counted changes and when it is whole, not each time between.
    #[test]
    fn amounts_are_told_sparingly() {
        let (tx, rx) = channel();
        let mut teller = Teller::new(tx, Arc::default());
        for done in 0..=1000u64 {
            assert!(teller.event(Event::Download { file: "install-1.bin", done, total: Some(1000) }));
        }
        let told: Vec<Told> = rx.try_iter().collect();
        assert!(told.len() < 100, "{}", told.len());
        assert_eq!(told.first(), Some(&Told::Amount { what: "install-1.bin".into(), done: 0, total: Some(1000), bytes: true }));
        assert_eq!(told.last(), Some(&Told::Amount { what: "install-1.bin".into(), done: 1000, total: Some(1000), bytes: true }));
        assert_eq!(amount_text("install-1.bin", 3 << 20, Some(1 << 30), true), "install-1.bin: 3.0 MB of 1.00 GB");
        assert_eq!(amount_text("Database records", 12, Some(788), false), "Database records: 12 of 788");
        assert_eq!(amount_text("x", 2048, None, true), "x: 2 KB");
    }

    /// A job whose work is gone without an answer has failed; closing waits only for a step that is writing.
    #[test]
    fn closing_waits_for_a_writing_step() {
        let job = |writing: bool| {
            let (tx, told) = channel();
            let (answer, _answers) = channel();
            let flags = Arc::<Flags>::default();
            flags.writing.store(writing, SeqCst);
            (tx, Job { installing: false, game: PathBuf::new(), told, answer, flags, status: String::new(), amount: None, step: None, licence: None, accepted: None, done: None, started: Instant::now() })
        };
        // (downloading: the window may go at once, and the job is told to stop)
        let (_tx, downloading) = job(false);
        let mut getting = Getting { job: Some(downloading), ..Default::default() };
        assert!(getting.may_close() && !getting.closed());
        assert!(getting.job.as_ref().unwrap().stopping());
        // (writing: the window stays until the job has ended)
        let (tx, writing) = job(true);
        let mut getting = Getting { job: Some(writing), ..Default::default() };
        assert!(!getting.may_close() && !getting.closed());
        tx.send(Told::Done(Err(Failure::Stopped))).unwrap();
        getting.job.as_mut().unwrap().poll();
        assert!(getting.closed());
        // (the work gone without a word)
        let (tx, lost) = job(true);
        let mut getting = Getting { job: Some(lost), ..Default::default() };
        assert!(!getting.may_close());
        drop(tx);
        getting.job.as_mut().unwrap().poll();
        assert!(matches!(getting.job.as_ref().unwrap().done, Some(Err(Failure::Other(_)))));
        assert!(getting.closed() && getting.may_close());
        // (no job at all)
        assert!(Getting::default().may_close());
    }

    #[test]
    fn the_two_licence_texts_are_compared_by_their_words() {
        assert!(same_words("END USER\r\nLICENSE  AGREEMENT.\r\n", "End user license agreement"));
        assert!(!same_words("END USER LICENSE AGREEMENT", "END USER LICENSE AGREEMENT 2"));
        assert!(!same_words("", ""));
        assert_eq!(sentence("the patch servers have no patches"), "The patch servers have no patches");
        assert_eq!(sentence(""), "");
    }
}
