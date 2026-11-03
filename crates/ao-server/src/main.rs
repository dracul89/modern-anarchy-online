//! The Anarchy Online server: the login / chat manager and the game.

use std::net::SocketAddr;
use std::time::Duration;

use ao_server::server;

fn main() -> std::io::Result<()> {
    let login_addr: SocketAddr = env_login_addr()?;
    let game_addr: SocketAddr = env_game_addr()?;
    let db_path = std::env::var("MAO_DB").unwrap_or_else(|_| "ao-server-data.db".to_string());
    let server = server::Server::new(login_addr, game_addr, &db_path);
    ao_server::trace::open();
    server.spawn_listeners()?;
    let admin_addr: SocketAddr = std::env::var("MAO_ADMIN_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string()).parse().map_err(|e| std::io::Error::other(e))?;
    ao_server::admin::spawn(admin_addr, server.db())?;
    println!("ao-server: login on {login_addr}, game on {game_addr}, database {db_path}");
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

/// The login / chat manager listens here: the one the client reaches for the
/// login and the chat.
fn env_login_addr() -> std::io::Result<SocketAddr> {
    let addr = std::env::var("MAO_LOGIN_ADDR").unwrap_or_else(|_| "127.0.0.1:7105".to_string());
    addr.parse().map_err(|e| std::io::Error::other(e))
}

/// The game server listens here: the one the character that logged in goes to.
fn env_game_addr() -> std::io::Result<SocketAddr> {
    let addr = std::env::var("MAO_GAME_ADDR").unwrap_or_else(|_| "127.0.0.1:7500".to_string());
    addr.parse().map_err(|e| std::io::Error::other(e))
}
