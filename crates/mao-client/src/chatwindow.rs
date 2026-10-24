//! The chat window: the lines that arrive (channels, tells, system notices) in tabs, and an input line. Typing a `/` opens the
//! original's list of commands under the cursor (filtered as you type, Up and Down to pick, Tab or Enter to take it); a line is
//! sent with Enter. Enter and `/` outside the window start typing (the `chat_input` and `chat_command` keys).

use crate::chat_commands::{self, Parsed};
use crate::theme::Palette;

/// What a line is, which decides its colour and tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    System,
    Tell,
    Channel,
    /// Something this character said (echoed).
    Own,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    pub text: String,
    /// The character a tell came from (for `/reply`).
    pub from: Option<String>,
}

impl Line {
    pub fn new(kind: Kind, text: impl Into<String>) -> Line {
        Line { kind, text: text.into(), from: None }
    }
}

/// Something to send on the chat connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    Tell { name: String, text: String },
    Org { text: String },
    /// Put a character on the friends list / take one off.
    Befriend { name: String },
    Unfriend { id: u32 },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    All,
    Tells,
    Channels,
    System,
}

pub struct ChatWindow {
    pub open: bool,
    lines: Vec<Line>,
    tab: Tab,
    input: String,
    /// The highlighted entry of the command list.
    selected: usize,
    focus_next: bool,
    last_tell: Option<String>,
    scroll_to_end: bool,
    /// Keeps the command list shown when the window itself has no keyboard focus (the previews).
    keep_list: bool,
}

impl Default for ChatWindow {
    fn default() -> Self {
        ChatWindow { open: true, lines: Vec::new(), tab: Tab::All, input: String::new(), selected: 0, focus_next: false, last_tell: None, scroll_to_end: true, keep_list: false }
    }
}

/// How many lines are kept.
const KEEP: usize = 600;

impl ChatWindow {
    pub fn push(&mut self, line: Line) {
        if line.kind == Kind::Tell {
            if let Some(from) = &line.from {
                self.last_tell = Some(from.clone());
            }
        }
        self.lines.push(line);
        if self.lines.len() > KEEP {
            self.lines.drain(..self.lines.len() - KEEP);
        }
        self.scroll_to_end = true;
    }

    /// Puts text in the input line and starts typing there (the previews use it).
    pub fn type_text(&mut self, text: &str) {
        self.input = text.to_string();
        self.keep_list = true;
        self.focus();
    }

    /// The Enter key: start typing in the window (opening it if it was closed).
    pub fn focus(&mut self) {
        self.open = true;
        self.focus_next = true;
    }

    /// Carries out a typed line: what to send (if anything), and the lines it adds to the window.
    pub fn run(&mut self, line: &str) -> Option<Out> {
        match chat_commands::parse(line) {
            Parsed::Empty => None,
            Parsed::Tell { name, text } => {
                self.push(Line::new(Kind::Own, format!("To {name}: {text}")));
                Some(Out::Tell { name, text })
            }
            Parsed::Reply { text } => match self.last_tell.clone() {
                Some(name) => {
                    self.push(Line::new(Kind::Own, format!("To {name}: {text}")));
                    Some(Out::Tell { name, text })
                }
                None => {
                    self.push(Line::new(Kind::System, "There is nobody to reply to."));
                    None
                }
            },
            Parsed::Befriend { name } => {
                self.push(Line::new(Kind::System, format!("Adding {name} to your friends.")));
                Some(Out::Befriend { name })
            }
            Parsed::Org { text } => {
                self.push(Line::new(Kind::Own, format!("[Org] {text}")));
                Some(Out::Org { text })
            }
            Parsed::Vicinity { command, .. } => {
                self.push(Line::new(Kind::System, format!("Speaking to the people around ({command}) is not available in this client yet.")));
                None
            }
            Parsed::Help => {
                self.push(Line::new(Kind::System, "Commands this client carries out:"));
                for (usage, what) in chat_commands::IMPLEMENTED {
                    self.push(Line::new(Kind::System, format!("  /{usage}: {what}")));
                }
                self.push(Line::new(Kind::System, "The other commands of the original are listed when you type a / but are not carried out yet."));
                None
            }
            Parsed::Version => {
                self.push(Line::new(Kind::System, format!("Modern Anarchy Online client {}", env!("CARGO_PKG_VERSION"))));
                None
            }
            Parsed::Unsupported(name) => {
                self.push(Line::new(Kind::System, format!("/{name} is not carried out by this client yet.")));
                None
            }
            Parsed::Unknown(word) => {
                self.push(Line::new(Kind::System, format!("Unknown command /{word}. Type /help.")));
                None
            }
            Parsed::Usage(usage) => {
                self.push(Line::new(Kind::System, format!("Usage: {usage}")));
                None
            }
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<Out> {
        if !self.open {
            return None;
        }
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut out = None;
        let screen = ctx.content_rect();
        let mut open = self.open;
        egui::Window::new(egui::RichText::new("Chat").color(accent))
            .open(&mut open)
            .collapsible(false)
            .default_pos(egui::pos2(8.0, (screen.height() - 270.0).max(40.0)))
            .default_size([560.0, 230.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (tab, name) in [(Tab::All, "All"), (Tab::Tells, "Tells"), (Tab::Channels, "Channels"), (Tab::System, "System")] {
                        ui.selectable_value(&mut self.tab, tab, name);
                    }
                });
                let height = (ui.available_height() - 30.0).max(40.0);
                egui::ScrollArea::vertical().max_height(height).min_scrolled_height(height).stick_to_bottom(true).show(ui, |ui| {
                    for line in self.lines.iter().filter(|l| match self.tab {
                        Tab::All => true,
                        Tab::Tells => matches!(l.kind, Kind::Tell | Kind::Own),
                        Tab::Channels => l.kind == Kind::Channel,
                        Tab::System => l.kind == Kind::System,
                    }) {
                        let color = match line.kind {
                            Kind::System => egui::Color32::from_rgb(255, 220, 90),
                            Kind::Tell => egui::Color32::from_rgb(120, 220, 255),
                            Kind::Channel => egui::Color32::from_rgb(200, 235, 235),
                            Kind::Own => egui::Color32::from_rgb(160, 200, 160),
                        };
                        ui.label(egui::RichText::new(&line.text).color(color));
                    }
                });
                self.scroll_to_end = false;
                let input_id = egui::Id::new("chat-input");
                if self.focus_next {
                    ctx.memory_mut(|m| m.request_focus(input_id));
                    self.focus_next = false;
                }
                let suggestions = chat_commands::suggest(&self.input);
                let has_focus = ctx.memory(|m| m.has_focus(input_id));
                // The list's keys are taken before the field sees them.
                let (mut up, mut down, mut take) = (false, false, false);
                if has_focus && !suggestions.is_empty() {
                    ctx.input_mut(|i| {
                        up = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
                        down = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
                        take = i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                    });
                }
                self.selected = self.selected.min(suggestions.len().saturating_sub(1));
                if up {
                    self.selected = self.selected.saturating_sub(1);
                }
                if down && self.selected + 1 < suggestions.len() {
                    self.selected += 1;
                }
                let response = ui.add(egui::TextEdit::singleline(&mut self.input).id(input_id).desired_width(f32::INFINITY).hint_text("Type to speak, / for commands"));
                if response.changed() {
                    self.selected = 0;
                    self.keep_list = false;
                }
                let enter = response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter));
                // Enter on a half-typed command takes the highlighted one instead of sending.
                let complete = !suggestions.is_empty() && (take || (enter && !suggestions.iter().any(|c| format!("/{c}").eq_ignore_ascii_case(self.input.trim()))));
                if complete {
                    self.input = format!("/{} ", suggestions[self.selected.min(suggestions.len() - 1)]);
                    response.request_focus();
                    // put the cursor after the completion
                    if let Some(mut state) = egui::TextEdit::load_state(ctx, input_id) {
                        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(self.input.chars().count()))));
                        state.store(ctx, input_id);
                    }
                } else if enter {
                    let line = std::mem::take(&mut self.input);
                    out = self.run(&line);
                    response.request_focus();
                }
                // The list of commands, above the input line, as the original's.
                if (has_focus || response.has_focus() || self.keep_list) && !suggestions.is_empty() {
                    // as many rows as fit above the input line, scrolling to keep the highlighted one in view
                    let fits = (((response.rect.top() - 14.0) / 15.0).max(3.0)) as usize;
                    let rows = suggestions.len().min(fits);
                    let first = if self.selected >= rows { self.selected + 1 - rows } else { 0 };
                    egui::Area::new(egui::Id::new("chat-commands")).order(egui::Order::Foreground).fixed_pos(egui::pos2(response.rect.left(), response.rect.top() - 4.0 - rows as f32 * 15.0)).show(ctx, |ui| {
                        egui::Frame::new().fill(egui::Color32::from_rgb(8, 40, 50)).stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(20, 90, 100))).inner_margin(4.0).show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for (n, command) in suggestions.iter().enumerate().skip(first).take(rows) {
                                let chosen = n == self.selected;
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(150.0, 15.0), egui::Sense::hover());
                                if chosen {
                                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 200));
                                }
                                ui.painter().text(rect.left_center() + egui::vec2(3.0, 0.0), egui::Align2::LEFT_CENTER, format!("/{command}"), egui::FontId::proportional(12.0), egui::Color32::from_rgb(220, 240, 245));
                            }
                        });
                    });
                }
            });
        self.open = open;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tell_is_echoed_and_sent_and_a_reply_goes_to_the_last_sender() {
        let mut w = ChatWindow::default();
        assert_eq!(w.run("/tell Abc hello"), Some(Out::Tell { name: "Abc".into(), text: "hello".into() }));
        assert_eq!(w.lines.last().unwrap().text, "To Abc: hello");
        assert_eq!(w.run("/r anyone"), None); // nobody has told us anything yet
        w.push(Line { kind: Kind::Tell, text: "Zed: hi".into(), from: Some("Zed".into()) });
        assert_eq!(w.run("/r hello Zed"), Some(Out::Tell { name: "Zed".into(), text: "hello Zed".into() }));
    }

    #[test]
    fn local_commands_only_add_lines() {
        let mut w = ChatWindow::default();
        assert_eq!(w.run("/help"), None);
        assert!(w.lines.len() >= 6);
        assert_eq!(w.run("/camp"), None);
        assert!(w.lines.last().unwrap().text.contains("not carried out"));
        assert_eq!(w.run("hello"), None);
        assert!(w.lines.last().unwrap().text.contains("not available"));
    }

    #[test]
    fn only_the_newest_lines_are_kept() {
        let mut w = ChatWindow::default();
        for i in 0..(KEEP + 50) {
            w.push(Line::new(Kind::System, i.to_string()));
        }
        assert_eq!(w.lines.len(), KEEP);
        assert_eq!(w.lines[0].text, "50");
    }
}
