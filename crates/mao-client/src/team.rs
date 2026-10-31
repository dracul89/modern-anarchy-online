//! The team: an invitation to answer, the members with their health and nano, and leaving. The protocol is the Arete Landing
//! capture's (2026-10-30, the original client): an invitation is the server's `CharacterAction` 0x1a naming who asks; the
//! answer is the client's 0x1c naming them with 1 (join) or 0 (no); the server then sends `TeamMember` (`46312d2e`: the
//! character, the team, a word, the level, the profession, the name) for each member and `TeamMemberInfo` (`28784248`:
//! health, max health, max nano, nano) as they change. Leaving is the client's 0x18; the server's 0x20 names who left (oneself:
//! the team is gone). aosharp's `Team` names the same actions.

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
}

impl Team {
    /// A member, added when new.
    pub fn member(&mut self, id: u32) -> &mut Mate {
        if let Some(at) = self.members.iter().position(|m| m.id == id) {
            return &mut self.members[at];
        }
        self.members.push(Mate { id, ..Mate::default() });
        self.members.last_mut().unwrap()
    }
}

/// What the window asks the connection to send.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Answer an invitation from this character.
    Reply(u32, bool),
    Leave,
}

static TEAM: Mutex<Team> = Mutex::new(Team { members: Vec::new(), invite: None });
static COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());

/// The team as the connection last heard it.
pub fn with<R>(f: impl FnOnce(&mut Team) -> R) -> R {
    f(&mut TEAM.lock().unwrap_or_else(|e| e.into_inner()))
}

pub fn take_commands() -> Vec<Command> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn send(c: Command) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(c);
}

/// The team window, GUI.dll's `TeamView_c` (FUN_10078c14, `team_view`, 1000 by 1000 at most): a header line (text.mdb
/// 10000's "Team level: %u  Team side: %s", or "No team"), a row of buttons (in a team: Leave, text.mdb's "Leave"; the
/// leader's invite button is left out: its join request was not captured) and six member views (FUN_1007981b): above
/// the first and under each a `GFX_GUI_FRIENDLIST_SPLITTER`, then the member's name in the middle (the leader's in
/// 0x11eeaa, others white, FUN_10077b3d) and its health and nano bars (`PowerbarView_c` of `GFX_GUI_BAR_HEALTH_TEAM` /
/// `_NANO_TEAM` over their `_BG`s). Places without a member are hidden (FUN_1007a154). An invitation asks "Want to join
/// %s's team?" (text.mdb 10000).
#[derive(Default)]
pub struct TeamWindow {
    pub open: bool,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id)>,
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
    format!(
        "<View view_layout=\"vertical\">\
           <BorderView layout_borders=\"Rect(4,2,4,2)\"><TextView name=\"header\" value=\"\" layout_borders=\"Rect(4,2,4,2)\"/></BorderView>\
           <View name=\"buttons\" view_layout=\"horizontal\" layout_borders=\"Rect(0,5,0,5)\">\
             <HLayoutSpacer/><Button name=\"leave\" label=\"{leave}\" layout_borders=\"Rect(5,0,5,0)\"/>\
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
            let mut answer = None;
            let mut dialog = true;
            crate::chrome::window(ctx, palette, "team-invite", &crate::gui::xml::ldb_text("#10000:3860335"), &mut dialog, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO), |ui| {
                ui.label(egui::RichText::new(&question).color(crate::chrome::rgb(palette.text)));
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, "Yes", true).clicked() {
                        answer = Some(true);
                    }
                    if crate::chrome::button(ui, palette, "No", true).clicked() {
                        answer = Some(false);
                    }
                });
            });
            if !dialog {
                answer = Some(false);
            }
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
            let level = team.members.iter().map(|m| m.level).max().unwrap_or(0);
            crate::gui::xml::ldb_text("#10000:56702133").replacen("%u", &level.to_string(), 1).replacen("%s", "", 1)
        };
        set_text(tree, root, "header", &header);
        show_view(tree, root, "buttons", !team.members.is_empty());
        for k in 0..6 {
            let mate = team.members.get(k);
            show_view(tree, root, &format!("mate{k}"), mate.is_some());
            let Some(m) = mate else { continue };
            // (the first member is the leader: the server lists the team from its leader)
            let colour = if k == 0 { "#11eeaa" } else { "#ffffff" };
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
        crate::chrome::window(ctx, palette, "Team", &title, &mut open, |w| w.default_pos(crate::layout::pos("Team", egui::pos2(20.0, 300.0))).default_size(min.max(egui::vec2(200.0, 0.0))).resizable(false), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        for (id, signal) in std::mem::take(&mut tree.signals) {
            if signal == crate::gui::host::Signal::Clicked && tree.get(id).name == "leave" {
                send(Command::Leave);
            }
        }
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_are_added_once() {
        let mut t = Team::default();
        t.member(5).name = "A".into();
        t.member(5).level = 7;
        t.member(6);
        assert_eq!(t.members.len(), 2);
        assert_eq!((t.members[0].name.as_str(), t.members[0].level), ("A", 7));
    }
}
