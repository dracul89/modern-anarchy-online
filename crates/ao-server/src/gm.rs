//! The game-master commands: a line of chat that starts with `/gm` is for the server, not for the others (the captures show no
//! such command in the original; this is the test server's own). `/gm item <template> [count] [quality]` puts an item in the
//! backpack, `/gm xp <amount>` gives experience, `/gm credits <amount>` gives credits.

/// A command, read from a line of chat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Item { template: u32, count: u16, quality: u32 },
    Xp(u64),
    Credits(u32),
    /// Completes the mission at this place in the log (from 1): it is removed and its reward given.
    Complete(usize),
    /// A `/gm` line that is not a command (with what was typed, for the usage line).
    Unknown(String),
}

/// The number a word stands for: decimal, or hexadecimal with `0x`.
fn number(word: &str) -> Option<u64> {
    match word.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16).ok(),
        None => word.parse().ok(),
    }
}

/// The command a line of chat is, when it is one.
pub fn parse(line: &str) -> Option<Command> {
    let rest = line.trim().strip_prefix("/gm")?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let words: Vec<&str> = rest.split_whitespace().collect();
    let command = match words.as_slice() {
        ["item", template, rest @ ..] => {
            let template = number(template).and_then(|t| u32::try_from(t).ok());
            let count = rest.first().and_then(|c| number(c)).and_then(|c| u16::try_from(c).ok()).unwrap_or(1);
            let quality = rest.get(1).and_then(|q| number(q)).and_then(|q| u32::try_from(q).ok()).unwrap_or(1);
            match template {
                Some(template) if count > 0 => Command::Item { template, count, quality },
                _ => Command::Unknown(line.trim().to_string()),
            }
        }
        ["xp", amount] => number(amount).map_or_else(|| Command::Unknown(line.trim().to_string()), Command::Xp),
        ["complete", n] => number(n).and_then(|n| usize::try_from(n).ok()).filter(|n| *n > 0).map_or_else(|| Command::Unknown(line.trim().to_string()), Command::Complete),
        ["credits", amount] => number(amount).and_then(|a| u32::try_from(a).ok()).map_or_else(|| Command::Unknown(line.trim().to_string()), Command::Credits),
        _ => Command::Unknown(line.trim().to_string()),
    };
    Some(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_commands_are_read() {
        assert_eq!(parse("/gm item 156020 3 5"), Some(Command::Item { template: 156020, count: 3, quality: 5 }));
        assert_eq!(parse("/gm item 0x2611c"), Some(Command::Item { template: 0x2611c, count: 1, quality: 1 }));
        assert_eq!(parse("/gm xp 5000"), Some(Command::Xp(5000)));
        assert_eq!(parse("/gm credits 250"), Some(Command::Credits(250)));
        assert_eq!(parse("/gm complete 2"), Some(Command::Complete(2)));
        assert!(matches!(parse("/gm complete 0"), Some(Command::Unknown(_))));
    }

    #[test]
    fn other_lines_are_not_commands() {
        assert_eq!(parse("hello there"), None);
        assert_eq!(parse("/gmail hi"), None);
        assert!(matches!(parse("/gm xp lots"), Some(Command::Unknown(_))));
        assert!(matches!(parse("/gm"), Some(Command::Unknown(_))));
    }
}
