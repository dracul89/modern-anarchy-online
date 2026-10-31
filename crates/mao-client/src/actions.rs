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

/// A perk action of the character's (the server's 0xb4 list): its item template (the special action's id), name, icon, what
/// it does, and when its lock (the template's lock-skill spell 0xcfc3: the group in its stat 0) is free again.
#[derive(Clone)]
pub struct PerkAction {
    pub template: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    pub lock: Option<u32>,
    /// When the lock is free again, and how long it runs in all (seconds).
    pub free_at: Option<(std::time::Instant, f32)>,
}

/// The perk actions described from their templates, with the locks running.
pub fn describe_perks(db: &mao_formats::resource::ResourceDatabase, textures: &crate::Textures, list: &[(u32, u32, [u8; 4])], locks: &std::collections::BTreeMap<u32, (std::time::Instant, f32)>) -> Vec<PerkAction> {
    use mao_formats::item::{self, Item};
    list.iter()
        .map(|&(template, number, code)| {
            let data = db.get(mao_formats::resource::Id { kind: item::TEMPLATE, instance: template }).ok().flatten();
            let parsed = data.as_deref().and_then(Item::parse);
            let icon = parsed.as_ref().and_then(|t| t.icon()).and_then(|id| textures.get(item::ICON_TYPE, id).map(|t| Arc::new(Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })));
            let lock = data.as_deref().and_then(|d| mao_formats::spell::lists(d).into_iter().flat_map(|(_, s)| s).find(|s| s.kind == 0xcfc3).map(|s| s.stat(0) as u32));
            PerkAction {
                template,
                name: parsed.as_ref().map_or_else(|| format!("{} ({number:#x})", String::from_utf8_lossy(&code)), |t| t.name.clone()),
                description: parsed.map(|t| t.description).unwrap_or_default(),
                icon,
                lock,
                free_at: lock.and_then(|l| locks.get(&l).copied()),
            }
        })
        .collect()
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
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, perks: &[PerkAction], palette: &Palette) {
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
        // (the weapon and character actions from the catalogue; the perk actions the server listed for this character, or the
        // catalogue's until it has: `N3Msg_GetSpecialActionList`)
        let now = std::time::Instant::now();
        let mut actions: Vec<(hotbar::Action, String, Option<Arc<Icon>>, String, Option<(f32, f32)>)> = ACTIONS
            .iter()
            .filter(|a| a.1 != Kind::Perk || perks.is_empty())
            .map(|&(id, kind, name, _, _)| {
                let icon = self.icons.get(icon_name(kind, name)).cloned();
                (hotbar::Action::Special { id, name: name.to_string(), icon: icon.as_ref().map(|i| i.id) }, name.to_string(), icon, (if kind == Kind::Weapon { "Weapon attack" } else if kind == Kind::Perk { "Perk action" } else { "Character action" }).to_string(), None)
            })
            .collect();
        for p in perks {
            let left = p.free_at.map(|(t, total)| (t.saturating_duration_since(now).as_secs_f32(), total)).filter(|&(s, _)| s > 0.0);
            actions.push((hotbar::Action::Special { id: p.template, name: p.name.clone(), icon: None }, p.name.clone(), p.icon.clone(), if p.description.is_empty() { "Perk action".to_string() } else { p.description.clone() }, left));
        }
        if actions.iter().any(|a| a.4.is_some()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        let rows: Vec<crate::gui::multilist::Item> = actions
            .iter()
            .enumerate()
            .map(|(k, (_, name, icon, _, left))| crate::gui::multilist::Item {
                id: k as i64,
                icon: icon.as_ref().map(|i| gear.texture(ctx, i)),
                cells: vec![(1, crate::gui::multilist::CellView::Text(name.clone()))],
                keys: vec![(1, name.clone())],
                // (a locked perk fades out until its lock is free, as the NCU's running nanos do)
                left: left.map(|(s, total)| 1.0 - s / total.max(1.0)),
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
                    // (a locked perk: "Feedback_ActionIsNotAvailable" in the original; nothing is sent)
                    if let Some((a, _, _, _, None)) = actions.get(k as usize) {
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
            && let Some((_, name, _, what, left)) = actions.get(k)
        {
            egui::Area::new(egui::Id::unique("action-tip")).order(egui::Order::Tooltip).fixed_pos(at + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(320.0);
                    ui.label(egui::RichText::new(name.as_str()).strong().color(accent));
                    if let Some(s) = left {
                        ui.label(egui::RichText::new(format!("Available in {:.0} s", s.0.ceil())).small());
                    }
                    ui.label(egui::RichText::new(what.as_str()).small().weak());
                });
            });
        }
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MAO_GAME`: the perk action the 2026-10-31 capture lists as `0x374d7`, number `0x2850`, code `QUBS`, is Quick Bash, whose
    /// lock (the capture's `PerkUnavailable` after using it: lock 0xc9 for 25 s) is group 201.
    #[test]
    fn a_listed_perk_action_is_described_from_its_template() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        let textures = crate::Textures::new(db.clone());
        let now = std::time::Instant::now();
        let locks = [(201, (now + std::time::Duration::from_secs(25), 25.0))].into_iter().collect();
        let perks = describe_perks(&db, &textures, &[(0x374d7, 0x2850, *b"QUBS")], &locks);
        assert_eq!(perks[0].name, "Quick Bash");
        assert_eq!(perks[0].lock, Some(201));
        assert!(perks[0].free_at.is_some());
    }
}
