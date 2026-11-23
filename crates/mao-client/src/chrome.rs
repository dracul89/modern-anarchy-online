//! Windows in the original's chrome. [`window`] is used in place of `egui::Window` by every window of the interface: when the
//! skin's pictures are there (`skin`), the window has no egui title bar and is drawn with the original's frame pieces
//! (`GFX_GUI_WINDOW3_BORDER_*`, greyscale masks tinted with the theme's border colour, as the original tints them with its
//! `GUIColors.xml`), the title inside the frame at the top left and the original's pin and close buttons at the top right;
//! the window is moved by the strip under its top border, as the original's (`WndBorder::HitTest`). Without the pictures
//! it is the plain egui window it was.
//!
//! A window fades when the pointer leaves it and comes back when it returns, unless its pin is down (the original's
//! autofade, [`fade_level`]). Each window starts where the player left it (`layout.rs`), and closes the next time it is
//! shown after Escape asked ([`close_soon`]).

use std::sync::Mutex;

use crate::theme::Palette;

/// The windows whose pin is down, by their name (none until the file is read: `pins.ini` beside the settings, one name a
/// line; the original keeps it per window as `WindowPinButtonState`, Window::SaveWndConfig). The pin is the window's fade
/// switch, nothing else: GUI.dll's `WndBorder::GetPinButtonState` (0x10159181) is asked only by `Window::_CanFade` and
/// the saving. Until 2026-11-18 a pinned window was the one Escape left open, which the original has no rule for.
static PINNED: Mutex<Option<Vec<String>>> = Mutex::new(None);

fn pins_path() -> std::path::PathBuf {
    crate::charprefs::file("pins.ini")
}

pub fn pinned(name: &str) -> bool {
    let mut list = PINNED.lock().unwrap();
    list.get_or_insert_with(|| std::fs::read_to_string(pins_path()).map(|t| t.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect()).unwrap_or_default()).iter().any(|n| n == name)
}

/// The pin set or taken off (the button, or the frame menu's "Autofade Window"): the window goes solid over 0.2 s
/// (`WndBorder::SetPinButtonState`, 0x1015b23a: `FadeTo(1.0, 0, 200000)`), and the list is kept.
fn set_pinned(name: &str, on: bool) {
    pinned(name);
    let mut guard = PINNED.lock().unwrap();
    let list = guard.get_or_insert_with(Vec::new);
    list.retain(|n| n != name);
    if on {
        list.push(name.to_string());
    }
    if std::env::var_os("MAO_UI_PREVIEW").is_none() {
        let _ = std::fs::write(pins_path(), list.join("\n") + "\n");
    }
    drop(guard);
    fade_to(name, 1.0, 0.0, 0.2);
}

/// A window's fading (`Window::FadeTo`, 0x10155b1e, run by `Window::UpdateFadeLevel`, 0x1015591b): from a level to another,
/// linearly over a time, starting after a wait; and whether the pointer was over it last frame.
#[derive(Clone, Copy)]
struct Fade {
    from: f32,
    to: f32,
    start: std::time::Instant,
    duration: f32,
    hovered: bool,
}

static FADES: Mutex<Vec<(String, Fade)>> = Mutex::new(Vec::new());

/// The windows fading by a rule of their own now ([`bare_window`]: a borderless chat window), not by the pointer.
static OWN: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn level_of(fade: &Fade, now: std::time::Instant) -> f32 {
    if now <= fade.start {
        return fade.from;
    }
    let done = if fade.duration <= 0.0 { 1.0 } else { (now - fade.start).as_secs_f32() / fade.duration };
    fade.from + (fade.to - fade.from) * done.min(1.0)
}

/// A window's level now (1: solid).
pub fn fade_level(name: &str) -> f32 {
    let now = std::time::Instant::now();
    FADES.lock().unwrap().iter().find(|f| f.0 == name).map_or(1.0, |f| level_of(&f.1, now))
}

fn fade_to(name: &str, to: f32, delay: f32, duration: f32) {
    let now = std::time::Instant::now();
    let mut fades = FADES.lock().unwrap();
    let hovered = fades.iter().find(|f| f.0 == name).is_some_and(|f| f.1.hovered);
    let from = fades.iter().find(|f| f.0 == name).map_or(1.0, |f| level_of(&f.1, now));
    fades.retain(|f| f.0 != name);
    fades.push((name.to_string(), Fade { from, to, start: now + std::time::Duration::from_secs_f32(delay), duration, hovered }));
}

/// Whether a window may fade (`Window::_CanFade`, 0x10154b4d: its pin is not down; `WindowController_c::FadeWindows` also
/// leaves a modal window alone: the message boxes here). A framed chat window fades as any other: its own fading
/// (`FUN_100972da`) is only that of a borderless one ([`bare_window`]).
fn can_fade(name: &str) -> bool {
    !pinned(name) && !DIALOGS.contains(&name)
}

/// A window's fading toward `to` over `duration` seconds from now, unless it is already going there: the chat window's
/// own (FUN_100972da: `Window::FadeTo(level, 0, duration)`).
fn fade_toward(name: &str, to: f32, duration: f32) {
    let going = FADES.lock().unwrap().iter().find(|f| f.0 == name).map(|f| f.1.to);
    if going != Some(to) {
        fade_to(name, to, 0.0, duration);
    }
}

/// While a fade of the window is under way or waiting, the next frames are asked for.
fn repaint_while_fading(ctx: &egui::Context, name: &str) {
    let now = std::time::Instant::now();
    if let Some((_, f)) = FADES.lock().unwrap().iter().find(|f| f.0 == name) {
        let end = f.start + std::time::Duration::from_secs_f32(f.duration);
        if now < end {
            ctx.request_repaint_after(if now < f.start { f.start - now } else { std::time::Duration::from_millis(30) });
        }
    }
}

/// The level a borderless chat window fades to, and over how long (FUN_100972da): its `window_transparency_active`
/// over 0.2 s while its chat is being typed in (the shown window's config +0xda, set as the input starts,
/// FUN_10099084, and cleared as it ends, FUN_1009b273), else its `window_transparency_inactive` over a second; at once
/// from where it is.
pub fn chat_fade(typing: bool, active: f32, inactive: f32) -> (f32, f32) {
    if typing { (active, 0.2) } else { (inactive, 1.0) }
}

/// The pointer's going in or out of a window, seen after it was drawn (`WindowController_c::HandleMouseMoved` 0x1015867f →
/// `FadeWindows` 0x1015831b, while no mouse button is held): the one it left fades to 0.3 over a second after two, the one it
/// came into goes solid over 0.2 s.
fn note_hover(ctx: &egui::Context, name: &str, layer: egui::LayerId) {
    if ctx.input(|i| i.pointer.any_down()) {
        return;
    }
    let hovered = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| ctx.layer_id_at(p) == Some(layer));
    let before = FADES.lock().unwrap().iter().find(|f| f.0 == name).is_some_and(|f| f.1.hovered);
    if hovered != before {
        if hovered {
            fade_to(name, 1.0, 0.0, 0.2);
        } else if can_fade(name) {
            fade_to(name, 0.3, 2.0, 1.0);
        }
        let mut fades = FADES.lock().unwrap();
        match fades.iter_mut().find(|f| f.0 == name) {
            Some(f) => f.1.hovered = hovered,
            None => fades.push((name.to_string(), Fade { from: 1.0, to: 1.0, start: std::time::Instant::now(), duration: 0.0, hovered })),
        }
    }
    // (a fade under way, or one waiting to start, is drawn as it goes)
    repaint_while_fading(ctx, name);
}

/// The windows that have a "?" button, and the help page each opens (`WndBorder::SetHelpFile`, 0x1015b139, called for these
/// nine windows; the button sends `file://<page>` to the info view, `WndBorder::SlotHelpButton`, which finds it under
/// `cd_image/text/help/`).
pub const HELP_PAGES: &[(&str, &str)] = &[
    ("Perks", "The Perks Window.html"),
    ("PetWindow", "The Pet Window.html"),
    ("LFT", "The LFT Window.html"),
    ("LaserTarget", "The laser target window.html"),
    ("MechSelect", "The Mech Select window.html"),
    ("SignupTerminal", "The signup terminal window.html"),
    ("Research", "The Research window.html"),
    ("Skills", "The Skill Window.html"),
    ("TeleportTarget", "The teleport target window.html"),
];

/// The help pages asked for by a "?" button, for the info view.
static HELP_ASKED: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn take_help() -> Vec<String> {
    std::mem::take(&mut *HELP_ASKED.lock().unwrap())
}

/// The windows Escape asked to close (`Ui::close_with_escape`): each closes the next time it is shown, as its close button would
/// close it (so a shop or an NPC chat tells the server); what is left at the end of the frame was not open and is dropped.
static CLOSING: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn close_soon(name: &str) {
    CLOSING.lock().unwrap().push(name.to_string());
}

/// The frame's menu button pressed: the window's name, where its menu goes, and the frame it was pressed in (none:
/// to be shown by the frame itself at once).
///
/// The original's window frame has a button at the left end of its title (`WndBorder`, GUI.dll FUN_1015ae85: a
/// `BorderButton_c` of `GFX_GUI_WINDOW_ICON_I` in the border's first place), which shows the window's menu under
/// itself, five below its bottom (FUN_1015aa2f): the entries the window's view puts in, then "Autofade Window"
/// and "Close Window" (FUN_1015b34f). Here a window that has entries of its own takes the press ([`frame_menu`])
/// and shows them with [`close_entry`] after them; for any other the frame shows its own two, "Autofade Window" (checked
/// while the window fades, `1 - pin`: `SlotFadeMenuEntry` 0x1015b2ed turns the pin) and "Close Window".
static FRAME_ASKED: Mutex<Option<(String, egui::Pos2, Option<u64>)>> = Mutex::new(None);

thread_local! {
    /// The frame's own menu while it is up, and whose it is.
    static FRAME_POPUP: std::cell::RefCell<Option<(String, crate::gui::popup::PopupMenu)>> = const { std::cell::RefCell::new(None) };
}

/// Where the menu of the window `name` is to be shown, when its frame's button was pressed: a window with a menu of
/// its own asks after it has been drawn.
pub fn frame_menu(name: &str) -> Option<egui::Pos2> {
    let mut asked = FRAME_ASKED.lock().unwrap();
    let at = asked.as_ref().filter(|(n, _, _)| n == name).map(|(_, at, _)| *at)?;
    *asked = None;
    Some(at)
}

/// Asks the window `name` for its menu at `at` (a rollup page's menu button, `rollup.rs`): a window with a menu of
/// its own takes it ([`frame_menu`]) while it is drawn.
pub fn ask_frame_menu(name: &str, at: egui::Pos2, frame: u64) {
    *FRAME_ASKED.lock().unwrap() = Some((name.to_string(), at, Some(frame)));
}

/// A press of [`ask_frame_menu`] from before frame `frame` that no window took: where its menu goes (the ask is gone).
pub fn frame_menu_left(name: &str, frame: u64) -> Option<egui::Pos2> {
    let mut asked = FRAME_ASKED.lock().unwrap();
    let at = asked.as_ref().filter(|(n, _, f)| n == name && f.is_none_or(|f| f < frame)).map(|(_, at, _)| *at)?;
    *asked = None;
    Some(at)
}

/// The frame menu's last entry, "Close Window".
pub fn close_entry() -> String {
    crate::gui::xml::ldb_named(10000, "CloseWindow", "Close")
}

/// The windows a menu's "Close Window" closes: asked after the window was drawn, so kept for the next frame.
static CLOSING_NEXT: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Closes a window the next time it is shown, asked from its own menu (after it was drawn this frame).
pub fn close_next(name: &str) {
    CLOSING_NEXT.lock().unwrap().push(name.to_string());
}

/// The end of a frame: Escape's requests not taken (the windows not shown) are dropped, and the menus' requests of
/// this frame wait for the next.
pub fn end_frame() {
    let next = std::mem::take(&mut *CLOSING_NEXT.lock().unwrap());
    *CLOSING.lock().unwrap() = next;
}

pub(crate) fn take_closing(name: &str) -> bool {
    let mut list = CLOSING.lock().unwrap();
    let before = list.len();
    list.retain(|n| n != name);
    list.len() != before
}

/// The message boxes' window names ([`dialog`]): none keeps a place, and Escape closes each with `esc_dialogs`.
pub const DIALOGS: &[&str] = &["TradeDoubleConfirmDlg", "perk-dialog", "team-invite", "remove-friend", "ignore-friend", "delete-mail", "disable-tips", "delete-mission", "delete-item", "delete-container-item", "reset-skill", "rename-chat-window", "private-invite", "start-minimize", "rename-backpack", "DuelChallenge", "error-box", "afk-message", "split-stack", "split-stack-bag", "OrgLeave", "OrgDisband", "OrgJoin", "OrgLeavePayTax", "OrgTaxChanged", "OrgPromotion", "ConfirmSendMail", "MailTakeItems", "BattleStationSpot", "BattleFinished", "drop-item", "delete-dropped-item", "slots-exhausted", "bind-key"];

/// The error boxes waiting to be shown ([`alert`]).
static ALERTS: Mutex<std::collections::VecDeque<String>> = Mutex::new(std::collections::VecDeque::new());

/// An error box: the original's message signal (`GlobalSignals_c` +0x184, which Gamecode and GUI.dll raise with a text
/// for a refusal that needs saying: a team's mission terminal used alone, a mail that cannot be deleted, a web view
/// that died) and the boxes the windows make themselves with the title `Error` and an OK button (the mission
/// terminal's `NotEnoughCredits_NeedX`, FUN_100d09cb). The boxes are shown one after the other ([`show_alerts`]); a
/// text already waiting is not queued twice. Any thread may call it.
pub fn alert(text: impl Into<String>) {
    let text = text.into();
    let mut queue = ALERTS.lock().unwrap_or_else(|e| e.into_inner());
    if !text.trim().is_empty() && !queue.contains(&text) {
        eprintln!("error box: {text}");
        queue.push_back(text);
    }
}

/// The error box at the head of the queue, until its OK is pressed or it is closed.
pub fn show_alerts(ctx: &egui::Context, palette: &Palette) {
    let Some(text) = ALERTS.lock().unwrap_or_else(|e| e.into_inner()).front().cloned() else { return };
    let named = |name: &str, or: &str| crate::feedback::texts().and_then(|t| t.named(10000, name)).unwrap_or(or).to_string();
    if dialog(ctx, palette, "error-box", &named("Error", "Error"), &text, &[&named("MsgBox_OK", "OK")]).is_some() {
        ALERTS.lock().unwrap_or_else(|e| e.into_inner()).pop_front();
    }
}

/// The error boxes waiting (for tests and the scripts' log).
pub fn alerts_waiting() -> Vec<String> {
    ALERTS.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect()
}

/// When each message box was last shown (the frame), to tell a new showing from the next frame of one.
static DIALOG_SEEN: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());

/// A box that asks for a text takes the keys when it comes up: GUI.dll's `TextInputDialogForm_c::AllAttached`
/// (0x101483d8) gives its text view the focus. Called every frame with the box's field, it asks for the focus on the
/// first frame of a showing; `name` is one of [`DIALOGS`].
pub fn focus_when_new(ctx: &egui::Context, name: &'static str, field: &egui::Response) {
    static SEEN: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());
    let frame = ctx.cumulative_frame_nr();
    let mut seen = SEEN.lock().unwrap();
    let was = seen.iter().find(|s| s.0 == name).map(|s| s.1);
    seen.retain(|s| s.0 != name);
    seen.push((name, frame));
    if was.is_none_or(|f| f + 1 < frame) {
        field.request_focus();
    }
}

/// The width a message box's text is wrapped to: `TextRenderer_c::SetAspectRatio(2.0)`, about twice as wide as tall.
pub fn dialog_width(height_at: impl Fn(f32) -> f32) -> f32 {
    let mut width = 120.0;
    while width < 600.0 && width < 2.0 * height_at(width) {
        width += 20.0;
    }
    width
}

/// A message box, GUI.dll's `DialogBox_c` (`_Init` 0x1012a7a8, `DialogBoxView_c` 0x1012aed3, `DialogButtons_c::_Initialize`
/// 0x1012b20f): a window that comes up at the pointer (`MoveToMouse`) holding its text (a `TextView_c` in font 5 with
/// markup, 15 from the sides, 5 from the top and 20 over the buttons, wrapped about twice as wide as tall) over its
/// buttons in a row 5 from the bottom, each 8 from the next: at the right, or in the middle when there is only one.
/// `Some(Some(k))` when button `k` was pressed, `Some(None)` when it was closed without one (its close button, or Escape
/// with `esc_dialogs`: `SlotSelected(-1)`), `None` while it waits. `name` is one of [`DIALOGS`].
pub fn dialog(ctx: &egui::Context, palette: &Palette, name: &'static str, title: &str, text: &str, buttons: &[&str]) -> Option<Option<usize>> {
    dialog_placed(ctx, palette, name, title, text, buttons, false)
}

/// [`dialog`], put in the screen's middle when it comes up when `centred` (the caller's `Window::MoveToCenter` after
/// `Go`), else at the pointer.
pub fn dialog_placed(ctx: &egui::Context, palette: &Palette, name: &'static str, title: &str, text: &str, buttons: &[&str], centred: bool) -> Option<Option<usize>> {
    debug_assert!(DIALOGS.contains(&name), "{name} is not listed in chrome::DIALOGS");
    let frame = ctx.cumulative_frame_nr();
    let fresh = {
        let mut seen = DIALOG_SEEN.lock().unwrap();
        let was = seen.iter().find(|s| s.0 == name).map(|s| s.1);
        seen.retain(|s| s.0 != name);
        seen.push((name, frame));
        was.is_none_or(|f| f + 1 < frame)
    };
    // (without a pointer yet, the screen's middle, and there every frame: the first frames' screen is egui's placeholder,
    // far larger than the window's, and a box put at its middle once stayed outside the picture)
    let pointer = ctx.input(|i| i.pointer.latest_pos()).filter(|_| !centred);
    let at = pointer.unwrap_or(ctx.content_rect().center());
    let font = crate::gui::font::FontId::NORMAL;
    let width = dialog_width(|w| crate::gui::controls::markup_height(ctx, text, font, w));
    let mut open = true;
    let mut answer = None;
    window(
        ctx,
        palette,
        name,
        title,
        &mut open,
        |w| {
            let w = w.resizable(false).pivot(egui::Align2::CENTER_CENTER);
            if fresh || pointer.is_none() { w.current_pos(at) } else { w }
        },
        |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                ui.allocate_ui_with_layout(egui::vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    crate::gui::controls::markup(ui, palette, text, font, rgb(palette.text));
                });
                ui.add_space(15.0);
            });
            ui.add_space(20.0);
            let row = |ui: &mut egui::Ui, answer: &mut Option<usize>| {
                for (k, label) in buttons.iter().enumerate().rev() {
                    ui.add_space(8.0);
                    if button(ui, palette, label, true).clicked() {
                        *answer = Some(k);
                    }
                    ui.add_space(8.0);
                }
            };
            if buttons.len() == 1 {
                ui.vertical_centered(|ui| {
                    if button(ui, palette, buttons[0], true).clicked() {
                        answer = Some(0);
                    }
                });
            } else {
                ui.allocate_ui_with_layout(egui::vec2(width + 30.0, 0.0), egui::Layout::right_to_left(egui::Align::Center), |ui| row(ui, &mut answer));
            }
            ui.add_space(5.0);
        },
    );
    match answer {
        Some(k) => Some(Some(k)),
        None if !open => Some(None),
        None => None,
    }
}

/// A message box that asks for a text: `DialogBox_c` holding a `TextInputDialogForm_c` (0x1014851a) and no message of its
/// own. The form is a column of its prompt (a `TextView_c` in font 5 with markup, 10 over the field) and the text field
/// (`TextInputView_c`, 150 wide, `field` written in it); the buttons are the box's, as [`dialog`] lays them. Enter in the
/// field answers as the first button does (`SlotEnterPressed` 0x101484a4 emits 0). The prompt is wrapped as a message
/// box's text is (a guess: the form's `TextView_c` sets no aspect ratio of its own). Answers as [`dialog`].
pub fn text_dialog(ctx: &egui::Context, palette: &Palette, name: &'static str, title: &str, prompt: &str, field: &mut String, buttons: &[&str]) -> Option<Option<usize>> {
    debug_assert!(DIALOGS.contains(&name), "{name} is not listed in chrome::DIALOGS");
    let frame = ctx.cumulative_frame_nr();
    let fresh = {
        let mut seen = DIALOG_SEEN.lock().unwrap();
        let was = seen.iter().find(|s| s.0 == name).map(|s| s.1);
        seen.retain(|s| s.0 != name);
        seen.push((name, frame));
        was.is_none_or(|f| f + 1 < frame)
    };
    let pointer = ctx.input(|i| i.pointer.latest_pos());
    let at = pointer.unwrap_or(ctx.content_rect().center());
    let font = crate::gui::font::FontId::NORMAL;
    let width = dialog_width(|w| crate::gui::controls::markup_height(ctx, prompt, font, w)).max(150.0);
    let mut open = true;
    let mut answer = None;
    window(
        ctx,
        palette,
        name,
        title,
        &mut open,
        |w| {
            let w = w.resizable(false).pivot(egui::Align2::CENTER_CENTER);
            if fresh || pointer.is_none() { w.current_pos(at) } else { w }
        },
        |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                ui.allocate_ui_with_layout(egui::vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    crate::gui::controls::markup(ui, palette, prompt, font, rgb(palette.text));
                    ui.add_space(10.0);
                    let typed = text_edit(ui, palette, egui::TextEdit::singleline(field).desired_width(150.0));
                    focus_when_new(ctx, name, &typed);
                    if typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        answer = Some(0);
                    }
                });
                ui.add_space(15.0);
            });
            ui.add_space(20.0);
            if buttons.len() == 1 {
                ui.vertical_centered(|ui| {
                    if button(ui, palette, buttons[0], true).clicked() {
                        answer = Some(0);
                    }
                });
            } else {
                ui.allocate_ui_with_layout(egui::vec2(width + 30.0, 0.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for (k, label) in buttons.iter().enumerate().rev() {
                        ui.add_space(8.0);
                        if button(ui, palette, label, true).clicked() {
                            answer = Some(k);
                        }
                        ui.add_space(8.0);
                    }
                });
            }
            ui.add_space(5.0);
        },
    );
    match answer {
        Some(k) => Some(Some(k)),
        None if !open => Some(None),
        None => None,
    }
}

/// The texts of a yes/no message box's buttons (text.mdb 10000's `MsgBox_Yes` and `MsgBox_No`).
pub fn yes_no() -> [String; 2] {
    [crate::gui::xml::ldb_named(10000, "MsgBox_Yes", "Yes"), crate::gui::xml::ldb_named(10000, "MsgBox_No", "No")]
}

/// A yes/no message box ([`dialog`]): `Some(true)` for Yes, `Some(false)` for No or when it was closed.
pub fn question(ctx: &egui::Context, palette: &Palette, name: &'static str, title: &str, text: &str) -> Option<bool> {
    question_placed(ctx, palette, name, title, text, false)
}

/// [`question`], in the screen's middle when `centred` ([`dialog_placed`]).
pub fn question_placed(ctx: &egui::Context, palette: &Palette, name: &'static str, title: &str, text: &str, centred: bool) -> Option<bool> {
    let [yes, no] = yes_no();
    dialog_placed(ctx, palette, name, title, text, &[&yes, &no], centred).map(|a| a == Some(0))
}

/// A right-click menu as the original's `PopupMenu_c` (`gui::popup`), for the client's windows: `entries` are the labels
/// and whether each can be chosen.
pub fn menu_at(ctx: &egui::Context, at: egui::Pos2, entries: &[(String, bool)]) -> crate::gui::popup::PopupMenu {
    let entries = entries.iter().map(|(label, enabled)| crate::gui::popup::Entry::text(label.clone(), *enabled)).collect();
    crate::gui::popup::PopupMenu::new(entries, at, None, ctx.cumulative_frame_nr())
}

/// A menu whose entries may have check marks ([`menu_at`]): label, whether it can be chosen, and the mark.
pub fn menu_marked(ctx: &egui::Context, at: egui::Pos2, entries: &[(String, bool, Option<bool>)]) -> crate::gui::popup::PopupMenu {
    let entries = entries.iter().map(|(label, enabled, mark)| crate::gui::popup::Entry { mark: *mark, ..crate::gui::popup::Entry::text(label.clone(), *enabled) }).collect();
    crate::gui::popup::PopupMenu::new(entries, at, None, ctx.cumulative_frame_nr())
}

/// A menu of entries of any kind: separators, sliders, sub-menus (`gui::popup::Entry`).
pub fn menu_of(ctx: &egui::Context, at: egui::Pos2, entries: Vec<crate::gui::popup::Entry>) -> crate::gui::popup::PopupMenu {
    crate::gui::popup::PopupMenu::new(entries, at, None, ctx.cumulative_frame_nr())
}

/// What a frame of a menu with sub-menus or sliders did ([`menu_path`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Picked {
    /// The entry chosen, by its place through the sub-menus; the menu is gone.
    Entry(Vec<usize>),
    /// A slider entry's value now, and whether its knob was let go; the menu stays.
    Slider(Vec<usize>, f64, bool),
}

/// A frame of an open menu that may have sub-menus and sliders ([`menu_of`]); the menu is gone once it closes.
pub fn menu_path(ctx: &egui::Context, palette: &Palette, name: &str, menu: &mut Option<crate::gui::popup::PopupMenu>) -> Option<Picked> {
    let open = menu.as_mut()?;
    let skin = crate::skin::skin(ctx);
    let measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
    let env = crate::gui::host::Env { skin: skin.as_deref(), palette, measure: &measure };
    match open.frame(ctx, &env, egui::Id::unique(name)) {
        crate::gui::popup::Event::None => None,
        crate::gui::popup::Event::Chosen(path) => {
            *menu = None;
            Some(Picked::Entry(path))
        }
        crate::gui::popup::Event::Closed => {
            *menu = None;
            None
        }
        crate::gui::popup::Event::Slider(path, value, done) => Some(Picked::Slider(path, value, done)),
    }
}

/// A frame of an open menu ([`menu_at`]): the entry chosen, if one was; the menu is gone once it closes.
pub fn menu(ctx: &egui::Context, palette: &Palette, name: &str, menu: &mut Option<crate::gui::popup::PopupMenu>) -> Option<usize> {
    let open = menu.as_mut()?;
    let skin = crate::skin::skin(ctx);
    let measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
    let env = crate::gui::host::Env { skin: skin.as_deref(), palette, measure: &measure };
    match open.show(ctx, &env, egui::Id::unique(name)) {
        crate::gui::popup::Outcome::None => None,
        crate::gui::popup::Outcome::Chosen(k) => {
            *menu = None;
            Some(k)
        }
        crate::gui::popup::Outcome::Closed => {
            *menu = None;
            None
        }
    }
}

pub fn rgb(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

/// The frame's margins: the title row at the top, the border pieces around.
const LEFT: i8 = 7;
const RIGHT: i8 = 7;
const TOP: i8 = 24;
const BOTTOM: i8 = 10;

/// What the frame of a window drawn with the skin adds round its content, across and down.
pub fn frame_room() -> egui::Vec2 {
    egui::vec2((LEFT + RIGHT) as f32, (TOP + BOTTOM) as f32)
}

/// A window named `name` (its egui id and its `layout.ini` entry) titled `title`, shown while `*open`; `build` sets the egui window's
/// size and place (`|w| w.resizable(false).default_pos(..)`), `add` fills it. Returns what `egui::Window::show` returns.
pub fn window<R>(ctx: &egui::Context, palette: &Palette, name: &str, title: &str, open: &mut bool, build: impl FnOnce(egui::Window<'static>) -> egui::Window<'static>, add: impl FnOnce(&mut egui::Ui) -> R) -> Option<egui::InnerResponse<Option<R>>> {
    if *open && take_closing(name) {
        *open = false;
    }
    // (a window docked in the rollup area is drawn as a page of it: `rollup.rs`)
    if *open && crate::rollup::docked(name) {
        return crate::rollup::page(ctx, palette, name, title, open, add);
    }
    if !*open {
        return None;
    }
    // (a movable window waits for the screen's real size — egui's first frames report a placeholder — so its starting
    // place and size are not taken from a 10000-point screen)
    if crate::layout::kept(name) && crate::layout::screen().is_none() && std::env::var_os("MAO_UI_PREVIEW").is_none() {
        return None;
    }
    // (one of the original's dockable views is a tab of its dock area's window, `docks.rs`: the window is the area's,
    // drawn by the view shown in it with its tabs, and titled with nothing but them — FUN_1003c78b makes it with an
    // empty title; a view behind another's tab is open but not drawn)
    let docked = if crate::docks::dockable(name) {
        match crate::docks::enter(name, title) {
            Some(shown) => Some(shown),
            None => return None,
        }
    } else {
        None
    };
    let win: &str = docked.as_ref().map_or(name, |d| d.area.as_str());
    let title = if docked.is_some() { "" } else { title };
    // (the place the player left it last time, `layout.ini`: over the caller's starting place; an area's window not
    // placed yet starts where its view was kept, else at the original's frame for the area, else the view's)
    crate::layout::track(win);
    let saved = if !crate::layout::kept(win) {
        None
    } else if win != name {
        crate::layout::spot(win).or_else(|| crate::layout::spot(name)).or_else(|| crate::layout::saved(win)).or_else(|| crate::layout::saved(name))
    } else {
        crate::layout::saved(name)
    };
    // (the original client's layout gives a size too, until the player has placed the window here)
    let sized = crate::layout::kept(win).then(|| crate::layout::original_size(win).or_else(|| (win != name).then(|| crate::layout::original_size(name)).flatten())).flatten();
    let relocate = crate::layout::relocate(win);
    let sizable = std::cell::Cell::new(false);
    let sizable = &sizable;
    let build = move |w: egui::Window<'static>| {
        let w = build(w);
        // (a window the player can size starts at the size it was left at, `sizes.ini`: over the original's frame and
        // the caller's own size)
        let w = w.resize(|r| {
            sizable.set(r.is_resizable().any());
            r
        });
        let sized = sizable.get().then(|| crate::layout::saved_size(win).or_else(|| (win != name).then(|| crate::layout::saved_size(name)).flatten())).flatten().or(sized);
        let w = match sized {
            Some(size) => w.default_size(size),
            None => w,
        };
        match saved {
            Some(at) if relocate => w.current_pos(at),
            Some(at) => w.default_pos(at),
            None => w,
        }
    };
    // (a dock area's window: its tabs over the shown view, `DockWindow_c`'s `TabView`; the tab picked is shown from
    // the next frame, and a tab taken up by the right button is carried, `docks::end_frame`)
    let tabbed = docked.as_ref().map(|d| (d.area.clone(), d.tabs.iter().map(|t| t.0.clone()).collect::<Vec<String>>(), d.tabs.iter().map(|t| t.1.clone()).collect::<Vec<String>>(), d.selected));
    let add = move |ui: &mut egui::Ui| -> R {
        let Some((area, views, titles, selected)) = tabbed else { return add(ui) };
        let names: Vec<&str> = titles.iter().map(String::as_str).collect();
        let mut picked = selected;
        let mut hits = TabHits::default();
        let result = tab_view_hits(ui, palette, &names, &mut picked, &mut hits, |ui, _| add(ui));
        crate::docks::after(&area, &views, picked, &hits, ui.input(|i| i.pointer.interact_pos()));
        result
    };
    let accent = rgb(palette.accent);
    let id = egui::Id::unique(win);
    let skin = crate::skin::skin(ctx).filter(|s| s.has("GFX_GUI_WINDOW3_BORDER_TL"));
    let Some(skin) = skin else {
        let mut is_open = *open;
        let shown = build(egui::Window::new(egui::RichText::new(title).color(accent)).collapsible(false)).id(id).open(&mut is_open).show(ctx, |ui| {
            let room = ui.max_rect().size();
            let result = add(ui);
            // (the size kept: the room and what egui's own frame and title add round the content)
            if sizable.get()
                && let Some(outer) = ctx.memory(|m| m.area_rect(id)).filter(|r| r.is_finite())
            {
                crate::layout::note_size(win, room + (outer.size() - ui.min_rect().size()).max(egui::Vec2::ZERO));
            }
            result
        });
        *open = is_open;
        if !is_open && let Some(d) = &docked {
            crate::docks::close_others(&d.area, name);
        }
        return shown;
    };
    if !*open {
        return None;
    }
    let panel = egui::Color32::from_rgba_unmultiplied(palette.panel[0], palette.panel[1], palette.panel[2], 225);
    let hover = rgb(palette.hover);
    let frame = egui::Frame::NONE.inner_margin(egui::Margin { left: LEFT, right: RIGHT, top: TOP, bottom: BOTTOM });
    let mut close = false;
    let title = title.to_string();
    let view = name;
    let name = name.to_string();
    if OWN.lock().unwrap().iter().any(|n| *n == name) {
        OWN.lock().unwrap().retain(|n| *n != name);
        fade_to(&name, 1.0, 0.0, 0.0);
    }
    // (the pin and the fading are the window's: an area's for a docked view)
    let win = win.to_string();
    let level = fade_level(&win);
    // (the window moves only by the strip under its top border, `WndBorder::HitTest` 0x101596b7's item 1, laid out by
    // `WndBorder::Layout` 0x1015a4ba: the top border's width, from a point under it down to 18; egui's own drag of a
    // window without a title bar, from anywhere on it, is held where the window is: the drag's start is made its place)
    if let Some(r) = ctx.read_response(id.with("move"))
        && r.dragged()
        && let (Some(at), Some(total)) = (egui::AreaState::load(ctx, id).and_then(|s| s.pivot_pos), r.total_drag_delta())
    {
        ctx.data_mut(|d| d.insert_temp(id.with("pivot_at_drag_start"), at - total));
    }
    if ctx.read_response(id.with("move-strip")).is_some_and(|r| r.dragged())
        && let Some(mut state) = egui::AreaState::load(ctx, id)
        && let Some(at) = state.pivot_pos.as_mut()
    {
        *at += ctx.input(|i| i.pointer.delta());
        ctx.memory_mut(|m| m.areas_mut().set_state(egui::LayerId::new(egui::Order::Middle, id), state));
    }
    let shown = build(egui::Window::new(title.as_str()).title_bar(false).collapsible(false).frame(frame)).id(id).show(ctx, |ui| {
        // (the window's fade over its frame and all it holds: `UpdateFadeLevel` multiplies the view's alpha)
        ui.multiply_opacity(level);
        let painter = ui.painter().clone();
        // the frame goes under the content: its place in the paint list is kept, it is drawn once the size is known
        let under = painter.add(egui::Shape::Noop);
        let room = ui.max_rect().size();
        let result = add(ui);
        // (the size kept: the room the window gives its content, and the frame)
        if sizable.get() {
            crate::layout::note_size(&win, room + egui::vec2((LEFT + RIGHT) as f32, (TOP + BOTTOM) as f32));
        }
        // The whole window as laid out: egui's record of the area (last frame's for a window that just appeared, hence the fallback
        // from the content).
        let content = ui.min_rect();
        let outer = ctx.memory(|m| m.area_rect(id)).filter(|r| r.is_finite() && r.width() > 20.0).unwrap_or_else(|| egui::Rect::from_min_max(content.min - egui::vec2(LEFT as f32, TOP as f32), content.max + egui::vec2(RIGHT as f32, BOTTOM as f32)));
        painter.set(under, frame_shape(&skin, outer, panel, accent));
        // (the strip that moves the window, under the buttons and the frame menu made after it)
        if let (Some(tl), Some(tr), Some(top)) = (skin.get("GFX_GUI_WINDOW3_BORDER_TL"), skin.get("GFX_GUI_WINDOW3_BORDER_TR"), skin.get("GFX_GUI_WINDOW3_BORDER_TOP")) {
            let strip = egui::Rect::from_min_max(egui::pos2(outer.left() + tl.size.x, outer.top() + top.size.y + 1.0), egui::pos2(outer.right() - tr.size.x, outer.top() + 18.0));
            if strip.is_positive() {
                ui.interact(strip, id.with("move-strip"), egui::Sense::drag());
            }
        }
        // (the frame's menu button at the title's left end, which moves the title right: `AddBorderView` in the
        // border's first place)
        let mut title_left = outer.left() + 9.0;
        if let Some(icon) = skin.get("GFX_GUI_WINDOW_ICON_I") {
            let rect = egui::Rect::from_min_size(egui::pos2(outer.left() + 7.0, outer.top() + 5.0 + ((15.0 - icon.size.y) * 0.5).floor()), icon.size);
            let response = ui.interact(rect.expand(1.0), id.with("frame-menu"), egui::Sense::click());
            let up = FRAME_POPUP.with(|p| p.borrow().as_ref().is_some_and(|(n, _)| *n == name));
            let tint = if response.hovered() { hover } else if up { rgb(palette.text_selected) } else { accent };
            painter.image(icon.texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), tint);
            // (`MAO_FRAME_MENU=<window>`: the frame's own menu up, for a picture; as if pressed the frame before, so
            // that no window's own menu takes its place)
            static PICTURED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            let pictured = std::env::var("MAO_FRAME_MENU").is_ok_and(|n| n == name) && !PICTURED.swap(true, std::sync::atomic::Ordering::Relaxed);
            if response.clicked() || pictured {
                *FRAME_ASKED.lock().unwrap() = Some((name.clone(), egui::pos2(rect.left(), rect.bottom() + 5.0), (!pictured).then(|| ctx.cumulative_frame_nr())));
            }
            title_left = rect.right() + 4.0;
        }
        // (a press no window took for a menu of its own: the frame's, which has the one entry)
        let stale = FRAME_ASKED.lock().unwrap().as_ref().filter(|(n, _, frame)| *n == name && frame.is_none_or(|f| f < ctx.cumulative_frame_nr())).map(|(_, at, _)| *at);
        if let Some(at) = stale {
            *FRAME_ASKED.lock().unwrap() = None;
            let autofade = crate::gui::xml::ldb_named(10000, "AutofadeWindow", "Autofade Window");
            FRAME_POPUP.with(|p| *p.borrow_mut() = Some((name.clone(), menu_marked(ctx, at, &[(autofade, true, Some(!pinned(&win))), (close_entry(), true, None)]))));
        }
        let mut own = FRAME_POPUP.with(|p| p.borrow_mut().take_if(|(n, _)| *n == name)).map(|(_, popup)| popup);
        if own.is_some() {
            match menu(ctx, palette, "frame-menu", &mut own) {
                Some(0) => set_pinned(&win, !pinned(&win)),
                Some(1) => close = true,
                _ => {}
            }
            if let Some(popup) = own {
                FRAME_POPUP.with(|p| *p.borrow_mut() = Some((name.clone(), popup)));
            }
        }
        // the title row (the title cut where the buttons begin: a narrow window's long title does not run under them)
        let mut x = outer.right() - RIGHT as f32 - 15.0;
        // (a dockable window that is no dock area's view has a third button, this client's: into the rollup area down the
        // right edge, `rollup.rs`; a dock area's view goes there as the original's does, its tab carried to the column)
        let dockable = crate::rollup::DOCKABLE.contains(&name.as_str()) && !crate::docks::dockable(&name);
        let help = HELP_PAGES.iter().find(|h| h.0 == name).map(|h| h.1);
        let buttons_left = x - 17.0 * (1.0 + dockable as u8 as f32 + help.is_some() as u8 as f32) - 3.0;
        let title_room = egui::Rect::from_min_max(outer.min, egui::pos2(buttons_left.max(outer.left()), outer.bottom()));
        crate::font::text(ctx, &painter.with_clip_rect(title_room.intersect(painter.clip_rect())), egui::pos2(title_left, outer.top() + 5.0), &title, 15.0, accent);
        // (`WndBorder::CreateBorderIcons`, 0x1015ae85: close, the pin and, for a window with a help page, the question mark)
        for (button, what) in [("CLOSE_X", "close"), ("PIN", "pin"), ("QUESTIONMARK", "help"), ("ROLLUP_COLLAPSE", "dock")] {
            if (what == "dock" && !dockable) || (what == "help" && help.is_none()) {
                continue;
            }
            let rect = egui::Rect::from_min_size(egui::pos2(x, outer.top() + 5.0), egui::vec2(15.0, 15.0));
            let response = ui.interact(rect, id.with(what), egui::Sense::click());
            let down = what == "pin" && pinned(&win);
            let state = if response.is_pointer_button_down_on() || down { "_STATE3" } else if response.hovered() { "_STATE2" } else { "" };
            let tint = if response.hovered() { hover } else if down { rgb(palette.text_selected) } else { accent };
            let drawn = if what == "dock" {
                skin.draw_in(&painter, &format!("GFX_GUI_{button}{}", if state.is_empty() { "_STATE1" } else { state }), rect, tint)
            } else {
                skin.draw_in(&painter, &format!("GFX_GUI_WINDOW_{button}{state}"), rect, tint)
            };
            if !drawn {
                skin.draw_in(&painter, &format!("GFX_GUI_WINDOW_{button}"), rect, tint);
            }
            if response.clicked() {
                match what {
                    "pin" => set_pinned(&win, !down),
                    "dock" => {
                        crate::docks::take_out(&name);
                        crate::rollup::dock(&name);
                    }
                    "help" => HELP_ASKED.lock().unwrap().extend(help.map(|page| format!("file://{page}"))),
                    _ => close = true,
                }
            }
            response.on_hover_text(match what {
                "pin" => crate::gui::xml::ldb_named(10000, "ToggleWindowFading", "Window fading on or off"),
                "help" => crate::gui::xml::ldb_named(10000, "InfoAboutThisWindow", "About this window"),
                "dock" => "Dock in the column at the right edge".to_string(),
                _ => crate::gui::xml::ldb_named(10000, "Close", "Close"),
            });
            x -= 17.0;
        }
        result
    });
    if close {
        *open = false;
        if let Some(d) = &docked {
            crate::docks::close_others(&d.area, view);
        }
    }
    if let Some(r) = &shown {
        note_hover(ctx, &win, r.response.layer_id);
    }
    shown
}

/// A window without the frame and the title, only a background of the panel's colour: a chat window in its
/// borderless mode (`Window::SetStyle(3, … | 0xc3c)`, FUN_1009767c: the 0x800 flag keeps it out of the pointer's
/// fading and without a pin). It fades by its own rule instead, `fade` = ([`chat_fade`]) the level and the time to it
/// (FUN_100972da), from full the first time it is drawn (`SetStyle` puts a 0x800 window at 1.0 at once); the level is
/// the whole window's, its background and all it holds, as `Window::UpdateFadeLevel` sets the root view's alpha that
/// `View::_CallRender` multiplies down the tree. Its place and size are kept under `name` as a framed window's are, so
/// one can be turned into the other where it stands.
pub fn bare_window(ctx: &egui::Context, palette: &Palette, name: &str, fade: (f32, f32), build: impl FnOnce(egui::Window<'static>) -> egui::Window<'static>, add: impl FnOnce(&mut egui::Ui)) {
    if crate::layout::kept(name) && crate::layout::screen().is_none() && std::env::var_os("MAO_UI_PREVIEW").is_none() {
        return;
    }
    crate::layout::track(name);
    {
        let mut own = OWN.lock().unwrap();
        if !own.iter().any(|n| n == name) {
            own.push(name.to_string());
            drop(own);
            fade_to(name, 1.0, 0.0, 0.0);
        }
    }
    fade_toward(name, fade.0.clamp(0.0, 1.0), fade.1);
    repaint_while_fading(ctx, name);
    let level = fade_level(name);
    let saved = crate::layout::kept(name).then(|| crate::layout::saved(name)).flatten();
    let fill = egui::Color32::from_rgba_unmultiplied(palette.panel[0], palette.panel[1], palette.panel[2], (level.clamp(0.0, 1.0) * 255.0) as u8);
    const MARGIN: i8 = 4;
    let mut w = build(egui::Window::new(name).title_bar(false).collapsible(false).frame(egui::Frame::NONE.fill(fill).inner_margin(MARGIN))).id(egui::Id::unique(name));
    if let Some(size) = crate::layout::saved_size(name) {
        w = w.default_size(size);
    }
    w = match saved {
        Some(at) if crate::layout::relocate(name) => w.current_pos(at),
        Some(at) => w.default_pos(at),
        None => w,
    };
    w.show(ctx, |ui| {
        ui.multiply_opacity(level);
        let room = ui.max_rect().size();
        add(ui);
        crate::layout::note_size(name, room + egui::Vec2::splat(f32::from(MARGIN) * 2.0));
    });
}

/// The window's fill and the original's border pieces around `outer` (thin top and sides, rounded bottom corners), and the
/// original's inner border with its ground round the client.
pub(crate) fn frame_shape(skin: &crate::skin::Skin, outer: egui::Rect, panel: egui::Color32, tint: egui::Color32) -> egui::Shape {
    let mut shapes = vec![egui::Shape::rect_filled(outer.shrink(2.0), 2.0, panel)];
    let size = |name: &str| skin.get(&format!("GFX_GUI_WINDOW3_BORDER_{name}")).map(|s| s.size);
    let (Some(tl), Some(tr), Some(bl), Some(br), Some(top), Some(bottom), Some(left), Some(right)) = (size("TL"), size("TR"), size("BL"), size("BR"), size("TOP"), size("BOTTOM"), size("LEFT"), size("RIGHT")) else {
        shapes.push(egui::Shape::rect_stroke(outer, 2.0, egui::Stroke::new(1.0, tint), egui::StrokeKind::Inside));
        return egui::Shape::Vec(shapes);
    };
    let mut piece = |name: &str, rect: egui::Rect| {
        if let Some(sprite) = skin.get(&format!("GFX_GUI_WINDOW3_BORDER_{name}")) {
            shapes.push(egui::Shape::image(sprite.texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), tint));
        }
    };
    let (l, t, r, b) = (outer.left(), outer.top(), outer.right(), outer.bottom());
    piece("TL", egui::Rect::from_min_size(egui::pos2(l, t), tl));
    piece("TOP", egui::Rect::from_min_max(egui::pos2(l + tl.x, t), egui::pos2(r - tr.x, t + top.y)));
    piece("TR", egui::Rect::from_min_size(egui::pos2(r - tr.x, t), tr));
    piece("LEFT", egui::Rect::from_min_max(egui::pos2(l, t + tl.y), egui::pos2(l + left.x, b - bl.y)));
    piece("RIGHT", egui::Rect::from_min_max(egui::pos2(r - right.x, t + tr.y), egui::pos2(r, b - br.y)));
    piece("BL", egui::Rect::from_min_size(egui::pos2(l, b - bl.y), bl));
    piece("BOTTOM", egui::Rect::from_min_max(egui::pos2(l + bl.x, b - bottom.y), egui::pos2(r - br.x, b)));
    piece("BR", egui::Rect::from_min_size(egui::pos2(r - br.x, b - br.y), br));
    // (the `top_level_border` over it: `WndBorder::SetBorderGfx` (0x1015ab0e) gives a normal window a `BorderView_c` of the
    // `GFX_GUI_TAB_BORDER_*` pieces round `GFX_GUI_TAB_BACKGROUND`, in colour code 1 at full alpha, placed round the client
    // grown by a point (`DoSetFrame` 0x10159b69); the window's own ground stays the panel fill above)
    let get = |name: &str| skin.get(&format!("GFX_GUI_TAB_{name}"));
    if let (Some(bg), Some(tl), Some(top), Some(tr), Some(left), Some(right), Some(bl), Some(bottom), Some(br)) =
        (get("BACKGROUND"), get("BORDER_TL"), get("BORDER_TOP"), get("BORDER_TR"), get("BORDER_LEFT"), get("BORDER_RIGHT"), get("BORDER_BL"), get("BORDER_BOTTOM"), get("BORDER_BR"))
    {
        let inner = top_level_border(outer, skin.get("GFX_GUI_WINDOW_CLOSE_X").map_or(0.0, |s| s.size.y));
        let (l, t, r, b) = (inner.left(), inner.top(), inner.right(), inner.bottom());
        let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        for (sprite, rect) in [
            (&bg, egui::Rect::from_min_max(egui::pos2(l + left.size.x, t + top.size.y), egui::pos2(r - right.size.x, b - bottom.size.y))),
            (&top, egui::Rect::from_min_max(egui::pos2(l + tl.size.x, t), egui::pos2(r - tr.size.x, t + top.size.y))),
            (&bottom, egui::Rect::from_min_max(egui::pos2(l + bl.size.x, b - bottom.size.y), egui::pos2(r - br.size.x, b))),
            (&left, egui::Rect::from_min_max(egui::pos2(l, t + tl.size.y), egui::pos2(l + left.size.x, b - bl.size.y))),
            (&right, egui::Rect::from_min_max(egui::pos2(r - right.size.x, t + tr.size.y), egui::pos2(r, b - br.size.y))),
            (&tl, egui::Rect::from_min_size(egui::pos2(l, t), tl.size)),
            (&tr, egui::Rect::from_min_size(egui::pos2(r - tr.size.x, t), tr.size)),
            (&bl, egui::Rect::from_min_size(egui::pos2(l, b - bl.size.y), bl.size)),
            (&br, egui::Rect::from_min_size(egui::pos2(r - br.size.x, b - br.size.y), br.size)),
        ] {
            shapes.push(egui::Shape::image(sprite.texture.id(), rect, uv, tint));
        }
    }
    egui::Shape::Vec(shapes)
}

/// Where a normal window whose outer rectangle is `outer` has its `top_level_border`: its client, inside the border sizes
/// `WndBorder::UpdateBorderSizes` (0x1015a279) gives the style (3 at the sides and bottom, 24 at the top, or the frame's
/// icons' height and 10 when that is more), grown by a point each way (`DoSetFrame` 0x10159b69).
fn top_level_border(outer: egui::Rect, icons: f32) -> egui::Rect {
    egui::Rect::from_min_max(outer.min + egui::vec2(3.0, 24f32.max(icons + 10.0)), outer.max - egui::vec2(3.0, 3.0)).expand(1.0)
}

/// What `ScrollBar_c` (GUI.dll) is doing with the pointer: pressing an arrow (`HitItem_e` 1 the up, 2 the down; held, it repeats
/// after half a second every 0.08 s, `_SetButtonState` 0x10141175 / `SlotRepeatTimer` 0x10141d21) or dragging the knob (3, the
/// point grabbed below its top, `MouseDown` 0x101411fe).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ScrollPress {
    item: u8,
    grab: f32,
    repeat_at: f64,
}

/// The knob of a vertical `ScrollBar_c` in a bar `top`..`bottom` (screen points, the bottom exclusive) whose arrows are `up` and
/// `down` high, showing `proportion` of the content at `fraction` of the way down (`_CalcKnobSizeAndPos` 0x10141ab9): between
/// the arrows less a point each side, `floor(proportion × length)` long but at least 8 (less than the length when that is
/// short), placed `floor((length − knob) × fraction)` down; its top and its exclusive bottom (the original's frames are
/// inclusive, so its knob is a point longer than the number).
pub fn knob(top: f32, bottom: f32, up: f32, down: f32, proportion: f32, fraction: f32) -> (f32, f32) {
    let start = top + up;
    let length = (bottom - down - 1.0) - start;
    let mut size = (proportion * length).floor();
    if size < 8.0 {
        size = if length - 1.0 < 8.0 { length - 1.0 } else { 8.0 };
    }
    let at = start + ((length - size) * fraction.clamp(0.0, 1.0)).floor();
    (at, at + size + 1.0)
}

/// A vertical scroll area with the scroll bar of the original's scroll views (`ScrollView_c::_UpdateScrollBars` 0x10142b68 makes
/// a `ScrollBar_c` with `GFX_GUI_SIMPLE_SB_UP`, `_DOWN` and `_SLIDER_VERT` and no track, a point right of the content, as wide
/// as its arrows), tinted with the theme's border colour as the frame is: an arrow scrolls 5 points (`ScrollBar_c`'s small step,
/// 0.05 of the 0..100 its scroll view makes it with), a click above or below the knob 20 (its large step, 0.2 of it), the knob
/// drags. egui's own bar is hidden; without the skin's pictures it is egui's scroll area.
pub fn scroll<R>(ui: &mut egui::Ui, palette: &Palette, area: egui::ScrollArea, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_SIMPLE_SB_UP"));
    let Some(skin) = skin else { return area.show(ui, add).inner };
    let (Some(up), Some(down)) = (skin.get("GFX_GUI_SIMPLE_SB_UP"), skin.get("GFX_GUI_SIMPLE_SB_DOWN")) else { return area.show(ui, add).inner };
    let width = up.size.x;
    let key = ui.make_persistent_id(("mao-scroll", ui.next_auto_id()));
    let ctx = ui.ctx().clone();
    let pending = ctx.data_mut(|d| d.remove_temp::<f32>(key.with("to")));
    let mut area = area.scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).content_margin(egui::Margin { left: 0, right: (width + 1.0) as i8, top: 0, bottom: 0 });
    if let Some(to) = pending {
        area = area.vertical_scroll_offset(to);
    }
    let out = area.show(ui, add);
    let inner = out.inner_rect;
    let overflow = out.content_size.y - inner.height();
    if overflow <= 0.5 {
        return out.inner;
    }
    let bar = egui::Rect::from_min_max(egui::pos2(inner.right() - width, inner.top()), egui::pos2(inner.right(), inner.bottom()));
    let offset = out.state.clamped_offset().y.clamp(0.0, overflow);
    let (k0, k1) = knob(bar.top(), bar.bottom(), up.size.y, down.size.y, inner.height() / out.content_size.y, offset / overflow);
    let up_rect = egui::Rect::from_min_size(bar.min, up.size);
    let down_rect = egui::Rect::from_min_size(egui::pos2(bar.left(), bar.bottom() - down.size.y), down.size);
    let knob_rect = egui::Rect::from_min_max(egui::pos2(bar.left(), k0), egui::pos2(bar.right(), k1));
    let response = ui.interact(bar, key.with("bar"), egui::Sense::click_and_drag());
    let now = ctx.input(|i| i.time);
    let mut press: ScrollPress = ctx.data(|d| d.get_temp(key.with("press"))).unwrap_or_default();
    let mut to = None;
    if response.is_pointer_button_down_on() {
        let at = response.interact_pointer_pos().unwrap_or(bar.center());
        if response.drag_started() || (press.item == 0 && ctx.input(|i| i.pointer.primary_pressed())) {
            if knob_rect.contains(at) {
                press = ScrollPress { item: 3, grab: at.y - k0, repeat_at: 0.0 };
            } else if up_rect.contains(at) {
                press = ScrollPress { item: 1, grab: 0.0, repeat_at: now + 0.5 };
                to = Some(offset - 5.0);
            } else if down_rect.contains(at) {
                press = ScrollPress { item: 2, grab: 0.0, repeat_at: now + 0.5 };
                to = Some(offset + 5.0);
            } else {
                press = ScrollPress { item: 4, grab: 0.0, repeat_at: 0.0 };
                to = Some(if at.y < k0 { offset - 20.0 } else { offset + 20.0 });
            }
        } else if press.item == 3 {
            // (`MouseMove` 0x101416a0: the knob's top over the room it moves in)
            let (start, end) = (bar.top() + up.size.y, bar.bottom() - down.size.y - 1.0 - (k1 - k0 - 1.0));
            if end > start {
                to = Some(overflow * (at.y - press.grab - start) / (end - start));
            }
        } else if (press.item == 1 || press.item == 2) && now >= press.repeat_at {
            let over = if press.item == 1 { up_rect.contains(at) } else { down_rect.contains(at) };
            if over {
                to = Some(offset + if press.item == 1 { -5.0 } else { 5.0 });
            }
            press.repeat_at = now + 0.08;
        }
        ctx.request_repaint();
    } else {
        press = ScrollPress::default();
    }
    ctx.data_mut(|d| d.insert_temp(key.with("press"), press));
    if let Some(to) = to {
        ctx.data_mut(|d| d.insert_temp(key.with("to"), to.clamp(0.0, overflow).floor()));
        ctx.request_repaint();
    }
    let tint = rgb(palette.accent);
    let painter = ui.painter();
    skin.draw_in(painter, "GFX_GUI_SIMPLE_SB_UP", up_rect, tint);
    skin.draw_in(painter, "GFX_GUI_SIMPLE_SB_DOWN", down_rect, tint);
    skin.draw_in(painter, "GFX_GUI_SIMPLE_SB_SLIDER_VERT", knob_rect, tint);
    out.inner
}

/// The colour of colour code 0x1000000 (`GUIColors.xml`'s `Default`, `LoadGUIColors` 0x1006c13d) for a control's art and
/// labels, on every theme (the AOU Translucent's is near black, 0x051a2a, and the original draws its controls in it too).
pub fn code1(palette: &Palette) -> crate::theme::Rgb {
    palette.default
}

/// The size `Button_c::CalculatePreferredSize` (GUI.dll 0x10128396) gives a button whose label is `label` big and whose border
/// pieces are `left`, `top`, `right`, `bottom` (`BorderView_c::GetBorderSizes`: the left and right pieces' widths, the top and
/// bottom's heights): the label plus the sides and 10, and the label plus the top and bottom and 4.
pub fn button_size(label: egui::Vec2, left: f32, top: f32, right: f32, bottom: f32) -> egui::Vec2 {
    egui::vec2(label.x + left + right + 10.0, label.y + top + bottom + 4.0)
}

/// A `Button_c` (GUI.dll 0x10129021) in the skin's art labelled `text`: `Initialize` (0x10128e46) makes it a `BorderView_c` of
/// `GFX_GUI_BORDER01_*` in the theme's `Default` colour, its label in font 5 (Verdana 13) in the same colour and centred;
/// `StateChanged` (0x101287ea) swaps in `GFX_GUI_BORDER04_*` in `Selected` (and the label in `Selected`) while it is pressed,
/// lays the `GFX_GUI_BORDER03_*` frame in `Hover` over it while the pointer is on it, and greys the whole by 0x909090 when it
/// is disabled. Without the pictures it is egui's button.
pub fn button(ui: &mut egui::Ui, palette: &Palette, text: &str, enabled: bool) -> egui::Response {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_BORDER01_TL") && s.has("GFX_GUI_BORDER04_TL"));
    let Some(skin) = skin else { return ui.add_enabled(enabled, egui::Button::new(text)) };
    let size = |name: &str| skin.get(name).map(|s| s.size).unwrap_or(egui::Vec2::ZERO);
    let font = crate::font::verdana(ui.ctx(), 13.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, egui::Color32::WHITE);
    let want = button_size(galley.size(), size("GFX_GUI_BORDER01_LEFT").x, size("GFX_GUI_BORDER01_TOP").y, size("GFX_GUI_BORDER01_RIGHT").x, size("GFX_GUI_BORDER01_BOTTOM").y);
    let (rect, response) = ui.allocate_exact_size(want, if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let pressed = enabled && response.is_pointer_button_down_on();
    let grey = |c: egui::Color32| if enabled { c } else { c * egui::Color32::from_rgb(0x90, 0x90, 0x90) };
    let colour = grey(rgb(if pressed { palette.selected } else { code1(palette) }));
    let painter = ui.painter();
    skin.nine_slice(painter, if pressed { "GFX_GUI_BORDER04" } else { "GFX_GUI_BORDER01" }, rect, colour, true);
    if enabled && response.hovered() {
        skin.nine_slice(painter, "GFX_GUI_BORDER03", rect, grey(rgb(palette.hover)), false);
    }
    painter.galley_with_override_text_color(rect.center() - galley.size() * 0.5, galley, colour);
    response
}

/// A `CheckBox_c` (GUI.dll, `Initialize` 0x10129eb6) in the skin's art: a toggle `Button_c` whose only picture is
/// `GFX_GUI_CHECKBOX_UNCHECKED` at rest (in the theme's `Default`) and `GFX_GUI_CHECKBOX_CHECKED` when on (the pressed state, in
/// `Selected`), nothing laid over it under the pointer, then its label 10 points to the right in Verdana 13 in `Default`. A
/// click flips `on`; the response is `changed()` then. Without the pictures it is egui's checkbox.
pub fn checkbox(ui: &mut egui::Ui, palette: &Palette, on: &mut bool, text: &str) -> egui::Response {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_CHECKBOX_CHECKED") && s.has("GFX_GUI_CHECKBOX_UNCHECKED"));
    let Some(skin) = skin else { return ui.checkbox(on, text) };
    let boxed = skin.get(if *on { "GFX_GUI_CHECKBOX_CHECKED" } else { "GFX_GUI_CHECKBOX_UNCHECKED" }).map(|s| s.size).unwrap_or(egui::vec2(11.0, 11.0));
    let font = crate::font::verdana(ui.ctx(), 13.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, egui::Color32::WHITE);
    let height = boxed.y.max(galley.size().y);
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(boxed.x + 10.0 + galley.size().x, height), egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let at = egui::Rect::from_min_size(egui::pos2(rect.left(), rect.center().y - boxed.y * 0.5), boxed);
    skin.draw_in(ui.painter(), if *on { "GFX_GUI_CHECKBOX_CHECKED" } else { "GFX_GUI_CHECKBOX_UNCHECKED" }, at, rgb(if *on { palette.selected } else { code1(palette) }));
    ui.painter().galley_with_override_text_color(egui::pos2(at.right() + 10.0, rect.center().y - galley.size().y * 0.5), galley, rgb(code1(palette)));
    response
}

/// A `TextInputView_c` (GUI.dll, `Initialize` 0x10148aff) in the skin's art: its text inside a `BorderView_c` with the class's
/// own pieces (`BorderView_c::BorderView_c` 0x10126da4: `GFX_GUI_INSET_TL`, `_TOP`, `_TR`, `_LEFT`, `_RIGHT`, `_BL`, `_BOTTOM`,
/// `_BR`, no background), the text 3 points in from the sides and 1 from the top and bottom (`BorderView_c::SetClient`); the
/// pieces in the theme's `Default` (the field sets no colour of its own and takes its window's). Without the pictures it is
/// egui's text field.
pub fn text_edit(ui: &mut egui::Ui, palette: &Palette, edit: egui::TextEdit<'_>) -> egui::Response {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_INSET_TL"));
    let Some(skin) = skin else { return ui.add(edit) };
    let size = |name: &str| skin.get(name).map(|s| s.size).unwrap_or(egui::Vec2::ZERO);
    let (left, top, right, bottom) = (size("GFX_GUI_INSET_LEFT").x, size("GFX_GUI_INSET_TOP").y, size("GFX_GUI_INSET_RIGHT").x, size("GFX_GUI_INSET_BOTTOM").y);
    let margin = egui::Margin { left: (left + 3.0) as i8, right: (right + 3.0) as i8, top: (top + 1.0) as i8, bottom: (bottom + 1.0) as i8 };
    let response = ui.add(edit.frame(egui::Frame::NONE).margin(margin));
    skin.nine_slice(ui.painter(), "GFX_GUI_INSET", response.rect, rgb(code1(palette)), false);
    response
}

/// The width a strip of these tabs takes when none is narrowed ([`tab_view`]'s own widths: each tab its text's width plus
/// a point and its 5 and 16 point margins, overlapping the one before by 6, 2 points in and 2 to spare), for a window
/// that sizes itself by its tabs.
pub fn tabs_width(ui: &egui::Ui, names: &[&str]) -> f32 {
    let widths = tab_widths(ui, names);
    tab_places(&widths, f32::MAX).last().map_or(40.0, |(x, w)| x + w + 2.0) + 2.0
}

/// Each tab's least and own width (`Tab`'s preferred sizes, FUN_101463a4: its first character's width, or its whole text's,
/// and a point and its 5 and 16 point margins).
fn tab_widths(ui: &egui::Ui, names: &[&str]) -> Vec<(f32, f32)> {
    let font = crate::font::verdana(ui.ctx(), 13.0);
    let width = |t: String| ui.painter().layout_no_wrap(t, font.clone(), egui::Color32::WHITE).size().x.ceil() + 1.0 + 5.0 + 16.0;
    names.iter().map(|n| (width(n.chars().next().map(String::from).unwrap_or_default()), width(n.to_string()))).collect()
}

/// `View::SpaceOut` (GUI.dll 0x1014a44a) without weights: `space` shared among children of least sizes `least` and most
/// `most`: each its least and an equal share of what is over (or short), a child reaching its most held there and the
/// shares made again among the rest; what is still left after goes to those under their most by how far under they
/// are. The sizes it gives (not clamped: short of room they fall below the least).
pub fn space_out(space: f32, least: &[f32], most: &[f32]) -> Vec<f32> {
    let n = least.len();
    let mut out = vec![0.0f32; n];
    let mut fixed = vec![false; n];
    let mut over = space - least.iter().sum::<f32>();
    let mut count = n as f32;
    let mut start = 0;
    loop {
        let mut k = start;
        let mut capped = false;
        while k < n {
            if !fixed[k] {
                out[k] = (1.0 / count) * over + least[k];
                if most[k] <= out[k] {
                    over -= most[k] - least[k];
                    count -= 1.0;
                    out[k] = most[k];
                    fixed[k] = true;
                    if k == start {
                        start = k + 1;
                    }
                    capped = true;
                    break;
                }
            }
            k += 1;
        }
        if !capped {
            break;
        }
    }
    let left = space - out.iter().sum::<f32>();
    if left > 0.0 {
        let under: f32 = (0..n).filter(|&k| most[k] > out[k]).map(|k| most[k] - out[k]).sum();
        if under > 0.0 {
            let mut left = left;
            for k in 0..n {
                if most[k] > out[k] {
                    let more = (most[k] - out[k]) / under * left;
                    out[k] += more;
                    left -= more;
                }
            }
        }
    }
    out
}

/// Where `TopBorderView_c` (GUI.dll, laid out by FUN_10147438) puts tabs of least and own widths `widths` (see [`tab_widths`])
/// in a strip `room` wide: the first 2 points in, each overlapping the one before by 6, their widths shared out of the strip
/// less 2 each side and the overlaps (`View::SpaceOut`, [`space_out`]), each rounded and kept within its least and own
/// width; short of room they keep their least and run past the strip. The tabs' left edges and widths.
pub fn tab_places(widths: &[(f32, f32)], room: f32) -> Vec<(f32, f32)> {
    let least: Vec<f32> = widths.iter().map(|w| w.0).collect();
    let most: Vec<f32> = widths.iter().map(|w| w.1.max(w.0)).collect();
    let space = room - 4.0 + 6.0 * widths.len().saturating_sub(1) as f32;
    let shares = space_out(space, &least, &most);
    let mut x = 2.0;
    shares
        .iter()
        .enumerate()
        .map(|(k, &share)| {
            let w = if share < least[k] { least[k] } else if share > most[k] { most[k] } else { (share + 0.5).floor() };
            let at = (x, w);
            x += w - 6.0;
            at
        })
        .collect()
}

/// A `TabView` (GUI.dll 0x10146767) in the skin's art with `names` for tabs, `selected` the open one, `add` filling it: its strip
/// (`TopBorderView_c`, FUN_10147b0f / FUN_1014706e: `GFX_GUI_TAB_BORDER_TL`, `_TOP`, `_TR` along its foot) with the tabs (`Tab`,
/// FUN_1014694d: `BorderView_c`s of `GFX_GUI_TAB_INACTIVE_LEFT/MIDDLE/RIGHT` at 0.6 of the alpha with black text and the open
/// one `GFX_GUI_TAB_ACTIVE_*` in front with white text, FUN_101462d8; the text in font 5, Verdana 13, from 5 points in), then
/// the body (`TabView::SetRenderFlags` 0x101464c4 and `LayoutBorders` 0x10145dad: `GFX_GUI_TAB_BORDER_LEFT`, `_RIGHT`, `_BL`,
/// `_BOTTOM`, `_BR` round it and `GFX_GUI_TAB_BACKGROUND` inside, the content 2 points inside the borders), all tinted with the
/// theme's border colour. A tab opens when it is pressed; `add` is told which is open. Without the pictures it is a row of egui's
/// selectable labels.
pub fn tab_view<R>(ui: &mut egui::Ui, palette: &Palette, names: &[&str], selected: &mut usize, add: impl FnOnce(&mut egui::Ui, usize) -> R) -> R {
    tab_view_hits(ui, palette, names, selected, &mut TabHits::default(), add)
}

/// Where a tab view's strip and tabs were drawn, and the tab the right button was pressed and let go on (GUI.dll's `Tab`
/// FUN_10146ef3: that begins a tab's drag, `TabView::SlotBeginDrag` 0x101459dc).
#[derive(Default)]
pub struct TabHits {
    pub strip: Option<egui::Rect>,
    pub tabs: Vec<egui::Rect>,
    pub right_released: Option<usize>,
}

/// [`tab_view`], telling where its strip and tabs are ([`TabHits`]).
pub fn tab_view_hits<R>(ui: &mut egui::Ui, palette: &Palette, names: &[&str], selected: &mut usize, hits: &mut TabHits, add: impl FnOnce(&mut egui::Ui, usize) -> R) -> R {
    let skin = crate::skin::skin(ui.ctx()).filter(|s| s.has("GFX_GUI_TAB_ACTIVE_MIDDLE") && s.has("GFX_GUI_TAB_BORDER_LEFT"));
    let Some(skin) = skin else {
        let strip = ui.horizontal(|ui| {
            for (i, name) in names.iter().enumerate() {
                let response = ui.selectable_label(*selected == i, *name);
                if response.clicked() {
                    *selected = i;
                }
                if response.secondary_clicked() {
                    hits.right_released = Some(i);
                }
                hits.tabs.push(response.rect);
            }
        });
        hits.strip = Some(strip.response.rect);
        return add(ui, *selected);
    };
    let tint = rgb(palette.accent);
    let size = |name: &str| skin.get(name).map(|s| s.size).unwrap_or(egui::vec2(1.0, 1.0));
    let height = size("GFX_GUI_TAB_ACTIVE_MIDDLE").y;
    let font = crate::font::verdana(ui.ctx(), 13.0);
    let widths = tab_widths(ui, names);
    let room = ui.available_width();
    let places = tab_places(&widths, room.max(40.0));
    let need = places.last().map(|(x, w)| x + w + 2.0).unwrap_or(40.0);
    let (strip, _) = ui.allocate_exact_size(egui::vec2(need.min(room), height + 1.0), egui::Sense::hover());
    hits.strip = Some(egui::Rect::from_min_max(strip.min, egui::pos2(strip.left() + room, strip.bottom())));
    hits.tabs = places.iter().map(|&(x, w)| egui::Rect::from_min_size(egui::pos2(strip.left() + x, strip.top()), egui::vec2(w, height))).collect();
    let painter = ui.painter().clone();
    let under = painter.add(egui::Shape::Noop);
    let mut order: Vec<usize> = (0..names.len()).filter(|&i| i != *selected).collect();
    order.extend((*selected < names.len()).then_some(*selected));
    for i in order {
        let (x, w) = places[i];
        let rect = egui::Rect::from_min_size(egui::pos2(strip.left() + x, strip.top()), egui::vec2(w, height));
        let response = ui.interact(rect, ui.scope_id().with(("tab", i)), egui::Sense::click());
        // (opens when pressed, as the original's; egui may not count the tab hovered on the press's own frame, so a click
        // opens it too)
        if response.clicked() || response.is_pointer_button_down_on() || (response.hovered() && ui.input(|inp| inp.pointer.primary_pressed())) {
            *selected = i;
        }
        if response.secondary_clicked() {
            hits.right_released = Some(i);
        }
        let active = i == *selected;
        skin.three_slice(&painter, if active { "GFX_GUI_TAB_ACTIVE" } else { "GFX_GUI_TAB_INACTIVE" }, rect, if active { tint } else { tint.gamma_multiply(0.6) });
        // (FUN_101462d8: white on the open tab, black on the others, whatever the theme)
        let text = if active { egui::Color32::WHITE } else { egui::Color32::BLACK };
        // (the text is the tab's child view: what does not fit inside its right margin is cut off)
        painter.with_clip_rect(rect.intersect(egui::Rect::from_min_max(rect.min, egui::pos2(rect.right() - 12.0, rect.bottom())))).text(egui::pos2(rect.left() + 5.0, rect.center().y), egui::Align2::LEFT_CENTER, names[i], font.clone(), text);
    }
    let (left, right, bottom) = (size("GFX_GUI_TAB_BORDER_LEFT").x, size("GFX_GUI_TAB_BORDER_RIGHT").x, size("GFX_GUI_TAB_BORDER_BOTTOM").y);
    let body = ui.painter().add(egui::Shape::Noop);
    let open = *selected;
    let inner = ui.scope(|ui| egui::Frame::NONE.inner_margin(egui::Margin { left: (left + 2.0) as i8, right: (right + 2.0) as i8, top: 2, bottom: (bottom + 2.0) as i8 }).show(ui, |ui| add(ui, open)).inner);
    let content = inner.response.rect;
    let (l, r) = (strip.left(), content.right().max(strip.right()));
    let piece = |shapes: &mut Vec<egui::Shape>, name: &str, rect: egui::Rect| {
        if let Some(s) = skin.get(name) {
            shapes.push(egui::Shape::image(s.texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), tint));
        }
    };
    // (the strip's foot: its corners at the bottom left and right, the top line between)
    let (tl, tr, top) = (size("GFX_GUI_TAB_BORDER_TL"), size("GFX_GUI_TAB_BORDER_TR"), size("GFX_GUI_TAB_BORDER_TOP").y);
    let foot = strip.bottom();
    let mut line = Vec::new();
    piece(&mut line, "GFX_GUI_TAB_BORDER_TL", egui::Rect::from_min_size(egui::pos2(l, foot - tl.y), tl));
    piece(&mut line, "GFX_GUI_TAB_BORDER_TOP", egui::Rect::from_min_max(egui::pos2(l + tl.x, foot - top), egui::pos2(r - tr.x, foot)));
    piece(&mut line, "GFX_GUI_TAB_BORDER_TR", egui::Rect::from_min_size(egui::pos2(r - tr.x, foot - tr.y), tr));
    painter.set(under, egui::Shape::Vec(line));
    // (the body round the content: the sides, the bottom corners and the bottom, the background inside)
    let (t, b) = (foot, content.bottom());
    let (bl, br) = (size("GFX_GUI_TAB_BORDER_BL"), size("GFX_GUI_TAB_BORDER_BR"));
    let mut frame = Vec::new();
    piece(&mut frame, "GFX_GUI_TAB_BACKGROUND", egui::Rect::from_min_max(egui::pos2(l + left + 1.0, t + 1.0), egui::pos2(r - right - 1.0, b - bottom - 1.0)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_LEFT", egui::Rect::from_min_max(egui::pos2(l, t), egui::pos2(l + left, b - bl.y)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_RIGHT", egui::Rect::from_min_max(egui::pos2(r - right, t), egui::pos2(r, b - br.y)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_BL", egui::Rect::from_min_size(egui::pos2(l, b - bl.y), bl));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_BOTTOM", egui::Rect::from_min_max(egui::pos2(l + bl.x, b - bottom), egui::pos2(r - br.x, b)));
    piece(&mut frame, "GFX_GUI_TAB_BORDER_BR", egui::Rect::from_min_size(egui::pos2(r - br.x, b - br.y), br));
    ui.painter().set(body, egui::Shape::Vec(frame));
    inner.inner
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A normal window's `top_level_border` (`WndBorder::UpdateBorderSizes` 0x1015a279, `DoSetFrame` 0x10159b69): its
    /// client 3 in at the sides and bottom and 24 down (or the frame's icons and 10), grown by a point.
    #[test]
    fn the_top_level_border_is_round_the_client() {
        let outer = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(300.0, 200.0));
        assert_eq!(top_level_border(outer, 0.0), egui::Rect::from_min_max(egui::pos2(102.0, 73.0), egui::pos2(398.0, 248.0)));
        assert_eq!(top_level_border(outer, 15.0), egui::Rect::from_min_max(egui::pos2(102.0, 74.0), egui::pos2(398.0, 248.0)));
    }

    #[test]
    fn pins_are_kept_by_name() {
        assert!(!pinned("Test window"));
        set_pinned("Test window", true);
        assert!(pinned("Test window"));
        set_pinned("Test window", false);
        assert!(!pinned("Test window"));
    }

    /// A borderless chat window goes to its active level over 0.2 s while typed in, else to its inactive one over a
    /// second (FUN_100972da); a fade already going there is not started again; a framed chat window may fade as any.
    #[test]
    fn a_borderless_chat_window_fades_by_its_own_rule() {
        assert_eq!(chat_fade(true, 0.8, 0.3), (0.8, 0.2));
        assert_eq!(chat_fade(false, 0.8, 0.3), (0.3, 1.0));
        fade_toward("Chat:test", 0.3, 1.0);
        let first = FADES.lock().unwrap().iter().find(|f| f.0 == "Chat:test").map(|f| f.1.start).unwrap();
        fade_toward("Chat:test", 0.3, 1.0);
        let again = FADES.lock().unwrap().iter().find(|f| f.0 == "Chat:test").map(|f| f.1.start).unwrap();
        assert_eq!(first, again);
        assert!(fade_level("Chat:test") > 0.9);
        assert!(can_fade("Chat:test"));
    }

    #[test]
    fn a_button_is_its_label_and_borders() {
        // (the default skin's GFX_GUI_BORDER01 sides: 5 wide, 11 high; a Verdana 13 label about 16 high)
        assert_eq!(button_size(egui::vec2(40.0, 16.0), 5.0, 11.0, 5.0, 11.0), egui::vec2(60.0, 42.0));
    }

    #[test]
    fn tabs_overlap_by_six_from_two_in() {
        assert_eq!(tab_places(&[(30.0, 40.0), (30.0, 30.0), (30.0, 50.0)], 500.0), vec![(2.0, 40.0), (36.0, 30.0), (60.0, 50.0)]);
        // (WIN-071: too wide, `View::SpaceOut`: each its least and an equal share of the room over; a tab at its own width
        // held there, the rest shared again)
        let p = tab_places(&[(30.0, 100.0), (30.0, 40.0), (30.0, 100.0)], 140.0);
        // (room 136 and 12 of overlap: 148 for 90 least, 58 over; the second reaches 40 and keeps it; 48 for the others)
        assert_eq!(p, vec![(2.0, 54.0), (50.0, 40.0), (84.0, 54.0)]);
        // (short of room: each its least, past the strip)
        let p = tab_places(&[(30.0, 100.0), (30.0, 100.0)], 40.0);
        assert_eq!(p, vec![(2.0, 30.0), (26.0, 30.0)]);
    }

    #[test]
    fn the_knob_fills_its_share_between_the_arrows() {
        // (a 100-point bar with 9-point arrows: 81 points from a point below the one to a point above the other)
        let (a, b) = knob(0.0, 100.0, 9.0, 9.0, 0.5, 0.0);
        assert_eq!((a, b), (9.0, 50.0));
        let (a, b) = knob(0.0, 100.0, 9.0, 9.0, 0.5, 1.0);
        assert_eq!((a, b), (50.0, 91.0));
        // (never under 8)
        let (a, b) = knob(0.0, 100.0, 9.0, 9.0, 0.01, 0.0);
        assert_eq!(b - a, 9.0);
    }
}
