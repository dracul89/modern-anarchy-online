//! The priorities behind the Skills window's "Suggested IP distribution": the game's `cd_image/data/ipdist.xml`, read as
//! GUI.dll's `RecommendIPUse` reads it (FUN_100f9787, FUN_100f982b).
//!
//! The file lists professions by name (text.mdb 2004's), under each its stats by name (text.mdb 2003's), under each
//! level ranges with a priority: `<profession name=".."><stat name=".."><levelrange min=".." max=".." pri=".."/>`. A
//! stat's priority for a character is that of the first range its level is in, of the first stat of that name under
//! the first profession of that name; nothing when any of them is missing. The shipped file covers levels 1 to 20.

/// A profession's stats, each with its level ranges (least level, most level, priority).
type Stats = Vec<(String, Vec<(i64, i64, i32)>)>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Priorities {
    professions: Vec<(String, Stats)>,
}

/// An attribute's value in an element's text (`name="value"`).
fn attribute<'a>(element: &'a str, name: &str) -> Option<&'a str> {
    let at = element.find(&format!("{name}=\""))? + name.len() + 2;
    element[at..].split('"').next()
}

pub fn parse(xml: &str) -> Priorities {
    let mut out = Priorities::default();
    for element in xml.split('<').skip(1) {
        let element = element.split('>').next().unwrap_or("");
        if element.starts_with("profession") {
            out.professions.push((attribute(element, "name").unwrap_or("").to_string(), Vec::new()));
        } else if element.starts_with("stat") {
            if let Some(p) = out.professions.last_mut() {
                p.1.push((attribute(element, "name").unwrap_or("").to_string(), Vec::new()));
            }
        } else if element.starts_with("levelrange") {
            let number = |name: &str| attribute(element, name).and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0);
            if let Some(s) = out.professions.last_mut().and_then(|p| p.1.last_mut()) {
                s.1.push((number("min"), number("max"), number("pri") as i32));
            }
        }
    }
    out
}

impl Priorities {
    /// FUN_100f982b: a stat's priority for a profession at a level; 0 for none.
    pub fn priority(&self, profession: &str, stat: &str, level: i64) -> i32 {
        let Some(stats) = self.professions.iter().find(|p| p.0 == profession).map(|p| &p.1) else { return 0 };
        let Some(ranges) = stats.iter().find(|s| s.0 == stat).map(|s| &s.1) else { return 0 };
        ranges.iter().find(|r| r.0 <= level && level <= r.1).map_or(0, |r| r.2)
    }
}

/// The game's file, read once its folder is known.
pub fn priorities() -> Option<&'static Priorities> {
    static FILE: std::sync::OnceLock<Priorities> = std::sync::OnceLock::new();
    if let Some(p) = FILE.get() {
        return Some(p);
    }
    let text = mao_formats::files::read_to_string(crate::theme::game_dir()?.join("cd_image/data/ipdist.xml")).ok()?;
    Some(FILE.get_or_init(|| parse(&text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stat_has_the_priority_of_its_level_range() {
        let p = parse(
            r#"<profession name="Fixer">
  <stat name="Agility">
     <levelrange min="1" max="20" pri="1"/>
     <levelrange min="21" max="40" pri="2"/>
  </stat>
  <stat name="Body Dev.">
     <levelrange min="1" max="20" pri="3"/>
  </stat>
</profession>
<profession name="Meta-Physicist">
  <stat name="Agility">
     <levelrange min="1" max="20" pri="0"/>
  </stat>
</profession>"#,
        );
        assert_eq!([p.priority("Fixer", "Agility", 1), p.priority("Fixer", "Agility", 20), p.priority("Fixer", "Agility", 21), p.priority("Fixer", "Agility", 41)], [1, 1, 2, 0]);
        assert_eq!([p.priority("Fixer", "Body Dev.", 5), p.priority("Meta-Physicist", "Agility", 5), p.priority("Soldier", "Agility", 5), p.priority("Fixer", "Sense", 5)], [3, 0, 0, 0]);
    }

    /// The game's own file names every profession's stats as text.mdb does.
    #[test]
    fn the_games_file_is_read() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let text = std::fs::read_to_string(std::path::Path::new(&game).join("cd_image/data/ipdist.xml")).unwrap();
        let p = parse(&text);
        assert_eq!(p.professions.len(), 14);
        assert!(p.professions.iter().all(|x| x.1.len() == 75));
        assert!(p.priority("Adventurer", "Agility", 10) > 0 && p.priority("Adventurer", "Agility", 21) == 0);
    }
}
