//! Reading and writing the game's TCP connections without socket time-outs.
//!
//! A Windows socket whose receive time-out (`SO_RCVTIMEO`) expires is left in an undefined state ("the socket state is
//! indeterminate, and should not be used": a read cancelled as data arrives loses it). The live client hit that as a
//! read failing with ERROR_IO_PENDING (997) or the next frame starting mid-stream. So the streams are non-blocking and a
//! read with a time-out polls instead.

use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// Makes the stream non-blocking (once, after connecting).
pub fn prepare(stream: &TcpStream) -> std::io::Result<()> {
    stream.set_nodelay(true)?;
    stream.set_nonblocking(true)
}

/// Reads what has come, waiting for something up to `timeout`; a `TimedOut` error when nothing came.
pub fn read(stream: &mut TcpStream, buffer: &mut [u8], timeout: Duration) -> std::io::Result<usize> {
    let until = Instant::now() + timeout;
    loop {
        match stream.read(buffer) {
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= until {
                    return Err(ErrorKind::TimedOut.into());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            other => return other,
        }
    }
}

/// Writes all of `bytes`, waiting while the send buffer is full.
pub fn write_all(stream: &mut TcpStream, mut bytes: &[u8]) -> std::io::Result<()> {
    let until = Instant::now() + Duration::from_secs(20);
    while !bytes.is_empty() {
        match stream.write(bytes) {
            Ok(0) => return Err(ErrorKind::WriteZero.into()),
            Ok(n) => bytes = &bytes[n..],
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= until {
                    return Err(ErrorKind::TimedOut.into());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
