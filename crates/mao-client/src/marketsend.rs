//! "Deposit to Market": GUI.dll's `MarketSendWindow_c` (made by FUN_10009dc6 from the skin's `Views/MarketSendWindow.xml`),
//! opened by the market page's `AnarchyOnline.OpenMarketSendWindow` (`BrowserWindow_c::SlotOpenMarketSendWindow`, which
//! wants stat 389's bit 8, the Alien Invasion expansion, else `Feedback_YouMustHaveAlienInvasion`).
//!
//! Its eight `ItemSlotView`s (`ItemView_0` to `ItemView_7`) take items dropped from the backpack (FUN_100097e4: from
//! elsewhere `Feedback_MarketItemMustBeInBackpack`, NoDrop items (stat 0's bit 0x4000000) `Feedback_MarketNoNodrops`,
//! containers `Feedback_MarketNoChests`); `Cash` is the credits. Send (FUN_10009c40): negative credits are
//! `Feedback_MarketNoNegative`; the credits are held to what the character has (stat 61); with items or credits it sends
//! `N3Msg_SendMarketItem` (the `MarketSend` message, [`mao_net::n3::market_send`]) and the window closes.

use crate::gear::{GearItem, GearWindow};
use crate::theme::Palette;

/// NoDrop (stat 0's flag).
const NO_DROP: u32 = 0x400_0000;

/// What Send asks for: the credits and the backpack slots of the items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Deposit {
    pub credits: i32,
    pub slots: [u32; 8],
    pub count: u8,
}

#[derive(Default)]
pub struct MarketSendWindow {
    pub open: bool,
    /// The backpack slot in each of the eight places.
    pub slots: [Option<u32>; 8],
    pub cash: String,
    window: Option<crate::gui::window::XmlWindow>,
}

/// Why an item can't go in (the original's feedback name), if it can't.
pub fn refusal(item: &GearItem) -> Option<&'static str> {
    if item.slot < 64 {
        Some("Feedback_MarketItemMustBeInBackpack")
    } else if item.flags & NO_DROP != 0 {
        Some("Feedback_MarketNoNodrops")
    } else if item.bag {
        Some("Feedback_MarketNoChests")
    } else {
        None
    }
}

/// FUN_10009c40's credits: the field read as a number (empty is 0), none when negative, held to what the character has.
pub fn credits(field: &str, cash: i64) -> Result<i32, &'static str> {
    let v: i64 = field.trim().parse().unwrap_or(0);
    if v < 0 {
        return Err("Feedback_MarketNoNegative");
    }
    Ok(v.min(cash).max(0) as i32)
}

impl MarketSendWindow {
    pub fn show_window(&mut self) {
        self.open = true;
        self.slots = [None; 8];
        self.cash = "0".into();
        if let Some(w) = self.window.as_mut() {
            if let Some(id) = w.tree.find(w.root, "Cash") {
                crate::gui::textinput::set_text(&mut w.tree, id, "0");
            }
        }
    }

    fn load(&mut self, ctx: &egui::Context) {
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        let Some((mut tree, root)) = crate::gui::load_view(&dirs, "MarketSendWindow.xml", "") else { return };
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        self.window = Some(crate::gui::window::XmlWindow { name: "Deposit to Market".into(), title: "Deposit to Market".into(), tree, root, open: true, default_size: egui::vec2(300.0, 0.0) });
    }

    /// Draws the window; the deposit to send, and the feedback to show, if any.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, items: &[GearItem], cash: i64, palette: &Palette) -> (Option<Deposit>, Option<&'static str>) {
        if !self.open {
            return (None, None);
        }
        if self.window.is_none() {
            self.load(ctx);
        }
        let Some(w) = self.window.as_mut() else { return (None, None) };
        let (mut deposit, mut feedback) = (None, None);
        let skin = crate::skin::skin(ctx);
        let root = w.root;
        let places: Vec<Option<crate::gui::view::Id>> = (0..8).map(|i| w.tree.find(root, &format!("ItemView_{i}"))).collect();
        // (the items in the slots)
        for (place, id) in self.slots.iter().zip(&places) {
            let Some(id) = *id else { continue };
            let item = place.and_then(|s| items.iter().find(|it| it.slot == s));
            let shown = item.and_then(|it| it.icon.as_ref().map(|icon| crate::gui::itemslot::SlotItem { texture: gear.texture(ctx, icon), name: it.name.clone() }));
            crate::gui::itemslot::set_item(&mut w.tree, id, shown);
        }
        let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
        let tree = &mut w.tree;
        let slots = &mut self.slots;
        let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
        let mut open = self.open;
        crate::chrome::window(ctx, palette, "Deposit to Market", "Deposit to Market", &mut open, |w| w.default_size(min).resizable(false), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            // (an item from the backpack let go over a slot: FUN_100097e4)
            if dragged == u32::MAX || !ui.input(|inp| inp.pointer.any_released()) {
                return;
            }
            let Some(at) = ui.input(|inp| inp.pointer.interact_pos()) else { return };
            for (place, id) in slots.iter_mut().zip(&places) {
                let Some(id) = *id else { continue };
                if !crate::gui::host::screen_rect(tree, id, response.rect.min).contains(at) {
                    continue;
                }
                if let Some(it) = items.iter().find(|it| it.slot == dragged) {
                    match refusal(it) {
                        Some(why) => feedback = Some(why),
                        None => *place = Some(dragged),
                    }
                }
            }
        });
        if let Some(id) = w.tree.find(root, "Cash") {
            self.cash = crate::gui::textinput::text(&w.tree, id).unwrap_or("").to_string();
        }
        for (id, signal) in std::mem::take(&mut w.tree.signals) {
            if !matches!(signal, crate::gui::host::Signal::Clicked) {
                continue;
            }
            // (an item dragged off its place leaves it: here a click takes it out)
            if let Some(k) = places.iter().position(|p| *p == Some(id)) {
                self.slots[k] = None;
                continue;
            }
            match w.tree.get(id).name.as_str() {
                "SendButton" => match credits(&self.cash, cash) {
                    Err(why) => feedback = Some(why),
                    Ok(c) => {
                        let mut d = Deposit { credits: c, slots: [0; 8], count: 0 };
                        for s in self.slots.iter().flatten() {
                            d.slots[d.count as usize] = *s;
                            d.count += 1;
                        }
                        if d.count > 0 || c != 0 {
                            deposit = Some(d);
                        }
                        open = false;
                    }
                },
                "Close" => open = false,
                _ => {}
            }
        }
        self.open = open;
        (deposit, feedback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposits_follow_the_originals_rules() {
        let item = |slot, flags, bag| GearItem { slot, item_id: 1, high_id: 1, count: 1, quality: 1, bag, flags, name: String::new(), description: String::new(), icon: None, effects: Vec::new(), actions: Vec::new() };
        assert_eq!(refusal(&item(70, 0, false)), None);
        assert_eq!(refusal(&item(6, 0, false)), Some("Feedback_MarketItemMustBeInBackpack"));
        assert_eq!(refusal(&item(70, NO_DROP, false)), Some("Feedback_MarketNoNodrops"));
        assert_eq!(refusal(&item(70, 0, true)), Some("Feedback_MarketNoChests"));
        assert_eq!(credits("", 100), Ok(0));
        assert_eq!(credits("250", 100), Ok(100));
        assert_eq!(credits("-5", 100), Err("Feedback_MarketNoNegative"));
    }
}
