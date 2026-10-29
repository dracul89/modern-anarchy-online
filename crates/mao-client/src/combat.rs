//! The fight's own rules on the client's side, from Gamecode: whom the targeting keys pick (`N3Msg_GetCloseTarget`
//! 0x1001c49a, called by GUI.dll's `TargetingModule_t::Get{Next,Prev}{Hostile,Friendly}TargetMessage`) and what the attack
//! key does (`N3Msg_DefaultAttack` 0x10028147 with the selection and `false`, from Interfaces.dll 0x10007cba; its check
//! FUN_10069a9f and the sender FUN_1006817d).

/// The radius the targeting keys look in (the locality query's 100, 0x10156eb0).
pub const TARGET_RADIUS: f32 = 100.0;
/// What a candidate nearer than the current target (or as near and listed before it) is pushed back by, so the order wraps
/// (0x10158870).
const WRAP: f32 = 1000.0;

/// A character the targeting keys can pick: its identity, place and side (stat 33, the full update's packed word).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub id: u32,
    pub position: [f32; 3],
    pub side: u8,
}

/// Whether a side is a friend's: the same as one's own (the original also counts the team, the raid and their pets,
/// FUN_10065d65 / FUN_10065dab / stat 196, which this client does not track).
pub fn friendly(own_side: u8, side: u8) -> bool {
    own_side == side
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// `N3Msg_GetCloseTarget(current, next, friendly)`: among the characters within [`TARGET_RADIUS`] (in the list's order,
/// neither oneself nor the current target) of the wanted kind, the one whose distance is the least (`next`) or the most
/// (not `next`) past the current target's, the nearer ones wrapped round by [`WRAP`]; none when there is none.
pub fn close_target(own: [f32; 3], own_side: u8, current: Option<u32>, list: &[Candidate], next: bool, want_friendly: bool) -> Option<u32> {
    let current_at = current.and_then(|c| list.iter().position(|x| x.id == c));
    let (current_distance, current_index) = current_at.map_or((0.0, 0), |i| (distance(list[i].position, own), i));
    let mut best: Option<(f32, u32)> = None;
    for (i, c) in list.iter().enumerate() {
        if Some(c.id) == current || distance(c.position, own) > TARGET_RADIUS || friendly(own_side, c.side) != want_friendly {
            continue;
        }
        let mut d = distance(c.position, own) - current_distance;
        if d < 0.0 {
            d += WRAP;
        }
        if d == 0.0 && i < current_index {
            d += WRAP;
        }
        let better = match best {
            None => true,
            Some((b, _)) if next => d < b,
            Some((b, _)) => d > b,
        };
        if better {
            best = Some((d, c.id));
        }
    }
    best.map(|b| b.1)
}

/// What the attack key does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attack {
    /// `N3Msg_StopAttack`: the `StopFight` message.
    Stop,
    /// FUN_1006817d: the `Attack` message.
    Start(u32),
    /// Nothing sent; the feedback line (text.mdb's name).
    Refuse(&'static str),
}

/// What the attack key's target is, as FUN_10069a9f looks at it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Nothing,
    Yourself,
    /// A character: whether it is dead.
    Character { dead: bool },
    /// Anything else (an item, a terminal).
    Other,
}

/// `N3Msg_DefaultAttack(selection, false)`: a fighting character stops; else the selection is checked (FUN_10069a9f's
/// first refusals: oneself, the dead, not a character; swimming refuses with its own line) and attacked, unless an attack
/// is still on its way (FUN_1006817d's +0x79).
pub fn default_attack(fighting: bool, swimming: bool, pending: bool, target: Option<u32>, kind: Target) -> Attack {
    if fighting {
        return Attack::Stop;
    }
    if swimming {
        return Attack::Refuse("Feedback_YouCantAttackWhileSwimming");
    }
    let Some(target) = target else { return Attack::Refuse("Feedback_UnableToAttackTarget") };
    match kind {
        Target::Yourself => Attack::Refuse("Feedback_YouCantAttackYourself"),
        Target::Character { dead: true } => Attack::Refuse("Feedback_TargetIsAlreadyDead"),
        Target::Nothing | Target::Other => Attack::Refuse("Feedback_UnableToAttackTarget"),
        Target::Character { dead: false } if pending => Attack::Refuse("Feedback_PleaseWaitUntilPreviousAction"),
        Target::Character { dead: false } => Attack::Start(target),
    }
}

/// FUN_1004e925: the nano cost modifier in percent: stat 318, for a player at least its breed's (Solitus and Opifex 50,
/// Nanomage 45, Atrox 55).
pub fn nano_cost_modifier(stat_318: i64, breed: i64, player: bool) -> i64 {
    let floor = match breed {
        1 | 2 => 50,
        3 => 45,
        4 => 55,
        _ => 0,
    };
    if player { stat_318.max(floor) } else { stat_318 }
}

/// What casting depends on: how the character moves and its nano.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Caster {
    pub dead: bool,
    pub swimming: bool,
    pub falling: bool,
    pub moving: bool,
    /// The current nano (stat 214).
    pub nano: i64,
}

/// FUN_1004fbe4 (`N3Msg_CastNanoSpell`'s check): why a nano program can't be cast, with the feedback line (and its number):
/// swimming, falling, moving (the program's cast needs standing still), or less nano than its cost (stat 407 times the
/// modifier, in percent). Dead casts nothing, silently (the movement states 8, 9, 11 and 12).
pub fn cast_refusal(caster: Caster, cost: i64, modifier: i64) -> Option<(&'static str, Option<i64>)> {
    if caster.dead {
        return Some(("", None));
    }
    if caster.swimming {
        return Some(("Feedback_YouCantExecuteNanoprogramsWhileSwimming", None));
    }
    if caster.falling {
        return Some(("Feedback_YouCantExecuteNanoprogramsWhileFalling", None));
    }
    if caster.moving {
        return Some(("Feedback_UnableToExecuteProgramsMustStandStill", None));
    }
    let needed = (cost as f64 * modifier as f64 / 100.0) as i64;
    (caster.nano < needed).then_some(("Feedback_YouNeedAtLeastRemainingNano", Some(needed)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn casting_needs_standing_and_nano() {
        let still = Caster { nano: 100, ..Default::default() };
        assert_eq!(cast_refusal(still, 80, 100), None);
        assert_eq!(cast_refusal(Caster { moving: true, ..still }, 80, 100), Some(("Feedback_UnableToExecuteProgramsMustStandStill", None)));
        assert_eq!(cast_refusal(Caster { falling: true, ..still }, 80, 100), Some(("Feedback_YouCantExecuteNanoprogramsWhileFalling", None)));
        assert_eq!(cast_refusal(still, 300, 50), Some(("Feedback_YouNeedAtLeastRemainingNano", Some(150))));
        assert_eq!(nano_cost_modifier(30, 3, true), 45);
        assert_eq!(nano_cost_modifier(100, 3, true), 100);
        assert_eq!(nano_cost_modifier(30, 3, false), 30);
    }

    use super::*;

    fn at(id: u32, x: f32, side: u8) -> Candidate {
        Candidate { id, position: [x, 0.0, 0.0], side }
    }

    #[test]
    fn tab_steps_outward_and_wraps() {
        // monsters (side 3) at 5, 10 and 20 m, a guard of one's own side (1) at 7 m, one out of reach at 150 m
        let list = [at(1, 10.0, 3), at(2, 5.0, 3), at(3, 20.0, 3), at(4, 7.0, 1), at(5, 150.0, 3)];
        let own = [0.0; 3];
        // nothing targeted: the nearest hostile
        assert_eq!(close_target(own, 1, None, &list, true, false), Some(2));
        // then farther each time, then round to the nearest
        assert_eq!(close_target(own, 1, Some(2), &list, true, false), Some(1));
        assert_eq!(close_target(own, 1, Some(1), &list, true, false), Some(3));
        assert_eq!(close_target(own, 1, Some(3), &list, true, false), Some(2));
        // Shift+Tab goes back
        assert_eq!(close_target(own, 1, Some(1), &list, false, false), Some(2));
        assert_eq!(close_target(own, 1, Some(2), &list, false, false), Some(3));
        // the friendly keys: the guard only
        assert_eq!(close_target(own, 1, None, &list, true, true), Some(4));
        assert_eq!(close_target(own, 1, Some(4), &list, true, true), None);
    }

    #[test]
    fn the_attack_key_toggles() {
        assert_eq!(default_attack(true, false, false, Some(9), Target::Character { dead: false }), Attack::Stop);
        assert_eq!(default_attack(false, false, false, Some(9), Target::Character { dead: false }), Attack::Start(9));
        assert_eq!(default_attack(false, false, false, Some(9), Target::Character { dead: true }), Attack::Refuse("Feedback_TargetIsAlreadyDead"));
        assert_eq!(default_attack(false, false, false, Some(1), Target::Yourself), Attack::Refuse("Feedback_YouCantAttackYourself"));
        assert_eq!(default_attack(false, false, false, None, Target::Nothing), Attack::Refuse("Feedback_UnableToAttackTarget"));
        assert_eq!(default_attack(false, false, true, Some(9), Target::Character { dead: false }), Attack::Refuse("Feedback_PleaseWaitUntilPreviousAction"));
    }
}
