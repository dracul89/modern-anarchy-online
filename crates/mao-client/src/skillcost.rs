//! What a point of a skill costs and how far a skill can be raised, as Gamecode works them out for the Skills window
//! (`N3Msg_GetSkillCost` FUN_10062521, `N3Msg_GetSkillCostLevel` FUN_100628de, `N3Msg_GetSkillMax` FUN_100656fb).
//!
//! The numbers are records of the resource database, each a 12-byte header and a run of little-endian words:
//!
//! - `0xf4312`, a breed's: what a point of each ability costs, in the order strength, intelligence, agility, sense,
//!   stamina, psychic (FUN_100c0236);
//! - `0xf425b`, a breed's: its number, where its six abilities start and their caps (FUN_100c6054);
//! - `0xf430e`, a profession's: the cost level of 77 skills in the order of [`ORDER`] (FUN_100c5054), ten times the IP
//!   a point costs at a value of one;
//! - `0xf4308`, a profession's: four words for each of the seven title levels, the first the level the title starts at
//!   (FUN_100c4e4a);
//! - `0xf4311`, a cost class's (1 to 5, the cost level over ten, rounded): a skill's cap at each title level and what
//!   the cap grows a level (FUN_100c5d1a).
//!
//! **Cost.** A point of an ability costs the breed's number times the value raised from; a point of a skill its cost
//! level times that value over ten; the level shown ([`Tables::cost_level`]) is ten times the breed's number for an
//! ability.
//!
//! **Cap of an ability** (FUN_10062c1e): its start plus three a level below level 200; from there the breed's cap plus
//! 20, 15 or 10 a level past 200 for a cost of 1, 2 or 3; never over the breed's cap (400, 300 or 200 more past 200).
//!
//! **Cap of a skill** (FUN_100656fb), the least of: 1000; the cost class's cap at the title level; for the first title
//! the level times the class's growth plus 5, for a later one the cap of the title before plus the growth for each
//! level into this title (levels past 204 not counted; the seventh title adds nothing); twice the abilities' weighted
//! mean (the trickle-down weights, `trickle_data`), when that is over nothing; and the cap of the title before plus six
//! for each level into this one. Past level 200 each level adds five times (six less the cost class).

use std::collections::BTreeMap;

/// The skills of a profession's cost record, in the order of its stream (FUN_100c5054).
pub const ORDER: [u32; 77] = [
    152, 137, 154, 153, 155, 156, 138, 142, 144, 100, 143, 93, 95, 139, 166, 117, 140, 92, 97, 168, 123, 124, 118, 119, 149, 120, 151, 167, 111, 148, 133, 134, 150, 114, 112, 116, 115, 113, 121,
    107, 106, 102, 104, 147, 105, 145, 146, 103, 101, 91, 128, 132, 130, 127, 129, 122, 131, 90, 126, 125, 96, 94, 165, 164, 135, 136, 163, 161, 159, 160, 162, 157, 158, 141, 110, 109, 108,
];

/// The abilities of a breed's cost record, in the order of its stream (FUN_100c0236).
const ABILITY_ORDER: [u32; 6] = [16, 19, 17, 20, 18, 21];

pub const ABILITY_COSTS: u32 = 0xf4312;
pub const BREEDS: u32 = 0xf425b;
pub const SKILL_COSTS: u32 = 0xf430e;
pub const TITLES: u32 = 0xf4308;
pub const CAPS: u32 = 0xf4311;

/// The tables, as read from the game's records.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tables {
    /// A breed's cost of a point of each ability (by the ability's stat less 16).
    ability_costs: BTreeMap<u32, [i32; 6]>,
    /// A breed's abilities at the start and their caps.
    breeds: BTreeMap<u32, ([i32; 6], [i32; 6])>,
    /// A profession's cost level of each skill.
    skill_costs: BTreeMap<u32, BTreeMap<u32, i32>>,
    /// The level each of a profession's seven titles starts at.
    titles: BTreeMap<u32, [i32; 7]>,
    /// A cost class's cap at each title level, and its growth a level.
    caps: BTreeMap<u32, ([i32; 7], i32)>,
}

/// Who the costs and caps are for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Character {
    pub breed: u32,
    /// The profession (stat 60); `None` for a character with none (the original's -1).
    pub profession: Option<u32>,
    pub level: i64,
    /// The title level (stat 37).
    pub title: i64,
    /// The abilities as trained, with the points being added in the window (stats 16 to 21).
    pub abilities: [i64; 6],
}

/// The words of a record after its 12-byte header.
fn words(record: &[u8]) -> Vec<i32> {
    record.get(12..).unwrap_or(&[]).chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// The x87 round-toward-zero the original ends its sums with (`_ftol2`).
fn whole(v: f64) -> i64 {
    v.trunc() as i64
}

impl Tables {
    /// Takes a record in (`kind` its type, `instance` the breed, profession or cost class); false when it is not one of
    /// the tables' or is too short.
    pub fn add(&mut self, kind: u32, instance: u32, record: &[u8]) -> bool {
        let w = words(record);
        match kind {
            ABILITY_COSTS if w.len() >= 6 => {
                let mut costs = [0; 6];
                for (k, stat) in ABILITY_ORDER.iter().enumerate() {
                    costs[(stat - 16) as usize] = w[k];
                }
                self.ability_costs.insert(instance, costs);
            }
            BREEDS if w.len() >= 13 => {
                let six = |from: usize| [w[from], w[from + 1], w[from + 2], w[from + 3], w[from + 4], w[from + 5]];
                self.breeds.insert(w[0] as u32, (six(1), six(7)));
            }
            SKILL_COSTS if w.len() >= ORDER.len() => {
                self.skill_costs.insert(instance, ORDER.iter().copied().zip(w.iter().copied()).collect());
            }
            TITLES if w.len() >= 28 => {
                let mut starts = [0; 7];
                for (k, s) in starts.iter_mut().enumerate() {
                    *s = w[k * 4];
                }
                self.titles.insert(instance, starts);
            }
            CAPS if w.len() >= 8 => {
                self.caps.insert(instance, ([w[0], w[1], w[2], w[3], w[4], w[5], w[6]], w[7]));
            }
            _ => return false,
        }
        true
    }

    /// The tables of a game's resource database.
    pub fn load(db: &mao_formats::resource::ResourceDatabase) -> Tables {
        let mut tables = Tables::default();
        for (kind, instances) in [(ABILITY_COSTS, 1..=7u32), (BREEDS, 1..=7), (SKILL_COSTS, 1..=15), (TITLES, 1..=15), (CAPS, 1..=7)] {
            for instance in instances {
                if let Ok(Some(record)) = db.get(mao_formats::resource::Id { kind, instance }) {
                    tables.add(kind, instance, &record);
                }
            }
        }
        tables
    }

    pub fn is_empty(&self) -> bool {
        self.skill_costs.is_empty() && self.ability_costs.is_empty()
    }

    /// FUN_100c01eb: what a point of an ability costs a breed; 999 for a breed with no record.
    fn ability_cost(&self, breed: u32, stat: u32) -> i32 {
        self.ability_costs.get(&breed).map_or(999, |c| c[(stat - 16) as usize])
    }

    /// FUN_100c5008: a skill's cost level for a profession; one for a skill the record says nothing of, nothing for a
    /// profession with no record.
    fn skill_cost_level(&self, profession: u32, stat: u32) -> i32 {
        match self.skill_costs.get(&profession).filter(|_| profession < 16) {
            Some(costs) => costs.get(&stat).copied().filter(|c| *c != 0).unwrap_or(1),
            None => 0,
        }
    }

    /// `N3Msg_GetSkillCostLevel` (FUN_100628de): what the window colours a skill's name by.
    pub fn cost_level(&self, c: &Character, stat: u32) -> i32 {
        match c.profession {
            None => 1,
            Some(_) if (16..=21).contains(&stat) => self.ability_cost(c.breed, stat) * 10,
            Some(p) if (100..=168).contains(&stat) => self.skill_cost_level(p, stat),
            Some(_) => 0,
        }
    }

    /// `N3Msg_GetSkillCost` (FUN_10062521): the IP a point costs at a value (one at the least).
    pub fn cost(&self, c: &Character, stat: u32, value: i64) -> f32 {
        let value = value.max(1) as i32;
        match c.profession {
            None => 1.0,
            Some(_) if (16..=21).contains(&stat) => (self.ability_cost(c.breed, stat) * value) as f32,
            Some(p) if (100..=168).contains(&stat) => (self.skill_cost_level(p, stat) * value) as f32 / 10.0,
            Some(_) => value as f32 * 0.5,
        }
    }

    /// The IP `added` points cost from `base` (the window's FUN_100f9b0d: each point's cost cut to a whole number).
    pub fn total_cost(&self, c: &Character, stat: u32, base: i64, added: i64) -> i64 {
        (0..added.max(0)).map(|i| whole(f64::from(self.cost(c, stat, base + i)))).sum()
    }

    /// FUN_10062c1e: how far an ability can be raised.
    fn ability_max(&self, c: &Character, stat: u32) -> i64 {
        let k = (stat - 16) as usize;
        let (start, cap) = self.breeds.get(&c.breed).map_or((0, 0), |b| (i64::from(b.0[k]), i64::from(b.1[k])));
        let cost = self.ability_cost(c.breed, stat);
        let by_cost = |one: i64, two: i64, three: i64| match cost {
            1 => one,
            2 => two,
            3 => three,
            _ => 0,
        };
        let limit = if c.level > 200 { cap + by_cost(400, 300, 200) } else { cap };
        let value = if c.level < 200 { c.level * 3 + start } else { (c.level - 200) * by_cost(20, 15, 10) + cap };
        value.min(limit)
    }

    /// `N3Msg_GetSkillMax` (FUN_100656fb): how far a skill or an ability can be raised.
    pub fn max(&self, c: &Character, stat: u32) -> i64 {
        // (the two skills nobody trains)
        if stat == 138 || stat == 140 {
            return 0;
        }
        if (16..=21).contains(&stat) {
            return self.ability_max(c, stat);
        }
        let level = self.skill_cost_level(c.profession.unwrap_or(u32::MAX), stat);
        let class = whole(f64::from(level) / 10.0 + 0.5) as u32;
        let table = self.caps.get(&class).filter(|_| (1..=7).contains(&class));
        let cap_at = |title: i64| table.filter(|_| (1..=7).contains(&title)).map_or(0, |t| t.0[(title - 1) as usize]) as f32;
        let title_start = |title: i64| c.profession.filter(|p| *p < 16).and_then(|p| self.titles.get(&p)).filter(|_| (1..=7).contains(&title)).map(|t| i64::from(t[(title - 1) as usize]));
        let mut max = 1000.0f32;
        'title: {
            if !(100..=168).contains(&stat) || c.profession.is_none() {
                break 'title;
            }
            let cap = cap_at(c.title);
            if cap <= 1000.0 {
                max = cap;
            }
            let growth = table.map_or(6, |t| t.1);
            let growth = i64::from(if (1..=10).contains(&growth) { growth } else { 6 });
            if c.title > 1 {
                let before = cap_at(c.title - 1);
                let Some(start) = title_start(c.title) else { break 'title };
                let v = if c.title == 7 { whole(f64::from(before)) } else { whole(((c.level.min(204) - start + 1) * growth) as f64 + f64::from(before)) } as f32;
                if v < max {
                    max = v;
                }
            } else if c.title == 1 {
                let v = whole((c.level * growth) as f64 + 5.0) as f32;
                if v < max {
                    max = v;
                }
            }
        }
        // (twice the weighted mean of the abilities: FUN_10065008's quarter of the weighted sum, times eight)
        let weights = crate::trickle_data::WEIGHTS.iter().find(|w| w.0 == stat).map_or([0u8; 6], |w| w.1);
        let weighted: f64 = (0..6).map(|k| f64::from(weights[k]) * c.abilities[k] as f64 / 100.0).sum();
        let by_abilities = whole(f64::from((weighted as f32) * 0.25) * 8.0);
        if (by_abilities as f32) < max && by_abilities > 0 {
            max = by_abilities as f32;
        }
        if c.title > 1
            && let Some(start) = title_start(c.title)
        {
            let v = ((c.level + 1 - start) * 6) as f32 + cap_at(c.title - 1);
            if v < max {
                max = v;
            }
        }
        if c.level > 200 {
            max += ((c.level - 200) * (6 - i64::from(class)) * 5) as f32;
        }
        whole(f64::from(max))
    }
}

/// The colour of a skill's name by its cost level (the window's FUN_100f99b1).
pub fn colour(cost_level: i32) -> u32 {
    match cost_level {
        i32::MIN..=5 => 0x7dcc5e,
        6..=10 => 0x00cc88,
        11..=15 => 0x00cccc,
        16..=20 => 0x0088cc,
        _ => 0x075aff,
    }
}

/// The game's tables, read once its folder is known.
pub fn tables() -> Option<&'static Tables> {
    static TABLES: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();
    if let Some(t) = TABLES.get() {
        return Some(t);
    }
    let db = mao_formats::resource::ResourceDatabase::open_game(crate::theme::game_dir()?).ok()?;
    let tables = Tables::load(&db);
    (!tables.is_empty()).then(|| TABLES.get_or_init(|| tables))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record: the 12-byte header and the words.
    fn record(words: &[i32]) -> Vec<u8> {
        let mut out = vec![0u8; 12];
        for w in words {
            out.extend(w.to_le_bytes());
        }
        out
    }

    /// Made-up tables: a breed whose agility is cheap, a profession with one cheap and one dear skill.
    fn tables() -> Tables {
        let mut t = Tables::default();
        // (strength, intelligence, agility, sense, stamina, psychic)
        assert!(t.add(ABILITY_COSTS, 2, &record(&[2, 2, 1, 1, 3, 2])));
        assert!(t.add(BREEDS, 2, &record(&[2, 3, 15, 6, 6, 10, 3, 464, 544, 480, 464, 512, 448])));
        let mut costs = vec![30; 77];
        costs[ORDER.iter().position(|s| *s == 152).unwrap()] = 10;
        costs[ORDER.iter().position(|s| *s == 100).unwrap()] = 24;
        assert!(t.add(SKILL_COSTS, 4, &record(&costs)));
        let titles: Vec<i32> = [1, 15, 50, 100, 150, 190, 205].iter().flat_map(|s| [*s, 0, 0, 0]).collect();
        assert!(t.add(TITLES, 4, &record(&titles)));
        assert!(t.add(CAPS, 1, &record(&[60, 200, 360, 490, 550, 600, 1200, 5])));
        assert!(t.add(CAPS, 2, &record(&[55, 180, 330, 450, 500, 540, 1040, 4])));
        assert!(t.add(CAPS, 3, &record(&[50, 160, 300, 410, 450, 480, 880, 4])));
        assert!(!t.add(CAPS, 4, &record(&[1, 2])) && !t.add(1, 1, &record(&[0; 20])));
        t
    }

    #[test]
    fn a_point_costs_by_the_breed_and_the_profession() {
        let t = tables();
        let c = Character { breed: 2, profession: Some(4), level: 10, title: 1, abilities: [50; 6] };
        // (an ability: the breed's number times the value; its level ten times the number)
        assert_eq!((t.cost(&c, 17, 40), t.cost_level(&c, 17)), (40.0, 10));
        assert_eq!((t.cost(&c, 18, 40), t.cost_level(&c, 18)), (120.0, 30));
        // (a skill: its cost level times the value, over ten)
        assert_eq!((t.cost(&c, 152, 25), t.cost_level(&c, 152)), (25.0, 10));
        assert_eq!((t.cost(&c, 100, 25), t.cost_level(&c, 100)), (60.0, 24));
        assert_eq!(t.cost(&c, 100, 0), 2.4);
        // (three points from 10: 24, 26.4 and 28.8, each cut to a whole number)
        assert_eq!(t.total_cost(&c, 100, 10, 3), 24 + 26 + 28);
        // (a character with no profession pays one a point; a breed with no record 999)
        assert_eq!(t.cost(&Character { profession: None, ..c }, 100, 25), 1.0);
        assert_eq!(t.cost(&Character { breed: 9, ..c }, 16, 2), 1998.0);
        assert_eq!([colour(5), colour(10), colour(15), colour(20), colour(30)], [0x7dcc5e, 0x00cc88, 0x00cccc, 0x0088cc, 0x075aff]);
    }

    #[test]
    fn an_ability_grows_three_a_level_to_the_breeds_cap() {
        let t = tables();
        let at = |level: i64| t.max(&Character { breed: 2, profession: Some(4), level, title: 1, abilities: [0; 6] }, 17);
        // (agility: starts at 15, capped at 544; past 200 twenty a level for a cost of one, to 400 over the cap)
        assert_eq!([at(1), at(100), at(199), at(200), at(210), at(220)], [18, 315, 544, 544, 744, 944]);
    }

    #[test]
    fn a_skill_is_capped_by_title_level_and_abilities() {
        let t = tables();
        let c = |level: i64, title: i64, abilities: i64| Character { breed: 2, profession: Some(4), level, title, abilities: [abilities; 6] };
        // (the first title: five a level plus 5 for the cheapest class, under the title's cap of 60)
        assert_eq!(t.max(&c(5, 1, 500), 152), 30);
        assert_eq!(t.max(&c(14, 1, 500), 152), 60);
        // (a dearer class grows four a level under its own caps)
        assert_eq!(t.max(&c(5, 1, 500), 100), 25);
        // (the second title, from level 15: the cap before plus the growth for each level into it)
        assert_eq!(t.max(&c(15, 2, 500), 152), 65);
        assert_eq!(t.max(&c(20, 2, 500), 152), 90);
        // (low abilities hold a skill down: twice their weighted mean)
        assert_eq!(t.max(&c(20, 2, 30), 152), 60);
        // (the two skills nobody trains; past level 200 each level adds five times six less the class)
        assert_eq!(t.max(&c(20, 2, 500), 138), 0);
        assert_eq!(t.max(&c(220, 7, 2000), 152), 600 + 20 * 5 * 5);
    }

    /// The game's own records are read: every profession and breed has its table.
    #[test]
    fn the_games_tables_are_read() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let t = Tables::load(&db);
        assert!(t.skill_costs.len() >= 14 && t.titles.len() >= 14 && t.ability_costs.len() >= 4 && t.breeds.len() >= 4 && t.caps.len() >= 5, "{t:?}");
        // (an Opifex pays one for agility and three for stamina; a level 1 Opifex Fixer's agility goes to 18)
        let c = Character { breed: 2, profession: Some(5), level: 1, title: 1, abilities: [10; 6] };
        assert_eq!((t.cost_level(&c, 17), t.cost_level(&c, 18)), (10, 30));
        assert_eq!(t.max(&c, 17), 18);
        // (every skill of the window has a cost level and a cap that is not negative)
        for (_, stats) in crate::skillwindow::GROUPS {
            for &stat in stats {
                assert!(t.cost_level(&c, stat) > 0 && t.max(&c, stat) >= 0, "{stat}");
            }
        }
    }
}
