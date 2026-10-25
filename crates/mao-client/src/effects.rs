//! Nano effects drawn as glowing sprites in the world (the renderer's particle pass). The templates come from `gfxtweak.bin`
//! (`mao_formats::gfx`); what is decoded so far is the persistent effect of a running buff (kind 1001): two spirals of sparks that
//! turn the opposite ways round the character and climb from its feet, in the buff's colour.
//!
//! What the data gives: the colour, each spiral's spin (about two turns a second), its start angle, spark size and count. What it
//! does not say (or is not read yet): the radius (the radius path's numbers, 0.06 to 0.15, are too small for a body; 0.42 m is used),
//! how high and how long a spark lives (1.9 m and 1.6 s here), and the trail (a short tail of fading sparks behind each one).

use mao_formats::gfx::{Aura, Emitter};
use mao_render::Particle;

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
    pub tracer: Option<Aura>,
    pub hit: Option<Aura>,
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
            out.extend(aura_shaped(tracer, [at[0], at[1], at[2] - 0.25], age, 0.5, 0.18));
        }
        let landed = if travelling { age - FLIGHT } else { age };
        if let (Some(hit), true) = (&self.hit, landed >= 0.0 && landed < LANDING) {
            let fade = (((LANDING - landed) / 0.4).min(1.0)).max(0.0) * (landed / 0.1).min(1.0);
            for mut p in aura_shaped(hit, [self.to[0], self.to[1], self.to[2] - 1.0], landed, 1.5, 0.35) {
                p.colour[3] *= fade;
                out.push(p);
            }
        }
        out
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
                out.push(Particle {
                    position: [feet[0] + radius * angle.cos(), feet[1] + radius * angle.sin(), feet[2] + height - 0.04 * k as f32],
                    size: helix.size.max(0.05) * (0.45 + 0.55 * fade) * 0.85,
                    colour: [c[0], c[1], c[2], 0.75 * strength * fade * fade],
                });
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
        out.push(Particle {
            position: [
                origin[0] + direction[0] * speed * a + jitter(4),
                origin[1] + direction[1] * speed * a + jitter(5),
                origin[2] + direction[2] * speed * a + 0.5 * e.gravity * a * a + jitter(6),
            ],
            size: mix(e.size.0, e.size.1).max(0.04),
            colour,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mao_formats::gfx::Helix;

    fn aura() -> Aura {
        let helix = |spin: f32| Helix { spin, phase: 0.0, size: 0.15, count: 10, colour: [0.5, 0.4, 1.0] };
        Aura { colour: [0.2, 0.2, 0.99], end: [0.2, 0.2, 0.99], helices: vec![helix(-12.566), helix(12.566)] }
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

    fn emitter() -> Emitter {
        Emitter { id: 1, kind: 1005, start: [1.0, 0.5, 0.4, 1.0], end: [0.0, 0.5, 0.4, 1.0], size: (0.1, 0.2), life: 1.0, rate: 10.0, azimuth: 6.28, elevation: (-1.57, 1.57), speed: (0.5, 1.0), gravity: 0.0 }
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
