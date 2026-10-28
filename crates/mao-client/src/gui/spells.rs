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
//! skill's base (0xcfc0), a nano program upload (0xcf1b), a nano cast (0xcf3b "Cast "), taunt, and the fixed texts of
//! 0xcf48, 0xcf4c, 0xcf7e, 0xcf81, 0xcf82, 0xcf86, 0xcf87 and 0xcfaf.

use mao_formats::spell::Spell;

fn text(category: i32, name: &str) -> String {
    crate::feedback::texts().and_then(|t| t.named(category, name)).map(str::to_string).unwrap_or_default()
}

/// A C `printf` of `%s`, `%d`, `%u` and `%%` with the arguments in order.
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
            Some(_) => {
                if let Some(a) = it.next() {
                    out += a;
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

/// One spell's text (the switch of `N3Msg_ConvertSpellList`); `None` for a type it does not write.
pub fn spell_text(s: &Spell, level: u32, nano_name: &dyn Fn(u32) -> Option<String>) -> Option<String> {
    let stat_name = |k: i32| super::criteria::stat_name(k as u32);
    let (a0, a39) = (s.stat(0), s.stat(39));
    Some(match s.kind {
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
        0xcff5 if a39 != 0 => format!("Modify {} {}%", stat_name(a0), a39),
        0xcfb7 => {
            let scaled = (level as i64 & 0xffff) * a39 as i64 / 200;
            if a0 == 0x168 {
                printf(&text(1005, "ScaleAt200"), &[level.to_string(), (scaled + 100).to_string(), (a39 + 200).to_string(), (a39 + 100).to_string()])
            } else {
                printf(&text(1005, "SkillAt200"), &[stat_name(a0), level.to_string(), scaled.to_string(), a39.to_string()])
            }
        }
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
        0xcf1b => printf(&text(1005, "UploadNanoprogram"), &[nano_name(a39 as u32)?]),
        0xcf3b => format!("Cast {}", nano_name(a39 as u32).unwrap_or_default()),
        0xcf7d => format!("Taunt {}", s.stat(0)),
        0xcf48 => "Spawn Item".into(),
        0xcf4c if s.stat(0) & 4 != 0 => "Root".into(),
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
pub fn convert(spells: &[Spell], level: u32, nano_name: &dyn Fn(u32) -> Option<String>) -> String {
    let mut out = String::new();
    let mut last_target = -1;
    for s in spells {
        let Some(line) = spell_text(s, level, nano_name) else { continue };
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

    #[test]
    fn a_modify_spell() {
        let s = Spell { kind: 0xcf35, stats: vec![(3, 1), (4, 0), (0x20, 2), (0x23, 9), (0, 16), (39, 20)], ..Default::default() };
        let t = spell_text(&s, 200, &|_| None).unwrap();
        assert!(t.starts_with("Modify ") && t.ends_with(" 20"));
        let line = format_modifier("On User:", "", &t, "");
        assert!(line.starts_with("On User:\n  Modify "));
    }
}
