//! Spells as text, as Gamecode writes them for the info view: `N3Msg_ConvertSpellList` (0x100213ec) writes a line for
//! each spell it knows, under its target's heading (text database category 506, by the spell's target: 1 "On Self:", 2
//! "On User:", 3 "On Target:" ...; written when the target changes), and `N3Msg_FormatModifierString` (0x100203d7)
//! lays a line out: the heading and a new line, two spaces, the text, ", <hits> hits, <delay>s delay" when it repeats,
//! " if" and the spell's criteria (`criteria.rs`, each of their lines four spaces in) when it has some, a new line.
//!
//! The spell types written here (the rest write nothing, as unknown types do in the original): modify a stat
//! (0xcf35, 0xcf14: "Modify %s %d", "%s %d%%" for stats 689 and 535-537, "Scale %u%%" for 360 with the value plus 100),
//! a percentage (0xcff5 "Modify %s %d%%"), a stat scaled by the level (0xcfb7: "%s at level %u %d (%d at lvl 200)",
//! "Scale at lvl ..." for 360), a temporary set (0xcf76: "%s temp set to %d", a profession's name for 368), a change of a
//! skill's base (0xcfc0), a nano program upload (0xcf1b), a nano cast (0xcf3b "Cast "), taunt, a stat set (0xcf22
//! "Set %s %d"), a skill lock (0xcf29, with the reader's own time by its skill lock modifier), a reduced lock (0xcfb9),
//! and the fixed texts of 0xcf48, 0xcf4c, 0xcf7e, 0xcf81, 0xcf82, 0xcf86, 0xcf87 and 0xcfaf; the nano casts (0xcf3b
//! "Cast ", the area ones 0xcf4a "Team Cast ", 0xcf5f "AOE %dm ", 0xcf61 "Cast "), each with a link to the nano's page
//! (`<a href="itemid://53019/<id>">name</a>`); a nano resist (0xcfaa "ResistNano"), an item's special (0xcfbe
//! "EnablesSpecial", with the item's description "EnablesSpecial2"), a perk lock (0xcfc3, the perk's name and rank), a
//! faction set (0xcfc5), a hit (0xcfcc and 0xcf0a: "SpellHealing" / "SpellDamage" on health, "NanoAdd" / "NanoReduce"
//! on nano, "FactionAdd" / "FactionReduce" on the factions 0x230-0x23c), the anchors (0xcfd8, 0xcfd9), the procs (0xcfe8
//! defensive, 0xcfeb offensive), the quests' and mail's texts (0xcfed, 0xd002-0xd004), a lock cleared (0xd000), the
//! organization's cash (0xd001), the end of a fight (0xd005) and sneaking (0xd006). The English words are the original
//! program's own (they are not in the text database). Not written: a hit on a map (stats 0x1d7, 0x1d8, 0x249, 0x24a:
//! "MapArea" and the map's name, from a table of names in the program the client does not have). A nano program's
//! spells (NanoItem_t's reading, FUN_10086774, sets the flag 0x100 on every spell of its lists) add after a health hit
//! the reader's own figure, by its stat 535 (healing) or 536 (damage) when that is not 100 ([`convert_nano`]). A
//! temporary set of the stats 0x1e5-0x1e7, 0x21b-0x21e or 0x228-0x22f writes nothing, as in the
//! original.

use mao_formats::spell::Spell;

fn text(category: i32, name: &str) -> String {
    crate::feedback::texts().and_then(|t| t.named(category, name)).map(str::to_string).unwrap_or_default()
}

/// A text of the database by its number (`LDBface::GetText(category, id)`).
fn text_of(category: i32, id: i32) -> String {
    crate::feedback::texts().and_then(|t| t.get(category, id)).map(str::to_string).unwrap_or_default()
}

/// A nano program's name as a link to its page: `<a href="itemid://%u/%u">%s</a>` of the nano identity 0xcf1b, the id
/// and `N3Msg_GetName`.
fn nano_link(id: i32, nano_name: &dyn Fn(u32) -> Option<String>) -> String {
    format!("<a href=\"itemid://{}/{}\">{}</a>", 0xcf1b, id as u32, nano_name(id as u32).unwrap_or_default())
}

/// The perk records' item templates and the ranks before them, by perk id (the perk manager's entries: the database's
/// records 0xf4264, the id at byte 4, the template at 12, the rank before at 16), read once.
fn perk_ranks() -> Option<&'static std::collections::HashMap<u32, (u32, u32)>> {
    static RANKS: std::sync::OnceLock<std::collections::HashMap<u32, (u32, u32)>> = std::sync::OnceLock::new();
    let db = crate::net::item_db()?;
    Some(RANKS.get_or_init(|| {
        let word = |d: &[u8], at: usize| d.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let ids: Vec<mao_formats::resource::Id> = db.ids().map(|it| it.filter_map(Result::ok).filter(|i| i.kind == 0xf4264).collect()).unwrap_or_default();
        ids.into_iter().filter_map(|id| db.get(id).ok().flatten()).filter_map(|r| Some((word(&r, 4)?, (word(&r, 12)?, word(&r, 16)?)))).collect()
    }))
}

/// A perk's name and rank (Gamecode FUN_10053bb2 with its rank: the name of the perk's item template, a space, and one
/// more than the ranks before it); "Missing perk: %d" for one the manager does not have.
fn perk_name(id: i32) -> String {
    let named = || {
        let ranks = perk_ranks()?;
        let &(template, mut before) = ranks.get(&(id as u32))?;
        let db = crate::net::item_db()?;
        let data = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: template }).ok().flatten()?;
        let name = mao_formats::item::Item::parse(&data)?.name;
        let mut rank = 1;
        while before != 0 {
            rank += 1;
            match ranks.get(&before) {
                Some(&(_, b)) => before = b,
                None => break,
            }
        }
        Some(format!("{name} {rank}"))
    };
    named().unwrap_or_else(|| format!("Missing perk: {id}"))
}

/// An item template's name and description (`N3Msg_CreateDummyItemID` of the template, then `N3Msg_GetName` and
/// `N3Msg_GetDesc`); none when there is no such template.
fn item_text(template: i32) -> Option<(String, String)> {
    let db = crate::net::item_db()?;
    let data = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: template as u32 }).ok().flatten()?;
    mao_formats::item::Item::parse(&data).map(|i| (i.name, i.description))
}

/// A hit's numbers, the low and high values (spell stats 2 and 37) as the faction and nano lines write them: "%d%%" of
/// the low one's size when the high one is 0, "%d" when the two are one, else "%d-%d" of their sizes.
fn hit_numbers(low: i32, high: i32, range: &str) -> String {
    if high == 0 {
        format!("{}%", low.wrapping_abs())
    } else if low == high {
        format!("{}", low.wrapping_abs())
    } else {
        range.replacen("{}", &low.wrapping_abs().to_string(), 1).replacen("{}", &high.wrapping_abs().to_string(), 1)
    }
}

/// A hit (spell 0xcfcc, and 0xcf0a, N3Msg_ConvertSpellList at 0x100224f4): by the stat hit (spell stat 0) and its low and
/// high values (2 and 37). `None` for the hit on a map, not written here (see the module's notes). `own`: a nano
/// program's spell, with the reader's stats 535 and 536 (`N3Msg_GetSkill(.., 2)`).
fn hit_text(s: &Spell, own: Option<(i32, i32)>) -> Option<String> {
    let (stat, low, high) = (s.stat(0), s.stat(2), s.stat(37));
    Some(match stat {
        // (health: healing when both are no lower than 0, else damage; the sizes after a space)
        27 => {
            let healing = low >= 0 && high >= 0;
            let mut t = text(1005, if healing { "SpellHealing" } else { "SpellDamage" });
            let (low, high) = (low.wrapping_abs(), high.wrapping_abs());
            if high == 0 {
                return Some(t + &format!(" {low}%"));
            }
            t += &if low == high { format!(" {low}") } else { format!(" {low}-{high}") };
            // (a nano's: the reader's own figure, by its healing (535) or damage (536) percentage when not 100)
            if let Some((heal, damage)) = own {
                let m = if healing { heal } else { damage };
                if m != 100 {
                    let (a, b) = (m.wrapping_mul(low) / 100, m.wrapping_mul(high) / 100);
                    t += &if a == b { format!("({a})") } else { format!("({a}-{b})") };
                }
            }
            t
        }
        // (nano: the numbers straight after the text, a range in brackets)
        0xd6 => text(1005, if low >= 0 && high >= 0 { "NanoAdd" } else { "NanoReduce" }) + &hit_numbers(low, high, "({}-{})"),
        0x1d7 | 0x1d8 | 0x249 | 0x24a => return None,
        // (a faction: its name, text category 2014, and the numbers)
        0x230..=0x23c => printf(&text(1005, if low >= 0 && high >= 0 { "FactionAdd" } else { "FactionReduce" }), &[text_of(2014, stat), hit_numbers(low, high, "{}-{}")]),
        _ => String::new(),
    })
}

/// A C `printf` of `%s`, `%d`, `%u` and `%%` with the arguments in order; a width before the conversion pads the
/// argument on the left with spaces, or zeros after a `0` flag, and on the right after a `-` ("%2d", "%02d", "%-5s").
fn printf(format: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut it = args.iter();
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some(first) => {
                let mut spec = String::new();
                let mut conversion = Some(first);
                while let Some(c) = conversion.filter(|c| c.is_ascii_digit() || "-+ #.".contains(*c)) {
                    spec.push(c);
                    conversion = chars.next();
                }
                if let Some(a) = it.next() {
                    let flags = spec.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
                    let (left, zero) = (flags.contains('-'), spec[flags.len()..].starts_with('0'));
                    let width: usize = spec[flags.len()..].split('.').next().and_then(|w| w.parse().ok()).unwrap_or(0);
                    let pad = width.saturating_sub(a.chars().count());
                    if left {
                        out += a;
                        out.extend(std::iter::repeat_n(' ', pad));
                    } else {
                        out.extend(std::iter::repeat_n(if zero { '0' } else { ' ' }, pad));
                        out += a;
                    }
                }
            }
            None => out.push('%'),
        }
    }
    out
}

/// `LDBformat` of ints over `%d`/`%u` (and `%%`).
pub fn printf_ints(format: &str, args: &[i32]) -> String {
    printf(format, &args.iter().map(i32::to_string).collect::<Vec<_>>())
}

/// `LDBformat` of floats over `%.02f`-style (two places) or `%f`.
pub fn printf_floats(format: &str, args: &[f32]) -> String {
    let mut out = String::new();
    let mut it = args.iter();
    let mut rest = format;
    while let Some(i) = rest.find('%') {
        out += &rest[..i];
        let spec_end = rest[i + 1..].find(|c: char| c.is_ascii_alphabetic() || c == '%').map_or(rest.len(), |e| i + 1 + e);
        let spec = &rest[i + 1..spec_end.min(rest.len())];
        let conv = rest[spec_end..].chars().next().unwrap_or('f');
        if conv == '%' {
            out.push('%');
        } else {
            let places = spec.trim_start_matches('.').trim_start_matches('0').parse::<usize>().unwrap_or(if spec.contains('.') { 0 } else { 6 });
            let places = if spec.starts_with(".0") && places == 0 { 0 } else { places };
            if let Some(v) = it.next() {
                out += &format!("{v:.places$}");
            }
        }
        rest = &rest[(spec_end + conv.len_utf8()).min(rest.len())..];
    }
    out + rest
}

/// How long a skill lock of `seconds` lasts for a character whose skill lock modifier (stat 382) is `modifier`
/// (`N3Msg_ConvertSpellList`, spell 0xcf29: the modifier no lower than -50, `seconds + modifier * seconds / 100.0 +
/// 0.5`, cut to a whole number as `_ftol` cuts).
pub fn skill_lock_seconds(seconds: i32, modifier: i32) -> i32 {
    let modifier = modifier.max(-50);
    (f64::from(modifier.wrapping_mul(seconds)) / 100.0 + f64::from(seconds) + 0.5) as i32
}

/// One spell's text (the switch of `N3Msg_ConvertSpellList`); `None` for a type it does not write. `skill_lock` is the
/// reader's skill lock modifier (stat 382).
pub fn spell_text(s: &Spell, level: u32, skill_lock: i32, nano_name: &dyn Fn(u32) -> Option<String>) -> Option<String> {
    spell_text_of(s, level, skill_lock, nano_name, None)
}

/// [`spell_text`], for a nano program's spell when `own` holds the reader's stats 535 and 536.
fn spell_text_of(s: &Spell, level: u32, skill_lock: i32, nano_name: &dyn Fn(u32) -> Option<String>, own: Option<(i32, i32)>) -> Option<String> {
    let stat_name = |k: i32| super::criteria::stat_name(k as u32);
    let (a0, a39) = (s.stat(0), s.stat(39));
    Some(match s.kind {
        // (the program's own English formats: "Set %s %d" with the stat's name, text category 2003)
        0xcf22 => format!("Set {} {}", stat_name(a0), a39),
        // ("%s skill locked for %d seconds", or "... for %d (%d) seconds" with the reader's own time when the
        // modifier is not 0)
        0xcf29 => {
            let left = skill_lock_seconds(a39, skill_lock);
            if skill_lock.max(-50) == 0 { format!("{} skill locked for {} seconds", stat_name(a0), a39) } else { format!("{} skill locked for {} ({}) seconds", stat_name(a0), a39, left) }
        }
        // ("Reduce %s by %d seconds", the name from text category 2009)
        0xcfb9 => format!("Reduce {} by {} seconds", crate::feedback::texts().and_then(|t| t.get(2009, a0)).map(str::to_string).unwrap_or_default(), a39),
        0xcf35 | 0xcf14 => {
            if a0 == 0x168 {
                printf(&text(1005, "ModifyScale"), &[(a39 + 100).to_string()])
            } else if matches!(a0, 0x2b1 | 0x217 | 0x218 | 0x219) {
                format!("{} {}%", stat_name(a0), a39)
            } else if a39 == 0 {
                String::new()
            } else {
                format!("Modify {} {}", stat_name(a0), a39)
            }
        }
        // (a value of 0: an empty line, which still takes the target's heading)
        0xcff5 if a39 == 0 => String::new(),
        0xcff5 => format!("Modify {} {}%", stat_name(a0), a39),
        0xcfb7 => {
            let scaled = (level as i64 & 0xffff) * a39 as i64 / 200;
            if a0 == 0x168 {
                printf(&text(1005, "ScaleAt200"), &[level.to_string(), (scaled + 100).to_string(), (a39 + 200).to_string(), (a39 + 100).to_string()])
            } else {
                printf(&text(1005, "SkillAt200"), &[stat_name(a0), level.to_string(), scaled.to_string(), a39.to_string()])
            }
        }
        // (not for the stats 0x1e5-0x1e7, 0x21b-0x21e and 0x228-0x22f: nothing is written)
        0xcf76 if matches!(a0, 0x1e5..=0x1e7 | 0x21b..=0x21e | 0x228..=0x22f) => return None,
        0xcf76 => {
            let what = stat_name(a0);
            if a0 == 0x170 {
                let name = crate::feedback::texts().and_then(|t| t.get(2004, a39)).map(str::to_string).unwrap_or_default();
                printf(&text(1005, "TempProfession"), &[what, name])
            } else {
                printf(&text(1005, "TempStat"), &[what, a39.to_string()])
            }
        }
        0xcfc0 => printf(&text(1005, "BaseModification"), &[stat_name(a0), a39.to_string()]),
        // (a nano with no name: an empty line)
        0xcf1b => nano_name(a39 as u32).map(|name| printf(&text(1005, "UploadNanoprogram"), &[name])).unwrap_or_default(),
        // (the casts: the nano's name as a link to its page; spell stat 39 the nano of a cast, 88 of an area cast, 11
        // the area's radius)
        0xcf3b => format!("Cast {}", nano_link(a39, nano_name)),
        0xcf4a => format!("Team Cast {}", nano_link(s.stat(88), nano_name)),
        0xcf5f => format!("AOE {}m {}", s.stat(11), nano_link(s.stat(88), nano_name)),
        0xcf61 => format!("Cast {}", nano_link(s.stat(88), nano_name)),
        0xcf7d => format!("Taunt {a39}"),
        0xcf48 => "Spawn Item".into(),
        // (spell stat 73's bit 4; else an empty line)
        0xcf4c if s.stat(73) & 4 != 0 => "Root".into(),
        0xcf4c => String::new(),
        // ("ResistNano" of the value and the nano school's name, text category 2009 by spell stat 152)
        0xcfaa => printf(&text(1005, "ResistNano"), &[a39.to_string(), text_of(2009, s.stat(152))]),
        // (the item of template 88 (quality 84): "EnablesSpecial" of its name, "EnablesSpecial2" of its name and
        // description when it has one; an empty line when there is no such item)
        0xcfbe => match item_text(s.stat(88)) {
            Some((name, desc)) if !desc.is_empty() => printf(&text(1005, "EnablesSpecial2"), &[name, desc]),
            Some((name, _)) => printf(&text(1005, "EnablesSpecial"), &[name]),
            None => String::new(),
        },
        0xcfc3 => format!("Perk {} locked for {} seconds", perk_name(a0), a39),
        // ("FactionSet" of the faction's name, text category 2014, and the value)
        0xcfc5 => printf(&text(1005, "FactionSet"), &[text_of(2014, a0), a39.to_string()]),
        0xcfcc | 0xcf0a => hit_text(s, own)?,
        0xcfd8 => "Set Anchor".into(),
        0xcfd9 => "Recall Anchor".into(),
        // ("%s (%d%%): %s": the kind, the chance (39) and the nano (10) as a link)
        0xcfe8 | 0xcfeb => format!("{} ({}%): {}", if s.kind == 0xcfe8 { "Defensive proc" } else { "Offensive proc" }, a39, nano_link(s.stat(10), nano_name)),
        0xcfed => text(1005, "SolveQuest"),
        0xd002 => text(1005, "DeleteQuest"),
        0xd003 => text(1005, "FailQuest"),
        0xd004 => text(1005, "SendMail"),
        // (a lock cleared: by spell stat 1 a skill's lock, its name from text category 2009, or a nano's)
        0xd000 => match s.stat(1) {
            1 => format!("Clear lock {}", text_of(2009, a39)),
            0xcf1b => format!("Clear lock {}", nano_name(a39 as u32).unwrap_or_default()),
            _ => "Unknown lock type".into(),
        },
        // ("ModifyOrgCash" of the value and, with spell stat 73's bit 1, whether it goes to the caster or comes from it)
        0xd001 => {
            let way = if s.stat(73) & 1 == 0 {
                ""
            } else if a39 < 0 {
                " (to caster)"
            } else {
                " (from caster)"
            };
            printf(&text(1005, "ModifyOrgCash"), &[a39.to_string(), way.to_string()])
        }
        0xd005 => printf(&text(1005, "EndFight"), &[if s.stat(73) & 1 != 0 { "attackers" } else { "self" }.to_string()]),
        0xd006 => text(2011, "ActionSneak"),
        0xcf7e => "Pacify".into(),
        0xcf81 => "Fear".into(),
        0xcf82 => "Stun".into(),
        0xcf86 => "Wipe hate list".into(),
        0xcf87 => "Charm".into(),
        0xcfaf => "Spawn Pet".into(),
        _ => return None,
    })
}

/// `N3Msg_ConvertSpellList` over one list: the spells' lines.
pub fn convert(spells: &[Spell], level: u32, skill_lock: i32, nano_name: &dyn Fn(u32) -> Option<String>) -> String {
    convert_of(spells, level, skill_lock, nano_name, None)
}

/// [`convert`] over a nano program's list, whose spells all have the flag 0x100 (NanoItem_t's FUN_10086774): `own` the
/// reader's stats 535 and 536.
pub fn convert_nano(spells: &[Spell], level: u32, skill_lock: i32, nano_name: &dyn Fn(u32) -> Option<String>, own: (i32, i32)) -> String {
    convert_of(spells, level, skill_lock, nano_name, Some(own))
}

fn convert_of(spells: &[Spell], level: u32, skill_lock: i32, nano_name: &dyn Fn(u32) -> Option<String>, own: Option<(i32, i32)>) -> String {
    let mut out = String::new();
    let mut last_target = -1;
    for s in spells {
        let Some(line) = spell_text_of(s, level, skill_lock, nano_name, own) else { continue };
        let target = s.stat(0x20);
        let mut heading = String::new();
        if target != 0 && target != last_target {
            last_target = target;
            heading = crate::feedback::texts().and_then(|t| t.get(506, target)).map(str::to_string).unwrap_or_default();
        }
        let criteria = if s.criteria.is_empty() { String::new() } else { super::criteria::list_to_string(&s.criteria) };
        // (a repeating spell: its hits and delay)
        let (hits, delay) = (s.stat(3), s.stat(4));
        let repeat = if delay > 0 && hits > 1 { format!(", {hits} hits, {:.1}s delay", delay as f32 / 100.0) } else { String::new() };
        out += &format_modifier(&heading, &criteria, &line, &repeat);
    }
    out
}

/// `N3Msg_FormatModifierString` (0x100203d7).
pub fn format_modifier(heading: &str, criteria: &str, line: &str, repeat: &str) -> String {
    if line.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    if !heading.is_empty() {
        s += heading;
        s.push('\n');
    }
    s += &format!("  {line}{repeat}");
    if !criteria.is_empty() {
        let lines: Vec<String> = criteria.split('\n').filter(|l| !l.is_empty()).map(|l| format!("    {l}")).collect();
        s += &format!(" if \n{}", lines.join("\n"));
    }
    s + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A width pads the number as C does.
    #[test]
    fn printf_pads_to_a_width() {
        assert_eq!(printf_ints("%d days, %2d hours, %02d minutes.", &[1, 2, 5]), "1 days,  2 hours, 05 minutes.");
        assert_eq!(printf_ints("%-3d|%3d|%d%%", &[7, 1234, 50]), "7  |1234|50%");
        assert_eq!(printf_ints("%u of %u", &[3, 30]), "3 of 30");
    }

    #[test]
    fn a_modify_spell() {
        let s = Spell { kind: 0xcf35, stats: vec![(3, 1), (4, 0), (0x20, 2), (0x23, 9), (0, 16), (39, 20)], ..Default::default() };
        let t = spell_text(&s, 200, 0, &|_| None).unwrap();
        assert!(t.starts_with("Modify ") && t.ends_with(" 20"));
        let line = format_modifier("On User:", "", &t, "");
        assert!(line.starts_with("On User:\n  Modify "));
    }

    /// N3Msg_ConvertSpellList's set, skill lock and reduce lines (0xcf22, 0xcf29, 0xcfb9).
    #[test]
    fn set_lock_and_reduce_spells() {
        let spell = |kind, value| Spell { kind, stats: vec![(0, 16), (39, value)], ..Default::default() };
        let set = spell_text(&spell(0xcf22, 5), 1, 0, &|_| None).unwrap();
        assert!(set.starts_with("Set ") && set.ends_with(" 5"));
        // (the lock's own time beside it unless the modifier is 0; the modifier no lower than -50, rounded)
        assert!(spell_text(&spell(0xcf29, 60), 1, 0, &|_| None).unwrap().ends_with(" skill locked for 60 seconds"));
        assert!(spell_text(&spell(0xcf29, 60), 1, -25, &|_| None).unwrap().ends_with(" skill locked for 60 (45) seconds"));
        assert_eq!((skill_lock_seconds(60, -90), skill_lock_seconds(15, -33), skill_lock_seconds(10, 5)), (30, 10, 11));
        assert!(spell_text(&spell(0xcfb9, 30), 1, 0, &|_| None).unwrap().ends_with(" by 30 seconds"));
        // (a temporary set of some stats writes nothing)
        let temp = |stat| Spell { kind: 0xcf76, stats: vec![(0, stat), (39, 1)], ..Default::default() };
        assert_eq!((spell_text(&temp(0x1e6), 1, 0, &|_| None), spell_text(&temp(0x22f), 1, 0, &|_| None)), (None, None));
        assert!(spell_text(&temp(0x1e8), 1, 0, &|_| None).is_some());
    }

    /// N3Msg_ConvertSpellList's casts, hits, procs, locks and the rest of its switch.
    #[test]
    fn casts_hits_procs_and_the_rest() {
        let spell = |kind, stats: &[(i32, i32)]| Spell { kind, stats: stats.to_vec(), ..Default::default() };
        let name = |id: u32| (id == 7).then(|| "Heal".to_string());
        let text = |s: &Spell| spell_text(s, 1, 0, &name);
        // (casts: the nano's name as a link; an area cast's nano is spell stat 88, its radius 11)
        assert_eq!(text(&spell(0xcf3b, &[(39, 7)])).as_deref(), Some("Cast <a href=\"itemid://53019/7\">Heal</a>"));
        assert_eq!(text(&spell(0xcf4a, &[(88, 7)])).as_deref(), Some("Team Cast <a href=\"itemid://53019/7\">Heal</a>"));
        assert_eq!(text(&spell(0xcf5f, &[(88, 7), (11, 20)])).as_deref(), Some("AOE 20m <a href=\"itemid://53019/7\">Heal</a>"));
        assert_eq!(text(&spell(0xcf61, &[(88, 9)])).as_deref(), Some("Cast <a href=\"itemid://53019/9\"></a>"));
        // (an upload of a nano with no name, a root without its bit, a percentage of 0: empty lines)
        assert_eq!(text(&spell(0xcf1b, &[(39, 9)])).as_deref(), Some(""));
        assert_eq!((text(&spell(0xcf4c, &[(73, 4)])).as_deref(), text(&spell(0xcf4c, &[(0, 4)])).as_deref()), (Some("Root"), Some("")));
        assert_eq!(text(&spell(0xcff5, &[(0, 16)])).as_deref(), Some(""));
        assert_eq!(text(&spell(0xcf7d, &[(0, 3), (39, 250)])).as_deref(), Some("Taunt 250"));
        // (procs)
        assert_eq!(text(&spell(0xcfe8, &[(39, 15), (10, 7)])).as_deref(), Some("Defensive proc (15%): <a href=\"itemid://53019/7\">Heal</a>"));
        assert!(text(&spell(0xcfeb, &[(39, 5), (10, 7)])).unwrap().starts_with("Offensive proc (5%): "));
        // (locks cleared, anchors, a perk lock of a perk the manager does not have)
        assert_eq!(text(&spell(0xd000, &[(1, 0xcf1b), (39, 7)])).as_deref(), Some("Clear lock Heal"));
        assert_eq!(text(&spell(0xd000, &[(1, 5)])).as_deref(), Some("Unknown lock type"));
        assert_eq!((text(&spell(0xcfd8, &[])).as_deref(), text(&spell(0xcfd9, &[])).as_deref()), (Some("Set Anchor"), Some("Recall Anchor")));
        assert!(text(&spell(0xcfc3, &[(0, 0x7fff_fff0), (39, 30)])).unwrap().ends_with(" locked for 30 seconds"));
        // (hits: on health the sizes after a space, a percentage when the high value is 0; a stat of no kind of its own
        // an empty line; a hit on a map not written)
        let hit = |stat, low, high| text(&spell(0xcfcc, &[(0, stat), (2, low), (37, high)])).unwrap();
        assert!(hit(27, -100, -200).ends_with(" 100-200"));
        assert!(hit(27, 50, 50).ends_with(" 50") && !hit(27, 50, 50).ends_with("-50"));
        assert!(hit(27, 30, 0).ends_with(" 30%"));
        assert!(hit(0xd6, 10, 20).ends_with("(10-20)"));
        assert_eq!(hit(16, 1, 2), "");
        assert_eq!(text(&spell(0xcf0a, &[(0, 0x1d7), (2, 1)])), None);
        // (a nano's: the reader's own figure by its stat 536 for damage, none at 100, none after a percentage)
        let nano_hit = |low, high, own| spell_text_of(&spell(0xcfcc, &[(0, 27), (2, low), (37, high)]), 1, 0, &name, Some(own)).unwrap();
        assert!(nano_hit(-100, -200, (100, 150)).ends_with(" 100-200(150-300)"));
        assert!(nano_hit(50, 50, (90, 100)).ends_with(" 50(45)"));
        assert!(nano_hit(-100, -200, (150, 100)).ends_with(" 100-200"));
        assert!(nano_hit(30, 0, (90, 90)).ends_with(" 30%"));
        assert_eq!((hit_numbers(-5, 0, "{}-{}"), hit_numbers(-5, -5, "{}-{}"), hit_numbers(-5, -9, "({}-{})")), ("5%".to_string(), "5".to_string(), "(5-9)".to_string()));
    }
}
