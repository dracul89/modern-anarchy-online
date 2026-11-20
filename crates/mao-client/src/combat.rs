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
    /// Whether `N3Msg_GetCloseTarget` takes it for one of one's own side whatever its own side is: a player in the own
    /// team's list (FUN_10065d65) or, in a raid, in any of the raid's lists (FUN_10065dab); a creature whose master
    /// (stat 196) is oneself or such a member. (The client knows only its own pets' masters.)
    pub mate: bool,
}

/// Whether a side is a friend's: the same as one's own.
pub fn friendly(own_side: u8, side: u8) -> bool {
    own_side == side
}

/// Whether a candidate is a friend for the targeting keys (`N3Msg_GetCloseTarget` 0x1001c49a): a mate's side counts as
/// one's own (0x1001c6db), else its own side is compared.
pub fn friend(own_side: u8, c: &Candidate) -> bool {
    c.mate || friendly(own_side, c.side)
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
        if Some(c.id) == current || distance(c.position, own) > TARGET_RADIUS || friend(own_side, c) != want_friendly {
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

/// What the attack key (or a Ctrl + click) does: the fight stopped first (`N3Msg_StopAttack`: the `StopFight` message),
/// an attack sent (FUN_1006817d: the `Attack` message, with its flag: 2 when a fight was left for it), and the feedback
/// lines (text.mdb's names), in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Attack {
    pub stop: bool,
    pub start: Option<(u32, u8)>,
    pub lines: Vec<&'static str>,
    /// An item's requirements refused it (FUN_100816f6 with its feedback on): their own lines come before `lines`.
    pub requirements: bool,
}

/// What the attack key's target is, as FUN_10069a9f looks at it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Nothing,
    Yourself,
    /// A character: whether it is dead, whether it fights oneself (its fight target is the own character: always
    /// attackable, 0x10069e86), whether it is in the own team or raid (FUN_10065d65 / FUN_10065dab), and what the check
    /// reads of its stats ([`Facts`]).
    Character { dead: bool, fights_me: bool, mate: bool, facts: Facts },
    /// An item standing in the world (`SimpleItem_t`): whether its flags have 0x10000000, and whether its requirements
    /// for action 11 are met (FUN_100816f6(0xb, own)).
    Item { attackable: bool, met: bool },
    /// Anything else (a terminal: no dynel).
    Other,
}

/// What FUN_10069a9f reads of a target character's stats (the client holds another's stats from the server's `Stat`
/// messages, as the original's dynel does; 0 for a stat never told): its state flags (stat 224, `Features`), stat 215
/// (`GmLevel`), its health (stat 27), whether it is an NPC (+0x21c), its `NPCIsSurrendering` (449) and whether its
/// `NPCSurrenderInstance` (451) is the own character's (FUN_10001ddc: `(0xc350, stat 451)` against the own identity).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub flags: i64,
    pub stat_215: i64,
    pub health: i64,
    pub npc: bool,
    pub surrendering: i64,
    pub surrender_to_me: bool,
}

/// What FUN_10069a9f asks of the own character: its movement mode swimming (4) or flying (7), its state flags (stat
/// 224, read through the +0x1ec object's +0x40, the character itself: FUN_10044dd8) and its stat 215.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Own {
    pub swimming: bool,
    pub flying: bool,
    pub flags: i64,
    pub stat_215: i64,
    /// The character's byte +0x21d (`spelllist::Control`: feared, or held by the spell 0xcff0), which `N3Msg_StopAttack` and
    /// the attack's sender FUN_1006817d ask (not FUN_10069a9f).
    pub held: bool,
}

/// FUN_10069a9f(target, with feedback), for a player's own character: whether the target may be attacked, and the line
/// it says when it may not. In its order: nothing there, or not a character (a terminal: no dynel), is refused without a
/// word; an item (`SimpleItem_t`, before any of the character's own checks) `Feedback_YouCantAttackThisItem` without its
/// flag 0x10000000, else by its requirements for action 11; oneself `Feedback_YouCantAttackYourself`; a target with state flag 0x20000000 `Feedback_CantbeAttacked`; one's
/// own flag 0x80000000 with one's own stat 215 set may attack any character (0x10069c3a: the own character's +0x1ec,
/// then its stat 0xd7), as may anyone at a target with the same of its own (0x10069c6f, the target's +0x1ec and stat
/// 0xd7); swimming `Feedback_YouCantAttackWhileSwimming`; flag 0x400 or flying `Feedback_YoureUnableToAttack`; a dead
/// target is refused without a word while its health (stat 27) is not 0, else `Feedback_TargetIsAlreadyDead`; an NPC
/// surrendering (stat 449 1 or 2, 0x10069d64): to oneself (stat 451) and done (2) `Feedback_YouAcceptedSurrender`, to
/// another `Feedback_TargetIsSurrendering` (1) or `Feedback_TargetHasSurrendered` (2); one that fights oneself may be
/// attacked; a team or raid member is refused with the local `CharacterAction` 0x76 of parameter 5 (FUN_100692b4, which
/// FUN_1005d61f's table 0x1005f62e says as `Feedback_NotAllowedToAttackTeamMembers`). Not checked: the suppression gas line (the districts' fight modes, FUN_1003e3e2, which
/// only tells and refuses nothing), the own stat 673 test at the start, and the team check's word at the character's
/// +0x1f0 (5 lets a team member be attacked).
pub fn can_attack(own: Own, kind: Target) -> Result<(), Option<&'static str>> {
    let Target::Character { dead, fights_me, mate, facts } = kind else {
        return match kind {
            Target::Yourself => Err(Some("Feedback_YouCantAttackYourself")),
            Target::Item { attackable: false, .. } => Err(Some("Feedback_YouCantAttackThisItem")),
            Target::Item { met: true, .. } => Ok(()),
            _ => Err(None),
        };
    };
    if facts.flags as u32 & 0x2000_0000 != 0 {
        return Err(Some("Feedback_CantbeAttacked"));
    }
    if own.flags as u32 & 0x8000_0000 != 0 && own.stat_215 != 0 {
        return Ok(());
    }
    if facts.flags as u32 & 0x8000_0000 != 0 && facts.stat_215 != 0 {
        return Ok(());
    }
    if own.swimming {
        return Err(Some("Feedback_YouCantAttackWhileSwimming"));
    }
    if own.flags & 0x400 != 0 || own.flying {
        return Err(Some("Feedback_YoureUnableToAttack"));
    }
    if dead {
        return Err(if facts.health != 0 { None } else { Some("Feedback_TargetIsAlreadyDead") });
    }
    if facts.npc && matches!(facts.surrendering, 1 | 2) {
        match (facts.surrender_to_me, facts.surrendering) {
            (true, 2) => return Err(Some("Feedback_YouAcceptedSurrender")),
            (true, _) => {}
            (false, 1) => return Err(Some("Feedback_TargetIsSurrendering")),
            (false, _) => return Err(Some("Feedback_TargetHasSurrendered")),
        }
    }
    if fights_me {
        return Ok(());
    }
    if mate {
        return Err(Some("Feedback_NotAllowedToAttackTeamMembers"));
    }
    Ok(())
}

/// `N3Msg_DefaultAttack(selection, switching)` (0x10028147; the attack key passes `false`, Interfaces.dll 0x10007cba;
/// `N3Msg_SwitchTarget` 0x10028422 `true`): a fighting character stops first; swimming (mode 4) then says
/// `Feedback_CantAttackInThisState` and nothing more; a fight that was going on ends there for the attack key, and for a
/// switch to the one fought. Else the target is checked ([`can_attack`]); a refusal's own line is followed by
/// `Feedback_UnableToAttackTarget`. An attack still on its way (FUN_1006817d's +0x79) says
/// `Feedback_PleaseWaitUntilPreviousAction`, and so does a held character (`Own::held`, FUN_1006817d's +0x21d), whose
/// `N3Msg_StopAttack` also sends nothing. `fought` is who the character fights now.
pub fn default_attack(fought: Option<u32>, fighting: bool, switching: bool, own: Own, pending: bool, target: Option<u32>, kind: Target) -> Attack {
    let mut a = Attack { stop: fighting && !own.held, ..Attack::default() };
    if own.swimming {
        a.lines.push("Feedback_CantAttackInThisState");
        return a;
    }
    if fighting && (!switching || (target.is_some() && target == fought)) {
        return a;
    }
    match (can_attack(own, kind), target) {
        (Ok(()), Some(t)) if !pending && !own.held => a.start = Some((t, if fighting { 2 } else { 0 })),
        (Ok(()), Some(_)) => a.lines.push("Feedback_PleaseWaitUntilPreviousAction"),
        (refusal, _) => {
            a.requirements = matches!(kind, Target::Item { attackable: true, met: false });
            a.lines.extend(refusal.err().flatten());
            a.lines.push("Feedback_UnableToAttackTarget");
        }
    }
    a
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

/// The lines a refused cast tells, in order (`N3Msg_CastNanoSpell` 0x1001b5d4): the check's own line, when it has one, then
/// `Feedback_UnableToExecuteOnThisTarget`, which the cast tells after every refusal of FUN_1004fbe4 (and alone for a held
/// character, `refusal` `None`).
pub fn refused_cast_lines(refusal: Option<(&'static str, Option<i64>)>) -> Vec<(&'static str, Option<i64>)> {
    let mut lines: Vec<_> = refusal.into_iter().filter(|r| !r.0.is_empty()).collect();
    lines.push(("Feedback_UnableToExecuteOnThisTarget", None));
    lines
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
        // (every refusal ends with the cast's own line; one that says nothing of itself has that line alone)
        assert_eq!(refused_cast_lines(cast_refusal(still, 300, 50)), [("Feedback_YouNeedAtLeastRemainingNano", Some(150)), ("Feedback_UnableToExecuteOnThisTarget", None)]);
        assert_eq!(refused_cast_lines(cast_refusal(Caster { dead: true, ..still }, 80, 100)), [("Feedback_UnableToExecuteOnThisTarget", None)]);
        assert_eq!(refused_cast_lines(None), [("Feedback_UnableToExecuteOnThisTarget", None)]);
        assert_eq!(nano_cost_modifier(30, 3, true), 45);
        assert_eq!(nano_cost_modifier(100, 3, true), 100);
        assert_eq!(nano_cost_modifier(30, 3, false), 30);
    }

    use super::*;

    fn at(id: u32, x: f32, side: u8) -> Candidate {
        Candidate { id, position: [x, 0.0, 0.0], side, mate: false }
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

    /// TGT-005: a team mate of another side, and one's own pet, are friends for the targeting keys.
    #[test]
    fn mates_are_friends_whatever_their_side() {
        let mate = Candidate { mate: true, ..at(6, 3.0, 2) };
        let list = [at(1, 10.0, 3), mate, at(4, 7.0, 1)];
        let own = [0.0; 3];
        assert_eq!(close_target(own, 1, None, &list, true, true), Some(6));
        assert_eq!(close_target(own, 1, Some(6), &list, true, true), Some(4));
        assert_eq!(close_target(own, 1, None, &list, true, false), Some(1));
        assert_eq!(close_target(own, 1, Some(1), &list, true, false), None);
    }

    #[test]
    fn the_attack_key_toggles() {
        let alive = Target::Character { dead: false, fights_me: false, mate: false, facts: Facts::default() };
        let me = Own::default();
        let start = |t: u32, flag: u8| Attack { start: Some((t, flag)), ..Attack::default() };
        let refuse = |lines: &[&'static str]| Attack { lines: lines.to_vec(), ..Attack::default() };
        assert_eq!(default_attack(Some(9), true, false, me, false, Some(9), alive), Attack { stop: true, ..Attack::default() });
        assert_eq!(default_attack(None, false, false, me, false, Some(9), alive), start(9, 0));
        assert_eq!(
            default_attack(None, false, false, me, false, Some(9), Target::Character { dead: true, fights_me: false, mate: false, facts: Facts::default() }),
            refuse(&["Feedback_TargetIsAlreadyDead", "Feedback_UnableToAttackTarget"])
        );
        assert_eq!(default_attack(None, false, false, me, false, Some(1), Target::Yourself), refuse(&["Feedback_YouCantAttackYourself", "Feedback_UnableToAttackTarget"]));
        assert_eq!(default_attack(None, false, false, me, false, None, Target::Nothing), refuse(&["Feedback_UnableToAttackTarget"]));
        assert_eq!(default_attack(None, false, false, me, false, Some(7), Target::Other), refuse(&["Feedback_UnableToAttackTarget"]));
        assert_eq!(default_attack(None, false, false, me, true, Some(9), alive), refuse(&["Feedback_PleaseWaitUntilPreviousAction"]));
        // (held, +0x21d: `N3Msg_StopAttack` sends nothing and FUN_1006817d says the wait line)
        let held = Own { held: true, ..me };
        assert_eq!(default_attack(Some(9), true, false, held, false, Some(9), alive), Attack::default());
        assert_eq!(default_attack(None, false, false, held, false, Some(9), alive), refuse(&["Feedback_PleaseWaitUntilPreviousAction"]));
        assert_eq!(default_attack(Some(5), true, true, held, false, Some(9), alive), refuse(&["Feedback_PleaseWaitUntilPreviousAction"]));
        // (Ctrl + click: not fighting, an attack; fighting another, a stop and an attack with the flag; the one fought, a stop)
        assert_eq!(default_attack(None, false, true, me, false, Some(9), alive), start(9, 0));
        assert_eq!(default_attack(Some(5), true, true, me, false, Some(9), alive), Attack { stop: true, ..start(9, 2) });
        assert_eq!(default_attack(Some(9), true, true, me, false, Some(9), alive), Attack { stop: true, ..Attack::default() });
        assert_eq!(
            default_attack(Some(5), true, true, me, false, Some(1), Target::Yourself),
            Attack { stop: true, ..refuse(&["Feedback_YouCantAttackYourself", "Feedback_UnableToAttackTarget"]) }
        );
    }

    /// TGT-004: FUN_10069a9f's refusals the client can tell, in their order.
    #[test]
    fn the_attack_check_refuses_as_the_original() {
        let alive = Target::Character { dead: false, fights_me: false, mate: false, facts: Facts::default() };
        let mate = Target::Character { dead: false, fights_me: false, mate: true, facts: Facts::default() };
        let me = Own::default();
        // swimming: the attack key's own line only, after the stop
        let swimming = Own { swimming: true, ..me };
        assert_eq!(default_attack(Some(5), true, false, swimming, false, Some(9), alive), Attack { stop: true, lines: vec!["Feedback_CantAttackInThisState"], start: None, requirements: false });
        assert_eq!(can_attack(swimming, alive), Err(Some("Feedback_YouCantAttackWhileSwimming")));
        // flying, or stat 224's 0x400
        assert_eq!(can_attack(Own { flying: true, ..me }, alive), Err(Some("Feedback_YoureUnableToAttack")));
        assert_eq!(can_attack(Own { flags: 0x400, ..me }, Target::Character { dead: true, fights_me: false, mate: false, facts: Facts::default() }), Err(Some("Feedback_YoureUnableToAttack")));
        assert_eq!(can_attack(Own { flags: 0x406, ..me }, alive), Err(Some("Feedback_YoureUnableToAttack")));
        // (CMB-003: one's own flag 0x80000000 with stat 215 lets one attack any character, even the dead and while
        // flying, but not oneself)
        let free = Own { flags: 0x8000_0006u32 as i64, stat_215: 1, flying: true, swimming: false, held: false };
        assert_eq!(default_attack(None, false, false, free, false, Some(9), Target::Character { dead: true, fights_me: false, mate: true, facts: Facts::default() }), Attack { start: Some((9, 0)), ..Attack::default() });
        assert_eq!(can_attack(free, Target::Yourself), Err(Some("Feedback_YouCantAttackYourself")));
        assert_eq!(can_attack(Own { flags: 0x8000_0000u32 as i64, ..me }, Target::Character { dead: true, fights_me: false, mate: false, facts: Facts::default() }), Err(Some("Feedback_TargetIsAlreadyDead")));
        // a team mate, unless it fights oneself
        assert_eq!(default_attack(None, false, false, me, false, Some(9), mate), Attack { lines: vec!["Feedback_NotAllowedToAttackTeamMembers", "Feedback_UnableToAttackTarget"], ..Attack::default() });
        assert_eq!(can_attack(me, Target::Character { dead: false, fights_me: true, mate: true, facts: Facts::default() }), Ok(()));
        assert_eq!(can_attack(me, Target::Yourself), Err(Some("Feedback_YouCantAttackYourself")));
    }

    /// CMB-003 / TGT-004: what FUN_10069a9f reads of the target's own stats: its flags 0x20000000 and 0x80000000 (with its
    /// stat 215), its health while dead, and an NPC's surrender (stats 449 and 451).
    #[test]
    fn the_targets_own_stats_refuse_as_the_original() {
        let me = Own::default();
        let with = |facts: Facts| Target::Character { dead: false, fights_me: false, mate: false, facts };
        // (Arete's newcomers carry 0x20000000: refused before anything else, even by one's own 0x80000000)
        let newcomer = with(Facts { flags: 0x2020_8001, ..Facts::default() });
        assert_eq!(can_attack(Own { flags: 0x8000_0000u32 as i64, stat_215: 1, ..me }, newcomer), Err(Some("Feedback_CantbeAttacked")));
        assert_eq!(default_attack(None, false, false, me, false, Some(9), newcomer).lines, vec!["Feedback_CantbeAttacked", "Feedback_UnableToAttackTarget"]);
        // (the target's own 0x80000000 with its stat 215: attackable even by a flyer)
        assert_eq!(can_attack(Own { flying: true, ..me }, with(Facts { flags: 0x8000_0000u32 as i64, stat_215: 2, ..Facts::default() })), Ok(()));
        assert_eq!(can_attack(Own { flying: true, ..me }, with(Facts { flags: 0x8000_0000u32 as i64, ..Facts::default() })), Err(Some("Feedback_YoureUnableToAttack")));
        // (dead with health left: refused without a word)
        let dead = |health: i64| Target::Character { dead: true, fights_me: false, mate: false, facts: Facts { health, ..Facts::default() } };
        assert_eq!(can_attack(me, dead(40)), Err(None));
        assert_eq!(default_attack(None, false, false, me, false, Some(9), dead(40)).lines, vec!["Feedback_UnableToAttackTarget"]);
        assert_eq!(can_attack(me, dead(0)), Err(Some("Feedback_TargetIsAlreadyDead")));
        // (an NPC's surrender: to another, surrendering or done; to oneself, done refuses and surrendering goes on)
        let surrender = |surrendering: i64, surrender_to_me: bool, npc: bool| with(Facts { npc, surrendering, surrender_to_me, ..Facts::default() });
        assert_eq!(can_attack(me, surrender(1, false, true)), Err(Some("Feedback_TargetIsSurrendering")));
        assert_eq!(can_attack(me, surrender(2, false, true)), Err(Some("Feedback_TargetHasSurrendered")));
        assert_eq!(can_attack(me, surrender(2, true, true)), Err(Some("Feedback_YouAcceptedSurrender")));
        assert_eq!(can_attack(me, surrender(1, true, true)), Ok(()));
        assert_eq!(can_attack(me, surrender(3, false, true)), Ok(()));
        assert_eq!(can_attack(me, surrender(1, false, false)), Ok(()));
        // (an item: its flag, then its requirements, whose own lines the caller tells first; flying is not asked of it)
        assert_eq!(can_attack(Own { flying: true, ..me }, Target::Item { attackable: true, met: true }), Ok(()));
        assert_eq!(default_attack(None, false, false, me, false, Some(7), Target::Item { attackable: false, met: true }).lines, vec!["Feedback_YouCantAttackThisItem", "Feedback_UnableToAttackTarget"]);
        let unmet = default_attack(None, false, false, me, false, Some(7), Target::Item { attackable: true, met: false });
        assert!(unmet.requirements && unmet.start.is_none());
        assert_eq!(unmet.lines, vec!["Feedback_UnableToAttackTarget"]);
    }
}
