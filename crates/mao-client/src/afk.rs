//! Away from the keyboard: `/afk`, the automatic answer to tells, and going away by itself after three minutes
//! without input. GUI.dll's, read on 2026-11-12:
//!
//! * **The state** (`ChatGUIModule_c`): the away message at +0x294. The character is away while it is not empty
//!   (`IsAFK` 0x1001b7c2); +0x2b0 is when it went away (`SetAFK` 0x10087669 notes the time when it was not away
//!   before, and leaves a battle station's queue, which this client has none of).
//! * **`/afk`** (FUN_100b82a7). `/afk status` says the message, or that the character is not away. `/afk` alone while
//!   away ends it: a line to oneself and "is back" as an emote to the vicinity. `/afk` alone while not away goes away
//!   with the default message at once and opens a box to change it ([`Dialog`]). `/afk <text>` goes away with the
//!   text (its `%t`, `%f`, `%m` put in), or changes the message when away already.
//! * **The box** (`AFKMessageDialog_c` FUN_1008307e, its view FUN_100836ab, its answer FUN_100b6b65): the body text,
//!   a count down from 30 seconds, the message to edit and one button. A key in the field stops the count
//!   (FUN_1008356e); Enter, the button and the count's end take the text (an empty one keeps the default message);
//!   Escape ends being away. While it is up, input does not count as coming back (`InputConfig_t::AFKIgnoreInput`).
//! * **Tells while away** (`HandlePrivateMessage` 0x10087e59): the tell is shown with "(AFK reply sent)" after it and
//!   answered with `ChatCmdFeedback_AFKReplyText` fed the character's name, the hours and minutes away and the
//!   message. The answer is a tell whose blob begins with the byte 6 ([`mao_net::chat::AWAY_REPLY`]): the client that
//!   gets it shows it without a name in front and never answers it, so two characters who are both away do not
//!   answer each other for ever.
//! * **Going away by itself** (`InputConfig_t::AddUserInput` 0x1001b115, `SlotAFKTimer` 0x1001a2cb,
//!   `ChatGUIModule_c::SlotAutoAFK` 0x100870a8): every key or button press and every movement of the mouse (the
//!   program's FUN_0040383a and FUN_00404a66; a release does not count) restarts a timer of 180 seconds. When it runs
//!   out and the `AutoAFK` setting is on (it is in the game's own `LoginPrefs.xml`) the character is away with the
//!   message "Auto-away". **With the setting on, any input ends being away**, however it began: `/afk brb` lasts until
//!   the mouse is next moved. A game master (stat 215 not 0) is left out of both, and a sneaking character
//!   (movement mode 6) says nothing to the vicinity.
//!
//! The texts are text.mdb's (10001, by name) where the player's install has them; without it, this client's own
//! words. The original has no control for `AutoAFK` in its option panel; here `/setoption AutoAFK off` sets it.

use mao_formats::ldb::{Arg, format};
use std::time::{Duration, Instant};

/// How long without input before the character is away by itself (`AddUserInput`'s timer: 180,000,000 microseconds).
pub const IDLE: Duration = Duration::from_secs(180);
/// The box's count down (FUN_10083496: thirty seconds from when it opened).
pub const COUNT: Duration = Duration::from_secs(30);
/// The message of a character that went away by itself (`SlotAutoAFK`'s own text).
pub const AUTO_MESSAGE: &str = "Auto-away";

/// Whether the player's character is sneaking (the game connection keeps it): `SlotAutoAFK` says nothing to the
/// vicinity then.
pub static SNEAKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A text of the away commands (text.mdb 10001 by name), or this client's own words.
pub fn text(name: &str) -> String {
    let own = match name {
        "ChatCmdFeedback_AFK_Status_AFKIsCurrentlyOn" => "You are away. Your message: %s",
        "ChatCmdFeedback_AFK_Status_AFKIsCurrentlyOff" => "You are not away.",
        "ChatCmdFeedback_AFK_AFKOff" => "You are no longer away.",
        "ChatCmdFeedback_AFK_AFKOn" => "You are away. Type /afk again when you are back.",
        "ChatCmdFeedback_AFK_IsBack" => "is back",
        "ChatCmdFeedback_AFK_IsAFK" => "is away from the keyboard",
        "ChatCmdFeedback_DefaultAFKReply" => "I am away from the keyboard.",
        "ChatCmdFeedback_AFKDialogBody" => "You are away. Those who send you a tell get this message; change it if you like.",
        "ChatCmdFeedback_ChangedAFKMessageTo" => "Away message changed from '%s' to '%s'.",
        "ChatCmdFeedback_AFKReplyText" => "%s has been away for %d h %d min: %s",
        _ => name,
    };
    crate::gui::xml::ldb_named(10001, name, own)
}

/// What a step asks of the rest of the client: lines for the player's own chat, and a line for the vicinity as an
/// emote (speech of kind 3, as `/me`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Did {
    pub lines: Vec<String>,
    pub emote: Option<String>,
}

impl Did {
    fn line(text: String) -> Did {
        Did { lines: vec![text], emote: None }
    }
}

/// The box `/afk` alone opens.
#[derive(Clone, Debug)]
pub struct Dialog {
    /// The body text, and the message being edited.
    pub body: String,
    pub message: String,
    opened: Instant,
    /// Whether the count down still runs (a key in the field stops it).
    pub counting: bool,
    /// The frames it has been drawn (it is put in the screen's middle on the first ones).
    pub shown: u8,
}

impl Dialog {
    /// The seconds the count down shows (FUN_10083496: thirty less the whole seconds since it opened), `None` once
    /// it is stopped.
    pub fn count(&self, now: Instant) -> Option<u64> {
        self.counting.then(|| COUNT.as_secs().saturating_sub(now.saturating_duration_since(self.opened).as_secs()))
    }
}

#[derive(Debug, Default)]
pub struct Away {
    message: String,
    since: Option<Instant>,
    pub dialog: Option<Dialog>,
    /// The last input, and whether the timer it started has run out since.
    last_input: Option<Instant>,
    ran_out: bool,
}

impl Away {
    pub fn is_away(&self) -> bool {
        !self.message.is_empty()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// `SetAFK`: the message; the time is noted when the character was not away before.
    fn set(&mut self, message: &str, now: Instant) {
        if !self.is_away() {
            self.since = Some(now);
        }
        self.message = message.to_string();
    }

    /// `/afk` with what follows it (FUN_100b82a7); `expand` puts a text's `%` arguments in.
    pub fn command(&mut self, rest: &str, expand: &dyn Fn(&str) -> String, now: Instant) -> Did {
        let rest = rest.trim();
        let was = self.is_away();
        let mut words = rest.split_whitespace();
        if let (Some(word), None) = (words.next(), words.next())
            && word.eq_ignore_ascii_case("status")
        {
            return Did::line(if was { format(&text("ChatCmdFeedback_AFK_Status_AFKIsCurrentlyOn"), &[Arg::Text(&self.message)]) } else { text("ChatCmdFeedback_AFK_Status_AFKIsCurrentlyOff") });
        }
        if was && rest.is_empty() {
            self.set("", now);
            return Did { lines: vec![text("ChatCmdFeedback_AFK_AFKOff")], emote: Some(text("ChatCmdFeedback_AFK_IsBack")) };
        }
        let message = if rest.is_empty() {
            let message = text("ChatCmdFeedback_DefaultAFKReply");
            self.dialog = Some(Dialog { body: text("ChatCmdFeedback_AFKDialogBody"), message: message.clone(), opened: now, counting: true, shown: 0 });
            message
        } else {
            expand(rest)
        };
        let did = if !was {
            Did { lines: vec![text("ChatCmdFeedback_AFK_AFKOn")], emote: Some(text("ChatCmdFeedback_AFK_IsAFK")) }
        } else if message != self.message {
            Did::line(format(&text("ChatCmdFeedback_ChangedAFKMessageTo"), &[Arg::Text(&self.message), Arg::Text(&message)]))
        } else {
            Did::default()
        };
        self.set(&message, now);
        did
    }

    /// The box answered (FUN_100b6b65): `accepted` for Enter, the button or the count's end, with the text of its
    /// field; otherwise Escape, which ends being away.
    pub fn answer(&mut self, accepted: bool, typed: &str, now: Instant) -> Did {
        self.dialog = None;
        if !accepted {
            self.set("", now);
            return Did { lines: vec!["AFK off.".to_string()], emote: Some("is back".to_string()) };
        }
        if typed.is_empty() {
            return Did::line("Using default AFK message.".to_string());
        }
        if typed == self.message {
            return Did::default();
        }
        let did = Did::line(format(&text("ChatCmdFeedback_ChangedAFKMessageTo"), &[Arg::Text(&self.message), Arg::Text(typed)]));
        self.set(typed, now);
        did
    }

    /// A key or button press, or a movement of the mouse (`AddUserInput`): with the setting on, the character is
    /// back (`SlotAutoAFK(false)`), unless the box is up; and the timer starts again. `auto` is the `AutoAFK`
    /// setting; a game master is left out.
    pub fn input(&mut self, auto: bool, game_master: bool, sneaking: bool, now: Instant) -> Did {
        self.last_input = Some(now);
        self.ran_out = false;
        if self.dialog.is_some() || game_master || !auto || !self.is_away() {
            return Did::default();
        }
        self.message.clear();
        Did { lines: vec![text("ChatCmdFeedback_AFK_AFKOff")], emote: (!sneaking).then(|| text("ChatCmdFeedback_AFK_IsBack")) }
    }

    /// Each frame: the box's count running out takes its text, and the timer of the last input running out puts the
    /// character away by itself (`SlotAFKTimer` → `SlotAutoAFK(true)`). There is no timer before the first input.
    pub fn tick(&mut self, auto: bool, game_master: bool, sneaking: bool, idle: Duration, now: Instant) -> Did {
        if let Some(d) = &self.dialog
            && d.counting
            && now.saturating_duration_since(d.opened) >= COUNT
        {
            let typed = d.message.clone();
            return self.answer(true, &typed, now);
        }
        let Some(last) = self.last_input else { return Did::default() };
        if self.ran_out || now.saturating_duration_since(last) < idle {
            return Did::default();
        }
        self.ran_out = true;
        if game_master || !auto || self.is_away() {
            return Did::default();
        }
        self.since = Some(now);
        self.message = AUTO_MESSAGE.to_string();
        Did { lines: Vec::new(), emote: (!sneaking).then(|| text("ChatCmdFeedback_AFK_IsAFK")) }
    }

    /// The answer to a tell that came while away (`HandlePrivateMessage`): `ChatCmdFeedback_AFKReplyText` fed the
    /// character's name, the hours and the minutes away, and the message. `None` when not away.
    pub fn reply(&self, own: &str, now: Instant) -> Option<String> {
        if !self.is_away() {
            return None;
        }
        let seconds = self.since.map_or(0, |s| now.saturating_duration_since(s).as_secs()) as i64;
        Some(format(&text("ChatCmdFeedback_AFKReplyText"), &[Arg::Text(own), Arg::Int(seconds / 3600), Arg::Int(seconds / 60 % 60), Arg::Text(&self.message)]))
    }
}

/// How long without input before going away: [`IDLE`], or `MAO_AFK_IDLE=<seconds>` for a trial.
pub fn idle() -> Duration {
    static IDLE_NOW: std::sync::OnceLock<Duration> = std::sync::OnceLock::new();
    *IDLE_NOW.get_or_init(|| std::env::var("MAO_AFK_IDLE").ok().and_then(|s| s.trim().parse::<f32>().ok()).filter(|s| *s > 0.0).map_or(IDLE, Duration::from_secs_f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(text: &str) -> String {
        text.to_string()
    }

    #[test]
    fn afk_with_a_text_goes_away_and_alone_comes_back() {
        let t = Instant::now();
        let mut a = Away::default();
        assert_eq!(a.command("status", &same, t).lines, vec![text("ChatCmdFeedback_AFK_Status_AFKIsCurrentlyOff")]);
        // (with a text: away with it, a line to oneself and an emote to the vicinity; no box)
        let did = a.command("  gone for tea ", &|s| s.replace("tea", "coffee"), t);
        assert_eq!(did, Did { lines: vec![text("ChatCmdFeedback_AFK_AFKOn")], emote: Some(text("ChatCmdFeedback_AFK_IsAFK")) });
        assert!(a.is_away() && a.dialog.is_none());
        assert_eq!(a.message(), "gone for coffee");
        assert_eq!(a.command("STATUS", &same, t).lines, vec![format(&text("ChatCmdFeedback_AFK_Status_AFKIsCurrentlyOn"), &[Arg::Text("gone for coffee")])]);
        // (another text while away changes the message and says so, with no emote; the same text says nothing)
        let did = a.command("back at nine", &same, t);
        assert_eq!(did, Did::line(format(&text("ChatCmdFeedback_ChangedAFKMessageTo"), &[Arg::Text("gone for coffee"), Arg::Text("back at nine")])));
        assert_eq!(a.command("back at nine", &same, t), Did::default());
        // ("status" with more after it is a message like any other)
        a.command("status report", &same, t);
        assert_eq!(a.message(), "status report");
        // (alone while away: back)
        let did = a.command("", &same, t);
        assert_eq!(did, Did { lines: vec![text("ChatCmdFeedback_AFK_AFKOff")], emote: Some(text("ChatCmdFeedback_AFK_IsBack")) });
        assert!(!a.is_away());
    }

    #[test]
    fn afk_alone_opens_the_box_and_is_away_meanwhile() {
        let t = Instant::now();
        let mut a = Away::default();
        let did = a.command("", &same, t);
        assert_eq!(did.emote, Some(text("ChatCmdFeedback_AFK_IsAFK")));
        assert_eq!(a.message(), text("ChatCmdFeedback_DefaultAFKReply"));
        let d = a.dialog.clone().unwrap();
        assert_eq!((d.count(t), d.count(t + Duration::from_millis(1500)), d.count(t + Duration::from_secs(40))), (Some(30), Some(29), Some(0)));
        // (input does not end being away while the box is up)
        assert_eq!(a.input(true, false, false, t), Did::default());
        assert!(a.is_away());
        // (a new text is taken and said; an empty one keeps the default; the same one says nothing)
        let did = a.answer(true, "lunch", t);
        assert_eq!(did, Did::line(format(&text("ChatCmdFeedback_ChangedAFKMessageTo"), &[Arg::Text(&text("ChatCmdFeedback_DefaultAFKReply")), Arg::Text("lunch")])));
        assert!(a.dialog.is_none() && a.message() == "lunch");
        assert_eq!(a.answer(true, "", t).lines, vec!["Using default AFK message."]);
        assert_eq!(a.answer(true, "lunch", t), Did::default());
        // (Escape ends being away)
        assert_eq!(a.answer(false, "whatever", t), Did { lines: vec!["AFK off.".into()], emote: Some("is back".into()) });
        assert!(!a.is_away());
        // (the count's end takes the text as it stands; a stopped count does not end)
        a.command("", &same, t);
        a.dialog.as_mut().unwrap().counting = false;
        assert_eq!(a.tick(true, false, false, IDLE, t + Duration::from_secs(31)), Did::default());
        assert!(a.dialog.is_some());
        a.dialog.as_mut().unwrap().counting = true;
        a.dialog.as_mut().unwrap().message = "typed".into();
        assert_eq!(a.tick(true, false, false, IDLE, t + Duration::from_secs(29)), Did::default());
        assert_eq!(a.tick(true, false, false, IDLE, t + Duration::from_secs(30)).lines.len(), 1);
        assert!(a.dialog.is_none() && a.message() == "typed");
    }

    #[test]
    fn three_minutes_without_input_is_away_and_input_is_back() {
        let t = Instant::now();
        let mut a = Away::default();
        // (no timer before the first input)
        assert_eq!(a.tick(true, false, false, IDLE, t + Duration::from_secs(1000)), Did::default());
        assert!(!a.is_away());
        a.input(true, false, false, t);
        assert_eq!(a.tick(true, false, false, IDLE, t + Duration::from_secs(179)), Did::default());
        // (away by itself: an emote to the vicinity and no line to oneself)
        assert_eq!(a.tick(true, false, false, IDLE, t + Duration::from_secs(180)), Did { lines: vec![], emote: Some(text("ChatCmdFeedback_AFK_IsAFK")) });
        assert_eq!(a.message(), AUTO_MESSAGE);
        assert_eq!(a.tick(true, false, false, IDLE, t + Duration::from_secs(400)), Did::default());
        // (the reply counts from when it went away)
        let reply = a.reply("Tester", t + Duration::from_secs(180 + 3600 + 125)).unwrap();
        assert_eq!(reply, format(&text("ChatCmdFeedback_AFKReplyText"), &[Arg::Text("Tester"), Arg::Int(1), Arg::Int(2), Arg::Text(AUTO_MESSAGE)]));
        // (any input is back)
        assert_eq!(a.input(true, false, false, t + Duration::from_secs(500)), Did { lines: vec![text("ChatCmdFeedback_AFK_AFKOff")], emote: Some(text("ChatCmdFeedback_AFK_IsBack")) });
        assert!(!a.is_away() && a.reply("Tester", t).is_none());
        // (a sneaking character says nothing to the vicinity, going or coming)
        assert_eq!(a.tick(true, false, true, IDLE, t + Duration::from_secs(700)), Did::default());
        assert!(a.is_away());
        assert_eq!(a.input(true, false, true, t + Duration::from_secs(701)).emote, None);
        // (with the setting on, input ends an away the player asked for too)
        a.command("brb", &same, t);
        assert_eq!(a.input(true, false, false, t).lines.len(), 1);
        assert!(!a.is_away());
        // (with the setting off, neither: the character stays as the player left it)
        a.command("brb", &same, t);
        assert_eq!(a.input(false, false, false, t), Did::default());
        assert!(a.is_away());
        a.command("", &same, t);
        assert_eq!(a.tick(false, false, false, IDLE, t + Duration::from_secs(900)), Did::default());
        assert!(!a.is_away());
        // (a game master is left out of both)
        a.input(true, true, false, t);
        assert_eq!(a.tick(true, true, false, IDLE, t + Duration::from_secs(900)), Did::default());
        assert!(!a.is_away());
    }
}
