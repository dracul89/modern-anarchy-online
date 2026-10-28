//! The environment's sounds, as `Environment_t`'s frame (Gamecode FUN_100becbe) plays them through `PlayEnvSound`: the
//! district's background by the time of day (FUN_100ba779 over the windows FUN_100bd930 makes), the rain, and the four winds
//! (FUN_100bff68). The sources are `Environment_t`'s (FUN_100be0bb's `GetSoundID`s): `SM_Sandy_Env_Rain` (+0x6c),
//! `SM_Sandy_Env_Wind`, `…_SandWind`, `…_FalloutRWind`, `…_FalloutGWind` (+0x7c to +0x88: the wind object's). The weather values
//! are the blended table the frame reads (`mao_formats::weather`, as `wind.rs` takes entry 7 for the wind).
//!
//! The quake ([`Quake`]) plays `SM_Sandy_Env_Quake` (+0x70) and shakes the camera. Not here: the lightning's thunder
//! (FUN_100bde3c: the client draws no lightning). The positional ambience zones (FUN_100ad1f8) play nothing: their list
//! (DAT_102e43f0) is only ever made and freed.

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
