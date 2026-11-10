//! The Skills window as GUI.dll's `SkillWindow` builds it (FUN_100fcbac): the skin's `Views/Skills.xml` with a `StatRow`
//! (FUN_100fef4c) for each skill of each group, twice: a compact one in the group's list (`<group>_view`, under the group's
//! button) and one in the group's page (`<group>_group`, the rows alternating between two columns), and the selected
//! skill's details below the page (FUN_100fa1c4).
//!
//! The groups and their skills are FUN_100fbfb4's (in its order); their titles text.mdb's category 10010 (`#10010:n`), a
//! skill's name category 2003, its full name 2002 and its description 2001 (the `LDBface::GetText` calls). A row is a
//! clickable frame shown only while selected (`BorderView_c::HideBorders`) holding the name, a spacer, the buffed value and
//! the decrease and increase buttons (`GFX_GUI_DEC_SKILL`, `GFX_GUI_INC_SKILL`; the `_CHANGED` pictures once points are
//! added, FUN_100f9a0f). The name is coloured by the skill's cost level (FUN_100f99b1) and a page's row ends in a small
//! bar of the trained value against the skill's cap (a `PowerbarView_c` of `GFX_GUI_HOR_BAR_SMALL_EMPTY` / `_BLUE`, 5
//! from the right; FUN_100fe550 sets it). The details show what the next point costs (FUN_100f9aeb), what the points
//! added cost together (FUN_100f9b0d) and the cap; the costs and caps are Gamecode's, from the game's records
//! (`skillcost`).
//!
//! The increase button (FUN_100fe968) adds a point unless the skill is at its cap or the IP left do not pay for it
//! (text.mdb 502's `Skill_LackIP`); let go with Shift or Ctrl held it raises the skill as far as the cap and the IP
//! allow (FUN_100fea68). Both buttons act at the press and then every 50 ms once held for half a second. The IP shown
//! are those left after the points added. The value is white, or green when it is over what training and the abilities
//! give and red when under.
//!
//! "Suggested IP distribution" (FUN_100fb8de) spends the IP by the game's own priorities (`ipdist`, levels 1 to 20) and
//! lists what it raised. The resets ask first (FUN_100f9db0, FUN_100f9c65) and then send `N3Msg_ResetSkill`: a skill's
//! for a reset point (stat 199; a disabled skill's is free, an ability has none), or all of them.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::derived::Breakdown;
use crate::gui::geom::Point;
use crate::gui::host::{Mouse, Paint, Signal};
use crate::gui::view::{Control, Id, Tree};
use crate::theme::Palette;

/// FUN_100fbfb4's groups: the names the XML's views are found by (`<name>_view`, `<name>_group`, the button `<name>`) and
/// their skills in order.
pub const GROUPS: [(&str, &[u32]); 11] = [
    ("abilities", &[16, 17, 18, 19, 20, 21]),
    ("body", &[152, 132, 155, 154, 153, 168, 145]),
    ("meleew", &[102, 103, 106, 107, 105, 104, 100, 101, 118, 120]),
    ("melees", &[146, 142, 147, 144, 143]),
    ("rangedw", &[112, 111, 114, 116, 115, 113, 133, 109, 110, 134, 119]),
    ("rangeds", &[150, 151, 148, 167, 121, 108]),
    ("nanocast", &[127, 128, 129, 122, 131, 130, 149]),
    ("exploring", &[139, 166, 117, 156, 137]),
    ("combatheal", &[136, 164, 162, 135, 123, 124]),
    ("traderepair", &[125, 126, 157, 163, 158, 160, 141, 165, 161, 159]),
    ("disabled", &[138, 140]),
];

/// The disabled group (its rows take no points: `FUN_100fef4c`'s third argument).
const DISABLED: usize = 10;
const IP: u32 = 53;

/// `StatRow`'s frame: clickable, its borders shown only while it is the selected row.
struct Row {
    pieces: crate::gui::controls::Pieces,
    selected: bool,
}

impl Control for Row {
    fn class(&self) -> &'static str {
        "StatRow"
    }
    fn paint(&self, p: &Paint) {
        if self.selected {
            crate::gui::controls::nine_slice(p, &self.pieces, p.rect, egui::Color32::WHITE);
        }
    }
    fn takes_mouse(&self) -> bool {
        true
    }
    fn mouse(&mut self, m: Mouse, _at: Point) -> Option<Signal> {
        matches!(m, Mouse::Up { inside: true }).then_some(Signal::Clicked)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The text of a key of text.mdb (the original's `LDBface::GetText`), else `fallback`.
fn text(category: i32, key: u32, fallback: &str) -> String {
    crate::feedback::texts().and_then(|t| t.get(category, key as i32)).filter(|s| !s.is_empty()).map_or_else(|| fallback.to_string(), str::to_string)
}

/// Escapes a value for an XML attribute.
fn attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

/// A row's views (FUN_100fef4c): `mode` 0 for a group's list, 1 for its page.
fn row_xml(stat: u32, mode: u8) -> String {
    let name = attr(&text(2003, stat, &crate::skills::stat_name(stat)));
    format!(
        "<BorderView name=\"row{mode}_{stat}\" view_layout=\"horizontal\" layout_borders=\"Rect(1,1,1,1)\">\
           <TextView name=\"name{mode}_{stat}\" value=\"{name}\" layout_borders=\"Rect(3,0,0,0)\"/>\
           <HLayoutSpacer/>\
           <TextView name=\"value{mode}_{stat}\" value=\"0\"/>\
           <View view_layout=\"horizontal\">\
             <Button name=\"dec{mode}_{stat}\" label=\"\"/>\
             <Button name=\"inc{mode}_{stat}\" label=\"\" layout_borders=\"Rect(1,0,0,0)\"/>\
           </View>\
           {bar}\
         </BorderView>",
        // (a page's row: the bar of the trained value against the cap)
        bar = if mode == 1 { format!("<PowerBar name=\"bar_{stat}\" bg_gfx=\"GFX_GUI_HOR_BAR_SMALL_EMPTY\" full_gfx=\"GFX_GUI_HOR_BAR_SMALL_BLUE\" layout_borders=\"Rect(0,0,5,0)\"/>") } else { String::new() }
    )
}

/// Builds a fragment of views and puts it under `parent`.
fn add_xml(tree: &mut Tree, parent: Id, xml: &str) -> Option<Id> {
    let doc = crate::gui::xml::parse(xml);
    let id = crate::gui::create(tree, doc.first()?)?;
    tree.add_child(parent, id);
    Some(id)
}

#[derive(Default)]
pub struct SkillWindow {
    window: Option<crate::gui::window::XmlWindow>,
    /// The points added to each skill, not yet accepted.
    pending: BTreeMap<u32, i64>,
    selected: Option<u32>,
    pub status: String,
    /// A preview's: the group whose page to open, the skill to select and the points to have added to it.
    pub demo: Option<(usize, u32, i64)>,
    /// The increase and decrease buttons: the view, its skill and whether it increases.
    buttons: Vec<(Id, u32, bool)>,
    /// The button held down, and when it acts next.
    held: Option<(Id, u32, bool, Instant)>,
    /// The reset being asked about: a skill's, or all of them.
    asking: Option<Option<u32>>,
}

impl SkillWindow {
    /// FUN_100fcbac: the skin's view, a row for each skill in its group's list and page.
    fn load(&mut self, ctx: &egui::Context) {
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        let Some((mut tree, root)) = crate::gui::load_view(&dirs, "Skills.xml", "parent") else { return };
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        for (g, (group, stats)) in GROUPS.iter().enumerate() {
            if let Some(list) = tree.find(root, &format!("{group}_view")) {
                for &stat in *stats {
                    add_xml(&mut tree, list, &row_xml(stat, 0));
                }
                // (the list laid out down its rows, `SetLayoutNode(VLayoutNode)`; folded at first, the group's button opening
                // it: FUN_100f9f70)
                tree.get_mut(list).layout = crate::gui::view::Layout::Vertical;
                tree.get_mut(list).hidden += 1;
            }
            if let Some(page) = tree.find(root, &format!("{group}_group")) {
                // (two columns with a spacer between, the rows alternating)
                let mut columns = (String::new(), String::new());
                for (k, &stat) in stats.iter().enumerate() {
                    let target = if k % 2 == 0 { &mut columns.0 } else { &mut columns.1 };
                    target.push_str(&row_xml(stat, 1));
                }
                // (the page laid out across them, `SetLayoutNode(HLayoutNode)`)
                tree.get_mut(page).layout = crate::gui::view::Layout::Horizontal;
                add_xml(&mut tree, page, &format!("<View view_layout=\"vertical\">{}<VLayoutSpacer/></View>", columns.0));
                add_xml(&mut tree, page, "<HLayoutSpacer/>");
                add_xml(&mut tree, page, &format!("<View view_layout=\"vertical\">{}<VLayoutSpacer/></View>", columns.1));
            }
            for (mode, stat) in stats.iter().flat_map(|&s| [(0u8, s), (1u8, s)]) {
                let Some(row) = tree.find(root, &format!("row{mode}_{stat}")) else { continue };
                tree.get_mut(row).control = Box::new(Row { pieces: crate::gui::controls::Pieces::named("GFX_GUI_BORDER03", false), selected: false });
                for (button, picture) in [("dec", "GFX_GUI_DEC_SKILL"), ("inc", "GFX_GUI_INC_SKILL")] {
                    if let Some(b) = tree.find(row, &format!("{button}{mode}_{stat}")) {
                        self.buttons.push((b, stat, button == "inc"));
                        if let Some(c) = tree.control_mut::<crate::gui::controls::Button>(b) {
                            c.pictures = Some([picture.to_string(), format!("{picture}_CHANGED")]);
                        }
                        // (a disabled skill takes no points)
                        if g == DISABLED {
                            tree.get_mut(b).enabled = false;
                        }
                    }
                }
            }
        }
        self.window = Some(crate::gui::window::XmlWindow { name: "Skills".into(), title: "Skills".into(), tree, root, open: true, default_size: egui::vec2(640.0, 480.0) });
        // (MAO_SKILL=<group number>,<stat>,<points added>: the window comes up on that group's page with the skill selected,
        // for pictures of it)
        if self.demo.is_none()
            && let Some(v) = std::env::var("MAO_SKILL").ok().map(|t| t.split(',').filter_map(|x| x.trim().parse::<i64>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 3)
        {
            self.demo = Some((v[0] as usize, v[1] as u32, v[2]));
        }
    }

    fn set_text(tree: &mut Tree, root: Id, name: &str, value: &str) {
        let Some(id) = tree.find(root, name) else { return };
        if let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id) {
            if t.text != value {
                t.text = value.to_string();
                tree.relayout = true;
            }
        }
    }

    /// Shows a view or hides it.
    fn set_shown(tree: &mut Tree, root: Id, name: &str, shown: bool) {
        let Some(id) = tree.find(root, name) else { return };
        let want = if shown { 0 } else { 1 };
        if tree.get(id).hidden.min(1) != want {
            tree.get_mut(id).hidden = want;
            tree.relayout = true;
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, Breakdown>, palette: &Palette) {
        if self.window.is_none() {
            self.load(ctx);
        }
        let Some(w) = self.window.as_mut() else { return };
        let (tree, root) = (&mut w.tree, w.root);
        if let Some((group, stat, added)) = self.demo.take() {
            if let Some(list) = GROUPS.get(group).and_then(|g| tree.find(root, &format!("{}_view", g.0))) {
                tree.get_mut(list).hidden = 0;
            }
            for (selector, value) in [("infoselect", 1), ("groupselect", group as i32)] {
                if let Some(s) = tree.find(root, selector) {
                    crate::gui::selector::set_value(tree, s, value);
                }
            }
            self.selected = Some(stat);
            if added > 0 {
                self.pending.insert(stat, added);
            }
            tree.relayout = true;
            // (MAO_SKILL_SUGGEST: the suggested distribution pressed as well)
            if std::env::var_os("MAO_SKILL_SUGGEST").is_some()
                && let Some(id) = tree.find(root, "suggest_ip")
            {
                tree.signals.push((id, Signal::Clicked));
            }
        }
        let costs = Costs { tables: crate::skillcost::tables(), stats, parts };
        let ip = costs.stored(IP);
        // the increase and decrease buttons: a point at the press (FUN_100fe968, FUN_100fe7d1), then one every 50 ms once
        // held for half a second (their `EventTimer_c`s, FUN_100fecde); let go with Shift or Ctrl held the increase goes as
        // far as the cap and the IP allow and the decrease takes every added point back (FUN_100fea68, FUN_100fe8bb)
        let held_now = self.buttons.iter().copied().find(|(id, _, _)| tree.control::<crate::gui::controls::Button>(*id).is_some_and(|b| b.held));
        let now = Instant::now();
        let step = |pending: &mut BTreeMap<u32, i64>, status: &mut String, stat: u32, increase: bool| {
            if increase {
                raise(pending, status, &costs, stat);
            } else {
                lower(pending, stat, false);
            }
        };
        match (self.held, held_now) {
            (was, Some((id, stat, increase))) if was.map(|h| h.0) != Some(id) => {
                self.selected = Some(stat);
                self.status.clear();
                step(&mut self.pending, &mut self.status, stat, increase);
                self.held = Some((id, stat, increase, now + Duration::from_millis(500)));
            }
            (Some((id, stat, increase, next)), Some(_)) if now >= next => {
                step(&mut self.pending, &mut self.status, stat, increase);
                self.held = Some((id, stat, increase, now + Duration::from_millis(50)));
            }
            (Some((_, stat, increase, _)), None) => {
                if ctx.input(|i| i.modifiers.shift || i.modifiers.ctrl) {
                    if increase {
                        let mut quiet = String::new();
                        while raise(&mut self.pending, &mut quiet, &costs, stat) {}
                    } else {
                        lower(&mut self.pending, stat, true);
                    }
                }
                self.held = None;
            }
            _ => {}
        }
        if self.held.is_some() {
            ctx.request_repaint_after(Duration::from_millis(25));
        }
        let character = costs.who(&self.pending);
        // the values: each row's buffed value with the points added (FUN_100fa0e0), the buttons' pictures changed once added
        Self::set_text(tree, root, "remaining_ip", &(ip - costs.spent(&self.pending)).to_string());
        Self::set_text(tree, root, "status", &self.status);
        for (_, stats) in GROUPS {
            for &stat in stats {
                let added = self.pending.get(&stat).copied().unwrap_or(0);
                // (the name in its cost level's colour, FUN_100f99b1; the page's bar: trained over the cap, FUN_100fe550)
                if let Some(t) = costs.tables {
                    let colour = crate::skillcost::colour(t.cost_level(&character, stat));
                    for mode in 0..2 {
                        if let Some(id) = tree.find(root, &format!("name{mode}_{stat}"))
                            && let Some(name) = tree.control_mut::<crate::gui::controls::TextView>(id)
                        {
                            name.colour = crate::gui::controls::Colour(colour);
                        }
                    }
                    if let Some(bar) = tree.find(root, &format!("bar_{stat}")) {
                        let max = t.max(&character, stat);
                        crate::gui::powerbar::set_value(tree, bar, if max > 0 { ((costs.stored(stat) + added) as f32 / max as f32).clamp(0.0, 1.0) } else { 0.0 });
                    }
                }
                // (the value white when it is what training and the abilities give, green when more, red when less:
                // FUN_100fe550's compare of the shown value with `N3Msg_GetSkill`'s kind 1)
                let colour = match parts.get(&stat).map(|b| costs.buffed(stat).cmp(&(b.stored + b.trickle))) {
                    Some(std::cmp::Ordering::Greater) => 0x00ff00,
                    Some(std::cmp::Ordering::Less) => 0xff0000,
                    _ => 0xffffff,
                };
                for mode in 0..2 {
                    let name = format!("value{mode}_{stat}");
                    Self::set_text(tree, root, &name, &(costs.buffed(stat) + added).to_string());
                    if let Some(id) = tree.find(root, &name)
                        && let Some(value) = tree.control_mut::<crate::gui::controls::TextView>(id)
                    {
                        value.colour = crate::gui::controls::Colour(colour);
                    }
                    for button in ["dec", "inc"] {
                        if let Some(b) = tree.find(root, &format!("{button}{mode}_{stat}")) {
                            if let Some(c) = tree.control_mut::<crate::gui::controls::Button>(b) {
                                c.on = added > 0;
                            }
                        }
                    }
                    if let Some(row) = tree.find(root, &format!("row{mode}_{stat}")) {
                        if let Some(r) = tree.control_mut::<Row>(row) {
                            r.selected = self.selected == Some(stat);
                        }
                    }
                }
            }
        }
        // the selected skill's details (FUN_100fa1c4)
        let disabled = self.selected.is_some_and(|stat| GROUPS[DISABLED].1.contains(&stat));
        if let Some(stat) = self.selected {
            let added = self.pending.get(&stat).copied().unwrap_or(0);
            let right = |v: String| format!("<div align=\"right\">{v}</div>");
            Self::set_text(tree, root, "statName", &format!("<center>{}</center>", text(2002, stat, &crate::skills::stat_name(stat))));
            Self::set_text(tree, root, "statdesc", &text(2001, stat, ""));
            Self::set_text(tree, root, "base", &right((costs.stored(stat) + added).to_string()));
            Self::set_text(tree, root, "buffed", &right((costs.buffed(stat) + added).to_string()));
            Self::set_text(tree, root, "pointdelta", &right(added.to_string()));
            // (what the next point costs, FUN_100f9aeb; the points added together, FUN_100f9b0d; the cap)
            let fields = match costs.tables {
                Some(t) => [t.cost(&character, stat, costs.stored(stat) + added).to_string(), t.total_cost(&character, stat, costs.stored(stat), added).to_string(), t.max(&character, stat).to_string()],
                None => ["-".to_string(), "-".to_string(), "-".to_string()],
            };
            for (field, value) in ["unitcost", "totalcost", "maxskill"].into_iter().zip(fields) {
                Self::set_text(tree, root, field, &right(value));
            }
            // (the reset: a disabled skill's is free; an ability has none; the others' takes a reset point, stat 199)
            if let Some(s) = tree.find(root, "resetselect") {
                crate::gui::selector::set_value(tree, s, i32::from(disabled));
            }
            Self::set_shown(tree, root, "resetselect", disabled || !(16..=21).contains(&stat));
            Self::set_text(tree, root, "rp", &stats.get(&199).copied().unwrap_or(0).to_string());
        }
        // (the notice that a skill is disabled: shown only for one of the disabled group's, FUN_100fa1c4's `Show`)
        Self::set_shown(tree, root, "disabledLbl", disabled);
        // ("Reset all skills (n)", FUN_100fae7a: one when stat 348's third bit is clear, and stat 691's; the button dead
        // at none. The original reads both stats whatever they hold; here a stat the server has not told of counts nothing)
        let resets = stats.get(&348).map_or(0, |f| i64::from(f & 4 == 0)) + stats.get(&691).copied().unwrap_or(0);
        if let Some(id) = tree.find(root, "resetAll") {
            let label = format!("{} ({resets})", crate::gui::xml::ldb_named(502, "SkillView_ResetAll", "Reset all skills"));
            if let Some(b) = tree.control_mut::<crate::gui::controls::Button>(id)
                && b.label != label
            {
                b.label = label;
                tree.relayout = true;
            }
            tree.get_mut(id).enabled = resets > 0;
        }
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(tree, root, false);
        let mut is_open = *open;
        crate::chrome::window(ctx, palette, "Skills", "Skills", &mut is_open, |w| w.default_size(min.max(egui::vec2(640.0, 480.0))), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        // what the views said
        for (id, signal) in std::mem::take(&mut tree.signals) {
            if signal != Signal::Clicked {
                continue;
            }
            let name = tree.get(id).name.clone();
            let stat_of = |prefix: &str| name.strip_prefix(prefix).and_then(|r| r.split('_').nth(1)).and_then(|s| s.parse::<u32>().ok());
            if let Some(g) = GROUPS.iter().position(|(group, _)| *group == name) {
                // a group's button: its list folded or opened, its page shown (FUN_100f9f70)
                if let Some(list) = tree.find(root, &format!("{name}_view")) {
                    let node = tree.get_mut(list);
                    node.hidden = if node.hidden > 0 { 0 } else { 1 };
                }
                if let Some(s) = tree.find(root, "infoselect") {
                    crate::gui::selector::set_value(tree, s, 1);
                }
                if let Some(s) = tree.find(root, "groupselect") {
                    crate::gui::selector::set_value(tree, s, g as i32);
                }
                tree.relayout = true;
            } else if stat_of("inc").or(stat_of("dec")).is_some() {
                // (carried out at the press, above)
            } else if let Some(stat) = stat_of("row") {
                self.selected = Some(stat);
                if let Some(s) = tree.find(root, "infoselect") {
                    crate::gui::selector::set_value(tree, s, 1);
                }
                if let Some((g, _)) = GROUPS.iter().enumerate().find(|(_, (_, stats))| stats.contains(&stat)) {
                    if let Some(s) = tree.find(root, "groupselect") {
                        crate::gui::selector::set_value(tree, s, g as i32);
                    }
                }
                tree.relayout = true;
            } else if name == "suggest_ip" {
                // (FUN_100fb8de: the distribution worked out twice over, then the skills it touched listed on the info area's
                // third page in two columns, each with the points it now has added)
                let profession = stats.get(&60).map(|p| text(2004, *p as u32, crate::launcher::profession(*p as i32))).unwrap_or_default();
                let level = stats.get(&54).copied().unwrap_or(1);
                let priority = |stat: u32| crate::ipdist::priorities().map_or(0, |p| p.priority(&profession, &text(2003, stat, &crate::skills::stat_name(stat)), level));
                let mut touched = suggest(&mut self.pending, &costs, &priority);
                for stat in suggest(&mut self.pending, &costs, &priority) {
                    if !touched.contains(&stat) {
                        touched.push(stat);
                    }
                }
                if std::env::var_os("MAO_TRACE_SKILLS").is_some() {
                    eprintln!("suggested for {profession:?} level {level}: {touched:?}, ip {ip}, priorities {:?}", GROUPS[0].1.iter().map(|s| priority(*s)).collect::<Vec<_>>());
                }
                if let (Some(a), Some(b)) = (tree.find(root, "col1"), tree.find(root, "col2")) {
                    tree.get_mut(a).children.clear();
                    tree.get_mut(b).children.clear();
                    for column in [a, b] {
                        tree.get_mut(column).layout = crate::gui::view::Layout::Vertical;
                    }
                    for (k, stat) in touched.iter().enumerate() {
                        // (the original wraps the two in left and right aligned blocks; the spacer between does the same here)
                        let name = attr(&text(2003, *stat, &crate::skills::stat_name(*stat)));
                        let points = self.pending.get(stat).copied().unwrap_or(0);
                        let row = format!(
                            "<View view_layout=\"horizontal\"><TextView value=\"{name}\" layout_borders=\"Rect(10,3,10,0)\"/><HLayoutSpacer/><TextView value=\"{points}\" layout_borders=\"Rect(10,3,10,0)\"/></View>"
                        );
                        add_xml(tree, if k % 2 == 0 { a } else { b }, &row);
                    }
                    if let Some(s) = tree.find(root, "infoselect") {
                        crate::gui::selector::set_value(tree, s, 2);
                    }
                }
                tree.relayout = true;
            } else if name == "reset" || name == "resetFree" {
                self.asking = self.selected.map(Some);
            } else if name == "resetAll" {
                self.asking = Some(None);
            } else if name == "Accept" {
                // (the raises: each skill's new trained value; the server checks them and answers with the IP left)
                let raises: Vec<(u32, u32)> = self.pending.iter().map(|(&stat, &p)| (stat, (costs.stored(stat) + p).max(0) as u32)).collect();
                if !raises.is_empty() {
                    crate::skills::push_raises(raises);
                    crate::tips::trigger("OnAcceptingSkillChanges", "");
                    self.status = "Changes sent.".into();
                }
                self.pending.clear();
            } else if name == "Quit" {
                is_open = false;
            }
        }
        // the question before a reset (FUN_100f9db0 a skill's, FUN_100f9c65 all of them): Yes sends `N3Msg_ResetSkill`
        // with the skill's stat, or nothing for all (FUN_100f9c3e, FUN_100f9c21)
        if let Some(what) = self.asking {
            let question = match what {
                Some(stat) => crate::gui::xml::ldb_named(502, "SkillReset_Confirm", "Reset the skill %s?").replace("%s", &text(2002, stat, &crate::skills::stat_name(stat))),
                None => crate::gui::xml::ldb_named(502, "SkillView_ResetAllConfirm", "Reset every ability and skill to its start and get the IP back?").replace("\r\n", "<br>"),
            };
            if let Some(yes) = crate::chrome::question(ctx, palette, "reset-skill", "Skills", &question) {
                if yes {
                    crate::skills::push_reset(what.unwrap_or(0));
                    match what {
                        Some(stat) => drop(self.pending.remove(&stat)),
                        None => self.pending.clear(),
                    }
                    self.status = "Reset asked for.".into();
                }
                self.asking = None;
            }
        }
        if !is_open {
            self.pending.clear();
            self.asking = None;
        }
        *open = is_open;
    }
}

/// What the window's sums are worked out from: Gamecode's tables and the character's stats.
struct Costs<'a> {
    tables: Option<&'static crate::skillcost::Tables>,
    stats: &'a BTreeMap<u32, i64>,
    parts: &'a BTreeMap<u32, Breakdown>,
}

impl Costs<'_> {
    /// A stat as trained (`N3Msg_GetSkill`'s kind 0).
    fn stored(&self, stat: u32) -> i64 {
        self.parts.get(&stat).map_or_else(|| self.stats.get(&stat).copied().unwrap_or(0), |b| b.stored)
    }

    /// A stat as shown (kind 2).
    fn buffed(&self, stat: u32) -> i64 {
        self.stats.get(&stat).copied().unwrap_or_else(|| self.stored(stat))
    }

    /// The character the costs and caps are for, with the points being added to its abilities.
    fn who(&self, pending: &BTreeMap<u32, i64>) -> crate::skillcost::Character {
        let level = self.stats.get(&54).copied().unwrap_or(1);
        crate::skillcost::Character {
            breed: self.stats.get(&4).copied().unwrap_or(0) as u32,
            profession: self.stats.get(&60).map(|p| *p as u32),
            level,
            // (the title level: the server's, else worked out from the level as the original does)
            title: self.stats.get(&37).copied().filter(|t| *t > 0).unwrap_or_else(|| crate::derived::title_level(level)),
            abilities: std::array::from_fn(|k| self.stored(16 + k as u32) + pending.get(&(16 + k as u32)).copied().unwrap_or(0)),
        }
    }

    /// The IP the points added cost together.
    fn spent(&self, pending: &BTreeMap<u32, i64>) -> i64 {
        let who = self.who(pending);
        self.tables.map_or(0, |t| pending.iter().map(|(&stat, &p)| t.total_cost(&who, stat, self.stored(stat), p)).sum())
    }

    /// Whether a point more fits under the skill's cap (FUN_100f9aac).
    fn below_cap(&self, pending: &BTreeMap<u32, i64>, stat: u32) -> bool {
        let added = pending.get(&stat).copied().unwrap_or(0);
        self.tables.is_none_or(|t| self.stored(stat) + added + 1 <= t.max(&self.who(pending), stat))
    }

    /// Whether the IP pay for a point more (FUN_100fe550's check).
    fn affordable(&self, pending: &BTreeMap<u32, i64>, stat: u32) -> bool {
        let ip = self.stored(IP);
        match self.tables {
            Some(_) => {
                let mut next = pending.clone();
                *next.entry(stat).or_insert(0) += 1;
                self.spent(&next) <= ip
            }
            None => ip > pending.values().sum::<i64>(),
        }
    }
}

/// The increase button (FUN_100fe968): a point more unless the skill is disabled, at its cap (FUN_100fdf13) or the IP
/// left do not pay for it (FUN_100fe550). Says why not in `status`; true when the point went in.
fn raise(pending: &mut BTreeMap<u32, i64>, status: &mut String, costs: &Costs, stat: u32) -> bool {
    if GROUPS[DISABLED].1.contains(&stat) {
        *status = "This skill has been disabled and can not be increased.".into();
    } else if !costs.below_cap(pending, stat) {
        *status = format!("{} cannot be raised further now.", text(2003, stat, &crate::skills::stat_name(stat)));
    } else if !costs.affordable(pending, stat) {
        *status = crate::gui::xml::ldb_named(502, "Skill_LackIP", "Not enough IP to raise this skill further.");
    } else {
        *pending.entry(stat).or_insert(0) += 1;
        return true;
    }
    false
}

/// The decrease button (FUN_100fe7d1): an added point back, or all of them (FUN_100fe8bb).
fn lower(pending: &mut BTreeMap<u32, i64>, stat: u32, all: bool) {
    if let Some(p) = pending.get_mut(&stat) {
        *p = if all { 0 } else { *p - 1 };
    }
    pending.retain(|_, p| *p > 0);
}

/// The suggested distribution (FUN_100fb667): the skills by their priority (`ipdist`), least number first and none for
/// 0; within a priority a point each in turn, a skill leaving the round once it is at its cap, until none is left or
/// the IP do not pay for the next point, which ends it all. The skills that got points, in the order they first did.
fn suggest(pending: &mut BTreeMap<u32, i64>, costs: &Costs, priority: &dyn Fn(u32) -> i32) -> Vec<u32> {
    let mut rounds: BTreeMap<i32, Vec<u32>> = BTreeMap::new();
    for (_, stats) in GROUPS {
        for &stat in stats {
            rounds.entry(priority(stat)).or_default().push(stat);
        }
    }
    let mut touched = Vec::new();
    for (_, mut round) in rounds.into_iter().filter(|(p, _)| *p != 0) {
        let mut at = 0;
        while !round.is_empty() {
            let stat = round[at];
            if GROUPS[DISABLED].1.contains(&stat) || !costs.below_cap(pending, stat) {
                round.remove(at);
            } else if !costs.affordable(pending, stat) {
                return touched;
            } else {
                *pending.entry(stat).or_insert(0) += 1;
                if !touched.contains(&stat) {
                    touched.push(stat);
                }
                at += 1;
            }
            if at >= round.len() {
                at = 0;
            }
        }
    }
    touched
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_suggestion_goes_round_the_first_priority() {
        // (without the game's tables a point costs one: five IP go round the two skills of priority 1; the one of
        // priority 2 gets none, nor those of none)
        let stats: BTreeMap<u32, i64> = [(IP, 5)].into_iter().collect();
        let parts = BTreeMap::new();
        let costs = Costs { tables: None, stats: &stats, parts: &parts };
        let priority = |stat: u32| match stat {
            16 | 17 => 1,
            18 => 2,
            _ => 0,
        };
        let mut pending = BTreeMap::new();
        assert_eq!(suggest(&mut pending, &costs, &priority), [16, 17]);
        assert_eq!(pending, [(16, 3), (17, 2)].into_iter().collect());
        // (the buttons: a point back, all back; a disabled skill takes none)
        lower(&mut pending, 16, false);
        lower(&mut pending, 17, true);
        assert_eq!(pending, [(16, 2)].into_iter().collect());
        let mut status = String::new();
        assert!(!raise(&mut pending, &mut status, &costs, 138) && status.contains("disabled"));
        assert!(raise(&mut pending, &mut status, &costs, 18) && pending[&18] == 1);
    }

    /// The skin's view has every group's list, page and button, and every row goes in.
    #[test]
    fn the_rows_go_into_the_views() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/gui/AOU_Slim_CNI");
        let (mut tree, root) = crate::gui::load_view(&[dir], "Skills.xml", "parent").expect("Skills.xml");
        for (group, stats) in GROUPS {
            for name in [group.to_string(), format!("{group}_view"), format!("{group}_group")] {
                assert!(tree.find(root, &name).is_some(), "{name}");
            }
            let list = tree.find(root, &format!("{group}_view")).unwrap();
            for &stat in stats {
                add_xml(&mut tree, list, &row_xml(stat, 0)).unwrap();
                assert!(tree.find(root, &format!("inc0_{stat}")).is_some());
            }
        }
        // every skill in one group only, the 75 of FUN_100fbfb4
        let all: Vec<u32> = GROUPS.iter().flat_map(|(_, s)| s.iter().copied()).collect();
        let unique: std::collections::BTreeSet<u32> = all.iter().copied().collect();
        assert_eq!(all.len(), unique.len());
        assert_eq!(all.len(), 75);
    }
}
