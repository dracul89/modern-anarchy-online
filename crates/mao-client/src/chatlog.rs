//! The chat log: every line the chat window receives is also appended to a text file, one file a day (UTC), in a `chat` folder
//! beside the settings file (`mao.ini`). `MAO_NO_CHATLOG=1` turns it off.

use std::io::Write;

use crate::chatwindow::{Kind, Line};

/// The calendar date (year, month, day) of a day count since 1970-01-01 (the civil-from-days algorithm).
pub fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// The file name (`2026-10-24.txt`) and the time of day (`09:15:02`) of a time in seconds since the epoch.
pub fn stamp(seconds: u64) -> (String, String) {
    let (y, m, d) = civil((seconds / 86_400) as i64);
    let day = seconds % 86_400;
    (format!("{y:04}-{m:02}-{d:02}.txt"), format!("{:02}:{:02}:{:02}", day / 3600, day / 60 % 60, day % 60))
}

fn label(kind: Kind) -> &'static str {
    match kind {
        Kind::System => "system",
        Kind::Tell => "tell",
        Kind::Channel => "channel",
        Kind::Own => "you",
        Kind::Combat => "combat",
    }
}

/// Appends a line to today's log (a failure to write is ignored: the log is a convenience).
pub fn append(line: &Line) {
    if std::env::var_os("MAO_NO_CHATLOG").is_some() {
        return;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (file, time) = stamp(now);
    let dir = crate::settings::Settings::path().with_file_name("chat");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(file)) {
        let _ = writeln!(f, "[{time}] {}: {}", label(line.kind), line.text.replace(['\r', '\n'], " "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_become_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(11_016), (2000, 2, 29));
        assert_eq!(civil(20_385), (2025, 10, 24));
        assert_eq!(stamp(1_792_800_000 % 86_400 + 20_385 * 86_400 + 3_723), ("2025-10-24.txt".to_string(), "01:02:03".to_string()));
    }
}
