//! The Action window: everything the character can do, as the original's grid of icon tiles. The catalogue
//! (`actions_data.rs`) was read from the running original client: the special actions (weapon attacks, the basic
//! character actions, perks) and the social actions with the dances among them (see `docs/GAME_MECHANICS.md`).
//! A tile is clicked to do it or dragged onto a hotbar slot.

use std::collections::HashMap;
use std::sync::Arc;

use crate::actions_data::ACTIONS;
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

pub struct ActionsWindow {
    pub open: bool,
    /// Icons by name, once loaded.
    icons: HashMap<&'static str, Arc<Icon>>,
    loaded: bool,
    /// What a click asked for: sent to the network thread by the viewer.
    pub clicked: Option<hotbar::Action>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id)>,
}

impl Default for ActionsWindow {
    fn default() -> Self {
        ActionsWindow { open: false, icons: HashMap::new(), loaded: false, clicked: None, views: None }
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

    /// The window, GUI.dll's `SpecialActionView_c` (FUN_100dc9b1): "Actions", a grid of the special actions in the
    /// smallest square icon size (`SetGridIconSize(0)`: 31 by 31 on `SLOT_32`), in the order the list holds them. A click
    /// performs the action (FUN_100dc033: `N3Msg_PerformSpecialAction`; the list does not select, so a left click is the use
    /// signal); one dragged out goes onto a hotbar. The social actions are the control center's Social menu
    /// (`ActionMenu/SocialMenu.xml`), as in the original.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, palette: &Palette) {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        if self.views.is_none() {
            use crate::gui::view::{Layout, Node, Tree};
            let mut tree = Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
            tree.get_mut(root).layout = Layout::Base;
            let list = crate::gui::multilist::create_grid(&mut tree, "SpecialActionView", -1, 5);
            crate::gui::multilist::set_grid(&mut tree, list, |g| g.set_icon_size(0));
            tree.add_child(root, list);
            self.views = Some((tree, root, list));
        }
        let actions: Vec<(hotbar::Action, &'static str, Option<&'static str>, &'static str)> = ACTIONS
            .iter()
            .map(|&(id, kind, name, _, _)| {
                let icon = icon_name(kind, name);
                (hotbar::Action::Special { id, name: name.to_string(), icon: self.icons.get(icon).map(|i| i.id) }, name, Some(icon), if kind == Kind::Weapon { "Weapon attack" } else if kind == Kind::Perk { "Perk action" } else { "Character action" })
            })
            .collect();
        let rows: Vec<crate::gui::multilist::Item> = actions
            .iter()
            .enumerate()
            .map(|(k, (_, name, icon, _))| crate::gui::multilist::Item {
                id: k as i64,
                icon: icon.and_then(|n| self.icons.get(n)).map(|i| gear.texture(ctx, i)),
                cells: vec![(1, crate::gui::multilist::CellView::Text(name.to_string()))],
                keys: vec![(1, name.to_string())],
                ..Default::default()
            })
            .collect();
        let Some((tree, root, list)) = self.views.as_mut() else { return };
        let (root, list) = (*root, *list);
        if crate::gui::multilist::items(tree, list) != rows.as_slice() {
            crate::gui::multilist::set_items(tree, list, rows);
        }
        crate::gui::multilist::update(tree, list);
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut hovered = None;
        crate::chrome::window(ctx, palette, "Actions", "Actions", &mut open, |w| w.default_pos(crate::layout::pos("Actions", egui::pos2(520.0, 70.0))).default_size(egui::vec2(5.0 * 42.0 + 30.0, 4.0 * 42.0 + 12.0)).resizable(true), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                hovered = crate::gui::multilist::item_at(tree, list, response.rect.min, at).map(|k| (k as usize, at));
            }
        });
        for (_, event) in crate::gui::multilist::take_events(tree) {
            match event {
                crate::gui::multilist::ItemEvent::Clicked(k, _) => {
                    if let Some((a, ..)) = actions.get(k as usize) {
                        self.clicked = Some(a.clone());
                    }
                }
                crate::gui::multilist::ItemEvent::Dragged(k) => {
                    if let Some((a, ..)) = actions.get(k as usize) {
                        egui::DragAndDrop::set_payload(ctx, a.clone());
                    }
                }
            }
        }
        tree.signals.clear();
        if let Some((k, at)) = hovered
            && let Some((_, name, _, what)) = actions.get(k)
        {
            egui::Area::new(egui::Id::unique("action-tip")).order(egui::Order::Tooltip).fixed_pos(at + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(egui::RichText::new(*name).strong().color(accent));
                    ui.label(egui::RichText::new(*what).small().weak());
                });
            });
        }
        self.open = open;
    }
}
