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
//! Not yet: the weapon, armour, implant and nano pages' own sections (FUN_1003732c, FUN_1003826b, FUN_10038586,
//! FUN_1003683d), the shop price, fixtures, towers and buildings, and the
//! stats interpolated between the low and high templates (the requirements are).

use mao_formats::item::{self, Item};
use mao_formats::resource::{Id, ResourceDatabase};

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

/// The page of an item at a quality.
pub fn item_page(db: &ResourceDatabase, low: u32, high: u32, ql: u32, level: u32) -> Option<String> {
    let template = |id: u32| db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten();
    let data = template(low).or_else(|| template(high))?;
    let it = Item::parse(&data)?;
    let stat = |k: u32| it.stat(k).map(|v| v as i32);
    let entry = mao_net::inventory::Entry { slot: 0, flags: 0, count: 1, container: (0, 0), low_id: low, high_id: high, quality: ql, extra: 0 };
    let actions = crate::derived::actions(db, &entry);
    let mut page = String::new();

    // the name in the rarity's colour
    const RARITY: [&str; 6] = ["CCItemUnknown", "CCItemTrash", "CCItemNormal", "CCItemExotic", "CCItemQuest", "CCItemSocial"];
    let rarity = stat(688).unwrap_or(0).max(0) as usize;
    page += &format!("<font color={}>{}</font><br>", RARITY.get(rarity).unwrap_or(&RARITY[0]), it.name);

    // NODROP, UNIQUE, playshift requirements
    let flags = stat(0).unwrap_or(0) as u32;
    let mut f = String::new();
    for (bit, name) in [(0x400_0000, "NODROP"), (0x800_0000, "UNIQUE"), (0x4_0000, "PlayshiftReq")] {
        if flags & bit != 0 {
            f += &text(name);
        }
    }
    if !f.is_empty() {
        page += &format!("<div indent=wrapped><font color=CCInfoText>{f}</font></div>");
    }

    // a temporary item's time
    if let Some(t) = stat(8).filter(|t| *t > 0) {
        let s = t / 100;
        let time = if s < 61 { format!("{s}s") } else if s < 3601 { format!("{}m", s / 60) } else { format!("{}h", s / 3600) };
        page += &section(&text("Temporary"), &time, 0);
    }

    // looted in PvP
    let can = stat(30).unwrap_or(0) as u32;
    if can & 0x40 != 0 {
        page += &text("ItemPvPLootable");
    }

    // rarity and quality level
    page += &section(&text("Rarity"), &ldb(2016, rarity as i32).unwrap_or_default(), 0);
    if let Some(q) = stat(701).filter(|q| *q != 0).or(Some(ql as i32).filter(|q| *q != 0)) {
        let value = if flags & 0x20 != 0 { text("SPECIAL") } else { q.to_string() };
        page += &section(&text("QualityLevel"), &value, 0);
    }

    // requirements
    if let Some(req) = super::criteria::use_by(&actions) {
        page += &section(&text("Requirements"), &req, 0);
    }

    // locations
    let base = match stat(76) {
        Some(1) => Some(300),
        Some(2) => Some(200),
        Some(3) | Some(5) => Some(400),
        _ => None,
    };
    if let (Some(base), Some(slots)) = (base, stat(298)) {
        let names: String = (1..16).filter(|b| slots as u32 & (1 << b) != 0).filter_map(|b| ldb(1005, base + b)).map(|n| n + " ").collect();
        if !names.is_empty() {
            page += &section(&text("Loc"), &names, 0);
        }
    }

    // modifiers (FUN_10032c1d: `N3Msg_GetModifierString` of the worn list, 14, and the used one, 0)
    let lists = mao_formats::spell::lists(&data);
    let nano_name = |id: u32| {
        let d = db.get(Id { kind: item::NANO_TEMPLATE, instance: id }).ok().flatten().or_else(|| db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten())?;
        Item::parse(&d).map(|i| i.name)
    };
    let mut modifiers = String::new();
    for list in [14, 0] {
        for (_, spells) in lists.iter().filter(|(l, _)| *l == list) {
            modifiers += &super::spells::convert(spells, level, &nano_name);
        }
    }
    if !modifiers.is_empty() {
        page += &section(&text("Modifier"), &modifiers, 0);
    }

    // charges
    if let Some(c) = stat(26).filter(|c| *c >= 0 && low != 0xc788) {
        let mut v = c.to_string();
        if can & 0x200 != 0 {
            v += &text("Stackable");
        }
        if can & 0x100_0000 == 0 {
            v += &text("Splitable");
        }
        page += &section(&text("Charges"), &v, 0);
    }

    // description
    if !it.description.is_empty() {
        let d = it.description.replace("\\n", "<br>");
        page += &section(&text("Description"), &d, 0);
    }
    Some(page)
}

/// The page for one of the URLs the game builds (`itemref://`); `None` when it is not one of those.
pub fn page(db: &ResourceDatabase, url: &str, level: u32) -> Option<String> {
    let (low, high, ql) = parse_itemref(url)?;
    item_page(db, low, high, ql, level)
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
            let page = page(&db, &format!("itemref://{spec}"), 200).unwrap();
            println!("{page}");
            return;
        }
        let found = (200_000..203_000).find_map(|id| {
            let data = db.get(Id { kind: item::TEMPLATE, instance: id }).ok().flatten()?;
            let it = Item::parse(&data)?;
            let has = item::actions(&data).iter().any(|a| a.kind == 6 && !a.criteria.is_empty());
            has.then(|| (id, it.name))
        });
        let (id, name) = found.expect("an item with wear requirements");
        let page = item_page(&db, id, id, 100, 200).unwrap();
        println!("{id} {name}\n{page}");
        assert!(page.contains(&name));
        assert!(page.contains("CCInfoHeader"));
    }

    #[test]
    fn reads_an_itemref() {
        assert_eq!(parse_itemref("itemref://246660/246661/200"), Some((246660, 246661, 200)));
        assert_eq!(parse_itemref("text://x"), None);
    }
}
