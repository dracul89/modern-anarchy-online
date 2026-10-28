//! Nano effects drawn as glowing sprites in the world (the renderer's particle pass). The templates come from `gfxtweak.bin`
//! (`mao_formats::gfx`); each effect class is played as its control in Gamecode plays it (the addresses are at each function).
//! A running buff's effect (kind 1001) pulses every few seconds (`pulses`); each pulse plays its two placeholders' rings
//! ([`ring`]: the cord ribbon `_GfxControlBuffPlaceHolder_t` lays as its point turns, of which the original only ever draws the
//! forward-spinning one).

use mao_formats::gfx::{colour_curve, curve, unpack_argb, BParticles, Cast, Cords, Deform, Emitter, Flare, Helix, Smoke, Sparks, Sprite, Stars, Visual, VolGrid};
use mao_formats::gfx::{Shield, Suns};
use mao_render::{Particle, Shell, ShellVertex, NO_SPRITE};

/// The wind the effects are pushed by, Z up: the effect handler's wind (+0x24, Sparks) and its smoothed copy (+0x30, Smoke).
/// The viewer sets it every frame (`crate::wind`); still air until then.
static WIND: std::sync::Mutex<([f32; 3], [f32; 3])> = std::sync::Mutex::new(([0.0; 3], [0.0; 3]));

pub fn set_wind(now: [f32; 3], smooth: [f32; 3]) {
    if let Ok(mut w) = WIND.lock() {
        *w = (now, smooth);
    }
}

fn wind() -> ([f32; 3], [f32; 3]) {
    WIND.lock().map(|w| *w).unwrap_or(([0.0; 3], [0.0; 3]))
}

/// `GfxVisualSprite2Type0::ProcessSprites`'s push curve of a kind at the height fraction `t` (kinds 1 to 6; 5 and 6 also move
/// the sprite on their own, which the callers do).
fn push_curve(kind: i32, t: f32) -> f32 {
    match kind {
        1 | 5 => 1.0,
        2 | 6 => t,
        3 => t * t,
        4 => (t * t + t) * 0.5,
        _ => 0.0,
    }
}

/// The height fraction of a push: `(height - low) / (high - low)` held to 0..1 (0 when the two heights are the same).
fn push_fraction(height: f32, (low, high): (f32, f32)) -> f32 {
    if high == low {
        0.0
    } else {
        ((height - low) / (high - low)).clamp(0.0, 1.0)
    }
}

/// `_GfxControlTracer4_t`'s three cords at `time` seconds, wound round the line from `from` to `to` (world, Z up) as FUN_100ffa7b
/// winds them: each of the twenty links of a cord drawn as a sprite of the link's width. The original's random flips and jitter
/// are stood in for by a hash of the link.
pub fn cords_particles(c: &Cords, from: [f32; 3], to: [f32; 3], time: f32) -> Vec<Particle> {
    let axis = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if length < 0.01 {
        return Vec::new();
    }
    let dir = axis.map(|v| v / length);
    // a frame across the line: a perpendicular and the third axis
    let seed = if dir[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let p1 = cross(dir, seed);
    let n1 = (p1[0] * p1[0] + p1[1] * p1[1] + p1[2] * p1[2]).sqrt().max(1e-6);
    let p1 = p1.map(|v| v / n1);
    let p2 = cross(dir, p1);
    let colour = [c.colour[1], c.colour[2], c.colour[3], c.colour[0]];
    let mut out = Vec::with_capacity(60);
    for cord in 0..3u32 {
        // (the three cords' phases: a third of a turn apart, spinning together)
        let mut phase = cord as f32 * std::f32::consts::TAU / 3.0 + 12.56 * time;
        for k in 0..20u32 {
            let t = k as f32 * 0.05;
            let salt = cord * 1013 + k * 7919;
            let r = ((t * std::f32::consts::PI * 20.0 / 19.0).sin() + 0.2) * c.radius;
            let along = c.radius * 0.0 + t * 1.1 * length;
            let (ca, sa) = (phase.cos(), phase.sin());
            let mut p = [0.0f32; 3];
            for i in 0..3 {
                p[i] = from[i] + dir[i] * along + p1[i] * ca * r + p2[i] * sa * r;
            }
            if hash(salt, 3) < 0.25 {
                let j = r * 0.5;
                p[0] += (hash(salt, 4) * 2.0 - 1.0) * j;
                p[2] += (hash(salt, 5) * 2.0 - 1.0) * j;
            }
            out.push(sprite(c.material, 0.0, p, c.width, colour));
            // (the step's sign flips at random a quarter of the frames in the original; here per link)
            phase += if hash(salt, 6) < 0.25 { -0.314 } else { 0.314 };
        }
    }
    out
}

/// `_GfxControlDeformer_t`'s mode 1 on a body's skinned parts (its vertex callback FUN_100d776f): each vertex pushed along its
/// normal by `amplitude × sin²(phase) × strength`, the phase running with the frequency over time and along the body, so the
/// mesh ripples for the hit's duration. `age` is seconds since the hit; nothing once the deform is over.
pub fn deform_parts(parts: &mut [mao_formats::model::Part], d: &Deform, age: f32) {
    let strength = d.strength_at(age);
    if strength <= 0.0 {
        return;
    }
    for part in parts {
        for v in &mut part.vertices {
            // (the original's two phase constants on the vertex's place are not legible; the ripple runs up the body)
            let phase = d.frequency * age * std::f32::consts::TAU * 4.0 + v.position[1] * 6.0;
            let push = d.amplitude * phase.sin().powi(2) * strength;
            for c in 0..3 {
                v.position[c] += v.normal[c] * push;
            }
        }
    }
}

/// `_GfxControlSparks_t`'s burst at `time` seconds after the hit, thrown from `origin` (world, Z up): word 31 sparks at once and
/// up to fifteen more over the first frames the gate lets through (FUN_100f24c8), each flying as FUN_100f1bbe throws it and
/// `GfxVisualSprite2Type0::ProcessSprites` moves it (its velocity pulled down by the gravity, its colour, sizes and frame run
/// at their rates). The original's random numbers are stood in for by a hash of the spark's number.
pub fn sparks_particles(s: &Sparks, origin: [f32; 3], time: f32) -> Vec<Particle> {
    let total = (s.count.max(0) + 15) as u32;
    let mut out = Vec::with_capacity(total as usize);
    // (the frames of the burst: the gate's mask thins them; a frame is a thirtieth)
    let frame = 1.0 / 30.0 * (s.gate.max(0) as f32 + 1.0);
    for k in 0..total {
        let salt = k.wrapping_mul(6353);
        let lerp = |(a, b): (f32, f32), r: f32| a + (b - a) * r;
        let born = if (k as i32) < s.count { 0.0 } else { (k as i32 - s.count) as f32 / 4.0 * frame };
        let age = time - born;
        let life = lerp(s.life, hash(salt, 1));
        if age < 0.0 || age > life {
            continue;
        }
        let t = age / life;
        let (az, el, speed) = (lerp(s.azimuth, hash(salt, 2)), lerp(s.elevation, hash(salt, 3)), lerp(s.speed, hash(salt, 4)) * s.scale);
        let v = [el.cos() * az.sin() * speed, el.cos() * az.cos() * speed, el.sin() * speed];
        let mut position = [origin[0] + v[0] * age, origin[1] + v[1] * age, origin[2] + v[2] * age + 0.5 * s.gravity * age * age];
        if s.push != 0 {
            // (the push of word 38: the wind times its curve of the spark's height above its birth, times its strength, added to its
            // place every frame; summed here over the spark's age in steps)
            let strength = (0.1 + 0.9 * hash(salt, 12)) * s.push_strength;
            let (now, _) = wind();
            let steps = 12;
            let mut weight = 0.0;
            for i in 0..steps {
                let tau = (i as f32 + 0.5) / steps as f32 * age;
                let height = v[2] * tau + 0.5 * s.gravity * tau * tau;
                weight += push_curve(s.push, push_fraction(height, s.push_heights)) * age / steps as f32;
            }
            for i in 0..3 {
                position[i] += now[i] * strength * weight;
            }
        }
        let colour = [1, 2, 3, 0].map(|c| lerp((s.colour_a[c], s.colour_b[c]), t));
        let size = lerp(s.width, t).max(lerp(s.height, t));
        out.push(sprite(s.material, t, position, size.max(0.0), [colour[0], colour[1], colour[2], colour[3]]));
    }
    out
}

/// `_GfxControlStars_t`'s stars at `time` seconds into the effect, about `centre` (world, Z up: the attach point's place), as the
/// run function FUN_100f8491 moves them in modes 6 and 19 (see `mao_formats::gfx::Stars`). The original's `rand()` is stood in
/// for by a hash of the star's number and its generation, so a star keeps its colour and path from frame to frame.
pub fn stars_particles(s: &Stars, centre: [f32; 3], time: f32, stopped: bool) -> Vec<Particle> {
    const STARS: u32 = 128;
    let mut out = Vec::new();
    // The ring modes: every star alive on a ring that swells word 31 times over the duration.
    if s.mode == 7 || s.mode == 8 {
        let duration = if s.duration > 0.0 { s.duration } else { 4.0 };
        let frac = (s.swirl * time / duration).fract().max(0.0);
        let reach = frac.sqrt() * 2.0 * s.word30 as f32;
        let size = s.size * frac.sqrt() + s.radius;
        for k in 0..STARS {
            let a = k as f32 * std::f32::consts::TAU / STARS as f32;
            let salt = k.wrapping_mul(7919);
            let colour = [1, 2, 3, 0].map(|c| s.colour_a[c] + (s.colour_b[c] - s.colour_a[c]) * hash(salt, c as u32));
            out.push(sprite(s.material, 0.0, [centre[0] + a.cos() * reach, centre[1] + a.sin() * reach, centre[2]], size.max(0.0), [colour[0], colour[1], colour[2], colour[3]]));
        }
        return out;
    }
    if s.mode == 22 {
        return stars_on_edges(s, centre, time, stopped);
    }
    if s.mode == 13 {
        return stars_ray(s, centre, time);
    }
    if s.mode == 1 {
        return stars_collapse(s, centre, time, stopped);
    }
    if s.mode == 9 {
        return stars_falling(s, centre, time, stopped);
    }
    if s.mode == 15 {
        // (no body known: a stand-in ring of eight points round the attach point, their normals outward)
        let turns = (0..8).map(|k| k as f32 * std::f32::consts::FRAC_PI_4);
        let points: Vec<[f32; 3]> = turns.clone().map(|a| [centre[0] + 0.25 * a.cos(), centre[1] + 0.25 * a.sin(), centre[2]]).collect();
        let normals: Vec<[f32; 3]> = turns.map(|a| [a.cos(), a.sin(), 0.0]).collect();
        return stars_off_body(s, &Body { points: &points, normals: &normals }, time);
    }
    // Mode 2 (`case 2`): all 128 stars from the start, each at a random point of the unit sphere, for 1.5 s; the sphere grows
    // to `√frac × 0.7 + 0.6`, the stars shrink by `(1 − frac) × word 28 × 20 / table[frame]`, their sheet frame
    // `(1 − frac) × 15` clamped to 1..15 (the odd stars one behind) and their colours random between the template's second
    // colour and a palette of eight (0x102c5ee8: pink, yellow, magenta, cyan, blue, green, red, mint), one per star in turn.
    if s.mode == 2 {
        const DURATION: f32 = 1.5;
        const DIVISORS: [f32; 16] = [10.0, 6.0, 7.0, 7.0, 9.0, 11.0, 16.0, 19.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0];
        const PALETTE: [[f32; 3]; 8] = [[1.0, 0.7, 0.7], [1.0, 1.0, 0.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.7, 1.0, 0.7]];
        if time < 0.0 || time >= DURATION {
            return out;
        }
        let frac = time / DURATION;
        let reach = frac.sqrt() * 0.7 + 0.6;
        let frame = (((1.0 - frac) * 15.0) as i32).clamp(1, 15);
        let size = (1.0 - frac) * s.size * 20.0 / DIVISORS[frame as usize];
        for k in 0..STARS {
            let salt = k.wrapping_mul(7919);
            let (a, b, r) = (hash(salt, 5) * std::f32::consts::TAU, hash(salt, 6) * 2.0 - 1.0, hash(salt, 7).cbrt());
            let flat = (1.0 - b * b).sqrt();
            let p = [flat * a.cos() * r, flat * a.sin() * r, b * r];
            let own = PALETTE[k as usize & 7];
            let colour = [1, 2, 3].map(|c| own[c - 1] + (s.colour_b[c] - own[c - 1]) * hash(salt, c as u32));
            let alpha = s.colour_a[0] + (s.colour_b[0] - s.colour_a[0]) * hash(salt, 0);
            let star_frame = (frame - (k as i32 & 1)).max(0) as f32 / 15.0;
            out.push(sprite(s.material, star_frame, [centre[0] + p[0] * reach, centre[1] + p[1] * reach, centre[2] + p[2] * reach], size.max(0.0), [colour[0], colour[1], colour[2], alpha]));
        }
        return out;
    }
    let spawn_per_frame: f32 = match s.mode {
        16 | 17 | 19 | 20 => 15.0,
        18 => 10.0,
        10 => 5.0,
        _ => 2.0,
    };
    let s = &Stars { life: if s.mode == 10 { s.size.max(0.1) } else { s.life }, ..s.clone() };
    // (the original bears that many a frame into its 128 slots while a slot is free: at 30 frames a second that is a birth every
    // `1 / (rate × 30)` seconds, so each slot is reborn every `128 / (rate × 30)` seconds, or as soon as it dies when that is
    // shorter than a life; `rate × 30 × life` are alive at most)
    let period = s.life.max(STARS as f32 / (spawn_per_frame * 30.0));
    for k in 0..STARS {
        // the star's births: every `period` seconds from its own stagger, each living `life` (mode 19: a random share)
        let stagger = k as f32 / STARS as f32 * period;
        if time < stagger {
            continue;
        }
        let generation = ((time - stagger) / period).floor();
        let born = stagger + generation * period;
        let age = time - born;
        let salt = k.wrapping_mul(7919).wrapping_add(generation as u32 * 104729);
        let life = if s.mode == 19 { s.life * (0.5 + 0.5 * hash(salt, 11)) } else { s.life };
        // (once stopped no star is born again: the first generation plays out)
        if age > life || (stopped && generation > 0.0) {
            continue;
        }
        let t = (age / life).clamp(0.0, 1.0);
        let mut colour = [1, 2, 3, 0].map(|c| s.colour_a[c] + (s.colour_b[c] - s.colour_a[c]) * hash(salt, c as u32));
        if s.mode == 10 {
            // a random alpha, and (word 30) one channel of the colour per star
            colour[3] = hash(salt, 13);
            if s.word30 != 0 {
                let keep = k % 3;
                for (c, v) in colour.iter_mut().enumerate().take(3) {
                    if c != keep as usize {
                        *v = 0.0;
                    }
                }
            }
        }
        let (size, position) = match s.mode {
            6 | 11 => {
                let angle = hash(salt, 5) * std::f32::consts::TAU;
                let (sx, sy) = (angle.cos() * s.radius, angle.sin() * s.radius);
                // the velocity: across the radius, half a unit up, swirled on by word 31 over the star's life
                let swirl = s.swirl / 100.0;
                let (vx, vy) = (-sy, sx);
                let x = centre[0] + sx + vx * age + vx * swirl * age * age;
                let y = centre[1] + sy + vy * age + vy * swirl * age * age;
                let z = centre[2] + 0.5 * age;
                ((t + 0.2) * (1.0 - t * t) * s.size, [x, y, z])
            }
            5 => {
                // born on the circle 1.2 below the point, flying outward at the radius a second and pulled back toward the axis by
                // word 31 hundredths of its place a frame (`case 5`: an oscillation along the radius), the size of mode 6
                let angle = hash(salt, 5) * std::f32::consts::TAU;
                let mut p = [angle.cos() * s.radius, angle.sin() * s.radius];
                let mut v = p;
                let pull = s.swirl / 100.0;
                let frames = (age * 30.0).floor().min(600.0) as u32;
                for _ in 0..frames {
                    v[0] -= p[0] * pull;
                    v[1] -= p[1] * pull;
                    p[0] += v[0] / 30.0;
                    p[1] += v[1] / 30.0;
                }
                ((t + 0.2) * (1.0 - t * t) * s.size, [centre[0] + p[0], centre[1] + p[1], centre[2] - 1.2])
            }
            3 => {
                // born in the sphere, swirling across the radius without rising, the swirl gaining a tenth a frame
                let (a, b, r) = (hash(salt, 5) * std::f32::consts::TAU, hash(salt, 6) * 2.0 - 1.0, hash(salt, 7).cbrt() * s.radius);
                let flat = (1.0 - b * b).sqrt();
                let (sx, sy, sz) = (flat * a.cos() * r, flat * a.sin() * r, b * r);
                let gain = 1.0 + 0.1 * age * 30.0 * 0.1;
                let position = [centre[0] + sx - sy * age * gain, centre[1] + sy + sx * age * gain, centre[2] + sz];
                ((t + 0.2) * (1.0 - t * t) * s.size, position)
            }
            4 => {
                // thrown from the point in a random direction at the radius a second, slowed 5% a frame, falling a tenth a frame
                let (a, b) = (hash(salt, 5) * std::f32::consts::TAU, hash(salt, 6) * 2.0 - 1.0);
                let flat = (1.0 - b * b).sqrt();
                let v = [flat * a.cos() * s.radius, flat * a.sin() * s.radius, b * s.radius];
                // (the frames at 30 a second: the slowed velocity's integral and the fall's)
                let frames = age * 30.0;
                let travelled = (1.0 - 0.95f32.powf(frames)) / (1.0 - 0.95) / 30.0;
                let fall = 0.1 * frames * (frames + 1.0) * 0.5 / 30.0;
                let position = [centre[0] + v[0] * travelled, centre[1] + v[1] * travelled, centre[2] + v[2] * travelled - fall];
                ((2.0 - t) * s.size, position)
            }
            17 => {
                // `case 0x11`: on the hit location's segment from its point to its second point (`d`, which is the 0.001 the
                // original adds to a zero one along x, the two points being the same), at a random fraction `f` along it and
                // `2f(1 - f)` out from it on the circle about it (FindPerpendicular's `u` and `w = u x d`: in the original's
                // Y-up frame (0, -0.707, -0.707) and (0, -0.707, 0.707)) at the angle `2f|d| + frac * 4 + word 29`, `frac` the
                // birth time over the duration (-1: the angle runs back at 4 rad/s): a spoke 0.5 long turning about x, which the
                // three templates' phases (0, 2.0, 4.1) make a pinwheel of; standing, sized `word 28 * (1 - (1 - 2a)^2)`.
                let f = hash(salt, 5);
                let frac = if s.duration > 0.0 { born / s.duration } else { -born };
                let angle = 2.0 * f * 0.001 + frac * 4.0 + s.radius;
                let reach = 2.0 * f * (1.0 - f);
                let (c, sn) = (angle.cos(), angle.sin());
                let h = std::f32::consts::FRAC_1_SQRT_2;
                // (Z up: the original's (x, y, z) is the client's (x, z, y))
                let (u, w) = ([0.0, -h, -h], [0.0, h, -h]);
                let ring = [0, 1, 2].map(|i| (w[i] * c + u[i] * sn) * reach);
                let w2 = 2.0 * t - 1.0;
                ((1.0 - w2 * w2) * s.size, [centre[0] + 0.001 * f + ring[0], centre[1] + ring[1], centre[2] + ring[2]])
            }
            20 => {
                // `case 0x14`: fifteen a frame born in the sphere about the hit location's point, standing, at the constant size;
                // (the second point the original slides toward over the duration is the same point, so there is no slide)
                let (a, b, r) = (hash(salt, 5) * std::f32::consts::TAU, hash(salt, 6) * 2.0 - 1.0, hash(salt, 7).cbrt() * s.radius);
                let flat = (1.0 - b * b).sqrt();
                (s.size, [centre[0] + flat * a.cos() * r, centre[1] + flat * a.sin() * r, centre[2] + b * r])
            }
            10 => {
                // standing on the sphere, swelling and shrinking
                let (a, b) = (hash(salt, 5) * std::f32::consts::TAU, hash(salt, 6) * 2.0 - 1.0);
                let flat = (1.0 - b * b).sqrt();
                let position = [centre[0] + flat * a.cos() * s.radius, centre[1] + flat * a.sin() * s.radius, centre[2] + b * s.radius];
                let w = 2.0 * t - 1.0;
                ((1.0 - w * w) * 2.0 * s.swirl + s.word11, position)
            }
            _ => {
                let (a, b, r) = (hash(salt, 5) * std::f32::consts::TAU, hash(salt, 6) * 2.0 - 1.0, hash(salt, 7).cbrt() * s.radius);
                let flat = (1.0 - b * b).sqrt();
                let position = [centre[0] + flat * a.cos() * r, centre[1] + flat * a.sin() * r, centre[2] + b * r];
                let w = 1.0 - 2.0 * t;
                ((1.0 - w * w) * s.size, position)
            }
        };
        let frame = match s.mode {
            4 => hash(salt, 9),
            10 => 0.0,
            // (the frame `15 - floor(a * 15.99)` of modes 16, 17 and 18, a the age over the life)
            16..=18 => (15 - ((t * 15.99) as i32).min(15)) as f32 / 15.0,
            _ => 1.0 - hash(salt, 9) * 0.25,
        };
        out.push(sprite(s.material, frame, position, size.max(0.0), [colour[0], colour[1], colour[2], colour[3]]));
    }
    out
}

/// Stars mode 9 (`case 9`): only every other slot, 64 stars, two a frame born on a random point of a level disc of the radius
/// (word 29) about the point, living word 30 ms. With `h = 0.5 - age * word 28` (0x10156f00) a star stands `h + 1.5`
/// (0x10161968) above its birth place, so it falls from 2 m at word 28 metres a second, and its size is word 11 plus, while `h`
/// is above 0, `h * word 31 / 100` (0x10156ec0): a large glow at the top shrinking to a small drop as it falls.
fn stars_falling(s: &Stars, centre: [f32; 3], time: f32, stopped: bool) -> Vec<Particle> {
    const STARS: u32 = 64;
    let mut out = Vec::new();
    let life = s.life.max(0.05);
    let period = life.max(STARS as f32 / (2.0f32 * 30.0)); // (each slot reborn every 128 / (rate × 30) s, or at its death)
    for k in 0..STARS {
        let stagger = k as f32 / STARS as f32 * period;
        if time < stagger {
            continue;
        }
        let generation = ((time - stagger) / period).floor();
        let age = time - stagger - generation * period;
        if age > life || (stopped && generation > 0.0) {
            continue;
        }
        let salt = k.wrapping_mul(7919).wrapping_add(generation as u32 * 104729);
        let (a, r) = (hash(salt, 5) * std::f32::consts::TAU, hash(salt, 6).sqrt() * s.radius);
        let h = 0.5 - age * s.size;
        let size = s.word11 + if h > 0.0 { h * s.swirl * 0.01 } else { 0.0 };
        let colour = [1, 2, 3, 0].map(|c| s.colour_a[c] + (s.colour_b[c] - s.colour_a[c]) * hash(salt, c as u32));
        out.push(sprite(s.material, 0.0, [centre[0] + a.cos() * r, centre[1] + a.sin() * r, centre[2] + h + 1.5], size.max(0.0), colour));
    }
    out
}

/// Stars mode 1 (`case 1`): two a frame born at rest on a random point of the unit sphere about the point, each living 1.2 s
/// (0x1016de58) and pulled back by a damped spring: every frame the acceleration is `(point - p) - 2 v` (0x10159794), the
/// velocity gains a tenth of it and the star moves a tenth of its velocity (0x10161850). With `frac` its age over its life, its
/// frame is `15 - floor(frac * 15.99)` and its size `frac * 0.01 * table[frame]` (the table at 0x102c5f68: the templates' size
/// word is 0), its colour random between the two.
fn stars_collapse(s: &Stars, centre: [f32; 3], time: f32, stopped: bool) -> Vec<Particle> {
    const STARS: u32 = 128;
    const LIFE: f32 = 1.2;
    const TABLE: [f32; 16] = [8.0, 8.0, 8.0, 8.0, 8.0, 12.0, 16.0, 19.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0];
    let mut out = Vec::new();
    let period = LIFE.max(STARS as f32 / (2.0f32 * 30.0)); // (each slot reborn every 128 / (rate × 30) s, or at its death)
    for k in 0..STARS {
        let stagger = k as f32 / STARS as f32 * period;
        if time < stagger {
            continue;
        }
        let generation = ((time - stagger) / period).floor();
        let age = time - stagger - generation * period;
        if age > LIFE || (stopped && generation > 0.0) {
            continue;
        }
        let salt = k.wrapping_mul(7919).wrapping_add(generation as u32 * 104729);
        let (a, b) = (hash(salt, 5) * std::f32::consts::TAU, hash(salt, 6) * 2.0 - 1.0);
        let flat = (1.0 - b * b).sqrt();
        let start = [flat * a.cos(), flat * a.sin(), b];
        // (the spring along the star's own line to the point: its distance from 1 at rest)
        let (mut x, mut v) = (1.0f32, 0.0f32);
        for _ in 0..(age * 30.0).floor().min(60.0) as u32 {
            let accel = -x - 2.0 * v;
            v += 0.1 * accel;
            x += 0.1 * v;
        }
        let frac = (age / LIFE).clamp(0.0, 1.0);
        let frame = 15 - ((frac * 15.99) as usize).min(15);
        let size = frac * 0.01 * TABLE[frame];
        let colour = [1, 2, 3, 0].map(|c| s.colour_a[c] + (s.colour_b[c] - s.colour_a[c]) * hash(salt, c as u32));
        out.push(sprite(s.material, frame as f32 / 15.0, [centre[0] + start[0] * x, centre[1] + start[1] * x, centre[2] + start[2] * x], size, colour));
    }
    out
}

/// A cast effect to play: the caster's feet (world, Z up) and heading, the cast template, and how long the cast lasts (the
/// casting state machine deletes the effect when the cast ends: here the length of the cast animation the caster plays).
#[derive(Clone, Debug, PartialEq)]
pub struct CastShow {
    pub feet: [f32; 3],
    pub heading: f32,
    pub cast: Cast,
    pub length: f32,
}

/// The length of the player cast animations: at someone (`spell-dir`: 1.17 to 1.23 s) and at oneself (`spell-self`: 1.9 to 2 s).
pub const CAST_AT_OTHER: f32 = 1.2;
pub const CAST_AT_SELF: f32 = 1.95;

/// A flare's streaks `t` seconds into its run, emitted at `at` (world, Z up), stopped (no more particles) at `stop`, as
/// `_GfxControlFlare_t` makes them (FUN_100ddc6d: up to `rate × elapsed` particles, FUN_100dd2eb each one) and
/// `GfxVisualFlareType0` moves them: particle `k` born at `(k + 1) / rate` with a life from words 34–35 and a velocity from the
/// two angles and the speed; over its life its head moves `head` times that velocity and its tail `tail` times it (both spread
/// over the life), its width and colour run from start to end. The streak is drawn as sprites from head to tail. `colours`
/// replace the flare's own (a Spell1 sets them from its words 10–17).
pub fn flare_particles(f: &Flare, at: [f32; 3], t: f32, stop: f32, colours: Option<([f32; 4], [f32; 4])>) -> Vec<Particle> {
    let mut out = Vec::new();
    if f.rate <= 0.0 || t < 0.0 {
        return out;
    }
    let (ca, cb) = colours.unwrap_or((f.colour_a, f.colour_b));
    let lerp = |(a, b): (f32, f32), r: f32| a + (b - a) * r;
    let longest = f.life.0.max(f.life.1).max(0.05);
    let emitting = if f.duration >= 0.0 { (f.duration - longest).min(stop) } else { stop };
    let made = (f.rate * t.min(emitting)).floor().max(0.0) as u32;
    let first = ((t - longest).max(0.0) * f.rate).floor() as u32;
    for k in first..made.min(first + 400) {
        let born = (k + 1) as f32 / f.rate;
        let salt = k.wrapping_mul(1005);
        let life = lerp(f.life, hash(salt, 1)).max(0.02);
        let age = t - born;
        if age < 0.0 || age >= life {
            continue;
        }
        let u = age / life;
        let (az, el, speed) = (lerp(f.azimuth, hash(salt, 2)), lerp(f.elevation, hash(salt, 3)), lerp(f.speed, hash(salt, 4)));
        let v = [el.cos() * az.cos() * speed, el.cos() * az.sin() * speed, el.sin() * speed];
        let head = [0, 1, 2].map(|i| at[i] + v[i] * f.head * u);
        let tail = [0, 1, 2].map(|i| at[i] + v[i] * f.tail * u);
        let width = lerp(f.width, u).max(0.0);
        let c = [1, 2, 3, 0].map(|i| ca[i] + (cb[i] - ca[i]) * u);
        for s in 0..3 {
            let w = s as f32 / 2.0;
            let p = [0, 1, 2].map(|i| head[i] + (tail[i] - head[i]) * w);
            out.push(sprite(f.material, u, p, width, c));
        }
    }
    out
}

/// `_GfxControlSpell1_t`'s cast effect `t` seconds after it began (see `mao_formats::gfx::Cast`): phase 1, the two hand flares
/// (words 27 and 28) emitting from word 18 until word 19 and deleted (with their particles) after word 20; phase 2, between
/// words 21 and 22, the line of quads between the hands (FUN_100f3d22 / FUN_100f3205: from each hand to the middle, the
/// quads `0.75 − c` to 0.25 wide, then from the middle back, 0.25 to `0.25 + 0.75 f`, `ftol(|span| × length × 50)` of them
/// between 2 and 50, each jittered by up to 0.01 of the line, in the first colour). All of it ends when the cast does
/// (`length`: the casting state machine deletes the effect). Phases 3 and 4 (the projectile and the impact) are not played:
/// no program names a template that has them.
pub fn cast_particles(c: &CastShow, t: f32) -> Vec<Particle> {
    let mut out = Vec::new();
    let s = &c.cast;
    if t < 0.0 || t >= c.length {
        return out;
    }
    let hands = [attach_point(2001, 0.0, c.feet, c.heading), attach_point(2000, 0.0, c.feet, c.heading)];
    let [start, stop, end] = [s.times[0], s.times[1], s.times[2]];
    if t >= start && t <= end {
        for (flare, hand) in s.flares.iter().zip(hands) {
            if let Some(f) = flare {
                out.extend(flare_particles(f, hand, t - start, stop - start, Some((s.colour_a, s.colour_b))));
            }
        }
    }
    let (from, to) = (s.times[3], s.times[4]);
    if t >= from && t <= to && to > from {
        let length = to - from;
        let half = length * 0.5;
        let into = t - from;
        let frame = (t * 30.0) as u32;
        let colour = [s.colour_a[1], s.colour_a[2], s.colour_a[3], s.colour_a[0]];
        let mut line = |a: f32, size_a: f32, b: f32, size_b: f32| {
            let d = [0, 1, 2].map(|i| hands[1][i] - hands[0][i]);
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            let count = (((b - a) * len).abs() * 50.0) as i32;
            let count = if count < 0 { 2 } else { count.min(50) }.max(1);
            let steps = (count - 1).max(1) as f32;
            for k in 0..count {
                let u = k as f32 / steps;
                let frac = a + (b - a) * u + hash(k as u32 ^ frame.wrapping_mul(977), 0x1010) * 0.02 - 0.01;
                let p = [0, 1, 2].map(|i| hands[0][i] + d[i] * frac);
                out.push(sprite(s.material, 0.0, p, (size_a + (size_b - size_a) * u).max(0.0), colour));
            }
        };
        if into < half {
            let c = 0.5 * into / half;
            line(0.0, 0.75 - c, c, 0.25);
            line(1.0, 0.75 - c, 1.0 - c, 0.25);
        } else {
            let f = (into - half) / half;
            let size = f * 0.75 + 0.25;
            line(0.5 * f, 0.25, 0.5, size);
            line(1.0 - 0.5 * f, 0.25, 0.5, size);
        }
    }
    out
}

/// The colours of mode 0's suns (0x102c60a0, ARGB), their alpha held to 0xc0.
const SUN_COLOURS: [u32; 8] = [0xffffc0c0, 0xffffff00, 0xffff00ff, 0xff00ffff, 0xff0000ff, 0xff00ff00, 0xffff0000, 0xffc0ffc0];

/// `_GfxControlSuns_t`'s suns `time` seconds in, about `at` (world, Z up), as FUN_100fc425 places them (see
/// `mao_formats::gfx::Visual::Suns`). The quads' turn is not drawn (the client's billboards do not turn).
pub fn suns_particles(s: &Suns, at: [f32; 3], time: f32) -> Vec<Particle> {
    let mut out = Vec::new();
    if time < 0.0 || (s.duration > 0.0 && time >= s.duration) {
        return out;
    }
    match s.mode {
        0 => {
            let f = if s.duration > 0.0 { time / s.duration } else { 0.5 };
            let e = 2.0 * f - 1.0;
            let envelope = 1.0 - e.powi(4);
            for k in 0..32u32 {
                let c = SUN_COLOURS[(k & 7) as usize] & 0xc0ff_ffff;
                let channel = |shift: u32| ((c >> shift) & 0xff) as f32 / 255.0;
                let size = (k as f32 * s.step + s.size) * envelope;
                out.push(sprite(s.material, 0.0, [at[0], at[1], at[2] + 2.0], size.max(0.0), [channel(16), channel(8), channel(0), channel(24)]));
            }
        }
        4 => {
            // (three a frame into the 32 slots: each slot reborn every 32 / (3 × 30) s, or at its death)
            let life = s.life.max(0.02);
            let period = life.max(32.0 / (3.0 * 30.0));
            for k in 0..32u32 {
                let stagger = k as f32 / 32.0 * period;
                if time < stagger {
                    continue;
                }
                let generation = ((time - stagger) / period).floor();
                let age = time - stagger - generation * period;
                if age >= life {
                    continue;
                }
                let salt = k.wrapping_mul(2005).wrapping_add(generation as u32 * 7919);
                let u = age / life;
                let frame = (15 - ((u * s.step) as i32).min(15)).max(0) as f32 / 15.0;
                let colour = [1, 2, 3, 0].map(|c| s.colour_a[c] + (s.colour_b[c] - s.colour_a[c]) * hash(salt, c as u32));
                out.push(sprite(s.material, frame, at, s.size.max(0.0), colour));
            }
        }
        _ => {}
    }
    out
}

/// The waves of `GfxVisualPlasma` (its constructor, `gfxplasma.rs`): amplitudes, wavelengths (frequencies 6.28 over them) and
/// speeds.
const PLASMA_WAVES: [(f32, f32, f32); 4] = [(0.15, 0.2, -0.8), (0.3, 0.3, 1.0), (0.2, 0.4, 1.2), (0.1, 0.5, 0.7)];

/// `_GfxControlPlasma_t`'s band `time` seconds into it, from `at` (world, Z up), as FUN_100ec2e6 runs it and `GfxVisualPlasma`
/// (FUN_1001bbf2) draws it: 75 segments of 0.01 along x (the step between the hit location's two points is nothing, so the
/// draw's own 0.01), 0.2 across, all displaced by a tenth of `Σ amplitude × sin³((speed × time) × 6.28 / wavelength)` (the
/// distance along being 0 for every segment); its colour each frame a random pick between the two (FUN_10108663), added to the
/// scene; gone after its duration. The band lies along the world's x (the visual is placed, not turned) and is drawn as sprites
/// a tenth apart, displaced up (the original displaces it across the view).
fn plasma_particles(colour_a: [f32; 4], colour_b: [f32; 4], material: i32, duration: f32, at: [f32; 3], time: f32) -> Vec<Particle> {
    if time < 0.0 || (duration > 0.0 && time >= duration) {
        return Vec::new();
    }
    let sum: f32 = PLASMA_WAVES.iter().map(|&(amp, wave, speed)| amp * ((speed * time) * (6.28 / wave)).sin().powi(3)).sum();
    let frame = (time * 30.0) as u32;
    let pick = |c: usize| colour_a[c] + (colour_b[c] - colour_a[c]) * hash(frame, 0x2002 + c as u32);
    let colour = [pick(1), pick(2), pick(3), pick(0)];
    (0..8)
        .map(|k| {
            let along = k as f32 * 0.75 / 7.0;
            sprite(material, 0.0, [at[0] + along, at[1], at[2] + 0.1 * sum], 0.2, colour)
        })
        .collect()
}

/// `GfxVisualShield`'s shell `t` seconds into a `_GfxControlShield_t` (see `mao_formats::gfx::Shield`) over a character's whole
/// posed mesh (`surface`, world, Z up; `origin` its feet), as FUN_1001c94f builds its vertices: each pushed out along its normal,
/// its texture coordinates by the mapping, its colour the template's, its alpha the template's times the run's factor times
/// `sin²` of the fade (from a point or along an axis with flags 0x800 / 0x1000), shimmering with flag 0x2000. The model frame
/// (Y up, its origin the feet) the mapping, the point and the shimmer are taken in is the world's, unturned by the heading.
pub fn shield_shell(s: &Shield, surface: &crate::blobshadow::Surface, origin: [f32; 3], t: f32) -> Option<Shell> {
    let (time, factor) = s.time(t)?;
    let n = surface.positions.len().min(surface.normals.len()).min(surface.uvs.len());
    if n == 0 || surface.indices.is_empty() {
        return None;
    }
    let channel = |shift: u32| ((s.colour >> shift) & 0xff) as f32 / 255.0;
    let (rgb, base) = ([channel(16), channel(8), channel(0)], channel(24) * factor);
    let (b, d) = (s.speed.0 * time, s.speed.1 * time);
    let axis = {
        let l = (s.axis[0] * s.axis[0] + s.axis[1] * s.axis[1] + s.axis[2] * s.axis[2]).sqrt();
        if l > 0.0 { s.axis.map(|c| c / l) } else { s.axis }
    };
    let vertex = |i: usize| {
        let p = surface.positions[i];
        let nrm = surface.normals[i];
        let position = [0, 1, 2].map(|k| p[k] + nrm[k] * s.push);
        // (the model frame: x, y up, z)
        let m = [p[0] - origin[0], p[2] - origin[2], p[1] - origin[1]];
        let uv0 = surface.uvs[i];
        let uv = match s.mapping {
            1 => [m[0].atan2(m[2]) * s.scale.0 / std::f32::consts::TAU + b, m[1] * s.scale.1 + b],
            2 => [m[0] * s.scale.0 + b, m[1] * s.scale.1 + d],
            _ => [(uv0[0] + b) * s.scale.0, (uv0[1] + d) * s.scale.1],
        };
        let q = if s.flags & 0x800 != 0 {
            let dv = [0, 1, 2].map(|k| m[k] - s.point[k]);
            let x = if s.flags & 0x1000 != 0 { dv[0] * axis[0] + dv[1] * axis[1] + dv[2] * axis[2] } else { (dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2]).sqrt() };
            (s.fade * time - x * s.distance).clamp(0.0, s.most.max(0.0)).sin()
        } else {
            (s.fade * time).clamp(0.0, s.most.max(0.0)).sin()
        };
        let mut alpha = base * q * q;
        if s.flags & 0x2000 != 0 {
            let w = (10.0 * m[0] + 10.0 * m[1] + 10.0 * m[2] + 3.0 * time).sin();
            alpha *= w * w;
        }
        ShellVertex { position, uv, colour: [rgb[0], rgb[1], rgb[2], alpha] }
    };
    let vertices = surface.indices.iter().filter(|&&i| (i as usize) < n).map(|&i| vertex(i as usize)).collect();
    Some(Shell { sprite: s.material.max(0) as u16, additive: s.flags & 0x400 != 0, vertices })
}

/// A character's posed surface: world points (Z up) and their unit normals, from its blob-shadow caster (every fourth vertex).
#[derive(Clone, Copy, Debug)]
pub struct Body<'a> {
    pub points: &'a [[f32; 3]],
    pub normals: &'a [[f32; 3]],
}

/// Stars mode 15 (`case 0xf`, the start registering FUN_100f740b as the target's `CATRender_t` vertex callback): the callback
/// keeps 30 of the skinned vertices and their normals (every `total / 31` from a random start); up to ten stars a frame are
/// born at the next of them in turn, with the normal times word 29 as velocity (turned by the body's rotation); every frame the
/// velocity is scaled by 0.96 and the star moves by a tenth of it; it lives word 30 ms, sized `(1.1 - age / life) * word 28`,
/// its colour random between the two. Here the 30 samples are drawn from the caster's points at that stride from a start that
/// changes with the star, and a star rides its sample's current place (the original froze it at birth).
fn stars_off_body(s: &Stars, body: &Body, time: f32) -> Vec<Particle> {
    const STARS: u32 = 128;
    let mut out = Vec::new();
    let n = body.points.len().min(body.normals.len());
    if n == 0 || time < 0.0 {
        return out;
    }
    let stride = (n / 31).max(1);
    let life = s.life.max(0.05);
    let period = life.max(STARS as f32 / (10.0f32 * 30.0)); // (each slot reborn every 128 / (rate × 30) s, or at its death)
    for k in 0..STARS {
        let stagger = k as f32 / STARS as f32 * period;
        if time < stagger {
            continue;
        }
        let generation = ((time - stagger) / period).floor();
        let age = time - stagger - generation * period;
        if age > life {
            continue;
        }
        let salt = k.wrapping_mul(7919).wrapping_add(generation as u32 * 104729);
        let start = (hash(salt, 30) * stride as f32) as usize;
        let at = (start + (k as usize % 30) * stride).min(n - 1);
        let (p, normal) = (body.points[at], body.normals[at]);
        // (frames at 30 a second: the velocity scaled by 0.96 before each step of a tenth of it)
        let frames = age * 30.0;
        let travelled = 0.1 * s.radius * 0.96 * (1.0 - 0.96f32.powf(frames)) / (1.0 - 0.96);
        let position = [0, 1, 2].map(|i| p[i] + normal[i] * travelled);
        let t = (age / life).clamp(0.0, 1.0);
        let colour = [1, 2, 3, 0].map(|c| s.colour_a[c] + (s.colour_b[c] - s.colour_a[c]) * hash(salt, c as u32));
        out.push(sprite(s.material, 0.0, position, ((1.1 - t) * s.size).max(0.0), colour));
    }
    out
}

/// Stars mode 13 (`case 0xd`, its start helpers FUN_100f73cf and FUN_100f7ec9): a ray that curls as it grows. A unit direction
/// (up at first) is turned each step by word 29 radians about an axis (+z at first) that drifts by 0.15 of a random unit vector
/// and is made unit again; the last 128 directions are kept in a ring (the start fills it, each frame adds seven). Star `k`
/// stands on its ring slot's direction at `word 28 × age / 128` from the point, `age` how many steps ago the slot was written,
/// so the newest are at the point and the oldest at the tip; it shows while `age / 128 < 1 − (2 × frac − 1)²` (the ray grows
/// and shrinks back over the duration), at the size word 11. Every template carries a duration of −1, which would hide the
/// whole ray in the original's formula; the hit's window gives it (the client's landing time, `LANDING`).
fn stars_ray(s: &Stars, centre: [f32; 3], time: f32) -> Vec<Particle> {
    const RING: u32 = 128;
    let mut out = Vec::new();
    let duration = if s.duration > 0.0 { s.duration } else { LANDING };
    if time < 0.0 || time >= duration {
        return out;
    }
    let frac = time / duration;
    let envelope = 1.0 - (2.0 * frac - 1.0).powi(2);
    let norm = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if l > 0.0 { v.map(|c| c / l) } else { v }
    };
    // (the original's Y-up frame: up is y)
    let (mut dir, mut axis) = ([0.0f32, 1.0, 0.0], [0.0f32, 0.0, 1.0]);
    let steps = RING + (time * 30.0).floor() as u32 * 7;
    let mut ring = vec![[0.0f32; 3]; RING as usize];
    for n in 0..steps.min(RING * 40) {
        let (a, b) = (hash(n, 0x130) * std::f32::consts::TAU, hash(n, 0x131) * 2.0 - 1.0);
        let flat = (1.0 - b * b).sqrt();
        let random = [flat * a.cos(), flat * a.sin(), b];
        axis = norm([0, 1, 2].map(|i| axis[i] + 0.15 * random[i]));
        // (Rodrigues: the direction turned by word 29 about the axis)
        let (sn, cs) = s.radius.sin_cos();
        let dot = axis[0] * dir[0] + axis[1] * dir[1] + axis[2] * dir[2];
        let cross = [axis[1] * dir[2] - axis[2] * dir[1], axis[2] * dir[0] - axis[0] * dir[2], axis[0] * dir[1] - axis[1] * dir[0]];
        dir = norm([0, 1, 2].map(|i| dir[i] * cs + cross[i] * sn + axis[i] * dot * (1.0 - cs)));
        ring[(n % RING) as usize] = dir;
    }
    let head = (steps.min(RING * 40) - 1) % RING;
    for k in 0..RING {
        let age = (head + RING - k) % RING;
        let fraction = age as f32 / RING as f32;
        if fraction >= envelope {
            continue;
        }
        let d = ring[k as usize];
        let reach = s.size * fraction;
        let salt = k.wrapping_mul(7919).wrapping_add(steps);
        let colour = [1, 2, 3, 0].map(|c| s.colour_a[c] + (s.colour_b[c] - s.colour_a[c]) * hash(salt, c as u32));
        out.push(sprite(s.material, 0.0, [centre[0] + d[0] * reach, centre[1] + d[2] * reach, centre[2] + d[1] * reach], s.word11.max(0.0), colour));
    }
    out
}

/// The attach points the body's six edges run between in Stars mode 22 (the table at 0x102c5fe8, in pairs): the calves to the
/// thighs, the pelvis to the head, the forearms to the upper arms, and the upper arms to each other.
const EDGE_POINTS: [(i32, i32); 6] = [(1013, 1011), (1014, 1012), (1000, 1006), (1009, 1007), (1010, 1008), (1007, 1008)];

/// Stars mode 22 (FUN_100f8491's `case 0x16`): each frame the six edges are measured between their attach points, and up to
/// word 31 stars are born on a random edge, a random way along it, out from it by word 28 (the field at +0x16d4, the other
/// modes' size) on a random side, at rest. Every frame after, a star's velocity gains a tenth of the way from it to the nearest
/// point of its edge and it moves by a tenth of that velocity (an undamped spring, so it swings through the edge); it dies when
/// within 0.04 of the edge or when its life (word 30) is spent. Its size is `(age / life × 0.7 + 0.3) × word 29` (+0x16d8, the
/// other modes' radius), its colour random between the two. The character's feet are at `feet` (world, Z up); its heading is
/// not known here, so the edges run across x.
fn stars_on_edges(s: &Stars, feet: [f32; 3], time: f32, stopped: bool) -> Vec<Particle> {
    const STARS: u32 = 128;
    let mut out = Vec::new();
    let point = |id: i32| {
        let (height, right, _) = anchor(id);
        [feet[0] + right, feet[1], feet[2] + height]
    };
    let life = s.life.max(0.05);
    let period = life.max(STARS as f32 / (s.swirl.max(1.0) * 30.0)); // (each slot reborn every 128 / (rate × 30) s, or at its death)
    for k in 0..STARS {
        let stagger = k as f32 / STARS as f32 * period;
        if time < stagger {
            continue;
        }
        let generation = ((time - stagger) / period).floor();
        let age = time - stagger - generation * period;
        if age > life || (stopped && generation > 0.0) {
            continue;
        }
        let salt = k.wrapping_mul(7919).wrapping_add(generation as u32 * 104729);
        let (a, b) = EDGE_POINTS[(hash(salt, 20) * 6.0).min(5.999) as usize];
        let (a, b) = (point(a), point(b));
        let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-3);
        let unit = dir.map(|c| c / len);
        // (a perpendicular pair: the unit crossed with whichever axis it is least along)
        let axis = if unit[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
        let u = [unit[1] * axis[2] - unit[2] * axis[1], unit[2] * axis[0] - unit[0] * axis[2], unit[0] * axis[1] - unit[1] * axis[0]];
        let ul = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt().max(1e-3);
        let u = u.map(|c| c / ul);
        let w = [unit[1] * u[2] - unit[2] * u[1], unit[2] * u[0] - unit[0] * u[2], unit[0] * u[1] - unit[1] * u[0]];
        let along = hash(salt, 21);
        let angle = hash(salt, 22) * std::f32::consts::TAU;
        let side = [0, 1, 2].map(|i| (u[i] * angle.cos() + w[i] * angle.sin()) * s.size);
        // the spring: at frame n the star is `cos(0.1 n)` of its start out from the edge, and dies as it crosses it
        let frames = age * 30.0;
        let swing = (0.1 * frames).cos();
        if (swing * s.size).abs() < 0.04 && frames > 1.0 {
            continue;
        }
        let position = [0, 1, 2].map(|i| a[i] + dir[i] * along + side[i] * swing);
        let t = (age / life).clamp(0.0, 1.0);
        let size = (t * 0.7 + 0.3) * s.radius;
        let colour = [1, 2, 3, 0].map(|c| s.colour_a[c] + (s.colour_b[c] - s.colour_a[c]) * hash(salt, c as u32));
        out.push(sprite(s.material, hash(salt, 9), position, size.max(0.0), [colour[0], colour[1], colour[2], colour[3]]));
    }
    out
}

/// The slices of a `GfxControlVolGrid_t` effect (kind 3038) `time` seconds after it began, its locator's point at `feet` (world,
/// Z up; the point itself, so the caller drops it to the ground for flag 0x2000). As FUN_10116292 runs it: the phase is the time's
/// fraction of the duration, the box's height, width and depth and the two colours are the curves at the phase, the whole turned
/// about the up axis by the spin times the time (from a random turn with flag 0x1000; here by `salt`); as displaysystem's
/// `GfxVisualVolGrid` (FUN_1002f86d) builds it: `across` slices across x at `k / across` of the width from its left edge, each a
/// fan from a centre half the height up, its bottom corners half the WIDTH out (the original's own asymmetry) in the middle colour,
/// its top corners half the depth out in the top colour, the texture's row `k / across` stretched across it; `deep` the same across
/// z; `up` square slices half the width out at `k / up` of the height, in the colours blended by that fraction, the whole texture
/// over each. The client's quads carry one colour each, so a standing slice is two quads, the lower at a quarter of the blend and
/// the upper at three quarters, each the trapezoid's width at its own middle; flag 0x4000 fades them by their facing on the card.
pub fn volgrid_particles(g: &VolGrid, feet: [f32; 3], time: f32, salt: u32) -> Vec<Particle> {
    let mut out = Vec::new();
    if g.duration <= 0.0 || time < 0.0 || time >= g.duration {
        return out;
    }
    let phase = (time % g.duration) / g.duration;
    let (h, w, d) = (curve(&g.height, phase), curve(&g.width, phase), curve(&g.depth, phase));
    let (middle, top) = (unpack_argb(colour_curve(&g.middle, phase)), unpack_argb(colour_curve(&g.top, phase)));
    let blend = |u: f32| [0, 1, 2, 3].map(|i| middle[i] + (top[i] - middle[i]) * u);
    let yaw = g.spin * time + if g.flags & 0x1000 != 0 { hash(salt, 0x3038) * std::f32::consts::TAU } else { 0.0 };
    // (the original's x and z axes in the client's Z-up world, turned by the spin)
    let ax = [yaw.cos(), yaw.sin(), 0.0];
    let az = [-yaw.sin(), yaw.cos(), 0.0];
    let scale = |v: [f32; 3], s: f32| [v[0] * s, v[1] * s, v[2] * s];
    let at = |a: f32, z: f32, b: f32| [feet[0] + ax[0] * a + az[0] * b, feet[1] + ax[1] * a + az[1] * b, feet[2] + z];
    let facing = g.flags & 0x4000 != 0;
    let (sprite, whole) = match mao_formats::gfx::material(g.material) {
        Some(_) => (g.material as u16, true),
        None => (NO_SPRITE, false),
    };
    let rect = |left: f32, top: f32, width: f32, height: f32| if whole { [left, top, width, height] } else { [0.0; 4] };
    let (hx, hz) = (w * 0.5, d * 0.5);
    for (family, count) in [(0, g.across), (1, g.deep)] {
        for k in 0..count {
            let f = k as f32 / count as f32;
            let p = f * w - hx;
            for (half, u) in [(0.25f32, 0.25f32), (0.75, 0.75)] {
                let reach = hx + (hz - hx) * half;
                let (centre, right) = if family == 0 { (at(p, h * half, 0.0), scale(az, reach)) } else { (at(0.0, h * half, p), scale(ax, reach)) };
                out.push(Particle::quad(centre, right, [0.0, 0.0, h * 0.25], blend(u), sprite, rect(0.0, f, 1.0, 0.0), facing));
            }
        }
    }
    for k in 0..g.up {
        let f = k as f32 / g.up as f32;
        out.push(Particle::quad(at(0.0, h * f, 0.0), scale(ax, hx), scale(az, hx), blend(f), sprite, rect(0.0, 0.0, 1.0, 1.0), facing));
    }
    out
}

/// `_GfxControlSmoke_t`'s stream (kind 1009) `time` seconds after it began, its attach point at `feet` (world, Z up), as
/// FUN_100f04bd makes the sprites and `GfxVisualSprite2Type0::ProcessSprites` moves them (see `mao_formats::gfx::Smoke` for the
/// words): sprite `k` is born at `k / rate` while the run is still emitting, at a random point of the emitter's disc at a random
/// height, and drifts by `(0.3 × x, 0.3 × z, −1) × drift / life` in the emitter's frame, which the template's turn (word 4 about x,
/// then 5 about y, 6 about z) and offset place at the attach point; its sizes, colour and frame run from start to end over its
/// life. Push kind 5 rises 1.5 a second on top, kind 7 flies at a random direction at the kind's speed; the other kinds ride the
/// effect handler's force, which the client does not keep (so they only drift). The original's random numbers are stood in for by
/// a hash of the sprite's number, so each keeps its path from frame to frame; the sprites' spin (the original's Type0 quads turn)
/// is not drawn, the client's billboards do not turn.
pub fn smoke_particles(s: &Smoke, feet: [f32; 3], time: f32) -> Vec<Particle> {
    let mut out = Vec::new();
    if s.rate <= 0.0 || s.life <= 0.0 || time < 0.0 {
        return out;
    }
    let lerp = |(a, b): (f32, f32), r: f32| a + (b - a) * r;
    // (the emitter's frame: the turn about each axis in order, applied to a vector of the original's Y-up frame)
    let turn = |v: [f32; 3]| {
        let (sx, cx) = s.turn[0].sin_cos();
        let v = [v[0], v[1] * cx - v[2] * sx, v[1] * sx + v[2] * cx];
        let (sy, cy) = s.turn[1].sin_cos();
        let v = [v[0] * cy + v[2] * sy, v[1], -v[0] * sy + v[2] * cy];
        let (sz, cz) = s.turn[2].sin_cos();
        [v[0] * cz - v[1] * sz, v[0] * sz + v[1] * cz, v[2]]
    };
    // (the original's Y-up frame to the client's Z-up world)
    let world = |v: [f32; 3]| [feet[0] + v[0], feet[1] + v[2], feet[2] + v[1]];
    // (only the sprites that can still be alive: the ones born within the longest life)
    let made = (s.rate * time).floor() as u32;
    let first = (((time - s.life * 1.3).max(0.0) * s.rate).floor() as u32 + 1).max(made.saturating_sub(600));
    for k in first..=made {
        let born = k as f32 / s.rate;
        if s.duration >= 0.0 && born >= s.duration - s.life {
            break;
        }
        let age = time - born;
        let salt = k.wrapping_mul(1009);
        let life = s.life * (1.0 + 0.3 * hash(salt, 1));
        if age < 0.0 || age > life {
            continue;
        }
        let t = age / life;
        let (az, el) = (hash(salt, 2) * std::f32::consts::TAU, hash(salt, 3) * std::f32::consts::FRAC_PI_2);
        let r = lerp(s.radius, hash(salt, 4)) * el.cos();
        let scale = 1.0 + 0.3 * hash(salt, 5);
        let y = lerp(s.rise, hash(salt, 6));
        let p = [az.cos() * r, y, az.sin() * r];
        let mut v = [0.3 * p[0] * s.drift / life, 0.3 * p[2] * s.drift / life, -s.drift / life];
        match s.kind {
            5 => v[1] += 1.5,
            7 => {
                // (a random direction: `2r − 1`, `0.89r + 0.1`, `2r − 1`, at the kind's speed)
                let d = [2.0 * hash(salt, 7) - 1.0, 0.89 * hash(salt, 8) + 0.1, 2.0 * hash(salt, 9) - 1.0];
                let speed = s.heights.1 + y;
                v = [d[0] * speed, d[1] * speed, d[2] * speed];
            }
            _ => {}
        }
        let (p, v) = (turn(p), turn(v));
        let mut local = [0, 1, 2].map(|i| p[i] + s.offset[i] + v[i] * age);
        if (1..=6).contains(&s.kind) {
            // (the push of word 29's kinds by the smoothed wind: its curve of the sprite's height between words 27 and 28 above its
            // birth height, times word 30's strength (0.1 + 0.9 random), added every frame; summed over the age in steps; in the
            // original's Y-up frame, so the wind is turned back to it)
            let strength = (0.1 + 0.9 * hash(salt, 10)) * s.strength;
            let (_, smooth) = wind();
            let smooth = [smooth[0], smooth[2], smooth[1]];
            let born = p[1] + s.offset[1];
            let steps = 12;
            let mut weight = 0.0;
            for i in 0..steps {
                let tau = (i as f32 + 0.5) / steps as f32 * age;
                let height = p[1] + s.offset[1] + v[1] * tau - born;
                weight += push_curve(s.kind, push_fraction(height, s.heights)) * age / steps as f32;
            }
            for i in 0..3 {
                local[i] += smooth[i] * strength * weight;
            }
        }
        let colour = [1, 2, 3, 0].map(|c| lerp((s.colour_a[c], s.colour_b[c]), t));
        let size = lerp(s.width, t).max(lerp(s.height, t)) * scale;
        out.push(sprite(s.material, t, world(local), size.max(0.0), colour));
    }
    out
}

/// `_GfxControlSprite_t`'s quad (kind 1012) `time` seconds after it began, its attach point at `feet` (world, Z up), as
/// FUN_100f6d2b runs it (see `mao_formats::gfx::Sprite` for the flags): nothing once its cycles are spent; the width, height,
/// colour and frame at the cycle's fraction; a billboard for a `GfxVisualSprite2`, and for a `GfxVisualSprite3` a quad standing
/// up at the point, turned to face the camera about the up axis (the original does so for flag 0x2000; for the other standing
/// templates the quad is set in the locator's frame, which the client does not carry here, so they face the camera too). The
/// random spot of the texture (flag 0x4 clear) moves with `salt` each frame.
pub fn sprite_particles(s: &Sprite, feet: [f32; 3], time: f32, salt: u32) -> Vec<Particle> {
    if time < 0.0 || s.duration >= 0.0 && time >= s.duration {
        return Vec::new();
    }
    let cycles = (time / s.period).floor();
    if s.flags & 0x40 != 0 && s.cycles >= 0 && cycles as i32 > s.cycles {
        return Vec::new();
    }
    let t = time / s.period - cycles;
    let lerp = |(a, b): (f32, f32), r: f32| a + (b - a) * r;
    let (mut width, mut height) = if s.flags & 0x100 != 0 { (lerp(s.width, t), lerp(s.height, t)) } else { (s.width.0, s.height.0) };
    if s.flags & 0x100 != 0 && s.flags & 0x800 != 0 {
        let swell = (time * std::f32::consts::TAU * s.pulse_rate).sin() * s.pulse;
        width += swell;
        height += swell;
    }
    let colour = if s.flags & 0x80 != 0 { [1, 2, 3, 0].map(|c| lerp((s.colour_a[c], s.colour_b[c]), t)) } else { [s.colour_a[1], s.colour_a[2], s.colour_a[3], s.colour_a[0]] };
    let frame = if s.flags & 0x20 != 0 { t } else { 0.0 };
    // (the original's Y-up offset in the client's Z-up world)
    let at = [feet[0] + s.offset[0], feet[1] + s.offset[2], feet[2] + s.offset[1]];
    let mut p = sprite(s.material, frame, at, width.max(0.0), colour);
    if s.flags & 0x4 == 0 && p.sprite != NO_SPRITE {
        // (a random spot of the texture, the whole of it wide: the sampler wraps)
        p.rect = [hash(salt, 0x1012), hash(salt, 0x1013), p.rect[2], p.rect[3]];
    }
    if s.standing() {
        p = Particle { size: width.max(0.0) * 0.5, right: [0.0; 3], up: [0.0, 0.0, height.max(0.0) * 0.5], ..p };
    }
    vec![p]
}

/// `GfxControlBParticle2_t`'s particles (kind 3028) `time` seconds after the effect began, thrown from `origin` (world, Z up), as
/// FUN_1010bf63 bears them and FUN_1010b499 moves them (see `mao_formats::gfx::BParticles` for the words): every particle is born
/// at once within the scatter of the point with its own velocity, spin, life and size; each frame its velocity is pulled by the
/// gravity and damped, its position moved, its size grown, its frame advanced and its colour read from the curve at its life's
/// fraction. In mode 1 a dead particle is reborn (the client gives each slot a chain of lives, hashed by its generation) while
/// the effect still emits. The original's random numbers are stood in for by a hash of the slot and generation. Not done: the
/// spin (the client's billboards do not turn), the ground bounce and the fades by height and daylight, the locator's turn.
pub fn bparticles(b: &BParticles, origin: [f32; 3], time: f32) -> Vec<Particle> {
    let mut out = Vec::new();
    if time < 0.0 || b.count <= 0 {
        return out;
    }
    let lerp = |(a, c): (f32, f32), r: f32| a + (c - a) * r;
    let longest = b.life.1.max(b.life.0).max(0.05);
    let emits_until = if b.duration < 0.0 { f32::MAX } else { b.duration - longest };
    for k in 0..b.count.min(400) as u32 {
        // (the slot's chain of lives: the first from the start, the next at each death while the effect emits)
        let mut born = 0.0;
        let mut generation = 0u32;
        let (age, life, salt) = loop {
            let salt = k.wrapping_mul(3028).wrapping_add(generation.wrapping_mul(0x9e37));
            let life = lerp(b.life, hash(salt, 1)).max(0.05);
            if time < born + life {
                break (time - born, life, salt);
            }
            if b.mode != 1 || born + life >= emits_until || generation > 400 {
                break (-1.0, life, salt);
            }
            born += life + b.interval.max(0.0);
            generation += 1;
        };
        if age < 0.0 {
            continue;
        }
        let t = age / life;
        let mut p = [0, 1, 2].map(|i| (2.0 * hash(salt, 2 + i as u32) - 1.0) * b.scatter);
        let mut v = [0, 1, 2].map(|i| lerp(b.velocity[i], hash(salt, 5 + i as u32)));
        if b.flags & 0x100000 == 0 {
            // (the velocity pulled along y by the gravity and damped by `(word 14 − 1) × 100` a second: integrated in steps)
            let steps = (age * 60.0).ceil().clamp(1.0, 240.0);
            let dt = age / steps;
            let damp = (b.damping - 1.0) * 100.0;
            for _ in 0..steps as u32 {
                for i in 0..3 {
                    p[i] += v[i] * dt;
                }
                v[1] += b.gravity * dt;
                for c in v.iter_mut() {
                    *c += *c * damp * dt;
                }
            }
        } else {
            p = [0.0; 3];
        }
        let size = lerp(b.size, hash(salt, 8)) * (1.0 + (b.growth - 1.0) * 100.0 * age).max(0.0);
        let colour = unpack_argb(colour_curve(&b.colours, t));
        // (the frame: from the first, or a random one with flag 0x20000, at the rate; wrapped, held or bounced by the mode)
        let frame = if b.frame_mode != 0 {
            let start = if b.flags & 0x20000 != 0 { hash(salt, 9) } else { 0.0 };
            let f = start + b.frame_rate * age;
            match b.frame_mode {
                1 => f.rem_euclid(1.0),
                3 => 1.0 - ((f.rem_euclid(2.0)) - 1.0).abs(),
                _ => f.min(1.0),
            }
        } else if b.flags & 0x20000 != 0 {
            hash(salt, 9)
        } else {
            0.0
        };
        out.push(sprite(b.material, frame, [origin[0] + p[0], origin[1] + p[2], origin[2] + p[1]], size.max(size / b.aspect), colour));
    }
    out
}

/// A particle cut from a material's sprite sheet (`mao_formats::gfx::MATERIALS`), `t` (0 to 1) of the way through the sheet's frames; a
/// material that is not known gives a soft dot.
fn sprite(material: i32, t: f32, position: [f32; 3], size: f32, colour: [f32; 4]) -> Particle {
    match mao_formats::gfx::material(material) {
        Some((_, cols, rows, first, last)) => {
            let frames = (last.saturating_sub(first)) as f32 + 1.0;
            let frame = first as u32 + (t.clamp(0.0, 0.999) * frames).floor() as u32;
            let (col, row) = (frame % cols.max(1) as u32, frame / cols.max(1) as u32);
            let (du, dv) = (1.0 / cols.max(1) as f32, 1.0 / rows.max(1) as f32);
            Particle { position, size, colour, sprite: material as u16, rect: [col as f32 * du, row as f32 * dv, du, dv], right: [0.0; 3], up: [0.0; 3] }
        }
        None => Particle::dot(position, size, colour),
    }
}

/// `Setupf/animcalibration.txt`: for a creature's mesh and one of its movement animations (both as the instance numbers of their
/// database objects) the factor the original plays the animation's speed with (the wolf walks at 0.7 and runs at 0.5, the sparrow's
/// flight is 1.2). Set once when the game is known.
static CALIBRATION: std::sync::OnceLock<std::collections::HashMap<(u32, u32), f32>> = std::sync::OnceLock::new();

/// Reads the lines `mesh animation factor` (tab separated; `#` starts a comment).
pub fn parse_calibration(text: &str) -> std::collections::HashMap<(u32, u32), f32> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| {
            let mut w = l.split_whitespace();
            Some(((w.next()?.parse().ok()?, w.next()?.parse().ok()?), w.next()?.parse().ok()?))
        })
        .collect()
}

/// Loads the calibration of the game in `game` (once).
pub fn load_calibration(game: &str) {
    let _ = CALIBRATION.set(std::fs::read_to_string(std::path::Path::new(game).join("Setupf").join("animcalibration.txt")).map(|t| parse_calibration(&t)).unwrap_or_default());
}

/// The playback factor of a creature's animation (1 when the file does not list it).
pub fn calibration(mesh: u32, animation: u32) -> f32 {
    CALIBRATION.get().and_then(|t| t.get(&(mesh, animation))).copied().unwrap_or(1.0)
}

/// The step of a placeholder's angle (0x10168fc8): the cord gets a link and the flare runs once each time the point has turned
/// this far.
const RING_STEP: f32 = 0.45;
/// The frame the placeholder's children run their clocks by: they run once a step, by the frame's step, so their clocks follow
/// the frame rate; the effects were made at 30 a second.
const RING_FRAME: f32 = 1.0 / 30.0;
/// A cord link's drift a run: the frame's step held to 0.01..0.025 (0x10161994, 0x1016c558): 0.025 at 30 a second.
const RING_DRIFT: f32 = 0.025;
/// The links a cord draws (`GfxVisualCord4`'s list).
const RING_LINKS: u32 = 32;

/// One placeholder's ring `t` seconds into its pulse, about `centre` (world, Z up) at `radius`, as the original makes it:
/// `_GfxControlBuffPlaceHolder_t`'s run (FUN_100d52c3) turns its point at word 23 rad/s and, for each 0.45 rad of turn
/// (`(target - angle) / 0.45` whole steps a frame: none at all when the spin is negative, the reason the left limb's ring of
/// every pair never shows), moves the angle on a step and hands the new point to its children: the cord (FUN_100d69d8 lays a
/// link there with FUN_100d6323 while its clock is under its duration less its life) and the flare (whose rate is 0 in every
/// template, so nothing). The cord's run (FUN_100d5f5a) moves each link by its offset from the ring's centre times the clamped
/// step (so the ring widens as it fades) and sets its alpha to `life left / life`; `GfxVisualCord4` draws a ribbon twice the
/// link width through the newest 32 links. The placeholder ends with its duration (word 8) and takes its children with it.
fn ring(out: &mut Vec<Particle>, centre: [f32; 3], radius: f32, helix: &Helix, colour: [f32; 3], t: f32) {
    if helix.spin <= 0.0 || t < 0.0 || t >= helix.life {
        return;
    }
    let now = (t * helix.spin / RING_STEP).floor() as u32;
    let (width, life) = (helix.ribbon_width.max(0.005), helix.ribbon_life.max(0.05));
    let mut links: Vec<([f32; 3], f32)> = Vec::new();
    for n in (1..=now).rev().take(RING_LINKS as usize) {
        // (the cord's clock when this link was laid, and the link's age, both in steps of a frame)
        let clock = n as f32 * RING_FRAME;
        if helix.ribbon_duration >= 0.0 && clock >= helix.ribbon_duration - life {
            continue;
        }
        let steps = now - n;
        let age = steps as f32 * RING_FRAME;
        if age >= life {
            break;
        }
        let angle = helix.phase + n as f32 * RING_STEP;
        let out_by = radius * (1.0 + RING_DRIFT * steps as f32);
        links.push(([centre[0] + out_by * angle.cos(), centre[1] + out_by * angle.sin(), centre[2]], (life - age) / life));
    }
    // (the ribbon: sprites along each segment, as wide as the ribbon, close enough to join)
    for pair in links.windows(2) {
        let ((a, fa), (b, fb)) = (pair[0], pair[1]);
        let length = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        let pieces = ((length / width).ceil() as u32).clamp(1, 8);
        for i in 0..pieces {
            let u = i as f32 / pieces as f32;
            let at = [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * u);
            let alpha = fa + (fb - fa) * u;
            out.push(sprite(helix.ribbon_material, 0.0, at, width * 2.0, [colour[0], colour[1], colour[2], alpha]));
        }
    }
    if let [(only, alpha)] = links.as_slice() {
        out.push(sprite(helix.ribbon_material, 0.0, *only, width * 2.0, [colour[0], colour[1], colour[2], *alpha]));
    }
}

/// A spell's flight and landing: the tracer's spiral travels from the caster to the target, then the hit effect's spiral stands at the
/// target (the effects are spirals of their own, tinted: see `Gfx::aura`).
#[derive(Clone, Debug, PartialEq)]
pub struct Shot {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub tracer: Option<Visual>,
    pub hit: Option<Visual>,
}

/// How long the tracer flies and how long the hit effect stays (seconds).
pub const FLIGHT: f32 = 0.45;
const LANDING: f32 = 1.4;

impl Shot {
    pub fn lasts(&self) -> f32 {
        FLIGHT + LANDING
    }

    /// The sparks of the shot `age` seconds after it was cast; `body` is the target's posed surface when it is known (the hits
    /// thrown from it, Stars mode 15, use it).
    pub fn particles(&self, age: f32, body: Option<Body>) -> Vec<Particle> {
        let mut out = Vec::new();
        let travelling = self.from != self.to;
        if let (Some(tracer), true) = (&self.tracer, travelling && age < FLIGHT) {
            match tracer {
                // (the cords span the whole line of flight for the tracer's duration)
                Visual::Cords(c) => out.extend(cords_particles(c, self.from, self.to, age)),
                _ => {
                    let t = age / FLIGHT;
                    let at = [0, 1, 2].map(|i| self.from[i] + (self.to[i] - self.from[i]) * t);
                    out.extend(visual_particles(tracer, [at[0], at[1], at[2] - 0.25], age, Reach::Small));
                }
            }
        }
        let landed = if travelling { age - FLIGHT } else { age };
        if let (Some(hit), true) = (&self.hit, landed >= 0.0 && landed < LANDING) {
            let fade = (((LANDING - landed) / 0.4).min(1.0)).max(0.0) * (landed / 0.1).min(1.0);
            let from_body = match (hit, &body) {
                (Visual::Stars(s), Some(b)) if s.mode == 15 => Some(stars_off_body(s, b, landed)),
                _ => None,
            };
            let parts = from_body.unwrap_or_else(|| visual_particles(hit, [self.to[0], self.to[1], self.to[2] - 1.0], landed, Reach::Medium));
            for mut p in parts {
                p.colour[3] *= fade;
                out.push(p);
            }
        }
        out
    }
}

/// How big a visual is drawn: a whole body's buff, a spell landing on a target, or a tracer in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reach {
    Body,
    Medium,
    Small,
}

/// The sparks of any visual at `time` seconds with its feet (or centre, for a ball and a flash) at `feet`.
pub fn visual_particles(v: &Visual, feet: [f32; 3], time: f32, reach: Reach) -> Vec<Particle> {
    match v {
        // (the attach point's height on a standing body; a tracer carries the point itself)
        Visual::Stars(s) => {
            // (mode 22 runs over the body's edges from the feet)
            let up = match reach {
                Reach::Small => 0.0,
                _ if s.mode == 22 => 0.0,
                _ => anchor(s.attach).0,
            };
            stars_particles(s, [feet[0], feet[1], feet[2] + up], time, false)
        }
        Visual::Sparks(s) => {
            let up = match reach {
                Reach::Small => 0.0,
                _ => anchor(s.attach).0,
            };
            sparks_particles(s, [feet[0], feet[1], feet[2] + up], time)
        }
        // (bends the target's mesh instead: see `deform_parts`)
        Visual::Deform(_) => Vec::new(),
        // (needs both ends of its line: `Shot::particles` draws it; alone, a short cord up from the point)
        Visual::Cords(c) => cords_particles(c, feet, [feet[0], feet[1], feet[2] + 1.0], time),
        // (the locator's point is the feet; the turn is drawn from the point so one effect keeps its turn)
        Visual::VolGrid(g) => volgrid_particles(g, feet, time, feet[0].to_bits() ^ feet[1].to_bits().rotate_left(13)),
        Visual::BParticles(b) => {
            let up = match reach {
                Reach::Small => 0.0,
                _ => anchor(b.attach).0,
            };
            bparticles(b, [feet[0], feet[1], feet[2] + up], time)
        }
        Visual::Sprite(s) => {
            let up = match reach {
                Reach::Small => 0.0,
                _ => anchor(s.attach).0,
            };
            // (the frame's random spot changes each frame: the time's bits are the salt)
            sprite_particles(s, [feet[0], feet[1], feet[2] + up], time, time.to_bits())
        }
        // (the attach point's height on a standing body; a tracer carries the point itself)
        Visual::Smoke(s) => {
            let up = match reach {
                Reach::Small => 0.0,
                _ => anchor(s.attach).0,
            };
            smoke_particles(s, [feet[0], feet[1], feet[2] + up], time)
        }
        // (the placeholders' rings at their attach points; a tracer carries them at its point)
        Visual::Spiral(aura) => {
            let mut out = Vec::new();
            let pulse = time.rem_euclid(aura.period.max(0.5));
            for helix in &aura.helices {
                let centre = match reach {
                    Reach::Small => feet,
                    _ => attach_point(helix.attach, helix.offset, feet, 0.0),
                };
                ring(&mut out, centre, helix.radius.max(0.03), helix, aura.colour, pulse);
            }
            out
        }
        // (drawn as a shell over the character's mesh: see `shield_shell`)
        Visual::Shield(_) => Vec::new(),
        Visual::Suns(s) => {
            let up = match reach {
                Reach::Small => 0.0,
                _ => anchor(s.attach).0,
            };
            suns_particles(s, [feet[0], feet[1], feet[2] + up], time)
        }
        // a quick bright burst: a core and a ring of sparks flying out
        Visual::Flash { colour, size } => {
            let t = (time / 0.5).clamp(0.0, 1.0);
            let mut out = vec![sprite(8, 0.0, [feet[0], feet[1], feet[2] + 1.0], size * (0.5 + t), [colour[0], colour[1], colour[2], 0.7 * (1.0 - t)])];
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0 + 0.4;
                out.push(sprite(15, 0.0, [feet[0] + a.cos() * t * 0.8, feet[1] + a.sin() * t * 0.8, feet[2] + 1.0 + (k % 3) as f32 * 0.12 * t], 0.12, [colour[0], colour[1], colour[2], 0.8 * (1.0 - t)]));
            }
            out
        }
        // (the point a tracer carries; see `plasma_particles`)
        Visual::Plasma { colour_a, colour_b, material, duration } => plasma_particles(*colour_a, *colour_b, *material, *duration, [feet[0], feet[1], feet[2] + 0.25], time),
    }
}

/// Where an attach point (`Helix::attach`) sits on a standing character: metres above the feet, metres to the character's right of
/// its middle, and the way its bone runs (+1 up toward the crown, -1 down toward the extremity, 0 none), along which the template's
/// offset is applied (a head ring with offset -0.18 climbs to the crown; a calf ring with -0.25 sits at mid-shin). Typical figures for
/// a standing body until the posed skeleton's bones are used.
fn anchor(attach: i32) -> (f32, f32, f32) {
    match attach {
        1000 => (0.98, 0.0, 0.0),
        1001 => (1.08, 0.0, 1.0),
        1002 => (1.18, 0.0, 1.0),
        1003 => (1.28, 0.0, 1.0),
        1004 => (1.40, 0.0, 1.0),
        1005 => (1.52, 0.0, 1.0),
        1006 | 2002 => (1.62, 0.0, 1.0),
        // The left limbs' bone frames are mirrored in the Biped rig, so the template's offset, which runs down the right limb, runs up
        // the left one. (The original shows these effects on the right limb only, seen on the thigh ring of a Martial Artist's buff;
        // the reason is the placeholder's run, not the frame: every left-limb placeholder spins backwards and so never steps, see
        // [`ring`].)
        1007 => (1.45, -0.2, 1.0),
        1008 => (1.45, 0.2, -1.0),
        1009 => (1.15, -0.22, 1.0),
        1010 => (1.15, 0.22, -1.0),
        1011 => (0.92, -0.1, 1.0),
        1012 => (0.92, 0.1, -1.0),
        1013 => (0.50, -0.1, 1.0),
        1014 => (0.50, 0.1, -1.0),
        1015 => (0.08, -0.1, 0.0),
        1016 => (0.08, 0.1, 0.0),
        1017 | 2001 => (0.90, -0.25, 0.0),
        1018 | 2000 => (0.90, 0.25, 0.0),
        2003 => (1.30, 0.0, 0.0),
        2004 => (1.50, -0.2, 0.0),
        2005 => (1.50, 0.2, 0.0),
        _ => (1.62, 0.0, 1.0),
    }
}

/// The world point of an attach point with the template's `offset`, for a character whose feet are at `feet` facing `heading`.
pub fn attach_point(attach: i32, offset: f32, feet: [f32; 3], heading: f32) -> [f32; 3] {
    let (height, right, dir) = anchor(attach);
    // (the character's right, in the viewer's Z-up world: x east, y the server's z)
    let (rx, ry) = (heading.cos(), -heading.sin());
    [feet[0] + rx * right, feet[1] + ry * right, feet[2] + height - dir * offset]
}
/// The sparks of one running effect `age` seconds into its pulse (from `pulses::Pulses::tick`), round a character whose feet are at `feet`
/// (world, Z up) facing `heading`, and whose body is `(breed, sex, fatness)` when known. `Visual::Spiral` is the kind-1001 pair of placeholders; a
/// shield is drawn as a shell (`shield_shell`).
pub fn pulse_particles(v: &Visual, feet: [f32; 3], heading: f32, age: f32, body: Option<(i32, i32, i32)>) -> Vec<Particle> {
    let mut out = Vec::new();
    match v {
        Visual::Spiral(aura) => {
            for helix in &aura.helices {
                if age >= helix.life {
                    continue;
                }
                let (offset, radius) = body.map_or((helix.offset, helix.radius), |(b, s, f)| helix.fit(b, s, f));
                let at = attach_point(helix.attach, offset, feet, heading);
                ring(&mut out, at, radius.max(0.03), helix, aura.colour, age);
            }
        }
        other => return visual_particles(other, feet, age, Reach::Body),
    }
    out
}

/// The period and life a visual pulses with, for the scheduler: the aura's own, the shield stand-in's.
pub fn pulse_timing(v: &Visual) -> (f32, f32, i32) {
    match v {
        Visual::Spiral(aura) => (aura.period, aura.helices.iter().map(|h| h.life).fold(0.0, f32::max).max(0.5), aura.plays),
        // (mode 2 runs the fixed 1.5 s its start function sets as the duration)
        Visual::Stars(s) => (
            if s.duration > 0.0 { s.duration } else { 4.0 },
            match s.mode {
                2 => 1.5,
                1 => 1.2 * 2.0,
                13 => if s.duration > 0.0 { s.duration } else { LANDING },
                _ => s.life * 2.0,
            },
            -1,
        ),
        Visual::Sparks(s) => (4.0, s.life.1 + 0.2, -1),
        Visual::Deform(d) => (4.0, if d.duration > 0.0 { d.duration } else { 1.5 }, -1),
        Visual::Cords(c) => (4.0, if c.duration > 0.0 { c.duration } else { 1.0 }, -1),
        Visual::VolGrid(g) => (if g.duration > 0.0 { g.duration } else { 3.0 }, if g.duration > 0.0 { g.duration } else { 3.0 }, -1),
        // (a burst lives its longest life; a stream runs its duration and that life, or for ever)
        Visual::BParticles(b) => {
            let longest = b.life.1.max(b.life.0) + b.interval.max(0.0);
            let long = if b.mode != 1 { longest } else if b.duration >= 0.0 { b.duration + longest } else { 600.0 };
            (long.max(0.5), long.max(0.5), -1)
        }
        // (a quad runs its cycles, or its duration, or for ever: restarted every ten minutes)
        Visual::Sprite(s) => {
            let long = if s.flags & 0x40 != 0 && s.cycles >= 0 { s.period * (s.cycles + 1) as f32 } else if s.duration >= 0.0 { s.duration } else { 600.0 };
            (long, long, -1)
        }
        // (a stream runs for its duration and its last sprites' lives; an endless one is restarted every ten minutes)
        Visual::Smoke(s) => {
            let long = if s.duration >= 0.0 { s.duration + s.life * 1.3 } else { 600.0 };
            (long, long, -1)
        }
        Visual::Suns(s) => {
            let long = if s.duration > 0.0 { s.duration } else { 600.0 };
            (long, long, -1)
        }
        // (a shield plays once for its duration; one without a duration runs while the effect does)
        Visual::Shield(s) => if s.duration > 0.0 { (600.0, s.duration, 1) } else { (600.0, 600.0, -1) },
        _ => (4.0, 1.5, -1),
    }
}


/// A cheap deterministic number in 0..1 for particle `k` and a purpose `salt` (so particles need no stored state).
fn hash(k: u32, salt: u32) -> f32 {
    let mut x = k.wrapping_mul(0x9e37_79b1) ^ salt.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297a_2d39);
    x ^= x >> 15;
    (x & 0xffff) as f32 / 65535.0
}

/// The particles of one emitter `age` seconds after its effect began, at `origin` (world, Z up). The emitter runs for `active`
/// seconds, making `rate` particles a second; each lives `life` seconds, flies out in a direction drawn from its angles and fades from
/// its start colour to its end colour while growing from its start size to its end size.
pub fn emitter_particles(e: &Emitter, origin: [f32; 3], age: f32, active: f32) -> Vec<Particle> {
    let mut out = Vec::new();
    let life = e.life.clamp(0.1, 6.0);
    let first = ((age - life).max(0.0) * e.rate).floor() as u32;
    let last = (age.min(active) * e.rate).floor() as u32;
    for k in first..=last.min(first + 400) {
        let born = k as f32 / e.rate.max(1.0);
        let a = age - born;
        if !(0.0..life).contains(&a) {
            continue;
        }
        let t = a / life;
        let azimuth = hash(k, 1) * e.azimuth.abs().max(0.001) * e.azimuth.signum();
        let (lo, hi) = (e.elevation.0.min(e.elevation.1), e.elevation.0.max(e.elevation.1));
        let elevation = lo + hash(k, 2) * (hi - lo);
        let speed = e.speed.0 + hash(k, 3) * (e.speed.1 - e.speed.0);
        let direction = [azimuth.cos() * elevation.cos(), azimuth.sin() * elevation.cos(), elevation.sin()];
        let mix = |a: f32, b: f32| a + (b - a) * t;
        // (a dense cloud of big sprites would add up to white: its strength drops with how much of a square metre it covers)
        let middle = 0.5 * (e.size.0 + e.size.1);
        let cover = e.rate * life * middle * middle;
        let share = (1.5 / cover.max(1.5)).min(1.0);
        let colour = [mix(e.start[1], e.end[1]), mix(e.start[2], e.end[2]), mix(e.start[3], e.end[3]), mix(e.start[0], e.end[0]) * share];
        // a little scatter round the origin so the particles of a still emitter do not sit on one point
        let jitter = |salt| (hash(k, salt) - 0.5) * 0.12;
        out.push(sprite(
            e.material,
            t,
            [
                origin[0] + direction[0] * speed * a + jitter(4),
                origin[1] + direction[1] * speed * a + jitter(5),
                origin[2] + direction[2] * speed * a + 0.5 * e.gravity * a * a + jitter(6),
            ],
            mix(e.size.0, e.size.1).max(0.04),
            colour,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stars_mode_17_is_a_turning_spoke_and_modes_16_and_18_stand_in_a_sphere() {
        // (templates 11100 and 11101: phases 0 and 2.0, 0.2 wide, 300 ms)
        let spoke = |phase: f32| Stars { mode: 17, colour_a: [1.0, 0.3, 1.0, 0.3], colour_b: [1.0, 0.3, 1.0, 0.3], size: 0.2, radius: phase, life: 0.3, swirl: 0.0, material: 33, attach: 1000, duration: -1.0, word11: 0.4, word30: 300 };
        let centre = [0.0, 0.0, 0.0];
        let at = stars_particles(&spoke(0.0), centre, 0.2, false);
        assert!(at.len() > 60, "{}", at.len());
        // within half a metre, in the plane across x, sized no more than word 28
        assert!(at.iter().all(|p| p.position[0].abs() < 0.002 && p.position[1].hypot(p.position[2]) <= 0.5 + 1e-4 && p.size <= 0.2 + 1e-4));
        // the stars born together lie on one line through the point: their directions agree (or are opposite)
        let dir = |p: &Particle| p.position[2].atan2(p.position[1]);
        let late: Vec<_> = at.iter().filter(|p| p.position[1].hypot(p.position[2]) > 0.2).collect();
        assert!(!late.is_empty());
        // another phase turns the spoke away
        let other = stars_particles(&spoke(2.0), centre, 0.2, false);
        let a: f32 = late.iter().map(|p| dir(p).sin()).sum::<f32>() / late.len() as f32;
        let b: Vec<_> = other.iter().filter(|p| p.position[1].hypot(p.position[2]) > 0.2).collect();
        let bb: f32 = b.iter().map(|p| dir(p).sin()).sum::<f32>() / b.len().max(1) as f32;
        assert!((a - bb).abs() > 0.1, "{a} {bb}");
        // modes 16 and 18: standing in a sphere of word 29
        for mode in [16, 18] {
            let s = Stars { mode, radius: 0.4, ..spoke(0.0) };
            let at = stars_particles(&s, centre, 0.2, false);
            assert!(!at.is_empty() && at.iter().all(|p| (p.position[0].powi(2) + p.position[1].powi(2) + p.position[2].powi(2)).sqrt() <= 0.4 + 1e-4));
        }
        assert!(stars_particles(&Stars { mode: 18, radius: 0.4, ..spoke(0.0) }, centre, 0.2, false).len() < stars_particles(&Stars { mode: 16, radius: 0.4, ..spoke(0.0) }, centre, 0.2, false).len());
    }

    #[test]
    fn the_wind_pushes_the_sparks_of_kind_4_by_their_height() {
        let s = Sparks { colour_a: [1.0; 4], colour_b: [1.0; 4], width: (0.1, 0.1), height: (0.1, 0.1), azimuth: (0.0, 6.28), elevation: (1.2, 1.4), speed: (4.0, 5.0), scale: 1.0, life: (1.0, 1.0), count: 8, gravity: -9.8, material: 16, attach: 1000, gate: 0, push: 4, push_heights: (0.0, 8.0), push_strength: 1.0 };
        set_wind([0.0; 3], [0.0; 3]);
        let still = sparks_particles(&s, [0.0; 3], 0.6);
        set_wind([20.0, 0.0, 0.0], [0.0; 3]);
        let blown = sparks_particles(&s, [0.0; 3], 0.6);
        set_wind([0.0; 3], [0.0; 3]);
        assert_eq!(still.len(), blown.len());
        // every spark carried downwind (+x), none across or up by it
        for (a, b) in still.iter().zip(&blown) {
            assert!(b.position[0] > a.position[0] && (b.position[1] - a.position[1]).abs() < 1e-4 && (b.position[2] - a.position[2]).abs() < 1e-4);
        }
        // the push grows with the curve of the height: a kind-0 template is never pushed
        let none = Sparks { push: 0, ..s.clone() };
        set_wind([20.0, 0.0, 0.0], [0.0; 3]);
        assert_eq!(sparks_particles(&none, [0.0; 3], 0.6), still);
        set_wind([0.0; 3], [0.0; 3]);
        assert_eq!(push_curve(4, 1.0), 1.0);
        assert_eq!(push_fraction(4.0, (0.0, 8.0)), 0.5);
        assert_eq!(push_fraction(4.0, (1.0, 1.0)), 0.0);
    }

    #[test]
    fn stars_mode_9_fall_from_two_metres_shrinking_as_they_drop() {
        // (template 13700: a 0.6 disc, falling 3 a second, 0.15 small, 5 per unit of height larger, living a second)
        let s = Stars { mode: 9, colour_a: [1.0, 0.4, 0.2, 0.7], colour_b: [1.0, 0.4, 0.1, 0.6], size: 3.0, radius: 0.6, life: 1.0, swirl: 500.0, material: 8, attach: 1000, duration: -1.0, word11: 0.15, word30: 1000 };
        let centre = [0.0, 0.0, 10.0];
        let at = stars_particles(&s, centre, 1.5, false);
        assert!(at.len() > 30 && at.len() <= 64, "{}", at.len());
        // within the disc; no higher than 2 m above the point, and down to a metre below it at the end of a life
        assert!(at.iter().all(|p| p.position[0].hypot(p.position[1]) <= 0.6 + 1e-4));
        assert!(at.iter().all(|p| p.position[2] <= 12.0 + 1e-4 && p.position[2] >= 10.0 - 1.0 - 1e-3));
        // the high ones are large, the fallen ones are the small size
        for p in &at {
            if p.position[2] > 11.6 {
                assert!(p.size > 0.15 + 0.5);
            }
            if p.position[2] < 11.5 {
                assert!((p.size - 0.15).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn stars_mode_1_collapse_from_a_metre_out_onto_the_point() {
        // (template 12000: at the hand, the size word 0, white-ish to orange)
        let s = Stars { mode: 1, colour_a: [1.0, 0.7, 0.3, 0.3], colour_b: [1.0, 1.0, 0.7, 0.7], size: 0.0, radius: 0.0, life: 0.1, swirl: 0.0, material: 33, attach: 2001, duration: -1.0, word11: 0.0, word30: 0 };
        let centre = [0.0, 0.0, 0.0];
        let reach = |p: &Particle| (p.position[0].powi(2) + p.position[1].powi(2) + p.position[2].powi(2)).sqrt();
        let at = stars_particles(&s, centre, 1.0, false);
        assert!(at.len() > 40, "{}", at.len());
        // all within the unit sphere; the newborn at its surface, the old ones pulled in close
        assert!(at.iter().all(|p| reach(p) <= 1.0 + 1e-4));
        assert!(at.iter().any(|p| reach(p) > 0.95) && at.iter().any(|p| reach(p) < 0.3));
        // sizes from the frame table: nothing at birth, never past 0.01 x 29
        assert!(at.iter().all(|p| p.size >= 0.0 && p.size <= 0.29 + 1e-4));
        assert!(at.iter().filter(|p| reach(p) > 0.99).all(|p| p.size < 0.02));
        assert_eq!(pulse_timing(&Visual::Stars(s.clone())).1, 2.4);
    }

    #[test]
    fn stars_mode_15_fly_off_the_body_s_surface_along_its_normals() {
        // (template 43047: thrown at 0.5 along the normal, 0.5 wide, 350 ms)
        let s = Stars { mode: 15, colour_a: [1.0, 0.2, 0.2, 1.0], colour_b: [1.0, 0.2, 1.0, 0.2], size: 0.5, radius: 0.5, life: 0.35, swirl: 5.0, material: 8, attach: 2000, duration: -1.0, word11: 0.4, word30: 350 };
        // a body of points on a vertical line at x = 1, all facing +x
        let points: Vec<[f32; 3]> = (0..124).map(|k| [1.0, 0.0, k as f32 * 0.015]).collect();
        let normals = vec![[1.0, 0.0, 0.0]; 124];
        let body = Body { points: &points, normals: &normals };
        let at = stars_off_body(&s, &body, 0.3);
        assert!(at.len() > 60, "{}", at.len());
        // every star off the line toward +x, within the decayed throw
        assert!(at.iter().all(|p| p.position[0] >= 1.0 && p.position[0] < 1.0 + 0.5 && p.position[1] == 0.0));
        assert!(at.iter().any(|p| p.position[0] > 1.1));
        assert!(at.iter().all(|p| p.size <= 0.55 + 1e-4 && p.size >= 0.1 * 0.5 - 1e-4));
        // a shot's hit uses the body when it is known, and a ring when not
        let shot = Shot { from: [0.0; 3], to: [1.0, 0.0, 1.2], tracer: None, hit: Some(Visual::Stars(s.clone())) };
        let with = shot.particles(FLIGHT + 0.3, Some(body));
        assert!(!with.is_empty() && with.iter().all(|p| p.position[1] == 0.0));
        assert!(!shot.particles(FLIGHT + 0.3, None).is_empty());
    }

    #[test]
    fn stars_mode_13_is_a_curling_ray_that_grows_and_shrinks() {
        // (template 14200: 0.7 long, turning 0.06 a step, stars 0.2 wide)
        let s = Stars { mode: 13, colour_a: [1.0, 0.5, 0.0, 0.0], colour_b: [0.6, 0.3, 0.7, 0.4], size: 0.7, radius: 0.06, life: 0.1, swirl: 0.0, material: 8, attach: 1005, duration: -1.0, word11: 0.2, word30: 0 };
        let centre = [0.0, 0.0, 0.0];
        let reach = |p: &Particle| (p.position[0].powi(2) + p.position[1].powi(2) + p.position[2].powi(2)).sqrt();
        // nothing at the very start, the most at the middle of the window, fewer again near its end
        assert!(stars_particles(&s, centre, 0.0, false).is_empty());
        let mid = stars_particles(&s, centre, LANDING * 0.5, false);
        let late = stars_particles(&s, centre, LANDING * 0.9, false);
        assert!(mid.len() > 100 && late.len() < mid.len(), "{} {}", mid.len(), late.len());
        // every star within the ray's length, all 0.2 wide; the stars at the tip sit apart, the ray is not a ball
        assert!(mid.iter().all(|p| reach(p) < 0.7 + 1e-4 && p.size == 0.2));
        let far: Vec<_> = mid.iter().filter(|p| reach(p) > 0.5).collect();
        assert!(!far.is_empty());
        let spread = far.iter().map(|p| p.position[2]).fold(f32::MIN, f32::max) - far.iter().map(|p| p.position[2]).fold(f32::MAX, f32::min);
        assert!(spread < 1.0);
        assert!(stars_particles(&s, centre, LANDING, false).is_empty());
    }

    #[test]
    fn stars_mode_2_is_a_sphere_of_all_the_stars_growing_and_shrinking_for_a_second_and_a_half() {
        // (template 13000: 2.1 wide, the colours and radius words unused)
        let s = Stars { mode: 2, colour_a: [1.0, 0.0, 1.0, 0.0], colour_b: [1.0, 0.7, 0.4, 0.4], size: 2.1, radius: 0.0, life: 0.1, swirl: 0.0, material: 33, attach: 1000, duration: -1.0, word11: 0.0, word30: 0 };
        let centre = [0.0, 0.0, 0.0];
        let reach = |p: &Particle| (p.position[0].powi(2) + p.position[1].powi(2) + p.position[2].powi(2)).sqrt();
        let early = stars_particles(&s, centre, 0.0, false);
        assert_eq!(early.len(), 128);
        // the sphere starts 0.6 wide and grows toward 1.3; the stars start at 2.1 × 20 / 29 and shrink to nothing
        assert!(early.iter().all(|p| reach(p) <= 0.6 + 1e-4) && early.iter().any(|p| reach(p) > 0.4));
        assert!(early.iter().all(|p| (p.size - 2.1 * 20.0 / 28.0).abs() < 1e-3 || (p.size - 2.1 * 20.0 / 29.0).abs() < 1e-3));
        let late = stars_particles(&s, centre, 1.4, false);
        assert!(late.iter().any(|p| reach(p) > 1.0) && late.iter().all(|p| p.size < 0.5));
        assert!(stars_particles(&s, centre, 1.5, false).is_empty());
        assert_eq!(pulse_timing(&Visual::Stars(s.clone())).1, 1.5);
    }

    #[test]
    fn stars_mode_20_stand_in_a_sphere_at_the_point_at_one_size() {
        // (template 17910: 0.3 about the point, 0.6 wide, 250 ms, fifteen a frame)
        let s = Stars { mode: 20, colour_a: [1.0, 1.0, 1.0, 1.0], colour_b: [1.0, 0.0, 0.0, 0.0], size: 0.6, radius: 0.3, life: 0.25, swirl: 6.0, material: 9, attach: 1000, duration: -1.0, word11: 0.4, word30: 250 };
        let centre = [1.0, 2.0, 3.0];
        let at = stars_particles(&s, centre, 0.2, false);
        assert!(at.len() > 60, "{}", at.len());
        for p in &at {
            let d = ((p.position[0] - 1.0).powi(2) + (p.position[1] - 2.0).powi(2) + (p.position[2] - 3.0).powi(2)).sqrt();
            assert!(d <= 0.3 + 1e-4 && p.size == 0.6);
        }
        // the cloud stands: the same star does not move between frames
        let later = stars_particles(&s, centre, 0.21, false);
        assert!(later.iter().any(|q| at.iter().any(|p| p.position == q.position)));
    }

    #[test]
    fn stars_mode_5_breathe_out_and_back_below_the_point() {
        // (template 13300: 0.4 out, pulled back by 5 hundredths a frame, living 1.3 s, sized 3.5)
        let s = Stars { mode: 5, colour_a: [1.0, 0.4, 0.1, 0.7], colour_b: [1.0, 0.3, 0.4, 0.7], size: 3.5, radius: 0.4, life: 1.3, swirl: 5.0, material: 33, attach: 1000, duration: -1.0, word11: 0.0, word30: 1300 };
        let centre = [0.0, 0.0, 10.0];
        let early = stars_particles(&s, centre, 0.4, false);
        assert!(!early.is_empty());
        // all 1.2 below the point; a young star has flown out past the circle, an older one has been pulled back inside it
        assert!(early.iter().all(|p| (p.position[2] - 8.8).abs() < 1e-4));
        let reach = |p: &Particle| p.position[0].hypot(p.position[1]);
        assert!(early.iter().any(|p| reach(p) > 0.5));
        let later = stars_particles(&s, centre, 1.3, false);
        assert!(later.iter().any(|p| reach(p) < 0.4), "{:?}", later.iter().map(reach).fold(f32::MAX, f32::min));
        assert!(later.iter().all(|p| p.size <= 3.5 * 1.2 * 1.0 + 1e-3));
    }

    #[test]
    fn stars_mode_22_ride_the_body_s_edges_and_die_crossing_them() {
        // (template 15200: six a frame, 0.2 out, living 350 ms, sized 0.6)
        let s = Stars { mode: 22, colour_a: [1.0, 0.3, 0.4, 1.0], colour_b: [1.0, 0.3, 1.0, 0.4], size: 0.6, radius: 0.2, life: 0.35, swirl: 6.0, material: 33, attach: 1000, duration: -1.0, word11: 0.0, word30: 350 };
        let feet = [5.0, 6.0, 7.0];
        let at = stars_particles(&s, feet, 0.3, false);
        assert!(at.len() > 20, "{}", at.len());
        // every star is within word 28 (0.6) of a body edge: between the calves (0.5) and the head (1.62), near the body's x;
        // sized by word 29 (0.2) from 0.3 of it
        for p in &at {
            assert!(p.position[2] >= feet[2] + 0.5 - 0.61 && p.position[2] <= feet[2] + 1.62 + 0.61, "{:?}", p.position);
            assert!((p.position[0] - feet[0]).abs() <= 0.25 + 0.61 && (p.position[1] - feet[1]).abs() <= 0.61);
            assert!(p.size >= 0.3 * 0.2 - 1e-4 && p.size <= 0.2 + 1e-4);
        }
        // a star at the quarter swing (about half a second) has crossed its edge and is gone: nothing older than that is left
        assert!(stars_particles(&s, feet, 10.0, false).len() > 20);
        // the feet, not the attach point, are the origin through the visual arm
        let through = visual_particles(&Visual::Stars(s.clone()), feet, 0.3, Reach::Medium);
        assert_eq!(through.len(), at.len());
    }

    #[test]
    fn bparticles_burst_from_the_point_fall_and_fade_by_their_curve() {
        // (template 71112's shape with fewer particles: a burst of 20 thrown ±220 across and 5–100 up, pulled down by 20,
        // damped by 0.92, living 1–2 s at 15–22 across, white at a tenth of the life and gone at its end)
        let b = BParticles {
            flags: 0x203,
            attach: 0,
            duration: 2.0,
            mode: 0,
            material: 94,
            count: 20,
            interval: 0.01,
            scatter: 8.0,
            damping: 0.92,
            per_frame: 1,
            gravity: -20.0,
            velocity: [(-220.0, 220.0), (5.0, 100.0), (-220.0, 220.0)],
            spin: (0.0, 0.0),
            life: (1.0, 2.0),
            size: (15.0, 22.0),
            bounce: 0.2,
            growth: 1.01,
            frame_rate: 0.0,
            frame_mode: 0,
            aspect: 1.0,
            colours: vec![(0.0, 0x00ff_ffff), (0.1, 0xffff_ffff), (1.0, 0x0000_0000)],
        };
        let feet = [10.0, 20.0, 30.0];
        // all twenty at once, within the scatter of the point, transparent at birth
        let born = bparticles(&b, feet, 0.0);
        assert_eq!(born.len(), 20);
        assert!(born.iter().all(|p| (p.position[0] - feet[0]).abs() <= 8.0 && (p.position[2] - feet[2]).abs() <= 8.0 && p.colour[3] == 0.0));
        // a tenth of a second on they are bright and have flown out; the damping keeps the fastest well under 22 m
        let flown = bparticles(&b, feet, 0.1);
        assert!(flown.iter().all(|p| p.colour[3] > 0.5));
        let spread = flown.iter().map(|p| (p.position[0] - feet[0]).abs()).fold(0.0, f32::max);
        assert!(spread > 5.0 && spread < 22.0, "{spread}");
        // sizes grow by 1% × 100 a second, the burst thins as lives end and is gone after the longest
        assert!(flown.iter().all(|p| p.size >= 15.0 * 1.1 - 1e-3));
        let late = bparticles(&b, feet, 1.5).len();
        assert!(late > 0 && late < 20, "{late}");
        assert!(bparticles(&b, feet, 2.1).is_empty());
        // a stream (mode 1) reborns them while it emits, and stops a longest life before its duration
        let stream = BParticles { mode: 1, duration: 10.0, ..b.clone() };
        assert_eq!(bparticles(&stream, feet, 5.0).len(), 20);
        assert!(bparticles(&stream, feet, 9.5).len() < 20 && bparticles(&stream, feet, 10.5).is_empty());
    }

    #[test]
    fn a_sprite_quad_cycles_its_size_and_colour_and_stands_when_asked() {
        // (template 7000's shape: a billboard halo 1 m wide pulsing 0.025 at 1 Hz, white, for ever)
        let s = Sprite { attach: 0, offset: [0.0; 3], duration: -1.0, material: 8, flags: 0x950, width: (1.0, 1.0), height: (1.0, 1.0), colour_a: [0.5, 1.0, 1.0, 1.0], colour_b: [0.5, 1.0, 1.0, 1.0], period: 1.0, cycles: -1, pulse: 0.025, pulse_rate: 1.0 };
        let feet = [1.0, 2.0, 3.0];
        let at = sprite_particles(&s, feet, 0.25, 7);
        assert_eq!(at.len(), 1);
        assert!((at[0].size - 1.025).abs() < 1e-4 && at[0].colour == [1.0, 1.0, 1.0, 0.5] && at[0].right == [0.0; 3] && at[0].up == [0.0; 3]);
        // the random spot moves with the salt; a frame-bound sprite (flag 0x4) keeps the sheet's own rectangle
        assert_ne!(sprite_particles(&s, feet, 0.25, 8)[0].rect, at[0].rect);
        let bound = Sprite { flags: 0x954, ..s.clone() };
        assert_eq!(sprite_particles(&bound, feet, 0.25, 8)[0].rect, sprite_particles(&bound, feet, 0.25, 9)[0].rect);
        // (template 61085's shape: a standing beam 1.5 wide and 70 tall at 29.7 up, 10 s a cycle)
        let beam = Sprite { offset: [0.0, 29.7, 0.0], material: 44, flags: 0x2953, width: (1.5, 1.5), height: (70.0, 70.0), period: 10.0, pulse: 0.07, pulse_rate: 10.0, ..s.clone() };
        let b = &sprite_particles(&beam, feet, 1.0, 1)[0];
        assert!(b.right == [0.0; 3] && (b.up[2] - 35.0).abs() < 0.1 && (b.size - 0.75).abs() < 0.1 && (b.position[2] - 32.7).abs() < 1e-4);
        // a counted one is gone after its cycles; its colour fades over a cycle when asked
        let counted = Sprite { flags: 0x9d0, cycles: 1, colour_b: [0.0, 1.0, 1.0, 1.0], ..s.clone() };
        assert!((sprite_particles(&counted, feet, 0.5, 1)[0].colour[3] - 0.25).abs() < 1e-5);
        assert!(!sprite_particles(&counted, feet, 1.9, 1).is_empty() && sprite_particles(&counted, feet, 2.1, 1).is_empty());
    }

    #[test]
    fn smoke_streams_from_its_emitter_and_drifts_the_way_its_turn_points() {
        // (template 45076's shape: ten a second for 25 s, born 1.2–1.9 out at 0.5–1 up, drifting 2 along −z over 2 s, which the
        // quarter turn about x points up; 1.5 m above the feet)
        let s = Smoke {
            flags: 5,
            attach: 0,
            offset: [0.0, 1.5, 0.0],
            turn: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            duration: 25.0,
            material: 31,
            rate: 10.0,
            radius: (1.2, 1.9),
            drift: 2.0,
            width: (7.2, 8.0),
            height: (7.0, 8.0),
            colour_a: [0.6, 0.0, 0.5, 0.1],
            colour_b: [0.4, 0.0, 0.5, 0.1],
            life: 2.0,
            rise: (0.5, 1.0),
            kind: 4,
            heights: (1.0, 8.0),
            strength: 0.25,
        };
        let feet = [100.0, 200.0, 10.0];
        // nothing yet, then one sprite a tenth of a second, up to the live ones over a life
        assert!(smoke_particles(&s, feet, 0.05).is_empty());
        assert_eq!(smoke_particles(&s, feet, 0.1).len(), 1);
        let live = smoke_particles(&s, feet, 5.0).len();
        assert!((20..=26).contains(&live), "{live}");
        // a newborn sits within the disc's reach of the offset point: the quarter turn about x sends the original's z (a disc
        // coordinate, up to 1.9 out) to −y, the height, and its y (the rise, 0.5 to 1) to z, the client's y
        for p in smoke_particles(&s, feet, 0.1) {
            let out = (p.position[0] - feet[0]).hypot(p.position[1] - feet[1]);
            assert!(out <= 1.9 + 1.0 + 1e-3, "{out}");
            assert!((p.position[2] - feet[2] - 1.5).abs() <= 1.9 + 1e-3);
            assert!(p.position[1] - feet[1] >= 0.5 - 1e-3 && p.position[1] - feet[1] <= 1.0 + 1e-3);
        }
        // the drift of −z becomes up: the oldest sprite at 5 s is higher than a newborn by about its age's share of the drift
        let at = smoke_particles(&s, feet, 5.0);
        let (lowest, highest) = at.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.position[2]), hi.max(p.position[2])));
        assert!(highest - lowest > 1.0, "{lowest} {highest}");
        // sizes are the template's 7-ish metres, colours fade from 0.6 alpha; emission stops a life before the end
        assert!(at.iter().all(|p| p.size > 7.0 && p.colour[3] <= 0.6 * 1.0001 && p.colour[3] >= 0.4 * 0.999));
        assert!(smoke_particles(&s, feet, 24.0).len() < live);
        assert!(smoke_particles(&s, feet, 27.0).is_empty());
        // an endless stream keeps going
        let endless = Smoke { duration: -1.0, ..s.clone() };
        assert!(!smoke_particles(&endless, feet, 100.0).is_empty());
    }

    #[test]
    fn a_volgrid_stacks_its_slices_in_the_box_the_curves_give() {
        // (template 72231's shape with a small grid: a column that shoots up to 80 at a tenth of the way, then sinks to 20)
        let g = VolGrid {
            flags: 0x4003,
            duration: 3.0,
            material: 100,
            across: 2,
            up: 1,
            deep: 2,
            spin: 0.0,
            height: vec![(0.0, 0.0), (0.1, 80.0), (1.0, 20.0)],
            width: vec![(0.0, 2.5), (0.1, 2.5), (1.0, 2.5)],
            depth: vec![(0.0, 0.0), (0.1, 2.5), (1.0, 2.5)],
            middle: vec![(0.0, 0x00ff_ffff), (0.1, 0xffff_ffff), (1.0, 0)],
            top: vec![(0.0, 0), (0.1, 0x0080_8080), (1.0, 0)],
        };
        let feet = [10.0, 20.0, 5.0];
        // two quads a standing slice, one an up slice
        let at = volgrid_particles(&g, feet, 0.3, 1);
        assert_eq!(at.len(), 2 * 2 * 2 + 1);
        // the box at a tenth of the way is 80 high: the standing slices' upper quads are centred at three quarters of it
        let height = 80.0 - 60.0 * (0.1 - 0.1) / 0.9;
        let tops: Vec<&Particle> = at.iter().filter(|p| (p.position[2] - feet[2] - height * 0.75).abs() < 1e-3).collect();
        assert_eq!(tops.len(), 4);
        // they are laid, not billboards, with their up half extent a quarter of the height, and fade by facing
        assert!(tops.iter().all(|p| p.up == [0.0, 0.0, height * 0.25] && p.size == 1.0 && p.right != [0.0; 3]));
        // the middle colour at the tenth is full white: the lower quads are brighter than the upper ones
        let lows: Vec<&Particle> = at.iter().filter(|p| (p.position[2] - feet[2] - height * 0.25).abs() < 1e-3).collect();
        assert!(lows[0].colour[3] > tops[0].colour[3]);
        // the up slice lies flat at the bottom, half the width out each way
        let flat = at.last().unwrap();
        assert!((flat.position[2] - feet[2]).abs() < 1e-6 && flat.up[2] == 0.0 && (flat.right[0].hypot(flat.right[1]) - 1.25).abs() < 1e-5);
        // gone when the duration is up; the random turn of flag 0x1000 changes with the salt only
        assert!(volgrid_particles(&g, feet, 3.0, 1).is_empty());
        let turned = VolGrid { flags: 0x5003, ..g.clone() };
        assert_eq!(volgrid_particles(&turned, feet, 0.3, 1), volgrid_particles(&turned, feet, 0.3, 1));
        assert_ne!(volgrid_particles(&turned, feet, 0.3, 1)[0].right, volgrid_particles(&turned, feet, 0.3, 2)[0].right);
    }

    #[test]
    fn a_pulse_rides_a_ring_at_the_body_s_place() {
        let helix = mao_formats::gfx::Helix { spin: 12.6, phase: 0.0, size: 0.13, count: 10, colour: [0.5, 0.4, 1.0], material: 15, attach: 1006, offset: -0.1, radius: 0.13, table: vec![(-0.5, 0.2); 24], life: 1.5, ribbon_width: 0.05, ribbon_life: 0.5, ribbon_duration: 1.5, ribbon_material: 8, sparkle_rate: 0.0 };
        let aura = Visual::Spiral(mao_formats::gfx::Aura { colour: [0.99, 0.99, 0.2], end: [0.99, 0.99, 0.2], helices: vec![helix], period: 4.0, plays: -1 });
        // without a body: the template's own place; with one: the table's
        // the head bone at 1.62 m: the offset climbs toward the crown
        let own = pulse_particles(&aura, [0.0, 0.0, 0.0], 0.0, 0.8, None);
        assert!(!own.is_empty() && own.iter().all(|p| (p.position[2] - 1.72).abs() < 0.06));
        let fitted = pulse_particles(&aura, [0.0, 0.0, 0.0], 0.0, 0.8, Some((1, 2, 1)));
        assert!(fitted.iter().all(|p| (p.position[2] - 2.12).abs() < 0.06));
        // the ribbon grows with the pulse, the cord stops laying links a life before its duration, nothing past the pulse
        assert!(pulse_particles(&aura, [0.0, 0.0, 0.0], 0.0, 0.1, None).len() < own.len());
        // (the last links, laid by 1.04 s, are still fading at 1.45 s: faint)
        assert!(pulse_particles(&aura, [0.0, 0.0, 0.0], 0.0, 1.45, None).iter().all(|p| p.colour[3] < 0.3));
        assert!(pulse_particles(&aura, [0.0, 0.0, 0.0], 0.0, 1.6, None).is_empty());
        // a calf ring sits at mid-shin, a right-hand ring to the character's right
        assert!((attach_point(1014, -0.25, [0.0; 3], 0.0)[2] - 0.25).abs() < 0.01);
        // the mirrored left calf: its ring goes up into the knee instead
        assert!((attach_point(1013, -0.25, [0.0; 3], 0.0)[2] - 0.75).abs() < 0.01);
        assert!(attach_point(2000, 0.0, [0.0; 3], 0.0)[0] > 0.2);
        assert_eq!(pulse_timing(&aura), (4.0, 1.5, -1));
    }
    use mao_formats::gfx::{Aura, Helix};

    fn aura() -> Aura {
        let helix = |spin: f32| Helix { spin, phase: 0.0, size: 0.15, count: 10, colour: [0.5, 0.4, 1.0], material: 15, attach: 1006, offset: -0.25, radius: 0.15, table: Vec::new(), life: 1.5, ribbon_width: 0.05, ribbon_life: 0.5, ribbon_duration: 1.5, ribbon_material: 8, sparkle_rate: 0.0 };
        Aura { colour: [0.2, 0.2, 0.99], end: [0.2, 0.2, 0.99], helices: vec![helix(-12.566), helix(12.566)], period: 4.0, plays: -1 }
    }

    #[test]
    fn suns_stand_at_the_point_in_mode_4_and_stack_above_it_in_mode_0() {
        // (template 17000: Electricify, 0.4 wide, 300 ms, blue)
        let four = Suns { mode: 4, attach: 1000, material: 2, colour_a: [1.0, 0.3, 0.3, 1.0], colour_b: [1.0, 0.3, 0.3, 1.0], duration: 1.0, size: 0.4, step: 15.99, life: 0.3 };
        let at = suns_particles(&four, [1.0, 2.0, 3.0], 0.5);
        assert!(at.len() >= 20 && at.len() <= 32, "{}", at.len());
        assert!(at.iter().all(|p| p.position == [1.0, 2.0, 3.0] && p.size == 0.4 && p.colour == [0.3, 0.3, 1.0, 1.0]));
        assert!(suns_particles(&four, [1.0, 2.0, 3.0], 1.0).is_empty());
        // (template 43768: 32 halos two above, 0.6 growing by 0.25 each, over 180 s)
        let zero = Suns { mode: 0, attach: 1000, material: 8, colour_a: [0.0; 4], colour_b: [0.0; 4], duration: 180.0, size: 0.6, step: 0.25, life: 0.0 };
        let mid = suns_particles(&zero, [0.0; 3], 90.0);
        assert_eq!(mid.len(), 32);
        assert!(mid.iter().all(|p| p.position == [0.0, 0.0, 2.0] && (p.colour[3] - 0xc0 as f32 / 255.0).abs() < 1e-5));
        assert!((mid[31].size - (0.6 + 31.0 * 0.25)).abs() < 1e-3);
        assert!(suns_particles(&zero, [0.0; 3], 0.0).iter().all(|p| p.size == 0.0));
    }

    #[test]
    fn a_shield_is_the_mesh_pushed_out_fading_in_and_gone_at_its_duration() {
        // (template 43608: cyan at 0x88, 0.255 out, thin smoke scrolling, in over a second, 3 s, added)
        let s = Shield { flags: 0x400, duration: 3.0, material: 32, colour: 0x8800_ffff, push: 0.255, point: [0.0; 3], axis: [0.0; 3], mapping: 0, scale: (2.0, 2.0), fade: 1.571, distance: 15.0, most: 1.571, speed: (0.05, 0.2), looping: 0, loop_range: (0.0, 0.0), ending: 1, end_time: 0.0, end_from: 0.0 };
        let surface = crate::blobshadow::Surface { positions: vec![[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]], normals: vec![[0.0, 0.0, 1.0]; 3], uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], indices: vec![0, 1, 2] };
        let at = |t: f32| shield_shell(&s, &surface, [0.0; 3], t);
        let half = at(0.5).unwrap();
        assert!(half.additive && half.sprite == 32 && half.vertices.len() == 3);
        // pushed out along the normal, cyan, the alpha sin² of the fade times 0x88 / 255
        let v = half.vertices[1];
        assert!((v.position[2] - 1.255).abs() < 1e-5 && v.colour[..3] == [0.0, 1.0, 1.0]);
        let q = (1.571f32 * 0.5).sin();
        assert!((v.colour[3] - 136.0 / 255.0 * q * q).abs() < 1e-4);
        // the texture scrolls: (uv + speed t) × scale
        assert!((v.uv[0] - (1.0 + 0.05 * 0.5) * 2.0).abs() < 1e-5 && (v.uv[1] - 0.2 * 0.5 * 2.0).abs() < 1e-5);
        // fully in after a second, ended at its duration (no fade-out time)
        assert!((at(1.5).unwrap().vertices[0].colour[3] - 136.0 / 255.0).abs() < 1e-3);
        assert!(at(3.01).is_none());
        // a fade-out of mode 1 over a second: half at its middle
        let fading = Shield { end_time: 1.0, ..s.clone() };
        assert!((fading.time(2.5).unwrap().1 - 0.5).abs() < 1e-5 && fading.time(3.1).is_none());
        assert_eq!(pulse_timing(&Visual::Shield(s)), (600.0, 3.0, 1));
    }

    #[test]
    fn the_plasma_tracer_is_a_band_along_x_wobbling_and_gone_after_its_duration() {
        // (template 17500: green, a second)
        let v = Visual::Plasma { colour_a: [1.0, 0.3, 1.0, 0.3], colour_b: [1.0, 0.3, 1.0, 0.3], material: 8, duration: 1.0 };
        let at = visual_particles(&v, [5.0, 6.0, 7.0], 0.3, Reach::Small);
        assert_eq!(at.len(), 8);
        // from the point 0.75 along x, 0.2 wide, within a tenth of the waves' 0.75 of it, green
        let xs: Vec<f32> = at.iter().map(|p| p.position[0]).collect();
        assert!((xs[0] - 5.0).abs() < 1e-4 && (xs[7] - 5.75).abs() < 1e-4);
        assert!(at.iter().all(|p| p.position[1] == 6.0 && (p.position[2] - 7.25).abs() <= 0.075 + 1e-4 && p.size == 0.2));
        assert!(at.iter().all(|p| p.colour == [0.3, 1.0, 0.3, 1.0]));
        // it moves with time and ends with its duration
        assert_ne!(visual_particles(&v, [5.0, 6.0, 7.0], 0.5, Reach::Small)[0].position, at[0].position);
        assert!(visual_particles(&v, [5.0, 6.0, 7.0], 1.0, Reach::Small).is_empty());
    }

    #[test]
    fn a_cast_sparkles_at_both_hands_until_the_cast_ends() {
        let flare = |attach: i32| Flare { duration: -1.0, material: 15, rate: 16.0, width: (0.05, 0.1), colour_a: [1.0, 0.5, 0.4, 1.0], colour_b: [0.0, 0.5, 0.4, 1.0], gate: 0, azimuth: (0.0, 6.283), elevation: (-1.571, 1.571), speed: (0.35, 0.85), head: -0.3, tail: 1.0, life: (0.5, 1.0), attach };
        // (template 46133: blue, phase 1 at 0–6 s, the beam at 4–5 s)
        let cast = Cast { material: 8, colour_a: [1.0, 0.2, 0.2, 1.0], colour_b: [0.0, 0.2, 0.2, 1.0], times: [0.0, 6.0, 8.0, 4.0, 5.0, 64.35, 70.0, 70.0, 80.0], flares: [Some(flare(2001)), Some(flare(2000))], released: false };
        let show = CastShow { feet: [10.0, 20.0, 0.0], heading: 0.0, cast, length: CAST_AT_OTHER };
        // nothing before the first particle, then streaks round each hand in the cast's colour, fading as they go
        assert!(cast_particles(&show, 0.01).is_empty());
        let at = cast_particles(&show, 0.8);
        assert!(at.len() >= 2 * 3 * 8, "{}", at.len());
        let hands = [attach_point(2001, 0.0, show.feet, 0.0), attach_point(2000, 0.0, show.feet, 0.0)];
        for p in &at {
            let near = hands.iter().map(|h| ((p.position[0] - h[0]).powi(2) + (p.position[1] - h[1]).powi(2) + (p.position[2] - h[2]).powi(2)).sqrt()).fold(f32::MAX, f32::min);
            assert!(near <= 0.85 + 1e-3, "{near}");
            assert!((p.colour[0] - 0.2).abs() < 1e-4 && (p.colour[2] - 1.0).abs() < 1e-4 && p.colour[3] <= 1.0);
        }
        // both hands: streaks to either side of the body
        assert!(at.iter().any(|p| p.position[0] < 10.0) && at.iter().any(|p| p.position[0] > 10.0));
        // gone with the cast; and a long cast shows the beam between the hands from 4 s
        assert!(cast_particles(&show, 1.3).is_empty());
        let long = CastShow { length: 10.0, ..show.clone() };
        let beam = cast_particles(&long, 4.2);
        assert!(beam.iter().any(|p| p.sprite == 8 && (p.size - 0.25).abs() < 0.6));
    }

    #[test]
    fn the_ring_is_the_cord_s_ribbon_laid_a_link_each_step_and_widening_as_it_fades() {
        let a = aura();
        let mut out = Vec::new();
        ring(&mut out, [10.0, 20.0, 5.0], 0.15, &a.helices[1], a.colour, 0.7);
        assert!(!out.is_empty());
        // on the ring's height, out from the centre by the radius to the drift of a whole life (15 steps of 0.025)
        for p in &out {
            let r = (p.position[0] - 10.0).hypot(p.position[1] - 20.0);
            assert!(p.position[2] == 5.0 && r >= 0.15 * 0.99 - 1e-3 && r <= 0.15 * (1.0 + 0.025 * 15.0) + 1e-3, "{r}");
            assert!(p.colour[3] > 0.0 && p.colour[3] <= 1.0 && p.size == 0.1);
        }
        // no more than the 32 newest links, and a life of 0.5 s at 30 a second is 15 of them
        let steps = (0.7f32 * 12.566 / 0.45).floor() as usize;
        assert!(steps > 15);
    }

    #[test]
    fn a_placeholder_that_spins_backwards_never_steps_so_draws_nothing() {
        let a = aura();
        let mut back = Vec::new();
        ring(&mut back, [0.0; 3], 0.15, &a.helices[0], a.colour, 0.7);
        assert!(a.helices[0].spin < 0.0 && back.is_empty());
        // the whole aura: only the forward ring shows
        let all = visual_particles(&Visual::Spiral(a.clone()), [0.0; 3], 0.7, Reach::Body);
        let mut forward = Vec::new();
        ring(&mut forward, attach_point(1006, -0.25, [0.0; 3], 0.0), 0.15, &a.helices[1], a.colour, 0.7);
        assert_eq!(all, forward);
    }

    #[test]
    fn the_calibration_file_is_read_by_mesh_and_animation() {
        let table = parse_calibration("# mesh anim calibration
29902	29914	0.70
29902	30227	0.50

broken line
");
        assert_eq!(table.get(&(29902, 29914)), Some(&0.7));
        assert_eq!(table.get(&(29902, 30227)), Some(&0.5));
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn stars_stand_in_their_sphere_with_colours_between_the_two() {
        let s = Stars { mode: 19, colour_a: [1.0, 1.0, 0.0, 0.0], colour_b: [1.0, 0.0, 0.0, 1.0], size: 0.5, radius: 0.7, life: 1.0, swirl: 30.0, material: 8, attach: 1000, duration: -1.0, word11: 0.0, word30: 1 };
        let stars = stars_particles(&s, [10.0, 20.0, 5.0], 0.6, false);
        assert!(!stars.is_empty());
        for p in &stars {
            let d = ((p.position[0] - 10.0).powi(2) + (p.position[1] - 20.0).powi(2) + (p.position[2] - 5.0).powi(2)).sqrt();
            assert!(d <= 0.7 + 1e-4, "{d}");
            assert!(p.colour[0] >= 0.0 && p.colour[0] <= 1.0 && p.colour[2] >= 0.0 && p.colour[2] <= 1.0 && p.colour[1] == 0.0);
            assert!(p.size >= 0.0 && p.size <= 0.5 + 1e-5);
        }
        // mode 6 rises: later stars sit higher than their birth height
        let s6 = Stars { mode: 6, ..s };
        let high = stars_particles(&s6, [0.0; 3], 0.9, false).iter().map(|p| p.position[2]).fold(f32::MIN, f32::max);
        assert!(high > 0.2, "{high}");
        // nothing before the first star is born
        assert!(stars_particles(&s6, [0.0; 3], 0.0, false).len() <= 1);
        // a ring: all 128 on one circle that swells with time
        let ring = Stars { mode: 7, duration: 4.0, swirl: 1.0, word30: 1, ..s.clone() };
        let at = |time: f32| stars_particles(&ring, [0.0; 3], time, false);
        assert_eq!(at(1.0).len(), 128);
        let reach = |p: &Particle| (p.position[0].powi(2) + p.position[1].powi(2)).sqrt();
        assert!(reach(&at(2.0)[0]) > reach(&at(1.0)[0]));
        // thrown stars fall
        let thrown = Stars { mode: 4, ..s.clone() };
        let low = stars_particles(&thrown, [0.0; 3], 0.9, false).iter().map(|p| p.position[2]).fold(f32::MAX, f32::min);
        assert!(low < -0.1, "{low}");
    }

    #[test]
    fn the_cords_wind_round_the_line_of_flight() {
        let c = Cords { width: 0.1, radius: 0.25, colour: [1.0, 0.5, 0.5, 1.0], material: 15, duration: 1.0 };
        let links = cords_particles(&c, [0.0, 0.0, 1.0], [10.0, 0.0, 1.0], 0.2);
        assert_eq!(links.len(), 60);
        // every link within the winding's reach of the line, spread along it
        for p in &links {
            let off = (p.position[1].powi(2) + (p.position[2] - 1.0).powi(2)).sqrt();
            assert!(off <= 0.25 * 1.2 * 1.5 + 1e-4, "{off}");
            assert!(p.position[0] >= -0.2 && p.position[0] <= 11.0, "{:?}", p.position);
        }
        let xs: Vec<f32> = links.iter().map(|p| p.position[0]).collect();
        assert!(xs.iter().cloned().fold(f32::MIN, f32::max) > 9.0);
        // the winding turns with time
        let later = cords_particles(&c, [0.0, 0.0, 1.0], [10.0, 0.0, 1.0], 0.25);
        assert!(later[5].position != links[5].position);
    }

    #[test]
    fn a_deform_ramps_in_ripples_and_ramps_out() {
        let d = Deform { duration: 1.1, strength: 1.0, ramp_in: 0.1, ramp_out: 1.0, frequency: 0.1, amplitude: 0.05 };
        assert!((d.strength_at(0.05) - 0.5).abs() < 1e-5);
        assert!((d.strength_at(0.1) - 1.0).abs() < 1e-5);
        assert!(d.strength_at(0.6) < 1.0 && d.strength_at(0.6) > 0.0);
        assert_eq!(d.strength_at(1.2), 0.0);
        let mut parts = vec![mao_formats::model::Part { name: String::new(), world: mao_formats::model::IDENTITY, vertices: vec![mao_formats::model::Vertex { position: [0.0, 1.0, 0.0], normal: [1.0, 0.0, 0.0], uv: [0.0; 2] }], indices: vec![], material: Default::default() }];
        deform_parts(&mut parts, &d, 0.3);
        let x = parts[0].vertices[0].position[0];
        assert!(x >= 0.0 && x <= 0.05 + 1e-6, "{x}");
        assert_eq!(parts[0].vertices[0].position[1], 1.0);
    }

    #[test]
    fn sparks_fly_out_fade_and_fall() {
        let s = Sparks { colour_a: [1.0, 1.0, 1.0, 0.0], colour_b: [0.0, 1.0, 0.0, 0.0], width: (0.1, 0.3), height: (0.1, 0.3), azimuth: (0.0, 6.28), elevation: (0.3, 1.2), speed: (2.0, 3.0), scale: 1.0, life: (0.8, 1.0), count: 8, gravity: -9.8, material: 16, attach: 1000, gate: 0, push: 0, push_heights: (0.0, 8.0), push_strength: 1.0 };
        let early = sparks_particles(&s, [0.0; 3], 0.1);
        assert!(early.len() >= 8 && early.len() <= 23, "{}", early.len());
        // they rise at first, then gravity brings them below the start
        assert!(early.iter().all(|p| p.position[2] > 0.0));
        let late = sparks_particles(&s, [0.0; 3], 0.79);
        assert!(late.iter().any(|p| p.position[2] < 0.0));
        // the alpha fades toward the end colour's
        assert!(late.iter().all(|p| p.colour[3] < early[0].colour[3]));
        // all gone past the longest life
        assert!(sparks_particles(&s, [0.0; 3], 1.3).is_empty());
    }

    fn emitter() -> Emitter {
        Emitter { id: 1, kind: 1005, start: [1.0, 0.5, 0.4, 1.0], end: [0.0, 0.5, 0.4, 1.0], size: (0.1, 0.2), life: 1.0, rate: 10.0, azimuth: 6.28, elevation: (-1.57, 1.57), speed: (0.5, 1.0), gravity: 0.0, material: 15 }
    }

    #[test]
    fn an_emitter_makes_its_rate_of_particles_that_fade_and_grow() {
        let e = emitter();
        // after 0.55 s, 6 particles have been born (at 0.0, 0.1 ... 0.5), all still alive
        let now = emitter_particles(&e, [0.0; 3], 0.55, 2.0);
        assert_eq!(now.len(), 6);
        // the oldest is the faintest and the largest
        let (oldest, youngest) = (&now[0], &now[5]);
        assert!(oldest.colour[3] < youngest.colour[3]);
        assert!(oldest.size > youngest.size);
        // once the emitter stopped (at 0.3 s) and everything has lived out, nothing is left
        assert!(emitter_particles(&e, [0.0; 3], 3.0, 0.3).is_empty());
        // the same time gives the same particles
        assert_eq!(emitter_particles(&e, [1.0, 2.0, 3.0], 0.4, 1.0), emitter_particles(&e, [1.0, 2.0, 3.0], 0.4, 1.0));
    }
}
