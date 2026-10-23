//! The mail protocol (`Mail`, `333b2867`), as the user's captures show it. Every message is a header
//! byte, an `i16` action and the rest; all big-endian.
//!
//! Client to server:
//! * action 1, then `i32 0, i32 id`: with id 0 the inbox list is wanted, otherwise that mail is opened (read);
//! * action 3, `i32 0, i32 id`: take the mail's attachments;
//! * action 5, `i32 0, i32 id`: delete the mail;
//! * action 6: send ([`crate::n3::mail_send`]).
//!
//! Server to client:
//! * action 0: the inbox, an array counted `(n + 1) * 0x3f1` of [`Entry`];
//! * action 2: one mail in full ([`Mail`]);
//! * action 4, `i32 0, i32 id, i32 flags`: the mail's flags changed;
//! * action 8: a mail was accepted for sending (`i16 6, i32 0, u8 1, i32 id, i32 0`).
//!
//! The flags seen: `0x28` for an unread mail with an item and credits, `0x29` once opened, `0x2b` once the
//! item was taken (so, as a reading of three values: bit 0 read, bit 1 taken, bit 3 an item, bit 5 credits).

use crate::frame::Frame;
use crate::n3::{Message, CHARACTER};

pub const READ: u32 = 0x01;
pub const TAKEN: u32 = 0x02;
pub const HAS_ITEM: u32 = 0x08;
pub const HAS_CREDITS: u32 = 0x20;

/// One line of the inbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: u32,
    pub sender_id: u32,
    pub sender: String,
    pub subject: String,
    /// Unix seconds.
    pub sent: u32,
    pub expires: u32,
    pub flags: u32,
    pub express: bool,
}

/// A mail opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mail {
    pub id: u32,
    pub sender_id: u32,
    pub sender: String,
    pub subject: String,
    pub sent: u32,
    pub expires: u32,
    pub flags: u32,
    pub credits: u32,
    /// The attached item: (low id, high id, quality).
    pub item: Option<(u32, u32, u32)>,
    pub body: String,
}

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.data.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_be_bytes(self.take(2)?.try_into().ok()?))
    }

    fn text(&mut self) -> Option<String> {
        let n = self.u16()? as usize;
        Some(String::from_utf8_lossy(self.take(n)?).trim_end_matches('\0').to_string())
    }
}

fn header(data: &[u8]) -> Option<(i16, Cursor<'_>)> {
    let action = i16::from_be_bytes(data.get(1..3)?.try_into().ok()?);
    Some((action, Cursor { data, at: 3 }))
}

/// An inbox entry as it appears in a list and (up to the flags) in an opened mail.
fn entry_fields(c: &mut Cursor) -> Option<(u32, u32, String, String, u32, u32, u32)> {
    c.u32()?;
    let id = c.u32()?;
    let sender_id = c.u32()?;
    let sender = c.text()?;
    let subject = c.text()?;
    let (sent, expires, flags) = (c.u32()?, c.u32()?, c.u32()?);
    Some((id, sender_id, sender, subject, sent, expires, flags))
}

/// The inbox in a server `Mail` of action 0.
pub fn parse_list(data: &[u8]) -> Option<Vec<Entry>> {
    let (action, mut c) = header(data)?;
    if action != 0 {
        return None;
    }
    let count = usize::try_from(i32::from_be_bytes(c.take(4)?.try_into().ok()?) / 0x3f1 - 1).ok()?;
    let mut entries = Vec::new();
    for _ in 0..count.min(64) {
        let (id, sender_id, sender, subject, sent, expires, flags) = entry_fields(&mut c)?;
        let express = c.take(1)?[0] != 0;
        entries.push(Entry { id, sender_id, sender, subject, sent, expires, flags, express });
    }
    Some(entries)
}

/// An opened mail in a server `Mail` of action 2.
pub fn parse_mail(data: &[u8]) -> Option<Mail> {
    let (action, mut c) = header(data)?;
    if action != 2 {
        return None;
    }
    let (id, sender_id, sender, subject, sent, expires, flags) = entry_fields(&mut c)?;
    // A byte after the flags (the list has the express flag there), then the credits.
    c.take(1)?;
    let credits = c.u32()?;
    let (low, high, quality) = (c.u32()?, c.u32()?, c.u32()?);
    c.u32()?;
    c.u32()?;
    let body = c.text().unwrap_or_default();
    Some(Mail { id, sender_id, sender, subject, sent, expires, flags, credits, item: (flags & HAS_ITEM != 0 && low != 0).then_some((low, high, quality)), body })
}

/// The new flags in a server `Mail` of action 4: (mail id, flags).
pub fn parse_flags(data: &[u8]) -> Option<(u32, u32)> {
    let (action, mut c) = header(data)?;
    if action != 4 {
        return None;
    }
    c.u32()?;
    Some((c.u32()?, c.u32()?))
}

fn request(character: u32, action: i16, id: u32) -> Frame {
    let mut data = vec![0u8];
    data.extend_from_slice(&action.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&id.to_be_bytes());
    Message { kind: 0x333b2867, identity: (CHARACTER, character), data }.frame(character)
}

/// Opens (reads) a mail.
pub fn open(character: u32, id: u32) -> Frame {
    request(character, 1, id)
}

/// Takes a mail's attachments.
pub fn take(character: u32, id: u32) -> Frame {
    request(character, 3, id)
}

/// Deletes a mail.
pub fn delete(character: u32, id: u32) -> Frame {
    request(character, 5, id)
}

/// A unix time as `YYYY-MM-DD HH:MM` (UTC).
pub fn time_text(unix: u32) -> String {
    let secs = unix as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let (y, doy) = (yoe + era * 400, doe - (365 * yoe + yoe / 4 - yoe / 100));
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y + (m <= 2) as i64, m, d, rest / 3600, rest % 3600 / 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    // From the capture: the recipient's inbox (one mail from "Conjecture"), the opened mail, the flag changes.
    const LIST: &str = "000000000007e200000000015600060004bc80000a436f6e6a656374757265000b68656c6c6f2074686572656adadeb66aed53b60000002801";
    const OPENED: &str = "00000200000000015600060004bc80000a436f6e6a656374757265000b68656c6c6f2074686572656adadeb66aed53b6000000280000009c4000035501000355010000006400000000000000010000";

    #[test]
    fn the_inbox_list() {
        let entries = parse_list(&hex(LIST)).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!((e.id, e.sender_id, e.sender.as_str(), e.subject.as_str()), (0x0156_0006, 0x4bc80, "Conjecture", "hello there"));
        assert_eq!((e.sent, e.expires, e.flags, e.express), (0x6adadeb6, 0x6aed53b6, 0x28, true));
        assert_eq!(parse_list(&hex("000000000003f1")).unwrap(), vec![]);
    }

    #[test]
    fn an_opened_mail_has_its_credits_and_item() {
        let m = parse_mail(&hex(OPENED)).unwrap();
        assert_eq!((m.credits, m.item, m.subject.as_str(), m.body.as_str()), (40_000, Some((0x35501, 0x35501, 100)), "hello there", ""));
    }

    #[test]
    fn flags_change_as_the_mail_is_read_and_taken() {
        assert_eq!(parse_flags(&hex("000004000000000156000600000029")), Some((0x0156_0006, 0x29)));
        assert_eq!(parse_flags(&hex("00000400000000015600060000002b")), Some((0x0156_0006, 0x2b)));
        assert_eq!(0x2b & TAKEN, TAKEN);
    }

    #[test]
    fn the_requests_match_the_capture() {
        let bytes = |f: Frame| -> String { Message::parse(&f).unwrap().data.iter().map(|b| format!("{b:02x}")).collect() };
        assert_eq!(bytes(open(0x1323e, 0x0156_0006)), "0000010000000001560006");
        assert_eq!(bytes(take(0x1323e, 0x0156_0006)), "0000030000000001560006");
        assert_eq!(bytes(delete(0x1323e, 0x0156_0006)), "0000050000000001560006");
    }

    #[test]
    fn times_read_as_dates() {
        assert_eq!(time_text(0), "1970-01-01 00:00");
        assert_eq!(time_text(0x6adadeb6), "2026-10-23 04:12");
    }
}
