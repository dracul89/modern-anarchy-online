//! The weather's rain, snow and storms round the eye, as `Environment_t`'s frame (Gamecode FUN_100becbe) runs them.
//!
//! - **The rain or snow** is entry 0 of the weather table (the object at `Environment_t` +0x10, FUN_100b9da8: shown
//!   while the entry is above 0, drawn at the entry's value as its alpha; no preference switches it off). Its look
//!   follows entry 8 (FUN_100b98fe, the object's numbers from FUN_100b9775): above 4 rain, blue-grey (0.4, 0.5, 0.7),
//!   falling at 16.2 and taking 0.4 of the wind, at 0.75 of the alpha; at −0.5 or below snow, grey 0.8, falling at 2.2
//!   and taking all of the wind, at the whole alpha; between the two a blend. The colour is scaled by the light × 1.4,
//!   kept within 0.5 and 1.
//! - **The four storms** are entries 3 to 6 (FUN_100b9ba5 over the objects FUN_100b9b10 sets up): line clouds in the
//!   colours of their fogs, (0.6, 0.35, 0), (0.18, 0.15, 0.15), (0.35, 0, 0) and (0, 0.4, 0), with almost no fall
//!   (0.3; the dark one 0.6) and 0.9 of the wind, so they are blown, not falling; their alpha is the entry (at most 1)
//!   cubed × 0.45, times 0.318, 0.7, 0.718 and 0.318. They are drawn only with the `UseStormParticles` preference on.
//!   Each storm also **tints the screen** in its colour (`VisualFader_t`: a sheet in front of the eye) at
//!   `flicker × 0.55 × entry³`, the flicker falling by a factor of 1.07 a frame and, one frame in five, rising by 0.17
//!   up to 0.85. The tint does not follow the preference.
//! - **A line cloud** is DisplaySystem's `VisualRain_t` (the drop-in's `rain.rs`, difftested): 350 lines in a box of
//!   15 that hangs on the camera, from 5 behind the eye forward along the view, its corner first (the camera's
//!   rotation after an eighth turn back about y and one about x). Each frame every line's tail takes its head's place
//!   and the head moves by the wind × the line's own speed (0.1 to 0.9 by its number) less the fall, so a streak is
//!   one frame's way long; one side of one axis is wrapped a frame, in turn. The heads have the alpha, the tails
//!   none. The lines are drawn twice, the second time one up and one along z.
//!
//! Here the lines are thin sheets turned to the eye, about a picture element wide (the original draws real lines),
//! and the tint is a sheet at a tenth of the original's distance and size, so that nothing near stands in front of it.
//! Positions are the server's (y up); the vertices are given in the renderer's `[x, z, y]`.

use mao_render::{ShellVertex, NO_SPRITE};

type V3 = [f32; 3];

const LINES: usize = 350;
const SPREAD: f32 = 15.0;

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn unit(a: V3) -> V3 {
    let len = dot(a, a).sqrt();
    if len > 1e-6 { scale(a, 1.0 / len) } else { [0.0; 3] }
}

/// The camera's own axes in the world (right, up, forward) for a view along `forward`, with no roll.
pub fn camera_axes(forward: V3) -> [V3; 3] {
    let f = unit(forward);
    let mut right = unit(cross([0.0, 1.0, 0.0], f));
    if right == [0.0; 3] {
        right = [1.0, 0.0, 0.0];
    }
    [right, cross(f, right), f]
}

/// A line cloud's axes in the world: the camera's rotation after an eighth turn back about y and an eighth turn about
/// x (`q_product(q_product(q2, q1), rotation)` in `VisualRain_t::process`).
fn cloud_axes(camera: &[V3; 3]) -> [V3; 3] {
    let (s, c) = std::f32::consts::FRAC_PI_4.sin_cos();
    // (a point turned by −45° about y, then by 45° about x: the three axes' images)
    let local = [[c, -s * s, s * c], [0.0, c, s], [-s, -s * c, c * c]];
    local.map(|l| add(add(scale(camera[0], l[0]), scale(camera[1], l[1])), scale(camera[2], l[2])))
}

/// How a line cloud looks and moves: its colour, its fall (`+0x24`), the part of the wind it takes (`+0x38`) and
/// what its alpha is multiplied by (`+0x5bc`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub colour: V3,
    pub gravity: f32,
    pub wind: f32,
    pub alpha: f32,
}

/// The rain's look for the weather's entry 8 and the day's light (FUN_100b98fe).
pub fn rain_look(e8: f32, light: f32) -> Look {
    const RAIN: Look = Look { colour: [0.4, 0.5, 0.7], gravity: 16.2, wind: 0.4, alpha: 0.75 };
    const SNOW: Look = Look { colour: [0.8, 0.8, 0.8], gravity: 2.2, wind: 1.0, alpha: 1.0 };
    let mut look = if e8 > 4.0 {
        RAIN
    } else if e8 <= -0.5 {
        SNOW
    } else {
        let f = 1.0 - (e8 + 0.5) / 4.5;
        Look {
            colour: [0, 1, 2].map(|k| RAIN.colour[k] + f * (SNOW.colour[k] - RAIN.colour[k])),
            gravity: SNOW.gravity + (RAIN.gravity - SNOW.gravity) * (1.0 - f),
            wind: 0.6 * f + 0.4,
            alpha: (SNOW.alpha - RAIN.alpha) * f + RAIN.alpha,
        }
    };
    let lit = (light * 1.4).clamp(0.5, 1.0);
    look.colour = look.colour.map(|c| c * lit);
    look
}

/// The four storms' looks, for the weather's entries 3 to 6.
pub const STORMS: [Look; 4] = [
    Look { colour: [0.6, 0.35, 0.0], gravity: 0.3, wind: 0.9, alpha: 0.318 },
    Look { colour: [0.18, 0.15, 0.15], gravity: 0.6, wind: 0.9, alpha: 0.7 },
    Look { colour: [0.35, 0.0, 0.0], gravity: 0.3, wind: 0.9, alpha: 0.718 },
    Look { colour: [0.0, 0.4, 0.0], gravity: 0.3, wind: 0.9, alpha: 0.318 },
];

/// `VisualRain_t`: the lines' heads and tails in the cloud's own frame, their speeds, and the axis side to wrap next.
#[derive(Clone, Debug)]
pub struct LineCloud {
    heads: Vec<V3>,
    tails: Vec<V3>,
    speeds: Vec<f32>,
    phase: u32,
}

impl LineCloud {
    /// FUN_10052889: each head at `(rand() % 1000 / 500 − 1) × 15` on each axis, its tail half a unit above it.
    pub fn new(rand: &mut crate::audio::Rand) -> LineCloud {
        let heads: Vec<V3> = (0..LINES).map(|_| [0, 1, 2].map(|_| ((rand.next() % 1000) as f32 / 500.0 - 1.0) * SPREAD)).collect();
        let tails = heads.iter().map(|p| add(*p, [0.0, 0.5, 0.0])).collect();
        let speeds = (0..LINES).map(|k| k as f32 / LINES as f32 * 0.8 + 0.1).collect();
        LineCloud { heads, tails, speeds, phase: 0 }
    }

    /// `process`'s movement: `fall` (the world's, already times the frame's time) and `wind` (the same) turned into
    /// the cloud's frame; one side of one axis wrapped; every tail to its head and every head on.
    pub fn step(&mut self, axes: &[V3; 3], fall: V3, wind: V3) {
        let fall = [dot(fall, axes[0]), dot(fall, axes[1]), dot(fall, axes[2])];
        let wind = [dot(wind, axes[0]), dot(wind, axes[1]), dot(wind, axes[2])];
        let (axis, low) = ((self.phase / 2) as usize, self.phase % 2 == 0);
        let count = self.heads.len();
        for k in (0..count).rev() {
            let c = &mut self.heads[k][axis];
            if low {
                if *c < 0.0 {
                    *c += SPREAD;
                }
            } else if *c > SPREAD {
                *c -= SPREAD;
            }
            self.tails[k] = self.heads[k];
            // (the last line's speed goes with the first line)
            let speed = self.speeds[count - 1 - k];
            self.heads[k] = add(self.heads[k], sub(scale(wind, speed), fall));
        }
        self.phase = (self.phase + 1) % 6;
    }

    /// The lines as sheets for the renderer: from `eye`, the cloud's corner 5 behind it along the view; twice, the
    /// second time one up and one along z.
    pub fn sheets(&self, eye: V3, camera: &[V3; 3], axes: &[V3; 3], colour: V3, alpha: f32, out: &mut Vec<ShellVertex>) {
        // (the alpha is a byte in the original)
        let alpha = ((alpha.clamp(0.0, 1.0) * 255.0) as u8) as f32 / 255.0;
        if alpha <= 0.0 {
            return;
        }
        let corner = sub(eye, scale(camera[2], 5.0));
        let world = |l: V3, at: V3| add(at, add(add(scale(axes[0], l[0]), scale(axes[1], l[1])), scale(axes[2], l[2])));
        let vertex = |p: V3, a: f32| ShellVertex { position: [p[0], p[2], p[1]], uv: [0.5, 0.5], colour: [colour[0], colour[1], colour[2], a] };
        for at in [corner, add(corner, [0.0, 1.0, 1.0])] {
            for (head, tail) in self.heads.iter().zip(&self.tails) {
                let (h, t) = (world(*head, at), world(*tail, at));
                let to = sub(h, eye);
                // (nothing behind the eye is worth a sheet)
                if dot(to, camera[2]) <= 0.2 {
                    continue;
                }
                let side = unit(cross(sub(h, t), to));
                if side == [0.0; 3] {
                    continue;
                }
                // (about one picture element wide at a thousand of them from top to bottom)
                let s = scale(side, (dot(to, to).sqrt() * 0.0006).max(0.002));
                let (t0, t1, h0, h1) = (sub(t, s), add(t, s), sub(h, s), add(h, s));
                out.extend([vertex(t0, 0.0), vertex(t1, 0.0), vertex(h1, alpha), vertex(t0, 0.0), vertex(h1, alpha), vertex(h0, alpha)]);
            }
        }
    }
}

/// The weather's five line clouds and the four storms' flickers.
pub struct WeatherFx {
    rain: LineCloud,
    storms: [LineCloud; 4],
    flicker: [f32; 4],
    rand: crate::audio::Rand,
    last: Option<std::time::Instant>,
}

impl Default for WeatherFx {
    fn default() -> WeatherFx {
        let mut rand = crate::audio::Rand::default();
        let rain = LineCloud::new(&mut rand);
        let storms = [(); 4].map(|_| LineCloud::new(&mut rand));
        WeatherFx { rain, storms, flicker: [0.0; 4], rand, last: None }
    }
}

impl WeatherFx {
    /// One frame of the weather `v` seen from `eye` along `forward` (the server's frame, y up), blown by `wind` (a
    /// second's way) in the day's `light`: the lines' and the tints' sheets for a blended shell with no picture.
    /// `storm_lines`: the `UseStormParticles` preference.
    pub fn frame(&mut self, v: &[f32; 14], eye: V3, forward: V3, wind: V3, light: f32, storm_lines: bool) -> Vec<ShellVertex> {
        let now = std::time::Instant::now();
        let dt = self.last.replace(now).map_or(1.0 / 60.0, |t| now.duration_since(t).as_secs_f32());
        self.advance(v, eye, forward, wind, light, storm_lines, dt)
    }

    /// [`WeatherFx::frame`] for a frame that took `dt` seconds.
    pub fn advance(&mut self, v: &[f32; 14], eye: V3, forward: V3, wind: V3, light: f32, storm_lines: bool, dt: f32) -> Vec<ShellVertex> {
        let dt = dt.min(0.2);
        let camera = camera_axes(forward);
        let axes = cloud_axes(&camera);
        let mut out = Vec::new();
        // the rain or snow
        if v[0] > 0.0 {
            let look = rain_look(v[8], light);
            self.rain.step(&axes, [0.0, look.gravity * dt, 0.0], scale(wind, look.wind * dt));
            self.rain.sheets(eye, &camera, &axes, look.colour, v[0] * look.alpha, &mut out);
        }
        // the four storms: their lines, then each one's tint
        for (k, look) in STORMS.iter().enumerate() {
            let x = v[3 + k];
            if x <= 0.0 {
                continue;
            }
            let x = x.min(1.0);
            let cubed = x * x * x;
            if storm_lines {
                self.storms[k].step(&axes, [0.0, look.gravity * dt, 0.0], scale(wind, look.wind * dt));
                self.storms[k].sheets(eye, &camera, &axes, look.colour, cubed * 0.45 * look.alpha, &mut out);
            }
            let fade = self.flicker[k] * 0.55 * cubed;
            if fade >= 0.01 {
                tint(eye, &camera, look.colour, fade, &mut out);
            }
            if self.flicker[k] > 0.0 {
                self.flicker[k] /= 1.07;
            }
            if self.rand.next() % 100 < 20 {
                self.flicker[k] += 0.17;
                if self.flicker[k] > 0.85 {
                    self.flicker[k] = 0.85;
                }
            }
        }
        out
    }
}

/// `VisualFader_t`'s sheet: 15 by 15 at 5 in front of the eye in the original; here a tenth of both.
fn tint(eye: V3, camera: &[V3; 3], colour: V3, fade: f32, out: &mut Vec<ShellVertex>) {
    let alpha = ((fade.clamp(0.0, 1.0) * 255.0) as u8) as f32 / 255.0;
    let middle = add(eye, scale(camera[2], 0.5));
    let (r, u) = (scale(camera[0], 0.75), scale(camera[1], 0.75));
    let corner = |a: f32, b: f32| {
        let p = add(middle, add(scale(r, a), scale(u, b)));
        ShellVertex { position: [p[0], p[2], p[1]], uv: [0.5, 0.5], colour: [colour[0], colour[1], colour[2], alpha] }
    };
    out.extend([corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)]);
}

/// The sheets as a shell for the scene (none when there is nothing to draw).
pub fn shell(vertices: Vec<ShellVertex>) -> Option<mao_render::Shell> {
    (!vertices.is_empty()).then(|| mao_render::Shell { sprite: NO_SPRITE, additive: false, vertices })
}

/// `MAO_WEATHER=<entry>:<value>,…`: the weather table's entries set by hand, for a picture of the rain or a storm.
pub fn forced() -> Option<[f32; 14]> {
    let text = std::env::var("MAO_WEATHER").ok()?;
    let mut v = [0.0f32; 14];
    for (k, x) in text.split(',').filter_map(|p| p.split_once(':')).filter_map(|(k, x)| Some((k.trim().parse::<usize>().ok()?, x.trim().parse::<f32>().ok()?))) {
        if k < v.len() {
            v[k] = x;
        }
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rain's look by the weather's entry 8: rain above 4, snow at −0.5 and below, a blend between; dimmed by the
    /// light, never below a half.
    #[test]
    fn the_rains_look_follows_the_cold() {
        let rain = rain_look(5.0, 1.0);
        assert_eq!((rain.colour, rain.gravity, rain.wind, rain.alpha), ([0.4, 0.5, 0.7], 16.2, 0.4, 0.75));
        let snow = rain_look(-0.5, 1.0);
        assert_eq!((snow.colour, snow.gravity, snow.wind, snow.alpha), ([0.8, 0.8, 0.8], 2.2, 1.0, 1.0));
        // (half way: e8 = 1.75 gives f = 0.5)
        let sleet = rain_look(1.75, 1.0);
        assert!((sleet.gravity - 9.2).abs() < 1e-4 && (sleet.wind - 0.7).abs() < 1e-5 && (sleet.alpha - 0.875).abs() < 1e-5, "{sleet:?}");
        assert!((sleet.colour[0] - 0.6).abs() < 1e-5 && (sleet.colour[2] - 0.75).abs() < 1e-5);
        // (night: the colour at a half; a bright day: whole)
        assert!((rain_look(5.0, 0.0).colour[1] - 0.25).abs() < 1e-6);
        assert_eq!(rain_look(5.0, 0.8).colour, [0.4, 0.5, 0.7]);
    }

    /// A line cloud: 350 lines whose heads fall by the world's down whatever way the camera looks, each tail where
    /// its head was, and the heads kept in the box by the wrap.
    #[test]
    fn the_lines_fall_in_the_world_and_stay_in_their_box() {
        let mut rand = crate::audio::Rand::default();
        let mut cloud = LineCloud::new(&mut rand);
        assert_eq!((cloud.heads.len(), cloud.speeds[0], cloud.tails[7]), (350, 0.1, add(cloud.heads[7], [0.0, 0.5, 0.0])));
        assert!(cloud.heads.iter().all(|p| p.iter().all(|c| c.abs() <= SPREAD)));
        // (the camera looks north-east and down)
        let camera = camera_axes([0.6, -0.5, 0.62]);
        let axes = cloud_axes(&camera);
        // (the cloud's axes are a turn: unit and square to one another; its diagonal lies along the view)
        for a in 0..3 {
            for b in 0..3 {
                assert!((dot(axes[a], axes[b]) - if a == b { 1.0 } else { 0.0 }).abs() < 1e-5);
            }
        }
        let diagonal = unit(add(add(axes[0], axes[1]), axes[2]));
        assert!(dot(diagonal, camera[2]) > 0.98, "{}", dot(diagonal, camera[2]));
        let before = cloud.heads.clone();
        cloud.step(&axes, [0.0, 0.27, 0.0], [0.0; 3]);
        let world = |l: V3| add(add(scale(axes[0], l[0]), scale(axes[1], l[1])), scale(axes[2], l[2]));
        for k in 0..350 {
            // (the first step wraps x from below: a head that was wrapped moved by the box's side as well)
            let wrapped = before[k][0] < 0.0;
            let was = if wrapped { add(before[k], [SPREAD, 0.0, 0.0]) } else { before[k] };
            assert_eq!(cloud.tails[k], was);
            let moved = world(sub(cloud.heads[k], was));
            assert!(moved[0].abs() < 1e-4 && (moved[1] + 0.27).abs() < 1e-4 && moved[2].abs() < 1e-4, "{moved:?}");
        }
        // (after many frames of rain every head is inside the box, give or take the six frames' fall between two
        // wraps of the same side)
        for _ in 0..600 {
            cloud.step(&axes, [0.0, 0.27, 0.0], [0.01, 0.0, 0.02]);
        }
        assert!(cloud.heads.iter().all(|p| p.iter().all(|c| (-2.0..=SPREAD + 2.0).contains(c))));
    }

    /// Clear weather draws nothing; rain draws its lines twice, heads with the alpha and tails with none; a storm's
    /// lines wait for the preference, its tint does not.
    #[test]
    fn the_weather_draws_its_lines_and_tints() {
        let mut fx = WeatherFx::default();
        let (eye, forward) = ([100.0, 20.0, 200.0], [0.0, 0.0, 1.0]);
        let mut v = [0.0f32; 14];
        assert!(fx.advance(&v, eye, forward, [0.0; 3], 1.0, true, 0.016).is_empty());
        v[0] = 0.8;
        v[8] = 5.0;
        let rain = fx.advance(&v, eye, forward, [1.0, 0.0, 0.0], 1.0, true, 0.016);
        assert!(!rain.is_empty() && rain.len() % 6 == 0 && rain.len() <= 350 * 2 * 6);
        // (0.8 × 0.75 as a byte)
        let head = rain.iter().map(|p| p.colour[3]).fold(0.0, f32::max);
        assert!((head - 0.6).abs() < 0.005 && rain.iter().any(|p| p.colour[3] == 0.0));
        assert!(rain.iter().all(|p| p.colour[..3] == [0.4, 0.5, 0.7]));
        // (a storm with its lines switched off: nothing until the flicker has risen, then only the tint's two triangles)
        let mut storm = [0.0f32; 14];
        storm[5] = 1.0;
        let mut fx = WeatherFx::default();
        let mut most = 0;
        for _ in 0..200 {
            let sheets = fx.advance(&storm, eye, forward, [0.0; 3], 1.0, false, 0.016);
            assert!(sheets.is_empty() || sheets.len() == 6);
            most = most.max(sheets.len());
            assert!(sheets.iter().all(|p| p.colour[..3] == [0.35, 0.0, 0.0] && p.colour[3] <= 0.85 * 0.55 + 1e-6));
        }
        assert_eq!(most, 6);
        assert!(fx.flicker[2] > 0.0 && fx.flicker[2] <= 0.85 && fx.flicker[0] == 0.0);
        // (with them on: lines too)
        assert!(fx.advance(&storm, eye, forward, [2.0, 0.0, 0.0], 1.0, true, 0.016).len() > 6);
    }

    #[test]
    fn the_cameras_axes_are_right_up_and_forward() {
        assert_eq!(camera_axes([0.0, 0.0, 2.0]), [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        // (looking east, north is to the left)
        let east = camera_axes([1.0, 0.0, 0.0]);
        assert_eq!(east[0], [0.0, 0.0, -1.0]);
        // (straight down: still three axes)
        assert_eq!(camera_axes([0.0, -1.0, 0.0])[0], [1.0, 0.0, 0.0]);
    }
}
