//! What the client leaves behind when it ends badly, in `crash.log` beside the settings (`mao.ini`): a panic's message, place
//! and backtrace (any thread's: the network thread's too), an error that ended a run, and a run that ended without a clean exit
//! at all (a native crash, a kill, a power loss), noticed by the next start of the same part (the launcher or the game) from the
//! run marker it left (`run-<part>.txt`: "running" while it runs, "ended" after).

use std::io::Write;
use std::path::PathBuf;

fn dir() -> PathBuf {
    let settings = crate::settings::Settings::path();
    settings.parent().map(|p| p.to_path_buf()).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from("."))
}

/// Where the log is.
pub fn log_path() -> PathBuf {
    dir().join("crash.log")
}

fn marker(part: &str) -> PathBuf {
    dir().join(format!("run-{part}.txt"))
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Adds lines to the log (best effort: a log that cannot be written is no reason to fail).
pub fn note(text: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "[{} pid {}] {text}", now(), std::process::id());
    }
}

/// Starts a run of `part`: notes the last one if it never ended, marks this one running, and logs any panic from here on (the
/// default report to stderr still happens).
pub fn start(part: &str, args: &[String]) {
    let path = marker(part);
    if let Ok(last) = std::fs::read_to_string(&path)
        && last.starts_with("running")
    {
        note(&format!("the last {part} run ({}) ended without a clean exit: no panic was caught, so a native crash, a kill or a power loss", last.trim()));
        eprintln!("the last {part} run ended without a clean exit (see {})", log_path().display());
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, format!("running pid {} since {} ({})", std::process::id(), now(), args.join(" ")));
    let part = part.to_string();
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        let place = info.location().map_or_else(String::new, |l| format!(" at {}:{}", l.file(), l.line()));
        let message = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_else(|| "(no message)".into());
        note(&format!("{part}: panic in thread '{thread}'{place}: {message}\n{}", std::backtrace::Backtrace::force_capture()));
        default(info);
    }));
}

/// Ends a run of `part` cleanly (any exit code: an error is noted as well).
pub fn end(part: &str, error: Option<&str>) {
    if let Some(e) = error {
        note(&format!("{part} ended with an error: {e}"));
    }
    let _ = std::fs::write(marker(part), format!("ended at {}", now()));
}
