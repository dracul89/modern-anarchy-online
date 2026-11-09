//! The raid window, GUI.dll's `RaidViewModule_c` (`raid_window`, the control center's Team menu "#Raid"): the skin's
//! `Views/Raid.xml`, six teams of six lines (a number, the profession's picture `t<n>p<m>_Prof`, the level `t<n>p<m>_Level`
//! and the name `t<n>p<m>_Name`), with "Convert To Raid" and "Raid Locks" above them.
//!
//! No capture has a raid, so the window shows what the client knows: the character's own team as Team 1 (its members in
//! their order, the leader's name in the team window's leader colour), the other teams empty, and the two buttons send
//! nothing yet (the raid's messages are unknown; `RaidBar.xml`, a member's bars with the profession's icon, is the raid
//! bars' view and is not drawn either).

use crate::theme::Palette;

/// A member as the window last showed it: id, name, level, profession, leader.
type Shown = Vec<(u32, String, i64, i64, bool)>;

#[derive(Default)]
pub struct RaidWindow {
    pub open: bool,
    window: Option<crate::gui::window::XmlWindow>,
    shown: Option<Shown>,
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
        let members: Shown = team.members.iter().map(|m| (m.id, m.name.clone(), m.level, m.profession, team.leader == Some(m.id))).collect();
        if self.shown.as_ref() != Some(&members) {
            let root = w.root;
            fill_team(&mut w.tree, root, 1, &members);
            for t in 2..=6 {
                fill_team(&mut w.tree, root, t, &[]);
            }
            w.tree.relayout = true;
            self.shown = Some(members);
        }
        let signals = w.show(ctx, palette);
        for (id, signal) in signals {
            if signal != crate::gui::host::Signal::Clicked {
                continue;
            }
            // ("Convert To Raid" / "Raid Locks": the raid's messages are not known; nothing is sent)
            let _ = w.tree.get(id).name.as_str();
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

    #[test]
    fn professions_have_pictures() {
        assert_eq!(profession_picture(6), "GFX_GUI_ICON_PROFESSION_6");
        assert_eq!(profession_picture(0), "");
        assert_eq!(profession_picture(16), "");
    }
}
