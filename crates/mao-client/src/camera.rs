//! The game's camera, as `n3Camera_t` and its `CameraVehicle_t`s (N3.dll) and `Vehicle_t` (Vehicle.dll) drive it. The
//! camera is a steered vehicle chasing a point behind the character, not a yaw/pitch/distance orbit:
//!
//! - **Modes** (`PreferredCameraMode`, the `3rdPersonCamera` switch): 0 first person (`CameraVehicleFirstPerson_t`),
//!   1 the free camera (`CameraVehicle_t`, FUN_1001f9c9: it keeps whatever direction it has and is pulled along at
//!   the preferred distance, so it ends up behind the character as it walks), 2 the rubber camera and 3 the locked camera
//!   (`CameraVehicleFixedThird_t`, FUN_1001faa1, with `snap` false / true: the preferred direction turns with the
//!   character; the rubber one steers to that point, the locked one is put there). The default when nothing is
//!   preferred is 1 (FUN_10021859).
//! - **The vehicle** (`Vehicle_t::Run` → FUN_1000e3d3, substeps of at most 0.4 s): `CalcSteering` gives a force, clamped
//!   to the maximum force, which changes the velocity by `force × dt / mass`; the speed is clamped to the maximum
//!   velocity and the position moves by `velocity × dt`. `SteeringArrive` (0x1000ab28): the wanted speed is
//!   `|to target| / brake distance × max velocity` (at most the maximum), the force `(wanted − velocity) × mass × 4`;
//!   within 0.01 of the target it halts. Each frame (FUN_10022345) the mass is 20 and `UpdateMotionConstraints` sets
//!   the maximum velocity to the speed (the character's run speed × 6, at most 16), the maximum force to
//!   `mass × speed / 0.3` and the brake distance to `speed² × mass / force` (0.3 × the speed: 4.8 at the full 16).
//! - **The preferred position** (`RecalcOptimalPos` 0x1001f371): the look target minus the preferred direction (default
//!   (0, 1.5, −4.5) normalised, `PreferredCamPosX/Y/Z`) times the preferred distance (default 5, `PreferredCamDist`),
//!   the direction turned by the character's rotation for the fixed cameras. With no line of sight from the look target
//!   to it, a bisection (20 steps, until within 0.001) finds the nearest fraction of the way that is seen, and the
//!   camera goes there instead.
//! - **Zoom and turning by key** (FUN_10022345 → FUN_1002118c): the rotate keys turn the camera round the target by
//!   0.02 rad a frame (× `MouseTurnSensitivity`), the zoom keys move it along the view by `dt × ZoomSpeed`; the
//!   distance is kept within 0.8 (any closer while zooming in switches to first person when `ZoomTo1stPerson` is
//!   set) and 25. The mouse look (`MouseCameraControl`) is the same turn; in first person it turns the head, the
//!   pitch kept within ±1.55334 rad.
//! - **Reset** (`N3Msg_ResetCamera` → FUN_10020e0c): the camera goes behind the character at the same horizontal
//!   distance, looking from where it was (first person: straight ahead).
//! - The look target is the character's camera attractor (`Attractor31_camera`, FUN_10020bdb) smoothed frame by frame,
//!   at least 0.3 above the feet: [`LookTarget`].
//! - The field of view is 90° across (`VisualCamera_t::SetViewPlaneWindow(1.5708, aspect)`, FUN_1002107a).
//!
//! - **The free camera's sensors** (`UpdateSensors` 0x1001e71f, `CalculateSensorSteerDir` 0x1001d955): with the character
//!   out of the camera's sight, the camera is flexed 0.5, 2, 8 and 32 units along its up, its down, the world's up and down,
//!   its forward and back, in that order, until a flexed place sees the character (from half a unit back along the flex)
//!   and is itself reached from the camera; the steering target is then nudged a unit that way (`CalcSteering` 0x1001e797).
//!   No flex seeing it, the camera is blind (`IsBlind`). Seeing the character, the target is kept 0.4 up when the camera
//!   sits under it.
//! - **Room camera attractors**: see `attractor`.
//!
//! - **The playfield** is asked through [`Surface`]: the line of sight (`n3Playfield_t::LineOfSight`: the rooms' doors and
//!   the surface's `GetLineIntersection` both ways), the distance to the surface's closest point (seeing the character, a
//!   camera within 0.4 of it has its target lifted 0.4: `CalcSteering`, `_DAT_1003d9d0`), and `Vehicle_t::EnsureSurfaceAlignment`
//!   after every step of the vehicle (the camera's sphere swept along the step and held off the surface: the camera's own
//!   collision, which a blind camera relies on).
//!
//! Not ported: the two-shot and secondary-target framing.
//!
//! All positions here are in the server's frame (x, height, z); the character's heading is 0 along +z and turns
//! towards +x as it grows (`rotate_y`).

/// `PreferredCameraMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    First = 0,
    Free = 1,
    Rubber = 2,
    Locked = 3,
}

/// The playfield as `CameraVehicle_t` asks `n3Playfield_t` and its surface about it.
pub trait Surface {
    /// `n3Playfield_t::LineOfSight` (0x1000d34c): in a dungeon both points lie in rooms, the same one or two with an open door
    /// between (`RoomCanSeeFromTo`), and the surface's `GetLineIntersection` finds nothing from the one to the other nor back.
    fn line_of_sight(&mut self, a: [f32; 3], b: [f32; 3]) -> bool;
    /// How far the point is from the surface's closest point (`CalculateClosestPoint`); none without a surface.
    fn surface_distance(&mut self, p: [f32; 3]) -> Option<f32>;
    /// `Vehicle_t::EnsureSurfaceAlignment` after a step from `previous` to `pos`: where the camera's sphere (radius 0.4, its
    /// bottom at the camera) comes to rest, swept along the step and held off the surface.
    fn align(&mut self, previous: [f32; 3], pos: [f32; 3]) -> [f32; 3];
}

/// A surface that is only a line-of-sight test (the tests): nothing to be near, nothing to be stopped by.
pub struct Sight<F>(pub F);

impl<F: FnMut([f32; 3], [f32; 3]) -> bool> Surface for Sight<F> {
    fn line_of_sight(&mut self, a: [f32; 3], b: [f32; 3]) -> bool {
        (self.0)(a, b)
    }

    fn surface_distance(&mut self, _: [f32; 3]) -> Option<f32> {
        None
    }

    fn align(&mut self, _: [f32; 3], pos: [f32; 3]) -> [f32; 3] {
        pos
    }
}

/// The look target's place on the character (`n3Camera_t`'s FUN_10020bdb, `CameraVehicle_t::SetEyeTargetLocalPos`): a point
/// in the character's frame (unturned, y up) that follows the body's camera attractor (`Attractor31_camera`, else the head's,
/// `AttractorMesh::GetName(0)`; no player body has the first: FUN_10020af1) — its x and z as they were when first found (times the body scale), its y live — and is
/// turned by the character's rotation and added to its position (`GetLookTargetPos`). Each frame it is smoothed: in first
/// person it goes to the attractor at once (`0.001 × old + 0.999 × new`); in third person a fifth of the way (`0.8 × old
/// + 0.2 × new`), or with `UseNoBobCamera` it holds still until the attractor has drifted over 0.25 away and then chases it a
/// hundredth of the way a frame until within 0.01. It is never under 0.3 (`_DAT_1003e29c`). It starts at 2 up (SetCamera,
/// `_DAT_1003c968`).
#[derive(Clone, Debug)]
pub struct LookTarget {
    pub local: [f32; 3],
    /// The attractor's x and z when first found.
    held: Option<[f32; 2]>,
    /// `DAT_1005c87c`: the no-bob chase under way.
    chasing: bool,
}

impl Default for LookTarget {
    fn default() -> LookTarget {
        LookTarget { local: [0.0, 2.0, 0.0], held: None, chasing: false }
    }
}

impl LookTarget {
    /// A frame: `attractor` is the camera attractor's place in the character's frame (unturned), when the body has one.
    pub fn update(&mut self, attractor: Option<[f32; 3]>, first_person: bool, no_bob: bool) -> [f32; 3] {
        let attractor = attractor.map(|a| {
            let held = *self.held.get_or_insert([a[0], a[2]]);
            [held[0], a[1], held[1]]
        });
        let old = self.local;
        let mut new = match (attractor, first_person, no_bob) {
            (Some(a), _, true) if !first_person => {
                let d = length(sub(old, a));
                if d >= 0.01 {
                    if self.chasing || d > 0.25 {
                        self.chasing = true;
                        add(scale(old, 0.99), scale(a, 0.01))
                    } else {
                        old
                    }
                } else {
                    self.chasing = false;
                    old
                }
            }
            (a, true, _) => add(scale(old, 0.001), scale(a.unwrap_or([0.0; 3]), 0.999)),
            (Some(a), false, _) if old[1] != 0.0 => add(scale(old, 0.8), scale(a, 0.2)),
            _ => old,
        };
        if new[1] < 0.3 {
            new[1] = 0.3;
        }
        self.local = new;
        new
    }

    /// The attractor's place forgotten (a new body: `+0x240`).
    pub fn new_body(&mut self) {
        self.held = None;
    }
}

/// What the player is doing to the camera this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Inputs {
    pub rotate_left: bool,
    pub rotate_right: bool,
    pub rotate_up: bool,
    pub rotate_down: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
    /// Mouse look, in radians: across (positive turns the view right) and up.
    pub mouse: (f32, f32),
    /// Mouse-wheel notches (positive zooms out), applied as that many quarter seconds of zoom.
    pub wheel: f32,
    pub reset: bool,
}

const MASS: f32 = 20.0;
const DEFAULT_SPEED: f32 = 16.0;
/// The maximum force is `mass × speed / 0.3` (`_DAT_1003d8b8`), which makes the brake distance `0.3 × speed`.
const FORCE_TIME: f32 = 0.3;
const SPEED_FACTOR: f32 = 6.0;
const DEFAULT_DISTANCE: f32 = 5.0;
const DEFAULT_DIRECTION: [f32; 3] = [0.0, 1.5, -4.5];
const MIN_DISTANCE: f32 = 0.8;
const MAX_DISTANCE: f32 = 25.0;
const ARRIVE_RADIUS: f32 = 0.01;
const MAX_STEP: f32 = 0.4;
const TURN_PER_FRAME: f32 = 0.02;
/// `MouseTurnSensitivity` and `ZoomSpeed` are interface values the client has no source for; 1 and 10 feel like the original.
const TURN_SENSITIVITY: f32 = 1.0;
const ZOOM_SPEED: f32 = 10.0;
const PITCH_LIMIT: f32 = 1.55334;
const UP: [f32; 3] = [0.0, 1.0, 0.0];
/// The vehicle's radius (`SetRadius`, 0.7), by which the found position is kept clear of what blocked it.
const RADIUS: f32 = 0.7;

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn length(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}
fn normalize(a: [f32; 3]) -> [f32; 3] {
    let l = length(a);
    if l > 1e-12 { scale(a, 1.0 / l) } else { a }
}
fn clamp_length(a: [f32; 3], max: f32) -> [f32; 3] {
    let l = length(a);
    if l > max && l > 0.0 { scale(a, max / l) } else { a }
}
/// A turn about the vertical: the character's heading `h` takes (0, 0, 1) to (sin h, 0, cos h).
pub fn rotate_y(v: [f32; 3], h: f32) -> [f32; 3] {
    let (s, c) = h.sin_cos();
    [v[0] * c + v[2] * s, v[1], -v[0] * s + v[2] * c]
}
/// Rodrigues' turn of `v` about the unit `axis` by `angle`.
fn rotate_about(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let (s, c) = angle.sin_cos();
    let k = cross(axis, v);
    add(add(scale(v, c), scale(k, s)), scale(axis, dot(axis, v) * (1.0 - c)))
}

#[derive(Clone, Debug)]
pub struct SteeringCamera {
    pub mode: Mode,
    /// The mode to come back to from first person.
    preferred: Mode,
    /// The vehicle: position, velocity, speed.
    pos: [f32; 3],
    vel: [f32; 3],
    speed: f32,
    placed: bool,
    /// A room's camera attractor the vehicle is drawn to (`CameraVehicle_t` +0x1a0, `PointCameraAttractor_t`'s steering:
    /// `SteeringArrive` to its point), in the server's frame; none outside such a room.
    pub attractor: Option<[f32; 3]>,
    /// `IsBlind` (+0x1e9): the free camera cannot see the character and no sensor found a way to.
    pub blind: bool,
    /// The preferred direction (in the character's frame for the fixed cameras) and distance (+0x1f8, +0x198).
    pref_dir: [f32; 3],
    pref_dist: f32,
    /// First person: the head's turn across and up (+0x158, +0x15c).
    first_yaw: f32,
    first_pitch: f32,
    max_vel: f32,
    max_force: f32,
    brake: f32,
}

impl SteeringCamera {
    pub fn new(mode: Mode) -> Self {
        let preferred = if mode == Mode::First { Mode::Free } else { mode };
        let len = length(DEFAULT_DIRECTION);
        SteeringCamera {
            mode,
            attractor: None,
            blind: false,
            preferred,
            pos: [0.0; 3],
            vel: [0.0; 3],
            speed: 0.0,
            placed: false,
            pref_dir: scale(DEFAULT_DIRECTION, 1.0 / len),
            pref_dist: len,
            first_yaw: 0.0,
            first_pitch: 0.0,
            max_vel: DEFAULT_SPEED,
            max_force: MASS * DEFAULT_SPEED / FORCE_TIME,
            brake: FORCE_TIME * DEFAULT_SPEED,
        }
    }

    /// A new camera for a character entering a playfield (`n3EngineClient_t::SetMainDynel` → `CreateCamera`, N3.dll
    /// 0x10007842): put at the character's place plus (0, 2, −1) turned by its rotation (2 up, 1 behind), at rest; from
    /// there it steers out to the preferred place as usual. (The old camera does not fly across the world to it.)
    pub fn create(&mut self, feet: [f32; 3], heading: f32) {
        self.pos = add(feet, rotate_y([0.0, 2.0, -1.0], heading));
        self.vel = [0.0; 3];
        self.speed = 0.0;
        self.placed = true;
        self.first_yaw = 0.0;
        self.first_pitch = 0.0;
    }

    /// `ToggleCameraView`: first person and back to the preferred third-person mode.
    pub fn toggle_view(&mut self) {
        if self.mode == Mode::First {
            self.set_mode(self.preferred);
        } else {
            self.set_mode(Mode::First);
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if mode == self.mode {
            return;
        }
        if mode == Mode::First {
            self.first_yaw = 0.0;
            self.first_pitch = 0.0;
        } else {
            self.preferred = mode;
            // (coming out of first person, the camera starts from where it is: unplaced, it goes to the preferred spot)
            self.placed = self.placed && self.mode != Mode::First;
            self.vel = [0.0; 3];
            self.speed = 0.0;
        }
        self.mode = mode;
    }

    pub fn is_first_person(&self) -> bool {
        self.mode == Mode::First
    }

    /// `UpdateMotionConstraints` (0x1001e602) with the frame's speed (FUN_10022345).
    fn constrain(&mut self, char_speed: f32, look: [f32; 3]) {
        let mut speed = if char_speed * SPEED_FACTOR <= DEFAULT_SPEED && char_speed > 0.0 { char_speed * SPEED_FACTOR } else { DEFAULT_SPEED };
        let d = length(sub(look, self.pos));
        if d > 6.0 && ((d - 5.0).sqrt() + 1.0) * speed > 80.0 {
            speed = 80.0;
        }
        speed = speed.max(2.0);
        self.max_vel = speed;
        self.max_force = MASS * speed / FORCE_TIME;
        self.brake = 2.0 * (speed * speed * MASS / self.max_force * 0.5);
    }

    /// The preferred position for the fixed cameras, and the one with a line of sight from the look target
    /// (`RecalcOptimalPos`): `None` when the preferred one is seen.
    fn optimal(&self, look: [f32; 3], heading: f32, surface: &mut dyn Surface) -> ([f32; 3], Option<[f32; 3]>) {
        let dir = rotate_y(self.pref_dir, heading);
        let optimal = add(look, scale(dir, self.pref_dist));
        if surface.line_of_sight(look, add(optimal, scale(dir, RADIUS))) {
            return (optimal, None);
        }
        let (mut lo, mut hi) = (0.01f32, 0.95f32);
        let mut blocked = look;
        for _ in 0..20 {
            let mid = (lo + hi) * 0.5;
            blocked = add(scale(look, 1.0 - mid), scale(optimal, mid));
            if surface.line_of_sight(look, add(blocked, scale(dir, RADIUS))) {
                lo = mid;
            } else {
                hi = mid;
            }
            if hi - lo <= 0.001 {
                break;
            }
        }
        (optimal, Some(blocked))
    }

    /// `CalculateSensorSteerDir` (0x1001d955): the camera flexed 0.5, 2, 8 and 32 units (`(1 << k) × 0.5`, k = 0, 2, 4, 6)
    /// along its up, its down, the world's up and down, its forward and back, in that order, until a flexed place sees the
    /// character from half a unit back along the flex and is itself reached from the camera (`CanSeeFlexedPos`): that unit
    /// direction, signed. None when no flex does.
    fn sensor_dir(&self, look: [f32; 3], surface: &mut dyn Surface) -> Option<[f32; 3]> {
        let forward = normalize(sub(look, self.pos));
        let mut up = sub(UP, scale(forward, dot(UP, forward)));
        up = if length(up) > 1e-6 { normalize(up) } else { UP };
        for k in [0u32, 2, 4, 6] {
            let flex = (1u32 << k) as f32 * 0.5;
            for (axis, sign) in [(up, 1.0), (up, -1.0), (UP, 1.0), (UP, -1.0), (forward, 1.0), (forward, -1.0)] {
                let d = scale(axis, sign);
                let flexed = add(self.pos, scale(d, flex));
                if surface.line_of_sight(sub(flexed, scale(d, 0.5)), look) && surface.line_of_sight(self.pos, flexed) {
                    return Some(d);
                }
            }
        }
        None
    }

    /// `SteeringArrive`: the force towards `target` (`None` to halt).
    fn arrive(&self, target: [f32; 3]) -> Option<[f32; 3]> {
        let to = sub(target, self.pos);
        let d2 = dot(to, to);
        if d2 < ARRIVE_RADIUS * ARRIVE_RADIUS || d2 < 0.01 {
            return None;
        }
        let d = d2.sqrt();
        let wanted = (d / self.brake * self.max_vel).min(self.max_vel);
        let desired = sub(scale(to, wanted / d), self.vel);
        Some(scale(desired, MASS * 4.0))
    }

    /// `SteeringCamArrive` (0x1001dc46): arriving, with the sideways push that takes the camera round the character
    /// instead of through it when the target lies beyond it.
    fn cam_arrive(&self, target: [f32; 3], look: [f32; 3]) -> Option<[f32; 3]> {
        let mut target = target;
        let to_look = {
            let mut v = sub(look, self.pos);
            v[1] = 0.0;
            v
        };
        let to_target = {
            let mut v = sub(target, self.pos);
            v[1] = 0.0;
            v
        };
        let (dl, dt) = (length(to_look), length(to_target));
        if dl > 1e-6 && dt > 1e-6 {
            let (nl, nt) = (scale(to_look, 1.0 / dl), scale(to_target, 1.0 / dt));
            let side = cross(nl, nt)[1];
            if dl < dt && dt > 1.0 && side.abs() < 0.4 && dot(nt, nl) > 0.0 {
                // the perpendicular (to the side the target lies), half a unit
                let perp = [to_look[2], 0.0, -to_look[0]];
                let perp = scale(normalize(perp), if side < 0.0 { -1.0 } else { 1.0 } * 0.5);
                target = add(target, perp);
            }
        }
        self.arrive(target)
    }

    /// `Vehicle_t::Run`: the force applied in substeps.
    fn integrate(&mut self, force: Option<[f32; 3]>, dt: f32, surface: &mut dyn Surface) {
        let mut left = dt.min(4.0);
        while left > 0.0 {
            let step = left.min(MAX_STEP);
            left -= step;
            match force {
                None => {
                    self.vel = [0.0; 3];
                    self.speed = 0.0;
                }
                Some(f) => {
                    let f = clamp_length(f, self.max_force);
                    self.vel = add(self.vel, scale(f, step / MASS));
                    self.vel = clamp_length(self.vel, self.max_vel);
                    self.speed = length(self.vel);
                }
            }
            let previous = self.pos;
            if self.speed > 0.001 {
                self.pos = add(self.pos, scale(self.vel, step));
            }
            // (`EnsureSurfaceAlignment` after every step of `Vehicle_t::Run`, FUN_1000e3d3)
            self.pos = surface.align(previous, self.pos);
        }
    }

    /// The turn and zoom by key or mouse (FUN_1002118c) applied to the camera's vector from the look target.
    fn turn_and_zoom(&mut self, look: [f32; 3], heading: f32, across: f32, up: f32, zoom: f32) -> bool {
        let mut v = sub(self.pos, look);
        if length(v) < 1e-6 {
            v = scale(rotate_y(self.pref_dir, heading), self.pref_dist.max(MIN_DISTANCE));
        }
        // (across positive turns the view right: the camera goes round to the left)
        v = rotate_y(v, across);
        let axis = normalize(cross(v, UP));
        if (dot(normalize(v), UP) + up).abs() <= 0.9999 && length(axis) > 0.0 {
            v = rotate_about(v, axis, up);
        }
        v = add(v, scale(normalize(v), zoom * ZOOM_SPEED));
        let len = length(v);
        if len < MIN_DISTANCE && zoom < 0.0 {
            // zoomed right in: first person
            self.set_mode(Mode::First);
            return false;
        }
        if len > MAX_DISTANCE {
            v = scale(normalize(v), MAX_DISTANCE);
        }
        self.pos = add(look, v);
        if self.mode != Mode::Free {
            // `UpdateHeadingToPos(pos, false)`: the preferred direction carries the new vector (its length over the preferred distance)
            self.pref_dir = scale(rotate_y(v, -heading), 1.0 / self.pref_dist);
        }
        if self.speed < 0.02 {
            self.vel = [0.0; 3];
            self.speed = 0.0;
        }
        true
    }

    /// `ResetCamera` (FUN_10020e0c): behind the character at the same horizontal distance.
    fn reset(&mut self, look: [f32; 3], heading: f32) {
        if self.mode == Mode::First {
            self.first_yaw = 0.0;
            self.first_pitch = 0.0;
            return;
        }
        let v = sub(self.pos, look);
        let flat = (v[0] * v[0] + v[2] * v[2]).sqrt().max(MIN_DISTANCE);
        let behind = rotate_y([0.0, v[1], -flat], heading);
        self.pos = add(look, behind);
        // `UpdateHeadingToPos(pos, true)`: the distance measured anew
        self.pref_dist = length(behind).max(MIN_DISTANCE);
        self.pref_dir = scale(rotate_y(behind, -heading), 1.0 / self.pref_dist);
        self.vel = [0.0; 3];
        self.speed = 0.0;
    }

    /// One frame: the camera's eye and the point it looks at, in the server's frame. `look` is the look target (the
    /// character's head), `heading` the character's, `char_speed` its speed, `los` whether the segment between two
    /// points is clear.
    pub fn update(&mut self, dt: f32, look: [f32; 3], heading: f32, char_speed: f32, inputs: Inputs, surface: &mut dyn Surface) -> ([f32; 3], [f32; 3]) {
        let dt = dt.clamp(0.0, 0.25);
        if inputs.reset {
            self.reset(look, heading);
        }
        if self.mode == Mode::First {
            self.first_yaw += inputs.mouse.0 + (inputs.rotate_right as i32 - inputs.rotate_left as i32) as f32 * TURN_PER_FRAME * TURN_SENSITIVITY * (dt * 60.0);
            self.first_pitch = (self.first_pitch + inputs.mouse.1 + (inputs.rotate_up as i32 - inputs.rotate_down as i32) as f32 * TURN_PER_FRAME * TURN_SENSITIVITY * (dt * 60.0)).clamp(-PITCH_LIMIT, PITCH_LIMIT);
            if inputs.zoom_out || inputs.wheel > 0.0 {
                // (`ZoomTo1stPerson`: zooming out leaves first person)
                self.set_mode(self.preferred);
                self.pos = add(look, scale(rotate_y(self.pref_dir, heading), self.pref_dist));
                self.placed = true;
            }
            let forward = rotate_y([0.0, 0.0, 1.0], heading + self.first_yaw);
            let (sp, cp) = self.first_pitch.sin_cos();
            let dir = [forward[0] * cp, sp, forward[2] * cp];
            return (look, add(look, dir));
        }
        // Third person.
        if !self.placed {
            self.pos = add(look, scale(rotate_y(self.pref_dir, heading), self.pref_dist));
            self.placed = true;
        }
        self.constrain(char_speed, look);
        // The keys and the mouse (FUN_10022345: 0.02 a frame; the client scales it to the original's frame rate).
        let frames = dt * 60.0;
        let across = inputs.mouse.0 + (inputs.rotate_right as i32 - inputs.rotate_left as i32) as f32 * TURN_PER_FRAME * TURN_SENSITIVITY * frames;
        let up = inputs.mouse.1 + (inputs.rotate_up as i32 - inputs.rotate_down as i32) as f32 * TURN_PER_FRAME * TURN_SENSITIVITY * frames;
        let zoom = (inputs.zoom_out as i32 - inputs.zoom_in as i32) as f32 * dt + inputs.wheel * 0.25;
        if across != 0.0 || up != 0.0 || zoom != 0.0 {
            if !self.turn_and_zoom(look, heading, across, up, zoom) {
                return self.update(0.0, look, heading, char_speed, Inputs::default(), surface);
            }
        }
        // The steering (CalcSteering) and the vehicle's step. Drawn to a room's camera attractor, the vehicle arrives at its
        // point instead (`PointCameraAttractor_t`'s steering, slot 1 of its table: `SteeringArrive` with a 0.1 radius) and
        // keeps looking at the character.
        if let Some(at) = self.attractor.filter(|_| self.mode != Mode::First) {
            let force = self.arrive(at);
            self.integrate(force, dt, surface);
            return (self.pos, look);
        }
        let force = match self.mode {
            Mode::Free => {
                // (`CalcSteering` 0x1001e797) the camera keeps its direction and is pulled along at the preferred distance
                let mut dir = sub(self.pos, look);
                if length(dir) < 1e-6 {
                    dir = rotate_y(self.pref_dir, heading);
                }
                let dir = normalize(dir);
                let mut target = add(look, scale(dir, self.pref_dist));
                // The sensors (`UpdateSensors`): out of sight of the character, the target is nudged a unit along the first
                // direction a flexed camera sees it from; no such direction leaves the camera blind.
                if surface.line_of_sight(self.pos, look) {
                    self.blind = false;
                    // (seeing it, a camera within 0.4 of the surface's closest point has its target lifted 0.4: `_DAT_1003d9d0`)
                    if surface.surface_distance(self.pos).is_some_and(|d| d < 0.4) {
                        target[1] += 0.4;
                    }
                } else if let Some(steer) = self.sensor_dir(look, surface) {
                    self.blind = false;
                    target = add(target, steer);
                } else {
                    self.blind = true;
                }
                if self.pos[1] < look[1] {
                    target[1] += 0.4;
                }
                self.cam_arrive(target, look)
            }
            Mode::Rubber => {
                let (optimal, blocked) = self.optimal(look, heading, surface);
                self.cam_arrive(blocked.unwrap_or(optimal), look)
            }
            Mode::Locked | Mode::First => {
                // `DecideSnap`: put there, easing out at a tenth a frame when the way opens
                let (optimal, blocked) = self.optimal(look, heading, surface);
                let wanted = blocked.unwrap_or(optimal);
                let v = sub(wanted, look);
                let d = length(v);
                let current = length(sub(self.pos, look));
                self.pos = if d <= current { wanted } else { add(look, scale(normalize(v), current * 0.9 + d * 0.1)) };
                self.vel = [0.0; 3];
                self.speed = 0.0;
                None
            }
        };
        if self.mode != Mode::Locked {
            self.integrate(force, dt, surface);
        }
        (self.pos, look)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear(_: [f32; 3], _: [f32; 3]) -> bool {
        true
    }

    #[test]
    fn the_look_target_follows_the_camera_attractor() {
        let mut t = LookTarget::default();
        assert_eq!(t.local, [0.0, 2.0, 0.0]);
        // third person: a fifth of the way each frame, the x and z held as first found, the y live
        let p = t.update(Some([0.1, 1.5, 0.05]), false, false);
        assert!((p[1] - 1.9).abs() < 1e-5 && (p[0] - 0.02).abs() < 1e-5, "{p:?}");
        for _ in 0..60 {
            t.update(Some([0.3, 1.6, 0.0]), false, false);
        }
        let p = t.local;
        assert!((p[1] - 1.6).abs() < 1e-3 && (p[0] - 0.1).abs() < 1e-3 && (p[2] - 0.05).abs() < 1e-3, "{p:?}");
        // never under 0.3; first person goes to the attractor at once
        assert_eq!(t.update(Some([0.0, 0.0, 0.0]), true, false)[1], 0.3);
        // no bob: still until the attractor has gone over 0.25 away, then chasing a hundredth a frame
        let mut t = LookTarget::default();
        let y = t.update(Some([0.0, 1.5, 0.0]), true, false)[1];
        assert!((y - 1.5005).abs() < 1e-5);
        assert_eq!(t.update(Some([0.0, 1.7, 0.0]), false, true)[1], y);
        let p = t.update(Some([0.0, 1.8, 0.0]), false, true);
        assert!((p[1] - (y * 0.99 + 0.018)).abs() < 1e-5, "{p:?}");
    }

    /// A flat floor at height `y` with nothing else: seen across, near when the camera is low, and the camera held a
    /// hundredth above it.
    struct Floor(f32);

    impl Surface for Floor {
        fn line_of_sight(&mut self, a: [f32; 3], b: [f32; 3]) -> bool {
            (a[1] - self.0) * (b[1] - self.0) > 0.0
        }

        fn surface_distance(&mut self, p: [f32; 3]) -> Option<f32> {
            Some((p[1] - self.0).abs())
        }

        fn align(&mut self, _: [f32; 3], pos: [f32; 3]) -> [f32; 3] {
            [pos[0], pos[1].max(self.0 + 0.01), pos[2]]
        }
    }

    #[test]
    fn the_free_camera_near_the_ground_is_lifted_and_never_under_it() {
        // (the character stands at 10 with its head at 11.3; the camera starts 1.5 up and is pulled down to the floor's height)
        let mut c = SteeringCamera::new(Mode::Free);
        let look = [100.0, 11.3, 100.0];
        c.update(0.016, look, 0.0, 0.0, Inputs::default(), &mut Sight(clear));
        c.pos[1] = 10.05;
        let mut floor = Floor(10.0);
        for _ in 0..60 {
            c.update(0.016, look, 0.0, 0.0, Inputs::default(), &mut floor);
            assert!(c.pos[1] >= 10.01 - 1e-5, "{:?}", c.pos);
        }
        // within 0.4 of the floor the target went 0.4 up (and 0.4 more for sitting under the look target): it climbed
        assert!(c.pos[1] > 10.3, "{:?}", c.pos);
    }

    #[test]
    fn the_free_camera_s_sensors_lift_it_over_what_hides_the_character() {
        let mut c = SteeringCamera::new(Mode::Free);
        let look = [100.0, 10.0, 100.0];
        // placed behind the character (z 95.5, 1.5 up), then a wall 11.8 high at z 97 between them: the direct line crosses
        // it at 11.0, the camera flexed two units up sees over it
        c.update(0.016, look, 0.0, 0.0, Inputs::default(), &mut Sight(clear));
        let before = c.pos;
        let mut blocked = |a: [f32; 3], b: [f32; 3]| {
            if (a[2] - 97.0) * (b[2] - 97.0) >= 0.0 {
                return true;
            }
            let t = (97.0 - a[2]) / (b[2] - a[2]);
            a[1] + (b[1] - a[1]) * t >= 11.8
        };
        for _ in 0..30 {
            c.update(0.016, look, 0.0, 0.0, Inputs::default(), &mut Sight(blocked));
        }
        assert!(!c.blind);
        assert!(c.pos[1] > before[1] + 0.05, "{:?} -> {:?}", before, c.pos);
    }

    #[test]
    fn the_locked_camera_sits_behind_the_character() {
        let mut c = SteeringCamera::new(Mode::Locked);
        let look = [100.0, 10.0, 100.0];
        let (eye, _) = c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut Sight(clear));
        // facing +z: behind is -z, up a little, at the default distance 4.74
        assert!((eye[2] - (100.0 - 4.5)).abs() < 1e-3 && (eye[1] - 11.5).abs() < 1e-3 && eye[0].abs() - 100.0 < 1e-3, "{eye:?}");
        // turned to face +x: the camera swings round at once
        let (eye, _) = c.update(0.016, look, std::f32::consts::FRAC_PI_2, 7.0, Inputs::default(), &mut Sight(clear));
        assert!((eye[0] - (100.0 - 4.5)).abs() < 1e-3 && (eye[2] - 100.0).abs() < 1e-3, "{eye:?}");
    }

    #[test]
    fn the_rubber_camera_arrives_rather_than_jumps() {
        let mut c = SteeringCamera::new(Mode::Rubber);
        let look = [0.0, 0.0, 0.0];
        c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut Sight(clear));
        let before = c.pos;
        let (eye, _) = c.update(0.016, look, std::f32::consts::FRAC_PI_2, 7.0, Inputs::default(), &mut Sight(clear));
        let moved = length(sub(eye, before));
        assert!(moved > 0.0 && moved < 0.2, "{moved}");
        // and gets there in a few seconds
        for _ in 0..300 {
            c.update(0.016, look, std::f32::consts::FRAC_PI_2, 7.0, Inputs::default(), &mut Sight(clear));
        }
        assert!((c.pos[0] + 4.5).abs() < 0.2 && c.pos[2].abs() < 0.2, "{:?}", c.pos);
    }

    #[test]
    fn the_free_camera_trails_the_walker() {
        let mut c = SteeringCamera::new(Mode::Free);
        let mut look = [0.0, 0.0, 0.0];
        c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut Sight(clear));
        // the character walks off along +x for a while: the camera ends up behind it along -x
        for _ in 0..600 {
            look[0] += 7.0 * 0.016;
            c.update(0.016, look, std::f32::consts::FRAC_PI_2, 7.0, Inputs::default(), &mut Sight(clear));
        }
        let v = sub(c.pos, look);
        assert!(v[0] < -3.0 && v[2].abs() < 1.0, "{v:?}");
        // (running, the camera lags the preferred spot by what the arrive speed allows: 7 m/s is 2.1 of the 4.8 brake distance)
        assert!(length(v) < 8.0, "{v:?}");
    }

    #[test]
    fn zoom_is_kept_within_the_limits_and_turns_to_first_person() {
        let mut c = SteeringCamera::new(Mode::Locked);
        let look = [0.0, 0.0, 0.0];
        c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut Sight(clear));
        for _ in 0..600 {
            c.update(0.016, look, 0.0, 7.0, Inputs { zoom_out: true, ..Default::default() }, &mut Sight(clear));
        }
        assert!((length(sub(c.pos, look)) - MAX_DISTANCE).abs() < 0.05);
        for _ in 0..600 {
            c.update(0.016, look, 0.0, 7.0, Inputs { zoom_in: true, ..Default::default() }, &mut Sight(clear));
            if c.mode == Mode::First {
                break;
            }
        }
        assert_eq!(c.mode, Mode::First);
        // zooming out comes back to the locked camera
        c.update(0.016, look, 0.0, 7.0, Inputs { zoom_out: true, ..Default::default() }, &mut Sight(clear));
        assert_eq!(c.mode, Mode::Locked);
    }

    #[test]
    fn a_blocked_view_pulls_the_camera_in() {
        let mut c = SteeringCamera::new(Mode::Locked);
        let look = [0.0, 0.0, 0.0];
        // a wall 2 units behind the character (at z = -2)
        let mut wall = |a: [f32; 3], b: [f32; 3]| !(a[2].min(b[2]) < -2.0);
        let (eye, _) = c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut Sight(wall));
        assert!(eye[2] > -2.0 && eye[2] < -1.0, "{eye:?}");
    }
}
