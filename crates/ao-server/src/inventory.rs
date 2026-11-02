//! The character's items as the client is sent them: `InventoryUpdate` (`4e536976`) with its slots (the schema's `InventorySlot`:
//! placement, flags, count, the bag's identity, the item's low and high template ids, its quality).

use mao_net::n3::Message;

use crate::progress::Item;
use crate::templates::{self, identity, int, list, record};

/// The flags of an item in a slot (the captures' `0x01`: an item, not a bag).
const ITEM_FLAGS: i64 = 0x01;

/// The `InventoryUpdate` that lists `items` for the character (the captured message's other fields kept).
pub fn update(character: u32, items: &[Item]) -> Result<Message, String> {
    let mut t = templates::of(0x4e536976).ok_or("no InventoryUpdate template")?;
    let slots = items
        .iter()
        .map(|i| {
            record(vec![
                ("Placement", int(i.placement as i64)),
                ("Flags", int(ITEM_FLAGS)),
                ("Count", int(i.count as i64)),
                ("Identity", identity((0, 0))),
                ("ItemLowId", int(i.template as i64)),
                ("ItemHighId", int(i.template as i64)),
                ("Quality", int(i.quality as i64)),
                ("Unknown", int(0)),
            ])
        })
        .collect();
    t.set("Items", list(slots));
    t.message(0x4e536976, (mao_net::n3::CHARACTER, character))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_items_are_listed_in_their_slots() {
        let items = vec![Item { placement: 0x40, template: 156020, count: 3, quality: 5 }, Item { placement: 0x41, template: 247123, count: 1, quality: 100 }];
        let m = update(7, &items).unwrap();
        let parsed = mao_net::schema::Schema::global().parse_n3(0x4e536976, &m.data).unwrap();
        let Some(mao_net::schema::Value::List(slots)) = parsed.value.get("Items") else { panic!("no slots") };
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].get("Placement").and_then(|v| v.as_int()), Some(0x40));
        assert_eq!(slots[0].get("ItemLowId").and_then(|v| v.as_int()), Some(156020));
        assert_eq!(slots[1].get("Count").and_then(|v| v.as_int()), Some(1));
    }
}
