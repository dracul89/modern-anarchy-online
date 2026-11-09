//! The character's mail (`Mail`, `333b2867`), as the 2026-11-08 capture shows a shop delivery being read at a mail terminal.
//! Every message is a header byte, an `i16` action and the rest. The client asks (action 1 with id 0) for the inbox, (1, id)
//! to open a mail, (3, id) to take what is attached and (5, id) to delete it; the server answers with the list (action 0), the
//! opened mail (2) and the flags as they change (4), and keeps stat 0x289 at 1 while unread mail waits. The item taken goes
//! into the backpack with the `TemplateAction` / `ContainerAddItem` pair the capture shows (`useitem`).
//!
//! The words of a mail are the server's own.

use mao_net::n3::{Message, CHARACTER};

pub const MAIL: u32 = 0x333b2867;
/// The stat the client shows unread mail by (1 while there is some).
pub const UNREAD_STAT: u32 = 0x289;
/// The flags of a mail: the captured shop mail arrives as 0x5c, opening sets bit 0, taking its attachment bit 1.
pub const READ: u32 = 0x01;
pub const TAKEN: u32 = 0x02;
pub const SHOP_FLAGS: u32 = 0x5c;
/// How long a mail keeps: thirty days.
const KEEP: u32 = 30 * 24 * 3600;

/// A mail in the inbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mail {
    pub id: u32,
    pub sender: String,
    pub subject: String,
    pub body: String,
    pub credits: u32,
    /// The attached item: (low id, high id, quality).
    pub item: Option<(u32, u32, u32)>,
    pub flags: u32,
    /// Unix seconds.
    pub sent: u32,
    pub expires: u32,
}

impl Mail {
    /// A shop's delivery of a bought item.
    pub fn from_shop(id: u32, template: u32, quality: u32, credits: u32, now: u32) -> Mail {
        Mail {
            id,
            sender: "Shop".to_string(),
            subject: "Your purchase".to_string(),
            body: "The item you bought is attached.".to_string(),
            credits,
            item: Some((template, template, quality)),
            flags: SHOP_FLAGS,
            sent: now,
            expires: now.wrapping_add(KEEP),
        }
    }

    pub fn is_unread(&self) -> bool {
        self.flags & READ == 0
    }

    pub fn is_taken(&self) -> bool {
        self.flags & TAKEN != 0
    }
}

/// The client's request: its action and the mail it names (0 for the inbox).
pub fn request(data: &[u8]) -> Option<(i16, u32)> {
    let action = i16::from_be_bytes(data.get(1..3)?.try_into().ok()?);
    let id = u32::from_be_bytes(data.get(7..11)?.try_into().ok()?);
    Some((action, id))
}

fn push_text(out: &mut Vec<u8>, text: &str) {
    let bytes: Vec<u8> = text.bytes().take(0x7fff).collect();
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(&bytes);
}

/// A mail's head, as the list and the opened mail carry it: a zero word, the id, a zero word (the sender's id: none for the
/// shop), the sender, the subject, the times and the flags.
fn head(out: &mut Vec<u8>, m: &Mail) {
    for word in [0, m.id, 0] {
        out.extend_from_slice(&word.to_be_bytes());
    }
    push_text(out, &m.sender);
    push_text(out, &m.subject);
    for word in [m.sent, m.expires, m.flags] {
        out.extend_from_slice(&word.to_be_bytes());
    }
}

fn message(character: u32, action: i16, rest: Vec<u8>) -> Message {
    let mut data = vec![0u8];
    data.extend_from_slice(&action.to_be_bytes());
    data.extend_from_slice(&rest);
    Message { kind: MAIL, identity: (CHARACTER, character), data }
}

/// The inbox (action 0): the count as `(n + 1) * 0x3f1`, then each mail's head and whether it went express.
pub fn list(character: u32, mails: &[Mail]) -> Message {
    let mut rest = ((mails.len() as u32 + 1) * 0x3f1).to_be_bytes().to_vec();
    for m in mails {
        head(&mut rest, m);
        rest.push(1);
    }
    message(character, 0, rest)
}

/// A mail opened (action 2): its head, a byte, the credits, the item (low, high, quality), a zero, a one and the body.
pub fn opened(character: u32, m: &Mail) -> Message {
    let mut rest = Vec::new();
    head(&mut rest, m);
    rest.push(0);
    let (low, high, quality) = m.item.unwrap_or((0, 0, 0));
    for word in [m.credits, low, high, quality, 0, 1] {
        rest.extend_from_slice(&word.to_be_bytes());
    }
    push_text(&mut rest, &m.body);
    message(character, 2, rest)
}

/// A mail's flags as they are now (action 4).
pub fn flags(character: u32, id: u32, flags: u32) -> Message {
    let mut rest = Vec::new();
    for word in [0, id, flags] {
        rest.extend_from_slice(&word.to_be_bytes());
    }
    message(character, 4, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shop_mail() -> Mail {
        Mail::from_shop(0x0157_4385, 303_463, 1, 0, 0x6af1_0fc6)
    }

    #[test]
    fn the_requests_are_read() {
        assert_eq!(request(&[0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0]), Some((1, 0)));
        assert_eq!(request(&[0, 0, 3, 0, 0, 0, 0, 1, 0x57, 0x43, 0x85]), Some((3, 0x0157_4385)));
        assert_eq!(request(&[0, 0, 5]), None);
    }

    #[test]
    fn the_list_and_the_opened_mail_are_what_the_client_reads() {
        let m = shop_mail();
        let entries = mao_net::mail::parse_list(&list(7, &[m.clone()]).data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!((entries[0].id, entries[0].sender.as_str(), entries[0].flags, entries[0].express), (m.id, "Shop", SHOP_FLAGS, true));
        assert_eq!(mao_net::mail::parse_list(&list(7, &[]).data).unwrap(), vec![]);
        let opened = mao_net::mail::parse_mail(&opened(7, &m).data).unwrap();
        assert_eq!((opened.id, opened.item, opened.body.as_str()), (m.id, Some((303_463, 303_463, 1)), "The item you bought is attached."));
        assert_eq!(mao_net::mail::parse_flags(&flags(7, m.id, SHOP_FLAGS | READ).data), Some((m.id, 0x5d)));
    }

    #[test]
    fn the_list_has_the_captured_layout() {
        // (the captured list of one shop mail: a zero word, the id, a zero word, the texts, the times, the flags, express)
        let data = list(7, &[shop_mail()]).data;
        assert_eq!(&data[..3], &[0, 0, 0]);
        assert_eq!(&data[3..7], &(2 * 0x3f1u32).to_be_bytes());
        assert_eq!(&data[7..15], &[0, 0, 0, 0, 0x01, 0x57, 0x43, 0x85]);
        assert_eq!(data[data.len() - 1], 1);
    }
}
