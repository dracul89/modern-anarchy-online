//! The weather's precipitation round the eye: the four precipitations of `Environment_t`'s state table (entries 3 to 6 of
//! `environment::compute`'s weather values: the sand wind's dust, the rain, and the red and the green fallout, in the order
//! the fog colours take them) as particles for the renderer's particle pass, falling through a box that follows the eye.
//!
//! The original draws its rain with `VisualRain_t` (FUN_100b9da8 / FUN_100b9ba5 run it from the frame block's strongest
//! precipitation at +0xf0); its vertex layout is ported in `AnarchyReimagined`'s drop-in (`rain.rs`: the colour at +0xc..+0xe
//! of each pair), the streaks' count and fall are the client's own here. The dust is the sand wind's, blown along the wind;
//! the fallouts fall like rain in their colours.
//!
//! Render space is Z-up (the viewer's `[x, z, y]` of the server's). Each particle keeps a fixed place in the box (from its
//! number) and a phase, so from frame to frame it falls rather than flickers; the box wraps round the eye.

use mao_render::{Particle, NO_SPRITE};

/// The box the precipitation fills: its half width round the eye and its height above and below.
const REACH: f32 = 14.0;
const HEIGHT: f32 = 16.0;
/// The most particles of each kind at full intensity.
const MOST: usize = 700;

/// Seconds since the client started, the fall's clock.
pub fn clock() -> f32 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs_f32()
}

/// A cheap hash of a particle's number to 0..1.
fn unit(n: u32, salt: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9e37_79b9) ^ salt.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    (x & 0xffff) as f32 / 65535.0
}

/// Wraps `v` into `[-REACH, REACH)` round `centre`.
fn wrap(v: f32, centre: f32) -> f32 {
    let d = v - centre;
    centre + (d + REACH).rem_euclid(2.0 * REACH) - REACH
}

/// The kinds, by their weather entry: (entry, colour, a streak's half height, a dot's size, fall speed, drift with the wind).
struct Kind {
    entry: usize,
    colour: [f32; 4],
    streak: f32,
    size: f32,
    speed: f32,
    drift: f32,
}

const KINDS: [Kind; 4] = [
    // the sand wind's dust: faint tan motes blown nearly level
    Kind { entry: 3, colour: [0.72, 0.58, 0.36, 0.22], streak: 0.0, size: 0.10, speed: 1.2, drift: 1.0 },
    // the rain: grey-blue streaks
    Kind { entry: 4, colour: [0.72, 0.78, 0.9, 0.38], streak: 0.32, size: 0.012, speed: 11.0, drift: 0.25 },
    // the red fallout
    Kind { entry: 5, colour: [0.6, 0.12, 0.1, 0.4], streak: 0.26, size: 0.014, speed: 8.0, drift: 0.3 },
    // the green fallout
    Kind { entry: 6, colour: [0.25, 0.65, 0.2, 0.4], streak: 0.26, size: 0.014, speed: 8.0, drift: 0.3 },
];

/// The particles of the weather `v` round `eye` (render space) at `time` seconds, blown by `wind` (render space, per second).
pub fn particles(v: &[f32; 14], eye: [f32; 3], wind: [f32; 3], time: f32) -> Vec<Particle> {
    let mut out = Vec::new();
    for (k, kind) in KINDS.iter().enumerate() {
        let intensity = v[kind.entry].clamp(0.0, 1.0);
        if intensity <= 0.0 {
            continue;
        }
        let count = ((MOST as f32) * intensity) as usize;
        for n in 0..count {
            let n = n as u32;
            let salt = k as u32 * 977;
            // (a fixed place in the box, blown by the wind and falling with time, wrapped round the eye)
            let drift_x = wind[0] * kind.drift * time;
            let drift_y = wind[1] * kind.drift * time;
            let x = wrap(eye[0] + (unit(n, salt) * 2.0 - 1.0) * REACH + drift_x, eye[0]);
            let y = wrap(eye[1] + (unit(n, salt + 1) * 2.0 - 1.0) * REACH + drift_y, eye[1]);
            let fall = (unit(n, salt + 2) * HEIGHT + time * kind.speed).rem_euclid(HEIGHT);
            let z = eye[2] + HEIGHT * 0.5 - fall;
            let position = [x, y, z];
            if kind.streak > 0.0 {
                // (a streak: a thin quad standing up, turned to face the eye)
                let (dx, dy) = (eye[0] - x, eye[1] - y);
                let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                let right = [-dy / len * kind.size, dx / len * kind.size, 0.0];
                out.push(Particle { position, size: kind.streak, colour: kind.colour, sprite: NO_SPRITE, rect: [0.0, 0.0, 1.0, 1.0], right, up: [0.0, 0.0, kind.streak] });
            } else {
                out.push(Particle::dot(position, kind.size, kind.colour));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rain_falls_round_the_eye_and_nothing_falls_in_clear_weather() {
        let mut v = [0.0f32; 14];
        assert!(particles(&v, [0.0; 3], [0.0; 3], 1.0).is_empty());
        v[4] = 0.5;
        let eye = [100.0, 200.0, 30.0];
        let a = particles(&v, eye, [0.0; 3], 1.0);
        assert_eq!(a.len(), 350);
        for p in &a {
            assert!((p.position[0] - eye[0]).abs() <= REACH && (p.position[1] - eye[1]).abs() <= REACH);
            assert!((p.position[2] - eye[2]).abs() <= HEIGHT * 0.5 + 1e-3);
            assert!(p.up[2] > 0.0, "a streak stands up");
        }
        // (a moment later every drop is lower, or wrapped to the top)
        let b = particles(&v, eye, [0.0; 3], 1.1);
        let lower = a.iter().zip(&b).filter(|(p, q)| q.position[2] < p.position[2]).count();
        assert!(lower > 300, "{lower}");
        // (dust is dots, blown along the wind)
        let mut d = [0.0f32; 14];
        d[3] = 1.0;
        let dust = particles(&d, eye, [3.0, 0.0, 0.0], 2.0);
        assert_eq!(dust.len(), MOST);
        assert!(dust.iter().all(|p| p.up == [0.0; 3] && p.right == [0.0; 3]));
    }
}
