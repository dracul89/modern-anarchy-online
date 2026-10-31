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
}

impl Default for MailWindow {
    fn default() -> MailWindow {
        MailWindow { open: false, count: None, entries: Vec::new(), selected: None, opened: None, status: String::new(), message: Default::default() }
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

    /// A mail's flags changed (read, taken ...).
    pub fn set_flags(&mut self, id: u32, flags: u32) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.flags = flags;
        }
        if let Some(o) = self.opened.as_mut().filter(|o| o.mail.id == id) {
            o.mail.flags = flags;
        }
    }

    /// Draws the windows; returns what to send, if anything.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, items: &[GearItem], palette: &Palette) -> Option<MailCommand> {
        let mut command = None;
        // The inbox.
        if self.open {
            let mut is_open = true;
            let mut close = false;
            crate::chrome::window(ctx, palette, "Mail", "Mail", &mut is_open, |w| w.resizable(false).default_pos(crate::layout::pos("Mail", egui::pos2(520.0, 90.0))), |ui| {
                ui.set_width(360.0);
                ui.horizontal(|ui| {
                    ui.label("Messages");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("Size: {}/30", self.count.unwrap_or(0)));
                    });
                });
                egui::Frame::new().fill(egui::Color32::from_black_alpha(110)).stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(40))).inner_margin(4).show(ui, |ui| {
                    ui.set_min_height(120.0);
                    ui.horizontal(|ui| {
                        ui.add_sized([230.0, 16.0], egui::Label::new(egui::RichText::new("Subject").weak()));
                        ui.label(egui::RichText::new("Sent").weak());
                    });
                    ui.separator();
                    if self.entries.is_empty() {
                        ui.label(egui::RichText::new(if self.count.is_none() { "Press Check mail at a mail terminal." } else { "No messages." }).weak());
                    }
                    crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(150.0), |ui| {
                        for e in &self.entries {
                            let unread = e.flags & mail::READ == 0;
                            let mut label = egui::RichText::new(&e.subject);
                            if unread {
                                label = label.strong();
                            }
                            let mark = format!("{}{}", if e.flags & mail::HAS_ITEM != 0 && e.flags & mail::TAKEN == 0 { " [item]" } else { "" }, if e.flags & mail::HAS_CREDITS != 0 && e.flags & mail::TAKEN == 0 { " [credits]" } else { "" });
                            ui.horizontal(|ui| {
                                if ui.selectable_label(self.selected == Some(e.id), label).clicked() {
                                    self.selected = Some(e.id);
                                    self.message.read(e.id);
                                    command = Some(MailCommand::Open(e.id));
                                }
                                ui.label(egui::RichText::new(mail::time_text(e.sent)).small().weak());
                            });
                            if !mark.is_empty() {
                                ui.label(egui::RichText::new(format!("   from {}{}", e.sender, mark)).small().weak());
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, "Check mail", true).on_hover_text("Uses the nearest known mail terminal and asks for the inbox").clicked() {
                        command = Some(MailCommand::Inbox);
                        self.status = "Asking the mail terminal ...".into();
                    }
                    if crate::chrome::button(ui, palette, "New", true).clicked() {
                        self.message.compose("", "");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::chrome::button(ui, palette, "Close", true).clicked() {
                            close = true;
                        }
                    });
                });
                if !self.status.is_empty() {
                    ui.label(egui::RichText::new(&self.status).small());
                }
            });
            self.open = is_open && !close;
        }
        // The message window (a new mail, or the one opened).
        if let Some(c) = self.message.show(ctx, gear, items, self.opened.as_ref(), palette) {
            if matches!(c, MailCommand::Send { .. }) {
                self.status = "Sending ...".into();
            }
            command = Some(c);
        }
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_known_terminal() {
        let old_athen: Vec<(f32, f32, u32)> = KNOWN_TERMINALS.iter().filter(|t| t.0 == 540).map(|t| (t.1, t.2, t.3)).collect();
        assert_eq!(nearest_in(&old_athen, 446.0, 455.0), Some(0xc003_021c));
        assert_eq!(nearest_in(&old_athen, 410.0, 405.0), Some(0xc002_021c));
        assert_eq!(nearest_in(&old_athen, 300.0, 300.0), None);
    }
}
