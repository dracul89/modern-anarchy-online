//! A city's controller window, GUI.dll's `CityTerminalWindow_c` (FUN_100e7847 on `Views/CityTerminal.xml`), and the
//! session behind it, city.dll's `CityClientInterface_c`. The whole reading is `docs/CITY_CONTROLLER_ORIGINAL.md`.
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
//! The window's close request (FUN_100e7203) only asks the server to end the session: the window stays till the
//! server's code 7.
//!
//! **Not here yet:** the city's map (`BuildingMapView_c`), so the build and demolish tabs show their texts and their
//! buttons, none of which can be pressed without a house chosen on the map; and GUI type 1, the dialog that buys the
//! land.

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
}

/// The next bill's date as the original writes it (`strftime("%#x")`, the long date of the user's locale: here the
/// English one, "Sunday, November 15, 2026").
pub fn long_date(unix: i64) -> String {
    let Ok(utc) = time::OffsetDateTime::from_unix_timestamp(unix) else { return String::new() };
    let local = time::UtcOffset::local_offset_at(utc).map_or(utc, |o| utc.to_offset(o));
    format!("{}, {} {:02}, {}", local.weekday(), local.month(), local.day(), local.year())
}

struct Views {
    /// The tabs' names and views.
    tabs: Vec<(String, Tree, Id)>,
}

#[derive(Default)]
pub struct CityTerminalWindow {
    pub session: Option<Session>,
    views: Option<Views>,
    /// What the views were last filled from.
    shown: Option<Session>,
    tab: usize,
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

impl CityTerminalWindow {
    /// The server's word on the session: `Some` as it opens or changes, `None` as it ends.
    pub fn set(&mut self, session: Option<Session>) {
        // (a window of another kind is closed first: the views are made anew)
        if session.as_ref().map(|s| (s.gui_type, s.lease == 1)) != self.session.as_ref().map(|s| (s.gui_type, s.lease == 1)) {
            self.views = None;
            self.tab = 0;
        }
        self.session = session;
        self.shown = None;
    }

    /// A sample for a headless preview.
    pub fn sample() -> CityTerminalWindow {
        let mut w = CityTerminalWindow::default();
        w.set(Some(Session { city: CityId { city: (0xc419, 0xc000), playfield: 1, model: (0xc79e, 6010) }, gui_type: 2, lease: 3, third: 0, org: "An organisation".into(), upkeep: 15_000_000, bill: Some(1_794_750_000), cloak: 1, cloak_possible: true, charge: 0.6 }));
        w.tab = std::env::var("MAO_CITY_TAB").ok().and_then(|t| t.parse().ok()).unwrap_or(0);
        w
    }

    fn load(ctx: &egui::Context, session: &Session) -> Option<Views> {
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        let view = |name: &str| {
            let (mut tree, root) = crate::gui::load_view(&dirs, "CityTerminal.xml", name)?;
            tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
            Some((tree, root))
        };
        let caption = |name: &str, own: &str| crate::gui::xml::ldb_named(10000, name, own);
        let long = |name: &str, own: &str| crate::gui::xml::ldb_named(10002, name, own);
        let mut tabs = Vec::new();
        if session.lease == 1 {
            let (mut tree, root) = view("InspectCityPanel")?;
            add_charge_bar(&mut tree, root);
            tabs.push((caption("CityTerminal", "City Terminal"), tree, root));
        } else {
            let (mut tree, root) = view("UpkeepController")?;
            add_charge_bar(&mut tree, root);
            set_text(&mut tree, root, "InfoTextView", &long("CityTerminal_UpkeepControllerHelpText", ""));
            tabs.push((caption("UpkeepController", "Upkeep"), tree, root));
            // (`HouseBuildView_c`: the map's view and the control view beside it, once for each of its two tabs here)
            for (key, own, text, page) in [("BuildHouses", "Build", "CityTerminal_BuildMapInfoText", 0), ("DemolishHouses", "Demolish", "CityTerminal_DemolishInfoText", 1)] {
                let (mut tree, root) = view("CityMapControlView")?;
                set_text(&mut tree, root, "InfoTextView", &long(text, ""));
                if let Some(selector) = tree.find(root, "ButtonSelector") {
                    crate::gui::selector::set_value(&mut tree, selector, page);
                }
                // (no map yet: nothing can be chosen, so nothing can be pressed)
                for button in ["RotateButton", "DeleteButton", "ConstructButton", "DemolishButton"] {
                    if let Some(id) = tree.find(root, button) {
                        tree.get_mut(id).enabled = false;
                    }
                }
                tabs.push((caption(key, own), tree, root));
            }
            let (mut tree, root) = view("CloakingController")?;
            if let Some(id) = tree.find(root, "EnableCloakingDeviceButton")
                && let Some(b) = tree.control_mut::<crate::gui::controls::Button>(id)
            {
                b.toggle = true;
            }
            tabs.push((caption("Misc", "Misc"), tree, root));
        }
        Some(Views { tabs })
    }

    /// The views filled from the session (the slots of the interface's signals: FUN_100e91af, FUN_100e92a4,
    /// FUN_100e91c6, FUN_100e9242, FUN_100e918e, FUN_100e70c5).
    fn fill(views: &mut Views, s: &Session) {
        for (_, tree, root) in &mut views.tabs {
            let root = *root;
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

    /// Draws the window; what it asks for.
    pub fn show(&mut self, ctx: &egui::Context, palette: &Palette) -> Option<Command> {
        let Some(session) = self.session.clone() else {
            self.views = None;
            return None;
        };
        // (GUI type 2 only: type 1, the buying of the land, is not here yet)
        if session.gui_type != 2 {
            return None;
        }
        if self.views.is_none() {
            self.views = Self::load(ctx, &session);
            self.shown = None;
        }
        let views = self.views.as_mut()?;
        if self.shown.as_ref() != Some(&session) {
            Self::fill(views, &session);
            self.shown = Some(session.clone());
        }
        let skin = crate::skin::skin(ctx);
        let names: Vec<String> = views.tabs.iter().map(|t| t.0.clone()).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut tab = self.tab.min(views.tabs.len().saturating_sub(1));
        let mut is_open = true;
        let size = egui::vec2(499.0, 499.0);
        let middle = ctx.content_rect().center() - size / 2.0;
        let tabs = &mut views.tabs;
        crate::chrome::window(ctx, palette, "CityTerminal", "", &mut is_open, |w| w.default_size(size).resizable(true).default_pos(crate::layout::pos("CityTerminal", middle)), |ui| {
            crate::chrome::tab_view(ui, palette, &names, &mut tab, |ui, tab| {
                if let Some((_, tree, root)) = tabs.get_mut(tab) {
                    crate::gui::host::show(ui, tree, *root, skin.as_deref(), palette);
                }
            });
        });
        self.tab = tab;
        let mut command = None;
        for (_, tree, _) in tabs.iter_mut() {
            for (id, signal) in std::mem::take(&mut tree.signals) {
                if signal != crate::gui::host::Signal::Clicked {
                    continue;
                }
                match tree.get(id).name.as_str() {
                    "DumpAdvantagesButton" => command = Some(Command::Advantages),
                    "EnableCloakingDeviceButton" => {
                        command = Some(Command::ToggleCloak);
                        // (the button shows the server's word, not the press: it turns when code 14 comes)
                        self.shown = None;
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
        assert!(s.full() && !s.cloak_possible && s.bill.is_none());
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
        assert!(s.apply(&Signal::Lease(2), 100) && !s.full());
        assert_eq!(Session::open(&Signal::Lease(3)), None);
    }

    #[test]
    fn a_window_of_another_kind_is_made_anew() {
        let mut w = CityTerminalWindow::default();
        let s = Session::open(&open()).unwrap();
        w.set(Some(s.clone()));
        w.tab = 3;
        // (the same kind: the tab stays)
        w.set(Some(Session { charge: 0.5, ..s.clone() }));
        assert_eq!(w.tab, 3);
        // (lease mode 1 is the one-tab window)
        w.set(Some(Session { lease: 1, ..s }));
        assert_eq!(w.tab, 0);
        w.set(None);
        assert!(w.session.is_none());
    }

    #[test]
    fn the_long_date() {
        // (noon UTC of 2026-11-15, a Sunday, is the same day in every zone a player's clock is likely in)
        let text = long_date(1_794_744_000);
        assert!(text.ends_with(", 2026") && text.contains("November"), "{text}");
    }
}
