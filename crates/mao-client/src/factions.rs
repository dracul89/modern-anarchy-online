//! The faction window, GUI.dll's `FactionView_c` (FUN_1005e3f5, `faction_window`, title "Faction"): the character's standing
//! with the seven Shadowlands factions, a `FactionBar_c` (FUN_1005e141) for each of stats 566 to 572 one under the other
//! (borders 2,10,2,1 for the first, 2,1,2,1 between, 2,1,2,2 for the last).
//!
//! A bar (FUN_1005de7f, again on each change of its stat): a `PowerbarView_c` over `GFX_GUI_HOR_BAR_EMPTY`, filled with
//! `GFX_GUI_HOR_BAR_GREEN` from the left for a standing of 0 or more and `GFX_GUI_HOR_BAR_RED` from the right below 0
//! (`GFX_GUI_HOR_BAR_YELLOW` from the left for Guardian of Shadow, stat 566, whatever the standing). It is filled to where
//! the standing is within its band (`N3Msg_GetFactionRange`: the database's 0xf4265 records, Dreaded -50000 to -30001 up to
//! Family 30000 to 50000), and below 0 the other way (one less that share). Its labels (`SetLabels`, font-5 text views
//! with borders 6 and 2 on the left, 6 on the right): the faction's short name (`GetFactionStr(stat, false)`, Gamecode
//! FUN_10035eaf's second table) and, in 0xffff88, the standing, or its band's title (`GetFactionTitle`: Gamecode's own
//! table of the titles) when `IsFactionTitleShown` is set. A left click on a bar switches that for every bar
//! (FUN_1005de3a); it starts unset (0x101249fc).

use crate::theme::Palette;

/// The bars' stats and the factions' short names (Gamecode FUN_10035eaf).
const FACTIONS: [(u32, &str); 7] = [(566, "Guardian"), (567, "Followers"), (568, "Operators"), (569, "Unredeemed"), (570, "Devoted"), (571, "Conservers"), (572, "Redeemed")];

/// The bands (the database's 0xf4265 records 1 to 12: low, high) and their titles (Gamecode FUN_10035eaf's third table,
/// which `GetFactionTitle` reads by the band's place).
const BANDS: [(i64, i64, &str); 12] = [
    (-50000, -30001, "Dreaded"),
    (-30000, -18001, "Hated"),
    (-18000, -10001, "Enemy"),
    (-10000, -5001, "Wanted"),
    (-5000, -1001, "Disliked"),
    (-1000, -1, "Indifferent"),
    (0, 999, "Neutral"),
    (1000, 4999, "Liked"),
    (5000, 9999, "Ally"),
    (10000, 17999, "Friend"),
    (18000, 29999, "Close friend"),
    (30000, 50000, "Family"),
];

/// `N3Msg_GetFactionRange` (Gamecode FUN_100c246b): the band a standing is in, and its title; outside them all, the
/// caller's -50000 to 50000 and Gamecode's "Missing FactionTitle".
pub fn band(standing: i64) -> (i64, i64, String) {
    match BANDS.iter().position(|b| b.0 <= standing && standing <= b.1) {
        Some(k) => (BANDS[k].0, BANDS[k].1, BANDS[k].2.to_string()),
        None => (-50000, 50000, "Missing FactionTitle: -1".to_string()),
    }
}

/// How full a bar is: the standing's share of its band, the other way below 0.
pub fn fill(standing: i64) -> f32 {
    let (low, high, _) = band(standing);
    let share = (standing - low) as f32 / (high - low) as f32;
    if standing < 0 { 1.0 - share } else { share }
}

/// The bar's picture and direction for a standing.
fn picture(stat: u32, standing: i64) -> (&'static str, crate::gui::powerbar::Direction) {
    use crate::gui::powerbar::Direction;
    if stat == 566 {
        ("GFX_GUI_HOR_BAR_YELLOW", Direction::Right)
    } else if standing < 0 {
        ("GFX_GUI_HOR_BAR_RED", Direction::Left)
    } else {
        ("GFX_GUI_HOR_BAR_GREEN", Direction::Right)
    }
}

fn view_xml() -> String {
    let mut bars = String::new();
    for k in 0..FACTIONS.len() {
        let borders = match k {
            0 => "Rect(2,10,2,1)",
            6 => "Rect(2,1,2,2)",
            _ => "Rect(2,1,2,1)",
        };
        bars += &format!("<PowerBar name=\"bar{k}\" bg_gfx=\"GFX_GUI_HOR_BAR_EMPTY\" full_gfx=\"GFX_GUI_HOR_BAR_GREEN\" layout_borders=\"{borders}\"/>");
    }
    format!("<View view_layout=\"vertical\">{bars}</View>")
}

#[derive(Default)]
pub struct FactionWindow {
    pub open: bool,
    /// `IsFactionTitleShown`.
    titles: bool,
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id)>,
}

impl FactionWindow {
    pub fn show(&mut self, ctx: &egui::Context, stats: &std::collections::BTreeMap<u32, i64>, palette: &Palette) {
        if !self.open {
            return;
        }
        if self.views.is_none() {
            let doc = crate::gui::xml::parse(&view_xml());
            let mut tree = crate::gui::view::Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            if let Some(root) = doc.first().and_then(|e| crate::gui::create(&mut tree, e)) {
                // (`SetLabels`: the two text views laid across the bar, a spacer between)
                for k in 0..FACTIONS.len() {
                    let labels = crate::gui::xml::parse(&format!("<TextView name=\"left{k}\" value=\"\" layout_borders=\"Rect(6,0,2,0)\"/><HLayoutSpacer/><TextView name=\"right{k}\" value=\"\" layout_borders=\"Rect(0,0,6,0)\"/>"));
                    let Some(bar) = tree.find(root, &format!("bar{k}")) else { continue };
                    tree.get_mut(bar).layout = crate::gui::view::Layout::Horizontal;
                    for e in &labels {
                        if let Some(id) = crate::gui::create(&mut tree, e) {
                            tree.add_child(bar, id);
                        }
                    }
                }
                self.views = Some((tree, root));
            }
        }
        let Some((tree, root)) = self.views.as_mut() else { return };
        let root = *root;
        for (k, &(stat, name)) in FACTIONS.iter().enumerate() {
            let standing = stats.get(&stat).copied().unwrap_or(0);
            let right = if self.titles { band(standing).2 } else { standing.to_string() };
            set_text(tree, root, &format!("left{k}"), name);
            set_text(tree, root, &format!("right{k}"), &format!("<font color=#ffff88>{right}</font>"));
            if let Some(id) = tree.find(root, &format!("bar{k}"))
                && let Some(bar) = tree.control_mut::<crate::gui::powerbar::PowerBar>(id)
            {
                let (full, direction) = picture(stat, standing);
                bar.full = Some(full.to_string());
                bar.direction = direction;
                bar.set_value(fill(standing));
            }
        }
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(tree, root, false);
        let mut open = self.open;
        let mut clicked = false;
        crate::chrome::window(ctx, palette, "Faction", "Faction", &mut open, |w| w.default_pos(crate::layout::pos("Faction", egui::pos2(300.0, 200.0))).default_size(min).resizable(false), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            clicked = ui.input(|i| i.pointer.primary_clicked()) && response.contains_pointer();
        });
        // (FUN_1005de3a: a left click on a bar shows the titles or the standings)
        if clicked {
            self.titles = !self.titles;
        }
        tree.signals.clear();
        self.open = open;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_and_fill_as_the_original() {
        assert_eq!(band(0).2, "Neutral");
        assert_eq!(band(-1).2, "Indifferent");
        assert_eq!(band(18000).2, "Close friend");
        assert_eq!(band(50000).2, "Family");
        // (the share of the band; below 0 the other way)
        assert_eq!(fill(0), 0.0);
        assert_eq!(fill(999), 1.0);
        assert_eq!(fill(-1), 0.0);
        assert_eq!(fill(-1000), 1.0);
        assert!((fill(30000 + 10000) - 0.5).abs() < 1e-6);
    }
}
