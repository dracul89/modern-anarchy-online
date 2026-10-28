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
//! Not here: vicinity speech (`HandleVicinityMessage`, the same voice after a leading type byte, under
//! `VoiceSndFxHearVicinityOn`: the client shows no vicinity speech yet) and sending a voice (the `/voice`-style command
//! FUN_100b88b1 → FUN_1008cbc2).

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

/// A preference of the GUI's `LoginPrefs.xml` (true or false).
fn preference(name: &str) -> Option<bool> {
    static TEXT: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    let text = TEXT.get_or_init(|| std::fs::read_to_string(crate::theme::game_dir()?.join("cd_image/gui/Default/LoginPrefs.xml")).ok()).as_ref()?;
    let at = text.find(&format!("name=\"{name}\""))?;
    let tag = &text[text[..at].rfind('<')?..at + text[at..].find('>')?];
    let v = tag.find("value=\"")? + 7;
    Some(&tag[v..v + tag[v..].find('"')?] == "true")
}

/// A group message from `sender` in a group of `kind` with `blob`: its voice line played as FUN_1008cdb3 plays it.
pub fn group_message(limiter: &mut Limiter, sender: &str, kind: u8, blob: &[u8]) {
    if STAT_389.load(Ordering::Relaxed) & 1 == 0 {
        return;
    }
    let allowed = match kind {
        3 => preference("VoiceSndFxHearGuildOn").unwrap_or(false),
        0x82 => preference("VoiceSndFxHearTeamOn").unwrap_or(false),
        _ => true,
    };
    let (Some(voice), _) = parse(blob) else { return };
    if !allowed || voice.line.is_empty() {
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
    fn one_line_per_sender_in_3_seconds() {
        let mut l = Limiter::default();
        assert!(l.allow("a", 100));
        assert!(!l.allow("a", 101));
        assert!(l.allow("b", 101));
        // (forgotten after 2 s)
        assert!(l.allow("a", 102));
    }
}
