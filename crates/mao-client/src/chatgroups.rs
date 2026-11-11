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
//! Chat windows that share a frame are its tabs: a window's config names its frame (`window_name`, "Window1") and its
//! place among the frame's tabs (`tab_index`); the original's two start in one frame, and so do the player's own in
//! their files. Here a window is a [`Pane`]; the client's chat window shows the panes of the first one's frame as its
//! tabs and each other frame in a window of its own. They start as the character's own from the original client's
//! folder when there are some, else as the original's two, and are kept in `chatwindows.ini` beside `mao.ini`.

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

/// Whether a group is one of the combat messages (FUN_1008537d: the client's groups from 0x42000000 to under
/// 0x4200001a; the loot messages and the research count with the public ones), the menu's and the chat
/// configuration's "Combat Messages".
pub fn is_combat(name: &str) -> bool {
    BUILTIN.iter().any(|g| g.1 == name && (0x4200_0000..0x4200_001a).contains(&g.0))
}

/// Whether a group is a private chat group (FUN_1008537d: a channel of kind 14).
pub fn is_private(name: &str) -> bool {
    SERVER_GROUPS.lock().unwrap().get(name).map(|g| g.0) == Some(0x0e)
}

/// Whether a group is there now: the client's own always, a channel once the chat server has told of it.
pub fn available(name: &str) -> bool {
    BUILTIN.iter().any(|g| g.1 == name) || SERVER_GROUPS.lock().unwrap().contains_key(name)
}

/// The name in TextColors.xml of the colour a group's lines are in (FUN_1008582a): the client's own groups by their
/// number (the fight's lines white there: their texts bring their own colours), a channel by its kind.
pub fn colour_name(name: &str) -> &'static str {
    match BUILTIN.iter().find(|g| g.1 == name).map(|g| g.0) {
        Some(0x4000_0001) => "ct_system",
        Some(0x4000_0002) => "ctch_vicinity",
        Some(0x4000_0003) => "ctch_tell",
        Some(0x4100_0000) => "ctch_mypet",
        Some(0x4100_0001) => "ctch_otherpet",
        Some(0x4200_001c) => "ctch_research",
        Some(_) => "white",
        None => match SERVER_GROUPS.lock().unwrap().get(name).map(|g| g.0) {
            Some(10) => "ctch_tower",
            Some(1) => "ctch_admin",
            Some(3) => "ctch_clan",
            Some(4) => "ctch_misc",
            Some(5) => "ctch_gm",
            Some(8) => "ctch_news",
            Some(0x0e) => "ctch_pgroup",
            Some(0x82) => "ctch_team",
            Some(0x86) => "ctch_seekingteam",
            Some(0x87) => "ctch_newbie",
            Some(0x8f) => "ctch_raid",
            _ => "white",
        },
    }
}

/// Whether lines can be typed to a group: of the client's own only Vicinity (the others are made read-only), and
/// the server's channels.
pub fn talkable(name: &str) -> bool {
    name == VICINITY || !BUILTIN.iter().any(|g| g.1 == name)
}

/// Whether a group takes no lines (the group's flag bit 1): the client's own but Vicinity, and a channel the chat
/// server marks so (flags 0x8006 on the tower battle outcome against 0x8004 on a team's).
pub fn read_only(name: &str) -> bool {
    !talkable(name) || SERVER_GROUPS.lock().unwrap().get(name).is_some_and(|g| g.1 & 2 != 0)
}

/// The first channel of a kind the chat server has told of: the organization's (3), the team's (0x82).
pub fn of_kind(kind: u8) -> Option<String> {
    SERVER_GROUPS.lock().unwrap().iter().find(|(_, g)| g.0 == kind).map(|(name, _)| name.clone())
}

/// The groups a typed name stands for (FUN_10083df7): those whose names begin with it, in any case; one it names
/// in full stands alone. Quotes around it are dropped (FUN_1009c5b8).
pub fn matching(typed: &str) -> Vec<String> {
    let typed = typed.trim_matches('"');
    let mut out = Vec::new();
    for name in BUILTIN.iter().map(|g| g.1.to_string()).chain(server_groups()) {
        if name.get(..typed.len()).is_some_and(|start| start.eq_ignore_ascii_case(typed)) {
            if name.len() == typed.len() {
                return vec![name];
            }
            out.push(name);
        }
    }
    out
}

/// What a typed group name comes to in `/ch` and `/group`.
#[derive(Debug, PartialEq, Eq)]
pub enum Pick {
    /// No group begins so.
    Unknown,
    Group(String),
    /// `/ch`: the window does not show the group.
    NotShown,
    ReadOnly(String),
    /// More than one usable group begins so.
    Ambiguous,
    /// Several begin so and none can be used.
    NoneUsable,
}

/// `/ch <name>` (FUN_1009c5b8; `shown_only`: a window's output group is one it shows) and `/group <name> <text>`
/// (FUN_1009c977): the one group the name stands for. Of several, the read-only ones (and for `/ch` those not
/// shown) do not count.
pub fn pick(matches: &[String], shown_only: bool, shows: &dyn Fn(&str) -> bool, read_only: &dyn Fn(&str) -> bool) -> Pick {
    let single = |g: &String| {
        if shown_only && !shows(g) {
            Pick::NotShown
        } else if read_only(g) {
            Pick::ReadOnly(g.clone())
        } else {
            Pick::Group(g.clone())
        }
    };
    match matches {
        [] => Pick::Unknown,
        [one] => single(one),
        many => {
            let mut usable = many.iter().filter(|g| !read_only(g) && (!shown_only || shows(g)));
            match (usable.next(), usable.next()) {
                (Some(_), Some(_)) => Pick::Ambiguous,
                (Some(g), None) => single(g),
                _ => Pick::NoneUsable,
            }
        }
    }
}

/// The names an ambiguous one stands for, as the original lists them (FUN_1009b144): red a read-only group, white
/// one the window shows, silver another; the comma inside the colour, as there.
pub fn ambiguous_list(matches: &[String], shows: &dyn Fn(&str) -> bool, read_only: &dyn Fn(&str) -> bool) -> String {
    let mut out = String::new();
    for (n, name) in matches.iter().enumerate() {
        let colour = if read_only(name) {
            "red"
        } else if shows(name) {
            "white"
        } else {
            "silver"
        };
        out += &format!("<font color={colour}>{}{name}</font>", if n > 0 { ", " } else { "" });
    }
    out
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
        "Feedback_OtherHitOther" | "Feedback_MonsterHitWithSpecial" | "Feedback_SomethingHitOther" | "Feedback_OtherDamageShieldHitOther" | "Feedback_SomethingHitOtherWithDamageShield" | "Feedback_OtherReflectShieldHitOther" | "Feedback_SomethingHitOtherWithReflectShield" | "Feedback_SomeoneAbsorbedDamage" => 0x4200_000a,
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

/// The chat server's channels heard of this session: their names, kinds and flags.
static SERVER_GROUPS: Mutex<std::collections::BTreeMap<String, (u8, u32)>> = Mutex::new(std::collections::BTreeMap::new());

/// Notes a channel of the chat server, with its kind (the first byte of its identity).
pub fn note(name: &str, kind: u8) {
    note_channel(name, kind, 0);
}

/// Notes a channel with the flags the chat server tells it with.
pub fn note_channel(name: &str, kind: u8, flags: u32) {
    if !name.is_empty() {
        SERVER_GROUPS.lock().unwrap().insert(name.to_string(), (kind, flags));
    }
}

/// A channel gone (a private group left, the team's when the team is).
pub fn forget(name: &str) {
    SERVER_GROUPS.lock().unwrap().remove(name);
}

/// The invitations to private chat groups the chat window has not seen yet: the owner's id and name.
static INVITES: Mutex<Vec<(u32, String)>> = Mutex::new(Vec::new());

pub fn invite(owner: u32, name: &str) {
    INVITES.lock().unwrap().push((owner, name.to_string()));
}

pub fn take_invites() -> Vec<(u32, String)> {
    std::mem::take(&mut *INVITES.lock().unwrap())
}

/// The characters a tell window is to open for, as the chat server spells their names: the chat window takes them
/// (`ChatGUIModule_c::OpenTellWindow` 0x10086323 is called once the name's id is known).
static TELL_WINDOWS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn open_tell(name: &str) {
    TELL_WINDOWS.lock().unwrap().push(name.to_string());
}

pub fn take_tells() -> Vec<String> {
    std::mem::take(&mut *TELL_WINDOWS.lock().unwrap())
}

pub fn server_groups() -> Vec<String> {
    SERVER_GROUPS.lock().unwrap().keys().cloned().collect()
}

/// How a chat window looks and takes typing: what its Visual menu sets (FUN_1009a00d, FUN_10097756).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    /// `visual_mode` 2: no frame and title, only a background (0 is the normal window).
    pub borderless: bool,
    /// `window_transparency_active` and `window_transparency_inactive`, in hundredths: how solid the borderless
    /// window's background is while it is active, and otherwise.
    pub solid: (u8, u8),
    /// `is_textinput_enabled`: the window has an input bar.
    pub input: bool,
    /// `hide_input_when_inactive`: the input bar shows only while typing.
    pub hide_input: bool,
    /// `is_message_fading_enabled`: the lines fade away while the window is not active.
    pub fade: bool,
    /// The frame's "always on top" (1) and "always behind" (-1).
    pub layer: i8,
}

impl Default for Look {
    /// As the original's configs come (0.8 and 0.3 are in every one of the player's own files).
    fn default() -> Look {
        Look { borderless: false, solid: (80, 30), input: true, hide_input: false, fade: false, layer: 0 }
    }
}

/// A chat window: here a tab of the chat window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pane {
    pub name: String,
    /// The frame it is a tab of (`window_name`).
    pub frame: String,
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
    pub look: Look,
}

impl Pane {
    /// A window that selects every built-in group but those named.
    fn all_but(name: &str, without: &[u32]) -> Pane {
        let known: BTreeSet<String> = BUILTIN.iter().map(|g| g.1.to_string()).collect();
        let selected = BUILTIN.iter().filter(|g| !without.contains(&g.0)).map(|g| g.1.to_string()).collect();
        Pane { name: name.to_string(), frame: FIRST_FRAME.to_string(), selected, known, auto: true, output: String::new(), timestamps: false, look: Look::default() }
    }

    /// A window that selects only the built-in groups named.
    fn only(name: &str, with: &[u32]) -> Pane {
        let known: BTreeSet<String> = BUILTIN.iter().map(|g| g.1.to_string()).collect();
        let selected = with.iter().filter_map(|id| name_of(*id)).map(str::to_string).collect();
        Pane { name: name.to_string(), frame: FIRST_FRAME.to_string(), selected, known, auto: false, output: String::new(), timestamps: false, look: Look::default() }
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

/// The frame the original's first windows share.
pub const FIRST_FRAME: &str = "Window1";

/// A name no chat window or frame has yet (FUN_10094701: the first `Window%d` free).
pub fn free_name(panes: &[Pane]) -> String {
    (1..).map(|n| format!("Window{n}")).find(|n| !panes.iter().any(|p| p.name == *n || p.frame == *n)).unwrap_or_default()
}

/// The frames of the windows, in the order they first come; the first is the client's chat window.
pub fn frames(panes: &[Pane]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in panes {
        if !out.contains(&p.frame) {
            out.push(p.frame.clone());
        }
    }
    out
}

/// The windows of a frame, by their numbers.
pub fn members(panes: &[Pane], frame: &str) -> Vec<usize> {
    (0..panes.len()).filter(|k| panes[*k].frame == frame).collect()
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
/// The numbers of an `<Array name=...>` of `<Int64 value="...">`.
fn numbers(xml: &str, name: &str) -> Vec<i64> {
    let Some(at) = xml.find(&format!("<Array name=\"{name}\"")) else { return Vec::new() };
    let body = &xml[at..];
    let body = &body[..body.find("</Array>").unwrap_or(body.len())];
    body.split("<Int64 value=\"").skip(1).filter_map(|s| s.split_once('"')?.0.parse().ok()).collect()
}

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
    let flag = |n: &str| value(xml, n) == Some("true");
    // (the groups the file lists, by name and, the client's own, by number too)
    let mut listed: BTreeSet<String> = strings(xml, "selected_group_names").into_iter().collect();
    listed.extend(numbers(xml, "selected_group_ids").into_iter().filter_map(|id| name_of(id as u32)).map(str::to_string));
    let mut known: BTreeSet<String> = BUILTIN.iter().map(|g| g.1.to_string()).collect();
    known.extend(listed.iter().cloned());
    // (FUN_1009e379: every group starts as the window's autosubscribe flag says and the listed ones are the others, so
    // a window that takes new channels by itself lists the groups it does NOT show: the default window's list is Other
    // Pets and the fight's groups, and it shows the system's lines, speech, tells and every channel)
    let auto = flag("is_autosubscribe_window");
    let selected: BTreeSet<String> = if auto { known.difference(&listed).cloned().collect() } else { listed };
    let frame = value(xml, "window_name").unwrap_or(FIRST_FRAME).to_string();
    let hundredths = |n: &str, or: u8| value(xml, n).and_then(|v| v.parse::<f32>().ok()).map_or(or, |v| (v.clamp(0.0, 1.0) * 100.0).round() as u8);
    let look = Look {
        borderless: value(xml, "visual_mode") == Some("2"),
        solid: (hundredths("window_transparency_active", 80), hundredths("window_transparency_inactive", 30)),
        input: value(xml, "is_textinput_enabled") != Some("false"),
        hide_input: flag("hide_input_when_inactive"),
        fade: flag("is_message_fading_enabled"),
        layer: 0,
    };
    let pane = Pane { name, frame, selected, known, auto, output: value(xml, "output_group").unwrap_or("").to_string(), timestamps: flag("show_timestamps"), look };
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

/// `chatwindows.ini`: a window a block: `window <name>`, `frame <name>`, `auto 0|1`, `timestamps 0|1`, `look` with
/// its seven numbers (borderless, solid while active and otherwise, input bar, hidden bar, fading, layer), `output
/// <group>`, then a line a group it has met, `+ <group>` shown or `- <group>` not.
pub fn render(panes: &[Pane]) -> String {
    let mut out = String::new();
    for p in panes {
        let l = p.look;
        out += &format!("window {}\nframe {}\nauto {}\ntimestamps {}\n", p.name, p.frame, u8::from(p.auto), u8::from(p.timestamps));
        out += &format!("look {} {} {} {} {} {} {}\noutput {}\n", u8::from(l.borderless), l.solid.0, l.solid.1, u8::from(l.input), u8::from(l.hide_input), u8::from(l.fade), l.layer, p.output);
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
            panes.push(Pane { name: rest.to_string(), frame: FIRST_FRAME.to_string(), selected: BTreeSet::new(), known: BTreeSet::new(), auto: false, output: String::new(), timestamps: false, look: Look::default() });
            continue;
        }
        let Some(p) = panes.last_mut() else { continue };
        match word {
            "auto" => p.auto = rest == "1",
            "frame" if !rest.is_empty() => p.frame = rest.to_string(),
            "timestamps" => p.timestamps = rest == "1",
            "look" => {
                if let [borderless, active, inactive, input, hide, fade, layer] = rest.split_whitespace().filter_map(|w| w.parse::<i32>().ok()).collect::<Vec<_>>()[..] {
                    p.look = Look { borderless: borderless == 1, solid: (active.clamp(0, 100) as u8, inactive.clamp(0, 100) as u8), input: input == 1, hide_input: hide == 1, fade: fade == 1, layer: layer.signum() as i8 };
                }
            }
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
        assert_eq!((pane.name.as_str(), pane.frame.as_str(), pane.output.as_str(), pane.timestamps, pane.auto, tab, startup), ("Fights", "Window1", "Clan OOC", true, false, 2, true));
        // (a window that takes new channels by itself lists what it does not show: the default window's file names
        // Other Pets and a fight group, by name and by number, and the window shows the rest, speech and tells among them)
        let default = r#"<Archive code="0"><Array name="selected_group_ids"><Int64 value="1090519041" /><Int64 value="1107296258" /></Array><Array name="selected_group_names"><String value='&quot;Other Pets&quot;' /><String value='&quot;Clan OOC&quot;' /></Array><String name="name" value='&quot;Default Window&quot;' /><Bool name="is_autosubscribe_window" value="true" /></Archive>"#;
        let (shown, _, _) = parse_config(default).unwrap();
        assert!(shown.auto && shown.selected.contains(SYSTEM) && shown.selected.contains(VICINITY) && shown.selected.contains(TELLS));
        assert!(!shown.selected.contains("Other Pets") && !shown.selected.contains("Clan OOC") && !shown.selected.contains(name_of(0x4200_0002).unwrap()));
        assert!(shown.known.contains("Clan OOC") && shown.known.contains("Other Pets"));
        // (a config that says nothing of the look has the original's; one that does is read)
        assert_eq!(pane.look, Look::default());
        let styled = r#"<Archive code="0"><String name="name" value="Side" /><Int32 name="visual_mode" value="2" /><Float name="window_transparency_active" value="0.500000" /><Float name="window_transparency_inactive" value="0.100000" /><Bool name="is_textinput_enabled" value="false" /><Bool name="hide_input_when_inactive" value="true" /><Bool name="is_message_fading_enabled" value="true" /></Archive>"#;
        assert_eq!(parse_config(styled).unwrap().0.look, Look { borderless: true, solid: (50, 10), input: false, hide_input: true, fade: true, layer: 0 });
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
        // (the original's two share a frame; one moved out has its own, and a new name is free of both kinds)
        assert_eq!((frames(&panes), members(&panes, FIRST_FRAME)), (vec![FIRST_FRAME.to_string()], vec![0, 1]));
        panes[1].frame = free_name(&panes);
        assert_eq!((panes[1].frame.as_str(), frames(&panes).len(), members(&panes, FIRST_FRAME)), ("Window2", 2, vec![0]));
        assert_eq!((free_name(&panes), parse(&render(&panes)) == panes), ("Window3".to_string(), true));
        // (the look goes round too; a file without it has the original's)
        panes[0].look = Look { borderless: true, solid: (60, 10), input: false, hide_input: true, fade: true, layer: -1 };
        assert_eq!(parse(&render(&panes)), panes);
        assert_eq!(parse("window A\n")[0].look, Look::default());
        // (a file of before the frames puts every window in the first)
        assert_eq!(parse("window A\nwindow B\n").iter().map(|p| p.frame.clone()).collect::<Vec<_>>(), ["Window1", "Window1"]);
    }

    #[test]
    fn a_typed_name_finds_its_group() {
        note_channel("Zz Trade", 4, 0x8004);
        note_channel("Zz Trade Results", 10, 0x8006);
        note_channel("Zz Team", 0x82, 0x8004);
        // (the start of a name in any case; the whole name alone; quotes dropped)
        assert_eq!(matching("zz tr"), ["Zz Trade", "Zz Trade Results"]);
        assert_eq!(matching("\"zz trade\""), ["Zz Trade"]);
        assert!(matching("Zz Nothing").is_empty());
        assert!(read_only("Zz Trade Results") && read_only(SYSTEM) && !read_only("Zz Trade") && !read_only(VICINITY));
        // (a kind of this test's own: the tests share the list)
        note_channel("Zz Gone", 0x71, 0);
        assert_eq!((of_kind(0x71).as_deref(), of_kind(0x7f)), (Some("Zz Gone"), None));
        forget("Zz Gone");
        assert_eq!(of_kind(0x71), None);
        let shows = |g: &str| g == "Zz Trade Results" || g == "Zz Team";
        let pick_of = |typed: &str, shown_only: bool| pick(&matching(typed), shown_only, &shows, &read_only);
        // (`/group`: of several the one that can be written to; `/ch`: only one the window shows)
        assert_eq!(pick_of("zz tr", false), Pick::Group("Zz Trade".into()));
        assert_eq!(pick_of("zz tr", true), Pick::NoneUsable);
        assert_eq!(pick_of("zz trade", true), Pick::NotShown);
        assert_eq!(pick_of("zz trade r", false), Pick::ReadOnly("Zz Trade Results".into()));
        assert_eq!(pick_of("zz te", true), Pick::Group("Zz Team".into()));
        assert_eq!(pick_of("zz t", false), Pick::Ambiguous);
        assert_eq!(pick_of("zz nothing", false), Pick::Unknown);
        assert_eq!(ambiguous_list(&matching("zz tr"), &shows, &read_only), "<font color=silver>Zz Trade</font><font color=red>, Zz Trade Results</font>");
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
        assert!(is_combat("Me hit by monster") && !is_combat(VICINITY) && !is_combat("Clan OOC") && !is_combat("Research") && !is_combat("Vicinity Loot Messages"));
        // (a group's colour: the client's own by number, a channel by its kind)
        note("Test Clan Channel", 3);
        note("Test Private Group", 0x0e);
        assert_eq!([colour_name(SYSTEM), colour_name(VICINITY), colour_name(TELLS), colour_name("Research"), colour_name("Me hit by monster")], ["ct_system", "ctch_vicinity", "ctch_tell", "ctch_research", "white"]);
        assert_eq!([colour_name("Test Clan Channel"), colour_name("Test Private Group"), colour_name("Never heard of")], ["ctch_clan", "ctch_pgroup", "white"]);
        assert!(is_private("Test Private Group") && !is_private("Test Clan Channel") && available("Test Clan Channel") && available(SYSTEM) && !available("Never heard of"));
        assert!(talkable(VICINITY) && talkable("Clan OOC") && !talkable(SYSTEM) && !talkable("Me got XP"));
    }
}
