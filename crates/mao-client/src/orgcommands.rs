//! `/org <command> [text]`: the organization's commands. Gamecode's, read on 2026-11-12 (GUI.dll hands `/org` to
//! `N3Msg_TextCommand`, whose FUN_1003fe10 does this for the word `org`; the organization's chat is `/o`):
//!
//! * The first word is looked up (FUN_10032366; the table is filled at 0x100334a0 on) and the rest is the command's
//!   text. A text with a character outside 0x20 to 0x7e is refused with `CannotUseLettersX` naming them.
//! * Most commands make one message, `OrgClientIIR_c` (FUN_101266b4; packed by FUN_1012653f: the command's number,
//!   an identity, the chat window's number, then a text for the commands that have one and a byte for number 10),
//!   with the selected target as the identity: [`mao_net::n3::org_command`]. The numbers are in [`step`].
//! * `help`, and a word that is no command, say the game's `OrgCommandHelp`; `startvote` and `vote` without a text
//!   say `OrgVoteHelp` and `OrgVoteHelpInfo`.
//! * `leave` and `disband` ask first (boxes the interface shows, answered through `N3Msg_OrgLeaveConfirmed` and
//!   `N3Msg_OrgDisbandConfirmed`): not carried out here.
//!
//! Tried live on 2026-11-12: `info`, `ranks` and `contract` only read. Outside an organization the server answers
//! each with a line; in one, `ranks` is answered with a line (a packed text with the ranks after it), and `info`
//! and `contract` with `OrgServer` messages (types 2 and 1), which this client does not show yet.
//!
//! **`history`, `description`, `objective`, `name` and `governingform` set that text of the organization**, and
//! they are sent with whatever follows, nothing included, as the original sends them: `/org history` alone sets an
//! empty history (seen live: "Organization info updated."). For a day this client would not send them without a
//! text; the user chose the original's way on 2026-11-12 (`docs/ORIGINAL_CODE_QUESTIONS.md`, 16). What `tax` alone
//! does was not tried.

use mao_formats::ldb::{Arg, format};

/// What a typed `/org` line comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// A line for the player.
    Say(String),
    /// The message to send: the command's number, whether the selected target goes with it, and its text.
    Send { command: u8, targeted: bool, text: String },
    /// A command this client does not carry out yet.
    NotDone(&'static str),
    Nothing,
}

/// A text of the game's (text.mdb 110 by name), or this client's own words.
fn game_text(name: &str) -> String {
    let own = match name {
        "OrgCommandHelp" => "Organization commands: /org info, ranks, contract, history, tax, bank, debt, city, invite, kick, promote, demote, name, description, objective, governingform, startvote, vote, stopvote, create, leave, disband.",
        "OrgVoteHelp" => "/org startvote <the vote's text and choices>",
        "OrgVoteHelpInfo" => "/org vote info, or /org vote <choice>",
        "CannotUseLettersX" => "These letters cannot be used: %s",
        _ => name,
    };
    crate::feedback::texts().and_then(|t| t.named(mao_formats::ldb::FEEDBACK, name)).map_or_else(|| own.to_string(), str::to_string)
}

/// `/org` with what follows it.
pub fn step(rest: &str) -> Step {
    let rest = rest.trim();
    let (word, text) = match rest.split_once(char::is_whitespace) {
        Some((w, t)) => (w.to_lowercase(), t.trim()),
        None => (rest.to_lowercase(), ""),
    };
    // (the letters that cannot be sent, each once, with ", " between)
    let mut bad: Vec<String> = Vec::new();
    for c in text.chars().filter(|c| !(' '..='~').contains(c)) {
        if !bad.contains(&c.to_string()) {
            bad.push(c.to_string());
        }
    }
    if !bad.is_empty() {
        return Step::Say(format(&game_text("CannotUseLettersX"), &[Arg::Text(&bad.join(", "))]));
    }
    let send = |command: u8, targeted: bool, text: &str| Step::Send { command, targeted, text: text.to_string() };
    match word.as_str() {
        "create" => send(1, true, text),
        "ranks" => send(2, false, ""),
        "contract" => send(3, false, ""),
        "info" => send(5, true, ""),
        "promote" => send(0xa, true, ""),
        "demote" => send(0xb, true, ""),
        "invite" => send(0xe, true, ""),
        // (the target, or the member named)
        "kick" if text.is_empty() => send(0xc, true, ""),
        "kick" => send(0xd, true, text),
        "tax" => send(0x11, true, text),
        "bank" => match text.split_once(char::is_whitespace).map_or((text, ""), |(a, b)| (a, b.trim())) {
            ("", _) => send(0x12, true, ""),
            (what, amount) if what.eq_ignore_ascii_case("add") && !amount.is_empty() => send(0x13, true, amount),
            (what, amount) if what.eq_ignore_ascii_case("remove") && !amount.is_empty() => send(0x14, true, amount),
            _ => Step::Nothing,
        },
        "debt" => send(0x16, false, ""),
        "history" => send(0x17, true, text),
        "objective" => send(0x18, true, text),
        "description" => send(0x19, true, text),
        "name" => send(0x1a, true, text),
        "governingform" => send(0x1b, true, text),
        "startvote" if text.is_empty() => Step::Say(game_text("OrgVoteHelp")),
        "startvote" => send(7, true, text),
        "vote" if text.is_empty() => Step::Say(game_text("OrgVoteHelpInfo")),
        "vote" if text.split_whitespace().next().is_some_and(|w| w.eq_ignore_ascii_case("info")) => send(8, true, text),
        "vote" => send(9, true, text),
        "stopvote" => send(0x1c, true, ""),
        "city" => send(0x1f, false, ""),
        "leave" => Step::NotDone("/org leave asks first, in a box this client has not: not carried out yet."),
        "disband" => Step::NotDone("/org disband asks first, in a box this client has not: not carried out yet."),
        _ => Step::Say(game_text("OrgCommandHelp")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(rest: &str) -> (u8, bool, String) {
        match step(rest) {
            Step::Send { command, targeted, text } => (command, targeted, text),
            other => panic!("{rest}: {other:?}"),
        }
    }

    #[test]
    fn org_commands_come_to_their_messages() {
        assert_eq!(sent("info"), (5, true, String::new()));
        assert_eq!(sent("RANKS"), (2, false, String::new()));
        assert_eq!(sent("contract"), (3, false, String::new()));
        assert_eq!((sent("kick"), sent("kick  Somebody")), ((0xc, true, String::new()), (0xd, true, "Somebody".into())));
        assert_eq!((sent("promote"), sent("demote"), sent("invite")), ((0xa, true, String::new()), (0xb, true, String::new()), (0xe, true, String::new())));
        assert_eq!((sent("bank"), sent("bank add 500"), sent("bank REMOVE 20")), ((0x12, true, String::new()), (0x13, true, "500".into()), (0x14, true, "20".into())));
        assert_eq!(step("bank add"), Step::Nothing);
        assert_eq!((sent("tax 5"), sent("history 2"), sent("debt"), sent("city")), ((0x11, true, "5".into()), (0x17, true, "2".into()), (0x16, false, String::new()), (0x1f, false, String::new())));
        assert_eq!((sent("name New Name"), sent("objective to win"), sent("description a b"), sent("governingform republic"), sent("create Us")), ((0x1a, true, "New Name".into()), (0x18, true, "to win".into()), (0x19, true, "a b".into()), (0x1b, true, "republic".into()), (1, true, "Us".into())));
        assert_eq!((sent("startvote Who? :: a :: b"), sent("vote info"), sent("vote a"), sent("stopvote")), ((7, true, "Who? :: a :: b".into()), (8, true, "info".into()), (9, true, "a".into()), (0x1c, true, String::new())));
        // (help, a word that is no command, and the votes without a text say the game's texts)
        assert_eq!(step("help"), Step::Say(game_text("OrgCommandHelp")));
        assert_eq!(step("hello everybody"), Step::Say(game_text("OrgCommandHelp")));
        assert_eq!(step(""), Step::Say(game_text("OrgCommandHelp")));
        assert_eq!((step("startvote"), step("vote")), (Step::Say(game_text("OrgVoteHelp")), Step::Say(game_text("OrgVoteHelpInfo"))));
        assert!(matches!(step("leave"), Step::NotDone(_)) && matches!(step("disband"), Step::NotDone(_)));
        // (the commands that set a text are sent with none when none is given, as the original sends them: that
        // empties the text)
        assert_eq!((sent("history"), sent("description"), sent("objective")), ((0x17, true, String::new()), (0x19, true, String::new()), (0x18, true, String::new())));
        assert_eq!((sent("name"), sent("governingform"), sent("create"), sent("tax")), ((0x1a, true, String::new()), (0x1b, true, String::new()), (1, true, String::new()), (0x11, true, String::new())));
        // (letters that cannot be sent are named, once each, and nothing goes)
        assert_eq!(step("name Caf\u{e9} \u{e9}t\u{e8}"), Step::Say(format(&game_text("CannotUseLettersX"), &[Arg::Text("\u{e9}, \u{e8}")])));
    }
}
