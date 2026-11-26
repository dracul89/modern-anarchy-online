//! Getting Anarchy Online's files on a machine that cannot run Funcom's installer and patcher: reading the installer
//! package Funcom's downloader fetches, and applying the patches the game's patcher applies. How each format was
//! read from the original programs is in `docs/INSTALL_WITHOUT_WINDOWS.md`.
//!
//! Nothing of Funcom's is in this crate: it reads files the player downloads from Funcom's servers.

pub mod crypto;
pub mod ctree;
pub mod fetch;
pub mod inno;
pub mod install;
pub mod patcher;
pub mod respatch;
pub mod rtpatch;
pub mod sha1;
