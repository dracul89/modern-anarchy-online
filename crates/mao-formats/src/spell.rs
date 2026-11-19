//! Spells as GameData.dll stores them (`SpellFormats_c`, the `gamedata` drop-in's `spell.rs` and its format table, copied
//! here as `spell_formats.rs`): a header (type, instance, version), the criteria (a count, then `(stat, value, operator)`
//! triples), then the values of the type's format: four every format starts with (spell stats 3 "hits", 4 "delay", 0x20
//! "target" and 0x23) and the format's own, each a 32-bit int or (complex type 1) a string (its length, then its bytes).
//! The texture spell (0xcf2f) has a short form of its own (`TextureSpellFormat_c`); 0xcf17 and 0xcf20 carry more after
//! their values (`SpellData_t::PostBinarySpellRead`, 0x1000d3ba: a second list of criteria, and an expression). A type
//! with no format of its own has the default one: the four common values alone.
//!
//! An item template keeps its spell lists as `2, <list>, (n + 1) × 1009` and then the list's n spells (list 14: worn,
//! 0: used ...).

#[derive(Clone, Copy)]
enum Kind {
    Plain,
    Texture,
}

use Kind::{Plain, Texture};

#[derive(Clone, Copy)]
enum ArgDef {
    /// Complex type, spell stat, default.
    I(i32, i32, i32),
    /// Complex type, string index.
    S(i32, u32),
}

use ArgDef::{I, S};

#[allow(dead_code)]
struct FormatDef {
    kind: Kind,
    custom_data: bool,
    fixed: bool,
    args: &'static [ArgDef],
    types: &'static [i32],
}

include!("spell_formats.rs");

/// A spell value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Int(i32),
    Text(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Spell {
    pub kind: u32,
    pub instance: u32,
    pub criteria: Vec<(u32, i32, u32)>,
    /// The second list of criteria of a spell of type 0xcf17.
    pub special: Vec<(u32, i32, u32)>,
    /// (spell stat, value) for an int argument; strings by index in `strings`.
    pub stats: Vec<(i32, i32)>,
    pub strings: Vec<(u32, String)>,
}

impl Spell {
    /// The value of a spell stat (`GetSpellStat`), 0 when the spell has none.
    pub fn stat(&self, stat: i32) -> i32 {
        self.stats.iter().find(|s| s.0 == stat).map_or(0, |s| s.1)
    }

    pub fn string(&self, index: u32) -> &str {
        self.strings.iter().find(|s| s.0 == index).map_or("", |s| s.1.as_str())
    }
}

/// The format of a spell type: its arguments after the four common ones. One not in the table has none
/// (`SpellFormats_c`'s default format); `None` for a texture format other than the texture spell's.
fn format(kind: u32) -> Option<&'static [ArgDef]> {
    let Some(f) = FORMATS.iter().find(|f| f.types.contains(&(kind as i32))) else { return Some(&[]) };
    match f.kind {
        Plain => Some(f.args),
        Texture => None,
    }
}

/// Where an expression that starts at `at` ends (`ExpressionData_t::CreateFromStream`): a node's type, then 1 a float, 2 a
/// stat and a flag, 3 and 4 an operator and two nodes more.
fn expression_end(data: &[u8], at: usize, big_endian: bool, depth: u32) -> Option<usize> {
    let bytes = data.get(at..at + 4)?;
    let kind = if big_endian { u32::from_be_bytes(bytes.try_into().ok()?) } else { u32::from_le_bytes(bytes.try_into().ok()?) };
    match kind {
        1 => data.get(at + 4..at + 8).map(|_| at + 8),
        2 => data.get(at + 4..at + 12).map(|_| at + 12),
        3 | 4 if depth < 64 => {
            data.get(at + 4..at + 8)?;
            let right = expression_end(data, at + 8, big_endian, depth + 1)?;
            expression_end(data, right, big_endian, depth + 1)
        }
        _ => None,
    }
}

const COMMON: [i32; 4] = [3, 4, 0x20, 0x23];

/// A spell at `at` (`SpellFormats_c::ReadBinary`), and where it ends.
pub fn read(data: &[u8], at: usize) -> Option<(Spell, usize)> {
    read_order(data, at, false)
}

/// A spell as the server sends it in a message (`GameData::operator>>` on a `SpellData_t`, as `SpellListIIR_t`'s FUN_100a71ae
/// reads each): the same values in the stream's big-endian order.
pub fn read_network(data: &[u8], at: usize) -> Option<(Spell, usize)> {
    read_order(data, at, true)
}

fn read_order(data: &[u8], at: usize, big_endian: bool) -> Option<(Spell, usize)> {
    let word = |o: usize| data.get(o..o + 4).map(|b| if big_endian { u32::from_be_bytes(b.try_into().unwrap()) } else { u32::from_le_bytes(b.try_into().unwrap()) });
    let (kind, instance, version, count) = (word(at)?, word(at + 4)?, word(at + 8)?, word(at + 12)? as usize);
    let texture = kind == 0xcf2f;
    let args = if texture { &[][..] } else { format(kind)? };
    if version != 4 || count > 64 {
        return None;
    }
    let mut p = at + 16;
    let mut spell = Spell { kind, instance, ..Default::default() };
    for _ in 0..count {
        spell.criteria.push((word(p)?, word(p + 4)? as i32, word(p + 8)?));
        p += 12;
    }
    for stat in COMMON {
        spell.stats.push((stat, word(p)? as i32));
        p += 4;
    }
    if texture {
        // (`TextureSpellFormat_c::ReadBinary`: the texture and a marker; below 100 the marker is the first value of a
        // short record and only the second follows, else the four follow; kept as their order, from spell stat 1000)
        let (tex, marker) = (word(p)? as i32, word(p + 4)? as i32);
        p += 8;
        spell.stats.push((1000, tex));
        let n = if marker < 100 { 1 } else { 4 };
        spell.stats.push((1001, marker));
        for k in 0..n {
            spell.stats.push((1002 + k, word(p)? as i32));
            p += 4;
        }
        return Some((spell, p));
    }
    for a in args {
        match *a {
            I(1, _, _) | S(1, _) => {
                let index = match *a {
                    S(_, i) => i,
                    _ => 0,
                };
                let len = word(p)? as usize;
                if len > 4096 {
                    return None;
                }
                let bytes = data.get(p + 4..p + 4 + len)?;
                let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                spell.strings.push((index, String::from_utf8_lossy(&bytes[..end]).into_owned()));
                p += 4 + len;
            }
            I(_, stat, _) => {
                spell.stats.push((stat, word(p)? as i32));
                p += 4;
            }
            S(..) => return None,
        }
    }
    // (`SpellData_t::PostBinarySpellRead`)
    if kind == 0xcf17 {
        let prefix = word(p)?;
        if prefix == 0 || prefix % 1009 != 0 || prefix / 1009 > 65 {
            return None;
        }
        p += 4;
        for _ in 1..prefix / 1009 {
            spell.special.push((word(p)?, word(p + 4)? as i32, word(p + 8)?));
            p += 12;
        }
    } else if kind == 0xcf20 {
        p = expression_end(data, p, big_endian, 0)?;
    }
    Some((spell, p))
}

/// An item template's spell lists: `(list, spells)` in their order. A list whose spells do not all read is left out.
pub fn lists(template: &[u8]) -> Vec<(u32, Vec<Spell>)> {
    let word = |o: usize| template.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let mut out = Vec::new();
    let mut at = 0;
    while at + 12 <= template.len() {
        let (key, list, count) = (word(at), word(at + 4), word(at + 8));
        if key == Some(2) && list.is_some_and(|l| l < 64) && count.is_some_and(|c| c % 1009 == 0 && (2..=101).contains(&(c / 1009))) {
            let n = (count.unwrap() / 1009 - 1) as usize;
            let mut p = at + 12;
            let mut spells = Vec::new();
            for _ in 0..n {
                match read(template, p) {
                    Some((s, end)) => {
                        spells.push(s);
                        p = end;
                    }
                    None => break,
                }
            }
            if spells.len() == n {
                out.push((list.unwrap(), spells));
                at = p;
                continue;
            }
        }
        at += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words of a list of spells, little-endian.
    fn bytes(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// A type with no format has the four common values alone; 0xcf17 and 0xcf20 carry more after theirs, and a list
    /// with one of them in it is read to its end.
    #[test]
    fn the_spells_that_used_to_end_a_list() {
        const MODIFY: [u32; 10] = [0xcf35, 0, 4, 0, 1, 0, 2, 9, 16, 7];
        let unknown = [0xcf78, 0, 4, 0, 1, 0, 2, 9];
        let (mut special, args) = (vec![0xcf17, 0, 4, 0, 1, 0, 2, 9], format(0xcf17).expect("a format").len());
        special.extend(std::iter::repeat_n(0, args));
        special.extend([3 * 1009, 54, 10, 2, 60, 6, 0]);
        let (mut expression, args) = (vec![0xcf20, 0, 4, 0, 1, 0, 2, 9], format(0xcf20).expect("a format").len());
        expression.extend(std::iter::repeat_n(0, args));
        // (a compound of a float and a function of a stat and a float)
        expression.extend([3, 1, 1, 0x3f80_0000, 4, 2, 2, 54, 0, 1, 0x4000_0000]);
        let mut words = vec![2, 14, 5 * 1009];
        words.extend(unknown);
        words.extend(special);
        words.extend(expression);
        words.extend(MODIFY);
        let found = lists(&bytes(&words));
        assert_eq!(found.len(), 1, "{found:?}");
        let spells = &found[0].1;
        assert_eq!(spells.iter().map(|s| s.kind).collect::<Vec<_>>(), [0xcf78, 0xcf17, 0xcf20, 0xcf35]);
        assert_eq!(spells[1].special, [(54, 10, 2), (60, 6, 0)]);
        assert_eq!((spells[3].stat(0), spells[3].stat(39)), (16, 7));
    }

    #[test]
    fn reads_a_list_of_modify_spells() {
        // 2, list 14, 2 spells: modify stat 0x5e by 0x333; modify 0x61 by 0x333 when level above 10
        let words: Vec<u32> = vec![2, 14, 3 * 1009, 0xcf35, 0, 4, 0, 1, 0, 2, 9, 0x5e, 0x333, 0xcf35, 0, 4, 1, 54, 10, 2, 1, 0, 2, 9, 0x61, 0x333];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let l = lists(&bytes);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].0, 14);
        assert_eq!(l[0].1.len(), 2);
        assert_eq!(l[0].1[0].stat(0), 0x5e);
        assert_eq!(l[0].1[0].stat(39), 0x333);
        assert_eq!(l[0].1[0].stat(0x20), 2);
        assert_eq!(l[0].1[1].criteria, vec![(54, 10, 2)]);
    }
}
