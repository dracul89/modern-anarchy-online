//! The network protocol, as the original client speaks it to the game's
//! "chat manager" server (`cm.d1.funcom.com`, the ports listed in the
//! launcher's dimension list).
//!
//! Derived from the original's `Connection.dll` (framing), `MessageProtocol.dll`
//! (messages), the login code in `Interfaces.dll`, and a capture of a real
//! login made with `tools/capture_proxy.py`.
//!
//! * [`frame`]: the 16-byte framing every message travels in.
//! * [`crypto`]: the login's challenge response.
//! * [`system`]: the system messages of the login, and the character list.
//! * [`n3`]: the game world's messages.
//! * [`session`]: a blocking connection that logs in and lists characters.

pub mod crypto;
pub mod frame;
pub mod n3;
pub mod session;
pub mod system;
