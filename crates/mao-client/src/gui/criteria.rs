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
//! Not ported: `ConvertCriteria`'s special criteria (items, perks, pets, places, flags lists and the like: they are
//! written as a plain stat criterion here), the breed and gender names, and `N3Msg_CriteriaListToString`'s reordering of
//! apply-on criteria.

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

/// `ConvertCriteria` (0x1001e2cf), for the operators and the stat criteria.
pub fn convert(c: &Criterion) -> String {
    let (stat, value, op) = *c;
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
    if stat == 33 {
        // (a side: its name, category 2005; "Not " before an unequal one's)
        let mut s = ldb(2005, value).unwrap_or_default();
        if op == 0x18 {
            s.insert_str(0, &ldb_named(1005, "OperatorNotOf"));
        }
        return s;
    }
    let mut s = stat_name(stat);
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

/// `N3Msg_CriteriaListToString` (0x10020bf0): the heading, then each criterion group, a line each.
pub fn list_to_string(list: &[Criterion]) -> String {
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

    #[test]
    fn indent_marks_the_lines_of_a_group() {
        let mut s = "  a\n  b".to_string();
        text_indent(&mut s, "  ");
        assert_eq!(s, "  . a\n  . b");
    }
}
