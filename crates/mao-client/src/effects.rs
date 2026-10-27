//! Nano effects drawn as glowing sprites in the world (the renderer's particle pass). The templates come from `gfxtweak.bin`
//! (`mao_formats::gfx`). A running buff's effect (kind 1001) is played as the original plays it: a short pulse every few seconds,
//! a ring of sparks at a height on the body that forms, spins and fades in a second and a half ([`running_particles`]); the hits
//! and tracers built on the same spirals run continuously for their short lives ([`aura_shaped`]).
//!
//! What the data gives: the colour, each spiral's spin (about two turns a second), its start angle, spark size and count. What it
//! does not say (or is not read yet): the radius (the radius path's numbers, 0.06 to 0.15, are too small for a body; 0.42 m is used),
//! how high and how long a spark lives (1.9 m and 1.6 s here), and the trail (a short tail of fading sparks behind each one).

use mao_formats::gfx::{Aura, Emitter, Visual};
use mao_render::{Particle, NO_SPRITE};

/// A particle cut from a material's sprite sheet (`mao_formats::gfx::MATERIALS`), `t` (0 to 1) of the way through the sheet's frames; a
/// material that is not known gives a soft dot.
fn sprite(material: i32, t: f32, position: [f32; 3], size: f32, colour: [f32; 4]) -> Particle {
    match mao_formats::gfx::material(material) {
        Some((_, cols, rows, first, last)) => {
            let frames = (last.saturating_sub(first)) as f32 + 1.0;
            let frame = first as u32 + (t.clamp(0.0, 0.999) * frames).floor() as u32;
            let (col, row) = (frame % cols.max(1) as u32, frame / cols.max(1) as u32);
            let (du, dv) = (1.0 / cols.max(1) as f32, 1.0 / rows.max(1) as f32);
            Particle { position, size, colour, sprite: material as u16, rect: [col as f32 * du, row as f32 * dv, du, dv] }
        }
        None => Particle { position, size, colour, sprite: NO_SPRITE, rect: [0.0; 4] },
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
            let t = age / FLIGHT;
            let at = [0, 1, 2].map(|i| self.from[i] + (self.to[i] - self.from[i]) * t);
            out.extend(visual_particles(tracer, [at[0], at[1], at[2] - 0.25], age, Reach::Small));
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

/// Where attach point 1006 (the placeholders' word 7) sits on a standing character, metres above the feet: a guess that puts the
/// ring (its offsets are -0.1 to -0.5 m from the point) just over the head, as the recording shows.
const ATTACH_1006: f32 = 2.1;
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
/// (world, Z up) and whose body is `(breed, sex, fatness)` when known. `Visual::Spiral` is the kind-1001 pair of placeholders; the shield
/// and the rest are stood in for.
pub fn pulse_particles(v: &Visual, feet: [f32; 3], age: f32, body: Option<(i32, i32, i32)>) -> Vec<Particle> {
    let mut out = Vec::new();
    match v {
        Visual::Spiral(aura) => {
            for helix in &aura.helices {
                if age >= helix.life {
                    continue;
                }
                let (offset, radius) = body.map_or((helix.offset, helix.radius), |(b, s, f)| helix.fit(b, s, f));
                ride(&mut out, feet, feet[2] + ATTACH_1006 + offset, radius.max(0.03), helix.phase, helix.spin, age, helix.life, aura.colour);
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
    fn a_pulse_rides_a_ring_at_the_body_s_place() {
        let helix = mao_formats::gfx::Helix { spin: 12.6, phase: 0.0, size: 0.13, count: 10, colour: [0.5, 0.4, 1.0], material: 15, offset: -0.1, radius: 0.13, table: vec![(-0.5, 0.2); 24], life: 1.5 };
        let aura = Visual::Spiral(mao_formats::gfx::Aura { colour: [0.99, 0.99, 0.2], end: [0.99, 0.99, 0.2], helices: vec![helix], period: 4.0, plays: -1 });
        // without a body: the template's own place; with one: the table's
        let own = pulse_particles(&aura, [0.0, 0.0, 0.0], 0.8, None);
        assert!(!own.is_empty() && own.iter().all(|p| (p.position[2] - (ATTACH_1006 - 0.1)).abs() < 0.06));
        let fitted = pulse_particles(&aura, [0.0, 0.0, 0.0], 0.8, Some((1, 2, 1)));
        assert!(fitted.iter().all(|p| (p.position[2] - (ATTACH_1006 - 0.5)).abs() < 0.06));
        // the trail grows with the pulse, and nothing is drawn past its life
        assert!(pulse_particles(&aura, [0.0, 0.0, 0.0], 0.2, None).len() < own.len());
        assert!(pulse_particles(&aura, [0.0, 0.0, 0.0], 1.6, None).is_empty());
        assert_eq!(pulse_timing(&aura), (4.0, 1.5, -1));
    }
    use mao_formats::gfx::Helix;

    fn aura() -> Aura {
        let helix = |spin: f32| Helix { spin, phase: 0.0, size: 0.15, count: 10, colour: [0.5, 0.4, 1.0], material: 15, offset: -0.25, radius: 0.15, table: Vec::new(), life: 1.5 };
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
