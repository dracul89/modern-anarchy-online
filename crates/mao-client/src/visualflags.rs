//! The character's looks options: the helmet, the four shoulder pad boxes, the two social tab boxes and the PvP title
//! shown. GUI.dll's `MiscOptionsMonitor_c` (constructor FUN_100c05d4) keeps them, and all of them are bits of the
//! character's stat 673, `VisualFlags`:
//!
//! - bits 0 to 6 are the option panel's seven boxes on the value `VisualFlags` (`opt_bitnum`: 0 right shoulder pad,
//!   1 left, 2 the helmet, 3 and 4 the doubled pads, 5 the social tab on, 6 the social tab over the other);
//! - bits 7 and 8 are the value `VisiblePVPTitle` (0 legacy, 1 duel, 2 solo, 3 team);
//! - bit 9 is `/rp`'s (`n3::ROLE_PLAY_BIT`).
//!
//! The server keeps the stat and makes the look from it: the client only asks. At login the monitor sets both values
//! from the stat (the default table has 0x1f for a stat not told, Gamecode FUN_10062e2c), and Gamecode sets
//! `VisualFlags` again whenever the own stat changes (FUN_1005a3b1). A box turned in the option panel changes the
//! value, and the monitor's slot (FUN_100c0427; FUN_100c04c4 for the title, which puts its two bits into the stat's
//! number) hands the new number to `N3Msg_EventFeedback` with the event 0x46 (Gamecode 0x1001cc3d): when it differs
//! from the stat it is sent as the `CharacterAction` 0xa6 with the number as the second identity's
//! (`n3::text_command`, as `/rp`). A change of bit 5, the social tab, is refused while fighting with
//! `Feedback_CantDoThisWhileFighting`, and the value is then put back to the stat's.
//!
//! Until 2026-11-25 the boxes only wrote the number into the preferences and nothing was sent, so none of them did
//! anything. Not ported: what the client itself does with bit 5 when it shows the worn items (FUN_1009cda2); the look
//! is taken from the server's update here.

use crate::gui::dvalue::Store;
use crate::gui::variant::Variant;

/// The stat, and its number when the server has not told it (Gamecode's default table, FUN_10062e2c).
pub const STAT: u32 = 673;
pub const DEFAULT: u32 = 0x1f;
/// The social tab's bit, which is not changed while fighting, and the PvP title's two bits.
const SOCIAL: u32 = 0x20;
const TITLE_SHIFT: u32 = 7;
const TITLE: u32 = 3 << TITLE_SHIFT;

const FLAGS_VALUE: &str = "VisualFlags";
const TITLE_VALUE: &str = "VisiblePVPTitle";

/// What a frame's look at the two values asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// The stat's new number, to send to the server.
    Send(u32),
    /// The social tab's change while fighting: the value is put back, and the original's line is said.
    Refused,
}

/// The monitor's memory: the stat's number as last seen, and the two values as last seen.
#[derive(Default)]
pub struct Monitor {
    stat: Option<u32>,
    flags: u32,
    title: u32,
}

impl Monitor {
    /// One look at the values. `stat` is the character's stat 673 ([`DEFAULT`] when not told), `fighting` whether
    /// something fights the character (stat 410; the original also asks a state of the character's that was not
    /// read, at 0x1001cd0b).
    pub fn step(&mut self, store: &mut Store, stat: u32, fighting: bool) -> Option<Ask> {
        let number = |store: &Store, name: &str| store.get(name).map_or(0, |v| v.as_int32() as u32);
        // (login, or the server changed the stat: the values follow it, and nothing is asked)
        if self.stat != Some(stat) {
            self.stat = Some(stat);
            self.flags = stat;
            self.title = (stat & TITLE) >> TITLE_SHIFT;
            store.set(FLAGS_VALUE, Variant::Int32(self.flags as i32));
            store.set(TITLE_VALUE, Variant::Int32(self.title as i32));
            return None;
        }
        let flags = number(store, FLAGS_VALUE);
        if flags != self.flags {
            self.flags = flags;
            if flags == stat {
                return None;
            }
            if (flags ^ stat) & SOCIAL != 0 && fighting {
                self.flags = stat;
                store.set(FLAGS_VALUE, Variant::Int32(stat as i32));
                return Some(Ask::Refused);
            }
            return Some(Ask::Send(flags));
        }
        let title = number(store, TITLE_VALUE) & 3;
        if title != self.title {
            self.title = title;
            let wanted = stat & !TITLE | title << TITLE_SHIFT;
            return (wanted != stat).then_some(Ask::Send(wanted));
        }
        None
    }

    /// The character is gone (logged out, another one comes): the next look takes the stat anew.
    pub fn forget(&mut self) {
        self.stat = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(flags: i32) -> Store {
        let mut s = Store::default();
        s.load_config(&format!(r#"<Root><Value name="VisualFlags" value="{flags}"/><Value name="VisiblePVPTitle" value="0"/></Root>"#));
        s
    }

    /// At login the values take the stat's number, whatever the preferences said, and nothing is sent.
    #[test]
    fn the_values_follow_the_stat() {
        let mut s = store(31);
        let mut m = Monitor::default();
        assert_eq!(m.step(&mut s, 0x1b | 2 << 7, false), None);
        assert_eq!(s.get("VisualFlags").map(|v| v.as_int32()), Some(0x1b | 2 << 7));
        assert_eq!(s.get("VisiblePVPTitle").map(|v| v.as_int32()), Some(2));
        assert_eq!(m.step(&mut s, 0x1b | 2 << 7, false), None);
        // the server changes it (the answer to a change, or `/rp`): followed, not sent back
        assert_eq!(m.step(&mut s, 0x1f | 2 << 7, false), None);
        assert_eq!(s.get("VisualFlags").map(|v| v.as_int32()), Some(0x1f | 2 << 7));
    }

    /// The helmet's box turned off sends the stat's number without its bit, once; the other bits are kept.
    #[test]
    fn a_box_turned_sends_the_new_number() {
        let mut s = store(31);
        let mut m = Monitor::default();
        let stat = 0x1f | 0x200;
        m.step(&mut s, stat, false);
        s.set_user("VisualFlags", Variant::Int32((stat & !4) as i32));
        assert_eq!(m.step(&mut s, stat, false), Some(Ask::Send(0x21b)));
        assert_eq!(m.step(&mut s, stat, false), None);
        // the server's answer
        assert_eq!(m.step(&mut s, 0x21b, false), None);
        // and back on, while fighting: only the social tab is held back by a fight
        s.set_user("VisualFlags", Variant::Int32(0x21f));
        assert_eq!(m.step(&mut s, 0x21b, true), Some(Ask::Send(0x21f)));
    }

    /// The social tab is not turned while fighting: the value goes back to the stat's.
    #[test]
    fn the_social_tab_is_refused_in_a_fight() {
        let mut s = store(31);
        let mut m = Monitor::default();
        m.step(&mut s, 0x1f, true);
        s.set_user("VisualFlags", Variant::Int32(0x3f));
        assert_eq!(m.step(&mut s, 0x1f, true), Some(Ask::Refused));
        assert_eq!(s.get("VisualFlags").map(|v| v.as_int32()), Some(0x1f));
        assert_eq!(m.step(&mut s, 0x1f, true), None);
        s.set_user("VisualFlags", Variant::Int32(0x3f));
        assert_eq!(m.step(&mut s, 0x1f, false), Some(Ask::Send(0x3f)));
    }

    /// The PvP title's choice goes into bits 7 and 8 of the stat's number.
    #[test]
    fn the_title_is_two_bits_of_the_stat() {
        let mut s = store(31);
        let mut m = Monitor::default();
        m.step(&mut s, 0x1b, false);
        s.set_user("VisiblePVPTitle", Variant::Int32(3));
        assert_eq!(m.step(&mut s, 0x1b, false), Some(Ask::Send(0x19b)));
        assert_eq!(m.step(&mut s, 0x19b, false), None);
        s.set_user("VisiblePVPTitle", Variant::Int32(0));
        assert_eq!(m.step(&mut s, 0x19b, false), Some(Ask::Send(0x1b)));
    }
}
