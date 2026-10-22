//! A blocking connection to the server: log in and list the account's
//! characters. The password is only ever used to build the response; it is
//! never stored, printed or put in an error.

use std::fmt;
use std::io::{Read, Write};
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
    /// character list.
    Refused { kind: u32, text: String },
    Closed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "network: {e}"),
            Error::Frame(e) => write!(f, "framing: {e}"),
            Error::Message(e) => write!(f, "message: {e}"),
            Error::Random(e) => write!(f, "random numbers: {e}"),
            Error::Refused { kind, text } => write!(f, "the server answered with system message {kind:#x} ({text:?}) instead of the character list"),
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
}

impl Connection {
    pub fn connect(address: impl ToSocketAddrs) -> Result<Self, Error> {
        let stream = TcpStream::connect(address)?;
        stream.set_read_timeout(Some(Duration::from_secs(20)))?;
        stream.set_nodelay(true)?;
        Ok(Connection { stream, reader: Reader::default(), sequence: 0 })
    }

    /// How long `receive` waits for data before failing with a timeout
    /// error (the connection stays usable).
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), Error> {
        Ok(self.stream.set_read_timeout(Some(timeout))?)
    }

    pub fn send(&mut self, frame: &Frame) -> Result<(), Error> {
        self.sequence = self.sequence.wrapping_add(1);
        if self.sequence == 0xffff {
            self.sequence = 0;
        }
        self.stream.write_all(&frame.encode(self.sequence))?;
        Ok(())
    }

    /// The next message, waiting for it.
    pub fn receive(&mut self) -> Result<Frame, Error> {
        loop {
            if let Some(frame) = self.reader.next()? {
                return Ok(frame);
            }
            let mut buffer = [0u8; 8192];
            let n = self.stream.read(&mut buffer)?;
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
    connection.send(&system::init_auth(user, version))?;
    let challenge = system::challenge(&connection.receive()?)?;
    let response = crate::crypto::response(user, &challenge, password).map_err(Error::Random)?;
    connection.send(&system::auth_client(user, &response))?;
    let reply = connection.receive()?;
    match system::character_list(&reply) {
        Ok((list, _rest)) => Ok(list),
        Err(_) => {
            let (kind, payload) = system::split(&reply)?;
            Err(Error::Refused { kind, text: String::from_utf8_lossy(payload).chars().filter(|c| !c.is_control()).collect() })
        }
    }
}

/// Log a character in on the chat server: the game server to go to.
pub fn login_character(connection: &mut Connection, id: u32) -> Result<system::Ticket, Error> {
    connection.send(&system::login_character(id))?;
    Ok(system::ticket(&connection.receive()?)?)
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
