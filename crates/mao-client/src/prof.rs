//! `MAO_PROFILE=1`: where a frame's time goes. The stages of a redraw add their times here; every five seconds the averages
//! and the worst of each are printed, with the frames per second.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Totals {
    since: Option<Instant>,
    frames: u32,
    stages: BTreeMap<&'static str, (Duration, Duration, u32)>,
}

static TOTALS: Mutex<Totals> = Mutex::new(Totals { since: None, frames: 0, stages: BTreeMap::new() });

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
}

/// Times a stage from `start` until now; returns now (the next stage's start).
pub fn lap(stage: &'static str, start: Instant) -> Instant {
    let now = Instant::now();
    add(stage, now - start);
    now
}

/// Ends a frame; prints the summary every five seconds.
pub fn frame() {
    if !on() {
        return;
    }
    let mut t = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
    t.frames += 1;
    let since = *t.since.get_or_insert_with(Instant::now);
    if since.elapsed() >= Duration::from_secs(5) {
        let seconds = since.elapsed().as_secs_f64();
        let mut line = format!("profile: {:.1} fps", t.frames as f64 / seconds);
        for (name, (total, worst, n)) in &t.stages {
            line.push_str(&format!(" | {name} {:.2} ms (worst {:.1}, {n}x)", total.as_secs_f64() * 1000.0 / (*n).max(1) as f64, worst.as_secs_f64() * 1000.0));
        }
        eprintln!("{line}");
        t.frames = 0;
        t.since = Some(Instant::now());
        t.stages.clear();
    }
}
