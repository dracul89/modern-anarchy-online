//! The vending machines (the Old Athens machine the captures show): opening one sends its stock (`ShopUpdate`) and a trade naming it
//! (`Trade` `Open`); buying from it is a trade's `AddItem` naming the machine and the stock slot. The stock is the captured one.

use mao_net::n3::Message;
use mao_net::schema::Value;

use crate::templates::{self, int, list, record};

/// The identity of the machine the captures show (`VendingMachineFullUpdate`'s).
pub const MACHINE: (u32, u32) = (0xc75b, 0x1324d831);
/// The trade's action numbers (`TradeAction`).
pub const TRADE_OPEN: i64 = 0;
pub const TRADE_COMPLETE: i64 = 4;
pub const TRADE_ADD_ITEM: i64 = 5;
/// The parameter the client's buy names (its `Param3`): the shop's own marker.
pub const BUY_MARKER: u32 = 0x6f;

/// The price of an item, in credits. The original prices by the item's value and the shop's rates (stats 427 and 156), which
/// this server does not have yet: until it does, every item costs the same.
pub const PRICE: u32 = 50;

/// One item the machine sells: the item's low and high template ids and its quality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stock {
    pub low: u32,
    pub high: u32,
    pub quality: u32,
}

/// The machine's stock: the captured one (`ShopUpdate`'s slots).
pub fn stock() -> Vec<Stock> {
    let Some(t) = templates::of(0x58362220) else { return Vec::new() };
    let Some(Value::List(slots)) = t.get("VendingMachineSlots") else { return Vec::new() };
    slots
        .iter()
        .filter_map(|s| {
            Some(Stock { low: s.get("ItemLowId")?.as_int()? as u32, high: s.get("ItemHighId")?.as_int()? as u32, quality: s.get("Quality")?.as_int()? as u32 })
        })
        .collect()
}

/// The `ShopUpdate` for the machine's stock.
pub fn shop_update() -> Result<Message, String> {
    let mut t = templates::of(0x58362220).ok_or("no ShopUpdate template")?;
    let slots = stock().iter().map(|s| record(vec![("ItemLowId", int(s.low as i64)), ("ItemHighId", int(s.high as i64)), ("Quality", int(s.quality as i64))])).collect();
    t.set("VendingMachineSlots", list(slots));
    t.message(0x58362220, t.identity)
}

/// The trade message (`36284f6e`) with `action` and its four parameters, from the captured one.
pub fn trade(character: u32, action: i64, params: [i64; 4]) -> Result<Message, String> {
    let mut t = templates::of(0x36284f6e).ok_or("no Trade template")?;
    t.set("Action", int(action));
    for (i, p) in params.iter().enumerate() {
        t.set(&format!("Param{}", i + 1), int(*p));
    }
    t.message(0x36284f6e, (mao_net::n3::CHARACTER, character))
}

/// The trade that opens the machine: naming it.
pub fn open(character: u32) -> Result<Message, String> {
    trade(character, TRADE_OPEN, [MACHINE.0 as i64, MACHINE.1 as i64, BUY_MARKER as i64, 0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_machine_has_its_captured_stock() {
        let s = stock();
        assert!(s.len() >= 10, "{}", s.len());
        assert_eq!((s[0].low, s[0].high, s[0].quality), (156020, 156021, 5));
    }

    #[test]
    fn the_shop_update_lists_the_stock() {
        let m = shop_update().unwrap();
        let parsed = mao_net::schema::Schema::global().parse_n3(0x58362220, &m.data).unwrap();
        let Some(Value::List(slots)) = parsed.value.get("VendingMachineSlots") else { panic!("no slots") };
        assert_eq!(slots.len(), stock().len());
    }

    #[test]
    fn opening_names_the_machine() {
        let m = open(7).unwrap();
        let parsed = mao_net::schema::Schema::global().parse_n3(0x36284f6e, &m.data).unwrap();
        assert_eq!(parsed.value.get("Param1").and_then(|v| v.as_int()), Some(MACHINE.0 as i64));
        assert_eq!(parsed.value.get("Param2").and_then(|v| v.as_int()), Some(MACHINE.1 as i64));
    }
}
