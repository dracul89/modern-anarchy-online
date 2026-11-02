//! Missions: the offers a mission terminal makes (`QuestAlternative`, built from the schema: the captures have no reply to copy), the
//! mission log (`QuestFullUpdate`, built from the captured log's quest entry) and a mission's removal (`Quest`, action Delete).
//! The offers' texts are the server's own. The fields the schema does not name are zero (see `docs/AO_SERVER.md`).

use mao_net::n3::Message;
use mao_net::schema::{Schema, Value};

use crate::progress::Hero;
use crate::templates::{self, identity, int, list, record};

/// The identity type of a mission (`MissionInfo`'s, as the captures name it).
pub const MISSION_TYPE: u32 = 0xdac3;

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

/// A mission the character has taken: its identity, its texts and what it pays when it is done.
#[derive(Clone, Debug, PartialEq)]
pub struct Taken {
    pub mission: (u32, u32),
    pub title: String,
    pub description: String,
    pub credits: u32,
    pub xp: u32,
}

impl Taken {
    /// The mission an offer is, once taken.
    pub fn of(offer: &Offer) -> Taken {
        Taken { mission: (MISSION_TYPE, offer.id), title: offer.title.to_string(), description: offer.description.to_string(), credits: offer.credits, xp: offer.xp }
    }
}

/// A vector value.
fn vector(v: [f32; 3]) -> Value {
    record(vec![("X", Value::Float(v[0] as f64)), ("Y", Value::Float(v[1] as f64)), ("Z", Value::Float(v[2] as f64))])
}

/// The bytes of a fixed-size unknown chunk.
fn chunk(n: usize) -> Value {
    Value::Bytes(vec![0; n])
}

/// One offer as the reply lists it (`MissionInfo`).
fn info(o: &Offer) -> Value {
    record(vec![
        ("MissionIdentity", identity((MISSION_TYPE, o.id))),
        ("UnkChunk1", chunk(16)),
        ("Title", Value::Str(o.title.to_string())),
        ("Description", Value::Str(o.description.to_string())),
        ("TerminalIdentity", identity((0xdac1, 0))),
        ("RewardDescriptorVersion", int(0)),
        ("Credits", int(o.credits as i64)),
        ("Unk1", int(0)),
        ("XpReward", int(o.xp as i64)),
        ("UnkChunk2", chunk(8)),
        ("MissionItemData", list(Vec::new())),
        ("UnkChunk3", chunk(44)),
        ("MissionIcon", int(0)),
        ("UnkChunk4", chunk(120)),
        ("Playfield", identity((0, o.playfield))),
        ("UnkChunk5", chunk(8)),
        ("Location", vector(o.location)),
        ("UnkChunk6", chunk(61)),
    ])
}

/// The `QuestAlternative` reply to a terminal's request: the request's sliders, scope and terminal, and the offers.
pub fn offers(character: u32, request: &Value) -> Result<Message, String> {
    let field = |name: &str| request.get(name).cloned().unwrap_or(Value::Null);
    let reply = record(vec![
        ("Unknown1", int(1)),
        ("MissionSliders", field("MissionSliders")),
        ("Unknown2", field("Unknown2")),
        ("Scope", field("Scope")),
        ("Terminal", field("Terminal")),
        ("MissionDetails", list(OFFERS.iter().map(info).collect())),
    ]);
    let data = mao_net::schema::Schema::global().encode_n3(0x5c436609, 1, &reply)?;
    Ok(Message { kind: 0x5c436609, identity: (mao_net::n3::CHARACTER, character), data })
}

/// The mission log: every mission the character has taken (`QuestFullUpdate`). Built from the schema alone: the fields the server
/// does not know are left zero, and the texts are the server's own (no captured log text is kept in the repository).
pub fn log(character: u32, hero: &Hero) -> Result<Message, String> {
    let quests = hero.missions.iter().map(|m| record(vec![("QuestId", identity(m.mission)), ("ShortInfo", Value::Str(m.title.clone())), ("LongInfo", Value::Str(m.description.clone()))])).collect();
    let value = record(vec![("Quests", list(quests))]);
    let data = Schema::global().encode_n3(0x465a4061, 1, &value)?;
    Ok(Message { kind: 0x465a4061, identity: (mao_net::n3::CHARACTER, character), data })
}

/// A mission's removal from the log (`Quest`, action 1: Delete).
pub fn remove(character: u32, mission: (u32, u32)) -> Result<Message, String> {
    let mut t = templates::of(0x212c487a).ok_or("no Quest template")?;
    t.set("Action", int(1));
    t.set("Mission", identity(mission));
    t.message(0x212c487a, (mao_net::n3::CHARACTER, character))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mao_net::schema::Schema;

    #[test]
    fn the_offers_are_listed_with_their_sliders_and_terminal() {
        let request = record(vec![
            ("MissionSliders", record(vec![("Difficulty", int(2)), ("GoodBad", int(20)), ("OrderChaos", int(161)), ("OpenHidden", int(30)), ("PhysicalMystical", int(184)), ("HeadonStealth", int(28)), ("CreditsXp", int(0))])),
            ("Unknown2", int(0)),
            ("Scope", int(5)),
            ("Terminal", identity((0xdac1, 0xc000021c))),
        ]);
        let m = offers(7, &request).unwrap();
        let parsed = Schema::global().parse_n3(0x5c436609, &m.data).unwrap();
        let Some(Value::List(details)) = parsed.value.get("MissionDetails") else { panic!("no details") };
        assert_eq!(details.len(), OFFERS.len());
        assert_eq!(details[0].get("Title").and_then(|v| match v {
            Value::Str(s) => Some(s.as_str()),
            _ => None,
        }), Some(OFFERS[0].title));
        assert_eq!(parsed.value.get("Scope").and_then(|v| v.as_int()), Some(5));
    }

    #[test]
    fn the_log_lists_the_taken_missions() {
        let mut hero = Hero::new(7);
        hero.missions.push(Taken::of(&OFFERS[1]));
        let m = log(7, &hero).unwrap();
        let parsed = Schema::global().parse_n3(0x465a4061, &m.data).unwrap();
        let Some(Value::List(quests)) = parsed.value.get("Quests") else { panic!("no quests") };
        assert_eq!(quests.len(), 1);
        assert_eq!(quests[0].get("ShortInfo").and_then(|v| match v {
            Value::Str(s) => Some(s.as_str()),
            _ => None,
        }), Some(OFFERS[1].title));
    }

    #[test]
    fn a_removal_names_the_mission() {
        let m = remove(7, (MISSION_TYPE, 2)).unwrap();
        let parsed = Schema::global().parse_n3(0x212c487a, &m.data).unwrap();
        assert_eq!(parsed.value.get("Action").and_then(|v| v.as_int()), Some(1));
        assert_eq!(parsed.value.get("Mission").and_then(|v| v.get("Instance")).and_then(|v| v.as_int()), Some(2));
    }
}
