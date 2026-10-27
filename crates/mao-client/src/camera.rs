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
//! - The look target is the character's camera attractor (`Attractor31_camera`, FUN_10020bdb), at least 0.3 above the
//!   feet; the client uses the character's eye height.
//! - The field of view is 90° across (`VisualCamera_t::SetViewPlaneWindow(1.5708, aspect)`, FUN_1002107a).
//!
//! Not ported: the free camera's sensors round obstacles (`CalculateSensorSteerDir`), the two-shot and secondary-target
//! framing, the camera attractors of rooms, the vehicle's own collision. Lines of sight come from the caller (the
//! client tests the terrain only).
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
    fn optimal(&self, look: [f32; 3], heading: f32, los: &mut dyn FnMut([f32; 3], [f32; 3]) -> bool) -> ([f32; 3], Option<[f32; 3]>) {
        let dir = rotate_y(self.pref_dir, heading);
        let optimal = add(look, scale(dir, self.pref_dist));
        if los(look, add(optimal, scale(dir, RADIUS))) {
            return (optimal, None);
        }
        let (mut lo, mut hi) = (0.01f32, 0.95f32);
        let mut blocked = look;
        for _ in 0..20 {
            let mid = (lo + hi) * 0.5;
            blocked = add(scale(look, 1.0 - mid), scale(optimal, mid));
            if los(look, add(blocked, scale(dir, RADIUS))) {
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
    fn integrate(&mut self, force: Option<[f32; 3]>, dt: f32) {
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
            if self.speed > 0.001 {
                self.pos = add(self.pos, scale(self.vel, step));
            }
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
    pub fn update(&mut self, dt: f32, look: [f32; 3], heading: f32, char_speed: f32, inputs: Inputs, los: &mut dyn FnMut([f32; 3], [f32; 3]) -> bool) -> ([f32; 3], [f32; 3]) {
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
                return self.update(0.0, look, heading, char_speed, Inputs::default(), los);
            }
        }
        // The steering (CalcSteering) and the vehicle's step.
        let force = match self.mode {
            Mode::Free => {
                // the camera keeps its direction and is pulled along at the preferred distance
                let mut dir = sub(self.pos, look);
                if length(dir) < 1e-6 {
                    dir = rotate_y(self.pref_dir, heading);
                }
                let dir = normalize(dir);
                let mut target = add(look, scale(dir, self.pref_dist));
                if self.pos[1] < look[1] {
                    target[1] += 0.4;
                }
                // no line of sight: the nearest seen point of the way there (in place of the sensors)
                if !los(look, add(target, scale(dir, RADIUS))) {
                    let (mut lo, mut hi) = (0.01f32, 0.95f32);
                    for _ in 0..20 {
                        let mid = (lo + hi) * 0.5;
                        let p = add(scale(look, 1.0 - mid), scale(target, mid));
                        if los(look, add(p, scale(dir, RADIUS))) {
                            lo = mid;
                        } else {
                            hi = mid;
                        }
                        if hi - lo <= 0.001 {
                            break;
                        }
                    }
                    target = add(scale(look, 1.0 - lo), scale(target, lo));
                }
                self.cam_arrive(target, look)
            }
            Mode::Rubber => {
                let (optimal, blocked) = self.optimal(look, heading, los);
                self.cam_arrive(blocked.unwrap_or(optimal), look)
            }
            Mode::Locked | Mode::First => {
                // `DecideSnap`: put there, easing out at a tenth a frame when the way opens
                let (optimal, blocked) = self.optimal(look, heading, los);
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
            self.integrate(force, dt);
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
    fn the_locked_camera_sits_behind_the_character() {
        let mut c = SteeringCamera::new(Mode::Locked);
        let look = [100.0, 10.0, 100.0];
        let (eye, _) = c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut clear);
        // facing +z: behind is -z, up a little, at the default distance 4.74
        assert!((eye[2] - (100.0 - 4.5)).abs() < 1e-3 && (eye[1] - 11.5).abs() < 1e-3 && eye[0].abs() - 100.0 < 1e-3, "{eye:?}");
        // turned to face +x: the camera swings round at once
        let (eye, _) = c.update(0.016, look, std::f32::consts::FRAC_PI_2, 7.0, Inputs::default(), &mut clear);
        assert!((eye[0] - (100.0 - 4.5)).abs() < 1e-3 && (eye[2] - 100.0).abs() < 1e-3, "{eye:?}");
    }

    #[test]
    fn the_rubber_camera_arrives_rather_than_jumps() {
        let mut c = SteeringCamera::new(Mode::Rubber);
        let look = [0.0, 0.0, 0.0];
        c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut clear);
        let before = c.pos;
        let (eye, _) = c.update(0.016, look, std::f32::consts::FRAC_PI_2, 7.0, Inputs::default(), &mut clear);
        let moved = length(sub(eye, before));
        assert!(moved > 0.0 && moved < 0.2, "{moved}");
        // and gets there in a few seconds
        for _ in 0..300 {
            c.update(0.016, look, std::f32::consts::FRAC_PI_2, 7.0, Inputs::default(), &mut clear);
        }
        assert!((c.pos[0] + 4.5).abs() < 0.2 && c.pos[2].abs() < 0.2, "{:?}", c.pos);
    }

    #[test]
    fn the_free_camera_trails_the_walker() {
        let mut c = SteeringCamera::new(Mode::Free);
        let mut look = [0.0, 0.0, 0.0];
        c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut clear);
        // the character walks off along +x for a while: the camera ends up behind it along -x
        for _ in 0..600 {
            look[0] += 7.0 * 0.016;
            c.update(0.016, look, std::f32::consts::FRAC_PI_2, 7.0, Inputs::default(), &mut clear);
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
        c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut clear);
        for _ in 0..600 {
            c.update(0.016, look, 0.0, 7.0, Inputs { zoom_out: true, ..Default::default() }, &mut clear);
        }
        assert!((length(sub(c.pos, look)) - MAX_DISTANCE).abs() < 0.05);
        for _ in 0..600 {
            c.update(0.016, look, 0.0, 7.0, Inputs { zoom_in: true, ..Default::default() }, &mut clear);
            if c.mode == Mode::First {
                break;
            }
        }
        assert_eq!(c.mode, Mode::First);
        // zooming out comes back to the locked camera
        c.update(0.016, look, 0.0, 7.0, Inputs { zoom_out: true, ..Default::default() }, &mut clear);
        assert_eq!(c.mode, Mode::Locked);
    }

    #[test]
    fn a_blocked_view_pulls_the_camera_in() {
        let mut c = SteeringCamera::new(Mode::Locked);
        let look = [0.0, 0.0, 0.0];
        // a wall 2 units behind the character (at z = -2)
        let mut wall = |a: [f32; 3], b: [f32; 3]| !(a[2].min(b[2]) < -2.0);
        let (eye, _) = c.update(0.016, look, 0.0, 7.0, Inputs::default(), &mut wall);
        assert!(eye[2] > -2.0 && eye[2] < -1.0, "{eye:?}");
    }
}
