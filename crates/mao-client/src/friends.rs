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
//! A row flashes its picture while the character has tells not read (`FriendListNode_c` +0x34, `FlashIcon` in
//! FUN_100a87fe): one rises with a tell from or to them that came while their tell window was shut and no chat window
//! that is up showed it (FUN_100a7394), and opening the tell window reads them (FUN_100a7ca7). It is selected while
//! the tell window is open (FUN_100a74bc). A folder's label turns red (0xff6666, 0xc04c4c) when a row put into it has
//! unread tells (FUN_100a87fe), and is set again as a row is taken out (FUN_100a86f6): a row changing is taken out and
//! put back (FUN_100a8a4c), and the original's taking out sets every folder before the row's back to white
//! ([`recolour`]).
//!
//! A press on a row (FUN_100aa208) with the left button opens the character's tell window or closes it (FUN_100a7ca7).
//! The right button opens the row's menu
//! (FUN_100a99cb): "Befriend %s" (a recent one only, FUN_100a8530: a buddy add), "Invite %s" (FUN_100a856f: chat packet
//! 50, to the player's private chat group), "Delete %s" (FUN_100a93fd, asking first), "Ignore %s" or "Unignore %s"
//! (FUN_100a961f, asking before ignoring) and, while the mail window is up, "Mail %s" (FUN_100a989a). Adding by name is
//! `/befriend <name>`, as the original's chat command.
//!
//! The chat windows folder lists every chat window (FUN_100a9fff: by its name, no regard to case, with
//! `GFX_GUI_FRIENDLIST_STATUS_GREEN`, selected while it is up). The left button shows or hides it (FUN_1009b4ff; this
//! client shows its chat windows together), the right button opens its menu (FUN_100a8a69, `ChatWindow::window_menu`).

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
/// The first chat window's row; the others' go down from it.
const CHAT: i64 = -10;
/// The folders in the order of the saved mask's bits.
const FOLDERS: [i64; 4] = [CHAT_WINDOWS, ONLINE, OFFLINE, RECENT];

/// What the window needs to know of the others.
pub struct Around<'a> {
    pub ignored: &'a dyn Fn(&str) -> bool,
    /// Whether the mail window is up.
    pub mail: bool,
    pub chat_open: bool,
    /// How many of a character's tells are not read (`FriendListNode_c` +0x34).
    pub unread: &'a dyn Fn(&str) -> u32,
    /// Whether a character's tell window is open.
    pub tell_open: &'a dyn Fn(&str) -> bool,
    /// Whether a character's invitation to a private chat group waits for an answer (`FriendListNode_c` +0x39).
    pub invited: &'a dyn Fn(&str) -> bool,
    /// The chat windows' names, in their order.
    pub windows: Vec<String>,
}

/// What the window asks of the others.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// Open the character's tell window.
    Tell(String),
    /// Close the character's tell window.
    CloseTell(String),
    /// Ask the question of the character's invitation to a private chat group (FUN_100a76c9).
    Invitation(String),
    /// A chat window's menu (FUN_100a8a69), by its place among them, at a place on the screen.
    WindowMenu(usize, egui::Pos2),
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
    /// The rows, the chat windows and whether they are up, as the list shows them now.
    shown: Vec<Row>,
    shown_windows: Vec<String>,
    chat_shown: bool,
    /// Whether the Online, Offline and Recent folders' labels are red ([`recolour`]).
    red: [bool; 3],
    /// The frame the window was last drawn in: one not drawn the frame before is built anew, as the original makes
    /// its view when the window opens (FUN_100aa315).
    drawn: u64,
    /// The character whose menu is open with its entries, and the menu.
    menu: Option<(u32, Vec<Entry>)>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// The question being asked, and of whom.
    asking: Option<(Ask, u32)>,
    /// A preview's: the menu of the first recent message and the question before removing the first friend, both up.
    pub demo: bool,
}

/// A character's row as the list shows it (`FriendListNode_c`): its folder (0 Online, 1 Offline, 2 Recent, FUN_100a87fe
/// by the node's kind), whether it has tells not read, whether its invitation waits, whether its tell window is open.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    id: u32,
    name: String,
    online: bool,
    folder: usize,
    unread: bool,
    invited: bool,
    open: bool,
}

/// The rows of the characters, folder by folder, each folder by name, no regard to case (FUN_100a87fe's
/// `CompareNoCase` insert: before the first row whose name comes after).
fn rows_of(friends: &[Friend], around: &Around) -> Vec<Row> {
    let mut rows: Vec<Row> = friends
        .iter()
        .map(|f| Row {
            id: f.id,
            name: f.name.clone(),
            online: f.online,
            folder: if f.recent { 2 } else if f.online { 0 } else { 1 },
            unread: (around.unread)(&f.name) > 0,
            invited: (around.invited)(&f.name),
            open: (around.tell_open)(&f.name),
        })
        .collect();
    rows.sort_by(|a, b| a.folder.cmp(&b.folder).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    rows
}

/// FUN_100a86f6: a character's row taken out, with the label colours of the Online, Offline and Recent folders set
/// again on the way. The folders are walked in order up to the row's: one whose rows do not hold it is set white;
/// in the row's own, red when another of its rows has unread tells (the row after the one taken out is passed over,
/// as the original's index steps on after the delete), and the walk ends there. `folders` are the ids of the rows,
/// `unread` the characters with tells not read.
fn taken_out(folders: &mut [Vec<u32>; 3], id: u32, unread: &dyn Fn(u32) -> bool, red: &mut [bool; 3]) {
    let mut target = id;
    for (k, rows) in folders.iter_mut().enumerate() {
        if target == 0 {
            return;
        }
        let mut seen = false;
        let mut decided = false;
        let mut i = 0;
        while i < rows.len() {
            if rows[i] == target {
                rows.remove(i);
                target = 0;
                if seen {
                    decided = true;
                    break;
                }
            } else if !seen && unread(rows[i]) {
                seen = true;
                if target == 0 {
                    decided = true;
                    break;
                }
            }
            i += 1;
        }
        red[k] = decided || (target == 0 && seen);
    }
}

/// The folders' label colours after the rows went from `old` to `new` (both in the order shown), as the original
/// sets them event by event: a row gone is taken out (FUN_100a86f6), a new one put in (FUN_100a87fe: its folder turns
/// red when it has unread tells), one that changed (its tells, its invitation, its tell window, where it goes) is
/// taken out and put back (FUN_100a8a4c).
fn recolour(mut red: [bool; 3], old: &[Row], new: &[Row]) -> [bool; 3] {
    let unread = |id: u32| new.iter().any(|r| r.id == id && r.unread);
    let mut folders: [Vec<u32>; 3] = Default::default();
    for r in old {
        folders[r.folder].push(r.id);
    }
    let put_in = |folders: &mut [Vec<u32>; 3], row: &Row, red: &mut [bool; 3]| {
        let rows = &mut folders[row.folder];
        let name = row.name.to_lowercase();
        let at = rows.iter().position(|id| new.iter().find(|r| r.id == *id).is_some_and(|r| r.name.to_lowercase() > name)).unwrap_or(rows.len());
        rows.insert(at, row.id);
        if row.unread {
            red[row.folder] = true;
        }
    };
    for gone in old.iter().filter(|o| !new.iter().any(|n| n.id == o.id)) {
        taken_out(&mut folders, gone.id, &unread, &mut red);
    }
    for row in new {
        match old.iter().find(|o| o.id == row.id) {
            None => put_in(&mut folders, row, &mut red),
            Some(was) if was != row => {
                taken_out(&mut folders, row.id, &unread, &mut red);
                put_in(&mut folders, row, &mut red);
            }
            Some(_) => {}
        }
    }
    red
}

/// The folders open in a saved mask (a bit a folder, in [`FOLDERS`]' order).
fn open_of(mask: usize) -> Vec<(i64, bool)> {
    FOLDERS.iter().enumerate().map(|(k, id)| (*id, mask & (1 << k) != 0)).collect()
}

/// The mask of the folders open now.
fn mask_of(items: &[TreeItem]) -> usize {
    FOLDERS.iter().enumerate().filter(|(_, id)| items.iter().any(|i| i.id == **id && i.open)).map(|(k, _)| 1 << k).sum()
}

/// The chat windows' rows: their places among the windows, by name, no regard to case (FUN_100a9fff).
fn window_order(windows: &[String]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..windows.len()).collect();
    order.sort_by_key(|k| windows[*k].to_lowercase());
    order
}

/// The folders with the chat windows and the characters in them.
fn folders(rows: &[Row], windows: &[String], open: &[(i64, bool)], chat_open: bool, red: [bool; 3]) -> Vec<TreeItem> {
    let is_open = |id: i64| open.iter().find(|o| o.0 == id).is_none_or(|o| o.1);
    let mut items = vec![
        TreeItem::folder(CHAT_WINDOWS, &crate::gui::xml::ldb_text("#10001:231562867"), is_open(CHAT_WINDOWS)),
        TreeItem::folder(ONLINE, &crate::gui::xml::ldb_text("#10001:199016563"), is_open(ONLINE)),
        TreeItem::folder(OFFLINE, &crate::gui::xml::ldb_text("#10001:22857075"), is_open(OFFLINE)),
        TreeItem::folder(RECENT, &crate::gui::xml::ldb_text("#10001:120922995"), is_open(RECENT)),
    ];
    for k in window_order(windows) {
        let mut chat = TreeItem::new(CHAT - k as i64, &windows[k], Some("GFX_GUI_FRIENDLIST_STATUS_GREEN"), None);
        chat.selected = chat_open;
        items[0].children.push(chat);
    }
    // (the folders' label colours: red, or white as they are made)
    for (k, red) in red.iter().enumerate() {
        if *red {
            items[k + 1].colours = (0xff6666, 0xc04c4c);
        }
    }
    for r in rows {
        let picture = if r.online { "GFX_GUI_FRIENDLIST_STATUS_GREEN" } else { "GFX_GUI_FRIENDLIST_STATUS_RED" };
        let mut item = TreeItem::new(r.id as i64, &r.name, Some(picture), None);
        item.selected = r.open;
        item.flash = r.unread || r.invited;
        items[r.folder + 1].children.push(item);
    }
    items
}

/// What the left button on a character's row asks (FUN_100aa208 → FUN_100a7ca7 with the window's state turned over):
/// the invitation's question while one waits, else the tell window shut or opened.
fn press(name: &str, around: &Around) -> Act {
    if (around.invited)(name) {
        Act::Invitation(name.to_string())
    } else if (around.tell_open)(name) {
        Act::CloseTell(name.to_string())
    } else {
        Act::Tell(name.to_string())
    }
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
        }
        let Some((tree, root, list)) = self.views.as_mut() else { return (outs, acts) };
        let (root, list) = (*root, *list);
        // (the window opened anew: its rows put in one by one, FUN_100aa315 → FUN_100a87fe, so a folder is red when
        // one of its characters has tells not read)
        let frame = ctx.cumulative_frame_nr();
        let anew = fresh || self.drawn + 1 < frame;
        self.drawn = frame;
        if anew {
            self.shown.clear();
            self.red = [false; 3];
        }
        let rows = rows_of(friends, around);
        // (the folders keep whether they are open when the list changes; at first as they were left, `tabs.ini`)
        if fresh || anew || self.shown != rows || self.shown_windows != around.windows || self.chat_shown != around.chat_open {
            let open: Vec<(i64, bool)> = if fresh {
                open_of(crate::layout::tab("FriendsWindowConfig").unwrap_or(15))
            } else {
                tree.control::<TreeList>(list).map(|t| t.items.iter().map(|i| (i.id, i.open)).collect()).unwrap_or_default()
            };
            self.red = recolour(self.red, &self.shown, &rows);
            if let Some(t) = tree.control_mut::<TreeList>(list) {
                t.items = folders(&rows, &around.windows, &open, around.chat_open, self.red);
            }
            tree.relayout = true;
            self.shown = rows;
            self.shown_windows = around.windows.clone();
            self.chat_shown = around.chat_open;
        }
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut right_clicked = None;
        crate::chrome::window(ctx, palette, "Friends", "Friends", &mut open, |w| w.default_pos(crate::layout::pos("Friends", egui::pos2(300.0, 120.0))).default_size(egui::vec2(169.0, 215.0)).resizable(true), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            // (the right button on a character: its menu; on a chat window: the window's, FUN_100a8a69)
            if response.secondary_clicked()
                && let Some(at) = response.interact_pointer_pos()
            {
                let r = crate::gui::host::screen_rect(tree, list, response.rect.min);
                match tree.control::<TreeList>(list).and_then(|t| t.id_at(at.y - r.min.y)) {
                    Some(id) if id > 0 => right_clicked = Some((id as u32, at)),
                    Some(id) if id <= CHAT => acts.push(Act::WindowMenu((CHAT - id) as usize, at)),
                    _ => {}
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
        // character opens its tell window or closes it, FUN_100a7ca7, or asks its invitation's question when one
        // waits; on a chat window shows or hides it)
        for (_, s) in std::mem::take(&mut tree.signals) {
            if let Signal::ItemClicked(id, clicks) = s {
                tree.relayout = true;
                if FOLDERS.contains(&id) {
                    if let Some(t) = tree.control::<TreeList>(list) {
                        crate::layout::set_tab("FriendsWindowConfig", mask_of(&t.items));
                    }
                } else if id <= CHAT {
                    acts.push(Act::ChatWindow(!around.chat_open));
                } else if clicks == 1
                    && let Some(f) = friends.iter().find(|f| f.id as i64 == id)
                {
                    acts.push(press(&f.name, around));
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

    fn friend(id: u32, name: &str, online: bool, recent: bool) -> Friend {
        Friend { id, name: name.into(), online, recent }
    }

    /// What the others say of the characters: those with tells not read, those whose tell window is open, those whose
    /// invitation waits.
    fn around<'a>(unread: &'a dyn Fn(&str) -> u32, open: &'a dyn Fn(&str) -> bool, invited: &'a dyn Fn(&str) -> bool) -> Around<'a> {
        Around { ignored: &|_| false, mail: false, chat_open: true, unread, tell_open: open, invited, windows: vec!["Default Window".into(), "combat".into(), "Alpha".into()] }
    }

    #[test]
    fn friends_go_into_their_folders_by_name() {
        let friends = vec![friend(1, "zed", true, false), friend(2, "Abe", false, false), friend(3, "bob", true, false), friend(4, "Cy", false, false), friend(5, "Dee", true, true), friend(6, "cal", false, true)];
        let around = around(&|n| u32::from(n == "Cy"), &|n| n == "bob", &|n| n == "zed");
        let rows = rows_of(&friends, &around);
        let items = folders(&rows, &around.windows, &[(OFFLINE, false)], true, [false; 3]);
        let names = |k: usize| items[k].children.iter().map(|c| c.label.clone()).collect::<Vec<_>>();
        assert_eq!(names(1), ["bob", "zed"]);
        assert_eq!(names(2), ["Abe", "Cy"]);
        // (one a tell came from who is no friend is a recent message, online or not; by name only, FUN_100a87fe)
        assert_eq!(names(3), ["cal", "Dee"]);
        assert!(items[1].open && !items[2].open);
        assert_eq!(items[1].children[0].picture.as_deref(), Some("GFX_GUI_FRIENDLIST_STATUS_GREEN"));
        // (a row flashes with tells not read or an invitation waiting, and is selected while its tell window is open)
        assert_eq!(items[1].children.iter().map(|c| (c.flash, c.selected)).collect::<Vec<_>>(), [(false, true), (true, false)]);
        assert!(items[2].children[1].flash && !items[2].children[0].flash);
        // (the chat windows by name, no regard to case, selected while they are up; FUN_100a9fff)
        assert_eq!(names(0), ["Alpha", "combat", "Default Window"]);
        assert_eq!(items[0].children.iter().map(|c| c.id).collect::<Vec<_>>(), [CHAT - 2, CHAT - 1, CHAT]);
        assert!(items[0].children.iter().all(|c| c.selected));
    }

    /// When a folder's label turns red and back (FUN_100a87fe, FUN_100a86f6), with the original's own ways: taking a
    /// row out sets the folders before it white, and passes over the row after it.
    #[test]
    fn a_folder_turns_red_with_unread_tells() {
        let row = |id: u32, name: &str, folder: usize, unread: bool| Row { id, name: name.into(), online: folder == 0, folder, unread, invited: false, open: false };
        let quiet = vec![row(1, "Abe", 0, false), row(2, "Bob", 0, false), row(3, "Cy", 1, false), row(4, "Dee", 2, false)];
        // (a tell to Bob: Online turns red; the window opened anew finds it so too)
        let mut told = quiet.clone();
        told[1].unread = true;
        let red = recolour([false; 3], &quiet, &told);
        assert_eq!(red, [true, false, false]);
        assert_eq!(recolour([false; 3], &[], &told), red);
        // (then one to Cy, offline: taking Cy's row out walks Online first and sets it white, though Bob's still flash)
        let mut both = told.clone();
        both[2].unread = true;
        assert_eq!(recolour(red, &told, &both), [false, true, false]);
        // (Bob's tells read: his row taken out of Online, which has no others with tells; Offline is left as it was)
        let mut read = both.clone();
        read[1].unread = false;
        assert_eq!(recolour([false, true, false], &both, &read), [false, true, false]);
        // (taking a row out passes over the one after it: Abe's row goes, Bob's after it is not looked at)
        let mut folders: [Vec<u32>; 3] = [vec![1, 2], vec![3], vec![4]];
        let mut red = [true, true, true];
        taken_out(&mut folders, 1, &|id| id == 2, &mut red);
        assert_eq!((red, folders[0].clone()), ([false, true, true], vec![2]));
        // (with another row before it that has tells, the folder stays red)
        let mut folders: [Vec<u32>; 3] = [vec![1, 2, 5], vec![3], vec![4]];
        taken_out(&mut folders, 2, &|id| id == 1, &mut red);
        assert_eq!(red, [true, true, true]);
    }

    #[test]
    fn a_press_on_a_row_opens_or_shuts_the_tell_window() {
        let around = around(&|_| 0, &|n| n == "Bob", &|n| n == "Cy");
        assert_eq!(press("Abe", &around), Act::Tell("Abe".into()));
        assert_eq!(press("Bob", &around), Act::CloseTell("Bob".into()));
        // (an invitation that waits: its question, FUN_100a7ca7 → FUN_100a76c9)
        assert_eq!(press("Cy", &around), Act::Invitation("Cy".into()));
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
        assert_eq!(mask_of(&folders(&[], &[], &open, false, [false; 3])), 0b1010);
        assert_eq!(mask_of(&folders(&[], &[], &[], false, [false; 3])), 15);
    }
}
