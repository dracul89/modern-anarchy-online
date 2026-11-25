//! The Action window: everything the character can do, as the original's grid of icon tiles. The catalogue
//! (`actions_data.rs`) was read from the running original client: the special actions (weapon attacks, the basic
//! character actions, perks) and the social actions with the dances among them (see `docs/GAME_MECHANICS.md`).
//! A tile is clicked to do it or dragged onto a hotbar slot.
//!
//! How the original's list comes to be (Gamecode): every special action has a number and an item template that names
//! and pictures it ([`TEMPLATES`], the table FUN_10042b17 fills). The character's own actions are added when the
//! character is made (FUN_1004326f: use, pick up, start combat, sit or stand, walk, sneak, camp, crawl, search, each
//! with the number it toggles to). The server's `SpecialAttackWeapon` message lists the character's special attacks
//! (an item, a stat and a four-letter code each), and each whose stat is in the table is added with the character as
//! its owner (FUN_1006b14c -> FUN_1006a166 -> FUN_10042f09): Dimach and Brawl in every capture, beside the plain
//! attack (stat 100, which has no action). The perk actions come with the server's 0xb4 list (FUN_10043080). A
//! weapon's own attacks (Burst, Fling Shot, Full Auto, Aimed Shot, ...) are added when the weapon is put on, from its
//! "can" flags and with the weapon as their owner, and Reload with them when the weapon takes ammunition
//! (FUN_1009e84b: [`WEAPON_CAN`], [`RELOAD`]): the server lists none of them, a soldier with a gun is sent the same
//! three as everyone (seen live, 2026-11-10).

use std::collections::HashMap;
use std::sync::Arc;

use crate::actions_data::ACTIONS;
use crate::gear::{GearWindow, Icon};
use crate::hotbar;
use crate::theme::Palette;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Weapon,
    Character,
    Perk,
}

/// Each special action's number and the item template that names and pictures it (Gamecode FUN_10042b17). The
/// numbers are character action codes (1 pick up, 3 use, 11 and 78 start and stop combat, 17 and 18 walk and run, 19
/// and 79 sneak, 20 and 141 crawl, 76 and 77 sit and stand, 81 and 82 camp, 110 reload, 134 search) and the stats of
/// the special attack skills (121 the bow's special attack, 142 brawl, 144 dimach, 146 sneak attack, 147 fast attack,
/// 148 burst, 150 fling shot, 151 aimed shot, 167 full auto, 489 backstab: the names are the templates').
pub const TEMPLATES: &[(u32, u32)] = &[
    (1, 0xc1aa), (3, 0xc1a3), (0xb, 0xc1a5), (0x4e, 0xc1ab), (0x11, 0xc1a2), (0x12, 0xc1a9), (0x13, 0xc1a7), (0x4f, 0x1412c), (0x4c, 0xc1a8),
    (0x4d, 0xc1a6), (0x51, 0x14124), (0x52, 0x1412b), (0x86, 0x1d9d7), (0x97, 0x14127), (0x94, 0x14123), (0xa7, 0x14121), (0x96, 0x14120),
    (0x8e, 0x14122), (0x90, 0x14125), (0x93, 0x14126), (0x79, 0x1e159), (0x92, 0x1f83a), (0x1e9, 0x32ba3), (0x14, 0x39c7d), (0x8d, 0x39c94),
    (0x6e, 0x3bd0b),
];

/// The actions a flying character cannot do: while its character is in fly mode the original marks these eight
/// unusable (`SwitchToFlyModeTransitionAction_t`, Gamecode FUN_1006e4f8, for the client's own character: stand, sit,
/// walk, run, sneak, leave sneak, crawl, leave crawl), and takes the marks away when it lands (FUN_1006e65e). A key
/// or a tile for one of them then says `Feedback_ActionIsNotAvailable` and does nothing (FUN_100427d6).
pub fn fly_refuses(number: u32) -> bool {
    matches!(number, 0x4d | 0x4c | 0x11 | 0x12 | 0x13 | 0x4f | 0x14 | 0x8d)
}

/// The movement that starts a crawl (`SwitchToCrawlMode`, FUN_1006eb8f: `WaitState` 0xe).
pub const START_CRAWL: u8 = 0x1b;
/// The movement that ends one (`LeaveCrawlMode`, FUN_1006eb2c: `WaitState` 0).
pub const LEAVE_CRAWL: u8 = 0x28;

/// What the Crawl tile does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crawl {
    Nothing,
    Polymorphed,
    Movement(u8),
}

/// `N3Msg_CrawlToggle` (Gamecode 0x10027a45): nothing while swimming (the movement's mode 4); crawling (`WaitState`, stat
/// 430, is 0xe) it leaves the crawl; a polymorphed character (stat 359, `MonsterData`, not 0) is told
/// `Feedback_YouCantBePolymorphed`; else it starts to crawl.
pub fn crawl_toggle(swimming: bool, crawling: bool, monster_data: i64) -> Crawl {
    if swimming {
        Crawl::Nothing
    } else if crawling {
        Crawl::Movement(LEAVE_CRAWL)
    } else if monster_data != 0 {
        Crawl::Polymorphed
    } else {
        Crawl::Movement(START_CRAWL)
    }
}

/// `N3Msg_DoSocialAction` (Gamecode 0x10026b4f): the feedback that refuses a social action, or `None` to send it. None
/// while swimming (the movement's mode 4); lounging and sleeping (0x45, 0x44) only sitting. (Its third refusal, a
/// vehicle equipped for those two, reads an object of the character's at +0x2c8 this client does not know.)
pub fn social_refusal(swimming: bool, sitting: bool, id: u32) -> Option<&'static str> {
    if swimming {
        Some("Feedback_CantDoSocialActionsWhileSwimming")
    } else if (id == 0x44 || id == 0x45) && !sitting {
        Some("Feedback_MustSitToLoungeOrSleep")
    } else {
        None
    }
}

/// The action's item template for a number, and the number for a template.
pub fn template_of(number: u32) -> Option<u32> {
    TEMPLATES.iter().find(|t| t.0 == number).map(|t| t.1)
}

pub fn number_of(template: u32) -> Option<u32> {
    TEMPLATES.iter().find(|t| t.1 == template).map(|t| t.0)
}

/// The special actions a weapon gives the character wearing it, by the bits of its "can" flags (stat 30): Gamecode
/// FUN_1009e84b, the weapon's own part of being put on (in a wear place, by the client's own character), adds each with
/// the weapon as its owner. Burst, fling shot, full auto, aimed shot, sneak attack, the bow's special attack and fast
/// attack. Beside sneak attack the original adds backstab ([`BACKSTAB`]) while bit 20 of the character's second flag
/// word is set, which the server's `CharacterAction` 0xb5 sets (FUN_10068976 → FUN_1009c5c8): `World::backstab`.
pub const WEAPON_CAN: [(u32, u32); 7] = [(0x800, 148), (0x1000, 150), (0x2000, 167), (0x4000, 151), (0x20000, 146), (0x8000, 121), (0x40000, 147)];

/// The special attacks' numbers, each its skill's stat: what `N3Msg_PerformSpecialAction` (FUN_100427d6) hands to
/// `N3Msg_SecondarySpecialAttack` with the target (Brawl and Dimach as the character's own, the rest as a weapon's).
pub const ATTACKS: [u32; 10] = [121, 142, 144, 146, 147, 148, 150, 151, 167, 489];

/// Search's number.
pub const SEARCH: u32 = 0x86;

/// The skills the server has locked, for the tiles' countdowns: until when and for how long in all, by the stat
/// (`SpecialUsed` and `SpecialAvailable`, as the hotbar's slots follow them; the original's `N3Msg_GetActionProgress`).
static LOCKS: std::sync::Mutex<Vec<(u32, std::time::Instant, f32)>> = std::sync::Mutex::new(Vec::new());

/// How long a lock the server tells of runs (Gamecode FUN_10063de2, asked by `SpecialUsed` 0xaa and by 0x14): the
/// seconds as told for the special attacks' own skills (142, 144, 146, 147, 148, 150, 151, 167, 579, 673); for any
/// other skill changed by the character's lock modifier (stat 382, a percentage, no less than -50): seconds +
/// modifier * seconds / 100, to the nearest whole. (The captures: a lock of stat 140 told as 60 s was freed after
/// 48.3 s on one character and after 61.6 s on another.)
pub fn lock_seconds(stat: u32, seconds: u32, modifier: i64) -> u32 {
    if matches!(stat, 142 | 144 | 146 | 147 | 148 | 150 | 151 | 167 | 579 | 673) {
        return seconds;
    }
    let m = modifier.max(-50) as f64;
    (m * seconds as f64 / 100.0 + seconds as f64 + 0.5).max(0.0) as u32
}

/// The lock a perk action's use starts: the number its template's lock spell (0xcfc3) names, which the server's
/// `PerkUnavailable` and `PerkAvailable` carry.
pub fn perk_lock(db: &mao_formats::resource::ResourceDatabase, template: u32) -> Option<u32> {
    let data = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: template }).ok().flatten()?;
    mao_formats::spell::lists(&data).into_iter().flat_map(|(_, s)| s).find(|s| s.kind == 0xcfc3).map(|s| s.stat(0) as u32)
}

/// The server locked a skill for `seconds` (0: it is ready again).
pub fn set_lock(stat: u32, seconds: f32) {
    let mut locks = LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    locks.retain(|l| l.0 != stat);
    if seconds > 0.0 {
        locks.push((stat, std::time::Instant::now() + std::time::Duration::from_secs_f32(seconds), seconds));
    }
}

/// What is left of an action's lock and its whole length, in seconds: a special attack's by its skill.
pub fn lock_left(id: u32) -> Option<(f32, f32)> {
    skill_lock_left(number_of(id).filter(|n| ATTACKS.contains(n))?)
}

/// What is left of a skill's lock and its whole length, in seconds: any skill the server has locked (First Aid and
/// Treatment after a stim or a kit, as well as the special attacks').
pub fn skill_lock_left(stat: u32) -> Option<(f32, f32)> {
    let locks = LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    let &(_, until, total) = locks.iter().find(|l| l.0 == stat)?;
    Some((until.saturating_duration_since(std::time::Instant::now()).as_secs_f32(), total)).filter(|l| l.0 > 0.0)
}

/// Whether the server has any skill locked now.
pub fn locks_running() -> bool {
    let now = std::time::Instant::now();
    LOCKS.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|l| l.1 > now)
}

/// An item's lock as a list view shows it now: the share gone, for the icon's fade, and the time written over it.
pub fn item_lock_state(template: u32) -> (Option<f32>, String) {
    let lock = item_lock_left(template);
    (lock_share(lock), timer_text(lock))
}

/// What is left of an item's lock: the longest of the locks on the skills its use locks (`net::lock_skills`: a
/// stim's use locks First Aid, and the server refuses the stim for as long: "Unable to perform action, First Aid
/// skill is locked"). The inventory and the hotbars fade such an item for that time.
pub fn item_lock_left(template: u32) -> Option<(f32, f32)> {
    longest(crate::net::lock_skills(mao_formats::item::TEMPLATE, template).into_iter().filter_map(skill_lock_left))
}

fn longest(locks: impl Iterator<Item = (f32, f32)>) -> Option<(f32, f32)> {
    locks.max_by(|a, b| a.0.total_cmp(&b.0))
}

/// The character's nano programs are busy until then, and for how long in all: a cast under way and the program's
/// recharge after it (`set_nano_lock`, from the client's own casts as the server starts them).
static NANO_LOCK: std::sync::Mutex<Option<(std::time::Instant, f32)>> = std::sync::Mutex::new(None);

/// A cast of the character's began: no program can be run for `seconds` (0: they are free).
pub fn set_nano_lock(seconds: f32) {
    *NANO_LOCK.lock().unwrap_or_else(|e| e.into_inner()) = (seconds > 0.0).then(|| (std::time::Instant::now() + std::time::Duration::from_secs_f32(seconds), seconds));
}

/// What is left of a nano program's lock: the cast and recharge under way, or a lock on a skill the program's own
/// run locks, whichever is longer.
pub fn nano_lock_left(nano: u32) -> Option<(f32, f32)> {
    let busy = NANO_LOCK.lock().unwrap_or_else(|e| e.into_inner()).map(|(until, total)| (until.saturating_duration_since(std::time::Instant::now()).as_secs_f32(), total)).filter(|l| l.0 > 0.0);
    longest(busy.into_iter().chain(crate::net::lock_skills(0xfde85, nano).into_iter().filter_map(skill_lock_left)))
}

/// The time left written over an icon while its lock runs (GUI FUN_1003f8c2 for the item, nano and action views),
/// empty without one.
pub fn timer_text(lock: Option<(f32, f32)>) -> String {
    lock.filter(|l| l.0 > 0.0).map_or_else(String::new, |(left, _)| crate::shortcutbar::timer_text(left))
}

/// The share of a lock's time that is gone, as the list views fade an icon by it (`multilist::Item::left`).
pub fn lock_share(lock: Option<(f32, f32)>) -> Option<f32> {
    lock.map(|(left, total)| 1.0 - left / total.max(1.0))
}

/// Sneak attack's number and backstab's, which goes with it for a character the server allows it.
pub const SNEAK_ATTACK: u32 = 146;
pub const BACKSTAB: u32 = 489;

/// Reload's number: added with the character as its owner when a weapon put on has an ammunition stat (26) that is
/// not -1 (the same function).
pub const RELOAD: u32 = 110;

/// Whether a weapon with these "can" flags lets its holder do the special attack of that number (Gamecode
/// FUN_1009aced, asked of each held weapon by `N3Msg_SecondarySpecialAttack`, 0x100281fc): the seven of
/// [`WEAPON_CAN`], Brawl by 0x2000000, Dimach by 0x4000000, and Backstab by the sneak attack's bit for a character
/// the server allowed it.
pub fn weapon_lets(can: u32, number: u32, backstab: bool) -> bool {
    let bit = match number {
        142 => 0x2000000,
        144 => 0x4000000,
        BACKSTAB if backstab => 0x20000,
        _ => WEAPON_CAN.iter().find(|f| f.1 == number).map_or(0, |f| f.0),
    };
    can & bit != 0
}

/// The numbers of the actions a weapon with these "can" flags gives.
pub fn weapon_numbers(can: u32) -> impl Iterator<Item = u32> {
    WEAPON_CAN.into_iter().filter(move |f| can & f.0 != 0).map(|f| f.1)
}

/// The character's own actions, in the order the original lists them when the character is made (Gamecode FUN_1004326f):
/// use, pick up, start combat, sit, walk, sneak, suspended animation, crawl and search. Each pair's first number is the
/// one the list starts with ([`PAIRS`]).
pub const CHARACTER_ORDER: [u32; 9] = [3, 1, 0xb, 0x4c, 0x11, 0x13, 0x51, 0x14, SEARCH];

/// The character's actions that come in pairs, each with the number it toggles to (FUN_1004326f): start and end combat, sit
/// and stand, walk and run, sneak and stop sneaking, suspended animation and disrupt, crawl and stop crawling.
pub const PAIRS: [(u32, u32); 6] = [(0xb, 0x4e), (0x4c, 0x4d), (0x11, 0x12), (0x13, 0x4f), (0x51, 0x52), (0x14, 0x8d)];

/// The states a pair's tile follows, as the network thread last saw them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Toggles {
    pub fighting: bool,
    pub sitting: bool,
    pub walking: bool,
    pub sneaking: bool,
    pub camping: bool,
    pub crawling: bool,
}

static TOGGLES: std::sync::Mutex<Toggles> = std::sync::Mutex::new(Toggles { fighting: false, sitting: false, walking: false, sneaking: false, camping: false, crawling: false });

/// The network thread tells the states the pairs' tiles follow.
pub fn set_toggles(t: Toggles) {
    *TOGGLES.lock().unwrap_or_else(|e| e.into_inner()) = t;
}

pub fn toggles() -> Toggles {
    *TOGGLES.lock().unwrap_or_else(|e| e.into_inner())
}

/// Which of a pair a tile shows: the one a click does next. The original swaps the entry when the state changes (Gamecode
/// FUN_1004300e, given the number just done or entered, shows the other one of its pair; called when a fight starts or
/// stops, the character sits or stands, changes its pace, sneaks, camps or crawls), and the Action window and the shortcut
/// bars follow (GlobalSignals +0xa8). A fresh character shows start combat, sit, walk (it runs), sneak, suspended animation
/// and crawl. A number of no pair is itself.
pub fn shown_number(number: u32, t: Toggles) -> u32 {
    let Some(&(first, second)) = PAIRS.iter().find(|p| p.0 == number || p.1 == number) else { return number };
    let on = match first {
        0xb => t.fighting,
        0x4c => t.sitting,
        0x11 => t.walking,
        0x13 => t.sneaking,
        0x51 => t.camping,
        _ => t.crawling,
    };
    if on { second } else { first }
}

/// [`shown_number`] by the action's template.
pub fn shown_template(template: u32, t: Toggles) -> u32 {
    number_of(template).map(|n| shown_number(n, t)).and_then(template_of).unwrap_or(template)
}

/// The templates the Action window lists, in the original's order (`N3Msg_GetSpecialActionList`, the list FUN_10042f09
/// adds to): the character's own actions as their pairs stand, the special attacks (the server's list, then what the
/// weapons in the hands give, Reload with them), then the perk actions the server listed. Nothing else: before the server
/// lists them there are no special attacks and no perks. The window shows one tile for an action listed twice
/// (FUN_100dc05e finds the tile by its identity).
pub fn listed_templates(specials: &[u32], perks: &[u32], t: Toggles) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for template in CHARACTER_ORDER.iter().filter_map(|&n| template_of(n)).map(|template| shown_template(template, t)).chain(specials.iter().copied()).chain(perks.iter().copied()) {
        if !out.contains(&template) {
            out.push(template);
        }
    }
    out
}

/// A special attack the server lists for the character whose stat has an action: the stat, the action's template
/// and what the template says of it.
#[derive(Clone)]
pub struct Special {
    pub stat: u32,
    pub template: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
}

/// The listed specials that have an action (FUN_10042f09: the stat must be in the table), described from the action's
/// template.
pub fn describe_specials(db: &mao_formats::resource::ResourceDatabase, textures: &crate::Textures, stats: &[u32]) -> Vec<Special> {
    use mao_formats::item::{self, Item};
    stats
        .iter()
        .filter_map(|&stat| Some((stat, template_of(stat).filter(|t| *t != 0)?)))
        .map(|(stat, template)| {
            let parsed = db.get(mao_formats::resource::Id { kind: item::TEMPLATE, instance: template }).ok().flatten().as_deref().and_then(Item::parse);
            let icon = parsed.as_ref().and_then(|t| t.icon()).and_then(|id| textures.get(item::ICON_TYPE, id).map(|t| Arc::new(Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })));
            Special { stat, template, name: parsed.as_ref().map_or_else(|| format!("Special {stat}"), |t| t.name.clone()), description: parsed.map(|t| t.description).unwrap_or_default(), icon }
        })
        .collect()
}

/// A perk action of the character's (the server's 0xb4 list): its item template (the special action's id), name, icon, what
/// it does, and when its lock (the template's lock-skill spell 0xcfc3: the group in its stat 0) is free again.
#[derive(Clone)]
pub struct PerkAction {
    pub template: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    pub lock: Option<u32>,
    /// When the lock is free again, and how long it runs in all (seconds).
    pub free_at: Option<(std::time::Instant, f32)>,
}

/// The perk actions described from their templates, with the locks running.
pub fn describe_perks(db: &mao_formats::resource::ResourceDatabase, textures: &crate::Textures, list: &[(u32, u32, [u8; 4])], locks: &std::collections::BTreeMap<u32, (std::time::Instant, f32)>) -> Vec<PerkAction> {
    use mao_formats::item::{self, Item};
    list.iter()
        .map(|&(template, number, code)| {
            let data = db.get(mao_formats::resource::Id { kind: item::TEMPLATE, instance: template }).ok().flatten();
            let parsed = data.as_deref().and_then(Item::parse);
            let icon = parsed.as_ref().and_then(|t| t.icon()).and_then(|id| textures.get(item::ICON_TYPE, id).map(|t| Arc::new(Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })));
            let lock = data.as_deref().and_then(|d| mao_formats::spell::lists(d).into_iter().flat_map(|(_, s)| s).find(|s| s.kind == 0xcfc3).map(|s| s.stat(0) as u32));
            PerkAction {
                template,
                name: parsed.as_ref().map_or_else(|| format!("{} ({number:#x})", String::from_utf8_lossy(&code)), |t| t.name.clone()),
                description: parsed.map(|t| t.description).unwrap_or_default(),
                icon,
                lock,
                free_at: lock.and_then(|l| locks.get(&l).copied()),
            }
        })
        .collect()
}

pub struct ActionsWindow {
    pub open: bool,
    /// The catalogue's icons by their actions' templates, once loaded.
    icons: HashMap<u32, Arc<Icon>>,
    loaded: bool,
    /// What a click asked for: sent to the network thread by the viewer.
    pub clicked: Option<hotbar::Action>,
    /// Info pages asked for (Shift + click on a tile).
    pub infos: Vec<String>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id)>,
    /// The other halves of the pairs the catalogue does not have (end combat, sit, run, stop sneaking, disrupt, stop
    /// crawling): named and pictured by their templates.
    pair_looks: HashMap<u32, (String, Option<Arc<Icon>>)>,
}

impl Default for ActionsWindow {
    fn default() -> Self {
        ActionsWindow { open: false, icons: HashMap::new(), loaded: false, clicked: None, infos: Vec::new(), views: None, pair_looks: HashMap::new() }
    }
}

impl ActionsWindow {
    /// Reads the icons the tiles use from the game's database (once).
    pub fn load_icons(&mut self, textures: &crate::Textures) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        use mao_formats::item::{self, Item};
        // (each action pictured by its item template's icon, as the original's are: FUN_10042b17's template names and
        // pictures it. Until 2026-11-19 the catalogue's were picked by name, and a perk's by words in its name.)
        for &(template, ..) in ACTIONS {
            let parsed = textures.db().get(mao_formats::resource::Id { kind: item::TEMPLATE, instance: template }).ok().flatten().as_deref().and_then(Item::parse);
            if let Some(icon) = parsed.as_ref().and_then(|t| t.icon()).and_then(|id| textures.get(item::ICON_TYPE, id).map(|t| Arc::new(Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba }))) {
                self.icons.insert(template, icon);
            }
        }
        for template in PAIRS.iter().flat_map(|p| [p.0, p.1]).filter_map(template_of).filter(|t| !ACTIONS.iter().any(|a| a.0 == *t)) {
            let parsed = textures.db().get(mao_formats::resource::Id { kind: item::TEMPLATE, instance: template }).ok().flatten().as_deref().and_then(Item::parse);
            let icon = parsed.as_ref().and_then(|t| t.icon()).and_then(|id| textures.get(item::ICON_TYPE, id).map(|t| Arc::new(Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })));
            self.pair_looks.insert(template, (parsed.map_or_else(|| format!("Action {template}"), |t| t.name), icon));
        }
    }

    /// The icon behind a hotbar entry's `icon` number (an index into the action icons).
    pub fn icon_by_instance(&self, instance: u32) -> Option<&Arc<Icon>> {
        self.icons.values().chain(self.pair_looks.values().filter_map(|l| l.1.as_ref())).find(|i| i.id == instance)
    }

    /// The icon of the action whose template is `template` (a shortcut kept with a picture this client no longer
    /// loads is drawn with its action's own).
    pub fn template_icon(&self, template: u32) -> Option<&Arc<Icon>> {
        self.icons.get(&template).or_else(|| self.pair_looks.get(&template).and_then(|l| l.1.as_ref()))
    }

    /// A catalogue action's or a pair's other half's name and icon, by its template.
    fn look(&self, template: u32) -> Option<(String, Option<Arc<Icon>>, Kind)> {
        if let Some(&(_, kind, name, _, _)) = ACTIONS.iter().find(|a| a.0 == template) {
            return Some((name.to_string(), self.icons.get(&template).cloned(), kind));
        }
        self.pair_looks.get(&template).map(|(name, icon)| (name.clone(), icon.clone(), Kind::Character))
    }

    /// A shortcut's action as its pair stands (the shortcut bar follows the swap, GUI.dll FUN_100d9380): a click does what
    /// the tile shows, the network thread going by the pair's numbers.
    pub fn paired(&self, action: hotbar::Action) -> hotbar::Action {
        let hotbar::Action::Special { id, .. } = &action else { return action };
        let shown = shown_template(*id, toggles());
        match self.look(shown) {
            Some((name, icon, _)) if shown != *id => hotbar::Action::Special { id: shown, name, icon: icon.map(|i| i.id) },
            _ => action,
        }
    }

    /// The window, GUI.dll's `SpecialActionView_c` (FUN_100dc9b1): "Actions", a grid of the special actions in the
    /// smallest square icon size (`SetGridIconSize(0)`: 31 by 31 on `SLOT_32`), in the order the list holds them. A click
    /// performs the action (FUN_100dc033: `N3Msg_PerformSpecialAction`; the list does not select, so a left click is the use
    /// signal); one dragged out goes onto a hotbar. The social actions are the control center's Social menu
    /// (`ActionMenu/SocialMenu.xml`), as in the original.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, perks: &[PerkAction], specials: Option<&[Special]>, palette: &Palette) {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        if self.views.is_none() {
            use crate::gui::view::{Layout, Node, Tree};
            let mut tree = Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
            tree.get_mut(root).layout = Layout::Base;
            let list = crate::gui::multilist::create_grid(&mut tree, "SpecialActionView", -1, 5);
            crate::gui::multilist::set_grid(&mut tree, list, |g| g.set_icon_size(0));
            tree.add_child(root, list);
            self.views = Some((tree, root, list));
        }
        // (the original's list, `listed_templates`: the character's own actions as their pairs stand, the special attacks
        // the server and the weapons give, the perk actions the server listed. Until 2026-11-18 the whole catalogue showed
        // until the server had listed them, and its perks whenever the character had none.)
        let now = std::time::Instant::now();
        let specials = specials.unwrap_or_default();
        let templates = listed_templates(&specials.iter().map(|s| s.template).collect::<Vec<_>>(), &perks.iter().map(|p| p.template).collect::<Vec<_>>(), toggles());
        let mut actions: Vec<(hotbar::Action, String, Option<Arc<Icon>>, String, Option<(f32, f32)>)> = Vec::new();
        for template in templates {
            if let Some(p) = perks.iter().find(|p| p.template == template) {
                let left = p.free_at.map(|(t, total)| (t.saturating_duration_since(now).as_secs_f32(), total)).filter(|&(s, _)| s > 0.0);
                actions.push((hotbar::Action::Special { id: p.template, name: p.name.clone(), icon: None }, p.name.clone(), p.icon.clone(), if p.description.is_empty() { "Perk action".to_string() } else { p.description.clone() }, left));
                continue;
            }
            // (a listed special the catalogue does not have: named and pictured by its action's template)
            let (name, icon, what) = match (self.look(template), specials.iter().find(|s| s.template == template)) {
                (Some((name, icon, kind)), _) => (name, icon, (if kind == Kind::Weapon { "Weapon attack" } else { "Character action" }).to_string()),
                (None, Some(s)) => (s.name.clone(), s.icon.clone(), if s.description.is_empty() { "Weapon attack".to_string() } else { s.description.clone() }),
                (None, None) => continue,
            };
            let shortcut_icon = icon.as_ref().map(|i| i.id).filter(|&i| self.icon_by_instance(i).is_some());
            actions.push((hotbar::Action::Special { id: template, name: name.clone(), icon: shortcut_icon }, name, icon, what, lock_left(template)));
        }
        if actions.iter().any(|a| a.4.is_some()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        let rows: Vec<crate::gui::multilist::Item> = actions
            .iter()
            .enumerate()
            .map(|(k, (_, name, icon, _, left))| crate::gui::multilist::Item {
                id: k as i64,
                icon: icon.as_ref().map(|i| gear.texture(ctx, i)),
                cells: vec![(1, crate::gui::multilist::CellView::Text(name.clone()))],
                keys: vec![(1, name.clone())],
                // (a locked perk fades out until its lock is free, as the NCU's running nanos do)
                left: left.map(|(s, total)| 1.0 - s / total.max(1.0)),
                timer: timer_text(*left),
                ..Default::default()
            })
            .collect();
        let Some((tree, root, list)) = self.views.as_mut() else { return };
        let (root, list) = (*root, *list);
        if crate::gui::multilist::items(tree, list) != rows.as_slice() {
            crate::gui::multilist::set_items(tree, list, rows);
        }
        crate::gui::multilist::update(tree, list);
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut hovered = None;
        crate::chrome::window(ctx, palette, "Actions", "Actions", &mut open, |w| w.default_pos(crate::layout::pos("Actions", egui::pos2(520.0, 70.0))).default_size(egui::vec2(5.0 * 42.0 + 30.0, 4.0 * 42.0 + 12.0)).resizable(true), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                hovered = crate::gui::multilist::item_at(tree, list, response.rect.min, at).map(|k| (k as usize, at));
            }
        });
        // (a tile is taken up by the button held on it, not by a press that moves: `multilist::hold_to_pick`,
        // FUN_100dc1ab on the list's pick-up signal)
        crate::gui::multilist::hold_to_pick(tree, list);
        for (_, event) in crate::gui::multilist::take_events(tree) {
            match event {
                crate::gui::multilist::ItemEvent::Clicked(k, _) => {
                    // (a locked one is clicked like any other: `N3Msg_PerformSpecialAction`, FUN_100427d6, then says
                    // the wait line of what locks it and sends nothing. The network thread says it: a perk's
                    // `UnableToPerformActionPerk`, a skill's `UnableToPerformActionSkill`. Until 2026-11-18 the click
                    // was dropped here without a word.)
                    // (with Shift the click shows the action's info and does not perform it: the list's qualifier
                    // signal, FUN_100dc3ae)
                    if let Some((a, ..)) = actions.get(k as usize) {
                        if ctx.input(|i| i.modifiers.shift) {
                            self.infos.extend(hotbar::info_url(a, |_| None));
                        } else {
                            self.clicked = Some(a.clone());
                        }
                    }
                }
                crate::gui::multilist::ItemEvent::Dragged(k) => {
                    if let Some((a, ..)) = actions.get(k as usize) {
                        egui::DragAndDrop::set_payload(ctx, a.clone());
                    }
                }
            }
        }
        tree.signals.clear();
        if let Some((k, at)) = hovered
            && let Some((_, name, _, what, left)) = actions.get(k)
        {
            egui::Area::new(egui::Id::unique("action-tip")).order(egui::Order::Tooltip).fixed_pos(at + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(320.0);
                    ui.label(egui::RichText::new(name.as_str()).strong().color(accent));
                    if let Some(s) = left {
                        ui.label(egui::RichText::new(format!("Available in {:.0} s", s.0.ceil())).small());
                    }
                    ui.label(egui::RichText::new(what.as_str()).small().weak());
                });
            });
        }
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `N3Msg_DoSocialAction`: swimming refuses every social action first; lounge and sleep want sitting.
    #[test]
    fn social_actions_are_refused_swimming_and_lounging_unseated() {
        assert_eq!(social_refusal(true, true, 0x10), Some("Feedback_CantDoSocialActionsWhileSwimming"));
        assert_eq!(social_refusal(false, false, 0x45), Some("Feedback_MustSitToLoungeOrSleep"));
        assert_eq!(social_refusal(false, true, 0x44), None);
        assert_eq!(social_refusal(false, false, 0x10), None);
    }

    /// A pistol's flags (0x1405) let no one brawl; a katana's (0x66000405) let one brawl and use Dimach; a burst
    /// weapon bursts.
    #[test]
    fn a_weapon_lets_its_own_specials_through() {
        assert!(!weapon_lets(0x1405, 142, false) && !weapon_lets(0x1405, 144, false));
        assert!(weapon_lets(0x66000405, 142, false) && weapon_lets(0x66000405, 144, false));
        assert!(weapon_lets(0x1405, 150, false) && !weapon_lets(0x1405, 148, false) && weapon_lets(0x805, 148, false));
        assert!(!weapon_lets(0x20000, BACKSTAB, false) && weapon_lets(0x20000, BACKSTAB, true) && weapon_lets(0x20000, SNEAK_ATTACK, false));
    }

    /// The Sit/Stand, Walk/Run, Sneak and Crawl tiles are among what a flyer cannot do; the fights' and Use are not.
    #[test]
    fn a_flyer_cannot_sit_sneak_crawl_or_change_its_pace() {
        for name in ["Stand", "Walk", "Sneak", "Crawl"] {
            let id = crate::actions_data::ACTIONS.iter().find(|a| a.2 == name).unwrap().0;
            assert!(number_of(id).is_some_and(fly_refuses), "{name}");
        }
        for name in ["Start Combat", "Use", "Reload", "Search"] {
            let id = crate::actions_data::ACTIONS.iter().find(|a| a.2 == name).unwrap().0;
            assert!(!number_of(id).is_some_and(fly_refuses), "{name}");
        }
    }

    /// FUN_10063de2: a special attack's own lock runs as told; another skill's by the lock modifier, which takes no
    /// more than half off.
    #[test]
    fn a_lock_runs_by_the_lock_modifier() {
        assert_eq!(lock_seconds(148, 30, -20), 30);
        assert_eq!(lock_seconds(140, 60, 0), 60);
        assert_eq!(lock_seconds(140, 60, -20), 48);
        assert_eq!(lock_seconds(123, 40, -80), 20);
        assert_eq!(lock_seconds(123, 45, 10), 50);
        assert_eq!(lock_seconds(123, 5, -10), 5);
    }

    /// A skill the server locked is locked for anything that goes by it; the nano programs are busy for a cast's
    /// time. (`MAO_GAME`: the starter's Health and Nano Stim locks First Aid, 123, and is locked with it.)
    #[test]
    fn a_locked_skill_locks_what_goes_by_it() {
        set_lock(990_123, 40.0);
        let (left, total) = skill_lock_left(990_123).unwrap();
        assert!(left > 39.0 && total == 40.0);
        assert_eq!(lock_share(Some((10.0, 40.0))), Some(0.75));
        set_lock(990_123, 0.0);
        assert_eq!(skill_lock_left(990_123), None);
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        crate::net::set_item_db(std::sync::Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap()));
        assert_eq!(crate::net::lock_skills(mao_formats::item::TEMPLATE, 291043), vec![123]);
        assert_eq!(item_lock_left(291043), None);
        set_lock(123, 40.0);
        assert!(item_lock_left(291043).is_some_and(|l| l.0 > 39.0));
        assert!(locks_running() && item_lock_state(291043).1 == "40s");
        set_lock(123, 0.0);
        assert_eq!(item_lock_left(291043), None);
        assert_eq!(item_lock_state(291043), (None, String::new()));
    }

    /// WIN-022: the window lists the character's nine actions first, then the specials as given, then the perks; nothing
    /// before the server's lists; an action listed twice shows once.
    #[test]
    fn the_window_lists_what_the_character_has_in_the_original_order() {
        let fresh = listed_templates(&[], &[], Toggles::default());
        assert_eq!(fresh, [0xc1a3, 0xc1aa, 0xc1a5, 0xc1a8, 0xc1a2, 0xc1a7, 0x14124, 0x39c7d, 0x1d9d7]);
        let all = listed_templates(&[0x14125, 0x14122, 0x14123, 0x14123, 0x3bd0b], &[0x374d7], Toggles::default());
        assert_eq!(all[9..], [0x14125, 0x14122, 0x14123, 0x3bd0b, 0x374d7]);
    }

    /// WIN-112: the Crawl tile starts and leaves a crawl as `N3Msg_CrawlToggle` does, and the tile then shows the other half.
    #[test]
    fn a_locked_icon_writes_its_time_left() {
        assert_eq!(timer_text(None), "");
        assert_eq!(timer_text(Some((0.0, 10.0))), "");
        assert_eq!(timer_text(Some((4.2, 10.0))), " 5s");
        assert_eq!(timer_text(Some((125.0, 300.0))), " 2m");
    }

    #[test]
    fn the_crawl_tile_toggles_the_crawl() {
        assert_eq!(crawl_toggle(false, false, 0), Crawl::Movement(START_CRAWL));
        assert_eq!(crawl_toggle(false, true, 0), Crawl::Movement(LEAVE_CRAWL));
        assert_eq!(crawl_toggle(false, false, 5), Crawl::Polymorphed);
        assert_eq!(crawl_toggle(false, true, 5), Crawl::Movement(LEAVE_CRAWL), "a crawler leaves the crawl whatever its body");
        assert_eq!(crawl_toggle(true, true, 0), Crawl::Nothing);
        let t = Toggles { crawling: true, ..Toggles::default() };
        assert_eq!((shown_number(0x14, Toggles::default()), shown_number(0x14, t)), (0x14, 0x8d));
    }

    /// WIN-113: a pair's tile shows what a click does next (FUN_1004300e).
    #[test]
    fn a_pairs_tile_shows_what_a_click_does_next() {
        let t = Toggles::default();
        assert_eq!([0xb, 0x4c, 0x11, 0x13, 0x51, 0x14].map(|n| shown_number(n, t)), [0xb, 0x4c, 0x11, 0x13, 0x51, 0x14]);
        let on = Toggles { fighting: true, sitting: true, walking: true, sneaking: true, camping: true, crawling: true };
        assert_eq!([0xb, 0x4c, 0x11, 0x13, 0x51, 0x14].map(|n| shown_number(n, on)), [0x4e, 0x4d, 0x12, 0x4f, 0x52, 0x8d]);
        // (either half of a pair goes the same way; a number of no pair is itself)
        assert_eq!((shown_number(0x4d, t), shown_number(0x4e, on), shown_number(3, on), shown_number(148, on)), (0x4c, 0x4e, 3, 148));
        // (by template: the catalogue's Stand tile is Sit while standing)
        assert_eq!((shown_template(0xc1a6, t), shown_template(0xc1a6, Toggles { sitting: true, ..t })), (0xc1a8, 0xc1a6));
    }

    #[test]
    fn the_listed_specials_decide_which_skill_actions_show() {
        // (the table: a number's template and back; the plain attack, stat 100, has no action)
        assert_eq!((template_of(144), template_of(142), template_of(100), number_of(0x14123)), (Some(0x14125), Some(0x14122), None, Some(148)));
        // (a weapon's flags: burst and full auto; fling shot, aimed shot and fast attack; none)
        assert_eq!(weapon_numbers(0x2800 | 0x4d).collect::<Vec<_>>(), [148, 167]);
        assert_eq!(weapon_numbers(0x45000).collect::<Vec<_>>(), [150, 151, 147]);
        assert_eq!(weapon_numbers(0x4d).count(), 0);
        // (the catalogue's weapon and character actions are all in the table)
        for a in ACTIONS.iter().filter(|a| a.1 != Kind::Perk) {
            assert!(number_of(a.0).is_some(), "{}", a.2);
        }
    }

    /// `MAO_GAME`: the specials every capture lists (the plain attack 100, Dimach 144, Brawl 142) become the two actions
    /// the original shows, named from the actions' templates.
    #[test]
    fn listed_specials_are_described_from_their_templates() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        let textures = crate::Textures::new(db.clone());
        let specials = describe_specials(&db, &textures, &[100, 144, 142, 151]);
        assert_eq!(specials.iter().map(|s| (s.stat, s.template)).collect::<Vec<_>>(), [(144, 0x14125), (142, 0x14122), (151, 0x14127)]);
        assert_eq!((specials[0].name.as_str(), specials[1].name.as_str()), ("Dimach", "Brawl"));
        assert!(!specials[2].name.starts_with("Special "), "{}", specials[2].name);
        // (backstab, which goes beside a weapon's sneak attack: its own template)
        let more = describe_specials(&db, &textures, &[SNEAK_ATTACK, BACKSTAB]);
        assert_eq!(more.iter().map(|s| (s.template, s.name.as_str())).collect::<Vec<_>>(), [(0x1f83a, "Sneak Attack"), (0x32ba3, "Backstab")]);
    }

    /// `MAO_GAME`: the perk action the 2026-10-31 capture lists as `0x374d7`, number `0x2850`, code `QUBS`, is Quick Bash, whose
    /// lock (the capture's `PerkUnavailable` after using it: lock 0xc9 for 25 s) is group 201.
    #[test]
    fn a_listed_perk_action_is_described_from_its_template() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        let textures = crate::Textures::new(db.clone());
        let now = std::time::Instant::now();
        let locks = [(201, (now + std::time::Duration::from_secs(25), 25.0))].into_iter().collect();
        let perks = describe_perks(&db, &textures, &[(0x374d7, 0x2850, *b"QUBS")], &locks);
        assert_eq!(perks[0].name, "Quick Bash");
        assert_eq!(perks[0].lock, Some(201));
        assert!(perks[0].free_at.is_some());
    }

    /// `MAO_GAME`: each catalogue action is pictured by its item template's icon (FUN_10042b17's template names and
    /// pictures the action): Reload has its own picture, not Aimed Shot's.
    #[test]
    fn catalogue_actions_are_pictured_by_their_templates() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        let textures = crate::Textures::new(db.clone());
        let mut window = ActionsWindow::default();
        window.load_icons(&textures);
        assert_eq!(window.look(245003).and_then(|l| l.1).map(|i| i.id), Some(245002));
        assert_eq!(window.template_icon(82211).map(|i| i.id), Some(82198));
        assert!(ACTIONS.iter().all(|a| window.template_icon(a.0).is_some()));
    }
}
