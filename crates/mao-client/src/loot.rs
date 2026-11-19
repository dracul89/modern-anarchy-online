//! The loot window: what a corpse or a bag holds, as the server's `InventoryUpdate` lists it for a container the character used.
//! GUI.dll's `InventoryView_c` (FUN_100ccefe) for a container that is not one's own: an icon view of the items (its
//! `item_position_map` places them three to a row, 21 places), titled with the container's name; a corpse or chest that
//! is empty closes a second later (its `EventTimer_c` started for 1 000 000 µs). An item is taken by a click (the
//! list's pick-up signal, which a container that is not one's own answers with `MoveItemToInventory`: FUN_100cc6e8),
//! by a right click (not on a corpse), or all of them at once. In one's own backpack and the bank a click takes the item up instead
//! (the same function's `BeginDrag`): it stays on the pointer until the next click, which over the inventory window
//! takes it out. Shift+click shows an item's info.
//!
//! One's own backpack (`N3Msg_IsItemMine`: a bag in the inventory) has a window of its own in the original, as many
//! open at once as there are bags opened: named `Container_<type>x<instance>` (so each keeps its place), 21 places
//! three to a row, titled with the bag's name, which the window's menu (FUN_100cbace) can change ("Rename":
//! FUN_100cb852's "Enter backpack name." box with OK, "Reset to default" and Cancel; the original keeps the name in
//! the bag's own file, `Containers/Container_<type>x<instance>.xml`, here in `backpacks.ini` beside `mao.ini`). It
//! does not close when empty and has no "Take all". Its items stand where they are put (`item_position_map`; here
//! `places.rs`): the item taken up let go on an empty cell of the same backpack moves there, on another item the two
//! change cells and the other is in hand (FUN_100cc855; nothing is sent). Stacks join and split inside a backpack as
//! they do in the inventory (the same handler, FUN_100cc855, serves every item view): a stack let go on another of
//! its backpack that joins with it (`places::joins`, what `N3Msg_JoinItems` asks) is joined, [`Take::Join`]; Ctrl+click
//! on a stack asks how many (the inventory's box; the original counts by the mouse's sideways movement), and that
//! many let go on an empty cell of the same backpack become a stack of their own there, [`Take::Split`]; a split in
//! hand let go anywhere else ends without anything done. An item of a bag goes on the wire as
//! `(0x6b, handle << 16 | place)` (`Chest_t`'s FUN_1007e3e3). Across two containers nothing is ever joined (a plain
//! move). Not ported for these windows: an inventory item shown at the cell it was let go on, the menu's List Mode
//! and Auto Arrange, closing by Escape (`esc_backpacks`), and telling the server that a bag was closed (the
//! original's message was not identified). Corpses, chests and the organization's contract open arranged by name, the
//! bank and the reclaim terminal in their places' order ([`first_sort`]; the bank's stacks are not joined or split by
//! hand here: its window has no placing by hand).
//!
//! **An item's menu** (GUI.dll FUN_100caf07, connected for every item view by `InventoryViewBase_c`'s FUN_100ccde7):
//! the right button held on an item for `MouseButtonHoldDelay`, 0.3 s (the list's FUN_10041aa4 starts a timer at
//! the button's going down, FUN_100420fd raises the menu signal for the item it went down on; let go before that it
//! is a click). Its entries ([`item_entries`]): Look at Item (the info window, as Shift+click), Create Reference,
//! Delete Item in every view that is not read-only or a wear page, a corpse or another's chest among them, and on a
//! corpse, for one in a team, a Looter submenu ([`looter_entries`]: free for all, a random looter, or one of the
//! team's members; in a raid the teams first). Delete asks first (`ReallyDeleteItemX`), then `N3Msg_DeleteItem`
//! sends `CharacterAction` 0x70 with the item's own identity, `(0x6b, handle << 16 | place)` in a bag, chest or
//! corpse, `(0x69, place)` in the bank, `(0x6e, place)` in the overflow, and the item's low and high template; the
//! item goes when the server answers with the same action ([`Take::Delete`]). A looter is `N3Msg_SetLootAccess`:
//! the raid command 5 with the corpse, the item's place with whether to roll, and the looter. One capture has a
//! delete inside a backpack, with the server's answer; the bank's, the overflow's, a corpse's and every loot access
//! are the code's alone.
//!
//! Here Create Reference puts the item's link into the chat's line, as the inventory window's menu does (the
//! original puts the item on the reference board, the Knowledge window, and makes a chat link only of what is
//! dropped on the chat's line). The organization's contract has the first two entries only. The Looter entry and,
//! in a raid, its teams are sub-menus (`gui::popup`, since 2026-11-18; they took the menu's place before). A plain
//! right click takes the item, which is the original's for the bank, a chest and the overflow (the original uses an
//! item of one's own backpack out of the bag); on a corpse it does nothing, as the corpse's list has the feature flag
//! 0x400 that silences button 2 (FUN_100ce55d, FUN_1004141b).

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use crate::gear::GearItem;
use crate::gui::view::{Id, Layout, Node, Tree};
use crate::theme::Palette;

/// A container's contents, ready to show.
pub struct LootView {
    pub title: String,
    /// The container's identity (type, instance): what an item put in names it by.
    pub identity: (u32, u32),
    /// The container's handle: what a take message names it by.
    pub handle: u32,
    /// The items; `slot` is the item's place in the container.
    pub items: Vec<GearItem>,
    /// One's own backpack (a bag in the inventory): a window of its own.
    pub own: bool,
    /// How many times the container has been opened: a window shut comes up again when it is opened anew.
    pub opened: u32,
}

impl LootView {
    /// Whether the items are placed by hand while the grid is not arranged: the original's flag of an item view
    /// (`InventoryViewBase_c` +0x14c, set by FUN_100ce55d), which one's own backpacks have and the bank.
    pub fn placed(&self) -> bool {
        self.own || self.identity.0 == BANK
    }

    /// How many cells the items are placed in.
    fn room(&self) -> usize {
        if self.identity.0 == BANK { max_items(BANK, false) as usize } else { BAG_PLACES }
    }
}

/// The identity type of a bag or chest.
pub const BAG: u32 = 0xc749;

/// The places of one's own backpack (`ItemContainerView_c` FUN_100ce55d: `SetMaxItemCount(21)`).
pub const BAG_PLACES: usize = 21;

/// The names the player gave their backpacks, by the bag's identity (`InventoryGUIModule_c::SetBackpackName`).
static BAG_NAMES: Mutex<Option<BTreeMap<(u32, u32), String>>> = Mutex::new(None);

fn names_path() -> std::path::PathBuf {
    crate::settings::Settings::path().with_file_name("backpacks.ini")
}

/// Whether the names are kept on disk (not in tests and previews).
fn names_kept() -> bool {
    !cfg!(test) && std::env::var_os("MAO_NO_LAYOUT").is_none() && std::env::var_os("MAO_UI_PREVIEW").is_none()
}

/// `backpacks.ini`: a line a bag, `<type> <instance> <name>`.
pub fn parse_names(text: &str) -> BTreeMap<(u32, u32), String> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.splitn(3, ' ');
            let identity = (words.next()?.parse().ok()?, words.next()?.parse().ok()?);
            let name = words.next()?.trim();
            (!name.is_empty()).then(|| (identity, name.to_string()))
        })
        .collect()
}

pub fn render_names(names: &BTreeMap<(u32, u32), String>) -> String {
    names.iter().map(|((kind, instance), name)| format!("{kind} {instance} {name}\n")).collect()
}

fn with_names<R>(f: impl FnOnce(&mut BTreeMap<(u32, u32), String>) -> R) -> R {
    let mut names = BAG_NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let names = names.get_or_insert_with(|| if names_kept() { std::fs::read_to_string(names_path()).map(|t| parse_names(&t)).unwrap_or_default() } else { BTreeMap::new() });
    f(names)
}

/// The name the player gave a backpack, if any.
pub fn bag_name(identity: (u32, u32)) -> Option<String> {
    with_names(|names| names.get(&identity).cloned())
}

/// Names a backpack, or with `None` gives it its own name back ("Reset to default").
pub fn set_bag_name(identity: (u32, u32), name: Option<&str>) {
    with_names(|names| {
        match name.map(str::trim).filter(|n| !n.is_empty()) {
            // (a name is one line: the file keeps a bag a line)
            Some(name) => names.insert(identity, name.replace(['\r', '\n'], " ")),
            None => names.remove(&identity),
        };
        if names_kept() {
            let _ = std::fs::write(names_path(), render_names(names));
        }
    });
}

/// A container window's name (GUI.dll's `InventoryView_c`, FUN_100ccefe): one's own backpack's by its identity
/// (`Container_%ux%u`), the bank's `bank_window`, any other's `TempContainer_%u` by its number (`temp`): the lowest no other
/// container window has (FUN_100cb7f9, given back when the window goes, FUN_100cade8).
pub fn window_name(view: &LootView, temp: Option<u32>) -> String {
    if view.own {
        format!("Container_{}x{}", view.identity.0, view.identity.1)
    } else if view.identity.0 == BANK {
        "bank_window".to_string()
    } else {
        format!("TempContainer_{}", temp.unwrap_or(0))
    }
}

/// The lowest number no other container window has (FUN_100cb7f9).
pub fn free_number(taken: impl Iterator<Item = u32>) -> u32 {
    let taken: Vec<u32> = taken.collect();
    (0..).find(|n| !taken.contains(n)).unwrap_or(0)
}

/// The identity type of a corpse.
pub const CORPSE: u32 = 0xc76a;
/// The identity type of the character's overflow (and the handle of its list in an `InventoryUpdate`). Its window is
/// the original's container window (GUI.dll `InventoryView_c`, as for a bag that is not one's own): a click, left or
/// right, moves the item into the inventory (FUN_100cc6e8, FUN_100cae1b), nothing can be put into it, and it stays up
/// when it is empty.
pub const OVERFLOW: u32 = 0x6e;
/// The identity type of the bank (`BankIIR_t`'s `BankEntry_t`, FUN_10125b1b).
pub const BANK: u32 = 0xdead;

/// What the window asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Take {
    One { handle: u32, place: u32 },
    All { handle: u32 },
    /// An inventory item (its place) dropped on the window: put it in the container.
    Put { identity: (u32, u32), from: u32 },
    /// Two stacks of one's own backpack made one (`N3Msg_JoinItems(onto, held)`): their places in it.
    Join { handle: u32, onto: u32, held: u32 },
    /// `count` taken off the stack at `place` of one's own backpack as a stack of its own (`N3Msg_SplitItem`).
    Split { handle: u32, place: u32, count: u32 },
    /// The item at `place` of the container destroyed (`N3Msg_DeleteItem`), after the question: it has to be the item
    /// of these templates still when the message goes out.
    Delete { handle: u32, place: u32, low: u32, high: u32 },
}

/// An entry of an item's menu in a container window, or of its Looter submenus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    LookAt,
    Reference,
    Delete,
    /// The submenu of who may loot the item.
    Looters,
    FreeForAll,
    RandomLooter,
    /// A member of the team: its id, its name, and whether the client knows the character (in the playfield).
    Member(u32, String, bool),
    /// In a raid: the submenu of the team at that place.
    Team(usize),
}

impl Entry {
    /// The entry's label and whether it can be chosen. The first three are the game's texts (10000's `LookAtItem`,
    /// `CreateReference`, `DeleteItem`); the looters' labels are literals in the original's program, here in this
    /// client's words.
    pub fn label(&self) -> (String, bool) {
        let text = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
        match self {
            Entry::LookAt => (text("LookAtItem", "Look at Item"), true),
            Entry::Reference => (text("CreateReference", "Create Reference"), true),
            Entry::Delete => (text("DeleteItem", "Delete Item"), true),
            Entry::Looters => ("Looter".to_string(), true),
            Entry::FreeForAll => ("Free for all".to_string(), true),
            Entry::RandomLooter => ("Random looter".to_string(), true),
            Entry::Member(_, name, known) => (name.clone(), *known),
            Entry::Team(place) => (format!("Team {}", place + 1), true),
        }
    }
}

/// The entries of an item's menu in a container of this kind (FUN_100caf07): look at it, make a reference to it,
/// delete it (not in the organization's contract here, whose items' identities were not read), and on a corpse, for
/// one in a team, the looters.
pub fn item_entries(kind: u32, in_team: bool) -> Vec<Entry> {
    let mut entries = vec![Entry::LookAt, Entry::Reference];
    if kind != ORGANIZATION {
        entries.push(Entry::Delete);
    }
    if kind == CORPSE && in_team {
        entries.push(Entry::Looters);
    }
    entries
}

/// The Looter submenu (built at 0x100cb1ae): free for all, a random looter, then the team's members, each enabled
/// while the client knows its character (`N3Msg_isDynelInTree`); in a raid the teams that have members instead,
/// each a submenu of its members (`place`: that team's submenu).
pub fn looter_entries(team: &crate::team::Team, place: Option<usize>, known: &dyn Fn(u32) -> bool) -> Vec<Entry> {
    let members = |list: &[crate::team::Mate]| list.iter().map(|m| Entry::Member(m.id, m.name.clone(), known(m.id))).collect::<Vec<_>>();
    match place {
        Some(place) => members(team.raid_team(place)),
        None => {
            let mut entries = vec![Entry::FreeForAll, Entry::RandomLooter];
            if team.in_raid() {
                entries.extend((0..6).filter(|k| !team.raid_team(*k).is_empty()).map(Entry::Team));
            } else {
                entries.extend(members(&team.members));
            }
            entries
        }
    }
}

/// The menu's entries with the sub-menus under them: the Looter entry's ([`looter_entries`]) and, in a raid, each
/// team's under that.
pub fn menu_tree(kind: u32, team: &crate::team::Team, known: &dyn Fn(u32) -> bool) -> Vec<(Entry, Vec<(Entry, Vec<Entry>)>)> {
    item_entries(kind, !team.members.is_empty())
        .into_iter()
        .map(|entry| {
            let sub = if entry == Entry::Looters {
                looter_entries(team, None, known)
                    .into_iter()
                    .map(|looter| {
                        let members = match looter {
                            Entry::Team(place) => looter_entries(team, Some(place), known),
                            _ => Vec::new(),
                        };
                        (looter, members)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            (entry, sub)
        })
        .collect()
}

/// The popup's entries for a [`menu_tree`].
pub fn popup_entries(tree: &[(Entry, Vec<(Entry, Vec<Entry>)>)]) -> Vec<crate::gui::popup::Entry> {
    use crate::gui::popup::Entry as Item;
    let plain = |e: &Entry| {
        let (label, enabled) = e.label();
        Item::text(label, enabled)
    };
    tree.iter()
        .map(|(entry, sub)| match entry {
            Entry::Looters => Item::submenu(entry.label().0, true, sub.iter().map(|(looter, members)| if matches!(looter, Entry::Team(_)) { Item::submenu(looter.label().0, true, members.iter().map(plain).collect()) } else { plain(looter) }).collect()),
            _ => plain(entry),
        })
        .collect()
}

/// The entry at a place of a [`menu_tree`], through its sub-menus.
pub fn entry_at<'a>(tree: &'a [(Entry, Vec<(Entry, Vec<Entry>)>)], path: &[usize]) -> Option<&'a Entry> {
    match path {
        [a] => tree.get(*a).map(|e| &e.0),
        [a, b] => tree.get(*a)?.1.get(*b).map(|e| &e.0),
        [a, b, c] => tree.get(*a)?.1.get(*b)?.1.get(*c),
        _ => None,
    }
}

/// What a looter entry asks of the server: `N3Msg_SetLootAccess`'s raid command 5 (0x100184cf), its three
/// identities: the corpse, the item's place in it with whether to roll for it, and the looter (none for the first
/// two entries). Nothing for an entry that is no looter.
pub fn loot_access(entry: &Entry, corpse: (u32, u32), place: u32) -> Option<[(u32, u32); 3]> {
    let (roll, looter) = match entry {
        Entry::FreeForAll => (0, (0, 0)),
        Entry::RandomLooter => (1, (0, 0)),
        Entry::Member(id, _, _) => (0, (0xc350, *id)),
        _ => return None,
    };
    Some([corpse, (place & 0xffff, roll), looter])
}

/// The item a menu or a question is about: the container it was opened in and the item as it was then.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Chosen {
    container: (u32, u32),
    handle: u32,
    place: u32,
    low: u32,
    high: u32,
}

impl Chosen {
    /// Whether the window still shows that container with that item at that place (the shared window goes from one
    /// container to the next, and a backpack opened again has another handle).
    fn still(&self, view: &LootView) -> bool {
        view.identity == self.container && view.handle == self.handle && view.items.iter().any(|i| i.slot == self.place && i.item_id == self.low && i.high_id == self.high)
    }
}

/// The split asked for by Ctrl+click on a stack of a backpack: the backpack, the stack's place and the count being
/// typed (one box for all the backpacks' windows).
static SPLIT_BOX: Mutex<Option<((u32, u32), u32, String)>> = Mutex::new(None);

#[derive(Default)]
pub struct LootWindow {
    pub open: bool,
    /// The icons' textures (the handles kept: a dropped handle frees its texture, and the icon draws as nothing).
    textures: HashMap<u32, egui::TextureHandle>,
    /// Which container (identity, handle) has been empty since when (it closes a second later).
    empty_since: Option<((u32, u32), u32, std::time::Instant)>,
    /// The window's views (the `ItemContainerView_c`'s grid), for the container they were built for.
    views: Option<(Tree, Id, Id, (u32, u32))>,
    /// The items last put in the grid (their places, counts, icons and, in one's own backpack, cells).
    shown: Vec<(u32, u16, Option<u32>, Option<(usize, usize)>)>,
    /// The place of the item taken up out of the grid, and the frame it was taken up in (the click that takes it up
    /// does not also put it down).
    dragging: Option<u32>,
    taken_up: u64,
    /// Info pages asked for by Shift+click (`itemref://` addresses for the info view).
    pub infos: Vec<String>,
    /// The place of an item let go over the world (not over a window): to be used on what is there.
    pub dropped_out: Option<u32>,
    /// An inventory item in hand right-clicked on an item here, to be used on it: (its place, the place here).
    pub used_on: Option<(u32, u32)>,
    /// The handle and opening the window was last shown for: a backpack or a container opened again shows again.
    pub handle_seen: Option<(u32, u32)>,
    /// A container window's number (`TempContainer_%u`), while it is up.
    pub temp: Option<u32>,
    /// One's own backpack's window.
    pub own: bool,
    /// A backpack's menu (`PopupMenu_c`'s look) and the name being typed in its rename box.
    popup: Option<crate::gui::popup::PopupMenu>,
    renaming: Option<String>,
    /// How many of the stack taken up are in hand (a split to be made; 0: the whole item).
    split: u32,
    /// The place a split's new stack will take and the cell it was let go on, until it is there.
    awaited: Option<(u32, usize, std::time::Instant)>,
    /// The bank's cell an item put in was let go on, and since when: the next item that comes takes it.
    awaited_cell: Option<(usize, std::time::Instant)>,
    /// Feedback lines to be said, by their names (a stack that may not be split, stacks of another level).
    pub feedback: Vec<&'static str>,
    /// The item the right button went down on, and when: its menu comes 0.3 s later.
    right_held: Option<(u32, std::time::Instant)>,
    /// The item menu that is up: the item and its entries with their sub-menus (`menu_tree`).
    item_menu: Option<(Chosen, Vec<(Entry, Vec<(Entry, Vec<Entry>)>)>)>,
    item_popup: Option<crate::gui::popup::PopupMenu>,
    /// The item whose deleting is being asked about.
    confirm: Option<Chosen>,
    /// Links for the chat's line (the menu's Create Reference).
    pub references: Vec<String>,
    /// The container kind the three below were read for ([`config_key`]): the shared window goes from one to the next.
    config: String,
    /// List mode (`listview_mode`), and how the grid is ordered (`grid_sort_column` / `grid_sort_order`; `None`: not
    /// auto arranged, each item at the cell it was put in), once read from what is kept.
    list: Option<bool>,
    grid_sort: Option<Option<(i32, i32)>>,
    /// The columns' widths and the list's sort, kept (`listview_config`).
    keeper: crate::gui::multilist::Keeper,
    /// The window menu that is up: its entries' keys, by their places through the sub-menu.
    window_menu: Option<Vec<Vec<String>>>,
}

/// What a container's list mode and order are kept under (the original keeps a file each: `Bank.xml`,
/// `Container_<type>x<instance>.xml` for one's own backpack, `TempContainer_<handle>.xml` for another's; here the
/// others share one, since their handles are new each time).
/// Whether the bank's window was up at the last look.
static BANK_UP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the bank's window was shut since the last look: it was up and is now there and down (the window of a bank
/// the server took away is gone, not shut: nothing is told for it). The original's window close uses the bank again
/// (FUN_100ca891), which tells the server (`net::close_bank`).
pub fn bank_shut(window: Option<bool>) -> bool {
    shut(BANK_UP.swap(window == Some(true), std::sync::atomic::Ordering::Relaxed), window)
}

fn shut(was_up: bool, window: Option<bool>) -> bool {
    was_up && window == Some(false)
}

/// Whether a plain right click on an item of a container takes it (FUN_1004141b's button 2, the use signal): not on a
/// corpse, whose list FUN_100ce55d gives the feature flag 0x400.
pub fn right_click_takes(kind: u32) -> bool {
    kind != CORPSE
}

/// The identity type of the reclaim terminal's list.
pub const RECLAIM: u32 = 0xdeae;

/// How a container's grid is arranged before anything is kept (`ItemContainerView_c` FUN_100ce55d): one placed by
/// hand (one's own backpack, the bank) not at all; another's container, the organization's contract and a trade's
/// list by name (`SetListSortColumn(1, 0)`, FUN_10041258(1)); the reclaim terminal's not (the same function leaves
/// it out of the sorting), its items in their places' order.
pub fn first_sort(kind: u32, placed: bool) -> Option<(i32, i32)> {
    if placed || kind == RECLAIM { None } else { Some((1, 0)) }
}

fn config_key(view: &LootView) -> String {
    match view.identity.0 {
        _ if view.own => format!("Container_{}x{}", view.identity.0, view.identity.1),
        BANK => "Bank".to_string(),
        _ => "TempContainer".to_string(),
    }
}

/// Which "Auto Arrange" sub-menu a container has (FUN_10041637's flag, `InventoryView_c` +0x14c): one's own
/// (the flag set: not arranging at all is offered, and the order as it is; the bank has the flag too) or another's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrange {
    Own,
    Other,
}

/// The item views' columns (FUN_100ce55d with FUN_100ccde7's flags 0xb, so no price): id, the program's name for it,
/// width, flags.
pub const COLUMNS: [(i32, &str, f32, u32); 4] = [(0, "Icon", 16.0, 8), (1, "Name", 200.0, 0xe), (2, "Count", 30.0, 0xe), (4, "Quality", 100.0, 0xe)];

/// The "Auto Arrange" sub-menu of an item view (FUN_10041637): with the flag "Don't Auto Arrange", marked while the
/// grid is not arranged, and a separator; for each column "By <label>", marked when the grid is arranged by it (the
/// icon's column is "Unsorted" with the flag and not there without); a separator; "Sort Ascending", marked when the
/// order is ascending and dead while not arranged or unsorted. The keys (none for a separator) and the entries.
pub fn arrange_menu(sort: Option<(i32, i32)>, kind: Arrange) -> (Vec<String>, Vec<crate::gui::popup::Entry>) {
    use crate::gui::popup::Entry;
    let (mut keys, mut entries) = (Vec::new(), Vec::new());
    if kind == Arrange::Own {
        keys.extend(["none".to_string(), String::new()]);
        entries.extend([Entry::marked("Don't Auto Arrange", true, sort.is_none()), Entry::separator()]);
    }
    for (id, label, _, _) in COLUMNS {
        if id == 0 && kind == Arrange::Other {
            continue;
        }
        keys.push(format!("by:{id}"));
        entries.push(Entry::marked(if id == 0 { "Unsorted".to_string() } else { format!("By {label}") }, true, sort.is_some_and(|s| s.0 == id)));
    }
    keys.extend([String::new(), "ascending".to_string()]);
    entries.extend([Entry::separator(), Entry::marked("Sort Ascending", sort.is_some_and(|s| s.0 != 0), sort.is_none_or(|s| s.1 == 0))]);
    (keys, entries)
}

/// What an "Auto Arrange" entry makes of the order (FUN_10041303, FUN_10041331, FUN_10041258): a column only when
/// its entry was not marked, keeping the order (the icon's column always ascending); the order turned round.
pub fn arrange_choice(key: &str, sort: Option<(i32, i32)>) -> Option<Option<(i32, i32)>> {
    match key.split_once(':').unwrap_or((key, "")) {
        ("none", _) => sort.is_some().then_some(None),
        ("by", id) => {
            let id: i32 = id.parse().ok()?;
            (sort.map(|s| s.0) != Some(id)).then(|| Some((id, if id == 0 { 0 } else { sort.map_or(0, |s| s.1) })))
        }
        ("ascending", _) => sort.map(|s| Some((s.0, 1 - s.1))),
        _ => None,
    }
}

/// The most items a container's grid shows slots for (`ItemContainerView_c` FUN_100ce55d: `SetMaxItemCount` 102 for the bank,
/// 21 for one's own backpack, none for a corpse or chest).
impl LootWindow {
    /// The identity type of the container shown.
    pub fn kind(&self) -> Option<u32> {
        self.views.as_ref().map(|v| v.3.0)
    }
}

/// The identity type of an organization: its contract is a container of that identity (`/org contract`; GUI.dll's
/// `ItemContainerView_c` FUN_100ce55d gives it 100 places and sorts it as it does the bank).
pub const ORGANIZATION: u32 = 0xdeaa;

pub fn max_items(kind: u32, own: bool) -> i32 {
    match kind {
        BANK => 0x66,
        ORGANIZATION => 100,
        _ if own => BAG_PLACES as i32,
        _ => -1,
    }
}

/// The columns the window opens with: the bank's eight, three for the rest (`InventoryView_c`'s position map: three to a
/// row).
fn columns(kind: u32) -> usize {
    if kind == BANK { 8 } else { 3 }
}

impl LootWindow {
    /// `InventoryView_c` (FUN_100ccefe) for a container: its `ItemContainerView_c` (FUN_100ce55d) of a grid-mode
    /// `MultiListView_c` (`gui::multilist`, feature flags 0x15) over the whole window.
    fn build(&mut self, ctx: &egui::Context, view: &LootView) {
        let mut tree = Tree::default();
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        let root = tree.add(Node::new("", Box::new(crate::gui::view::Plain)));
        tree.get_mut(root).layout = Layout::Base;
        let list = crate::gui::multilist::create_grid(&mut tree, "ItemContainerView", max_items(view.identity.0, view.own), columns(view.identity.0));
        tree.add_child(root, list);
        // (the list mode's columns, sorted by name: `SetListSortColumn(1, 0)`)
        for (id, label, width, flags) in COLUMNS {
            crate::gui::multilist::add_column(&mut tree, list, id, label, width, flags);
        }
        crate::gui::multilist::set_sort(&mut tree, list, 1, 0);
        self.keeper = Default::default();
        self.views = Some((tree, root, list, view.identity));
        self.shown.clear();
    }

    pub fn show(&mut self, ctx: &egui::Context, view: &LootView, palette: &Palette) -> Option<Take> {
        let mut take = None;
        let mut open = self.open;
        // (FUN_100ccefe: an empty corpse or chest closes after a second; the bank and one's own backpack stay)
        if view.items.is_empty() && !matches!(view.identity.0, BANK | ORGANIZATION | OVERFLOW) && !view.own {
            if self.empty_since.is_none_or(|e| (e.0, e.1) != (view.identity, view.handle)) {
                self.empty_since = Some((view.identity, view.handle, std::time::Instant::now()));
            }
            if self.empty_since.is_some_and(|e| e.2.elapsed() >= std::time::Duration::from_secs(1)) {
                self.open = false;
                return None;
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        } else {
            self.empty_since = None;
        }
        if self.views.as_ref().is_none_or(|v| v.3 != view.identity) {
            self.build(ctx, view);
        }
        // (the items in the grid in their places' order, as `AddItem` puts them; auto arranged, but for one's own
        // backpack: there each item stands at its own cell, counted along the rows the window's width makes: the one
        // it was put in by hand, else the first free one)
        let mut items: Vec<&GearItem> = view.items.iter().collect();
        items.sort_by_key(|i| i.slot);
        // (the container's list mode and order, as kept: one's own backpack starts not arranged, each item at the
        // cell it is given (`places.rs`); another's container starts arranged by name (FUN_100ce55d's
        // `SetListSortColumn(1, 0)` and FUN_10041258(1, 0)); the bank in its places' order, as it stood here before)
        let key = config_key(view);
        if self.config != key {
            (self.list, self.grid_sort, self.keeper) = (None, None, Default::default());
            self.config = key.clone();
        }
        // (`MAO_LOOT_LIST=1` and `MAO_LOOT_SORT=<n>`, the kept number: for a picture)
        let in_list = *self.list.get_or_insert_with(|| std::env::var_os("MAO_LOOT_LIST").is_some() || crate::layout::tab(&format!("{key}List")) == Some(1));
        let arranged = *self.grid_sort.get_or_insert_with(|| match std::env::var("MAO_LOOT_SORT").ok().and_then(|v| v.parse().ok()).or_else(|| crate::layout::tab(&format!("{key}GridSort"))) {
            Some(0) if view.placed() => None,
            Some(n) if n > 0 => Some((((n - 1) / 2) as i32, ((n - 1) % 2) as i32)),
            _ => first_sort(view.identity.0, view.placed()),
        });
        // (placed by hand: the grid of one's own backpack or of the bank while it is not arranged)
        let hand = view.placed() && arranged.is_none() && !in_list;
        let textures = &mut self.textures;
        let (tree, root, list, _) = self.views.as_mut()?;
        let (root, list) = (*root, *list);
        crate::gui::multilist::set_list_mode(tree, list, in_list);
        if !in_list {
            crate::gui::multilist::set_grid(tree, list, |m| m.auto_arrange = !hand);
        }
        self.keeper.before(&format!("{key}Config"), tree, list, (!in_list).then(|| arranged.unwrap_or((0, 0))));
        crate::gui::multilist::update(tree, list);
        let grid_columns = crate::gui::multilist::grid_columns(tree, list).unwrap_or(3).max(1);
        // (one's own backpack's cells are kept and counted whatever the view shows, so that "unsorted" is their order
        // and switching the arranging off finds them)
        // (an item dropped into the bank on a cell: the server gives it a place of its choosing, so the cell waits for
        // the next item that has none yet, as the original keeps the cell for the next item added, `+0x150`)
        let slots: Vec<u32> = items.iter().map(|i| i.slot).collect();
        let awaited = self.awaited.map(|a| (a.0, a.1)).or_else(|| {
            let (cell, _) = self.awaited_cell?;
            let known = crate::places::remembered(view.identity);
            // (nothing remembered yet: the items are all new, and none of them is the one dropped)
            let new = slots.iter().copied().find(|p| !known.contains_key(p)).filter(|_| !known.is_empty())?;
            Some((new, cell))
        });
        if awaited.is_some() && self.awaited.is_none() || self.awaited_cell.is_some_and(|(_, since)| since.elapsed() > std::time::Duration::from_secs(5)) {
            self.awaited_cell = None;
        }
        let cells = if view.placed() { crate::places::arrange(view.identity, &slots, view.room(), awaited) } else { crate::places::Cells::new() };
        if self.awaited.is_some_and(|(place, _, since)| items.iter().any(|i| i.slot == place) || since.elapsed() > std::time::Duration::from_secs(5)) {
            self.awaited = None;
        }
        let now: Vec<(u32, u16, Option<u32>, Option<(usize, usize)>)> = items.iter().map(|i| (i.slot, i.count, i.icon.as_ref().map(|c| c.id), cells.get(&i.slot).filter(|_| hand).map(|c| (c % grid_columns, c / grid_columns)))).collect();
        if self.shown != now {
            let rows: Vec<crate::gui::multilist::Item> = items
                .iter()
                .zip(&now)
                .map(|(i, shown)| crate::gui::multilist::Item {
                    id: i.slot as i64,
                    icon: i.icon.as_ref().map(|icon| textures.entry(icon.id).or_insert_with(|| ctx.load_texture(format!("loot-icon-{}", icon.id), egui::ColorImage::from_rgba_unmultiplied([icon.width, icon.height], &icon.rgba), egui::TextureOptions::LINEAR)).id()),
                    count: i.count as u32,
                    // (FUN_1003dee6: the name, how many (nothing counted reads 1) and the quality; sorted by the place
                    // or cell, the name without its case, and the two numbers, FUN_1003d5f3)
                    cells: vec![
                        (1, crate::gui::multilist::CellView::Text(i.name.clone())),
                        (2, crate::gui::multilist::CellView::Text(i.count.max(1).to_string())),
                        (4, crate::gui::multilist::CellView::Text(i.quality.to_string())),
                    ],
                    keys: vec![
                        (0, format!("{:08}", cells.get(&i.slot).map_or(i.slot as usize, |c| *c))),
                        (1, i.name.to_lowercase()),
                        (2, format!("{:010}", i.count.max(1))),
                        (4, format!("{:010}", i.quality)),
                    ],
                    pos: shown.3,
                    left: None,
                    colour: None,
                })
                .collect();
            crate::gui::multilist::set_items(tree, list, rows);
            self.shown = now;
        }
        let mut hovered_cell: Option<usize> = None;
        let mut hovered_item: Option<u32> = None;
        // (a menu or a question about an item of another container, or of an item no longer there, goes)
        if self.item_menu.as_ref().is_some_and(|m| !m.0.still(view)) {
            (self.item_menu, self.item_popup) = (None, None);
        }
        if self.confirm.as_ref().is_some_and(|c| !c.still(view)) {
            self.confirm = None;
        }
        let menu_up = self.item_menu.is_some();
        let skin = crate::skin::skin(ctx);
        let cols = columns(view.identity.0);
        // (room for the columns and the scroll bar, and up to six rows: the bank's, or as many as the items fill)
        // (one's own backpack: its 21 places)
        let rows_shown = if view.identity.0 == BANK { 6 } else if view.own { BAG_PLACES / cols } else { view.items.len().div_ceil(cols).clamp(1, 6) };
        let size = egui::vec2(cols as f32 * crate::gui::multilist::grid::STEP + 2.0 * crate::gui::multilist::grid::BORDER + 32.0, crate::gui::multilist::grid::size(rows_shown * cols, cols).1 + if view.identity.0 == BANK || view.own { 0.0 } else { 26.0 });
        // (a backpack's window by its identity, titled with the name the player gave it or the bag's own)
        let name = window_name(view, self.temp);
        let title = if view.own { bag_name(view.identity).unwrap_or_else(|| view.title.clone()) } else { view.title.clone() };
        let at = if view.own { egui::pos2(420.0 + (view.identity.1 % 5) as f32 * 30.0, 160.0 + (view.identity.1 % 4) as f32 * 30.0) } else { egui::pos2(380.0 + self.temp.unwrap_or(0) as f32 * 30.0, 140.0 + self.temp.unwrap_or(0) as f32 * 30.0) };
        let mut menu_at = None;
        let mut used_on = None;
        let tip = |item: &GearItem| {
            // (an item's tip: its name, quality and description)
            let mut tip = item.name.clone();
            if item.quality > 1 {
                tip.push_str(&format!("\nQL {}", item.quality));
            }
            if !item.description.is_empty() {
                tip.push_str("\n\n");
                tip.push_str(&item.description.chars().take(300).collect::<String>());
            }
            tip
        };
        let shown = crate::chrome::window(ctx, palette, &name, &title, &mut open, |w| w.default_pos(crate::layout::pos(&name, at)).default_size(size).resizable(true), |ui| {
            // (the views over the window less the room of the "Take all" row under them)
            // (laid out from the bottom: the button first, the views over what is left, so the window does not grow)
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                if !view.items.is_empty() && !matches!(view.identity.0, BANK | ORGANIZATION) && !view.own && crate::chrome::button(ui, palette, "Take all", true).clicked() {
                    take = Some(Take::All { handle: view.handle });
                }
                let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
                // (`MAO_ITEM_TRACE`: where the container's list and each of its items are, in the window's pixels, for a
                // script's clicks; told when it changes)
                if crate::gear::traced() {
                    let k = ui.ctx().pixels_per_point();
                    let mut places: std::collections::BTreeMap<i64, (f32, f32, u32)> = std::collections::BTreeMap::new();
                    let (mut x, rect) = (response.rect.left() + 4.0, response.rect.intersect(ui.clip_rect()));
                    while x < rect.right() {
                        let mut y = rect.top() + 4.0;
                        while y < rect.bottom() {
                            if let Some(slot) = crate::gui::multilist::item_at(tree, list, response.rect.min, egui::pos2(x, y)) {
                                let p = places.entry(slot).or_insert((0.0, 0.0, 0));
                                *p = (p.0 + x, p.1 + y, p.2 + 1);
                            }
                            y += 4.0;
                        }
                        x += 4.0;
                    }
                    let told = format!(
                        "the container {:#x}:{:#x} ({name}): its list from {:.0};{:.0} to {:.0};{:.0}, items {:?}",
                        view.identity.0,
                        view.identity.1,
                        rect.left() * k,
                        rect.top() * k,
                        rect.right() * k,
                        rect.bottom() * k,
                        places.iter().map(|(slot, p)| format!("{slot} at {:.0};{:.0} {:?}", p.0 / p.2 as f32 * k, p.1 / p.2 as f32 * k, view.items.iter().find(|i| i.slot as i64 == *slot).map_or("", |i| i.name.as_str()))).collect::<Vec<_>>()
                    );
                    let id = egui::Id::unique(("item-trace-container", view.identity));
                    if ui.ctx().data(|d| d.get_temp::<String>(id)).as_ref() != Some(&told) {
                        eprintln!("items: {told}");
                        ui.ctx().data_mut(|d| d.insert_temp(id, told));
                    }
                }
                let hovered = ui.input(|i| i.pointer.hover_pos()).and_then(|at| crate::gui::multilist::item_at(tree, list, response.rect.min, at));
                // (the cell under the pointer, with an item or not: where an item of the backpack is put by hand)
                hovered_cell = ui.input(|i| i.pointer.hover_pos()).and_then(|at| crate::gui::multilist::cell_at(tree, list, response.rect.min, at)).map(|(x, y)| y * grid_columns + x).filter(|c| hand && *c < view.room());
                hovered_item = hovered.map(|slot| slot as u32);
                // (a right click takes the item: the list's use signal, FUN_1004141b's button 2; not on a corpse, whose
                // list has the feature flag 0x400 (FUN_100ce55d) that makes button 2 do nothing there (the container
                // lists never set +0x319, the new mouse's own branch); not with an inventory item in hand, whose right
                // click is a use of it on another; and not the letting go that ends the held button's menu)
                if let Some(slot) = hovered
                    && !menu_up
                    && ui.input(|i| i.pointer.secondary_clicked())
                {
                    match crate::gear::held() {
                        None if !right_click_takes(view.identity.0) => {}
                        None => take = Some(Take::One { handle: view.handle, place: slot as u32 }),
                        // (the item in hand is used on the one clicked, FUN_100cc855's right button: not a split, and
                        // not with Shift, which is the inventory's combine and ends the drag anywhere else.)
                        Some(from) => {
                            if crate::gear::split_count() == 0 && !ui.input(|i| i.modifiers.shift) {
                                used_on = Some((from, slot as u32));
                            }
                            crate::gear::put_down();
                        }
                    }
                }
                // (the window's menu, FUN_100cbace: the right button beside the items. The original's is the frame's
                // menu button.)
                if hovered.is_none()
                    && ui.input(|i| i.pointer.secondary_clicked())
                    && let Some(at) = ui.input(|i| i.pointer.interact_pos()).filter(|at| response.rect.contains(*at))
                {
                    menu_at = Some(at);
                }
                if let Some(item) = hovered.and_then(|slot| view.items.iter().find(|i| i.slot as i64 == slot)) {
                    response.on_hover_text(tip(item));
                }
            });
        });
        self.keeper.after(&format!("{key}Config"), tree, list, in_list);
        if used_on.is_some() {
            self.used_on = used_on;
        }
        // (a click: with Shift the item's info; in one's own backpack or the bank it takes the item up; in a corpse
        // or chest it takes the item into the inventory. A press that moves is nothing.)
        let (shift, ctrl) = ctx.input(|i| (i.modifiers.shift, i.modifiers.ctrl));
        let hands_free = self.dragging.is_none() && crate::gear::held().is_none();
        crate::gui::multilist::click_only(tree, list);
        for (_, event) in crate::gui::multilist::take_events(tree) {
            let crate::gui::multilist::ItemEvent::Clicked(slot, _) = event else { continue };
            crate::gui::multilist::select(tree, list, Some(slot));
            // (Ctrl on a stack of more than one of one's own backpack: a split, FUN_10041926)
            let split = ctrl && view.own && view.items.iter().any(|i| i.slot as i64 == slot && i.count > 1);
            match container_click(shift, view.own || view.identity.0 == BANK, hands_free, split) {
                ContainerClick::Info => {
                    if crate::gear::traced() {
                        eprintln!("items: the container's place {slot}: its info");
                    }
                    self.infos.extend(view.items.iter().find(|i| i.slot as i64 == slot).map(|i| format!("itemref://{}/{}/{}", i.item_id, i.high_id, i.quality)));
                }
                ContainerClick::TakeUp => {
                    self.dragging = Some(slot as u32);
                    self.taken_up = ctx.cumulative_frame_nr();
                    if crate::gear::traced() {
                        eprintln!("items: the container's place {slot} taken up");
                    }
                }
                ContainerClick::Take => take = Some(Take::One { handle: view.handle, place: slot as u32 }),
                ContainerClick::Split => {
                    if crate::gear::traced() {
                        eprintln!("items: a split of the container's place {slot} asked for");
                    }
                    *SPLIT_BOX.lock().unwrap_or_else(|e| e.into_inner()) = Some((view.identity, slot as u32, "1".to_string()));
                }
                ContainerClick::Nothing => {}
            }
        }
        tree.signals.clear();
        // (the item taken up follows the pointer; let go over the inventory window, it is taken into the backpack)
        if let Some(place) = self.dragging {
            let pointer = ctx.input(|i| i.pointer.hover_pos());
            if let (Some(at), Some(icon)) = (pointer, view.items.iter().find(|i| i.slot == place).and_then(|i| i.icon.as_ref()).and_then(|icon| self.textures.get(&icon.id))) {
                ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("dragged-loot"))).image(icon.id(), egui::Rect::from_center_size(at, egui::vec2(40.0, 40.0)), egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::from_white_alpha(200));
            }
            if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) && self.taken_up != ctx.cumulative_frame_nr() {
                self.dragging = None;
                let split = std::mem::take(&mut self.split);
                if crate::gear::traced() {
                    eprintln!("items: the container's place {place} let go");
                }
                // (a split in hand is made only in its own container: let go anywhere else the drag ends with nothing
                // done, FUN_100cc855's `split_count > 0` branches)
                if split == 0 && !ctx.is_pointer_over_egui() {
                    self.dropped_out = Some(place);
                }
                let inventory = ctx.memory(|m| m.area_rect(egui::Id::unique("Inventory")));
                if split == 0
                    && let (Some(at), Some(r)) = (pointer, inventory)
                    && r.contains(at)
                {
                    take = Some(Take::One { handle: view.handle, place });
                }
                // (let go on a cell of its own backpack, FUN_100cc855: an empty cell takes the item
                // (`MultiListView_c::MoveItem`, the client's own matter), or with a split in hand that many of the
                // stack as a stack of their own; on another item the two join as stacks when they can
                // (`N3Msg_JoinItems`), else they change cells and the other is in hand; a split on an item ends)
                // (in list mode and in an arranged grid there are no cells to put an item in: stacks still join, and a
                // split's new stack takes the first free place, FUN_100cc855's `GetLayoutMode() != 1` branches and
                // `MoveItem` under auto arrange)
                if view.placed()
                    && shown.as_ref().is_some_and(|s| s.response.contains_pointer())
                    && (!hand || hovered_cell.is_some())
                    && let Some(held) = view.items.iter().find(|i| i.slot == place)
                {
                    use crate::places::Placing;
                    let cell = hovered_cell.unwrap_or(0);
                    let stack = |i: &GearItem| crate::places::Stack { place: i.slot, low: i.item_id, high: i.high_id, quality: i.quality, can: i.can, count: i.count as u32 };
                    let onto = if hand { crate::places::at(&cells, cell).and_then(|p| view.items.iter().find(|i| i.slot == p)) } else { hovered_item.filter(|p| *p != place).and_then(|p| view.items.iter().find(|i| i.slot == p)) };
                    match (crate::places::placing(&stack(held), split, onto.map(stack).as_ref()), onto) {
                        (Placing::Move, _) if hand => {
                            crate::places::put(view.identity, place, cell);
                            if crate::gear::traced() {
                                eprintln!("items: the container's place {place} put into cell {cell}");
                            }
                        }
                        // (not in the bank: how its stacks' joining goes on the wire is not known here, so an item let
                        // go on a stack it could join stays where it was)
                        (Placing::Join, Some(other)) if view.own => {
                            take = Some(Take::Join { handle: view.handle, onto: other.slot, held: place });
                            if crate::gear::traced() {
                                eprintln!("items: the container's place {place} joined to the stack at place {}", other.slot);
                            }
                        }
                        (Placing::Swap, Some(other)) if hand => {
                            if crate::places::other_level(&stack(held), &stack(other)) {
                                self.feedback.push("Feedback_CanOnlyStackItemsOfSameLevel");
                            }
                            crate::places::exchange(view.identity, place, other.slot);
                            if crate::gear::traced() {
                                eprintln!("items: the container's places {place} and {} change cells: place {place} into cell {cell}, the other in hand", other.slot);
                            }
                            self.dragging = Some(other.slot);
                            self.taken_up = ctx.cumulative_frame_nr();
                        }
                        // (`N3Msg_SplitItem` looks first whether the item may be split and a place of the backpack
                        // is free, from its place 0; the new stack is shown in this cell)
                        (Placing::Split, _) => {
                            let free = (0..BAG_PLACES as u32).find(|p| !view.items.iter().any(|i| i.slot == *p));
                            match crate::places::split_refusal(&stack(held), split, free.is_some()) {
                                Some(why) => self.feedback.extend(why),
                                None => {
                                    take = Some(Take::Split { handle: view.handle, place, count: split });
                                    self.awaited = free.filter(|_| hand).map(|free| (free, cell, std::time::Instant::now()));
                                    if crate::gear::traced() {
                                        eprintln!("items: {split} of the container's place {place} split off into cell {cell}");
                                    }
                                }
                            }
                        }
                        // (let go on itself, or a split on an item: back in place)
                        _ => {}
                    }
                }
            }
        }
        // An inventory item dropped on the window goes in (not into a corpse).
        if let Some(shown) = shown {
            let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
            if dragged != u32::MAX && !matches!(view.identity.0, CORPSE | OVERFLOW) && crate::gear::letting_go(ctx) && shown.response.contains_pointer() {
                take = Some(Take::Put { identity: view.identity, from: dragged });
                // (into the bank by hand: the cell it is let go on is where it will stand, when that cell is empty;
                // FUN_100cc855 keeps the cell, or the first free one for a taken cell, and sends no cell)
                if view.identity.0 == BANK
                    && let Some(cell) = hovered_cell.filter(|c| crate::places::at(&cells, *c).is_none())
                {
                    self.awaited_cell = Some((cell, std::time::Instant::now()));
                }
            }
        }
        // (an item's menu: the right button held on it for 0.3 s, for the item it went down on wherever the pointer is
        // by then, FUN_10041aa4 / FUN_100420fd; not with something in hand)
        if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
            self.right_held = hovered_item.map(|place| (place, std::time::Instant::now()));
        }
        if !ctx.input(|i| i.pointer.secondary_down()) && !ctx.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
            self.right_held = None;
        }
        if let Some((place, since)) = self.right_held {
            ctx.request_repaint();
            if since.elapsed() >= std::time::Duration::from_millis(300)
                && self.item_menu.is_none()
                && self.dragging.is_none()
                && crate::gear::held().is_none()
                && let Some(at) = ctx.input(|i| i.pointer.interact_pos())
                && let Some(item) = view.items.iter().find(|i| i.slot == place)
            {
                if crate::gear::traced() {
                    eprintln!("items: the menu of the container's place {place} opened by the held right button");
                }
                let chosen = Chosen { container: view.identity, handle: view.handle, place, low: item.item_id, high: item.high_id };
                let tree = crate::team::with(|t| menu_tree(view.identity.0, t, &crate::team::present));
                self.item_popup = Some(crate::chrome::menu_of(ctx, at, popup_entries(&tree)));
                self.item_menu = Some((chosen, tree));
                self.confirm = None;
                self.right_held = None;
            }
        }
        if let Some(crate::chrome::Picked::Entry(path)) = crate::chrome::menu_path(ctx, palette, "container-item-menu", &mut self.item_popup)
            && let Some((chosen, tree)) = self.item_menu.clone()
            && let (Some(entry), Some(item)) = (entry_at(&tree, &path), view.items.iter().find(|i| i.slot == chosen.place))
        {
            if crate::gear::traced() {
                eprintln!("items: the container's place {}: the menu's {entry:?}", chosen.place);
            }
            match entry {
                Entry::LookAt => self.infos.push(format!("itemref://{}/{}/{}", item.item_id, item.high_id, item.quality)),
                Entry::Reference => self.references.push(crate::invwindows::reference(item)),
                Entry::Delete => self.confirm = Some(chosen.clone()),
                // (the sub-menus' own entries are not chosen: they open)
                Entry::Looters | Entry::Team(_) => {}
                looter => {
                    if let Some(ids) = loot_access(looter, chosen.container, chosen.place) {
                        crate::net::raid_command(5, ids);
                    }
                }
            }
        }
        if self.item_popup.is_none() {
            self.item_menu = None;
        }
        // (deleting asks first: `DialogBox_c` titled 10001's "Warning" with `ReallyDeleteItemX` fed the item's name, Yes
        // and No; FUN_100ca997, then FUN_100ca649)
        if let Some(chosen) = self.confirm.clone() {
            let name = view.items.iter().find(|i| i.slot == chosen.place).map_or("", |i| i.name.as_str());
            let text = crate::feedback::texts().and_then(|t| t.named(10000, "ReallyDeleteItemX")).map_or_else(|| format!("Destroy <font color=yellow>{name}</font>? It cannot be brought back."), |t| mao_formats::ldb::format(t, &[mao_formats::ldb::Arg::Text(name)]));
            if let Some(yes) = crate::chrome::question(ctx, palette, "delete-container-item", &crate::gui::xml::ldb_named(10001, "Warning", "Warning"), &text) {
                if yes {
                    take = Some(Take::Delete { handle: chosen.handle, place: chosen.place, low: chosen.low, high: chosen.high });
                }
                self.confirm = None;
            }
        }
        // a backpack's menu and its rename box ("Enter backpack name.", FUN_100cb852: OK, back to its own name, or not)
        let text = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
        // (FUN_100cbace: List Mode with its mark, Auto Arrange with its sub-menu, dead in list mode, and for one's own
        // backpack Rename)
        // (`MAO_LOOT_MENU=1`: the menu with its sub-menu up, for a picture)
        let pictured = std::env::var_os("MAO_LOOT_MENU").is_some() && self.popup.is_none() && ctx.cumulative_frame_nr() < 3;
        if let Some(at) = menu_at.or_else(|| crate::chrome::frame_menu(&name)).or(pictured.then_some(egui::pos2(520.0, 200.0))) {
            use crate::gui::popup::Entry;
            let kind = if view.placed() { Arrange::Own } else { Arrange::Other };
            let (arrange_keys, arrange) = arrange_menu(arranged, kind);
            let mut keys = vec![vec!["list".to_string()], arrange_keys];
            let mut entries = vec![Entry::marked(text("ListMode", "As a list"), true, in_list), Entry::submenu(text("AutoArrange", "Order"), !in_list, arrange)];
            if view.own {
                keys.push(vec!["rename".to_string()]);
                entries.push(Entry::text(text("Rename", "Rename"), true));
            }
            // (the frame's own entry after the view's: FUN_1015b34f)
            keys.push(vec!["close".to_string()]);
            entries.push(Entry::text(crate::chrome::close_entry(), true));
            let mut popup = crate::chrome::menu_of(ctx, at, entries);
            if pictured {
                popup.open_path(&[1], ctx.cumulative_frame_nr());
            }
            self.popup = Some(popup);
            self.window_menu = Some(keys);
        }
        if let Some(crate::chrome::Picked::Entry(path)) = crate::chrome::menu_path(ctx, palette, "backpack-menu", &mut self.popup)
            && let Some(keys) = self.window_menu.take()
            && let Some(chosen) = path.first().and_then(|k| keys.get(*k)).and_then(|k| k.get(path.get(1).copied().unwrap_or(0)))
        {
            match chosen.as_str() {
                "rename" => self.renaming = Some(title.clone()),
                "close" => crate::chrome::close_next(&name),
                "list" => {
                    self.list = Some(!in_list);
                    crate::layout::set_tab(&format!("{key}List"), usize::from(!in_list));
                }
                other => {
                    if let Some(new) = arrange_choice(other, arranged) {
                        // (`SetAutoArrange(false)`: each item stays in the cell the arranging had it in)
                        if new.is_none() && view.placed() {
                            for (cell, id) in crate::gui::multilist::order(tree, list).into_iter().enumerate() {
                                crate::places::put(view.identity, id as u32, cell);
                            }
                        }
                        self.grid_sort = Some(new);
                        crate::layout::set_tab(&format!("{key}GridSort"), new.map_or(0, |s| 1 + (s.0 * 2 + s.1) as usize));
                    }
                }
            }
        }
        if self.popup.is_none() {
            self.window_menu = None;
        }
        if let Some(typed) = self.renaming.as_mut() {
            let mut asking = true;
            let mut answer = None;
            crate::chrome::window(ctx, palette, "rename-backpack", "Enter backpack name.", &mut asking, |w| w.resizable(false).pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.content_rect().center()), |ui| {
                let field = crate::chrome::text_edit(ui, palette, egui::TextEdit::singleline(typed).desired_width(200.0));
                crate::chrome::focus_when_new(ctx, "rename-backpack", &field);
                if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    answer = Some(Some(()));
                }
                ui.horizontal(|ui| {
                    if crate::chrome::button(ui, palette, &text("MsgBox_OK", "OK"), true).clicked() {
                        answer = Some(Some(()));
                    }
                    if crate::chrome::button(ui, palette, &text("Reset2Default", "Its own name"), true).clicked() {
                        typed.clear();
                        answer = Some(Some(()));
                    }
                    if crate::chrome::button(ui, palette, &text("MsgBox_Cancel", "Cancel"), true).clicked() {
                        answer = Some(None);
                    }
                });
            });
            if let Some(Some(())) = answer {
                set_bag_name(view.identity, Some(typed.as_str()).filter(|n| !n.trim().is_empty() && *n != view.title));
            }
            if answer.is_some() || !asking {
                self.renaming = None;
            }
        }
        // (a split asked for by Ctrl+click: how many of the stack; OK has that many in hand, to be put into an empty
        // cell of this backpack)
        {
            let mut asked = SPLIT_BOX.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((_, place, typed)) = asked.as_mut().filter(|b| b.0 == view.identity) {
                let place = *place;
                let count = view.items.iter().find(|i| i.slot == place).map_or(0, |i| i.count as u32);
                if let Some(answer) = crate::invwindows::split_dialog(ctx, palette, "split-stack-bag", typed, count) {
                    if let Some(n) = answer {
                        self.dragging = Some(place);
                        self.split = n;
                        self.taken_up = ctx.cumulative_frame_nr();
                        if crate::gear::traced() {
                            eprintln!("items: {n} of the container's place {place} in hand, to be split off");
                        }
                    }
                    *asked = None;
                }
            }
        }
        self.open = open;
        // (an organization's contract is only looked at here: what the original does with a click on one of its
        // items, or with an item dropped on it, was not read)
        take.filter(|_| view.identity.0 != ORGANIZATION)
    }
}

/// What a click on an item of a container does.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ContainerClick {
    /// The item goes onto the pointer.
    TakeUp,
    /// The item goes into the inventory.
    Take,
    Info,
    /// The box that asks how many of the stack to take off it.
    Split,
    Nothing,
}

/// A click's meaning: by Shift, by whether the container is one's own (a backpack, the bank), by whether nothing is
/// in hand yet, and by `split`: Ctrl held on a stack of more than one of one's own backpack (the list's
/// FUN_10041926: a split, and nothing is taken up). There is no drag: the list tells of a click only
/// (`MultiListView_c::MouseUp`, GUI.dll 0x10133d04).
pub fn container_click(shift: bool, own: bool, hands_free: bool, split: bool) -> ContainerClick {
    match (shift, own) {
        (true, _) => ContainerClick::Info,
        (false, _) if split && hands_free => ContainerClick::Split,
        (false, _) if split => ContainerClick::Nothing,
        (false, true) if hands_free => ContainerClick::TakeUp,
        (false, false) if hands_free => ContainerClick::Take,
        _ => ContainerClick::Nothing,
    }
}

/// The window's title for a container's identity type: a corpse (`0xc76a`), a bag (`0xc749`) or something else.
pub fn title(kind: u32) -> &'static str {
    match kind {
        CORPSE => "Corpse",
        0xc749 => "Bag",
        BANK => "Bank",
        OVERFLOW => "Overflow",
        ORGANIZATION => "Organization contract",
        _ => "Container",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FUN_1004141b with FUN_100ce55d's flag 0x400: a right click takes from a chest or the bank, not from a corpse.
    #[test]
    fn a_right_click_takes_but_not_from_a_corpse() {
        assert!(!right_click_takes(CORPSE));
        assert!(right_click_takes(BAG) && right_click_takes(BANK));
    }

    /// FUN_100ca891: the bank's window going down is told; one that was not up, or that is gone, is not.
    #[test]
    fn the_bank_window_shut_is_told() {
        assert!(shut(true, Some(false)));
        assert!(!shut(true, Some(true)));
        assert!(!shut(true, None));
        assert!(!shut(false, Some(false)));
    }

    /// FUN_100ce55d: another's container and the contract by name; one's own, the bank and the reclaim list not.
    #[test]
    fn a_container_opens_arranged_by_name_or_not() {
        assert_eq!(first_sort(0xc76a, false), Some((1, 0)));
        assert_eq!(first_sort(BAG, false), Some((1, 0)));
        assert_eq!(first_sort(ORGANIZATION, false), Some((1, 0)));
        assert_eq!(first_sort(BAG, true), None);
        assert_eq!(first_sort(BANK, true), None);
        assert_eq!(first_sort(RECLAIM, false), None);
    }

    #[test]
    fn a_backpack_has_its_own_window_and_name() {
        let bag = LootView { title: "Backpack".into(), identity: (BAG, 77), handle: 0x71, items: Vec::new(), own: true, opened: 0 };
        let chest = LootView { title: "Chest".into(), identity: (BAG, 78), handle: 0x72, items: Vec::new(), own: false, opened: 0 };
        let bank = LootView { title: "Bank".into(), identity: (BANK, 0), handle: 0, items: Vec::new(), own: false, opened: 0 };
        assert_eq!((window_name(&bag, None).as_str(), window_name(&chest, Some(2)).as_str(), window_name(&bank, None).as_str()), ("Container_51017x77", "TempContainer_2", "bank_window"));
        // (WIN-023: a container window takes the lowest number no other has)
        assert_eq!((free_number([].into_iter()), free_number([0, 1, 3].into_iter()), free_number([1].into_iter())), (0, 2, 0));
        // (21 places for one's own, as many as there are for a chest or corpse, the bank's 102)
        assert_eq!((max_items(BAG, true), max_items(BAG, false), max_items(BANK, false)), (21, -1, 102));
        // (the names' file: a line a bag, a name may have spaces; a line without a name is none)
        let names: BTreeMap<(u32, u32), String> = [((BAG, 77), "Ammo and kits".to_string()), ((BAG, 9), "Loot".to_string())].into_iter().collect();
        assert_eq!(parse_names(&render_names(&names)), names);
        assert!(parse_names("51017 5\njunk\n51017 x name\n").is_empty());
        // (named, and given its own name back)
        set_bag_name((BAG, 77), Some("  Ammo  "));
        assert_eq!(bag_name((BAG, 77)).as_deref(), Some("Ammo"));
        set_bag_name((BAG, 77), None);
        assert_eq!(bag_name((BAG, 77)), None);
    }

    #[test]
    fn a_click_in_a_container_takes_or_takes_up() {
        // (one's own backpack: onto the pointer; a corpse: into the inventory; Shift: the info)
        assert_eq!(container_click(false, true, true, false), ContainerClick::TakeUp);
        assert_eq!(container_click(false, false, true, false), ContainerClick::Take);
        assert_eq!(container_click(true, true, true, false), ContainerClick::Info);
        assert_eq!(container_click(true, false, false, false), ContainerClick::Info);
        // (nothing with something in hand already, which the click puts down)
        assert_eq!(container_click(false, true, false, false), ContainerClick::Nothing);
        assert_eq!(container_click(false, false, false, false), ContainerClick::Nothing);
        // (Ctrl on a stack of one's own backpack: the split box, with free hands only; Shift still shows the info)
        assert_eq!(container_click(false, true, true, true), ContainerClick::Split);
        assert_eq!(container_click(false, true, false, true), ContainerClick::Nothing);
        assert_eq!(container_click(true, true, true, true), ContainerClick::Info);
    }

    /// An item's menu by its container: look at, reference and delete everywhere but the organization's contract
    /// (no delete there); the looters on a corpse for one in a team. The looters: free for all, random, the team's
    /// members (one the client does not know cannot be chosen), in a raid the teams that have members. What each
    /// looter asks: the corpse, the place with the roll, the looter.
    /// The "Auto Arrange" sub-menu by whose container it is, and what its entries make of the order.
    #[test]
    fn the_arrange_menu_follows_the_container() {
        // (another's container, arranged by name: the three columns, a separator, the order)
        let (keys, entries) = arrange_menu(Some((1, 0)), Arrange::Other);
        assert_eq!(keys, ["by:1", "by:2", "by:4", "", "ascending"]);
        assert_eq!(entries.iter().map(|e| e.label.as_str()).collect::<Vec<_>>(), ["By Name", "By Count", "By Quality", "", "Sort Ascending"]);
        assert_eq!(entries.iter().map(|e| e.mark).collect::<Vec<_>>(), [Some(true), Some(false), Some(false), None, Some(true)]);
        assert!(entries[4].enabled);
        // (one's own, not arranged: "Don't Auto Arrange" marked, the order as it is offered, nothing to turn round)
        let (keys, entries) = arrange_menu(None, Arrange::Own);
        assert_eq!(keys, ["none", "", "by:0", "by:1", "by:2", "by:4", "", "ascending"]);
        assert_eq!((entries[0].mark, entries[2].label.as_str(), entries[2].mark, entries[7].enabled), (Some(true), "Unsorted", Some(false), false));
        // (unsorted has nothing to turn round either)
        let (keys, entries) = arrange_menu(Some((0, 0)), Arrange::Own);
        assert_eq!((keys[2].as_str(), entries[0].mark, entries[2].mark, entries.last().unwrap().enabled), ("by:0", Some(false), Some(true), false));
        // (the bank is placed by hand as one's own backpack is, in its 102 cells)
        let bank = LootView { title: "Bank".into(), identity: (BANK, 1), handle: 0, items: Vec::new(), own: false, opened: 0 };
        let chest = LootView { title: "Chest".into(), identity: (BAG, 78), handle: 0x72, items: Vec::new(), own: false, opened: 0 };
        assert_eq!((bank.placed(), bank.room(), chest.placed()), (true, 102, false));
        // (a column only when it is not the one already; the order kept, but for the unsorted; the order turned)
        assert_eq!(arrange_choice("by:2", Some((1, 1))), Some(Some((2, 1))));
        assert_eq!(arrange_choice("by:1", Some((1, 1))), None);
        assert_eq!(arrange_choice("by:0", Some((1, 1))), Some(Some((0, 0))));
        assert_eq!(arrange_choice("by:4", None), Some(Some((4, 0))));
        assert_eq!(arrange_choice("ascending", Some((2, 0))), Some(Some((2, 1))));
        assert_eq!(arrange_choice("ascending", None), None);
        assert_eq!((arrange_choice("none", Some((1, 0))), arrange_choice("none", None), arrange_choice("", None)), (Some(None), None, None));
    }

    #[test]
    fn an_items_menu_follows_its_container() {
        use Entry::*;
        assert_eq!(item_entries(BAG, false), [LookAt, Reference, Delete]);
        assert_eq!(item_entries(BANK, true), [LookAt, Reference, Delete]);
        assert_eq!(item_entries(OVERFLOW, false), [LookAt, Reference, Delete]);
        assert_eq!(item_entries(ORGANIZATION, true), [LookAt, Reference]);
        assert_eq!(item_entries(CORPSE, false), [LookAt, Reference, Delete]);
        assert_eq!(item_entries(CORPSE, true), [LookAt, Reference, Delete, Looters]);
        let mate = |id: u32, name: &str| crate::team::Mate { id, name: name.to_string(), ..Default::default() };
        let mut team = crate::team::Team { own: 1, members: vec![mate(1, "Own"), mate(2, "Mate")], ..Default::default() };
        let known = |id: u32| id == 1;
        assert_eq!(looter_entries(&team, None, &known), [FreeForAll, RandomLooter, Member(1, "Own".into(), true), Member(2, "Mate".into(), false)]);
        assert_eq!(Member(2, "Mate".into(), false).label(), ("Mate".to_string(), false));
        team.set_place(Some(1));
        team.others[4] = vec![mate(9, "Far")];
        assert_eq!(looter_entries(&team, None, &known), [FreeForAll, RandomLooter, Team(1), Team(4)]);
        assert_eq!(looter_entries(&team, Some(4), &known), [Member(9, "Far".into(), false)]);
        assert_eq!(Team(4).label().0, "Team 5");
        let corpse = (CORPSE, 0x77);
        assert_eq!(loot_access(&FreeForAll, corpse, 0x10003), Some([corpse, (3, 0), (0, 0)]));
        assert_eq!(loot_access(&RandomLooter, corpse, 3), Some([corpse, (3, 1), (0, 0)]));
        assert_eq!(loot_access(&Member(2, "Mate".into(), true), corpse, 3), Some([corpse, (3, 0), (0xc350, 2)]));
        assert_eq!((loot_access(&Delete, corpse, 3), loot_access(&Team(1), corpse, 3)), (None, None));
        // (the menu as the popup shows it: the Looter entry a sub-menu, in a raid its teams sub-menus under it; and the
        // entry a choice's path names)
        let tree = menu_tree(CORPSE, &team, &known);
        let shown = popup_entries(&tree);
        assert_eq!(shown.len(), 4);
        let looters = shown[3].sub.as_ref().unwrap();
        assert_eq!((looters.len(), looters[0].sub.is_none(), looters[2].sub.as_ref().map(Vec::len), looters[3].sub.as_ref().map(|s| (s[0].label.as_str(), s[0].enabled))), (4, true, Some(2), Some(("Far", false))));
        assert_eq!((entry_at(&tree, &[2]), entry_at(&tree, &[3, 1]), entry_at(&tree, &[3, 3, 0]), entry_at(&tree, &[3, 9])), (Some(&Delete), Some(&RandomLooter), Some(&Member(9, "Far".into(), false)), None));
    }

    #[test]
    fn titles_follow_the_container_kind() {
        assert_eq!(title(0xc76a), "Corpse");
        assert_eq!(title(0xc749), "Bag");
        assert_eq!(title(1), "Container");
    }
}
