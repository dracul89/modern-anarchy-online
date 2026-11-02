//! The game server connection: the world's messages of one character. A session answers the client's frames with the server's:
//! the world entry (the place, the time, the stats and the items), the character's own movement and sitting, the game-master
//! commands typed as chat, the vending machines (their stock, and buying from them). Progress is saved as it changes.

use std::sync::Arc;
use std::time::Instant;

use mao_net::frame::{Frame, N3, PING, TEXT};
use mao_net::n3;
use mao_net::schema::{Schema, Value};

use crate::db::Db;
use crate::progress::{self, Hero};

/// The playfield the character is in (the default: the first place a new character is put).
pub const PLAYFIELD: u32 = 0x5f4b1a39;
/// The server's game clock.
pub const GAME_TIME: u32 = 0x5f52412e;
/// The client's first message after entering: the world is ready.
pub const READY: u32 = 0x570c2039;
/// A position update.
pub const POSITION_UPDATE: u32 = 0x54111123;
/// A zoning / move within the playfield.
pub const N3_TELEPORT: u32 = 0x43197d22;
/// The action message: the actions the character performs (`0x5e477770`).
pub const ACTION: u32 = 0x5e477770;
/// `GenericCmd` (`52526858`): a use of an object the character names.
pub const GENERIC_CMD: u32 = 0x52526858;
/// `Trade` (`36284f6e`): a trade, a shop's purchase among it.
pub const TRADE: u32 = 0x36284f6e;
/// `ChatText` (`5f4b442a`): a line the game shows in the chat (two zero bytes, the length and the text).
pub const CHAT_TEXT: u32 = 0x5f4b442a;
/// The stand-up action: `0x57`, sent after the world is ready to free the new connection from the ignored movement.
pub const STAND: u8 = 0x57;
/// The logout action: `0x78`, the character sits and the server starts the timed logout.
pub const LOGOUT: u8 = 0x78;
/// The server stops a timed logout: the action it sends when the countdown is given up (`0x7a`).
pub const STOP_LOGOUT: u8 = 0x7a;
/// The source the server's frames carry (the zone's object the captures show).
pub const SERVER: u32 = 0xdbc;
/// `QuestAlternative` (`5c436609`): a mission terminal's request for the missions it offers.
pub const QUEST_ALTERNATIVE: u32 = 0x5c436609;
/// `CreateQuest` (`291f361b`): the character takes an offered mission.
pub const CREATE_QUEST: u32 = 0x291f361b;
/// The most missions a character can have taken at once.
pub const MAX_MISSIONS: usize = 25;
/// `GenericCmd`'s action `Use`.
const USE: i64 = 3;

pub struct GameSession {
    pub character_id: u32,
    pub ready_sent: bool,
    pub playfield: u32,
    pub arrived: [f32; 3],
    pub clock: f32,
    started: Instant,
    /// The movement of a new connection is ignored until the stand-up frees it.
    pub movement_allowed: bool,
    /// The character is logging out: the server stops the timed logout.
    pub logging_out: bool,
    /// The character's progress (saved to `db` as it changes).
    pub hero: Hero,
    db: Option<Arc<Db>>,
}

/// A server message to the character: the zone is the source, the character the destination.
fn to_client(character: u32, kind: u32, identity: (u32, u32), data: Vec<u8>) -> Frame {
    let mut frame = n3::Message { kind, identity, data }.frame(SERVER);
    frame.destination = character;
    frame
}

/// The text a chat line carries (`ChatText`'s data): two zero bytes, the length, the text.
fn chat_line(character: u32, text: &str) -> Frame {
    let text: Vec<u8> = text.bytes().take(255).collect();
    let mut data = vec![0, 0, text.len() as u8];
    data.extend_from_slice(&text);
    to_client(character, CHAT_TEXT, (n3::CHARACTER, character), data)
}

/// The text of a vicinity line the client sent (the `TEXT` frame's data: a `u16` length, the text, the kind).
fn typed_text(body: &[u8]) -> Option<String> {
    let len = u32::from_be_bytes(body.get(12..16)?.try_into().ok()?) as usize;
    let data = body.get(16..16 + len)?;
    let n = u16::from_be_bytes(data.get(0..2)?.try_into().ok()?) as usize;
    String::from_utf8(data.get(2..2 + n)?.to_vec()).ok()
}

fn field_int(v: &Value, name: &str) -> i64 {
    v.get(name).and_then(Value::as_int).unwrap_or(0)
}

impl GameSession {
    /// A session for a character with its progress (`db`, when given, is where the progress is saved).
    pub fn new(character_id: u32, hero: Hero, db: Option<Arc<Db>>) -> Self {
        GameSession {
            character_id,
            ready_sent: false,
            playfield: if hero.playfield == 0 { PLAYFIELD } else { hero.playfield },
            arrived: hero.position,
            clock: 0.0,
            started: Instant::now(),
            movement_allowed: false,
            logging_out: false,
            hero,
            db,
        }
    }

    pub fn ready_sent(&self) -> bool {
        self.ready_sent
    }

    /// The client's own clock, for the ping's receive / transmit stamps.
    pub fn clock_ms(&self) -> u32 {
        self.started.elapsed().as_millis() as u32
    }

    /// Saves the progress (a failure to save is not the client's to see: the session carries on).
    fn save(&self) {
        if let Some(db) = &self.db {
            let _ = db.save_hero(&self.hero);
        }
    }

    /// The world entry: the place, the time, the position, the stats and the items.
    fn world_entry(&self, out: &mut Vec<Frame>) {
        out.push(crate::frames::playfield(self.character_id, 2, self.playfield, self.arrived));
        out.push(crate::frames::game_time(self.character_id, 2, self.clock, 27, 0, 0));
        out.push(crate::frames::position_update(self.character_id, 0, [0.0, 0.0, 0.0, 1.0], self.arrived, 0));
        if let Ok(m) = progress::stat_message(self.character_id, &progress::stats(&self.hero)) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
        if let Ok(m) = crate::inventory::update(self.character_id, &self.hero.items) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
    }

    /// The stats as they are now, after a change to them.
    fn stats_changed(&self, out: &mut Vec<Frame>) {
        if let Ok(m) = progress::stat_message(self.character_id, &progress::stats(&self.hero)) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
    }

    /// The items as they are now, after a change to them.
    fn items_changed(&self, out: &mut Vec<Frame>) {
        if let Ok(m) = crate::inventory::update(self.character_id, &self.hero.items) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
    }

    /// The frames to send back in answer, and whether the world went ready.
    pub fn handle(&mut self, frame: &Frame) -> (bool, Vec<Frame>) {
        let mut out = Vec::new();
        if self.logging_out {
            // The character has logged out: the world is not answered any more.
            return (self.ready_sent, out);
        }
        if frame.kind == PING {
            if let Some(reply) = mao_net::frame::ping_reply(frame, self.character_id, self.clock_ms()) {
                out.push(reply);
            }
            return (self.ready_sent, out);
        }
        if frame.kind == TEXT && frame.source == self.character_id {
            if let Some(text) = typed_text(&frame.body) {
                self.typed(&text, &mut out);
            }
            return (self.ready_sent, out);
        }
        if frame.kind == N3 {
            let Some(m) = n3::Message::parse(frame) else { return (self.ready_sent, out) };
            match m.kind {
                READY if frame.source == self.character_id => {
                    self.ready_sent = true;
                    self.world_entry(&mut out);
                }
                GAME_TIME => {
                    if let Some((seconds, _period, _days, _server_time)) = m.game_time() {
                        self.clock = seconds;
                    }
                }
                POSITION_UPDATE if frame.source == self.character_id => {
                    if let Some((_movement, _rotation, position)) = m.movement() {
                        self.arrived = position;
                        self.hero.position = position;
                    }
                }
                ACTION if frame.source == self.character_id => {
                    let action = frame.body.get(16).copied().unwrap_or(0);
                    if action == STAND {
                        // The stand-up: the new connection is freed from the ignored movement.
                        self.movement_allowed = true;
                    }
                    if action == LOGOUT {
                        // The character sits: the server starts the timed logout and stops answering the world.
                        self.logging_out = true;
                        self.save();
                    }
                    if action == STOP_LOGOUT {
                        // The timed logout is given up: the server stops it.
                        let mut data = vec![0u8; 27];
                        data[4] = STOP_LOGOUT;
                        out.push(to_client(self.character_id, ACTION, (n3::CHARACTER, self.character_id), data));
                    }
                }
                N3_TELEPORT if frame.source == self.character_id => {
                    if let Some(t) = n3::teleport(&m) {
                        self.arrived = t.position;
                        if !t.within_playfield() {
                            // A zoning: the playfield the character is sent to.
                            self.playfield = t.playfield.1;
                            self.hero.playfield = self.playfield;
                            self.ready_sent = true;
                            self.world_entry(&mut out);
                            self.save();
                        }
                    }
                }
                GENERIC_CMD if frame.source == self.character_id => self.generic_command(&m.data, &mut out),
                TRADE if frame.source == self.character_id => self.trade(&m.data, &mut out),
                QUEST_ALTERNATIVE if frame.source == self.character_id => self.offers(&m.data, &mut out),
                CREATE_QUEST if frame.source == self.character_id => self.accept(&m.data, &mut out),
                _ => {}
            }
        }
        (self.ready_sent, out)
    }

    /// A line the character typed: a game-master command (`/gm`), which is answered in the chat.
    fn typed(&mut self, text: &str, out: &mut Vec<Frame>) {
        use crate::gm::Command;
        let Some(command) = crate::gm::parse(text) else { return };
        match command {
            Command::Item { template, count, quality } => match self.hero.add_item(template, count, quality) {
                Some(slot) => {
                    out.push(chat_line(self.character_id, &format!("You receive item {template:#x} x{count} (slot {slot:#x}).")));
                    self.items_changed(out);
                    self.save();
                }
                None => out.push(chat_line(self.character_id, "Your backpack is full.")),
            },
            Command::Xp(amount) => {
                let up = self.hero.grant_xp(amount);
                out.push(chat_line(self.character_id, &format!("You receive {amount} xp.")));
                if up {
                    out.push(chat_line(self.character_id, &format!("You are now level {}.", self.hero.level)));
                }
                self.stats_changed(out);
                self.save();
            }
            Command::Credits(amount) => {
                self.hero.credits = self.hero.credits.saturating_add(amount);
                out.push(chat_line(self.character_id, &format!("You receive {amount} credits.")));
                self.stats_changed(out);
                self.save();
            }
            Command::Complete(n) => match (n >= 1 && n <= self.hero.missions.len()).then_some(n - 1) {
                Some(at) => {
                    let done = self.hero.missions.remove(at);
                    let up = self.hero.grant_xp(done.xp as u64);
                    self.hero.credits = self.hero.credits.saturating_add(done.credits);
                    if let Ok(m) = crate::missions::remove(self.character_id, done.mission) {
                        out.push(to_client(self.character_id, m.kind, m.identity, m.data));
                    }
                    out.push(chat_line(self.character_id, &format!("You complete {} and receive {} xp and {} credits.", done.title, done.xp, done.credits)));
                    if up {
                        out.push(chat_line(self.character_id, &format!("You are now level {}.", self.hero.level)));
                    }
                    self.stats_changed(out);
                    self.save();
                }
                None => out.push(chat_line(self.character_id, "You have no mission at that place in the log.")),
            },
            Command::Unknown(line) => out.push(chat_line(self.character_id, &format!("Usage: /gm item <template> [count] [quality] | /gm xp <amount> | /gm credits <amount> (got {line})"))),
        }
    }

    /// `GenericCmd`: using the vending machine opens it (its stock, and a trade naming it).
    fn generic_command(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        let Ok(parsed) = Schema::global().parse_n3(GENERIC_CMD, data) else { return };
        let target = parsed.value.get("Target").map(|t| (field_int(t, "Type") as u32, field_int(t, "Instance") as u32));
        let action = parsed.value.get("Action").and_then(Value::as_int);
        if action == Some(USE) && target == Some(crate::shop::MACHINE) {
            if let Ok(m) = crate::shop::shop_update() {
                out.push(to_client(self.character_id, m.kind, m.identity, m.data));
            }
            if let Ok(m) = crate::shop::open(self.character_id) {
                out.push(to_client(self.character_id, m.kind, m.identity, m.data));
            }
        }
    }

    /// `QuestAlternative` from the character: the terminal's offers, answered with the request's sliders, scope and terminal.
    fn offers(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        let Ok(parsed) = Schema::global().parse_n3(QUEST_ALTERNATIVE, data) else { return };
        if let Ok(m) = crate::missions::offers(self.character_id, &parsed.value) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
    }

    /// `CreateQuest` from the character: takes the offered mission (it is added to the log, which is sent again).
    fn accept(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        let Ok(parsed) = Schema::global().parse_n3(CREATE_QUEST, data) else { return };
        let Some(v) = parsed.value.get("MissionId") else { return };
        let mission = (field_int(v, "Type") as u32, field_int(v, "Instance") as u32);
        let Some(offer) = crate::missions::offer_of(mission) else {
            out.push(chat_line(self.character_id, "That mission is not on offer."));
            return;
        };
        if self.hero.missions.iter().any(|m| m.mission == mission) {
            out.push(chat_line(self.character_id, "You already have that mission."));
            return;
        }
        if self.hero.missions.len() >= MAX_MISSIONS {
            out.push(chat_line(self.character_id, "You have too many missions."));
            return;
        }
        self.hero.missions.push(crate::missions::Taken::of(&offer));
        if let Ok(m) = crate::missions::log(self.character_id, &self.hero) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
        out.push(chat_line(self.character_id, &format!("You accept {}.", offer.title)));
        self.save();
    }

    /// `Trade`: a purchase from the machine (action `AddItem`, the machine named, the buy marker, the stock slot).
    fn trade(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        let Ok(parsed) = Schema::global().parse_n3(TRADE, data) else { return };
        let v = &parsed.value;
        let action = v.get("Action").and_then(Value::as_int);
        let named = (field_int(v, "Param1") as u32, field_int(v, "Param2") as u32);
        let marked = field_int(v, "Param3") as u32 == crate::shop::BUY_MARKER;
        if action != Some(crate::shop::TRADE_ADD_ITEM) || named != crate::shop::MACHINE || !marked {
            return;
        }
        let index = field_int(v, "Param4");
        let stock = crate::shop::stock();
        let Some(item) = usize::try_from(index).ok().and_then(|i| stock.get(i)).copied() else {
            out.push(chat_line(self.character_id, "That item is not for sale."));
            return;
        };
        if self.hero.credits < crate::shop::PRICE {
            out.push(chat_line(self.character_id, "You cannot afford that."));
            return;
        }
        if self.hero.free_slot().is_none() && !self.hero.items.iter().any(|i| i.template == item.low && i.quality == item.quality) {
            out.push(chat_line(self.character_id, "Your backpack is full."));
            return;
        }
        let _ = self.hero.spend(crate::shop::PRICE);
        self.hero.add_item(item.low, 1, item.quality);
        self.items_changed(out);
        self.stats_changed(out);
        if let Ok(m) = crate::shop::trade(self.character_id, crate::shop::TRADE_COMPLETE, [crate::shop::MACHINE.0 as i64, crate::shop::MACHINE.1 as i64, crate::shop::BUY_MARKER as i64, index]) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> GameSession {
        GameSession::new(0x7b1a0021, Hero::new(0x7b1a0021), None)
    }

    fn ready() -> Frame {
        n3::ready(0x7b1a0021)
    }

    #[test]
    fn the_world_entry_carries_the_stats_and_the_items() {
        let mut s = session();
        let (ready, out) = s.handle(&ready());
        assert!(ready);
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert!(kinds.contains(&0x2b333d6e), "stats: {kinds:x?}");
        assert!(kinds.contains(&0x4e536976), "inventory: {kinds:x?}");
    }

    #[test]
    fn a_gm_line_gives_experience_and_a_level() {
        let mut s = session();
        let (_, out) = s.handle(&typed_frame("/gm xp 100000"));
        assert!(s.hero.level > 1, "level {}", s.hero.level);
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert!(kinds.contains(&0x2b333d6e), "the stats changed: {kinds:x?}");
        assert!(kinds.contains(&CHAT_TEXT));
    }

    #[test]
    fn a_gm_item_goes_in_the_backpack() {
        let mut s = session();
        let (_, out) = s.handle(&typed_frame("/gm item 156020 2 5"));
        assert_eq!(s.hero.items.len(), 1);
        assert_eq!((s.hero.items[0].template, s.hero.items[0].count), (156020, 2));
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert!(kinds.contains(&0x4e536976));
    }

    #[test]
    fn using_the_machine_opens_it_with_its_stock() {
        let mut s = session();
        let use_machine = generic_use(crate::shop::MACHINE);
        let (_, out) = s.handle(&use_machine);
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert_eq!(kinds, vec![0x58362220, TRADE]);
    }

    #[test]
    fn buying_takes_credits_and_gives_the_item() {
        let mut s = session();
        s.hero.credits = 200;
        let (_, out) = s.handle(&buy_frame(0));
        assert_eq!(s.hero.credits, 200 - crate::shop::PRICE);
        assert_eq!(s.hero.items.len(), 1);
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert!(kinds.contains(&0x4e536976) && kinds.contains(&0x2b333d6e), "{kinds:x?}");
    }

    #[test]
    fn buying_without_the_credits_gives_nothing() {
        let mut s = session();
        let (_, out) = s.handle(&buy_frame(0));
        assert!(s.hero.items.is_empty());
        assert!(out.iter().any(|f| n3::Message::parse(f).is_some_and(|m| m.kind == CHAT_TEXT)));
    }

    /// The client's typed line, as a `TEXT` frame from the character.
    fn typed_frame(line: &str) -> Frame {
        let text = line.as_bytes();
        let mut vicinity = (text.len() as u16).to_be_bytes().to_vec();
        vicinity.extend_from_slice(text);
        vicinity.push(0);
        let mut body = 3u32.to_be_bytes().to_vec();
        body.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
        body.extend_from_slice(&(vicinity.len() as u32).to_be_bytes());
        body.extend_from_slice(&vicinity);
        Frame { kind: TEXT, source: 0x7b1a0021, destination: 2, body }
    }

    /// A `GenericCmd` Use from the character on `target`.
    fn generic_use(target: (u32, u32)) -> Frame {
        let mut data = vec![1u8];
        for word in [0u32, 1, 3, 0, n3::CHARACTER, 0x7b1a0021, target.0, target.1] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        n3::Message { kind: GENERIC_CMD, identity: (n3::CHARACTER, 0x7b1a0021), data }.frame(0x7b1a0021)
    }

    /// The client's buy of stock slot `index` from the machine.
    fn buy_frame(index: u32) -> Frame {
        let mut data = vec![1u8, 0, 0, 0, 0, 5];
        for word in [crate::shop::MACHINE.0, crate::shop::MACHINE.1, crate::shop::BUY_MARKER, index] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        n3::Message { kind: TRADE, identity: (n3::CHARACTER, 0x7b1a0021), data }.frame(0x7b1a0021)
    }

    #[test]
    fn a_terminal_offers_missions_and_taking_one_logs_it() {
        let mut s = session();
        let request = n3::quest_alternative(0x7b1a0021, [2, 20, 161, 30, 184, 28, 0], 5, (0xdac1, 0xc000021c));
        let (_, out) = s.handle(&request);
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert_eq!(kinds, vec![0x5c436609], "the offers");
        let (_, out) = s.handle(&n3::create_quest(0x7b1a0021, (crate::missions::MISSION_TYPE, 2)));
        assert_eq!(s.hero.missions.len(), 1);
        assert_eq!(s.hero.missions[0].mission, (crate::missions::MISSION_TYPE, 2));
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert!(kinds.contains(&0x465a4061) && kinds.contains(&CHAT_TEXT), "{kinds:x?}");
        // (taking it again is refused)
        let (_, _) = s.handle(&n3::create_quest(0x7b1a0021, (crate::missions::MISSION_TYPE, 2)));
        assert_eq!(s.hero.missions.len(), 1);
    }

    #[test]
    fn completing_a_mission_pays_and_removes_it() {
        let mut s = session();
        s.hero.missions.push(crate::missions::Taken::of(&crate::missions::OFFERS[0]));
        let (_, out) = s.handle(&typed_frame("/gm complete 1"));
        assert!(s.hero.missions.is_empty());
        assert_eq!(s.hero.credits, crate::missions::OFFERS[0].credits);
        assert_eq!(s.hero.xp, crate::missions::OFFERS[0].xp as u64);
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert!(kinds.contains(&0x212c487a), "the mission is removed from the log: {kinds:x?}");
    }
}
