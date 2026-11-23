//! The chat windows: the lines that arrive (channels, tells, system notices) in tabs, and an input line. The first
//! frame of chat windows (`chatgroups`) is the window with the input line; each other frame is a window of its own with
//! its tabs and lines (the original gives every chat window its own input bar, which it can turn off). A tab is dragged:
//! taken up by the right button and put down by the left, on a tab strip or anywhere else for a window of its own
//! (`ChatWindow::carry`); its menu also moves it (this client's own); closing such a window puts its tabs back into the
//! first. Typing a `/` opens the
//! original's list of commands under the cursor (filtered as you type, Up and Down to pick, Tab or Enter to take it); a line is
//! sent with Enter. Enter and `/` outside the window start typing (the `chat_input` and `chat_command` keys).
//!
//! A character's name in a line is a link (`user://<name>`, as FUN_1009bc20 and FUN_100a681d write it). The chat
//! view's `MouseDown` (FUN_1008fa92) and its signals' handlers in `GroupChatView_c` (FUN_100ac65f): the left button on
//! a name opens the tell window for the character (FUN_100ac2fa), with Shift held it types the start of a tell
//! instead (FUN_100ac4c4, FUN_100ac42b); the right button opens a menu (FUN_1008e5ea: `IgnoreUser`, `OpenChat`,
//! `SendTell`). A tell window (`ChatGUIModule_c::OpenTellWindow` 0x10086323) is a window of the tells with one
//! character: what is typed in it is a tell to them; it holds the tells from before it was opened too (the original
//! keeps each character's, FUN_100a7394), and the chat windows that show the tells' group go on showing them.

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
    /// The automatic answer of a character that is away to a tell sent to it: shown with the tells, with no name in
    /// front, and never answered (`afk.rs`).
    AwayReply,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    pub text: String,
    /// The character a tell came from (for `/reply`), or the one an echoed tell went to: whose tell window shows it.
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
            Kind::Tell | Kind::Own | Kind::AwayReply => chatgroups::TELLS,
            _ => chatgroups::SYSTEM,
        }
    }

    /// The line's markup: `prefix` before the server's text, kept only when either has tags.
    pub fn with_markup(mut self, prefix: &str, raw: &str) -> Line {
        if raw.contains('<') || prefix.contains('<') {
            self.markup = Some(format!("{prefix}{raw}"));
        }
        self
    }
}

/// A character's name as the link the original's chat lines make of it (FUN_1009bc20's format; FUN_100a681d's pieces).
pub fn name_link(name: &str) -> String {
    format!("<a style=\"text-decoration:none\" href=\"user://{name}\">{name}</a>")
}

/// A channel's name as the link the original's chat lines put before the speaker (FUN_1009bc20's `chatgroup://` link,
/// for a group of the chat server's).
pub fn group_link(name: &str) -> String {
    format!("<a style=\"text-decoration:none\" href=\"chatgroup://{name}\">{name}</a>")
}

/// The group a `chatgroup://` link names.
fn linked_group(link: &str) -> Option<&str> {
    link.get(..12).filter(|p| p.eq_ignore_ascii_case("chatgroup://")).map(|_| &link[12..]).filter(|n| !n.is_empty())
}

/// The command a press on a group's link types when it does not pick the group to talk to (FUN_100ac4f0): `/group`
/// and the name, in quotes when it has a space in it, and a space after.
fn group_command(group: &str) -> String {
    if group.chars().any(char::is_whitespace) { format!("/group \"{group}\" ") } else { format!("/group {group} ") }
}

/// The character a link names (`user://`, any case as the original compares it).
fn linked_name(link: &str) -> Option<&str> {
    link.get(..7).filter(|p| p.eq_ignore_ascii_case("user://")).map(|_| &link[7..]).filter(|n| !n.is_empty())
}

/// Whether a line is one a view shows: a tell window (`only`: its character) the tells from and to that character,
/// a chat window the lines of the groups selected in it.
fn line_shows(line: &Line, pane: Option<&Pane>, only: Option<&str>) -> bool {
    match only {
        Some(name) => matches!(line.kind, Kind::Tell | Kind::Own | Kind::AwayReply) && line.from.as_deref().is_some_and(|f| f.eq_ignore_ascii_case(name)),
        None => pane.is_none_or(|p| p.selected.contains(line.group())),
    }
}

/// An invitation to a private chat group that waits for the player's answer: the inviter (the group goes by the
/// inviter's name), whether it marks the inviter's row in the friends list (`FriendListNode_c` +0x39), and the
/// question when it is up, with the chat windows chosen in its list.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Invite {
    owner: u32,
    name: String,
    marked: bool,
    asked: Option<std::collections::BTreeSet<usize>>,
}

/// The chat windows the question's list starts with chosen (FUN_100aef3a): those that show the group, when it is there.
fn shown_in(panes: &[Pane], group: &str) -> std::collections::BTreeSet<usize> {
    if !chatgroups::available(group) {
        return Default::default();
    }
    (0..panes.len()).filter(|k| panes[*k].selected.contains(group)).collect()
}

/// The order of the chat windows in the question's list: by name, no regard to case (FUN_100aef3a; the original puts
/// its default window first, which this client has not).
fn invitation_order(panes: &[Pane]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..panes.len()).collect();
    order.sort_by_key(|k| panes[*k].name.to_lowercase());
    order
}

/// The answer yes (FUN_100a74cb): every chat window chosen shows the group (FUN_1009d5cc), every other does not
/// (FUN_1009d87a), one that takes new groups by itself too.
fn put_group(panes: &mut [Pane], group: &str, chosen: &std::collections::BTreeSet<usize>) {
    for (k, pane) in panes.iter_mut().enumerate() {
        pane.known.insert(group.to_string());
        if chosen.contains(&k) {
            pane.selected.insert(group.to_string());
        } else {
            pane.selected.remove(group);
        }
    }
}

/// The invitation's question (`InviteDialog_c`, FUN_100ae840 from FUN_100a76c9): "PrivateGroupInvitation" over the text
/// with the inviter's name, a list of the chat windows (FUN_100af0ed: several can be chosen, a press turns one on or
/// off) and Yes and No. Some(true) for yes; Some(false) for no or the window closed.
fn invitation_box(ctx: &egui::Context, palette: &Palette, name: &str, panes: &[Pane], chosen: &mut std::collections::BTreeSet<usize>) -> Option<bool> {
    let title = crate::gui::xml::ldb_named(10001, "PrivateGroupInvitation", "Private chat group");
    let text = crate::gui::xml::ldb_named(10001, "ChatFriendList_InvitedToPrivateGroupDialogText", "%s invites you to a private chat group. Show it in the chat windows chosen below?").replace("%s", &format!("<font color=yellow>{name}</font>"));
    let [yes, no] = crate::chrome::yes_no();
    let font = crate::gui::font::FontId::NORMAL;
    let mut open = true;
    let mut answer = None;
    crate::chrome::window(ctx, palette, "private-invite", &title, &mut open, |w| w.resizable(false).pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.content_rect().center()), |ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        ui.set_width(260.0);
        ui.add_space(5.0);
        crate::gui::controls::markup(ui, palette, &text, font, crate::chrome::rgb(palette.text));
        ui.add_space(8.0);
        // (the list of the chat windows: a chosen one white on the selection's colour, another grey, the list view's
        // label colours)
        egui::Frame::new().stroke(egui::Stroke::new(1.0, crate::chrome::rgb(palette.text).gamma_multiply(0.4))).inner_margin(2.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            for k in invitation_order(panes) {
                let on = chosen.contains(&k);
                let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), egui::Sense::click());
                if on {
                    ui.painter().rect_filled(rect, 0.0, crate::chrome::rgb(palette.text).gamma_multiply(0.25));
                }
                let colour = if on { egui::Color32::from_rgb(0xff, 0xff, 0xff) } else { egui::Color32::from_rgb(0xc0, 0xc0, 0xc0) };
                ui.painter().text(rect.left_center() + egui::vec2(4.0, 0.0), egui::Align2::LEFT_CENTER, &panes[k].name, egui::FontId::proportional(12.0), colour);
                if response.clicked() && !chosen.remove(&k) {
                    chosen.insert(k);
                }
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if crate::chrome::button(ui, palette, &yes, true).clicked() {
                answer = Some(true);
            }
            ui.add_space(8.0);
            if crate::chrome::button(ui, palette, &no, true).clicked() {
                answer = Some(false);
            }
        });
        ui.add_space(5.0);
    });
    if !open { Some(false) } else { answer }
}

/// Which of the original's first places a chat window takes (`first_place`).
#[derive(Clone, Copy, Debug, PartialEq)]
enum FirstPlace {
    /// The default window, which FUN_100950df makes with its flag set ("Default Window").
    Default,
    /// Any other chat window (the "Combat" one, a new or cloned window, a torn-off tab).
    Other,
    /// The tell window (`TellWindow_c`, FUN_100b243e).
    Tell,
}

/// Where the original first puts a chat window on a `screen`, before its saved place (`Window::LoadWndConfig`) replaces
/// it: the window's top left and its client's size. The default window spans the foot of the screen, 224 in from each
/// side, from 200 above the bottom to 3 above it (FUN_100971a3, which FUN_1009829a gives a window made with the flag:
/// the window's outer rectangle, so its client is that less the `frame` round it); another is made with a 400 by 200
/// client (FUN_1009829a) and a tell window with a 389 by 219 one (FUN_100b243e), each put in the middle of the screen
/// (`Window::MoveToCenter`).
fn first_place(kind: FirstPlace, screen: egui::Rect, frame: egui::Vec2) -> (egui::Pos2, egui::Vec2) {
    let client = match kind {
        FirstPlace::Default => {
            let outer = egui::Rect::from_min_max(egui::pos2(screen.left() + 224.0, screen.bottom() - 200.0), egui::pos2(screen.right() - 224.0, screen.bottom() - 3.0));
            return (outer.min, (outer.size() - frame).max(egui::Vec2::ZERO));
        }
        FirstPlace::Other => egui::vec2(400.0, 200.0),
        FirstPlace::Tell => egui::vec2(389.0, 219.0),
    };
    (screen.center() - (client + frame) * 0.5, client)
}

/// Whether a tell from or to a character counts as one not read (FUN_100a7394): not when the character's tell window
/// is open (it is written there), nor when a chat window that is up has shown it (the `param_2` FUN_100a681d and
/// FUN_100a5f39 pass).
fn counts_unread(window_open: bool, shown: bool) -> bool {
    !window_open && !shown
}

/// Something to send on the chat connection.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    Tell { name: String, text: String },
    /// The automatic answer to a tell that came while away (`afk.rs`), to the character who sent it.
    AwayReply { name: String, text: String },
    /// `/dvalue`: one of the original's stored values shown (no value) or set.
    DValue { name: String, value: Option<String> },
    /// One of Gamecode's text commands (`mao_net::n3::text_command`): its action and number.
    TextCommand { action: u8, value: u32 },
    /// One of the organization's commands (`mao_net::n3::org_command`): its number, whether the selected target
    /// goes with it, and its text.
    OrgCommand { command: u8, targeted: bool, text: String },
    /// One of the raid's commands (`mao_net::n3::raid_command`): its number and its three identities.
    RaidCommand { command: u32, ids: [(u32, u32); 3] },
    /// Open the tell window for a character, once the chat server knows the name (it comes back through
    /// `chatgroups::open_tell`).
    TellWindow { name: String },
    /// `/voice`: a voice line to speak (its checks and text are the chat connection's, `voices::command`), in the
    /// window's output group: a channel, else (`None`) the vicinity (FUN_100b88b1's FUN_1009a9bd).
    Voice { line: String, channel: Option<String> },
    /// Vicinity speech (0 say, 1 whisper, 2 shout, 3 emote), sent on the game connection.
    Vicinity { kind: u8, text: String },
    /// An emote: the social action's `AbstractAnimID`.
    Social { id: u32 },
    /// Put a character on the friends list / take one off.
    Befriend { name: String },
    Unfriend { id: u32 },
    /// `/duel`: challenge the target (0) or accept (1), reject (2), stop (3), propose a draw (4).
    Duel(u32),
    /// `/petduel`: challenge the target (0) or accept (1), reject (2), stop (3).
    PetDuel(u32),
    /// A line to a channel of the chat server, by its name (the window's output group).
    Channel { name: String, text: String },
    /// Invite a character to the player's private chat group (the friends window's "Invite").
    PrivateInvite { id: u32 },
    /// Join the private chat group of the character who invited, or decline the invitation.
    PrivateJoin { owner: u32 },
    PrivateDecline { owner: u32 },
    /// Leave a private chat group, by its name.
    PrivateLeave { name: String },
    /// Disband the character's own private chat group.
    PrivateDisband,
    /// Sit down to log out / leave the client (handled by the window, not sent to the chat server).
    Camp,
    /// Show or hide the performance overlay (`/perf`).
    Perf,
    /// A command to the pets (`/pet`, `chat_commands::pet_line`).
    Pet(crate::chat_commands::PetLine),
    /// A command the server carries out (`/stuck` and the like): its text.
    Server(String),
    /// `/fxscript`: the line for the effect scripts' engine.
    FxScript(String),
    Follow,
    Inspect,
    Quit,
    /// Show or hide a window by name (empty and hidden: all of them).
    Window { name: String, open: bool },
    /// `/toggle`: show a window that is hidden, hide one that is shown.
    ToggleWindow { name: String },
    /// `/invite` (3), `/kick` (4), `/leave` (5): a request about a private chat group by a character's name, which
    /// the chat connection looks up first.
    PrivateByName { what: u8, name: String },
    /// `/cc <words>`: a command for the chat server, its words with their `%` arguments put in.
    Cc { words: Vec<String> },
    /// `/cc info <name>`: the chat server asked about the character named.
    CcInfo { name: String },
    /// `/rp`: the role-playing mark turned over (the game's connection sends it).
    RolePlay,
    /// `/name <new name>`: the request written to the renaming character.
    NameRequest { name: String },
    /// `/assist`: fight the opponent of the target, or of the character named.
    Assist { name: Option<String> },
    Waypoint { x: f32, z: f32 },
    SelectSelf,
    /// Set a client setting (empty name: say the list).
    SetOption { name: String, value: String },
    /// The original's `ViewDistance` preference to set (a command line's `/viewdist n`: n times 0.01).
    ViewDist(f32),
    /// The original's `DisplayCharViewDistance` value to set (`/chardist n`).
    CharDist(i32),
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
    /// "Transparency": the two sliders, for a window not active and for an active one.
    Solid,
    /// The right button on a character's name (FUN_1008e5ea): ignore, open the tell window, start a tell.
    Name,
    /// A chat window's menu from the friends list (FUN_100a8a69): its entries this client has no other menu for.
    Window,
}

/// An entry of an open menu with what choosing it means: the menu it is of, its key there, and its sub-menu's.
struct Node {
    menu: Menu,
    key: String,
    entry: crate::gui::popup::Entry,
    sub: Vec<Node>,
}

impl Node {
    fn leaf(menu: Menu, key: &str, entry: crate::gui::popup::Entry) -> Node {
        Node { menu, key: key.to_string(), entry, sub: Vec::new() }
    }

    fn parent(menu: Menu, key: &str, label: String, enabled: bool, sub: Vec<Node>) -> Node {
        Node { menu, key: key.to_string(), entry: crate::gui::popup::Entry::submenu(label, enabled, popup_entries(&sub)), sub }
    }

    fn separator(menu: Menu) -> Node {
        Node::leaf(menu, "", crate::gui::popup::Entry::separator())
    }
}

fn popup_entries(nodes: &[Node]) -> Vec<crate::gui::popup::Entry> {
    nodes.iter().map(|n| n.entry.clone()).collect()
}

/// The entry at a place through the sub-menus.
fn node_at<'a>(nodes: &'a [Node], path: &[usize]) -> Option<&'a Node> {
    let (&k, rest) = path.split_first()?;
    let node = nodes.get(k)?;
    if rest.is_empty() { Some(node) } else { node_at(&node.sub, rest) }
}

/// The place of the entry whose sub-menu is this menu.
fn path_to(nodes: &[Node], menu: Menu) -> Option<Vec<usize>> {
    for (k, n) in nodes.iter().enumerate() {
        if n.sub.first().is_some_and(|s| s.menu == menu) {
            return Some(vec![k]);
        }
        if let Some(mut path) = path_to(&n.sub, menu) {
            path.insert(0, k);
            return Some(path);
        }
    }
    None
}

/// The id of a frame's input bar.
fn bar_id(frame: &str) -> egui::Id {
    egui::Id::unique(("chat-input", frame))
}

/// The game owns the screen: the exclusive full-screen mode is on (`viewer::apply_mode`), the original's
/// `DisplaySystem_t` full-screen byte, which `/start` asks before it opens an address.
pub static EXCLUSIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub struct ChatWindow {
    pub open: bool,
    /// Whether every line is also written to the chat log (the live client turns it on).
    pub log: bool,
    lines: Vec<Line>,
    /// The original's chat windows, here the tabs (`chatgroups`), and the character they were loaded for.
    panes: Vec<Pane>,
    /// The tab being dragged (its pane) and where it was taken hold of, from the tab's corner.
    carried: Option<(usize, egui::Vec2)>,
    /// Where each frame's tab strip and its tabs (by pane) were drawn this frame, for a dragged tab to go to.
    strips: Vec<(String, egui::Rect, Vec<(usize, egui::Rect)>)>,
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
    menu: Option<(Vec<Node>, egui::Pos2)>,
    popup: Option<crate::gui::popup::PopupMenu>,
    /// The chat configuration window (`chatconfig`).
    pub config: crate::chatconfig::ChatConfig,
    /// What to do with an invitation to a private chat group (`ChatPGInviteAction`: 0 decline, else ask), the
    /// invitations waiting for an answer, and what the window sends besides the typed line.
    pub invite_action: i32,
    /// `ChatWarnWhenSpeakingToUnsubGroups`: a line sent to a group the window does not show is warned of.
    pub warn_unshown: bool,
    invites: Vec<Invite>,
    /// A `/start` address that waits for the answer to "leave the full screen?" ([`EXCLUSIVE`]).
    start_pending: Option<String>,
    pub outs: Vec<Out>,
    /// A preview's: the tab to show and the menu to have open (main, talk, public or combat).
    pub demo: Option<(usize, String)>,
    /// The names the `%` arguments of a sent line stand for (`chat_commands::expand`): the window's owner sets them.
    pub names: chat_commands::Names,
    /// The tell windows open: each one's place among them (its window and input bar go by it, so a layout is kept for
    /// the first, the second, not for every character ever told) and its character.
    tells: Vec<(usize, String)>,
    /// Each character's tells not read yet, by the name in lower case: the original's `FriendListNode_c` +0x34, which
    /// the friends list shows by a flashing row and a red folder ([`ChatWindow::unread`]).
    unread: std::collections::BTreeMap<String, u32>,
    /// The character whose tell window's input bar is being drawn: a plain line typed there is a tell.
    telling: Option<String>,
    /// The name menu's character, and the frame whose input bar "Send Tell" types into (`None`: the chat window's).
    menu_name: (String, Option<String>),
    /// A tell window to bring to the front (asked for again while open).
    raise: Option<usize>,
    /// How many more frames `focus_frame`'s bar is asked to take the keys: a window on its first frame is only
    /// measured, and what is asked of it then is lost.
    focus_tries: u8,
    /// A preview's: the character whose name menu is to be open.
    demo_name: Option<String>,
    input: String,
    /// The highlighted entry of the command list.
    selected: usize,
    focus_next: bool,
    last_tell: Option<String>,
    scroll_to_end: bool,
    /// Keeps the command list shown when the window itself has no keyboard focus (the previews).
    keep_list: bool,
    /// The characters whose tells and channel lines are hidden (lower case), kept in the character's `ignore.ini`
    /// (the original's `IgnoreList.bin` in the character's folder: `IgnoreSystem_t`, `Preferences_t::DeleteGamePrefs`).
    ignored: std::collections::BTreeSet<String>,
    /// The lines sent before, oldest first, and the one recalled with the arrow keys (`None`: the line being typed).
    history: Vec<String>,
    recalled: Option<usize>,
    /// The links clicked in the lines' markup (`itemref://`, `text://`, `chatcmd://`), for the info view.
    pub links: Vec<String>,
    /// The tip `/tipoftheday` showed last (the original's `CurrentTipOfTheDay`), 0 before the first.
    tip_of_the_day: i32,
    /// Away from the keyboard (`/afk`, `afk.rs`).
    pub away: crate::afk::Away,
    /// The lines of the scripts being run (`scripts.rs`).
    scripts: crate::scripts::Runner,
    /// The chat filter (`/filter`, `chatfilter.rs`).
    pub filter: crate::chatfilter::Filter,
    /// A macro `/macro` has just made (its number and name), for the pointer to carry to a hotbar slot.
    pub macro_made: Option<(i32, String)>,
    /// Set while a line of a script or a macro is carried out: it is not kept among the lines typed.
    unkept: bool,
}

impl Default for ChatWindow {
    fn default() -> Self {
        ChatWindow { open: true, log: false, lines: Vec::new(), panes: chatgroups::defaults(), loaded_for: None, tab: 0, shown: Default::default(), menu_pane: 0, inputs: Default::default(), focus_frame: None, born: Vec::new(), fade_times: (8.0, 0.3), menu: None, popup: None, config: Default::default(), invite_action: 1, warn_unshown: true, invites: Vec::new(), start_pending: None, outs: Vec::new(), demo: None, names: Default::default(), tells: Vec::new(), unread: Default::default(), telling: None, menu_name: Default::default(), raise: None, focus_tries: 0, demo_name: None, input: String::new(), selected: 0, focus_next: false, last_tell: None, scroll_to_end: true, keep_list: false, ignored: load_ignored(), history: Vec::new(), recalled: None, links: Vec::new(), tip_of_the_day: 0, away: Default::default(), scripts: Default::default(), filter: crate::chatfilter::Filter::load(), macro_made: None, unkept: false, carried: None, strips: Vec::new() }
    }
}

fn ignore_path() -> std::path::PathBuf {
    crate::charprefs::file("ignore.ini")
}

/// The ignore list as the chat connection's thread sees it (`IgnoreSystem_t::IsCharacterIgnored`, which
/// `HandleVicinityMessage` and `HandleGroupMessage` ask before anything else).
static IGNORED: std::sync::Mutex<std::collections::BTreeSet<String>> = std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Whether a character is on the ignore list.
pub fn ignores(name: &str) -> bool {
    IGNORED.lock().unwrap_or_else(|e| e.into_inner()).contains(&name.to_lowercase())
}

fn load_ignored() -> std::collections::BTreeSet<String> {
    // (previews and tests do not read the player's list)
    if std::env::var_os("MAO_UI_PREVIEW").is_some() || cfg!(test) {
        return Default::default();
    }
    let list: std::collections::BTreeSet<String> = std::fs::read_to_string(ignore_path()).map(|t| t.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect()).unwrap_or_default();
    *IGNORED.lock().unwrap_or_else(|e| e.into_inner()) = list.clone();
    list
}

fn save_ignored(list: &std::collections::BTreeSet<String>) {
    if !cfg!(test) {
        *IGNORED.lock().unwrap_or_else(|e| e.into_inner()) = list.clone();
        let _ = std::fs::write(ignore_path(), list.iter().map(|n| format!("{n}\n")).collect::<String>());
    }
}

/// How many lines are kept.
const KEEP: usize = 600;
/// The lines a chat view shows at most: ChatTextView_c takes the oldest out of its text while it has more than 200
/// (FUN_10091d8c, after each line FUN_10093ae6 appends).
const VIEW_LINES: usize = 200;
/// The lines sent that the arrow keys go through (FUN_1008f5b5).
const HISTORY: usize = 200;

/// How a fading chat line is drawn `age_ms` after it came (`ChatTextView_c`'s frame timer, GUI.dll FUN_100926f6):
/// `None` once it is older than the delay and the fade time together (the line is taken away), else its alpha, the
/// time left over the fade time, below 1 only once the delay is past. A fade time of 0 changes no alpha. The two are
/// whole milliseconds, the settings' seconds times 1000 cut (FUN_1008f8e7).
pub fn fade_alpha(age_ms: i64, delay_ms: i64, time_ms: i64) -> Option<f32> {
    if age_ms > delay_ms + time_ms {
        return None;
    }
    if time_ms <= 0 {
        return Some(1.0);
    }
    Some((((delay_ms + time_ms - age_ms) as f64 / time_ms as f64) as f32).min(1.0))
}

/// The lines of a chat window's selected groups, in a scroll view `height` tall that stays at its end.
/// With `fade` (the delay and the time, in seconds) a line older than the delay is drawn fainter, and not at all once
/// the time is over ([`fade_alpha`]); `born` is when each line came.
#[allow(clippy::too_many_arguments)]
fn draw_lines(ui: &mut egui::Ui, palette: &Palette, lines: &[Line], born: &[std::time::Instant], pane: Option<&Pane>, only: Option<&str>, height: f32, fade: Option<(f32, f32)>, clicks: &mut Vec<crate::gui::controls::LinkClick>) {
    let now = std::time::Instant::now();
    // (the height each marked-up line took, by its text, the width and the font's line height: a line out of sight is
    // then not laid out again every frame)
    let heights_id = egui::Id::unique("chat-line-heights");
    let mut heights: std::collections::HashMap<(u64, u32, u32), f32> = ui.ctx().data_mut(|d| std::mem::take(d.get_temp_mut_or_default(heights_id)));
    if heights.len() > 4000 {
        heights.clear();
    }
    let text = |name: &str, (r, g, b): (u8, u8, u8)| palette.text(name).map_or(egui::Color32::from_rgb(r, g, b), |c| egui::Color32::from_rgb(c[0], c[1], c[2]));
    let chat_font = if crate::font::enabled() { crate::font::chat(ui.ctx()) } else { None };
    let stamps = pane.is_some_and(|p| p.timestamps);
    crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(height).min_scrolled_height(height).auto_shrink([false, false]).stick_to_bottom(true), |ui| {
        let shown: Vec<usize> = lines.iter().enumerate().filter(|(_, l)| line_shows(l, pane, only)).map(|(k, _)| k).collect();
        for &k in &shown[shown.len().saturating_sub(VIEW_LINES)..] {
            let line = &lines[k];
            let ms = |seconds: f32| (seconds as f64 * 1000.0) as i64;
            let faint = match fade.zip(born.get(k)) {
                Some(((delay, time), came)) => match fade_alpha(now.duration_since(*came).as_millis() as i64, ms(delay), ms(time)) {
                    Some(alpha) => alpha,
                    None => continue,
                },
                None => 1.0,
            };
            // (set for each line: the setting stays on the view for what is drawn after)
            if fade.is_some() {
                ui.set_opacity(faint);
            }
            // (`show_timestamps`: the line led by its time, strftime's `(%H:%M) `)
            let stamp = line.at.filter(|_| stamps).map(|(h, m)| format!("({h:02}:{m:02}) ")).unwrap_or_default();
            let color = match line.kind {
                // (the original's TextColors.xml names when the theme has them)
                Kind::System => text("ct_cmd_feedback", (255, 220, 90)),
                Kind::Tell | Kind::AwayReply => text("ct_itell", (120, 220, 255)),
                // (a channel's lines in its group's colour, FUN_1008582a)
                Kind::Channel => text(chatgroups::colour_name(line.group()), (200, 235, 235)),
                Kind::Own => text("ct_otell", (160, 200, 160)),
                Kind::Combat => text("CCMeHitOtherColor", (240, 170, 130)),
            };
            if let Some(m) = &line.markup {
                let font = if chat_font.is_some() { crate::gui::font::FontId::named("CHAT") } else { crate::gui::font::FontId::NORMAL };
                let shown = format!("{stamp}{m}");
                let key = {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    shown.hash(&mut h);
                    let line_height = crate::gui::font::Measure { ctx: Some(ui.ctx().clone()) }.height(font);
                    (h.finish(), ui.available_width().max(10.0).to_bits(), line_height.to_bits())
                };
                let (took, click) = crate::gui::controls::markup_click(ui, palette, &shown, font, color, heights.get(&key).copied());
                heights.insert(key, took);
                clicks.extend(click);
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
    ui.ctx().data_mut(|d| d.insert_temp(heights_id, heights));
}

/// `/start`: an address handed to the system to open (the original's `ShellExecute` "open").
fn open_outside(address: &str) {
    if cfg!(test) {
        return;
    }
    #[cfg(windows)]
    let started = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", address]).spawn();
    #[cfg(target_os = "macos")]
    let started = std::process::Command::new("open").arg(address).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let started = std::process::Command::new("xdg-open").arg(address).spawn();
    if let Err(e) = started {
        eprintln!("/start: {e}");
    }
}

/// `/played`'s own line (FUN_100b2906): "Time: 14:05 local (20:05 GMT), 03:41 game" and "Date: 12. Nov 2026 local".
fn played_line() -> String {
    let utc = time::OffsetDateTime::now_utc();
    let local = time::OffsetDateTime::now_local().unwrap_or(utc);
    let (hour, minute) = crate::gametime::now();
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][local.month() as usize - 1];
    format!("Time: {:02}:{:02} local ({:02}:{:02} GMT), {hour:02}:{minute:02} game\nDate: {:02}. {month} {} local", local.hour(), local.minute(), utc.hour(), utc.minute(), local.day(), local.year())
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
        "ChatCmdFeedback_Ignore_IgnoringCharacter" => "%s is ignored now.",
        "ChatCmdFeedback_Ignore_UnignoringCharacter" => "%s is not ignored any more.",
        "ChatCmdFeedback_Ignore_CantIgnoreYourself" => "You cannot ignore your own character.",
        "ChatCmdFeedback_Ignore_NoTargetOrNick" => "Name a character, or select one, to ignore.",
        "ChatCmdFeedback_Ignore_ToManyArguments" => "Name one character only.",
        _ => name,
    };
    crate::gui::xml::ldb_named(10001, name, own)
}

/// What `/ignore` asks for (GUI.dll FUN_100bafeb): more than one word is too many; `list` the list; a word the
/// character so named; nothing the selected target, or nobody to ignore. (The original also takes a character's number
/// and refuses a target that is not a character, which the chat window cannot tell here.)
#[derive(Debug, PartialEq, Eq)]
pub enum IgnoreRequest {
    TooMany,
    List,
    Toggle(String),
    NoTarget,
}

pub fn ignore_request(text: &str, names: &chat_commands::Names) -> IgnoreRequest {
    let words: Vec<&str> = text.split_whitespace().collect();
    match words[..] {
        [] => names.target.clone().filter(|t| !t.is_empty()).map_or(IgnoreRequest::NoTarget, IgnoreRequest::Toggle),
        ["list"] => IgnoreRequest::List,
        [name] => IgnoreRequest::Toggle(name.to_string()),
        _ => IgnoreRequest::TooMany,
    }
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
        "IgnoreUser" => "Ignore",
        "OpenChat" => "Open chat",
        "SendTell" => "Send tell",
        "EnableClickthrough" => "Clicks go through",
        "ChatWindowMenu_DeactivateOnSend" => "Stop typing once sent",
        "ChatWindowMenu_DefaultWindow" => "Default window",
        "ShowChatWindow" => "Show the window",
        "ShowLogWindow" => "Show the message log",
        "LogMessages" => "Log messages",
        "RenameX" => "Rename %s",
        "DeleteX" => "Delete %s",
        "CloneX" => "Copy %s",
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
        // (the ignore list is the character's too)
        self.ignored = load_ignored();
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

    /// Whether a character's invitation to a private chat group marks their row in the friends list (+0x39).
    pub fn invited(&self, name: &str) -> bool {
        self.invites.iter().any(|i| i.marked && i.name.eq_ignore_ascii_case(name))
    }

    /// The friends list's press on the row of a character whose invitation is marked (FUN_100a7ca7 → FUN_100a76c9): the
    /// mark goes and the question comes.
    pub fn ask_invitation(&mut self, name: &str) {
        let panes = &self.panes;
        if let Some(invite) = self.invites.iter_mut().find(|i| i.marked && i.name.eq_ignore_ascii_case(name)) {
            invite.marked = false;
            invite.asked.get_or_insert_with(|| shown_in(panes, &invite.name));
        }
    }

    /// The invitations that came (FUN_100a7986): one from an ignored character is declined without a word; with the
    /// option at 0 it is declined and said so; else it is told of and waits for the player's answer, marked in the
    /// friends list (+0x39), with the question up at once at 1 (FUN_100a76c9) and at a press on the row at 2.
    fn take_invitations(&mut self) {
        for (owner, name) in chatgroups::take_invites() {
            let said = |key: &str, own: &str| crate::gui::xml::ldb_named(10001, key, own).replace("%s", &name);
            if self.is_ignored(&name) {
                self.outs.push(Out::PrivateDecline { owner });
            } else if self.invite_action == 0 {
                self.push(Line::new(Kind::System, said("ChatInviteAutoDeclined", "The invitation to %s's private chat group was declined.")));
                self.outs.push(Out::PrivateDecline { owner });
            } else {
                let asked = (self.invite_action == 1).then(|| shown_in(&self.panes, &name));
                self.push(Line::new(Kind::System, said("ChatInvitePending", "%s invites you to a private chat group.")));
                self.invites.push(Invite { owner, name, marked: true, asked });
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

    /// The right button on a character's name (FUN_1008e5ea): the three entries the chat view's flags allow, which a
    /// chat window's are all of.
    fn open_name_menu(&mut self, ctx: &egui::Context, name: &str, frame: Option<String>, at: egui::Pos2) {
        let nodes: Vec<Node> = [("ignore", "IgnoreUser"), ("chat", "OpenChat"), ("tell", "SendTell")].into_iter().map(|(k, n)| Node::leaf(Menu::Name, k, crate::gui::popup::Entry::text(menu_text(n), true))).collect();
        self.popup = Some(crate::chrome::menu_of(ctx, at, popup_entries(&nodes)));
        self.menu = Some((nodes, at));
        self.menu_name = (name.to_string(), frame);
    }

    /// The window's menu (FUN_100abb98 under FUN_10097a40's "Visual"), its sub-menus inside it.
    fn menu_tree(&self) -> Vec<Node> {
        use crate::gui::popup::Entry;
        let Some(pane) = self.panes.get(self.menu_pane) else { return Vec::new() };
        let mut groups: std::collections::BTreeSet<String> = chatgroups::BUILTIN.iter().map(|g| g.1.to_string()).collect();
        groups.extend(chatgroups::server_groups());
        groups.extend(pane.known.iter().cloned());
        let look = pane.look;
        // (FUN_10099a56: the mode is for a window that has its frame to itself)
        let alone = chatgroups::members(&self.panes, &pane.frame).len() == 1;
        let mode = [("normal", "ChatWindowMenu_Style_Mode_Normal", !look.borderless), ("borderless", "ChatWindowMenu_Style_Mode_Borderless", look.borderless)]
            .into_iter()
            .map(|(k, label, mark)| Node::leaf(Menu::Mode, k, Entry::marked(menu_text(label), true, mark)))
            .collect();
        let mut visual = vec![Node::parent(Menu::Visual, "mode", menu_text("ChatWindowMenu_Style_Mode"), alone, mode)];
        // (how solid: only a borderless window's; two sliders from 0 to 1, the window not active first)
        if look.borderless {
            let solid = [("inactive", "ChatWindowMenu_Style_Transparancy_InactiveToolTip", look.solid.1), ("active", "ChatWindowMenu_Style_Transparancy_ActiveToolTip", look.solid.0)]
                .into_iter()
                .map(|(k, tip, now)| Node::leaf(Menu::Solid, k, Entry::slider(0.0, 1.0, f64::from(now) / 100.0).tipped(menu_text(tip))))
                .collect();
            visual.push(Node::parent(Menu::Visual, "solid", menu_text("ChatWindowMenu_Style_Transparancy"), true, solid));
        }
        visual.push(Node::separator(Menu::Visual));
        for (k, label, enabled, mark) in [
            ("timestamps", "ShowTimestamps", true, pane.timestamps),
            ("noinput", "DisableTextInput", true, !look.input),
            ("hideinput", "ChatWindowMenu_Style_HideInputBarWhenInactive", look.input, look.hide_input),
            ("fade", "ChatWindowMenu_Style_FadeMessages", true, look.fade),
        ] {
            visual.push(Node::leaf(Menu::Visual, k, Entry::marked(menu_text(label), enabled, mark)));
        }
        visual.push(Node::separator(Menu::Visual));
        visual.push(Node::leaf(Menu::Visual, "behind", Entry::marked(menu_text("ChatWindowMenu_Style_AlwaysBehind"), true, look.layer < 0)));
        visual.push(Node::leaf(Menu::Visual, "top", Entry::marked(menu_text("ChatWindowMenu_Style_AlwaysOnTop"), true, look.layer > 0)));

        let output = if pane.output.is_empty() { chatgroups::VICINITY } else { pane.output.as_str() };
        let mut talk: Vec<Node> = groups.iter().filter(|g| pane.selected.contains(*g) && chatgroups::talkable(g)).map(|g| Node::leaf(Menu::Talk, g, Entry::marked(g.clone(), true, g == output))).collect();
        if talk.is_empty() {
            talk.push(Node::leaf(Menu::Talk, "", Entry::text(menu_text("NoSelectableGroups"), false)));
        }
        // (FUN_100ab758: the kinds of groups, each kind's groups with their marks)
        let kind = |combat: bool| -> Vec<Node> { groups.iter().filter(|g| chatgroups::is_combat(g) == combat).map(|g| Node::leaf(Menu::Groups { combat }, g, Entry::marked(g.clone(), true, pane.selected.contains(g)))).collect() };
        let subscribe = vec![Node::parent(Menu::Subscribe, "public", "Public Groups".to_string(), true, kind(false)), Node::parent(Menu::Subscribe, "combat", "Combat Messages".to_string(), true, kind(true))];

        let mut main = vec![
            Node::parent(Menu::Main, "visual", menu_text("ChatWindowMenu_Visual"), true, visual),
            Node::separator(Menu::Main),
            Node::parent(Menu::Main, "talk", menu_text("ChatWindowMenu_TalkToChannel"), true, talk),
            Node::parent(Menu::Main, "subscribe", menu_text("ChatWindowMenu_ChannelSubscribeMenu"), true, subscribe),
            Node::leaf(Menu::Main, "auto", Entry::marked(menu_text("ChatWindowMenu_AutoSubscribeChannels"), true, pane.auto)),
            Node::separator(Menu::Main),
            // (marked while its window is up)
            Node::leaf(Menu::Main, "config", Entry::marked(menu_text("ChatWindowMenu_ChatConfiguration"), true, self.config.open)),
            Node::separator(Menu::Main),
            Node::leaf(Menu::Main, "rename", Entry::text(menu_text("RenameWindow"), true)),
            Node::leaf(Menu::Main, "delete", Entry::text(menu_text("DeleteWindow"), self.panes.len() > 1)),
            Node::leaf(Menu::Main, "new", Entry::text(menu_text("NewWindow"), true)),
        ];
        // (this client's own, in place of dragging the tab: out into a window of its own when it shares its frame,
        // or into another frame, named by its first window)
        let mut own = Vec::new();
        if chatgroups::members(&self.panes, &pane.frame).len() > 1 {
            own.push(Node::leaf(Menu::Main, "detach", Entry::text(menu_text("own:detach"), true)));
        }
        for f in chatgroups::frames(&self.panes).into_iter().filter(|f| *f != pane.frame) {
            let first = self.panes.iter().find(|p| p.frame == f).map_or(f.clone(), |p| p.name.clone());
            own.push(Node::leaf(Menu::Main, &format!("attach:{f}"), Entry::text(menu_text("own:attach").replace("%s", &first), true)));
        }
        if !own.is_empty() {
            main.push(Node::separator(Menu::Main));
            main.extend(own);
        }
        main
    }

    /// The chat windows' names, in their order (the friends list's "Chat Windows").
    pub fn window_names(&self) -> Vec<String> {
        self.panes.iter().map(|p| p.name.clone()).collect()
    }

    /// A chat window's menu as the friends list's right button opens it (FUN_100a8a69): "Visual" (the window menu's own,
    /// FUN_1009a00d), a separator, "Enable Click-through", "Disable Text Input", "Deactivate On Send", a separator,
    /// "Default Window", "Auto Subscribe Channels", a separator, "Show Chat Window", "Show Log Window", "Log Messages",
    /// a separator, "Rename %s", "Delete %s", "Clone %s" with the window's name and "New Window" (FUN_1009b901). The
    /// click-through, the default window and the log window are not in this client: their entries are drawn and cannot be
    /// chosen.
    fn window_menu_tree(&self) -> Vec<Node> {
        use crate::gui::popup::Entry;
        let Some(pane) = self.panes.get(self.menu_pane) else { return Vec::new() };
        let named = |label: &str| menu_text(label).replace("%s", &pane.name);
        let not_here = |label: &str| Node::leaf(Menu::Window, "", Entry::marked(menu_text(label), false, false));
        let mut nodes: Vec<Node> = self.menu_tree().into_iter().take(1).collect();
        nodes.extend([
            Node::separator(Menu::Window),
            not_here("EnableClickthrough"),
            Node::leaf(Menu::Visual, "noinput", Entry::marked(menu_text("DisableTextInput"), true, !pane.look.input)),
            Node::leaf(Menu::Visual, "deactivate", Entry::marked(menu_text("ChatWindowMenu_DeactivateOnSend"), true, pane.look.deactivate)),
            Node::separator(Menu::Window),
            not_here("ChatWindowMenu_DefaultWindow"),
            Node::leaf(Menu::Main, "auto", Entry::marked(menu_text("ChatWindowMenu_AutoSubscribeChannels"), true, pane.auto)),
            Node::separator(Menu::Window),
            Node::leaf(Menu::Window, "show", Entry::marked(menu_text("ShowChatWindow"), true, self.open)),
            not_here("ShowLogWindow"),
            Node::leaf(Menu::Window, "log", Entry::marked(menu_text("LogMessages"), true, self.log)),
            Node::separator(Menu::Window),
            Node::leaf(Menu::Main, "rename", Entry::text(named("RenameX"), true)),
            Node::leaf(Menu::Main, "delete", Entry::text(named("DeleteX"), self.panes.len() > 1)),
            Node::leaf(Menu::Window, "clone", Entry::text(named("CloneX"), true)),
            Node::leaf(Menu::Main, "new", Entry::text(menu_text("NewWindow"), true)),
        ]);
        nodes
    }

    /// Opens a chat window's menu (FUN_100a8a69) at a place: the friends list's right button on the window's row.
    pub fn window_menu(&mut self, ctx: &egui::Context, pane: usize, at: egui::Pos2) {
        if pane >= self.panes.len() {
            return;
        }
        self.menu_pane = pane;
        let nodes = self.window_menu_tree();
        self.popup = Some(crate::chrome::menu_of(ctx, at, popup_entries(&nodes)));
        self.menu = Some((nodes, at));
    }

    /// Opens the window's menu at a place, with one of its sub-menus up when one is named.
    fn open_menu(&mut self, ctx: &egui::Context, at: egui::Pos2, sub: Option<Menu>) {
        let nodes = self.menu_tree();
        if nodes.is_empty() {
            return;
        }
        let mut popup = crate::chrome::menu_of(ctx, at, popup_entries(&nodes));
        if let Some(path) = sub.and_then(|m| path_to(&nodes, m)) {
            popup.open_path(&path, ctx.cumulative_frame_nr());
        }
        self.popup = Some(popup);
        self.menu = Some((nodes, at));
    }

    /// An entry of the open menu was chosen.
    fn choose(&mut self, ctx: &egui::Context, menu: Menu, key: &str, at: egui::Pos2) {
        // (the name's menu: FUN_100ac39e ignores the character or stops ignoring, FUN_100ac2fa opens the tell window,
        // FUN_100ac42b types the start of a tell)
        if menu == Menu::Name {
            let (name, frame) = self.menu_name.clone();
            match key {
                "ignore" => self.toggle_ignore(&name),
                "chat" => self.ask_tell(&name),
                _ => self.type_in(frame.as_deref(), &format!("/tell {name} ")),
            }
            return;
        }
        let tab = self.menu_pane.min(self.panes.len().saturating_sub(1));
        match (menu, key) {
            (Menu::Main, "config") => self.config.open = !self.config.open,
            // (FUN_100a8a69's own: the window shown or hidden, its lines logged or not, a copy of it in a frame of its
            // own as the chat configuration's "Clone Window" makes)
            (Menu::Window, "show") => self.open = !self.open,
            (Menu::Window, "log") => self.log = !self.log,
            (Menu::Window, "clone") => {
                if let Some(mut pane) = self.panes.get(tab).cloned() {
                    pane.name = chatgroups::free_name(&self.panes);
                    pane.frame = pane.name.clone();
                    self.panes.push(pane);
                }
            }
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
                    (Menu::Visual, "deactivate") => pane.look.deactivate = !pane.look.deactivate,
                    (Menu::Mode, mode) => pane.look.borderless = mode == "borderless",
                    (Menu::Talk, group) if !group.is_empty() => pane.output = group.to_string(),
                    (Menu::Groups { .. }, group) if !group.is_empty() => {
                        pane.toggle(group);
                        // (this client's own: the list stays up for the next one)
                        self.open_menu(ctx, at, Some(menu));
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
        // (the chat filter: a group's line a rule matches is not shown, FUN_100854a8; the text is what was said,
        // after the speaker's name)
        if line.kind == Kind::Channel && self.filter.enabled {
            let said = line.from.as_ref().and_then(|who| line.text.split_once(&format!("{who}: "))).map_or(line.text.as_str(), |p| p.1);
            if self.filter.hides(said) {
                return;
            }
        }
        // (a tell that comes while away is marked so and answered: `HandlePrivateMessage`)
        if line.kind == Kind::Tell
            && let Some(from) = line.from.clone()
            && !from.eq_ignore_ascii_case(&self.names.own)
            && let Some(reply) = self.away.reply(&self.names.own, std::time::Instant::now())
        {
            line.text.push_str(" (AFK reply sent)");
            if let Some(markup) = &mut line.markup {
                markup.push_str(" <font color=white>(AFK reply sent)</font>");
            }
            self.outs.push(Out::AwayReply { name: from, text: reply });
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
        // (a tell from or to a character, FUN_100a681d and FUN_100a5f39, is kept with the character, FUN_100a7394: with
        // its tell window up it goes there; else it is one more not read, unless a chat window that is up shows the
        // tells' group, FUN_1008543d(1))
        if matches!(line.kind, Kind::Tell | Kind::Own | Kind::AwayReply)
            && let Some(who) = &line.from
            && counts_unread(self.tell_open(who), self.group_up(&group))
        {
            *self.unread.entry(who.to_lowercase()).or_default() += 1;
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

    /// What a step of being away asks for (`afk::Did`): its lines in the player's chat, and its emote to the vicinity.
    fn away_did(&mut self, did: crate::afk::Did) -> Option<Out> {
        for line in &did.lines {
            self.tell(line);
        }
        // (`MAO_AFK_QUIET`: a trial that says nothing to those around)
        did.emote.filter(|_| std::env::var_os("MAO_AFK_QUIET").is_none()).map(|text| Out::Vicinity { kind: 3, text })
    }

    /// Each frame before the windows are drawn, so before a line typed this frame is carried out (the original looks
    /// at the input first, `InputConfig_t::AddUserInput`, and `/afk` typed is not undone by its own Enter): input
    /// brings the character back and restarts the timer of going away by itself. `auto` is the `AutoAFK` setting;
    /// `game_master` leaves the character out (`SlotAutoAFK`).
    pub fn away_input(&mut self, ctx: &egui::Context, auto: bool, game_master: bool) {
        // (a key or button going down, the mouse moved; a release is not input: FUN_00404a66 marks it and
        // `AddUserInput` passes it by)
        // (dead, the game takes no input: the timer stops, and starts again when it takes input again; what is
        // pressed meanwhile still counts, `afk::INPUT_WHILE_STOPPED`)
        let now = std::time::Instant::now();
        self.away.input_off(crate::net::INPUT_STOPPED.load(std::sync::atomic::Ordering::Relaxed), now);
        let stopped_input = crate::afk::INPUT_WHILE_STOPPED.swap(false, std::sync::atomic::Ordering::Relaxed);
        let input = stopped_input || ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. } | egui::Event::PointerButton { pressed: true, .. } | egui::Event::PointerMoved(_) | egui::Event::MouseMoved(_) | egui::Event::MouseWheel { .. } | egui::Event::Text(_))));
        if input {
            let sneaking = crate::afk::SNEAKING.load(std::sync::atomic::Ordering::Relaxed);
            let did = self.away.input(auto, game_master, sneaking, now);
            let out = self.away_did(did);
            self.outs.extend(out);
        }
    }

    /// Each frame after the windows, whether the chat window is open or not: the timer of going away by itself and
    /// the box's count are looked at, and the box of `/afk` is drawn (over the chat's windows).
    pub fn away_frame(&mut self, ctx: &egui::Context, palette: &Palette, auto: bool, game_master: bool) {
        let now = std::time::Instant::now();
        let sneaking = crate::afk::SNEAKING.load(std::sync::atomic::Ordering::Relaxed);
        let did = self.away.tick(auto, game_master, sneaking, crate::afk::idle(), now);
        let out = self.away_did(did);
        self.outs.extend(out);
        // The box (`AFKMessageDialog_c`): its text, the count down, the message in a field and one button, in the
        // screen's middle (`MoveToCenter`).
        let Some(dialog) = self.away.dialog.as_mut() else { return };
        let mut open = true;
        let mut answer = None;
        let count = dialog.count(now);
        let mut typed = false;
        // (`MoveToCenter` when it comes up; on its first frames, as the very first frames' screen is not the window's)
        let fresh = dialog.shown < 3;
        dialog.shown = dialog.shown.saturating_add(1);
        let middle = ctx.content_rect().center();
        let body = dialog.body.trim_end().replace("\r\n", "<br>").replace('\n', "<br>");
        let place = |w: egui::Window<'static>| {
            let w = w.resizable(false).pivot(egui::Align2::CENTER_CENTER);
            if fresh { w.current_pos(middle) } else { w }
        };
        crate::chrome::window(ctx, palette, "afk-message", "AFK", &mut open, place, |ui| {
            let font = crate::gui::font::FontId::NORMAL;
            let width = 260.0;
            ui.set_width(width);
            crate::gui::controls::markup(ui, palette, &body, font, crate::chrome::rgb(palette.text));
            ui.add_space(3.0);
            // (the count is a line of its own, empty once a key stopped it)
            crate::gui::controls::markup(ui, palette, &count.map(|c| c.to_string()).unwrap_or_else(|| " ".to_string()), font, crate::chrome::rgb(palette.text));
            let field = crate::chrome::text_edit(ui, palette, egui::TextEdit::singleline(&mut dialog.message).desired_width(width - 12.0));
            crate::chrome::focus_when_new(ctx, "afk-message", &field);
            // (FUN_1008356e, the field's key signal: any key stops the count, Enter takes the text, Escape ends being
            // away. egui's field gives up the focus on either of the two, so "had it or just lost it".)
            let key = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. } | egui::Event::Text(_))));
            typed = field.changed() || (key && field.has_focus());
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                answer = Some(true);
            } else if (field.lost_focus() || field.has_focus()) && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                answer = Some(false);
            }
            ui.add_space(4.0);
            ui.vertical_centered(|ui| {
                if crate::chrome::button(ui, palette, "Ok", true).clicked() {
                    answer = Some(true);
                }
            });
        });
        if typed {
            dialog.counting = false;
        }
        // (closed without the button, with Escape: no longer away)
        if !open && answer.is_none() {
            answer = Some(false);
        }
        if let Some(accepted) = answer {
            let typed = dialog.message.clone();
            let did = self.away.answer(accepted, &typed, now);
            let out = self.away_did(did);
            self.outs.extend(out);
        }
        // (the count is redrawn as it goes)
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    /// Whether what a character says is hidden (`/ignore`).
    pub fn is_ignored(&self, name: &str) -> bool {
        self.ignored.contains(&name.to_lowercase())
    }

    /// Puts text in the input line and starts typing there (the previews use it).
    /// Text added to the end of the line being typed (an item's link from its menu), a space before it when the line has
    /// text.
    pub fn add_text(&mut self, text: &str) {
        if !self.input.is_empty() && !self.input.ends_with(' ') {
            self.input.push(' ');
        }
        self.input.push_str(text);
        self.keep_list = true;
        self.focus();
    }

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

    /// Puts text in a frame's input bar (`None`: the chat window's own) and starts typing there.
    fn type_in(&mut self, frame: Option<&str>, text: &str) {
        match frame {
            None => {
                self.input = text.to_string();
                self.focus();
            }
            Some(frame) => {
                self.inputs.insert(frame.to_string(), text.to_string());
                self.focus_frame = Some(frame.to_string());
            }
        }
    }

    /// The chat window a frame shows now (`None`: the chat window itself; a tell window has none).
    fn pane_of(&self, frame: Option<&str>) -> Option<usize> {
        match frame {
            None => Some(self.tab),
            Some(frame) => self.shown.get(frame).copied().filter(|k| self.panes.get(*k).is_some_and(|p| p.frame == frame)).or_else(|| chatgroups::members(&self.panes, frame).first().copied()),
        }
    }

    /// A group's name pressed in a line (`chatgroup://`, FUN_100ac4f0): with no Shift held, in a chat window
    /// that shows the group, the window talks to it from then on (FUN_1009a7c0, when the group takes lines); else the
    /// window's input bar is given `/group` and the name. A group not there is passed over.
    fn group_pressed(&mut self, frame: Option<&str>, group: &str, qualified: bool) {
        if !chatgroups::available(group) {
            return;
        }
        let pane = self.pane_of(frame).filter(|k| !qualified && self.panes.get(*k).is_some_and(|p| p.selected.contains(group)));
        match pane {
            Some(k) => {
                if self.panes[k].output != group && !chatgroups::read_only(group) {
                    self.panes[k].output = group.to_string();
                    chatgroups::save(&self.panes);
                }
            }
            None => self.type_in(frame, &group_command(group)),
        }
    }

    /// A sent line's `%` arguments put in (`chat_commands::expand`).
    fn expand(&self, text: &str) -> String {
        chat_commands::expand(text, &self.names)
    }

    /// The frame name of a tell window, by its place among them.
    fn tell_frame(slot: usize) -> String {
        format!("Tell:{slot}")
    }

    /// The characters with a tell window open.
    pub fn tell_windows(&self) -> Vec<&str> {
        self.tells.iter().map(|t| t.1.as_str()).collect()
    }

    /// Whether a character's tell window is open (FUN_100a74bc, which selects its row in the friends list).
    pub fn tell_open(&self, name: &str) -> bool {
        self.tells.iter().any(|t| t.1.eq_ignore_ascii_case(name))
    }

    /// How many of a character's tells are not read yet (`FriendListNode_c` +0x34).
    pub fn unread(&self, name: &str) -> u32 {
        self.unread.get(&name.to_lowercase()).copied().unwrap_or(0)
    }

    /// Whether a chat window that is up shows a group (FUN_1008543d with 1: a window whose frame is there, +0xd0, and
    /// that shows the group). Every frame of chat windows is up while the chat window is.
    fn group_up(&self, group: &str) -> bool {
        self.open && self.panes.iter().any(|p| p.selected.contains(group))
    }

    /// Opens the tell window for a character (`ChatGUIModule_c::OpenTellWindow`), or brings up the one open, and
    /// starts typing in it. Its tells are read then: FUN_100a7ca7 sets the count back to 0 as it opens the window.
    pub fn open_tell(&mut self, name: &str) {
        let slot = match self.tells.iter().find(|t| t.1.eq_ignore_ascii_case(name)) {
            Some(t) => t.0,
            None => {
                let slot = (0..).find(|s| !self.tells.iter().any(|t| t.0 == *s)).unwrap_or(0);
                self.tells.push((slot, name.to_string()));
                self.focus_tries = 2;
                self.unread.remove(&name.to_lowercase());
                slot
            }
        };
        self.raise = Some(slot);
        self.focus_frame = Some(Self::tell_frame(slot));
    }

    /// Closes a character's tell window (FUN_100a7ca7 with false: the friends list's press on the row of a character
    /// whose window is open).
    pub fn close_tell(&mut self, name: &str) {
        if let Some(slot) = self.tells.iter().find(|t| t.1.eq_ignore_ascii_case(name)).map(|t| t.0) {
            self.tells.retain(|t| t.0 != slot);
            self.inputs.remove(&Self::tell_frame(slot));
        }
    }

    /// Asks for the tell window of a character: the chat server is asked for the name first (FUN_1008a2c1 opens the
    /// window only for a name it knows, and says so for one it does not), unless the window is open already.
    /// None for the player's own character (FUN_10089940).
    pub fn ask_tell(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if name.eq_ignore_ascii_case(&self.names.own) {
            self.push(Line::new(Kind::System, "You cannot send private messages to yourself."));
        } else if self.tells.iter().any(|t| t.1.eq_ignore_ascii_case(name)) {
            self.open_tell(name);
        } else {
            self.outs.push(Out::TellWindow { name: name.to_string() });
        }
    }

    /// A tell to a character: its echo (`ChatTellMsgToField` with the name as a link, FUN_100a5f39) and what to
    /// send. FUN_10089940: none to the player's own character, and a text over 0x400 long is dropped without a word.
    fn send_tell(&mut self, name: String, text: String) -> Option<Out> {
        if text.len() > 0x400 {
            return None;
        }
        if name.eq_ignore_ascii_case(&self.names.own) {
            self.push(Line::new(Kind::System, "You cannot send private messages to yourself."));
            return None;
        }
        let to = crate::gui::xml::ldb_named(10001, "ChatTellMsgToField", "To [%s]: ");
        let mut line = Line::new(Kind::Own, format!("{}{}", to.replace("%s", &name), mao_net::chat::plain(&text))).with_markup(&to.replace("%s", &name_link(&name)), &text);
        line.from = Some(name.clone());
        self.push(line);
        Some(Out::Tell { name, text })
    }

    /// `/ignore <name>` and the name menu's entry: the character's lines hidden, or shown again.
    pub fn toggle_ignore(&mut self, name: &str) {
        // (`/ignore` FUN_100bafeb: not the player's own character; then ignored or no longer, said with the name)
        if !self.names.own.is_empty() && name.eq_ignore_ascii_case(&self.names.own) {
            self.push(Line::new(Kind::System, command_text("ChatCmdFeedback_Ignore_CantIgnoreYourself")));
            return;
        }
        let key = name.to_lowercase();
        let text = if self.ignored.remove(&key) {
            command_text("ChatCmdFeedback_Ignore_UnignoringCharacter")
        } else {
            self.ignored.insert(key);
            command_text("ChatCmdFeedback_Ignore_IgnoringCharacter")
        };
        save_ignored(&self.ignored);
        self.push(Line::new(Kind::System, text.replace("%s", name)));
    }

    /// Carries out a typed line: what to send (if anything), and the lines it adds to the window.
    /// Remembers a sent line for the arrow keys (FUN_1008f5b5: the line stripped; an empty one is not kept; the same
    /// line kept before is taken out of its place, the oldest go while there are 200 or more, and it goes last).
    fn remember(&mut self, line: &str) {
        let line = line.trim();
        if !line.is_empty() {
            if let Some(k) = self.history.iter().position(|l| l == line) {
                self.history.remove(k);
            }
            while self.history.len() >= HISTORY {
                self.history.remove(0);
            }
            self.history.push(line.to_string());
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
        if self.warn_unshown && pane < self.panes.len() && !self.panes.get(pane).is_some_and(|p| p.selected.contains(group)) {
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
    /// A line carried out as if typed, but not kept among the lines typed: a macro's, a script's.
    pub fn run_unkept(&mut self, line: &str) -> Option<Out> {
        self.unkept = true;
        let out = self.run_in(self.tab, line);
        self.unkept = false;
        out
    }

    /// Each frame (`dt` seconds): the macros pressed are carried out, and the next line of a running script
    /// (FUN_100b1d6c: one a frame).
    pub fn script_frame(&mut self, dt: f32) {
        for line in crate::macros::take_pressed() {
            let out = self.run_unkept(&line);
            self.outs.extend(out);
        }
        if let Some(line) = self.scripts.tick(dt) {
            let out = self.run_unkept(&line);
            self.outs.extend(out);
        }
    }

    /// A script's lines queued, or why not said (FUN_100b1517).
    fn run_script(&mut self, line: &str) {
        match crate::scripts::load(line) {
            Ok(lines) => self.scripts.add(lines),
            Err(problem) => self.tell(&problem.text()),
        }
    }

    /// `/filter` (FUN_100b933d).
    fn filter_command(&mut self, what: &str, rest: &str) {
        let said = match what {
            "list" => {
                for (number, rule) in self.filter.rules.clone() {
                    self.push(Line::new(Kind::System, format!("{number:3}: {rule}")));
                }
                return;
            }
            "add" | "del" if rest.is_empty() => "Usage: /filter [list|del|add|enable|disable|clear]".to_string(),
            "add" => format!("Added as {}.", self.filter.add(rest)),
            "del" => {
                // (`RemoveName` of the word: one that is no rule's number removes nothing, and it says "Deleted." all the same)
                if let Ok(number) = rest.split_whitespace().next().unwrap_or("").parse() {
                    self.filter.delete(number);
                }
                "Deleted.".to_string()
            }
            "enable" | "disable" => {
                self.filter.enabled = what == "enable";
                if self.filter.enabled { "Enabled." } else { "Disabled." }.to_string()
            }
            "clear" => {
                self.filter.rules.clear();
                "Cleared.".to_string()
            }
            _ => "Usage: /filter [list|del|add|enable|disable|clear]".to_string(),
        };
        self.filter.save();
        self.push(Line::new(Kind::System, said));
    }

    pub fn run_in(&mut self, pane: usize, line: &str) -> Option<Out> {
        if !self.unkept {
            self.remember(line);
        }
        // (a plain line typed in a tell window is a tell to its character)
        if !line.trim().is_empty()
            && !line.trim_start().starts_with('/')
            && let Some(name) = self.telling.clone()
        {
            let text = self.expand(line.trim());
            return self.send_tell(name, text);
        }
        // (a plain line goes to the window's output group when that is a channel; the vicinity's goes on as speech)
        if !line.trim().is_empty()
            && !line.trim_start().starts_with('/')
            && let Some(name) = self.output_channel(pane)
        {
            return Some(Out::Channel { name: name.to_string(), text: self.expand(line.trim()) });
        }
        match chat_commands::parse(line) {
            Parsed::Empty => None,
            // (FUN_100ba8d5: the name and the text with their `%` arguments put in; no text opens the tell window)
            Parsed::Tell { name, text } => {
                let (name, text) = (self.expand(&name), self.expand(&text));
                if text.is_empty() {
                    self.ask_tell(&name);
                    None
                } else {
                    self.send_tell(name, text)
                }
            }
            Parsed::Reply { text } => match self.last_tell.clone() {
                Some(name) => {
                    let text = self.expand(&text);
                    self.send_tell(name, text)
                }
                None => {
                    self.push(Line::new(Kind::System, "There is nobody to reply to."));
                    None
                }
            },
            Parsed::Voice { line } => Some(Out::Voice { line, channel: self.output_channel(pane).map(str::to_string) }),
            Parsed::Camp => Some(Out::Camp),
            Parsed::Pet { mut line } => {
                line.text = self.expand(&line.text);
                Some(Out::Pet(line))
            }
            Parsed::Perf => Some(Out::Perf),
            // (FUN_100b285d: its `%` arguments put in)
            Parsed::Server { text } => Some(Out::Server(self.expand(&text))),
            Parsed::FxScript { line } => Some(Out::FxScript(line)),
            Parsed::Quit => Some(Out::Quit),
            Parsed::Follow => Some(Out::Follow),
            Parsed::Window { name, open } => Some(Out::Window { name, open }),
            Parsed::Toggle { name } => Some(Out::ToggleWindow { name }),
            // (`/text`: FUN_1009bad0 with the text as it is)
            Parsed::Text { text } => {
                self.push(Line::new(Kind::System, text));
                None
            }
            // (`/showfile`: the info view's `file://` page, as a clicked link is one)
            Parsed::ShowFile { file } => {
                self.links.push(format!("file://{file}"));
                None
            }
            // (`/tipoftheday`: the section after the one shown last, or before it; `CurrentTipOfTheDay`)
            Parsed::TipOfTheDay { previous } => {
                self.tip_of_the_day = if previous { (self.tip_of_the_day - 1).max(0) } else { self.tip_of_the_day + 1 };
                self.links.push(format!("file://TipOfTheDay.html?section={}", self.tip_of_the_day));
                None
            }
            // (`/macro`, FUN_100b8c82 and the shortcut bar's FUN_100d8c4a: the macro made and saved, and carried by
            // the pointer to where it is dropped; nothing is said)
            Parsed::Macro { name, command } => {
                let id = crate::macros::create(&name, &command);
                let name = crate::macros::get(id).map_or(name, |m| m.name);
                self.macro_made = Some((id, name));
                None
            }
            Parsed::Script { line } => {
                self.run_script(&line);
                None
            }
            Parsed::Filter { what, rest } => {
                self.filter_command(&what, &rest);
                None
            }
            Parsed::DValue { name, value } => Some(Out::DValue { name, value }),
            // (FUN_100ba073, FUN_100b9acf: only an http or https address, opened by the system. While the game owns the
            // screen (the display system's full-screen byte: the exclusive mode, not a borderless window) a box asks
            // first, titled "Warning:" with text 10000's `ReallyMinimizeGame`, and only Yes opens the address.)
            Parsed::Start { address } => {
                let lower = address.to_lowercase();
                if lower.starts_with("http://") || lower.starts_with("https://") {
                    if EXCLUSIVE.load(std::sync::atomic::Ordering::Relaxed) {
                        self.start_pending = Some(address);
                    } else {
                        open_outside(&address);
                    }
                }
                None
            }
            // (FUN_100b2906: the time here, in Greenwich and in the game, and the date; then "played" goes to
            // Gamecode's text commands, FUN_1003fe10, which asks the server with a `CharacterAction`)
            Parsed::Played => {
                self.push(Line::new(Kind::System, played_line()));
                Some(Out::TextCommand { action: mao_net::n3::TEXT_PLAYED, value: 0 })
            }
            Parsed::Born => Some(Out::TextCommand { action: mao_net::n3::TEXT_BORN, value: 0 }),
            Parsed::ItemsList => Some(Out::TextCommand { action: mao_net::n3::TEXT_ITEMS_LIST, value: 0 }),
            // (`/org`: Gamecode's organization commands, the line's `%` arguments put in first as for the others)
            Parsed::OrgCommand { text } => match crate::orgcommands::step(&self.expand(&text)) {
                crate::orgcommands::Step::Say(line) => {
                    self.tell(&line);
                    None
                }
                crate::orgcommands::Step::Send { command, targeted, text } => Some(Out::OrgCommand { command, targeted, text }),
                crate::orgcommands::Step::Ask(question) => {
                    crate::orgboxes::ask(question);
                    None
                }
                crate::orgcommands::Step::Nothing => None,
            },
            // (`/raid`: `raidcommands.rs`. `listlocal` says the team's members here and sends nothing (FUN_10066446:
            // nothing at all outside a team; the six teams of a raid it would list are not kept by this client))
            Parsed::RaidCommand { text } => match crate::raidcommands::step(&self.expand(&text)) {
                crate::raidcommands::Step::Nothing => None,
                crate::raidcommands::Step::ListLocal => {
                    let names = crate::team::with(|t| t.members.iter().map(|m| m.name.clone()).collect::<Vec<_>>());
                    if !names.is_empty() {
                        self.tell("Team members:");
                        for name in names {
                            self.tell(&name);
                        }
                    }
                    None
                }
                crate::raidcommands::Step::Send { command, ids } => Some(Out::RaidCommand { command, ids }),
            },
            // (FUN_1003fe10's `team`: not in a team, said so; alone, its three lines of help; `loot` asks for the
            // order, and with a word after it the leader sets it, the word's number sent whatever it is: its table
            // has team 1, loot 2, all 3, leader 4, alpha 5, and nothing for any other word. Another first word
            // does nothing.)
            Parsed::TeamCommand { words } => {
                let (in_team, leader) = crate::team::with(|t| (!t.members.is_empty(), t.members.iter().position(|m| m.id == t.own).is_some_and(|k| crate::team::is_leader(t, k))));
                let word = |w: &str| ["team", "loot", "all", "leader", "alpha"].iter().position(|k| *k == w).map_or(0, |p| p as u32 + 1);
                match words.as_slice() {
                    _ if !in_team => {
                        self.tell(&crate::feedback::plain("Feedback_YouAreNotMemberOfTeam"));
                        None
                    }
                    [] => {
                        for name in ["Feedback_AvailableTeamCommands", "Feedback_TeamLoot", "Feedback_TeamLootAll"] {
                            self.tell(&crate::feedback::plain(name));
                        }
                        None
                    }
                    [first, ..] if word(first) != 2 => None,
                    [_] => Some(Out::TextCommand { action: mao_net::n3::TEXT_TEAM_LOOT, value: 0 }),
                    [_, order, ..] if leader => Some(Out::TextCommand { action: mao_net::n3::TEXT_TEAM_LOOT_SET, value: word(order) }),
                    _ => {
                        self.tell(&crate::feedback::plain("Feedback_OnlyTeamLeaderCanChangeLootOrder"));
                        None
                    }
                }
            }
            // (FUN_1003fe10's `bank`: `info` asks the server; `open` is a game master's; `close` shuts the bank's
            // window, which this client leaves to the window; any other word does nothing)
            // (`close` shuts the bank's window, in the client alone; `open` is a game master's)
            Parsed::Bank { word } => match word.as_str() {
                "info" => Some(Out::TextCommand { action: mao_net::n3::TEXT_BANK_INFO, value: 0 }),
                "close" => Some(Out::Window { name: "bank".into(), open: false }),
                _ => None,
            },
            Parsed::RolePlay => Some(Out::RolePlay),
            Parsed::NameRequest { name } => Some(Out::NameRequest { name: self.expand(&name) }),
            // (`N3Msg_EnableGFX`: the template's number in or out of the set of switched-off templates; the original
            // says nothing)
            Parsed::Gfx { on, number } => {
                mao_formats::gfx::set_enabled(number, on);
                None
            }
            // (FUN_100b6019 asks in a box and then hands the text to `BugReport_t::SendUserMadeBugReport`, the
            // original's report to its makers about their own client: not sent from this one, and nothing is said)
            Parsed::Bug => None,
            Parsed::Nothing => None,
            // (FUN_100b6a54: both distances, set without a word)
            Parsed::BothDist { characters, share } => {
                self.outs.push(Out::CharDist(characters));
                Some(Out::ViewDist(share))
            }
            // (`/afk`: FUN_100b82a7)
            Parsed::Afk { text } => {
                let names = self.names.clone();
                let did = self.away.command(&text, &|t| chat_commands::expand(t, &names), std::time::Instant::now());
                self.away_did(did)
            }
            // (`/messagebox`: a `DialogBox_c` with the text and OK, put in the screen's middle)
            Parsed::MessageBox { text } => {
                crate::chrome::alert(self.expand(&text));
                None
            }
            Parsed::PrivateByName { what, name } => Some(Out::PrivateByName { what, name: self.expand(&name) }),
            // (FUN_100ba6f4: exactly `info` and one more word is the request about that character; anything else is
            // the chat server's command, each word expanded on its own)
            Parsed::Cc { words } => {
                if words.len() == 2 && words[0].eq_ignore_ascii_case("info") {
                    Some(Out::CcInfo { name: self.expand(&words[1]) })
                } else {
                    Some(Out::Cc { words: words.iter().map(|w| self.expand(w)).collect() })
                }
            }
            Parsed::Assist { name } => Some(Out::Assist { name: name.map(|n| self.expand(&n)) }),
            // (FUN_100b9966: the waypoint's signal, nothing said)
            Parsed::Waypoint { x, z } => Some(Out::Waypoint { x, z }),
            Parsed::SelectSelf => Some(Out::SelectSelf),
            Parsed::SetOption { name, value } => Some(Out::SetOption { name, value }),
            Parsed::InspectEquipment { id } => {
                crate::charinfo::ask_equipment(id);
                None
            }
            // (FUN_100b69a7, FUN_100b68dc: the value set without a word and without limits; with nothing after the
            // command, the error line FUN_1009bad0 writes in colour 0x51)
            Parsed::ViewDist { share: Some(share) } => Some(Out::ViewDist(share)),
            Parsed::CharDist { distance: Some(d) } => Some(Out::CharDist(d)),
            Parsed::CharDist { distance: None } | Parsed::ViewDist { share: None } | Parsed::TooFewArguments => {
                self.push(Line::new(Kind::System, "Error: too few arguments."));
                None
            }
            Parsed::Ignore { name } => {
                match ignore_request(&name, &self.names) {
                    IgnoreRequest::TooMany => self.push(Line::new(Kind::System, command_text("ChatCmdFeedback_Ignore_ToManyArguments"))),
                    IgnoreRequest::NoTarget => self.push(Line::new(Kind::System, command_text("ChatCmdFeedback_Ignore_NoTargetOrNick"))),
                    IgnoreRequest::List => {
                        // (a line of the program's own, then a line for each)
                        self.push(Line::new(Kind::System, "The characters you ignore:".to_string()));
                        for ignored in self.ignored.clone() {
                            self.push(Line::new(Kind::System, ignored));
                        }
                    }
                    IgnoreRequest::Toggle(name) => self.toggle_ignore(&name),
                }
                None
            }
            Parsed::Befriend { name } => Some(Out::Befriend { name }),
            // (FUN_1009cdfe, FUN_1009d028: the first channel of the organization's kind, of the team's)
            Parsed::Org { text } => match chatgroups::of_kind(3) {
                Some(group) => self.say_in(pane, &group, self.expand(&text)),
                None => {
                    self.tell(&command_text("ChatCmdFeedback_O_YouAreNotMemberOfAnOrganizaion"));
                    None
                }
            },
            Parsed::Team { text } => match chatgroups::of_kind(0x82) {
                Some(group) => self.say_in(pane, &group, self.expand(&text)),
                None => {
                    self.tell(&command_text("ChatCmdFeedback_T_YouAreNotMemberOfATeam"));
                    None
                }
            },
            Parsed::Duel { what } => Some(Out::Duel(what.unwrap_or(0))),
            Parsed::PetDuel { what } => Some(Out::PetDuel(what.unwrap_or(0))),
            Parsed::Output { word, name } => self.group_command(pane, &word, &name, None),
            Parsed::Group { word, name, text } => self.group_command(pane, &word, &name, Some(self.expand(&text))),
            Parsed::Vicinity { command, text } => Some(Out::Vicinity { kind: match command {
                "whisper" => 1,
                "shout" => 2,
                "emote" => 3,
                _ => 0,
            }, text: self.expand(&text) }),
            Parsed::Social { id } => Some(Out::Social { id }),
            Parsed::NoEmote(name) => {
                self.push(Line::new(Kind::System, format!("Error: No emote named '{name}'")));
                None
            }
            // (`/help`, FUN_100b7345: the page in the info view; a topic it does not know is an error line)
            Parsed::Help { topic } => {
                match chat_commands::help_file(topic.as_deref()) {
                    Some(file) => self.links.push(format!("file://{file}")),
                    None => self.push(Line::new(Kind::System, format!("Error: no help topic named '{}'.", topic.unwrap_or_default()))),
                }
                None
            }
            // (GUI.dll has no command of the name: the line goes to Gamecode's text commands (FUN_100b285d ->
            // `N3Msg_TextCommand`), whose FUN_1003fe10 asks the server with `CharacterAction` 0x9d and says nothing itself)
            Parsed::Version => Some(Out::TextCommand { action: mao_net::n3::TEXT_VERSION, value: 0 }),
            Parsed::Unsupported(name) => {
                self.push(Line::new(Kind::System, format!("/{name} is not carried out by this client yet.")));
                None
            }
            // (a command the list does not have is a script file's name, or nothing: FUN_100b1517 says which)
            Parsed::Unknown(_) => {
                self.run_script(line.trim());
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
            if self.focus_tries > 0 && self.focus_frame.as_deref() == Some(frame) {
                self.focus_tries -= 1;
            } else {
                self.focus_frame = None;
            }
            // (the cursor after what is already there: `/` starts the line with its slash)
            let mut state = egui::TextEdit::load_state(ctx, input_id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(self.input.chars().count()))));
            state.store(ctx, input_id);
        }
        let suggestions = chat_commands::suggest_with(&self.input, &crate::scripts::names());
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
            // (the line sent, the keys are the game's again: Enter starts the next one; with the window's
            // "Deactivate On Send" off the bar stays typed in, FUN_100ab37b)
            let line = std::mem::take(&mut self.input);
            out = self.run_in(pane, &line);
            if self.panes.get(pane).is_none_or(|p| p.look.deactivate) {
                response.surrender_focus();
            } else {
                response.request_focus();
            }
        }
        // The list of commands, above the input line (the client's own: the original completes in its field, FUN_100a44e8;
        // CHT-035). It scrolls (the wheel, or its bar) over every command that matches, keeps the highlighted one in
        // view when the keys move it, and a press on a row takes that command as Tab does (on the press: the field
        // loses the keys when the button is let go, and the list goes with them).
        if (has_focus || response.has_focus() || self.keep_list) && !suggestions.is_empty() {
            // (at most 15 rows, fewer where the screen above the bar holds fewer; its foot just above the bar)
            let fits = (((response.rect.top() - 14.0) / 15.0).max(3.0)) as usize;
            let rows = suggestions.len().min(fits).min(15);
            let mut picked = None;
            egui::Area::new(input_id.with("commands")).order(egui::Order::Foreground).pivot(egui::Align2::LEFT_BOTTOM).fixed_pos(egui::pos2(response.rect.left(), response.rect.top() - 4.0)).show(ctx, |ui| {
                egui::Frame::new().fill(egui::Color32::from_rgb(8, 40, 50)).stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(20, 90, 100))).inner_margin(4.0).show(ui, |ui| {
                    egui::ScrollArea::vertical().id_salt("command-list").max_height(rows as f32 * 15.0).auto_shrink([true, true]).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (n, command) in suggestions.iter().enumerate() {
                            let chosen = n == self.selected;
                            let (rect, row) = ui.allocate_exact_size(egui::vec2(150.0, 15.0), egui::Sense::click());
                            if chosen || row.hovered() {
                                let fill = if chosen { egui::Color32::from_rgb(30, 30, 200) } else { egui::Color32::from_rgb(20, 70, 90) };
                                ui.painter().rect_filled(rect, 0.0, fill);
                            }
                            ui.painter().text(rect.left_center() + egui::vec2(3.0, 0.0), egui::Align2::LEFT_CENTER, format!("/{command}"), egui::FontId::proportional(12.0), egui::Color32::from_rgb(220, 240, 245));
                            if chosen && (up || down) {
                                row.scroll_to_me(None);
                            }
                            if row.hovered() && ctx.input(|i| i.pointer.primary_pressed()) {
                                picked = Some(n);
                            }
                        }
                    });
                });
            });
            if let Some(n) = picked {
                self.selected = n;
                self.input = format!("/{} ", suggestions[n]);
                response.request_focus();
                if let Some(mut state) = egui::TextEdit::load_state(ctx, input_id) {
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(self.input.chars().count()))));
                    state.store(ctx, input_id);
                }
            }
        }
        if let Some(mut own) = own {
            std::mem::swap(&mut self.input, &mut own);
            self.inputs.insert(frame.to_string(), own);
        }
        out
    }

    /// Whether a frame's chat is being typed in, or is about to be.
    fn frame_typing(&self, ctx: &egui::Context, frame: &str, main: bool) -> bool {
        ctx.memory(|m| m.has_focus(bar_id(frame))) || (main && self.focus_next) || self.focus_frame.as_deref() == Some(frame)
    }

    /// A frame of chat windows inside its window on screen: the tabs (none when `bare`), the shown window's lines and
    /// its input bar. `n` is the frame's number, 0 the chat window itself.
    #[allow(clippy::too_many_arguments)]
    fn frame_body(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, palette: &Palette, n: usize, frame: &str, members: &[usize], bare: bool, clicks: &mut Vec<(Option<String>, crate::gui::controls::LinkClick)>, menu_at: &mut Option<(egui::Pos2, usize)>) -> Option<Out> {
        let mut on_tab = false;
        let main = n == 0;
        let mut pressed = Vec::new();
        let Some(&lead) = members.first() else { return None };
        let mut shown = if main { self.tab } else { self.shown.get(frame).copied().unwrap_or(lead) };
        if !members.contains(&shown) {
            shown = lead;
        }
        let area = ui.max_rect();
        let typing = ctx.memory(|m| m.has_focus(bar_id(frame))) || (main && self.focus_next) || self.focus_frame.as_deref() == Some(frame);
        let look = self.panes.get(shown).map(|p| p.look).unwrap_or_default();
        // (the input bar: none with `is_textinput_enabled` off, and with `hide_input_when_inactive` only while typing;
        // Enter still starts a line in the chat window itself)
        let bar = (look.input && (!look.hide_input || typing)) || (main && self.focus_next);
        // (`is_message_fading_enabled`: a line fades once it is `ChatTextFadeDelay` old, whether the window is active or
        // not: FUN_10098f99 and FUN_1009b4ff switch the fading by that setting alone, FUN_1009002a)
        let fade = look.fade.then_some(self.fade_times);
        let below = if bar { 41.0 } else { 4.0 };
        if bare {
            let height = (ui.available_height() - below).max(40.0);
            draw_lines(ui, palette, &self.lines, &self.born, self.panes.get(shown), None, height, fade, &mut pressed);
        } else {
            // (each tab one of the frame's chat windows: the lines of the groups selected in it)
            let names: Vec<String> = members.iter().map(|k| self.panes[*k].name.clone()).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let mut tab = members.iter().position(|k| *k == shown).unwrap_or(0);
            let mut hits = crate::chrome::TabHits::default();
            crate::chrome::tab_view_hits(ui, palette, &names, &mut tab, &mut hits, |ui, tab| {
                shown = members.get(tab).copied().unwrap_or(lead);
                let height = (ui.available_height() - below).max(40.0);
                draw_lines(ui, palette, &self.lines, &self.born, self.panes.get(shown), None, height, fade, &mut pressed);
            });
            // (the right button let go on a tab takes it up, `ChatWindow_c`'s FUN_10097af7; the window's menu does not come)
            if let (Some(i), None) = (hits.right_released, self.carried) {
                let grab = ui.input(|inp| inp.pointer.interact_pos()).zip(hits.tabs.get(i)).map_or(egui::Vec2::ZERO, |(at, rect)| at - rect.min);
                self.carried = members.get(i).map(|&p| (p, grab));
                on_tab = true;
            }
            if let Some(strip) = hits.strip {
                self.strips.push((frame.to_string(), strip, members.iter().copied().zip(hits.tabs).collect()));
            }
        }
        // (the right button on a name is the name's menu, not the window's: FUN_1008fa92)
        let on_name = on_tab || pressed.iter().any(|c| c.secondary && linked_name(&c.link).is_some());
        clicks.extend(pressed.into_iter().map(|c| ((!main).then(|| frame.to_string()), c)));
        if main {
            self.tab = shown;
        } else {
            self.shown.insert(frame.to_string(), shown);
        }
        let pointer = ui.input(|i| i.pointer.interact_pos()).filter(|p| area.contains(*p));
        // (the right button over the window: its menu, FUN_10097a40)
        if !on_name
            && ui.input(|i| i.pointer.secondary_clicked())
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

    /// A tell window's inside: the tells with its character and the input bar, whose plain lines are tells to them.
    fn tell_body(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, palette: &Palette, name: &str, frame: &str, clicks: &mut Vec<(Option<String>, crate::gui::controls::LinkClick)>) -> Option<Out> {
        let height = (ui.available_height() - 41.0).max(40.0);
        let mut pressed = Vec::new();
        draw_lines(ui, palette, &self.lines, &self.born, None, Some(name), height, None, &mut pressed);
        clicks.extend(pressed.into_iter().map(|c| (Some(frame.to_string()), c)));
        self.telling = Some(name.to_string());
        let out = self.input_bar(ui, ctx, frame, false, usize::MAX);
        self.telling = None;
        out
    }

    /// A tab carried (taken up by the right button on it): its picture follows the pointer, held where it was taken
    /// (`DragObject_c::StartDragging`, 0x1012ba1c), until the left button puts it down:
    /// - on a tab of a frame's strip, before that tab; on the strip past the tabs, last (FUN_10098038 with
    ///   `TabView::SlotItemDropped` 0x10145a95); a frame left without tabs is gone;
    /// - anywhere else, into a window of its own whose corner is there (FUN_100984c2), but not the only tab of its frame.
    ///
    /// Escape lets go of it (this client's own: how the original drops a drag otherwise was not read). Which button
    /// takes a tab up is as the decompile reads (`Tab` FUN_10146ef3 begins on the second button let go on it), not seen live.
    fn carry(&mut self, ctx: &egui::Context, palette: &Palette) {
        let strips = std::mem::take(&mut self.strips);
        let Some((pane, grab)) = self.carried else { return };
        let Some(name) = self.panes.get(pane).map(|p| p.name.clone()) else {
            self.carried = None;
            return;
        };
        ctx.request_repaint();
        let Some(at) = ctx.input(|i| i.pointer.latest_pos()) else { return };
        // (the tab's picture: an active tab with its title, `TabView::CreateDragImage` 0x10146b1e)
        egui::Area::new(egui::Id::unique("chat-tab-carried")).order(egui::Order::Tooltip).interactable(false).fixed_pos(at - grab).show(ctx, |ui| {
            let font = crate::font::verdana(ctx, 13.0);
            let width = ui.painter().layout_no_wrap(name.clone(), font.clone(), egui::Color32::WHITE).size().x.ceil() + 22.0;
            let skin = crate::skin::skin(ctx);
            let height = skin.as_ref().and_then(|s| s.get("GFX_GUI_TAB_ACTIVE_MIDDLE")).map_or(18.0, |s| s.size.y);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
            if !skin.is_some_and(|s| s.three_slice(ui.painter(), "GFX_GUI_TAB_ACTIVE", rect, crate::chrome::rgb(palette.accent))) {
                ui.painter().rect_filled(rect, 3.0, crate::chrome::rgb(palette.panel));
            }
            ui.painter().text(egui::pos2(rect.left() + 5.0, rect.center().y), egui::Align2::LEFT_CENTER, &name, font, egui::Color32::WHITE);
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.carried = None;
            return;
        }
        if !ctx.input(|i| i.pointer.primary_clicked()) {
            return;
        }
        self.carried = None;
        let source = self.panes[pane].frame.clone();
        let shown = self.panes.get(self.tab).map(|p| p.name.clone());
        match strips.iter().find(|s| s.1.contains(at)) {
            Some((frame, _, tabs)) => {
                // (put down on itself: nothing moves)
                if tabs.iter().any(|t| t.0 == pane && t.1.contains(at)) {
                    return;
                }
                // (before the tab it is put on, or after the frame's last)
                let before = tabs.iter().find(|t| t.1.contains(at)).map(|t| t.0).filter(|&t| t != pane).and_then(|t| self.panes.get(t)).map(|p| p.name.clone());
                let last = chatgroups::members(&self.panes, frame).last().and_then(|&k| self.panes.get(k)).map(|p| p.name.clone());
                let mut moved = self.panes.remove(pane);
                moved.frame = frame.clone();
                let at_index = match before.or(last) {
                    Some(n) => self.panes.iter().position(|p| p.name == n).map_or(self.panes.len(), |k| if tabs.iter().any(|t| t.1.contains(at)) { k } else { k + 1 }),
                    None => self.panes.len(),
                };
                self.panes.insert(at_index.min(self.panes.len()), moved);
            }
            None => {
                if chatgroups::members(&self.panes, &source).len() < 2 {
                    return;
                }
                let frame = chatgroups::free_name(&self.panes);
                crate::layout::place(&frame, at - grab);
                self.panes[pane].frame = frame;
            }
        }
        if let Some(k) = shown.and_then(|n| self.panes.iter().position(|p| p.name == n)) {
            self.tab = k;
        }
        chatgroups::save(&self.panes);
    }

    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<Out> {
        // (the tell windows asked for whose names the chat server knew)
        for name in chatgroups::take_tells() {
            self.open_tell(&name);
        }
        // (the tell windows are windows of their own: they stay when the chat window is closed; so does a chat window's
        // menu the friends list opened)
        // (the invitations come whether or not the chat window is up: their marks are the friends list's)
        self.take_invitations();
        if !self.open && self.tells.is_empty() && self.popup.is_none() && !self.invites.iter().any(|i| i.asked.is_some()) {
            return None;
        }
        let mut out = None;
        let mut links = Vec::new();
        let mut clicks: Vec<(Option<String>, crate::gui::controls::LinkClick)> = Vec::new();
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
            if !self.open {
                break;
            }
            let members = chatgroups::members(&self.panes, frame);
            let Some(&lead) = members.first() else { continue };
            let name = if n == 0 { "Chat".to_string() } else { format!("Chat:{frame}") };
            let look = self.panes[lead].look;
            // (`visual_mode` 2: a window alone in its frame has no frame or title, and fades as a whole to its setting
            // for a window typed in or for one that is not, FUN_100972da; a framed one fades as every window does)
            let bare = look.borderless && members.len() == 1;
            // (the original's first places, `first_place`; a borderless window's frame is the bare window's margins)
            let (at, size) = first_place(if n == 0 { FirstPlace::Default } else { FirstPlace::Other }, screen, if bare { egui::vec2(8.0, 8.0) } else { crate::chrome::frame_room() });
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
                let fade = crate::chrome::chat_fade(self.frame_typing(ctx, frame, n == 0), f32::from(look.solid.0) / 100.0, f32::from(look.solid.1) / 100.0);
                crate::chrome::bare_window(ctx, palette, &name, fade, build, |ui| sent = self.frame_body(ui, ctx, palette, n, frame, &members, true, &mut clicks, &mut menu_at));
            } else {
                let open = if n == 0 { &mut open } else { &mut stays };
                crate::chrome::window(ctx, palette, &name, "Chat", open, build, |ui| sent = self.frame_body(ui, ctx, palette, n, frame, &members, false, &mut clicks, &mut menu_at));
            }
            out = out.or(sent);
            if !stays {
                for k in members {
                    self.panes[k].frame = frames[0].clone();
                }
                docked = true;
            }
        }
        // the tell windows, each titled with its character
        let mut closed = Vec::new();
        for (slot, name) in self.tells.clone() {
            let frame = Self::tell_frame(slot);
            let id = format!("Chat:{frame}");
            // (in the middle of the screen until the player puts it somewhere, `first_place`)
            let (middle, size) = first_place(FirstPlace::Tell, screen, crate::chrome::frame_room());
            let place = crate::layout::pos(&id, middle);
            let (mut stays, mut sent) = (true, None);
            crate::chrome::window(ctx, palette, &id, &name, &mut stays, move |w| w.default_pos(place).default_size(size), |ui| sent = self.tell_body(ui, ctx, palette, &name, &frame, &mut clicks));
            if self.raise == Some(slot) {
                ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, egui::Id::unique(&id)));
            }
            out = out.or(sent);
            if !stays {
                closed.push(slot);
                self.inputs.remove(&frame);
            }
        }
        self.raise = None;
        self.tells.retain(|t| !closed.contains(&t.0));
        self.scroll_to_end = false;
        if docked {
            chatgroups::save(&self.panes);
        }
        if let Some((at, pane)) = menu_at {
            self.menu_pane = pane;
            self.open_menu(ctx, at, None);
        }
        self.carry(ctx, palette);
        // the presses on the lines' links: a character's name (FUN_1008fa92, FUN_100ac4c4), else the info view's
        for (frame, click) in clicks {
            match linked_name(&click.link).map(str::to_string) {
                Some(name) if click.secondary => self.open_name_menu(ctx, &name, frame, click.at),
                Some(name) if click.qualified => self.type_in(frame.as_deref(), &format!("/tell {name} ")),
                Some(name) => self.ask_tell(&name),
                None if !click.secondary
                    && let Some(group) = linked_group(&click.link) =>
                {
                    let group = group.to_string();
                    self.group_pressed(frame.as_deref(), &group, click.qualified);
                }
                None if !click.secondary => links.push(click.link),
                None => {}
            }
        }
        if let Some((_, menu)) = &self.demo
            && let Some(name) = menu.strip_prefix("name:")
        {
            self.demo_name = Some(name.to_string());
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
            let sub = match menu.as_str() {
                "main" => Some(None),
                "talk" => Some(Some(Menu::Talk)),
                "public" => Some(Some(Menu::Groups { combat: false })),
                "combat" => Some(Some(Menu::Groups { combat: true })),
                "visual" => Some(Some(Menu::Visual)),
                "mode" => Some(Some(Menu::Mode)),
                "solid" => Some(Some(Menu::Solid)),
                _ => None,
            };
            if let Some(sub) = sub {
                self.open_menu(ctx, egui::pos2(420.0, 120.0), sub);
            }
            // (`name:<character>`: the menu of a name's right button)
            if let Some(name) = self.demo_name.take() {
                self.open_name_menu(ctx, &name, None, egui::pos2(160.0, ctx.content_rect().height() - 215.0));
            }
        }
        match crate::chrome::menu_path(ctx, palette, "chat-menu", &mut self.popup) {
            Some(crate::chrome::Picked::Entry(path)) => {
                if let Some((nodes, at)) = self.menu.take()
                    && let Some(node) = node_at(&nodes, &path)
                {
                    let (menu, key) = (node.menu, node.key.clone());
                    self.choose(ctx, menu, &key, at);
                }
            }
            // (a slider of "how solid": the window follows the knob, and the menu stays)
            Some(crate::chrome::Picked::Slider(path, value, done)) => {
                let which = self.menu.as_ref().and_then(|(nodes, _)| node_at(nodes, &path)).filter(|n| n.menu == Menu::Solid).map(|n| n.key == "active");
                let tab = self.menu_pane.min(self.panes.len().saturating_sub(1));
                if let (Some(active), Some(pane)) = (which, self.panes.get_mut(tab)) {
                    let hundredths = (value * 100.0).round().clamp(0.0, 100.0) as u8;
                    if active { pane.look.solid.0 = hundredths } else { pane.look.solid.1 = hundredths }
                    if done {
                        chatgroups::save(&self.panes);
                    }
                }
            }
            None => {}
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        self.config.own = self.names.own.clone();
        if self.config.show(ctx, palette, &mut self.panes, &mut self.tab, &mut self.log) {
            chatgroups::save(&self.panes);
        }
        for name in self.config.leaves.drain(..) {
            self.outs.push(Out::PrivateLeave { name });
        }
        if std::mem::take(&mut self.config.disband) {
            self.outs.push(Out::PrivateDisband);
        }
        // an invitation to a private chat group: the original's question (FUN_100a76c9) with its list of the chat windows
        // to put the group in
        if let Some(address) = self.start_pending.clone() {
            let question = crate::gui::xml::ldb_named(10000, "ReallyMinimizeGame", "Opening this leaves the game's full screen. Go on?");
            if let Some(yes) = crate::chrome::question(ctx, palette, "start-minimize", "Warning:", &question) {
                if yes {
                    open_outside(&address);
                }
                self.start_pending = None;
            }
        }
        if let Some(k) = self.invites.iter().position(|i| i.asked.is_some()) {
            let name = self.invites[k].name.clone();
            let mut chosen = self.invites[k].asked.take().unwrap_or_default();
            let answer = invitation_box(ctx, palette, &name, &self.panes, &mut chosen);
            self.invites[k].asked = Some(chosen);
            if let Some(yes) = answer {
                let invite = self.invites.remove(k);
                let owner = invite.owner;
                // (FUN_100a74cb: yes puts the group into the windows chosen and out of the others, then joins)
                if yes {
                    put_group(&mut self.panes, &invite.name, &invite.asked.unwrap_or_default());
                    chatgroups::save(&self.panes);
                }
                self.outs.push(if yes { Out::PrivateJoin { owner } } else { Out::PrivateDecline { owner } });
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

    /// FUN_100971a3 / FUN_1009829a / FUN_100b243e: the default chat window across the screen's foot 224 in from each side
    /// and 3 above the bottom; another chat window a 400 by 200 client, a tell window a 389 by 219 one, in the middle.
    #[test]
    fn chat_windows_first_go_where_the_originals_do() {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));
        let frame = egui::vec2(14.0, 34.0);
        assert_eq!(first_place(FirstPlace::Default, screen, frame), (egui::pos2(224.0, 520.0), egui::vec2(818.0, 163.0)));
        assert_eq!(first_place(FirstPlace::Other, screen, frame), (egui::pos2(433.0, 243.0), egui::vec2(400.0, 200.0)));
        assert_eq!(first_place(FirstPlace::Tell, screen, frame), (egui::pos2(438.5, 233.5), egui::vec2(389.0, 219.0)));
    }

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
        // (and `/voice` speaks there too, FUN_100b88b1's selected group)
        assert_eq!(w.run("/voice heal"), Some(Out::Voice { line: "heal".into(), channel: Some("Yy Clan Chat".into()) }));
        assert_eq!(ChatWindow::default().run("/voice heal"), Some(Out::Voice { line: "heal".into(), channel: None }));
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
        assert_eq!(w.lines.last().unwrap().text, "To [Abc]: hello");
        assert!(w.lines.last().unwrap().markup.as_deref().is_some_and(|m| m.contains("href=\"user://Abc\"")));
        assert_eq!(w.run("/r anyone"), None); // nobody has told us anything yet
        w.push(Line { kind: Kind::Tell, text: "Zed: hi".into(), from: Some("Zed".into()), markup: None, group: String::new(), at: None });
        assert_eq!(w.run("/r hello Zed"), Some(Out::Tell { name: "Zed".into(), text: "hello Zed".into() }));
    }

    #[test]
    fn the_target_is_told_and_its_tell_window_asked_for() {
        let mut w = ChatWindow::default();
        w.names = chat_commands::Names { own: "Me".into(), target: Some("Abc".into()), fighting: None };
        // (`%t` is the target, in the name and in the text)
        assert_eq!(w.run("/tell %t hello %t"), Some(Out::Tell { name: "Abc".into(), text: "hello Abc".into() }));
        assert_eq!(w.run("look at %t"), Some(Out::Vicinity { kind: 0, text: "look at Abc".into() }));
        // (a name without a text: the tell window, once the server knows the name; the command alone does nothing,
        // FUN_100ba8d5, target or not)
        assert_eq!((w.run("/tell %t"), w.run("/tell"), w.run("/tell Zed")), (None, None, None));
        assert_eq!(w.outs, vec![Out::TellWindow { name: "Abc".into() }, Out::TellWindow { name: "Zed".into() }]);
        // (none to oneself)
        w.outs.clear();
        w.names.target = None;
        let before = w.lines.len();
        assert_eq!((w.run("/tell me hi"), w.run("/tell %m"), w.run("/tell")), (None, None, None));
        assert!(w.outs.is_empty());
        assert_eq!(w.lines.len(), before + 2);
    }

    #[test]
    fn a_tell_window_holds_the_tells_with_its_character() {
        let mut w = ChatWindow::default();
        w.names.own = "Me".into();
        w.push(Line { kind: Kind::Tell, text: "[Zed]: hi".into(), from: Some("Zed".into()), markup: None, group: String::new(), at: None });
        w.push(Line { kind: Kind::Tell, text: "[Abc]: hello".into(), from: Some("Abc".into()), markup: None, group: String::new(), at: None });
        w.open_tell("zed");
        w.open_tell("Zed");
        assert_eq!(w.tell_windows(), vec!["zed"]);
        // (asked for again while open, it is brought up without another question to the server)
        w.ask_tell("ZED");
        assert!(w.outs.is_empty());
        // (a plain line typed in it is a tell; a command is a command)
        w.telling = Some("Zed".into());
        assert_eq!(w.run_in(usize::MAX, "hello there"), Some(Out::Tell { name: "Zed".into(), text: "hello there".into() }));
        assert_eq!(w.run_in(usize::MAX, "/tell Abc psst"), Some(Out::Tell { name: "Abc".into(), text: "psst".into() }));
        w.telling = None;
        // (it shows the tells from and to its character, old and new; a chat window shows them still)
        let shown: Vec<&str> = w.lines.iter().filter(|l| line_shows(l, None, Some("Zed"))).map(|l| l.text.as_str()).collect();
        assert_eq!(shown, vec!["[Zed]: hi", "To [Zed]: hello there"]);
        assert!(w.lines.iter().all(|l| line_shows(l, w.panes.first(), None)));
    }

    #[test]
    fn a_name_in_a_line_is_a_link() {
        assert_eq!(linked_name("user://Abc"), Some("Abc"));
        assert_eq!(linked_name("USER://Abc"), Some("Abc"));
        assert_eq!((linked_name("user://"), linked_name("itemref://1/2/3"), linked_name("us")), (None, None, None));
        let line = Line::new(Kind::Channel, "[Trade] Abc: hi").with_markup(&format!("[Trade] {}: ", name_link("Abc")), "hi");
        assert_eq!(line.markup.as_deref(), Some("[Trade] <a style=\"text-decoration:none\" href=\"user://Abc\">Abc</a>: hi"));
    }

    #[test]
    fn the_filter_hides_group_lines_and_scripts_run_a_line_a_frame() {
        let mut w = ChatWindow::default();
        w.filter = Default::default();
        let said = |who: &str, text: &str| Line { kind: Kind::Channel, text: format!("[Trade] {who}: {text}"), from: Some(who.into()), markup: None, group: "Trade".into(), at: None };
        let lines = |w: &ChatWindow| w.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>();
        // (the rules are tried on what was said, not on the name before it; tells are not looked at)
        w.filter.add("^WTS");
        w.filter.add("credits");
        w.push(said("Zed", "WTS a pistol"));
        w.filter.enabled = true;
        w.push(said("Zed", "WTS a rifle"));
        w.push(said("WTSman", "hello all"));
        w.push(said("Abc", "cheap credits here"));
        w.push(Line { kind: Kind::Tell, text: "[Abc]: credits?".into(), from: Some("Abc".into()), markup: None, group: String::new(), at: None });
        assert_eq!(lines(&w), ["[Trade] Zed: WTS a pistol", "[Trade] WTSman: hello all", "[Abc]: credits?"]);
        // (a script's lines: one each frame, a delay waited out, none of them kept among the typed lines)
        w.lines.clear();
        w.scripts.add(vec!["/text one".into(), "/delay 100".into(), "/text two".into()]);
        w.script_frame(0.05);
        assert_eq!(lines(&w), ["one"]);
        for _ in 0..3 {
            w.script_frame(0.05);
        }
        assert_eq!(lines(&w), ["one"]);
        w.script_frame(0.05);
        assert_eq!(lines(&w), ["one", "two"]);
        assert!(w.history.is_empty());
        // (the commands that only make a request)
        assert_eq!(w.run("/played"), Some(Out::TextCommand { action: 0x85, value: 0 }));
        let time = &w.lines.last().unwrap().text;
        assert!(time.starts_with("Time: ") && time.contains("Date: "), "{time}");
        assert_eq!(w.run("/born"), Some(Out::TextCommand { action: 0x86, value: 0 }));
        // (CHT-002: `/version` asks the server, FUN_1003fe10 at 0x1003fec0)
        assert_eq!(w.run("/version"), Some(Out::TextCommand { action: 0x9d, value: 0 }));
        assert_eq!(w.run("/items"), Some(Out::TextCommand { action: 0xf4, value: 0 }));
        assert_eq!(w.run("/org info"), Some(Out::OrgCommand { command: 5, targeted: true, text: String::new() }));
        assert_eq!(w.run("/org kick Somebody"), Some(Out::OrgCommand { command: 0xd, targeted: true, text: "Somebody".into() }));
        assert_eq!(w.run("/cc addbuddy Zed"), Some(Out::Cc { words: vec!["addbuddy".into(), "Zed".into()] }));
        assert_eq!(w.run("/cc INFO Zed"), Some(Out::CcInfo { name: "Zed".into() }));
        assert_eq!(w.run("/cc info"), Some(Out::Cc { words: vec!["info".into()] }));
        assert_eq!((w.run("/rp"), w.run("/name Zed")), (Some(Out::RolePlay), Some(Out::NameRequest { name: "Zed".into() })));
        assert_eq!((w.run("/bank close"), w.run("/bank open"), w.run("/bug it broke badly, twenty letters")), (Some(Out::Window { name: "bank".into(), open: false }), None, None));
        assert_eq!((w.run("/org"), w.run("/org leave")), (None, None));
        assert_eq!(w.run("/bank info"), Some(Out::TextCommand { action: 0x5c, value: 0 }));
        assert_eq!((w.run("/bank open"), w.run("/bank")), (None, None));
        // (`/team` outside a team says so and sends nothing)
        let said = w.lines.len();
        assert_eq!(w.run("/team loot all"), None);
        assert_eq!((w.lines.len(), w.lines.last().unwrap().text.clone()), (said + 1, crate::feedback::plain("Feedback_YouAreNotMemberOfTeam")));
        assert_eq!(w.run("/tower info"), None);
        assert!(w.lines.last().unwrap().text.contains("not carried out"));
        assert_eq!(w.run("/dvalue AutoAFK"), Some(Out::DValue { name: "AutoAFK".into(), value: None }));
        assert_eq!(w.run("/char&viewdist 50 9000"), Some(Out::ViewDist(9000.0 * 0.01f32)));
        assert_eq!(w.outs, vec![Out::CharDist(50)]);
        assert_eq!(w.run("/viewdist"), None);
        assert_eq!(w.lines.last().unwrap().text, "Error: too few arguments.");
    }

    /// FUN_100926f6: a line keeps its alpha through the delay, fades over the fade time, and is gone after both; a
    /// fade time of 0 only takes it away.
    #[test]
    fn a_fading_line_goes_by_the_original_timer() {
        assert_eq!((fade_alpha(0, 8000, 300), fade_alpha(8000, 8000, 300), fade_alpha(8150, 8000, 300), fade_alpha(8300, 8000, 300), fade_alpha(8301, 8000, 300)), (Some(1.0), Some(1.0), Some(0.5), Some(0.0), None));
        assert_eq!((fade_alpha(8000, 8000, 0), fade_alpha(8001, 8000, 0)), (Some(1.0), None));
    }

    #[test]
    fn a_tell_that_comes_while_away_is_marked_and_answered() {
        let mut w = ChatWindow::default();
        w.names.own = "Tester".into();
        let tell = |from: &str| Line { kind: Kind::Tell, text: format!("[{from}]: hi"), from: Some(from.into()), markup: None, group: String::new(), at: None }.with_markup(&format!("[{}]: ", name_link(from)), "hi");
        // (not away: shown as it is)
        w.push(tell("Zed"));
        assert_eq!((w.lines.last().unwrap().text.as_str(), w.outs.len()), ("[Zed]: hi", 0));
        // (`/afk <text>`: a line to oneself and an emote to the vicinity)
        assert_eq!(w.run("/afk back at nine"), Some(Out::Vicinity { kind: 3, text: crate::afk::text("ChatCmdFeedback_AFK_IsAFK") }));
        assert!(w.away.is_away() && w.away.dialog.is_none());
        w.push(tell("Zed"));
        let shown = w.lines.last().unwrap();
        assert_eq!(shown.text, "[Zed]: hi (AFK reply sent)");
        assert!(shown.markup.as_ref().unwrap().ends_with("hi <font color=white>(AFK reply sent)</font>"));
        let [Out::AwayReply { name, text }] = &w.outs[..] else { panic!("{:?}", w.outs) };
        assert_eq!(name, "Zed");
        assert!(text.contains("Tester") && text.contains("back at nine"), "{text}");
        w.outs.clear();
        // (an automatic answer that comes is shown in its character's tell window and not answered; nor is what an
        // ignored character sends)
        w.push(Line { kind: Kind::AwayReply, text: "Zed is away".into(), from: Some("Zed".into()), markup: None, group: String::new(), at: None });
        assert!(line_shows(w.lines.last().unwrap(), None, Some("zed")) && w.outs.is_empty());
        w.toggle_ignore("Abc");
        w.push(tell("Abc"));
        assert!(w.outs.is_empty());
        // (`/afk` alone while away: back)
        assert_eq!(w.run("/afk"), Some(Out::Vicinity { kind: 3, text: crate::afk::text("ChatCmdFeedback_AFK_IsBack") }));
        assert!(!w.away.is_away());
        w.push(tell("Zed"));
        assert!(w.outs.is_empty());
        // (`/afk` alone while not away: away with the default message, and the box to change it)
        assert!(w.run("/afk").is_some());
        assert!(w.away.is_away() && w.away.dialog.is_some());
    }

    /// `/ignore`'s words: one name, `list`, the target when none, and no more than one.
    #[test]
    fn ignore_takes_one_name_the_list_or_the_target() {
        let names = chat_commands::Names { own: "Me".into(), target: Some("Zed".into()), fighting: None };
        assert_eq!(ignore_request("Abc", &names), IgnoreRequest::Toggle("Abc".into()));
        assert_eq!(ignore_request("list", &names), IgnoreRequest::List);
        assert_eq!(ignore_request("", &names), IgnoreRequest::Toggle("Zed".into()));
        assert_eq!(ignore_request("Abc Def", &names), IgnoreRequest::TooMany);
        assert_eq!(ignore_request(" ", &chat_commands::Names::default()), IgnoreRequest::NoTarget);
    }

    #[test]
    fn local_commands_only_add_lines() {
        let mut w = ChatWindow::default();
        assert_eq!(w.run("/help"), None);
        assert_eq!(w.links.last().map(String::as_str), Some("file://helpcommands.html"));
        assert_eq!(w.run("/gfx 1 on"), None);
        assert!(w.lines.is_empty());
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
        assert_eq!(shown, vec![command_text("ChatCmdFeedback_Ignore_IgnoringCharacter").replace("%s", "Zed").as_str(), "still shown"]);
        assert_eq!(w.run("/ignore zed"), None);
        w.push(Line { kind: Kind::Tell, text: "Zed tells you: back".into(), from: Some("Zed".into()), markup: None, group: String::new(), at: None });
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
        // (a line sent again leaves its old place and goes last; the history holds 200)
        w.remember(" /help ");
        assert_eq!(w.history, vec!["hello", "/help"]);
        for i in 0..250 {
            w.remember(&format!("line {i}"));
        }
        assert_eq!((w.history.len(), w.history[0].as_str()), (HISTORY, "line 50"));
        w.history = vec!["/help".into(), "hello".into()];
        w.recalled = None;
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

    /// When a character's tells count as not read (FUN_100a7394) and when reading them clears the count (FUN_100a7ca7).
    #[test]
    fn tells_not_read_are_counted_until_the_tell_window_opens() {
        let mut w = ChatWindow::default();
        let tell = |from: &str| Line { kind: Kind::Tell, text: format!("[{from}]: hi"), from: Some(from.into()), markup: None, group: String::new(), at: None };
        // (a chat window that is up shows the tells: none counts)
        w.push(tell("Zed"));
        assert_eq!(w.unread("zed"), 0);
        // (with the chat window shut, each one counts, the echo of one sent too)
        w.open = false;
        w.push(tell("Zed"));
        w.push(tell("ZED"));
        w.send_tell("Zed".into(), "back".into());
        assert_eq!(w.unread("Zed"), 3);
        // (with the window up but no window showing the tells' group, they count too)
        w.open = true;
        for p in &mut w.panes {
            p.selected.remove(chatgroups::TELLS);
        }
        w.push(tell("Abe"));
        assert_eq!(w.unread("abe"), 1);
        // (opening the tell window reads them; while it is open, none count)
        w.open_tell("zed");
        assert_eq!(w.unread("Zed"), 0);
        w.push(tell("Zed"));
        assert_eq!(w.unread("Zed"), 0);
        // (shut again, they count again)
        w.close_tell("Zed");
        assert!(!w.tell_open("zed"));
        w.push(tell("Zed"));
        assert_eq!((w.unread("Zed"), w.unread("Abe")), (1, 1));
        assert!(counts_unread(false, false) && !counts_unread(true, false) && !counts_unread(false, true));
    }

    /// The friends list's menu of a chat window (FUN_100a8a69): the original's entries in its order, those this client
    /// cannot carry out drawn but not to be chosen.
    #[test]
    fn a_chat_windows_menu_from_the_friends_list() {
        let mut w = ChatWindow::default();
        w.menu_pane = 1;
        let nodes = w.window_menu_tree();
        let keys: Vec<(String, bool)> = nodes.iter().map(|n| (n.key.clone(), n.entry.enabled)).collect();
        let want = [("visual", true), ("", true), ("", false), ("noinput", true), ("deactivate", true), ("", true), ("", false), ("auto", true), ("", true), ("show", true), ("", false), ("log", true), ("", true), ("rename", true), ("delete", true), ("clone", true), ("new", true)];
        assert_eq!(keys, want.iter().map(|(k, e)| (k.to_string(), *e)).collect::<Vec<_>>());
        assert!(nodes[13].entry.label.contains("Combat"));
        // (its own entries: the window shown or not, logged or not, copied into a frame of its own)
        let ctx = egui::Context::default();
        w.choose(&ctx, Menu::Window, "show", egui::Pos2::ZERO);
        w.choose(&ctx, Menu::Window, "log", egui::Pos2::ZERO);
        w.choose(&ctx, Menu::Window, "clone", egui::Pos2::ZERO);
        assert!(!w.open && w.log);
        // (deactivating on send, on as the original's windows come, turned off for the window)
        assert!(w.panes[1].look.deactivate);
        w.choose(&ctx, Menu::Visual, "deactivate", egui::Pos2::ZERO);
        assert!(!w.panes[1].look.deactivate);
        assert_eq!(w.panes.len(), 3);
        assert_eq!((w.panes[2].selected.clone(), w.panes[2].frame.clone()), (w.panes[1].selected.clone(), w.panes[2].name.clone()));
    }

    /// A channel's name in a line is a link (FUN_1009bc20); pressing it picks the channel to talk to in a window that
    /// shows it, else types `/group` and the name (FUN_100ac4f0).
    #[test]
    fn a_channels_name_is_a_link() {
        assert!(group_link("Yy Shopping").contains("href=\"chatgroup://Yy Shopping\""));
        assert_eq!(linked_group("chatgroup://Yy Shopping"), Some("Yy Shopping"));
        assert_eq!(linked_group("user://Zed"), None);
        assert_eq!((group_command("Yy OOC"), group_command("Yytrade")), ("/group \"Yy OOC\" ".to_string(), "/group Yytrade ".to_string()));
        chatgroups::note_channel("Yy Linked", 3, 0x8004);
        chatgroups::note_channel("Yy Linkedro", 3, 0x8006);
        let mut w = ChatWindow::default();
        w.panes[0].selected.insert("Yy Linked".into());
        w.panes[0].selected.insert("Yy Linkedro".into());
        w.group_pressed(None, "Yy Linked", false);
        assert_eq!((w.panes[0].output.as_str(), w.input.as_str()), ("Yy Linked", ""));
        // (a read-only group is not taken; with Shift, or in a window that does not show it, the command is typed)
        w.group_pressed(None, "Yy Linkedro", false);
        assert_eq!(w.panes[0].output, "Yy Linked");
        w.group_pressed(None, "Yy Linked", true);
        assert_eq!(w.input, "/group \"Yy Linked\" ");
        w.tab = 1;
        w.input.clear();
        w.group_pressed(None, "Yy Linked", false);
        assert_eq!((w.panes[1].output.as_str(), w.input.as_str()), ("", "/group \"Yy Linked\" "));
        // (a group not there is passed over)
        w.input.clear();
        w.group_pressed(None, "Yy Nowhere", true);
        assert_eq!(w.input, "");
    }

    #[test]
    fn an_invitation_waits_is_declined_or_is_dropped() {
        // (asked about by default; declined and said so with the option at 0; from an ignored character declined
        // without a word)
        let mut w = ChatWindow::default();
        chatgroups::invite(71, "Inviterone");
        w.take_invitations();
        assert_eq!(w.invites.iter().map(|i| (i.owner, i.name.as_str(), i.marked, i.asked.is_some())).collect::<Vec<_>>(), [(71, "Inviterone", true, true)]);
        assert!(w.outs.is_empty() && w.invited("inviterone"));
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

    /// `ChatPGInviteAction` 2 only marks the inviter's row (FUN_100a7986); the press on it brings the question
    /// (FUN_100a76c9), whose yes puts the group into the chat windows chosen and out of the others (FUN_100a74cb).
    #[test]
    fn an_invitation_is_marked_then_asked_with_its_chat_windows() {
        let mut w = ChatWindow::default();
        w.invite_action = 2;
        w.invites.push(Invite { owner: 74, name: "Inviterfour".into(), marked: true, asked: None });
        assert!(w.invited("INVITERFOUR") && !w.invited("Zed"));
        w.ask_invitation("inviterfour");
        assert!(!w.invited("Inviterfour"));
        // (the group not there yet: no window starts chosen)
        assert_eq!(w.invites[0].asked, Some(Default::default()));
        // (the list by name, no regard to case)
        w.panes[1].name = "alpha".into();
        assert_eq!(invitation_order(&w.panes), [1, 0]);
        // (yes with the second window chosen: it shows the group, the first, which takes new groups by itself, not)
        put_group(&mut w.panes, "Inviterfour", &[1].into_iter().collect());
        assert!(w.panes[1].selected.contains("Inviterfour") && !w.panes[0].selected.contains("Inviterfour"));
        assert!(!w.panes[0].shows("Inviterfour"));
        // (a group there already: the windows that show it start chosen)
        chatgroups::note("Yy Invitergroup", 0x0e);
        w.panes[0].selected.insert("Yy Invitergroup".into());
        assert_eq!(shown_in(&w.panes, "Yy Invitergroup"), [0].into_iter().collect());
    }

    /// The window's menu holds its sub-menus as the original's does: "Visual" (with the mode and, for a borderless
    /// window, the two sliders of how solid), a separator, the channels to talk to and to show, and so on.
    #[test]
    fn the_windows_menu_holds_its_sub_menus() {
        let mut w = ChatWindow::default();
        let tree = w.menu_tree();
        let keys: Vec<&str> = tree.iter().map(|n| n.key.as_str()).collect();
        assert_eq!(keys[..11], ["visual", "", "talk", "subscribe", "auto", "", "config", "", "rename", "delete", "new"]);
        assert!(tree[1].entry.is_static() && tree[0].entry.sub.as_ref().is_some_and(|s| s.len() == tree[0].sub.len()));
        // (the menus by their places: the talk list, the combat groups under "subscribe", the mode under "visual")
        assert_eq!(path_to(&tree, Menu::Talk), Some(vec![2]));
        assert_eq!(path_to(&tree, Menu::Groups { combat: true }), Some(vec![3, 1]));
        assert_eq!(path_to(&tree, Menu::Mode), Some(vec![0, 0]));
        // (a window with its frame: no sliders)
        assert_eq!(path_to(&tree, Menu::Solid), None);
        assert_eq!(tree[0].sub.iter().map(|n| n.key.as_str()).collect::<Vec<_>>(), ["mode", "", "timestamps", "noinput", "hideinput", "fade", "", "behind", "top"]);
        // (borderless: the two sliders from 0 to 1, the window not active first, at the window's values)
        w.panes[0].look.borderless = true;
        w.panes[0].look.solid = (80, 30);
        let tree = w.menu_tree();
        assert_eq!(path_to(&tree, Menu::Solid), Some(vec![0, 1]));
        let sliders: Vec<(&str, crate::gui::popup::Kind)> = tree[0].sub[1].sub.iter().map(|n| (n.key.as_str(), n.entry.kind.clone())).collect();
        assert_eq!(sliders.iter().map(|s| s.0).collect::<Vec<_>>(), ["inactive", "active"]);
        let value = |k: &crate::gui::popup::Kind| match k {
            crate::gui::popup::Kind::Slider { min, max, value } => (*min, *max, (*value * 100.0).round()),
            _ => panic!(),
        };
        assert_eq!((value(&sliders[0].1), value(&sliders[1].1)), ((0.0, 1.0, 30.0), (0.0, 1.0, 80.0)));
        assert_eq!(node_at(&tree, &[0, 1, 1]).map(|n| (n.menu, n.key.as_str())), Some((Menu::Solid, "active")));
        assert!(node_at(&tree, &[0, 1, 2]).is_none() && node_at(&tree, &[]).is_none());
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
