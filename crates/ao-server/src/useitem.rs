//! Using an item out of the backpack (`GenericCmd` Use on `(0x68, slot)`), and the messages an item's arrival and departure
//! take: the 2026-11-08 capture's item-shop lottery ticket (used: "Congratulations!", a prize item put in the backpack, the
//! ticket gone) and its prize (used: "New Victory Points gained.", the prize gone, the Victory Points stat up).
//!
//! * `TemplateAction` (`35505644`): the item's low and high template, its quality, a count, what happened (0x57 added to the
//!   inventory, 0x03 used from a place) and the place (`(0x6e, 0)` the inventory, `(0x68, slot)` a backpack slot).
//! * `ContainerAddItem` (`47537a24`): the inventory `(0x6e, 0)` to the character's inventory `(0x6e, character)`, slot 0x6f.
//! * `CharacterAction` 0x70 from the server: the item at `(0x68, slot)` is gone.
//! * `FormatFeedback` (`206b4b73`): a line for the chat, packed as the game packs its text (a prefix naming the text.mdb line
//!   that shows one string, the string's length plus one, the string).

use mao_net::n3::{Message, CHARACTER};
use mao_net::schema::{Schema, Value};

use crate::templates::{identity, int, record};

/// The item-shop lottery ticket of the capture.
pub const LOTTERY_TICKET: u32 = 303_463;
/// Its prize, a token used for Victory Points.
pub const VICTORY_TOKEN: u32 = 294_604;
/// The Victory Points a token gives (the server's own number: the capture shows the total, not the gain).
pub const TOKEN_POINTS: u32 = 100;
/// The Victory Points stat.
pub const VICTORY_POINTS_STAT: u32 = 0x29d;
/// `TemplateAction`'s words: added to the inventory, and used from a place.
pub const ADDED: u32 = 0x57;
pub const USED: u32 = 0x03;
/// The inventory's identity type, and a backpack place's.
pub const INVENTORY: u32 = 0x6e;
pub const BACKPACK: u32 = 0x68;

/// What using an item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// The item is used up and another is put in the backpack, with a line.
    Prize { template: u32, line: &'static str },
    /// The item is used up for Victory Points, with a line.
    VictoryPoints { points: u32, line: &'static str },
}

/// The effect of using an item of `template`, if the server knows one.
pub fn effect(template: u32) -> Option<Effect> {
    match template {
        LOTTERY_TICKET => Some(Effect::Prize { template: VICTORY_TOKEN, line: "Congratulations!" }),
        VICTORY_TOKEN => Some(Effect::VictoryPoints { points: TOKEN_POINTS, line: "New Victory Points gained." }),
        _ => None,
    }
}

/// A `TemplateAction` about an item: `what` is [`ADDED`] or [`USED`], `place` where it was or went.
pub fn template_action(character: u32, template: (u32, u32), quality: u32, count: u32, what: u32, place: (u32, u32)) -> Result<Message, String> {
    let value = record(vec![
        ("ItemLowId", int(template.0 as i64)),
        ("ItemHighId", int(template.1 as i64)),
        ("Quality", int(quality as i64)),
        ("Unknown1", int(count as i64)),
        ("Unknown2", int(what as i64)),
        ("Placement", identity(place)),
        ("Unknown3", int(0)),
        ("Unknown4", int(0)),
    ]);
    let data = Schema::global().encode_n3(0x35505644, 0, &value)?;
    Ok(Message { kind: 0x35505644, identity: (CHARACTER, character), data })
}

/// The `ContainerAddItem` that follows an item into the inventory.
pub fn container_add(character: u32) -> Result<Message, String> {
    let value = record(vec![("Source", identity((INVENTORY, 0))), ("Target", identity((INVENTORY, character))), ("Slot", int(0x6f))]);
    let data = Schema::global().encode_n3(0x47537a24, 0, &value)?;
    Ok(Message { kind: 0x47537a24, identity: (CHARACTER, character), data })
}

/// The server's `CharacterAction` 0x70: the item at a backpack slot is gone.
pub fn deleted(character: u32, slot: u32) -> Message {
    let mut data = vec![0u8];
    for word in [0x70, 0, BACKPACK, slot, 0, 0] {
        data.extend_from_slice(&word.to_be_bytes());
    }
    data.extend_from_slice(&[0, 0]);
    Message { kind: 0x5e477770, identity: (CHARACTER, character), data }
}

/// A `FormatFeedback` showing `line` (the client unpacks the prefix and takes the string).
pub fn feedback(character: u32, line: &str) -> Result<Message, String> {
    let text: String = line.chars().filter(|c| c.is_ascii() && !c.is_ascii_control()).take(200).collect();
    let mut packed = String::from("~&!!!\":!!!)<s");
    packed.push(char::from(text.len() as u8 + 1));
    packed.push_str(&text);
    let value = record(vec![("Unknown1", int(0)), ("Message", Value::Str(packed)), ("Unknown2", int(0))]);
    let data = Schema::global().encode_n3(0x206b4b73, 1, &value)?;
    Ok(Message { kind: 0x206b4b73, identity: (CHARACTER, character), data })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(m: &Message) -> String {
        m.data.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn the_item_messages_are_the_captured_ones() {
        // (the prize into the inventory, the ticket used from slot 0x5c, the inventory's add, the deletion)
        assert_eq!(hex(&template_action(1, (VICTORY_TOKEN, VICTORY_TOKEN), 1, 1, ADDED, (INVENTORY, 0)).unwrap()), "0000047ecc00047ecc0000000100000001000000570000006e000000000000000000000000");
        assert_eq!(hex(&template_action(1, (LOTTERY_TICKET, LOTTERY_TICKET), 1, 1, USED, (BACKPACK, 0x5c)).unwrap()), "000004a1670004a167000000010000000100000003000000680000005c0000000000000000");
        assert_eq!(hex(&container_add(0x39c16ee4).unwrap()), "000000006e000000000000006e39c16ee40000006f");
        assert_eq!(hex(&deleted(1, 0x5c)), "000000007000000000000000680000005c00000000000000000000");
    }

    #[test]
    fn the_feedback_is_packed_as_the_captured_line() {
        let m = feedback(1, "Congratulations!").unwrap();
        assert_eq!(hex(&m), "0100000000001e7e26212121223a212121293c7311436f6e67726174756c6174696f6e732100000000");
        assert_eq!(mao_net::n3::feedback_text(&m.data).as_deref(), Some("Congratulations!"));
        let m = feedback(1, "New Victory Points gained.").unwrap();
        assert_eq!(mao_net::n3::feedback_text(&m.data).as_deref(), Some("New Victory Points gained."));
    }

    #[test]
    fn the_ticket_gives_a_token_and_the_token_points() {
        assert_eq!(effect(LOTTERY_TICKET), Some(Effect::Prize { template: VICTORY_TOKEN, line: "Congratulations!" }));
        assert!(matches!(effect(VICTORY_TOKEN), Some(Effect::VictoryPoints { points: TOKEN_POINTS, .. })));
        assert_eq!(effect(156_020), None);
    }
}
