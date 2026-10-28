//! The environment's sounds, as `Environment_t`'s frame (Gamecode FUN_100becbe) plays them through `PlayEnvSound`: the
//! district's background by the time of day (FUN_100ba779 over the windows FUN_100bd930 makes), the rain, and the four winds
//! (FUN_100bff68). The sources are `Environment_t`'s (FUN_100be0bb's `GetSoundID`s): `SM_Sandy_Env_Rain` (+0x6c),
//! `SM_Sandy_Env_Wind`, `…_SandWind`, `…_FalloutRWind`, `…_FalloutGWind` (+0x7c to +0x88: the wind object's). The weather values
//! are the blended table the frame reads (`mao_formats::weather`, as `wind.rs` takes entry 7 for the wind).
//!
//! Not here: the quake (`SM_Sandy_Env_Quake`, +0x70, with its camera shake), the lightning's thunder (FUN_100bde3c: the client
//! draws no lightning) and the positional ambience zones (FUN_100ad1f8).

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
