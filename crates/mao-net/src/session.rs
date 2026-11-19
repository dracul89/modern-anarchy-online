//! A blocking connection to the server: log in and list the account's
//! characters. The password is only ever used to build the response; it is
//! never stored, printed or put in an error.

use std::fmt;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::frame::{self, Frame, Reader};
use crate::system::{self, Character};

/// Rubi-Ka, from the launcher's dimension list.
pub const RUBI_KA: &str = "cm.d1.funcom.com:7505";

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Frame(frame::Error),
    Message(system::Error),
    Random(getrandom::Error),
    /// The server answered the login with something else than the
    /// character list: the system message's number and the code it
    /// carries (`Client_t::ProcessMessage` 0x10002a9e reads one number
    /// of message 0xd, the login refused, and hands both on).
    Refused { kind: u32, code: Option<u32>, text: String },
    /// The server answered the login with system message 0x4e ([`system::LIMBO`]): a character of the account is still
    /// in the world, and may be logged in again for the time it says (the original's limbo window,
    /// `LoginModule_c::SlotLoginLimbo` 0x10011c56). The connection stays logged in: a 0x16 for that character on it
    /// enters it.
    Limbo(Box<system::Limbo>),
    Closed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "network: {e}"),
            Error::Frame(e) => write!(f, "framing: {e}"),
            Error::Message(e) => write!(f, "message: {e}"),
            Error::Random(e) => write!(f, "random numbers: {e}"),
            Error::Refused { kind, code, text } => write!(f, "the server answered with system message {kind:#x} (code {code:?}, {text:?}) instead of the character list"),
            Error::Limbo(l) => write!(f, "the character {:?} is still in the world (limbo, {} seconds)", l.character.name, l.timeout),
            Error::Closed => write!(f, "the server closed the connection"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
impl From<frame::Error> for Error {
    fn from(e: frame::Error) -> Self {
        Error::Frame(e)
    }
}
impl From<system::Error> for Error {
    fn from(e: system::Error) -> Self {
        Error::Message(e)
    }
}

pub struct Connection {
    stream: TcpStream,
    reader: Reader,
    sequence: u16,
    timeout: Duration,
}

impl Connection {
    pub fn connect(address: impl ToSocketAddrs) -> Result<Self, Error> {
        let stream = TcpStream::connect(address)?;
        crate::socket::prepare(&stream)?;
        Ok(Connection { stream, reader: Reader::default(), sequence: 0, timeout: Duration::from_secs(20) })
    }

    /// How long `receive` waits for data before failing with a timeout
    /// error (the connection stays usable).
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), Error> {
        self.timeout = timeout;
        Ok(())
    }

    pub fn send(&mut self, frame: &Frame) -> Result<(), Error> {
        self.sequence = self.sequence.wrapping_add(1);
        if self.sequence == 0xffff {
            self.sequence = 0;
        }
        crate::socket::write_all(&mut self.stream, &frame.encode(self.sequence))?;
        Ok(())
    }

    /// Closes the connection (the server must see it go before the character logs in again).
    pub fn close(&self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }

    /// The next message if one has already come (nothing is waited for).
    pub fn try_receive(&mut self) -> Result<Option<Frame>, Error> {
        loop {
            if let Some(frame) = self.reader.next()? {
                return Ok(Some(frame));
            }
            let mut buffer = [0u8; 8192];
            let n = match crate::socket::read(&mut self.stream, &mut buffer, Duration::ZERO) {
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => return Ok(None),
                Err(e) => return Err(e.into()),
            };
            if n == 0 {
                return Err(Error::Closed);
            }
            self.reader.push(&buffer[..n])?;
        }
    }

    /// The next message, waiting for it.
    pub fn receive(&mut self) -> Result<Frame, Error> {
        loop {
            if let Some(frame) = self.reader.next()? {
                return Ok(frame);
            }
            let mut buffer = [0u8; 8192];
            let n = crate::socket::read(&mut self.stream, &mut buffer, self.timeout)?;
            if n == 0 {
                return Err(Error::Closed);
            }
            self.reader.push(&buffer[..n])?;
        }
    }
}

/// Log in and list the account's characters. `version` is the first line of
/// the game's `version.id`.
pub fn login(connection: &mut Connection, user: &str, password: &str, version: &str) -> Result<Vec<Character>, Error> {
    login_account(connection, user, password, version).map(|(list, _)| list)
}

/// What follows the characters in the list: the account's character slots and its expansion flags (GUI.dll's
/// `CharSelectWindow_c::SlotCharListReceived` 0x1000ebd5 takes them so: the slots for "x/y slots available", the flags
/// as the value `ExpansionFlags`, whose bit 2 is the Shadowlands).
pub fn account_trailer(rest: &[u8]) -> Option<(u32, u32)> {
    let word = |at: usize| Some(u32::from_be_bytes(rest.get(at..at + 4)?.try_into().ok()?));
    Some((word(0)?, word(4)?))
}

/// The list's third number after the characters, the one the captures have as zero: Interfaces.dll's
/// `Client_t::ProcessMessage` 0x10002a9e (message 0xe, at 0x1000319b) sets `Client_t::s_nSLProfsEnabled` to whether
/// it is not zero, which lets the Keeper and the Shade through whatever the expansion flags. False when the list has
/// no such number.
pub fn sl_professions_enabled(rest: &[u8]) -> bool {
    rest.get(8..12).is_some_and(|w| w.iter().any(|&b| b != 0))
}

/// [`login`], with the account's slots, expansion flags and [`sl_professions_enabled`] when the list has them.
pub fn login_account(connection: &mut Connection, user: &str, password: &str, version: &str) -> Result<(Vec<Character>, Option<(u32, u32, bool)>), Error> {
    connection.send(&system::init_auth(user, version))?;
    let challenge = system::challenge(&connection.receive()?)?;
    let response = crate::crypto::response(user, &challenge, password).map_err(Error::Random)?;
    connection.send(&system::auth_client(user, &response))?;
    let reply = connection.receive()?;
    match system::character_list(&reply) {
        Ok((list, rest)) => Ok((list, account_trailer(&rest).map(|(slots, flags)| (slots, flags, sl_professions_enabled(&rest))))),
        Err(_) => {
            let (kind, payload) = system::split(&reply)?;
            // (Interfaces.dll's `Client_t::ProcessMessage` reads 0x4e for the limbo window, not as a refusal)
            if kind == system::LIMBO {
                return Err(Error::Limbo(Box::new(system::limbo(&reply)?)));
            }
            Err(Error::Refused { kind, code: refusal_code(payload), text: String::from_utf8_lossy(payload).chars().filter(|c| !c.is_control()).collect() })
        }
    }
}

/// The code of a refusal: the first number of its message.
pub fn refusal_code(payload: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(payload.get(..4)?.try_into().ok()?))
}

/// The ticket of the last character logged in: its cookies and the account's ids are what the in-game web pages are
/// sent (`Client_t`'s `s_nCookie1`, `s_nCookie2`, `s_nEventServerType`, `s_nPlayerID`).
static LAST_TICKET: std::sync::Mutex<Option<system::Ticket>> = std::sync::Mutex::new(None);

/// Log a character in on the chat server: the game server to go to. Another system message in the ticket's place is
/// a refusal ([`Error::Refused`] with its kind and code: the original's `LoginModule_c::SlotLoginReply` 0x10012179 shows
/// 0x10 and 0x21 with their error pages, as 0xd).
pub fn login_character(connection: &mut Connection, id: u32) -> Result<system::Ticket, Error> {
    connection.send(&system::login_character(id))?;
    let reply = connection.receive()?;
    if let Ok((kind, payload)) = system::split(&reply)
        && kind != system::CHARACTER_TICKET
    {
        return Err(Error::Refused { kind, code: refusal_code(payload), text: String::from_utf8_lossy(payload).chars().filter(|c| !c.is_control()).collect() });
    }
    let ticket = system::ticket(&reply)?;
    if let Ok(mut last) = LAST_TICKET.lock() {
        *last = Some(ticket.clone());
    }
    Ok(ticket)
}

/// Sends a creation message ([`crate::create`]) on a logged-in connection and waits for the login server's answer (other
/// messages that come first are passed over).
pub fn ask_creation(connection: &mut Connection, frame: &Frame) -> Result<crate::create::Reply, Error> {
    connection.send(frame)?;
    loop {
        let reply = connection.receive()?;
        if let Ok(Some(r)) = crate::create::reply(&reply) {
            return Ok(r);
        }
    }
}

/// The ticket the last [`login_character`] got.
pub fn last_ticket() -> Option<system::Ticket> {
    LAST_TICKET.lock().ok().and_then(|t| t.clone())
}

/// Enter the world on the game server named in the ticket. The connection
/// is returned with the first messages still to read. The caller sends
/// [`crate::n3::ready`] once the world has arrived, as the original does
/// after loading: the server ignores a ready that comes earlier, and then
/// accepts no movement.
pub fn enter_world(ticket: &system::Ticket) -> Result<Connection, Error> {
    let mut connection = Connection::connect(ticket.host().as_str())?;
    connection.send(&system::enter_game_server(ticket))?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The list's trailer as the captures have it: the slots, the expansion flags, a zero.
    #[test]
    fn the_account_trailer() {
        assert_eq!(account_trailer(&[0, 0, 0, 8, 0, 0, 0, 0x7f, 0, 0, 0, 0]), Some((8, 0x7f)));
        assert_eq!(account_trailer(&[0, 0, 0, 8]), None);
        assert_eq!(account_trailer(&[]), None);
        // (the third number: `s_nSLProfsEnabled`)
        assert!(!sl_professions_enabled(&[0, 0, 0, 8, 0, 0, 0, 0x7f, 0, 0, 0, 0]));
        assert!(sl_professions_enabled(&[0, 0, 0, 8, 0, 0, 0, 0x7f, 0, 0, 0, 1]));
        assert!(!sl_professions_enabled(&[0, 0, 0, 8, 0, 0, 0, 0x7f]));
    }

    #[test]
    fn a_refusal_has_its_code() {
        // (a login with a wrong password, as the server answered it on 2026-11-12: message 0xd with the number 106)
        assert_eq!(refusal_code(&[0, 0, 0, 106]), Some(106));
        assert_eq!(refusal_code(&[0, 0, 1, 0, 9]), Some(256));
        assert_eq!(refusal_code(&[0, 0]), None);
    }
}
