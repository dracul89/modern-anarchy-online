//! The interface drawn over the world: the heads-up display (where you are,
//! the messages, labels over the whompah booths) and the options window
//! (window mode, size, keys). `egui` does the widgets; the look is the AOU
//! mod's translucent theme: dark warm panels with silver and gold accents.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use mao_render::glam::Vec3;
use mao_render::{wgpu, Gpu};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::settings::{Action, Settings, WindowMode, ACTIONS};
use crate::gear::{GearItem, GearWindow};
use crate::hotbar::{self, Hotbars};
use crate::theme::{self, Palette};

/// Text floating over a point of the world (a whompah booth's destination).
pub struct Label {
    pub world: Vec3,
    pub text: String,
}

/// A number rising over a point of the world and fading (damage dealt or taken, healing).
pub struct Floater {
    pub world: Vec3,
    pub text: String,
    /// 0 damage to someone else, 1 damage to you, 2 healing.
    pub kind: u8,
    pub born: Instant,
}

/// A soft coloured burst over a point of the world (a nano program landing on someone).
pub struct Glow {
    pub world: Vec3,
    pub color: [u8; 3],
    pub born: Instant,
}

/// How long a glow lives, in seconds.
pub const GLOW_LIFE: f32 = 1.1;

/// The colour of a nano program's glow: its own effect colour stats when it has them, else one of a few by its school.
pub fn glow_color(red: Option<u32>, green: Option<u32>, blue: Option<u32>, school: u32) -> [u8; 3] {
    if let (Some(r), Some(g), Some(b)) = (red, green, blue) {
        return [r.min(255) as u8, g.min(255) as u8, b.min(255) as u8];
    }
    const BY_SCHOOL: [[u8; 3]; 6] = [[120, 200, 255], [255, 170, 90], [150, 255, 150], [220, 140, 255], [255, 120, 140], [255, 235, 130]];
    BY_SCHOOL[school as usize % BY_SCHOOL.len()]
}

/// How long a floating number lives, in seconds.
pub const FLOATER_LIFE: f32 = 1.6;

/// What the display shows besides the options.
#[derive(Default)]
pub struct Hud {
    /// The playfield's name.
    pub location: String,
    /// Where the character stands (x, height, z), the way it faces, and the camera's yaw (for the map).
    pub position: Option<[f32; 3]>,
    /// The playfield the character is in (for the planet map).
    pub playfield: u32,
    /// The waypoint uploaded from a mission (`missionlog.rs`): its playfield and place, server coordinates.
    pub waypoint: Option<(u32, [f32; 3])>,
    pub heading: f32,
    pub camera_yaw: f32,
    /// The character's name and stats (by stat number), for the Stats window.
    pub character_name: String,
    pub stats: std::collections::BTreeMap<u32, i64>,
    /// What each shown value is made of (for the hover text of the Skills window).
    pub stat_parts: std::collections::BTreeMap<u32, crate::derived::Breakdown>,
    /// Who is about, for the map: (x, z) and what.
    pub map_dots: Vec<([f32; 2], crate::automap::DotKind)>,
    /// Numbers rising over the fight.
    pub floaters: Vec<Floater>,
    pub glows: Vec<Glow>,
    /// The names floating over the characters and creatures near.
    pub nameplates: Vec<crate::nametag::Tag>,
    pub labels: Vec<Label>,
    /// The friends list.
    pub friends: Vec<crate::friends::Friend>,
    /// The character's nano programs.
    pub programs: Vec<crate::nanoprograms::Program>,
    /// The character's perks.
    pub perk_list: Vec<crate::perks::PerkInfo>,
    /// The open container (corpse, bag).
    pub loot: Option<crate::loot::LootView>,
    pub loot_preview: std::sync::Mutex<Option<crate::loot::LootWindow>>,
    pub perks_preview: std::sync::Mutex<Option<crate::perks::PerksWindow>>,
    pub programs_preview: std::sync::Mutex<Option<crate::nanoprograms::ProgramsWindow>>,
    pub friends_preview: std::sync::Mutex<Option<crate::friends::FriendsWindow>>,
    /// What can be clicked in the world: ids and Z-up positions.
    /// What the mouse can pick: (kind, id, feet (Z up), radius, height, picked only when nothing else is (the
    /// character itself)).
    pub pickables: Vec<(u32, u32, [f32; 3], f32, f32, bool)>,
    pub log: VecDeque<(Instant, String)>,
    /// The current target, for the target frame.
    pub target: Option<String>,
    /// The target for the original's frame at the top: name, level, health, distance and what it is.
    pub target_bars: Vec<TargetBar>,
    /// The character's items (the inventory window), and whether a headless preview shows it.
    pub gear: Vec<crate::gear::GearItem>,
    /// The last inspection (counted, so each opens the window): the character's name and worn items.
    pub inspected: (u32, String, Vec<crate::gear::GearItem>),
    /// The running nano programs.
    pub nanos: Vec<crate::ncu::NanoInfo>,
    pub ncu_preview: std::sync::Mutex<Option<crate::ncu::NcuWindow>>,
    pub chat_preview: std::sync::Mutex<Option<crate::chatwindow::ChatWindow>>,
    pub gear_open: bool,
    /// Hotbars shown by a headless preview (a live window keeps its own in the `Ui`).
    pub preview_hotbars: Hotbars,
    /// The travel list the server sent for this playfield.
    pub destinations: Option<mao_net::destinations::DestinationList>,
    /// Whether a headless preview shows the Whom-Pah window.
    pub whompah_open: bool,
    /// A mission window a headless preview shows.
    pub mission_preview: std::sync::Mutex<Option<crate::mission::MissionWindow>>,
    /// A mail window a headless preview shows.
    pub mail_preview: std::sync::Mutex<Option<crate::mailbox::MailWindow>>,
    /// A shop window a headless preview shows.
    pub shop_preview: std::sync::Mutex<Option<crate::shop::ShopWindow>>,
    /// The mission journal and the planet map a headless preview shows.
    pub mission_log_preview: std::sync::Mutex<Option<crate::missionlog::MissionLog>>,
    pub planet_preview: std::sync::Mutex<Option<crate::planetwindow::PlanetWindow>>,
    /// An NPC chat window a headless preview shows.
    pub npc_chat_preview: std::sync::Mutex<Option<crate::npcchat::NpcChatWindow>>,
    /// An Action window a headless preview shows.
    pub actions_preview: std::sync::Mutex<Option<crate::actions::ActionsWindow>>,
    /// A map a headless preview shows.
    pub map_preview: std::sync::Mutex<Option<crate::automap::AutoMap>>,
    /// A Stats window a headless preview shows.
    pub stats_preview: std::sync::Mutex<Option<crate::stats::StatsWindow>>,
    pub skills_preview: std::sync::Mutex<Option<crate::skills::SkillsWindow>>,
    /// What the last frame drew (parts), shown beside the frame rate.
    pub draw_info: String,
}

impl Hud {
    /// Adds a message to the log (the last few stay visible for a while).
    pub fn say(&mut self, text: impl Into<String>) {
        self.log.push_back((Instant::now(), text.into()));
        while self.log.len() > 8 {
            self.log.pop_front();
        }
    }
}

/// What the options window asks the window to do.
/// What the target frame shows.
#[derive(Clone, Debug, PartialEq)]
pub struct TargetBar {
    /// The control center's hostile bar (the one fought) or friendly one (the selection): `CCTargetControl_c`'s +0x154.
    pub hostile: bool,
    /// Its header (`TargetHeader_c`): "Selection", "Nano Target", "Fighting Target" or "Nano & Fighting Target".
    pub title: String,
    /// The name's colour (the one fought 0xff4444, else white) and the health bar's (`N3Msg_Consider`'s for a character).
    pub name_colour: u32,
    pub fill: u32,
    /// Whom it fights (`N3Msg_GetTargetTarget`), with the option `Targetstarget` (FUN_100741d8).
    pub target_of: Option<String>,
    pub name: String,
    pub level: Option<i64>,
    /// Health and its maximum (0 when the maximum is not known).
    pub hp: Option<(i64, i64)>,
    pub distance: f32,
    pub kind: crate::automap::DotKind,
}

#[derive(Default)]
pub struct Requests {
    /// A theme picked in the options window.
    pub theme: Option<String>,
    /// A skin folder typed in the options window.
    pub gui_dir: Option<String>,
    pub mode: Option<WindowMode>,
    pub size: Option<(u32, u32)>,
    pub save: bool,
    pub quit: bool,
    /// A hotbar slot clicked with the mouse.
    pub press: Option<usize>,
    /// A mission window command (request missions / accept one).
    pub mission: Option<crate::mission::Command>,
    /// A mail command (check the inbox, send).
    pub mail: Option<crate::mailbox::MailCommand>,
    /// A shop command (buy, close).
    pub shop: Option<crate::shop::ShopCommand>,
    /// What the NPC chat window sends (an answer, a description asked for, the window closed).
    pub npc_chat: Vec<crate::npcchat::Command>,
    /// "Deposit to Market"'s Send.
    pub market_send: Option<crate::marketsend::Deposit>,
    /// Feedback lines to show (text.mdb names).
    pub feedback: Vec<&'static str>,
    /// What the mission journal sends (a deletion), the waypoint it uploaded and the lines it tells.
    pub mission_log: Vec<crate::missionlog::Command>,
    pub waypoint: Option<(u32, [f32; 3])>,
    pub lines: Vec<String>,
    /// Lines the chat window sends.
    pub chat: Vec<crate::chatwindow::Out>,
    /// Something taken from the open container.
    pub loot_take: Option<crate::loot::Take>,
    /// Items dragged onto a place of the inventory window: `(from, to)`.
    pub item_moves: Vec<(u32, u32)>,
    /// A running nano the player cancelled in the NCU window.
    pub cancel_nano: Option<u32>,
    /// Items chosen to use or to destroy in the inventory window's menu (by place).
    pub item_uses: Vec<u32>,
    pub item_deletes: Vec<u32>,
    /// An action clicked in the Action window.
    pub action: Option<hotbar::Action>,
    /// A control center target button: 0 yourself, 1 the next hostile.
    pub target: Option<u8>,
    /// The AGG/DEF slider let go at this value (stat 51).
    pub aggdef: Option<i32>,
    /// A place to walk to (world x, z), from the Whom-Pah window.
    pub goto: Option<[f32; 2]>,
    /// The hotbars were edited (to be saved).
    pub hotbars_changed: bool,
}

/// One frame of interface, ready to paint.
pub struct Frame {
    /// The pointer egui wants (for the skin's pointers).
    pub cursor: egui::CursorIcon,
    jobs: Vec<egui::ClippedPrimitive>,
    textures: egui::TexturesDelta,
    pixels_per_point: f32,
}

static DEBUG_OVERLAY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the developer overlay is on (the `debug_overlay` setting, or `MAO_DEBUG_HUD` in the environment).
pub fn debug_overlay() -> bool {
    DEBUG_OVERLAY.load(std::sync::atomic::Ordering::Relaxed) || std::env::var_os("MAO_DEBUG_HUD").is_some()
}

pub fn set_debug_overlay(on: bool) {
    DEBUG_OVERLAY.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub struct Ui {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    pub options_open: bool,
    pub gear_open: bool,
    pub whompah_open: bool,
    /// The windows last written to `open.ini`.
    open_seen: Vec<&'static str>,
    pub mission: crate::mission::MissionWindow,
    pub mail: crate::mailbox::MailWindow,
    pub shop: crate::shop::ShopWindow,
    pub npc_chat: crate::npcchat::NpcChatWindow,
    /// The mission journal (`mission_window`) and the planet map.
    pub mission_log: crate::missionlog::MissionLog,
    pub planet: crate::planetwindow::PlanetWindow,
    pub actions: crate::actions::ActionsWindow,
    pub automap: crate::automap::AutoMap,
    pub stats: crate::stats::StatsWindow,
    pub skills: crate::skills::SkillsWindow,
    pub team: crate::team::TeamWindow,
    /// The faction standings (`faction_window`).
    pub faction: crate::factions::FactionWindow,
    pub ncu: crate::ncu::NcuWindow,
    pub friends: crate::friends::FriendsWindow,
    pub programs: crate::nanoprograms::ProgramsWindow,
    pub perks: crate::perks::PerksWindow,
    pub help: crate::help::HelpWindow,
    /// Tips (`tips.rs`).
    pub tips: crate::tips::Tips,
    /// The info view (GUI.dll's `InfoView_c`): item pages and help texts.
    pub info: crate::gui::infoview::InfoView,
    /// The in-game web page's window (the market's).
    pub browser: crate::browser::Browser,
    /// "Deposit to Market".
    pub market_send: crate::marketsend::MarketSendWindow,
    /// The skin's control center (`Views/ControlCenter.xml` and its menus), once the skin has loaded.
    pub control_center: Option<crate::gui::controlcenter::ControlCenter>,
    /// The skin's option panel (`OptionPanel/Root.xml`).
    pub option_panel: crate::gui::options::OptionPanel,
    pub loot: crate::loot::LootWindow,
    pub chat: crate::chatwindow::ChatWindow,
    gear: crate::gear::GearWindow,
    /// The hotbars, and the clock their cooldowns run on.
    pub hotbars: Hotbars,
    clock: Instant,
    /// The action the options window is waiting for a key for.
    pub rebinding: Option<Action>,
    /// The colours in use, and the themes the options window offers.
    pub palette: Palette,
    themes: Vec<String>,
    last_frame: Instant,
    fps: f32,
}

/// An `esc_*` option's value in the game's LoginPrefs.xml (for when the skin's values are not loaded).
fn esc_default(name: &str) -> bool {
    !matches!(name, "esc_inventory" | "esc_npcchat" | "esc_nano" | "esc_never")
}

/// When the timed logout began (`FlowControlModule_t::m_pcCampTimer`): set by the live loop, drawn as the "Logout" bar.
pub static CAMP: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
/// How long the logout bar runs (`CampStartedMessage`'s `CreateTimer(40000, ..., "Logout", 0xffffff)`).
pub const CAMP_MS: u64 = 40_000;

/// The logout timer (`TimerBar_c`, FUN_100519d6: a 128 by 16 `PowerBar_t` in white with its label over it), in the middle
/// of the screen (`CampStartedMessage`: at half the display less half the bar), running out over [`CAMP_MS`].
fn camp_bar(ctx: &egui::Context, screen: egui::Rect, palette: &Palette) {
    let Some(began) = *CAMP.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    let left = 1.0 - (began.elapsed().as_millis() as f32 / CAMP_MS as f32).clamp(0.0, 1.0);
    let size = egui::vec2(128.0, 16.0);
    let at = screen.center() - size * 0.5;
    egui::Area::new(egui::Id::unique("camp-timer")).fixed_pos(at - egui::vec2(0.0, 14.0)).interactable(false).order(egui::Order::Foreground).show(ctx, |ui| {
        let (title, _) = ui.allocate_exact_size(egui::vec2(size.x, 14.0), egui::Sense::hover());
        let text = egui::Color32::WHITE;
        match crate::font::tooltip(ctx) {
            Some(font) => {
                font.paint(ui.painter(), egui::pos2(title.center().x - font.width("Logout") * 0.5, title.top()), "Logout", text);
            }
            None => {
                ui.painter().text(title.center_top(), egui::Align2::CENTER_TOP, "Logout", egui::FontId::proportional(11.0), text);
            }
        }
        let (bar, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter().rect_filled(bar, 2.0, egui::Color32::from_black_alpha(170));
        ui.painter().rect_filled(egui::Rect::from_min_size(bar.min + egui::vec2(2.0, 2.0), egui::vec2((size.x - 4.0) * left, size.y - 4.0)), 1.0, text);
        ui.painter().rect_stroke(bar, 2.0, egui::Stroke::new(1.0, rgb(palette.accent)), egui::StrokeKind::Inside);
    });
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}

fn rgb(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

fn lifted(c: [u8; 3], by: u8) -> egui::Color32 {
    egui::Color32::from_rgb(c[0].saturating_add(by), c[1].saturating_add(by), c[2].saturating_add(by))
}

/// Dresses the interface in a palette (a theme's GUIColors.xml).
fn theme(ctx: &egui::Context, p: &Palette) {
    let mut visuals = egui::Visuals::dark();
    let panel = egui::Color32::from_rgba_unmultiplied(p.panel[0], p.panel[1], p.panel[2], 215);
    visuals.window_fill = panel;
    visuals.panel_fill = panel;
    visuals.window_stroke = egui::Stroke::new(1.0, rgb(p.accent));
    visuals.override_text_color = Some(rgb(p.text));
    visuals.selection.bg_fill = rgb(p.accent.map(|v| (v as f32 * 0.4) as u8));
    visuals.selection.stroke = egui::Stroke::new(1.0, rgb(p.text_selected));
    visuals.hyperlink_color = rgb(p.accent);
    visuals.widgets.inactive.bg_fill = lifted(p.panel, 26);
    visuals.widgets.inactive.weak_bg_fill = lifted(p.panel, 26);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, rgb(p.hover));
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, rgb(p.hover));
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, lifted(p.panel, 60));
    ctx.set_visuals(visuals);
}

/// `camera_rotate_left` as "Camera rotate left".
fn pretty(name: &str) -> String {
    let mut s = name.replace('_', " ");
    if let Some(first) = s.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    s
}

impl Ui {
    pub fn new(window: &Window, gpu: &Gpu, format: wgpu::TextureFormat, settings: &Settings) -> Ui {
        let ctx = egui::Context::default();
        // (a double click as Windows counts one, its default 500 ms: the original takes them from `WM_LBUTTONDBLCLK`; egui's
        // own 300 ms missed ordinary double clicks)
        ctx.options_mut(|o| o.input_options.max_double_click_delay = 0.5);
        let palette = theme::load(&settings.theme, &settings.aou_mod);
        theme(&ctx, &palette);
        crate::skin::configure(&ctx, crate::skin::dirs(&settings.gui_dir, &settings.theme, &settings.aou_mod));
        let state = egui_winit::State::new(ctx.clone(), egui::ViewportId::ROOT, window, Some(window.scale_factor() as f32), window.theme(), None);
        let renderer = egui_wgpu::Renderer::new(&gpu.device, format, egui_wgpu::RendererOptions::default());
        let mut ui = Ui { ctx, state, renderer, open_seen: Vec::new(), options_open: false, gear_open: false, whompah_open: false, mission: Default::default(), mail: Default::default(), shop: Default::default(), npc_chat: Default::default(), mission_log: Default::default(), planet: Default::default(), actions: Default::default(), automap: Default::default(), stats: Default::default(), skills: Default::default(), team: Default::default(), faction: Default::default(), ncu: Default::default(), friends: Default::default(), programs: Default::default(), perks: Default::default(), help: Default::default(), tips: Default::default(), info: Default::default(), browser: Default::default(), market_send: Default::default(), control_center: None, option_panel: Default::default(), loot: Default::default(), chat: Default::default(), gear: Default::default(), hotbars: hotbar::load(), clock: Instant::now(), rebinding: None, palette, themes: theme::available(&settings.aou_mod), last_frame: Instant::now(), fps: 60.0 };
        // (the windows the player left open come back open)
        for name in crate::layout::load_open() {
            ui.set_window(&name, true);
        }
        ui.open_seen = ui.open_names();
        ui
    }

    /// The windows worth reopening next time (the ones the player opens by hand).
    fn open_names(&self) -> Vec<&'static str> {
        [(self.gear_open, "inventory"), (self.stats.open, "stats"), (self.skills.open, "skills"), (self.ncu.open, "ncu"), (self.friends.open, "friends"), (self.programs.open, "programs"), (self.perks.open, "perks"), (self.help.open, "help"), (self.actions.open, "actions"), (self.mission.open, "missions"), (self.automap.overlay, "map")]
            .into_iter()
            .filter_map(|(open, name)| open.then_some(name))
            .collect()
    }

    /// Switches to the theme `name` (a `theme =` setting).
    pub fn set_theme(&mut self, name: &str, aou_mod: &str) {
        self.palette = theme::load(name, aou_mod);
        theme(&self.ctx, &self.palette);
    }

    /// Reads the skin's pictures again from the folders the settings name (after `theme` or `gui_dir` changed).
    pub fn set_skin(&mut self, settings: &Settings) {
        crate::skin::configure(&self.ctx, crate::skin::dirs(&settings.gui_dir, &settings.theme, &settings.aou_mod));
        crate::pointer::reset();
    }

    /// Sets the window's pointer to the skin's picture of the one egui wants.
    pub fn pointer(&self, event_loop: &winit::event_loop::ActiveEventLoop, window: &Window, cursor: egui::CursorIcon) {
        crate::pointer::apply(event_loop, window, &self.ctx, cursor);
    }

    /// Gives the interface a window event; true when it used the event (the
    /// pointer is over one of its windows) and the game should not.
    pub fn event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        // (a key is the interface's only while a text field is typed in: egui takes the keyboard whenever any widget has
        // the focus, and its Tab moves the focus to the first button, after which every game key went to the interface)
        if matches!(event, WindowEvent::KeyboardInput { .. }) && !self.ctx.text_edit_focused() && !crate::gui::host::keyboard_taken() {
            if let Some(id) = self.ctx.memory(|m| m.focused()) {
                self.ctx.memory_mut(|m| m.surrender_focus(id));
            }
            return false;
        }
        self.state.on_window_event(window, event).consumed
    }

    /// Where the web page goes, in the window's physical pixels (x, y, width, height), while its window is open.
    pub fn browser_bounds(&self) -> Option<(i32, i32, u32, u32)> {
        let r = self.browser.area?;
        let k = self.ctx.pixels_per_point();
        Some(((r.min.x * k).round() as i32, (r.min.y * k).round() as i32, (r.width() * k).round() as u32, (r.height() * k).round() as u32))
    }

    /// What is under the pointer (for the mouse log): the interface's layer there and the widget it is dragging.
    pub fn under_pointer(&self) -> String {
        let at = self.ctx.pointer_latest_pos();
        format!("layer {:?}, dragging {:?}", at.and_then(|p| self.ctx.layer_id_at(p)), self.ctx.dragged_id())
    }

    /// What Escape does in the original: closes the windows that are set to close with it (dialogs, containers, shops, the options and
    /// the perk window; the inventory, the nano and the character windows stay). True when a window was closed.
    /// Escape: the original's windows that ask for it (each reads its `esc_*` option when it is made: LoginPrefs.xml's
    /// defaults leave the inventory, NPC chat and nano windows open; mail, the mission journal, the bank and the other
    /// windows without an option are never closed by it), and the client's own windows.
    pub fn close_with_escape(&mut self) -> bool {
        let store = self.control_center.as_ref().map(|cc| &cc.store);
        let esc = |name: &str| match store.and_then(|s| s.get(name)) {
            Some(_) => store.is_some_and(|s| s.get_bool(name)),
            None => esc_default(name),
        };
        // (a window whose pin is down stays, as in the original)
        let mut closed = false;
        let mut shut = |name: &str, option: Option<&str>, flag: &mut bool| {
            if *flag && option.is_none_or(&esc) && !crate::chrome::pinned(name) {
                *flag = false;
                closed = true;
            }
        };
        // (the client's own windows)
        shut("Options", None, &mut self.options_open);
        shut("Whom-Pah", None, &mut self.whompah_open);
        shut("Browser", None, &mut self.browser.open);
        shut("Help", None, &mut self.help.open);
        // (the original's)
        shut("Option panel", Some("esc_optionpanel"), &mut self.option_panel.open);
        // (InventoryView_c FUN_100ccefe: a corpse, a container (an own backpack: the option of chests is for one in the
        // world, which the client does not tell apart), not the bank)
        let loot_option = match self.loot.kind() {
            Some(crate::loot::CORPSE) => Some("esc_corpses"),
            Some(0xc749) => Some("esc_backpacks"),
            _ => Some("esc_never"),
        };
        shut("loot", loot_option, &mut self.loot.open);
        shut("Shop", Some("esc_shops"), &mut self.shop.open);
        shut("Perks", Some("esc_perkwindow"), &mut self.perks.open);
        shut("Missions", Some("esc_missionselection"), &mut self.mission.open);
        shut("PlanetMap", Some("esc_planetmap"), &mut self.planet.open);
        shut("Info", Some("esc_infoview"), &mut self.info.open);
        shut("NPCChat", Some("esc_npcchat"), &mut self.npc_chat.open);
        shut("Nano programs", Some("esc_nano"), &mut self.programs.open);
        shut("Inventory", Some("esc_inventory"), &mut self.gear_open);
        if esc("esc_wear") && !crate::chrome::pinned("Wear") && self.gear.close_wear(self.gear_open) {
            closed = true;
        }
        if esc("esc_tips") && self.tips.close() {
            closed = true;
        }
        if !self.options_open {
            self.rebinding = None;
        }
        closed
    }

    /// Whether the window of that name is shown (the names of [`Ui::set_window`]).
    pub fn window_open(&self, name: &str) -> bool {
        match name {
            "inventory" => self.gear_open,
            "skills" => self.skills.open,
            "perks" => self.perks.open,
            "help" => self.help.open,
            "stats" => self.stats.open,
            "ncu" => self.ncu.open,
            "programs" => self.programs.open,
            "actions" => self.actions.open,
            "missions" | "mission" => self.mission_log.open,
            "missionselection" => self.mission.open,
            "friends" => self.friends.open,
            "factions" | "faction" => self.faction.open,
            "map" | "planetmap" => self.planet.open,
            "automap" | "overlay" => self.automap.overlay,
            "optionpanel" => self.option_panel.open,
            "itemshop" => self.browser.showing(crate::browser::Mode::ItemStore),
            "petition" => self.browser.showing(crate::browser::Mode::Petition),
            "dailyrewards" => self.browser.showing(crate::browser::Mode::DailyRewards),
            _ => false,
        }
    }

    /// Shows or hides a window by name (`/open`, `/close`); an empty name with `open` false hides them all. False for an unknown name.
    pub fn set_window(&mut self, name: &str, open: bool) -> bool {
        let name = name.trim().to_lowercase();
        if name.is_empty() {
            if open {
                return false;
            }
            self.gear_open = false;
            self.whompah_open = false;
            self.stats.open = false;
            self.skills.open = false;
            self.ncu.open = false;
            self.friends.open = false;
            self.programs.open = false;
            self.perks.open = false;
            self.help.open = false;
            self.actions.open = false;
            self.mission.open = false;
            self.mission_log.open = false;
            self.planet.open = false;
            self.mail.open = false;
            self.automap.overlay = false;
            self.loot.open = false;
            return true;
        }
        // (BrowserModule_c's windows: the web pages)
        let web = match name.as_str() {
            "itemshop" | "item shop" => Some(crate::browser::Mode::ItemStore),
            "petition" => Some(crate::browser::Mode::Petition),
            "dailyrewards" | "daily rewards" => Some(crate::browser::Mode::DailyRewards),
            _ => None,
        };
        if let Some(mode) = web {
            if open && !self.browser.showing(mode) {
                self.browser.show_page(crate::browser::request(mode));
            } else if !open && self.browser.showing(mode) {
                self.browser.open = false;
            }
            return true;
        }
        let flag: &mut bool = match name.as_str() {
            "inventory" | "items" | "gear" => &mut self.gear_open,
            "stats" => &mut self.stats.open,
            "skills" => &mut self.skills.open,
            "team" => &mut self.team.open,
            "factions" | "faction" => &mut self.faction.open,
            "ncu" | "nanos" => &mut self.ncu.open,
            "friends" => &mut self.friends.open,
            "programs" | "nano programs" => &mut self.programs.open,
            "perks" => &mut self.perks.open,
            "help" => &mut self.help.open,
            "actions" => &mut self.actions.open,
            "missions" | "mission" => &mut self.mission_log.open,
            "missionselection" | "mission selection" => &mut self.mission.open,
            "mail" => &mut self.mail.open,
            "map" | "planetmap" | "planet map" => &mut self.planet.open,
            "automap" | "overlay" => &mut self.automap.overlay,
            "chat" => &mut self.chat.open,
            "whompah" | "whom-pah" | "travel" => &mut self.whompah_open,
            "loot" | "container" => &mut self.loot.open,
            "optionpanel" | "option panel" => &mut self.option_panel.open,
            _ => return false,
        };
        *flag = open;
        true
    }

    /// The options key (F10, `optionpanel_window`): the skin's option panel when the skin has one, else (and from the
    /// panel's "Client settings") the client's own settings window.
    pub fn toggle_options(&mut self) {
        let dirs: Vec<std::path::PathBuf> = crate::skin::skin(&self.ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
        if crate::gui::options::skin_has_panel(&dirs) {
            self.option_panel.open = !self.option_panel.open;
        } else {
            self.options_open = !self.options_open;
            self.rebinding = None;
        }
    }

    /// Seconds on the hotbars' clock.
    pub fn now(&self) -> f64 {
        self.clock.elapsed().as_secs_f64()
    }

    /// Lays the interface out for one frame.
    /// `project` finds where a world point lands on the screen (0..1 from the top left).
    pub fn run(&mut self, window: &Window, settings: &mut Settings, hud: &Hud, project: &dyn Fn(Vec3) -> Option<[f32; 2]>) -> (Frame, Requests) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().max(1e-4);
        self.last_frame = now;
        self.fps += (1.0 / dt - self.fps) * 0.05;
        let names = self.open_names();
        if names != self.open_seen {
            crate::layout::save_open(&names);
            self.open_seen = names;
        }
        let states: Vec<&str> = ["inventory", "stats", "ncu", "programs", "actions", "missions", "friends", "map"].into_iter().filter(|n| self.window_open(n)).collect();
        let cc_open: Vec<(&str, bool)> = crate::gui::controlcenter::WINDOWS.iter().map(|(_, n)| (*n, if *n == "options" { self.options_open } else { self.window_open(n) })).collect();
        let mut menu_click: Option<&'static str> = None;
        let raw = self.state.take_egui_input(window);
        let mut requests = Requests::default();
        let (options_open, rebinding, fps, palette, themes) = (&mut self.options_open, &mut self.rebinding, self.fps, &self.palette, &self.themes);
        let (gear_open, gear, whompah_open) = (&mut self.gear_open, &mut self.gear, &mut self.whompah_open);
        let (mission, mail, shop, actions, automap, stats_window, skills_window, ncu_window, chat_window) = (&mut self.mission, &mut self.mail, &mut self.shop, &mut self.actions, &mut self.automap, &mut self.stats, &mut self.skills, &mut self.ncu, &mut self.chat);
        let team_window = &mut self.team;
        let faction_window = &mut self.faction;
        let (friends_window, programs_window, perks_window, loot_window, help_window) = (&mut self.friends, &mut self.programs, &mut self.perks, &mut self.loot, &mut self.help);
        let info_view = &mut self.info;
        let tips = &mut self.tips;
        let npc_chat = &mut self.npc_chat;
        let (mission_log, planet) = (&mut self.mission_log, &mut self.planet);
        let browser = &mut self.browser;
        let market_send = &mut self.market_send;
        let control_center = &mut self.control_center;
        let option_panel = &mut self.option_panel;
        let mut cc_windows: Vec<(&'static str, bool)> = Vec::new();
        let (hotbars, clock_now) = (&mut self.hotbars, self.clock.elapsed().as_secs_f64());
        crate::layout::remember(&self.ctx);
        // (the options' slider: `zoom_factor` multiplies the window's own scale)
        if (self.ctx.zoom_factor() - settings.ui_scale).abs() > 1e-3 {
            self.ctx.set_zoom_factor(settings.ui_scale);
        }
        crate::gui::host::begin_frame();
        let output = self.ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx().clone();
            hud_layer(&ctx, hud, project, fps, settings.mode, palette);
            let booths: Vec<[f32; 2]> = hud.labels.iter().map(|l| [l.world.x, l.world.y]).collect();
            automap.show(&ctx, hud.position.map(|p| ([p[0], p[2]], hud.heading)), hud.camera_yaw, &hud.map_dots, &booths, &hud.location);
            if control_center.is_none() && crate::skin::skin(&ctx).is_some() {
                *control_center = crate::gui::controlcenter::ControlCenter::load(&ctx);
            }
            if let Some(cc) = control_center.as_mut() {
                let out = cc.show(&ctx, palette, &hud.stats, &|n| cc_open.iter().any(|(m, on)| *m == n && *on));
                cc_windows = out.windows;
                for command in out.commands {
                    if let Some(line) = chat_window.run(&command) {
                        requests.chat.push(line);
                    }
                }
                option_panel.show(&ctx, palette, &mut cc.store, &hud.stats);
                if std::mem::take(&mut option_panel.client_settings) {
                    *options_open = true;
                }
                // (the wings, bars and buttons where the skin's ControlCenter.xml docks them)
                let clicks = crate::vitals::show_docked(&ctx, hud.camera_yaw, &hud.stats, &hud.stat_parts, palette, &cc.docks);
                if clicks.rollup {
                    cc.toggle_rollup();
                }
                if let Some(t) = clicks.target {
                    menu_click = Some(t);
                }
                if clicks.aggdef.is_some() {
                    requests.aggdef = clicks.aggdef;
                }
            } else if let Some(w) = crate::vitals::show(&ctx, hud.camera_yaw, &hud.stats, &hud.stat_parts, palette, &|n| states.contains(&n), true) {
                menu_click = Some(w);
            }
            hotbar_layer(&ctx, hotbars, gear, &hud.gear, &*actions, clock_now, palette, &mut requests);
            if stats_window.open {
                stats_window.show(&ctx, &hud.character_name, &hud.stats, &hud.stat_parts, palette);
            }
            if let Some(out) = chat_window.show(&ctx, palette) {
                requests.chat.push(out);
            }
            for url in chat_window.links.drain(..) {
                info_view.show_url(&url);
            }
            if let (true, Some(view)) = (loot_window.open, hud.loot.as_ref()) {
                if let Some(take) = loot_window.show(&ctx, view, palette) {
                    requests.loot_take = Some(take);
                }
            }
            if help_window.open {
                help_window.show(&ctx, &settings.keymap, palette);
            }
            tips.watch(&hud.location, *gear_open, skills_window.open, shop.open, hud.stats.get(&60).copied().unwrap_or(0) as i64);
            for url in tips.show(&ctx, palette, dt) {
                info_view.show_url(&url);
            }
            info_view.show(&ctx, palette);
            for command in info_view.take_commands() {
                if let Some(out) = chat_window.run(&command) {
                    requests.chat.push(out);
                }
            }
            if perks_window.open {
                perks_window.show(&ctx, &hud.perk_list, palette);
            }
            if programs_window.open {
                if let Some(action) = programs_window.show(&ctx, &hud.programs, palette) {
                    requests.action = Some(action);
                }
                for url in programs_window.infos.drain(..) {
                    info_view.show_url(&url);
                }
            }
            if friends_window.open {
                let (outs, tell) = friends_window.show(&ctx, &hud.friends, palette);
                requests.chat.extend(outs);
                if let Some(name) = tell {
                    chat_window.type_text(&format!("/tell {name} "));
                }
            }
            if ncu_window.open {
                if let Some(nano) = ncu_window.show(&ctx, &hud.nanos, &hud.stats, palette) {
                    requests.cancel_nano = Some(nano);
                }
                if let Some(url) = ncu_window.info_url.take() {
                    info_view.show_url(&url);
                }
            }
            if skills_window.open {
                skills_window.show(&ctx, &hud.character_name, &hud.stats, &hud.stat_parts, palette);
            }
            team_window.show(&ctx, palette);
            faction_window.show(&ctx, &hud.stats, palette);
            if actions.open {
                actions.show(&ctx, gear, palette);
                if let Some(a) = actions.clicked.take() {
                    requests.action = Some(a);
                }
            }
            if *whompah_open {
                whompah_window(&ctx, whompah_open, hud, palette, &mut requests);
            }
            if mission.open {
                let command = mission.show(&ctx, gear, palette);
                if command.what != 0 {
                    requests.mission = Some(command);
                }
            }
            if shop.open {
                if let Some(command) = shop.show(&ctx, gear, &hud.gear, palette) {
                    requests.shop = Some(command);
                }
                if let Some(url) = shop.info_url.take() {
                    info_view.show_url(&url);
                }
            }
            // (the compass's yellow indicator: the waypoint's bearing when it is in this playfield)
            let waypoint = requests.waypoint.or(hud.waypoint);
            if let Ok(mut b) = crate::vitals::WAYPOINT_BEARING.lock() {
                *b = match (waypoint, hud.position) {
                    (Some((pf, w)), Some(p)) if pf == hud.playfield => Some((w[0] - p[0]).atan2(w[2] - p[2])),
                    _ => None,
                };
            }
            let shown = mission_log.show(&ctx, palette, hud.playfield);
            requests.mission_log.extend(shown.commands);
            if shown.waypoint.is_some() {
                requests.waypoint = shown.waypoint;
            }
            requests.lines.extend(shown.lines);
            if let Some(markup) = shown.info {
                info_view.show_text(&markup);
            }
            planet.show(&ctx, palette, hud.playfield, hud.position, hud.heading, requests.waypoint.or(hud.waypoint));
            if npc_chat.open {
                requests.npc_chat.extend(npc_chat.show(&ctx, palette, &hud.gear, gear));
                if let Some(url) = npc_chat.info_url.take() {
                    info_view.show_url(&url);
                }
            }
            if mail.open || mail.message.open {
                if let Some(command) = mail.show(&ctx, gear, &hud.gear, palette) {
                    requests.mail = Some(command);
                }
            }
            for url in gear.show_inspect(&ctx, &hud.inspected, &hud.stats, palette) {
                info_view.show_url(&url);
            }
            if *gear_open {
                let events = gear.show(&ctx, gear_open, &hud.gear, &hud.stats, palette);
                requests.item_moves.extend(events.moves);
                requests.item_uses.extend(events.uses);
                requests.item_deletes.extend(events.deletes);
                for url in events.infos {
                    info_view.show_url(&url);
                }
                if shop.open
                    && let Some(&slot) = events.trade_adds.last()
                {
                    requests.shop = Some(crate::shop::ShopCommand::Sell(slot));
                }
            }
            browser.show(&ctx, palette);
            if market_send.open {
                let cash = hud.stats.get(&61).copied().unwrap_or(0);
                let (deposit, feedback) = market_send.show(&ctx, gear, &hud.gear, cash, palette);
                requests.market_send = deposit;
                if let Some(name) = feedback {
                    requests.feedback.push(name);
                }
            }
            if *options_open {
                options_window(&ctx, options_open, rebinding, settings, &mut requests, palette, themes);
            }
        });
        if !self.ctx.input(|i| i.pointer.any_down()) {
            crate::gear::DRAGGED.store(u32::MAX, std::sync::atomic::Ordering::Relaxed);
        }
        for (w, on) in cc_windows {
            self.set_window(w, on);
        }
        if let Some(n) = menu_click.and_then(|n| ["target_self", "target_next", "camera_1st", "camera_3rd", "camera_lock", "camera_rubber", "camera_trail"].iter().position(|t| *t == n)) {
            requests.target = Some(n as u8);
        } else if menu_click == Some("options") {
            self.toggle_options();
        } else if let Some(name) = menu_click {
            let now = self.window_open(name);
            self.set_window(name, !now);
        }
        let cursor = output.platform_output.cursor_icon;
        self.state.handle_platform_output(window, output.platform_output);
        let jobs = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        (Frame { cursor, jobs, textures: output.textures_delta, pixels_per_point: output.pixels_per_point }, requests)
    }

    /// Lays out a custom interface (the launcher's form) for one frame: `build` draws into the context.
    pub fn run_custom(&mut self, window: &Window, build: &mut dyn FnMut(&egui::Context)) -> Frame {
        let raw = self.state.take_egui_input(window);
        crate::gui::host::begin_frame();
        let output = self.ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx().clone();
            build(&ctx);
        });
        let cursor = output.platform_output.cursor_icon;
        self.state.handle_platform_output(window, output.platform_output);
        let jobs = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        Frame { cursor, jobs, textures: output.textures_delta, pixels_per_point: output.pixels_per_point }
    }

    /// Paints a frame over what is already in `target`.
    pub fn paint(&mut self, gpu: &Gpu, target: &wgpu::TextureView, size: (u32, u32), frame: Frame) {
        let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [size.0, size.1], pixels_per_point: frame.pixels_per_point };
        for (id, deltas) in &frame.textures.set {
            for delta in deltas {
                self.renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let extra = self.renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &frame.jobs, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("interface"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &frame.jobs, &screen);
        }
        gpu.queue.submit(extra.into_iter().chain(Some(encoder.finish())));
        for id in &frame.textures.free {
            self.renderer.free_texture(id);
        }
        // (epaint checks at drop that the deltas were handled; they were, above)
        let mut frame = frame;
        frame.textures.clear();
    }
}

/// Paints the interface over `target` with no window behind it (a headless
/// preview): the heads-up display, and the options window when asked.
pub fn paint_headless(gpu: &Gpu, target: &wgpu::TextureView, format: wgpu::TextureFormat, size: (u32, u32), settings: &mut Settings, hud: &Hud, project: &dyn Fn(Vec3) -> Option<[f32; 2]>, options: bool) {
    let ctx = egui::Context::default();
    let palette = theme::load(&settings.theme, &settings.aou_mod);
    let themes = theme::available(&settings.aou_mod);
    theme(&ctx, &palette);
    crate::skin::configure(&ctx, crate::skin::dirs(&settings.gui_dir, &settings.theme, &settings.aou_mod));
    crate::font::set_enabled(settings.original_font);
    let mut renderer = egui_wgpu::Renderer::new(&gpu.device, format, egui_wgpu::RendererOptions::default());
    let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [size.0, size.1], pixels_per_point: 1.0 };
    let (mut open, mut rebinding, mut requests) = (options, None, Requests::default());
    let (mut gear_open, mut gear) = (hud.gear_open, crate::gear::GearWindow::default());
    if let Ok(tab) = std::env::var("MAO_GEAR_TAB") {
        gear.select_tab(&tab);
    }
    let mut hotbars = hud.preview_hotbars.clone();
    let mut preview_mission = hud.mission_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_mail = hud.mail_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_shop = hud.shop_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_npc_chat = hud.npc_chat_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_mission_log = hud.mission_log_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_planet = hud.planet_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_actions = hud.actions_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_map = hud.map_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_stats = hud.stats_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_skills = hud.skills_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_ncu = hud.ncu_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_chat = hud.chat_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_friends = hud.friends_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_programs = hud.programs_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_perks = hud.perks_preview.lock().ok().and_then(|mut m| m.take());
    let mut preview_loot = hud.loot_preview.lock().ok().and_then(|mut m| m.take());
    // MAO_UI_PREVIEW=view MAO_VIEW=<file in the skin's Views>: that file's window, made by the GUI.dll view system (`gui/`).
    let mut preview_view: Option<crate::gui::window::XmlWindow> = None;
    let mut preview_cc: Option<crate::gui::controlcenter::ControlCenter> = None;
    // MAO_UI_PREVIEW=optionpanel: the skin's option panel (MAO_OPTION_PAGE=<n> picks a page).
    let mut preview_panel = crate::gui::options::OptionPanel::default();
    preview_panel.open = std::env::var("MAO_UI_PREVIEW").as_deref() == Ok("optionpanel");
    // (MAO_UI_PREVIEW=marketsend: "Deposit to Market", the first two backpack items in it)
    // (MAO_UI_PREVIEW=team: the team window on a sample team)
    let mut preview_team = (std::env::var("MAO_UI_PREVIEW").as_deref() == Ok("team")).then(|| { let mut w = crate::team::TeamWindow::default(); w.open = true; w });
    // (MAO_UI_PREVIEW=tip: the tip window on the tips triggered)
    let mut preview_tips = (std::env::var("MAO_UI_PREVIEW").as_deref() == Ok("tip")).then(crate::tips::Tips::default);
    // (MAO_UI_PREVIEW=factions: the faction window on sample standings)
    let mut preview_factions = (std::env::var("MAO_UI_PREVIEW").as_deref() == Ok("factions")).then(|| { let mut w = crate::factions::FactionWindow::default(); w.open = true; w });
    let faction_sample: std::collections::BTreeMap<u32, i64> = [(566, 1200), (567, 40000), (568, -12000), (569, -45000), (570, 0), (571, 7500), (572, -300)].into_iter().collect();
    // (MAO_UI_PREVIEW=inspect: the inspect window)
    let mut preview_inspect = (std::env::var("MAO_UI_PREVIEW").as_deref() == Ok("inspect")).then(crate::gear::GearWindow::default);
    let mut preview_deposit = crate::marketsend::MarketSendWindow::default();
    if std::env::var("MAO_UI_PREVIEW").as_deref() == Ok("marketsend") {
        preview_deposit.show_window();
        for (place, item) in preview_deposit.slots.iter_mut().zip(hud.gear.iter().filter(|i| i.slot >= 64)) {
            *place = Some(item.slot);
        }
    }
    if let Some(n) = std::env::var("MAO_OPTION_PAGE").ok().and_then(|v| v.parse().ok()) {
        preview_panel.select(n);
    }
    let mut last = None;
    // Several frames: the first lays out and uploads the fonts, and the windows
    // fade in over the next ones; the last is what is drawn.
    // (MAO_CLICK's clicks take five frames each)
    let clicks = std::env::var("MAO_CLICK").map_or(0, |v| v.split(';').count());
    // (MAO_DRAG=x1,y1,x2,y2 drags from the one point to the other after the clicks: pressed, moved over four frames,
    // released)
    let drag: Option<[f32; 4]> = std::env::var("MAO_DRAG").ok().and_then(|v| v.split(',').map(|n| n.trim().parse().ok()).collect::<Option<Vec<f32>>>()).and_then(|v| v.try_into().ok());
    let drag_start = 4 + 5 * clicks;
    for frame in 0..(12usize).max(4 + 5 * clicks + 4).max(if drag.is_some() { drag_start + 10 } else { 0 }) {
        // MAO_HOVER=x,y puts the pointer there (pixels) so a tooltip shows in the picture.
        let mut events: Vec<egui::Event> = std::env::var("MAO_HOVER").ok().and_then(|v| { let (x, y) = v.split_once(',')?; Some(egui::Event::PointerMoved(egui::pos2(x.trim().parse().ok()?, y.trim().parse().ok()?))) }).into_iter().collect();
        // MAO_CLICK=x,y[;x,y...] clicks there (pixels), one click every fifth frame from the fourth (frames between for what
        // it opened to appear and take the mouse), to open menus for a picture.
        if let Ok(v) = std::env::var("MAO_CLICK") {
            let clicks: Vec<egui::Pos2> = v.split(';').filter_map(|c| { let (x, y) = c.split_once(',')?; Some(egui::pos2(x.trim().parse().ok()?, y.trim().parse().ok()?)) }).collect();
            if frame >= 4 && (frame - 4) % 5 < 2 {
                if let Some(&pos) = clicks.get((frame - 4) / 5) {
                    let pressed = (frame - 4) % 5 == 0;
                    events.push(egui::Event::PointerMoved(pos));
                    events.push(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() });
                }
            }
        }
        if let Some([x1, y1, x2, y2]) = drag
            && (drag_start..=drag_start + 5).contains(&frame)
        {
            let k = (frame - drag_start) as f32 / 5.0;
            let pos = egui::pos2(x1 + (x2 - x1) * k, y1 + (y2 - y1) * k);
            events.push(egui::Event::PointerMoved(pos));
            if frame == drag_start || frame == drag_start + 5 {
                events.push(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: frame == drag_start, modifiers: Default::default() });
            }
        }
        // MAO_TYPE=text types it after the clicks (into the field a click gave the keyboard); a `|` presses Left once.
        if let Ok(v) = std::env::var("MAO_TYPE")
            && frame == 4 + 5 * clicks + 1
        {
            for (k, part) in v.split('|').enumerate() {
                if k > 0 {
                    events.push(egui::Event::Key { key: egui::Key::ArrowLeft, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
                }
                if !part.is_empty() {
                    events.push(egui::Event::Text(part.to_string()));
                }
            }
        }
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size.0 as f32, size.1 as f32))), time: Some(frame as f64 * 0.1), events, ..Default::default() };
        if std::env::var_os("MAO_HOVER").is_some() {
            ctx.global_style_mut(|style| { style.interaction.tooltip_delay = 0.0; style.interaction.show_tooltips_only_when_still = false; });
        }
        let output = ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx().clone();
            hud_layer(&ctx, hud, project, 60.0, settings.mode, &palette);
            if preview_cc.is_none() && crate::skin::skin(&ctx).is_some() {
                preview_cc = crate::gui::controlcenter::ControlCenter::load(&ctx);
            }
            match preview_cc.as_mut() {
                Some(cc) => {
                    cc.show(&ctx, &palette, &hud.stats, &|_| false);
                    preview_panel.show(&ctx, &palette, &mut cc.store, &hud.stats);
                    if crate::vitals::show_docked(&ctx, hud.camera_yaw, &hud.stats, &hud.stat_parts, &palette, &cc.docks).rollup {
                        cc.toggle_rollup();
                    }
                }
                None => {
                    crate::vitals::show(&ctx, hud.camera_yaw, &hud.stats, &hud.stat_parts, &palette, &|_| false, true);
                }
            }
            hotbar_layer(&ctx, &mut hotbars, &mut gear, &hud.gear, &crate::actions::ActionsWindow::default(), 1.5, &palette, &mut requests);
            if hud.whompah_open {
                whompah_window(&ctx, &mut true, hud, &palette, &mut requests);
            }
            if let Some(window) = preview_mission.as_mut() {
                window.show(&ctx, &mut gear, &palette);
            }
            if std::env::var("MAO_UI_PREVIEW").as_deref() == Ok("view") {
                if preview_view.is_none() {
                    let file = std::env::var("MAO_VIEW").unwrap_or_else(|_| "TipView.xml".into());
                    preview_view = crate::gui::window::XmlWindow::load(&ctx, "view", &file, &file);
                    // MAO_DROPDOWN_ITEMS=a;b;c: the entries of every drop-down and combo box in the view
                    if let (Some(w), Ok(items)) = (preview_view.as_mut(), std::env::var("MAO_DROPDOWN_ITEMS")) {
                        for node in w.tree.nodes.iter_mut() {
                            if let Some(d) = node.control.as_any_mut().downcast_mut::<crate::gui::dropdown::DropdownMenu>() {
                                for item in items.split(';') {
                                    d.append(item);
                                }
                            }
                            if let Some(c) = node.control.as_any_mut().downcast_mut::<crate::gui::combobox::ComboBox>() {
                                c.items.extend(items.split(';').enumerate().map(|(k, s)| (s.to_string(), k as i32)));
                            }
                        }
                        w.tree.relayout = true;
                    }
                    // MAO_VIEW_TEXT=<markup>: put into the view named BrowserView (the info and tip views' text)
                    if let (Some(w), Ok(text)) = (preview_view.as_mut(), std::env::var("MAO_VIEW_TEXT")) {
                        if let Some(id) = w.tree.find(w.root, "BrowserView") {
                            if let Some(t) = w.tree.control_mut::<crate::gui::controls::TextView>(id) {
                                t.set_text(&text);
                            }
                        }
                    }
                }
                if let Some(window) = preview_view.as_mut() {
                    window.show(&ctx, &palette);
                }
            }
            if let Some(window) = preview_stats.as_mut() {
                window.show(&ctx, &hud.character_name, &hud.stats, &hud.stat_parts, &palette);
            }
            if let Some(team) = preview_team.as_mut() {
                team.show(&ctx, &palette);
            }
            if let Some(window) = preview_factions.as_mut() {
                window.show(&ctx, &faction_sample, &palette);
            }
            if let Some(window) = preview_inspect.as_mut() {
                window.show_inspect(&ctx, &hud.inspected, &hud.stats, &palette);
            }
            if let Some(tips) = preview_tips.as_mut() {
                tips.show(&ctx, &palette, 0.0);
            }
            if let (Some(window), Some(view)) = (preview_loot.as_mut(), hud.loot.as_ref()) {
                window.show(&ctx, view, &palette);
            }
            if let Some(window) = preview_perks.as_mut() {
                window.show(&ctx, &hud.perk_list, &palette);
            }
            if let Some(window) = preview_programs.as_mut() {
                window.show(&ctx, &hud.programs, &palette);
            }
            if let Some(window) = preview_friends.as_mut() {
                window.show(&ctx, &hud.friends, &palette);
            }
            if let Some(window) = preview_chat.as_mut() {
                window.show(&ctx, &palette);
            }
            if let Some(window) = preview_ncu.as_mut() {
                window.show(&ctx, &hud.nanos, &hud.stats, &palette);
            }
            if let Some(window) = preview_skills.as_mut() {
                window.show(&ctx, &hud.character_name, &hud.stats, &hud.stat_parts, &palette);
            }
            if let Some(map) = preview_map.as_mut() {
                let at = hud.position.unwrap_or([0.0; 3]);
                map.show(&ctx, Some(([at[0], at[2]], hud.heading)), hud.camera_yaw, &hud.map_dots, &[], &hud.location);
            }
            if let Some(window) = preview_actions.as_mut() {
                window.show(&ctx, &mut gear, &palette);
            }
            if let Some(window) = preview_mission_log.as_mut() {
                let shown = window.show(&ctx, &palette, hud.playfield);
                for line in shown.lines {
                    eprintln!("mission log: {line}");
                }
                if let Some(w) = shown.waypoint {
                    eprintln!("mission log: waypoint {w:?}");
                }
            }
            if let Some(window) = preview_planet.as_mut() {
                window.show(&ctx, &palette, hud.playfield, hud.position, hud.heading, hud.waypoint);
                // (the tiles come from a thread: give it time between the frames)
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
            if let Some(window) = preview_npc_chat.as_mut() {
                for command in window.show(&ctx, &palette, &hud.gear, &mut gear) {
                    eprintln!("npc chat command: {command:?}");
                }
            }
            if let Some(window) = preview_shop.as_mut()
                && let Some(command) = window.show(&ctx, &mut gear, &hud.gear, &palette)
            {
                eprintln!("shop command: {command:?}");
            }
            if preview_deposit.open {
                let _ = preview_deposit.show(&ctx, &mut gear, &hud.gear, 1000, &palette);
            }
            if let Some(window) = preview_mail.as_mut() {
                window.show(&ctx, &mut gear, &hud.gear, &palette);
            }
            if gear_open {
                let _ = gear.show(&ctx, &mut gear_open, &hud.gear, &hud.stats, &palette);
            }
            if open {
                options_window(&ctx, &mut open, &mut rebinding, settings, &mut requests, &palette, &themes);
            }
        });
        let mut textures = output.textures_delta;
        for (id, deltas) in &textures.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        // (applied: egui's debug build checks that a delta is not dropped unused)
        textures.clear();
        let jobs = ctx.tessellate(output.shapes, 1.0);
        last = Some(jobs);
    }
    let jobs = last.unwrap_or_default();
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let extra = renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &jobs, &screen);
    {
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("interface"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            })
            .forget_lifetime();
        renderer.render(&mut pass, &jobs, &screen);
    }
    gpu.queue.submit(extra.into_iter().chain(Some(encoder.finish())));
}

/// Where you are, the messages and the labels over the world.
/// The Whom-Pah window: the booths of this area (their labels say where each leads, once known), how
/// far they are, and a button that walks to one (the server zones a character that walks into a booth).
fn whompah_window(ctx: &egui::Context, open: &mut bool, hud: &Hud, palette: &Palette, requests: &mut Requests) {
    let mut is_open = *open;
    crate::chrome::window(ctx, palette, "Whom-Pah", "Whom-Pah", &mut is_open, |w| w.resizable(true).default_width(340.0).anchor(egui::Align2::RIGHT_CENTER, egui::vec2(-16.0, 0.0)), |ui| {
        let here = hud.position;
        let mut booths: Vec<(f32, &Label)> = hud.labels.iter().map(|l| (here.map_or(0.0, |p| ((l.world.x - p[0]).powi(2) + (l.world.y - p[2]).powi(2)).sqrt()), l)).collect();
        booths.sort_by(|a, b| a.0.total_cmp(&b.0));
        if booths.is_empty() {
            ui.label("No Whom-Pah booths in this area.");
        }
        for (distance, label) in booths {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&label.text).strong());
                ui.label(egui::RichText::new(format!("{distance:.0} m")).weak());
                if crate::chrome::button(ui, palette, "Go", true).on_hover_text("Walk to the booth (any movement key stops)").clicked() {
                    requests.goto = Some([label.world.x, label.world.y]);
                }
            });
        }
    });
    *open = is_open;
}

/// The hotbar: tabs for the six bars and ten slots. A slot shows the action that would fire (its icon,
/// a cooldown sweep and the time left); a yellow frame means a press is queued. Left click presses,
/// right click edits the slot's list, and items dragged from the inventory window are added to it.
fn hotbar_layer(ctx: &egui::Context, bars: &mut Hotbars, gear: &mut GearWindow, items: &[GearItem], actions: &crate::actions::ActionsWindow, now: f64, palette: &Palette, requests: &mut Requests) {
    let accent = rgb(palette.accent);
    let bar = bars.current;
    // Drag it by its frame or the grip; the button beside the bar numbers turns it between horizontal and vertical.
    let screen = ctx.content_rect();
    let pos = bars.pos.map_or(egui::pos2(screen.center().x, screen.bottom() - 62.0), |p| egui::pos2(p[0], p[1]));
    let pos = egui::pos2(pos.x.clamp(screen.left() + 20.0, screen.right() - 20.0), pos.y.clamp(screen.top() + 60.0, screen.bottom()));
    let vertical = bars.vertical;
    let mut rotate = false;
    // With the skin: the original's slot frames (`MULTILISTVIEW_SLOT_32`, 38 px) and its big arrows to step through the bars.
    let skin = crate::skin::skin(ctx).filter(|s| s.has("GFX_GUI_MULTILISTVIEW_SLOT_32_CLOSED") && s.has("GFX_GUI_CONTROLCENTER_BIGARROW_LEFT_STATE1"));
    let slot_size = if skin.is_some() { 38.0 } else { 46.0 };
    let area = egui::Area::new(egui::Id::unique("hotbar")).pivot(egui::Align2::CENTER_BOTTOM).current_pos(pos).movable(true).show(ctx, |ui| {
        egui::Frame::new().fill(egui::Color32::from_black_alpha(if skin.is_some() { 90 } else { 130 })).corner_radius(4).inner_margin(if skin.is_some() { 3 } else { 6 }).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("::").weak()).on_hover_text("Drag the bar by here or its edge");
                if ui.small_button(if vertical { "=" } else { "||" }).on_hover_text("Turn the bar between horizontal and vertical").clicked() {
                    rotate = true;
                }
                match &skin {
                    Some(skin) => {
                        // the arrows step through the six bars; the number between them is the bar shown
                        for (side, step) in [("LEFT", hotbar::BARS - 1), ("RIGHT", 1)] {
                            let (rect, response) = ui.allocate_exact_size(egui::vec2(10.0, 20.0), egui::Sense::click());
                            let state = if response.is_pointer_button_down_on() { 3 } else if response.hovered() { 2 } else { 1 };
                            skin.draw_in(ui.painter(), &format!("GFX_GUI_CONTROLCENTER_BIGARROW_{side}_STATE{state}"), rect, accent);
                            if response.on_hover_text(if side == "LEFT" { "The bar before (Alt+1 to Alt+6 pick one)" } else { "The next bar" }).clicked() {
                                bars.current = (bars.current + step) % hotbar::BARS;
                            }
                            if side == "LEFT" {
                                ui.label(egui::RichText::new(format!("{}", bars.current + 1)).small().color(accent));
                            }
                        }
                    }
                    None => {
                        for b in 0..hotbar::BARS {
                            if ui.selectable_label(bars.current == b, format!("{}", b + 1)).clicked() {
                                bars.current = b;
                            }
                        }
                    }
                }
                if !vertical && debug_overlay() {
                    ui.label(egui::RichText::new("Alt+1-6 bars, 1-0 slots, drag items here, right-click to edit").small().weak());
                }
            });
            let direction = if vertical { egui::Direction::TopDown } else { egui::Direction::LeftToRight };
            ui.with_layout(egui::Layout::from_main_dir_and_cross_align(direction, egui::Align::Min), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(if skin.is_some() { 1.0 } else { 3.0 }, if skin.is_some() { 1.0 } else { 3.0 });
                for slot in 0..hotbar::SLOTS {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(slot_size, slot_size), egui::Sense::click());
                    let queued = bars.is_queued(bar, slot);
                    let shown = bars.slot(bar, slot).display(now).map(|d| (d.entry.action.clone(), d.entry.cooldown, d.remaining));
                    let count = bars.slot(bar, slot).entries.len();
                    let hovering = response.dnd_hover_payload::<hotbar::Action>().is_some();
                    let painter = ui.painter();
                    match &skin {
                        Some(skin) => {
                            skin.draw_in(painter, if shown.is_some() { "GFX_GUI_MULTILISTVIEW_SLOT_32_CLOSED" } else { "GFX_GUI_MULTILISTVIEW_SLOT_32_OPEN" }, rect, egui::Color32::WHITE);
                        }
                        None => {
                            painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(140));
                        }
                    }
                    let border = if queued { egui::Color32::YELLOW } else if hovering { accent } else if shown.is_some() { lifted(palette.panel, 90) } else { egui::Color32::from_white_alpha(30) };
                    if let Some((action, cooldown, remaining)) = &shown {
                        let icon = match action {
                            hotbar::Action::Item { icon: Some(id), .. } => items.iter().find_map(|i| i.icon.as_ref().filter(|ic| ic.id == *id)).cloned(),
                            hotbar::Action::Special { icon: Some(id), .. } => actions.icon_by_instance(*id).cloned(),
                            _ => None,
                        };
                        match icon {
                            Some(icon) => {
                                let id = gear.texture(ctx, &icon);
                                ui.painter().image(id, rect.shrink(3.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                            None => {
                                let letter = action.name().chars().next().unwrap_or('?');
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, letter, egui::FontId::proportional(20.0), accent);
                            }
                        }
                        if *remaining > 0.0 {
                            // The cooldown sweeps down from the top as it runs out.
                            let fraction = (remaining / cooldown.max(0.1)).clamp(0.0, 1.0);
                            let cover = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + rect.height() * fraction));
                            ui.painter().rect_filled(cover, 3.0, egui::Color32::from_black_alpha(170));
                            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{remaining:.1}"), egui::FontId::proportional(13.0), egui::Color32::WHITE);
                        }
                        if count > 1 {
                            ui.painter().text(rect.right_bottom() - egui::vec2(3.0, 2.0), egui::Align2::RIGHT_BOTTOM, format!("+{}", count - 1), egui::FontId::proportional(10.0), egui::Color32::LIGHT_GRAY);
                        }
                    }
                    if skin.is_none() || queued || hovering {
                        ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(if queued || hovering { 2.0 } else { 1.0 }, border), egui::StrokeKind::Inside);
                    }
                    ui.painter().text(rect.left_top() + egui::vec2(3.0, 2.0), egui::Align2::LEFT_TOP, format!("{}", (slot + 1) % 10), egui::FontId::proportional(10.0), egui::Color32::from_white_alpha(160));
                    if let Some(payload) = response.dnd_release_payload::<hotbar::Action>() {
                        bars.add(bar, slot, (*payload).clone(), 5.0);
                        requests.hotbars_changed = true;
                    }
                    if response.clicked() {
                        requests.press = Some(slot);
                    }
                    let tooltip: Vec<String> = bars.slot(bar, slot).entries.iter().enumerate().map(|(i, e)| format!("{}. {} ({:.0}s){}", i + 1, e.action.name(), e.cooldown, if e.ready_at > now { format!(", {:.1}s left", e.ready_at - now) } else { String::new() })).collect();
                    if !tooltip.is_empty() {
                        response.clone().on_hover_text(tooltip.join("\n"));
                    }
                    response.context_menu(|ui| {
                        ui.label(egui::RichText::new(format!("Bar {} slot {}: the first ready action fires", bar + 1, (slot + 1) % 10)).small());
                        let mut changed = false;
                        let (mut remove, mut up, mut down) = (None, None, None);
                        let entries = &mut bars.slot_mut(bar, slot).entries;
                        let last = entries.len().saturating_sub(1);
                        for (i, entry) in entries.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}. {}", i + 1, entry.action.name()));
                                changed |= ui.add(egui::DragValue::new(&mut entry.cooldown).range(0.0..=3600.0).speed(0.1).suffix(" s")).changed();
                                if i > 0 && ui.small_button("^").clicked() {
                                    up = Some(i);
                                }
                                if i < last && ui.small_button("v").clicked() {
                                    down = Some(i);
                                }
                                if ui.small_button("x").clicked() {
                                    remove = Some(i);
                                }
                            });
                        }
                        if let Some(i) = up {
                            entries.swap(i, i - 1);
                            changed = true;
                        }
                        if let Some(i) = down {
                            entries.swap(i, i + 1);
                            changed = true;
                        }
                        if let Some(i) = remove {
                            entries.remove(i);
                            changed = true;
                        }
                        if ui.button("Add Attack").clicked() {
                            entries.push(hotbar::Entry::new(hotbar::Action::Attack, 2.0));
                            changed = true;
                            ui.close();
                        }
                        if ui.button("Clear slot").clicked() {
                            entries.clear();
                            changed = true;
                            ui.close();
                        }
                        if changed {
                            requests.hotbars_changed = true;
                            if bars.slot(bar, slot).entries.is_empty() {
                                bars.cleared(bar, slot);
                            }
                        }
                    });
                }
            });
        });
    });
    // Where the bar was dropped (or turned) is kept.
    let rect = area.response.rect;
    if area.response.dragged() || area.response.drag_stopped() {
        bars.pos = Some([rect.center().x, rect.bottom()]);
        requests.hotbars_changed = area.response.drag_stopped();
    }
    if rotate {
        bars.vertical = !bars.vertical;
        bars.pos = Some(bars.pos.unwrap_or([rect.center().x, rect.bottom()]));
        requests.hotbars_changed = true;
    }
    // While an item is being dragged, say what to do with it.
    if egui::DragAndDrop::has_payload_of_type::<hotbar::Action>(ctx) {
        if let Some(pos) = ctx.pointer_interact_pos() {
            ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("drag-hint"))).text(pos + egui::vec2(14.0, 14.0), egui::Align2::LEFT_TOP, "Drop on a hotbar slot", egui::FontId::proportional(12.0), egui::Color32::WHITE);
        }
    }
}

fn hud_layer(ctx: &egui::Context, hud: &Hud, project: &dyn Fn(Vec3) -> Option<[f32; 2]>, fps: f32, mode: WindowMode, palette: &Palette) {
    let accent = rgb(palette.accent);
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::unique("labels")));
    for label in &hud.labels {
        let Some([x, y]) = project(label.world) else { continue };
        let at = egui::pos2(screen.left() + x * screen.width(), screen.top() + y * screen.height());
        let galley = painter.layout_no_wrap(label.text.clone(), egui::FontId::proportional(14.0), egui::Color32::WHITE);
        let rect = egui::Rect::from_min_size(egui::pos2(at.x - galley.size().x * 0.5, at.y - galley.size().y), galley.size());
        painter.rect_filled(rect.expand(4.0), 4.0, egui::Color32::from_black_alpha(150));
        painter.rect_stroke(rect.expand(4.0), 4.0, egui::Stroke::new(1.0, accent), egui::StrokeKind::Outside);
        painter.galley(rect.min, galley, egui::Color32::WHITE);
    }
    // A nano program landing: rings that widen and fade around the target.
    for g in &hud.glows {
        let age = g.born.elapsed().as_secs_f32() / GLOW_LIFE;
        if age >= 1.0 {
            continue;
        }
        let Some([x, y]) = project(g.world) else { continue };
        let at = egui::pos2(screen.left() + x * screen.width(), screen.top() + y * screen.height());
        for (k, scale) in [1.0f32, 0.62, 0.3].into_iter().enumerate() {
            let radius = (14.0 + age * 46.0) * scale;
            let alpha = ((1.0 - age) * (60.0 + 40.0 * k as f32)) as u8;
            painter.circle_filled(at, radius, egui::Color32::from_rgba_unmultiplied(g.color[0], g.color[1], g.color[2], alpha / 2));
            painter.circle_stroke(at, radius, egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(g.color[0], g.color[1], g.color[2], alpha)));
        }
    }
    // The numbers of the fight: they rise and fade.
    for f in &hud.floaters {
        let age = f.born.elapsed().as_secs_f32();
        if age > FLOATER_LIFE {
            continue;
        }
        let Some([x, y]) = project(f.world) else { continue };
        let at = egui::pos2(screen.left() + x * screen.width(), screen.top() + y * screen.height() - 10.0 - age * 38.0);
        let alpha = ((1.0 - age / FLOATER_LIFE) * 255.0) as u8;
        let (r, g, b) = match f.kind {
            1 => (255, 90, 80),
            2 => (110, 235, 120),
            _ => (255, 230, 120),
        };
        let galley = painter.layout_no_wrap(f.text.clone(), egui::FontId::proportional(18.0), egui::Color32::from_rgba_unmultiplied(r, g, b, alpha));
        let pos = egui::pos2(at.x - galley.size().x * 0.5, at.y - galley.size().y);
        painter.galley_with_override_text_color(pos + egui::vec2(1.0, 1.0), galley.clone(), egui::Color32::from_black_alpha(alpha));
        painter.galley(pos, galley, egui::Color32::from_rgba_unmultiplied(r, g, b, alpha));
    }
    // The tags over the characters (`Indicator_t`): the selected one's, the fought one's, and with `ShowAllNames` everyone's.
    crate::nametag::draw(ctx, &painter, screen, &hud.nameplates, project, palette);
    camp_bar(ctx, screen, palette);
    // (the loading screen over everything while the world comes in)
    crate::loading::draw(ctx, screen);
    let frame = || egui::Frame::new().fill(egui::Color32::from_black_alpha(130)).corner_radius(4).inner_margin(6);
    // (the location, the coordinates and the frame rate are for developers: the original shows none of them)
    if debug_overlay() {
    egui::Area::new(egui::Id::unique("info")).anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 8.0)).interactable(false).show(ctx, |ui| {
        frame().show(ui, |ui| {
            if !hud.location.is_empty() {
                ui.label(egui::RichText::new(&hud.location).color(accent).strong());
            }
            if let Some([x, y, z]) = hud.position {
                ui.label(format!("{x:.0}, {z:.0}  (height {y:.1})"));
            }
        });
    });
    egui::Area::new(egui::Id::unique("fps")).anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 8.0)).interactable(false).show(ctx, |ui| {
        frame().show(ui, |ui| {
            ui.label(format!("{fps:.0} fps  {}  (F10: options)", mode.name()));
            if !hud.draw_info.is_empty() {
                ui.label(egui::RichText::new(&hud.draw_info).small());
            }
        });
    });
    }
    // (ControlCenterModule_c::CreateTargetMenus: the friendly bar centred at a quarter of the width less 192, the hostile at
    // three quarters, 5 down)
    let skinned_target = match crate::skin::skin(ctx) {
        Some(skin) if skin.has("GFX_GUI_TARGET_HB_LEFT") && !hud.target_bars.is_empty() => {
            for bar in &hud.target_bars {
                let half = (screen.width() - 192.0) * 0.5;
                let x = screen.left() + if bar.hostile { half * 1.5 } else { half * 0.5 };
                target_frame(ctx, &skin, bar, egui::pos2(x, screen.top() + 5.0), palette);
            }
            true
        }
        _ => false,
    };
    if let (Some(target), false) = (&hud.target, skinned_target) {
        egui::Area::new(egui::Id::unique("target")).anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 36.0)).interactable(false).show(ctx, |ui| {
            frame().show(ui, |ui| {
                ui.label(egui::RichText::new("Target").small().color(accent));
                ui.label(egui::RichText::new(target).strong());
            });
        });
    }
    let recent: Vec<&String> = hud.log.iter().filter(|(t, _)| t.elapsed() < Duration::from_secs(14)).map(|(_, m)| m).collect();
    if !recent.is_empty() {
        egui::Area::new(egui::Id::unique("log")).anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(225.0, -150.0)).interactable(false).show(ctx, |ui| {
            frame().show(ui, |ui| {
                let font = crate::font::chat(ctx);
                for line in recent {
                    match &font {
                        Some(font) => {
                            font.label(ui, line, egui::Color32::WHITE);
                        }
                        None => {
                            ui.label(line);
                        }
                    }
                }
            });
        });
    }
}

/// The original's target frame under the compass: the name (with the level when known) between the frame's corners
/// (`GFX_GUI_CC_TARGET_FRAME_*`), and the health bar (`GFX_GUI_TARGET_HB_*`: end caps, groove and the sliding fill, tinted red for
/// an enemy, green for a friend, blue for a player), with the distance at its right.
fn target_frame(ctx: &egui::Context, skin: &crate::skin::Skin, bar: &TargetBar, top_centre: egui::Pos2, palette: &Palette) {
    let accent = rgb(palette.accent);
    let colour = |c: u32| egui::Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
    let (fill, text_color) = (colour(bar.fill), colour(bar.name_colour));
    let width = 200.0f32;
    // (each a window of its own the player can move, `CCFriendlyHealthBarConfig` / `CCHostileHealthBarConfig`, starting where
    // the original puts it)
    egui::Area::new(egui::Id::unique(("target", bar.hostile))).default_pos(top_centre - egui::vec2(width * 0.5, 0.0)).movable(true).show(ctx, |ui| {
        // (the header's title over the frame)
        let (title, _) = ui.allocate_exact_size(egui::vec2(width, 13.0), egui::Sense::hover());
        match crate::font::tooltip(ctx) {
            Some(font) => {
                font.paint(ui.painter(), egui::pos2(title.center().x - font.width(&bar.title) * 0.5, title.top()), &bar.title, accent);
            }
            None => {
                ui.painter().text(title.center_top(), egui::Align2::CENTER_TOP, &bar.title, egui::FontId::proportional(10.0), accent);
            }
        }
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 40.0), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(120));
        for (corner, at) in [("TL", rect.left_top()), ("TR", rect.right_top() - egui::vec2(12.0, 0.0)), ("BL", rect.left_bottom() - egui::vec2(0.0, 11.0)), ("BR", rect.right_bottom() - egui::vec2(12.0, 11.0))] {
            skin.draw(painter, &format!("GFX_GUI_CC_TARGET_FRAME_{corner}"), at, 1.0, accent);
        }
        let name = match bar.level {
            Some(level) => format!("{} ({level})", bar.name),
            None => bar.name.clone(),
        };
        match crate::font::shell(ctx) {
            Some(font) => {
                font.paint(painter, egui::pos2(rect.center().x - font.width(&name) * 0.5, rect.top() + 3.0), &name, text_color);
            }
            None => {
                painter.text(egui::pos2(rect.center().x, rect.top() + 4.0), egui::Align2::CENTER_TOP, name, egui::FontId::proportional(14.0), text_color);
            }
        }
        // the bar: caps at both ends, the groove between, the fill over the fraction of health left
        let (left, right, top) = (rect.left() + 14.0, rect.right() - 14.0, rect.bottom() - 15.0);
        skin.draw(painter, "GFX_GUI_TARGET_HB_LEFT", egui::pos2(left, top), 1.0, accent);
        skin.draw(painter, "GFX_GUI_TARGET_HB_RIGHT", egui::pos2(right - 9.0, top), 1.0, accent);
        let groove = egui::Rect::from_min_max(egui::pos2(left + 9.0, top), egui::pos2(right - 9.0, top + 11.0));
        skin.draw_in(painter, "GFX_GUI_TARGET_HB_BACKGROUND", groove, accent);
        let fraction = match bar.hp {
            Some((hp, max)) if max > 0 => (hp as f32 / max as f32).clamp(0.0, 1.0),
            Some(_) => 1.0,
            None => 0.0,
        };
        if fraction > 0.0 {
            let filled = egui::Rect::from_min_max(groove.min, egui::pos2(groove.left() + groove.width() * fraction, groove.bottom()));
            if !skin.draw_in(&painter.with_clip_rect(filled), "GFX_GUI_TARGET_HB_SLIDER", groove, fill) {
                painter.rect_filled(filled, 0.0, fill);
            }
        }
        // (the target's target under the bar)
        if let Some(of) = &bar.target_of {
            let line = egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(0.0, 1.0), egui::vec2(width, 12.0));
            match crate::font::tooltip(ctx) {
                Some(font) => {
                    font.paint(painter, egui::pos2(line.center().x - font.width(of) * 0.5, line.top()), of, egui::Color32::WHITE);
                }
                None => {
                    painter.text(line.center_top(), egui::Align2::CENTER_TOP, of, egui::FontId::proportional(10.0), egui::Color32::WHITE);
                }
            }
        }
        let caption = match bar.hp {
            Some((hp, max)) if max > 0 => format!("{hp} / {max}   {:.0} m", bar.distance),
            Some((hp, _)) => format!("{hp}   {:.0} m", bar.distance),
            None => format!("{:.0} m", bar.distance),
        };
        match crate::font::chat(ctx) {
            Some(font) => {
                font.paint(painter, egui::pos2(rect.center().x - font.width(&caption) * 0.5, groove.top() - 1.0), &caption, egui::Color32::WHITE);
            }
            None => {
                painter.text(egui::pos2(rect.center().x, groove.center().y), egui::Align2::CENTER_CENTER, caption, egui::FontId::proportional(10.0), egui::Color32::WHITE);
            }
        }
    });
}

/// The options window: how the window is shown, its size, and the keys.
fn options_window(ctx: &egui::Context, open: &mut bool, rebinding: &mut Option<Action>, settings: &mut Settings, requests: &mut Requests, palette: &Palette, themes: &[String]) {
    let mut is_open = *open;
    let mut close = false;
    crate::chrome::window(ctx, palette, "Options", "Options", &mut is_open, |w| w.resizable(true).default_width(520.0).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)), |ui| {
        ui.heading("Window");
        ui.horizontal(|ui| {
            for mode in [WindowMode::Windowed, WindowMode::Borderless, WindowMode::Fullscreen] {
                if ui.selectable_label(settings.mode == mode, mode.name()).clicked() && settings.mode != mode {
                    requests.mode = Some(mode);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Window size");
            ui.add(egui::DragValue::new(&mut settings.width).range(640..=7680).speed(8));
            ui.label("x");
            ui.add(egui::DragValue::new(&mut settings.height).range(480..=4320).speed(8));
            if crate::chrome::button(ui, palette, "Apply", true).clicked() {
                requests.size = Some((settings.width, settings.height));
            }
        });
        ui.horizontal(|ui| {
            ui.label("Render scale");
            ui.add(egui::Slider::new(&mut settings.render_scale, 0.4..=1.0).step_by(0.01).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
            ui.label(egui::RichText::new("(below 100% the scene is upscaled with AMD FSR 1)").weak());
        });
        ui.horizontal(|ui| {
            ui.label("Sharpness");
            ui.add(egui::Slider::new(&mut settings.sharpness, 0.0..=2.0).text("stops down").step_by(0.05));
        });
        ui.separator();
        ui.heading("Interface");
        ui.horizontal(|ui| {
            ui.label("View distance");
            ui.add(egui::Slider::new(&mut settings.view_distance, 200.0..=4000.0).logarithmic(true).step_by(10.0));
        });
        ui.horizontal(|ui| {
            ui.label("Character distance");
            ui.add(egui::Slider::new(&mut settings.char_distance, 0.0..=400.0).step_by(5.0).custom_formatter(|v, _| if v < 1.0 { "all".to_string() } else { format!("{v:.0}") }));
        });
        crate::chrome::checkbox(ui, palette, &mut settings.auto_loot, "Take everything from a corpse as soon as it opens");
        if crate::chrome::checkbox(ui, palette, &mut settings.original_font, "The original's chat font (its own 9 px bitmap font)").changed() {
            crate::font::set_enabled(settings.original_font);
        }
        ui.horizontal(|ui| {
            ui.label("Interface size");
            ui.add(egui::Slider::new(&mut settings.ui_scale, 0.6..=2.0).step_by(0.05).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
        });
        ui.horizontal(|ui| {
            ui.label("Theme");
            egui::ComboBox::from_id_salt("theme").selected_text(&settings.theme).show_ui(ui, |ui| {
                for name in themes {
                    if ui.selectable_label(&settings.theme == name, name).clicked() && &settings.theme != name {
                        requests.theme = Some(name.clone());
                    }
                }
            });
            ui.label(egui::RichText::new("(the AOU mod's themes need its folder in the settings)").weak());
        });
        ui.horizontal(|ui| {
            ui.label("Skin folder");
            let edit = crate::chrome::text_edit(ui, palette, egui::TextEdit::singleline(&mut settings.gui_dir).desired_width(320.0).hint_text("a Gui/<skin> folder with Graphics.uvga; empty: the game's own"));
            if edit.lost_focus() {
                requests.gui_dir = Some(settings.gui_dir.clone());
            }
        }).response.on_hover_text("The original's skins (the Gui folder under Funcom/Anarchy Online in your local app data, or the AOU mod's) are read as they are: what the folder lacks comes from the game's own pictures.");
        ui.separator();
        ui.heading("Keys");
        ui.label("Click a key to change it, then press the new one (Esc cancels).");
        crate::chrome::scroll(ui, palette, egui::ScrollArea::vertical().max_height(300.0), |ui| {
            egui::Grid::new("keys").striped(true).num_columns(3).show(ui, |ui| {
                for &(action, name, _) in ACTIONS {
                    ui.label(pretty(name));
                    let keys = settings.keymap.keys(action);
                    let text = if *rebinding == Some(action) {
                        "press a key...".to_string()
                    } else if keys.is_empty() {
                        "(none)".to_string()
                    } else {
                        keys
                    };
                    if ui.add_sized([180.0, 20.0], egui::Button::new(text)).clicked() {
                        *rebinding = Some(action);
                    }
                    if ui.small_button("x").on_hover_text("Unbind").clicked() {
                        settings.keymap.set(action, "");
                        requests.save = true;
                    }
                    ui.end_row();
                }
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            if crate::chrome::button(ui, palette, "Default keys", true).clicked() {
                settings.keymap = crate::settings::Keymap::default();
                requests.save = true;
            }
            if crate::chrome::button(ui, palette, "Save settings", true).clicked() {
                requests.save = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::chrome::button(ui, palette, "Quit game", true).clicked() {
                    requests.quit = true;
                }
                if crate::chrome::button(ui, palette, "Resume", true).clicked() {
                    close = true;
                }
            });
        });
    });
    *open = is_open && !close;
}

#[cfg(test)]
mod glow_tests {
    use super::*;

    #[test]
    fn a_glow_takes_its_own_colour_or_one_by_school() {
        assert_eq!(glow_color(Some(10), Some(300), Some(5), 0), [10, 255, 5]);
        assert_eq!(glow_color(None, Some(1), None, 7), glow_color(None, None, None, 1));
    }
}
