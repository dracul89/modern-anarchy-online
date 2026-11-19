//! The vehicles window, GUI.dll's mech select (FUN_100f3e80, the module `mechselect_window`, `MechSelectConfig`): the
//! vehicles the character's side has researched, one to be ordered, and the way out of the one it sits in.
//!
//! * It is opened by the control center's vehicle button and its key (`OpenVehicleWindowMessage`, Shift+V), which
//!   turn the module on, and by a used terminal whose template names window kind 7 (for a character whose stat 389
//!   has bit 0x20; `client_window`). Turned on, the module (FUN_1002ef9c) makes the window only when more than 20
//!   seconds have gone since a vehicle was ordered (`mech_ordered`, the server's time then), `N3Msg_CanUseMech`
//!   holds (outside, and vehicles allowed in the playfield: each says its line otherwise) and the character's stat
//!   0x213 has not bit 0.
//! * The window is `Views/MechSelect.xml` in a window with one tab, "Vehicle" (not drawn here), moved to the screen's
//!   middle. `MechButtons` is emptied and gets a button for each finished global research goal
//!   (`N3Msg_GetCompletedGlobalResearchGoals`) whose item's stat 177 has 0x200 (FUN_100f3b73): its label the goal's
//!   name, its picture the item's icon, alive when the character meets the goal's criteria
//!   (`N3Msg_MeetsPerkCriteria`); all as wide as the widest. It asks for the global research as it comes up
//!   (`N3Msg_UpdateGlobalResearch`).
//! * A click on one (LAB_100f3a51) chooses it: `VehicleInfo` shows its description (`N3Msg_GetPerkDescription`) and
//!   "Order Vehicle", dead until then, comes alive.
//! * `mode` shows "Leave vehicle" in place of "Order Vehicle" when the character's stat 0x296 (`MechData`) is not 0
//!   as the window is made.
//! * "Order Vehicle" (LAB_100f3931): `N3Msg_ActivateMech` of the one chosen, and the server's time kept as
//!   `mech_ordered`. "Leave vehicle" (FUN_100f3a36): `N3Msg_ActivateMech(0)`, and the window closes. "Close" closes.
//! * `N3Msg_ActivateMech` (Gamecode 0x10027250) asks FUN_10079c50 first. While a lock on 0x296 runs it sends nothing
//!   and says how long is left (FUN_10064847, the line that names nothing). For a vehicle it then wants the
//!   character outside (`Feedback_MustBeOutside`), not falling, in a playfield that allows vehicles
//!   (`Feedback_MechCantBeOrdered`), not in resurrection shock (stat 0xf7; `Feedback_NotWhileInResurrectionShock`)
//!   and, for a vehicle whose record's flags have 0x100, on a battlestation or in the organisation's own tower
//!   area (`Feedback_OnlyInOwnLCArea`). Then `SpawnMech` (`464d000a`) goes to the server with the goal's number, 0
//!   to leave.
//!
//! Here the lock and the resurrection shock are asked; whether the character is outside, falling, where vehicles
//! are allowed or in its own tower area is left to the server, which this client cannot tell. The buttons have no
//! picture. No `SpawnMech` is in a capture: the message is the decompile's.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::perks::PerkState;
use crate::theme::Palette;

/// What the window asks of the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// The global research lines (`3115534d`).
    Research,
    /// `SpawnMech` with a research goal's number, or 0 to leave the vehicle.
    Spawn(u32),
}

static COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());

pub fn take_commands() -> Vec<Command> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn send(c: Command) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(c);
}

/// The server's time when a vehicle was last ordered (the original's `mech_ordered`).
static ORDERED: AtomicU32 = AtomicU32::new(0);

/// An item's flag (stat 177) that makes a research goal a vehicle.
const VEHICLE: u32 = 0x200;
/// The lock that holds orders back, and the stat that says the character sits in a vehicle.
const MECH: u32 = 0x296;

/// Whether the module makes the window when it is turned on (FUN_1002ef9c), by what this client can tell: more than
/// 20 seconds since the last order, and the character's stat 0x213 without bit 0.
pub fn may_open(stats: &BTreeMap<u32, i64>, now: u32) -> bool {
    now.wrapping_sub(ORDERED.load(Ordering::Relaxed)) > 20 && stats.get(&0x213).copied().unwrap_or(0) & 1 == 0
}

/// A vehicle to order: the research goal's number, its name, and whether the character meets its criteria.
#[derive(Clone, Debug, PartialEq)]
pub struct Vehicle {
    pub id: u32,
    pub name: String,
    pub meets: bool,
}

/// FUN_100f3b73's list: the finished global research goals whose item is a vehicle.
pub fn vehicles(model: &crate::research::Model) -> Vec<Vehicle> {
    model
        .finished_global()
        .into_iter()
        .filter_map(|id| model.catalog.get(id))
        .filter(|r| r.more_flags & VEHICLE != 0)
        .map(|r| Vehicle { id: r.id, name: r.name.clone(), meets: crate::perks::meets_perk(r, model.stats, &model.owned) })
        .collect()
}

/// What an order does (`N3Msg_ActivateMech` with FUN_10079c50, as far as it is asked here): the message sent, or
/// the line said instead.
pub fn order(vehicle: u32, lock_left: Option<f32>, stats: &BTreeMap<u32, i64>) -> Result<Command, String> {
    if let Some(left) = lock_left {
        return Err(crate::feedback::unable_to_perform_plain(left));
    }
    if vehicle != 0 && stats.get(&0xf7).copied().unwrap_or(0) != 0 {
        return Err(crate::feedback::plain("Feedback_NotWhileInResurrectionShock"));
    }
    Ok(Command::Spawn(vehicle))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

#[derive(Default)]
pub struct VehiclesWindow {
    pub open: bool,
    window: Option<crate::gui::window::XmlWindow>,
    /// The vehicles the buttons were made for, and the one chosen.
    listed: Vec<Vehicle>,
    chosen: Option<u32>,
}

impl VehiclesWindow {
    /// The key and the button: off when it is up, else on when the module would make it.
    pub fn toggle(&mut self, stats: &BTreeMap<u32, i64>) {
        self.open = !self.open && may_open(stats, crate::gametime::server_now());
    }

    fn load(&mut self, ctx: &egui::Context, stats: &BTreeMap<u32, i64>) -> bool {
        let Some(mut w) = crate::gui::window::XmlWindow::load(ctx, "MechSelect", "Vehicle", "MechSelect.xml") else { return false };
        w.tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = w.root;
        if let Some(order) = w.tree.find(root, "OrderVehicle") {
            w.tree.get_mut(order).enabled = false;
        }
        if stats.get(&MECH).copied().unwrap_or(0) != 0
            && let Some(mode) = w.tree.find(root, "mode")
        {
            crate::gui::selector::set_value(&mut w.tree, mode, 1);
        }
        self.window = Some(w);
        self.listed.clear();
        self.chosen = None;
        send(Command::Research);
        true
    }

    /// The buttons made again for the vehicles there are now.
    fn fill(&mut self, list: Vec<Vehicle>) {
        let Some(w) = self.window.as_mut() else { return };
        let Some(buttons) = w.tree.find(w.root, "MechButtons") else { return };
        w.tree.get_mut(buttons).children.clear();
        for v in &list {
            let text = format!("<Button label=\"{}\" layout_borders=\"Rect(5,5,5,5)\" width_group=\"vehicles\" width_group_owner=\":our_parent:\" name=\"{}\"/>", escape(&v.name), v.id);
            if let Some(id) = crate::gui::xml::parse(&text).first().and_then(|e| crate::gui::create(&mut w.tree, e)) {
                w.tree.get_mut(id).enabled = v.meets;
                w.tree.add_child(buttons, id);
            }
        }
        if let Some(id) = crate::gui::xml::parse("<VLayoutSpacer/>").first().and_then(|e| crate::gui::create(&mut w.tree, e)) {
            w.tree.add_child(buttons, id);
        }
        w.tree.get_mut(buttons).layout = crate::gui::view::Layout::Vertical;
        w.tree.relayout = true;
        self.listed = list;
    }

    /// Draws the window: the lines it says.
    pub fn show(&mut self, ctx: &egui::Context, state: &PerkState, stats: &BTreeMap<u32, i64>, palette: &Palette) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.open {
            self.window = None;
            return lines;
        }
        if self.window.is_none() && !self.load(ctx, stats) {
            self.open = false;
            return lines;
        }
        let research = crate::research::snapshot();
        let model = crate::research::Model::new(&state.catalog, &research, stats, &state.owned);
        let list = vehicles(&model);
        // (the original makes the buttons once, as the window is made; the research asked for then comes after, so
        // here they follow what is known)
        if list != self.listed || self.window.as_ref().is_some_and(|w| w.tree.find(w.root, "Mech 1").is_some()) {
            self.fill(list);
        }
        let Some(w) = self.window.as_mut() else { return lines };
        let root = w.root;
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
        let tree = &mut w.tree;
        let mut is_open = true;
        let size = min.max(egui::vec2(420.0, 300.0));
        let middle = ctx.content_rect().center() - size / 2.0;
        crate::chrome::window(ctx, palette, "MechSelect", "Vehicle", &mut is_open, |win| win.default_size(size).resizable(true).default_pos(crate::layout::pos("MechSelect", middle)), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        let lock_left = crate::actions::skill_lock_left(MECH).map(|l| l.0);
        for (id, signal) in std::mem::take(&mut w.tree.signals) {
            if signal != crate::gui::host::Signal::Clicked {
                continue;
            }
            let name = w.tree.get(id).name.clone();
            match name.as_str() {
                "OrderVehicle" => {
                    if let Some(vehicle) = self.chosen {
                        match order(vehicle, lock_left, stats) {
                            Ok(command) => send(command),
                            Err(line) => lines.push(line),
                        }
                        ORDERED.store(crate::gametime::server_now(), Ordering::Relaxed);
                    }
                }
                "LeaveVehicle" => {
                    match order(0, lock_left, stats) {
                        Ok(command) => send(command),
                        Err(line) => lines.push(line),
                    }
                    is_open = false;
                }
                "Quit" => is_open = false,
                other => {
                    // (a vehicle's button, named by its number: chosen, its description shown, the order alive)
                    if let Some(vehicle) = other.parse::<u32>().ok().filter(|v| self.listed.iter().any(|l| l.id == *v)) {
                        self.chosen = Some(vehicle);
                        if let Some(info) = w.tree.find(root, "VehicleInfo").and_then(|id| w.tree.control_mut::<crate::gui::controls::TextView>(id)) {
                            info.set_text(&model.description(vehicle));
                        }
                        if let Some(order) = w.tree.find(root, "OrderVehicle") {
                            w.tree.get_mut(order).enabled = true;
                        }
                        w.tree.relayout = true;
                    }
                }
            }
        }
        if !is_open {
            self.open = false;
            self.window = None;
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_order_waits_for_its_lock_and_for_the_shock_to_pass() {
        let stats = |shock: i64| [(0xf7, shock)].into_iter().collect::<BTreeMap<u32, i64>>();
        assert_eq!(order(0xc80, None, &stats(0)), Ok(Command::Spawn(0xc80)));
        assert!(order(0xc80, Some(12.0), &stats(0)).is_err());
        assert!(order(0xc80, None, &stats(1)).is_err());
        // (leaving is an order of none: the lock holds it back, the shock does not)
        assert_eq!(order(0, None, &stats(1)), Ok(Command::Spawn(0)));
        assert!(order(0, Some(3.0), &stats(0)).is_err());
    }

    #[test]
    fn the_window_is_not_made_right_after_an_order() {
        let none = BTreeMap::new();
        ORDERED.store(1000, Ordering::Relaxed);
        assert!(!may_open(&none, 1015));
        assert!(may_open(&none, 1021));
        assert!(!may_open(&[(0x213, 1)].into_iter().collect(), 1021));
        ORDERED.store(0, Ordering::Relaxed);
    }
}
