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
    /// FUN_1006817d: the `Attack` message.
    Start(u32),
    /// Nothing sent; the feedback lines (text.mdb's names), in order.
    Refuse(&'static [&'static str]),
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

/// One's own character as FUN_10069a9f looks at it: whether it flies (movement mode 7), its state flags (stat 224) and
/// its stat 215.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Own {
    pub flying: bool,
    pub flags: i64,
    pub stat_215: i64,
}

/// FUN_10069a9f(target, feedback): whether the selection may be attacked, else the line it says (none when it says
/// nothing). Of its checks those whose facts this client keeps: oneself; one's own state flag 0x80000000 with stat 215
/// set lets any character be attacked; swimming (mode 4); one's own flag 0x400 or flying (mode 7); the dead (the
/// original says nothing for one whose health stat is not 0). Not followed: the target's own flags (0x20000000 "can't be
/// attacked", 0x80000000), surrender (stats 449, 451), the suppression districts, the side and team checks
/// (FUN_10067c06, FUN_10065d65 ...) and items (flag 0x10000000 and use criterion 0xb, else
/// `Feedback_YouCantAttackThisItem`): the client counts every other thing as not attackable, silently.
fn attackable(swimming: bool, kind: Target, own: Own) -> Result<(), Option<&'static str>> {
    match kind {
        Target::Nothing | Target::Other => Err(None),
        Target::Yourself => Err(Some("Feedback_YouCantAttackYourself")),
        Target::Character { .. } if own.flags as u32 & 0x8000_0000 != 0 && own.stat_215 != 0 => Ok(()),
        Target::Character { .. } if swimming => Err(Some("Feedback_YouCantAttackWhileSwimming")),
        Target::Character { .. } if own.flags & 0x400 != 0 || own.flying => Err(Some("Feedback_YoureUnableToAttack")),
        Target::Character { dead: true } => Err(Some("Feedback_TargetIsAlreadyDead")),
        Target::Character { dead: false } => Ok(()),
    }
}

/// `N3Msg_DefaultAttack(selection, switching)` (Gamecode 0x10028147): a fighting character stops (`N3Msg_StopAttack`)
/// first; swimming refuses (`Feedback_CantAttackInThisState`, after the stop); a fight stopped by the attack key ends
/// there, and so does a switch to the one already fought (`fought`); else the selection is checked (FUN_10069a9f, whose
/// own line comes first, then `Feedback_UnableToAttackTarget`) and attacked unless an attack is still on its way
/// (FUN_1006817d's +0x79), with the message's flag 2 when a fight was left for it. Returns whether to stop, what to do
/// next and the flag.
pub fn attack_key(fought: Option<u32>, fighting: bool, switching: bool, swimming: bool, pending: bool, target: Option<u32>, kind: Target, own: Own) -> (bool, Option<Attack>, u8) {
    if swimming {
        return (fighting, Some(Attack::Refuse(&["Feedback_CantAttackInThisState"])), 0);
    }
    if fighting && (!switching || target.is_some_and(|t| fought == Some(t))) {
        return (true, None, 0);
    }
    let flag = if fighting { 2 } else { 0 };
    let attack = match (target, attackable(swimming, kind, own)) {
        (Some(_), Ok(())) if pending => Attack::Refuse(&["Feedback_PleaseWaitUntilPreviousAction"]),
        (Some(t), Ok(())) => Attack::Start(t),
        (_, Err(Some("Feedback_YouCantAttackYourself"))) => Attack::Refuse(&["Feedback_YouCantAttackYourself", "Feedback_UnableToAttackTarget"]),
        (_, Err(Some("Feedback_YouCantAttackWhileSwimming"))) => Attack::Refuse(&["Feedback_YouCantAttackWhileSwimming", "Feedback_UnableToAttackTarget"]),
        (_, Err(Some("Feedback_YoureUnableToAttack"))) => Attack::Refuse(&["Feedback_YoureUnableToAttack", "Feedback_UnableToAttackTarget"]),
        (_, Err(Some("Feedback_TargetIsAlreadyDead"))) => Attack::Refuse(&["Feedback_TargetIsAlreadyDead", "Feedback_UnableToAttackTarget"]),
        _ => Attack::Refuse(&["Feedback_UnableToAttackTarget"]),
    };
    (fighting, Some(attack), flag)
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

/// The radius the original counts for each body in a special attack's range: the field it reads (+0x30 of the object at
/// a character's +0x1ec, Gamecode FUN_10067f0a) is 1.0 from that object's constructor (FUN_1004496c), and nothing was found
/// to change it.
pub const BODY_RADIUS: f32 = 1.0;

/// How far a special attack reaches with an item of this range (stat 287), for a character with this bonus (stat 380, in
/// percent): the range and the bonus's share of it, cut to a whole metre, no more than 40 (Gamecode FUN_1009abd5).
pub fn special_range(range: i64, bonus: i64) -> i64 {
    ((bonus as f64 * range as f64 / 100.0 + range as f64) as i64).min(40)
}

/// Whether a special attack reaches a target this far away, centre to centre (Gamecode FUN_10067f0a): the distance less the
/// attacker's body and, for a character, the target's, must be less than the reach (not equal).
pub fn special_reaches(distance: f32, character: bool, range: i64, bonus: i64) -> bool {
    let gap = distance - BODY_RADIUS - if character { BODY_RADIUS } else { 0.0 };
    special_range(range, bonus) as f64 > gap as f64
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
    /// CMB-022: Brawl's item reaches 3 m past the two bodies, so a target 4.9 m away is reached and one 5 m away is not;
    /// a bonus of 50 % makes it 4 m (cut, not rounded); no item reaches past 40 m.
    #[test]
    fn a_special_reaches_by_its_items_range() {
        assert!(special_reaches(4.9, true, 3, 0) && !special_reaches(5.0, true, 3, 0));
        assert!(special_reaches(3.9, false, 3, 0) && !special_reaches(4.0, false, 3, 0));
        assert_eq!((special_range(3, 50), special_range(3, 40), special_range(30, 100)), (4, 4, 40));
        assert!(special_reaches(11.5, true, 10, 0) && !special_reaches(12.0, true, 10, 0));
    }

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
        let (alive, me) = (Target::Character { dead: false }, Own::default());
        let key = |fighting, swimming, pending, target, kind, own| attack_key(None, fighting, false, swimming, pending, target, kind, own);
        assert_eq!(key(true, false, false, Some(9), alive, me), (true, None, 0));
        assert_eq!(key(false, false, false, Some(9), alive, me), (false, Some(Attack::Start(9)), 0));
        assert_eq!(key(false, false, false, Some(9), Target::Character { dead: true }, me), (false, Some(Attack::Refuse(&["Feedback_TargetIsAlreadyDead", "Feedback_UnableToAttackTarget"])), 0));
        assert_eq!(key(false, false, false, Some(1), Target::Yourself, me), (false, Some(Attack::Refuse(&["Feedback_YouCantAttackYourself", "Feedback_UnableToAttackTarget"])), 0));
        assert_eq!(key(false, false, false, None, Target::Nothing, me), (false, Some(Attack::Refuse(&["Feedback_UnableToAttackTarget"])), 0));
        assert_eq!(key(false, false, true, Some(9), alive, me), (false, Some(Attack::Refuse(&["Feedback_PleaseWaitUntilPreviousAction"])), 0));
        // (CMB-003: swimming says its own line, after a fight is stopped; flying or state flag 0x400 can't attack; flag
        // 0x80000000 with stat 215 lets one attack even the dead)
        assert_eq!(key(true, true, false, Some(9), alive, me), (true, Some(Attack::Refuse(&["Feedback_CantAttackInThisState"])), 0));
        let unable = Some(Attack::Refuse(&["Feedback_YoureUnableToAttack", "Feedback_UnableToAttackTarget"]));
        assert_eq!(key(false, false, false, Some(9), alive, Own { flying: true, ..me }), (false, unable, 0));
        assert_eq!(key(false, false, false, Some(9), alive, Own { flags: 0x406, ..me }), (false, unable, 0));
        let free = Own { flags: 0x8000_0006u32 as i64, stat_215: 1, flying: true };
        assert_eq!(key(false, false, false, Some(9), Target::Character { dead: true }, free), (false, Some(Attack::Start(9)), 0));
        assert_eq!(key(false, false, false, Some(1), Target::Yourself, free).1, Some(Attack::Refuse(&["Feedback_YouCantAttackYourself", "Feedback_UnableToAttackTarget"])));
        // (Ctrl + click: not fighting, an attack; fighting another, a stop and an attack with the flag; the one fought, a stop)
        assert_eq!(attack_key(None, false, true, false, false, Some(9), alive, me), (false, Some(Attack::Start(9)), 0));
        assert_eq!(attack_key(Some(5), true, true, false, false, Some(9), alive, me), (true, Some(Attack::Start(9)), 2));
        assert_eq!(attack_key(Some(9), true, true, false, false, Some(9), alive, me), (true, None, 0));
        assert_eq!(attack_key(Some(5), true, true, false, false, Some(1), Target::Yourself, me), (true, Some(Attack::Refuse(&["Feedback_YouCantAttackYourself", "Feedback_UnableToAttackTarget"])), 2));
    }
}
