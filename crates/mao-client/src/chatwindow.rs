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
    /// Fighting: damage, healing and the actions performed.
    Combat,
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
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    Tell { name: String, text: String },
    Org { text: String },
    /// `/voice`: a voice line to speak (its checks and text are the chat connection's, `voices::command`).
    Voice { line: String },
    /// Put a character on the friends list / take one off.
    Befriend { name: String },
    Unfriend { id: u32 },
    /// Sit down to log out / leave the client (handled by the window, not sent to the chat server).
    Camp,
    Follow,
    Inspect,
    Quit,
    /// Show or hide a window by name (empty and hidden: all of them).
    Window { name: String, open: bool },
    Waypoint { x: f32, z: f32 },
    SelectSelf,
    /// Set a client setting (empty name: say the list).
    SetOption { name: String, value: String },
    /// The view distance to set (a command line's `/viewdist n`).
    ViewDist(u32),
    /// The character draw distance to set (0: all).
    CharDist(u32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    All,
    Tells,
    Channels,
    System,
    Combat,
}

pub struct ChatWindow {
    pub open: bool,
    /// Whether every line is also written to the chat log (the live client turns it on).
    pub log: bool,
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
    /// The characters whose tells and channel lines are hidden (lower case), kept in `ignore.ini` beside `mao.ini`.
    ignored: std::collections::BTreeSet<String>,
    /// The lines sent before, oldest first, and the one recalled with the arrow keys (`None`: the line being typed).
    history: Vec<String>,
    recalled: Option<usize>,
}

impl Default for ChatWindow {
    fn default() -> Self {
        ChatWindow { open: true, log: false, lines: Vec::new(), tab: Tab::All, input: String::new(), selected: 0, focus_next: false, last_tell: None, scroll_to_end: true, keep_list: false, ignored: load_ignored(), history: Vec::new(), recalled: None }
    }
}

fn ignore_path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("ignore.ini")
}

fn load_ignored() -> std::collections::BTreeSet<String> {
    // (previews and tests do not read the player's list)
    if std::env::var_os("MAO_UI_PREVIEW").is_some() || cfg!(test) {
        return Default::default();
    }
    std::fs::read_to_string(ignore_path()).map(|t| t.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect()).unwrap_or_default()
}

fn save_ignored(list: &std::collections::BTreeSet<String>) {
    if !cfg!(test) {
        let _ = std::fs::write(ignore_path(), list.iter().map(|n| format!("{n}\n")).collect::<String>());
    }
}

/// How many lines are kept.
const KEEP: usize = 600;

impl ChatWindow {
    pub fn push(&mut self, line: Line) {
        // (what an ignored character says is not shown)
        if matches!(line.kind, Kind::Tell | Kind::Channel) && line.from.as_ref().is_some_and(|f| self.ignored.contains(&f.to_lowercase())) {
            return;
        }
        if self.log {
            crate::chatlog::append(&line);
        }
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
    /// Remembers a sent line for the arrow keys (the same line twice in a row is kept once).
    fn remember(&mut self, line: &str) {
        let line = line.trim();
        if !line.is_empty() && self.history.last().map(String::as_str) != Some(line) {
            self.history.push(line.to_string());
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
        self.recalled = None;
    }

    /// Up (`older`) or Down: the line recalled from the history, or an empty one past the newest.
    fn recall(&mut self, older: bool) -> Option<String> {
        if self.history.is_empty() {
            return None;
        }
        let next = match (self.recalled, older) {
            (None, true) => Some(self.history.len() - 1),
            (Some(i), true) => Some(i.saturating_sub(1)),
            (Some(i), false) if i + 1 < self.history.len() => Some(i + 1),
            (Some(_), false) => None,
            (None, false) => return None,
        };
        self.recalled = next;
        Some(next.map_or(String::new(), |i| self.history[i].clone()))
    }

    pub fn run(&mut self, line: &str) -> Option<Out> {
        self.remember(line);
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
            Parsed::Voice { line } => Some(Out::Voice { line }),
            Parsed::Camp => {
                self.push(Line::new(Kind::System, "Sitting down to log out (stand up or move to cancel)."));
                Some(Out::Camp)
            }
            Parsed::Quit => Some(Out::Quit),
            Parsed::Follow => Some(Out::Follow),
            Parsed::Window { name, open } => Some(Out::Window { name, open }),
            Parsed::Waypoint { x, z } => {
                self.push(Line::new(Kind::System, format!("Walking to {x:.0}, {z:.0} (any movement key stops).")));
                Some(Out::Waypoint { x, z })
            }
            Parsed::SelectSelf => Some(Out::SelectSelf),
            Parsed::SetOption { name, value } => Some(Out::SetOption { name, value }),
            Parsed::Inspect => Some(Out::Inspect),
            Parsed::ViewDist { distance: Some(d) } => {
                self.push(Line::new(Kind::System, format!("View distance {}.", d.clamp(200, 4000))));
                Some(Out::ViewDist(d.clamp(200, 4000)))
            }
            Parsed::CharDist { distance: Some(d) } => {
                self.push(Line::new(Kind::System, if d == 0 { "Drawing all characters.".to_string() } else { format!("Drawing characters up to {} away.", d.min(4000)) }));
                Some(Out::CharDist(d.min(4000)))
            }
            Parsed::CharDist { distance: None } => {
                self.push(Line::new(Kind::System, "Usage: /chardist <distance, 0 for all>"));
                None
            }
            Parsed::ViewDist { distance: None } => {
                self.push(Line::new(Kind::System, "Usage: /viewdist <200 to 4000>"));
                None
            }
            Parsed::Ignore { name } if name.is_empty() => {
                let list = if self.ignored.is_empty() { "nobody".to_string() } else { self.ignored.iter().cloned().collect::<Vec<_>>().join(", ") };
                self.push(Line::new(Kind::System, format!("Ignoring: {list}.")));
                None
            }
            Parsed::Ignore { name } => {
                let key = name.to_lowercase();
                let text = if self.ignored.remove(&key) {
                    format!("{name} is no longer ignored.")
                } else {
                    self.ignored.insert(key);
                    format!("{name} is ignored: their tells and channel lines are hidden.")
                };
                save_ignored(&self.ignored);
                self.push(Line::new(Kind::System, text));
                None
            }
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
        let text = |name: &str, (r, g, b): (u8, u8, u8)| palette.text(name).map_or(egui::Color32::from_rgb(r, g, b), |c| egui::Color32::from_rgb(c[0], c[1], c[2]));
        let mut out = None;
        let screen = ctx.content_rect();
        let mut open = self.open;
        crate::chrome::window(ctx, palette, "Chat", "Chat", &mut open, |w| w.default_pos(crate::layout::pos("Chat", egui::pos2(8.0, (screen.height() - 270.0).max(40.0)))).default_size([560.0, 230.0]), |ui| {
                // The tabs in the original's TabView (`chrome::tab_view`), the input line in its input bar.
                let chat_font = if crate::font::enabled() { crate::font::chat(ctx) } else { None };
                const TABS: [Tab; 5] = [Tab::All, Tab::Tells, Tab::Channels, Tab::System, Tab::Combat];
                let mut tab = TABS.iter().position(|t| *t == self.tab).unwrap_or(0);
                crate::chrome::tab_view(ui, palette, &["All", "Tells", "Channels", "System", "Combat"], &mut tab, |ui, tab| {
                self.tab = TABS[tab.min(4)];
                let height = (ui.available_height() - 41.0).max(40.0);
                crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(height).min_scrolled_height(height).stick_to_bottom(true), |ui| {
                    for line in self.lines.iter().filter(|l| match self.tab {
                        Tab::All => true,
                        Tab::Tells => matches!(l.kind, Kind::Tell | Kind::Own),
                        Tab::Channels => l.kind == Kind::Channel,
                        Tab::System => l.kind == Kind::System,
                        Tab::Combat => l.kind == Kind::Combat,
                    }) {
                        let color = match line.kind {
                            // (the original's TextColors.xml names when the theme has them)
                            Kind::System => text("ct_cmd_feedback", (255, 220, 90)),
                            Kind::Tell => text("ct_itell", (120, 220, 255)),
                            Kind::Channel => text("ctch_misc", (200, 235, 235)),
                            Kind::Own => text("ct_otell", (160, 200, 160)),
                            Kind::Combat => text("CCMeHitOtherColor", (240, 170, 130)),
                        };
                        match &chat_font {
                            Some(font) => {
                                font.label(ui, &line.text, color);
                            }
                            None => {
                                ui.label(egui::RichText::new(&line.text).color(color));
                            }
                        }
                    }
                });
                });
                self.scroll_to_end = false;
                let input_id = egui::Id::unique("chat-input");
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
                // Up and Down walk the lines sent before when no command list is open.
                if has_focus && suggestions.is_empty() {
                    let (older, newer) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp), i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)));
                    if older || newer {
                        if let Some(line) = self.recall(older) {
                            self.input = line;
                            if let Some(mut state) = egui::TextEdit::load_state(ctx, input_id) {
                                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(self.input.chars().count()))));
                                state.store(ctx, input_id);
                            }
                        }
                    }
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
                    egui::Area::new(egui::Id::unique("chat-commands")).order(egui::Order::Foreground).fixed_pos(egui::pos2(response.rect.left(), response.rect.top() - 4.0 - rows as f32 * 15.0)).show(ctx, |ui| {
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
        assert_eq!(w.run("/afk"), None);
        assert!(w.lines.last().unwrap().text.contains("not carried out"));
        assert_eq!(w.run("hello"), None);
        assert!(w.lines.last().unwrap().text.contains("not available"));
    }

    #[test]
    fn an_ignored_character_is_not_shown_and_the_command_toggles() {
        let mut w = ChatWindow::default();
        assert_eq!(w.run("/ignore Zed"), None);
        w.push(Line { kind: Kind::Tell, text: "Zed tells you: hi".into(), from: Some("zed".into()) });
        w.push(Line { kind: Kind::Channel, text: "[OOC] Zed: yo".into(), from: Some("Zed".into()) });
        w.push(Line::new(Kind::System, "still shown"));
        let shown: Vec<&str> = w.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(shown, vec!["Zed is ignored: their tells and channel lines are hidden.", "still shown"]);
        assert_eq!(w.run("/ignore zed"), None);
        w.push(Line { kind: Kind::Tell, text: "Zed tells you: back".into(), from: Some("Zed".into()) });
        assert!(w.lines.last().unwrap().text.contains("back"));
        assert_eq!(w.run("/camp"), Some(Out::Camp));
        assert_eq!(w.run("/quit"), Some(Out::Quit));
    }

    #[test]
    fn the_arrow_keys_walk_the_lines_sent_before() {
        let mut w = ChatWindow::default();
        assert_eq!(w.recall(true), None);
        w.run("/help");
        w.run("hello");
        w.run("hello");
        assert_eq!(w.history, vec!["/help", "hello"]);
        assert_eq!(w.recall(true).as_deref(), Some("hello"));
        assert_eq!(w.recall(true).as_deref(), Some("/help"));
        assert_eq!(w.recall(true).as_deref(), Some("/help"));
        assert_eq!(w.recall(false).as_deref(), Some("hello"));
        assert_eq!(w.recall(false).as_deref(), Some(""));
        assert_eq!(w.recall(false), None);
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
