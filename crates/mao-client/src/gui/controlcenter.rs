//! The control center as GUI.dll builds it (FUN_1006f911, `control_center_window`): the skin's `Views/ControlCenter.xml`
//! over the whole screen, its views shown by their `activate_criteria` (FUN_1006f697) against the distributed values
//! (`dvalue.rs`), and its `CCMenu` views — `ControlMenu_c`s of the skin's menu scripts (`menu.rs`) — as rows of
//! buttons whose `<Menu>` entries open menus above them.
//!
//! The docks the program fills itself (FUN_1006f911: `LeftWingDock`, `RightWingDock`, `LeftBarDock`, `RightBarDock`,
//! `LeftTargetCtrlDock`, `RightTargetCtrlDock`, `RollupControllerDock`) take the size of what goes in them (the skin's
//! wing, strip, target button and rollup pictures), are laid out with the rest, and are handed to `vitals.rs`
//! ([`ControlCenter::docks`]), which draws those parts there; the rollup folds the wings by `cc_left_wing` and
//! `cc_right_wing`. A menu entry's value is a window's open flag where the client has that window ([`WINDOWS`]): the
//! client's window state is written into the values before the frame, and the values the menus changed are handed back
//! ([`Output::windows`]).
//!
//! The buttons are the skin's `GFX_GUI_CONTROLCENTER_BUTTON01` (up), `02` (under the pointer) and `03` (pressed, or a
//! toggle that is on) strips, the label's markup drawn by the text view (`TextView_c`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::controls::{Colour, TextView};
use super::dvalue::Store;
use super::font::{FontId, Measure};
use super::host::{Env, Paint};
use super::menu::{Action, Entry, Mode};
use super::variant::Variant;
use super::view::{Control, Id, Node, Tree};
use super::xml::Element;
use crate::theme::Palette;

/// The windows the menus name and the client's names for them (`Ui::set_window`); `options` is the options window.
pub const WINDOWS: &[(&str, &str)] = &[
    ("inventory_window", "inventory"),
    ("wear_window", "inventory"),
    ("skill_window", "skills"),
    ("perk_window", "perks"),
    ("stat_window", "stats"),
    ("nano_window", "programs"),
    ("ncu_window", "ncu"),
    ("mission_window", "missions"),
    ("friends_window", "friends"),
    ("faction_window", "factions"),
    ("pet_window", "pets"),
    ("tradeskill_window", "tradeskill"),
    ("map_window", "planetmap"),
    ("planetmap_window", "planetmap"),
    ("specialaction_window", "actions"),
    ("mail_window", "mail"),
    ("optionpanel_window", "optionpanel"),
    ("itemshop_window", "itemshop"),
    ("petition_window", "petition"),
    ("dailylogin_window", "dailyrewards"),
];

/// A `CCMenu` view in the tree: its script's entries (loaded once the skin's folders are known) and where each was last
/// drawn.
pub struct CCMenuView {
    pub script: String,
    pub criteria: String,
    pub entries: Vec<Entry>,
    /// Each entry's button size (from the skin, once loaded), and whether its criteria held this frame.
    pub sizes: Vec<egui::Vec2>,
    pub shown: Vec<bool>,
}

/// A button's size: its `bgicon` picture's, else its label's text with 8 pixels each side and the strip's height.
fn button_size(m: &Measure, skin: Option<&crate::skin::Skin>, e: &Entry) -> egui::Vec2 {
    if let Some(s) = skin.filter(|_| !e.icon.is_empty()).and_then(|s| s.get(&e.icon)) {
        return s.size;
    }
    let height = skin.and_then(|s| s.get("GFX_GUI_CONTROLCENTER_BUTTON01_MIDDLE")).map_or(20.0, |s| s.size.y);
    let view = TextView::new(&e.label);
    let w = view.preferred(m, false).map_or(40.0, |p| p.x + 1.0);
    egui::vec2(w + 16.0, height)
}

/// The gap between a row's icon buttons.
const ICON_GAP: f32 = 1.0;

/// `MiniToolbar_c` (the XML `CCMiniToolbar`, FUN_10072ced): toggle buttons (`ToolbarButton_c`, FUN_1007316d) for these
/// windows' values, with their tips and pictures (off; on, also under the pointer: `SetGfx` 0, 1 and 2), side by side
/// from the left (FUN_10072b22: as wide as all of them, as tall as the tallest). A button shows its value
/// (FUN_10072aaf) and a click flips it (FUN_10072a63).
pub const MINI_TOOLBAR: [(&str, &str, &str, &str); 10] = [
    ("wear_window", "Wear", "GFX_GUI_SB_WEAR_OFF", "GFX_GUI_SB_WEAR"),
    ("specialaction_window", "Controls", "GFX_GUI_SB_ACTION_OFF", "GFX_GUI_SB_ACTION"),
    ("knowledge_window", "Knowledge", "GFX_GUI_SB_REF_OFF", "GFX_GUI_SB_REF"),
    ("mission_window", "Mission", "GFX_GUI_SB_MISSION_OFF", "GFX_GUI_SB_MISSION"),
    ("team_view", "Team", "GFX_GUI_SB_TEAM_OFF", "GFX_GUI_SB_TEAM"),
    ("map_window", "Map", "GFX_GUI_SB_MAP_OFF", "GFX_GUI_SB_MAP"),
    ("friends_window", "Friends", "GFX_GUI_SB_FRIENDS_OFF", "GFX_GUI_SB_FRIENDS"),
    ("nano_window", "Programs", "GFX_GUI_SB_NANO_OFF", "GFX_GUI_SB_NANO"),
    ("stat_window", "Stats", "GFX_GUI_SB_STAT_OFF", "GFX_GUI_SB_STAT"),
    ("ncu_window", "NCU", "GFX_GUI_SB_NCU_OFF", "GFX_GUI_SB_NCU"),
];

/// The `CCMiniToolbar` view: its buttons' sizes (from the skin, once loaded).
#[derive(Default)]
pub struct MiniToolbar {
    pub sizes: Vec<egui::Vec2>,
}

impl Control for MiniToolbar {
    fn class(&self) -> &'static str {
        "CCMiniToolbar"
    }
    fn preferred(&self, _m: &Measure, _max: bool) -> Option<super::geom::Point> {
        let w: f32 = self.sizes.iter().map(|s| s.x).sum();
        let h = self.sizes.iter().map(|s| s.y - 1.0).fold(-1.0, f32::max);
        Some(super::geom::Point::new(w - 1.0, h))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl CCMenuView {
    pub fn create(tree: &mut Tree, e: &Element) -> Id {
        let criteria = e.attribute("criteria").unwrap_or("").to_string();
        let view = CCMenuView { script: e.attribute("script").unwrap_or("").to_string(), criteria: criteria.clone(), entries: Vec::new(), sizes: Vec::new(), shown: Vec::new() };
        let mut node = Node::from_element(e, 4, Box::new(view));
        // (its own criteria join its activation's, FUN_10065300's "(%s) && (%s)")
        if !criteria.trim().is_empty() {
            node.criteria = if node.criteria.trim().is_empty() { criteria } else { format!("({}) && ({criteria})", node.criteria) };
        }
        tree.add(node)
    }
}

impl Control for CCMenuView {
    fn class(&self) -> &'static str {
        "CCMenu"
    }
    fn preferred(&self, _m: &Measure, _max: bool) -> Option<super::geom::Point> {
        let shown = || self.sizes.iter().zip(&self.entries).zip(&self.shown).filter(|(_, on)| **on).map(|(se, _)| se);
        let w: f32 = shown().map(|(s, e)| s.x + if e.icon.is_empty() { 0.0 } else { ICON_GAP }).sum();
        let h = shown().map(|(s, _)| s.y).fold(0.0, f32::max);
        Some(super::geom::Point::new((w - 1.0).max(0.0), (h - 1.0).max(0.0)))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// An open menu: its entries, where it opened from, and the entry index path that opened it.
struct Popup {
    entries: Vec<Entry>,
    anchor: egui::Rect,
    /// Opens upward (from a button near the screen's bottom) or to the right of a menu entry.
    beside: bool,
    owner: Entry,
}

#[derive(Default)]
pub struct Output {
    /// Windows to show or hide (the client's names).
    pub windows: Vec<(&'static str, bool)>,
    /// Chat commands to run.
    pub commands: Vec<String>,
}

/// Where the player's own values are kept (beside the client's settings, as the program's `Prefs.xml`).
pub fn prefs_path() -> PathBuf {
    crate::settings::Settings::path().with_file_name("prefs.xml")
}

/// GUI.dll's sound settings (FUN_100c573c's observers, FUN_100c5242 on): the volumes and the switches (off mutes) given
/// to the sound module (SandyInterface).
fn apply_sound(store: &Store, name: &str) {
    let v = store.get(name).cloned().unwrap_or_default();
    let (f, on) = (v.as_float(), v.as_bool());
    crate::audio::with(|m| match name {
        "MasterVolume" => m.set_master_volume(f),
        "SoundFXOn" => m.set_master_fx_mute(!on),
        "FXVolume" => m.set_master_fx_volume(f),
        "MusicOn" => m.set_master_music_mute(!on),
        "MusicVolume" => m.set_master_music_volume(f),
        "VoiceSndFxOn" => m.set_master_player_fx_mute(!on),
        "VoiceSndFxVolume" => m.set_master_player_fx_volume(f),
        // (`SoundOnOff`: everything)
        "SoundOnOff" => m.set_master_mute(!on),
        _ => {}
    });
}

const SOUND: [&str; 8] = ["SoundOnOff", "MasterVolume", "SoundFXOn", "FXVolume", "MusicOn", "MusicVolume", "VoiceSndFxOn", "VoiceSndFxVolume"];

/// The docks and the pictures that size them.
const DOCKS: [(&str, &str); 7] = [
    ("LeftWingDock", "GFX_GUI_CONTROLCENTER_WING_LEFT"),
    ("RightWingDock", "GFX_GUI_CONTROLCENTER_WING_RIGHT"),
    ("LeftBarDock", "GFX_GUI_CONTROLCENTER_BOTTOM_LEFT"),
    ("RightBarDock", "GFX_GUI_CONTROLCENTER_BOTTOM_RIGHT"),
    ("LeftTargetCtrlDock", "GFX_GUI_CONTROLCENTER_TARGET_BUTTON"),
    ("RightTargetCtrlDock", "GFX_GUI_CONTROLCENTER_TARGET_BUTTON"),
    ("RollupControllerDock", "GFX_GUI_ROLLUP_COLLAPSE_STATE1"),
];

pub struct ControlCenter {
    pub store: Store,
    /// Where the docks were laid out in the last frame.
    pub docks: crate::vitals::Docks,
    dirs: Vec<PathBuf>,
    tree: Tree,
    root: Id,
    popups: Vec<Popup>,
}

impl ControlCenter {
    /// The skin's control center, its values and menus.
    pub fn load(ctx: &egui::Context) -> Option<ControlCenter> {
        let skin = crate::skin::skin(ctx)?;
        let dirs: Vec<PathBuf> = skin.dirs().into_iter().map(|d| d.to_path_buf()).collect();
        let game_default = crate::theme::game_dir().map(|g| g.join("cd_image/gui/Default"));
        let own = dirs.first().filter(|d| Some(*d) != game_default.as_ref()).cloned();
        let mut store = Store::load(game_default.as_deref(), own.as_deref());
        store.load_user(&prefs_path());
        // (MAO_DVALUES=name=value;...: values set for a test, e.g. `cc_mini_toolbar=true`)
        if let Ok(list) = std::env::var("MAO_DVALUES") {
            for (name, value) in list.split(';').filter_map(|kv| kv.split_once('=')) {
                let v = match value.trim() {
                    "true" => Variant::from(true),
                    "false" => Variant::from(false),
                    t => t.parse::<f64>().map_or_else(|_| Variant::String(t.as_bytes().to_vec()), Variant::from),
                };
                store.set(name.trim(), v);
            }
            store.take_changed();
        }
        for (name, v) in store.numbers() {
            crate::prefs::set(&name, v);
        }
        for name in SOUND {
            if store.get(name).is_some() {
                apply_sound(&store, name);
            }
        }
        let (mut tree, root) = super::load_view(&dirs, "ControlCenter.xml", "")?;
        let measure = Measure { ctx: Some(ctx.clone()) };
        // (each dock the size of what goes in it)
        for (dock, picture) in DOCKS {
            if let (Some(id), Some(size)) = (tree.find(root, dock), skin.get(picture).map(|s| s.size)) {
                let p = super::geom::Point::new(size.x - 1.0, size.y - 1.0);
                // (the rollup's dock holds the `RollupArea` DockArea_c windows dock into, which takes the height to spare:
                // it pushes the right wing down to the screen's bottom)
                let max = if dock == "RollupControllerDock" { super::geom::Point::new(p.x, 16000.0) } else { p };
                tree.get_mut(id).control = Box::new(super::view::Spacer { min: p, max });
            }
        }
        for n in &mut tree.nodes {
            if n.control.class() == "CCMiniToolbar" {
                let sizes = MINI_TOOLBAR.iter().map(|(_, _, off, _)| skin.get(off).map_or(egui::vec2(20.0, 20.0), |s| s.size)).collect();
                n.control = Box::new(MiniToolbar { sizes });
                continue;
            }
            if let Some(m) = n.control.as_any_mut().downcast_mut::<CCMenuView>() {
                m.entries = super::menu::load_script(&dirs, &m.script);
                m.sizes = m.entries.iter().map(|e| button_size(&measure, Some(&skin), e)).collect();
                m.shown = vec![true; m.entries.len()];
            }
        }
        eprintln!("control center: {} views from {}", tree.nodes.len(), dirs.first().map_or(String::new(), |d| d.display().to_string()));
        Some(ControlCenter { store, docks: Default::default(), dirs, tree, root, popups: Vec::new() })
    }

    fn holds(&self, criteria: &str, stats: &BTreeMap<u32, i64>) -> bool {
        self.store.criteria(criteria, stats, &|_| None)
    }

    /// Runs an entry's actions for an event.
    fn run(&mut self, entry: &Entry, event: &str, stats: &BTreeMap<u32, i64>, out: &mut Output) {
        for block in entry.actions_for(event) {
            if !self.holds(&block.criteria, stats) {
                continue;
            }
            for a in &block.actions {
                match a {
                    Action::SetValue { name, value } => {
                        let v = if value.trim().is_empty() { Some(Variant::from(true)) } else { self.store.eval(value, stats, &|_| None) };
                        if let Some(v) = v {
                            self.store.set(name, v);
                        }
                    }
                    Action::CloseMenu => self.popups.clear(),
                    Action::RunChatScript(c) => out.commands.push(c.clone()),
                }
            }
        }
    }

    /// A click on an entry (`CCMenuEntry_c`'s invoke).
    fn invoke(&mut self, entry: &Entry, anchor: egui::Rect, beside: bool, depth: usize, stats: &BTreeMap<u32, i64>, out: &mut Output) {
        if let Some(sub) = &entry.sub {
            // (a menu entry: its menu opens, or closes when it was open)
            let was_open = self.popups.get(depth).is_some_and(|p| p.owner.label == entry.label && p.owner.name == entry.name);
            for p in self.popups.drain(depth..).collect::<Vec<_>>() {
                self.run(&p.owner, "on_deactivate", stats, out);
            }
            if !was_open {
                let entries = super::menu::sub_entries(&self.dirs, sub);
                self.run(entry, "on_activate", stats, out);
                self.popups.push(Popup { entries, anchor, beside, owner: entry.clone() });
            }
            return;
        }
        if entry.actions_for("on_invoke").next().is_some() {
            self.run(entry, "on_invoke", stats, out);
        } else if !entry.name.is_empty() {
            let on = self.store.get_bool(&entry.name);
            self.store.set(&entry.name, Variant::from(!on));
        }
    }

    /// Shows the control center for a frame; `open` tells which of the client's windows are open.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, stats: &BTreeMap<u32, i64>, open: &dyn Fn(&str) -> bool) -> Output {
        let mut out = Output::default();
        // (the client's windows into the values, without counting as changes)
        for (dv, name) in WINDOWS {
            self.store.set(dv, Variant::from(open(name)));
        }
        self.store.take_changed();
        let store = &self.store;
        self.tree.apply_criteria(&|c| store.criteria(c, stats, &|_| None));
        // (the entries whose criteria hold, which size their rows)
        let mut resized = false;
        for n in &mut self.tree.nodes {
            if let Some(m) = n.control.as_any_mut().downcast_mut::<CCMenuView>() {
                let shown: Vec<bool> = m.entries.iter().map(|e| store.criteria(&e.criteria, stats, &|_| None)).collect();
                resized |= shown != m.shown;
                m.shown = shown;
            }
        }
        if resized {
            self.tree.clear_cache();
        }
        let screen = ctx.content_rect();
        self.tree.measure = Measure { ctx: Some(ctx.clone()) };
        super::host::fit(&mut self.tree, self.root, screen.size());
        let skin = crate::skin::skin(ctx);
        let measure = self.tree.measure.clone();
        let env = Env { skin: skin.as_deref(), palette, measure: &measure };

        // the menu rows
        let menus: Vec<Id> = (0..self.tree.nodes.len()).filter(|&i| self.tree.get(i).control.as_any().is::<CCMenuView>() && self.tree.is_visible(i)).collect();
        let mut clicked: Option<(Entry, egui::Rect, bool, usize)> = None;
        for id in menus {
            let rect = super::host::screen_rect(&self.tree, id, screen.min);
            let Some(view) = self.tree.get(id).control.as_any().downcast_ref::<CCMenuView>() else { continue };
            let entries: Vec<(Entry, egui::Vec2)> = view.entries.iter().zip(&view.sizes).zip(&view.shown).filter(|(_, on)| **on).map(|((e, s), _)| (e.clone(), *s)).collect();
            egui::Area::new(egui::Id::unique(("cc-menu", id))).fixed_pos(rect.min).order(egui::Order::Foreground).show(ctx, |ui| {
                let mut x = rect.left();
                for (e, size) in &entries {
                    let r = egui::Rect::from_min_size(egui::pos2(x, rect.bottom() - size.y), *size);
                    if let Some(c) = self.button(ui, &env, e, r) {
                        clicked = Some((e.clone(), c, false, 0));
                    }
                    x += size.x + if e.icon.is_empty() { 0.0 } else { ICON_GAP };
                }
            });
        }

        // the mini toolbars
        let toolbars: Vec<Id> = (0..self.tree.nodes.len()).filter(|&i| self.tree.get(i).control.as_any().is::<MiniToolbar>() && self.tree.is_visible(i)).collect();
        let mut flipped = Vec::new();
        for id in toolbars {
            let rect = super::host::screen_rect(&self.tree, id, screen.min);
            let Some(bar) = self.tree.get(id).control.as_any().downcast_ref::<MiniToolbar>() else { continue };
            let sizes = bar.sizes.clone();
            egui::Area::new(egui::Id::unique(("cc-mini-toolbar", id))).fixed_pos(rect.min).order(egui::Order::Foreground).show(ctx, |ui| {
                let mut x = rect.left();
                for ((dv, tip_text, off, on), size) in MINI_TOOLBAR.iter().zip(&sizes) {
                    let r = egui::Rect::from_min_size(egui::pos2(x, rect.top()), *size);
                    x += size.x;
                    let response = ui.interact(r, ui.scope_id().with(("mini", *dv)), egui::Sense::click());
                    let value = self.store.get_bool(dv);
                    let picture = if value || response.hovered() || response.is_pointer_button_down_on() { on } else { off };
                    if let Some(skin) = env.skin {
                        let colour = if value { Colour::SELECTED } else { Colour::DEFAULT };
                        skin.draw_in(ui.painter(), picture, r, colour.rgb(palette));
                    }
                    super::tooltip::hover(ui.ctx(), &env, response.id.with("tip"), response.hovered().then_some(response.id), tip_text, "");
                    if response.clicked() {
                        flipped.push((*dv, !value));
                    }
                }
            });
        }
        for (dv, v) in flipped {
            self.store.set(dv, Variant::from(v));
        }

        // the open menus
        let mut depth = 0;
        while depth < self.popups.len() {
            let p = &self.popups[depth];
            let entries: Vec<Entry> = p.entries.iter().filter(|e| self.store.criteria(&e.criteria, stats, &|_| None)).cloned().collect();
            let height = skin.as_ref().and_then(|s| s.get("GFX_GUI_CONTROLCENTER_BUTTON01_MIDDLE")).map_or(20.0, |s| s.size.y);
            // (a menu's entries are text buttons, its widest label's width)
            let width = entries.iter().map(|e| button_size(&measure, None, &Entry { icon: String::new(), ..e.clone() }).x).fold(60.0, f32::max);
            let total = height * entries.len() as f32;
            // (a menu from a button opens above it; one from a menu entry beside it)
            let at = if p.beside { egui::pos2(p.anchor.right(), (p.anchor.bottom() - total).max(screen.top())) } else { egui::pos2(p.anchor.left(), (p.anchor.top() - total).max(screen.top())) };
            egui::Area::new(egui::Id::unique(("cc-popup", depth))).fixed_pos(at).order(egui::Order::Foreground).show(ctx, |ui| {
                ui.painter().rect_filled(egui::Rect::from_min_size(at, egui::vec2(width, total)), 0.0, egui::Color32::from_black_alpha(170));
                for (i, e) in entries.iter().enumerate() {
                    let r = egui::Rect::from_min_size(at + egui::vec2(0.0, i as f32 * height), egui::vec2(width, height));
                    if let Some(c) = self.button(ui, &env, e, r) {
                        clicked = Some((e.clone(), c, true, depth + 1));
                    }
                }
            });
            depth += 1;
        }

        // (a click elsewhere closes the menus)
        if clicked.is_none() && !self.popups.is_empty() && ctx.input(|i| i.pointer.primary_clicked()) && !ctx.is_pointer_over_egui() {
            for p in std::mem::take(&mut self.popups) {
                self.run(&p.owner, "on_deactivate", stats, &mut out);
            }
        }
        if let Some((entry, rect, beside, depth)) = clicked {
            self.invoke(&entry, rect, beside, depth, stats, &mut out);
        }

        // the docks, for `vitals.rs`
        let dock = |name: &str| self.tree.find(self.root, name).filter(|&id| self.tree.is_visible(id)).map(|id| super::host::screen_rect(&self.tree, id, screen.min));
        let on = |name: &str| self.store.get_bool("cc_section1") && self.store.get_bool(name);
        self.docks = crate::vitals::Docks {
            left_wing: dock("LeftWingDock"),
            right_wing: dock("RightWingDock"),
            left_bar: dock("LeftBarDock"),
            right_bar: dock("RightBarDock"),
            rollup: dock("RollupControllerDock"),
            left_target: dock("LeftTargetCtrlDock"),
            right_target: dock("RightTargetCtrlDock"),
            compass: on("cc_compass"),
            health: on("cc_health_bar"),
            nano: on("cc_nano_bar"),
            xp: on("cc_xp_bar"),
            alien_xp: on("cc_alienxp_bar"),
            rolled: !self.store.get_bool("cc_left_wing") && !self.store.get_bool("cc_right_wing"),
        };

        // the values the menus and the option panel changed: the client's windows, the sound, the player's file
        let mut save = false;
        for name in self.store.take_changed() {
            if let Some((_, w)) = WINDOWS.iter().find(|(dv, _)| *dv == name) {
                out.windows.push((w, self.store.get_bool(&name)));
            } else {
                save = true;
            }
            if SOUND.contains(&name.as_str()) {
                apply_sound(&self.store, &name);
            }
            if let Some(v) = self.store.get(&name).filter(|v| !matches!(v, Variant::String(_) | Variant::Void)) {
                crate::prefs::set(&name, v.as_double());
            }
        }
        if save {
            self.store.save_user(&prefs_path());
        }
        out
    }

    /// The rollup's click: the wings fold away, or come back.
    pub fn toggle_rollup(&mut self) {
        let rolled = self.docks.rolled;
        self.store.set("cc_left_wing", Variant::from(rolled));
        self.store.set("cc_right_wing", Variant::from(rolled));
        self.store.take_changed();
    }

    /// One entry's button: the strip for its state and its label; the rect when it was clicked.
    fn button(&self, ui: &mut egui::Ui, env: &Env, e: &Entry, r: egui::Rect) -> Option<egui::Rect> {
        let response = ui.interact(r, ui.scope_id().with((&e.name, &e.label, r.min.x as i32, r.min.y as i32)), egui::Sense::click());
        let on = e.mode == Mode::Toggle && self.store.get_bool(&e.name);
        // (an icon button: its picture, brighter under the pointer or while on)
        if let Some(skin) = env.skin.filter(|s| !e.icon.is_empty() && s.has(&e.icon)) {
            let tint = if on || response.is_pointer_button_down_on() { egui::Color32::WHITE } else if response.hovered() { egui::Color32::from_gray(235) } else { egui::Color32::from_gray(190) };
            skin.draw_in(ui.painter(), &e.icon, r, tint);
            tip(ui, env, &response, e);
            return response.clicked().then_some(r);
        }
        let strip = if response.is_pointer_button_down_on() || on {
            "GFX_GUI_CONTROLCENTER_BUTTON03"
        } else if response.hovered() {
            "GFX_GUI_CONTROLCENTER_BUTTON02"
        } else {
            "GFX_GUI_CONTROLCENTER_BUTTON01"
        };
        let painter = ui.painter();
        if !env.skin.is_some_and(|s| s.three_slice(painter, strip, r, egui::Color32::WHITE)) {
            painter.rect_filled(r.shrink(1.0), 2.0, if response.hovered() { egui::Color32::from_gray(70) } else { egui::Color32::from_gray(40) });
        }
        let mut label = TextView::new(&e.label);
        label.colour = Colour::WHITE;
        label.font = FontId::NORMAL;
        let text = label.preferred(env.measure, false).unwrap_or_default();
        let text_rect = egui::Rect::from_min_size(egui::pos2(r.center().x - (text.x + 1.0) / 2.0, r.center().y - (text.y + 1.0) / 2.0), egui::vec2(text.x + 1.0, text.y + 1.0));
        label.paint(&Paint { painter, env, rect: text_rect, hovered: false, pressed: false, enabled: true });
        tip(ui, env, &response, e);
        response.clicked().then_some(r)
    }
}

/// An entry's tip while the pointer rests on its button (`tooltip`, `tooltip_body`; the window controller's tip).
fn tip(ui: &egui::Ui, env: &Env, response: &egui::Response, e: &Entry) {
    let source = response.id.with("tip");
    super::tooltip::hover(ui.ctx(), env, source, response.hovered().then_some(response.id), &e.tooltip, &e.tooltip_body);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bundled AOU skins' control centers: their menu rows show (the values from the game's Default and the skin's
    /// LoginPrefs) and hold their scripts' buttons. Needs `MAO_GAME` for the Default values.
    #[test]
    fn the_aou_control_center_has_its_menus() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let Some(bundled) = crate::theme::bundled_gui_dir() else { return };
        let default = std::path::Path::new(&game).join("cd_image/gui/Default");
        for skin in ["AOU_CYT", "AOU_Translucent", "AOU_Slim_CNI", "AOU_Slim_CNT"] {
            let dir = bundled.join(skin);
            let dirs = vec![dir.clone(), default.clone()];
            let store = Store::load(Some(&default), Some(&dir));
            let (mut tree, root) = crate::gui::load_view(&dirs, "ControlCenter.xml", "").unwrap();
            for n in &mut tree.nodes {
                if let Some(m) = n.control.as_any_mut().downcast_mut::<CCMenuView>() {
                    m.entries = crate::gui::menu::load_script(&dirs, &m.script);
                    m.sizes = m.entries.iter().map(|e| button_size(&Measure::default(), None, e)).collect();
                    m.shown = vec![true; m.entries.len()];
                }
            }
            let stats = BTreeMap::from([(54u32, 50i64), (389, 0xff)]);
            tree.apply_criteria(&|c| store.criteria(c, &stats, &|_| None));
            crate::gui::host::fit(&mut tree, root, egui::vec2(1280.0, 800.0));
            let menus: Vec<(String, bool, usize, egui::Rect)> = (0..tree.nodes.len())
                .filter_map(|i| tree.get(i).control.as_any().downcast_ref::<CCMenuView>().map(|m| (m.script.clone(), tree.is_visible(i), m.entries.len(), crate::gui::host::screen_rect(&tree, i, egui::pos2(0.0, 0.0)))))
                .collect();
            println!("{skin}: {menus:?}");
            assert!(menus.iter().any(|(_, shown, n, r)| *shown && *n > 0 && r.width() > 10.0), "{skin}");
        }
    }
}
