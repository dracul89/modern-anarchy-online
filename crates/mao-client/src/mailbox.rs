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

/// The known terminal nearest to a place in a playfield (within 25 units).
pub fn nearest_terminal(playfield: u32, x: f32, z: f32) -> Option<u32> {
    KNOWN_TERMINALS.iter().filter(|t| t.0 == playfield).map(|t| (((t.1 - x).powi(2) + (t.2 - z).powi(2)).sqrt(), t.3)).filter(|(d, _)| *d < 25.0).min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, s)| s)
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
    pub compose_open: bool,
    /// Messages in the inbox, as the server last reported (None before it answered).
    pub count: Option<usize>,
    pub entries: Vec<Entry>,
    pub selected: Option<u32>,
    pub opened: Option<Opened>,
    pub status: String,
    to: String,
    subject: String,
    body: String,
    credits: u32,
    cod: u32,
    /// The attached item: the backpack slot.
    item: Option<u32>,
    express: bool,
}

impl Default for MailWindow {
    fn default() -> MailWindow {
        MailWindow { open: false, compose_open: false, count: None, entries: Vec::new(), selected: None, opened: None, status: String::new(), to: String::new(), subject: String::new(), body: String::new(), credits: 0, cod: 0, item: None, express: false }
    }
}

impl MailWindow {
    /// A sample for a headless preview.
    pub fn sample() -> MailWindow {
        MailWindow { open: true, compose_open: true, count: Some(0), to: "muggles".into(), subject: "hi".into(), express: true, ..Default::default() }
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
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut command = None;
        // The inbox.
        if self.open {
            let mut is_open = true;
            let mut close = false;
            egui::Window::new(egui::RichText::new("Mail").color(accent)).open(&mut is_open).collapsible(false).resizable(false).default_pos(egui::pos2(520.0, 90.0)).show(ctx, |ui| {
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
                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
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
                if let Some(o) = self.opened.as_ref().filter(|o| Some(o.mail.id) == self.selected) {
                    let m = &o.mail;
                    ui.separator();
                    ui.label(egui::RichText::new(&m.subject).strong().color(accent));
                    ui.label(egui::RichText::new(format!("From {} on {}", m.sender, mail::time_text(m.sent))).small().weak());
                    if !m.body.is_empty() {
                        ui.label(&m.body);
                    }
                    let taken = m.flags & mail::TAKEN != 0;
                    if m.credits > 0 {
                        ui.label(format!("Credits: {}{}", m.credits, if taken { " (taken)" } else { "" }));
                    }
                    if let (Some((low, _, quality)), Some((name, icon))) = (m.item, o.item.as_ref()) {
                        ui.horizontal(|ui| {
                            if let Some(icon) = icon {
                                let id = gear.texture(ctx, icon);
                                ui.add(egui::Image::new((id, egui::vec2(32.0, 32.0))));
                            }
                            ui.label(format!("{name} (QL {quality}, #{low:x}){}", if taken { " (taken)" } else { "" }));
                        });
                    }
                    ui.horizontal(|ui| {
                        let can_take = !taken && (m.credits > 0 || m.item.is_some());
                        if ui.add_enabled(can_take, egui::Button::new("Take")).on_hover_text("Takes the item and the credits").clicked() {
                            command = Some(MailCommand::Take(m.id));
                        }
                        if ui.button("Delete").clicked() {
                            command = Some(MailCommand::Delete(m.id));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Check mail").on_hover_text("Uses the nearest known mail terminal and asks for the inbox").clicked() {
                        command = Some(MailCommand::Inbox);
                        self.status = "Asking the mail terminal ...".into();
                    }
                    if ui.button("New").clicked() {
                        self.compose_open = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
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
        // The compose window.
        if self.compose_open {
            let mut is_open = true;
            let mut close = false;
            egui::Window::new(egui::RichText::new("Message").color(accent)).open(&mut is_open).collapsible(false).resizable(false).default_pos(egui::pos2(60.0, 90.0)).show(ctx, |ui| {
                ui.set_width(380.0);
                egui::Grid::new("compose").num_columns(2).spacing(egui::vec2(8.0, 4.0)).show(ui, |ui| {
                    ui.label("To");
                    ui.add(egui::TextEdit::singleline(&mut self.to).desired_width(300.0));
                    ui.end_row();
                    ui.label("Subject");
                    ui.add(egui::TextEdit::singleline(&mut self.subject).desired_width(300.0));
                    ui.end_row();
                });
                ui.add(egui::TextEdit::multiline(&mut self.body).desired_rows(8).desired_width(f32::INFINITY));
                egui::Grid::new("compose-extras").num_columns(2).spacing(egui::vec2(8.0, 4.0)).show(ui, |ui| {
                    ui.label("Credits");
                    ui.add(egui::DragValue::new(&mut self.credits).range(0..=u32::MAX).speed(100.0));
                    ui.end_row();
                    ui.label("C.O.D.");
                    ui.add(egui::DragValue::new(&mut self.cod).range(0..=u32::MAX).speed(100.0)).on_hover_text("Collect on delivery: not sent yet (the message in the capture has no field for it)");
                    ui.end_row();
                    ui.label("Item");
                    let selected = self.item.and_then(|s| items.iter().find(|i| i.slot == s));
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(42.0, 42.0), egui::Sense::hover());
                        ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(50)), egui::StrokeKind::Inside);
                        if let Some(icon) = selected.and_then(|i| i.icon.as_ref()) {
                            let id = gear.texture(ctx, icon);
                            ui.painter().image(id, rect.shrink(3.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                        }
                        egui::ComboBox::from_id_salt("mail-item").width(240.0).selected_text(selected.map_or("(none)".to_string(), |i| i.name.clone())).show_ui(ui, |ui| {
                            if ui.selectable_label(self.item.is_none(), "(none)").clicked() {
                                self.item = None;
                            }
                            for i in items.iter().filter(|i| i.slot >= mao_net::inventory::BACKPACK_FIRST) {
                                if ui.selectable_label(self.item == Some(i.slot), &i.name).clicked() {
                                    self.item = Some(i.slot);
                                }
                            }
                        });
                    });
                    ui.end_row();
                    ui.label("Delivery mode");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.express, false, "Standard");
                        ui.radio_value(&mut self.express, true, "Express");
                    });
                    ui.end_row();
                    ui.label("Total cost");
                    ui.label(egui::RichText::new("(worked out by the server)").weak());
                    ui.end_row();
                });
                ui.horizontal(|ui| {
                    let can = !self.to.trim().is_empty() && !self.subject.trim().is_empty();
                    if ui.add_enabled(can, egui::Button::new("Send")).clicked() {
                        let item = self.item.map_or((0, 0), |s| (0x68, s));
                        command = Some(MailCommand::Send { recipient: self.to.trim().to_string(), subject: self.subject.trim().to_string(), body: self.body.clone(), item, credits: self.credits, express: self.express });
                        self.status = "Sending ...".into();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
                if !self.status.is_empty() {
                    ui.label(egui::RichText::new(&self.status).small());
                }
            });
            self.compose_open = is_open && !close;
        }
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_known_terminal() {
        assert_eq!(nearest_terminal(540, 446.0, 455.0), Some(0xc003_021c));
        assert_eq!(nearest_terminal(540, 410.0, 405.0), Some(0xc002_021c));
        assert_eq!(nearest_terminal(540, 300.0, 300.0), None);
        assert_eq!(nearest_terminal(545, 446.0, 455.0), None);
    }
}
