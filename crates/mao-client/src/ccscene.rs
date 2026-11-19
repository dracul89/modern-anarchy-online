//! The character creation's world and its camera: GUI.dll's `CharCreateModule_t` (0x1011cbcf) without its views (those
//! are `charcreate.rs`).
//!
//! **The camera file.** `CharCreateCamera.dat` in the game's folder is text: `CameraCount`, then for each camera an
//! `ID` of one to five numbers, `Pos`, `Rot` (a quaternion) and `degFOV` ([`crate::hall::places`]); then
//! `TransitionCount` and for each transition a `TransitionId`, a `Duration` in seconds and its `KeyframeCount`
//! cameras, each a `C:` line with a camera's id. [`CameraFile`] reads both.
//!
//! **The mover** (the module's camera control: its step is FUN_1011698d, play FUN_101167a6, the skip 0x10116959).
//! A transition runs from 0 to 1 in its duration; the place along it is eased, `(cos(pi (p - 1)) + 1) / 2`, and
//! measured along the straight lines between its cameras' places, so the camera keeps one speed whatever the
//! spacing of the cameras; between two cameras the place and the angle go in a straight line and the rotation by
//! the shortest turn. Each frame the result is then mixed with where the camera was, keeping `0.9 + (0.035 - dt)`
//! of the old (`dt` at most 0.035 s), which rounds the path's corners. The transition is over when it has run its
//! time and the camera is within a millimetre of the last camera; the module takes the camera to be "there" from
//! 0.1 units ([`Mover::at_rest`]). Escape puts the camera at the transition's end at once.
//!
//! **The module's states** (`FrameProcess` 0x1011c355), here [`Director`]: the intro in space (transition `5 1`,
//! 23 s, with the opening music; two lines of text after 4 s; the picture fades in over 10 s and out over the last
//! 3 s), then the way down through the station (`1 1 1`, 25 s) into the breed scene, then the four scenes in the
//! order breed, appearance, profession, name, and after a character is made the walk to the shuttle (`4 3`,
//! 10.8 s). A scene starts when the camera has come to rest; halfway there the chosen character is put on the
//! scene's connector (`character_0` breed, `character_1` appearance, `character_3` name) and the seven breed
//! figures and the profession hall's meshes are shown or hidden.
//!
//! **The scenes' camera orders** (each scene's `SlotNextPressed` / `SlotPrevPressed` and the appearance's
//! `MoveCamereToHead` / `MoveCameraToFullView`, the profession's `ProfessionChanged`): [`Director::go`]. Those that
//! start "from where the camera is" (the appearance scene's) are built by the original the same way: it writes the
//! camera's place into camera `2 3` and makes the transition from it.
//!
//! Not done: the meshes' own animations (the profession figures and the intro's ships stand still), the selection
//! glow's look (the hovered breed is named in the text only), the camera's debug display.

use crate::hall::{rotate, Place};

/// A transition of the camera file.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub id: Vec<u32>,
    pub seconds: f32,
    pub keys: Vec<Vec<u32>>,
}

/// `CharCreateCamera.dat`.
#[derive(Clone, Debug, Default)]
pub struct CameraFile {
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
}

impl CameraFile {
    pub fn parse(text: &str) -> CameraFile {
        let ids = |rest: &str| -> Vec<u32> { rest.split_whitespace().filter_map(|t| t.parse().ok()).collect() };
        let mut transitions: Vec<Transition> = Vec::new();
        for line in text.lines() {
            let Some((key, rest)) = line.split_once(':') else { continue };
            match key.trim() {
                "TransitionId" => transitions.push(Transition { id: ids(rest), seconds: 0.0, keys: Vec::new() }),
                "Duration" => {
                    if let (Some(t), Some(s)) = (transitions.last_mut(), rest.split_whitespace().next().and_then(|v| v.parse().ok())) {
                        t.seconds = s;
                    }
                }
                "C" => {
                    if let Some(t) = transitions.last_mut() {
                        t.keys.push(ids(rest));
                    }
                }
                _ => {}
            }
        }
        CameraFile { places: crate::hall::places(text), transitions }
    }

    pub fn place(&self, id: &[u32]) -> Option<&Place> {
        self.places.iter().find(|p| p.id == id)
    }

    pub fn transition(&self, id: &[u32]) -> Option<&Transition> {
        self.transitions.iter().find(|t| t.id == id)
    }

    /// A transition's cameras and its time.
    fn keys_of(&self, id: &[u32]) -> Option<(Vec<Place>, f32)> {
        let t = self.transition(id)?;
        let keys: Vec<Place> = t.keys.iter().filter_map(|k| self.place(k).cloned()).collect();
        (keys.len() == t.keys.len() && !keys.is_empty()).then_some((keys, t.seconds))
    }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// The turn from `a` to `b` by the shortest way, `t` of it (quaternions x, y, z, w).
pub fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let b = if dot < 0.0 {
        dot = -dot;
        b.map(|v| -v)
    } else {
        b
    };
    let (ka, kb) = if dot > 0.9995 {
        (1.0 - t, t)
    } else {
        let angle = dot.clamp(-1.0, 1.0).acos();
        (((1.0 - t) * angle).sin() / angle.sin(), (t * angle).sin() / angle.sin())
    };
    let q = [a[0] * ka + b[0] * kb, a[1] * ka + b[1] * kb, a[2] * ka + b[2] * kb, a[3] * ka + b[3] * kb];
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt().max(1e-9);
    q.map(|v| v / n)
}

/// The share of the old camera a frame keeps at its longest (FUN_1011698d: 0.9, and up to 0.035 more by the frame's
/// time).
const KEPT: f32 = 0.9;
const KEPT_TIME: f32 = 0.035;

/// The camera and the transition it is on.
#[derive(Clone, Debug)]
pub struct Mover {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub fov: f32,
    keys: Vec<Place>,
    seconds: f32,
    progress: f32,
    active: bool,
}

impl Default for Mover {
    fn default() -> Self {
        // (`InitialiseMessage`: the camera made at (0, 1, -5), unrotated, 60 degrees)
        Mover { position: [0.0, 1.0, -5.0], rotation: [0.0, 0.0, 0.0, 1.0], fov: 60.0, keys: Vec::new(), seconds: 1.0, progress: 0.0, active: false }
    }
}

impl Mover {
    /// The camera as a place.
    pub fn place(&self) -> Place {
        Place { id: Vec::new(), position: self.position, rotation: self.rotation, fov_degrees: self.fov }
    }

    /// Chooses a transition and plays it: the camera goes to its first camera at once (FUN_1011673d, then command
    /// 0x2e).
    pub fn play(&mut self, keys: Vec<Place>, seconds: f32) {
        let Some(first) = keys.first() else { return };
        (self.position, self.rotation, self.fov) = (first.position, first.rotation, first.fov_degrees);
        (self.keys, self.seconds, self.progress, self.active) = (keys, seconds.max(1e-3), 0.0, true);
    }

    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// The seconds the running transition takes (none running: 0).
    pub fn seconds(&self) -> f32 {
        if self.active {
            self.seconds
        } else {
            0.0
        }
    }

    /// FUN_1011573d: no transition runs, or the camera is within 0.1 of its last camera.
    pub fn at_rest(&self) -> bool {
        !self.active || self.keys.last().is_some_and(|k| distance(self.position, k.position) < 0.1)
    }

    /// Escape (command 0x31): the camera at the transition's end.
    pub fn skip(&mut self) {
        if !self.active {
            return;
        }
        if let Some(last) = self.keys.last() {
            (self.position, self.rotation, self.fov) = (last.position, last.rotation, last.fov_degrees);
        }
        (self.progress, self.active) = (1.0, false);
    }

    /// FUN_1011698d.
    pub fn step(&mut self, dt: f32) {
        if !self.active || self.keys.is_empty() {
            return;
        }
        self.progress = (self.progress + dt / self.seconds).min(1.0);
        let eased = ((std::f32::consts::PI * (self.progress - 1.0)).cos() + 1.0) * 0.5;
        let total: f32 = self.keys.windows(2).map(|w| distance(w[0].position, w[1].position)).sum();
        let old = (self.position, self.rotation, self.fov);
        let first = &self.keys[0];
        (self.position, self.rotation, self.fov) = (first.position, first.rotation, first.fov_degrees);
        let last = self.keys.len().saturating_sub(2);
        if total > 0.0 {
            let mut gone = 0.0;
            for (i, w) in self.keys.windows(2).enumerate() {
                let length = distance(w[0].position, w[1].position);
                // (two cameras at one place: the original leaves the frame here, the camera at the first camera)
                if length < 1e-4 {
                    return;
                }
                gone += length;
                if eased < gone / total || i == last {
                    let share = length / total;
                    let t = (eased - (gone / total - share)) / share;
                    self.position = mix(w[0].position, w[1].position, t);
                    self.rotation = slerp(w[0].rotation, w[1].rotation, t);
                    self.fov = w[0].fov_degrees * (1.0 - t) + w[1].fov_degrees * t;
                    break;
                }
            }
        }
        let kept = (KEPT_TIME - dt.min(KEPT_TIME)) + KEPT;
        self.position = mix(self.position, old.0, kept);
        self.rotation = slerp(old.1, self.rotation, 1.0 - kept);
        self.fov = self.fov * (1.0 - kept) + old.2 * kept;
        if self.progress >= 1.0 && self.keys.last().is_some_and(|k| distance(self.position, k.position) < 0.001) {
            self.active = false;
        }
    }

    /// Where the camera looks from, its direction and its upright, in the renderer's frame (Z up; the game's has Y).
    pub fn view(&self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let swap = |v: [f32; 3]| [v[0], v[2], v[1]];
        (swap(self.position), swap(rotate(self.rotation, [0.0, 0.0, 1.0])), swap(rotate(self.rotation, [0.0, 1.0, 0.0])))
    }
}

/// The four scenes, in the original's order (their ids 0x3e9 to 0x3ec).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    Breed,
    Appearance,
    Profession,
    Name,
}

/// Where the creation is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// In space (states 0x44d and 0x44e).
    Intro,
    /// The camera on its way to a scene (0x3f3 to 0x3f6).
    Entering(Scene),
    /// A scene running (state 0).
    In(Scene),
    /// The character is made: the way to the shuttle (0x4b1).
    Leaving,
}

/// A camera order of a scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Order {
    /// The breed scene's Next.
    BreedNext,
    /// The appearance scene's Next and Back; the mouse over the head and off it.
    AppearanceNext,
    AppearanceBack,
    Head([f32; 3]),
    FullView,
    /// A profession picked (the one before it, 0 for none).
    Profession { from: i32 },
    ProfessionNext,
    ProfessionBack,
    NameBack,
    /// The character was made.
    Made,
}

/// A line of the intro's text: its name in the texts' category 600, where it stands (shares of the window) and how
/// much of it shows (0 to 1).
#[derive(Clone, Debug, PartialEq)]
pub struct Caption {
    pub text: &'static str,
    pub at: [f32; 2],
    pub alpha: f32,
}

/// `TextWindowFade_t`'s times as the module makes its lines: shown for five seconds (six after the creation), of
/// which the first and last second fade. (The class takes the two numbers 5 and 4; the fade's own length was not
/// read, a second is this port's.)
fn caption_alpha(age: f32, shown: f32) -> f32 {
    (age.min(shown - age)).clamp(0.0, 1.0)
}

/// The creation's state machine.
pub struct Director {
    pub file: CameraFile,
    pub mover: Mover,
    pub phase: Phase,
    /// The seven breed figures show (the module's +0xa9), the profession hall's meshes show (+0xaa).
    pub figures: bool,
    pub professions: bool,
    /// The connector of the main mesh the chosen character stands on.
    pub connector: &'static str,
    /// Sounds to play, oldest first (the views add theirs).
    pub sounds: Vec<&'static str>,
    /// The ambience runs (+0xad).
    pub ambience: bool,
    /// The chosen profession (1 to 14, 0 none): the profession scene's cameras follow it.
    pub profession: i32,
    /// Seconds in the phase, and since the creation began.
    clock: f32,
    total: f32,
    intro_text: Option<f32>,
    /// The black over the picture, 0 to 1, and what it goes to and how fast (a second's share).
    pub black: f32,
    black_to: (f32, f32),
}

impl Director {
    /// `InitialiseMessage`, then the states 0x514 and 0x44d: everything made, the intro started.
    pub fn new(file: CameraFile) -> Director {
        let mut d = Director {
            file,
            mover: Mover::default(),
            phase: Phase::Intro,
            figures: true,
            professions: false,
            connector: "character_0",
            sounds: vec!["SM_Sandy_CC_Opening_Music"],
            ambience: false,
            profession: 0,
            clock: 0.0,
            total: 0.0,
            intro_text: None,
            black: 1.0,
            // (`RunIntro`: `PlainSpriteFade_t(10, 1)`, the picture comes up out of black in ten seconds)
            black_to: (0.0, 1.0 / 10.0),
        };
        // (`RunIntro`: the transition 5 1)
        if let Some((keys, seconds)) = d.file.keys_of(&[5, 1]) {
            d.mover.play(keys, seconds);
        }
        d
    }

    /// The creation straight in a scene, its camera at rest (the pictures the tests take).
    pub fn at_scene(file: CameraFile, scene: Scene, profession: i32) -> Director {
        let mut d = Director::new(file);
        d.sounds.clear();
        (d.black, d.black_to) = (0.0, (0.0, 1.0));
        d.profession = profession;
        let camera: Vec<u32> = match scene {
            Scene::Breed => vec![1, 1],
            Scene::Appearance => vec![2, 1],
            Scene::Profession if profession > 0 => vec![3, 1, profession as u32],
            Scene::Profession => vec![3, 1],
            Scene::Name => vec![4, 1],
        };
        if let Some(place) = d.file.place(&camera).cloned() {
            d.mover.play(vec![place], 1.0);
            d.mover.skip();
        }
        d.enter(scene);
        d.settle(scene);
        d.phase = Phase::In(scene);
        d
    }

    fn enter(&mut self, scene: Scene) {
        self.phase = Phase::Entering(scene);
        self.clock = 0.0;
        // (state 0x3eb: the profession hall shows from the moment its scene is chosen)
        if scene == Scene::Profession {
            self.professions = true;
        }
    }

    /// Halfway to a scene (the camera's transition half run, or the camera at rest): the character on the scene's
    /// connector, the figures and the hall shown or hidden.
    fn settle(&mut self, scene: Scene) {
        match scene {
            Scene::Breed => (self.connector, self.figures, self.professions) = ("character_0", true, false),
            Scene::Appearance => (self.connector, self.figures, self.professions) = ("character_1", false, false),
            Scene::Profession => (self.figures, self.professions) = (false, true),
            Scene::Name => (self.connector, self.figures, self.professions) = ("character_3", false, false),
        }
    }

    /// The scene that runs (its views take the mouse).
    pub fn scene(&self) -> Option<Scene> {
        match self.phase {
            Phase::In(scene) => Some(scene),
            _ => None,
        }
    }

    /// The intro's lines of text that show now.
    pub fn captions(&self) -> Vec<Caption> {
        let mut out = Vec::new();
        if let (Phase::Intro | Phase::Entering(Scene::Breed), Some(since)) = (self.phase, self.intro_text) {
            let alpha = caption_alpha(self.total - since, 5.0);
            if alpha > 0.0 {
                out.push(Caption { text: "MorningStar", at: [0.5, 0.9], alpha });
                out.push(Caption { text: "GeosynchronousOrbit", at: [0.5, 0.925], alpha });
            }
        }
        if self.phase == Phase::Leaving {
            // (`StaticDoneTimerCallback`: three lines, the first alone, then two)
            let first = caption_alpha(self.clock, 6.0);
            if first > 0.0 {
                out.push(Caption { text: "BoardingShuttle", at: [0.5, 0.9], alpha: first });
            }
        }
        out
    }

    /// Escape while the camera flies: the flight's end (`CharCreateModule_t::SlotEscPressed` 0x1011b69e: command 0x31; a
    /// click does not skip, the module's only other input being `CameraControlMessage`'s keys).
    pub fn skip(&mut self) {
        if matches!(self.phase, Phase::Intro | Phase::Entering(Scene::Breed)) {
            self.mover.skip();
        }
    }

    /// A frame (`FrameProcess`): the camera moved, the states followed. True when the walk to the shuttle is over.
    /// Seconds since the creation began, each frame counted as half a second at most: the time the meshes'
    /// animations run by (`CharCreateModule_t::FrameProcess`, GUI.dll 0x1011c355, gives every mesh the frame's
    /// time, in every scene).
    pub fn total(&self) -> f32 {
        self.total
    }

    pub fn frame(&mut self, dt: f32) -> bool {
        // (the original takes at most half a second for a frame)
        let dt = dt.clamp(0.0, 0.5);
        self.clock += dt;
        self.total += dt;
        self.black += (self.black_to.0 - self.black).clamp(-self.black_to.1 * dt, self.black_to.1 * dt);
        // (the first block: halfway to a scene)
        if let Phase::Entering(scene) = self.phase {
            if self.mover.progress() >= 0.5 || self.mover.at_rest() {
                self.settle(scene);
            }
        }
        match self.phase {
            Phase::Intro => {
                if self.mover.at_rest() {
                    // (the way down: the transition 1 1 1; the picture, should it be dark, comes back in four seconds)
                    if let Some((keys, seconds)) = self.file.keys_of(&[1, 1, 1]) {
                        self.mover.play(keys, seconds);
                    }
                    self.black_to = (0.0, 1.0 / 4.0);
                    self.enter(Scene::Breed);
                } else {
                    let gone = self.mover.progress() * self.mover.seconds();
                    let left = self.mover.seconds() - gone;
                    if gone > 4.0 && self.intro_text.is_none() {
                        self.intro_text = Some(self.total);
                    } else if left < 3.0 && self.black_to.0 == 0.0 && self.intro_text.is_some() {
                        // (`PlainSpriteFade_t(left + 2, 0)`: to black a little slower than the flight ends)
                        self.black_to = (1.0, 1.0 / (left + 2.0).max(0.1));
                    }
                }
            }
            Phase::Entering(scene) => {
                if self.mover.at_rest() {
                    self.phase = Phase::In(scene);
                    self.ambience = true;
                    match scene {
                        Scene::Breed => self.sounds.push("SM_Sandy_CC_Select_Breed"),
                        Scene::Profession => self.sounds.push("SM_Sandy_CC_Select_Profession"),
                        Scene::Name => self.sounds.push("SM_Sandy_CC_Name_Character"),
                        Scene::Appearance => self.sounds.push("SM_Sandy_CC_Appearance"),
                    }
                }
            }
            Phase::In(_) => {}
            Phase::Leaving => {
                self.mover.step(dt);
                return self.mover.at_rest() && self.black >= 1.0;
            }
        }
        self.mover.step(dt);
        false
    }

    fn place(&self, id: &[u32]) -> Option<Place> {
        self.file.place(id).cloned()
    }

    /// A scene's camera order, and the change of scene that goes with it.
    pub fn go(&mut self, order: Order) {
        let here = self.mover.place();
        let chosen = |d: &Director| (d.profession > 0).then(|| d.place(&[3, 1, d.profession as u32])).flatten();
        let (keys, seconds, to): (Vec<Option<Place>>, f32, Option<Scene>) = match order {
            // (the file's transition 1 1: the breed camera, round the corner, the appearance's)
            Order::BreedNext => match self.file.keys_of(&[1, 1]) {
                Some((keys, seconds)) => (keys.into_iter().map(Some).collect(), seconds, Some(Scene::Appearance)),
                None => (Vec::new(), 1.0, Some(Scene::Appearance)),
            },
            Order::AppearanceNext => {
                let mut keys = vec![Some(here), self.place(&[3, 1])];
                if self.profession > 0 {
                    keys.push(chosen(self));
                }
                (keys, 1.9, Some(Scene::Profession))
            }
            Order::AppearanceBack => (vec![Some(here), self.place(&[1, 1])], 1.9, Some(Scene::Breed)),
            // (`MoveCamereToHead`: camera 2 2's rotation and angle, put before the head)
            Order::Head(at) => {
                let face = self.place(&[2, 2]).map(|p| Place { position: at, ..p });
                (vec![Some(here), face], 0.3, None)
            }
            Order::FullView => (vec![Some(here), self.place(&[2, 1])], 0.3, None),
            Order::Profession { from } => {
                let start = if from < 1 { self.place(&[3, 1]) } else { self.place(&[3, 1, from as u32]) };
                // (the corridor's middle on the way: `ProfessionChanged` takes camera 3 2 n, n from how far down the
                // corridor the profession stands)
                let n = if self.profession > 6 { 14 - self.profession } else { self.profession };
                (vec![start, self.place(&[3, 2, (n / 2 + 1) as u32]), chosen(self)], 0.3, None)
            }
            Order::ProfessionNext => (vec![chosen(self), self.place(&[3, 1]), self.place(&[3, 3, 1]), self.place(&[3, 3, 2]), self.place(&[4, 1])], 3.4, Some(Scene::Name)),
            Order::ProfessionBack => (vec![chosen(self), self.place(&[3, 1]), self.place(&[2, 1])], 1.9, Some(Scene::Appearance)),
            Order::NameBack => (vec![self.place(&[4, 1]), self.place(&[3, 3, 2]), self.place(&[3, 3, 1]), self.place(&[3, 1]), chosen(self)], 3.7, Some(Scene::Profession)),
            Order::Made => match self.file.keys_of(&[4, 3]) {
                Some((keys, seconds)) => (keys.into_iter().map(Some).collect(), seconds, None),
                None => (Vec::new(), 1.0, None),
            },
        };
        let keys: Vec<Place> = keys.into_iter().flatten().collect();
        if keys.len() >= 2 {
            self.mover.play(keys, seconds);
        }
        if let Some(scene) = to {
            self.enter(scene);
        }
        if order == Order::Made {
            self.phase = Phase::Leaving;
            self.clock = 0.0;
            // (`StaticDoneTimerCallback`: the picture goes to black as the walk ends; two seconds for it)
            self.black_to = (1.0, 1.0 / 8.0);
        }
    }
}

/// The breed figures' connectors of the main mesh, by breed choice less one (GUI.dll's table at 0x10272490).
pub const BREED_CONNECTORS: [&str; 7] = ["breed_0", "breed_1", "breed_2", "breed_3", "breed_4", "breed_5", "breed_6"];

/// The creation's meshes (the table at 0x102724e4): the station, the profession hall's two, the intro's, the nano
/// effect and the adventurer. The hall's four (1, 3, 4, 5) show only from the profession scene on.
pub const MESHES: [&str; 6] = [
    "charactercreation_main.abiff",
    "charactercreation_professions.abiff",
    "charactercreation_seq01.abiff",
    "charactercreation_professions02.abiff",
    "charactercreation_nanoeffect.abiff",
    "charactercreation_adventurer.abiff",
];

/// Whether a mesh of [`MESHES`] is one of the profession hall's.
pub fn of_the_hall(mesh: usize) -> bool {
    matches!(mesh, 1 | 3 | 4 | 5)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> CameraFile {
        CameraFile::parse(
            "CameraCount: 3\nID: 1 \nPos: 0 0 0\nRot: 0 0 0 1\ndegFOV: 60\nID: 1 1 \nPos: 10 0 0\nRot: 0 1 0 0\ndegFOV: 80\nID: 2 \nPos: 10 0 30\nRot: 0 1 0 0\ndegFOV: 80\n\
             TransitionCount: 2\nTransitionId: 1 \nDuration: 1.8 sec\nKeyframeCount: 2\nC: 1 \nC: 1 1 \nTransitionId: 1 1 \nDuration: 4 sec\nKeyframeCount: 3\nC: 1 \nC: 1 1 \nC: 2 \n",
        )
    }

    #[test]
    fn the_file_gives_cameras_and_transitions() {
        let f = file();
        assert_eq!(f.places.len(), 3);
        assert_eq!(f.transitions, vec![
            Transition { id: vec![1], seconds: 1.8, keys: vec![vec![1], vec![1, 1]] },
            Transition { id: vec![1, 1], seconds: 4.0, keys: vec![vec![1], vec![1, 1], vec![2]] },
        ]);
        assert_eq!(f.place(&[1, 1]).unwrap().fov_degrees, 80.0);
        assert!(f.transition(&[2]).is_none());
    }

    /// A transition starts at its first camera, runs its time and ends at its last; halfway in time the eased place
    /// is halfway along the path, whatever the cameras' spacing (here a quarter and three quarters of the way).
    #[test]
    fn the_camera_runs_a_transition() {
        let f = file();
        let (keys, seconds) = f.keys_of(&[1, 1]).unwrap();
        let mut m = Mover::default();
        m.play(keys, seconds);
        assert_eq!((m.position, m.fov), ([0.0, 0.0, 0.0], 60.0));
        assert!(!m.at_rest());
        // (many small frames: the smoothing has almost nothing to hold back)
        for _ in 0..2000 {
            m.step(0.001);
        }
        // (two seconds of four: half of the 40 units, which is 10 units into the second stretch; the smoothing lags
        // a little)
        assert!((m.position[0] - 10.0).abs() < 0.01 && (m.position[2] - 10.0).abs() < 0.6, "{:?}", m.position);
        assert!((m.fov - 80.0).abs() < 1e-3);
        for _ in 0..4000 {
            m.step(0.001);
        }
        assert!(m.at_rest() && m.seconds() == 0.0);
        assert!((m.position[2] - 30.0).abs() < 0.01);
    }

    #[test]
    fn escape_ends_the_flight() {
        let f = file();
        let (keys, seconds) = f.keys_of(&[1]).unwrap();
        let mut m = Mover::default();
        m.play(keys, seconds);
        m.step(0.02);
        m.skip();
        assert!(m.at_rest());
        assert_eq!((m.position, m.rotation, m.fov), ([10.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], 80.0));
    }

    /// A frame keeps most of where the camera was: after one frame of a 30th the camera has gone a tenth of the way
    /// to where the path has it.
    #[test]
    fn a_frame_keeps_most_of_the_old_camera() {
        let mut m = Mover::default();
        let a = Place { id: vec![], position: [0.0; 3], rotation: [0.0, 0.0, 0.0, 1.0], fov_degrees: 60.0 };
        let b = Place { position: [100.0, 0.0, 0.0], ..a.clone() };
        m.play(vec![a, b], 0.01);
        m.step(1.0 / 30.0);
        // (the path is at its end, 100; kept 0.9 + (0.035 - 0.0333) of the start)
        assert!((m.position[0] - 100.0 * (1.0 - (0.9 + 0.035 - 1.0 / 30.0))).abs() < 1e-3, "{:?}", m.position);
        assert!(!m.at_rest());
    }

    #[test]
    fn the_shortest_turn() {
        let q = slerp([0.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 0.0], 0.5);
        assert!((q[1] - 0.5f32.sqrt()).abs() < 1e-5 && (q[3] - 0.5f32.sqrt()).abs() < 1e-5);
        // (the same turn written the other way round is the same turn)
        let p = slerp([0.0, 0.0, 0.0, 1.0], [0.0, -1.0, 0.0, 0.0], 0.5);
        assert!((p[1].abs() - 0.5f32.sqrt()).abs() < 1e-5);
        assert_eq!(slerp([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0], 0.3), [0.0, 0.0, 0.0, 1.0]);
    }

    /// The game's own file: every transition's cameras are in it, and the cameras the scenes ask for by number.
    #[test]
    fn the_games_file_has_what_the_scenes_ask_for() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let text = std::fs::read_to_string(std::path::Path::new(&game).join("CharCreateCamera.dat")).unwrap();
        let f = CameraFile::parse(&text);
        assert_eq!((f.places.len(), f.transitions.len()), (53, 23));
        for t in &f.transitions {
            assert!(f.keys_of(&t.id).is_some(), "{:?}", t.id);
        }
        for id in [&[1, 1][..], &[2, 1], &[2, 2], &[3, 1], &[3, 3, 1], &[3, 3, 2], &[4, 1], &[3, 2, 1], &[3, 2, 4]] {
            assert!(f.place(id).is_some(), "{id:?}");
        }
        for profession in 1..=14u32 {
            assert!(f.place(&[3, 1, profession]).is_some());
        }
        assert_eq!((f.transition(&[5, 1]).unwrap().seconds, f.transition(&[1, 1, 1]).unwrap().seconds, f.transition(&[4, 3]).unwrap().seconds), (23.0, 25.0, 10.8));
    }

    /// The scenes follow one another as the original has them, and what shows follows the scene.
    #[test]
    fn the_scenes_in_order() {
        let mut d = Director::new(file());
        assert_eq!(d.phase, Phase::Intro);
        assert_eq!(d.sounds, vec!["SM_Sandy_CC_Opening_Music"]);
        // (no transition 5 1 in the little file: the camera is at rest, the way down starts at once, and with no
        // 1 1 1 either the breed scene starts)
        d.frame(0.02);
        assert_eq!(d.phase, Phase::Entering(Scene::Breed));
        d.frame(0.02);
        assert_eq!((d.phase, d.scene(), d.figures, d.professions, d.connector), (Phase::In(Scene::Breed), Some(Scene::Breed), true, false, "character_0"));
        assert!(d.ambience && d.sounds.contains(&"SM_Sandy_CC_Select_Breed"));
        d.go(Order::BreedNext);
        assert_eq!((d.phase, d.scene()), (Phase::Entering(Scene::Appearance), None));
        // (halfway there the character changes its place and the figures go)
        for _ in 0..600 {
            d.frame(0.02);
        }
        assert_eq!((d.phase, d.figures, d.connector), (Phase::In(Scene::Appearance), false, "character_1"));
        d.go(Order::AppearanceNext);
        assert!(d.professions);
        d.mover.skip();
        d.frame(0.02);
        assert_eq!(d.phase, Phase::In(Scene::Profession));
        d.go(Order::ProfessionNext);
        d.mover.skip();
        d.frame(0.02);
        assert_eq!((d.phase, d.professions, d.connector), (Phase::In(Scene::Name), false, "character_3"));
        d.go(Order::NameBack);
        assert_eq!(d.phase, Phase::Entering(Scene::Profession));
        d.go(Order::Made);
        assert_eq!(d.phase, Phase::Leaving);
    }
}
