//! The protocol layer of the server: the frames and the chat packets it
//! answers with are the ones the captures show (the capture vectors of
//! `mao-net`).

use ao_server::chat;
use ao_server::frames;
use mao_net::frame::{N3, SYSTEM};
use mao_net::n3;
use mao_net::system;

#[test]
fn the_challenge_is_the_captured_one() {
    let f = frames::challenge(1, 0xba20, "75185ea0b3e792676b76b0741410e9d4");
    assert_eq!(f.kind, SYSTEM);
    assert_eq!(f.source, 1);
    assert_eq!(f.destination, 0xba20);
    assert_eq!(&f.body[..4], &[0, 0, 0, 0x24]);
    assert_eq!(&f.body[4..], b"75185ea0b3e792676b76b0741410e9d4");
    let (kind, payload) = system::split(&f).unwrap();
    assert_eq!(kind, 0x24);
    assert_eq!(system::challenge(&f).unwrap(), "75185ea0b3e792676b76b0741410e9d4");
    assert_eq!(payload.len(), 32);
}

#[test]
fn the_position_update_is_the_captured_shape() {
    let f = frames::position_update(0x9210c, 12, [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0], 7661);
    let m = n3::Message::parse(&f).unwrap();
    let (kind, _rotation, position) = m.movement().unwrap();
    assert_eq!(kind, 12);
    assert_eq!(position, [0.0, 0.0, 0.0]);
    let mine = n3::position_update(0x9210c, 12, 0.0, [0.0, 0.0, 0.0], 7661);
    assert_eq!(f.body, mine.body);
}

#[test]
fn the_ready_frame_is_the_captured_shape() {
    let f = frames::ready(0x7b1a0021);
    assert_eq!(f.kind, N3);
    assert_eq!(f.source, 0x7b1a0021);
    let m = n3::Message::parse(&f).unwrap();
    assert_eq!(m.kind, n3::READY);
}

#[test]
fn the_game_time_frame_carries_the_server_clock() {
    let f = frames::game_time(0x7b1a0021, 2, 123.5, 27, 0, 0);
    let m = n3::Message::parse(&f).unwrap();
    let (seconds, period, days, server_time) = m.game_time().unwrap();
    assert_eq!((seconds, period, days, server_time), (123.5, 27, 0, 0));
}

#[test]
fn the_redirect_names_the_game_server() {
    let f = frames::redirect(1, 0, [37, 18, 192, 41], 7513);
    assert_eq!(f.kind, SYSTEM);
    let (kind, payload) = system::split(&f).unwrap();
    assert_eq!(kind, 0x3c);
    assert_eq!(payload.len(), 6);
    assert_eq!(system::redirect(&f).as_deref(), Some("37.18.192.41:7513"));
}

#[test]
fn the_chat_login_ok_is_the_captured_shape() {
    assert_eq!(chat::login_ok(), vec![0u8, 5, 0, 0]);
}

#[test]
fn the_login_answers_the_challenge_and_logs_the_character_in() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stream = std::net::TcpStream::connect(addr).unwrap();
    let mut session = ao_server::login::LoginSession::new(stream);
    let answered = mao_net::system::init_auth("someone", "18.8.62_EP1");
    let (phase, replies) = session.handle(&answered, &[]);
    assert_eq!(phase, ao_server::login::Phase::AwaitingAuth);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].kind, mao_net::frame::SYSTEM);
    assert_eq!(u32::from_be_bytes(replies[0].body[..4].try_into().unwrap()), ao_server::login::CHALLENGE);
    let accepted = mao_net::system::auth_client("someone", &"a".repeat(32));
    let (phase, replies) = session.handle(&accepted, &[]);
    assert_eq!(phase, ao_server::login::Phase::Authenticated);
    assert_eq!(replies.len(), 1);
    let entered = mao_net::system::login_character(0x9210c);
    let (phase, replies) = session.handle(&entered, &[]);
    assert_eq!(phase, ao_server::login::Phase::CharacterSelected);
    assert_eq!(replies.len(), 1);
    let ticket = mao_net::system::ticket(&replies[0]).unwrap();
    assert_eq!(ticket.address, [127, 0, 0, 1]);
    assert_eq!(ticket.port, 7500);
}

#[test]
fn the_login_answers_a_creation_with_the_new_character() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stream = std::net::TcpStream::connect(addr).unwrap();
    let mut session = ao_server::login::LoginSession::new(stream);
    let _ = session.handle(&mao_net::system::init_auth("someone", "18.8.62_EP1"), &[]);
    let _ = session.handle(&mao_net::system::auth_client("someone", &"a".repeat(32)), &[]);
    let _ = session.handle(&mao_net::system::login_character(0x9210c), &[]);
    let made = mao_net::create::create_character(&mao_net::create::NewCharacter { name: "Worp".into(), breed: 1, sex: 2, profession: 1, head: 0, height: 100, width: 0 });
    let (phase, replies) = session.handle(&made, &[]);
    assert_eq!(phase, ao_server::login::Phase::CharacterSelected);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].kind, mao_net::frame::SYSTEM);
    assert_eq!(u32::from_be_bytes(replies[0].body[..4].try_into().unwrap()), ao_server::login::CREATED);
    let id = u32::from_be_bytes(replies[0].body[4..8].try_into().unwrap());
    assert_eq!(id, session.character_id());
    assert_eq!(ao_server::create::read_new_character(&made.body[4..]).unwrap().name, "Worp");
}

#[test]
fn the_game_answers_the_ready_with_the_playfield_and_the_clock() {
    let mut session = ao_server::game::GameSession::new(0x9210c, ao_server::progress::Hero::new(0x9210c), None);
    let ready = frames::ready(0x9210c);
    let (sent, replies) = session.handle(&ready);
    assert!(sent);
    assert_eq!(replies.len(), 9);
    assert_eq!(n3::Message::parse(&replies[0]).unwrap().kind, ao_server::game::PLAYFIELD);
    assert_eq!(n3::Message::parse(&replies[1]).unwrap().kind, ao_server::game::GAME_TIME);
    assert_eq!(n3::Message::parse(&replies[2]).unwrap().kind, ao_server::game::POSITION_UPDATE);
    assert_eq!(n3::Message::parse(&replies[3]).unwrap().kind, 0x2b333d6e, "the stats");
    assert_eq!(n3::Message::parse(&replies[4]).unwrap().kind, 0x4e536976, "the items");
    assert!(replies[5..].iter().all(|f| n3::Message::parse(f).unwrap().kind == 0x271b3a6b), "the monsters of Old Athens");
}

#[test]
fn the_game_clock_and_position_are_read_back() {
    let mut session = ao_server::game::GameSession::new(0x9210c, ao_server::progress::Hero::new(0x9210c), None);
    let (sent, replies) = session.handle(&frames::game_time(0x9210c, 2, 123.5, 27, 0, 0));
    assert!(!sent);
    assert!(replies.is_empty());
    assert_eq!(session.clock, 123.5);
    let (sent, replies) = session.handle(&frames::position_update(0x9210c, 12, [0.0, 0.0, 0.0, 1.0], [3.0, 4.0, 5.0], 7661));
    assert!(!sent);
    assert!(replies.is_empty());
    assert_eq!(session.arrived, [3.0, 4.0, 5.0]);
}
