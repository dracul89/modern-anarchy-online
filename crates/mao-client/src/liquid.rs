//! The playfield's liquid surfaces, for the effects that ask for them (N3's `n3Playfield_t::GetLiquidSurfaceHeight`, from
//! a zone's water infos, each tested by DisplaySystem's `WaterCollisionInfo_t::PerformCollisionTest`; ported in
//! `AnarchyReimagined`'s `dropin/displaysystem`: `water.rs`, `liquid.rs`).
//!
//! The infos are what `VisualWater_t::Create` makes of the playfield's water triangles (Gamecode FUN_100b84b9 hands it the
//! definition object's, `mao_formats::playfield::Anarchy::water`): only the still types (the low bit clear) get one, each
//! triangle cut into four by its edges' middles while twice its area is over 10000 (FUN_1003a3b4), and kept only when its
//! middle falls on the 60 by 60 tiles of 100 and its corners make a face (FUN_1003a0d6), its corners ordered so its normal
//! points up (FUN_10039f14). A point is in an info's liquid when it is below the box's top, inside its x and z, and on the
//! inner side of its three edges; the liquid's height there is its plane's, and counts when above 0 (`PerformCollisionTest`)
//! and when its depth (the info's value) reaches down past the point (`n3Zone_t::GetLiquidSurfaceHeight`).
//!
//! Not here: the zones' lists (the infos are tested in the order made, all of them; a zone's stops at its first hit).

use std::sync::{Arc, Mutex};

/// One liquid surface (`VisualWaterInfo_t`, 0x6c bytes).
#[derive(Clone, Debug, PartialEq)]
struct Info {
    top: [f32; 3],
    corners: [[f32; 3]; 3],
    kind: u32,
    value: f32,
    normal: [f32; 3],
    min: [f32; 2],
    distance: f32,
}

/// What a liquid test found: the surface's height, its type and its value (depth).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub height: f32,
    pub kind: u32,
    pub value: f32,
}

/// The playfield's liquid.
#[derive(Clone, Debug, Default)]
pub struct Liquid {
    infos: Vec<Info>,
}

fn minus(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// `(p − q) / 2 + q`.
fn middle(p: &[f32; 3], q: &[f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|i| (p[i] - q[i]) / 2.0 + q[i])
}

impl Liquid {
    /// The infos of the playfield's water triangles.
    pub fn new(water: &[mao_formats::playfield::WaterTriangle]) -> Liquid {
        let mut l = Liquid::default();
        for t in water {
            let [a, b, c] = t.points;
            if a == b || b == c || c == a || t.kind & 1 != 0 {
                continue;
            }
            l.split(t.kind, t.value, a, b, c);
        }
        l
    }

    /// FUN_1003a3b4: cut in four while twice the area is over 10000.
    fn split(&mut self, kind: u32, value: f32, a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
        let n = cross(&minus(&b, &a), &minus(&c, &a));
        if (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() > 10000.0 {
            let (ab, bc, ca) = (middle(&b, &a), middle(&c, &b), middle(&a, &c));
            self.split(kind, value, a, ab, middle(&c, &a));
            self.split(kind, value, b, bc, middle(&a, &b));
            self.split(kind, value, c, ca, middle(&b, &c));
            self.split(kind, value, ab, bc, ca);
            return;
        }
        self.triangle(kind, value, a, b, c);
    }

    /// FUN_1003a0d6 and `info_set`: kept when its middle is on the tiles and it makes a face; its info.
    fn triangle(&mut self, kind: u32, value: f32, a: [f32; 3], mut b: [f32; 3], mut c: [f32; 3]) {
        let mz = (b[2] + a[2] + c[2]) / 3.0;
        let row = (mz / -100.0) as i32;
        let column = ((b[0] + a[0] + c[0]) / 3.0 / 100.0) as i32;
        if column.wrapping_sub(row.wrapping_mul(60)) as u32 > 0xe0f || a == b || b == c || c == a {
            return;
        }
        let mut n = cross(&minus(&b, &a), &minus(&c, &a));
        if n[1] < 0.0 {
            std::mem::swap(&mut b, &mut c);
            n = cross(&minus(&b, &a), &minus(&c, &a));
        }
        if n.iter().all(|v| v.abs() < 1e-5) {
            return;
        }
        let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        let n = n.map(|v| v / l);
        let most = |i: usize| a[i].max(b[i]).max(c[i]);
        let least = |i: usize| a[i].min(b[i]).min(c[i]);
        let distance = -(n[1] * a[1] + n[0] * a[0] + n[2] * a[2]);
        self.infos.push(Info { top: [most(0), most(1), most(2)], corners: [a, b, c], kind, value, normal: n, min: [least(0), least(2)], distance });
    }

    /// The liquid at a point (Y up): `GetLiquidSurfaceHeight(point, …, point's own height)`, none where there is none.
    pub fn at(&self, p: [f32; 3]) -> Option<Surface> {
        for i in &self.infos {
            let side = |k: usize| {
                let (a, b) = (i.corners[k], i.corners[(k + 1) % 3]);
                (p[2] as f64 - a[2] as f64) * (b[0] - a[0]) as f64 - (p[0] as f64 - a[0] as f64) * (b[2] - a[2]) as f64
            };
            let inside = i.top[1] > p[1] && i.min[0] < p[0] && i.top[0] > p[0] && i.min[1] < p[2] && i.top[2] > p[2] && side(0) < 0.0 && side(1) < 0.0 && side(2) < 0.0;
            if !inside {
                continue;
            }
            let h = (((-(p[0] as f64 * i.normal[0] as f64) - i.normal[2] as f64 * p[2] as f64) - i.distance as f64) / i.normal[1] as f64) as f32;
            if 0.0 < h {
                // (the zone's first hit decides: its depth reaches the point, or there is no liquid)
                return (h - i.value < p[1]).then_some(Surface { height: h, kind: i.kind, value: i.value });
            }
        }
        None
    }

    /// The liquid surface under a point: the first info whose triangle holds the point's x and z and whose plane there lies below
    /// its height. It is what the fall effect's walk along a waterfall's foot asks of the playfield's collision (Gamecode
    /// FUN_100afc4c's effect 2 puts each step on the liquid and keeps the ones that went down).
    pub fn surface_below(&self, p: [f32; 3]) -> Option<f32> {
        for i in &self.infos {
            let side = |k: usize| {
                let (a, b) = (i.corners[k], i.corners[(k + 1) % 3]);
                (p[2] as f64 - a[2] as f64) * (b[0] - a[0]) as f64 - (p[0] as f64 - a[0] as f64) * (b[2] - a[2]) as f64
            };
            let inside = i.min[0] < p[0] && i.top[0] > p[0] && i.min[1] < p[2] && i.top[2] > p[2] && side(0) < 0.0 && side(1) < 0.0 && side(2) < 0.0;
            if !inside {
                continue;
            }
            let h = (((-(p[0] as f64 * i.normal[0] as f64) - i.normal[2] as f64 * p[2] as f64) - i.distance as f64) / i.normal[1] as f64) as f32;
            if h < p[1] {
                return Some(h);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.infos.is_empty()
    }
}

/// Where the fall effect is made (server coordinates, Y up): `VisualWater_t::Create` (DisplaySystem FUN_1003b1cd, drop-in
/// `liquid.rs` `start_fall`) starts the environment's effect 2 for every falling triangle (the type's low bit) over a tenth high
/// with one high corner — but lava, types 2 and 3 — at one low corner with the other as its ground; Gamecode's FUN_100afc4c then
/// walks from the one towards the other a unit at a time (`floor(length) + 1` steps from the first) and makes gfx 0x30d6 at each
/// step with a liquid surface below it ([`Liquid::surface_below`]), at the step's own height.
pub fn fall_effects(water: &[mao_formats::playfield::WaterTriangle], liquid: &Liquid) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for t in water {
        let [a, b, c] = t.points;
        if a == b || b == c || c == a || t.kind & 1 == 0 || t.kind == 2 || t.kind == 3 {
            continue;
        }
        let most = a[1].max(b[1]).max(c[1]);
        let least = a[1].min(b[1]).min(c[1]);
        let h = most - least;
        if h < 0.1 {
            continue;
        }
        // (the original snaps each height to 1 above the half or −0.01 below it: one high corner is two low ones)
        let low = |y: f32| (y - least) / h <= 0.5;
        let (at, ground) = match (low(a[1]), low(b[1]), low(c[1])) {
            (true, true, _) => (a, b),
            (true, false, true) => (c, a),
            (false, true, true) => (b, c),
            _ => continue,
        };
        let d = minus(&ground, &at);
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if len <= 0.0 {
            continue;
        }
        let step = [d[0] / len, d[1] / len, d[2] / len];
        for k in 0..(len.floor() as u32 + 1) {
            let p = [at[0] + step[0] * k as f32, at[1] + step[1] * k as f32, at[2] + step[2] * k as f32];
            if liquid.surface_below(p).is_some() {
                out.push(p);
            }
        }
    }
    out
}

/// The current playfield's liquid (set by the game connection when the playfield loads), and the feet of its waterfalls
/// ([`fall_effects`]; set with the terrain, for the pictures that draw the fall effect).
static CURRENT: Mutex<Option<Arc<Liquid>>> = Mutex::new(None);
static FALLS: Mutex<Vec<[f32; 3]>> = Mutex::new(Vec::new());

pub fn set_falls(f: Vec<[f32; 3]>) {
    *FALLS.lock().unwrap_or_else(|e| e.into_inner()) = f;
}

pub fn falls() -> Vec<[f32; 3]> {
    FALLS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn set_current(l: Option<Arc<Liquid>>) {
    *CURRENT.lock().unwrap_or_else(|e| e.into_inner()) = l;
}

pub fn current() -> Option<Arc<Liquid>> {
    CURRENT.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mao_formats::playfield::WaterTriangle;

    /// Which playfields have the fall effect, and at how many places (`MAO_GAME`).
    #[test]
    #[ignore]
    fn fall_effect_survey() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        for p in mao_formats::playfield::all(&db) {
            let Some(a) = mao_formats::playfield::load_anarchy(&db, p.number) else { continue };
            let falling = a.water.iter().filter(|t| t.kind & 1 != 0).count();
            if falling == 0 {
                continue;
            }
            let feet = fall_effects(&a.water, &Liquid::new(&a.water));
            println!("{} {}: {} falling triangles, the effect at {} places{}", p.number, p.name.trim(), falling, feet.len(), feet.first().map_or(String::new(), |f| format!(", first {f:?}")));
        }
    }

    #[test]
    fn a_fall_over_a_pool_gets_its_effect_along_its_foot() {
        // (a pool at height 0 over (0..20, 0..20) and a fall 5 high dropping to height 1 along z = 10 from x 2 to x 8)
        let pool = |a, b, c| WaterTriangle { kind: 0, value: 100.0, points: [a, b, c] };
        let fall = |kind, y| WaterTriangle { kind, value: 100.0, points: [[2.0, y, 10.0], [8.0, y, 10.0], [8.0, y + 5.0, 10.0]] };
        let water = vec![pool([0.0, 0.0, 0.0], [20.0, 0.0, 0.0], [20.0, 0.0, 20.0]), pool([0.0, 0.0, 0.0], [20.0, 0.0, 20.0], [0.0, 0.0, 20.0]), fall(1, 1.0)];
        let liquid = Liquid::new(&water);
        assert_eq!(liquid.surface_below([5.0, 1.0, 10.0]), Some(0.0));
        assert_eq!(liquid.surface_below([5.0, 0.0, 10.0]), None, "a surface at the point's height is not below it");
        let feet = fall_effects(&water, &liquid);
        assert_eq!(feet.len(), 7, "{feet:?}");
        assert_eq!((feet[0], feet[6]), ([2.0, 1.0, 10.0], [8.0, 1.0, 10.0]));
        // a fall whose foot sits at the pool's surface, one over no pool, and lava get none
        for water in [vec![water[0].clone(), water[1].clone(), fall(1, 0.0)], vec![fall(1, 1.0)], vec![water[0].clone(), water[1].clone(), fall(3, 1.0)]] {
            assert!(fall_effects(&water, &Liquid::new(&water)).is_empty());
        }
    }

    #[test]
    fn a_pool_s_surface() {
        // (a flat pool at 20 over (100..300, 100..300), two triangles of the still water type 0, 100000 deep)
        let tri = |points| WaterTriangle { kind: 0, value: 100000.0, points };
        let w = [tri([[100.0, 20.0, 100.0], [300.0, 20.0, 100.0], [300.0, 20.0, 300.0]]), tri([[100.0, 20.0, 100.0], [300.0, 20.0, 300.0], [100.0, 20.0, 300.0]])];
        let l = Liquid::new(&w);
        // (each cut small: twice its area 40000 → pieces under 10000)
        assert!(l.infos.len() >= 8);
        // (a point off the pieces' edges: an edge itself is outside, as the original's `< 0` has it)
        let s = l.at([210.0, 15.0, 160.0]).unwrap();
        assert!((s.height - 20.0).abs() < 1e-3 && s.kind == 0);
        assert!(l.at([210.0, 25.0, 160.0]).is_none());
        assert!(l.at([50.0, 15.0, 150.0]).is_none());
        // (falling water, the low bit set, makes no surface)
        assert!(Liquid::new(&[WaterTriangle { kind: 1, ..w[0].clone() }]).is_empty());
    }
}
