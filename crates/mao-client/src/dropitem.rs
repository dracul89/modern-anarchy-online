//! An item let go over the ground: GUI.dll's FUN_100cbcb5, the drop of an `inventory/item` drag on no window.
//!
//! What is under the pointer decides. A character takes the item's use on it (`N3Msg_UseItemOnCharacter`, the use
//! message's action 0x20), any other object its use on that (`UseItemOnItem`, action 5), and the ground
//! (`viewer.rs` finds the place) is asked about with `N3Msg_IsDropPossible` (Gamecode 0x10026946, FUN_10048dbd),
//! whose answer picks a box:
//!
//! | answer | box |
//! |---|---|
//! | 2 | the question `Want2DropItemXonGround` (Yes / No) unless the preference `ShowDropItemDialog` is off; Yes is `N3Msg_DropItem` |
//! | 0x12 | `PlaceNonFixture`, "not a fixture, delete it instead?", with Delete and Cancel; Delete is `DeleteItem` |
//! | 0xf | `Feedback_CantDropUnique`, OK |
//! | 0x11 | `Feedback_ThisIsANoDropItem`, OK |
//! | 0xd | `Feedback_AccessingImplants`, OK |
//! | any other | `Feedback_ItemCantBeDroppedAtThatLocation`, OK |
//!
//! [`reason`] is the part of FUN_10048dbd that is ported, in its order: the place more than 5 from the character
//! (8); an item that is no fixture, which is nearly every item (0x12: only one whose `Can` has bit 0 and whose
//! flags have 0x1000 passes); a unique item (0xf); a no-drop one (0x11); else 2. Not ported: the character's state
//! test (0xb), the placing of a fixture in a building (`PlayfieldAnarchy_t::CanAddAnotherFixture`, FUN_10088c4f)
//! and a worn item's own tests (0xd, 0x10), which all come after the fixture test: the server answers those. The
//! distance is taken from the character's place; the original takes it from the character's sphere.
//!
//! The drop itself is `DropTemplateIIR_t` (`mao_net::n3::drop_template`): **it was never captured**, its bytes are
//! the decompile's, and it is said in the log when it goes.

use std::sync::Mutex;

/// The item let go: its identity, its place when it is of the inventory, and what the tests and the boxes need.
#[derive(Clone, Debug)]
pub struct Held {
    pub identity: (u32, u32),
    pub place: Option<u32>,
    pub name: String,
    pub low: u32,
    pub high: u32,
    pub flags: u32,
    pub can: u32,
}

/// What a box's answer asks for.
#[derive(Clone, Debug)]
pub enum Command {
    /// `DeleteItem` of the item.
    Delete(Held),
    /// `N3Msg_DropItem`: the item and the place on the ground (the game's frame: x, height, z).
    Drop { item: (u32, u32), position: [f32; 3] },
    /// A refusal to say (a feedback text's name).
    Refused(&'static str),
}

enum Ask {
    NotFixture(Held),
    Drop(Held, [f32; 3]),
    DropNow(Held, [f32; 3]),
}

static HELD: Mutex<Option<Held>> = Mutex::new(None);
static ASK: Mutex<Option<Ask>> = Mutex::new(None);
/// What the box's answer asked for, until the frame's owner takes it ([`take_command`]).
static ANSWER: Mutex<Option<Command>> = Mutex::new(None);

/// The answer given since the last call.
pub fn take_command() -> Option<Command> {
    ANSWER.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// How far from the character the ground may be (Gamecode's float at 0x101584fc, FUN_1005a367).
pub const REACH: f32 = 5.0;

/// `N3Msg_IsDropPossible`'s answer, as far as it is ported (see the module's text): by the place's distance from the
/// character and the item's flags (stat 0) and `Can` (stat 30).
pub fn reason(distance: f32, flags: u32, can: u32) -> u32 {
    if distance > REACH {
        8
    } else if can & 1 == 0 || flags & 0x1000 == 0 {
        0x12
    } else if flags & 0x8000000 != 0 {
        0xf
    } else if flags & 0x4000000 != 0 {
        0x11
    } else {
        2
    }
}

/// The window's word: this item was let go over the world (what is there is found after).
pub fn let_go(item: Held) {
    *HELD.lock().unwrap_or_else(|e| e.into_inner()) = Some(item);
}

/// The item let go met something other than the ground: no box.
pub fn forget() {
    *HELD.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn feedback(name: &str, own: &str) -> String {
    crate::feedback::texts().and_then(|t| t.named(110, name)).map_or_else(|| own.to_string(), str::to_string)
}

/// The item let go met the ground at `position` (the game's frame), `distance` from the character.
pub fn over_terrain(position: [f32; 3], distance: f32) {
    let Some(item) = HELD.lock().unwrap_or_else(|e| e.into_inner()).take() else { return };
    let answer = reason(distance, item.flags, item.can);
    if crate::gear::traced() {
        eprintln!("items: {:?} let go over the ground at {position:?}, {distance:.1} away: answer {answer:#x}", item.name);
    }
    let ask = match answer {
        2 if crate::prefs::on("ShowDropItemDialog") => Ask::Drop(item, position),
        2 => Ask::DropNow(item, position),
        0x12 => Ask::NotFixture(item),
        0xf => return crate::chrome::alert(feedback("Feedback_CantDropUnique", "A unique item cannot be dropped.")),
        0x11 => return crate::chrome::alert(feedback("Feedback_ThisIsANoDropItem", "This item cannot be dropped.")),
        _ => return crate::chrome::alert(feedback("Feedback_ItemCantBeDroppedAtThatLocation", "The item cannot be dropped there.")),
    };
    *ASK.lock().unwrap_or_else(|e| e.into_inner()) = Some(ask);
}

/// `N3Msg_DropItem`'s own first refusal (Gamecode 0x10027e1f): a worn item is not dropped.
fn drop_of(item: &Held, position: [f32; 3]) -> Command {
    if item.identity.0 < 0x68 {
        Command::Refused("Feedback_MoveItemToInventoryBeforeDropping")
    } else {
        Command::Drop { item: item.identity, position }
    }
}

/// The box that waits, if any, shown with the other boxes of the screen; its answer is kept for [`take_command`].
/// (`MAO_DROP_BOX=fixture|drop` puts one of the two boxes up at the start, for a picture.)
pub fn show(ctx: &egui::Context, palette: &crate::theme::Palette) {
    if let Some(command) = answer(ctx, palette) {
        *ANSWER.lock().unwrap_or_else(|e| e.into_inner()) = Some(command);
    }
}

fn answer(ctx: &egui::Context, palette: &crate::theme::Palette) -> Option<Command> {
    static PICTURE: std::sync::Once = std::sync::Once::new();
    PICTURE.call_once(|| {
        let item = Held { identity: (0x68, 0x41), place: Some(0x41), name: "Solar-Powered Pistol".into(), low: 0, high: 0, flags: 0, can: 0 };
        match std::env::var("MAO_DROP_BOX").as_deref() {
            Ok("fixture") => *ASK.lock().unwrap_or_else(|e| e.into_inner()) = Some(Ask::NotFixture(item)),
            Ok("drop") => *ASK.lock().unwrap_or_else(|e| e.into_inner()) = Some(Ask::Drop(item, [0.0; 3])),
            _ => {}
        }
    });
    let mut ask = ASK.lock().unwrap_or_else(|e| e.into_inner());
    let named = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
    let fed = |text: Option<&str>, own: String, name: &str| text.map_or(own, |t| mao_formats::ldb::format(t, &[mao_formats::ldb::Arg::Text(name)]));
    let command = match ask.as_ref()? {
        Ask::DropNow(item, position) => Some(drop_of(item, *position)),
        Ask::Drop(item, position) => {
            let text = fed(crate::feedback::texts().and_then(|t| t.named(10000, "Want2DropItemXonGround")), format!("Drop {} on the ground?", item.name), &item.name);
            match crate::chrome::question(ctx, palette, "drop-item", &crate::gui::xml::ldb_named(10001, "Warning", "Warning"), &text)? {
                true => Some(drop_of(item, *position)),
                false => None,
            }
        }
        Ask::NotFixture(item) => {
            let text = fed(crate::feedback::texts().and_then(|t| t.named(110, "PlaceNonFixture")), format!("{} is not a fixture and cannot be put there. Delete it instead?", item.name), &item.name);
            match crate::chrome::dialog(ctx, palette, "delete-dropped-item", &named("Error", "Error"), &text, &[&named("MsgBox_Delete", "Delete"), &named("MsgBox_Cancel", "Cancel")])? {
                Some(0) => Some(Command::Delete(item.clone())),
                _ => None,
            }
        }
    };
    *ask = None;
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FUN_10048dbd's order: the distance, the fixture test, unique, no-drop.
    #[test]
    fn the_ground_takes_fixtures_only() {
        // (an ordinary item: no fixture, whatever else it is)
        assert_eq!(reason(2.0, 0, 0x208), 0x12);
        assert_eq!(reason(2.0, 0x4000000, 1), 0x12);
        assert_eq!(reason(2.0, 0x1000, 0), 0x12);
        // (too far comes first)
        assert_eq!(reason(5.5, 0, 0), 8);
        assert_eq!(reason(5.5, 0x1000, 1), 8);
        // (a fixture: unique before no-drop, else it may be dropped)
        assert_eq!(reason(5.0, 0x1000, 1), 2);
        assert_eq!(reason(1.0, 0x1000 | 0x8000000 | 0x4000000, 1), 0xf);
        assert_eq!(reason(1.0, 0x1000 | 0x4000000, 1), 0x11);
    }

    /// `MAO_GAME`: the game's own items by the test. Said in the test's output; few are fixtures.
    #[test]
    fn the_games_items_are_mostly_no_fixtures() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let (mut all, mut droppable, mut named) = (0u32, 0u32, Vec::new());
        for id in db.ids().unwrap().flatten().filter(|id| id.kind == mao_formats::item::TEMPLATE) {
            let Some(item) = db.get(id).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)) else { continue };
            all += 1;
            if reason(0.0, item.stat(0).unwrap_or(0), item.stat(30).unwrap_or(0)) != 0x12 {
                droppable += 1;
                if named.len() < 8 {
                    named.push(id.instance);
                }
            }
        }
        eprintln!("item templates {all}, of which fixtures by the test {droppable} (the first: {named:?})");
        assert!(all > 10000 && droppable * 2 < all, "{droppable} of {all}");
    }

    /// A worn item passes the question and is refused by the drop itself.
    #[test]
    fn a_worn_item_is_not_dropped() {
        let item = |identity| Held { identity, place: None, name: "Chair".into(), low: 1, high: 1, flags: 0x1000, can: 1 };
        assert!(matches!(drop_of(&item((0x66, 0x1b)), [1.0, 2.0, 3.0]), Command::Refused("Feedback_MoveItemToInventoryBeforeDropping")));
        assert!(matches!(drop_of(&item((0x68, 0x41)), [1.0, 2.0, 3.0]), Command::Drop { item: (0x68, 0x41), position: [1.0, 2.0, 3.0] }));
        assert!(matches!(drop_of(&item((0x6b, 0x70006)), [1.0, 2.0, 3.0]), Command::Drop { .. }));
    }
}
