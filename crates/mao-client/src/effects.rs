//! Nano effects drawn as glowing sprites in the world (the renderer's particle pass). The templates come from `gfxtweak.bin`
//! (`mao_formats::gfx`). A running buff's effect (kind 1001) is played as the original plays it: a short pulse every few seconds,
//! a ring of sparks at a height on the body that forms, spins and fades in a second and a half ([`running_particles`]); the hits
//! and tracers built on the same spirals run continuously for their short lives ([`aura_shaped`]).
//!
//! What the data gives: the colour, each spiral's spin (about two turns a second), its start angle, spark size and count. What it
//! does not say (or is not read yet): the radius (the radius path's numbers, 0.06 to 0.15, are too small for a body; 0.42 m is used),
//! how high and how long a spark lives (1.9 m and 1.6 s here), and the trail (a short tail of fading sparks behind each one).

use mao_formats::gfx::{colour_curve, curve, unpack_argb, Aura, Cords, Deform, Emitter, Smoke, Sparks, Sprite, Stars, Visual, VolGrid};
use mao_render::{Particle, NO_SPRITE};

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
        let position = [origin[0] + v[0] * age, origin[1] + v[1] * age, origin[2] + v[2] * age + 0.5 * s.gravity * age * age];
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
    let spawn_per_frame: f32 = match s.mode {
        19 => 15.0,
        10 => 5.0,
        _ => 2.0,
    };
    let s = &Stars { life: if s.mode == 10 { s.size.max(0.1) } else { s.life }, ..s.clone() };
    // (the original spawns at most that many a frame of its run; at the client's frame rate that is a cadence per star)
    let cadence = s.life / STARS as f32 * (30.0 / spawn_per_frame).min(STARS as f32);
    for k in 0..STARS {
        // the star's births: every `cadence` seconds from its own stagger, each living `life` (mode 19: a random share)
        let stagger = k as f32 / STARS as f32 * s.life;
        if time < stagger {
            continue;
        }
        let generation = ((time - stagger) / (s.life + cadence)).floor();
        let born = stagger + generation * (s.life + cadence);
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
            _ => 1.0 - hash(salt, 9) * 0.25,
        };
        out.push(sprite(s.material, frame, position, size.max(0.0), [colour[0], colour[1], colour[2], colour[3]]));
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
        let local = [0, 1, 2].map(|i| p[i] + s.offset[i] + v[i] * age);
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

/// How far from the body the spirals run, in metres (a choice: see above).
const RADIUS: f32 = 0.42;
/// How tall the spirals climb, in metres, and how long a spark takes to do it.
const HEIGHT: f32 = 1.9;
const LIFE: f32 = 1.6;
/// The sparks behind each spark of a spiral, and how far apart they trail (radians).
const TRAIL: usize = 5;
const TRAIL_STEP: f32 = 0.22;

/// The sparks of an aura at `time` seconds, round a character whose feet are at `feet` (world, Z up).
pub fn aura_particles(aura: &Aura, feet: [f32; 3], time: f32) -> Vec<Particle> {
    aura_shaped(aura, feet, time, HEIGHT, RADIUS)
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
const FLIGHT: f32 = 0.45;
const LANDING: f32 = 1.4;

impl Shot {
    pub fn lasts(&self) -> f32 {
        FLIGHT + LANDING
    }

    /// The sparks of the shot `age` seconds after it was cast.
    pub fn particles(&self, age: f32) -> Vec<Particle> {
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
            for mut p in visual_particles(hit, [self.to[0], self.to[1], self.to[2] - 1.0], landed, Reach::Medium) {
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
            let up = match reach {
                Reach::Small => 0.0,
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
        Visual::Spiral(aura) => match reach {
            Reach::Body => aura_shaped(aura, feet, time, HEIGHT, RADIUS),
            Reach::Medium => aura_shaped(aura, feet, time, 1.5, 0.35),
            Reach::Small => aura_shaped(aura, feet, time, 0.5, 0.18),
        },
        // a halo under the feet that breathes
        Visual::Glow { colour, .. } => {
            let breathe = 0.75 + 0.25 * (time * 2.5).sin();
            let wide = match reach {
                Reach::Body => 1.7,
                Reach::Medium => 1.4,
                Reach::Small => 0.6,
            };
            vec![
                sprite(8, 0.0, [feet[0], feet[1], feet[2] + 0.08], wide, [colour[0], colour[1], colour[2], colour[3] * 0.55 * breathe]),
                sprite(8, 0.0, [feet[0], feet[1], feet[2] + 0.12], wide * 0.55, [colour[0], colour[1], colour[2], colour[3] * 0.5 * breathe]),
            ]
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
        Visual::Orb { colour, size } => {
            let pulse = 0.85 + 0.15 * (time * 18.0).sin();
            vec![
                sprite(8, 0.0, [feet[0], feet[1], feet[2] + 0.25], size * 2.2 * pulse, [colour[0], colour[1], colour[2], 0.35]),
                sprite(15, 0.0, [feet[0], feet[1], feet[2] + 0.25], *size, [colour[0], colour[1], colour[2], 0.9]),
            ]
        }
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
        // the left one into the body, where the ring is hidden: the original shows these effects on the right limb only (seen on the
        // thigh ring of a Martial Artist's buff).
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
/// The shield effect (kind 3003, `_GfxControlShield_t`: the character's own mesh drawn again with a material) is stood in for by a
/// ring that climbs the legs about every twenty seconds, as the recording shows its band doing; its real timing is in the material.
const SHIELD_PERIOD: f32 = 20.0;
const SHIELD_LIFE: f32 = 1.5;
/// The trail behind the riding point (kind 1003: 60 sprites a second of light_halo2, each 0.05 m, living 1.5 s).
const TRAIL_RATE: f32 = 60.0;
const TRAIL_LIFE: f32 = 1.5;

/// One placeholder's pulse `t` seconds in: the point riding the circle (radius `radius`, centre `feet` + `z`) at `spin` radians a second
/// from `angle0`, drawn as the trail it leaves (one sprite per 1/60 s of its path, each fading over its 1.5 s life) and a few sparkles at
/// the point. The whole pulse fades out over its last 0.3 s.
fn ride(out: &mut Vec<Particle>, feet: [f32; 3], z: f32, radius: f32, angle0: f32, spin: f32, t: f32, life: f32, colour: [f32; 3]) {
    let end_fade = ((life - t) / 0.3).clamp(0.0, 1.0);
    let samples = (t * TRAIL_RATE) as usize;
    let first = samples.saturating_sub((TRAIL_LIFE * TRAIL_RATE) as usize);
    for k in first..=samples {
        let born = k as f32 / TRAIL_RATE;
        let age = (t - born) / TRAIL_LIFE;
        let angle = angle0 + spin * born;
        let alpha = (1.0 - age).clamp(0.0, 1.0) * 0.55 * end_fade;
        out.push(sprite(8, 0.0, [feet[0] + radius * angle.cos(), feet[1] + radius * angle.sin(), z], 0.05 + 0.03 * age, [colour[0], colour[1], colour[2], alpha]));
    }
    // the sparkles: tiny bright points scattered round the riding point
    let head = angle0 + spin * t;
    for k in 0..4u32 {
        let (dx, dy, dz) = (hash(k, 11) - 0.5, hash(k, 12) - 0.5, hash(k, 13) - 0.5);
        out.push(sprite(15, 0.0, [feet[0] + radius * head.cos() + dx * 0.05, feet[1] + radius * head.sin() + dy * 0.05, z + dz * 0.05], 0.025, [1.0, 1.0, 1.0, 0.8 * end_fade]));
    }
}

/// The sparks of one running effect `age` seconds into its pulse (from `pulses::Pulses::tick`), round a character whose feet are at `feet`
/// (world, Z up) facing `heading`, and whose body is `(breed, sex, fatness)` when known. `Visual::Spiral` is the kind-1001 pair of placeholders; the shield
/// and the rest are stood in for.
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
                ride(&mut out, at, at[2], radius.max(0.03), helix.phase, helix.spin, age, helix.life, aura.colour);
            }
        }
        Visual::Glow { colour, size } => {
            if age < SHIELD_LIFE {
                let u = age / SHIELD_LIFE;
                ride(&mut out, feet, feet[2] + 0.12 + 0.55 * u, 0.3 * size.max(0.5).min(1.5), 0.0, 9.0, age, SHIELD_LIFE, [colour[0], colour[1], colour[2]]);
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
        Visual::Stars(s) => (if s.duration > 0.0 { s.duration } else { 4.0 }, s.life * 2.0, -1),
        Visual::Sparks(s) => (4.0, s.life.1 + 0.2, -1),
        Visual::Deform(d) => (4.0, if d.duration > 0.0 { d.duration } else { 1.5 }, -1),
        Visual::Cords(c) => (4.0, if c.duration > 0.0 { c.duration } else { 1.0 }, -1),
        Visual::VolGrid(g) => (if g.duration > 0.0 { g.duration } else { 3.0 }, if g.duration > 0.0 { g.duration } else { 3.0 }, -1),
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
        Visual::Glow { .. } => (SHIELD_PERIOD, SHIELD_LIFE, -1),
        _ => (4.0, 1.5, -1),
    }
}

/// [`aura_particles`] for a spiral of another height and radius (a tracer is short, a body's is tall).
pub fn aura_shaped(aura: &Aura, feet: [f32; 3], time: f32, height_m: f32, radius_m: f32) -> Vec<Particle> {
    let mut out = Vec::new();
    for helix in &aura.helices {
        // (the spirals' sparks start at the colour of the buff and warm toward the helix's own light colour)
        let n = (helix.count as usize).clamp(1, 24);
        for i in 0..n {
            let age = (time / LIFE + i as f32 / n as f32).fract();
            let height = age * height_m;
            let strength = (1.0 - age).powf(0.7) * (age * 8.0).min(1.0);
            let colour = |t: f32| {
                [
                    aura.colour[0] * (1.0 - t) + helix.colour[0] * t,
                    aura.colour[1] * (1.0 - t) + helix.colour[1] * t,
                    aura.colour[2] * (1.0 - t) + helix.colour[2] * t,
                ]
            };
            let c = colour(0.12 + 0.2 * age);
            for k in 0..=TRAIL {
                let angle = helix.phase + helix.spin * time - helix.spin.signum() * TRAIL_STEP * k as f32 + i as f32 * 0.6;
                let fade = 1.0 - k as f32 / (TRAIL + 1) as f32;
                // each spark leans in a little as it rises, like a flame
                let radius = radius_m * (1.0 - 0.25 * age);
                out.push(sprite(
                    helix.material,
                    age,
                    [feet[0] + radius * angle.cos(), feet[1] + radius * angle.sin(), feet[2] + height - 0.04 * k as f32],
                    helix.size.max(0.05) * (0.45 + 0.55 * fade) * 0.85,
                    [c[0], c[1], c[2], 0.75 * strength * fade * fade],
                ));
            }
        }
    }
    out
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
        let helix = mao_formats::gfx::Helix { spin: 12.6, phase: 0.0, size: 0.13, count: 10, colour: [0.5, 0.4, 1.0], material: 15, attach: 1006, offset: -0.1, radius: 0.13, table: vec![(-0.5, 0.2); 24], life: 1.5 };
        let aura = Visual::Spiral(mao_formats::gfx::Aura { colour: [0.99, 0.99, 0.2], end: [0.99, 0.99, 0.2], helices: vec![helix], period: 4.0, plays: -1 });
        // without a body: the template's own place; with one: the table's
        // the head bone at 1.62 m: the offset climbs toward the crown
        let own = pulse_particles(&aura, [0.0, 0.0, 0.0], 0.0, 0.8, None);
        assert!(!own.is_empty() && own.iter().all(|p| (p.position[2] - 1.72).abs() < 0.06));
        let fitted = pulse_particles(&aura, [0.0, 0.0, 0.0], 0.0, 0.8, Some((1, 2, 1)));
        assert!(fitted.iter().all(|p| (p.position[2] - 2.12).abs() < 0.06));
        // the trail grows with the pulse, and nothing is drawn past its life
        assert!(pulse_particles(&aura, [0.0, 0.0, 0.0], 0.0, 0.2, None).len() < own.len());
        assert!(pulse_particles(&aura, [0.0, 0.0, 0.0], 0.0, 1.6, None).is_empty());
        // a calf ring sits at mid-shin, a right-hand ring to the character's right
        assert!((attach_point(1014, -0.25, [0.0; 3], 0.0)[2] - 0.25).abs() < 0.01);
        // the mirrored left calf: its ring goes up into the knee instead
        assert!((attach_point(1013, -0.25, [0.0; 3], 0.0)[2] - 0.75).abs() < 0.01);
        assert!(attach_point(2000, 0.0, [0.0; 3], 0.0)[0] > 0.2);
        assert_eq!(pulse_timing(&aura), (4.0, 1.5, -1));
    }
    use mao_formats::gfx::Helix;

    fn aura() -> Aura {
        let helix = |spin: f32| Helix { spin, phase: 0.0, size: 0.15, count: 10, colour: [0.5, 0.4, 1.0], material: 15, attach: 1006, offset: -0.25, radius: 0.15, table: Vec::new(), life: 1.5 };
        Aura { colour: [0.2, 0.2, 0.99], end: [0.2, 0.2, 0.99], helices: vec![helix(-12.566), helix(12.566)], period: 4.0, plays: -1 }
    }

    #[test]
    fn sparks_stay_round_the_character_and_below_head_height() {
        let sparks = aura_particles(&aura(), [10.0, 20.0, 5.0], 0.7);
        // two spirals of ten sparks, each with its trail
        assert_eq!(sparks.len(), 2 * 10 * (TRAIL + 1));
        for s in &sparks {
            let (dx, dy) = (s.position[0] - 10.0, s.position[1] - 20.0);
            assert!((dx * dx + dy * dy).sqrt() <= RADIUS + 1e-3);
            assert!(s.position[2] >= 5.0 - 0.04 * TRAIL as f32 - 1e-3 && s.position[2] <= 5.0 + HEIGHT + 1e-3);
            assert!(s.colour[3] >= 0.0 && s.colour[3] <= 1.0);
        }
    }

    #[test]
    fn the_two_spirals_turn_opposite_ways() {
        let a = aura();
        let at = |time: f32| aura_particles(&a, [0.0; 3], time);
        // the lead spark of each spiral: indices 0 and 10 * (TRAIL + 1)
        let angle = |p: &Particle| p.position[1].atan2(p.position[0]);
        let (first, second) = (0, 10 * (TRAIL + 1));
        let (a0, b0) = (angle(&at(0.0)[first]), angle(&at(0.0)[second]));
        let (a1, b1) = (angle(&at(0.02)[first]), angle(&at(0.02)[second]));
        let turn = |x: f32, y: f32| (y - x + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        assert!(turn(a0, a1) < 0.0 && turn(b0, b1) > 0.0);
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
        let s = Sparks { colour_a: [1.0, 1.0, 1.0, 0.0], colour_b: [0.0, 1.0, 0.0, 0.0], width: (0.1, 0.3), height: (0.1, 0.3), azimuth: (0.0, 6.28), elevation: (0.3, 1.2), speed: (2.0, 3.0), scale: 1.0, life: (0.8, 1.0), count: 8, gravity: -9.8, material: 16, attach: 1000, gate: 0 };
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
