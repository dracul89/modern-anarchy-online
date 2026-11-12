//! The rooms of an interior as the character walks them, after `N3.dll`'s `n3Playfield_t` and `n3RoomMonitor_t`:
//!
//! - **Doorways.** A room's doors (`mao_formats::rooms::Door`) each open into another room; a door dynel the server announces
//!   stands in one of them (`Door_t::LinkDoorToRooms`, Gamecode FUN_100800cb: `GetDoorLinkFromPos` finds the doorway under the
//!   dynel, `RegisterDoorWithRoomLink` ties the dynel to the pair of rooms).
//! - **Crossing.** A character may pass from one room to another only through an open door (`PlayfieldAnarchy_t::
//!   IsDynelRoomTransitionAllowed` 0x101228c4: the doorway's door, or the doors of a two-room way round, all open; no
//!   registered door at all forbids the crossing). The original asks this as the dynel's room-space cell changes; the client
//!   asks it of the character's own steps and holds the character in its room when the answer is no.
//! - **What is drawn.** Only the room the character is in and the rooms behind open doors: `n3RoomMonitor_t::OnCellChanged`
//!   shows the room entered, `DoorOpened` the room beyond a door that opens, and its `Run` hides a room two seconds after
//!   the door to it closes (`_DAT_1003c968`). The room left is kept the same two seconds here (the original hides it when
//!   its locality monitor loses it).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::time::{Duration, Instant};

use mao_formats::rooms::{Grid, RoomPlayfield};

/// How long a room stays shown after the door to it closes or the character leaves it (`n3RoomMonitor_t::Run`).
const LINGER: Duration = Duration::from_secs(2);

/// An interior's rooms with their grid and, for each room, its parts in the scene model.
pub struct Interior {
    pub rooms: RoomPlayfield,
    pub grid: Grid,
    pub parts: Vec<Range<usize>>,
}

impl Interior {
    /// The room the point (the server's frame, y up) is in, trying `hint` first.
    pub fn room_at(&self, p: [f32; 3], hint: Option<usize>) -> Option<usize> {
        self.rooms.room_at(&self.grid, p, hint)
    }

    /// The doorway a door dynel at `at` stands in: the two rooms it joins, the lower index first; none at a way out of the
    /// playfield or away from any doorway.
    pub fn doorway_at(&self, at: [f32; 3]) -> Option<(usize, usize)> {
        for (i, room) in self.rooms.rooms.iter().enumerate() {
            if let Some((_, door)) = room.door_at(&self.grid, at[0], at[2]) {
                let other = door.room? as usize;
                if other == i || other >= self.rooms.rooms.len() {
                    return None;
                }
                return Some((i.min(other), i.max(other)));
            }
        }
        None
    }

    /// Whether the two rooms share a doorway.
    fn joined(&self, a: usize, b: usize) -> bool {
        self.rooms.rooms.get(a).is_some_and(|r| r.doors().any(|d| d.room == Some(b as u16)))
    }
}

/// Where the character is among the rooms and what the doors let it do.
#[derive(Default)]
pub struct RoomState {
    /// The room the character is in (kept while it stands in a doorway between two).
    pub current: Option<usize>,
    /// Where the character stood when it was last in its room.
    pub last_position: [f32; 3],
    /// The door dynels (by their number) in each doorway, the rooms' lower index first.
    links: BTreeMap<(usize, usize), Vec<u8>>,
    registered: BTreeSet<u8>,
    /// Until when each room stays shown.
    shown_until: BTreeMap<usize, Instant>,
}

impl RoomState {
    /// Ties the door dynels the server has placed (`slots`: door number → id and position) to their doorways.
    pub fn register(&mut self, interior: &Interior, slots: &BTreeMap<u8, (u32, [f32; 3])>) {
        for (&n, &(_, at)) in slots {
            if self.registered.insert(n)
                && let Some(pair) = interior.doorway_at(at)
            {
                self.links.entry(pair).or_default().push(n);
            }
        }
    }

    /// The doors of the doorway between `a` and `b`.
    fn doors(&self, a: usize, b: usize) -> &[u8] {
        self.links.get(&(a.min(b), a.max(b))).map_or(&[], Vec::as_slice)
    }

    /// Whether the character may pass from room `from` to room `to` (`IsDynelRoomTransitionAllowed`): through the doorway's
    /// doors when it has any, else through a room between that has doors both ways; every door on the way must be open.
    pub fn allowed(&self, interior: &Interior, from: usize, to: usize, open: &dyn Fn(u8) -> bool) -> bool {
        if from == to {
            return true;
        }
        let direct = self.doors(from, to);
        if !direct.is_empty() {
            return direct.iter().all(|&n| open(n));
        }
        if interior.joined(from, to) {
            // (a doorway the server placed no door in: nothing to open, the original forbids the crossing)
            return false;
        }
        let rooms = interior.rooms.rooms.len();
        (0..rooms).filter(|&mid| mid != from && mid != to && interior.joined(from, mid) && interior.joined(mid, to)).any(|mid| {
            let (a, b) = (self.doors(from, mid), self.doors(mid, to));
            !a.is_empty() && !b.is_empty() && a.iter().chain(b).all(|&n| open(n))
        })
    }

    /// The rooms to draw now: the character's own, the rooms behind its open doors, and for two seconds the rooms that were
    /// shown. One flag per room; empty when the character is in no room yet.
    pub fn shown(&mut self, interior: &Interior, now: Instant, open: &dyn Fn(u8) -> bool) -> Vec<bool> {
        let Some(current) = self.current else { return Vec::new() };
        let until = now + LINGER;
        self.shown_until.insert(current, until);
        for (&(a, b), doors) in &self.links {
            if (a == current || b == current) && doors.iter().any(|&n| open(n)) {
                self.shown_until.insert(if a == current { b } else { a }, until);
            }
        }
        (0..interior.rooms.rooms.len()).map(|room| room == current || self.shown_until.get(&room).is_some_and(|&t| t > now)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mao_formats::rooms::Room;

    fn interior() -> Interior {
        // three 2 x 2 rooms in a row along x, each joined to the next by a doorway on its far x side
        let room = |x: f32, links: Vec<(u16, u16)>| Room { flags: 0, rect: [0, 0, 2, 2], position: [x, 0.0, 2.0], links, name: String::new(), water: Vec::new(), light: Vec::new(), attractors: Vec::new() };
        let rooms = vec![room(2.0, vec![(1, 1 << 2 | 1)]), room(6.0, vec![(0, 0 << 2 | 3), (2, 1 << 2 | 1)]), room(10.0, vec![(1, 0 << 2 | 3)])];
        let cells = vec![1u8; 16];
        let tables = mao_formats::dungeon::Tables::new(&[0, 1], &[], Default::default());
        let images = mao_formats::dungeon::CellImages { width: 4, height: 4, places: &cells, doors: &[], choices: None };
        let words = mao_formats::dungeon::cells(&images, &tables);
        let grid = Grid { width: 4, height: 4, cells, heights: vec![0; 16], codes: vec![0; 16], layers: 0, tiles: Vec::new(), scale: 2.0, step: 0.2, palette: Vec::new(), tables, words, type_heights: Vec::new() };
        Interior { rooms: RoomPlayfield { number: 1, name: String::new(), tilemap: 1, rooms }, grid, parts: vec![0..2, 2..4, 4..6] }
    }

    /// `MAO_GAME`: the neutral shop (playfield 1186). Its doorways have doors without a mesh: six of the seven stand in
    /// a doorway between two rooms (the seventh is the way out), none is locked (so each opens at a touch, `approach_doors`), and a doorway is crossed
    /// with its door open and not with it shut.
    #[test]
    fn a_shops_doors_stand_in_its_doorways_and_open_at_a_touch() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let (_, interior) = crate::load_room_shell_parts(&db, 1186).unwrap();
        let table = db.get(mao_formats::resource::Id { kind: mao_formats::dynels::KIND, instance: 1186 }).unwrap().unwrap();
        let slots: BTreeMap<u8, (u32, [f32; 3])> = mao_formats::dynels::parse(&table).into_iter().filter(|d| d.kind == 0xc748).map(|d| ((d.instance >> 16) as u8, (d.instance, d.position))).collect();
        assert_eq!(slots.len(), 7);
        let mut state = RoomState::default();
        state.register(&interior, &slots);
        // (six between rooms; the seventh, door 4, is the way out of the playfield and ties no two rooms)
        assert_eq!(state.links.values().map(Vec::len).sum::<usize>(), 6, "{:?}", state.links);
        assert!(interior.doorway_at(slots[&4].1).is_none());
        for &(a, b) in state.links.keys() {
            assert!(state.allowed(&interior, a, b, &|_| true) && state.allowed(&interior, b, a, &|_| true));
            assert!(!state.allowed(&interior, a, b, &|_| false));
        }
        assert_eq!(crate::approach_doors(&db, 1186).len(), 7);
    }

    #[test]
    fn doors_are_tied_to_their_doorways_and_gate_the_crossing() {
        let interior = interior();
        let mut state = RoomState::default();
        // door 7 stands between rooms 0 and 1 (at x 4, in the first row of cells: z 1), door 8 between 1 and 2 (at x 8)
        let slots = BTreeMap::from([(7u8, (1u32, [4.0f32, 0.0, 1.0])), (8, (2, [8.0, 0.0, 1.0]))]);
        state.register(&interior, &slots);
        assert_eq!(interior.doorway_at([4.0, 0.0, 1.0]), Some((0, 1)));
        assert_eq!(interior.doorway_at([4.0, 0.0, 3.0]), None, "the doorway is not two units along the wall");
        let closed = |_: u8| false;
        let only_7 = |n: u8| n == 7;
        let all = |_: u8| true;
        assert!(!state.allowed(&interior, 0, 1, &closed));
        assert!(state.allowed(&interior, 0, 1, &only_7));
        assert!(state.allowed(&interior, 1, 0, &only_7));
        // room 0 to room 2 goes through room 1: both doors must be open
        assert!(!state.allowed(&interior, 0, 2, &only_7));
        assert!(state.allowed(&interior, 0, 2, &all));
    }

    #[test]
    fn the_rooms_shown_are_the_own_one_and_those_behind_open_doors_for_a_while() {
        let interior = interior();
        let mut state = RoomState::default();
        state.register(&interior, &BTreeMap::from([(7u8, (1u32, [4.0f32, 0.0, 1.0])), (8, (2, [8.0, 0.0, 1.0]))]));
        assert!(state.shown(&interior, Instant::now(), &|_| false).is_empty());
        state.current = Some(0);
        let t0 = Instant::now();
        assert_eq!(state.shown(&interior, t0, &|_| false), vec![true, false, false]);
        assert_eq!(state.shown(&interior, t0, &|n| n == 7), vec![true, true, false]);
        // the door shut: room 1 lingers two seconds
        assert_eq!(state.shown(&interior, t0 + Duration::from_secs(1), &|_| false), vec![true, true, false]);
        assert_eq!(state.shown(&interior, t0 + Duration::from_secs(3), &|_| false), vec![true, false, false]);
        // moving on into room 1: room 0 lingers
        state.current = Some(1);
        let t1 = t0 + Duration::from_secs(4);
        assert_eq!(state.shown(&interior, t1, &|_| false), vec![true, true, false]);
        assert_eq!(state.shown(&interior, t1 + Duration::from_secs(3), &|_| false), vec![false, true, false]);
    }
}
