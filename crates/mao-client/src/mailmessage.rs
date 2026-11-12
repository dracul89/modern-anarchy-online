//! The mail message window, GUI.dll's (FUN_10107fc8) on the skin's `Views/MailMessageWindow.xml`: one window for a new mail
//! and for one received.
//!
//! A new mail (no mail id): the `To` label and field (`FromToLabelSelector`/`FromToInputSelector` 0), Send
//! (`SendTakeButtonSelector` 0), Take All hidden, Reply disabled. A received one: `From`, the fields made read-only
//! (`HTMLParser_c::ClearFeatureFlags(1)`, the text no longer typed in), the delivery choice disabled, Reply / Return To
//! Sender / Delete (`SendTakeButtonSelector` 1), the text "Loading, please wait..." until the server's mail comes
//! (`N3Msg_RequestMailMessage`), Take All and Delete enabled once it has. Return To Sender is left disabled, as the window
//! starts it (its message is not known here); C.O.D. is not sent (the captured send has no field for it).

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
    /// What to put in a new mail's fields when it is built (a reply's).
    prefill: Option<(String, String)>,
    pub status: String,
    window: Option<crate::gui::window::XmlWindow>,
}

impl Default for MessageWindow {
    fn default() -> MessageWindow {
        MessageWindow { open: false, mode: Mode::Compose, item: None, filled: None, prefill: None, status: String::new(), window: None }
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
            set_field(&mut w.tree, root, "Cash", &m.credits.to_string());
            let taken = m.flags & mail::TAKEN != 0;
            enable(&mut w.tree, root, "TakeButton", !taken && (m.credits > 0 || m.item.is_some()));
            enable(&mut w.tree, root, "Delete", true);
            self.filled = Some(id);
        }
        // the item: a received mail's, a new one's from the backpack
        if let Some(slot) = w.tree.find(root, "ItemView") {
            let shown = match self.mode {
                Mode::Read(_) => opened.and_then(|o| o.item.as_ref()).and_then(|(name, icon)| icon.as_ref().map(|icon| crate::gui::itemslot::SlotItem { texture: gear.texture(ctx, icon), name: name.clone() })),
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
                && ui.input(|i| i.pointer.any_released())
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
                    let credits = field(&w.tree, root, "Cash").trim().parse().unwrap_or(0);
                    let express = w.tree.find(root, "DeliveryMode").and_then(|g| crate::gui::radio::value(&w.tree, g)).is_some_and(|v| v == "1");
                    if to.trim().is_empty() || subject.trim().is_empty() {
                        self.status = "A mail needs a recipient and a subject.".into();
                    } else {
                        command = Some(MailCommand::Send { recipient: to.trim().to_string(), subject: subject.trim().to_string(), body, item: self.item.map_or((0, 0), |s| (0x68, s)), credits, express });
                        is_open = false;
                    }
                }
                "ItemView" if compose => self.item = None,
                "TakeButton" => {
                    if let Mode::Read(m) = self.mode {
                        command = Some(MailCommand::Take(m));
                        enable(&mut w.tree, root, "TakeButton", false);
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
        self.open = is_open;
        command
    }
}
