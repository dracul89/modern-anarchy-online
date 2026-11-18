//! The team: an invitation to answer, the members with their health and nano, and leaving. The protocol is the Arete Landing
//! capture's (2026-10-30, the original client): an invitation is the server's `CharacterAction` 0x1a naming who asks; the
//! answer is the client's 0x1c naming them with 1 (join) or 0 (no); the server then sends `TeamMember` (`46312d2e`: the
//! character, the team, a word, the level, the profession, the name) for each member and `TeamMemberInfo` (`28784248`:
//! health, max health, max nano, nano) as they change. Leaving is the client's 0x18; the server's 0x20 names who left (oneself:
//! the team is gone). aosharp's `Team` names the same actions. Asking somebody in is the client's own 0x1a naming them
//! (Gamecode `N3Msg_TeamJoinRequest`), from the window's Recruit button (GUI.dll FUN_100782b2).
//!
//! A raid is up to six teams. Gamecode's team object (made by FUN_10065dff) has six member lists and, at +0x38, the
//! own team's place among them, -1 outside a raid; outside a raid the own team is list 0. Here the own team is
//! always [`Team::members`] (what the team window, the chat and the missions read), the place is [`Team::raid`], and
//! the raid's other teams are [`Team::others`] by their place ([`Team::raid_team`] is the original's
//! `N3Msg_GetTeamMemberList` in a raid). The lists come with the world-entry message (its team flags' bit 1: the own
//! place and all six lists), a member with `TeamMember` (its third field is the member's team's place), one leaves
//! with `CharacterAction` 0x20 (its second parameter the place), and `RaidIIR_c` of type 0 says the own team is
//! place 0 of a new raid. No capture has a raid: all of it is read from the code.

use std::sync::Mutex;

use crate::theme::Palette;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mate {
    pub id: u32,
    pub name: String,
    pub level: i64,
    pub profession: i64,
    /// (now, max)
    pub health: (i64, i64),
    pub nano: (i64, i64),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Team {
    pub members: Vec<Mate>,
    /// Who asks us to join (their id and name).
    pub invite: Option<(u32, String)>,
    /// The leader (`TeamViewModule_c` +0xe0, from interface message 0x17, the server's `CharacterAction` 0x23 naming it):
    /// its name is drawn in 0x11eeaa (FUN_10078bc2 → FUN_10077b3d) and `IsTeamLeader` is whether it is oneself.
    pub leader: Option<u32>,
    /// The team's level (`GetTeamLevel`, +0xec, from interface message 0x10, the server's `CharacterAction` 0x1b with the
    /// level in the target's instance), shown in the header.
    pub level: Option<i64>,
    /// The client's own character (the connection keeps it told).
    pub own: u32,
    /// The own team's place in the raid, 0 to 5 (Gamecode's team object +0x38); none outside a raid.
    pub raid: Option<usize>,
    /// The raid's other teams by their place (the own team's place stays empty here: it is `members`).
    pub others: [Vec<Mate>; 6],
}

impl Team {
    /// `N3Msg_IsInRaidTeam` (0x10018439): in a team that has a place in a raid.
    pub fn in_raid(&self) -> bool {
        !self.members.is_empty() && self.raid.is_some()
    }

    /// Whether the client's own character leads its team, as `N3Msg_CreateRaid` asks (0x1001856a: in a team whose
    /// leader, as the server named it, is oneself; not [`is_leader`]'s guess of the first member).
    pub fn leads(&self) -> bool {
        !self.members.is_empty() && self.leader == Some(self.own)
    }

    /// The members of the raid's team at `place` (`N3Msg_GetTeamMemberList` in a raid): the own team at its place.
    pub fn raid_team(&self, place: usize) -> &[Mate] {
        if self.raid == Some(place) { &self.members } else { self.others.get(place).map_or(&[], Vec::as_slice) }
    }

    /// The list a member of the team at `place` is kept in, as the server names the place (`TeamMember`'s field,
    /// 0x20's parameter; below 0 outside a raid, which Gamecode takes for list 0, FUN_10066361): the own team's, or
    /// one of the others'. None for a place past the sixth.
    fn list(&mut self, place: i32) -> Option<&mut Vec<Mate>> {
        let place = place.max(0) as usize;
        if self.raid.is_none() || self.raid == Some(place) { Some(&mut self.members) } else { self.others.get_mut(place) }
    }

    /// A member told by the server (`TeamMember`: FUN_1007a57c → FUN_10066361 adding): one's own character's
    /// place is the own team's from now on; the member is put in its team's list.
    pub fn joined(&mut self, id: u32, place: i32, name: &str, level: i64, profession: i64) {
        if id == self.own {
            self.set_place((0..6).contains(&place).then_some(place as usize));
        }
        let Some(list) = self.list(place) else { return };
        let at = match list.iter().position(|m| m.id == id) {
            Some(at) => at,
            None => {
                list.push(Mate { id, ..Mate::default() });
                list.len() - 1
            }
        };
        (list[at].name, list[at].level, list[at].profession) = (name.to_string(), level, profession);
    }

    /// The own team's place in the raid changes: the team kept for that place among the others is the own one's
    /// part now (its members join `members`), and nothing of the own team is left among the others.
    pub fn set_place(&mut self, place: Option<usize>) {
        self.raid = place;
        if let Some(place) = place {
            for mate in std::mem::take(&mut self.others[place]) {
                if !self.members.iter().any(|m| m.id == mate.id) {
                    self.members.push(mate);
                }
            }
        }
    }

    /// One left (the server's `CharacterAction` 0x20, case 8 of FUN_1005d61f → FUN_10066361 removing): oneself,
    /// the team and the raid are gone (FUN_10065ec2: leader, place and all six lists); another is taken off the list
    /// of the place the action names.
    pub fn left(&mut self, id: u32, place: i32) {
        if id == self.own {
            self.members.clear();
            self.others = Default::default();
            (self.leader, self.raid) = (None, None);
        } else if let Some(list) = self.list(place) {
            list.retain(|m| m.id != id);
        }
    }

    /// A member wherever it is kept, the own team or another of the raid.
    pub fn known(&mut self, id: u32) -> Option<&mut Mate> {
        self.members.iter_mut().chain(self.others.iter_mut().flatten()).find(|m| m.id == id)
    }

    /// The whole of the teams as the world-entry message has them (`FullCharacterIIR_t`'s exec FUN_10073f78: the
    /// leader, and the own team's list, or with a raid the own place and the six lists). A member that stays keeps
    /// the health and nano last heard.
    pub fn set_all(&mut self, leader: u32, place: Option<usize>, lists: Vec<Vec<Mate>>) {
        let before: Vec<Mate> = self.members.drain(..).chain(self.others.iter_mut().flat_map(|l| l.drain(..))).collect();
        let kept = |mut mate: Mate| {
            if let Some(old) = before.iter().find(|m| m.id == mate.id) {
                (mate.health, mate.nano) = (old.health, old.nano);
            }
            mate
        };
        if let Some(place) = place {
            self.raid = Some(place);
        }
        for (k, list) in lists.into_iter().enumerate().take(6) {
            let list: Vec<Mate> = list.into_iter().map(&kept).collect();
            // (without a raid there is the one list, the own team's)
            if place.is_none() || place == Some(k) { self.members = list } else { self.others[k] = list }
        }
        self.leader = Some(leader);
    }
    /// A member, added when new.
    pub fn member(&mut self, id: u32) -> &mut Mate {
        if let Some(at) = self.members.iter().position(|m| m.id == id) {
            return &mut self.members[at];
        }
        self.members.push(Mate { id, ..Mate::default() });
        self.members.last_mut().unwrap()
    }
}

#[cfg(test)]
mod raid_tests {
    use super::*;

    fn names(list: &[Mate]) -> Vec<&str> {
        list.iter().map(|m| m.name.as_str()).collect()
    }

    /// Outside a raid every member is the own team's, whatever place the server names (-1 in the captures). Once
    /// the own character has a place, members of that place are the own team's and the others' go to their teams;
    /// one who leaves is taken off the list of the place named; oneself leaving ends it all.
    #[test]
    fn members_go_to_their_teams() {
        let mut t = Team { own: 1, ..Team::default() };
        t.joined(1, -1, "Own", 5, 4);
        t.joined(2, -1, "Mate", 7, 3);
        assert_eq!((names(&t.members), t.raid, t.in_raid()), (vec!["Own", "Mate"], None, false));
        // (told again, a member is not there twice, and takes the new level)
        t.joined(2, -1, "Mate", 8, 3);
        assert_eq!((t.members.len(), t.members[1].level, t.members[1].profession), (2, 8, 3));
        // (the raid: the own team is place 0; another team's member comes)
        t.set_place(Some(0));
        t.joined(9, 3, "Far", 20, 6);
        t.joined(3, 0, "Third", 6, 1);
        assert_eq!((names(&t.members), names(t.raid_team(3)), names(t.raid_team(0)), t.raid_team(1).len()), (vec!["Own", "Mate", "Third"], vec!["Far"], vec!["Own", "Mate", "Third"], 0));
        assert!(t.in_raid() && t.known(9).is_some() && t.known(77).is_none());
        // (a place past the sixth is nobody's)
        t.joined(10, 6, "Nowhere", 1, 1);
        assert!(t.known(10).is_none());
        // (one's own place told with one's own member message: the team kept for that place joins the own)
        let mut moved = Team { own: 1, ..Team::default() };
        moved.joined(2, -1, "Mate", 7, 3);
        moved.set_place(Some(0));
        moved.joined(8, 4, "Later", 9, 2);
        moved.joined(1, 4, "Own", 5, 4);
        assert_eq!((moved.raid, names(&moved.members)), (Some(4), vec!["Mate", "Later", "Own"]));
        assert!(moved.others.iter().all(Vec::is_empty));
        // (leaving: another by its place, then oneself)
        t.left(9, 0);
        assert!(t.known(9).is_some());
        t.left(9, 3);
        t.left(2, 0);
        assert_eq!((names(&t.members), t.raid_team(3).len()), (vec!["Own", "Third"], 0));
        t.joined(9, 3, "Far", 20, 6);
        t.leader = Some(1);
        t.left(1, 0);
        assert!(t.members.is_empty() && t.others.iter().all(Vec::is_empty) && t.raid.is_none() && t.leader.is_none());
    }

    /// The world-entry message's teams: without a raid the one list is the own team; with one, the list at the own
    /// place is, and the rest are the others. A member that stays keeps its bars.
    #[test]
    fn the_teams_are_set_whole() {
        let mate = |id: u32, name: &str| Mate { id, name: name.to_string(), ..Mate::default() };
        let mut t = Team { own: 1, ..Team::default() };
        t.joined(2, -1, "Mate", 7, 3);
        t.known(2).unwrap().health = (40, 50);
        t.set_all(2, None, vec![vec![mate(2, "Mate"), mate(1, "Own")]]);
        assert_eq!((names(&t.members), t.leader, t.raid, t.members[0].health), (vec!["Mate", "Own"], Some(2), None, (40, 50)));
        assert!(t.leads() == false && !t.in_raid());
        t.set_all(1, Some(2), vec![vec![mate(7, "Far")], Vec::new(), vec![mate(1, "Own"), mate(2, "Mate")], Vec::new(), Vec::new(), vec![mate(8, "Last")]]);
        assert_eq!((names(&t.members), names(t.raid_team(0)), names(t.raid_team(5)), t.raid, t.leads()), (vec!["Own", "Mate"], vec!["Far"], vec!["Last"], Some(2), true));
        assert_eq!(t.members[1].health, (40, 50));
        assert!(t.others[2].is_empty());
    }
}

/// What the window asks the connection to send.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Answer an invitation from this character.
    Reply(u32, bool),
    Leave,
    /// Ask the target to join (the Recruit button).
    Invite,
    /// A member clicked: it becomes the target.
    Select(u32),
    /// A member's menu: its page in the info window, the leadership handed to it, or it put out of the team.
    Info(u32),
    Lead(u32),
    Kick(u32),
}

static TEAM: Mutex<Team> = Mutex::new(Team { members: Vec::new(), invite: None, leader: None, level: None, own: 0, raid: None, others: [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()] });
static COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());
/// The team's and the raid's members whose characters the client knows now (they are in the playfield:
/// `N3Msg_isDynelInTree`), as the connection last told: the corpse's Looter menu enables only those.
static PRESENT: Mutex<Vec<u32>> = Mutex::new(Vec::new());

pub fn set_present(ids: Vec<u32>) {
    *PRESENT.lock().unwrap_or_else(|e| e.into_inner()) = ids;
}

pub fn present(id: u32) -> bool {
    PRESENT.lock().unwrap_or_else(|e| e.into_inner()).contains(&id)
}

/// The team as the connection last heard it.
pub fn with<R>(f: impl FnOnce(&mut Team) -> R) -> R {
    f(&mut TEAM.lock().unwrap_or_else(|e| e.into_inner()))
}

pub fn take_commands() -> Vec<Command> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

pub fn send(c: Command) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(c);
}

/// `TeamViewModule_c::SlotTeamEvent` (0x1007ac17) on message 0x10 (`CharacterAction` 0x1b, case 6 0x1005d7fc, for one's own
/// character): the team level is kept, and in a team the window is told (signal +0x14) and the chat line text.mdb 10000's
/// "TeamLevel_isX" fed the level is shown (`LDBformat::Dump`, global signal +0x17c). The line, when there is one.
pub fn set_level(team: &mut Team, level: i64) -> Option<String> {
    team.level = Some(level);
    let text = crate::feedback::texts().and_then(|t| t.named(10000, "TeamLevel_isX")).unwrap_or("Team level is %u.");
    (!team.members.is_empty()).then(|| mao_formats::ldb::format(text, &[mao_formats::ldb::Arg::Int(level)]))
}

/// Whether a member's name is the leader's colour: the leader 0x17 named, else (none heard yet) the first member, as the
/// server lists the team from its leader.
pub fn is_leader(team: &Team, k: usize) -> bool {
    match team.leader {
        Some(leader) => team.members.get(k).is_some_and(|m| m.id == leader),
        None => k == 0,
    }
}

/// The team window, GUI.dll's `TeamView_c` (FUN_10078c14, `team_view`, 1000 by 1000 at most): a header line (text.mdb
/// 10000's "Team level: %u  Team side: %s", or "No team"), a row of buttons (FUN_100785d7: without a team Recruit at
/// the left; in a team Leave at the right and, for the leader, Recruit after it; Recruit asks the target in,
/// FUN_100782b2) and six member views (FUN_1007981b): above
/// the first and under each a `GFX_GUI_FRIENDLIST_SPLITTER`, then the member's name in the middle (the leader's in
/// 0x11eeaa, others white, FUN_10077b3d) and its health and nano bars (`PowerbarView_c` of `GFX_GUI_BAR_HEALTH_TEAM` /
/// `_NANO_TEAM` over their `_BG`s). Places without a member are hidden (FUN_1007a154). An invitation asks "Want to join
/// %s's team?" (text.mdb 10000). A member's view takes the mouse (FUN_1007a23a): a click makes the member the target,
/// a right click brings a menu of "info on" the member and, for the leader on another member, making it the leader
/// and putting it out of the team (`N3Msg_TransferTeamLeadership`, `N3Msg_KickTeamMember`).
#[derive(Default)]
pub struct TeamWindow {
    pub open: bool,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id)>,
    /// A member's menu and whom it is for; whether it has the leader's two entries.
    popup: Option<crate::gui::popup::PopupMenu>,
    popup_for: (u32, bool),
}

/// The entries of a member's menu (FUN_1007a23a): the game's texts with the member's name, the leader's two only when
/// the member is another and the client's character leads.
pub fn member_menu(name: &str, own: bool, leading: bool) -> Vec<String> {
    let text = |key: &str, own_words: &str| crate::gui::xml::ldb_named(10000, key, own_words);
    let mut entries = vec![format!("{}{name}", text("InfoOn", "Info on "))];
    if !own && leading {
        entries.push(mao_formats::ldb::format(&text("MakeXLeader", "Make %s the leader"), &[mao_formats::ldb::Arg::Text(name)]));
        entries.push(format!("{}{name}", text("Kick_", "Put out ")));
    }
    entries
}

/// An XML attribute's text.
fn attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The window's views as the original builds them.
fn view_xml() -> String {
    let splitter = "<BitmapView bitmap_id=\"GFX_GUI_FRIENDLIST_SPLITTER\" max_size=\"Point(16000,-1)\"/>";
    let mut mates = String::new();
    for k in 0..6 {
        let top = if k == 0 { splitter } else { "" };
        mates += &format!(
            "<View name=\"mate{k}\" view_layout=\"vertical\">{top}\
               <View view_layout=\"horizontal\">\
                 <TextView name=\"lead{k}\" value=\"\" layout_borders=\"Rect(3,2,0,2)\"/>\
                 <HLayoutSpacer/>\
                 <TextView name=\"name{k}\" value=\"\" layout_borders=\"Rect(5,2,5,2)\"/>\
                 <HLayoutSpacer/>\
                 <View view_layout=\"vertical\" layout_borders=\"Rect(2,2,2,2)\">\
                   <PowerBar name=\"hp{k}\" bg_gfx=\"GFX_GUI_BAR_HEALTH_TEAM_BG\" full_gfx=\"GFX_GUI_BAR_HEALTH_TEAM\"/>\
                   <PowerBar name=\"np{k}\" bg_gfx=\"GFX_GUI_BAR_NANO_TEAM_BG\" full_gfx=\"GFX_GUI_BAR_NANO_TEAM\"/>\
                 </View>\
               </View>{splitter}\
             </View>"
        );
    }
    let leave = attr(&crate::gui::xml::ldb_text("#10000:5421253"));
    let recruit = attr(&crate::gui::xml::ldb_named(10000, "Recruit", "Invite"));
    format!(
        "<View view_layout=\"vertical\">\
           <BorderView layout_borders=\"Rect(4,2,4,2)\"><TextView name=\"header\" value=\"\" layout_borders=\"Rect(4,2,4,2)\"/></BorderView>\
           <View name=\"buttons\" view_layout=\"horizontal\" layout_borders=\"Rect(0,5,0,5)\">\
             <Button name=\"recruit0\" label=\"{recruit}\" layout_borders=\"Rect(5,0,5,0)\"/>\
             <HLayoutSpacer/><Button name=\"leave\" label=\"{leave}\" layout_borders=\"Rect(5,0,5,0)\"/>\
             <Button name=\"recruit1\" label=\"{recruit}\" layout_borders=\"Rect(5,0,5,0)\"/>\
           </View>{mates}\
         </View>"
    )
}

fn set_text(tree: &mut crate::gui::view::Tree, root: crate::gui::view::Id, name: &str, text: &str) {
    if let Some(id) = tree.find(root, name)
        && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id)
        && t.text != text
    {
        t.set_text(text);
        tree.relayout = true;
    }
}

fn show_view(tree: &mut crate::gui::view::Tree, root: crate::gui::view::Id, name: &str, on: bool) {
    if let Some(v) = tree.find(root, name) {
        let want = u32::from(!on);
        if tree.get(v).hidden.min(1) != want {
            tree.get_mut(v).flags |= crate::gui::view::FLAG_COLLAPSE;
            tree.get_mut(v).hidden = want;
            tree.relayout = true;
        }
    }
}

impl TeamWindow {
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) {
        let team = with(|t| t.clone());
        // (an invitation: the original's question)
        if let Some((id, name)) = &team.invite {
            let question = crate::gui::xml::ldb_text("#10000:110085117").replace("%s", name);
            let answer = crate::chrome::question(ctx, palette, "team-invite", &crate::gui::xml::ldb_text("#10000:3860335"), &question);
            if let Some(join) = answer {
                send(Command::Reply(*id, join));
                with(|t| t.invite = None);
                if join {
                    self.open = true;
                }
            }
        }
        if !self.open {
            return;
        }
        if self.views.is_none() {
            let doc = crate::gui::xml::parse(&view_xml());
            let mut tree = crate::gui::view::Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            if let Some(root) = doc.first().and_then(|e| crate::gui::create(&mut tree, e)) {
                self.views = Some((tree, root));
            }
        }
        let Some((tree, root)) = self.views.as_mut() else { return };
        let root = *root;
        // (the header, the buttons in a team, a member in each of the first places)
        let header = if team.members.is_empty() {
            crate::gui::xml::ldb_text("#10000:89435005")
        } else {
            // (`GetTeamLevel`; before the server has told it, the highest member's)
            let level = team.level.unwrap_or_else(|| team.members.iter().map(|m| m.level).max().unwrap_or(0));
            crate::gui::xml::ldb_text("#10000:56702133").replacen("%u", &level.to_string(), 1).replacen("%s", "", 1)
        };
        set_text(tree, root, "header", &header);
        // (FUN_100785d7: Recruit alone without a team; Leave in one, and Recruit beside it for the leader)
        let in_team = !team.members.is_empty();
        let leading = in_team && team.members.iter().position(|m| m.id == team.own).is_some_and(|k| is_leader(&team, k));
        show_view(tree, root, "recruit0", !in_team);
        show_view(tree, root, "leave", in_team);
        show_view(tree, root, "recruit1", leading);
        for k in 0..6 {
            let mate = team.members.get(k);
            show_view(tree, root, &format!("mate{k}"), mate.is_some());
            let Some(m) = mate else { continue };
            let colour = if is_leader(&team, k) { "#11eeaa" } else { "#ffffff" };
            set_text(tree, root, &format!("name{k}"), &format!("<font color={colour}>{}</font>", m.name));
            let share = |(now, max): (i64, i64)| if max > 0 { now as f32 / max as f32 } else { 0.0 };
            if let Some(id) = tree.find(root, &format!("hp{k}")) {
                crate::gui::powerbar::set_value(tree, id, share(m.health));
            }
            if let Some(id) = tree.find(root, &format!("np{k}")) {
                crate::gui::powerbar::set_value(tree, id, share(m.nano));
            }
        }
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(tree, root, false);
        let mut open = self.open;
        let title = crate::gui::xml::ldb_text("#10000:371581");
        let mut pressed: Option<(usize, bool, egui::Pos2)> = None;
        crate::chrome::window(ctx, palette, "Team", &title, &mut open, |w| w.default_pos(crate::layout::pos("Team", egui::pos2(20.0, 300.0))).default_size(min.max(egui::vec2(200.0, 0.0))).resizable(false), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            // (a member's view under the mouse: its click or right click)
            let (left, right, at) = ui.input(|i| (i.pointer.primary_clicked(), i.pointer.secondary_clicked(), i.pointer.interact_pos()));
            if let Some(at) = at.filter(|_| (left || right) && response.contains_pointer()) {
                pressed = (0..team.members.len()).find(|k| tree.find(root, &format!("mate{k}")).is_some_and(|v| crate::gui::host::screen_rect(tree, v, response.rect.min).contains(at))).map(|k| (k, right, at));
            }
        });
        match pressed {
            Some((k, false, _)) => send(Command::Select(team.members[k].id)),
            Some((k, true, at)) => {
                let m = &team.members[k];
                let entries: Vec<(String, bool)> = member_menu(&m.name, m.id == team.own, leading).into_iter().map(|e| (e, true)).collect();
                self.popup_for = (m.id, entries.len() > 1);
                self.popup = Some(crate::chrome::menu_at(ctx, at, &entries));
            }
            None => {}
        }
        if let Some(chosen) = crate::chrome::menu(ctx, palette, "team-member-menu", &mut self.popup) {
            let who = self.popup_for.0;
            match chosen {
                0 => send(Command::Info(who)),
                1 if self.popup_for.1 => send(Command::Lead(who)),
                2 if self.popup_for.1 => send(Command::Kick(who)),
                _ => {}
            }
        }
        for (id, signal) in std::mem::take(&mut tree.signals) {
            if signal != crate::gui::host::Signal::Clicked {
                continue;
            }
            match tree.get(id).name.as_str() {
                "leave" => send(Command::Leave),
                "recruit0" | "recruit1" => send(Command::Invite),
                _ => {}
            }
        }
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_members_menu_has_the_leaders_entries_for_another_only() {
        // (oneself, or without the leadership: the info alone; the leader on another: three, each naming the member)
        assert_eq!(member_menu("Me", true, true).len(), 1);
        assert_eq!(member_menu("Other", false, false).len(), 1);
        let entries = member_menu("Other", false, true);
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().all(|e| e.contains("Other")), "{entries:?}");
    }

    #[test]
    fn members_are_added_once() {
        let mut t = Team::default();
        t.member(5).name = "A".into();
        t.member(5).level = 7;
        t.member(6);
        assert_eq!(t.members.len(), 2);
        assert_eq!((t.members[0].name.as_str(), t.members[0].level), ("A", 7));
    }

    /// The level is kept, and told only in a team; the leader is the one 0x17 named, else the first member.
    #[test]
    fn level_and_leader() {
        let mut t = Team::default();
        assert_eq!(set_level(&mut t, 3), None);
        t.member(5);
        t.member(6);
        assert!(set_level(&mut t, 4).is_some_and(|l| l.contains('4')));
        assert_eq!(t.level, Some(4));
        assert!(is_leader(&t, 0) && !is_leader(&t, 1));
        t.leader = Some(6);
        assert!(!is_leader(&t, 0) && is_leader(&t, 1));
    }
}
