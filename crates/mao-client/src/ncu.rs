//! The NCU window: the nano programs running on the character (what the server lists in the character's own `SimpleCharFullUpdate`:
//! the nano's id and the time it has left), each with its icon, name and a bar for the time left, and the NCU memory in use against
//! the maximum. Pointing at a nano shows what it does (its description and the stat changes it makes).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};

use crate::gear::Icon;
use crate::theme::Palette;

/// The database type of nano programs; the item `stat` of a nano's duration in milliseconds.
const NANO: u32 = 0xfde85;
const DURATION: u32 = 8;

/// One running nano, ready to show.
pub struct NanoInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    /// The time left when the server told us, the nano's whole duration (both in milliseconds) and when we were told.
    pub remaining_ms: u32,
    pub total_ms: u32,
    pub at: Instant,
    /// What the nano changes in the stats (`derived::nano_modifiers`), for the tooltip.
    pub effects: Vec<(u32, i64)>,
    /// The NCU memory the nano takes: the nano's own quality stat (the sum over the running nanos was the original's current NCU for two
    /// characters, exactly); the free nano programs carry 999 there and take none.
    pub cost: u32,
}

impl NanoInfo {
    /// The time left now, in milliseconds.
    pub fn left(&self, now: Instant) -> u32 {
        self.remaining_ms.saturating_sub(now.saturating_duration_since(self.at).as_millis().min(u32::MAX as u128) as u32)
    }
}

/// The running nanos `(id, time left in milliseconds)` with their names, icons and effects from the database.
pub fn describe(db: &ResourceDatabase, textures: &crate::Textures, running: &[(u32, u32)], level: i64, at: Instant) -> Vec<NanoInfo> {
    let mut icons: HashMap<u32, Option<Arc<Icon>>> = HashMap::new();
    running
        .iter()
        .map(|&(id, remaining_ms)| {
            let data = db.get(Id { kind: NANO, instance: id }).ok().flatten();
            let template = data.as_deref().and_then(Item::parse);
            let icon = template.as_ref().and_then(|t| t.icon()).and_then(|icon_id| {
                icons
                    .entry(icon_id)
                    .or_insert_with(|| textures.get(item::ICON_TYPE, icon_id).map(|t| Arc::new(Icon { id: icon_id, width: t.width as usize, height: t.height as usize, rgba: t.rgba })))
                    .clone()
            });
            let effects = data.as_deref().map(|d| item::nano_modifiers_at(d, level)).unwrap_or_default().into_iter().map(|(s, a)| (s, a as i64)).collect();
            NanoInfo {
                id,
                name: template.as_ref().map_or_else(|| format!("Nano #{id}"), |t| t.name.clone()),
                description: template.as_ref().map(|t| t.description.clone()).unwrap_or_default(),
                icon,
                remaining_ms,
                total_ms: template.as_ref().and_then(|t| t.stat(DURATION)).unwrap_or(0),
                at,
                effects,
                cost: template.as_ref().and_then(|t| t.stat(54)).filter(|&c| c < 999).unwrap_or(0),
            }
        })
        .collect()
}

/// `mm:ss` (or `h:mm:ss`) of a time in milliseconds.
pub fn clock(ms: u32) -> String {
    let seconds = ms / 1000;
    if seconds >= 3600 {
        format!("{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

/// The NCU window, GUI.dll's `NCUView_c` (FUN_100d774b): an `NCUListView_c` (FUN_10045d42) grid of the running nanos
/// (`N3Msg_GetNanoTemplateInfoList`) in the small icon size (`SetGridIconSize(2)`: 39 by 23 on `SLOT_40X24`), packed in the
/// order of their time left (`FUN_10041258(0xc)`: the grid sorted by column 12, auto arranged), no horizontal scroll bar,
/// 160 by 160 to start; each icon fades from the right as its time runs out (`ItemIconView_c` FUN_1003f9ea). The title is
/// text.mdb 10000's "NCU (%u/%u)" with NCU in use and the most (stats 180 and 181: FUN_100d7689). A click shows the nano's
/// page (FUN_10045259: `itemid://53019/<id>`); the client's own right-click menu cancels it.
pub struct NcuWindow {
    pub open: bool,
    textures: HashMap<u32, egui::TextureHandle>,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id)>,
    /// The info page asked for (for the info view).
    pub info_url: Option<String>,
    /// The program whose menu is up, and the menu (`PopupMenu_c`'s look).
    menu: Option<(u32, egui::Pos2)>,
    popup: Option<crate::gui::popup::PopupMenu>,
}

impl Default for NcuWindow {
    fn default() -> NcuWindow {
        NcuWindow { open: false, textures: HashMap::new(), views: None, info_url: None, menu: None, popup: None }
    }
}

impl NcuWindow {
    fn build(ctx: &egui::Context) -> (crate::gui::view::Tree, crate::gui::view::Id, crate::gui::view::Id) {
        use crate::gui::view::{Layout, Node, Tree};
        let mut tree = Tree::default();
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
        tree.get_mut(root).layout = Layout::Base;
        let list = crate::gui::multilist::create_grid(&mut tree, "NCUListView", -1, 3);
        crate::gui::multilist::set_grid(&mut tree, list, |g| g.set_icon_size(2));
        tree.add_child(root, list);
        (tree, root, list)
    }

    /// Shows the window; returns the program the player chose to cancel (the client's right-click menu).
    pub fn show(&mut self, ctx: &egui::Context, nanos: &[NanoInfo], stats: &std::collections::BTreeMap<u32, i64>, palette: &Palette) -> Option<u32> {
        let mut cancel = None;
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let now = Instant::now();
        if self.views.is_none() {
            self.views = Some(Self::build(ctx));
        }
        let textures = &mut self.textures;
        let (tree, root, list) = self.views.as_mut()?;
        let (root, list) = (*root, *list);
        // (the nanos by their time left, each icon faded past its share of time left; drawn again as the time runs)
        let mut order: Vec<&NanoInfo> = nanos.iter().collect();
        order.sort_by_key(|n| n.left(now));
        let rows: Vec<crate::gui::multilist::Item> = order
            .iter()
            .map(|n| crate::gui::multilist::Item {
                id: n.id as i64,
                icon: n.icon.as_ref().map(|icon| textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("nano-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id()),
                left: (n.total_ms > 0).then(|| n.left(now) as f32 / n.total_ms as f32),
                cells: vec![(1, crate::gui::multilist::CellView::Text(n.name.clone()))],
                keys: vec![(1, n.name.clone())],
                ..Default::default()
            })
            .collect();
        if crate::gui::multilist::items(tree, list) != rows.as_slice() {
            crate::gui::multilist::set_items(tree, list, rows);
        }
        crate::gui::multilist::update(tree, list);
        let used = stats.get(&180).copied().unwrap_or_else(|| nanos.iter().map(|n| n.cost as i64).sum());
        let max = stats.get(&181).copied().unwrap_or(0);
        let title = crate::gui::xml::ldb_text("#10000:89261993").replacen("%u", &used.to_string(), 1).replacen("%u", &max.to_string(), 1);
        let skin = crate::skin::skin(ctx);
        let mut open = self.open;
        let mut hovered = None;
        crate::chrome::window(ctx, palette, "NCU", &title, &mut open, |w| w.default_pos(crate::layout::pos("NCU", egui::pos2(560.0, 90.0))).default_size(egui::vec2(160.0, 160.0)).resizable(true), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                hovered = crate::gui::multilist::item_at(tree, list, response.rect.min, at).map(|id| (id as u32, at));
            }
        });
        if !nanos.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        for (_, event) in crate::gui::multilist::take_events(tree) {
            if let crate::gui::multilist::ItemEvent::Clicked(id, _) = event {
                self.info_url = Some(format!("itemid://{}/{}", crate::gui::iteminfo::NANO_IDENTITY, id));
            }
        }
        tree.signals.clear();
        if let Some((id, at)) = hovered
            && ctx.input(|i| i.pointer.secondary_clicked())
        {
            self.menu = Some((id, at));
            self.popup = Some(crate::chrome::menu_at(ctx, at, &[("Cancel this nano".to_string(), true)]));
        }
        // (a nano's tip: its name, time left, description and what it changes)
        if let Some((id, at)) = hovered.filter(|_| self.menu.is_none())
            && let Some(nano) = nanos.iter().find(|n| n.id == id)
        {
            let left = nano.left(now);
            egui::Area::new(egui::Id::unique("nano-tip")).order(egui::Order::Tooltip).fixed_pos(at + egui::vec2(16.0, 16.0)).interactable(false).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(320.0);
                    ui.label(egui::RichText::new(&nano.name).strong().color(accent));
                    if nano.total_ms > 0 {
                        ui.label(egui::RichText::new(format!("{} of {} left", clock(left), clock(nano.total_ms))).small());
                    }
                    if !nano.description.is_empty() {
                        ui.label(nano.description.chars().take(420).collect::<String>().replace("\r\n", "\n"));
                    }
                    for &(stat, amount) in nano.effects.iter().filter(|e| e.1 != 0).take(10) {
                        ui.horizontal(|ui| {
                            ui.label(crate::skills::stat_name(stat));
                            let color = if amount > 0 { egui::Color32::from_rgb(120, 220, 120) } else { egui::Color32::from_rgb(230, 110, 110) };
                            ui.label(egui::RichText::new(format!("{amount:+}")).color(color));
                        });
                    }
                });
            });
        }
        if crate::chrome::menu(ctx, palette, "nano-menu", &mut self.popup).is_some()
            && let Some((id, _)) = self.menu
            && nanos.iter().any(|n| n.id == id)
        {
            cancel = Some(id);
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        self.open = open;
        cancel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_reads_minutes_and_hours() {
        assert_eq!(clock(0), "0:00");
        assert_eq!(clock(61_000), "1:01");
        assert_eq!(clock(1_371_193), "22:51");
        assert_eq!(clock(3_725_000), "1:02:05");
    }

    #[test]
    fn the_time_left_counts_down_from_when_we_were_told() {
        let at = Instant::now();
        let nano = NanoInfo { id: 1, name: String::new(), description: String::new(), icon: None, remaining_ms: 10_000, total_ms: 20_000, at, effects: Vec::new(), cost: 0 };
        assert_eq!(nano.left(at), 10_000);
        assert_eq!(nano.left(at + std::time::Duration::from_secs(4)), 6_000);
        assert_eq!(nano.left(at + std::time::Duration::from_secs(60)), 0);
    }
}
