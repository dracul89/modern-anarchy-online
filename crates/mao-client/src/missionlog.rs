//! The mission journal: GUI.dll's `MissionView_c` (`mission_window`, "Missions", FUN_100d3644), the character's own
//! missions, not the booth's offers (`mission.rs`, `MissionSelectionView_c`).
//!
//! The view is one `MissionListView_c` (FUN_100d3df7): a `MultiListView_c` in its list mode with three columns named
//! from text.mdb's category 10000, "Icon" (16 wide, sortable), "Name" (200) and "Tab_Remaning" (55; both resizable,
//! movable and sortable), 167 by 112 when nothing says otherwise (FUN_100d0177). Each mission is an
//! `InventoryListViewItem_c` row (FUN_100d2c3b): its icon, its name and the time it has left as hours and minutes
//! (FUN_1003d7d0's column 15: "%02d:%02d"), set again every second (FUN_100d3db7 on the list's timer; FUN_100d3958: no
//! time left shows 00:00). The time left is `N3Msg_QuestRemainingTime` (Interfaces.dll 0x1000a8ac): of the mission's
//! actions whose deadline has not passed, the last one's deadline less the server's time
//! (`N3Msg_GetServerSynceSystemTime`).
//!
//! The selected row is not the clicked one but the active mission (`+0x1a4`, FUN_100d3366): the one whose place was
//! last uploaded to the map (FUN_100d341a), until it goes (FUN_100d3441). A click on a row shows the mission in the
//! info view (FUN_100d2cfa: `itemid://<type>/<instance>`), whose page is FUN_100360d5 ([`MissionLog::page`]).
//!
//! The server keeps the list in `QuestFullUpdate` (`465a4061`, every mission: its identity, flags, title, text, reward,
//! icon and the actions to do, each with a deadline, a playfield and a place) and drops one with `Quest` action Delete
//! (`212c487a`). A row's menu (FUN_100d3123) is "Look at Mission", "Upload to map" (when the mission names a place:
//! `N3Msg_GetQuestWorldPos`), "Update Team Members" (only a team mission's: `N3Msg_IsTeamMission`, the flags' bit 8) and
//! "Delete Mission" (asked first: "ReallyWant2DeleteMission"). Uploading (FUN_10067d5f) puts the place on the planet map
//! and the compass and says so: "MissionWptUploaded", then "MissionShownAsDot" in the mission's playfield, else
//! "MissionNotInArea"; a mission without a place "NoWptInfo".
//!
//! Not done: the columns' widths, order and sort kept between sessions (`MissionViewConfig`'s `listview_config`), the
//! active mission kept with them (`active_mission`, whose place the original uploads again at the start), and "Update
//! Team Members" itself (`GiveQuestToMembers`, never captured).

use std::sync::Arc;

use mao_net::schema::Value;

use crate::gear::Icon;
use crate::theme::Palette;

/// The identity type of a mission, as `itemid://` names it.
pub const MISSION_TYPE: u32 = 0xdac3;

/// The view's size when nothing says otherwise (FUN_100d0177: 167 by 112 in the original's units).
const SIZE: egui::Vec2 = egui::vec2(168.0, 113.0);

/// The time-left column's id (`InventoryListViewItem_c::ColumnID_e` 15).
const REMAINING: i32 = 15;

/// A reward item of a mission (`GameData::ACGItem_t`).
#[derive(Clone, Debug, PartialEq)]
pub struct Reward {
    pub low: u32,
    pub high: u32,
    pub quality: u32,
}

/// One of the character's missions.
#[derive(Clone)]
pub struct Mission {
    pub id: (u32, u32),
    pub title: String,
    /// The text (the server's markup: `<BR>` breaks).
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// The first action's playfield and place (server coordinates), when it names one.
    pub place: Option<(u32, [f32; 3])>,
    /// `Quest_t` +0xa4: bit 8 a team mission, bit 9 a team mission's copy.
    pub flags: u32,
    /// What it pays when done (`N3Msg_QuestGetCashReward`, `N3Msg_QuestGetXPReward`, `N3Msg_GetItemRewardList`).
    pub credits: i64,
    pub xp: i64,
    pub rewards: Vec<Reward>,
    /// Each action's deadline (`QuestAction_t` +0x5c, the server's time in seconds; 0 none), in the actions' order.
    pub deadlines: Vec<u32>,
}

impl Mission {
    /// `N3Msg_QuestRemainingTime`: the seconds left at the server's time `now`; -1 when no action has a deadline still
    /// ahead (the last such action's counts).
    pub fn remaining(&self, now: u32) -> i64 {
        let mut left = -1;
        for &deadline in &self.deadlines {
            if deadline != 0 && deadline >= now {
                left = (deadline - now) as i64;
            }
        }
        left
    }

    /// `N3Msg_IsTeamMission`.
    pub fn team(&self) -> bool {
        self.flags & 0x100 != 0
    }

    /// `itemid://<type>/<instance>`, as FUN_100d2cfa writes it ("%d").
    pub fn url(&self) -> String {
        format!("itemid://{}/{}", self.id.0 as i32, self.id.1 as i32)
    }
}

/// The list's time-left cell (FUN_100d3958 and FUN_1003d7d0's column 15): hours and minutes, nothing left as 00:00.
pub fn remaining_cell(left: i64) -> String {
    let left = left.max(0);
    format!("{:02}:{:02}", left / 3600, left % 3600 / 60)
}

/// What the window asks the network thread to send.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Delete((u32, u32)),
}

/// The missions of a parsed `QuestFullUpdate`.
pub fn missions_from(v: &Value, icon: &dyn Fn(u32) -> Option<Arc<Icon>>) -> Vec<Mission> {
    let int = |v: &Value, f: &str| v.get(f).and_then(Value::as_int).unwrap_or(0);
    let float = |v: &Value, f: &str| match v.get(f) {
        Some(Value::Float(x)) => *x as f32,
        Some(other) => other.as_int().unwrap_or(0) as f32,
        None => 0.0,
    };
    let text = |v: &Value, f: &str| match v.get(f) {
        Some(Value::Str(s)) => s.clone(),
        Some(Value::Bytes(b)) => String::from_utf8_lossy(b).into_owned(),
        _ => String::new(),
    };
    let Some(Value::List(quests)) = v.get("Quests") else { return Vec::new() };
    quests
        .iter()
        .map(|q| {
            let id = q.get("QuestId").map_or((0, 0), |i| (int(i, "Type") as u32, int(i, "Instance") as u32));
            let actions: &[Value] = match q.get("QuestActions") {
                Some(Value::List(actions)) => actions,
                _ => &[],
            };
            let place = actions.iter().find_map(|a| {
                let pf = a.get("PlayfieldId").map_or(0, |p| int(p, "Instance") as u32);
                let p = a.get("Position")?;
                (pf != 0).then(|| (pf, [float(p, "X"), float(p, "Y"), float(p, "Z")]))
            });
            let rewards = match q.get("MissionItemData") {
                Some(Value::List(items)) => items.iter().map(|r| Reward { low: int(r, "LowId") as u32, high: int(r, "HighId") as u32, quality: int(r, "Ql") as u32 }).collect(),
                _ => Vec::new(),
            };
            Mission {
                id,
                title: text(q, "ShortInfo"),
                description: text(q, "LongInfo"),
                icon: icon(int(q, "MissionIconId") as u32),
                place,
                flags: int(q, "Flags") as u32,
                credits: int(q, "Credits"),
                xp: int(q, "Xp"),
                rewards,
                deadlines: actions.iter().map(|a| int(a, "Deadline") as u32).collect(),
            }
        })
        .collect()
}

/// A GUI text (text.mdb category 10000, the names GUI.dll asks for).
fn text(name: &str) -> String {
    let english = match name {
        "Tab_Remaning" => "Remaining",
        "Look@Mission" => "Look at Mission",
        "Upload2Map" => "Upload to map",
        "UpdateTeamMembers" => "Update Team Members",
        "DeleteMission" => "Delete Mission",
        "ReallyWant2DeleteMission" => "Do you really want to delete this mission?",
        "MissionWptUploaded" => "Waypoint uploaded to map and compass.",
        "MissionShownAsDot" => "The waypoint is shown as a white dot on the map and a yellow indicator on the compass.",
        "MissionNotInArea" => "Mission is not in this area. You will be informed when entering correct area.",
        "NoWptInfo" => "No waypoint info available in mission data.",
        _ => name,
    };
    crate::feedback::texts().and_then(|t| t.named(10000, name)).unwrap_or(english).to_string()
}

/// An info page text (text.mdb category 506 by name), with this client's own words when the game's are not at hand.
fn info_text(name: &str) -> String {
    let own = crate::gui::iteminfo::text(name);
    if !own.is_empty() {
        return own;
    }
    match name {
        "Description" => "Description: ",
        "RealtimeLeft" => "Time left: ",
        "DayHourMinute" => "%d days, %d hours, %d minutes",
        "TeamMissionRules" => "Team: ",
        "ThisIsaTeamMission" => "this is a team mission",
        "Reward" => "<br>Reward<br>",
        "Cash" => "Cash: ",
        "NumCredits" => "%d credits",
        "Experience" => "Experience: ",
        "NumPoints" => "%d points",
        "ItemReward" => "Items: ",
        "NoReward" => "none",
        _ => "",
    }
    .to_string()
}

#[derive(Default)]
pub struct MissionLog {
    pub open: bool,
    pub missions: Vec<Mission>,
    /// The active mission (`MissionView_c` +0x1a4): the one last uploaded to the map, the list's selected row.
    active: Option<(u32, u32)>,
    /// The mission whose deletion awaits an answer.
    confirm: Option<(u32, u32)>,
    textures: std::collections::HashMap<u32, egui::TextureHandle>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id)>,
    /// The mission whose menu is open, and where.
    menu: Option<((u32, u32), egui::Pos2)>,
}

/// What a frame of the window did.
#[derive(Default)]
pub struct Shown {
    pub commands: Vec<Command>,
    /// A waypoint uploaded, and the lines to tell.
    pub waypoint: Option<(u32, [f32; 3])>,
    pub lines: Vec<String>,
    /// A mission's page for the info view (its URL).
    pub info: Option<String>,
}

impl MissionLog {
    /// The server's full list.
    pub fn set(&mut self, missions: Vec<Mission>) {
        self.missions = missions;
        // (FUN_100d347d: an active mission no longer in the list is not shown as one)
        if self.active.is_some_and(|a| !self.missions.iter().any(|m| m.id == a)) {
            self.active = None;
        }
    }

    /// The server dropped a mission (FUN_100d3441: the active one stops being it).
    pub fn remove(&mut self, id: (u32, u32)) {
        self.missions.retain(|m| m.id != id);
        if self.active == Some(id) {
            self.active = None;
        }
    }

    /// FUN_10067d5f: the mission's place as the waypoint, and what is told; the mission becomes the active one
    /// (FUN_100d341a).
    fn upload(&mut self, id: (u32, u32), playfield: u32, shown: &mut Shown) {
        let Some(m) = self.missions.iter().find(|m| m.id == id) else { return };
        match m.place {
            Some((pf, p)) => {
                shown.waypoint = Some((pf, p));
                shown.lines.push(text("MissionWptUploaded"));
                shown.lines.push(text(if pf == playfield { "MissionShownAsDot" } else { "MissionNotInArea" }));
                self.active = Some(id);
            }
            None => shown.lines.push(text("NoWptInfo")),
        }
    }

    /// The info view's page of a mission (FUN_100360d5), for `itemid://<mission type>/<instance>`: its name
    /// (FUN_10031d7c, in the colour of an item without a rarity), its text (FUN_100332ff: a written "\n" is a new line),
    /// the time left in days, hours and minutes when it has a deadline ahead, the team mission's note, and what it
    /// pays: the credits, the experience (not for a reader of level 200 to 220, who is paid in shadow knowledge the
    /// original works out with `GetSKForXP`: not ported) and the reward items as links with their icons, or that there
    /// is none. `now` is the server's time, `level` the reader's.
    pub fn page(&self, url: &str, now: u32, level: i64, db: &mao_formats::resource::ResourceDatabase) -> Option<String> {
        let (kind, instance) = crate::gui::iteminfo::parse_itemid(url)?;
        let m = self.missions.iter().find(|m| kind == MISSION_TYPE && m.id == (kind, instance))?;
        let item = |id: u32| db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: id }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)).map(|t| (t.icon(), t.name));
        Some(page_of(m, now, level, &item))
    }

    /// The window: the `MissionListView_c` list of the missions (icon, name, time left), the active one selected; a click
    /// shows the mission in the info view (FUN_100d2cfa), a right click opens its menu (FUN_100d3123).
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, playfield: u32) -> Shown {
        let mut shown = Shown::default();
        if !self.open {
            return shown;
        }
        if self.views.is_none() {
            use crate::gui::multilist::{self, MOVABLE, RESIZABLE, SORTABLE};
            use crate::gui::view::{Layout, Node, Tree};
            let mut tree = Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
            tree.get_mut(root).layout = Layout::Base;
            let list = multilist::create(&mut tree, "MissionListView");
            multilist::add_column(&mut tree, list, 0, &text("Icon"), 16.0, SORTABLE);
            multilist::add_column(&mut tree, list, 1, &text("Name"), 200.0, RESIZABLE | MOVABLE | SORTABLE);
            multilist::add_column(&mut tree, list, REMAINING, &text("Tab_Remaning"), 55.0, RESIZABLE | MOVABLE | SORTABLE);
            tree.add_child(root, list);
            self.views = Some((tree, root, list));
        }
        // (the rows, their time left as the list's timer sets it each second)
        let now = crate::gametime::server_now();
        let textures = &mut self.textures;
        let rows: Vec<crate::gui::multilist::Item> = self
            .missions
            .iter()
            .enumerate()
            .map(|(k, m)| {
                let left = m.remaining(now).max(0);
                crate::gui::multilist::Item {
                    id: k as i64,
                    icon: m.icon.as_ref().map(|icon| textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("mission-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id()),
                    cells: vec![(1, crate::gui::multilist::CellView::Text(m.title.clone())), (REMAINING, crate::gui::multilist::CellView::Text(remaining_cell(left)))],
                    keys: vec![(1, m.title.clone()), (REMAINING, format!("{left:010}"))],
                    ..Default::default()
                }
            })
            .collect();
        let active = self.active.and_then(|a| self.missions.iter().position(|m| m.id == a)).map(|k| k as i64);
        let Some((tree, root, list)) = self.views.as_mut() else { return shown };
        let (root, list) = (*root, *list);
        {
            // (compared whatever the list's order: a sorted list keeps its rows until one changes)
            let mut had: Vec<&crate::gui::multilist::Item> = crate::gui::multilist::items(tree, list).iter().collect();
            had.sort_by_key(|i| i.id);
            if had.len() != rows.len() || had.iter().zip(&rows).any(|(a, b)| *a != b) {
                crate::gui::multilist::set_items(tree, list, rows);
            }
        }
        // (FUN_100d3366: the selected row is the active mission's)
        if crate::gui::multilist::selected(tree, list) != active {
            crate::gui::multilist::select(tree, list, active);
        }
        crate::gui::multilist::update(tree, list);
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut hovered = None;
        // (named apart from the mission selection window, which Escape closes; the journal keeps its own place)
        crate::chrome::window(ctx, palette, "MissionView", "Missions", &mut open, |w| w.default_size(SIZE).resizable(true), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                hovered = crate::gui::multilist::item_at(tree, list, response.rect.min, at).map(|k| (k as usize, at));
            }
        });
        for (_, event) in crate::gui::multilist::take_events(tree) {
            if let crate::gui::multilist::ItemEvent::Clicked(k, _) = event
                && let Some(m) = self.missions.get(k as usize)
            {
                shown.info = Some(m.url());
            }
        }
        tree.signals.clear();
        if let Some((k, at)) = hovered
            && ctx.input(|i| i.pointer.secondary_clicked())
            && let Some(m) = self.missions.get(k)
        {
            self.menu = Some((m.id, at));
        }
        // (FUN_100d3123's menu)
        if let Some((id, at)) = self.menu
            && let Some(m) = self.missions.iter().find(|m| m.id == id).cloned()
        {
            let mut keep = true;
            let mut upload = false;
            let area = egui::Area::new(egui::Id::unique("mission-menu")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    if ui.button(text("Look@Mission")).clicked() {
                        shown.info = Some(m.url());
                        keep = false;
                    }
                    if m.place.is_some() && ui.button(text("Upload2Map")).clicked() {
                        upload = true;
                        keep = false;
                    }
                    // (a team mission's only; `GiveQuestToMembers` is not known to this client)
                    if m.team() {
                        ui.add_enabled(false, egui::Button::new(text("UpdateTeamMembers")));
                    }
                    if ui.button(text("DeleteMission")).clicked() {
                        self.confirm = Some(m.id);
                        keep = false;
                    }
                });
            });
            if upload {
                self.upload(id, playfield, &mut shown);
            }
            let elsewhere = ctx.input(|i| i.pointer.any_pressed()) && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| !area.response.rect.contains(p));
            if !keep || elsewhere {
                self.menu = None;
            }
        } else {
            self.menu = None;
        }
        // ("ReallyWant2DeleteMission")
        if let Some(id) = self.confirm {
            let mut keep = true;
            egui::Window::new("Delete Mission").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(ctx, |ui| {
                ui.label(text("ReallyWant2DeleteMission"));
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, "Yes", true).clicked() {
                        shown.commands.push(Command::Delete(id));
                        keep = false;
                    }
                    if crate::chrome::button(ui, palette, "No", true).clicked() {
                        keep = false;
                    }
                });
            });
            if !keep {
                self.confirm = None;
            }
        }
        self.open = open;
        shown
    }
}

/// FUN_100360d5's markup for a mission; `item` finds a reward item's icon and name by its template.
fn page_of(m: &Mission, now: u32, level: i64, item: &dyn Fn(u32) -> Option<(Option<u32>, String)>) -> String {
    use crate::gui::iteminfo::section;
    use crate::gui::spells::printf_ints;
    let mut out = format!("<font color=CCItemUnknown>{}</font><br>", m.title);
    out += &section(&info_text("Description"), &m.description.replace("\\n", "<br>"), 0);
    out += "<br>";
    let left = m.remaining(now);
    if left > 0 {
        let minutes = left / 60;
        let hours = minutes / 60;
        out += &section(&info_text("RealtimeLeft"), &printf_ints(&info_text("DayHourMinute"), &[(hours / 24) as i32, (hours % 24) as i32, (minutes % 60) as i32]), 0);
    }
    if m.team() {
        out += &section(&info_text("TeamMissionRules"), &info_text("ThisIsaTeamMission"), 0);
    }
    out += &info_text("Reward");
    let mut pays = false;
    if m.credits > 0 {
        pays = true;
        out += &section(&info_text("Cash"), &printf_ints(&info_text("NumCredits"), &[m.credits as i32]), 0);
    }
    if m.xp > 0 && !(200..=220).contains(&level) {
        pays = true;
        out += &section(&info_text("Experience"), &printf_ints(&info_text("NumPoints"), &[m.xp as i32]), 0);
    }
    if !m.rewards.is_empty() {
        pays = true;
        out += &info_text("ItemReward");
        for r in &m.rewards {
            let Some((Some(icon), name)) = item(r.low).or_else(|| item(r.high)) else { continue };
            let name: String = name.chars().filter(|c| !"<>&\n\"".contains(*c)).collect();
            out += &format!("<a href=itemref://{}/{}/{} title=\"{name}\"><img src=rdb://{icon}></a> ", r.low, r.high, r.quality);
        }
    }
    if !pays {
        out += &info_text("NoReward");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mission(deadlines: &[u32]) -> Mission {
        Mission { id: (MISSION_TYPE, 7), title: "A mission".into(), description: "Go there\\nand back".into(), icon: None, place: None, flags: 2, credits: 0, xp: 0, rewards: Vec::new(), deadlines: deadlines.to_vec() }
    }

    /// `N3Msg_QuestRemainingTime`: the last deadline still ahead counts, a passed one or none leaves -1; the list shows
    /// hours and minutes, and nothing left as 00:00.
    #[test]
    fn the_time_left_follows_the_deadlines() {
        assert_eq!(mission(&[]).remaining(1000), -1);
        assert_eq!(mission(&[0]).remaining(1000), -1);
        assert_eq!(mission(&[900]).remaining(1000), -1);
        assert_eq!(mission(&[1000 + 172_799]).remaining(1000), 172_799);
        assert_eq!(mission(&[5000, 0, 3000, 900]).remaining(1000), 2000);
        assert_eq!(remaining_cell(172_799), "47:59");
        assert_eq!(remaining_cell(59), "00:00");
        assert_eq!(remaining_cell(-1), "00:00");
        assert_eq!(remaining_cell(3600 * 100 + 61), "100:01");
    }

    /// A `QuestFullUpdate`'s fields as the schema names them.
    #[test]
    fn the_list_is_read_from_the_message() {
        let s = |v: Vec<(&str, Value)>| Value::Struct(v.into_iter().map(|(n, v)| (n.to_string(), v)).collect());
        let id = |t: i64, i: i64| s(vec![("Type", Value::Int(t)), ("Instance", Value::Int(i))]);
        let action = |deadline: i64, pf: i64| s(vec![("Deadline", Value::Int(deadline)), ("PlayfieldId", id(40016, pf)), ("Position", s(vec![("X", Value::Float(10.0)), ("Y", Value::Float(2.0)), ("Z", Value::Float(30.0))]))]);
        let reward = s(vec![("LowId", Value::Int(11)), ("HighId", Value::Int(12)), ("Ql", Value::Int(118))]);
        let quest = s(vec![
            ("QuestId", id(MISSION_TYPE as i64, 99)),
            ("Flags", Value::Int(0x102)),
            ("ShortInfo", Value::Str("Short".into())),
            ("LongInfo", Value::Str("Long".into())),
            ("Credits", Value::Int(41720)),
            ("Xp", Value::Int(111218)),
            ("MissionItemData", Value::List(vec![reward])),
            ("QuestActions", Value::List(vec![action(0, 0), action(1_794_355_876, 505)])),
        ]);
        let list = missions_from(&s(vec![("Quests", Value::List(vec![quest]))]), &|_| None);
        assert_eq!(list.len(), 1);
        let m = &list[0];
        assert_eq!((m.id, m.title.as_str(), m.description.as_str()), ((MISSION_TYPE, 99), "Short", "Long"));
        assert!(m.team());
        assert_eq!((m.credits, m.xp), (41720, 111218));
        assert_eq!(m.rewards, vec![Reward { low: 11, high: 12, quality: 118 }]);
        assert_eq!(m.deadlines, vec![0, 1_794_355_876]);
        assert_eq!(m.place, Some((505, [10.0, 2.0, 30.0])));
        assert_eq!(m.remaining(1_794_355_876 - 3 * 3600 - 120), 3 * 3600 + 120);
        assert_eq!(m.url(), "itemid://56003/99");
    }

    /// The page's parts in FUN_100360d5's order: the name, the text with its written new line, the time left, the pay
    /// and the reward's link; a mission that pays nothing says so (whatever the words are).
    #[test]
    fn the_page_lists_the_time_and_the_pay() {
        let mut m = mission(&[1000 + 26 * 3600 + 5 * 60]);
        m.credits = 41720;
        m.xp = 111218;
        m.rewards = vec![Reward { low: 11, high: 12, quality: 118 }];
        let item = |id: u32| (id == 11).then(|| (Some(4242), "A \"thing\"".to_string()));
        let page = page_of(&m, 1000, 60, &item);
        assert!(page.starts_with("<font color=CCItemUnknown>A mission</font><br>"));
        assert!(page.contains("Go there<br>and back"));
        let at = |needle: &str| page.find(needle).unwrap_or_else(|| panic!("{needle} in {page}"));
        assert!(at("Go there") < at("41720") && at("41720") < at("111218") && at("111218") < at("itemref://11/12/118"));
        assert!(page.contains("<a href=itemref://11/12/118 title=\"A thing\"><img src=rdb://4242></a> "));
        // (level 200 to 220: no experience row)
        assert!(!page_of(&m, 1000, 205, &item).contains("111218"));
        // (no deadline ahead: no time row, so the page is shorter by it)
        let later = page_of(&m, 1000 + 27 * 3600, 60, &item);
        assert!(later.len() < page.len());
        let nothing = page_of(&mission(&[]), 1000, 60, &item);
        assert!(!nothing.contains("itemref://") && !nothing.contains("41720"));
    }

    /// `MAO_GAME`: the page with the game's own texts (text.mdb's category 506) still has every part, and each format
    /// took its numbers.
    #[test]
    fn the_page_takes_the_games_texts() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        crate::theme::set_game_dir(&game.to_string_lossy());
        let mut m = mission(&[1000 + 26 * 3600 + 5 * 60]);
        m.flags = 0x100;
        m.credits = 41720;
        m.xp = 111218;
        let page = page_of(&m, 1000, 60, &|_| None);
        if std::env::var_os("MAO_PRINT").is_some() {
            println!("{page}");
        }
        for name in ["Description", "RealtimeLeft", "DayHourMinute", "TeamMissionRules", "ThisIsaTeamMission", "Reward", "Cash", "NumCredits", "Experience", "NumPoints", "ItemReward", "NoReward"] {
            assert!(!crate::gui::iteminfo::text(name).is_empty(), "{name} is not in the text database");
        }
        assert!(page.contains("41720") && page.contains("111218") && !page.contains('%'), "{page}");
    }
}
