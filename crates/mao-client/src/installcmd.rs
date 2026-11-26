//! `mao-client install <folder>` and `mao-client patch <folder>`: the game's files from Funcom's servers, for a
//! machine that cannot run Funcom's installer and patcher (`mao-install`; `docs/INSTALL_WITHOUT_WINDOWS.md`).

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use mao_install::patcher::Event;
use mao_install::{install, patcher};

pub const USAGE: &str = "\
mao-client install <folder> [--accept-eula] [--downloads <folder>] [--keep-downloads] [--allow-unknown-installer]
    downloads Funcom's installer (2.1 GB) from Funcom's servers, unpacks the game into <folder> (2.9 GB) and patches
    it to the version the servers run. Nothing downloaded is run. A run that was cut short goes on where it stopped.
mao-client patch <folder>
    brings an installed game up to the version the servers run, as Funcom's patcher does";

/// A size for a person.
pub fn size(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 30 => format!("{:.2} GB", b as f64 / (1u64 << 30) as f64),
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1u64 << 20) as f64),
        b => format!("{} KB", b.div_ceil(1024)),
    }
}

/// Prints what the installer and the patcher say they are doing: a line for each thing, and for a download or the
/// unpacking a line that is written over as it goes (every whole percent, so a log of the output stays short).
struct Shown {
    last: Option<(String, u64)>,
}

impl Shown {
    fn line(&mut self, text: &str) {
        self.end();
        println!("{text}");
    }

    fn end(&mut self) {
        if self.last.take().is_some() {
            println!();
        }
    }

    fn counted(&mut self, what: &str, done: u64, total: Option<u64>) {
        let percent = total.filter(|&t| t > 0).map_or(done >> 24, |t| done * 100 / t);
        if self.last.as_ref().is_some_and(|(shown, at)| shown == what && *at == percent) {
            return;
        }
        if self.last.as_ref().is_some_and(|(shown, _)| shown != what) {
            println!();
        }
        match total {
            Some(total) => print!("\r  {what}: {} of {} ({percent}%)   ", size(done), size(total)),
            None => print!("\r  {what}: {}   ", size(done)),
        }
        let _ = std::io::stdout().flush();
        self.last = Some((what.to_owned(), percent));
    }

    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Status(text) => self.line(text),
            Event::Download { file, done, total } => self.counted(file, done, total),
            Event::Unpack { done, total, .. } => self.counted("unpacking", done, Some(total)),
            Event::Step { index, count, from, to } => self.line(&format!("Patching version {from} to {to} ({} of {count}).", index + 1)),
            Event::Records { .. } | Event::Writing => {}
        }
        true
    }
}

fn patch(game: &Path) -> Result<(), String> {
    let mut shown = Shown { last: None };
    let local = patcher::local_version(game).map_err(|e| e.to_string())?;
    println!("Current version is : {local}");
    let plan = patcher::plan(game, &mut |e| shown.event(e)).map_err(|e| e.to_string())?;
    if plan.steps.is_empty() {
        shown.line(&format!("The game is at the servers' version, {}. Nothing to patch.", plan.target));
        return Ok(());
    }
    shown.line(&format!("Remote version: {}; {} patches to apply.", plan.target, plan.steps.len()));
    let result = patcher::run(game, &plan, &mut |e| shown.event(e));
    shown.end();
    result.map_err(|e| e.to_string())?;
    println!("Patching has completed. The game is at version {}.", plan.target);
    Ok(())
}

/// Shows the licence and asks; `false` when it is declined or cannot be asked.
fn ask(licence: &str) -> bool {
    println!("\n{}\n", licence.trim());
    if !std::io::stdin().is_terminal() {
        eprintln!("This is Funcom's licence for the game. Run the command again with --accept-eula to accept it.");
        return false;
    }
    print!("This is Funcom's licence for the game. Do you accept it? [y/N] ");
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    let _ = std::io::stdin().lock().read_line(&mut answer);
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn run_install(args: &[String]) -> Result<(), String> {
    let mut folder: Option<PathBuf> = None;
    let mut downloads: Option<PathBuf> = None;
    let (mut accepted, mut keep, mut allow_unknown) = (false, false, false);
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--accept-eula" => accepted = true,
            "--keep-downloads" => keep = true,
            "--allow-unknown-installer" => allow_unknown = true,
            "--downloads" => downloads = Some(PathBuf::from(rest.next().ok_or("--downloads needs a folder")?)),
            other if other.starts_with("--") => return Err(format!("unknown option {other}\n{USAGE}")),
            other if folder.is_none() => folder = Some(PathBuf::from(other)),
            _ => return Err(USAGE.into()),
        }
    }
    let game = folder.ok_or(USAGE)?;
    // (a folder of the downloads that the caller named is the caller's to keep)
    let keep = keep || downloads.is_some();
    let downloads = downloads.unwrap_or_else(|| install::default_downloads(&game));
    let mut shown = Shown { last: None };
    println!("Installing Anarchy Online into {} from Funcom's servers.", game.display());
    let mut accept = |licence: &str| accepted || ask(licence);
    let result = install::install(&game, &downloads, keep, allow_unknown, &mut accept, &mut |e| shown.event(e));
    shown.end();
    result.map_err(|e| match e {
        // (a part that came down damaged stays damaged: it has to go before the next try)
        install::Error::Package(_) => format!("{e}
If the download was damaged, delete {} and run the command again.", downloads.display()),
        e => e.to_string(),
    })?;
    let version = patcher::local_version(&game).map_err(|e| e.to_string())?;
    println!("Anarchy Online {version} is in {}.", game.display());
    println!("Start it with: mao-client \"{}\" launch", game.display());
    Ok(())
}

/// `install ...` and `patch ...`: `None` when the words are neither.
pub fn run(args: &[String]) -> Option<Result<(), String>> {
    match args {
        [word, rest @ ..] if word == "install" => Some(run_install(rest)),
        [word, folder] if word == "patch" => Some(patch(Path::new(folder))),
        [word, ..] if word == "patch" => Some(Err(USAGE.into())),
        _ => None,
    }
}
