//! The chat windows: the lines that arrive (channels, tells, system notices) in tabs, and an input line. The first
//! frame of chat windows (`chatgroups`) is the window with the input line; each other frame is a window of its own with
//! its tabs and lines (the original gives every chat window its own input bar, which it can turn off). A tab is moved
//! to a window of its own, or into another, from its menu (the original drags tabs); closing such a window puts its
//! tabs back into the first. Typing a `/` opens the
//! original's list of commands under the cursor (filtered as you type, Up and Down to pick, Tab or Enter to take it); a line is
//! sent with Enter. Enter and `/` outside the window start typing (the `chat_input` and `chat_command` keys).

use crate::chat_commands::{self, Parsed};
use crate::chatgroups::{self, Pane};
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
    /// The line as the server sent it, markup and all (`<font color=...>`, `<a href="itemref://...">`), when it has
    /// some: drawn as GUI.dll's text views draw it (`gui::controls::markup`).
    pub markup: Option<String>,
    /// The chat group the line belongs to, by the original's name (`chatgroups`); empty for the kind's own
    /// ([`Line::group`]).
    pub group: String,
    /// When it arrived (the hour and minute, local), for a window that shows the times.
    pub at: Option<(u8, u8)>,
}

impl Line {
    pub fn new(kind: Kind, text: impl Into<String>) -> Line {
        // (a fight line is of the group its feedback text names, `feedback::take_group`; the group is taken either way so
        // it cannot be left for a later line)
        let told = crate::feedback::take_group().and_then(chatgroups::name_of);
        let group = if kind == Kind::Combat { told.unwrap_or(chatgroups::SYSTEM).to_string() } else { String::new() };
        Line { kind, text: text.into(), from: None, markup: None, group, at: None }
    }

    /// The line put in a chat group.
    pub fn in_group(mut self, group: &str) -> Line {
        self.group = group.to_string();
        self
    }

    /// The line's chat group: its own, else tells for a tell and its echo, and the system's for the rest.
    pub fn group(&self) -> &str {
        match self.kind {
            _ if !self.group.is_empty() => &self.group,
            Kind::Tell | Kind::Own => chatgroups::TELLS,
            _ => chatgroups::SYSTEM,
        }
    }

    /// The line's markup: `prefix` (plain) before the server's text, kept only when the text has tags.
    pub fn with_markup(mut self, prefix: &str, raw: &str) -> Line {
        if raw.contains('<') {
            self.markup = Some(format!("{prefix}{raw}"));
        }
        self
    }
}

/// Something to send on the chat connection.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    Tell { name: String, text: String },
    /// `/voice`: a voice line to speak (its checks and text are the chat connection's, `voices::command`).
    Voice { line: String },
    /// Vicinity speech (0 say, 1 whisper, 2 shout, 3 emote), sent on the game connection.
    Vicinity { kind: u8, text: String },
    /// An emote: the social action's `AbstractAnimID`.
    Social { id: u32 },
    /// Put a character on the friends list / take one off.
    Befriend { name: String },
    Unfriend { id: u32 },
    /// `/duel`: challenge the target (0) or accept (1), reject (2), stop (3), propose a draw (4).
    Duel(u32),
    /// A line to a channel of the chat server, by its name (the window's output group).
    Channel { name: String, text: String },
    /// Invite a character to the player's private chat group (the friends window's "Invite").
    PrivateInvite { id: u32 },
    /// Join the private chat group of the character who invited, or decline the invitation.
    PrivateJoin { owner: u32 },
    PrivateDecline { owner: u32 },
    /// Leave a private chat group, by its name.
    PrivateLeave { name: String },
    /// Sit down to log out / leave the client (handled by the window, not sent to the chat server).
    Camp,
    /// Sit down or stand up (`/sit`, as X).
    Sit,
    /// Show or hide the performance overlay (`/perf`).
    Perf,
    /// A command to the pets (`/pet`): its `PetCommand` number.
    Pet(u32),
    /// A command the server carries out (`/stuck` and the like): its text.
    Server(String),
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

/// The menus of a chat window (FUN_100abb98 and the ones under it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Menu {
    Main,
    /// "Talk to Channel": the groups of the window that can be talked to.
    Talk,
    /// "Subscribe Channels": the kinds of groups (FUN_100ab758).
    Subscribe,
    /// The groups of a kind, each with its mark: public or combat.
    Groups { combat: bool },
    /// "Visual" (FUN_10097756 over FUN_1009a00d): the mode, how solid, the times, the input bar, fading, the layer.
    Visual,
    /// "Mode": normal or borderless.
    Mode,
    /// "Transparency": the original's two sliders, here steps of ten for a window not active and for an active one.
    Solid,
}

/// The id of a frame's input bar.
fn bar_id(frame: &str) -> egui::Id {
    egui::Id::unique(("chat-input", frame))
}

pub struct ChatWindow {
    pub open: bool,
    /// Whether every line is also written to the chat log (the live client turns it on).
    pub log: bool,
    lines: Vec<Line>,
    /// The original's chat windows, here the tabs (`chatgroups`), and the character they were loaded for.
    panes: Vec<Pane>,
    loaded_for: Option<Option<u32>>,
    tab: usize,
    /// The window shown in each other frame, and the window the open menu is about.
    shown: std::collections::BTreeMap<String, usize>,
    menu_pane: usize,
    /// What is typed in the other frames' input bars (the first frame's is `input`), and the frame whose bar takes
    /// the keys next (a press on a window that hides its bar).
    inputs: std::collections::BTreeMap<String, String>,
    focus_frame: Option<String>,
    /// When each of `lines` came, for the fading.
    born: Vec<std::time::Instant>,
    /// `ChatTextFadeDelay` and `ChatTextFadeTime`: how old a line is when it starts to fade, and how long it takes.
    pub fade_times: (f32, f32),
    /// The menu open (its kind, what each entry stands for and where it opened) and its picture.
    menu: Option<(Menu, Vec<String>, egui::Pos2)>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// The chat configuration window (`chatconfig`).
    pub config: crate::chatconfig::ChatConfig,
    /// What to do with an invitation to a private chat group (`ChatPGInviteAction`: 0 decline, else ask), the
    /// invitations waiting for an answer, and what the window sends besides the typed line.
    pub invite_action: i32,
    /// `ChatWarnWhenSpeakingToUnsubGroups`: a line sent to a group the window does not show is warned of.
    pub warn_unshown: bool,
    invites: Vec<(u32, String)>,
    pub outs: Vec<Out>,
    /// A preview's: the tab to show and the menu to have open (main, talk, public or combat).
    pub demo: Option<(usize, String)>,
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
    /// The links clicked in the lines' markup (`itemref://`, `text://`, `chatcmd://`), for the info view.
    pub links: Vec<String>,
}

impl Default for ChatWindow {
    fn default() -> Self {
        ChatWindow { open: true, log: false, lines: Vec::new(), panes: chatgroups::defaults(), loaded_for: None, tab: 0, shown: Default::default(), menu_pane: 0, inputs: Default::default(), focus_frame: None, born: Vec::new(), fade_times: (8.0, 0.3), menu: None, popup: None, config: Default::default(), invite_action: 1, warn_unshown: true, invites: Vec::new(), outs: Vec::new(), demo: None, input: String::new(), selected: 0, focus_next: false, last_tell: None, scroll_to_end: true, keep_list: false, ignored: load_ignored(), history: Vec::new(), recalled: None, links: Vec::new() }
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

/// The lines of a chat window's selected groups, in a scroll view `height` tall that stays at its end.
/// With `fade` (the delay and the time, in seconds) a line older than the delay is drawn fainter, to nothing once
/// the time is over; `born` is when each line came.
#[allow(clippy::too_many_arguments)]
fn draw_lines(ui: &mut egui::Ui, palette: &Palette, lines: &[Line], born: &[std::time::Instant], pane: Option<&Pane>, height: f32, fade: Option<(f32, f32)>, links: &mut Vec<String>) {
    let now = std::time::Instant::now();
    let text = |name: &str, (r, g, b): (u8, u8, u8)| palette.text(name).map_or(egui::Color32::from_rgb(r, g, b), |c| egui::Color32::from_rgb(c[0], c[1], c[2]));
    let chat_font = if crate::font::enabled() { crate::font::chat(ui.ctx()) } else { None };
    let stamps = pane.is_some_and(|p| p.timestamps);
    crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(height).min_scrolled_height(height).auto_shrink([false, false]).stick_to_bottom(true), |ui| {
        for (k, line) in lines.iter().enumerate().filter(|(_, l)| pane.is_none_or(|p| p.selected.contains(l.group()))) {
            let faint = fade.zip(born.get(k)).map_or(1.0, |((delay, time), came)| (1.0 - (now.duration_since(*came).as_secs_f32() - delay) / time.max(0.01)).clamp(0.0, 1.0));
            // (set for each line: the setting stays on the view for what is drawn after)
            if fade.is_some() {
                ui.set_opacity(faint);
            }
            // (`show_timestamps`: the line led by its time, strftime's `(%H:%M) `)
            let stamp = line.at.filter(|_| stamps).map(|(h, m)| format!("({h:02}:{m:02}) ")).unwrap_or_default();
            let color = match line.kind {
                // (the original's TextColors.xml names when the theme has them)
                Kind::System => text("ct_cmd_feedback", (255, 220, 90)),
                Kind::Tell => text("ct_itell", (120, 220, 255)),
                // (a channel's lines in its group's colour, FUN_1008582a)
                Kind::Channel => text(chatgroups::colour_name(line.group()), (200, 235, 235)),
                Kind::Own => text("ct_otell", (160, 200, 160)),
                Kind::Combat => text("CCMeHitOtherColor", (240, 170, 130)),
            };
            if let Some(m) = &line.markup {
                let font = if chat_font.is_some() { crate::gui::font::FontId::named("CHAT") } else { crate::gui::font::FontId::NORMAL };
                if let Some(link) = crate::gui::controls::markup(ui, palette, &format!("{stamp}{m}"), font, color) {
                    links.push(link);
                }
                continue;
            }
            let shown = format!("{stamp}{}", line.text);
            match &chat_font {
                Some(font) => {
                    font.label(ui, &shown, color);
                }
                None => {
                    ui.label(egui::RichText::new(&shown).color(color));
                }
            }
        }
    });
}

/// A text the chat commands answer with (text.mdb 10001 by name), or this client's own words.
fn command_text(name: &str) -> String {
    let own = match name {
        "ChatCmdFeedback_TalkToUnsubscribedChannel" => "This window does not show that group: the answers will not be seen here.",
        "ChatCmdFeedback_Ch_CanOnlySelectSubscribedGroups" => "Only a group this window shows can be chosen.",
        "ChatCmdFeedback_GroupIsReadOnly" => "The group '%s' takes no lines.",
        "ChatCmdFeedback_AmbiguousGroupName" => "More than one group begins so: %s. Type more of the name after %s (typed: %s).",
        "ChatCmdFeedback_Ch_NoMatchingGroupAreSelectable" => "None of the groups that begin so can be chosen here.",
        "ChatCmdFeedback_AllMatchingGroupsAreReadOnly" => "None of the groups that begin so takes lines.",
        "ChatCmdFeedback_O_YouAreNotMemberOfAnOrganizaion" => "You are not in an organization.",
        "ChatCmdFeedback_T_YouAreNotMemberOfATeam" => "You are not in a team.",
        _ => name,
    };
    crate::gui::xml::ldb_named(10001, name, own)
}

/// A text of the chat window's menus (text.mdb 10001 by name), or this client's own words.
fn menu_text(name: &str) -> String {
    let own = match name {
        "ChatWindowMenu_TalkToChannel" => "Talk to a channel",
        "ChatWindowMenu_ChannelSubscribeMenu" => "Channels shown",
        "ChatWindowMenu_AutoSubscribeChannels" => "Show new channels by itself",
        "ShowTimestamps" => "Show the times",
        "NewWindow" => "New window",
        "DeleteWindow" => "Delete this window",
        "NoSelectableGroups" => "No groups to choose",
        "ChatWindowMenu_ChatConfiguration" => "Chat settings",
        "ChatWindowMenu_Visual" => "Looks",
        "ChatWindowMenu_Style_Mode" => "Kind of window",
        "ChatWindowMenu_Style_Mode_Normal" => "With frame and title",
        "ChatWindowMenu_Style_Mode_Borderless" => "Without frame",
        "ChatWindowMenu_Style_Transparancy" => "How solid",
        "ChatWindowMenu_Style_Transparancy_InactiveToolTip" => "While the window is not in use",
        "ChatWindowMenu_Style_Transparancy_ActiveToolTip" => "While the window is in use",
        "DisableTextInput" => "No input line",
        "ChatWindowMenu_Style_HideInputBarWhenInactive" => "Input line only while typing",
        "ChatWindowMenu_Style_FadeMessages" => "Lines fade away",
        "ChatWindowMenu_Style_AlwaysBehind" => "Always behind the other windows",
        "ChatWindowMenu_Style_AlwaysOnTop" => "Always over the other windows",
        "RenameWindow" => "Rename this window",
        "own:detach" => "Move to a window of its own",
        "own:attach" => "Move into the window of %s",
        _ => name,
    };
    crate::gui::xml::ldb_named(10001, name, own)
}

impl ChatWindow {
    /// The windows of the character playing, loaded once it is known (and again for another character); the lines
    /// already here are met by them.
    fn load_panes(&mut self) {
        let character = crate::layout::character();
        if self.loaded_for == Some(character) {
            return;
        }
        self.loaded_for = Some(character);
        let prefs = character.and_then(crate::aoprefs::prefs_dirs).map(|d| d.1);
        self.panes = chatgroups::load(prefs.as_deref());
        self.tab = self.tab.min(self.panes.len().saturating_sub(1));
        let groups: std::collections::BTreeSet<String> = self.lines.iter().map(|l| l.group().to_string()).collect();
        for g in groups {
            self.meet(&g);
        }
    }

    /// A group turned up: each window that selects new groups by itself selects it.
    fn meet(&mut self, group: &str) {
        let before: Vec<usize> = self.panes.iter().map(|p| p.known.len()).collect();
        for p in &mut self.panes {
            p.shows(group);
        }
        if self.loaded_for.is_some() && self.panes.iter().zip(before).any(|(p, n)| p.known.len() != n) {
            chatgroups::save(&self.panes);
        }
    }

    /// The invitations that came (FUN_100a7986): one from an ignored character is declined without a word; with the
    /// option at 0 it is declined and said so; else it is told of and waits for the player's answer.
    fn take_invitations(&mut self) {
        for (owner, name) in chatgroups::take_invites() {
            let said = |key: &str, own: &str| crate::gui::xml::ldb_named(10001, key, own).replace("%s", &name);
            if self.is_ignored(&name) {
                self.outs.push(Out::PrivateDecline { owner });
            } else if self.invite_action == 0 {
                self.push(Line::new(Kind::System, said("ChatInviteAutoDeclined", "The invitation to %s's private chat group was declined.")));
                self.outs.push(Out::PrivateDecline { owner });
            } else {
                self.push(Line::new(Kind::System, said("ChatInvitePending", "%s invites you to a private chat group.")));
                self.invites.push((owner, name));
            }
        }
    }

    /// The group plain typed lines go to: the shown window's output group, when it is a channel the chat server has
    /// told of this session (a window taken over from another character may name one this character is not in).
    fn output_channel(&self, pane: usize) -> Option<&str> {
        let output = self.panes.get(pane)?.output.as_str();
        let known = cfg!(test) || chatgroups::server_groups().iter().any(|g| g == output);
        (!output.is_empty() && output != chatgroups::VICINITY && chatgroups::talkable(output) && known).then_some(output)
    }

    /// Opens one of the window's menus at a place.
    fn open_menu(&mut self, ctx: &egui::Context, menu: Menu, at: egui::Pos2) {
        let Some(pane) = self.panes.get(self.menu_pane) else { return };
        let mut groups: std::collections::BTreeSet<String> = chatgroups::BUILTIN.iter().map(|g| g.1.to_string()).collect();
        groups.extend(chatgroups::server_groups());
        groups.extend(pane.known.iter().cloned());
        let (keys, entries): (Vec<String>, Vec<(String, bool, Option<bool>)>) = match menu {
            // (FUN_10097a40 puts "Visual" over FUN_100abb98's entries; the configuration's is marked while its window
            // is up)
            Menu::Main => [
                ("visual", menu_text("ChatWindowMenu_Visual"), true, None),
                ("talk", menu_text("ChatWindowMenu_TalkToChannel"), true, None),
                ("subscribe", menu_text("ChatWindowMenu_ChannelSubscribeMenu"), true, None),
                ("auto", menu_text("ChatWindowMenu_AutoSubscribeChannels"), true, Some(pane.auto)),
                ("config", menu_text("ChatWindowMenu_ChatConfiguration"), true, Some(self.config.open)),
                ("rename", menu_text("RenameWindow"), true, None),
                ("delete", menu_text("DeleteWindow"), self.panes.len() > 1, None),
                ("new", menu_text("NewWindow"), true, None),
            ]
            .into_iter()
            .map(|(k, label, enabled, mark)| (k.to_string(), (label, enabled, mark)))
            // (this client's own, in place of dragging the tab: out into a window of its own when it shares its frame,
            // or into another frame, named by its first window)
            .chain((chatgroups::members(&self.panes, &pane.frame).len() > 1).then(|| ("detach".to_string(), (menu_text("own:detach"), true, None))))
            .chain(chatgroups::frames(&self.panes).into_iter().filter(|f| *f != pane.frame).map(|f| {
                let first = self.panes.iter().find(|p| p.frame == f).map_or(f.clone(), |p| p.name.clone());
                (format!("attach:{f}"), (menu_text("own:attach").replace("%s", &first), true, None))
            }))
            .unzip(),
            Menu::Visual => {
                let look = pane.look;
                // (FUN_10099a56: the mode is for a window that has its frame to itself)
                let alone = chatgroups::members(&self.panes, &pane.frame).len() == 1;
                [
                    ("mode", menu_text("ChatWindowMenu_Style_Mode"), alone, None),
                    ("solid", menu_text("ChatWindowMenu_Style_Transparancy"), true, None),
                    ("timestamps", menu_text("ShowTimestamps"), true, Some(pane.timestamps)),
                    ("noinput", menu_text("DisableTextInput"), true, Some(!look.input)),
                    ("hideinput", menu_text("ChatWindowMenu_Style_HideInputBarWhenInactive"), look.input, Some(look.hide_input)),
                    ("fade", menu_text("ChatWindowMenu_Style_FadeMessages"), true, Some(look.fade)),
                    ("behind", menu_text("ChatWindowMenu_Style_AlwaysBehind"), true, Some(look.layer < 0)),
                    ("top", menu_text("ChatWindowMenu_Style_AlwaysOnTop"), true, Some(look.layer > 0)),
                ]
                .into_iter()
                // (how solid: only a borderless window's)
                .filter(|e| e.0 != "solid" || look.borderless)
                .map(|(k, label, enabled, mark)| (k.to_string(), (label, enabled, mark)))
                .unzip()
            }
            Menu::Mode => [("normal", "ChatWindowMenu_Style_Mode_Normal", !pane.look.borderless), ("borderless", "ChatWindowMenu_Style_Mode_Borderless", pane.look.borderless)]
                .into_iter()
                .map(|(k, label, mark)| (k.to_string(), (menu_text(label), true, Some(mark))))
                .unzip(),
            Menu::Solid => {
                let mut rows = Vec::new();
                for (key, label, now) in [("inactive", "ChatWindowMenu_Style_Transparancy_InactiveToolTip", pane.look.solid.1), ("active", "ChatWindowMenu_Style_Transparancy_ActiveToolTip", pane.look.solid.0)] {
                    rows.push((String::new(), (menu_text(label), false, None)));
                    for step in (0..=100u8).step_by(10) {
                        rows.push((format!("{key}:{step}"), (format!("{step}%"), true, Some((now + 5) / 10 * 10 == step))));
                    }
                }
                rows.into_iter().unzip()
            }
            Menu::Subscribe => [("public", "Public Groups"), ("combat", "Combat Messages")].into_iter().map(|(k, label)| (k.to_string(), (label.to_string(), true, None))).unzip(),
            Menu::Talk => {
                let output = if pane.output.is_empty() { chatgroups::VICINITY } else { pane.output.as_str() };
                let list: Vec<&String> = groups.iter().filter(|g| pane.selected.contains(*g) && chatgroups::talkable(g)).collect();
                if list.is_empty() {
                    (vec![String::new()], vec![(menu_text("NoSelectableGroups"), false, None)])
                } else {
                    list.into_iter().map(|g| (g.clone(), (g.clone(), true, Some(g == output)))).unzip()
                }
            }
            Menu::Groups { combat } => groups.iter().filter(|g| chatgroups::is_combat(g) == combat).map(|g| (g.clone(), (g.clone(), true, Some(pane.selected.contains(g))))).unzip(),
        };
        self.popup = Some(crate::chrome::menu_marked(ctx, at, &entries));
        self.menu = Some((menu, keys, at));
    }

    /// An entry of the open menu was chosen.
    fn choose(&mut self, ctx: &egui::Context, menu: Menu, key: &str, at: egui::Pos2) {
        let tab = self.menu_pane.min(self.panes.len().saturating_sub(1));
        match (menu, key) {
            (Menu::Main, "talk") => self.open_menu(ctx, Menu::Talk, at),
            (Menu::Main, "subscribe") => self.open_menu(ctx, Menu::Subscribe, at),
            (Menu::Subscribe, kind) => self.open_menu(ctx, Menu::Groups { combat: kind == "combat" }, at),
            (Menu::Main, "config") => self.config.open = !self.config.open,
            (Menu::Main, "visual") => self.open_menu(ctx, Menu::Visual, at),
            (Menu::Visual, "mode") => self.open_menu(ctx, Menu::Mode, at),
            (Menu::Visual, "solid") => self.open_menu(ctx, Menu::Solid, at),
            (Menu::Main, "rename") => self.config.rename(tab, &self.panes),
            // (always behind and always on top are the frame's: every window in it)
            (Menu::Visual, "behind" | "top") => {
                let want = if key == "top" { 1 } else { -1 };
                if let Some((frame, now)) = self.panes.get(tab).map(|p| (p.frame.clone(), p.look.layer)) {
                    for pane in self.panes.iter_mut().filter(|p| p.frame == frame) {
                        pane.look.layer = if now == want { 0 } else { want };
                    }
                }
            }
            (Menu::Main, "new") => {
                // (FUN_10094701: the first `Window%d` not taken, in a frame of its own)
                let mut pane = chatgroups::defaults().remove(0);
                pane.name = chatgroups::free_name(&self.panes);
                pane.frame = pane.name.clone();
                pane.selected.clear();
                pane.auto = false;
                self.panes.push(pane);
            }
            (Menu::Main, "delete") if self.panes.len() > 1 => {
                self.panes.remove(tab);
                self.tab = self.tab.min(self.panes.len() - 1);
            }
            (Menu::Main, "detach") => {
                let frame = chatgroups::free_name(&self.panes);
                if let Some(pane) = self.panes.get_mut(tab) {
                    pane.frame = frame;
                }
            }
            (Menu::Main, key) if key.starts_with("attach:") => {
                if let Some(pane) = self.panes.get_mut(tab) {
                    pane.frame = key["attach:".len()..].to_string();
                }
            }
            (_, _) => {
                let Some(pane) = self.panes.get_mut(tab) else { return };
                match (menu, key) {
                    (Menu::Main, "auto") => pane.auto = !pane.auto,
                    (Menu::Visual, "timestamps") => pane.timestamps = !pane.timestamps,
                    (Menu::Visual, "noinput") => pane.look.input = !pane.look.input,
                    (Menu::Visual, "hideinput") => pane.look.hide_input = !pane.look.hide_input,
                    (Menu::Visual, "fade") => pane.look.fade = !pane.look.fade,
                    (Menu::Mode, mode) => pane.look.borderless = mode == "borderless",
                    (Menu::Solid, step) => {
                        if let Some((which, value)) = step.split_once(':').and_then(|(w, v)| Some((w, v.parse::<u8>().ok()?))) {
                            if which == "active" { pane.look.solid.0 = value } else { pane.look.solid.1 = value }
                        }
                        // (the list stays up, as a slider would)
                        self.open_menu(ctx, menu, at);
                    }
                    (Menu::Talk, group) if !group.is_empty() => pane.output = group.to_string(),
                    (Menu::Groups { .. }, group) => {
                        pane.toggle(group);
                        // (the list stays up for the next one)
                        self.open_menu(ctx, menu, at);
                    }
                    _ => {}
                }
            }
        }
        chatgroups::save(&self.panes);
    }

    pub fn push(&mut self, mut line: Line) {
        // (what an ignored character says is not shown)
        if matches!(line.kind, Kind::Tell | Kind::Channel) && line.from.as_ref().is_some_and(|f| self.ignored.contains(&f.to_lowercase())) {
            return;
        }
        if line.at.is_none() {
            line.at = time::OffsetDateTime::now_local().ok().map(|t| (t.hour(), t.minute()));
        }
        let group = line.group().to_string();
        self.meet(&group);
        if self.log {
            crate::chatlog::append(&line);
        }
        if line.kind == Kind::Tell {
            if let Some(from) = &line.from {
                self.last_tell = Some(from.clone());
            }
        }
        self.lines.push(line);
        self.born.push(std::time::Instant::now());
        if self.lines.len() > KEEP {
            self.lines.drain(..self.lines.len() - KEEP);
        }
        if self.born.len() > self.lines.len() {
            self.born.drain(..self.born.len() - self.lines.len());
        }
        self.scroll_to_end = true;
    }

    /// Whether what a character says is hidden (`/ignore`).
    pub fn is_ignored(&self, name: &str) -> bool {
        self.ignored.contains(&name.to_lowercase())
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
        self.run_in(self.tab, line)
    }

    /// A line of the game's or this client's about a command, with its markup.
    fn tell(&mut self, text: &str) {
        let text = text.replace("\r\n", "<br>");
        self.push(Line::new(Kind::System, mao_net::chat::plain(&text)).with_markup("", &text));
    }

    /// A line to a group (FUN_10085a42): Vicinity's is spoken, a channel's sent to it. With the option on, a group
    /// the window does not show is warned of.
    fn say_in(&mut self, pane: usize, group: &str, text: String) -> Option<Out> {
        if self.warn_unshown && !self.panes.get(pane).is_some_and(|p| p.selected.contains(group)) {
            self.tell(&command_text("ChatCmdFeedback_TalkToUnsubscribedChannel"));
        }
        Some(if group == chatgroups::VICINITY { Out::Vicinity { kind: 0, text } } else { Out::Channel { name: group.to_string(), text } })
    }

    /// `/ch <name>` (no `text`: the group becomes the window's output group, FUN_1009c5b8) and `/group <name> <text>`
    /// (the line goes to the group, FUN_1009c977).
    fn group_command(&mut self, pane: usize, word: &str, name: &str, text: Option<String>) -> Option<Out> {
        use chatgroups::Pick;
        use mao_formats::ldb::{Arg, format as fill};
        let matches = chatgroups::matching(name);
        let shown = self.panes.get(pane).map(|p| p.selected.clone()).unwrap_or_default();
        let shows = |g: &str| shown.contains(g);
        match chatgroups::pick(&matches, text.is_none(), &shows, &chatgroups::read_only) {
            Pick::Unknown => self.tell(&format!("There is no chat group named '{}'.", name.trim_matches('"'))),
            Pick::NotShown => self.tell(&command_text("ChatCmdFeedback_Ch_CanOnlySelectSubscribedGroups")),
            Pick::ReadOnly(group) => self.tell(&fill(&command_text("ChatCmdFeedback_GroupIsReadOnly"), &[Arg::Text(&group)])),
            Pick::Ambiguous => {
                let list = chatgroups::ambiguous_list(&matches, &shows, &chatgroups::read_only);
                self.tell(&fill(&command_text("ChatCmdFeedback_AmbiguousGroupName"), &[Arg::Text(&list), Arg::Text(word), Arg::Text(name)]));
            }
            Pick::NoneUsable => self.tell(&command_text(if text.is_none() { "ChatCmdFeedback_Ch_NoMatchingGroupAreSelectable" } else { "ChatCmdFeedback_AllMatchingGroupsAreReadOnly" })),
            Pick::Group(group) => match text {
                Some(text) => return self.say_in(pane, &group, text),
                None => {
                    if let Some(p) = self.panes.get_mut(pane) {
                        p.output = group;
                    }
                    chatgroups::save(&self.panes);
                }
            },
        }
        None
    }

    /// Carries out a line typed in one of the chat windows.
    pub fn run_in(&mut self, pane: usize, line: &str) -> Option<Out> {
        self.remember(line);
        // (a plain line goes to the window's output group when that is a channel; the vicinity's goes on as speech)
        if !line.trim().is_empty()
            && !line.trim_start().starts_with('/')
            && let Some(name) = self.output_channel(pane)
        {
            return Some(Out::Channel { name: name.to_string(), text: line.trim().to_string() });
        }
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
            Parsed::Camp => Some(Out::Camp),
            Parsed::Sit => Some(Out::Sit),
            Parsed::Pet { command } => Some(Out::Pet(command)),
            Parsed::Perf => Some(Out::Perf),
            Parsed::Server { text } => Some(Out::Server(text)),
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
            // (FUN_1009cdfe, FUN_1009d028: the first channel of the organization's kind, of the team's)
            Parsed::Org { text } => match chatgroups::of_kind(3) {
                Some(group) => self.say_in(pane, &group, text),
                None => {
                    self.tell(&command_text("ChatCmdFeedback_O_YouAreNotMemberOfAnOrganizaion"));
                    None
                }
            },
            Parsed::Team { text } => match chatgroups::of_kind(0x82) {
                Some(group) => self.say_in(pane, &group, text),
                None => {
                    self.tell(&command_text("ChatCmdFeedback_T_YouAreNotMemberOfATeam"));
                    None
                }
            },
            Parsed::Duel { what } => Some(Out::Duel(what.unwrap_or(0))),
            Parsed::Output { word, name } => self.group_command(pane, &word, &name, None),
            Parsed::Group { word, name, text } => self.group_command(pane, &word, &name, Some(text)),
            Parsed::Vicinity { command, text } => Some(Out::Vicinity { kind: match command {
                "whisper" => 1,
                "shout" => 2,
                "emote" => 3,
                _ => 0,
            }, text }),
            Parsed::Social { id } => Some(Out::Social { id }),
            Parsed::NoEmote(name) => {
                self.push(Line::new(Kind::System, format!("Error: No emote named '{name}'")));
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

    /// The input bar of a frame of chat windows: the line typed (the first frame's is `input`, another's is kept by
    /// its frame), the list of commands over it, the lines sent before. What is typed goes to window `pane`.
    fn input_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, frame: &str, main: bool, pane: usize) -> Option<Out> {
        let mut out = None;
        // (another frame's line stands in `input` while its bar is drawn)
        let mut own = (!main).then(|| self.inputs.remove(frame).unwrap_or_default());
        if let Some(own) = own.as_mut() {
            std::mem::swap(&mut self.input, own);
        }
        let input_id = bar_id(frame);
        if (main && self.focus_next) || self.focus_frame.as_deref() == Some(frame) {
            ctx.memory_mut(|m| m.request_focus(input_id));
            self.focus_next &= !main;
            self.focus_frame = None;
            // (the cursor after what is already there: `/` starts the line with its slash)
            let mut state = egui::TextEdit::load_state(ctx, input_id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(self.input.chars().count()))));
            state.store(ctx, input_id);
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
            // (the line sent, the keys are the game's again: Enter starts the next one)
            let line = std::mem::take(&mut self.input);
            out = self.run_in(pane, &line);
            response.surrender_focus();
        }
        // The list of commands, above the input line, as the original's.
        if (has_focus || response.has_focus() || self.keep_list) && !suggestions.is_empty() {
            // as many rows as fit above the input line, scrolling to keep the highlighted one in view
            let fits = (((response.rect.top() - 14.0) / 15.0).max(3.0)) as usize;
            let rows = suggestions.len().min(fits);
            let first = if self.selected >= rows { self.selected + 1 - rows } else { 0 };
            egui::Area::new(input_id.with("commands")).order(egui::Order::Foreground).fixed_pos(egui::pos2(response.rect.left(), response.rect.top() - 4.0 - rows as f32 * 15.0)).show(ctx, |ui| {
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
        if let Some(mut own) = own {
            std::mem::swap(&mut self.input, &mut own);
            self.inputs.insert(frame.to_string(), own);
        }
        out
    }

    /// Whether a frame's window is active: typed in, about to be, or under the pointer.
    fn frame_active(&self, ctx: &egui::Context, name: &str, frame: &str, main: bool) -> bool {
        let typing = ctx.memory(|m| m.has_focus(bar_id(frame))) || (main && self.focus_next) || self.focus_frame.as_deref() == Some(frame);
        typing || ctx.memory(|m| m.area_rect(egui::Id::unique(name))).zip(ctx.input(|i| i.pointer.hover_pos())).is_some_and(|(rect, at)| rect.contains(at))
    }

    /// A frame of chat windows inside its window on screen: the tabs (none when `bare`), the shown window's lines and
    /// its input bar. `n` is the frame's number, 0 the chat window itself.
    #[allow(clippy::too_many_arguments)]
    fn frame_body(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, palette: &Palette, n: usize, frame: &str, members: &[usize], bare: bool, links: &mut Vec<String>, menu_at: &mut Option<(egui::Pos2, usize)>) -> Option<Out> {
        let main = n == 0;
        let Some(&lead) = members.first() else { return None };
        let mut shown = if main { self.tab } else { self.shown.get(frame).copied().unwrap_or(lead) };
        if !members.contains(&shown) {
            shown = lead;
        }
        let area = ui.max_rect();
        let name = if main { "Chat".to_string() } else { format!("Chat:{frame}") };
        let active = self.frame_active(ctx, &name, frame, main);
        let typing = ctx.memory(|m| m.has_focus(bar_id(frame))) || (main && self.focus_next) || self.focus_frame.as_deref() == Some(frame);
        let look = self.panes.get(shown).map(|p| p.look).unwrap_or_default();
        // (the input bar: none with `is_textinput_enabled` off, and with `hide_input_when_inactive` only while typing;
        // Enter still starts a line in the chat window itself)
        let bar = (look.input && (!look.hide_input || typing)) || (main && self.focus_next);
        // (`is_message_fading_enabled`: while the window is not active a line fades once it is `ChatTextFadeDelay` old)
        let fade = (look.fade && !active).then_some(self.fade_times);
        let below = if bar { 41.0 } else { 4.0 };
        if bare {
            let height = (ui.available_height() - below).max(40.0);
            draw_lines(ui, palette, &self.lines, &self.born, self.panes.get(shown), height, fade, links);
        } else {
            // (each tab one of the frame's chat windows: the lines of the groups selected in it)
            let names: Vec<String> = members.iter().map(|k| self.panes[*k].name.clone()).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let mut tab = members.iter().position(|k| *k == shown).unwrap_or(0);
            crate::chrome::tab_view(ui, palette, &names, &mut tab, |ui, tab| {
                shown = members.get(tab).copied().unwrap_or(lead);
                let height = (ui.available_height() - below).max(40.0);
                draw_lines(ui, palette, &self.lines, &self.born, self.panes.get(shown), height, fade, links);
            });
        }
        if main {
            self.tab = shown;
        } else {
            self.shown.insert(frame.to_string(), shown);
        }
        let pointer = ui.input(|i| i.pointer.interact_pos()).filter(|p| area.contains(*p));
        // (the right button over the window: its menu, FUN_10097a40)
        if ui.input(|i| i.pointer.secondary_clicked())
            && let Some(at) = pointer
        {
            *menu_at = Some((at, shown));
        }
        // (a press on a window whose bar hides: typing starts in it)
        if look.input && look.hide_input && !typing && pointer.is_some() && ui.input(|i| i.pointer.primary_clicked()) {
            self.focus_frame = Some(frame.to_string());
        }
        if bar { self.input_bar(ui, ctx, frame, main, shown) } else { None }
    }

    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<Out> {
        if !self.open {
            return None;
        }
        let mut out = None;
        let mut links = Vec::new();
        let screen = ctx.content_rect();
        let mut open = self.open;
        self.load_panes();
        // (a channel the chat server has told of is a group from then on, not only once a line comes in it: a window
        // that takes new channels by itself shows it as selected in its lists at once, as the original's does)
        for group in chatgroups::server_groups() {
            if self.panes.iter().any(|p| !p.known.contains(&group)) {
                self.meet(&group);
            }
        }
        let mut menu_at: Option<(egui::Pos2, usize)> = None;
        // each frame of chat windows is a window on screen with its tabs, the shown one's lines and its input bar: the
        // first the chat window itself; another, closed, puts its tabs back into the first
        let frames = chatgroups::frames(&self.panes);
        let first = frames.first().map(|f| chatgroups::members(&self.panes, f)).unwrap_or_default();
        if !first.contains(&self.tab) {
            self.tab = first.first().copied().unwrap_or(0);
        }
        let mut docked = false;
        for (n, frame) in frames.iter().enumerate() {
            let members = chatgroups::members(&self.panes, frame);
            let Some(&lead) = members.first() else { continue };
            let name = if n == 0 { "Chat".to_string() } else { format!("Chat:{frame}") };
            let look = self.panes[lead].look;
            // (`visual_mode` 2: a window alone in its frame has no frame or title, and a background as solid as its
            // setting for an active window or for one that is not)
            let bare = look.borderless && members.len() == 1;
            let (at, size) = if n == 0 { (egui::pos2(8.0, (screen.height() - 270.0).max(40.0)), [560.0, 230.0]) } else { (egui::pos2(60.0, 160.0), [420.0, 170.0]) };
            let order = match look.layer {
                1 => egui::Order::Foreground,
                -1 => egui::Order::Background,
                _ => egui::Order::Middle,
            };
            let place = crate::layout::pos(&name, at);
            let build = move |w: egui::Window<'static>| w.default_pos(place).default_size(size).order(order);
            let mut stays = true;
            let mut sent = None;
            if bare {
                let active = self.frame_active(ctx, &name, frame, n == 0);
                let solid = f32::from(if active { look.solid.0 } else { look.solid.1 }) / 100.0;
                crate::chrome::bare_window(ctx, palette, &name, solid, build, |ui| sent = self.frame_body(ui, ctx, palette, n, frame, &members, true, &mut links, &mut menu_at));
            } else {
                let open = if n == 0 { &mut open } else { &mut stays };
                crate::chrome::window(ctx, palette, &name, "Chat", open, build, |ui| sent = self.frame_body(ui, ctx, palette, n, frame, &members, false, &mut links, &mut menu_at));
            }
            out = out.or(sent);
            if !stays {
                for k in members {
                    self.panes[k].frame = frames[0].clone();
                }
                docked = true;
            }
        }
        self.scroll_to_end = false;
        if docked {
            chatgroups::save(&self.panes);
        }
        if let Some((at, pane)) = menu_at {
            self.menu_pane = pane;
            self.open_menu(ctx, Menu::Main, at);
        }
        if let Some((tab, menu)) = self.demo.take() {
            self.tab = tab.min(self.panes.len().saturating_sub(1));
            self.menu_pane = self.tab;
            // (`split`: the second window in a frame of its own)
            if menu == "split"
                && let Some(pane) = self.panes.get_mut(1)
            {
                pane.frame = "Window2".to_string();
            }
            // (`borderless`: the second window alone and without its frame; `visual`, `mode`, `solid`: those menus)
            if menu == "borderless"
                && let Some(pane) = self.panes.get_mut(1)
            {
                pane.frame = "Window2".to_string();
                pane.look.borderless = true;
            }
            if menu == "solid"
                && let Some(pane) = self.panes.get_mut(self.tab)
            {
                pane.look.borderless = true;
            }
            // (`config`: the chat configuration window up)
            self.config.open = menu == "config";
            let menu = match menu.as_str() {
                "main" => Some(Menu::Main),
                "talk" => Some(Menu::Talk),
                "public" => Some(Menu::Groups { combat: false }),
                "combat" => Some(Menu::Groups { combat: true }),
                "visual" => Some(Menu::Visual),
                "mode" => Some(Menu::Mode),
                "solid" => Some(Menu::Solid),
                _ => None,
            };
            if let Some(menu) = menu {
                self.open_menu(ctx, menu, egui::pos2(420.0, 120.0));
            }
        }
        if let Some(k) = crate::chrome::menu(ctx, palette, "chat-menu", &mut self.popup)
            && let Some((menu, keys, at)) = self.menu.take()
            && let Some(key) = keys.get(k)
        {
            self.choose(ctx, menu, key, at);
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        if self.config.show(ctx, palette, &mut self.panes, &mut self.tab, &mut self.log) {
            chatgroups::save(&self.panes);
        }
        for name in self.config.leaves.drain(..) {
            self.outs.push(Out::PrivateLeave { name });
        }
        // an invitation to a private chat group: the original's question (FUN_100a76c9, without its list of chat
        // windows to put the group in: the windows that take new channels by themselves show it)
        self.take_invitations();
        if let Some((owner, name)) = self.invites.first().cloned() {
            let title = crate::gui::xml::ldb_named(10001, "PrivateGroupInvitation", "Private chat group");
            let question = crate::gui::xml::ldb_named(10001, "ChatInvitePending", "%s invites you to a private chat group.").replace("%s", &format!("<font color=yellow>{name}</font>"));
            if let Some(yes) = crate::chrome::question(ctx, palette, "private-invite", &title, &question) {
                self.outs.push(if yes { Out::PrivateJoin { owner } } else { Out::PrivateDecline { owner } });
                self.invites.remove(0);
            }
        }
        self.open = open;
        self.links.extend(links);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_group_commands_find_their_groups() {
        chatgroups::note_channel("Yy Clan Chat", 3, 0x8004);
        chatgroups::note_channel("Yy Team", 0x82, 0x8004);
        chatgroups::note_channel("Yy Results", 10, 0x8006);
        let mut w = ChatWindow::default();
        // (`/ch`: only a group the window shows, and not one that takes no lines)
        assert_eq!((w.run("/ch \"yy clan chat\""), w.panes[0].output.as_str()), (None, ""));
        w.panes[0].selected.insert("Yy Clan Chat".into());
        w.panes[0].selected.insert("Yy Results".into());
        assert_eq!((w.run("/ch \"yy r\""), w.panes[0].output.as_str()), (None, ""));
        assert_eq!((w.run("/ch \"yy c\""), w.panes[0].output.as_str()), (None, "Yy Clan Chat"));
        // (a plain line then goes there; `/group` sends one line elsewhere and warns when the window does not show it)
        assert_eq!(w.run("hello"), Some(Out::Channel { name: "Yy Clan Chat".into(), text: "hello".into() }));
        let before = w.lines.len();
        assert_eq!(w.run("/g \"Yy Team\" over here"), Some(Out::Channel { name: "Yy Team".into(), text: "over here".into() }));
        assert_eq!(w.lines.len(), before + 1);
        w.warn_unshown = false;
        assert_eq!(w.run("/t on my way"), Some(Out::Channel { name: "Yy Team".into(), text: "on my way".into() }));
        assert_eq!(w.lines.len(), before + 1);
        assert!(matches!(w.run("/o hi all"), Some(Out::Channel { text, .. }) if text == "hi all"));
        // (the vicinity's group is spoken; a group nobody knows and one that takes no lines are told of)
        assert_eq!(w.run("/group vic hi"), Some(Out::Vicinity { kind: 0, text: "hi".into() }));
        assert_eq!((w.run("/g \"yy results\" x"), w.run("/g nosuchgroup x"), w.lines.len()), (None, None, before + 3));
    }

    #[test]
    fn a_tell_is_echoed_and_sent_and_a_reply_goes_to_the_last_sender() {
        let mut w = ChatWindow::default();
        assert_eq!(w.run("/tell Abc hello"), Some(Out::Tell { name: "Abc".into(), text: "hello".into() }));
        assert_eq!(w.lines.last().unwrap().text, "To Abc: hello");
        assert_eq!(w.run("/r anyone"), None); // nobody has told us anything yet
        w.push(Line { kind: Kind::Tell, text: "Zed: hi".into(), from: Some("Zed".into()), markup: None, group: String::new(), at: None });
        assert_eq!(w.run("/r hello Zed"), Some(Out::Tell { name: "Zed".into(), text: "hello Zed".into() }));
    }

    #[test]
    fn local_commands_only_add_lines() {
        let mut w = ChatWindow::default();
        assert_eq!(w.run("/help"), None);
        assert!(w.lines.len() >= 6);
        assert_eq!(w.run("/afk"), None);
        assert!(w.lines.last().unwrap().text.contains("not carried out"));
        assert_eq!(w.run("/me waves"), Some(Out::Vicinity { kind: 3, text: "waves".into() }));
        assert_eq!(w.run("/wave"), Some(Out::Social { id: 62 }));
        assert_eq!(w.run("/emote BOW"), Some(Out::Social { id: 9 }));
        assert_eq!(w.run("/emote moonwalk"), None);
        assert!(w.lines.last().unwrap().text.contains("No emote named 'moonwalk'"));
        // (vicinity speech goes out: say, shout, whisper)
        assert_eq!(w.run("hello"), Some(Out::Vicinity { kind: 0, text: "hello".into() }));
        assert_eq!(w.run("/s HEY"), Some(Out::Vicinity { kind: 2, text: "HEY".into() }));
        assert_eq!(w.run("/w psst"), Some(Out::Vicinity { kind: 1, text: "psst".into() }));
    }

    #[test]
    fn an_ignored_character_is_not_shown_and_the_command_toggles() {
        let mut w = ChatWindow::default();
        assert_eq!(w.run("/ignore Zed"), None);
        w.push(Line { kind: Kind::Tell, text: "Zed tells you: hi".into(), from: Some("zed".into()), markup: None, group: String::new(), at: None });
        w.push(Line { kind: Kind::Channel, text: "[OOC] Zed: yo".into(), from: Some("Zed".into()), markup: None, group: String::new(), at: None });
        w.push(Line::new(Kind::System, "still shown"));
        let shown: Vec<&str> = w.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(shown, vec!["Zed is ignored: their tells and channel lines are hidden.", "still shown"]);
        assert_eq!(w.run("/ignore zed"), None);
        w.push(Line { kind: Kind::Tell, text: "Zed tells you: back".into(), from: Some("Zed".into()), markup: None, group: String::new(), at: None });
        assert!(w.lines.last().unwrap().text.contains("back"));
        assert_eq!(w.run("/camp"), Some(Out::Camp));
        assert_eq!(w.run("/sit"), Some(Out::Sit));
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
    fn a_line_shows_in_the_windows_that_select_its_group() {
        let mut w = ChatWindow::default();
        w.push(Line::new(Kind::System, "notice"));
        w.push(Line::new(Kind::Channel, "[Clan OOC] Zed: hi").in_group("Clan OOC"));
        w.push(Line { kind: Kind::Tell, text: "Zed tells you: hi".into(), from: Some("Zed".into()), markup: None, group: String::new(), at: None });
        crate::feedback::take_group();
        w.push(Line::new(Kind::Combat, crate::feedback::executing("Lifeblood")));
        let shown = |w: &ChatWindow, tab: usize| w.lines.iter().filter(|l| w.panes[tab].selected.contains(l.group())).map(|l| l.group().to_string()).collect::<Vec<_>>();
        // (the default window: the system's lines, the channel that turned up, the tell; the combat window: the cast)
        assert_eq!(shown(&w, 0), ["System", "Clan OOC", "Tell Messages"]);
        assert_eq!(shown(&w, 1), ["Me Cast Nano"]);
        // (a plain line goes to the window's output group when that is a channel)
        assert_eq!(w.run("hello"), Some(Out::Vicinity { kind: 0, text: "hello".into() }));
        w.panes[0].output = "Clan OOC".into();
        assert_eq!(w.run("hello"), Some(Out::Channel { name: "Clan OOC".into(), text: "hello".into() }));
        assert_eq!(w.run("/s HEY"), Some(Out::Vicinity { kind: 2, text: "HEY".into() }));
        w.panes[0].output = "System".into();
        assert_eq!(w.run("hello"), Some(Out::Vicinity { kind: 0, text: "hello".into() }));
    }

    #[test]
    fn an_invitation_waits_is_declined_or_is_dropped() {
        // (asked about by default; declined and said so with the option at 0; from an ignored character declined
        // without a word)
        let mut w = ChatWindow::default();
        chatgroups::invite(71, "Inviterone");
        w.take_invitations();
        assert_eq!((w.invites.clone(), w.outs.clone()), (vec![(71, "Inviterone".to_string())], vec![]));
        assert!(w.lines.last().unwrap().text.contains("Inviterone"));
        w.invite_action = 0;
        chatgroups::invite(72, "Invitertwo");
        w.take_invitations();
        assert_eq!(w.outs, vec![Out::PrivateDecline { owner: 72 }]);
        assert!(w.lines.last().unwrap().text.contains("Invitertwo") && w.invites.len() == 1);
        w.run("/ignore Inviterthree");
        let said = w.lines.len();
        chatgroups::invite(73, "Inviterthree");
        w.take_invitations();
        assert_eq!((w.outs.last(), w.lines.len()), (Some(&Out::PrivateDecline { owner: 73 }), said));
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
