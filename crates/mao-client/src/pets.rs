//! The pet window, GUI.dll's `PetWindowModule_c` (`pet_window`, the control center's Pets menu's "#PetInfo") and its
//! `PetWindow_c` (FUN_1007e9d7): a window 400 by 200 (`Rect(200, 300, 600, 500)`, moved to the middle) titled text.mdb
//! 10000's "PetWindow" ("Pet Window"), its one tab "Pets" holding a `PetListView_c` (FUN_1007cd89, `pet_view`): the
//! character's pets one under the other, a `PetView_c` (FUN_1007d817) each, in the order of their places
//! (FUN_1007cc58 asks `GetPetID` for place 1 on and stops at the first empty one after 5).
//!
//! A pet's view: a row of a spacer (4 to 16000), its health bar (`PowerbarView_c` of `GFX_GUI_HOR_BAR_EMPTY`, 0xdb, filled
//! with `GFX_GUI_HOR_BAR_RED`, 0xdd, from the left) labelled with its name (FUN_1007d583: the name without control
//! characters in a font colour, FUN_1007d67c: 0x808080 while the pet is not in the playfield, white while it is the
//! selected target, 0xbbbbbb otherwise), a spacer and three buttons with tooltips: Attack ("ButtonPetAttack", `PetCommand`
//! 7), or Heal ("ButtonPetHeal", 12) for a pet of `NPCFamily` (stat 455) 0x60 (FUN_1007d29c); Follow (1) and Wait (4)
//! (the handlers at 0x1007cf02..0x1007cf59 call `N3Msg_SendPetCommand` with the pet). The buttons are shown with the
//! window's `ShowButtons` (`PetWindowConfig`, on at first) and enabled only while the pet is in the playfield
//! (FUN_1007d72b). The bar is the pet's health (stat 27) over its maximum (stat 1), empty while it is away (FUN_1007cf95).
//!
//! The name flashes red: `PetToMaster` (`0d381f02`, command 1 with a count, 0x0a in the captures, beside "Your pet is out of
//! range") reaches the pet's view through global signal +0x40 (Gamecode FUN_10011fa1; the view's FUN_1007d273 sets its
//! counter +0x140 to twice the count). The view's timer (500 ms, `EventTimer_c::Start` in FUN_1007d817) runs FUN_1007d72b,
//! which takes one off the counter while it is not below zero; while the counter is odd (and not -1) the name is
//! `#ff0000` (FUN_1007d67c) and the bar shows the other way (FUN_1007cf95: empty in the playfield, full away).
//!
//! **The programs running on a pet** (2026-11-18): under the bar row each `PetView_c` has an `NCUListView_c`
//! (FUN_10045d42 with the pet's identity, the constructor the NCU and target windows use): a grid in the smallest
//! icon size (`SetGridIconSize(3)`, 19 by 11), spacing 7, borders 6, sorted by the time left, filled from the same
//! per-character list the target windows read (`N3Msg_GetNanoTemplateInfoList(pet)`: here `World::nanos_on`), each
//! icon fading as its time runs. With no programs the list takes no room; with any it is one row (FUN_1007d0fa).
//! A pet that is not in the playfield has none. Shift+click on an icon shows the program's page in the original,
//! and so does the held right button's one entry; here any click does, as in the target windows. Not done: the
//! flash of an icon in its last ten seconds, the icons' tooltip.
//!
//! **A pet's place and name**: `PetTypeToLocation` (0x1007ed93) gives a pet of `PetType` (stat 512) 10 the place 1,
//! 12 the place 2, 11 the place 3 and 15 the place 6, unless its family (stat 455) is 0x5e; any other has none.
//! "Fixed Pet Buttons" (`EnableFixedPetButtons`) holds each such pet at its place. "Remember Pet Names"
//! (`AutoPetRenaming`, on at first): when a pet's name changes (the server's `SetNameIIR_t`, which `PetRenamed`
//! 0x1007ee41 hears) the name is kept for the pet's place (the original's `PetName<place>`, per character; here
//! `petnames.ini` beside `mao.ini`), and a pet that comes (`AddPet` 0x1007f0f9) with a place, a kept name and
//! another name of its own is renamed: `N3Msg_SendPetCommand(0, pet, 15, 0, name)`. The colour codes a name may
//! carry are kept as they come (the original stores them as font tags). No capture has a rename or a name change:
//! both messages are read from the code, and nothing is sent by itself until a name change has come and been kept.

use std::sync::Mutex;

use crate::theme::Palette;

/// A pet as the connection last saw it.
#[derive(Clone, Debug, PartialEq)]
pub struct Pet {
    pub id: (u32, u32),
    pub name: String,
    /// (now, max)
    pub health: (i64, i64),
    /// In the playfield (`N3Msg_isDynelInTree`).
    pub in_play: bool,
    /// The character's selected target.
    pub selected: bool,
    /// `NPCFamily` (stat 455), from its entity message.
    pub family: Option<u16>,
    /// `PetType` (stat 512), from its entity message.
    pub pet_type: Option<u16>,
    /// The view's flash counter (+0x140, -1 when not flashing: `flash_value`).
    pub flash: i32,
}

impl Default for Pet {
    fn default() -> Pet {
        Pet { id: (0, 0), name: String::new(), health: (0, 0), in_play: false, selected: false, family: None, pet_type: None, flash: -1 }
    }
}

/// The view's flash counter `elapsed` after a `PetToMaster` with `count`: twice the count, one less each 500 ms tick of the
/// view's timer (FUN_1007d72b), down to -1.
pub fn flash_value(count: u32, elapsed: std::time::Duration) -> i32 {
    let ticks = (elapsed.as_millis() / 500).min(i32::MAX as u128) as i32;
    (count.min(1 << 20) as i32 * 2 - ticks).max(-1)
}

/// Whether the name is red now (FUN_1007d67c: the counter odd and not -1).
fn flashing(pet: &Pet) -> bool {
    pet.flash & 1 != 0 && pet.flash != -1
}

/// The `PetCommand` numbers of the buttons (the schema's `PetCommand`).
pub const ATTACK: u32 = 7;
pub const HEAL: u32 = 12;
pub const FOLLOW: u32 = 1;
pub const WAIT: u32 = 4;

static PETS: Mutex<Vec<Pet>> = Mutex::new(Vec::new());
static COMMANDS: Mutex<Vec<((u32, u32), u32, String)>> = Mutex::new(Vec::new());
/// The programs running on each pet (by its instance), as the connection last worked them out.
static NANOS: Mutex<Vec<(u32, std::sync::Arc<Vec<crate::ncu::NanoInfo>>)>> = Mutex::new(Vec::new());
/// "Remember Pet Names" (`AutoPetRenaming`), as the settings have it: the connection asks it when a pet comes.
static REMEMBER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
/// The names kept by character and place (`PetName<place>`, the original's in the character's preferences):
/// `petnames.ini` in the character's folder.
static NAMES: Mutex<Option<std::collections::BTreeMap<(u32, u32), String>>> = Mutex::new(None);

pub fn set_nanos(nanos: Vec<(u32, std::sync::Arc<Vec<crate::ncu::NanoInfo>>)>) {
    *NANOS.lock().unwrap_or_else(|e| e.into_inner()) = nanos;
}

fn nanos_of(pet: u32) -> Option<std::sync::Arc<Vec<crate::ncu::NanoInfo>>> {
    NANOS.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|n| n.0 == pet).map(|n| n.1.clone())
}

pub fn set_remembering(on: bool) {
    REMEMBER.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn remembering() -> bool {
    REMEMBER.load(std::sync::atomic::Ordering::Relaxed)
}

/// `PetTypeToLocation` (0x1007ed93): the place a pet of this family and type has in the window and in the kept
/// names; 0 for one that has none.
pub fn place(family: Option<u16>, pet_type: Option<u16>) -> u32 {
    if family == Some(0x5e) {
        return 0;
    }
    match pet_type {
        Some(10) => 1,
        Some(11) => 3,
        Some(12) => 2,
        Some(15) => 6,
        _ => 0,
    }
}

fn names_path() -> std::path::PathBuf {
    crate::charprefs::file("petnames.ini")
}

fn names_kept() -> bool {
    !cfg!(test) && std::env::var_os("MAO_NO_LAYOUT").is_none() && std::env::var_os("MAO_UI_PREVIEW").is_none()
}

/// `petnames.ini`: a line a name, `<character> <place> <name>`.
pub fn parse_names(text: &str) -> std::collections::BTreeMap<(u32, u32), String> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.splitn(3, ' ');
            let key = (words.next()?.parse().ok()?, words.next()?.parse().ok()?);
            let name = words.next()?;
            (!name.is_empty()).then(|| (key, name.to_string()))
        })
        .collect()
}

pub fn render_names(names: &std::collections::BTreeMap<(u32, u32), String>) -> String {
    names.iter().map(|((character, place), name)| format!("{character} {place} {name}\n")).collect()
}

fn with_names<R>(f: impl FnOnce(&mut std::collections::BTreeMap<(u32, u32), String>) -> R) -> R {
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let names = names.get_or_insert_with(|| if names_kept() { std::fs::read_to_string(names_path()).map(|t| parse_names(&t)).unwrap_or_default() } else { Default::default() });
    f(names)
}

/// The character changed: its own names are read when next asked (`charprefs.rs`).
pub fn forget_names() {
    *NAMES.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// The name kept for a character's pet of that place.
pub fn remembered(character: u32, place: u32) -> Option<String> {
    with_names(|names| names.get(&(character, place)).cloned())
}

/// Keeps a pet's new name for its place (`PetRenamed`: whether or not the names are being given back).
pub fn remember(character: u32, place: u32, name: &str) {
    with_names(|names| {
        names.insert((character, place), name.replace(['\r', '\n'], " "));
        if names_kept() {
            let _ = std::fs::write(names_path(), render_names(names));
        }
    });
}

/// The pets, in the order of their places.
pub fn set(pets: Vec<Pet>) {
    *PETS.lock().unwrap_or_else(|e| e.into_inner()) = pets;
}

pub fn get() -> Vec<Pet> {
    PETS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The commands to one pet not sent yet: (pet, command, its text): the buttons', and a kept name given back
/// (command 15 with the name).
pub fn take_commands() -> Vec<((u32, u32), u32, String)> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn send(pet: (u32, u32), command: u32) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push((pet, command, String::new()));
}

/// The rename the window's module asks for a pet that came with another name than the one kept for its place.
pub fn send_rename(pet: (u32, u32), name: String) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push((pet, 15, name));
}

/// A text of text.mdb's category 10000 by its name (`LDBface::GetText(0x2710, name)`).
fn text(name: &str) -> String {
    crate::gui::xml::ldb_text(&format!("#10000:{}", mao_formats::ldb::elf_hash(name) as i32))
}

/// An XML attribute's text.
fn attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The first button's label, tooltip and command: Heal for a healing pet (FUN_1007d29c: `NPCFamily` 0x60), else Attack.
fn first_button(pet: &Pet) -> (&'static str, &'static str, u32) {
    if pet.family == Some(0x60) { ("ButtonPetHeal", "TooltipPetHeal", HEAL) } else { ("ButtonPetAttack", "TooltipPetAttack", ATTACK) }
}

/// The name's colour (FUN_1007d67c).
pub fn name_colour(pet: &Pet) -> &'static str {
    if flashing(pet) {
        return "#ff0000";
    }
    match (pet.in_play, pet.selected) {
        (false, _) => "#808080",
        (true, true) => "#ffffff",
        (true, false) => "#bbbbbb",
    }
}

/// The bar's fill (FUN_1007cf95): health over its maximum in the playfield, else empty; flashing, the other way round (empty
/// in the playfield, full away).
pub fn fill(pet: &Pet) -> f32 {
    let (now, max) = pet.health;
    match (pet.in_play, flashing(pet)) {
        (true, true) | (false, false) => 0.0,
        (false, true) => 1.0,
        (true, false) if max > 0 => now as f32 / max as f32,
        (true, false) => 0.0,
    }
}

/// The pet list's views: a `PetView_c` for each pet.
fn view_xml(pets: &[Pet]) -> String {
    let mut views = String::new();
    for (k, pet) in pets.iter().enumerate() {
        let (label, tip, _) = first_button(pet);
        let button = |name: &str, label: &str, tip: &str| format!("<Button name=\"{name}{k}\" label=\"{}\" tooltip=\"{}\" tooltip_body=\"{}\"/>", attr(&text(label)), attr(&text(label)), attr(&text(tip)));
        views += &format!(
            "<View name=\"pet{k}\" view_layout=\"vertical\">\
               <View view_layout=\"horizontal\">\
                 <HLayoutSpacer min_size=\"4\"/>\
                 <PowerBar name=\"hp{k}\" bg_gfx=\"GFX_GUI_HOR_BAR_EMPTY\" full_gfx=\"GFX_GUI_HOR_BAR_RED\"/>\
                 <HLayoutSpacer min_size=\"4\"/>{}{}{}\
               </View>\
             </View>",
            button("act", label, tip),
            button("follow", "ButtonPetFollow", "TooltipPetFollow"),
            button("wait", "ButtonPetWait", "TooltipPetWait"),
        );
    }
    format!("<View view_layout=\"vertical\">{views}</View>")
}

fn set_text(tree: &mut crate::gui::view::Tree, root: crate::gui::view::Id, name: &str, text: &str) {
    if let Some(id) = tree.find(root, name)
        && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id)
        && t.text != text
    {
        t.set_text(text);
        tree.relayout = true;
    }
}

fn show_view(tree: &mut crate::gui::view::Tree, root: crate::gui::view::Id, name: &str, on: bool, enabled: bool) {
    if let Some(v) = tree.find(root, name) {
        let want = u32::from(!on);
        if tree.get(v).hidden.min(1) != want {
            tree.get_mut(v).flags |= crate::gui::view::FLAG_COLLAPSE;
            tree.get_mut(v).hidden = want;
            tree.relayout = true;
        }
        tree.get_mut(v).enabled = enabled;
    }
}

pub struct PetWindow {
    pub open: bool,
    /// `ShowButtons` (`PetWindowConfig`): the buttons beside each bar.
    pub show_buttons: bool,
    /// The views and the pets (and their first buttons) they were made for: a change makes them again, as the list view
    /// does on each change of the pets (FUN_1007cc58).
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, Vec<((u32, u32), u32)>)>,
    /// Each pet's list of running programs (its view in the tree), in the pets' order.
    lists: Vec<crate::gui::view::Id>,
    textures: std::collections::HashMap<u32, egui::TextureHandle>,
    /// The page of a program clicked (`itemid://`), for the info window.
    pub info_url: Option<String>,
    /// The frame's icon menu while it is up.
    popup: Option<crate::gui::popup::PopupMenu>,
}

impl Default for PetWindow {
    fn default() -> PetWindow {
        PetWindow { open: false, show_buttons: true, views: None, lists: Vec::new(), textures: Default::default(), info_url: None, popup: None }
    }
}

/// The pets in the order the window shows them: as they came, or with "Fixed Pet Buttons" each kind of pet at its
/// own place (`PetTypeToLocation`: [`place`]), the pets without one after them, so a pet's buttons stay where they
/// were. (Until 2026-11-18 the family stood in for the type, which the entity message was thought not to give.)
pub fn ordered(pets: &[Pet], fixed: bool) -> Vec<Pet> {
    let mut out: Vec<Pet> = pets.to_vec();
    if fixed {
        out.sort_by_key(|p| (match place(p.family, p.pet_type) { 0 => u32::MAX, at => at }, p.id.1));
    }
    out
}

/// The height of a pet's list of programs: nothing without any, else the one row (the grid's cell of 11 between
/// borders of 6: `CellCountToPreferredSize`).
pub fn list_height(programs: usize) -> f32 {
    if programs == 0 { 0.0 } else { 23.0 }
}

impl PetWindow {
    /// Draws the window. `fixed` is the "Fixed Pet Buttons" option; the icon menu (the original's FUN_1007e75e: "Show
    /// buttons", "Fixed Pet Buttons", "Remember Pet Names") changes it, `show_buttons` and `remember`, and the call
    /// returns whether any changed.
    pub fn show(&mut self, ctx: &egui::Context, pets: &[Pet], palette: &Palette, fixed: &mut bool, remember: &mut bool) -> bool {
        if !self.open {
            return false;
        }
        let pets = ordered(pets, *fixed);
        let pets = pets.as_slice();
        let before = (self.show_buttons, *fixed, *remember);
        let made_for: Vec<((u32, u32), u32)> = pets.iter().map(|p| (p.id, first_button(p).2)).collect();
        if self.views.as_ref().is_none_or(|v| v.2 != made_for) {
            self.views = None;
            let doc = crate::gui::xml::parse(&view_xml(pets));
            let mut tree = crate::gui::view::Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            if let Some(root) = doc.first().and_then(|e| crate::gui::create(&mut tree, e)) {
                // (`SetLabel`: the name across the bar, a spacer each side, PowerbarView_c::SetLabel 0x1013f37c)
                for k in 0..pets.len() {
                    let Some(bar) = tree.find(root, &format!("hp{k}")) else { continue };
                    tree.get_mut(bar).layout = crate::gui::view::Layout::Horizontal;
                    for e in &crate::gui::xml::parse(&format!("<HLayoutSpacer/><TextView name=\"name{k}\" value=\"\"/><HLayoutSpacer/>")) {
                        if let Some(id) = crate::gui::create(&mut tree, e) {
                            tree.add_child(bar, id);
                        }
                    }
                }
                // (under each pet's row its `NCUListView_c`: FUN_1007d817 from 0x1007e06c on)
                self.lists.clear();
                for k in 0..pets.len() {
                    let Some(view) = tree.find(root, &format!("pet{k}")) else { continue };
                    let list = crate::gui::multilist::create_grid(&mut tree, "NCUListView", -1, 50);
                    crate::gui::multilist::set_grid(&mut tree, list, |g| {
                        g.set_icon_size(3);
                        g.spacing = (7.0, 7.0);
                    });
                    crate::gui::multilist::add_column(&mut tree, list, 12, &crate::gui::xml::ldb_named(10000, "MultiviewRemaining", "Remaining"), 55.0, 0xe);
                    crate::gui::multilist::set_sort(&mut tree, list, 12, 0);
                    tree.add_child(view, list);
                    self.lists.push(list);
                }
                self.views = Some((tree, root, made_for));
            }
        }
        let textures = &mut self.textures;
        let Some((tree, root, _)) = self.views.as_mut() else { return false };
        let root = *root;
        // (each pet's programs, the ones the list shows (FUN_100458b0), by the time left; none for a pet that is away)
        let now = std::time::Instant::now();
        let mut any = false;
        for (k, pet) in pets.iter().enumerate() {
            let Some(&list) = self.lists.get(k) else { continue };
            let nanos = nanos_of(pet.id.1).filter(|_| pet.in_play);
            let mut order: Vec<&crate::ncu::NanoInfo> = nanos.iter().flat_map(|n| n.iter()).filter(|n| !n.columns.hidden).collect();
            order.sort_by_key(|n| n.left(now));
            any |= !order.is_empty();
            let rows = crate::ncu::nano_rows(ctx, textures, &order, now);
            if crate::gui::multilist::items(tree, list) != rows.as_slice() {
                crate::gui::multilist::set_items(tree, list, rows);
                tree.relayout = true;
            }
            crate::gui::multilist::update(tree, list);
            // (no programs: the list takes no room; any: one row, FUN_1007d0fa)
            let height = list_height(order.len());
            let hidden = u32::from(order.is_empty());
            let node = tree.get_mut(list);
            if node.hidden.min(1) != hidden || node.min_size.y != height {
                node.flags |= crate::gui::view::FLAG_COLLAPSE;
                node.hidden = hidden;
                (node.min_size.y, node.max_limit.y) = (height, height);
                tree.relayout = true;
            }
        }
        if any {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        for (k, pet) in pets.iter().enumerate() {
            let name: String = pet.name.chars().filter(|&c| c >= ' ').collect();
            set_text(tree, root, &format!("name{k}"), &format!("<font color={}>{name}</font>", name_colour(pet)));
            if let Some(id) = tree.find(root, &format!("hp{k}")) {
                crate::gui::powerbar::set_value(tree, id, fill(pet));
            }
            for b in ["act", "follow", "wait"] {
                show_view(tree, root, &format!("{b}{k}"), self.show_buttons, pet.in_play);
            }
        }
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let title = text("PetWindow");
        crate::chrome::window(ctx, palette, "PetWindow", &title, &mut open, |w| w.default_pos(crate::layout::pos("PetWindow", ctx.content_rect().center() - egui::vec2(200.0, 100.0))).default_size(egui::vec2(400.0, 200.0)), |ui| {
            crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
        });
        // (the frame's icon menu, made each time it opens (FUN_1007e9d7 hands FUN_1007e75e to `GetIconMenu`): three
        // ticked entries for `ShowButtons`, `EnableFixedPetButtons` and `AutoPetRenaming`; the frame's "Close Window" after
        // them as on the client's other windows with a menu of their own)
        if let Some(at) = crate::chrome::frame_menu("PetWindow") {
            self.popup = Some(crate::chrome::menu_marked(ctx, at, &[("Show buttons".to_string(), true, Some(self.show_buttons)), ("Fixed Pet Buttons".to_string(), true, Some(*fixed)), ("Remember Pet Names".to_string(), true, Some(*remember)), (crate::chrome::close_entry(), true, None)]));
        }
        match crate::chrome::menu(ctx, palette, "pet-window-menu", &mut self.popup) {
            Some(0) => self.show_buttons = !self.show_buttons,
            Some(1) => *fixed = !*fixed,
            Some(2) => *remember = !*remember,
            Some(3) => crate::chrome::close_next("PetWindow"),
            _ => {}
        }
        // (a program's icon clicked: its page)
        for (_, event) in crate::gui::multilist::take_events(tree) {
            if let crate::gui::multilist::ItemEvent::Clicked(id, _) = event {
                self.info_url = Some(format!("itemid://{}/{}", crate::gui::iteminfo::NANO_IDENTITY, id));
            }
        }
        for (id, signal) in std::mem::take(&mut tree.signals) {
            if signal != crate::gui::host::Signal::Clicked {
                continue;
            }
            let name = tree.get(id).name.clone();
            let Some((k, command)) = [("act", None), ("follow", Some(FOLLOW)), ("wait", Some(WAIT))].iter().find_map(|(b, c)| Some((name.strip_prefix(b)?.parse::<usize>().ok()?, *c))) else { continue };
            if let Some(pet) = pets.get(k).filter(|p| p.in_play) {
                send(pet.id, command.unwrap_or(first_button(pet).2));
            }
        }
        self.open = open;
        (self.show_buttons, *fixed, *remember) != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_buttons_and_fill_as_the_original() {
        let mut pet = Pet { id: (0xc350, 0x7b1e6d64), name: "Ravening M-60".into(), health: (100, 400), in_play: true, selected: false, family: Some(0x5f), pet_type: Some(12), flash: -1 };
        assert_eq!((name_colour(&pet), fill(&pet), first_button(&pet).2), ("#bbbbbb", 0.25, ATTACK));
        pet.selected = true;
        assert_eq!(name_colour(&pet), "#ffffff");
        pet.family = Some(0x60);
        assert_eq!(first_button(&pet).2, HEAL);
        pet.in_play = false;
        assert_eq!((name_colour(&pet), fill(&pet)), ("#808080", 0.0));
    }

    /// A pet's place by its type (`PetTypeToLocation`), which the fixed order follows; the kept names by character
    /// and place; the list of programs takes room only when there are any.
    #[test]
    fn a_pet_has_its_place_and_its_kept_name() {
        assert_eq!((place(Some(0x5f), Some(10)), place(Some(0x5f), Some(12)), place(Some(0x5f), Some(11)), place(Some(0x60), Some(15))), (1, 2, 3, 6));
        assert_eq!((place(Some(0x5e), Some(10)), place(Some(0x5f), Some(0)), place(None, None)), (0, 0, 0));
        let pet = |id: u32, pet_type: u16| Pet { id: (0xc350, id), pet_type: Some(pet_type), family: Some(0x5f), ..Pet::default() };
        let pets = [pet(3, 0), pet(1, 12), pet(2, 10)];
        assert_eq!(ordered(&pets, true).iter().map(|p| p.id.1).collect::<Vec<_>>(), [2, 1, 3]);
        assert_eq!(ordered(&pets, false).iter().map(|p| p.id.1).collect::<Vec<_>>(), [3, 1, 2]);
        remember(0x77aa01, 2, "Rex the second");
        remember(0x77aa01, 1, "Bolt");
        assert_eq!((remembered(0x77aa01, 2).as_deref(), remembered(0x77aa01, 3), remembered(0x77aa02, 2)), (Some("Rex the second"), None, None));
        let names: std::collections::BTreeMap<(u32, u32), String> = [((7, 1), "A pet".to_string()), ((7, 6), "Other".to_string())].into_iter().collect();
        assert_eq!(parse_names(&render_names(&names)), names);
        assert!(parse_names("7 x name\n7 1\njunk\n").is_empty());
        assert_eq!((list_height(0), list_height(1), list_height(9)), (0.0, 23.0, 23.0));
    }

    /// A count of 10 flashes the name ten times over ten seconds: red on each odd counter value.
    #[test]
    fn the_name_flashes_red_count_times() {
        use std::time::Duration;
        let ms = |n: u64| Duration::from_millis(n);
        assert_eq!((flash_value(10, ms(0)), flash_value(10, ms(500)), flash_value(10, ms(9_999)), flash_value(10, ms(60_000))), (20, 19, 1, -1));
        let reds = (0..25).filter(|k| flashing(&Pet { flash: flash_value(10, ms(k * 500)), ..Pet::default() })).count();
        assert_eq!(reds, 10);
        let mut pet = Pet { health: (100, 400), in_play: true, flash: 19, ..Pet::default() };
        assert_eq!((name_colour(&pet), fill(&pet)), ("#ff0000", 0.0));
        pet.in_play = false;
        assert_eq!((name_colour(&pet), fill(&pet)), ("#ff0000", 1.0));
        pet.flash = 18;
        assert_eq!((name_colour(&pet), fill(&pet)), ("#808080", 0.0));
    }
}
