//! The chat groups and the chat windows that show them, as GUI.dll's `ChatGUIModule_c` keeps them.
//!
//! **Groups.** Every line of chat belongs to a group. The client's own groups are made by FUN_10084436 (each a
//! `FUN_100841c7(id, name, ...)`): System (0x40000001), Vicinity (0x40000002, the only one of them that can be talked
//! to), Tell Messages (0x40000003), Your Pets and Other Pets (0x41000000 / 1), and the combat groups 0x42000001 on
//! ([`BUILTIN`]). The chat server's channels are groups too, known by their names. Gamecode's feedback formatter
//! (FUN_10012c3e) names the group with each line it makes: [`feedback_group`] is that table, by the text's name.
//!
//! **Windows.** A chat window (`ChatGroupWindow_c`, FUN_1009e379) shows the groups selected in it. Its config
//! (FUN_1009aecc, the character's `Chat/Windows/Window<n>/Config.xml`) keeps `selected_group_names`, `output_group`
//! (the group typed lines go to), `show_timestamps` (each line led by its time as `(%H:%M) `), `name` (the tab's
//! title; `window_name` is the frame the tabs share), `is_autosubscribe_window` (a group that turns up later is selected by itself) and more. With no saved window
//! the original makes two (FUN_100950df): "Default Window", which selects everything but Other Pets and the combat
//! groups 0x42000001-0a, 0d-18 and 1b, and "Combat", which selects the combat groups 1-3, 5-9, 0d-12 and 14-18.
//!
//! Here the windows are the tabs of the client's one chat window ([`Pane`]). They start as the character's own from
//! the original client's folder when there are some, else as the original's two, and are kept in `chatwindows.ini`
//! beside `mao.ini`.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Mutex;

pub const SYSTEM: &str = "System";
pub const VICINITY: &str = "Vicinity";
pub const TELLS: &str = "Tell Messages";

/// The client's own groups: id and name (FUN_10084436).
pub const BUILTIN: &[(u32, &str)] = &[
    (0x4000_0001, SYSTEM),
    (0x4000_0002, VICINITY),
    (0x4000_0003, TELLS),
    (0x4100_0000, "Your Pets"),
    (0x4100_0001, "Other Pets"),
    (0x4200_0001, "Me hit by environment"),
    (0x4200_0002, "Me hit by nano"),
    (0x4200_0003, "Your pet hit by nano"),
    (0x4200_0004, "Other hit by nano"),
    (0x4200_0005, "You hit other with nano"),
    (0x4200_0006, "Me hit by monster"),
    (0x4200_0007, "Me hit by player"),
    (0x4200_0008, "You hit other"),
    (0x4200_0009, "Your pet hit by other"),
    (0x4200_000a, "Other hit by other"),
    (0x4200_000b, "Me got XP"),
    (0x4200_000c, "Me got SK"),
    (0x4200_0011, "Your pet hit by monster"),
    (0x4200_0012, "Your misses"),
    (0x4200_0013, "Other misses"),
    (0x4200_0014, "You gave health"),
    (0x4200_0015, "Me got health"),
    (0x4200_0016, "Me got nano"),
    (0x4200_0017, "You gave nano"),
    (0x4200_0018, "Me Cast Nano"),
    (0x4200_001a, "Team Loot Messages"),
    (0x4200_001b, "Vicinity Loot Messages"),
    (0x4200_001c, "Research"),
];

/// A built-in group's name by its id.
pub fn name_of(id: u32) -> Option<&'static str> {
    BUILTIN.iter().find(|g| g.0 == id).map(|g| g.1)
}

/// Whether a group is one of the combat messages (ids 0x42......: the menu's "Combat Messages").
pub fn is_combat(name: &str) -> bool {
    BUILTIN.iter().any(|g| g.1 == name && g.0 >> 24 == 0x42)
}

/// Whether lines can be typed to a group: of the client's own only Vicinity (the others are made read-only), and
/// the server's channels.
pub fn talkable(name: &str) -> bool {
    name == VICINITY || !BUILTIN.iter().any(|g| g.1 == name)
}

/// The group Gamecode's feedback formatter (FUN_10012c3e) tells a text in, by the text's name in text.mdb's category
/// 110; `None` for a text it gives no group of its own (the critical and glancing additions, the plain feedback).
/// Where the original picks between two groups by whether the one hit is the client's pet, or whether the attacker
/// is a player, the client does not know and the other's is taken.
pub fn feedback_group(name: &str) -> Option<u32> {
    Some(match name {
        // (0x1a: the client hurt by a nano, and its falls)
        "Feedback_FallDamage" | "Feedback_AttackedByForPointsOfDamage" | "Feedback_YouWereAttackedByNanobotsFrom" | "Feedback_AttackedByNanobotsForPointsOfDamage" => 0x4200_0002,
        // (0x1b / 0x1c: another hurt by a nano; 0x42000003 when it is the client's pet)
        "Feedback_TookPointsOfFallDamage" | "Feedback_WasAttackedByForPointsOfDamageFrom" | "Feedback_WasAttackedByForPointsOfDamage" | "Feedback_WasAttackedByNanobotsFrom" | "Feedback_WasAttackedByNanobots" => 0x4200_0004,
        // (0x40)
        "Feedback_YouHitOtherWith" | "Feedback_YouHitOtherByNanobots" => 0x4200_0005,
        // (0x1d; the shields 0x25 / 0x27 and the absorbing 0x34 when the client is the one hit)
        "Feedback_HitYouForPointsOfDamage" | "Feedback_WereHitForPointsOfDamage" | "Feedback_YouWereHitForPointsOfDamage" | "Feedback_SomeonesDamageShieldHitYou" | "Feedback_SomeonesReflectShieldHitYou" | "Feedback_YouAbsorbedDamage" => 0x4200_0006,
        // (0x1e)
        "Feedback_PlayerHitYouForPointsOfDamage" | "Feedback_PlayerHitYou" => 0x4200_0007,
        // (0x1f; the client's own shields 0x26 / 0x28)
        "Feedback_HitWithSpecial" | "Feedback_YourDamageShieldHitOther" | "Feedback_YourReflectShieldHitOther" => 0x4200_0008,
        // (0x20 / 0x21, the others' shields and absorbing 0x35; 0x42000009 when the one hit is the client's pet)
        "Feedback_OtherHitOther" | "Feedback_SomethingHitOther" | "Feedback_OtherDamageShieldHitOther" | "Feedback_SomethingHitOtherWithDamageShield" | "Feedback_OtherReflectShieldHitOther" | "Feedback_SomethingHitOtherWithReflectShield" | "Feedback_SomeoneAbsorbedDamage" => 0x4200_000a,
        // (0x24 and 0x45)
        "Feedback_LostXP" | "Feedback_ReceivedXP" | "Feedback_GainedAlienXP" => 0x4200_000b,
        // (0x3e)
        "Feedback_LostSK" | "Feedback_GainedSK" => 0x4200_000c,
        // (0x3b)
        "TryToAttackWithSpecialButMiss" | "Feedback_YouTriedToHitMissed" => 0x4200_0012,
        "OtherTriesToAttackWithSpecialButMisses" | "Feedback_OtherTriedToHitMissed" => 0x4200_0013,
        // (0x22)
        "Feedback_HealedForPoints" => 0x4200_0015,
        // (0x47)
        "Feedback_ExecutingNanoProgram" => 0x4200_0018,
        _ => return None,
    })
}

/// The chat server's channels heard of this session (their names), for the menus.
static SERVER_GROUPS: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Notes a channel of the chat server.
pub fn note(name: &str) {
    if !name.is_empty() {
        SERVER_GROUPS.lock().unwrap().insert(name.to_string());
    }
}

pub fn server_groups() -> Vec<String> {
    SERVER_GROUPS.lock().unwrap().iter().cloned().collect()
}

/// A chat window: here a tab of the chat window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pane {
    pub name: String,
    /// The groups shown (`selected_group_names`).
    pub selected: BTreeSet<String>,
    /// The groups this window has met: one not met yet is selected by itself when `auto` is on.
    pub known: BTreeSet<String>,
    /// `is_autosubscribe_window`.
    pub auto: bool,
    /// `output_group`: the group plain typed lines go to (empty: the vicinity).
    pub output: String,
    /// `show_timestamps`.
    pub timestamps: bool,
}

impl Pane {
    /// A window that selects every built-in group but those named.
    fn all_but(name: &str, without: &[u32]) -> Pane {
        let known: BTreeSet<String> = BUILTIN.iter().map(|g| g.1.to_string()).collect();
        let selected = BUILTIN.iter().filter(|g| !without.contains(&g.0)).map(|g| g.1.to_string()).collect();
        Pane { name: name.to_string(), selected, known, auto: true, output: String::new(), timestamps: false }
    }

    /// A window that selects only the built-in groups named.
    fn only(name: &str, with: &[u32]) -> Pane {
        let known: BTreeSet<String> = BUILTIN.iter().map(|g| g.1.to_string()).collect();
        let selected = with.iter().filter_map(|id| name_of(*id)).map(str::to_string).collect();
        Pane { name: name.to_string(), selected, known, auto: false, output: String::new(), timestamps: false }
    }

    /// Whether the window shows a group's lines; a group it has not met is selected first when the window selects
    /// new ones by itself.
    pub fn shows(&mut self, group: &str) -> bool {
        if !self.known.contains(group) {
            self.known.insert(group.to_string());
            if self.auto {
                self.selected.insert(group.to_string());
            }
        }
        self.selected.contains(group)
    }

    /// Selects a group or takes it out (the menu's check mark).
    pub fn toggle(&mut self, group: &str) {
        self.known.insert(group.to_string());
        if !self.selected.remove(group) {
            self.selected.insert(group.to_string());
        }
    }
}

/// The two windows the original makes when none is saved (FUN_100950df).
pub fn defaults() -> Vec<Pane> {
    let combat = |ids: std::ops::RangeInclusive<u32>| ids.map(|n| 0x4200_0000 + n);
    let without: Vec<u32> = std::iter::once(0x4100_0001).chain(combat(0x01..=0x0a)).chain(combat(0x0d..=0x18)).chain(combat(0x1b..=0x1b)).collect();
    let with: Vec<u32> = combat(0x01..=0x03).chain(combat(0x05..=0x09)).chain(combat(0x0d..=0x12)).chain(combat(0x14..=0x18)).collect();
    vec![Pane::all_but("Default Window", &without), Pane::only("Combat", &with)]
}

/// An attribute's value in the config's XML: `<Kind name="name" value="...">`, quotes of a string taken off.
fn value<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let at = xml.find(&format!("name=\"{name}\""))?;
    let rest = &xml[at..];
    let rest = &rest[..rest.find("/>")?];
    let v = rest.split_once("value=")?.1.trim();
    let v = v.strip_prefix('\'').and_then(|v| v.split_once('\'')).or_else(|| v.strip_prefix('"').and_then(|v| v.split_once('"')))?.0;
    Some(v.trim_start_matches("&quot;").trim_end_matches("&quot;"))
}

/// The strings of an array of the config (`<Array name="..."> <String value='&quot;...&quot;' /> ...`).
fn strings(xml: &str, name: &str) -> Vec<String> {
    let Some(at) = xml.find(&format!("<Array name=\"{name}\"")) else { return Vec::new() };
    let body = &xml[at..];
    let body = &body[..body.find("</Array>").unwrap_or(body.len())];
    body.split("<String value=").skip(1).filter_map(|s| s.split_once("&quot;")?.1.split_once("&quot;").map(|v| v.0.replace("&amp;", "&"))).collect()
}

/// A window of the original's `Config.xml`: the pane, its place among the tabs and whether it is the startup one.
pub fn parse_config(xml: &str) -> Option<(Pane, i64, bool)> {
    // (`name` is the window's own, the tab's title; `window_name` the frame the tabs share, "Window1")
    let name = value(xml, "name").or_else(|| value(xml, "window_name"))?.to_string();
    let selected: BTreeSet<String> = strings(xml, "selected_group_names").into_iter().collect();
    let mut known: BTreeSet<String> = BUILTIN.iter().map(|g| g.1.to_string()).collect();
    known.extend(selected.iter().cloned());
    let flag = |n: &str| value(xml, n) == Some("true");
    let pane = Pane { name, selected, known, auto: flag("is_autosubscribe_window"), output: value(xml, "output_group").unwrap_or("").to_string(), timestamps: flag("show_timestamps") };
    Some((pane, value(xml, "tab_index").and_then(|v| v.parse().ok()).unwrap_or(0), flag("is_startup_window")))
}

/// The windows of an original client's character folder (`Chat/Windows/*/Config.xml`): the startup window first, then
/// by their place among the tabs and name.
pub fn from_original(prefs: &Path) -> Vec<Pane> {
    let mut found: Vec<(Pane, i64, bool)> = std::fs::read_dir(prefs.join("Chat/Windows"))
        .map(|d| d.flatten().filter_map(|e| parse_config(&std::fs::read_to_string(e.path().join("Config.xml")).ok()?)).collect())
        .unwrap_or_default();
    found.sort_by(|a, b| b.2.cmp(&a.2).then(a.1.cmp(&b.1)).then(a.0.name.cmp(&b.0.name)));
    found.into_iter().map(|f| f.0).collect()
}

/// `chatwindows.ini`: a window a block: `window <name>`, `auto 0|1`, `timestamps 0|1`, `output <group>`, then a line
/// a group it has met, `+ <group>` shown or `- <group>` not.
pub fn render(panes: &[Pane]) -> String {
    let mut out = String::new();
    for p in panes {
        out += &format!("window {}\nauto {}\ntimestamps {}\noutput {}\n", p.name, u8::from(p.auto), u8::from(p.timestamps), p.output);
        for g in &p.known {
            out += &format!("{} {g}\n", if p.selected.contains(g) { '+' } else { '-' });
        }
    }
    out
}

pub fn parse(text: &str) -> Vec<Pane> {
    let mut panes: Vec<Pane> = Vec::new();
    for line in text.lines() {
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim();
        if word == "window" && !rest.is_empty() {
            panes.push(Pane { name: rest.to_string(), selected: BTreeSet::new(), known: BTreeSet::new(), auto: false, output: String::new(), timestamps: false });
            continue;
        }
        let Some(p) = panes.last_mut() else { continue };
        match word {
            "auto" => p.auto = rest == "1",
            "timestamps" => p.timestamps = rest == "1",
            "output" => p.output = rest.to_string(),
            "+" | "-" if !rest.is_empty() => {
                p.known.insert(rest.to_string());
                if word == "+" {
                    p.selected.insert(rest.to_string());
                }
            }
            _ => {}
        }
    }
    panes
}

fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("chatwindows.ini")
}

/// Whether the player's file is left alone (previews and tests).
fn detached() -> bool {
    cfg!(test) || std::env::var_os("MAO_UI_PREVIEW").is_some()
}

/// The windows to start with: the player's own (`chatwindows.ini`), else the character's of the original client
/// (`prefs`, its folder there), else the original's two.
pub fn load(prefs: Option<&Path>) -> Vec<Pane> {
    if !detached()
        && let Ok(text) = std::fs::read_to_string(path())
    {
        let panes = parse(&text);
        if !panes.is_empty() {
            return panes;
        }
    }
    let own = prefs.filter(|_| !detached()).map(from_original).unwrap_or_default();
    if own.is_empty() { defaults() } else { own }
}

pub fn save(panes: &[Pane]) {
    if !detached() {
        let _ = std::fs::write(path(), render(panes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_windows_are_the_originals() {
        let panes = defaults();
        assert_eq!(panes.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Default Window", "Combat"]);
        let (mut default, mut combat) = (panes[0].clone(), panes[1].clone());
        // (the default window: talk, tells, the system's lines, the experience; not the fight)
        for g in [SYSTEM, VICINITY, TELLS, "Your Pets", "Me got XP", "Me got SK", "Research", "Team Loot Messages"] {
            assert!(default.shows(g), "{g}");
        }
        for g in ["Other Pets", "Me hit by monster", "You hit other", "Other hit by other", "Your misses", "Me Cast Nano", "Vicinity Loot Messages"] {
            assert!(!default.shows(g), "{g}");
        }
        // (the combat window: the client's own fight, not the others' between themselves)
        for g in ["Me hit by monster", "You hit other", "Your misses", "Me got health", "Me Cast Nano", "Me hit by environment"] {
            assert!(combat.shows(g), "{g}");
        }
        for g in [SYSTEM, VICINITY, "Other hit by nano", "Other hit by other", "Other misses", "Me got XP"] {
            assert!(!combat.shows(g), "{g}");
        }
        // (a channel that turns up is shown by the window that selects new ones by itself)
        assert!(default.shows("Clan OOC") && !combat.shows("Clan OOC"));
        default.toggle("Clan OOC");
        assert!(!default.shows("Clan OOC"));
    }

    #[test]
    fn the_originals_config_is_read() {
        let xml = r#"<Archive code="0">
    <Array name="selected_group_ids">
        <Int64 value="1107296262" />
    </Array>
    <Array name="selected_group_names">
        <String value='&quot;Me hit by monster&quot;' />
        <String value='&quot;Clan OOC&quot;' />
    </Array>
    <Archive code="0" name="chat_window_config" />
    <String name="output_group" value='&quot;Clan OOC&quot;' />
    <Bool name="show_timestamps" value="true" />
    <Bool name="is_autosubscribe_window" value="false" />
    <Int32 name="tab_index" value="2" />
    <String name="window_name" value='&quot;Window1&quot;' />
    <String name="name" value='&quot;Fights&quot;' />
    <Bool name="is_startup_window" value="true" />
</Archive>"#;
        let (mut pane, tab, startup) = parse_config(xml).unwrap();
        assert_eq!((pane.name.as_str(), pane.output.as_str(), pane.timestamps, pane.auto, tab, startup), ("Fights", "Clan OOC", true, false, 2, true));
        assert!(pane.shows("Me hit by monster") && pane.shows("Clan OOC") && !pane.shows(SYSTEM) && !pane.shows("Neu. OOC"));
    }

    #[test]
    fn the_windows_are_kept_in_a_file() {
        let mut panes = defaults();
        panes[0].output = "Clan OOC".into();
        panes[0].timestamps = true;
        panes[0].toggle("Clan OOC");
        panes[0].toggle("OT OOC");
        panes[0].toggle("OT OOC");
        assert_eq!(parse(&render(&panes)), panes);
        assert!(parse("junk\n+ orphan\n").is_empty());
    }

    #[test]
    fn the_fight_lines_have_their_groups() {
        assert_eq!(feedback_group("Feedback_HitYouForPointsOfDamage").and_then(name_of), Some("Me hit by monster"));
        assert_eq!(feedback_group("Feedback_PlayerHitYouForPointsOfDamage").and_then(name_of), Some("Me hit by player"));
        assert_eq!(feedback_group("Feedback_HitWithSpecial").and_then(name_of), Some("You hit other"));
        assert_eq!(feedback_group("Feedback_YouTriedToHitMissed").and_then(name_of), Some("Your misses"));
        assert_eq!(feedback_group("Feedback_ReceivedXP").and_then(name_of), Some("Me got XP"));
        assert_eq!(feedback_group("Feedback_ExecutingNanoProgram").and_then(name_of), Some("Me Cast Nano"));
        assert_eq!(feedback_group("Feedback_CriticalHit"), None);
        assert!(is_combat("Me hit by monster") && !is_combat(VICINITY) && !is_combat("Clan OOC"));
        assert!(talkable(VICINITY) && talkable("Clan OOC") && !talkable(SYSTEM) && !talkable("Me got XP"));
    }
}
