//! A character's page in the info view, as GUI.dll builds it (FUN_1003482c), from what the server's info packet and the
//! character's own data say.
//!
//! The headline (`CCInfoHeadline`) is the first name, the name in quotes and the last name. Then a labelled line each
//! (FUN_10031b5b, the labels text.mdb 506 by name): `Breed`, `Gender`, `ProfessionTitle` (the title's name,
//! `GetTitleStr` Gamecode 0x10016185: text.mdb 1060 by profession * 100 + title level, then its level as
//! " (TitleLevel %d)"; always, whatever the level), `Profession` (`NotChosenYet` for none), `Level` (with `ShadowLevel`,
//! the levels past 200, beside it for levels 201 to 220), `NumInvadersKilled`, `KilledByInvaders`, `DefenderRank` (the
//! rank's name, `N3Msg_GetAlienLevelString` Gamecode 0x10016d9b: text.mdb 1169 by alien level * 100), `Alignment` (the side's name,
//! text.mdb 2005), the organization and the rank in it, a break, the duel wins and losses and the solo and team kills,
//! and for another character a link that inspects its equipment (`chatcmd:///inspect <name>`).
//!
//! Not here: the side experience bonus, the city, the PvP ranks' names, the own character's tokens and veteran
//! points, the specializations and the account's items.

use crate::gui::iteminfo::section;

/// What a character's page is made of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Character {
    /// The character's number (its identity's instance): the inspect link carries it.
    pub id: u32,
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

/// A profession title's name (`GetTitleStr`, Gamecode 0x10016185: text.mdb 1060 by `profession * 100 + title level`);
/// empty when the game has none.
fn title_name(profession: i32, title_level: i64) -> String {
    crate::feedback::texts().and_then(|t| t.get(1060, profession * 100 + title_level as i32)).unwrap_or_default().to_string()
}

/// A defender rank's name (`N3Msg_GetAlienLevelString`, Gamecode 0x10016d9b: text.mdb 1169 by `alien level * 100`); the
/// number when the game has no text.
fn defender_rank(alien_level: i64) -> String {
    crate::feedback::texts().and_then(|t| t.get(1169, (alien_level * 100) as i32)).map_or_else(|| alien_level.to_string(), str::to_string)
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
    out += &section(&label("ProfessionTitle"), &format!("{} (TitleLevel {})", title_name(c.profession, c.title_level), c.title_level), 0);
    let profession = crate::launcher::profession(c.profession);
    let profession = if c.profession == 0 || c.profession == 0xff || profession.is_empty() { label("NotChosenYet") } else { profession.to_string() };
    out += &section(&label("Profession"), &profession, 0);
    out += &level(c.level);
    out += &section(&label("NumInvadersKilled"), &c.invaders_killed.to_string(), 0);
    out += &section(&label("KilledByInvaders"), &c.killed_by_invaders.to_string(), 0);
    out += &section(&label("DefenderRank"), &defender_rank(c.ai_level), 0);
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
    // (FUN_1003482c: the link is the command with the character's number, which `/inspect` reads with `strtoul`)
    if !c.own {
        out += &format!("<br><a href=\"chatcmd:///inspect {}\">Inspect Equipment</a>", c.id);
    }
    out
}

/// The characters whose equipment was asked for (`/inspect <number>`) and not yet sent for.
static ASKED: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

/// `/inspect <number>`: the net thread sends the request (`mao_net::n3::inspect_request`).
pub fn ask_equipment(id: u32) {
    ASKED.lock().unwrap_or_else(|e| e.into_inner()).push(id);
}

pub fn take_equipment_asks() -> Vec<u32> {
    std::mem::take(&mut *ASKED.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MAO_GAME`: the game has the words `GetTitleStr` and `N3Msg_GetAlienLevelString` read: a title for every
    /// profession's title levels 1 to 7 (1060, profession * 100 + level) and a name for defender ranks 0 to 30 (1169,
    /// rank * 100).
    #[test]
    fn titles_and_defender_ranks_are_the_games_texts() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let texts = mao_formats::ldb::Texts::parse(&std::fs::read(std::path::Path::new(&game).join("cd_image/text/text.mdb")).unwrap()).unwrap();
        for level in 1..=7 {
            assert!(texts.get(1060, 100 + level).is_some_and(|t| !t.is_empty()), "title {level}");
        }
        for rank in 0..=30 {
            assert!(texts.get(1169, rank * 100).is_some_and(|t| !t.is_empty()), "rank {rank}");
        }
    }

    #[test]
    fn the_page_follows_the_originals_order() {
        let c = Character { id: 77, name: "Zed".into(), first: "Zachary".into(), last: "Edwards".into(), breed: 2, sex: 3, profession: 4, title_level: 3, level: 205, side: Some(1), org: Some("The Guild".into()), rank: Some("Member".into()), duel_wins: 2, ..Default::default() };
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
        // (no shadow level under 201, no inspect link for the client's own character; the title line always, as
        // FUN_1003482c has no condition on it)
        assert!(page.contains("chatcmd:///inspect 77\">Inspect Equipment"));
        let own = super::page(&Character { name: "Me".into(), level: 60, own: true, ..Default::default() });
        assert!(!own.contains("Inspect") && own.contains(">60<") && own.contains("(TitleLevel 0)") && own.contains("\"Me\""));
        assert_eq!(level(200).matches("font color=CCInfoHeader").count(), 1);
        assert_eq!(level(201).matches("font color=CCInfoHeader").count(), 2);
    }
}
