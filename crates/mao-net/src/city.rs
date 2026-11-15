//! A city's controller: the signals the server sends about a city and the requests its terminal window makes.
//!
//! The controller (a `CityTerminal_t`, identity type 0xc418, a plain item of the playfield's table) is used as any
//! terminal is. The server answers with `AOTransportSignalIIR_c` code 5, "open a GUI session", which brings up
//! GUI.dll's `CityTerminalWindow_c`; while the session lasts (till code 7) it sends the lease mode, the upkeep, the
//! time to the next bill, the cloaking device's state, the controller's charge and the owner's name by the same
//! message. The houses' changes (codes 2, 8, 9) come by it too, to everyone in the playfield. The whole reading is
//! `docs/CITY_CONTROLLER_ORIGINAL.md`.
//!
//! The signal (Gamecode's reader FUN_1012faab, dispatch FUN_1012fdaa): after the lead byte a code, then the code's
//! payload. A `CityID_t` (city.dll 0x100114a9) is five words: the city's identity (`(0xc419, n)`, `n` the high half of
//! the controller's instance: 0xc000 in playfield 6010), the playfield's instance and the playfield model's identity.
//!
//! The requests each carry the city's number (the second word of its identity) first; all are sent with the
//! character's identity and a zero lead byte. None asks for confirmation in the original, and none can be taken back.

use crate::frame::Frame;
use crate::landcontrol::House;
use crate::n3::{Message, CHARACTER};

/// `AOTransportSignalIIR_c`.
pub const TRANSPORT_SIGNAL: u32 = 0x6274_1e15;
/// `ClientRequestBuyIIR_c`, `ClientRequestBuildIIR_c`, `ClientRequestDemolishIIR_c`, `ClientRqToggleCloakingIIR_c`,
/// `ClientRequestCloseGUIIIR_c`.
pub const REQUEST_BUY: u32 = 0x5579_6602;
pub const REQUEST_BUILD: u32 = 0x5301_1416;
pub const REQUEST_DEMOLISH: u32 = 0x3f1b_6f70;
pub const REQUEST_TOGGLE_CLOAK: u32 = 0x3f5e_4b46;
pub const REQUEST_CLOSE: u32 = 0x1b3c_614d;
/// `CityAdvantagesIIR_t`.
pub const ADVANTAGES: u32 = 0x365e_555b;
/// The organisation command that lists the city's advantages (`N3Msg_OrgShowCity`, Gamecode 0x1001a8be).
pub const ORG_SHOW_CITY: u8 = 0x1f;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CityId {
    pub city: (u32, u32),
    pub playfield: u32,
    pub model: (u32, u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Signal {
    /// 2: a house was built (its record, as in the playfield's list).
    HouseAdded(House),
    /// 8: the demolishing of the house on this tile began.
    DemolishStarted(i32, i32),
    /// 9: the house on this tile is gone.
    HouseRemoved(i32, i32),
    /// 5: the terminal's window is to open. `gui_type` 1 is the dialog that buys the land, 2 the terminal;
    /// `lease` 1 shows the city only, 3 lets everything be done, another shows without doing; `third` is a word the
    /// window lets build and demolish by when it is 0 or 1 (it reads like a rank in the organisation: a guess).
    /// Nothing reads `identity`.
    Open { city: CityId, identity: (u32, u32), gui_type: i32, lease: i32, third: i32, org: String },
    /// 6.
    Lease(i32),
    /// 7: the session is over, the window goes.
    Close(CityId),
    /// 10: the city's upkeep.
    Upkeep(i32),
    /// 13: the seconds to the next bill.
    BillIn(i32),
    /// 14: the cloaking device: on above 0, off below, none with 0; it may be switched when `time` is below 0.
    Cloak { state: i32, time: i32 },
    /// 15: the controller's charge, 0 to 1.
    Charge(f32),
    /// 16: the owner's name.
    OrgName(String),
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn word(&mut self) -> Option<u32> {
        let (head, rest) = self.0.split_at_checked(4)?;
        self.0 = rest;
        Some(u32::from_be_bytes(head.try_into().unwrap()))
    }

    fn int(&mut self) -> Option<i32> {
        self.word().map(|w| w as i32)
    }

    fn identity(&mut self) -> Option<(u32, u32)> {
        Some((self.word()?, self.word()?))
    }

    fn city(&mut self) -> Option<CityId> {
        Some(CityId { city: self.identity()?, playfield: self.word()?, model: self.identity()? })
    }

    /// A text as Gamecode's FUN_10038af8 reads it: a 16-bit length below 0x8000 and the bytes.
    fn text(&mut self) -> Option<String> {
        let (head, rest) = self.0.split_at_checked(2)?;
        let length = u16::from_be_bytes(head.try_into().unwrap()) as usize;
        if length >= 0x8000 {
            return None;
        }
        let (text, rest) = rest.split_at_checked(length)?;
        self.0 = rest;
        Some(String::from_utf8_lossy(text).into_owned())
    }
}

/// A [`TRANSPORT_SIGNAL`]'s body (from its lead byte); `None` for a code not known here or a payload cut short.
pub fn signal(data: &[u8]) -> Option<Signal> {
    let mut r = Reader(data.get(1..)?);
    Some(match r.int()? {
        2 => Signal::HouseAdded(House::parse(r.0.get(..House::SIZE)?)?),
        8 => Signal::DemolishStarted(r.int()?, r.int()?),
        9 => Signal::HouseRemoved(r.int()?, r.int()?),
        5 => Signal::Open { city: r.city()?, identity: r.identity()?, gui_type: r.int()?, lease: r.int()?, third: r.int()?, org: r.text()? },
        6 => Signal::Lease(r.int()?),
        7 => Signal::Close(r.city()?),
        10 => Signal::Upkeep(r.int()?),
        13 => Signal::BillIn(r.int()?),
        14 => Signal::Cloak { state: r.int()?, time: r.int()? },
        15 => Signal::Charge(f32::from_bits(r.word()?)),
        16 => Signal::OrgName(r.text()?),
        _ => return None,
    })
}

fn request(kind: u32, character: u32, words: &[u32]) -> Frame {
    let mut data = vec![0u8];
    for word in words {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Message { kind, identity: (CHARACTER, character), data }.frame(character)
}

/// The window's close request (GUI.dll FUN_100e7203): the server answers with code 7, and the window goes then.
pub fn close_session(character: u32, city: u32) -> Frame {
    request(REQUEST_CLOSE, character, &[city])
}

/// The cloaking device switched the other way (GUI.dll FUN_100e72a2 → `CityAI_c::ClientRequestToggleCloakingDevice`).
pub fn toggle_cloak(character: u32, city: u32) -> Frame {
    request(REQUEST_TOGGLE_CLOAK, character, &[city])
}

/// The house on a tile to be demolished (GUI.dll FUN_100e48bb; the writer Gamecode FUN_10130312: the city, then the
/// tile's x and z).
pub fn demolish(character: u32, city: u32, tile: (i32, i32)) -> Frame {
    request(REQUEST_DEMOLISH, character, &[city, tile.0 as u32, tile.1 as u32])
}

/// A house to be built (GUI.dll FUN_100e498a; the writer Gamecode FUN_10130053): the city, the house's template
/// (`(0xf4268, n)`), the tile, the turn in degrees (0, 90, 180, 270) and the building item of the inventory.
pub fn build(character: u32, city: u32, template: (u32, u32), tile: (i32, i32), rotation: i32, item: (u32, u32)) -> Frame {
    request(REQUEST_BUILD, character, &[city, template.0, template.1, tile.0 as u32, tile.1 as u32, rotation as u32, item.0, item.1])
}

/// The land bought (GUI.dll FUN_100e6225).
pub fn buy(character: u32, city: u32) -> Frame {
    request(REQUEST_BUY, character, &[city])
}

/// One of a city's advantages ([`advantages`]): the item that gives it and whether it is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Advantage {
    pub low: u32,
    pub high: u32,
    pub quality: u32,
    pub active: bool,
}

/// A [`ADVANTAGES`] message's body (Gamecode's reader FUN_10038cb9): a plain count, then four words an entry.
pub fn advantages(data: &[u8]) -> Option<Vec<Advantage>> {
    let mut r = Reader(data.get(1..)?);
    let count = r.word()? as usize;
    if r.0.len() < count.checked_mul(16)? {
        return None;
    }
    (0..count).map(|_| Some(Advantage { low: r.word()?, high: r.word()?, quality: r.word()?, active: r.word()? != 0 })).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::map_to_key;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn the_numbers_are_the_class_names() {
        assert_eq!(map_to_key("AOTransportSignalIIR_c"), TRANSPORT_SIGNAL);
        assert_eq!(map_to_key("ClientRequestBuyIIR_c"), REQUEST_BUY);
        assert_eq!(map_to_key("ClientRequestBuildIIR_c"), REQUEST_BUILD);
        assert_eq!(map_to_key("ClientRequestDemolishIIR_c"), REQUEST_DEMOLISH);
        assert_eq!(map_to_key("ClientRqToggleCloakingIIR_c"), REQUEST_TOGGLE_CLOAK);
        assert_eq!(map_to_key("ClientRequestCloseGUIIIR_c"), REQUEST_CLOSE);
        assert_eq!(map_to_key("CityAdvantagesIIR_t"), ADVANTAGES);
    }

    /// Made up to the original's readers: no capture has one of these.
    #[test]
    fn the_signals_read() {
        let city = "0000c4190000c0000020e166" .to_string() + "0000c79e0000177a";
        let id = CityId { city: (0xc419, 0xc000), playfield: 0x20e166, model: (0xc79e, 0x177a) };
        let open = hex(&format!("0000000005{city}0000c3504deb97f9000000020000000300000000000641206e616d65"));
        assert_eq!(signal(&open), Some(Signal::Open { city: id, identity: (0xc350, 0x4deb97f9), gui_type: 2, lease: 3, third: 0, org: "A name".into() }));
        assert_eq!(signal(&open[..open.len() - 1]), None);
        assert_eq!(signal(&hex(&format!("0000000007{city}"))), Some(Signal::Close(id)));
        assert_eq!(signal(&hex("000000000600000001")), Some(Signal::Lease(1)));
        assert_eq!(signal(&hex("000000000a00e4e1c0")), Some(Signal::Upkeep(15_000_000)));
        assert_eq!(signal(&hex("000000000d00015180")), Some(Signal::BillIn(86400)));
        assert_eq!(signal(&hex("000000000e00000001ffffffff")), Some(Signal::Cloak { state: 1, time: -1 }));
        assert_eq!(signal(&hex("000000000f3f000000")), Some(Signal::Charge(0.5)));
        assert_eq!(signal(&hex("000000001000024f6b")), Some(Signal::OrgName("Ok".into())));
        assert_eq!(signal(&hex("0000000008000000d900000140")), Some(Signal::DemolishStarted(217, 320)));
        assert_eq!(signal(&hex("0000000009000000d900000140")), Some(Signal::HouseRemoved(217, 320)));
        let added = signal(&hex("00000000024459000040a0000044a000000000010e00000f4268006a0003")).unwrap();
        assert_eq!(added, Signal::HouseAdded(House { position: [868.0, 5.0, 1280.0], rotation: 270, flag: false, template: (0xf4268, 0x6a0003) }));
        assert_eq!(signal(&hex("0000000063")), None);
    }

    #[test]
    fn the_requests_carry_the_city_first() {
        let body = |f: Frame, n: usize| f.body[f.body.len() - n..].to_vec();
        assert_eq!(body(close_session(7, 0xc000), 5), hex("000000c000"));
        assert_eq!(body(toggle_cloak(7, 0xc000), 5), hex("000000c000"));
        assert_eq!(body(buy(7, 0xc000), 5), hex("000000c000"));
        assert_eq!(body(demolish(7, 0xc000, (217, 320)), 13), hex("000000c000000000d900000140"));
        assert_eq!(body(build(7, 0xc000, (0xf4268, 0x6a0003), (217, 320), 270, (0x68, 0x41)), 33), hex("000000c000000f4268006a0003000000d9000001400000010e0000006800000041"));
        let f = close_session(7, 0xc000);
        assert_eq!(&f.body[..12], &hex("1b3c614d0000c35000000007")[..]);
    }

    #[test]
    fn the_advantages_read() {
        let list = advantages(&hex("0000000002000000010000000200000064000000010000000300000004000000c800000000")).unwrap();
        assert_eq!(list, [Advantage { low: 1, high: 2, quality: 100, active: true }, Advantage { low: 3, high: 4, quality: 200, active: false }]);
        assert_eq!(advantages(&hex("000000000200000001")), None);
    }
}
