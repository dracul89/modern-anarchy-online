//! The game's timers and their bars: Gamecode's timer list (the character's +0x1e8, made by FUN_10058b83 → FUN_1003d334)
//! and GUI.dll's `TimerSystemModule_t`, which shows the list's entries as bars.
//!
//! A `TimerEntry_t` (0x1c bytes, FUN_100666ab: a `SignalBase_c` first, so the entry can tell its owner when it runs out) keeps
//! the time left (+4, a float in hundredths of a second), two times (+8 the first phase's, +0xc the second's, ints), flags
//! (+0x10), its kind (+0x14: 1 attack, 2 special, 3 nano, 4 item, 5 reload, 6 equip) and its place in the list (+0x18). The
//! flags: 1 the first phase runs (else the second, the recharge), 2 the entry takes part in the list's turns, 4 held (its
//! first phase is over and the second waits), 8 never held, 0x10 dropped when its phases are over, 0x20 may run whatever the
//! character's Features stat says.
//!
//! The list (FUN_1003d334) keeps its owner (+0), the running entry (+4) and the running special (+8), and the entries'
//! smart pointers (+0xc..+0x10). FUN_1003d8c3 (through FUN_1003d9e0, which makes a new entry) puts an entry in the first empty
//! place, starts its first phase and signals the interface (`GlobalSignals_c` +0x1f8, FUN_1003da6d) unless it is a nano;
//! FUN_1003d37a takes it out and signals the end (+0x1fc) with (kind, place). FUN_1003d517, run every frame, moves them on:
//! see [`TimerList::tick`].
//!
//! GUI.dll: `TimerSystemModule_t::InitialiseMessage` (0x100522c8) connects `StartTimerBar` (0x1005214f) to +0x1f8 and
//! `StopTimerbarMessage` (0x10051fd8) to +0x1fc. `StartTimerBar` keeps one bar per (kind, place) (`GetTimer`), makes a
//! `GameTimerBar_c` (`CreateTimer` 0x10052083, FUN_10051c6a) in the first free of 20 places (`FindNextFreePos`) with the
//! kind's label and colour; the bar follows its entry through a weak pointer (FUN_10051cc4: the first phase fills,
//! `1 - left / first`, the second empties, `left / second`) and goes when the entry is gone (`FrameProcess` 0x10051f8b).
//! The signals go only for the client's own character (+0x140), and an attack's not while the character's attack part is
//! stopped (+0x1d4 → +0x44 = 1, set by `StopFight`).

/// The flags of an entry (+0x10).
pub const RUNNING: u32 = 1;
pub const TAKES_TURNS: u32 = 2;
pub const HELD: u32 = 4;
pub const NEVER_HELD: u32 = 8;
pub const ONCE: u32 = 0x10;
pub const FREE: u32 = 0x20;

/// The kinds (+0x14).
pub const ATTACK: u32 = 1;
pub const SPECIAL: u32 = 2;
pub const NANO: u32 = 3;
pub const ITEM: u32 = 4;
pub const RELOAD: u32 = 5;

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// +4: the time left of the phase, in hundredths of a second.
    pub left: f32,
    /// +8, +0xc: the first and the second phase's times, in hundredths.
    pub first: i32,
    pub second: i32,
    pub flags: u32,
    pub kind: u32,
    /// What the entry is for (a weapon's place, a nano): the client's own key, not the original's.
    pub tag: u32,
    /// Which entry this is (the original's weak pointers tell a new entry in an old place from the old one).
    pub serial: u64,
}

/// What the list told the interface and the entries' owners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// +0x1f8: show the bar of the entry in this place.
    Start(usize),
    /// +0x1fc: take away the bars of a kind, of one place or (`None`, the original's -1) all.
    Stop(u32, Option<usize>),
    /// The entry's own signal: its first phase ran out (kind, tag).
    Done(u32, u32),
}

#[derive(Clone, Debug, Default)]
pub struct TimerList {
    pub entries: Vec<Option<Entry>>,
    /// +4 and +8: the running entry and the running special.
    running: Option<usize>,
    special: Option<usize>,
    serials: u64,
    pub events: Vec<Event>,
    /// The character's attack part is stopped (+0x1d4 → +0x44 == 1): no attack bars are shown.
    pub attacks_stopped: bool,
}

impl TimerList {
    pub fn get(&self, place: usize) -> Option<&Entry> {
        self.entries.get(place)?.as_ref()
    }

    /// The place of the entry of a kind and tag.
    pub fn find(&self, kind: u32, tag: u32) -> Option<usize> {
        self.entries.iter().position(|e| e.as_ref().is_some_and(|e| e.kind == kind && e.tag == tag))
    }

    /// FUN_1003d049: is there an entry of the kind.
    pub fn has(&self, kind: u32) -> bool {
        self.entries.iter().flatten().any(|e| e.kind == kind)
    }

    /// FUN_1003d0cb: is there an entry of the kind that is not held.
    pub fn has_unheld(&self, kind: u32) -> bool {
        self.entries.iter().flatten().any(|e| e.kind == kind && e.flags & HELD == 0)
    }

    /// The signal to the interface, as FUN_1003da6d is called: never for a nano, an attack's not while attacks are stopped
    /// (FUN_1003d517's condition; FUN_1003d8c3 itself only leaves out the nano).
    fn signal(&mut self, place: usize, check_attacks: bool) {
        let Some(e) = self.get(place) else { return };
        if e.kind != NANO && !(check_attacks && e.kind == ATTACK && self.attacks_stopped) {
            self.events.push(Event::Start(place));
        }
    }

    /// FUN_1003d9e0 → FUN_1003d8c3: a new entry of a kind in the first empty place (or after the last), its first phase
    /// started, the interface told. Its place.
    pub fn add(&mut self, first: i32, second: i32, kind: u32, tag: u32) -> usize {
        let place = self.entries.iter().position(Option::is_none).unwrap_or(self.entries.len());
        self.start_at(place, first, second, kind, tag, None)
    }

    /// FUN_1003d8c3 with an entry that is already in the list (a weapon's attack entry started again): FUN_1003d86a first
    /// takes it out of its place (the end signalled), then it goes in the first empty place as a new one would. Its place.
    pub fn restart(&mut self, place: usize, first: i32, second: i32) -> usize {
        let Some(e) = self.get(place).cloned() else { return place };
        self.remove(place);
        let at = self.entries.iter().position(Option::is_none).unwrap_or(self.entries.len());
        self.start_at(at, first, second, e.kind, e.tag, Some(e.serial))
    }

    fn start_at(&mut self, place: usize, first: i32, second: i32, kind: u32, tag: u32, serial: Option<u64>) -> usize {
        if place >= self.entries.len() {
            self.entries.resize(place + 1, None);
        }
        let serial = serial.unwrap_or_else(|| {
            self.serials += 1;
            self.serials
        });
        // (FUN_1003d8c3: the flags cleared to 1, the time left the first phase's, +4 as a float)
        self.entries[place] = Some(Entry { left: first as f32, first, second, flags: RUNNING, kind, tag, serial });
        self.signal(place, false);
        place
    }

    /// FUN_1003d1bb: the entry in `place` becomes the running one; the one running before (unless a special or a nano) goes
    /// back to the start of its first phase, free to run again (+4 = +8, flags 0x20 and 2, not held).
    pub fn run(&mut self, place: usize) {
        if let Some(old) = self.running.filter(|&r| r != place) {
            if let Some(e) = self.entries.get_mut(old).and_then(Option::as_mut).filter(|e| e.kind != NANO && e.kind != SPECIAL) {
                e.left = e.first as f32;
                e.flags = (e.flags | FREE | TAKES_TURNS) & !HELD;
            }
        }
        self.running = Some(place);
    }

    /// FUN_1003d37a: the entry taken out, the list's end trimmed of empty places, the end signalled (not for a nano).
    pub fn remove(&mut self, place: usize) {
        let Some(e) = self.entries.get_mut(place).and_then(Option::take) else { return };
        if self.running == Some(place) {
            self.running = None;
        }
        if self.special == Some(place) {
            self.special = None;
        }
        while matches!(self.entries.last(), Some(None)) {
            self.entries.pop();
        }
        if e.kind != NANO {
            self.events.push(Event::Stop(e.kind, Some(place)));
        }
    }

    /// FUN_1003d496: every entry of a kind taken out.
    pub fn remove_kind(&mut self, kind: u32) {
        while let Some(place) = self.entries.iter().position(|e| e.as_ref().is_some_and(|e| e.kind == kind)) {
            self.remove(place);
        }
    }

    /// FUN_1003d29b: the bars of every attack entry shown again.
    pub fn signal_attacks(&mut self) {
        for place in 0..self.entries.len() {
            if self.get(place).is_some_and(|e| e.kind == ATTACK) {
                self.events.push(Event::Start(place));
            }
        }
    }

    /// FUN_1003d517: the list moved on by `seconds` (its argument times DAT_10159670, 100: hundredths). `features` is
    /// FUN_10044dd8(2), the character's Features stat (224) having bit 2: without it attacks and specials neither start nor
    /// run (a captured player's Features was 0x01000006).
    pub fn tick(&mut self, seconds: f32, features: bool) {
        let step = seconds * 100.0;
        let count = self.entries.len();
        // (the newest waiting entry starts when nothing of its sort runs)
        if self.running.is_none() || self.special.is_none() {
            for place in (0..count).rev() {
                let Some(e) = self.get(place) else { continue };
                let f = e.flags;
                if f & TAKES_TURNS == 0 || f & RUNNING == 0 || f & HELD != 0 || !(features || f & FREE != 0) {
                    continue;
                }
                if self.special.is_none() && e.kind == SPECIAL {
                    self.special = Some(place);
                } else if self.running.is_none() && e.kind != SPECIAL {
                    self.running = Some(place);
                } else {
                    continue;
                }
                self.signal(place, true);
                break;
            }
        }
        // (the running entries' first phases, and every nano's)
        let mut ran = false;
        for place in 0..count {
            let Some(e) = self.get(place) else { continue };
            if e.flags & RUNNING == 0 || !(self.running == Some(place) || e.kind == NANO || self.special == Some(place)) {
                continue;
            }
            if !features && (e.kind == ATTACK || e.kind == SPECIAL) {
                continue;
            }
            ran = true;
            let e = self.entries[place].as_mut().unwrap();
            e.left -= step;
            if e.left > 0.0 {
                continue;
            }
            if e.flags & NEVER_HELD == 0 {
                e.flags |= HELD;
            }
            e.flags &= !RUNNING;
            e.left = e.second as f32;
            let (kind, tag, flags, second) = (e.kind, e.tag, e.flags, e.second);
            if self.running == Some(place) {
                self.running = None;
            }
            if self.special == Some(place) {
                self.special = None;
            }
            self.events.push(Event::Done(kind, tag));
            if flags & ONCE == 0 || second != 0 {
                self.signal(place, true);
            } else {
                self.remove(place);
            }
        }
        // (the second phases, FUN_100665e2: taking turns, not running, time left and not held; an attack's or a special's
        // waits while a first phase ran this frame)
        for place in (0..count).rev() {
            let Some(e) = self.get(place) else { continue };
            if e.flags & TAKES_TURNS == 0 || e.flags & RUNNING != 0 || e.left <= 0.0 || e.flags & HELD != 0 {
                continue;
            }
            if (ran || !features) && (e.kind == ATTACK || e.kind == SPECIAL) {
                continue;
            }
            let e = self.entries[place].as_mut().unwrap();
            e.left -= step;
            if e.left <= 0.0 {
                e.flags |= RUNNING;
                e.left = e.first as f32;
                if e.flags & ONCE != 0 {
                    self.remove(place);
                }
            }
        }
    }
}

/// How full an entry's bar is (FUN_10051cc4): its first phase fills, its second empties.
pub fn fill(e: &Entry) -> f32 {
    let f = if e.flags & RUNNING != 0 { 1.0 - e.left / e.first as f32 } else { e.left / e.second as f32 };
    if f.is_finite() { f.clamp(0.0, 1.0) } else { 0.0 }
}

/// A bar the interface shows (`GameTimerBar_c`): its kind and entry, and its place (0..20, drawn at (40, (place + 2) × 20)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar {
    pub kind: u32,
    pub entry: usize,
    pub serial: u64,
    pub place: usize,
}

/// GUI.dll's `TimerSystemModule_t`: the bars shown.
#[derive(Clone, Debug, Default)]
pub struct TimerBars {
    pub bars: Vec<Bar>,
}

impl TimerBars {
    /// `FindNextFreePos`: the first of the 20 places no bar has (0 when all are taken, as the original).
    fn free_place(&self) -> usize {
        (0..20).find(|p| !self.bars.iter().any(|b| b.place == *p)).unwrap_or(0)
    }

    /// The list's signals taken (`StartTimerBar`, `StopTimerbarMessage`), then `FrameProcess`: a bar whose entry is gone goes.
    pub fn update(&mut self, list: &mut TimerList) {
        for event in std::mem::take(&mut list.events) {
            match event {
                Event::Start(place) => {
                    let Some(e) = list.get(place) else { continue };
                    // (an item's bar only for a phase of half a second or more: +8 while the first phase runs, else +0xc,
                    // over DAT_101adfb0 = 100 against DAT_101a9fa4 = 0.5)
                    if e.kind == ITEM && (if e.flags & RUNNING != 0 { e.first } else { e.second }) as f32 / 100.0 < 0.5 {
                        continue;
                    }
                    if !self.bars.iter().any(|b| b.kind == e.kind && b.entry == place) {
                        let at = self.free_place();
                        self.bars.push(Bar { kind: e.kind, entry: place, serial: e.serial, place: at });
                    }
                }
                Event::Stop(kind, place) => self.bars.retain(|b| !(b.kind == kind && place.is_none_or(|p| p == b.entry))),
                Event::Done(..) => {}
            }
        }
        self.bars.retain(|b| list.get(b.entry).is_some_and(|e| e.serial == b.serial));
    }

    /// The bars to draw: (place, kind, fill).
    pub fn shown(&self, list: &TimerList) -> Vec<(usize, u32, f32)> {
        self.bars.iter().filter_map(|b| Some((b.place, b.kind, fill(list.get(b.entry)?)))).collect()
    }
}

/// Gamecode FUN_1005141d: a nano's cast time in seconds for the caster — the nano's ItemDelay (294) less the caster's
/// initiative share (NanoProwessInitiative, 149: half of it up to 1200, a sixth above, `(skill - 1200) / 6 + 600`) and its
/// AggDef (51), all in hundredths; no less than 0, nor than the nano's stat 523 when it has one.
pub fn cast_seconds(nano: &dyn Fn(u32) -> Option<i64>, player: &dyn Fn(u32) -> i64) -> f32 {
    let init = player(149);
    let share = if init < 0x4b1 { init as f32 * 0.5 } else { (init - 0x4b0) as f32 / 6.0 + 600.0 };
    let delay = nano(294).unwrap_or(1234567890) as f32;
    let mut t = (delay / 100.0 - share / 100.0) - player(51) as f32 / 100.0;
    if t < 0.0 {
        t = 0.0;
    }
    if let Some(least) = nano(523).filter(|&v| v != 1234567890) {
        let least = least as f32 / 100.0;
        if t < least {
            t = least;
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A weapon's attack entry as FUN_1006872e makes it (FUN_10068331 then flag 2): the first phase fills over the attack
    /// time, then the bar holds full (flag 4) until the server's attack (FUN_1009b8ba clears it), then empties over the
    /// recharge and starts again.
    #[test]
    fn an_attack_entry_fills_holds_and_empties() {
        let mut list = TimerList::default();
        let mut bars = TimerBars::default();
        let place = list.add(150, 100, ATTACK, 6);
        list.entries[place].as_mut().unwrap().flags |= TAKES_TURNS;
        bars.update(&mut list);
        assert_eq!(bars.bars.len(), 1);
        list.tick(0.0, true);
        list.tick(0.75, true);
        assert!((fill(list.get(place).unwrap()) - 0.5).abs() < 1e-4);
        list.tick(1.0, true);
        let e = list.get(place).unwrap().clone();
        assert_eq!(e.flags & (RUNNING | HELD), HELD);
        assert!(list.events.contains(&Event::Done(ATTACK, 6)));
        assert_eq!(fill(&e), 1.0);
        // (held: nothing moves)
        list.tick(5.0, true);
        assert_eq!(fill(list.get(place).unwrap()), 1.0);
        // (the attack came: it empties)
        list.entries[place].as_mut().unwrap().flags &= !HELD;
        list.tick(0.5, true);
        assert!((fill(list.get(place).unwrap()) - 0.5).abs() < 1e-4);
        list.tick(0.6, true);
        let e = list.get(place).unwrap();
        assert_eq!(e.flags & RUNNING, RUNNING);
        assert_eq!(fill(e), 0.0);
        bars.update(&mut list);
        assert_eq!(bars.shown(&list).len(), 1);
    }

    /// The special queued by `QueuePerk` (0x1005f12e: `(delay, 0, 2)`, flags 2, 0x10, 8): it fills over the delay and goes.
    #[test]
    fn a_queued_special_fills_and_goes() {
        let mut list = TimerList::default();
        let mut bars = TimerBars::default();
        let place = list.add(200, 0, SPECIAL, 0);
        list.entries[place].as_mut().unwrap().flags |= TAKES_TURNS | ONCE | NEVER_HELD;
        list.tick(0.0, true);
        bars.update(&mut list);
        assert_eq!(bars.shown(&list), vec![(0, SPECIAL, 0.0)]);
        list.tick(1.0, true);
        bars.update(&mut list);
        assert!((bars.shown(&list)[0].2 - 0.5).abs() < 1e-4);
        list.tick(1.01, true);
        bars.update(&mut list);
        assert!(list.entries.is_empty() && bars.bars.is_empty());
        // (without Features bit 2 a special never runs)
        let mut list = TimerList::default();
        let place = list.add(200, 0, SPECIAL, 0);
        list.entries[place].as_mut().unwrap().flags |= TAKES_TURNS | ONCE | NEVER_HELD;
        list.tick(3.0, false);
        assert_eq!(list.get(place).unwrap().left, 200.0);
    }

    /// A reload (FUN_10068fd2: the weapon's stat 210, no second phase, flags 0x1a, made the running entry): it fills over
    /// the reload and goes, and the attack that ran starts its first phase again (FUN_1003d1bb).
    #[test]
    fn a_reload_fills_and_restarts_the_attack() {
        let mut list = TimerList::default();
        let attack = list.add(300, 100, ATTACK, 6);
        list.entries[attack].as_mut().unwrap().flags |= TAKES_TURNS;
        list.tick(1.0, true);
        assert_eq!(list.get(attack).unwrap().left, 200.0);
        let reload = list.add(150, 0, RELOAD, 6);
        list.entries[reload].as_mut().unwrap().flags |= 0x1a;
        list.run(reload);
        assert_eq!(list.get(attack).unwrap().left, 300.0);
        assert_ne!(list.get(attack).unwrap().flags & FREE, 0);
        list.tick(1.6, true);
        assert!(list.get(reload).is_none());
        assert!(list.events.contains(&Event::Stop(RELOAD, Some(reload))));
        // (the attack waits for its turn again, then runs)
        list.tick(0.0, true);
        list.tick(1.0, true);
        assert_eq!(list.get(attack).unwrap().left, 200.0);
    }

    /// Bars take the first free of 20 places and give it back; `StopFight`'s (1, -1) takes every attack bar.
    #[test]
    fn bars_take_free_places() {
        let mut list = TimerList::default();
        let mut bars = TimerBars::default();
        let a = list.add(100, 100, ATTACK, 6);
        let b = list.add(100, 100, ATTACK, 8);
        let n = list.add(100, 0, RELOAD, 6);
        list.signal(a, false);
        bars.update(&mut list);
        assert_eq!(bars.bars.iter().map(|b| b.place).collect::<Vec<_>>(), vec![0, 1, 2]);
        list.remove(b);
        bars.update(&mut list);
        let c = list.add(100, 0, ITEM, 0);
        bars.update(&mut list);
        assert_eq!(bars.bars.iter().find(|x| x.entry == c).unwrap().place, 1);
        list.events.push(Event::Stop(ATTACK, None));
        bars.update(&mut list);
        assert_eq!(bars.bars.iter().map(|x| x.kind).collect::<Vec<_>>(), vec![RELOAD, ITEM]);
        let _ = n;
        // (an item's bar needs half a second)
        let short = list.add(40, 0, ITEM, 1);
        bars.update(&mut list);
        assert!(!bars.bars.iter().any(|x| x.entry == short));
    }

    /// FUN_1005141d's cast time: a 3 s nano, a caster of initiative 400 (200 off) and AggDef 0 casts in 1 s; one of 1500
    /// takes (1500 − 1200) / 6 + 600 = 650 off, which floors at 0, or at the nano's stat 523.
    #[test]
    fn nano_cast_times() {
        let nano = |k: u32| match k {
            294 => Some(300),
            _ => None,
        };
        assert_eq!(cast_seconds(&nano, &|k| if k == 149 { 400 } else { 0 }), 1.0);
        assert_eq!(cast_seconds(&nano, &|k| if k == 149 { 1500 } else { 0 }), 0.0);
        let least = |k: u32| match k {
            294 => Some(300),
            523 => Some(50),
            _ => None,
        };
        assert_eq!(cast_seconds(&least, &|k| if k == 149 { 1500 } else { 0 }), 0.5);
        assert_eq!(cast_seconds(&nano, &|k| match k {
            149 => 100,
            51 => 50,
            _ => 0,
        }), 2.0);
    }
}
