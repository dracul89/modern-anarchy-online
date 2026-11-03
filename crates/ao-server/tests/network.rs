//! The server over real sockets, as the client reaches it: log in, make a character, log in again and list it, enter the world
//! through the ticket, and use the game-master commands. The servers listen on free ports of this machine, and their database is
//! in memory.

use std::net::{SocketAddr, TcpListener};
use std::time::{Duration, Instant};

use mao_net::create::{create_character, NewCharacter};
use mao_net::frame::{Frame, TEXT};
use mao_net::n3;
use mao_net::session::{self, Connection};

fn free_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    listener.local_addr().expect("the port")
}

/// The client's typed line, as a `TEXT` frame from the character (the vicinity text the chat window sends).
fn typed(character: u32, line: &str) -> Frame {
    let text = line.as_bytes();
    let mut vicinity = (text.len() as u16).to_be_bytes().to_vec();
    vicinity.extend_from_slice(text);
    vicinity.push(0);
    let mut body = 3u32.to_be_bytes().to_vec();
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(&(vicinity.len() as u32).to_be_bytes());
    body.extend_from_slice(&vicinity);
    Frame { kind: TEXT, source: character, destination: 2, body }
}

/// The messages the game sends for a while: their kinds, until `wanted` are all seen or the time is up.
fn collect(world: &mut Connection, wanted: &[u32], within: Duration) -> Vec<(u32, Vec<u8>)> {
    world.set_timeout(Duration::from_millis(200)).expect("a timeout");
    let until = Instant::now() + within;
    let mut seen = Vec::new();
    while Instant::now() < until && !wanted.iter().all(|k| seen.iter().any(|(s, _): &(u32, Vec<u8>)| s == k)) {
        if let Ok(frame) = world.receive() {
            if let Some(m) = n3::Message::parse(&frame) {
                seen.push((m.kind, m.data));
            }
        }
    }
    seen
}

#[test]
fn a_character_is_made_entered_and_commanded_over_the_network() {
    let (login, game) = (free_address(), free_address());
    let server = ao_server::server::Server::new(login, game, ":memory:");
    server.spawn_listeners().expect("the listeners");
    // (an account is made in the admin panel; the test makes it the same way)
    server.db().save_account("testaccount", "not-checked").expect("the account");

    // The first login: an account with no characters.
    let mut connection = Connection::connect(login).expect("the login server");
    let list = session::login(&mut connection, "testaccount", "not-checked", "1.0").expect("the login");
    assert!(list.is_empty());

    // Make a character (the client's creation: the answer is the new character's id).
    let made = NewCharacter { name: "Nettest".into(), breed: 1, sex: 2, profession: 1, head: 0, height: 100, width: 0 };
    connection.send(&create_character(&made)).expect("the creation");
    let reply = connection.receive().expect("the answer");
    assert_eq!(&reply.body[..4], &[0, 0, 0, 0x11], "created");
    drop(connection);

    // Log in again: the character is listed, and the game ticket is for the game server.
    let mut connection = Connection::connect(login).expect("the login server");
    let list = session::login(&mut connection, "testaccount", "not-checked", "1.0").expect("the login");
    assert_eq!(list.len(), 1);
    let id = list[0].id;
    let ticket = session::login_character(&mut connection, id).expect("the ticket");
    assert_eq!(ticket.port, game.port());
    drop(connection);

    // Enter the world, say ready, and see the stats and the items.
    let mut world = session::enter_world(&ticket).expect("the game server");
    world.send(&n3::ready(id)).expect("ready");
    let seen = collect(&mut world, &[0x2b333d6e, 0x4e536976], Duration::from_secs(5));
    assert!(seen.iter().any(|(k, _)| *k == 0x2b333d6e), "the stats: {:x?}", seen.iter().map(|(k, _)| k).collect::<Vec<_>>());
    assert!(seen.iter().any(|(k, _)| *k == 0x4e536976), "the items");

    // A game-master command: experience (the level and the stats change, and the chat says so).
    world.send(&typed(id, "/gm xp 100000")).expect("the command");
    let seen = collect(&mut world, &[0x2b333d6e, 0x5f4b442a], Duration::from_secs(5));
    assert!(seen.iter().any(|(k, _)| *k == 0x5f4b442a), "the chat line");
    let stats = seen.iter().find(|(k, _)| *k == 0x2b333d6e).expect("the stats after the experience");
    let parsed = mao_net::schema::Schema::global().parse_n3(0x2b333d6e, &stats.1).expect("the stats parse");
    let mao_net::schema::Value::List(items) = parsed.value.get("Stats").expect("the list") else { panic!("no list") };
    let level = items.iter().find(|s| s.get("Item1").and_then(|v| v.as_int()) == Some(0x36)).and_then(|s| s.get("Item2")).and_then(|v| v.as_int());
    assert!(level.is_some_and(|l| l > 1), "level {level:?}");

    // The character's progress is kept: a login after this one lists the new level.
    drop(world);
    std::thread::sleep(Duration::from_millis(200));
    let mut connection = Connection::connect(login).expect("the login server");
    let list = session::login(&mut connection, "testaccount", "not-checked", "1.0").expect("the login");
    assert!(list[0].level > 1, "the list shows level {}", list[0].level);
}
