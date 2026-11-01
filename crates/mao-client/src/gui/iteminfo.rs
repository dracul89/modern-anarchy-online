//! The info view's item pages: GUI.dll's FUN_10038f6b and its general builder FUN_1003882b, whose sections each make one
//! piece of markup from the item's stats (`N3Msg_GetSkill` on the item: here its template's stats) and the text
//! database (category 506 by name). A section is FUN_10031b5b: `<div indent=wrapped><font color=CCInfoHeader>` its
//! label `</font><font color=CCInfoText>` its value `</font></div>`.
//!
//! The sections, in order: the name in its rarity's colour (FUN_10031d7c, `N3Msg_GetRarityColor`: stat 688 picks
//! CCItemUnknown, CCItemTrash, CCItemNormal, CCItemExotic, CCItemQuest, CCItemSocial), NODROP / UNIQUE / Playshift
//! requirements (FUN_100335df, flags stat 0 bits 0x4000000, 0x8000000, 0x40000), the time left of a temporary item
//! (FUN_10031c4b, stat 8 in hundredths), "can be looted in PvP" (FUN_1003198d, stat 30 bit 0x40), rarity and quality
//! level (FUN_10033403, stats 688 and 701, "SPECIAL" for flag 0x20), the requirements (FUN_100331a7,
//! `N3Msg_GetUseByString`: `criteria.rs`), the locations (FUN_10033255, `N3Msg_GetLocationString` 0x1001a0a5: stat 298's
//! bits 1 to 15 named from category 1005 at 300 for weapons, 200 armour, 400 implants), the charges (stat 26, with
//! " (Stackable)" for stat 30 bit 0x200, " (Splitable)" without bit 0x1000000) and the description (FUN_100332ff,
//! `N3Msg_GetDesc`, "\n" as a new line).
//!
//! The modifiers (FUN_10032c1d): the worn (14) and used (0) spell lists as `spells.rs` writes them.
//!
//! A weapon (item class 1) has FUN_1003732c's page: the ammunition in the colour of how full the clip is, its type,
//! damage, attack and recharge times, equip delay, initiative, range, dual wield, special attacks, maximum beneficial
//! skill and damage type as well.
//!
//! The attack and defence skills (FUN_10032ac9, FUN_10032b73: the template's skill lists 12 and 13, `item::skill_lists`,
//! each skill's name and " <share>% ") and the skills it is used with (FUN_100330f9, list 3: the name, then " <value> "
//! or " (<value>) " when negative, unless it is 1; never for a nano program).
//!
//! A nano program (`itemid://53019/<id>`, identity 0xcf1b) has FUN_1003683d's page: the name, its nano cost (stat 407
//! times `N3Msg_GetNanoCostModifier`, the reader's stat 318 but no lower than the breed's 50, 45 or 55, over 100; in
//! red when the reader's stat 221 is below it, then the cost without the modifier in brackets), the NCU it takes (stat
//! 54 unless 999), its school (stat 405: Combat, Medical, Protection, Psi, Space from category 10000), stacking order
//! (551), nanolines (stats 75 and 546-550 named from category 2009, not its own id), duration (`N3Msg_GetBuffTotalTime`:
//! its repeating spells' time, else stat 8, in hundredths as h:m:s), range, damage, attack time (FUN_10032582, which here takes 10 as none), the local
//! cooldown (stat 254, at least a second) and the nanoline ones (stats 643-648 against the nanolines), attack and defence
//! skills, the modifiers, what it does on death (list 20) and on cancel (list 28), the requirements and the description.
//! Stats its record lacks take NanoItem_t's defaults (FUN_10086142: attack and recharge 2s, range 20m, duration 10s ...).
//!
//! A weapon's damage per second (FUN_10033e0d, `N3Msg_GetDpsString` 0x10018daf): its damage over the attack and recharge
//! times the reader would have (Gamecode FUN_10067c74 and FUN_10067d92), "%.1f-%.1f(%.1f)".
//!
//! Not yet: the shop price (FUN_10031fda, a vendor's stat 74), the nano page's radius (`N3Msg_GetFormularRadius`, a field the
//! NanoItem_t computes when it loads), the armour, implant and nano pages' own sections (FUN_1003826b, FUN_10038586),
//! the shop price, fixtures, towers and buildings, and the stats interpolated between the low and high templates (the
//! requirements are).

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};
use std::collections::BTreeMap;

/// The identity type of a nano program, as `itemid://` names it.
pub const NANO_IDENTITY: u32 = 0xcf1b;

fn text(name: &str) -> String {
    crate::feedback::texts().and_then(|t| t.named(506, name)).map(str::to_string).unwrap_or_default()
}

fn ldb(category: i32, key: i32) -> Option<String> {
    crate::feedback::texts().and_then(|t| t.get(category, key)).map(str::to_string)
}

/// FUN_10031b5b: a labelled value, in a wrapped-indent block (`mode` 0) or followed by a line break (1).
pub fn section(label: &str, value: &str, mode: u8) -> String {
    let body = format!("<font color=CCInfoHeader>{label}</font><font color=CCInfoText>{value}</font>");
    match mode {
        0 => format!("<div indent=wrapped>{body}</div>"),
        1 => format!("{body}<br>"),
        _ => body,
    }
}

/// `itemref://<low>/<high>/<ql>`: the three numbers.
pub fn parse_itemref(url: &str) -> Option<(u32, u32, u32)> {
    let rest = url.get(10..).filter(|_| url[..10].eq_ignore_ascii_case("itemref://"))?;
    let mut parts = rest.split('/').map(|p| p.trim().parse::<i64>().ok());
    Some((parts.next()?? as u32, parts.next()?? as u32, parts.next()?? as u32))
}

/// What the sections read: the item's template, its stats and requirements at the quality, the reader's level.
struct Ctx<'a> {
    db: &'a ResourceDatabase,
    it: Item,
    data: Vec<u8>,
    actions: Vec<item::Action>,
    low: u32,
    ql: u32,
    level: u32,
    nano: bool,
    player: &'a BTreeMap<u32, i64>,
}

/// The stats a NanoItem_t starts with before its record is read (Gamecode FUN_10086142); stat 75, its nanoline, is its own
/// id.
fn nano_default(k: u32, id: u32) -> Option<i32> {
    Some(match k {
        1 | 0x1b => 10,
        8 => 1000,
        0 | 0x195 | 0x36 | 0x197 => 1,
        0x4b => id as i32,
        0xa8 => 100,
        0x178 => 0xcb,
        0x179 => 0xc9,
        0x17a => 0xca,
        0x126 | 0xd2 | 0xd3 => 200,
        0x4c => 4,
        0x11f => 20,
        0x4a => 1000,
        0x1ac | 0x19e | 0x16e | 0x1a3 | 0x169 => 49999,
        0x175 | 0x9c | 0x19 | 0x14c | 0x160 | 0xe | 0x48 | 0x222..=0x227 | 0x1a6 | 0x181 | 0x182 | 0x212 | 0x4f | 0x125 | 0xb7 | 0x19d => 0,
        _ => return None,
    })
}

/// FUN_10067c74 and FUN_10067d92's share of the reader's initiative skill: half of it up to 1200, a sixth above.
fn initiative_bonus(skill: i64) -> f32 {
    if skill < 1201 { skill as f32 * 0.5 } else { (skill - 1200) as f32 / 6.0 + 600.0 }
}

/// Gamecode FUN_10067c74 (`recharge` false) and FUN_10067d92 (true): a character's attack or recharge time with a weapon in
/// hundredths — the weapon's (stat 294, 210) plus 75, less a third of the initiative bonus (two thirds for the recharge) and
/// the character's stat 51, no less than the weapon's minimum (523, 524) or 100. `item` is the weapon's stat (`None` for one it
/// lacks), `player` the character's. (The character's +0x21c flag adds DAT_10156e58, which is 0.)
pub fn swing_time(item: &dyn Fn(u32) -> Option<i32>, player: &dyn Fn(u32) -> i64, recharge: bool) -> f32 {
    let init = item(440).unwrap_or(1234567890);
    let (base, skill, least) = if recharge {
        (item(210), if (0..=1200).contains(&init) { init } else { 118 }, item(524))
    } else {
        (item(294), init, item(523))
    };
    let bonus = initiative_bonus(player(skill as u32));
    let bonus = if recharge { bonus + bonus } else { bonus };
    let mut t = base.unwrap_or(1234567890) as f32 + 75.0 - bonus / 3.0 - player(51) as f32;
    if let Some(least) = least {
        t = t.max(least as f32);
    }
    if 100.0 < t { t } else { 100.0 }
}

impl Ctx<'_> {
    fn player(&self, k: u32) -> i64 {
        self.player.get(&k).copied().unwrap_or(0)
    }

    /// Gamecode FUN_10067c74 (`recharge` false) and FUN_10067d92 (true) for the reader and this weapon ([`swing_time`]).
    fn swing_time(&self, recharge: bool) -> f32 {
        swing_time(&|k| self.stat(k), &|k| self.player(k), recharge)
    }

    /// FUN_10033e0d, `N3Msg_GetDpsString`: damage over the reader's attack and recharge times.
    fn dps(&self) -> String {
        if self.stat(76) != Some(1) {
            return String::new();
        }
        let Some(min) = self.stat(286) else { return String::new() };
        let t = (self.swing_time(false) + self.swing_time(true)) * 0.01;
        let (max, crit) = (self.stat(285).unwrap_or(1234567890), self.stat(284).unwrap_or(1234567890));
        section(&text("Dps"), &format!("{:.1}-{:.1}({:.1})", min as f32 / t, max as f32 / t, crit as f32 / t), 0)
    }

    /// `N3Msg_GetSkill` on the item (`None` for the original's 1234567890, a stat it does not have); a nano program's
    /// missing stats take NanoItem_t's defaults.
    fn stat(&self, k: u32) -> Option<i32> {
        self.it.stat(k).map(|v| v as i32).or_else(|| if self.nano { nano_default(k, self.low) } else { None })
    }

    /// FUN_10031d7c: the name in the rarity's colour.
    fn header(&self) -> String {
        const RARITY: [&str; 6] = ["CCItemUnknown", "CCItemTrash", "CCItemNormal", "CCItemExotic", "CCItemQuest", "CCItemSocial"];
        let rarity = self.stat(688).unwrap_or(0).max(0) as usize;
        format!("<font color={}>{}</font><br>", RARITY.get(rarity).unwrap_or(&RARITY[0]), self.it.name)
    }

    /// FUN_100335df: NODROP, UNIQUE, playshift requirements.
    fn flags(&self) -> String {
        let flags = self.stat(0).unwrap_or(0) as u32;
        let mut f = String::new();
        for (bit, name) in [(0x400_0000, "NODROP"), (0x800_0000, "UNIQUE"), (0x4_0000, "PlayshiftReq")] {
            if flags & bit != 0 {
                f += &text(name);
            }
        }
        if f.is_empty() { f } else { format!("<div indent=wrapped><font color=CCInfoText>{f}</font></div>") }
    }

    /// FUN_10031c4b: a temporary item's time.
    fn temporary(&self) -> String {
        let Some(t) = self.stat(8).filter(|t| *t > 0) else { return String::new() };
        let s = t / 100;
        let time = if s < 61 { format!("{s}s") } else if s < 3601 { format!("{}m", s / 60) } else { format!("{}h", s / 3600) };
        section(&text("Temporary"), &time, 0)
    }

    /// FUN_1003198d: lootable in PvP.
    fn pvp(&self) -> String {
        if self.stat(30).unwrap_or(0) as u32 & 0x40 != 0 { text("ItemPvPLootable") } else { String::new() }
    }

    /// FUN_10033403: rarity and quality level.
    fn rarity(&self) -> String {
        let rarity = self.stat(688).unwrap_or(0).max(0);
        let mut s = section(&text("Rarity"), &ldb(2016, rarity).unwrap_or_default(), 0);
        if let Some(q) = self.stat(701).filter(|q| *q != 0).or(Some(self.ql as i32).filter(|q| *q != 0)) {
            let value = if self.stat(0).unwrap_or(0) as u32 & 0x20 != 0 { text("SPECIAL") } else { q.to_string() };
            s += &section(&text("QualityLevel"), &value, 0);
        }
        s
    }

    /// FUN_100331a7: the requirements.
    fn requirements(&self) -> String {
        super::criteria::use_by(&self.actions).map_or(String::new(), |r| section(&text("Requirements"), &r, 0))
    }

    /// FUN_10033255: the locations.
    fn location(&self) -> String {
        let base = match self.stat(76) {
            Some(1) => 300,
            Some(2) => 200,
            Some(3) | Some(5) => 400,
            _ => return String::new(),
        };
        let Some(slots) = self.stat(298) else { return String::new() };
        let names: String = (1..16).filter(|b| slots as u32 & (1 << b) != 0).filter_map(|b| ldb(1005, base + b)).map(|n| n + " ").collect();
        if names.is_empty() { names } else { section(&text("Loc"), &names, 0) }
    }

    /// FUN_10032c1d: the modifiers, `N3Msg_GetModifierString` of the worn list (14) and the used one (0).
    fn modifiers(&self) -> String {
        let lists = mao_formats::spell::lists(&self.data);
        let db = self.db;
        let nano_name = |id: u32| nano_name(db, id);
        let mut m = String::new();
        for list in [14, 0] {
            for (_, spells) in lists.iter().filter(|(l, _)| *l == list) {
                m += &super::spells::convert(spells, self.level, &nano_name);
            }
        }
        if m.is_empty() { m } else { section(&text("Modifier"), &m, 0) }
    }

    /// FUN_1003882b's charges.
    fn charges(&self) -> String {
        let can = self.stat(30).unwrap_or(0) as u32;
        let Some(c) = self.stat(26).filter(|c| *c >= 0 && self.low != 0xc788) else { return String::new() };
        let mut v = c.to_string();
        if can & 0x200 != 0 {
            v += &text("Stackable");
        }
        if can & 0x100_0000 == 0 {
            v += &text("Splitable");
        }
        section(&text("Charges"), &v, 0)
    }

    /// FUN_100332ff: the description.
    fn description(&self) -> String {
        if self.it.description.is_empty() {
            return String::new();
        }
        section(&text("Description"), &self.it.description.replace("\\n", "<br>"), 0)
    }

    /// FUN_1003732c's ammunition: the count in the colour of how full the clip is.
    fn ammo(&self) -> String {
        let kind = self.stat(420);
        if matches!(kind, None | Some(-1) | Some(0)) || self.low == 0xc788 {
            return String::new();
        }
        let (count, max) = (self.stat(26), self.stat(212));
        let mut s = String::new();
        if matches!(count, None | Some(-1)) || max.is_none() {
            if let Some(m) = max.filter(|m| *m != -1) {
                s += &section(&text("MaxAmmo"), &m.to_string(), 0);
            }
        }
        let (Some(count), Some(max)) = (count, max) else { return s };
        if max == -1 {
            return s;
        }
        let colour = if count == 0 {
            "CCRed"
        } else if count == max {
            "CCGreen"
        } else if count < max * 30 / 100 {
            "CCYellow"
        } else {
            "CCWhite"
        };
        s += &format!("<div indent=wrapped><font color={colour}>");
        let can = self.stat(30).unwrap_or(0) as u32;
        if can & 0x200 == 0 {
            s += &super::spells::printf_ints(&text("AmmoCntOf"), &[count, max]);
        } else {
            s += &super::spells::printf_ints(&text("AmmoCnt"), &[count]);
            if can & 0x100_0000 == 0 {
                s += &text("Splitable");
            }
        }
        s + "</font></div>"
    }

    /// FUN_1003732c's ammunition type.
    fn ammo_type(&self) -> String {
        let Some(kind) = self.stat(420).filter(|k| *k != -1 && *k != 0) else { return String::new() };
        let name = match kind {
            1 => text("EnergyAmmo"),
            2 => text("Bullets"),
            3 => text("FlamethrowerAmmo"),
            4 => text("ShotgunShells"),
            5 => text("Arrows"),
            6 => text("GrenadeAmmo"),
            7 => text("BloodAmmo"),
            8 => text("BazookaAmmo"),
            10 => text("SelfSupplied"),
            k => format!("Error {k}"),
        };
        section(&text("Ammotype"), &name, 0)
    }

    /// FUN_10032324, `N3Msg_GetDamageString` (0x1001a9bc): a weapon's minimum, maximum and critical damage.
    fn damage(&self) -> String {
        if self.stat(76) != Some(1) {
            return String::new();
        }
        let Some(min) = self.stat(286) else { return String::new() };
        let (max, crit) = (self.stat(285).unwrap_or(0), self.stat(284).unwrap_or(0));
        section(&text("Damage"), &format!("{min}-{max}({crit})"), 0)
    }

    /// FUN_10032582: the attack and recharge times (stats 294 and 210, hundredths of a second; a nano program's 10 is none).
    fn speed(&self) -> String {
        let at = self.stat(294).filter(|v| *v != 0 && !(self.nano && *v == 10));
        let re = self.stat(210).filter(|v| *v != 0 && !(self.nano && *v == 10));
        let value = match (at, re) {
            (None, None) => return String::new(),
            (Some(a), Some(r)) => super::spells::printf_floats(&text("SpeedAtRe"), &[a as f32 / 100.0, r as f32 / 100.0]),
            (Some(a), None) => super::spells::printf_floats(&text("SpeedAt"), &[a as f32 / 100.0]),
            (None, Some(r)) => super::spells::printf_floats(&text("SpeedRe"), &[r as f32 / 100.0]),
        };
        section(&text("Speed"), &value, 0)
    }

    /// FUN_100324b1: the equip delay (stat 211).
    fn equip_delay(&self) -> String {
        self.stat(211).map_or(String::new(), |d| section(&text("EquipDelay"), &format!("{:.2}s", d as f32 / 100.0), 0))
    }

    /// FUN_100323ce: the initiative skill (stat 440).
    fn initiative(&self) -> String {
        let name = match self.stat(440) {
            Some(118) => text("InitiativeMelee"),
            Some(119) => text("InitiativeRanged"),
            Some(120) => text("InitiativePhysical"),
            Some(149) => text("InitiativeNano"),
            _ => return String::new(),
        };
        section(&text("Initiative"), &name, 0)
    }

    /// FUN_10032275: the range (stat 287).
    fn range(&self) -> String {
        self.stat(287).map_or(String::new(), |r| section(&text("Range"), &format!("{r}m"), 0))
    }

    /// FUN_1003732c's dual wield (stats 101 and 134) and martial arts combined attack (stat 100).
    fn dual_wield(&self) -> String {
        let mut s = String::new();
        let line = |stat: u32, label: &str, v: i32| section(&text(label), &format!("{} {v}", super::criteria::stat_name(stat)), 0);
        if let Some(v) = self.stat(101) {
            s += &line(101, "DualWield", v);
        }
        let ranged = self.stat(134);
        if let Some(v) = ranged {
            s += &line(134, "DualWield", v);
        }
        if let (Some(v), Some(r)) = (self.stat(100), ranged) {
            if r != 0 {
                s += &line(100, "MAcombinedAttack", v);
            }
        }
        s
    }

    /// FUN_1003732c's special attacks (stat 30's bits).
    fn specials(&self) -> String {
        let Some(can) = self.stat(30) else { return String::new() };
        let mut names = String::new();
        for (bit, name) in [(0x800, "Burst"), (0x1000, "FlingShot"), (0x2000, "FullAuto"), (0x4000, "Snipe"), (0x8000, "BowSpecial"), (0x2_0000, "SneakAttack"), (0x4_0000, "FastAttack")] {
            if can as u32 & bit != 0 {
                names += &text(name);
            }
        }
        if names.is_empty() { names } else { section(&text("Special"), &names, 0) }
    }

    /// FUN_1003732c's maximum beneficial skill (stat 538).
    fn max_beneficial(&self) -> String {
        self.stat(538).filter(|v| *v != 0).map_or(String::new(), |v| section(&text("MaxBeneficialSkill"), &v.to_string(), 0))
    }

    /// FUN_10032ac9 and FUN_10032b73 (`N3Msg_GetSkillToAttackString`, `GetSkillToDefendString`): skill list 12 or 13.
    fn skills(&self, list: u32, label: &str) -> String {
        let lists = item::skill_lists(&self.data);
        let Some((_, skills)) = lists.iter().find(|(l, s)| *l == list && !s.is_empty()) else { return String::new() };
        let v: String = skills.iter().map(|&(stat, share)| format!("{} {share}% ", super::criteria::stat_name(stat))).collect();
        section(&text(label), &v, 0)
    }

    /// FUN_100330f9 (`N3Msg_GetSkillToUseString`): skill list 3, none for a nano program.
    fn skill_to_use(&self) -> String {
        if self.nano {
            return String::new();
        }
        let lists = item::skill_lists(&self.data);
        let Some((_, skills)) = lists.iter().find(|(l, s)| *l == 3 && !s.is_empty()) else { return String::new() };
        let mut v = String::new();
        for &(stat, value) in skills {
            v += &super::criteria::stat_name(stat);
            v.push(' ');
            if value != 1 {
                v += &if value < 0 { format!(" ({value}) ") } else { format!(" {value} ") };
            }
        }
        section(&text("SkillToUse"), &v, 0)
    }

    /// FUN_10033c4b and FUN_10033d2c: a spell list's modifier string on a new line (on death 20, on cancel 28).
    fn spell_list(&self, list: u32, label: &str) -> String {
        let lists = mao_formats::spell::lists(&self.data);
        let db = self.db;
        let nano_name = |id: u32| nano_name(db, id);
        let m: String = lists.iter().filter(|(l, _)| *l == list).map(|(_, spells)| super::spells::convert(spells, self.level, &nano_name)).collect();
        if m.is_empty() { m } else { section(&text(label), &format!("\n{m}"), 0) }
    }

    /// FUN_1003683d's nano cost, against the reader's nano (stat 221) and cost modifier.
    fn nano_cost(&self, player: &BTreeMap<u32, i64>) -> String {
        let base = self.stat(407).unwrap_or(0) as i64;
        let cost = (cost_modifier(player) as f64 * base as f64 / 100.0) as i64;
        let nano = player.get(&221).copied().unwrap_or(i64::MAX);
        let colour = |v: i64| if nano < v { "<font color=CCRed>" } else { "<font color=CCInfoText>" };
        format!("{}{}{cost}</font>{} ({base})</font>", text("NanoCost"), colour(cost), colour(base))
    }

    /// FUN_1003683d's NCU cost (stat 54, 999 for none).
    fn ncu(&self) -> String {
        self.stat(54).filter(|v| *v != 999).map_or(String::new(), |v| section(&text("NCUcost"), &v.to_string(), 0))
    }

    /// FUN_10033a35: the school (stat 405).
    fn school(&self) -> String {
        let v = self.stat(405).unwrap_or(0);
        let name = match v {
            1 => "NanoSchool_Combat",
            2 => "NanoSchool_Medical",
            3 => "NanoSchool_Prot",
            4 => "NanoSchool_Psi",
            5 => "NanoSchool_Space",
            _ => return section(&text("School"), &format!("unknown {}", v - 1), 0),
        };
        let school = crate::feedback::texts().and_then(|t| t.named(10000, name)).map(str::to_string).unwrap_or_default();
        section(&text("School"), &school, 0)
    }

    /// FUN_10033bae: the stacking order (stat 551).
    fn stacking_order(&self) -> String {
        self.stat(551).map_or(String::new(), |v| section(&text("StackingOrder"), &v.to_string(), 0))
    }

    /// FUN_10033752: a nanoline (named from category 2009), unless none or the program's own id.
    fn nanoline(&self, stat: u32) -> String {
        match self.stat(stat) {
            Some(v) if v != 0 && v as u32 != self.low => section(&text("Nanoline"), &ldb(2009, v).unwrap_or_default(), 0),
            _ => String::new(),
        }
    }

    /// `N3Msg_GetBuffTotalTime` (FUN_1008651e): the time its repeating spells run when it has some (FUN_100a49b1, the
    /// longest of a 0xcf0e's hits times its delay and a 0xcf16's delay plus its stat 25), else stat 8; hundredths, shown
    /// as h:m:s.
    fn duration(&self) -> String {
        let mut longest = 0;
        for (_, spells) in mao_formats::spell::lists(&self.data) {
            for sp in spells {
                let t = match sp.kind {
                    0xcf16 => sp.stat(4).abs() + sp.stat(0x19),
                    0xcf0e => sp.stat(3) * sp.stat(4).abs(),
                    _ => 0,
                };
                longest = longest.max(t);
            }
        }
        let t = if longest > 0 { longest } else { self.stat(8).unwrap_or(0) };
        let t = t.max(0) / 100;
        section(&text("Duration"), &format!("{:02}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60), 0)
    }

    /// FUN_1003283f: the local cooldown (stat 254), from a second.
    fn local_cooldown(&self) -> String {
        self.stat(254).filter(|v| *v >= 100).map_or(String::new(), |v| section(&text("LocalCooldown"), &format!("{}.00s", v / 100), 0))
    }

    /// FUN_1003290b: each nanoline's cooldown (stats 643-648 against stats 75 and 546-550).
    fn nanoline_cooldowns(&self) -> String {
        let mut s = String::new();
        for (i, line) in [75, 546, 547, 548, 549, 550].into_iter().enumerate() {
            if let (Some(cd), Some(l)) = (self.stat(643 + i as u32), self.stat(line)) {
                if l > 0 && cd > 99 {
                    s += &section(&text("NanolineCooldown"), &format!("{} for {}.00s", ldb(2009, l).unwrap_or_default(), cd / 100), 0);
                }
            }
        }
        s
    }

    /// FUN_1003732c's damage type (stat 436, a stat's name).
    fn damage_type(&self) -> String {
        self.stat(436).map_or(String::new(), |v| section(&text("DamageType"), &super::criteria::stat_name(v as u32), 0))
    }
}

/// A nano program's (else an item's) name, for the spells that upload or cast one.
pub(crate) fn nano_name(db: &ResourceDatabase, id: u32) -> Option<String> {
    let d = db.get(Id { kind: item::NANO_TEMPLATE, instance: id }).ok().flatten().or_else(|| db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten())?;
    Item::parse(&d).map(|i| i.name)
}

/// `N3Msg_GetNanoCostModifier` (Gamecode FUN_1004e925): the reader's stat 318 (100 when it is not known), no lower than
/// the breed's (stat 4) 50 for solitus and opifex, 45 for nanomage, 55 for atrox.
fn cost_modifier(player: &BTreeMap<u32, i64>) -> i64 {
    let floor = match player.get(&4) {
        Some(1) | Some(2) => 50,
        Some(3) => 45,
        Some(4) => 55,
        _ => 0,
    };
    player.get(&318).copied().unwrap_or(100).max(floor)
}

/// The page of a nano program (FUN_1003683d).
pub fn nano_page(db: &ResourceDatabase, id: u32, player: &BTreeMap<u32, i64>) -> Option<String> {
    let data = db.get(Id { kind: item::NANO_TEMPLATE, instance: id }).ok().flatten()?;
    let it = Item::parse(&data)?;
    let actions = item::actions(&data);
    let level = player.get(&crate::stats::stat::LEVEL).copied().unwrap_or(1).max(1) as u32;
    let c = Ctx { db, it, data, actions, low: id, ql: 0, level, nano: true, player };
    let mut parts = vec![c.header(), c.nano_cost(player), c.ncu(), c.school(), c.stacking_order()];
    parts.extend([75, 546, 547, 548, 549, 550].map(|s| c.nanoline(s)));
    parts.extend([
        c.duration(), c.range(), c.damage(), c.speed(), c.local_cooldown(), c.nanoline_cooldowns(), c.skills(12, "AtkSkills"),
        c.skills(13, "DefSkills"), c.modifiers(), c.spell_list(20, "OnDeath"), c.spell_list(28, "OnCancel"), c.skill_to_use(),
        c.requirements(), c.description(),
    ]);
    Some(parts.concat())
}

/// The page of an item at a quality (FUN_10038f6b by item class, stat 76): a weapon's (1, FUN_1003732c), armour's (2,
/// FUN_1003826b) and implants' (3 and 5, FUN_10038586), else the general one (FUN_1003882b).
pub fn item_page(db: &ResourceDatabase, low: u32, high: u32, ql: u32, player: &BTreeMap<u32, i64>) -> Option<String> {
    let level = player.get(&crate::stats::stat::LEVEL).copied().unwrap_or(1).max(1) as u32;
    let template = |id: u32| db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten();
    let data = template(low).or_else(|| template(high))?;
    let it = Item::parse(&data)?;
    // (`OnLookatItem`'s tips, by the item's name)
    crate::tips::trigger("OnLookatItem", &it.name);
    let entry = mao_net::inventory::Entry { slot: 0, flags: 0, count: 1, container: (0, 0), low_id: low, high_id: high, quality: ql, extra: 0 };
    let actions = crate::derived::actions(db, &entry);
    let c = Ctx { db, it, data, actions, low, ql, level, nano: false, player };
    let parts: Vec<String> = if c.stat(76) == Some(1) {
        vec![
            c.header(), c.flags(), c.temporary(), c.pvp(), c.ammo(), c.rarity(), c.requirements(), c.ammo_type(), c.damage(), c.dps(), c.speed(),
            c.equip_delay(), c.initiative(), c.location(), c.range(), c.dual_wield(), c.specials(), c.max_beneficial(),
            c.skills(12, "AtkSkills"), c.skills(13, "DefSkills"), c.modifiers(), c.skill_to_use(), c.damage_type(), c.description(),
        ]
    } else if matches!(c.stat(76), Some(2) | Some(3) | Some(5)) {
        // (armour FUN_1003826b, implants FUN_10038586: no charges; armour's "Fabric" shows only in a debug input mode)
        vec![c.header(), c.flags(), c.temporary(), c.pvp(), c.rarity(), c.requirements(), c.location(), c.modifiers(), c.description()]
    } else {
        vec![c.header(), c.flags(), c.temporary(), c.pvp(), c.rarity(), c.requirements(), c.location(), c.modifiers(), c.charges(), c.description()]
    };
    Some(parts.concat())
}

/// `itemid://<type>/<instance>`: the identity.
pub fn parse_itemid(url: &str) -> Option<(u32, u32)> {
    let rest = url.get(9..).filter(|_| url[..9].eq_ignore_ascii_case("itemid://"))?;
    let parts: Vec<u32> = rest.split('/').map(|p| p.trim().parse::<i64>().ok().map(|v| v as u32)).collect::<Option<_>>()?;
    (parts.len() == 2).then(|| (parts[0], parts[1]))
}

/// The page for one of the URLs the game builds (`itemref://`, a nano program's `itemid://`), for a reader with these
/// stats; `None` when it is not one of those.
pub fn page(db: &ResourceDatabase, url: &str, player: &BTreeMap<u32, i64>) -> Option<String> {
    if let Some((kind, id)) = parse_itemid(url) {
        return if kind == NANO_IDENTITY { nano_page(db, id, player) } else { None };
    }
    let (low, high, ql) = parse_itemref(url)?;
    item_page(db, low, high, ql, player)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MAO_GAME`: a page of a wearable item with requirements is built (`MAO_ITEM=<low>/<high>/<ql>` prints that item's
    /// page instead, for `MAO_VIEW_TEXT`).
    #[test]
    fn builds_a_page_from_the_game() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        crate::theme::set_game_dir(&game.to_string_lossy());
        let db = ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        if let Ok(id) = std::env::var("MAO_ITEM_WORDS") {
            let data = db.get(Id { kind: item::TEMPLATE, instance: id.parse().unwrap() }).ok().flatten().unwrap();
            let at = data.windows(4).position(|w| w == 0xcf35u32.to_le_bytes()).unwrap_or(0);
            let from = at.saturating_sub(48);
            let words: Vec<String> = data[from..(at + 160).min(data.len())].chunks(4).map(|c| format!("{:x}", u32::from_le_bytes([c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0), *c.get(3).unwrap_or(&0)]))).collect();
            println!("len {} spell at {at}: {}", data.len(), words.join(" "));
            let mut p = at;
            while let Some((s, end)) = mao_formats::spell::read(&data, p) {
                println!("{:x} {:?} -> {end}", s.kind, s.stats);
                p = end;
            }
            let w = |o: usize| data.get(o..o + 4).map(|b| format!("{:x}", u32::from_le_bytes(b.try_into().unwrap()))).unwrap_or_default();
            println!("stopped at {p}: {} {} {} {} {} {}", w(p), w(p + 4), w(p + 8), w(p + 12), w(p + 16), w(p + 20));
            return;
        }
        if let Ok(spec) = std::env::var("MAO_ITEM") {
            let page = page(&db, &format!("itemref://{spec}"), &BTreeMap::from([(crate::stats::stat::LEVEL, 200)])).unwrap();
            println!("{page}");
            return;
        }
        let found = (200_000..203_000).find_map(|id| {
            let data = db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten()?;
            let it = Item::parse(&data)?;
            let has = item::actions(&data).iter().any(|a| a.kind == 6 && !a.criteria.is_empty());
            has.then(|| (id, it.name))
        });
        // (and a weapon's page: item class 1)
        let weapon = (200_000..210_000).find_map(|id| {
            let data = db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten()?;
            let it = Item::parse(&data)?;
            (it.stat(76) == Some(1) && it.stat(286).is_some() && it.stat(420).is_some_and(|a| a != 0)).then_some(id)
        });
        let level200 = BTreeMap::from([(crate::stats::stat::LEVEL, 200)]);
        if let Some(w) = weapon {
            let page = item_page(&db, w, w, 100, &level200).unwrap();
            println!("weapon {w}\n{page}");
            assert!(page.contains("Damage") && page.contains("Dps: "));
        }
        // (and a nano program's)
        let nano = (95_000..96_000).find(|&id| db.get(Id { kind: item::NANO_TEMPLATE, instance: id }).ok().flatten().is_some()).unwrap();
        let page = page(&db, &format!("itemid://{NANO_IDENTITY}/{nano}"), &BTreeMap::new()).unwrap();
        println!("nano {nano}\n{page}");
        assert!(page.contains("Nano Cost") && page.contains("Duration"));
        let (id, name) = found.expect("an item with wear requirements");
        let page = item_page(&db, id, id, 100, &level200).unwrap();
        println!("{id} {name}\n{page}");
        assert!(page.contains(&name));
        assert!(page.contains("CCInfoHeader"));
    }

    #[test]
    fn reads_an_itemref() {
        assert_eq!(parse_itemref("itemref://246660/246661/200"), Some((246660, 246661, 200)));
        assert_eq!(parse_itemref("text://x"), None);
        assert_eq!(parse_itemid("itemid://53019/95000"), Some((NANO_IDENTITY, 95000)));
        assert_eq!(parse_itemid("itemid://1/2/3/4"), None);
    }
}
