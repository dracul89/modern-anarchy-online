//! The friends window (`L`), GUI.dll's `FriendListView_c` (FUN_100aa315): a `ListViewBase_c` tree (`gui::treelist`) in a
//! scroll view, 169 by 215 to start, of four folders: text.mdb 10001's "Chat Windows", "Online Friends", "Offline
//! Friends" and "Recent Messages", each open unless `FriendsWindowConfig` says not (`is_chat_window_list_open`,
//! `is_online_list_open`, `is_offline_list_open`, `is_recent_list_open`, FUN_100a84c8; here a line of `tabs.ini`).
//!
//! A character (`FriendListNode_c`, FUN_100a87fe) is of a kind: 0 a friend online, 1 a friend offline, 2 one a tell was
//! sent to or came from who is no friend (FUN_100a6487, a temporary buddy on the chat server: the "Recent Messages"),
//! 3 one the window's config remembered a tell window of. It goes into its folder by name (no regard to case) with
//! `GFX_GUI_FRIENDLIST_STATUS_GREEN` online or `_RED` offline. Clicks do not select (`+0x155` cleared): a row is
//! selected while its window is open.
//!
//! A press on a row (FUN_100aa208) with the left button opens the character's tell window or closes it (FUN_100a7ca7):
//! this client has no tell windows and starts a tell in the chat line. The right button opens the row's menu
//! (FUN_100a99cb): "Befriend %s" (a recent one only, FUN_100a8530: a buddy add), "Invite %s" (FUN_100a856f: chat packet
//! 50, to the player's private chat group), "Delete %s" (FUN_100a93fd, asking first), "Ignore %s" or "Unignore %s"
//! (FUN_100a961f, asking before ignoring) and, while the mail window is up, "Mail %s" (FUN_100a989a). Adding by name is
//! `/befriend <name>`, as the original's chat command.
//!
//! The chat windows folder lists this client's one chat window: a press shows or hides it (FUN_1009b4ff). Its own menu
//! (FUN_100a8a69: visual settings, click-through, text input) is not here, nor the flashing of a row with unread tells.

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
    /// Not a friend: one of the recent messages (a temporary buddy).
    pub recent: bool,
}

const CHAT_WINDOWS: i64 = -1;
const ONLINE: i64 = -2;
const OFFLINE: i64 = -3;
const RECENT: i64 = -4;
/// The chat window's row.
const CHAT: i64 = -10;
/// The folders in the order of the saved mask's bits.
const FOLDERS: [i64; 4] = [CHAT_WINDOWS, ONLINE, OFFLINE, RECENT];

/// What the window needs to know of the others.
pub struct Around<'a> {
    pub ignored: &'a dyn Fn(&str) -> bool,
    /// Whether the mail window is up.
    pub mail: bool,
    pub chat_open: bool,
}

/// What the window asks of the others.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    Tell(String),
    /// Ignore the character, or stop ignoring it.
    Ignore(String),
    Mail(String),
    /// Show or hide the chat window.
    ChatWindow(bool),
}

/// A menu entry (FUN_100a99cb).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Befriend,
    Invite,
    Delete,
    Ignore,
    Mail,
}

/// The entries of a character's menu, in the original's order.
fn entries(recent: bool, mail: bool) -> Vec<Entry> {
    let mut list = Vec::new();
    if recent {
        list.push(Entry::Befriend);
    }
    list.extend([Entry::Invite, Entry::Delete, Entry::Ignore]);
    if mail {
        list.push(Entry::Mail);
    }
    list
}

/// A text of text.mdb 10001 by name, with `%s` the character's name.
fn text(name: &str, who: &str) -> String {
    let own = match name {
        "BefriendX" => "Add %s as a friend",
        "InviteX" => "Invite %s",
        "DeleteX" => "Remove %s",
        "IgnoreX" => "Ignore %s",
        "UnignoreX" => "Stop ignoring %s",
        "MailX" => "Mail %s",
        "ReallyWant2IgnoreX" => "Ignore what <font color=yellow>%s</font> says from now on?",
        "ReallyWantToRemoveXfromFriends" => "Take <font color=yellow>%s</font> off your friends?",
        _ => name,
    };
    crate::gui::xml::ldb_named(10001, name, own).replace("%s", who)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ask {
    Remove,
    Ignore,
}

#[derive(Default)]
pub struct FriendsWindow {
    pub open: bool,
    views: Option<(Tree, Id, Id)>,
    shown: Vec<Friend>,
    chat_shown: bool,
    /// The character whose menu is open with its entries, and the menu.
    menu: Option<(u32, Vec<Entry>)>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// The question being asked, and of whom.
    asking: Option<(Ask, u32)>,
    /// A preview's: the menu of the first recent message and the question before removing the first friend, both up.
    pub demo: bool,
}

/// The order of a folder: by name, no regard to case (FUN_100a87fe's `CompareNoCase` insert).
pub fn sorted(friends: &[Friend]) -> Vec<&Friend> {
    let mut list: Vec<&Friend> = friends.iter().collect();
    list.sort_by(|a, b| b.online.cmp(&a.online).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    list
}

/// The folders open in a saved mask (a bit a folder, in [`FOLDERS`]' order).
fn open_of(mask: usize) -> Vec<(i64, bool)> {
    FOLDERS.iter().enumerate().map(|(k, id)| (*id, mask & (1 << k) != 0)).collect()
}

/// The mask of the folders open now.
fn mask_of(items: &[TreeItem]) -> usize {
    FOLDERS.iter().enumerate().filter(|(_, id)| items.iter().any(|i| i.id == **id && i.open)).map(|(k, _)| 1 << k).sum()
}

/// The folders with the chat window and the characters in them.
fn folders(friends: &[Friend], open: &[(i64, bool)], chat_open: bool) -> Vec<TreeItem> {
    let is_open = |id: i64| open.iter().find(|o| o.0 == id).is_none_or(|o| o.1);
    let mut items = vec![
        TreeItem::folder(CHAT_WINDOWS, &crate::gui::xml::ldb_text("#10001:231562867"), is_open(CHAT_WINDOWS)),
        TreeItem::folder(ONLINE, &crate::gui::xml::ldb_text("#10001:199016563"), is_open(ONLINE)),
        TreeItem::folder(OFFLINE, &crate::gui::xml::ldb_text("#10001:22857075"), is_open(OFFLINE)),
        TreeItem::folder(RECENT, &crate::gui::xml::ldb_text("#10001:120922995"), is_open(RECENT)),
    ];
    let mut chat = TreeItem::new(CHAT, "Chat", None, None);
    chat.selected = chat_open;
    items[0].children.push(chat);
    for f in sorted(friends) {
        let picture = if f.online { "GFX_GUI_FRIENDLIST_STATUS_GREEN" } else { "GFX_GUI_FRIENDLIST_STATUS_RED" };
        let folder = match (f.recent, f.online) {
            (true, _) => 3,
            (false, true) => 1,
            (false, false) => 2,
        };
        items[folder].children.push(TreeItem::new(f.id as i64, &f.name, Some(picture), None));
    }
    items
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

    /// Shows the window; returns what to send to the chat server and what it asks of the other windows.
    pub fn show(&mut self, ctx: &egui::Context, friends: &[Friend], around: &Around, palette: &Palette) -> (Vec<Out>, Vec<Act>) {
        let (mut outs, mut acts) = (Vec::new(), Vec::new());
        let fresh = self.views.is_none();
        if fresh {
            self.views = Some(Self::build(ctx));
            self.shown.clear();
        }
        let Some((tree, root, list)) = self.views.as_mut() else { return (outs, acts) };
        let (root, list) = (*root, *list);
        // (the folders keep whether they are open when the list changes; at first as they were left, `tabs.ini`)
        if fresh || self.shown != friends || self.chat_shown != around.chat_open {
            let open: Vec<(i64, bool)> = if fresh {
                open_of(crate::layout::tab("FriendsWindowConfig").unwrap_or(15))
            } else {
                tree.control::<TreeList>(list).map(|t| t.items.iter().map(|i| (i.id, i.open)).collect()).unwrap_or_default()
            };
            if let Some(t) = tree.control_mut::<TreeList>(list) {
                t.items = folders(friends, &open, around.chat_open);
            }
            tree.relayout = true;
            self.shown = friends.to_vec();
            self.chat_shown = around.chat_open;
        }
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut right_clicked = None;
        crate::chrome::window(ctx, palette, "Friends", "Friends", &mut open, |w| w.default_pos(crate::layout::pos("Friends", egui::pos2(300.0, 120.0))).default_size(egui::vec2(169.0, 215.0)).resizable(true), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            // (the right button on a character: its menu)
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
        if std::mem::take(&mut self.demo) {
            right_clicked = friends.iter().find(|f| f.recent).map(|f| (f.id, egui::pos2(470.0, 150.0)));
            self.asking = friends.first().map(|f| (Ask::Remove, f.id));
        }
        if let Some((id, at)) = right_clicked
            && let Some(f) = friends.iter().find(|f| f.id == id)
        {
            let list = entries(f.recent, around.mail);
            let labels: Vec<(String, bool)> = list
                .iter()
                .map(|e| {
                    let name = match e {
                        Entry::Befriend => "BefriendX",
                        Entry::Invite => "InviteX",
                        Entry::Delete => "DeleteX",
                        Entry::Ignore if (around.ignored)(&f.name) => "UnignoreX",
                        Entry::Ignore => "IgnoreX",
                        Entry::Mail => "MailX",
                    };
                    (text(name, &f.name), true)
                })
                .collect();
            self.popup = Some(crate::chrome::menu_at(ctx, at, &labels));
            self.menu = Some((id, list));
        }
        // (a folder opened or closed lays the list out again and is remembered; FUN_100aa208: the left button on a
        // character starts a tell, on the chat window shows or hides it)
        for (_, s) in std::mem::take(&mut tree.signals) {
            if let Signal::ItemClicked(id, clicks) = s {
                tree.relayout = true;
                if FOLDERS.contains(&id) {
                    if let Some(t) = tree.control::<TreeList>(list) {
                        crate::layout::set_tab("FriendsWindowConfig", mask_of(&t.items));
                    }
                } else if id == CHAT {
                    acts.push(Act::ChatWindow(!around.chat_open));
                } else if clicks == 1
                    && let Some(f) = friends.iter().find(|f| f.id as i64 == id)
                {
                    acts.push(Act::Tell(f.name.clone()));
                }
            }
        }
        if let Some(k) = crate::chrome::menu(ctx, palette, "friend-menu", &mut self.popup)
            && let Some((id, list)) = self.menu.take()
            && let Some(f) = friends.iter().find(|f| f.id == id)
        {
            match list.get(k) {
                Some(Entry::Befriend) => outs.push(Out::Befriend { name: f.name.clone() }),
                Some(Entry::Invite) => outs.push(Out::PrivateInvite { id }),
                Some(Entry::Delete) => self.asking = Some((Ask::Remove, id)),
                // (FUN_100a961f: asked before ignoring, not before ignoring no more)
                Some(Entry::Ignore) if (around.ignored)(&f.name) => acts.push(Act::Ignore(f.name.clone())),
                Some(Entry::Ignore) => self.asking = Some((Ask::Ignore, id)),
                Some(Entry::Mail) => acts.push(Act::Mail(f.name.clone())),
                None => {}
            }
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        if let Some((ask, id)) = self.asking {
            match friends.iter().find(|f| f.id == id) {
                Some(f) => {
                    let title = crate::gui::xml::ldb_named(10001, "Warning", "Warning");
                    let (name, question) = match ask {
                        Ask::Remove => ("remove-friend", text("ReallyWantToRemoveXfromFriends", &f.name)),
                        Ask::Ignore => ("ignore-friend", text("ReallyWant2IgnoreX", &f.name)),
                    };
                    if let Some(yes) = crate::chrome::question(ctx, palette, name, &title, &question) {
                        if yes {
                            match ask {
                                Ask::Remove => outs.push(Out::Unfriend { id }),
                                Ask::Ignore => acts.push(Act::Ignore(f.name.clone())),
                            }
                        }
                        self.asking = None;
                    }
                }
                None => self.asking = None,
            }
        }
        self.open = open;
        (outs, acts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friends_go_into_their_folders_by_name() {
        let friend = |id: u32, name: &str, online: bool, recent: bool| Friend { id, name: name.into(), online, recent };
        let friends = vec![friend(1, "zed", true, false), friend(2, "Abe", false, false), friend(3, "bob", true, false), friend(4, "Cy", false, false), friend(5, "Dee", true, true)];
        let items = folders(&friends, &[(OFFLINE, false)], true);
        let names = |k: usize| items[k].children.iter().map(|c| c.label.clone()).collect::<Vec<_>>();
        assert_eq!(names(1), ["bob", "zed"]);
        assert_eq!(names(2), ["Abe", "Cy"]);
        // (one a tell came from who is no friend is a recent message, online or not)
        assert_eq!(names(3), ["Dee"]);
        assert!(items[1].open && !items[2].open);
        assert_eq!(items[1].children[0].picture.as_deref(), Some("GFX_GUI_FRIENDLIST_STATUS_GREEN"));
        // (the chat window's row is selected while the window shows)
        assert!(items[0].children[0].selected);
    }

    #[test]
    fn the_menu_is_the_originals() {
        assert_eq!(entries(false, false), [Entry::Invite, Entry::Delete, Entry::Ignore]);
        assert_eq!(entries(true, true), [Entry::Befriend, Entry::Invite, Entry::Delete, Entry::Ignore, Entry::Mail]);
    }

    #[test]
    fn the_open_folders_are_a_mask() {
        let open = open_of(0b1010);
        assert_eq!(open, [(CHAT_WINDOWS, false), (ONLINE, true), (OFFLINE, false), (RECENT, true)]);
        assert_eq!(mask_of(&folders(&[], &open, false)), 0b1010);
        assert_eq!(mask_of(&folders(&[], &[], false)), 15);
    }
}
