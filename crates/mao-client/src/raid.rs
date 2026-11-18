//! The raid window, GUI.dll's `RaidViewModule_c` (`raid_window`, the control center's Team menu "#Raid"): the skin's
//! `Views/Raid.xml`, six teams of six lines (a number, the profession's picture `t<n>p<m>_Prof`, the level `t<n>p<m>_Level`
//! and the name `t<n>p<m>_Name`), with "Convert To Raid" and "Raid Locks" above them.
//!
//! As the original's (`RaidWindow_c`, FUN_10103127; its refill FUN_10102bbe):
//!
//! * **The teams are filled only in a raid** (`N3Msg_IsInRaidTeam`): each team's members in their order
//!   (`N3Msg_GetTeamMemberList(place)`: name, the profession's picture, the level), the one leader's name (of one's
//!   own team, as the server named it) in 0x11eeaa. Outside a raid, in a team or not, all 36 lines are empty. (Until
//!   2026-11-18 the own team stood as Team 1 in a plain team too.) Nothing marks one's own character, a dead member
//!   or an absent one.
//! * **Convert To Raid** is enabled in a team that is no raid yet; a click asks `N3Msg_CreateRaid`, which sends the
//!   raid command 1 only for the team's leader and says nothing otherwise. **Raid Locks** always asks for the lock
//!   list (command 6); the server's answer is said in the chat (`net.rs`).
//!
//! No capture has a raid: the teams' data (`team.rs`) and both commands are read from the code and were never
//! seen on the wire. Not ported: a line's menu for the leader (make leader, kick), dragging a member onto another
//! team (`N3Msg_MoveRaidMember`; `/raid move` does it by numbers) and the raid bars (`RaidBar.xml`: a small window
//! per member, made by dragging a member out of this one).

use crate::theme::Palette;

/// A member as the window last showed it: id, name, level, profession, leader.
type Shown = Vec<(u32, String, i64, i64, bool)>;

/// What the six teams show: in a raid each team's members (`Team::raid_team`), outside one nothing.
pub fn teams(team: &crate::team::Team) -> [Shown; 6] {
    std::array::from_fn(|place| {
        if !team.in_raid() {
            return Vec::new();
        }
        team.raid_team(place).iter().map(|m| (m.id, m.name.clone(), m.level, m.profession, team.leader == Some(m.id))).collect()
    })
}

/// What a click on one of the two buttons sends: the raid command's number. "Convert To Raid" (`CreateRaid`) only
/// for the leader of a team (`N3Msg_CreateRaid`), "Raid Locks" (`ListLocks`) always (`N3Msg_RequestRaidLocks`).
pub fn button(name: &str, team: &crate::team::Team) -> Option<u32> {
    match name {
        "CreateRaid" if team.leads() => Some(1),
        "ListLocks" => Some(6),
        _ => None,
    }
}

#[derive(Default)]
pub struct RaidWindow {
    pub open: bool,
    window: Option<crate::gui::window::XmlWindow>,
    shown: Option<[Shown; 6]>,
}

/// The skin's picture of a profession (`GFX_GUI_ICON_PROFESSION_<n>`, the profession's number).
pub fn profession_picture(profession: i64) -> String {
    if (1..=15).contains(&profession) {
        format!("GFX_GUI_ICON_PROFESSION_{profession}")
    } else {
        String::new()
    }
}

/// Fills team `t`'s six lines (`t<t>p<p>_*`) with `members` (none past them).
fn fill_team(tree: &mut crate::gui::view::Tree, root: crate::gui::view::Id, t: usize, members: &[(u32, String, i64, i64, bool)]) {
    for p in 1..=6 {
        let m = members.get(p - 1);
        if let Some(id) = tree.find(root, &format!("t{t}p{p}_Level"))
            && let Some(v) = tree.control_mut::<crate::gui::controls::TextView>(id)
        {
            v.set_text(&m.map(|m| m.2.to_string()).unwrap_or_default());
        }
        if let Some(id) = tree.find(root, &format!("t{t}p{p}_Name"))
            && let Some(v) = tree.control_mut::<crate::gui::controls::TextView>(id)
        {
            let name = m.map(|m| if m.4 { format!("<font color=#11eeaa>{}</font>", m.1) } else { m.1.clone() }).unwrap_or_default();
            v.set_text(&name);
        }
        if let Some(id) = tree.find(root, &format!("t{t}p{p}_Prof"))
            && let Some(v) = tree.control_mut::<crate::gui::controls::BitmapView>(id)
        {
            v.bitmap = m.map(|m| profession_picture(m.3)).unwrap_or_default();
        }
    }
}

impl RaidWindow {
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) {
        if !self.open {
            self.window = None;
            return;
        }
        if self.window.is_none() {
            let Some(mut w) = crate::gui::window::XmlWindow::load(ctx, "Raid", "Raid", "Raid.xml") else {
                self.open = false;
                return;
            };
            w.tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            w.default_size = egui::vec2(560.0, 520.0);
            self.window = Some(w);
            self.shown = None;
        }
        let Some(w) = self.window.as_mut() else { return };
        let team = crate::team::with(|t| t.clone());
        let members = teams(&team);
        if self.shown.as_ref() != Some(&members) {
            let root = w.root;
            for (place, shown) in members.iter().enumerate() {
                fill_team(&mut w.tree, root, place + 1, shown);
            }
            w.tree.relayout = true;
            self.shown = Some(members);
        }
        // (the button as the original has it at opening and after each of its signals: in a team, not in a raid)
        let root = w.root;
        if let Some(id) = w.tree.find(root, "CreateRaid") {
            let enabled = !team.members.is_empty() && !team.in_raid();
            if w.tree.get(id).enabled != enabled {
                w.tree.get_mut(id).enabled = enabled;
            }
        }
        let signals = w.show(ctx, palette);
        for (id, signal) in signals {
            if signal != crate::gui::host::Signal::Clicked {
                continue;
            }
            if let Some(command) = button(w.tree.get(id).name.as_str(), &team) {
                crate::net::raid_command(command, [(0, 0); 3]);
            }
        }
        self.open = w.open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MAO_GAME`: the game's `Raid.xml` has the six teams' lines and the two buttons, and the lines fill.
    #[test]
    fn the_skin_has_the_raid_view() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let dir = std::path::Path::new(&game).join("cd_image/gui/Default");
        let (mut tree, root) = crate::gui::load_view(&[dir], "Raid.xml", "").unwrap();
        for name in ["CreateRaid", "ListLocks", "t1p1_Prof", "t1p1_Level", "t1p1_Name", "t6p6_Name"] {
            assert!(tree.find(root, name).is_some(), "{name}");
        }
        fill_team(&mut tree, root, 1, &[(7, "Nerfit".to_string(), 42, 6, true)]);
        let id = tree.find(root, "t1p1_Name").unwrap();
        assert!(tree.control::<crate::gui::controls::TextView>(id).unwrap().text.contains("Nerfit"));
        let id = tree.find(root, "t1p1_Prof").unwrap();
        assert_eq!(tree.control::<crate::gui::controls::BitmapView>(id).unwrap().bitmap, "GFX_GUI_ICON_PROFESSION_6");
    }

    /// Outside a raid no team is shown, in a team or not; in a raid the own team stands at its place and the
    /// others at theirs, the one leader marked. The buttons: the raid is asked for only by a team's leader, the
    /// locks by anyone.
    #[test]
    fn the_teams_show_only_in_a_raid() {
        use crate::team::{Mate, Team};
        let mate = |id: u32, name: &str| Mate { id, name: name.to_string(), level: 10, profession: 4, ..Mate::default() };
        let mut team = Team { own: 1, ..Team::default() };
        assert!(teams(&team).iter().all(Vec::is_empty));
        assert_eq!((button("CreateRaid", &team), button("ListLocks", &team), button("t1p1_Line", &team)), (None, Some(6), None));
        team.members = vec![mate(1, "Own"), mate(2, "Mate")];
        assert!(teams(&team).iter().all(Vec::is_empty));
        // (a member who was never named the leader asks for nothing; the leader does)
        assert_eq!(button("CreateRaid", &team), None);
        team.leader = Some(2);
        assert_eq!(button("CreateRaid", &team), None);
        team.leader = Some(1);
        assert_eq!(button("CreateRaid", &team), Some(1));
        team.set_place(Some(2));
        team.others[0] = vec![mate(7, "Other")];
        let shown = teams(&team);
        assert_eq!(shown[2].iter().map(|m| (m.0, m.4)).collect::<Vec<_>>(), [(1, true), (2, false)]);
        assert_eq!(shown[0].iter().map(|m| (m.1.as_str(), m.4)).collect::<Vec<_>>(), [("Other", false)]);
        assert!(shown[1].is_empty() && shown[5].is_empty());
    }

    #[test]
    fn professions_have_pictures() {
        assert_eq!(profession_picture(6), "GFX_GUI_ICON_PROFESSION_6");
        assert_eq!(profession_picture(0), "");
        assert_eq!(profession_picture(16), "");
    }
}
