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
//! Left out: the icon list under each pet (a `MultiListView_c` of its running nano programs with their time remaining,
//! FUN_10045d42: the client does not know a pet's programs' times); the icon menu's "Show buttons", "Fixed Pet Buttons" (pets by their
//! `PetType`, stat 512, `PetTypeToLocation`) and "Remember Pet Names" (`/pet rename` on summoning).

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
    /// The view's flash counter (+0x140, -1 when not flashing: `flash_value`).
    pub flash: i32,
}

impl Default for Pet {
    fn default() -> Pet {
        Pet { id: (0, 0), name: String::new(), health: (0, 0), in_play: false, selected: false, family: None, flash: -1 }
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
static COMMANDS: Mutex<Vec<((u32, u32), u32)>> = Mutex::new(Vec::new());

/// The pets, in the order of their places.
pub fn set(pets: Vec<Pet>) {
    *PETS.lock().unwrap_or_else(|e| e.into_inner()) = pets;
}

pub fn get() -> Vec<Pet> {
    PETS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The buttons' commands not sent yet: (pet, command).
pub fn take_commands() -> Vec<((u32, u32), u32)> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn send(pet: (u32, u32), command: u32) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push((pet, command));
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
}

impl Default for PetWindow {
    fn default() -> PetWindow {
        PetWindow { open: false, show_buttons: true, views: None }
    }
}

impl PetWindow {
    pub fn show(&mut self, ctx: &egui::Context, pets: &[Pet], palette: &Palette) {
        if !self.open {
            return;
        }
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
                self.views = Some((tree, root, made_for));
            }
        }
        let Some((tree, root, _)) = self.views.as_mut() else { return };
        let root = *root;
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_buttons_and_fill_as_the_original() {
        let mut pet = Pet { id: (0xc350, 0x7b1e6d64), name: "Ravening M-60".into(), health: (100, 400), in_play: true, selected: false, family: Some(0x5f), flash: -1 };
        assert_eq!((name_colour(&pet), fill(&pet), first_button(&pet).2), ("#bbbbbb", 0.25, ATTACK));
        pet.selected = true;
        assert_eq!(name_colour(&pet), "#ffffff");
        pet.family = Some(0x60);
        assert_eq!(first_button(&pet).2, HEAL);
        pet.in_play = false;
        assert_eq!((name_colour(&pet), fill(&pet)), ("#808080", 0.0));
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
