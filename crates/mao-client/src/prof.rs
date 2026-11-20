//! `MAO_PROFILE=1`: where a frame's time goes. The stages of a redraw add their times here; every five seconds the averages
//! and the worst of each are printed, with the frames per second, and then the slowest frames of those seconds stage by
//! stage (`slow:`), with the time between frames that no stage accounts for as `outside` (the event loop, the system).

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How many of the slowest frames each report shows.
const SLOWEST: usize = 3;

struct Totals {
    since: Option<Instant>,
    frames: u32,
    stages: BTreeMap<&'static str, (Duration, Duration, u32)>,
    /// The thread the frames end on (the first to call [`frame`]): only its stages make up a frame's own breakdown, as the
    /// network thread's are its own.
    main: Option<std::thread::ThreadId>,
    /// The main thread's stages since the last frame ended, and when it ended.
    this_frame: Vec<(&'static str, Duration)>,
    last_end: Option<Instant>,
    /// The slowest frames since the last report: the time from the frame before's end, and the stages.
    slowest: Vec<(Duration, Vec<(&'static str, Duration)>)>,
}

static TOTALS: Mutex<Totals> =
    Mutex::new(Totals { since: None, frames: 0, stages: BTreeMap::new(), main: None, this_frame: Vec::new(), last_end: None, slowest: Vec::new() });

/// Whether profiling is on (read once).
pub fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("MAO_PROFILE").is_some())
}

/// Adds a stage's time.
pub fn add(stage: &'static str, took: Duration) {
    if !on() {
        return;
    }
    let mut t = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
    let s = t.stages.entry(stage).or_default();
    s.0 += took;
    s.1 = s.1.max(took);
    s.2 += 1;
    if t.main == Some(std::thread::current().id()) {
        t.this_frame.push((stage, took));
    }
}

/// Times a stage from `start` until now; returns now (the next stage's start).
pub fn lap(stage: &'static str, start: Instant) -> Instant {
    let now = Instant::now();
    add(stage, now - start);
    now
}

/// Keeps a frame among the `SLOWEST` slowest, slowest first.
fn keep_slowest(slowest: &mut Vec<(Duration, Vec<(&'static str, Duration)>)>, took: Duration, stages: Vec<(&'static str, Duration)>) {
    let at = slowest.iter().position(|(t, _)| took > *t).unwrap_or(slowest.len());
    if at < SLOWEST {
        slowest.insert(at, (took, stages));
        slowest.truncate(SLOWEST);
    }
}

/// A slow frame's line: its time, its stages above a tenth of a millisecond (repeats summed, in the order first seen), and
/// what no stage accounts for.
fn slow_line(took: Duration, stages: &[(&'static str, Duration)]) -> String {
    let mut summed: Vec<(&'static str, Duration)> = Vec::new();
    for &(name, d) in stages {
        match summed.iter_mut().find(|(n, _)| *n == name) {
            Some(s) => s.1 += d,
            None => summed.push((name, d)),
        }
    }
    let counted: Duration = summed.iter().map(|(_, d)| *d).sum();
    let mut line = format!("slow: {:.1} ms", took.as_secs_f64() * 1000.0);
    for (name, d) in summed.iter().filter(|(_, d)| *d >= Duration::from_micros(100)) {
        line.push_str(&format!(" | {name} {:.1}", d.as_secs_f64() * 1000.0));
    }
    line.push_str(&format!(" | outside {:.1}", took.saturating_sub(counted).as_secs_f64() * 1000.0));
    line
}

/// Ends a frame; prints the summary every five seconds.
pub fn frame() {
    if !on() {
        return;
    }
    let mut t = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    t.main.get_or_insert_with(|| std::thread::current().id());
    let stages = std::mem::take(&mut t.this_frame);
    if let Some(last) = t.last_end.replace(now) {
        keep_slowest(&mut t.slowest, now - last, stages);
    }
    t.frames += 1;
    let since = *t.since.get_or_insert_with(Instant::now);
    if since.elapsed() >= Duration::from_secs(5) {
        let seconds = since.elapsed().as_secs_f64();
        let mut line = format!("profile: {:.1} fps", t.frames as f64 / seconds);
        for (name, (total, worst, n)) in &t.stages {
            line.push_str(&format!(" | {name} {:.2} ms (worst {:.1}, {n}x)", total.as_secs_f64() * 1000.0 / (*n).max(1) as f64, worst.as_secs_f64() * 1000.0));
        }
        eprintln!("{line}");
        for (took, stages) in &t.slowest {
            eprintln!("{}", slow_line(*took, stages));
        }
        t.frames = 0;
        t.since = Some(Instant::now());
        t.stages.clear();
        t.slowest.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slowest_frames_are_kept_slowest_first() {
        let mut slowest = Vec::new();
        for ms in [5, 30, 8, 12, 40, 1] {
            keep_slowest(&mut slowest, Duration::from_millis(ms), Vec::new());
        }
        assert_eq!(slowest.iter().map(|(d, _)| d.as_millis()).collect::<Vec<_>>(), vec![40, 30, 12]);
    }

    #[test]
    fn a_slow_line_sums_repeats_and_names_what_is_left() {
        let ms = Duration::from_millis;
        let line = slow_line(ms(20), &[("scene", ms(3)), ("ui", ms(4)), ("scene", ms(2)), ("tiny", Duration::from_micros(10))]);
        assert_eq!(line, "slow: 20.0 ms | scene 5.0 | ui 4.0 | outside 11.0");
    }
}
