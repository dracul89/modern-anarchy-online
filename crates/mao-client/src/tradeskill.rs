//! The tradeskill window, GUI.dll's `TradeSkillView_c` (FUN_100df2a7, `tradeskill_window`, "Tradeskill Kit", 300 by 400,
//! the control center's Commands menu "#TradeskKit"): the skin's `Views/TradeskillPanel.xml` with its source, target and
//! result slots, Clear and Build, the quality slider and the four feedback texts.
//!
//! The window asks the server, not Gamecode's `N3Msg_TradeskillCombine` (that is the inventory's item-on-item, `0x51`):
//! a backpack item let go on the source slot (the mouse-up handler FUN_100dec39, FUN_100de715) clears the result, the
//! source's and the build's texts, shows the item and sends `N3Msg_InsertSourceAnalyzerItem` (`CharacterAction` 0xdc with
//! the item); on the target slot the same with the target's text and `N3Msg_InsertTargetAnalyzerItem` (0xdd, FUN_100de7a1);
//! a click on either without an item takes it out (the same with nobody). A click on the result slot with a result shows
//! its page (`itemref://low/high/ql`). Clear (FUN_100de639) empties the slots and the texts and takes out what was in;
//! Build (FUN_100de82d) sends `N3Msg_BuildAnalyzerItem` with the quality chosen (0xde) and clears without taking out.
//! Build is enabled with both items in, Clear with either (FUN_100de46d).
//!
//! The server's answers (see [`mao_net::n3::tradeskill_reply`]) fill the texts (text.mdb 10000): the source's and the
//! target's "X_Lvl_Y_used_in_Z" (the item's name, its quality, the number of processes: FUN_100de868, FUN_100de973),
//! the build's "MaybyUsedAsSource" (not valid, FUN_100de4dd), "X_MustBeLvl_Y_ToCombineWithZZ" (out of range,
//! FUN_100de52d) or "YouNeedAtLeastSkill" (a requirement: the value, the skill's name twice, the character's own skill,
//! FUN_100dea7e), and the result (FUN_100df277: its lowest and highest quality and templates) "ResultWillBeXY" with its
//! icon in the result slot (FUN_100df105). The slider is a share 0 to 100 (100 at first) of the result's range: the
//! quality is `(max - min + 1) * share / 100 + min`, held to `max` (FUN_100def2d, which also labels "QL n" the chosen, the
//! lowest and the highest quality; 1 and 300 before a result).
//!
//! Left out: the build feedback's text signal (GlobalSignals +0x1d8, FUN_100de45e) and +0x1d4, whose senders were not
//! traced; the result's name is its low template's (the original names the quality's interpolated item); Escape
//! (`esc_tradeskill`).

use std::sync::{Arc, Mutex};

use crate::gear::{GearItem, GearWindow, Icon};
use crate::theme::Palette;

/// What the server said, for the window.
#[derive(Clone)]
pub enum Event {
    /// `TradeskillSource` / `TradeskillTarget`: in how many processes the item is used.
    Source(i32),
    Target(i32),
    NotValid,
    /// The level the source must have.
    OutOfRange(i32),
    /// A skill and the value it must have.
    Requirement(u32, i32),
    /// The result's lowest and highest quality, its templates, and its name and icon (its low template's).
    Result { min: i32, max: i32, low: u32, high: u32, name: String, icon: Option<Arc<Icon>> },
}

/// What the window asks the connection to send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// The source or target: a backpack place, or none (taken out).
    Source(Option<u32>),
    Target(Option<u32>),
    /// Build at this quality.
    Build(i32),
}

static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());
static COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());

pub fn push_event(e: Event) {
    EVENTS.lock().unwrap_or_else(|e| e.into_inner()).push(e);
}

pub fn take_commands() -> Vec<Command> {
    std::mem::take(&mut *COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

fn send(c: Command) {
    COMMANDS.lock().unwrap_or_else(|e| e.into_inner()).push(c);
}

/// A text of text.mdb's category 10000 by its name (`LDBface::GetText(0x2710, name)`).
fn text(name: &str) -> String {
    crate::gui::xml::ldb_text(&format!("#10000:{}", mao_formats::ldb::elf_hash(name) as i32))
}

/// FUN_100def2d's quality: the share of the result's range, held to its highest.
pub fn quality(min: i32, max: i32, share: i32) -> i32 {
    ((max - min + 1) * share / 100 + min).min(max)
}

/// The result: lowest and highest quality, templates, name and icon.
#[derive(Clone)]
struct Result {
    low: u32,
    high: u32,
    name: String,
    icon: Option<Arc<Icon>>,
}

pub struct TradeskillWindow {
    pub open: bool,
    /// The source's and target's backpack places (`+0x1a0`, `+0x1a8`).
    source: Option<u32>,
    target: Option<u32>,
    /// The result's range (`+0x1b8`, `+0x1bc`: 1 and 300 at first) and the result (`+0x1b0`).
    min: i32,
    max: i32,
    result: Option<Result>,
    /// The slider's share (`+0x1c4`, 100 at first).
    share: i32,
    /// The texts: source, target, result, build.
    texts: [String; 4],
    views: Option<(crate::gui::view::Tree, crate::gui::view::Id)>,
}

impl Default for TradeskillWindow {
    fn default() -> TradeskillWindow {
        TradeskillWindow { open: false, source: None, target: None, min: 1, max: 300, result: None, share: 100, texts: Default::default(), views: None }
    }
}

const SOURCE_TEXT: usize = 0;
const TARGET_TEXT: usize = 1;
const RESULT_TEXT: usize = 2;
const BUILD_TEXT: usize = 3;

impl TradeskillWindow {
    /// The chosen quality (`+0x1c0`): none without a result.
    fn chosen(&self) -> i32 {
        if self.result.is_some() { quality(self.min, self.max, self.share) } else { 0 }
    }

    /// FUN_100de715 / FUN_100de7a1: an item into the source or the target.
    fn put(&mut self, target: bool, place: Option<u32>) {
        self.result = None;
        self.texts[if target { TARGET_TEXT } else { SOURCE_TEXT }].clear();
        self.texts[RESULT_TEXT].clear();
        self.texts[BUILD_TEXT].clear();
        if target {
            self.target = place;
            send(Command::Target(place));
        } else {
            self.source = place;
            send(Command::Source(place));
        }
    }

    /// FUN_100de639: Clear; what was in is taken out when `tell`.
    fn clear(&mut self, tell: bool) {
        self.result = None;
        self.texts = Default::default();
        if self.source.take().is_some() && tell {
            send(Command::Source(None));
        }
        if self.target.take().is_some() && tell {
            send(Command::Target(None));
        }
    }

    /// The server's answers.
    fn apply(&mut self, e: Event, items: &[GearItem], stats: &std::collections::BTreeMap<u32, i64>) {
        use mao_formats::ldb::{Arg, format};
        let item = |place: Option<u32>| place.and_then(|p| items.iter().find(|i| i.slot == p));
        let (src, dst) = (item(self.source), item(self.target));
        let name = |i: Option<&GearItem>| i.map(|i| i.name.clone()).unwrap_or_default();
        let ql = |i: Option<&GearItem>| i.map_or(0, |i| i.quality as i64);
        match e {
            Event::Source(count) => self.texts[SOURCE_TEXT] = format(&text("X_Lvl_Y_used_in_Z"), &[Arg::Text(&name(src)), Arg::Int(ql(src)), Arg::Int(count as i64)]),
            Event::Target(count) => self.texts[TARGET_TEXT] = format(&text("X_Lvl_Y_used_in_Z"), &[Arg::Text(&name(dst)), Arg::Int(ql(dst)), Arg::Int(count as i64)]),
            Event::NotValid => self.texts[BUILD_TEXT] = text("MaybyUsedAsSource"),
            Event::OutOfRange(level) => {
                self.texts[BUILD_TEXT] = format(&text("X_MustBeLvl_Y_ToCombineWithZZ"), &[Arg::Text(&name(src)), Arg::Int(level as i64), Arg::Text(&name(dst)), Arg::Int(ql(dst))]);
            }
            Event::Requirement(stat, value) => {
                let skill = crate::gui::stat_names::STAT_NAMES.iter().find(|s| s.0 == stat).map_or_else(|| stat.to_string(), |s| s.1.to_string());
                let own = stats.get(&stat).copied().unwrap_or(0);
                self.texts[BUILD_TEXT] = format(&text("YouNeedAtLeastSkill"), &[Arg::Int(value as i64), Arg::Text(&skill), Arg::Text(&skill), Arg::Int(own)]);
            }
            Event::Result { min, max, low, high, name, icon } => {
                (self.min, self.max) = (min, max);
                self.result = Some(Result { low, high, name, icon });
            }
        }
    }

    /// Draws the window; an item page to show (the result slot clicked), if any.
    pub fn show(&mut self, ctx: &egui::Context, gear: &mut GearWindow, items: &[GearItem], stats: &std::collections::BTreeMap<u32, i64>, palette: &Palette) -> Option<String> {
        let events = std::mem::take(&mut *EVENTS.lock().unwrap_or_else(|e| e.into_inner()));
        for e in events {
            self.apply(e, items, stats);
        }
        if !self.open {
            return None;
        }
        if self.views.is_none() {
            let dirs: Vec<std::path::PathBuf> = crate::skin::skin(ctx).map(|s| s.dirs().into_iter().map(|d| d.to_path_buf()).collect()).unwrap_or_default();
            if let Some((mut tree, root)) = crate::gui::load_view(&dirs, "TradeskillPanel.xml", "") {
                tree.measure = crate::gui::font::Measure { ctx: Some(ctx.clone()) };
                if let Some(id) = tree.find(root, "TradeskillSlider")
                    && let Some(s) = tree.control_mut::<crate::gui::controls::Slider>(id)
                {
                    s.set(self.share as f64);
                }
                self.views = Some((tree, root));
            }
        }
        let Some((tree, root)) = self.views.as_mut() else { return None };
        let root = *root;
        // (the slider's share, the chosen quality and the labels, FUN_100df221 / FUN_100def2d)
        if let Some(id) = tree.find(root, "TradeskillSlider")
            && let Some(s) = tree.control::<crate::gui::controls::Slider>(id)
        {
            self.share = s.value as i32;
        }
        let chosen = quality(self.min, self.max, self.share);
        let labels = [("TradeskillLbl", chosen), ("TradeskillLblMax", self.max), ("TradeskillLblMin", self.min)];
        for (name, v) in labels {
            set_text(tree, root, name, &format!("QL {v}"));
        }
        // (the result's line, FUN_100df105)
        self.texts[RESULT_TEXT] = match &self.result {
            Some(r) => mao_formats::ldb::format(&text("ResultWillBeXY"), &[mao_formats::ldb::Arg::Int(chosen as i64), mao_formats::ldb::Arg::Text(&r.name)]),
            None => self.texts[RESULT_TEXT].clone(),
        };
        for (name, t) in ["SrcFeedback", "DstFeedback", "ResultFeedback", "BuildFeedback"].iter().zip(&self.texts) {
            set_text(tree, root, name, t);
        }
        // (the items in the slots)
        let slot_item = |gear: &mut GearWindow, place: Option<u32>| {
            place.and_then(|p| items.iter().find(|i| i.slot == p)).and_then(|it| it.icon.as_ref().map(|icon| crate::gui::itemslot::SlotItem { texture: gear.texture(ctx, icon), name: it.name.clone() }))
        };
        let shown = [("SrcIconSlot", slot_item(gear, self.source)), ("DstIconSlot", slot_item(gear, self.target))];
        for (name, item) in shown {
            if let Some(id) = tree.find(root, name) {
                crate::gui::itemslot::set_item(tree, id, item);
            }
        }
        let result = self.result.as_ref().and_then(|r| r.icon.as_ref().map(|icon| crate::gui::itemslot::SlotItem { texture: gear.texture(ctx, icon), name: r.name.clone() }));
        if let Some(id) = tree.find(root, "ResultIconSlot") {
            crate::gui::itemslot::set_item(tree, id, result);
        }
        // (FUN_100de46d: Build with both, Clear with either)
        for (name, on) in [("BuildButton", self.source.is_some() && self.target.is_some()), ("ClearButton", self.source.is_some() || self.target.is_some())] {
            if let Some(id) = tree.find(root, name) {
                tree.get_mut(id).enabled = on;
            }
        }
        let skin = crate::skin::skin(ctx);
        let dragged = crate::gear::DRAGGED.load(std::sync::atomic::Ordering::Relaxed);
        let mut open = self.open;
        let mut dropped: Option<(bool, u32)> = None;
        crate::chrome::window(ctx, palette, "Tradeskill", "Tradeskill Kit", &mut open, |w| w.default_pos(crate::layout::pos("Tradeskill", egui::pos2(260.0, 120.0))).default_size(egui::vec2(300.0, 400.0)), |ui| {
            let response = crate::gui::host::show(ui, tree, root, skin.as_deref(), palette);
            // (a backpack item let go over the source or the target slot, FUN_100dec39)
            if dragged == u32::MAX || dragged < mao_net::inventory::BACKPACK_FIRST || !ui.input(|i| i.pointer.any_released()) {
                return;
            }
            let Some(at) = ui.input(|i| i.pointer.interact_pos()) else { return };
            for (target, name) in [(false, "SrcIconSlot"), (true, "DstIconSlot")] {
                if let Some(id) = tree.find(root, name)
                    && crate::gui::host::screen_rect(tree, id, response.rect.min).contains(at)
                {
                    dropped = Some((target, dragged));
                }
            }
        });
        if let Some((target, place)) = dropped {
            self.put(target, Some(place));
        }
        let mut page = None;
        let signals = std::mem::take(&mut self.views.as_mut().map(|v| &mut v.0).unwrap().signals);
        for (id, signal) in signals {
            if signal != crate::gui::host::Signal::Clicked {
                continue;
            }
            let name = self.views.as_ref().unwrap().0.get(id).name.clone();
            match name.as_str() {
                // (a click on a slot without dragging: the source or target taken out, the result's page shown)
                "SrcIconSlot" if dropped.is_none() => self.put(false, None),
                "DstIconSlot" if dropped.is_none() => self.put(true, None),
                "ResultIconSlot" => {
                    if let Some(r) = &self.result {
                        page = Some(format!("itemref://{}/{}/{}", r.low, r.high, self.chosen()));
                    }
                }
                "ClearButton" => self.clear(true),
                "BuildButton" => {
                    send(Command::Build(self.chosen()));
                    self.clear(false);
                }
                _ => {}
            }
        }
        self.open = open;
        page
    }
}

/// `MAO_UI_PREVIEW=tradeskill`: the window as the 2026-10-30 capture's combine left it (Lock Picks on a Nano Programming
/// Interface: the source used in 3 processes, the target in 1, BreakingEntry 60 needed, a quality 1 result), on these
/// sample items (backpack places 0x4b and 0x5b).
pub fn preview() -> (TradeskillWindow, Vec<GearItem>) {
    let item = |slot, name: &str, quality| GearItem { slot, item_id: 1, high_id: 1, count: 1, quality, bag: false, flags: 0, name: name.into(), description: String::new(), icon: None, effects: Vec::new(), actions: Vec::new() };
    let items = vec![item(0x4b, "Lock Pick", 1), item(0x5b, "Nano Programming Interface", 1)];
    let mut w = TradeskillWindow { open: true, ..TradeskillWindow::default() };
    (w.source, w.target) = (Some(0x4b), Some(0x5b));
    let stats = [(165u32, 12i64)].into_iter().collect();
    for e in [Event::Source(3), Event::Target(1), Event::Requirement(165, 60), Event::Result { min: 1, max: 1, low: 0x26ad0, high: 0x26ad0, name: "Lock Pick".into(), icon: None }] {
        w.apply(e, &items, &stats);
    }
    (w, items)
}

fn set_text(tree: &mut crate::gui::view::Tree, root: crate::gui::view::Id, name: &str, text: &str) {
    if let Some(id) = tree.find(root, name)
        && let Some(t) = tree.control_mut::<crate::gui::controls::TextView>(id)
        && t.text != text
    {
        t.set_text(text);
        tree.relayout = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_from_the_share_as_the_original() {
        // (FUN_100def2d: 1 to 300 at 100% is 302, held to 300; the capture's result 1 to 1 builds at 1)
        assert_eq!(quality(1, 300, 100), 300);
        assert_eq!(quality(1, 300, 0), 1);
        assert_eq!(quality(1, 300, 50), 151);
        assert_eq!(quality(1, 1, 100), 1);
    }

    #[test]
    fn clear_takes_out_what_was_in_and_build_does_not() {
        take_commands();
        let mut w = TradeskillWindow::default();
        w.put(false, Some(0x5a));
        w.put(true, Some(0x5b));
        w.apply(Event::Result { min: 1, max: 1, low: 0x26ad0, high: 0x26ad0, name: "Lock Pick".into(), icon: None }, &[], &Default::default());
        assert_eq!(w.chosen(), 1);
        w.clear(true);
        assert_eq!(take_commands(), vec![Command::Source(Some(0x5a)), Command::Target(Some(0x5b)), Command::Source(None), Command::Target(None)]);
        assert!(w.result.is_none() && w.source.is_none());
    }
}
