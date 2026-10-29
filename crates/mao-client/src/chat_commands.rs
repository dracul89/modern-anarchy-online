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

/// The emotes (GUI.dll's table at 0x101bae30, FUN_100b2fc3 looks a name up in it, any case): each name's place, from 1, is
/// its `AbstractAnimID`; FUN_100b348c makes `/<name>` a command for each, and `/emote <name>` takes them too.
pub const EMOTES: [&str; 70] = [
    "prostrate", "angry", "apachi", "applause", "itch", "backflip", "ballet", "blowkiss", "bow", "bulge", "chicken", "cross", "crossarm",
    "adjust", "curt", "disco", "drink", "eat", "fblock", "fishsize", "flamenco", "flip", "giggle", "gloat", "greet", "italian", "kneel",
    "laugh-b", "laugh-s", "legshake", "lookout", "moon", "nod", "nono", "pointba", "pointfor", "pointlef", "pointrig", "pointup", "pray",
    "puke", "pulp", "read", "rocky", "salute", "scared", "scratch", "shake", "shrug", "slap", "speech", "spit", "strong1", "strong2",
    "strong3", "strong4", "surprised", "surrender", "swroyal", "thinker", "thumbs", "wave", "ymca", "kiss", "kisslow", "kisshigh", "hug",
    "sleep", "lounge", "facepalm",
];

/// FUN_100b2fc3: an emote's `AbstractAnimID` by name (0 for none).
pub fn emote_id(name: &str) -> u32 {
    EMOTES.iter().position(|e| e.eq_ignore_ascii_case(name)).map_or(0, |i| i as u32 + 1)
}

/// A one-line description of the commands this client carries out (for `/help`).
pub const IMPLEMENTED: &[(&str, &str)] = &[
    ("tell <name> <text>", "a private message (also /t)"),
    ("reply <text>", "answer the last tell (also /r)"),
    ("org <text>", "your organization's channel (also /o)"),
    ("say <text>", "speak to the people around (plain text too); /shout or /s shouts, /whisper or /w whispers, /me emotes"),
    ("emote <name>", "do an emote (also /<name>: /wave, /bow, /dance-like ones such as /disco; sleep and lounge sitting)"),
    ("voice <sound name>", "speak a voice line (heal, help, inc, no, run, yes) to the people around"),
    ("befriend <name>", "put a character on your friends list (L shows it)"),
    ("ignore [name]", "hide a character's tells and channel lines (again: show them; alone: the list)"),
    ("camp", "sit down to log out (stand up or move to cancel)"),
    ("sit", "sit down, or stand up (the X key)"),
    ("follow", "walk after the target (again, or any movement key: stop)"),
    ("inspect", "level, profession and health of the target"),
    ("open <window>", "show a window (inventory, stats, skills, ncu, friends, programs, perks, help, map, ...); /items is /open inventory"),
    ("close [window]", "hide a window, or all of them"),
    ("waypoint <x> <z>", "walk to a place (any movement key stops)"),
    ("selectself", "target your own character"),
    ("setoption [name value]", "a setting of the client; alone, the list"),
    ("chardist [n]", "how far characters are drawn (0 for all)"),
    ("viewdist [n]", "how far the world is seen (200 to 4000)"),
    ("quit", "leave the client"),
    ("help", "this list"),
    ("version", "the client's version"),
];

/// What a typed line asks for.
#[derive(Clone, Debug, PartialEq)]
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
    /// `/viewdist [n]`: how far the world is seen (alone: the setting).
    ViewDist { distance: Option<u32> },
    /// `/chardist [n]`: how far characters are drawn (0: all).
    CharDist { distance: Option<u32> },
    /// `/follow`: walk after the current target (again: stop).
    Follow,
    /// `/inspect`: say what is known of the current target (level, profession, health).
    Inspect,
    /// `/open <window>` and `/close [window]`: show or hide a window (`close` alone hides them all).
    Window { name: String, open: bool },
    /// `/waypoint <x> <z>`: walk to a place.
    Waypoint { x: f32, z: f32 },
    /// `/selectself`: target the character itself.
    SelectSelf,
    /// `/setoption [name value]`: a setting of the client (alone: the list).
    SetOption { name: String, value: String },
    /// `/camp`: sit down to log out.
    Camp,
    /// `/sit`: sit down or stand up, as the X key.
    Sit,
    /// `/emote <name>` or `/<name>`: an emote (a social action, its `AbstractAnimID`; GUI.dll FUN_100b30bf).
    Social { id: u32 },
    /// `/emote` with a name that is none (FUN_100b30bf: "Error: No emote named '<name>'").
    NoEmote(String),
    /// `/voice <sound name>`: a voice line spoken (GUI.dll FUN_100b88b1; the name its first word).
    Voice { line: String },
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
/// Commands of this client's own, beyond the original's list: `/sit` (the X key's sit and stand).
pub const EXTRA: &[&str] = &["sit"];

fn known(word: &str) -> Option<&'static str> {
    COMMANDS.iter().chain(EXTRA).copied().find(|c| c.eq_ignore_ascii_case(word))
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
    // (`/<emote>`: FUN_100b30bf takes the command's name without its slash)
    if emote_id(word) != 0 {
        return Parsed::Social { id: emote_id(word) };
    }
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
        "sit" => Parsed::Sit,
        "voice" => match text.split_whitespace().next() {
            Some(line) => Parsed::Voice { line: line.to_string() },
            None => Parsed::Usage("/voice <sound name>"),
        },
        "follow" => Parsed::Follow,
        "inspect" => Parsed::Inspect,
        "open" | "items" => Parsed::Window { name: if command.eq_ignore_ascii_case("items") { "inventory".to_string() } else { text.to_string() }, open: true },
        "close" => Parsed::Window { name: text.to_string(), open: false },
        "selectself" => Parsed::SelectSelf,
        "setoption" | "option" => {
            let mut words = text.split_whitespace();
            Parsed::SetOption { name: words.next().unwrap_or("").to_string(), value: words.next().unwrap_or("").to_string() }
        }
        "waypoint" => {
            let n: Vec<f32> = text.split(|c: char| c.is_whitespace() || c == ',').filter_map(|t| t.parse().ok()).collect();
            match n[..] {
                [x, z] => Parsed::Waypoint { x, z },
                _ => Parsed::Usage("/waypoint <x> <z>"),
            }
        }
        "chardist" => Parsed::CharDist { distance: text.split_whitespace().next().and_then(|t| t.parse().ok()) },
        "viewdist" => Parsed::ViewDist { distance: text.split_whitespace().next().and_then(|t| t.parse().ok()) },
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
        "emote" => match text.split_whitespace().next() {
            None => Parsed::Usage("/emote <emote-name>"),
            Some(name) if emote_id(name) == 0 => Parsed::NoEmote(name.to_string()),
            Some(name) => Parsed::Social { id: emote_id(name) },
        },
        // (FUN_1009d247: `/whisper` and `/w` kind 1, `/shout` and `/s` 2, `/me` 3, the rest 0)
        "say" | "shout" | "whisper" | "s" | "w" | "me" => {
            let name = match command.to_lowercase().as_str() {
                "s" | "shout" => "shout",
                "w" | "whisper" => "whisper",
                "me" => "emote",
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
        assert_eq!(parse("/sit"), Parsed::Sit);
        assert_eq!(parse("/follow"), Parsed::Follow);
        assert_eq!(parse("/inspect"), Parsed::Inspect);
        assert_eq!(parse("/open skills"), Parsed::Window { name: "skills".into(), open: true });
        assert_eq!(parse("/items"), Parsed::Window { name: "inventory".into(), open: true });
        assert_eq!(parse("/close"), Parsed::Window { name: String::new(), open: false });
        assert_eq!(parse("/waypoint 450.5, 310"), Parsed::Waypoint { x: 450.5, z: 310.0 });
        assert_eq!(parse("/waypoint 450"), Parsed::Usage("/waypoint <x> <z>"));
        assert_eq!(parse("/selectself"), Parsed::SelectSelf);
        assert_eq!(parse("/setoption vsync on"), Parsed::SetOption { name: "vsync".into(), value: "on".into() });
        assert_eq!(parse("/viewdist 600"), Parsed::ViewDist { distance: Some(600) });
        assert_eq!(parse("/viewdist"), Parsed::ViewDist { distance: None });
        assert_eq!(parse("/QUIT"), Parsed::Quit);
        assert_eq!(parse("/befriend Abc"), Parsed::Befriend { name: "Abc".into() });
        assert_eq!(parse("/befriend"), Parsed::Usage("/befriend <name>"));
        assert_eq!(parse("/o hello org"), Parsed::Org { text: "hello org".into() });
        assert_eq!(parse("/voice heal"), Parsed::Voice { line: "heal".into() });
        assert_eq!(parse("/voice"), Parsed::Usage("/voice <sound name>"));
        assert_eq!(parse("hello world"), Parsed::Vicinity { command: "say", text: "hello world".into() });
        assert_eq!(parse("/s HEY"), Parsed::Vicinity { command: "shout", text: "HEY".into() });
        assert_eq!(parse("/help"), Parsed::Help);
        assert_eq!(parse("/afk"), Parsed::Unsupported("afk".into()));
        assert_eq!(parse("/nonsense x"), Parsed::Unknown("nonsense".into()));
        assert_eq!(parse("   "), Parsed::Empty);
    }
}
