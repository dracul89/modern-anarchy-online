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
    /// The fight mode of its district ([`fight_mode`]).
    pub fight_mode: i64,
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
    /// The fight mode of the character's district (FUN_1003e3e2: [`fight_mode`]).
    pub fight_mode: i64,
}

/// The fight mode (`FightTypeAllowed_e`) of a character's district: FUN_1003e3e2 takes the playfield's district info for the
/// character's zone, its district's mode (`DistrictData_t` +0x54, 0 to 4: text.mdb's `SuppressionField100`, 75, 25, 5 and
/// 0) through the playfield's `FightModeHandler_t` (FUN_1011fddd: the district's base and the handler's changes for it,
/// clamped to 0..4); 2 when the playfield has no district info or none for the zone. The handler's changes come from the
/// server and are not kept here: the base stands.
pub fn fight_mode(district: Option<Option<u8>>) -> i64 {
    match district {
        Some(Some(mode)) => mode as i64,
        _ => 2,
    }
}

/// The text.mdb name (category 110) of a fight mode's line, which FUN_1003f4a3 tells when one's own character comes into a
/// district of another fight mode (FUN_1003f5b2, from FUN_1003f6e6 as its zone changes): `SuppressionField100` for 0 ...
/// `SuppressionField0` for 4.
pub fn suppression_field(mode: i64) -> Option<&'static str> {
    ["SuppressionField100", "SuppressionField75", "SuppressionField25", "SuppressionField5", "SuppressionField0"].get(usize::try_from(mode).ok()?).copied()
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
/// FUN_1005d61f's table 0x1005f62e says as `Feedback_NotAllowedToAttackTeamMembers`). The suppression gas line, which only tells, is
/// [`can_attack_told`]'s. Not checked: the own stat 673 test at the start, and the team check's word at the character's
/// +0x1f0 (5 lets a team member be attacked).
pub fn can_attack(own: Own, kind: Target) -> Result<(), Option<&'static str>> {
    can_attack_told(own, kind).0
}

/// [`can_attack`] with whether FUN_10069a9f also told `Feedback_TargetIsInADistrictWithHigherSuppression` on the way: for a
/// character target that got past the surrender checks and does not fight oneself, when its district's fight mode is
/// below one's own (0x10069e67..0x10069ee8). The line refuses nothing.
pub fn can_attack_told(own: Own, kind: Target) -> (Result<(), Option<&'static str>>, bool) {
    let told = matches!(kind, Target::Character { fights_me: false, facts, .. } if facts.fight_mode < own.fight_mode);
    let result = judge(own, kind);
    // (the line is said only on the way past the checks before it: an attack refused before then never reaches it)
    let reached = match result {
        Ok(()) => true,
        Err(Some(line)) => line == "Feedback_NotAllowedToAttackTeamMembers",
        Err(None) => false,
    };
    (result, told && reached && !matches!(kind, Target::Character { facts, .. } if (facts.flags as u32 & 0x8000_0000 != 0 && facts.stat_215 != 0) || (own.flags as u32 & 0x8000_0000 != 0 && own.stat_215 != 0)))
}

fn judge(own: Own, kind: Target) -> Result<(), Option<&'static str>> {
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
    let (judged, told) = can_attack_told(own, kind);
    if told {
        a.lines.push("Feedback_TargetIsInADistrictWithHigherSuppression");
    }
    match (judged, target) {
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

/// What casting depends on: the character's own state and its nano.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Caster {
    /// Dead: the client's own refusal, without a line (the original's check has none for it).
    pub dead: bool,
    /// Its stat 0x296 (662) is set: it is in a vehicle.
    pub vehicle: bool,
    /// Its state flags (stat 224) have 0x800.
    pub forbidden: bool,
    /// Its state flags lack 2 (a fear takes it, `spelllist::Control`).
    pub unavailable: bool,
    pub moving: bool,
    /// Movement mode 4.
    pub swimming: bool,
    /// Movement mode 5 (`WaitState` 0xe here).
    pub crawling: bool,
    pub falling: bool,
    /// Movement modes 8, 0xb and 0xc: sitting, sleeping, lounging (and 9, which the client does not keep).
    pub sitting: bool,
    /// The current nano (stat 214).
    pub nano: i64,
}

/// FUN_1004fbe4 (`N3Msg_CastNanoSpell`'s check) up to the nano's cost: why a nano program can't be cast, with the feedback line
/// (and its number), in the original's order (every line with the category 0x42000018): in a vehicle (stat 0x296)
/// `Feedback_NotInVehicle`; state flag 0x800 `Feedback_ExecutingProgramsForbidden`; without state flag 2
/// `Feedback_ExecutingProgramsUnavailable`; moving (FUN_1005a00b, the vehicle's +0x9c) `..._MustStandStill`; swimming (mode
/// 4), crawling (5), falling (+0x52); sitting, sleeping or lounging (modes 8, 9, 0xb, 0xc)
/// `Feedback_YouMustBeStandingUpToExecuteNanoprogram`; then less nano (stat 0xd6) than the cost (stat 0x197, FUN_1008648b, times
/// the modifier FUN_1004e925, in percent, cut). Dead casts nothing and says nothing of itself: the client's own.
pub fn cast_refusal(caster: Caster, cost: i64, modifier: i64) -> Option<(&'static str, Option<i64>)> {
    if caster.dead {
        return Some(("", None));
    }
    let state = [
        (caster.vehicle, "Feedback_NotInVehicle"),
        (caster.forbidden, "Feedback_ExecutingProgramsForbidden"),
        (caster.unavailable, "Feedback_ExecutingProgramsUnavailable"),
        (caster.moving, "Feedback_UnableToExecuteProgramsMustStandStill"),
        (caster.swimming, "Feedback_YouCantExecuteNanoprogramsWhileSwimming"),
        (caster.crawling, "Feedback_YouCantExecuteNanoprogramsWhileCrawling"),
        (caster.falling, "Feedback_YouCantExecuteNanoprogramsWhileFalling"),
        (caster.sitting, "Feedback_YouMustBeStandingUpToExecuteNanoprogram"),
    ];
    if let Some(&(_, line)) = state.iter().find(|s| s.0) {
        return Some((line, None));
    }
    let needed = (cost as f64 * modifier as f64 / 100.0) as i64;
    (caster.nano < needed).then_some(("Feedback_YouNeedAtLeastRemainingNano", Some(needed)))
}

/// What the nano stacking rules read of a nano program's template: its flags (stat 0), its stacking order (stat 0x227, 551),
/// its strain (stat 0x4b, 75) and the five more lines it belongs to (stats 0x222–0x226, 546–550). A stat the template does not
/// have is 0 here: read as the original's default 1234567890 every nano would share a line with every other, which the game
/// does not do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NanoLine {
    pub flags: i64,
    pub order: i64,
    pub strain: i64,
    pub lines: [i64; 5],
}

/// FUN_1004e70e: whether a running nano is of the new one's line in the way asked. Not `replacing` (FUN_1004fb6a): the running
/// one stacks above (its order higher than the new one's); `replacing` (FUN_1004fb8d): the new one stacks at least as high.
/// The line: when neither has any of the five more lines, the same strain; else any of the running one's six (strain and
/// lines) that is not 0 among the new one's six (FUN_10051e9e).
pub fn same_line(new: &NanoLine, running: &NanoLine, replacing: bool) -> bool {
    if replacing && new.order < running.order || !replacing && running.order <= new.order {
        return false;
    }
    if running.lines.iter().sum::<i64>() == 0 && new.lines.iter().sum::<i64>() == 0 {
        return running.strain == new.strain;
    }
    let theirs = [running.strain, running.lines[0], running.lines[1], running.lines[2], running.lines[3], running.lines[4]];
    let ours = [new.strain, new.lines[0], new.lines[1], new.lines[2], new.lines[3], new.lines[4]];
    theirs.iter().any(|&v| v != 0 && ours.contains(&v))
}

/// The stacking refusals of FUN_1004fbe4 for a nano at a target running `running` (FUN_1004f8ac over the target's running
/// list): one of its line stacking above it `Feedback_NCUErrorBetterProgramRunning` (FUN_1004fb6a); a nano with the flag
/// 0x800000 that would replace one of its line `Feedback_NCUErrorNanoprogramCantReplaceOther` (FUN_1004fb8d).
pub fn stacking_refusal(new: &NanoLine, running: &[NanoLine]) -> Option<&'static str> {
    if running.iter().any(|r| same_line(new, r, false)) {
        return Some("Feedback_NCUErrorBetterProgramRunning");
    }
    if new.flags & 0x80_0000 != 0 && running.iter().any(|r| same_line(new, r, true)) {
        return Some("Feedback_NCUErrorNanoprogramCantReplaceOther");
    }
    None
}

/// FUN_1004fbe4's range for a nano (FUN_1008658e): its stat 0x11f (287) with the caster's stat 0x17d (381) share of it, cut,
/// at most 40 (as a special's, [`special_range`]); the gap is the distance less both bodies.
/// `Feedback_TargetOutOfRangeForNanoprogram` when the gap, cut to whole metres, is past it (FUN_1004fbe4);
/// `Feedback_TargetIsOutsideRange` when the gap itself is (FUN_1004ed2f, after the line of sight).
pub fn nano_range_line(distance: f32, range: i64, bonus: i64) -> Option<&'static str> {
    let reach = special_range(range, bonus);
    let gap = distance - BODY_RADIUS - BODY_RADIUS;
    if gap as i64 > reach {
        Some("Feedback_TargetOutOfRangeForNanoprogram")
    } else if (reach as f64) < gap as f64 {
        Some("Feedback_TargetIsOutsideRange")
    } else {
        None
    }
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
        // (FUN_1004fbe4's order: a vehicle before the state flags, moving before swimming, sitting last)
        let all = Caster { vehicle: true, forbidden: true, unavailable: true, moving: true, swimming: true, crawling: true, falling: true, sitting: true, ..still };
        assert_eq!(cast_refusal(all, 80, 100), Some(("Feedback_NotInVehicle", None)));
        assert_eq!(cast_refusal(Caster { vehicle: false, ..all }, 80, 100), Some(("Feedback_ExecutingProgramsForbidden", None)));
        assert_eq!(cast_refusal(Caster { forbidden: false, vehicle: false, ..all }, 80, 100), Some(("Feedback_ExecutingProgramsUnavailable", None)));
        assert_eq!(cast_refusal(Caster { moving: true, swimming: true, ..still }, 80, 100), Some(("Feedback_UnableToExecuteProgramsMustStandStill", None)));
        assert_eq!(cast_refusal(Caster { crawling: true, ..still }, 80, 100), Some(("Feedback_YouCantExecuteNanoprogramsWhileCrawling", None)));
        assert_eq!(cast_refusal(Caster { sitting: true, ..still }, 80, 100), Some(("Feedback_YouMustBeStandingUpToExecuteNanoprogram", None)));
        // (the range: 10 m and the two bodies; the gap cut to whole metres first, then the gap itself)
        assert_eq!(nano_range_line(12.0, 10, 0), None);
        assert_eq!(nano_range_line(12.5, 10, 0), Some("Feedback_TargetIsOutsideRange"));
        assert_eq!(nano_range_line(13.0, 10, 0), Some("Feedback_TargetOutOfRangeForNanoprogram"));
        assert_eq!(nano_range_line(14.9, 10, 30), None);
        // (NAN-041: the stacking: a better one of the strain running refuses; a weaker one is replaced, unless the new one
        // has the flag 0x800000; the lines 546-550 tie nanos of other strains)
        let nano = |order: i64, strain: i64, lines: [i64; 5], flags: i64| NanoLine { flags, order, strain, lines };
        let new = nano(10, 147, [0; 5], 0);
        assert_eq!(stacking_refusal(&new, &[nano(20, 147, [0; 5], 0)]), Some("Feedback_NCUErrorBetterProgramRunning"));
        assert_eq!(stacking_refusal(&new, &[nano(10, 147, [0; 5], 0), nano(5, 147, [0; 5], 0), nano(50, 196, [0; 5], 0)]), None);
        assert_eq!(stacking_refusal(&nano(10, 147, [0; 5], 0x80_0000), &[nano(10, 147, [0; 5], 0)]), Some("Feedback_NCUErrorNanoprogramCantReplaceOther"));
        assert_eq!(stacking_refusal(&nano(10, 223, [151, 3, 0, 0, 0], 0), &[nano(20, 151, [0; 5], 0)]), Some("Feedback_NCUErrorBetterProgramRunning"));
        assert_eq!(stacking_refusal(&nano(10, 223, [151, 0, 0, 0, 0], 0), &[nano(20, 152, [0; 5], 0)]), None);
        assert_eq!(cast_refusal(Caster { moving: true, ..still }, 80, 100), Some(("Feedback_UnableToExecuteProgramsMustStandStill", None)));
        assert_eq!(cast_refusal(Caster { falling: true, ..still }, 80, 100), Some(("Feedback_YouCantExecuteNanoprogramsWhileFalling", None)));
        assert_eq!(cast_refusal(still, 300, 50), Some(("Feedback_YouNeedAtLeastRemainingNano", Some(150))));
        assert_eq!(cast_refusal(Caster { dead: true, ..still }, 80, 100), Some(("", None)));
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
        // (a target in a district of more suppression: the line, and the attack goes out all the same)
        let suppressed = Target::Character { dead: false, fights_me: false, mate: false, facts: Facts { fight_mode: 1, ..Facts::default() } };
        let at_two = Own { fight_mode: 2, ..me };
        assert_eq!(default_attack(None, false, false, at_two, false, Some(9), suppressed), Attack { start: Some((9, 0)), lines: vec!["Feedback_TargetIsInADistrictWithHigherSuppression"], ..Attack::default() });
        assert_eq!(default_attack(None, false, false, Own { fight_mode: 1, ..me }, false, Some(9), suppressed), start(9, 0));
        let fighting_me = Target::Character { dead: false, fights_me: true, mate: false, facts: Facts { fight_mode: 1, ..Facts::default() } };
        assert_eq!(default_attack(None, false, false, at_two, false, Some(9), fighting_me), start(9, 0));
        assert_eq!((fight_mode(None), fight_mode(Some(None)), fight_mode(Some(Some(1)))), (2, 2, 1));
        assert_eq!((suppression_field(0), suppression_field(4), suppression_field(5)), (Some("SuppressionField100"), Some("SuppressionField0"), None));
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
        let free = Own { flags: 0x8000_0006u32 as i64, stat_215: 1, flying: true, swimming: false, held: false, fight_mode: 0 };
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
