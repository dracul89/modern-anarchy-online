//! The character's body against the playfield, as Vehicle.dll moves it: `Vehicle_t::Run` (FUN_1000e3d3) steps the body by its
//! velocity and gravity, and after each step `Vehicle_t::EnsureSurfaceAlignment` (0x1000d1aa) settles it on what it walked
//! into: a sphere of radius 0.4 whose bottom is the feet is swept from the last place along the move, sliding along what it
//! touches (FUN_1000b2e5); the ground under the feet is found by the playfield's closest point and three rays round the feet,
//! which also decide whether the body stands or falls (a face steeper than 60 degrees is no footing) and whether it is in
//! deep liquid (swimming). The playfield's side is N3.dll's `n3TilemapSurface_t` ([`Ground`]): the zones' collision
//! surfaces (`mao_formats::zones`), the tile-map's ground (`Terrain::surface`), the liquid (`crate::liquid`), the bounds.
//!
//! Game coordinates, y up. The original's float arithmetic is kept where it decides (the order of the sums, the compares);
//! the KD trees' walks are replaced by a grid with the same answers.

use std::sync::Arc;

use mao_formats::terrain::Terrain;
use mao_formats::zones::Zones;

pub type V3 = [f32; 3];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// FUN_100013c3: the squared length (z, x, y summed).
fn lsq(a: V3) -> f32 {
    a[2] * a[2] + a[0] * a[0] + a[1] * a[1]
}

fn dot(a: V3, b: V3) -> f32 {
    a[2] * b[2] + a[0] * b[0] + a[1] * b[1]
}

/// FUN_10001351: `a × b`.
fn cross(a: V3, b: V3) -> V3 {
    [b[2] * a[1] - b[1] * a[2], a[2] * b[0] - a[0] * b[2], b[1] * a[0] - b[0] * a[1]]
}

/// FUN_10002a59.
fn is_zero(a: V3) -> bool {
    a[0] == 0.0 && a[1] == 0.0 && a[2] == 0.0
}

/// FUN_100026a4: `a` scaled to length 1 (by 1/length, FUN_10001f4e).
fn unit(a: V3) -> V3 {
    let inverse = 1.0 / lsq(a).sqrt();
    scale(a, inverse)
}

/// No liquid (`_DAT_1003d2ec`).
pub const NO_LIQUID: f32 = -100000.0;
/// Gravity (`Vehicle_t::s_vGravityAccel`).
pub const GRAVITY: f32 = -20.0;
/// The sphere's radius and its centre's height over the feet (`_DAT_100127a0`).
const RADIUS: f32 = 0.4;
/// The longest step `Run` takes at once (+0x104).
const STEP: f32 = 0.4;

/// A segment's first front face on a triangle: the point, its normal and how far along (0 to 1).
fn triangle_hit(t: &[V3; 3], start: V3, end: V3) -> Option<(V3, V3, f32)> {
    let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
    let d = sub(end, start);
    let dn = dot(d, n);
    if dn >= 0.0 {
        return None;
    }
    let s = dot(sub(t[0], start), n) / dn;
    if !(0.0..=1.0).contains(&s) {
        return None;
    }
    let p = add(start, scale(d, s));
    for k in 0..3 {
        let (a, b) = (t[k], t[(k + 1) % 3]);
        if dot(cross(sub(b, a), sub(p, a)), n) < 0.0 {
            return None;
        }
    }
    Some((p, unit(n), s))
}

/// The playfield's surface for the vehicle: `n3TilemapSurface_t` outdoors (the zones and the tile-map's ground), the
/// zones alone indoors.
pub struct Ground {
    pub zones: Arc<Zones>,
    /// The tile-map's ground, its height scale, and the client's offset of the ground's heights to the server's.
    pub terrain: Option<(Arc<Terrain>, f32)>,
    pub offset: f32,
}

impl Ground {
    /// FUN_10017800: the ground's height and normal under (`x`, `z`) (none off the map).
    pub fn terrain_at(&self, x: f32, z: f32) -> Option<(f32, V3)> {
        let (t, scale) = self.terrain.as_ref()?;
        t.surface(x, z, *scale).map(|(h, n)| (h + self.offset, n))
    }

    /// The tile-map ground's triangles of tile (`i`, `j`), facing up, split as FUN_1001769d splits it.
    fn tile(&self, i: i64, j: i64) -> Option<[[V3; 3]; 2]> {
        let (t, scale) = self.terrain.as_ref()?;
        let size = t.tile_size;
        if i < 0 || j < 0 || i as usize >= t.width || j as usize >= t.height {
            return None;
        }
        // (a corner past the last point is the last point's height, as FUN_1001769d clamps)
        let corner = |a: i64, b: i64| {
            let (ci, cj) = ((a as usize).min(t.width - 1), (b as usize).min(t.height - 1));
            [a as f32 * size, t.heights[cj * t.width + ci] as f32 / 256.0 * scale + self.offset, b as f32 * size]
        };
        let (p0, p1, p2, p3) = (corner(i, j), corner(i + 1, j), corner(i + 1, j + 1), corner(i, j + 1));
        // (wound so the face is up: `triangle_hit`'s normal is (b − a) × (c − a))
        if (!j ^ i) & 1 != 0 {
            Some([[p0, p2, p1], [p0, p3, p2]])
        } else {
            Some([[p0, p3, p1], [p1, p3, p2]])
        }
    }

    /// `n3TilemapSurface_t::GetLineIntersection` (0x10018b72): the zones' first hit and the tile-map ground's along the
    /// tiles the segment crosses (`Intersect_Tile`), the nearer: the point and the face's normal.
    pub fn line(&self, start: V3, end: V3) -> Option<(V3, V3)> {
        let zone = self.zones.line(start, end).map(|(hit, n)| (hit, n, lsq(sub(hit, start))));
        let mut ground: Option<(V3, V3, f32)> = None;
        if let Some((t, _)) = self.terrain.as_ref() {
            let size = t.tile_size;
            let across = ((end[0] - start[0]).hypot(end[2] - start[2]) / (size * 0.5)).ceil().min(4000.0) as usize + 1;
            let mut last = None;
            for k in 0..=across {
                let p = add(start, scale(sub(end, start), k as f32 / across as f32));
                let tile = ((p[0] / size).floor() as i64, (p[2] / size).floor() as i64);
                // (each tile once; its neighbours too, as a segment may cut a tile's corner between two samples)
                if last == Some(tile) {
                    continue;
                }
                last = Some(tile);
                for (di, dj) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                    for tri in self.tile(tile.0 + di, tile.1 + dj).into_iter().flatten() {
                        if let Some((hit, n, s)) = triangle_hit(&tri, start, end)
                            && ground.is_none_or(|g| g.2 > s)
                        {
                            ground = Some((hit, n, s));
                        }
                    }
                }
            }
        }
        let ground = ground.map(|(hit, n, _)| (hit, n, lsq(sub(hit, start))));
        match (zone, ground) {
            (Some(z), Some(g)) => Some(if g.2 < z.2 { (g.0, g.1) } else { (z.0, z.1) }),
            (Some(z), None) => Some((z.0, z.1)),
            (None, Some(g)) => Some((g.0, g.1)),
            (None, None) => None,
        }
    }

    /// `n3TilemapSurface_t::CalculateClosestPoint` (0x10018f0c): the ground under `pos` (the zones' first floor within 0.3
    /// under the tile-map's ground's height over it, when that is higher than the ground), raised to 1.2 under the
    /// liquid's surface; its normal; the liquid's height (`m_vLiquidHeight`, [`NO_LIQUID`] when there is none).
    pub fn closest(&self, pos: V3) -> (V3, V3, f32) {
        let (mut closest, mut normal) = match self.terrain_at(pos[0], pos[2]) {
            Some((h, n)) => {
                let above = pos[1] - h;
                match (above >= 0.0).then(|| self.zones.floor_below(pos, above + 0.3)).flatten() {
                    Some((hit, n)) if hit[1] > h => (hit, n),
                    _ => ([pos[0], h, pos[2]], n),
                }
            }
            // (indoors `n3RoomSurface_t::CalculateClosestPoint` 0x10013ee6 takes the room's floor under (x, z) whatever the
            // height: feet a little under it, as a server's position can be, stand on it rather than fall to 0)
            None if self.terrain.is_none() => match self.zones.floor_below(pos, 2000.0).or_else(|| self.zones.floor_below([pos[0], pos[1] + 1.0, pos[2]], 2001.0)) {
                Some((hit, n)) => (hit, n),
                None => ([pos[0], 0.0, pos[2]], [0.0, 1.0, 0.0]),
            },
            // (off the map the original leaves the outputs as they were: zero)
            None => return ([0.0; 3], [0.0; 3], NO_LIQUID),
        };
        let liquid = crate::liquid::current().and_then(|l| l.at([closest[0], pos[1], closest[2]])).map_or(NO_LIQUID, |l| l.height);
        // (`_DAT_1003d370`)
        if closest[1] < liquid - 1.2 {
            closest[1] = liquid - 1.2;
            normal = [0.0, 1.0, 0.0];
        }
        (closest, normal, liquid)
    }

    /// `n3TilemapSurface_t::VetoPosition` (0x10018a7c): a place off the map (or not over 0 and under 2000 high) is refused:
    /// back to `previous` when there is one, then held 0.1 inside the map and between 0.01 and 1999.9 high.
    pub fn veto(&self, pos: &mut V3, previous: Option<V3>) -> bool {
        let Some((t, _)) = self.terrain.as_ref() else { return false };
        let (w, h) = (t.tile_size * t.width as f32, t.tile_size * t.height as f32);
        let inside = |p: &V3| p[0] > 0.0 && p[0] < w && p[2] > 0.0 && p[2] < h && p[1] > 0.0 && p[1] < 2000.0;
        if inside(pos) {
            return false;
        }
        if let Some(p) = previous {
            *pos = p;
        }
        pos[0] = pos[0].clamp(0.1, w - 0.1);
        pos[2] = pos[2].clamp(0.1, h - 0.1);
        pos[1] = pos[1].clamp(0.01, 1999.9);
        true
    }
}

/// What a step did that the character answers (the vehicle's callbacks).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Started falling (FUN_1000a1a7, slot 0x74).
    Fell,
    /// Landed at this height (`LandNow`, slot 0x6c: the jump's stop).
    Landed(f32),
    /// Into deep liquid and out of it (slots 0x80 and 0x84: the swim mode asked for, 0x1a, and left, 0x23).
    Swim,
    LeaveSwim,
}

/// The vehicle's state the alignment keeps.
#[derive(Clone, Debug)]
pub struct Body {
    /// Falling is on (+0x50; a character's always is but for flying) and the body is in the air (+0x52).
    pub gravity: bool,
    pub falling: bool,
    /// The vertical speed (+0x54).
    pub vy: f32,
    /// Steps in the air without moving (+0x138).
    stuck: u32,
    /// The surface's normal under the body and its smoothed up (+0xa0, +0x12c).
    pub normal: V3,
    smoothed: V3,
    /// In deep liquid (+0x120) and how far under its surface the feet are (+0x10c).
    pub in_liquid: bool,
    pub immersion: f32,
    /// The liquid behaviour (+0xfc: 0 for a character) and the depth that is swimming (+0x100, `_DAT_100127a4`).
    pub liquid_mode: u32,
    pub swim_depth: f32,
}

impl Default for Body {
    fn default() -> Body {
        Body { gravity: true, falling: false, vy: 0.0, stuck: 0, normal: [0.0, 1.0, 0.0], smoothed: [0.0, 1.0, 0.0], in_liquid: false, immersion: -9999.0, liquid_mode: 0, swim_depth: 1.19 }
    }
}

impl Body {
    /// `LandNow` (0x1000a719).
    fn land(&mut self, height: f32, events: &mut Vec<Event>) {
        self.vy = 0.0;
        events.push(Event::Landed(height));
        self.falling = false;
    }

    /// FUN_1000a1a7.
    fn fall(&mut self, events: &mut Vec<Event>) {
        if !self.falling {
            self.falling = true;
            events.push(Event::Fell);
        }
    }

    /// The jump (CharVehicle_t FUN_1006ff32): the height asked for, less what a ceiling leaves (a segment 100 up from the
    /// feet; at least 0.1 clear of it after twice the body's scale), at speed `√(2 × height × 20)` up (`Vehicle_t::Impact`,
    /// only from the ground).
    pub fn jump(&mut self, ground: &Ground, pos: V3, height: f32, body_scale: f32, events: &mut Vec<Event>) {
        let mut height = height;
        if let Some((hit, _)) = ground.line(pos, add(pos, [0.0, 100.0, 0.0])) {
            let room = (hit[1] - pos[1]) - (body_scale + body_scale);
            let room = if room < 0.1 { 0.1 } else { room };
            if room <= height {
                height = room;
            }
        }
        let speed = ((height + height) * GRAVITY.abs()).sqrt();
        if !self.falling {
            self.vy += speed;
            self.fall(events);
        }
    }

    /// `Vehicle_t::Run`'s steps (FUN_1000e3d3): `dt` cut into steps of at most 0.4; each falls by gravity (in the air;
    /// the speed held within 50), moves by the horizontal `velocity` and the vertical speed, and is settled by
    /// [`Body::align`]; a step that moves nothing ends them.
    pub fn run(&mut self, ground: &Ground, pos: &mut V3, velocity: [f32; 2], dt: f32, events: &mut Vec<Event>) {
        if dt > 4.0 {
            return;
        }
        let mut done = 0.0;
        while dt > done {
            let step = (dt - done).min(STEP);
            let previous = *pos;
            if self.falling {
                self.vy += GRAVITY * step;
                if self.vy.abs() > 50.0 {
                    self.vy = if self.vy < 0.0 { -50.0 } else { 50.0 };
                }
            }
            let moving = velocity[0].hypot(velocity[1]) > 0.001;
            if moving || self.vy != 0.0 {
                pos[0] += velocity[0] * step;
                pos[2] += velocity[1] * step;
                pos[1] += self.vy * step;
            }
            if !self.align(ground, pos, previous, false, events) {
                break;
            }
            done += step;
        }
    }

    /// `Vehicle_t::EnsureSurfaceAlignment` (0x1000d1aa): `pos` (moved from `previous`) settled on the playfield; whether it
    /// moved.
    pub fn align(&mut self, ground: &Ground, pos: &mut V3, previous: V3, forced: bool, events: &mut Vec<Event>) -> bool {
        let free = if !self.gravity || self.falling || forced { 1.0 } else { 0.0 };
        let up = [0.0, RADIUS, 0.0];
        // (a refused place steps back towards the last, a tenth at a time)
        let tenth = scale(sub(*pos, previous), 0.1);
        let mut at = *pos;
        let mut k = 9;
        // (indoors a place in no room is refused too, `n3RoomSurface_t::VetoPosition` 0x10015587 and its
        // `VetoRoomTransition` 0x1001462f, unless the body is in none already: the step back then finds nowhere better)
        let from_room = ground.zones.in_room(previous[0], previous[2]) == Some(true);
        loop {
            if !ground.veto(&mut at, None) && !(from_room && ground.zones.in_room(at[0], at[2]) == Some(false)) {
                break;
            }
            at = add(previous, scale(tenth, k as f32));
            k -= 1;
            if k <= 0 {
                break;
            }
        }
        // (still out of the rooms: along the edge on either axis, else not at all)
        if from_room && ground.zones.in_room(at[0], at[2]) == Some(false) {
            at = [[at[0], at[1], previous[2]], [previous[0], at[1], at[2]]].into_iter().find(|p| ground.zones.in_room(p[0], p[2]) == Some(true)).unwrap_or(previous);
        }
        let mut moved = sub(at, previous);
        let mut centre = add(previous, up);
        let mut top = previous[1] + 0.4;
        let length2 = lsq(moved);
        if 0.0 < length2 {
            if forced {
                moved[1] *= free;
                centre = add(moved, centre);
            } else {
                let flat = [moved[0], 0.0, moved[2]];
                let mut across = lsq(flat);
                if 0.0 < across {
                    across = across.sqrt();
                }
                let mut budget = if free <= 0.0 {
                    if across < 1e-6 { length2.sqrt() } else { 10000.0 }
                } else {
                    length2.sqrt()
                };
                moved[1] *= free;
                let mut dir = moved;
                if is_zero(dir) {
                    dir = [0.0, -1.0, 0.0];
                } else {
                    dir = unit(dir);
                }
                // (`_DAT_100127f0`, `_DAT_10012134`; +0x13c is never set for a character)
                let slope = if self.gravity { 0.5 } else { -1.0 };
                sweep(&mut centre, RADIUS, dir, &mut across, &mut budget, ground, &mut top, if self.gravity { 3 } else { 1 }, 10, slope);
            }
        }
        if top < centre[1] {
            top = centre[1];
        }
        let mut feet = [centre[0], centre[1] - 0.4, centre[2]];
        let (closest, _closest_normal, liquid) = ground.closest(feet);
        let margin = if self.liquid_mode == 4 { 0.25 } else { 0.01 };
        if feet[1] < closest[1] + margin {
            feet[1] = closest[1] + margin;
        }
        if forced {
            top = feet[1] + 1.0;
        } else if top < feet[1] {
            top = feet[1];
        }
        let mut reach = 0.48;
        if self.gravity && !self.falling && !forced {
            reach = lsq(moved).sqrt() * 1.154_700_517_654_419 + 0.48;
            if top < reach {
                reach = top;
            }
        }
        // Three rays down round the feet (0.04 out at 0°, 135° and 225°) from over the highest point reached to 0: the
        // face they find and the surface's normal from their hits.
        let raise = if self.gravity { up } else { [0.0; 3] };
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let around = [[1.0, 0.0, 0.0], [-s, 0.0, s], [-s, 0.0, -s]];
        let mut hits = [[0.0f32; 3]; 3];
        for (k, d) in around.iter().enumerate() {
            let offset = scale(*d, 0.04);
            let from = add(add([feet[0], top, feet[2]], raise), offset);
            let to = add([feet[0], 0.0, feet[2]], offset);
            hits[k] = ground.line(from, to).map_or(to, |(hit, _)| hit);
        }
        let mut normal = unit(cross(sub(hits[1], hits[0]), sub(hits[2], hits[0])));
        if normal[1] < 0.0 {
            normal = scale(normal, -1.0);
        }
        self.normal = normal;
        // (+0xb0, the alignment's kind, is 0 for a character: the body stays upright)
        let upright = [0.0, 1.0, 0.0];
        let depth = liquid - closest[1];
        let highest = hits[0][1].max(hits[1][1]).max(hits[2][1]);
        let mut in_air = true;
        if feet[1] - reach >= highest.max(liquid) {
            in_air = true;
        } else {
            let floor = if highest < closest[1] { closest[1] } else { highest };
            if (self.vy as f64) < 0.1 && self.gravity {
                feet[1] = floor + 0.01;
            }
            // (a face steeper than 60 degrees is no footing)
            if !(self.normal[1] < 0.5) && (self.vy as f64) < 0.1 {
                in_air = false;
            }
        }
        if feet[1] == previous[1] && self.vy < -0.1 && in_air {
            self.stuck += 1;
            if self.stuck > 4 {
                in_air = false;
            }
        } else {
            self.stuck = 0;
        }
        self.normal = upright;
        self.smoothed = unit(add(scale(self.smoothed, 0.9), scale(self.normal, 0.1)));
        self.normal = self.smoothed;
        if self.normal[1] < 0.5 {
            self.normal = upright;
        }
        if self.gravity {
            if in_air {
                self.fall(events);
            } else if self.falling {
                self.land(feet[1], events);
            }
        }
        // The liquid (mode 0, a character's): within 0.1 over its surface, deeper than the swimming depth swims with the
        // feet held 0.01 under the surface; shallower wades, on the ground.
        let mut height = feet[1];
        match self.liquid_mode {
            0 => {
                if feet[1] < liquid + 0.1 {
                    if depth <= self.swim_depth {
                        if self.in_liquid {
                            events.push(Event::LeaveSwim);
                            self.in_liquid = false;
                        }
                        if (self.vy as f64) < 0.1 && self.gravity {
                            height = (liquid - depth) + 0.01;
                        }
                    } else {
                        if !self.in_liquid {
                            if self.gravity {
                                events.push(Event::Swim);
                                self.in_liquid = true;
                            }
                        } else if !self.gravity {
                            events.push(Event::LeaveSwim);
                            self.in_liquid = false;
                        }
                        let surface = liquid - 0.01;
                        if feet[1] < surface {
                            height = surface;
                        }
                    }
                } else {
                    self.immersion = -9999.0;
                    if self.in_liquid {
                        events.push(Event::LeaveSwim);
                        self.in_liquid = false;
                    }
                }
            }
            4 => {
                let surface = liquid + 0.25;
                if feet[1] < surface {
                    height = surface;
                }
            }
            _ => {}
        }
        feet[1] = height;
        if feet[1] < liquid {
            self.immersion = liquid - feet[1];
        }
        ground.veto(&mut feet, None);
        *pos = feet;
        *pos != previous
    }
}

/// FUN_1000b2e5: the sphere at `centre` (radius `r`) swept along `dir` towards a point 10 ahead, sliding along what it
/// meets, `iterations` times at most, until `across` (the horizontal distance) or `budget` (the whole) is used; `top` the
/// highest centre reached. `mode` over 1 first measures the room on each side (rays of `r` across the direction) and
/// centres the sphere between, and sweeps rays from half that out too; `slope` is the normal's y under which a face met
/// going up is taken as a wall (its normal laid flat, the radius grown by `n.y² × r`).
#[allow(clippy::too_many_arguments)]
fn sweep(centre: &mut V3, r: f32, dir: V3, across: &mut f32, budget: &mut f32, ground: &Ground, top: &mut f32, mode: i32, iterations: i32, slope: f32) {
    let mut target = add(*centre, scale(dir, 10.0));
    let mut iterations = iterations;
    // (a move along `step`, its horizontal part counted against `across`, the whole against `budget`; true: used up)
    let advance = |centre: &mut V3, step: V3, across: &mut f32, budget: &mut f32, top: &mut f32| -> Option<bool> {
        let flat = lsq([step[0], 0.0, step[2]]);
        let flat = if 0.0 < flat { flat.sqrt() } else { flat };
        if *across <= 0.0 || flat <= 0.0 {
            let length = lsq(step);
            if length <= 0.0 {
                return None;
            }
            let length = length.sqrt();
            if length <= 0.0 {
                return None;
            }
            if *budget < length {
                *centre = add(*centre, scale(step, *budget / length));
                if *top < centre[1] {
                    *top = centre[1];
                }
                *budget = 0.0;
                *across = 0.0;
                return Some(true);
            }
            *centre = add(*centre, step);
            if *top < centre[1] {
                *top = centre[1];
            }
            *across -= flat;
            *budget -= length;
        } else {
            if *across < flat {
                *centre = add(*centre, scale(step, *across / flat));
                if *top < centre[1] {
                    *top = centre[1];
                }
                *budget = 0.0;
                *across = 0.0;
                return Some(true);
            }
            *centre = add(*centre, step);
            if *top < centre[1] {
                *top = centre[1];
            }
            *across -= flat;
            *budget -= lsq(step).sqrt();
        }
        Some(false)
    };
    loop {
        if iterations < 1 {
            return;
        }
        iterations -= 1;
        let to_target = sub(target, *centre);
        if lsq(to_target) < 1e-8 {
            return;
        }
        let d = unit(to_target);
        let mut side_a: Option<(V3, V3)> = None;
        let mut side_b: Option<(V3, V3)> = None;
        if 1 < mode {
            let (mut left, mut right) = if ((d[1] * d[1]) as f64) < 0.999_998_986_721_038_8 {
                let side = scale(unit(cross(d, [0.0, 1.0, 0.0])), r);
                (side, scale(side, -1.0))
            } else {
                ([r, 0.0, 0.0], [-r, -0.0, -0.0])
            };
            if let Some((hit, _)) = ground.line(*centre, add(*centre, left)) {
                left = sub(hit, *centre);
            }
            if let Some((hit, _)) = ground.line(*centre, add(*centre, right)) {
                right = sub(hit, *centre);
            }
            if lsq(left) != lsq(right) {
                let (a, b) = (add(*centre, left), add(*centre, right));
                *centre = scale(add(a, b), 1.0 / 2.0);
                left = sub(a, *centre);
                right = sub(b, *centre);
                if *top < centre[1] {
                    *top = centre[1];
                }
            }
            left = scale(left, 0.5);
            right = scale(right, 0.5);
            // (each side's ray, its hit carried onto the centre's line)
            let side = |offset: V3| -> Option<(V3, V3)> {
                if is_zero(offset) {
                    return None;
                }
                let (hit, n) = ground.line(add(*centre, offset), add(target, offset))?;
                let facing = dot(unit(offset), n);
                let den = d[2] * n[2] + d[0] * n[0] + n[1] * d[1];
                if 0.0 < facing || den == 0.0 {
                    return None;
                }
                let t = dot(sub(hit, *centre), n) / den;
                Some((add(*centre, scale(d, t)), n))
            };
            side_a = side(left);
            side_b = side(right);
        }
        let ahead = ground.line(*centre, target);
        if side_a.is_none() && side_b.is_none() && ahead.is_none() {
            // Nothing in the way: up to the target less the radius.
            if *centre == target {
                return;
            }
            let mut step = sub(target, *centre);
            let length = lsq(step).sqrt();
            step = scale(step, (length - r) / length);
            match advance(centre, step, across, budget, top) {
                None | Some(true) => return,
                Some(false) => {}
            }
            target = add(*centre, scale(dir, 10.0));
            continue;
        }
        // The nearest of the three hits.
        let far = 10000.0f32;
        let da = side_a.map_or(far, |(h, _)| lsq(sub(h, *centre)));
        let db = side_b.map_or(far, |(h, _)| lsq(sub(h, *centre)));
        let dc = ahead.map_or(far, |(h, _)| lsq(sub(h, *centre)));
        let (hit, mut n) = if db <= da || dc <= da {
            if dc <= db { ahead.unwrap() } else { side_b.unwrap() }
        } else {
            side_a.unwrap()
        };
        let mut radius = r;
        if 0.0 < d[1] && n[1] < slope {
            radius = n[1] * n[1] * r + r;
            n = if n[1] <= -0.99 || lsq(n) <= 0.001 || (n[0].abs() <= 0.001 && n[2].abs() <= 0.001) {
                scale(d, -1.0)
            } else {
                unit([n[0], 0.0, n[2]])
            };
        }
        let back = scale(d, -1.0);
        let cosine = dot(back, n).abs();
        let stop = if cosine <= 0.0 {
            if r * r < lsq(sub(hit, *centre)) { add(hit, scale(back, r)) } else { *centre }
        } else {
            let k = radius / cosine;
            if k * k <= lsq(sub(hit, *centre)) { add(hit, scale(back, k)) } else { *centre }
        };
        if stop != *centre && advance(centre, sub(stop, *centre), across, budget, top) == Some(true) {
            return;
        }
        // Slide: along the face, across the line from the face to the centre.
        let away = sub(*centre, hit);
        if is_zero(away) {
            return;
        }
        let away = unit(away);
        let off = lsq(sub(away, n));
        if off < 1e-5 || off > 3.999_989_986_419_677_7 {
            return;
        }
        if is_zero(n) {
            return;
        }
        let along = unit(cross(cross(away, n), n));
        let forward = dir[2] * along[2] + along[0] * dir[0] + dir[1] * along[1];
        if forward <= 0.0 {
            return;
        }
        target = add(*centre, scale(along, 10.0));
        if n[1] == 0.0 && centre[1] < target[1] {
            target[1] = centre[1];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A floor at y 0.5 over x, z 0..40 and a wall at x 20 (facing -x), as zone triangles (y up).
    fn ground() -> Ground {
        let mut z = Zones::default();
        z.add(&[[[0.0, 0.5, 0.0], [0.0, 0.5, 40.0], [40.0, 0.5, 0.0]], [[40.0, 0.5, 0.0], [0.0, 0.5, 40.0], [40.0, 0.5, 40.0]]]);
        let wall = [[[20.0, 0.0, 0.0], [20.0, 10.0, 0.0], [20.0, 0.0, 40.0]], [[20.0, 10.0, 0.0], [20.0, 10.0, 40.0], [20.0, 0.0, 40.0]]];
        // (facing whichever way the records make it: keep the one that faces -x)
        let probe = {
            let mut p = Zones::default();
            p.add(&wall);
            p.line([10.0, 2.0, 5.0], [30.0, 2.0, 5.0]).is_some()
        };
        if probe {
            z.add(&wall);
        } else {
            z.add(&[[wall[0][0], wall[0][2], wall[0][1]], [wall[1][0], wall[1][2], wall[1][1]]]);
        }
        Ground { zones: Arc::new(z), terrain: None, offset: 0.0 }
    }

    #[test]
    fn walks_on_the_floor_and_stops_at_the_wall() {
        let g = ground();
        let mut body = Body::default();
        let mut events = Vec::new();
        let mut pos = [10.0, 0.51, 5.0];
        // a second running east at 5 a second, in small steps
        for _ in 0..60 {
            body.run(&g, &mut pos, [5.0, 0.0], 1.0 / 60.0, &mut events);
        }
        assert!((pos[1] - 0.51).abs() < 0.02, "on the floor: {pos:?}");
        assert!((pos[0] - 15.0).abs() < 0.2, "moved 5: {pos:?}");
        for _ in 0..120 {
            body.run(&g, &mut pos, [5.0, 0.0], 1.0 / 60.0, &mut events);
        }
        assert!(pos[0] < 20.0 - 0.3 && pos[0] > 19.0, "held by the wall: {pos:?}");
        assert!(!body.falling, "{events:?}");
    }

    #[test]
    fn slides_along_a_wall_met_at_an_angle() {
        let g = ground();
        let mut body = Body::default();
        let mut events = Vec::new();
        let mut pos = [19.0, 0.51, 5.0];
        for _ in 0..60 {
            body.run(&g, &mut pos, [3.0, 3.0], 1.0 / 60.0, &mut events);
        }
        assert!(pos[0] < 20.0 && pos[2] > 7.0, "slid north along it: {pos:?}");
    }

    /// A quad's two triangles wound so the KD records' normal ((p0 − p2) × (p1 − p0)) points along `facing`.
    fn quad(z: &mut Zones, q: [V3; 4], facing: V3) {
        for t in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
            let n = cross(sub(t[0], t[2]), sub(t[1], t[0]));
            z.add(&[if dot(n, facing) > 0.0 { t } else { [t[0], t[2], t[1]] }]);
        }
    }

    /// A floor at 0 over x, z 0..40 with a block from x 20 to 30 of `height` (its top and its west face).
    fn step(height: f32) -> Ground {
        let mut z = Zones::default();
        quad(&mut z, [[0.0, 0.0, 0.0], [20.0, 0.0, 0.0], [20.0, 0.0, 40.0], [0.0, 0.0, 40.0]], [0.0, 1.0, 0.0]);
        quad(&mut z, [[20.0, height, 0.0], [30.0, height, 0.0], [30.0, height, 40.0], [20.0, height, 40.0]], [0.0, 1.0, 0.0]);
        quad(&mut z, [[20.0, 0.0, 0.0], [20.0, height, 0.0], [20.0, height, 40.0], [20.0, 0.0, 40.0]], [-1.0, 0.0, 0.0]);
        Ground { zones: Arc::new(z), terrain: None, offset: 0.0 }
    }

    /// A floor at 0 to x 20, then a ramp up at `degrees` to x 30, flat beyond.
    fn ramp(degrees: f32) -> Ground {
        let rise = 10.0 * degrees.to_radians().tan();
        let mut z = Zones::default();
        quad(&mut z, [[0.0, 0.0, 0.0], [20.0, 0.0, 0.0], [20.0, 0.0, 40.0], [0.0, 0.0, 40.0]], [0.0, 1.0, 0.0]);
        quad(&mut z, [[20.0, 0.0, 0.0], [30.0, rise, 0.0], [30.0, rise, 40.0], [20.0, 0.0, 40.0]], [0.0, 1.0, 0.0]);
        quad(&mut z, [[30.0, rise, 0.0], [40.0, rise, 0.0], [40.0, rise, 40.0], [30.0, rise, 40.0]], [0.0, 1.0, 0.0]);
        Ground { zones: Arc::new(z), terrain: None, offset: 0.0 }
    }

    fn run_east(g: &Ground, from: V3, seconds: f32) -> (V3, Vec<Event>) {
        let (mut body, mut events, mut pos) = (Body::default(), Vec::new(), from);
        for _ in 0..(seconds * 60.0) as usize {
            body.run(g, &mut pos, [5.0, 0.0], 1.0 / 60.0, &mut events);
        }
        (pos, events)
    }

    #[test]
    fn steps_are_climbed_and_ledges_are_not() {
        let (pos, _) = run_east(&step(0.3), [15.0, 0.01, 5.0], 2.0);
        assert!(pos[0] > 22.0 && (pos[1] - 0.31).abs() < 0.02, "up the step: {pos:?}");
        let (pos, _) = run_east(&step(1.0), [15.0, 0.01, 5.0], 2.0);
        assert!(pos[0] < 20.0 && pos[1] < 0.1, "held by the ledge: {pos:?}");
    }

    #[test]
    fn gentle_ramps_are_walked_and_steep_ones_are_not() {
        let (pos, _) = run_east(&ramp(30.0), [15.0, 0.01, 5.0], 4.0);
        assert!(pos[0] > 30.0 && pos[1] > 5.0, "up the ramp: {pos:?}");
        let (pos, _) = run_east(&ramp(70.0), [15.0, 0.01, 5.0], 4.0);
        assert!(pos[0] < 22.0 && pos[1] < 3.0, "not up the cliff: {pos:?}");
    }

    #[test]
    fn a_jump_rises_and_lands() {
        let g = ground();
        let mut body = Body::default();
        let mut events = Vec::new();
        let mut pos = [10.0, 0.51, 5.0];
        body.jump(&g, pos, 1.0, 1.0, &mut events);
        let mut highest = pos[1];
        for _ in 0..120 {
            body.run(&g, &mut pos, [0.0, 0.0], 1.0 / 60.0, &mut events);
            highest = highest.max(pos[1]);
        }
        assert!((highest - 1.51).abs() < 0.1, "rose 1 m: {highest}");
        assert!(!body.falling && (pos[1] - 0.51).abs() < 0.02, "landed: {pos:?} {events:?}");
        assert!(events.iter().any(|e| matches!(e, Event::Landed(_))));
    }
}
