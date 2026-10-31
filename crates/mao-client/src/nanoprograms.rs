//! The nano programs window: the programs the character has (the world-entry message's `UploadedNanoIds`), each with its icon and
//! name, searchable. A click casts the program at the target (or at oneself without one: the captured casts, `n3::cast_nano`);
//! a program can be dragged onto a hotbar. Pointing at one shows its description and what it changes.

use std::collections::HashMap;
use std::sync::Arc;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};

use crate::gear::Icon;
use crate::hotbar;
use crate::theme::Palette;

/// The database type of nano programs.
const NANO: u32 = 0xfde85;

pub struct Program {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// The NCU memory it takes (its quality stat).
    pub cost: u32,
    /// What it changes in the stats (`derived::nano_modifiers`).
    pub effects: Vec<(u32, i64)>,
    /// Its school's grid ([`school`]).
    pub school: usize,
}

/// The programs with their names, icons and effects from the database, by name.
pub fn describe(db: &ResourceDatabase, textures: &crate::Textures, ids: &[u32], level: i64) -> Vec<Program> {
    let mut icons: HashMap<u32, Option<Arc<Icon>>> = HashMap::new();
    let mut list: Vec<Program> = ids
        .iter()
        .map(|&id| {
            let data = db.get(Id { kind: NANO, instance: id }).ok().flatten();
            let template = data.as_deref().and_then(Item::parse);
            let icon = template.as_ref().and_then(|t| t.icon()).and_then(|icon_id| {
                icons
                    .entry(icon_id)
                    .or_insert_with(|| textures.get(item::ICON_TYPE, icon_id).map(|t| Arc::new(Icon { id: icon_id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })))
                    .clone()
            });
            Program {
                id,
                name: template.as_ref().map_or_else(|| format!("Nano #{id}"), |t| t.name.clone()),
                description: template.as_ref().map(|t| t.description.clone()).unwrap_or_default(),
                icon,
                cost: template.as_ref().and_then(|t| t.stat(54)).filter(|&c| c < 999).unwrap_or(0),
                school: school(template.as_ref().and_then(|t| t.stat(405))),
                effects: data.as_deref().map(|d| item::nano_modifiers_at(d, level)).unwrap_or_default().into_iter().map(|(s, a)| (s, a as i64)).collect(),
            }
        })
        .collect();
    list.sort_by_key(|p| p.name.to_lowercase());
    list
}

/// A program's school's grid (`NanoView_c` FUN_100d48a9: the template's stat 405 less one, 0 to 6, else 5).
pub fn school(stat: Option<u32>) -> usize {
    match stat.map(|s| s as i64 - 1) {
        Some(s) if (0..=6).contains(&s) => s as usize,
        _ => 5,
    }
}

/// The school buttons in their order (`SchoolSelector_c` FUN_100d6f3d): the button's school and its text.mdb 10000 label
/// (Misc, Combat, Medical, Prot, Psi, Space).
const SCHOOLS: [(usize, &str); 6] = [(5, "#10000:344211"), (0, "#10000:175102436"), (1, "#10000:251981772"), (2, "#10000:23966852"), (3, "#10000:236378921"), (4, "#10000:114743685")];

/// The nano programs window, GUI.dll's `NanoView_c` (FUN_100d5328): the `SchoolSelector_c` (a toggle button per school, in
/// rows as wide as the window: FUN_100d6051) over a tab of item grids, one per school, in the smallest square icon size
/// (`SetGridIconSize(0)`: 31 by 31 on `SLOT_32_CLOSED`), sorted by name; the school shown at first is Misc (5:
/// `selected_school`'s default). A click casts the program at the target (FUN_100d44cd: `N3Msg_CastNanoSpell`; with
/// `UseNewMouseFunc` on, a left click on a list that does not select is the use signal); a right click (the original's menu,
/// FUN_100d49a0) shows its page here; a program dragged out goes onto a hotbar.
#[derive(Default)]
pub struct ProgramsWindow {
    pub open: bool,
    textures: HashMap<u32, egui::TextureHandle>,
    /// The info view pages asked for.
    pub infos: Vec<String>,
    school: Option<usize>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id, Vec<(crate::gui::view::Id, usize)>)>,
}

impl ProgramsWindow {
    fn build(ctx: &egui::Context) -> (crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id, Vec<(crate::gui::view::Id, usize)>) {
        use crate::gui::view::{Layout, Node, Tree};
        let mut tree = Tree::default();
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
        tree.get_mut(root).layout = Layout::Vertical;
        // (the school buttons, 3 in from the edges)
        let selector = tree.add(Node::new("SchoolSelector", Box::new(crate::gui::view::Plain)));
        tree.get_mut(selector).layout = Layout::Horizontal;
        tree.get_mut(selector).borders = crate::gui::geom::Rect::new(3.0, 3.0, 3.0, 3.0);
        let mut buttons = Vec::new();
        for (school, label) in SCHOOLS {
            let mut b = crate::gui::controls::Button::new(&crate::gui::xml::ldb_text(label));
            b.toggle = true;
            let mut node = Node::new(&format!("school{school}"), Box::new(b));
            node.borders = crate::gui::geom::Rect::new(0.0, 0.0, 3.0, 0.0);
            let id = tree.add(node);
            tree.add_child(selector, id);
            buttons.push((id, school));
        }
        tree.add_child(root, selector);
        let list = crate::gui::multilist::create_grid(&mut tree, "NanoListView", -1, 6);
        crate::gui::multilist::set_grid(&mut tree, list, |g| g.set_icon_size(0));
        tree.get_mut(list).borders = crate::gui::geom::Rect::new(3.0, 3.0, 3.0, 3.0);
        tree.add_child(root, list);
        (tree, root, list, buttons)
    }

    /// Shows the window; returns the program to cast (a click).
    pub fn show(&mut self, ctx: &egui::Context, programs: &[Program], palette: &Palette) -> Option<hotbar::Action> {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut cast = None;
        if self.views.is_none() {
            self.views = Some(Self::build(ctx));
        }
        let school = *self.school.get_or_insert(5);
        let textures = &mut self.textures;
        let (tree, root, list, buttons) = self.views.as_mut()?;
        let (root, list) = (*root, *list);
        // (the school's button down, the others up; the school's programs in its grid)
        for &(b, s) in buttons.iter() {
            if let Some(button) = tree.control_mut::<crate::gui::controls::Button>(b) {
                button.on = s == school;
            }
        }
        let rows: Vec<crate::gui::multilist::Item> = programs
            .iter()
            .filter(|p| p.school == school)
            .map(|p| crate::gui::multilist::Item {
                id: p.id as i64,
                icon: p.icon.as_ref().map(|icon| textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("program-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id()),
                cells: vec![(1, crate::gui::multilist::CellView::Text(p.name.clone()))],
                keys: vec![(1, p.name.to_lowercase())],
                ..Default::default()
            })
            .collect();
        if crate::gui::multilist::items(tree, list).iter().map(|i| i.id).ne(rows.iter().map(|i| i.id)) {
            crate::gui::multilist::set_items(tree, list, rows);
        }
        crate::gui::multilist::update(tree, list);
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut hovered = None;
        let title = crate::gui::xml::ldb_text("#10000:346191");
        crate::chrome::window(ctx, palette, "Nano programs", &title, &mut open, |w| w.default_pos(crate::layout::pos("Nano programs", egui::pos2(220.0, 90.0))).default_size([300.0, 300.0]).resizable(true), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                hovered = crate::gui::multilist::item_at(tree, list, response.rect.min, at).map(|id| id as u32);
            }
        });
        let program = |id: i64| programs.iter().find(|p| p.id as i64 == id);
        let action = |p: &Program| hotbar::Action::Nano { id: p.id, name: p.name.clone(), icon: p.icon.as_ref().map(|i| i.id) };
        for (_, event) in crate::gui::multilist::take_events(tree) {
            match event {
                crate::gui::multilist::ItemEvent::Clicked(id, clicks) => {
                    // (a click casts: the list is not selectable, so FUN_1004141b's left click is the use signal -> FUN_100d44cd)
                    let _ = clicks;
                    if let Some(p) = program(id) {
                        cast = Some(action(p));
                    }
                }
                // (dragged out: onto a hotbar)
                crate::gui::multilist::ItemEvent::Dragged(id) => {
                    if let Some(p) = program(id) {
                        egui::DragAndDrop::set_payload(ctx, action(p));
                    }
                }
            }
        }
        // (a school's button pressed shows its grid)
        for (id, signal) in std::mem::take(&mut tree.signals) {
            if matches!(signal, crate::gui::host::Signal::Clicked | crate::gui::host::Signal::Toggled(_))
                && let Some(&(_, s)) = buttons.iter().find(|b| b.0 == id)
            {
                self.school = Some(s);
            }
        }
        if let Some(id) = hovered {
            if ctx.input(|i| i.pointer.secondary_clicked()) {
                self.infos.push(format!("itemid://{}/{}", crate::gui::iteminfo::NANO_IDENTITY, id));
            } else if let Some(p) = program(id as i64) {
                let at = ctx.input(|i| i.pointer.hover_pos()).unwrap_or_default();
                egui::Area::new(egui::Id::unique("program-tip")).order(egui::Order::Tooltip).fixed_pos(at + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_max_width(320.0);
                        ui.label(egui::RichText::new(&p.name).strong().color(accent));
                        if p.cost > 0 {
                            ui.label(egui::RichText::new(format!("NCU {}", p.cost)).small());
                        }
                        if !p.description.is_empty() {
                            ui.label(p.description.chars().take(420).collect::<String>().replace("\r\n", "\n"));
                        }
                        for &(stat, amount) in p.effects.iter().filter(|e| e.1 != 0).take(10) {
                            ui.horizontal(|ui| {
                                ui.label(crate::skills::stat_name(stat));
                                let color = if amount > 0 { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(230, 110, 110) };
                                ui.label(egui::RichText::new(format!("{amount:+}")).color(color));
                            });
                        }
                    });
                });
            }
        }
        self.open = open;
        cast
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schools_are_the_stat_less_one() {
        assert_eq!(school(Some(1)), 0);
        assert_eq!(school(Some(7)), 6);
        assert_eq!(school(Some(0)), 5);
        assert_eq!(school(None), 5);
    }
}
