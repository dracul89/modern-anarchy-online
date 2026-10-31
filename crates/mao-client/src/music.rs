//! The interactive music player: SIMPlayer.dll's `SIMPlayer_t`, `SampleManager_t` and `PauseControl_c` (the `simplayer`
//! drop-in's `player.rs`) over the project of [`mao_formats::music`], playing on the sound engine's streams
//! ([`crate::mixer::Streams`]).
//!
//! The player keeps three streams by role: the one playing, the one ending and the one being prepared. A thread steps it every
//! 10 ms through a state machine: 0 stopped (an entry sample of the event's layer restarted when asked), 1 playing (the next
//! transition picked), 2 a transition picked (its sample prepared 200 ms before the fade out), 3 prepared (started at the fade
//! out), 4 cross-fading by the fades' tables (then the prepared stream plays: state 1), 5 no transition (waiting for the
//! stream's end), 6 ended (an event starts an entry sample), 7 stopping at the stream's end. The manager picks the next
//! transition (FUN_10008d2e): to a sample of the event's layer, the earliest after the position (by weight among those as
//! early); else, forced, one made to an entry sample of that layer when there is time; else one within the layer. A layer's
//! pause track pauses the music after its play beats and plays again after its pause beats.
//!
//! The driver's latency (`AIL_digital_latency`) is 0 here: the streams start on the same mixer that times them.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mao_formats::music::{Fade, Project, Transition};

use crate::audio::Rand;
use crate::mixer::{Pcm, Streams, DONE, PLAYING};

/// `CFadeInfo::s_nFadeElementInterval` as the player sets it (FUN_1000984d: 50 ms).
const INTERVAL: u32 = 50;

/// A transition under way: the sample it leaves and the transition (a project's, by its place, or one the manager made).
#[derive(Clone, Debug)]
struct Taken {
    from: usize,
    /// The project's transition's place in its sample's list (its pick weight's key), none for a made one.
    index: Option<usize>,
    t: Transition,
}

/// `SampleManager_t`.
#[derive(Default)]
struct Manager {
    sample: Option<usize>,
    result: Option<Taken>,
    position: u32,
    layer: i32,
    done: bool,
    kind: i32,
    time: u32,
}

/// `PauseControl_c`.
#[derive(Default)]
struct PauseControl {
    enabled: bool,
    active: bool,
    running: bool,
    track: Option<mao_formats::music::PauseTrack>,
    index: u32,
    node: (i32, i32),
    elapsed: u32,
    last: Option<Instant>,
    sample: Option<usize>,
    playing: bool,
}

/// `SIMPlayer_t`.
pub struct Player {
    project: Arc<Project>,
    /// The project's directory (the layers' directories are under it).
    directory: PathBuf,
    streams: Streams,
    handles: [Option<usize>; 3],
    samples: [Option<usize>; 3],
    playing: usize,
    ending: usize,
    preparing: usize,
    layer_volumes: [f32; 3],
    fades: [f32; 3],
    volume: f32,
    pause_volume: f32,
    transition: Option<Taken>,
    /// The pick weights of the project's transitions (+0x34, 1 to start).
    weights: std::collections::HashMap<(usize, usize), i32>,
    state: i32,
    no_entry: bool,
    stop_asked: bool,
    restart: bool,
    paused: i32,
    last_position: i32,
    fade_in: Vec<f32>,
    fade_out: Vec<f32>,
    fade_start: u32,
    fade_length: u32,
    timer: Option<Instant>,
    manager: Manager,
    pause: PauseControl,
    rand: Rand,
    /// Decoded samples, by path (the streams read their files whole).
    cache: std::collections::HashMap<PathBuf, Arc<Pcm>>,
}

/// A fade's table value at `ms` (FUN_1000924c: clamped).
fn fade_value(table: &[f32], ms: u32) -> f32 {
    let i = (ms / INTERVAL) as i32;
    let last = table.len() as i32 - 1;
    let i = if i.min(last) < 0 { 0 } else { i.min(last) };
    table.get(i as usize).copied().unwrap_or(0.0)
}

/// `CTransition(from, to)`: the samples' fades, out at the first's end less the second's offset, the second's offset the time.
fn between(project: &Project, from: usize, to: usize) -> Transition {
    let (f, t) = (&project.samples[from], &project.samples[to]);
    Transition { to, priority: 0, fade_out: f.fade_out, fade_in: t.fade_in, out_time: f.end_time.wrapping_sub(t.offset) as i32, in_position: -1, in_time: 0, time: t.offset as i32 }
}

impl Player {
    /// `SIMPlayer_t(driver, path)` with `FUN_1000984d`'s fields, over a loaded project; the pause track of a random entry
    /// sample's layer.
    pub fn new(project: Arc<Project>, directory: &Path, streams: Streams) -> Player {
        let mut p = Player {
            project,
            directory: directory.to_path_buf(),
            streams,
            handles: [None; 3],
            samples: [None; 3],
            playing: 0,
            ending: 2,
            preparing: 1,
            layer_volumes: [1.0; 3],
            fades: [0.0; 3],
            volume: 1.0,
            pause_volume: 1.0,
            transition: None,
            weights: Default::default(),
            state: 0,
            no_entry: false,
            stop_asked: false,
            restart: false,
            paused: 0,
            last_position: 0,
            fade_in: Vec::new(),
            fade_out: Vec::new(),
            fade_start: 0,
            fade_length: 0,
            timer: None,
            manager: Manager { layer: -1, ..Manager::default() },
            pause: PauseControl { enabled: true, ..PauseControl::default() },
            // (the original seeds the CRT with the tick count)
            rand: Rand::default(),
            cache: Default::default(),
        };
        if let Some(e) = p.entry_sample(-1, false) {
            let layer = p.project.samples[e].layer;
            let track = p.project.layer(layer).and_then(|l| l.pause.clone());
            p.pause_set_track(track);
        }
        p
    }

    /// `CProject::GetEntrySample(layer, any)`: a random entry sample of the layer (of a random layer for -1); of all its samples
    /// when it has none and `any`.
    fn entry_sample(&mut self, mut layer: i32, any: bool) -> Option<usize> {
        if layer == -1 {
            if self.project.layers.is_empty() {
                return None;
            }
            // (the original's layers are a map by id: the k-th by id)
            let mut ids: Vec<i32> = self.project.layers.iter().map(|l| l.id).collect();
            ids.sort();
            layer = ids[(self.rand.next() % ids.len() as i32) as usize];
        }
        self.project.layer(layer)?;
        let of_layer: Vec<usize> = (0..self.project.samples.len()).filter(|&i| self.project.samples[i].layer == layer).collect();
        let entries: Vec<usize> = of_layer.iter().copied().filter(|&i| self.project.samples[i].entry).collect();
        let pool = if !entries.is_empty() || !any { entries } else { of_layer };
        if pool.is_empty() { None } else { Some(pool[(self.rand.next() % pool.len() as i32) as usize]) }
    }

    /// The sample's file: `<directory><layer>/<name>.wav` (`GetAbsoluteSampleName`, its `\` made `/`).
    fn path(&self, s: usize) -> Option<PathBuf> {
        let sample = &self.project.samples[s];
        let layer = self.project.layer(sample.layer)?;
        Some(self.directory.join(layer.name.replace('\\', "/")).join(format!("{}.wav", sample.name)))
    }

    /// `AIL_open_stream` of the sample's file.
    fn open(&mut self, s: usize) -> Option<usize> {
        let path = self.path(s)?;
        let pcm = match self.cache.get(&path) {
            Some(p) => p.clone(),
            None => {
                let pcm = Arc::new(Pcm::decode(&mao_formats::files::read(&path).ok()?)?);
                if self.cache.len() > 64 {
                    self.cache.clear();
                }
                self.cache.insert(path, pcm.clone());
                pcm
            }
        };
        Some(self.streams.open(pcm))
    }

    fn close(&mut self, i: usize) {
        if let Some(h) = self.handles[i].take() {
            self.streams.close(h);
        }
    }

    /// FUN_100092dd: each stream's level from its layer, the volumes, its sample and its fade.
    fn set_volumes(&mut self, volume: f32, pause: f32, force: bool) {
        if !force && volume == self.volume && pause == self.pause_volume {
            return;
        }
        self.volume = volume;
        self.pause_volume = pause;
        let both = volume * pause;
        for i in 0..3 {
            if let (Some(h), Some(s)) = (self.handles[i], self.samples[i]) {
                let level = self.layer_volumes[i] * both * self.project.samples[s].volume * self.fades[i];
                self.streams.set_volume(h, level);
            }
        }
    }

    /// FUN_10009374: the layer's volume of the stream's sample (1 for none).
    fn layer_volume(&self, i: usize) -> f32 {
        self.samples[i].and_then(|s| self.project.layer(self.project.samples[s].layer)).map_or(1.0, |l| l.volume)
    }

    /// `SetVolume(volume)`.
    pub fn set_volume(&mut self, volume: f32) {
        let pause = self.pause_volume;
        self.set_volumes(volume, pause, false);
    }

    fn position(&self) -> (i32, i32) {
        self.handles[self.playing].map_or((0, 0), |h| self.streams.ms_position(h))
    }

    /// FUN_10009529: at the stream's end, stopped (state 7 to 0) or ended (5 to 6); else waiting on for what is left.
    fn timer_fired(&mut self) {
        self.timer = None;
        let h = self.handles[self.playing];
        let (total, current) = self.position();
        let ended = h.is_none_or(|h| self.streams.status(h) == DONE) || current >= total;
        if ended {
            match self.state {
                7 => self.state = 0,
                5 => self.state = 6,
                _ => {}
            }
        } else {
            self.timer = Some(Instant::now() + Duration::from_millis((total - current).max(0) as u64));
        }
    }

    /// `Stop(now, tracks)`: the streams paused now (stopped), or at the stream's end (a stop mid-fade waits for its end); the
    /// sample the transition under way goes to.
    pub fn stop(&mut self, now: bool, tracks: bool) -> Option<usize> {
        if tracks {
            self.pause.running = false;
        }
        if now {
            for h in self.handles.into_iter().flatten() {
                self.streams.pause(h, true);
            }
            self.state = 0;
        } else if self.state == 4 {
            self.stop_asked = true;
        } else if self.state != 0 {
            self.state = 7;
            let (total, current) = self.position();
            self.timer = Some(Instant::now() + Duration::from_millis((total - current).max(0) as u64));
        }
        self.transition.as_ref().map(|t| t.t.to)
    }

    /// `Pause(now, tracks)`.
    fn pause_player(&mut self, now: bool, tracks: bool) -> Option<usize> {
        self.paused = tracks as i32 + 1;
        self.stop(now, tracks)
    }

    /// FUN_10008c7e: a transition picked at random by weight, the others' weights raised (all made 1 when one passes 999).
    fn pick(&mut self, from: usize, candidates: &[usize]) -> Option<Taken> {
        if candidates.is_empty() {
            return None;
        }
        let weight = |p: &Player, k: usize| p.weights.get(&(from, k)).copied().unwrap_or(1);
        let total = candidates.iter().fold(0i32, |s, &k| s.wrapping_add(weight(self, k)));
        let mut r = self.rand.next() % total.max(1);
        let mut i = 0;
        while r > weight(self, candidates[i]) && i + 1 < candidates.len() {
            r -= weight(self, candidates[i]);
            i += 1;
        }
        let chosen = candidates[i];
        let mut over = false;
        for &k in candidates {
            if k != chosen {
                let w = self.weights.entry((from, k)).or_insert(1);
                *w += 1;
                over |= *w > 999;
            }
        }
        if over {
            for &k in candidates {
                self.weights.insert((from, k), 1);
            }
        }
        Some(Taken { from, index: Some(chosen), t: self.project.samples[from].transitions[chosen].clone() })
    }

    /// FUN_10008bd8: a made transition's fades by the event's kind: from 1 to 0 and 0 to 1, along an arc below kind 7, with the
    /// curves' factors by kind; its time the manager's.
    fn manager_fades(&self, t: &mut Transition, from_end: u32) {
        let m = &self.manager;
        // (`SetFadeTime`: the time, and the fade out from the first sample's end)
        t.time = m.time as i32;
        t.out_time = from_end.wrapping_sub(m.time) as i32;
        let (out, inn) = (&mut t.fade_out, &mut t.fade_in);
        *out = Fade { level_in: 1.0, level_out: 0.0, ..*out };
        *inn = Fade { level_in: 0.0, level_out: 1.0, ..*inn };
        let curve = if m.kind < 7 { 1 } else { 2 };
        out.curve = curve;
        inn.curve = curve;
        let factors = match m.kind {
            2 => Some((0.5, 0.5)),
            3 | 7 => Some((-1.0, 1.0)),
            4 | 8 => Some((-1.0, -1.0)),
            5 | 9 => Some((1.0, 1.0)),
            6 => Some((1.0, -1.0)),
            _ => None,
        };
        if let Some((o, i)) = factors {
            out.factor = o;
            inn.factor = i;
        }
    }

    /// FUN_10008d2e: the transition to take from the sample at the position (remembered until the sample changes or plays back).
    fn manager_next(&mut self, s: usize, position: u32) -> Option<Taken> {
        if self.manager.layer == -1 {
            self.manager.layer = self.project.samples[s].layer;
        }
        let target = self.manager.layer;
        if Some(s) != self.manager.sample || position < self.manager.position {
            self.manager.result = None;
            self.manager.sample = Some(s);
            self.manager.done = false;
        }
        self.manager.position = position;
        let mut result = self.manager.result.clone();
        let mut done = self.manager.done;
        if !self.manager.done {
            let layer = self.project.samples[s].layer;
            let (mut same, mut candidates, mut best) = (Vec::new(), Vec::new(), u32::MAX);
            for (k, t) in self.project.samples[s].transitions.iter().enumerate() {
                let to_layer = self.project.samples[t.to].layer;
                let at = (t.out_time as u32).wrapping_sub(10);
                if at > position {
                    if to_layer == layer {
                        same.push(k);
                    } else if to_layer == target {
                        if candidates.is_empty() || at < best {
                            candidates.clear();
                            candidates.push(k);
                            best = at;
                        } else if at == best {
                            candidates.push(k);
                        }
                    }
                }
            }
            result = if layer != target && !candidates.is_empty() {
                self.pick(s, &candidates)
            } else if layer == target || self.manager.kind == 0 {
                self.pick(s, &same)
            } else {
                match self.entry_sample(target, false) {
                    None => None,
                    Some(e) => {
                        let a = self.project.samples[s].end_time;
                        let x = self.manager.time.min((a - (a >> 2)).wrapping_sub(10));
                        let end = self.project.samples[e].end_time;
                        let x = x.min((end - (end >> 2)).wrapping_sub(10));
                        self.manager.time = x;
                        if position < a.wrapping_sub(x).wrapping_sub(10) {
                            let mut t = between(&self.project, s, e);
                            if self.manager.kind != 1 {
                                self.manager_fades(&mut t, a);
                            }
                            done = true;
                            Some(Taken { from: s, index: None, t })
                        } else {
                            self.pick(s, &same)
                        }
                    }
                }
            };
            if result.is_none() {
                done = true;
            }
        }
        self.manager.result = result.clone();
        self.manager.done = done;
        result
    }

    /// FUN_10002712: the pause track (none stops it).
    fn pause_set_track(&mut self, track: Option<mao_formats::music::PauseTrack>) {
        let p = &mut self.pause;
        p.sample = None;
        p.index = 0;
        p.active = p.enabled && track.is_some();
        p.track = track;
        p.running = false;
    }

    /// FUN_1000289f: the track from its first node, now.
    fn pause_start(&mut self) {
        let p = &mut self.pause;
        p.sample = None;
        p.playing = true;
        p.last = Some(Instant::now());
        p.elapsed = 0;
        p.index = 0;
        if p.active {
            if let Some(&n) = p.track.as_ref().and_then(|t| t.nodes.first()) {
                p.node = n;
            }
        }
        p.running = true;
    }

    /// FUN_10002761: the time passed; paused after a node's play beats, playing again after its pause beats (then the next).
    fn pause_tick(&mut self) {
        if !self.pause.running || !self.pause.active {
            return;
        }
        let now = Instant::now();
        let d = self.pause.last.map_or(0, |l| now.duration_since(l).as_millis() as u32);
        self.pause.elapsed = self.pause.elapsed.wrapping_add(d);
        self.pause.last = Some(now);
        let Some(track) = self.pause.track.clone() else { return };
        let beat = 60000 / track.bpm.max(1);
        let (play, pause) = self.pause.node;
        let end = (pause.wrapping_add(play)).wrapping_mul(beat) as u32;
        let play_ms = play.wrapping_mul(beat) as u32;
        if self.pause.elapsed >= end {
            if !self.pause.playing {
                self.pause.playing = true;
                let s = self.pause.sample;
                self.play(s, false);
            }
            // (`GetNextNode`: back to the loop node after the last)
            self.pause.index = self.pause.index.wrapping_add(1);
            if (track.nodes.len() as u32).wrapping_sub(1) < self.pause.index {
                self.pause.index = track.loop_to;
            }
            self.pause.elapsed = self.pause.elapsed.wrapping_sub(end);
            if let Some(&n) = track.nodes.get(self.pause.index as usize) {
                self.pause.node = n;
            }
        } else if self.pause.elapsed > play_ms && self.pause.playing {
            self.pause.playing = false;
            self.pause.sample = self.pause_player(false, false);
        }
    }

    /// FUN_10009ffb: the sample alone on the first stream, started at full fade.
    fn start_sample(&mut self, s: usize) {
        for i in 0..3 {
            self.close(i);
        }
        self.stop_asked = false;
        self.restart = false;
        self.paused = 0;
        self.transition = None;
        (self.playing, self.ending, self.preparing) = (0, 2, 1);
        self.samples[0] = Some(s);
        self.handles[0] = self.open(s);
        if let Some(h) = self.handles[0] {
            self.fades = [0.0; 3];
            self.fades[self.playing] = 1.0;
            for i in 0..3 {
                self.layer_volumes[i] = self.layer_volume(i);
            }
            let (v, p) = (self.volume, self.pause_volume);
            self.set_volumes(v, p, true);
            self.streams.start(h);
            self.state = 1;
        }
    }

    /// `Play(sample, tracks)`: the sample (an entry sample of the event's layer for none), the pause tracks started if asked.
    pub fn play(&mut self, s: Option<usize>, tracks: bool) {
        self.stop(true, tracks);
        let s = s.or_else(|| {
            let layer = self.manager.layer;
            let e = self.entry_sample(layer, false);
            self.no_entry = e.is_none();
            e
        });
        if let Some(s) = s {
            self.start_sample(s);
            if tracks {
                self.pause_start();
            }
        }
    }

    /// `SignalEvent(layer, force, time)`: the event for the manager (a new layer starts its search again); then an entry sample
    /// of the layer played if the music had ended (or a pause tracks' pause), else the search started again.
    pub fn signal_event(&mut self, layer: i32, force: i32, time: u32) {
        if layer != self.manager.layer {
            self.manager.layer = layer;
            self.manager.kind = force;
            self.manager.time = time;
            self.manager.result = None;
            self.manager.done = false;
        }
        match self.state {
            6 => {
                let s = self.entry_sample(layer, true);
                self.play(s, true);
            }
            0 if self.paused == 1 => self.play(None, true),
            5 => {
                self.timer = None;
                self.state = 1;
            }
            2 => self.state = 1,
            _ if self.paused == 1 => self.restart = true,
            _ => {}
        }
    }

    /// FUN_10009acd (state 1): the next transition (state 2); none, the stream's end waited for (state 5).
    fn choose_next(&mut self, position: i32, total: i32) {
        self.last_position = position;
        if self.handles[self.playing].is_none() {
            return;
        }
        if self.stop_asked {
            self.stop_asked = false;
            self.stop(false, true);
            return;
        }
        let Some(s) = self.samples[self.playing] else { return };
        if let Some(t) = self.manager_next(s, position as u32) {
            self.transition = Some(t);
            self.state = 2;
        }
        if self.transition.is_none() {
            self.state = 5;
            self.timer = Some(Instant::now() + Duration::from_millis((total - position).max(0) as u64));
        }
    }

    /// FUN_1000a143: the transition's sample opened on the prepared stream (at its fade in, paused); false when the ending one
    /// hadn't ended or the file can't be played.
    fn prepare(&mut self) -> bool {
        let mut ok = true;
        if let Some(h) = self.handles[self.preparing] {
            if self.streams.status(h) == PLAYING {
                let status = self.handles[self.ending].map_or(DONE, |e| self.streams.status(e));
                self.close(self.ending);
                ok = status == DONE;
                std::mem::swap(&mut self.ending, &mut self.preparing);
            } else {
                self.close(self.preparing);
            }
        }
        let Some(t) = self.transition.clone() else { return false };
        let p = self.preparing;
        self.handles[p] = self.open(t.t.to);
        let Some(h) = self.handles[p] else {
            self.transition = None;
            self.state = 1;
            return false;
        };
        self.samples[p] = Some(t.t.to);
        self.layer_volumes[p] = self.layer_volume(p);
        self.fades[p] = if t.t.time == 0 { 1.0 } else { 0.0 };
        let (v, pv) = (self.volume, self.pause_volume);
        self.set_volumes(v, pv, true);
        self.streams.set_ms_position(h, t.t.in_time.max(0));
        self.streams.start(h);
        self.streams.pause(h, true);
        self.state = 3;
        ok
    }

    /// FUN_1000963a: the transition's fades made tables for its time.
    fn begin_fade(&mut self) {
        let Some(t) = &self.transition else { return };
        self.fade_length = t.t.time as u32;
        self.fade_in = t.t.fade_in.table_at(self.fade_length, INTERVAL);
        self.fade_out = t.t.fade_out.table_at(self.fade_length, INTERVAL);
        self.fade_start = t.t.out_time as u32;
    }

    /// FUN_1000a2c8 (state 2): prepared 200 ms before the fade out; its fade begun.
    fn prepare_step(&mut self, position: u32) {
        let Some(t) = &self.transition else { return };
        if position < (t.t.out_time as u32).wrapping_sub(200) {
            return;
        }
        if self.prepare() {
            self.state = 3;
            self.begin_fade();
        }
    }

    /// FUN_100096b6 (state 3): the prepared stream started at the fade out; a layer transition's pause track taken on.
    fn start_prepared(&mut self, position: u32) {
        let Some(h) = self.handles[self.preparing] else {
            self.state = 1;
            return;
        };
        if position >= self.fade_start {
            self.streams.start(h);
            self.state = 4;
            if let Some(t) = &self.transition {
                let (from, to) = (self.project.samples[t.from].layer, self.project.samples[t.t.to].layer);
                if from != to {
                    let track = self.project.layer(to).and_then(|l| l.pause.clone());
                    self.pause_set_track(track);
                    self.pause_start();
                }
            }
        }
    }

    /// FUN_1000972d (state 4): the fades' levels at the position; at the end the prepared stream the one playing.
    fn fade_step(&mut self, position: i32) {
        let mut x = 0u32;
        if position > self.fade_start as i32 {
            x = (position as u32).wrapping_sub(self.fade_start).min(self.fade_length);
            if self.fade_length != 0 && (x as i32) >= 0 {
                self.fades = [0.0; 3];
                self.fades[self.preparing] = fade_value(&self.fade_in, x);
                self.fades[self.playing] = fade_value(&self.fade_out, x);
                let (v, p) = (self.volume, self.pause_volume);
                self.set_volumes(v, p, true);
            }
        }
        if x == self.fade_length {
            std::mem::swap(&mut self.playing, &mut self.preparing);
            self.transition = None;
            self.state = 1;
        }
    }

    /// FUN_10009802 (state 5): playing again from the start when the stream comes round.
    fn waiting(&mut self, position: i32) {
        if position < self.last_position {
            if self.state == 5 {
                self.timer = None;
            }
            self.state = 1;
        }
        self.last_position = position;
    }

    /// FUN_1000a552 (state 0): when asked to, an entry sample of the event's layer played with its pause track.
    fn restart(&mut self) {
        if !self.restart {
            return;
        }
        let id = self.manager.layer;
        let track = self.project.layer(id).and_then(|l| l.pause.clone());
        self.pause_set_track(track);
        if let Some(e) = self.entry_sample(id, false) {
            self.play(Some(e), true);
        }
    }

    /// `Update()`: the timer, the state's step at the stream's position and the pause tracks' tick.
    pub fn update(&mut self) {
        if self.timer.is_some_and(|t| Instant::now() >= t) {
            self.timer_fired();
        }
        let (total, current) = self.position();
        match self.state {
            0 => self.restart(),
            1 => self.choose_next(current, total),
            2 => self.prepare_step(current as u32),
            3 => self.start_prepared(current as u32),
            4 => self.fade_step(current),
            5 => self.waiting(current),
            _ => {}
        }
        self.pause_tick();
    }

    /// The layer the manager is after (`SampleManager_t` +0x14).
    pub fn layer(&self) -> i32 {
        self.manager.layer
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The sample playing, if any (for the tests and the `sound` command).
    pub fn playing_sample(&self) -> Option<usize> {
        self.handles[self.playing].and(self.samples[self.playing]).filter(|_| self.state != 0)
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        for i in 0..3 {
            self.close(i);
        }
    }
}

/// The player on a thread of its own stepping it every 10 ms (FUN_1000a7b5), shared with the sound system.
pub struct Running {
    pub player: Arc<Mutex<Player>>,
    quit: Arc<std::sync::atomic::AtomicBool>,
}

impl Running {
    pub fn start(player: Player) -> Running {
        let player = Arc::new(Mutex::new(player));
        let quit = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (p, q) = (player.clone(), quit.clone());
        std::thread::spawn(move || {
            while !q.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(mut p) = p.lock() {
                    p.update();
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        Running { player, quit }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut Player) -> R) -> R {
        f(&mut self.player.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.quit.store(true, std::sync::atomic::Ordering::Relaxed);
        self.with(|p| p.stop(true, true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mao_formats::music::{Layer, Sample};

    /// A project of two layers of two samples each, the files 1 s of silence written to a folder of the test's.
    fn setup() -> (Player, Streams, tempfile::Dir) {
        let dir = tempfile::Dir::new();
        let sample = |name: &str, layer: i32, transitions: Vec<Transition>| Sample { name: name.into(), layer, offset: 0, end_time: 1000, fade_in: Fade::default(), fade_out: Fade::default(), entry: true, volume: 1.0, transitions, layer_transitions: vec![] };
        let t = |to: usize, out_time: i32| Transition { to, priority: 0, fade_out: Fade::default(), fade_in: Fade::default(), out_time, in_position: 0, in_time: 0, time: 0 };
        let project = Project {
            layers: vec![Layer { id: 0, name: "a".into(), volume: 1.0, pause: None }, Layer { id: 1, name: "b".into(), volume: 0.5, pause: None }],
            samples: vec![sample("a1", 0, vec![t(1, 1000), t(2, 1000)]), sample("a2", 0, vec![t(0, 1000)]), sample("b1", 1, vec![t(3, 1000)]), sample("b2", 1, vec![t(2, 1000)])],
        };
        for (layer, name) in [("a", "a1"), ("a", "a2"), ("b", "b1"), ("b", "b2")] {
            std::fs::create_dir_all(dir.0.join(layer)).unwrap();
            std::fs::write(dir.0.join(layer).join(format!("{name}.wav")), tempfile::silence(22050, 22050)).unwrap();
        }
        let streams = crate::mixer::Engine::new(false).streams();
        (Player::new(Arc::new(project), &dir.0, streams.clone()), streams, dir)
    }

    mod tempfile {
        pub struct Dir(pub std::path::PathBuf);
        impl Dir {
            pub fn new() -> Dir {
                let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
                let d = std::env::temp_dir().join(format!("mao-music-{n}"));
                std::fs::create_dir_all(&d).unwrap();
                Dir(d)
            }
        }
        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        /// A mono 16-bit wave of `frames` zeros.
        pub fn silence(rate: u32, frames: u32) -> Vec<u8> {
            let mut d = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
            d.extend(16u32.to_le_bytes());
            for v in [1u16, 1] {
                d.extend(v.to_le_bytes());
            }
            d.extend(rate.to_le_bytes());
            d.extend((rate * 2).to_le_bytes());
            d.extend(2u16.to_le_bytes());
            d.extend(16u16.to_le_bytes());
            d.extend(b"data");
            d.extend((frames * 2).to_le_bytes());
            d.extend(vec![0u8; frames as usize * 2]);
            d
        }
    }

    /// Runs the player and the mixer on together, `ms` at a time.
    fn run(p: &mut Player, streams: &Streams, ms: u32, steps: u32) {
        for _ in 0..steps {
            streams.advance((crate::mixer::RATE * ms / 1000) as usize);
            p.update();
        }
    }

    #[test]
    fn a_layer_plays_on_and_an_event_moves_it_to_another() {
        let (mut p, streams, _dir) = setup();
        p.signal_event(0, 1, 2000);
        p.play(None, true);
        assert!(matches!(p.playing_sample(), Some(0 | 1)));
        // (samples of layer a follow each other at their ends)
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..40 {
            run(&mut p, &streams, 100, 1);
            seen.extend(p.playing_sample());
        }
        assert!(seen.iter().all(|&s| s < 2), "{seen:?}");
        // (an event for layer b: a1's transition to b1, else a made one to an entry sample of b)
        p.signal_event(1, 1, 2000);
        for _ in 0..40 {
            run(&mut p, &streams, 100, 1);
        }
        assert!(matches!(p.playing_sample(), Some(2 | 3)), "{:?}", p.playing_sample());
        assert_eq!(p.layer(), 1);
    }
}
