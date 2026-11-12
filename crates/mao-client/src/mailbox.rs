//! The mail windows, laid out as in the original (from screenshots): the inbox ("Messages", "Size: n/30",
//! a Subject / Sent list, New / Delete / Close) and the compose window ("Message": To, Subject, a body,
//! Credits, C.O.D., an item, Standard / Express, Total cost, Send / Close).
//!
//! The wire side is partly known (see `mao_net::n3::mail_request` / `mail_send`, both byte-exact against the
//! user's capture): the inbox request and the send. What an inbox *contains* is not: the only captured
//! reply is an empty list, so the list shows the count the server reports and no rows until a capture with
//! mail in it shows their layout. Sending needs a paying account (the server refuses a free one).

use std::sync::Arc;

use crate::gear::{GearItem, GearWindow, Icon};
use mao_net::mail::{self, Entry, Mail};
use crate::theme::Palette;

/// A mail terminal's serial (the identity is `(0xc773, serial)`) seen in a capture, with where the player stood.
/// The server does not announce mail terminals, so these come from captures: (playfield, x, z, serial).
pub const KNOWN_TERMINALS: &[(u32, f32, f32, u32)] = &[(540, 445.4, 456.1, 0xc003_021c), (540, 408.1, 407.3, 0xc002_021c), (655, 3237.6, 771.4, 0xc000_028f), (4682, 351.5, 378.7, 0xc000_124a)];

/// The terminal nearest to a place among `(x, z, serial)` entries (within 25 units).
pub fn nearest_in(terminals: &[(f32, f32, u32)], x: f32, z: f32) -> Option<u32> {
    terminals.iter().map(|t| (((t.0 - x).powi(2) + (t.1 - z).powi(2)).sqrt(), t.2)).filter(|(d, _)| *d < 25.0).min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, s)| s)
}

/// The mail terminals of a playfield: the ones built into it (its dynel table lists them with their serials: 385 of them in 203
/// playfields), else the ones seen in the captures.
pub fn terminals(db: &mao_formats::resource::ResourceDatabase, playfield: u32) -> Vec<(f32, f32, u32)> {
    let built_in: Vec<(f32, f32, u32)> = db
        .get(mao_formats::resource::Id { kind: mao_formats::dynels::KIND, instance: playfield })
        .ok()
        .flatten()
        .map(|table| mao_formats::dynels::parse(&table).into_iter().filter(|d| d.kind == 0xc773).map(|d| (d.position[0], d.position[2], d.instance)).collect())
        .unwrap_or_default();
    if built_in.is_empty() {
        KNOWN_TERMINALS.iter().filter(|t| t.0 == playfield).map(|t| (t.1, t.2, t.3)).collect()
    } else {
        built_in
    }
}

/// The mail terminal nearest to a place in a playfield (within 25 units).
pub fn nearest_terminal(db: &mao_formats::resource::ResourceDatabase, playfield: u32, x: f32, z: f32) -> Option<u32> {
    nearest_in(&terminals(db, playfield), x, z)
}

/// A unix time as boost's `posix_time::to_simple_string` writes it (`2026-Oct-30 21:05:09`, UTC), as the mail list shows it.
pub fn simple_time(unix: u32) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let t = mail::time_text(unix);
    let (date, clock) = t.split_once(' ').unwrap_or((&t, ""));
    let mut d = date.split('-');
    let (y, m, day) = (d.next().unwrap_or(""), d.next().and_then(|m| m.parse::<usize>().ok()).unwrap_or(1), d.next().unwrap_or(""));
    format!("{y}-{}-{day} {clock}:{:02}", MONTHS[(m - 1).min(11)], unix % 60)
}

/// What the windows ask the network thread to do.
#[derive(Clone, Debug, PartialEq)]
pub enum MailCommand {
    /// Use the nearest known mail terminal and ask for the inbox.
    Inbox,
    /// Read a mail (by its id).
    Open(u32),
    /// Take a mail's item and credits.
    Take(u32),
    Delete(u32),
    Send { recipient: String, subject: String, body: String, item: (u32, u32), credits: u32, express: bool },
}

/// A mail that was opened, with its attached item's name and icon from the game's database.
pub struct Opened {
    pub mail: Mail,
    pub item: Option<(String, Option<Arc<Icon>>)>,
}

pub struct MailWindow {
    /// The inbox window.
    pub open: bool,
    /// Messages in the inbox, as the server last reported (None before it answered).
    pub count: Option<usize>,
    pub entries: Vec<Entry>,
    pub selected: Option<u32>,
    pub opened: Option<Opened>,
    pub status: String,
    /// The message window (a new mail, or the one opened): `mailmessage.rs`.
    pub message: crate::mailmessage::MessageWindow,
    /// The inbox on the skin's view, its list, what it last showed, and a mail waiting for "delete?".
    window: Option<crate::gui::window::XmlWindow>,
    list: Option<crate::gui::view::Id>,
    shown: Option<(Vec<Entry>, Option<usize>)>,
    deleting: Option<u32>,
    /// Lines for the chat window: what the game says of a mail (a delete it refuses).
    pub said: Vec<String>,
}

impl Default for MailWindow {
    fn default() -> MailWindow {
        MailWindow { open: false, count: None, entries: Vec::new(), selected: None, opened: None, status: String::new(), message: Default::default(), window: None, list: None, shown: None, said: Vec::new(), deleting: None }
    }
}

impl MailWindow {
    /// A sample for a headless preview.
    pub fn sample() -> MailWindow {
        let mut w = MailWindow { open: true, count: Some(0), ..Default::default() };
        w.message.compose("muggles", "hi");
        w
    }

    /// A new inbox from the server.
    pub fn set_entries(&mut self, entries: Vec<Entry>) {
        self.count = Some(entries.len());
        self.status = format!("The inbox holds {} messages.", entries.len());
        if self.selected.is_some_and(|id| !entries.iter().any(|e| e.id == id)) {
            self.selected = None;
            self.opened = None;
        }
        self.entries = entries;
    }

    /// A mail's delete, the question answered yes (FUN_101096f1, which the inbox's Delete and the message window's both
    /// come to): a mail with an item that was not taken is not deleted, and the game says why
    /// (`Feedback_MailDeleteAttachments`; the server would not delete it either, and says nothing). Else the server is
    /// told, the row goes at once and the message window showing that mail closes.
    fn delete(&mut self, id: u32) -> Option<MailCommand> {
        let flags = self.entries.iter().find(|e| e.id == id).map(|e| e.flags).or_else(|| self.opened.as_ref().filter(|o| o.mail.id == id).map(|o| o.mail.flags)).unwrap_or(0);
        if !deletable(flags) {
            let text = crate::feedback::plain("Feedback_MailDeleteAttachments");
            let text = if text.is_empty() { "Take what is attached to the message before deleting it.".to_string() } else { text };
            self.status = text.clone();
            self.said.push(text);
            return None;
        }
        self.entries.retain(|e| e.id != id);
        self.count = Some(self.entries.len());
        if self.selected == Some(id) {
            self.selected = None;
        }
        if self.opened.as_ref().is_some_and(|o| o.mail.id == id) {
            self.opened = None;
        }
        self.message.close_mail(id);
        Some(MailCommand::Delete(id))
    }

    /// A mail's flags changed (read, taken ...).
    pub fn set_flags(&mut self, id: u32, flags: u32) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.flags = flags;
        }
        if let Some(o) = self.opened.as_mut().filter(|o| o.mail.id == id) {
            o.mail.flags = flags;
        }
    }

    /// `MailWindow_c` (FUN_10109a11) on `Views/MailWindow.xml`: the list in `InboxBorder` (a list-mode `MultiListView_c` of
    /// Icon 16 (sortable), From, Subject, Sent and Expires 96/164/96/96 (resizable, movable, sortable), 5 in from the
    /// border), `InboxSize` "n/30" (FUN_10109489) with text.mdb's "Inbox size" tip, New, Delete and Close; the inbox is
    /// asked for as it opens (`N3Msg_RequestMailMessage(0)`).
    fn load(&mut self, ctx: &egui::Context) -> bool {
        let Some(mut w) = crate::gui::window::XmlWindow::load(ctx, "Mail", "Mail", "MailWindow.xml") else { return false };
        w.tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        w.default_size = egui::vec2(550.0, 300.0);
        let root = w.root;
        if let Some(border) = w.tree.find(root, "InboxBorder") {
            use crate::gui::multilist::{self, MOVABLE, RESIZABLE, SORTABLE};
            let list = multilist::create(&mut w.tree, "Inbox");
            for (id, label, width, flags) in [(0, "Icon", 16.0, SORTABLE), (1, "From", 96.0, RESIZABLE | MOVABLE | SORTABLE), (2, "Subject", 164.0, RESIZABLE | MOVABLE | SORTABLE), (3, "Sent", 96.0, RESIZABLE | MOVABLE | SORTABLE), (4, "Expires", 96.0, RESIZABLE | MOVABLE | SORTABLE)] {
                multilist::add_column(&mut w.tree, list, id, label, width, flags);
            }
            w.tree.get_mut(list).borders = crate::gui::geom::Rect::new(5.0, 5.0, 5.0, 5.0);
            // (the border's `LayoutNode`: the list over all of it)
            w.tree.get_mut(border).layout = crate::gui::view::Layout::Base;
            w.tree.add_child(border, list);
            self.list = Some(list);
        }
        if let Some(id) = w.tree.find(root, "InboxSize") {
            w.tree.get_mut(id).tooltip = crate::gui::xml::ldb_text("#Inbox size");
        }
        self.window = Some(w);
        self.shown = None;
        true
    }

    /// The rows for the inbox (`MailListItem_c` FUN_10104395): the read or unread picture (0x1d7 + read), the sender, the
    /// subject, and the times as boost's `to_simple_string` writes them; the icon column sorts unread first
    /// (FUN_1010422b), the others by their text.
    fn rows(entries: &[Entry]) -> Vec<crate::gui::multilist::Item> {
        use crate::gui::multilist::{CellView, Item};
        entries
            .iter()
            .map(|e| {
                let read = e.flags & mail::READ != 0;
                let (sent, expires) = (simple_time(e.sent), simple_time(e.expires));
                Item {
                    id: e.id as i64,
                    cells: vec![
                        (0, CellView::Picture(if read { "GFX_GUI_MAIL_READ" } else { "GFX_GUI_MAIL_UNREAD" }.into())),
                        (1, CellView::Text(e.sender.clone())),
                        (2, CellView::Text(e.subject.clone())),
                        (3, CellView::Text(sent.clone())),
                        (4, CellView::Text(expires.clone())),
                    ],
                    keys: vec![(0, if read { "1" } else { "0" }.into()), (1, e.sender.clone()), (2, e.subject.clone()), (3, sent), (4, expires)],
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Draws the windows; returns what to send, if anything.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, items: &[GearItem], palette: &Palette) -> Option<MailCommand> {
        let mut command = None;
        if !self.open {
            self.window = None;
        } else if self.window.is_none() && self.load(ctx) {
            command = Some(MailCommand::Inbox);
        }
        if let (Some(w), Some(list)) = (self.window.as_mut(), self.list) {
            let root = w.root;
            // (the rows follow the inbox; the size "n/30")
            let shown = (self.entries.clone(), self.count);
            if self.shown.as_ref() != Some(&shown) {
                let rows = Self::rows(&self.entries);
                crate::gui::multilist::set_items(&mut w.tree, list, rows);
                crate::gui::multilist::select(&mut w.tree, list, self.selected.map(i64::from));
                if let Some(id) = w.tree.find(root, "InboxSize")
                    && let Some(t) = w.tree.control_mut::<crate::gui::controls::TextView>(id)
                {
                    t.set_text(&format!("{}/30", self.count.unwrap_or(self.entries.len())));
                }
                w.tree.relayout = true;
                self.shown = Some(shown);
            }
            let skin = crate::skin::skin(ctx);
            let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
            let (tree, status) = (&mut w.tree, &self.status);
            let mut is_open = true;
            crate::chrome::window(ctx, palette, "Mail", "Mail", &mut is_open, |win| win.default_size(min.max(egui::vec2(550.0, 300.0))).resizable(true).default_pos(crate::layout::pos("Mail", egui::pos2(520.0, 90.0))), |ui| {
                // (the views over the window less the room of the status line under them)
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    if !status.is_empty() {
                        ui.label(egui::RichText::new(status).small());
                    }
                    crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
                });
            });
            // (a row clicked: selected; twice, opened in the message window: FUN_10109548)
            for (_, id, clicks) in crate::gui::multilist::take_signals(&mut w.tree) {
                crate::gui::multilist::select(&mut w.tree, list, Some(id));
                self.selected = Some(id as u32);
                if clicks > 1 {
                    self.message.read(id as u32);
                    command = Some(MailCommand::Open(id as u32));
                }
            }
            for (id, signal) in std::mem::take(&mut w.tree.signals) {
                if signal != crate::gui::host::Signal::Clicked {
                    continue;
                }
                match w.tree.get(id).name.as_str() {
                    "NewButton" => self.message.compose("", ""),
                    // (FUN_101098a0: "Are you sure you want to delete this message?")
                    "DeleteButton" => self.deleting = crate::gui::multilist::selected(&w.tree, list).map(|m| m as u32),
                    "CloseButton" => is_open = false,
                    _ => {}
                }
            }
            self.open = is_open;
        }
        if let Some(m) = self.deleting {
            let question = crate::gui::xml::ldb_text("#1000:208922380");
            let answer = crate::chrome::question(ctx, palette, "delete-mail", &crate::gui::xml::ldb_text("#10000:120699445"), &question);
            if answer == Some(true) {
                command = self.delete(m).or(command);
            }
            if answer.is_some() {
                self.deleting = None;
            }
        }
        // The message window (a new mail, or the one opened).
        if let Some(c) = self.message.show(ctx, gear, items, self.opened.as_ref(), palette) {
            if matches!(c, MailCommand::Send { .. }) {
                self.status = "Sending ...".into();
            }
            match c {
                // (its Delete asks the same question first: FUN_1010686c, `ConfirmDeleteMail`)
                MailCommand::Delete(m) => self.deleting = Some(m),
                c => command = Some(c),
            }
        }
        command
    }
}

/// Whether a mail with these flags is deleted when asked (FUN_101096f1): one with an item is only once the item was
/// taken; credits alone do not hold it.
pub fn deletable(flags: u32) -> bool {
    flags & mao_net::mail::HAS_ITEM == 0 || flags & mao_net::mail::TAKEN != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mail_with_its_item_still_in_it_is_not_deleted() {
        use mao_net::mail::{HAS_CREDITS, HAS_ITEM, READ, TAKEN};
        assert!(deletable(0) && deletable(READ) && deletable(HAS_CREDITS | READ));
        assert!(!deletable(HAS_ITEM) && !deletable(HAS_ITEM | HAS_CREDITS | READ));
        assert!(deletable(HAS_ITEM | TAKEN | READ));
        let entry = |id: u32, flags: u32| Entry { id, sender_id: 1, sender: "Shop".into(), subject: "s".into(), sent: 0, expires: 0, flags, express: false };
        let mut w = MailWindow::default();
        w.set_entries(vec![entry(7, HAS_ITEM | READ), entry(8, READ)]);
        // (the one with its item: nothing sent, the game's line said, the row stays)
        assert_eq!((w.delete(7), w.entries.len(), w.said.len()), (None, 2, 1));
        // (taken, it goes: the server told, the row gone at once)
        w.set_flags(7, HAS_ITEM | READ | TAKEN);
        assert_eq!((w.delete(7), w.delete(8), w.entries.len(), w.count), (Some(MailCommand::Delete(7)), Some(MailCommand::Delete(8)), 0, Some(0)));
    }

    #[test]
    fn the_nearest_known_terminal() {
        let old_athen: Vec<(f32, f32, u32)> = KNOWN_TERMINALS.iter().filter(|t| t.0 == 540).map(|t| (t.1, t.2, t.3)).collect();
        assert_eq!(nearest_in(&old_athen, 446.0, 455.0), Some(0xc003_021c));
        assert_eq!(nearest_in(&old_athen, 410.0, 405.0), Some(0xc002_021c));
        assert_eq!(nearest_in(&old_athen, 300.0, 300.0), None);
    }

    #[test]
    fn boost_simple_time() {
        assert_eq!(simple_time(0), "1970-Jan-01 00:00:00");
        assert_eq!(simple_time(1_793_394_309), "2026-Oct-30 21:05:09");
    }
}
