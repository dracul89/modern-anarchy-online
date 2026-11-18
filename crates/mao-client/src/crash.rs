//! What the client leaves behind when it ends badly, in `crash.log` beside the settings (`mao.ini`): a panic's message, place
//! and backtrace (any thread's: the network thread's too), an error that ended a run, and a run that ended without a clean exit
//! at all (a native crash, a kill, a power loss), noticed by the next start of the same part (the launcher or the game) from the
//! run marker it left (`run-<part>.txt`: "running" while it runs, "ended" after).
//!
//! The packaged program has no console (`hide-console`), so what ends the launcher or the login is also told in a box
//! of the system's ([`tell`]): a panic of the main thread, and the error a run ended with. A part the window can do
//! without (a picture, the news page) runs [`guarded`]: its panic is logged and the window goes on.

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

thread_local! {
    /// How many [`guarded`] calls this thread is inside: a panic there is caught, so it is logged and not told in a box.
    static GUARDED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// One box a run: what follows the first failure is its consequence.
static TOLD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Runs `f`; `None` when it panicked. For what the window can do without: the panic is in the log (when a run was
/// started) and the caller goes on.
pub fn guarded<T>(what: &str, f: impl FnOnce() -> T) -> Option<T> {
    GUARDED.with(|g| g.set(g.get() + 1));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    GUARDED.with(|g| g.set(g.get().saturating_sub(1)));
    if result.is_err() {
        eprintln!("{what} failed (see {}): going on without", log_path().display());
    }
    result.ok()
}

/// Whether a failure is told in a box: by default where there is no console to say it in (the packaged program);
/// `MAO_ERROR_BOX=1` asks for the box in any build, `MAO_ERROR_BOX=0` forbids it (an unattended run must not wait for
/// a click).
fn boxes(setting: Option<&str>, packaged: bool) -> bool {
    match setting.map(str::trim) {
        Some("0") => false,
        Some(_) => true,
        None => packaged,
    }
}

/// What the box says: what happened and where the details are.
pub fn box_text(what: &str) -> String {
    format!("{what}\n\nThe details are in:\n{}", log_path().display())
}

/// No box for this run's ending (an ending that was told already, or that is no fault of this process).
pub fn no_box() {
    TOLD.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Tells the person what ended the run, in a box of the system's (once a run; on the main thread only, as the
/// systems want it).
pub fn tell(what: &str) {
    if !boxes(std::env::var("MAO_ERROR_BOX").ok().as_deref(), cfg!(feature = "hide-console")) || TOLD.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    #[cfg(any(windows, target_os = "macos"))]
    {
        let _ = rfd::MessageDialog::new().set_level(rfd::MessageLevel::Error).set_title("Modern Anarchy Online").set_description(box_text(what)).set_buttons(rfd::MessageButtons::Ok).show();
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = what;
}

/// Starts a run of `part`: notes the last one if it never ended, marks this one running, and logs any panic from here on (the
/// default report to stderr still happens). A panic of the main thread that nothing catches ends the program: it is
/// told in a box as well.
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
        if thread == "main" && GUARDED.with(|g| g.get()) == 0 {
            // (the run is over, and it is known how: the next start has nothing to wonder at)
            let _ = std::fs::write(marker(&part), format!("ended by a panic at {}", now()));
            tell(&format!("Modern Anarchy Online ran into an error and has to close.\n\n{message}{place}"));
        }
    }));
}

/// Ends a run of `part` cleanly (any exit code: an error is noted as well).
pub fn end(part: &str, error: Option<&str>) {
    if let Some(e) = error {
        note(&format!("{part} ended with an error: {e}"));
    }
    let _ = std::fs::write(marker(part), format!("ended at {}", now()));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A guarded part that panics gives nothing and the caller goes on; one that does not gives its value.
    #[test]
    fn a_guarded_part_may_fail() {
        assert_eq!(guarded("a sum", || 2 + 2), Some(4));
        let quiet = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let none: Option<u32> = guarded("a part that fails", || panic!("as meant"));
        std::panic::set_hook(quiet);
        assert_eq!(none, None);
        assert_eq!(GUARDED.with(|g| g.get()), 0);
    }

    /// The box is for the packaged program, unless the setting says otherwise; its text names the log.
    #[test]
    fn the_box_is_the_packaged_programs() {
        assert_eq!((boxes(None, true), boxes(None, false)), (true, false));
        assert_eq!((boxes(Some("1"), false), boxes(Some("0"), true), boxes(Some(" 0 "), true)), (true, false, false));
        let text = box_text("No graphics device could be started.");
        assert!(text.starts_with("No graphics device could be started.") && text.contains("crash.log"), "{text}");
    }
}
