//! Where the items of a container stand on screen, placed by hand.
//!
//! The original keeps each item's grid cell per container, apart from the item's place on the server
//! (`InventoryView_c`'s `item_position_map`, FUN_100ccefe / FUN_100cac66: `Inventory.xml`, `Bank.xml`,
//! `Containers/Container_<type>x<instance>.xml`), and does not arrange one's own containers: an item stays in the cell
//! it was put in, a new one takes the cell remembered from the drop that brought it (`+0x150`), else its mapped cell,
//! else the first free one (FUN_100ce25d). With an item in hand, a click on a cell of the same container
//! (FUN_100cc855): an empty cell takes the item (`MultiListView_c::MoveItem`, nothing is sent); a cell with another
//! item asks to join the two as stacks (`N3Msg_JoinItems`), and when they do not join the two swap cells and the other
//! is in hand; with a split count in hand an empty cell splits the stack (`N3Msg_SplitItem`) and an occupied one ends
//! the drag.
//!
//! Here the cells are kept in `places.ini` beside `mao.ini`, a line an item: `<type> <instance> <place> <cell>`, the
//! container's identity (zeros for the inventory), the item's place in it as the server numbers it, and its cell.
//! A cell is counted along the rows (this client's grids take as many columns as their window is wide, and show the
//! cells in that order), where the original keeps a column and a row.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

/// A container: the inventory, or a bag by its identity.
pub type Key = (u32, u32);
pub const INVENTORY: Key = (0, 0);

/// A container's cells: each item's place and the cell it stands in.
pub type Cells = BTreeMap<u32, usize>;

static PLACES: Mutex<Option<BTreeMap<Key, Cells>>> = Mutex::new(None);

fn path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("places.ini")
}

/// Whether the cells are kept on disk (not in tests and previews).
fn kept() -> bool {
    !cfg!(test) && std::env::var_os("MAO_NO_LAYOUT").is_none() && std::env::var_os("MAO_UI_PREVIEW").is_none()
}

/// `places.ini`: a line an item, `<type> <instance> <place> <cell>`.
pub fn parse(text: &str) -> BTreeMap<Key, Cells> {
    let mut out: BTreeMap<Key, Cells> = BTreeMap::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if let [kind, instance, place, cell] = words[..]
            && let (Ok(kind), Ok(instance), Ok(place), Ok(cell)) = (kind.parse(), instance.parse(), place.parse(), cell.parse())
        {
            out.entry((kind, instance)).or_default().insert(place, cell);
        }
    }
    out
}

pub fn render(places: &BTreeMap<Key, Cells>) -> String {
    places.iter().flat_map(|((kind, instance), cells)| cells.iter().map(move |(place, cell)| format!("{kind} {instance} {place} {cell}\n"))).collect()
}

fn with<R>(f: impl FnOnce(&mut BTreeMap<Key, Cells>) -> R) -> R {
    let mut places = PLACES.lock().unwrap_or_else(|e| e.into_inner());
    let places = places.get_or_insert_with(|| if kept() { std::fs::read_to_string(path()).map(|t| parse(&t)).unwrap_or_default() } else { BTreeMap::new() });
    f(places)
}

/// Changes a container's cells; the file is written when they are no longer what they were.
fn change<R>(key: Key, f: impl FnOnce(&mut Cells) -> R) -> R {
    with(|places| {
        let before = places.get(&key).cloned().unwrap_or_default();
        let out = f(places.entry(key).or_default());
        if places.get(&key).is_some_and(|c| c.is_empty()) {
            places.remove(&key);
        }
        if places.get(&key).cloned().unwrap_or_default() != before && kept() {
            let _ = std::fs::write(path(), render(places));
        }
        out
    })
}

/// The cells of a container's items (their places, in order), `room` cells in all: the remembered ones kept, those of
/// items no longer there forgotten, the rest given the first free cells ([`assign`]); remembered from then on.
pub fn arrange(key: Key, items: &[u32], room: usize, awaited: Option<(u32, usize)>) -> Cells {
    change(key, |cells| {
        assign(cells, items, room, awaited);
        cells.clone()
    })
}

/// Puts the item of a place into a cell ([`move_to`]).
pub fn put(key: Key, place: u32, cell: usize) {
    change(key, |cells| move_to(cells, place, cell));
}

/// The items of two places change cells ([`swap`]).
pub fn exchange(key: Key, a: u32, b: u32) {
    change(key, |cells| swap(cells, a, b));
}

/// Gives every item a cell (FUN_100ce25d): an item keeps the cell remembered for its place (unless that is past the
/// container's `room`, or another item has it already); the cell of a place that holds nothing any more is forgotten;
/// a new item at the `awaited` place takes the cell awaited for it (the cell a drop or a split named) when that is
/// free; every other new item takes the first free cell, in the places' order. Whether anything changed.
pub fn assign(cells: &mut Cells, items: &[u32], room: usize, awaited: Option<(u32, usize)>) -> bool {
    let before = cells.clone();
    let mut taken = BTreeSet::new();
    cells.retain(|place, cell| items.contains(place) && *cell < room && taken.insert(*cell));
    if let Some((place, cell)) = awaited
        && items.contains(&place)
        && !cells.contains_key(&place)
        && cell < room
        && taken.insert(cell)
    {
        cells.insert(place, cell);
    }
    let mut new: Vec<u32> = items.iter().copied().filter(|p| !cells.contains_key(p)).collect();
    new.sort_unstable();
    new.dedup();
    for place in new {
        // (past the container's room when it holds more than it should: the grid grows a row)
        let free = (0..).find(|c| !taken.contains(c)).unwrap_or(0);
        taken.insert(free);
        cells.insert(place, free);
    }
    *cells != before
}

/// The item of a place put into an empty cell (`MultiListView_c::MoveItem`); nothing when another item stands there.
pub fn move_to(cells: &mut Cells, place: u32, cell: usize) {
    if !cells.iter().any(|(p, c)| *c == cell && *p != place) {
        cells.insert(place, cell);
    }
}

/// Two items change cells: the one in hand goes where the other stood, and the other, in hand next, will come back to
/// where the first was unless it is put elsewhere.
pub fn swap(cells: &mut Cells, a: u32, b: u32) {
    if let (Some(&at_a), Some(&at_b)) = (cells.get(&a), cells.get(&b)) {
        cells.insert(a, at_b);
        cells.insert(b, at_a);
    }
}

/// The item whose cell this is.
pub fn at(cells: &Cells, cell: usize) -> Option<u32> {
    cells.iter().find(|(_, c)| **c == cell).map(|(p, _)| *p)
}

/// What the stack rules look at in an item: its place, its template (low and high id: stats 0x2be and 0x2bf), its
/// quality (0x2bd), its `Can` flags (stat 0x1e) and how many it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stack {
    pub place: u32,
    pub low: u32,
    pub high: u32,
    pub quality: u32,
    pub can: u32,
    pub count: u32,
}

/// `Can`'s stackable bit (`N3Msg_JoinItems` asks it of both), and the one that forbids a split (`N3Msg_SplitItem`).
pub const CAN_STACK: u32 = 0x200;
pub const CAN_NOT_SPLIT: u32 = 0x100_0000;
/// The most a stack holds (`N3Msg_JoinItems`: the two together below 0xc351).
pub const STACK_MOST: u32 = 50000;

/// What letting the item in hand go on a cell of its own container does (GUI.dll FUN_100cc855, the left button).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placing {
    /// Back where it was: let go on itself, or a split let go on an item.
    Cancel,
    /// Into the empty cell.
    Move,
    /// The two join as one stack (the one in hand goes into the other).
    Join,
    /// The two change cells and the other is in hand.
    Swap,
    /// The split count becomes a stack of its own in the empty cell.
    Split,
}

/// Whether two items join as stacks (`N3Msg_JoinItems` 0x1001bb2a: both stackable, the same template low and high,
/// the same quality, and no more than a stack holds together).
pub fn joins(a: &Stack, b: &Stack) -> bool {
    a.place != b.place && a.can & b.can & CAN_STACK != 0 && (a.low, a.high, a.quality) == (b.low, b.high, b.quality) && a.count + b.count <= STACK_MOST
}

/// Two stacks of one template that do not join for their quality alone: the original says so
/// (`Feedback_CanOnlyStackItemsOfSameLevel`) before it swaps them.
pub fn other_level(a: &Stack, b: &Stack) -> bool {
    a.place != b.place && a.can & b.can & CAN_STACK != 0 && (a.low, a.high) == (b.low, b.high) && a.quality != b.quality
}

/// The left button let go with `held` in hand (`split` of it when a split is in progress, else 0) on a cell of the
/// same container, holding `onto` or nothing.
pub fn placing(held: &Stack, split: u32, onto: Option<&Stack>) -> Placing {
    match (split, onto) {
        (0, None) => Placing::Move,
        (0, Some(other)) if other.place == held.place => Placing::Cancel,
        (0, Some(other)) if joins(held, other) => Placing::Join,
        (0, Some(_)) => Placing::Swap,
        (_, None) => Placing::Split,
        (_, Some(_)) => Placing::Cancel,
    }
}

/// Why a stack is not split (`N3Msg_SplitItem` 0x1001bd13: the feedback's name), if it is not: the item forbids it, or
/// no place is free for the new stack. (Fewer in the stack than asked plus one: nothing, and nothing said.)
pub fn split_refusal(item: &Stack, count: u32, free_place: bool) -> Option<Option<&'static str>> {
    if item.can & CAN_NOT_SPLIT != 0 {
        Some(Some("Feedback_YouCantSplitThisItem"))
    } else if count == 0 || item.count < count + 1 {
        Some(None)
    } else if !free_place {
        Some(Some("Feedback_CantSplitAsInventoryIsFull"))
    } else {
        None
    }
}

/// The count typed into the split box for a stack of `count`: 1 to one less than the stack (the original's split mode
/// keeps its number in that range, FUN_10041ca4); nothing for what is no number.
pub fn split_count(typed: &str, count: u32) -> Option<u32> {
    let n: u32 = typed.trim().parse().ok()?;
    (count > 1).then(|| n.clamp(1, count - 1))
}

/// A split made in the client's own list of the inventory, as the original makes it before it tells the server
/// (Gamecode FUN_1004a2bf, called by `N3Msg_SplitItem`): the stack at `place` is `count` fewer, and a stack of `count`
/// of the same item stands at the first free place of the backpack (`first..first + room`). The new stack's place; nothing
/// when the stack is not there, has no more than `count`, or no place is free.
pub fn split_entry(entries: &mut Vec<mao_net::inventory::Entry>, place: u32, count: u32, first: u32, room: u32) -> Option<u32> {
    let at = entries.iter().position(|e| e.slot == place)?;
    let free = (first..first + room).find(|p| !entries.iter().any(|e| e.slot == *p))?;
    if count == 0 || entries[at].count as u32 <= count {
        return None;
    }
    let mut new = entries[at].clone();
    new.slot = free;
    new.count = count as u16;
    entries[at].count -= count as u16;
    entries.push(new);
    Some(free)
}

/// The server's `JoinItems` carried out in the client's own list (Gamecode FUN_1004a12e): the stack at `held` goes into
/// the one at `onto` and is gone. Whether it was done (both there, and no more than a stack holds together).
pub fn join_entries(entries: &mut Vec<mao_net::inventory::Entry>, onto: u32, held: u32) -> bool {
    let (Some(to), Some(from)) = (entries.iter().position(|e| e.slot == onto), entries.iter().position(|e| e.slot == held)) else { return false };
    let sum = entries[to].count as u32 + entries[from].count as u32;
    if to == from || sum > STACK_MOST {
        return false;
    }
    entries[to].count = sum as u16;
    entries.remove(from);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(list: &[(u32, usize)]) -> Cells {
        list.iter().copied().collect()
    }

    #[test]
    fn items_keep_their_cells_and_new_ones_take_the_first_free() {
        // (nothing remembered: the cells in the places' order, as the grid packed them before)
        let mut c = Cells::new();
        assert!(assign(&mut c, &[66, 64, 65], 30, None));
        assert_eq!(c, cells(&[(64, 0), (65, 1), (66, 2)]));
        // (65 put into cell 7 stays there; 64 gone: its cell is forgotten, and 66 does not move up; a new item takes
        // the first free cell)
        move_to(&mut c, 65, 7);
        assert!(assign(&mut c, &[65, 66, 70], 30, None));
        assert_eq!(c, cells(&[(65, 7), (66, 2), (70, 0)]));
        // (nothing changed: nothing to save)
        assert!(!assign(&mut c, &[65, 66, 70], 30, None));
        // (a new item awaited in a cell takes it when it comes, if the cell is free; not an item that was there already)
        assert!(assign(&mut c, &[65, 66, 70, 71], 30, Some((71, 12))));
        assert_eq!(c.get(&71), Some(&12));
        assert!(assign(&mut c, &[65, 66, 70, 71, 72, 73], 30, Some((72, 7))));
        assert_eq!((c.get(&72), c.get(&73), c.get(&65)), (Some(&1), Some(&3), Some(&7)));
        assert!(!assign(&mut c, &[65, 66, 70, 71, 72, 73], 30, Some((66, 20))));
        // (a file's cell past the container's room, and one two items claim: the first keeps it, the others are placed)
        let mut c = cells(&[(64, 40), (65, 3), (66, 3)]);
        assign(&mut c, &[64, 65, 66], 30, None);
        assert_eq!(c, cells(&[(64, 0), (65, 3), (66, 1)]));
        // (more items than cells: past the room, never on another)
        let mut c = Cells::new();
        assign(&mut c, &[1, 2, 3], 2, None);
        assert_eq!(c, cells(&[(1, 0), (2, 1), (3, 2)]));
    }

    #[test]
    fn a_move_takes_an_empty_cell_and_a_swap_changes_two() {
        let mut c = cells(&[(64, 0), (65, 1)]);
        move_to(&mut c, 64, 9);
        assert_eq!(c, cells(&[(64, 9), (65, 1)]));
        // (not onto another item; onto its own cell is nothing)
        move_to(&mut c, 64, 1);
        move_to(&mut c, 65, 1);
        assert_eq!(c, cells(&[(64, 9), (65, 1)]));
        swap(&mut c, 64, 65);
        assert_eq!(c, cells(&[(64, 1), (65, 9)]));
        assert_eq!((at(&c, 9), at(&c, 1), at(&c, 2)), (Some(65), Some(64), None));
        // (an item with no cell swaps with nothing)
        swap(&mut c, 64, 99);
        assert_eq!(c, cells(&[(64, 1), (65, 9)]));
    }

    #[test]
    fn the_file_is_a_line_an_item() {
        let mut places: BTreeMap<Key, Cells> = BTreeMap::new();
        places.insert(INVENTORY, cells(&[(64, 3), (70, 0)]));
        places.insert((51017, 77), cells(&[(2, 20)]));
        let text = render(&places);
        assert_eq!(text, "0 0 64 3\n0 0 70 0\n51017 77 2 20\n");
        assert_eq!(parse(&text), places);
        assert!(parse("junk\n0 0 64\n0 0 x 1\n0 0 64 3 9\n").is_empty());
    }

    #[test]
    fn a_held_item_moves_joins_swaps_or_splits() {
        let stack = |place, low, quality, can, count| Stack { place, low, high: low, quality, can, count };
        let ammo = stack(64, 100, 1, CAN_STACK, 200);
        let more = stack(65, 100, 1, CAN_STACK, 300);
        let gun = stack(66, 7, 50, 0, 1);
        // (an empty cell takes the item; itself: back in place)
        assert_eq!(placing(&ammo, 0, None), Placing::Move);
        assert_eq!(placing(&ammo, 0, Some(&ammo)), Placing::Cancel);
        // (two stacks of one template and quality join; anything else swaps)
        assert_eq!(placing(&ammo, 0, Some(&more)), Placing::Join);
        assert_eq!(placing(&ammo, 0, Some(&gun)), Placing::Swap);
        assert_eq!(placing(&gun, 0, Some(&ammo)), Placing::Swap);
        // (not of another quality, which is said; not another high template; not unstackable ones; not past 50000)
        let other = stack(67, 100, 2, CAN_STACK, 5);
        assert_eq!((placing(&ammo, 0, Some(&other)), other_level(&ammo, &other), other_level(&ammo, &more)), (Placing::Swap, true, false));
        assert!(!joins(&ammo, &Stack { high: 101, ..more }));
        assert!(!joins(&stack(1, 7, 50, 0, 1), &stack(2, 7, 50, 0, 1)));
        assert!(joins(&stack(1, 100, 1, CAN_STACK, 20000), &stack(2, 100, 1, CAN_STACK, 30000)));
        assert!(!joins(&stack(1, 100, 1, CAN_STACK, 20001), &stack(2, 100, 1, CAN_STACK, 30000)));
        // (a split in hand: an empty cell splits, an item ends it)
        assert_eq!(placing(&ammo, 50, None), Placing::Split);
        assert_eq!(placing(&ammo, 50, Some(&more)), Placing::Cancel);
        assert_eq!(placing(&ammo, 50, Some(&ammo)), Placing::Cancel);
        // (the split's refusals: an item that forbids it, more than the stack less one, no free place)
        assert_eq!(split_refusal(&ammo, 50, true), None);
        assert_eq!(split_refusal(&ammo, 199, true), None);
        assert_eq!(split_refusal(&ammo, 200, true), Some(None));
        assert_eq!(split_refusal(&ammo, 50, false), Some(Some("Feedback_CantSplitAsInventoryIsFull")));
        assert_eq!(split_refusal(&Stack { can: CAN_STACK | CAN_NOT_SPLIT, ..ammo }, 50, true), Some(Some("Feedback_YouCantSplitThisItem")));
        // (the count typed: 1 to the stack less one)
        assert_eq!((split_count("50", 200), split_count(" 0 ", 200), split_count("999", 200), split_count("x", 200), split_count("1", 1)), (Some(50), Some(1), Some(199), None, None));
    }

    #[test]
    fn the_inventory_list_splits_and_joins() {
        let entry = |slot, count, low_id| mao_net::inventory::Entry { slot, flags: 0, count, container: (0, 0), low_id, high_id: low_id, quality: 1, extra: 0 };
        let list = |e: &[mao_net::inventory::Entry]| e.iter().map(|e| (e.slot, e.count, e.low_id)).collect::<Vec<_>>();
        let mut e = vec![entry(64, 200, 100), entry(65, 1, 7)];
        // (fifty off the stack: a new stack of the item at the first free place)
        assert_eq!(split_entry(&mut e, 64, 50, 64, 30), Some(66));
        assert_eq!(list(&e), vec![(64, 150, 100), (65, 1, 7), (66, 50, 100)]);
        // (not all of a stack, not of nothing, not into a full backpack)
        assert_eq!((split_entry(&mut e, 66, 50, 64, 30), split_entry(&mut e, 65, 1, 64, 30), split_entry(&mut e, 90, 1, 64, 30), split_entry(&mut e, 64, 10, 64, 3)), (None, None, None, None));
        assert_eq!(list(&e), vec![(64, 150, 100), (65, 1, 7), (66, 50, 100)]);
        // (joined again: the held stack goes into the other and is gone)
        assert!(join_entries(&mut e, 66, 64));
        assert_eq!(list(&e), vec![(65, 1, 7), (66, 200, 100)]);
        assert!(!join_entries(&mut e, 66, 66));
        assert!(!join_entries(&mut e, 66, 99));
        let mut big = vec![entry(64, 30000, 100), entry(65, 20001, 100)];
        assert!(!join_entries(&mut big, 64, 65));
        big[1].count = 20000;
        assert!(join_entries(&mut big, 64, 65));
        assert_eq!(list(&big), vec![(64, 50000, 100)]);
    }
}
