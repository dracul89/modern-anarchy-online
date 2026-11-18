//! `/raid <word> ...`: the raid's commands.
//!
//! In the original `/raid` is one of the lines GUI.dll hands to Gamecode's text interpreter (registered by
//! FUN_100b2ac9 with the handler 0x100b285d, which calls `N3Msg_TextCommand`; no window is opened by it: the raid
//! window is the `raid_window` value's, the control center's menu entry). The interpreter (FUN_1003fe10, the branch
//! at 0x10040725) looks the word after `raid` up in its table (filled at 0x10033a26: create 1, list 2, listlocal 3,
//! move 4, lootaccess 5, locks 6; as written, the small letters only) and:
//!
//! * no word, or one not in the table: nothing is done and nothing is said;
//! * `listlocal`: the team's members are said, here in the chat (FUN_10066446; nothing is sent);
//! * `create`, `list`, `locks`: `RaidCmdIIR_c` (`mao_net::n3::raid_command`) with the word's number and no identity.
//!   Nothing is checked first, not even being in a team: the server decides (the game's help page says only a team's
//!   leader makes a raid);
//! * `move <character's number> <team 0-6>`: the two numbers as the message's first identity (`strtoul` and `atoi`;
//!   a team above 6 does nothing);
//! * `lootaccess <corpse's number> <place> <character's number>`: the corpse `(0xc76a, number)`, `(place, 0)` and the
//!   character `(50000, number)`.
//!
//! A word that is missing reads as the word before it: each is read with `>>` into one string, which a read at the
//! line's end leaves as it was. So `/raid move 5` sends `(5, 5)` and `/raid move` alone `(0, 0)` ("move" is no
//! number). Kept, since it is what the original sends.

/// What a `/raid` line does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Nothing,
    /// Say the team's members.
    ListLocal,
    /// Send `RaidCmdIIR_c`: the command's number and its three identities (the last two go out with number 5 only).
    Send { command: u32, ids: [(u32, u32); 3] },
}

/// The interpreter's table of the words after `raid` (0 for any other).
pub fn number(word: &str) -> u32 {
    ["create", "list", "listlocal", "move", "lootaccess", "locks"].iter().position(|w| *w == word).map_or(0, |p| p as u32 + 1)
}

/// The sign and the digits a C number parse takes off a word's front: whether it is negative, the digits' value
/// (held at `u64::MAX` when there are too many).
fn leading(word: &str) -> (bool, u64) {
    let (negative, digits) = match word.as_bytes().first() {
        Some(b'-') => (true, &word[1..]),
        Some(b'+') => (false, &word[1..]),
        _ => (false, word),
    };
    let value = digits.bytes().take_while(u8::is_ascii_digit).fold(0u64, |v, d| v.saturating_mul(10).saturating_add((d - b'0') as u64));
    (negative, value)
}

/// C's `atoi`: the leading digits with their sign, held at the ends of an `int`; 0 for a word that starts with none.
fn atoi(word: &str) -> i32 {
    let (negative, value) = leading(word);
    if negative { (-(value.min(1 << 31) as i64)) as i32 } else { value.min(i32::MAX as u64) as i32 }
}

/// C's `strtoul(word, 0, 10)`: the leading digits, `ULONG_MAX` for too many, and a minus sign's value turned over.
fn strtoul(word: &str) -> u32 {
    let (negative, value) = leading(word);
    let value = value.min(u32::MAX as u64) as u32;
    if negative { value.wrapping_neg() } else { value }
}

/// What the words after `/raid` do (the line's `%` arguments already put in).
pub fn step(text: &str) -> Step {
    let mut words = text.split_whitespace();
    // (one string for every word read: a read past the line's end leaves the word before)
    let mut word = String::new();
    let mut next = |word: &mut String| {
        if let Some(w) = words.next() {
            *word = w.to_string();
        }
    };
    next(&mut word);
    let command = number(&word);
    let mut ids = [(0, 0); 3];
    match command {
        0 => return Step::Nothing,
        3 => return Step::ListLocal,
        4 => {
            next(&mut word);
            let who = strtoul(&word);
            next(&mut word);
            let team = atoi(&word) as u32;
            if team > 6 {
                return Step::Nothing;
            }
            ids[0] = (who, team);
        }
        5 => {
            next(&mut word);
            let corpse = atoi(&word) as u32;
            next(&mut word);
            let place = atoi(&word) as u32;
            next(&mut word);
            ids = [(0xc76a, corpse), (place, 0), (50000, strtoul(&word))];
        }
        _ => {}
    }
    Step::Send { command, ids }
}

/// The lines of the server's lock list (`RaidIIR_c` of type 1; Gamecode FUN_100430c2 makes one text of them): a line
/// for each lock whose time is not this very second, with its playfield's name (or that it is not known), the
/// playfield's number, the lock's second number and the time left in hours, minutes and seconds: the lock's time
/// less the server's time now, taken as a number without sign, so a lock that has run out shows a very long time,
/// as in the original. The words are this client's own (the original's are in its program, not in the game's texts).
pub fn lock_lines(locks: &[((u32, u32), (u32, u32), u32)], now: u32, playfield: impl Fn(u32) -> Option<String>) -> Vec<String> {
    if locks.is_empty() {
        return vec!["You have no raid locks.".to_string()];
    }
    let mut lines = vec!["Your raid locks:".to_string()];
    for (a, b, until) in locks {
        let left = until.wrapping_sub(now);
        if left != 0 {
            let name = playfield(a.1).unwrap_or_else(|| "An unknown place".to_string());
            lines.push(format!("{name} ({}), {}: {} h {} min {} s", a.1, b.1, left / 3600, left % 3600 / 60, left % 3600 % 60));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_raid_words_are_the_interpreters() {
        assert_eq!((number("create"), number("list"), number("listlocal"), number("move"), number("lootaccess"), number("locks")), (1, 2, 3, 4, 5, 6));
        assert_eq!((number("Create"), number(""), number("help")), (0, 0, 0));
        // (no word, or another: nothing)
        assert_eq!((step(""), step("help"), step("LOCKS")), (Step::Nothing, Step::Nothing, Step::Nothing));
        assert_eq!(step("listlocal"), Step::ListLocal);
        for (word, command) in [("create", 1), ("list", 2), ("locks", 6)] {
            assert_eq!(step(word), Step::Send { command, ids: [(0, 0); 3] });
            assert_eq!(step(&format!("  {word} with more words")), Step::Send { command, ids: [(0, 0); 3] });
        }
    }

    #[test]
    fn move_and_lootaccess_read_their_numbers_as_c_does() {
        let moved = |text: &str| step(text);
        assert_eq!(moved("move 305419896 2"), Step::Send { command: 4, ids: [(305419896, 2), (0, 0), (0, 0)] });
        // (a team above 6, or below 0: nothing)
        assert_eq!((moved("move 7 7"), moved("move 7 -1")), (Step::Nothing, Step::Nothing));
        assert_eq!(moved("move 7 6"), Step::Send { command: 4, ids: [(7, 6), (0, 0), (0, 0)] });
        // (a missing word is the word before it: "move" is 0, and one number is both)
        assert_eq!(moved("move"), Step::Send { command: 4, ids: [(0, 0), (0, 0), (0, 0)] });
        assert_eq!(moved("move 5"), Step::Send { command: 4, ids: [(5, 5), (0, 0), (0, 0)] });
        assert_eq!(moved("move 123"), Step::Nothing);
        // (leading digits only; a name is 0)
        assert_eq!(moved("move 12ab 3x"), Step::Send { command: 4, ids: [(12, 3), (0, 0), (0, 0)] });
        assert_eq!(moved("move Somebody 1"), Step::Send { command: 4, ids: [(0, 1), (0, 0), (0, 0)] });
        assert_eq!(step("lootaccess 77 3 4660"), Step::Send { command: 5, ids: [(0xc76a, 77), (3, 0), (50000, 4660)] });
        assert_eq!(step("lootaccess"), Step::Send { command: 5, ids: [(0xc76a, 0), (0, 0), (50000, 0)] });
        assert_eq!(step("lootaccess 9"), Step::Send { command: 5, ids: [(0xc76a, 9), (9, 0), (50000, 9)] });
        assert_eq!((atoi("-12"), atoi("+7x"), atoi("99999999999"), atoi("-99999999999"), atoi("x1")), (-12, 7, i32::MAX, i32::MIN, 0));
        assert_eq!((strtoul("4294967295"), strtoul("99999999999"), strtoul("-1"), strtoul("")), (u32::MAX, u32::MAX, u32::MAX, 0));
    }

    #[test]
    fn the_lock_list_is_told_lock_by_lock() {
        let name = |id: u32| (id == 4365).then(|| "Somewhere".to_string());
        assert_eq!(lock_lines(&[], 1000, name), ["You have no raid locks."]);
        let locks = [((0xc79d, 4365), (0, 12), 1000 + 3 * 3600 + 4 * 60 + 5), ((0xc79d, 9), (0, 1), 1000), ((0xc79d, 9), (0, 2), 1061)];
        assert_eq!(lock_lines(&locks, 1000, name), ["Your raid locks:", "Somewhere (4365), 12: 3 h 4 min 5 s", "An unknown place (9), 2: 0 h 1 min 1 s"]);
    }
}
