//! Talking to an NPC: the `Knubot` messages, as the 2026-11-08 capture shows them at the daily-mission agency and in the
//! Arbiter's office. The client opens the window (`KnubotOpenChatWindow`, which the server echoes), the server appends lines
//! (`KnubotAppendText`) and lists the answers (`KnubotAnswerList`), the client picks one (`KnubotAnswer`, counted from 0), and
//! the server closes the window (`KnubotCloseChatWindow`, its word 5) when the talk is over. Every message carries a short 2 and
//! the NPC's identity first.
//!
//! The server's one NPC is the agency receptionist in Old Athens: applying for a job gives the daily mission, and talking to her
//! again with it completes it (the Arbiter capture's completion: `CharacterAction` 0x3b, then the log's Delete). The words are
//! the server's own.

use mao_net::n3::{Message, CHARACTER};
use mao_net::schema::{Schema, Value};

use crate::missions::{Taken, MISSION_TYPE, REWARD};
use crate::templates::{identity, int, list, record};

pub const OPEN: u32 = 0x3b132d64;
pub const TEXT: u32 = 0x5d70532a;
pub const ANSWERS: u32 = 0x55704d31;
pub const ANSWER: u32 = 0x2103247d;
pub const CLOSE: u32 = 0x270a4c62;

/// An NPC the character can talk to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Npc {
    pub id: u32,
    pub name: &'static str,
    pub playfield: u32,
    pub position: [f32; 3],
}

/// The agency receptionist, by the spawn in Old Athens.
pub const RECEPTIONIST: Npc = Npc { id: 0x7f00_0101, name: "Agency Receptionist", playfield: crate::monsters::OLD_ATHENS, position: [crate::monsters::SPAWN[0] + 6.0, crate::monsters::SPAWN[1], crate::monsters::SPAWN[2] - 9.0] };

pub const NPCS: &[Npc] = &[RECEPTIONIST];

/// The NPC with an id, if the server has one.
pub fn npc(id: u32) -> Option<Npc> {
    NPCS.iter().copied().find(|n| n.id == id)
}

/// The NPCs of a playfield.
pub fn in_playfield(playfield: u32) -> Vec<Npc> {
    NPCS.iter().copied().filter(|n| n.playfield == playfield).collect()
}

/// The daily mission's instance (one per character: the mission is the same each time).
pub const DAILY: u32 = 0x7d01;

/// The daily mission the receptionist hands out: completed by coming back to her.
pub fn daily(npc: &Npc) -> Taken {
    Taken {
        mission: (MISSION_TYPE, DAILY),
        title: "Daily Assignment: Rounds".to_string(),
        description: "Daily Assignment: Rounds<BR><BR>Walk the streets of Old Athen and come back to the agency to report. The office pays on your return.".to_string(),
        credits: 1500,
        xp: 6000,
        giver: (CHARACTER, npc.id),
        reward: REWARD,
        playfield: npc.playfield,
        location: npc.position,
    }
}

/// How far the talk has come.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The greeting: "I'm applying for a job." / "Goodbye".
    Greeting,
    /// The preferences: "Straight forward, please." / "Goodbye".
    Preferences,
    /// Only "Goodbye" is left.
    Farewell,
}

/// A talk in progress: with which NPC, and where it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Talk {
    pub npc: u32,
    pub step: Step,
}

/// What the server does in answer to the client.
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    /// Lines to append and the answers to list; the talk goes on at `step`.
    Say { lines: Vec<String>, answers: Vec<&'static str>, step: Step },
    /// A line, then the daily mission is given and the window closes.
    Grant { line: String },
    /// A line, then the mission is completed and only "Goodbye" is left.
    Complete { line: String },
    /// The window closes.
    Close,
}

/// The receptionist's greeting when the window opens: a character carrying her mission has it completed; the others may apply.
pub fn greet(npc: &Npc, name: &str, has_daily: bool) -> Reply {
    if has_daily {
        Reply::Complete { line: format!("Welcome back, {name}. Your assignment is complete; the office thanks you for your service.") }
    } else {
        Reply::Say { lines: vec![format!("{name}, status - unassigned. What can I do for you?")], answers: vec!["I'm applying for a job.", "Goodbye"], step: Step::Greeting }
    }
    .about(npc)
}

/// The answer to an answer.
pub fn answer(npc: &Npc, step: Step, choice: u32) -> Reply {
    match (step, choice) {
        (Step::Greeting, 0) => Reply::Say { lines: vec!["Please state your preferences and I'll take it into consideration.".to_string()], answers: vec!["Straight forward, please.", "Goodbye"], step: Step::Preferences },
        (Step::Preferences, 0) => Reply::Grant { line: "I'm uploading a mission into your system. Report back here when it is done.".to_string() },
        _ => Reply::Close,
    }
    .about(npc)
}

impl Reply {
    // (the replies are the same for every NPC today; the hook is here for the next one)
    fn about(self, _npc: &Npc) -> Reply {
        self
    }
}

/// A `Knubot` message about `npc` to the character: the header byte, the short 2, the NPC, then `fields`.
fn message(kind: u32, character: u32, npc: u32, fields: Vec<(&str, Value)>) -> Result<Message, String> {
    let mut all = vec![("Unknown1", int(2)), ("Target", identity((CHARACTER, npc)))];
    all.extend(fields);
    let data = Schema::global().encode_n3(kind, 0, &record(all))?;
    Ok(Message { kind, identity: (CHARACTER, character), data })
}

/// The server's echo of the window's opening.
pub fn open(character: u32, npc: u32) -> Result<Message, String> {
    message(OPEN, character, npc, vec![("Unknown2", int(0)), ("Unknown3", int(0))])
}

/// A line of the NPC's.
pub fn text(character: u32, npc: u32, line: &str) -> Result<Message, String> {
    message(TEXT, character, npc, vec![("Unknown2", int(0)), ("Text", Value::Str(line.to_string()))])
}

/// The answers the character may give.
pub fn answers(character: u32, npc: u32, options: &[&str]) -> Result<Message, String> {
    let options = options.iter().map(|o| record(vec![("Text", Value::Str(o.to_string()))])).collect();
    message(ANSWERS, character, npc, vec![("DialogOptions", list(options))])
}

/// The window's closing (the captured word 5).
pub fn close(character: u32, npc: u32) -> Result<Message, String> {
    message(CLOSE, character, npc, vec![("Unknown2", int(5)), ("Unknown3", int(0))])
}

/// The NPC and the field a client `Knubot` message names (`Answer` for an answer, else 0).
pub fn request(kind: u32, data: &[u8]) -> Option<(u32, u32)> {
    let parsed = Schema::global().parse_n3(kind, data).ok()?;
    let target = parsed.value.get("Target")?;
    let npc = target.get("Instance").and_then(Value::as_int)? as u32;
    let answer = parsed.value.get("Answer").and_then(Value::as_int).unwrap_or(0) as u32;
    Some((npc, answer))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(m: &Message) -> String {
        m.data.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn the_messages_have_the_captured_shapes() {
        // (the capture: the open's echo, a line, two answers, the close, about NPC 0x7b22e197)
        assert_eq!(hex(&open(1, 0x7b22e197).unwrap()), "0000020000c3507b22e1970000000000000000");
        assert_eq!(hex(&text(1, 0x7b22e197, "Hi").unwrap()), "0000020000c3507b22e197000000000000000248 69".replace(' ', ""));
        assert_eq!(hex(&answers(1, 0x7b22e197, &["Goodbye"]).unwrap()), "0000020000c3507b22e1970000000100000007476f6f64627965");
        assert_eq!(hex(&close(1, 0x7b22e197).unwrap()), "0000020000c3507b22e1970000000500000000");
    }

    #[test]
    fn the_clients_requests_are_read() {
        let open = mao_net::n3::npc_chat_open(1, (CHARACTER, RECEPTIONIST.id));
        let m = Message::parse(&open).unwrap();
        assert_eq!(request(m.kind, &m.data), Some((RECEPTIONIST.id, 0)));
        let answer = mao_net::n3::npc_chat_answer(1, (CHARACTER, RECEPTIONIST.id), 1);
        let m = Message::parse(&answer).unwrap();
        assert_eq!(request(m.kind, &m.data), Some((RECEPTIONIST.id, 1)));
    }

    #[test]
    fn the_talk_leads_to_the_daily_and_back() {
        assert!(matches!(greet(&RECEPTIONIST, "Nerfit", false), Reply::Say { step: Step::Greeting, .. }));
        assert!(matches!(answer(&RECEPTIONIST, Step::Greeting, 0), Reply::Say { step: Step::Preferences, .. }));
        assert!(matches!(answer(&RECEPTIONIST, Step::Preferences, 0), Reply::Grant { .. }));
        assert_eq!(answer(&RECEPTIONIST, Step::Greeting, 1), Reply::Close);
        assert_eq!(answer(&RECEPTIONIST, Step::Farewell, 0), Reply::Close);
        assert!(matches!(greet(&RECEPTIONIST, "Nerfit", true), Reply::Complete { .. }));
        assert_eq!(daily(&RECEPTIONIST).giver, (CHARACTER, RECEPTIONIST.id));
    }
}
