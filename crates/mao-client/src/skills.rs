//! The Skills window: the attributes and all the skills in the original's groups, each with the value the original shows
//! (the stored value plus bonuses plus the attribute trickle-down; see `derived::shown`). A search box narrows the list.

use std::collections::BTreeMap;
use std::sync::Mutex;

/// The raises accepted in the window, for the connection to send (`Skill`: each skill with its new trained value).
static RAISES: Mutex<Vec<Vec<(u32, u32)>>> = Mutex::new(Vec::new());

/// The raises waiting to be sent.
pub fn take_raises() -> Vec<Vec<(u32, u32)>> {
    std::mem::take(&mut *RAISES.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Whether IP raise a stat: the abilities (16..21) and the skills (100..168), the ranges the original's cost lookup
/// (`N3Msg_GetSkillCost`, FUN_10062521) prices by the breed's and the profession's tables.
pub fn trainable(stat: u32) -> bool {
    (16..=21).contains(&stat) || (100..=168).contains(&stat)
}


use crate::derived::Breakdown;
use crate::theme::Palette;

/// The groups: a title, and for each skill its stat number and name.
pub const GROUPS: &[(&str, &[(u32, &str)])] = &[
    ("Attributes", &[(16, "Strength"), (17, "Agility"), (18, "Stamina"), (19, "Intelligence"), (20, "Sense"), (21, "Psychic")]),
    (
        "Body & Defense",
        &[(152, "Body Development"), (132, "Nano Pool"), (168, "Nano Resist"), (153, "Duck-Exp"), (154, "Dodge-Rng"), (155, "Evade-ClsC"), (156, "Run Speed")],
    ),
    (
        "Melee Weapons",
        &[
            (100, "Martial Arts"),
            (101, "Multi Melee"),
            (102, "1h Blunt"),
            (103, "1h Edged"),
            (104, "Melee Energy"),
            (105, "2h Edged"),
            (106, "Piercing"),
            (107, "2h Blunt"),
            (108, "Sharp Objects"),
        ],
    ),
    (
        "Ranged Weapons",
        &[
            (109, "Grenade"),
            (110, "Heavy Weapons"),
            (111, "Bow"),
            (112, "Pistol"),
            (113, "Rifle"),
            (114, "MG / SMG"),
            (115, "Shotgun"),
            (116, "Assault Rifle"),
            (133, "Ranged Energy"),
            (134, "Multi Ranged"),
        ],
    ),
    (
        "Special Attacks",
        &[
            (142, "Brawl"),
            (143, "Riposte"),
            (144, "Dimach"),
            (145, "Parry"),
            (146, "Sneak Attack"),
            (147, "Fast Attack"),
            (148, "Burst"),
            (150, "Fling Shot"),
            (151, "Aimed Shot"),
            (167, "Full Auto"),
            (121, "Bow Special Attack"),
        ],
    ),
    ("Initiative", &[(118, "Melee Init."), (119, "Ranged Init."), (120, "Physical Init."), (149, "Nano C. Init.")]),
    (
        "Nano Skills",
        &[(127, "Matter Metamorphosis"), (128, "Biological Metamorphosis"), (129, "Psychological Modifications"), (130, "Matter Creation"), (131, "Time & Space"), (160, "Nano Programming")],
    ),
    (
        "Trade & Repair",
        &[
            (125, "Mechanical Engineering"),
            (126, "Electrical Engineering"),
            (157, "Quantum Physics"),
            (158, "Weapon Smithing"),
            (159, "Pharma Tech"),
            (161, "Computer Literacy"),
            (163, "Chemistry"),
        ],
    ),
    ("Medical", &[(123, "First Aid"), (124, "Treatment"), (162, "Psychology")]),
    (
        "Misc",
        &[
            (122, "Sensory Improvement"),
            (135, "Trap Disarm"),
            (136, "Perception"),
            (137, "Adventuring"),
            (138, "Swimming"),
            (140, "Map Navigation"),
            (141, "Tutoring"),
            (164, "Concealment"),
            (165, "Breaking & Entry"),
            (117, "Vehicle Water"),
            (139, "Vehicle Air"),
            (166, "Vehicle Ground"),
        ],
    ),
];

/// The name of a stat for a tooltip: the skills and attributes by their window names, the common others by hand.
pub fn stat_name(stat: u32) -> String {
    if let Some(name) = GROUPS.iter().flat_map(|(_, g)| g.iter()).find(|(s, _)| *s == stat).map(|(_, n)| *n) {
        return name.to_string();
    }
    match stat {
        54 => "Level".into(),
        37 => "Title Level".into(),
        169 => "Alien Level".into(),
        1 => "Max Health".into(),
        221 => "Max Nano Energy".into(),
        90 => "Projectile AC".into(),
        91 => "Melee AC".into(),
        92 => "Energy AC".into(),
        93 => "Chemical AC".into(),
        94 => "Radiation AC".into(),
        95 => "Cold AC".into(),
        96 => "Poison AC".into(),
        97 => "Fire AC".into(),
        276 => "Add. Proj. Damage".into(),
        277 => "Add. Melee Damage".into(),
        278 => "Add. Energy Damage".into(),
        279 => "Add. Chem. Damage".into(),
        280 => "Add. Rad. Damage".into(),
        281 => "Add. Cold Damage".into(),
        282 => "Add. Poison Damage".into(),
        311 => "Add. Fire Damage".into(),
        316 => "Add. Nano Damage".into(),
        317 => "Add. Nano Damage (b)".into(),
        318 => "Nano Cost Modifier".into(),
        343 => "Nano Delta".into(),
        364 => "Nano Interval".into(),
        181 => "NCU Memory".into(),
        _ => format!("Stat {stat}"),
    }
}

/// The rows of a group that match the search words (all of the words, in any case), as `(name, value)`; a skill the
/// server never sent and the client cannot work out has no value and shows a dash.
pub fn rows<'a>(group: &'a [(u32, &'a str)], stats: &BTreeMap<u32, i64>, search: &str) -> Vec<(u32, &'a str, Option<i64>)> {
    let words: Vec<String> = search.split_whitespace().map(str::to_lowercase).collect();
    group
        .iter()
        .filter(|(_, name)| {
            let lower = name.to_lowercase();
            words.iter().all(|w| lower.contains(w))
        })
        .map(|&(stat, name)| (stat, name, stats.get(&stat).copied()))
        .collect()
}

/// The hover text of a row: the trained (stored) value and every bonus with where it comes from. The "Trained" line is left out
/// for a stat whose stored value is 0 but that has bonuses (the armour classes).
pub fn breakdown_lines(b: &Breakdown) -> Vec<(&'static str, i64)> {
    // A stat nothing trains (the armour classes) has no "Trained" line unless that is all there is.
    let mut lines = if b.stored != 0 || b.total() == 0 { vec![("Trained", b.stored)] } else { Vec::new() };
    for (what, amount) in [("Equipment", b.worn), ("Perks", b.perks), ("Nano buffs", b.nanos), ("Attributes", b.trickle), ("Organization", b.org)] {
        if amount != 0 {
            lines.push((what, amount));
        }
    }
    lines
}

pub(crate) fn breakdown_text(ui: &mut egui::Ui, skill: &str, b: &Breakdown, accent: egui::Color32) {
    ui.label(egui::RichText::new(skill).strong().color(accent));
    for (what, amount) in breakdown_lines(b) {
        ui.horizontal(|ui| {
            ui.label(what);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(if what == "Trained" { amount.to_string() } else { format!("{amount:+}") });
            });
        });
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Total").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(b.total().to_string()).strong());
        });
    });
}

/// The Skills window: GUI.dll's `SkillWindow` on the skin's `Skills.xml` (`skillwindow.rs`).
#[derive(Default)]
pub struct SkillsWindow {
    pub open: bool,
    view: crate::skillwindow::SkillWindow,
}

impl SkillsWindow {
    pub fn show(&mut self, ctx: &egui::Context, _name: &str, stats: &BTreeMap<u32, i64>, parts: &BTreeMap<u32, Breakdown>, palette: &Palette) {
        self.view.show(ctx, &mut self.open, stats, parts, palette);
    }
}

/// Raises accepted in the window, for the connection to send.
pub fn push_raises(raises: Vec<(u32, u32)>) {
    RAISES.lock().unwrap_or_else(|e| e.into_inner()).push(raises);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_trickle_skill_is_in_a_group() {
        let listed: std::collections::BTreeSet<u32> = GROUPS.iter().flat_map(|(_, g)| g.iter().map(|&(s, _)| s)).collect();
        for &(stat, _) in crate::trickle_data::WEIGHTS {
            assert!(listed.contains(&stat), "skill {stat} is not in any group");
        }
        for attribute in 16..=21 {
            assert!(listed.contains(&attribute));
        }
    }

    #[test]
    fn stats_have_names() {
        assert_eq!(stat_name(16), "Strength");
        assert_eq!(stat_name(91), "Melee AC");
        assert_eq!(stat_name(9999), "Stat 9999");
    }

    #[test]
    fn the_hover_lists_only_the_parts_that_count() {
        let b = Breakdown { stored: 40, worn: 7, perks: 0, nanos: 10, trickle: 25, org: 39 };
        assert_eq!(breakdown_lines(&b), vec![("Trained", 40), ("Equipment", 7), ("Nano buffs", 10), ("Attributes", 25), ("Organization", 39)]);
        assert_eq!(b.total(), 121);
        let armour = Breakdown { worn: 230, ..Default::default() };
        assert_eq!(breakdown_lines(&armour), vec![("Equipment", 230)]);
    }

    #[test]
    fn the_search_narrows_the_rows() {
        let stats: BTreeMap<u32, i64> = [(103, 500), (102, 400)].into();
        let melee = GROUPS.iter().find(|g| g.0 == "Melee Weapons").unwrap().1;
        assert_eq!(rows(melee, &stats, "edged"), vec![(103, "1h Edged", Some(500)), (105, "2h Edged", None)]);
        assert_eq!(rows(melee, &stats, "1H BLUNT"), vec![(102, "1h Blunt", Some(400))]);
        assert!(rows(melee, &stats, "nothing here").is_empty());
    }
}
