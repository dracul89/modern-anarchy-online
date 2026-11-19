//! The option window's "Key bindings" tab: GUI.dll's `BindView_c` (FUN_100bcfb3), the original's editor of the
//! rebindable keys, here over this client's own key store (`settings::Keymap`, kept in `mao.ini`).
//!
//! * **The view** (FUN_100bcfb3): a `MultiListView_c` in list mode with three columns, "Category" 100 wide, "Function"
//!   200 and "Key" 100 (flags 0xe: resizable, movable, sortable; the first, so the rows sort by it), in a
//!   `BorderView_c` 5 in from the sides, over a row of four buttons pushed to the right by a spacer: "Reset All",
//!   "Add", "Change" and "Clear", each 5 in. A click on a row selects it (FUN_100bc5de).
//! * **The rows** (FUN_100bc713): for every action provider of `InputConfig_t` (registered by GUI.dll's modules with
//!   FUN_10018a00: [`PROVIDERS`]), a row for each key bound to it, or one row with none. A row (`BindListItem_c`,
//!   FUN_100bdb23) shows the provider's category (a `#name` is the text of that name in text.mdb's category 2012) and
//!   function (category 2011), and the key as FUN_100bc5f5 writes it: `CTRL+`, `ALT+`, `SHIFT+` (FUN_10018b05, none
//!   for a modifier key itself) and the key's name in `InputConfig_t`'s table ([`INPUT_NAMES`]; no key is its entry
//!   0, "Inactive"), in aqua. The rows sort (FUN_100bde5f) by the column chosen and then the columns right of it,
//!   comparing the texts as they are, markup and all.
//! * **Add / Change** (FUN_100bcca5 with mode 0 / 1), with a row selected and no box up: the "Bind Key" box
//!   (`DialogBox_c` with OK and Cancel, text.mdb 10000's `BindDialogText`, two breaks and the row's key), the input
//!   taken from the game (`InputConfig_t` +0x24). Each key pressed (FUN_100bcc67: the key with 0x80000 for Alt,
//!   0x40000 for Ctrl, 0x20000 for Shift) becomes the box's key and is written in its text (FUN_100bcaaf). OK
//!   (FUN_100bc8c1): with Add on a provider that has keys, the key is bound to it as well (FUN_10018d71, which refuses
//!   a key it has) and a new row shows it; otherwise (Change, or Add on a provider without keys) the row's key is
//!   unbound (FUN_10018db2) and the new one bound, and the row shows it if that bind took. A key stays on whatever
//!   else it is bound to.
//! * **Clear** (0x100bc68e): the selected row's key unbound; the row goes when the provider has other keys, else it
//!   shows none.
//! * **Reset All** (FUN_100bcf9c): the `KeyBindings` value back to its default (FUN_10019196) and the rows again.
//!
//! Here the providers are those this client has an action for (`keybindings::PROVIDERS`), one row set per action:
//! the original's two sets of movement providers are the same actions here, so only the plain ones are listed (not
//! the `*_GLOBAL_V2` ones); providers without an action here (the pets', the team members', the per-bar shortcut keys, the
//! team, raid, research and other windows' keys, the next/previous camera views, copying to the clipboard) are not
//! listed. Mouse buttons (the box binds a button pressed on it, FUN_100bcc26) are not taken: this client's keys are
//! keyboard keys. Add with no key pressed binds the original's "no key" to a provider that has keys, which this
//! client's store cannot hold: nothing is done then. A key bound to two actions does the first of them here (the
//! original does both).

use crate::settings::{Action, Binding, Keymap};

use super::geom::{Point, Rect};
use super::multilist::{self, CellView, Item, MOVABLE, RESIZABLE, SORTABLE};
use super::view::{Id, Layout, Node, Plain, Spacer, Tree};

/// `InputConfig_t::m_asInputMethodIDTable` (GUI.dll 0x10262bf8, read by `GetInputModeName` 0x1001a0b6): the original's
/// names of its input numbers.
pub const INPUT_NAMES: [&str; 128] = [
    "Inactive", "LeftPress", "LeftDoubleClick", "LeftPressHold", "LeftRelease", "RightPress", "RightDoubleClick",
    "RightPressHold", "RightRelease", "MiddlePress", "MiddleDoubleClick", "MiddlePressHold", "MiddleRelease",
    "MouseWheelMovement", "ESC", "TAB", "CAPS_LOCK", "SHIFT", "CTRL", "ALT", "SHIFT_RIGHT", "CTRL_RIGHT",
    "ALT_RIGHT", "SPACE", "ENTER", "BACKSPACE", "INSERT", "DELETE", "HOME", "END", "PGUP", "PGDN", "CursorLeft",
    "CursorRight", "CursorUp", "CursorDown", "PIPE", "PERIOD", "COMMA", "F1", "F2", "F3", "F4", "F5", "F6", "F7",
    "F8", "F9", "F10", "F11", "F12", "F13", "F14", "F15", "F16", "F17", "F18", "F19", "F20", "F21", "F22", "F23",
    "F24", "SCROLL", "PAUSE", "NUMPAD_NUMLOCK", "NUMPAD_DIV", "NUMPAD_MUL", "NUMPAD_SUB", "NUMPAD_ADD", "NUMPAD_0",
    "NUMPAD_1", "NUMPAD_2", "NUMPAD_3", "NUMPAD_4", "NUMPAD_5", "NUMPAD_6", "NUMPAD_7", "NUMPAD_8", "NUMPAD_9",
    "NUMPAD_DECIMAL", "AnyTextKey", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
    "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "SLASH",
    "MouseMove", "MouseMoveDelta", "XButton1Press", "XButton2Press", "XButton1Release", "XButton2Release",
    "XButton1DoubleClick", "XButton2DoubleClick", "",
];

/// The original's action providers as GUI.dll registers them (FUN_10018a00, in the order of the calls): the name
/// (whose hash is a bind's `Provider`), the category and the function, each `#name` a text of text.mdb (2012, 2011).
/// The shortcut bar's are made in loops ([`BAR_PROVIDERS`]).
pub const PROVIDERS: &[(&str, &str, &str)] = &[
    // (FUN_10022a34's module: the text views)
    ("TEXT_COPYTOCLIPBOARD", "#Text", "#CopyToClip"),
    // (TargetingModule_t, 0x10027061..)
    ("TARGET_SELECTSELF", "#Target", "#TargetSelectSelf"),
    ("TARGET_SELECTPET0", "#Target", "#TargetSelectPet0"),
    ("TARGET_SELECTPET1", "#Target", "#TargetSelectPet1"),
    ("TARGET_SELECTPET2", "#Target", "#TargetSelectPet2"),
    ("TARGET_SELECTPET3", "#Target", "#TargetSelectPet3"),
    ("TARGET_SELECTPET4", "#Target", "#TargetSelectPet4"),
    ("TARGET_SELECTPET5", "#Target", "#TargetSelectPet5"),
    // (FlowControlModule_t 0x1002b70a)
    ("MOVEMENT_FORWARD_GLOBAL_V2", "#Movement", "#MoveForwardGlobal"),
    ("MOVEMENT_BACK_GLOBAL_V2", "#Movement", "#MoveBackGlobal"),
    ("MOVEMENT_LEFT_GLOBAL_V2", "#Movement", "#MoveLeftGlobal"),
    ("MOVEMENT_RIGHT_GLOBAL_V2", "#Movement", "#MoveRightGlobal"),
    ("MOVEMENT_FORWARD_V3", "#Movement", "#MoveForward"),
    ("MOVEMENT_BACK_V2", "#Movement", "#MoveBack"),
    ("MOVEMENT_LEFT_V2", "#Movement", "#MoveLeft"),
    ("MOVEMENT_RIGHT_V2", "#Movement", "#MoveRight"),
    ("MOVEMENT_AUTORUN", "#Movement", "#MoveAutoRun"),
    ("MOVEMENT_STRAFELEFT", "#Movement", "#MoveStrafeLeft"),
    ("MOVEMENT_STRAFERIGHT", "#Movement", "#MoveStrafeRight"),
    ("MOVEMENT_JUMP", "#Movement", "#MoveJump"),
    ("MOVEMENT_TOGGLEWALK", "#Movement", "#MoveToggleWalk"),
    ("CAMERA_TOGGLE3RD", "#Camera", "#CameraToggle3rdPerson"),
    ("CAMERA_NEXTVIEW", "#Camera", "#CameraNextView"),
    ("CAMERA_PREVVIEW", "#Camera", "#CameraPrevView"),
    ("CAMERA_SCREENSHOT", "#Camera", "#CameraScreenshot"),
    ("ACTION_PICKUPITEM", "#Action", "#ActionPickupItem"),
    // (the control center's module, 0x100695cc..)
    ("WINDOW_INVENTORY", "#Window", "#WindowInventory"),
    ("WINDOW_SKILLS", "#Window", "#WindowSkills"),
    ("WINDOW_PLANETMAP", "#Window", "#WindowPlanetMap"),
    ("WINDOW_SHORTCUTBAR", "#Window", "#WindowShortcutBar"),
    ("WINDOW_LFT", "#Window", "#WindowLFT"),
    ("WINDOW_RESEARCH", "#Window", "#WindowResearch"),
    ("WINDOW_RAID", "#Window", "#WindowRaid"),
    ("WINDOW_OPTIONS", "#Window", "#WindowOptions"),
    ("WINDOW_WEAR", "#Window", "#WindowWear"),
    ("WINDOW_SPECIALACTION", "#Window", "#WindowSpecialAction"),
    ("WINDOW_KNOWLEDGE", "#Window", "#WindowKnowledge"),
    ("WINDOW_MISSION", "#Window", "#WindowMission"),
    ("WINDOW_TEAM", "#Window", "#WindowTeam"),
    ("WINDOW_MAP", "#Window", "#WindowMap"),
    ("WINDOW_FRIENDS", "#Window", "#WindowFriends"),
    ("WINDOW_NANO", "#Window", "#WindowNano"),
    ("WINDOW_NCU", "#Window", "#WindowNCU"),
    ("WINDOW_STAT", "#Window", "#WindowStat"),
    ("WINDOW_TS", "#Window", "#WindowTS"),
    // (its function is a plain text, not a text.mdb name)
    ("WINDOW_DL", "#Window", "Daily Login"),
    ("ACTION_ATTACK", "#Action", "#ActionAttack"),
    ("ACTION_USE", "#Action", "#ActionUse"),
    ("ACTION_SNEAK", "#Action", "#ActionSneak"),
    ("ACTION_SIT", "#Action", "#ActionSit"),
    ("ACTION_SNEAKATTACK", "#Action", "#ActionSneakAttack"),
    ("ACTION_FULLAUTO", "#Action", "#ActionFullAuto"),
    ("ACTION_BURST", "#Action", "#ActionBurst"),
    ("ACTION_BRAWL", "#Action", "#ActionBrawl"),
    ("ACTION_RELOAD", "#Action", "#ActionReload"),
    ("ACTION_FASTATTACK", "#Action", "#ActionFastAttack"),
    ("ACTION_DIMACH", "#Action", "#ActionDimach"),
    ("ACTION_FLINGSHOT", "#Action", "#ActionFlingShot"),
    ("ACTION_AIMEDSHOT", "#Action", "#ActionAimedShot"),
    ("ACTION_BOWSPECIALATTACK", "#Action", "#ActionBowSpecialAttack"),
    ("ACTION_CREATEREF", "#Action", "#ActionCreateRef"),
    ("ACTION_LOOKAT", "#Action", "#ActionLookAt"),
    ("ACTION_SWITCHTARGET", "#Action", "#ActionSwitchTarget"),
    // (the team module, 0x1007c180..)
    ("TARGET_SELECTTEAMMEMBER1", "#Target", "#TargetSelectTeamMember1"),
    ("TARGET_SELECTTEAMMEMBER2", "#Target", "#TargetSelectTeamMember2"),
    ("TARGET_SELECTTEAMMEMBER3", "#Target", "#TargetSelectTeamMember3"),
    ("TARGET_SELECTTEAMMEMBER4", "#Target", "#TargetSelectTeamMember4"),
    ("TARGET_SELECTTEAMMEMBER5", "#Target", "#TargetSelectTeamMember5"),
    // (the browser window's module, 0x1010c2c8..)
    ("WINDOW_BROWSER", "#Window", "#WindowBrowser"),
    ("WINDOW_ITEMSHOP", "#Window", "#WindowItemShop"),
];

/// The shortcut bar's providers (`ShortcutBarContainer`, FUN_100da3f7), ten of each: the name with the number 0 to 9
/// after it, category `#ShortcutBar`, the function the text of text.mdb 2011's name with the number plus one put in
/// (its `%02d`). (Also `SHORTCUT_BAR_<bar>_<slot>`, "ShortcutBarKey" with both numbers, a key for each slot of each bar,
/// which this client has no action for.)
pub const BAR_PROVIDERS: &[(&str, &str)] = &[("SHORTUCT_BAR_SELECT_", "ShortcutBarSelect"), ("SHORTUCT_BAR_ROW_", "ShortcutBarRow"), ("SHORTUCT_BAR_ACTIVE_", "ShortcutBarActive")];

/// A text with its first `%02d` (or `%d`) filled in, as `String::Format` does.
fn format_number(text: &str, n: u32) -> String {
    if let Some(at) = text.find("%02d") {
        format!("{}{n:02}{}", &text[..at], &text[at + 4..])
    } else if let Some(at) = text.find("%d") {
        format!("{}{n}{}", &text[..at], &text[at + 2..])
    } else {
        text.to_string()
    }
}

/// A listed provider: this client's action and the texts of its category and function.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub action: Action,
    pub category: String,
    pub function: String,
}

/// The providers listed, in the original's order: those with an action here, for an action with two the one
/// `keybindings::PROVIDERS` names first (the movement keys' first set). `text` gives the text of a name in a text.mdb
/// category (the name itself when there is none).
pub fn entries(text: &dyn Fn(i32, &str) -> String) -> Vec<Entry> {
    let action_of = |name: &str| {
        let action = crate::keybindings::PROVIDERS.iter().find(|p| p.0 == name).map(|p| p.1)?;
        (crate::keybindings::PROVIDERS.iter().find(|p| p.1 == action).map(|p| p.0) == Some(name)).then_some(action)
    };
    let looked = |category: i32, raw: &str| raw.strip_prefix('#').map_or_else(|| raw.to_string(), |name| text(category, name));
    let mut out: Vec<Entry> = Vec::new();
    let push = |out: &mut Vec<Entry>, action: Option<Action>, category: String, function: String| {
        if let Some(action) = action.filter(|a| !out.iter().any(|e| e.action == *a)) {
            out.push(Entry { action, category, function });
        }
    };
    for &(name, category, function) in PROVIDERS {
        push(&mut out, action_of(name), looked(2012, category), looked(2011, function));
    }
    for n in 0..10u32 {
        for &(prefix, function) in BAR_PROVIDERS {
            push(&mut out, action_of(&format!("{prefix}{n}")), looked(2012, "#ShortcutBar"), format_number(&text(2011, function), n + 1));
        }
    }
    out
}

/// The original's number for a key of this client's with its modifiers (Shift 0x20000, Ctrl 0x40000, Alt 0x80000);
/// none for a key the original's table has no number for.
pub fn input_of(binding: &Binding) -> Option<u32> {
    let name = crate::settings::binding_name(&Binding { key: binding.key, shift: false, ctrl: false, alt: false });
    let key = (0..128u32).find(|&n| crate::keybindings::key_name(n).is_some_and(|k| k.eq_ignore_ascii_case(&name)))?;
    Some(key | if binding.shift { 0x20000 } else { 0 } | if binding.ctrl { 0x40000 } else { 0 } | if binding.alt { 0x80000 } else { 0 })
}

/// A key as FUN_100bc5f5 writes it, without its colour: the modifiers (FUN_10018b05: Ctrl, Alt, Shift, in that order)
/// and the key's name in the original's table, "Inactive" for none. A key the original has no number for is written
/// with this client's name for it.
pub fn key_name(key: Option<&Binding>) -> String {
    let Some(b) = key else { return INPUT_NAMES[0].to_string() };
    let mut text = String::new();
    if b.ctrl {
        text += "CTRL+";
    }
    if b.alt {
        text += "ALT+";
    }
    if b.shift {
        text += "SHIFT+";
    }
    match input_of(b) {
        Some(input) => text += INPUT_NAMES[(input & 0x1ffff) as usize],
        None => text += &crate::settings::binding_name(&Binding { key: b.key, shift: false, ctrl: false, alt: false }).to_ascii_uppercase(),
    }
    text
}

/// [`key_name`] in aqua, as the row and the box show it (and as the rows sort by it).
pub fn key_text(key: Option<&Binding>) -> String {
    format!("<font color=aqua>{}</font>", key_name(key))
}

/// A row: its provider (an index into the entries) and its key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub entry: usize,
    pub key: Option<Binding>,
}

/// The rows (FUN_100bc713): a row for each key of each provider, one without a key for a provider with none.
pub fn fill(entries: &[Entry], keymap: &Keymap) -> Vec<Row> {
    let mut rows = Vec::new();
    for (k, e) in entries.iter().enumerate() {
        let keys = keymap.bindings_of(e.action);
        if keys.is_empty() {
            rows.push(Row { entry: k, key: None });
        }
        rows.extend(keys.into_iter().map(|key| Row { entry: k, key: Some(key) }));
    }
    rows
}

/// Clear on a row (0x100bc68e): its key unbound; then the row goes when its provider has keys left, else shows none.
/// Whether anything changed.
pub fn clear(rows: &mut Vec<Row>, at: usize, entries: &[Entry], keymap: &mut Keymap) -> bool {
    let Some(row) = rows.get(at).copied() else { return false };
    let action = entries[row.entry].action;
    if !row.key.is_some_and(|key| keymap.unbind(action, key)) {
        return false;
    }
    if keymap.bindings_of(action).is_empty() {
        rows[at].key = None;
    } else {
        rows.remove(at);
    }
    true
}

/// OK in the "Bind Key" box (FUN_100bc8c1): `change` false for Add, true for Change; `key` the key pressed, none when
/// none was. Whether anything changed.
pub fn bound(rows: &mut Vec<Row>, at: usize, change: bool, key: Option<Binding>, entries: &[Entry], keymap: &mut Keymap) -> bool {
    let Some(row) = rows.get(at).copied() else { return false };
    let action = entries[row.entry].action;
    if !change && !keymap.bindings_of(action).is_empty() {
        // (the original binds its "no key" here: this client's store cannot hold it)
        let Some(key) = key else { return false };
        if keymap.bind(action, key) {
            rows.push(Row { entry: row.entry, key: Some(key) });
            return true;
        }
        return false;
    }
    let mut changed = row.key.is_some_and(|old| keymap.unbind(action, old));
    match key {
        Some(key) => {
            if keymap.bind(action, key) {
                rows[at].key = Some(key);
                changed = true;
            }
        }
        // (the original binds "no key", which takes: the row shows none)
        None => rows[at].key = None,
    }
    changed
}

/// Reset All (FUN_100bcf9c): the keys of every action of the original's providers back to their defaults.
pub fn reset_all(keymap: &mut Keymap) {
    for &(action, _, keys) in crate::settings::ACTIONS {
        if crate::keybindings::PROVIDERS.iter().any(|p| p.1 == action) {
            keymap.set(action, keys);
        }
    }
}

/// The "Bind Key" box while it is up: the row it is for, Change (or Add), the key it shows and the key pressed.
#[derive(Clone, Copy, Debug)]
struct Dialog {
    row: usize,
    change: bool,
    shown: Option<Binding>,
    input: Option<Binding>,
}

/// The tab's view.
pub struct BindView {
    tree: Tree,
    root: Id,
    list: Id,
    entries: Vec<Entry>,
    rows: Vec<Row>,
    dialog: Option<Dialog>,
}

impl BindView {
    /// The view and its rows (FUN_100bcfb3).
    pub fn new(keymap: &Keymap) -> BindView {
        let mut tree = Tree::default();
        let root = tree.add(Node::new("BindView", Box::new(Plain)));
        tree.get_mut(root).layout = Layout::Vertical;
        let list = multilist::create(&mut tree, "BindList");
        for (k, label, width) in [(0, "Category", 100.0), (1, "Function", 200.0), (2, "Key", 100.0)] {
            multilist::add_column(&mut tree, list, k, label, width, RESIZABLE | MOVABLE | SORTABLE);
        }
        // (the list in a `BorderView_c`, both 5 in; `SetClient` puts the list over the whole border view)
        let border = super::xml::parse("<BorderView/>").first().and_then(|e| super::create(&mut tree, e)).unwrap_or_else(|| tree.add(Node::new("", Box::new(Plain))));
        tree.get_mut(border).layout = Layout::Base;
        tree.get_mut(border).borders = Rect::new(5.0, 5.0, 5.0, 5.0);
        tree.add_child(border, list);
        tree.add_child(root, border);
        // (the buttons' row: `HLayoutSpacer("spacer", 0, 16000, 1)`, then the four buttons)
        let row = tree.add(Node::new("", Box::new(Plain)));
        tree.get_mut(row).layout = Layout::Horizontal;
        let spacer = tree.add(Node::new("spacer", Box::new(Spacer { min: Point::new(0.0, -1.0), max: Point::new(16000.0, -1.0) })));
        tree.add_child(row, spacer);
        for (name, label) in [("DefaultButton", "Reset All"), ("AddButton", "Add"), ("ChangeButton", "Change"), ("RemoveButton", "Clear")] {
            let button = tree.add(Node::new(name, Box::new(super::controls::Button::new(label))));
            tree.get_mut(button).borders = Rect::new(5.0, 5.0, 5.0, 5.0);
            tree.add_child(row, button);
        }
        tree.add_child(root, row);
        let entries = entries(&|category, name| super::xml::ldb_named(category, name, name));
        let rows = fill(&entries, keymap);
        let mut view = BindView { tree, root, list, entries, rows, dialog: None };
        view.set_items(None);
        view
    }

    /// The rows to the list, `selected` selected.
    fn set_items(&mut self, selected: Option<usize>) {
        let items = self
            .rows
            .iter()
            .enumerate()
            .map(|(k, r)| {
                let e = &self.entries[r.entry];
                let key = key_text(r.key.as_ref());
                // (FUN_100bde5f: a column, then those right of it)
                let keys = vec![(0, format!("{}\u{1}{}\u{1}{key}", e.category, e.function)), (1, format!("{}\u{1}{key}", e.function)), (2, key)];
                Item { id: k as i64, cells: vec![(0, CellView::Text(e.category.clone())), (1, CellView::Text(e.function.clone())), (2, CellView::Tinted(key_name(r.key.as_ref()), 0x00ffff))], keys, ..Default::default() }
            })
            .collect();
        multilist::set_items(&mut self.tree, self.list, items);
        multilist::select(&mut self.tree, self.list, selected.map(|k| k as i64));
    }

    fn selected(&self) -> Option<usize> {
        multilist::selected(&self.tree, self.list).map(|k| k as usize).filter(|&k| k < self.rows.len())
    }

    /// Whether the "Bind Key" box waits for a key (the game's keys go to it then).
    pub fn wants_key(&self) -> bool {
        self.dialog.is_some()
    }

    /// A key pressed while the box is up (FUN_100bcc67 / FUN_100bcaaf): the box's key now.
    pub fn key_entered(&mut self, key: Binding) {
        if let Some(d) = self.dialog.as_mut() {
            d.shown = Some(key);
            d.input = Some(key);
        }
    }

    /// Draws the tab, and the "Bind Key" box when it is up; whether the keys changed.
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &crate::theme::Palette, keymap: &mut Keymap) -> bool {
        let skin = crate::skin::skin(ui.ctx());
        super::host::show(ui, &mut self.tree, self.root, skin.as_deref(), palette);
        for (_, id, _) in multilist::take_signals(&mut self.tree) {
            multilist::select(&mut self.tree, self.list, Some(id));
        }
        let mut changed = false;
        for (id, signal) in std::mem::take(&mut self.tree.signals) {
            if signal != super::host::Signal::Clicked {
                continue;
            }
            let selected = self.selected();
            match self.tree.get(id).name.as_str() {
                "DefaultButton" => {
                    reset_all(keymap);
                    self.rows = fill(&self.entries, keymap);
                    self.set_items(None);
                    changed = true;
                }
                "AddButton" | "ChangeButton" => {
                    if let (None, Some(row)) = (self.dialog, selected) {
                        self.dialog = Some(Dialog { row, change: self.tree.get(id).name == "ChangeButton", shown: self.rows[row].key, input: None });
                    }
                }
                "RemoveButton" => {
                    if let Some(row) = selected {
                        let count = self.rows.len();
                        if clear(&mut self.rows, row, &self.entries, keymap) {
                            changed = true;
                            self.set_items((self.rows.len() == count).then_some(row));
                        }
                    }
                }
                _ => {}
            }
        }
        changed
    }

    /// The "Bind Key" box (FUN_100bcca5), at the pointer; whether its OK changed the keys.
    pub fn show_dialog(&mut self, ctx: &egui::Context, palette: &crate::theme::Palette, keymap: &mut Keymap) -> bool {
        let Some(d) = self.dialog else { return false };
        let text = format!("{}<br><br>{}", super::xml::ldb_named(10000, "BindDialogText", "Press the keys to bind."), key_text(d.shown.as_ref()));
        let buttons = [super::xml::ldb_named(10000, "MsgBox_OK", "OK"), super::xml::ldb_named(10000, "MsgBox_Cancel", "Cancel")];
        let Some(answer) = crate::chrome::dialog(ctx, palette, "bind-key", "Bind Key", &text, &[&buttons[0], &buttons[1]]) else { return false };
        self.dialog = None;
        if answer != Some(0) {
            return false;
        }
        let selected = self.selected();
        let changed = bound(&mut self.rows, d.row, d.change, d.input, &self.entries, keymap);
        if changed {
            self.set_items(selected);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::parse_binding;

    fn key(text: &str) -> Binding {
        parse_binding(text).unwrap()
    }

    fn listed() -> Vec<Entry> {
        entries(&|_, name| name.to_string())
    }

    fn row_of(entries: &[Entry], rows: &[Row], action: Action) -> Vec<Option<Binding>> {
        rows.iter().filter(|r| entries[r.entry].action == action).map(|r| r.key).collect()
    }

    /// Every provider this client has an action for is in the original's table; each action is listed once, from
    /// its first provider (the second movement set is the same actions here); the bars' functions are numbered.
    #[test]
    fn the_providers_listed_are_the_clients_actions() {
        for (name, _) in crate::keybindings::PROVIDERS {
            let known = PROVIDERS.iter().any(|p| p.0 == *name) || BAR_PROVIDERS.iter().any(|b| name.strip_prefix(b.0).is_some_and(|n| n.len() == 1));
            assert!(known, "{name}");
        }
        let e = listed();
        let actions: std::collections::BTreeSet<String> = e.iter().map(|e| format!("{:?}", e.action)).collect();
        assert_eq!(actions.len(), e.len());
        let forward = e.iter().find(|e| e.action == Action::Forward).unwrap();
        assert_eq!((forward.category.as_str(), forward.function.as_str()), ("Movement", "MoveForward"));
        let bar = e.iter().find(|e| e.action == Action::HotbarRow(9)).unwrap();
        assert_eq!((bar.category.as_str(), bar.function.as_str()), ("ShortcutBar", "ShortcutBarRow"));
        let texts = entries(&|_, name| if name == "ShortcutBarSelect" { "Pick %02d".into() } else { name.into() });
        assert_eq!(texts.iter().find(|e| e.action == Action::HotbarBar(0)).unwrap().function, "Pick 01");
        assert!(!e.iter().any(|e| e.function.contains("Global") || e.function.contains("Pet")));
    }

    /// A key is written as the original writes it: Ctrl, Alt, Shift and the table's name; no key is "Inactive".
    #[test]
    fn keys_are_written_with_the_originals_names() {
        assert_eq!(key_name(None), "Inactive");
        assert_eq!(key_name(Some(&key("Ctrl+Shift+Alt+Numpad4"))), "CTRL+ALT+SHIFT+NUMPAD_4");
        assert_eq!(key_name(Some(&key("Up"))), "CursorUp");
        assert_eq!(key_name(Some(&key("Backspace"))), "BACKSPACE");
        assert_eq!(key_name(Some(&key("Shift+T"))), "SHIFT+T");
        assert_eq!(key_name(Some(&key("Backslash"))), "PIPE");
        assert_eq!(key_name(Some(&key("Semicolon"))), "SEMICOLON");
        assert_eq!(input_of(&key("Ctrl+1")), Some(0x40000 | 108));
        assert_eq!(key_text(Some(&key("F8"))), "<font color=aqua>F8</font>");
    }

    /// The rows: one per key, one without a key for an action with none.
    #[test]
    fn a_row_for_each_key() {
        let e = listed();
        let mut keymap = Keymap::default();
        keymap.set(Action::Jump, "");
        let rows = fill(&e, &keymap);
        assert_eq!(row_of(&e, &rows, Action::Forward), [Some(key("W")), Some(key("Up"))]);
        assert_eq!(row_of(&e, &rows, Action::Jump), [None]);
        assert_eq!(rows.len(), e.iter().map(|e| keymap.bindings_of(e.action).len().max(1)).sum::<usize>());
    }

    /// Clear: the row goes while the action has other keys; its last key leaves the row without one; a row without
    /// a key does nothing.
    #[test]
    fn clear_takes_the_rows_key() {
        let e = listed();
        let mut keymap = Keymap::default();
        let mut rows = fill(&e, &keymap);
        let at = |rows: &[Row], k: &str| rows.iter().position(|r| e[r.entry].action == Action::Forward && r.key == Some(key(k))).unwrap();
        let n = rows.len();
        let k = at(&rows, "W");
        assert!(clear(&mut rows, k, &e, &mut keymap));
        assert_eq!((rows.len(), keymap.keys(Action::Forward)), (n - 1, "Up".to_string()));
        let k = at(&rows, "Up");
        assert!(clear(&mut rows, k, &e, &mut keymap));
        assert_eq!(row_of(&e, &rows, Action::Forward), [None]);
        assert_eq!(rows.len(), n - 1);
        let empty = rows.iter().position(|r| e[r.entry].action == Action::Forward).unwrap();
        assert!(!clear(&mut rows, empty, &e, &mut keymap));
    }

    /// Add and Change: Add on an action with keys binds one more (a new row; the same key again does nothing);
    /// Change replaces the row's key; Add on an action without keys does the same as Change; OK without a key
    /// pressed leaves a Change'd row without its key and an Add does nothing. The key stays on other actions.
    #[test]
    fn the_bind_box_adds_and_changes() {
        let e = listed();
        let mut keymap = Keymap::default();
        let mut rows = fill(&e, &keymap);
        let attack = rows.iter().position(|r| e[r.entry].action == Action::Attack).unwrap();
        let n = rows.len();
        assert!(bound(&mut rows, attack, false, Some(key("F")), &e, &mut keymap));
        assert_eq!((rows.len(), keymap.keys(Action::Attack)), (n + 1, "Q, F".to_string()));
        assert!(!bound(&mut rows, attack, false, Some(key("F")), &e, &mut keymap));
        assert!(!bound(&mut rows, attack, false, None, &e, &mut keymap));
        assert!(bound(&mut rows, attack, true, Some(key("E")), &e, &mut keymap));
        assert_eq!((rows[attack].key, keymap.keys(Action::Attack)), (Some(key("E")), "F, E".to_string()));
        assert_eq!(keymap.keys(Action::Use), "E");
        assert!(bound(&mut rows, attack, true, None, &e, &mut keymap));
        assert_eq!((rows[attack].key, keymap.keys(Action::Attack)), (None, "F".to_string()));
        keymap.set(Action::Jump, "");
        let mut rows = fill(&e, &keymap);
        let jump = rows.iter().position(|r| e[r.entry].action == Action::Jump).unwrap();
        let n = rows.len();
        assert!(bound(&mut rows, jump, false, Some(key("Space")), &e, &mut keymap));
        assert_eq!((rows.len(), rows[jump].key), (n, Some(key("Space"))));
    }

    /// Reset All brings the providers' actions back to their default keys and leaves the client's other keys.
    #[test]
    fn reset_all_restores_the_defaults() {
        let mut keymap = Keymap::default();
        keymap.set(Action::Attack, "F");
        keymap.set(Action::HotbarRow(3), "");
        keymap.set(Action::CameraReset, "Home");
        reset_all(&mut keymap);
        assert_eq!((keymap.keys(Action::Attack), keymap.keys(Action::HotbarRow(3)), keymap.keys(Action::CameraReset)), ("Q".to_string(), "Shift+4".to_string(), "Home".to_string()));
    }

    /// The rows sort as the original compares them: by category, then function, then the key's text with its markup
    /// (so F10 comes before F1, whose closing `<` sorts after `0`).
    #[test]
    fn rows_sort_by_their_texts() {
        let mut keymap = Keymap::default();
        keymap.set(Action::Screenshot, "F1, F10");
        let view = BindView::new(&keymap);
        let order = multilist::order(&view.tree, view.list);
        let shown: Vec<(String, String, Option<Binding>)> = order.iter().map(|&k| view.rows[k as usize]).map(|r| (view.entries[r.entry].category.clone(), view.entries[r.entry].function.clone(), r.key)).collect();
        let mut sorted = shown.clone();
        sorted.sort_by(|a, b| (a.0.as_str(), a.1.as_str(), key_text(a.2.as_ref())).cmp(&(b.0.as_str(), b.1.as_str(), key_text(b.2.as_ref()))));
        assert_eq!(shown, sorted);
        let screenshot: Vec<Option<Binding>> = shown.iter().filter(|s| s.1 == "CameraScreenshot").map(|s| s.2).collect();
        assert_eq!(screenshot, [Some(key("F10")), Some(key("F1"))]);
    }

    /// `MAO_GAME`: every category and function name has its text in text.mdb.
    #[test]
    fn the_texts_are_in_the_game() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let texts = mao_formats::ldb::Texts::parse(&std::fs::read(std::path::Path::new(&game).join("cd_image/text/text.mdb")).unwrap()).unwrap();
        let missing: Vec<String> = entries(&|category, name| texts.named(category, name).map_or_else(|| format!("missing {category} {name}"), str::to_string)).into_iter().flat_map(|e| [e.category, e.function]).filter(|t| t.starts_with("missing")).collect();
        assert!(missing.is_empty(), "{missing:?}");
    }
}
