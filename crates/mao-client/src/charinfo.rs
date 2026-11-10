//! A character's page in the info view, as GUI.dll builds it (FUN_1003482c), from what the server's info packet and the
//! character's own data say.
//!
//! The headline (`CCInfoHeadline`) is the first name, the name in quotes and the last name. Then a labelled line each
//! (FUN_10031b5b, the labels text.mdb 506 by name): `Breed`, `Gender`, `ProfessionTitle` (the title and its level as
//! " (TitleLevel %d)"), `Profession` (`NotChosenYet` for none), `Level` (with `ShadowLevel`, the levels past 200, beside
//! it for levels 201 to 220), `NumInvadersKilled`, `KilledByInvaders`, `DefenderRank`, `Alignment` (the side's name,
//! text.mdb 2005), the organization and the rank in it, a break, the duel wins and losses and the solo and team kills,
//! and for another character a link that inspects its equipment (`chatcmd:///inspect <name>`).
//!
//! Not here: the profession title's name and the defender rank's (the number is shown; the original asks Gamecode
//! for the words), the side experience bonus, the city, the PvP ranks' names, the own character's tokens and veteran
//! points, the specializations and the account's items.

use crate::gui::iteminfo::section;

/// What a character's page is made of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Character {
    pub name: String,
    pub first: String,
    pub last: String,
    pub breed: i32,
    pub sex: i32,
    pub profession: i32,
    pub title_level: i64,
    pub level: i64,
    /// The side (stat 33), when known.
    pub side: Option<i64>,
    pub org: Option<String>,
    pub rank: Option<String>,
    pub invaders_killed: i64,
    pub killed_by_invaders: i64,
    pub ai_level: i64,
    pub duel_wins: i64,
    pub duel_losses: i64,
    pub solo_kills: i64,
    pub team_kills: i64,
    /// Whether it is the client's own character (no inspect link).
    pub own: bool,
}

/// A label of the page: text.mdb 506 by name, or this client's own word.
fn label(name: &str) -> String {
    let own = match name {
        "Breed" => "Breed: ",
        "Gender" => "Gender: ",
        "ProfessionTitle" => "Title: ",
        "Profession" => "Profession: ",
        "NotChosenYet" => "None yet.",
        "Level" => "Level: ",
        "ShadowLevel" => "Shadow level: ",
        "NumInvadersKilled" => "Invaders killed: ",
        "KilledByInvaders" => "Killed by invaders: ",
        "DefenderRank" => "Defender rank: ",
        "Alignment" => "Side: ",
        "Clan" => "Organization: ",
        "ClanTitle" => "Rank: ",
        "DuelKills" => "Duels won: ",
        "DuelDeaths" => "Duels lost: ",
        "SoloKills" => "Solo kills: ",
        "TeamKills" => "Team kills: ",
        _ => name,
    };
    let text = crate::gui::iteminfo::text(name);
    if text.is_empty() { own.to_string() } else { text }
}

/// The level line: the level, and for levels 201 to 220 the shadow level beside it (FUN_1003482c: `level - 0xc9 < 0x14`).
fn level(level: i64) -> String {
    let mut out = section(&label("Level"), &level.to_string(), 2);
    if (201..=220).contains(&level) {
        out += "&nbsp;";
        out += &section(&label("ShadowLevel"), &(level - 200).to_string(), 1);
    } else {
        out += "<br>";
    }
    out
}

/// The page's markup.
pub fn page(c: &Character) -> String {
    let mut name = String::new();
    if !c.first.is_empty() {
        name += &format!("{} ", c.first);
    }
    name += &format!("\"{}\"", c.name);
    if !c.last.is_empty() {
        name += &format!(" {}", c.last);
    }
    let mut out = format!("<font color=CCInfoHeadline>{name}</font>\n");
    let breed = crate::launcher::breed(c.breed);
    if !breed.is_empty() {
        out += &section(&label("Breed"), breed, 0);
    }
    let sex = crate::launcher::sex(c.sex);
    if !sex.is_empty() {
        out += &section(&label("Gender"), sex, 0);
    }
    if c.title_level > 0 {
        out += &section(&label("ProfessionTitle"), &format!(" (TitleLevel {})", c.title_level), 0);
    }
    let profession = crate::launcher::profession(c.profession);
    let profession = if c.profession == 0 || c.profession == 0xff || profession.is_empty() { label("NotChosenYet") } else { profession.to_string() };
    out += &section(&label("Profession"), &profession, 0);
    out += &level(c.level);
    out += &section(&label("NumInvadersKilled"), &c.invaders_killed.to_string(), 0);
    out += &section(&label("KilledByInvaders"), &c.killed_by_invaders.to_string(), 0);
    out += &section(&label("DefenderRank"), &c.ai_level.to_string(), 0);
    if let Some(side) = c.side {
        let side = crate::feedback::texts().and_then(|t| t.get(2005, side as i32)).map_or_else(|| ["neutral", "clan", "omni"].get(side as usize).copied().unwrap_or("-").to_string(), str::to_string);
        out += &section(&label("Alignment"), &side, 0);
    }
    if let Some(org) = c.org.as_deref().filter(|o| !o.is_empty()) {
        out += &section(&label("Clan"), org, 0);
        if let Some(rank) = c.rank.as_deref().filter(|r| !r.is_empty()) {
            out += &section(&label("ClanTitle"), &format!(" {rank}"), 0);
        }
    }
    out += "<br>";
    for (name, value) in [("DuelKills", c.duel_wins), ("DuelDeaths", c.duel_losses), ("SoloKills", c.solo_kills), ("TeamKills", c.team_kills)] {
        out += &section(&label(name), &format!(" {value}"), 0);
    }
    if !c.own {
        out += &format!("<br><a href=\"chatcmd:///inspect {}\">Inspect Equipment</a>", c.name);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_follows_the_originals_order() {
        let c = Character { name: "Zed".into(), first: "Zachary".into(), last: "Edwards".into(), breed: 2, sex: 3, profession: 4, title_level: 3, level: 205, side: Some(1), org: Some("The Guild".into()), rank: Some("Member".into()), duel_wins: 2, ..Default::default() };
        let page = page(&c);
        if std::env::var_os("MAO_PRINT").is_some() {
            println!("{page}");
        }
        assert!(page.starts_with("<font color=CCInfoHeadline>Zachary \"Zed\" Edwards</font>"));
        let order = ["Opifex", "female", "(TitleLevel 3)", "Fixer", ">205<", ">5<", "The Guild", "Member", "Inspect Equipment"];
        let mut at = 0;
        for word in order {
            let found = page[at..].find(word).unwrap_or_else(|| panic!("{word} is missing or out of order in {page}"));
            at += found;
        }
        // (no shadow level under 201, no inspect link for the client's own character, no title line without a title)
        let own = super::page(&Character { name: "Me".into(), level: 60, own: true, ..Default::default() });
        assert!(!own.contains("Inspect") && own.contains(">60<") && !own.contains("TitleLevel") && own.contains("\"Me\""));
        assert_eq!(level(200).matches("font color=CCInfoHeader").count(), 1);
        assert_eq!(level(201).matches("font color=CCInfoHeader").count(), 2);
    }
}
