//! The Skills window: the attributes and all the skills in the original's groups, each with the value the original shows
//! (the stored value plus bonuses plus the attribute trickle-down; see `derived::shown`). A search box narrows the list.

use std::collections::BTreeMap;

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

/// The rows of a group that match the search words (all of the words, in any case), as `(name, value)`; a skill the
/// server never sent and the client cannot work out has no value and shows a dash.
pub fn rows<'a>(group: &'a [(u32, &'a str)], stats: &BTreeMap<u32, i64>, search: &str) -> Vec<(&'a str, Option<i64>)> {
    let words: Vec<String> = search.split_whitespace().map(str::to_lowercase).collect();
    group
        .iter()
        .filter(|(_, name)| {
            let lower = name.to_lowercase();
            words.iter().all(|w| lower.contains(w))
        })
        .map(|&(stat, name)| (name, stats.get(&stat).copied()))
        .collect()
}

#[derive(Default)]
pub struct SkillsWindow {
    pub open: bool,
    search: String,
}

impl SkillsWindow {
    pub fn show(&mut self, ctx: &egui::Context, name: &str, stats: &BTreeMap<u32, i64>, palette: &Palette) {
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let mut open = self.open;
        egui::Window::new(egui::RichText::new("Skills").color(accent)).open(&mut open).collapsible(false).default_pos(egui::pos2(300.0, 90.0)).default_size([260.0, 420.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.label(egui::RichText::new(if name.is_empty() { "-" } else { name }).color(accent));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(stats.get(&0x36).map_or("-".to_string(), |l| l.to_string())).color(accent));
                    ui.label("Level:");
                });
            });
            ui.horizontal(|ui| {
                ui.label("Find:");
                ui.text_edit_singleline(&mut self.search);
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (title, group) in GROUPS {
                    let shown = rows(group, stats, &self.search);
                    if shown.is_empty() {
                        continue;
                    }
                    egui::CollapsingHeader::new(egui::RichText::new(*title).color(accent)).default_open(true).show(ui, |ui| {
                        for (skill, value) in shown {
                            ui.horizontal(|ui| {
                                ui.label(skill);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(value.map_or("-".to_string(), |v| v.to_string()));
                                });
                            });
                        }
                    });
                }
            });
        });
        self.open = open;
    }
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
    fn the_search_narrows_the_rows() {
        let stats: BTreeMap<u32, i64> = [(103, 500), (102, 400)].into();
        let melee = GROUPS.iter().find(|g| g.0 == "Melee Weapons").unwrap().1;
        assert_eq!(rows(melee, &stats, "edged"), vec![("1h Edged", Some(500)), ("2h Edged", None)]);
        assert_eq!(rows(melee, &stats, "1H BLUNT"), vec![("1h Blunt", Some(400))]);
        assert!(rows(melee, &stats, "nothing here").is_empty());
    }
}
