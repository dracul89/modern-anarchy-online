//! The sound system: SandyInterface.dll's `SandyInterface_t` and `SandyInterfaceModule_t` (the `sandyinterface` drop-in's
//! `sandy.rs`, `source.rs` and `module.rs`) over the engine of [`crate::mixer`].
//!
//! Sources come from the source files (`SM_Sandy_Gui.sbf` when the interface starts, `SM_Sandy_Game_Dummy.sbf` when a
//! playfield is entered: `ActivateGameZone`), kept by ID. Playing one (`PlaySample`) works out its volume (by distance
//! between its minimum and maximum, its random range) and passes the play on to its variant (picked by the surface's sound
//! material) or its children (all, one at random, in turn, or on timers); the loudest play of each source in a frame goes
//! into the play list `Frameprocess` carries out at its end (the sample started looping or once, or for a playing doppler
//! source its pitch moved). Samples are read when first played, within a memory budget that frees the least recently used.
//!
//! The music is SIMPlayer's player ([`crate::music`]) over `sound/music/env/anarchy.sws`, made when music first plays
//! (`LoadSimProject`): a layer is played by an event to the player (`PlayMusic`), the district's layer each second
//! (`SetupENVSounds`), the battle layers while the combat music's state says so (`ProcessCombatMusic` over the frame's
//! `CombatUpdate` reports).
//!
//! Not here: the alert and noise radii told to AFCM (module 9's messages 0xbf and 0x119, for the game's AI).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mao_formats::sound::{Record, parse_bank, sound_id};

use crate::mixer::{Engine, SoundKey};

/// A source (0x288 bytes in the original), see `source.rs`.
#[derive(Clone, Debug)]
pub struct Source {
    pub id: u32,
    pub all: bool,
    pub random: bool,
    /// Listed in the queue this frame.
    pub queued: bool,
    /// The child played in turn.
    pub current: i32,
    pub sequential: bool,
    pub interval_min: f32,
    pub interval_max: f32,
    pub timed: bool,
    pub timer: f32,
    pub children: Vec<usize>,
    pub u34: bool,
    pub random_volume: bool,
    pub volume_min: f32,
    pub volume_max: f32,
    pub fade_in: f32,
    pub loop_min: f32,
    pub loop_max: f32,
    pub pass_delay: bool,
    pub rate: i32,
    pub has_file: bool,
    pub file: String,
    pub looping: bool,
    pub playing: bool,
    pub volume: f32,
    pub fade: f32,
    pub changed: bool,
    pub prefetched: bool,
    pub length: f32,
    pub doppler: bool,
    pub u16d: bool,
    pub position: [f32; 3],
    pub remaining: f32,
    pub doppler_filter: f32,
    pub velocity: [f32; 3],
    pub release: f32,
    pub priority: u8,
    pub played: f32,
    pub chance: u8,
    pub chance_second: i32,
    pub alert_radius: f32,
    pub sound: Option<SoundKey>,
    pub variants: [Option<usize>; 46],
    pub has_variants: bool,
    pub min_distance: f32,
    pub max_distance: f32,
    pub group: u8,
    pub stop_group: u8,
    pub stop_others: bool,
    pub file_size: u32,
    pub last_used: i32,
    pub noise_radius: f32,
    pub player_fx: bool,
}

impl Default for Source {
    /// FUN_10001063: every setting at its default (the fade 1, the priority 1, a 100% chance, a maximum distance of 30,
    /// passing the delay on).
    fn default() -> Source {
        Source {
            id: 0,
            all: false,
            random: false,
            queued: false,
            current: 0,
            sequential: false,
            interval_min: 0.0,
            interval_max: 0.0,
            timed: false,
            timer: 0.0,
            children: Vec::new(),
            u34: false,
            random_volume: false,
            volume_min: 0.0,
            volume_max: 0.0,
            fade_in: 0.0,
            loop_min: 0.0,
            loop_max: 0.0,
            pass_delay: true,
            rate: 0,
            has_file: false,
            file: String::new(),
            looping: false,
            playing: false,
            volume: 0.0,
            fade: 1.0,
            changed: false,
            prefetched: false,
            length: 0.0,
            doppler: false,
            u16d: false,
            position: [0.0; 3],
            remaining: 0.0,
            doppler_filter: 0.0,
            velocity: [0.0; 3],
            release: 0.0,
            priority: 1,
            played: 0.0,
            chance: 100,
            chance_second: 0,
            alert_radius: 0.0,
            sound: None,
            variants: [None; 46],
            has_variants: false,
            min_distance: 0.0,
            max_distance: 30.0,
            group: 0,
            stop_group: 0,
            stop_others: false,
            file_size: 0,
            last_used: 0,
            noise_radius: 0.0,
            player_fx: false,
        }
    }
}

/// A play put off (`PlaySample`'s `defer`).
#[derive(Clone, Debug)]
struct Deferred {
    source: usize,
    volume: f32,
    position: [f32; 3],
    delay: f32,
    index: i32,
    defer: f32,
}

/// The frame's play of a source.
#[derive(Clone, Copy, Debug)]
struct Play {
    volume: f32,
    delay: f32,
    pitch: i32,
    velocity: [f32; 3],
}

/// The CRT's `rand()` (msvcr100: the state times 214013 plus 2531011, bits 16 to 30), seeded 1.
#[derive(Clone, Debug)]
pub struct Rand(u32);

impl Default for Rand {
    fn default() -> Rand {
        Rand(1)
    }
}

impl Rand {
    pub fn next(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        ((self.0 >> 16) & 0x7fff) as i32
    }
}

fn sub(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [(a[0] as f64 - b[0] as f64) as f32, (a[1] as f64 - b[1] as f64) as f32, (a[2] as f64 - b[2] as f64) as f32]
}

fn length(v: &[f32; 3]) -> f32 {
    let [x, y, z] = v.map(|c| c as f64);
    ((x * x + y * y + z * z) as f32 as f64).sqrt() as f32
}

fn normalized(v: &[f32; 3]) -> [f32; 3] {
    let inverse = (1.0 / length(v) as f64) as f32;
    v.map(|c| (c as f64 * inverse as f64) as f32)
}

fn is_zero(v: &[f32; 3]) -> bool {
    v[0] == 0.0 && v[1] == 0.0 && v[2] == 0.0
}

/// The x87's truncation as the SSE2 `_ftol2` does it: out of range and NaN are `i32::MIN`.
fn ftol(x: f64) -> i32 {
    let t = x.trunc();
    if t.is_nan() || !(-2147483648.0..=2147483647.0).contains(&t) { i32::MIN } else { t as i32 }
}

/// `SandyInterface_t`.
pub struct Sandy {
    pub initialized: bool,
    /// The sound folder (`<CD path>/sound`).
    root: PathBuf,
    pub sources: Vec<Source>,
    by_id: HashMap<u32, usize>,
    /// The sources marked to be listed, in order (+8).
    all: Vec<usize>,
    /// The sources queued in a frame (+0xc).
    queued: Vec<usize>,
    /// The sources playing on (+0x3c).
    active: Vec<usize>,
    deferred: Vec<Deferred>,
    pub listener: [f32; 3],
    memory: u32,
    budget: u32,
    frame: i32,
    protected: Option<usize>,
    pub engine: Engine,
    mute_all_but_next: bool,
    next_allowed: bool,
    game_pending: bool,
    gui_pending: bool,
    game_done: bool,
    gui_done: bool,
    second: f32,
    seconds: i32,
    /// The frame's plays by source. (The original's map orders them by the sources' addresses; here by their order made.)
    plays: BTreeMap<usize, Play>,
    pub rand: Rand,
    /// The statics: `m_vMasterFXVolume`, `m_vMasterPlayerFXVolume`, `m_vMasterVolume`.
    pub master_fx: f32,
    pub master_player_fx: f32,
    pub master: f32,
    /// `m_vMasterMusicVolume` and `m_bMasterMusicVolumeUpdated`.
    pub master_music: f32,
    music_updated: bool,
    /// `m_nCombatMusicPreferences` (the battle music option: 0 none).
    pub combat_preferences: i32,
    /// The music player (the module's, DAT_1000f0b0) and the land control layer (DAT_1000f0b4).
    pub player: Option<crate::music::Running>,
    landcontrol: i32,
    /// The district (+0x5c), the music layer (+0x74), the combat music's state (+0x90), its override (+0x64, +0x108), the
    /// battle layers (+0xc4), music muted and transitions forced (+0x10c, +0x10d).
    pub district: i32,
    pub layer: i32,
    pub music_state: i32,
    override_name: Option<String>,
    override_layer: i32,
    layers: [i32; 17],
    music_mute: bool,
    force_transitions: bool,
    combat: Combat,
}

/// `CombatUpdate`'s reports and what `ProcessCombatMusic` kept of them (+0x78 to +0xc0).
#[derive(Clone, Debug, Default)]
struct Combat {
    enemies: i32,
    faction: i32,
    level: i32,
    health: i32,
    flag: bool,
    target_enemies: i32,
    target_level: i32,
    target: i32,
    target_health: i32,
    target_flag: bool,
    last_target: i32,
    updates: i32,
    last_enemies: i32,
    last_target_enemies: i32,
    last_target_level: i32,
    last_target_health: i32,
    timer: f32,
}

/// The battle layers' names (+0xc8 on).
const BATTLE: [&str; 16] = [
    "battle\\NeutralRebirth",
    "battle\\ClanRebirth",
    "battle\\OmniRebirth",
    "battle\\NeutralDeath",
    "battle\\ClanDeath",
    "battle\\OmniDeath",
    "battle\\NeutralVictory",
    "battle\\NeutralVictory",
    "battle\\NeutralVictory",
    "battle\\Neutral",
    "battle\\WinningMedium",
    "battle\\WinningSlightlyBig",
    "battle\\WinningGreatlyBig",
    "battle\\LosingMedium",
    "battle\\LosingSlightlyBig",
    "battle\\LosingBadlyBig",
];

impl Sandy {
    /// `SandyInterface_t()` (its engine `SoundEngineInterface_t::GetInstance()`), over the sound folder.
    pub fn new(root: &Path, engine: Engine) -> Sandy {
        Sandy {
            initialized: false,
            root: root.to_path_buf(),
            sources: Vec::new(),
            by_id: HashMap::new(),
            all: Vec::new(),
            queued: Vec::new(),
            active: Vec::new(),
            deferred: Vec::new(),
            listener: [0.0; 3],
            memory: 0,
            budget: 0xf00000,
            frame: 0,
            protected: None,
            engine,
            mute_all_but_next: false,
            next_allowed: false,
            game_pending: false,
            gui_pending: false,
            game_done: false,
            gui_done: false,
            second: 0.0,
            seconds: 0,
            plays: BTreeMap::new(),
            rand: Rand::default(),
            master_fx: 1.0,
            master_player_fx: 1.0,
            master: 1.0,
            master_music: 1.0,
            music_updated: true,
            combat_preferences: 0,
            player: None,
            landcontrol: 0,
            district: -1,
            layer: -1,
            music_state: 0,
            override_name: None,
            override_layer: 0,
            layers: [0; 17],
            music_mute: false,
            force_transitions: true,
            combat: Combat { enemies: 100, last_enemies: 100, ..Combat::default() },
        }
    }

    /// `InitializeSandy()` (and `InitialiseSandyNow()`): the engine started, the pending source files read.
    pub fn initialize(&mut self) -> bool {
        if self.engine.allocated {
            self.initialized = true;
            let master = self.master;
            self.engine.set_master_volume(master);
        }
        self.frame = 0;
        if self.game_pending {
            self.setup_game_sounds();
        }
        if self.gui_pending {
            self.setup_gui_sounds();
        }
        self.initialized
    }

    /// `DeinitializeSandy()`: the samples released, the engine shut.
    pub fn deinitialize(&mut self) {
        if self.initialized {
            self.stop_music();
            self.release_all();
        }
        self.initialized = false;
    }

    /// `GetSoundPointer(id)`.
    pub fn source_of(&self, id: u32) -> Option<usize> {
        self.by_id.get(&id).copied()
    }

    /// The source with this ID, made if there is none; whether it was made.
    fn obtain(&mut self, id: u32) -> (usize, bool) {
        if let Some(&s) = self.by_id.get(&id) {
            return (s, false);
        }
        self.sources.push(Source::default());
        let s = self.sources.len() - 1;
        self.by_id.insert(id, s);
        (s, true)
    }

    /// `ParseSourceFile(path)`: the records read into sources (FUN_100111b0 for each), the changed ones that had their
    /// sample read again and played again if they were playing (from a fade of 0.1). False when it can't be read.
    pub fn parse_source_file(&mut self, path: &Path) -> bool {
        let Ok(data) = std::fs::read(path) else { return false };
        let records = parse_bank(&data).unwrap_or_default();
        for r in &records {
            let variants: [Option<usize>; 46] = std::array::from_fn(|j| (r.variants[j] != 0).then(|| self.obtain(r.variants[j]).0));
            let children: Vec<usize> = r.children.iter().map(|&c| self.obtain(c).0).collect();
            let (s, new) = self.obtain(r.id);
            if new && r.listed {
                self.all.push(s);
            }
            update(&mut self.sources[s], r, &variants, children.len(), &mut self.rand);
            self.sources[s].children.extend(children);
        }
        for k in 0..self.all.len() {
            let s = self.all[k];
            if !self.sources[s].changed {
                continue;
            }
            self.sources[s].changed = false;
            if self.sources[s].prefetched {
                self.sources[s].prefetched = false;
                self.prefetch(s, false);
                if self.sources[s].playing {
                    self.sources[s].playing = false;
                    if !self.sources[s].u34 {
                        self.sources[s].fade = f32::from_bits(0x3dcccccd);
                        let (v, p, r) = (self.sources[s].volume, self.sources[s].position, self.sources[s].remaining);
                        self.play_sample(Some(s), v, p, r, 0, 0.0, 100, 1, 0, [0.0; 3]);
                    }
                }
            }
        }
        true
    }

    /// A source file under the sound folder read once (put off until initialised).
    fn setup(&mut self, name: &str, gui: bool) {
        let done = if gui { self.gui_done } else { self.game_done };
        if done {
            return;
        }
        if !self.initialized {
            if gui {
                self.gui_pending = true;
            } else {
                self.game_pending = true;
            }
            return;
        }
        let path = self.root.join("SourceFiles").join(name);
        self.parse_source_file(&path);
        if gui {
            self.gui_done = true;
            self.gui_pending = false;
        } else {
            // (`SetupGameSounds` leaves its pending flag set)
            self.game_done = true;
        }
    }

    /// `SetupGUISounds()`.
    pub fn setup_gui_sounds(&mut self) {
        self.setup("SM_Sandy_Gui.sbf", true);
    }

    /// `SetupGameSounds()`.
    pub fn setup_game_sounds(&mut self) {
        self.setup("SM_Sandy_Game_Dummy.sbf", false);
    }

    /// `PrefetchSample(source, keep)`: its file sized and read into a sound, the budget kept by freeing others first (all
    /// but this one, unless `keep`); prefetched even without a file. Nothing when the file is missing.
    pub fn prefetch(&mut self, s: usize, keep: bool) {
        if !self.initialized || self.sources[s].prefetched {
            return;
        }
        if self.sources[s].has_file {
            let path = self.root.join(&self.sources[s].file);
            let Ok(meta) = std::fs::metadata(&path) else { return };
            let size = meta.len() as u32;
            self.sources[s].file_size = size;
            self.memory = self.memory.wrapping_add(size);
            if self.budget < self.memory {
                if !keep {
                    self.protected = Some(s);
                }
                self.force_free(self.memory.wrapping_sub(self.budget));
                if !keep {
                    self.protected = None;
                }
            }
            let priority = self.sources[s].priority;
            self.sources[s].sound = self.engine.create_sound(&path, priority);
        }
        self.sources[s].prefetched = true;
        self.sources[s].last_used = self.frame;
    }

    /// `ForceFreeMemory(size)`: prefetched sources not playing (but the one being read; at most 512), least recently used
    /// first, released until more than `size` is freed.
    fn force_free(&mut self, size: u32) {
        let mut found: Vec<(usize, u32)> =
            self.all.iter().filter(|&&s| self.protected != Some(s) && self.sources[s].prefetched && !self.sources[s].playing).take(0x200).map(|&s| (s, self.sources[s].last_used as u32)).collect();
        // (the original's bubble sort: stable)
        found.sort_by_key(|f| f.1);
        let mut freed = 0u32;
        let mut before = self.memory;
        for (s, _) in found {
            self.stop_sample(Some(s));
            self.release_sample(Some(s), -1.0);
            freed = freed.wrapping_add(before.wrapping_sub(self.memory));
            if size < freed {
                return;
            }
            before = self.memory;
        }
    }

    /// `SetSourceVolume(source, volume)`: clamped, times the master effect volume.
    pub fn set_source_volume(&mut self, s: Option<usize>, v: f32) {
        let Some(s) = s else { return };
        let v = v.clamp(0.0, 1.0);
        if self.initialized && self.engine.is_sound_allocated(self.sources[s].sound) {
            let volume = (self.master_fx as f64 * v as f64) as f32;
            self.engine.set_volume(self.sources[s].sound.unwrap(), volume);
        }
    }

    /// `SetMasterFXVolume(volume)`: the active sources' volumes set again.
    pub fn set_master_fx(&mut self, v: f32) {
        if v == self.master_fx {
            return;
        }
        self.master_fx = v;
        for k in 0..self.active.len() {
            let s = self.active[k];
            if self.engine.is_playing(self.sources[s].sound) || self.sources[s].playing {
                let volume = self.sources[s].volume;
                self.set_source_volume(Some(s), volume);
            }
        }
    }

    /// `StopSample(source)`: its sound stopped, its first active entry dropped when playing, its variants and children
    /// stopped.
    pub fn stop_sample(&mut self, s: Option<usize>) {
        let Some(s) = s else { return };
        if !self.initialized {
            return;
        }
        if let Some(sound) = self.sources[s].sound {
            self.engine.stop(sound, true);
        }
        if self.sources[s].playing {
            if let Some(i) = self.active.iter().position(|&a| a == s) {
                self.active.remove(i);
            }
        }
        if self.sources[s].has_variants {
            for v in self.sources[s].variants.into_iter().flatten() {
                self.stop_sample(Some(v));
            }
        }
        for k in 0..self.sources[s].children.len() {
            let c = self.sources[s].children[k];
            self.stop_sample(Some(c));
        }
        self.sources[s].playing = false;
    }

    /// `ReleaseSample(source, time)`: stopped, its sound deleted (the memory given back); its children and variants released.
    pub fn release_sample(&mut self, s: Option<usize>, time: f32) {
        let Some(s) = s else { return };
        if !self.initialized || !self.sources[s].prefetched {
            return;
        }
        self.stop_sample(Some(s));
        if self.engine.is_sound_allocated(self.sources[s].sound) {
            self.engine.delete_sound(self.sources[s].sound.take().unwrap());
            self.memory = self.memory.wrapping_sub(self.sources[s].file_size);
            self.sources[s].file_size = 0;
        }
        self.sources[s].playing = false;
        self.sources[s].prefetched = false;
        for k in 0..self.sources[s].children.len() {
            let c = self.sources[s].children[k];
            self.release_sample(Some(c), time);
        }
        if self.sources[s].has_variants {
            for v in self.sources[s].variants.into_iter().flatten() {
                self.release_sample(Some(v), time);
            }
        }
    }

    /// `ReleaseAllSamples()`.
    pub fn release_all(&mut self) {
        for k in 0..self.all.len() {
            let s = self.all[k];
            self.stop_sample(Some(s));
            self.release_sample(Some(s), -1.0);
        }
    }

    /// `StopAllSoundEffects()`.
    pub fn stop_all(&mut self) {
        for k in 0..self.all.len() {
            let s = self.all[k];
            self.stop_sample(Some(s));
        }
    }

    /// `PlaySample(source, volume, position, delay, variant, defer, pitch, mode, range, velocity)`.
    #[allow(clippy::too_many_arguments)]
    pub fn play_sample(&mut self, s: Option<usize>, mut volume: f32, pos: [f32; 3], delay: f32, mut index: i32, defer: f32, pitch: i32, mode: i32, range: u32, velocity: [f32; 3]) {
        if !self.initialized {
            return;
        }
        if self.mute_all_but_next {
            if !self.next_allowed {
                return;
            }
            self.next_allowed = false;
        }
        let Some(s) = s else { return };
        self.sources[s].last_used = self.frame;
        self.sources[s].velocity = velocity;
        if mode != 1 && index != 0 {
            let e = match mode {
                0 => index + 0x1e,
                2 => index + 0xf,
                _ => index,
            };
            // (the original reads past the array for a large index: the fields after it; here nothing)
            if usize::try_from(e).ok().and_then(|e| self.sources[s].variants.get(e).copied().flatten()).is_some() {
                index = e;
            }
        }
        if defer != 0.0 {
            self.deferred.push(Deferred { source: s, volume, position: pos, delay, index, defer });
            return;
        }
        if self.sources[s].chance < 100 {
            if self.sources[s].chance_second == self.seconds {
                return;
            }
            self.sources[s].chance_second = self.seconds;
            if self.rand.next() % 200 >= self.sources[s].chance as i32 {
                return;
            }
        }
        if self.sources[s].stop_others && self.sources[s].stop_group != 0 {
            let group = self.sources[s].stop_group;
            for k in 0..self.all.len() {
                let other = self.all[k];
                if self.sources[other].group == group {
                    self.stop_sample(Some(other));
                }
            }
        }
        let range = range as u16;
        let (max_d, min_d) = (self.sources[s].max_distance, self.sources[s].min_distance);
        let (maximum, minimum) = if range == 0 {
            (max_d, min_d)
        } else {
            let r = range as f32;
            let min = if max_d > min_d {
                if 0.0 < max_d { ((min_d as f64 / max_d as f64) * r as f64) as f32 } else { 0.0 }
            } else {
                r
            };
            (r, min)
        };
        if !is_zero(&pos) {
            let distance = length(&sub(&self.listener, &pos));
            if maximum <= distance {
                return;
            }
            if minimum < distance {
                let a = maximum as f64 - minimum as f64;
                let b = distance as f64 - minimum as f64;
                volume = (((a - b) / a) * volume as f64) as f32;
            }
            self.sources[s].position = pos;
        }
        if volume <= 0.0 && self.sources[s].volume <= 0.0 {
            self.stop_sample(Some(s));
            return;
        }
        volume = volume.clamp(0.0, 1.0);
        let mut activate = false;
        {
            let length = self.sources[s].length;
            if delay != 0.0 {
                self.sources[s].remaining = (length as f64 + delay as f64) as f32;
                activate = true;
            } else {
                let (lmin, lmax) = (self.sources[s].loop_min, self.sources[s].loop_max);
                self.sources[s].remaining = if lmin == 0.0 {
                    0.0
                } else if lmax == lmin {
                    (length as f64 + lmin as f64) as f32
                } else {
                    let r = self.rand.next() % 1000;
                    ((r as f64 * (lmin as f64 - lmax as f64)) / 1000.0 + lmax as f64 + length as f64) as f32
                };
                if 0.0 < self.sources[s].remaining {
                    activate = true;
                }
            }
        }
        if activate {
            if !self.sources[s].playing && !self.active.contains(&s) {
                self.active.push(s);
            }
            self.sources[s].playing = true;
        }
        let (vmin, vmax) = (self.sources[s].volume_min, self.sources[s].volume_max);
        volume = if !self.sources[s].random_volume {
            (vmax as f64 * volume as f64) as f32
        } else {
            let r = self.rand.next() % 256;
            (((r as f64 * (vmax as f64 - vmin as f64)) / 255.0 + vmin as f64) * volume as f64) as f32
        };
        self.sources[s].volume = volume;
        let passed = if self.sources[s].pass_delay { self.sources[s].remaining } else { 0.0 };
        if self.sources[s].has_variants && (index as u32).wrapping_sub(1) <= 0x2c {
            let v = self.sources[s].variants[index as usize].or(self.sources[s].variants[0]);
            if v.is_some() {
                self.play_sample(v, volume, [0.0; 3], passed, 0, 0.0, pitch, 1, 0, [0.0; 3]);
            }
        }
        if !self.sources[s].timed && !self.sources[s].children.is_empty() {
            let child_delay = if self.sources[s].pass_delay { delay } else { 0.0 };
            let count = self.sources[s].children.len() as i32;
            if self.sources[s].sequential {
                let child = self.sources[s].children[self.sources[s].current as usize];
                self.play_sample(Some(child), volume, [0.0; 3], child_delay, index, 0.0, pitch, 1, 0, [0.0; 3]);
                if !self.sources[s].queued {
                    self.queued.push(s);
                }
                self.sources[s].queued = true;
            } else if self.sources[s].random {
                if !self.sources[s].queued {
                    self.queued.push(s);
                    self.sources[s].current += 1;
                    self.sources[s].queued = true;
                }
                if self.sources[s].current >= count {
                    self.sources[s].current = 0;
                }
                let child = self.sources[s].children[self.sources[s].current as usize];
                self.play_sample(Some(child), volume, [0.0; 3], child_delay, index, 0.0, pitch, 1, 0, [0.0; 3]);
            } else if self.sources[s].all {
                for k in 0..self.sources[s].children.len() {
                    let child = self.sources[s].children[k];
                    self.play_sample(Some(child), volume, [0.0; 3], child_delay, index, 0.0, pitch, 1, 0, [0.0; 3]);
                }
            }
        }
        if self.plays.get(&s).is_some_and(|p| !(p.volume < volume)) {
            return;
        }
        let velocity = self.sources[s].velocity;
        self.plays.insert(s, Play { volume, delay, pitch, velocity });
    }

    /// `ExecuteCorePlayList(time)`: each source of the frame's play list started (looping when it loops or has a delay,
    /// fading in from 0.01 with a fade-in) or, playing with doppler on, its pitch moved; its volume set (the master effect or
    /// player effect volume times the fade), or stopped when not positive.
    fn execute_play_list(&mut self) {
        let plays = std::mem::take(&mut self.plays);
        for (s, p) in plays {
            if self.initialized && self.sources[s].sound.is_none() {
                self.prefetch(s, false);
            }
            let looping = self.sources[s].looping;
            if !self.sources[s].playing && looping && !self.active.contains(&s) {
                self.active.push(s);
            }
            if let Some(sound) = self.sources[s].sound.filter(|&k| self.engine.is_sound_allocated(Some(k))) {
                if 0.0 < p.volume {
                    if !self.sources[s].playing || !self.engine.is_playing(Some(sound)) {
                        let priority = self.sources[s].priority as i32;
                        if looping || 0.0 < p.delay {
                            self.engine.play_looping(sound, priority, p.pitch);
                        } else {
                            self.engine.play(sound, priority, p.pitch);
                        }
                        self.sources[s].fade = if 0.0 < self.sources[s].fade_in { f32::from_bits(0x3c23d70a) } else { 1.0 };
                    } else if self.sources[s].doppler {
                        self.doppler(s, &p.velocity);
                    }
                }
                if 1.0 < p.volume {
                    self.engine.set_volume(sound, self.master_fx);
                } else if 0.0 < p.volume || p.volume.is_nan() {
                    let master = if self.sources[s].player_fx { self.master_player_fx } else { self.master_fx };
                    self.engine.set_volume(sound, ((p.volume as f64 * master as f64) * self.sources[s].fade as f64) as f32);
                } else {
                    self.engine.stop(sound, true);
                }
            }
            if looping {
                self.sources[s].playing = true;
            }
        }
    }

    /// A playing doppler source's pitch (see the drop-in's `doppler`).
    fn doppler(&mut self, s: usize, velocity: &[f32; 3]) {
        const EPSILON: f64 = 9.999999747378752e-05;
        let moving = |v: &[f32; 3]| v.iter().any(|&c| c.abs() as f64 > EPSILON);
        if !moving(velocity) {
            return;
        }
        let direction = normalized(velocity);
        let to_listener = sub(&self.listener, &self.sources[s].position);
        let dot = if moving(&to_listener) {
            let d = normalized(&to_listener);
            ((direction[1] as f64 * d[1] as f64 + direction[0] as f64 * d[0] as f64) + direction[2] as f64 * d[2] as f64) as f32
        } else {
            -1.0
        };
        let speed = length(velocity);
        let x = (((f32::from_bits(0x3cf5c28f) as f64 * speed as f64) * dot as f64) as f32).clamp(-1.0, 1.0);
        let c = (x as f64 * f32::from_bits(0x3e99999a) as f64) as f32;
        let filtered = (self.sources[s].doppler_filter as f64 * f32::from_bits(0x3f666666) as f64 + c as f64 * f32::from_bits(0x3dcccccd) as f64) as f32;
        self.sources[s].doppler_filter = filtered;
        let shift = ftol(filtered as f64 * -100.0);
        let factor = speed.clamp(f32::from_bits(0x3f4ccccd), 1.0);
        let percent = ftol(100i32.wrapping_sub(shift) as f64 * factor as f64);
        if let Some(sound) = self.sources[s].sound {
            self.engine.set_pitch(sound, percent);
        }
    }

    /// `FrameProcessSound(source, time)`: an active source's frame (its play time, release timer, loop or delay run down,
    /// fading in, its volume over the last stretch; its children's timers), then its children's.
    fn frame_process_sound(&mut self, s: usize, time: f32) {
        if self.sources[s].playing {
            self.sources[s].last_used = self.frame;
        }
        if self.engine.is_playing(self.sources[s].sound) {
            self.sources[s].played = (self.sources[s].played as f64 + time as f64) as f32;
            self.sources[s].last_used = self.frame;
        } else {
            self.sources[s].played = 0.0;
        }
        if 0.0 < self.sources[s].release {
            let r = (self.sources[s].release as f64 - time as f64) as f32;
            self.sources[s].release = r;
            if r <= 0.0 {
                self.sources[s].release = 0.0;
                self.release_sample(Some(s), 0.0);
            }
        }
        'volume: {
            if !(0.0 < self.sources[s].remaining) {
                break 'volume;
            }
            if !self.sources[s].playing {
                self.sources[s].remaining = 0.0;
                break 'volume;
            }
            let r = (self.sources[s].remaining as f64 - time as f64) as f32;
            self.sources[s].remaining = r;
            let (fade_in, length, volume) = (self.sources[s].fade_in, self.sources[s].length, self.sources[s].volume);
            if 0.0 < fade_in && self.sources[s].fade < 1.0 {
                let f = (time as f64 / fade_in as f64 + self.sources[s].fade as f64) as f32;
                self.sources[s].fade = f.min(1.0);
            }
            if r <= 0.0 {
                self.sources[s].remaining = 0.0;
                self.stop_sample(Some(s));
                break 'volume;
            }
            let fade = self.sources[s].fade;
            let v = if 0.0 < length && length > r {
                let v = (((volume as f64 * r as f64) / length as f64) as f32).clamp(0.0, 1.0);
                if 0.0 < fade_in { (fade as f64 * v as f64) as f32 } else { v }
            } else {
                if !(0.0 < fade_in) || !(fade < 1.0) {
                    break 'volume;
                }
                ((volume as f64 * fade as f64) as f32).clamp(0.0, 1.0)
            };
            self.set_source_volume(Some(s), v);
        }
        if self.sources[s].playing && !self.sources[s].children.is_empty() {
            for k in 0..self.sources[s].children.len() {
                let c = self.sources[s].children[k];
                if self.sources[s].timed {
                    let timer = (time as f64 + self.sources[c].timer as f64) as f32;
                    self.sources[c].timer = timer;
                    let threshold = |sandy: &Sandy| (sandy.sources[c].interval_max as f64 - sandy.sources[c].interval_min as f64) * (1.0 - sandy.sources[s].volume as f64) + sandy.sources[c].interval_min as f64;
                    if threshold(self) < timer as f64 {
                        let delay = if self.sources[s].pass_delay { self.sources[s].remaining } else { 0.0 };
                        let position = self.sources[s].position;
                        self.play_sample(Some(c), 1.0, position, delay, 0, 0.0, 100, 1, 0, [0.0; 3]);
                        self.sources[c].timer = (self.sources[c].timer as f64 - threshold(self)) as f32;
                    }
                } else if 0.0 < self.sources[c].timer {
                    let timer = (self.sources[c].timer as f64 - time as f64) as f32;
                    self.sources[c].timer = timer;
                    if timer <= 0.0 {
                        let delay = if self.sources[s].pass_delay { self.sources[s].remaining } else { 0.0 };
                        let volume = self.sources[s].volume;
                        self.play_sample(Some(c), volume, [0.0; 3], delay, 0, 0.0, 100, 1, 0, [0.0; 3]);
                        let r = self.rand.next() % 1000;
                        let (imin, imax) = (self.sources[c].interval_min, self.sources[c].interval_max);
                        self.sources[c].timer = ((r as f64 * (imax as f64 - imin as f64)) / 1000.0 + imin as f64) as f32;
                    }
                }
                self.frame_process_sound(c, time);
            }
        }
    }

    /// `Frameprocess(time)`: the engine's master volume set, the seconds counted, the first deferred play due carried out,
    /// the active sources (the first 511) processed, the queued ones picked a new child at random (not the last), the play
    /// list carried out.
    pub fn frame_process(&mut self, time: f32) {
        if !self.initialized {
            return;
        }
        let master = self.master;
        self.engine.set_master_volume(master);
        let second = (time as f64 + self.second as f64) as f32;
        self.second = second;
        if 1.0 < second {
            self.seconds += 1;
            self.second = (second as f64 - (second as f64).floor()) as f32;
        }
        if self.music_updated {
            if let Some(p) = &self.player {
                let v = self.master_music;
                p.with(|p| p.set_volume(v));
                self.music_updated = false;
            }
        }
        if 0.0 < self.combat.timer {
            self.combat.timer = (self.combat.timer as f64 - time as f64) as f32;
        }
        if !(0.0 < self.combat.timer) && !self.combat.timer.is_nan() {
            self.process_combat_music();
        }
        self.frame += 1;
        self.combat.updates = 0;
        for d in &mut self.deferred {
            d.defer = (d.defer as f64 - time as f64) as f32;
        }
        if let Some(i) = self.deferred.iter().position(|d| d.defer <= 0.0) {
            // (each one counted down until the first due, as the original's walk stops there)
            for d in &mut self.deferred[i + 1..] {
                d.defer = (d.defer as f64 + time as f64) as f32;
            }
            let d = self.deferred.remove(i);
            self.play_sample(Some(d.source), d.volume, d.position, d.delay, d.index, 0.0, 100, 1, 0, [0.0; 3]);
        }
        let active: Vec<usize> = self.active.iter().take(0x1ff).copied().collect();
        for s in active {
            self.frame_process_sound(s, time);
        }
        for s in std::mem::take(&mut self.queued) {
            if self.sources[s].sequential && self.sources[s].queued {
                let n = self.sources[s].children.len() as i32;
                let mut r = self.rand.next() % n;
                if r == self.sources[s].current {
                    r += 1;
                    if n <= r {
                        r = 0;
                    }
                }
                self.sources[s].current = r;
            }
            self.sources[s].queued = false;
        }
        self.execute_play_list();
    }

    /// `SetStaticMasterMusicVolume(volume)`: noted for the next frame.
    pub fn set_static_master_music(&mut self, v: f32) {
        if v != self.master_music {
            self.music_updated = true;
            self.master_music = v;
        }
    }

    /// `LoadSimProject()`: without a player, one made from `sound/music/env/anarchy.sws` at the music volume; then the battle
    /// layers looked up (after `sim.config`, which the game does not have), and it played.
    fn load_sim_project(&mut self) {
        if self.player.is_none() {
            let dir = self.root.join("music/env");
            if let Some(project) = std::fs::read(dir.join("anarchy.sws")).ok().and_then(|d| mao_formats::music::Project::parse(&d)) {
                let mut p = crate::music::Player::new(std::sync::Arc::new(project), &dir, self.engine.streams());
                p.set_volume(self.master_music);
                self.player = Some(crate::music::Running::start(p));
            }
        }
        if let Some(p) = &self.player {
            let layers = p.with(|p| {
                let mut l = [-1i32; 17];
                for (k, name) in BATTLE.iter().enumerate() {
                    l[k + 1] = p.project().find_layer(name);
                }
                l
            });
            self.layers = layers;
            p.with(|p| p.play(None, true));
        }
    }

    /// The project's layer of a name (`FindLayerID`), -2 without a player.
    pub fn find_layer(&self, name: &str) -> i32 {
        self.player.as_ref().map_or(-2, |p| p.with(|p| p.project().find_layer(name)))
    }

    /// `PlayMusic(layer)`: the layer (or with combat music on, the override's or the battle layer of its state) signalled to
    /// the player (made and started if there is none; stopped for no layer), unless muted or already playing it.
    pub fn play_music(&mut self, layer: i32) {
        if self.music_mute || !self.initialized {
            return;
        }
        let mut layer = layer;
        let state = self.music_state;
        if 0 < state && state < 0x11 {
            if let Some(name) = self.override_name.clone().filter(|_| self.override_layer == 0) {
                if self.player.is_some() {
                    self.override_layer = self.find_layer(&name);
                }
                if self.override_layer == -1 {
                    self.override_layer = 0;
                }
            }
            layer = self.override_layer;
            if layer == 0 {
                layer = self.layers[state as usize];
            }
        }
        let force = self.force_transitions as i32;
        match &self.player {
            None => {
                self.load_sim_project();
                let Some(p) = &self.player else { return };
                p.with(|p| {
                    p.signal_event(layer, force, 2000);
                    p.play(None, true);
                });
            }
            Some(p) => {
                if layer == 0xffff {
                    layer = -1;
                }
                if self.layer == layer {
                    return;
                }
                let force = force != 0 || layer == -1;
                let old = self.layer;
                p.with(|p| {
                    if layer != -1 {
                        p.signal_event(layer, force as i32, 2000);
                        if old == -1 {
                            p.play(None, true);
                        }
                    } else {
                        p.stop(false, true);
                    }
                });
            }
        }
        self.layer = layer;
    }

    /// `StopMusic()`: the player stopped and deleted.
    pub fn stop_music(&mut self) {
        self.player = None;
        self.district = -1;
        self.layer = -1;
        self.music_state = 0;
        self.override_layer = 0;
    }

    /// `SetMusicMute(mute)`: muting stops the music.
    pub fn set_music_mute(&mut self, mute: bool) {
        self.music_mute = mute;
        if mute {
            self.player = None;
            self.layer = -1;
        }
    }

    /// `PlayStartupMusic()`: `mountain\night`.
    pub fn play_startup_music(&mut self) {
        self.load_sim_project();
        if self.player.is_some() {
            let layer = self.find_layer("mountain\\night");
            self.play_music(layer);
        }
    }

    /// `SetupENVSounds(district, layer)`: the district noted and its music played (-911: the `Landcontrol_neutral` layer).
    pub fn setup_env_sounds(&mut self, district: u32, layer: i32) {
        if !self.initialized {
            self.district = -1;
            self.layer = -1;
            return;
        }
        self.district = district as i32;
        let mut layer = layer;
        if layer == -0x38f {
            if self.landcontrol == 0 {
                if self.player.is_none() {
                    return;
                }
                self.landcontrol = self.find_layer("Landcontrol_neutral");
                if self.landcontrol == 0 {
                    return;
                }
            }
            layer = self.landcontrol;
        }
        self.play_music(layer);
    }

    /// `PlayDistrictSample(name)`: the source `<name>_<district>` played.
    pub fn play_district(&mut self, name: &str) {
        let s = self.source_of(sound_id(&format!("{name}_{}", self.district)));
        self.play_sample(s, 1.0, [0.0; 3], 0.0, 0, 0.0, 100, 1, 0, [0.0; 3]);
    }

    /// `SetCombatMusicOverride(name)`: kept, its layer looked up when next needed.
    pub fn set_combat_music_override(&mut self, name: Option<&str>) {
        self.override_name = name.map(str::to_string);
        self.override_layer = 0;
    }

    /// `CombatUpdate(own, enemies, level, faction, target, flag, health)`: a report of the player's own fight, or of the
    /// target's (kept when about the same target with no enemies, or with more health, or as much and more enemies).
    #[allow(clippy::too_many_arguments)]
    pub fn combat_update(&mut self, own: bool, enemies: i32, mut level: i32, mut faction: i32, target: i32, flag: bool, health: i32) {
        let c = &mut self.combat;
        if !(0..=2).contains(&faction) {
            faction = 0;
        }
        if level < 0 {
            level = 0;
        }
        if own {
            (c.faction, c.health, c.enemies, c.level, c.flag) = (faction, health, enemies, level, flag);
        } else if target == c.last_target && enemies == 0 {
            (c.target_enemies, c.target_health, c.target_level, c.target, c.target_flag) = (0, health, level, target, flag);
        } else {
            let take = if c.target_health < health && enemies != 0 {
                c.target_level = level;
                true
            } else {
                c.target_health == health && enemies != 0 && c.target_enemies < enemies
            };
            if take {
                (c.target_enemies, c.target_flag, c.target_health, c.target) = (enemies, flag, health, target);
            }
        }
        c.updates += 1;
    }

    /// `ProcessCombatMusic()`: the combat music's state from the frame's reports (see the drop-in's `process_combat_music`).
    fn process_combat_music(&mut self) {
        let updates = self.combat.updates;
        if updates == 0 || self.combat_preferences == 0 {
            self.music_state = 0;
            return;
        }
        let c = &mut self.combat;
        let started = if c.last_enemies == 0 {
            (c.enemies != 0).then(|| (c.faction + 1, f32::from_bits(0x41c80000)))
        } else {
            (c.enemies == 0).then(|| (c.faction + 4, f32::from_bits(0x40a00000)))
        };
        match started {
            Some((state, timer)) => {
                c.timer = timer;
                self.music_state = state;
            }
            None => self.combat_going_on(updates),
        }
        let c = &mut self.combat;
        c.last_enemies = c.enemies;
        c.enemies = 0;
        if updates > 1 {
            (c.last_target, c.last_target_enemies, c.last_target_level, c.last_target_health) = (c.target, c.target_enemies, c.target_level, c.target_health);
        }
        (c.target_enemies, c.target_level, c.target_health) = (0, 0, 0);
    }

    /// The fight going on (or none): the balance of the player's and the target's health picking a layer.
    fn combat_going_on(&mut self, updates: i32) {
        let preferences = self.combat_preferences;
        if updates < 2 {
            self.music_state = 0;
            return;
        }
        let (theirs, mine) = (self.combat.target_health, self.combat.health);
        let balance = if mine < theirs {
            200 - mine.wrapping_mul(100) / theirs
        } else if theirs < mine {
            theirs.wrapping_mul(100) / mine
        } else {
            if preferences < 2 {
                self.music_state = 0;
                return;
            }
            return self.combat_winning(100);
        };
        if balance < 0x19 || (balance < 100 && preferences < 3) || (balance < 0x96 && preferences < 2) {
            self.music_state = 0;
            return;
        }
        self.combat_winning(balance);
    }

    fn combat_winning(&mut self, balance: i32) {
        let c = &mut self.combat;
        if 99 < balance || !c.target_flag {
            if c.last_target_enemies != 0 && c.target_enemies == 0 {
                c.timer = f32::from_bits(0x41a00000);
                self.music_state = c.faction + 7;
                return;
            }
            let d = c.enemies.wrapping_sub(c.target_enemies);
            if 0x3a < (d.wrapping_add(0x1d)) as u32 {
                let big = (0x85 < balance) as i32;
                self.music_state = if d < 0x3c {
                    if d < 0x1e {
                        if d < -0x3b {
                            big * 2 + 0xe
                        } else {
                            if -0x1e < d {
                                self.music_state = 0;
                                return;
                            }
                            big + 0xe
                        }
                    } else {
                        big + 0xb
                    }
                } else {
                    big * 2 + 0xb
                };
                return;
            }
        }
        self.music_state = 10;
    }

    /// `PlayPlayerFX(name, volume)`: `sfx\Player\<name>` played, made as a player effect source the first time.
    pub fn play_player_fx(&mut self, name: &str, volume: f32) {
        let full = format!("sfx\\Player\\{name}");
        let id = sound_id(&full);
        let s = match self.source_of(id) {
            Some(s) => s,
            None => {
                let s = Source { rate: 0xac44, has_file: true, file: full.replace('\\', "/"), volume_max: 1.0, priority: 1, player_fx: true, ..Source::default() };
                self.sources.push(s);
                let s = self.sources.len() - 1;
                self.by_id.insert(id, s);
                self.all.push(s);
                s
            }
        };
        self.play_sample(Some(s), volume, [0.0; 3], 0.0, 0, 0.0, 100, 1, 0, [0.0; 3]);
    }

    /// `MuteAllbutNext()`.
    pub fn mute_all_but_next(&mut self) {
        self.mute_all_but_next = true;
        self.next_allowed = true;
    }
}

/// FUN_100111b0: a record's settings taken, a differing one marking the source changed (a differing child count empties
/// the children, the caller adds them again; an equal one keeps them, so they double); a positive loop minimum makes it
/// loop. When changed, the turn restarts, the volume is random if its range is, and a timed source's timer starts at 0,
/// another's at a random point of its interval.
fn update(s: &mut Source, r: &Record, variants: &[Option<usize>; 46], count: usize, rand: &mut Rand) {
    s.id = r.id;
    macro_rules! set {
        ($field:ident, $value:expr) => {
            if s.$field != $value {
                s.$field = $value;
                s.changed = true;
            }
        };
    }
    set!(all, r.all);
    set!(priority, r.priority);
    set!(random, r.random);
    set!(sequential, r.sequential);
    set!(timed, r.timed);
    set!(interval_min, r.interval_min);
    set!(interval_max, r.interval_max);
    set!(volume_min, r.volume_min);
    set!(volume_max, r.volume_max);
    set!(loop_min, r.loop_min);
    set!(loop_max, r.loop_max);
    set!(rate, 0x5622);
    set!(has_file, r.has_file);
    set!(looping, r.looping);
    set!(length, r.length);
    set!(doppler, r.doppler);
    set!(u16d, r.u16d);
    set!(chance, r.chance);
    set!(alert_radius, r.alert_radius);
    set!(min_distance, r.min_distance);
    set!(max_distance, r.max_distance);
    set!(noise_radius, r.noise_radius);
    set!(group, r.group);
    set!(stop_group, r.stop_group);
    set!(stop_others, r.stop_others);
    set!(fade_in, r.fade_in);
    set!(pass_delay, r.pass_delay);
    if 0.0 < s.loop_min {
        s.looping = true;
    }
    if s.children.len() != count {
        s.children.clear();
        s.changed = true;
    }
    if s.file != r.file {
        s.file = r.file.clone();
        s.changed = true;
    }
    s.has_variants = false;
    for k in 0..46 {
        if s.variants[k] != variants[k] {
            s.variants[k] = variants[k];
            s.changed = true;
        }
        if s.variants[k].is_some() {
            s.has_variants = true;
        }
    }
    if s.changed {
        s.current = 0;
        s.random_volume = r.volume_max != r.volume_min;
        s.timer = if s.timed { 0.0 } else { (((rand.next() % 1000) as f64 * (s.interval_max as f64 - s.interval_min as f64)) / 1000.0 + s.interval_min as f64) as f32 };
    }
}

/// `SandyInterfaceModule_t`: the interface (made on the initialise message) and the volumes the game sets, each with a
/// mute, their products the totals the interface gets.
pub struct Module {
    pub sandy: Option<Sandy>,
    pub master: f32,
    pub master_mute: f32,
    pub fx: f32,
    pub fx_mute: f32,
    pub player_fx: f32,
    pub player_fx_mute: f32,
    pub music: f32,
    pub music_mute: f32,
    pub total_fx: f32,
    pub total_player_fx: f32,
    pub total_music: f32,
}

/// A product of floats in the order given, as the x87 chains them.
fn product(v: &[f32]) -> f32 {
    v.iter().skip(1).fold(v[0] as f64, |p, &x| p * x as f64) as f32
}

impl Default for Module {
    fn default() -> Module {
        Module { sandy: None, master: 1.0, master_mute: 1.0, fx: 1.0, fx_mute: 1.0, player_fx: 1.0, player_fx_mute: 1.0, music: 1.0, music_mute: 1.0, total_fx: 1.0, total_player_fx: 1.0, total_music: 1.0 }
    }
}

/// `MapGameMaterialToSoundMaterial(material)`.
pub fn map_material(material: i32) -> i32 {
    match material {
        1 => 1,
        2 => 2,
        4 => 4,
        5 | 0xe => 5,
        6 => 6,
        7 | 0xd => 7,
        8 => 8,
        9 => 9,
        10 => 10,
        0xb => 0xb,
        0xc => 0xf,
        0xf => 0xc,
        0x10 => 0xd,
        0x11 => 0xe,
        _ => 3,
    }
}

impl Module {
    fn pass_fx(&mut self) {
        let total = self.total_fx;
        if let Some(s) = &mut self.sandy {
            s.set_master_fx(total);
            s.master_fx = total;
        }
    }

    fn pass_player_fx(&mut self) {
        let total = self.total_player_fx;
        if let Some(s) = &mut self.sandy {
            s.master_player_fx = total;
        }
    }

    /// `SetMasterVolume(volume)`: the effect and music totals.
    pub fn set_master_volume(&mut self, v: f32) {
        self.master = v;
        self.total_fx = product(&[self.fx, v, self.master_mute, self.fx_mute]);
        self.total_music = product(&[self.music, v, self.master_mute, self.music_mute]);
        let total = self.total_fx;
        if let Some(s) = &mut self.sandy {
            s.set_master_fx(total);
            s.master = v;
        }
    }

    /// `SetMasterFXVolume(volume)`.
    pub fn set_master_fx_volume(&mut self, v: f32) {
        self.fx = v;
        self.total_fx = product(&[self.master, v, self.master_mute, self.fx_mute]);
        self.pass_fx();
    }

    /// `SetMasterPlayerFXVolume(volume)`.
    pub fn set_master_player_fx_volume(&mut self, v: f32) {
        self.player_fx = v;
        self.total_player_fx = product(&[self.master, v, self.master_mute, self.player_fx_mute]);
        self.pass_player_fx();
    }

    /// `SetMasterPlayerFXMute(mute)`.
    pub fn set_master_player_fx_mute(&mut self, mute: bool) {
        self.player_fx_mute = if mute { 0.0 } else { 1.0 };
        self.total_player_fx = product(&[self.master, self.player_fx, self.master_mute, self.player_fx_mute]);
        self.pass_player_fx();
    }

    /// `SetMasterMute(mute)`: the totals (only the effect one passed on).
    pub fn set_master_mute(&mut self, mute: bool) {
        self.master_mute = if mute { 0.0 } else { 1.0 };
        self.total_fx = product(&[self.master, self.fx, self.master_mute, self.fx_mute]);
        self.total_music = product(&[self.master, self.music, self.master_mute, self.music_mute]);
        self.pass_fx();
    }

    /// `SetMasterFXMute(mute)`.
    pub fn set_master_fx_mute(&mut self, mute: bool) {
        self.fx_mute = if mute { 0.0 } else { 1.0 };
        self.total_fx = product(&[self.master, self.fx, self.master_mute, self.fx_mute]);
        self.pass_fx();
    }

    /// `SetFadeFXVolume(fade)`: the effect total faded.
    pub fn set_fade_fx_volume(&mut self, fade: f32) {
        self.total_fx = product(&[self.master, self.fx, self.master_mute, self.fx_mute, fade]);
        self.pass_fx();
    }

    /// `SetMasterMusicVolume(volume)`.
    pub fn set_master_music_volume(&mut self, v: f32) {
        self.music = v;
        self.total_music = product(&[self.master, v, self.master_mute, self.music_mute]);
        let total = self.total_music;
        if let Some(s) = &mut self.sandy {
            s.set_static_master_music(total);
        }
    }

    /// `SetMasterMusicMute(mute)`: the music muted too.
    pub fn set_master_music_mute(&mut self, mute: bool) {
        self.music_mute = if mute { 0.0 } else { 1.0 };
        self.total_music = product(&[self.master, self.music, self.master_mute, self.music_mute]);
        let total = self.total_music;
        if let Some(s) = &mut self.sandy {
            s.set_static_master_music(total);
            s.set_music_mute(mute);
        }
    }

    /// `SetBattleMusicMode(mode)`.
    pub fn set_battle_music_mode(&mut self, mode: i32) {
        if let Some(s) = &mut self.sandy {
            s.combat_preferences = mode;
        }
    }

    /// `PlayStartupMusic()`.
    pub fn play_startup_music(&mut self) {
        if let Some(s) = &mut self.sandy {
            s.play_startup_music();
        }
    }

    /// `ActivateEnvZone(district, layer)`.
    pub fn activate_env_zone(&mut self, district: u32, layer: i32) {
        if let Some(s) = &mut self.sandy {
            s.setup_env_sounds(district, layer);
        }
    }

    /// `CombatUpdate(...)`.
    #[allow(clippy::too_many_arguments)]
    pub fn combat_update(&mut self, own: bool, enemies: i32, level: i32, faction: i32, target: i32, flag: bool, health: i32) {
        if let Some(s) = &mut self.sandy {
            s.combat_update(own, enemies, level, faction, target, flag, health);
        }
    }

    /// `FindLayerID(name)`.
    pub fn find_layer(&self, name: &str) -> i32 {
        self.sandy.as_ref().map_or(-2, |s| s.find_layer(name))
    }

    /// `PlaySample(id)`.
    pub fn play_sample(&mut self, id: u32) {
        if let Some(s) = &mut self.sandy {
            let source = s.source_of(id);
            s.play_sample(source, 1.0, [0.0; 3], 0.0, 0, 0.0, 100, 1, 0, [0.0; 3]);
        }
    }

    /// `StopSample(id)`, `StopGameSound(id)`, `StopEnvSound(id)`.
    pub fn stop_sample(&mut self, id: u32) {
        if let Some(s) = &mut self.sandy {
            let source = s.source_of(id);
            s.stop_sample(source);
        }
    }

    /// `PlayGameSound(id, position, delay, volume, material, defer, pitch, mode, range, velocity)`.
    #[allow(clippy::too_many_arguments)]
    pub fn play_game_sound(&mut self, id: u32, position: [f32; 3], delay: f32, volume: f32, material: i32, defer: f32, pitch: i32, mode: i32, range: u32, velocity: [f32; 3]) {
        if let Some(s) = &mut self.sandy {
            let m = map_material(material);
            let source = s.source_of(id);
            s.play_sample(source, volume, position, delay, m, defer, pitch, mode, range, velocity);
        }
    }

    /// `PlayEnvSound(id, position, delay, volume, material, defer, pitch)`.
    #[allow(clippy::too_many_arguments)]
    pub fn play_env_sound(&mut self, id: u32, position: [f32; 3], delay: f32, volume: f32, material: i32, defer: f32, pitch: i32) {
        if let Some(s) = &mut self.sandy {
            let m = map_material(material);
            let source = s.source_of(id);
            s.play_sample(source, volume, position, delay, m, defer, pitch, 1, 0, [0.0; 3]);
        }
    }

    /// `PlaySoundCommand()`: a play as an effect's sound control sends it: the range less 0.49999, rounded to even.
    #[allow(clippy::too_many_arguments)]
    pub fn play_sound_command(&mut self, id: u32, position: [f32; 3], velocity: [f32; 3], volume: f32, range: f32, delay: f32, defer: f32, pitch: i32) {
        if let Some(s) = &mut self.sandy {
            let r = (range as f64 - 0.49999).round_ties_even();
            let range = if r.is_nan() || !(-2147483648.0..=2147483647.0).contains(&r) { i32::MIN } else { r as i32 };
            let source = s.source_of(id);
            s.play_sample(source, volume, position, delay, 0, defer, pitch, 1, range as u32, velocity);
        }
    }

    /// `PlayPlayerFX(name, volume)`: at the player effect total.
    pub fn play_player_fx(&mut self, name: &str, v: f32) {
        let total = self.total_player_fx;
        if let Some(s) = &mut self.sandy {
            s.play_player_fx(name, (total as f64 * v as f64) as f32);
        }
    }

    /// `SetListenerRotPos(rotation, position)`: the position.
    pub fn set_listener(&mut self, position: [f32; 3]) {
        if let Some(s) = &mut self.sandy {
            s.listener = position;
        }
    }

    /// `ActivateGameZone()`: the game's source file.
    pub fn activate_game_zone(&mut self) {
        if let Some(s) = &mut self.sandy {
            s.setup_game_sounds();
        }
    }

    /// `StopAllSoundEffects()`.
    pub fn stop_all_sound_effects(&mut self) {
        if let Some(s) = &mut self.sandy {
            s.stop_all();
        }
    }

    /// `FrameProcess()`: the interface's frame.
    pub fn frame_process(&mut self, time: f32) {
        if let Some(s) = &mut self.sandy {
            s.frame_process(time);
        }
    }
}

/// The module (the original's static `SandyInterfaceModule_t`), shared by the threads that play sounds.
static MODULE: Mutex<Option<Module>> = Mutex::new(None);

/// The module, made with its interface and started (`InitialiseMessage`, `InitializeSandy`, then GUI.dll's
/// `GUISoundInitialisation`) over the game's sound folder, the first time. `MAO_NO_SOUND` keeps it silent.
pub fn start(game: &Path) {
    let mut m = MODULE.lock().unwrap_or_else(|e| e.into_inner());
    if m.is_some() {
        return;
    }
    let mut module = Module::default();
    if std::env::var_os("MAO_NO_SOUND").is_none() {
        let mut sandy = Sandy::new(&game.join("cd_image/sound"), Engine::new(true));
        sandy.initialize();
        sandy.setup_gui_sounds();
        module.sandy = Some(sandy);
    }
    apply_preferences(&mut module, &game.join("cd_image/gui/Default/LoginPrefs.xml"));
    *m = Some(module);
}

/// The sound preferences of the GUI's `LoginPrefs.xml` passed on as GUI.dll's slots pass them (0x100c5242 on:
/// `SlotPrefMasterVolumeChanged` → `SetMasterVolume`, `SoundFXOn` → `SetMasterFXMute`, `FXVolume`, `MusicOn` →
/// `SetMasterMusicMute`, `MusicVolume`, `BattlemusicMode`, `VoiceSndFxOn` → `SetMasterPlayerFXMute`, `VoiceSndFxVolume`).
fn apply_preferences(m: &mut Module, path: &Path) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    let value = |name: &str| -> Option<String> {
        let at = text.find(&format!("name=\"{name}\""))?;
        let line = &text[text[..at].rfind('<')?..at + text[at..].find('>')?];
        let v = line.find("value=\"")? + 7;
        Some(line[v..v + line[v..].find('"')?].to_string())
    };
    let float = |name: &str| value(name).and_then(|v| v.parse::<f32>().ok());
    let on = |name: &str| value(name).map(|v| v == "true");
    if let Some(v) = float("MasterVolume") {
        m.set_master_volume(v);
    }
    if let Some(on) = on("SoundFXOn") {
        m.set_master_fx_mute(!on);
    }
    if let Some(v) = float("FXVolume") {
        m.set_master_fx_volume(v);
    }
    if let Some(on) = on("MusicOn") {
        m.set_master_music_mute(!on);
    }
    if let Some(v) = float("MusicVolume") {
        m.set_master_music_volume(v);
    }
    if let Some(v) = value("BattlemusicMode").and_then(|v| v.parse::<i32>().ok()) {
        m.set_battle_music_mode(v);
    }
    if let Some(on) = on("VoiceSndFxOn") {
        m.set_master_player_fx_mute(!on);
    }
    if let Some(v) = float("VoiceSndFxVolume") {
        m.set_master_player_fx_volume(v);
    }
}

/// The module, if started.
pub fn with<R>(f: impl FnOnce(&mut Module) -> R) -> Option<R> {
    let mut m = MODULE.lock().unwrap_or_else(|e| e.into_inner());
    m.as_mut().map(f)
}

/// GUI.dll and Gamecode's `GetSoundID(name)` then `PlaySample(id)`.
pub fn play_named(name: &str) {
    with(|m| m.play_sample(sound_id(name)));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An interface over sources made by hand (no files, no device).
    fn sandy() -> Sandy {
        let mut s = Sandy::new(Path::new("."), Engine::new(false));
        s.initialize();
        s
    }

    fn record(id: u32) -> Record {
        let mut d = vec![0xb, 0, 0, 0, 1, 0, 0, 0];
        d.extend(vec![0u8; 0xe4]);
        let mut r = parse_bank(&d).unwrap().remove(0);
        r.id = id;
        r.volume_max = 0.5;
        r.volume_min = 0.5;
        r.max_distance = 30.0;
        r.priority = 1;
        r.chance = 100;
        r
    }

    fn add(s: &mut Sandy, r: &Record) -> usize {
        let variants = std::array::from_fn(|j| (r.variants[j] != 0).then(|| s.obtain(r.variants[j]).0));
        let children: Vec<usize> = r.children.iter().map(|&c| s.obtain(c).0).collect();
        let (k, _) = s.obtain(r.id);
        s.all.push(k);
        update(&mut s.sources[k], r, &variants, children.len(), &mut Rand::default());
        s.sources[k].children.extend(children);
        k
    }

    #[test]
    fn distance_scales_the_volume_and_the_loudest_play_counts() {
        let mut s = sandy();
        let mut r = record(1);
        r.min_distance = 10.0;
        let a = add(&mut s, &r);
        // (20 of 30 away, 10 to 30 fading: half volume, times the record's 0.5)
        s.play_sample(Some(a), 1.0, [20.0, 0.0, 0.0], 0.0, 0, 0.0, 100, 1, 0, [0.0; 3]);
        assert_eq!(s.plays[&a].volume, 0.25);
        s.play_sample(Some(a), 1.0, [5.0, 0.0, 0.0], 0.0, 0, 0.0, 100, 1, 0, [0.0; 3]);
        assert_eq!(s.plays[&a].volume, 0.5);
        // (out of range: no play)
        let mut far = sandy();
        let b = add(&mut far, &r);
        far.play_sample(Some(b), 1.0, [31.0, 0.0, 0.0], 0.0, 0, 0.0, 100, 1, 0, [0.0; 3]);
        assert!(far.plays.is_empty());
    }

    #[test]
    fn a_variant_by_material_and_children_in_turn() {
        let mut s = sandy();
        let mut walk = record(1);
        walk.variants[0] = 2;
        walk.variants[3] = 3;
        let w = add(&mut s, &walk);
        let (grass, dirt) = (add(&mut s, &record(2)), add(&mut s, &record(3)));
        // (`PlayGameSound`'s material 0x11 maps to 0xe: no variant 0xe, so the first)
        s.play_sample(Some(w), 1.0, [0.0; 3], 0.0, map_material(0x11), 0.0, 100, 1, 0, [0.0; 3]);
        assert!(s.plays.contains_key(&grass) && !s.plays.contains_key(&dirt));
        s.frame_process(0.016);
        s.play_sample(Some(w), 1.0, [0.0; 3], 0.0, 3, 0.0, 100, 1, 0, [0.0; 3]);
        assert!(s.plays.contains_key(&dirt));
        // (a source of three children in turn picks another child each frame)
        let mut seq = record(10);
        seq.sequential = true;
        seq.children = vec![11, 12, 13];
        let q = add(&mut s, &seq);
        for id in 11..14 {
            add(&mut s, &record(id));
        }
        let mut picked = Vec::new();
        for _ in 0..6 {
            s.play_sample(Some(q), 1.0, [0.0; 3], 0.0, 0, 0.0, 100, 1, 0, [0.0; 3]);
            picked.push(s.sources[q].current);
            s.frame_process(0.016);
        }
        assert!(picked.windows(2).all(|w| w[0] != w[1]), "{picked:?}");
    }

    #[test]
    fn deferred_plays_and_volume_totals() {
        let mut s = sandy();
        let a = add(&mut s, &record(1));
        s.play_sample(Some(a), 1.0, [0.0; 3], 0.0, 0, 0.5, 100, 1, 0, [0.0; 3]);
        assert!(s.plays.is_empty());
        s.frame_process(0.3);
        assert_eq!(s.deferred.len(), 1);
        s.frame_process(0.3);
        assert!(s.deferred.is_empty());
        let mut m = Module { sandy: Some(s), ..Module::default() };
        m.set_master_volume(0.5);
        m.set_master_fx_volume(0.5);
        assert_eq!(m.total_fx, 0.25);
        m.set_master_fx_mute(true);
        assert_eq!(m.sandy.as_ref().unwrap().master_fx, 0.0);
    }

    #[test]
    fn crt_rand() {
        let mut r = Rand::default();
        assert_eq!([r.next(), r.next(), r.next()], [41, 18467, 6334]);
    }
}
