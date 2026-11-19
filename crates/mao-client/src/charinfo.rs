//! A character's page in the info view, as GUI.dll builds it (FUN_1003482c), from what the server's info packet and the
//! character's own data say.
//!
//! The headline (`CCInfoHeadline`) is the first name, the name in quotes and the last name. Then a labelled line each
//! (FUN_10031b5b, the labels text.mdb 506 by name): `Breed`, `Gender`, `ProfessionTitle` (the title's name,
//! `GetTitleStr` Gamecode 0x10016185: text.mdb 1060 by profession * 100 + title level, then its level as
//! " (TitleLevel %d)"; always, whatever the level), `Profession` (`NotChosenYet` for none), `Level` (with `ShadowLevel`,
//! the levels past 200, beside it for levels 201 to 220), `NumInvadersKilled`, `KilledByInvaders`, `DefenderRank` (the
//! rank's name, `N3Msg_GetAlienLevelString` Gamecode 0x10016d9b: text.mdb 1169 by alien level * 100), `Alignment` (the side's name,
//! text.mdb 2005; side 3 `NotChosenYet`), for the own character of side 1 or 2 its `Tokens` (stat 62, an omni's 75),
//! `SideXpBonus` (the info packet's short over 100, `N3Msg_GetInfoPacketSideXpBonus` 0x10027394, "%.02f%%"), the
//! organization and the rank in it (`Clan` / `ClanTitle`, an omni's `Detachment` / `DetachmentTitle`), a break; for the
//! own character its veteran points (`VeteranPoints` of text.mdb 10004, stat 68 grouped, when stat 69 is above 5), its
//! victory points (stat 669, when stat 389 has 0x20), its ICC commendations (695) and Freelancers Inc. tokens (696), and
//! its `Specializations` (`N3Msg_GetSpecializationString` 0x1001b17d: a feedback text for each of stat 182's four
//! bits, a line each); then the duel, solo and team ranks (`N3Msg_GetPVPRank`, [`pvp_rank`] of stats 684, 682 and 683:
//! text.mdb 2010 by 2000 + rank, "%s (%d)"), each before its kills (and the duel's deaths), and for another character a
//! link that inspects its equipment (`chatcmd:///inspect <name>`).
//!
//! Not here: the organization's grid destinations (`MastersOf`, FUN_10031eda: `N3Msg_GetGridDestinationList`), the city
//! (`OwnersOfCity`: Gamecode's `N3Msg_GetCityNameForClanMember` 0x10016dfc names the info packet's `+0xf0` with a
//! table, FUN_10036982, which holds the stats' names: what it gives for a city was not settled) and the account's items
//! (`N3Msg_GetInfoPacketACGItemData`). Another character's PvP ranks are read from its stats in the original; the
//! client has none of them, so they are rank 0 here (a guess at what the original shows when the server sends none).

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
    /// The info packet's side experience bonus (hundredths of a percent).
    pub side_xp: u32,
    /// The own character's stats (empty for another): tokens, veteran points, specializations, PvP scores.
    pub stats: std::collections::BTreeMap<u32, i64>,
}

/// A PvP rank's lowest score (Gamecode FUN_1003ea89): 2500 to the 0.1111111 (as a float, `_CIpow`) raised to `rank - 1`
/// by halving (FUN_1003dfc0, a float at each step), times 20, cut to a whole number, times 100.
fn pvp_threshold(rank: i64) -> i64 {
    let mut base = 2500f64.powf(0.1111111119389534) as f32;
    let (mut e, mut out) = ((rank - 1) as u32, 1.0f32);
    loop {
        if e & 1 != 0 {
            out *= base;
        }
        e >>= 1;
        if e == 0 {
            break;
        }
        base *= base;
    }
    (out as f64 * 20.0) as i64 * 100
}

/// A PvP score's rank (`N3Msg_GetPVPRank` Gamecode 0x10027b11 -> FUN_1003eae2): the first rank of 1 to 10 whose lowest
/// score it is under, less one; 10 past them all.
pub fn pvp_rank(score: i64) -> i64 {
    (1..11).find(|&n| score < pvp_threshold(n)).map_or(10, |n| n - 1)
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
        "Tokens" => "Tokens: ",
        "SideXpBonus" => "Side experience bonus: ",
        "Clan" | "Detachment" => "Organization: ",
        "ClanTitle" | "DetachmentTitle" => "Rank: ",
        "Specializations" => "Specializations: ",
        "DuelRank" => "Duel rank: ",
        "SoloRank" => "Solo rank: ",
        "TeamRank" => "Team rank: ",
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

/// The own character's specializations (`N3Msg_GetSpecializationString`, Gamecode 0x1001b17d): for each of stat 182's
/// bits 1, 2, 4 and 8 the feedback text of that specialization done, a line each.
fn specializations(bits: i64) -> String {
    const DONE: [&str; 4] = ["Feedback_FirstSpecializationCompleted", "Feedback_SecondSpecializationCompleted", "Feedback_ThirdSpecializationCompleted", "Feedback_FourthSpecializationCompleted"];
    DONE.iter().enumerate().filter(|(k, _)| bits & (1 << k) != 0).map(|(_, name)| crate::feedback::plain(name)).collect::<Vec<_>>().join("\n")
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
        // (FUN_1003482c: side 3 is `NotChosenYet`, the others text.mdb 2005)
        let name = if side == 3 { label("NotChosenYet") } else { crate::feedback::texts().and_then(|t| t.get(2005, side as i32)).map_or_else(|| ["neutral", "clan", "omni"].get(side as usize).copied().unwrap_or("-").to_string(), str::to_string) };
        out += &section(&label("Alignment"), &name, 0);
    }
    let stat = |n: u32| c.stats.get(&n).copied().unwrap_or(0);
    let omni = c.side == Some(2);
    // (the own character's tokens, a clan's stat 62, an omni's 75: `((side != 2) - 1 & 0xd) + 0x3e`)
    if c.own && matches!(c.side, Some(1 | 2)) {
        out += &section(&label("Tokens"), &stat(if omni { 75 } else { 62 }).to_string(), 0);
    }
    out += &section(&label("SideXpBonus"), &format!("{:.2}%", (c.side_xp as f32 / 100.0) as f64), 0);
    if let Some(org) = c.org.as_deref().filter(|o| !o.is_empty()) {
        out += &section(&label(if omni { "Detachment" } else { "Clan" }), org, 0);
    }
    if let Some(rank) = c.rank.as_deref().filter(|r| !r.is_empty()) {
        out += &section(&label(if omni { "DetachmentTitle" } else { "ClanTitle" }), &format!(" {rank}"), 0);
    }
    out += "<br>";
    if c.own {
        if stat(69) > 5 {
            out += &section(&crate::gui::xml::ldb_named(10004, "VeteranPoints", "Veteran points: "), &crate::vitals::grouped(stat(68)), 0);
        }
        if stat(389) & 0x20 != 0 {
            out += &section("Victory Points: ", &stat(669).to_string(), 0);
        }
        out += &section("ICC Commendations: ", &stat(695).to_string(), 0);
        out += &section("Freelancers Inc. Tokens: ", &stat(696).to_string(), 0);
        out += &section(&label("Specializations"), &specializations(stat(182)), 0);
    }
    // (the ranks: text.mdb 2010 by 2000 + rank, "%s (%d)")
    let rank = |score: u32| {
        let r = pvp_rank(stat(score));
        format!("{} ({r})", crate::feedback::texts().and_then(|t| t.get(2010, 2000 + r as i32)).unwrap_or_default())
    };
    out += &section(&label("DuelRank"), &rank(684), 0);
    for (name, value) in [("DuelKills", c.duel_wins), ("DuelDeaths", c.duel_losses)] {
        out += &section(&label(name), &format!(" {value}"), 0);
    }
    out += &section(&label("SoloRank"), &rank(682), 0);
    out += &section(&label("SoloKills"), &format!(" {}", c.solo_kills), 0);
    out += &section(&label("TeamRank"), &rank(683), 0);
    out += &section(&label("TeamKills"), &format!(" {}", c.team_kills), 0);
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

    /// WIN-116: the PvP ranks' lowest scores (FUN_1003ea89): 2000 for rank 1, about 2500 * 20 * 100 for rank 10.
    #[test]
    fn pvp_ranks_follow_the_originals_thresholds() {
        assert_eq!(pvp_threshold(1), 2000);
        assert!((pvp_threshold(10) - 5_000_000).abs() <= 500, "{}", pvp_threshold(10));
        assert!((2..=10).all(|n| pvp_threshold(n) > pvp_threshold(n - 1)));
        assert_eq!([pvp_rank(0), pvp_rank(1999), pvp_rank(2000), pvp_rank(pvp_threshold(4) - 1), pvp_rank(pvp_threshold(4)), pvp_rank(i64::from(i32::MAX))], [0, 0, 1, 3, 4, 10]);
    }

    /// WIN-116: the side experience bonus for anyone; the tokens, veteran points and the rest for the own character only;
    /// an omni's organization lines are the detachment's.
    #[test]
    fn the_own_characters_lines() {
        let stats = [(75, 42), (62, 7), (69, 6), (68, 12345), (389, 0x20), (669, 3), (695, 4), (696, 5), (684, 2000)].into_iter().collect();
        let own = page(&Character { name: "Me".into(), side: Some(2), own: true, side_xp: 1250, org: Some("Unit".into()), rank: Some("Agent".into()), stats, ..Default::default() });
        for word in [">42<", "12.50%", ">12,345<", "Victory Points: ", ">3<", "ICC Commendations: ", ">4<", ">5<", "(1)", "(0)"] {
            assert!(own.contains(word), "{word} is missing in {own}");
        }
        let other = page(&Character { name: "Zed".into(), side: Some(1), side_xp: 7, ..Default::default() });
        assert!(other.contains("0.07%") && !other.contains("Victory") && !other.contains("ICC") && other.contains("(0)"));
        assert!(!page(&Character { side: Some(2), own: true, ..Default::default() }).contains("Victory Points"));
    }
}
