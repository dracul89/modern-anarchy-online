//! The damage meter: who did how much damage, to whom, with what. This client's own (the original has no such window;
//! players used programs that read the chat log's combat lines).
//!
//! It is fed by the fight's own messages as the client reads them (`net.rs`): a weapon's hit (`AttackInfo`: the amount,
//! and a critical or glancing hit by its hit type 4 or 2), a special attack's (`SpecialAttackInfo`, named by its skill),
//! a miss (`MissedAttackInfo`), a loss or gain of health with its source (`HealthDamage`: a nano program's damage, a
//! heal), and a damage or reflect shield's hit. The server tells these of everyone near, so the meter counts other
//! players, their pets and the creatures as well as the client's own character. What it cannot see is not counted: a
//! loss of health told only as a new health value has no source and is left out.
//!
//! The events fall into fights. A fight is those who hit one another: a hit belongs to the fight going on that its
//! attacker or its victim is in (or the owner or a pet of either), and begins a new fight when there is none. Two
//! fights going on at once stay apart until someone of the one hits someone of the other, which makes them one. A
//! fight ends when nothing in it has been hit for [`GAP`] seconds. So what others near by do to creatures of their own
//! neither lengthens one's own fight nor is counted in it.
//!
//! Within a fight every blow is kept by who did it, to whom and with what ([`Blow`]), with the time of the first and
//! the last. The rows are read from those: the damage done, taken or healed by each, all of it or only what passed
//! between a row and one other ([`Query::other`]: the damage done to one creature, the damage one creature did).
//!
//! A row's damage per second is its damage over the length of the fights it has a part in (a fight's first to last
//! hit, at least a second), so a player who joins late or stops early has the lower figure, as in a raid's parse. With
//! one other chosen the length is that of the blows that passed with that other: a creature's from its first hit to
//! its last.
//!
//! The fights kept are the character's from one run to the next: `damagefights.txt` in the character's folder
//! (`charprefs.rs`), written when the game ends, when the fights are reset and now and then as new fights begin, and
//! read when the character is known ([`set_own`]). The times in it are the clock's (seconds since 1970); read back
//! they lie before the run's start, so a fight of an earlier run is over whoever hits whom next. A pet's owner is kept
//! with the fight ([`Fight::pets`]), since a pet's number may be another creature's in a later run. The window's
//! choices are kept in `damage.ini` beside it.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use crate::theme::Palette;

/// The first line of the file the fights are kept in.
const HEAD: &str = "mao damage fights 1";

/// The seconds without a hit after which the next one begins a new fight.
pub const GAP: f64 = 12.0;
/// The fights kept (the one that ended longest ago goes first).
pub const KEPT: usize = 40;

/// How a blow landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Normal,
    Critical,
    Glancing,
    Miss,
}

impl Hit {
    /// `AttackInfo`'s hit type: 4 a critical hit, 2 a glancing one (the lines `Feedback_CriticalHit`, `Feedback_GlancingHit`).
    pub fn of_type(hit_type: i64) -> Hit {
        match hit_type {
            4 => Hit::Critical,
            2 => Hit::Glancing,
            _ => Hit::Normal,
        }
    }
}

/// Someone in a fight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    pub id: u32,
    pub name: String,
    pub player: bool,
}

/// One thing that happened.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    /// Who did it; none when the client does not know.
    pub attacker: Option<Actor>,
    pub victim: Actor,
    /// Points of damage, or of healing when `heal`; nothing for a miss.
    pub amount: i64,
    pub heal: bool,
    pub hit: Hit,
    /// What it was done with: "Weapon", a special attack's skill, a nano program's name, "Damage shield".
    pub source: String,
}

/// A count of blows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tally {
    pub total: i64,
    pub hits: u32,
    pub criticals: u32,
    pub glancing: u32,
    pub misses: u32,
    pub largest: i64,
}

impl Tally {
    fn add(&mut self, amount: i64, hit: Hit) {
        match hit {
            Hit::Miss => self.misses += 1,
            _ => {
                self.total += amount;
                self.hits += 1;
                self.largest = self.largest.max(amount);
                self.criticals += u32::from(hit == Hit::Critical);
                self.glancing += u32::from(hit == Hit::Glancing);
            }
        }
    }

    fn join(&mut self, other: &Tally) {
        self.total += other.total;
        self.hits += other.hits;
        self.criticals += other.criticals;
        self.glancing += other.glancing;
        self.misses += other.misses;
        self.largest = self.largest.max(other.largest);
    }

    /// The share of the swings that landed, in percent (none without a swing).
    pub fn accuracy(&self) -> Option<f32> {
        let swings = self.hits + self.misses;
        (swings > 0).then(|| 100.0 * self.hits as f32 / swings as f32)
    }

    /// The share of the hits that were critical, in percent.
    pub fn critical_share(&self) -> Option<f32> {
        (self.hits > 0).then(|| 100.0 * self.criticals as f32 / self.hits as f32)
    }

    pub fn average(&self) -> i64 {
        if self.hits == 0 { 0 } else { self.total / self.hits as i64 }
    }
}

/// What was done, by whom, to whom, with what: the key a fight counts its blows by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Blow {
    /// Who did it; 0 when the client does not know.
    pub by: u32,
    pub on: u32,
    pub heal: bool,
    pub source: String,
}

/// The blows of one kind in a fight, and when the first and the last of them fell.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Count {
    pub tally: Tally,
    pub first: f64,
    pub last: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fight {
    /// Its number, which stays its own while it is kept (the fights' places change as old ones go).
    pub id: u64,
    pub started: f64,
    pub last: f64,
    /// Those in it: the name, and whether a player.
    pub actors: BTreeMap<u32, (String, bool)>,
    /// The pets in it whose owners were known when they fought: the pet, and its owner with the owner's name.
    pub pets: BTreeMap<u32, (u32, String)>,
    pub blows: BTreeMap<Blow, Count>,
    /// What the fight is called: the first one hit that is no player, else the first one hit.
    pub label: String,
    labelled_by_creature: bool,
}

impl Fight {
    /// The fight's length in seconds, at least one.
    pub fn seconds(&self) -> f64 {
        (self.last - self.started).max(1.0)
    }

    /// Another fight's blows made this one's (someone of the one hit someone of the other).
    fn join(&mut self, other: Fight) {
        self.started = self.started.min(other.started);
        self.last = self.last.max(other.last);
        for (id, actor) in other.actors {
            self.actors.entry(id).or_insert(actor);
        }
        for (pet, owner) in other.pets {
            self.pets.entry(pet).or_insert(owner);
        }
        for (blow, count) in other.blows {
            let mine = self.blows.entry(blow).or_insert(Count { first: count.first, last: count.last, ..Default::default() });
            mine.tally.join(&count.tally);
            mine.first = mine.first.min(count.first);
            mine.last = mine.last.max(count.last);
        }
        if self.label.is_empty() || (!self.labelled_by_creature && other.labelled_by_creature) {
            self.label = other.label;
            self.labelled_by_creature = other.labelled_by_creature;
        }
    }

    /// Whether a character or a pet of its has a part in the fight.
    pub fn has(&self, own: u32) -> bool {
        self.actors.contains_key(&own) || self.pets.values().any(|p| p.0 == own)
    }

    /// The row someone's figures go to: a pet's to its owner's when pets go with their owners.
    fn row_of(&self, id: u32, merge_pets: bool) -> u32 {
        if merge_pets { self.pets.get(&id).map_or(id, |p| p.0) } else { id }
    }

    /// A row's name and whether it is a player's (a pet whose owner is known counts as one).
    fn named(&self, id: u32) -> (String, bool) {
        if id == 0 {
            return ("Not known".to_string(), false);
        }
        match self.actors.get(&id) {
            Some((name, player)) => (name.clone(), *player || self.pets.contains_key(&id)),
            // (an owner whose pet alone is in the fight)
            None => self.pets.values().find(|p| p.0 == id).map_or_else(|| ("?".to_string(), false), |p| (p.1.clone(), true)),
        }
    }

    /// The fight's times moved by so many seconds.
    fn shift(&mut self, by: f64) {
        self.started += by;
        self.last += by;
        for count in self.blows.values_mut() {
            count.first += by;
            count.last += by;
        }
    }
}

/// What the rows count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Dealt,
    Taken,
    Healed,
}

/// Which fights the rows are of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    /// The fight in which something happened last.
    #[default]
    Latest,
    /// A fight by its number ([`Fight::id`]).
    Fight(u64),
    /// All the fights kept, together.
    All,
}

/// What the rows are asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Query {
    pub view: View,
    pub mode: Mode,
    /// A pet's figures are its owner's, each of the pet's sources named for the pet.
    pub merge_pets: bool,
    /// Only the players' rows (and their pets', merged or not).
    pub players_only: bool,
    /// Only the fights this character or a pet of its has a part in (for the latest fight and for all together).
    pub mine: Option<u32>,
    /// Only what passed between a row and this one: the one hit (damage done), the one who hit (damage taken), the one
    /// healed (healing done).
    pub other: Option<u32>,
}

impl Query {
    /// A blow's two parties as the mode has them, the row's first; none when the blow is not the mode's, or nobody's.
    /// A loss of health whose source is the one who lost it (the server tells a few so: the captures have four) is
    /// damage that one took, from itself, and no damage it did.
    fn parties(&self, blow: &Blow) -> Option<(u32, u32)> {
        if blow.heal != (self.mode == Mode::Healed) || (self.mode == Mode::Dealt && blow.by == blow.on) {
            return None;
        }
        let (row, other) = if self.mode == Mode::Taken { (blow.on, blow.by) } else { (blow.by, blow.on) };
        (row != 0).then_some((row, other))
    }
}

/// A line of the window.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: u32,
    pub name: String,
    pub player: bool,
    pub value: i64,
    pub per_second: f64,
    /// Of all the rows' values, in percent.
    pub share: f32,
    pub tally: Tally,
    /// What it was done with, the largest first.
    pub sources: Vec<(String, Tally)>,
    /// The other party of the row's blows, the largest first: whom the damage was done to, who did it, who was healed.
    pub others: Vec<(u32, String, Tally)>,
}

/// A row as it is gathered.
#[derive(Default)]
struct Gathered {
    name: String,
    player: bool,
    tally: Tally,
    seconds: f64,
    sources: BTreeMap<String, Tally>,
    others: BTreeMap<u32, (String, Tally)>,
}

#[derive(Default)]
pub struct Meter {
    pub fights: VecDeque<Fight>,
    /// The pets whose owners are known: the pet, and its owner with the owner's name.
    pub pets: BTreeMap<u32, (u32, String)>,
    /// The next fight's number.
    next: u64,
}

impl Meter {
    /// Someone with their own: the owner (or the one, if nobody's pet) and all the owner's pets. A fight one of them
    /// is in is the fight of them all.
    fn kin(&self, id: u32) -> Vec<u32> {
        let owner = self.pets.get(&id).map_or(id, |p| p.0);
        let mut all = vec![owner];
        all.extend(self.pets.iter().filter(|(_, p)| p.0 == owner).map(|(&pet, _)| pet));
        all
    }

    /// Adds what happened at `at` (seconds on any clock that only goes forward).
    pub fn add(&mut self, at: f64, event: &Event) {
        let by = event.attacker.as_ref().map_or(0, |a| a.id);
        let on = event.victim.id;
        let going = |fights: &VecDeque<Fight>, ids: &[u32]| -> Vec<usize> { (0..fights.len()).filter(|&k| at - fights[k].last <= GAP && ids.iter().any(|id| fights[k].actors.contains_key(id))).collect() };
        let place = if event.heal {
            // (a heal is of the fight the one healed is in, else of the healer's; with no fight going on it is no fight)
            let mut found = going(&self.fights, &self.kin(on));
            if found.is_empty() && by != 0 {
                found = going(&self.fights, &self.kin(by));
            }
            let Some(k) = found.into_iter().max_by(|&a, &b| self.fights[a].last.total_cmp(&self.fights[b].last)) else { return };
            k
        } else {
            let mut ids = self.kin(on);
            if by != 0 {
                ids.extend(self.kin(by));
            }
            // (the fights are kept in the order they began: the first found is the oldest, and the others become its)
            match going(&self.fights, &ids).as_slice() {
                [] => {
                    if self.fights.len() >= KEPT {
                        let oldest = (0..self.fights.len()).min_by(|&a, &b| self.fights[a].last.total_cmp(&self.fights[b].last)).unwrap_or(0);
                        self.fights.remove(oldest);
                    }
                    self.fights.push_back(Fight { id: self.next, started: at, last: at, ..Default::default() });
                    self.next += 1;
                    self.fights.len() - 1
                }
                [first, rest @ ..] => {
                    for &k in rest.iter().rev() {
                        if let Some(other) = self.fights.remove(k) {
                            self.fights[*first].join(other);
                        }
                    }
                    *first
                }
            }
        };
        let fight = &mut self.fights[place];
        if !event.heal {
            fight.last = fight.last.max(at);
            if fight.label.is_empty() || (!fight.labelled_by_creature && !event.victim.player) {
                fight.label = event.victim.name.clone();
                fight.labelled_by_creature = !event.victim.player;
            }
        }
        fight.actors.entry(on).or_insert_with(|| (event.victim.name.clone(), event.victim.player));
        if let Some(a) = &event.attacker {
            fight.actors.entry(a.id).or_insert_with(|| (a.name.clone(), a.player));
        }
        // (a pet's owner, as known now, stays with the fight)
        for id in [by, on] {
            if let Some(owner) = self.pets.get(&id) {
                fight.pets.entry(id).or_insert_with(|| owner.clone());
            }
        }
        let count = fight.blows.entry(Blow { by, on, heal: event.heal, source: event.source.clone() }).or_insert(Count { first: at, last: at, ..Default::default() });
        count.tally.add(event.amount, event.hit);
        count.first = count.first.min(at);
        count.last = count.last.max(at);
    }

    /// The fights of a query.
    fn chosen(&self, q: &Query) -> Vec<&Fight> {
        let wanted = |f: &&Fight| q.mine.is_none_or(|own| f.has(own));
        match q.view {
            View::Latest => self.fights.iter().filter(wanted).max_by(|a, b| a.last.total_cmp(&b.last)).into_iter().collect(),
            View::Fight(id) => self.fights.iter().filter(|f| f.id == id).collect(),
            View::All => self.fights.iter().filter(wanted).collect(),
        }
    }

    /// The seconds of a fight a query's rows are measured over: the fight's length, or with one other chosen the
    /// time from the first to the last blow that passed with that other (nothing when none did).
    fn span(&self, fight: &Fight, q: &Query) -> f64 {
        let Some(wanted) = q.other else { return fight.seconds() };
        let passed = fight.blows.iter().filter(|(b, _)| q.parties(b).is_some_and(|(_, other)| fight.row_of(other, q.merge_pets) == wanted));
        let times = passed.fold(None, |t: Option<(f64, f64)>, (_, c)| Some(t.map_or((c.first, c.last), |(a, z)| (a.min(c.first), z.max(c.last)))));
        times.map_or(0.0, |(a, z)| (z - a).max(1.0))
    }

    /// The seconds a query's fights lasted together (see [`Meter::span`]), and what they are called.
    pub fn summary(&self, q: &Query) -> (f64, String) {
        let fights = self.chosen(q);
        let label = match (q.view, fights.as_slice()) {
            (View::All, _) => format!("{} fight{}", fights.len(), if fights.len() == 1 { "" } else { "s" }),
            (_, [f]) => f.label.clone(),
            _ => String::new(),
        };
        (fights.iter().map(|f| self.span(f, q)).sum(), label)
    }

    /// When a query's fight began, when it is of one fight.
    pub fn began(&self, q: &Query) -> Option<f64> {
        match self.chosen(q).as_slice() {
            [f] => Some(f.started),
            _ => None,
        }
    }

    /// The fights as text, to keep them for the next run: a heading, then for each fight a `fight` line (its first and
    /// last hit, whether a creature names it, its name) and its `actor`, `pet` and `blow` lines. `epoch` is the clock's
    /// time (seconds since 1970) at the meter's zero: the times written are the clock's.
    pub fn to_text(&self, epoch: f64) -> String {
        // (a name is the rest of its line)
        let rest = |s: &str| s.replace(['\n', '\r'], " ");
        let mut out = format!("{HEAD}\n");
        for f in &self.fights {
            out += &format!("fight {:.3} {:.3} {} {}\n", epoch + f.started, epoch + f.last, u8::from(f.labelled_by_creature), rest(&f.label));
            for (id, (name, player)) in &f.actors {
                out += &format!("actor {id:x} {} {}\n", u8::from(*player), rest(name));
            }
            for (pet, (owner, name)) in &f.pets {
                out += &format!("pet {pet:x} {owner:x} {}\n", rest(name));
            }
            for (b, c) in &f.blows {
                let t = &c.tally;
                out += &format!("blow {:x} {:x} {} {:.3} {:.3} {} {} {} {} {} {} {}\n", b.by, b.on, u8::from(b.heal), epoch + c.first, epoch + c.last, t.total, t.hits, t.criticals, t.glancing, t.misses, t.largest, rest(&b.source));
            }
        }
        out
    }

    /// The fights of a text ([`Meter::to_text`]) made the meter's, in place of those it has; false, and nothing changed,
    /// when the text is not such a one. `epoch` is the clock's time at this meter's zero and `now` the meter's time: the
    /// fights read are over, so where the clock went back since they were written they are moved back until the last
    /// of them ended more than [`GAP`] before `now`. A line that cannot be read is passed over.
    pub fn restore(&mut self, text: &str, epoch: f64, now: f64) -> bool {
        let mut lines = text.lines();
        if lines.next() != Some(HEAD) {
            return false;
        }
        let mut fights: Vec<Fight> = Vec::new();
        for line in lines {
            let Some((word, rest)) = line.split_once(' ') else { continue };
            let read = |fight: &mut Vec<Fight>| -> Option<()> {
                match word {
                    "fight" => {
                        let mut p = rest.splitn(4, ' ');
                        let (started, last, by_creature) = (p.next()?.parse::<f64>().ok()? - epoch, p.next()?.parse::<f64>().ok()? - epoch, p.next()? == "1");
                        fight.push(Fight { started, last: last.max(started), label: p.next().unwrap_or("").to_string(), labelled_by_creature: by_creature, ..Default::default() });
                    }
                    "actor" => {
                        let mut p = rest.splitn(3, ' ');
                        let (id, player) = (u32::from_str_radix(p.next()?, 16).ok()?, p.next()? == "1");
                        fight.last_mut()?.actors.insert(id, (p.next().unwrap_or("").to_string(), player));
                    }
                    "pet" => {
                        let mut p = rest.splitn(3, ' ');
                        let (pet, owner) = (u32::from_str_radix(p.next()?, 16).ok()?, u32::from_str_radix(p.next()?, 16).ok()?);
                        fight.last_mut()?.pets.insert(pet, (owner, p.next().unwrap_or("").to_string()));
                    }
                    "blow" => {
                        let mut p = rest.splitn(12, ' ');
                        let (by, on, heal) = (u32::from_str_radix(p.next()?, 16).ok()?, u32::from_str_radix(p.next()?, 16).ok()?, p.next()? == "1");
                        let (first, last) = (p.next()?.parse::<f64>().ok()? - epoch, p.next()?.parse::<f64>().ok()? - epoch);
                        let mut n = || p.next()?.parse::<i64>().ok();
                        let tally = Tally { total: n()?, hits: n()? as u32, criticals: n()? as u32, glancing: n()? as u32, misses: n()? as u32, largest: n()? };
                        let source = p.next().unwrap_or("").to_string();
                        fight.last_mut()?.blows.insert(Blow { by, on, heal, source }, Count { tally, first, last });
                    }
                    _ => {}
                }
                Some(())
            };
            read(&mut fights);
        }
        fights.retain(|f| !f.blows.is_empty());
        fights.sort_by(|a, b| a.started.total_cmp(&b.started));
        let over = now - GAP - 1.0;
        let late = fights.iter().map(|f| f.last).fold(f64::MIN, f64::max);
        for (k, f) in fights.iter_mut().enumerate() {
            if late > over {
                f.shift(over - late);
            }
            f.id = k as u64;
        }
        self.next = fights.len() as u64;
        self.fights = fights.into();
        true
    }

    /// The rows of a query, the largest first.
    pub fn rows(&self, q: &Query) -> Vec<Row> {
        let mut found: BTreeMap<u32, Gathered> = BTreeMap::new();
        for fight in self.chosen(q) {
            let span = self.span(fight, q);
            // (the rows that have a part in this fight: its length is theirs once)
            let mut here: Vec<u32> = Vec::new();
            for (blow, count) in &fight.blows {
                let Some((actor, other)) = q.parties(blow) else { continue };
                let (row, with) = (fight.row_of(actor, q.merge_pets), fight.row_of(other, q.merge_pets));
                if q.other.is_some_and(|o| o != with) {
                    continue;
                }
                let g = found.entry(row).or_default();
                if g.name.is_empty() {
                    (g.name, g.player) = fight.named(row);
                }
                g.tally.join(&count.tally);
                let source = if blow.source.is_empty() { "Not known" } else { blow.source.as_str() };
                // (a pet's blow in its owner's row: named for the pet)
                let source = if row != actor && actor == blow.by { format!("{}: {source}", fight.actors.get(&actor).map_or("Pet", |a| a.0.as_str())) } else { source.to_string() };
                g.sources.entry(source).or_default().join(&count.tally);
                g.others.entry(with).or_insert_with(|| (fight.named(with).0, Tally::default())).1.join(&count.tally);
                if !here.contains(&row) {
                    here.push(row);
                    g.seconds += span;
                }
            }
        }
        let largest_first = |a: &Tally, b: &Tally| b.total.cmp(&a.total);
        let mut rows: Vec<Row> = found
            .into_iter()
            .filter(|(_, g)| !q.players_only || g.player)
            .map(|(id, g)| {
                let mut sources: Vec<(String, Tally)> = g.sources.into_iter().collect();
                sources.sort_by(|a, b| largest_first(&a.1, &b.1).then_with(|| a.0.cmp(&b.0)));
                let mut others: Vec<(u32, String, Tally)> = g.others.into_iter().map(|(id, (name, t))| (id, name, t)).collect();
                others.sort_by(|a, b| largest_first(&a.2, &b.2).then_with(|| a.1.cmp(&b.1)));
                Row { id, name: g.name, player: g.player, value: g.tally.total, per_second: g.tally.total as f64 / g.seconds.max(1.0), share: 0.0, tally: g.tally, sources, others }
            })
            .filter(|r| r.value > 0 || (q.mode != Mode::Healed && r.tally.misses > 0))
            .collect();
        let all: i64 = rows.iter().map(|r| r.value).sum();
        for r in &mut rows {
            r.share = if all > 0 { 100.0 * r.value as f32 / all as f32 } else { 0.0 };
        }
        rows.sort_by(|a, b| b.value.cmp(&a.value).then_with(|| a.name.cmp(&b.name)));
        rows
    }

    /// The other parties of a query's rows, whichever is chosen, the largest first: those the damage was done to (damage
    /// done), those who did it (damage taken), those healed (healing done), each with its points.
    pub fn others(&self, q: &Query) -> Vec<(u32, String, i64)> {
        let mut all: BTreeMap<u32, (String, i64)> = BTreeMap::new();
        for row in self.rows(&Query { other: None, ..*q }) {
            for (id, name, tally) in row.others {
                all.entry(id).or_insert((name, 0)).1 += tally.total;
            }
        }
        let mut all: Vec<(u32, String, i64)> = all.into_iter().map(|(id, (name, points))| (id, name, points)).collect();
        all.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
        all
    }

    /// The rows as lines of text, to paste somewhere: a heading and "1. Name 1,234 (56.7/s, 40%)".
    pub fn report(&self, q: &Query) -> String {
        let (seconds, label) = self.summary(q);
        let what = match q.mode {
            Mode::Dealt => "Damage",
            Mode::Taken => "Damage taken",
            Mode::Healed => "Healing",
        };
        let other = q.other.and_then(|o| self.others(q).into_iter().find(|x| x.0 == o)).map_or(String::new(), |o| format!(" {} {}", q.mode.joiner(), o.1));
        let mut out = format!("{what}{other}: {label} ({})\n", clock(seconds));
        for (k, r) in self.rows(q).iter().enumerate() {
            out += &format!("{}. {} {} ({:.1}/s, {:.0}%)\n", k + 1, r.name, grouped(r.value), r.per_second, r.share);
        }
        out
    }
}

impl Mode {
    /// The word between a mode's rows and their other party: damage done "on" one, taken "from" one, healing "to" one.
    pub fn joiner(self) -> &'static str {
        match self {
            Mode::Dealt => "on",
            Mode::Taken => "from",
            Mode::Healed => "to",
        }
    }
}

/// A number with its thousands apart: 1,234,567.
pub fn grouped(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (k, c) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// Seconds as m:ss.
pub fn clock(seconds: f64) -> String {
    let s = seconds.round().max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// A count with its word: "1 hit", "3 hits".
fn counted(n: u32, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

static METER: Mutex<Option<Meter>> = Mutex::new(None);
/// The meter's zero: when it was first asked, and the clock's time then (seconds since 1970).
static STARTED: std::sync::OnceLock<(std::time::Instant, f64)> = std::sync::OnceLock::new();
/// The meter's time at which the fights were last written down.
static KEPT_AT: Mutex<f64> = Mutex::new(0.0);
/// The seconds between two writings of the fights as new ones begin.
const KEEP_EVERY: f64 = 30.0;

/// The meter's time now, and the clock's time at its zero.
fn now() -> (f64, f64) {
    let (zero, epoch) = STARTED.get_or_init(|| (std::time::Instant::now(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())));
    (zero.elapsed().as_secs_f64(), *epoch)
}

/// The meter all the threads share.
pub fn with<R>(f: impl FnOnce(&mut Meter) -> R) -> R {
    f(METER.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Meter::default))
}

/// Something happened now (the network thread's call). A fight that begins is the time to write the earlier ones
/// down, no more often than every [`KEEP_EVERY`] seconds.
pub fn record(event: Event) {
    let (at, _) = now();
    let began = with(|m| {
        let before = m.next;
        m.add(at, &event);
        m.next != before
    });
    if began && at - *KEPT_AT.lock().unwrap_or_else(|e| e.into_inner()) >= KEEP_EVERY {
        save();
    }
}

/// Where a character's fights are kept: `damagefights.txt` in its folder.
fn fights_path(character: u32) -> std::path::PathBuf {
    crate::charprefs::dir(character).join("damagefights.txt")
}

/// Writes a character's fights down (not a test's or a preview's).
fn save_as(character: u32) {
    if character == 0 || crate::charprefs::detached() {
        return;
    }
    let (at, epoch) = now();
    *KEPT_AT.lock().unwrap_or_else(|e| e.into_inner()) = at;
    // (the text is made with the meter held, the file written without it: the window asks the meter every frame)
    let text = with(|m| m.to_text(epoch));
    let path = fights_path(character);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, text) {
        eprintln!("can't keep the damage meter's fights in {}: {e}", path.display());
    }
}

/// Writes the fights of the client's own character down for its next run (the game's end, Reset, a new fight).
pub fn save() {
    save_as(own());
}

/// How long ago a time of the meter's was, in words, when it was before this run began: a fight of an earlier run.
pub fn earlier(at: f64) -> Option<String> {
    let (now, epoch) = now();
    (at < 0.0).then(|| crate::itemindex::ago((epoch + at).max(0.0) as u64, (epoch + now).max(0.0) as u64))
}

static OWN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The client's own character (its row is marked, and its fights are the ones shown first). The fights kept for it
/// are read in place of the meter's, which are written down for the character they were of.
pub fn set_own(id: u32) {
    let before = OWN.swap(id, std::sync::atomic::Ordering::Relaxed);
    if before == id || crate::charprefs::detached() {
        return;
    }
    save_as(before);
    let text = std::fs::read_to_string(fights_path(id)).unwrap_or_default();
    let (at, epoch) = now();
    with(|m| {
        m.pets.clear();
        if !m.restore(&text, epoch, at) {
            m.fights.clear();
        }
    });
}

pub fn own() -> u32 {
    OWN.load(std::sync::atomic::Ordering::Relaxed)
}

/// A pet and its owner, as the client learns of them.
pub fn pet(pet: u32, owner: u32, owner_name: &str) {
    with(|m| {
        m.pets.insert(pet, (owner, owner_name.to_string()));
    });
}

/// The height kept free under the rows for the window's buttons.
const FOOT: f32 = 48.0;

/// The window.
pub struct DamageWindow {
    pub open: bool,
    view: View,
    mode: Mode,
    merge_pets: bool,
    players_only: bool,
    /// Only the fights the client's own character (or a pet of its) has a part in.
    mine_only: bool,
    /// The one other the rows are narrowed to ([`Query::other`]).
    other: Option<u32>,
    /// The row whose sources and others are shown.
    selected: Option<u32>,
    /// What the Copy button put on the clipboard last (for the tests).
    pub copied: String,
    /// The choices as `damage.ini` has them ([`DamageWindow::choices`]); none until the file was read.
    kept: Option<String>,
}

impl Default for DamageWindow {
    fn default() -> Self {
        DamageWindow { open: false, view: View::Latest, mode: Mode::Dealt, merge_pets: true, players_only: false, mine_only: true, other: None, selected: None, copied: String::new(), kept: None }
    }
}

impl DamageWindow {
    /// The window's choices as the lines of `damage.ini`: what is counted, the latest fight or all, the three ticks.
    /// (One fight chosen from the list is not kept: the latest is shown at the next start.)
    fn choices(&self) -> String {
        let mode = match self.mode {
            Mode::Dealt => "done",
            Mode::Taken => "taken",
            Mode::Healed => "healed",
        };
        let on = |b: bool| u8::from(b);
        format!("count={mode}\nfights={}\nplayers_only={}\npets_with_owners={}\nmy_fights_only={}\n", if self.view == View::All { "all" } else { "latest" }, on(self.players_only), on(self.merge_pets), on(self.mine_only))
    }

    /// The choices of such lines taken; a line that says nothing known changes nothing.
    fn set_choices(&mut self, text: &str) {
        for (key, value) in text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim(), v.trim())) {
            match (key, value) {
                ("count", "done") => self.mode = Mode::Dealt,
                ("count", "taken") => self.mode = Mode::Taken,
                ("count", "healed") => self.mode = Mode::Healed,
                ("fights", "all") => self.view = View::All,
                ("fights", "latest") => self.view = View::Latest,
                ("players_only", "0" | "1") => self.players_only = value == "1",
                ("pets_with_owners", "0" | "1") => self.merge_pets = value == "1",
                ("my_fights_only", "0" | "1") => self.mine_only = value == "1",
                _ => {}
            }
        }
    }

    /// The choices read when the window is first shown, and written when one changes (the character's `damage.ini`;
    /// not a test's or a preview's).
    fn keep_choices(&mut self) {
        let file = !crate::charprefs::detached();
        if self.kept.is_none() {
            if let Some(text) = file.then(|| std::fs::read_to_string(crate::charprefs::file("damage.ini")).ok()).flatten() {
                self.set_choices(&text);
            }
            self.kept = Some(self.choices());
        }
        let now = self.choices();
        if self.kept.as_deref() != Some(now.as_str()) {
            if file {
                let _ = std::fs::write(crate::charprefs::file("damage.ini"), &now);
            }
            self.kept = Some(now);
        }
    }

    /// What the window asks the meter.
    fn query(&self) -> Query {
        let own = own();
        Query { view: self.view, mode: self.mode, merge_pets: self.merge_pets, players_only: self.players_only, mine: (self.mine_only && own != 0).then_some(own), other: self.other }
    }

    /// A row's sources shown (a preview's).
    pub fn select(&mut self, row: usize) {
        self.selected = with(|m| m.rows(&self.query()).get(row).map(|r| r.id));
    }

    /// A preview's other choices, by the environment: `MAO_DAMAGE_MODE=taken|healed`, `MAO_DAMAGE_VIEW=all` (all the
    /// fights), `MAO_DAMAGE_MINE=0` (others' fights too), `MAO_DAMAGE_OTHER=<n>` (the rows narrowed to the n-th other
    /// of the list), then `MAO_DAMAGE_ROW=<n>` (that row's detail).
    pub fn preview(&mut self) {
        let var = |name: &str| std::env::var(name).ok();
        match var("MAO_DAMAGE_MODE").as_deref() {
            Some("taken") => self.mode = Mode::Taken,
            Some("healed") => self.mode = Mode::Healed,
            _ => {}
        }
        if var("MAO_DAMAGE_VIEW").as_deref() == Some("all") {
            self.view = View::All;
        }
        if var("MAO_DAMAGE_MINE").as_deref() == Some("0") {
            self.mine_only = false;
        }
        // (`MAO_DAMAGE_AGO=<seconds>`: the fights as an earlier run's, that long ago)
        if let Some(ago) = var("MAO_DAMAGE_AGO").and_then(|v| v.parse::<f64>().ok()) {
            with(|m| m.fights.iter_mut().for_each(|f| f.shift(-ago - f.last)));
        }
        if let Some(n) = var("MAO_DAMAGE_OTHER").and_then(|v| v.parse::<usize>().ok()) {
            self.other = with(|m| m.others(&self.query()).get(n).map(|o| o.0));
        }
        if let Some(row) = var("MAO_DAMAGE_ROW").and_then(|v| v.parse().ok()) {
            self.select(row);
        }
    }

    /// Draws the window; the client's own character's row is marked ([`set_own`]).
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) {
        if !self.open {
            return;
        }
        self.keep_choices();
        let own = own();
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let text = egui::Color32::from_rgb(palette.text[0], palette.text[1], palette.text[2]);
        let mut q = self.query();
        let (others, fights) = with(|m| {
            // (the fights of the list: one's own alone while only those are asked for)
            let fights: Vec<(u64, String)> = m.fights.iter().filter(|f| q.mine.is_none_or(|own| f.has(own))).map(|f| (f.id, format!("{} ({}){}", if f.label.is_empty() { "?" } else { &f.label }, clock(f.seconds()), earlier(f.started).map_or(String::new(), |ago| format!(", {ago}"))))).collect();
            (m.others(&q), fights)
        });
        // (the one the rows were narrowed to is not in these fights: all of them again)
        if self.other.is_some_and(|o| !others.iter().any(|x| x.0 == o)) {
            self.other = None;
            q.other = None;
        }
        let (rows, (seconds, label), began) = with(|m| (m.rows(&q), m.summary(&q), m.began(&q)));
        // (a fight of an earlier run says how long ago it was)
        let label = began.and_then(earlier).map_or(label.clone(), |ago| format!("{label}, {ago}"));
        let mut is_open = self.open;
        let (mut reset, mut copy) = (false, false);
        crate::chrome::window(ctx, palette, "Damage", "Damage", &mut is_open, |w| w.resizable(true).default_size(egui::vec2(340.0, 400.0)).min_size(egui::vec2(260.0, 280.0)).default_pos(crate::layout::pos("Damage", egui::pos2(80.0, 200.0))), |ui| {
            ui.horizontal(|ui| {
                let shown = match self.view {
                    View::Latest => "Latest fight".to_string(),
                    View::All => "All fights".to_string(),
                    View::Fight(id) => fights.iter().find(|f| f.0 == id).map_or_else(|| "A fight".to_string(), |f| f.1.clone()),
                };
                egui::ComboBox::from_id_salt("damage-view").selected_text(shown).width(150.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.view, View::Latest, "Latest fight");
                    ui.selectable_value(&mut self.view, View::All, "All fights");
                    for (id, name) in fights.iter().rev() {
                        ui.selectable_value(&mut self.view, View::Fight(*id), name);
                    }
                });
                egui::ComboBox::from_id_salt("damage-mode")
                    .selected_text(match self.mode {
                        Mode::Dealt => "Damage done",
                        Mode::Taken => "Damage taken",
                        Mode::Healed => "Healing done",
                    })
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.mode, Mode::Dealt, "Damage done");
                        ui.selectable_value(&mut self.mode, Mode::Taken, "Damage taken");
                        ui.selectable_value(&mut self.mode, Mode::Healed, "Healing done");
                    });
            });
            ui.horizontal(|ui| {
                // (the one other the rows are narrowed to: whom the damage was done to, who did it, who was healed)
                let (joiner, everyone) = match q.mode {
                    Mode::Dealt => ("On", "On everyone"),
                    Mode::Taken => ("From", "From everyone"),
                    Mode::Healed => ("To", "To everyone"),
                };
                let shown = self.other.and_then(|o| others.iter().find(|x| x.0 == o)).map_or_else(|| everyone.to_string(), |o| format!("{joiner} {}", o.1));
                egui::ComboBox::from_id_salt("damage-other").selected_text(shown).width(150.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.other, None, everyone);
                    for (id, name, points) in &others {
                        ui.selectable_value(&mut self.other, Some(*id), format!("{name}  {}", grouped(*points)));
                    }
                });
                crate::chrome::checkbox(ui, palette, &mut self.mine_only, "My fights only").on_hover_text("Only the fights you or a pet of yours have a part in: the latest of them, and all of them together.");
            });
            ui.horizontal(|ui| {
                crate::chrome::checkbox(ui, palette, &mut self.players_only, "Players only");
                crate::chrome::checkbox(ui, palette, &mut self.merge_pets, "Pets with their owners");
            });
            let total: i64 = rows.iter().map(|r| r.value).sum();
            let nothing = match (q.mine.is_some(), self.other.is_some()) {
                (_, true) => "Nothing of that in this fight.",
                (true, false) => "You have hit nothing yet, and nothing has hit you.",
                (false, false) => "Nothing has been hit yet.",
            };
            ui.label(egui::RichText::new(if seconds > 0.0 { format!("{label}   {}   {} in all, {:.1} a second", clock(seconds), grouped(total), total as f64 / seconds) } else { nothing.to_string() }).small().color(text));
            ui.separator();
            let chosen = self.selected.and_then(|s| rows.iter().find(|r| r.id == s)).filter(|r| !r.sources.is_empty());
            // (the buttons' row below is kept free, or the window grows a little with every frame; with a row's detail
            // shown the rows have what they need and the detail the rest)
            let free = (ui.available_height() - FOOT).max(60.0);
            let room = if chosen.is_some() { (rows.len() as f32 * 22.0 + 4.0).clamp(60.0, (free - 120.0).max(60.0)) } else { free };
            crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("damage-rows").max_height(room).auto_shrink([false, false]), |ui| {
                let largest = rows.first().map_or(1, |r| r.value.max(1));
                for (k, r) in rows.iter().enumerate() {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::click());
                    let painter = ui.painter();
                    // (the bar: the row's value against the largest; one's own row in the accent)
                    let bar = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * (r.value as f32 / largest as f32).clamp(0.0, 1.0), rect.height() - 2.0));
                    let fill = if r.id == own { accent.gamma_multiply(0.55) } else if r.player { egui::Color32::from_rgba_unmultiplied(70, 110, 170, 150) } else { egui::Color32::from_rgba_unmultiplied(150, 70, 60, 150) };
                    painter.rect_filled(bar, 2.0, fill);
                    if self.selected == Some(r.id) {
                        painter.rect_stroke(rect.shrink2(egui::vec2(0.0, 1.0)), 2.0, egui::Stroke::new(1.0, accent), egui::StrokeKind::Inside);
                    }
                    let font = egui::FontId::proportional(12.0);
                    painter.text(rect.left_center() + egui::vec2(4.0, -1.0), egui::Align2::LEFT_CENTER, format!("{}. {}", k + 1, r.name), font.clone(), text);
                    painter.text(rect.right_center() + egui::vec2(-4.0, -1.0), egui::Align2::RIGHT_CENTER, format!("{}  ({:.1}/s)  {:.0}%", grouped(r.value), r.per_second, r.share), font, text);
                    if response.clicked() {
                        self.selected = if self.selected == Some(r.id) { None } else { Some(r.id) };
                    }
                    let t = &r.tally;
                    response.on_hover_text(if q.mode == Mode::Healed {
                        format!("{}; the largest {}, {} on average", counted(t.hits, "heal"), grouped(t.largest), grouped(t.average()))
                    } else {
                        format!("{}, {} missed{}; the largest {}, {} on average{}", counted(t.hits, "hit"), t.misses, t.accuracy().map_or(String::new(), |a| format!(" ({a:.0}% landed)")), grouped(t.largest), grouped(t.average()), t.critical_share().map_or(String::new(), |c| format!("; {c:.0}% critical")))
                    });
                }
            });
            if let Some(r) = chosen {
                ui.separator();
                let (with_what, whom) = match q.mode {
                    Mode::Dealt => (format!("{}: what the damage was done with", r.name), "Whom it was done to"),
                    Mode::Taken => (format!("{}: what did the damage", r.name), "Who did it"),
                    Mode::Healed => (format!("{}: what the healing was done with", r.name), "Who was healed"),
                };
                let small = |s: String| egui::RichText::new(s).small().color(text);
                let share = |t: &Tally| format!("{:.0}%", if r.value > 0 { 100.0 * t.total as f32 / r.value as f32 } else { 0.0 });
                let detail = (ui.available_height() - FOOT - 2.0).max(60.0);
                let blows = |t: &Tally| if q.mode == Mode::Healed { counted(t.hits, "heal") } else { format!("{}, {} missed", counted(t.hits, "hit"), t.misses) };
                crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().id_salt("damage-sources").max_height(detail).auto_shrink([false, false]), |ui| {
                    ui.label(egui::RichText::new(with_what).small().color(accent));
                    egui::Grid::new("damage-source-grid").num_columns(5).spacing(egui::vec2(8.0, 2.0)).show(ui, |ui| {
                        for (name, t) in &r.sources {
                            ui.label(small(name.clone()));
                            ui.label(small(grouped(t.total)));
                            ui.label(small(share(t)));
                            ui.label(small(blows(t)));
                            ui.label(small(format!("{} largest{}", grouped(t.largest), t.critical_share().filter(|c| *c > 0.0).map_or(String::new(), |c| format!(", {c:.0}% crit")))));
                            ui.end_row();
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(whom).small().color(accent));
                    egui::Grid::new("damage-other-grid").num_columns(4).spacing(egui::vec2(8.0, 2.0)).show(ui, |ui| {
                        for (id, name, t) in &r.others {
                            // (a click narrows the rows to that one, and a click on the one chosen widens them again)
                            let narrowed = self.other == Some(*id);
                            let label = ui.add(egui::Label::new(egui::RichText::new(name.clone()).small().color(if narrowed { accent } else { text })).sense(egui::Sense::click()));
                            if label.on_hover_text(if narrowed { "Show all again." } else { "Show only what passed with this one." }).clicked() {
                                self.other = if narrowed { None } else { Some(*id) };
                            }
                            ui.label(small(grouped(t.total)));
                            ui.label(small(share(t)));
                            ui.label(small(blows(t)));
                            ui.end_row();
                        }
                    });
                });
            }
            ui.horizontal(|ui| {
                reset = crate::chrome::button(ui, palette, "Reset", true).clicked();
                copy = crate::chrome::button(ui, palette, "Copy", !rows.is_empty()).on_hover_text("The rows as text, to paste into a chat line or a note.").clicked();
            });
        });
        self.open = is_open;
        self.keep_choices();
        if reset {
            with(|m| m.fights.clear());
            save();
            self.view = View::Latest;
            self.selected = None;
            self.other = None;
        }
        if copy {
            self.copied = with(|m| m.report(&q));
            ctx.copy_text(self.copied.clone());
        }
        // (a fight that is no longer kept, or has become another's: back to the latest)
        if let View::Fight(id) = self.view {
            if !with(|m| m.fights.iter().any(|f| f.id == id)) {
                self.view = View::Latest;
            }
        }
        // (the figures move while a fight goes on)
        if seconds > 0.0 {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}

/// One fight for the tests: two players, one with a pet, and a creature.
fn one_fight() -> Meter {
    let mut m = Meter::default();
    let (you, other, pet, rat) = (Actor { id: 1, name: "Ballbearing".into(), player: true }, Actor { id: 2, name: "Brasscasings".into(), player: true }, Actor { id: 3, name: "Automaton".into(), player: false }, Actor { id: 9, name: "Rollerrat".into(), player: false });
    m.pets.insert(3, (1, "Ballbearing".into()));
    m.add(0.0, &blow(&you, &rat, 120, Hit::Normal, "Weapon"));
    m.add(1.0, &blow(&other, &rat, 310, Hit::Critical, "Weapon"));
    m.add(1.5, &blow(&pet, &rat, 45, Hit::Normal, "Weapon"));
    m.add(2.0, &blow(&you, &rat, 0, Hit::Miss, "Weapon"));
    m.add(3.0, &blow(&other, &rat, 480, Hit::Normal, "Burst"));
    m.add(4.0, &blow(&rat, &you, 37, Hit::Glancing, "Weapon"));
    m.add(5.0, &blow(&you, &rat, 200, Hit::Normal, "Electrical Discharge"));
    m.add(6.0, &Event { attacker: Some(other.clone()), victim: you.clone(), amount: 150, heal: true, hit: Hit::Normal, source: "Lifeblood".into() });
    m.add(10.0, &blow(&pet, &rat, 55, Hit::Normal, "Weapon"));
    m
}

/// A hit, a miss when `hit` says so.
fn blow(by: &Actor, on: &Actor, amount: i64, hit: Hit, source: &str) -> Event {
    Event { attacker: Some(by.clone()), victim: on.clone(), amount, heal: false, hit, source: source.into() }
}

/// Fights for pictures: [`one_fight`] with a second creature in it, and two strangers' fight near by at the same time.
pub fn sample() -> Meter {
    let mut m = one_fight();
    let (you, other, rat, reet) = (Actor { id: 1, name: "Ballbearing".into(), player: true }, Actor { id: 2, name: "Brasscasings".into(), player: true }, Actor { id: 9, name: "Rollerrat".into(), player: false }, Actor { id: 10, name: "Reet".into(), player: false });
    let (far, leet) = (Actor { id: 20, name: "Gridfan".into(), player: true }, Actor { id: 21, name: "Leet".into(), player: false });
    m.add(3.0, &blow(&far, &leet, 900, Hit::Normal, "Weapon"));
    m.add(6.0, &blow(&reet, &you, 64, Hit::Normal, "Weapon"));
    m.add(7.0, &blow(&you, &reet, 160, Hit::Critical, "Weapon"));
    m.add(8.0, &blow(&other, &reet, 240, Hit::Normal, "Weapon"));
    m.add(9.0, &blow(&rat, &other, 52, Hit::Normal, "Weapon"));
    m.add(12.0, &blow(&far, &leet, 1100, Hit::Normal, "Weapon"));
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(view: View, mode: Mode, merge_pets: bool, players_only: bool) -> Query {
        Query { view, mode, merge_pets, players_only, ..Default::default() }
    }

    fn who(id: u32, name: &str, player: bool) -> Actor {
        Actor { id, name: name.into(), player }
    }

    fn values(rows: &[Row]) -> Vec<(&str, i64)> {
        rows.iter().map(|r| (r.name.as_str(), r.value)).collect()
    }

    #[test]
    fn a_fight_is_counted_by_who_did_what() {
        let m = one_fight();
        assert_eq!(m.fights.len(), 1);
        assert_eq!(m.summary(&asked(View::Latest, Mode::Dealt, true, false)), (10.0, "Rollerrat".to_string()));
        // (the pet's with its owner: 120 + 200 and the pet's 45 + 55)
        let rows = m.rows(&asked(View::Latest, Mode::Dealt, true, false));
        assert_eq!(values(&rows), vec![("Brasscasings", 790), ("Ballbearing", 420), ("Rollerrat", 37)]);
        assert!((rows[0].per_second - 79.0).abs() < 1e-9);
        assert!((rows[0].share - 100.0 * 790.0 / 1247.0).abs() < 1e-3);
        // (a miss is a swing, not a hit; the critical one counted)
        let own = &rows[1];
        assert_eq!((own.tally.hits, own.tally.misses, own.tally.largest), (4, 1, 200));
        assert_eq!(rows[0].tally.criticals, 1);
        assert_eq!(rows[0].tally.critical_share(), Some(50.0));
        assert_eq!(own.tally.accuracy(), Some(80.0));
        // (what it was done with, the largest first; the pet's named for the pet)
        let sources: Vec<(&str, i64)> = own.sources.iter().map(|s| (s.0.as_str(), s.1.total)).collect();
        assert_eq!(sources, vec![("Electrical Discharge", 200), ("Weapon", 120), ("Automaton: Weapon", 100)]);
        // (apart: the pet has its own row, and counts among the players')
        let apart = m.rows(&asked(View::Latest, Mode::Dealt, false, true));
        assert_eq!(values(&apart), vec![("Brasscasings", 790), ("Ballbearing", 320), ("Automaton", 100)]);
        // (taken and healed)
        let taken = m.rows(&asked(View::Latest, Mode::Taken, true, false));
        assert_eq!(values(&taken), vec![("Rollerrat", 1210), ("Ballbearing", 37)]);
        let healed = m.rows(&asked(View::Latest, Mode::Healed, true, false));
        assert_eq!(values(&healed), vec![("Brasscasings", 150)]);
        assert_eq!(healed[0].others.iter().map(|o| (o.1.as_str(), o.2.total)).collect::<Vec<_>>(), vec![("Ballbearing", 150)]);
        assert_eq!(healed[0].sources[0].0, "Lifeblood");
    }

    #[test]
    fn a_pause_ends_a_fight() {
        let mut m = one_fight();
        let (a, v) = (who(1, "Ballbearing", true), who(10, "Reet", false));
        // (a heal long after is no fight; a hit is the next one)
        m.add(40.0, &Event { attacker: Some(a.clone()), victim: a.clone(), amount: 80, heal: true, hit: Hit::Normal, source: "Kit".into() });
        assert_eq!(m.fights.len(), 1);
        m.add(50.0, &blow(&a, &v, 300, Hit::Normal, "Weapon"));
        m.add(50.0 + GAP, &blow(&a, &v, 100, Hit::Normal, "Weapon"));
        assert_eq!(m.fights.len(), 2);
        let latest = asked(View::Latest, Mode::Dealt, true, false);
        assert_eq!(m.summary(&latest), (GAP, "Reet".to_string()));
        assert_eq!(m.summary(&Query { view: View::Fight(m.fights[0].id), ..latest }).1, "Rollerrat");
        // (all together: the lengths added, 10 s and 12 s)
        let all = m.rows(&Query { view: View::All, ..latest });
        assert_eq!(all[0].name, "Ballbearing");
        assert_eq!(all[0].value, 820);
        assert!((all[0].per_second - 820.0 / (10.0 + GAP)).abs() < 1e-9);
        assert_eq!(m.summary(&Query { view: View::All, ..latest }).1, "2 fights");
        // (the other player was in the first fight only: its length alone is that player's)
        assert!((all[1].per_second - 79.0).abs() < 1e-9);
        // (a source the client does not know hurts all the same)
        m.add(60.0, &Event { attacker: None, victim: a.clone(), amount: 25, heal: false, hit: Hit::Normal, source: String::new() });
        let taken = m.rows(&Query { mode: Mode::Taken, ..latest });
        let hurt = taken.iter().find(|r| r.id == 1).expect("the row");
        assert_eq!(hurt.value, 25);
        assert_eq!((hurt.sources[0].0.as_str(), hurt.others[0].1.as_str()), ("Not known", "Not known"));
        // (no more than are kept, and the one that goes is the one that ended first)
        for k in 0..KEPT + 5 {
            m.add(1000.0 + 100.0 * k as f64, &blow(&a, &v, 1, Hit::Normal, "Weapon"));
        }
        assert_eq!(m.fights.len(), KEPT);
        assert!(m.fights.iter().all(|f| f.started >= 1000.0));
        let ids: Vec<u64> = m.fights.iter().map(|f| f.id).collect();
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn fights_near_one_another_stay_apart() {
        let mut m = Meter::default();
        let (me, rat) = (who(1, "Ballbearing", true), who(9, "Rollerrat", false));
        let (far, leet) = (who(20, "Gridfan", true), who(21, "Leet", false));
        m.add(0.0, &blow(&me, &rat, 100, Hit::Normal, "Weapon"));
        m.add(2.0, &blow(&far, &leet, 500, Hit::Normal, "Weapon"));
        m.add(4.0, &blow(&me, &rat, 100, Hit::Normal, "Weapon"));
        // (the stranger fights on long after one's own fight is over: it does not lengthen it)
        for k in 1..=5 {
            m.add(2.0 + 10.0 * k as f64, &blow(&far, &leet, 500, Hit::Normal, "Weapon"));
        }
        assert_eq!(m.fights.len(), 2);
        let mine = Query { mine: Some(1), ..Default::default() };
        assert_eq!(m.summary(&mine), (4.0, "Rollerrat".to_string()));
        let rows = m.rows(&mine);
        assert_eq!(values(&rows), vec![("Ballbearing", 200)]);
        assert!((rows[0].per_second - 50.0).abs() < 1e-9);
        // (without "mine" the latest is the one something happened in last)
        assert_eq!(m.summary(&Query::default()), (50.0, "Leet".to_string()));
        // (all together: each row over the fights it is in, not over the others')
        let all = m.rows(&Query { view: View::All, ..Default::default() });
        assert_eq!(values(&all), vec![("Gridfan", 3000), ("Ballbearing", 200)]);
        assert!((all[0].per_second - 60.0).abs() < 1e-9);
        assert!((all[1].per_second - 50.0).abs() < 1e-9);
        assert_eq!(values(&m.rows(&Query { view: View::All, ..mine })), vec![("Ballbearing", 200)]);
        assert_eq!(m.summary(&Query { view: View::All, ..mine }), (4.0, "1 fight".to_string()));
        // (a heal is of the fight the one healed is in, and lengthens nothing)
        m.add(53.0, &Event { attacker: Some(me.clone()), victim: far.clone(), amount: 70, heal: true, hit: Hit::Normal, source: "Kit".into() });
        assert_eq!(m.fights.len(), 2);
        assert_eq!(values(&m.rows(&Query { mode: Mode::Healed, ..Default::default() })), vec![("Ballbearing", 70)]);
        assert_eq!(m.summary(&Query::default()).0, 50.0);
    }

    #[test]
    fn a_hit_between_two_fights_makes_them_one() {
        let mut m = Meter::default();
        let (me, rat) = (who(1, "Ballbearing", true), who(9, "Rollerrat", false));
        let (far, leet) = (who(20, "Gridfan", true), who(21, "Leet", false));
        m.add(0.0, &blow(&far, &leet, 500, Hit::Normal, "Weapon"));
        m.add(1.0, &blow(&me, &rat, 100, Hit::Normal, "Weapon"));
        m.add(2.0, &blow(&me, &rat, 100, Hit::Normal, "Weapon"));
        assert_eq!(m.fights.len(), 2);
        let first = m.fights[0].id;
        // (the stranger's creature turns on one: one fight, the older one's number, both counts kept)
        m.add(5.0, &blow(&leet, &me, 30, Hit::Normal, "Weapon"));
        assert_eq!(m.fights.len(), 1);
        assert_eq!((m.fights[0].id, m.fights[0].started, m.fights[0].last), (first, 0.0, 5.0));
        assert_eq!(values(&m.rows(&Query { mine: Some(1), ..Default::default() })), vec![("Gridfan", 500), ("Ballbearing", 200), ("Leet", 30)]);
        let mine = &m.fights[0].blows[&Blow { by: 1, on: 9, heal: false, source: "Weapon".into() }];
        assert_eq!((mine.tally.hits, mine.first, mine.last), (2, 1.0, 2.0));
        // (what one did to oneself is damage taken, from oneself, and not damage done)
        m.add(6.0, &blow(&me, &me, 8, Hit::Normal, "Nano program or other"));
        assert_eq!(m.rows(&Query::default()).iter().find(|r| r.id == 1).map(|r| r.value), Some(200));
        let hurt = m.rows(&Query { mode: Mode::Taken, ..Default::default() });
        let own = hurt.iter().find(|r| r.id == 1).expect("the row");
        assert_eq!((own.value, own.others[0].1.as_str(), own.others[1].1.as_str()), (38, "Leet", "Ballbearing"));
        // (a pet's fight is its owner's: the pet on one creature, the owner on another)
        let mut m = Meter::default();
        let pet = who(3, "Automaton", false);
        m.pets.insert(3, (1, "Ballbearing".into()));
        m.add(0.0, &blow(&pet, &leet, 40, Hit::Normal, "Weapon"));
        m.add(1.0, &blow(&me, &rat, 100, Hit::Normal, "Weapon"));
        assert_eq!(m.fights.len(), 1);
        // (and a fight the pet alone is in is one's own, its row named for the owner)
        let mut m = Meter::default();
        m.pets.insert(3, (1, "Ballbearing".into()));
        m.add(0.0, &blow(&pet, &leet, 40, Hit::Normal, "Weapon"));
        let rows = m.rows(&Query { mine: Some(1), merge_pets: true, ..Default::default() });
        assert_eq!((rows[0].id, rows[0].name.as_str(), rows[0].player, rows[0].value), (1, "Ballbearing", true, 40));
        assert_eq!(rows[0].sources[0].0, "Automaton: Weapon");
    }

    #[test]
    fn the_rows_narrow_to_one_other() {
        let m = sample();
        // (the strangers' fight is another: one's own has the two creatures)
        assert_eq!(m.fights.len(), 2);
        let mine = Query { mine: Some(1), merge_pets: true, ..Default::default() };
        assert_eq!(m.summary(&mine), (10.0, "Rollerrat".to_string()));
        assert_eq!(m.others(&mine), vec![(9, "Rollerrat".to_string(), 1210), (10, "Reet".to_string(), 400), (1, "Ballbearing".to_string(), 101), (2, "Brasscasings".to_string(), 52)]);
        // (whom a row's damage was done to)
        let rows = m.rows(&mine);
        assert_eq!(values(&rows), vec![("Brasscasings", 1030), ("Ballbearing", 580), ("Rollerrat", 89), ("Reet", 64)]);
        assert_eq!(rows[1].others.iter().map(|o| (o.1.as_str(), o.2.total)).collect::<Vec<_>>(), vec![("Rollerrat", 420), ("Reet", 160)]);
        // (the damage done to one creature, over the time it was being hit: the reet from 7 s to 8 s)
        let reet = Query { other: Some(10), ..mine };
        let rows = m.rows(&reet);
        assert_eq!(values(&rows), vec![("Brasscasings", 240), ("Ballbearing", 160)]);
        assert!((rows[0].per_second - 240.0).abs() < 1e-9);
        assert_eq!(rows[1].sources.iter().map(|s| (s.0.as_str(), s.1.total)).collect::<Vec<_>>(), vec![("Weapon", 160)]);
        assert_eq!(m.summary(&reet).0, 1.0);
        assert_eq!(m.report(&reet), "Damage on Reet: Rollerrat (0:01)\n1. Brasscasings 240 (240.0/s, 60%)\n2. Ballbearing 160 (160.0/s, 40%)\n");
        // (the choice stays whole whatever is chosen)
        assert_eq!(m.others(&reet).len(), 4);
        // (who hurt one, and with what)
        let taken = m.rows(&Query { mode: Mode::Taken, ..mine });
        let own = taken.iter().find(|r| r.id == 1).expect("the row");
        assert_eq!(own.value, 101);
        assert_eq!(own.others.iter().map(|o| (o.1.as_str(), o.2.total)).collect::<Vec<_>>(), vec![("Reet", 64), ("Rollerrat", 37)]);
        // (the damage one creature did: the rat's, from 4 s to 9 s)
        let by_rat = m.rows(&Query { mode: Mode::Taken, other: Some(9), ..mine });
        assert_eq!(values(&by_rat), vec![("Brasscasings", 52), ("Ballbearing", 37)]);
        assert!((by_rat[0].per_second - 52.0 / 5.0).abs() < 1e-9);
        // (one who is not in the fights: nothing, and no time)
        let nobody = Query { other: Some(77), ..mine };
        assert!(m.rows(&nobody).is_empty());
        assert_eq!(m.summary(&nobody).0, 0.0);
        // (pets apart: the pet is an other of its own when it is hit)
        let mut m = sample();
        m.add(9.5, &blow(&who(9, "Rollerrat", false), &who(3, "Automaton", false), 20, Hit::Normal, "Weapon"));
        assert!(m.others(&Query { mode: Mode::Taken, merge_pets: false, mine: Some(1), ..Default::default() }).iter().any(|o| o.1 == "Rollerrat"));
        let apart = m.rows(&Query { mode: Mode::Taken, merge_pets: false, mine: Some(1), ..Default::default() });
        assert!(apart.iter().any(|r| r.name == "Automaton" && r.value == 20));
        let merged = m.rows(&Query { mode: Mode::Taken, merge_pets: true, mine: Some(1), ..Default::default() });
        assert_eq!(merged.iter().find(|r| r.id == 1).map(|r| r.value), Some(121));
    }

    #[test]
    fn the_fights_are_kept_as_text() {
        // (written by a run whose meter began at 1000 s of the clock, read by one that began 500 s later)
        let mut m = sample();
        m.add(9.5, &blow(&who(9, "Rollerrat", false), &who(3, "Automaton", false), 20, Hit::Normal, "Weapon"));
        m.add(9.6, &Event { attacker: None, victim: who(1, "Ballbearing", true), amount: 5, heal: false, hit: Hit::Normal, source: String::new() });
        let text = m.to_text(1000.0);
        assert!(text.starts_with("mao damage fights 1\nfight 1000.000 1010.000 1 Rollerrat\n"));
        assert!(text.contains("\nactor 3 0 Automaton\n") && text.contains("\npet 3 1 Ballbearing\n"));
        assert!(text.contains("\nblow 1 9 0 1005.000 1005.000 200 1 0 0 0 200 Electrical Discharge\n"));
        let mut read = Meter::default();
        assert!(read.restore(&text, 1500.0, 2.0));
        assert_eq!(read.fights.len(), 2);
        assert_eq!((read.fights[0].started, read.fights[0].last, read.fights[0].id, read.fights[1].id), (-500.0, -490.0, 0, 1));
        // (the same rows whatever is asked: the pet with its owner by the fight's own word, the meter knowing no pets)
        assert!(read.pets.is_empty());
        for q in [
            Query { mine: Some(1), merge_pets: true, ..Default::default() },
            Query { mode: Mode::Taken, merge_pets: false, ..Default::default() },
            Query { mode: Mode::Healed, mine: Some(1), ..Default::default() },
            Query { view: View::All, players_only: true, merge_pets: true, ..Default::default() },
            Query { other: Some(10), mine: Some(1), merge_pets: true, ..Default::default() },
        ] {
            assert_eq!(read.rows(&q), m.rows(&q));
            assert_eq!(read.summary(&q), m.summary(&q));
            assert_eq!(read.others(&q), m.others(&q));
        }
        assert_eq!(read.began(&Query { mine: Some(1), ..Default::default() }), Some(-500.0));
        assert_eq!(read.began(&Query { view: View::All, ..Default::default() }), None);
        // (written again by the second run: the same text)
        assert_eq!(read.to_text(1500.0), text);
        // (a fight read is over: the next hit by the same begins another, numbered on)
        read.add(2.0, &blow(&who(1, "Ballbearing", true), &who(9, "Rollerrat", false), 10, Hit::Normal, "Weapon"));
        assert_eq!((read.fights.len(), read.fights[2].id, read.fights[2].started), (3, 2, 2.0));
        // (the clock went back since: the fights read are moved back until they are over)
        let mut read = Meter::default();
        assert!(read.restore(&text, 900.0, 5.0));
        let late = read.fights.iter().map(|f| f.last).fold(f64::MIN, f64::max);
        assert_eq!(late, 5.0 - GAP - 1.0);
        assert_eq!(read.fights[0].seconds(), 10.0);
        read.add(5.0, &blow(&who(1, "Ballbearing", true), &who(9, "Rollerrat", false), 10, Hit::Normal, "Weapon"));
        assert_eq!(read.fights.len(), 3);
        // (not such a text: nothing changes; lines that cannot be read, and a fight without a blow, are passed over)
        assert!(!read.restore("something else\nfight 1 2 1 Rat\n", 0.0, 0.0));
        assert_eq!(read.fights.len(), 3);
        let torn = "mao damage fights 1\nblow 1 9 0 1 1 5 1 0 0 0 5 Weapon\nfight 100.000 104.000 1 Leet of the hills\nactor 1 1 Ballbearing\nactor zz 0 Leet\nblow 1 15 0 100.000 104.000 30 2 0 0 0 20 \nblow 1 15 0 100.0\nfight 300 301 0 Nobody\nactor 2 1 Brasscasings\n";
        assert!(read.restore(torn, 1000.0, 0.0));
        assert_eq!(read.fights.len(), 1);
        let f = &read.fights[0];
        assert_eq!((f.label.as_str(), f.started, f.actors.len(), f.blows.len()), ("Leet of the hills", -900.0, 1, 1));
        assert_eq!(f.blows[&Blow { by: 1, on: 0x15, heal: false, source: String::new() }].tally.total, 30);
    }

    #[test]
    fn the_windows_choices_are_kept_as_lines() {
        let mut w = DamageWindow::default();
        assert_eq!(w.choices(), "count=done\nfights=latest\nplayers_only=0\npets_with_owners=1\nmy_fights_only=1\n");
        (w.mode, w.view, w.players_only, w.merge_pets, w.mine_only) = (Mode::Taken, View::All, true, false, false);
        let text = w.choices();
        assert_eq!(text, "count=taken\nfights=all\nplayers_only=1\npets_with_owners=0\nmy_fights_only=0\n");
        let mut read = DamageWindow::default();
        read.set_choices(&text);
        assert_eq!((read.mode, read.view, read.players_only, read.merge_pets, read.mine_only), (Mode::Taken, View::All, true, false, false));
        // (one fight of the list is not kept; what the file does not say, or says wrongly, stays as it is)
        w.view = View::Fight(3);
        assert!(w.choices().contains("fights=latest\n"));
        read.set_choices("count = healed\r\nfights=7\nplayers_only=yes\nsomething=1\n");
        assert_eq!((read.mode, read.view, read.players_only), (Mode::Healed, View::All, true));
    }

    #[test]
    fn the_report_is_lines_to_paste() {
        let m = one_fight();
        let text = m.report(&asked(View::Latest, Mode::Dealt, true, true));
        assert_eq!(text, "Damage: Rollerrat (0:10)\n1. Brasscasings 790 (79.0/s, 65%)\n2. Ballbearing 420 (42.0/s, 35%)\n");
        assert_eq!(grouped(1234567), "1,234,567");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(-1000), "-1,000");
        assert_eq!(clock(125.4), "2:05");
        assert_eq!((counted(1, "hit"), counted(2, "hit"), counted(0, "heal")), ("1 hit".to_string(), "2 hits".to_string(), "0 heals".to_string()));
        assert_eq!([Hit::of_type(4), Hit::of_type(2), Hit::of_type(0)], [Hit::Critical, Hit::Glancing, Hit::Normal]);
    }
}
