//! The Action window: everything the character can do, as the original's grid of icon tiles. The catalogue
//! (`actions_data.rs`) was read from the running original client: the special actions (weapon attacks, the basic
//! character actions, perks) and the social actions with the dances among them (see `docs/GAME_MECHANICS.md`).
//! A tile is clicked to do it or dragged onto a hotbar slot.

use std::collections::HashMap;
use std::sync::Arc;

use crate::actions_data::{ACTIONS, SOCIAL};
use crate::gear::{GearWindow, Icon};
use crate::hotbar;
use crate::theme::Palette;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Weapon,
    Character,
    Perk,
}

/// The icon (a texture name of type 0xf6958) of an action, by what it is.
pub fn icon_name(kind: Kind, name: &str) -> &'static str {
    match name {
        "Burst" => "action_burst.png",
        "Fling Shot" => "action_fling.png",
        "Full Auto" => "action_auto.png",
        "Brawl" => "action_brawl.png",
        "Dimach" => "action_dimach.png",
        "Reload" => "action_aimed.png",
        "Use" => "action_use.png",
        "Pick-Up" => "action_get.png",
        "Start Combat" => "action_startattack.png",
        "Stand" => "action_stand.png",
        "Walk" => "action_walk.png",
        "Sneak" => "action_sneak.png",
        "Suspended Animation" => "action_camp.png",
        "Crawl" => "action_crawl.png",
        "Search" => "action_search.png",
        _ if kind == Kind::Perk => perk_icon(name),
        _ => "action_use.png",
    }
}

/// A perk's icon, by the kind of thing it does (the game has a family of icons per effect; which one a perk
/// uses is in data this client does not read, so this picks the family from the name).
fn perk_icon(name: &str) -> &'static str {
    let n = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    if has(&["heal", "bandage", "blessing", "lifeblood", "life", "reconditioned", "bearhug"]) {
        "perk_heals_1a.png"
    } else if has(&["bash", "grip", "grasp", "violence", "guardian"]) {
        "perk_dd_hammer_1a.png"
    } else if has(&["blood", "artery", "jugular", "vein", "draw"]) {
        "perk_dot_1a.png"
    } else if has(&["target", "paint", "triangulate", "tracer", "acquired"]) {
        "perk_damagebuffs_1a.png"
    } else if has(&["armor", "absorption", "stone", "encase", "fuzz", "fuse"]) {
        "perk_acbuffs_1a.png"
    } else if has(&["shot", "burst", "ammunition", "fire", "napalm", "muzzle", "clip", "horde", "frenzy", "double", "initiative", "spray", "detonate"]) {
        "perk_dd_gun_1a.png"
    } else {
        "perk_procs_1a.png"
    }
}

/// The window's tabs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Combat,
    Perks,
    Social,
}

pub struct ActionsWindow {
    pub open: bool,
    tab: Tab,
    /// Icons by name, once loaded.
    icons: HashMap<&'static str, Arc<Icon>>,
    loaded: bool,
    /// What a click asked for: sent to the network thread by the viewer.
    pub clicked: Option<hotbar::Action>,
}

impl Default for ActionsWindow {
    fn default() -> Self {
        ActionsWindow { open: false, tab: Tab::Combat, icons: HashMap::new(), loaded: false, clicked: None }
    }
}

impl ActionsWindow {
    /// Reads the icons the tiles use from the game's database (once).
    pub fn load_icons(&mut self, textures: &crate::Textures) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        for &(_, kind, name, _, _) in ACTIONS {
            let icon = icon_name(kind, name);
            if !self.icons.contains_key(icon) {
                if let Some(i) = textures.icon_named(icon) {
                    self.icons.insert(icon, i);
                }
            }
        }
    }

    /// The icon behind a hotbar entry's `icon` number (an index into the action icons).
    pub fn icon_by_instance(&self, instance: u32) -> Option<&Arc<Icon>> {
        self.icons.values().find(|i| i.id == instance)
    }

    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, palette: &Palette) {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut open = self.open;
        crate::chrome::window(ctx, palette, "Actions", "Actions", &mut open, |w| w.resizable(false).default_pos(crate::layout::pos("Actions", egui::pos2(520.0, 70.0))), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Combat, "Combat");
                ui.selectable_value(&mut self.tab, Tab::Perks, "Perks");
                ui.selectable_value(&mut self.tab, Tab::Social, "Social");
            });
            ui.label(egui::RichText::new("Click to do it, drag onto a hotbar slot").small().weak());
            ui.separator();
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                let mut tiles: Vec<(String, Option<&'static str>, hotbar::Action, Option<String>)> = Vec::new();
                match self.tab {
                    Tab::Combat => {
                        for &(id, kind, name, _, _) in ACTIONS.iter().filter(|a| a.1 != Kind::Perk) {
                            let icon = icon_name(kind, name);
                            tiles.push((name.to_string(), Some(icon), hotbar::Action::Special { id, name: name.to_string(), icon: self.icons.get(icon).map(|i| i.id) }, Some(format!("{}", if kind == Kind::Weapon { "Weapon attack" } else { "Character action" }))));
                        }
                    }
                    Tab::Perks => {
                        for &(id, kind, name, hash, _) in ACTIONS.iter().filter(|a| a.1 == Kind::Perk) {
                            let icon = icon_name(kind, name);
                            tiles.push((name.to_string(), Some(icon), hotbar::Action::Special { id, name: name.to_string(), icon: self.icons.get(icon).map(|i| i.id) }, Some(format!("Perk {hash}"))));
                        }
                    }
                    Tab::Social => {
                        for &(id, name, dance) in SOCIAL {
                            tiles.push((name.to_string(), None, hotbar::Action::Social { id, name: name.to_string() }, Some(if dance { "Dance".to_string() } else { "Social action".to_string() })));
                        }
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    // Four tiles across, as the original's.
                    ui.set_max_width(4.0 * 46.0 + 3.0 * 4.0);
                    for (name, icon, action, subtitle) in tiles {
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::click_and_drag());
                        let painter = ui.painter();
                        painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(140));
                        let border = if response.hovered() { accent } else { egui::Color32::from_white_alpha(50) };
                        match icon.and_then(|n| self.icons.get(n)) {
                            Some(i) => {
                                let id = gear.texture(ctx, i);
                                ui.painter().image(id, rect.shrink(3.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                            None => {
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, name.chars().take(2).collect::<String>().to_uppercase(), egui::FontId::proportional(14.0), accent);
                            }
                        }
                        ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(if response.hovered() { 2.0 } else { 1.0 }, border), egui::StrokeKind::Inside);
                        response.dnd_set_drag_payload(action.clone());
                        if response.clicked() {
                            self.clicked = Some(action);
                        }
                        response.on_hover_ui(|ui| {
                            ui.label(egui::RichText::new(&name).strong());
                            if let Some(s) = subtitle {
                                ui.label(egui::RichText::new(s).small().weak());
                            }
                        });
                    }
                });
            });
        });
        self.open = open;
    }
}
