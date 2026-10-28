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
//! Not here: the sprung item's effect (0x834, `_EffectHandler_t::CreateEffect2`) and a mine's bleep when a player walks into
//! it (FUN_10085faa, from FUN_10089f0a's collisions: it needs the mine's collision volume).

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
}

/// What an action did: the sounds to play (id, where) and the feedback line's name.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    pub sounds: Vec<(u32, [f32; 3])>,
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
        let shut = self.map.get(&instance).is_some_and(|h| h.shut);
        self.map.insert(instance, Hazard { kind, at, shut });
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
                if character && who == own {
                    out.line = Some(if mine { "Feedback_MineWasSprung" } else { "Feedback_ATrapWasSprung" });
                }
            }
            (0x6c, Kind::Mine | Kind::Trap) if character => {
                out.line = Some(if mine { "Feedback_YouSuccededDisarmingMine" } else { "Feedback_YouSuccededDisarmingTrap" });
            }
            (0x8c, Kind::Mine) => out.sounds.push((sound_id("SM_Sandy_Hazards_mine_malfunction"), h.at)),
            _ => {}
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
        h.map.insert(1, Hazard { kind: Kind::Mine, at: [1.0, 2.0, 3.0], shut: false });
        h.map.insert(2, Hazard { kind: Kind::Trap, at: [4.0, 5.0, 6.0], shut: false });
        let boom = sound_id("SM_Sandy_Game_Explo_Big");
        assert_eq!(h.action(1, &action(0x6d, (0xc350, 7)), 7), Outcome { sounds: vec![(boom, [1.0, 2.0, 3.0])], line: Some("Feedback_MineWasSprung") });
        assert_eq!(h.action(2, &action(0x6d, (0xc350, 8)), 7), Outcome { sounds: vec![(boom, [4.0, 5.0, 6.0])], line: None });
        assert_eq!(h.action(2, &action(0x6c, (0xc350, 7)), 7).line, Some("Feedback_YouSuccededDisarmingTrap"));
        assert_eq!(h.action(1, &action(0x8c, (0, 0)), 7).sounds, vec![(sound_id("SM_Sandy_Hazards_mine_malfunction"), [1.0, 2.0, 3.0])]);
        assert_eq!(h.action(2, &action(0x8c, (0, 0)), 7), Outcome::default());
        assert_eq!(h.action(3, &action(0x6d, (0xc350, 7)), 7), Outcome::default());

        h.map.insert(9, Hazard { kind: Kind::Controller(0), at: [0.0; 3], shut: false });
        assert_eq!(h.frame(), vec![(sound_id("SM_Sandy_Hazards_central_controller_idle"), [0.0; 3])]);
        h.state(9, &[1, 1]);
        assert_eq!(h.frame(), vec![(sound_id("SM_Sandy_Hazards_central_controller_shutdown"), [0.0; 3])]);
        assert!(h.frame().is_empty());
        h.state(9, &[1, 2]);
        assert!(h.frame().is_empty());
    }
}
