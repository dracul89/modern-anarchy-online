//! A robust Anarchy Online server, rebuilt from the protocol documentation
//! kept in `rea_ao` (the captures, the `mao-net` protocol code and the notes in
//! `modern-anarchy-online/docs`).

pub mod admin;
pub mod chat;
pub mod create;
pub mod db;
pub mod frames;
pub mod game;
pub mod gm;
pub mod inventory;
pub mod knubot;
pub mod login;
pub mod mail;
pub mod missions;
pub mod monsters;
pub mod progress;
pub mod server;
pub mod shop;
pub mod templates;
pub mod trace;
pub mod useitem;

pub use frames::*;
