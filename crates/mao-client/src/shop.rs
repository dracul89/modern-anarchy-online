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
//!
//! **The buy-only shops** (2026-11-16). The trade's kind is decided as it starts (Gamecode FUN_1006692a, which
//! raises `InventoryGUIModule_c::SlotStartTrade`): 0 with a player, 1 with a shop whose type (stat 156, 61 when it
//! has none: FUN_10099ae4) is 61, 2 with any other shop. **The type is the stat the shop is paid in**: 61 is the
//! character's credits, and what a character can afford is its stat of the shop's type against the cost (Gamecode
//! FUN_1009a69a). The game's data has four other types, all of them points (`shopkinds` lists them, 2026-11-16):
//! 68 (the veterans' vendor), 669 (the twenty Ofab vendors), 672 (the three Phasefront vendors) and 695 (the ICC
//! reward terminal). Nothing is sold to a shop for points, which is why such a trade has no list of one's own;
//! Computer Literacy moves the prices at a shop of credits only. Kind 2 is the view `ShopBuy`,
//! which has the stock and the bought items and no "Sold items" list, and the view's own flag at +0x1a8 is put
//! out: nothing of one's own goes in ([`Rates::buy_only`]). Not seen at such a shop: the view was checked against
//! the game's file only.

use std::sync::Arc;

use crate::gear::{GearItem, GearWindow, Icon};
use crate::gui::multilist;
use crate::theme::Palette;

/// An item's stat read as none (the original's 1234567890).
const NO_VALUE: u32 = 1_234_567_890;
/// The shop type (stat 156) of a shop paid in credits (the type is the stat paid from, and 61 is the character's
/// credits): the default when the shop has none. Only here do the prices follow Computer Literacy.
const CREDITS_SHOP: i64 = 0x3d;

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

/// What a click on a row of one of the trade's three lists does.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RowClick {
    /// The row's own action: a stock item is put in (one of it), a bought or sold one is taken out.
    Act,
    /// The item's page.
    Page,
    Nothing,
}

/// The right button held this long on a row is no click (the lists' `MouseButtonHoldDelay`: the list raises its
/// menu signal then, which a trade's lists give nobody, and the release after it does nothing).
pub const HOLD: f32 = 0.3;

/// A click on a row by its button, Shift, and how long the button was down. The trade's lists are plain
/// `ItemContainerView_c`s whose flags have bit 2 (GUI.dll FUN_100e07b9, FUN_100e0942, FUN_100e0b00: 6, 6 and 0xe),
/// so their mouse-up (FUN_1004141b) raises the click signal for a left click without Shift and for any right
/// click, and the Shift-click signal for a left click with it. `TradeView_c` hangs the row's action on the first
/// (FUN_100dffe8 `N3Msg_TradeAddItem`, FUN_100dffa3 and FUN_100dff6d `N3Msg_TradeRemoveItem`) and the item's page on
/// the second (FUN_100e0096, FUN_100e002d). There is no double click: two clicks are two actions.
pub fn row_click(right: bool, shift: bool, held: f32) -> RowClick {
    match (right, shift) {
        (true, _) if held >= HOLD => RowClick::Nothing,
        (false, true) => RowClick::Page,
        _ => RowClick::Act,
    }
}

/// Whether a trade's window is up, for the inventory's Ctrl + right click (GUI.dll's FUN_100cae1b asks the
/// inventory module for its trade view, FUN_1002fcb7).
static TRADING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn trading() -> bool {
    TRADING.load(std::sync::atomic::Ordering::Relaxed)
}

/// Told each frame by the window's owner.
pub fn set_trading(open: bool) {
    TRADING.store(open, std::sync::atomic::Ordering::Relaxed);
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
        Rates { buy: stat(427).unwrap_or(0), sell: stat(426).unwrap_or(0), kind: stat(156).filter(|&k| k != 0).unwrap_or(CREDITS_SHOP) }
    }

    /// Whether the shop only sells (Gamecode FUN_10099b27, FUN_1006692a): it is not paid in credits. A shop whose
    /// type is not known (0) is taken for one of credits, as one without the stat is.
    pub fn buy_only(&self) -> bool {
        self.kind != 0 && self.kind != CREDITS_SHOP
    }
}

/// A shop that was opened: the booth, its name and its stock.
pub struct ShopState {
    pub shop: (u32, u32),
    pub name: String,
    pub stock: Vec<Slot>,
    /// The shop takes nothing in (`Rates::buy_only`): the view `ShopBuy`.
    pub buy_only: bool,
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
    if rates.kind != CREDITS_SHOP {
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
    if rates.kind == CREDITS_SHOP {
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

/// The three docks' lists: the stock (`ShopInventoryDock`, FUN_100e07b9), the partner's items or those bought
/// (`PartnerInventoryDock`, FUN_100e0942) and one's own or those sold (`OurInventoryDock`, FUN_100e0b00).
const DOCKS: [&str; 3] = ["ShopInventoryDock", "PartnerInventoryDock", "OurInventoryDock"];

/// The columns an item list can have (FUN_100ce55d): the id, the bit of the list's column flags that adds it (none for
/// the icon's, always there), the program's name for it, its width and its flags (the icon's sortable; the others
/// resizable, movable and sortable; the price's also 0x10 with the list's flag 0x10, which no trade list has).
const COLUMNS: [(i32, u32, &str, f32, u32); 5] = [(0, 0, "Icon", 16.0, 8), (1, 2, "Name", 200.0, 0xe), (2, 1, "Count", 30.0, 0xe), (3, 4, "Price", 100.0, 0xe), (4, 8, "Quality", 100.0, 0xe)];

/// A dock's list as the trade view makes it (FUN_100e07b9, FUN_100e0942, FUN_100e0b00): its column flags and the most
/// cells it asks room for downwards (the fewest are one by one, the most across 1000, which `SetViewCellCounts` makes
/// 50, as it makes 1000 down). The stock has every column (0xf) and room for many rows; the partner's list has the
/// price only at a shop (0xf, else 0xb) and one row of room beside a stock list, many without one (a player trade);
/// one's own list has no price (0xb) and one row.
pub fn dock_list(dock: usize, shop: bool) -> (u32, usize) {
    match dock {
        0 => (0xf, 1000),
        1 if shop => (0xf, 1),
        1 => (0xb, 1000),
        _ => (0xb, 1),
    }
}

/// The columns a list with these column flags has (FUN_100ce55d's `AddColumn`s): id, name, width, flags.
pub fn columns(flags: u32) -> Vec<(i32, &'static str, f32, u32)> {
    COLUMNS.iter().filter(|c| c.1 == 0 || flags & c.1 != 0).map(|c| (c.0, c.2, c.3, c.4)).collect()
}

/// A list's row (`InventoryListViewItem_c`, FUN_1003dee6): the name, how many (nothing counted reads 1) and the quality,
/// and where the list has the price column the shop's price (FUN_100cded9: `N3Msg_GetShopItemStat(item, 74)`, which
/// is `VendingMachine_t`'s FUN_10099e9b, what the shop sells it for), written with the thousands grouped
/// (`FormatNumeric`, FUN_1003d7d0); sorted by the order it came in (the icon's column), the name without its case and
/// the numbers.
pub fn row(id: i64, icon: Option<egui::TextureId>, name: &str, count: u32, quality: u32, price: Option<i64>) -> multilist::Item {
    use multilist::CellView::Text;
    let mut cells = vec![(1, Text(name.to_string())), (2, Text(count.max(1).to_string()))];
    let mut keys = vec![(0, format!("{id:08}")), (1, name.to_lowercase()), (2, format!("{:010}", count.max(1)))];
    if let Some(price) = price {
        cells.push((3, Text(crate::vitals::grouped(price))));
        keys.push((3, format!("{:020}", price.max(0))));
    }
    cells.push((4, Text(quality.to_string())));
    keys.push((4, format!("{quality:010}")));
    multilist::Item { id, icon, count: count.max(1), cells, keys, ..Default::default() }
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
    /// An item's page for the Info window (a row clicked with Shift: `itemref://low/high/ql`), until taken.
    pub info_url: Option<String>,
    /// When the right button went down.
    right_since: Option<std::time::Instant>,
    window: Option<crate::gui::window::XmlWindow>,
    /// A trade with another player (`TradeView_c`'s mode 0, the skin's `PlayerTrade` view): the partner, the credits they put
    /// in, whether each side has confirmed, and whether "Are you sure you want to complete this trade?" is up.
    pub partner: Option<u32>,
    pub partner_credits: i64,
    pub confirmed: (bool, bool),
    asking: bool,
    /// The view the window was built from (`ShopTrade`, `ShopBuy` or `PlayerTrade`).
    view: &'static str,
    /// The shop only sells: nothing of one's own goes in.
    pub buy_only: bool,
    /// The credits last sent, and a confirm waiting for the frame after them.
    credits_sent: i64,
    pending_confirm: bool,
    /// The docks' lists (none where the view has no such dock), the rows each last showed, and the keepers of their
    /// widths and list sorts.
    lists: [Option<crate::gui::view::Id>; 3],
    shown: [Vec<multilist::Item>; 3],
    keepers: [multilist::Keeper; 3],
    /// The stock list's (in a player trade the partner's) list mode and grid order, as kept ([`ShopWindow::config`]).
    in_list: Option<bool>,
    grid_sort: Option<(i32, i32)>,
    /// The window's menu (FUN_100e03c7) and its entries' keys.
    popup: Option<crate::gui::popup::PopupMenu>,
    menu: Option<Vec<Vec<String>>>,
}

impl ShopWindow {
    pub fn set(&mut self, state: ShopState) {
        self.partner = None;
        self.buy_only = state.buy_only;
        self.shop = state.shop;
        self.name = state.name;
        self.stock = state.stock;
        self.bought.clear();
        self.sold.clear();
        self.open = true;
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
                // (a player trade's window closes when it goes through)
                if self.partner.take().is_some() {
                    self.open = false;
                    self.asking = false;
                }
            }
            Change::PlayerOpen { partner, name } => {
                let (window, lists) = (self.window.take(), self.lists);
                *self = ShopWindow { partner: Some(partner), name, open: true, window, lists, view: self.view, ..Default::default() };
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

    /// The view of the trade's kind (FUN_100e1074).
    fn wanted_view(&self) -> &'static str {
        if self.partner.is_some() {
            "PlayerTrade"
        } else if self.buy_only {
            "ShopBuy"
        } else {
            "ShopTrade"
        }
    }

    /// The trade's config (FUN_100e121e: `TradeViewConfig` with a player, `ShopViewConfig` at a shop), the list whose mode
    /// and order it keeps and that list's entry in it: at a shop the stock's (`shop_listview_config`), with a player the
    /// partner's (`partner_listview_config`). They are the lists the window's menu acts on (FUN_100e03c7).
    fn config(&self) -> (&'static str, usize, &'static str) {
        if self.partner.is_some() { ("TradeViewConfig", 1, "partner_listview_config") } else { ("ShopViewConfig", 0, "shop_listview_config") }
    }

    /// FUN_100e1074 and the docks: `ShopTrade` (or, trading with a player, `PlayerTrade`) of the skin's `TradeGUI.xml`, each
    /// dock given its `ItemContainerView_c` (FUN_100ce55d) and the fields given room.
    fn load(&mut self, ctx: &egui::Context) {
        let view = self.wanted_view();
        self.view = view;
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        let Some((mut tree, root)) = crate::gui::load_view(&dirs, "TradeGUI.xml", view) else { return };
        // (the text measured from the start, so the first layout gives the labels their width)
        tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
        self.lists = add_lists(&mut tree, root);
        self.shown = Default::default();
        self.keepers = Default::default();
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
        self.window = Some(crate::gui::window::XmlWindow { name: "Shop".into(), title: "Shop".into(), tree, root, open: true, default_size: egui::vec2(TRADE_SIZE.0, TRADE_SIZE.1) });
    }

    /// Draws the window; returns what to send, if anything.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, items: &[GearItem], palette: &Palette) -> Option<ShopCommand> {
        if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
            self.right_since = Some(std::time::Instant::now());
        }
        let right_held = self.right_since.map_or(0.0, |t| t.elapsed().as_secs_f32());
        let wanted = self.wanted_view();
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
        let text = egui::Color32::from_rgb(palette.text[0], palette.text[1], palette.text[2]);
        let green = egui::Color32::from_rgb(0x44, 0xdd, 0x44);
        // (the kept list mode and grid order of the list the trade's config keeps: as the original, a grid sorted by
        // name, FUN_100ce55d's `SetListSortColumn(1, 0)` and FUN_10041258(1), until the menu changes them;
        // `MAO_SHOP_LIST=1`: the list mode, for a picture)
        let (config, kept, entry) = self.config();
        let key = format!("{config}.{entry}");
        let in_list = *self.in_list.get_or_insert_with(|| std::env::var_os("MAO_SHOP_LIST").is_some() || crate::layout::tab(&format!("{key}.List")) == Some(1));
        let grid_sort = *self.grid_sort.get_or_insert_with(|| match crate::layout::tab(&format!("{key}.GridSort")) {
            Some(n) if n > 0 => (((n - 1) / 2) as i32, ((n - 1) % 2) as i32),
            _ => (1, 0),
        });
        let w = self.window.as_mut()?;
        let skin = crate::skin::skin(ctx);
        let root = w.root;
        // (the lists' rows: the stock's with its price, the bought items' with the stock's price at a shop, one's own
        // without; each its place in its list as its id)
        let shop_prices = !player;
        let rows: [Vec<multilist::Item>; 3] = [
            self.stock.iter().enumerate().map(|(i, s)| row(i as i64, s.icon.as_ref().map(|c| gear.texture(ctx, c)), &s.name, 1, s.quality, Some(s.price))).collect(),
            self.bought.iter().enumerate().map(|(i, r)| row(i as i64, r.icon.as_ref().map(|c| gear.texture(ctx, c)), &r.name, r.count, r.quality, shop_prices.then_some(r.price))).collect(),
            self.sold.iter().enumerate().map(|(i, r)| row(i as i64, r.icon.as_ref().map(|c| gear.texture(ctx, c)), &r.name, r.count, r.quality, None)).collect(),
        ];
        for (k, rows) in rows.into_iter().enumerate() {
            let Some(list) = self.lists[k] else { continue };
            let tree = &mut w.tree;
            let list_mode = k == kept && in_list;
            multilist::set_list_mode(tree, list, list_mode);
            self.keepers[k].before(&format!("{config}.{}", DOCKS[k]), tree, list, (!list_mode).then_some(if k == kept { grid_sort } else { (1, 0) }));
            multilist::update(tree, list);
            if self.shown[k] != rows {
                let prices = rows.iter().filter_map(|r| r.keys.iter().find(|c| c.0 == 3).and_then(|c| c.1.parse::<i64>().ok()).map(|p| (r.id, p))).collect();
                multilist::set_items(tree, list, rows.clone());
                multilist::set_prices(tree, list, prices);
                self.shown[k] = rows;
            }
        }
        let min = crate::gui::host::preferred_pixels(&w.tree, root, false);
        let tree = &mut w.tree;
        let lists = self.lists;
        let (stock, bought, sold, info_url) = (&self.stock, &self.bought, &self.sold, &mut self.info_url);
        let buy_only = self.buy_only && self.partner.is_none();
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
        let menu_up = self.popup.is_some();
        let mut menu_at = None;
        // (the shop's name in the title too: the skins' `ShopTrade` views leave `PartnerName` out)
        let title = name.clone();
        let shift = ctx.input(|i| i.modifiers.shift);
        // (a list's row a right click acted on, and an item's page asked for: the list and the row)
        let (mut right_act, mut page): (Option<(usize, usize)>, Option<(usize, usize)>) = (None, None);
        crate::chrome::window(ctx, palette, "Shop", &title, &mut is_open, |w| w.default_size(min.max(egui::vec2(TRADE_SIZE.0, TRADE_SIZE.1))).resizable(true), |ui| {
            // (the views over the whole window, laid out from the bottom so the window does not grow)
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
                let origin = response.rect.min;
                let rect_of = |tree: &crate::gui::view::Tree, name: &str| tree.find(root, name).map(|id| crate::gui::host::screen_rect(tree, id, origin));
                let font = egui::FontId::proportional(12.0);
                // (each side's cost in the green GUI.dll gives the cash fields, 0x44dd44; with a player, the partner's
                // credits, and our own field is typed in)
                if player {
                    if let Some(r) = rect_of(tree, "PartnerStatusDock") {
                        let status = if confirmed.1 { "Partner accepted" } else { "Partner trading" };
                        ui.painter().text(r.left_center() + egui::vec2(4.0, 0.0), egui::Align2::LEFT_CENTER, status, font.clone(), if confirmed.1 { green } else { text });
                    }
                }
                for (field, rows) in [("PartnerCashView", bought), ("OurCashView", sold)] {
                    if player {
                        break;
                    }
                    if let Some(r) = rect_of(tree, field) {
                        ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(40)), egui::StrokeKind::Inside);
                        ui.painter().text(r.right_center() - egui::vec2(4.0, 0.0), egui::Align2::RIGHT_CENTER, rows.iter().map(|r| r.price).sum::<i64>().to_string(), font.clone(), green);
                    }
                }
                // (the row under the pointer, in which list)
                let pointer = ui.input(|i| i.pointer.hover_pos());
                let hovered = pointer.and_then(|at| lists.iter().enumerate().find_map(|(k, l)| l.and_then(|l| multilist::item_at(tree, l, origin, at)).map(|i| (k, i as usize))));
                // (a right click on a row: the list's click signal, unless the button was held for the menu's delay
                // (`row_click`); beside the rows, the window's menu, FUN_100e03c7. The original's is the frame's menu
                // button.)
                if ui.input(|i| i.pointer.secondary_clicked()) && !menu_up {
                    match hovered {
                        Some(at) => {
                            if row_click(true, shift, right_held) == RowClick::Act {
                                right_act = Some(at);
                            }
                        }
                        None => menu_at = pointer.filter(|p| response.rect.contains(*p)),
                    }
                }
                // (an inventory item dropped on any of the three lists goes in: in shop mode FUN_100e121e gives each
                // list's drop signal, +0x2dc, FUN_100e015f, which adds an "inventory/item" drag to the trade)
                let areas: Vec<egui::Rect> = DOCKS.iter().filter_map(|d| rect_of(tree, d)).collect();
                let dropped = crate::gear::letting_go(ui.ctx()) && ui.input(|i| i.pointer.interact_pos().is_some_and(|p| areas.iter().any(|a| a.contains(p))));
                if dragged != u32::MAX && !buy_only && dropped {
                    command = Some(ShopCommand::Sell(dragged));
                }
                // (a stock row's description as its tip)
                if let Some(tip) = hovered.filter(|h| h.0 == 0).and_then(|(_, i)| stock.get(i)).map(|s| s.description.replace("\r\n", "\n")).filter(|t| !t.is_empty()) {
                    response.on_hover_text(tip);
                }
            });
        });
        // (a left click: the click signal, or with Shift the item's page, `row_click`; a press that moves is nothing)
        for list in lists.into_iter().flatten() {
            multilist::click_only(tree, list);
        }
        for (list, event) in multilist::take_events(tree) {
            let multilist::ItemEvent::Clicked(i, _) = event else { continue };
            let Some(k) = lists.iter().position(|l| *l == Some(list)) else { continue };
            match row_click(false, shift, 0.0) {
                RowClick::Act => right_act = Some((k, i as usize)),
                RowClick::Page => page = Some((k, i as usize)),
                RowClick::Nothing => {}
            }
        }
        // (a stock item clicked goes in, one of it a click; one in either list clicked comes out; with Shift the click
        // shows the item's page. The original's pages are `shopitemid://type/instance` for a stock row, which adds
        // the shop's price (FUN_10031fda), and `itemid://type/instance` for the others: here the client's own
        // `shopitemid://low/high/quality/price` for a stock row and the item's own page for the other two.)
        let page_of = |item: [u32; 4]| match item[2] {
            0x6f => stock.get(item[3] as usize).map(|s| format!("itemref://{}/{}/{}", s.low, s.high, s.quality)),
            _ => items.iter().find(|g| g.slot == item[3]).map(|g| format!("itemref://{}/{}/{}", g.item_id, g.high_id, g.quality)),
        };
        match right_act {
            Some((0, i)) if i < stock.len() => command = Some(ShopCommand::Buy(i)),
            Some((1, i)) => command = bought.get(i).map(|r| ShopCommand::Remove(r.item)).or(command),
            Some((2, i)) => command = sold.get(i).map(|r| ShopCommand::Remove(r.item)).or(command),
            _ => {}
        }
        match page {
            Some((0, i)) => {
                if let Some(s) = stock.get(i) {
                    *info_url = Some(format!("shopitemid://{}/{}/{}/{}", s.low, s.high, s.quality, s.price));
                }
            }
            Some((1, i)) => {
                if let Some(url) = bought.get(i).and_then(|r| page_of(r.item)) {
                    *info_url = Some(url);
                }
            }
            Some((2, i)) => {
                if let Some(url) = sold.get(i).and_then(|r| page_of(r.item)) {
                    *info_url = Some(url);
                }
            }
            _ => {}
        }
        for (k, list) in lists.iter().enumerate() {
            if let Some(list) = *list {
                self.keepers[k].after(&format!("{config}.{}", DOCKS[k]), tree, list, k == kept && in_list);
            }
        }
        // (the window's menu, FUN_100e03c7: "List Mode" with its mark, and "Auto Arrange" with FUN_10041637(0)'s sub-menu,
        // dead in list mode, for the list the config keeps; then the frame's own entry)
        if let Some(list) = lists[kept]
            && let Some(at) = menu_at.or_else(|| crate::chrome::frame_menu("Shop"))
        {
            use crate::gui::popup::Entry;
            let text = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
            let labels: Vec<(i32, String)> = multilist::columns(tree, list).into_iter().map(|c| (c.id, c.label)).collect();
            let (arrange_keys, arrange) = crate::nanoprograms::arrange_menu(&labels, Some(grid_sort), false);
            let keys = vec![vec!["list".to_string()], arrange_keys, vec!["close".to_string()]];
            let entries = vec![Entry::marked(text("GTK_Container_ListMode", "As a list"), true, in_list), Entry::submenu(text("GTK_Container_AutoArrange", "Order"), !in_list, arrange), Entry::text(crate::chrome::close_entry(), true)];
            self.popup = Some(crate::chrome::menu_of(ctx, at, entries));
            self.menu = Some(keys);
        }
        if let Some(crate::chrome::Picked::Entry(path)) = crate::chrome::menu_path(ctx, palette, "shop-menu", &mut self.popup)
            && let Some(keys) = self.menu.take()
            && let Some(chosen) = path.first().and_then(|k| keys.get(*k)).and_then(|k| k.get(path.get(1).copied().unwrap_or(0)))
        {
            match chosen.as_str() {
                "close" => crate::chrome::close_next("Shop"),
                "list" => {
                    self.in_list = Some(!in_list);
                    crate::layout::set_tab(&format!("{key}.List"), usize::from(!in_list));
                }
                other => {
                    if let Some(Some(new)) = crate::loot::arrange_choice(other, Some(grid_sort)) {
                        self.grid_sort = Some(new);
                        crate::layout::set_tab(&format!("{key}.GridSort"), 1 + (new.0 * 2 + new.1) as usize);
                    }
                }
            }
        }
        if self.popup.is_none() {
            self.menu = None;
        }
        // (a player trade: the credits typed in, `OurCashView`'s text as a number)
        let typed = tree.find(root, "OurCashView").and_then(|id| crate::gui::textinput::text(tree, id)).map(|t| t.chars().filter(char::is_ascii_digit).collect::<String>().parse::<i64>().unwrap_or(0));
        for (id, signal) in std::mem::take(&mut tree.signals) {
            let name = tree.get(id).name.clone();
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

/// `TradeView_c`'s own size as it is made (FUN_100e121e: `Rect(0, 0, 119, 452)`), the window's to start with where the
/// view needs no more.
const TRADE_SIZE: (f32, f32) = (119.0, 452.0);

/// Each dock of the view its `ItemContainerView_c` (FUN_100ce55d) as the trade view makes them (FUN_100e07b9,
/// FUN_100e0942, FUN_100e0b00; the stock's only at a shop, FUN_100e121e): a grid-mode list (`MultiListView_c` starts in
/// it, layout mode 0) of the columns its flags give, sorted by name in both modes and arranged (`SetListSortColumn(1, 0)`,
/// FUN_10041258(1)), no slots beyond its items (no `SetMaxItemCount`), the vertical bar always there
/// (`SetVScrollBarMode(1)`), and room for one cell to as many as [`dock_list`] gives (`SetViewCellCounts`); the dock's
/// `LayoutNode` puts it over the whole dock. The original's horizontal bar is never shown (`SetHScrollBarMode(0)`):
/// here it is shown when needed, which an arranged grid never is, so that the grid's width does not hold the window's.
fn add_lists(tree: &mut crate::gui::view::Tree, root: crate::gui::view::Id) -> [Option<crate::gui::view::Id>; 3] {
    let shop = tree.find(root, DOCKS[0]).is_some();
    let mut lists = [None; 3];
    for (k, dock) in DOCKS.iter().enumerate() {
        let Some(at) = tree.find(root, dock) else { continue };
        let (flags, rows) = dock_list(k, shop);
        let list = multilist::create_grid(tree, "ItemContainerView", -1, 1);
        for (id, label, width, column_flags) in columns(flags) {
            multilist::add_column(tree, list, id, label, width, column_flags);
        }
        multilist::set_sort(tree, list, 1, 0);
        multilist::set_scroll_modes(tree, list, 2, 1);
        multilist::set_view_cells(tree, list, (1, 1), (1000, rows));
        multilist::click_only(tree, list);
        tree.get_mut(at).layout = crate::gui::view::Layout::Base;
        tree.add_child(at, list);
        lists[k] = Some(list);
    }
    lists
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

    /// FUN_1004141b for a list whose flags have bit 2: a left click and a right click raise the click signal, a
    /// left click with Shift the other one, and a right button held for the hold delay raises neither.
    #[test]
    fn one_click_on_a_row_acts() {
        assert_eq!(row_click(false, false, 0.0), RowClick::Act);
        assert_eq!(row_click(true, false, 0.1), RowClick::Act);
        assert_eq!(row_click(false, true, 0.0), RowClick::Page);
        // (the right button's release tests no key)
        assert_eq!(row_click(true, true, 0.1), RowClick::Act);
        assert_eq!(row_click(true, false, HOLD), RowClick::Nothing);
    }

    #[test]
    fn prices_follow_the_vending_machine() {
        // (Old Athens' booths: the template 0x184f1 sells at 105 % and buys at 4 %, type 61 by default)
        let rates = Rates { buy: 105, sell: 4, kind: CREDITS_SHOP };
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
        // (the buy-only view: the stock and the bought items, and no place for one's own)
        let dir = std::path::Path::new(&game).join("cd_image/gui/Default");
        let (tree, root) = crate::gui::load_view(&[dir], "TradeGUI.xml", "ShopBuy").unwrap();
        for name in ["PartnerName", "ShopInventoryDock", "PartnerInventoryDock", "PartnerCashView", "AcceptButton", "DeclineButton"] {
            assert!(tree.find(root, name).is_some(), "{name}");
        }
        assert!(tree.find(root, "OurInventoryDock").is_none() && tree.find(root, "OurCashView").is_none());
    }

    /// A shop paid in something else than credits only sells; one without a type, or of none known, is paid in
    /// credits. `MAO_GAME`: the veterans' vendor (0x3f2ea) is paid in stat 68 and an Ofab vendor (0x41142) in 669,
    /// so they only sell; Old Athens' booth (0x184f1) takes credits.
    #[test]
    fn a_shop_of_another_type_only_sells() {
        let rates = |kind| Rates { buy: 100, sell: 10, kind };
        assert!(!rates(CREDITS_SHOP).buy_only() && !rates(0).buy_only());
        assert!(rates(0x3e).buy_only() && rates(1).buy_only());
        let mut w = ShopWindow::default();
        assert_eq!(w.wanted_view(), "ShopTrade");
        w.buy_only = true;
        assert_eq!(w.wanted_view(), "ShopBuy");
        w.partner = Some(7);
        assert_eq!(w.wanted_view(), "PlayerTrade");
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let of = |instance: u32| Rates::of(&template(&db, instance).unwrap());
        assert_eq!((of(0x3f2ea).kind, of(0x41142).kind, of(0x184f1).kind), (68, 669, CREDITS_SHOP));
        assert!(of(0x3f2ea).buy_only() && of(0x41142).buy_only() && !of(0x184f1).buy_only());
    }

    /// WIN-035: the docks' lists have FUN_100ce55d's columns by the trade view's flags (FUN_100e07b9, FUN_100e0942,
    /// FUN_100e0b00); a row's cells (FUN_1003dee6, FUN_100cded9, the price grouped as `FormatNumeric` writes it) and
    /// what it sorts by, the name without its case first.
    #[test]
    fn the_docks_lists_have_the_originals_columns() {
        let ids = |flags| columns(flags).iter().map(|c| c.0).collect::<Vec<_>>();
        assert_eq!(dock_list(0, true), (0xf, 1000));
        assert_eq!((ids(dock_list(0, true).0), ids(dock_list(1, true).0)), (vec![0, 1, 2, 3, 4], vec![0, 1, 2, 3, 4]));
        // (no price with a player, nor in one's own list)
        assert_eq!((ids(dock_list(1, false).0), ids(dock_list(2, true).0)), (vec![0, 1, 2, 4], vec![0, 1, 2, 4]));
        // (one row of room for the bought and the sold items beside the stock; a player's list room for many)
        assert_eq!((dock_list(1, true).1, dock_list(1, false).1, dock_list(2, false).1), (1, 1000, 1));
        assert_eq!(columns(0xf)[3], (3, "Price", 100.0, 0xe));
        assert_eq!(columns(0xb)[0], (0, "Icon", 16.0, 8));
        use multilist::CellView::Text;
        let r = row(2, None, "Bolt", 0, 25, Some(12_345));
        assert_eq!(r.count, 1);
        assert_eq!(r.cells, vec![(1, Text("Bolt".into())), (2, Text("1".into())), (3, Text("12,345".into())), (4, Text("25".into()))]);
        assert!(row(3, None, "Bolt", 2, 25, None).cells.iter().all(|c| c.0 != 3));
        // (sorted by name, as the lists start: `SetListSortColumn(1, 0)`)
        let mut tree = crate::gui::view::Tree::default();
        let list = multilist::create_grid(&mut tree, "ItemContainerView", -1, 1);
        for (id, label, width, flags) in columns(0xf) {
            multilist::add_column(&mut tree, list, id, label, width, flags);
        }
        multilist::set_sort(&mut tree, list, 1, 0);
        multilist::set_items(&mut tree, list, vec![row(0, None, "zeta", 1, 1, Some(5)), row(1, None, "Alpha", 1, 1, Some(500)), row(2, None, "beta", 1, 1, Some(50))]);
        assert_eq!(multilist::order(&tree, list), [1, 2, 0]);
        // (by price, the numbers' order, the dearest first)
        multilist::set_sort(&mut tree, list, 3, 1);
        assert_eq!(multilist::order(&tree, list), [1, 2, 0]);
    }

    /// WIN-035, `MAO_GAME`: in the game's `ShopTrade` view the bought and sold items' lists are one row of cells high
    /// and the stock's takes the rest of the window's height; a player's trade has the two lists only.
    #[test]
    fn the_docks_are_sized_by_their_lists() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let dir = std::path::Path::new(&game).join("cd_image/gui/Default");
        let (mut tree, root) = crate::gui::load_view(std::slice::from_ref(&dir), "TradeGUI.xml", "ShopTrade").unwrap();
        let lists = add_lists(&mut tree, root);
        assert!(lists.iter().all(Option::is_some));
        for l in lists.iter().flatten() {
            multilist::update(&mut tree, *l);
        }
        crate::gui::host::fit(&mut tree, root, egui::vec2(TRADE_SIZE.0 + 100.0, TRADE_SIZE.1));
        let h = |k: usize| tree.get(lists[k].unwrap()).frame.height();
        assert_eq!((h(1), h(2)), (59.0, 59.0));
        assert!(h(0) > 59.0);
        let (mut tree, root) = crate::gui::load_view(&[dir], "TradeGUI.xml", "PlayerTrade").unwrap();
        let lists = add_lists(&mut tree, root);
        assert!(lists[0].is_none() && lists[1].is_some() && lists[2].is_some());
        assert_eq!(multilist::columns(&tree, lists[1].unwrap()).len(), 4);
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
