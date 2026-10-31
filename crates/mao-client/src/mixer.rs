//! The sound engine the sound system plays through: SandyInterface.dll's `SoundEngineInterface_t` (the `sandyinterface`
//! drop-in's `engine.rs`) and the Miles sample handles under it, here mixed into one rodio source.
//!
//! As the original: a limit of 13 channels; a sound takes a channel when it plays, a new handle while under the limit, else
//! the handle of the first listed sound not playing, else that of the first sound of priority 2, then 1, ... down to the
//! asked (`AllocateChannel`); a one-shot play is listed and restarts when playing (`SE_Play2DSound`), a looping play is
//! not listed and does nothing while playing (`SE_Play2DSoundLooping`); the pitch is the rate the sample played at times
//! a percentage (`SE_Update2DSoundPitch`); volumes are Miles's linear levels, the same on both sides, under the driver's
//! master level. The driver is 44.1 kHz stereo (`SE_Initialise`).

use std::sync::{Arc, Mutex};

/// A decoded sample: its rate, channels (1 or 2) and interleaved samples.
#[derive(Debug)]
pub struct Pcm {
    pub rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl Pcm {
    /// A sample file decoded (None for what Miles can't play: `SE_Create2DSoundFromFile` drops other types).
    pub fn decode(data: &[u8]) -> Option<Pcm> {
        use mao_formats::sound::{Sample, read_sample};
        let streamed = |decoder: Result<rodio::Decoder<std::io::Cursor<Vec<u8>>>, rodio::decoder::DecoderError>| -> Option<Pcm> {
            use rodio::Source;
            let decoder = decoder.ok()?;
            let (rate, channels) = (decoder.sample_rate().get(), decoder.channels().get());
            let samples: Vec<f32> = decoder.map(|s| s as f32).collect();
            (channels <= 2 && !samples.is_empty()).then_some(Pcm { rate, channels, samples })
        };
        match read_sample(data)? {
            Sample::Pcm { rate, channels, samples } => (channels <= 2).then(|| Pcm { rate, channels, samples: samples.into_iter().map(|s| s as f32 / 32768.0).collect() }),
            Sample::Mp3(d) => streamed(rodio::Decoder::new_mp3(std::io::Cursor::new(d.to_vec()))),
            Sample::Ogg(d) => streamed(rodio::Decoder::new_vorbis(std::io::Cursor::new(d.to_vec()))),
        }
    }

    fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }
}

/// The Miles status of a sample playing (`SMP_PLAYING`) and of one done (`SMP_DONE`).
pub const PLAYING: i32 = 4;
pub const DONE: i32 = 2;
/// A stream opened and not started, or paused (`SMP_STOPPED`).
pub const STOPPED: i32 = 8;

/// A Miles sample handle: its sample, where it is, its rate, loops left (0: for ever), status and levels.
#[derive(Default)]
struct Voice {
    pcm: Option<Arc<Pcm>>,
    position: f64,
    rate: u32,
    loops: i32,
    status: i32,
    volume: f32,
}

/// What the output reads: the handles (None: free) and the driver's master level.
#[derive(Default)]
pub struct Voices {
    voices: Vec<Option<Voice>>,
    master: f32,
}

/// The driver's rate.
pub const RATE: u32 = 44100;

impl Voices {
    /// Mixed stereo frames into `out` (interleaved), the handles moved on.
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let master = self.master;
        for voice in self.voices.iter_mut().flatten() {
            if voice.status != PLAYING {
                continue;
            }
            let Some(pcm) = voice.pcm.clone() else { continue };
            let frames = pcm.frames();
            if frames == 0 {
                voice.status = DONE;
                continue;
            }
            let step = voice.rate as f64 / RATE as f64;
            let level = voice.volume * master;
            for frame in out.chunks_exact_mut(2) {
                let at = voice.position as usize;
                if at >= frames {
                    if voice.loops == 1 {
                        voice.status = DONE;
                        break;
                    }
                    if voice.loops > 1 {
                        voice.loops -= 1;
                    }
                    voice.position -= frames as f64;
                    continue;
                }
                let next = (at + 1).min(frames - 1);
                let t = (voice.position - at as f64) as f32;
                let sample = |frame: usize, c: usize| pcm.samples[frame * pcm.channels as usize + c.min(pcm.channels as usize - 1)];
                for (c, o) in frame.iter_mut().enumerate() {
                    let s = sample(at, c) + (sample(next, c) - sample(at, c)) * t;
                    *o += s * level;
                }
                voice.position += step;
            }
        }
        for o in out.iter_mut() {
            *o = o.clamp(-1.0, 1.0);
        }
    }
}

/// The rodio source reading the handles in blocks.
struct Output {
    voices: Arc<Mutex<Voices>>,
    block: Vec<f32>,
    at: usize,
}

impl Iterator for Output {
    type Item = rodio::Sample;
    fn next(&mut self) -> Option<rodio::Sample> {
        if self.at == self.block.len() {
            if let Ok(mut v) = self.voices.lock() {
                v.render(&mut self.block);
            }
            self.at = 0;
        }
        self.at += 1;
        Some(self.block[self.at - 1] as rodio::Sample)
    }
}

impl rodio::Source for Output {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        std::num::NonZero::new(2).unwrap()
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        std::num::NonZero::new(RATE).unwrap()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

/// A sound made from a file (`Sound2D_s`): its sample, the handle it holds, its priority and the rate it played at.
pub struct Sound {
    pcm: Arc<Pcm>,
    handle: Option<usize>,
    pub priority: u8,
    rate: i32,
}

/// A sound's place in the engine.
pub type SoundKey = usize;

/// `SoundEngineInterface_t`.
pub struct Engine {
    voices: Arc<Mutex<Voices>>,
    /// Open (`SE_IsSoundEngineAllocated`): a device was found.
    pub allocated: bool,
    /// The sounds that took a channel, in order.
    listed: Vec<SoundKey>,
    max_channels: i32,
    channels: i32,
    sounds: Vec<Option<Sound>>,
}

impl Engine {
    /// An engine; `output` makes it play (else it only keeps the handles, as the tests use it).
    pub fn new(output: bool) -> Engine {
        let voices = Arc::new(Mutex::new(Voices { voices: Vec::new(), master: 1.0 }));
        let allocated = !output || start_output(voices.clone());
        Engine { voices, allocated, listed: Vec::new(), max_channels: 13, channels: 0, sounds: Vec::new() }
    }

    fn voices(&self) -> std::sync::MutexGuard<'_, Voices> {
        self.voices.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `SE_Create2DSoundFromFile(path, priority)`: a sound of the file's sample, or None.
    pub fn create_sound(&mut self, path: &std::path::Path, priority: u8) -> Option<SoundKey> {
        if !self.allocated {
            return None;
        }
        let pcm = Pcm::decode(&mao_formats::files::read(path).ok()?)?;
        let sound = Sound { pcm: Arc::new(pcm), handle: None, priority, rate: 44100 };
        let key = match self.sounds.iter().position(Option::is_none) {
            Some(k) => {
                self.sounds[k] = Some(sound);
                k
            }
            None => {
                self.sounds.push(Some(sound));
                self.sounds.len() - 1
            }
        };
        Some(key)
    }

    /// `SE_Is2DSoundAllocated(sound)`.
    pub fn is_sound_allocated(&self, sound: Option<SoundKey>) -> bool {
        sound.is_some_and(|k| self.sounds.get(k).is_some_and(Option::is_some))
    }

    fn handle(&self, sound: SoundKey) -> Option<usize> {
        self.sounds.get(sound)?.as_ref()?.handle
    }

    fn status(&self, handle: usize) -> i32 {
        self.voices().voices.get(handle).and_then(Option::as_ref).map_or(DONE, |v| v.status)
    }

    /// `SE_SetMasterVolume(volume)`.
    pub fn set_master_volume(&mut self, volume: f32) {
        if self.allocated {
            self.voices().master = volume;
        }
    }

    /// `SE_Set2DSoundVolume(sound, volume)`.
    pub fn set_volume(&mut self, sound: SoundKey, volume: f32) {
        if let Some(h) = self.handle(sound) {
            if let Some(Some(v)) = self.voices().voices.get_mut(h) {
                v.volume = volume;
            }
        }
    }

    /// `SE_Update2DSoundPitch(sound, percent)`: the rate it played at scaled.
    pub fn set_pitch(&mut self, sound: SoundKey, percent: i32) {
        let Some(rate) = self.sounds.get(sound).and_then(Option::as_ref).map(|s| s.rate) else { return };
        if let Some(h) = self.handle(sound) {
            if let Some(Some(v)) = self.voices().voices.get_mut(h) {
                v.rate = scaled(rate, percent);
            }
        }
    }

    /// `SE_Is2DSoundPlaying(sound)`.
    pub fn is_playing(&self, sound: Option<SoundKey>) -> bool {
        sound.and_then(|s| self.handle(s)).is_some_and(|h| self.status(h) == PLAYING)
    }

    /// `AllocateChannel(priority)`.
    fn allocate_channel(&mut self, priority: i32) -> Option<usize> {
        let mut handle = None;
        if self.channels < self.max_channels {
            let mut v = self.voices();
            let free = v.voices.iter().position(Option::is_none);
            let h = match free {
                Some(h) => h,
                None => {
                    v.voices.push(None);
                    v.voices.len() - 1
                }
            };
            v.voices[h] = Some(Voice { status: DONE, ..Voice::default() });
            handle = Some(h);
            drop(v);
            self.channels += 1;
        } else {
            // (a sound done with its channel)
            if let Some(i) = self.listed.iter().position(|&s| self.handle(s).is_some_and(|h| self.status(h) != PLAYING)) {
                let s = self.listed.remove(i);
                handle = self.sounds[s].as_mut().and_then(|s| s.handle.take());
            }
            // (else one of the lowest priority, the highest number, ended)
            let mut level = 2;
            while handle.is_none() && priority <= level {
                while let Some(i) = self.listed.iter().position(|&s| self.sounds[s].as_ref().is_some_and(|s| s.priority as i32 == level)) {
                    let s = self.listed.remove(i);
                    if let Some(h) = self.sounds[s].as_mut().and_then(|s| s.handle.take()) {
                        if let Some(Some(v)) = self.voices().voices.get_mut(h) {
                            v.status = DONE;
                        }
                        handle = Some(h);
                        break;
                    }
                }
                level -= 1;
            }
        }
        if let Some(h) = handle {
            if let Some(Some(v)) = self.voices().voices.get_mut(h) {
                *v = Voice { status: DONE, ..Voice::default() };
            }
        }
        handle
    }

    /// A sample started on the sound's handle, looped `loops` times (0: for ever), its rate scaled.
    fn start(&mut self, sound: SoundKey, loops: i32, percent: i32) -> bool {
        let Some(s) = self.sounds[sound].as_mut() else { return false };
        let Some(h) = s.handle else { return false };
        s.rate = s.pcm.rate as i32;
        let (pcm, rate) = (s.pcm.clone(), scaled(s.rate, percent));
        if let Some(Some(v)) = self.voices().voices.get_mut(h) {
            *v = Voice { pcm: Some(pcm), position: 0.0, rate, loops, status: PLAYING, volume: v.volume };
        }
        true
    }

    /// `SE_Play2DSoundLooping(sound, priority, percent)`.
    pub fn play_looping(&mut self, sound: SoundKey, _priority: i32, percent: i32) {
        if !self.is_sound_allocated(Some(sound)) || self.is_playing(Some(sound)) {
            return;
        }
        if self.handle(sound).is_none() {
            let Some(h) = self.allocate_channel(0) else { return };
            self.sounds[sound].as_mut().unwrap().handle = Some(h);
        }
        self.start(sound, 0, percent);
    }

    /// `SE_Play2DSound(sound, priority, percent)`.
    pub fn play(&mut self, sound: SoundKey, priority: i32, percent: i32) {
        if !self.is_sound_allocated(Some(sound)) {
            return;
        }
        let mut fresh = true;
        if self.handle(sound).is_some() {
            if self.is_playing(Some(sound)) {
                self.stop(sound, false);
            }
            fresh = false;
        }
        if self.handle(sound).is_none() {
            let Some(h) = self.allocate_channel(priority) else { return };
            self.sounds[sound].as_mut().unwrap().handle = Some(h);
        }
        if self.start(sound, 1, percent) && fresh {
            self.listed.push(sound);
        }
    }

    /// `SE_Stop2DSound(sound, release)`: ended; when releasing, its channel given back and it taken off the list.
    pub fn stop(&mut self, sound: SoundKey, release: bool) {
        if let Some(h) = self.handle(sound) {
            if let Some(Some(v)) = self.voices().voices.get_mut(h) {
                v.status = DONE;
            }
            if !release {
                return;
            }
            self.give_back(h);
            if let Some(s) = self.sounds[sound].as_mut() {
                s.handle = None;
            }
        }
        if release {
            if let Some(i) = self.listed.iter().position(|&s| s == sound) {
                self.listed.remove(i);
            }
        }
    }

    /// One channel fewer (the count kept at 0, the handle then kept, as the original's).
    fn give_back(&mut self, handle: usize) {
        self.channels -= 1;
        if self.channels < 0 {
            self.channels = 0;
        } else if let Some(v) = self.voices().voices.get_mut(handle) {
            *v = None;
        }
    }

    /// `SE_Delete2DSound(sound)`: stopped and released, the sound gone.
    pub fn delete_sound(&mut self, sound: SoundKey) {
        if !self.is_sound_allocated(Some(sound)) {
            return;
        }
        self.stop(sound, true);
        self.sounds[sound] = None;
    }

    /// The streams on the engine's output.
    pub fn streams(&self) -> Streams {
        Streams(self.voices.clone())
    }

    /// Channels taken (for the tests).
    pub fn channels(&self) -> i32 {
        self.channels
    }

    /// Moves the handles on by `frames` without a device (for the tests).
    pub fn advance(&mut self, frames: usize) {
        let mut out = vec![0.0; frames * 2];
        self.voices().render(&mut out);
    }
}

/// Miles's streams (`HSTREAM`) on the same output, apart from the engine's 13 channels: the music player's (SIMPlayer.dll
/// opens them on the sound engine's driver). A stream plays once from where it is.
#[derive(Clone)]
pub struct Streams(Arc<Mutex<Voices>>);

impl Streams {
    fn voices(&self) -> std::sync::MutexGuard<'_, Voices> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn with<R>(&self, h: usize, f: impl FnOnce(&mut Voice) -> R) -> Option<R> {
        self.voices().voices.get_mut(h).and_then(Option::as_mut).map(f)
    }

    /// `AIL_open_stream`: the sample on a stream of its own, stopped at its start, at full volume.
    pub fn open(&self, pcm: Arc<Pcm>) -> usize {
        let mut v = self.voices();
        let voice = Voice { rate: pcm.rate, pcm: Some(pcm), position: 0.0, loops: 1, status: STOPPED, volume: 1.0 };
        match v.voices.iter().position(Option::is_none) {
            Some(h) => {
                v.voices[h] = Some(voice);
                h
            }
            None => {
                v.voices.push(Some(voice));
                v.voices.len() - 1
            }
        }
    }

    /// `AIL_close_stream`.
    pub fn close(&self, h: usize) {
        if let Some(v) = self.voices().voices.get_mut(h) {
            *v = None;
        }
    }

    /// `AIL_start_stream`: playing from where it is (no project's transition enters a sample past its start, so where Miles
    /// starts it does not show).
    pub fn start(&self, h: usize) {
        self.with(h, |v| {
            if v.status != DONE {
                v.status = PLAYING;
            }
        });
    }

    /// `AIL_pause_stream(on)`.
    pub fn pause(&self, h: usize, on: bool) {
        self.with(h, |v| {
            if v.status != DONE {
                v.status = if on { STOPPED } else { PLAYING };
            }
        });
    }

    /// `AIL_stream_status` (DONE for none).
    pub fn status(&self, h: usize) -> i32 {
        self.with(h, |v| v.status).unwrap_or(DONE)
    }

    /// `AIL_stream_ms_position`: the length and where it is, in ms.
    pub fn ms_position(&self, h: usize) -> (i32, i32) {
        self.with(h, |v| {
            let Some(p) = &v.pcm else { return (0, 0) };
            let rate = p.rate.max(1) as f64;
            ((p.frames() as f64 * 1000.0 / rate) as i32, (v.position.min(p.frames() as f64) * 1000.0 / rate) as i32)
        })
        .unwrap_or((0, 0))
    }

    /// `AIL_set_stream_ms_position`.
    pub fn set_ms_position(&self, h: usize, ms: i32) {
        self.with(h, |v| {
            if let Some(p) = &v.pcm {
                v.position = ms.max(0) as f64 * p.rate as f64 / 1000.0;
            }
        });
    }

    /// `AIL_set_sample_volume_levels` on the stream's sample.
    pub fn set_volume(&self, h: usize, volume: f32) {
        self.with(h, |v| v.volume = volume);
    }

    /// Moves the voices on by `frames` without a device (for the tests).
    pub fn advance(&self, frames: usize) {
        let mut out = vec![0.0; frames * 2];
        self.voices().render(&mut out);
    }
}

/// A rate times a percentage (the original's unsigned division).
fn scaled(rate: i32, percent: i32) -> u32 {
    (rate.wrapping_mul(percent) as u32) / 100
}

/// The device opened on a thread of its own (the stream lives there) and the handles' source added to it.
fn start_output(voices: Arc<Mutex<Voices>>) -> bool {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut s) => {
                s.log_on_drop(false);
                s
            }
            Err(e) => {
                eprintln!("audio: no output device ({e})");
                let _ = tx.send(false);
                return;
            }
        };
        sink.mixer().add(Output { voices, block: vec![0.0; 1024], at: 1024 });
        let _ = tx.send(true);
        loop {
            std::thread::park();
        }
    });
    rx.recv().unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_with(n: usize, priority: u8) -> (Engine, Vec<SoundKey>) {
        let mut e = Engine::new(false);
        let keys = (0..n)
            .map(|_| {
                e.sounds.push(Some(Sound { pcm: Arc::new(Pcm { rate: 22050, channels: 1, samples: vec![0.5; 2205] }), handle: None, priority, rate: 44100 }));
                e.sounds.len() - 1
            })
            .collect();
        (e, keys)
    }

    #[test]
    fn channels_run_out_and_lower_priorities_give_way() {
        let (mut e, keys) = engine_with(14, 2);
        for &k in &keys[..13] {
            e.play(k, 2, 100);
        }
        assert_eq!(e.channels(), 13);
        assert!(keys[..13].iter().all(|&k| e.is_playing(Some(k))));
        // (the 14th takes the first priority-2 sound's channel, ending it)
        e.play(keys[13], 2, 100);
        assert!(e.is_playing(Some(keys[13])) && !e.is_playing(Some(keys[0])));
        assert_eq!(e.channels(), 13);
    }

    #[test]
    fn a_one_shot_ends_and_a_loop_goes_on() {
        let (mut e, keys) = engine_with(2, 1);
        e.play(keys[0], 1, 100);
        e.play_looping(keys[1], 1, 200);
        // (past 0.1 s at 22.05 kHz; the loop at twice the rate)
        e.advance(5000);
        assert!(!e.is_playing(Some(keys[0])));
        assert!(e.is_playing(Some(keys[1])));
        let rate = e.voices().voices[e.handle(keys[1]).unwrap()].as_ref().unwrap().rate;
        assert_eq!(rate, 44100);
        e.stop(keys[1], true);
        assert_eq!(e.channels(), 1);
    }
}
