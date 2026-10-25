//! What a character looked like the last time it was played: its clothes' textures and the meshes it wore (hair, weapons, shoulder
//! pads...). The server's character list carries none of it, so the live client writes it down (`looks.ini` beside `mao.ini`, one
//! line per character) and the character screen dresses the figure from it, as the original shows gear only after a first login.

use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Look {
    pub name: String,
    pub breed: u8,
    pub sex: u8,
    /// The worn clothes' textures by body part, and the texture overrides by material name.
    pub clothes: Vec<(String, u32)>,
    pub textures: Vec<(String, u32)>,
    /// The worn meshes: attachment position, static mesh instance and replacement texture instance.
    pub gear: Vec<(u8, u32, u32)>,
}

fn path() -> PathBuf {
    crate::settings::Settings::path().with_file_name("looks.ini")
}

fn pairs(list: &[(String, u32)]) -> String {
    list.iter().map(|(n, i)| format!("{n}={i}")).collect::<Vec<_>>().join(",")
}

fn parse_pairs(text: &str) -> Vec<(String, u32)> {
    text.split(',').filter_map(|p| p.split_once('=')).filter_map(|(n, i)| Some((n.to_string(), i.parse().ok()?))).collect()
}

/// A line of the file: `name|breed|sex|clothes|textures|gear` (`part=instance,...`; gear is `position:mesh:texture` in hex).
pub fn render(look: &Look) -> String {
    let gear = look.gear.iter().map(|(p, m, t)| format!("{p}:{m:x}:{t:x}")).collect::<Vec<_>>().join(",");
    format!("{}|{}|{}|{}|{}|{gear}", look.name, look.breed, look.sex, pairs(&look.clothes), pairs(&look.textures))
}

pub fn parse_line(line: &str) -> Option<Look> {
    let f: Vec<&str> = line.split('|').collect();
    if f.len() != 6 || f[0].is_empty() {
        return None;
    }
    let gear = f[5]
        .split(',')
        .filter_map(|g| {
            let mut w = g.split(':');
            Some((w.next()?.parse().ok()?, u32::from_str_radix(w.next()?, 16).ok()?, u32::from_str_radix(w.next()?, 16).ok()?))
        })
        .collect();
    Some(Look { name: f[0].to_string(), breed: f[1].parse().ok()?, sex: f[2].parse().ok()?, clothes: parse_pairs(f[3]), textures: parse_pairs(f[4]), gear })
}

pub fn load() -> Vec<Look> {
    std::fs::read_to_string(path()).map(|t| t.lines().filter_map(parse_line).collect()).unwrap_or_default()
}

/// The remembered look of a character (names compare without regard to case).
pub fn find(looks: &[Look], name: &str) -> Option<Look> {
    looks.iter().find(|l| l.name.eq_ignore_ascii_case(name)).cloned()
}

/// Writes down a character's look, replacing its earlier line.
pub fn save(look: &Look) {
    if cfg!(test) || std::env::var_os("MAO_NO_LAYOUT").is_some() {
        return;
    }
    let mut looks = load();
    looks.retain(|l| !l.name.eq_ignore_ascii_case(&look.name));
    looks.push(look.clone());
    let _ = std::fs::write(path(), looks.iter().map(|l| render(l) + "\n").collect::<String>());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_round_trips_through_its_line() {
        let look = Look {
            name: "Brasscasings".into(),
            breed: 1,
            sex: 2,
            clothes: vec![("body".into(), 303241), ("legs".into(), 303245)],
            textures: vec![("skin".into(), 8816)],
            gear: vec![(0, 0x4e39, 0), (3, 0x2cf7, 0x4a0b1)],
        };
        let line = render(&look);
        assert_eq!(line, "Brasscasings|1|2|body=303241,legs=303245|skin=8816|0:4e39:0,3:2cf7:4a0b1");
        assert_eq!(parse_line(&line), Some(look));
        assert_eq!(parse_line("name|1|2|||"), Some(Look { name: "name".into(), breed: 1, sex: 2, ..Default::default() }));
        assert_eq!(parse_line("broken"), None);
    }

    #[test]
    fn a_look_is_found_by_name_in_any_case() {
        let looks = vec![Look { name: "Mothra".into(), ..Default::default() }];
        assert!(find(&looks, "mothra").is_some());
        assert!(find(&looks, "other").is_none());
    }
}
