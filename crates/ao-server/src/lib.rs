//! A robust Anarchy Online server, rebuilt from the protocol documentation
//! kept in `rea_ao` (the captures, the `mao-net` protocol code and the notes in
//! `modern-anarchy-online/docs`).

pub mod chat;
pub mod create;
pub mod db;
pub mod frames;
pub mod game;
pub mod gm;
pub mod inventory;
pub mod login;
pub mod progress;
pub mod server;
pub mod shop;
pub mod templates;

pub use frames::*;
