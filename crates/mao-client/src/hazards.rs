//! The hazards' sounds: mines (`Mine_t`, dynel type 0xc741), traps (`TrapItem_t`, 0xc740) and central controllers
//! (`CentralController_t`, 0xc742), each a `SimpleItem_t` the client makes from its type (the factories Gamecode registers at
//! 0x101521d6, 0x1015233e and 0x10152122).
//!
//! * Their places come from their full updates (`MineFullUpdateIIR_t` 0x215b5678, `TrapItemFullUpdateIIR_t` 0x59313928,
//!   `CentralControllerFullUpdateIIR_t` 0x15253307), each read as a `SimpleItemFullUpdateIIR_t` first (FUN_100a1661, then
//!   FUN_100a28ca, FUN_100a0da4 and FUN_1009f466 read their own fields).
//! * What befalls a mine or trap comes as an `ActionIIR_t` (0x2049527c, FUN_1009f042 reads it: a version, the action and
//!   whose; FUN_1009f098 hands it to the item's slot 34): 0x6d sprung, 0x6c disarmed, 0x8c a mine's malfunction
//!   (FUN_10085b15, FUN_10099536).
//! * A central controller plays its state's sound every frame (FUN_1007e0a8): idle, or shutdown once; its state comes in its
//!   full update and in `CentralControllerStateIIR_t` (0x08536f65, FUN_1009f624; FUN_1009f5e2 → FUN_1007e206).
//!
//! * A mine bleeps when the client's character walks into it ([`Hazards::bleeps`]): FUN_10089f0a, the item's frame, reads the
//!   collisions N3 found (`RunDynelDynelCollChecking`: only the client's character is checked, `n3Dynel_t::CheckBodyCollision`
//!   against every other dynel within 10 m on x and z, sphere against sphere, `CollPrim_t::CheckSphSph`) and hands each to the
//!   mine's slot 55 (FUN_10085faa), which plays `SM_Sandy_Hazards_mine_bleep` on the frames the game clock's second turns.
//!
//! * Sprung, the item's effect 0x834 is made at it (`_EffectHandler_t::CreateEffect2`, the handler's +0x54 set to 0x20 first).
//!
//! Not here: the signal FUN_10085faa sends after the bleep (`GlobalSignals_c` +0x14c with 3.0, FUN_10011a4b).

use std::collections::BTreeMap;

use mao_formats::sound::sound_id;

pub const MINE_FULL_UPDATE: u32 = 0x215b5678;
pub const TRAP_FULL_UPDATE: u32 = 0x59313928;
pub const CONTROLLER_FULL_UPDATE: u32 = 0x15253307;
pub const CONTROLLER_STATE: u32 = 0x08536f65;
pub const ACTION: u32 = 0x2049527c;
/// `SimpleItemFullUpdateIIR_t`, whose reader the hazards' full updates start with.
const SIMPLE_ITEM_FULL_UPDATE: u32 = 0x3b11256f;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Mine,
    Trap,
    /// A central controller and its state (+0x1d4: 0 idle, 1 shut down, 2 none).
    Controller(u8),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hazard {
    pub kind: Kind,
    pub at: [f32; 3],
    /// A controller's shutdown sound has played (FUN_1007e0a8 clears its +0x12c after it).
    shut: bool,
    /// The item's stats from its full update (the `StatHolder_t`, FUN_1002e62b: 1234567890 for one not given).
    pub stats: BTreeMap<u32, i64>,
    /// A trap's or mine's "armed" flag (+0x1e4: bit 1 of the full update's flags byte, FUN_100a28ca; FUN_100a2a42 →
    /// FUN_10085de7 → FUN_1009979d).
    armed: bool,
    /// A mine has malfunctioned (+0x1f8, set by the action 0x8c).
    malfunctioned: bool,
    /// The item frame's count (+0x114 of the visual dynel: one more every other second of the game clock).
    count: i64,
}

impl Hazard {
    fn new(kind: Kind, at: [f32; 3]) -> Hazard {
        Hazard { kind, at, shut: false, stats: BTreeMap::new(), armed: false, malfunctioned: false, count: 0 }
    }

    /// A stat (`GetStat`): the given value, else the holder's 1234567890; a mine's 36 is 1 until told (FUN_10085ec5's
    /// `SetStat(0x24, 1)`).
    pub fn stat(&self, stat: u32) -> i64 {
        match self.stats.get(&stat) {
            Some(&v) => v,
            None if stat == 36 && self.kind == Kind::Mine => 1,
            None => 1234567890,
        }
    }
}

/// FUN_1006aa89 (DisplaySystem): a byte of a mesh's packed sphere word (signed) as `0.05x + 6e-6x³`.
pub fn curve(byte: u8) -> f32 {
    let x = byte as i8 as f64;
    (x * 0.05000000074505806 + 6.000000212225132e-06 * x * x * x) as f32
}

/// A mesh's torso sphere from its packed word (`RDBMesh_t::ReadBlob`, `RDBCATMesh_t`'s FUN_1006aab4): the radius from the top
/// byte, the centre from the low three; as `n3VisualDynel_t::UpdateCollision` (N3 0x10019be4) makes the body's sphere of it:
/// a radius below 0 taken as 0.5, both times the body's scale, the centre straight above the dynel at its height times the
/// scale. Returns (height of the centre, radius).
pub fn body_sphere(packed: u32, scale: f32) -> (f32, f32) {
    let [_, b1, _, b3] = packed.to_le_bytes();
    let mut radius = curve(b3);
    if radius < 0.0 {
        radius = 0.5;
    }
    (curve(b1) * scale, scale * radius)
}

/// `GameTime_t::RunFunction`'s turns of a second (Gamecode 0x1000b27e): the real time (+0x28) runs on by the frame's; once it
/// reaches the last turn (+0x30) plus 1 the second turns (+0x18), the next turn kept no later than a second ago, and every
/// other turn (+0x1a, by the parity +0xb0) the items' frames count on.
#[derive(Default)]
pub struct SecondTicks {
    last: Option<std::time::Instant>,
    real: f64,
    turn: f64,
    parity: u32,
}

impl SecondTicks {
    /// A frame: (the second turned, the other second turned).
    pub fn step(&mut self) -> (bool, bool) {
        let now = std::time::Instant::now();
        let dt = self.last.map_or(0.0, |l| now.duration_since(l).as_secs_f32());
        self.last = Some(now);
        self.real += dt as f64;
        if self.real < self.turn + 1.0 {
            return (false, false);
        }
        self.parity = self.parity.wrapping_sub(1) & 1;
        let left = 1.0 - (self.real - self.turn);
        let keep = if left <= 1.0 { left.max(0.0) } else { 1.0 };
        self.turn = self.real - keep;
        (true, self.parity != 0)
    }
}

/// What a frame of the mines needs: the game clock's turns (`GameTime_t` +0x18 a second, +0x1a every other second), the
/// client's character's place and body sphere (height, radius), and each mine's body sphere by its stats.
pub struct Frame<'a> {
    pub second: bool,
    pub other_second: bool,
    pub own: [f32; 3],
    pub own_sphere: (f32, f32),
    pub sphere: &'a dyn Fn(&Hazard) -> Option<(f32, f32)>,
}

/// What an action did: the sounds to play (id, where), the effects to make (template, where) and the feedback line's name.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    pub sounds: Vec<(u32, [f32; 3])>,
    pub effects: Vec<(i32, [f32; 3])>,
    pub line: Option<&'static str>,
}

#[derive(Default)]
pub struct Hazards {
    pub map: BTreeMap<u32, Hazard>,
}

impl Hazards {
    /// A full update of a mine, trap or controller (`kind`, the message's) for the dynel `instance`. Its place is the
    /// `SimpleItemFullUpdate`'s (only when it has one: owner type 0); a controller's two bytes follow (FUN_1009f466: a byte
    /// below 4, +0x1d0, then the state, below 3).
    pub fn full_update(&mut self, kind: u32, instance: u32, data: &[u8]) {
        let Ok(p) = mao_net::schema::Schema::global().parse_n3(SIMPLE_ITEM_FULL_UPDATE, data) else { return };
        let Some(pos) = p.value.get("Position") else { return };
        let f = |n: &str| match pos.get(n) {
            Some(mao_net::schema::Value::Float(x)) => *x as f32,
            _ => 0.0,
        };
        let at = [f("X"), f("Y"), f("Z")];
        let kind = match kind {
            MINE_FULL_UPDATE => Kind::Mine,
            TRAP_FULL_UPDATE => Kind::Trap,
            CONTROLLER_FULL_UPDATE => {
                let rest = &data[data.len() - p.left..];
                match rest {
                    [a, state, ..] if *a < 4 && *state < 3 => Kind::Controller(*state),
                    _ => return,
                }
            }
            _ => return,
        };
        let mut h = Hazard::new(kind, at);
        if let Some(old) = self.map.get(&instance) {
            (h.shut, h.malfunctioned, h.count) = (old.shut, old.malfunctioned, old.count);
        }
        if let Some(mao_net::schema::Value::List(stats)) = p.value.get("Stats") {
            for s in stats {
                if let (Some(k), Some(v)) = (s.get("Item1").and_then(mao_net::schema::Value::as_int), s.get("Item2").and_then(mao_net::schema::Value::as_int)) {
                    h.stats.insert(k as u32, v);
                }
            }
        }
        // (a trap's fields, FUN_100a28ca: its version 2, a word, two identities, the flags byte)
        if matches!(kind, Kind::Mine | Kind::Trap) {
            let rest = &data[data.len() - p.left..];
            if rest.len() >= 25 && u32::from_be_bytes(rest[0..4].try_into().unwrap()) == 2 {
                h.armed = rest[24] >> 1 & 1 != 0;
            }
        }
        self.map.insert(instance, h);
    }

    /// A controller's new state (`CentralControllerStateIIR_t`: after the header byte, the state, below 3).
    pub fn state(&mut self, instance: u32, data: &[u8]) {
        if let (Some(Hazard { kind: Kind::Controller(s), .. }), Some(&state)) = (self.map.get_mut(&instance), data.get(1)) {
            if state < 3 {
                *s = state;
            }
        }
    }

    /// An `ActionIIR_t` to the dynel `instance` (its data: the header byte, the version 1, the action, whose identity), the
    /// client's character being `own`. Sprung (0x6d): `SM_Sandy_Game_Explo_Big` at the item, and to the one it caught, if the
    /// client's, "Feedback_MineWasSprung" or "Feedback_ATrapWasSprung"; disarmed (0x6c): to a character, "you succeeded"; a
    /// mine's malfunction (0x8c): `SM_Sandy_Hazards_mine_malfunction` at it.
    pub fn action(&mut self, instance: u32, data: &[u8], own: u32) -> Outcome {
        let mut out = Outcome::default();
        let Some(h) = self.map.get(&instance) else { return out };
        if data.len() < 17 || u32::from_be_bytes(data[1..5].try_into().unwrap()) != 1 {
            return out;
        }
        let action = u32::from_be_bytes(data[5..9].try_into().unwrap());
        let (who_type, who) = (u32::from_be_bytes(data[9..13].try_into().unwrap()), u32::from_be_bytes(data[13..17].try_into().unwrap()));
        let character = who_type == 0xc350;
        let mine = h.kind == Kind::Mine;
        match (action, h.kind) {
            (0x6d, Kind::Mine | Kind::Trap) => {
                out.sounds.push((sound_id("SM_Sandy_Game_Explo_Big"), h.at));
                out.effects.push((0x834, h.at));
                if character && who == own {
                    out.line = Some(if mine { "Feedback_MineWasSprung" } else { "Feedback_ATrapWasSprung" });
                }
            }
            (0x6c, Kind::Mine | Kind::Trap) if character => {
                out.line = Some(if mine { "Feedback_YouSuccededDisarmingMine" } else { "Feedback_YouSuccededDisarmingTrap" });
            }
            (0x8c, Kind::Mine) => {
                out.sounds.push((sound_id("SM_Sandy_Hazards_mine_malfunction"), h.at));
                self.map.get_mut(&instance).unwrap().malfunctioned = true;
            }
            _ => {}
        }
        out
    }

    /// The mines' bleeps this frame: (sound, where, pitch). Each mine's frame (FUN_10089f0a) counts every other second, and
    /// on the frames its count is a multiple of its stat 437 reads its collisions: the client's character's sphere against
    /// the mine's (when the mine wants to be told, its flags stat 0 bit 0x80000, `SetWantCollinfo`; within 10 m on x and z;
    /// `dist² < (r1 + r2)²`). FUN_10085faa then bleeps when the mine's stat 36 is set, it has not malfunctioned, it is armed
    /// and the game clock's second has just turned, at the mine, at the pitch `(3 − d) × 10 + 100` (truncated) of the
    /// distance `d` between the two.
    pub fn bleeps(&mut self, f: &Frame) -> Vec<(u32, [f32; 3], u32)> {
        let mut out = Vec::new();
        for h in self.map.values_mut() {
            if h.kind != Kind::Mine {
                continue;
            }
            if f.other_second {
                h.count += 1;
            }
            let every = h.stat(437);
            if every == 0 || h.count % every != 0 {
                continue;
            }
            if h.stat(0) & 0x80000 == 0 {
                continue;
            }
            let (dx, dz) = (h.at[0] - f.own[0], h.at[2] - f.own[2]);
            if !(-10.0..=10.0).contains(&dx) || !(-10.0..=10.0).contains(&dz) {
                continue;
            }
            let Some((height, radius)) = (f.sphere)(h) else { continue };
            let dy = (h.at[1] + height) - (f.own[1] + f.own_sphere.0);
            let (d2, r) = (dx * dx + dy * dy + dz * dz, radius + f.own_sphere.1);
            if !(d2 < r * r) {
                continue;
            }
            if h.stat(36) == 0 || h.malfunctioned || !h.armed || !f.second {
                continue;
            }
            let d = (dx * dx + (h.at[1] - f.own[1]).powi(2) + dz * dz).sqrt();
            let pitch = ((3.0 - d as f64) * 10.0 + 100.0) as i32 as u32;
            out.push((sound_id("SM_Sandy_Hazards_mine_bleep"), h.at, pitch));
        }
        out
    }

    /// The controllers' sounds this frame (FUN_1007e0a8: each its state's, `SM_Sandy_Hazards_central_controller_idle` or,
    /// once, `..._shutdown`, at it).
    pub fn frame(&mut self) -> Vec<(u32, [f32; 3])> {
        let mut out = Vec::new();
        for h in self.map.values_mut() {
            match h.kind {
                Kind::Controller(0) => out.push((sound_id("SM_Sandy_Hazards_central_controller_idle"), h.at)),
                Kind::Controller(1) if !h.shut => {
                    h.shut = true;
                    out.push((sound_id("SM_Sandy_Hazards_central_controller_shutdown"), h.at));
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(action: u32, who: (u32, u32)) -> Vec<u8> {
        let mut d = vec![1, 0, 0, 0, 1];
        for w in [action, who.0, who.1] {
            d.extend(w.to_be_bytes());
        }
        d
    }

    #[test]
    fn the_action_message_is_read_as_captured() {
        // (a captured `ActionIIR_t` to a door, 0xc76a: the header, version 1, action 0x66, the character)
        let captured = [0x01, 0, 0, 0, 1, 0, 0, 0, 0x66, 0, 0, 0xc3, 0x50, 0, 0x04, 0xc2, 0xde];
        assert_eq!(captured.to_vec(), action(0x66, (0xc350, 0x4c2de)));
    }

    #[test]
    fn mines_traps_and_controllers() {
        let mut h = Hazards::default();
        h.map.insert(1, Hazard::new(Kind::Mine, [1.0, 2.0, 3.0]));
        h.map.insert(2, Hazard::new(Kind::Trap, [4.0, 5.0, 6.0]));
        let boom = sound_id("SM_Sandy_Game_Explo_Big");
        assert_eq!(h.action(1, &action(0x6d, (0xc350, 7)), 7), Outcome { sounds: vec![(boom, [1.0, 2.0, 3.0])], effects: vec![(0x834, [1.0, 2.0, 3.0])], line: Some("Feedback_MineWasSprung") });
        assert_eq!(h.action(2, &action(0x6d, (0xc350, 8)), 7), Outcome { sounds: vec![(boom, [4.0, 5.0, 6.0])], effects: vec![(0x834, [4.0, 5.0, 6.0])], line: None });
        assert_eq!(h.action(2, &action(0x6c, (0xc350, 7)), 7).line, Some("Feedback_YouSuccededDisarmingTrap"));
        assert_eq!(h.action(1, &action(0x8c, (0, 0)), 7).sounds, vec![(sound_id("SM_Sandy_Hazards_mine_malfunction"), [1.0, 2.0, 3.0])]);
        assert_eq!(h.action(2, &action(0x8c, (0, 0)), 7), Outcome::default());
        assert_eq!(h.action(3, &action(0x6d, (0xc350, 7)), 7), Outcome::default());

        h.map.insert(9, Hazard::new(Kind::Controller(0), [0.0; 3]));
        assert_eq!(h.frame(), vec![(sound_id("SM_Sandy_Hazards_central_controller_idle"), [0.0; 3])]);
        h.state(9, &[1, 1]);
        assert_eq!(h.frame(), vec![(sound_id("SM_Sandy_Hazards_central_controller_shutdown"), [0.0; 3])]);
        assert!(h.frame().is_empty());
        h.state(9, &[1, 2]);
        assert!(h.frame().is_empty());
    }

    #[test]
    fn a_mine_bleeps_when_walked_into() {
        let mut h = Hazards::default();
        let mut mine = Hazard::new(Kind::Mine, [10.0, 0.0, 10.0]);
        mine.stats.extend([(0, 0x80000), (437, 1)]);
        mine.armed = true;
        h.map.insert(1, mine);
        let sphere = |_: &Hazard| Some((0.2, 0.5));
        let frame = |own: [f32; 3], second: bool| Frame { second, other_second: true, own, own_sphere: (1.0, 0.5), sphere: &sphere };
        // (touching: the spheres 0.8 apart, under their 1.0; the pitch of 0.5 m: 125)
        assert_eq!(h.bleeps(&frame([10.0, 0.0, 10.5], true)), vec![(sound_id("SM_Sandy_Hazards_mine_bleep"), [10.0, 0.0, 10.0], 125)]);
        // (not on the second's turn, nor out of reach)
        assert!(h.bleeps(&frame([10.0, 0.0, 10.5], false)).is_empty());
        assert!(h.bleeps(&frame([10.0, 0.0, 12.0], true)).is_empty());
        // (unarmed, malfunctioned, or without stat 437: silent)
        h.action(1, &action(0x8c, (0, 0)), 7);
        assert!(h.bleeps(&frame([10.0, 0.0, 10.5], true)).is_empty());
        let mut quiet = Hazard::new(Kind::Mine, [0.0; 3]);
        quiet.stats.insert(0, 0x80000);
        quiet.armed = true;
        h.map.insert(1, quiet);
        assert!(h.bleeps(&frame([0.0, 0.0, 0.2], true)).is_empty());
        // (the sphere words: radius byte 0x14 = 20 → 1.048, height byte 0x0a = 10 → 0.506)
        let (y, r) = body_sphere(0x1400_0a00, 1.0);
        assert!((y - 0.506).abs() < 1e-6 && (r - 1.048).abs() < 1e-6);
        assert_eq!(body_sphere(0xf000_0000, 2.0).1, 1.0);
    }
}
