//! The shop window: GUI.dll's `TradeView_c` (FUN_100e121e) in its shop mode, the skin's `Views/TradeGUI.xml` view
//! `ShopTrade` (FUN_100e1074 picks `PlayerTrade`, `ShopTrade` or `ShopBuy` by the mode). Its three docks hold item lists
//! (`ItemContainerView_c`, FUN_100ce55d: columns Icon, Name, Count, Price and Quality): `ShopInventoryDock` the shop's stock
//! (FUN_100e07b9), `PartnerInventoryDock` "Bought items" (FUN_100e0942) and `OurInventoryDock` "Sold items" (FUN_100e0b00);
//! `PartnerCashView` and `OurCashView` show what each side's items cost (`N3Msg_TradeGetInventoryCost`), `AcceptButton`
//! and `DeclineButton` end the trade (FUN_100e0c89).
//!
//! The messages are the captures' (`capture-221539.log`, `capture-225436.log`): using a booth (`GenericCmd` Use on
//! `(0xc75b, serial)`) brings the stock (`ShopUpdate`: `(low id, high id, quality)` each) and a `Trade` open; an item is
//! put in with `Trade` action 5 (`AddItem`: the stock item `(booth, 0x6f, index)` or one of your own `(yourself, 0x68,
//! slot)`) and taken out with action 6 (`RemoveItem`), and the server echoes both, which is when the item shows in the
//! list; action 1 accepts, the server answers action 4 (complete), and the booth is used again for the next trade.
//! Decline is action 2.
//!
//! The prices are the client's own (no message carries them), Gamecode's `VendingMachine_t`: a stock item costs its value
//! (stat 74 at its quality, [`value_at`]) times the shop's stat 427 / 100, less a Computer Literacy discount ([`buy_price`],
//! FUN_10099e9b); your items fetch their value times the shop's stat 426 / 100, plus a Computer Literacy bonus
//! ([`sell_price`], FUN_1009a12d). The shop's stats are its template's (`StaticInstance`).

use std::sync::Arc;

use crate::gear::{GearItem, GearWindow, Icon};
use crate::theme::Palette;

/// An item's stat read as none (the original's 1234567890).
const NO_VALUE: u32 = 1_234_567_890;
/// The shop type (stat 156) whose prices follow Computer Literacy: the default when the shop has none.
const COMPUTER_LITERACY_SHOP: i64 = 0x3d;

/// One item for sale, with what it costs this character.
pub struct Slot {
    pub low: u32,
    pub high: u32,
    pub quality: u32,
    pub name: String,
    pub description: String,
    pub icon: Option<Arc<Icon>>,
    pub price: i64,
}

/// A row of the bought or sold list: the item as the trade names it (the `AddItem` parameters), with what shows.
#[derive(Clone)]
pub struct Row {
    pub item: [u32; 4],
    pub name: String,
    pub icon: Option<Arc<Icon>>,
    pub count: u32,
    pub quality: u32,
    pub price: i64,
}

/// What changed in the trade (the server's echoes).
#[derive(Clone)]
pub enum Change {
    /// A stock item put in (`PartnerInventoryDock`, "Bought items").
    Bought(Row),
    /// One of your own items put in (`OurInventoryDock`, "Sold items").
    Sold(Row),
    /// An item taken out.
    Removed([u32; 4]),
    /// The trade went through.
    Complete,
    /// A trade with another player opened (`Trade` action 0 naming them): `TradeView_c` in its player mode.
    PlayerOpen { partner: u32, name: String },
    /// The partner's credits (action 7 naming them).
    PartnerCredits(i64),
    /// Your side confirmed (action 3 naming you), or the partner's confirmed or accepted (3 or 1 naming them).
    Confirmed { ours: bool },
    /// The trade was declined (action 2).
    Declined,
}

/// What the window asks the network thread to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopCommand {
    /// Put the stock item at this index in (`N3Msg_TradeAddItem`).
    Buy(usize),
    /// Put the inventory item in this slot in.
    Sell(u32),
    /// Take an item out (`N3Msg_TradeRemoveItem`).
    Remove([u32; 4]),
    /// `N3Msg_TradeAccept`.
    Accept,
    /// Close the shop (`N3Msg_TradeAbort`: a trade decline).
    Close,
    /// `N3Msg_TradeSetCash`: the credits you put in (a player trade).
    Credits(i64),
    /// `N3Msg_TradeConfirm`: a player trade's Accept button (the final accept follows the "Are you sure" dialog).
    Confirm,
}

/// A shop's pricing stats: 427 (what it sells at, in percent of the value), 426 (what it buys at) and 156 (its type).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rates {
    pub buy: i64,
    pub sell: i64,
    pub kind: i64,
}

impl Rates {
    /// From the shop's template (the original's `GetSkill` falls back to it): a type of 0 or none is 61.
    pub fn of(template: &mao_formats::item::Item) -> Rates {
        let stat = |s: u32| template.stat(s).map(|v| v as i32 as i64);
        Rates { buy: stat(427).unwrap_or(0), sell: stat(426).unwrap_or(0), kind: stat(156).filter(|&k| k != 0).unwrap_or(COMPUTER_LITERACY_SHOP) }
    }
}

/// A shop that was opened: the booth, its name and its stock.
pub struct ShopState {
    pub shop: (u32, u32),
    pub name: String,
    pub stock: Vec<Slot>,
}

/// FUN_100cb909: a value between the low and high templates' at a quality, on the line between their qualities
/// (rounded); the low one's when the values or qualities agree, the high one's at its own quality.
fn interpolate(low: i64, high: i64, ql_low: u16, ql_high: u16, ql: u16) -> i64 {
    if low == high || ql_low == ql_high || ql_low == ql {
        return low;
    }
    if ql_high == ql {
        return high;
    }
    let (ql, ql_low, ql_high) = (ql as f64, ql_low as f64, ql_high as f64);
    if low < high {
        ((ql - ql_low) * (high - low) as f64 / (ql_high - ql_low) + 0.5) as i64 + low
    } else {
        ((ql - ql_high) * (low - high) as f64 / (ql_low - ql_high) + 0.5) as i64 + high
    }
}

/// FUN_100cbb68: a value moved onto the curve between the two (`t * t` of the way along, in single precision).
fn curve(v: i64, a: i64, b: i64) -> i64 {
    if a == b || v == a || v == b {
        return v;
    }
    let (lo, hi) = (a.min(b) as f32, a.max(b) as f32);
    let range = hi - lo;
    if range == 0.0 {
        return v;
    }
    let t = (v as f32 - lo) / range;
    (t as f64 * t as f64 * range as f64 + lo as f64) as i64
}

/// An item's value at a quality (FUN_10099991 for stat 74): the low and high templates' values (none reads 0) between
/// their qualities (stat 54), then onto the curve.
pub fn value_at(low: &mao_formats::item::Item, high: &mao_formats::item::Item, ql: u32) -> i64 {
    let value = |t: &mao_formats::item::Item| t.stat(74).filter(|&v| v != NO_VALUE).map_or(0, |v| v as i32 as i64);
    let (a, b) = (value(low), value(high));
    let (ql_low, ql_high) = (low.stat(54).unwrap_or(1), high.stat(54).unwrap_or(1));
    let mut v = a;
    if ql_low != ql_high {
        v = interpolate(a, b, ql_low as u16, ql_high as u16, ql as u16);
    }
    curve(v, a, b)
}

/// The Computer Literacy points the prices use: a tenth of the skill (up to 3000) as a quarter.
fn literacy(computer_literacy: i64) -> i64 {
    (computer_literacy.min(3000) as f64 / 10.0 * 0.25) as i64
}

/// FUN_10099e9b: what a stock item of this value costs: the shop's rate of it, less the Computer Literacy points (at most
/// 90) in percent at a shop of type 61.
pub fn buy_price(value: i64, rates: Rates, computer_literacy: i64) -> i64 {
    let price = (value as f64 * rates.buy as f64 / 100.0) as i64;
    if rates.kind != COMPUTER_LITERACY_SHOP {
        return price;
    }
    let discount = literacy(computer_literacy).min(90);
    price + (discount as f64 * price as f64 / -100.0) as i64
}

/// FUN_1009a12d: what a stack of an item of this value fetches: the value (in single precision, divided by the item's
/// stat 212 when it has one), plus the Computer Literacy points in percent at a shop of type 61, times the count and the
/// shop's rate.
pub fn sell_price(value: i64, divisor: Option<u32>, count: u32, rates: Rates, computer_literacy: i64) -> i64 {
    let mut v = value as f32;
    if let Some(d) = divisor.filter(|&d| d != NO_VALUE && (d as i32) > 0) {
        v /= d as f32;
    }
    if rates.kind == COMPUTER_LITERACY_SHOP {
        let bonus = (literacy(computer_literacy) as f64 * v as f64 / 100.0) as i64;
        v = (bonus as f64 + v as f64) as f32;
    }
    (rates.sell as f64 * (count as f64 * v as f64) / 100.0) as i64
}

fn template(db: &mao_formats::resource::ResourceDatabase, id: u32) -> Option<mao_formats::item::Item> {
    db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: id }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d))
}

/// A stock item as the window shows it: its template's name, description and icon, and its price here.
pub fn slot(db: &mao_formats::resource::ResourceDatabase, icon: &dyn Fn(u32) -> Option<Arc<Icon>>, (low, high, quality): (u32, u32, u32), rates: Rates, computer_literacy: i64) -> Slot {
    let (lo, hi) = (template(db, low), template(db, high));
    let price = match (&lo, hi.as_ref().or(lo.as_ref())) {
        (Some(l), Some(h)) => buy_price(value_at(l, h, quality), rates, computer_literacy),
        _ => 0,
    };
    match lo.or(hi) {
        Some(t) => Slot { low, high, quality, icon: t.icon().and_then(icon), name: t.name, description: t.description, price },
        None => Slot { low, high, quality, name: format!("Item #{low:x}"), description: String::new(), icon: None, price },
    }
}

/// One of your own items put in, as the sold list shows it.
pub fn sold_row(db: &mao_formats::resource::ResourceDatabase, icon: &dyn Fn(u32) -> Option<Arc<Icon>>, item: [u32; 4], entry: &mao_net::inventory::Entry, rates: Rates, computer_literacy: i64) -> Row {
    let (lo, hi) = (template(db, entry.low_id), template(db, entry.high_id));
    let count = entry.count.max(1) as u32;
    let price = match (&lo, hi.as_ref().or(lo.as_ref())) {
        (Some(l), Some(h)) => sell_price(value_at(l, h, entry.quality), l.stat(212), count, rates, computer_literacy),
        _ => 0,
    };
    let (name, icon) = lo.or(hi).map_or_else(|| (format!("Item #{:x}", entry.low_id), None), |t| (t.name.clone(), t.icon().and_then(icon)));
    Row { item, name, icon, count, quality: entry.quality, price }
}

/// The window: the trade view's tree from the skin and the lists it shows.
#[derive(Default)]
pub struct ShopWindow {
    pub open: bool,
    pub shop: (u32, u32),
    pub name: String,
    pub stock: Vec<Slot>,
    pub bought: Vec<Row>,
    pub sold: Vec<Row>,
    pub selected: Option<usize>,
    pub status: String,
    /// A stock item's page for the Info window (right-clicked: `itemref://low/high/ql`), until taken.
    pub info_url: Option<String>,
    window: Option<crate::gui::window::XmlWindow>,
    /// A trade with another player (`TradeView_c`'s mode 0, the skin's `PlayerTrade` view): the partner, the credits they put
    /// in, whether each side has confirmed, and whether "Are you sure you want to complete this trade?" is up.
    pub partner: Option<u32>,
    pub partner_credits: i64,
    pub confirmed: (bool, bool),
    asking: bool,
    /// The view the window was built from (`ShopTrade` or `PlayerTrade`).
    view: &'static str,
    /// The credits last sent, and a confirm waiting for the frame after them.
    credits_sent: i64,
    pending_confirm: bool,
}

/// The docks' heights (the item lists GUI.dll puts in them size them; here they are fixed).
const DOCKS: [(&str, f32); 3] = [("ShopInventoryDock", 176.0), ("PartnerInventoryDock", 84.0), ("OurInventoryDock", 84.0)];
/// A list row's height (the icon's).
const ROW: f32 = 26.0;

impl ShopWindow {
    pub fn set(&mut self, state: ShopState) {
        self.partner = None;
        self.shop = state.shop;
        self.name = state.name;
        self.stock = state.stock;
        self.bought.clear();
        self.sold.clear();
        self.open = true;
        self.selected = None;
        self.status.clear();
    }

    /// The server's echo of an item put in or taken out, or the end of the trade.
    pub fn apply(&mut self, change: Change) {
        match change {
            Change::Bought(row) => self.bought.push(row),
            Change::Sold(row) => self.sold.push(row),
            Change::Removed(item) => {
                self.bought.retain(|r| r.item != item);
                self.sold.retain(|r| r.item != item);
            }
            Change::Complete => {
                self.bought.clear();
                self.sold.clear();
                self.status = "Trade complete.".into();
                // (a player trade's window closes when it goes through)
                if self.partner.take().is_some() {
                    self.open = false;
                    self.asking = false;
                }
            }
            Change::PlayerOpen { partner, name } => {
                *self = ShopWindow { partner: Some(partner), name, open: true, window: self.window.take(), view: self.view, ..Default::default() };
            }
            Change::PartnerCredits(credits) => self.partner_credits = credits,
            Change::Confirmed { ours } => {
                if ours {
                    self.confirmed.0 = true;
                } else {
                    self.confirmed.1 = true;
                }
                // (both sides done: FUN_100e0f5b's "TradeDoubleConfirmDlg")
                self.asking = self.confirmed.0 && self.confirmed.1;
            }
            Change::Declined => {
                if self.partner.take().is_some() {
                    self.open = false;
                    self.asking = false;
                }
            }
        }
    }

    /// FUN_100e1074 and the docks: `ShopTrade` (or, trading with a player, `PlayerTrade`) of the skin's `TradeGUI.xml`, the
    /// docks and the fields given room.
    fn load(&mut self, ctx: &egui::Context) {
        let view = if self.partner.is_some() { "PlayerTrade" } else { "ShopTrade" };
        self.view = view;
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        let Some((mut tree, root)) = crate::gui::load_view(&dirs, "TradeGUI.xml", view) else { return };
        // (the text measured from the start, so the first layout gives the labels their width)
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        for (dock, height) in DOCKS {
            if let Some(id) = tree.find(root, dock) {
                tree.get_mut(id).min_size = crate::gui::geom::Point::new(330.0, height);
            }
        }
        for field in ["PartnerName", "PartnerCashView", "OurCashView"] {
            if let Some(id) = tree.find(root, field) {
                let node = tree.get_mut(id);
                node.min_size = crate::gui::geom::Point::new(node.min_size.x.max(90.0), 16.0);
            }
        }
        // (the partner's status beside the buttons: GUI.dll fills the dock with a view of its own)
        if let Some(id) = tree.find(root, "PartnerStatusDock") {
            tree.get_mut(id).min_size = crate::gui::geom::Point::new(110.0, 16.0);
        }
        self.window = Some(crate::gui::window::XmlWindow { name: "Shop".into(), title: "Shop".into(), tree, root, open: true, default_size: egui::vec2(360.0, 0.0) });
    }

    /// Draws the window; returns what to send, if anything.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, _items: &[GearItem], palette: &Palette) -> Option<ShopCommand> {
        let wanted = if self.partner.is_some() { "PlayerTrade" } else { "ShopTrade" };
        if self.window.is_none() || self.view != wanted {
            self.load(ctx);
        }
        let player = self.partner.is_some();
        // ("Are you sure you want to complete this trade?": yes accepts, no aborts, FUN_100e0132)
        let mut answer = None;
        if self.asking {
            match crate::chrome::dialog(ctx, palette, "TradeDoubleConfirmDlg", "Trade", "Are you sure you want to complete this trade?", &crate::chrome::yes_no().each_ref().map(String::as_str)) {
                Some(Some(k)) => answer = Some(k == 0),
                // (closed, or Escape: `SlotSelected(-1)`, which FUN_100e0132 ignores; the trade stays open)
                Some(None) => self.asking = false,
                None => {}
            }
        }
        if let Some(yes) = answer {
            self.asking = false;
            if !yes {
                self.partner = None;
                self.open = false;
            }
            return Some(if yes { ShopCommand::Accept } else { ShopCommand::Close });
        }
        let mut command = None;
        let accent = egui::Color32::from_rgb(palette.accent[0], palette.accent[1], palette.accent[2]);
        let text = egui::Color32::from_rgb(palette.text[0], palette.text[1], palette.text[2]);
        let green = egui::Color32::from_rgb(0x44, 0xdd, 0x44);
        let w = self.window.as_mut()?;
        let skin = crate::skin::skin(ctx);
        let min = crate::gui::host::preferred_pixels(&w.tree, w.root, false);
        let root = w.root;
        let tree = &mut w.tree;
        let (stock, bought, sold, selected, info_url) = (&self.stock, &self.bought, &self.sold, &mut self.selected, &mut self.info_url);
        let name = if self.name.is_empty() { format!("Booth {:x}", self.shop.1 & 0xffff) } else { self.name.clone() };
        let (partner_credits, confirmed) = (self.partner_credits, self.confirmed);
        // (the partner's credits in their field, `PartnerCashView`, which the player does not type in)
        if self.partner.is_some()
            && let Some(id) = tree.find(root, "PartnerCashView")
            && crate::gui::textinput::text(tree, id) != Some(partner_credits.to_string().as_str())
        {
            crate::gui::textinput::set_text(tree, id, &partner_credits.to_string());
        }
        // (the partner's name in its `ScrollingTextView`, started over only when it changes)
        if let Some(id) = tree.find(root, "PartnerName") {
            if tree.control::<crate::gui::scrolling::ScrollingText>(id).is_some_and(|v| v.text.text != name) {
                crate::gui::scrolling::set_text(tree, id, &name, None, false);
            }
        }
        let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
        let mut is_open = self.open;
        // (the shop's name in the title too: the skins' `ShopTrade` views leave `PartnerName` out)
        let title = name.clone();
        crate::chrome::window(ctx, palette, "Shop", &title, &mut is_open, |w| w.default_size(min.max(egui::vec2(360.0, 0.0))).resizable(false), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            let origin = response.rect.min;
            let tree: &crate::gui::view::Tree = tree;
            let rect_of = |name: &str| tree.find(root, name).map(|id| crate::gui::host::screen_rect(tree, id, origin));
            let font = egui::FontId::proportional(12.0);
            // (each side's cost in the green GUI.dll gives the cash fields, 0x44dd44; with a player, the partner's credits,
            // and our own field is typed in)
            if player {
                if let Some(r) = rect_of("PartnerStatusDock") {
                    let status = if confirmed.1 { "Partner accepted" } else { "Partner trading" };
                    ui.painter().text(r.left_center() + egui::vec2(4.0, 0.0), egui::Align2::LEFT_CENTER, status, font.clone(), if confirmed.1 { green } else { text });
                }
            }
            for (field, rows) in [("PartnerCashView", bought), ("OurCashView", sold)] {
                if player {
                    break;
                }
                if let Some(r) = rect_of(field) {
                    ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(40)), egui::StrokeKind::Inside);
                    ui.painter().text(r.right_center() - egui::vec2(4.0, 0.0), egui::Align2::RIGHT_CENTER, rows.iter().map(|r| r.price).sum::<i64>().to_string(), font.clone(), green);
                }
            }
            // the three lists: Icon, Name, Count, Price, Quality (a row's description as its tooltip); returns the rows clicked,
            // double-clicked and right-clicked
            let mut list = |ui: &mut egui::Ui, dock: &str, rows: Vec<(Option<&Arc<Icon>>, &str, u32, i64, u32, &str)>, chosen: Option<usize>| -> (Option<usize>, Option<usize>, Option<usize>, Option<egui::Rect>) {
                let Some(area) = rect_of(dock) else { return (None, None, None, None) };
                let (mut clicked, mut double, mut info) = (None, None, None);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area.shrink(2.0)).id_salt(dock));
                crate::chrome::scroll(&mut child, palette, egui::ScrollArea::vertical().max_height(area.height() - 4.0).auto_shrink([false, false]), |ui| {
                    for (i, (icon, name, count, price, quality, tip)) in rows.into_iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(area.width() - 16.0, ROW), egui::Sense::click());
                        if chosen == Some(i) {
                            ui.painter().rect_filled(r, 0.0, accent.gamma_multiply(0.25));
                        } else if resp.hovered() {
                            ui.painter().rect_filled(r, 0.0, egui::Color32::from_white_alpha(12));
                        }
                        let icon_rect = egui::Rect::from_min_size(r.min + egui::vec2(1.0, 1.0), egui::vec2(ROW - 2.0, ROW - 2.0));
                        if let Some(icon) = icon {
                            ui.painter().image(gear.texture(ctx, icon), icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                        }
                        let x = |f: f32| r.left() + f * r.width();
                        let y = r.center().y;
                        ui.painter().text(egui::pos2(icon_rect.right() + 4.0, y), egui::Align2::LEFT_CENTER, name, font.clone(), text);
                        ui.painter().text(egui::pos2(x(0.68), y), egui::Align2::RIGHT_CENTER, count.to_string(), font.clone(), text);
                        ui.painter().text(egui::pos2(x(0.87), y), egui::Align2::RIGHT_CENTER, price.to_string(), font.clone(), green);
                        ui.painter().text(egui::pos2(x(0.99), y), egui::Align2::RIGHT_CENTER, quality.to_string(), font.clone(), text);
                        if resp.clicked() {
                            clicked = Some(i);
                        }
                        if resp.double_clicked() {
                            double = Some(i);
                        }
                        if resp.secondary_clicked() {
                            info = Some(i);
                        }
                        if !tip.is_empty() {
                            resp.on_hover_text(tip.replace("\r\n", "\n"));
                        }
                    }
                });
                (clicked, double, info, Some(area))
            };
            fn rows(rs: &[Row]) -> Vec<(Option<&Arc<Icon>>, &str, u32, i64, u32, &str)> {
                rs.iter().map(|r| (r.icon.as_ref(), r.name.as_str(), r.count, r.price, r.quality, "")).collect()
            }
            // (a stock item double-clicked goes in, right-clicked shows its page; one in either list double-clicked comes out)
            let (c, d, i, stock_area) = list(ui, "ShopInventoryDock", stock.iter().map(|s| (s.icon.as_ref(), s.name.as_str(), 1, s.price, s.quality, s.description.as_str())).collect(), *selected);
            if c.is_some() {
                *selected = c;
            }
            if let Some(i) = d {
                command = Some(ShopCommand::Buy(i));
            }
            if let Some(s) = i.and_then(|i| stock.get(i)) {
                *info_url = Some(format!("itemref://{}/{}/{}", s.low, s.high, s.quality));
            }
            let (_, d, _, bought_area) = list(ui, "PartnerInventoryDock", rows(bought), None);
            if let Some(i) = d {
                command = Some(ShopCommand::Remove(bought[i].item));
            }
            let (_, d, _, sold_area) = list(ui, "OurInventoryDock", rows(sold), None);
            if let Some(i) = d {
                command = Some(ShopCommand::Remove(sold[i].item));
            }
            // (an inventory item dropped on any of the three lists goes in: in shop mode FUN_100e121e gives each list's
            // drop signal, +0x2dc, FUN_100e015f, which adds an "inventory/item" drag to the trade)
            let dropped = |area: &Option<egui::Rect>| area.is_some_and(|a| ui.input(|i| i.pointer.any_released() && i.pointer.interact_pos().is_some_and(|p| a.contains(p))));
            if dragged != u32::MAX && [&stock_area, &bought_area, &sold_area].into_iter().any(dropped) {
                command = Some(ShopCommand::Sell(dragged));
            }
        });
        // (a player trade: the credits typed in, `OurCashView`'s text as a number)
        let typed = w.tree.find(w.root, "OurCashView").and_then(|id| crate::gui::textinput::text(&w.tree, id)).map(|t| t.chars().filter(char::is_ascii_digit).collect::<String>().parse::<i64>().unwrap_or(0));
        for (id, signal) in std::mem::take(&mut w.tree.signals) {
            let name = w.tree.get(id).name.clone();
            match (signal, name.as_str()) {
                // (FUN_100e0622: the cash sent, then a player trade confirmed, a shop's accepted)
                (crate::gui::host::Signal::Clicked, "AcceptButton") if player => {
                    if let Some(credits) = typed.filter(|&c| c != self.credits_sent) {
                        self.credits_sent = credits;
                        command = Some(ShopCommand::Credits(credits));
                        self.pending_confirm = true;
                    } else {
                        command = Some(ShopCommand::Confirm);
                    }
                }
                (crate::gui::host::Signal::Clicked, "AcceptButton") => command = Some(ShopCommand::Accept),
                (crate::gui::host::Signal::Clicked, "DeclineButton") => is_open = false,
                (crate::gui::host::Signal::Enter, "OurCashView") if player => {
                    if let Some(credits) = typed.filter(|&c| c != self.credits_sent) {
                        self.credits_sent = credits;
                        command = Some(ShopCommand::Credits(credits));
                    }
                }
                _ => {}
            }
        }
        // (the Accept after the credits went: the confirm the next frame)
        if command.is_none() && std::mem::take(&mut self.pending_confirm) {
            command = Some(ShopCommand::Confirm);
        }
        if !is_open && self.open {
            self.open = false;
            self.partner = None;
            command = Some(ShopCommand::Close);
        }
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 2026-10-31 15:05 capture's trade with a player: opened, our side confirmed, the partner accepted (the "Are you sure"
    /// dialog), complete (the window closes).
    #[test]
    fn a_player_trade_runs_as_captured() {
        let mut w = ShopWindow::default();
        w.apply(Change::PlayerOpen { partner: 0x57d8, name: "Partner".into() });
        assert!(w.open && w.partner == Some(0x57d8));
        w.apply(Change::PartnerCredits(500));
        w.apply(Change::Confirmed { ours: true });
        assert!(!w.asking);
        w.apply(Change::Confirmed { ours: false });
        assert!(w.asking && w.partner_credits == 500);
        w.apply(Change::Complete);
        assert!(!w.open && w.partner.is_none() && !w.asking);
    }

    #[test]
    fn prices_follow_the_vending_machine() {
        // (Old Athens' booths: the template 0x184f1 sells at 105 % and buys at 4 %, type 61 by default)
        let rates = Rates { buy: 105, sell: 4, kind: COMPUTER_LITERACY_SHOP };
        // 1000 at 105 % is 1050; 400 Computer Literacy is 10 points, 10 % off
        assert_eq!(buy_price(1000, rates, 400), 945);
        // (3000 and more Computer Literacy is 75 points; the cap of 90 is never reached)
        assert_eq!(buy_price(1000, rates, 5000), 1050 - 787);
        // another type of shop has no discount
        assert_eq!(buy_price(1000, Rates { kind: 1, ..rates }, 400), 1050);
        // a stack of 5 at 1000 each, 10 points of bonus: 1100 * 5 * 4 %
        assert_eq!(sell_price(1000, None, 5, rates, 400), 220);
        // stat 212 divides the value first
        assert_eq!(sell_price(1000, Some(10), 1, rates, 0), 4);
    }

    /// `MAO_GAME`: the game's `TradeGUI.xml` has the shop view's docks, fields and buttons.
    #[test]
    fn the_skin_has_the_shop_view() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let dir = std::path::Path::new(&game).join("cd_image/gui/Default");
        let (tree, root) = crate::gui::load_view(&[dir], "TradeGUI.xml", "ShopTrade").unwrap();
        for name in ["PartnerName", "ShopInventoryDock", "PartnerInventoryDock", "OurInventoryDock", "PartnerCashView", "OurCashView", "AcceptButton", "DeclineButton"] {
            assert!(tree.find(root, name).is_some(), "{name}");
        }
    }

    #[test]
    fn values_lie_between_the_templates() {
        // (FUN_100cb909: QL 1 to 100 between 10 and 1000, at 50: 10 + 49 * 990 / 99 + 0.5)
        assert_eq!(interpolate(10, 1000, 1, 100, 50), 500);
        assert_eq!(interpolate(10, 1000, 1, 100, 100), 1000);
        assert_eq!(interpolate(10, 1000, 1, 100, 1), 10);
        // (FUN_100cbb68: half way along is a quarter of the way up)
        assert_eq!(curve(505, 10, 1000), 257);
        assert_eq!(curve(10, 10, 1000), 10);
    }
}
