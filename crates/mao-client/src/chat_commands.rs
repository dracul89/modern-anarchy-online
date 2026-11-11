//! The chat commands of the original game (the list its input line offers when a `/` is typed) and what this client does with
//! them: a typed line becomes a [`Parsed`].

/// Every command the original lists, in its order (the case is the original's).
/// (Under review: the AOU and other script-file names in this list, `aliens` to `MapMarker`, came from one install's
/// `scripts` folder, not the game; see docs/ORIGINAL_AUDIT.md, "For review and removal".)
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
    ("tell <name> <text>", "a private message; without the text, a window for talking with that character; alone, one with your target"),
    ("%t %f %m", "in a line you send: the name of your target, of the one you fight, your own (%% for a percent sign)"),
    ("t <text>", "your team's channel"),
    ("duel", "ask the targeted player to duel; /duel accept, reject, stop or draw answer and end one"),
    ("petduel", "ask the targeted player for a duel of pets; /petduel accept, reject or stop answer and end one"),
    ("ch <group>", "the group this chat window's plain lines go to (the start of its name is enough)"),
    ("group <group> <text>", "one line to a group (also /g)"),
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
    /// A private message to a character; with no text, the window for talking with the character (FUN_100ba8d5 sends
    /// the empty text on, and FUN_10089940 opens the tell window for it).
    Tell { name: String, text: String },
    /// `/tell` alone: the window for talking with the target. This client's own: the original does nothing without a
    /// name.
    TellTarget,
    /// An answer to the last tell.
    Reply { text: String },
    /// `/ch <group name>`: the window's output group. `word` is the command as typed.
    Output { word: String, name: String },
    /// `/group <group name> <text>` (`/g`): a line to a group.
    Group { word: String, name: String, text: String },
    /// `/t <text>`: a line to the team's channel.
    Team { text: String },
    /// `/duel`: challenge the target (`None`), or `accept` (1), `reject` (2), `stop` (3), `draw` (4).
    Duel { what: Option<u32> },
    /// `/petduel`: challenge the target to a duel of pets (`None`), or `accept` (1), `reject` (2), `stop` (3).
    PetDuel { what: Option<u32> },
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
    /// `/fxscript <script> [parameters] #`: an effect script of the game's `cd_image/twk` folder run (GUI.dll
    /// FUN_100b70be): the line as the script engine's signal takes it, `/fxscript <script> <parameters>`.
    FxScript { line: String },
    /// `/selectself`: target the character itself.
    SelectSelf,
    /// `/setoption [name value]`: a setting of the client (alone: the list).
    SetOption { name: String, value: String },
    /// `/camp`: sit down to log out.
    Camp,
    /// `/sit`: sit down or stand up, as the X key.
    Sit,
    /// `/perf`: the performance overlay shown or hidden.
    Perf,
    /// `/pet <command>`: a command to the pets (its `PetCommand` number).
    Pet { command: u32 },
    /// `/stuck`, `/anon`, `/list` and `/shop`: commands the server carries out (GUI.dll's `CCFanatic_c` list, FUN_100b4001, each
    /// sent as its text by FUN_100b36da: the line without its slash).
    Server { text: String },
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
/// `/pet` is Gamecode's own (FUN_100542ad), not in the input line's list, but the control center's Pets menu runs it.
pub const EXTRA: &[&str] = &["sit", "pet", "perf"];

/// `/pet <command>`'s words and the `PetCommand` numbers they send (Gamecode FUN_100542ad's table).
pub const PET_COMMANDS: &[(&str, u32)] = &[
    ("follow", 1), ("behind", 2), ("survive", 3), ("wait", 4), ("cycle", 5), ("guard", 6), ("attack", 7), ("social", 9), ("terminate", 10),
    ("free", 11), ("heal", 12), ("report", 14), ("rename", 15), ("chat", 16), ("script", 17),
];

/// The names a sent line's `%` arguments stand for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Names {
    /// The player's own character.
    pub own: String,
    /// The selected target and the one fought.
    pub target: Option<String>,
    pub fighting: Option<String>,
}

/// `ChatGUIModule_c::ExpandChatTextArgs` 0x100866c6, which every command that sends text runs it through: `%t` is the
/// target's name, `%f` the name of the one fought, `%m` the player's own, `%%` a percent sign; any other `%x` and a
/// `%` at the end stay as they are. Without a target the original puts its own words there, marked up as text
/// (`&lt;no target&gt;`).
pub fn expand(text: &str, names: &Names) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some('f') => out.push_str(names.fighting.as_deref().unwrap_or("&lt;no fighting target&gt;")),
            Some('m') => out.push_str(&names.own),
            Some('t') => out.push_str(names.target.as_deref().unwrap_or("&lt;no target&gt;")),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

fn known(word: &str) -> Option<&'static str> {
    COMMANDS.iter().chain(EXTRA).copied().find(|c| c.eq_ignore_ascii_case(word))
}

/// The first argument of a group command and what follows it: a word, or several in quotes (kept, as the original's
/// arguments keep them).
fn group_name(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    let end = match text.strip_prefix('"') {
        Some(rest) => rest.find('"').map_or(text.len(), |n| n + 2),
        None => text.find(char::is_whitespace).unwrap_or(text.len()),
    };
    (end > 0).then(|| (&text[..end], text[end..].trim()))
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
        "tell" => {
            let (name, message) = match text.split_once(char::is_whitespace) {
                Some((n, m)) => (n, m.trim()),
                None => (text, ""),
            };
            if name.is_empty() { Parsed::TellTarget } else { Parsed::Tell { name: name.to_string(), text: message.to_string() } }
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
        "stuck" | "anon" | "list" | "shop" => Parsed::Server { text: rest.to_string() },
        "sit" => Parsed::Sit,
        "perf" => Parsed::Perf,
        "pet" => match text.split_whitespace().next().and_then(|w| PET_COMMANDS.iter().find(|c| c.0.eq_ignore_ascii_case(w))) {
            Some(&(_, command)) => Parsed::Pet { command },
            None => Parsed::Usage("/pet follow|behind|wait|guard|attack|heal|report|terminate|..."),
        },
        "voice" => match text.split_whitespace().next() {
            Some(line) => Parsed::Voice { line: line.to_string() },
            None => Parsed::Usage("/voice <sound name>"),
        },
        "follow" => Parsed::Follow,
        "inspect" => Parsed::Inspect,
        // (`/raid` and `/lft` open their windows, as the original's commands do)
        "raid" | "lft" => Parsed::Window { name: command.to_lowercase(), open: true },
        "open" | "items" => Parsed::Window { name: if command.eq_ignore_ascii_case("items") { "inventory".to_string() } else { text.to_string() }, open: true },
        "close" => Parsed::Window { name: text.to_string(), open: false },
        "selectself" => Parsed::SelectSelf,
        "setoption" | "option" => {
            let mut words = text.split_whitespace();
            Parsed::SetOption { name: words.next().unwrap_or("").to_string(), value: words.next().unwrap_or("").to_string() }
        }
        // (the original wants the script's name at least and hands the whole line on; a script is only run when a
        // parameters part follows its name, which the scripts' own calls fill with a lone `#`, a comment: that is put
        // there when the name stands alone, this client's own help)
        "fxscript" => match text.split_whitespace().count() {
            0 => Parsed::Usage("/fxscript <script> [parameters] #  (a script of the game's cd_image/twk folder, without .txt)"),
            1 => Parsed::FxScript { line: format!("/fxscript {} #", text.trim()) },
            _ => Parsed::FxScript { line: format!("/fxscript {}", text.trim()) },
        },
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
        // (GUI.dll FUN_100b8d78: no word challenges the target; else one of three words)
        "petduel" => match text.to_lowercase().as_str() {
            "" => Parsed::PetDuel { what: None },
            "accept" => Parsed::PetDuel { what: Some(1) },
            "reject" => Parsed::PetDuel { what: Some(2) },
            "stop" => Parsed::PetDuel { what: Some(3) },
            _ => Parsed::Usage("/petduel (asks the target player for a duel of pets), /petduel accept, /petduel reject, /petduel stop"),
        },
        // (GUI.dll's `/duel` handler: no word challenges the target; else one of four words)
        "duel" => match text.to_lowercase().as_str() {
            "" => Parsed::Duel { what: None },
            "accept" => Parsed::Duel { what: Some(1) },
            "reject" => Parsed::Duel { what: Some(2) },
            "stop" => Parsed::Duel { what: Some(3) },
            "draw" => Parsed::Duel { what: Some(4) },
            _ => Parsed::Usage("/duel (asks the target player), /duel accept, /duel reject, /duel stop, /duel draw"),
        },
        // (FUN_1009d028: `/t` is the team's channel; `/tell` the private message)
        "t" => {
            if text.is_empty() {
                Parsed::Usage("/t <text>")
            } else {
                Parsed::Team { text: text.to_string() }
            }
        }
        // (FUN_1009c5b8: the group's name and nothing more; a name with spaces goes in quotes)
        "ch" => match group_name(text) {
            Some((name, "")) => Parsed::Output { word: format!("/{word}"), name: name.to_string() },
            _ => Parsed::Usage("/ch <group name>"),
        },
        // (FUN_1009c977: the group's name, then the line)
        "group" | "g" => match group_name(text) {
            Some((name, line)) if !line.is_empty() => Parsed::Group { word: format!("/{word}"), name: name.to_string(), text: line.to_string() },
            _ => Parsed::Usage("/group <group name> <text>"),
        },
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
    fn the_percent_arguments_become_names() {
        let names = Names { own: "Me".into(), target: Some("Abc".into()), fighting: None };
        assert_eq!(expand("hi %t, I am %m", &names), "hi Abc, I am Me");
        assert_eq!(expand("%f is at 50%% and %x stays, as 100%", &names), "&lt;no fighting target&gt; is at 50% and %x stays, as 100%");
        assert_eq!(expand("%%t %t", &Names::default()), "%t &lt;no target&gt;");
    }

    #[test]
    fn lines_become_commands() {
        assert_eq!(parse("/tell Abc hello there"), Parsed::Tell { name: "Abc".into(), text: "hello there".into() });
        assert_eq!(parse("/TELL Abc hi"), Parsed::Tell { name: "Abc".into(), text: "hi".into() });
        assert_eq!(parse("/tell Abc"), Parsed::Tell { name: "Abc".into(), text: String::new() });
        assert_eq!(parse("/tell"), Parsed::TellTarget);
        assert_eq!(parse("/r thanks"), Parsed::Reply { text: "thanks".into() });
        assert_eq!(parse("/ignore Abc"), Parsed::Ignore { name: "Abc".into() });
        assert_eq!(parse("/ignore"), Parsed::Ignore { name: String::new() });
        assert_eq!(parse("/camp"), Parsed::Camp);
        assert_eq!(parse("/pet attack"), Parsed::Pet { command: 7 });
        assert_eq!(parse("/stuck"), Parsed::Server { text: "stuck".into() });
        assert_eq!(parse("/list doctor"), Parsed::Server { text: "list doctor".into() });
        assert_eq!(parse("/pet Terminate"), Parsed::Pet { command: 10 });
        assert_eq!(parse("/sit"), Parsed::Sit);
        assert_eq!(parse("/follow"), Parsed::Follow);
        assert_eq!(parse("/inspect"), Parsed::Inspect);
        assert_eq!(parse("/open skills"), Parsed::Window { name: "skills".into(), open: true });
        assert_eq!(parse("/raid"), Parsed::Window { name: "raid".into(), open: true });
        assert_eq!(parse("/lft"), Parsed::Window { name: "lft".into(), open: true });
        assert_eq!(parse("/items"), Parsed::Window { name: "inventory".into(), open: true });
        assert_eq!(parse("/close"), Parsed::Window { name: String::new(), open: false });
        assert_eq!(parse("/waypoint 450.5, 310"), Parsed::Waypoint { x: 450.5, z: 310.0 });
        assert_eq!(parse("/waypoint 450"), Parsed::Usage("/waypoint <x> <z>"));
        // (`/fxscript`: the line as the engine's signal takes it; a lone name is given the empty parameters part)
        assert_eq!(parse("/fxscript EP03_Orbital_Attack"), Parsed::FxScript { line: "/fxscript EP03_Orbital_Attack #".into() });
        assert_eq!(parse("/fxscript Alien_Event_Flyby_01 Float LifeTime 30.0 #"), Parsed::FxScript { line: "/fxscript Alien_Event_Flyby_01 Float LifeTime 30.0 #".into() });
        assert!(matches!(parse("/fxscript"), Parsed::Usage(_)));
        assert_eq!(parse("/selectself"), Parsed::SelectSelf);
        assert_eq!(parse("/setoption vsync on"), Parsed::SetOption { name: "vsync".into(), value: "on".into() });
        assert_eq!(parse("/viewdist 600"), Parsed::ViewDist { distance: Some(600) });
        assert_eq!(parse("/viewdist"), Parsed::ViewDist { distance: None });
        assert_eq!(parse("/QUIT"), Parsed::Quit);
        assert_eq!(parse("/befriend Abc"), Parsed::Befriend { name: "Abc".into() });
        assert_eq!(parse("/befriend"), Parsed::Usage("/befriend <name>"));
        assert_eq!(parse("/o hello org"), Parsed::Org { text: "hello org".into() });
        assert_eq!((parse("/duel"), parse("/duel Accept"), parse("/duel draw")), (Parsed::Duel { what: None }, Parsed::Duel { what: Some(1) }, Parsed::Duel { what: Some(4) }));
        assert!(matches!(parse("/duel now"), Parsed::Usage(_)));
        assert_eq!((parse("/petduel"), parse("/petduel REJECT"), parse("/petduel stop")), (Parsed::PetDuel { what: None }, Parsed::PetDuel { what: Some(2) }, Parsed::PetDuel { what: Some(3) }));
        assert!(matches!(parse("/petduel draw"), Parsed::Usage(_)));
        // (the original's `/t` is the team's channel, not a tell)
        assert_eq!(parse("/t on my way"), Parsed::Team { text: "on my way".into() });
        assert_eq!(parse("/t"), Parsed::Usage("/t <text>"));
        assert_eq!(parse("/ch clan"), Parsed::Output { word: "/ch".into(), name: "clan".into() });
        assert_eq!(parse("/CH \"Clan OOC\""), Parsed::Output { word: "/CH".into(), name: "\"Clan OOC\"".into() });
        assert_eq!(parse("/ch"), Parsed::Usage("/ch <group name>"));
        assert_eq!(parse("/ch clan ooc"), Parsed::Usage("/ch <group name>"));
        assert_eq!(parse("/g \"Clan OOC\" hello there"), Parsed::Group { word: "/g".into(), name: "\"Clan OOC\"".into(), text: "hello there".into() });
        assert_eq!(parse("/group trade selling a pistol"), Parsed::Group { word: "/group".into(), name: "trade".into(), text: "selling a pistol".into() });
        assert_eq!(parse("/group trade"), Parsed::Usage("/group <group name> <text>"));
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
