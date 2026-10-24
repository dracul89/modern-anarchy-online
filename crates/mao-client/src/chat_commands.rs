//! The chat commands of the original game (the list its input line offers when a `/` is typed) and what this client does with
//! them: a typed line becomes a [`Parsed`].

/// Every command the original lists, in its order (the case is the original's).
pub const COMMANDS: &[&str] = &[
    "afk", "aliens", "anon", "aou_about", "aou_gui_default", "aou_gui_default_1", "aou_gui_slim", "aou_gui_slim_1", "aou_gui_slim_2", "aou_gui_slim_3",
    "aou_gui_translucent", "AOU_mod", "ao_original", "APanel", "assist", "AVbasic", "bank", "Befriend", "born", "bug", "camp", "cc", "ch", "char&viewdist",
    "chardist", "close", "custom_maps", "duel", "dvalue", "emote", "filter", "follow", "FriendshipInvite", "fxscript", "g", "gfx", "group", "help", "ignore",
    "inspect", "invite", "items", "kick", "leave", "lft", "Link_URL_Description", "list", "lof", "macro", "me", "MapMarker", "name", "o", "open", "option",
    "org", "petduel", "petition", "played", "quit", "r", "raid", "reclaim", "reply", "rp", "s", "say", "script", "selectself", "setoption", "shop", "shout",
    "showfile", "stuck", "t", "team", "tell", "terminate", "text", "tipoftheday", "toggle", "tower", "version", "viewdist", "voice", "w", "waypoint", "whisper",
];

/// A one-line description of the commands this client carries out (for `/help`).
pub const IMPLEMENTED: &[(&str, &str)] = &[
    ("tell <name> <text>", "a private message (also /t)"),
    ("reply <text>", "answer the last tell (also /r)"),
    ("org <text>", "your organization's channel (also /o)"),
    ("befriend <name>", "put a character on your friends list (L shows it)"),
    ("ignore [name]", "hide a character's tells and channel lines (again: show them; alone: the list)"),
    ("camp", "sit down to log out (stand up or move to cancel)"),
    ("quit", "leave the client"),
    ("help", "this list"),
    ("version", "the client's version"),
];

/// What a typed line asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parsed {
    /// Nothing typed.
    Empty,
    /// A private message to a character.
    Tell { name: String, text: String },
    /// An answer to the last tell.
    Reply { text: String },
    /// `/befriend <name>`: a character to put on the friends list.
    Befriend { name: String },
    /// `/ignore <name>` (or `/ignore` alone: the list): hide a character's tells and channel lines.
    Ignore { name: String },
    /// `/camp`: sit down to log out.
    Camp,
    /// `/quit`: leave the client.
    Quit,
    /// A line for the organization channel.
    Org { text: String },
    /// Plain speech or /say, /shout, /whisper, /me: vicinity chat.
    Vicinity { command: &'static str, text: String },
    /// `/help`.
    Help,
    /// `/version`.
    Version,
    /// A command of the original's list this client does not carry out yet.
    Unsupported(String),
    /// A word that is not a command of the original.
    Unknown(String),
    /// A command missing something (the usage line).
    Usage(&'static str),
}

/// The commands that begin with what has been typed after the `/` (the first word only), in the original's order; empty once the
/// first word is complete and followed by a space.
pub fn suggest(input: &str) -> Vec<&'static str> {
    let Some(rest) = input.strip_prefix('/') else { return Vec::new() };
    if rest.contains(char::is_whitespace) {
        return Vec::new();
    }
    let prefix = rest.to_lowercase();
    COMMANDS.iter().copied().filter(|c| c.to_lowercase().starts_with(&prefix)).collect()
}

/// The command with this name (any case), as the original writes it.
fn known(word: &str) -> Option<&'static str> {
    COMMANDS.iter().copied().find(|c| c.eq_ignore_ascii_case(word))
}

pub fn parse(line: &str) -> Parsed {
    let line = line.trim();
    if line.is_empty() {
        return Parsed::Empty;
    }
    let Some(rest) = line.strip_prefix('/') else { return Parsed::Vicinity { command: "say", text: line.to_string() } };
    let (word, text) = match rest.split_once(char::is_whitespace) {
        Some((w, t)) => (w, t.trim()),
        None => (rest, ""),
    };
    let Some(command) = known(word) else { return Parsed::Unknown(word.to_string()) };
    match command.to_lowercase().as_str() {
        "tell" | "t" => {
            let (name, message) = match text.split_once(char::is_whitespace) {
                Some((n, m)) => (n, m.trim()),
                None => (text, ""),
            };
            if name.is_empty() || message.is_empty() {
                Parsed::Usage("/tell <name> <text>")
            } else {
                Parsed::Tell { name: name.to_string(), text: message.to_string() }
            }
        }
        "reply" | "r" => {
            if text.is_empty() {
                Parsed::Usage("/reply <text>")
            } else {
                Parsed::Reply { text: text.to_string() }
            }
        }
        "ignore" => Parsed::Ignore { name: text.split_whitespace().next().unwrap_or("").to_string() },
        "camp" => Parsed::Camp,
        "quit" => Parsed::Quit,
        "befriend" => {
            if text.is_empty() {
                Parsed::Usage("/befriend <name>")
            } else {
                Parsed::Befriend { name: text.split_whitespace().next().unwrap_or(text).to_string() }
            }
        }
        "org" | "o" => {
            if text.is_empty() {
                Parsed::Usage("/org <text>")
            } else {
                Parsed::Org { text: text.to_string() }
            }
        }
        "say" | "shout" | "whisper" | "s" | "w" | "me" | "emote" => {
            let name = match command.to_lowercase().as_str() {
                "s" | "shout" => "shout",
                "w" | "whisper" => "whisper",
                "me" | "emote" => "emote",
                _ => "say",
            };
            if text.is_empty() {
                Parsed::Usage("/say <text>")
            } else {
                Parsed::Vicinity { command: name, text: text.to_string() }
            }
        }
        "help" => Parsed::Help,
        "version" => Parsed::Version,
        _ => Parsed::Unsupported(command.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_whole_list_is_offered_for_a_bare_slash() {
        assert_eq!(suggest("/").len(), COMMANDS.len());
        assert_eq!(COMMANDS.len(), 88);
        assert_eq!(suggest("/aou_gui_s"), vec!["aou_gui_slim", "aou_gui_slim_1", "aou_gui_slim_2", "aou_gui_slim_3"]);
        assert_eq!(suggest("/TE"), vec!["team", "tell", "terminate", "text"]);
        assert!(suggest("/tell ").is_empty());
        assert!(suggest("hello").is_empty());
    }

    #[test]
    fn lines_become_commands() {
        assert_eq!(parse("/tell Abc hello there"), Parsed::Tell { name: "Abc".into(), text: "hello there".into() });
        assert_eq!(parse("/T Abc hi"), Parsed::Tell { name: "Abc".into(), text: "hi".into() });
        assert_eq!(parse("/tell Abc"), Parsed::Usage("/tell <name> <text>"));
        assert_eq!(parse("/r thanks"), Parsed::Reply { text: "thanks".into() });
        assert_eq!(parse("/ignore Abc"), Parsed::Ignore { name: "Abc".into() });
        assert_eq!(parse("/ignore"), Parsed::Ignore { name: String::new() });
        assert_eq!(parse("/camp"), Parsed::Camp);
        assert_eq!(parse("/QUIT"), Parsed::Quit);
        assert_eq!(parse("/befriend Abc"), Parsed::Befriend { name: "Abc".into() });
        assert_eq!(parse("/befriend"), Parsed::Usage("/befriend <name>"));
        assert_eq!(parse("/o hello org"), Parsed::Org { text: "hello org".into() });
        assert_eq!(parse("hello world"), Parsed::Vicinity { command: "say", text: "hello world".into() });
        assert_eq!(parse("/s HEY"), Parsed::Vicinity { command: "shout", text: "HEY".into() });
        assert_eq!(parse("/help"), Parsed::Help);
        assert_eq!(parse("/afk"), Parsed::Unsupported("afk".into()));
        assert_eq!(parse("/nonsense x"), Parsed::Unknown("nonsense".into()));
        assert_eq!(parse("   "), Parsed::Empty);
    }
}
