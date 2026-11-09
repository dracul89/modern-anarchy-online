//! Missions: the offers a mission terminal makes (`QuestAlternative`), the mission log (`QuestFullUpdate`), a mission's removal
//! (`Quest`, action Delete) and its completion (`CharacterAction` 0x3b). The layouts are the 2026-11-08 capture's (the ICC
//! Mission Central terminal's five offers, the accepted one in the log, the Arbiter mission's completion): the fields the schema
//! does not name carry the captured constants. The texts are the server's own; no captured mission text is kept.

use mao_net::n3::{Message, CHARACTER};
use mao_net::schema::{Schema, Value};

use crate::progress::Hero;
use crate::templates::{self, identity, int, list, record};

/// The identity type of a mission (`MissionInfo`'s, as the captures name it).
pub const MISSION_TYPE: u32 = 0xdac3;
/// The identity type of a mission terminal.
pub const TERMINAL_TYPE: u32 = 0xdac1;
/// The identity type of a playfield in these messages.
pub const PLAYFIELD_TYPE: u32 = 0x9c50;
/// The deadline of a terminal mission, in minutes: 48 hours (the captured `0xb40`, shown as 47:59 in the log).
pub const DEADLINE_MINUTES: u32 = 2880;
/// The icon the offers carry, and the one the log shows for a taken mission (captured).
pub const OFFER_ICON: u32 = 11330;
pub const LOG_ICON: u32 = 11329;
/// The marker word of a kill mission's reward descriptor in the captured offers ("MSRE").
const MSRE: u32 = 0x4d53_5245;
/// The item a mission rewards (a stock item of the captured vending machine, as the monsters' loot).
pub const REWARD: (u32, u32) = (crate::monsters::LOOT, crate::monsters::LOOT);

/// A mission a terminal offers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Offer {
    /// The mission's instance (its identity is `(MISSION_TYPE, id)`).
    pub id: u32,
    pub title: &'static str,
    pub description: &'static str,
    pub credits: u32,
    pub xp: u32,
    /// Where it leads: the playfield and the place.
    pub playfield: u32,
    pub location: [f32; 3],
}

/// The missions the terminals offer.
pub const OFFERS: &[Offer] = &[
    Offer { id: 1, title: "A Small Errand", description: "Carry a parcel across town and report back.", credits: 500, xp: 2000, playfield: 540, location: [400.0, 9.0, 300.0] },
    Offer { id: 2, title: "Pest Control", description: "Clear the vermin from the lower streets.", credits: 800, xp: 3000, playfield: 540, location: [360.0, 9.0, 480.0] },
    Offer { id: 3, title: "Scout the Ruins", description: "Walk to the old ruins and bring back what you find.", credits: 1200, xp: 4500, playfield: 540, location: [520.0, 9.0, 250.0] },
];

/// The offer of a mission identity, if it is one of ours.
pub fn offer_of(mission: (u32, u32)) -> Option<Offer> {
    if mission.0 != MISSION_TYPE {
        return None;
    }
    OFFERS.iter().copied().find(|o| o.id == mission.1)
}

/// A mission the character has taken: its identity, its texts, what it pays when it is done, who gave it (the terminal or the
/// NPC: the log names it, and an NPC's mission is completed by talking to that NPC again), its reward item and where it leads.
#[derive(Clone, Debug, PartialEq)]
pub struct Taken {
    pub mission: (u32, u32),
    pub title: String,
    pub description: String,
    pub credits: u32,
    pub xp: u32,
    pub giver: (u32, u32),
    pub reward: (u32, u32),
    pub playfield: u32,
    pub location: [f32; 3],
}

impl Taken {
    /// The mission an offer is, once taken from `terminal`.
    pub fn of(offer: &Offer, terminal: (u32, u32)) -> Taken {
        Taken {
            mission: (MISSION_TYPE, offer.id),
            title: offer.title.to_string(),
            description: offer.description.to_string(),
            credits: offer.credits,
            xp: offer.xp,
            giver: terminal,
            reward: REWARD,
            playfield: offer.playfield,
            location: offer.location,
        }
    }
}

/// A vector value.
fn vector(v: [f32; 3]) -> Value {
    record(vec![("X", Value::Float(v[0] as f64)), ("Y", Value::Float(v[1] as f64)), ("Z", Value::Float(v[2] as f64))])
}

/// Big-endian words as bytes.
fn words(ws: &[u32]) -> Vec<u8> {
    ws.iter().flat_map(|w| w.to_be_bytes()).collect()
}

/// A reward item as the offers and the log list it.
fn reward(item: (u32, u32), quality: u32) -> Value {
    record(vec![("LowId", int(item.0 as i64)), ("HighId", int(item.1 as i64)), ("Ql", int(quality as i64)), ("Unk", int(0))])
}

/// One offer as the reply lists it (`MissionInfo`), in the captured layout. `level` is the character's (the reward's quality and a
/// word of the descriptor), `deadline` the unix time the mission runs out, `index` the offer's place in the reply.
fn info(o: &Offer, terminal: (u32, u32), level: u32, deadline: u32, index: usize) -> Value {
    // (the descriptor: three zero words, "MSRE", the level, six zero words)
    let mut chunk3 = words(&[0, 0, 0, MSRE, level]);
    chunk3.extend_from_slice(&[0; 24]);
    // (the deadline in minutes, a one-element list whose element is a version 1 record with the code "Q011", zeros, the deadline
    // as a unix time, three zero words)
    let mut chunk4 = words(&[0, DEADLINE_MINUTES, 0x7e2, 1, 0, 0, 0, 0, 0x111d3, 0x5130_3131]);
    chunk4.resize(104, 0);
    chunk4.extend_from_slice(&deadline.to_be_bytes());
    chunk4.extend_from_slice(&[0; 12]);
    let mut chunk6 = words(&[0x3f1, 0, 0, 0, 6, 0x3f1, 0, 0x476 + index as u32, 0, 0, 0, 0, 0, 1, 0x3f1]);
    chunk6.push(index as u8);
    record(vec![
        ("MissionIdentity", identity((MISSION_TYPE, o.id))),
        ("UnkChunk1", Value::Bytes(words(&[0xf, 0, 0, 0]))),
        ("Title", Value::Str(o.title.to_string())),
        ("Description", Value::Str(o.description.to_string())),
        ("TerminalIdentity", identity(terminal)),
        ("RewardDescriptorVersion", int(6)),
        ("Credits", int(o.credits as i64)),
        ("Unk1", int(0)),
        ("XpReward", int(o.xp as i64)),
        ("UnkChunk2", Value::Bytes(words(&[0x3f1, 0x3f1]))),
        ("MissionItemData", list(vec![reward(REWARD, level)])),
        ("UnkChunk3", Value::Bytes(chunk3)),
        ("MissionIcon", int(OFFER_ICON as i64)),
        ("UnkChunk4", Value::Bytes(chunk4)),
        ("Playfield", identity((PLAYFIELD_TYPE, o.playfield))),
        ("UnkChunk5", Value::Bytes(words(&[41959, 42152]))),
        ("Location", vector(o.location)),
        ("UnkChunk6", Value::Bytes(chunk6)),
    ])
}

/// The `QuestAlternative` reply to a terminal's request: the request's first byte, sliders, scope and terminal (as the original
/// echoes them), a server word (the captured one is a random-looking number: the time serves), and the offers.
pub fn offers(character: u32, request: &Value, level: u32, now: u32) -> Result<Message, String> {
    let field = |name: &str| request.get(name).cloned().unwrap_or(Value::Null);
    let terminal = request.get("Terminal").map(|t| (t.get("Type").and_then(Value::as_int).unwrap_or(0) as u32, t.get("Instance").and_then(Value::as_int).unwrap_or(0) as u32)).unwrap_or((TERMINAL_TYPE, 0));
    let deadline = now.wrapping_add(DEADLINE_MINUTES * 60);
    let reply = record(vec![
        ("Unknown1", request.get("Unknown1").cloned().unwrap_or(int(4))),
        ("MissionSliders", field("MissionSliders")),
        ("Unknown2", int(now as i64)),
        ("Scope", field("Scope")),
        ("Terminal", identity(terminal)),
        ("MissionDetails", list(OFFERS.iter().enumerate().map(|(i, o)| info(o, terminal, level, deadline, i)).collect())),
    ]);
    let data = Schema::global().encode_n3(0x5c436609, 0, &reply)?;
    Ok(Message { kind: 0x5c436609, identity: (CHARACTER, character), data })
}

/// One mission of the log (`Quest`), in the captured layout: the giver, the reward, the deadline, one action naming where it
/// leads (so the client's "Upload to map" has a place), the character as the only player.
fn quest(character: u32, level: u32, m: &Taken) -> Value {
    let me = identity((CHARACTER, character));
    let none = identity((0, 0));
    let zero = Value::Float(0.0);
    let action = record(vec![
        ("Version", int(8)),
        ("Action", none.clone()),
        ("UnknownId1", identity(m.giver)),
        ("UnknownId2", none.clone()),
        ("UnknownId3", none.clone()),
        ("UnknownId4", none.clone()),
        ("Unknown1", zero.clone()),
        ("Unknown2", zero.clone()),
        ("Unknown3", zero.clone()),
        ("Unknown4", zero.clone()),
        ("UnknownId5", none.clone()),
        ("Unknown5", zero.clone()),
        ("Unknown6", zero.clone()),
        ("Unknown7", zero.clone()),
        ("Unknown8", zero),
        ("UnknownId6", none.clone()),
        ("UnknownHash1", Value::Str(String::new())),
        ("Unknown9", int(0)),
        ("UnknownId7", none.clone()),
        ("PlayfieldId", identity((PLAYFIELD_TYPE, m.playfield))),
        ("Unknown10", int(0)),
        ("Unknown11", int(0)),
        ("Position", vector(m.location)),
    ]);
    record(vec![
        ("QuestId", identity(m.mission)),
        ("Unknown1", int(15)),
        ("Unknown2", int(0)),
        ("Unknown3", int(0)),
        ("Unknown4", int(2)),
        ("ShortInfo", Value::Str(m.title.clone())),
        ("LongInfo", Value::Str(m.description.clone())),
        ("UnknownId1", identity(m.giver)),
        ("Unknown5", int(6)),
        ("Unknown6", int(m.credits as i64)),
        ("Unknown7", int(0)),
        ("Unknown8", int(m.xp as i64)),
        ("Unknown9", int(1009)),
        ("Unknown10", int(1009)),
        ("MissionItemData", list(vec![reward(m.reward, level)])),
        ("Unknown11", int(0)),
        ("Unknown14", int(0)),
        ("Unknown12", int(0)),
        ("Unknown15", int(MSRE as i64)),
        ("Unknown13", int(level as i64)),
        ("Unknown16", int(0)),
        ("UnknownHash1", Value::Str(String::new())),
        ("Unknown17", int(0)),
        ("Unknown18", int(0)),
        ("UnknownId2", me.clone()),
        ("MissionIconId", int(LOG_ICON as i64)),
        ("Unknown20", int(DEADLINE_MINUTES as i64)),
        ("Unknown21", int(DEADLINE_MINUTES as i64)),
        ("QuestActions", list(vec![action])),
        ("PlayerIds", list(vec![me.clone()])),
        ("UnknownArray1", list(Vec::new())),
        ("UnknownArray2", list(Vec::new())),
        ("CharacterInfos", list(Vec::new())),
        ("Unknown22", int(6)),
        ("PlayerIds2", list(vec![me])),
        ("Unknown23", int(0)),
        ("Unknown24", int(1205)),
        ("UnknownId3", none),
        ("Unknown25", int(0)),
        ("Unknown26", int(0)),
        ("QuestIdentities", list(Vec::new())),
        ("Unknown27", int(1)),
        ("FactionInfos", list(Vec::new())),
    ])
}

/// The mission log: every mission the character has taken (`QuestFullUpdate`), as the original sends it after a mission is taken.
pub fn log(character: u32, hero: &Hero) -> Result<Message, String> {
    let quests = hero.missions.iter().map(|m| quest(character, hero.level, m)).collect();
    let value = record(vec![("Quests", list(quests)), ("Unknown28", int(1))]);
    let data = Schema::global().encode_n3(0x465a4061, 1, &value)?;
    Ok(Message { kind: 0x465a4061, identity: (CHARACTER, character), data })
}

/// A mission's removal from the log (`Quest`, action 1: Delete).
pub fn remove(character: u32, mission: (u32, u32)) -> Result<Message, String> {
    let mut t = templates::of(0x212c487a).ok_or("no Quest template")?;
    t.set("Action", int(1));
    t.set("Mission", identity(mission));
    t.message(0x212c487a, (CHARACTER, character))
}

/// A mission accomplished: `CharacterAction` 0x3b naming the mission as the target and as both parameters (the capture's, sent
/// right before the log's Delete; the client shows its "mission accomplished" line on it).
pub fn accomplished(character: u32, mission: (u32, u32)) -> Message {
    let mut data = vec![0u8];
    for word in [0x3b, 0, mission.0, mission.1, mission.0, mission.1] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(&[0, 0]);
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mao_net::schema::Schema;

    const TERMINAL: (u32, u32) = (TERMINAL_TYPE, 0xc000021c);

    fn request() -> Value {
        record(vec![
            ("Unknown1", int(4)),
            ("MissionSliders", record(vec![("Difficulty", int(2)), ("GoodBad", int(20)), ("OrderChaos", int(161)), ("OpenHidden", int(30)), ("PhysicalMystical", int(184)), ("HeadonStealth", int(28)), ("CreditsXp", int(0))])),
            ("Unknown2", int(0)),
            ("Scope", int(1)),
            ("Terminal", identity(TERMINAL)),
        ])
    }

    fn text(v: &Value, name: &str) -> Option<String> {
        match v.get(name) {
            Some(Value::Str(s)) => Some(s.clone()),
            _ => None,
        }
    }

    #[test]
    fn the_offers_are_listed_in_the_captured_layout() {
        let m = offers(7, &request(), 116, 1_794_183_500).unwrap();
        let parsed = Schema::global().parse_n3(0x5c436609, &m.data).unwrap();
        assert_eq!(parsed.left, 0, "the whole reply is described");
        assert_eq!(parsed.value.get("Unknown1").and_then(Value::as_int), Some(4));
        assert_eq!(parsed.value.get("Scope").and_then(Value::as_int), Some(1));
        let Some(Value::List(details)) = parsed.value.get("MissionDetails") else { panic!("no details") };
        assert_eq!(details.len(), OFFERS.len());
        let first = &details[0];
        assert_eq!(text(first, "Title").as_deref(), Some(OFFERS[0].title));
        assert_eq!(first.get("RewardDescriptorVersion").and_then(Value::as_int), Some(6));
        assert_eq!(first.get("MissionIcon").and_then(Value::as_int), Some(OFFER_ICON as i64));
        // (the schema reads an identity's instance as a signed word)
        assert_eq!(first.get("TerminalIdentity").and_then(|t| t.get("Instance")).and_then(Value::as_int), Some(0xc000021c_u32 as i32 as i64));
        let Some(Value::List(items)) = first.get("MissionItemData") else { panic!("no reward") };
        assert_eq!(items[0].get("Ql").and_then(Value::as_int), Some(116));
        // (the chunks have the captured sizes and marks)
        let chunk = |name: &str| match first.get(name) {
            Some(Value::Bytes(b)) => b.clone(),
            other => panic!("{name}: {other:?}"),
        };
        assert_eq!(chunk("UnkChunk3").len(), 44);
        assert_eq!(&chunk("UnkChunk3")[12..20], &[b'M', b'S', b'R', b'E', 0, 0, 0, 116]);
        assert_eq!(chunk("UnkChunk4").len(), 120);
        assert_eq!(&chunk("UnkChunk4")[4..8], &[0, 0, 0x0b, 0x40]);
        assert_eq!(&chunk("UnkChunk4")[104..108], &(1_794_183_500u32 + 2880 * 60).to_be_bytes());
        assert_eq!(chunk("UnkChunk6").len(), 61);
        assert_eq!(chunk("UnkChunk6")[60], 0);
    }

    #[test]
    fn the_log_lists_the_taken_missions_with_their_place() {
        let mut hero = Hero::new(7);
        hero.missions.push(Taken::of(&OFFERS[1], TERMINAL));
        let m = log(7, &hero).unwrap();
        let parsed = Schema::global().parse_n3(0x465a4061, &m.data).unwrap();
        assert_eq!(parsed.left, 0);
        let Some(Value::List(quests)) = parsed.value.get("Quests") else { panic!("no quests") };
        assert_eq!(quests.len(), 1);
        let q = &quests[0];
        assert_eq!(text(q, "ShortInfo").as_deref(), Some(OFFERS[1].title));
        assert_eq!(q.get("UnknownId1").and_then(|t| t.get("Type")).and_then(Value::as_int), Some(TERMINAL_TYPE as i64));
        assert_eq!(q.get("MissionIconId").and_then(Value::as_int), Some(LOG_ICON as i64));
        assert_eq!(q.get("Unknown20").and_then(Value::as_int), Some(2880));
        let Some(Value::List(actions)) = q.get("QuestActions") else { panic!("no actions") };
        assert_eq!(actions[0].get("PlayfieldId").and_then(|p| p.get("Instance")).and_then(Value::as_int), Some(540));
        let Some(Value::List(players)) = q.get("PlayerIds") else { panic!("no players") };
        assert_eq!(players[0].get("Instance").and_then(Value::as_int), Some(7));
    }

    #[test]
    fn a_removal_names_the_mission() {
        let m = remove(7, (MISSION_TYPE, 2)).unwrap();
        let parsed = Schema::global().parse_n3(0x212c487a, &m.data).unwrap();
        assert_eq!(parsed.value.get("Action").and_then(|v| v.as_int()), Some(1));
        assert_eq!(parsed.value.get("Mission").and_then(|v| v.get("Instance")).and_then(|v| v.as_int()), Some(2));
    }

    #[test]
    fn an_accomplished_mission_is_the_captured_action() {
        let m = accomplished(0x39c16ee4, (MISSION_TYPE, 0x56398aee));
        assert_eq!(m.data.iter().map(|b| format!("{b:02x}")).collect::<String>(), "000000003b000000000000dac356398aee0000dac356398aee0000");
        let parsed = Schema::global().parse_n3(0x5e477770, &m.data).unwrap();
        assert_eq!(parsed.value.get("Action").and_then(Value::as_int), Some(0x3b));
    }
}
