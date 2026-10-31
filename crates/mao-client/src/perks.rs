//! The perk window (`P`, `/perks`), GUI.dll's `PerkWindowModule_c` and `PerkWindow_c` (FUN_10063ff8, `perk_window`, title
//! text 10000 "PerkWindow", 400 × 200 at first, help "The Perks Window.html"): three tabs, "ProfessionPerks", "GroupPerks"
//! and "GeneralPerks", over one `PerkView_c` (FUN_10061a07) that shows the open tab's list.
//!
//! The lists (`PerkWindowModule_c::LoadPerks` 0x1006372d) come from Gamecode's perk manager (`N3Msg_GetFullPerkMap`, the
//! database's perk records, type `0xf4264`: the id at byte 4, the item template at 12, the rank before at 16, the flags at 20,
//! the professions at 24). A record is listed unless its flags have bit 0 (special); an alien one (bit 1) needs the Alien
//! Invasion expansion (stat 389 bit 8), any other needs Shadowlands (bit 2) and none of the flags 0xc0; flag bits 2 to 5 limit
//! it to breed 1 to 4 (stat 4). The ranks are linked by their rank before; a line's first rank goes to the profession list when
//! its professions are exactly the character's (stat 60, `N3Msg_IsProfessionPerk`), the group list when they include it
//! (`IsGroupPerk`), the general list when they are none (`IsGeneralPerk`), else nowhere. Each list is sorted (FUN_100643db)
//! by whether the line's first rank's criteria are met, then whether it is owned, then by name.
//!
//! The view: "PerkLines" over the list of lines on the left (`PerkListView_c`, FUN_1005f20d, a row per line, scrolling), the
//! chosen line's name over its ranks on the right in two columns, then (with Shadowlands, stat 389 bit 2) "UsedPerkPoints"
//! and "AvailablePerkPoints", (with Alien Invasion or Lost Eden, bits 8 and 16) "UsedAlienPerkPoints" and
//! "AvailableAlienPerkPoints", and under them the "RemovePerk" button beside the untraining timer. A row (`PerkLineView_c`,
//! FUN_1005f134) is a toggle button with a status icon (`StatusIcon_c`, bitmap 0x121) tinted by FUN_1005eab2: owned 0x006e00,
//! criteria met 0x646464, else 0x960000; an alien perk's label is in `<font color=#30ff30>`; a rank's button is down when the
//! rank is owned, a line's when it is the chosen one.
//!
//! Clicking a rank trains it (FUN_10061021) after the checks the original makes, each a message box; the remove button
//! (FUN_100616d2) untrains the highest owned rank of the chosen line. Training and untraining are `N3Msg_TrainPerk` /
//! `N3Msg_UntrainPerk` (CharacterAction 0xbb / 0xbc with the rank, the 2026-10-31 capture), sent on Yes (FUN_100605d3,
//! FUN_1006089f).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};

use crate::theme::Palette;

const PERK_RECORD: u32 = 0xf4264;

/// One perk rank: its record and what its item template says.
pub struct PerkRecord {
    pub id: u32,
    /// The rank before (0 for a line's first).
    pub prev: u32,
    /// Bit 0 special, 1 alien, 2 to 5 breed 1 to 4 only, 6 and 7 not perks (research, vehicles).
    pub flags: u32,
    /// The professions, a bit each (none: a general perk).
    pub professions: u32,
    /// The research amount of a research rank (byte 32, the perk manager's entry +0x18: what `N3Msg_GetPerkProgress`
    /// divides by).
    pub total: u32,
    pub name: String,
    pub description: String,
    /// The criteria of the template's wear action (what `N3Msg_MeetsPerkCriteria` checks).
    pub criteria: Vec<(u32, i32, u32)>,
    /// `N3Msg_GetUseByString` and the modifiers (`N3Msg_GetModifierString` of lists 14 and 0), for the tip.
    pub use_by: Option<String>,
    pub modifiers: String,
}

#[derive(Default)]
pub struct Catalog {
    pub records: Vec<PerkRecord>,
    by_id: HashMap<u32, usize>,
}

impl Catalog {
    pub fn get(&self, id: u32) -> Option<&PerkRecord> {
        self.by_id.get(&id).map(|&k| &self.records[k])
    }

    pub fn from_records(records: Vec<PerkRecord>) -> Catalog {
        let by_id = records.iter().enumerate().map(|(k, r)| (r.id, k)).collect();
        Catalog { records, by_id }
    }
}

/// The perk records, read once (in id order, as the perk manager's map keeps them).
pub fn catalog(db: &ResourceDatabase) -> Arc<Catalog> {
    static CATALOG: std::sync::OnceLock<Arc<Catalog>> = std::sync::OnceLock::new();
    CATALOG
        .get_or_init(|| {
            let word = |d: &[u8], at: usize| d.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
            let mut ids: Vec<Id> = db.ids().map(|it| it.filter_map(Result::ok).filter(|i| i.kind == PERK_RECORD).collect()).unwrap_or_default();
            ids.sort_by_key(|i| i.instance);
            let records = ids
                .into_iter()
                .filter_map(|id| {
                    let r = db.get(id).ok().flatten()?;
                    let (id, template, prev, flags, professions) = (word(&r, 4)?, word(&r, 12)?, word(&r, 16)?, word(&r, 20)?, word(&r, 24)?);
                    let total = word(&r, 32).unwrap_or(0);
                    let data = db.get(Id { kind: item::TEMPLATE, instance: template }).ok().flatten();
                    let parsed = data.as_deref().and_then(Item::parse);
                    let actions = data.as_deref().map(item::actions).unwrap_or_default();
                    let criteria = actions.iter().find(|a| a.kind == 6).map(|a| a.criteria.clone()).unwrap_or_default();
                    let modifiers = data.as_deref().map(|d| modifiers(db, d, parsed.as_ref())).unwrap_or_default();
                    Some(PerkRecord {
                        id,
                        prev,
                        flags,
                        professions,
                        total,
                        name: parsed.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
                        description: parsed.map(|p| p.description).unwrap_or_default(),
                        criteria,
                        use_by: crate::gui::criteria::use_by(&actions),
                        modifiers,
                    })
                })
                .collect();
            Arc::new(Catalog::from_records(records))
        })
        .clone()
}

/// `N3Msg_GetModifierString` of the template's worn (14) and used (0) lists, at its quality.
fn modifiers(db: &ResourceDatabase, data: &[u8], parsed: Option<&Item>) -> String {
    let level = parsed.and_then(|p| p.stat(54)).unwrap_or(1);
    let lists = mao_formats::spell::lists(data);
    let nano_name = |id: u32| crate::gui::iteminfo::nano_name(db, id);
    let mut m = String::new();
    for list in [14, 0] {
        for (_, spells) in lists.iter().filter(|(l, _)| *l == list) {
            m += &crate::gui::spells::convert(spells, level, &nano_name);
        }
    }
    m
}

/// What the window works from: the records, the ranks the character owns (`N3Msg_HasPerk`: the world-entry `Perks` and the
/// server's train/untrain echoes) and the server clock's offset from this one (`N3Msg_GetServerSynceSystemTime`).
#[derive(Clone, Default)]
pub struct PerkState {
    pub catalog: Arc<Catalog>,
    pub owned: Vec<u32>,
    pub server_offset: i64,
}

/// `LoadPerks`'s filter: whether a record with these flags is listed for this expansion set (stat 389) and breed (stat 4).
pub fn listed(flags: u32, expansion: i64, breed: i64) -> bool {
    if flags & 1 != 0 {
        return false;
    }
    let expansion_ok = if flags & 2 != 0 {
        expansion & 8 != 0
    } else {
        if flags & 0xc0 != 0 {
            return false;
        }
        expansion & 2 != 0
    };
    expansion_ok && (2..6).all(|bit| flags & (1 << bit) == 0 || breed == bit as i64 - 1)
}

/// The list a line's first rank goes to: 0 profession (`IsProfessionPerk`, Gamecode FUN_100266a4: the professions are exactly
/// the character's), 1 group (`IsGroupPerk`: they include it), 2 general (`IsGeneralPerk`: none), else none.
pub fn kind(professions: u32, profession: i64) -> Option<usize> {
    let bit = 1u32.wrapping_shl(profession as u32 & 0x1f);
    if professions != 0 && bit & professions == professions {
        Some(0)
    } else if bit & professions != 0 {
        Some(1)
    } else if professions == 0 {
        Some(2)
    } else {
        None
    }
}

/// `N3Msg_MeetsPerkCriteria` (Gamecode FUN_10052e48: the perk template's requirement check gives 2): the template's wear
/// criteria, a postfix list (operators 4 and, 3 or; comparisons 0 equal, 1 below, 2 above, 24 unequal, 22 all these bits;
/// 93 owns the perk). Other operators (one record has 108 with an item template) are not known here and fail.
pub fn meets(criteria: &[(u32, i32, u32)], stats: &BTreeMap<u32, i64>, owned: &HashSet<u32>) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    for &(stat, value, op) in criteria {
        let v = value as i64;
        let have = stats.get(&stat).copied().unwrap_or(0);
        let r = match op {
            3 | 4 => {
                let (Some(b), Some(a)) = (stack.pop(), stack.pop()) else { return false };
                if op == 4 { a && b } else { a || b }
            }
            0 => have == v,
            1 => have < v,
            2 => have > v,
            24 => have != v,
            22 => have & v == v,
            93 => owned.contains(&(value as u32)),
            _ => false,
        };
        stack.push(r);
    }
    stack.into_iter().all(|b| b)
}

/// A line of ranks (record indices, the first rank first).
pub type Line = Vec<usize>;

/// One rank's state as the window shows it (`PerkNode_t` +0x48 criteria met, +0x49 owned).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub meets: bool,
    pub owned: bool,
}

pub struct Model<'a> {
    pub catalog: &'a Catalog,
    pub owned: HashSet<u32>,
    pub stats: &'a BTreeMap<u32, i64>,
}

impl<'a> Model<'a> {
    pub fn new(catalog: &'a Catalog, owned: &[u32], stats: &'a BTreeMap<u32, i64>) -> Model<'a> {
        Model { catalog, owned: owned.iter().copied().collect(), stats }
    }

    fn stat(&self, k: u32) -> i64 {
        self.stats.get(&k).copied().unwrap_or(0)
    }

    pub fn status(&self, k: usize) -> Status {
        let r = &self.catalog.records[k];
        Status { meets: meets(&r.criteria, self.stats, &self.owned), owned: self.owned.contains(&r.id) }
    }

    /// `LoadPerks`: the three lists' lines (profession, group, general), each sorted as FUN_100643db.
    pub fn lists(&self) -> [Vec<Line>; 3] {
        let (expansion, breed, profession) = (self.stat(389), self.stat(4), self.stat(60));
        let mut order: Vec<usize> = (0..self.catalog.records.len()).filter(|&k| listed(self.catalog.records[k].flags, expansion, breed)).collect();
        order.sort_by_key(|&k| self.catalog.records[k].id);
        let shown: HashMap<u32, usize> = order.iter().map(|&k| (self.catalog.records[k].id, k)).collect();
        // (each listed rank's next: the last listed rank, in id order, whose rank before it is)
        let mut next: HashMap<usize, usize> = HashMap::new();
        let mut roots: Vec<usize> = Vec::new();
        for &k in &order {
            let r = &self.catalog.records[k];
            if r.prev == 0 {
                roots.push(k);
            } else if let Some(&parent) = shown.get(&r.prev) {
                next.insert(parent, k);
            }
        }
        let mut lists: [Vec<Line>; 3] = Default::default();
        for root in roots {
            let Some(list) = kind(self.catalog.records[root].professions, profession) else { continue };
            let mut line = vec![root];
            while let Some(&n) = next.get(line.last().unwrap()) {
                if line.contains(&n) {
                    break;
                }
                line.push(n);
            }
            lists[list].push(line);
        }
        for list in &mut lists {
            let mut keyed: Vec<(Status, String, Line)> = std::mem::take(list).into_iter().map(|l| (self.status(l[0]), self.catalog.records[l[0]].name.clone(), l)).collect();
            keyed.sort_by(|a, b| b.0.meets.cmp(&a.0.meets).then(b.0.owned.cmp(&a.0.owned)).then(a.1.cmp(&b.1)));
            *list = keyed.into_iter().map(|k| k.2).collect();
        }
        lists
    }

    /// `N3Msg_GetNumberOfUsedPerks` / `..AvailablePerks` / `..UsedAlienPerks` / `..AvailableAlienPerks` (Gamecode FUN_1005310d,
    /// FUN_10053140, FUN_100531b1, FUN_100531e7): the owned ranks without the flags 0xc2; the level (stat 54) over 10 below 200,
    /// less 180 from 200, less those; the owned alien ranks; the alien level (stat 169) less those.
    pub fn points(&self) -> (i64, i64, i64, i64) {
        let flags: Vec<u32> = self.owned.iter().filter_map(|&id| self.catalog.get(id)).map(|r| r.flags).collect();
        let used = flags.iter().filter(|f| *f & 0xc2 == 0).count() as i64;
        let alien = flags.iter().filter(|f| *f & 2 != 0).count() as i64;
        let level = self.stat(54);
        let earned = if level < 200 { level / 10 } else { level - 180 };
        (used, earned - used, alien, self.stat(169) - alien)
    }

    /// FUN_1005ff0a: a rank's name, the line's first rank's name (`N3Msg_GetPerkName`) and, with `rank`, its place from 1.
    pub fn name(&self, line: &Line, place: usize, rank: bool) -> String {
        let base = &self.catalog.records[line[0]].name;
        if rank { format!("{base} {}", place + 1) } else { base.clone() }
    }
}

/// What a click on a rank, or on the remove button, comes to: a message box, or a question that sends on Yes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    NoAlienPoints,
    NoPoints,
    Known,
    NotMet,
    NeedPrevious,
    Train { id: u32, name: String },
    NotOwned,
    Untrain { id: u32, name: String },
}

/// FUN_10061021: a click on rank `place` of `line`: no (alien) points left, already owned, criteria not met (or a special
/// perk), the rank before not owned, else the question whether to train it.
pub fn train(model: &Model, line: &Line, place: usize, available: i64, alien_available: i64) -> Option<Answer> {
    let mut before_owned = true;
    for (k, &r) in line.iter().enumerate() {
        let status = model.status(r);
        if k == place {
            let flags = model.catalog.records[r].flags;
            let alien = flags & 2 != 0;
            return Some(if alien && alien_available == 0 {
                Answer::NoAlienPoints
            } else if !alien && available == 0 {
                Answer::NoPoints
            } else if status.owned {
                Answer::Known
            } else if !status.meets || flags & 1 != 0 {
                Answer::NotMet
            } else if before_owned {
                Answer::Train { id: model.catalog.records[r].id, name: model.name(line, k, true) }
            } else {
                Answer::NeedPrevious
            });
        }
        before_owned = status.owned;
    }
    None
}

/// FUN_100616d2: the remove button on a line: its first rank not owned, else the question whether to untrain the last owned
/// rank of the run from it.
pub fn untrain(model: &Model, line: &Line) -> Answer {
    if !model.status(line[0]).owned {
        return Answer::NotOwned;
    }
    let mut last = 0;
    while last + 1 < line.len() && model.status(line[last + 1]).owned {
        last += 1;
    }
    Answer::Untrain { id: model.catalog.records[line[last]].id, name: model.name(line, last, true) }
}

/// How long untraining waits (FUN_10060d7e: 0x1c20 seconds; the questions' "%d hours" are given 2).
const UNTRAIN_WAIT: i64 = 7200;

/// FUN_10060d7e: with the last perk reset (`N3Msg_GetLastPerkResetTime`, stat 577) and the server time, the share of the wait
/// that has passed and the time left as `hh:mm:ss`; nothing when there was no reset or the wait is over.
pub fn untrain_wait(reset: i64, now: i64) -> Option<(f32, String)> {
    let since = now - reset;
    if reset == 0 || since > UNTRAIN_WAIT - 1 {
        return None;
    }
    let left = UNTRAIN_WAIT - since;
    Some((since as f32 / UNTRAIN_WAIT as f32, format!("{:02}:{:02}:{:02}", left / 3600, left % 3600 / 60, left % 3600 % 60)))
}

/// FUN_1005eab2 (not highlighted): a status icon's tint, owned, met, or neither.
pub fn tint(meets: bool, owned: bool) -> [u8; 3] {
    if owned {
        [0, 0x6e, 0]
    } else if meets {
        [100, 100, 100]
    } else {
        [0x96, 0, 0]
    }
}

/// A text of category 10000 by name (`LDBface::GetText(10000, name)`).
fn text(name: &str) -> String {
    crate::feedback::texts().and_then(|t| t.named(10000, name)).map(|s| s.replace("\r\n", "\n")).unwrap_or_else(|| name.to_string())
}

/// The 0xccffff of the headers and the point labels.
const LABEL: egui::Color32 = egui::Color32::from_rgb(0xcc, 0xff, 0xff);

struct Dialog {
    title: String,
    text: String,
    /// The rank to train (true) or untrain on Yes; a plain message box without.
    ask: Option<(u32, bool)>,
}

pub struct PerksWindow {
    pub open: bool,
    tab: usize,
    /// The chosen line of each tab, by its first rank's id (`PerkView_c` +0x128).
    chosen: [Option<u32>; 3],
    dialog: Option<Dialog>,
    /// +0x134: untraining waits (the remove button is off); +0x1c4: the timer has yet to show the end of a wait.
    waiting: bool,
    wait_shown: bool,
    /// The countdown's text and share (+0x14c, +0x1a4).
    wait: (String, f32),
    next_tick: Option<std::time::Instant>,
    /// FUN_10060b04: the points last shown and the levels then; the labels blink (+0x1bc, +0x1c0 half seconds) when a level
    /// change changes them.
    last_points: Option<(i64, i64, i64, i64)>,
    blink: (i32, i32),
    blink_on: bool,
    next_blink: Option<std::time::Instant>,
}

impl Default for PerksWindow {
    fn default() -> Self {
        PerksWindow { open: false, tab: crate::layout::tab("PerkWindowConfig").unwrap_or(0).min(2), chosen: [None; 3], dialog: None, waiting: false, wait_shown: true, wait: ("--:--:--".into(), 1.0), next_tick: None, last_points: None, blink: (0, 0), blink_on: true, next_blink: None }
    }
}

impl PerksWindow {
    /// Opens a tab (0 profession, 1 group, 2 general; the previews use it).
    pub fn select_tab(&mut self, tab: usize) {
        self.tab = tab.min(2);
    }

    /// The half-second timers: the untraining wait (FUN_10060d7e) and the points' blinking (FUN_10060b04, FUN_10060514).
    fn tick(&mut self, ctx: &egui::Context, stats: &BTreeMap<u32, i64>, server_offset: i64, available: i64, alien_available: i64) {
        let now = std::time::Instant::now();
        if self.next_tick.is_none_or(|t| now >= t) {
            self.next_tick = Some(now + std::time::Duration::from_millis(500));
            let server_now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64) + server_offset;
            match untrain_wait(stats.get(&577).copied().unwrap_or(0), server_now) {
                None => {
                    if self.wait_shown {
                        self.wait = ("--:--:--".into(), 1.0);
                        self.waiting = false;
                        self.wait_shown = false;
                    }
                }
                Some((share, left)) => {
                    self.wait = (left, share);
                    self.waiting = true;
                    self.wait_shown = true;
                }
            }
        }
        let level = (stats.get(&54).copied().unwrap_or(0), stats.get(&169).copied().unwrap_or(0));
        if let Some((last_available, last_alien, last_level, last_alien_level)) = self.last_points {
            if level.0 != last_level && available != last_available {
                self.blink.0 += 10;
            }
            if level.1 != last_alien_level && alien_available != last_alien {
                self.blink.1 += 10;
            }
        }
        self.last_points = Some((available, alien_available, level.0, level.1));
        if (self.blink.0 > 0 || self.blink.1 > 0) && self.next_blink.is_none_or(|t| now >= t) {
            if self.next_blink.is_some() {
                self.blink.0 = (self.blink.0 - 1).max(0);
                self.blink.1 = (self.blink.1 - 1).max(0);
                self.blink_on = !self.blink_on;
            }
            self.next_blink = Some(now + std::time::Duration::from_millis(500));
        }
        if self.blink == (0, 0) {
            self.blink_on = true;
            self.next_blink = None;
        }
        if self.next_blink.is_some() || self.waiting {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    /// The window; a rank to train (true) or untrain once its question was answered Yes.
    pub fn show(&mut self, ctx: &egui::Context, state: &PerkState, stats: &BTreeMap<u32, i64>, palette: &Palette) -> Option<(u32, bool)> {
        let model = Model::new(&state.catalog, &state.owned, stats);
        let lists = model.lists();
        let (used, available, alien_used, alien_available) = model.points();
        let expansion = stats.get(&389).copied().unwrap_or(0);
        self.tick(ctx, stats, state.server_offset, available, alien_available);

        let mut answer: Option<Answer> = None;
        let mut open = self.open;
        let tabs = [text("ProfessionPerks"), text("GroupPerks"), text("GeneralPerks")];
        let tab_names: Vec<&str> = tabs.iter().map(String::as_str).collect();
        let font = crate::font::verdana(ctx, 13.0);
        let skin = crate::skin::skin(ctx);
        let width_of = |s: &str| ctx.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font.clone(), egui::Color32::WHITE).size().x);
        // (GetListViewWidths: the widest line name and the widest rank name, each with 35 for the icon and the margins)
        let line_width = lists.iter().flatten().map(|l| width_of(&model.name(l, 0, false))).fold(0.0, f32::max) + 35.0;
        let rank_width = lists.iter().flatten().flat_map(|l| (0..l.len()).map(|k| width_of(&model.name(l, k, true))).collect::<Vec<_>>()).fold(0.0, f32::max) + 35.0;
        let row = 22.0;
        crate::chrome::window(ctx, palette, "Perks", &text("PerkWindow"), &mut open, |w| w.default_pos(crate::layout::pos("Perks", egui::pos2(200.0, 300.0))).default_size([400.0, 200.0]), |ui| {
            let mut tab = self.tab;
            crate::chrome::tab_view(ui, palette, &tab_names, &mut tab, |ui, tab| {
                let lines = &lists[tab];
                // (FUN_100607e3: the tab's chosen line, else its first)
                let chosen = self.chosen[tab].and_then(|id| lines.iter().position(|l| state.catalog.records[l[0]].id == id)).unwrap_or(0);
                self.chosen[tab] = lines.get(chosen).map(|l| state.catalog.records[l[0]].id);
                ui.horizontal_top(|ui| {
                    // the lines
                    ui.vertical(|ui| {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(text("PerkLines")).font(font.clone()).color(LABEL));
                        group(ui, egui::vec2(line_width + 22.0, (ui.available_height() - 6.0).max(row * 6.0)), |ui| {
                            crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt(("perk-lines", tab)).auto_shrink([false, false]), |ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                for (k, line) in lines.iter().enumerate() {
                                    let alien = state.catalog.records[line[0]].flags & 2 != 0;
                                    if perk_button(ui, skin.as_deref(), palette, &font, egui::vec2(line_width, row), &model.name(line, 0, false), model.status(line[0]), alien, k == chosen).clicked() {
                                        self.chosen[tab] = Some(state.catalog.records[line[0]].id);
                                    }
                                }
                            });
                        });
                    });
                    // the ranks, the points and the remove button
                    ui.vertical(|ui| {
                        ui.add_space(8.0);
                        let line = lines.get(chosen);
                        // (FUN_10060f2d: the header is the chosen line's first rank's name with its rank; "Perk" before)
                        let header = line.map_or_else(|| text("Perk"), |l| model.name(l, 0, true));
                        ui.label(egui::RichText::new(header).font(font.clone()).color(LABEL));
                        let per_column = line.map_or(1, |l| l.len().div_ceil(2)).max(1);
                        group(ui, egui::vec2(rank_width * 2.0 + 13.0, per_column as f32 * (row + 1.0) + 8.0), |ui| {
                            let Some(line) = line else { return };
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(5.0, 1.0);
                                for column in 0..2 {
                                    ui.vertical(|ui| {
                                        for k in column * per_column..((column + 1) * per_column).min(line.len()) {
                                            let r = line[k];
                                            let status = model.status(r);
                                            let alien = state.catalog.records[r].flags & 2 != 0;
                                            let name = model.name(line, k, true);
                                            let response = perk_button(ui, skin.as_deref(), palette, &font, egui::vec2(rank_width, row), &name, status, alien, status.owned);
                                            let response = response.on_hover_ui(|ui| tip(ui, palette, &state.catalog.records[r], &name));
                                            if response.clicked() {
                                                answer = train(&model, line, k, available, alien_available);
                                            }
                                        }
                                    });
                                }
                            });
                        });
                        let shown = |n: i32| n <= 0 || self.blink_on;
                        let label_width = rank_width * 2.0 + 13.0;
                        if expansion & 2 != 0 {
                            points_label(ui, &font, label_width, &text("UsedPerkPoints"), used, shown(self.blink.0));
                            points_label(ui, &font, label_width, &text("AvailablePerkPoints"), available, shown(self.blink.0));
                        }
                        if expansion & 0x18 != 0 {
                            points_label(ui, &font, label_width, &text("UsedAlienPerkPoints"), alien_used, shown(self.blink.1));
                            points_label(ui, &font, label_width, &text("AvailableAlienPerkPoints"), alien_available, shown(self.blink.1));
                        }
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.add_space(5.0);
                            // (FUN_10060796: on while not waiting and any perk point is used)
                            let enabled = !self.waiting && (used > 0 || alien_used > 0);
                            let info = text("UntrainPerkInfo").replace("%d", "2");
                            let remove = crate::chrome::button(ui, palette, &text("RemovePerk"), enabled).on_hover_text(format!("{}\n{info}", text("UntrainPerk")));
                            if remove.clicked()
                                && let Some(line) = line
                            {
                                answer = Some(untrain(&model, line));
                            }
                            ui.add_space(10.0);
                            wait_view(ui, &font, &self.wait.0, self.wait.1).on_hover_text(format!("{}\n{}", text("UntrainPerkTimeout"), text("UntrainPerkTimeoutInfo")));
                        });
                    });
                });
            });
            // (`PerkWindowConfig`'s `SelectedTab`: FUN_10064a67 saves it when the window goes, FUN_10063ff8 selects it)
            if self.tab != tab {
                self.tab = tab;
                crate::layout::set_tab("PerkWindowConfig", tab);
            }
        });
        self.open = open;
        if let Some(a) = answer {
            self.dialog = Some(dialog(a));
        }
        self.show_dialog(ctx, palette)
    }

    /// The message box or question open (`DialogBox_c`), and its answer.
    fn show_dialog(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<(u32, bool)> {
        let dialog = self.dialog.as_ref()?;
        let mut keep = true;
        let mut result = None;
        crate::chrome::window(ctx, palette, "perk-dialog", &dialog.title, &mut keep, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).default_width(300.0), |ui| {
            ui.set_max_width(300.0);
            ui.label(egui::RichText::new(&dialog.text).color(crate::chrome::rgb(palette.text)));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if dialog.ask.is_some() {
                    if crate::chrome::button(ui, palette, &text("MsgBox_Yes"), true).clicked() {
                        result = Some(true);
                    }
                    if crate::chrome::button(ui, palette, &text("MsgBox_No"), true).clicked() {
                        result = Some(false);
                    }
                } else if crate::chrome::button(ui, palette, &text("MsgBox_OK"), true).clicked() {
                    result = Some(false);
                }
            });
        });
        let mut asked = None;
        if result == Some(true)
            && let Some((id, learn)) = dialog.ask
        {
            asked = Some((id, learn));
            // (FUN_1006089f: untraining turns the remove button off at once)
            if !learn {
                self.waiting = true;
            }
        }
        if result.is_some() || !keep {
            self.dialog = None;
        }
        asked
    }
}

/// The message boxes of FUN_10061021 and FUN_100616d2 (title, text; the questions with Yes and No, the rest with OK).
fn dialog(a: Answer) -> Dialog {
    let info = text("Info");
    let plain = |title: String, body: &str| Dialog { title, text: text(body), ask: None };
    match a {
        Answer::NoAlienPoints => plain(text("NoAPerkPts"), "HowToGetAPerks"),
        Answer::NoPoints => plain(text("NoPerkPts"), "HowToGetPerks"),
        Answer::Known => plain(info, "YouKnowThisPerk"),
        Answer::NotMet => plain(info, "DoNotMeetPerkReq"),
        Answer::NeedPrevious => plain(info, "MustTrainPrecursoryPerk"),
        Answer::NotOwned => plain(info, "MustKnowPerk2Untrain"),
        Answer::Train { id, name } => Dialog { title: info, text: text("ReallyTrainPerk").replacen("%s", &name, 1).replacen("%d", "2", 1), ask: Some((id, true)) },
        Answer::Untrain { id, name } => Dialog { title: info, text: text("ReallyRemovePerk").replacen("%s", &name, 1).replacen("%d", "2", 1), ask: Some((id, false)) },
    }
}

/// A `perk_group` (FUN_10062f83): a `BorderView_c` in black at the interface's layer-2 alpha round a list.
fn group(ui: &mut egui::Ui, size: egui::Vec2, add: impl FnOnce(&mut egui::Ui)) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(rect, 2.0, egui::Color32::from_black_alpha(110));
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(4.0)).layout(egui::Layout::top_down(egui::Align::Min)));
    add(&mut inner);
}

/// A `PerkLineView_c` row: a toggle button (down when `down`) with the status icon and the label.
#[allow(clippy::too_many_arguments)]
fn perk_button(ui: &mut egui::Ui, skin: Option<&crate::skin::Skin>, palette: &Palette, font: &egui::FontId, size: egui::Vec2, label: &str, status: Status, alien: bool, down: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter();
    let colour = crate::chrome::rgb(if down { palette.selected } else { crate::chrome::code1(palette) });
    if !skin.is_some_and(|s| s.nine_slice(painter, if down { "GFX_GUI_BORDER04" } else { "GFX_GUI_BORDER01" }, rect, colour, true)) {
        painter.rect_filled(rect, 2.0, if down { egui::Color32::from_white_alpha(40) } else { egui::Color32::from_white_alpha(12) });
    }
    if response.hovered() && !skin.is_some_and(|s| s.nine_slice(painter, "GFX_GUI_BORDER03", rect, crate::chrome::rgb(palette.hover), false)) {
        painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, crate::chrome::rgb(palette.hover)), egui::StrokeKind::Inside);
    }
    let t = tint(status.meets, status.owned);
    let icon = egui::Rect::from_center_size(egui::pos2(rect.left() + 12.0, rect.center().y), egui::vec2(10.0, 10.0));
    painter.rect_filled(icon, 2.0, egui::Color32::from_rgb(t[0], t[1], t[2]));
    let text_colour = if alien { egui::Color32::from_rgb(0x30, 0xff, 0x30) } else { colour };
    painter.text(egui::pos2(rect.left() + 22.0, rect.center().y), egui::Align2::LEFT_CENTER, label, font.clone(), text_colour);
    response
}

/// A `perk_label` (FUN_10062d0b): the label, a spacer, the value; both hidden while blinking off.
fn points_label(ui: &mut egui::Ui, font: &egui::FontId, width: f32, label: &str, value: i64, shown: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 18.0), egui::Sense::hover());
    if !shown {
        return;
    }
    let painter = ui.painter();
    painter.text(egui::pos2(rect.left() + 10.0, rect.center().y), egui::Align2::LEFT_CENTER, label, font.clone(), LABEL);
    painter.text(egui::pos2(rect.right() - 10.0, rect.center().y), egui::Align2::RIGHT_CENTER, value.to_string(), font.clone(), LABEL);
}

/// The untraining timer (FUN_10060920 with its text, FUN_1006063d): the share of the wait passed under the time left.
fn wait_view(ui: &mut egui::Ui, font: &egui::FontId, left: &str, share: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(90.0, 20.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(140));
    let fill = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * share.clamp(0.0, 1.0), rect.height()));
    painter.rect_filled(fill, 2.0, egui::Color32::from_rgba_unmultiplied(60, 120, 160, 120));
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, left, font.clone(), LABEL);
    response
}

/// FUN_1005f908: a rank's tip, its name with the rank over "Requirements_NL" and the use-by string (past its first six
/// characters, the "Wear:" heading, unless it is no more than that), "Modifies:_NL" and the modifiers, and the description.
fn tip(ui: &mut egui::Ui, palette: &Palette, r: &PerkRecord, name: &str) {
    ui.set_max_width(340.0);
    ui.label(egui::RichText::new(name).strong().color(crate::chrome::rgb(palette.accent)));
    let mut body = String::new();
    if let Some(u) = &r.use_by {
        body += &text("Requirements_NL");
        let wear = crate::feedback::texts().and_then(|t| t.named(1005, "Wear:")).unwrap_or("Wear:");
        body += if u == wear { u.as_str() } else { u.get(6..).unwrap_or("") };
    }
    if !r.modifiers.is_empty() {
        body += "\n";
        body += &text("Modifies:_NL");
        body += &r.modifiers;
    }
    if !r.description.is_empty() {
        body += "\n";
        body += &r.description;
    }
    let body = body.trim().replace("\r\n", "\n").replace('\n', "<br>");
    crate::gui::controls::markup(ui, palette, &body, crate::gui::font::FontId::NORMAL, crate::chrome::rgb(palette.text));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u32, prev: u32, flags: u32, professions: u32, name: &str, criteria: Vec<(u32, i32, u32)>) -> PerkRecord {
        PerkRecord { id, prev, flags, professions, total: 0, name: name.into(), description: String::new(), criteria, use_by: None, modifiers: String::new() }
    }

    fn sample() -> Catalog {
        Catalog::from_records(vec![
            // a general line of three: level 10, 30, 80 and the rank before
            record(130, 0, 0, 0, "Ambidextrous", vec![(54, 9, 2)]),
            record(131, 130, 0, 0, "Ambidextrous", vec![(54, 29, 2), (0, 130, 93), (0, 0, 4)]),
            record(132, 131, 0, 0, "Ambidextrous", vec![(54, 79, 2), (0, 131, 93), (0, 0, 4)]),
            // a soldier's own line (profession 1) and a group line (soldier or enforcer)
            record(140, 0, 0, 1 << 1, "Zealot", vec![(60, 1, 0)]),
            record(150, 0, 0, (1 << 1) | (1 << 9), "Brawler", vec![(54, 999, 2)]),
            // an agent's, an atrox-only one (flag bit 5), research (0x40) and an alien one (bit 1)
            record(160, 0, 0, 1 << 5, "Assassin", vec![]),
            record(170, 0, 0x20, 0, "Atrox Genome", vec![]),
            record(180, 0, 0x40, 0, "Research", vec![]),
            record(190, 0, 0x2, 0, "Alien Tech", vec![]),
        ])
    }

    fn stats(level: i64, expansion: i64) -> BTreeMap<u32, i64> {
        BTreeMap::from([(54, level), (60, 1), (4, 1), (389, expansion), (169, 3)])
    }

    #[test]
    fn the_lists_are_by_kind_and_sorted_by_status_then_name() {
        let catalog = sample();
        let names = |l: &Vec<Line>| l.iter().map(|l| catalog.records[l[0]].name.clone()).collect::<Vec<_>>();
        let s = stats(50, 2);
        let lists = Model::new(&catalog, &[130], &s).lists();
        assert_eq!(names(&lists[0]), ["Zealot"]);
        assert_eq!(names(&lists[1]), ["Brawler"]);
        // (no atrox genome for a solitus, no research, no alien perk without Alien Invasion)
        assert_eq!(names(&lists[2]), ["Ambidextrous"]);
        assert_eq!(lists[2][0].len(), 3);
        // with Alien Invasion the alien line shows, after the met and owned one
        let s = stats(50, 2 | 8);
        assert_eq!(names(&Model::new(&catalog, &[130], &s).lists()[2]), ["Ambidextrous", "Alien Tech"]);
        // and the unmet before the met one when nothing is owned: met first
        assert_eq!(names(&Model::new(&catalog, &[], &stats(5, 2 | 8)).lists()[2]), ["Alien Tech", "Ambidextrous"]);
        // nothing at all without Shadowlands
        let s = stats(50, 0);
        assert!(Model::new(&catalog, &[], &s).lists().iter().all(Vec::is_empty));
    }

    #[test]
    fn criteria_and_points() {
        let catalog = sample();
        let s = stats(50, 2);
        let model = Model::new(&catalog, &[130], &s);
        assert_eq!(model.status(1), Status { meets: true, owned: false });
        assert_eq!(model.status(2), Status { meets: false, owned: false });
        // level 50: 5 points, one used; alien level 3
        assert_eq!(model.points(), (1, 4, 0, 3));
        // level 210: 30 points; research does not count
        let s = stats(210, 2);
        assert_eq!(Model::new(&catalog, &[130, 131, 180], &s).points(), (2, 28, 0, 3));
    }

    #[test]
    fn training_and_untraining_ask_as_the_original() {
        let catalog = sample();
        let s = stats(50, 2);
        let model = Model::new(&catalog, &[130], &s);
        let line = model.lists()[2][0].clone();
        assert_eq!(train(&model, &line, 0, 4, 0), Some(Answer::Known));
        assert_eq!(train(&model, &line, 1, 4, 0), Some(Answer::Train { id: 131, name: "Ambidextrous 2".into() }));
        assert_eq!(train(&model, &line, 2, 4, 0), Some(Answer::NotMet));
        assert_eq!(train(&model, &line, 1, 0, 0), Some(Answer::NoPoints));
        assert_eq!(untrain(&model, &line), Answer::Untrain { id: 130, name: "Ambidextrous 1".into() });
        let s = stats(100, 2);
        let model = Model::new(&catalog, &[130, 131], &s);
        assert_eq!(untrain(&model, &line), Answer::Untrain { id: 131, name: "Ambidextrous 2".into() });
        assert_eq!(untrain(&Model::new(&catalog, &[], &s), &line), Answer::NotOwned);
        // (a rank whose criteria are met but whose rank before is not owned)
        let open = Catalog::from_records(vec![record(1, 0, 0, 0, "A", vec![]), record(2, 1, 0, 0, "A", vec![])]);
        let model = Model::new(&open, &[], &s);
        assert_eq!(train(&model, &vec![0, 1], 1, 4, 0), Some(Answer::NeedPrevious));
    }

    #[test]
    fn the_untraining_wait() {
        assert_eq!(untrain_wait(0, 1000), None);
        assert_eq!(untrain_wait(1000, 1000 + 7200), None);
        assert_eq!(untrain_wait(1000, 1000 + 7199).map(|w| w.1), Some("00:00:01".into()));
        assert_eq!(untrain_wait(1000, 1000 + 61).map(|w| w.1), Some("01:58:59".into()));
    }

    #[test]
    fn status_tints() {
        assert_eq!(tint(true, true), [0, 0x6e, 0]);
        assert_eq!(tint(true, false), [100, 100, 100]);
        assert_eq!(tint(false, false), [0x96, 0, 0]);
    }

    /// `MAO_GAME`: the records as the perk manager holds them (`docs/perk_records.json`: 2063, 100 to 109 a line).
    #[test]
    fn the_catalog_reads_the_records() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let c = catalog(&db);
        assert_eq!(c.records.len(), 2063);
        let r = c.get(101).unwrap();
        assert_eq!((r.prev, r.flags, r.professions, r.name.as_str()), (100, 0, 0x80, "Accumulator"));
        assert_eq!(c.get(1000).map(|r| r.flags), Some(2));
        assert!(!r.criteria.is_empty());
    }
}
