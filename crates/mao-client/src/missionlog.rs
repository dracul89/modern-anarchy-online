//! The mission journal: GUI.dll's `MissionView_c` (`mission_window`, "Missions", FUN_100d3644), the character's own
//! missions, not the booth's offers (`mission.rs`, `MissionSelectionView_c`).
//!
//! The server keeps the list in `QuestFullUpdate` (`465a4061`, every mission: its identity, title, text, icon and the
//! actions to do, each with a playfield and a place) and drops one with `Quest` action Delete (`212c487a`). The list
//! (`MissionListView_c`, FUN_100d3df7) shows each mission; its menu (FUN_100d3123) is "Look at Mission", "Upload to map"
//! (when the mission names a place: `N3Msg_GetQuestWorldPos`), "Update Team Members" (team missions) and "Delete
//! Mission" (asked first: "ReallyWant2DeleteMission"). Uploading (FUN_10067d5f) puts the place on the planet map and the
//! compass and says so: "MissionWptUploaded", then "MissionShownAsDot" in the mission's playfield, else
//! "MissionNotInArea"; a mission without a place "NoWptInfo".

use std::sync::Arc;

use mao_net::schema::Value;

use crate::gear::Icon;
use crate::theme::Palette;

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
            let place = match q.get("QuestActions") {
                Some(Value::List(actions)) => actions.iter().find_map(|a| {
                    let pf = a.get("PlayfieldId").map_or(0, |p| int(p, "Instance") as u32);
                    let p = a.get("Position")?;
                    (pf != 0).then(|| (pf, [float(p, "X"), float(p, "Y"), float(p, "Z")]))
                }),
                _ => None,
            };
            Mission { id, title: text(q, "ShortInfo"), description: text(q, "LongInfo"), icon: icon(int(q, "MissionIconId") as u32), place }
        })
        .collect()
}

/// A GUI text (text.mdb category 10000, the names GUI.dll asks for).
fn text(name: &str) -> String {
    let english = match name {
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

#[derive(Default)]
pub struct MissionLog {
    pub open: bool,
    pub missions: Vec<Mission>,
    selected: Option<(u32, u32)>,
    /// The mission whose deletion awaits an answer.
    confirm: Option<(u32, u32)>,
    textures: std::collections::HashMap<u32, egui::TextureHandle>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id)>,
    /// The mission whose menu is open, and where.
    menu: Option<(usize, egui::Pos2)>,
}

/// What a frame of the window did.
#[derive(Default)]
pub struct Shown {
    pub commands: Vec<Command>,
    /// A waypoint uploaded, and the lines to tell.
    pub waypoint: Option<(u32, [f32; 3])>,
    pub lines: Vec<String>,
    /// A mission's page for the info view (its markup).
    pub info: Option<String>,
}

impl MissionLog {
    /// The server's full list.
    pub fn set(&mut self, missions: Vec<Mission>) {
        self.missions = missions;
        if self.selected.is_some_and(|s| !self.missions.iter().any(|m| m.id == s)) {
            self.selected = None;
        }
    }

    /// The server dropped a mission.
    pub fn remove(&mut self, id: (u32, u32)) {
        self.missions.retain(|m| m.id != id);
    }

    /// FUN_10067d5f: the mission's place as the waypoint, and what is told.
    fn upload(m: &Mission, playfield: u32, shown: &mut Shown) {
        match m.place {
            Some((pf, p)) => {
                shown.waypoint = Some((pf, p));
                shown.lines.push(text("MissionWptUploaded"));
                shown.lines.push(text(if pf == playfield { "MissionShownAsDot" } else { "MissionNotInArea" }));
            }
            None => shown.lines.push(text("NoWptInfo")),
        }
    }

    /// The window: the `MissionListView_c` grid of the missions' icons (the item windows' 47-pixel cells), a click shows
    /// the mission in the info view (FUN_100d2cfa: its page), a right click opens its menu (FUN_100d3123).
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, playfield: u32) -> Shown {
        let mut shown = Shown::default();
        if !self.open {
            return shown;
        }
        if self.views.is_none() {
            use crate::gui::view::{Layout, Node, Tree};
            let mut tree = Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
            tree.get_mut(root).layout = Layout::Base;
            let list = crate::gui::multilist::create_grid(&mut tree, "MissionListView", -1, 4);
            tree.add_child(root, list);
            self.views = Some((tree, root, list));
        }
        let textures = &mut self.textures;
        let rows: Vec<crate::gui::multilist::Item> = self
            .missions
            .iter()
            .enumerate()
            .map(|(k, m)| crate::gui::multilist::Item {
                id: k as i64,
                icon: m.icon.as_ref().map(|icon| textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("mission-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id()),
                cells: vec![(1, crate::gui::multilist::CellView::Text(m.title.clone()))],
                keys: vec![(1, m.title.clone())],
                ..Default::default()
            })
            .collect();
        let Some((tree, root, list)) = self.views.as_mut() else { return shown };
        let (root, list) = (*root, *list);
        if crate::gui::multilist::items(tree, list) != rows.as_slice() {
            crate::gui::multilist::set_items(tree, list, rows);
        }
        crate::gui::multilist::update(tree, list);
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut hovered = None;
        // (named apart from the mission selection window, which Escape closes; the journal keeps its own place)
        crate::chrome::window(ctx, palette, "MissionView", "Missions", &mut open, |w| w.default_size(egui::vec2(4.0 * crate::gui::multilist::grid::STEP + 32.0, 2.0 * crate::gui::multilist::grid::STEP + 12.0)).resizable(true), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                hovered = crate::gui::multilist::item_at(tree, list, response.rect.min, at).map(|k| (k as usize, at));
            }
        });
        let missions = &self.missions;
        for (_, event) in crate::gui::multilist::take_events(tree) {
            if let crate::gui::multilist::ItemEvent::Clicked(k, _) = event
                && let Some(m) = missions.get(k as usize)
            {
                crate::gui::multilist::select(tree, list, Some(k));
                self.selected = Some(m.id);
                shown.info = Some(format!("<font color=CCInfoHeadline>{}</font><br><br>{}", m.title, m.description));
            }
        }
        tree.signals.clear();
        if let Some((k, at)) = hovered {
            if ctx.input(|i| i.pointer.secondary_clicked()) {
                self.menu = Some((k, at));
            } else if let Some(m) = missions.get(k)
                && self.menu.is_none()
            {
                egui::Area::new(egui::Id::unique("mission-tip")).order(egui::Order::Tooltip).fixed_pos(at + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(&m.title);
                    });
                });
            }
        }
        // (FUN_100d3123's menu)
        if let Some((k, at)) = self.menu
            && let Some(m) = missions.get(k)
        {
            let mut keep = true;
            let area = egui::Area::new(egui::Id::unique("mission-menu")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    if ui.button(text("Look@Mission")).clicked() {
                        shown.info = Some(format!("<font color=CCInfoHeadline>{}</font><br><br>{}", m.title, m.description));
                        keep = false;
                    }
                    if m.place.is_some() && ui.button(text("Upload2Map")).clicked() {
                        MissionLog::upload(m, playfield, &mut shown);
                        keep = false;
                    }
                    // (team missions are not known to the client yet: `N3Msg_IsTeamMission`)
                    ui.add_enabled(false, egui::Button::new(text("UpdateTeamMembers")));
                    if ui.button(text("DeleteMission")).clicked() {
                        self.confirm = Some(m.id);
                        keep = false;
                    }
                });
            });
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

