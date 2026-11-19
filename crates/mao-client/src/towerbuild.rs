//! The tower build menu, GUI.dll's `TowerBuildMenuView_c` (FUN_1005cb45, `towerbuildmenu_window`, 369 by 409 in the
//! screen's middle): a tower or a controller item checked against the place the character stands on and built
//! there, and in its other way of working a shield disabler used on a controller's shield.
//!
//! * **What opens it.** The map control's "Build" and "War" buttons (FUN_100ec95f, FUN_100ec9f3), each closing the
//!   menu again when it is up that way already and holds nothing new (FUN_1002de1a); and the use of an item: the
//!   interface's `N3Msg_UseItem` (Interfaces.dll 0x1000a73f) does not use an item whose template's stat 388 is not
//!   0, or one that is a shield disabler (its use action has a criterion of operator 0x53, Gamecode 0x1002765d),
//!   but opens the menu with it. An inventory item let go on the menu is taken the same way (FUN_1005cadf).
//! * **With nothing in it** (FUN_1005c605 with no item): the text and the label ask for a tower item or for a
//!   disabler, and the second button reads "Build" or "Disable shield".
//! * **With an item** (FUN_1005c605): a disabler makes it the war menu; an item whose stat 388 is 0 is no tower item
//!   (a line says so, nothing changes); 1 is a controller, anything else a tower. The label takes the checklist's
//!   name, the item goes into the slot, the second button dies, and the checklist is asked for at once.
//! * **Check** (FUN_1005aece, `N3Msg_RequestChecklist`, Gamecode 0x10029668): while the lock on Computer Literacy
//!   (stat 161) runs nothing is sent and the wait is said; else the item's template (low and high) and quality go
//!   to the server (`CharacterAction` 0xaf). Either way the text is emptied and both buttons die; Check lives
//!   again when that lock has run out (the specials' signal, FUN_1005aaa5).
//! * **The answer** (`CharacterAction` 0xb0 to the character, its second parameter the flags of what failed;
//!   FUN_1005ac9a): a line for each entry of the item's list ([`CONTROLLER`], [`TOWER`], [`WAR`]), marked failed
//!   when its bit is in the flags, each a link to its help page. With nothing failed the second button lives.
//!   Flags no entry knows are told in a line.
//! * **Moving** (a timer every 0.2 seconds, FUN_1005a9d7): the second button dies when the character is no longer
//!   where it was, until the next check passes.
//! * **Build / Disable shield** (FUN_1005aa2c): the item is used, as any item is (`BuildTower` is the plain
//!   `N3Msg_UseItem`, without the interface's detour), and the menu closes at the click.
//! * The menu closes when its item is gone (FUN_1005aa52).
//!
//! None of its messages is in a capture: the checklist's request and answer are the decompile's. An item's low
//! and high template and its quality are taken for the three stats the original sends (702, 703, 701). Not ported:
//! the answer for an item that is not one's own (a "simulated check"; the menu only takes one's own here), the
//! mark the inventory puts on the item that is in the menu, the lock following an item that is moved.

use std::sync::Mutex;

use crate::gear::GearItem;
use crate::theme::Palette;

/// What the menu asks of the server: the checklist of an item (template low and high, quality).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Check {
    pub low: u32,
    pub high: u32,
    pub quality: u32,
}

static CHECKS: Mutex<Vec<Check>> = Mutex::new(Vec::new());

pub fn take_checks() -> Vec<Check> {
    std::mem::take(&mut *CHECKS.lock().unwrap_or_else(|e| e.into_inner()))
}

/// The lock that holds a check back: Computer Literacy.
pub const LOCK: u32 = 161;

/// What kind of item is in the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Controller,
    Tower,
    Disabler,
}

/// Whether an item opens the menu when it is used, and as what (the interface's `N3Msg_UseItem`): `tower` is its
/// template's stat 388.
pub fn kind_of(item: &GearItem, tower: Option<u32>) -> Option<Kind> {
    match tower {
        Some(1) => Some(Kind::Controller),
        Some(v) if v != 0 => Some(Kind::Tower),
        _ => item.actions.iter().any(|a| a.kind == 3 && a.criteria.iter().any(|c| c.2 == 0x53)).then_some(Kind::Disabler),
    }
}

/// A checklist's entry: the bit of the answer that fails it, its text's name (category 503) and its help page.
type Entry = (u32, &'static str, &'static str);

/// A controller's checklist (FUN_1005af0d's first list).
pub const CONTROLLER: [Entry; 13] = [
    (0x20, "LC_CheckOrg", "BuildMenu_Controller_InOrg"),
    (0x80, "LC_CheckOrgRank", "BuildMenu_Controller_HighOrgRank"),
    (0x8000, "LC_CheckOrgControllerCount", "BuildMenu_Controller_WithinTowerLimit"),
    (0x1000, "LC_CheckLCEnabled", "BuildMenu_Controller_PositionIsLegal"),
    (0x40_0000, "LC_CheckLCDisabled", "BuildMenu_Controller_Approved"),
    (0x40, "LC_CheckSameOrg", "BuildMenu_Controller_OrgOwn"),
    (0x400, "LC_CheckExistingController", "BuildMenu_Controller_OneController"),
    (0x2000, "LC_CheckPeaceState", "BuildMenu_Controller_Aftermath"),
    (0x4_0000, "LC_CheckTowerLevel", "BuildMenu_Controller_WithinRange"),
    (0x1_0000, "LC_CheckControllerType", "BuildMenu_Controller_UniqueType"),
    (0x8_0000, "LC_CheckSkillReq", "BuildMenu_Controller_Skills"),
    (0x200, "LC_CheckExp", "BuildMenu_Controller_Skills"),
    (0x80_0000, "LC_CheckFlying", "BuildMenu_Player_Flying"),
];

/// A tower's.
pub const TOWER: [Entry; 15] = [
    (0x20, "LC_CheckOrg", "BuildMenu_Tower_InOrg"),
    (0x1000, "LC_CheckLCEnabled", "BuildMenu_Tower_PositionIsLegal"),
    (0x40_0000, "LC_CheckLCDisabled", "BuildMenu_Tower_Approved"),
    (0x100, "LC_CheckControllerPresent", "BuildMenu_Tower_ControllerInDistrict"),
    (0x20_0000, "LC_CheckDefenceShield", "BuildMenu_Tower_ControllerShield"),
    (0x40, "LC_CheckSameOrg", "BuildMenu_Tower_OrgOwn"),
    (0x4_0000, "LC_CheckTowerLevel", "BuildMenu_Tower_DistrictLevelRange"),
    (0x4000, "LC_CheckTowerCount", "BuildMenu_Tower_TowerDistrictLimit"),
    (0x4, "LC_CheckTowerProximity", "BuildMenu_Tower_TooCrowded"),
    (0x2, "LC_CheckTowerDistance", "BuildMenu_Tower_TooClose"),
    (0x2_0000, "LC_CheckCharTowerCount", "BuildMenu_Tower_OwnTooMany"),
    (0x10_0000, "LC_CheckCharTowerUnique", "BuildMenu_Tower_Unique"),
    (0x8_0000, "LC_CheckSkillReq", "BuildMenu_Tower_SkillRequirement"),
    (0x200, "LC_CheckExp", "BuildMenu_Tower_SkillRequirement"),
    (0x80_0000, "LC_CheckFlying", "BuildMenu_Player_Flying"),
];

/// A shield disabler's.
pub const WAR: [Entry; 6] = [
    (0x8, "LC_CheckFightMode", "WarMenu_NoFighting"),
    (0x10, "LC_CheckControllerPresent", "WarMenu_NoController"),
    (0x2, "LC_CheckDefenceShield", "WarMenu_NotEnabled"),
    (0x4, "LC_CheckDisablerLevel", "WarMenu_TooLowLevel"),
    (0x40, "LC_CheckPvPLevelRange", "WarMenu_TooHighLevel"),
    (0x8_0000, "LC_CheckSkillReq", "WarMenu_SkillRequirement"),
];

fn list(kind: Kind) -> &'static [Entry] {
    match kind {
        Kind::Controller => &CONTROLLER,
        Kind::Tower => &TOWER,
        Kind::Disabler => &WAR,
    }
}

/// The checklist as the answer's flags mark it (FUN_1005ac9a): its lines' markup, whether a check failed, and the
/// flags no entry of the list knows. `text` gives an entry's text by its name.
pub fn checklist(kind: Kind, flags: u32, text: &dyn Fn(&str) -> String) -> (String, bool, u32) {
    let (mut html, mut failed, mut known) = (String::new(), false, 0);
    for &(mask, label, page) in list(kind) {
        let fails = mask & flags != 0;
        failed |= fails;
        known |= mask;
        let marker = if fails { "GFX_GUI_CHECKLIST_MARKER_NEGATIVE" } else { "GFX_GUI_CHECKLIST_MARKER_POSITIVE" };
        html += &format!("<div indent=wrapped><img src=tdb://id:{marker}> <a href=\"/showfile {page}.html\">{}</a></div>", text(label));
    }
    (html, failed, flags & !known)
}

/// The item in the menu.
#[derive(Clone)]
struct Held {
    place: u32,
    low: u32,
    high: u32,
    quality: u32,
    kind: Kind,
    icon: Option<std::sync::Arc<crate::gear::Icon>>,
}

/// What the menu did this frame, for the interface to carry out.
#[derive(Default)]
pub struct Output {
    /// Lines for the chat's system channel.
    pub lines: Vec<String>,
    /// Chat commands (a help page's `/showfile`).
    pub commands: Vec<String>,
    /// The inventory place of the item to be used (Build, Disable shield).
    pub use_item: Option<u32>,
}

#[derive(Default)]
pub struct TowerWindow {
    pub open: bool,
    /// The war menu (a disabler's), else the build menu.
    war: bool,
    item: Option<Held>,
    text: String,
    label: String,
    build_on: bool,
    check_on: bool,
    /// A check was made: Check lives again when the lock on Computer Literacy, once seen, is over.
    waiting: Option<bool>,
    stood: Option<[f32; 3]>,
    /// Lines to be said the next time the menu is drawn (the answer's unknown marks).
    pub pending: Vec<String>,
}

fn text(name: &str, own: &str) -> String {
    crate::gui::xml::ldb_named(503, name, own)
}

impl TowerWindow {
    /// Whether the map marks where the tower would stand: the menu took an item (FUN_1005c605 tells the map data its
    /// identity through GlobalSignals +0x160, FUN_10042f87 sets +0x75), until the menu goes (its destructor FUN_1005aadb
    /// tells (0, 0)) or the item does (+0x9c, FUN_10042fc6).
    pub fn placing(&self) -> bool {
        self.open && self.item.is_some()
    }

    /// The map control's button: the menu that way with nothing in it, or closed when it is up that way already.
    pub fn toggle(&mut self, build: bool) {
        if self.open && self.war != build {
            self.open = false;
            return;
        }
        self.open = true;
        self.war = !build;
        (self.text, self.label) = if build { (text("LC_DropTowerMsg", "Put a tower item here."), text("LC_InsertTowerItem", "Tower item")) } else { (text("LC_DropDefShieldNeut", "Put a shield disabler here."), text("LC_InsertDisablerItem", "Shield disabler")) };
        self.check_on = true;
    }

    /// An item used, or let go on the menu: the line said when it is no tower item.
    pub fn insert(&mut self, item: &GearItem, tower: Option<u32>, lock_left: Option<f32>) -> Option<String> {
        let Some(kind) = kind_of(item, tower) else { return Some(text("LC_NotTowerItem", "That is no tower item.")) };
        self.open = true;
        self.war = kind == Kind::Disabler;
        self.label = match kind {
            Kind::Disabler => text("LC_WarCheckList", "War checklist"),
            Kind::Controller => text("LC_ControllerCheckList", "Controller checklist"),
            Kind::Tower => text("LC_TowerCheckList", "Tower checklist"),
        };
        self.item = Some(Held { place: item.slot, low: item.item_id, high: item.high_id, quality: item.quality, kind, icon: item.icon.clone() });
        self.build_on = false;
        self.check(lock_left)
    }

    /// Check: the checklist asked for, or the wait said while the lock runs.
    fn check(&mut self, lock_left: Option<f32>) -> Option<String> {
        let held = self.item.as_ref()?;
        let line = match lock_left {
            Some(left) => Some(crate::feedback::unable_to_perform(&crate::feedback::skill_name(LOCK), left)),
            None => {
                CHECKS.lock().unwrap_or_else(|e| e.into_inner()).push(Check { low: held.low, high: held.high, quality: held.quality });
                None
            }
        };
        self.text.clear();
        (self.build_on, self.check_on) = (false, false);
        self.waiting = Some(lock_left.is_some());
        line
    }

    /// The server's answer to a check.
    pub fn set_flags(&mut self, flags: u32) -> Option<String> {
        let held = self.item.as_ref()?;
        let (html, failed, unknown) = checklist(held.kind, flags, &|name| text(name, name));
        self.text = html;
        self.build_on = !failed;
        (!failed && flags != 0).then(|| format!("The checklist's answer has marks this menu does not know: {unknown:08x}."))
    }

    /// The item of an inventory place, for the previews.
    pub fn sample(item: &GearItem, flags: u32) -> TowerWindow {
        let mut w = TowerWindow::default();
        w.insert(item, Some(2), None);
        take_checks();
        w.set_flags(flags);
        w
    }

    /// Draws the menu.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, gear: &[GearItem], position: Option<[f32; 3]>, lock_left: Option<f32>, texture: &mut dyn FnMut(&crate::gear::Icon) -> egui::TextureId) -> Output {
        let mut out = Output { lines: std::mem::take(&mut self.pending), ..Default::default() };
        if !self.open {
            (self.item, self.waiting, self.stood) = (None, None, None);
            return out;
        }
        // (the item gone from its place: the menu goes)
        if self.item.as_ref().is_some_and(|h| !gear.iter().any(|g| g.slot == h.place && g.item_id == h.low)) {
            self.open = false;
            return out;
        }
        // (Check lives again when the lock on Computer Literacy has come and gone)
        if let Some(seen) = self.waiting {
            match (seen, lock_left.is_some()) {
                (false, true) => self.waiting = Some(true),
                (true, false) => (self.waiting, self.check_on) = (None, true),
                _ => {}
            }
        }
        // (moved since the last look: the second button dies until a check passes again)
        if position != self.stood {
            self.stood = position;
            self.build_on = false;
        }
        // (an inventory item let go on the menu is taken)
        let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
        let mut is_open = true;
        let (mut check, mut build, mut dropped) = (false, false, false);
        let title = crate::gui::xml::ldb_named(10000, "BuildMenu", "Build menu");
        let size = egui::vec2(369.0, 409.0);
        let middle = ctx.content_rect().center() - size / 2.0;
        let button = if self.war { text("LC_ButtonDisableShield", "Disable shield") } else { text("LC_ButtonBuild", "Build") };
        let icon = self.item.as_ref().and_then(|h| h.icon.as_ref()).map(|i| texture(i));
        let shown = crate::chrome::window(ctx, palette, "TowerBuildMenu", &title, &mut is_open, |w| w.default_pos(crate::layout::pos("TowerBuildMenu", middle)).default_size(size).resizable(false), |ui| {
            // (369 by 409 with the frame: the text's room is what the label, the slot and the buttons leave)
            ui.set_width(size.x - 16.0);
            ui.vertical_centered(|ui| {
                ui.add_space(5.0);
                ui.colored_label(egui::Color32::from_rgb(0xbb, 0xbb, 0xbb), &self.label);
                ui.add_space(10.0);
                let (slot, _) = ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::hover());
                ui.painter().rect_stroke(slot, 2.0, egui::Stroke::new(1.0, crate::chrome::rgb(palette.accent)), egui::StrokeKind::Inside);
                if let Some(icon) = icon {
                    ui.painter().image(icon, slot.shrink(1.0), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                }
            });
            ui.add_space(10.0);
            let room = 232.0;
            egui::Frame::NONE.stroke(egui::Stroke::new(1.0, crate::chrome::rgb(palette.accent))).inner_margin(6).show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(room).auto_shrink([false, false]).show(ui, |ui| {
                    if let Some(link) = crate::gui::controls::markup(ui, palette, &self.text, crate::gui::font::FontId::NORMAL, crate::chrome::rgb(palette.text)) {
                        out.commands.push(link);
                    }
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                build = crate::chrome::button(ui, palette, &button, self.build_on && self.item.is_some()).clicked();
                check = crate::chrome::button(ui, palette, &text("LC_ButtonCheck", "Check"), self.check_on && self.item.is_some()).clicked();
            });
        });
        if dragged != u32::MAX
            && crate::gear::letting_go(ctx)
            && shown.as_ref().is_some_and(|s| s.response.contains_pointer())
        {
            dropped = true;
        }
        if dropped && let Some(item) = gear.iter().find(|g| g.slot == dragged) {
            out.lines.extend(self.insert(item, crate::net::item_template_stat(item.item_id, 388), lock_left));
        }
        if check {
            out.lines.extend(self.check(lock_left));
        }
        if build && let Some(held) = self.item.as_ref() {
            out.use_item = Some(held.place);
            is_open = false;
        }
        if !is_open {
            self.open = false;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(actions: Vec<mao_formats::item::Action>) -> GearItem {
        GearItem { slot: 70, item_id: 11, high_id: 12, count: 1, quality: 150, bag: false, container: (0, 0), flags: 0, can: 0, name: "A thing".into(), description: String::new(), icon: None, effects: Vec::new(), actions }
    }

    #[test]
    fn an_items_kind_is_by_its_tower_stat_or_its_use() {
        assert_eq!(kind_of(&item(Vec::new()), Some(1)), Some(Kind::Controller));
        assert_eq!(kind_of(&item(Vec::new()), Some(7)), Some(Kind::Tower));
        assert_eq!(kind_of(&item(Vec::new()), Some(0)), None);
        assert_eq!(kind_of(&item(Vec::new()), None), None);
        // (a disabler: its use has a criterion of operator 0x53; the same criterion on another action is nothing)
        let with = |kind| vec![mao_formats::item::Action { kind, criteria: vec![(1, 2, 0x53)] }];
        assert_eq!(kind_of(&item(with(3)), None), Some(Kind::Disabler));
        assert_eq!(kind_of(&item(with(6)), Some(0)), None);
    }

    #[test]
    fn a_checklist_marks_what_failed() {
        let name = |n: &str| n.to_string();
        let (html, failed, unknown) = checklist(Kind::Tower, 0x1004, &name);
        assert!(failed && unknown == 0);
        assert_eq!(html.matches("MARKER_NEGATIVE").count(), 2);
        assert_eq!(html.matches("MARKER_POSITIVE").count(), 13);
        assert!(html.contains("<a href=\"/showfile BuildMenu_Tower_TooCrowded.html\">LC_CheckTowerProximity</a>"));
        // (nothing failed; a mark no entry knows is handed back)
        let (_, failed, unknown) = checklist(Kind::Controller, 0, &name);
        assert!(!failed && unknown == 0);
        let (_, failed, unknown) = checklist(Kind::Disabler, 0x1000_0000, &name);
        assert!(!failed && unknown == 0x1000_0000);
        // (the war list's own bits: 0x40 there is the level range, not the organisation)
        assert!(checklist(Kind::Disabler, 0x40, &name).0.contains("MARKER_NEGATIVE> <a href=\"/showfile WarMenu_TooHighLevel.html\""));
    }

    #[test]
    fn the_menu_checks_an_item_and_waits_for_its_lock() {
        let mut w = TowerWindow::default();
        // (the map's button: up that way; again: gone)
        w.toggle(true);
        assert!(w.open && !w.war && w.check_on);
        w.toggle(true);
        assert!(!w.open);
        // (no tower item: a line, and nothing changes)
        assert!(w.insert(&item(Vec::new()), Some(0), None).is_some());
        assert!(!w.open);
        // (a tower: checked at once, both buttons dead)
        take_checks();
        assert!(w.insert(&item(Vec::new()), Some(5), None).is_none());
        assert_eq!(take_checks(), vec![Check { low: 11, high: 12, quality: 150 }]);
        assert!(w.open && !w.build_on && !w.check_on);
        // (the answer with nothing failed: the second button lives)
        assert!(w.set_flags(0).is_none());
        assert!(w.build_on);
        // (a check while the lock runs: the wait said, nothing sent)
        assert!(w.check(Some(12.0)).is_some());
        assert!(take_checks().is_empty() && !w.build_on);
    }
}
