//! The research window, GUI.dll's `ResearchWindow_c` (FUN_100f750e, `research_window`, "#WindowResearch", 650 by 520 at
//! first, help "The Research window.html"): three tabs over the skin's views, "Allocate Research" (`Views/Research.xml`),
//! "Available Research" (`Views/ResearchDetails.xml`) and "Finished Research" (`Views/ResearchFinished.xml`).
//!
//! Research ranks are perk records (`perks.rs`) with flag 0x40 (personal) or 0x80 (global), the research amount at byte 32
//! (the perk manager's entry +0x18). Gamecode keeps how much is left of each:
//!
//! - personal research (FUN_10053210's map): the world entry's `Perks` list (a research rank with what is left in its last
//!   field; 0 is finished) and the server's `435f7023` (aosharp's "PerkUpdate", `PerkUpdateIIR`'s apply FUN_10063e88: the
//!   rank, the goal, which it sets as stat 265, and what is left, held to 0 or more by FUN_10052d86);
//! - global research, a map for each side (stat 33 below 3, FUN_1002be1a; a value below 0 is the rank's whole amount):
//!   `ResearchUpdate` (`253d0240`), a version byte, then each global rank with what is left for side 0, 1 and 2, then a 0
//!   (the order FUN_1003a88c writes them); the window asks for it on opening (`N3Msg_UpdateGlobalResearch`, `3115534d`).
//!
//! A rank's progress (`N3Msg_GetPerkProgress` 0x1002781f) is `(amount - left) / amount` from the global map for a global
//! rank, the personal one for a personal rank, 0 without an entry. Its name (`N3Msg_GetPerkName(id, true)`, FUN_10053bb2)
//! is its template's with its place in the line from 1. The lists:
//!
//! - the personal goals (`N3Msg_PersonalResearchGoals`, FUN_10053f71): each personal rank not finished whose rank before
//!   (if any) is, for the character's profession (FUN_10052f68: the wear criteria's "profession equals" lines; none is any
//!   profession), with whether its criteria are met (FUN_10052e48);
//! - the global goals (`N3Msg_GlobalResearchGoals`, FUN_1002bec7): each global rank with some left whose rank before is
//!   finished;
//! - the finished ones (`N3Msg_GetCompletedPersonalResearchGoals` 0x10029e98, `...GlobalResearchGoals` 0x1002868e): the
//!   personal ranks finished, a rank left out when the next finished one is of the same name; the global ones with none
//!   left;
//! - a line (`N3Msg_GetPerkLine`, FUN_10053e05): the ranks of the same name before and after.
//!
//! The tabs:
//!
//! - Allocate: the sliders `PersonalResearchSlider` / `GlobalResearchSlider` (from stats 263 / 264); moving one (FUN_100f719a,
//!   FUN_100f7357) labels it "n%", and when the two would pass 100 sets the other to the rest and sends it, then sends its
//!   own (`N3Msg_SetStat` 263 / 264, a message for each step as the captures show). "Started Personal Research"
//!   (FUN_100f68bb) has a row for each personal goal, shown once it has progress; "Current Global Research Goal"
//!   (FUN_100f5c7c) one for each global goal, shown for the one chosen (stat 266).
//! - Available: a radio button with a row for each personal goal (disabled when its criteria are not met), set to stat 265,
//!   choosing one sends `SetStat` 265 (LAB_100f5033); the global goals the same with stat 266 (FUN_100f5382). An empty
//!   list has an empty text instead.
//! - Finished: a row for each finished rank, personal and global (FUN_100f5f14, FUN_100f630d).
//!
//! A row (FUN_100f56b6) is a small blue bar of its progress (`PowerbarView_c` of bitmaps 0xe0 / 0xe1) and its name.
//! The pointer on a row (`ResearchGoalEntry`'s mouse slot FUN_100f5546) shows its description
//! (`N3Msg_GetPerkDescription`: the template's description, "Modifiers" and "Requirements") in the "info" texts and
//! takes it away on leaving; a click on a goal or a finished rank (LAB_100f5b7d, FUN_100f5998) fills that tab's
//! "PerkLine" with a row for each rank of its line. Stats 53 and 60 changing and a personal rank finishing build the
//! lists again (LAB_100f712a, LAB_100f7151, FUN_100f70b0); here they are built again whenever what they show changed.
//!
//! The control center's Char menu "Research" (`research_window`, with stat 389 bit 32) or `/open research` opens it;
//! Escape closes it by `esc_research`; `MAO_UI_PREVIEW=research` (`MAO_RESEARCH_TAB` 0 to 2) shows it on the 2026-10-30
//! capture's data.
//!
//! Left out: the "got_tech" distributed value the window clears on opening (the control center's research blink), the
//! window's `ResearchConfig` placement, and the requirements text is `N3Msg_GetUseByString`'s rather than
//! `N3Msg_GetCriteriaString`'s. Whether the server answers `SetStat` 265 / 266 with the stat was not captured; the window
//! keeps its own choice until the stat changes.

use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;

use crate::gui::view::{Id, Tree};
use crate::perks::{Catalog, PerkRecord, PerkState};
use crate::theme::Palette;

const PERSONAL: u32 = 0x40;
const GLOBAL: u32 = 0x80;

/// What the server told of the research: what is left of each personal rank, and of each global rank for side 0 to 2.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Research {
    pub personal: BTreeMap<u32, i64>,
    pub global: BTreeMap<u32, [i64; 3]>,
    /// Steps with each change.
    pub version: u64,
}

static DATA: Mutex<Research> = Mutex::new(Research { personal: BTreeMap::new(), global: BTreeMap::new(), version: 0 });

fn data() -> std::sync::MutexGuard<'static, Research> {
    DATA.lock().unwrap_or_else(|e| e.into_inner())
}

/// The world entry's `Perks` (the id and the last field, what is left): the personal map anew.
pub fn entered(perks: impl IntoIterator<Item = (u32, i64)>) {
    let mut d = data();
    d.personal = perks.into_iter().collect();
    d.version += 1;
}

/// `ResearchUpdate`'s lines.
pub fn set_global(lines: Vec<(u32, [i64; 3])>) {
    let mut d = data();
    d.global.extend(lines);
    d.version += 1;
}

/// `435f7023`: what is left of a personal rank (FUN_10052d86 holds it to 0 or more).
pub fn set_personal(id: u32, left: i64) {
    let mut d = data();
    d.personal.insert(id, left.max(0));
    d.version += 1;
}

pub fn snapshot() -> Research {
    data().clone()
}

/// `ResearchUpdate` (`253d0240`, after the message's leading byte): the version byte, then 16 bytes a line (the rank, what
/// is left for side 0, 1 and 2), then an int.
pub fn parse_update(data: &[u8]) -> Option<Vec<(u32, [i64; 3])>> {
    let body = data.get(2..)?;
    let count = body.len().checked_sub(4)? / 16;
    let int = |at: usize| i32::from_be_bytes(body[at..at + 4].try_into().unwrap());
    Some((0..count).map(|k| (int(k * 16) as u32, [int(k * 16 + 4) as i64, int(k * 16 + 8) as i64, int(k * 16 + 12) as i64])).collect())
}

/// `435f7023` (after the leading byte): the rank, the goal (stat 265) and what is left.
pub fn parse_progress(data: &[u8]) -> Option<(u32, u32, i64)> {
    let int = |at: usize| data.get(at..at + 4).map(|b| i32::from_be_bytes(b.try_into().unwrap()));
    Some((int(1)? as u32, int(5)? as u32, int(9)? as i64))
}

/// What the window asks the connection to send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// `3115534d`.
    Request,
    /// `SetStat` of a stat and a value.
    SetStat(u32, i32),
}

static COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());

pub fn take_commands() -> Vec<Command> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn send(c: Command) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(c);
}

/// The research as the window reads it.
pub struct Model<'a> {
    pub catalog: &'a Catalog,
    pub research: &'a Research,
    pub stats: &'a BTreeMap<u32, i64>,
    pub owned: HashSet<u32>,
}

impl<'a> Model<'a> {
    pub fn new(catalog: &'a Catalog, research: &'a Research, stats: &'a BTreeMap<u32, i64>, owned: &[u32]) -> Model<'a> {
        Model { catalog, research, stats, owned: owned.iter().copied().collect() }
    }

    fn stat(&self, s: u32) -> i64 {
        self.stats.get(&s).copied().unwrap_or(0)
    }

    /// FUN_1002be1a: what is left of a global rank for the character's side (stat 33), the whole amount when below 0.
    pub fn global_left(&self, id: u32) -> Option<i64> {
        let side = self.stat(33);
        if !(0..3).contains(&side) {
            return None;
        }
        let left = self.research.global.get(&id)?[side as usize];
        Some(if left < 0 { self.catalog.get(id).map_or(0, |r| r.total as i64) } else { left })
    }

    /// FUN_10053210: what is left of a personal rank.
    pub fn personal_left(&self, id: u32) -> Option<i64> {
        self.research.personal.get(&id).copied()
    }

    /// `N3Msg_GetPerkProgress`.
    pub fn progress(&self, id: u32) -> f32 {
        let Some(r) = self.catalog.get(id) else { return 0.0 };
        let left = if r.flags & GLOBAL != 0 {
            self.global_left(id)
        } else if r.flags & PERSONAL != 0 {
            self.personal_left(id)
        } else {
            return 0.0;
        };
        match left {
            Some(left) if r.total > 0 => (r.total as i64 - left) as f32 / r.total as f32,
            _ => 0.0,
        }
    }

    /// `N3Msg_GetPerkName(id, true)`: the template's name and the rank's place in its line from 1.
    pub fn name(&self, id: u32) -> String {
        let Some(r) = self.catalog.get(id) else { return format!("Missing perk: {id}") };
        let mut place = 1;
        let mut at = r;
        while at.prev != 0 {
            match self.catalog.get(at.prev) {
                Some(p) => {
                    place += 1;
                    at = p;
                }
                None => break,
            }
        }
        format!("{} {place}", r.name)
    }

    /// FUN_10052f68: the wear criteria's "profession equals" lines name the character's profession (stat 60), or there
    /// are none.
    fn for_profession(&self, r: &PerkRecord) -> bool {
        let profession = self.stat(60);
        let mut ok = true;
        for &(stat, value, op) in &r.criteria {
            if op == 0 && stat == 60 {
                if value as i64 == profession {
                    return true;
                }
                ok = false;
            }
        }
        ok
    }

    /// `N3Msg_PersonalResearchGoals`: (rank, criteria met).
    pub fn personal_goals(&self) -> Vec<(u32, bool)> {
        let done = |id: u32| self.personal_left(id) == Some(0);
        self.catalog
            .records
            .iter()
            .filter(|r| r.flags & PERSONAL != 0 && !done(r.id) && (r.prev == 0 || done(r.prev)) && self.for_profession(r))
            .map(|r| (r.id, crate::perks::meets(&r.criteria, self.stats, &self.owned)))
            .collect()
    }

    /// `N3Msg_GlobalResearchGoals`.
    pub fn global_goals(&self) -> Vec<u32> {
        self.catalog
            .records
            .iter()
            .filter(|r| r.flags & GLOBAL != 0 && self.global_left(r.id).is_some_and(|l| l > 0) && (r.prev == 0 || self.global_left(r.prev) == Some(0)))
            .map(|r| r.id)
            .collect()
    }

    /// `N3Msg_GetCompletedGlobalResearchGoals`.
    pub fn finished_global(&self) -> Vec<u32> {
        self.catalog.records.iter().filter(|r| r.flags & GLOBAL != 0 && self.global_left(r.id) == Some(0)).map(|r| r.id).collect()
    }

    /// `N3Msg_GetCompletedPersonalResearchGoals`: the finished personal ranks (by id), a rank left out when a finished rank
    /// after it is of the same name.
    pub fn finished_personal(&self) -> Vec<u32> {
        let mut done: BTreeMap<u32, u32> = BTreeMap::new();
        for (&id, &left) in &self.research.personal {
            if let Some(r) = self.catalog.get(id).filter(|r| left == 0 && r.flags & PERSONAL != 0) {
                done.insert(id, r.prev);
            }
        }
        let template_name = |id: u32| self.catalog.get(id).map(|r| r.name.clone());
        let pairs: Vec<(u32, u32)> = done.iter().map(|(&a, &b)| (a, b)).collect();
        for (id, prev) in pairs {
            if id != prev && done.contains_key(&prev) && template_name(prev) == template_name(id) {
                done.remove(&prev);
            }
        }
        done.into_keys().collect()
    }

    /// `N3Msg_GetPerkLine`: the ranks of the same name before and after, the first first.
    pub fn line(&self, id: u32) -> Vec<u32> {
        let Some(r) = self.catalog.get(id) else { return Vec::new() };
        let mut first = r;
        while let Some(p) = self.catalog.get(first.prev).filter(|p| first.prev != 0 && p.name == r.name) {
            first = p;
        }
        let mut line = vec![first.id];
        loop {
            let last = *line.last().unwrap();
            match self.catalog.records.iter().find(|n| n.prev == last && n.name == r.name && !line.contains(&n.id)) {
                Some(n) => line.push(n.id),
                None => break,
            }
        }
        line
    }

    /// `N3Msg_GetPerkDescription`: the description, the modifiers (lists 14 and 0) and the requirements.
    pub fn description(&self, id: u32) -> String {
        let Some(r) = self.catalog.get(id) else { return String::new() };
        format!("{}\n<font color=yellow>Modifiers</font>:\n{}\n<font color=yellow>Requirements</font>:\n{}", r.description, r.modifiers, r.use_by.clone().unwrap_or_default())
    }
}

/// The tabs' names (FUN_100f750e's `AppendTab`s).
const TABS: [&str; 3] = ["Allocate Research", "Available Research", "Finished Research"];
const FILES: [&str; 3] = ["Research.xml", "ResearchDetails.xml", "ResearchFinished.xml"];

pub struct ResearchWindow {
    pub open: bool,
    was_open: bool,
    tab: usize,
    /// The sliders' values (stats 263 and 264 when the window opened).
    personal: i32,
    global: i32,
    /// The goals chosen (stats 265 and 266, or the window's own choice since): (the stat last seen, the value).
    goals: [(Option<i64>, i64); 2],
    /// The line shown in the Available and Finished tabs' "PerkLine".
    lines: [Option<u32>; 2],
    info: String,
    /// How many personal ranks were finished when the lists were last built.
    finished: usize,
    trees: Option<Vec<(Tree, Id)>>,
    built: Option<(u64, Vec<i64>, usize, [i64; 2], [Option<u32>; 2])>,
}

impl Default for ResearchWindow {
    fn default() -> ResearchWindow {
        ResearchWindow { open: false, was_open: false, tab: 0, personal: 0, global: 0, goals: [(None, 0); 2], lines: [None; 2], info: String::new(), finished: 0, trees: None, built: None }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn add_xml(tree: &mut Tree, parent: Id, xml: &str) -> Option<Id> {
    let doc = crate::gui::xml::parse(xml);
    let id = crate::gui::create(tree, doc.first()?)?;
    tree.add_child(parent, id);
    Some(id)
}

/// FUN_100f56b6: a row of a rank's progress bar and name.
fn row(tree: &mut Tree, parent: Id, name: &str, label: &str, progress: f32) -> Option<Id> {
    let xml = format!(
        "<View name=\"{name}\" view_layout=\"horizontal\" layout_borders=\"Rect(8,2,0,2)\"><PowerBar name=\"powerbar_view\" bg_gfx=\"GFX_GUI_HOR_BAR_SMALL_EMPTY\" full_gfx=\"GFX_GUI_HOR_BAR_SMALL_BLUE\"/><TextView name=\"label\" value=\"{}\" layout_borders=\"Rect(5,1,1,1)\"/><HLayoutSpacer/></View>",
        escape(label)
    );
    let doc = crate::gui::xml::parse(&xml);
    let id = crate::gui::create(tree, doc.first()?)?;
    if let Some(bar) = tree.find(id, "powerbar_view")
        && let Some(b) = tree.control_mut::<crate::gui::powerbar::PowerBar>(bar)
    {
        b.set_value(progress);
    }
    tree.add_child(parent, id);
    Some(id)
}

/// FUN_100f6702: a radio button of the rank (its value) with the row as its label.
fn radio(tree: &mut Tree, group: Id, model: &Model, id: u32, enabled: bool) {
    let Some(button) = add_xml(tree, group, &format!("<RadioButton name=\"radio:{id}\" value=\"{id}\"/>")) else { return };
    // (the row in the label's place)
    if row(tree, button, &format!("goal:{id}"), &model.name(id), model.progress(id)).is_some() {
        let children = &mut tree.get_mut(button).children;
        if children.len() == 3 {
            children.remove(1);
        }
    }
    tree.get_mut(button).enabled = enabled;
}

fn clear(tree: &mut Tree, root: Id, name: &str) -> Option<Id> {
    let id = tree.find(root, name)?;
    // (`DeleteAllChildren`, then the rows under a `VLayoutNode`)
    tree.get_mut(id).children.clear();
    tree.get_mut(id).layout = crate::gui::view::Layout::Vertical;
    Some(id)
}

fn spacer(tree: &mut Tree, parent: Id, vertical: bool) {
    add_xml(tree, parent, if vertical { "<VLayoutSpacer/>" } else { "<HLayoutSpacer/>" });
}

fn set_text(tree: &mut Tree, root: Id, name: &str, text: &str) {
    if let Some(id) = tree.find(root, name)
        && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id)
        && t.text != text
    {
        t.set_text(text);
        tree.relayout = true;
    }
}

/// The rank a view is in the row of: (its id, whether a goal's or finished rank's row rather than a line's).
fn row_of(tree: &Tree, mut id: Id) -> Option<(u32, bool)> {
    loop {
        let name = &tree.get(id).name;
        if let Some(v) = name.strip_prefix("goal:").or_else(|| name.strip_prefix("done:")) {
            return v.parse().ok().map(|v| (v, true));
        }
        if let Some(v) = name.strip_prefix("rank:") {
            return v.parse().ok().map(|v| (v, false));
        }
        id = tree.get(id).parent?;
    }
}

impl ResearchWindow {
    /// The goal chosen: the stat once it changes, else the window's own choice.
    fn goal(&mut self, k: usize, stats: &BTreeMap<u32, i64>) -> i64 {
        let seen = stats.get(&(265 + k as u32)).copied();
        if seen != self.goals[k].0 {
            self.goals[k] = (seen, seen.unwrap_or(0));
        }
        self.goals[k].1
    }

    /// Opens a tab (the preview uses it).
    pub fn select_tab(&mut self, tab: usize) {
        self.tab = tab.min(2);
    }

    /// FUN_100f719a / FUN_100f7357: a slider moved to `value`; the other is held so the two make 100 at most.
    fn slide(&mut self, global: bool, value: i32) {
        let (own, other, own_stat, other_stat) = if global { (&mut self.global, &mut self.personal, 264, 263) } else { (&mut self.personal, &mut self.global, 263, 264) };
        *own = value;
        if 100 < *other + value {
            *other = 100 - value;
            send(Command::SetStat(other_stat, *other));
        }
        send(Command::SetStat(own_stat, value));
    }

    fn build(&self, ctx: &egui::Context, model: &Model) -> Option<Vec<(Tree, Id)>> {
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        let mut trees = Vec::new();
        for file in FILES {
            let (mut tree, root) = crate::gui::load_view(&dirs, file, "")?;
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            trees.push((tree, root));
        }
        let personal = model.personal_goals();
        let global = model.global_goals();
        let global_goal = self.goals[1].1;
        // Allocate: the sliders, the started personal goals and the global goal chosen.
        {
            let (tree, root) = &mut trees[0];
            let root = *root;
            for (name, v) in [("PersonalResearchSlider", self.personal), ("GlobalResearchSlider", self.global)] {
                if let Some(id) = tree.find(root, name)
                    && let Some(s) = tree.control_mut::<crate::gui::controls::Slider>(id)
                {
                    s.set(v as f64);
                }
            }
            if let Some(list) = clear(tree, root, "StartedPersonal") {
                for &(id, _) in &personal {
                    let progress = model.progress(id);
                    if let Some(r) = row(tree, list, &format!("goal:{id}"), &model.name(id), progress)
                        && progress == 0.0
                    {
                        tree.get_mut(r).hidden += 1;
                    }
                }
                spacer(tree, list, true);
            }
            if let Some(list) = clear(tree, root, "StartedGlobal") {
                for &id in &global {
                    if let Some(r) = row(tree, list, &format!("goal:{id}"), &model.name(id), model.progress(id))
                        && id as i64 != global_goal
                    {
                        tree.get_mut(r).hidden += 1;
                    }
                }
                spacer(tree, list, true);
            }
        }
        // Available: the radio buttons of the goals, and the chosen line.
        {
            let (tree, root) = &mut trees[1];
            let root = *root;
            if let Some(group) = clear(tree, root, "personalRadio") {
                for &(id, enabled) in &personal {
                    radio(tree, group, model, id, enabled);
                }
                if personal.is_empty() {
                    add_xml(tree, group, "<TextView value=\"\" layout_borders=\"Rect(5,5,0,0)\"/>");
                }
                crate::gui::radio::set_value(tree, group, &self.goals[0].1.to_string());
            }
            if let Some(group) = clear(tree, root, "globalRadio") {
                for &id in &global {
                    radio(tree, group, model, id, true);
                }
                if global.is_empty() {
                    add_xml(tree, group, &format!("<TextView value=\"{}\" layout_borders=\"Rect(5,5,0,0)\"/>", " ".repeat(69)));
                }
                crate::gui::radio::set_value(tree, group, &global_goal.to_string());
            }
        }
        // Finished: the finished ranks.
        {
            let (tree, root) = &mut trees[2];
            let root = *root;
            for (name, list) in [("finishedPersonal", model.finished_personal()), ("finishedGlobal", model.finished_global())] {
                let Some(view) = clear(tree, root, name) else { continue };
                for &id in &list {
                    row(tree, view, &format!("done:{id}"), &model.name(id), model.progress(id));
                }
                if list.is_empty() {
                    add_xml(tree, view, "<TextView value=\"\" layout_borders=\"Rect(5,5,0,0)\"/>");
                }
                spacer(tree, view, true);
            }
        }
        // (FUN_100f5057: the two lists of a tab as wide as the wider, their scroll views at least 100 tall)
        for (k, names) in [(1, ["personalRadio", "globalRadio"]), (2, ["finishedPersonal", "finishedGlobal"])] {
            let (tree, root) = &mut trees[k];
            let root = *root;
            let ids: Vec<Id> = names.iter().filter_map(|n| tree.find(root, n)).collect();
            let width = ids.iter().map(|&id| tree.preferred(id, false).x).fold(0.0, f32::max);
            for id in ids {
                let n = tree.get_mut(id);
                n.min_size.x = n.min_size.x.max(width);
                let grand = tree.get(id).parent.and_then(|p| tree.get(p).parent);
                if let Some(g) = grand {
                    let n = tree.get_mut(g);
                    n.min_size.y = n.min_size.y.max(100.0);
                }
            }
        }
        // (FUN_100f5998: the chosen line's ranks)
        for (k, line) in self.lines.iter().enumerate() {
            let (tree, root) = &mut trees[k + 1];
            let root = *root;
            let Some(view) = clear(tree, root, "PerkLine") else { continue };
            if let Some(id) = line {
                for rank in model.line(*id) {
                    row(tree, view, &format!("rank:{rank}"), &model.name(rank), model.progress(rank));
                }
            }
            spacer(tree, view, true);
        }
        Some(trees)
    }

    /// Draws the window.
    pub fn show(&mut self, ctx: &egui::Context, state: &PerkState, stats: &BTreeMap<u32, i64>, palette: &Palette) {
        if self.open && !self.was_open {
            // (FUN_100f750e: the sliders from stats 263 and 264; N3Msg_UpdateGlobalResearch asks for the global lines)
            self.personal = stats.get(&263).copied().unwrap_or(0) as i32;
            self.global = stats.get(&264).copied().unwrap_or(0) as i32;
            self.goals = [(None, 0); 2];
            self.lines = [None; 2];
            self.info.clear();
            self.trees = None;
            self.built = None;
            send(Command::Request);
        }
        self.was_open = self.open;
        if !self.open {
            return;
        }
        let goals = [self.goal(0, stats), self.goal(1, stats)];
        let research = snapshot();
        let model = Model::new(&state.catalog, &research, stats, &state.owned);
        let key = (research.version, [33, 53, 54, 60].iter().map(|s| stats.get(s).copied().unwrap_or(0)).collect::<Vec<_>>(), state.owned.len() + state.catalog.records.len(), goals, self.lines);
        if self.built.as_ref() != Some(&key) {
            // (a personal rank finished: the lists built again and the global lines asked for, FUN_100f70b0 through
            // FUN_100f6dce)
            let finished = model.finished_personal().len();
            if self.built.is_some() && finished > self.finished {
                send(Command::Request);
            }
            self.finished = finished;
            self.trees = self.build(ctx, &model);
            self.built = Some(key);
        }
        let Some(trees) = self.trees.as_mut() else { return };
        // (the sliders' labels)
        {
            let (tree, root) = &mut trees[0];
            let root = *root;
            set_text(tree, root, "PersonalResearchLbl", &format!("{}%", self.personal));
            set_text(tree, root, "GlobalResearchLbl", &format!("{}%", self.global));
        }
        for k in 1..3 {
            let (tree, root) = &mut trees[k];
            let root = *root;
            set_text(tree, root, "info", &self.info);
        }
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut tab = self.tab;
        let mut hovered: Option<(u32, bool)> = None;
        let mut clicked = false;
        crate::chrome::window(ctx, palette, "Research", "", &mut open, |w| w.default_pos(crate::layout::pos("Research", egui::pos2(200.0, 180.0))).default_size(egui::vec2(650.0, 520.0)), |ui| {
            crate::chrome::tab_view(ui, palette, &TABS, &mut tab, |ui, tab| {
                let (tree, root) = &mut trees[tab];
                let response = crate::gui::host::show(ui, tree, *root, skin.as_deref(), palette);
                if response.contains_pointer() {
                    hovered = tree.hover.and_then(|h| row_of(tree, h));
                    clicked = ui.input(|i| i.pointer.primary_clicked());
                }
            });
        });
        self.tab = tab;
        self.open = open;
        // (FUN_100f5546: the description while the pointer is on a row)
        let info = hovered.map(|(id, _)| model.description(id)).unwrap_or_default();
        if info != self.info {
            self.info = info;
        }
        let trees = self.trees.as_mut().unwrap();
        // (the sliders, FUN_100f719a / FUN_100f7357)
        let read = |tree: &Tree, root: Id, name: &str| tree.find(root, name).and_then(|id| tree.control::<crate::gui::controls::Slider>(id)).map(|s| s.value as i32);
        let (p, g) = {
            let (tree, root) = &trees[0];
            (read(tree, *root, "PersonalResearchSlider"), read(tree, *root, "GlobalResearchSlider"))
        };
        if let Some(p) = p.filter(|&p| p != self.personal) {
            self.slide(false, p);
        } else if let Some(g) = g.filter(|&g| g != self.global) {
            self.slide(true, g);
        }
        let trees = self.trees.as_mut().unwrap();
        for (name, v) in [("PersonalResearchSlider", self.personal), ("GlobalResearchSlider", self.global)] {
            let (tree, root) = &mut trees[0];
            if let Some(id) = tree.find(*root, name)
                && let Some(s) = tree.control_mut::<crate::gui::controls::Slider>(id)
                && s.value as i32 != v
            {
                s.set(v as f64);
            }
        }
        // (the radio buttons: LAB_100f5033 sends stat 265, FUN_100f5382 stat 266)
        let mut chosen: Vec<(usize, i64)> = Vec::new();
        {
            let (tree, _) = &mut trees[1];
            for (id, signal) in std::mem::take(&mut tree.signals) {
                if let crate::gui::host::Signal::Value(v) = signal {
                    let k = if tree.get(id).name == "globalRadio" { 1 } else { 0 };
                    if let Ok(v) = v.parse::<i64>() {
                        chosen.push((k, v));
                    }
                }
            }
        }
        for k in [0, 2] {
            trees[k].0.signals.clear();
        }
        // (a click on a goal's row: its radio button picked, its line shown; on a finished rank's: its line)
        if clicked && let Some((id, true)) = hovered {
            match self.tab {
                1 => {
                    let (tree, root) = &trees[1];
                    let button = tree.find(*root, &format!("radio:{id}"));
                    if let Some(b) = button.filter(|&b| tree.get(b).enabled) {
                        let global = tree.get(b).parent.is_some_and(|g| tree.get(g).name == "globalRadio");
                        chosen.push((global as usize, id as i64));
                    }
                    self.lines[0] = Some(id);
                }
                2 => self.lines[1] = Some(id),
                _ => {}
            }
        }
        for (k, v) in chosen {
            if self.goals[k].1 != v {
                self.goals[k].1 = v;
                send(Command::SetStat(265 + k as u32, v as i32));
            }
        }
    }
}

/// `MAO_UI_PREVIEW=research`: the 2026-10-30 capture's character after choosing its goal (a fixer, stat 60 4; the personal
/// slider at 100; goal 0x988 with 0xb93b left, its `435f7023` of 1793399679) and the captured `ResearchUpdate` (all 54 global
/// ranks finished for every side): stats, the personal entries and the global lines.
pub fn preview_data() -> (Vec<(u32, i64)>, Vec<(u32, i64)>, Vec<(u32, [i64; 3])>) {
    let stats = vec![(60, 4), (54, 15), (33, 1), (263, 100), (264, 0), (265, 0x988), (266, 0)];
    (stats, vec![(0x988, 0xb93b)], CAPTURED_GLOBAL.iter().map(|&id| (id, [0; 3])).collect())
}

/// The global ranks of the captured `ResearchUpdate`s, in their order.
const CAPTURED_GLOBAL: [u32; 54] = [
    0xbb8, 0xbc2, 0xbc3, 0xbc4, 0xbcc, 0xbcd, 0xbce, 0xbd6, 0xbe0, 0xbe1, 0xbe2, 0xbea, 0xbeb, 0xbec, 0xbf4, 0xbfe, 0xbff, 0xc00, 0xc08, 0xc09, 0xc0a, 0xc12, 0xc1c, 0xc1d, 0xc1e, 0xc26, 0xc27,
    0xc28, 0xc30, 0xc3a, 0xc3b, 0xc3c, 0xc44, 0xc45, 0xc46, 0xc80, 0xc81, 0xc82, 0xc83, 0xc84, 0xc85, 0xc86, 0xc87, 0xc88, 0xc89, 0xc8a, 0xc8b, 0xc8c, 0xc8d, 0xc8e, 0xc8f, 0xc90, 0xc91, 0xc92,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u32, prev: u32, flags: u32, total: u32, name: &str, criteria: Vec<(u32, i32, u32)>) -> PerkRecord {
        PerkRecord { id, prev, flags, professions: 0, total, name: name.into(), description: String::new(), criteria, use_by: None, modifiers: String::new() }
    }

    /// Two personal lines (a fixer's and a soldier's) and a global line of two ranks.
    fn catalog() -> Catalog {
        Catalog::from_records(vec![
            record(0x988, 0, PERSONAL, 100, "Acquisition", vec![(54, 0, 2), (60, 4, 0), (4, 0, 4)]),
            record(0x989, 0x988, PERSONAL, 200, "Acquisition", vec![(54, 30, 2), (60, 4, 0), (4, 0, 4)]),
            record(0x98a, 0x989, PERSONAL, 400, "Acquisition", vec![(60, 4, 0)]),
            record(0x990, 0, PERSONAL, 100, "Soldier line", vec![(60, 1, 0)]),
            record(0xbb8, 0, GLOBAL, 1000, "Global", vec![]),
            record(0xbb9, 0xbb8, GLOBAL, 2000, "Global", vec![]),
        ])
    }

    #[test]
    fn the_captured_messages_parse() {
        // (arete-20261030-1655 #30710: S>C 435f7023, 13 bytes)
        let progress: Vec<u8> = (0..26).step_by(2).map(|k| u8::from_str_radix(&"0100000988000009880000b93b"[k..k + 2], 16).unwrap()).collect();
        assert_eq!(parse_progress(&progress), Some((0x988, 0x988, 0xb93b)));
        // (the captured ResearchUpdate: the leading byte, version 1, 54 lines all 0, a 0: 870 bytes)
        let mut update = vec![1u8, 1];
        for id in CAPTURED_GLOBAL {
            update.extend_from_slice(&id.to_be_bytes());
            update.extend_from_slice(&[0; 12]);
        }
        update.extend_from_slice(&[0; 4]);
        assert_eq!(update.len(), 870);
        let lines = parse_update(&update).unwrap();
        assert_eq!(lines.len(), 54);
        assert_eq!(lines[0], (0xbb8, [0, 0, 0]));
        assert_eq!(lines[53], (0xc92, [0, 0, 0]));
    }

    #[test]
    fn goals_progress_and_finished_as_gamecode() {
        let catalog = catalog();
        let stats: BTreeMap<u32, i64> = [(60, 4), (54, 15), (33, 1)].into_iter().collect();
        let mut research = Research::default();
        research.personal.insert(0x988, 25);
        research.global.insert(0xbb8, [5, 0, 7]);
        research.global.insert(0xbb9, [9, -1, 9]);
        let m = Model::new(&catalog, &research, &stats, &[]);
        // (the fixer's first rank, not finished; the second needs the first finished; the soldier's is another profession)
        assert_eq!(m.personal_goals(), vec![(0x988, true)]);
        assert!((m.progress(0x988) - 0.75).abs() < 1e-6);
        assert_eq!(m.name(0x989), "Acquisition 2");
        // (side 1: the first global rank finished, the second's -1 is its whole amount)
        assert_eq!(m.global_left(0xbb9), Some(2000));
        assert_eq!(m.global_goals(), vec![0xbb9]);
        assert_eq!(m.finished_global(), vec![0xbb8]);
        assert_eq!(m.progress(0xbb9), 0.0);
        // (the first rank finished: the second is the goal, its criteria not met at level 15)
        research.personal.insert(0x988, 0);
        research.personal.insert(0x989, 0);
        let m = Model::new(&catalog, &research, &stats, &[]);
        assert_eq!(m.personal_goals(), vec![(0x98a, true)]);
        // (two finished ranks of one line: the last)
        assert_eq!(m.finished_personal(), vec![0x989]);
        assert_eq!(m.line(0x989), vec![0x988, 0x989, 0x98a]);
        research.personal.remove(&0x989);
        let m = Model::new(&catalog, &research, &stats, &[]);
        assert_eq!(m.personal_goals(), vec![(0x989, false)]);
        // (no side outside 0 to 2)
        let other: BTreeMap<u32, i64> = [(60, 4), (33, 3)].into_iter().collect();
        assert!(Model::new(&catalog, &research, &other, &[]).global_goals().is_empty());
    }

    #[test]
    fn the_sliders_make_100_at_most() {
        let mut w = ResearchWindow { personal: 50, global: 30, ..ResearchWindow::default() };
        take_commands();
        // (FUN_100f719a: personal to 80 takes the global down to 20 first)
        w.slide(false, 80);
        assert_eq!((w.personal, w.global), (80, 20));
        // (FUN_100f7357: global to 10 leaves the personal)
        w.slide(true, 10);
        assert_eq!((w.personal, w.global), (80, 10));
        let sent: Vec<Command> = take_commands().into_iter().filter(|c| matches!(c, Command::SetStat(263 | 264, _))).collect();
        assert_eq!(sent, vec![Command::SetStat(264, 20), Command::SetStat(263, 80), Command::SetStat(264, 10)]);
    }

    /// `MAO_GAME`: the database's global ranks are the captured `ResearchUpdate`'s; a fixer's first goals.
    #[test]
    fn the_game_research_ranks() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let catalog = crate::perks::catalog(&db);
        let global: Vec<u32> = catalog.records.iter().filter(|r| r.flags & GLOBAL != 0).map(|r| r.id).collect();
        assert_eq!(global, CAPTURED_GLOBAL.to_vec());
        let (stats, personal, lines) = preview_data();
        let stats: BTreeMap<u32, i64> = stats.into_iter().collect();
        let research = Research { personal: personal.into_iter().collect(), global: lines.into_iter().collect(), version: 0 };
        let m = Model::new(&catalog, &research, &stats, &[]);
        let goals = m.personal_goals();
        let names: Vec<String> = goals.iter().map(|g| m.name(g.0)).collect();
        eprintln!("{names:?}");
        assert!(names.contains(&"Acquisition 1".to_string()), "{names:?}");
        assert!(m.global_goals().is_empty());
        assert_eq!(m.finished_global().len(), 54);
        let r = catalog.get(0x988).unwrap();
        assert!(r.total > 0 && m.progress(0x988) > 0.0 && m.progress(0x988) < 1.0, "{} {}", r.total, m.progress(0x988));
    }
}
