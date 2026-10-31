//! The friends window (`L`), GUI.dll's `FriendListView_c` (FUN_100aa315): a `ListViewBase_c` tree (`gui::treelist`) in a
//! scroll view, 169 by 215 to start, of four folders, each open unless `FriendsWindowConfig` says not: text.mdb 10001's
//! "Chat Windows", "Online Friends", "Offline Friends" and "Recent Messages". A friend (FUN_100a87fe) goes into the
//! online or offline folder by name (no regard to case) with `GFX_GUI_FRIENDLIST_STATUS_GREEN` online or `_RED` offline.
//! A double click on a friend starts a tell (FUN_100aa208 -> FUN_100a99cb); the client's own right-click menu tells or
//! takes the friend off the list (asking first: text.mdb 10001's "Are you sure you want to remove user ..."). Clicks do
//! not select (`+0x155` cleared). Adding is `/befriend <name>`, as the original's chat command.
//!
//! The chat windows folder lists no windows yet (the client's chat has one), and recent messages are not kept.

use crate::chatwindow::Out;
use crate::gui::host::Signal;
use crate::gui::treelist::{TreeItem, TreeList};
use crate::gui::view::{Id, Layout, Node, Tree};
use crate::theme::Palette;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Friend {
    pub id: u32,
    pub name: String,
    pub online: bool,
}

const CHAT_WINDOWS: i64 = -1;
const ONLINE: i64 = -2;
const OFFLINE: i64 = -3;
const RECENT: i64 = -4;

#[derive(Default)]
pub struct FriendsWindow {
    pub open: bool,
    views: Option<(Tree, Id, Id)>,
    shown: Vec<Friend>,
    /// The friend whose menu is open (and where), and the one to be removed once confirmed.
    menu: Option<(u32, egui::Pos2)>,
    removing: Option<u32>,
}

/// The order of a folder: by name, no regard to case (FUN_100a87fe's `CompareNoCase` insert).
pub fn sorted(friends: &[Friend]) -> Vec<&Friend> {
    let mut list: Vec<&Friend> = friends.iter().collect();
    list.sort_by(|a, b| b.online.cmp(&a.online).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    list
}

/// The folders with the friends in them.
fn folders(friends: &[Friend], open: &[(i64, bool)]) -> Vec<TreeItem> {
    let is_open = |id: i64| open.iter().find(|o| o.0 == id).is_none_or(|o| o.1);
    let mut items = vec![
        TreeItem::folder(CHAT_WINDOWS, &crate::gui::xml::ldb_text("#10001:231562867"), is_open(CHAT_WINDOWS)),
        TreeItem::folder(ONLINE, &crate::gui::xml::ldb_text("#10001:199016563"), is_open(ONLINE)),
        TreeItem::folder(OFFLINE, &crate::gui::xml::ldb_text("#10001:22857075"), is_open(OFFLINE)),
        TreeItem::folder(RECENT, &crate::gui::xml::ldb_text("#10001:120922995"), is_open(RECENT)),
    ];
    for f in sorted(friends) {
        let picture = if f.online { "GFX_GUI_FRIENDLIST_STATUS_GREEN" } else { "GFX_GUI_FRIENDLIST_STATUS_RED" };
        items[if f.online { 1 } else { 2 }].children.push(TreeItem::new(f.id as i64, &f.name, Some(picture), None));
    }
    items
}

/// Markup as plain text (the question's `<font>` tags).
fn plain(markup: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in markup.chars() {
        match c {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out
}

impl FriendsWindow {
    fn build(ctx: &egui::Context) -> (Tree, Id, Id) {
        let mut tree = Tree::default();
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
        tree.get_mut(root).layout = Layout::Base;
        let scroll = tree.add(Node::new("", Box::new(crate::gui::scroll::ScrollView::new(2, 2))));
        tree.get_mut(scroll).layout = Layout::Scroll;
        let child = tree.add(Node::new("", Box::new(crate::gui::scroll::ScrollChild)));
        tree.get_mut(child).layout = Layout::ScrollChild;
        tree.get_mut(child).resize_mask = 0xf;
        let list = tree.add(Node::new("", Box::new(TreeList::new(false))));
        tree.add_child(child, list);
        tree.add_child(scroll, child);
        tree.add_child(root, scroll);
        (tree, root, list)
    }

    /// Shows the window; returns what to send to the chat server and the name to start a tell to, if one was chosen.
    pub fn show(&mut self, ctx: &egui::Context, friends: &[Friend], palette: &Palette) -> (Vec<Out>, Option<String>) {
        let (mut outs, mut tell) = (Vec::new(), None);
        if self.views.is_none() {
            self.views = Some(Self::build(ctx));
            self.shown.clear();
        }
        let Some((tree, root, list)) = self.views.as_mut() else { return (outs, tell) };
        let (root, list) = (*root, *list);
        // (the folders keep whether they are open when the list changes)
        if self.shown != friends {
            let open: Vec<(i64, bool)> = tree.control::<TreeList>(list).map(|t| t.items.iter().map(|i| (i.id, i.open)).collect()).unwrap_or_default();
            if let Some(t) = tree.control_mut::<TreeList>(list) {
                t.items = folders(friends, &open);
            }
            tree.relayout = true;
            self.shown = friends.to_vec();
        }
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut right_clicked = None;
        crate::chrome::window(ctx, palette, "Friends", "Friends", &mut open, |w| w.default_pos(crate::layout::pos("Friends", egui::pos2(300.0, 120.0))).default_size(egui::vec2(169.0, 215.0)).resizable(true), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            // (a right click on a friend opens the client's menu)
            if response.secondary_clicked()
                && let Some(at) = response.interact_pointer_pos()
            {
                let r = crate::gui::host::screen_rect(tree, list, response.rect.min);
                if let Some(id) = tree.control::<TreeList>(list).and_then(|t| t.id_at(at.y - r.min.y))
                    && id > 0
                {
                    right_clicked = Some((id as u32, at));
                }
            }
        });
        if right_clicked.is_some() {
            self.menu = right_clicked;
        }
        // (a folder opened or closed lays the list out again; FUN_100aa208: a double click on a friend starts a tell)
        for (_, s) in std::mem::take(&mut tree.signals) {
            if let Signal::ItemClicked(id, clicks) = s {
                tree.relayout = true;
                if clicks > 1
                    && let Some(f) = friends.iter().find(|f| f.id as i64 == id)
                {
                    tell = Some(f.name.clone());
                }
            }
        }
        if let Some((id, at)) = self.menu
            && let Some(f) = friends.iter().find(|f| f.id == id)
        {
            let mut keep = true;
            let shown = egui::Area::new(egui::Id::unique("friend-menu")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(egui::RichText::new(&f.name).strong());
                    if ui.button("Tell").clicked() {
                        tell = Some(f.name.clone());
                        keep = false;
                    }
                    if ui.button("Remove").clicked() {
                        self.removing = Some(id);
                        keep = false;
                    }
                });
            });
            let elsewhere = ctx.input(|i| i.pointer.any_pressed()) && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| !shown.response.rect.contains(p));
            if !keep || elsewhere {
                self.menu = None;
            }
        } else {
            self.menu = None;
        }
        if let Some(id) = self.removing
            && let Some(f) = friends.iter().find(|f| f.id == id)
        {
            let question = plain(&crate::gui::xml::ldb_text("#10001:146099907")).replace("%s", &f.name);
            let mut answer = None;
            let mut dialog = true;
            crate::chrome::window(ctx, palette, "remove-friend", "Friends", &mut dialog, |w| w.resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO), |ui| {
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
            if answer == Some(true) {
                outs.push(Out::Unfriend { id });
            }
            if answer.is_some() || !dialog {
                self.removing = None;
            }
        }
        self.open = open;
        (outs, tell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friends_go_into_their_folders_by_name() {
        let friends = vec![
            Friend { id: 1, name: "zed".into(), online: true },
            Friend { id: 2, name: "Abe".into(), online: false },
            Friend { id: 3, name: "bob".into(), online: true },
            Friend { id: 4, name: "Cy".into(), online: false },
        ];
        let items = folders(&friends, &[(OFFLINE, false)]);
        let names = |k: usize| items[k].children.iter().map(|c| c.label.clone()).collect::<Vec<_>>();
        assert_eq!(names(1), ["bob", "zed"]);
        assert_eq!(names(2), ["Abe", "Cy"]);
        assert!(items[1].open && !items[2].open);
        assert_eq!(items[1].children[0].picture.as_deref(), Some("GFX_GUI_FRIENDLIST_STATUS_GREEN"));
        assert_eq!(plain("<font color=CCInfoText>Remove <font color=yellow>%s</font>?</font>"), "Remove %s?");
    }
}
