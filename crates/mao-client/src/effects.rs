//! Nano effects drawn as glowing sprites in the world (the renderer's particle pass). The templates come from `gfxtweak.bin`
//! (`mao_formats::gfx`); what is decoded so far is the persistent effect of a running buff (kind 1001): two spirals of sparks that
//! turn the opposite ways round the character and climb from its feet, in the buff's colour.
//!
//! What the data gives: the colour, each spiral's spin (about two turns a second), its start angle, spark size and count. What it
//! does not say (or is not read yet): the radius (the radius path's numbers, 0.06 to 0.15, are too small for a body; 0.42 m is used),
//! how high and how long a spark lives (1.9 m and 1.6 s here), and the trail (a short tail of fading sparks behind each one).

use mao_formats::gfx::Aura;
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
    let mut out = Vec::new();
    for helix in &aura.helices {
        // (the spirals' sparks start at the colour of the buff and warm toward the helix's own light colour)
        let n = (helix.count as usize).clamp(1, 24);
        for i in 0..n {
            let age = (time / LIFE + i as f32 / n as f32).fract();
            let height = age * HEIGHT;
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
                let radius = RADIUS * (1.0 - 0.25 * age);
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
}
