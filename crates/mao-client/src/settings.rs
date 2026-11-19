//! The client's settings: how the window is shown and which keys do what.
//!
//! They live in a plain `mao.ini` (the working directory's, else the user's
//! config folder; `MAO_SETTINGS=path` picks another). A missing file is
//! written with the defaults, so there is always something to edit.
//!
//! ```text
//! [window]
//! mode = windowed        ; windowed | borderless | fullscreen
//! width = 1280
//! height = 800
//!
//! [keys]
//! forward = W, Up        ; any number of keys per action; Shift+Tab, Ctrl+R ...
//! ```
//! `--window-mode`, `--width` and `--height` on the command line win over the file.
//!
//! The actions are named as the original client names its fixed key
//! commands (`KEY_COMMAND_ROTATE_CAMERA_LEFT` is `camera_rotate_left`, and so
//! on) plus the movement of this client. The default keys are the original's:
//! the rebindable ones the shipped `CharPrefs.xml`'s (`keybindings.rs`), the
//! fixed ones (camera, targets, chat, coordinates) GUI.dll's built-in key table
//! (the text at 0x101aab28 that `ParseFile` 0x1001b058 reads).

use std::collections::HashMap;
use std::path::PathBuf;

use winit::keyboard::{KeyCode, ModifiersState};

/// How the window is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowMode {
    /// A normal window with a title bar and a resizable frame.
    Windowed,
    /// A borderless window covering the screen (switching away is instant).
    Borderless,
    /// Exclusive fullscreen at the monitor's best video mode.
    Fullscreen,
}

impl WindowMode {
    pub fn name(self) -> &'static str {
        match self {
            WindowMode::Windowed => "windowed",
            WindowMode::Borderless => "borderless",
            WindowMode::Fullscreen => "fullscreen",
        }
    }

    pub fn parse(text: &str) -> Option<WindowMode> {
        match text.trim().to_ascii_lowercase().as_str() {
            "windowed" | "window" => Some(WindowMode::Windowed),
            "borderless" | "borderless-fullscreen" => Some(WindowMode::Borderless),
            "fullscreen" | "exclusive" => Some(WindowMode::Fullscreen),
            _ => None,
        }
    }

    /// The next mode in the order windowed, borderless, fullscreen.
    pub fn next(self) -> WindowMode {
        match self {
            WindowMode::Windowed => WindowMode::Borderless,
            WindowMode::Borderless => WindowMode::Fullscreen,
            WindowMode::Fullscreen => WindowMode::Windowed,
        }
    }
}

/// Something a key can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Forward,
    Back,
    TurnLeft,
    TurnRight,
    Jump,
    /// Walk through props (a debugging aid).
    NoClip,
    StrafeLeft,
    StrafeRight,
    CameraRotateLeft,
    CameraRotateRight,
    CameraRotateUp,
    CameraRotateDown,
    CameraZoomIn,
    CameraZoomOut,
    CameraReset,
    CameraSavePreferred,
    NextHostileTarget,
    PrevHostileTarget,
    NextFriendlyTarget,
    PrevFriendlyTarget,
    /// Attack the current target.
    Attack,
    /// The inventory window.
    InventoryWindow,
    /// The wear window, a window of its own (the original's `WINDOW_WEAR`).
    WearWindow,
    /// Sit down (which starts the 30 s logout countdown) or stand up (which cancels it).
    SitStand,
    /// Use the targeted thing (talk to an NPC, open a door).
    Use,
    /// The Whom-Pah window: the booths here, with a button that walks to one.
    WhompahWindow,
    /// The mission journal (`mission_window`): the character's missions.
    MissionWindow,
    /// The planet map (`map_window`, `#WindowPlanetMap`).
    PlanetMapWindow,
    /// The original's playfield map window (`WINDOW_MAP`, no key of its own in the game's files).
    MapWindow,
    /// The mail windows.
    MailWindow,
    /// The Action window: special actions, perks, social actions and dances.
    ActionsWindow,
    /// The Stats window: health, nano and armour classes.
    StatsWindow,
    /// The Skills window: the attributes and every skill with the value the original shows.
    SkillsWindow,
    /// The NCU window: the running nano programs and the NCU memory.
    NcuWindow,
    /// The friends list.
    FriendsWindow,
    /// The nano programs window.
    ProgramsWindow,
    /// Keeps walking forward until a forward or back key is pressed.
    AutoRun,
    /// Walk or run (the original's `MOVEMENT_TOGGLEWALK`).
    ToggleWalk,
    /// The help window: keys, mouse and commands.
    HelpWindow,
    /// Make the interface larger / smaller (by a tenth).
    UiLarger,
    UiSmaller,
    /// Saves a picture of the world (with the heads-up display) to the `screenshots` folder.
    Screenshot,
    /// The big map over the view (the corner map is always there).
    MapOverlay,
    /// Press slot n (0..10) of the shown hotbar.
    HotbarSlot(u8),
    /// Show hotbar n (0..6); with the original's shortcut bars, make bar n (0..10) the primary.
    HotbarBar(u8),
    /// The original's shortcut bars: show page n (0..10) of the primary bar (`SHORTUCT_BAR_ROW_n`).
    HotbarRow(u8),
    ChatInput,
    ChatCommand,
    ChatReply,
    ChatPageUp,
    ChatPageDown,
    PerkWindow,
    VehicleWindow,
    TradeskillWindow,
    /// Cycle windowed, borderless, fullscreen.
    CycleWindowMode,
    ToggleFullscreen,
    /// The options window (window mode, keys).
    ToggleOptions,
    Quit,
    /// Target the character itself (the original's F1).
    SelectSelf,
    /// The info view on the target (what `/inspect` says, and a terminal's description).
    TargetInfo,
    /// Close the windows the original closes with Escape (dialogs, containers, shops, the options...).
    CloseWindows,
    /// Say where the character is in the chat window (the original's F9).
    Coordinates,
    /// Swap between the camera's distance and a first-person-close one (the original's F8).
    CameraToggleView,
    /// A special action or character action done at once (index into [`QUICK`]).
    Quick(u8),
}

/// The special and character actions the original binds to keys: (name in the settings file, default key, the action's id in the
/// Action window, its name).
pub const QUICK: &[(&str, &str, u32, &str)] = &[
    ("action_brawl", "B", 82210, "Brawl"),
    ("action_burst", "M", 82211, "Burst"),
    ("action_dimach", "K", 82213, "Dimach"),
    ("action_fling_shot", "L", 82208, "Fling Shot"),
    ("action_full_auto", "Comma", 82209, "Full Auto"),
    ("action_reload", "V", 245003, "Reload"),
    ("action_pick_up", "R", 49578, "Pick-Up"),
    ("action_sneak", "H", 49575, "Sneak"),
    // (the original binds these four too, GUI.dll 0x1006951c: ActionSneakAttack, ActionFastAttack, ActionAimedShot,
    // ActionBowSpecialAttack: J, N, O and Period in the shipped `CharPrefs.xml`, read on 2026-11-18 (`keybindings.rs`)
    // and this client's defaults since, at the user's word: until then they started without a key)
    ("action_sneak_attack", "J", 0x1f83a, "Sneak Attack"),
    ("action_fast_attack", "N", 0x14126, "Fast Attack"),
    ("action_aimed_shot", "O", 0x14127, "Aimed Shot"),
    ("action_bow_special_attack", "Period", 0x1e159, "Bow Special Attack"),
];

/// Every action with its name in the settings file and its default keys.
/// (The targets, chat and window actions are reserved: there is no interface
/// for them yet, but the keys are already part of the settings.)
pub const ACTIONS: &[(Action, &str, &str)] = &[
    // The original's default keys (the shipped `CharPrefs.xml`'s bindings: the key codes and the hashed names of the actions).
    (Action::Forward, "forward", "W, Up"),
    (Action::Back, "back", "S, Down"),
    (Action::TurnLeft, "turn_left", "A, Left"),
    (Action::TurnRight, "turn_right", "D, Right"),
    (Action::Jump, "jump", "Space"),
    (Action::NoClip, "noclip", ""),
    (Action::StrafeLeft, "strafe_left", "Z"),
    (Action::StrafeRight, "strafe_right", "C"),
    // The camera's keys sit on the numeric pad.
    (Action::CameraRotateLeft, "camera_rotate_left", "Numpad4"),
    (Action::CameraRotateRight, "camera_rotate_right", "Numpad6"),
    (Action::CameraRotateUp, "camera_rotate_up", "Numpad2"),
    (Action::CameraRotateDown, "camera_rotate_down", "Numpad8"),
    (Action::CameraZoomIn, "camera_zoom_in", "NumpadAdd"),
    (Action::CameraZoomOut, "camera_zoom_out", "NumpadSubtract"),
    (Action::CameraReset, "camera_reset", "Numpad5"),
    (Action::CameraSavePreferred, "camera_save_preferred", "Numpad7"),
    (Action::CameraToggleView, "camera_toggle_view", "F8"),
    (Action::NextHostileTarget, "next_hostile_target", "Tab"),
    (Action::PrevHostileTarget, "prev_hostile_target", "Shift+Tab"),
    (Action::NextFriendlyTarget, "next_friendly_target", "Ctrl+Tab"),
    (Action::PrevFriendlyTarget, "prev_friendly_target", "Ctrl+Shift+Tab"),
    (Action::SelectSelf, "select_self", "F1"),
    (Action::TargetInfo, "target_info", "T"),
    (Action::Attack, "attack", "Q"),
    (Action::SitStand, "sit_stand", "X"),
    (Action::Use, "use", "E"),
    // The windows: I the inventory, Ctrl+1 the worn gear (each a window of its own), U the skills, Ctrl+2 to Ctrl+0 the others.
    (Action::InventoryWindow, "inventory", "I"),
    (Action::WearWindow, "wear_window", "Ctrl+1"),
    (Action::SkillsWindow, "skills_window", "U"),
    (Action::ActionsWindow, "actions_window", "Ctrl+2"),
    (Action::MissionWindow, "mission_window", "Ctrl+4"),
    (Action::PlanetMapWindow, "planet_map", "P, Ctrl+6"),
    (Action::MapWindow, "map_window", ""),
    // (this client's own: it had O, which is the original's aimed shot, until 2026-11-18; Ctrl+P is the user's word
    // "for now", a key the original has nothing on)
    (Action::MapOverlay, "map_overlay", "Ctrl+P"),
    (Action::FriendsWindow, "friends_window", "Ctrl+7"),
    (Action::ProgramsWindow, "programs_window", "Ctrl+8"),
    (Action::StatsWindow, "stats_window", "Ctrl+9"),
    (Action::NcuWindow, "ncu_window", "Ctrl+0"),
    (Action::PerkWindow, "open_perk_window", "Shift+P"),
    (Action::VehicleWindow, "open_vehicle_window", "Shift+V"),
    // (the key is the user's word for the original's, 2026-11-12; the action's name is this client's)
    (Action::TradeskillWindow, "tradeskill_window", "Shift+T"),
    // (This client's own windows have no key in the original: they are reached by using the booth or the terminal.)
    (Action::WhompahWindow, "whompah_window", ""),
    (Action::MailWindow, "mail_window", ""),
    (Action::HelpWindow, "help_window", ""),
    (Action::UiLarger, "ui_larger", "Ctrl+Equal"),
    (Action::UiSmaller, "ui_smaller", "Ctrl+Minus"),
    (Action::Screenshot, "screenshot", "F12"),
    (Action::Coordinates, "coordinates", "F9"),
    (Action::AutoRun, "auto_run", "Numpad0"),
    // (`MOVEMENT_TOGGLEWALK`'s key in the shipped `CharPrefs.xml`; Numpad / here until 2026-11-18)
    (Action::ToggleWalk, "toggle_walk", "Backspace"),
    // The shortcut bar: the keys 1 to 0 use the slots of the shown bar, Alt+1 to Alt+6 show another bar.
    (Action::HotbarSlot(0), "hotbar_slot_1", "1"),
    (Action::HotbarSlot(1), "hotbar_slot_2", "2"),
    (Action::HotbarSlot(2), "hotbar_slot_3", "3"),
    (Action::HotbarSlot(3), "hotbar_slot_4", "4"),
    (Action::HotbarSlot(4), "hotbar_slot_5", "5"),
    (Action::HotbarSlot(5), "hotbar_slot_6", "6"),
    (Action::HotbarSlot(6), "hotbar_slot_7", "7"),
    (Action::HotbarSlot(7), "hotbar_slot_8", "8"),
    (Action::HotbarSlot(8), "hotbar_slot_9", "9"),
    (Action::HotbarSlot(9), "hotbar_slot_10", "0"),
    (Action::HotbarBar(0), "hotbar_bar_1", "Alt+1"),
    (Action::HotbarBar(1), "hotbar_bar_2", "Alt+2"),
    (Action::HotbarBar(2), "hotbar_bar_3", "Alt+3"),
    (Action::HotbarBar(3), "hotbar_bar_4", "Alt+4"),
    (Action::HotbarBar(4), "hotbar_bar_5", "Alt+5"),
    (Action::HotbarBar(5), "hotbar_bar_6", "Alt+6"),
    // (the original's ten bars and ten pages: `SHORTUCT_BAR_SELECT_n` on Alt+1 to 0, `SHORTUCT_BAR_ROW_n` on Shift+1 to 0)
    (Action::HotbarBar(6), "hotbar_bar_7", "Alt+7"),
    (Action::HotbarBar(7), "hotbar_bar_8", "Alt+8"),
    (Action::HotbarBar(8), "hotbar_bar_9", "Alt+9"),
    (Action::HotbarBar(9), "hotbar_bar_10", "Alt+0"),
    (Action::HotbarRow(0), "hotbar_row_1", "Shift+1"),
    (Action::HotbarRow(1), "hotbar_row_2", "Shift+2"),
    (Action::HotbarRow(2), "hotbar_row_3", "Shift+3"),
    (Action::HotbarRow(3), "hotbar_row_4", "Shift+4"),
    (Action::HotbarRow(4), "hotbar_row_5", "Shift+5"),
    (Action::HotbarRow(5), "hotbar_row_6", "Shift+6"),
    (Action::HotbarRow(6), "hotbar_row_7", "Shift+7"),
    (Action::HotbarRow(7), "hotbar_row_8", "Shift+8"),
    (Action::HotbarRow(8), "hotbar_row_9", "Shift+9"),
    (Action::HotbarRow(9), "hotbar_row_10", "Shift+0"),
    // The special and character actions.
    (Action::Quick(0), "action_brawl", "B"),
    (Action::Quick(1), "action_burst", "M"),
    (Action::Quick(2), "action_dimach", "K"),
    (Action::Quick(3), "action_fling_shot", "L"),
    (Action::Quick(4), "action_full_auto", "Comma"),
    (Action::Quick(5), "action_reload", "V"),
    (Action::Quick(6), "action_pick_up", "R"),
    (Action::Quick(7), "action_sneak", "H"),
    (Action::Quick(8), "action_sneak_attack", "J"),
    (Action::Quick(9), "action_fast_attack", "N"),
    (Action::Quick(10), "action_aimed_shot", "O"),
    (Action::Quick(11), "action_bow_special_attack", "Period"),
    // The chat.
    // (GUI.dll's key table: `COMMAND_TEXT_INPUT_START_TEXT_INPUT` on Enter, Shift+Enter and Alt+Enter, the last
    // left to this client's window mode; `COMMAND_START_CHAT_CMD_INPUT` on Slash and the numeric pad's divide)
    (Action::ChatInput, "chat_input", "Enter, Shift+Enter"),
    (Action::ChatCommand, "chat_command", "Slash, NumpadDivide"),
    (Action::ChatReply, "chat_reply", "Shift+R"),
    (Action::ChatPageUp, "chat_history_page_up", "PageUp"),
    (Action::ChatPageDown, "chat_history_page_down", "PageDown"),
    // This client.
    (Action::CloseWindows, "close_windows", "Escape"),
    (Action::CycleWindowMode, "cycle_window_mode", "Alt+Enter"),
    (Action::ToggleFullscreen, "toggle_fullscreen", "F11"),
    (Action::ToggleOptions, "options", "F10"),
    (Action::Quit, "quit", "Ctrl+Q"),
];

/// The version of the key table in the settings file: a file written for another version has its keys replaced by the defaults (the
/// first versions used keys of this client's own), but for version 2's, which are kept and brought up to date
/// ([`MOVED_DEFAULTS`]).
pub const KEYMAP_VERSION: u32 = 3;

/// The default keys that changed from version 2 of the key table to version 3 (2026-11-18, the user's "match the
/// original": the shipped `CharPrefs.xml`'s keys for walk/run and four special attacks, and the map overlay off O,
/// which is the aimed shot's there, onto Ctrl+P): the action, its keys in version 2 and its keys now. A version 2 file's action
/// that still has its old default gets the new one, when no other action of the file has that key; a key the player
/// chose stays.
pub const MOVED_DEFAULTS: &[(Action, &str, &str)] = &[
    (Action::MapOverlay, "O", "Ctrl+P"),
    (Action::ToggleWalk, "NumpadDivide", "Backspace"),
    (Action::Quick(8), "", "J"),
    (Action::Quick(9), "", "N"),
    (Action::Quick(10), "", "O"),
    (Action::Quick(11), "", "Period"),
];

/// Whether the server's messages go into the game's log now (`[debug] log_messages`, `/setoption log_messages on`,
/// `MAO_TRACE=1`): set when the game starts and when the setting is changed in it.
pub static LOG_MESSAGES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A key with the modifiers that must be held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Binding {
    pub key: KeyCode,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Binding {
    pub fn matches(&self, key: KeyCode, modifiers: ModifiersState) -> bool {
        self.key == key && self.shift == modifiers.shift_key() && self.ctrl == modifiers.control_key() && self.alt == modifiers.alt_key()
    }
}

/// The names keys have in the settings file.
const KEY_NAMES: &[(&str, KeyCode)] = &[
    ("Space", KeyCode::Space),
    ("Tab", KeyCode::Tab),
    ("Enter", KeyCode::Enter),
    ("Escape", KeyCode::Escape),
    ("Backspace", KeyCode::Backspace),
    ("Insert", KeyCode::Insert),
    ("Delete", KeyCode::Delete),
    ("Home", KeyCode::Home),
    ("End", KeyCode::End),
    ("PageUp", KeyCode::PageUp),
    ("PageDown", KeyCode::PageDown),
    ("Up", KeyCode::ArrowUp),
    ("Down", KeyCode::ArrowDown),
    ("Left", KeyCode::ArrowLeft),
    ("Right", KeyCode::ArrowRight),
    ("Comma", KeyCode::Comma),
    ("Period", KeyCode::Period),
    ("Slash", KeyCode::Slash),
    ("Backslash", KeyCode::Backslash),
    ("Semicolon", KeyCode::Semicolon),
    ("Quote", KeyCode::Quote),
    ("Minus", KeyCode::Minus),
    ("Equal", KeyCode::Equal),
    ("BracketLeft", KeyCode::BracketLeft),
    ("BracketRight", KeyCode::BracketRight),
    ("Backquote", KeyCode::Backquote),
    ("NumpadAdd", KeyCode::NumpadAdd),
    ("NumpadSubtract", KeyCode::NumpadSubtract),
    ("NumpadMultiply", KeyCode::NumpadMultiply),
    ("NumpadDivide", KeyCode::NumpadDivide),
    ("NumpadEnter", KeyCode::NumpadEnter),
    ("Numpad0", KeyCode::Numpad0),
    ("Numpad1", KeyCode::Numpad1),
    ("Numpad2", KeyCode::Numpad2),
    ("Numpad3", KeyCode::Numpad3),
    ("Numpad4", KeyCode::Numpad4),
    ("Numpad5", KeyCode::Numpad5),
    ("Numpad6", KeyCode::Numpad6),
    ("Numpad7", KeyCode::Numpad7),
    ("Numpad8", KeyCode::Numpad8),
    ("Numpad9", KeyCode::Numpad9),
    ("F1", KeyCode::F1),
    ("F2", KeyCode::F2),
    ("F3", KeyCode::F3),
    ("F4", KeyCode::F4),
    ("F5", KeyCode::F5),
    ("F6", KeyCode::F6),
    ("F7", KeyCode::F7),
    ("F8", KeyCode::F8),
    ("F9", KeyCode::F9),
    ("F10", KeyCode::F10),
    ("F11", KeyCode::F11),
    ("F12", KeyCode::F12),
    ("A", KeyCode::KeyA),
    ("B", KeyCode::KeyB),
    ("C", KeyCode::KeyC),
    ("D", KeyCode::KeyD),
    ("E", KeyCode::KeyE),
    ("F", KeyCode::KeyF),
    ("G", KeyCode::KeyG),
    ("H", KeyCode::KeyH),
    ("I", KeyCode::KeyI),
    ("J", KeyCode::KeyJ),
    ("K", KeyCode::KeyK),
    ("L", KeyCode::KeyL),
    ("M", KeyCode::KeyM),
    ("N", KeyCode::KeyN),
    ("O", KeyCode::KeyO),
    ("P", KeyCode::KeyP),
    ("Q", KeyCode::KeyQ),
    ("R", KeyCode::KeyR),
    ("S", KeyCode::KeyS),
    ("T", KeyCode::KeyT),
    ("U", KeyCode::KeyU),
    ("V", KeyCode::KeyV),
    ("W", KeyCode::KeyW),
    ("X", KeyCode::KeyX),
    ("Y", KeyCode::KeyY),
    ("Z", KeyCode::KeyZ),
    ("0", KeyCode::Digit0),
    ("1", KeyCode::Digit1),
    ("2", KeyCode::Digit2),
    ("3", KeyCode::Digit3),
    ("4", KeyCode::Digit4),
    ("5", KeyCode::Digit5),
    ("6", KeyCode::Digit6),
    ("7", KeyCode::Digit7),
    ("8", KeyCode::Digit8),
    ("9", KeyCode::Digit9),
];

/// `Shift+Tab`, `ctrl+r`, `W`: a binding from its text.
pub fn parse_binding(text: &str) -> Option<Binding> {
    let mut binding = Binding { key: KeyCode::Escape, shift: false, ctrl: false, alt: false };
    let mut key = None;
    for part in text.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "shift" => binding.shift = true,
            "ctrl" | "control" => binding.ctrl = true,
            "alt" => binding.alt = true,
            other => key = KEY_NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(other)).map(|&(_, code)| code),
        }
    }
    binding.key = key?;
    Some(binding)
}

/// The text of a binding (the inverse of [`parse_binding`]).
pub fn binding_name(binding: &Binding) -> String {
    let mut parts = Vec::new();
    if binding.ctrl {
        parts.push("Ctrl");
    }
    if binding.shift {
        parts.push("Shift");
    }
    if binding.alt {
        parts.push("Alt");
    }
    parts.push(KEY_NAMES.iter().find(|&&(_, code)| code == binding.key).map_or("?", |&(name, _)| name));
    parts.join("+")
}

/// Which keys do what.
#[derive(Clone, Debug)]
pub struct Keymap {
    bindings: HashMap<Action, Vec<Binding>>,
    /// The keys the character playing rebound in the original client (`keybindings.rs`), which stand for the
    /// session in place of this client's own keys of those actions. Not part of the settings file.
    character: HashMap<Action, Vec<Binding>>,
}

impl Default for Keymap {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        for &(action, _, keys) in ACTIONS {
            bindings.insert(action, keys.split(',').filter_map(parse_binding).collect());
        }
        Keymap { bindings, character: HashMap::new() }
    }
}

impl Keymap {
    /// An action's keys in force: the character's own from the original where it has any, else this client's.
    fn of(&self, action: &Action) -> Option<&Vec<Binding>> {
        self.character.get(action).or_else(|| self.bindings.get(action))
    }

    /// The action a key press means, if any (bindings with more modifiers win). An action with keys of the
    /// character's own from the original comes before the others: there the rebindable keys are asked before the
    /// fixed ones (`InputConfig_t::ProcessInput` 0x1001a759).
    pub fn action(&self, key: KeyCode, modifiers: ModifiersState) -> Option<Action> {
        let hit = |a: &Action| self.of(a).is_some_and(|list| list.iter().any(|b| b.matches(key, modifiers)));
        let actions = || ACTIONS.iter().map(|&(a, _, _)| a);
        actions().filter(|a| self.character.contains_key(a)).find(hit).or_else(|| actions().find(hit))
    }

    /// Lays the keys a character rebound in the original over this client's, for the actions whose key was not
    /// changed here (one that is no longer this client's default was chosen in this client, the later word). The
    /// actions taken over.
    pub fn set_character(&mut self, keys: HashMap<Action, Vec<Binding>>) -> Vec<Action> {
        let defaults = Keymap::default();
        self.character = keys.into_iter().filter(|(action, _)| self.bindings.get(action) == defaults.bindings.get(action)).collect();
        ACTIONS.iter().map(|&(a, _, _)| a).filter(|a| self.character.contains_key(a)).collect()
    }

    /// An action's keys as the settings file keeps them: this client's own, without the character's from the
    /// original.
    pub fn saved_keys(&self, action: Action) -> String {
        self.bindings.get(&action).map(|list| list.iter().map(binding_name).collect::<Vec<_>>().join(", ")).unwrap_or_default()
    }

    /// [`Keymap::action`] for a physical key (none for keys without a code).
    pub fn action_any(&self, key: winit::keyboard::PhysicalKey, modifiers: ModifiersState) -> Option<Action> {
        match key {
            winit::keyboard::PhysicalKey::Code(code) => self.action(code, modifiers),
            _ => None,
        }
    }

    /// Every action a key release can end (modifiers don't matter on release).
    pub fn actions_for_key(&self, key: KeyCode) -> Vec<Action> {
        ACTIONS.iter().map(|&(a, _, _)| a).filter(|a| self.of(a).is_some_and(|list| list.iter().any(|b| b.key == key))).collect()
    }

    /// Makes `binding` the only key of `action`, taking it from any other action. (A key chosen here is this
    /// client's: the character's own from the original no longer stands for the action.)
    pub fn assign(&mut self, action: Action, binding: Binding) {
        for list in self.bindings.values_mut().chain(self.character.values_mut()) {
            list.retain(|b| *b != binding);
        }
        self.character.remove(&action);
        self.bindings.insert(action, vec![binding]);
    }

    pub fn set(&mut self, action: Action, keys: &str) {
        self.character.remove(&action);
        self.bindings.insert(action, keys.split(',').filter_map(parse_binding).collect());
    }

    /// The key this client has one of the original's key commands on (the `KEY_...` ids of the skins'
    /// `OptionPanel/HotKeys.xml`, the option window's "Fixed keys"): none for a command this client does not have
    /// (the chat's history keys, its reply key, the place said to the chat) or has no key for.
    pub fn original_key(&self, id: &str) -> Option<String> {
        let action = match id {
            "KEY_NEXT_FRIENDLY_TARGET" => Action::NextFriendlyTarget,
            "KEY_PREV_FRIENDLY_TARGET" => Action::PrevFriendlyTarget,
            "KEY_NEXT_HOSTILE_TARGET" => Action::NextHostileTarget,
            "KEY_PREV_HOSTILE_TARGET" => Action::PrevHostileTarget,
            "KEY_COMMAND_ROTATE_CAMERA_LEFT" => Action::CameraRotateLeft,
            "KEY_COMMAND_ROTATE_CAMERA_RIGHT" => Action::CameraRotateRight,
            "KEY_COMMAND_ROTATE_CAMERA_UP" => Action::CameraRotateUp,
            "KEY_COMMAND_ROTATE_CAMERA_DOWN" => Action::CameraRotateDown,
            "KEY_COMMAND_ZOOM_CAMERA_IN" => Action::CameraZoomIn,
            "KEY_COMMAND_ZOOM_CAMERA_OUT" => Action::CameraZoomOut,
            "KEY_COMMAND_CAMERA_SET_PREFERRED_POS" => Action::CameraSavePreferred,
            "KEY_COMMAND_CAMERA_RESET" => Action::CameraReset,
            _ => return None,
        };
        let names: Vec<String> = self.bindings_of(action).iter().map(binding_name).collect();
        (!names.is_empty()).then(|| names.join(", "))
    }

    /// The bindings of an action (none when it is unbound).
    pub fn bindings_of(&self, action: Action) -> Vec<Binding> {
        self.of(&action).cloned().unwrap_or_default()
    }

    pub fn keys(&self, action: Action) -> String {
        self.of(&action).map(|list| list.iter().map(binding_name).collect::<Vec<_>>().join(", ")).unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub mode: WindowMode,
    pub width: u32,
    pub height: u32,
    /// The graphics device by its name (`mao_render::adapter_names`); empty for the system's choice.
    pub adapter: String,
    /// The most frames a second (0: no limit beyond the monitor's when `vsync` is on), and the most while the window is
    /// not the one in front (0: the same).
    pub fps_limit: u32,
    pub background_fps: u32,
    /// Keep the textures on the graphics card block-compressed (a quarter of the video memory; `mao_render::block_levels`).
    pub texture_compression: bool,
    /// The sun's shadows: off, or from a small, middling or large shadow map.
    pub shadows: mao_render::Shadows,
    /// The monitor a borderless or full-screen window covers (0: the one the window is on; else the system's n-th).
    pub monitor: u32,
    /// The original game's folder (the one with `cd_image` in it), which the client reads everything from; the launcher's
    /// settings set it, and the launcher started without a folder uses it.
    pub game_dir: String,
    /// Wait for the monitor's refresh (capped at its rate); off = uncapped.
    pub vsync: bool,
    /// The share of the window the scene is drawn at before FSR 1 upscales it (1 = native).
    pub render_scale: f32,
    /// FSR's sharpening in stops (0 = most).
    pub sharpness: f32,
    /// How the scene's edges are smoothed: off, 4x multisampling or temporal (jittered frames blended by their motion).
    pub antialiasing: mao_render::Antialiasing,
    /// The interface's size (1.0 is as large as the window's scale makes it).
    pub ui_scale: f32,
    /// How far the world is seen: where the fog ends (the live view).
    pub view_distance: f32,
    /// How far characters and creatures are drawn (0: all that are known).
    pub char_distance: f32,
    /// Take everything from a corpse as soon as it opens.
    pub auto_loot: bool,
    /// The interface colours: `built-in`, `original`, `aou_translucent` ... or a folder
    /// (see [`crate::theme`]).
    pub theme: String,
    /// The AOU mod's folder (the one with `Gui` in it), for its themes.
    pub aou_mod: String,
    /// A skin folder (one with `Graphics.uvga`), searched before the theme's folder and the game's own (see [`crate::skin`]).
    pub gui_dir: String,
    /// The developer overlay: the location and coordinates, the frame rate and the shortcut bar's hint.
    pub debug_overlay: bool,
    /// The performance overlay: frame rate and times, processor, memory and the graphics card (`/perf`).
    pub perf_overlay: bool,
    /// The game's log (`game-log.txt` beside the settings, written by a game the login starts) also gets the server's
    /// messages as they come (`MAO_TRACE`): for finding out why something went wrong. Off unless asked for.
    pub log_messages: bool,
    /// The chat lines in the original's own bitmap font.
    pub original_font: bool,
    /// This client's own smart hotbars instead of the original's shortcut bars.
    pub smart_hotbar: bool,
    /// This client's own round corner map and its overlay, on by default (the user's choice, 2026-11-18); the original's
    /// square playfield map window opens from the control center's Map menu either way. Kept as `round_map`: the first
    /// `own_map` setting was written off by the build that made the square window the default.
    pub own_map: bool,
    /// How many of the original's shortcut bars show (`NumHotbars`, 1 to 10) and whether they are locked (`LockHotbars`).
    pub hotbars: u8,
    pub lock_hotbars: bool,
    /// How the third-person camera follows a turning character: `lock` (stays behind), `rubber` (swings round) or `trail` (slowly).
    pub camera_mode: String,
    /// The planet maps (the original's `PlanetMapIndexFile` and `ShadowlandMapIndexFile`): an index file's path from a
    /// maps' folder (`crate::planetmaps`).
    pub planet_map: String,
    pub shadowlands_map: String,
    /// The pet window's icon menu (`PetWindowConfig` `ShowButtons`, `EnableFixedPetButtons`): the buttons beside each
    /// pet's bar, and the pets held in fixed rows by their kind rather than the order they came.
    pub pet_buttons: bool,
    pub fixed_pet_buttons: bool,
    /// The pet window's "Remember Pet Names" (`AutoPetRenaming`, on in the game's `CharPrefs.xml`): a pet that comes
    /// is given the name last kept for its kind.
    pub remember_pet_names: bool,
    pub keymap: Keymap,
}

/// The planet maps used unless another is chosen: Saavick's Map of Rubi-Ka and SLmap.
pub const DEFAULT_PLANET_MAP: &str = "SaavicksMap/saavicksmap.txt";
pub const DEFAULT_SHADOWLANDS_MAP: &str = "SLmap/SLmap.txt";

impl Default for Settings {
    fn default() -> Self {
        Settings { mode: WindowMode::Windowed, width: 1280, height: 800, adapter: String::new(), game_dir: String::new(), fps_limit: 0, background_fps: 30, shadows: mao_render::Shadows::High, texture_compression: true, monitor: 0, vsync: false, render_scale: 1.0, sharpness: 0.2, antialiasing: mao_render::Antialiasing::default(), ui_scale: 1.0, view_distance: 1800.0, char_distance: 0.0, auto_loot: false, theme: "original".into(), aou_mod: String::new(), gui_dir: String::new(), debug_overlay: false, perf_overlay: false, log_messages: false, original_font: true, own_map: true, smart_hotbar: false, hotbars: 1, lock_hotbars: false, camera_mode: "trail".into(), planet_map: DEFAULT_PLANET_MAP.into(), shadowlands_map: DEFAULT_SHADOWLANDS_MAP.into(), pet_buttons: true, fixed_pet_buttons: false, remember_pet_names: true, keymap: Keymap::default() }
    }
}

/// How long before a limited frame is due the window's thread stops sleeping and waits awake (`viewer.rs`).
pub const LIMIT_SPIN_MICROS: u64 = 2000;

/// The frame limit in force: the background's while the window is not in front and has one, else the general one; none
/// when that is 0.
pub fn frame_limit(limit: u32, background: u32, focused: bool) -> Option<u32> {
    let fps = if !focused && background != 0 { if limit != 0 { background.min(limit) } else { background } } else { limit };
    (fps != 0).then_some(fps)
}

/// When the next frame is due under a limit: a step after the last one was, or now when that is already more than a
/// step ago (a slow frame is not made up for with a burst).
pub fn next_frame_due(last_due: std::time::Instant, now: std::time::Instant, fps: u32) -> std::time::Instant {
    let step = std::time::Duration::from_secs_f64(1.0 / fps.max(1) as f64);
    let due = last_due + step;
    if due + step < now { now } else { due }
}

impl Settings {
    /// `/setoption name value`: sets one of the client's settings and says what happened; without a name, lists them.
    /// Changes the interface scale by `steps` tenths (kept between 0.6 and 2.0).
    pub fn step_ui_scale(&mut self, steps: i32) {
        self.ui_scale = (((self.ui_scale * 10.0).round() + steps as f32) / 10.0).clamp(0.6, 2.0);
    }

    pub fn set_option(&mut self, name: &str, value: &str) -> String {
        let on = |v: &str| matches!(v.to_lowercase().as_str(), "on" | "true" | "1" | "yes");
        let number = |v: &str, lo: f32, hi: f32| v.parse::<f32>().ok().map(|n| n.clamp(lo, hi));
        if name.is_empty() {
            return format!(
                "vsync {}, render_scale {}, sharpness {}, ui_scale {}, view_distance {}, char_distance {}, auto_loot {}, antialiasing {}",
                if self.vsync { "on" } else { "off" },
                self.render_scale,
                self.sharpness,
                self.ui_scale,
                self.view_distance,
                self.char_distance,
                if self.auto_loot { "on" } else { "off" },
                self.antialiasing.name()
            );
        }
        let done = match name.to_lowercase().as_str() {
            "vsync" if !value.is_empty() => {
                self.vsync = on(value);
                true
            }
            "auto_loot" | "autoloot" if !value.is_empty() => {
                self.auto_loot = on(value);
                true
            }
            // (the camera: the original's free camera, which keeps its direction and is pulled along (`free`, the file's
            // `trail`), the rubber one that swings in behind the character as it turns, or the locked one)
            "camera_mode" | "camera" => match value.to_lowercase().as_str() {
                "free" | "trail" => {
                    self.camera_mode = "trail".to_string();
                    true
                }
                "rubber" => {
                    self.camera_mode = "rubber".to_string();
                    true
                }
                "lock" | "locked" => {
                    self.camera_mode = "lock".to_string();
                    true
                }
                _ => return "camera_mode needs free, rubber or lock.".to_string(),
            },
            // (the server's messages in the game's log, from now on and in the games to come)
            "log_messages" if !value.is_empty() => {
                self.log_messages = on(value);
                LOG_MESSAGES.store(self.log_messages, std::sync::atomic::Ordering::Relaxed);
                true
            }
            "render_scale" => number(value, 0.4, 1.0).map(|n| self.render_scale = n).is_some(),
            "sharpness" => number(value, 0.0, 2.0).map(|n| self.sharpness = n).is_some(),
            "antialiasing" | "aa" => mao_render::Antialiasing::from_name(value).map(|a| self.antialiasing = a).is_some(),
            "ui_scale" => number(value, 0.6, 2.0).map(|n| self.ui_scale = n).is_some(),
            "view_distance" | "viewdist" => number(value, 200.0, 4000.0).map(|n| self.view_distance = n).is_some(),
            "char_distance" | "chardist" => number(value, 0.0, 4000.0).map(|n| self.char_distance = n).is_some(),
            _ => return format!("No setting called {name:?} (vsync, render_scale, sharpness, antialiasing, ui_scale, view_distance, char_distance, auto_loot, camera_mode, log_messages)."),
        };
        if done {
            format!("{name} is now {value}.")
        } else {
            format!("{name} needs a value (a number, or on/off).")
        }
    }

    /// The settings text: the window options, then every action's keys.
    pub fn to_text(&self) -> String {
        let mut out = String::from("; Modern Anarchy Online settings\n[window]\n; windowed | borderless | fullscreen (F11 / Alt+Enter change it while playing)\n");
        out += &format!("mode = {}\nwidth = {}\nheight = {}\n; the graphics device (empty: the system's choice; the launcher's settings list them)\nadapter = {}\nvsync = {}\n; the most frames a second (0: no limit), and while the window is not in front (0: the same)\nfps_limit = {}\nbackground_fps = {}\n; off | low | medium | high\nshadows = {}\n; the textures block-compressed on the graphics card (a quarter of the video memory, a little longer to load)\ntexture_compression = {}\n; the monitor a borderless or full-screen window covers (0: the window's own)\nmonitor = {}\nrender_scale = {}\nsharpness = {}\nantialiasing = {}\nui_scale = {}\nview_distance = {}\nchar_distance = {}\nauto_loot = {}\n\n", self.mode.name(), self.width, self.height, self.adapter, if self.vsync { "on" } else { "off" }, self.fps_limit, self.background_fps, self.shadows.name(), if self.texture_compression { "on" } else { "off" }, self.monitor, self.render_scale, self.sharpness, self.antialiasing.name(), self.ui_scale, self.view_distance, self.char_distance, if self.auto_loot { "on" } else { "off" });
        out += &format!("[interface]\n; built-in | original | aou_translucent | aou_cyt | aou_slim_cni | aou_slim_cnt | a folder with GUIColors.xml\ntheme = {}\n; the AOU mod's folder (the one with Gui in it), for its themes\naou_mod = {}\n; a skin folder (Graphics.uvga and Graphics.uvgi, as the original's Gui/<skin> folders), used before the theme's and the game's own pictures\ngui_dir = {}\n; show the location and coordinates, the frame rate and key hints over the game (for developers)\ndebug_overlay = {}\n; the frame rate, processor, memory and graphics card over the game (/perf)\nperf_overlay = {}\n; the chat lines in the original's own bitmap font\noriginal_font = {}\n; the third-person camera: lock (stays behind the character), rubber (swings round behind it), trail (slowly)\ncamera_mode = {}\n; the planet maps: an index file under a maps' folder (the client's PlanetMap folder, then the game's textures/PlanetMap)\nplanet_map = {}\nshadowlands_map = {}\n; the shortcut bars: the original's (smart_hotbar = off; how many show, 1 to 10, and whether they are locked) or this client's smart hotbars\nsmart_hotbar = {}\nhotbars = {}\nlock_hotbars = {}\n; this client's own round corner map and its overlay (on), beside the original's square playfield map window (the Map menu)\nround_map = {}\n\n", self.theme, self.aou_mod, self.gui_dir, if self.debug_overlay { "on" } else { "off" }, if self.perf_overlay { "on" } else { "off" }, if self.original_font { "on" } else { "off" }, self.camera_mode, self.planet_map, self.shadowlands_map, if self.smart_hotbar { "on" } else { "off" }, self.hotbars, if self.lock_hotbars { "on" } else { "off" }, if self.own_map { "on" } else { "off" });
        out += &format!("; the pet window: the buttons beside each pet's bar, and the pets in fixed rows by their kind\npet_buttons = {}\nfixed_pet_buttons = {}\n; a pet that comes is given the name last kept for its kind\nremember_pet_names = {}\n\n", if self.pet_buttons { "on" } else { "off" }, if self.fixed_pet_buttons { "on" } else { "off" }, if self.remember_pet_names { "on" } else { "off" });
        out += &format!("[game]\n; the original game's folder (the one with cd_image in it); the launcher's settings set it\ndir = {}\n\n", self.game_dir);
        out += &crate::helpwindow::settings_text();
        out += &format!("[debug]\n; on: game-log.txt (beside this file) also gets the server's messages as they come\nlog_messages = {}\n\n", if self.log_messages { "on" } else { "off" });
        out += "[keys]\n; one or more keys per action, separated by commas; Shift+, Ctrl+ and Alt+ combine\n";
        out += &format!("version = {KEYMAP_VERSION}\n");
        for &(action, name, _) in ACTIONS {
            out += &format!("{name} = {}\n", self.keymap.saved_keys(action));
        }
        out
    }

    /// Reads settings text over the defaults; unknown lines are reported and skipped.
    pub fn from_text(text: &str) -> Settings {
        let mut settings = Settings::default();
        let mut section = String::new();
        // (the key lines wait until the file's key version is known)
        let mut key_lines: Vec<(usize, String, String)> = Vec::new();
        let mut key_version = 0u32;
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split(';').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.trim().to_ascii_lowercase();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                eprintln!("settings line {}: expected name = value: {raw:?}", n + 1);
                continue;
            };
            let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
            match (section.as_str(), key.as_str()) {
                ("window", "mode") => match WindowMode::parse(value) {
                    Some(m) => settings.mode = m,
                    None => eprintln!("settings line {}: unknown window mode {value:?}", n + 1),
                },
                ("window", "adapter") => settings.adapter = value.to_string(),
                ("window", "fps_limit") => settings.fps_limit = value.parse().unwrap_or(settings.fps_limit).min(1000),
                ("window", "background_fps") => settings.background_fps = value.parse().unwrap_or(settings.background_fps).min(1000),
                ("window", "texture_compression") => settings.texture_compression = !matches!(value.to_ascii_lowercase().as_str(), "off" | "false" | "0" | "no"),
                ("window", "shadows") => settings.shadows = mao_render::Shadows::from_name(value).unwrap_or(settings.shadows),
                ("window", "monitor") => settings.monitor = value.parse().unwrap_or(settings.monitor).min(16),
                ("window", "width") => settings.width = value.parse().unwrap_or(settings.width).clamp(320, 16384),
                ("window", "render_scale") => settings.render_scale = value.parse().unwrap_or(settings.render_scale).clamp(0.4, 1.0),
                ("window", "antialiasing") => settings.antialiasing = mao_render::Antialiasing::from_name(value).unwrap_or(settings.antialiasing),
                ("window", "sharpness") => settings.sharpness = value.parse().unwrap_or(settings.sharpness).clamp(0.0, 4.0),
                ("window", "auto_loot") => settings.auto_loot = value == "on" || value == "true",
                ("window", "char_distance") => settings.char_distance = value.parse().unwrap_or(settings.char_distance).clamp(0.0, 4000.0),
                ("window", "view_distance") => settings.view_distance = value.parse().unwrap_or(settings.view_distance).clamp(150.0, 6000.0),
                ("window", "ui_scale") => settings.ui_scale = value.parse().unwrap_or(settings.ui_scale).clamp(0.5, 3.0),
                ("window", "vsync") => settings.vsync = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("window", "height") => settings.height = value.parse().unwrap_or(settings.height).clamp(240, 16384),
                ("interface", "theme") => settings.theme = value.to_string(),
                ("interface", "aou_mod") => settings.aou_mod = value.to_string(),
                ("interface", "planet_map") => settings.planet_map = value.to_string(),
                ("interface", "shadowlands_map") => settings.shadowlands_map = value.to_string(),
                ("interface", "gui_dir") => settings.gui_dir = value.to_string(),
                ("interface", "camera_mode") if ["lock", "rubber", "trail"].contains(&value.to_ascii_lowercase().as_str()) => settings.camera_mode = value.to_ascii_lowercase(),
                ("interface", "original_font") => settings.original_font = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("interface", "round_map") => settings.own_map = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("interface", "smart_hotbar") => settings.smart_hotbar = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("interface", "hotbars") => settings.hotbars = value.parse().unwrap_or(settings.hotbars).clamp(1, 10),
                ("interface", "lock_hotbars") => settings.lock_hotbars = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("interface", "perf_overlay") => settings.perf_overlay = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("debug", "log_messages") => settings.log_messages = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("interface", "debug_overlay") => settings.debug_overlay = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("interface", "pet_buttons") => settings.pet_buttons = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("interface", "fixed_pet_buttons") => settings.fixed_pet_buttons = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("interface", "remember_pet_names") => settings.remember_pet_names = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("game", "dir") => settings.game_dir = value.to_string(),
                // (the help assistant's own section: `helpwindow.rs` keeps it)
                ("help", name) if crate::helpwindow::configure(name, value) => {}
                ("keys", "version") => key_version = value.parse().unwrap_or(0),
                ("keys", name) => key_lines.push((n, name.to_string(), value.to_string())),
                _ => eprintln!("settings line {}: unknown setting {key:?}", n + 1),
            }
        }
        if key_version == KEYMAP_VERSION || key_version == 2 {
            // (a version 2 file starts from version 2's defaults, so an action it has no line for is as it was)
            if key_version == 2 {
                for &(action, old, _) in MOVED_DEFAULTS {
                    settings.keymap.set(action, old);
                }
            }
            for (n, name, value) in key_lines {
                match ACTIONS.iter().find(|(_, n, _)| *n == name) {
                    Some(&(action, _, _)) => {
                        let bad: Vec<&str> = value.split(',').map(str::trim).filter(|k| !k.is_empty() && parse_binding(k).is_none()).collect();
                        if !bad.is_empty() {
                            eprintln!("settings line {}: unknown keys {bad:?}", n + 1);
                        }
                        // (the overlay had Ctrl+6, the planet map's key in the original, before the planet map window)
                        if action == Action::MapOverlay && value.trim().eq_ignore_ascii_case("Ctrl+6") {
                            continue;
                        }
                        settings.keymap.set(action, &value);
                    }
                    None => eprintln!("settings line {}: unknown action {name:?}", n + 1),
                }
            }
            // (version 2's keys brought up to date: `MOVED_DEFAULTS`, in its order, so the overlay lets go of O
            // before the aimed shot asks for it)
            if key_version == 2 {
                let keys = |text: &str| -> Vec<Binding> { text.split(',').filter_map(parse_binding).collect() };
                for &(action, old, new) in MOVED_DEFAULTS {
                    let free = keys(new).iter().all(|b| ACTIONS.iter().all(|&(other, _, _)| other == action || !settings.keymap.bindings_of(other).contains(b)));
                    if settings.keymap.bindings_of(action) == keys(old) && free {
                        settings.keymap.set(action, new);
                    }
                }
            }
        } else if !key_lines.is_empty() {
            eprintln!("the keys of an older settings file are replaced by the original's defaults");
        }
        settings
    }

    /// Writes the settings to the file (what the options window does).
    pub fn save(&self) {
        let path = Settings::path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&path, self.to_text()) {
            eprintln!("can't save the settings to {}: {e}", path.display());
        }
    }

    /// Where the file is: `MAO_SETTINGS`, else `mao.ini` here if it exists, else
    /// in the user's config folder.
    pub fn path() -> PathBuf {
        if let Ok(p) = std::env::var("MAO_SETTINGS") {
            return PathBuf::from(p);
        }
        let here = PathBuf::from("mao.ini");
        if here.exists() {
            return here;
        }
        let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("XDG_CONFIG_HOME")).map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
        base.map(|b| b.join("ModernAnarchyOnline").join("mao.ini")).unwrap_or(here)
    }

    /// The settings from the file (written with the defaults if there is none),
    /// then the command line's `--window-mode`, `--width` and `--height`.
    pub fn load() -> Settings {
        let path = Settings::path();
        let mut settings = match std::fs::read_to_string(&path) {
            Ok(text) => Settings::from_text(&text),
            Err(_) => {
                let defaults = Settings::default();
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if std::fs::write(&path, defaults.to_text()).is_ok() {
                    eprintln!("wrote the default settings to {}", path.display());
                }
                defaults
            }
        };
        let args: Vec<String> = std::env::args().collect();
        let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
        if let Some(mode) = value("--window-mode").and_then(|v| WindowMode::parse(&v)) {
            settings.mode = mode;
        }
        if let Some(w) = value("--width").and_then(|v| v.parse().ok()) {
            settings.width = w;
        }
        if let Some(h) = value("--height").and_then(|v| v.parse().ok()) {
            settings.height = h;
        }
        settings
    }
}

#[cfg(test)]
mod tests {
    /// The frame limit: the general one in front, the background's behind (never above the general one), none at 0.
    #[test]
    fn the_frame_limit_follows_the_windows_place() {
        use super::{frame_limit, next_frame_due};
        assert_eq!(frame_limit(0, 30, true), None);
        assert_eq!(frame_limit(144, 30, true), Some(144));
        assert_eq!(frame_limit(144, 30, false), Some(30));
        assert_eq!(frame_limit(20, 30, false), Some(20));
        assert_eq!(frame_limit(0, 30, false), Some(30));
        assert_eq!(frame_limit(60, 0, false), Some(60));
        let start = std::time::Instant::now();
        let ms = std::time::Duration::from_millis;
        // (on time: a step after the last; far behind: now)
        assert_eq!(next_frame_due(start, start + ms(5), 100), start + ms(10));
        assert_eq!(next_frame_due(start, start + ms(500), 100), start + ms(500));
    }

    use super::*;

    /// Shift+T is the tradeskill window and T alone stays the target's info; a settings file written before the key
    /// was there (it has no line for it) keeps the default.
    #[test]
    fn shift_t_is_the_tradeskill_window() {
        let shift = ModifiersState::SHIFT;
        let keys = Keymap::default();
        assert_eq!(keys.action(KeyCode::KeyT, shift), Some(Action::TradeskillWindow));
        assert_eq!(keys.action(KeyCode::KeyT, ModifiersState::empty()), Some(Action::TargetInfo));
        let old = Settings::from_text(&format!("[keys]
version = {KEYMAP_VERSION}
target_info = T
"));
        assert_eq!(old.keymap.action(KeyCode::KeyT, shift), Some(Action::TradeskillWindow));
    }

    #[test]
    fn the_message_log_is_read_and_written() {
        let s = Settings::from_text("[keys]
version = 2
quit = Ctrl+Q

[debug]
; a note: with = in it
log_messages = on
");
        assert!(s.log_messages);
        assert!(Settings::from_text(&s.to_text()).log_messages);
        // (the camera: the original's free one unless another is chosen, by its name in the game too)
        assert_eq!(Settings::default().camera_mode, "trail");
        let mut c = Settings::default();
        assert_eq!(c.set_option("camera_mode", "rubber"), "camera_mode is now rubber.");
        assert_eq!(c.camera_mode, "rubber");
        c.set_option("camera", "free");
        assert_eq!(c.camera_mode, "trail");
        assert!(c.set_option("camera_mode", "orbit").contains("needs free, rubber or lock"));
        assert_eq!(Settings::from_text(&c.to_text()).camera_mode, "trail");
        // (and by its name in the game: `/setoption log_messages on`)
        let mut t = Settings::default();
        assert_eq!(t.set_option("log_messages", "on"), "log_messages is now on.");
        assert!(t.log_messages && LOG_MESSAGES.load(std::sync::atomic::Ordering::Relaxed));
        t.set_option("log_messages", "off");
        assert!(!t.log_messages && !LOG_MESSAGES.load(std::sync::atomic::Ordering::Relaxed));
        assert!(!Settings::from_text(&Settings::default().to_text()).log_messages);
    }

    #[test]
    fn options_are_set_by_name() {
        let mut s = Settings::default();
        assert_eq!(s.set_option("vsync", "on"), "vsync is now on.");
        assert!(s.vsync);
        assert_eq!(s.set_option("render_scale", "0.8"), "render_scale is now 0.8.");
        assert_eq!(s.render_scale, 0.8);
        assert_eq!(s.set_option("ui_scale", "9"), "ui_scale is now 9.");
        assert_eq!(s.ui_scale, 2.0);
        s.step_ui_scale(1);
        assert_eq!(s.ui_scale, 2.0);
        s.step_ui_scale(-3);
        assert_eq!(s.ui_scale, 1.7);
        assert!(s.set_option("render_scale", "x").contains("needs a value"));
        assert!(s.set_option("colour", "red").contains("No setting"));
        assert!(s.set_option("", "").contains("render_scale 0.8"));
    }

    #[test]
    fn bindings_round_trip() {
        for text in ["W", "Shift+Tab", "Ctrl+Shift+Tab", "Alt+Enter", "Numpad4", "F11", "Slash"] {
            let b = parse_binding(text).unwrap();
            assert_eq!(binding_name(&b), text);
        }
        assert!(parse_binding("Shift+Nonsense").is_none());
        assert_eq!(parse_binding("ctrl+r").unwrap().key, KeyCode::KeyR);
    }

    #[test]
    fn defaults_parse_and_resolve() {
        let keymap = Keymap::default();
        assert_eq!(keymap.action(KeyCode::KeyW, ModifiersState::empty()), Some(Action::Forward));
        assert_eq!(keymap.action(KeyCode::ArrowLeft, ModifiersState::empty()), Some(Action::TurnLeft));
        assert_eq!(keymap.action(KeyCode::Tab, ModifiersState::SHIFT), Some(Action::PrevHostileTarget));
        assert_eq!(keymap.action(KeyCode::Tab, ModifiersState::empty()), Some(Action::NextHostileTarget));
        assert_eq!(keymap.action(KeyCode::Enter, ModifiersState::ALT), Some(Action::CycleWindowMode));
        assert_eq!(keymap.action(KeyCode::Enter, ModifiersState::empty()), Some(Action::ChatInput));
        // (GUI.dll's key table: the chat's second keys)
        assert_eq!(keymap.action(KeyCode::Enter, ModifiersState::SHIFT), Some(Action::ChatInput));
        assert_eq!(keymap.action(KeyCode::NumpadDivide, ModifiersState::empty()), Some(Action::ChatCommand));
        // Every action's defaults are valid, and no two actions share a binding.
        let mut seen: HashMap<Binding, Action> = HashMap::new();
        for &(action, name, keys) in ACTIONS {
            for k in keys.split(',').map(str::trim).filter(|k| !k.is_empty()) {
                let b = parse_binding(k).unwrap_or_else(|| panic!("{name}: bad default key {k}"));
                if let Some(other) = seen.insert(b, action) {
                    panic!("{k} is bound to both {other:?} and {action:?}");
                }
            }
        }
    }

    #[test]
    fn settings_round_trip_and_overrides() {
        let defaults = Settings::default();
        let again = Settings::from_text(&defaults.to_text());
        assert_eq!(again.mode, WindowMode::Windowed);
        assert_eq!((again.width, again.height), (1280, 800));
        for &(action, _, _) in ACTIONS {
            assert_eq!(again.keymap.keys(action), defaults.keymap.keys(action));
        }
        let custom = Settings::from_text("[window]\nmode = Borderless ; fine\nwidth = 1920\n[interface]\ntheme = aou_cyt\naou_mod = C:/mods/aou\ngui_dir = D:/skins/Blue\n[keys]\nversion = 2\nforward = Up\njump = Ctrl+Space\n");
        assert_eq!((custom.theme.as_str(), custom.aou_mod.as_str(), custom.gui_dir.as_str()), ("aou_cyt", "C:/mods/aou", "D:/skins/Blue"));
        assert_eq!(Settings::from_text(&custom.to_text()).theme, "aou_cyt");
        assert_eq!(custom.mode, WindowMode::Borderless);
        assert_eq!(custom.width, 1920);
        assert_eq!(custom.keymap.action(KeyCode::KeyW, ModifiersState::empty()), None);
        assert_eq!(custom.keymap.action(KeyCode::ArrowUp, ModifiersState::empty()), Some(Action::Forward));
        assert_eq!(custom.keymap.action(KeyCode::Space, ModifiersState::CONTROL), Some(Action::Jump));
    }

    #[test]
    fn assigning_a_key_takes_it_from_the_other_action() {
        let mut keymap = Keymap::default();
        let space = parse_binding("Space").unwrap();
        keymap.assign(Action::NoClip, space);
        assert_eq!(keymap.action(KeyCode::Space, ModifiersState::empty()), Some(Action::NoClip));
        assert_eq!(keymap.keys(Action::Jump), "");
        assert_eq!(keymap.keys(Action::NoClip), "Space");
    }

    /// The keys a character rebound in the original stand in place of this client's for the session: asked first,
    /// shown as the action's keys, and not written to the settings. An action whose key was changed here keeps it;
    /// a key chosen or cleared here afterwards is this client's again.
    #[test]
    fn a_characters_own_keys_lie_over_the_clients() {
        let key = |text: &str| parse_binding(text).unwrap();
        let none = ModifiersState::empty();
        let mut settings = Settings::default();
        settings.keymap.set(Action::Use, "G");
        let taken = settings.keymap.set_character([(Action::Attack, vec![key("F")]), (Action::Use, vec![key("Y")]), (Action::Jump, Vec::new()), (Action::SitStand, vec![key("Q")])].into_iter().collect());
        assert_eq!(taken, [Action::Jump, Action::Attack, Action::SitStand]);
        let k = &settings.keymap;
        assert_eq!((k.keys(Action::Attack), k.keys(Action::Use), k.keys(Action::Jump), k.keys(Action::SitStand)), ("F".to_string(), "G".to_string(), String::new(), "Q".to_string()));
        assert_eq!((k.action(KeyCode::KeyF, none), k.action(KeyCode::Space, none), k.action(KeyCode::KeyG, none)), (Some(Action::Attack), None, Some(Action::Use)));
        // (Q is the character's sit key now, though the client's own attack still has it underneath)
        assert_eq!(k.action(KeyCode::KeyQ, none), Some(Action::SitStand));
        // (a release ends every action the key has under any modifiers: the character's sit, and Ctrl+Q, but no longer the attack)
        assert_eq!(k.actions_for_key(KeyCode::KeyQ), [Action::SitStand, Action::Quit]);
        assert_eq!(k.bindings_of(Action::Attack), [key("F")]);
        // (the settings file keeps the client's own)
        let text = settings.to_text();
        assert!(text.contains("\nattack = Q\n") && text.contains("\njump = Space\n") && text.contains("\nuse = G\n") && text.contains("\nsit_stand = X\n"));
        // (a key chosen here takes the action back, and takes the key from the character's others)
        settings.keymap.assign(Action::Attack, key("Q"));
        let k = &settings.keymap;
        assert_eq!((k.keys(Action::Attack), k.keys(Action::SitStand), k.action(KeyCode::KeyQ, none)), ("Q".to_string(), String::new(), Some(Action::Attack)));
        settings.keymap.set(Action::Jump, "Space");
        assert_eq!(settings.keymap.action(KeyCode::Space, none), Some(Action::Jump));
        // (read back, the file has no trace of the character's keys)
        assert_eq!(Settings::from_text(&text).keymap.keys(Action::Attack), "Q");
    }

    /// The keys that are the shipped file's since version 3 of the key table, and a version 2 file brought up to
    /// date: an action still on its old default gets the new one; a key the player chose stays; a new key another
    /// action of the file has is not taken.
    #[test]
    fn the_older_key_table_is_brought_up_to_date() {
        let k = Keymap::default();
        assert_eq!((k.keys(Action::ToggleWalk), k.keys(Action::MapOverlay)), ("Backspace".to_string(), "Ctrl+P".to_string()));
        assert_eq!(k.action(KeyCode::KeyP, ModifiersState::CONTROL), Some(Action::MapOverlay));
        assert_eq!(k.action(KeyCode::KeyP, ModifiersState::empty()), Some(Action::PlanetMapWindow));
        assert_eq!((k.keys(Action::Quick(8)), k.keys(Action::Quick(9)), k.keys(Action::Quick(10)), k.keys(Action::Quick(11))), ("J".to_string(), "N".to_string(), "O".to_string(), "Period".to_string()));
        for &(action, _, new) in MOVED_DEFAULTS {
            assert_eq!(k.keys(action), new);
        }
        // (a version 2 file as this client wrote it, untouched: every one of the six moves)
        let mut old = Settings::default();
        for &(action, keys, _) in MOVED_DEFAULTS {
            old.keymap.set(action, keys);
        }
        let untouched = old.to_text().replace(&format!("version = {KEYMAP_VERSION}"), "version = 2");
        assert!(untouched.contains("\nmap_overlay = O\n") && untouched.contains("\ntoggle_walk = NumpadDivide\n") && untouched.contains("\naction_aimed_shot = \n"));
        let k = Settings::from_text(&untouched).keymap;
        for &(action, _, new) in MOVED_DEFAULTS {
            assert_eq!(k.keys(action), new);
        }
        assert_eq!(k.action(KeyCode::KeyO, ModifiersState::empty()), Some(Action::Quick(10)));
        // (the player's own choices: the overlay kept on O, so the aimed shot stays without a key; walk/run moved
        // to another key stays there; J given to the jump, so the sneak attack does not take it)
        let chosen = untouched.replace("\ntoggle_walk = NumpadDivide\n", "\ntoggle_walk = F7\n").replace("\njump = Space\n", "\njump = Space, J\n").replace("\nmap_overlay = O\n", "\nmap_overlay = O, F6\n");
        let k = Settings::from_text(&chosen).keymap;
        assert_eq!((k.keys(Action::ToggleWalk), k.keys(Action::MapOverlay), k.keys(Action::Quick(10)), k.keys(Action::Quick(8)), k.keys(Action::Quick(9))), ("F7".to_string(), "O, F6".to_string(), String::new(), String::new(), "N".to_string()));
        // (a version 2 file without the lines is as version 2 was, then brought up to date the same)
        let bare = Settings::from_text("[keys]\nversion = 2\nattack = F\n").keymap;
        assert_eq!((bare.keys(Action::Attack), bare.keys(Action::ToggleWalk), bare.keys(Action::Quick(10))), ("F".to_string(), "Backspace".to_string(), "O".to_string()));
        // (a file of the version before, or after, has its keys replaced by the defaults, as before)
        assert_eq!(Settings::from_text("[keys]\nversion = 1\nattack = F\n").keymap.keys(Action::Attack), "Q");
        // (and one written now is read as it is: an aimed shot unbound here stays unbound)
        let mut now = Settings::default();
        now.keymap.set(Action::Quick(10), "");
        assert_eq!(Settings::from_text(&now.to_text()).keymap.keys(Action::Quick(10)), "");
    }

    #[test]
    fn window_modes_cycle() {
        assert_eq!(WindowMode::Windowed.next().next().next(), WindowMode::Windowed);
        assert_eq!(WindowMode::parse("Borderless"), Some(WindowMode::Borderless));
        assert_eq!(WindowMode::parse("exclusive"), Some(WindowMode::Fullscreen));
        assert_eq!(WindowMode::parse("tiny"), None);
    }

    #[test]
    fn the_defaults_are_the_originals_and_no_key_is_bound_twice() {
        let k = Keymap::default();
        assert_eq!(k.keys(Action::Forward), "W, Up");
        assert_eq!(k.keys(Action::StrafeLeft), "Z");
        assert_eq!(k.keys(Action::StrafeRight), "C");
        assert_eq!(k.keys(Action::Attack), "Q");
        assert_eq!(k.keys(Action::SkillsWindow), "U");
        assert_eq!(k.keys(Action::InventoryWindow), "I");
        assert_eq!(k.keys(Action::WearWindow), "Ctrl+1");
        assert_eq!(k.keys(Action::ToggleOptions), "F10");
        assert_eq!(k.keys(Action::SelectSelf), "F1");
        assert_eq!(k.keys(Action::AutoRun), "Numpad0");
        assert_eq!(k.keys(Action::CameraRotateUp), "Numpad2");
        assert_eq!(k.keys(Action::ChatReply), "Shift+R");
        assert_eq!(k.keys(Action::HotbarBar(2)), "Alt+3");
        assert_eq!(k.keys(Action::Quick(0)), "B");
        // no two actions share a key and its modifiers
        let mut seen = std::collections::HashMap::new();
        for &(action, name, _) in ACTIONS {
            for b in k.bindings_of(action) {
                if let Some(other) = seen.insert(b, name) {
                    panic!("{other} and {name} are both bound to {}", binding_name(&b));
                }
            }
        }
    }

    #[test]
    fn the_keys_of_an_older_file_give_way_to_the_defaults() {
        let old = "[keys]
forward = Y
attack = F
";
        assert_eq!(Settings::from_text(old).keymap.keys(Action::Forward), "W, Up");
        let current = format!("[keys]
version = {KEYMAP_VERSION}
forward = Y
");
        assert_eq!(Settings::from_text(&current).keymap.keys(Action::Forward), "Y");
        // what is written reads back
        let text = Settings::default().to_text();
        assert_eq!(Settings::from_text(&text).keymap.keys(Action::Attack), "Q");
    }
}
