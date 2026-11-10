//! The chat configuration window, GUI.dll's `GroupControlView_c` (FUN_100ad81c) in its "Chat Config" window
//! (`ChatConfig`, closed by Escape with `esc_chatconfig`).
//!
//! At the left, under "Chat Windows", the list of the chat windows (a `ListViewBase_c` in a scroll view in an inset
//! frame); the one selected is remembered (`selected_chat_window`). At the right the selected window's controls
//! (FUN_100a19ff), top to bottom:
//!
//! - "Chat Channels" over the tree of the chat groups in three folders, "Public Channels", "Private Channels" and
//!   "Combat Messages" (FUN_1008537d: a client group numbered under 0x4200001a is a combat message, a channel of kind
//!   14 private, the rest public), each open or not as the config has it. A group's row (FUN_100a0963) is its name, in
//!   0xff6666 when the group is not there this session, placed by name; it is selected, and shows
//!   `GFX_GUI_MENU_CHECK`, while the window shows the group. A press on it turns that on or off (FUN_100a18ae);
//! - "Hide channels subscribed to other windows": the groups another window shows and this one does not are left out;
//! - in a frame, "New Window" over "Rename Window" at the left and "Clone Window" over "Remove Window" at the right,
//!   then "Log messages" and "Show message log".
//!
//! The chat windows here are the tabs of the client's chat window (`chatgroups::Pane`). "Log messages" turns the
//! client's one chat log on and off (the original logs each window by itself); there is no log window to show. Not
//! here: a window row's own menu (its style), and disbanding one's own private group. The right button on a private
//! group's row opens its menu (FUN_100a1102): "Leave %s".

use crate::chatgroups::{self, Pane};
use crate::gui::host::Signal;
use crate::gui::treelist::{TreeItem, TreeList};
use crate::gui::view::{Id, Layout, Node, Tree};
use crate::theme::Palette;

const PUBLIC: i64 = -1;
const PRIVATE: i64 = -2;
const COMBAT: i64 = -3;
const FOLDERS: [i64; 3] = [PUBLIC, PRIVATE, COMBAT];

/// A text of the window (text.mdb 10001 by name), or this client's own words.
fn text(name: &str) -> String {
    let own = match name {
        "ChatConfig" => "Chat settings",
        "ChatWindows" => "Chat windows",
        "ChatChannels" => "Channels",
        "PublicChannels" => "Public channels",
        "PrivateChannels" => "Private channels",
        "CombatMessages" => "Fight messages",
        "HideChannelsInOtherWnd" => "Leave out channels other windows show",
        "NewWindow" => "New window",
        "RenameWindow" => "Rename window",
        "CloneWindow" => "Copy window",
        "RemoveWindow" => "Remove window",
        "LogMessages" => "Log messages",
        "ShowMessageLog" => "Show the message log",
        _ => name,
    };
    crate::gui::xml::ldb_named(10001, name, own)
}

/// The groups a window's tree lists: the client's own, the server's channels and those the window has met; with
/// `hide_others`, not those another window shows and this one does not (FUN_100a0963's first test).
pub fn listed(panes: &[Pane], selected: usize, hide_others: bool) -> Vec<String> {
    let Some(pane) = panes.get(selected) else { return Vec::new() };
    let mut groups: std::collections::BTreeSet<String> = chatgroups::BUILTIN.iter().map(|g| g.1.to_string()).collect();
    groups.extend(chatgroups::server_groups());
    groups.extend(pane.known.iter().cloned());
    let elsewhere = |g: &str| panes.iter().enumerate().any(|(k, p)| k != selected && p.selected.contains(g));
    groups.into_iter().filter(|g| !(hide_others && !pane.selected.contains(g) && elsewhere(g))).collect()
}

/// The tree's folders with the groups in them, each by name (no regard to case) and selected while the window shows it.
fn folders(panes: &[Pane], selected: usize, hide_others: bool, open: usize) -> (Vec<TreeItem>, Vec<String>) {
    let groups = listed(panes, selected, hide_others);
    let mut items = vec![
        TreeItem::folder(PUBLIC, &text("PublicChannels"), open & 1 != 0),
        TreeItem::folder(PRIVATE, &text("PrivateChannels"), open & 2 != 0),
        TreeItem::folder(COMBAT, &text("CombatMessages"), open & 4 != 0),
    ];
    let mut order: Vec<usize> = (0..groups.len()).collect();
    order.sort_by_key(|k| groups[*k].to_lowercase());
    for k in order {
        let group = &groups[k];
        let mut row = TreeItem::new(k as i64 + 1, group, Some("GFX_GUI_MENU_CHECK"), None);
        row.selected = panes.get(selected).is_some_and(|p| p.selected.contains(group));
        row.picture_hidden = !row.selected;
        // (white, or 0xff6666 for a group that is not there now)
        let colour = if chatgroups::available(group) { 0xffffff } else { 0xff6666 };
        row.colours = (colour, colour);
        let folder = if chatgroups::is_combat(group) {
            2
        } else if chatgroups::is_private(group) {
            1
        } else {
            0
        };
        items[folder].children.push(row);
    }
    (items, groups)
}

/// A list in a scroll view (the trees of both sides).
fn build(ctx: &egui::Context, select_on_click: bool) -> (Tree, Id, Id) {
    let mut tree = Tree::default();
    tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
    let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
    tree.get_mut(root).layout = Layout::Base;
    let scroll = tree.add(Node::new("", Box::new(crate::gui::scroll::ScrollView::new(0, 3))));
    tree.get_mut(scroll).layout = Layout::Scroll;
    let child = tree.add(Node::new("", Box::new(crate::gui::scroll::ScrollChild)));
    tree.get_mut(child).layout = Layout::ScrollChild;
    tree.get_mut(child).resize_mask = 0xf;
    let list = tree.add(Node::new("", Box::new(TreeList::new(select_on_click))));
    tree.add_child(child, list);
    tree.add_child(scroll, child);
    tree.add_child(root, scroll);
    (tree, root, list)
}

/// A list in the original's inset frame, `size` over all.
fn framed(ui: &mut egui::Ui, size: egui::Vec2, tree: &mut Tree, root: Id, palette: &Palette) -> egui::Response {
    let skin = crate::skin::skin(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let drawn = skin.as_deref().is_some_and(|s| s.nine_slice(ui.painter(), "GFX_GUI_INSET", rect, egui::Color32::WHITE, false));
    if !drawn {
        ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, crate::chrome::rgb(palette.accent)), egui::StrokeKind::Inside);
    }
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(5.0)));
    crate::gui::host::show(&mut inner, tree, root, skin.as_deref(), palette)
}

#[derive(Default)]
pub struct ChatConfig {
    pub open: bool,
    windows: Option<(Tree, Id, Id)>,
    groups: Option<(Tree, Id, Id)>,
    /// What the two lists were last built from.
    built: Option<(Vec<Pane>, usize, bool)>,
    /// The groups of the tree's rows, by their number less one.
    listed: Vec<String>,
    /// The window whose controls show, once read from the kept choice.
    selected: Option<usize>,
    hide_others: bool,
    /// The name being typed for the selected window.
    renaming: Option<String>,
    /// The private group whose menu is up, and the menu.
    menu: Option<String>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// The private groups the player chose to leave, for the chat window to send.
    pub leaves: Vec<String>,
}

impl ChatConfig {
    /// Shows the window; true when the chat windows were changed (to be kept). `tab` is the chat window's shown tab,
    /// `log` whether the chat is logged.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, panes: &mut Vec<Pane>, tab: &mut usize, log: &mut bool) -> bool {
        if !self.open || panes.is_empty() {
            return false;
        }
        let mut changed = false;
        let selected = self.selected.get_or_insert_with(|| crate::layout::tab("ChatConfig").unwrap_or(*tab)).to_owned().min(panes.len() - 1);
        self.selected = Some(selected);
        if self.windows.is_none() {
            self.windows = Some(build(ctx, false));
            self.groups = Some(build(ctx, false));
            self.built = None;
        }
        let (Some((wtree, wroot, wlist)), Some((gtree, groot, glist))) = (self.windows.as_mut(), self.groups.as_mut()) else { return false };
        let (wroot, wlist, groot, glist) = (*wroot, *wlist, *groot, *glist);
        // (the lists again when the windows, the selection or the hiding changed; the folders as they were left)
        let now = (panes.clone(), selected, self.hide_others);
        if self.built.as_ref() != Some(&now) {
            if let Some(t) = wtree.control_mut::<TreeList>(wlist) {
                t.items = panes
                    .iter()
                    .enumerate()
                    .map(|(k, p)| {
                        let mut row = TreeItem::new(k as i64 + 1, &p.name, None, None);
                        row.selected = k == selected;
                        row
                    })
                    .collect();
            }
            let open = match gtree.control::<TreeList>(glist).filter(|t| !t.items.is_empty()) {
                Some(t) => FOLDERS.iter().enumerate().filter(|(_, id)| t.items.iter().any(|i| i.id == **id && i.open)).map(|(k, _)| 1 << k).sum(),
                None => crate::layout::tab("ChatConfigFolders").unwrap_or(7),
            };
            let (items, listed) = folders(panes, selected, self.hide_others, open);
            if let Some(t) = gtree.control_mut::<TreeList>(glist) {
                t.items = items;
            }
            self.listed = listed;
            wtree.relayout = true;
            gtree.relayout = true;
            self.built = Some(now);
        }
        let mut open = self.open;
        let mut hide_others = self.hide_others;
        let (mut new, mut rename, mut clone, mut remove) = (false, false, false, false);
        let mut right_clicked = None;
        crate::chrome::window(ctx, palette, "ChatConfig", &text("ChatConfig"), &mut open, |w| w.default_pos(crate::layout::pos("ChatConfig", egui::pos2(260.0, 90.0))).resizable(false), |ui| {
            let white = egui::Color32::WHITE;
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(text("ChatWindows")).color(white));
                    framed(ui, egui::vec2(150.0, 330.0), wtree, wroot, palette);
                });
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(text("ChatChannels")).color(white));
                    let response = framed(ui, egui::vec2(260.0, 200.0), gtree, groot, palette);
                    // (the right button on a group's row)
                    if response.secondary_clicked()
                        && let Some(at) = response.interact_pointer_pos()
                    {
                        let r = crate::gui::host::screen_rect(gtree, glist, response.rect.min);
                        right_clicked = gtree.control::<TreeList>(glist).and_then(|t| t.id_at(at.y - r.min.y)).filter(|id| *id > 0).map(|id| (id as usize - 1, at));
                    }
                    crate::chrome::checkbox(ui, palette, &mut hide_others, &text("HideChannelsInOtherWnd"));
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            new = crate::chrome::button(ui, palette, &text("NewWindow"), true).clicked();
                            rename = crate::chrome::button(ui, palette, &text("RenameWindow"), true).clicked();
                        });
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            clone = crate::chrome::button(ui, palette, &text("CloneWindow"), true).clicked();
                            remove = crate::chrome::button(ui, palette, &text("RemoveWindow"), panes.len() > 1).clicked();
                        });
                    });
                    crate::chrome::checkbox(ui, palette, log, &text("LogMessages"));
                    ui.add_enabled_ui(false, |ui| {
                        let mut off = false;
                        crate::chrome::checkbox(ui, palette, &mut off, &text("ShowMessageLog"));
                    });
                });
            });
        });
        self.hide_others = hide_others;
        if let Some((k, at)) = right_clicked
            && let Some(group) = self.listed.get(k).filter(|g| chatgroups::is_private(g))
        {
            let label = crate::gui::xml::ldb_named(10001, "LeaveChatGrp", "Leave %s").replace("%s", group);
            self.popup = Some(crate::chrome::menu_at(ctx, at, &[(label, true)]));
            self.menu = Some(group.clone());
        }
        if crate::chrome::menu(ctx, palette, "chat-group-menu", &mut self.popup).is_some()
            && let Some(group) = self.menu.take()
        {
            self.leaves.push(group);
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        // (a press on a window selects it; on a group turns it on or off in the selected window; on a folder opens or
        // closes it, which is remembered)
        for (_, s) in std::mem::take(&mut wtree.signals) {
            if let Signal::ItemClicked(id, _) = s
                && id > 0
            {
                self.selected = Some(id as usize - 1);
                crate::layout::set_tab("ChatConfig", id as usize - 1);
            }
        }
        for (_, s) in std::mem::take(&mut gtree.signals) {
            let Signal::ItemClicked(id, _) = s else { continue };
            gtree.relayout = true;
            if FOLDERS.contains(&id) {
                if let Some(t) = gtree.control::<TreeList>(glist) {
                    let mask = FOLDERS.iter().enumerate().filter(|(_, f)| t.items.iter().any(|i| i.id == **f && i.open)).map(|(k, _)| 1 << k).sum();
                    crate::layout::set_tab("ChatConfigFolders", mask);
                }
            } else if let (Some(group), Some(pane)) = (self.listed.get(id as usize - 1), panes.get_mut(selected)) {
                pane.toggle(group);
                changed = true;
            }
        }
        // the buttons (the window menu's own entries do the same)
        // (a new window and a copy come up in frames of their own, as the original's)
        if new {
            let mut pane = chatgroups::defaults().remove(0);
            pane.name = chatgroups::free_name(panes);
            pane.frame = pane.name.clone();
            pane.selected.clear();
            pane.auto = false;
            panes.push(pane);
            self.selected = Some(panes.len() - 1);
            changed = true;
        }
        if clone && let Some(mut pane) = panes.get(selected).cloned() {
            pane.name = chatgroups::free_name(panes);
            pane.frame = pane.name.clone();
            panes.push(pane);
            self.selected = Some(panes.len() - 1);
            changed = true;
        }
        if remove && panes.len() > 1 {
            panes.remove(selected);
            self.selected = Some(selected.min(panes.len() - 1));
            *tab = (*tab).min(panes.len() - 1);
            changed = true;
        }
        if rename {
            self.renaming = panes.get(selected).map(|p| p.name.clone());
        }
        // (the name asked for in a small window of its own, as the original's text input dialog)
        if let Some(name) = self.renaming.as_mut() {
            let mut asking = true;
            let mut answer = None;
            crate::chrome::window(ctx, palette, "rename-chat-window", &text("RenameWindow"), &mut asking, |w| w.resizable(false).pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.content_rect().center()), |ui| {
                let field = crate::chrome::text_edit(ui, palette, egui::TextEdit::singleline(name).desired_width(180.0));
                if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    answer = Some(true);
                }
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, "OK", true).clicked() {
                        answer = Some(true);
                    }
                    if crate::chrome::button(ui, palette, "Cancel", true).clicked() {
                        answer = Some(false);
                    }
                });
            });
            if answer == Some(true) {
                let name = name.trim().to_string();
                if !name.is_empty()
                    && let Some(pane) = panes.get_mut(selected)
                {
                    pane.name = name;
                    changed = true;
                }
            }
            if answer.is_some() || !asking {
                self.renaming = None;
            }
        }
        if !open {
            self.renaming = None;
            self.built = None;
        }
        self.open = open;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_groups_go_into_their_folders() {
        let mut panes = chatgroups::defaults();
        panes[0].toggle("Zeta OOC");
        let (items, listed) = folders(&panes, 0, false, 0b101);
        assert!(items[0].open && !items[1].open && items[2].open);
        let names = |k: usize| items[k].children.iter().map(|c| c.label.clone()).collect::<Vec<_>>();
        // (the fight under Combat Messages; the loot and the research with the public ones, as the original's rule has it)
        assert!(names(2).contains(&"Me hit by monster".to_string()) && !names(2).contains(&"Research".to_string()));
        for group in ["System", "Vicinity", "Tell Messages", "Research", "Team Loot Messages", "Zeta OOC"] {
            assert!(names(0).contains(&group.to_string()), "{group}");
        }
        // (the private folder holds private groups only: other tests may have told of one)
        assert!(names(1).iter().all(|n| chatgroups::is_private(n)));
        // (by name; selected with its check while the window shows the group)
        let public = names(0);
        let mut sorted = public.clone();
        sorted.sort_by_key(|n| n.to_lowercase());
        assert_eq!(public, sorted);
        let row = |name: &str| items.iter().flat_map(|f| f.children.iter()).find(|c| c.label == name).unwrap();
        assert!(row("Vicinity").selected && !row("Vicinity").picture_hidden);
        assert!(!row("Me hit by monster").selected && row("Me hit by monster").picture_hidden);
        // (a row's number finds its group again)
        assert_eq!(listed[row("Zeta OOC").id as usize - 1], "Zeta OOC");
    }

    #[test]
    fn groups_of_other_windows_can_be_left_out() {
        let panes = chatgroups::defaults();
        // (the combat window shows "You hit other", the default window does not: hidden from the default window's tree
        // when asked; what it shows itself stays)
        assert!(listed(&panes, 0, false).contains(&"You hit other".to_string()));
        let hidden = listed(&panes, 0, true);
        assert!(!hidden.contains(&"You hit other".to_string()) && hidden.contains(&"Vicinity".to_string()) && hidden.contains(&"Other hit by other".to_string()));
    }
}
