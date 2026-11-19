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
//! (FUN_1005de3a). The value is a character preference whose default is 1 (`Preferences_t::SetDefaultCharPrefs`
//! 0x10124951 hands `InitDefaultInt` the name, 1, 0, 1 at 0x101249f7): the window starts with the titles. Kept here in
//! `tabs.ini` under its name. (Until 2026-11-18 this said "it starts unset" and the window began with the numbers,
//! forgetting the choice at each start.)

use crate::theme::Palette;

/// The bars' stats and the factions' short names (Gamecode FUN_10035eaf).
const FACTIONS: [(u32, &str); 7] = [(566, "Guardian"), (567, "Followers"), (568, "Operators"), (569, "Unredeemed"), (570, "Devoted"), (571, "Conservers"), (572, "Redeemed")];

/// The bands' titles (Gamecode FUN_10035eaf's third table, which `GetFactionTitle` reads by the band's place).
const TITLES: [&str; 12] = ["Dreaded", "Hated", "Enemy", "Wanted", "Disliked", "Indifferent", "Neutral", "Liked", "Ally", "Friend", "Close friend", "Family"];

/// The bands as Gamecode reads them at start (FUN_100c6914): each of the database's 0xf4265 records (FUN_100c24b8: its
/// low and high, then two strings) in a map by instance, so a band's place is its record's among them.
pub fn bands_of(db: &mao_formats::resource::ResourceDatabase) -> Vec<(i64, i64)> {
    let mut ids: Vec<mao_formats::resource::Id> = db.ids().map(|ids| ids.flatten().filter(|i| i.kind == 0xf4265).collect()).unwrap_or_default();
    ids.sort();
    ids.into_iter()
        .filter_map(|id| {
            let data = db.get(id).ok()??;
            // (after the object's type, instance and revision)
            let word = |at: usize| data.get(at..at + 4).map(|b| i64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]])));
            Some((word(12)?, word(16)?))
        })
        .collect()
}

static BANDS: std::sync::Mutex<Option<Vec<(i64, i64)>>> = std::sync::Mutex::new(None);

/// The bands, read once the game's database is open (none before).
fn bands() -> Vec<(i64, i64)> {
    let mut kept = BANDS.lock().unwrap();
    if kept.is_none()
        && let Some(db) = crate::net::item_db()
    {
        *kept = Some(bands_of(&db));
    }
    kept.clone().unwrap_or_default()
}

/// `N3Msg_GetFactionRange` (Gamecode FUN_100c246b): the band a standing is in, and its title; outside them all, the
/// caller's -50000 to 50000 and Gamecode's "Missing FactionTitle".
pub fn band(standing: i64) -> (i64, i64, String) {
    band_in(&bands(), standing)
}

fn band_in(bands: &[(i64, i64)], standing: i64) -> (i64, i64, String) {
    match bands.iter().position(|b| b.0 <= standing && standing <= b.1) {
        Some(k) => (bands[k].0, bands[k].1, TITLES.get(k).map_or_else(|| format!("Missing FactionTitle: {}", k + 1), |t| t.to_string())),
        None => (-50000, 50000, "Missing FactionTitle: -1".to_string()),
    }
}

/// How full a bar is: the standing's share of its band, the other way below 0.
pub fn fill(standing: i64) -> f32 {
    fill_in(&bands(), standing)
}

fn fill_in(bands: &[(i64, i64)], standing: i64) -> f32 {
    let (low, high, _) = band_in(bands, standing);
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
    /// `IsFactionTitleShown`; not set until the kept choice is read.
    titles: Option<bool>,
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
        let titles = *self.titles.get_or_insert_with(|| crate::layout::tab("IsFactionTitleShown").is_none_or(|v| v != 0));
        let Some((tree, root)) = self.views.as_mut() else { return };
        let root = *root;
        for (k, &(stat, name)) in FACTIONS.iter().enumerate() {
            let standing = stats.get(&stat).copied().unwrap_or(0);
            let right = if titles { band(standing).2 } else { standing.to_string() };
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
            self.titles = Some(!titles);
            crate::layout::set_tab("IsFactionTitleShown", usize::from(!titles));
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
        // (two bands: below 0 and from 0)
        let bands = [(-1000, -1), (0, 999)];
        assert_eq!(band_in(&bands, 0), (0, 999, "Hated".to_string()));
        assert_eq!(band_in(&bands, -1).2, "Dreaded");
        assert_eq!(band_in(&bands, 5000).2, "Missing FactionTitle: -1");
        // (the share of the band; below 0 the other way)
        assert_eq!(fill_in(&bands, 0), 0.0);
        assert_eq!(fill_in(&bands, 999), 1.0);
        assert_eq!(fill_in(&bands, -1), 0.0);
        assert_eq!(fill_in(&bands, -1000), 1.0);
    }

    /// WIN-053: the bands are the game database's 0xf4265 records, twelve, one after the other from the lowest standing.
    #[test]
    fn bands_are_read_from_the_database() {
        let Ok(game) = std::env::var("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let bands = bands_of(&db);
        assert_eq!(bands.len(), TITLES.len());
        assert!(bands.windows(2).all(|w| w[0].1 + 1 == w[1].0));
        assert_eq!(band_in(&bands, 0).2, "Neutral");
        assert_eq!(band_in(&bands, -1).2, "Indifferent");
    }
}
