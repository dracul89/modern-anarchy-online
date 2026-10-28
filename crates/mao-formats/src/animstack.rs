//! A character's running animations, as `VisualCATMesh_t` keeps and blends them (DisplaySystem: `dropin/displaysystem/src/
//! catloader.rs` `SetAnimation` FUN_100736xx and `apply_animation` FUN_10074180, `visualcatmesh.rs` `SetTime` / `set_range`
//! FUN_10072cc7 and `anim_time`, `catrun.rs` `advance` FUN_10074228, `run_anim` FUN_10072fde, `blend_weight` FUN_10072562) over
//! randy31's `CATKeyframeAnim_t` / `CATAnimBlend_t` (`dropin/randy31/src/anim.rs`): several animations play at once, each on the
//! body layers left to it.
//!
//! # The rules
//! - An animation is set with an id, a priority, a loop count (-1 forever), the layers it claims (bits 1 and 2: the player
//!   animations tag each bone track 1 or 2, the root both; Gamecode's movement claims 2, a social or attack 0, `wield` 1), a
//!   direction (0 forward, 1 backward, 2 to and fro, 3 once) and a speed. The list is kept by priority, highest first, a new
//!   one after the others of its priority. Setting a looping animation already playing at that priority (same id and
//!   direction) only returns it, its remaining time topped up to at least 200 ms.
//! - A new animation fades in from 0 over what was left of the cycle of the last animation at its priority, kept within 200
//!   and 300 ms (or 200 ms with none). A counted one fades out again over the same time before its last lap ends.
//! - Each animation plays every track of its data except those on layers claimed by animations of higher priority
//!   (`apply_animation`: the lower one's keyframe layers become `!above & 3`, so a claim of 3 above it silences it).
//! - Each frame (`advance`): a finished counted animation is dropped; a looping one that has fully faded in drops the others
//!   before it at its priority; then the blends are chained from the top: each animation's blend takes the chain so far as its
//!   first and its own keyframes as its second, so on a track its keyframes play (enabled) the track is its own, faded by its
//!   weight over the chain; a track its keyframes do not play is the chain's. A fully faded-in animation whose keyframes play
//!   all layers cuts the chain above it. The last blend is what the mesh poses from: so a claim wins its layers from below,
//!   and on unclaimed layers the latest animation wins.
//! - Time runs in milliseconds at the speed (a delay used up first); the animation's time within its range follows its
//!   direction and lap, stopping just short of the counted end (`anim_time`).

use crate::catanim::CatAnim;
use crate::catmesh::{BoneMatrix, CatMesh};

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: u32,
    pub handle: u32,
    pub priority: i32,
    /// The layers claimed from lower priorities (`Animation` +0x10).
    pub claims: u32,
    pub direction: u32,
    pub speed: f32,
    /// -1 forever.
    pub loops: i32,
    /// The range played, in ms (`SetTime`).
    pub start: f32,
    pub end: f32,
    pub delay: f32,
    /// Ms run, at the speed.
    pub elapsed: f32,
    /// The fade: from `fade_start` for `fade_len` ms, `fade_from` to `fade_to` (the speed given: 1).
    pub fade_start: f32,
    pub fade_len: f32,
    pub fade_from: f32,
    pub fade_to: f32,
    pub weight: f32,
    /// The layers its keyframes play (3 less what is claimed above).
    pub plays: u32,
}

impl Entry {
    /// FUN_1007219c: the laps run.
    fn laps(&self) -> i32 {
        if self.end == self.start {
            return 0;
        }
        (self.elapsed / (self.end - self.start)).abs() as i32
    }

    /// `anim_time`: the animation's time for its elapsed time.
    pub fn time(&self) -> f32 {
        if self.start == self.end {
            return self.start;
        }
        let len = (self.end - self.start).abs();
        if len <= 0.0 {
            return 0.0;
        }
        let mut t = self.elapsed;
        let looped = if self.loops > 0 {
            Some(self.loops as f32 * len)
        } else if self.direction == 3 {
            Some(self.end - self.start)
        } else {
            None
        };
        if let Some(l) = looped {
            if l <= t {
                t = l - 0.001;
            }
        }
        let f = t % len;
        let mut mode = self.direction;
        if mode == 2 && ((t / len) as i32) & 1 != 0 {
            mode = 1;
        }
        if mode == 1 {
            if self.start < self.end { self.end - f } else { f + self.end }
        } else if self.start < self.end {
            self.start + f
        } else {
            self.start - f
        }
    }

    /// `set_range`: the range changed, the elapsed time re-based so the same point is kept.
    fn set_range(&mut self, p1: f32, p2: f32) {
        if self.end == p2 && self.start == p1 {
            return;
        }
        let d = p2 - p1;
        let mut cur = self.time();
        if d > 0.0 && cur > 0.0 {
            if cur > d {
                cur = d;
            }
            let laps = (self.elapsed / d) as i32;
            self.elapsed = d * (laps + 1) as f32 + cur;
        }
        self.start = p1;
        self.end = p2;
    }

    /// `step_time`: the delay used up first.
    fn step_time(&mut self, mut step: f32) {
        if self.delay > 0.0 {
            let used = self.delay.min(step);
            self.delay -= used;
            step -= used;
        }
        if step > 0.0 {
            self.elapsed += step;
        }
    }

    /// `blend_weight`.
    fn blend_weight(&mut self) {
        let t = self.elapsed;
        if self.fade_len > 0.0 {
            let end = self.fade_start + self.fade_len;
            if t > end && self.loops > 0 {
                self.fade_len = -self.fade_len;
                self.fade_start = (self.end - self.start).abs() * self.loops as f32;
            }
        }
        if self.fade_len == 0.0 {
            self.weight = self.fade_to;
        } else {
            let v = (t - self.fade_start) * (self.fade_to - self.fade_from) / self.fade_len + self.fade_from;
            self.weight = v.clamp(self.fade_from.min(self.fade_to), self.fade_to.max(self.fade_from));
            if self.fade_from > v {
                self.weight = self.fade_from;
            } else if self.fade_to < v {
                self.weight = self.fade_to;
            }
        }
    }

    /// `run_anim`: false when a counted animation is done.
    fn run(&mut self, dt_seconds: f32) -> bool {
        if self.loops >= 0 && self.laps() >= self.loops {
            return false;
        }
        let step = dt_seconds * 1000.0 * self.speed;
        self.step_time(step.abs());
        self.blend_weight();
        true
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stack {
    /// By priority, highest first; within a priority in the order set.
    pub entries: Vec<Entry>,
    handles: u32,
}

impl Stack {
    /// `SetAnimation(id, priority, loops, layer, direction, speed)` with the animation's total time (its range to start
    /// with): the handle of the animation playing, or the new one's.
    pub fn set_animation(&mut self, id: u32, priority: i32, loops: i32, claims: u32, direction: u32, speed: f32, total: f32) -> u32 {
        let mut left = 0.0f32;
        let mut same = None;
        let mut place = self.entries.len();
        for (k, e) in self.entries.iter().enumerate() {
            if e.priority < priority {
                place = k;
                break;
            }
            if e.priority == priority {
                let len = (e.end - e.start).abs();
                if len > 0.0 {
                    left = len - e.elapsed % len;
                }
                if loops < 0 && e.loops < 0 {
                    same = if id == e.id && direction == e.direction { Some(e.handle) } else { None };
                    if left < 200.0 {
                        left = if len > 0.0 { (200.0 / len).ceil() * len + left } else { 200.0 };
                    }
                }
            }
            place = k + 1;
        }
        if let Some(h) = same {
            return h;
        }
        self.handles = self.handles.wrapping_add(1);
        let handle = self.handles;
        let entry = Entry {
            id,
            handle,
            priority,
            claims,
            direction,
            speed: 1.0,
            loops,
            start: 0.0,
            end: total,
            delay: 0.0,
            elapsed: 0.0,
            fade_start: 0.0,
            fade_len: left.clamp(200.0, 300.0),
            fade_from: 0.0,
            fade_to: speed,
            weight: 0.0,
            plays: 3,
        };
        self.entries.insert(place, entry);
        self.mask();
        handle
    }

    fn find(&mut self, handle: u32) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.handle == handle)
    }

    /// `SetTime(handle, start, end)`.
    pub fn set_time(&mut self, handle: u32, start: f32, end: f32) {
        if let Some(e) = self.find(handle) {
            e.set_range(start, end);
        }
    }

    /// `SetSpeedScale(handle, speed)`.
    pub fn set_speed_scale(&mut self, handle: u32, speed: f32) {
        if let Some(e) = self.find(handle) {
            e.speed = speed;
        }
    }

    /// `SetLoopCount(handle, n)`.
    pub fn set_loop_count(&mut self, handle: u32, loops: i32) {
        if let Some(e) = self.find(handle) {
            e.loops = loops;
        }
    }

    /// `SetAnimationDelay(handle, ms)`.
    pub fn set_delay(&mut self, handle: u32, delay: f32) {
        if let Some(e) = self.find(handle) {
            e.delay = delay;
        }
    }

    pub fn is_playing(&self, handle: u32) -> bool {
        self.entries.iter().any(|e| e.handle == handle)
    }

    /// `apply_animation`'s masking: each animation's keyframes kept off the layers claimed above its priority.
    fn mask(&mut self) {
        let mut priority = i32::MAX;
        let (mut above, mut layers) = (0u32, 0u32);
        for e in &mut self.entries {
            if priority != e.priority {
                priority = e.priority;
                above = layers;
            }
            e.plays = if above & 3 != 0 { !(above & 3) & 3 } else { 3 };
            layers |= e.claims;
        }
    }

    /// `advance(dt)`: the animations run on by `dt` seconds, the finished and the superseded dropped.
    pub fn advance(&mut self, dt: f32) {
        let mut k = 0;
        let mut priority = i32::MAX;
        let mut first = 0usize;
        while k < self.entries.len() {
            if priority != self.entries[k].priority {
                priority = self.entries[k].priority;
                first = k;
            }
            let alive = dt <= 0.0 || self.entries[k].run(dt);
            if alive {
                let e = &self.entries[k];
                if e.loops < 0 && e.weight >= 1.0 && first != k {
                    // FUN_10074162: the rest of the priority before it dropped.
                    self.entries.drain(first..k);
                    k = first;
                }
                k += 1;
            } else {
                self.entries.remove(k);
            }
        }
        self.mask();
    }

    /// The pose of `mesh` from the chain of blends, `anim` giving each entry's animation by id.
    pub fn pose<'a>(&self, mesh: &CatMesh, anim: &dyn Fn(u32) -> Option<&'a CatAnim>) -> Vec<BoneMatrix> {
        let playing: Vec<(&Entry, &CatAnim, f32)> = self.entries.iter().filter_map(|e| anim(e.id).map(|a| (e, a, e.time()))).collect();
        mesh.pose_with(&|track| {
            let mut chain: Option<([f32; 4], [f32; 3])> = None;
            for (e, a, t) in &playing {
                let enabled = a.layers.get(track).is_some_and(|&l| l & e.plays != 0);
                let own = enabled.then(|| (a.rotation(track, *t), a.translation(track, *t)));
                // (a fully faded-in animation of all layers cuts the chain above it)
                let cut = e.weight >= 1.0 && e.plays >= 3;
                chain = match (if cut { None } else { chain }, own) {
                    (Some(first), Some(second)) => Some(mix(first, second, e.weight)),
                    (None, Some(second)) => Some(second),
                    (first, None) => first,
                };
            }
            chain.unwrap_or(([0.0, 0.0, 0.0, 1.0], [0.0; 3]))
        })
    }
}

/// A blend's track in mode 3: the rotations slerped and the translations lerped by the weight.
fn mix(a: ([f32; 4], [f32; 3]), b: ([f32; 4], [f32; 3]), weight: f32) -> ([f32; 4], [f32; 3]) {
    let (ra, rb) = (a.0, b.0);
    let dot: f32 = (0..4).map(|i| ra[i] * rb[i]).sum();
    let sign = if dot < 0.0 { -1.0 } else { 1.0 };
    let mut r = [0.0f32; 4];
    for i in 0..4 {
        r[i] = ra[i] * (1.0 - weight) + rb[i] * sign * weight;
    }
    let length = r.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-6);
    (r.map(|v| v / length), [0, 1, 2].map(|i| a.1[i] * (1.0 - weight) + b.1[i] * weight))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_death_holds_its_last_frame() {
        // (direction 3 with no loop count: played once, then kept at the end, as a corpse lies)
        let mut s = Stack::default();
        let h = s.set_animation(7, 0, -1, 2, 3, 1.0, 1200.0);
        s.set_time(h, 0.0, 1200.0);
        for _ in 0..50 {
            s.advance(0.1);
        }
        let e = s.entries.iter().find(|e| e.handle == h).unwrap();
        assert!(e.time() > 1199.0 && e.time() <= 1200.0);
    }

    #[test]
    fn a_draw_plays_before_the_idle_set_to_wait_for_it() {
        // (FUN_1003cce2: the weapon's start once, then its idle delayed by the start's length less 300 ms)
        let mut s = Stack::default();
        let start = s.set_animation(1, 0, 1, 2, 0, 1.0, 1000.0);
        s.set_time(start, 0.0, 1000.0);
        let idle = s.set_animation(2, 0, -1, 2, 0, 1.0, 2000.0);
        s.set_delay(idle, 700.0);
        s.advance(0.5);
        let weight = |s: &Stack, h: u32| s.entries.iter().find(|e| e.handle == h).map(|e| e.weight);
        assert!(weight(&s, start).unwrap() > 0.9);
        assert_eq!(weight(&s, idle), Some(0.0));
        // (once the idle has faded in the start is dropped)
        for _ in 0..20 {
            s.advance(0.1);
        }
        assert!(!s.is_playing(start));
        assert_eq!(weight(&s, idle), Some(1.0));
    }

    #[test]
    fn a_new_animation_fades_in_and_a_looping_one_drops_the_old() {
        let mut s = Stack::default();
        let idle = s.set_animation(1, 0, -1, 2, 0, 1.0, 1000.0);
        s.advance(1.0);
        assert_eq!(s.entries.len(), 1);
        assert_eq!(s.entries[0].weight, 1.0);
        // the same again: the same handle
        assert_eq!(s.set_animation(1, 0, -1, 2, 0, 1.0, 1000.0), idle);
        // a walk: after the idle, fading in over 200-300 ms
        let walk = s.set_animation(2, 0, -1, 2, 0, 1.0, 2000.0);
        assert_ne!(walk, idle);
        assert_eq!(s.entries.iter().map(|e| e.id).collect::<Vec<_>>(), vec![1, 2]);
        s.advance(0.1);
        assert!(s.entries[1].weight > 0.0 && s.entries[1].weight < 1.0, "{}", s.entries[1].weight);
        s.advance(0.4);
        // fully in: the idle is dropped
        assert_eq!(s.entries.iter().map(|e| e.id).collect::<Vec<_>>(), vec![2]);
    }

    #[test]
    fn a_counted_animation_fades_out_and_goes() {
        let mut s = Stack::default();
        s.set_animation(1, 0, -1, 2, 0, 1.0, 1000.0);
        s.advance(1.0);
        let wave = s.set_animation(5, 0, 1, 0, 0, 1.0, 1000.0);
        s.advance(0.3);
        assert!(s.is_playing(wave));
        assert!((s.entries[1].weight - 1.0).abs() < 1e-5);
        s.advance(0.6);
        // in its last 200 ms: fading out
        assert!(s.entries[1].weight < 1.0 && s.entries[1].weight > 0.0, "{}", s.entries[1].weight);
        s.advance(0.2);
        // (its laps are counted before it steps: gone the frame after its last)
        s.advance(0.001);
        assert!(!s.is_playing(wave));
        assert_eq!(s.entries.len(), 1);
    }

    #[test]
    fn a_lower_priority_keeps_only_the_layers_left_to_it() {
        let mut s = Stack::default();
        s.set_animation(1, 0, -1, 2, 0, 1.0, 1000.0);
        s.set_animation(7, -2, 1, 0, 0, 1.0, 1000.0);
        assert_eq!(s.entries.iter().map(|e| (e.id, e.plays)).collect::<Vec<_>>(), vec![(1, 3), (7, 1)]);
        // a claim of everything above silences the lower one
        let mut s = Stack::default();
        s.set_animation(1, 0, -1, 3, 0, 1.0, 1000.0);
        s.set_animation(7, -2, 1, 0, 0, 1.0, 1000.0);
        assert_eq!(s.entries[1].plays, 0);
    }

    #[test]
    fn the_time_follows_the_range_and_direction() {
        let mut e = Entry { id: 0, handle: 0, priority: 0, claims: 0, direction: 0, speed: 1.0, loops: -1, start: 100.0, end: 400.0, delay: 0.0, elapsed: 350.0, fade_start: 0.0, fade_len: 0.0, fade_from: 0.0, fade_to: 1.0, weight: 1.0, plays: 3 };
        assert_eq!(e.time(), 150.0);
        e.direction = 1;
        assert_eq!(e.time(), 350.0);
        e.direction = 2;
        assert_eq!(e.time(), 350.0);
        e.elapsed = 50.0;
        assert_eq!(e.time(), 150.0);
        // a counted one stops just short of its end
        e.direction = 0;
        e.loops = 1;
        e.elapsed = 900.0;
        assert!((e.time() - 399.999).abs() < 0.01);
        // the range changed keeps the point
        e.loops = -1;
        e.elapsed = 350.0;
        e.set_range(0.0, 600.0);
        assert!((e.time() - 150.0).abs() < 1e-3, "{}", e.time());
    }
}
