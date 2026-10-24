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
    /// The mission booth's window (asks the nearest booth).
    MissionWindow,
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
    /// The help window: keys, mouse and commands.
    HelpWindow,
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
}

/// Every action with its name in the settings file and its default keys.
/// (The targets, chat and window actions are reserved: there is no interface
/// for them yet, but the keys are already part of the settings.)
pub const ACTIONS: &[(Action, &str, &str)] = &[
    (Action::Forward, "forward", "W, Up"),
    (Action::Back, "back", "S, Down"),
    (Action::TurnLeft, "turn_left", "A, Left"),
    (Action::TurnRight, "turn_right", "D, Right"),
    (Action::Jump, "jump", "Space"),
    (Action::NoClip, "noclip", "Ctrl+Shift+C"),
    (Action::StrafeLeft, "strafe_left", "Z"),
    (Action::StrafeRight, "strafe_right", "C"),
    // The original's fixed camera commands (default keys assumed).
    (Action::CameraRotateLeft, "camera_rotate_left", "Numpad4"),
    (Action::CameraRotateRight, "camera_rotate_right", "Numpad6"),
    (Action::CameraRotateUp, "camera_rotate_up", "Numpad8"),
    (Action::CameraRotateDown, "camera_rotate_down", "Numpad2"),
    (Action::CameraZoomIn, "camera_zoom_in", "PageUp, NumpadAdd"),
    (Action::CameraZoomOut, "camera_zoom_out", "PageDown, NumpadSubtract"),
    (Action::CameraReset, "camera_reset", "Home"),
    (Action::CameraSavePreferred, "camera_save_preferred", "End"),
    // The original's fixed targeting commands (default keys assumed).
    (Action::NextHostileTarget, "next_hostile_target", "Tab"),
    (Action::PrevHostileTarget, "prev_hostile_target", "Shift+Tab"),
    (Action::NextFriendlyTarget, "next_friendly_target", "Ctrl+Tab"),
    (Action::PrevFriendlyTarget, "prev_friendly_target", "Ctrl+Shift+Tab"),
    (Action::Attack, "attack", "F"),
    (Action::InventoryWindow, "inventory", "I"),
    (Action::SitStand, "sit_stand", "X"),
    (Action::Use, "use", "E"),
    (Action::WhompahWindow, "whompah_window", "B"),
    (Action::MissionWindow, "mission_window", "M"),
    (Action::MailWindow, "mail_window", "Ctrl+M"),
    (Action::ActionsWindow, "actions_window", "K"),
    (Action::MapOverlay, "map_overlay", "O"),
    (Action::StatsWindow, "stats_window", "U"),
    (Action::SkillsWindow, "skills_window", "J"),
    (Action::NcuWindow, "ncu_window", "N"),
    (Action::FriendsWindow, "friends_window", "L"),
    (Action::ProgramsWindow, "programs_window", "Ctrl+N"),
    (Action::Screenshot, "screenshot", "F12"),
    (Action::HelpWindow, "help_window", "Ctrl+H"),
    (Action::AutoRun, "auto_run", "NumpadMultiply"),
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
    (Action::HotbarBar(0), "hotbar_bar_1", "F1"),
    (Action::HotbarBar(1), "hotbar_bar_2", "F2"),
    (Action::HotbarBar(2), "hotbar_bar_3", "F3"),
    (Action::HotbarBar(3), "hotbar_bar_4", "F4"),
    (Action::HotbarBar(4), "hotbar_bar_5", "F5"),
    (Action::HotbarBar(5), "hotbar_bar_6", "F6"),
    // The original's fixed chat commands (default keys assumed).
    (Action::ChatInput, "chat_input", "Enter"),
    (Action::ChatCommand, "chat_command", "Slash"),
    (Action::ChatReply, "chat_reply", "Ctrl+R"),
    (Action::ChatPageUp, "chat_history_page_up", "Shift+PageUp"),
    (Action::ChatPageDown, "chat_history_page_down", "Shift+PageDown"),
    (Action::PerkWindow, "open_perk_window", "P"),
    (Action::VehicleWindow, "open_vehicle_window", ""),
    // This client.
    (Action::CycleWindowMode, "cycle_window_mode", "Alt+Enter"),
    (Action::ToggleFullscreen, "toggle_fullscreen", "F11"),
    (Action::ToggleOptions, "options", "Escape, F10"),
    (Action::Quit, "quit", "Ctrl+Q"),
];

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
    /// The interface colours: `built-in`, `original`, `aou_translucent` ... or a folder
    /// (see [`crate::theme`]).
    pub theme: String,
    /// The AOU mod's folder (the one with `Gui` in it), for its themes.
    pub aou_mod: String,
    pub keymap: Keymap,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { mode: WindowMode::Windowed, width: 1280, height: 800, vsync: false, render_scale: 1.0, sharpness: 0.2, theme: "built-in".into(), aou_mod: String::new(), keymap: Keymap::default() }
    }
}

impl Settings {
    /// The settings text: the window options, then every action's keys.
    pub fn to_text(&self) -> String {
        let mut out = String::from("; Modern Anarchy Online settings\n[window]\n; windowed | borderless | fullscreen (F11 / Alt+Enter change it while playing)\n");
        out += &format!("mode = {}\nwidth = {}\nheight = {}\nvsync = {}\nrender_scale = {}\nsharpness = {}\n\n", self.mode.name(), self.width, self.height, if self.vsync { "on" } else { "off" }, self.render_scale, self.sharpness);
        out += &format!("[interface]\n; built-in | original | aou_translucent | aou_cyt | aou_slim_cni | aou_slim_cnt | a folder with GUIColors.xml\ntheme = {}\n; the AOU mod's folder (the one with Gui in it), for its themes\naou_mod = {}\n\n", self.theme, self.aou_mod);
        out += "[keys]\n; one or more keys per action, separated by commas; Shift+, Ctrl+ and Alt+ combine\n";
        for &(action, name, _) in ACTIONS {
            out += &format!("{name} = {}\n", self.keymap.keys(action));
        }
        out
    }

    /// Reads settings text over the defaults; unknown lines are reported and skipped.
    pub fn from_text(text: &str) -> Settings {
        let mut settings = Settings::default();
        let mut section = String::new();
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
                ("window", "vsync") => settings.vsync = matches!(value.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
                ("window", "height") => settings.height = value.parse().unwrap_or(settings.height).clamp(240, 16384),
                ("interface", "theme") => settings.theme = value.to_string(),
                ("interface", "aou_mod") => settings.aou_mod = value.to_string(),
                ("keys", name) => match ACTIONS.iter().find(|(_, n, _)| *n == name) {
                    Some(&(action, _, _)) => {
                        let bad: Vec<&str> = value.split(',').map(str::trim).filter(|k| !k.is_empty() && parse_binding(k).is_none()).collect();
                        if !bad.is_empty() {
                            eprintln!("settings line {}: unknown keys {bad:?}", n + 1);
                        }
                        settings.keymap.set(action, value);
                    }
                    None => eprintln!("settings line {}: unknown action {name:?}", n + 1),
                },
                _ => eprintln!("settings line {}: unknown setting {key:?}", n + 1),
            }
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
        let custom = Settings::from_text("[window]\nmode = Borderless ; fine\nwidth = 1920\n[interface]\ntheme = aou_cyt\naou_mod = C:/mods/aou\n[keys]\nforward = Up\njump = Ctrl+Space\n");
        assert_eq!((custom.theme.as_str(), custom.aou_mod.as_str()), ("aou_cyt", "C:/mods/aou"));
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
}
