//! The chat's voice lines: a team, guild or other group message may carry a voice in its blob, which GUI.dll plays from
//! `sound/sfx/player/` (`ChatGUIModule_c::HandleGroupMessage` → FUN_1008cdb3).
//!
//! * The blob (FUN_10086075, read with FUN_10171b9b's formats: `B` a byte, `I` a big-endian word, `S` a `u16` length and the
//!   bytes) is a run of records, each a tag byte: 0 `IIISS` (skipped), 1 `BBSS` the voice (breed, sex, mood, line), 2 `B` a
//!   flag; an unknown tag or a short record ends it.
//! * FUN_1008cdb3 plays it when the client's stat 389 has bit 0, the group's preference allows it (`VoiceSndFxHearGuildOn`
//!   for an organisation's group, kind 3; `VoiceSndFxHearTeamOn` for the team, 0x82) and the line is not empty; at most one
//!   per sender in 3 s (FUN_1008c7c0, after FUN_1008c757 forgets the senders of 2 s ago).
//! * The names tried in order (FUN_1008c847, with FUN_1008c548's breeds and FUN_1008c59c's sexes): `<sender>_<line>_01`,
//!   `<breed>_<sex>_<mood>_<line>_01`, `<sex>_<mood>_<line>_01`, `<sex>_<line>_01`, `<line>_01`; the first whose `.wav` is
//!   there (with any `..` taken out) plays as `PlayPlayerFX(name.wav, 1)`.
//!
//! * Vicinity speech (chat packets 34 and 35, `ChatGUIModule_c::HandleVicinityMessage`) carries the same records after a
//!   leading byte (the speech's kind, which picks its group); its voice plays under `VoiceSndFxHearVicinityOn`.
//!
//! * `/voice <line>` ([`command`], GUI.dll FUN_100b88b1) speaks one: with stat 389's bit 0 (else "This function requires
//!   Notum Wars.", FUN_1016c32c's names of the expansions), the character's breed (stat 4), sex (stat 59) and the
//!   `VoiceSndFxType` mood (`CharPrefs.xml`: 0 simple, 1 distunguished, as the files spell it, 2 cool, 3 military;
//!   FUN_1008c472), when some name tried has both its `.wav` and its `.txt` (FUN_1008cbc2; else "You do not have that voice
//!   effect installed."); its text the first line of the first `.txt` (FUN_1008cfe3), any leading `[tags]` taken off
//!   (FUN_1008c5db), sent with record 1 (`BBBSS`, FUN_10089628) as the blob, said in the vicinity (the original's selected
//!   group, Vicinity by default; this client has no other to pick).
//!
//! Not here: the ignore list's say (a sender on it is heard only when record 2's flag has bit 0), and the tags' doings
//! (FUN_1008c5db: `T` flashes the speaker's name in the team window, `M` signals `GlobalSignals_c` +0x150).

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

/// A voice of a message's blob (FUN_10086075's record 1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Voice {
    pub breed: u8,
    pub sex: u8,
    pub mood: String,
    pub line: String,
}

/// The client's stat 389 (the `N3Msg_GetSkill(0x185)` FUN_1008cdb3 asks), kept by the game connection.
static STAT_389: AtomicI64 = AtomicI64::new(0);

pub fn set_stat_389(v: i64) {
    STAT_389.store(v, Ordering::Relaxed);
}

/// Who speaks a `/voice` line: the client's character's name, breed and sex (kept by the game connection).
static OWN: std::sync::Mutex<(String, u8, u8)> = std::sync::Mutex::new((String::new(), 0, 0));

pub fn set_own(name: String, breed: u8, sex: u8) {
    *OWN.lock().unwrap_or_else(|e| e.into_inner()) = (name, breed, sex);
}

/// FUN_1008c472(1): the mood of `VoiceSndFxType` (the character's preference, 0 by `CharPrefs.xml`).
fn mood() -> &'static str {
    let kind = setting("cd_image/gui/Default/CharPrefs.xml", "VoiceSndFxType").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    match kind {
        1 => "distunguished",
        2 => "cool",
        3 => "military",
        _ => "simple",
    }
}

/// Record 1 packed (FUN_10089628, `BBBSS`: the tag 1, breed, sex, mood, line), none for an empty line.
pub fn pack(v: &Voice) -> Vec<u8> {
    if v.line.is_empty() {
        return Vec::new();
    }
    let mut out = vec![1, v.breed, v.sex];
    for s in [&v.mood, &v.line] {
        out.extend((s.len() as u16).to_be_bytes());
        out.extend(s.as_bytes());
    }
    out
}

/// FUN_1008c5db: a voice text's leading `[tags]` taken off.
fn untagged(text: &str) -> &str {
    match (text.starts_with('['), text.find(']')) {
        (true, Some(end)) => &text[end + 1..],
        _ => text,
    }
}

/// `/voice <line>` (FUN_100b88b1): the text to send and its blob, or the line to tell why not.
pub fn command(line: &str) -> Result<(String, Vec<u8>), String> {
    if STAT_389.load(Ordering::Relaxed) & 1 == 0 {
        return Err("This function requires Notum Wars.".into());
    }
    let (name, breed, sex) = OWN.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let voice = Voice { breed, sex, mood: mood().to_string(), line: line.to_string() };
    let dir = crate::theme::game_dir().ok_or("You do not have that voice effect installed.")?.join("cd_image/sound/sfx/player");
    let names: Vec<String> = candidates(&name, &voice).into_iter().map(|n| n.replace("..", "")).collect();
    // (FUN_1008cbc2: installed when a name has both its sound and its text)
    if !names.iter().any(|n| dir.join(format!("{n}.wav")).exists() && dir.join(format!("{n}.txt")).exists()) {
        return Err("You do not have that voice effect installed.".into());
    }
    // (FUN_1008cfe3: the first line of the first text that opens, up to 1023 characters)
    let text = names.iter().find_map(|n| std::fs::read(dir.join(format!("{n}.txt"))).ok()).map(|b| {
        let s = String::from_utf8_lossy(&b).into_owned();
        let first: String = s.split_inclusive('\n').next().unwrap_or("").chars().take(1023).collect();
        first.trim_end_matches(['\r', '\n']).to_string()
    });
    let text = untagged(&text.unwrap_or_default()).to_string();
    Ok((text, pack(&voice)))
}

/// The blob's voice and its flag (record 2).
pub fn parse(blob: &[u8]) -> (Option<Voice>, u8) {
    let mut at = 0;
    let (mut voice, mut flag) = (None, 0);
    let byte = |at: &mut usize| -> Option<u8> {
        let b = *blob.get(*at)?;
        *at += 1;
        Some(b)
    };
    let text = |at: &mut usize| -> Option<String> {
        let n = u16::from_be_bytes(blob.get(*at..*at + 2)?.try_into().ok()?) as usize;
        let s = blob.get(*at + 2..*at + 2 + n)?;
        *at += 2 + n;
        Some(String::from_utf8_lossy(s).into_owned())
    };
    while let Some(tag) = byte(&mut at) {
        match tag {
            0 => {
                if blob.len() < at + 12 {
                    break;
                }
                at += 12;
                if text(&mut at).is_none() || text(&mut at).is_none() {
                    break;
                }
            }
            1 => {
                let read = (|| Some(Voice { breed: byte(&mut at)?, sex: byte(&mut at)?, mood: text(&mut at)?, line: text(&mut at)? }))();
                match read {
                    Some(v) => voice = Some(v),
                    None => break,
                }
            }
            2 => match byte(&mut at) {
                Some(b) => flag = b,
                None => break,
            },
            _ => break,
        }
    }
    (voice, flag)
}

/// The names to try (FUN_1008c847).
pub fn candidates(sender: &str, v: &Voice) -> Vec<String> {
    let breed = match v.breed {
        2 => "opifex",
        3 => "nano",
        4 => "atrox",
        _ => "solitus",
    };
    let sex = if v.sex == 3 { "female" } else { "male" };
    let (mood, line) = (&v.mood, &v.line);
    vec![
        format!("{sender}_{line}_01"),
        format!("{breed}_{sex}_{mood}_{line}_01"),
        format!("{sex}_{mood}_{line}_01"),
        format!("{sex}_{line}_01"),
        format!("{line}_01"),
    ]
}

/// The senders heard lately (FUN_1008c757, FUN_1008c7c0: their time in seconds).
#[derive(Default)]
pub struct Limiter {
    heard: HashMap<String, i64>,
}

impl Limiter {
    /// Whether `sender` may be heard at `now` (seconds), noting it if so.
    pub fn allow(&mut self, sender: &str, now: i64) -> bool {
        self.heard.retain(|_, t| now - *t < 2);
        if self.heard.get(sender).is_some_and(|t| now - *t < 3) {
            return false;
        }
        self.heard.insert(sender.to_string(), now);
        true
    }
}

/// A value of one of the GUI's preference files (`<Value name=... value=...>`).
fn setting(file: &str, name: &str) -> Option<String> {
    let text = std::fs::read_to_string(crate::theme::game_dir()?.join(file)).ok()?;
    let at = text.find(&format!("name=\"{name}\""))?;
    let tag = &text[text[..at].rfind('<')?..at + text[at..].find('>')?];
    let v = tag.find("value=\"")? + 7;
    Some(tag[v..v + tag[v..].find('"')?].to_string())
}

/// A preference of the GUI's `LoginPrefs.xml` (true or false).
fn preference(name: &str) -> Option<bool> {
    setting("cd_image/gui/Default/LoginPrefs.xml", name).map(|v| v == "true")
}

/// A group message from `sender` in a group of `kind` with `blob`: its voice line played as FUN_1008cdb3 plays it.
pub fn group_message(limiter: &mut Limiter, sender: &str, kind: u8, blob: &[u8]) {
    let allowed = match kind {
        3 => preference("VoiceSndFxHearGuildOn").unwrap_or(false),
        0x82 => preference("VoiceSndFxHearTeamOn").unwrap_or(false),
        _ => true,
    };
    if allowed {
        play(limiter, sender, parse(blob).0);
    }
}

/// Vicinity speech from `sender` (empty for an anonymous one) with `blob`: after its leading byte the records
/// (`HandleVicinityMessage` reads them when the blob has more than the byte), the voice played under
/// `VoiceSndFxHearVicinityOn`.
pub fn vicinity_message(limiter: &mut Limiter, sender: &str, blob: &[u8]) {
    if blob.len() > 1 && preference("VoiceSndFxHearVicinityOn").unwrap_or(false) {
        play(limiter, sender, parse(&blob[1..]).0);
    }
}

/// FUN_1008cdb3: a voice line played (the client's stat 389 bit 0, a line, the sender not heard lately).
fn play(limiter: &mut Limiter, sender: &str, voice: Option<Voice>) {
    if STAT_389.load(Ordering::Relaxed) & 1 == 0 {
        return;
    }
    let Some(voice) = voice else { return };
    if voice.line.is_empty() {
        return;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    if !limiter.allow(sender, now) {
        return;
    }
    let Some(game) = crate::theme::game_dir() else { return };
    for name in candidates(sender, &voice) {
        let name = name.replace("..", "");
        if game.join("cd_image/sound/sfx/player").join(format!("{name}.wav")).exists() {
            crate::audio::with(|m| m.play_player_fx(&format!("{name}.wav"), 1.0));
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(t: &str) -> Vec<u8> {
        let mut v = (t.len() as u16).to_be_bytes().to_vec();
        v.extend(t.as_bytes());
        v
    }

    #[test]
    fn a_voice_blob() {
        // (a record 0 to skip, the voice, the flag)
        let mut blob = vec![0];
        blob.extend([0u8; 12]);
        blob.extend(s("a"));
        blob.extend(s("b"));
        blob.extend([1, 4, 2]);
        blob.extend(s("cool"));
        blob.extend(s("heal"));
        blob.extend([2, 1]);
        let (v, flag) = parse(&blob);
        let v = v.unwrap();
        assert_eq!((v.breed, v.sex, v.mood.as_str(), v.line.as_str(), flag), (4, 2, "cool", "heal", 1));
        assert_eq!(candidates("Leet", &v), ["Leet_heal_01", "atrox_male_cool_heal_01", "male_cool_heal_01", "male_heal_01", "heal_01"]);
        // (a short record ends it)
        assert_eq!(parse(&[1, 4]), (None, 0));
    }

    #[test]
    fn a_spoken_voice_s_record_and_text() {
        let v = Voice { breed: 4, sex: 2, mood: "cool".into(), line: "heal".into() };
        let blob = pack(&v);
        assert_eq!(blob, [&[1u8, 4, 2, 0, 4][..], b"cool", &[0, 4], b"heal"].concat());
        // (what the receiving side reads back)
        assert_eq!(parse(&blob).0, Some(v));
        assert!(pack(&Voice::default()).is_empty());
        assert_eq!(untagged("[T]Medic!"), "Medic!");
        assert_eq!(untagged("[TM]Run!"), "Run!");
        assert_eq!(untagged("Yes!"), "Yes!");
        set_stat_389(0);
        assert_eq!(command("heal").unwrap_err(), "This function requires Notum Wars.");
    }

    #[test]
    fn one_line_per_sender_in_3_seconds() {
        let mut l = Limiter::default();
        assert!(l.allow("a", 100));
        assert!(!l.allow("a", 101));
        assert!(l.allow("b", 101));
        // (forgotten after 2 s)
        assert!(l.allow("a", 102));
    }
}
