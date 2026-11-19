//! The mail message window, GUI.dll's (FUN_10107fc8) on the skin's `Views/MailMessageWindow.xml`: one window for a new mail
//! and for one received.
//!
//! A new mail (no mail id): the `To` label and field (`FromToLabelSelector`/`FromToInputSelector` 0), Send
//! (`SendTakeButtonSelector` 0), Take All hidden, Reply disabled. A received one: `From`, the fields made read-only
//! (`HTMLParser_c::ClearFeatureFlags(1)`, the text no longer typed in), the delivery choice disabled, Reply / Return To
//! Sender / Delete (`SendTakeButtonSelector` 1), the text "Loading, please wait..." until the server's mail comes
//! (`N3Msg_RequestMailMessage`), Take All and Delete enabled once it has.
//!
//! Since 2026-11-16, from the window's code: a mail's money is signed, and a negative amount is cash on delivery.
//! A received mail shows it in the C.O.D. field ([`received`], FUN_10107b87), Take All asks before paying it
//! (FUN_10106692: `ConfirmTakeMailItems`), and Return To Sender is on for a mail with an item or with something
//! to pay that was not taken and may be returned (its handler at 0x101063d4 is `N3Msg_ReturnMail`). Sending
//! (FUN_1010733d): the Credits field's number, else the C.O.D. field's negated; a negative number in either is
//! `Feedback_MailNoNegative`; a text over 2000 letters (4000 bytes) or a subject over 64 is
//! `Feedback_TextSubjectTooLong`; the name is written with a capital and small letters; with money or a C.O.D.
//! the window asks first (`ConfirmSendMail`, `ConfirmSendMailCOD`). **None of this was sent to a server.**

use crate::gear::{GearItem, GearWindow};
use crate::gui::view::{Id, Tree};
use crate::mailbox::{MailCommand, Opened};
use crate::theme::Palette;
use mao_net::mail;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Compose,
    Read(u32),
}

pub struct MessageWindow {
    pub open: bool,
    pub mode: Mode,
    /// A new mail's item: its backpack slot.
    item: Option<u32>,
    /// The received mail shown (once its text came).
    filled: Option<u32>,
    /// The received mail shown as taken: nothing left in it.
    taken: Option<u32>,
    /// What to put in a new mail's fields when it is built (a reply's).
    prefill: Option<(String, String)>,
    pub status: String,
    /// What waits for a yes: a mail to send with money or a C.O.D., or a C.O.D. mail's things to take.
    asking: Option<Asking>,
    window: Option<crate::gui::window::XmlWindow>,
}

#[derive(Clone, Debug, PartialEq)]
enum Asking {
    Send { text: String, command: MailCommand },
    Take { text: String, mail: u32 },
}

impl Default for MessageWindow {
    fn default() -> MessageWindow {
        MessageWindow { open: false, mode: Mode::Compose, item: None, filled: None, taken: None, prefill: None, status: String::new(), asking: None, window: None }
    }
}

fn field(tree: &Tree, root: Id, name: &str) -> String {
    tree.find(root, name).and_then(|id| crate::gui::textinput::text(tree, id)).unwrap_or("").to_string()
}

fn set_field(tree: &mut Tree, root: Id, name: &str, value: &str) {
    if let Some(id) = tree.find(root, name) {
        crate::gui::textinput::set_text(tree, id, value);
    }
}

fn enable(tree: &mut Tree, root: Id, name: &str, on: bool) {
    if let Some(id) = tree.find(root, name) {
        tree.get_mut(id).enabled = on;
    }
}

/// What a received mail shows and allows (GUI.dll FUN_10107b87). Bit 1 of the flags is "taken", and a taken mail
/// shows no item and no money. Money sent along is in the Credits field and cash on delivery (a negative amount)
/// in the C.O.D. field, the other "0". Take All is on while there is an item or money either way, and Delete only
/// when it is off. Return To Sender is on for a mail not taken, not barred from it (bit 4), that has an item or
/// something to pay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Received {
    cash: u32,
    cod: u32,
    take: bool,
    give_back: bool,
}

fn received(m: &mail::Mail) -> Received {
    let taken = m.flags & mail::TAKEN != 0;
    let has_item = !taken && m.item.is_some();
    let money = if taken { 0 } else { m.credits };
    Received { cash: money.max(0) as u32, cod: money.min(0).unsigned_abs(), take: has_item || money != 0, give_back: !taken && m.flags & mail::NO_RETURN == 0 && (has_item || m.credits < 0) }
}

/// What a new mail's two money fields come to (FUN_1010733d): the credits to send, a C.O.D. as a negative number,
/// or `None` when either field holds a negative number. The fields are read as C's `atoi` reads them.
fn money_to_send(cash: &str, cod: &str) -> Option<i32> {
    let number = |text: &str| {
        let text = text.trim_start();
        let digits: String = text.chars().enumerate().take_while(|&(k, c)| c.is_ascii_digit() || (k == 0 && (c == '-' || c == '+'))).map(|(_, c)| c).collect();
        digits.parse::<i64>().unwrap_or(0).clamp(i32::MIN as i64 + 1, i32::MAX as i64) as i32
    };
    match (number(cash), number(cod)) {
        (cash, cod) if cash < 0 || cod < 0 => None,
        (cash, _) if cash >= 1 => Some(cash),
        (_, cod) if cod >= 1 => Some(-cod),
        _ => Some(0),
    }
}

/// The recipient's name as the original sends it: small letters after a capital.
fn recipient_name(to: &str) -> String {
    let lower = to.trim().to_lowercase();
    let mut letters = lower.chars();
    letters.next().map_or(String::new(), |first| first.to_uppercase().chain(letters).collect())
}

impl MessageWindow {
    /// A new mail, to `to` about `subject` (both may be empty).
    pub fn compose(&mut self, to: &str, subject: &str) {
        self.open = true;
        self.mode = Mode::Compose;
        self.item = None;
        self.status.clear();
        self.prefill = Some((to.to_string(), subject.to_string()));
        self.window = None;
    }

    /// A received mail, by its id (its text comes from the server).
    /// The mail it shows was deleted: the window goes (FUN_101096f1 closes the mail's message window).
    pub fn close_mail(&mut self, id: u32) {
        if matches!(self.mode, Mode::Read(m) if m == id) {
            self.open = false;
        }
    }

    pub fn read(&mut self, id: u32) {
        self.open = true;
        self.mode = Mode::Read(id);
        self.filled = None;
        self.status.clear();
        self.window = None;
    }

    /// FUN_10107fc8: the view and what each mode shows.
    fn load(&mut self, ctx: &egui::Context) {
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        let Some((mut tree, root)) = crate::gui::load_view(&dirs, "MailMessageWindow.xml", "") else { return };
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let read = matches!(self.mode, Mode::Read(_));
        for name in ["FromToLabelSelector", "FromToInputSelector", "SendTakeButtonSelector"] {
            if let Some(id) = tree.find(root, name) {
                crate::gui::selector::set_value(&mut tree, id, i32::from(read));
            }
        }
        if read {
            // (the received mail's fields read only: their text no longer typed in)
            for name in ["From", "Subject", "MessageText", "Cash", "COD", "Total"] {
                if let Some(f) = tree.find(root, name)
                    && let Some(editor) = tree.find(f, "_editor")
                    && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(editor)
                {
                    t.feature &= !crate::gui::text::flags::ACCEPT_TXT_INPUT;
                }
            }
            set_field(&mut tree, root, "MessageText", "Loading, please wait...");
            for name in ["SendButton", "ReturnButton", "TakeButton", "Delete", "DeliveryMode"] {
                enable(&mut tree, root, name, false);
            }
        } else {
            if let Some(id) = tree.find(root, "TakeButton") {
                tree.get_mut(id).hidden += 1;
            }
            enable(&mut tree, root, "ReplyButton", false);
            if let Some((to, subject)) = self.prefill.take() {
                set_field(&mut tree, root, "To", &to);
                set_field(&mut tree, root, "Subject", &subject);
            }
            if let Some(id) = tree.find(root, "DeliveryMode") {
                crate::gui::radio::set_value(&mut tree, id, "0");
            }
        }
        self.window = Some(crate::gui::window::XmlWindow { name: "Message".into(), title: "Message".into(), tree, root, open: true, default_size: egui::vec2(380.0, 0.0) });
    }

    /// Draws the window; returns what to send, if anything.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, items: &[GearItem], opened: Option<&Opened>, palette: &Palette) -> Option<MailCommand> {
        if !self.open {
            return None;
        }
        if self.window.is_none() {
            self.load(ctx);
        }
        let w = self.window.as_mut()?;
        let root = w.root;
        let mut command = None;
        // a received mail's text, once it came (FUN_10107b87: the mail's fields, Take All while there is something to take)
        if let Mode::Read(id) = self.mode
            && self.filled != Some(id)
            && let Some(o) = opened.filter(|o| o.mail.id == id)
        {
            let m = &o.mail;
            set_field(&mut w.tree, root, "From", &m.sender);
            set_field(&mut w.tree, root, "Subject", &m.subject);
            set_field(&mut w.tree, root, "MessageText", &m.body);
            // (a mail whose things were taken shows neither item nor money; Delete only when nothing is left to take, the
            // original's two `View::Enable` in a pair)
            let shown = received(m);
            set_field(&mut w.tree, root, "Cash", &shown.cash.to_string());
            set_field(&mut w.tree, root, "COD", &shown.cod.to_string());
            enable(&mut w.tree, root, "TakeButton", shown.take);
            enable(&mut w.tree, root, "Delete", !shown.take);
            enable(&mut w.tree, root, "ReturnButton", shown.give_back);
            // (the delivery choice shows how the mail came: bit 5 of its flags)
            if let Some(group) = w.tree.find(root, "DeliveryMode") {
                crate::gui::radio::set_value(&mut w.tree, group, if m.flags & mail::EXPRESS != 0 { "1" } else { "0" });
            }
            self.filled = Some(id);
            self.taken = (m.flags & mail::TAKEN != 0).then_some(id);
        }
        // the server's word that the mail's things were taken (the flags' change, FUN_10106eaa: the item's place emptied,
        // the money fields "0", Take All and Return To Sender off, Delete on)
        if let Mode::Read(id) = self.mode
            && self.filled == Some(id)
            && self.taken != Some(id)
            && opened.is_some_and(|o| o.mail.id == id && o.mail.flags & mail::TAKEN != 0)
        {
            set_field(&mut w.tree, root, "Cash", "0");
            set_field(&mut w.tree, root, "COD", "0");
            enable(&mut w.tree, root, "TakeButton", false);
            enable(&mut w.tree, root, "Delete", true);
            enable(&mut w.tree, root, "ReturnButton", false);
            self.taken = Some(id);
        }
        // the item: a received mail's, a new one's from the backpack
        if let Some(slot) = w.tree.find(root, "ItemView") {
            let shown = match self.mode {
                Mode::Read(_) => opened.filter(|o| o.mail.flags & mail::TAKEN == 0).and_then(|o| o.item.as_ref()).and_then(|(name, icon)| icon.as_ref().map(|icon| crate::gui::itemslot::SlotItem { texture: gear.texture(ctx, icon), name: name.clone() })),
                Mode::Compose => self.item.and_then(|s| items.iter().find(|i| i.slot == s)).and_then(|it| it.icon.as_ref().map(|icon| crate::gui::itemslot::SlotItem { texture: gear.texture(ctx, icon), name: it.name.clone() })),
            };
            crate::gui::itemslot::set_item(&mut w.tree, slot, shown);
        }
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
        let tree = &mut w.tree;
        let compose = self.mode == Mode::Compose;
        let item = &mut self.item;
        let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
        let mut is_open = self.open;
        crate::chrome::window(ctx, palette, "Message", "Message", &mut is_open, |w| w.default_size(min).resizable(false), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            // (a backpack item let go over the item place of a new mail)
            if compose
                && dragged != u32::MAX
                && crate::gear::letting_go(ui.ctx())
                && let (Some(at), Some(slot)) = (ui.input(|i| i.pointer.interact_pos()), tree.find(root, "ItemView"))
                && crate::gui::host::screen_rect(tree, slot, response.rect.min).contains(at)
                && dragged >= mao_net::inventory::BACKPACK_FIRST
            {
                *item = Some(dragged);
            }
        });
        crate::gui::radio::take_signals(&mut w.tree);
        for (id, signal) in std::mem::take(&mut w.tree.signals) {
            if signal != crate::gui::host::Signal::Clicked {
                continue;
            }
            match w.tree.get(id).name.as_str() {
                "SendButton" => {
                    let (to, subject, body) = (field(&w.tree, root, "To"), field(&w.tree, root, "Subject"), field(&w.tree, root, "MessageText"));
                    let money = money_to_send(&field(&w.tree, root, "Cash"), &field(&w.tree, root, "COD"));
                    let express = w.tree.find(root, "DeliveryMode").and_then(|g| crate::gui::radio::value(&w.tree, g)).is_some_and(|v| v == "1");
                    let feedback = |name: &str, own: &str| crate::gui::xml::ldb_named(mao_formats::ldb::FEEDBACK, name, own);
                    if body.chars().count() > 2000 || body.len() > 4000 || subject.chars().count() > 0x40 {
                        crate::chrome::alert(feedback("Feedback_TextSubjectTooLong", "The text or the subject is too long."));
                    // (FUN_1010733d checks no recipient or subject: an empty one goes to the server as it is)
                    } else if let Some(credits) = money {
                        let recipient = recipient_name(&to);
                        let send = MailCommand::Send { recipient: recipient.clone(), subject: subject.trim().to_string(), body, item: self.item.map_or((0, 0), |s| (0x68, s)), credits, express };
                        // (with money or a C.O.D. the window asks first, naming the item, the name and the amount)
                        let item_name = self.item.and_then(|s| items.iter().find(|i| i.slot == s)).map_or("no item".to_string(), |i| i.name.clone());
                        let amount = crate::vitals::grouped(credits.unsigned_abs() as i64);
                        let text = |name: &str, own: &str| crate::gui::xml::ldb_named(1000, name, own);
                        use mao_formats::ldb::{Arg, format};
                        if credits > 0 {
                            let text = format(&text("ConfirmSendMail", "Send %s credits and %s to %s?"), &[Arg::Text(&amount), Arg::Text(&item_name), Arg::Text(&recipient)]);
                            self.asking = Some(Asking::Send { text, command: send });
                        } else if credits < 0 {
                            let text = format(&text("ConfirmSendMailCOD", "Send %s to %s and ask %s credits on delivery?"), &[Arg::Text(&item_name), Arg::Text(&recipient), Arg::Text(&amount)]);
                            self.asking = Some(Asking::Send { text, command: send });
                        } else {
                            command = Some(send);
                            is_open = false;
                        }
                    } else {
                        crate::chrome::alert(feedback("Feedback_MailNoNegative", "A mail's credits cannot be negative."));
                    }
                }
                "ItemView" if compose => self.item = None,
                // (FUN_10106692: a mail with cash on delivery asks first, naming what is to be paid)
                "TakeButton" => {
                    if let Mode::Read(m) = self.mode {
                        match opened.filter(|o| o.mail.id == m && o.mail.credits < 0) {
                            Some(o) => {
                                let amount = crate::vitals::grouped(o.mail.credits.unsigned_abs() as i64);
                                let text = mao_formats::ldb::format(&crate::gui::xml::ldb_named(1000, "ConfirmTakeMailItems", "Take the mail's things and pay %s credits to the sender?"), &[mao_formats::ldb::Arg::Text(&amount)]);
                                self.asking = Some(Asking::Take { text, mail: m });
                            }
                            None => {
                                command = Some(MailCommand::Take(m));
                                enable(&mut w.tree, root, "TakeButton", false);
                            }
                        }
                    }
                }
                // (0x101063d4: `N3Msg_ReturnMail` with the mail's number)
                "ReturnButton" => {
                    if let Mode::Read(m) = self.mode {
                        command = Some(MailCommand::Return(m));
                        enable(&mut w.tree, root, "ReturnButton", false);
                        is_open = false;
                    }
                }
                // (the owner asks "delete?" and closes this window when the mail goes: `MailWindow::delete`)
                "Delete" => {
                    if let Mode::Read(m) = self.mode {
                        command = Some(MailCommand::Delete(m));
                    }
                }
                "ReplyButton" => {
                    // (FUN_10107a0a: a new mail to the sender about the subject)
                    if let Some(o) = opened.filter(|o| Some(o.mail.id) == self.filled) {
                        let (to, subject) = (o.mail.sender.clone(), format!("Re: {}", o.mail.subject));
                        self.compose(&to, &subject);
                        return command;
                    }
                }
                "Close" => is_open = false,
                _ => {}
            }
        }
        // (the questions: titled as the original titles its boxes)
        match self.asking.clone() {
            Some(Asking::Send { text, command: send }) => {
                if let Some(yes) = crate::chrome::question(ctx, palette, "ConfirmSendMail", "ConfirmSendMail", &text) {
                    self.asking = None;
                    if yes {
                        command = Some(send);
                        is_open = false;
                    }
                }
            }
            Some(Asking::Take { text, mail }) => {
                if let Some(yes) = crate::chrome::question(ctx, palette, "MailTakeItems", "MailTakeItems", &text) {
                    self.asking = None;
                    if yes {
                        command = Some(MailCommand::Take(mail));
                        enable(&mut w.tree, root, "TakeButton", false);
                    }
                }
            }
            None => {}
        }
        self.open = is_open;
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The user's mail of 2026-11-15: flags 0x7c with an item, then 0x7d and 0x7f from the server as it was taken.
    #[test]
    fn a_taken_mail_holds_nothing() {
        let mut m = mail::Mail { id: 1, sender_id: 0, sender: String::new(), subject: String::new(), sent: 0, expires: 0, flags: 0x7c, credits: 0, item: Some((1, 1, 1)), body: String::new() };
        let shown = |m: &mail::Mail| (received(m).cash, received(m).take);
        assert_eq!(shown(&m), (0, true));
        m.flags = 0x7d;
        assert_eq!(shown(&m), (0, true));
        m.flags = 0x7f;
        assert_eq!(shown(&m), (0, false));
        m.item = None;
        m.flags = 0x7c;
        m.credits = 500;
        assert_eq!(shown(&m), (500, true));
        m.credits = 0;
        assert_eq!(shown(&m), (0, false));
    }

    /// Cash on delivery is a negative amount: shown in its own field, taken only after a yes, and such a mail (or
    /// one with an item) can go back to its sender unless it was taken or is barred from it.
    #[test]
    fn cash_on_delivery_and_the_way_back() {
        let mut m = mail::Mail { id: 1, sender_id: 0, sender: String::new(), subject: String::new(), sent: 0, expires: 0, flags: mail::HAS_ITEM, credits: -750, item: Some((1, 1, 1)), body: String::new() };
        assert_eq!(received(&m), Received { cash: 0, cod: 750, take: true, give_back: true });
        m.item = None;
        m.flags = 0;
        assert_eq!(received(&m), Received { cash: 0, cod: 750, take: true, give_back: true });
        m.flags = mail::NO_RETURN;
        assert!(!received(&m).give_back);
        m.flags = mail::TAKEN;
        assert_eq!(received(&m), Received { cash: 0, cod: 0, take: false, give_back: false });
        // (money sent along, and nothing in it: no way back)
        m.flags = 0;
        m.credits = 40;
        assert_eq!(received(&m), Received { cash: 40, cod: 0, take: true, give_back: false });
        m.item = Some((1, 1, 1));
        assert!(received(&m).give_back);
        // (what a new mail's fields come to)
        assert_eq!((money_to_send("500", "0"), money_to_send("", "250"), money_to_send("500", "250"), money_to_send("", ""), money_to_send("abc", "12x")), (Some(500), Some(-250), Some(500), Some(0), Some(-12)));
        assert_eq!((money_to_send("-1", "5"), money_to_send("5", "-1")), (None, None));
        assert_eq!((recipient_name(" mUGGLES "), recipient_name("")), ("Muggles".to_string(), String::new()));
    }
}
