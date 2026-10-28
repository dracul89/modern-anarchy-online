//! The environment's sounds, as `Environment_t`'s frame (Gamecode FUN_100becbe) plays them through `PlayEnvSound`: the
//! district's background by the time of day (FUN_100ba779 over the windows FUN_100bd930 makes), the rain, and the four winds
//! (FUN_100bff68). The sources are `Environment_t`'s (FUN_100be0bb's `GetSoundID`s): `SM_Sandy_Env_Rain` (+0x6c),
//! `SM_Sandy_Env_Wind`, `…_SandWind`, `…_FalloutRWind`, `…_FalloutGWind` (+0x7c to +0x88: the wind object's). The weather values
//! are the blended table the frame reads (`mao_formats::weather`, as `wind.rs` takes entry 7 for the wind).
//!
//! The quake ([`Quake`]) plays `SM_Sandy_Env_Quake` (+0x70) and shakes the camera; the rain's lightning ([`Lightning`])
//! thunders. FUN_100bde3c's strike at the character (AFCM message 0x7f, FUN_100b2d7e) has no sender in the client. The
//! positional ambience zones (FUN_100ad1f8) play nothing: their list (DAT_102e43f0) is only ever made and freed.

use mao_formats::sound::sound_id;

/// The day's windows in seconds (FUN_100bd930 from `GameTime_t`'s hours: dawn 7 for 2, dusk 21 for 2, an hour 3600 s), each
/// `(fading in from, full from, fading out from, silent from)`, the fades 300 s (0x1015e680).
const FADE: f32 = 300.0;
fn windows() -> [(&'static str, [f32; 4]); 4] {
    let (dawn, dawn_end, dusk, dusk_end) = (7.0 * 3600.0, 9.0 * 3600.0, 21.0 * 3600.0, 23.0 * 3600.0);
    [
        ("SM_Sandy_Env_BackgroundNight", [0.0, 0.0, dawn - FADE, dawn]),
        ("SM_Sandy_Env_BackgroundDawn", [dawn - FADE, dawn, dawn_end - FADE, dawn_end]),
        ("SM_Sandy_Env_BackgroundDay", [dawn_end - FADE, dawn_end, dusk - FADE, dusk]),
        ("SM_Sandy_Env_BackgroundDusk", [dusk - FADE, dusk, dusk_end - FADE, dusk_end]),
    ]
}

/// The background sources playing at a time of day in a district (FUN_100ba779: an entry's level is 0 outside its window,
/// rising over the fade in, 1, falling over the fade out; each above 0 plays its `<name>_<district>` at full volume). After the
/// dusk's last fade the frame takes the time as 3600 s (0x10169460), in the night's window.
pub fn background(seconds: f32, district: u32) -> Vec<u32> {
    if district >= 1000 {
        return Vec::new();
    }
    let t = if (23.0 * 3600.0 - FADE) < seconds { 3600.0 } else { seconds };
    windows()
        .into_iter()
        .filter(|(_, w)| {
            let level = if t <= w[0] || ((w[3] < t) != (w[3] == t)) {
                0.0
            } else if ((w[1] < t) == (w[1] == t)) || w[2] < t {
                if w[1] <= t { 1.0 - (t - w[2]) / (w[3] - w[2]) } else { (t - w[0]) / (w[1] - w[0]) }
            } else {
                1.0
            };
            0.0 < level
        })
        .map(|(name, _)| sound_id(&format!("{name}_{district}")))
        .collect()
}

/// The rain's volume (FUN_100becbe: the table's entry 0 squared times its entry 8 over 3.5, held to 0..1).
pub fn rain(v: &[f32; 14]) -> (u32, f32) {
    let k = (v[8] / 3.5).clamp(0.0, 1.0);
    (sound_id("SM_Sandy_Env_Rain"), v[0] * k * v[0])
}

/// The winds' volumes (FUN_100bff68 as FUN_100becbe calls it: the strength, entry 7, held to 0..33; the gust's length over 33,
/// held to 0..1; each wind `(0.5 + 0.5 × gust) × strength / 33` times its share: the wind `max(1 − entry 2, entry 4)`, the
/// sand wind entry 3, the red and the green fallout winds entries 5 and 6).
pub fn winds(v: &[f32; 14], gust: f32) -> [(u32, f32); 4] {
    let strength = if 33.0 < v[7] || v[7] < 0.0 { if v[7] < 0.0 { 0.0 } else { 33.0 } } else { v[7] };
    let g = (gust / 33.0).clamp(0.0, 1.0);
    let level = (0.5 + g * 0.5) * (strength / 33.0);
    let wind = (1.0 - v[2]).max(v[4]);
    [
        (sound_id("SM_Sandy_Env_Wind"), level * wind),
        (sound_id("SM_Sandy_Env_SandWind"), level * v[3]),
        (sound_id("SM_Sandy_Env_FalloutRWind"), level * v[5]),
        (sound_id("SM_Sandy_Env_FalloutGWind"), level * v[6]),
    ]
}

/// The quake of `Environment_t`'s frame (FUN_100becbe; its fields +0x54 the quake, +0x58 its full strength, 2 s, +0x74 the
/// count towards the next): while the weather's entry 12 is above 0 the count grows each frame by `r % 10000 / 100 × dt / 10 ×
/// entry 12`; past 100 it drops by 100 and a quake of `2 / (4 − k)` starts (FUN_100bd902, keeping a stronger one), `k` from 1 to
/// `entry 12 / 33.33` (at least 1). While it lasts it plays the quake's sound at its share of the full strength, shakes the
/// eye by `(r % 100 − 50) × 0.5 / 300 × share` on each axis (`n3Camera_t` +0x1d4) and runs down by the frame's time; at 0 the
/// sound is stopped (`StopEnvSound`).
#[derive(Clone, Debug, Default)]
pub struct Quake {
    pub amount: f32,
    count: f32,
    rand: crate::audio::Rand,
}

/// The quake's full strength (+0x58, 0x10159794).
const QUAKE: f32 = 2.0;

impl Quake {
    /// A frame of `dt` seconds under the weather's entry 12: the sound's volume (none: stopped) and the eye's offset (Y up).
    pub fn step(&mut self, entry12: f32, dt: f32) -> (Option<f32>, [f32; 3]) {
        if 0.0 < entry12 {
            let r = (self.rand.next() % 10000) as f32;
            self.count += ((r / 100.0) * dt / 10.0) * entry12;
            if 100.0 <= self.count {
                self.count -= 100.0;
                let n = ((entry12 as f64 / 33.33332824707031) as i32).max(1);
                let k = self.rand.next() % n + 1;
                let start = QUAKE / (4 - k) as f32;
                if self.amount < start {
                    self.amount = start;
                }
            }
        }
        if self.amount <= 0.0 {
            return (None, [0.0; 3]);
        }
        let share = self.amount / QUAKE;
        let mut shake = || ((((self.rand.next() % 100 - 50) as f32 * 0.5) / 300.0) * self.amount) / QUAKE;
        let offset = [shake(), shake(), shake()];
        self.amount = (self.amount - dt).max(0.0);
        (Some(share), offset)
    }
}

/// The thunder of the rain's lightning (`VisualRain_t`'s owner, Gamecode FUN_100b9775 makes it, FUN_100b9da8 its frame from
/// FUN_100becbe): two schedules of strikes ([`Bolts`]), the storm's (seed 0x85c19, gaps of 0.1 to 20 s over 600 s) and the
/// fallout's (0x1e23a, 0.1 to 2 s over 300 s), the second used while the weather's entries 5 and 6 add up to more than 0.2
/// (FUN_100becbe sets +0x74). While the weather's entry 9 is above 0 (+0x68, FUN_100b9aff) and its entry 2 at least 0.4
/// (+0x60), each strike due plays `SM_Sandy_Env_NearLightning` (+0x70) through `PlayEnvSound` at `1 − d / S × 0.5`, deferred
/// `d / 100` s, `d` the strike's distance across the ground; otherwise the due strikes pass unheard. Not here: the bolt drawn
/// (`VisualEnvFX_t` effect 8, whose `ActivateFX` is taken to succeed) and the eye's shake within 25 m (AFCM message 9/0xbf).
pub struct Lightning {
    bolts: [Bolts; 2],
}

/// One strike: its sound's volume and deferral.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thunder {
    pub volume: f32,
    pub defer: f32,
}

impl Lightning {
    /// Made at `now`, the game clock's seconds into the day (each schedule caught up to it, FUN_100b148b).
    pub fn new(now: f32) -> Lightning {
        Lightning { bolts: [Bolts::new(0x85c19, 600.0, 0.1, 20.0, now), Bolts::new(0x1e23a, 300.0, 0.1, 2.0, now)] }
    }

    /// A frame (FUN_100b9da8's part of it): the weather table, the game clock's seconds into the day, where the character is
    /// (the listener) and the zone's side (tile size × zone size; none without a tile map), of which the strikes' cell `S` is
    /// 6 (FUN_100b1bff). The strikes heard.
    pub fn frame(&mut self, v: &[f32; 14], now: f32, at: [f32; 3], zone_side: Option<f32>) -> Vec<Thunder> {
        let b = &mut self.bolts[if v[5] + v[6] > 0.2 { 1 } else { 0 }];
        let mut out = Vec::new();
        let on = 0.0 < v[9] && !(0.4 > v[2]);
        if let (true, Some(side)) = (on, zone_side) {
            let s = side * 6.0;
            while b.step(now) {
                // (FUN_100b1bff, FUN_100b1d1f: the cell's corner under the character plus the strike's next number across it,
                // kept within half a cell of the character)
                let place = |b: &mut Bolts, p: f32| {
                    let base = ((p / s) as f64).floor() as f32 * s;
                    let x = (b.next_float() as f64 * s as f64 + base as f64) as f32;
                    if x - p > s * 0.5 {
                        x - s
                    } else if x - p < -(s * 0.5) {
                        s + x
                    } else {
                        x
                    }
                };
                let x = place(b, at[0]);
                let z = place(b, at[2]);
                let (dx, dz) = (x - at[0], z - at[2]);
                let d = (dx * dx + dz * dz).sqrt();
                out.push(Thunder { volume: (1.0 - (d as f64 / s as f64) * 0.5) as f32, defer: (d as f64 / 100.0) as f32 });
            }
        }
        while b.step(now) {}
        out
    }
}

/// A seeded R250 generator (Gamecode FUN_1013de47 seeds it from the table at 0x101727f8, FUN_1013de89 a word, FUN_1013dec9
/// a word over 2³²).
#[derive(Clone)]
struct R250 {
    w: [u32; 250],
    i: usize,
}

impl R250 {
    fn new(seed: u32) -> R250 {
        let mut r = R250 { w: R250_TABLE, i: 0 };
        r.seed(seed);
        r
    }

    fn seed(&mut self, seed: u32) {
        self.w = R250_TABLE;
        self.i = ((seed & 0xff) as usize).min(0xf9);
        self.w[self.i] = self.w[self.i].wrapping_add(seed);
    }

    fn next(&mut self) -> u32 {
        let j = if self.i < 0x93 { self.i + 0x67 } else { self.i - 0x93 };
        let v = self.w[j] ^ self.w[self.i];
        self.w[self.i] = v;
        self.i = if self.i < 0xf9 { self.i + 1 } else { 0 };
        v
    }

    fn float(&mut self) -> f32 {
        (self.next() as f64 * 2.3283064365386963e-10) as f32
    }
}

/// A strike of a schedule: its time in the period, its numbers (16 floats) and which was read last.
#[derive(Clone)]
struct Strike {
    time: f32,
    floats: Vec<f32>,
    read: usize,
}

/// A schedule of strikes (FUN_100b15a4 makes it, FUN_100b1277 steps it): gaps drawn at random between `min` and `max` until
/// they fill the period, all but the last kept as strikes, each with its own seed (a word of the generator) for its numbers;
/// the period runs on the game clock's seconds into the day over its pace of 15 (`GameTime_t` +0x5c), wrapping.
#[derive(Clone)]
struct Bolts {
    period: f32,
    strikes: Vec<Strike>,
    current: usize,
    last: usize,
    previous: f32,
    fired: usize,
    started: bool,
}

impl Bolts {
    fn new(seed: u32, period: f32, min: f32, max: f32, now: f32) -> Bolts {
        let period = if period < 0.1 { 0.1 } else { period };
        let mut r = R250::new(seed);
        let span = max as f64 - min as f64;
        let (mut count, mut t) = (0usize, 0.0f32);
        if period > 0.0 {
            loop {
                count += 1;
                t = (r.float() as f64 * span + min as f64 + t as f64) as f32;
                if !(t < period) {
                    break;
                }
            }
        }
        let count = if count < 2 { 0 } else { count - 1 };
        r.seed(seed);
        let mut times = Vec::with_capacity(count);
        let mut t = 0.0f32;
        for _ in 0..count {
            t = (r.float() as f64 * span + min as f64 + t as f64) as f32;
            times.push(t);
        }
        let seeds: Vec<u32> = (0..count).map(|_| r.next()).collect();
        let strikes = times
            .into_iter()
            .zip(seeds)
            .map(|(time, seed)| {
                r.seed(seed);
                let floats = (0..16).map(|_| r.float()).collect();
                Strike { time, floats, read: 0 }
            })
            .collect();
        let mut b = Bolts { period, strikes, current: 0, last: 0, previous: 0.0, fired: 0, started: false };
        while b.step(now) {}
        b
    }

    /// FUN_100b1277: the next strike due by `now` (seconds into the day), if any; when none is, the time is kept for the next
    /// frame.
    fn step(&mut self, now: f32) -> bool {
        let now = ((now as f64 / 15.0) as f32 as f64 % self.period as f64) as f32;
        let n = self.strikes.len();
        if n != 0 && self.fired < n {
            if self.started {
                self.current += 1;
                if self.current == n {
                    self.current = 0;
                }
            }
            let i = self.current;
            let time = self.strikes[i].time;
            // (wrapped round the period: due at or before now, or after the last time; else between the two)
            let due = if self.previous > now { time <= now || self.previous < time } else { time <= now && self.previous < time };
            if due {
                self.fired += 1;
                self.last = i;
                self.strikes[i].read = 0;
                self.started = true;
                return true;
            }
        }
        self.previous = now;
        self.fired = 0;
        self.started = false;
        false
    }

    /// FUN_100b138c: the last strike's next float (its index stepped first, masked by 15).
    fn next_float(&mut self) -> f32 {
        let s = &mut self.strikes[self.last];
        s.read = (s.read + 1) & 15;
        s.floats[s.read]
    }
}

/// The R250 generator's starting words (Gamecode 0x101727f8).
#[rustfmt::skip]
const R250_TABLE: [u32; 250] = [
    0xf6230029, 0x26e16784, 0x20ae2cd6, 0xe7495f90, 0xa8bb5af1, 0xadb301eb, 0x923c12db, 0x963e390c, 0x3d5e0124, 0xfa06491c,
    0xe7de1547, 0xcf4d2d12, 0xedbb6443, 0x731f26a6, 0x7f7d7a5a, 0x24251238, 0xd1d46e5d, 0x8a966bfc, 0x213b4e45, 0x6189260d,
    0xa5db301c, 0x9b200732, 0x62ee2350, 0x96365878, 0x28493e12, 0x479e3bf6, 0xa0dc5f49, 0xac14314f, 0x48404944, 0xf26b1cd0,
    0x4cb74230, 0x37a12c3b, 0x99223ef6, 0x99e1409d, 0xc3da121f, 0x349926ca, 0xca727bb9, 0xe92c7049, 0x8fc5187e, 0x93e93cd5,
    0xfaea5db2, 0xe85348cc, 0x83d65c67, 0x14d62f14, 0xdadc422d, 0x96830d66, 0x4d494657, 0xe3692fff, 0xf3cd3a61, 0xc29d261e,
    0x13721916, 0xb01d32e6, 0x354f0384, 0x93020677, 0x7c396be8, 0xa2cb1953, 0x6c1e0e12, 0x289e7874, 0x89d511f4, 0xe8d45a9f,
    0x277e2059, 0x453207cf, 0xc1cc1af4, 0x239001d3, 0xd8d36048, 0xeee60975, 0xc02a591d, 0x62f71dc0, 0x93815078, 0x060e7b44,
    0xff001850, 0x848d7f61, 0x45050c7b, 0x443b3807, 0x3d1f7282, 0xee926270, 0x83545064, 0x72853bb1, 0x92156d69, 0x4c6a5c46,
    0x50731796, 0x301673d9, 0x8d684d67, 0x754a2cf7, 0x26575ed0, 0x70fa5876, 0xc21149bb, 0xd5244eae, 0x2efe5579, 0x642548db,
    0x9b3c0de5, 0xcbd35f45, 0x02ce0a28, 0x25c568f5, 0xb13d3459, 0x4a8a4027, 0xf82d5e76, 0xa2c97ac2, 0x31d42668, 0x4179086a,
    0x3a614e08, 0x68b17014, 0x51a50d6a, 0xf9c12528, 0x05a954d6, 0xb3973087, 0x25f1412f, 0xd89a441d, 0xbb9b00c1, 0x877e4fc0,
    0x989a5fa8, 0xf3c26486, 0xcabf7a54, 0x33d92fe7, 0xde5579d1, 0xd6282a38, 0xcd6c10d9, 0x125e4c66, 0x8c3001e1, 0x6ad94efe,
    0xeae2159f, 0xb40c28e2, 0x024766b4, 0x202a4e38, 0xd2a91289, 0x677a2079, 0x55c20878, 0x992626b1, 0x632927da, 0xc462113e,
    0x51127296, 0x88394ebf, 0x30891d3f, 0x4b6d1ff1, 0x226c06e3, 0x2c1e36a1, 0xdfcb721d, 0x363f1003, 0x08840607, 0x56057514,
    0x0a28791b, 0xbfc56bc9, 0x468e212c, 0xef087a36, 0xada84af3, 0xc5be78fe, 0x877100eb, 0x72d064a0, 0xbd061c75, 0x7287357e,
    0xd84d6d73, 0xe68254be, 0x11c243db, 0xd12b5841, 0x103003fa, 0xadf05a70, 0x35867954, 0x26aa1295, 0x7d132568, 0xeefc58e6,
    0x3fbe1eca, 0xf9890d9f, 0x991b0a41, 0x0b4f7cb8, 0x41721af6, 0xc3996014, 0x8f0d27d3, 0xac3a2044, 0x1c6613a6, 0xd20b7833,
    0x0a1052a1, 0x76ec3a4c, 0xb773134c, 0x49b43605, 0xe9aa4a0e, 0x9cd532cf, 0xe62765ca, 0x5f9d31d8, 0x6af4194d, 0x0f593a27,
    0x969c387c, 0xc7cd6af8, 0x53207987, 0xb1b87e64, 0x74954987, 0x8b5a5ab0, 0xad2d214e, 0xec3d5ae7, 0x83de3260, 0x39ad2780,
    0x3ae92d41, 0x8ba55e41, 0x3ae4199f, 0x6d15749f, 0x8a260e00, 0x72d1424c, 0x24935804, 0x708f09b3, 0x1a402753, 0x76af328a,
    0xcc6f5cca, 0xf2d34ecf, 0x1fce0c95, 0x1c657cbe, 0x44f65d2a, 0x91a158ad, 0xd3a60665, 0x4ec007c9, 0x20341b32, 0x394d012c,
    0x37174d8f, 0xf6db2cc6, 0xa7a64fc8, 0xef88480b, 0x4f612738, 0x50124e68, 0xea5403f9, 0x247177e7, 0x6a784e48, 0x51294a92,
    0xa2296586, 0x3cdb4b72, 0x64fa561c, 0x9eb82237, 0x789562b0, 0xdcdf0ef5, 0x9a6d726c, 0xe8d97374, 0x62651943, 0x9aa57a08,
    0xe0bd5079, 0x981521eb, 0x75e164e0, 0x366e3d8f, 0x54f46d7b, 0xb68a4962, 0x0127188f, 0xd5e501f7, 0x3b3b164a, 0x2b61569b,
    0xa0c62f84, 0x2da751b1, 0xca107c4a, 0x106f6275, 0x469e012f, 0x65102bef, 0xc4c80b9b, 0x7c292b43, 0xa1f6448a, 0x80473b9e,
];

/// `PlayEnvSound(id, origin, 0, volume, 0, 0, 100)`.
pub fn play(id: u32, volume: f32) {
    crate::audio::with(|m| m.play_env_sound(id, [0.0; 3], 0.0, volume, 0, 0.0, 100));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_day_s_backgrounds() {
        let at = |h: f32| background(h * 3600.0, 4);
        assert_eq!(at(3.0), vec![sound_id("SM_Sandy_Env_BackgroundNight_4")]);
        // (a crossfade: at 6:58 the night fading out and the dawn fading in)
        assert_eq!(at(6.0 + 58.0 / 60.0).len(), 2);
        assert_eq!(at(12.0), vec![sound_id("SM_Sandy_Env_BackgroundDay_4")]);
        assert_eq!(at(22.0), vec![sound_id("SM_Sandy_Env_BackgroundDusk_4")]);
        // (late night: taken as 1:00)
        assert_eq!(at(25.0), vec![sound_id("SM_Sandy_Env_BackgroundNight_4")]);
        assert!(background(0.0, 1000).is_empty());
    }

    #[test]
    fn a_quake_comes_shakes_and_goes() {
        let mut q = Quake::default();
        // (no quakes in calm weather)
        for _ in 0..1000 {
            assert_eq!(q.step(0.0, 0.1), (None, [0.0; 3]));
        }
        // (a strong entry 12 soon brings one; it plays and shakes, then runs down within its 2 s)
        let mut heard = false;
        for _ in 0..2000 {
            let (volume, offset) = q.step(80.0, 0.016);
            if let Some(v) = volume {
                heard = true;
                assert!(v > 0.0 && v <= 1.0 && offset.iter().all(|c| c.abs() <= 0.084));
            }
        }
        assert!(heard);
        let mut calm = q.clone();
        for _ in 0..200 {
            calm.step(0.0, 0.016);
        }
        assert_eq!(calm.step(0.0, 0.016).0, None);
    }

    #[test]
    fn the_storm_s_schedule() {
        // (the storm's schedule: a few dozen strikes over its 600 s, in order, each a gap of 0.1 to 20 s after the last)
        let b = Bolts::new(0x85c19, 600.0, 0.1, 20.0, 0.0);
        assert!(b.strikes.len() > 40 && b.strikes.len() < 120, "{}", b.strikes.len());
        let mut last = 0.0;
        for s in &b.strikes {
            assert!(s.time - last >= 0.099 && s.time - last < 20.0 && s.time < 600.0);
            last = s.time;
        }
        // (one game day's 15 game seconds a real second: the strikes of a stretch fire once, in order, then none)
        let mut l = Lightning::new(0.0);
        let mut v = [0.0f32; 14];
        v[9] = 1.0;
        v[2] = 0.5;
        let to = b.strikes[3].time * 15.0 + 1.0;
        let heard = l.frame(&v, to, [1000.0, 0.0, 1000.0], Some(256.0));
        assert_eq!(heard.len(), 4);
        assert!(heard.iter().all(|t| t.volume > 0.5 && t.volume <= 1.0 && t.defer >= 0.0 && t.defer < 16.0));
        assert!(l.frame(&v, to, [1000.0, 0.0, 1000.0], Some(256.0)).is_empty());
        // (in calm weather they pass unheard)
        let to2 = b.strikes[8].time * 15.0 + 1.0;
        assert!(l.frame(&[0.0; 14], to2, [0.0; 3], Some(256.0)).is_empty());
        assert!(l.frame(&v, to2, [0.0; 3], Some(256.0)).is_empty());
    }

    #[test]
    fn weather_volumes() {
        let mut v = [0.0f32; 14];
        v[7] = 33.0;
        let w = winds(&v, 0.0);
        assert_eq!(w[0].1, 0.5);
        v[2] = 1.0;
        v[3] = 1.0;
        let w = winds(&v, 66.0);
        assert_eq!((w[0].1, w[1].1), (0.0, 1.0));
        v[0] = 0.5;
        v[8] = 3.5;
        assert_eq!(rain(&v).1, 0.25);
    }
}
