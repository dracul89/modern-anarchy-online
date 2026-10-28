//! DisplaySystem's single clouds (`VisualSingleCloud_t`, the `e_SingleCloud` effect: `AnarchyReimagined`'s
//! `dropin/displaysystem` `fxsinglecloud.rs`): a field of 30 cloud puffs round the camera at the script's `Altitude`.
//!
//! - Made (FUN_100532b8): each puff's drift speed (`rand % 10000 / 10000 × 0.5 + 0.6`) and size (a factor
//!   `(1 − q²) × 5 + 1`, its width and depth `(2t + 1) × 100` times it).
//! - Its mesh (FUN_10053681, the realistic clouds: 10 by 10): four rows of sine heights (`(rand & 3) + 1` half turns
//!   across the row, `0.1`–`0.2` deep), each vertex across the grid scaled by the rows its side takes, flat, with a random
//!   mostly level normal; texture coordinates across the grid inset 0.5%; a light factor `1 ± 0.1`.
//! - Each frame (FUN_1005485a): the camera's level move taken (a jump over 800 scatters every puff); each puff moves
//!   against the move by a tenth (the high wind's drift is not here: its scale, the environment's +0x44/+0x48, is not
//!   found), is reshaped and placed again across the 3200-wide box when it leaves it (FUN_10053e45: wrapped to the far side,
//!   placed apart from the others on the other axis), sinks with its distance from the altitude (`altitude × −1.5 × t²`,
//!   `t` its distance over 1500 up to 1), tilts away from the middle by a quarter of `t²`, fades with the storm (`1 −
//!   storm²`) and takes a light level from the suns' height and the clouds (`((b − 0.14) × (1 − clouds × 0.9) + 0.14) ×`
//!   its light factor, `b` the suns' direction's height plus 0.89 within 0.14 and 1; by 200).
//! - Drawn (FUN_10054e07): the ground light at 0.55; puffs 0–1, 2–3, 4–26 and 27–29 with the materials of the environment's
//!   textures 9 to 12 (`single_cloud01.png` to `04`), each emissive in its first puff's light; their vertices' colour that
//!   light level too, lit by the ground light (D3D's vertex colour as the diffuse), alpha the puff's fade; blended.

use super::math::{self, Q4, V3};
use super::visuals::{Light, Vert};

const PUFFS: usize = 30;
const SIDE: usize = 10;

#[derive(Clone, Debug, Default)]
struct Puff {
    place: V3,
    speed: f32,
    width: f32,
    depth: f32,
    heights: Vec<f32>,
    normals: Vec<V3>,
    light_factor: f32,
}

/// The field.
pub struct Clouds {
    centre: V3,
    puffs: Vec<Puff>,
    rand: crate::audio::Rand,
}

/// The frame's inputs: the camera, the suns' direction (`GameSunDirection`), the storm and the clouds (the frame block's
/// +0x14, +0xf8, +0xf0, +0x114) and the ground light.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sky {
    pub camera: V3,
    pub sun_direction: V3,
    pub storm: f32,
    pub thick_clouds: f32,
    pub ground: Option<Light>,
}

impl Clouds {
    fn r10k(&mut self) -> f32 {
        (self.rand.next() % 10000) as f32
    }

    fn size_factor(&mut self) -> f32 {
        let q = self.r10k() / 10000.0;
        (1.0 - q * q) * 5.0 + 1.0
    }

    fn spread(&mut self, factor: f32) -> f32 {
        let t = self.r10k() / 10000.0;
        (t + t + 1.0) * 100.0 * factor
    }

    fn light_factor(&mut self) -> f32 {
        (self.r10k() / 5000.0 - 1.0) * 0.1 + 1.0
    }

    fn jitter(&mut self) -> V3 {
        let x = (self.r10k() / 5000.0 - 1.0) * 8.0;
        let y = self.r10k() / 10000.0 * 0.4 + 0.01;
        let z = (self.r10k() / 5000.0 - 1.0) * 8.0;
        math::with_len(&[x, y, z], 1.0)
    }

    fn place_across(&mut self) -> f32 {
        (self.r10k() / 5000.0 - 1.0) * 1600.0 - 200.0
    }

    /// FUN_100532b8 and FUN_10053681 at 10 by 10.
    pub fn new() -> Clouds {
        let mut c = Clouds { centre: [-10000.0, 0.0, -10000.0], puffs: Vec::new(), rand: Default::default() };
        for _ in 0..PUFFS {
            let speed = c.r10k() / 10000.0 * 0.5 + 0.6;
            let factor = c.size_factor();
            let width = c.spread(factor);
            let depth = c.spread(factor);
            c.puffs.push(Puff { speed, width, depth, ..Default::default() });
        }
        for k in 0..PUFFS {
            c.shape(k);
        }
        c
    }

    /// Puff `k`'s height rows, normals and light factor anew (FUN_10053681 / FUN_10053e45's mesh).
    fn shape(&mut self, k: usize) {
        let mut heights = Vec::with_capacity(4 * SIDE);
        for _ in 0..4 {
            let amplitude = (((self.rand.next() & 3) + 1) as f32) * 180.0;
            let depth = self.r10k() / 10000.0 * 0.1 + 0.1;
            for j in 0..SIDE {
                let a = j as f32 / (SIDE - 1) as f32 * amplitude * 3.14 / 180.0;
                heights.push(a.sin() * depth + 1.0);
            }
        }
        let normals = (0..SIDE * SIDE).map(|_| self.jitter()).collect();
        let light_factor = self.light_factor();
        let p = &mut self.puffs[k];
        p.heights = heights;
        p.normals = normals;
        p.light_factor = light_factor;
    }

    /// The puff's vertices (its own frame): the grid scaled by its rows.
    fn grid(p: &Puff) -> Vec<(V3, [f32; 2], V3)> {
        let last = (SIDE - 1) as f32;
        let mut out = Vec::with_capacity(SIDE * SIDE);
        for a in 0..SIDE {
            let ta = a as f32 / last;
            let ua = ta - 0.5;
            for b in 0..SIDE {
                let tb = b as f32 / last;
                let ub = tb - 0.5;
                let e = if 0.0 > ub { SIDE } else { 0 };
                let c = if 0.0 > ua { 3 * SIDE } else { 2 * SIDE };
                let x = p.heights[e + a] * (p.width * ub);
                let z = p.heights[c + b] * (p.depth * ua);
                out.push(([x, 0.0, z], [tb * 0.99 + 0.005, ta * 0.99 + 0.005], p.normals[a * SIDE + b]));
            }
        }
        out
    }

    /// FUN_10053e45's placing: across the box on one axis at the altitude, up to ten tries until apart (by 50 in distance
    /// from the middle) from the others; `both`: on both axes.
    fn scatter(&mut self, k: usize, altitude: f32, axis: usize, both: bool) {
        self.puffs[k].place[1] = altitude;
        let close = |a: f32, b: f32| (a.abs() - b.abs()).abs() < 50.0;
        for _ in 0..10 {
            if both {
                self.puffs[k].place[0] = self.place_across();
            }
            self.puffs[k].place[axis] = self.place_across();
            let me = self.puffs[k].place;
            let apart = self.puffs.iter().enumerate().filter(|(j, _)| *j != k).all(|(_, o)| {
                if both { !(close(me[0], o.place[0]) || close(me[2], o.place[2])) } else { !close(me[axis], o.place[axis]) }
            });
            if apart {
                break;
            }
        }
    }

    /// FUN_10053e45: reshaped (its rows, size and normals anew) and placed again.
    fn reshape(&mut self, k: usize, scattered: bool, altitude: f32) {
        self.shape_rows_only(k);
        if scattered {
            self.scatter(k, altitude, 2, true);
            return;
        }
        let p = &mut self.puffs[k];
        let x = p.place[0];
        let wrapped = if x <= -1600.0 {
            p.place[0] = 1500.0;
            true
        } else if x >= 1600.0 {
            p.place[0] = -1500.0;
            true
        } else {
            false
        };
        let z = p.place[2];
        if z <= -1600.0 {
            p.place[2] = 1500.0;
            self.scatter(k, altitude, 0, false);
        } else if z >= 1600.0 {
            p.place[2] = -1500.0;
            self.scatter(k, altitude, 0, false);
        } else if wrapped {
            self.scatter(k, altitude, 2, false);
        } else {
            self.scatter(k, altitude, 0, false);
        }
    }

    /// The reshape's mesh part: rows, size, normals (per vertex, in the order the original draws them) and light factor.
    fn shape_rows_only(&mut self, k: usize) {
        let mut heights = Vec::with_capacity(4 * SIDE);
        for _ in 0..4 {
            let amplitude = (((self.rand.next() & 3) + 1) as f32) * 180.0;
            let depth = self.r10k() / 10000.0 * 0.1 + 0.1;
            for j in 0..SIDE {
                let a = j as f32 / (SIDE - 1) as f32 * amplitude * 3.14 / 180.0;
                heights.push(a.sin() * depth + 1.0);
            }
        }
        let factor = self.size_factor();
        let width = self.spread(factor);
        let depth = self.spread(factor);
        let normals = (0..SIDE * SIDE).map(|_| self.jitter()).collect();
        let light_factor = self.light_factor();
        let p = &mut self.puffs[k];
        p.heights = heights;
        p.width = width;
        p.depth = depth;
        p.normals = normals;
        p.light_factor = light_factor;
    }

    /// FUN_1005485a then FUN_10054e07: the frame's puffs as four triangle lists (one a material, its texture's name),
    /// around the camera (its own frame: the camera at the origin).
    pub fn frame(&mut self, altitude: f32, sky: &Sky) -> Vec<(&'static str, Vec<Vert>)> {
        let mut moved = math::sub(&self.centre, &sky.camera);
        self.centre = sky.camera;
        moved[1] = 0.0;
        let jump = !math::is_zero(&moved) && math::len2(&moved).sqrt() > 800.0;
        let brightness = 1.0 - sky.storm * sky.storm;
        let b = (sky.sun_direction[1] + 0.89).clamp(0.14, 1.0);
        let mut shown: Vec<(V3, Q4, u8)> = Vec::with_capacity(PUFFS);
        for k in 0..PUFFS {
            let drift = moved.map(|c| c / 10.0);
            let p = &mut self.puffs[k];
            let _ = p.speed;
            p.place = math::add(&p.place, &drift);
            let [x, _, z] = p.place;
            if x <= -1600.0 || x >= 1600.0 || z <= -1600.0 || z >= 1600.0 || jump {
                self.reshape(k, jump, altitude);
            }
            let q = self.puffs[k].place;
            let d = math::len2(&math::sub(&q, &[0.0, altitude, 0.0])).sqrt();
            let t = (d / 1500.0).min(1.0);
            let t2 = t * t;
            let sink = altitude * -1.5 * t2;
            let at = math::add(&q, &[0.0, sink, 0.0]);
            let axis = if q[0] == 0.0 && q[2] == 0.0 { [0.0, 1.0, 0.0] } else { math::with_len(&math::cross(&[0.0, 1.0, 0.0], &q), 1.0) };
            let tilt = math::axis_angle(&axis, t2 * 0.25);
            let level = (((b - 0.14) * (1.0 - sky.thick_clouds * 0.9) + 0.14) * self.puffs[k].light_factor).min(1.0);
            shown.push((at, tilt, (level * 200.0) as i64 as u8));
        }
        let groups: [(&'static str, std::ops::Range<usize>); 4] =
            [("single_cloud01.png", 0..2), ("single_cloud02.png", 2..4), ("single_cloud03.png", 4..27), ("single_cloud04.png", 27..30)];
        let mut out = Vec::new();
        for (texture, range) in groups {
            // (the material's emissive: its first puff's light level)
            let emissive = shown[range.start].2 as f32 / 255.0;
            let mut vertices = Vec::new();
            for k in range {
                let (at, tilt, light) = shown[k];
                let alpha = if brightness < 1.0 { ((brightness * 255.0) as i64 as u8) as f32 / 255.0 } else { 1.0 };
                if alpha <= 0.0 {
                    continue;
                }
                let diffuse = light as f32 / 255.0;
                let grid = Self::grid(&self.puffs[k]);
                let vertex = |i: usize| {
                    let (p, uv, n) = grid[i];
                    let mut p = p;
                    math::rotate_by(&mut p, &tilt);
                    let mut n = n;
                    math::rotate_by(&mut n, &tilt);
                    // (D3D: the emissive, plus the vertex colour as the diffuse under the ground light at 0.55)
                    let lit = sky.ground.map_or(0.0, |g| (-(n[0] * g.direction[0] + n[1] * g.direction[1] + n[2] * g.direction[2])).max(0.0));
                    let c = |ch: usize| emissive + diffuse * sky.ground.map_or(0.0, |g| g.colour[ch]) * 0.55 * lit;
                    Vert { pos: math::add(&p, &at), uv, colour: [c(0), c(1), c(2), alpha] }
                };
                for a in 0..SIDE - 1 {
                    for bb in 0..SIDE - 1 {
                        let row = SIDE * a + bb;
                        let next = SIDE * (a + 1) + bb;
                        for i in [row, row + 1, next, row + 1, next + 1, next] {
                            vertices.push(vertex(i));
                        }
                    }
                }
            }
            out.push((texture, vertices));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_puffs_sit_round_the_camera() {
        let mut c = Clouds::new();
        let sky = Sky { camera: [500.0, 10.0, 500.0], sun_direction: [0.0, 1.0, 0.0], storm: 0.0, thick_clouds: 0.0, ground: None };
        // (the first frame is a jump: every puff scattered across the box at the altitude)
        let groups = c.frame(100.0, &sky);
        assert_eq!(groups.len(), 4);
        assert_eq!(groups.iter().map(|g| g.1.len()).sum::<usize>(), 30 * 81 * 6);
        for p in &c.puffs {
            assert!(p.place[0].abs() <= 1800.0 && p.place[2].abs() <= 1800.0 && p.place[1] == 100.0);
        }
        // (a step of 100 moves them back by 10)
        let k = c.puffs.iter().position(|p| p.place[0].abs() < 1400.0 && p.place[2].abs() < 1400.0).unwrap();
        let before = c.puffs[k].place;
        c.frame(100.0, &Sky { camera: [600.0, 10.0, 500.0], ..sky });
        assert!((c.puffs[k].place[0] - (before[0] - 10.0)).abs() < 1e-3, "{before:?} {:?}", c.puffs[k].place);
    }
}

#[cfg(test)]
mod textures {
    /// The puffs' textures load (with an install's further database segments: `ctree`).
    #[test]
    fn the_cloud_textures_load() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = std::sync::Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        let textures = crate::Textures::new(db);
        assert!(textures.sheet("single_cloud01.png").is_some());
    }
}
