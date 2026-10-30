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
//! on) plus the movement of this client. The default keys of the original's
//! own commands are not recorded in its data files, so the defaults below are
//! the ones its players know (marked "assumed" where they could not be checked
//! against the original).

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
    /// Show hotbar n (0..6).
    HotbarBar(u8),
    ChatInput,
    ChatCommand,
    ChatReply,
    ChatPageUp,
    ChatPageDown,
    PerkWindow,
    VehicleWindow,
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
    // The windows: I the inventory (Ctrl+1 the worn gear, which is the same window here), U the skills, Ctrl+2 to Ctrl+0 the others.
    (Action::InventoryWindow, "inventory", "I, Ctrl+1"),
    (Action::SkillsWindow, "skills_window", "U"),
    (Action::ActionsWindow, "actions_window", "Ctrl+2"),
    (Action::MissionWindow, "mission_window", "Ctrl+4"),
    (Action::PlanetMapWindow, "planet_map", "P, Ctrl+6"),
    (Action::MapOverlay, "map_overlay", "O"),
    (Action::FriendsWindow, "friends_window", "Ctrl+7"),
    (Action::ProgramsWindow, "programs_window", "Ctrl+8"),
    (Action::StatsWindow, "stats_window", "Ctrl+9"),
    (Action::NcuWindow, "ncu_window", "Ctrl+0"),
    (Action::PerkWindow, "open_perk_window", "Shift+P"),
    (Action::VehicleWindow, "open_vehicle_window", "Shift+V"),
    // (This client's own windows have no key in the original: they are reached by using the booth or the terminal.)
    (Action::WhompahWindow, "whompah_window", ""),
    (Action::MailWindow, "mail_window", ""),
    (Action::HelpWindow, "help_window", ""),
    (Action::UiLarger, "ui_larger", "Ctrl+Equal"),
    (Action::UiSmaller, "ui_smaller", "Ctrl+Minus"),
    (Action::Screenshot, "screenshot", "F12"),
    (Action::Coordinates, "coordinates", "F9"),
    (Action::AutoRun, "auto_run", "Numpad0"),
    (Action::ToggleWalk, "toggle_walk", "NumpadDivide"),
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
    // The special and character actions.
    (Action::Quick(0), "action_brawl", "B"),
    (Action::Quick(1), "action_burst", "M"),
    (Action::Quick(2), "action_dimach", "K"),
    (Action::Quick(3), "action_fling_shot", "L"),
    (Action::Quick(4), "action_full_auto", "Comma"),
    (Action::Quick(5), "action_reload", "V"),
    (Action::Quick(6), "action_pick_up", "R"),
    (Action::Quick(7), "action_sneak", "H"),
    // The chat.
    (Action::ChatInput, "chat_input", "Enter"),
    (Action::ChatCommand, "chat_command", "Slash"),
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
/// first versions used keys of this client's own).
pub const KEYMAP_VERSION: u32 = 2;

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
}

impl Default for Keymap {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        for &(action, _, keys) in ACTIONS {
            bindings.insert(action, keys.split(',').filter_map(parse_binding).collect());
        }
        Keymap { bindings }
    }
}

impl Keymap {
    /// The action a key press means, if any (bindings with more modifiers win).
    pub fn action(&self, key: KeyCode, modifiers: ModifiersState) -> Option<Action> {
        ACTIONS.iter().map(|&(a, _, _)| a).find(|a| self.bindings.get(a).is_some_and(|list| list.iter().any(|b| b.matches(key, modifiers))))
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
        ACTIONS.iter().map(|&(a, _, _)| a).filter(|a| self.bindings.get(a).is_some_and(|list| list.iter().any(|b| b.key == key))).collect()
    }

    /// Makes `binding` the only key of `action`, taking it from any other action.
    pub fn assign(&mut self, action: Action, binding: Binding) {
        for list in self.bindings.values_mut() {
            list.retain(|b| *b != binding);
        }
        self.bindings.insert(action, vec![binding]);
    }

    pub fn set(&mut self, action: Action, keys: &str) {
        self.bindings.insert(action, keys.split(',').filter_map(parse_binding).collect());
    }

    /// The bindings of an action (none when it is unbound).
    pub fn bindings_of(&self, action: Action) -> Vec<Binding> {
        self.bindings.get(&action).cloned().unwrap_or_default()
    }

    pub fn keys(&self, action: Action) -> String {
        self.bindings.get(&action).map(|list| list.iter().map(binding_name).collect::<Vec<_>>().join(", ")).unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub mode: WindowMode,
    pub width: u32,
    pub height: u32,
    /// Wait for the monitor's refresh (capped at its rate); off = uncapped.
    pub vsync: bool,
    /// The share of the window the scene is drawn at before FSR 1 upscales it (1 = native).
    pub render_scale: f32,
    /// FSR's sharpening in stops (0 = most).
    pub sharpness: f32,
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
    /// The chat lines in the original's own bitmap font.
    pub original_font: bool,
    /// How the third-person camera follows a turning character: `lock` (stays behind), `rubber` (swings round) or `trail` (slowly).
    pub camera_mode: String,
    /// The planet maps (the original's `PlanetMapIndexFile` and `ShadowlandMapIndexFile`): an index file's path from a
    /// maps' folder (`crate::planetmaps`).
    pub planet_map: String,
    pub shadowlands_map: String,
    pub keymap: Keymap,
}

/// The planet maps used unless another is chosen: Saavick's Map of Rubi-Ka and SLmap.
pub const DEFAULT_PLANET_MAP: &str = "SaavicksMap/saavicksmap.txt";
pub const DEFAULT_SHADOWLANDS_MAP: &str = "SLmap/SLmap.txt";

impl Default for Settings {
    fn default() -> Self {
        Settings { mode: WindowMode::Windowed, width: 1280, height: 800, vsync: false, render_scale: 1.0, sharpness: 0.2, ui_scale: 1.0, view_distance: 1800.0, char_distance: 0.0, auto_loot: false, theme: "original".into(), aou_mod: String::new(), gui_dir: String::new(), debug_overlay: false, original_font: true, camera_mode: "rubber".into(), planet_map: DEFAULT_PLANET_MAP.into(), shadowlands_map: DEFAULT_SHADOWLANDS_MAP.into(), keymap: Keymap::default() }
    }
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
                "vsync {}, render_scale {}, sharpness {}, ui_scale {}, view_distance {}, char_distance {}, auto_loot {}",
                if self.vsync { "on" } else { "off" },
                self.render_scale,
                self.sharpness,
                self.ui_scale,
                self.view_distance,
                self.char_distance,
                if self.auto_loot { "on" } else { "off" }
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
            "render_scale" => number(value, 0.4, 1.0).map(|n| self.render_scale = n).is_some(),
            "sharpness" => number(value, 0.0, 2.0).map(|n| self.sharpness = n).is_some(),
            "ui_scale" => number(value, 0.6, 2.0).map(|n| self.ui_scale = n).is_some(),
            "view_distance" | "viewdist" => number(value, 200.0, 4000.0).map(|n| self.view_distance = n).is_some(),
            "char_distance" | "chardist" => number(value, 0.0, 4000.0).map(|n| self.char_distance = n).is_some(),
            _ => return format!("No setting called {name:?} (vsync, render_scale, sharpness, ui_scale, view_distance, char_distance, auto_loot)."),
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
        out += &format!("mode = {}\nwidth = {}\nheight = {}\nvsync = {}\nrender_scale = {}\nsharpness = {}\nui_scale = {}\nview_distance = {}\nchar_distance = {}\nauto_loot = {}\n\n", self.mode.name(), self.width, self.height, if self.vsync { "on" } else { "off" }, self.render_scale, self.sharpness, self.ui_scale, self.view_distance, self.char_distance, if self.auto_loot { "on" } else { "off" });
        out += &format!("[interface]\n; built-in | original | aou_translucent | aou_cyt | aou_slim_cni | aou_slim_cnt | a folder with GUIColors.xml\ntheme = {}\n; the AOU mod's folder (the one with Gui in it), for its themes\naou_mod = {}\n; a skin folder (Graphics.uvga and Graphics.uvgi, as the original's Gui/<skin> folders), used before the theme's and the game's own pictures\ngui_dir = {}\n; show the location and coordinates, the frame rate and key hints over the game (for developers)\ndebug_overlay = {}\n; the chat lines in the original's own bitmap font\noriginal_font = {}\n; the third-person camera: lock (stays behind the character), rubber (swings round behind it), trail (slowly)\ncamera_mode = {}\n; the planet maps: an index file under a maps' folder (the client's PlanetMap folder, then the game's textures/PlanetMap)\nplanet_map = {}\nshadowlands_map = {}\n\n", self.theme, self.aou_mod, self.gui_dir, if self.debug_overlay { "on" } else { "off" }, if self.original_font { "on" } else { "off" }, self.camera_mode, self.planet_map, self.shadowlands_map);
        out += "[keys]\n; one or more keys per action, separated by commas; Shift+, Ctrl+ and Alt+ combine\n";
        out += &format!("version = {KEYMAP_VERSION}\n");
        for &(action, name, _) in ACTIONS {
            out += &format!("{name} = {}\n", self.keymap.keys(action));
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
                ("window", "width") => settings.width = value.parse().unwrap_or(settings.width).clamp(320, 16384),
                ("window", "render_scale") => settings.render_scale = value.parse().unwrap_or(settings.render_scale).clamp(0.4, 1.0),
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
                ("interface", "debug_overlay") => settings.debug_overlay = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("keys", "version") => key_version = value.parse().unwrap_or(0),
                ("keys", name) => key_lines.push((n, name.to_string(), value.to_string())),
                _ => eprintln!("settings line {}: unknown setting {key:?}", n + 1),
            }
        }
        if key_version == KEYMAP_VERSION {
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
    use super::*;

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
        assert_eq!(k.keys(Action::InventoryWindow), "I, Ctrl+1");
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
