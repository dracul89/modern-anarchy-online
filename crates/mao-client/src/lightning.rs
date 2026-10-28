//! The lightning's light: what a strike of the rain's lightning (`ambience::Lightning`) shows, as DisplaySystem's environment
//! effects draw it (ported in `AnarchyReimagined`'s `dropin/displaysystem`: `fxcloud.rs`, `fxlightning.rs`, `envfx.rs`).
//!
//! Gamecode's FUN_100afc4c starts the strike's effect 8, `VisualCloudlight_t`, at it (the place 0.2 lower, kept above the
//! ground; its ground place 70 below, put on the ground), seeded with the strike's word. The cloud light lays ten square flashes
//! across the sky there ([`Cloud`]: the first 55 either way, the others scattered by its R250 generator) and starts effect 9,
//! `VisualLightning_t`, a bolt ([`Bolt`]) from its place to the ground, and another at each scattered flash four times in
//! ten. FUN_100afc4c runs the started effects in turn, each put on the ground again. Both are drawn added to the scene
//! (blended, `D3DBLEND_ONE`, no z writes), the cloud light with `dustcloud.png` (the environment's texture 7) and the bolt with
//! `newsun_frame01.png` (texture 0).
//!
//! Each frame a bolt lives it starts effects 0x2d and 0x2c at its tip (force 100), which the environment's frame runs straight
//! after (FUN_100b05b4: `ReadyNextFX` → FUN_100afc4c). 0x2d, out of the water and over force 80, throws a rock ([`Rock`]):
//! effect 0xb to 0x11 at random (`rand() % 7`, `rock01` to `rock07.abiff`, `VisualTinyRock_t`) at force 5, flown by Gamecode's
//! `EnvFXGravityObject_t` (FUN_100b103b made, FUN_100b0d0f each frame). 0x2c is gfx effect 0x859b, a `_GfxControlSplash_t`,
//! which only makes its cones over a liquid (FUN_100f5a77: `GetLiquidSurfaceHeight`).
//!
//! Not here: the splash (the client knows no liquids, so it would make none either way), FUN_100b8b35's note of the bolt's
//! ground place (radius 50), the water level in the ground's height (a rock never enters the water), and the pools: each strike
//! gets its own effects, so none is ever refused.

use crate::ambience::R250;
use crate::audio::Rand;
use mao_render::{Shell, ShellVertex};

/// A cloud light (`VisualCloudlight_t`): its place (Y up), ten flashes of five vertices (four corners round a centre, local),
/// the centres' colours, its age (from 1, lit while above 0.01).
#[derive(Clone, Debug)]
pub struct Cloud {
    at: [f32; 3],
    flashes: [[[f32; 3]; 5]; 10],
    colours: [[u8; 3]; 10],
    age: f32,
    lit: bool,
}

/// A bolt (`VisualLightning_t`): its place (Y up), its segments' crosses (four vertices each, local), its age (`rand() % 500
/// / 1000`) and whether it has been lit.
#[derive(Clone, Debug)]
pub struct Bolt {
    at: [f32; 3],
    crosses: Vec<[[f32; 3]; 4]>,
    age: f32,
    lit: bool,
}

/// A thrown rock (`EnvFXGravityObject_t`, 0x50 bytes, over a `VisualTinyRock_t`): its mesh, place and velocity (Y up), the
/// axis it turns about, its spin and angle, the bounces left (7) and whether it is done.
#[derive(Clone, Debug)]
pub struct Rock {
    mesh: &'static str,
    at: [f32; 3],
    velocity: [f32; 3],
    axis: [f32; 3],
    spin: f32,
    angle: f32,
    bounces: i32,
    done: bool,
}

/// `EnvFXGravityObject_t`'s constants: the throw's force (+0x1a0 set to 5, DAT_101584fc), the bounce's divisor and the drag
/// in water (+0x44 and +0x4c, 4), the fall (+0x48, 9.8 a second a second).
const ROCK_FORCE: f32 = 5.0;
const ROCK_BOUNCE: f32 = 4.0;
const ROCK_FALL: f32 = 9.8;

/// The strikes' effects under way, and the CRT's `rand` they draw from.
#[derive(Default)]
pub struct Sky {
    rand: Rand,
    clouds: Vec<Cloud>,
    bolts: Vec<Bolt>,
    rocks: Vec<Rock>,
}

fn normalized(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0.0 { v.map(|c| c / l) } else { v }
}

/// The ground's normal under (x, z) (Y up), from its heights either side: what the ground probe (FUN_100ada15) gives back.
fn ground_normal(ground: &dyn Fn(f32, f32) -> Option<f32>, x: f32, z: f32) -> [f32; 3] {
    let e = 0.5;
    match (ground(x - e, z), ground(x + e, z), ground(x, z - e), ground(x, z + e)) {
        (Some(a), Some(b), Some(c), Some(d)) => normalized([a - b, 2.0 * e, c - d]),
        _ => [0.0, 1.0, 0.0],
    }
}

/// A flash's corners round (`x`, `z`), half-side `s`, and its centre.
fn flash(x: i32, z: i32, s: i32) -> [[f32; 3]; 5] {
    let f = |a: i32| a as f32;
    [[f(x - s), 0.0, f(z - s)], [f(x + s), 0.0, f(z - s)], [f(x - s), 0.0, f(z + s)], [f(x + s), 0.0, f(z + s)], [f(x), 0.0, f(z)]]
}

impl Sky {
    /// A strike lit at `at` (Y up) with the strike's `seed`; `ground` the ground's height under a point (x, z), if any.
    pub fn strike(&mut self, at: [f32; 3], seed: u32, ground: &dyn Fn(f32, f32) -> Option<f32>) {
        // (FUN_100afc4c for each effect: the place 0.2 lower and kept above the ground, FUN_100ad98f; the ground place on the
        // ground, FUN_100ad8f8)
        let settle = |mut at: [f32; 3], mut low: [f32; 3]| {
            at[1] -= 0.2;
            if let Some(h) = ground(at[0], at[2]) {
                at[1] = at[1].max(h);
            }
            if let Some(h) = ground(low[0], low[2]) {
                low[1] = h;
            }
            (at, low)
        };
        let (place, low) = settle(at, [at[0], at[1] - 70.0, at[2]]);
        let cloud = self.cloud(place);
        let mut bolts = vec![(place, low)];
        // (FUN_100508bd's activation: the scattered flashes, each four times in ten a bolt of its own)
        let mut r = R250::new(seed);
        let mut draw = || r.next() as i32;
        let mut flashes = cloud.flashes;
        for f in flashes.iter_mut().skip(1) {
            let x = draw() % 0x46 - 0x23;
            let z = draw() % 0x46 - 0x23;
            let s = draw() % 0x14 + 0x19;
            *f = flash(x, z, s);
            if draw() % 100 < 0x28 {
                let off = [x as f32, 0.0, z as f32];
                bolts.push(([place[0] + off[0], place[1], place[2] + off[2]], [low[0] + off[0], low[1], low[2] + off[2]]));
            }
        }
        self.clouds.push(Cloud { flashes, ..cloud });
        for (from, to) in bolts {
            let (from, to) = settle(from, to);
            let bolt = self.bolt(from, to);
            self.bolts.push(bolt);
        }
    }

    /// FUN_100516ce (setup) and the activation's first flash: the shapes from `rand` (all but the first replaced on
    /// activation), the centres' colours, `(B, G, R)` = `(0, 255 − r, 255)` a third of the time, else `(255, 255 − r, 200 −
    /// r)`, `r` a `rand() & 0x7f`.
    fn cloud(&mut self, at: [f32; 3]) -> Cloud {
        let mut flashes = [[[0.0; 3]; 5]; 10];
        flashes[0] = flash(0, 0, 55);
        for f in flashes.iter_mut().skip(1) {
            let x = self.rand.next() % 0x46 - 0x23;
            let z = self.rand.next() % 0x46 - 0x23;
            let s = self.rand.next() % 0x14 + 0x19;
            *f = flash(x, z, s);
        }
        let mut colours = [[0u8; 3]; 10];
        for c in colours.iter_mut() {
            *c = if self.rand.next() % 3 == 0 {
                [0, 0xff - (self.rand.next() as u8 & 0x7f), 0xff]
            } else {
                let g = 0xff - (self.rand.next() as u8 & 0x7f);
                [0xff, g, 0xc8u8.wrapping_sub(self.rand.next() as u8 & 0x7f)]
            };
        }
        Cloud { at, flashes, colours, age: 1.0, lit: false }
    }

    /// FUN_10052182 (setup: `rand() % 10 + 5` segments) and FUN_10051bca (activation): the life, then each segment's cross
    /// along the way, jittered by `rand` times its share of the way and its width (2, narrowing by `2 / n`).
    fn bolt(&mut self, at: [f32; 3], ground: [f32; 3]) -> Bolt {
        let n = self.rand.next() % 10 + 5;
        let age = ((self.rand.next() % 500) as f64 / 1000.0) as f32;
        let way = [ground[0] - at[0], ground[1] - at[1], ground[2] - at[2]];
        let d = way.map(|c| c / (n - 1) as f32);
        let inc = 2.0 / n as f32;
        let (mut p, mut t, mut width) = ([0.0f32; 3], 0.0f32, 2.0f32);
        let mut crosses = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let o = self.offset(4000, 2.0);
            let q = o.map(|c| c * t * width);
            crosses.push([[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 0.0, -0.5], [0.0, 0.0, 0.5]].map(|c: [f32; 3]| [p[0] + q[0] + c[0], p[1] + q[1] + c[1], p[2] + q[2] + c[2]]));
            for c in 0..3 {
                p[c] += d[c];
            }
            t += inc;
            width -= inc;
        }
        Bolt { at, crosses, age, lit: false }
    }

    /// A random offset across the ground: `rand() % range / 1000 − half` across, then along.
    fn offset(&mut self, range: i32, half: f64) -> [f32; 3] {
        let mut f = || ((self.rand.next() % range) as f64 / 1000.0 - half) as f32;
        let x = f();
        let z = f();
        [x, 0.0, z]
    }

    /// FUN_100b103b: a rock thrown from `at` at the force's speed (`rand() % 1000 / 1000 × 3 − 1.5 + 5`, at least 0.3) in a
    /// random direction (`rand() % 10000 − 5000`, `rand() % 10000`, `rand() % 10000 − 5000`, made a unit), turning about the
    /// up direction crossed with its way at `rand() % 4000 / 1000 + 2` radians a second; 7 bounces.
    fn throw(&mut self, mesh: &'static str, at: [f32; 3]) {
        let speed = ((self.rand.next() % 1000) as f32 / 1000.0 * 3.0 - 1.5 + ROCK_FORCE).max(0.3);
        let dx = (self.rand.next() % 10000 - 5000) as f32 / 1000.0;
        let dy = (self.rand.next() % 10000) as f32 / 1000.0;
        let dz = (self.rand.next() % 10000 - 5000) as f32 / 1000.0;
        let mut velocity = normalized([dx, dy, dz]).map(|c| c * speed);
        if velocity[0] == 0.0 && velocity[2] == 0.0 {
            velocity[0] = 0.1;
        }
        let axis = normalized([velocity[2], 0.0, -velocity[0]]);
        let spin = (self.rand.next() % 4000) as f32 / 1000.0 + 2.0;
        self.rocks.push(Rock { mesh, at, velocity, axis, spin, angle: 0.0, bounces: 7, done: false });
    }

    /// The rocks thrown and still flying, for the viewer to draw (Z up).
    pub fn rocks(&self) -> Vec<crate::effects::Rock> {
        let z_up = |v: [f32; 3]| [v[0], v[2], v[1]];
        self.rocks.iter().map(|r| crate::effects::Rock { mesh: r.mesh, at: z_up(r.at), axis: z_up(r.axis), angle: r.angle.rem_euclid(std::f32::consts::TAU) }).collect()
    }

    /// A frame of `dt` seconds (`ground` the ground's height under a point, Y up): the rocks fly (FUN_100b0d0f: their turn on
    /// by the spin, done past 2000; on by their velocity; set on the ground when below it, which bounces them: a bounce
    /// spent, the velocity divided by `4 × normal.y + 4` and given `|vy| / 4` along the normal, moved on once more, the spin
    /// divided by 1.5; then they fall), the cloud lights age (FUN_10050bc3: lit while above 0.01), the bolts age and jitter
    /// more (FUN_10051e5a: each segment by `offset(16) × share × width × dt`; gone at 0) and each throws a rock from its tip
    /// (effect 0x2d's run, then the rock's: each put 0.2 lower and kept above the ground).
    pub fn step(&mut self, dt: f32, ground: &dyn Fn(f32, f32) -> Option<f32>) {
        for r in &mut self.rocks {
            r.angle += dt * r.spin;
            if 2000.0 < r.angle {
                r.done = true;
            }
            let mut at = [0, 1, 2].map(|c| r.at[c] + r.velocity[c] * dt);
            let before = at[1];
            let mut normal = [0.0, 1.0, 0.0];
            if let Some(h) = ground(at[0], at[2]) {
                if at[1] < h {
                    at[1] = h;
                    normal = ground_normal(ground, at[0], at[2]);
                }
            }
            if before < at[1] {
                r.bounces -= 1;
                if r.bounces == 0 {
                    r.done = true;
                } else {
                    let up = r.velocity[1].abs() / ROCK_BOUNCE;
                    let k = normal[1] * ROCK_BOUNCE + ROCK_BOUNCE;
                    for c in 0..3 {
                        r.velocity[c] = r.velocity[c] / k + normal[c] * up;
                        at[c] += r.velocity[c] * dt;
                    }
                    r.spin /= 1.5;
                }
            }
            r.at = at;
            r.velocity[1] -= ROCK_FALL * dt;
        }
        self.rocks.retain(|r| !r.done);
        for c in &mut self.clouds {
            c.age -= dt;
            c.lit = 0.01 < c.age;
        }
        self.clouds.retain(|c| c.lit);
        let mut bolts = std::mem::take(&mut self.bolts);
        bolts.retain_mut(|b| {
            b.age -= dt;
            if !(0.0 < b.age) {
                return false;
            }
            let n = b.crosses.len();
            let inc = 2.0 / n as f32;
            let (mut t, mut width) = (0.0f32, 2.0f32);
            for k in 0..n {
                let o = self.offset(16000, 8.0);
                let q = o.map(|c| c * t * width * dt);
                for v in &mut b.crosses[k] {
                    for c in 0..3 {
                        v[c] += q[c];
                    }
                }
                t += inc;
                width -= inc;
            }
            b.lit = true;
            true
        });
        // (each live bolt's tip, its last segment's fourth vertex, starts 0x2d: a rock, thrown from it twice settled)
        let tips: Vec<[f32; 3]> = bolts.iter().filter_map(|b| b.crosses.last().map(|c| [b.at[0] + c[3][0], b.at[1] + c[3][1], b.at[2] + c[3][2]])).collect();
        self.bolts = bolts;
        let settle = |mut p: [f32; 3]| {
            p[1] -= 0.2;
            if let Some(h) = ground(p[0], p[2]) {
                p[1] = p[1].max(h);
            }
            p
        };
        for tip in tips {
            let kind = 0xb + self.rand.next() % 7;
            let at = settle(settle(tip));
            if let Some(mesh) = mao_formats::gfx::rock_mesh(kind) {
                self.throw(mesh, at);
            }
        }
    }

    /// The shells to draw (Z up): each lit cloud light's flashes (`bolt_sheet`, `cloud_sheet` the textures' sheets), their
    /// centres' alpha `(255 − (rand() & 0x40)) × (1 − (1 − age)³)`, the corners clear; each lit bolt's crosses joined segment
    /// to segment, its colour (B, G, R) (255, 255, 200), clear at its first and last segments.
    pub fn shells(&mut self, bolt_sheet: u16, cloud_sheet: u16) -> Vec<Shell> {
        let world = |at: [f32; 3], v: [f32; 3]| [at[0] + v[0], at[2] + v[2], at[1] + v[1]];
        let mut out = Vec::new();
        for c in &self.clouds {
            if !c.lit {
                continue;
            }
            let t = 1.0 - c.age;
            let level = 1.0 - t * t * t;
            let mut vertices = Vec::new();
            for (f, bgr) in c.flashes.iter().zip(c.colours) {
                let a = ((0xff - (self.rand.next() & 0x40)) as f32 * level) as i32 as f32 / 255.0;
                let centre = ShellVertex { position: world(c.at, f[4]), uv: [0.5, 0.5], colour: [bgr[2] as f32 / 255.0, bgr[1] as f32 / 255.0, bgr[0] as f32 / 255.0, a] };
                let corner = |k: usize, uv: [f32; 2]| ShellVertex { position: world(c.at, f[k]), uv, colour: [0.0; 4] };
                let corners = [corner(0, [1.0, 1.0]), corner(1, [0.0, 1.0]), corner(2, [1.0, 0.0]), corner(3, [0.0, 0.0])];
                for (a, b) in [(0, 1), (1, 3), (3, 2), (2, 0)] {
                    vertices.extend([corners[a], corners[b], centre]);
                }
            }
            out.push(Shell { sprite: cloud_sheet, additive: true, vertices });
        }
        for b in &self.bolts {
            if !b.lit {
                continue;
            }
            let n = b.crosses.len();
            let vertex = |k: usize, c: usize| {
                let alpha = if k == 0 || k + 1 == n { 0.0 } else { 1.0 };
                ShellVertex { position: world(b.at, b.crosses[k][c]), uv: [(c % 2) as f32, 0.65], colour: [200.0 / 255.0, 1.0, 1.0, alpha] }
            };
            let mut vertices = Vec::new();
            for k in 0..n.saturating_sub(1) {
                for (s, c) in [(0, 0), (0, 1), (1, 0), (0, 1), (1, 1), (1, 0), (0, 2), (0, 3), (1, 2), (0, 3), (1, 3), (1, 2)] {
                    vertices.push(vertex(k + s, c));
                }
            }
            out.push(Shell { sprite: bolt_sheet, additive: true, vertices });
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.clouds.is_empty() && self.bolts.is_empty() && self.rocks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strike_lights_the_sky_and_bolts_to_the_ground() {
        let mut sky = Sky::default();
        sky.strike([100.0, 50.0, 200.0], 0x1234, &|_, _| Some(10.0));
        // (the first bolt from the strike, 0.2 lower twice (the cloud light's place, then the bolt's own run), to the ground at
        // 10, its last segment there)
        let first = &sky.bolts[0];
        assert_eq!(first.at, [100.0, 49.6, 200.0]);
        let end = first.crosses.last().unwrap()[0];
        assert!((end[1] - (10.0 - 49.6)).abs() < 1e-3);
        assert!(first.crosses.len() >= 5 && first.crosses.len() < 15);
        // (nothing drawn before the first frame; then the flashes and the bolts that still live)
        assert!(sky.shells(1, 2).is_empty());
        let ground = |_: f32, _: f32| Some(10.0);
        sky.step(0.016, &ground);
        // (each live bolt threw a rock from its tip, above the ground)
        assert_eq!(sky.rocks.len(), sky.bolts.len());
        assert!(sky.rocks.iter().all(|r| r.at[1] >= 10.0 && r.mesh.starts_with("rock0")));
        let shells = sky.shells(1, 2);
        assert!(shells.iter().any(|s| s.sprite == 2 && s.vertices.len() == 10 * 12));
        // (the cloud light fades within its second; the bolts within half a second)
        for _ in 0..32 {
            sky.step(0.016, &ground);
        }
        assert!(sky.bolts.is_empty() && !sky.clouds.is_empty());
        for _ in 0..40 {
            sky.step(0.016, &ground);
        }
        assert!(sky.clouds.is_empty() && sky.bolts.is_empty());
        // (the rocks bounce on the ground, never through it, and come to an end)
        for _ in 0..2000 {
            sky.step(0.016, &ground);
            assert!(sky.rocks.iter().all(|r| r.at[1] >= 10.0 - 1e-3));
        }
        assert!(sky.is_empty());
    }
}
