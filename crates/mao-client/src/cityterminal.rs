//! A city's controller window, GUI.dll's `CityTerminalWindow_c` (FUN_100e7847 on `Views/CityTerminal.xml`), and the
//! session behind it, city.dll's `CityClientInterface_c`. The readings are `docs/CITY_CONTROLLER_ORIGINAL.md` and
//! `docs/CITY_MAP_ORIGINAL.md`.
//!
//! The controller is used as any terminal is; the server answers with `AOTransportSignal` code 5 (`mao_net::city`),
//! which opens the session and the window (`CityTerminalGUIModule_c::SlotOpenCityTerminal`, FUN_100e6c24). While the
//! session lasts the server tells the lease mode, the upkeep, the time to the next bill, the cloaking device's state,
//! the controller's charge and the owner's name; code 7 ends it and the window goes.
//!
//! The window (no title; 499 by 499 in the screen's middle) for GUI type 2:
//!
//! * lease mode 1: one tab, the city seen (`InspectCityPanel_c` FUN_100e893e): the charge bar, the owner, the map;
//! * else four tabs (FUN_100e755a): the upkeep (`UpkeepControllerView_c` FUN_100e9374: a help text, the owner, the
//!   next bill's date, the upkeep, the charge bar, and "dump the city's advantages", which is the organisation
//!   command 0x1f and enabled with lease mode 3 only), the houses to build and the houses to demolish (one
//!   `HouseBuildView_c` shown twice, the tab choosing its mode), and the cloaking device's button (a toggle: down
//!   while the device is on, enabled with lease mode 3 while the server lets it be switched; a press sends the
//!   request at once, FUN_100e72a2: the original asks nothing first).
//!
//! The map (`BuildingMapView_c`; its keeping is `citymap.rs`): the ground from above, 2 pixels to a world unit, over
//! the city's zones and 50 units round them, in a scroll view; on it the zones' tiles a house may stand on, the
//! houses (yellow; blue-green once their demolishing began; the selected one bright), the houses about to be built
//! (green where they may stand, red where not) and a dot for each door (blue) and guard (red). A building item of the
//! inventory let go on the map (build mode) waits there as a house about to be built, its least corner on the tile
//! under the pointer; it can be dragged, turned ("Rotate") and taken off ("Delete"). "Construct" asks the server for
//! every waiting house at once, the headquarters first; "Demolish" for the selected standing house's demolishing.
//! **Neither asks first, as the original's do not, and neither can be taken back.** The map only shows, and the
//! buttons are not there, unless the lease mode is 3 and the opening's third word 0 or 1.
//!
//! The window's close request (FUN_100e7203) only asks the server to end the session: the window stays till the
//! server's code 7.
//!
//! **GUI type 1 is the dialog that buys the land** (the same opening with its type 1; FUN_100e7339): 389 by 239, two
//! tabs. "Buy Land Area" (`BuyLandView_c`, FUN_100e6388, built in code, not from the XML): the text
//! `CityTerminal_BuyLandDialogText` fed the land's price and its upkeep (the city record's two numbers), and the
//! buttons Purchase and Cancel. Purchase can be pressed in lease mode 3 only, and its press sends the request
//! (`ClientRequestBuyIIR_c`, FUN_100e6225) **at once: the original asks nothing first, and the price is in the
//! hundreds of millions**. Cancel and the window's close ask the server to end the session. "City Map": the map,
//! shown only. No capture has a type 1 opening, a purchase or its answer: all of it rests on the code.
//!
//! An item that waits on the map is locked in the inventory (`InventoryItemLocker_c`: `gear::city_locked`, drawn and
//! refused in `invwindows.rs`) until it is taken off the map, built, or the window goes. Over the map the pointer's
//! place in the world is told as a tip (FUN_100478c7: "%.1fx%.1f" of x and z, not snapped) once it has rested.

use crate::citymap::{Build, Geometry, Map, Mode, Refusal, TILE};
use crate::gui::view::{Id, Tree};
use crate::theme::Palette;
use mao_net::city::{CityId, Signal};

/// The session's state: what `CityClientInterface_c` keeps (+0x34 on).
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub city: CityId,
    pub gui_type: i32,
    pub lease: i32,
    /// The third word of the opening: building and demolishing want it 0 or 1.
    pub third: i32,
    pub org: String,
    pub upkeep: i32,
    /// When the next bill comes (unix seconds, this machine's clock: the time code 13 came plus its seconds).
    pub bill: Option<i64>,
    pub cloak: i32,
    /// `IsCloakingTogglePossible`: the state is not 0 and the time told with it was below 0.
    pub cloak_possible: bool,
    pub charge: f32,
}

impl Session {
    /// The session code 5 opens.
    pub fn open(signal: &Signal) -> Option<Session> {
        let Signal::Open { city, gui_type, lease, third, org, .. } = signal else { return None };
        Some(Session { city: *city, gui_type: *gui_type, lease: *lease, third: *third, org: org.clone(), upkeep: 0, bill: None, cloak: 0, cloak_possible: false, charge: 0.0 })
    }

    /// A signal heard while the session is open (the receivers `AOCityClientInterface_c` registers, Gamecode
    /// FUN_1012be1d: codes 6, 10, 13, 14, 15, 16; 7 is the caller's). Whether it changed anything.
    pub fn apply(&mut self, signal: &Signal, now: i64) -> bool {
        let before = self.clone();
        match signal {
            Signal::Lease(l) => self.lease = *l,
            Signal::Upkeep(u) => self.upkeep = *u,
            Signal::BillIn(seconds) => self.bill = Some(now + *seconds as i64),
            Signal::Cloak { state, time } => {
                self.cloak = *state;
                self.cloak_possible = *state != 0 && *time < 0;
            }
            Signal::Charge(c) => self.charge = *c,
            Signal::OrgName(name) => self.org = name.clone(),
            _ => {}
        }
        *self != before
    }

    /// Whether the window's doing controls are enabled: lease mode 3 alone.
    pub fn full(&self) -> bool {
        self.lease == 3
    }

    /// Whether the map may build and demolish (`HouseBuildView_c`'s read-only flag, FUN_100e7e89, the other way
    /// round): lease mode 3 and the opening's third word below 2, a signed compare (the window's +0x78 and +0xa0 as
    /// FUN_100e755a makes the map view, FUN_100e8108's last argument).
    pub fn may_build(&self) -> bool {
        self.lease == 3 && self.third <= 1
    }
}

/// What the window asks of the network loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Command {
    /// The close request: the server is asked to end the session.
    #[default]
    Close,
    /// "Dump the city's advantages": the organisation command 0x1f.
    Advantages,
    /// The cloaking device switched the other way.
    ToggleCloak,
    /// The standing house on this tile to be demolished.
    Demolish(i32, i32),
    /// The houses in [`BUILDS`] to be built.
    Construct,
    /// The land bought (GUI type 1's Purchase).
    Buy,
}

/// The houses "Construct" asks for, till the network loop takes them (the command itself carries nothing: the loop's
/// input is one small value a frame).
pub static BUILDS: std::sync::Mutex<Vec<Build>> = std::sync::Mutex::new(Vec::new());

/// The next bill's date as the original writes it: `strftime("%#x")` of the local time, the C runtime's long date. No
/// module of the game calls `setlocale` (only msvcp100.dll imports it), so the runtime stays in the "C" locale, whose
/// long date is "Sunday, November 15, 2026" whatever the system's language; `#` drops the day's leading zero.
pub fn long_date(unix: i64) -> String {
    let Ok(utc) = time::OffsetDateTime::from_unix_timestamp(unix) else { return String::new() };
    let local = time::UtcOffset::local_offset_at(utc).map_or(utc, |o| utc.to_offset(o));
    format!("{}, {} {}, {}", local.weekday(), local.month(), local.day(), local.year())
}

/// What a tab shows: a view of its own, or the house view in one of its modes.
#[derive(Clone, Copy, PartialEq)]
enum Page {
    View(usize),
    Houses(Mode),
}

struct Views {
    trees: Vec<(Tree, Id)>,
    /// The tabs' names and pages.
    tabs: Vec<(String, Page)>,
    /// The tree with the map's dock (the house view, or the inspect panel).
    map: Option<usize>,
}

/// The mouse on the map (`BuildingMapView_c` +0x4b8): a pending node dragged (the pointer's place from the node's
/// least corner as the drag began), or the map scrolled.
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Node([f32; 2]),
    Scroll,
}

#[derive(Default)]
pub struct CityTerminalWindow {
    pub session: Option<Session>,
    pub map: Map,
    views: Option<Views>,
    /// What the views were last filled from.
    shown: Option<Session>,
    tab: usize,
    drag: Option<Drag>,
}

fn set_text(tree: &mut Tree, root: Id, name: &str, text: &str) {
    if let Some(id) = tree.find(root, name)
        && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id)
        && t.text != text
    {
        t.set_text(text);
        tree.relayout = true;
    }
}

/// The charge bar in its dock (`PowerbarView_c` with the pictures 0xdb and 0xdd, the empty bar and the red one).
fn add_charge_bar(tree: &mut Tree, root: Id) {
    let Some(dock) = tree.find(root, "ChargeBarDock") else { return };
    let bar = crate::gui::xml::parse("<PowerBar name=\"ChargeBar\" bg_gfx=\"GFX_GUI_HOR_BAR_EMPTY\" full_gfx=\"GFX_GUI_HOR_BAR_RED\"/>");
    if let Some(id) = bar.first().and_then(|e| crate::gui::create(tree, e)) {
        tree.get_mut(id).tooltip = crate::gui::xml::ldb_named(10000, "ControllerCharge", "Controller Charge");
        // (the dock is a plain view: the bar is laid in it at its own size, as the original's is made with its rect)
        tree.get_mut(dock).layout = crate::gui::view::Layout::Vertical;
        tree.add_child(dock, id);
    }
}

/// The buying page's text: `CityTerminal_BuyLandDialogText` fed the land's price, then its upkeep (FUN_100e6388).
pub fn buy_text(costs: (u32, u32)) -> String {
    let text = crate::gui::xml::ldb_named(10002, "CityTerminal_BuyLandDialogText", "This land costs %d credits, and %d credits of upkeep.");
    mao_formats::ldb::format(&text, &[mao_formats::ldb::Arg::Int(costs.0 as i64), mao_formats::ldb::Arg::Int(costs.1 as i64)])
}

/// The map tip's body for a place in the world (x, z): the original's "%.1fx%.1f".
pub fn pointer_text(place: [f32; 2]) -> String {
    format!("{:.1}x{:.1}", place[0], place[1])
}

/// The text of a drop's refusal (the original's error box).
fn refusal_text(refusal: &Refusal) -> String {
    match refusal {
        Refusal::NotAHouse => crate::gui::xml::ldb_named(10000, "OnlyDropHousesOnMap", "Only building items can be put on the map."),
        Refusal::Side(side) => {
            let name = crate::feedback::texts().and_then(|t| t.get(2005, *side as i32)).unwrap_or("another side").to_string();
            let text = crate::gui::xml::ldb_named(10002, "CityTerminal_HouseCanOnlyBeBuiltBy", "This building can only be built by %s.");
            mao_formats::ldb::format(&text, &[mao_formats::ldb::Arg::Text(&name)])
        }
        Refusal::Limit(key) => crate::gui::xml::ldb_named(10000, key, "The city cannot have another building of this kind."),
    }
}

impl CityTerminalWindow {
    /// The server's word on the session: `Some` as it opens or changes, `None` as it ends.
    pub fn set(&mut self, session: Option<Session>) {
        // (a window of another kind is closed first: the views are made anew)
        let kind = |s: &Session| (s.gui_type, s.lease == 1, s.may_build());
        if session.as_ref().map(kind) != self.session.as_ref().map(kind) {
            self.views = None;
            self.tab = 0;
        }
        if session.is_none() {
            self.map = Map::default();
            self.drag = None;
        }
        if let Some(s) = &session {
            self.map.no_market = s.city.model.0 == 0xc79e;
        }
        self.session = session;
        self.shown = None;
    }

    /// The network loop's word on the houses, the building items and the zones.
    pub fn set_map(&mut self, data: crate::citymap::Data) {
        // (the land's price is in the buying page's text)
        if data.costs != self.map.costs {
            self.shown = None;
        }
        self.map.set(data);
    }

    /// A sample for a headless preview: a made-up session over made-up zones, two houses standing and two waiting.
    pub fn sample() -> CityTerminalWindow {
        use crate::citymap::{Buildable, Data, Node, Shape, Zones};
        let mut w = CityTerminalWindow::default();
        w.set(Some(Session { city: CityId { city: (0xc419, 0xc000), playfield: 1, model: (0xc79e, 6010) }, gui_type: std::env::var("MAO_CITY_TYPE").ok().and_then(|t| t.parse().ok()).unwrap_or(2), lease: std::env::var("MAO_CITY_LEASE").ok().and_then(|t| t.parse().ok()).unwrap_or(3), third: 0, org: "An organisation".into(), upkeep: 15_000_000, bill: Some(1_794_750_000), cloak: 1, cloak_possible: true, charge: 0.6 }));
        w.tab = std::env::var("MAO_CITY_TAB").ok().and_then(|t| t.parse().ok()).unwrap_or(0);
        let shape = |kind: i32, tiles: (i32, i32)| Shape { kind, tiles, mask: vec![0xff; ((tiles.0 * tiles.1 + 7) / 8) as usize], doors: vec![[tiles.0 as f32 * 2.0, 0.0, 1.0]], guards: vec![[2.0, 0.0, tiles.1 as f32 * 4.0 - 2.0]] };
        let standing = |kind: i32, tiles: (i32, i32), tile: (i32, i32), demolishing: bool| Node { item: None, template: kind as u32, shape: shape(kind, tiles), tile, rotation: 0, demolishing, valid: true };
        let zones = Zones((0..3).flat_map(|z| (0..4).map(move |x| ((210 + x * 10, 280 + z * 10), if (x + z) % 2 == 0 { [0xff; 13] } else { [0x55; 13] }))).collect());
        w.set_map(Data {
            houses: vec![standing(1, (8, 6), (211, 301), false), standing(0, (4, 4), (222, 301), true)],
            buildables: vec![Buildable { slot: 70, template: 50, side: None, shape: shape(0, (5, 3)) }, Buildable { slot: 71, template: 51, side: None, shape: shape(0, (3, 3)) }],
            zones,
            tiles: (600, 600),
            costs: (500_000_000, 15_000_000),
        });
        let _ = w.map.drop_item(70, (231, 302), None);
        let _ = w.map.drop_item(71, (224, 303), None);
        w.map.select(Some(0));
        w
    }

    fn load(ctx: &egui::Context, session: &Session) -> Option<Views> {
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        let text = dirs.iter().find_map(|d| mao_formats::files::read(d.join("Views").join("CityTerminal.xml")).ok())?;
        let document = crate::gui::xml::parse(&String::from_utf8_lossy(&text));
        let view = |name: &str| {
            let mut tree = Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            let root = crate::gui::create(&mut tree, &crate::gui::xml::view_element(&document, name)?)?;
            Some((tree, root))
        };
        let caption = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
        let (mut trees, mut tabs, map);
        (trees, tabs) = (Vec::new(), Vec::new());
        if session.gui_type == 1 {
            // (FUN_100e7339: the buying page, made in code as FUN_100e6388 makes it — a spacer, the text with borders
            // of 5, a spacer, and the two buttons at the right — and the map, shown only: mode 2, no control view)
            // (the original gives the text an aspect ratio of 2, which this layout has not: a least height stands in)
            let escape = |t: String| t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
            let label = |name: &str, own: &str| escape(crate::gui::xml::ldb_named(10002, name, own));
            let page = format!(
                "<View name=\"BuyLandView\" view_layout=\"vertical\"><VLayoutSpacer/>\
                 <TextView feature_flags=\"TVF_MULTILINE|TVF_WORD_WRAP\" layout_borders=\"Rect(5,5,5,5)\" max_size=\"Point(16000,16000)\" min_size=\"Point(0,120)\" name=\"BuyLandText\"/>\
                 <VLayoutSpacer/><View view_layout=\"horizontal\"><HLayoutSpacer/>\
                 <Button label=\"{}\" layout_borders=\"Rect(5,0,0,5)\" name=\"BuyLandPurchaseButton\"/>\
                 <Button label=\"{}\" layout_borders=\"Rect(5,0,5,5)\" name=\"BuyLandCancelButton\"/></View></View>",
                label("CityTerminal_BuyLandPurchaseButLabel", "Purchase"),
                label("CityTerminal_BuyLandCancelButLabel", "Cancel")
            );
            let mut tree = Tree::default();
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            let root = crate::gui::create(&mut tree, crate::gui::xml::parse(&page).first()?)?;
            trees.push((tree, root));
            tabs.push((caption("BuyLandArea", "Buy Land Area"), Page::View(0)));
            trees.push(view("CityMapView")?);
            map = Some(1);
            tabs.push((caption("CityMap", "City Map"), Page::Houses(Mode::View)));
        } else if session.lease == 1 {
            let (mut tree, root) = view("InspectCityPanel")?;
            add_charge_bar(&mut tree, root);
            map = Some(0);
            trees.push((tree, root));
            tabs.push((caption("CityTerminal", "City Terminal"), Page::View(0)));
        } else {
            let (mut tree, root) = view("UpkeepController")?;
            add_charge_bar(&mut tree, root);
            set_text(&mut tree, root, "InfoTextView", &crate::gui::xml::ldb_named(10002, "CityTerminal_UpkeepControllerHelpText", ""));
            trees.push((tree, root));
            tabs.push((caption("UpkeepController", "Upkeep"), Page::View(0)));
            // (`HouseBuildView_c` FUN_100e8108: the map's view with the control view beside it, shown by two tabs; the
            // control view is not there when the map only shows)
            let (mut tree, root) = view("CityMapView")?;
            if session.may_build()
                && let Some(controls) = crate::gui::xml::view_element(&document, "CityMapControlView").and_then(|e| crate::gui::create(&mut tree, &e))
            {
                tree.add_child(root, controls);
            }
            map = Some(1);
            trees.push((tree, root));
            tabs.push((caption("BuildHouses", "Build"), Page::Houses(Mode::Build)));
            tabs.push((caption("DemolishHouses", "Demolish"), Page::Houses(Mode::Demolish)));
            let (mut tree, root) = view("CloakingController")?;
            if let Some(id) = tree.find(root, "EnableCloakingDeviceButton")
                && let Some(b) = tree.control_mut::<crate::gui::controls::Button>(id)
            {
                b.toggle = true;
            }
            trees.push((tree, root));
            tabs.push((caption("Misc", "Misc"), Page::View(2)));
        }
        // (the map's dock takes the room there is: the original puts a scroll view in it, which does)
        for (tree, root) in &mut trees {
            if let Some(dock) = tree.find(*root, "MapViewDock") {
                tree.get_mut(dock).max_extend = crate::gui::geom::Point::new(16000.0, 16000.0);
            }
        }
        Some(Views { trees, tabs, map })
    }

    /// The views filled from the session (the slots of the interface's signals: FUN_100e91af, FUN_100e92a4,
    /// FUN_100e91c6, FUN_100e9242, FUN_100e918e, FUN_100e70c5).
    fn fill(views: &mut Views, s: &Session, costs: (u32, u32)) {
        for (tree, root) in &mut views.trees {
            let root = *root;
            // (the buying page: the price and the upkeep in its text; Purchase with lease mode 3 only, FUN_100e6208)
            set_text(tree, root, "BuyLandText", &buy_text(costs));
            if let Some(id) = tree.find(root, "BuyLandPurchaseButton") {
                tree.get_mut(id).enabled = s.full();
            }
            set_text(tree, root, "OrgNameView", &s.org);
            set_text(tree, root, "UpkeepDateView", &s.bill.map(long_date).unwrap_or_default());
            set_text(tree, root, "UpkeepAmountView", &crate::vitals::grouped(s.upkeep as i64));
            if let Some(id) = tree.find(root, "ChargeBar")
                && let Some(bar) = tree.control_mut::<crate::gui::powerbar::PowerBar>(id)
            {
                bar.set_value(s.charge);
            }
            if let Some(id) = tree.find(root, "DumpAdvantagesButton") {
                tree.get_mut(id).enabled = s.full();
            }
            if let Some(id) = tree.find(root, "EnableCloakingDeviceButton") {
                tree.get_mut(id).enabled = s.full() && s.cloak_possible;
                if let Some(b) = tree.control_mut::<crate::gui::controls::Button>(id) {
                    b.on = s.cloak > 0;
                }
            }
        }
    }

    /// The house view's mode (FUN_100e7f62): its text, its page of buttons, and which of them can be pressed
    /// (FUN_100e7eb2, FUN_100e7ec9).
    fn house_controls(tree: &mut Tree, root: Id, map: &Map) {
        let (text, page) = match map.mode {
            Mode::Demolish => ("CityTerminal_DemolishInfoText", 1),
            _ => ("CityTerminal_BuildMapInfoText", 0),
        };
        set_text(tree, root, "InfoTextView", &crate::gui::xml::ldb_named(10002, text, ""));
        if let Some(selector) = tree.find(root, "ButtonSelector")
            && crate::gui::selector::value(tree, selector) != page
        {
            crate::gui::selector::set_value(tree, selector, page);
            tree.relayout = true;
        }
        for (button, enabled) in [("RotateButton", map.pending_selected()), ("DeleteButton", map.pending_selected()), ("ConstructButton", map.can_construct), ("DemolishButton", map.standing_selected())] {
            if let Some(id) = tree.find(root, button) {
                tree.get_mut(id).enabled = enabled;
            }
        }
    }

    /// The map in its dock: drawn, and the mouse on it.
    fn show_map(&mut self, ui: &mut egui::Ui, palette: &Palette, ground: Option<(egui::TextureId, [f32; 2])>, own_side: Option<i64>) {
        let Some(g) = Geometry::new(&self.map.zones, self.map.tiles) else { return };
        let skin = crate::skin::skin(ui.ctx());
        let (map, drag) = (&mut self.map, &mut self.drag);
        egui::ScrollArea::both().id_salt("city-map").auto_shrink([false, false]).scroll_source(egui::scroll_area::ScrollSource { drag: egui::scroll_area::DragScroll::Never, ..egui::scroll_area::ScrollSource::ALL }).show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(egui::vec2(g.size[0], g.size[1]), egui::Sense::click_and_drag());
            let visible = ui.clip_rect();
            let painter = ui.painter_at(rect.intersect(visible));
            let at = |p: [f32; 2]| rect.min + egui::vec2(p[0], p[1]);
            // (black, then the ground from above under its veil, as the playfield map draws it)
            painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
            if let Some((texture, extent)) = ground {
                let (low, right) = (g.top - g.size[1] / crate::citymap::SCALE, g.left + g.size[0] / crate::citymap::SCALE);
                let uv = egui::Rect::from_min_max(egui::pos2(g.left / extent[0], g.top / extent[1]), egui::pos2(right / extent[0], low / extent[1]));
                let mut mesh = egui::Mesh::with_texture(texture);
                mesh.add_rect_with_uv(rect, uv, egui::Color32::from_gray(crate::automap::VEIL));
                painter.add(egui::Shape::mesh(mesh));
            }
            // (the overlay's tiles, a later one in place of an earlier: the zones, the standing houses, the waiting
            // ones, the selected one; FUN_100e3fe7's words as colours)
            let colour = |r: u8, gr: u8, b: u8, a: u8| egui::Color32::from_rgba_unmultiplied(r, gr, b, a);
            let mut tiles: std::collections::HashMap<(i32, i32), egui::Color32> = map.zones.tiles().map(|t| (t, colour(0, 0, 255, 68))).collect();
            let selected = map.selected;
            for (_, n) in map.nodes.iter().enumerate().filter(|(i, n)| !n.pending() && Some(*i) != selected) {
                tiles.extend(n.tiles().map(|t| (t, if n.demolishing { colour(0, 136, 136, 136) } else { colour(136, 136, 0, 136) })));
            }
            for (_, n) in map.nodes.iter().enumerate().filter(|(i, n)| n.pending() && Some(*i) != selected) {
                tiles.extend(n.tiles().map(|t| (t, if n.valid { colour(0, 136, 0, 136) } else { colour(136, 0, 0, 136) })));
            }
            if let Some(n) = selected.and_then(|i| map.nodes.get(i)) {
                let c = match (n.pending(), n.valid) {
                    (false, _) => colour(255, 255, 0, 136),
                    (true, true) => colour(0, 255, 0, 136),
                    (true, false) => colour(255, 0, 0, 136),
                };
                tiles.extend(n.tiles().map(|t| (t, c)));
            }
            let side = TILE * crate::citymap::SCALE;
            for (tile, c) in &tiles {
                let r = egui::Rect::from_min_size(at(g.tile_pixel(*tile)), egui::vec2(side, side));
                if r.intersects(visible) {
                    painter.rect_filled(r, 0.0, *c);
                }
            }
            // (a dot for each door, then for each guard, of every node: `GFX_GUI_MAPDOT_BLUE` and `_RED`, 4 pixels: the
            // marker atlas, U:0x10046d0b, keeps only each picture's top-left 4 x 4 texels)
            for (guards, picture, own) in [(false, "GFX_GUI_MAPDOT_BLUE", colour(60, 120, 255, 255)), (true, "GFX_GUI_MAPDOT_RED", colour(255, 60, 60, 255))] {
                let sprite = skin.as_deref().and_then(|s| s.get(picture));
                for n in &map.nodes {
                    for local in if guards { &n.shape.guards } else { &n.shape.doors } {
                        let r = egui::Rect::from_center_size(at(g.marker_pixel(n.place(*local))), egui::vec2(4.0, 4.0));
                        match &sprite {
                            Some(s) => {
                                painter.image(s.texture.id(), r, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(4.0 / s.size.x, 4.0 / s.size.y)), egui::Color32::WHITE);
                            }
                            None => {
                                painter.rect_filled(r, 0.0, own);
                            }
                        }
                    }
                }
            }
            // The mouse (FUN_100e56a8, FUN_100e4abf, FUN_100e34e9).
            let pointer = ui.input(|i| i.pointer.interact_pos()).filter(|p| rect.contains(*p) && visible.contains(*p));
            let content = |p: egui::Pos2| [p.x - rect.min.x, p.y - rect.min.y];
            let held = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
            // (the pointer's place as a tip, FUN_100478c7: the world's x and z under it, not snapped, in every mode;
            // it comes when the pointer has rested, as every tip does, and a button down takes it away)
            let measure = crate::gui::font::Measure { ctx: Some(ui.ctx().clone()) };
            let env = crate::gui::host::Env { skin: skin.as_deref(), palette, measure: &measure };
            let source = egui::Id::new("city-map-tip");
            let place = pointer.filter(|_| held == u32::MAX && !ui.input(|i| i.pointer.any_down())).map(|p| g.world(content(p)));
            let body = place.map_or(String::new(), pointer_text);
            crate::gui::tooltip::hover(ui.ctx(), &env, source, place.map(|_| source.with(&body)), &crate::gui::xml::ldb_named(10000, "PointerPosition", "Pointer position"), &body);
            if map.mode == Mode::Build && held != u32::MAX {
                // (an inventory item let go over the map: a house to build with its least corner on that tile, or an
                // error box. The hand is emptied either way.)
                if crate::gear::letting_go(ui.ctx())
                    && let Some(p) = pointer
                {
                    if let Err(refusal) = map.drop_item(held, g.tile_at(content(p)), own_side) {
                        crate::chrome::alert(refusal_text(&refusal));
                    }
                    crate::gear::DRAGGED.store(u32::MAX, std::sync::atomic::Ordering::Relaxed);
                }
                return;
            }
            // (`BuildingMapView_c`'s MouseDown never reads its button: any button selects, drags a house or scrolls)
            let (pressed, down) = ui.input(|i| (i.pointer.any_pressed(), i.pointer.any_down()));
            if pressed && let Some(p) = pointer {
                let p = content(p);
                *drag = Some(Drag::Scroll);
                if map.mode != Mode::View {
                    let hit = map.hit(g.tile_at(p));
                    if hit != map.selected {
                        map.select(hit);
                    }
                    // (a waiting house under the pointer in build mode is dragged, held where it was taken)
                    if map.mode == Mode::Build
                        && let Some(n) = hit.and_then(|i| map.nodes.get(i)).filter(|n| n.pending())
                    {
                        let corner = [(n.tile.0 as f32 * TILE - g.left) * crate::citymap::SCALE, (g.top - n.tile.1 as f32 * TILE) * crate::citymap::SCALE];
                        *drag = Some(Drag::Node([p[0] - corner[0], p[1] - corner[1]]));
                    }
                }
            }
            if !down {
                *drag = None;
            }
            match *drag {
                Some(Drag::Node(anchor)) => {
                    if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                        let p = content(p);
                        map.move_selected(g.tile_at([p[0] - anchor[0], p[1] - anchor[1]]));
                    }
                }
                Some(Drag::Scroll) => {
                    let delta = response.drag_delta();
                    if delta != egui::Vec2::ZERO {
                        ui.scroll_with_delta(delta);
                    }
                }
                None => {}
            }
        });
    }

    /// Draws the window; what it asks for. `ground` is the playfield's ground from above (the playfield map's
    /// picture and the world units it spans); `own_side` the character's side (stat 33).
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette, ground: Option<(egui::TextureId, [f32; 2])>, own_side: Option<i64>) -> Option<Command> {
        let Some(session) = self.session.clone() else {
            self.views = None;
            return None;
        };
        // (`SlotOpenCityTerminal`: types 1 and 2 have a window)
        if !(1..=2).contains(&session.gui_type) {
            return None;
        }
        if self.views.is_none() {
            self.views = Self::load(ctx, &session);
            self.shown = None;
        }
        let views = self.views.as_mut()?;
        if self.shown.as_ref() != Some(&session) {
            Self::fill(views, &session, self.map.costs);
            self.shown = Some(session.clone());
        }
        let skin = crate::skin::skin(ctx);
        let names: Vec<String> = views.tabs.iter().map(|t| t.0.clone()).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut tab = self.tab.min(views.tabs.len().saturating_sub(1));
        let mut is_open = true;
        // (the buying dialog is a window of its own size and place: `CityTerminalBuyDlgConfig`)
        let (name, size) = if session.gui_type == 1 { ("CityTerminalBuy", egui::vec2(389.0, 239.0)) } else { ("CityTerminal", egui::vec2(499.0, 499.0)) };
        let middle = ctx.content_rect().center() - size / 2.0;
        // (the map's dock on the screen, when the open tab has it)
        let mut dock = None;
        let (tabs, trees, map_tree) = (&views.tabs, &mut views.trees, views.map);
        let may_build = session.may_build();
        let map = &mut self.map;
        crate::chrome::window(ctx, palette, name, "", &mut is_open, |w| w.default_size(size).resizable(true).default_pos(crate::layout::pos(name, middle)), |ui| {
            crate::chrome::tab_view(ui, palette, &names, &mut tab, |ui, tab| {
                let Some(&(_, page)) = tabs.get(tab) else { return };
                let index = match page {
                    Page::View(i) => i,
                    Page::Houses(_) => 1,
                };
                // (the map's mode follows the tab, FUN_100e7169; it only shows where nothing may be built)
                if Some(index) == map_tree {
                    map.mode = match page {
                        Page::Houses(mode) if may_build => mode,
                        _ => Mode::View,
                    };
                }
                let Some((tree, root)) = trees.get_mut(index) else { return };
                if matches!(page, Page::Houses(_)) {
                    Self::house_controls(tree, *root, map);
                }
                let response = crate::gui::host::show(ui, tree, *root, skin.as_deref(), palette);
                if Some(index) == map_tree
                    && let Some(id) = tree.find(*root, "MapViewDock")
                {
                    let f = tree.window_frame(id);
                    let rect = egui::Rect::from_min_max(response.rect.min + egui::vec2(f.left, f.top), response.rect.min + egui::vec2(f.right + 1.0, f.bottom + 1.0));
                    dock = Some((ui.layer_id(), ui.clip_rect(), rect));
                }
            });
        });
        self.tab = tab;
        if let Some((layer, clip, rect)) = dock {
            // (drawn after the window's own views, in their layer, inside the dock)
            let mut ui = egui::Ui::new(ctx.clone(), egui::Id::new("city-map-dock"), egui::UiBuilder::new().layer_id(layer).max_rect(rect));
            ui.set_clip_rect(rect.intersect(clip));
            self.show_map(&mut ui, palette, ground, own_side);
        }
        let views = self.views.as_mut()?;
        let mut command = None;
        for (tree, _) in views.trees.iter_mut() {
            for (id, signal) in std::mem::take(&mut tree.signals) {
                if signal != crate::gui::host::Signal::Clicked {
                    continue;
                }
                match tree.get(id).name.as_str() {
                    "DumpAdvantagesButton" => command = Some(Command::Advantages),
                    // (FUN_100e6225 and FUN_100e62ba: the purchase asked for at the press, nothing asked first; Cancel
                    // is the close request)
                    "BuyLandPurchaseButton" => command = Some(Command::Buy),
                    "BuyLandCancelButton" => command = Some(Command::Close),
                    "EnableCloakingDeviceButton" => {
                        command = Some(Command::ToggleCloak);
                        // (the button shows the server's word, not the press: it turns when code 14 comes)
                        self.shown = None;
                    }
                    "RotateButton" => self.map.rotate(),
                    "DeleteButton" => self.map.delete(),
                    "ConstructButton" => {
                        let builds = self.map.construct();
                        if !builds.is_empty() {
                            BUILDS.lock().unwrap_or_else(|e| e.into_inner()).extend(builds);
                            command = Some(Command::Construct);
                        }
                    }
                    "DemolishButton" => {
                        if let Some((x, z)) = self.map.demolish() {
                            command = Some(Command::Demolish(x, z));
                        }
                    }
                    _ => {}
                }
            }
        }
        if !is_open {
            command = Some(Command::Close);
        }
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> Signal {
        Signal::Open { city: CityId { city: (0xc419, 0xc000), playfield: 7, model: (0xc79e, 6010) }, identity: (0, 0), gui_type: 2, lease: 3, third: 0, org: "A name".into() }
    }

    #[test]
    fn the_session_takes_the_servers_signals() {
        let mut s = Session::open(&open()).unwrap();
        assert!(s.full() && s.may_build() && !s.cloak_possible && s.bill.is_none());
        assert!(s.apply(&Signal::Upkeep(15_000_000), 100));
        assert!(s.apply(&Signal::BillIn(3600), 100));
        assert_eq!((s.upkeep, s.bill), (15_000_000, Some(3700)));
        // (the device may be switched when it has a state and the time told is below 0)
        assert!(s.apply(&Signal::Cloak { state: 1, time: -1 }, 100));
        assert!(s.cloak > 0 && s.cloak_possible);
        s.apply(&Signal::Cloak { state: -1, time: 600 }, 100);
        assert!(s.cloak < 0 && !s.cloak_possible);
        s.apply(&Signal::Cloak { state: 0, time: -1 }, 100);
        assert!(!s.cloak_possible);
        assert!(s.apply(&Signal::Charge(0.25), 100) && s.charge == 0.25);
        assert!(s.apply(&Signal::OrgName("Another".into()), 100) && s.org == "Another");
        // (the same again changes nothing; a lease mode below 3 enables nothing)
        assert!(!s.apply(&Signal::Charge(0.25), 100));
        assert!(s.apply(&Signal::Lease(2), 100) && !s.full() && !s.may_build());
        assert_eq!(Session::open(&Signal::Lease(3)), None);
        // (lease mode 3 with the third word past 1: everything but the map's building)
        let s = Session { third: 2, ..Session::open(&open()).unwrap() };
        assert!(s.full() && !s.may_build());
    }

    /// The buying page's text carries the price and then the upkeep; the map tip is the world's x and z to a tenth.
    #[test]
    fn the_buying_text_and_the_map_tip() {
        let text = buy_text((500_000_000, 15_000_000));
        let (price, upkeep) = (text.find("500000000"), text.find("15000000"));
        assert!(price.is_some() && upkeep.is_some() && price < upkeep, "{text}");
        assert_eq!(pointer_text([2104.5, 3310.0]), "2104.5x3310.0");
        // (a type 1 opening is a session of its own kind, with Purchase only in lease mode 3)
        let s = Session { gui_type: 1, ..Session::open(&open()).unwrap() };
        assert!(s.full());
        assert!(!Session { lease: 2, ..s }.full());
    }

    #[test]
    fn a_window_of_another_kind_is_made_anew() {
        let mut w = CityTerminalWindow::default();
        let s = Session::open(&open()).unwrap();
        w.set(Some(s.clone()));
        assert!(w.map.no_market);
        w.tab = 3;
        // (the same kind: the tab stays)
        w.set(Some(Session { charge: 0.5, ..s.clone() }));
        assert_eq!(w.tab, 3);
        // (lease mode 1 is the one-tab window)
        w.set(Some(Session { lease: 1, ..s }));
        assert_eq!(w.tab, 0);
        w.set(None);
        assert!(w.session.is_none() && w.map.nodes.is_empty());
    }

    #[test]
    fn the_long_date() {
        // (noon UTC of 2026-11-15, a Sunday, is the same day in every zone a player's clock is likely in)
        let text = long_date(1_794_744_000);
        assert!(text.ends_with(", 2026") && text.contains("November"), "{text}");
        // (`%#x`: no leading zero on a day under 10; noon UTC of 2026-11-05, a Thursday)
        assert_eq!(long_date(1_793_880_000), "Thursday, November 5, 2026");
    }
}
