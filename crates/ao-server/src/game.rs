//! The game server connection: the world's messages of one character. A session answers the client's frames with the server's:
//! the world entry (the place, the time, the stats and the items), the character's own movement and sitting, the game-master
//! commands typed as chat, the vending machines (their stock, and buying from them). Progress is saved as it changes.

use std::sync::Arc;
use std::time::{Duration, Instant};

use mao_net::frame::{Frame, N3, PING, SYSTEM, TEXT};
use mao_net::n3;
use mao_net::schema::{Schema, Value};

use crate::db::Db;
use crate::knubot::{self, Reply, Step, Talk};
use crate::mail::{self, Mail};
use crate::monsters::{self, Monster};
use crate::progress::{self, Hero};
use crate::templates::{self, identity, int};
use crate::useitem::{self, Effect};

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
/// The system message the client sends first on the game connection: it enters the world (`EnterGameServer`).
pub const ENTER_GAME_SERVER: u32 = 0x1b;
/// `Attack` (`28494070`): the character attacks the target it names.
pub const ATTACK: u32 = 0x28494070;
/// `StopFight` (`4a41203e`): the character stops fighting.
pub const STOP_FIGHT: u32 = 0x4a41203e;
/// The time between one swing and the next.
const SWING: Duration = Duration::from_secs(2);
/// `QuestAlternative` (`5c436609`): a mission terminal's request for the missions it offers.
pub const QUEST_ALTERNATIVE: u32 = 0x5c436609;
/// `CreateQuest` (`291f361b`): the character takes an offered mission.
pub const CREATE_QUEST: u32 = 0x291f361b;
/// The most missions a character can have taken at once.
pub const MAX_MISSIONS: usize = 25;
/// `GenericCmd`'s action `Use`.
const USE: i64 = 3;
/// `Mail` (`333b2867`): the inbox, a mail opened, its attachments taken, its deletion.
pub const MAIL: u32 = mail::MAIL;
/// The `Knubot` messages the client sends: the window opened, an answer, the window closed.
pub const KNUBOT_OPEN: u32 = knubot::OPEN;
pub const KNUBOT_ANSWER: u32 = knubot::ANSWER;
pub const KNUBOT_CLOSE: u32 = knubot::CLOSE;

/// The unix time now, in seconds.
fn unix_now() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as u32).unwrap_or(0)
}

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
    /// The monsters of the playfield, as the character sees them (they come with the world entry).
    monsters: Vec<Monster>,
    /// The monster the character is fighting, and when its next swing is due.
    fight: Option<Fight>,
    /// The character's name (the NPCs address it).
    pub name: String,
    /// The NPC the character is talking to, and where the talk stands.
    talk: Option<Talk>,
    /// The mission terminal last asked for offers (a taken mission names it as its giver).
    terminal: (u32, u32),
    /// The id the last mail got, so each is new.
    next_mail: u32,
}

#[derive(Clone, Copy, Debug)]
struct Fight {
    target: u32,
    next: Instant,
}

/// Whether a frame of the client's is the entry into the world (`EnterGameServer`), which turns the compression on.
pub fn is_enter(frame: &Frame) -> bool {
    frame.kind == SYSTEM && frame.body.get(..4) == Some(&ENTER_GAME_SERVER.to_be_bytes()[..])
}

/// Whether the game session acts on a frame of the client's (the trace marks the others as UNHANDLED).
pub fn answers(frame: &Frame) -> bool {
    if frame.kind == PING || frame.kind == TEXT || is_enter(frame) {
        return true;
    }
    if frame.kind != N3 {
        return false;
    }
    n3::Message::parse(frame).is_some_and(|m| matches!(m.kind, READY | GAME_TIME | POSITION_UPDATE | ACTION | N3_TELEPORT | ATTACK | STOP_FIGHT | GENERIC_CMD | TRADE | QUEST_ALTERNATIVE | CREATE_QUEST | MAIL | KNUBOT_OPEN | KNUBOT_ANSWER | KNUBOT_CLOSE))
}

/// A frame's kind and what it is called, for the trace.
pub fn describe(frame: &Frame) -> String {
    if frame.kind == SYSTEM {
        let kind = frame.body.get(..4).map(|b| u32::from_be_bytes(b.try_into().unwrap())).unwrap_or(0);
        return format!("system {kind:#x}");
    }
    match n3::Message::parse(frame) {
        Some(m) => format!("n3 {:#010x} {}", m.kind, mao_net::schema::Schema::global().n3_name(m.kind).unwrap_or("?")),
        None => format!("frame kind {:#x}", frame.kind),
    }
}

/// A server message to the character: the zone is the source, the character the destination.
fn to_client(character: u32, kind: u32, identity: (u32, u32), data: Vec<u8>) -> Frame {
    let mut frame = n3::Message { kind, identity, data }.frame(SERVER);
    frame.destination = character;
    frame
}

/// The swing of an attack on a target: the attack's record (`AttackInfo`), the character's, with the damage and a normal hit.
fn attack_info(character: u32, target: (u32, u32), damage: u32) -> Option<Frame> {
    let mut t = templates::of(0x46002f16)?;
    t.set("Amount", int(damage as i64)).set("AmmoCount", int(-1)).set("WeaponSlot", int(0)).set("Target", identity(target)).set("Unk1", int(0)).set("HitType", int(3)).set("WeaponInstance", int(0));
    let m = t.message(0x46002f16, (n3::CHARACTER, character)).ok()?;
    Some(to_client(character, m.kind, m.identity, m.data))
}

/// The damage a target took and the health it has left (`HealthDamage`, about the damaged target).
fn health_damage(character: u32, target: (u32, u32), health: u32, damage: u32) -> Option<Frame> {
    let mut t = templates::of(0x3710256c)?;
    t.set("TargetHp", int(health as i64)).set("Amount", int(damage as i64)).set("Target", identity(target)).set("Unk2", int(0));
    let m = t.message(0x3710256c, target).ok()?;
    Some(to_client(character, m.kind, m.identity, m.data))
}

/// A frame as the server sends it: the zone is its source and the character its destination (the captures' direction).
fn server_frame(character: u32, mut frame: Frame) -> Frame {
    frame.source = SERVER;
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
    /// A session for a character with its progress (`db`, when given, is where the progress is saved). A character that has
    /// never been placed starts in Old Athens, at the spawn.
    pub fn new(character_id: u32, mut hero: Hero, db: Option<Arc<Db>>) -> Self {
        if hero.playfield == 0 {
            hero.playfield = monsters::OLD_ATHENS;
        }
        if hero.position == [0.0; 3] {
            hero.position = monsters::SPAWN;
        }
        GameSession {
            character_id,
            ready_sent: false,
            playfield: hero.playfield,
            arrived: hero.position,
            clock: 0.0,
            started: Instant::now(),
            movement_allowed: false,
            logging_out: false,
            hero,
            db,
            monsters: Vec::new(),
            fight: None,
            name: "Citizen".to_string(),
            talk: None,
            terminal: (crate::missions::TERMINAL_TYPE, 0),
            next_mail: unix_now(),
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
    fn world_entry(&mut self, out: &mut Vec<Frame>) {
        self.monsters = monsters::spawns(self.playfield);
        self.fight = None;
        out.push(server_frame(self.character_id, crate::frames::game_time(SERVER, self.character_id, self.clock, 27, 0, 0)));
        out.push(server_frame(self.character_id, crate::frames::position_update(self.character_id, 0, [0.0, 0.0, 0.0, 1.0], self.arrived, 0)));
        if let Ok(m) = progress::stat_message(self.character_id, &progress::stats(&self.hero)) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
        if let Ok(m) = crate::inventory::update(self.character_id, &self.hero.items) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
        for m in &self.monsters {
            if let Some(msg) = monsters::entity(m) {
                out.push(to_client(self.character_id, msg.kind, msg.identity, msg.data));
            }
        }
        for n in knubot::in_playfield(self.playfield) {
            if let Some(msg) = monsters::npc_entity(&n) {
                out.push(to_client(self.character_id, msg.kind, msg.identity, msg.data));
            }
        }
        self.talk = None;
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
        if is_enter(frame) {
            // The original answers the entry with the chat server (the client opens its chat connection there) and the playfield.
            // (the original sends its first burst here, before the client's ready: the chat server, the playfield, the time, the
            // position, the stats, the items and the monsters; the ready that follows is the client's own)
            out.push(crate::frames::chat_server_info(SERVER, self.character_id, crate::login::CHAT_HOST, crate::login::CHAT_PORT));
            out.push(crate::frames::playfield(SERVER, self.character_id, self.playfield, self.arrived));
            self.world_entry(&mut out);
            return (self.ready_sent, out);
        }
        if frame.kind == N3 {
            let Some(m) = n3::Message::parse(frame) else { return (self.ready_sent, out) };
            match m.kind {
                READY if frame.source == self.character_id => {
                    self.ready_sent = true;
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
                            out.push(crate::frames::playfield(SERVER, self.character_id, self.playfield, self.arrived));
                            self.world_entry(&mut out);
                            self.save();
                        }
                    }
                }
                ATTACK if frame.source == self.character_id => self.attack(&m.data),
                STOP_FIGHT if frame.source == self.character_id => self.fight = None,
                GENERIC_CMD if frame.source == self.character_id => self.generic_command(&m.data, &mut out),
                TRADE if frame.source == self.character_id => self.trade(&m.data, &mut out),
                QUEST_ALTERNATIVE if frame.source == self.character_id => self.offers(&m.data, &mut out),
                CREATE_QUEST if frame.source == self.character_id => self.accept(&m.data, &mut out),
                MAIL if frame.source == self.character_id => self.mail(&m.data, &mut out),
                KNUBOT_OPEN if frame.source == self.character_id => self.talk_open(&m.data, &mut out),
                KNUBOT_ANSWER if frame.source == self.character_id => self.talk_answer(&m.data, &mut out),
                KNUBOT_CLOSE if frame.source == self.character_id => self.talk = None,
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
                Some(at) => self.complete_mission(at, out),
                None => out.push(chat_line(self.character_id, "You have no mission at that place in the log.")),
            },
            Command::Mail { template, quality, credits } => {
                self.next_mail = self.next_mail.wrapping_add(1);
                self.hero.mail.push(Mail::from_shop(self.next_mail, template, quality, credits, unix_now()));
                out.push(chat_line(self.character_id, "You have mail: a delivery waits at a mail terminal."));
                self.stats_changed(out);
                self.save();
            }
            Command::Unknown(line) => out.push(chat_line(self.character_id, &format!("Usage: /gm item <template> [count] [quality] | /gm xp <amount> | /gm credits <amount> | /gm complete <n> | /gm mail <template> [quality] [credits] (got {line})"))),
        }
    }

    /// `GenericCmd`: a Use is echoed first (the header byte 0 and the first word 1, the rest as sent: the original's answer, which the
    /// client's windows open on), then acted on: the vending machine opens with its stock, an item of the backpack is used
    /// (`useitem`); a mission terminal or a mail terminal needs nothing more (the client asks on with its own messages).
    fn generic_command(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        let Ok(parsed) = Schema::global().parse_n3(GENERIC_CMD, data) else { return };
        let target = parsed.value.get("Target").map(|t| (field_int(t, "Type") as u32, field_int(t, "Instance") as u32));
        let action = parsed.value.get("Action").and_then(Value::as_int);
        if action != Some(USE) {
            return;
        }
        let mut echo = data.to_vec();
        echo[0] = 0;
        if echo.len() >= 5 {
            echo[1..5].copy_from_slice(&1u32.to_be_bytes());
        }
        out.push(to_client(self.character_id, GENERIC_CMD, (n3::CHARACTER, self.character_id), echo));
        match target {
            Some(t) if t == crate::shop::MACHINE => {
                if let Ok(m) = crate::shop::shop_update() {
                    out.push(to_client(self.character_id, m.kind, m.identity, m.data));
                }
                if let Ok(m) = crate::shop::open(self.character_id) {
                    out.push(to_client(self.character_id, m.kind, m.identity, m.data));
                }
            }
            Some((useitem::BACKPACK, slot)) => self.use_item(slot, out),
            _ => {}
        }
    }

    /// The character used an item of the backpack: what the server knows of the item happens (`useitem`), the item is used up
    /// (`TemplateAction` Used, the slot's deletion), and the inventory and stats follow.
    fn use_item(&mut self, slot: u32, out: &mut Vec<Frame>) {
        let character = self.character_id;
        let Some(at) = self.hero.items.iter().position(|i| i.placement == slot) else { return };
        let item = self.hero.items[at].clone();
        let Some(effect) = useitem::effect(item.template) else { return };
        let push = |out: &mut Vec<Frame>, m: Result<n3::Message, String>| {
            if let Ok(m) = m {
                out.push(to_client(character, m.kind, m.identity, m.data));
            }
        };
        // (as captured: what the item gives arrives first, into a new slot, then the item itself is used up and its slot emptied)
        match effect {
            Effect::Prize { template, line } => {
                push(out, useitem::feedback(character, line));
                if self.hero.add_item(template, 1, item.quality).is_some() {
                    push(out, useitem::template_action(character, (template, template), item.quality, 1, useitem::ADDED, (useitem::INVENTORY, 0)));
                    push(out, useitem::container_add(character));
                }
            }
            Effect::VictoryPoints { points, line } => {
                push(out, useitem::feedback(character, line));
                self.hero.victory_points = self.hero.victory_points.saturating_add(points);
            }
        }
        if item.count > 1 {
            self.hero.items[at].count -= 1;
        } else {
            self.hero.items.remove(at);
        }
        push(out, useitem::template_action(character, (item.template, item.template), item.quality, 1, useitem::USED, (useitem::BACKPACK, slot)));
        if !self.hero.items.iter().any(|i| i.placement == slot) {
            let m = useitem::deleted(character, slot);
            out.push(to_client(character, m.kind, m.identity, m.data));
        }
        self.items_changed(out);
        self.stats_changed(out);
        self.save();
    }

    /// `Mail` from the character: the inbox (action 1, id 0), a mail opened (1, id), its attachments taken (3) or its deletion (5),
    /// answered as the capture shows: the list, the opened mail and the flags as they change, the item into the backpack.
    fn mail(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        let Some((action, id)) = mail::request(data) else { return };
        let character = self.character_id;
        let push = |out: &mut Vec<Frame>, m: n3::Message| out.push(to_client(character, m.kind, m.identity, m.data));
        match (action, id) {
            (1, 0) => push(out, mail::list(character, &self.hero.mail)),
            (1, id) => {
                let Some(m) = self.hero.mail.iter_mut().find(|m| m.id == id) else { return };
                m.flags |= mail::READ;
                let (opened, flags) = (mail::opened(character, m), mail::flags(character, id, m.flags));
                push(out, opened);
                push(out, flags);
                self.stats_changed(out);
                self.save();
            }
            (3, id) => {
                let Some(at) = self.hero.mail.iter().position(|m| m.id == id) else { return };
                if self.hero.mail[at].is_taken() {
                    return;
                }
                let (item, credits) = (self.hero.mail[at].item, self.hero.mail[at].credits);
                if let Some((low, high, quality)) = item {
                    if self.hero.add_item(low, 1, quality).is_none() {
                        out.push(chat_line(character, "Your backpack is full."));
                        return;
                    }
                    if let Ok(m) = useitem::template_action(character, (low, high), quality, 1, useitem::ADDED, (useitem::INVENTORY, 0)) {
                        push(out, m);
                    }
                    if let Ok(m) = useitem::container_add(character) {
                        push(out, m);
                    }
                }
                self.hero.credits = self.hero.credits.saturating_add(credits);
                self.hero.mail[at].flags |= mail::TAKEN | mail::READ;
                push(out, mail::flags(character, id, self.hero.mail[at].flags));
                self.items_changed(out);
                self.stats_changed(out);
                self.save();
            }
            (5, id) => {
                self.hero.mail.retain(|m| m.id != id);
                self.stats_changed(out);
                self.save();
            }
            _ => {}
        }
    }

    /// `KnubotOpenChatWindow`: the server echoes it, and the NPC greets (or, when the character carries its mission, completes it).
    fn talk_open(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        let Some((npc_id, _)) = knubot::request(KNUBOT_OPEN, data) else { return };
        let Some(npc) = knubot::npc(npc_id) else { return };
        if let Ok(m) = knubot::open(self.character_id, npc.id) {
            out.push(to_client(self.character_id, m.kind, m.identity, m.data));
        }
        let has_mission = self.hero.missions.iter().any(|m| m.giver == (n3::CHARACTER, npc.id));
        let reply = knubot::greet(&npc, &self.name, has_mission);
        self.reply(&npc, reply, out);
    }

    /// `KnubotAnswer`: the talk goes on from where it stands.
    fn talk_answer(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        let Some((npc_id, choice)) = knubot::request(KNUBOT_ANSWER, data) else { return };
        let Some(npc) = knubot::npc(npc_id) else { return };
        let Some(talk) = self.talk.filter(|t| t.npc == npc.id) else { return };
        let reply = knubot::answer(&npc, talk.step, choice);
        self.reply(&npc, reply, out);
    }

    /// The NPC's side of the talk: its lines and the answers, the mission it gives or completes, the window's closing.
    fn reply(&mut self, npc: &knubot::Npc, reply: Reply, out: &mut Vec<Frame>) {
        let character = self.character_id;
        let push = |out: &mut Vec<Frame>, m: Result<n3::Message, String>| {
            if let Ok(m) = m {
                out.push(to_client(character, m.kind, m.identity, m.data));
            }
        };
        match reply {
            Reply::Say { lines, answers, step } => {
                for line in &lines {
                    push(out, knubot::text(character, npc.id, line));
                }
                push(out, knubot::answers(character, npc.id, &answers));
                self.talk = Some(Talk { npc: npc.id, step });
            }
            Reply::Grant { line } => {
                push(out, knubot::text(character, npc.id, &line));
                let daily = knubot::daily(npc);
                if self.hero.missions.len() < MAX_MISSIONS && !self.hero.missions.iter().any(|m| m.mission == daily.mission) {
                    self.hero.missions.push(daily);
                    if let Ok(m) = crate::missions::log(character, &self.hero) {
                        out.push(to_client(character, m.kind, m.identity, m.data));
                    }
                    self.save();
                }
                push(out, knubot::close(character, npc.id));
                self.talk = None;
            }
            Reply::Complete { line } => {
                push(out, knubot::text(character, npc.id, &line));
                if let Some(at) = self.hero.missions.iter().position(|m| m.giver == (n3::CHARACTER, npc.id)) {
                    self.complete_mission(at, out);
                }
                push(out, knubot::answers(character, npc.id, &["Goodbye"]));
                self.talk = Some(Talk { npc: npc.id, step: Step::Farewell });
            }
            Reply::Close => {
                push(out, knubot::close(character, npc.id));
                self.talk = None;
            }
        }
    }

    /// A mission is done (the Arbiter capture's completion): the accomplished action, the log's Delete, the reward, the stats.
    fn complete_mission(&mut self, at: usize, out: &mut Vec<Frame>) {
        let done = self.hero.missions.remove(at);
        let up = self.hero.grant_xp(done.xp as u64);
        self.hero.credits = self.hero.credits.saturating_add(done.credits);
        let m = crate::missions::accomplished(self.character_id, done.mission);
        out.push(to_client(self.character_id, m.kind, m.identity, m.data));
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

    /// `Attack` from the character: a monster it names becomes the fight (its first swing is due at the next tick).
    fn attack(&mut self, data: &[u8]) {
        let Ok(parsed) = Schema::global().parse_n3(ATTACK, data) else { return };
        let Some(t) = parsed.value.get("Target") else { return };
        let target = (field_int(t, "Type") as u32, field_int(t, "Instance") as u32);
        if target.0 != n3::CHARACTER || !self.monsters.iter().any(|m| m.id == target.1 && m.health > 0) {
            return;
        }
        self.fight = Some(Fight { target: target.1, next: Instant::now() });
    }

    /// The time has come: the fight's next swing (and, when the monster is dead, its experience, its loot and its removal). `now` is
    /// the server's clock. Returns the frames to send.
    pub fn tick(&mut self, now: Instant) -> Vec<Frame> {
        let mut out = Vec::new();
        let Some(fight) = self.fight else { return out };
        if now < fight.next {
            return out;
        }
        let Some(index) = self.monsters.iter().position(|m| m.id == fight.target) else {
            self.fight = None;
            return out;
        };
        let damage = 10 + self.hero.level * 2;
        let monster = &mut self.monsters[index];
        monster.health = monster.health.saturating_sub(damage);
        let (name, health, id, xp, loot) = (monster.name.clone(), monster.health, monster.id, monster.xp, monster.loot);
        let character = self.character_id;
        let target = (n3::CHARACTER, id);
        if let Some(f) = attack_info(character, target, damage) {
            out.push(f);
        }
        if let Some(f) = health_damage(character, target, health, damage) {
            out.push(f);
        }
        out.push(chat_line(character, &format!("You hit {name} for {damage} damage.")));
        if health > 0 {
            self.fight = Some(Fight { next: now + SWING, ..fight });
            return out;
        }
        // Dead: the fight ends, the monster is gone, and the character has its experience and the loot.
        self.fight = None;
        self.monsters.remove(index);
        if let Ok(m) = templates::of(0x4a41203e).ok_or(()).and_then(|t| t.message(0x4a41203e, (n3::CHARACTER, character)).map_err(|_| ())) {
            out.push(to_client(character, m.kind, m.identity, m.data));
        }
        if let Ok(m) = templates::of(0x36510078).ok_or(()).and_then(|t| t.message(0x36510078, target).map_err(|_| ())) {
            out.push(to_client(character, m.kind, m.identity, m.data));
        }
        let up = self.hero.grant_xp(xp as u64);
        out.push(chat_line(character, &format!("You kill {name}. You receive {xp} xp.")));
        if up {
            out.push(chat_line(character, &format!("You are now level {}.", self.hero.level)));
        }
        if self.hero.add_item(loot, 1, 5).is_some() {
            out.push(chat_line(character, &format!("You receive item {loot:#x}.")));
            self.items_changed(&mut out);
        }
        self.stats_changed(&mut out);
        self.save();
        out
    }

    /// `QuestAlternative` from the character: the terminal's offers, answered with the request's sliders, scope and terminal (which is
    /// remembered: a mission taken next names it).
    fn offers(&mut self, data: &[u8], out: &mut Vec<Frame>) {
        let Ok(parsed) = Schema::global().parse_n3(QUEST_ALTERNATIVE, data) else { return };
        if let Some(t) = parsed.value.get("Terminal") {
            self.terminal = (field_int(t, "Type") as u32, field_int(t, "Instance") as u32);
        }
        if let Ok(m) = crate::missions::offers(self.character_id, &parsed.value, self.hero.level, unix_now()) {
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
        self.hero.missions.push(crate::missions::Taken::of(&offer, self.terminal));
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

    /// The client's entry into the world (the server answers with the world).
    fn enter() -> Frame {
        Frame { kind: SYSTEM, source: 1, destination: 2, body: vec![0, 0, 0, 0x1b, 0, 0, 0, 1, 1, 2, 3, 4, 5, 6, 7, 8] }
    }

    #[test]
    fn the_world_entry_carries_the_stats_and_the_items() {
        let mut s = session();
        let (_, out) = s.handle(&enter());
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
        // (the use is echoed first, as the original does)
        assert_eq!(kinds, vec![GENERIC_CMD, 0x58362220, TRADE]);
    }

    #[test]
    fn a_shop_mail_is_listed_opened_and_its_item_taken() {
        let mut s = session();
        s.handle(&typed_frame("/gm mail 156020 7 250"));
        assert_eq!(s.hero.mail.len(), 1);
        let id = s.hero.mail[0].id;
        // (the unread stat is 1 while the mail waits)
        let stats = progress::stats(&s.hero);
        assert!(stats.contains(&(mail::UNREAD_STAT, 1)));
        let (_, out) = s.handle(&n3::mail_request(0x7b1a0021));
        let list = out.iter().filter_map(|f| n3::Message::parse(f)).find(|m| m.kind == MAIL).expect("the inbox");
        let entries = mao_net::mail::parse_list(&list.data).unwrap();
        assert_eq!((entries.len(), entries[0].id, entries[0].sender.as_str()), (1, id, "Shop"));
        let (_, out) = s.handle(&mao_net::mail::open(0x7b1a0021, id));
        let opened = out.iter().filter_map(|f| n3::Message::parse(f)).filter(|m| m.kind == MAIL).find_map(|m| mao_net::mail::parse_mail(&m.data)).expect("the mail");
        assert_eq!((opened.item, opened.credits), (Some((156020, 156020, 7)), 250));
        assert!(!s.hero.mail[0].is_unread());
        let (_, out) = s.handle(&mao_net::mail::take(0x7b1a0021, id));
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert!(kinds.contains(&0x35505644) && kinds.contains(&0x47537a24) && kinds.contains(&MAIL) && kinds.contains(&0x4e536976), "{kinds:x?}");
        assert_eq!((s.hero.items.len(), s.hero.items[0].template, s.hero.credits), (1, 156020, 250));
        assert!(s.hero.mail[0].is_taken());
        s.handle(&mao_net::mail::delete(0x7b1a0021, id));
        assert!(s.hero.mail.is_empty());
        assert!(progress::stats(&s.hero).contains(&(mail::UNREAD_STAT, 0)));
    }

    #[test]
    fn a_lottery_ticket_gives_a_token_and_the_token_victory_points() {
        let mut s = session();
        s.handle(&typed_frame(&format!("/gm item {}", useitem::LOTTERY_TICKET)));
        let slot = s.hero.items[0].placement;
        let (_, out) = s.handle(&generic_use((useitem::BACKPACK, slot)));
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        // (the echo, "Congratulations!", the prize's arrival, the ticket used and gone, the inventory and the stats)
        assert_eq!(kinds[0], GENERIC_CMD);
        assert!(kinds.contains(&0x206b4b73) && kinds.contains(&0x35505644) && kinds.contains(&0x47537a24) && kinds.contains(&ACTION), "{kinds:x?}");
        assert_eq!((s.hero.items.len(), s.hero.items[0].template), (1, useitem::VICTORY_TOKEN));
        let slot = s.hero.items[0].placement;
        let (_, out) = s.handle(&generic_use((useitem::BACKPACK, slot)));
        assert!(s.hero.items.is_empty());
        assert_eq!(s.hero.victory_points, useitem::TOKEN_POINTS);
        let stats = out.iter().filter_map(|f| n3::Message::parse(f)).filter(|m| m.kind == 0x2b333d6e).count();
        assert!(stats >= 1);
        assert!(progress::stats(&s.hero).contains(&(useitem::VICTORY_POINTS_STAT, useitem::TOKEN_POINTS)));
    }

    #[test]
    fn the_receptionist_gives_the_daily_and_completes_it_on_the_next_visit() {
        let mut s = session();
        let (_, out) = s.handle(&enter());
        let npcs = out.iter().filter(|f| n3::Message::parse(f).is_some_and(|m| m.kind == 0x271b3a6b && m.identity.1 == knubot::RECEPTIONIST.id)).count();
        assert_eq!(npcs, 1, "the receptionist is in the world");
        let npc = (n3::CHARACTER, knubot::RECEPTIONIST.id);
        let (_, out) = s.handle(&n3::npc_chat_open(0x7b1a0021, npc));
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert_eq!(kinds, vec![KNUBOT_OPEN, knubot::TEXT, knubot::ANSWERS]);
        s.handle(&n3::npc_chat_answer(0x7b1a0021, npc, 0));
        let (_, out) = s.handle(&n3::npc_chat_answer(0x7b1a0021, npc, 0));
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert_eq!(kinds, vec![knubot::TEXT, 0x465a4061, KNUBOT_CLOSE], "the line, the log, the close: {kinds:x?}");
        assert_eq!(s.hero.missions.len(), 1);
        assert_eq!(s.hero.missions[0].giver, npc);
        // (back at the agency: the mission is accomplished and removed, the reward paid)
        let (_, out) = s.handle(&n3::npc_chat_open(0x7b1a0021, npc));
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert!(kinds.contains(&ACTION) && kinds.contains(&0x212c487a) && kinds.contains(&knubot::ANSWERS), "{kinds:x?}");
        assert!(s.hero.missions.is_empty());
        assert_eq!(s.hero.credits, knubot::daily(&knubot::RECEPTIONIST).credits);
        let (_, out) = s.handle(&n3::npc_chat_answer(0x7b1a0021, npc, 0));
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert_eq!(kinds, vec![KNUBOT_CLOSE]);
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
        s.hero.missions.push(crate::missions::Taken::of(&crate::missions::OFFERS[0], (crate::missions::TERMINAL_TYPE, 0xc000021c)));
        let (_, out) = s.handle(&typed_frame("/gm complete 1"));
        assert!(s.hero.missions.is_empty());
        assert_eq!(s.hero.credits, crate::missions::OFFERS[0].credits);
        assert_eq!(s.hero.xp, crate::missions::OFFERS[0].xp as u64);
        let kinds: Vec<u32> = out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)).collect();
        assert!(kinds.contains(&0x212c487a), "the mission is removed from the log: {kinds:x?}");
        let accomplished = out.iter().filter_map(|f| n3::Message::parse(f)).find(|m| m.kind == ACTION).expect("the accomplished action");
        assert_eq!(accomplished.data.get(4), Some(&0x3b));
    }

    #[test]
    fn a_new_character_starts_in_old_athens_where_the_client_reads_its_arrival() {
        let mut s = session();
        let (_, out) = s.handle(&enter());
        assert_eq!(s.playfield, monsters::OLD_ATHENS);
        let enter = Frame { kind: SYSTEM, source: 1, destination: 2, body: vec![0, 0, 0, 0x1b, 0, 0, 0, 1, 1, 2, 3, 4, 5, 6, 7, 8] };
        let (_, entered) = s.handle(&enter);
        let first = n3::Message::parse(&entered[1]).unwrap();
        let (playfield, at) = first.arrival().unwrap();
        assert_eq!(playfield, monsters::OLD_ATHENS);
        assert_eq!(at, monsters::SPAWN);
        // (the four training dummies, and the agency receptionist by the same message)
        let monsters_seen = out.iter().filter(|f| n3::Message::parse(f).is_some_and(|m| m.kind == 0x271b3a6b && m.identity.1 != knubot::RECEPTIONIST.id)).count();
        assert_eq!(monsters_seen, 4);
    }

    #[test]
    fn a_fight_is_swung_until_the_monster_dies_and_pays() {
        let mut s = session();
        s.handle(&enter());
        let target = s.monsters[0].id;
        s.handle(&n3::attack(0x7b1a0021, target));
        let mut now = Instant::now();
        let mut kinds = Vec::new();
        for _ in 0..200 {
            now += Duration::from_secs(3);
            let out = s.tick(now);
            kinds.extend(out.iter().filter_map(|f| n3::Message::parse(f).map(|m| m.kind)));
            if !s.monsters.iter().any(|m| m.id == target) {
                break;
            }
        }
        assert!(!s.monsters.iter().any(|m| m.id == target), "the monster is dead");
        assert!(kinds.contains(&0x46002f16) && kinds.contains(&0x3710256c), "the swings: {kinds:x?}");
        assert!(kinds.contains(&0x36510078), "the monster is taken away");
        assert!(s.hero.xp >= 400, "the experience: {}", s.hero.xp);
        assert!(s.hero.items.iter().any(|i| i.template == monsters::LOOT), "the loot");
    }

    #[test]
    fn an_attack_on_nothing_starts_no_fight() {
        let mut s = session();
        s.handle(&enter());
        s.handle(&n3::attack(0x7b1a0021, 0x12345));
        assert!(s.tick(Instant::now() + Duration::from_secs(10)).is_empty());
    }
}
