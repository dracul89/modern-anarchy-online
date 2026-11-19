//! An item's requirements as text, as Gamecode writes them for the info view: `N3Msg_GetUseByString` (0x100230a3) heads
//! each action's requirements with its name (text database category 1005: "Get:", "Drop:", "Use:", "Repair:", "UseOn:",
//! "Wear:", "Unwear:", "Wield:", "Unwield:", "Split:" for actions 1 to 10) and `N3Msg_GetCriteriaString` (0x1002135d) →
//! `N3Msg_CriteriaListToString` (0x10020bf0) writes the criteria: read from the end (they are postfix), "On Self:" or the
//! apply-on criterion's own heading (category 1005, key 1000 + its operator: 1019 "On Self:", 1018 "On Target:", 1026
//! "On User:", ...), then `CriteriaAndOrNot` (0x1002070c): an and/or writes its second operand, the operator's word
//! (" and ", " or ") and a new line, then its first; groups of two lines or more are indented (`CriteriaTextIndent`
//! 0x1001c389 puts ". " after the indent of each of their lines); "not" goes before its operand. Each criterion is
//! `ConvertCriteria` (0x1001e2cf): a stat's name (category 2003) and " exactly ", " below ", " from " (the value plus one),
//! " not " ... and the value; a profession its name (category 2004).
//!
//! A breed, a sex and a side are written by their names, and `ConvertCriteria`'s own lines are ported: those it picks
//! by the operator (items, nanos, strains, perks, pickers, free places, teams and the like: [`by_operator`]) and most
//! of those it picks by the stat ([`by_stat`]). Not ported: the stats it writes as mission flags (`CompletedMission`
//! with category 2017, stats 0x41-0x43, 0xc6, 0x100, 0x101, 0x12f, 0x220, 0x221, 0x269-0x26b, 0x2ad, 0x2ae, 0x2b2-0x2b6),
//! the flag lists of stats 0x112 and 0x167 (and 0x2b2) under "all of"/"none of", the team side (0xd5), the
//! expansions (0x185), the random rolls (0x208, 0x20a); they are written as a plain stat criterion here.
//! `N3Msg_CriteriaListToString` first moves each apply-on criterion down past the and/or/not before it ([`reorder`]).

/// A criterion: (stat, value, operator) as the item templates keep them.
pub type Criterion = (u32, i32, u32);

fn ldb(category: i32, key: i32) -> Option<String> {
    crate::feedback::texts().and_then(|t| t.get(category, key)).map(str::to_string)
}

fn ldb_named(category: i32, name: &str) -> String {
    crate::feedback::texts().and_then(|t| t.named(category, name)).map(str::to_string).unwrap_or_default()
}

/// `IsApplyOnCriteria` (0x1001ab59).
fn is_apply_on(c: &Criterion) -> bool {
    c.0 == 0 && matches!(c.2, 100 | 110 | 19 | 18 | 21 | 26)
}

/// `IsOperatorCriteria` (0x1001ab9b): and (4), or (3).
fn is_operator(c: &Criterion) -> bool {
    c.0 == 0 && (c.2 == 3 || c.2 == 4)
}

/// `FindApplyOnCriteria` (0x1001e248): the heading of the first apply-on criterion from `pos` down, "On Self:" (19)
/// without one.
fn apply_on_heading(list: &[Criterion], pos: usize) -> String {
    let op = (0..pos).rev().map(|i| &list[i]).find(|c| is_apply_on(c)).map_or(19, |c| c.2);
    ldb(1005, 1000 + op as i32).unwrap_or_default()
}

/// The name of a stat (category 2003), else the client's own.
pub fn stat_name(stat: u32) -> String {
    ldb(2003, stat as i32).unwrap_or_else(|| crate::skills::stat_name(stat).to_string())
}

fn text(category: i32, name: &str) -> String {
    ldb_named(category, name)
}

fn filled(category: i32, name: &str, args: &[mao_formats::ldb::Arg]) -> String {
    mao_formats::ldb::format(&text(category, name), args)
}

/// `N3Msg_GetName` of an item template (identity type 0xf4254) or a nano program (0xcf1b), from the database.
fn template_name(nano: bool, id: u32) -> String {
    let Some(db) = crate::net::item_db() else { return String::new() };
    if nano {
        return super::iteminfo::nano_name(&db, id).unwrap_or_default();
    }
    db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: id }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)).map(|i| i.name).unwrap_or_default()
}

/// FUN_10053bb2: a perk's name, its record's (kind 0xf4264, by the perk's number) template's; with `rank`, a space
/// and the rank, the number of records down its chain of ranks before it (+8) and itself. (The stream's exact form
/// of the rank was not read: a guess that it is written as a plain number.)
fn perk_name(perk: u32, rank: bool) -> String {
    let Some(db) = crate::net::item_db() else { return String::new() };
    let record = |id: u32| {
        let r = db.get(mao_formats::resource::Id { kind: 0xf4264, instance: id }).ok().flatten()?;
        let word = |at: usize| r.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        (word(4)? == id).then(|| (word(12).unwrap_or(0), word(16).unwrap_or(0)))
    };
    let Some((template, mut prev)) = record(perk) else { return String::new() };
    let name = template_name(false, template);
    if !rank {
        return name;
    }
    let mut count = 1;
    while prev != 0 {
        let Some((_, p)) = record(prev) else { break };
        prev = p;
        count += 1;
    }
    format!("{name} {count}")
}

/// `ConvertCriteria`'s branches by the operator (0x1001e31d on, and its table at 0x10020367 for 0x70 to 0x8f), which
/// it takes before it looks at the stat: a text of category 1005 (or 100 or 110) by its name, some fed a name or a
/// number.
fn by_operator(c: &Criterion) -> Option<String> {
    use mao_formats::ldb::Arg;
    let (stat, value, op) = *c;
    let v = value as u32;
    // (category 2015: the pickers' names, "self", "user", "target" ...)
    let picker = |w: u32| ldb(2015, w as i32).unwrap_or_default();
    Some(match op {
        0x20 => filled(1005, "MustNotHaveItemEquipped", &[Arg::Text(&template_name(false, v))]),
        0x21 => filled(1005, "MustHaveItemEquipped", &[Arg::Text(&template_name(false, v))]),
        0x23 => filled(1005, "MustHaveFormula", &[Arg::Text(&template_name(true, v))]),
        0x24 => filled(1005, "MustNotHaveFormula", &[Arg::Text(&template_name(true, v))]),
        0x2c => text(1005, "IsNPC"),
        // (one picker against another: oneself and the target is its own line; the value's picker is fed first)
        0x32 if (stat, v) == (1, 3) || (stat, v) == (3, 1) => text(110, "Feedback_TargetMustBeSelf"),
        0x32 => filled(110, "Feedback_MustBeSameAs", &[Arg::Text(&picker(v)), Arg::Text(&picker(stat))]),
        0x46 => text(100, "MustBeFlying"),
        0x50 => text(1005, "MustBeInOrgAndNW"),
        // (a nano program by its name; a strain by its name, category 2009)
        0x5b => filled(1005, "AffectedBy", &[Arg::Text(&template_name(true, v))]),
        0x5c => filled(1005, "AffectedBy", &[Arg::Text(&ldb(2009, value).unwrap_or_default())]),
        0x65 => filled(1005, "NotAffectedBy", &[Arg::Text(&template_name(true, v))]),
        0x66 => filled(1005, "NotAffectedBy", &[Arg::Text(&ldb(2009, value).unwrap_or_default())]),
        // (the perk's name with its rank after the text; locked: `N3Msg_GetPerkName` without the rank, fed)
        0x5d => text(100, "MustHavePerk") + &perk_name(v, true),
        0x67 => text(100, "MustNotHavePerk") + &perk_name(v, true),
        0x5e => filled(1005, "PerkMustBeLocked", &[Arg::Text(&perk_name(v, false))]),
        0x61 => filled(1005, "PerkCannotBeLocked", &[Arg::Text(&perk_name(v, false))]),
        0x6a if value == 1 => text(1005, "MustHaveAvailableInvSlot"),
        0x6a => filled(1005, "MustHaveAvailableInvSlots", &[Arg::Int(value as i64)]),
        0x6c => filled(1005, "MustHaveUnique", &[Arg::Text(&template_name(false, v))]),
        0x6d => filled(1005, "MustNotHaveUnique", &[Arg::Text(&template_name(false, v))]),
        0x6f => text(1005, "NoVehicleEquipped"),
        0x70 => text(100, "FlyingMustMeAllowedOnThePlayfield"),
        0x75 => text(110, "Feedback_MustHaveQuest"),
        0x76 => text(1005, "MustBeCastersPet"),
        0x77 => text(110, "IsInGracePeriod"),
        0x78 => text(110, "IsInLCAreaRange"),
        0x79 => text(1005, "MustBeInRaid"),
        0x7b => text(1005, "InDuel"),
        0x7c => text(1005, "MustBeAbleToTeleport"),
        0x7d => text(1005, "MustNotHaveAnythingWorn"),
        0x7f => filled(1005, "MustHaveNCUFor", &[Arg::Text(&template_name(true, v))]),
        0x80 => filled(1005, "MustNotHaveNCUFor", &[Arg::Text(&template_name(true, v))]),
        0x87 => text(1005, "MustAlliedCombat"),
        0x88 => text(1005, "MustNotAlliedCombat"),
        0x8a..=0x8f => {
            const NAMES: [&str; 6] = [
                "Feedback_TargetMustBeInTeamWith",
                "Feedback_TargetMustNotBeInTeamWith",
                "Feedback_TargetMustBeInRaidWith",
                "Feedback_TargetMustNotBeInRaidWith",
                "Feedback_TargetMustBeInOrgWith",
                "Feedback_TargetMustNotBeInOrgWith",
            ];
            filled(110, NAMES[(op - 0x8a) as usize], &[Arg::Text(&picker(v))])
        }
        _ => return None,
    })
}

/// `ConvertCriteria`'s branches by the stat (0x1001ed11 on) that write a text of their own; `None` for the stats it
/// writes with the stat's name, and for those not ported (see the module's notes).
fn by_stat(c: &Criterion) -> Option<String> {
    use mao_formats::ldb::Arg;
    let (stat, value, op) = *c;
    let named = |name: &str| text(1005, name);
    Some(match stat {
        // (an organization: only with the value 0)
        5 => match (value, op) {
            (0, 0x55) => named("Must_inYourOrganization"),
            (0, 0) => named("WithoutOrganization"),
            (0, 0x18) => named("MustBeInOrganization"),
            _ => String::new(),
        },
        // (a team: 0 is none, unequal turns it round)
        6 => named(if (value == 0) == (op == 0x18) { "MustBeInTeam" } else { "CannotBeInTeam" }),
        // (the movement mode: only flying, 7, has words; the original writes a line of its own for the rest)
        0xad if value == 7 => named(if op == 0x18 { "CannotBeFlying" } else { "MustBeFlying" }),
        0xad => "Unsupported movement mode.".into(),
        // (the specializations: the highest bit set)
        0xb6 => [(8, "4th_Specialization"), (4, "3rd_Specialization"), (2, "2nd_Specialization"), (1, "1st_Specialization")].iter().find(|b| value & b.0 != 0).map(|b| named(b.1)).unwrap_or_default(),
        // (the weapons that may be wielded, by the value; its flag tests, 0x16 and 0x6b, are not ported)
        0x112 if op != 0x16 && op != 0x6b => match value {
            1 => named("CannotWieldWeapons"),
            2 => named("WhileWieldingMelee"),
            4 => named("WhileWieldingDistance"),
            5 => named("NotWieldingMelee"),
            6 => named("MustMeleeAndDistance"),
            7 => named("MustMelee_Distance_MA"),
            _ => String::new(),
        },
        // (experience gain: its flag 0x10; otherwise the operator's words without the stat's name)
        0x15d => match op {
            0x6b if value & 0x10 != 0 => named("XPGainNotDisabled"),
            0x16 if value & 0x10 != 0 => named("XPGainDisabled"),
            _ => generic("", value, op),
        },
        // (an expansion flag, the highest of 0x10, 8, 4, 2, 1 and 0x40: a text of category 1005 by its number, 500 on,
        // the next number unless the operator is "none of")
        0x163 => match [(0x10, 0x1f4), (8, 0x1f6), (4, 0x1f8), (2, 0x1fa), (1, 0x1fc), (0x40, 0x1fe)].iter().find(|b| value & b.0 != 0) {
            Some(&(_, key)) => ldb(1005, key + i32::from(op != 0x6b)).unwrap_or_default(),
            None => String::new(),
        },
        // (a shape: 0 the normal one; a few monster bodies and the Manta by their numbers)
        0x167 if op != 0x16 && op != 0x6b => match value {
            0 => named(if op == 0 { "NormalShape" } else { "MustBePolymorphed" }),
            0x44f7 => named("MonsterTypeLeet"),
            0x452e => named("MonsterTypePitLizard"),
            0x768c => named("MonsterTypeWolf"),
            0x7694 => named("MonsterTypeSabretooth"),
            0x769d => named("MonsterTypeReet"),
            0x31d88..=0x31d8a => named("MustHaveThisManta"),
            _ => named("MustBeTransformed"),
        },
        // (the body's scale: the value and 100, in percent)
        0x168 => filled(1005, "Scale", &[Arg::Int(value as i64 + 100)]),
        // (the target's type: category 2007 after "Target " or "Target not ")
        0x18d => named(if op == 0x6b { "CriteriaTargetTypeNot" } else { "CriteriaTargetType" }) + &ldb(2007, value).unwrap_or_default(),
        0x19a => named(if value == 0 { "CannotBeFighting" } else { "MustBeFighting" }),
        0x1ae if value == 2 => named("MustBeOnGround"),
        0x1ae => String::new(),
        // (the selected target: only 0x23 has words; the original writes a line of its own with the number)
        0x1af if value == 0x23 => named("CannotBeMonster"),
        0x1af => format!("Unsupported selected target {}", value as u32),
        0x1b2 if value == 10 => named("MustBeInCombat"),
        0x1b2 => String::new(),
        // (where: indoors, a battle station; outdoors)
        0x1b6 => match op {
            0x16 if value & 1 != 0 => named("IndoorsOnly"),
            0x16 if value & 2 != 0 => named("BattleStationOnly"),
            0x6b if value & 1 != 0 => named("OutdoorsOnly"),
            _ => String::new(),
        },
        // (a player, the kinds of pet, a control tower; any operator but equal: not; any other number is fed to its
        // own line)
        0x1c7 => {
            let kinds = [
                (0, "MustBePlayer", "MustNotBePlayer"),
                (0x5e, "MustBeCharmPet", "MustNotBeCharmPet"),
                (0x5f, "MustBeRobotPet", "MustNotBeRobotPet"),
                (0x60, "MustHealingPet", "MustNotHealingPet"),
                (0x61, "MustBeCombatPet", "MustNotBeCombatPet"),
                (0x62, "MustPsychosisPet", "MustNotPsychosisPet"),
                (0x9d, "MustControlTower", "MustNotControlTower"),
            ];
            match kinds.iter().find(|k| k.0 == value) {
                Some(k) => named(if op == 0 { k.1 } else { k.2 }),
                None => filled(1005, "MustUnsupported", &[Arg::Int(value as i64)]),
            }
        }
        0x48 | 0x1d6 | 0x1d8 | 0x249 | 0x24a => String::new(),
        0x1d7 if value == 0 && op == 0 => named("MustNotHaveOtherMaps"),
        0x1d7 => String::new(),
        0x213 => named(if value & 1 != 0 { "OnlyInShadowlands" } else { "OnlyOnRubi-ka" }),
        // (the account: paid, free; otherwise the operator's words without the stat's name)
        0x294 => match op {
            0x6b if value & 1 != 0 => named("PayingPlayer"),
            0x16 if value & 1 != 0 => named("FreePlayer"),
            _ => generic("", value, op),
        },
        _ => return None,
    })
}

/// `ConvertCriteria` (0x1001e2cf), for the operators and the stat criteria.
pub fn convert(c: &Criterion) -> String {
    let (stat, value, op) = *c;
    if let Some(s) = by_operator(c) {
        return s;
    }
    if stat == 0 {
        // (the and/or/not operators' words)
        return match op {
            3 => ldb_named(1005, "OperatorOr"),
            4 => ldb_named(1005, "OperatorAnd"),
            0x18 => ldb_named(1005, "OperatorUnequal"),
            0x2a => ldb_named(1005, "OperatorNot"),
            _ => String::new(),
        };
    }
    if stat == 60 || stat == 368 {
        // (a profession: its name, "Visual Profession: " before a visual one's, "Not " before an unequal one's)
        if value == 0 {
            return String::new();
        }
        let mut s = ldb(2004, value).unwrap_or_default();
        if stat == 368 {
            s.insert_str(0, &ldb_named(1005, "VisualProfession"));
        }
        if op == 0x18 {
            s.insert_str(0, &ldb_named(1005, "OperatorNotOf"));
        }
        return s;
    }
    if stat == 33 || stat == 4 || stat == 0x3b {
        // (a side: its name, category 2005; a breed (stat 4) and a sex (0x3b) theirs, which Gamecode takes from its own
        // name tables (FUN_10036b15, FUN_10036a4a) and here from the text database's (category 1005, 600 and 100 on,
        // as the feedback lines name them); "Not " before an unequal one's)
        let mut s = match stat {
            4 => ldb(1005, 600 + value),
            0x3b => ldb(1005, 100 + value),
            _ => ldb(2005, value),
        }
        .unwrap_or_default();
        if op == 0x18 {
            s.insert_str(0, &ldb_named(1005, "OperatorNotOf"));
        }
        return s;
    }
    if let Some(s) = by_stat(c) {
        return s;
    }
    generic(&stat_name(stat), value, op)
}

/// The stat criteria's words (0x1001f9a6 on): the name, the operator's word and the value.
fn generic(name: &str, value: i32, op: u32) -> String {
    let mut s = name.to_string();
    let mut value = value as i64;
    let word = |name: &str| format!(" {} ", ldb_named(1005, name));
    match op {
        0 => s += &word("Op_Equal"),
        1 => s += &word("Op_Less"),
        2 => {
            s += &word("Op_Larger");
            value += 1;
        }
        3 => s += &word("Op_Or"),
        4 => s += &word("Op_And"),
        0x16 => s += &ldb_named(1005, "MustHave_NL"),
        0x18 => s += &word("Op_Unequal"),
        0x49 => {
            s = ldb_named(1005, "BaseFrom").replacen("%s", &s, 1);
            value += 1;
        }
        0x4a => s = ldb_named(1005, "BaseBelow").replacen("%s", &s, 1),
        _ => s += &format!(" Op: {op} Val: "),
    }
    s + &value.to_string()
}

/// `CriteriaTextIndent` (0x1001c389): ". " after the indent of every line that starts with it.
fn text_indent(text: &mut String, indent: &str) {
    let mut starts: Vec<usize> = vec![0];
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    for &s in starts.iter().rev() {
        if indent.is_empty() || text[s..].starts_with(indent) {
            text.insert_str(s + indent.len(), ". ");
        }
    }
}

/// `CriteriaAndOrNot` (0x1002070c): the criteria from `pos` down (the list read backwards) into `out`, counting the
/// lines; false when there is nothing left.
fn and_or_not(list: &[Criterion], pos: &mut usize, out: &mut String, count: &mut i32, indent: &str) -> bool {
    if *pos > 0 && is_apply_on(&list[*pos - 1]) {
        *pos -= 1;
        if *pos > 0 {
            *out += &format!("{}\n", apply_on_heading(list, *pos));
        }
    }
    if *pos == 0 {
        return false;
    }
    let c = list[*pos - 1];
    if is_operator(&c) {
        let word = convert(&c);
        *pos -= 1;
        if *pos == 0 {
            return false;
        }
        let (mut a, mut b) = (String::new(), String::new());
        let (mut ca, mut cb) = (0, 0);
        and_or_not(list, pos, &mut a, &mut ca, indent);
        // (the same operator again: a chain, its lines not indented again)
        let chain = *pos > 0 && list[*pos - 1].2 == c.2 && list[*pos - 1].0 == 0;
        and_or_not(list, pos, &mut b, &mut cb, indent);
        if ca >= 2 {
            text_indent(&mut a, indent);
            text_indent(&mut b, indent);
        } else if cb > 1 && !chain {
            text_indent(&mut b, indent);
        }
        let between = if ca < 2 { String::new() } else { format!("\n{indent}") };
        *out += &format!("{a}{between}{word}\n{b}");
        *count += ca + cb + if ca > 1 { 1 } else { 0 };
        return true;
    }
    if !(c.0 == 0 && c.2 == 0x2a) {
        *out += &format!("{indent}{}", convert(&c));
        *pos -= 1;
        *count += 1;
        return true;
    }
    // (not: its word, then its operand on the same line when that is one line)
    let word = convert(&c);
    *pos -= 1;
    if *pos == 0 {
        return false;
    }
    let mut a = String::new();
    let mut ca = 0;
    and_or_not(list, pos, &mut a, &mut ca, indent);
    if ca < 2 {
        a = a.get(indent.len()..).unwrap_or("").to_string();
    } else {
        text_indent(&mut a, indent);
    }
    let sep = if ca < 2 { " " } else { "\n" };
    *out += &format!("{indent}{word}{sep}{a}");
    *count += ca;
    true
}

/// `N3Msg_CriteriaListToString`'s first loop (0x10020c18): from the end of the list back to its second criterion, an
/// apply-on criterion right after a "not" or an and/or (`IsOperatorCriteria`) changes places with it, so that it goes
/// on down past a run of them.
fn reorder(list: &mut [Criterion]) {
    let mut p = list.len();
    while p >= 2 {
        let i = p - 1;
        let before = list[i - 1];
        if is_apply_on(&list[i]) && ((before.0 == 0 && before.2 == 0x2a) || is_operator(&before)) {
            list.swap(i, i - 1);
        }
        p -= 1;
    }
}

/// `N3Msg_CriteriaListToString` (0x10020bf0): the heading, then each criterion group, a line each.
pub fn list_to_string(list: &[Criterion]) -> String {
    let mut list = list.to_vec();
    reorder(&mut list);
    let list = &list[..];
    let mut pos = list.len();
    let mut out = format!("{}\n", apply_on_heading(list, pos));
    loop {
        let mut text = String::new();
        let mut count = 0;
        if !and_or_not(list, &mut pos, &mut text, &mut count, "  ") {
            break;
        }
        out += &text;
        out.push('\n');
        if pos == 0 {
            break;
        }
    }
    out
}

/// `N3Msg_GetUseByString` (0x100230a3): each action's heading and requirements; `None` without any.
pub fn use_by(actions: &[mao_formats::item::Action]) -> Option<String> {
    const NAMES: [&str; 10] = ["Get:", "Drop:", "Use:", "Repair:", "UseOn:", "Wear:", "Unwear:", "Wield:", "Unwield:", "Split:"];
    let mut out = String::new();
    for (k, name) in NAMES.iter().enumerate() {
        let kind = k as u32 + 1;
        for a in actions.iter().filter(|a| a.kind == kind && !a.criteria.is_empty()) {
            let text = list_to_string(&a.criteria);
            if !text.is_empty() {
                out += &ldb_named(1005, name);
                out += &text;
            }
        }
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_and_of_two_stats() {
        // (Strength above 99 and Agility above 49: postfix)
        let list = [(16, 99, 2), (17, 49, 2), (0, 0, 4)];
        let mut pos = list.len();
        let (mut out, mut count) = (String::new(), 0);
        assert!(and_or_not(&list, &mut pos, &mut out, &mut count, "  "));
        assert_eq!(count, 2);
        // the second operand first, then the operator's word and a new line, then the first
        assert!(out.starts_with("  "));
        assert!(out.contains("49") || out.contains("50"));
        assert_eq!(out.matches('\n').count(), 1);
    }

    /// ConvertCriteria (0x1001e2cf): a breed or a sex is written by its name, not as a stat and a number.
    #[test]
    fn a_breed_or_sex_is_named() {
        for c in [(4, 2, 0), (0x3b, 1, 0), (4, 3, 0x18)] {
            let s = convert(&c);
            assert!(!s.contains(&c.1.to_string()) && !s.contains("Op"), "{c:?}: {s}");
        }
        assert_eq!(convert(&(4, 2, 0x18)), ldb_named(1005, "OperatorNotOf") + &ldb(1005, 602).unwrap_or_default());
    }

    /// `ConvertCriteria`'s own lines: by the operator before the stat (a perk, free places, pickers, a strain), and by
    /// the stat for the ones with words of their own (a team, an organization, a pet kind), the rest by the stat's name.
    #[test]
    fn special_criteria_have_their_own_lines() {
        use mao_formats::ldb::{format, Arg};
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        crate::theme::set_game_dir(&game.to_string_lossy());
        let db = std::sync::Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        crate::net::set_item_db(db.clone());
        assert!(crate::feedback::texts().is_some());
        // (a nano program and an item by their names, a perk by its template's name and its rank)
        let nano = super::super::iteminfo::nano_name(&db, 25994).unwrap();
        assert_eq!(convert(&(0, 25994, 0x24)), format(&ldb_named(1005, "MustNotHaveFormula"), &[Arg::Text(&nano)]));
        assert!(convert(&(0, 70254, 0x21)).contains(&template_name(false, 70254)) && !template_name(false, 70254).is_empty());
        assert!(perk_name(101, true).ends_with(" 2") && perk_name(100, true).ends_with(" 1"), "{} / {}", perk_name(100, true), perk_name(101, true));
        assert_eq!(convert(&(0, 1, 0x6a)), ldb_named(1005, "MustHaveAvailableInvSlot"));
        assert_eq!(convert(&(0, 3, 0x6a)), format(&ldb_named(1005, "MustHaveAvailableInvSlots"), &[Arg::Int(3)]));
        assert_eq!(convert(&(1, 3, 0x32)), ldb_named(110, "Feedback_TargetMustBeSelf"));
        let same = convert(&(1, 2, 0x32));
        assert_eq!(same, format(&ldb_named(110, "Feedback_MustBeSameAs"), &[Arg::Text(&ldb(2015, 2).unwrap()), Arg::Text(&ldb(2015, 1).unwrap())]));
        assert_eq!(convert(&(0, 596, 0x5c)), format(&ldb_named(1005, "AffectedBy"), &[Arg::Text(&ldb(2009, 596).unwrap_or_default())]));
        assert!(convert(&(0, 113, 0x5d)).starts_with(&ldb_named(100, "MustHavePerk")));
        assert_eq!(convert(&(0, 0, 0x2c)), ldb_named(1005, "IsNPC"));
        // (by the stat: in a team, not in one; an organization only with 0; a pet's kind and its "not")
        assert_eq!(convert(&(6, 0, 0)), ldb_named(1005, "CannotBeInTeam"));
        assert_eq!(convert(&(6, 0, 0x18)), ldb_named(1005, "MustBeInTeam"));
        assert_eq!(convert(&(5, 0, 0x18)), ldb_named(1005, "MustBeInOrganization"));
        assert_eq!(convert(&(5, 7, 0x18)), "");
        assert_eq!(convert(&(455, 0x5f, 0)), ldb_named(1005, "MustBeRobotPet"));
        assert_eq!(convert(&(455, 0x5f, 0x18)), ldb_named(1005, "MustNotBeRobotPet"));
        assert_eq!(convert(&(360, 10, 0)), format(&ldb_named(1005, "Scale"), &[Arg::Int(110)]));
        // (the experience flag without its bit: the operator's words without a name)
        assert!(convert(&(349, 2, 0x16)).starts_with(&ldb_named(1005, "MustHave_NL")));
        // (an ordinary stat keeps its name)
        assert!(convert(&(16, 99, 2)).starts_with(&stat_name(16)));
    }

    /// The apply-on criterion at a list's end goes down past the and/or/not before it.
    #[test]
    fn apply_on_goes_before_the_operators() {
        let mut list = vec![(16, 99, 2), (17, 49, 2), (0, 0, 4), (0, 0, 0x2a), (0, 0, 18)];
        reorder(&mut list);
        assert_eq!(list, vec![(16, 99, 2), (17, 49, 2), (0, 0, 18), (0, 0, 4), (0, 0, 0x2a)]);
        // (after a plain criterion it stays; the first criterion is never moved)
        let mut list = vec![(16, 99, 2), (0, 0, 18)];
        reorder(&mut list);
        assert_eq!(list, vec![(16, 99, 2), (0, 0, 18)]);
        let mut list = vec![(0, 0, 4), (0, 0, 19)];
        reorder(&mut list);
        assert_eq!(list, vec![(0, 0, 19), (0, 0, 4)]);
    }

    #[test]
    fn indent_marks_the_lines_of_a_group() {
        let mut s = "  a\n  b".to_string();
        text_indent(&mut s, "  ");
        assert_eq!(s, "  . a\n  . b");
    }
}

